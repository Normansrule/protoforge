//! Component placement.
//!
//! 1. **Constraint-driven constructive placement** (deterministic): the
//!    USB-C receptacle goes to the rear edge, the display FPC connector and the
//!    battery connector to the hinge end, the SoC next to them with the chip
//!    antenna on the outer (top) edge and a copper keep-out, the two
//!    microphones on the bottom edge exactly `mic_spacing_mm` apart. Every
//!    other part is then placed greedily, most-connected first, at the legal
//!    position/rotation minimising its wire length to already-placed pins;
//!    decoupling capacitors are pulled hard towards the pin they decouple.
//! 2. **GPIO pin swapping**: signals are assigned to SoC GPIOs by a
//!    min-cost assignment (Hungarian algorithm, Kuhn 1955 / Munkres 1957) on
//!    the distance from each GPIO pad to the signal's other pins.
//! 3. **Simulated annealing** (Kirkpatrick, Gelatt & Vecchi, *Science* 1983)
//!    on half-perimeter wire length (HPWL) with an overlap penalty — offered
//!    as an interactive optimiser whose history the lab animates.

use serde::{Deserialize, Serialize};

use super::board::{part_footprint, place_box, PartPos};
use super::footprints::{rot_pt, Footprint};
use super::netlist::{self, DECOUPLING, GPIO_NETS, SOC_RESERVED};
use crate::math::Rng;
use crate::project::Project;

/// (my pin, target points on the same net, weight)
type PinTargets = (String, Vec<(f64, f64)>, f64);

/// A part to place with its footprint.
struct Item {
    refdes: String,
    fp: Footprint,
}

/// Pins of each part → net (static table only; GPIO nets on the SoC are
/// represented by the SoC outline until they are assigned).
fn static_pin_nets(p: &Project, refs: &[String]) -> Vec<(String, String, String)> {
    let present = |r: &str, pin: &str| refs.iter().any(|x| x == r) && part_footprint(p, r).map(|f| f.pads.iter().any(|pd| pd.num == pin)).unwrap_or(false);
    let nets = netlist::build_nets(p, &present, &[]);
    let mut out = Vec::new();
    for n in nets {
        if n.name == "GND" {
            continue;
        }
        for (r, pin) in n.pins {
            out.push((r, pin, n.name.clone()));
        }
    }
    out
}

fn snap(v: f64, g: f64) -> f64 {
    (v / g).round() * g
}

fn overlaps(a: [f64; 4], b: [f64; 4], gap: f64) -> bool {
    a[0] < b[2] + gap - 1e-9 && b[0] < a[2] + gap - 1e-9 && a[1] < b[3] + gap - 1e-9 && b[1] < a[3] + gap - 1e-9
}

/// Everything the placer decides.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlaceResult {
    pub parts: Vec<PartPos>,
    /// (GPIO net, SoC pin).
    pub gpio: Vec<(String, String)>,
    /// Rectangles no courtyard may enter (antenna keep-out).
    pub keepouts: Vec<[f64; 4]>,
}

/// Deterministic constructive placement.
pub fn constructive(p: &Project) -> PlaceResult {
    let pcb = &p.design.pcb;
    let (w, h) = (pcb.width_mm, pcb.height_mm);
    let g = pcb.grid_mm.clamp(0.1, 1.0);
    let mut items: Vec<Item> = Vec::new();
    for c in p.components.iter().filter(|c| c.on_pcb) {
        if let Some(fp) = super::footprints::footprint(&c.footprint) {
            items.push(Item { refdes: c.refdes.clone(), fp });
        }
    }
    let refs: Vec<String> = items.iter().map(|i| i.refdes.clone()).collect();
    let pin_nets = static_pin_nets(p, &refs);
    let mut placed: Vec<(PartPos, [f64; 4])> = Vec::new();
    let mut keepouts: Vec<[f64; 4]> = Vec::new();

    let find = |fp_prefix: &str, items: &[Item]| items.iter().position(|i| i.fp.name.starts_with(fp_prefix));
    let fixed = |idx: usize, x: f64, y: f64, rot: f64, placed: &mut Vec<(PartPos, [f64; 4])>, items: &[Item]| {
        let it = &items[idx];
        let cy = place_box(it.fp.courtyard, x, y, rot);
        placed.push((PartPos { refdes: it.refdes.clone(), x, y, rot, side: "top".into() }, cy));
    };

    // --- anchors -------------------------------------------------------
    if let Some(i) = find("USB-C", &items) {
        let x = ((w - items[i].fp.body[2]) / g).floor() * g;
        fixed(i, x, snap(h / 2.0, g), 0.0, &mut placed, &items);
    }
    let mut hinge_x = 0.3;
    if let Some(i) = find("FPC", &items) {
        let c = items[i].fp.courtyard;
        let x = snap(-c[0] + 0.25, 0.5);
        let y = snap(-c[1] + 0.15, g);
        fixed(i, x, y, 0.0, &mut placed, &items);
        hinge_x = x + c[2];
    }
    if let Some(i) = find("JST", &items) {
        let c = items[i].fp.courtyard;
        let x = snap(-c[0] + 0.3, g);
        let y = ((h - c[3] - 0.2) / g).floor() * g;
        fixed(i, x, y, 0.0, &mut placed, &items);
    }
    // Main IC: the part with the most pads.
    let main = items
        .iter()
        .enumerate()
        .filter(|(_, it)| it.fp.name.starts_with("QFN") || it.fp.pads.len() >= 20 && !it.fp.name.starts_with("FPC") && !it.fp.name.starts_with("USB"))
        .max_by_key(|(_, it)| it.fp.pads.len())
        .map(|(i, _)| i);
    let mut soc_box = None;
    if let Some(i) = main {
        let c = items[i].fp.courtyard;
        // leave a 3 mm routing region between the FPC connector and the SoC for the display bus
        let gap: f64 = 3.0;
        let x = snap(hinge_x - c[0] + gap, g);
        let y = snap(h / 2.0, g);
        fixed(i, x, y, 0.0, &mut placed, &items);
        soc_box = Some(place_box(c, x, y, 0.0));
    }
    if let Some(i) = find("ANT", &items) {
        let c = items[i].fp.courtyard;
        let right = soc_box.map(|b| b[0]).unwrap_or(w / 2.0);
        let x = snap(right - c[2] - 0.6, g);
        let y = ((h - c[3] - 0.2) / g).floor() * g;
        // rotate 180° so the feed pad (pin 1) faces the SoC
        fixed(i, x, y, 180.0, &mut placed, &items);
        let cy = place_box(c, x, y, 180.0);
        keepouts.push([(cy[0] - 0.8).max(0.0), (cy[1] - 1.0).max(0.0), cy[2] + 0.4, h]);
    }
    // Microphones: bottom edge, centred in the free span right of the SoC.
    let mics: Vec<usize> = items.iter().enumerate().filter(|(_, it)| it.fp.name.starts_with("MIC")).map(|(i, _)| i).collect();
    if !mics.is_empty() {
        let lo = soc_box.map(|b| b[2]).unwrap_or(0.0);
        let hi = placed.iter().find(|(pp, _)| items.iter().any(|it| it.refdes == pp.refdes && it.fp.name.starts_with("USB"))).map(|(_, b)| b[0]).unwrap_or(w);
        let s = p.design.audio.mic_spacing_mm.max(4.0);
        let span = s * (mics.len() as f64 - 1.0);
        let c = items[mics[0]].fp.courtyard;
        let (xlo, xhi) = (-c[0] + 0.2, w - span + c[0] - 0.2);
        let x0 = snap((lo + hi) / 2.0 - span / 2.0, g).min(xhi).max(xlo);
        let y = snap(-c[1] + 0.15, g);
        for (k, &i) in mics.iter().enumerate() {
            fixed(i, x0 + s * k as f64, y, 0.0, &mut placed, &items);
        }
    }

    // --- routing demand (RUDY) ----------------------------------------------
    // Rectangular Uniform wire DensitY (Spindler & Johannes, DATE 2007): each
    // connection spreads its expected wire length uniformly over its bounding
    // box. Placing a part on top of high demand blocks routing channels, so the
    // greedy placer pays for it. Seeded with connections between anchored
    // parts, including connector → SoC GPIO signals.
    let dc = 0.5;
    let dnx = (w / dc).ceil() as usize + 1;
    let dny = (h / dc).ceil() as usize + 1;
    let mut demand = vec![0.0f64; dnx * dny];
    let add_demand = |demand: &mut Vec<f64>, a: (f64, f64), b: (f64, f64)| {
        let (x0, x1) = (a.0.min(b.0) - 0.5, a.0.max(b.0) + 0.5);
        let (y0, y1) = (a.1.min(b.1) - 0.5, a.1.max(b.1) + 0.5);
        let area = (x1 - x0) * (y1 - y0);
        let dens = ((x1 - x0) + (y1 - y0)) / area; // wire length per unit area
        let i0 = (x0 / dc).floor().max(0.0) as usize;
        let i1 = ((x1 / dc).ceil() as usize).min(dnx - 1);
        let j0 = (y0 / dc).floor().max(0.0) as usize;
        let j1 = ((y1 / dc).ceil() as usize).min(dny - 1);
        for j in j0..=j1 {
            for i in i0..=i1 {
                demand[j * dnx + i] += dens;
            }
        }
    };
    {
        let pin_at = |refdes: &str, pin: &str, placed: &Vec<(PartPos, [f64; 4])>| -> Option<(f64, f64)> {
            let (pp, _) = placed.iter().find(|(pp, _)| pp.refdes == refdes)?;
            let fp = &items.iter().find(|x| x.refdes == refdes)?.fp;
            let pd = fp.pads.iter().find(|pd| pd.num == pin)?;
            let (dx, dy) = rot_pt(pd.x, pd.y, pp.rot);
            Some((pp.x + dx, pp.y + dy))
        };
        for (r, pin, net) in netlist::PINS.iter() {
            if !GPIO_NETS.contains(net) {
                continue;
            }
            if let (Some(a), Some(sb)) = (pin_at(r, pin, &placed), soc_box) {
                let b = (a.0.clamp(sb[0], sb[2]), a.1.clamp(sb[1], sb[3]));
                add_demand(&mut demand, a, b);
            }
        }
    }
    let demand_w: f64 = 3.0;

    // --- greedy placement of the rest ---------------------------------------
    let dec = |r: &str| DECOUPLING.iter().find(|(c, _, _)| *c == r).map(|(_, ic, pin)| (ic.to_string(), pin.to_string()));
    let mut remaining: Vec<usize> = (0..items.len()).filter(|&i| !placed.iter().any(|(pp, _)| pp.refdes == items[i].refdes)).collect();
    let soc_ref = main.map(|i| items[i].refdes.clone());
    while !remaining.is_empty() {
        // pick the part with most connections to placed parts (ICs before passives)
        let score = |i: usize| -> (i64, i64) {
            let r = &items[i].refdes;
            let mut conn = 0;
            for (pr, _, net) in pin_nets.iter().filter(|(pr, _, _)| pr == r) {
                let _ = pr;
                if pin_nets.iter().any(|(r2, _, n2)| n2 == net && r2 != r && placed.iter().any(|(pp, _)| &pp.refdes == r2)) {
                    conn += 1;
                }
            }
            if let Some(soc) = &soc_ref {
                if netlist_gpio_nets(r).count() > 0 && placed.iter().any(|(pp, _)| &pp.refdes == soc) {
                    conn += netlist_gpio_nets(r).count() as i64;
                }
            }
            let pads = items[i].fp.pads.len() as i64;
            let pri = if pads >= 5 {
                3000 + conn * 30 + pads
            } else if let Some((ic, _)) = dec(r) {
                if placed.iter().any(|(pp, _)| pp.refdes == ic) { 2000 + conn } else { 500 + conn }
            } else {
                1000 + conn * 10
            };
            (pri, -(i as i64))
        };
        let (k, &i) = remaining.iter().enumerate().max_by_key(|(_, &i)| score(i)).unwrap();
        remaining.remove(k);
        let it = &items[i];
        let r = it.refdes.clone();
        // target pins: for each of my pins, the placed pins on the same net
        let placed_pin = |refdes: &str, pin: &str| -> Option<(f64, f64)> {
            let (pp, _) = placed.iter().find(|(pp, _)| pp.refdes == refdes)?;
            let fp = &items.iter().find(|x| x.refdes == refdes)?.fp;
            let pd = fp.pads.iter().find(|pd| pd.num == pin)?;
            let (dx, dy) = rot_pt(pd.x, pd.y, pp.rot);
            Some((pp.x + dx, pp.y + dy))
        };
        let mut targets: Vec<PinTargets> = Vec::new();
        for (_, pin, net) in pin_nets.iter().filter(|(pr, _, _)| *pr == r) {
            let pts: Vec<(f64, f64)> = pin_nets
                .iter()
                .filter(|(r2, _, n2)| n2 == net && *r2 != r)
                .filter_map(|(r2, p2, _)| placed_pin(r2, p2))
                .collect();
            if !pts.is_empty() {
                targets.push((pin.clone(), pts, 1.0));
            }
        }
        let gpio_mine: Vec<String> = netlist_gpio_nets(&r).map(|(pin, _)| pin.to_string()).collect();
        let decouple = dec(&r).and_then(|(ic, pin)| placed_pin(&ic, &pin).map(|pt| (pt, ic)));
        // functional zones along the temple: charger by the USB port, the buck
        // converter next to it; everything else follows its connections.
        let my_nets: Vec<&str> = pin_nets.iter().filter(|(pr, _, _)| *pr == r).map(|(_, _, n)| n.as_str()).collect();
        let dec_ic = dec(&r).map(|(ic, _)| ic);
        let zone: Option<(f64, f64)> = if my_nets.iter().any(|n| *n == "VBUS" || *n == "PROG") {
            Some((w - 17.0, w))
        } else if my_nets.contains(&"SW") || dec_ic.as_deref() == Some("U5") {
            Some((w - 24.0, w - 6.0))
        } else {
            None
        };
        let is_ic = it.fp.pads.len() >= 5;
        // fan-out halo: keep 0.5 mm around the SoC courtyard free for pin escapes
        let halo: f64 = 0.5;
        let ic_boxes: Vec<[f64; 4]> = placed
            .iter()
            .filter(|(pp, _)| items.iter().find(|x| x.refdes == pp.refdes).map(|x| x.fp.pads.len() >= 5 && !x.fp.name.starts_with("MIC")).unwrap_or(false))
            .map(|(_, b)| *b)
            .collect();
        let mut best: Option<(f64, PartPos, [f64; 4])> = None;
        let step = if it.fp.pads.len() > 4 { 0.5 } else { 0.25 };
        let nx = (w / step) as i64;
        let ny = (h / step) as i64;
        for rot in [0.0, 90.0, 180.0, 270.0] {
            let cyl = super::footprints::rot_box(it.fp.courtyard, rot);
            for jy in 0..=ny {
                let y = jy as f64 * step;
                if y + cyl[1] < 0.1 || y + cyl[3] > h - 0.1 {
                    continue;
                }
                for ix in 0..=nx {
                    let x = ix as f64 * step;
                    if x + cyl[0] < 0.1 || x + cyl[2] > w - 0.1 {
                        continue;
                    }
                    let cy = [x + cyl[0], y + cyl[1], x + cyl[2], y + cyl[3]];
                    if placed.iter().any(|(_, b)| overlaps(cy, *b, 0.0)) || keepouts.iter().any(|k| overlaps(cy, *k, 0.0)) {
                        continue;
                    }
                    // ICs keep a routing channel between their courtyards
                    if is_ic && ic_boxes.iter().any(|b| overlaps(cy, *b, 0.5)) {
                        continue;
                    }
                    // fan-out halo around the SoC: only its own decoupling may sit there
                    if let Some(sb) = soc_box {
                        if dec_ic.as_deref() != soc_ref.as_deref() && overlaps(cy, sb, halo) {
                            continue;
                        }
                    }
                    let mut cost = 0.0;
                    {
                        let i0 = (cy[0] / dc).floor().max(0.0) as usize;
                        let i1 = ((cy[2] / dc).ceil() as usize).min(dnx - 1);
                        let j0 = (cy[1] / dc).floor().max(0.0) as usize;
                        let j1 = ((cy[3] / dc).ceil() as usize).min(dny - 1);
                        let mut dsum = 0.0;
                        for j in j0..=j1 {
                            for i in i0..=i1 {
                                dsum += demand[j * dnx + i];
                            }
                        }
                        cost += demand_w * dsum * dc * dc;
                    }
                    if let Some((z0, z1)) = zone {
                        cost += 40.0 * ((z0 - x).max(x - z1).max(0.0));
                    }
                    for (pin, pts, wgt) in &targets {
                        let pd = it.fp.pads.iter().find(|pd| &pd.num == pin).unwrap();
                        let (dx, dy) = rot_pt(pd.x, pd.y, rot);
                        let (px, py) = (x + dx, y + dy);
                        let d = pts.iter().map(|(tx, ty)| (tx - px).abs() + (ty - py).abs()).fold(f64::INFINITY, f64::min);
                        cost += wgt * d;
                    }
                    if let (Some(sb), false) = (soc_box, gpio_mine.is_empty()) {
                        for pin in &gpio_mine {
                            if let Some(pd) = it.fp.pads.iter().find(|pd| &pd.num == pin) {
                                let (dx, dy) = rot_pt(pd.x, pd.y, rot);
                                let (px, py) = (x + dx, y + dy);
                                let ddx = (sb[0] - px).max(px - sb[2]).max(0.0);
                                let ddy = (sb[1] - py).max(py - sb[3]).max(0.0);
                                cost += ddx + ddy;
                            }
                        }
                    }
                    if let Some(((tx, ty), _)) = decouple {
                        let pd = &it.fp.pads[0];
                        let (dx, dy) = rot_pt(pd.x, pd.y, rot);
                        cost += 6.0 * ((tx - x - dx).abs() + (ty - y - dy).abs());
                    }
                    // mild preference for the board mid-line (keeps routing channels on both edges)
                    cost += 0.02 * (y - h / 2.0).abs() + 0.001 * rot;
                    if best.as_ref().map(|b| cost < b.0 - 1e-9).unwrap_or(true) {
                        best = Some((cost, PartPos { refdes: r.clone(), x, y, rot, side: "top".into() }, cy));
                    }
                }
            }
        }
        if let Some((_, pp, cy)) = best {
            placed.push((pp, cy));
        } else {
            // no legal spot: park it outside the board so DRC reports it
            let x = w + 3.0 + 4.0 * placed.len() as f64 % 20.0;
            placed.push((PartPos { refdes: r.clone(), x, y: -4.0, rot: 0.0, side: "top".into() }, [x - 1.0, -5.0, x + 1.0, -3.0]));
        }
    }
    let mut parts: Vec<PartPos> = placed.into_iter().map(|(pp, _)| pp).collect();
    place_features(p, &mut parts);
    let gpio = assign_gpio(p, &parts);
    PlaceResult { parts, gpio, keepouts }
}

/// Pins of `refdes` that land on a GPIO net (static table).
fn netlist_gpio_nets(refdes: &str) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
    netlist::PINS.iter().filter(move |(r, _, n)| *r == refdes && GPIO_NETS.contains(n)).map(|(_, p, n)| (*p, *n))
}


/// Board features: touch electrode between the microphones on the bottom
/// copper, SWD test pads on the bottom next to the SoC's debug pins.
fn place_features(p: &Project, parts: &mut Vec<PartPos>) {
    let h = p.design.pcb.height_mm;
    let w = p.design.pcb.width_mm;
    let g = p.design.pcb.grid_mm.clamp(0.1, 1.0);
    let feats = netlist::features_for(p);
    let get = |r: &str, parts: &Vec<PartPos>| parts.iter().find(|x| x.refdes == r).cloned();
    if feats.contains(&"E1") {
        // Touch electrode: bottom copper, upper half, as far back as the USB
        // receptacle allows (the middle of the board is the busiest routing area).
        let usb_left = parts
            .iter()
            .find(|x| part_footprint(p, &x.refdes).map(|f| f.name.starts_with("USB")).unwrap_or(false))
            .and_then(|x| part_footprint(p, &x.refdes).map(|f| x.x + f.courtyard[0]))
            .unwrap_or(w);
        let x = usb_left - 0.5 - 3.75;
        parts.push(PartPos { refdes: "E1".into(), x: snap(x, g), y: snap(h - 0.6 - 2.25, g), rot: 0.0, side: "bottom".into() });
    }
    if let Some(soc) = get("U1", parts) {
        if feats.contains(&"TP1") {
            // on the bottom copper under the SoC body, next to its debug pins
            parts.push(PartPos { refdes: "TP1".into(), x: snap(soc.x - 2.25, g), y: snap(soc.y - 2.25, g), rot: 0.0, side: "bottom".into() });
            parts.push(PartPos { refdes: "TP2".into(), x: snap(soc.x - 2.25, g), y: snap(soc.y + 2.25, g), rot: 0.0, side: "bottom".into() });
        }
    }
}

// ---------------------------------------------------------------------------
// GPIO assignment (Hungarian algorithm)
// ---------------------------------------------------------------------------

/// Assign each GPIO signal to an SoC pin by minimum total Manhattan distance
/// from the pad (pushed 1 mm outward) to the signal's other pins.
pub fn assign_gpio(p: &Project, parts: &[PartPos]) -> Vec<(String, String)> {
    let Some(soc) = parts.iter().find(|x| x.refdes == "U1") else { return vec![] };
    let Some(fp) = part_footprint(p, "U1") else { return vec![] };
    let pin_pos = |r: &str, pin: &str| -> Option<(f64, f64)> {
        let pp = parts.iter().find(|x| x.refdes == r)?;
        let f = part_footprint(p, r)?;
        let pd = f.pads.iter().find(|pd| pd.num == pin)?;
        let (dx, dy) = rot_pt(pd.x, pd.y, pp.rot);
        Some((pp.x + dx, pp.y + dy))
    };
    // signals that have at least one peripheral pin on the board
    let mut sigs: Vec<(&str, (f64, f64))> = Vec::new();
    for n in GPIO_NETS {
        let pts: Vec<(f64, f64)> = netlist::PINS.iter().filter(|(r, _, nn)| nn == n && *r != "U1").filter_map(|(r, pin, _)| pin_pos(r, pin)).collect();
        if pts.is_empty() {
            continue;
        }
        // nearest peripheral pin to the SoC is the one we aim at
        let best = pts.iter().copied().min_by(|a, b| {
            let da = (a.0 - soc.x).abs() + (a.1 - soc.y).abs();
            let db = (b.0 - soc.x).abs() + (b.1 - soc.y).abs();
            da.partial_cmp(&db).unwrap()
        });
        sigs.push((n, best.unwrap()));
    }
    let pins: Vec<(String, f64, f64)> = fp
        .pads
        .iter()
        .filter(|pd| !SOC_RESERVED.contains(&pd.num.as_str()))
        .map(|pd| {
            let (dx, dy) = rot_pt(pd.x, pd.y, soc.rot);
            // escape point 1 mm outside the pad
            let n = (dx.hypot(dy)).max(1e-9);
            let (ex, ey) = if dx.abs() > dy.abs() { (dx.signum(), 0.0) } else { (0.0, dy.signum()) };
            let _ = n;
            (pd.num.clone(), soc.x + dx + ex, soc.y + dy + ey)
        })
        .collect();
    if sigs.is_empty() || pins.len() < sigs.len() {
        return vec![];
    }
    let h = p.design.pcb.height_mm;
    let mut cost: Vec<Vec<f64>> = sigs
        .iter()
        .map(|(_, (tx, ty))| {
            pins.iter()
                .map(|(_, ex, ey)| {
                    // pins facing a board edge must turn in a narrow channel: small penalty
                    let edge = if *ey > h - 1.2 || *ey < 1.2 { 0.75 } else { 0.0 };
                    (tx - ex).abs() + (ty - ey).abs() + edge
                })
                .collect()
        })
        .collect();
    // Adjacent pads on a 0.4 mm-pitch side cannot both escape on a 0.5 mm
    // routing pitch without jogging, so we forbid neighbours of fixed power /
    // RF / debug pins and iteratively forbid one pin of every adjacent pair the
    // assignment produces (re-solving each time) while enough pins remain.
    let num = |s: &str| s.parse::<i32>().unwrap_or(-100);
    let side = |n: i32| (n - 1) / 12;
    let adjacent = |a: i32, b: i32| (a - b).abs() == 1 && side(a) == side(b);
    let fixed: Vec<i32> = SOC_RESERVED.iter().map(|s| num(s)).filter(|&n| n > 0).collect();
    let big = 1e6;
    let mut forbidden = vec![false; pins.len()];
    for (k, (pn, _, _)) in pins.iter().enumerate() {
        let n = num(pn);
        if fixed.iter().any(|&f| adjacent(f, n) && !(3..=12).contains(&f)) {
            forbidden[k] = true;
        }
    }
    let apply = |cost: &mut Vec<Vec<f64>>, forbidden: &[bool]| {
        for row in cost.iter_mut() {
            for (k, c) in row.iter_mut().enumerate() {
                if forbidden[k] && *c < big {
                    *c += big;
                }
            }
        }
    };
    let free = |forbidden: &[bool]| forbidden.iter().filter(|f| !**f).count();
    if free(&forbidden) < sigs.len() {
        forbidden.iter_mut().for_each(|f| *f = false);
    }
    apply(&mut cost, &forbidden);
    let mut assign = hungarian(&cost);
    for _ in 0..24 {
        let used: Vec<i32> = assign.iter().map(|&k| num(&pins[k].0)).collect();
        let mut conflict: Option<(usize, usize)> = None;
        'o: for a in 0..assign.len() {
            for b2 in a + 1..assign.len() {
                if adjacent(used[a], used[b2]) {
                    conflict = Some((a, b2));
                    break 'o;
                }
            }
        }
        let Some((a, b2)) = conflict else { break };
        if free(&forbidden) <= sigs.len() {
            break;
        }
        // forbid the pin of the pair whose signal has the better alternative
        let second_best = |row: usize, not: usize| cost[row].iter().enumerate().filter(|(k, _)| *k != not && !assign.contains(k)).map(|(_, c)| *c).fold(f64::INFINITY, f64::min);
        let ra = second_best(a, assign[a]) - cost[a][assign[a]];
        let rb = second_best(b2, assign[b2]) - cost[b2][assign[b2]];
        let k = if ra <= rb { assign[a] } else { assign[b2] };
        forbidden[k] = true;
        apply(&mut cost, &forbidden);
        assign = hungarian(&cost);
    }
    sigs.iter().zip(assign).map(|((n, _), k)| (n.to_string(), pins[k].0.clone())).collect()
}

/// Minimum-cost assignment of rows to distinct columns (rows ≤ columns),
/// O(n²m) shortest-augmenting-path Hungarian algorithm.
pub fn hungarian(cost: &[Vec<f64>]) -> Vec<usize> {
    let n = cost.len();
    let m = cost[0].len();
    let inf = f64::INFINITY;
    let mut u = vec![0.0; n + 1];
    let mut v = vec![0.0; m + 1];
    let mut pcol = vec![0usize; m + 1]; // row assigned to column j (1-based), 0 = none
    let mut way = vec![0usize; m + 1];
    for i in 1..=n {
        pcol[0] = i;
        let mut j0 = 0usize;
        let mut minv = vec![inf; m + 1];
        let mut used = vec![false; m + 1];
        loop {
            used[j0] = true;
            let i0 = pcol[j0];
            let mut delta = inf;
            let mut j1 = 0;
            for j in 1..=m {
                if !used[j] {
                    let cur = cost[i0 - 1][j - 1] - u[i0] - v[j];
                    if cur < minv[j] {
                        minv[j] = cur;
                        way[j] = j0;
                    }
                    if minv[j] < delta {
                        delta = minv[j];
                        j1 = j;
                    }
                }
            }
            for j in 0..=m {
                if used[j] {
                    u[pcol[j]] += delta;
                    v[j] -= delta;
                } else {
                    minv[j] -= delta;
                }
            }
            j0 = j1;
            if pcol[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = way[j0];
            pcol[j0] = pcol[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }
    let mut ans = vec![0usize; n];
    for j in 1..=m {
        if pcol[j] > 0 {
            ans[pcol[j] - 1] = j - 1;
        }
    }
    ans
}

// ---------------------------------------------------------------------------
// HPWL and simulated annealing
// ---------------------------------------------------------------------------

/// Half-perimeter wire length of every non-ground net (mm):
/// HPWL = Σ_nets (max x − min x) + (max y − min y).
pub fn hpwl(p: &Project, parts: &[PartPos], gpio: &[(String, String)]) -> f64 {
    let b = super::board::build(p, parts, gpio);
    hpwl_board(&b)
}

pub fn hpwl_board(b: &super::board::Board) -> f64 {
    let mut total = 0.0;
    for (ni, n) in b.nets.iter().enumerate() {
        if n.name == "GND" {
            continue;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut cnt = 0;
        for pd in b.pads.iter().filter(|pd| pd.net_id as usize == ni) {
            x0 = x0.min(pd.x);
            x1 = x1.max(pd.x);
            y0 = y0.min(pd.y);
            y1 = y1.max(pd.y);
            cnt += 1;
        }
        if cnt >= 2 {
            total += (x1 - x0) + (y1 - y0);
        }
    }
    total
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnnealFrame {
    pub iteration: usize,
    pub temperature: f64,
    pub hpwl: f64,
    pub overlap: f64,
    pub cost: f64,
    pub parts: Vec<PartPos>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnnealResult {
    /// Sampled every few iterations for the plot.
    pub iterations: Vec<usize>,
    pub hpwl: Vec<f64>,
    pub best_hpwl: Vec<f64>,
    pub temperature: Vec<f64>,
    pub acceptance: Vec<f64>,
    /// Board snapshots for the animation.
    pub frames: Vec<AnnealFrame>,
    pub initial_hpwl: f64,
    pub final_hpwl: f64,
    pub constructive_hpwl: f64,
    pub final_parts: Vec<PartPos>,
    pub movable: Vec<String>,
    pub final_overlap: f64,
}

/// Simulated annealing of the movable parts (everything except the edge
/// anchors: connectors, antenna, microphones, SoC and board features),
/// starting from a random scatter so the optimisation is visible.
/// Cost = HPWL + λ·(courtyard overlap area + keep-out/board violations).
pub fn anneal(p: &Project, iterations: usize, seed: u64) -> AnnealResult {
    let base = constructive(p);
    let pcb = &p.design.pcb;
    let (w, h) = (pcb.width_mm, pcb.height_mm);
    let g = 0.25;
    let anchor = |r: &str| -> bool {
        let fp = part_footprint(p, r).map(|f| f.name).unwrap_or_default();
        r == "U1" || fp.starts_with("USB") || fp.starts_with("FPC") || fp.starts_with("JST") || fp.starts_with("ANT") || fp.starts_with("MIC") || super::netlist::BOARD_FEATURES.contains(&r)
    };
    let fps: Vec<Footprint> = base.parts.iter().map(|pp| part_footprint(p, &pp.refdes).unwrap()).collect();
    let movable: Vec<usize> = (0..base.parts.len()).filter(|&i| !anchor(&base.parts[i].refdes)).collect();
    let mut rng = Rng::new(seed);
    let mut cur = base.parts.clone();
    for &i in &movable {
        cur[i].x = snap(2.0 + rng.uniform() * (w - 4.0), g);
        cur[i].y = snap(1.5 + rng.uniform() * (h - 3.0), g);
        cur[i].rot = 90.0 * (rng.uniform() * 4.0).floor();
    }
    // Net → list of (part index, local pad offset) for fast incremental HPWL.
    let b0 = super::board::build(p, &cur, &base.gpio);
    let mut nets: Vec<Vec<(usize, f64, f64)>> = Vec::new();
    for (ni, n) in b0.nets.iter().enumerate() {
        if n.name == "GND" {
            continue;
        }
        let mut pins = Vec::new();
        for pd in b0.pads.iter().filter(|pd| pd.net_id as usize == ni) {
            let pi = cur.iter().position(|pp| pp.refdes == pd.refdes).unwrap();
            let fpd = fps[pi].pads.iter().find(|x| x.num == pd.pin).unwrap();
            pins.push((pi, fpd.x, fpd.y));
        }
        if pins.len() >= 2 {
            nets.push(pins);
        }
    }
    let net_hpwl = |parts: &[PartPos], pins: &Vec<(usize, f64, f64)>| -> f64 {
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for &(pi, lx, ly) in pins {
            let pp = &parts[pi];
            let (dx, dy) = rot_pt(lx, ly, pp.rot);
            let (x, y) = (pp.x + dx, pp.y + dy);
            x0 = x0.min(x);
            x1 = x1.max(x);
            y0 = y0.min(y);
            y1 = y1.max(y);
        }
        (x1 - x0) + (y1 - y0)
    };
    let boxes = |parts: &[PartPos]| -> Vec<[f64; 4]> { parts.iter().zip(fps.iter()).map(|(pp, fp)| place_box(fp.courtyard, pp.x, pp.y, pp.rot)).collect() };
    let area_ov = |a: [f64; 4], b: [f64; 4]| -> f64 { ((a[2].min(b[2]) - a[0].max(b[0])).max(0.0)) * ((a[3].min(b[3]) - a[1].max(b[1])).max(0.0)) };
    let out_of_board = |a: [f64; 4]| -> f64 {
        let ww = a[2] - a[0];
        let hh = a[3] - a[1];
        let inside = area_ov(a, [0.1, 0.1, w - 0.1, h - 0.1]);
        ww * hh - inside
    };
    let top_side = |i: usize, parts: &[PartPos]| parts[i].side == "top";
    let is_mov: Vec<bool> = (0..base.parts.len()).map(|i| movable.contains(&i)).collect();
    let overlap_of = |i: usize, parts: &[PartPos], bx: &[[f64; 4]]| -> f64 {
        if !is_mov[i] {
            return 0.0;
        }
        let mut s = out_of_board(bx[i]) * 2.0;
        for k in &base.keepouts {
            s += area_ov(bx[i], *k) * 2.0;
        }
        for j in 0..parts.len() {
            if j != i && top_side(j, parts) == top_side(i, parts) {
                s += area_ov(bx[i], bx[j]);
            }
        }
        s
    };
    let _ = &is_mov;
    let total_overlap = |parts: &[PartPos]| -> f64 {
        let bx = boxes(parts);
        let mut s = 0.0;
        for i in 0..parts.len() {
            if !is_mov[i] {
                continue;
            }
            s += out_of_board(bx[i]) * 2.0;
            for k in &base.keepouts {
                s += area_ov(bx[i], *k) * 2.0;
            }
            for j in 0..parts.len() {
                if j != i && (!is_mov[j] || j > i) && top_side(j, parts) == top_side(i, parts) {
                    s += area_ov(bx[i], bx[j]);
                }
            }
        }
        s
    };
    // overlap weight ramps up so the anneal first explores, then legalises
    let lambda_at = |it: usize, n: usize| 2.0 + 60.0 * (it as f64 / n.max(1) as f64);
    let lambda = 62.0;
    let total_hpwl = |parts: &[PartPos]| nets.iter().map(|n| net_hpwl(parts, n)).sum::<f64>();
    let mut cur_h = total_hpwl(&cur);
    let mut cur_o = total_overlap(&cur);
    let initial_hpwl = cur_h;
    let mut best = cur.clone();
    let mut best_cost = f64::INFINITY;
    let iterations = iterations.clamp(100, 200_000);
    // Initial temperature from the average uphill move size (White 1984 style).
    let t0: f64 = 20.0;
    let t_end: f64 = 0.02;
    let alpha = (t_end / t0).powf(1.0 / iterations as f64);
    let mut t = t0;
    let sample_every = (iterations / 200).max(1);
    let frame_every = (iterations / 60).max(1);
    let mut res = AnnealResult {
        iterations: vec![],
        hpwl: vec![],
        best_hpwl: vec![],
        temperature: vec![],
        acceptance: vec![],
        frames: vec![],
        initial_hpwl,
        final_hpwl: 0.0,
        constructive_hpwl: hpwl(p, &base.parts, &base.gpio),
        final_parts: vec![],
        movable: movable.iter().map(|&i| cur[i].refdes.clone()).collect(),
        final_overlap: 0.0,
    };
    let nets_of: Vec<Vec<usize>> = (0..cur.len()).map(|pi| (0..nets.len()).filter(|&ni| nets[ni].iter().any(|x| x.0 == pi)).collect()).collect();
    let mut accepted = 0usize;
    let mut tried = 0usize;
    if movable.is_empty() {
        res.final_parts = cur;
        return res;
    }
    for it in 0..iterations {
        let i = movable[(rng.uniform() * movable.len() as f64) as usize % movable.len()];
        let old = cur[i].clone();
        let range = (t / t0).sqrt() * w * 0.5 + 0.5;
        let mv = rng.uniform();
        // swap two movable parts with the same footprint, rotate, or displace
        let mut swapped: Option<usize> = None;
        if mv < 0.1 {
            let j = movable[(rng.uniform() * movable.len() as f64) as usize % movable.len()];
            if j != i && fps[j].name == fps[i].name {
                let (xi, yi) = (cur[i].x, cur[i].y);
                cur[i].x = cur[j].x;
                cur[i].y = cur[j].y;
                cur[j].x = xi;
                cur[j].y = yi;
                swapped = Some(j);
            }
        } else if mv < 0.25 {
            cur[i].rot = (cur[i].rot + 90.0) % 360.0;
        } else {
            cur[i].x = snap((cur[i].x + (rng.uniform() * 2.0 - 1.0) * range).clamp(0.0, w), g);
            cur[i].y = snap((cur[i].y + (rng.uniform() * 2.0 - 1.0) * range.min(h)).clamp(0.0, h), g);
        }
        // incremental cost change
        let touched: Vec<usize> = match swapped {
            Some(j) => vec![i, j],
            None => vec![i],
        };
        let mut affected: Vec<usize> = touched.iter().flat_map(|&k| nets_of[k].iter().copied()).collect();
        affected.sort_unstable();
        affected.dedup();
        let mut old_parts = cur.clone();
        old_parts[i] = old.clone();
        if let Some(j) = swapped {
            old_parts[j].x = cur[i].x;
            old_parts[j].y = cur[i].y;
        }
        let dh: f64 = affected.iter().map(|&ni| net_hpwl(&cur, &nets[ni]) - net_hpwl(&old_parts, &nets[ni])).sum();
        let bx_new = boxes(&cur);
        let bx_old = boxes(&old_parts);
        let mut d_o = 0.0;
        for &k in &touched {
            d_o += overlap_of(k, &cur, &bx_new) - overlap_of(k, &old_parts, &bx_old);
        }
        if touched.len() == 2 {
            d_o -= area_ov(bx_new[touched[0]], bx_new[touched[1]]) - area_ov(bx_old[touched[0]], bx_old[touched[1]]);
        }
        let dc = dh + lambda_at(it, iterations) * d_o;
        tried += 1;
        if dc <= 0.0 || rng.uniform() < (-dc / t).exp() {
            cur_h += dh;
            cur_o += d_o;
            accepted += 1;
            let c = cur_h + lambda * cur_o;
            if c < best_cost - 1e-9 && it > iterations / 3 {
                best_cost = c;
                best = cur.clone();
            }
        } else {
            cur = old_parts;
        }
        if it % sample_every == 0 || it + 1 == iterations {
            res.iterations.push(it);
            res.hpwl.push(cur_h);
            res.best_hpwl.push(total_hpwl(&best));
            res.temperature.push(t);
            res.acceptance.push(accepted as f64 / tried.max(1) as f64);
            accepted = 0;
            tried = 0;
        }
        if it % frame_every == 0 || it + 1 == iterations {
            res.frames.push(AnnealFrame { iteration: it, temperature: t, hpwl: cur_h, overlap: cur_o.max(0.0), cost: cur_h + lambda * cur_o, parts: movable.iter().map(|&k| cur[k].clone()).collect() });
        }
        t *= alpha;
    }
    // Greedy legalisation pass: nudge overlapping parts to the nearest free spot.
    let mut fin = if best_cost.is_finite() { best } else { cur.clone() };
    legalize(&mut fin, &fps, &movable, &base.keepouts, w, h);
    res.final_hpwl = total_hpwl(&fin);
    res.final_overlap = total_overlap(&fin);
    res.frames.push(AnnealFrame {
        iteration: iterations,
        temperature: t,
        hpwl: res.final_hpwl,
        overlap: res.final_overlap,
        cost: res.final_hpwl + lambda * res.final_overlap,
        parts: movable.iter().map(|&k| fin[k].clone()).collect(),
    });
    res.final_parts = fin;
    res
}

fn legalize(parts: &mut [PartPos], fps: &[Footprint], movable: &[usize], keepouts: &[[f64; 4]], w: f64, h: f64) {
    for &i in movable {
        let bx = |pp: &PartPos, fp: &Footprint| place_box(fp.courtyard, pp.x, pp.y, pp.rot);
        let bad = |parts: &[PartPos], i: usize| {
            let b = bx(&parts[i], &fps[i]);
            b[0] < 0.1 || b[1] < 0.1 || b[2] > w - 0.1 || b[3] > h - 0.1
                || keepouts.iter().any(|k| overlaps(b, *k, 0.0))
                || (0..parts.len()).any(|j| j != i && parts[j].side == parts[i].side && overlaps(b, bx(&parts[j], &fps[j]), 0.0))
        };
        if !bad(parts, i) {
            continue;
        }
        let (x0, y0) = (parts[i].x, parts[i].y);
        let mut found = false;
        'outer: for rad in 1..80 {
            let r = rad as f64 * 0.25;
            for k in 0..(8 * rad) {
                let a = k as f64 / (8 * rad) as f64 * std::f64::consts::TAU;
                parts[i].x = snap(x0 + r * a.cos(), 0.25);
                parts[i].y = snap(y0 + r * a.sin(), 0.25);
                if !bad(parts, i) {
                    found = true;
                    break 'outer;
                }
            }
        }
        if !found {
            parts[i].x = x0;
            parts[i].y = y0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hungarian_small() {
        let c = vec![vec![4.0, 1.0, 3.0], vec![2.0, 0.0, 5.0], vec![3.0, 2.0, 2.0]];
        let a = hungarian(&c);
        let total: f64 = a.iter().enumerate().map(|(i, &j)| c[i][j]).sum();
        assert_eq!(total, 5.0);
    }

    #[test]
    #[allow(clippy::needless_range_loop)]
    fn constructive_is_deterministic_and_legal() {
        let p = Project::iris();
        let a = constructive(&p);
        let b = constructive(&p);
        assert_eq!(a.parts, b.parts);
        let fps: Vec<Footprint> = a.parts.iter().map(|pp| part_footprint(&p, &pp.refdes).unwrap()).collect();
        for i in 0..a.parts.len() {
            let bi = place_box(fps[i].courtyard, a.parts[i].x, a.parts[i].y, a.parts[i].rot);
            let slack = if fps[i].name.starts_with("USB") { 0.5 } else { 0.0 };
            assert!(bi[0] >= 0.0 && bi[2] <= p.design.pcb.width_mm + slack && bi[1] >= 0.0 && bi[3] <= p.design.pcb.height_mm, "{} {:?}", a.parts[i].refdes, bi);
            for j in i + 1..a.parts.len() {
                if a.parts[i].side != a.parts[j].side {
                    continue;
                }
                let bj = place_box(fps[j].courtyard, a.parts[j].x, a.parts[j].y, a.parts[j].rot);
                assert!(!overlaps(bi, bj, 0.0), "{} overlaps {}", a.parts[i].refdes, a.parts[j].refdes);
            }
        }
        // microphones exactly mic_spacing apart
        let m1 = a.parts.iter().find(|x| x.refdes == "MK1").unwrap();
        let m2 = a.parts.iter().find(|x| x.refdes == "MK2").unwrap();
        assert!(((m2.x - m1.x) - p.design.audio.mic_spacing_mm).abs() < 1e-9);
        assert_eq!(a.gpio.len(), GPIO_NETS.len());
    }

    #[test]
    fn annealing_reduces_hpwl() {
        let p = Project::iris();
        let r = anneal(&p, 20000, 7);
        assert!(r.final_hpwl < r.initial_hpwl * 0.7, "{} -> {}", r.initial_hpwl, r.final_hpwl);
        assert!(r.final_overlap < 1e-9, "legalised result must not overlap: {}", r.final_overlap);
        assert!(!r.frames.is_empty());
    }
}
