//! First-order (paraxial) analysis of the Iris "birdbath / beam-splitter"
//! display: micro-display → thin collimating lens (focal length f, clear
//! aperture D) → 45° partially reflective combiner → eye.
//!
//! The combiner is a flat fold mirror, so the optics can be analysed
//! *unfolded* along one straight axis: display at `−d`, lens at `0`, eye at
//! `L = lens_to_combiner + eye_relief`.
//!
//! **Imaging.** Gaussian thin-lens equation with real-is-positive distances
//! (Hecht, *Optics* 5th ed., §5.2.3):
//!
//! ```text
//! 1/s_o + 1/s_i = 1/f      s_o = d
//! s_i = f d / (d − f)      (negative ⇒ virtual image on the display side)
//! m   = −s_i / s_o         (lateral magnification, +: upright)
//! ```
//!
//! With `d` just inside `f` the image is virtual, upright and far away — a
//! simple magnifier. Its distance from the eye is `|s_i| + L`.
//!
//! **Field of view (FOV).** The angle the virtual image subtends at the eye,
//! `FOV = 2·atan(m·w/2 / (|s_i| + L))`. In the collimated limit (`d → f`)
//! this becomes `2·atan(w / 2f)` and no longer depends on the eye position.
//! **Pixels per degree (PPD)** = pixels across / FOV.
//!
//! **Ray-transfer (ABCD) matrices** (Hecht §6.2; Smith, *Modern Optical
//! Engineering* 4th ed., ch. 2): `[y, u]ᵀ` evolves by
//! `T(t) = [[1, t],[0, 1]]` through a gap and `Λ(f) = [[1, 0],[−1/f, 1]]`
//! through a thin lens. Display → eye is `M = T(L)·Λ(f)·T(d)`.
//!
//! **Eyebox.** The lens aperture is the stop. A ray from display height `y₀`
//! through lens height `y_l` reaches the eye plane at
//! `y_e = k·y_l − y₀·L/d` with `k = 1 + L/d − L/f` (= `(|s_i| + L)/|s_i|`
//! for a virtual image). The beam from each field point therefore covers
//! `D·|k|` at the eye, shifted by `−y₀L/d`; the pupil centre sees the whole
//! field while `|x| ≤ (D|k| − w·L/d)/2`, i.e.
//!
//! ```text
//! eyebox = D·|k| − w·L/d   →   D − 2·L·tan(FOV/2)  (collimated limit)
//! ```
//!
//! the textbook magnifier result (Rolland & Hua 2005; Kress 2020, ch. 5).
//! A pupil of diameter `p` changes it to `eyebox − p` (every field point
//! fills the whole pupil) or `eyebox + p` (at least some light from every
//! field point enters the pupil).
//!
//! **Photometry.** An ideal magnifier conserves luminance, so the virtual
//! image luminance is `L_display · T_lens · R_combiner`; the world is seen
//! through the combiner with `T = 1 − R` (absorption neglected). The ambient
//! contrast ratio against a background of luminance `L_bg` is
//! `(L_img + T·L_bg) / (T·L_bg)`.

use serde::{Deserialize, Serialize};

use crate::project::OpticsParams;

/// A 2 × 2 ray-transfer matrix acting on `[y (mm), u (rad)]`.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Abcd {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
}

/// `M₁ * M₂` applies `M₂` first (matrix product).
impl std::ops::Mul for Abcd {
    type Output = Abcd;
    fn mul(self, r: Abcd) -> Abcd {
        Abcd {
            a: self.a * r.a + self.b * r.c,
            b: self.a * r.b + self.b * r.d,
            c: self.c * r.a + self.d * r.c,
            d: self.c * r.b + self.d * r.d,
        }
    }
}

impl Abcd {
    pub fn gap(t: f64) -> Abcd {
        Abcd {
            a: 1.0,
            b: t,
            c: 0.0,
            d: 1.0,
        }
    }
    pub fn lens(f: f64) -> Abcd {
        Abcd {
            a: 1.0,
            b: 0.0,
            c: -1.0 / f,
            d: 1.0,
        }
    }
    pub fn apply(self, y: f64, u: f64) -> (f64, f64) {
        (self.a * y + self.b * u, self.c * y + self.d * u)
    }
    pub fn det(self) -> f64 {
        self.a * self.d - self.b * self.c
    }
}

/// Analysis options that are not part of the project's `OpticsParams`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Extras {
    /// Eye pupil diameter, mm (≈ 2 mm in sunlight, 4 mm indoors, 7 mm dark).
    pub pupil_mm: f64,
    /// Transmission of the collimating lens (two uncoated surfaces ≈ 0.92).
    pub lens_transmission: f64,
    /// Background luminances for the contrast table, nits (cd/m²).
    pub backgrounds: Vec<(String, f64)>,
    /// Distance of the world the wearer is looking at, m (for the focus mismatch).
    pub world_m: f64,
}

impl Default for Extras {
    fn default() -> Self {
        Extras {
            pupil_mm: 4.0,
            lens_transmission: 0.92,
            backgrounds: vec![
                ("Dim room".into(), 30.0),
                ("Office / indoor".into(), 100.0),
                ("Overcast street".into(), 2000.0),
                ("Sunny street".into(), 5000.0),
                ("Sunlit snow / concrete".into(), 12000.0),
            ],
            world_m: 10.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContrastRow {
    pub name: String,
    pub background_nits: f64,
    /// Background as seen through the combiner.
    pub see_through_nits: f64,
    pub contrast_ratio: f64,
    /// "good" (≥ 3:1), "marginal" (1.5–3) or "washed out" (< 1.5).
    pub verdict: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Analysis {
    pub f_mm: f64,
    pub d_mm: f64,
    pub aperture_mm: f64,
    /// Lens → eye along the (unfolded) path.
    pub path_mm: f64,
    /// Signed image distance from the thin-lens equation (negative = virtual).
    pub s_i_mm: f64,
    /// "virtual", "collimated" or "real".
    pub image_kind: String,
    /// Virtual image distance behind the lens (|s_i|), mm (1e12 when collimated).
    pub image_behind_lens_mm: f64,
    /// Virtual image distance from the eye, m.
    pub virtual_image_m: f64,
    pub magnification: f64,
    pub image_width_mm: f64,
    pub image_height_mm: f64,
    pub hfov_deg: f64,
    pub vfov_deg: f64,
    pub dfov_deg: f64,
    /// 2·atan(w/2f): the collimated-limit FOV, for comparison.
    pub hfov_collimated_deg: f64,
    pub ppd: f64,
    pub ppd_center: f64,
    pub ppd_v: f64,
    pub arcmin_per_px: f64,
    pub pixel_pitch_um: f64,
    pub f_number: f64,
    /// Numerical aperture of the cone accepted from an on-axis display point.
    pub na: f64,
    pub eyebox_mm: f64,
    pub eyebox_v_mm: f64,
    pub eyebox_approx_mm: f64,
    pub eyebox_full_pupil_mm: f64,
    pub eyebox_partial_mm: f64,
    pub pupil_mm: f64,
    /// k = 1 + L/d − L/f: beam footprint at the eye = D·|k|.
    pub k: f64,
    /// Smallest combiner (along its 45° face) that catches every ray, mm.
    pub combiner_min_mm: f64,
    /// Smallest lens-to-combiner distance at which the 45° combiner clears the lens, mm.
    pub min_lens_to_combiner_mm: f64,
    /// Display → eye ray-transfer matrix.
    pub abcd: Abcd,
    /// Accommodation demand, dioptres (1 / virtual image distance in m).
    pub accommodation_d: f64,
    /// Focus mismatch between the image and the world at `world_m`, dioptres.
    pub focus_mismatch_d: f64,
    /// Change of image vergence per millimetre of display shift, dioptres/mm (d(1/s_i)/dd = 1/d² → 1000/d²).
    pub focus_sensitivity_d_per_mm: f64,
    pub see_through_pct: f64,
    pub image_nits: f64,
    pub lens_transmission: f64,
    pub contrast: Vec<ContrastRow>,
    pub warnings: Vec<String>,
}

/// Clearance between the lens plane and the combiner's upper edge, mm.
pub const COMBINER_CLEARANCE_MM: f64 = 1.0;

/// Distance treated as "at infinity" (collimated), mm.
pub const INF_MM: f64 = 1e12;

pub fn contrast_ratio(image_nits: f64, background_nits: f64, see_through: f64) -> f64 {
    let bg = (background_nits * see_through).max(1e-9);
    (image_nits + bg) / bg
}

pub fn verdict(cr: f64) -> &'static str {
    if cr >= 3.0 {
        "good"
    } else if cr >= 1.5 {
        "marginal"
    } else {
        "washed out"
    }
}

/// Signed thin-lens image distance (real positive); INF_MM for d = f.
pub fn image_distance(f: f64, d: f64) -> f64 {
    if (d - f).abs() < 1e-9 {
        -INF_MM
    } else {
        f * d / (d - f)
    }
}

/// Angle (rad) at which the eye centre sees display height `y0`: the ray
/// through the lens that lands on the eye centre (`y_e = 0`).
pub fn field_angle(f: f64, d: f64, l: f64, y0: f64) -> f64 {
    let k = 1.0 + l / d - l / f;
    let yl = if k.abs() > 1e-12 { y0 * l / (d * k) } else { 0.0 };
    let u = (yl - y0) / d - yl / f;
    (-u).atan()
}

pub fn analyze(o: &OpticsParams, x: &Extras) -> Analysis {
    let f = o.lens_focal_mm.max(0.1);
    let d = o.display_to_lens_mm.max(0.01);
    let dap = o.lens_diameter_mm.max(0.01);
    let l = (o.lens_to_combiner_mm + o.eye_relief_mm).max(0.0);
    let (w, h) = (o.display_width_mm.max(1e-6), o.display_height_mm.max(1e-6));
    let (pxw, pxh) = (o.px_w.max(1) as f64, o.px_h.max(1) as f64);
    let mut warn = Vec::new();
    let si = image_distance(f, d);
    let kind = if si <= -INF_MM / 2.0 {
        "collimated"
    } else if si < 0.0 {
        "virtual"
    } else {
        "real"
    };
    let behind = if kind == "real" { 0.0 } else { -si };
    let m = if kind == "collimated" { INF_MM } else { -si / d };
    let vi_m = match kind {
        "virtual" => (behind + l) / 1000.0,
        "collimated" => INF_MM / 1000.0,
        _ => 0.0,
    };
    if kind == "real" {
        warn.push(format!(
            "The display is {:.2} mm beyond the focal length: the lens forms a real image {:.0} mm behind it, not a heads-up virtual image. Move the display inside f.",
            d - f,
            si
        ));
    } else if kind == "collimated" {
        warn.push("Display exactly at the focal plane: the image is at infinity (collimated).".into());
    }
    let ang = |y: f64| field_angle(f, d, l, y);
    let hf = 2.0 * ang(w / 2.0).abs();
    let vf = 2.0 * ang(h / 2.0).abs();
    let df = 2.0 * ang((w * w + h * h).sqrt() / 2.0).abs();
    let hfov = hf.to_degrees();
    let vfov = vf.to_degrees();
    // angular size of one pixel at the centre
    let pitch = w / pxw;
    let ppd_center = 1.0 / (ang(pitch / 2.0) - ang(-pitch / 2.0)).abs().to_degrees();
    let k = 1.0 + l / d - l / f;
    let eyebox = dap * k.abs() - w * l / d;
    let eyebox_v = dap * k.abs() - h * l / d;
    let approx = dap - 2.0 * l * (hf / 2.0).tan();
    let pupil = x.pupil_mm.max(0.0);
    if eyebox <= 0.0 {
        warn.push(
            "The lens aperture is too small to show the whole field to any eye position (eyebox ≤ 0): the edges are vignetted.".into(),
        );
    }
    // combiner footprint: rays from the field edges through the aperture edges, hitting the 45° mirror
    let lc = o.lens_to_combiner_mm.max(0.0);
    let mut smin = f64::INFINITY;
    let mut smax = f64::NEG_INFINITY;
    // the upper end of the 45° mirror must clear the lens: every ray must meet
    // the mirror at x ≥ COMBINER_CLEARANCE_MM, i.e. L_c ≥ y_l + (1 + u)·clearance
    let mut lc_min: f64 = 0.0;
    for &y0 in &[-w / 2.0, w / 2.0] {
        for &yl in &[-dap / 2.0, dap / 2.0] {
            let u = (yl - y0) / d - yl / f;
            lc_min = lc_min.max(yl + (1.0 + u) * COMBINER_CLEARANCE_MM);
            if 1.0 + u > 1e-6 {
                let t = (lc - yl) / (1.0 + u);
                let s = t - lc; // position along the mirror in x
                smin = smin.min(s);
                smax = smax.max(s);
            }
        }
    }
    let comb = if smax > smin {
        (smax - smin) * std::f64::consts::SQRT_2
    } else {
        0.0
    };
    if lc < lc_min - 1e-9 {
        warn.push(format!(
            "The 45° combiner would cut through the lens: with a {dap:.1} mm aperture the lens-to-combiner distance must be at least {lc_min:.1} mm (now {lc:.1} mm). Rays drawn behind the lens are not physical."
        ));
    }
    let abcd = Abcd::gap(l) * Abcd::lens(f) * Abcd::gap(d);
    let r = o.combiner_reflectance.clamp(0.0, 1.0);
    let t_see = 1.0 - r;
    let lt = x.lens_transmission.clamp(0.0, 1.0);
    let img = o.display_luminance_nits.max(0.0) * lt * r;
    let contrast = x
        .backgrounds
        .iter()
        .map(|(n, b)| {
            let cr = contrast_ratio(img, *b, t_see);
            ContrastRow {
                name: n.clone(),
                background_nits: *b,
                see_through_nits: b * t_see,
                contrast_ratio: cr,
                verdict: verdict(cr).into(),
            }
        })
        .collect();
    let acc = if kind == "virtual" { 1.0 / vi_m } else { 0.0 };
    let world = if x.world_m > 0.0 { 1.0 / x.world_m } else { 0.0 };
    Analysis {
        f_mm: f,
        d_mm: d,
        aperture_mm: dap,
        path_mm: l,
        s_i_mm: si,
        image_kind: kind.into(),
        image_behind_lens_mm: if kind == "collimated" { INF_MM } else { behind },
        virtual_image_m: vi_m,
        magnification: m,
        image_width_mm: if kind == "virtual" { m * w } else { 0.0 },
        image_height_mm: if kind == "virtual" { m * h } else { 0.0 },
        hfov_deg: hfov,
        vfov_deg: vfov,
        dfov_deg: df.to_degrees(),
        hfov_collimated_deg: 2.0 * (w / (2.0 * f)).atan().to_degrees(),
        ppd: if hfov > 0.0 { pxw / hfov } else { 0.0 },
        ppd_center,
        ppd_v: if vfov > 0.0 { pxh / vfov } else { 0.0 },
        arcmin_per_px: if ppd_center > 0.0 { 60.0 / ppd_center } else { 0.0 },
        pixel_pitch_um: pitch * 1000.0,
        f_number: f / dap,
        na: (dap / 2.0 / (d * d + dap * dap / 4.0).sqrt()).min(1.0),
        eyebox_mm: eyebox,
        eyebox_v_mm: eyebox_v,
        eyebox_approx_mm: approx,
        eyebox_full_pupil_mm: eyebox - pupil,
        eyebox_partial_mm: eyebox + pupil,
        pupil_mm: pupil,
        k,
        combiner_min_mm: comb,
        min_lens_to_combiner_mm: lc_min,
        abcd,
        accommodation_d: acc,
        focus_mismatch_d: (acc - world).abs(),
        focus_sensitivity_d_per_mm: 1000.0 / (d * d),
        see_through_pct: t_see * 100.0,
        image_nits: img,
        lens_transmission: lt,
        contrast,
        warnings: warn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> OpticsParams {
        OpticsParams {
            lens_diameter_mm: 18.0,
            ..OpticsParams::default()
        }
    }

    #[test]
    fn thin_lens_equation_cases() {
        // 2f–2f: real, inverted, unit magnification
        assert!((image_distance(10.0, 20.0) - 20.0).abs() < 1e-12);
        // object inside f: virtual
        let si = image_distance(20.0, 19.7);
        assert!((si - 20.0 * 19.7 / (19.7 - 20.0)).abs() < 1e-9);
        assert!(si < -1300.0 && si > -1320.0);
        // 1/so + 1/si = 1/f
        assert!((1.0 / 19.7 + 1.0 / si - 1.0 / 20.0).abs() < 1e-12);
        // at f: infinity
        assert!(image_distance(20.0, 20.0) <= -INF_MM);
    }

    #[test]
    fn iris_defaults() {
        let a = analyze(&params(), &Extras::default());
        assert_eq!(a.image_kind, "virtual");
        assert!(
            (a.virtual_image_m - (1313.33 + 29.0) / 1000.0).abs() < 1e-3,
            "{}",
            a.virtual_image_m
        );
        assert!(a.hfov_deg > 23.0 && a.hfov_deg < 24.0, "{}", a.hfov_deg);
        assert!(a.ppd > 26.5 && a.ppd < 28.0, "{}", a.ppd);
        assert!(a.eyebox_mm > 5.5 && a.eyebox_mm < 6.5, "{}", a.eyebox_mm);
        // exact eyebox is slightly larger than the collimated approximation by (s+L)/s
        assert!(a.eyebox_mm > a.eyebox_approx_mm);
        assert!((a.see_through_pct - 70.0).abs() < 1e-9);
        assert!((a.image_nits - 3000.0 * 0.92 * 0.3).abs() < 1e-9);
        // FOV from the virtual image geometry
        let hw = a.image_width_mm / 2.0;
        let fov = 2.0 * (hw / (a.image_behind_lens_mm + 29.0)).atan().to_degrees();
        assert!((fov - a.hfov_deg).abs() < 1e-9);
        // diagonal > horizontal > vertical
        assert!(a.dfov_deg > a.hfov_deg && a.hfov_deg > a.vfov_deg);
        // outdoors the image washes out
        let sunny = a.contrast.iter().find(|c| c.name.starts_with("Sunny")).unwrap();
        assert!(sunny.contrast_ratio < 1.5);
        let office = a.contrast.iter().find(|c| c.name.starts_with("Office")).unwrap();
        assert!(office.contrast_ratio > 3.0);
    }

    #[test]
    fn collimated_limit_fov() {
        let o = OpticsParams {
            display_to_lens_mm: 20.0 - 1e-7,
            ..params()
        };
        let a = analyze(&o, &Extras::default());
        let expect = 2.0 * (8.4f64 / 40.0).atan().to_degrees();
        assert!((a.hfov_deg - expect).abs() < 1e-4, "{} vs {expect}", a.hfov_deg);
        assert!((a.hfov_collimated_deg - expect).abs() < 1e-12);
        // eyebox → D − 2 L tan(FOV/2)
        let eb = 18.0 - 2.0 * 29.0 * (expect.to_radians() / 2.0).tan();
        assert!((a.eyebox_mm - eb).abs() < 1e-3, "{} vs {eb}", a.eyebox_mm);
        let exact = OpticsParams {
            display_to_lens_mm: 20.0,
            ..params()
        };
        let b = analyze(&exact, &Extras::default());
        assert_eq!(b.image_kind, "collimated");
        assert!((b.hfov_deg - expect).abs() < 1e-9);
    }

    #[test]
    fn abcd_ray_matches_thin_lens_image() {
        let (f, d, y0) = (20.0, 19.7, 3.0);
        let si = image_distance(f, d);
        let m_expect = -si / d;
        // two rays from the same display point, traced through lens with ABCD
        let sys = Abcd::lens(f) * Abcd::gap(d);
        assert!((sys.det() - 1.0).abs() < 1e-12);
        let (y1, u1) = sys.apply(y0, 0.0);
        let (y2, u2) = sys.apply(y0, 0.2);
        // back-extrapolate: y1 + u1 z = y2 + u2 z
        let z = (y2 - y1) / (u1 - u2);
        assert!((z - si).abs() < 1e-6, "{z} vs {si}");
        let yi = y1 + u1 * z;
        assert!((yi - m_expect * y0).abs() < 1e-6);
        // real imaging case: B element of T(si)·Λ·T(so) vanishes
        let img = Abcd::gap(20.0) * Abcd::lens(10.0) * Abcd::gap(20.0);
        assert!(img.b.abs() < 1e-12);
        assert!((img.a + 1.0).abs() < 1e-12);
    }

    #[test]
    fn eyebox_formula_and_pupil_variants() {
        let a = analyze(&params(), &Extras::default());
        let si = image_distance(20.0, 19.7).abs();
        let l = 29.0;
        let tan = (a.hfov_deg.to_radians() / 2.0).tan();
        let exact = (si + l) / si * (18.0 - 2.0 * l * tan);
        assert!((a.eyebox_mm - exact).abs() < 1e-6, "{} vs {exact}", a.eyebox_mm);
        assert!((a.eyebox_full_pupil_mm - (exact - 4.0)).abs() < 1e-6);
        assert!((a.eyebox_partial_mm - (exact + 4.0)).abs() < 1e-6);
        // an 18 mm aperture needs ≥ ~10.2 mm lens-to-combiner (Iris v0.3 had 8 mm; v0.4 uses 11 mm)
        assert!(
            a.min_lens_to_combiner_mm > 10.0 && a.min_lens_to_combiner_mm < 10.5,
            "{}",
            a.min_lens_to_combiner_mm
        );
        assert!(!a.warnings.iter().any(|w| w.contains("cut through the lens")));
        // the v0.3 layout (8 mm) collides with the lens
        let v03 = analyze(
            &OpticsParams {
                lens_to_combiner_mm: 8.0,
                ..params()
            },
            &Extras::default(),
        );
        assert!(v03.warnings.iter().any(|w| w.contains("cut through the lens")));
        // a 10 mm aperture would give ≈ 0 eyebox
        let small = analyze(
            &OpticsParams {
                lens_diameter_mm: 10.0,
                ..params()
            },
            &Extras::default(),
        );
        assert!(small.eyebox_mm < 0.5, "{}", small.eyebox_mm);
    }
}
