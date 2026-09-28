//! 2D paraxial ray trace of the folded display path for the ray diagram and
//! the eyebox demonstrations.
//!
//! **Folded coordinates** (top view, mm): the display lies in the plane
//! `x = −d`, the thin lens at `x = 0` with clear aperture `±D/2`, the 45°
//! combiner passes through `(L_c, 0)` along direction `(1, −1)` and folds the
//! beam towards the eye, whose pupil sits in the plane `y = −eye_relief`
//! centred at `x = L_c + e` (`e` = lateral eye offset). World light arrives
//! from `+y` through the combiner.
//!
//! Reflection in the combiner maps an unfolded point `(z, y)` to
//! `(L_c − y, L_c − z)`, so the virtual image — unfolded at `z = s_i` —
//! appears above the combiner at `y = L_c + |s_i|`, mirrored left–right
//! (firmware must flip the frame buffer horizontally).
//!
//! Each ray is refracted exactly by the paraxial thin-lens rule
//! `u' = u − y/f` (the ABCD lens matrix), intersected with the tilted mirror
//! and reflected (`d' = d − 2(d·n)n`). Rays that clear the pupil continue
//! into a *reduced eye* — one refracting surface of power `P = P₀ + 1/s`
//! (60 D relaxed, plus accommodation to the image at distance `s`) and index
//! 1.336, retina 22.3 mm behind it — so each field point focuses to one
//! retinal spot (Hecht 5th ed. §5.7.1; Smith 4th ed. ch. 5).

use serde::{Deserialize, Serialize};

use super::paraxial::{analyze, image_distance, Analysis, Extras, INF_MM};
use crate::project::OpticsParams;

/// Reduced eye: refractive index and axial length (mm).
pub const EYE_N: f64 = 1.336;
pub const EYE_LEN_MM: f64 = 22.27;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TraceConfig {
    /// Field points across the display width (odd numbers include the centre).
    pub fields: usize,
    /// Rays per field across the lens aperture.
    pub rays: usize,
    /// Lateral eye offset from the nominal position, mm.
    pub eye_offset_mm: f64,
    /// Length of the dashed virtual back-extension above the combiner, mm.
    pub back_mm: f64,
    /// Eye-offset sweep for the eyebox plot.
    pub sweep_points: usize,
    pub extras: Extras,
}

impl Default for TraceConfig {
    fn default() -> Self {
        TraceConfig {
            fields: 5,
            rays: 7,
            eye_offset_mm: 0.0,
            back_mm: 60.0,
            sweep_points: 121,
            extras: Extras::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Ray {
    pub field: usize,
    /// Display height (mm) and lens height (mm) of the ray.
    pub y0: f64,
    pub yl: f64,
    /// Folded polyline: display → lens → combiner → eye plane (→ retina).
    pub points: Vec<[f64; 2]>,
    pub through_pupil: bool,
    /// Ray through the pupil centre (the chief ray to the eye).
    pub chief: bool,
    /// Dashed virtual continuation above the combiner (where the eye thinks the ray came from).
    pub back: Vec<[f64; 2]>,
    /// Unfolded polyline: display → lens → eye plane.
    pub unfolded: Vec<[f64; 2]>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FieldInfo {
    pub y0: f64,
    pub angle_deg: f64,
    /// Fraction of the pupil width that receives light from this field point.
    pub pupil_fill: f64,
    /// Folded position of the virtual image point (can be ~1 m away).
    pub virtual_point: [f64; 2],
    /// Unfolded virtual image point (z = s_i).
    pub virtual_unfolded: [f64; 2],
    pub retina_x: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Geometry {
    pub display: [[f64; 2]; 2],
    pub lens_x: f64,
    pub lens_half: f64,
    pub combiner: [[f64; 2]; 2],
    pub eye_x: f64,
    pub eye_y: f64,
    pub pupil_half: f64,
    pub eyeball_r: f64,
    pub retina_y: f64,
    pub lc: f64,
    pub eye_relief: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EyeboxSweep {
    pub offset_mm: Vec<f64>,
    /// Horizontal field visible (pupil at least half filled), degrees.
    pub visible_fov_deg: Vec<f64>,
    /// Fraction of the field visible.
    pub visible_frac: Vec<f64>,
    /// Pupil fill per displayed field point (outer index = field).
    pub field_fill: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TraceOut {
    pub analysis: Analysis,
    pub geometry: Geometry,
    pub rays: Vec<Ray>,
    pub fields: Vec<FieldInfo>,
    pub sweep: EyeboxSweep,
    /// Visible horizontal FOV at the requested eye offset, degrees.
    pub visible_fov_deg: f64,
    pub eye_offset_mm: f64,
}

/// Fraction of a pupil (centre `c`, width `p`) lit by a beam covering `[lo, hi]`.
fn overlap_frac(lo: f64, hi: f64, c: f64, p: f64) -> f64 {
    if p <= 1e-9 {
        return if c >= lo && c <= hi { 1.0 } else { 0.0 };
    }
    let a = lo.max(c - p / 2.0);
    let b = hi.min(c + p / 2.0);
    ((b - a) / p).clamp(0.0, 1.0)
}

/// Beam footprint at the eye plane in unfolded coordinates for field `y0`.
fn footprint(f: f64, d: f64, l: f64, dap: f64, y0: f64) -> (f64, f64) {
    let k = 1.0 + l / d - l / f;
    let c = -y0 * l / d;
    let hw = dap * k.abs() / 2.0;
    (c - hw, c + hw)
}

pub fn trace(o: &OpticsParams, cfg: &TraceConfig) -> TraceOut {
    let a = analyze(o, &cfg.extras);
    let f = a.f_mm;
    let d = a.d_mm;
    let dap = a.aperture_mm;
    let lc = o.lens_to_combiner_mm.max(0.0);
    let er = o.eye_relief_mm.max(0.0);
    let l = lc + er;
    let w = o.display_width_mm.max(1e-6);
    let e = cfg.eye_offset_mm;
    let p = cfg.extras.pupil_mm.max(0.0);
    let eye_x = lc + e;
    let si = image_distance(f, d);
    // eye accommodation: object distance from the pupil, mm (0 power for collimated / real)
    let s_eye = if a.image_kind == "virtual" { -si + l } else { INF_MM };
    let p0 = EYE_N / EYE_LEN_MM;
    let power = p0 + 1.0 / s_eye;
    let nf = cfg.fields.clamp(1, 21);
    let nr = cfg.rays.clamp(2, 41);
    let ys: Vec<f64> = if nf == 1 {
        vec![0.0]
    } else {
        (0..nf).map(|i| w / 2.0 - w * i as f64 / (nf - 1) as f64).collect()
    };
    let k = a.k;
    let mut rays = Vec::new();
    let mut fields = Vec::new();
    let mirror = |yl: f64, u: f64| -> Option<[f64; 2]> {
        if 1.0 + u <= 1e-6 {
            return None;
        }
        let t = (lc - yl) / (1.0 + u);
        Some([t, yl + u * t])
    };
    let into_eye = |p3x: f64, u: f64| -> [f64; 2] {
        let r = p3x - eye_x;
        let u_air = -u; // dx per unit descent
        let u_in = (u_air - r * power) / EYE_N;
        [p3x + u_in * EYE_LEN_MM, -er - EYE_LEN_MM]
    };
    let mut comb_lo = f64::INFINITY;
    let mut comb_hi = f64::NEG_INFINITY;
    for (fi, &y0) in ys.iter().enumerate() {
        let mut add_ray = |yl: f64, chief: bool, rays: &mut Vec<Ray>| {
            let u0 = (yl - y0) / d;
            let u = u0 - yl / f;
            let Some(p2) = mirror(yl, u) else { return };
            comb_lo = comb_lo.min(p2[0]);
            comb_hi = comb_hi.max(p2[0]);
            let s = p2[1] + er;
            let p3 = [p2[0] - u * s, -er];
            let through = (p3[0] - eye_x).abs() <= p / 2.0 + 1e-9;
            let mut pts = vec![[-d, y0], [0.0, yl], p2, p3];
            if through {
                pts.push(into_eye(p3[0], u));
            }
            let back = vec![p2, [p2[0] + u * cfg.back_mm, p2[1] + cfg.back_mm]];
            let ye = yl + u * l;
            rays.push(Ray {
                field: fi,
                y0,
                yl,
                points: pts,
                through_pupil: through,
                chief,
                back,
                unfolded: vec![[-d, y0], [0.0, yl], [l, ye]],
            });
        };
        for j in 0..nr {
            let yl = dap / 2.0 - dap * j as f64 / (nr - 1) as f64;
            add_ray(yl, false, &mut rays);
        }
        // chief ray to the pupil centre (unfolded eye height −e)
        let yl_c = if k.abs() > 1e-12 { (y0 * l / d - e) / k } else { 0.0 };
        if yl_c.abs() <= dap / 2.0 + 1e-9 {
            add_ray(yl_c, true, &mut rays);
        }
        let (lo, hi) = footprint(f, d, l, dap, y0);
        let fill = overlap_frac(lo, hi, -e, p);
        let m = if a.image_kind == "virtual" { a.magnification } else { 0.0 };
        let yi = m * y0;
        let vz = if a.image_kind == "virtual" { si } else { -INF_MM };
        let retina = if fill > 0.0 && a.image_kind != "real" {
            // any ray of this field through the pupil lands on the same retinal point
            let yl = if yl_c.abs() <= dap / 2.0 { yl_c } else { (y0 * l / d - e) / k };
            let u = (yl - y0) / d - yl / f;
            mirror(yl, u).map(|p2| {
                let p3x = p2[0] - u * (p2[1] + er);
                into_eye(p3x, u)[0]
            })
        } else {
            None
        };
        fields.push(FieldInfo {
            y0,
            angle_deg: super::paraxial::field_angle(f, d, l, y0).to_degrees(),
            pupil_fill: fill,
            virtual_point: [lc - yi, lc - vz],
            virtual_unfolded: [vz, yi],
            retina_x: retina,
        });
    }
    let half = if comb_hi > comb_lo {
        (comb_hi - comb_lo) / 2.0 + 1.0
    } else {
        dap / 2.0
    };
    let mid = if comb_hi > comb_lo { (comb_hi + comb_lo) / 2.0 } else { lc };
    let geometry = Geometry {
        display: [[-d, w / 2.0], [-d, -w / 2.0]],
        lens_x: 0.0,
        lens_half: dap / 2.0,
        combiner: [[mid - half, lc - (mid - half)], [mid + half, lc - (mid + half)]],
        eye_x,
        eye_y: -er,
        pupil_half: p / 2.0,
        eyeball_r: 12.0,
        retina_y: -er - EYE_LEN_MM,
        lc,
        eye_relief: er,
    };
    // eyebox sweep over eye offsets
    let span = (a.eyebox_mm.max(0.0) / 2.0 + p + 3.0).max(6.0);
    let ns = cfg.sweep_points.clamp(3, 1001);
    let offs = crate::math::linspace(-span, span, ns);
    let fine: Vec<f64> = (0..=80).map(|i| -w / 2.0 + w * i as f64 / 80.0).collect();
    let fine_ang: Vec<f64> = fine
        .iter()
        .map(|&y| super::paraxial::field_angle(f, d, l, y).to_degrees())
        .collect();
    let visible_at = |off: f64| -> (f64, f64) {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        let mut cnt = 0usize;
        for (i, &y) in fine.iter().enumerate() {
            let (a0, b0) = footprint(f, d, l, dap, y);
            if overlap_frac(a0, b0, -off, p) >= 0.5 - 1e-9 {
                cnt += 1;
                lo = lo.min(fine_ang[i]);
                hi = hi.max(fine_ang[i]);
            }
        }
        (if cnt > 0 { hi - lo } else { 0.0 }, cnt as f64 / fine.len() as f64)
    };
    let mut sweep = EyeboxSweep {
        field_fill: vec![Vec::with_capacity(ns); ys.len()],
        ..Default::default()
    };
    for &off in &offs {
        let (fov, frac) = visible_at(off);
        sweep.visible_fov_deg.push(fov);
        sweep.visible_frac.push(frac);
        for (i, &y0) in ys.iter().enumerate() {
            let (lo, hi) = footprint(f, d, l, dap, y0);
            sweep.field_fill[i].push(overlap_frac(lo, hi, -off, p));
        }
    }
    sweep.offset_mm = offs;
    let (vis, _) = visible_at(e);
    TraceOut {
        analysis: a,
        geometry,
        rays,
        fields,
        sweep,
        visible_fov_deg: vis,
        eye_offset_mm: e,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iris() -> OpticsParams {
        OpticsParams {
            lens_diameter_mm: 18.0,
            ..OpticsParams::default()
        }
    }

    #[test]
    fn rays_back_extrapolate_to_the_virtual_image() {
        let o = iris();
        let t = trace(&o, &TraceConfig::default());
        for fi in 0..t.fields.len() {
            let rs: Vec<&Ray> = t.rays.iter().filter(|r| r.field == fi && !r.chief).collect();
            let (a, b) = (&rs[0].unfolded, &rs[rs.len() - 1].unfolded);
            // lines through lens point and eye-plane point, intersect
            let s1 = (a[2][1] - a[1][1]) / (a[2][0] - a[1][0]);
            let s2 = (b[2][1] - b[1][1]) / (b[2][0] - b[1][0]);
            let z = (b[1][1] - a[1][1]) / (s1 - s2);
            let y = a[1][1] + s1 * z;
            let v = t.fields[fi].virtual_unfolded;
            assert!((z - v[0]).abs() < 1e-6 * v[0].abs(), "{z} vs {}", v[0]);
            assert!((y - v[1]).abs() < 1e-6 * v[1].abs().max(1.0));
        }
    }

    #[test]
    fn folded_trace_matches_unfolded_mapping() {
        let t = trace(&iris(), &TraceConfig::default());
        let lc = 8.0;
        for r in &t.rays {
            // eye-plane point: folded x = L_c − unfolded y
            let ye = r.unfolded[2][1];
            assert!((r.points[3][0] - (lc - ye)).abs() < 1e-9);
            assert!((r.points[3][1] + 18.0).abs() < 1e-12);
            // combiner hit lies on the 45° mirror y = L_c − x
            assert!((r.points[2][1] - (lc - r.points[2][0])).abs() < 1e-9);
        }
    }

    #[test]
    fn each_field_focuses_to_one_retinal_point() {
        let t = trace(&iris(), &TraceConfig::default());
        for fi in 0..t.fields.len() {
            let xs: Vec<f64> = t
                .rays
                .iter()
                .filter(|r| r.field == fi && r.through_pupil)
                .map(|r| r.points[4][0])
                .collect();
            assert!(!xs.is_empty());
            for x in &xs {
                assert!((x - xs[0]).abs() < 1e-6, "field {fi}: {xs:?}");
            }
            assert!((t.fields[fi].retina_x.unwrap() - xs[0]).abs() < 1e-6);
        }
        // centre field lands on the eye axis
        let c = &t.fields[2];
        assert!((c.retina_x.unwrap() - t.geometry.eye_x).abs() < 1e-9);
    }

    #[test]
    fn eyebox_sweep_is_consistent_with_formula() {
        let o = iris();
        let t = trace(&o, &TraceConfig::default());
        let eb = t.analysis.eyebox_mm;
        // at the centre the whole field is visible; beyond eyebox/2 + pupil/2 an edge is lost
        assert!((t.visible_fov_deg - t.analysis.hfov_deg).abs() < 1e-9);
        let n = t.sweep.offset_mm.len();
        let mid = n / 2;
        assert!((t.sweep.visible_frac[mid] - 1.0).abs() < 1e-12);
        for (i, &off) in t.sweep.offset_mm.iter().enumerate() {
            // the pupil centre inside the eyebox → every field at least half fills the pupil
            if off.abs() < eb / 2.0 - 1e-6 {
                assert!((t.sweep.visible_frac[i] - 1.0).abs() < 1e-12, "offset {off}");
            }
            if off.abs() > eb / 2.0 + 0.3 {
                assert!(t.sweep.visible_frac[i] < 1.0, "offset {off}");
            }
        }
        // offset eye: chief rays of the far edge are vignetted
        let off = trace(
            &o,
            &TraceConfig {
                eye_offset_mm: eb / 2.0 + 2.0,
                ..Default::default()
            },
        );
        assert!(off.visible_fov_deg < off.analysis.hfov_deg);
        assert!(off.fields.iter().any(|f| f.pupil_fill < 0.5));
    }
}
