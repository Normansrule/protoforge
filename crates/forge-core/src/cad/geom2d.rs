//! 2D polygon toolkit: primitive outlines, signed area, vertex-wise (mitre)
//! offsets, raw offsets with round joins, winding numbers and polygon-with-
//! holes triangulation by ear clipping with hole bridging.
//!
//! Conventions: a polygon is a closed loop `Vec<P2>` without a repeated
//! closing vertex. Outer boundaries run counter-clockwise (positive signed
//! area), holes run clockwise, so the material is always on the **left** of
//! every directed edge. A positive offset `d` moves every edge `d` to its
//! **right**, i.e. away from the material (dilation); negative shrinks.
//!
//! References:
//! * M. de Berg, O. Cheong, M. van Kreveld, M. Overmars, *Computational
//!   Geometry: Algorithms and Applications*, 3rd ed., Springer 2008, ch. 3
//!   (polygon triangulation).
//! * D. Eberly, "Triangulation by Ear Clipping", Geometric Tools (2002, rev.
//!   2015) — hole bridging via the mutually-visible vertex search used here.
//! * X. Chen, S. McMains, "Polygon Offsetting by Computing Winding Numbers",
//!   ASME IDETC 2005 — raw offset curves cleaned by a winding-number rule.

use crate::math::PI;

pub type P2 = [f64; 2];

#[inline]
pub fn sub(a: P2, b: P2) -> P2 {
    [a[0] - b[0], a[1] - b[1]]
}
#[inline]
pub fn add(a: P2, b: P2) -> P2 {
    [a[0] + b[0], a[1] + b[1]]
}
#[inline]
pub fn scale(a: P2, k: f64) -> P2 {
    [a[0] * k, a[1] * k]
}
#[inline]
pub fn cross(a: P2, b: P2) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
#[inline]
pub fn dot(a: P2, b: P2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
#[inline]
pub fn len(a: P2) -> f64 {
    a[0].hypot(a[1])
}
#[inline]
pub fn dist(a: P2, b: P2) -> f64 {
    len(sub(a, b))
}
#[inline]
pub fn lerp(a: P2, b: P2, t: f64) -> P2 {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}
/// Unit normal to the right of the direction `a → b`.
#[inline]
pub fn right_normal(a: P2, b: P2) -> P2 {
    let d = sub(b, a);
    let l = len(d).max(1e-300);
    [d[1] / l, -d[0] / l]
}
/// Orientation test: > 0 when `c` is left of `a → b`.
#[inline]
pub fn orient(a: P2, b: P2, c: P2) -> f64 {
    cross(sub(b, a), sub(c, a))
}

/// Signed area (shoelace formula); positive for counter-clockwise loops.
pub fn area(poly: &[P2]) -> f64 {
    let n = poly.len();
    let mut s = 0.0;
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        s += a[0] * b[1] - b[0] * a[1];
    }
    0.5 * s
}

/// Area-weighted centroid of a simple polygon.
pub fn centroid(poly: &[P2]) -> P2 {
    let n = poly.len();
    let (mut cx, mut cy, mut s) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        let c = a[0] * b[1] - b[0] * a[1];
        s += c;
        cx += (a[0] + b[0]) * c;
        cy += (a[1] + b[1]) * c;
    }
    if s.abs() < 1e-300 {
        return poly.first().copied().unwrap_or([0.0, 0.0]);
    }
    [cx / (3.0 * s), cy / (3.0 * s)]
}

/// Closed-loop perimeter length.
pub fn perimeter(poly: &[P2]) -> f64 {
    let n = poly.len();
    (0..n).map(|i| dist(poly[i], poly[(i + 1) % n])).sum()
}

pub fn bbox(poly: &[P2]) -> (P2, P2) {
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for p in poly {
        lo[0] = lo[0].min(p[0]);
        lo[1] = lo[1].min(p[1]);
        hi[0] = hi[0].max(p[0]);
        hi[1] = hi[1].max(p[1]);
    }
    (lo, hi)
}

pub fn reversed(poly: &[P2]) -> Vec<P2> {
    poly.iter().rev().copied().collect()
}

/// Force a loop to the requested orientation.
pub fn oriented(mut poly: Vec<P2>, ccw: bool) -> Vec<P2> {
    if (area(&poly) > 0.0) != ccw {
        poly.reverse();
    }
    poly
}

pub fn translate(poly: &[P2], dx: f64, dy: f64) -> Vec<P2> {
    poly.iter().map(|p| [p[0] + dx, p[1] + dy]).collect()
}

/// Resample a closed loop to `count` points equally spaced by arc length
/// (starting at vertex 0).
pub fn resample_closed(poly: &[P2], count: usize) -> Vec<P2> {
    let n = poly.len();
    if n < 3 || count < 3 {
        return poly.to_vec();
    }
    let mut cum = Vec::with_capacity(n + 1);
    cum.push(0.0);
    for i in 0..n {
        let l = dist(poly[i], poly[(i + 1) % n]);
        cum.push(cum[i] + l);
    }
    let total = cum[n];
    let mut out = Vec::with_capacity(count);
    let mut seg = 0;
    for k in 0..count {
        let s = total * k as f64 / count as f64;
        while seg < n - 1 && cum[seg + 1] < s {
            seg += 1;
        }
        let l = cum[seg + 1] - cum[seg];
        let t = if l > 0.0 { (s - cum[seg]) / l } else { 0.0 };
        out.push(lerp(poly[seg], poly[(seg + 1) % n], t));
    }
    out
}

/// Remove consecutive duplicates and (nearly) collinear vertices.
pub fn simplify(poly: &[P2], tol: f64) -> Vec<P2> {
    let mut pts: Vec<P2> = Vec::with_capacity(poly.len());
    for &p in poly {
        if pts.last().map(|&q| dist(p, q) > tol).unwrap_or(true) {
            pts.push(p);
        }
    }
    while pts.len() > 1 && dist(pts[0], *pts.last().unwrap()) <= tol {
        pts.pop();
    }
    if pts.len() < 3 {
        return pts;
    }
    // Drop vertices whose distance to the chord of their neighbours < tol.
    let mut changed = true;
    while changed && pts.len() > 3 {
        changed = false;
        let n = pts.len();
        let mut keep = vec![true; n];
        let mut i = 0;
        while i < n {
            let a = pts[(i + n - 1) % n];
            let b = pts[i];
            let c = pts[(i + 1) % n];
            let base = dist(a, c);
            let h = if base > 1e-12 { orient(a, c, b).abs() / base } else { dist(a, b) };
            if h < tol && keep[(i + n - 1) % n] {
                keep[i] = false;
                changed = true;
                i += 2; // never remove two neighbours in one sweep
                continue;
            }
            i += 1;
        }
        if changed {
            pts = pts.iter().zip(&keep).filter(|(_, &k)| k).map(|(p, _)| *p).collect();
        }
    }
    pts
}

// ---------------------------------------------------------------- shapes

/// Superellipse (Lamé curve) |x/a|^n + |y/b|^n = 1, counter-clockwise,
/// resampled to `count` points of equal arc length. `n = 2` is an ellipse;
/// larger exponents give the squarer lens shapes common in eyewear.
pub fn superellipse(a: f64, b: f64, n: f64, count: usize) -> Vec<P2> {
    let n = n.max(1.2);
    let dense = (count * 8).max(256);
    let e = 2.0 / n;
    let pts: Vec<P2> = (0..dense)
        .map(|k| {
            let t = 2.0 * PI * k as f64 / dense as f64;
            let (s, c) = t.sin_cos();
            [a * c.signum() * c.abs().powf(e), b * s.signum() * s.abs().powf(e)]
        })
        .collect();
    resample_closed(&pts, count)
}

/// Area of a superellipse: 4ab·Γ(1+1/n)²/Γ(1+2/n) (used by the tests).
pub fn superellipse_area(a: f64, b: f64, n: f64) -> f64 {
    4.0 * a * b * gamma(1.0 + 1.0 / n).powi(2) / gamma(1.0 + 2.0 / n)
}

/// Lanczos approximation of Γ(x) for x > 0 (g = 7, n = 9).
pub fn gamma(x: f64) -> f64 {
    const G: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        return PI / ((PI * x).sin() * gamma(1.0 - x));
    }
    let x = x - 1.0;
    let mut a = G[0];
    let t = x + 7.5;
    for (i, g) in G.iter().enumerate().skip(1) {
        a += g / (x + i as f64);
    }
    (2.0 * PI).sqrt() * t.powf(x + 0.5) * (-t).exp() * a
}

/// Circle, counter-clockwise, starting at angle 0.
pub fn circle(cx: f64, cy: f64, r: f64, n: usize) -> Vec<P2> {
    (0..n)
        .map(|k| {
            let t = 2.0 * PI * k as f64 / n as f64;
            [cx + r * t.cos(), cy + r * t.sin()]
        })
        .collect()
}

/// Axis-aligned rounded rectangle centred at (cx, cy), counter-clockwise.
/// `r` is clamped to half the smaller side; `seg` segments per corner arc.
pub fn rounded_rect(cx: f64, cy: f64, w: f64, h: f64, r: f64, seg: usize) -> Vec<P2> {
    let r = r.clamp(0.0, 0.5 * w.min(h) - 1e-9).max(0.0);
    let (hx, hy) = (0.5 * w - r, 0.5 * h - r);
    let corners = [(hx, -hy, -0.5 * PI), (hx, hy, 0.0), (-hx, hy, 0.5 * PI), (-hx, -hy, PI)];
    let mut out = Vec::new();
    for (x, y, a0) in corners {
        if r <= 1e-9 {
            out.push([cx + x, cy + y]);
            continue;
        }
        for k in 0..=seg {
            let t = a0 + 0.5 * PI * k as f64 / seg as f64;
            out.push([cx + x + r * t.cos(), cy + y + r * t.sin()]);
        }
    }
    out
}

/// Exact area of a rounded rectangle.
pub fn rounded_rect_area(w: f64, h: f64, r: f64) -> f64 {
    w * h - (4.0 - PI) * r * r
}

// ---------------------------------------------------------------- offsets

/// Vertex-wise mitre offset that **preserves the vertex count and order**
/// (so rings at different heights can be stitched into side walls). Every
/// edge moves `d` to its right; each vertex moves to the intersection of its
/// two offset edges, limited to `mitre_limit·|d|`. Correct as long as `|d|`
/// is small compared with the local feature size — used for draft and
/// fillet lofts, where offsets are fractions of a millimetre.
pub fn offset_vertexwise(poly: &[P2], d: f64, mitre_limit: f64) -> Vec<P2> {
    let n = poly.len();
    if d == 0.0 || n < 3 {
        return poly.to_vec();
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let a = poly[(i + n - 1) % n];
        let b = poly[i];
        let c = poly[(i + 1) % n];
        let n1 = right_normal(a, b);
        let n2 = right_normal(b, c);
        let den = 1.0 + dot(n1, n2);
        let m = if den > 1e-9 { scale(add(n1, n2), 1.0 / den) } else { n1 };
        let ml = len(m);
        let m = if ml > mitre_limit { scale(m, mitre_limit / ml) } else { m };
        out.push(add(b, scale(m, d)));
    }
    out
}

/// How convex corners are joined when a polygon grows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Join {
    Mitre,
    Round,
}

/// Raw offset curve of one loop, every edge moved `d` to its right
/// (d > 0 grows the material region, d < 0 shrinks it).
///
/// Returns `(curve, clean)`. When every offset edge keeps its direction
/// (the corner set-backs `|d|·tan(θ/2)` of both ends fit inside the edge),
/// overlapping corners are joined at their mitre point and diverging
/// corners with a round (or mitre) join: the curve is then the exact offset
/// and `clean = true`. Otherwise the curve is the **raw offset** of Chen &
/// McMains — offset edges joined by arcs that sweep each vertex's signed
/// turning angle — whose positive-winding set is the true offset region
/// (see [`super::clip::offset_region`]); `clean = false`.
pub fn raw_offset(poly: &[P2], d: f64, join: Join, arc_tol: f64) -> (Vec<P2>, bool) {
    let n = poly.len();
    if n < 3 || d == 0.0 {
        return (poly.to_vec(), true);
    }
    // Turning angle at every vertex.
    let turn: Vec<f64> = (0..n)
        .map(|i| {
            let a = poly[(i + n - 1) % n];
            let b = poly[i];
            let c = poly[(i + 1) % n];
            let t1 = sub(b, a);
            let t2 = sub(c, b);
            cross(t1, t2).atan2(dot(t1, t2))
        })
        .collect();
    let overlap = |th: f64| if d < 0.0 { th > 1e-12 } else { th < -1e-12 };
    let setback: Vec<f64> = turn.iter().map(|&th| if overlap(th) { d.abs() * (0.5 * th.abs()).tan() } else { 0.0 }).collect();
    let clean = (0..n).all(|i| setback[i] + setback[(i + 1) % n] <= dist(poly[i], poly[(i + 1) % n]) + 1e-12)
        && turn.iter().all(|t| t.abs() < PI - 1e-3);
    let r = d.abs();
    let step = if r > arc_tol { 2.0 * (1.0 - arc_tol / r).clamp(-1.0, 1.0).acos() } else { PI / 2.0 };
    let mut out = Vec::with_capacity(n * 2);
    for i in 0..n {
        let a = poly[(i + n - 1) % n];
        let b = poly[i];
        let c = poly[(i + 1) % n];
        let n1 = right_normal(a, b);
        let n2 = right_normal(b, c);
        let e1 = add(b, scale(n1, d));
        let th = turn[i];
        if th.abs() <= 1e-12 {
            out.push(e1);
            continue;
        }
        if clean && overlap(th) {
            let den = 1.0 + dot(n1, n2);
            out.push(add(b, scale(add(n1, n2), d / den.max(1e-12))));
            continue;
        }
        if clean && join == Join::Mitre {
            let den = 1.0 + dot(n1, n2);
            if den > 0.25 {
                out.push(add(b, scale(add(n1, n2), d / den)));
                continue;
            }
        }
        // Arc from e1 sweeping the signed turning angle.
        let a0 = (e1[1] - b[1]).atan2(e1[0] - b[0]);
        let k = ((th.abs() / step.max(1e-3)).ceil() as usize).clamp(1, 64);
        for j in 0..=k {
            let t = a0 + th * j as f64 / k as f64;
            out.push([b[0] + r * t.cos(), b[1] + r * t.sin()]);
        }
    }
    (out, clean)
}

// ---------------------------------------------------------------- winding

/// Winding number of `p` with respect to a set of closed loops
/// (Sunday's crossing algorithm; points on edges are ambiguous).
pub fn winding(p: P2, loops: &[Vec<P2>]) -> i32 {
    let mut w = 0;
    for poly in loops {
        let n = poly.len();
        for i in 0..n {
            let a = poly[i];
            let b = poly[(i + 1) % n];
            if a[1] <= p[1] {
                if b[1] > p[1] && orient(a, b, p) > 0.0 {
                    w += 1;
                }
            } else if b[1] <= p[1] && orient(a, b, p) < 0.0 {
                w -= 1;
            }
        }
    }
    w
}

pub fn point_in_polygon(p: P2, poly: &[P2]) -> bool {
    let n = poly.len();
    let mut inside = false;
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// A point strictly inside a simple loop (used for nesting tests).
pub fn interior_point(poly: &[P2]) -> P2 {
    let n = poly.len();
    let ccw = area(poly) > 0.0;
    // Nudge the midpoint of the longest edge towards the interior.
    let mut best = 0;
    let mut bl = -1.0;
    for i in 0..n {
        let l = dist(poly[i], poly[(i + 1) % n]);
        if l > bl {
            bl = l;
            best = i;
        }
    }
    let a = poly[best];
    let b = poly[(best + 1) % n];
    let m = lerp(a, b, 0.5);
    let nr = right_normal(a, b);
    let s = if ccw { -1.0 } else { 1.0 };
    let eps = (bl * 1e-3).clamp(1e-6, 1e-2);
    add(m, scale(nr, s * eps))
}

// ---------------------------------------------------------------- triangulation

/// Triangulate a polygon with holes by ear clipping. `outer` must be
/// counter-clockwise and `holes` clockwise (they are re-oriented if not).
/// Returned indices address the concatenation `outer ++ holes[0] ++ …`.
/// Output triangles are counter-clockwise (normal +Z).
pub fn triangulate(outer: &[P2], holes: &[Vec<P2>]) -> Vec<[u32; 3]> {
    // Concatenated vertex store and loops of indices.
    let mut pts: Vec<P2> = Vec::new();
    let mut outer_idx: Vec<usize> = Vec::new();
    let flip_outer = area(outer) < 0.0;
    for k in 0..outer.len() {
        outer_idx.push(k);
    }
    if flip_outer {
        outer_idx.reverse();
    }
    pts.extend_from_slice(outer);
    let mut hole_loops: Vec<Vec<usize>> = Vec::new();
    for h in holes {
        if h.len() < 3 {
            continue;
        }
        let base = pts.len();
        let mut idx: Vec<usize> = (base..base + h.len()).collect();
        if area(h) > 0.0 {
            idx.reverse();
        }
        pts.extend_from_slice(h);
        hole_loops.push(idx);
    }
    // Bridge holes, rightmost first.
    hole_loops.sort_by(|a, b| {
        let ma = a.iter().map(|&i| pts[i][0]).fold(f64::NEG_INFINITY, f64::max);
        let mb = b.iter().map(|&i| pts[i][0]).fold(f64::NEG_INFINITY, f64::max);
        mb.partial_cmp(&ma).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut poly = outer_idx;
    for hole in hole_loops {
        poly = bridge_hole(&pts, poly, &hole);
    }
    ear_clip(&pts, &poly)
}

/// Splice a (clockwise) hole into the (counter-clockwise) polygon through a
/// mutually visible vertex pair (Eberly §3).
fn bridge_hole(pts: &[P2], poly: Vec<usize>, hole: &[usize]) -> Vec<usize> {
    // Hole vertex with maximum x.
    let (hi, &m_idx) = hole
        .iter()
        .enumerate()
        .max_by(|a, b| pts[*a.1][0].partial_cmp(&pts[*b.1][0]).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();
    let m = pts[m_idx];
    let n = poly.len();
    // Cast a ray towards +x and find the closest edge hit.
    let mut best_t = f64::INFINITY;
    let mut best_edge = usize::MAX;
    let mut best_pt = m;
    for i in 0..n {
        let a = pts[poly[i]];
        let b = pts[poly[(i + 1) % n]];
        // Edge must straddle the ray's y (outer is ccw so we want edges going up on the right).
        if (a[1] > m[1]) == (b[1] > m[1]) && a[1] != m[1] && b[1] != m[1] {
            continue;
        }
        if (a[1] - b[1]).abs() < 1e-15 {
            // horizontal edge on the ray line: take the nearer endpoint
            for &(q, _) in &[(a, 0), (b, 1)] {
                if q[1] == m[1] && q[0] >= m[0] && q[0] - m[0] < best_t {
                    best_t = q[0] - m[0];
                    best_edge = i;
                    best_pt = q;
                }
            }
            continue;
        }
        let t = (m[1] - a[1]) / (b[1] - a[1]);
        if !(0.0..=1.0).contains(&t) {
            continue;
        }
        let x = a[0] + t * (b[0] - a[0]);
        if x >= m[0] - 1e-12 && x - m[0] < best_t {
            best_t = x - m[0];
            best_edge = i;
            best_pt = [x, m[1]];
        }
    }
    if best_edge == usize::MAX {
        // Degenerate (hole outside?) — append without bridging.
        let mut out = poly;
        out.extend_from_slice(hole);
        return out;
    }
    let ea = poly[best_edge];
    let eb = poly[(best_edge + 1) % n];
    // Candidate: the edge endpoint with larger x.
    let (mut p_pos, mut p_idx) = if pts[ea][0] > pts[eb][0] { (best_edge, ea) } else { ((best_edge + 1) % n, eb) };
    if dist(best_pt, pts[p_idx]) > 1e-12 {
        // Look for reflex vertices inside triangle (m, i, p); pick the one
        // with the smallest angle to the ray.
        let i_pt = best_pt;
        let p = pts[p_idx];
        let mut best_ang = f64::INFINITY;
        let mut best_d = f64::INFINITY;
        for k in 0..n {
            let v = poly[k];
            if v == p_idx {
                continue;
            }
            let q = pts[v];
            let prev = pts[poly[(k + n - 1) % n]];
            let next = pts[poly[(k + 1) % n]];
            let reflex = orient(prev, q, next) <= 0.0;
            if !reflex {
                continue;
            }
            if point_in_triangle_inclusive(q, m, i_pt, p) || point_in_triangle_inclusive(q, m, p, i_pt) {
                let dx = q[0] - m[0];
                let dy = (q[1] - m[1]).abs();
                if dx <= 0.0 {
                    continue;
                }
                let ang = dy.atan2(dx);
                let dd = dx * dx + dy * dy;
                if ang < best_ang - 1e-12 || ((ang - best_ang).abs() <= 1e-12 && dd < best_d) {
                    best_ang = ang;
                    best_d = dd;
                    p_pos = k;
                    p_idx = v;
                }
            }
        }
    }
    // Build: poly[..=p_pos], hole from m around back to m, p, poly[p_pos+1..]
    let mut out = Vec::with_capacity(poly.len() + hole.len() + 2);
    out.extend_from_slice(&poly[..=p_pos]);
    let hn = hole.len();
    for k in 0..=hn {
        out.push(hole[(hi + k) % hn]);
    }
    out.push(p_idx);
    out.extend_from_slice(&poly[p_pos + 1..]);
    out
}

fn point_in_triangle_inclusive(p: P2, a: P2, b: P2, c: P2) -> bool {
    let d1 = orient(a, b, p);
    let d2 = orient(b, c, p);
    let d3 = orient(c, a, p);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
}

/// Ear clipping of a (weakly) simple counter-clockwise index polygon that
/// may contain duplicated bridge vertices.
fn ear_clip(pts: &[P2], poly: &[usize]) -> Vec<[u32; 3]> {
    let n = poly.len();
    let mut tris = Vec::with_capacity(n.saturating_sub(2));
    if n < 3 {
        return tris;
    }
    let mut prev: Vec<usize> = (0..n).map(|i| (i + n - 1) % n).collect();
    let mut next: Vec<usize> = (0..n).map(|i| (i + 1) % n).collect();
    let mut alive = vec![true; n];
    let p = |i: usize| pts[poly[i]];
    let is_reflex = |i: usize, prev: &Vec<usize>, next: &Vec<usize>| orient(p(prev[i]), p(i), p(next[i])) <= 0.0;
    let mut reflex: Vec<usize> = (0..n).filter(|&i| is_reflex(i, &prev, &next)).collect();
    let mut remaining = n;
    let mut i = 0;
    let mut since_clip = 0;
    while remaining > 3 {
        let a = prev[i];
        let c = next[i];
        let mut ear = false;
        let o = orient(p(a), p(i), p(c));
        if o > 0.0 {
            ear = true;
            let (pa, pb, pc) = (p(a), p(i), p(c));
            for &r in &reflex {
                if !alive[r] || r == a || r == i || r == c {
                    continue;
                }
                let q = p(r);
                // Coincident duplicates (bridge vertices) do not block.
                if poly[r] == poly[a] || poly[r] == poly[i] || poly[r] == poly[c] {
                    continue;
                }
                if q == pa || q == pb || q == pc {
                    continue;
                }
                if !is_reflex(r, &prev, &next) {
                    continue;
                }
                if point_in_triangle_inclusive(q, pa, pb, pc) {
                    ear = false;
                    break;
                }
            }
        }
        if !ear && since_clip > remaining {
            // No ear found in a full pass (degenerate input): clip the
            // flattest convex-or-collinear vertex to guarantee progress.
            let mut best = i;
            let mut best_o = f64::NEG_INFINITY;
            let mut k = i;
            for _ in 0..remaining {
                let ok = orient(p(prev[k]), p(k), p(next[k]));
                if ok > best_o {
                    best_o = ok;
                    best = k;
                }
                k = next[k];
            }
            i = best;
            ear = true;
        }
        if ear {
            let a = prev[i];
            let c = next[i];
            if poly[a] != poly[i] && poly[i] != poly[c] && poly[a] != poly[c] {
                tris.push([poly[a] as u32, poly[i] as u32, poly[c] as u32]);
            }
            alive[i] = false;
            next[a] = c;
            prev[c] = a;
            remaining -= 1;
            since_clip = 0;
            // Neighbours may have become convex.
            reflex.retain(|&r| alive[r] && is_reflex(r, &prev, &next));
            i = a;
        } else {
            i = next[i];
            since_clip += 1;
        }
    }
    let a = prev[i];
    let c = next[i];
    if poly[a] != poly[i] && poly[i] != poly[c] && poly[a] != poly[c] && orient(p(a), p(i), p(c)).abs() > 0.0 {
        tris.push([poly[a] as u32, poly[i] as u32, poly[c] as u32]);
    }
    tris
}

/// Offset an open polyline into a closed band outline. `half` gives the
/// half-width at every path vertex; the far end gets a semicircular cap,
/// the start end is left square. Returned loop is counter-clockwise when
/// the path runs left-to-right.
pub fn band_outline(path: &[P2], half: &[f64], cap_segments: usize) -> Vec<P2> {
    let n = path.len();
    let mut left = Vec::with_capacity(n);
    let mut right = Vec::with_capacity(n);
    for i in 0..n {
        let t = if i == 0 {
            sub(path[1], path[0])
        } else if i == n - 1 {
            sub(path[n - 1], path[n - 2])
        } else {
            let t1 = sub(path[i], path[i - 1]);
            let t2 = sub(path[i + 1], path[i]);
            add(scale(t1, 1.0 / len(t1)), scale(t2, 1.0 / len(t2)))
        };
        let l = len(t).max(1e-300);
        let nl = [-t[1] / l, t[0] / l];
        // Mitre correction so the band keeps its width at corners.
        let corr = if i > 0 && i < n - 1 {
            let t1 = sub(path[i], path[i - 1]);
            let n1 = [-t1[1] / len(t1), t1[0] / len(t1)];
            1.0 / dot(nl, n1).max(0.5)
        } else {
            1.0
        };
        left.push(add(path[i], scale(nl, half[i] * corr)));
        right.push(add(path[i], scale(nl, -half[i] * corr)));
    }
    let mut out: Vec<P2> = right.clone();
    // Round cap at the end.
    let end = path[n - 1];
    let r = half[n - 1];
    let d = sub(path[n - 1], path[n - 2]);
    let a_right = (-d[0]).atan2(d[1]); // angle of the right normal
    for k in 1..cap_segments {
        let t = a_right + PI * k as f64 / cap_segments as f64;
        out.push([end[0] + r * t.cos(), end[1] + r * t.sin()]);
    }
    out.extend(left.iter().rev().copied());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri_area(pts: &[P2], tris: &[[u32; 3]]) -> f64 {
        tris.iter()
            .map(|t| 0.5 * orient(pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]))
            .sum()
    }

    #[test]
    fn square_area_and_orientation() {
        let sq = vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]];
        assert!((area(&sq) - 4.0).abs() < 1e-12);
        assert!((area(&reversed(&sq)) + 4.0).abs() < 1e-12);
        assert!((perimeter(&sq) - 8.0).abs() < 1e-12);
        let c = centroid(&sq);
        assert!((c[0] - 1.0).abs() < 1e-12 && (c[1] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn gamma_matches_known_values() {
        assert!((gamma(1.0) - 1.0).abs() < 1e-12);
        assert!((gamma(0.5) - PI.sqrt()).abs() < 1e-10);
        assert!((gamma(5.0) - 24.0).abs() < 1e-9);
    }

    #[test]
    fn superellipse_area_converges() {
        for n in [2.0, 3.0, 4.5] {
            let p = superellipse(25.0, 19.0, n, 400);
            let exact = superellipse_area(25.0, 19.0, n);
            assert!(((area(&p) - exact) / exact).abs() < 2e-3, "n={n}: {} vs {exact}", area(&p));
        }
        // n = 2 is an ellipse: πab
        assert!((superellipse_area(3.0, 2.0, 2.0) - PI * 6.0).abs() < 1e-9);
    }

    #[test]
    fn rounded_rect_area_matches() {
        let p = rounded_rect(0.0, 0.0, 20.0, 10.0, 3.0, 32);
        assert!((area(&p) - rounded_rect_area(20.0, 10.0, 3.0)).abs() < 0.02);
    }

    #[test]
    fn vertexwise_offset_of_square() {
        let sq = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let o = offset_vertexwise(&sq, 1.0, 4.0);
        assert!((area(&o) - 144.0).abs() < 1e-9);
        let i = offset_vertexwise(&sq, -1.0, 4.0);
        assert!((area(&i) - 64.0).abs() < 1e-9);
    }

    #[test]
    fn triangulates_polygon_with_two_holes() {
        let outer = rounded_rect(0.0, 0.0, 100.0, 40.0, 6.0, 8);
        let h1 = reversed(&circle(-25.0, 0.0, 12.0, 40));
        let h2 = reversed(&superellipse(10.0, 8.0, 3.0, 50));
        let h2 = translate(&h2, 25.0, 2.0);
        let tris = triangulate(&outer, &[h1.clone(), h2.clone()]);
        let mut pts = outer.clone();
        pts.extend(&h1);
        pts.extend(&h2);
        let expect = area(&outer) + area(&h1) + area(&h2);
        let got = tri_area(&pts, &tris);
        assert!((got - expect).abs() < 1e-6 * expect.abs(), "{got} vs {expect}");
        assert_eq!(tris.len(), outer.len() + h1.len() + h2.len() + 2 * 2 - 2);
        assert!(tris.iter().all(|t| orient(pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]) > -1e-9));
    }

    #[test]
    fn triangulates_concave_polygon() {
        // A "C" shape.
        let c = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 2.0], [2.0, 2.0], [2.0, 8.0], [10.0, 8.0], [10.0, 10.0], [0.0, 10.0]];
        let tris = triangulate(&c, &[]);
        assert_eq!(tris.len(), 6);
        assert!((tri_area(&c, &tris) - area(&c)).abs() < 1e-9);
    }

    #[test]
    fn winding_numbers() {
        let sq = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        assert_eq!(winding([5.0, 5.0], std::slice::from_ref(&sq)), 1);
        assert_eq!(winding([15.0, 5.0], std::slice::from_ref(&sq)), 0);
        assert_eq!(winding([5.0, 5.0], &[reversed(&sq)]), -1);
        assert!(point_in_polygon([5.0, 5.0], &sq));
        let ip = interior_point(&sq);
        assert!(point_in_polygon(ip, &sq));
    }

    #[test]
    fn band_is_ccw_and_right_area() {
        let path: Vec<P2> = (0..=10).map(|i| [i as f64 * 10.0, 0.0]).collect();
        let half = vec![2.0; path.len()];
        let b = band_outline(&path, &half, 12);
        let a = area(&b);
        let expect = 100.0 * 4.0 + 0.5 * PI * 4.0;
        assert!(a > 0.0);
        assert!((a - expect).abs() < 0.2, "{a} vs {expect}");
    }
}
