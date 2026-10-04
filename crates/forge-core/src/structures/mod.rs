//! # Structures — strength, stiffness, fasteners, fatigue and tolerances
//!
//! | file | what |
//! |---|---|
//! | [`materials`] | structural materials database (printed values flagged, Z knock-down) |
//! | [`hex8`] | 8-node brick element with Wilson–Taylor incompatible modes |
//! | [`voxel`] | ray-cast, partial-volume voxelisation of CAD meshes |
//! | [`fea3d`] | matrix-free PCG with Jacobi or geometric-multigrid preconditioning |
//! | [`strength`] | Iris load cases, stress/safety-factor post-processing, drop model |
//! | [`iris`] | the full Iris verification behind the metrics |
//! | [`topo`] | 2D plane-stress solver and SIMP topology optimisation |
//! | [`calc`] | beams, snap fits, bosses/inserts, bolted joints and thread stripping, press fits, buckling, living hinges, thermal mismatch, stress concentrations |
//! | [`fatigue`] | S-N (Basquin), Goodman, Miner |
//! | [`tolerance`] | stack-ups (worst case, RSS, Monte Carlo), ISO 286 fits, the PCB-in-pod and display-focus stacks |
//! | [`modal`] | beam modes of the temples |
//!
//! Metrics (see [`metrics`]): `structures.min_safety_factor` (recommend ≥ 2)
//! and `structures.pod_fit_worst_mm` (recommend ≥ 0), plus informational
//! `structures.temple_deflection_mm`, `structures.temple_stiffness_n_mm`,
//! `structures.drop_safety_factor`, `structures.pod_fit_yield_pct`,
//! `structures.first_mode_hz`, `structures.hinge_fatigue_cycles` and
//! `structures.focus_error_worst_d`.

#![allow(clippy::needless_range_loop)] // index-heavy numeric code reads better with explicit indices

use serde::Deserialize;
use serde_json::{json, Value};

use crate::api::{handle, ApiResult};
use crate::project::{Metric, Project};

pub mod calc;
pub mod fatigue;
pub mod fea3d;
pub mod hex8;
pub mod iris;
pub mod materials;
pub mod modal;
pub mod strength;
pub mod tolerance;
pub mod topo;
pub mod voxel;

#[derive(Deserialize, Default)]
#[serde(default)]
struct ProjIn {
    project: Option<Project>,
    /// "metric" (coarse, fast) or "fine".
    fidelity: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct PodIn {
    project: Option<Project>,
    #[serde(flatten)]
    opts: tolerance::PodStackIn,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct FocusIn {
    project: Option<Project>,
    #[serde(flatten)]
    opts: tolerance::FocusStackIn,
}

#[derive(Deserialize)]
#[serde(default)]
struct DropIn {
    project: Option<Project>,
    height_m: f64,
    part: String,
    material: Option<String>,
    voxel_mm: f64,
    /// Override the moving mass, g.
    mass_g: Option<f64>,
}

impl Default for DropIn {
    fn default() -> Self {
        DropIn {
            project: None,
            height_m: 1.5,
            part: "temple_r".into(),
            material: None,
            voxel_mm: 1.5,
            mass_g: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct StudyIn {
    project: Option<Project>,
    part: String,
    case: String,
    material: Option<String>,
    sizes: Vec<f64>,
}

impl Default for StudyIn {
    fn default() -> Self {
        StudyIn {
            project: None,
            part: "temple_r".into(),
            case: "temple_spread".into(),
            material: None,
            sizes: vec![2.0, 1.5, 1.2, 1.0, 0.8],
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct LockIn {
    /// Elements through the thickness.
    layers: usize,
    element: String,
}

impl Default for LockIn {
    fn default() -> Self {
        LockIn {
            layers: 2,
            element: "h8i".into(),
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct IrisFatigueIn {
    project: Option<Project>,
    years: Option<f64>,
    blocks: Option<Vec<fatigue::Block>>,
    material: Option<String>,
}

fn fidelity(s: &str, p: &Project) -> iris::Fidelity {
    if s == "fine" {
        iris::Fidelity::fine(p)
    } else {
        iris::Fidelity::metric(p)
    }
}

/// JSON API entry point for `structures.*` methods.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "structures.materials" => Some(crate::api::to_value(materials::MATERIALS)),
        "structures.cases" => Some(crate::api::to_value(strength::cases())),
        "structures.fea" => handle(input, strength::run),
        "structures.iris" => handle(input, |i: ProjIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            iris::analyze(&p, fidelity(&i.fidelity, &p))
        }),
        "structures.drop" => handle(input, drop),
        "structures.mesh_study" => handle(input, mesh_study),
        "structures.locking_demo" => handle(input, locking_demo),
        "structures.topo" => handle(input, topo::optimize),
        "structures.topo_presets" => Some(crate::api::to_value(topo::presets())),
        "structures.plane_stress" => handle(input, topo::plane_stress),
        "structures.beam" => handle(input, calc::beam),
        "structures.snap_fit" => handle(input, calc::snap_fit),
        "structures.boss" => handle(input, calc::boss),
        "structures.bolt" => handle(input, calc::bolt),
        "structures.press_fit" => handle(input, calc::press_fit),
        "structures.buckling" => handle(input, calc::buckling),
        "structures.living_hinge" => handle(input, calc::living_hinge),
        "structures.fatigue" => handle(input, fatigue::analyze),
        "structures.iris_fatigue" => handle(input, iris_fatigue),
        "structures.stackup" => handle(input, tolerance::stackup),
        "structures.fit" => handle(input, tolerance::fit),
        "structures.fits" => Some(Ok(json!(tolerance::common_fits()
            .iter()
            .map(|(h, s, n)| json!({"hole": h, "shaft": s, "name": n}))
            .collect::<Vec<_>>()))),
        "structures.pod_stack" => handle(input, |i: PodIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            tolerance::pod_stack(&p, &i.opts)
        }),
        "structures.modal" => handle(input, |i: ProjIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            iris::analyze(&p, fidelity(&i.fidelity, &p)).map(|o| o.modal)
        }),
        "structures.focus_stack" => handle(input, |i: FocusIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            let mut opts = i.opts;
            // the project's focus design (e.g. the v0.5 adjustable focus cell) applies unless the caller overrides it
            if opts.adjust_step_mm.is_none() {
                if let Some(proj) = p
                    .design
                    .extra
                    .get("focus_stack")
                    .and_then(|v| serde_json::from_value::<tolerance::FocusStackIn>(v.clone()).ok())
                {
                    opts.adjust_step_mm = proj.adjust_step_mm;
                    opts.lock_shift_mm = proj.lock_shift_mm;
                }
            }
            tolerance::focus_stack(&p, &opts)
        }),
        "structures.thermal" => handle(input, calc::thermal),
        "structures.stress_conc" => handle(input, calc::stress_conc),
        "structures.beam_modes" => handle(input, beam_modes),
        _ => None,
    }
}

fn drop(i: DropIn) -> Result<strength::DropResult, String> {
    let p = i.project.unwrap_or_else(Project::iris);
    let mat = materials::get(i.material.as_deref().unwrap_or(&p.design.frame.material));
    let model = strength::build_model(&p, &i.part, "drop_side", i.voxel_mm.max(0.4), 20_000, 1.0)?;
    let s = strength::solve_model(model, mat, fea3d::Precond::Multigrid, true, 1e-5, 800);
    let sm = strength::summarize(&s);
    let k = if sm.load_disp > 0.0 { 1.0 / sm.load_disp } else { 0.0 };
    let m = i.mass_g.unwrap_or_else(|| strength::moving_mass_g(&p, &i.part));
    Ok(strength::drop_analysis(
        i.height_m,
        m,
        k,
        sm.max_vm,
        sm.sf,
        if sm.across { "across layers" } else { "in-plane" },
    ))
}

#[derive(serde::Serialize)]
struct StudyRow {
    h_mm: f64,
    elements: usize,
    iterations: usize,
    max_vm_mpa: f64,
    peak_vm_mpa: f64,
    safety_factor: f64,
    deflection_mm: f64,
    volume_mm3: f64,
}

/// Mesh-convergence study: the same case at several voxel sizes.
fn mesh_study(i: StudyIn) -> Result<Vec<StudyRow>, String> {
    let p = i.project.unwrap_or_else(Project::iris);
    let mat = materials::get(i.material.as_deref().unwrap_or(&p.design.frame.material));
    let f = strength::cases()
        .into_iter()
        .find(|c| c.id == i.case)
        .map(|c| c.default_force_n)
        .unwrap_or(1.0);
    let mut rows = Vec::new();
    for &h in i.sizes.iter().take(8) {
        let m = strength::build_model(&p, &i.part, &i.case, h.max(0.4), 40_000, f)?;
        let s = strength::solve_model(m, mat, fea3d::Precond::Multigrid, true, 1e-6, 800);
        let sm = strength::summarize(&s);
        rows.push(StudyRow {
            h_mm: s.model.grid.h,
            elements: s.model.grid.count(),
            iterations: s.hist.len().saturating_sub(1),
            max_vm_mpa: sm.max_vm,
            peak_vm_mpa: sm.peak_vm,
            safety_factor: sm.sf,
            deflection_mm: sm.load_disp,
            volume_mm3: s.model.grid.volume(),
        });
    }
    Ok(rows)
}

#[derive(serde::Serialize)]
struct LockOut {
    layers: usize,
    element: String,
    fea_tip_mm: f64,
    beam_tip_mm: f64,
    timoshenko_tip_mm: f64,
    error_pct: f64,
    iterations: usize,
    history: Vec<f64>,
    /// Deformed centre line (x, w) for drawing.
    x: Vec<f64>,
    w: Vec<f64>,
}

/// Cantilever 40 × 4 × 4 mm (E = 2 GPa, ν = 0.3, 10 N tip load) with
/// `layers` voxels through the thickness: shows shear locking of the plain
/// brick and the fix by incompatible modes.
fn locking_demo(i: LockIn) -> Result<LockOut, String> {
    let layers = i.layers.clamp(1, 8);
    let (l, b, t) = (40.0, 4.0, 4.0);
    let h = t / layers as f64;
    let (nx, ny, nz) = ((l / h).round() as usize, layers, layers);
    let mut g = fea3d::Grid::new(nx, ny, nz, h, [0.0; 3]);
    g.fill_all();
    let mut bcs = fea3d::Bcs::new(&g);
    for k in 0..=nz {
        for j in 0..=ny {
            bcs.fixed[g.nid(0, j, k)] = 7;
        }
    }
    let nface = ((ny + 1) * (nz + 1)) as f64;
    for k in 0..=nz {
        for j in 0..=ny {
            bcs.force[3 * g.nid(nx, j, k) + 2] = -10.0 / nface;
        }
    }
    let inc = i.element != "h8";
    let opts = fea3d::SolveOpts {
        e_mpa: 2000.0,
        nu: 0.3,
        incompatible: inc,
        precond: fea3d::Precond::Multigrid,
        tol: 1e-8,
        max_iter: 2000,
    };
    let hex = hex8::UnitHex::new(0.3, inc);
    let s = fea3d::solve(&g, &bcs, &opts, &hex);
    let mid = |ii: usize| -> f64 {
        let mut acc = 0.0;
        for k in 0..=nz {
            for j in 0..=ny {
                acc += s.u[3 * g.nid(ii, j, k) + 2];
            }
        }
        -acc / nface
    };
    let tip = mid(nx);
    let ii = b * t * t * t / 12.0;
    let eb = 10.0 * l * l * l / (3.0 * 2000.0 * ii);
    let gm = 2000.0 / 2.6;
    let tim = eb + 10.0 * l / (5.0 / 6.0 * gm * b * t);
    Ok(LockOut {
        layers,
        element: if inc { "h8i".into() } else { "h8".into() },
        fea_tip_mm: tip,
        beam_tip_mm: eb,
        timoshenko_tip_mm: tim,
        error_pct: 100.0 * (tip - tim) / tim,
        iterations: s.history.len().saturating_sub(1),
        history: s.history,
        x: (0..=nx).map(|q| q as f64 * h).collect(),
        w: (0..=nx).map(mid).collect(),
    })
}

fn iris_fatigue(i: IrisFatigueIn) -> Result<fatigue::FatigueOut, String> {
    let p = i.project.unwrap_or_else(Project::iris);
    let o = iris::analyze(&p, iris::Fidelity::metric(&p))?;
    if i.blocks.is_none() && i.years.is_none() && i.material.is_none() {
        return Ok(o.fatigue);
    }
    let worse = o
        .cases
        .iter()
        .filter(|c| c.case == "temple_spread")
        .min_by(|a, b| a.safety_factor.total_cmp(&b.safety_factor))
        .ok_or("no temple case")?;
    let per_n = worse.max_vm_mpa / worse.force_n.max(1e-9);
    let mat = materials::get(i.material.as_deref().unwrap_or(&p.design.frame.material));
    fatigue::analyze(fatigue::FatigueIn {
        material: mat.id.into(),
        years: i.years.unwrap_or(3.0),
        blocks: i.blocks.unwrap_or_else(|| iris::hinge_spectrum(per_n)),
        strength_factor: if worse.governing == "across layers" {
            mat.z_strength_factor
        } else {
            1.0
        },
    })
}

#[derive(Deserialize)]
#[serde(default)]
struct BeamModesIn {
    length_mm: f64,
    section: calc::Section,
    material: String,
    /// Tip mass (e.g. an ear piece or a pod), g.
    tip_mass_g: f64,
    /// "cantilever" (clamped–free) or "simply_supported"-like free–free is not
    /// supported; clamped–free only.
    segments: usize,
}

impl Default for BeamModesIn {
    fn default() -> Self {
        BeamModesIn {
            length_mm: 80.0,
            section: calc::Section {
                shape: "rect".into(),
                b: 8.0,
                h: 4.0,
                t: 0.0,
            },
            material: "PETG".into(),
            tip_mass_g: 0.0,
            segments: 24,
        }
    }
}

#[derive(serde::Serialize)]
struct BeamModesOut {
    freqs_hz: Vec<f64>,
    /// Analytical first three modes for a uniform cantilever without tip mass.
    analytic_hz: Vec<f64>,
    stations_mm: Vec<f64>,
    shapes: Vec<Vec<f64>>,
    ei_nmm2: f64,
    mass_g: f64,
}

/// Natural frequencies of a uniform clamped–free beam (Hermite beam
/// elements, consistent mass) with an optional tip mass, next to the
/// analytical β_n L = 1.8751, 4.6941, 7.8548.
fn beam_modes(i: BeamModesIn) -> Result<BeamModesOut, String> {
    let sp = calc::section_props(&i.section)?;
    let m = materials::get(&i.material);
    let l = i.length_mm;
    if l.is_nan() || l <= 0.0 {
        return Err("length must be positive".into());
    }
    let n = i.segments.clamp(4, 60);
    let ei = m.e_mpa() * sp.i_mm4; // N·mm²
    let mu = sp.area_mm2 * m.density_g_cm3 / 1000.0; // g/mm = kg/m
    let mut segs: Vec<modal::Seg> = (0..n)
        .map(|_| modal::Seg {
            len: l / n as f64 / 1000.0,
            ei: ei * 1e-6,
            m: mu,
        })
        .collect();
    // Tip mass lumped into the last segment's distributed mass (over its length).
    if i.tip_mass_g > 0.0 {
        if let Some(last) = segs.last_mut() {
            last.m += i.tip_mass_g / 1000.0 / last.len;
        }
    }
    let (f, shapes) = modal::cantilever_modes(&segs, 3);
    let base = (ei * 1e-6 / (mu * (l / 1000.0).powi(4))).sqrt() / (2.0 * std::f64::consts::PI);
    Ok(BeamModesOut {
        freqs_hz: f,
        analytic_hz: [1.875_104, 4.694_091, 7.854_757].iter().map(|b: &f64| b * b * base).collect(),
        stations_mm: (0..=n).map(|k| l * k as f64 / n as f64).collect(),
        shapes,
        ei_nmm2: ei,
        mass_g: mu * l + i.tip_mass_g,
    })
}

thread_local! {
    static METRICS: crate::memo::Slot<Vec<Metric>> = const { std::cell::RefCell::new(None) };
}

/// Metrics this module contributes to requirement verification. The finite
/// element solves are memoised inside [`iris::analyze`] on the frame, optics
/// and PCB; the whole list is memoised on everything it reads.
pub fn metrics(p: &Project) -> Vec<Metric> {
    let comps: Vec<(&str, f64, f64)> = p.components.iter().map(|c| (c.location.as_str(), c.mass_g, c.qty)).collect();
    let key = crate::memo::key(&(
        &p.design.frame,
        &p.design.optics,
        &p.design.pcb,
        &p.design.print,
        &p.design.manufacturing.molding_material,
        comps,
        p.design.extra.get("focus_stack"),
    ));
    crate::memo::memo(&METRICS, key, || metrics_uncached(p))
}

fn metrics_uncached(p: &Project) -> Vec<Metric> {
    let fid = iris::Fidelity::metric(p);
    let o = match iris::analyze(p, fid) {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    let src = "structures";
    let peak = o
        .cases
        .iter()
        .filter(|c| c.counts_for_min)
        .map(|c| c.peak_vm_mpa)
        .fold(0.0, f64::max);
    let mut m = vec![
        Metric::new("structures.min_safety_factor", "Minimum structural safety factor", o.min_safety_factor, "x", src).with_note(&format!(
            "worst of temple spread (5 N, both temples) and bridge bend (10 N): {} — {} yield {:.0} MPa vs design von Mises on {:.1} mm voxels (sharp-corner zones excluded){}; sharp-corner nodal peak {:.0} MPa (stress concentration — fillet it)",
            o.worst_case,
            o.material_name,
            o.yield_mpa,
            fid.h_temple,
            if o.printed { ", print-layer check included" } else { "" },
            peak
        )),
        Metric::new("structures.pod_fit_worst_mm", "PCB-in-pod worst-case clearance", o.pod.stack.worst_min, "mm", src).with_note(&format!(
            "stack-up: pod {:.2} ± {:.2} mm ({}) − PCB {:.2} mm − edge overhang, incl. cold (−10 °C) shrink; RSS −3σ {:.2} mm",
            o.pod.pod_inner_mm,
            o.pod.dims.first().map(|d| d.plus).unwrap_or(0.0),
            o.pod.process,
            o.pod.pcb_len_mm,
            o.pod.stack.rss_min
        )),
        Metric::new("structures.temple_deflection_mm", "Temple tip deflection (5 N spread)", o.temple_deflection_mm, "mm", src)
            .with_note("larger of the two temples; hinge end clamped, 5 N outward at the tip (coarse voxels read slightly stiff)"),
        Metric::new("structures.drop_safety_factor", "Drop safety factor (1.5 m onto the pod)", o.drop.safety_factor, "x", src).with_note(&format!(
            "1-DOF energy method, rigid floor: {:.0} N peak on a {:.0} N/mm pod, {:.0} g moving mass — an upper bound on force (a physical drop test decides)",
            o.drop.peak_force_n, o.drop.stiffness_n_mm, o.drop.moving_mass_g
        )),
        Metric::new("structures.pod_fit_yield_pct", "PCB-in-pod assembly yield (Monte Carlo)", o.pod.stack.yield_pct, "%", src),
        Metric::new("structures.first_mode_hz", "First bending mode of the right temple", o.modal.first_mode_hz, "Hz", src)
            .with_note("Euler–Bernoulli beam from voxel slices, clamped at the hinge, electronics as added mass"),
        Metric::new("structures.focus_error_worst_d", "Display focus error, worst case", o.focus.worst_diopters, "D", src).with_note(&format!(
            "display-to-lens chain ±{:.3} mm worst case × {:.2} D/mm; RSS {:.2} D{}; allowance ±{:.2} D",
            0.5 * (o.focus.stack.worst_max - o.focus.stack.worst_min),
            o.focus.sensitivity_d_per_mm,
            o.focus.rss_diopters,
            if o.focus.stack.rss_capped {
                format!(
                    " (= worst case: {}, σ = half-range/√3, so ±3σ would lie outside {} own limits and is capped there)",
                    if o.focus.dims.len() == 1 { "one uniform contributor".to_string() } else { format!("{} contributors", o.focus.dims.len()) },
                    if o.focus.dims.len() == 1 { "its" } else { "the parts'" }
                )
            } else {
                String::new()
            },
            o.focus.allowable_d
        )),
    ];
    if let Some(c) = o.cases.iter().find(|c| c.case == "temple_spread" && c.part == "temple_r") {
        m.push(Metric::new(
            "structures.temple_stiffness_n_mm",
            "Right temple lateral stiffness",
            c.stiffness_n_mm,
            "N/mm",
            src,
        ));
    }
    if o.fatigue.has_data {
        m.push(
            Metric::new(
                "structures.hinge_fatigue_cycles",
                "Temple open/close cycles to fatigue failure",
                o.hinge_cycles.min(1e12),
                "cycles",
                src,
            )
            .with_note(&hinge_fatigue_note(&o.fatigue)),
        );
    }
    m
}

/// `x` as "1.3 × 10⁷".
fn sci(x: f64) -> String {
    if !x.is_finite() {
        return "∞".into();
    }
    if x.abs() < 1e4 {
        return format!("{x:.0}");
    }
    let e = x.abs().log10().floor() as i32;
    let sup: String = e
        .to_string()
        .chars()
        .map(|c| match c {
            '-' => '⁻',
            d => ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'][d.to_digit(10).unwrap_or(0) as usize],
        })
        .collect();
    format!("{:.1} × 10{sup}", x / 10f64.powi(e))
}

/// Note for `structures.hinge_fatigue_cycles`: what is analysed (the temple
/// plastic, not the hinge spring), which load block governs and whether its
/// life is inside the material's S-N data or extrapolated — so it reads next
/// to `mechanisms.hinge_life_cycles`, whose 10⁷ is a spring-wire run-out cap.
pub fn hinge_fatigue_note(f: &fatigue::FatigueOut) -> String {
    let gov = f.blocks.iter().max_by(|a, b| a.damage.total_cmp(&b.damage));
    let data_n = f.fatigue_point[0];
    let governing = match gov {
        Some(b) if b.damage > 0.0 && b.cycles_to_failure.is_finite() => format!(
            "the damage comes from \"{}\" (N_f {} of those, {} the material's S-N data, quoted to {} cycles), so this is a computed life, not a run-out cap",
            b.name,
            sci(b.cycles_to_failure),
            if b.cycles_to_failure <= data_n { "within" } else { "Basquin-extrapolated beyond" },
            sci(data_n)
        ),
        _ => "no block causes damage".into(),
    };
    format!(
        "temple plastic ({}) at the hinge under temple-spread loads — not the hinge spring, which is mechanisms.hinge_life_cycles (spring wire, reported as 10⁷ at run-out); Miner's rule on the usage spectrum (12 open/close per day + walking), Basquin + Goodman on the design stress (no notch factor; polymers have no endurance limit, so no run-out cap); {governing}: {} open/close cycles ≈ {:.0} years at 12 a day",
        f.material,
        sci(f.life_years.min(1e9) * 365.0 * iris::HINGE_CYCLES_PER_DAY),
        f.life_years.min(1e9)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The temple-fatigue note says what it analyses (the plastic, not the
    /// hinge spring) and why it is not capped like the spring's run-out.
    #[test]
    fn hinge_fatigue_note_explains_itself() {
        assert_eq!(sci(1.322e7), "1.3 × 10⁷");
        assert_eq!(sci(2.2e6), "2.2 × 10⁶");
        let p = Project::iris();
        let o = iris::analyze(&p, iris::Fidelity::metric(&p)).unwrap();
        let n = hinge_fatigue_note(&o.fatigue);
        assert!(
            n.contains("temple plastic") && n.contains("not the hinge spring") && n.contains("mechanisms.hinge_life_cycles"),
            "{n}"
        );
        // Iris: the 5 N wide spreads govern, with a life inside the 10⁷-cycle S-N data
        let gov = o.fatigue.blocks.iter().max_by(|a, b| a.damage.total_cmp(&b.damage)).unwrap();
        assert!(gov.name.contains("wide spread") && gov.cycles_to_failure < o.fatigue.fatigue_point[0]);
        assert!(
            n.contains("within the material's S-N data, quoted to 1.0 × 10⁷ cycles") && n.contains("not a run-out cap"),
            "{n}"
        );
        // open/close cycles = life in years × 365 × 12
        assert!((o.hinge_cycles - o.fatigue.life_years * 365.0 * 12.0).abs() < 1e-6 * o.hinge_cycles);
        assert!(n.contains(&sci(o.hinge_cycles)), "{n}");
    }

    #[test]
    fn metrics_present_for_iris() {
        let m = metrics(&Project::iris());
        for k in [
            "structures.min_safety_factor",
            "structures.pod_fit_worst_mm",
            "structures.temple_deflection_mm",
            "structures.drop_safety_factor",
            "structures.first_mode_hz",
            "structures.focus_error_worst_d",
            "structures.hinge_fatigue_cycles",
        ] {
            let x = m.iter().find(|x| x.key == k).unwrap_or_else(|| panic!("missing {k}"));
            assert!(x.value.is_finite(), "{k} = {}", x.value);
        }
        let u = |k: &str| m.iter().find(|x| x.key == k).unwrap().unit.clone();
        assert_eq!(u("structures.min_safety_factor"), "x");
        assert_eq!(u("structures.temple_deflection_mm"), "mm");
        // Recommended requirements: SF ≥ 2 and a non-negative worst-case fit.
        let v = |k: &str| m.iter().find(|x| x.key == k).unwrap().value;
        assert!(v("structures.min_safety_factor") > 1.0, "{}", v("structures.min_safety_factor"));
        assert!(v("structures.pod_fit_worst_mm") > 0.0);
        // Warm call is served from the memo.
        let t = std::time::Instant::now();
        let again = metrics(&Project::iris());
        assert_eq!(again.len(), m.len());
        assert!(t.elapsed().as_millis() < 200);
    }

    #[test]
    fn dispatch_calculators() {
        for (mth, inp) in [
            ("structures.beam", json!({})),
            ("structures.snap_fit", json!({})),
            ("structures.boss", json!({})),
            ("structures.bolt", json!({})),
            ("structures.press_fit", json!({})),
            ("structures.buckling", json!({})),
            ("structures.living_hinge", json!({})),
            ("structures.fatigue", json!({})),
            ("structures.fit", json!({})),
            ("structures.fits", json!({})),
            ("structures.materials", json!({})),
            ("structures.cases", json!({})),
            ("structures.topo_presets", json!({})),
            ("structures.plane_stress", json!({})),
            ("structures.stackup", json!({"dims": [{"name": "a", "nominal": 1.0}]})),
            ("structures.locking_demo", json!({"layers": 2})),
            ("structures.thermal", json!({})),
            ("structures.stress_conc", json!({})),
            ("structures.beam_modes", json!({"tip_mass_g": 2.0})),
            ("structures.focus_stack", json!({})),
        ] {
            let r = dispatch(mth, &inp).unwrap();
            assert!(r.is_ok(), "{mth}: {r:?}");
        }
        assert!(dispatch("structures.nope", &json!({})).is_none());
    }

    #[test]
    fn beam_modes_match_the_analytic_cantilever() {
        let r = beam_modes(BeamModesIn::default()).unwrap();
        for k in 0..3 {
            let rel = (r.freqs_hz[k] - r.analytic_hz[k]).abs() / r.analytic_hz[k];
            assert!(rel < 2e-3, "mode {k}: {} vs {}", r.freqs_hz[k], r.analytic_hz[k]);
        }
        let with_mass = beam_modes(BeamModesIn {
            tip_mass_g: 5.0,
            ..Default::default()
        })
        .unwrap();
        assert!(with_mass.freqs_hz[0] < r.freqs_hz[0]);
    }
}
