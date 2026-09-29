//! The Iris structural verification: every load case on the real CAD parts,
//! the drop estimate, the PCB-in-pod and display-focus stack-ups, temple
//! modes and hinge fatigue — the numbers behind the `structures.*` metrics.
//!
//! ## Resolution rule (metrics)
//! The metric pass must stay fast, so it uses the coarsest grid that still
//! bends correctly: **two voxels through the thinnest bending member**. The
//! incompatible-mode brick ([`super::hex8`]) is accurate in bending with two
//! layers (see `structures.locking_demo`), and the grid is aligned with the
//! temple faces ([`super::voxel::voxelize_aligned`]) so those two layers are
//! full voxels. For Iris that is h = temple width / 2 = 2 mm for the temples
//! and front thickness / 2 = 2.5 mm for the front. The lab's "fine" pass
//! (and its mesh study) refines this in a Web Worker.
//!
//! The expensive part (voxelise + solve) is memoised on the inputs it reads —
//! the frame, optics and PCB parameters — so component, print or tolerance
//! edits re-use it; only a geometry edit re-solves.

use serde::Serialize;

use super::fatigue::{self, Block, FatigueIn, FatigueOut};
use super::fea3d::{self, Grid, Precond, SolveOpts};
use super::hex8::UnitHex;
use super::materials;
use super::modal::{self, ModalOut};
use super::strength::{self, drop_analysis, DropResult, Model, Solved};
use super::tolerance::{self, FocusStackIn, FocusStackOut, PodStackIn, PodStackOut};
use crate::cad::frame::FrameLayout;
use crate::project::Project;

/// Resolution and solver settings for one analysis pass.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Fidelity {
    pub h_temple: f64,
    pub h_front: f64,
    pub h_pod: f64,
    pub max_elements: usize,
    pub tol: f64,
    /// Run the pod-squeeze case (needs voxels ≤ the 1.2 mm pod wall to bend
    /// correctly, so only the fine pass includes it).
    pub squeeze: bool,
}

impl Fidelity {
    /// Metric pass: two voxels through the temple width and the front thickness.
    pub fn metric(p: &Project) -> Fidelity {
        let f = &p.design.frame;
        let tw = f.temple_width_mm.max(0.5);
        let n = (tw / 2.0).round().max(1.0);
        Fidelity {
            h_temple: (tw / n).clamp(1.0, 2.5),
            h_front: (f.front_thickness_mm / 2.0).clamp(1.25, 3.0),
            h_pod: 1.2,
            max_elements: 8000,
            tol: 1e-3,
            squeeze: false,
        }
    }
    /// Fine pass for the lab (seconds, in a Web Worker): four voxels through
    /// the temple, and the pod squeeze with ~1.5 voxels through its wall.
    pub fn fine(p: &Project) -> Fidelity {
        let f = &p.design.frame;
        let tw = f.temple_width_mm.max(0.5);
        let n = (tw / 1.0).round().max(2.0);
        Fidelity {
            h_temple: (tw / n).clamp(0.5, 1.5),
            h_front: (f.front_thickness_mm / 4.0).clamp(0.8, 2.0),
            h_pod: 0.8,
            max_elements: 30_000,
            tol: 1e-5,
            squeeze: true,
        }
    }
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
    /// Design von Mises stress (sharp-corner zones excluded), MPa.
    pub max_vm_mpa: f64,
    /// Raw nodal peak (sharp corners included), MPa.
    pub peak_vm_mpa: f64,
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
    pub fidelity: String,
    pub material: String,
    pub material_name: String,
    pub printed: bool,
    pub yield_mpa: f64,
    pub cases: Vec<CaseSummary>,
    pub min_safety_factor: f64,
    pub worst_case: String,
    pub temple_deflection_mm: f64,
    pub drop: DropResult,
    pub pod: PodStackOut,
    pub focus: FocusStackOut,
    pub modal: ModalOut,
    pub fatigue: FatigueOut,
    /// Put-on/take-off cycles to predicted fatigue failure (Miner, whole spectrum).
    pub hinge_cycles: f64,
    pub solid_fraction_note: String,
    pub notes: Vec<String>,
}

/// The memoised finite element part of the verification.
#[derive(Clone)]
struct FeaCore {
    cases: Vec<CaseSummary>,
    drop_k: f64,
    drop_vm_per_n: f64,
    drop_sf_per_n: f64,
    drop_across: bool,
    temple_grid: Grid,
    layout: FrameLayout,
}

thread_local! {
    static CORE: crate::memo::Slot<FeaCore> = const { std::cell::RefCell::new(None) };
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
        peak_vm_mpa: sm.peak_vm,
        safety_factor: sm.sf,
        sf_von_mises: sm.sf_vm,
        governing: if sm.across { "across layers".into() } else { "in-plane".into() },
        deflection_mm: sm.load_disp,
        stiffness_n_mm: if sm.load_disp > 0.0 {
            s.model.total_force_n / sm.load_disp
        } else {
            0.0
        },
        region: hint.region,
        counts_for_min: counts,
    }
}

/// Solve several models that share one grid and one set of supports with a
/// single operator (one multigrid setup); falls back to separate solves.
fn solve_shared(models: Vec<Model>, mat: &'static materials::StructMaterial, tol: f64) -> Vec<Solved> {
    let opts = SolveOpts {
        e_mpa: mat.e_mpa(),
        nu: mat.nu,
        incompatible: true,
        precond: Precond::Multigrid,
        tol,
        max_iter: 400,
    };
    let hex = UnitHex::new(mat.nu, true);
    let same = models
        .windows(2)
        .all(|w| w[0].grid.solid == w[1].grid.solid && w[0].bcs.fixed == w[1].bcs.fixed);
    let sols: Vec<fea3d::Solved> = if same && !models.is_empty() {
        let forces: Vec<&[f64]> = models.iter().map(|m| m.bcs.force.as_slice()).collect();
        fea3d::solve_many(&models[0].grid, &models[0].bcs.fixed, &forces, &opts, &hex)
    } else {
        models.iter().map(|m| fea3d::solve(&m.grid, &m.bcs, &opts, &hex)).collect()
    };
    models
        .into_iter()
        .zip(sols)
        .map(|(m, s)| {
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
        })
        .collect()
}

fn fea_core(p: &Project, fid: Fidelity) -> Result<FeaCore, String> {
    let mat = materials::get(&p.design.frame.material);
    let cases = strength::cases();
    let force = |id: &str| cases.iter().find(|c| c.id == id).map(|c| c.default_force_n).unwrap_or(1.0);
    // Right temple: spread and the unit drop load share grid and supports.
    let mr = strength::build_model(
        p,
        "temple_r",
        "temple_spread",
        fid.h_temple,
        fid.max_elements,
        force("temple_spread"),
    )?;
    let md = strength::build_model(p, "temple_r", "drop_side", fid.h_temple, fid.max_elements, 1.0)?;
    let mut right = solve_shared(vec![mr, md], mat, fid.tol);
    let sd = right.pop().ok_or("drop solve failed")?;
    let sr = right.pop().ok_or("temple solve failed")?;
    let ml = strength::build_model(
        p,
        "temple_l",
        "temple_spread",
        fid.h_temple,
        fid.max_elements,
        force("temple_spread"),
    )?;
    let sl = solve_shared(vec![ml], mat, fid.tol).pop().ok_or("left temple solve failed")?;
    let mf = strength::build_model(p, "front", "bridge_bend", fid.h_front, fid.max_elements, force("bridge_bend"))?;
    let sf = solve_shared(vec![mf], mat, fid.tol).pop().ok_or("front solve failed")?;
    let mut out = vec![
        summary(&sr, "Temple spread — right (electronics pod)", true),
        summary(&sl, "Temple spread — left (battery pod)", true),
        summary(&sf, "Bridge bend", true),
    ];
    if fid.squeeze {
        let mp = strength::build_model(p, "temple_r", "pod_squeeze", fid.h_pod, fid.max_elements, force("pod_squeeze"))?;
        let sp = solve_shared(vec![mp], mat, fid.tol).pop().ok_or("squeeze solve failed")?;
        out.push(summary(&sp, "Pod squeeze", true));
    }
    let dsum = strength::summarize(&sd);
    let dcase = summary(&sd, "Drop on the pod — unit load (1 N)", false);
    out.push(dcase.clone());
    Ok(FeaCore {
        cases: out,
        drop_k: dcase.stiffness_n_mm,
        drop_vm_per_n: dsum.max_vm,
        drop_sf_per_n: dsum.sf,
        drop_across: dsum.across,
        temple_grid: sr.model.grid.clone(),
        layout: sr.model.layout.clone(),
    })
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

/// Open/close cycles per day in [`hinge_spectrum`].
pub const HINGE_CYCLES_PER_DAY: f64 = 12.0;

pub fn analyze(p: &Project, fid: Fidelity) -> Result<IrisOut, String> {
    let mat = materials::get(&p.design.frame.material);
    let key = crate::memo::key(&(&p.design.frame, &p.design.optics, &p.design.pcb, fid));
    let core = {
        let cached = CORE.with(|s| s.borrow().as_ref().filter(|(k, _)| *k == key).map(|(_, v)| v.clone()));
        match cached {
            Some(c) => c,
            None => {
                let c = fea_core(p, fid)?;
                CORE.with(|s| *s.borrow_mut() = Some((key, c.clone())));
                c
            }
        }
    };
    let mut out = IrisOut {
        fidelity: if fid.squeeze { "fine".into() } else { "metric".into() },
        material: mat.id.into(),
        material_name: mat.name.into(),
        printed: mat.printed,
        yield_mpa: mat.yield_mpa,
        ..Default::default()
    };
    let worst = core
        .cases
        .iter()
        .filter(|c| c.counts_for_min)
        .min_by(|a, b| a.safety_factor.total_cmp(&b.safety_factor))
        .cloned()
        .unwrap_or_default();
    out.min_safety_factor = worst.safety_factor;
    out.worst_case = format!("{} ({})", worst.label, worst.region);
    let spread: Vec<&CaseSummary> = core.cases.iter().filter(|c| c.case == "temple_spread").collect();
    out.temple_deflection_mm = spread.iter().map(|c| c.deflection_mm).fold(0.0, f64::max);
    // ---- drop (linear: SF at F newtons = SF(1 N) / F)
    let m_eff = strength::moving_mass_g(p, "temple_r");
    out.drop = drop_analysis(
        1.5,
        m_eff,
        core.drop_k,
        core.drop_vm_per_n,
        core.drop_sf_per_n,
        if core.drop_across { "across layers" } else { "in-plane" },
    );
    // ---- stack-ups
    out.pod = tolerance::pod_stack(p, &PodStackIn::default())?;
    // the project may describe its focus design in design.extra.focus_stack
    let focus_in: FocusStackIn = p
        .design
        .extra
        .get("focus_stack")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    out.focus = tolerance::focus_stack(p, &focus_in)?;
    // ---- modes of the right temple (the spread model's grid)
    let l = &core.layout;
    let added: f64 = p
        .components
        .iter()
        .filter(|c| c.location == "right_pod")
        .map(|c| c.mass_g * c.qty)
        .sum::<f64>()
        + crate::cad::frame_geom(p)
            .parts
            .iter()
            .filter(|b| b.name == "lid_r")
            .map(|b| b.assembly.volume().abs() * p.design.frame.density_g_cm3 / 1000.0)
            .sum::<f64>();
    out.modal = modal::temple_modal(
        &core.temple_grid,
        "temple_r",
        mat.e_mpa(),
        p.design.frame.density_g_cm3,
        l.pod_front_y - 1.5,
        l.pod_rear_y,
        added,
    );
    // ---- hinge fatigue from the worse temple's stress per newton
    let worse = spread
        .iter()
        .min_by(|a, b| a.safety_factor.total_cmp(&b.safety_factor))
        .map(|c| (*c).clone())
        .unwrap_or_default();
    let per_n = worse.max_vm_mpa / worse.force_n.max(1e-9);
    let sfac = if worse.governing == "across layers" {
        mat.z_strength_factor
    } else {
        1.0
    };
    out.fatigue = fatigue::analyze(FatigueIn {
        material: mat.id.into(),
        years: 3.0,
        blocks: hinge_spectrum(per_n),
        strength_factor: sfac,
    })?;
    out.hinge_cycles = if out.fatigue.has_data {
        out.fatigue.life_years * 365.0 * HINGE_CYCLES_PER_DAY
    } else {
        f64::INFINITY
    };
    // ---- print solidity note
    let pr = &p.design.print;
    let shell = 2.0 * pr.perimeters as f64 * pr.line_width_mm;
    let tw = p.design.frame.temple_width_mm;
    out.solid_fraction_note = if mat.printed {
        if shell >= tw {
            format!(
                "With {} perimeters × {:.2} mm the {:.1} mm temple prints fully solid — the solid-material assumption holds.",
                pr.perimeters, pr.line_width_mm, tw
            )
        } else {
            let core_w = tw - shell;
            let i_loss = core_w.powi(3) / tw.powi(3) * (1.0 - pr.infill_pct / 100.0);
            format!(
                "{} perimeters × {:.2} mm leave a {:.2} mm infilled core ({:.0} %) in the {:.1} mm temple: bending stiffness is ≈ {:.1} % below the solid-part result.",
                pr.perimeters,
                pr.line_width_mm,
                core_w,
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
            "Voxel sizes: temples {:.2} mm, front {:.2} mm{}; incompatible-mode bricks aligned with the temple faces; multigrid-preconditioned CG to a relative residual of {:.0e}.",
            fid.h_temple,
            fid.h_front,
            if fid.squeeze { format!(", pod {:.2} mm", fid.h_pod) } else { String::new() },
            fid.tol
        ),
        "Safety factors compare the design von Mises stress (one-voxel zones around sharp inside edges and the clamps excluded — a nominal stress) with yield, with the print-layer check. Raw nodal peaks at sharp corners are listed separately — they are stress concentrations, not converged values.".into(),
    ];
    if !fid.squeeze {
        out.notes.push("The pod-squeeze case needs voxels no larger than the 1.2 mm pod wall to bend correctly, so it runs only in the fine pass (lab button), not in the metric.".into());
    }
    out.cases = core.cases;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_metric_pass_is_sane() {
        let p = Project::iris();
        let fid = Fidelity::metric(&p);
        assert!((fid.h_temple - 2.0).abs() < 1e-12 && (fid.h_front - 2.5).abs() < 1e-12);
        let o = analyze(&p, fid).unwrap();
        for c in &o.cases {
            assert!(c.safety_factor.is_finite() && c.safety_factor > 0.0, "{c:?}");
            assert!(c.deflection_mm > 0.0, "{c:?}");
            // The design stress never exceeds the raw nodal peak.
            assert!(c.max_vm_mpa <= c.peak_vm_mpa + 1e-9, "{c:?}");
        }
        // Hand check: the slim arm as a cantilever from the pod's rear face.
        // Tip load centroid ≈ 145 mm behind the hinge along Y, pod rear face at
        // 74.4 mm → lever ≈ 70.6 mm; section 8 mm tall × 4 mm wide, bent across
        // its width: σ = F L / (h w² / 6) = 5 × 70.6 / 21.33 ≈ 16.5 MPa.
        let r = o.cases.iter().find(|c| c.case == "temple_spread" && c.part == "temple_r").unwrap();
        let hand = 5.0 * 70.6 / (8.0 * 16.0 / 6.0);
        assert!(
            r.max_vm_mpa > 0.85 * hand && r.max_vm_mpa < 1.3 * hand,
            "{} vs {hand}",
            r.max_vm_mpa
        );
        // Tip deflection of the arm alone (fixed at the pod) is a lower bound:
        // δ = F L³ / (3 E I) with I = h w³ / 12.
        let arm_only = 5.0 * 70.6f64.powi(3) / (3.0 * 2000.0 * 8.0 * 64.0 / 12.0);
        assert!(r.deflection_mm > arm_only, "{} vs {arm_only}", r.deflection_mm);
        assert!(
            o.temple_deflection_mm > 5.0 && o.temple_deflection_mm < 60.0,
            "{}",
            o.temple_deflection_mm
        );
        assert!(o.drop.peak_force_n > 10.0);
        assert!(
            o.modal.first_mode_hz > 5.0 && o.modal.first_mode_hz < 2000.0,
            "{}",
            o.modal.first_mode_hz
        );
        assert!(o.pod.stack.worst_min.is_finite());
        assert!(o.focus.worst_diopters > 0.0);
        // Memo: a component-only edit reuses the solve.
        let mut q = p.clone();
        if let Some(c) = q.components.first_mut() {
            c.mass_g += 1.0;
        }
        let o2 = analyze(&q, fid).unwrap();
        assert_eq!(o2.cases.len(), o.cases.len());
        assert!((o2.min_safety_factor - o.min_safety_factor).abs() < 1e-12);
    }
}
