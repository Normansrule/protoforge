//! # Optics engine (`optics.*`)
//!
//! First-order design of the Iris heads-up display: a micro-display, a thin
//! collimating lens and a 45° partially reflective combiner in front of the
//! eye (a "birdbath / beam-splitter" layout).
//!
//! | file | what |
//! |---|---|
//! | [`paraxial`] | thin-lens imaging, ray-transfer (ABCD) matrices, field of view (FOV), pixels per degree (PPD), eyebox, photometry and contrast |
//! | [`trace`] | 2D ray trace of the folded path into a reduced eye, vignetting vs eye position |
//! | [`design`] | FOV-vs-resolution focal-length sweep, text legibility, contrast vs background |
//!
//! Metrics: `optics.hfov_deg` (R-004), `optics.ppd` (R-005),
//! `optics.virtual_image_m` (R-006), `optics.eyebox_mm` (R-007), plus
//! `optics.vfov_deg`, `optics.dfov_deg`, `optics.image_nits`,
//! `optics.see_through_pct`. All closed-form (microseconds).
//!
//! Paraxial optics is exact only for small angles and a perfect thin lens;
//! real designs add aberrations (field curvature and distortion dominate at
//! ±12°), which need a real ray tracer (Zemax OpticStudio, or the
//! open-source `rayoptics`). References: E. Hecht, *Optics* (5th ed.,
//! Pearson, 2017); W. J. Smith, *Modern Optical Engineering* (4th ed.,
//! McGraw-Hill, 2008); B. C. Kress, *Optical Architectures for Augmented-,
//! Virtual-, and Mixed-Reality Headsets* (SPIE Press, 2020); J. P. Rolland &
//! H. Hua, "Head-mounted display systems", *Encyclopedia of Optical
//! Engineering* (2005).

pub mod design;
pub mod paraxial;
pub mod trace;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Metric, OpticsParams, Project};

pub use paraxial::{analyze, Analysis, Extras};

/// Input shared by the optics methods: explicit `optics` parameters win,
/// otherwise the project's `design.optics`, otherwise Iris.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct OptIn {
    project: Option<Project>,
    optics: Option<OpticsParams>,
    extras: Option<Extras>,
}

impl OptIn {
    fn params(&self) -> OpticsParams {
        self.optics
            .clone()
            .or_else(|| self.project.as_ref().map(|p| p.design.optics.clone()))
            .unwrap_or_else(|| Project::iris().design.optics)
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TraceIn {
    #[serde(flatten)]
    who: OptIn,
    config: trace::TraceConfig,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct SweepIn {
    #[serde(flatten)]
    who: OptIn,
    config: design::SweepConfig,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct LegIn {
    #[serde(flatten)]
    who: OptIn,
    config: design::LegibilityConfig,
}

#[derive(Serialize)]
struct AnalyzeOut {
    #[serde(flatten)]
    analysis: Analysis,
    optics: OpticsParams,
}

/// JSON API entry point for `optics.*` methods.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "optics.analyze" => handle(input, |i: OptIn| {
            let o = i.params();
            Ok(AnalyzeOut {
                analysis: analyze(&o, &i.extras.clone().unwrap_or_default()),
                optics: o,
            })
        }),
        "optics.trace" => handle(input, |i: TraceIn| {
            let mut cfg = i.config;
            if let Some(x) = i.who.extras.clone() {
                cfg.extras = x;
            }
            Ok(trace::trace(&i.who.params(), &cfg))
        }),
        "optics.design_sweep" => handle(input, |i: SweepIn| {
            let mut cfg = i.config;
            if let Some(x) = i.who.extras.clone() {
                cfg.extras = x;
            }
            Ok(design::focal_sweep(&i.who.params(), i.who.project.as_ref(), &cfg))
        }),
        "optics.legibility" => handle(input, |i: LegIn| {
            let mut cfg = i.config;
            if let Some(x) = i.who.extras.clone() {
                cfg.extras = x;
            }
            Ok(design::legibility(&i.who.params(), &cfg))
        }),
        "optics.contrast" => handle(input, |i: OptIn| {
            Ok(design::contrast_sweep(&i.params(), &i.extras.clone().unwrap_or_default()))
        }),
        _ => None,
    }
}

/// Metrics this module contributes to requirement verification.
pub fn metrics(p: &Project) -> Vec<Metric> {
    let o = &p.design.optics;
    let x = Extras::default();
    let a = analyze(o, &x);
    let vi_note = match a.image_kind.as_str() {
        "virtual" => format!(
            "{:.0} mm behind the lens + {:.0} mm lens-to-eye path (display {:.2} mm inside f)",
            a.image_behind_lens_mm,
            a.path_mm,
            a.f_mm - a.d_mm
        ),
        "collimated" => "display at the focal plane: image at infinity".into(),
        _ => "display beyond f: real image, not a heads-up display".into(),
    };
    vec![
        Metric::new("optics.hfov_deg", "Horizontal field of view", a.hfov_deg, "deg", "optics").with_note(&format!(
            "2·atan of the {:.0} mm-wide virtual image at {:.2} m",
            a.image_width_mm, a.virtual_image_m
        )),
        Metric::new("optics.ppd", "Angular resolution (average)", a.ppd, "px/deg", "optics").with_note(&format!(
            "{} px across {:.1}° ({:.2} arcmin per pixel at the centre)",
            o.px_w, a.hfov_deg, a.arcmin_per_px
        )),
        Metric::new(
            "optics.virtual_image_m",
            "Virtual image distance from the eye",
            // collimated → report a large finite distance (still outside 1–3 m)
            a.virtual_image_m.min(1e6),
            "m",
            "optics",
        )
        .with_note(&vi_note),
        Metric::new("optics.eyebox_mm", "Eyebox width (horizontal)", a.eyebox_mm, "mm", "optics").with_note(&format!(
            "D·|k| − w·L/d with D = {:.1} mm, L = {:.0} mm (≈ D − 2L·tan(FOV/2) = {:.2} mm); {:.1} mm with a {:.0} mm pupil fully filled",
            a.aperture_mm, a.path_mm, a.eyebox_approx_mm, a.eyebox_full_pupil_mm, a.pupil_mm
        )),
        Metric::new("optics.vfov_deg", "Vertical field of view", a.vfov_deg, "deg", "optics"),
        Metric::new("optics.dfov_deg", "Diagonal field of view", a.dfov_deg, "deg", "optics"),
        Metric::new("optics.image_nits", "Virtual image luminance", a.image_nits, "nits", "optics").with_note(&format!(
            "{:.0} nits × lens {:.0} % × combiner R {:.0} %",
            o.display_luminance_nits,
            a.lens_transmission * 100.0,
            o.combiner_reflectance * 100.0
        )),
        Metric::new(
            "optics.see_through_pct",
            "See-through transmission",
            a.see_through_pct,
            "%",
            "optics",
        ),
        Metric::new(
            "optics.combiner_clearance_mm",
            "Combiner-to-lens clearance",
            o.lens_to_combiner_mm - a.min_lens_to_combiner_mm,
            "mm",
            "optics",
        )
        .with_note(&format!(
            "lens-to-combiner {:.1} mm vs {:.1} mm needed so the 45° combiner clears an {:.1} mm aperture",
            o.lens_to_combiner_mm, a.min_lens_to_combiner_mm, a.aperture_mm
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_metrics() {
        let p = Project::iris();
        let m = metrics(&p);
        let get = |k: &str| m.iter().find(|x| x.key == k).unwrap().value;
        let hfov = get("optics.hfov_deg");
        assert!(hfov > 23.0 && hfov < 24.5, "{hfov}");
        let ppd = get("optics.ppd");
        assert!(ppd > 26.0 && ppd < 28.0, "{ppd}");
        let vi = get("optics.virtual_image_m");
        assert!(vi > 1.0 && vi < 3.0 && (vi - 1.342).abs() < 0.01, "{vi}");
        let eb = get("optics.eyebox_mm");
        assert!(eb > 5.5 && eb < 6.5, "{eb}");
        assert!(get("optics.combiner_clearance_mm") > 0.0);
        assert!((get("optics.see_through_pct") - 70.0).abs() < 1e-9);
        assert!(get("optics.dfov_deg") > hfov);
    }

    #[test]
    fn dispatch_roundtrip() {
        let out = dispatch("optics.analyze", &serde_json::json!({})).unwrap().unwrap();
        assert!(out["hfov_deg"].as_f64().unwrap() > 20.0);
        let t = dispatch("optics.trace", &serde_json::json!({ "config": { "eye_offset_mm": 2.0 } }))
            .unwrap()
            .unwrap();
        assert!(!t["rays"].as_array().unwrap().is_empty());
        for m in ["optics.design_sweep", "optics.legibility", "optics.contrast"] {
            assert!(dispatch(m, &serde_json::json!({})).unwrap().is_ok(), "{m}");
        }
        assert!(dispatch("optics.nope", &serde_json::json!({})).is_none());
        // collimated and real configurations serialise (no infinities)
        for d in [20.0, 21.0] {
            let v = dispatch(
                "optics.trace",
                &serde_json::json!({ "optics": { "display_to_lens_mm": d, "lens_diameter_mm": 18.0 } }),
            )
            .unwrap();
            assert!(v.is_ok());
        }
    }

    #[test]
    fn metrics_are_fast() {
        let p = Project::iris();
        let t0 = std::time::Instant::now();
        for _ in 0..100 {
            let _ = metrics(&p);
        }
        assert!(t0.elapsed().as_millis() < 300);
    }
}

#[cfg(test)]
mod debug {
    use super::*;

    /// `cargo test -p forge-core optics::debug -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn dump() {
        let p = Project::iris();
        for m in metrics(&p) {
            println!("{:26} {:10.3} {:7} {}", m.key, m.value, m.unit, m.note);
        }
        let a = analyze(&p.design.optics, &Extras::default());
        println!(
            "ppd centre {:.2}, f/# {:.2}, combiner {:.1} mm, focus {:.2} D/mm, contrast {:?}",
            a.ppd_center,
            a.f_number,
            a.combiner_min_mm,
            a.focus_sensitivity_d_per_mm,
            a.contrast
                .iter()
                .map(|c| (c.name.clone(), (c.contrast_ratio * 100.0).round() / 100.0))
                .collect::<Vec<_>>()
        );
        let s = design::focal_sweep(&p.design.optics, Some(&p), &design::SweepConfig::default());
        println!("band {:?} with eyebox {:?}", s.band_mm, s.band_eyebox_mm);
        let l = design::legibility(&p.design.optics, &design::LegibilityConfig::default());
        println!("min em 16' {} 20' {}", l.min_em_16, l.min_em_20);
    }
}
