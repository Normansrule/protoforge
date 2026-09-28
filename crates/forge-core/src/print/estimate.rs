//! Fast analytic print estimator used for requirement metrics.
//!
//! Instead of generating toolpaths, every layer is **sectioned** (cheap:
//! one pass over the triangles) and summarised by its area A, boundary
//! length L, Euler characteristic χ = (#outer loops − #holes) and number of
//! sharp corners. Offsetting a region inwards by d then follows Steiner's
//! formula for parallel curves:
//!
//! ```text
//!   A(d) = A − L·d + π·χ·d²        L(d) = L − 2π·χ·d
//! ```
//!
//! which gives the perimeter loop lengths (at d = w/2 + k·s), whether they
//! still exist (A(d) > 0 — thin walls lose inner perimeters), the infill
//! area, and — comparing with the sections B layers below and T above — the
//! solid skin area. Infill line counts follow from Cauchy–Crofton (a family
//! of parallel lines spaced s apart crosses a boundary of length L about
//! 2L/(πs) times). Times use the same trapezoidal motion model as the full
//! slicer. The tests check the estimate against full slices of every Iris
//! part.
//!
//! References: L. A. Santaló, *Integral Geometry and Geometric
//! Probability*, Addison-Wesley 1976 (Steiner and Cauchy–Crofton formulas).

use super::motion::{bead_area, e_per_mm, speed, trapezoid_time, LayerStats, SliceStats};
use super::slicer::{extrusion_spacing, layer_tops, Kind, SliceSettings, TriIndex};
use crate::cad::geom2d as g;
use crate::cad::mesh::Mesh;
use crate::math::PI;
use crate::project::PrintParams;

#[derive(Clone, Copy, Debug, Default)]
struct Sec {
    area: f64,
    perim: f64,
    chi: f64,
    loops: usize,
    sharp: usize,
    extent: f64,
}

fn section(idx: &TriIndex, z: f64) -> Sec {
    let loops = idx.section(z);
    let mut s = Sec::default();
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    let cos30 = 30f64.to_radians().cos();
    for l in &loops {
        let a = g::area(l);
        s.area += a;
        s.perim += g::perimeter(l);
        s.chi += if a > 0.0 { 1.0 } else { -1.0 };
        s.loops += 1;
        let n = l.len();
        for i in 0..n {
            let p = l[i];
            lo = [lo[0].min(p[0]), lo[1].min(p[1])];
            hi = [hi[0].max(p[0]), hi[1].max(p[1])];
            let d1 = g::sub(p, l[(i + n - 1) % n]);
            let d2 = g::sub(l[(i + 1) % n], p);
            let den = g::len(d1) * g::len(d2);
            if den > 0.0 && g::dot(d1, d2) / den < cos30 {
                s.sharp += 1;
            }
        }
    }
    if s.loops > 0 {
        s.extent = ((hi[0] - lo[0]) + (hi[1] - lo[1])) / std::f64::consts::SQRT_2;
    }
    s
}

/// Area remaining after an inward offset d (Steiner), clamped at 0.
fn eroded_area(s: &Sec, d: f64) -> f64 {
    (s.area - s.perim * d + PI * s.chi * d * d).max(0.0)
}

fn eroded_len(s: &Sec, d: f64) -> f64 {
    (s.perim - 2.0 * PI * s.chi * d).max(0.0)
}

/// Time of `n` equal runs of total length `len`.
fn runs_time(len: f64, n: f64, v: f64, vj: f64, a: f64) -> f64 {
    if len <= 0.0 {
        return 0.0;
    }
    let n = n.max(1.0);
    n * trapezoid_time(len / n, v, vj, a)
}

pub fn estimate_fast(mesh: &Mesh, pp: &PrintParams, st: &SliceSettings, density: f64) -> SliceStats {
    let per = debug_layers(mesh, pp, st);
    super::motion::totals(&per, None, density, st.cost_per_kg, mesh.volume(), pp.filament_diameter_mm)
}

/// Per-layer modelled statistics.
pub fn debug_layers(mesh: &Mesh, pp: &PrintParams, st: &SliceSettings) -> Vec<LayerStats> {
    let (lo, hi) = mesh.bbox();
    let m = if lo[2].abs() > 1e-9 { mesh.translated(0.0, 0.0, -lo[2]) } else { mesh.clone() };
    let tops = layer_tops(hi[2] - lo[2], st.first_layer_mm, pp.layer_height_mm);
    let idx = TriIndex::new(&m);
    let secs: Vec<Sec> = tops.iter().map(|&(z, h)| section(&idx, z - h / 2.0 + 1e-7)).collect();
    let w = pp.line_width_mm.clamp(0.1, 2.0);
    let p = pp.perimeters.clamp(0, 20) as usize;
    let rho = (pp.infill_pct / 100.0).clamp(0.0, 1.0);
    let ov = (st.infill_overlap_pct / 100.0).clamp(0.0, 0.5);
    let (a, vj) = (st.accel_mm_s2, st.corner_speed_mm_s);
    let n = tops.len();
    let mut per = Vec::with_capacity(n);
    for (i, (&(z, h), s)) in tops.iter().zip(&secs).enumerate() {
        let mut ls = LayerStats { index: i, z, height: h, area_mm2: s.area, ..Default::default() };
        if s.loops == 0 {
            per.push(ls);
            continue;
        }
        let sp = extrusion_spacing(w, h);
        let vt = speed(Kind::Travel, i, pp, st);
        let mut add = |k: Kind, len: f64, t: f64| {
            ls.length_mm[k.code() as usize] += len;
            ls.time_by_kind_s[k.code() as usize] += t;
        };
        // Skirt.
        if i == 0 && st.skirt {
            let len = 2.0 * s.extent * std::f64::consts::FRAC_1_SQRT_2 * 2.0 + 2.0 * PI * 3.0;
            add(Kind::Skirt, len, runs_time(len, 4.0, speed(Kind::Skirt, i, pp, st), vj, a));
        }
        // Perimeters.
        let mut all = true;
        let mut loops_printed = 0.0;
        for k in 0..p {
            let d = 0.5 * w + k as f64 * sp;
            if eroded_area(s, d) <= 0.0 {
                all = false;
                break;
            }
            let len = eroded_len(s, d);
            let kind = if k == 0 { Kind::ExternalPerimeter } else { Kind::Perimeter };
            let runs = (s.sharp as f64).max(s.loops as f64);
            add(kind, len, runs_time(len, runs, speed(kind, i, pp, st), vj, a));
            loops_printed += s.loops as f64;
        }
        // Travel to each perimeter loop.
        add(Kind::Travel, loops_printed * sp, loops_printed * trapezoid_time(sp, vt, vj, a));
        if all {
            let d_inf = if p == 0 { 0.5 * w } else { 0.5 * w + p as f64 * sp - ov * sp };
            let a_in = eroded_area(s, d_inf);
            // Interior = where material continues B layers below and T above.
            let b = st.bottom_layers as usize;
            let t = st.top_layers as usize;
            let a_int = if i >= b && i + t < n {
                a_in.min(eroded_area(&secs[i - b], d_inf)).min(eroded_area(&secs[i + t], d_inf))
            } else {
                0.0
            };
            let solid = (a_in - a_int).max(0.0);
            let l_in = eroded_len(s, d_inf);
            for (area, kind, spacing) in [(solid, Kind::SolidInfill, sp), (a_int, Kind::SparseInfill, if rho > 0.0 { sp / rho } else { 0.0 })] {
                if area <= 1e-9 || spacing <= 0.0 {
                    continue;
                }
                let len = area / spacing;
                let share = if a_in > 0.0 { area / a_in } else { 0.0 };
                // Number of line segments ≈ boundary crossings / 2, and by
                // Cauchy–Crofton a family of lines spaced `spacing` apart
                // crosses a curve of length L about 2L/(π·spacing) times.
                let lines = (share * l_in / (PI * spacing)).max(1.0);
                let v = speed(kind, i, pp, st);
                add(kind, len, runs_time(len, lines, v, vj, a));
                let hop = spacing * std::f64::consts::SQRT_2;
                add(Kind::Travel, lines * hop, lines * trapezoid_time(hop, vt, vj, a));
            }
        }
        let epm = e_per_mm(w, h, pp.filament_diameter_mm);
        let extruded: f64 = ls.length_mm.iter().skip(1).sum();
        ls.e_mm = extruded * epm;
        ls.volume_mm3 = extruded * bead_area(w, h);
        ls.retracts = s.loops.min(8);
        let retract_t = ls.retracts as f64 * 2.0 * st.retract_mm / st.retract_speed_mm_s.max(1.0);
        ls.raw_time_s = ls.time_by_kind_s.iter().sum::<f64>() + retract_t + 0.1 + h / 10.0;
        ls.time_s = ls.raw_time_s.max(st.min_layer_time_s);
        ls.slowed = ls.raw_time_s < st.min_layer_time_s;
        per.push(ls);
    }
    per
}
