//! Display legibility and eyebox coverage for a see-through head-worn display.
//!
//! * **Angular size.** A glyph `h` pixels tall subtends `h · (arcmin per
//!   pixel)`; the optics lab supplies arcmin per pixel at the field centre.
//!   A 20/20 Snellen letter is 5 arcmin tall (1 arcmin strokes); someone with
//!   20/x vision needs letters x/20 times larger to read them at all. Reading
//!   comfortably needs much more than threshold: display ergonomics guidance
//!   (ANSI/HFES 100-2007) gives ≈ 16 arcmin minimum and 20–22 arcmin preferred
//!   character height (Sanders & McCormick 1993 give similar figures).
//! * **Ambient contrast ratio (ACR).** An optical see-through display adds its
//!   light to the world: `ACR = (L_img + T L_bg) / (T L_bg)` with
//!   `L_img = L_display · T_lens · R_combiner` and see-through transmission
//!   `T = (1 − R_combiner) · T_tint` (Kress & Starner 2013; Kress 2020). About
//!   3 : 1 is the usual floor for text on displays; below ≈ 1.5 : 1 text
//!   washes out.
//! * **Eyebox coverage.** A monocular display on the right lens projects its
//!   exit pupil around the display axis at `x_d = (A + DBL)/2 − offset` from
//!   the face mid-line. A wearer sees the whole image while the pupil centre,
//!   at `IPD/2` (plus any face asymmetry), stays within ±eyebox/2 of `x_d`:
//!
//! ```text
//! coverage = Σ_g w_g [Φ((2(x_d + e/2) − μ_g)/σ_g) − Φ((2(x_d − e/2) − μ_g)/σ_g)]
//! ```
//!
//!   summed over women and men with their population weights `w_g`.

use serde::{Deserialize, Serialize};

use super::anthro::{dims, Group, IPD};
use super::model::{ErgoConfig, FrameGeom};
use super::stats::Mixture;
use crate::optics::paraxial::{analyze, contrast_ratio, Extras};
use crate::project::OpticsParams;

/// Scenes for the contrast table: name, background luminance (cd/m²), description.
pub const SCENES: [(&str, f64, &str); 5] = [
    ("Dim room", 30.0, "evening indoors, a lamp on"),
    ("Office / indoor", 100.0, "typical lit interior surfaces"),
    ("Overcast street", 2000.0, "outdoors under cloud"),
    ("Sunlit street", 5000.0, "outdoors in direct sun"),
    ("Snow / white concrete in sun", 12000.0, "bright, high-albedo surfaces"),
];

pub fn legibility_verdict(arcmin: f64, threshold: f64) -> &'static str {
    if arcmin < threshold {
        "below acuity"
    } else if arcmin < 2.0 * threshold.max(5.0) {
        "threshold"
    } else if arcmin < 16.0 {
        "small"
    } else if arcmin <= 40.0 {
        "comfortable"
    } else {
        "large"
    }
}

pub fn contrast_verdict(cr: f64) -> &'static str {
    if cr >= 3.0 {
        "good"
    } else if cr >= 1.5 {
        "marginal"
    } else {
        "washed out"
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneRow {
    pub name: String,
    pub description: String,
    pub background_nits: f64,
    pub see_through_nits: f64,
    pub image_nits: f64,
    pub contrast_ratio: f64,
    pub verdict: String,
    /// Display luminance needed for 3 : 1 at this background, nits.
    pub nits_for_3to1: f64,
}

/// Photometry of the Iris display with an optional tint.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Photometry {
    pub image_nits: f64,
    pub see_through: f64,
    pub lens_transmission: f64,
    pub combiner_reflectance: f64,
    pub tint: f64,
}

pub fn photometry(o: &OpticsParams, tint: f64) -> Photometry {
    let x = Extras::default();
    let r = o.combiner_reflectance.clamp(0.0, 1.0);
    let lt = x.lens_transmission.clamp(0.0, 1.0);
    Photometry {
        image_nits: o.display_luminance_nits.max(0.0) * lt * r,
        see_through: (1.0 - r) * tint,
        lens_transmission: lt,
        combiner_reflectance: r,
        tint,
    }
}

pub fn scene_row(ph: &Photometry, name: &str, desc: &str, bg: f64) -> SceneRow {
    let cr = contrast_ratio(ph.image_nits, bg, ph.see_through);
    // (L_img + T L)/(T L) = 3  →  L_img = 2 T L  →  L_disp = 2 T L / (T_lens R)
    let per_nit = ph.lens_transmission * ph.combiner_reflectance;
    let need = if per_nit > 0.0 {
        2.0 * ph.see_through * bg / per_nit
    } else {
        f64::INFINITY
    };
    SceneRow {
        name: name.into(),
        description: desc.into(),
        background_nits: bg,
        see_through_nits: bg * ph.see_through,
        image_nits: ph.image_nits,
        contrast_ratio: cr,
        verdict: contrast_verdict(cr).into(),
        nits_for_3to1: need,
    }
}

/// IPD window [lo, hi] (mm) whose pupils land inside the eyebox.
pub fn eyebox_window(display_x: f64, eyebox: f64) -> [f64; 2] {
    [2.0 * (display_x - eyebox / 2.0), 2.0 * (display_x + eyebox / 2.0)]
}

/// Analytic eyebox coverage (fraction 0..1) of a population group.
pub fn eyebox_coverage(ipd: &Mixture, display_x: f64, eyebox: f64) -> f64 {
    if eyebox <= 0.0 {
        return 0.0;
    }
    let [lo, hi] = eyebox_window(display_x, eyebox);
    ipd.prob_between(lo, hi)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EyeboxGroup {
    pub group: Group,
    pub coverage_pct: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EyeboxOut {
    pub eyebox_mm: f64,
    pub eyebox_full_pupil_mm: f64,
    pub display_x_mm: f64,
    pub ipd_window_mm: [f64; 2],
    pub groups: Vec<EyeboxGroup>,
    /// Display offset (towards the nose) that maximises combined coverage.
    pub best_offset_mm: f64,
    pub best_coverage_pct: f64,
    /// Sweep for plotting: offsets and combined coverage.
    pub offsets: Vec<f64>,
    pub coverage_vs_offset: Vec<f64>,
    /// Eyebox width needed for 95 % combined coverage at the best offset.
    pub eyebox_for_95_mm: f64,
    /// IPD density (combined) for the window plot.
    pub ipd_x: Vec<f64>,
    pub ipd_pdf: Vec<f64>,
    /// IPD statistics used, [[men mean, sd], [women mean, sd]] (asymmetry included).
    pub ipd_stats: [[f64; 2]; 2],
}

pub fn eyebox(fg: &FrameGeom, o: &OpticsParams, cfg: &ErgoConfig) -> EyeboxOut {
    let a = analyze(o, &Extras::default());
    let eb = a.eyebox_mm.max(0.0);
    let d = dims(cfg);
    let mix = |g| {
        let mut m = d[IPD].group(g, cfg.female_fraction);
        // face asymmetry widens the monocular-PD spread: σ² → σ² + (2σ_a)²
        if cfg.asymmetry_sd_mm > 0.0 {
            for (_, n) in m.0.iter_mut() {
                n.sd = (n.sd * n.sd + (2.0 * cfg.asymmetry_sd_mm).powi(2)).sqrt();
            }
        }
        m
    };
    let comb = mix(Group::Combined);
    let dx = fg.lens_center_x - cfg.display_offset_mm;
    let groups = [Group::Men, Group::Women, Group::Combined]
        .iter()
        .map(|&g| EyeboxGroup {
            group: g,
            coverage_pct: 100.0 * eyebox_coverage(&mix(g), dx, eb),
        })
        .collect();
    let offsets = crate::math::linspace(-6.0, 10.0, 161);
    let cov: Vec<f64> = offsets
        .iter()
        .map(|off| 100.0 * eyebox_coverage(&comb, fg.lens_center_x - off, eb))
        .collect();
    // the optimum centres the window on the combined median monocular PD
    let best_x = comb.quantile(0.5) / 2.0;
    let best_offset = fg.lens_center_x - best_x;
    let best = 100.0 * eyebox_coverage(&comb, best_x, eb);
    // eyebox for 95 %: bisection on width
    let (mut lo, mut hi) = (0.0, 60.0);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if eyebox_coverage(&comb, best_x, mid) < 0.95 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let (mm, mw) = (mix(Group::Men).0[0].1, mix(Group::Women).0[0].1);
    let ipd_x = crate::math::linspace(48.0, 80.0, 161);
    let ipd_pdf = ipd_x.iter().map(|&x| comb.pdf(x)).collect();
    EyeboxOut {
        eyebox_mm: eb,
        eyebox_full_pupil_mm: a.eyebox_full_pupil_mm,
        display_x_mm: dx,
        ipd_window_mm: eyebox_window(dx, eb),
        groups,
        best_offset_mm: best_offset,
        best_coverage_pct: best,
        offsets,
        coverage_vs_offset: cov,
        eyebox_for_95_mm: hi,
        ipd_x,
        ipd_pdf,
        ipd_stats: [[mm.mean, mm.sd], [mw.mean, mw.sd]],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ergonomics::stats::Normal;

    #[test]
    fn contrast_ratio_by_hand() {
        // 3000 nits × 0.92 × 0.3 = 828 nits; T = 0.7; 5000 nits background:
        // ACR = (828 + 3500)/3500 = 1.2366
        let o = OpticsParams::default();
        let ph = photometry(&o, 1.0);
        assert!((ph.image_nits - 828.0).abs() < 1e-9);
        let r = scene_row(&ph, "sun", "", 5000.0);
        assert!((r.contrast_ratio - (828.0 + 3500.0) / 3500.0).abs() < 1e-12);
        assert_eq!(r.verdict, "washed out");
        // a 30 % tint: T = 0.21 → (828 + 1050)/1050 = 1.789
        let ph2 = photometry(&o, 0.3);
        let r2 = scene_row(&ph2, "sun", "", 5000.0);
        assert!((r2.contrast_ratio - (828.0 + 1050.0) / 1050.0).abs() < 1e-9);
        // nits for 3:1 at 5000 nits, clear: 2·0.7·5000/(0.92·0.3) = 25 362 nits
        assert!((r.nits_for_3to1 - 2.0 * 0.7 * 5000.0 / 0.276).abs() < 1e-6);
    }

    #[test]
    fn eyebox_coverage_by_hand() {
        // One normal IPD N(64, 3.4²); display axis at 34 mm; 6 mm eyebox →
        // window 62..74 mm → Φ((74−64)/3.4) − Φ((62−64)/3.4) = Φ(2.941) − Φ(−0.588)
        let m = Mixture(vec![(1.0, Normal::new(64.0, 3.4))]);
        let c = eyebox_coverage(&m, 34.0, 6.0);
        let expect = 0.998_365 - 0.278_2;
        assert!((c - expect).abs() < 2e-3, "{c} vs {expect}");
        assert_eq!(eyebox_window(34.0, 6.0), [62.0, 74.0]);
        // centred on the mean, ±3 mm eyebox → IPD ±6 mm = ±1.765σ → 92.2 %
        let c2 = eyebox_coverage(&m, 32.0, 6.0);
        assert!((c2 - 0.9224).abs() < 2e-3, "{c2}");
    }
}
