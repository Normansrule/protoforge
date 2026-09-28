//! 2-D geometry used by the router, the pour generator and the design-rule
//! checker. Copper is modelled with two primitive shapes:
//!
//! * [`Shape::Rect`] – an axis-aligned rectangle (pads; components are only
//!   rotated in 90° steps so pads stay axis-aligned).
//! * [`Shape::Capsule`] – a segment swept by a disc (a track of width `2r`,
//!   or a round pad / via when both end points coincide).
//!
//! Clearance between two copper objects is the Euclidean distance between
//! their point sets, which reduces to point–segment, segment–segment and
//! segment–rectangle distances (see e.g. C. Ericson, *Real-Time Collision
//! Detection*, Morgan Kaufmann 2005, §5.1).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Shape {
    /// Centre + half extents.
    Rect { cx: f64, cy: f64, hw: f64, hh: f64 },
    /// Segment a→b swept by radius r.
    Capsule { ax: f64, ay: f64, bx: f64, by: f64, r: f64 },
}

impl Shape {
    pub fn circle(x: f64, y: f64, r: f64) -> Shape {
        Shape::Capsule { ax: x, ay: y, bx: x, by: y, r }
    }
    /// Axis-aligned bounding box `[xmin, ymin, xmax, ymax]`.
    pub fn bbox(&self) -> [f64; 4] {
        match *self {
            Shape::Rect { cx, cy, hw, hh } => [cx - hw, cy - hh, cx + hw, cy + hh],
            Shape::Capsule { ax, ay, bx, by, r } => [ax.min(bx) - r, ay.min(by) - r, ax.max(bx) + r, ay.max(by) + r],
        }
    }
    /// Distance from a point to the shape (0 inside).
    pub fn dist_point(&self, px: f64, py: f64) -> f64 {
        match *self {
            Shape::Rect { cx, cy, hw, hh } => point_rect(px, py, cx, cy, hw, hh),
            Shape::Capsule { ax, ay, bx, by, r } => (point_seg(px, py, ax, ay, bx, by) - r).max(0.0),
        }
    }
    /// Distance from a segment (zero width) to the shape.
    pub fn dist_seg(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
        match *self {
            Shape::Rect { cx, cy, hw, hh } => seg_rect(x0, y0, x1, y1, cx, cy, hw, hh),
            Shape::Capsule { ax, ay, bx, by, r } => (seg_seg(x0, y0, x1, y1, ax, ay, bx, by) - r).max(0.0),
        }
    }
    /// Distance between two shapes (0 when they touch or overlap).
    pub fn dist(&self, o: &Shape) -> f64 {
        match (*self, *o) {
            (Shape::Rect { cx, cy, hw, hh }, Shape::Rect { cx: cx2, cy: cy2, hw: hw2, hh: hh2 }) => {
                let dx = ((cx - cx2).abs() - hw - hw2).max(0.0);
                let dy = ((cy - cy2).abs() - hh - hh2).max(0.0);
                dx.hypot(dy)
            }
            (Shape::Capsule { ax, ay, bx, by, r }, other) | (other, Shape::Capsule { ax, ay, bx, by, r }) => {
                (other.dist_seg(ax, ay, bx, by) - r).max(0.0)
            }
        }
    }
    pub fn area(&self) -> f64 {
        match *self {
            Shape::Rect { hw, hh, .. } => 4.0 * hw * hh,
            Shape::Capsule { ax, ay, bx, by, r } => 2.0 * r * (bx - ax).hypot(by - ay) + std::f64::consts::PI * r * r,
        }
    }
}

pub fn point_rect(px: f64, py: f64, cx: f64, cy: f64, hw: f64, hh: f64) -> f64 {
    let dx = ((px - cx).abs() - hw).max(0.0);
    let dy = ((py - cy).abs() - hh).max(0.0);
    dx.hypot(dy)
}

pub fn point_seg(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let (dx, dy) = (bx - ax, by - ay);
    let l2 = dx * dx + dy * dy;
    let t = if l2 <= 1e-18 { 0.0 } else { (((px - ax) * dx + (py - ay) * dy) / l2).clamp(0.0, 1.0) };
    (px - (ax + t * dx)).hypot(py - (ay + t * dy))
}

fn cross(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ax * by - ay * bx
}

/// Do two closed segments intersect?
#[allow(clippy::too_many_arguments)]
pub fn segs_intersect(x0: f64, y0: f64, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) -> bool {
    let d1 = cross(x3 - x2, y3 - y2, x0 - x2, y0 - y2);
    let d2 = cross(x3 - x2, y3 - y2, x1 - x2, y1 - y2);
    let d3 = cross(x1 - x0, y1 - y0, x2 - x0, y2 - y0);
    let d4 = cross(x1 - x0, y1 - y0, x3 - x0, y3 - y0);
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0)) && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

#[allow(clippy::too_many_arguments)]
pub fn seg_seg(x0: f64, y0: f64, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) -> f64 {
    if segs_intersect(x0, y0, x1, y1, x2, y2, x3, y3) {
        return 0.0;
    }
    point_seg(x0, y0, x2, y2, x3, y3)
        .min(point_seg(x1, y1, x2, y2, x3, y3))
        .min(point_seg(x2, y2, x0, y0, x1, y1))
        .min(point_seg(x3, y3, x0, y0, x1, y1))
}

#[allow(clippy::too_many_arguments)]
pub fn seg_rect(x0: f64, y0: f64, x1: f64, y1: f64, cx: f64, cy: f64, hw: f64, hh: f64) -> f64 {
    // Liang–Barsky clip: if any part of the segment is inside the rectangle, distance is 0.
    let (dx, dy) = (x1 - x0, y1 - y0);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    let mut inside = true;
    for (p, q) in [(-dx, x0 - (cx - hw)), (dx, (cx + hw) - x0), (-dy, y0 - (cy - hh)), (dy, (cy + hh) - y0)] {
        if p.abs() < 1e-15 {
            if q < 0.0 {
                inside = false;
                break;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
            if t0 > t1 {
                inside = false;
                break;
            }
        }
    }
    if inside {
        return 0.0;
    }
    let corners = [(cx - hw, cy - hh), (cx + hw, cy - hh), (cx + hw, cy + hh), (cx - hw, cy + hh)];
    let mut d = point_rect(x0, y0, cx, cy, hw, hh).min(point_rect(x1, y1, cx, cy, hw, hh));
    for (qx, qy) in corners {
        d = d.min(point_seg(qx, qy, x0, y0, x1, y1));
    }
    d
}

/// Uniform-grid spatial hash over bounding boxes, for fast neighbour queries.
pub struct SpatialHash {
    pub cell: f64,
    nx: usize,
    ny: usize,
    x0: f64,
    y0: f64,
    buckets: Vec<Vec<usize>>,
}

impl SpatialHash {
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64, cell: f64) -> Self {
        let nx = (((x1 - x0) / cell).ceil() as usize).max(1) + 2;
        let ny = (((y1 - y0) / cell).ceil() as usize).max(1) + 2;
        SpatialHash { cell, nx, ny, x0: x0 - cell, y0: y0 - cell, buckets: vec![Vec::new(); nx * ny] }
    }
    fn range(&self, b: [f64; 4]) -> (usize, usize, usize, usize) {
        let c = |v: f64, o: f64, n: usize| (((v - o) / self.cell).floor().max(0.0) as usize).min(n - 1);
        (c(b[0], self.x0, self.nx), c(b[1], self.y0, self.ny), c(b[2], self.x0, self.nx), c(b[3], self.y0, self.ny))
    }
    pub fn insert(&mut self, id: usize, b: [f64; 4]) {
        let (i0, j0, i1, j1) = self.range(b);
        for j in j0..=j1 {
            for i in i0..=i1 {
                self.buckets[j * self.nx + i].push(id);
            }
        }
    }
    /// Ids whose boxes may intersect `b` (may contain duplicates).
    pub fn query(&self, b: [f64; 4], out: &mut Vec<usize>) {
        out.clear();
        let (i0, j0, i1, j1) = self.range(b);
        for j in j0..=j1 {
            for i in i0..=i1 {
                out.extend_from_slice(&self.buckets[j * self.nx + i]);
            }
        }
        out.sort_unstable();
        out.dedup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances() {
        let r = Shape::Rect { cx: 0.0, cy: 0.0, hw: 1.0, hh: 0.5 };
        assert!((r.dist_point(2.0, 0.0) - 1.0).abs() < 1e-12);
        assert!((r.dist_point(4.0, 4.5) - 5.0).abs() < 1e-12);
        let t = Shape::Capsule { ax: 3.0, ay: -5.0, bx: 3.0, by: 5.0, r: 0.5 };
        assert!((r.dist(&t) - 1.5).abs() < 1e-12);
        // crossing segment → 0
        assert_eq!(r.dist_seg(-3.0, 0.0, 3.0, 0.2), 0.0);
        // parallel capsules 1 mm apart, radius 0.1 each → 0.8
        let a = Shape::Capsule { ax: 0.0, ay: 0.0, bx: 5.0, by: 0.0, r: 0.1 };
        let b = Shape::Capsule { ax: 1.0, ay: 1.0, bx: 4.0, by: 1.0, r: 0.1 };
        assert!((a.dist(&b) - 0.8).abs() < 1e-12);
        // rect to rect diagonal gap (3,4) → 5
        let r2 = Shape::Rect { cx: 5.0, cy: 5.5, hw: 1.0, hh: 0.5 };
        assert!((r.dist(&r2) - (3.0f64).hypot(4.5)).abs() < 1e-12);
    }
}
