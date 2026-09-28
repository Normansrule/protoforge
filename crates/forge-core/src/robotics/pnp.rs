//! Pick-and-place planning for the Iris main PCB with a SCARA robot.
//!
//! **Cell layout** (machine frame, mm, SCARA base at the origin): a rail of
//! tape feeders along `y = feeder_y`, an up-looking camera between the
//! feeders and the board, and the board with its lower-left corner at
//! `board_origin`. One feeder per unique part (same example part +
//! footprint).
//!
//! **Timing model per placement** (single vacuum nozzle):
//! `travel(prev → feeder)`, Z down, vacuum dwell, Z up, `travel(feeder →
//! camera)` + vision dwell (optional), `travel(camera → pad)`, Z down,
//! release dwell, Z up. XY travels are *joint-space* moves of the two SCARA
//! arms with trapezoidal profiles synchronised to the slower joint; the
//! nozzle rotation (J4) runs concurrently and only matters if it is the
//! slowest axis. Board handling (conveyor in/out, fiducial vision) is added
//! once per board.
//!
//! **Sequencing**: feeders are ordered along the rail by the mean board x of
//! their parts; placements are ordered greedily (nearest next pick+place in
//! time) and improved by pairwise swaps. The naïve BOM order is reported
//! for comparison.
//!
//! The machine parameters are a generic desktop SCARA model (editable), not
//! a vendor specification.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::arm::Arm2;
use super::traj::{synchronized, Trapezoid};
use crate::project::Project;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PnpConfig {
    pub l1_mm: f64,
    pub l2_mm: f64,
    pub j1_vmax_dps: f64,
    pub j1_amax_dps2: f64,
    pub j2_vmax_dps: f64,
    pub j2_amax_dps2: f64,
    pub j4_vmax_dps: f64,
    pub j4_amax_dps2: f64,
    /// Nozzle Z stroke between travel height and pick/place height, mm.
    pub z_stroke_mm: f64,
    pub z_vmax_mm_s: f64,
    pub z_amax_mm_s2: f64,
    pub pick_dwell_s: f64,
    pub place_dwell_s: f64,
    /// Stop over the up-camera for part alignment.
    pub vision: bool,
    pub vision_dwell_s: f64,
    pub feeder_pitch_mm: f64,
    pub feeder_y_mm: f64,
    pub camera_xy_mm: [f64; 2],
    pub board_origin_mm: [f64; 2],
    /// Conveyor load + unload per board, s.
    pub board_handling_s: f64,
    /// Down-camera dwell per fiducial (2 fiducials per side), s.
    pub fiducial_dwell_s: f64,
    /// Flip + re-register time if any part is on the bottom side, s.
    pub flip_s: f64,
    /// Optimise feeder order and placement sequence.
    pub optimize: bool,
    /// Trajectory samples per second for animation.
    pub fps: f64,
}

impl Default for PnpConfig {
    fn default() -> Self {
        PnpConfig {
            l1_mm: 250.0,
            l2_mm: 200.0,
            j1_vmax_dps: 360.0,
            j1_amax_dps2: 2400.0,
            j2_vmax_dps: 540.0,
            j2_amax_dps2: 3600.0,
            j4_vmax_dps: 1200.0,
            j4_amax_dps2: 12000.0,
            z_stroke_mm: 8.0,
            z_vmax_mm_s: 400.0,
            z_amax_mm_s2: 6000.0,
            pick_dwell_s: 0.05,
            place_dwell_s: 0.04,
            vision: true,
            vision_dwell_s: 0.12,
            feeder_pitch_mm: 12.0,
            feeder_y_mm: 200.0,
            camera_xy_mm: [0.0, 250.0],
            board_origin_mm: [-29.0, 290.0],
            board_handling_s: 3.0,
            fiducial_dwell_s: 0.25,
            flip_s: 8.0,
            optimize: true,
            fps: 30.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Feeder {
    pub slot: usize,
    pub key: String,
    pub footprint: String,
    pub refdes: Vec<String>,
    /// Pick position, machine mm.
    pub xy: [f64; 2],
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Placement {
    pub refdes: String,
    pub name: String,
    pub footprint: String,
    pub feeder: usize,
    /// Board coordinates (mm) and machine coordinates.
    pub board_xy: [f64; 2],
    pub xy: [f64; 2],
    pub rot_deg: f64,
    pub side: String,
    /// Approximate body size for drawing, mm.
    pub size_mm: [f64; 2],
    /// Time the part is released on the board.
    pub t_placed: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Event {
    pub t0: f64,
    pub t1: f64,
    /// travel | z | vision | dwell | handling
    pub kind: String,
    pub label: String,
    pub refdes: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Breakdown {
    pub travel: f64,
    pub z: f64,
    pub vision: f64,
    pub dwell: f64,
    pub handling: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Frames {
    pub t: Vec<f64>,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub z: Vec<f64>,
    pub q1: Vec<f64>,
    pub q2: Vec<f64>,
    pub nozzle_deg: Vec<f64>,
    /// Index into `placements` of the part on the nozzle (−1 = none).
    pub holding: Vec<i32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PnpPlan {
    pub cycle_s: f64,
    pub naive_cycle_s: f64,
    pub per_part_s: f64,
    pub cph: f64,
    pub breakdown: Breakdown,
    pub feeders: Vec<Feeder>,
    pub placements: Vec<Placement>,
    pub events: Vec<Event>,
    pub frames: Frames,
    pub travel_mm: f64,
    pub board_w_mm: f64,
    pub board_h_mm: f64,
    pub config: PnpConfig,
    pub reach_mm: [f64; 2],
    pub warnings: Vec<String>,
}

/// Approximate package body size from the footprint name.
pub fn footprint_size(fp: &str) -> [f64; 2] {
    let known: &[(&str, [f64; 2])] = &[
        ("0201", [0.6, 0.3]),
        ("0402", [1.0, 0.5]),
        ("0603", [1.6, 0.8]),
        ("0805", [2.0, 1.25]),
        ("1206", [3.2, 1.6]),
        ("SOT-23", [2.9, 2.8]),
        ("USB-C", [8.9, 7.3]),
        ("FPC-24", [15.0, 4.0]),
        ("JST-SH-2", [4.0, 4.3]),
        ("ANT_3216", [3.2, 1.6]),
    ];
    for (k, s) in known {
        if fp.starts_with(k) {
            return *s;
        }
    }
    // "..._AxB" suffix, e.g. QFN-48_6x6, LGA-14_3x2.5
    if let Some(tail) = fp.rsplit('_').next() {
        let parts: Vec<&str> = tail.split('x').collect();
        if parts.len() == 2 {
            if let (Ok(a), Ok(b)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                return [a, b];
            }
        }
    }
    [2.0, 2.0]
}

struct Machine {
    arm: Arm2,
    cfg: PnpConfig,
}

impl Machine {
    fn ik(&self, xy: [f64; 2]) -> Option<[f64; 2]> {
        // one consistent elbow ("right-handed") for the whole cell
        self.arm.ik(xy[0], xy[1], false)
    }
    fn move_time(&self, a: [f64; 2], b: [f64; 2], rot: f64) -> f64 {
        let (qa, qb) = match (self.ik(a), self.ik(b)) {
            (Some(x), Some(y)) => (x, y),
            _ => return f64::INFINITY,
        };
        let c = &self.cfg;
        let (t, _) = synchronized(
            &[(qb[0] - qa[0]).to_degrees(), (qb[1] - qa[1]).to_degrees(), rot],
            &[c.j1_vmax_dps, c.j2_vmax_dps, c.j4_vmax_dps],
            &[c.j1_amax_dps2, c.j2_amax_dps2, c.j4_amax_dps2],
        );
        t
    }
    fn z_time(&self) -> f64 {
        Trapezoid::min_time(self.cfg.z_stroke_mm, self.cfg.z_vmax_mm_s, self.cfg.z_amax_mm_s2).t_total
    }
    /// Pick at feeder f and place part at p, starting from `from`.
    fn part_time(&self, from: [f64; 2], feeder: [f64; 2], place: [f64; 2], rot: f64) -> f64 {
        let z = self.z_time();
        let c = &self.cfg;
        let mut t = self.move_time(from, feeder, 0.0) + 2.0 * z + c.pick_dwell_s;
        if c.vision {
            t += self.move_time(feeder, c.camera_xy_mm, rot * 0.5) + c.vision_dwell_s + self.move_time(c.camera_xy_mm, place, rot * 0.5);
        } else {
            t += self.move_time(feeder, place, rot);
        }
        t + 2.0 * z + c.place_dwell_s
    }
}

/// Precomputed move costs: `approach[a][i]` = travel from the position
/// after placing part `a` (index n = home) to part i's feeder; `rest[i]` =
/// everything from the pick at the feeder to the release on the pad.
struct Costs {
    approach: Vec<Vec<f64>>,
    rest: Vec<f64>,
    ret: Vec<f64>,
}

impl Costs {
    fn new(m: &Machine, home: [f64; 2], parts: &[Placement], feeders: &[Feeder]) -> Costs {
        let n = parts.len();
        let pos = |a: usize| if a == n { home } else { parts[a].xy };
        let approach = (0..=n).map(|a| parts.iter().map(|p| m.move_time(pos(a), feeders[p.feeder].xy, 0.0)).collect()).collect();
        let rest = parts.iter().map(|p| m.part_time(feeders[p.feeder].xy, feeders[p.feeder].xy, p.xy, p.rot_deg)).collect();
        let ret = (0..=n).map(|a| m.move_time(pos(a), home, 0.0)).collect();
        Costs { approach, rest, ret }
    }
    fn total(&self, order: &[usize]) -> f64 {
        let n = self.rest.len();
        let mut prev = n;
        let mut t = 0.0;
        for &i in order {
            t += self.approach[prev][i] + self.rest[i];
            prev = i;
        }
        t + self.ret[prev]
    }
}

fn sequence_time(m: &Machine, home: [f64; 2], order: &[usize], parts: &[Placement], feeders: &[Feeder]) -> f64 {
    Costs::new(m, home, parts, feeders).total(order)
}

/// Build the feeder bank and placement list; returns (feeders, placements).
fn build_parts(p: &Project, cfg: &PnpConfig, optimize: bool) -> (Vec<Feeder>, Vec<Placement>) {
    let placed = crate::pcb::placement(p);
    let mut parts: Vec<Placement> = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    let mut key_of: Vec<String> = Vec::new();
    for pl in &placed {
        let comp = p.components.iter().find(|c| c.refdes == pl.refdes);
        let (name, fp, key) = match comp {
            Some(c) => {
                let val = if c.example_part.is_empty() { c.name.clone() } else { c.example_part.clone() };
                (c.name.clone(), c.footprint.clone(), format!("{val} [{}]", c.footprint))
            }
            None => (pl.refdes.clone(), String::new(), pl.refdes.clone()),
        };
        if !keys.contains(&key) {
            keys.push(key.clone());
        }
        let side = if pl.side.is_empty() { "top".to_string() } else { pl.side.clone() };
        let bx = if side == "bottom" { p.design.pcb.width_mm - pl.x_mm } else { pl.x_mm };
        parts.push(Placement {
            refdes: pl.refdes.clone(),
            name,
            size_mm: footprint_size(&fp),
            footprint: fp,
            feeder: 0,
            board_xy: [pl.x_mm, pl.y_mm],
            xy: [cfg.board_origin_mm[0] + bx, cfg.board_origin_mm[1] + pl.y_mm],
            rot_deg: pl.rot_deg,
            side,
            t_placed: 0.0,
        });
        key_of.push(key);
    }
    // feeder order along the rail
    let mut order: Vec<usize> = (0..keys.len()).collect();
    if optimize {
        let mean_x = |k: &String| -> f64 {
            let xs: Vec<f64> = parts.iter().zip(&key_of).filter(|(_, kk)| *kk == k).map(|(p, _)| p.xy[0]).collect();
            xs.iter().sum::<f64>() / xs.len().max(1) as f64
        };
        order.sort_by(|a, b| mean_x(&keys[*a]).partial_cmp(&mean_x(&keys[*b])).unwrap());
    }
    let nf = keys.len();
    let x0 = -(nf as f64 - 1.0) * cfg.feeder_pitch_mm / 2.0;
    let mut feeders: Vec<Feeder> = Vec::new();
    let mut slot_of: BTreeMap<String, usize> = BTreeMap::new();
    for (slot, &ki) in order.iter().enumerate() {
        let key = keys[ki].clone();
        slot_of.insert(key.clone(), slot);
        let fp = parts.iter().zip(&key_of).find(|(_, k)| **k == key).map(|(p, _)| p.footprint.clone()).unwrap_or_default();
        feeders.push(Feeder {
            slot,
            key: key.clone(),
            footprint: fp,
            refdes: parts.iter().zip(&key_of).filter(|(_, k)| **k == key).map(|(p, _)| p.refdes.clone()).collect(),
            xy: [x0 + slot as f64 * cfg.feeder_pitch_mm, cfg.feeder_y_mm],
        });
    }
    for (pp, k) in parts.iter_mut().zip(&key_of) {
        pp.feeder = slot_of[k];
    }
    (feeders, parts)
}

fn optimize_order(m: &Machine, home: [f64; 2], parts: &[Placement], feeders: &[Feeder]) -> Vec<usize> {
    let costs = Costs::new(m, home, parts, feeders);
    let n = parts.len();
    // bottom-side parts are placed after the flip: keep sides grouped
    let mut result = Vec::new();
    for side in ["top", "bottom"] {
        let mut left: Vec<usize> = (0..n).filter(|&i| (parts[i].side == "bottom") == (side == "bottom")).collect();
        let mut prev = n;
        let mut seq = Vec::new();
        while !left.is_empty() {
            let (bi, _) = left
                .iter()
                .enumerate()
                .map(|(k, &i)| (k, costs.approach[prev][i] + costs.rest[i]))
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .unwrap();
            let i = left.remove(bi);
            prev = i;
            seq.push(i);
        }
        // pairwise-swap improvement (the approach costs make order matter)
        let mut best = costs.total(&seq);
        let mut improved = true;
        let mut rounds = 0;
        while improved && rounds < 8 {
            improved = false;
            rounds += 1;
            for a in 0..seq.len() {
                for b in (a + 1)..seq.len() {
                    seq.swap(a, b);
                    let t = costs.total(&seq);
                    if t + 1e-9 < best {
                        best = t;
                        improved = true;
                    } else {
                        seq.swap(a, b);
                    }
                }
            }
        }
        result.extend(seq);
    }
    result
}

struct Recorder<'a> {
    m: &'a Machine,
    t: f64,
    pos: [f64; 2],
    z: f64,
    nozzle: f64,
    holding: i32,
    frames: Frames,
    events: Vec<Event>,
    bd: Breakdown,
    next_frame: f64,
    dt_frame: f64,
    travel_mm: f64,
}

impl Recorder<'_> {
    fn push_frame(&mut self, t: f64, xy: [f64; 2], z: f64, nozzle: f64) {
        let q = self.m.ik(xy).unwrap_or([0.0, 0.0]);
        self.frames.t.push(t);
        self.frames.x.push(xy[0]);
        self.frames.y.push(xy[1]);
        self.frames.z.push(z);
        self.frames.q1.push(q[0].to_degrees());
        self.frames.q2.push(q[1].to_degrees());
        self.frames.nozzle_deg.push(nozzle);
        self.frames.holding.push(self.holding);
    }
    fn hold(&mut self, dur: f64, kind: &str, label: &str, refdes: Option<&str>) {
        let t0 = self.t;
        self.t += dur;
        while self.next_frame <= self.t {
            let tf = self.next_frame;
            self.push_frame(tf, self.pos, self.z, self.nozzle);
            self.next_frame += self.dt_frame;
        }
        self.add(kind, label, refdes, t0, dur);
    }
    fn add(&mut self, kind: &str, label: &str, refdes: Option<&str>, t0: f64, dur: f64) {
        match kind {
            "travel" => self.bd.travel += dur,
            "z" => self.bd.z += dur,
            "vision" => self.bd.vision += dur,
            "dwell" => self.bd.dwell += dur,
            _ => self.bd.handling += dur,
        }
        if dur > 0.0 {
            self.events.push(Event { t0, t1: t0 + dur, kind: kind.into(), label: label.into(), refdes: refdes.map(|s| s.to_string()) });
        }
    }
    fn zmove(&mut self, down: bool, label: &str, refdes: Option<&str>) {
        let c = &self.m.cfg;
        let tr = Trapezoid::min_time(c.z_stroke_mm, c.z_vmax_mm_s, c.z_amax_mm_s2);
        let t0 = self.t;
        let z0 = self.z;
        while self.next_frame <= t0 + tr.t_total {
            let tau = self.next_frame - t0;
            let dz = tr.eval(tau).0;
            let z = if down { z0 - dz } else { z0 + dz };
            let tf = self.next_frame;
            self.push_frame(tf, self.pos, z, self.nozzle);
            self.next_frame += self.dt_frame;
        }
        self.t += tr.t_total;
        self.z = if down { z0 - c.z_stroke_mm } else { z0 + c.z_stroke_mm };
        self.add("z", label, refdes, t0, tr.t_total);
    }
    fn travel(&mut self, to: [f64; 2], rot: f64, label: &str, refdes: Option<&str>) {
        let (qa, qb) = match (self.m.ik(self.pos), self.m.ik(to)) {
            (Some(a), Some(b)) => (a, b),
            _ => return,
        };
        let c = &self.m.cfg;
        let d = [(qb[0] - qa[0]).to_degrees(), (qb[1] - qa[1]).to_degrees(), rot - self.nozzle];
        let (tt, profs) = synchronized(&d, &[c.j1_vmax_dps, c.j2_vmax_dps, c.j4_vmax_dps], &[c.j1_amax_dps2, c.j2_amax_dps2, c.j4_amax_dps2]);
        let t0 = self.t;
        let n0 = self.nozzle;
        while self.next_frame <= t0 + tt {
            let tau = self.next_frame - t0;
            let q1 = qa[0] + profs[0].eval(tau).0.to_radians();
            let q2 = qa[1] + profs[1].eval(tau).0.to_radians();
            let (_, tip) = self.m.arm.fk(q1, q2);
            let nz = n0 + profs[2].eval(tau).0;
            let tf = self.next_frame;
            self.push_frame(tf, tip, self.z, nz);
            self.next_frame += self.dt_frame;
        }
        self.travel_mm += ((to[0] - self.pos[0]).powi(2) + (to[1] - self.pos[1]).powi(2)).sqrt();
        self.t += tt;
        self.pos = to;
        self.nozzle = rot;
        self.add("travel", label, refdes, t0, tt);
    }
}

/// Plan the board. With `optimize` on, the optimised sequence is compared
/// against the plain BOM order and the faster one is returned (ordering
/// heuristics are not guaranteed to win on every placement).
pub fn plan(p: &Project, cfg: &PnpConfig, record: bool) -> PnpPlan {
    if !cfg.optimize {
        return plan_inner(p, cfg, record);
    }
    let mut naive_cfg = cfg.clone();
    naive_cfg.optimize = false;
    let naive = plan_inner(p, &naive_cfg, false);
    let opt = plan_inner(p, cfg, false);
    let mut best = if opt.cycle_s <= naive.cycle_s { plan_inner(p, cfg, record) } else { plan_inner(p, &naive_cfg, record) };
    best.naive_cycle_s = naive.cycle_s;
    best
}

fn plan_inner(p: &Project, cfg: &PnpConfig, record: bool) -> PnpPlan {
    let arm = Arm2 { l1: cfg.l1_mm, l2: cfg.l2_mm, q1_min: -180.0, q1_max: 180.0, q2_min: -180.0, q2_max: 180.0 };
    let m = Machine { arm, cfg: cfg.clone() };
    let home = cfg.camera_xy_mm;
    let (feeders, mut parts) = build_parts(p, cfg, cfg.optimize);
    let mut warnings = Vec::new();
    for pp in &parts {
        if m.ik(pp.xy).is_none() {
            warnings.push(format!("{} at ({:.0}, {:.0}) mm is outside the SCARA reach", pp.refdes, pp.xy[0], pp.xy[1]));
        }
    }
    for f in &feeders {
        if m.ik(f.xy).is_none() {
            warnings.push(format!("feeder {} at x = {:.0} mm is outside the SCARA reach", f.slot, f.xy[0]));
        }
    }
    let order: Vec<usize> = if cfg.optimize { optimize_order(&m, home, &parts, &feeders) } else {
        let mut o: Vec<usize> = (0..parts.len()).filter(|&i| parts[i].side != "bottom").collect();
        o.extend((0..parts.len()).filter(|&i| parts[i].side == "bottom"));
        o
    };
    // naive comparison (BOM order, BOM feeder order)
    let naive_cycle = {
        let (f2, p2) = build_parts(p, cfg, false);
        let mut o: Vec<usize> = (0..p2.len()).filter(|&i| p2[i].side != "bottom").collect();
        o.extend((0..p2.len()).filter(|&i| p2[i].side == "bottom"));
        let has_bottom = p2.iter().any(|x| x.side == "bottom");
        sequence_time(&m, home, &o, &p2, &f2) + handling_time(cfg, has_bottom, &m, p)
    };

    let fps = if record { cfg.fps.clamp(5.0, 120.0) } else { 1e-9 };
    let mut rec = Recorder {
        m: &m,
        t: 0.0,
        pos: home,
        z: 0.0,
        nozzle: 0.0,
        holding: -1,
        frames: Frames::default(),
        events: vec![],
        bd: Breakdown::default(),
        next_frame: if record { 0.0 } else { f64::INFINITY },
        dt_frame: 1.0 / fps,
        travel_mm: 0.0,
    };
    let has_bottom = parts.iter().any(|x| x.side == "bottom");
    let bw = p.design.pcb.width_mm;
    let bh = p.design.pcb.height_mm;
    let fid = [[cfg.board_origin_mm[0] + 1.5, cfg.board_origin_mm[1] + 1.5], [cfg.board_origin_mm[0] + bw - 1.5, cfg.board_origin_mm[1] + bh - 1.5]];
    rec.hold(cfg.board_handling_s / 2.0, "handling", "conveyor: load board", None);
    let mut side_now = "top".to_string();
    let fiducials = |rec: &mut Recorder, lbl: &str| {
        for (k, f) in fid.iter().enumerate() {
            rec.travel(*f, 0.0, &format!("{lbl} fiducial {}", k + 1), None);
            rec.hold(cfg.fiducial_dwell_s, "vision", &format!("{lbl} fiducial {} image", k + 1), None);
        }
    };
    fiducials(&mut rec, "top");
    for &i in &order {
        let pp = parts[i].clone();
        if pp.side != side_now {
            rec.travel(home, 0.0, "park for flip", None);
            rec.hold(cfg.flip_s, "handling", "flip board", None);
            fiducials(&mut rec, "bottom");
            side_now = pp.side.clone();
        }
        let rd = Some(pp.refdes.as_str());
        let f = feeders[pp.feeder].xy;
        rec.travel(f, 0.0, &format!("to feeder {}", pp.feeder), rd);
        rec.zmove(true, "Z down (pick)", rd);
        rec.holding = i as i32;
        rec.hold(cfg.pick_dwell_s, "dwell", "vacuum on", rd);
        rec.zmove(false, "Z up", rd);
        if cfg.vision {
            rec.travel(cfg.camera_xy_mm, pp.rot_deg * 0.5, "to up-camera", rd);
            rec.hold(cfg.vision_dwell_s, "vision", "align part", rd);
        }
        rec.travel(pp.xy, pp.rot_deg, "to pad", rd);
        rec.zmove(true, "Z down (place)", rd);
        rec.hold(cfg.place_dwell_s, "dwell", "vacuum off / blow", rd);
        parts[i].t_placed = rec.t;
        rec.holding = -1;
        rec.zmove(false, "Z up", rd);
    }
    rec.travel(home, 0.0, "return home", None);
    rec.hold(cfg.board_handling_s / 2.0, "handling", "conveyor: unload board", None);
    let _ = has_bottom;
    let cycle = rec.t;
    let n = parts.len().max(1) as f64;
    PnpPlan {
        cycle_s: cycle,
        naive_cycle_s: if cfg.optimize { naive_cycle } else { cycle },
        per_part_s: (cycle - rec.bd.handling) / n,
        cph: 3600.0 * n / (cycle - rec.bd.handling).max(1e-9),
        breakdown: rec.bd,
        feeders,
        placements: parts,
        events: if record { rec.events } else { vec![] },
        frames: rec.frames,
        travel_mm: rec.travel_mm,
        board_w_mm: bw,
        board_h_mm: bh,
        config: cfg.clone(),
        reach_mm: [(cfg.l1_mm - cfg.l2_mm).abs(), cfg.l1_mm + cfg.l2_mm],
        warnings,
    }
}

fn handling_time(cfg: &PnpConfig, has_bottom: bool, m: &Machine, p: &Project) -> f64 {
    let bw = p.design.pcb.width_mm;
    let bh = p.design.pcb.height_mm;
    let f1 = [cfg.board_origin_mm[0] + 1.5, cfg.board_origin_mm[1] + 1.5];
    let f2 = [cfg.board_origin_mm[0] + bw - 1.5, cfg.board_origin_mm[1] + bh - 1.5];
    let fid = m.move_time(cfg.camera_xy_mm, f1, 0.0) + m.move_time(f1, f2, 0.0) + 2.0 * cfg.fiducial_dwell_s;
    cfg.board_handling_s + fid + if has_bottom { cfg.flip_s + fid } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_cycle_is_finite_and_positive() {
        let p = Project::iris();
        let plan = plan(&p, &PnpConfig::default(), true);
        assert!(plan.cycle_s.is_finite() && plan.cycle_s > 0.0);
        assert!(plan.warnings.is_empty(), "{:?}", plan.warnings);
        let n_on = p.components.iter().filter(|c| c.on_pcb).count();
        assert_eq!(plan.placements.len(), n_on);
        let bd = &plan.breakdown;
        let sum = bd.travel + bd.z + bd.vision + bd.dwell + bd.handling;
        assert!((sum - plan.cycle_s).abs() < 1e-6);
        assert!(plan.cycle_s <= plan.naive_cycle_s + 1e-9);
        assert!(!plan.frames.t.is_empty());
        // fewer feeders than parts (shared values)
        assert!(plan.feeders.len() < plan.placements.len());
    }

    #[test]
    fn footprint_sizes() {
        assert_eq!(footprint_size("QFN-48_6x6"), [6.0, 6.0]);
        assert_eq!(footprint_size("0402"), [1.0, 0.5]);
        assert_eq!(footprint_size("LGA-14_3x2.5"), [3.0, 2.5]);
    }
}
