//! Design-space sweeps and text legibility.
//!
//! **FOV vs resolution.** For a fixed micro-display, `PPD · FOV ≈ pixels`:
//! a shorter focal length widens the field of view (FOV) but spreads the
//! same pixels over more degrees, lowering pixels per degree (PPD). With
//! requirements on both, the focal length is squeezed into a band
//! `f_min ≤ f ≤ f_max` — the classic head-mounted display trade (Kress
//! 2020, ch. 3; Rolland & Hua 2005).
//!
//! **Legibility.** Letter size is judged by the angle it subtends. A Snellen
//! 20/20 optotype is 5 arcminutes tall (1 arcmin per stroke). Reading is
//! comfortable well above threshold — display-ergonomics guidance such as
//! ANSI/HFES 100-2007 asks for roughly 16 arcmin minimum and 20–22 arcmin
//! preferred character height.

use serde::{Deserialize, Serialize};

use super::paraxial::{analyze, contrast_ratio, Extras};
use crate::project::{OpticsParams, Project};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SweepConfig {
    pub f_min_mm: f64,
    pub f_max_mm: f64,
    pub points: usize,
    /// Requirement thresholds (None = read from the project's requirements).
    pub min_hfov_deg: Option<f64>,
    pub min_ppd: Option<f64>,
    pub min_eyebox_mm: Option<f64>,
    pub extras: Extras,
}

impl Default for SweepConfig {
    fn default() -> Self {
        SweepConfig {
            f_min_mm: 8.0,
            f_max_mm: 40.0,
            points: 129,
            min_hfov_deg: None,
            min_ppd: None,
            min_eyebox_mm: None,
            extras: Extras::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DesignSweep {
    pub f_mm: Vec<f64>,
    pub hfov_deg: Vec<f64>,
    pub ppd: Vec<f64>,
    /// Eyebox with the lens diameter held fixed.
    pub eyebox_fixed_d_mm: Vec<f64>,
    /// Eyebox with the f-number held fixed (lens grows with f).
    pub eyebox_fixed_n_mm: Vec<f64>,
    /// Pixels needed across to reach the PPD requirement at each FOV.
    pub px_needed: Vec<f64>,
    pub min_hfov_deg: f64,
    pub min_ppd: f64,
    pub min_eyebox_mm: f64,
    /// Focal-length band meeting FOV and PPD (None if empty).
    pub band_mm: Option<(f64, f64)>,
    /// Band also meeting the eyebox (fixed D).
    pub band_eyebox_mm: Option<(f64, f64)>,
    pub current_f_mm: f64,
    /// Virtual image distance kept constant while sweeping (display refocused), m.
    pub image_m: f64,
    pub note: String,
}

fn req_target(p: Option<&Project>, key: &str) -> Option<f64> {
    p?.requirements
        .iter()
        .find(|r| r.verification.metric.as_deref() == Some(key))
        .map(|r| r.verification.target)
}

/// Sweep the focal length, refocusing the display so the virtual image stays
/// at the current distance.
pub fn focal_sweep(o: &OpticsParams, p: Option<&Project>, cfg: &SweepConfig) -> DesignSweep {
    let base = analyze(o, &cfg.extras);
    let min_fov = cfg.min_hfov_deg.or_else(|| req_target(p, "optics.hfov_deg")).unwrap_or(20.0);
    let min_ppd = cfg.min_ppd.or_else(|| req_target(p, "optics.ppd")).unwrap_or(25.0);
    let min_eb = cfg.min_eyebox_mm.or_else(|| req_target(p, "optics.eyebox_mm")).unwrap_or(5.0);
    // keep the image this far behind the lens (fallback 1 m for real/collimated)
    let s = if base.image_kind == "virtual" {
        base.image_behind_lens_mm
    } else {
        1000.0
    };
    let n_number = base.f_number;
    let fs = crate::math::linspace(
        cfg.f_min_mm.max(1.0),
        cfg.f_max_mm.max(cfg.f_min_mm + 1.0),
        cfg.points.clamp(3, 2000),
    );
    let mut out = DesignSweep {
        min_hfov_deg: min_fov,
        min_ppd,
        min_eyebox_mm: min_eb,
        current_f_mm: base.f_mm,
        image_m: (s + base.path_mm) / 1000.0,
        note: "Display refocused at each f so the virtual image stays at the current distance; lens-to-eye path unchanged.".into(),
        ..Default::default()
    };
    for &f in &fs {
        // 1/d = 1/f + 1/s  (virtual image s behind the lens)
        let d = 1.0 / (1.0 / f + 1.0 / s);
        let q = OpticsParams {
            lens_focal_mm: f,
            display_to_lens_mm: d,
            ..o.clone()
        };
        let a = analyze(&q, &cfg.extras);
        let qn = OpticsParams {
            lens_diameter_mm: f / n_number,
            ..q.clone()
        };
        let an = analyze(&qn, &cfg.extras);
        out.hfov_deg.push(a.hfov_deg);
        out.ppd.push(a.ppd);
        out.eyebox_fixed_d_mm.push(a.eyebox_mm);
        out.eyebox_fixed_n_mm.push(an.eyebox_mm);
        out.px_needed.push(min_ppd * a.hfov_deg);
    }
    out.f_mm = fs;
    let band = |ok: &dyn Fn(usize) -> bool| -> Option<(f64, f64)> {
        let idx: Vec<usize> = (0..out.f_mm.len()).filter(|&i| ok(i)).collect();
        if idx.is_empty() {
            None
        } else {
            Some((out.f_mm[idx[0]], out.f_mm[*idx.last().unwrap()]))
        }
    };
    let b1 = band(&|i| out.hfov_deg[i] >= min_fov && out.ppd[i] >= min_ppd);
    let b2 = band(&|i| out.hfov_deg[i] >= min_fov && out.ppd[i] >= min_ppd && out.eyebox_fixed_d_mm[i] >= min_eb);
    out.band_mm = b1;
    out.band_eyebox_mm = b2;
    out
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LegibilityConfig {
    /// Font sizes (em height in display pixels) to tabulate.
    pub sizes_px: Vec<f64>,
    /// Cap height as a fraction of the em.
    pub cap_ratio: f64,
    /// Character advance as a fraction of the em (monospace ≈ 0.6).
    pub advance_ratio: f64,
    /// Line pitch as a fraction of the em.
    pub line_ratio: f64,
    pub extras: Extras,
}

impl Default for LegibilityConfig {
    fn default() -> Self {
        LegibilityConfig {
            sizes_px: vec![8.0, 10.0, 12.0, 14.0, 16.0, 20.0, 24.0, 28.0, 32.0, 40.0, 48.0, 64.0],
            cap_ratio: 0.7,
            advance_ratio: 0.6,
            line_ratio: 1.25,
            extras: Extras::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LegibilityRow {
    pub em_px: f64,
    pub cap_px: f64,
    pub cap_arcmin: f64,
    /// Snellen denominator a reader needs to just resolve the letters (20/x).
    pub snellen_20_over: f64,
    pub chars_per_line: u32,
    pub lines: u32,
    /// "below acuity", "threshold", "small", "comfortable", "large".
    pub verdict: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Legibility {
    pub arcmin_per_px: f64,
    pub ppd_center: f64,
    pub rows: Vec<LegibilityRow>,
    /// Smallest em (px) giving ≥ 16 / ≥ 20 arcmin cap height.
    pub min_em_16: f64,
    pub min_em_20: f64,
    pub px_w: u32,
    pub px_h: u32,
    pub hfov_deg: f64,
    pub vfov_deg: f64,
}

pub fn legibility_verdict(arcmin: f64) -> &'static str {
    if arcmin < 5.0 {
        "below acuity"
    } else if arcmin < 10.0 {
        "threshold"
    } else if arcmin < 16.0 {
        "small"
    } else if arcmin <= 40.0 {
        "comfortable"
    } else {
        "large"
    }
}

pub fn legibility(o: &OpticsParams, cfg: &LegibilityConfig) -> Legibility {
    let a = analyze(o, &cfg.extras);
    let apx = a.arcmin_per_px;
    let row = |em: f64| {
        let cap = em * cfg.cap_ratio;
        let am = cap * apx;
        let adv = (em * cfg.advance_ratio).max(1.0);
        let lh = (em * cfg.line_ratio).max(1.0);
        LegibilityRow {
            em_px: em,
            cap_px: cap,
            cap_arcmin: am,
            snellen_20_over: 20.0 * am / 5.0,
            chars_per_line: (o.px_w as f64 / adv).floor() as u32,
            lines: (o.px_h as f64 / lh).floor() as u32,
            verdict: legibility_verdict(am).into(),
        }
    };
    let min_em = |target: f64| if apx > 0.0 { (target / (apx * cfg.cap_ratio)).ceil() } else { 0.0 };
    Legibility {
        arcmin_per_px: apx,
        ppd_center: a.ppd_center,
        rows: cfg.sizes_px.iter().map(|&s| row(s)).collect(),
        min_em_16: min_em(16.0),
        min_em_20: min_em(20.0),
        px_w: o.px_w,
        px_h: o.px_h,
        hfov_deg: a.hfov_deg,
        vfov_deg: a.vfov_deg,
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContrastSweep {
    pub background_nits: Vec<f64>,
    pub contrast_ratio: Vec<f64>,
    /// Display luminance needed for 3:1 at each background.
    pub nits_for_3: Vec<f64>,
    pub image_nits: f64,
    pub see_through: f64,
    /// Background at which contrast drops to 3:1 and 1.5:1.
    pub bg_at_3: f64,
    pub bg_at_1_5: f64,
}

/// Contrast ratio against a logarithmic sweep of background luminance.
pub fn contrast_sweep(o: &OpticsParams, x: &Extras) -> ContrastSweep {
    let a = analyze(o, x);
    let t = a.see_through_pct / 100.0;
    let bgs = crate::math::logspace(1.0, 31_622.8, 91);
    let gain = (o.display_luminance_nits * a.lens_transmission * o.combiner_reflectance.clamp(0.0, 1.0)).max(0.0);
    ContrastSweep {
        contrast_ratio: bgs.iter().map(|&b| contrast_ratio(a.image_nits, b, t)).collect(),
        nits_for_3: bgs
            .iter()
            .map(|&b| {
                let per = if o.display_luminance_nits > 0.0 {
                    gain / o.display_luminance_nits
                } else {
                    0.0
                };
                if per > 0.0 {
                    2.0 * t * b / per
                } else {
                    f64::MAX.min(1e12)
                }
            })
            .collect(),
        background_nits: bgs,
        image_nits: a.image_nits,
        see_through: t,
        bg_at_3: if t > 0.0 { a.image_nits / (2.0 * t) } else { 0.0 },
        bg_at_1_5: if t > 0.0 { a.image_nits / (0.5 * t) } else { 0.0 },
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
    fn fov_resolution_band() {
        let p = Project::iris();
        let s = focal_sweep(&iris(), Some(&p), &SweepConfig::default());
        // FOV falls and PPD rises with f; their product is ≈ the pixel count
        assert!(s.hfov_deg.windows(2).all(|w| w[1] < w[0]));
        assert!(s.ppd.windows(2).all(|w| w[1] > w[0]));
        for i in 0..s.f_mm.len() {
            assert!((s.ppd[i] * s.hfov_deg[i] - 640.0).abs() < 1e-6);
        }
        // collimated-approximation band: FOV ≥ 20 → f ≤ 4.2/tan 10° ≈ 23.8; PPD ≥ 25 → f ≥ 4.2/tan 12.8° ≈ 18.5
        let (lo, hi) = s.band_mm.unwrap();
        assert!(lo > 17.8 && lo < 19.0, "{lo}");
        assert!(hi > 23.3 && hi < 24.3, "{hi}");
        assert!(s.current_f_mm >= lo && s.current_f_mm <= hi);
    }

    #[test]
    fn legibility_numbers() {
        let l = legibility(&iris(), &LegibilityConfig::default());
        assert!(l.arcmin_per_px > 2.0 && l.arcmin_per_px < 2.4, "{}", l.arcmin_per_px);
        let r = l.rows.iter().find(|r| r.em_px == 16.0).unwrap();
        assert!((r.cap_arcmin - 16.0 * 0.7 * l.arcmin_per_px).abs() < 1e-9);
        assert_eq!(r.chars_per_line, (640.0f64 / 9.6).floor() as u32);
        // R-004 rationale: two lines of 20 characters at a comfortable size
        let comfy = l.rows.iter().find(|r| r.verdict == "comfortable").unwrap();
        assert!(comfy.chars_per_line >= 20 && comfy.lines >= 2);
        // Snellen: 5 arcmin ↔ 20/20
        assert_eq!(legibility_verdict(4.9), "below acuity");
    }

    #[test]
    fn contrast_falls_with_background() {
        let c = contrast_sweep(&iris(), &Extras::default());
        assert!(c.contrast_ratio.windows(2).all(|w| w[1] < w[0]));
        // CR = 3 at bg_at_3
        let cr = contrast_ratio(c.image_nits, c.bg_at_3, c.see_through);
        assert!((cr - 3.0).abs() < 1e-9);
    }
}
