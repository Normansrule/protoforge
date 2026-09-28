//! 2D region booleans and offsets by the **winding-number rule**.
//!
//! A region is a set of closed loops, outer boundaries counter-clockwise and
//! holes clockwise. Every point gets a winding number `w` (the number of
//! times the loops wind around it). Booleans then become thresholds:
//!
//! | operation | loops fed in | keep points with |
//! |---|---|---|
//! | union of regions | all loops | `w ≥ 1` |
//! | intersection of k regions | all loops | `w ≥ k` |
//! | difference A − B | A, reversed B | `w ≥ 1` |
//! | offset (Chen–McMains) | raw offset curves | `w ≥ 1` |
//!
//! The algorithm splits every edge at every intersection (a uniform grid
//! keeps this near-linear), samples the winding number just left and right
//! of each sub-edge, keeps the sub-edges that separate kept from discarded
//! space, and chains them back into loops. It is the same idea as the
//! non-zero / positive fill rules used by Clipper (A. Johnson), which most
//! open-source slicers are built on.
//!
//! Reference: X. Chen, S. McMains, "Polygon Offsetting by Computing Winding
//! Numbers", Proc. ASME IDETC/CIE 2005, DETC2005-85513.

use std::collections::HashMap;

use super::geom2d::{self as g, P2};

#[derive(Clone, Copy)]
struct Edge {
    a: P2,
    b: P2,
    loop_id: usize,
    idx: usize,
    loop_len: usize,
}

fn collect_edges(loops: &[Vec<P2>]) -> Vec<Edge> {
    let mut edges = Vec::new();
    for (li, l) in loops.iter().enumerate() {
        let n = l.len();
        if n < 3 {
            continue;
        }
        for i in 0..n {
            let a = l[i];
            let b = l[(i + 1) % n];
            if a != b {
                edges.push(Edge {
                    a,
                    b,
                    loop_id: li,
                    idx: i,
                    loop_len: n,
                });
            }
        }
    }
    edges
}

/// Uniform grid over edge bounding boxes.
struct Grid {
    lo: P2,
    cell: f64,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<u32>>,
}

impl Grid {
    fn new(edges: &[Edge]) -> Grid {
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        let mut total = 0.0;
        for e in edges {
            for p in [e.a, e.b] {
                lo[0] = lo[0].min(p[0]);
                lo[1] = lo[1].min(p[1]);
                hi[0] = hi[0].max(p[0]);
                hi[1] = hi[1].max(p[1]);
            }
            total += g::dist(e.a, e.b);
        }
        let n = edges.len().max(1) as f64;
        let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-9);
        let avg = total / n;
        let w = (hi[0] - lo[0]).max(1e-9);
        let h = (hi[1] - lo[1]).max(1e-9);
        // About one cell per edge, but never smaller than ~1.5 edge lengths.
        let cell = (avg * 1.5).max((w * h / n).sqrt()).max(span / 1024.0).max(1e-9);
        let nx = ((w / cell).floor() as usize + 1).min(1024);
        let ny = ((h / cell).floor() as usize + 1).min(1024);
        let cell = cell.max((hi[0] - lo[0]) / nx as f64).max((hi[1] - lo[1]) / ny as f64) * (1.0 + 1e-9);
        let mut grid = Grid {
            lo,
            cell,
            nx,
            ny,
            cells: vec![Vec::new(); nx * ny],
        };
        for (i, e) in edges.iter().enumerate() {
            let (x0, y0, x1, y1) = grid.range(e);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    grid.cells[y * nx + x].push(i as u32);
                }
            }
        }
        grid
    }
    fn cell_of(&self, v: f64, lo: f64, n: usize) -> usize {
        (((v - lo) / self.cell).floor().max(0.0) as usize).min(n - 1)
    }
    fn range(&self, e: &Edge) -> (usize, usize, usize, usize) {
        let x0 = self.cell_of(e.a[0].min(e.b[0]), self.lo[0], self.nx);
        let x1 = self.cell_of(e.a[0].max(e.b[0]), self.lo[0], self.nx);
        let y0 = self.cell_of(e.a[1].min(e.b[1]), self.lo[1], self.ny);
        let y1 = self.cell_of(e.a[1].max(e.b[1]), self.lo[1], self.ny);
        (x0, y0, x1, y1)
    }
    /// Visit each candidate pair once.
    fn pairs(&self, edges: &[Edge], mut f: impl FnMut(usize, usize) -> bool) {
        for cy in 0..self.ny {
            for cx in 0..self.nx {
                let list = &self.cells[cy * self.nx + cx];
                for (k, &i) in list.iter().enumerate() {
                    let ri = self.range(&edges[i as usize]);
                    for &j in &list[k + 1..] {
                        let rj = self.range(&edges[j as usize]);
                        // First shared cell = (max x0, max y0): only test there.
                        if ri.0.max(rj.0) != cx || ri.1.max(rj.1) != cy {
                            continue;
                        }
                        if !f(i as usize, j as usize) {
                            return;
                        }
                    }
                }
            }
        }
    }
}

fn adjacent(e: &Edge, f: &Edge) -> bool {
    e.loop_id == f.loop_id && ((e.idx + 1) % e.loop_len == f.idx || (f.idx + 1) % f.loop_len == e.idx)
}

/// Proper or T intersection parameters (t on e, u on f), ignoring parallels.
fn intersect(e: &Edge, f: &Edge) -> Option<(f64, f64)> {
    let r = g::sub(e.b, e.a);
    let s = g::sub(f.b, f.a);
    let den = g::cross(r, s);
    let scale = g::len(r) * g::len(s);
    if den.abs() <= 1e-12 * scale {
        return None;
    }
    let qp = g::sub(f.a, e.a);
    let t = g::cross(qp, s) / den;
    let u = g::cross(qp, r) / den;
    let tol = 1e-12;
    if t < -tol || t > 1.0 + tol || u < -tol || u > 1.0 + tol {
        return None;
    }
    Some((t.clamp(0.0, 1.0), u.clamp(0.0, 1.0)))
}

/// True when any two non-adjacent edges of the loops touch or cross.
pub fn has_intersections(loops: &[Vec<P2>]) -> bool {
    let edges = collect_edges(loops);
    if edges.len() < 2 {
        return false;
    }
    let grid = Grid::new(&edges);
    let mut found = false;
    grid.pairs(&edges, |i, j| {
        let (e, f) = (&edges[i], &edges[j]);
        if adjacent(e, f) {
            return true;
        }
        if intersect(e, f).is_some() {
            found = true;
            return false;
        }
        true
    });
    found
}

/// Horizontal-row index of edges for fast winding queries.
struct Rows {
    lo: f64,
    h: f64,
    rows: Vec<Vec<u32>>,
}

impl Rows {
    fn new(edges: &[Edge]) -> Rows {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for e in edges {
            lo = lo.min(e.a[1].min(e.b[1]));
            hi = hi.max(e.a[1].max(e.b[1]));
        }
        let n = ((edges.len() as f64).sqrt() * 2.0).clamp(1.0, 2048.0) as usize;
        let h = ((hi - lo) / n as f64).max(1e-9);
        let mut rows = vec![Vec::new(); n];
        for (i, e) in edges.iter().enumerate() {
            let r0 = (((e.a[1].min(e.b[1]) - lo) / h).floor().max(0.0) as usize).min(n - 1);
            let r1 = (((e.a[1].max(e.b[1]) - lo) / h).floor().max(0.0) as usize).min(n - 1);
            for row in rows.iter_mut().take(r1 + 1).skip(r0) {
                row.push(i as u32);
            }
        }
        Rows { lo, h, rows }
    }
    fn winding(&self, edges: &[Edge], p: P2) -> i32 {
        let n = self.rows.len();
        let r = (p[1] - self.lo) / self.h;
        if r < 0.0 || r >= n as f64 + 1.0 {
            return 0;
        }
        let r = (r.floor() as usize).min(n - 1);
        let mut w = 0;
        for &i in &self.rows[r] {
            let e = &edges[i as usize];
            let (a, b) = (e.a, e.b);
            if a[1] <= p[1] {
                if b[1] > p[1] && g::orient(a, b, p) > 0.0 {
                    w += 1;
                }
            } else if b[1] <= p[1] && g::orient(a, b, p) < 0.0 {
                w -= 1;
            }
        }
        w
    }
}

#[inline]
fn key(p: P2) -> (u64, u64) {
    // Normalise -0.0 so equal points hash equally.
    ((p[0] + 0.0).to_bits(), (p[1] + 0.0).to_bits())
}

/// General winding-rule boolean: returns the boundary loops of
/// `{ p : keep(w(p)) }` (outer loops ccw, holes cw).
pub fn boolean(loops: &[Vec<P2>], keep: impl Fn(i32) -> bool) -> Vec<Vec<P2>> {
    let edges = collect_edges(loops);
    if edges.is_empty() {
        return vec![];
    }
    // 1. split points
    let mut splits: Vec<Vec<(f64, P2)>> = vec![Vec::new(); edges.len()];
    let grid = Grid::new(&edges);
    grid.pairs(&edges, |i, j| {
        let (e, f) = (&edges[i], &edges[j]);
        if adjacent(e, f) {
            return true;
        }
        if let Some((t, u)) = intersect(e, f) {
            let p = if t <= 0.0 {
                e.a
            } else if t >= 1.0 {
                e.b
            } else if u <= 0.0 {
                f.a
            } else if u >= 1.0 {
                f.b
            } else {
                g::lerp(e.a, e.b, t)
            };
            if t > 0.0 && t < 1.0 {
                splits[i].push((t, p));
            }
            if u > 0.0 && u < 1.0 {
                splits[j].push((u, p));
            }
        }
        true
    });
    // 2. sub-edges
    let mut subs: Vec<(P2, P2)> = Vec::with_capacity(edges.len() * 2);
    for (i, e) in edges.iter().enumerate() {
        let s = &mut splits[i];
        s.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut prev = e.a;
        for &(_, p) in s.iter() {
            if p != prev {
                subs.push((prev, p));
                prev = p;
            }
        }
        if e.b != prev {
            subs.push((prev, e.b));
        }
    }
    // 3. classify
    let rows = Rows::new(&edges);
    let mut kept: Vec<(P2, P2)> = Vec::new();
    for &(a, b) in &subs {
        let l = g::dist(a, b);
        if l < 1e-12 {
            continue;
        }
        let m = g::lerp(a, b, 0.5);
        let nr = g::right_normal(a, b);
        let eps = (l * 1e-3).clamp(1e-9, 1e-5);
        let wl = rows.winding(&edges, g::add(m, g::scale(nr, -eps)));
        let wr = rows.winding(&edges, g::add(m, g::scale(nr, eps)));
        let (kl, kr) = (keep(wl), keep(wr));
        if kl && !kr {
            kept.push((a, b));
        } else if kr && !kl {
            kept.push((b, a));
        }
    }
    // 4. chain (coincident duplicates, e.g. from identical input loops,
    // are kept once)
    let mut seen = std::collections::HashSet::with_capacity(kept.len());
    kept.retain(|(a, b)| seen.insert((key(*a), key(*b))));
    chain(kept)
}

fn chain(segs: Vec<(P2, P2)>) -> Vec<Vec<P2>> {
    let mut out_of: HashMap<(u64, u64), Vec<usize>> = HashMap::with_capacity(segs.len());
    for (i, s) in segs.iter().enumerate() {
        out_of.entry(key(s.0)).or_default().push(i);
    }
    let mut used = vec![false; segs.len()];
    let mut loops = Vec::new();
    for start in 0..segs.len() {
        if used[start] {
            continue;
        }
        let mut poly = vec![segs[start].0];
        used[start] = true;
        let mut cur = start;
        let origin = key(segs[start].0);
        let mut guard = 0;
        loop {
            guard += 1;
            if guard > segs.len() + 2 {
                break;
            }
            let end = segs[cur].1;
            if key(end) == origin {
                break;
            }
            poly.push(end);
            let dir_in = g::sub(end, segs[cur].0);
            let cands = match out_of.get(&key(end)) {
                Some(c) => c,
                None => break,
            };
            // Sharpest right turn keeps pinched loops apart.
            let mut best = usize::MAX;
            let mut best_ang = f64::INFINITY;
            for &c in cands {
                if used[c] {
                    continue;
                }
                let d = g::sub(segs[c].1, segs[c].0);
                let ang = g::cross(dir_in, d).atan2(g::dot(dir_in, d));
                if ang < best_ang {
                    best_ang = ang;
                    best = c;
                }
            }
            if best == usize::MAX {
                break;
            }
            used[best] = true;
            cur = best;
        }
        if poly.len() >= 3 {
            let s = g::simplify(&poly, 1e-7);
            if s.len() >= 3 && g::area(&s).abs() > 1e-9 {
                loops.push(s);
            }
        }
    }
    loops
}

pub fn union(loops: &[Vec<P2>]) -> Vec<Vec<P2>> {
    boolean(loops, |w| w >= 1)
}

/// Intersection of several regions (each a set of loops).
pub fn intersection(regions: &[&[Vec<P2>]]) -> Vec<Vec<P2>> {
    let k = regions.len() as i32;
    let all: Vec<Vec<P2>> = regions.iter().flat_map(|r| r.iter().cloned()).collect();
    boolean(&all, move |w| w >= k)
}

/// A − B.
pub fn difference(a: &[Vec<P2>], b: &[Vec<P2>]) -> Vec<Vec<P2>> {
    let mut all: Vec<Vec<P2>> = a.to_vec();
    all.extend(b.iter().map(|l| g::reversed(l)));
    boolean(&all, |w| w >= 1)
}

pub fn region_area(loops: &[Vec<P2>]) -> f64 {
    loops.iter().map(|l| g::area(l)).sum()
}

/// Keep whole loops (that are known not to cross each other) whose two
/// sides straddle the `keep` threshold; orient them with the kept side on
/// the left.
pub fn classify_loops(loops: Vec<Vec<P2>>, keep: impl Fn(i32) -> bool) -> Vec<Vec<P2>> {
    let edges = collect_edges(&loops);
    if edges.is_empty() {
        return vec![];
    }
    let rows = Rows::new(&edges);
    let mut out = Vec::new();
    for l in loops {
        if l.len() < 3 {
            continue;
        }
        // Longest edge gives the most robust sample.
        let n = l.len();
        let mut bi = 0;
        let mut bl = -1.0;
        for i in 0..n {
            let len = g::dist(l[i], l[(i + 1) % n]);
            if len > bl {
                bl = len;
                bi = i;
            }
        }
        let (a, b) = (l[bi], l[(bi + 1) % n]);
        let m = g::lerp(a, b, 0.5);
        let nr = g::right_normal(a, b);
        let eps = (bl * 1e-3).clamp(1e-9, 1e-5);
        let wl = rows.winding(&edges, g::add(m, g::scale(nr, -eps)));
        let wr = rows.winding(&edges, g::add(m, g::scale(nr, eps)));
        match (keep(wl), keep(wr)) {
            (true, false) => out.push(l),
            (false, true) => out.push(g::reversed(&l)),
            _ => {}
        }
    }
    out
}

/// Offset a region (outer ccw, holes cw) by `d` (negative = shrink).
/// Fast path: when every loop's offset is clean and no two offset loops
/// touch, whole loops are classified by winding number (this handles walls
/// that vanish by containment). Otherwise the raw Chen–McMains curves are
/// cleaned by the positive-winding rule.
pub fn offset_region(loops: &[Vec<P2>], d: f64, arc_tol: f64) -> Vec<Vec<P2>> {
    if d == 0.0 {
        return loops.to_vec();
    }
    let mut all_clean = true;
    let raw: Vec<Vec<P2>> = loops
        .iter()
        .filter(|l| l.len() >= 3)
        .map(|l| {
            let (r, ok) = g::raw_offset(l, d, g::Join::Round, arc_tol);
            all_clean &= ok;
            r
        })
        .collect();
    if all_clean && !has_intersections(&raw) {
        return classify_loops(raw, |w| w >= 1);
    }
    union(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::geom2d::{circle, rounded_rect};
    use crate::math::PI;

    fn sq(x0: f64, y0: f64, s: f64) -> Vec<P2> {
        vec![[x0, y0], [x0 + s, y0], [x0 + s, y0 + s], [x0, y0 + s]]
    }

    #[test]
    fn union_of_overlapping_squares() {
        let u = union(&[sq(0.0, 0.0, 10.0), sq(5.0, 5.0, 10.0)]);
        assert_eq!(u.len(), 1);
        assert!((region_area(&u) - 175.0).abs() < 1e-9);
    }

    #[test]
    fn union_of_touching_squares_removes_shared_edge() {
        let u = union(&[sq(0.0, 0.0, 10.0), sq(10.0, 0.0, 10.0)]);
        assert_eq!(u.len(), 1);
        assert!((region_area(&u) - 200.0).abs() < 1e-9);
    }

    #[test]
    fn identical_regions() {
        let a = vec![sq(0.0, 0.0, 10.0)];
        let i = intersection(&[&a, &a]);
        assert!((region_area(&i) - 100.0).abs() < 1e-9, "{i:?}");
        let i3 = intersection(&[&a, &i]);
        assert!((region_area(&i3) - 100.0).abs() < 1e-9);
        assert!(region_area(&difference(&a, &a)).abs() < 1e-9);
    }

    #[test]
    fn intersection_and_difference() {
        let a = vec![sq(0.0, 0.0, 10.0)];
        let b = vec![sq(5.0, 5.0, 10.0)];
        let i = intersection(&[&a, &b]);
        assert!((region_area(&i) - 25.0).abs() < 1e-9);
        let d = difference(&a, &b);
        assert!((region_area(&d) - 75.0).abs() < 1e-9);
        // Hole cut completely inside -> ring with a hole loop.
        let hole = vec![sq(3.0, 3.0, 4.0)];
        let ring = difference(&a, &hole);
        assert_eq!(ring.len(), 2);
        assert!((region_area(&ring) - 84.0).abs() < 1e-9);
    }

    #[test]
    fn offset_square_inward_and_vanish() {
        let r = offset_region(&[sq(0.0, 0.0, 10.0)], -2.0, 0.01);
        assert!((region_area(&r) - 36.0).abs() < 1e-9);
        let gone = offset_region(&[sq(0.0, 0.0, 10.0)], -6.0, 0.01);
        assert!(gone.is_empty() || region_area(&gone).abs() < 1e-9);
    }

    #[test]
    fn offset_ring_wall_vanishes_when_too_thin() {
        // 1.2 mm wall ring shrunk by 0.7 from both sides disappears.
        let outer = rounded_rect(0.0, 0.0, 30.0, 14.0, 3.0, 8);
        let inner = g::reversed(&rounded_rect(0.0, 0.0, 27.6, 11.6, 1.8, 8));
        let r = offset_region(&[outer.clone(), inner.clone()], -0.7, 0.01);
        assert!(region_area(&r).abs() < 1e-6, "area {}", region_area(&r));
        let r2 = offset_region(&[outer, inner], -0.3, 0.01);
        assert!(region_area(&r2) > 10.0);
    }

    #[test]
    fn offset_notch_shape_uses_winding_cleanup() {
        // U-shape whose arms are 2 mm wide: shrink 1.2 removes the arms.
        let u = vec![
            [0.0, 0.0],
            [20.0, 0.0],
            [20.0, 20.0],
            [18.0, 20.0],
            [18.0, 4.0],
            [2.0, 4.0],
            [2.0, 20.0],
            [0.0, 20.0],
        ];
        let r = offset_region(&[u], -1.2, 0.01);
        let a = region_area(&r);
        // Remaining: base bar 20x4 shrunk by 1.2 -> 17.6 x 1.6
        assert!((a - 17.6 * 1.6).abs() < 0.2, "area {a}");
    }

    #[test]
    fn circle_offset_area() {
        let c = circle(0.0, 0.0, 10.0, 256);
        let r = offset_region(&[c], -3.0, 0.005);
        assert!((region_area(&r) - PI * 49.0).abs() / (PI * 49.0) < 2e-3);
        let grown = offset_region(&[sq(0.0, 0.0, 10.0)], 1.0, 0.001);
        let expect = 100.0 + 40.0 + PI;
        assert!((region_area(&grown) - expect).abs() < 0.01, "{}", region_area(&grown));
    }
}
