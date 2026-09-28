//! Optics Pro: JSON API and metrics for the real-lens tools — glass
//! catalogue, sequential ray tracing, image quality and lens optimisation
//! of the Iris heads-up display lens (see [`super::lensdesign`]).
//!
//! | method | input | output |
//! |---|---|---|
//! | `optics.materials` | – | catalogue with nd, Vd, n(λ) curves |
//! | `optics.lens_presets` | `{project?, optics?}` | presets as reversed-HUD systems |
//! | `optics.lens_analyze` | `{project?, optics?, preset? \| system?, config?}` | layout, paraxial data, spots, fans, field curves, colour, OPD, MTF |
//! | `optics.lens_optimize` | `{project?, optics?, preset? \| system?, variables?, max_iter?}` | damped-least-squares history (+ a layout per step) |
//! | `optics.achromat` | `{f_mm, crown, flint}` | thin-lens achromat powers and starting radii |
//!
//! Metrics (informational; worst field, lens chosen by
//! `design.extra.optics_lens`, default [`lensdesign::DEFAULT_LENS`]):
//! `optics.rms_spot_um`, `optics.rms_spot_arcmin`, `optics.mtf_at_nyquist`,
//! `optics.lateral_color_arcmin`. The MTF is the **geometric** MTF (Fourier
//! transform of the ray-spot distribution) multiplied by the diffraction MTF
//! of the circular pupil — not an FFT of the diffraction point spread
//! function (PSF).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::glass;
use super::lensdesign::{self, OptConfig, Variable, LENS_START};
use super::paraxial::Extras;
use super::quality::{self, Quality, QualityConfig};
use super::seq::{self, Layout, Surface, System};
use crate::api::{handle, ApiResult};
use crate::project::{Metric, OpticsParams, Project};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct Who {
    project: Option<Project>,
    optics: Option<OpticsParams>,
}

impl Who {
    fn params(&self) -> OpticsParams {
        self.optics
            .clone()
            .or_else(|| self.project.as_ref().map(|p| p.design.optics.clone()))
            .unwrap_or_else(|| Project::iris().design.optics)
    }
}

#[derive(Clone, Debug, Serialize)]
struct PresetOut {
    id: String,
    title: String,
    summary: String,
    variables: Vec<Variable>,
    system: System,
    efl: f64,
    mass_g: f64,
}

fn preset_system(o: &OpticsParams, id: &str) -> Result<System, String> {
    let lens = lensdesign::preset_lens(id, o).ok_or_else(|| format!("unknown lens preset `{id}`"))?;
    let mut sys = lensdesign::hud_system(o, &Extras::default(), &lens);
    sys.name = format!(
        "Iris HUD — {}",
        lensdesign::presets()
            .into_iter()
            .find(|p| p.id == id)
            .map(|p| p.title)
            .unwrap_or_default()
    );
    lensdesign::paraxial_focus(&mut sys);
    Ok(sys)
}

fn presets_out(o: &OpticsParams) -> Vec<PresetOut> {
    lensdesign::presets()
        .into_iter()
        .filter_map(|p| {
            let sys = preset_system(o, &p.id).ok()?;
            let lens = &sys.surfaces[LENS_START..];
            Some(PresetOut {
                efl: lensdesign::lens_efl(lens),
                mass_g: lensdesign::lens_mass_g(lens),
                id: p.id,
                title: p.title,
                summary: p.summary,
                variables: p.variables,
                system: sys,
            })
        })
        .collect()
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
struct AnalyzeCfg {
    rings: usize,
    rays_per_fan: usize,
    auto_focus: bool,
    fan_points: usize,
}

impl Default for AnalyzeCfg {
    fn default() -> Self {
        AnalyzeCfg {
            rings: 6,
            rays_per_fan: 9,
            auto_focus: true,
            fan_points: 33,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct AnalyzeIn {
    #[serde(flatten)]
    who: Who,
    preset: Option<String>,
    system: Option<System>,
    config: AnalyzeCfg,
}

#[derive(Serialize)]
struct AnalyzeOut {
    /// The analysed system with the focus applied.
    system: System,
    layout: Layout,
    quality: Quality,
    lens_efl: f64,
    lens_mass_g: f64,
    pixel_mm: f64,
    nyquist_lp_mm: f64,
    /// Edge / centre thickness of every element, mm.
    elements: Vec<ElementInfo>,
    /// Which lens the project's metrics use.
    project_lens: String,
}

#[derive(Serialize)]
struct ElementInfo {
    surfaces: [usize; 2],
    material: String,
    center_mm: f64,
    edge_mm: f64,
}

fn pick_system(who: &Who, preset: &Option<String>, system: &Option<System>) -> Result<System, String> {
    let o = who.params();
    match (system, preset) {
        (Some(s), _) if !s.surfaces.is_empty() => Ok(s.clone()),
        (_, Some(id)) => preset_system(&o, id),
        _ => {
            let id = who
                .project
                .as_ref()
                .map(|p| lensdesign::project_lens(p).0)
                .unwrap_or_else(|| lensdesign::DEFAULT_LENS.into());
            if id == "custom" {
                let p = who.project.as_ref().ok_or("custom lens needs the project")?;
                let (_, lens) = lensdesign::project_lens(p);
                let mut sys = lensdesign::hud_system(&o, &Extras::default(), &lens);
                lensdesign::paraxial_focus(&mut sys);
                Ok(sys)
            } else {
                preset_system(&o, &id)
            }
        }
    }
}

fn analyze(i: AnalyzeIn) -> Result<AnalyzeOut, String> {
    let o = i.who.params();
    let sys = pick_system(&i.who, &i.preset, &i.system)?;
    if sys.surfaces.len() > 60 {
        return Err("at most 60 surfaces".into());
    }
    let (pitch, nyq) = lensdesign::display_sampling(&o);
    let q = quality::analyze(
        &sys,
        &QualityConfig {
            rings: i.config.rings.clamp(1, 10),
            fan_points: i.config.fan_points.clamp(5, 101),
            nyquist_lp_mm: nyq,
            pixel_mm: pitch,
            auto_focus: i.config.auto_focus,
            ..Default::default()
        },
    )?;
    let mut sys = sys;
    let last = sys.surfaces.len() - 1;
    sys.surfaces[last].thickness = q.image_distance_mm;
    let prep = sys.prepare()?;
    let layout = seq::layout(&sys, &prep, i.config.rays_per_fan.clamp(2, 31));
    let start = sys
        .surfaces
        .iter()
        .position(|s| s.is_ideal() || s.curvature() != 0.0 || !s.material.trim().is_empty())
        .unwrap_or(0);
    let lens = &sys.surfaces[start..];
    let elements = layout
        .elements
        .iter()
        .map(|e| ElementInfo {
            surfaces: [e.from, e.to],
            material: e.material.clone(),
            center_mm: e.center_mm,
            edge_mm: e.edge_mm,
        })
        .collect();
    Ok(AnalyzeOut {
        lens_efl: seq::fin(lensdesign::lens_efl(lens)),
        lens_mass_g: lensdesign::lens_mass_g(lens),
        pixel_mm: pitch,
        nyquist_lp_mm: nyq,
        elements,
        layout,
        quality: q,
        system: sys,
        project_lens: i
            .who
            .project
            .as_ref()
            .map(|p| lensdesign::project_lens(p).0)
            .unwrap_or_else(|| lensdesign::DEFAULT_LENS.into()),
    })
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct OptimizeIn {
    #[serde(flatten)]
    who: Who,
    preset: Option<String>,
    system: Option<System>,
    /// Index of the first lens surface (default: the HUD layout's 2).
    lens_start: Option<usize>,
    variables: Option<Vec<Variable>>,
    max_iter: Option<usize>,
    efl_mm: Option<f64>,
}

#[derive(Serialize)]
struct OptimizeOut {
    #[serde(flatten)]
    result: lensdesign::OptResult,
    /// One meridional drawing per history step (for animating the lens bending).
    layouts: Vec<Layout>,
    variables: Vec<Variable>,
}

fn optimize(i: OptimizeIn) -> Result<OptimizeOut, String> {
    let o = i.who.params();
    let sys = pick_system(&i.who, &i.preset, &i.system)?;
    let start = i.lens_start.unwrap_or(LENS_START).min(sys.surfaces.len().saturating_sub(1));
    let variables = match (&i.variables, &i.preset) {
        (Some(v), _) if !v.is_empty() => v.clone(),
        (_, Some(id)) => lensdesign::presets()
            .into_iter()
            .find(|p| &p.id == id)
            .map(|p| p.variables)
            .unwrap_or_default(),
        _ => {
            // default: every curvature in the lens group
            (start..sys.surfaces.len())
                .filter(|&k| !sys.surfaces[k].is_ideal() && !sys.surfaces[k].fold)
                .map(|k| Variable {
                    surface: k - start,
                    param: "curvature".into(),
                })
                .collect()
        }
    };
    if variables.is_empty() {
        return Err("nothing to optimise: choose at least one variable (curvature, conic or aspheric term)".into());
    }
    let cfg = OptConfig {
        variables: variables.clone(),
        efl_mm: i.efl_mm.unwrap_or(o.lens_focal_mm),
        max_iter: i.max_iter.unwrap_or(40).clamp(1, 150),
        ..Default::default()
    };
    let r = lensdesign::optimize(&sys, start, &cfg)?;
    let mut layouts = Vec::new();
    for step in &r.history {
        let mut s = sys.clone();
        s.surfaces.truncate(start);
        s.surfaces.extend(step.surfaces.iter().cloned());
        let mut s2 = s.clone();
        s2.wavelengths_nm = vec![sys.wavelengths_nm[sys.primary()]];
        s2.primary = 0;
        if let Ok(p) = s2.prepare() {
            layouts.push(seq::layout(&s2, &p, 5));
        }
    }
    Ok(OptimizeOut {
        result: r,
        layouts,
        variables,
    })
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
struct AchromatIn {
    f_mm: f64,
    crown: String,
    flint: String,
}

impl Default for AchromatIn {
    fn default() -> Self {
        AchromatIn {
            f_mm: 20.0,
            crown: "PMMA".into(),
            flint: "PC".into(),
        }
    }
}

#[derive(Serialize)]
struct AchromatOut {
    n1: f64,
    n2: f64,
    v1: f64,
    v2: f64,
    phi1: f64,
    phi2: f64,
    f1_mm: f64,
    f2_mm: f64,
    /// Equiconvex crown, cemented: R1 = −R2, R3 from the flint power.
    r1_mm: f64,
    r2_mm: f64,
    r3_mm: f64,
    /// Petzval sum Σ φ/(n) and radius, mm.
    petzval_radius_mm: f64,
    /// Longitudinal colour of a single lens of the same power (f/V) vs the achromat's secondary spectrum estimate.
    singlet_lca_mm: f64,
}

fn achromat(i: AchromatIn) -> Result<AchromatOut, String> {
    let m1 = glass::lookup(&i.crown)?.ok_or("crown must be a material, not air")?;
    let m2 = glass::lookup(&i.flint)?.ok_or("flint must be a material, not air")?;
    let (n1, n2, v1, v2) = (m1.nd(), m2.nd(), m1.vd(), m2.vd());
    if (v1 - v2).abs() < 1.0 {
        return Err("the two materials have almost the same Abbe number: no achromat possible".into());
    }
    let phi = 1.0 / i.f_mm;
    let (p1, p2) = lensdesign::achromat_powers(phi, v1, v2);
    let c1 = p1 / (n1 - 1.0) / 2.0;
    let c2 = -c1;
    let c3 = c2 - p2 / (n2 - 1.0);
    let inv = |c: f64| if c.abs() < 1e-12 { 0.0 } else { 1.0 / c };
    let pz = p1 / n1 + p2 / n2;
    Ok(AchromatOut {
        n1,
        n2,
        v1,
        v2,
        phi1: p1,
        phi2: p2,
        f1_mm: 1.0 / p1,
        f2_mm: 1.0 / p2,
        r1_mm: inv(c1),
        r2_mm: inv(c2),
        r3_mm: inv(c3),
        petzval_radius_mm: if pz.abs() > 1e-12 { -1.0 / pz } else { 1e9 },
        singlet_lca_mm: i.f_mm / v1,
    })
}

/// JSON API for the Optics Pro methods (`None` for other methods).
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "optics.materials" => handle(input, |_: Value| Ok(glass::catalog_table())),
        "optics.lens_presets" => handle(input, |i: Who| Ok(presets_out(&i.params()))),
        "optics.lens_analyze" => handle(input, analyze),
        "optics.lens_optimize" => handle(input, optimize),
        "optics.achromat" => handle(input, achromat),
        _ => None,
    }
}

thread_local! {
    static CACHE: crate::memo::Slot<Vec<Metric>> = const { std::cell::RefCell::new(None) };
}

/// Image-quality metrics of the project's HUD lens (worst field), memoised.
pub fn metrics(p: &Project) -> Vec<Metric> {
    let o = &p.design.optics;
    let (id, lens) = lensdesign::project_lens(p);
    let key = crate::memo::key(&(o, &id, &lens));
    crate::memo::memo(&CACHE, key, || metrics_uncached(o, &id, &lens))
}

fn metrics_uncached(o: &OpticsParams, id: &str, lens: &[Surface]) -> Vec<Metric> {
    let Ok((sys, q)) = lensdesign::hud_quality(o, lens, true) else {
        return vec![];
    };
    let title = lensdesign::presets()
        .into_iter()
        .find(|x| x.id == id)
        .map(|x| x.title)
        .unwrap_or_else(|| "custom lens".into());
    let worst_field = q
        .spots
        .iter()
        .max_by(|a, b| a.rms_mm.partial_cmp(&b.rms_mm).unwrap_or(std::cmp::Ordering::Equal))
        .map(|s| s.field_deg)
        .unwrap_or(0.0);
    let fmax = sys.fields_deg.iter().cloned().fold(0.0, f64::max);
    let pitch_um = o.display_width_mm / o.px_w.max(1) as f64 * 1000.0;
    let rms_um = q.worst_rms_mm * 1000.0;
    let centre_mtf = q.mtf.fields.first().map(|m| m.at_nyquist_t.min(m.at_nyquist_s)).unwrap_or(0.0);
    vec![
        Metric::new("optics.rms_spot_um", "RMS spot radius on the display (worst field)", rms_um, "um", "optics").with_note(&format!(
            "{title}: polychromatic (F, d, C) RMS radius about the centroid, real rays from a 4 mm eye pupil (reversed model), worst at {worst_field:.1}° of 0–{fmax:.1}°, display at best focus; = {:.1} pixels of {pitch_um:.1} µm",
            rms_um / pitch_um.max(1e-9)
        )),
        Metric::new("optics.rms_spot_arcmin", "RMS spot size seen by the eye (worst field)", q.worst_rms_arcmin, "arcmin", "optics").with_note(&format!(
            "RMS spot / EFL; one pixel = {:.2}′",
            q.pixel_arcmin
        )),
        Metric::new("optics.mtf_at_nyquist", "MTF at the display Nyquist frequency (worst field)", q.worst_mtf_nyquist, "", "optics").with_note(&format!(
            "geometric MTF of the spot × diffraction MTF of the pupil, min of tangential/sagittal, at {:.1} cycles/mm (pixel pitch {pitch_um:.1} µm); on-axis {centre_mtf:.2}; diffraction limit alone {:.2}",
            q.mtf.nyquist_lp_mm, q.mtf.diffraction_at_nyquist
        )),
        Metric::new("optics.lateral_color_arcmin", "Lateral colour F−C (worst field)", q.worst_lateral_arcmin, "arcmin", "optics").with_note(&format!(
            "chief-ray image height difference between 486 nm and 656 nm, as an angle at the eye ({title})"
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_roundtrip() {
        let v = dispatch("optics.materials", &serde_json::json!({})).unwrap().unwrap();
        assert!(v["materials"].as_array().unwrap().len() >= 7);
        let v = dispatch("optics.lens_presets", &serde_json::json!({})).unwrap().unwrap();
        assert!(v.as_array().unwrap().len() >= 5);
        let a = dispatch("optics.lens_analyze", &serde_json::json!({ "preset": "pmma_pcx" }))
            .unwrap()
            .unwrap();
        assert!(a["quality"]["spots"].as_array().unwrap().len() == 4);
        assert!(a["layout"]["rays"].as_array().unwrap().len() > 10);
        let ach = dispatch(
            "optics.achromat",
            &serde_json::json!({ "f_mm": 20, "crown": "N-BK7", "flint": "N-SF11" }),
        )
        .unwrap()
        .unwrap();
        let (p1, p2) = (ach["phi1"].as_f64().unwrap(), ach["phi2"].as_f64().unwrap());
        assert!((p1 / 64.17 + p2 / 25.68).abs() < 1e-4);
        assert!(dispatch("optics.nope", &serde_json::json!({})).is_none());
    }

    #[test]
    fn optimiser_improves_a_bent_singlet() {
        let r = dispatch(
            "optics.lens_optimize",
            &serde_json::json!({ "preset": "pmma_pcx", "variables": [{"surface": 0, "param": "curvature"}, {"surface": 1, "param": "curvature"}], "max_iter": 8 }),
        )
        .unwrap()
        .unwrap();
        let h = r["history"].as_array().unwrap();
        assert!(h.len() >= 2);
        let first = h[0]["merit"].as_f64().unwrap();
        let last = h[h.len() - 1]["merit"].as_f64().unwrap();
        assert!(last < first, "{first} → {last}");
        assert_eq!(r["layouts"].as_array().unwrap().len(), h.len());
    }

    #[test]
    fn metrics_are_present_and_fast() {
        let p = Project::iris();
        let t0 = std::time::Instant::now();
        let m = metrics_uncached(&p.design.optics, lensdesign::DEFAULT_LENS, &lensdesign::project_lens(&p).1);
        let dt = t0.elapsed();
        let get = |k: &str| m.iter().find(|x| x.key == k).unwrap().value;
        let rms = get("optics.rms_spot_arcmin");
        let um = get("optics.rms_spot_um");
        // µm on the display ↔ arcmin at the eye through the 20 mm EFL
        assert!(
            (um / 1000.0 / p.design.optics.lens_focal_mm * quality::RAD_TO_ARCMIN - rms).abs() < 0.02 * rms,
            "{um} µm vs {rms}′"
        );
        let mtf = get("optics.mtf_at_nyquist");
        let lat = get("optics.lateral_color_arcmin");
        assert!(rms > 0.0 && rms < 100.0, "{rms}");
        assert!((0.0..=1.0).contains(&mtf), "{mtf}");
        assert!(lat >= 0.0);
        // debug build: generous bound (release WebAssembly is ~10–30× faster)
        assert!(dt.as_millis() < 400, "{dt:?}");
        // the ideal lens meets everything
        let mut p2 = p.clone();
        p2.design.extra.insert("optics_lens".into(), serde_json::json!("ideal"));
        let m2 = metrics(&p2);
        let rms2 = m2.iter().find(|x| x.key == "optics.rms_spot_um").unwrap().value;
        assert!(rms2 < 1e-6);
        let mtf2 = m2.iter().find(|x| x.key == "optics.mtf_at_nyquist").unwrap().value;
        assert!(mtf2 > 0.8, "ideal lens is diffraction-limited: {mtf2}");
    }
}
