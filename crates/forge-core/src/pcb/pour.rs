//! Ground pours ("copper fill") on both layers.
//!
//! The pour is computed on a fine raster: a cell is copper when its whole
//! square keeps `pour_clearance` from every non-ground object on the bottom
//! layer, `edge_clearance` from the board outline and stays out of pour
//! keep-outs. Connected regions (4-neighbourhood flood fill) that touch no
//! ground via / through-hole pad are *islands* and are removed (they would be
//! floating copper — an antenna, not a ground). The surviving raster is
//! compressed into rectangles (row runs merged vertically) which become
//! Gerber G36/G37 regions.

use serde::{Deserialize, Serialize};

use super::board::Board;
use super::geom::Shape;
use super::route::{Track, Via};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Pour {
    /// 0 = F.Cu, 1 = B.Cu.
    pub layer: u8,
    pub cell_mm: f64,
    /// Filled rectangles `[x0, y0, x1, y1]`.
    pub rects: Vec<[f64; 4]>,
    pub area_mm2: f64,
    pub islands_removed: usize,
    /// Number of connected pour regions kept.
    pub regions: usize,
    /// For connectivity: (anchor kind 'v'/'p'/'t', index, region id).
    #[serde(skip)]
    pub anchors: Vec<(char, usize, usize)>,
    /// Raster of kept region ids (u32::MAX = no copper), row-major.
    #[serde(skip)]
    pub label: Vec<u32>,
    #[serde(skip)]
    pub nx: usize,
    #[serde(skip)]
    pub ny: usize,
}

impl Pour {
    /// Region id under a point, if the pour covers it.
    pub fn region_at(&self, x: f64, y: f64) -> Option<usize> {
        if self.nx == 0 || x < 0.0 || y < 0.0 {
            return None;
        }
        let i = (x / self.cell_mm) as usize;
        let j = (y / self.cell_mm) as usize;
        if i >= self.nx || j >= self.ny {
            return None;
        }
        let l = self.label[j * self.nx + i];
        if l == u32::MAX {
            None
        } else {
            Some(l as usize)
        }
    }
}

pub fn compute(b: &Board, tracks: &[Track], vias: &[Via], layer: u8) -> Pour {
    let on = |top: bool, bot: bool| if layer == 0 { top } else { bot };
    let c = 0.1;
    let nx = (b.width / c).ceil() as usize;
    let ny = (b.height / c).ceil() as usize;
    let hd = c * std::f64::consts::FRAC_1_SQRT_2;
    let gnd = b.gnd;
    let r = &b.rules;
    let mut clear = vec![false; nx * ny];
    let mark = |sh: &Shape, dist: f64, clear: &mut Vec<bool>| {
        let bb = sh.bbox();
        let rr = dist + hd;
        let i0 = ((bb[0] - rr) / c).floor().max(0.0) as usize;
        let j0 = ((bb[1] - rr) / c).floor().max(0.0) as usize;
        let i1 = (((bb[2] + rr) / c).ceil() as usize).min(nx);
        let j1 = (((bb[3] + rr) / c).ceil() as usize).min(ny);
        for j in j0..j1 {
            for i in i0..i1 {
                let (x, y) = ((i as f64 + 0.5) * c, (j as f64 + 0.5) * c);
                if sh.dist_point(x, y) < rr {
                    clear[j * nx + i] = true;
                }
            }
        }
    };
    let pc = r.pour_clearance_mm;
    for pd in b.pads.iter().filter(|p| on(p.top, p.bottom) && p.net_id != gnd) {
        mark(&pd.shape_geom(), pc, &mut clear);
    }
    for t in tracks.iter().filter(|t| t.layer == layer && t.net_id != gnd) {
        mark(&t.shape(), pc, &mut clear);
    }
    for v in vias.iter().filter(|v| v.net_id != gnd) {
        mark(&v.shape(), pc, &mut clear);
    }
    for h in &b.holes {
        mark(&Shape::circle(h.x, h.y, h.d / 2.0), r.hole_clearance_mm, &mut clear);
    }
    for k in b.keepouts.iter().filter(|k| k.no_pour && on(k.top, k.bottom)) {
        let sh = Shape::Rect {
            cx: (k.rect[0] + k.rect[2]) / 2.0,
            cy: (k.rect[1] + k.rect[3]) / 2.0,
            hw: (k.rect[2] - k.rect[0]) / 2.0,
            hh: (k.rect[3] - k.rect[1]) / 2.0,
        };
        mark(&sh, 0.0, &mut clear);
    }
    let e = r.edge_clearance_mm + hd;
    for j in 0..ny {
        for i in 0..nx {
            let (x, y) = ((i as f64 + 0.5) * c, (j as f64 + 0.5) * c);
            if x.min(y).min(b.width - x).min(b.height - y) < e {
                clear[j * nx + i] = true;
            }
        }
    }
    // label connected regions
    let mut label = vec![usize::MAX; nx * ny];
    let mut regions = 0;
    let mut stack = Vec::new();
    for s in 0..nx * ny {
        if clear[s] || label[s] != usize::MAX {
            continue;
        }
        label[s] = regions;
        stack.push(s);
        while let Some(k) = stack.pop() {
            let (i, j) = (k % nx, k / nx);
            let mut nb = |q: usize, stack: &mut Vec<usize>| {
                if !clear[q] && label[q] == usize::MAX {
                    label[q] = regions;
                    stack.push(q);
                }
            };
            if i > 0 {
                nb(k - 1, &mut stack);
            }
            if i + 1 < nx {
                nb(k + 1, &mut stack);
            }
            if j > 0 {
                nb(k - nx, &mut stack);
            }
            if j + 1 < ny {
                nb(k + nx, &mut stack);
            }
        }
        regions += 1;
    }
    // anchors: ground copper that reaches the bottom layer
    let mut anchors = Vec::new();
    let touch = |kind: char, idx: usize, sh: Shape, anchors: &mut Vec<(char, usize, usize)>| {
        let bb = sh.bbox();
        let i0 = (bb[0] / c).floor().max(0.0) as usize;
        let j0 = (bb[1] / c).floor().max(0.0) as usize;
        let i1 = ((bb[2] / c).ceil() as usize).min(nx);
        let j1 = ((bb[3] / c).ceil() as usize).min(ny);
        let mut seen: Vec<usize> = Vec::new();
        for j in j0..j1 {
            for i in i0..i1 {
                let q = j * nx + i;
                if clear[q] {
                    continue;
                }
                let (x, y) = ((i as f64 + 0.5) * c, (j as f64 + 0.5) * c);
                if sh.dist_point(x, y) <= 0.0 && !seen.contains(&label[q]) {
                    seen.push(label[q]);
                    anchors.push((kind, idx, label[q]));
                }
            }
        }
    };
    for (i, v) in vias.iter().enumerate().filter(|(_, v)| v.net_id == gnd) {
        touch('v', i, v.shape(), &mut anchors);
    }
    for (i, pd) in b.pads.iter().enumerate().filter(|(_, p)| on(p.top, p.bottom) && p.net_id == gnd) {
        touch('p', i, pd.shape_geom(), &mut anchors);
    }
    for (i, t) in tracks.iter().enumerate().filter(|(_, t)| t.layer == layer && t.net_id == gnd) {
        touch('t', i, t.shape(), &mut anchors);
    }
    let mut keep = vec![false; regions];
    for a in &anchors {
        keep[a.2] = true;
    }
    let islands_removed = keep.iter().filter(|k| !**k).count();
    // renumber kept regions
    let mut remap = vec![usize::MAX; regions];
    let mut kept = 0;
    for (k, &kp) in keep.iter().enumerate() {
        if kp {
            remap[k] = kept;
            kept += 1;
        }
    }
    for a in anchors.iter_mut() {
        a.2 = remap[a.2];
    }
    let filled = |q: usize| !clear[q] && keep[label[q]];
    // rectangles: row runs merged vertically
    let mut rects: Vec<[f64; 4]> = Vec::new();
    let mut open: Vec<(usize, usize, usize)> = Vec::new(); // (i0, i1, j_start)
    let mut area = 0.0;
    for j in 0..=ny {
        let mut runs = Vec::new();
        if j < ny {
            let mut i = 0;
            while i < nx {
                if filled(j * nx + i) {
                    let s = i;
                    while i < nx && filled(j * nx + i) {
                        i += 1;
                    }
                    runs.push((s, i));
                    area += (i - s) as f64 * c * c;
                } else {
                    i += 1;
                }
            }
        }
        let mut next_open = Vec::new();
        for &(a, bnd, js) in &open {
            if runs.contains(&(a, bnd)) {
                next_open.push((a, bnd, js));
            } else {
                rects.push([a as f64 * c, js as f64 * c, bnd as f64 * c, j as f64 * c]);
            }
        }
        for &(a, bnd) in &runs {
            if !next_open.iter().any(|o: &(usize, usize, usize)| o.0 == a && o.1 == bnd) {
                next_open.push((a, bnd, j));
            }
        }
        open = next_open;
    }
    let label_out: Vec<u32> = (0..nx * ny)
        .map(|q| if filled(q) { remap[label[q]] as u32 } else { u32::MAX })
        .collect();
    Pour {
        layer,
        cell_mm: c,
        rects,
        area_mm2: area,
        islands_removed,
        regions: kept,
        anchors,
        label: label_out,
        nx,
        ny,
    }
}
