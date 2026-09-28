//! Lens design for the Iris heads-up display (HUD): catalogue presets, the
//! reversed eyepiece model, the achromat condition and a damped
//! least-squares optimiser.
//!
//! **Reversed model.** Eyepieces and head-mounted displays are designed
//! backwards (Smith, *Modern Optical Engineering* 4th ed., ch. 13; Kress
//! 2020, ch. 5): the **eye pupil is the aperture stop**, rays leave it at the
//! field angles the wearer looks along, cross the (unfolded) 45° combiner,
//! pass through the collimating lens and land on the micro-display. The
//! object is the virtual image (1–3 m away, traced as a *virtual* object) or
//! infinity. Spot sizes on the display convert to angles seen by the eye by
//! dividing by the effective focal length (EFL); one display pixel subtends
//! `pitch / EFL`.
//!
//! **Achromat.** Two thin lenses in contact with powers φ₁, φ₂ and Abbe
//! numbers V₁, V₂ have no longitudinal colour when `φ₁/V₁ + φ₂/V₂ = 0`, so
//! `φ₁ = φ V₁/(V₁ − V₂)`, `φ₂ = −φ V₂/(V₁ − V₂)` (Kingslake & Johnson,
//! *Lens Design Fundamentals* 2nd ed., 2010, ch. 5; Smith ch. 13). PMMA
//! (V ≈ 57) with polycarbonate (V ≈ 30) is the classic moulded-plastic pair.
//!
//! **Optimiser.** Damped least squares (Levenberg 1944; Marquardt 1963),
//! the workhorse of Zemax/CODE V local optimisation: residuals are the
//! angular ray errors about each field's polychromatic centroid (so the
//! merit is the RMS spot size in arcminutes) plus a heavily weighted
//! effective-focal-length target; the display is refocused in closed form
//! at every evaluation; element centre thicknesses follow the curvatures so
//! the edges stay at least `min_edge_mm` thick.

use serde::{Deserialize, Serialize};

use super::glass;
use super::paraxial::{analyze as thin_analyze, Extras};
use super::quality::{self, best_focus_shift, hexapolar, QualityConfig, RAD_TO_ARCMIN};
use super::seq::{paraxial, sag_at, trace, Fail, Surface, System};
use crate::project::OpticsParams;

/// Presets are designed at this focal length and scaled to the project's.
pub const DESIGN_F_MM: f64 = 20.0;
/// Default lens for the `optics.*` image-quality metrics.
pub const DEFAULT_LENS: &str = "achromat_pmma_pc";
/// Minimum element edge thickness kept by the thickness fit, mm.
pub const MIN_EDGE_MM: f64 = 0.8;
/// Minimum element centre thickness, mm.
pub const MIN_CENTER_MM: f64 = 1.0;

fn s(label: &str, radius: f64, thickness: f64, material: &str) -> Surface {
    Surface {
        label: label.into(),
        radius,
        thickness,
        material: material.into(),
        ..Default::default()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LensPreset {
    pub id: String,
    pub title: String,
    pub summary: String,
    /// Lens surfaces only (first faces the eye); the last thickness is replaced by the focus.
    pub lens: Vec<Surface>,
    /// Suggested optimisation variables.
    pub variables: Vec<Variable>,
}

/// One optimisation variable.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Variable {
    /// Index into the *lens* surfaces (0 = first lens surface).
    pub surface: usize,
    /// `curvature`, `conic`, `a4`, `a6`, `a8`, `a10` or `thickness`.
    pub param: String,
}

fn var(surface: usize, param: &str) -> Variable {
    Variable {
        surface,
        param: param.into(),
    }
}

/// Built-in lens presets at f = 20 mm (before scaling).
pub fn presets() -> Vec<LensPreset> {
    let n_pmma = glass::index_of("PMMA", glass::LAMBDA_D).unwrap_or(1.4918);
    // (a) ideal thin lens
    let ideal = vec![Surface {
        label: "ideal lens".into(),
        paraxial_f: DESIGN_F_MM,
        thickness: DESIGN_F_MM,
        ..Default::default()
    }];
    // (b) plano-convex PMMA, convex side towards the eye (the collimated side)
    let pcx = vec![s("PMMA front", (n_pmma - 1.0) * DESIGN_F_MM, 6.0, "PMMA"), s("PMMA back (flat)", 0.0, 15.0, "")];
    // (c) aspheric PMMA singlet — optimised by `optimize` from the plano-convex start
    //     (see the test `asphere_preset_is_a_local_optimum`)
    let asph = vec![
        Surface {
            label: "PMMA asphere front".into(),
            radius: ASPH[0],
            conic: ASPH[1],
            aspheric: vec![ASPH[2], ASPH[3]],
            thickness: 6.0,
            material: "PMMA".into(),
            ..Default::default()
        },
        Surface {
            label: "PMMA asphere back".into(),
            radius: ASPH[4],
            aspheric: vec![ASPH[5], ASPH[6]],
            thickness: 15.0,
            ..Default::default()
        },
    ];
    // (d) cemented PMMA + polycarbonate achromat (UV-cured optical adhesive), optimised from the thin-lens solution
    let ach = vec![
        Surface {
            label: "PMMA front (asphere)".into(),
            radius: ACH_PC[0],
            conic: ACH_PC[1],
            aspheric: vec![ACH_PC[2], ACH_PC[3]],
            thickness: 7.0,
            material: "PMMA".into(),
            ..Default::default()
        },
        s("PMMA/PC cemented", ACH_PC[4], 1.2, "PC"),
        Surface {
            label: "PC back (asphere)".into(),
            radius: ACH_PC[5],
            aspheric: vec![ACH_PC[6], ACH_PC[7]],
            thickness: 12.0,
            ..Default::default()
        },
    ];
    // (e) cemented glass achromat N-BK7 + N-SF11, from the thin-lens solution, optimised bending
    let glass_ach = vec![s("N-BK7 front", ACH_G[0], 6.0, "N-BK7"), s("cemented", ACH_G[1], 1.2, "N-SF11"), s("N-SF11 back", ACH_G[2], 12.0, "")];
    vec![
        LensPreset {
            id: "ideal".into(),
            title: "Ideal thin lens".into(),
            summary: "The paraxial lens the first-order tabs assume: aberration-free by definition — the reference every real lens is measured against.".into(),
            lens: ideal,
            variables: vec![],
        },
        LensPreset {
            id: "pmma_pcx".into(),
            title: "PMMA plano-convex".into(),
            summary: "The cheapest real lens: one spherical acrylic surface, R = (n − 1)·f. Field curvature, astigmatism and lateral colour grow fast off-axis.".into(),
            lens: pcx,
            variables: vec![var(0, "curvature"), var(1, "curvature")],
        },
        LensPreset {
            id: "pmma_asphere".into(),
            title: "Aspheric PMMA singlet".into(),
            summary: "Both surfaces bent and made aspheric (conic + A4/A6) by the optimiser: moulded plastic makes aspheres free. Monochromatic blur drops; colour stays.".into(),
            lens: asph,
            variables: vec![
                var(0, "curvature"),
                var(0, "conic"),
                var(0, "a4"),
                var(0, "a6"),
                var(1, "curvature"),
                var(1, "a4"),
                var(1, "a6"),
            ],
        },
        LensPreset {
            id: "achromat_pmma_pc".into(),
            title: "PMMA + PC achromat".into(),
            summary: "Positive acrylic 'crown' and negative polycarbonate 'flint' satisfying φ₁/V₁ + φ₂/V₂ = 0, with one aspheric surface: kills lateral colour, still injection-mouldable.".into(),
            lens: ach,
            variables: vec![
                var(0, "curvature"),
                var(0, "conic"),
                var(0, "a4"),
                var(0, "a6"),
                var(1, "curvature"),
                var(2, "curvature"),
                var(2, "a4"),
                var(2, "a6"),
            ],
        },
        LensPreset {
            id: "achromat_glass".into(),
            title: "Glass achromat (N-BK7 + N-SF11)".into(),
            summary: "Classic cemented crown–flint doublet, spherical surfaces only. Excellent colour correction but heavy (N-SF11 is 4.7 g/cm³).".into(),
            lens: glass_ach,
            variables: vec![var(0, "curvature"), var(1, "curvature"), var(2, "curvature")],
        },
    ]
}

// Baked optimiser results at f = 20 mm, 18 mm aperture, 4 mm eye pupil,
// ±14° diagonal field (Iris defaults). Regenerate with
// `cargo test -p forge-core optics::lensdesign::bake -- --ignored --nocapture`.
const ASPH: [f64; 7] = [9.93444, -0.2324, -2.3589345881209228e-4, 5.975621675626905e-7, -825.35903, -2.173599004520222e-4, 1.949397185545372e-6];
const ACH_PC: [f64; 8] = [9.42, 0.0, 0.0, 0.0, -9.42, -75.0, 0.0, 0.0];
const ACH_G: [f64; 3] = [12.4, -12.4, -26.2];

/// Thin-lens achromat powers `(φ₁, φ₂)` for total power `phi`.
pub fn achromat_powers(phi: f64, v1: f64, v2: f64) -> (f64, f64) {
    let dv = v1 - v2;
    (phi * v1 / dv, -phi * v2 / dv)
}

/// Scale a lens by `k` (all lengths × k; aspheric A_{2m} ÷ k^{2m−1}).
pub fn scale_lens(lens: &mut [Surface], k: f64) {
    for s in lens.iter_mut() {
        s.radius *= k;
        s.thickness *= k;
        s.semi_diameter *= k;
        s.paraxial_f *= k;
        for (j, a) in s.aspheric.iter_mut().enumerate() {
            let m = (j + 2) as i32;
            *a /= k.powi(2 * m - 1);
        }
    }
}

/// Largest usable height of a surface (inside its conic domain).
fn max_height(s: &Surface) -> f64 {
    let c = s.curvature();
    let k1 = 1.0 + s.conic;
    if c != 0.0 && k1 > 0.0 {
        0.98 / (c.abs() * k1.sqrt())
    } else {
        f64::INFINITY
    }
}

/// Grow element centre thicknesses so every edge is ≥ `min_edge` (and centres ≥ `MIN_CENTER_MM`).
/// Surfaces are indexed within `lens`; the last surface's thickness (the focus) is untouched.
pub fn fit_thickness(lens: &mut [Surface], min_edge: f64) {
    let n = lens.len();
    for i in 0..n.saturating_sub(1) {
        let m = lens[i].material.trim().to_string();
        let glass = !(m.is_empty() || m.eq_ignore_ascii_case("air") || lens[i].is_mirror());
        let h = lens[i].semi_diameter.max(lens[i + 1].semi_diameter);
        let h = h.min(max_height(&lens[i])).min(max_height(&lens[i + 1]));
        let edge_extra = sag_at(&lens[i + 1], h) - sag_at(&lens[i], h);
        if glass {
            // centre t: edge = t + sag2(h) − sag1(h) ≥ min_edge
            let need = (min_edge - edge_extra).max(MIN_CENTER_MM);
            lens[i].thickness = need;
        } else {
            // air gap inside the lens group: keep the surfaces from crossing at the edge
            let need = (0.05 - edge_extra).max(0.02);
            if lens[i].thickness < need {
                lens[i].thickness = need;
            }
        }
    }
}

/// Build the reversed HUD system: eye pupil (stop) → unfolded combiner → lens → display.
pub fn hud_system(o: &OpticsParams, x: &Extras, lens: &[Surface]) -> System {
    let a = thin_analyze(o, x);
    let mut surfaces = vec![
        Surface {
            label: "eye pupil (stop)".into(),
            thickness: o.eye_relief_mm,
            semi_diameter: x.pupil_mm / 2.0,
            ..Default::default()
        },
        Surface {
            label: "combiner (45° fold, unfolded)".into(),
            thickness: o.lens_to_combiner_mm,
            fold: true,
            ..Default::default()
        },
    ];
    surfaces.extend(lens.iter().cloned());
    let object = if a.image_kind == "virtual" && a.virtual_image_m.is_finite() && a.virtual_image_m < 1e3 {
        Some(-a.virtual_image_m * 1000.0)
    } else {
        None
    };
    let half = (a.dfov_deg / 2.0).max(0.1);
    System {
        name: "Iris HUD (reversed: eye → display)".into(),
        surfaces,
        pupil_diameter_mm: x.pupil_mm,
        object_distance_mm: object,
        fields_deg: vec![0.0, 0.5 * half, 0.7071 * half, half],
        wavelengths_nm: vec![glass::LAMBDA_F, glass::LAMBDA_D, glass::LAMBDA_C],
        primary: 1,
    }
}

/// Index of the first lens surface in a HUD system.
pub const LENS_START: usize = 2;

/// A preset lens scaled to the project's focal length and aperture, with fitted thicknesses.
pub fn preset_lens(id: &str, o: &OpticsParams) -> Option<Vec<Surface>> {
    let p = presets().into_iter().find(|p| p.id == id)?;
    Some(adapt_lens(p.lens, o))
}

/// Scale a lens designed at `DESIGN_F_MM` to `o.lens_focal_mm`, set clear
/// apertures to `o.lens_diameter_mm`, refit thicknesses and rescale so the
/// EFL equals f exactly.
pub fn adapt_lens(mut lens: Vec<Surface>, o: &OpticsParams) -> Vec<Surface> {
    let f = o.lens_focal_mm.max(1.0);
    scale_lens(&mut lens, f / DESIGN_F_MM);
    let sd = o.lens_diameter_mm / 2.0;
    for _ in 0..4 {
        for s in lens.iter_mut() {
            if !s.is_ideal() {
                s.semi_diameter = sd.min(max_height(s));
            }
        }
        fit_thickness(&mut lens, MIN_EDGE_MM);
        let efl = lens_efl(&lens);
        if !(efl.is_finite() && efl > 0.0) || (efl - f).abs() < 1e-9 {
            break;
        }
        let k = f / efl;
        scale_lens(&mut lens, k);
    }
    for s in lens.iter_mut() {
        if !s.is_ideal() {
            s.semi_diameter = sd.min(max_height(s));
        }
    }
    lens
}

/// EFL (d line) of a lens on its own.
pub fn lens_efl(lens: &[Surface]) -> f64 {
    let mut surfaces = vec![Surface {
        thickness: 1.0,
        ..Default::default()
    }];
    surfaces.extend(lens.iter().cloned());
    let sys = System {
        surfaces,
        wavelengths_nm: vec![glass::LAMBDA_D],
        primary: 0,
        ..Default::default()
    };
    match sys.prepare() {
        Ok(p) => paraxial(&sys, &p, 0).efl,
        Err(_) => f64::NAN,
    }
}

/// Put the display at the paraxial image of the primary wavelength.
pub fn paraxial_focus(sys: &mut System) {
    let Ok(prep) = sys.prepare() else { return };
    let px = paraxial(sys, &prep, sys.primary());
    let last = sys.surfaces.len() - 1;
    if px.z_paraxial_image.is_finite() {
        let t = px.z_paraxial_image - prep.z[last];
        if t.is_finite() && t.abs() < 1e4 {
            sys.surfaces[last].thickness = t;
        }
    }
}

/// Display geometry for the quality analysis: (pixel pitch mm, Nyquist cycles/mm).
pub fn display_sampling(o: &OpticsParams) -> (f64, f64) {
    let pitch = o.display_width_mm / (o.px_w.max(1) as f64);
    (pitch, 0.5 / pitch)
}

/// Lens mass from its element volumes (solid of revolution between the surfaces), g.
pub fn lens_mass_g(lens: &[Surface]) -> f64 {
    let mut m = 0.0;
    for i in 0..lens.len().saturating_sub(1) {
        let Ok(Some(mat)) = glass::lookup(&lens[i].material) else { continue };
        let h = lens[i].semi_diameter.max(lens[i + 1].semi_diameter).min(max_height(&lens[i])).min(max_height(&lens[i + 1]));
        // ∫ 2πr (t + sag2(r) − sag1(r)) dr
        let n = 60;
        let mut v = 0.0;
        for k in 0..n {
            let r = h * (k as f64 + 0.5) / n as f64;
            let th = (lens[i].thickness + sag_at(&lens[i + 1], r) - sag_at(&lens[i], r)).max(0.0);
            v += 2.0 * std::f64::consts::PI * r * th * (h / n as f64);
        }
        m += v * 1e-3 * mat.density_g_cm3;
    }
    m
}

// ---------------------------------------------------------------------------
// Optimiser
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct OptConfig {
    pub variables: Vec<Variable>,
    /// Target EFL, mm (0 = keep the starting EFL).
    pub efl_mm: f64,
    pub max_iter: usize,
    /// Pupil rings used by the merit function.
    pub rings: usize,
    pub min_edge_mm: f64,
}

impl Default for OptConfig {
    fn default() -> Self {
        OptConfig {
            variables: vec![],
            efl_mm: 0.0,
            max_iter: 40,
            rings: 4,
            min_edge_mm: MIN_EDGE_MM,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct OptStep {
    pub iter: usize,
    /// RMS spot over all fields (arcmin), the merit without the EFL term.
    pub rms_arcmin: f64,
    /// Full merit (with the EFL penalty).
    pub merit: f64,
    pub efl: f64,
    pub damping: f64,
    pub values: Vec<f64>,
    /// Lens surfaces at this step (index `lens_start..`).
    pub surfaces: Vec<Surface>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OptResult {
    pub history: Vec<OptStep>,
    pub system: System,
    pub start_rms_arcmin: f64,
    pub final_rms_arcmin: f64,
    pub evaluations: usize,
    pub converged: bool,
    pub note: String,
}

fn var_scale(p: &str) -> f64 {
    match p {
        "curvature" => 0.05,
        "conic" => 1.0,
        "a4" => 1e-4,
        "a6" => 1e-6,
        "a8" => 1e-8,
        "a10" => 1e-10,
        _ => 1.0,
    }
}

fn get_var(s: &Surface, p: &str) -> f64 {
    let asph = |k: usize| s.aspheric.get(k).copied().unwrap_or(0.0);
    match p {
        "curvature" => s.curvature(),
        "conic" => s.conic,
        "a4" => asph(0),
        "a6" => asph(1),
        "a8" => asph(2),
        "a10" => asph(3),
        "thickness" => s.thickness,
        _ => 0.0,
    }
}

fn set_var(s: &mut Surface, p: &str, v: f64) {
    let mut set_a = |k: usize, v: f64| {
        if s.aspheric.len() <= k {
            s.aspheric.resize(k + 1, 0.0);
        }
        s.aspheric[k] = v;
    };
    match p {
        "curvature" => s.radius = if v.abs() < 1e-9 { 0.0 } else { 1.0 / v },
        "conic" => s.conic = v,
        "a4" => set_a(0, v),
        "a6" => set_a(1, v),
        "a8" => set_a(2, v),
        "a10" => set_a(3, v),
        "thickness" => s.thickness = v.max(0.05),
        _ => {}
    }
}

struct Evaluator<'a> {
    base: &'a System,
    start: usize,
    cfg: &'a OptConfig,
    efl_target: f64,
    pupil: Vec<(f64, f64)>,
    evals: usize,
}

const FAIL_ARCMIN: f64 = 60.0;

enum Ray {
    Hit([f64; 2]),
    Vignetted,
    Failed,
}
const EFL_WEIGHT: f64 = 3000.0;

impl Evaluator<'_> {
    fn build(&self, v: &[f64]) -> System {
        let mut sys = self.base.clone();
        for (k, var) in self.cfg.variables.iter().enumerate() {
            if let Some(s) = sys.surfaces.get_mut(self.start + var.surface) {
                set_var(s, &var.param, v[k]);
            }
        }
        let fit_t = !self.cfg.variables.iter().any(|v| v.param == "thickness");
        if fit_t {
            let n = sys.surfaces.len();
            fit_thickness(&mut sys.surfaces[self.start..n], self.cfg.min_edge_mm);
        }
        sys
    }

    /// Residuals (arcmin) and the RMS-spot part of the merit.
    fn residuals(&mut self, v: &[f64]) -> (Vec<f64>, f64, f64, System) {
        self.evals += 1;
        let mut sys = self.build(v);
        let nw = sys.wavelengths_nm.len();
        let mut r = Vec::new();
        let Ok(mut prep) = sys.prepare() else {
            return (vec![1e6], 1e6, f64::NAN, sys);
        };
        let efl = paraxial(&sys, &prep, sys.primary()).efl;
        let shift = best_focus_shift(&sys, &prep, 2);
        if shift.is_finite() && shift.abs() < 1e3 {
            let last = sys.surfaces.len() - 1;
            sys.surfaces[last].thickness += shift;
            if let Ok(p) = sys.prepare() {
                prep = p;
            }
        }
        let k = RAD_TO_ARCMIN / self.efl_target.abs().max(1e-9);
        let mut sum2 = 0.0;
        let mut cnt = 0usize;
        for &f in &sys.fields_deg {
            let mut pts: Vec<Ray> = Vec::with_capacity(nw * self.pupil.len());
            for w in 0..nw {
                for &(px, py) in &self.pupil {
                    let (p, d, o) = sys.launch(f, px, py);
                    let ro = trace(&sys, &prep, w, p, d, o, false);
                    pts.push(match ro.fail {
                        None => {
                            let q = ro.at_z(prep.z_image);
                            Ray::Hit([q[0], q[1]])
                        }
                        // vignetted rays are excluded (as in Zemax); TIR and misses are penalised
                        Some((_, Fail::Clipped)) => Ray::Vignetted,
                        Some(_) => Ray::Failed,
                    });
                }
            }
            let good: Vec<[f64; 2]> = pts
                .iter()
                .filter_map(|p| match p {
                    Ray::Hit(q) => Some(*q),
                    _ => None,
                })
                .collect();
            let n = good.len().max(1) as f64;
            let cx = good.iter().map(|p| p[0]).sum::<f64>() / n;
            let cy = good.iter().map(|p| p[1]).sum::<f64>() / n;
            for p in &pts {
                let (ex, ey) = match p {
                    Ray::Hit(q) => ((q[0] - cx) * k, (q[1] - cy) * k),
                    Ray::Failed => (FAIL_ARCMIN, FAIL_ARCMIN),
                    Ray::Vignetted => (0.0, 0.0),
                };
                if !matches!(p, Ray::Vignetted) {
                    sum2 += ex * ex + ey * ey;
                    cnt += 1;
                }
                r.push(ex);
                r.push(ey);
            }
        }
        let rms = (sum2 / cnt.max(1) as f64).sqrt();
        // scale per-ray residuals so |r|² / count = RMS²
        let norm = 1.0 / (cnt.max(1) as f64).sqrt();
        for x in r.iter_mut() {
            *x *= norm;
        }
        let e = if efl.is_finite() { (efl - self.efl_target) / self.efl_target } else { 1.0 };
        r.push(EFL_WEIGHT * e);
        (r, rms, efl, sys)
    }
}

fn sumsq(r: &[f64]) -> f64 {
    r.iter().map(|x| x * x).sum()
}

/// Damped least squares (Levenberg–Marquardt) on the chosen variables.
/// `lens_start` is the index of the first lens surface in `sys` (variables index from it).
pub fn optimize(sys: &System, lens_start: usize, cfg: &OptConfig) -> Result<OptResult, String> {
    sys.prepare()?;
    let nv = cfg.variables.len();
    let efl0 = {
        let p = sys.prepare()?;
        paraxial(sys, &p, sys.primary()).efl
    };
    let efl_target = if cfg.efl_mm > 0.0 { cfg.efl_mm } else { efl0 };
    if !(efl_target.is_finite() && efl_target.abs() > 1e-6) {
        return Err("the starting system has no focal length (afocal)".into());
    }
    for v in &cfg.variables {
        if lens_start + v.surface >= sys.surfaces.len() {
            return Err(format!("variable on surface {} is out of range", v.surface));
        }
    }
    let mut ev = Evaluator {
        base: sys,
        start: lens_start,
        cfg,
        efl_target,
        pupil: hexapolar(cfg.rings.clamp(1, 6)),
        evals: 0,
    };
    let mut x: Vec<f64> = cfg
        .variables
        .iter()
        .map(|v| get_var(&sys.surfaces[lens_start + v.surface], &v.param))
        .collect();
    let (mut r, mut rms, mut efl, mut cur) = ev.residuals(&x);
    let start_rms = rms;
    let mut m = sumsq(&r);
    let lens_of = |s: &System| s.surfaces[lens_start..].to_vec();
    let mut history = vec![OptStep {
        iter: 0,
        rms_arcmin: rms,
        merit: m.sqrt(),
        efl,
        damping: 0.0,
        values: x.clone(),
        surfaces: lens_of(&cur),
    }];
    let mut lambda = 1e-2;
    let mut converged = nv == 0;
    let mut it = 0;
    while it < cfg.max_iter && nv > 0 {
        it += 1;
        // forward-difference Jacobian
        let nr = r.len();
        let mut jac = vec![vec![0.0; nv]; nr];
        for j in 0..nv {
            let h = 1e-6 * var_scale(&cfg.variables[j].param).max(x[j].abs() * 1e-2);
            let mut xp = x.clone();
            xp[j] += h;
            let (rp, _, _, _) = ev.residuals(&xp);
            for i in 0..nr.min(rp.len()) {
                jac[i][j] = (rp[i] - r[i]) / h;
            }
        }
        let mut a = vec![vec![0.0; nv]; nv];
        let mut g = vec![0.0; nv];
        for i in 0..nr {
            for p in 0..nv {
                g[p] += jac[i][p] * r[i];
                for q in 0..nv {
                    a[p][q] += jac[i][p] * jac[i][q];
                }
            }
        }
        let mut improved = false;
        for _ in 0..10 {
            let mut aa = a.clone();
            for p in 0..nv {
                aa[p][p] += lambda * a[p][p].max(1e-12) + 1e-18;
            }
            let Some(dx) = crate::math::solve(aa, g.iter().map(|v| -v).collect()) else {
                lambda *= 10.0;
                continue;
            };
            let xn: Vec<f64> = x.iter().zip(&dx).map(|(a, b)| a + b).collect();
            let (rn, rmsn, efln, sysn) = ev.residuals(&xn);
            let mn = sumsq(&rn);
            if mn.is_finite() && mn < m {
                let rel = (m - mn) / m.max(1e-300);
                x = xn;
                r = rn;
                m = mn;
                rms = rmsn;
                efl = efln;
                cur = sysn;
                lambda = (lambda / 3.0).max(1e-9);
                improved = true;
                if rel < 1e-7 {
                    converged = true;
                }
                break;
            }
            lambda *= 6.0;
        }
        history.push(OptStep {
            iter: it,
            rms_arcmin: rms,
            merit: m.sqrt(),
            efl,
            damping: lambda,
            values: x.clone(),
            surfaces: lens_of(&cur),
        });
        if !improved || converged {
            converged = true;
            break;
        }
    }
    let note = format!(
        "{} iterations, {} merit evaluations: RMS spot {:.2}′ → {:.2}′ with EFL held at {:.3} mm (now {:.3} mm)",
        it, ev.evals, start_rms, rms, efl_target, efl
    );
    Ok(OptResult {
        history,
        system: cur,
        start_rms_arcmin: start_rms,
        final_rms_arcmin: rms,
        evaluations: ev.evals,
        converged,
        note,
    })
}

// ---------------------------------------------------------------------------
// Project-level analysis (for metrics and the UI)
// ---------------------------------------------------------------------------

/// Which lens the project uses: `design.extra.optics_lens` (preset id or
/// `"custom"` with `design.extra.optics_lens_rx` = lens surfaces at the
/// project's focal length).
pub fn project_lens(p: &crate::project::Project) -> (String, Vec<Surface>) {
    let o = &p.design.optics;
    let id = p.design.extra.get("optics_lens").and_then(|v| v.as_str()).unwrap_or(DEFAULT_LENS).to_string();
    if id == "custom" {
        if let Some(rx) = p.design.extra.get("optics_lens_rx") {
            if let Ok(lens) = serde_json::from_value::<Vec<Surface>>(rx.clone()) {
                if !lens.is_empty() {
                    return (id, lens);
                }
            }
        }
    }
    match preset_lens(&id, o) {
        Some(l) => (id, l),
        None => (DEFAULT_LENS.into(), preset_lens(DEFAULT_LENS, o).unwrap_or_default()),
    }
}

/// Reversed-HUD image quality for a lens (fast path used by the metrics).
pub fn hud_quality(o: &OpticsParams, lens: &[Surface], lite: bool) -> Result<(System, quality::Quality), String> {
    let x = Extras::default();
    let mut sys = hud_system(o, &x, lens);
    paraxial_focus(&mut sys);
    let (pitch, nyq) = display_sampling(o);
    let q = quality::analyze(
        &sys,
        &QualityConfig {
            rings: if lite { 4 } else { 6 },
            nyquist_lp_mm: nyq,
            pixel_mm: pitch,
            auto_focus: true,
            lite,
            ..Default::default()
        },
    )?;
    let last = sys.surfaces.len() - 1;
    sys.surfaces[last].thickness = q.image_distance_mm;
    Ok((sys, q))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Project;

    #[test]
    fn achromat_condition() {
        let (p1, p2) = achromat_powers(0.05, 57.4, 29.9);
        assert!((p1 + p2 - 0.05).abs() < 1e-15);
        assert!((p1 / 57.4 + p2 / 29.9).abs() < 1e-15);
        assert!(p1 > 0.0 && p2 < 0.0);
    }

    #[test]
    fn scaling_preserves_shape_and_scales_focal_length() {
        let lens = presets().into_iter().find(|p| p.id == "achromat_glass").unwrap().lens;
        let f0 = lens_efl(&lens);
        let mut l2 = lens.clone();
        scale_lens(&mut l2, 1.5);
        assert!((lens_efl(&l2) - 1.5 * f0).abs() < 1e-9);
    }

    #[test]
    fn presets_adapt_to_the_project_focal_length() {
        let o = Project::iris().design.optics;
        for p in presets() {
            let l = preset_lens(&p.id, &o).unwrap();
            let f = lens_efl(&l);
            assert!((f - o.lens_focal_mm).abs() < 1e-3, "{}: {f}", p.id);
            // positive edge and centre thicknesses
            for i in 0..l.len() - 1 {
                if !l[i].material.is_empty() {
                    assert!(l[i].thickness >= MIN_CENTER_MM * 0.99, "{}", p.id);
                }
            }
        }
    }

    #[test]
    fn ideal_hud_is_perfect_and_real_lenses_are_not() {
        let o = Project::iris().design.optics;
        let (_, qi) = hud_quality(&o, &preset_lens("ideal", &o).unwrap(), true).unwrap();
        assert!(qi.worst_rms_arcmin < 1e-6, "{}", qi.worst_rms_arcmin);
        // pixel ≈ 2.26′ at f = 20 mm with 13.1 µm pixels
        assert!((qi.pixel_arcmin - 2.256).abs() < 0.01, "{}", qi.pixel_arcmin);
        let (_, qp) = hud_quality(&o, &preset_lens("pmma_pcx", &o).unwrap(), true).unwrap();
        assert!(qp.worst_rms_arcmin > 1.0, "{}", qp.worst_rms_arcmin);
        assert!(qp.worst_lateral_arcmin > 2.0, "a singlet has lateral colour: {}", qp.worst_lateral_arcmin);
    }
}

#[cfg(test)]
mod bake {
    use super::*;
    use crate::project::Project;

    fn run(o: &OpticsParams, name: &str, lens: Vec<Surface>, vars: Vec<Variable>) -> (f64, Vec<Surface>) {
        let lens = adapt_lens(lens, o);
        let (_, q0) = hud_quality(o, &lens, true).unwrap();
        let mut sys = hud_system(o, &Extras::default(), &lens);
        paraxial_focus(&mut sys);
        let t0 = std::time::Instant::now();
        let r = optimize(&sys, LENS_START, &OptConfig { variables: vars, efl_mm: o.lens_focal_mm, max_iter: 150, ..Default::default() }).unwrap();
        let lens2 = r.system.surfaces[LENS_START..].to_vec();
        let (_, q1) = hud_quality(o, &lens2, true).unwrap();
        println!(
            "{:22} start rms {:.2}' mtf {:.2} lat {:.2}'  → opt rms {:.2}' mtf {:.2} lat {:.2}' eflx {:.3} ({} ms, {})",
            name, q0.worst_rms_arcmin, q0.worst_mtf_nyquist, q0.worst_lateral_arcmin, q1.worst_rms_arcmin, q1.worst_mtf_nyquist,
            q1.worst_lateral_arcmin, lens_efl(&lens2), t0.elapsed().as_millis(), r.note
        );
        for s in &lens2 {
            println!("    R {:10.5} k {:8.4} A {:?} t {:.3} {}", s.radius, s.conic, s.aspheric, s.thickness, s.material);
        }
        println!("    mass {:.2} g  spots {:?}", lens_mass_g(&lens2), q1.spots.iter().map(|s| (s.rms_arcmin * 100.0).round() / 100.0).collect::<Vec<_>>());
        (q1.worst_rms_arcmin, lens2)
    }

    /// Re-run the optimiser from textbook starting points and print the
    /// coefficients to paste into `ASPH`, `ACH_PC`, `ACH_G`.
    #[test]
    #[ignore]
    fn bake() {
        let o = Project::iris().design.optics;
        let o20 = OpticsParams { lens_focal_mm: DESIGN_F_MM, ..o.clone() };
        for p in presets() {
            run(&o20, &p.id, p.lens.clone(), p.variables.clone());
        }
        let v = |l: &[(usize, &str)]| l.iter().map(|(a, b)| var(*a, b)).collect::<Vec<_>>();
        let asv = v(&[(0, "curvature"), (0, "conic"), (0, "a4"), (0, "a6"), (1, "curvature"), (1, "a4"), (1, "a6")]);
        run(&o20, "asph from bent", vec![s("a", 15.0, 5.0, "PMMA"), s("b", -25.0, 15.0, "")], asv.clone());
        run(&o20, "asph from meniscus", vec![s("a", -30.0, 5.0, "PMMA"), s("b", -7.0, 15.0, "")], asv.clone());
        run(&o20, "asph flat-first", vec![s("a", 0.0, 5.0, "PMMA"), s("b", -9.84, 15.0, "")], asv.clone());
        let av = v(&[(0, "curvature"), (0, "conic"), (0, "a4"), (0, "a6"), (1, "curvature"), (2, "curvature"), (2, "a4"), (2, "a6")]);
        run(&o20, "ach PMMA bent", vec![s("a", 12.0, 7.0, "PMMA"), s("c", -8.0, 1.2, "PC"), s("d", -40.0, 12.0, "")], av.clone());
        run(&o20, "ach PC first", vec![s("a", 40.0, 1.2, "PC"), s("c", 9.0, 7.0, "PMMA"), s("d", -20.0, 12.0, "")], av.clone());
        run(&o20, "ach COP/PC", vec![s("a", 9.6, 7.0, "COP"), s("c", -9.6, 1.2, "PC"), s("d", -75.0, 12.0, "")], av.clone());
        run(&o20, "ach PMMA flatter", vec![s("a", 11.0, 7.0, "PMMA"), s("c", -11.0, 1.2, "PC"), s("d", -200.0, 12.0, "")], av.clone());
    }
}

#[cfg(test)]
mod dbg {
    #[test]
    #[ignore]
    fn show() {
        let o = crate::project::Project::iris().design.optics;
        for p in super::presets() {
            let l = super::preset_lens(&p.id, &o).unwrap();
            println!("{} efl {}", p.id, super::lens_efl(&l));
            for s in &l { println!("  R {} t {} sd {} {}", s.radius, s.thickness, s.semi_diameter, s.material); }
        }
    }
}
