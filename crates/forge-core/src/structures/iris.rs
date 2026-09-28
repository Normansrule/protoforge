//! The Iris structural verification: every load case on the real CAD parts,
//! the drop estimate, the PCB-in-pod stack-up, temple modes and hinge
//! fatigue — the numbers behind the `structures.*` metrics.

use serde::Serialize;

use super::fatigue::{self, Block, FatigueIn, FatigueOut};
use super::fea3d::{self, Precond, SolveOpts};
use super::hex8::UnitHex;
use super::materials;
use super::modal::{self, ModalOut};
use super::strength::{self, drop_analysis, DropResult, Model, Solved};
use super::tolerance::{self, PodStackIn, PodStackOut};
use crate::project::Project;

/// Resolution and solver settings for one analysis pass.
#[derive(Clone, Copy, Debug)]
pub struct Fidelity {
    pub h_temple: f64,
    pub h_front: f64,
    pub h_pod: f64,
    pub max_elements: usize,
    pub tol: f64,
}

impl Fidelity {
    /// Coarse settings used by `metrics()` (fast; partial-volume voxels keep
    /// the thin walls).
    pub const METRIC: Fidelity = Fidelity {
        h_temple: 1.5,
        h_front: 2.5,
        h_pod: 1.2,
        max_elements: 4000,
        tol: 1e-4,
    };
    /// Finer settings for the lab's on-demand run.
    pub const FINE: Fidelity = Fidelity {
        h_temple: 0.6,
        h_front: 1.0,
        h_pod: 0.6,
        max_elements: 30_000,
        tol: 1e-6,
    };
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CaseSummary {
    pub case: String,
    pub part: String,
    pub label: String,
    pub force_n: f64,
    pub h_mm: f64,
    pub elements: usize,
    pub iterations: usize,
    pub max_vm_mpa: f64,
    pub safety_factor: f64,
    pub sf_von_mises: f64,
    pub governing: String,
    /// Load-point displacement along the load, mm.
    pub deflection_mm: f64,
    pub stiffness_n_mm: f64,
    pub region: String,
    pub counts_for_min: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct IrisOut {
    pub material: String,
    pub material_name: String,
    pub printed: bool,
    pub cases: Vec<CaseSummary>,
    pub min_safety_factor: f64,
    pub worst_case: String,
    pub temple_deflection_mm: f64,
    pub drop: DropResult,
    pub pod: PodStackOut,
    pub modal: ModalOut,
    pub fatigue: FatigueOut,
    pub solid_fraction_note: String,
    pub notes: Vec<String>,
}

fn solve_one(model: Model, mat: &'static materials::StructMaterial, tol: f64) -> Solved {
    strength::solve_model(model, mat, Precond::Multigrid, true, tol, 400)
}

fn summary(s: &Solved, label: &str, counts: bool) -> CaseSummary {
    let sm = strength::summarize(s);
    let hint = strength::hint_for(s, &sm);
    CaseSummary {
        case: s.model.case.clone(),
        part: s.model.part.clone(),
        label: label.into(),
        force_n: s.model.total_force_n,
        h_mm: s.model.grid.h,
        elements: s.model.grid.count(),
        iterations: s.hist.len().saturating_sub(1),
        max_vm_mpa: sm.max_vm,
        safety_factor: sm.sf,
        sf_von_mises: sm.sf_vm,
        governing: if sm.across { "across layers".into() } else { "in-plane".into() },
        deflection_mm: sm.load_disp,
        stiffness_n_mm: if sm.load_disp > 0.0 { s.model.total_force_n / sm.load_disp } else { 0.0 },
        region: hint.region,
        counts_for_min: counts,
    }
}

/// Hinge usage spectrum for Miner's rule, scaled by the temple-spread
/// stress per newton.
pub fn hinge_spectrum(stress_per_n: f64) -> Vec<Block> {
    vec![
        Block {
            name: "Put on / take off (≈2.5 N spread)".into(),
            max_mpa: 2.5 * stress_per_n,
            min_mpa: 0.0,
            cycles_per_day: 10.0,
        },
        Block {
            name: "One-handed removal, wide spread (5 N)".into(),
            max_mpa: 5.0 * stress_per_n,
            min_mpa: 0.0,
            cycles_per_day: 2.0,
        },
        Block {
            name: "Walking bounce (±0.1 N, 1 h/day at 1.8 Hz)".into(),
            max_mpa: 0.1 * stress_per_n,
            min_mpa: -0.1 * stress_per_n,
            cycles_per_day: 6480.0,
        },
    ]
}

pub fn analyze(p: &Project, fid: Fidelity) -> Result<IrisOut, String> {
    let mat = materials::get(&p.design.frame.material);
    let cases = strength::cases();
    let force = |id: &str| cases.iter().find(|c| c.id == id).map(|c| c.default_force_n).unwrap_or(1.0);
    let mut out = IrisOut {
        material: mat.id.into(),
        material_name: mat.name.into(),
        printed: mat.printed,
        ..Default::default()
    };
    // ---- (a) temple spread (right, with the drop load on the same model) and (c) drop stiffness.
    let mr = strength::build_model(p, "temple_r", "temple_spread", fid.h_temple, fid.max_elements, force("temple_spread"))?;
    let drop_model = strength::build_model(p, "temple_r", "drop_side", fid.h_temple, fid.max_elements, 1.0)?;
    // Same grid and supports: solve both loads with one multigrid setup.
    let (sr, sd) = {
        let opts = SolveOpts {
            e_mpa: mat.e_mpa(),
            nu: mat.nu,
            incompatible: true,
            precond: Precond::Multigrid,
            tol: fid.tol,
            max_iter: 400,
        };
        let hex = UnitHex::new(mat.nu, true);
        let same = mr.grid.solid == drop_model.grid.solid && mr.bcs.fixed == drop_model.bcs.fixed;
        if same {
            let mut sols = fea3d::solve_many(&mr.grid, &mr.bcs.fixed, &[&mr.bcs.force, &drop_model.bcs.force], &opts, &hex);
            let b = sols.pop().unwrap();
            let a = sols.pop().unwrap();
            let pack = |m: Model, s: fea3d::Solved| {
                let st = fea3d::stresses(&m.grid, &hex, opts.e_mpa, &s.u, m.build_dir);
                Solved {
                    model: m,
                    u: s.u,
                    st,
                    hist: s.history,
                    converged: s.converged,
                    levels: s.levels,
                    free_dofs: s.free_dofs,
                    mat,
                }
            };
            (pack(mr, a), pack(drop_model, b))
        } else {
            (solve_one(mr, mat, fid.tol), solve_one(drop_model, mat, fid.tol))
        }
    };
    let spread_r = summary(&sr, "Temple spread — right (electronics pod)", true);
    // Left temple (battery pod, longer slim arm).
    let ml = strength::build_model(p, "temple_l", "temple_spread", fid.h_temple, fid.max_elements, force("temple_spread"))?;
    let sl = solve_one(ml, mat, fid.tol);
    let spread_l = summary(&sl, "Temple spread — left (battery pod)", true);
    // ---- (b) bridge bend.
    let mf = strength::build_model(p, "front", "bridge_bend", fid.h_front, fid.max_elements, force("bridge_bend"))?;
    let sf = solve_one(mf, mat, fid.tol);
    let bridge = summary(&sf, "Bridge bend", true);
    // ---- (d) pod squeeze.
    let mp = strength::build_model(p, "temple_r", "pod_squeeze", fid.h_pod, fid.max_elements, force("pod_squeeze"))?;
    let sp = solve_one(mp, mat, fid.tol);
    let squeeze = summary(&sp, "Pod squeeze", true);
    // ---- drop
    let dsum = strength::summarize(&sd);
    let dcase = summary(&sd, "Drop on the pod — unit load (1 N)", false);
    let m_eff = strength::moving_mass_g(p, "temple_r");
    // Unit-load model: stress per newton = max_vm, SF at F = SF(1 N) / F (linear).
    out.drop = drop_analysis(1.5, m_eff, dcase.stiffness_n_mm, dsum.max_vm, dsum.sf, if dsum.across { "across layers" } else { "in-plane" });
    // ---- worst over (a), (b), (d)
    let all = vec![spread_r.clone(), spread_l.clone(), bridge, squeeze, dcase];
    let worst = all
        .iter()
        .filter(|c| c.counts_for_min)
        .min_by(|a, b| a.safety_factor.total_cmp(&b.safety_factor))
        .cloned()
        .unwrap_or_default();
    out.min_safety_factor = worst.safety_factor;
    out.worst_case = format!("{} ({})", worst.label, worst.region);
    out.temple_deflection_mm = spread_r.deflection_mm.max(spread_l.deflection_mm);
    // ---- pod stack-up
    out.pod = tolerance::pod_stack(p, &PodStackIn::default())?;
    // ---- modes of the right temple (reuse the spread model's grid)
    let l = &sr.model.layout;
    let added: f64 = p.components.iter().filter(|c| c.location == "right_pod").map(|c| c.mass_g * c.qty).sum::<f64>()
        + crate::cad::frame_geom(p)
            .parts
            .iter()
            .filter(|b| b.name == "lid_r")
            .map(|b| b.assembly.volume().abs() * p.design.frame.density_g_cm3 / 1000.0)
            .sum::<f64>();
    out.modal = modal::temple_modal(&sr.model.grid, "temple_r", mat.e_mpa(), p.design.frame.density_g_cm3, l.pod_front_y - 1.5, l.pod_rear_y, added);
    // ---- hinge fatigue from the worse temple's stress per newton
    let worse = if spread_l.safety_factor < spread_r.safety_factor { &spread_l } else { &spread_r };
    let per_n = worse.max_vm_mpa / worse.force_n.max(1e-9);
    let sfac = if worse.governing == "across layers" { mat.z_strength_factor } else { 1.0 };
    out.fatigue = fatigue::analyze(FatigueIn {
        material: mat.id.into(),
        years: 3.0,
        blocks: hinge_spectrum(per_n),
        strength_factor: sfac,
    })?;
    // ---- print solidity note
    let pr = &p.design.print;
    let shell = 2.0 * pr.perimeters as f64 * pr.line_width_mm;
    let tw = p.design.frame.temple_width_mm;
    out.solid_fraction_note = if mat.printed {
        if shell >= tw {
            format!("With {} perimeters × {:.2} mm the {:.1} mm temple prints fully solid — the solid-material assumption holds.", pr.perimeters, pr.line_width_mm, tw)
        } else {
            let core = tw - shell;
            let i_loss = core.powi(3) / tw.powi(3) * (1.0 - pr.infill_pct / 100.0);
            format!(
                "{} perimeters × {:.2} mm leave a {:.2} mm infilled core ({:.0} %) in the {:.1} mm temple: bending stiffness is ≈ {:.1} % below the solid-part result.",
                pr.perimeters,
                pr.line_width_mm,
                core,
                pr.infill_pct,
                tw,
                100.0 * i_loss
            )
        }
    } else {
        String::new()
    };
    out.notes = vec![
        format!(
            "Voxel sizes: temples {:.2} mm, front {:.2} mm, pod {:.2} mm; incompatible-mode bricks, multigrid-preconditioned CG to a relative residual of {:.0e}.",
            fid.h_temple, fid.h_front, fid.h_pod, fid.tol
        ),
        "Safety factors are on yield with the print-layer knock-down; sharp CAD corners are stress raisers — the voxel peak there is indicative.".into(),
    ];
    out.cases = all;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_metric_pass_is_sane() {
        let p = Project::iris();
        let o = analyze(&p, Fidelity::METRIC).unwrap();
        for c in &o.cases {
            assert!(c.safety_factor.is_finite() && c.safety_factor > 0.0, "{c:?}");
            assert!(c.deflection_mm > 0.0, "{c:?}");
        }
        assert!(o.temple_deflection_mm > 1.0 && o.temple_deflection_mm < 60.0, "{}", o.temple_deflection_mm);
        assert!(o.drop.peak_force_n > 10.0);
        assert!(o.modal.first_mode_hz > 5.0 && o.modal.first_mode_hz < 2000.0, "{}", o.modal.first_mode_hz);
        assert!(o.pod.stack.worst_min.is_finite());
    }
}
