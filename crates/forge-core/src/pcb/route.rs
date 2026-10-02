//! Two-layer grid maze router.
//!
//! The board is sampled on a square grid (`grid_mm`). Every grid point on
//! every layer carries an *occupancy owner*: free, owned by one net (it lies
//! within clearance of that net's copper, so only that net may run a track
//! centre-line there) or blocked (too close to copper of two nets, the board
//! edge, a hole or a keep-out). Grid *edges* carry the same state so that
//! off-grid obstacles (fine-pitch pads) are respected exactly between grid
//! points. A separate via map records where a via pad fits.
//!
//! Each connection is found with Lee's wavefront expansion (C. Y. Lee, "An
//! algorithm for path connections and its applications", *IRE Trans.
//! Electronic Computers* EC-10, 1961) in its weighted form (Dijkstra; with an
//! admissible Manhattan heuristic this is A*, Hart, Nilsson & Raphael 1968):
//! steps on the bottom layer cost more than on the top (the bottom is the
//! ground pour), bends and vias cost extra. Multi-pin nets grow a tree —
//! each new pin searches back to *any* copper already on the net (a
//! rectilinear Steiner heuristic). Nets that fail trigger rip-up and
//! re-route of the nets that block them.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use serde::{Deserialize, Serialize};

use super::board::Board;
use super::geom::{Shape, SpatialHash};
use super::netlist::NetClass;

pub const FREE: u16 = u16::MAX;
pub const BLOCK: u16 = u16::MAX - 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub net: String,
    #[serde(skip)]
    pub net_id: u16,
    /// 0 = F.Cu (top), 1 = B.Cu (bottom).
    pub layer: u8,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub width: f64,
}

impl Track {
    pub fn shape(&self) -> Shape {
        Shape::Capsule {
            ax: self.x0,
            ay: self.y0,
            bx: self.x1,
            by: self.y1,
            r: self.width / 2.0,
        }
    }
    pub fn length(&self) -> f64 {
        (self.x1 - self.x0).hypot(self.y1 - self.y0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Via {
    pub net: String,
    #[serde(skip)]
    pub net_id: u16,
    pub x: f64,
    pub y: f64,
    pub drill: f64,
    pub pad: f64,
}

impl Via {
    pub fn shape(&self) -> Shape {
        Shape::circle(self.x, self.y, self.pad / 2.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Copper,
    /// Keep-out region: only `net` (if not FREE) may enter.
    Keep,
    /// Non-plated hole.
    Hole,
}

#[derive(Clone, Copy, Debug)]
struct Obj {
    shape: Shape,
    top: bool,
    bot: bool,
    net: u16,
    kind: Kind,
}

/// A stub from a pad onto the routing grid.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Stub {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

#[derive(Clone, Copy, Debug)]
struct Access {
    node: usize,
    cost: u32,
    stub: Option<Stub>,
}

/// Per-net routing status.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NetRoute {
    pub net: String,
    pub pins: usize,
    pub connected_pins: usize,
    pub failed_pins: Vec<String>,
    pub ripups: u32,
}

/// One recorded wavefront (Lee expansion) for animation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Wave {
    pub from_pin: String,
    /// Expansion order: flat [layer, i, j, cost, ...].
    pub expanded: Vec<u32>,
    /// Final path: flat [layer, i, j, ...].
    pub path: Vec<u32>,
    pub found: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WaveTrace {
    pub net: String,
    pub nx: usize,
    pub ny: usize,
    pub grid_mm: f64,
    pub waves: Vec<Wave>,
}

pub struct Router<'a> {
    pub b: &'a Board,
    pub nx: usize,
    pub ny: usize,
    pub g: f64,
    n: usize,
    w: f64,
    via_r: f64,
    r_trk: f64,
    r_via: f64,
    // static occupancy (pads, edge, keep-outs, holes)
    s_trk: Vec<u16>,
    s_edg: Vec<u16>,
    s_via: Vec<u16>,
    // static + locked (ground fan-out) occupancy
    l_trk: Vec<u16>,
    l_edg: Vec<u16>,
    l_via: Vec<u16>,
    // live occupancy
    trk: Vec<u16>,
    edg: Vec<u16>,
    via: Vec<u16>,
    statics: Vec<Obj>,
    s_hash: SpatialHash,
    dobjs: Vec<Obj>,
    d_hash: SpatialHash,
    pub tracks: Vec<Track>,
    pub vias: Vec<Via>,
    /// Tracks/vias before these indices are locked (ground fan-out).
    locked_tracks: usize,
    locked_vias: usize,
    pub status: Vec<NetRoute>,
    /// GND pads the fan-out could not connect (pad indices).
    pub gnd_fanout_failed: Vec<usize>,
    gen: u32,
    stamp: Vec<u32>,
    dist: Vec<u32>,
    prev: Vec<u32>,
    seen: Vec<u32>,
    pub ripups_total: u32,
    recording: Option<Vec<Wave>>,
    pub expansions: u64,
    // negotiated congestion state
    nc_mode: bool,
    pres: u32,
    occ_trk: Vec<Occ>,
    occ_edg: Vec<Occ>,
    occ_via: Vec<Occ>,
    hist_trk: Vec<u32>,
    hist_edg: Vec<u32>,
    hist_via: Vec<u32>,
    /// Negotiated-congestion iterations used and conflicts per iteration.
    pub nc_history: Vec<usize>,
    /// When set, any node on this layer is a search target (ground drop to the pour).
    target_layer: Option<usize>,
    /// Ground pads already tied to the pour by thermal vias.
    gnd_done: Vec<usize>,
    /// Net of the via centred on each grid point (FREE if none).
    via_at: Vec<u16>,
    /// Number of vias whose drill is too close for a new drill here.
    via_near: Vec<u16>,
    /// Search window margin (mm); 0 = whole board.
    window_mm: f64,
    // ---- pin swapping: SoC GPIO pads any GPIO net may claim ----
    pool_base: u16,
    /// pool index → pad index
    pool_pad: Vec<usize>,
    /// pad index → pool index
    pad_pool: Vec<Option<usize>>,
    /// pool index → net that claimed the pad (FREE if unclaimed)
    pub claim: Vec<u16>,
    /// nets whose SoC end may be any free pool pad
    swap_nets: Vec<u16>,
    /// set while searching for a swappable net: pool zones are passable at a price
    swap_search: bool,
    /// O(1) target membership for the search
    tgt_stamp: Vec<u32>,
    /// Arrival direction per search node (4 = source / none), for bend costs.
    dir_of: Vec<u8>,
    /// Access points valid against static copper, cached for the negotiation phase.
    access_cache: HashMap<(usize, u16), Vec<Access>>,
    /// Bottom-layer step costs along x / y (the bottom carries the ground pour).
    pub bot_cost: (u32, u32),
}

const POOL_ZONE: u32 = 30;

/// Up to four nets (with reference counts) whose copper influences a grid
/// resource — used by negotiated congestion to price sharing.
#[derive(Clone, Copy, Default)]
struct Occ {
    n: [u16; 4],
    c: [u16; 4],
}

impl Occ {
    fn add(&mut self, net: u16) {
        for k in 0..4 {
            if self.c[k] > 0 && self.n[k] == net {
                self.c[k] += 1;
                return;
            }
        }
        for k in 0..4 {
            if self.c[k] == 0 {
                self.n[k] = net;
                self.c[k] = 1;
                return;
            }
        }
    }
    fn remove(&mut self, net: u16) {
        for k in 0..4 {
            if self.c[k] > 0 && self.n[k] == net {
                self.c[k] -= 1;
                return;
            }
        }
    }
    #[inline]
    fn others(&self, net: u16) -> u32 {
        (0..4).filter(|&k| self.c[k] > 0 && self.n[k] != net).count() as u32
    }
}

const EPS: f64 = 1e-6;

const STEP_TOP: u32 = 10;
// The bottom layer carries the ground pour: short vertical under-passes are
// cheap-ish, long runs along the board (which would slice the pour) are
// expensive — see `Router::bot_cost`.
const STEP_TOP_Y: u32 = 11;
const BEND: u32 = 8;
const VIA_COST: u32 = 70;
const RELAX_PENALTY: u32 = 400;
/// Pad access stubs reach at most this many grid steps (see `access_as`).
const ACCESS_REACH_STEPS: usize = 3;
const DIRS: [(i64, i64); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

fn combine(s: &mut u16, net: u16) {
    if *s == FREE {
        *s = net;
    } else if *s != net {
        *s = BLOCK;
    }
}
#[inline]
fn ok(s: u16, net: u16) -> bool {
    s == FREE || s == net
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    Relaxed,
    Negotiated,
}

const HIST_INC: u32 = 16;

/// (score, stub, new via position) for a ground fan-out candidate.
type FanoutChoice = (f64, Stub, Option<(f64, f64)>);
/// Negotiation snapshot: (conflicts, tracks, vias, pin claims, status).
type NcSnapshot = (usize, Vec<Track>, Vec<Via>, Vec<u16>, HashMap<u16, NetRoute>);

impl<'a> Router<'a> {
    pub fn new(b: &'a Board) -> Router<'a> {
        Self::with_pool(b, &[], &[])
    }

    /// Router whose `swap_nets` may end on any of the `pool` pads (pin swapping).
    pub fn with_pool(b: &'a Board, pool: &[usize], swap_nets: &[u16]) -> Router<'a> {
        let r = &b.rules;
        // The router works on a half-grid of the placement grid so that
        // 0.4 / 0.5 mm-pitch pads have grid points close to their centres.
        let g = r.grid_mm / 2.0;
        let nx = (b.width / g + 1e-9).floor() as usize + 1;
        let ny = (b.height / g + 1e-9).floor() as usize + 1;
        let n = nx * ny;
        let mut rt = Router {
            b,
            nx,
            ny,
            g,
            n,
            w: r.trace_mm,
            via_r: r.via_pad_mm / 2.0,
            r_trk: r.clearance_mm + r.trace_mm / 2.0,
            r_via: r.clearance_mm + r.via_pad_mm / 2.0,
            s_trk: vec![FREE; 2 * n],
            s_edg: vec![FREE; 4 * n],
            s_via: vec![FREE; n],
            l_trk: vec![],
            l_edg: vec![],
            l_via: vec![],
            trk: vec![],
            edg: vec![],
            via: vec![],
            statics: vec![],
            s_hash: SpatialHash::new(0.0, 0.0, b.width, b.height, 1.0),
            dobjs: vec![],
            d_hash: SpatialHash::new(0.0, 0.0, b.width, b.height, 1.0),
            tracks: vec![],
            vias: vec![],
            locked_tracks: 0,
            locked_vias: 0,
            status: vec![],
            gnd_fanout_failed: vec![],
            gen: 0,
            stamp: vec![0; 2 * n],
            dist: vec![0; 2 * n],
            prev: vec![u32::MAX; 2 * n],
            seen: vec![0; 2 * n],
            ripups_total: 0,
            recording: None,
            expansions: 0,
            nc_mode: false,
            pres: 0,
            occ_trk: vec![Occ::default(); 2 * n],
            occ_edg: vec![Occ::default(); 4 * n],
            occ_via: vec![Occ::default(); n],
            hist_trk: vec![0; 2 * n],
            hist_edg: vec![0; 4 * n],
            hist_via: vec![0; n],
            nc_history: vec![],
            target_layer: None,
            gnd_done: vec![],
            via_at: vec![FREE; n],
            via_near: vec![0; n],
            window_mm: 0.0,
            pool_base: b.net_count,
            pool_pad: vec![],
            pad_pool: vec![None; b.pads.len()],
            claim: vec![],
            swap_nets: vec![],
            swap_search: false,
            tgt_stamp: vec![0; 2 * n],
            dir_of: vec![4; 2 * n],
            access_cache: HashMap::new(),
            bot_cost: (40, 16),
        };
        for (k, &pi) in pool.iter().enumerate() {
            rt.pool_pad.push(pi);
            rt.pad_pool[pi] = Some(k);
            rt.claim.push(FREE);
        }
        rt.swap_nets = swap_nets.to_vec();
        rt.build_static();
        rt.l_trk = rt.s_trk.clone();
        rt.l_edg = rt.s_edg.clone();
        rt.l_via = rt.s_via.clone();
        rt.trk = rt.s_trk.clone();
        rt.edg = rt.s_edg.clone();
        rt.via = rt.s_via.clone();
        rt
    }

    #[inline]
    pub fn xy(&self, cell: usize) -> (f64, f64) {
        ((cell % self.nx) as f64 * self.g, (cell / self.nx) as f64 * self.g)
    }

    fn build_static(&mut self) {
        let b = self.b;
        let r = &b.rules;
        let e_t = r.edge_clearance_mm + self.w / 2.0 - EPS;
        let e_v = r.edge_clearance_mm + self.via_r - EPS;
        for c in 0..self.n {
            let (x, y) = self.xy(c);
            let d = x.min(y).min(b.width - x).min(b.height - y);
            if d < e_t {
                self.s_trk[c] = BLOCK;
                self.s_trk[self.n + c] = BLOCK;
            }
            if d < e_v {
                self.s_via[c] = BLOCK;
            }
        }
        let mut objs = Vec::new();
        for (pi, pd) in b.pads.iter().enumerate() {
            let owner = match self.pad_pool[pi] {
                Some(k) => self.pool_base + k as u16,
                None => pd.net_id,
            };
            objs.push(Obj {
                shape: pd.shape_geom(),
                top: pd.top,
                bot: pd.bottom,
                net: owner,
                kind: Kind::Copper,
            });
        }
        for k in &b.keepouts {
            let net = if k.allow_net.is_empty() {
                FREE
            } else {
                b.nets.iter().position(|n| n.name == k.allow_net).map(|i| i as u16).unwrap_or(FREE)
            };
            let shape = Shape::Rect {
                cx: (k.rect[0] + k.rect[2]) / 2.0,
                cy: (k.rect[1] + k.rect[3]) / 2.0,
                hw: (k.rect[2] - k.rect[0]) / 2.0,
                hh: (k.rect[3] - k.rect[1]) / 2.0,
            };
            objs.push(Obj {
                shape,
                top: k.top,
                bot: k.bottom,
                net,
                kind: Kind::Keep,
            });
        }
        for h in &b.holes {
            objs.push(Obj {
                shape: Shape::circle(h.x, h.y, h.d / 2.0),
                top: true,
                bot: true,
                net: BLOCK,
                kind: Kind::Hole,
            });
        }
        for (i, o) in objs.iter().enumerate() {
            self.s_hash.insert(i, o.shape.bbox());
        }
        let mut s_trk = std::mem::take(&mut self.s_trk);
        let mut s_edg = std::mem::take(&mut self.s_edg);
        let mut s_via = std::mem::take(&mut self.s_via);
        for o in &objs {
            self.mark(o, &mut s_trk, &mut s_edg, &mut s_via);
        }
        // no via inside any SMD pad (solder would wick into it)
        for pd in b.pads.iter().filter(|p| p.is_smd()) {
            let sh = pd.shape_geom();
            let r = self.via_r + 0.05;
            let bb = sh.bbox();
            let g = self.g;
            let i0 = ((bb[0] - r) / g).floor().max(0.0) as usize;
            let j0 = ((bb[1] - r) / g).floor().max(0.0) as usize;
            let i1 = (((bb[2] + r) / g).ceil().max(0.0) as usize).min(self.nx - 1);
            let j1 = (((bb[3] + r) / g).ceil().max(0.0) as usize).min(self.ny - 1);
            for j in j0..=j1.max(j0) {
                for i in i0..=i1.max(i0) {
                    if i < self.nx && j < self.ny && sh.dist_point(i as f64 * g, j as f64 * g) < r {
                        s_via[j * self.nx + i] = BLOCK;
                    }
                }
            }
        }
        self.s_trk = s_trk;
        self.s_edg = s_edg;
        self.s_via = s_via;
        self.statics = objs;
    }

    /// Mark an object's influence into occupancy arrays.
    fn mark(&self, o: &Obj, trk: &mut [u16], edg: &mut [u16], via: &mut [u16]) {
        let (rt, rv, own) = match o.kind {
            Kind::Copper => (self.r_trk - EPS, self.r_via - EPS, o.net),
            Kind::Keep => (self.w / 2.0 + 0.02, self.via_r + 0.02, o.net),
            Kind::Hole => {
                let hc = self.b.rules.hole_clearance_mm;
                (hc + self.w / 2.0 - EPS, hc + self.via_r - EPS, BLOCK)
            }
        };
        let (nx, n, g) = (self.nx, self.n, self.g);
        let bb = o.shape.bbox();
        let rr = rt.max(rv) + g;
        if bb[2] + rr < 0.0 || bb[3] + rr < 0.0 {
            return;
        }
        let i0 = ((bb[0] - rr) / g).floor().max(0.0) as usize;
        let j0 = ((bb[1] - rr) / g).floor().max(0.0) as usize;
        let i1 = (((bb[2] + rr) / g).ceil().max(0.0) as usize).min(self.nx - 1);
        let j1 = (((bb[3] + rr) / g).ceil().max(0.0) as usize).min(self.ny - 1);
        if i0 > i1 || j0 > j1 {
            return;
        }
        for j in j0..=j1 {
            for i in i0..=i1 {
                let c = j * nx + i;
                let (x, y) = (i as f64 * g, j as f64 * g);
                let d = o.shape.dist_point(x, y);
                if d > rr + g {
                    continue;
                }
                for (layer, on) in [(0usize, o.top), (1usize, o.bot)] {
                    if !on {
                        continue;
                    }
                    if d < rt {
                        if o.kind == Kind::Keep && own == FREE {
                            trk[layer * n + c] = BLOCK;
                        } else {
                            combine(&mut trk[layer * n + c], own);
                        }
                    }
                    for (k, (ex, ey), valid) in [(0usize, (x + g, y), i + 1 < self.nx), (1usize, (x, y + g), j + 1 < self.ny)] {
                        if valid && o.shape.dist_seg(x, y, ex, ey) < rt {
                            let e = &mut edg[(layer * n + c) * 2 + k];
                            if o.kind == Kind::Keep && own == FREE {
                                *e = BLOCK;
                            } else {
                                combine(e, own);
                            }
                        }
                    }
                }
                if d < rv {
                    if o.kind == Kind::Keep {
                        via[c] = BLOCK;
                    } else {
                        combine(&mut via[c], own);
                    }
                }
            }
        }
    }

    fn obj_of_track(t: &Track) -> Obj {
        Obj {
            shape: t.shape(),
            top: t.layer == 0,
            bot: t.layer == 1,
            net: t.net_id,
            kind: Kind::Copper,
        }
    }
    fn obj_of_via(v: &Via) -> Obj {
        Obj {
            shape: v.shape(),
            top: true,
            bot: true,
            net: v.net_id,
            kind: Kind::Copper,
        }
    }

    fn push_dyn(&mut self, o: Obj) {
        if self.nc_mode {
            self.occ_apply(&o, true);
            return;
        }
        let mut trk = std::mem::take(&mut self.trk);
        let mut edg = std::mem::take(&mut self.edg);
        let mut via = std::mem::take(&mut self.via);
        self.mark(&o, &mut trk, &mut edg, &mut via);
        self.trk = trk;
        self.edg = edg;
        self.via = via;
        self.d_hash.insert(self.dobjs.len(), o.shape.bbox());
        self.dobjs.push(o);
    }

    fn add_track(&mut self, t: Track) {
        if t.length() < 1e-9 && t.width <= 0.0 {
            return;
        }
        self.push_dyn(Self::obj_of_track(&t));
        self.tracks.push(t);
    }

    /// Update the via bookkeeping (reuse map + drill spacing) for one via.
    fn note_via(&mut self, v: &Via, add: bool) {
        let g = self.g;
        let rad = v.drill + 0.2; // centre distance below which a second drill violates hole spacing
        let i0 = ((v.x - rad) / g).floor().max(0.0) as usize;
        let j0 = ((v.y - rad) / g).floor().max(0.0) as usize;
        let i1 = (((v.x + rad) / g).ceil().max(0.0) as usize).min(self.nx - 1);
        let j1 = (((v.y + rad) / g).ceil().max(0.0) as usize).min(self.ny - 1);
        for j in j0..=j1 {
            for i in i0..=i1 {
                let (x, y) = (i as f64 * g, j as f64 * g);
                let d = (x - v.x).hypot(y - v.y);
                let c = j * self.nx + i;
                if d < 1e-6 {
                    self.via_at[c] = if add { v.net_id } else { FREE };
                }
                if d < rad - 1e-6 {
                    if add {
                        self.via_near[c] += 1;
                    } else {
                        self.via_near[c] = self.via_near[c].saturating_sub(1);
                    }
                }
            }
        }
    }

    fn recompute_vias(&mut self) {
        self.via_at.iter_mut().for_each(|x| *x = FREE);
        self.via_near.iter_mut().for_each(|x| *x = 0);
        let mut vs = self.vias.clone();
        // plated pad drills and non-plated holes also keep new drills away
        for p in self.b.pads.iter().filter(|p| p.drill > 0.0) {
            vs.push(Via {
                net: String::new(),
                net_id: BLOCK,
                x: p.x,
                y: p.y,
                drill: (p.drill + self.b.rules.via_drill_mm) / 2.0,
                pad: 0.0,
            });
        }
        for h in &self.b.holes {
            vs.push(Via {
                net: String::new(),
                net_id: BLOCK,
                x: h.x,
                y: h.y,
                drill: (h.d + self.b.rules.via_drill_mm) / 2.0,
                pad: 0.0,
            });
        }
        for v in &vs {
            self.note_via(v, true);
        }
    }

    fn add_via(&mut self, v: Via) {
        self.note_via(&v, true);
        self.push_dyn(Self::obj_of_via(&v));
        self.vias.push(v);
    }

    /// Freeze the current copper (ground fan-out) so rip-up never removes it.
    fn lock(&mut self) {
        self.l_trk = self.trk.clone();
        self.l_edg = self.edg.clone();
        self.l_via = self.via.clone();
        self.locked_tracks = self.tracks.len();
        self.locked_vias = self.vias.len();
    }

    /// Rebuild the live occupancy from the locked state + unlocked copper.
    fn rebuild(&mut self) {
        self.trk.copy_from_slice(&self.l_trk);
        self.edg.copy_from_slice(&self.l_edg);
        self.via.copy_from_slice(&self.l_via);
        self.dobjs.clear();
        self.d_hash = SpatialHash::new(0.0, 0.0, self.b.width, self.b.height, 1.0);
        let objs: Vec<Obj> = self
            .tracks
            .iter()
            .map(Self::obj_of_track)
            .chain(self.vias.iter().map(Self::obj_of_via))
            .collect();
        let n_locked_t = self.locked_tracks;
        let n_t = self.tracks.len();
        let n_locked_v = self.locked_vias;
        let mut trk = std::mem::take(&mut self.trk);
        let mut edg = std::mem::take(&mut self.edg);
        let mut via = std::mem::take(&mut self.via);
        for (i, o) in objs.iter().enumerate() {
            let locked = if i < n_t { i < n_locked_t } else { i - n_t < n_locked_v };
            if !locked {
                self.mark(o, &mut trk, &mut edg, &mut via);
            }
            self.d_hash.insert(i, o.shape.bbox());
        }
        self.trk = trk;
        self.edg = edg;
        self.via = via;
        self.dobjs = objs;
        self.recompute_vias();
    }

    fn rip_up(&mut self, net: u16) {
        self.release_claims(net);
        let lt = self.locked_tracks;
        let lv = self.locked_vias;
        let mut k = 0;
        self.tracks.retain(|t| {
            k += 1;
            k <= lt || t.net_id != net
        });
        let mut k = 0;
        self.vias.retain(|v| {
            k += 1;
            k <= lv || v.net_id != net
        });
    }

    // -----------------------------------------------------------------
    // geometric checks against real copper
    // -----------------------------------------------------------------

    /// Is a zero-width segment on `layer` at least trace-clearance away from
    /// every object `net` must avoid?
    fn seg_clear(&self, layer: u8, net: u16, s: Stub, include_dynamic: bool) -> bool {
        let bb = [
            s.x0.min(s.x1) - 1.0,
            s.y0.min(s.y1) - 1.0,
            s.x0.max(s.x1) + 1.0,
            s.y0.max(s.y1) + 1.0,
        ];
        let mut ids = Vec::new();
        self.s_hash.query(bb, &mut ids);
        for &i in &ids {
            let o = &self.statics[i];
            if !(if layer == 0 { o.top } else { o.bot }) {
                continue;
            }
            let need = match o.kind {
                Kind::Copper if !self.own(o.net, net) => self.r_trk,
                Kind::Keep if o.net != net => self.w / 2.0 + 0.02,
                Kind::Hole => self.b.rules.hole_clearance_mm + self.w / 2.0,
                _ => continue,
            };
            if o.shape.dist_seg(s.x0, s.y0, s.x1, s.y1) < need - EPS {
                return false;
            }
        }
        let dyn_limit = if include_dynamic {
            self.dobjs.len()
        } else {
            self.locked_tracks.min(self.dobjs.len())
        };
        self.d_hash.query(bb, &mut ids);
        for &i in &ids {
            if !include_dynamic && !self.is_locked_obj(i) {
                continue;
            }
            let _ = dyn_limit;
            let o = &self.dobjs[i];
            if !(if layer == 0 { o.top } else { o.bot }) || o.net == net {
                continue;
            }
            if o.shape.dist_seg(s.x0, s.y0, s.x1, s.y1) < self.r_trk - EPS {
                return false;
            }
        }
        let e = self.b.rules.edge_clearance_mm + self.w / 2.0 - EPS;
        for (x, y) in [(s.x0, s.y0), (s.x1, s.y1)] {
            if x.min(y).min(self.b.width - x).min(self.b.height - y) < e {
                return false;
            }
        }
        true
    }

    /// May `net` treat copper owned by `owner` as its own?
    #[inline]
    fn own(&self, owner: u16, net: u16) -> bool {
        if owner == net {
            return true;
        }
        let k = owner.wrapping_sub(self.pool_base) as usize;
        owner >= self.pool_base && k < self.claim.len() && self.claim[k] == net
    }

    #[inline]
    fn okc(&self, s: u16, net: u16) -> bool {
        s == FREE || self.own(s, net)
    }

    fn is_locked_obj(&self, i: usize) -> bool {
        let n_t = self.tracks.len();
        if i < n_t {
            i < self.locked_tracks
        } else {
            i - n_t < self.locked_vias
        }
    }

    /// Can a via of `net` sit at (x, y)? `own_pad` exempts one pad (via-in-pad for thermal pads).
    pub fn via_clear(&self, x: f64, y: f64, net: u16, own_pad: Option<usize>) -> bool {
        let r = &self.b.rules;
        let vr = self.via_r;
        if x.min(y).min(self.b.width - x).min(self.b.height - y) < r.edge_clearance_mm + vr - EPS {
            return false;
        }
        let bb = [x - 2.0, y - 2.0, x + 2.0, y + 2.0];
        let mut ids = Vec::new();
        self.s_hash.query(bb, &mut ids);
        for &i in &ids {
            let o = &self.statics[i];
            let d = o.shape.dist_point(x, y);
            let bad = match o.kind {
                Kind::Copper => {
                    if !self.own(o.net, net) {
                        d < r.clearance_mm + vr - EPS
                    } else {
                        // same-net SMD pad: no via-in-pad except the exempted one
                        let is_own = own_pad.map(|pi| i == pi).unwrap_or(false);
                        !is_own && i < self.b.pads.len() && self.b.pads[i].is_smd() && d < vr + 0.05
                    }
                }
                Kind::Keep => d < vr,
                Kind::Hole => d < r.hole_clearance_mm + vr - EPS,
            };
            if bad {
                return false;
            }
        }
        self.d_hash.query(bb, &mut ids);
        for &i in &ids {
            let o = &self.dobjs[i];
            if o.net != net && o.shape.dist_point(x, y) < r.clearance_mm + vr - EPS {
                return false;
            }
        }
        // keep drills apart (hole-to-hole spacing), any net
        let d = self.b.rules.via_drill_mm;
        if self
            .vias
            .iter()
            .any(|v| (v.x - x).hypot(v.y - y) < (v.drill + d) / 2.0 + 0.2 - 1e-6)
        {
            return false;
        }
        if self
            .b
            .pads
            .iter()
            .any(|p| p.drill > 0.0 && (p.x - x).hypot(p.y - y) < (p.drill + d) / 2.0 + 0.2 - 1e-6)
        {
            return false;
        }
        if self.b.holes.iter().any(|h| (h.x - x).hypot(h.y - y) < (h.d + d) / 2.0 + 0.2 - 1e-6) {
            return false;
        }
        true
    }

    /// Candidate grid entry points for a pad.
    fn access(&self, pi: usize, mode: Mode) -> Vec<Access> {
        self.access_as(pi, mode, self.b.pads[pi].net_id)
    }

    /// Entry points for pad `pi` when it is (or becomes) part of `net`.
    ///
    /// Stubs are kept short (three grid steps): long diagonal stubs sneak
    /// past neighbouring pads, cross each other and are invisible to the
    /// negotiation's grid-based history, which then cannot converge. Only a
    /// pad with no entry that close falls back to a 1 mm reach.
    fn access_as(&self, pi: usize, mode: Mode, net: u16) -> Vec<Access> {
        let short = self.access_within(pi, mode, net, ACCESS_REACH_STEPS as f64 * self.g);
        if !short.is_empty() {
            return short;
        }
        self.access_within(pi, mode, net, 1.0)
    }

    fn access_within(&self, pi: usize, mode: Mode, net: u16, reach: f64) -> Vec<Access> {
        let pd = &self.b.pads[pi];
        let sh = pd.shape_geom();
        let bb = sh.bbox();
        let g = self.g;
        let arr = if mode == Mode::Relaxed { &self.l_trk } else { &self.trk };
        let mut out = Vec::new();
        let i0 = ((bb[0] - reach) / g).floor().max(0.0) as usize;
        let j0 = ((bb[1] - reach) / g).floor().max(0.0) as usize;
        let i1 = (((bb[2] + reach) / g).ceil().max(0.0) as usize).min(self.nx - 1);
        let j1 = (((bb[3] + reach) / g).ceil().max(0.0) as usize).min(self.ny - 1);
        for layer in 0..2u8 {
            if (layer == 0 && !pd.top) || (layer == 1 && !pd.bottom) {
                continue;
            }
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let c = j * self.nx + i;
                    let node = layer as usize * self.n + c;
                    if !self.okc(arr[node], net) {
                        continue;
                    }
                    let (x, y) = (i as f64 * g, j as f64 * g);
                    let d = sh.dist_point(x, y);
                    if d > reach {
                        continue;
                    }
                    if d <= 1e-9 {
                        out.push(Access { node, cost: 0, stub: None });
                        continue;
                    }
                    // stub from the closest point inside the pad
                    let (qx, qy) = match sh {
                        Shape::Rect { cx, cy, hw, hh } => {
                            let sx = (hw - self.w / 2.0).max(0.0);
                            let sy = (hh - self.w / 2.0).max(0.0);
                            (x.clamp(cx - sx, cx + sx), y.clamp(cy - sy, cy + sy))
                        }
                        Shape::Capsule { ax, ay, .. } => (ax, ay),
                    };
                    let stub = Stub {
                        x0: qx,
                        y0: qy,
                        x1: x,
                        y1: y,
                    };
                    if !self.seg_clear(layer, net, stub, mode == Mode::Normal) {
                        continue;
                    }
                    let len = (x - qx).hypot(y - qy);
                    out.push(Access {
                        node,
                        cost: 2 + (len / g * STEP_TOP as f64) as u32,
                        stub: Some(stub),
                    });
                }
            }
        }
        out
    }

    // -----------------------------------------------------------------
    // wavefront search
    // -----------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    fn search(
        &mut self,
        net: u16,
        sources: &[Access],
        targets: &HashMap<usize, Option<Stub>>,
        mode: Mode,
        allow_via: bool,
        astar: bool,
        rec: Option<&mut Wave>,
    ) -> Option<Vec<usize>> {
        self.search_w(net, sources, targets, mode, allow_via, astar, rec)
    }

    #[allow(clippy::too_many_arguments)]
    fn search_w(
        &mut self,
        net: u16,
        sources: &[Access],
        targets: &HashMap<usize, Option<Stub>>,
        mode: Mode,
        allow_via: bool,
        astar: bool,
        rec: Option<&mut Wave>,
    ) -> Option<Vec<usize>> {
        if rec.is_none() && self.target_layer.is_none() {
            self.window_mm = 3.0;
            let r = self.search_once(net, sources, targets, mode, allow_via, astar, None);
            self.window_mm = 0.0;
            if r.is_some() {
                return r;
            }
        }
        self.search_once(net, sources, targets, mode, allow_via, astar, rec)
    }

    #[allow(clippy::too_many_arguments)]
    fn search_once(
        &mut self,
        net: u16,
        sources: &[Access],
        targets: &HashMap<usize, Option<Stub>>,
        mode: Mode,
        allow_via: bool,
        astar: bool,
        rec: Option<&mut Wave>,
    ) -> Option<Vec<usize>> {
        let tl = self.target_layer;
        if sources.is_empty() || (targets.is_empty() && tl.is_none()) {
            return None;
        }
        self.gen = self.gen.wrapping_add(1);
        if self.gen == 0 {
            self.stamp.iter_mut().for_each(|s| *s = 0);
            self.seen.iter_mut().for_each(|s| *s = 0);
            self.tgt_stamp.iter_mut().for_each(|s| *s = 0);
            self.gen = 1;
        }
        let gen = self.gen;
        let (n, nx, ny) = (self.n, self.nx, self.ny);
        let mut stamp = std::mem::take(&mut self.stamp);
        let mut dist = std::mem::take(&mut self.dist);
        let mut prev = std::mem::take(&mut self.prev);
        let mut seen = std::mem::take(&mut self.seen);
        // target bbox for the A* heuristic
        let (mut ti0, mut tj0, mut ti1, mut tj1) = (usize::MAX, usize::MAX, 0usize, 0usize);
        for &t in targets.keys() {
            let c = t % n;
            let (i, j) = (c % nx, c / nx);
            ti0 = ti0.min(i);
            tj0 = tj0.min(j);
            ti1 = ti1.max(i);
            tj1 = tj1.max(j);
        }
        // search window: bounding box of sources and targets plus a margin
        // (retried on the whole board when it fails).
        let (mut wi0, mut wj0, mut wi1, mut wj1) = (ti0, tj0, ti1, tj1);
        for s in sources {
            let c = s.node % n;
            wi0 = wi0.min(c % nx);
            wj0 = wj0.min(c / nx);
            wi1 = wi1.max(c % nx);
            wj1 = wj1.max(c / nx);
        }
        let margin = (self.window_mm / self.g).ceil() as usize;
        let windowed = self.window_mm > 0.0 && tl.is_none();
        let (wi0, wj0) = (wi0.saturating_sub(margin), wj0.saturating_sub(margin));
        let (wi1, wj1) = ((wi1 + margin).min(nx - 1), (wj1 + margin).min(ny - 1));
        let heur = |c: usize| -> u32 {
            if !astar || tl.is_some() {
                return 0;
            }
            let (i, j) = (c % nx, c / nx);
            let dx = if i < ti0 { ti0 - i } else { i.saturating_sub(ti1) };
            let dy = if j < tj0 { tj0 - j } else { j.saturating_sub(tj1) };
            ((dx + dy) as u32) * STEP_TOP
        };
        let mut heap: BinaryHeap<Reverse<(u32, u32)>> = BinaryHeap::new();
        let mut dir_of = std::mem::take(&mut self.dir_of);
        for s in sources {
            let st = s.node;
            if stamp[st] != gen || dist[st] > s.cost {
                stamp[st] = gen;
                dist[st] = s.cost;
                prev[st] = u32::MAX;
                dir_of[st] = 4;
                heap.push(Reverse((s.cost + heur(s.node % n), st as u32)));
            }
        }
        let (live_trk, live_edg, live_via) = (&self.trk, &self.edg, &self.via);
        let (base_trk, base_edg, base_via) = (&self.l_trk, &self.l_edg, &self.l_via);
        let (occ_trk, occ_edg, occ_via) = (&self.occ_trk, &self.occ_edg, &self.occ_via);
        let (hist_trk, hist_edg, hist_via) = (&self.hist_trk, &self.hist_edg, &self.hist_via);
        let pres = self.pres;
        let (via_at, via_near) = (&self.via_at, &self.via_near);
        let (bot_x, bot_y) = self.bot_cost;
        let (pool_base, claim, swap_search) = (self.pool_base, &self.claim, self.swap_search);
        // copper owner check honouring pin-swap claims; unclaimed pool zones
        // are passable at a price while a swappable net looks for its pad.
        let own_cost = |s: u16| -> Option<u32> {
            if s == FREE || s == net {
                return Some(0);
            }
            let k = s.wrapping_sub(pool_base) as usize;
            if s >= pool_base && k < claim.len() {
                if claim[k] == net {
                    return Some(0);
                }
                if swap_search && claim[k] == FREE {
                    return Some(POOL_ZONE);
                }
            }
            None
        };
        let ok = |s: u16, _net: u16| own_cost(s).is_some();
        // cost of entering a resource: k = 0 track cell, 1 edge, 2 via site
        let gate = |k: u8, idx: usize| -> Option<u32> {
            let (base, live) = match k {
                0 => (base_trk[idx], live_trk[idx]),
                1 => (base_edg[idx], live_edg[idx]),
                _ => (base_via[idx], live_via[idx]),
            };
            let zone = own_cost(base).unwrap_or(0);
            match mode {
                Mode::Normal => {
                    if ok(live, net) {
                        Some(zone)
                    } else {
                        None
                    }
                }
                Mode::Relaxed => {
                    if !ok(base, net) {
                        None
                    } else if ok(live, net) {
                        Some(0)
                    } else {
                        Some(RELAX_PENALTY)
                    }
                }
                Mode::Negotiated => {
                    if !ok(base, net) {
                        return None;
                    }
                    let (o, h) = match k {
                        0 => (occ_trk[idx].others(net), hist_trk[idx]),
                        1 => (occ_edg[idx].others(net), hist_edg[idx]),
                        _ => (occ_via[idx].others(net), hist_via[idx]),
                    };
                    Some(pres * o + h + zone)
                }
            }
        };
        let mut tgt_stamp = std::mem::take(&mut self.tgt_stamp);
        for &t in targets.keys() {
            tgt_stamp[t] = gen;
        }
        let mut found: Option<usize> = None;
        let mut rec = rec;
        let mut expanded = 0u64;
        while let Some(Reverse((f, st))) = heap.pop() {
            let st = st as usize;
            let gcost = dist[st];
            let node = st;
            let dir = dir_of[st] as usize;
            if f > gcost + heur(node % n) {
                continue; // stale heap entry
            }
            expanded += 1;
            if let Some(r) = rec.as_deref_mut() {
                // cap the recording (~40k cells) so the animation payload stays small
                if seen[node] != gen && r.expanded.len() < 160_000 {
                    seen[node] = gen;
                    let c = node % n;
                    r.expanded
                        .extend_from_slice(&[(node / n) as u32, (c % nx) as u32, (c / nx) as u32, gcost]);
                }
            }
            if tgt_stamp[node] == gen || tl.map(|l| node / n == l).unwrap_or(false) {
                found = Some(st);
                break;
            }
            let layer = node / n;
            let c = node % n;
            let (i, j) = (c % nx, c / nx);
            for (d, &(di, dj)) in DIRS.iter().enumerate() {
                let step = match (layer, d < 2) {
                    (0, true) => STEP_TOP,
                    (0, false) => STEP_TOP_Y,
                    (_, true) => bot_x,
                    (_, false) => bot_y,
                };
                let (ni, nj) = (i as i64 + di, j as i64 + dj);
                if ni < 0 || nj < 0 || ni >= nx as i64 || nj >= ny as i64 {
                    continue;
                }
                if windowed && ((ni as usize) < wi0 || (ni as usize) > wi1 || (nj as usize) < wj0 || (nj as usize) > wj1) {
                    continue;
                }
                let nc = nj as usize * nx + ni as usize;
                let eidx = match d {
                    0 => (layer * n + c) * 2,
                    1 => (layer * n + nc) * 2,
                    2 => (layer * n + c) * 2 + 1,
                    _ => (layer * n + nc) * 2 + 1,
                };
                let Some(pe) = gate(1, eidx) else { continue };
                let nnode = layer * n + nc;
                let Some(pc) = gate(0, nnode) else { continue };
                let bend = if d != dir && dir < 4 { BEND } else { 0 };
                let ng = gcost + step + bend + pe + pc;
                let ns = nnode;
                if stamp[ns] != gen || ng < dist[ns] {
                    stamp[ns] = gen;
                    dist[ns] = ng;
                    prev[ns] = st as u32;
                    dir_of[ns] = d as u8;
                    heap.push(Reverse((ng + heur(nc), ns as u32)));
                }
            }
            if allow_via {
                let other = (1 - layer) * n + c;
                let reuse = via_at[c] == net;
                let via_gate = if reuse {
                    Some(0)
                } else if via_near[c] > 0 {
                    None
                } else {
                    gate(2, c)
                };
                if let (Some(pv), Some(po)) = (via_gate, gate(0, other)) {
                    let ng = gcost + if reuse { 4 } else { VIA_COST } + pv + po;
                    let ns = other;
                    if stamp[ns] != gen || ng < dist[ns] {
                        stamp[ns] = gen;
                        dist[ns] = ng;
                        prev[ns] = st as u32;
                        dir_of[ns] = dir as u8;
                        heap.push(Reverse((ng + heur(c), ns as u32)));
                    }
                }
            }
        }
        self.expansions += expanded;
        let result = found.map(|end| {
            let mut path = Vec::new();
            let mut s = end;
            loop {
                let node = s;
                if path.last() != Some(&node) {
                    path.push(node);
                }
                let p = prev[s];
                if p == u32::MAX {
                    break;
                }
                s = p as usize;
            }
            path.reverse();
            path
        });
        self.stamp = stamp;
        self.dist = dist;
        self.prev = prev;
        self.seen = seen;
        self.tgt_stamp = tgt_stamp;
        self.dir_of = dir_of;
        let path = result?;
        if let Some(r) = rec {
            for &node in &path {
                let c = node % n;
                r.path.extend_from_slice(&[(node / n) as u32, (c % nx) as u32, (c / nx) as u32]);
            }
            r.found = true;
        }
        Some(path)
    }

    /// Convert a grid path into tracks + vias and commit it.
    fn commit_path(&mut self, net: u16, path: &[usize]) {
        let name = self.b.nets[net as usize].name.clone();
        let n = self.n;
        let mut k = 0;
        while k + 1 < path.len() {
            let (a, b) = (path[k], path[k + 1]);
            if a % n == b % n {
                let (x, y) = self.xy(a % n);
                if self.via_at[a % n] != net {
                    self.add_via(Via {
                        net: name.clone(),
                        net_id: net,
                        x,
                        y,
                        drill: self.b.rules.via_drill_mm,
                        pad: self.b.rules.via_pad_mm,
                    });
                }
                k += 1;
                continue;
            }
            // extend a straight run on one layer
            let layer = a / n;
            let (x0, y0) = self.xy(a % n);
            // direction of a grid step as integer signs (f64::signum(0.0) is 1.0!)
            let step = |a: usize, b: usize| -> (i64, i64) {
                let (ca, cb) = (a % n, b % n);
                let (ia, ja) = ((ca % self.nx) as i64, (ca / self.nx) as i64);
                let (ib, jb) = ((cb % self.nx) as i64, (cb / self.nx) as i64);
                ((ib - ia).signum(), (jb - ja).signum())
            };
            let dir = step(a, b);
            let mut e = k + 1;
            while e + 1 < path.len() && path[e + 1] / n == layer && path[e + 1] % n != path[e] % n {
                if step(path[e], path[e + 1]) != dir {
                    break;
                }
                e += 1;
            }
            let (x1, y1) = self.xy(path[e] % n);
            self.add_track(Track {
                net: name.clone(),
                net_id: net,
                layer: layer as u8,
                x0,
                y0,
                x1,
                y1,
                width: self.w,
            });
            k = e;
        }
    }

    fn commit_stub(&mut self, net: u16, layer: u8, s: Stub) {
        let name = self.b.nets[net as usize].name.clone();
        self.add_track(Track {
            net: name,
            net_id: net,
            layer,
            x0: s.x0,
            y0: s.y0,
            x1: s.x1,
            y1: s.y1,
            width: self.w,
        });
    }

    // -----------------------------------------------------------------
    // nets
    // -----------------------------------------------------------------

    fn net_pads(&self, net: u16) -> Vec<usize> {
        self.b
            .pads
            .iter()
            .enumerate()
            .filter(|(_, p)| p.net_id == net)
            .map(|(i, _)| i)
            .collect()
    }

    /// Prim order of pads by Manhattan distance.
    fn prim_order(&self, pads: &[usize]) -> Vec<usize> {
        if pads.is_empty() {
            return vec![];
        }
        let p = &self.b.pads;
        let mut done = vec![false; pads.len()];
        let mut order = vec![pads[0]];
        done[0] = true;
        let mut best = vec![f64::INFINITY; pads.len()];
        let mut last = 0;
        for _ in 1..pads.len() {
            let mut pick = usize::MAX;
            let mut pv = f64::INFINITY;
            for k in 0..pads.len() {
                if done[k] {
                    continue;
                }
                let d = (p[pads[k]].x - p[pads[last]].x).abs() + (p[pads[k]].y - p[pads[last]].y).abs();
                if d < best[k] {
                    best[k] = d;
                }
                if best[k] < pv {
                    pv = best[k];
                    pick = k;
                }
            }
            done[pick] = true;
            order.push(pads[pick]);
            last = pick;
        }
        order
    }

    fn pin_label(&self, pi: usize) -> String {
        let p = &self.b.pads[pi];
        format!("{}.{}", p.refdes, p.pin)
    }

    /// Route one net as a growing tree. Returns the status.
    fn route_net(&mut self, net: u16) -> NetRoute {
        let pads = self.net_pads(net);
        if !self.swap_nets.contains(&net) || self.pool_pad.is_empty() {
            return self.route_pad_tree(net, &pads);
        }
        // pin-swappable net: route the peripheral pins, then join the tree to
        // the best free SoC GPIO pad.
        let periph: Vec<usize> = pads.iter().copied().filter(|&p| self.pad_pool[p].is_none()).collect();
        let mut st = self.route_pad_tree(net, &periph);
        st.pins += 1;
        let tree = self.net_nodes(net, &periph);
        if tree.is_empty() {
            st.failed_pins.push("U1.(GPIO)".into());
            return st;
        }
        let (srcs, tag) = self.pool_access(net);
        let mode = if self.nc_mode { Mode::Negotiated } else { Mode::Normal };
        self.swap_search = true;
        let mut wave = self.recording.as_ref().map(|_| Wave {
            from_pin: "U1 (any free GPIO)".into(),
            ..Default::default()
        });
        let astar = wave.is_none();
        let path = self.search(net, &srcs, &tree, mode, true, astar, wave.as_mut());
        self.swap_search = false;
        if let (Some(w), Some(rec)) = (wave, self.recording.as_mut()) {
            rec.push(w);
        }
        let Some(path) = path else {
            st.failed_pins.push("U1.(GPIO)".into());
            return st;
        };
        let first = path[0];
        let k = tag[&first];
        // reject paths that wander through another free pad's clearance zone
        let n = self.n;
        let base = self.pool_base;
        let crosses = path.iter().any(|&node| {
            let s = self.l_trk[node];
            let j = s.wrapping_sub(base) as usize;
            s >= base && j < self.claim.len() && j != k && self.claim[j] != net
        });
        if crosses {
            st.failed_pins.push("U1.(GPIO)".into());
            return st;
        }
        self.claim[k] = net;
        if let Some(a) = srcs.iter().filter(|a| a.node == first).min_by_key(|a| a.cost) {
            if let Some(s) = a.stub {
                self.commit_stub(net, (first / n) as u8, s);
            }
        }
        let last = *path.last().unwrap();
        if let Some(Some(s)) = tree.get(&last).copied() {
            self.commit_stub(net, (last / n) as u8, s);
        }
        self.commit_path(net, &path);
        st.connected_pins += 1;
        st
    }

    /// Grid nodes already carrying `net` copper plus the access points of
    /// the given pads (with their stubs) — the targets for joining a tree.
    fn net_nodes(&self, net: u16, pads: &[usize]) -> HashMap<usize, Option<Stub>> {
        let mut m: HashMap<usize, Option<Stub>> = HashMap::new();
        let g = self.g;
        for t in self.tracks.iter().filter(|t| t.net_id == net) {
            let on = |v: f64| ((v / g) - (v / g).round()).abs() < 1e-6;
            if !(on(t.x0) && on(t.y0) && on(t.x1) && on(t.y1)) {
                if on(t.x1) && on(t.y1) {
                    let c = (t.y1 / g).round() as usize * self.nx + (t.x1 / g).round() as usize;
                    m.insert(t.layer as usize * self.n + c, None);
                }
                continue;
            }
            let (i0, j0) = ((t.x0 / g).round() as i64, (t.y0 / g).round() as i64);
            let (i1, j1) = ((t.x1 / g).round() as i64, (t.y1 / g).round() as i64);
            let steps = (i1 - i0).abs().max((j1 - j0).abs());
            let (di, dj) = ((i1 - i0).signum(), (j1 - j0).signum());
            for s in 0..=steps {
                let c = (j0 + dj * s) as usize * self.nx + (i0 + di * s) as usize;
                m.insert(t.layer as usize * self.n + c, None);
            }
        }
        for &pi in pads {
            let mode = if self.nc_mode { Mode::Relaxed } else { Mode::Normal };
            for a in self.access(pi, mode) {
                m.entry(a.node).or_insert(a.stub);
            }
        }
        m
    }

    /// Connect a set of pads of one net as a growing tree.
    fn route_pad_tree(&mut self, net: u16, pads: &[usize]) -> NetRoute {
        let name = self.b.nets[net as usize].name.clone();
        let pads = pads.to_vec();
        let mut st = NetRoute {
            net: name,
            pins: pads.len(),
            ..Default::default()
        };
        if pads.len() < 2 {
            st.connected_pins = pads.len();
            return st;
        }
        let allow_via = self.b.nets[net as usize].class != NetClass::Rf;
        let order = self.prim_order(&pads);
        let mut tree: HashMap<usize, Option<Stub>> = HashMap::new();
        let mut started = false;
        let mode = if self.nc_mode { Mode::Negotiated } else { Mode::Normal };
        for &pi in &order {
            let srcs = if self.nc_mode {
                self.nc_access(pi)
            } else {
                self.access(pi, Mode::Normal)
            };
            if srcs.is_empty() {
                st.failed_pins.push(self.pin_label(pi));
                continue;
            }
            if !started {
                for a in &srcs {
                    tree.insert(a.node, a.stub);
                }
                started = true;
                st.connected_pins += 1;
                continue;
            }
            let mut wave = self.recording.as_ref().map(|_| Wave {
                from_pin: self.pin_label(pi),
                ..Default::default()
            });
            let astar = wave.is_none();
            let path = self.search(net, &srcs, &tree, mode, allow_via, astar, wave.as_mut());
            if let (Some(w), Some(rec)) = (wave, self.recording.as_mut()) {
                rec.push(w);
            }
            match path {
                Some(path) => {
                    let first = path[0];
                    let last = *path.last().unwrap();
                    let n = self.n;
                    // source stub (cheapest access at that node)
                    if let Some(a) = srcs.iter().filter(|a| a.node == first).min_by_key(|a| a.cost) {
                        if let Some(s) = a.stub {
                            self.commit_stub(net, (first / n) as u8, s);
                        }
                    }
                    if let Some(Some(s)) = tree.get(&last).copied() {
                        self.commit_stub(net, (last / n) as u8, s);
                    }
                    self.commit_path(net, &path);
                    tree.insert(last, None);
                    for a in &srcs {
                        tree.entry(a.node).or_insert(a.stub);
                    }
                    tree.insert(first, None);
                    for &node in &path {
                        tree.insert(node, None);
                    }
                    st.connected_pins += 1;
                }
                None => st.failed_pins.push(self.pin_label(pi)),
            }
        }
        if !started {
            st.connected_pins = 0;
        }
        st
    }

    /// Nets whose copper lies on the cheapest path that ignores other signal nets.
    fn find_blockers(&mut self, net: u16) -> Vec<u16> {
        let pads = self.net_pads(net);
        let mut out: Vec<u16> = Vec::new();
        let order = self.prim_order(&pads);
        for w in order.windows(2) {
            let srcs = self.access(w[1], Mode::Relaxed);
            let tg: HashMap<usize, Option<Stub>> = self.access(w[0], Mode::Relaxed).into_iter().map(|a| (a.node, a.stub)).collect();
            let allow_via = self.b.nets[net as usize].class != NetClass::Rf;
            let Some(path) = self.search(net, &srcs, &tg, Mode::Relaxed, allow_via, true, None) else {
                continue;
            };
            let mut ids = Vec::new();
            for &node in &path {
                if ok(self.trk[node], net) {
                    continue;
                }
                let layer = node / self.n;
                let (x, y) = self.xy(node % self.n);
                self.d_hash.query([x - 1.0, y - 1.0, x + 1.0, y + 1.0], &mut ids);
                for &i in &ids {
                    if self.is_locked_obj(i) {
                        continue;
                    }
                    let o = &self.dobjs[i];
                    let on = if layer == 0 { o.top } else { o.bot };
                    if on && o.net != net && o.shape.dist_point(x, y) < self.r_via && !out.contains(&o.net) {
                        out.push(o.net);
                    }
                }
            }
        }
        out
    }

    /// Place ground vias next to every surface-mount GND pad ("fan-out"), so
    /// ground reaches the bottom-layer pour through the shortest possible path.
    pub fn gnd_fanout(&mut self, thermal_only: bool) {
        let gnd = self.b.gnd;
        if gnd == u16::MAX {
            return;
        }
        let mut pads: Vec<usize> = self
            .net_pads(gnd)
            .into_iter()
            .filter(|&i| self.b.pads[i].is_smd() && self.b.pads[i].top)
            .collect();
        pads.sort_by(|&a, &b| {
            let pa = &self.b.pads[a];
            let pb = &self.b.pads[b];
            (pb.w * pb.h).partial_cmp(&(pa.w * pa.h)).unwrap().then(a.cmp(&b))
        });
        let drill = self.b.rules.via_drill_mm;
        let vpad = self.b.rules.via_pad_mm;
        let mut done: Vec<usize> = Vec::new();
        for pi in pads {
            let pd = self.b.pads[pi].clone();
            let area = pd.w * pd.h;
            if area >= 1.0 && pd.w.min(pd.h) >= vpad + 0.1 {
                // thermal pad: array of vias inside the pad
                let k = if pd.w.min(pd.h) >= 3.0 { 2 } else { 1 };
                // compact array (pitch = 2 via pads) keeps bottom channels open under the part
                let pitch = 2.0 * vpad;
                let mut placed = 0;
                for a in 0..k {
                    for c in 0..k {
                        let x = pd.x + (a as f64 - (k as f64 - 1.0) / 2.0) * pitch;
                        let y = pd.y + (c as f64 - (k as f64 - 1.0) / 2.0) * pitch;
                        if self.via_clear(x, y, gnd, Some(pi)) {
                            self.add_via(Via {
                                net: "GND".into(),
                                net_id: gnd,
                                x,
                                y,
                                drill,
                                pad: vpad,
                            });
                            placed += 1;
                        }
                    }
                }
                if placed > 0 {
                    done.push(pi);
                    self.gnd_done.push(pi);
                    continue;
                }
            }
            if thermal_only {
                continue;
            }
            let sh = pd.shape_geom();
            let clamp_in = |x: f64, y: f64| -> (f64, f64) {
                let sx = (pd.w / 2.0 - self.w / 2.0).max(0.0);
                let sy = (pd.h / 2.0 - self.w / 2.0).max(0.0);
                (x.clamp(pd.x - sx, pd.x + sx), y.clamp(pd.y - sy, pd.y + sy))
            };
            let mut best: Option<FanoutChoice> = None;
            // 1) join an already fanned-out GND pad or via nearby
            for &oi in &done {
                let o = &self.b.pads[oi];
                let osh = o.shape_geom();
                let (tx, ty) = {
                    let sx = (o.w / 2.0 - self.w / 2.0).max(0.0);
                    let sy = (o.h / 2.0 - self.w / 2.0).max(0.0);
                    (pd.x.clamp(o.x - sx, o.x + sx), pd.y.clamp(o.y - sy, o.y + sy))
                };
                if osh.dist_point(pd.x, pd.y) > 1.0 {
                    continue;
                }
                let (qx, qy) = clamp_in(tx, ty);
                let s = Stub {
                    x0: qx,
                    y0: qy,
                    x1: tx,
                    y1: ty,
                };
                let len = (tx - qx).hypot(ty - qy);
                if self.seg_clear(0, gnd, s, true) && best.as_ref().map(|b| len - 0.6 < b.0).unwrap_or(true) {
                    best = Some((len - 0.6, s, None));
                }
            }
            for v in self.vias.iter().filter(|v| v.net_id == gnd) {
                if sh.dist_point(v.x, v.y) > 1.3 {
                    continue;
                }
                let (qx, qy) = clamp_in(v.x, v.y);
                let s = Stub {
                    x0: qx,
                    y0: qy,
                    x1: v.x,
                    y1: v.y,
                };
                let len = (v.x - qx).hypot(v.y - qy);
                if self.seg_clear(0, gnd, s, true) && best.as_ref().map(|b| len - 0.3 < b.0).unwrap_or(true) {
                    best = Some((len - 0.3, s, None));
                }
            }
            // 2) a new via on a grid point within 1.4 mm
            let g = self.g;
            let bb = sh.bbox();
            let r = 1.4;
            let i0 = ((bb[0] - r) / g).floor().max(0.0) as usize;
            let j0 = ((bb[1] - r) / g).floor().max(0.0) as usize;
            let i1 = (((bb[2] + r) / g).ceil().max(0.0) as usize).min(self.nx - 1);
            let j1 = (((bb[3] + r) / g).ceil().max(0.0) as usize).min(self.ny - 1);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let c = j * self.nx + i;
                    if !ok(self.via[c], gnd) || !ok(self.trk[c], gnd) || !ok(self.trk[self.n + c], gnd) {
                        continue;
                    }
                    let (x, y) = (i as f64 * g, j as f64 * g);
                    let d = sh.dist_point(x, y);
                    if d > r {
                        continue;
                    }
                    let (qx, qy) = clamp_in(x, y);
                    let s = Stub {
                        x0: qx,
                        y0: qy,
                        x1: x,
                        y1: y,
                    };
                    let len = (x - qx).hypot(y - qy);
                    // penalise vias that sit in front of other nets' pads (they block escapes)
                    let mut crowd = 0.0;
                    for o in self.b.pads.iter() {
                        if o.net_id == gnd || !o.top {
                            continue;
                        }
                        let dd = o.shape_geom().dist_point(x, y);
                        if dd < 1.1 {
                            crowd += 1.1 - dd;
                        }
                    }
                    let score = len + 1.5 * crowd;
                    if best.as_ref().map(|b| score < b.0).unwrap_or(true)
                        && self.seg_clear(0, gnd, s, true)
                        && self.via_clear(x, y, gnd, None)
                    {
                        best = Some((score, s, Some((x, y))));
                    }
                }
            }
            match best {
                Some((_, s, v)) => {
                    if (s.x1 - s.x0).hypot(s.y1 - s.y0) > 1e-9 {
                        self.commit_stub(gnd, 0, s);
                    }
                    if let Some((x, y)) = v {
                        self.add_via(Via {
                            net: "GND".into(),
                            net_id: gnd,
                            x,
                            y,
                            drill,
                            pad: vpad,
                        });
                    }
                    done.push(pi);
                }
                None => self.gnd_fanout_failed.push(pi),
            }
        }
        self.lock();
    }

    /// Route all signal/power nets in the given order with rip-up and re-route.
    pub fn route_all(&mut self, order: &[u16], passes: usize) {
        let mut status: HashMap<u16, NetRoute> = HashMap::new();
        for &net in order {
            let s = self.route_net(net);
            status.insert(net, s);
        }
        let score = |st: &HashMap<u16, NetRoute>| st.values().map(|s| s.connected_pins.min(s.pins)).sum::<usize>();
        let mut best = (score(&status), self.tracks.clone(), self.vias.clone(), status.clone());
        let mut ripped: HashMap<u16, u32> = HashMap::new();
        for _pass in 0..passes {
            let failed: Vec<u16> = order.iter().copied().filter(|n| !status[n].failed_pins.is_empty()).collect();
            if failed.is_empty() {
                break;
            }
            let mut progress = false;
            for f in failed {
                if status[&f].failed_pins.is_empty() {
                    continue;
                }
                let mut blockers = self.find_blockers(f);
                blockers.retain(|b| *ripped.get(b).unwrap_or(&0) < 4);
                if blockers.is_empty() {
                    continue;
                }
                progress = true;
                self.ripups_total += blockers.len() as u32;
                for &b in &blockers {
                    *ripped.entry(b).or_insert(0) += 1;
                    self.rip_up(b);
                }
                self.rip_up(f);
                self.rebuild();
                let s = self.route_net(f);
                status.insert(f, s);
                for &b in &blockers {
                    let mut s = self.route_net(b);
                    s.ripups = *ripped.get(&b).unwrap_or(&0);
                    status.insert(b, s);
                }
                let sc = score(&status);
                if sc > best.0 {
                    best = (sc, self.tracks.clone(), self.vias.clone(), status.clone());
                }
            }
            if !progress {
                break;
            }
        }
        if score(&status) < best.0 {
            self.tracks = best.1;
            self.vias = best.2;
            status = best.3;
            self.rebuild();
        }
        let mut st: Vec<NetRoute> = order.iter().map(|n| status.remove(n).unwrap()).collect();
        for s in st.iter_mut() {
            if let Some(r) = ripped.get(&(self.b.nets.iter().position(|x| x.name == s.net).unwrap_or(0) as u16)) {
                s.ripups = *r;
            }
        }
        self.status = st;
    }

    /// Bridge two ground groups: search from any grid node of the source set
    /// to any node of the target set on the GND net and commit the path.
    pub fn stitch(&mut self, src: &[usize], tgt: &[usize]) -> bool {
        let gnd = self.b.gnd;
        let sources: Vec<Access> = src
            .iter()
            .filter(|&&n| ok(self.trk[n], gnd))
            .map(|&node| Access { node, cost: 0, stub: None })
            .collect();
        let targets: HashMap<usize, Option<Stub>> = tgt.iter().filter(|&&n| ok(self.trk[n], gnd)).map(|&n| (n, None)).collect();
        match self.search(gnd, &sources, &targets, Mode::Normal, true, true, None) {
            Some(path) if path.len() >= 2 => {
                self.commit_path(gnd, &path);
                true
            }
            _ => false,
        }
    }

    /// Join a stranded ground group to the main one by rip-up and re-route:
    /// find the cheapest bridge that ignores other nets' (unlocked) copper,
    /// rip the nets on it, commit the bridge strictly and re-route the ripped
    /// nets. Kept only if every ripped net routes again; otherwise the copper
    /// is restored. `src`/`tgt` are grid nodes of the two groups, `pads` the
    /// group's surface pads (entered through their access stubs).
    pub fn ground_bridge_repair(&mut self, src: &[usize], pads: &[usize], tgt: &[usize]) -> bool {
        let gnd = self.b.gnd;
        let mk_sources = |rt: &Router, mode: Mode| -> Vec<Access> {
            let arr = if mode == Mode::Relaxed { &rt.l_trk } else { &rt.trk };
            let mut v: Vec<Access> = src
                .iter()
                .filter(|&&n| ok(arr[n], gnd))
                .map(|&node| Access { node, cost: 0, stub: None })
                .collect();
            for &pi in pads {
                v.extend(rt.access(pi, mode));
            }
            v
        };
        let srcs = mk_sources(self, Mode::Relaxed);
        let targets: HashMap<usize, Option<Stub>> = tgt.iter().filter(|&&n| ok(self.l_trk[n], gnd)).map(|&n| (n, None)).collect();
        let Some(path) = self.search(gnd, &srcs, &targets, Mode::Relaxed, true, true, None) else {
            return false;
        };
        // nets whose unlocked copper lies on the relaxed bridge: within track
        // clearance of each step, within via clearance of each layer change
        let mut blockers: Vec<u16> = Vec::new();
        let mut ids = Vec::new();
        for k in 0..path.len() {
            let node = path[k];
            let layer = node / self.n;
            let (x, y) = self.xy(node % self.n);
            let next = path.get(k + 1).copied();
            let is_via = next.map(|m| m % self.n == node % self.n).unwrap_or(false);
            let (nx_, ny_) = match next {
                Some(m) if !is_via => self.xy(m % self.n),
                _ => (x, y),
            };
            self.d_hash.query([x - 1.0, y - 1.0, x + 1.0, y + 1.0], &mut ids);
            for &i in &ids {
                if self.is_locked_obj(i) {
                    continue;
                }
                let o = &self.dobjs[i];
                if o.net == gnd || blockers.contains(&o.net) {
                    continue;
                }
                let hit = if is_via {
                    o.shape.dist_point(x, y) < self.r_via - EPS
                } else {
                    (if layer == 0 { o.top } else { o.bot }) && o.shape.dist_seg(x, y, nx_, ny_) < self.r_trk - EPS
                };
                if hit {
                    blockers.push(o.net);
                }
            }
        }
        // the access stub the bridge starts from may cross other copper too
        if let Some(a) = srcs.iter().filter(|a| a.node == path[0]).min_by_key(|a| a.cost) {
            if let Some(st) = a.stub {
                let layer = path[0] / self.n;
                let bb = [
                    st.x0.min(st.x1) - 1.0,
                    st.y0.min(st.y1) - 1.0,
                    st.x0.max(st.x1) + 1.0,
                    st.y0.max(st.y1) + 1.0,
                ];
                self.d_hash.query(bb, &mut ids);
                for &i in &ids {
                    if self.is_locked_obj(i) {
                        continue;
                    }
                    let o = &self.dobjs[i];
                    let on = if layer == 0 { o.top } else { o.bot };
                    if on && o.net != gnd && o.shape.dist_seg(st.x0, st.y0, st.x1, st.y1) < self.r_trk && !blockers.contains(&o.net) {
                        blockers.push(o.net);
                    }
                }
            }
        }
        if blockers.len() > 4 {
            return false;
        }
        let snap = (self.tracks.clone(), self.vias.clone(), self.claim.clone(), self.status.clone());
        let mut ok_all = false;
        // two sequences: bridge first (it gets the contested channel), or the
        // ripped nets first with the bridge squeezing through what is left
        for bridge_first in [true, false] {
            for &bl in &blockers {
                self.rip_up(bl);
            }
            self.rebuild();
            let mut sts = Vec::new();
            let mut good = true;
            for step in 0..2 {
                if (step == 0) == bridge_first {
                    let srcs = mk_sources(self, Mode::Normal);
                    let targets: HashMap<usize, Option<Stub>> = tgt.iter().filter(|&&n| ok(self.trk[n], gnd)).map(|&n| (n, None)).collect();
                    match self.search(gnd, &srcs, &targets, Mode::Normal, true, true, None) {
                        Some(path) => {
                            let first = path[0];
                            if let Some(a) = srcs.iter().filter(|a| a.node == first).min_by_key(|a| a.cost) {
                                if let Some(st) = a.stub {
                                    self.commit_stub(gnd, (first / self.n) as u8, st);
                                }
                            }
                            if path.len() >= 2 {
                                self.commit_path(gnd, &path);
                            }
                        }
                        None => good = false,
                    }
                } else {
                    for &bl in &blockers {
                        let st = self.route_net(bl);
                        if !st.failed_pins.is_empty() {
                            good = false;
                            break;
                        }
                        sts.push(st);
                    }
                }
                if !good {
                    break;
                }
            }
            if good {
                for st in sts {
                    if let Some(x) = self.status.iter_mut().find(|x| x.net == st.net) {
                        let r = x.ripups + 1;
                        *x = st;
                        x.ripups = r;
                    }
                }
                self.ripups_total += blockers.len() as u32;
                ok_all = true;
                break;
            }
            self.tracks = snap.0.clone();
            self.vias = snap.1.clone();
            self.claim = snap.2.clone();
            self.status = snap.3.clone();
            self.rebuild();
        }
        ok_all
    }

    /// Grid nodes that belong to a pad (access points without stubs are
    /// used directly; stubbed ones are committed when chosen).
    pub fn pad_nodes(&self, pi: usize) -> Vec<usize> {
        self.access(pi, Mode::Normal)
            .into_iter()
            .filter(|a| a.stub.is_none())
            .map(|a| a.node)
            .collect()
    }

    pub fn grid_n(&self) -> usize {
        self.n
    }

    /// Add a ground stitching via at a grid point if it fits.
    pub fn try_gnd_via(&mut self, c: usize) -> bool {
        let gnd = self.b.gnd;
        if self.via_near[c] > 0 || !ok(self.via[c], gnd) || !ok(self.trk[c], gnd) || !ok(self.trk[self.n + c], gnd) {
            return false;
        }
        let (x, y) = self.xy(c);
        if !self.via_clear(x, y, gnd, None) {
            return false;
        }
        self.add_via(Via {
            net: self.b.nets[gnd as usize].name.clone(),
            net_id: gnd,
            x,
            y,
            drill: self.b.rules.via_drill_mm,
            pad: self.b.rules.via_pad_mm,
        });
        true
    }

    /// Connect a ground pad (with its access stub) to any of the target nodes.
    pub fn connect_pad(&mut self, pi: usize, tgt: &[usize]) -> bool {
        let gnd = self.b.gnd;
        let srcs = self.access(pi, Mode::Normal);
        let targets: HashMap<usize, Option<Stub>> = tgt.iter().filter(|&&n| ok(self.trk[n], gnd)).map(|&n| (n, None)).collect();
        match self.search(gnd, &srcs, &targets, Mode::Normal, true, true, None) {
            Some(path) => {
                let first = path[0];
                if let Some(a) = srcs.iter().filter(|a| a.node == first).min_by_key(|a| a.cost) {
                    if let Some(s) = a.stub {
                        self.commit_stub(gnd, (first / self.n) as u8, s);
                    }
                }
                if path.len() >= 2 {
                    self.commit_path(gnd, &path);
                }
                true
            }
            None => false,
        }
    }

    // -----------------------------------------------------------------
    // negotiated congestion (PathFinder)
    // -----------------------------------------------------------------

    /// Visit every routing resource a copper object influences.
    fn influence(&self, o: &Obj, mut f: impl FnMut(u8, usize)) {
        let (rt, rv) = (self.r_trk - EPS, self.r_via - EPS);
        let (nx, n, g) = (self.nx, self.n, self.g);
        let bb = o.shape.bbox();
        let rr = rt.max(rv) + g;
        let i0 = ((bb[0] - rr) / g).floor().max(0.0) as usize;
        let j0 = ((bb[1] - rr) / g).floor().max(0.0) as usize;
        let i1 = (((bb[2] + rr) / g).ceil().max(0.0) as usize).min(self.nx - 1);
        let j1 = (((bb[3] + rr) / g).ceil().max(0.0) as usize).min(self.ny - 1);
        for j in j0..=j1 {
            for i in i0..=i1 {
                let c = j * nx + i;
                let (x, y) = (i as f64 * g, j as f64 * g);
                let d = o.shape.dist_point(x, y);
                if d > rr + g {
                    continue;
                }
                for (layer, on) in [(0usize, o.top), (1usize, o.bot)] {
                    if !on {
                        continue;
                    }
                    if d < rt {
                        f(0, layer * n + c);
                    }
                    if i + 1 < nx && o.shape.dist_seg(x, y, x + g, y) < rt {
                        f(1, (layer * n + c) * 2);
                    }
                    if j + 1 < self.ny && o.shape.dist_seg(x, y, x, y + g) < rt {
                        f(1, (layer * n + c) * 2 + 1);
                    }
                }
                if d < rv {
                    f(2, c);
                }
            }
        }
    }

    fn occ_apply(&mut self, o: &Obj, add: bool) {
        let mut idx: Vec<(u8, usize)> = Vec::new();
        self.influence(o, |k, i| idx.push((k, i)));
        for (k, i) in idx {
            let cell = match k {
                0 => &mut self.occ_trk[i],
                1 => &mut self.occ_edg[i],
                _ => &mut self.occ_via[i],
            };
            if add {
                cell.add(o.net);
            } else {
                cell.remove(o.net);
            }
        }
    }

    /// Access points priced against other nets' copper instead of excluded.
    fn nc_access(&mut self, pi: usize) -> Vec<Access> {
        self.nc_access_inner(pi)
    }

    /// Access points valid against static + locked copper (cached: those never
    /// change while nets negotiate).
    fn relaxed_access(&mut self, pi: usize, net: u16) -> Vec<Access> {
        if let Some(v) = self.access_cache.get(&(pi, net)) {
            return v.clone();
        }
        let v = self.access_as(pi, Mode::Relaxed, net);
        self.access_cache.insert((pi, net), v.clone());
        v
    }

    /// Occupancy cost of a stub, sampled at grid points along it.
    fn stub_cost(&self, layer: usize, net: u16, s: Stub) -> u32 {
        let len = (s.x1 - s.x0).hypot(s.y1 - s.y0);
        let k = ((len / self.g).ceil() as usize).max(1);
        let mut o = 0;
        for t in 0..=k {
            let f = t as f64 / k as f64;
            let (x, y) = (s.x0 + (s.x1 - s.x0) * f, s.y0 + (s.y1 - s.y0) * f);
            let (i, j) = ((x / self.g).round() as usize, (y / self.g).round() as usize);
            if i < self.nx && j < self.ny {
                o = o.max(self.occ_trk[layer * self.n + j * self.nx + i].others(net));
            }
        }
        o
    }

    fn nc_access_inner(&mut self, pi: usize) -> Vec<Access> {
        let net = self.b.pads[pi].net_id;
        let mut v = self.relaxed_access(pi, net);
        for a in v.iter_mut() {
            a.cost += self.pres * self.occ_trk[a.node].others(net) + self.hist_trk[a.node];
            if let Some(s) = a.stub {
                a.cost += 2 * self.pres * self.stub_cost(a.node / self.n, net, s);
            }
        }
        v
    }

    /// Entry points on every pool pad `net` could claim, tagged with the pool index.
    fn pool_access(&mut self, net: u16) -> (Vec<Access>, HashMap<usize, usize>) {
        let mut v = Vec::new();
        let mut tag: HashMap<usize, usize> = HashMap::new();
        let mode = if self.nc_mode { Mode::Relaxed } else { Mode::Normal };
        for k in 0..self.pool_pad.len() {
            if self.claim[k] != FREE && self.claim[k] != net {
                continue;
            }
            let pi = self.pool_pad[k];
            let prev = self.claim[k];
            self.claim[k] = net; // the pad is ours while we look at it
            let mut acc = if self.nc_mode {
                self.relaxed_access(pi, net)
            } else {
                self.access_as(pi, mode, net)
            };
            if self.nc_mode {
                for a in acc.iter_mut() {
                    a.cost += self.pres * self.occ_trk[a.node].others(net) + self.hist_trk[a.node];
                    if let Some(s) = a.stub {
                        a.cost += 2 * self.pres * self.stub_cost(a.node / self.n, net, s);
                    }
                }
            }
            self.claim[k] = prev;
            for a in acc {
                match tag.get(&a.node) {
                    Some(_) => {}
                    None => {
                        tag.insert(a.node, k);
                        v.push(a);
                    }
                }
            }
        }
        (v, tag)
    }

    fn release_claims(&mut self, net: u16) {
        for c in self.claim.iter_mut() {
            if *c == net {
                *c = FREE;
            }
        }
    }

    fn nc_rip(&mut self, net: u16) {
        self.release_claims(net);
        self.nc_rip_inner(net);
    }

    fn nc_rip_inner(&mut self, net: u16) {
        let lt = self.locked_tracks;
        let lv = self.locked_vias;
        let objs: Vec<Obj> = self
            .tracks
            .iter()
            .enumerate()
            .filter(|(k, t)| *k >= lt && t.net_id == net)
            .map(|(_, t)| Self::obj_of_track(t))
            .chain(
                self.vias
                    .iter()
                    .enumerate()
                    .filter(|(k, v)| *k >= lv && v.net_id == net)
                    .map(|(_, v)| Self::obj_of_via(v)),
            )
            .collect();
        for o in &objs {
            self.occ_apply(o, false);
        }
        self.rip_up(net);
        self.recompute_vias();
    }

    fn refresh_dyn_index(&mut self) {
        self.dobjs = self
            .tracks
            .iter()
            .map(Self::obj_of_track)
            .chain(self.vias.iter().map(Self::obj_of_via))
            .collect();
        self.d_hash = SpatialHash::new(0.0, 0.0, self.b.width, self.b.height, 1.0);
        for (i, o) in self.dobjs.iter().enumerate() {
            self.d_hash.insert(i, o.shape.bbox());
        }
    }

    /// Nets whose copper overlaps another net's clearance zone; bumps the
    /// history cost of every contested resource.
    fn nc_conflicts(&mut self) -> Vec<u16> {
        self.refresh_dyn_index();
        self.nc_conflicts_for(None, true)
    }

    fn nc_conflicts_for(&mut self, only: Option<u16>, bump: bool) -> Vec<u16> {
        let mut bad: Vec<u16> = Vec::new();
        let g = self.g;
        let n = self.n;
        for ti in self.locked_tracks..self.tracks.len() {
            let t = self.tracks[ti].clone();
            let net = t.net_id;
            if only.map(|o| o != net).unwrap_or(false) {
                continue;
            }
            let layer = t.layer as usize;
            let on_grid = |v: f64| ((v / g) - (v / g).round()).abs() < 1e-6;
            let axis = (t.x0 - t.x1).abs() < 1e-9 || (t.y0 - t.y1).abs() < 1e-9;
            let mut conflict = false;
            if axis && on_grid(t.x0) && on_grid(t.y0) && on_grid(t.x1) && on_grid(t.y1) {
                let (i0, j0) = ((t.x0 / g).round() as i64, (t.y0 / g).round() as i64);
                let (i1, j1) = ((t.x1 / g).round() as i64, (t.y1 / g).round() as i64);
                let steps = (i1 - i0).abs().max((j1 - j0).abs());
                let (di, dj) = ((i1 - i0).signum(), (j1 - j0).signum());
                for s in 0..=steps {
                    let (i, j) = (i0 + di * s, j0 + dj * s);
                    let c = j as usize * self.nx + i as usize;
                    let node = layer * n + c;
                    if self.occ_trk[node].others(net) > 0 {
                        if bump {
                            self.hist_trk[node] += HIST_INC;
                        }
                        conflict = true;
                    }
                    if s < steps {
                        let e = if di > 0 {
                            (layer * n + c) * 2
                        } else if di < 0 {
                            (layer * n + c - 1) * 2
                        } else if dj > 0 {
                            (layer * n + c) * 2 + 1
                        } else {
                            (layer * n + c - self.nx) * 2 + 1
                        };
                        if self.occ_edg[e].others(net) > 0 {
                            if bump {
                                self.hist_edg[e] += HIST_INC;
                            }
                            conflict = true;
                        }
                    }
                }
            } else {
                let s = Stub {
                    x0: t.x0,
                    y0: t.y0,
                    x1: t.x1,
                    y1: t.y1,
                };
                if !self.seg_clear(t.layer, net, s, true) {
                    conflict = true;
                    let (i, j) = ((t.x1 / g).round() as usize, (t.y1 / g).round() as usize);
                    if i < self.nx && j < self.ny && bump {
                        self.hist_trk[layer * n + j * self.nx + i] += HIST_INC;
                    }
                }
            }
            if conflict && !bad.contains(&net) {
                bad.push(net);
            }
        }
        for vi in self.locked_vias..self.vias.len() {
            let v = &self.vias[vi];
            if only.map(|o| o != v.net_id).unwrap_or(false) {
                continue;
            }
            let (i, j) = ((v.x / g).round() as usize, (v.y / g).round() as usize);
            if i >= self.nx || j >= self.ny {
                continue;
            }
            let c = j * self.nx + i;
            if self.occ_via[c].others(v.net_id) > 0 || self.occ_trk[c].others(v.net_id) > 0 || self.occ_trk[n + c].others(v.net_id) > 0 {
                if bump {
                    self.hist_via[c] += HIST_INC;
                }
                if !bad.contains(&v.net_id) {
                    bad.push(v.net_id);
                }
            }
        }
        bad
    }

    /// Drop every surface-mount ground pad to the bottom layer (ground pour)
    /// through the cheapest nearby via — the router's version of "fan-out".
    fn route_gnd_pads(&mut self) -> NetRoute {
        let gnd = self.b.gnd;
        // Ground backbone: battery return, USB return, thermal pads and one
        // display-connector ground are joined by real copper so no pour
        // island can strand them.
        let mut backbone: Vec<usize> = Vec::new();
        let gp: Vec<usize> = self.net_pads(gnd);
        for &pi in &gp {
            let p = &self.b.pads[pi];
            let first_of_ref = gp.iter().find(|&&q| self.b.pads[q].refdes == p.refdes) == Some(&pi);
            let is_conn = p.refdes.starts_with('J') && p.is_smd() && first_of_ref;
            if is_conn || self.gnd_done.contains(&pi) {
                backbone.push(pi);
            }
        }
        let bb = if backbone.len() >= 2 {
            Some(self.route_pad_tree(gnd, &backbone))
        } else {
            None
        };
        let pads: Vec<usize> = self
            .b
            .pads
            .iter()
            .enumerate()
            .filter(|(i, p)| p.net_id == gnd && p.is_smd() && p.top && !p.bottom && !self.gnd_done.contains(i))
            .map(|(i, _)| i)
            .collect();
        let mut st = NetRoute {
            net: self.b.nets[gnd as usize].name.clone(),
            pins: pads.len(),
            ..Default::default()
        };
        self.target_layer = Some(1);
        let mode = if self.nc_mode { Mode::Negotiated } else { Mode::Normal };
        let empty: HashMap<usize, Option<Stub>> = HashMap::new();
        for pi in pads {
            let srcs = if self.nc_mode {
                self.nc_access(pi)
            } else {
                self.access(pi, Mode::Normal)
            };
            match self.search(gnd, &srcs, &empty, mode, true, false, None) {
                Some(path) => {
                    let first = path[0];
                    if let Some(a) = srcs.iter().filter(|a| a.node == first).min_by_key(|a| a.cost) {
                        if let Some(s) = a.stub {
                            self.commit_stub(gnd, (first / self.n) as u8, s);
                        }
                    }
                    if path.len() >= 2 {
                        self.commit_path(gnd, &path);
                    }
                    st.connected_pins += 1;
                }
                None => st.failed_pins.push(self.pin_label(pi)),
            }
        }
        self.target_layer = None;
        if let Some(b) = bb {
            st.pins += b.pins;
            st.connected_pins += b.connected_pins;
            st.failed_pins.extend(b.failed_pins);
        }
        st
    }

    /// PathFinder-style negotiated congestion routing (McMurchie & Ebeling,
    /// FPGA 1995): every net is routed even if it has to share resources, the
    /// price of sharing rises each iteration (present congestion) and
    /// contested resources remember it (history), until nets negotiate a
    /// legal solution. Leftover conflicts are resolved by a strict pass.
    pub fn route_negotiated(&mut self, order: &[u16], iterations: usize) {
        // occupancy from the locked copper is already in the l_* arrays.
        self.nc_mode = true;
        self.pres = 3;
        self.dobjs = self
            .tracks
            .iter()
            .map(Self::obj_of_track)
            .chain(self.vias.iter().map(Self::obj_of_via))
            .collect();
        let mut status: HashMap<u16, NetRoute> = HashMap::new();
        let gnd = self.b.gnd;
        let with_gnd: Vec<u16> = if gnd != u16::MAX {
            std::iter::once(gnd).chain(order.iter().copied()).collect()
        } else {
            order.to_vec()
        };
        let order: &[u16] = &with_gnd;
        let mut todo: Vec<u16> = order.to_vec();
        let mut best: Option<NcSnapshot> = None;
        for _it in 0..iterations {
            for &net in &todo {
                self.nc_rip(net);
                let s = if net == gnd { self.route_gnd_pads() } else { self.route_net(net) };
                status.insert(net, s);
            }
            let bad = self.nc_conflicts();
            self.nc_history.push(bad.len());
            if best.as_ref().map(|b| bad.len() < b.0).unwrap_or(true) {
                best = Some((
                    bad.len(),
                    self.tracks.clone(),
                    self.vias.clone(),
                    self.claim.clone(),
                    status.clone(),
                ));
            }
            if bad.is_empty() {
                break;
            }
            // classic PathFinder: every net is ripped and re-routed each
            // iteration so nets that merely *block* others move too.
            todo = order.to_vec();
            self.pres = (self.pres as f64 * 1.35 + 2.0).min(700.0) as u32;
        }
        // strict clean-up: drop nets still in conflict, rebuild hard occupancy, re-route them strictly
        // restore the least-conflicted iteration
        if let Some((_, t, v, c, s)) = best {
            if self.nc_history.last().copied().unwrap_or(0) > 0 {
                self.tracks = t;
                self.vias = v;
                self.claim = c;
                status = s;
                for o in self.occ_trk.iter_mut() {
                    *o = Occ::default();
                }
                for o in self.occ_edg.iter_mut() {
                    *o = Occ::default();
                }
                for o in self.occ_via.iter_mut() {
                    *o = Occ::default();
                }
                let objs: Vec<Obj> = self.tracks[self.locked_tracks..]
                    .iter()
                    .map(Self::obj_of_track)
                    .chain(self.vias[self.locked_vias..].iter().map(Self::obj_of_via))
                    .collect();
                for o in &objs {
                    self.occ_apply(o, true);
                }
                self.recompute_vias();
            }
        }
        // Resolve what is left: repeatedly rip the *smallest* conflicting net
        // (fewest pins, then shortest) until the copper is conflict-free, so
        // big nets such as the 1.8 V rail keep their negotiated routes.
        let mut bad: Vec<u16> = Vec::new();
        loop {
            let conf = self.nc_conflicts();
            if conf.is_empty() {
                break;
            }
            let pick = conf
                .iter()
                .copied()
                .min_by_key(|&n| {
                    let pins = self.b.pads.iter().filter(|p| p.net_id == n).count();
                    let len: f64 = self.tracks.iter().filter(|t| t.net_id == n).map(|t| t.length()).sum();
                    (pins, (len * 100.0) as i64)
                })
                .unwrap();
            self.nc_rip(pick);
            bad.push(pick);
        }
        self.nc_mode = false;
        self.rebuild();
        for &net in &bad {
            let s = if net == gnd { self.route_gnd_pads() } else { self.route_net(net) };
            status.insert(net, s);
        }
        let mut st: Vec<NetRoute> = Vec::new();
        for &net in order {
            st.push(status.remove(&net).unwrap_or_default());
        }
        self.status = st;
        // final rip-up & re-route for whatever is still open
        let open: Vec<u16> = order
            .iter()
            .copied()
            .filter(|n| {
                self.status
                    .iter()
                    .any(|s| s.net == self.b.nets[*n as usize].name && !s.failed_pins.is_empty())
            })
            .collect();
        if !open.is_empty() {
            let prev = std::mem::take(&mut self.status);
            self.repair(order, &open, prev);
        }
    }

    /// Strict rip-up-and-re-route of the given open nets.
    fn repair(&mut self, order: &[u16], open: &[u16], prev: Vec<NetRoute>) {
        let mut status: HashMap<u16, NetRoute> = HashMap::new();
        for (k, &net) in order.iter().enumerate() {
            status.insert(net, prev.get(k).cloned().unwrap_or_default());
        }
        let score = |st: &HashMap<u16, NetRoute>| st.values().map(|s| s.connected_pins.min(s.pins)).sum::<usize>();
        let mut best = (
            score(&status),
            self.tracks.clone(),
            self.vias.clone(),
            status.clone(),
            self.claim.clone(),
        );
        let mut ripped: HashMap<u16, u32> = HashMap::new();
        let _ = open;
        for _pass in 0..4 {
            let failed: Vec<u16> = order
                .iter()
                .copied()
                .filter(|n| *n != self.b.gnd && !status[n].failed_pins.is_empty())
                .collect();
            if failed.is_empty() {
                break;
            }
            let mut progress = false;
            for f in failed {
                let mut blockers = self.find_blockers(f);
                blockers.retain(|b| *b != self.b.gnd && *ripped.get(b).unwrap_or(&0) < 3);
                if blockers.is_empty() {
                    continue;
                }
                progress = true;
                self.ripups_total += blockers.len() as u32;
                for &b in &blockers {
                    *ripped.entry(b).or_insert(0) += 1;
                    self.rip_up(b);
                }
                self.rip_up(f);
                self.rebuild();
                let s = self.route_net(f);
                status.insert(f, s);
                for &b in &blockers {
                    let s = self.route_net(b);
                    status.insert(b, s);
                }
                let sc = score(&status);
                if sc > best.0 {
                    best = (sc, self.tracks.clone(), self.vias.clone(), status.clone(), self.claim.clone());
                }
            }
            if !progress {
                break;
            }
        }
        if score(&status) < best.0 {
            self.tracks = best.1;
            self.vias = best.2;
            status = best.3;
            self.claim = best.4;
            self.rebuild();
        }
        self.status = order.iter().map(|n| status.remove(n).unwrap_or_default()).collect();
    }

    /// Re-route one net from scratch, recording Lee's wavefront for each
    /// connection (Dijkstra without heuristic so the flood is visible).
    pub fn trace_net(&mut self, net: u16) -> WaveTrace {
        self.rip_up(net);
        self.rebuild();
        self.recording = Some(Vec::new());
        let _ = self.route_net(net);
        let waves = self.recording.take().unwrap_or_default();
        WaveTrace {
            net: self.b.nets[net as usize].name.clone(),
            nx: self.nx,
            ny: self.ny,
            grid_mm: self.g,
            waves,
        }
    }
}
