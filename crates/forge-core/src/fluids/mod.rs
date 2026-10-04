//! # Thermal & Fluids Lab engine
//!
//! The spatially resolved companion of the Power Lab's lumped thermal
//! network (`crate::power::thermal`, metric `thermal.max_skin_c`):
//!
//! | File | What |
//! |---|---|
//! | [`fea`] | Voxel finite element method (FEM) for heat conduction: trilinear bricks, anisotropic conductivity, convective faces, CSR assembly, Jacobi / SSOR-preconditioned conjugate gradient, implicit-Euler transient |
//! | [`pod`] | The Iris right electronics pod, optics housing and front voxelised from the real CAD, PCB / spreader / display / USB-C receptacle boxes, cavity air, heat sources from the power budget at their PCB placement, correlation boundary with Picard updates; steady, transient and spreader trade |
//! | [`convection`] | Churchill–Chu (vertical plate, horizontal cylinder), horizontal plates, Churchill–Bernstein, laminar flat plate, mixed convection, radiation |
//! | [`props`] | Air properties (Incropera Table A.4) and typical solid / sheet properties |
//! | [`lbm`] | D2Q9 BGK lattice-Boltzmann wind tunnel: bounce-back, free-slip, non-equilibrium-extrapolation inlet/outlet, Guo forcing, momentum-exchange forces, Strouhal number, tracers; stateful sessions streamed to the page |
//!
//! ## Metrics (computed on the project, memoised)
//!
//! | Key | Meaning |
//! |---|---|
//! | `fluids.peak_skin_c` | Steady worst exposed frame-surface temperature, display always on (the R-008 case), 1.5 mm voxels |
//! | `fluids.hotspot_over_mean_c` | Peak minus area-mean frame-surface temperature — what a single-node model cannot see |
//! | `fluids.spreader_gain_c` | Peak reduction from a heat spreader (the project's choice, or a 25 µm graphite sheet at the hotspot if none) |
//! | `fluids.pod_peak_c` | Peak on the pod shell and lid only (the skin-contact surfaces R-008 is written for) |
//!
//! ## REFERENCES
//! * F. P. Incropera, D. P. DeWitt, T. L. Bergman, A. S. Lavine, *Fundamentals of Heat and Mass Transfer*, 7th ed., Wiley, 2011.
//! * S. W. Churchill, H. H. S. Chu, "Correlating equations for laminar and turbulent free convection from a vertical plate", Int. J. Heat Mass Transfer 18 (1975) 1323–1329; "… from a horizontal cylinder", 18 (1975) 1049–1053.
//! * S. W. Churchill, M. Bernstein, "A correlating equation for forced convection from gases and liquids to a circular cylinder in crossflow", J. Heat Transfer 99 (1977) 300–306.
//! * S. V. Patankar, *Numerical Heat Transfer and Fluid Flow*, Hemisphere, 1980.
//! * T. Krüger et al., *The Lattice Boltzmann Method: Principles and Practice*, Springer, 2017.
//! * S. Succi, *The Lattice Boltzmann Equation for Fluid Dynamics and Beyond*, Oxford, 2001.
//! * M. Schäfer, S. Turek, "Benchmark computations of laminar flow around a cylinder", Notes on Numerical Fluid Mechanics 52, Vieweg, 1996.
//! * A. J. C. Ladd, "Numerical simulations of particulate suspensions via a discretized Boltzmann equation", J. Fluid Mech. 271 (1994).
//! * Z. Guo, C. Zheng, B. Shi, "Non-equilibrium extrapolation method for velocity and pressure boundary conditions in the lattice Boltzmann method", Chinese Physics 11 (2002) 366; "Discrete lattice effects on the forcing term in the lattice Boltzmann method", Phys. Rev. E 65 (2002) 046308.
//! * C. H. K. Williamson, "Vortex dynamics in the cylinder wake", Annu. Rev. Fluid Mech. 28 (1996) 477–539.
//! * Y. Saad, *Iterative Methods for Sparse Linear Systems*, 2nd ed., SIAM, 2003.
//! * Open tools studied: OpenFOAM, Elmer FEM (ElmerCSC/elmerfem), Palabos, FluidX3D (ProjectPhysX/FluidX3D).

pub mod convection;
pub mod fea;
pub mod lbm;
pub mod pod;
pub mod props;

use std::cell::RefCell;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Metric, Project};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct SteadyIn {
    project: Option<Project>,
    config: pod::PodConfig,
    /// CAD-surface and board meshes with temperatures (default true).
    surfaces: Option<bool>,
    /// The voxel boundary mesh (default false).
    voxels: bool,
    /// Quantised temperature + material grid for slices (default true).
    slices: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct StudyIn {
    project: Option<Project>,
    config: pod::PodConfig,
    sizes: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
struct StudyRow {
    voxel_mm: f64,
    elements: usize,
    dofs: usize,
    iterations: usize,
    peak_skin_c: f64,
    mean_skin_c: f64,
    hotspot_over_mean_c: f64,
    board_max_c: f64,
    peak_part: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct AirIn {
    t_c: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
struct MaterialsOut {
    shell: Vec<props::ThermalMaterial>,
    spreaders: Vec<props::ThermalMaterial>,
    pcb: props::ThermalMaterial,
    display: props::ThermalMaterial,
}

#[derive(Clone, Debug, Serialize)]
struct ScenarioInfo {
    id: String,
    name: String,
    description: String,
}

#[derive(Clone, Debug, Serialize)]
struct ScenariosOut {
    steady: Vec<ScenarioInfo>,
    transient: Vec<(String, String, Vec<pod::Segment>)>,
}

/// JSON API for the Thermal & Fluids Lab.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "fluids.pod_steady" => handle(input, |i: SteadyIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            let cfg = i.config.resolve(&p);
            pod::steady(
                &p,
                &cfg,
                pod::Want {
                    surfaces: i.surfaces.unwrap_or(true),
                    voxels: i.voxels,
                    slices: i.slices.unwrap_or(true),
                },
            )
        }),
        "fluids.pod_transient" => handle(input, pod::transient),
        "fluids.spreader_trade" => handle(input, pod::trade),
        "fluids.mesh_study" => handle(input, |i: StudyIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            let sizes = if i.sizes.is_empty() { vec![2.0, 1.5, 1.2, 1.0] } else { i.sizes };
            let mut rows = Vec::new();
            for h in sizes.into_iter().take(6) {
                let mut c = i.config.clone();
                c.voxel_mm = Some(h);
                let cfg = c.resolve(&p);
                let o = pod::steady(&p, &cfg, pod::Want::default())?;
                rows.push(StudyRow {
                    voxel_mm: o.voxel_mm,
                    elements: o.elements,
                    dofs: o.dofs,
                    iterations: o.iterations_total,
                    peak_skin_c: o.peak_skin_c,
                    mean_skin_c: o.mean_skin_c,
                    hotspot_over_mean_c: o.hotspot_over_mean_c,
                    board_max_c: o.board_max_c,
                    peak_part: o.peak_part,
                });
            }
            Ok(rows)
        }),
        "fluids.convection" => handle(input, convection::calculator),
        "fluids.air" => handle(input, |i: AirIn| Ok(props::air(i.t_c.unwrap_or(25.0) + props::K0))),
        "fluids.materials" => handle(input, |_: Value| {
            Ok(MaterialsOut {
                shell: props::shell_materials(),
                spreaders: ["graphite", "copper", "aluminium"]
                    .iter()
                    .filter_map(|k| props::spreader(k))
                    .collect(),
                pcb: props::pcb_material(0.8, 2, 35.0, 0.5),
                display: props::display_material(),
            })
        }),
        "fluids.scenarios" => handle(input, |_: Value| {
            Ok(ScenariosOut {
                steady: crate::power::budget::scenarios()
                    .into_iter()
                    .map(|s| ScenarioInfo {
                        id: s.id,
                        name: s.name,
                        description: s.description,
                    })
                    .collect(),
                transient: ["r008", "commute", "captions"]
                    .iter()
                    .map(|id| {
                        let label = match *id {
                            "commute" => "Commute: walk, train, walk, desk",
                            "captions" => "Meeting with live captions, then a break",
                            _ => "Put on, display always on (R-008)",
                        };
                        (id.to_string(), label.to_string(), pod::preset_segments(id))
                    })
                    .collect(),
            })
        }),
        "fluids.lbm_presets" => handle(input, |_: Value| Ok(lbm::presets())),
        "fluids.lbm_init" => handle(input, lbm::init),
        "fluids.lbm_step" => handle(input, lbm::step_session),
        "fluids.lbm_paint" => handle(input, lbm::paint),
        "fluids.lbm_run" => handle(input, lbm::run),
        _ => None,
    }
}

thread_local! {
    static METRICS: crate::memo::Slot<Vec<Metric>> = const { RefCell::new(None) };
}

/// Voxel size used for the requirement metrics, mm.
pub const METRIC_VOXEL_MM: f64 = 1.5;

/// Metrics feeding requirement verification (memoised on every input they read).
pub fn metrics(p: &Project) -> Vec<Metric> {
    let subs: Vec<(&str, &crate::project::Domain)> = p.subsystems.iter().map(|s| (s.id.as_str(), &s.domain)).collect();
    let key = crate::memo::key(&(
        &p.design.frame,
        &p.design.optics,
        &p.design.pcb,
        &p.design.thermal,
        &p.design.power_rails,
        &p.design.battery,
        &p.components,
        subs,
        p.design.extra.get("fluids"),
    ));
    crate::memo::memo(&METRICS, key, || metrics_uncached(p))
}

fn metrics_uncached(p: &Project) -> Vec<Metric> {
    let src = "fluids";
    let base = pod::PodConfig {
        voxel_mm: Some(METRIC_VOXEL_MM),
        scenario: Some("display_on".into()),
        tol: Some(1e-7),
        ..Default::default()
    };
    let cfg = base.resolve(p);
    let pm = match pod::build(p, &cfg) {
        Ok(m) => m,
        Err(_) => return vec![],
    };
    let (theta, _, _, _) = pod::solve_steady(&pm, &pm.q, 3, None);
    let sk = pod::skin_stats(&pm, &theta);
    let ta = cfg.ambient_c;
    let peak = ta + sk.peak;
    let part = pod::PART_NAMES[sk.peak_part as usize];
    let pod_peak = ta + sk.per_part[pod::PART_POD as usize].0.max(sk.per_part[pod::PART_LID as usize].0);
    let lumped = crate::power::thermal::worst_case(p).0;
    // Spreader gain: the project's spreader vs none, or (none chosen) a 25 µm
    // graphite sheet where the hotspot is.
    let (gain, gain_note) = if cfg.spreader == "none" {
        let loc = if sk.peak_part == pod::PART_OPTICS || sk.peak_part == pod::PART_FRONT {
            "optics"
        } else {
            "floor"
        };
        let mut c = cfg.clone();
        c.spreader = "graphite".into();
        c.spreader_um = 25.0;
        c.spreader_location = loc.into();
        let g = pod::build(p, &c).map(|m| {
            let (t, _, _, _) = pod::solve_steady(&m, &m.q, 3, Some(&theta));
            sk.peak - pod::skin_stats(&m, &t).peak
        });
        (
            g.unwrap_or(0.0),
            format!(
                "a 25 µm graphite sheet ({}) would lower the peak by this much; the design has no spreader yet",
                if loc == "optics" {
                    "behind the display, on the optics-housing end wall"
                } else {
                    "on the pod's skin-side wall"
                }
            ),
        )
    } else {
        let mut c = cfg.clone();
        c.spreader = "none".into();
        let g = pod::build(p, &c).map(|m| {
            let (t, _, _, _) = pod::solve_steady(&m, &m.q, 3, Some(&theta));
            pod::skin_stats(&m, &t).peak - sk.peak
        });
        (
            g.unwrap_or(0.0),
            format!(
                "the design's {} µm {} sheet ({}) vs none",
                cfg.spreader_um, cfg.spreader, cfg.spreader_location
            ),
        )
    };
    vec![
        Metric::new("fluids.peak_skin_c", "Peak frame surface temperature (3-D, display always on)", peak, "C", src).with_note(&format!(
            "steady 3-D conduction on {:.1} mm voxels, {:.0} mW at {:.0} °C ambient, natural convection + radiation: hottest exposed surface on the {} ({}); lumped two-node model says {:.1} °C",
            METRIC_VOXEL_MM,
            pm.total_w * 1e3,
            ta,
            part.to_lowercase(),
            if cfg.display_heat_in == "pod" { "display heat lumped into the pod" } else { "display heat in the optics housing, as the BOM places it" },
            lumped
        )),
        Metric::new("fluids.hotspot_over_mean_c", "Hotspot above the mean surface temperature", sk.peak - sk.mean, "C", src).with_note(&format!(
            "peak {:.1} °C vs area-mean {:.1} °C over {:.0} cm² of exposed frame — a single-node (lumped) model sees only the mean",
            peak,
            ta + sk.mean,
            sk.area * 1e4
        )),
        Metric::new("fluids.spreader_gain_c", "Peak reduction from a heat spreader", gain, "C", src).with_note(&gain_note),
        Metric::new("fluids.pod_peak_c", "Peak pod shell temperature (skin-contact side)", pod_peak, "C", src)
            .with_note("pod shell and lid only — the surfaces R-008's lumped model represents"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_metrics_and_energy_balance() {
        // v0.6 has a graphite sheet behind the display: the hotspot is within the skin limit
        let v6 = metrics(&Project::iris());
        let peak6 = v6.iter().find(|x| x.key == "fluids.peak_skin_c").unwrap().value;
        assert!(peak6 <= 41.0, "{peak6}");
        // v0.5 had none
        let p = crate::bridge::diff::iris_v05();
        let m = metrics(&p);
        let get = |k: &str| m.iter().find(|x| x.key == k).unwrap_or_else(|| panic!("missing {k}")).value;
        let peak = get("fluids.peak_skin_c");
        let hot = get("fluids.hotspot_over_mean_c");
        let gain = get("fluids.spreader_gain_c");
        let pod_peak = get("fluids.pod_peak_c");
        // display heat (140 mW) concentrated in the plastic optics housing → a hotspot far above the lumped 39 °C
        assert!(peak > 50.0 && peak < 80.0, "{peak}");
        assert!(hot > 15.0, "{hot}");
        assert!(gain > 10.0, "{gain}");
        assert!(pod_peak > 30.0 && pod_peak < 36.0, "{pod_peak}");
        // memoised
        assert_eq!(metrics(&p), m);
        // the heat placed equals the Power Lab pod heat, and it all leaves through the surface
        let cfg = pod::PodConfig {
            voxel_mm: Some(2.0),
            ..Default::default()
        }
        .resolve(&p);
        let o = pod::steady(&p, &cfg, pod::Want::default()).unwrap();
        let ph = crate::power::budget::pod_heat_mw(&p, &pod::profile_for("display_on", None));
        assert!((o.total_mw - ph).abs() < 1e-6, "{} vs {ph}", o.total_mw);
        assert!(
            (o.heat_out_mw - o.total_mw).abs() / o.total_mw < 1e-4,
            "{} vs {}",
            o.heat_out_mw,
            o.total_mw
        );
        assert!(o.converged);
        assert!(o.cavity_cells as f64 > 0.6 * o.cavity_expected_cells);
    }

    #[test]
    fn display_heat_in_the_pod_compares_with_the_lumped_model() {
        // Like-for-like with the lumped model (all heat in the pod): the 3-D mean is far
        // below 39 °C but the hotspot over the board's hot parts reaches about the lumped value.
        let p = crate::bridge::diff::iris_v05();
        let cfg = pod::PodConfig {
            voxel_mm: Some(1.5),
            display_heat_in: Some("pod".into()),
            ..Default::default()
        }
        .resolve(&p);
        let o = pod::steady(&p, &cfg, pod::Want::default()).unwrap();
        assert!(o.peak_part.starts_with("Pod"), "{}", o.peak_part);
        assert!(o.mean_skin_c < o.lumped.shell_c - 4.0);
        assert!(o.peak_skin_c > o.mean_skin_c + 5.0);
        assert!(o.board_max_c > o.peak_skin_c);
        // a floor graphite sheet lowers the pod hotspot
        let g = pod::steady(
            &p,
            &pod::PodConfig {
                spreader: Some("graphite".into()),
                ..pod::PodConfig {
                    voxel_mm: Some(1.5),
                    display_heat_in: Some("pod".into()),
                    ..Default::default()
                }
            }
            .resolve(&p),
            pod::Want::default(),
        )
        .unwrap();
        assert!(g.peak_skin_c < o.peak_skin_c - 0.5, "{} vs {}", g.peak_skin_c, o.peak_skin_c);
    }

    #[test]
    fn dispatch_roundtrip() {
        let v = serde_json::json!({ "config": { "voxel_mm": 2.0 }, "voxels": true });
        let o = dispatch("fluids.pod_steady", &v).unwrap().unwrap();
        assert!(o["peak_skin_c"].as_f64().unwrap() > 30.0);
        assert!(o["surfaces"].as_array().unwrap().len() == 4);
        assert!(o["slices"]["temp_u8"].as_str().unwrap().len() > 100);
        assert!(o["voxel_surface"]["triangles"].as_u64().unwrap() > 100);
        let c = dispatch("fluids.convection", &serde_json::json!({})).unwrap().unwrap();
        assert!(c["rows"].as_array().unwrap().len() == 7);
        assert!(dispatch("fluids.air", &serde_json::json!({ "t_c": 30 })).unwrap().is_ok());
        assert!(dispatch("fluids.materials", &serde_json::json!({})).unwrap().is_ok());
        assert!(dispatch("fluids.scenarios", &serde_json::json!({})).unwrap().is_ok());
        assert!(dispatch("fluids.lbm_presets", &serde_json::json!({})).unwrap().is_ok());
        let r = dispatch(
            "fluids.lbm_run",
            &serde_json::json!({ "config": { "nx": 60, "ny": 30, "size_cells": 6, "tracers": 0 }, "steps": 30, "frame_every": 10 }),
        )
        .unwrap()
        .unwrap();
        assert_eq!(r["frames"].as_array().unwrap().len(), 3);
        assert!(dispatch("fluids.nope", &v).is_none());
        assert!(dispatch("fluids.pod_steady", &serde_json::json!({ "config": { "voxel_mm": "x" } }))
            .unwrap()
            .is_err());
    }

    #[test]
    fn subdivision_keeps_area() {
        let m = crate::cad::solid::box_mesh([0.0, 0.0, 0.0], [10.0, 4.0, 2.0]);
        let s = pod::subdivide(&m, 0.7);
        assert!((s.area() - m.area()).abs() < 1e-9);
        assert!(s.triangle_count() > 20 * m.triangle_count());
        // the drawn Iris surfaces stay light enough for software WebGL
        let p = Project::iris();
        let cfg = pod::PodConfig {
            voxel_mm: Some(1.5),
            ..Default::default()
        }
        .resolve(&p);
        let o = pod::steady(
            &p,
            &cfg,
            pod::Want {
                surfaces: true,
                ..Default::default()
            },
        )
        .unwrap();
        let tris: usize = o.surfaces.iter().chain(o.inner.iter()).map(|s| s.triangles).sum();
        assert!(tris < 20_000, "{tris} triangles");
        let v = o.volume.unwrap();
        assert_eq!(v.nx * v.ny * v.nz, crate::fluids::lbm::base64_decode(&v.data).unwrap().len());
    }

    #[test]
    fn transient_rises_towards_steady_state() {
        let p = Project::iris();
        let o = pod::transient(pod::TransientIn {
            config: pod::PodConfig {
                voxel_mm: Some(2.0),
                display_heat_in: Some("pod".into()),
                ..Default::default()
            },
            segments: vec![pod::Segment {
                duration_s: 3.0 * 3600.0,
                display_duty: 1.0,
                label: "on".into(),
            }],
            dt_s: Some(60.0),
            frames: Some(10),
            ..Default::default()
        })
        .unwrap();
        let n = o.peak_skin_c.len();
        assert!(n > 50);
        // monotone rise from ambient, ending near the steady state
        assert!((o.peak_skin_c[0] - o.ambient_c).abs() < 1e-9);
        for w in o.peak_skin_c.windows(2) {
            assert!(w[1] >= w[0] - 1e-6);
        }
        let last = o.peak_skin_c[n - 1];
        assert!((last - o.steady_peak_c).abs() < 0.3, "{last} vs {}", o.steady_peak_c);
        assert_eq!(o.frames.len(), o.frame_t_s.len());
        assert!(o.frames.len() >= 10);
        // the lumped network sees the same heat, reaching its own 39 °C
        assert!((o.lumped_shell_c[n - 1] - crate::power::thermal::worst_case(&p).0).abs() < 0.2);
        let _ = p;
    }

    #[test]
    fn spreader_trade_rows() {
        let o = pod::trade(pod::TradeIn {
            config: pod::PodConfig {
                voxel_mm: Some(2.0),
                display_heat_in: Some("pod".into()),
                ..Default::default()
            },
            thicknesses_um: vec![25.0, 100.0],
            ..Default::default()
        })
        .unwrap();
        assert_eq!(o.rows.len(), 9);
        // every spreader helps or is neutral; thicker graphite at least as good as thinner
        for r in &o.rows[1..] {
            assert!(r.gain_c > -0.2, "{} {}", r.label, r.gain_c);
        }
        let g = &o.sweeps[0].peak_skin_c;
        assert!(g[1] <= g[0] + 0.05);
        assert_eq!(o.hotspot_location, "floor");
    }

    /// Over-full voxels (sheet + housing wall + display module) used to scale
    /// every fraction down by the overfill, which removed 8 % of a 70 µm
    /// graphite sheet but 20 % of a 0.5 mm aluminium plate — the two then gave
    /// the same peak although their sheet conductances k·t differ (105 vs
    /// 115 mW/K). The sheet now keeps its whole volume.
    #[test]
    fn sheet_keeps_its_volume_in_over_full_voxels() {
        let p = Project::iris();
        for (kind, t) in [("graphite", 70.0), ("aluminium", 500.0), ("copper", 35.0)] {
            let c = pod::PodConfig {
                voxel_mm: Some(1.5),
                spreader: Some(kind.into()),
                spreader_um: Some(t),
                spreader_location: Some("optics".into()),
                ..Default::default()
            }
            .resolve(&p);
            let m = pod::build(&p, &c).unwrap();
            let b = m.spreader_box.unwrap();
            let vol: f64 = (0..3).map(|d| b.hi[d] - b.lo[d]).product();
            // f32 voxel fractions: agreement to 0.1 % (the old scaling lost 8–20 %)
            assert!(
                (m.spreader_volume_mm3 - vol).abs() < 1e-3 * vol,
                "{kind} {t}: {} vs {vol}",
                m.spreader_volume_mm3
            );
        }
    }

    #[test]
    fn trade_with_brackets_that_change_the_mesh() {
        // Page settings: display heat in the optics housing, 1.5 mm voxels. The 0.5 / 1 mm
        // aluminium brackets fill voxels the baseline left empty, so the baseline warm start
        // has a different length — regression test for an out-of-bounds panic in WebAssembly.
        let p = Project::iris();
        let o = pod::trade(pod::TradeIn {
            config: pod::PodConfig {
                voxel_mm: Some(1.5),
                display_heat_in: Some("optics".into()),
                ..Default::default()
            },
            thicknesses_um: vec![25.0],
            ..Default::default()
        })
        .unwrap();
        assert_eq!(o.rows.len(), 9);
        assert_eq!(o.hotspot_location, "optics");
        let base = pod::build(
            &p,
            &pod::PodConfig {
                voxel_mm: Some(1.5),
                ..Default::default()
            }
            .resolve(&p),
        )
        .unwrap();
        let al = pod::build(
            &p,
            &pod::PodConfig {
                voxel_mm: Some(1.5),
                spreader: Some("aluminium".into()),
                spreader_um: Some(1000.0),
                spreader_location: Some("optics".into()),
                ..Default::default()
            }
            .resolve(&p),
        )
        .unwrap();
        assert!(al.model.ndof() >= base.model.ndof());
        // every sheet behind the display helps a lot; thicker aluminium at least as much
        let opt: Vec<&pod::TradeRow> = o.rows.iter().filter(|r| r.location == "optics").collect();
        assert_eq!(opt.len(), 5);
        for r in &opt {
            assert!(r.gain_c > 5.0, "{} {}", r.label, r.gain_c);
        }
        assert!(opt[4].peak_skin_c <= opt[3].peak_skin_c + 0.05);
        // 70 µm graphite (105 mW/K) and a 0.5 mm aluminium plate (115 mW/K) are
        // different sheets and give different peaks; the more conductive one is cooler
        let g70 = o.rows.iter().find(|r| r.id == "graphite_70_optics").unwrap();
        let al = o.rows.iter().find(|r| r.id == "aluminium_500_optics").unwrap();
        assert!(al.sheet_conductance_mw_k > g70.sheet_conductance_mw_k);
        assert!(al.peak_skin_c < g70.peak_skin_c - 0.02, "{} vs {}", al.peak_skin_c, g70.peak_skin_c);
    }
}
