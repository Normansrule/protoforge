//! # Control engine (`control.*`)
//!
//! Classical and modern control for the Control Lab:
//!
//! * [`lti`] — transfer functions, exact-ZOH time responses, Bode, gain and
//!   phase margins, Nyquist, root locus, a PID with derivative filter and
//!   back-calculation anti-windup.
//! * [`plants`] — DC motor (speed/position), mass–spring–damper, FOPDT,
//!   Iris pod thermal RC and the inverted pendulum.
//! * [`tuning`] — Ziegler–Nichols ultimate-gain and Skogestad SIMC rules.
//! * [`lqr`] — Riccati-based LQR balancing the nonlinear cart–pole.
//! * [`imu`] — Iris head tracking: IMU model + gyro/accel/complementary/
//!   Kalman estimators; produces the `control.head_rms_err_deg` metric.
//!
//! References: Åström & Murray, *Feedback Systems* 2nd ed. (fbswiki.org);
//! Franklin, Powell & Emami-Naeini, *Feedback Control of Dynamic Systems*;
//! Ziegler & Nichols 1942; Skogestad 2003; Kalman 1960; Mahony, Hamel &
//! Pflimlin 2008; Madgwick et al. 2011.

#![allow(clippy::needless_range_loop)] // index-heavy numeric code reads better with explicit indices

pub mod imu;
pub mod lqr;
pub mod lti;
pub mod mat;
pub mod plants;
pub mod tuning;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::math::{poly_add, C64};
use crate::project::{Metric, Project};
use lti::{Bode, InputKind, LoopSim, LoopSimOptions, Margins, Nyquist, Pid, Response, RootLocus, StepMetrics, Tf};

/// Which plant to use: a catalog id with parameter overrides, or a custom TF.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PlantSel {
    pub id: String,
    pub params: BTreeMap<String, f64>,
    pub num: Vec<f64>,
    pub den: Vec<f64>,
    pub delay: f64,
}

impl Default for PlantSel {
    fn default() -> Self {
        PlantSel {
            id: "mass_spring_damper".into(),
            params: BTreeMap::new(),
            num: vec![1.0],
            den: vec![1.0, 1.0],
            delay: 0.0,
        }
    }
}

impl PlantSel {
    pub fn build(&self, p: Option<&Project>) -> Result<Tf, String> {
        let custom = Tf {
            num: self.num.clone(),
            den: self.den.clone(),
            delay: self.delay.max(0.0),
        };
        plants::build(&self.id, &self.params, Some(&custom), p)
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct CatalogIn {
    project: Option<Project>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct AnalyzeIn {
    pub plant: PlantSel,
    pub pid: Pid,
    pub sim: LoopSimOptions,
    pub project: Option<Project>,
    /// Root-locus gain range as factors of the current controller.
    pub rl_k_min: Option<f64>,
    pub rl_k_max: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct AnalyzeOut {
    pub plant: Tf,
    pub controller: Tf,
    pub loop_tf: Tf,
    /// Closed loop r → y (dead time Padé-approximated).
    pub closed_loop: Tf,
    pub plant_poles: Vec<C64>,
    pub plant_zeros: Vec<C64>,
    pub closed_poles: Vec<C64>,
    pub stable: bool,
    pub open_step: Response,
    pub open_metrics: StepMetrics,
    pub sim: LoopSim,
    pub metrics: StepMetrics,
    pub t_end: f64,
    pub bode_plant: Bode,
    pub bode_loop: Bode,
    pub bode_closed: Bode,
    pub margins: Margins,
    pub nyquist: Nyquist,
    pub root_locus: RootLocus,
    /// Sensitivity peak Ms = max |1/(1+L)| (robustness; ≤ 2 is good).
    pub ms: f64,
    pub bandwidth: Option<f64>,
}

/// Full PID-loop analysis for the designer tab.
pub fn analyze(i: &AnalyzeIn) -> Result<AnalyzeOut, String> {
    let plant = i.plant.build(i.project.as_ref())?;
    let pid = i.pid;
    let c = pid.tf();
    let l = lti::loop_tf(&plant, &pid);
    let lp = l.pade(3);
    let cl_den = poly_add(&lp.den, &lp.num);
    let closed = Tf::new(lp.num.clone(), cl_den.clone());
    let closed_poles = crate::math::poly_roots(&closed.den);
    let stable = closed_poles.iter().all(|p| p.re < -1e-9);
    // time horizon: closed-loop dynamics if stable, else plant dynamics
    let t_end = i.sim.t_end.filter(|t| *t > 0.0).unwrap_or_else(|| {
        let base = if stable {
            lti::horizon_from_poles(&closed_poles)
        } else {
            plant.natural_horizon()
        };
        (base + 3.0 * plant.delay).clamp(1e-3, 1e5)
    });
    let sim = lti::simulate_pid_loop(&plant, &pid, &i.sim, t_end)?;
    let metrics = lti::step_metrics(&sim.t, &sim.y, i.sim.setpoint);
    let open_t = if plant.poles().iter().any(|p| p.re > 1e-9) {
        t_end.min(plant.natural_horizon())
    } else {
        plant.natural_horizon()
    };
    let open_step = lti::simulate(&plant, InputKind::Step, open_t, 600, 0.0)?;
    let open_metrics = lti::step_metrics(&open_step.t, &open_step.y, plant.dc_gain());
    let w = lti::auto_grid(&[&plant, &l], 400);
    let bode_plant = lti::bode(&plant, &w);
    let bode_loop = lti::bode(&l, &w);
    let bode_closed = lti::bode(&closed, &w);
    let margins = lti::margins(&l);
    let nyquist = lti::nyquist(&l, &w);
    let root_locus = lti::root_locus(&l, i.rl_k_min.unwrap_or(1e-3), i.rl_k_max.unwrap_or(1e3), 240);
    let mut ms: f64 = 0.0;
    for &wi in &w {
        let s = C64::ONE / (C64::ONE + l.eval(C64::new(0.0, wi)));
        ms = ms.max(s.abs());
    }
    let dc = closed.dc_gain().abs();
    let bandwidth = if dc > 1e-9 {
        let target = dc / 2f64.sqrt();
        w.iter().find(|&&wi| closed.eval(C64::new(0.0, wi)).abs() < target).copied()
    } else {
        None
    };
    Ok(AnalyzeOut {
        plant_poles: plant.poles(),
        plant_zeros: plant.zeros(),
        plant,
        controller: c,
        loop_tf: l,
        closed_loop: closed,
        closed_poles,
        stable,
        open_step,
        open_metrics,
        sim,
        metrics,
        t_end,
        bode_plant,
        bode_loop,
        bode_closed,
        margins,
        nyquist,
        root_locus,
        ms,
        bandwidth,
    })
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct TuneIn {
    plant: PlantSel,
    /// zn_p | zn_pi | zn_pid | pessen | no_overshoot | simc_pi | simc_pid
    method: String,
    tau_c: Option<f64>,
    n: Option<f64>,
    project: Option<Project>,
}

fn tune(i: TuneIn) -> Result<tuning::Tuning, String> {
    let plant = i.plant.build(i.project.as_ref())?;
    let n = i.n.unwrap_or(10.0);
    match i.method.as_str() {
        "simc_pi" => tuning::simc(&plant, false, i.tau_c, n),
        "simc_pid" | "simc" | "lambda" => tuning::simc(&plant, true, i.tau_c, n),
        "zn_p" => tuning::ziegler_nichols(&plant, "p", n),
        "zn_pi" => tuning::ziegler_nichols(&plant, "pi", n),
        "pessen" => tuning::ziegler_nichols(&plant, "pessen", n),
        "no_overshoot" => tuning::ziegler_nichols(&plant, "no_overshoot", n),
        _ => tuning::ziegler_nichols(&plant, "pid", n),
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
struct ResponseIn {
    num: Vec<f64>,
    den: Vec<f64>,
    delay: f64,
    input: InputKind,
    t_end: Option<f64>,
    samples: usize,
    freq_hz: f64,
}

impl Default for ResponseIn {
    fn default() -> Self {
        ResponseIn {
            num: vec![1.0],
            den: vec![1.0, 1.0],
            delay: 0.0,
            input: InputKind::Step,
            t_end: None,
            samples: 1000,
            freq_hz: 1.0,
        }
    }
}

#[derive(Serialize)]
struct ResponseOut {
    response: Response,
    metrics: StepMetrics,
    poles: Vec<C64>,
    zeros: Vec<C64>,
    dc_gain: f64,
}

fn response(i: ResponseIn) -> Result<ResponseOut, String> {
    let tf = Tf::new(i.num, i.den).with_delay(i.delay);
    let t_end = i.t_end.unwrap_or_else(|| tf.natural_horizon());
    let r = lti::simulate(&tf, i.input, t_end, i.samples, i.freq_hz)?;
    let metrics = lti::step_metrics(&r.t, &r.y, tf.dc_gain());
    Ok(ResponseOut {
        metrics,
        poles: tf.poles(),
        zeros: tf.zeros(),
        dc_gain: tf.dc_gain(),
        response: r,
    })
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct FreqIn {
    num: Vec<f64>,
    den: Vec<f64>,
    delay: f64,
    w_min: Option<f64>,
    w_max: Option<f64>,
    points: Option<usize>,
}

#[derive(Serialize)]
struct FreqOut {
    bode: Bode,
    margins: Margins,
    nyquist: Nyquist,
}

fn freq(i: FreqIn) -> Result<FreqOut, String> {
    let tf = Tf::new(i.num, i.den).with_delay(i.delay);
    let n = i.points.unwrap_or(400).clamp(20, 5000);
    let w = match (i.w_min, i.w_max) {
        (Some(a), Some(b)) if a > 0.0 && b > a => crate::math::logspace(a, b, n),
        _ => lti::auto_grid(&[&tf], n),
    };
    Ok(FreqOut {
        bode: lti::bode(&tf, &w),
        margins: lti::margins(&tf),
        nyquist: lti::nyquist(&tf, &w),
    })
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct RlIn {
    num: Vec<f64>,
    den: Vec<f64>,
    delay: f64,
    k_min: Option<f64>,
    k_max: Option<f64>,
    points: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct HeadIn {
    #[serde(flatten)]
    inner: imu::HeadInput,
    project: Option<Project>,
}

/// JSON API entry point for `control.*` methods.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "control.plants" => handle(input, |i: CatalogIn| Ok(plants::catalog(i.project.as_ref()))),
        "control.analyze" => handle(input, |i: AnalyzeIn| analyze(&i)),
        "control.tune" => handle(input, tune),
        "control.response" => handle(input, response),
        "control.frequency" => handle(input, freq),
        "control.root_locus" => handle(input, |i: RlIn| {
            let tf = Tf::new(i.num, i.den).with_delay(i.delay);
            Ok(lti::root_locus(
                &tf,
                i.k_min.unwrap_or(1e-3),
                i.k_max.unwrap_or(1e3),
                i.points.unwrap_or(240).clamp(10, 2000),
            ))
        }),
        "control.cartpole" => handle(input, |i: lqr::CartPoleInput| lqr::simulate_cartpole(&i)),
        "control.head_tracking" => handle(input, |i: HeadIn| {
            let mut inner = i.inner;
            if let Some(p) = i.project {
                inner.imu = p.design.imu.clone();
            }
            Ok(imu::head_tracking(&inner))
        }),
        _ => None,
    }
}

/// Metrics this module contributes to requirement verification.
pub fn metrics(p: &Project) -> Vec<Metric> {
    let imu = &p.design.imu;
    let v = imu::head_rms_err_deg(imu);
    let rate = imu.rate_hz.clamp(10.0, 4000.0);
    let a = imu.fusion_alpha.clamp(0.0, 0.99999);
    let fc = (1.0 - a) * rate / (2.0 * crate::math::PI * a.max(1e-9));
    vec![Metric::new("control.head_rms_err_deg", "Head-tracking tilt error (complementary filter, RMS)", v, "deg", "control").with_note(&format!(
        "60 s synthetic scenario (turns, glance, 2 Hz walking, tilt gesture, nods); IMU {rate:.0} Hz, α = {a:.3} (crossover {fc:.2} Hz), bias {:.2} °/s. Pitch/roll only — yaw is unobservable without a magnetometer.",
        imu.gyro_bias_dps
    ))]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_metric_passes() {
        let m = metrics(&Project::iris());
        assert_eq!(m[0].key, "control.head_rms_err_deg");
        assert!(m[0].value > 0.0 && m[0].value <= 2.0, "{}", m[0].value);
    }

    #[test]
    fn analyze_every_catalog_plant() {
        let p = Project::iris();
        for info in plants::catalog(Some(&p)) {
            let out = analyze(&AnalyzeIn {
                plant: PlantSel {
                    id: info.id.clone(),
                    ..Default::default()
                },
                pid: info.pid,
                project: Some(p.clone()),
                ..Default::default()
            })
            .unwrap();
            assert!(!out.sim.t.is_empty(), "{}", info.id);
            assert!(
                out.stable,
                "default PID should stabilise {} (poles {:?})",
                info.id, out.closed_poles
            );
        }
    }

    /// `cargo test -p forge-core control::tests::summary -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn summary() {
        let r = imu::head_tracking(&imu::HeadInput {
            sweep: true,
            ..Default::default()
        });
        for e in &r.estimates {
            println!(
                "{:28} roll {:7.3} pitch {:7.3} tilt {:7.3} max {:7.3} yaw_end {:7.2}",
                e.name, e.rms_roll, e.rms_pitch, e.rms_tilt, e.max_tilt_err, e.final_yaw_err
            );
        }
        println!("sweep {:?}", r.sweep);
        println!("kf bias {:?}", r.kf_bias_est);
        let t0 = std::time::Instant::now();
        let m = metrics(&Project::iris());
        println!("metric {:?} in {:?}", m[0].value, t0.elapsed());
        let p = Project::iris();
        for info in plants::catalog(Some(&p)) {
            let t0 = std::time::Instant::now();
            let out = analyze(&AnalyzeIn {
                plant: PlantSel {
                    id: info.id.clone(),
                    ..Default::default()
                },
                pid: info.pid,
                project: Some(p.clone()),
                ..Default::default()
            })
            .unwrap();
            println!(
                "{:20} t_end {:8.3} os {:6.1}% ts {:?} pm {:?} gm {:?} {:?}",
                info.id,
                out.t_end,
                out.metrics.overshoot_pct,
                out.metrics.settling_time,
                out.margins.pm_deg,
                out.margins.gm_db,
                t0.elapsed()
            );
            for m in ["zn_pid", "simc_pi", "simc_pid"] {
                let t = tune(TuneIn {
                    plant: PlantSel {
                        id: info.id.clone(),
                        ..Default::default()
                    },
                    method: m.into(),
                    project: Some(p.clone()),
                    ..Default::default()
                });
                match t {
                    Ok(t) => println!("   {m}: kp {:.4} ki {:.4} kd {:.4}", t.pid.kp, t.pid.ki, t.pid.kd),
                    Err(e) => println!("   {m}: {e}"),
                }
            }
        }
        let c = lqr::simulate_cartpole(&lqr::CartPoleInput::default()).unwrap();
        println!("K {:?} poles {:?} maxF {}", c.k, c.closed_poles, c.max_abs_force);
    }

    #[test]
    fn dispatch_roundtrip() {
        let v = serde_json::json!({"num": [1.0], "den": [1.0, 2.0, 1.0], "input": "step"});
        let r = dispatch("control.response", &v).unwrap().unwrap();
        assert!(r["metrics"]["final_value"].as_f64().unwrap() > 0.9);
    }
}
