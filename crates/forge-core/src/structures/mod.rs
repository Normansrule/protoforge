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
//! | [`calc`] | beams, snap fits, bosses/inserts, bolted joints, press fits, buckling, living hinges |
//! | [`fatigue`] | S-N (Basquin), Goodman, Miner |
//! | [`tolerance`] | stack-ups (worst case, RSS, Monte Carlo), ISO 286 fits, the PCB-in-pod stack |
//! | [`modal`] | beam modes of the temples |
//!
//! Metrics (see [`metrics`]): `structures.min_safety_factor`,
//! `structures.temple_deflection_mm`, `structures.drop_safety_factor`,
//! `structures.pod_fit_worst_mm`, plus informational
//! `structures.first_mode_hz`, `structures.pod_fit_yield_pct`,
//! `structures.hinge_fatigue_life_years` and `structures.temple_stiffness_n_mm`.

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

fn fidelity(s: &str) -> iris::Fidelity {
    if s == "fine" {
        iris::Fidelity::FINE
    } else {
        iris::Fidelity::METRIC
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
            iris::analyze(&p, fidelity(&i.fidelity))
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
            iris::analyze(&p, fidelity(&i.fidelity)).map(|o| o.modal)
        }),
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
    let f = strength::cases().into_iter().find(|c| c.id == i.case).map(|c| c.default_force_n).unwrap_or(1.0);
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
    let o = iris::analyze(&p, iris::Fidelity::METRIC)?;
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
        strength_factor: if worse.governing == "across layers" { mat.z_strength_factor } else { 1.0 },
    })
}

thread_local! {
    static METRICS: crate::memo::Slot<Vec<Metric>> = const { std::cell::RefCell::new(None) };
}

/// Metrics this module contributes to requirement verification (memoised on
/// the frame, PCB, print settings and component masses).
pub fn metrics(p: &Project) -> Vec<Metric> {
    let comps: Vec<(&str, f64, f64)> = p.components.iter().map(|c| (c.location.as_str(), c.mass_g, c.qty)).collect();
    let key = crate::memo::key(&(&p.design.frame, &p.design.optics, &p.design.pcb, &p.design.print, comps));
    crate::memo::memo(&METRICS, key, || metrics_uncached(p))
}

fn metrics_uncached(p: &Project) -> Vec<Metric> {
    let o = match iris::analyze(p, iris::Fidelity::METRIC) {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    let src = "structures";
    let mut m = vec![
        Metric::new("structures.min_safety_factor", "Minimum structural safety factor", o.min_safety_factor, "x", src).with_note(&format!(
            "worst of temple spread, bridge bend and pod squeeze: {} — {} vs yield{} (coarse voxel model)",
            o.worst_case,
            o.material_name,
            if o.printed { ", print-layer knock-down applied" } else { "" }
        )),
        Metric::new("structures.temple_deflection_mm", "Temple tip deflection (5 N spread)", o.temple_deflection_mm, "mm", src)
            .with_note("larger of the two temples; hinge end clamped, 5 N outward at the tip"),
        Metric::new("structures.drop_safety_factor", "Drop safety factor (1.5 m onto the pod)", o.drop.safety_factor, "x", src).with_note(&format!(
            "1-DOF energy method: {:.0} N peak on a {:.1} N/mm temple, {:.0} g moving mass — an upper bound on force",
            o.drop.peak_force_n, o.drop.stiffness_n_mm, o.drop.moving_mass_g
        )),
        Metric::new("structures.pod_fit_worst_mm", "PCB-in-pod worst-case clearance", o.pod.stack.worst_min, "mm", src).with_note(&format!(
            "stack-up: pod {:.2} ± {:.2} mm ({}) − PCB {:.2} mm − edge overhang, incl. cold (−10 °C) shrink; RSS −3σ {:.2} mm",
            o.pod.pod_inner_mm,
            o.pod.dims.first().map(|d| d.plus).unwrap_or(0.0),
            o.pod.process,
            o.pod.pcb_len_mm,
            o.pod.stack.rss_min
        )),
        Metric::new("structures.pod_fit_yield_pct", "PCB-in-pod assembly yield (Monte Carlo)", o.pod.stack.yield_pct, "%", src),
        Metric::new("structures.first_mode_hz", "First bending mode of the right temple", o.modal.first_mode_hz, "Hz", src)
            .with_note("Euler–Bernoulli beam from voxel slices, clamped at the hinge, electronics as added mass"),
    ];
    if let Some(c) = o.cases.iter().find(|c| c.case == "temple_spread" && c.part == "temple_r") {
        m.push(Metric::new("structures.temple_stiffness_n_mm", "Right temple lateral stiffness", c.stiffness_n_mm, "N/mm", src));
    }
    if o.fatigue.has_data {
        m.push(
            Metric::new("structures.hinge_fatigue_life_years", "Temple fatigue life (Miner, usage spectrum)", o.fatigue.life_years.min(1e6), "years", src)
                .with_note("10 put-on/take-off + 2 wide spreads per day + walking; Basquin + Goodman"),
        );
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_present_for_iris() {
        let m = metrics(&Project::iris());
        for k in [
            "structures.min_safety_factor",
            "structures.temple_deflection_mm",
            "structures.drop_safety_factor",
            "structures.pod_fit_worst_mm",
            "structures.first_mode_hz",
        ] {
            let x = m.iter().find(|x| x.key == k).unwrap_or_else(|| panic!("missing {k}"));
            assert!(x.value.is_finite(), "{k} = {}", x.value);
        }
        let u = |k: &str| m.iter().find(|x| x.key == k).unwrap().unit.clone();
        assert_eq!(u("structures.min_safety_factor"), "x");
        assert_eq!(u("structures.temple_deflection_mm"), "mm");
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
        ] {
            let r = dispatch(mth, &inp).unwrap();
            assert!(r.is_ok(), "{mth}: {r:?}");
        }
        assert!(dispatch("structures.nope", &json!({})).is_none());
    }
}
