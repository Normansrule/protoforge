//! # Molding Lab engine — injection-molding design for manufacture (DFM)
//!
//! Works on any triangle [`Mesh`] (the frame parts from [`crate::cad::frame_parts`]):
//!
//! * **Draft analysis** — for a pull direction **d** each face gets a draft
//!   angle θ = 90° − ∠(**n**, **d**) = asin(**n**·**d**). Faces with
//!   |**n**·**d**| ≥ cos 45° are floors/tops (formed "face-on"); the rest are
//!   *side walls* whose draft must exceed the material's minimum (≈1°) so the
//!   part releases without scuffing. A side wall whose release path along
//!   ±**d** is blocked by other geometry is an **undercut** (found by ray
//!   casting) and needs a side action or lifter. The pull direction is picked
//!   automatically among the world axes and the principal axes of the part
//!   (the one that maximises the minimum side-wall draft) unless given.
//! * **Wall thickness** — rays from each face centroid along −**n** to the
//!   opposite wall (uniform-grid acceleration, Möller–Trumbore intersection).
//! * **Projected area** on the parting plane (rasterised union of the
//!   triangles' projections) → **clamp force** F = A·p_cavity.
//! * **Cooling time** (one-dimensional slab conduction, first Fourier term):
//!   t_c = s² / (π² α) · ln[(8/π²)·(T_melt − T_mold)/(T_eject − T_mold)]
//!   with s the wall thickness and α the thermal diffusivity.
//! * Fill / pack / mold-open estimates → cycle time; tooling and piece-part
//!   cost models (indicative, editable).
//!
//! References: D. V. Rosato, M. G. Rosato, *Injection Molding Handbook*
//! (3rd ed., 2000); T. A. Osswald, *Understanding Polymer Processing* (2010);
//! G. Menges, W. Michaeli, P. Mohren, *How to Make Injection Molds* (2001);
//! T. Möller, B. Trumbore, "Fast, minimum storage ray/triangle intersection",
//! *J. Graphics Tools* 2 (1997); Protolabs injection-molding design
//! guidelines (<https://www.protolabs.com/services/injection-molding/>).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::cad::{Mesh, Part};
use crate::project::{Metric, Project};

pub mod materials;
use materials::{material, steel, Material, MATERIALS, STEELS};

// ---------------------------------------------------------------------------
// Vector helpers
// ---------------------------------------------------------------------------

type V3 = [f64; 3];
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: V3) -> V3 {
    let n = norm(a);
    if n < 1e-15 {
        [0.0, 0.0, 1.0]
    } else {
        scale(a, 1.0 / n)
    }
}

#[derive(Clone, Copy, Debug)]
struct Tri {
    a: V3,
    b: V3,
    c: V3,
    n: V3,
    area: f64,
    centroid: V3,
}

fn triangles(m: &Mesh) -> Vec<Tri> {
    let mut v = Vec::with_capacity(m.triangle_count());
    for t in m.indices.chunks_exact(3) {
        let (a, b, c) = (m.vertex(t[0]), m.vertex(t[1]), m.vertex(t[2]));
        let cr = cross(sub(b, a), sub(c, a));
        let area = 0.5 * norm(cr);
        v.push(Tri {
            a,
            b,
            c,
            n: unit(cr),
            area,
            centroid: scale(add(add(a, b), c), 1.0 / 3.0),
        });
    }
    v
}

/// Signed volume by the divergence theorem (mm³); positive for outward normals.
pub fn volume(m: &Mesh) -> f64 {
    triangles(m).iter().map(|t| dot(t.a, cross(t.b, t.c)) / 6.0).sum::<f64>()
}

pub fn surface_area(m: &Mesh) -> f64 {
    triangles(m).iter().map(|t| t.area).sum()
}

fn bbox(m: &Mesh) -> (V3, V3) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in m.positions.chunks_exact(3) {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    (lo, hi)
}

/// Principal axes of the area-weighted face-centroid distribution (Jacobi eigen-solver).
#[allow(clippy::needless_range_loop)]
fn principal_axes(tris: &[Tri]) -> [V3; 3] {
    let tot: f64 = tris.iter().map(|t| t.area).sum::<f64>().max(1e-12);
    let mut c = [0.0; 3];
    for t in tris {
        c = add(c, scale(t.centroid, t.area / tot));
    }
    let mut m = [[0.0f64; 3]; 3];
    for t in tris {
        let d = sub(t.centroid, c);
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += t.area * d[i] * d[j];
            }
        }
    }
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..50 {
        let (mut p, mut q, mut big) = (0, 1, 0.0);
        for i in 0..3 {
            for j in i + 1..3 {
                if m[i][j].abs() > big {
                    big = m[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if big < 1e-12 {
            break;
        }
        let th = 0.5 * (2.0 * m[p][q]).atan2(m[q][q] - m[p][p]);
        let (s, co) = th.sin_cos();
        let mut r = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        r[p][p] = co;
        r[q][q] = co;
        r[p][q] = s;
        r[q][p] = -s;
        // m = rᵀ m r ; v = v r
        let mut t1 = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                t1[i][j] = (0..3).map(|k| m[i][k] * r[k][j]).sum();
            }
        }
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] = (0..3).map(|k| r[k][i] * t1[k][j]).sum();
            }
        }
        let mut nv = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                nv[i][j] = (0..3).map(|k| v[i][k] * r[k][j]).sum();
            }
        }
        v = nv;
    }
    [
        [v[0][0], v[1][0], v[2][0]],
        [v[0][1], v[1][1], v[2][1]],
        [v[0][2], v[1][2], v[2][2]],
    ]
}

// ---------------------------------------------------------------------------
// Ray casting (uniform grid)
// ---------------------------------------------------------------------------

/// Möller–Trumbore ray/triangle intersection; returns t along the ray.
fn ray_tri(o: V3, d: V3, t: &Tri) -> Option<f64> {
    let e1 = sub(t.b, t.a);
    let e2 = sub(t.c, t.a);
    let p = cross(d, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = sub(o, t.a);
    let u = dot(s, p) * inv;
    if !(-1e-9..=1.0 + 1e-9).contains(&u) {
        return None;
    }
    let q = cross(s, e1);
    let v = dot(d, q) * inv;
    if v < -1e-9 || u + v > 1.0 + 1e-9 {
        return None;
    }
    Some(dot(e2, q) * inv)
}

struct Grid3 {
    lo: V3,
    cell: f64,
    n: [usize; 3],
    cells: Vec<Vec<u32>>,
}

impl Grid3 {
    fn new(tris: &[Tri], lo: V3, hi: V3) -> Grid3 {
        let ext = sub(hi, lo);
        let maxe = ext[0].max(ext[1]).max(ext[2]).max(1e-6);
        let cell = (maxe / 48.0).max(0.8);
        let n = [
            ((ext[0] / cell).ceil() as usize).max(1),
            ((ext[1] / cell).ceil() as usize).max(1),
            ((ext[2] / cell).ceil() as usize).max(1),
        ];
        let mut cells = vec![Vec::new(); n[0] * n[1] * n[2]];
        for (ti, t) in tris.iter().enumerate() {
            let mut a = [0usize; 3];
            let mut b = [0usize; 3];
            for k in 0..3 {
                let mn = t.a[k].min(t.b[k]).min(t.c[k]);
                let mx = t.a[k].max(t.b[k]).max(t.c[k]);
                a[k] = (((mn - lo[k]) / cell).floor().max(0.0) as usize).min(n[k] - 1);
                b[k] = (((mx - lo[k]) / cell).floor().max(0.0) as usize).min(n[k] - 1);
            }
            for x in a[0]..=b[0] {
                for y in a[1]..=b[1] {
                    for z in a[2]..=b[2] {
                        cells[(z * n[1] + y) * n[0] + x].push(ti as u32);
                    }
                }
            }
        }
        Grid3 { lo, cell, n, cells }
    }

    /// Nearest hit t > eps along the ray (ignoring triangle `skip`), up to `tmax`.
    #[allow(clippy::too_many_arguments)]
    fn cast(&self, tris: &[Tri], o: V3, d: V3, skip: usize, tmax: f64, stamp: &mut [u32], gen: u32) -> Option<f64> {
        let step = self.cell * 0.5;
        let mut best: Option<f64> = None;
        let mut s = 0.0;
        while s <= tmax + self.cell {
            if let Some(b) = best {
                if b < s - self.cell {
                    break;
                }
            }
            let p = add(o, scale(d, s));
            let mut idx = [0usize; 3];
            let mut inside = true;
            for k in 0..3 {
                let f = ((p[k] - self.lo[k]) / self.cell).floor();
                if f < 0.0 || f >= self.n[k] as f64 {
                    inside = false;
                    break;
                }
                idx[k] = f as usize;
            }
            if inside {
                for &ti in &self.cells[(idx[2] * self.n[1] + idx[1]) * self.n[0] + idx[0]] {
                    let ti = ti as usize;
                    if ti == skip || stamp[ti] == gen {
                        continue;
                    }
                    stamp[ti] = gen;
                    if let Some(t) = ray_tri(o, d, &tris[ti]) {
                        if t > 1e-4 && t <= tmax && best.map(|b| t < b).unwrap_or(true) {
                            best = Some(t);
                        }
                    }
                }
            }
            s += step;
        }
        best
    }
}

// ---------------------------------------------------------------------------
// Draft analysis
// ---------------------------------------------------------------------------

/// Face classes of the draft map.
pub const CLASS_FLOOR: u8 = 0;
pub const CLASS_OK: u8 = 1;
pub const CLASS_LOW: u8 = 2;
pub const CLASS_ZERO: u8 = 3;
pub const CLASS_UNDERCUT: u8 = 4;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ColoredMesh {
    /// Non-indexed positions (3 vertices per triangle) so faces can be flat-coloured.
    pub positions: Vec<f64>,
    pub indices: Vec<u32>,
    pub colors: Vec<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Candidate {
    pub label: String,
    pub dir: V3,
    pub min_draft_deg: f64,
    pub undercut_area_mm2: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DraftResult {
    pub part: String,
    pub pull: V3,
    pub pull_label: String,
    pub min_draft_required_deg: f64,
    /// Minimum draft over side walls (excluding slivers and undercuts), degrees.
    pub min_draft_deg: f64,
    pub min_face_center: V3,
    pub faces: usize,
    pub side_faces: usize,
    pub area_floor_mm2: f64,
    pub area_ok_mm2: f64,
    pub area_low_mm2: f64,
    pub area_zero_mm2: f64,
    pub area_undercut_mm2: f64,
    pub undercut_faces: usize,
    /// Histogram of side-wall draft (area-weighted): bin lower edges (deg) and areas.
    pub hist_edges: Vec<f64>,
    pub hist_area: Vec<f64>,
    pub candidates: Vec<Candidate>,
    pub mesh: Option<ColoredMesh>,
}

const SIDE_COS: f64 = std::f64::consts::FRAC_1_SQRT_2; // |n·d| < cos 45° → side wall

fn sliver_threshold(tris: &[Tri]) -> f64 {
    let total: f64 = tris.iter().map(|t| t.area).sum();
    (total * 1e-5).max(0.005)
}

/// Undercut flags: a face is an undercut when the ray from its centroid in
/// its release direction (sign(n·d)·d) hits the part again.
fn undercuts(tris: &[Tri], grid: &Grid3, d: V3, diag: f64) -> Vec<bool> {
    let mut stamp = vec![0u32; tris.len()];
    let mut out = vec![false; tris.len()];
    for (i, t) in tris.iter().enumerate() {
        let c = dot(t.n, d);
        if c.abs() >= SIDE_COS || t.area <= 0.0 {
            continue;
        }
        let dir = if c >= 0.0 { d } else { scale(d, -1.0) };
        let o = add(t.centroid, scale(t.n, 1e-3));
        let gen = (i as u32).wrapping_add(1);
        out[i] = grid.cast(tris, o, dir, i, diag, &mut stamp, gen).is_some();
    }
    out
}

struct DraftStats {
    min_draft: f64,
    min_face: usize,
    undercut_area: f64,
}

fn draft_stats(tris: &[Tri], d: V3, under: &[bool], sliver: f64) -> DraftStats {
    let mut min_draft = 90.0f64;
    let mut min_face = 0;
    let mut undercut_area = 0.0;
    for (i, t) in tris.iter().enumerate() {
        let c = dot(t.n, d);
        if c.abs() >= SIDE_COS || t.area < sliver {
            continue;
        }
        if under[i] {
            undercut_area += t.area;
            continue;
        }
        let th = c.abs().asin().to_degrees();
        if th < min_draft {
            min_draft = th;
            min_face = i;
        }
    }
    DraftStats {
        min_draft,
        min_face,
        undercut_area,
    }
}

/// Draft analysis of one mesh. `pull = None` chooses the best direction.
pub fn draft_analysis(name: &str, mesh: &Mesh, pull: Option<V3>, min_draft: f64, with_mesh: bool) -> DraftResult {
    let tris = triangles(mesh);
    if tris.is_empty() {
        return DraftResult {
            part: name.into(),
            ..Default::default()
        };
    }
    let (lo, hi) = bbox(mesh);
    let diag = norm(sub(hi, lo)) + 1.0;
    let grid = Grid3::new(&tris, lo, hi);
    let sliver = sliver_threshold(&tris);
    let pa = principal_axes(&tris);
    let mut cands: Vec<(String, V3)> = vec![
        ("+X".into(), [1.0, 0.0, 0.0]),
        ("+Y".into(), [0.0, 1.0, 0.0]),
        ("+Z".into(), [0.0, 0.0, 1.0]),
    ];
    for (k, a) in pa.iter().enumerate() {
        // skip principal axes that coincide with a world axis
        if cands.iter().take(3).any(|(_, w)| dot(*w, *a).abs() > 0.9999) {
            continue;
        }
        cands.push((format!("principal axis {}", k + 1), unit(*a)));
    }
    let mut evaluated: Vec<Candidate> = Vec::new();
    let mut best: Option<(V3, String, Vec<bool>, DraftStats, f64)> = None;
    let dirs: Vec<(String, V3)> = match pull {
        Some(p) => vec![("custom".into(), unit(p))],
        None => cands,
    };
    for (label, d) in dirs {
        let under = undercuts(&tris, &grid, d, diag);
        let st = draft_stats(&tris, d, &under, sliver);
        // depth of the part along the pull: shallower pulls are easier to mold
        let depth = mesh
            .positions
            .chunks_exact(3)
            .map(|p| dot([p[0], p[1], p[2]], d))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |a, x| (a.0.min(x), a.1.max(x)));
        let depth = depth.1 - depth.0;
        evaluated.push(Candidate {
            label: label.clone(),
            dir: d,
            min_draft_deg: st.min_draft,
            undercut_area_mm2: st.undercut_area,
        });
        let better = match &best {
            None => true,
            Some((_, _, _, b, bd)) => {
                if (st.min_draft - b.min_draft).abs() > 1e-6 {
                    st.min_draft > b.min_draft
                } else if (st.undercut_area - b.undercut_area).abs() > 1e-6 {
                    st.undercut_area < b.undercut_area
                } else {
                    depth < bd - 1e-6
                }
            }
        };
        if better {
            best = Some((d, label, under, st, depth));
        }
    }
    let (d, label, under, st, _) = best.unwrap();
    let mut r = DraftResult {
        part: name.into(),
        pull: d,
        pull_label: label,
        min_draft_required_deg: min_draft,
        min_draft_deg: st.min_draft,
        min_face_center: tris[st.min_face].centroid,
        faces: tris.len(),
        candidates: evaluated,
        ..Default::default()
    };
    let bins = 24;
    r.hist_edges = (0..bins).map(|k| k as f64 * 0.5).collect();
    r.hist_area = vec![0.0; bins];
    let mut classes = Vec::with_capacity(tris.len());
    let mut drafts = Vec::with_capacity(tris.len());
    for (i, t) in tris.iter().enumerate() {
        let c = dot(t.n, d);
        let th = c.abs().asin().to_degrees();
        drafts.push(th);
        let cls = if c.abs() >= SIDE_COS {
            r.area_floor_mm2 += t.area;
            CLASS_FLOOR
        } else if under[i] && t.area >= sliver {
            r.area_undercut_mm2 += t.area;
            r.undercut_faces += 1;
            CLASS_UNDERCUT
        } else {
            r.side_faces += 1;
            let b = ((th / 0.5) as usize).min(bins - 1);
            r.hist_area[b] += t.area;
            if th < 0.1 {
                r.area_zero_mm2 += t.area;
                CLASS_ZERO
            } else if th < min_draft {
                r.area_low_mm2 += t.area;
                CLASS_LOW
            } else {
                r.area_ok_mm2 += t.area;
                CLASS_OK
            }
        };
        classes.push(cls);
    }
    if with_mesh {
        let mut cm = ColoredMesh::default();
        for (i, t) in tris.iter().enumerate() {
            let col = draft_color(classes[i], drafts[i], min_draft);
            for v in [t.a, t.b, t.c] {
                cm.positions.extend_from_slice(&v);
                cm.colors.extend_from_slice(&col);
            }
            let k = (i * 3) as u32;
            cm.indices.extend_from_slice(&[k, k + 1, k + 2]);
        }
        r.mesh = Some(cm);
    }
    r
}

/// Heat-map colour: red (no draft) → amber (below minimum) → green (≥ 2× minimum);
/// floors neutral blue-grey; undercuts magenta.
fn draft_color(cls: u8, th: f64, min: f64) -> [f64; 3] {
    match cls {
        CLASS_FLOOR => [0.55, 0.62, 0.72],
        CLASS_UNDERCUT => [0.85, 0.2, 0.75],
        _ => {
            let f = (th / (2.0 * min.max(0.1))).clamp(0.0, 1.0);
            if f < 0.5 {
                let g = f / 0.5;
                [0.86, 0.18 + 0.52 * g, 0.16]
            } else {
                let g = (f - 0.5) / 0.5;
                [0.86 - 0.7 * g, 0.7 + 0.05 * g, 0.16 + 0.2 * g]
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Wall thickness
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ThicknessResult {
    pub part: String,
    pub min_mm: f64,
    pub max_mm: f64,
    /// Area-weighted median ("nominal wall").
    pub nominal_mm: f64,
    pub p95_mm: f64,
    pub hist_edges: Vec<f64>,
    pub hist_area: Vec<f64>,
    /// Area flagged as thick (sink/void risk) and thin (short-shot risk).
    pub thick_area_mm2: f64,
    pub thin_area_mm2: f64,
    pub thick_center: Option<V3>,
    pub sampled_faces: usize,
    pub mesh: Option<ColoredMesh>,
}

/// Ray-cast wall thickness. `max_samples` limits the number of faces probed
/// (largest faces first) — 0 means all faces.
pub fn thickness(name: &str, mesh: &Mesh, max_samples: usize, with_mesh: bool) -> ThicknessResult {
    let tris = triangles(mesh);
    let mut r = ThicknessResult {
        part: name.into(),
        ..Default::default()
    };
    if tris.is_empty() {
        return r;
    }
    let (lo, hi) = bbox(mesh);
    let diag = norm(sub(hi, lo)) + 1.0;
    let grid = Grid3::new(&tris, lo, hi);
    let mut order: Vec<usize> = (0..tris.len()).collect();
    if max_samples > 0 && tris.len() > max_samples {
        order.sort_by(|&a, &b| tris[b].area.partial_cmp(&tris[a].area).unwrap());
        order.truncate(max_samples);
    }
    let mut stamp = vec![0u32; tris.len()];
    let mut th = vec![f64::NAN; tris.len()];
    let mut gen = 0u32;
    for &i in &order {
        let t = &tris[i];
        let d = scale(t.n, -1.0);
        let o = add(t.centroid, scale(d, 1e-3));
        // local thickness ≈ shortest of five rays in a 45° cone around −n
        // (a single ray along −n reports the full part width at plate edges)
        let helper = if d[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
        let u = unit(cross(d, helper));
        let v = cross(d, u);
        let k45 = std::f64::consts::FRAC_1_SQRT_2;
        let dirs = [
            d,
            unit(add(scale(d, k45), scale(u, k45))),
            unit(add(scale(d, k45), scale(u, -k45))),
            unit(add(scale(d, k45), scale(v, k45))),
            unit(add(scale(d, k45), scale(v, -k45))),
        ];
        let mut best: Option<f64> = None;
        for dir in dirs {
            gen = gen.wrapping_add(1);
            if let Some(h) = grid.cast(&tris, o, dir, i, diag, &mut stamp, gen) {
                let h = h + 1e-3; // the ray started 1 µm inside the wall
                if best.map(|b| h < b).unwrap_or(true) {
                    best = Some(h);
                }
            }
        }
        if let Some(h) = best {
            th[i] = h;
        }
    }
    r.sampled_faces = order.len();
    let mut vals: Vec<(f64, f64)> = order
        .iter()
        .filter(|&&i| th[i].is_finite())
        .map(|&i| (th[i], tris[i].area))
        .collect();
    if vals.is_empty() {
        return r;
    }
    vals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let tot: f64 = vals.iter().map(|v| v.1).sum();
    let q = |p: f64| {
        let mut acc = 0.0;
        for v in &vals {
            acc += v.1;
            if acc >= p * tot {
                return v.0;
            }
        }
        vals.last().unwrap().0
    };
    r.min_mm = vals[0].0;
    r.max_mm = vals.last().unwrap().0;
    r.nominal_mm = q(0.5);
    r.p95_mm = q(0.95);
    let top = (r.p95_mm * 1.25).max(r.nominal_mm * 2.0).max(0.5);
    let bins = 20;
    r.hist_edges = (0..bins).map(|k| k as f64 * top / bins as f64).collect();
    r.hist_area = vec![0.0; bins];
    for v in &vals {
        let b = ((v.0 / top * bins as f64) as usize).min(bins - 1);
        r.hist_area[b] += v.1;
    }
    let thick_lim = (1.5 * r.nominal_mm).max(r.nominal_mm + 0.8);
    let thin_lim = (0.5 * r.nominal_mm).min(0.6);
    let mut best: Option<(f64, V3)> = None;
    for &i in &order {
        if !th[i].is_finite() {
            continue;
        }
        if th[i] > thick_lim {
            r.thick_area_mm2 += tris[i].area;
        }
        if th[i] < thin_lim {
            r.thin_area_mm2 += tris[i].area;
        }
        if best.map(|b| th[i] > b.0).unwrap_or(true) && th[i] <= r.p95_mm * 1.01 {
            best = Some((th[i], add(tris[i].centroid, scale(tris[i].n, -th[i] / 2.0))));
        }
    }
    r.thick_center = best.map(|b| b.1);
    if with_mesh {
        let mut cm = ColoredMesh::default();
        for (i, t) in tris.iter().enumerate() {
            let col = if th[i].is_finite() {
                thickness_color(th[i], r.nominal_mm, thin_lim, thick_lim)
            } else {
                [0.5, 0.5, 0.55]
            };
            for v in [t.a, t.b, t.c] {
                cm.positions.extend_from_slice(&v);
                cm.colors.extend_from_slice(&col);
            }
            let k = (i * 3) as u32;
            cm.indices.extend_from_slice(&[k, k + 1, k + 2]);
        }
        r.mesh = Some(cm);
    }
    r
}

fn thickness_color(t: f64, nominal: f64, thin: f64, thick: f64) -> [f64; 3] {
    if t < thin {
        [0.9, 0.2, 0.2]
    } else if t > thick {
        [0.45, 0.3, 0.85]
    } else {
        let f = ((t - thin) / (thick - thin).max(1e-9)).clamp(0.0, 1.0);
        let _ = nominal;
        // blue-green ramp inside the acceptable band
        [0.15 + 0.1 * f, 0.55 + 0.25 * (1.0 - (2.0 * f - 1.0).abs()), 0.35 + 0.45 * f]
    }
}

// ---------------------------------------------------------------------------
// Projected area, clamp force, cycle time, costs
// ---------------------------------------------------------------------------

/// Area of the part's shadow on the plane ⊥ `d` (mm²), rasterised at `cell` mm.
pub fn projected_area(mesh: &Mesh, d: V3, cell: f64) -> f64 {
    let d = unit(d);
    let helper = if d[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let u = unit(cross(d, helper));
    let v = cross(d, u);
    let pts: Vec<(f64, f64)> = mesh
        .positions
        .chunks_exact(3)
        .map(|p| (dot([p[0], p[1], p[2]], u), dot([p[0], p[1], p[2]], v)))
        .collect();
    if pts.is_empty() {
        return 0.0;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &(x, y) in &pts {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let nx = ((x1 - x0) / cell).ceil() as usize + 1;
    let ny = ((y1 - y0) / cell).ceil() as usize + 1;
    if nx * ny > 16_000_000 {
        return (x1 - x0) * (y1 - y0);
    }
    let mut cov = vec![false; nx * ny];
    for t in mesh.indices.chunks_exact(3) {
        let (a, b, c) = (pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]);
        let area2 = (b.0 - a.0) * (c.1 - a.1) - (c.0 - a.0) * (b.1 - a.1);
        if area2.abs() < 1e-12 {
            continue;
        }
        let i0 = (((a.0.min(b.0).min(c.0)) - x0) / cell).floor().max(0.0) as usize;
        let i1 = ((((a.0.max(b.0).max(c.0)) - x0) / cell).ceil() as usize).min(nx - 1);
        let j0 = (((a.1.min(b.1).min(c.1)) - y0) / cell).floor().max(0.0) as usize;
        let j1 = ((((a.1.max(b.1).max(c.1)) - y0) / cell).ceil() as usize).min(ny - 1);
        for j in j0..=j1 {
            for i in i0..=i1 {
                let (px, py) = (x0 + (i as f64 + 0.5) * cell, y0 + (j as f64 + 0.5) * cell);
                let w0 = (b.0 - a.0) * (py - a.1) - (b.1 - a.1) * (px - a.0);
                let w1 = (c.0 - b.0) * (py - b.1) - (c.1 - b.1) * (px - b.0);
                let w2 = (a.0 - c.0) * (py - c.1) - (a.1 - c.1) * (px - c.0);
                if (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0) || (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0) {
                    cov[j * nx + i] = true;
                }
            }
        }
    }
    cov.iter().filter(|c| **c).count() as f64 * cell * cell
}

/// Cooling time of a slab of thickness `s_mm` (first Fourier term), seconds.
pub fn cooling_time_s(s_mm: f64, alpha_mm2_s: f64, t_melt: f64, t_mold: f64, t_eject: f64) -> f64 {
    let pi2 = std::f64::consts::PI.powi(2);
    let ratio = (8.0 / pi2) * (t_melt - t_mold) / (t_eject - t_mold).max(1e-6);
    if ratio <= 1.0 {
        return 0.0;
    }
    s_mm * s_mm / (pi2 * alpha_mm2_s) * ratio.ln()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CycleBreakdown {
    pub fill_s: f64,
    pub pack_s: f64,
    pub cool_s: f64,
    pub open_eject_close_s: f64,
    pub total_s: f64,
}

/// Cycle estimate: fill at a small-press injection rate, pack/hold ≈ gate
/// freeze (¼ of cooling, ≥ 1 s), cooling from the formula, dry cycle for
/// open/eject/close growing slightly with part depth.
pub fn cycle(shot_cm3: f64, wall_mm: f64, depth_mm: f64, m: &Material) -> CycleBreakdown {
    let cool = cooling_time_s(wall_mm, m.diffusivity_mm2_s, m.t_melt_c, m.t_mold_c, m.t_eject_c);
    let fill = 0.4 + shot_cm3 / 25.0;
    let pack = (0.25 * cool).max(1.0);
    let open = 2.5 + 0.02 * depth_mm;
    CycleBreakdown {
        fill_s: fill,
        pack_s: pack,
        cool_s: cool,
        open_eject_close_s: open,
        total_s: fill + pack + cool + open,
    }
}

/// Indicative tool cost (USD).
pub fn tool_cost_usd(proj_area_mm2: f64, side_actions: usize, steel_id: &str, cavities: u32) -> f64 {
    let s = steel(steel_id);
    let one = s.base_usd + s.per_cm2_usd * proj_area_mm2 / 100.0 + s.side_action_usd * side_actions as f64;
    // extra cavities re-use the mold base: ~65 % of the cavity cost each
    let cav = cavities.max(1) as f64;
    one * (1.0 + 0.65 * (cav - 1.0))
}

/// Indicative press hourly rate (USD/h) by clamp tonnage.
pub fn machine_rate_usd_h(tonnes: f64) -> f64 {
    35.0 + 0.25 * tonnes
}

// ---------------------------------------------------------------------------
// Whole-part process analysis
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PartProcess {
    pub part: String,
    pub pull_label: String,
    pub pull: V3,
    pub volume_cm3: f64,
    pub mass_g: f64,
    pub projected_area_mm2: f64,
    pub clamp_force_kn: f64,
    pub clamp_tonnes: f64,
    pub recommended_press_t: f64,
    pub wall_nominal_mm: f64,
    pub wall_max_mm: f64,
    pub cycle: CycleBreakdown,
    pub min_draft_deg: f64,
    pub undercut_area_mm2: f64,
    pub side_actions: usize,
    pub gate: V3,
    pub flow_length_mm: f64,
    pub flow_ratio: f64,
    pub flow_ratio_limit: f64,
    pub shrink_scale: f64,
    pub tool_cost_usd: f64,
    pub shot_life: f64,
    pub piece: PieceCost,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PieceCost {
    pub material_usd: f64,
    pub machine_usd: f64,
    pub total_usd: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ProcessOptions {
    pub material: String,
    pub steel: String,
    pub cavities: u32,
    pub min_draft_deg: f64,
    /// Allowance for runner/sprue area and safety on clamp force.
    pub clamp_allowance: f64,
    pub scrap_pct: f64,
    /// Runner/sprue material as a fraction of part mass (cold runner).
    pub runner_fraction: f64,
    /// Override machine rate (USD/h); 0 = tonnage-based estimate.
    pub machine_rate_usd_h: f64,
}

impl Default for ProcessOptions {
    fn default() -> Self {
        ProcessOptions {
            material: "PC".into(),
            steel: "P20".into(),
            cavities: 1,
            min_draft_deg: 1.0,
            clamp_allowance: 1.1,
            scrap_pct: 3.0,
            runner_fraction: 0.15,
            machine_rate_usd_h: 0.0,
        }
    }
}

impl ProcessOptions {
    pub fn from_project(p: &Project) -> ProcessOptions {
        let m = &p.design.manufacturing;
        ProcessOptions {
            material: m.molding_material.clone(),
            steel: m.mold_steel.clone(),
            cavities: m.cavities.max(1),
            ..Default::default()
        }
    }
}

/// Full DFM + process estimate for one part (thickness sampled for speed).
pub fn process_part(part: &Part, o: &ProcessOptions, samples: usize) -> PartProcess {
    let m = material(&o.material);
    let dr = draft_analysis(&part.name, &part.mesh, None, o.min_draft_deg, false);
    let th = thickness(&part.name, &part.mesh, samples, false);
    let vol = volume(&part.mesh).abs() / 1000.0;
    let mass = vol * m.density_g_cm3;
    let area = projected_area(&part.mesh, dr.pull, 0.25);
    let force_n = area * m.cavity_pressure_mpa * o.clamp_allowance * o.cavities.max(1) as f64;
    let tonnes = force_n / 9_806.65;
    let (lo, hi) = bbox(&part.mesh);
    let depth = dot(sub(hi, lo), [dr.pull[0].abs(), dr.pull[1].abs(), dr.pull[2].abs()]);
    let wall = if th.p95_mm > 0.0 { th.p95_mm } else { 2.0 };
    let cyc = cycle(vol * o.cavities.max(1) as f64 * (1.0 + o.runner_fraction), wall, depth, m);
    // gate: thickest section (fill thick → thin); flow length = farthest vertex
    let gate = th.thick_center.unwrap_or(scale(add(lo, hi), 0.5));
    let flow = part
        .mesh
        .positions
        .chunks_exact(3)
        .map(|p| norm(sub([p[0], p[1], p[2]], gate)))
        .fold(0.0, f64::max);
    let flow_ratio = if th.nominal_mm > 0.0 { flow / th.nominal_mm } else { 0.0 };
    let side_actions = if dr.area_undercut_mm2 > 1.0 {
        ((dr.undercut_faces as f64).sqrt().ceil() as usize).clamp(1, 4)
    } else {
        0
    };
    let tool = tool_cost_usd(area, side_actions, &o.steel, o.cavities);
    let rate = if o.machine_rate_usd_h > 0.0 {
        o.machine_rate_usd_h
    } else {
        machine_rate_usd_h(tonnes)
    };
    let mat_usd = mass / 1000.0 * (1.0 + o.runner_fraction) * m.price_usd_kg / (1.0 - o.scrap_pct / 100.0);
    let mach_usd = rate * cyc.total_s / 3600.0 / o.cavities.max(1) as f64 / (1.0 - o.scrap_pct / 100.0);
    let mut warnings = Vec::new();
    if dr.min_draft_deg < o.min_draft_deg {
        warnings.push(format!(
            "Side walls with only {:.2}° draft (need ≥ {:.1}°) — add taper in CAD.",
            dr.min_draft_deg, o.min_draft_deg
        ));
    }
    if dr.area_undercut_mm2 > 1.0 {
        warnings.push(format!(
            "{:.0} mm² of undercut surface — needs side actions / lifters (+ tool cost) or a redesign.",
            dr.area_undercut_mm2
        ));
    }
    if th.max_mm > 0.0 && th.thick_area_mm2 > 0.02 * surface_area(&part.mesh) {
        warnings.push(format!(
            "Thick sections up to {:.1} mm (nominal {:.1} mm) — core them out to avoid sink marks and long cooling.",
            th.p95_mm, th.nominal_mm
        ));
    }
    if flow_ratio > m.max_flow_ratio {
        warnings.push(format!(
            "Flow length / wall ratio {:.0} exceeds ~{:.0} for {} — add gates or thicken walls.",
            flow_ratio, m.max_flow_ratio, m.id
        ));
    }
    PartProcess {
        part: part.name.clone(),
        pull_label: dr.pull_label.clone(),
        pull: dr.pull,
        volume_cm3: vol,
        mass_g: mass,
        projected_area_mm2: area,
        clamp_force_kn: force_n / 1000.0,
        clamp_tonnes: tonnes,
        recommended_press_t: (tonnes * 1.25).ceil().max(10.0),
        wall_nominal_mm: th.nominal_mm,
        wall_max_mm: wall,
        cycle: cyc,
        min_draft_deg: dr.min_draft_deg,
        undercut_area_mm2: dr.area_undercut_mm2,
        side_actions,
        gate,
        flow_length_mm: flow,
        flow_ratio,
        flow_ratio_limit: m.max_flow_ratio,
        shrink_scale: 1.0 / (1.0 - m.shrinkage_pct / 100.0),
        tool_cost_usd: tool,
        shot_life: steel(&o.steel).shot_life,
        piece: PieceCost {
            material_usd: mat_usd,
            machine_usd: mach_usd,
            total_usd: mat_usd + mach_usd,
        },
        warnings,
    }
}

/// Per-unit cost of a set of molded parts at quantity `q`, tooling amortised
/// (tools are re-made when the quantity exceeds their shot life).
pub fn unit_cost_at(parts: &[PartProcess], q: f64) -> f64 {
    let q = q.max(1.0);
    parts
        .iter()
        .map(|pp| {
            let tools = (q / pp.shot_life.max(1.0)).ceil().max(1.0);
            pp.piece.total_usd + pp.tool_cost_usd * tools / q
        })
        .sum()
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Analysis {
    pub material: Material,
    pub steel: materials::Steel,
    pub options: ProcessOptions,
    pub parts: Vec<PartProcess>,
    /// Largest per-tool clamp requirement (each part in its own tool), tonnes.
    pub clamp_tonnes: f64,
    /// Longest cycle among the parts (they run in parallel tools), seconds.
    pub cycle_s: f64,
    pub min_draft_deg: f64,
    pub min_draft_part: String,
    pub tooling_total_usd: f64,
    pub quantities: Vec<f64>,
    pub unit_cost: Vec<f64>,
}

impl Default for Material {
    fn default() -> Self {
        MATERIALS[0].clone()
    }
}
impl Default for materials::Steel {
    fn default() -> Self {
        STEELS[1].clone()
    }
}

/// Analyse every frame part of the project.
pub fn analyze(p: &Project, o: &ProcessOptions, samples: usize) -> Analysis {
    let parts = crate::cad::frame_parts_print(p);
    let pp: Vec<PartProcess> = parts
        .iter()
        .filter(|x| x.mesh.triangle_count() > 0)
        .map(|x| process_part(x, o, samples))
        .collect();
    let mut a = Analysis {
        material: material(&o.material).clone(),
        steel: steel(&o.steel).clone(),
        options: o.clone(),
        ..Default::default()
    };
    a.clamp_tonnes = pp.iter().map(|x| x.clamp_tonnes).fold(0.0, f64::max);
    a.cycle_s = pp.iter().map(|x| x.cycle.total_s).fold(0.0, f64::max);
    let (mut md, mut mp) = (90.0f64, String::new());
    for x in &pp {
        if x.min_draft_deg < md {
            md = x.min_draft_deg;
            mp = x.part.clone();
        }
    }
    a.min_draft_deg = if pp.is_empty() { 0.0 } else { md };
    a.min_draft_part = mp;
    a.tooling_total_usd = pp.iter().map(|x| x.tool_cost_usd).sum();
    let mut qs = p.design.manufacturing.quantities.clone();
    if qs.is_empty() {
        qs = vec![1.0, 10.0, 100.0, 1000.0, 10000.0];
    }
    a.unit_cost = qs.iter().map(|&q| unit_cost_at(&pp, q)).collect();
    a.quantities = qs;
    a.parts = pp;
    a
}

// ---------------------------------------------------------------------------
// API
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct PartIn {
    project: Project,
    #[serde(default)]
    part: String,
    #[serde(default)]
    pull: Option<V3>,
    #[serde(default = "one_deg")]
    min_draft_deg: f64,
    #[serde(default = "yes")]
    mesh: bool,
}
fn one_deg() -> f64 {
    1.0
}
fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct AnalyzeIn {
    project: Project,
    #[serde(default)]
    options: Option<ProcessOptions>,
}

#[derive(Serialize)]
struct PartInfo {
    name: String,
    color: String,
    triangles: usize,
    volume_cm3: f64,
    bbox_min: V3,
    bbox_max: V3,
    mesh: Mesh,
}

#[derive(Deserialize)]
struct CoolIn {
    wall_mm: f64,
    #[serde(default)]
    material: String,
}

fn pick_part(p: &Project, name: &str) -> Result<Part, String> {
    let parts = crate::cad::frame_parts_print(p);
    if parts.is_empty() {
        return Err("the project has no frame parts".into());
    }
    Ok(parts.iter().find(|x| x.name == name).cloned().unwrap_or_else(|| parts[0].clone()))
}

/// JSON API entry point for `molding.*` methods.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "molding.materials" => Some(crate::api::to_value(
            serde_json::json!({ "materials": MATERIALS, "steels": STEELS }),
        )),
        "molding.parts" => handle(input, |p: Project| {
            Ok(crate::cad::frame_parts_print(&p)
                .into_iter()
                .map(|x| {
                    let (lo, hi) = bbox(&x.mesh);
                    PartInfo {
                        name: x.name.clone(),
                        color: x.color.clone(),
                        triangles: x.mesh.triangle_count(),
                        volume_cm3: volume(&x.mesh).abs() / 1000.0,
                        bbox_min: lo,
                        bbox_max: hi,
                        mesh: x.mesh,
                    }
                })
                .collect::<Vec<_>>())
        }),
        "molding.draft" => handle(input, |i: PartIn| {
            let part = pick_part(&i.project, &i.part)?;
            Ok(draft_analysis(&part.name, &part.mesh, i.pull, i.min_draft_deg, i.mesh))
        }),
        "molding.thickness" => handle(input, |i: PartIn| {
            let part = pick_part(&i.project, &i.part)?;
            Ok(thickness(&part.name, &part.mesh, 0, i.mesh))
        }),
        "molding.analyze" => handle(input, |i: AnalyzeIn| {
            let o = i.options.unwrap_or_else(|| ProcessOptions::from_project(&i.project));
            Ok(analyze(&i.project, &o, 1500))
        }),
        "molding.cooling" => handle(input, |i: CoolIn| {
            let m = material(if i.material.is_empty() { "PC" } else { &i.material });
            Ok(serde_json::json!({
                "cool_s": cooling_time_s(i.wall_mm, m.diffusivity_mm2_s, m.t_melt_c, m.t_mold_c, m.t_eject_c),
                "material": m,
            }))
        }),
        _ => None,
    }
}

thread_local! {
    static METRICS_CACHE: crate::memo::Slot<Vec<Metric>> = const { std::cell::RefCell::new(None) };
}

/// Metrics this module contributes to requirement verification (memoised on
/// the inputs it reads, because verification re-runs on every project edit).
pub fn metrics(p: &Project) -> Vec<Metric> {
    let key = crate::memo::key(&(
        &p.design.frame,
        &p.design.optics,
        &p.design.pcb,
        &p.design.manufacturing,
        &p.design.extra,
    ));
    crate::memo::memo(&METRICS_CACHE, key, || metrics_uncached(p))
}

fn metrics_uncached(p: &Project) -> Vec<Metric> {
    let o = ProcessOptions::from_project(p);
    let a = analyze(p, &o, 400);
    if a.parts.is_empty() {
        return vec![];
    }
    let worst = a.parts.iter().find(|x| x.part == a.min_draft_part);
    let press = a
        .parts
        .iter()
        .max_by(|x, y| x.clamp_tonnes.partial_cmp(&y.clamp_tonnes).unwrap())
        .unwrap();
    let slow = a
        .parts
        .iter()
        .max_by(|x, y| x.cycle.total_s.partial_cmp(&y.cycle.total_s).unwrap())
        .unwrap();
    let under: f64 = a.parts.iter().map(|x| x.undercut_area_mm2).sum();
    vec![
        Metric::new(
            "molding.min_draft_deg",
            "Minimum side-wall draft",
            a.min_draft_deg,
            "deg",
            "molding",
        )
        .with_note(&format!(
            "{} (pull {}){}",
            a.min_draft_part,
            worst.map(|w| w.pull_label.clone()).unwrap_or_default(),
            if under > 1.0 {
                format!("; {:.0} mm² undercut needs side actions", under)
            } else {
                String::new()
            }
        )),
        Metric::new(
            "molding.clamp_tonnage_t",
            "Clamp force (largest tool)",
            a.clamp_tonnes,
            "t",
            "molding",
        )
        .with_note(&format!(
            "{}: {:.0} mm² projected × {:.0} MPa {} × {:.2} allowance",
            press.part, press.projected_area_mm2, a.material.cavity_pressure_mpa, a.material.id, o.clamp_allowance
        )),
        Metric::new(
            "molding.cycle_time_s",
            "Molding cycle time (slowest part)",
            a.cycle_s,
            "s",
            "molding",
        )
        .with_note(&format!(
            "{}: fill {:.1} + pack {:.1} + cool {:.1} (wall {:.2} mm) + open/eject {:.1} s",
            slow.part, slow.cycle.fill_s, slow.cycle.pack_s, slow.cycle.cool_s, slow.wall_max_mm, slow.cycle.open_eject_close_s
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frustum-shaped box: bottom w×d at z=0, top shrunk by h·tan(taper) on every side.
    pub fn tapered_box(w: f64, d: f64, h: f64, taper_deg: f64) -> Mesh {
        let s = h * taper_deg.to_radians().tan();
        let v = [
            [0.0, 0.0, 0.0],
            [w, 0.0, 0.0],
            [w, d, 0.0],
            [0.0, d, 0.0],
            [s, s, h],
            [w - s, s, h],
            [w - s, d - s, h],
            [s, d - s, h],
        ];
        let positions = v.iter().flat_map(|p| p.iter().copied()).collect();
        let indices = vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0, 4, 3, 4, 7,
        ];
        Mesh { positions, indices }
    }

    #[test]
    fn draft_of_tapered_box_equals_taper() {
        for taper in [0.5, 1.5, 3.0] {
            let m = tapered_box(40.0, 20.0, 10.0, taper);
            let r = draft_analysis("box", &m, None, 1.0, false);
            assert!((r.min_draft_deg - taper).abs() < 1e-6, "taper {taper}: got {}", r.min_draft_deg);
            assert_eq!(r.pull_label, "+Z");
            assert_eq!(r.undercut_faces, 0);
        }
        // a straight box has zero draft
        let r = draft_analysis("box", &tapered_box(40.0, 20.0, 10.0, 0.0), Some([0.0, 0.0, 1.0]), 1.0, false);
        assert!(r.min_draft_deg.abs() < 1e-9);
    }

    #[test]
    fn projected_area_and_volume_of_box() {
        let m = tapered_box(40.0, 20.0, 10.0, 0.0);
        let a = projected_area(&m, [0.0, 0.0, 1.0], 0.1);
        assert!((a - 800.0).abs() / 800.0 < 0.01, "{a}");
        assert!((volume(&m) - 8000.0).abs() < 1e-6);
        let side = projected_area(&m, [1.0, 0.0, 0.0], 0.1);
        assert!((side - 200.0).abs() / 200.0 < 0.02, "{side}");
    }

    #[test]
    fn thickness_of_plate() {
        let m = tapered_box(40.0, 30.0, 2.5, 0.0);
        let t = thickness("plate", &m, 0, false);
        assert!((t.nominal_mm - 2.5).abs() < 1e-6, "{}", t.nominal_mm);
    }

    #[test]
    fn cooling_time_spot_check() {
        // PC, 2 mm wall: t = 4/(π²·0.13)·ln(0.8106·210/37) ≈ 3.118·1.5285 ≈ 4.77 s
        let m = material("PC");
        let t = cooling_time_s(2.0, m.diffusivity_mm2_s, m.t_melt_c, m.t_mold_c, m.t_eject_c);
        let expect = 4.0 / (std::f64::consts::PI.powi(2) * 0.13) * ((8.0 / std::f64::consts::PI.powi(2)) * 210.0 / 37.0).ln();
        assert!((t - expect).abs() < 1e-9);
        assert!((t - 4.77).abs() < 0.05, "{t}");
        // quadratic in thickness
        let t2 = cooling_time_s(4.0, m.diffusivity_mm2_s, m.t_melt_c, m.t_mold_c, m.t_eject_c);
        assert!((t2 / t - 4.0).abs() < 1e-9);
    }

    #[test]
    fn clamp_force_units() {
        // 800 mm² × 55 MPa = 44 kN = 4.49 tf (×1.1 allowance = 4.93 tf)
        let part = Part {
            name: "box".into(),
            mesh: tapered_box(40.0, 20.0, 10.0, 2.0),
            color: String::new(),
            explode: [0.0; 3],
        };
        let o = ProcessOptions::default();
        let r = process_part(&part, &o, 0);
        let expect_t = r.projected_area_mm2 * 55.0 * 1.1 / 9806.65;
        assert!((r.clamp_tonnes - expect_t).abs() < 1e-9);
        assert!((r.projected_area_mm2 - 800.0).abs() < 10.0);
        assert!((r.clamp_tonnes - 4.93).abs() < 0.1, "{}", r.clamp_tonnes);
    }

    #[test]
    fn undercut_detected_by_raycast() {
        // an overhang: a box whose top is wider than its bottom seen along +Z
        // flips draft sign on the side walls → shadowed by nothing, but a
        // stacked second box above a first larger box creates a shadowed
        // step: we test a "mushroom" (big cap over thin stem) for undercuts.
        let stem = tapered_box(4.0, 4.0, 10.0, 0.0);
        let cap = tapered_box(20.0, 20.0, 2.0, 0.0);
        let mut m = Mesh::default();
        let off = [8.0, 8.0, 0.0];
        for p in stem.positions.chunks_exact(3) {
            m.positions.extend_from_slice(&[p[0] + off[0], p[1] + off[1], p[2]]);
        }
        m.indices.extend(stem.indices.iter().copied());
        let n0 = (m.positions.len() / 3) as u32;
        for p in cap.positions.chunks_exact(3) {
            m.positions.extend_from_slice(&[p[0], p[1], p[2] + 10.0]);
        }
        m.indices.extend(cap.indices.iter().map(|i| i + n0));
        let r = draft_analysis("mushroom", &m, Some([0.0, 0.0, 1.0]), 1.0, false);
        // the stem's vertical walls release towards ±x/±y? no: pulling along +Z
        // the stem walls are vertical (0° draft); tilt the pull slightly so the
        // walls have a release sign and the cap shadows them
        let r2 = draft_analysis("mushroom", &m, Some([0.05, 0.0, 1.0]), 1.0, false);
        assert!(r.min_draft_deg < 0.1);
        assert!(r2.undercut_faces > 0, "cap should shadow the stem walls");
    }

    #[test]
    fn iris_metrics_present() {
        let p = Project::iris();
        let m = metrics(&p);
        let get = |k: &str| m.iter().find(|x| x.key == k).map(|x| x.value);
        assert!(get("molding.clamp_tonnage_t").unwrap() > 0.5);
        assert!(get("molding.cycle_time_s").unwrap() > 5.0);
        assert!(get("molding.min_draft_deg").is_some());
    }
}
