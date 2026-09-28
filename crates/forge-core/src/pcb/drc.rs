//! Connectivity extraction and design-rule check (DRC).
//!
//! Connectivity is extracted from the *geometry* (not from what the router
//! believes it did): copper items of the same net that touch on a shared
//! layer are merged with a union–find, ground items are joined through the
//! pour regions they touch, and a net is complete when all its pads end up
//! in one set. The DRC then checks, independently of the router's grid:
//! copper-to-copper clearance between different nets (pad/track/via, exact
//! distances), minimum track width, copper-to-edge clearance, drill size,
//! annular ring and hole spacing, non-plated-hole clearance, keep-outs,
//! courtyard overlaps, parts outside the outline and unrouted connections.
//! Default limits follow common 2-layer prototype capabilities and the
//! IPC-2221 generic design guidance; check your fab's published rules.

use serde::{Deserialize, Serialize};

use super::board::Board;
use super::geom::{Shape, SpatialHash};
use super::pour::Pour;
use super::route::{Track, Via};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Violation {
    pub kind: String,
    pub message: String,
    pub x: f64,
    pub y: f64,
    pub items: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NetConn {
    pub net: String,
    pub pins: usize,
    /// Number of separate copper islands the pins fall into.
    pub islands: usize,
    pub unconnected: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Connectivity {
    pub nets: Vec<NetConn>,
    pub connections: usize,
    pub routed: usize,
}

struct Dsu(Vec<usize>);
impl Dsu {
    fn find(&mut self, a: usize) -> usize {
        let mut r = a;
        while self.0[r] != r {
            r = self.0[r];
        }
        let mut a = a;
        while self.0[a] != r {
            let nx = self.0[a];
            self.0[a] = r;
            a = nx;
        }
        r
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.0[ra] = rb;
        }
    }
}

/// One copper item for checks.
#[derive(Clone, Copy)]
struct Item {
    shape: Shape,
    top: bool,
    bot: bool,
    net: u16,
    /// 'p' pad, 't' track, 'v' via
    kind: char,
    idx: usize,
}

fn items(b: &Board, tracks: &[Track], vias: &[Via]) -> Vec<Item> {
    let mut v = Vec::new();
    for (i, p) in b.pads.iter().enumerate() {
        v.push(Item {
            shape: p.shape_geom(),
            top: p.top,
            bot: p.bottom,
            net: p.net_id,
            kind: 'p',
            idx: i,
        });
    }
    for (i, t) in tracks.iter().enumerate() {
        v.push(Item {
            shape: t.shape(),
            top: t.layer == 0,
            bot: t.layer == 1,
            net: t.net_id,
            kind: 't',
            idx: i,
        });
    }
    for (i, vi) in vias.iter().enumerate() {
        v.push(Item {
            shape: vi.shape(),
            top: true,
            bot: true,
            net: vi.net_id,
            kind: 'v',
            idx: i,
        });
    }
    v
}

fn label(b: &Board, tracks: &[Track], vias: &[Via], it: &Item) -> String {
    match it.kind {
        'p' => {
            let p = &b.pads[it.idx];
            format!(
                "pad {}.{}{}",
                p.refdes,
                p.pin,
                if p.net.is_empty() { String::new() } else { format!(" ({})", p.net) }
            )
        }
        't' => format!(
            "track {} on {}",
            tracks[it.idx].net,
            if tracks[it.idx].layer == 0 { "F.Cu" } else { "B.Cu" }
        ),
        _ => format!("via {}", vias[it.idx].net),
    }
}

/// Ground connectivity groups (for pour stitching).
#[derive(Clone, Debug, Default)]
pub struct GndGroups {
    /// Group of each pour region, per pour (same order as passed in).
    pub region_group: Vec<Vec<usize>>,
    /// (pad index, group) for every ground pad.
    pub pad_group: Vec<(usize, usize)>,
    /// (via index, group) for every ground via.
    pub via_group: Vec<(usize, usize)>,
    /// Group holding the most ground pads.
    pub main: usize,
    /// Number of groups that contain ground pads.
    pub count: usize,
}

fn union_all(b: &Board, tracks: &[Track], vias: &[Via], pours: &[&Pour]) -> (Dsu, usize) {
    let its = items(b, tracks, vias);
    let n_items = its.len();
    let total_regions: usize = pours.iter().map(|p| p.regions).sum();
    let mut dsu = Dsu((0..n_items + total_regions).collect());
    let mut hash = SpatialHash::new(-2.0, -2.0, b.width + 2.0, b.height + 2.0, 1.0);
    for (i, it) in its.iter().enumerate() {
        hash.insert(i, it.shape.bbox());
    }
    let mut ids = Vec::new();
    for (i, a) in its.iter().enumerate() {
        let bb = a.shape.bbox();
        hash.query([bb[0] - 0.01, bb[1] - 0.01, bb[2] + 0.01, bb[3] + 0.01], &mut ids);
        for &j in &ids {
            if j <= i {
                continue;
            }
            let c = &its[j];
            if c.net != a.net || !((a.top && c.top) || (a.bot && c.bot)) {
                continue;
            }
            if a.shape.dist(&c.shape) <= 1e-6 {
                dsu.union(i, j);
            }
        }
    }
    let base = |kind: char, idx: usize| -> usize {
        match kind {
            'p' => idx,
            't' => b.pads.len() + idx,
            _ => b.pads.len() + tracks.len() + idx,
        }
    };
    let mut off = n_items;
    for pour in pours {
        for &(kind, idx, region) in &pour.anchors {
            if region != usize::MAX {
                dsu.union(base(kind, idx), off + region);
            }
        }
        off += pour.regions;
    }
    (dsu, n_items)
}

pub fn gnd_groups(b: &Board, tracks: &[Track], vias: &[Via], pours: &[&Pour]) -> GndGroups {
    let (mut dsu, n_items) = union_all(b, tracks, vias, pours);
    let mut roots: Vec<usize> = Vec::new();
    let gid = |r: usize, roots: &mut Vec<usize>| -> usize {
        if let Some(k) = roots.iter().position(|&x| x == r) {
            k
        } else {
            roots.push(r);
            roots.len() - 1
        }
    };
    let mut g = GndGroups::default();
    for (i, p) in b.pads.iter().enumerate() {
        if p.net_id == b.gnd {
            let r = dsu.find(i);
            let k = gid(r, &mut roots);
            g.pad_group.push((i, k));
        }
    }
    g.count = roots.len();
    for (i, v) in vias.iter().enumerate() {
        if v.net_id == b.gnd {
            let r = dsu.find(b.pads.len() + tracks.len() + i);
            let k = gid(r, &mut roots);
            g.via_group.push((i, k));
        }
    }
    let mut off = n_items;
    for pour in pours {
        let mut v = Vec::new();
        for reg in 0..pour.regions {
            let r = dsu.find(off + reg);
            v.push(gid(r, &mut roots));
        }
        g.region_group.push(v);
        off += pour.regions;
    }
    let mut counts = vec![0usize; roots.len()];
    for &(_, k) in &g.pad_group {
        counts[k] += 1;
    }
    g.main = counts.iter().enumerate().max_by_key(|(_, c)| **c).map(|(k, _)| k).unwrap_or(0);
    g
}

pub fn connectivity(b: &Board, tracks: &[Track], vias: &[Via], pours: &[&Pour]) -> Connectivity {
    let (mut dsu, _) = union_all(b, tracks, vias, pours);
    let mut out = Connectivity::default();
    for (ni, net) in b.nets.iter().enumerate() {
        let pads: Vec<usize> = b
            .pads
            .iter()
            .enumerate()
            .filter(|(_, p)| p.net_id as usize == ni)
            .map(|(i, _)| i)
            .collect();
        if pads.len() < 2 {
            continue;
        }
        let mut roots: Vec<(usize, usize)> = Vec::new(); // (root, count)
        let pr: Vec<usize> = pads.iter().map(|&p| dsu.find(p)).collect();
        for &r in &pr {
            if let Some(e) = roots.iter_mut().find(|e| e.0 == r) {
                e.1 += 1;
            } else {
                roots.push((r, 1));
            }
        }
        let main = roots.iter().max_by_key(|e| e.1).map(|e| e.0).unwrap();
        let unconnected: Vec<String> = pads
            .iter()
            .zip(pr.iter())
            .filter(|(_, &r)| r != main)
            .map(|(&p, _)| format!("{}.{}", b.pads[p].refdes, b.pads[p].pin))
            .collect();
        out.connections += pads.len() - 1;
        out.routed += pads.len() - roots.len();
        out.nets.push(NetConn {
            net: net.name.clone(),
            pins: pads.len(),
            islands: roots.len(),
            unconnected,
        });
    }
    out
}

pub fn check(b: &Board, tracks: &[Track], vias: &[Via], pours: &[&Pour], conn: &Connectivity) -> Vec<Violation> {
    let r = &b.rules;
    let mut out = Vec::new();
    let its = items(b, tracks, vias);
    let mut hash = SpatialHash::new(-2.0, -2.0, b.width + 2.0, b.height + 2.0, 1.0);
    for (i, it) in its.iter().enumerate() {
        hash.insert(i, it.shape.bbox());
    }
    let mut ids = Vec::new();
    // 1. clearance between different nets
    for (i, a) in its.iter().enumerate() {
        let bb = a.shape.bbox();
        hash.query(
            [
                bb[0] - r.clearance_mm,
                bb[1] - r.clearance_mm,
                bb[2] + r.clearance_mm,
                bb[3] + r.clearance_mm,
            ],
            &mut ids,
        );
        for &j in &ids {
            if j <= i {
                continue;
            }
            let c = &its[j];
            if c.net == a.net || !((a.top && c.top) || (a.bot && c.bot)) {
                continue;
            }
            // pads of the same footprint are the footprint designer's business
            // unless they actually short.
            let d = a.shape.dist(&c.shape);
            if d < r.clearance_mm - 1e-6 {
                let (bx, by) = centre_between(&a.shape, &c.shape);
                out.push(Violation {
                    kind: if d <= 0.0 { "short".into() } else { "clearance".into() },
                    message: format!(
                        "{:.3} mm between {} and {} (min {:.3})",
                        d,
                        label(b, tracks, vias, a),
                        label(b, tracks, vias, c),
                        r.clearance_mm
                    ),
                    x: bx,
                    y: by,
                    items: vec![label(b, tracks, vias, a), label(b, tracks, vias, c)],
                });
            }
        }
    }
    // 2. pours vs other copper on the same layer
    for pour in pours {
        let layer = pour.layer;
        for rect in &pour.rects {
            let sh = Shape::Rect {
                cx: (rect[0] + rect[2]) / 2.0,
                cy: (rect[1] + rect[3]) / 2.0,
                hw: (rect[2] - rect[0]) / 2.0,
                hh: (rect[3] - rect[1]) / 2.0,
            };
            let bb = sh.bbox();
            hash.query(
                [
                    bb[0] - r.clearance_mm,
                    bb[1] - r.clearance_mm,
                    bb[2] + r.clearance_mm,
                    bb[3] + r.clearance_mm,
                ],
                &mut ids,
            );
            for &j in &ids {
                let c = &its[j];
                if c.net == b.gnd || !(if layer == 0 { c.top } else { c.bot }) {
                    continue;
                }
                let d = sh.dist(&c.shape);
                if d < r.clearance_mm - 1e-6 {
                    out.push(Violation {
                        kind: "clearance".into(),
                        message: format!("GND pour {:.3} mm from {}", d, label(b, tracks, vias, c)),
                        x: (rect[0] + rect[2]) / 2.0,
                        y: (rect[1] + rect[3]) / 2.0,
                        items: vec!["GND pour".into(), label(b, tracks, vias, c)],
                    });
                }
            }
        }
    }
    // 3. track width
    for t in tracks {
        if t.width < r.trace_mm - 1e-9 {
            out.push(Violation {
                kind: "width".into(),
                message: format!("track {} is {:.3} mm wide (min {:.3})", t.net, t.width, r.trace_mm),
                x: (t.x0 + t.x1) / 2.0,
                y: (t.y0 + t.y1) / 2.0,
                items: vec![t.net.clone()],
            });
        }
    }
    // 4. board edge
    for it in &its {
        let bb = it.shape.bbox();
        let d = bb[0].min(bb[1]).min(b.width - bb[2]).min(b.height - bb[3]);
        if d < r.edge_clearance_mm - 1e-6 {
            out.push(Violation {
                kind: "edge".into(),
                message: format!(
                    "{} is {:.3} mm from the board edge (min {:.2})",
                    label(b, tracks, vias, it),
                    d,
                    r.edge_clearance_mm
                ),
                x: (bb[0] + bb[2]) / 2.0,
                y: (bb[1] + bb[3]) / 2.0,
                items: vec![label(b, tracks, vias, it)],
            });
        }
    }
    // 5. drills: size, annular ring, hole-to-hole, NPTH clearance
    let mut drills: Vec<(f64, f64, f64, String)> = Vec::new();
    for v in vias {
        drills.push((v.x, v.y, v.drill, format!("via {}", v.net)));
        if (v.pad - v.drill) / 2.0 < r.min_annular_mm - 1e-9 {
            out.push(Violation {
                kind: "annular".into(),
                message: format!("via annular ring {:.3} mm < {:.2}", (v.pad - v.drill) / 2.0, r.min_annular_mm),
                x: v.x,
                y: v.y,
                items: vec![v.net.clone()],
            });
        }
    }
    for p in b.pads.iter().filter(|p| p.drill > 0.0) {
        drills.push((p.x, p.y, p.drill, format!("pad {}.{}", p.refdes, p.pin)));
        if (p.w.min(p.h) - p.drill) / 2.0 < r.min_annular_mm - 1e-9 {
            out.push(Violation {
                kind: "annular".into(),
                message: format!("pad {}.{} annular ring too small", p.refdes, p.pin),
                x: p.x,
                y: p.y,
                items: vec![],
            });
        }
    }
    for h in &b.holes {
        drills.push((h.x, h.y, h.d, format!("NPTH {}", h.refdes)));
        let sh = Shape::circle(h.x, h.y, h.d / 2.0);
        for it in &its {
            let d = sh.dist(&it.shape);
            if d < r.hole_clearance_mm - 1e-6 {
                out.push(Violation {
                    kind: "hole_clearance".into(),
                    message: format!("{} is {:.3} mm from a non-plated hole", label(b, tracks, vias, it), d),
                    x: h.x,
                    y: h.y,
                    items: vec![label(b, tracks, vias, it)],
                });
            }
        }
    }
    for (i, a) in drills.iter().enumerate() {
        if a.2 < r.min_drill_mm - 1e-9 {
            out.push(Violation {
                kind: "drill".into(),
                message: format!("{} drill {:.2} mm < {:.2} mm", a.3, a.2, r.min_drill_mm),
                x: a.0,
                y: a.1,
                items: vec![a.3.clone()],
            });
        }
        for c in drills.iter().skip(i + 1) {
            let gap = (a.0 - c.0).hypot(a.1 - c.1) - (a.2 + c.2) / 2.0;
            if gap < 0.2 {
                out.push(Violation {
                    kind: "hole_spacing".into(),
                    message: format!("{} and {}: {:.3} mm between drill edges", a.3, c.3, gap),
                    x: (a.0 + c.0) / 2.0,
                    y: (a.1 + c.1) / 2.0,
                    items: vec![a.3.clone(), c.3.clone()],
                });
            }
        }
    }
    // 6. keep-outs
    for k in &b.keepouts {
        let owner = k.name.split_whitespace().next().unwrap_or("");
        let sh = Shape::Rect {
            cx: (k.rect[0] + k.rect[2]) / 2.0,
            cy: (k.rect[1] + k.rect[3]) / 2.0,
            hw: (k.rect[2] - k.rect[0]) / 2.0,
            hh: (k.rect[3] - k.rect[1]) / 2.0,
        };
        for it in &its {
            if !((k.top && it.top) || (k.bottom && it.bot)) {
                continue;
            }
            if it.kind == 'p' && (b.pads[it.idx].refdes == owner || (k.name.starts_with("Touch") && b.pads[it.idx].refdes == "E1")) {
                continue;
            }
            let allowed = !k.allow_net.is_empty() && b.nets.get(it.net as usize).map(|n| n.name == k.allow_net).unwrap_or(false);
            if allowed && k.top && it.top {
                continue;
            }
            if allowed && !k.top {
                continue;
            }
            if sh.dist(&it.shape) <= 1e-9 && overlap_area_positive(&sh, &it.shape) {
                let bb = it.shape.bbox();
                out.push(Violation {
                    kind: "keepout".into(),
                    message: format!("{} inside {}", label(b, tracks, vias, it), k.name),
                    x: (bb[0] + bb[2]) / 2.0,
                    y: (bb[1] + bb[3]) / 2.0,
                    items: vec![label(b, tracks, vias, it)],
                });
            }
        }
    }
    // 7. courtyards
    let parts: Vec<_> = b.parts.iter().filter(|p| !p.feature).collect();
    for (i, a) in parts.iter().enumerate() {
        let ca = a.courtyard;
        if ca[0] < -1e-6 || ca[1] < -1e-6 || ca[2] > b.width + 1e-6 || ca[3] > b.height + 1e-6 {
            // connectors may overhang the edge by design; only report parts whose centre is outside
            if a.x < 0.0 || a.y < 0.0 || a.x > b.width || a.y > b.height {
                out.push(Violation {
                    kind: "outline".into(),
                    message: format!("{} is outside the board outline", a.refdes),
                    x: a.x.clamp(0.0, b.width),
                    y: a.y.clamp(0.0, b.height),
                    items: vec![a.refdes.clone()],
                });
            }
        }
        for c in parts.iter().skip(i + 1) {
            if a.side != c.side {
                continue;
            }
            let cb = c.courtyard;
            let ox = ca[2].min(cb[2]) - ca[0].max(cb[0]);
            let oy = ca[3].min(cb[3]) - ca[1].max(cb[1]);
            if ox > 1e-6 && oy > 1e-6 {
                out.push(Violation {
                    kind: "courtyard".into(),
                    message: format!("courtyards of {} and {} overlap ({:.2} × {:.2} mm)", a.refdes, c.refdes, ox, oy),
                    x: (ca[0].max(cb[0]) + ca[2].min(cb[2])) / 2.0,
                    y: (ca[1].max(cb[1]) + ca[3].min(cb[3])) / 2.0,
                    items: vec![a.refdes.clone(), c.refdes.clone()],
                });
            }
        }
    }
    // 8. unrouted
    for nc in &conn.nets {
        for pin in &nc.unconnected {
            let (x, y) = pin
                .split_once('.')
                .and_then(|(r, p)| b.pads.iter().find(|pd| pd.refdes == r && pd.pin == p))
                .map(|pd| (pd.x, pd.y))
                .unwrap_or((0.0, 0.0));
            out.push(Violation {
                kind: "unrouted".into(),
                message: format!("{} is not connected to the rest of net {}", pin, nc.net),
                x,
                y,
                items: vec![pin.clone(), nc.net.clone()],
            });
        }
    }
    out
}

/// True when the shape actually enters the rectangle (not merely touches its edge).
fn overlap_area_positive(rect: &Shape, s: &Shape) -> bool {
    match (*rect, *s) {
        (Shape::Rect { cx, cy, hw, hh }, other) => {
            let inner = Shape::Rect {
                cx,
                cy,
                hw: (hw - 1e-4).max(0.0),
                hh: (hh - 1e-4).max(0.0),
            };
            inner.dist(&other) <= 0.0
        }
        _ => true,
    }
}

fn centre_between(a: &Shape, b: &Shape) -> (f64, f64) {
    let ba = a.bbox();
    let bb = b.bbox();
    let cx = (ba[0].max(bb[0]) + ba[2].min(bb[2])) / 2.0;
    let cy = (ba[1].max(bb[1]) + ba[3].min(bb[3])) / 2.0;
    (cx, cy)
}
