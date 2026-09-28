//! # Power & thermal engine (`power.*`)
//!
//! | file | what |
//! |---|---|
//! | [`budget`] | duty-cycle energy budget per component / subsystem, usage profiles and named scenarios, battery life |
//! | [`battery`] | open-circuit-voltage (OCV) + internal-resistance cell model, constant-power discharge, temperature derating, capacity trade |
//! | [`thermal`] | two-node RC thermal network (board → pod shell → air): exact transient, steady state, eigenvalues, display throttling, sweeps |
//!
//! Metrics (requirement keys in the Iris project):
//!
//! | key | meaning | requirement |
//! |---|---|---|
//! | `power.battery_life_h` | usable energy / average battery power, reference profile | R-002 ≥ 8 h |
//! | `power.avg_power_mw` | average battery power, reference profile | R-003 ≤ 80 mW |
//! | `thermal.max_skin_c` | steady pod-shell temperature, display always on, at `thermal.ambient_c` | R-008 ≤ 41 °C |
//! | `power.peak_power_mw`, `power.energy_mwh`, `thermal.board_max_c`, `thermal.time_constant_s` | informational | |
//!
//! Every metric is a closed-form expression (a few microseconds), so no
//! memoisation is needed. `thermal.max_skin_c` uses the analytic steady
//! state, which is the supremum of the transient from ambient because the
//! network's step response is monotone (real negative eigenvalues, positive
//! input coupling).

pub mod battery;
pub mod budget;
pub mod thermal;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Metric, Project};

pub use budget::{budget, Budget, UsageProfile};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct ProfileIn {
    project: Option<Project>,
    /// Built-in scenario id (see `power.scenarios`); applied before `profile`.
    scenario: Option<String>,
    profile: Option<UsageProfile>,
}

impl ProfileIn {
    fn resolve(self) -> Result<(Project, UsageProfile), String> {
        let p = self.project.unwrap_or_else(Project::iris);
        let mut prof = match &self.scenario {
            Some(id) if !id.is_empty() && id != "custom" => {
                budget::scenario_profile(id).ok_or_else(|| format!("unknown scenario `{id}`"))?
            }
            _ => UsageProfile::default(),
        };
        if let Some(extra) = self.profile {
            prof.subsystem_scale.extend(extra.subsystem_scale);
            prof.subsystem_duty.extend(extra.subsystem_duty);
            prof.component_duty.extend(extra.component_duty);
            if extra.display_duty.is_some() {
                prof.display_duty = extra.display_duty;
            }
        }
        Ok((p, prof))
    }
}

#[derive(Serialize)]
struct BudgetOut {
    #[serde(flatten)]
    budget: Budget,
    profile: UsageProfile,
    scenario: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct DischargeIn {
    #[serde(flatten)]
    who: ProfileIn,
    config: battery::DischargeConfig,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ThermalIn {
    project: Option<Project>,
    scenario: Option<String>,
    config: thermal::ThermalConfig,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct SweepIn {
    project: Option<Project>,
    config: thermal::SweepConfig,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TradeIn {
    #[serde(flatten)]
    who: ProfileIn,
    config: battery::TradeConfig,
}

#[derive(Serialize)]
struct OcvOut {
    soc: Vec<f64>,
    ocv_v: Vec<f64>,
    temp_c: Vec<f64>,
    capacity_factor: Vec<f64>,
    r_factor: Vec<f64>,
}

/// JSON API entry point for `power.*` methods.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "power.budget" => handle(input, |i: ProfileIn| {
            let scen = i.scenario.clone().unwrap_or_else(|| "reference".into());
            let (p, prof) = i.resolve()?;
            Ok(BudgetOut {
                budget: budget::budget(&p, &prof),
                profile: prof,
                scenario: scen,
            })
        }),
        "power.scenarios" => handle(input, |i: ProfileIn| {
            let (p, _) = i.resolve()?;
            Ok(budget::compare(&p))
        }),
        "power.discharge" => handle(input, |i: DischargeIn| {
            let (p, prof) = i.who.resolve()?;
            let b = budget::budget(&p, &prof);
            Ok(battery::discharge(&p, b.avg_mw, b.peak_mw, &i.config))
        }),
        "power.ocv" => handle(input, |i: ProfileIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            let soc = crate::math::linspace(0.0, 1.0, 101);
            let temps = crate::math::linspace(-20.0, 60.0, 81);
            Ok(OcvOut {
                ocv_v: soc.iter().map(|&s| battery::ocv(s, p.design.battery.full_v)).collect(),
                soc,
                capacity_factor: temps.iter().map(|&t| battery::temp_factors(t).0).collect(),
                r_factor: temps.iter().map(|&t| battery::temp_factors(t).1).collect(),
                temp_c: temps,
            })
        }),
        "power.thermal" => handle(input, |i: ThermalIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            let mut cfg = i.config;
            if let Some(id) = i.scenario.as_deref().filter(|s| !s.is_empty() && *s != "custom") {
                cfg.profile = budget::scenario_profile(id).ok_or_else(|| format!("unknown scenario `{id}`"))?;
            }
            Ok(thermal::transient(&p, &cfg))
        }),
        "power.thermal_sweep" => handle(input, |i: SweepIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            Ok(thermal::sweep(&p, &i.config))
        }),
        "power.trade" => handle(input, |i: TradeIn| {
            let (p, prof) = i.who.resolve()?;
            let b = budget::budget(&p, &prof);
            Ok(battery::trade(&p, b.avg_mw, &i.config))
        }),
        _ => None,
    }
}

/// Metrics this module contributes to requirement verification.
pub fn metrics(p: &Project) -> Vec<Metric> {
    let b = budget::budget(p, &UsageProfile::default());
    let (skin, board, heat) = thermal::worst_case(p);
    let net = thermal::Network::from_params(&p.design.thermal);
    let (tau_fast, tau_slow) = net.time_constants();
    vec![
        Metric::new("power.battery_life_h", "Battery life (reference profile)", b.life_h, "h", "power").with_note(&format!(
            "{:.0} mWh usable ({:.0} mAh × {:.2} V × {:.0} %) ÷ {:.1} mW",
            b.energy_mwh,
            p.design.battery.capacity_mah,
            p.design.battery.nominal_v,
            p.design.battery.usable_fraction * 100.0,
            b.avg_mw
        )),
        Metric::new(
            "power.avg_power_mw",
            "Average battery power (reference profile)",
            b.avg_mw,
            "mW",
            "power",
        )
        .with_note(&format!(
            "loads {:.1} mW ÷ η {:.0} % + quiescent {:.3} mW",
            b.load_mw,
            b.buck_efficiency * 100.0,
            b.quiescent_mw
        )),
        Metric::new(
            "power.peak_power_mw",
            "Peak battery power (all loads active)",
            b.peak_mw,
            "mW",
            "power",
        )
        .with_note(&format!("≈ {:.0} mA at the nominal voltage", b.peak_current_ma)),
        Metric::new("power.energy_mwh", "Usable battery energy", b.energy_mwh, "mWh", "power"),
        Metric::new("thermal.max_skin_c", "Pod skin temperature, display always on", skin, "C", "power").with_note(&format!(
            "steady state: {:.1} °C ambient + {:.0} mW × {:.0} °C/W",
            p.design.thermal.ambient_c, heat, net.r_pa
        )),
        Metric::new("thermal.board_max_c", "Board temperature, display always on", board, "C", "power"),
        Metric::new("thermal.time_constant_s", "Dominant thermal time constant", tau_slow, "s", "power").with_note(&format!(
            "fast mode {tau_fast:.1} s (board), slow mode {tau_slow:.0} s (shell + air)"
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
        assert!((get("power.battery_life_h") - 10.34).abs() < 0.05);
        assert!((get("power.avg_power_mw") - 54.76).abs() < 0.05);
        assert!(get("power.avg_power_mw") <= 80.0);
        let skin = get("thermal.max_skin_c");
        assert!(skin > 38.5 && skin < 39.5, "{skin}");
        assert!(get("thermal.board_max_c") > skin);
        assert!((get("thermal.time_constant_s") - 419.0).abs() < 2.0);
        assert!(get("power.peak_power_mw") > 190.0);
    }

    #[test]
    fn dispatch_roundtrip() {
        let v = serde_json::json!({ "scenario": "captions" });
        let out = dispatch("power.budget", &v).unwrap().unwrap();
        assert!(out["avg_mw"].as_f64().unwrap() > 100.0);
        assert_eq!(out["scenario"], "captions");
        assert!(dispatch("power.nope", &v).is_none());
        let d = dispatch("power.discharge", &serde_json::json!({})).unwrap().unwrap();
        assert!(d["main"]["cutoff_h"].as_f64().unwrap() > 8.0);
        let t = dispatch("power.thermal", &serde_json::json!({ "config": { "duration_s": 600 } }))
            .unwrap()
            .unwrap();
        assert!(t["t_s"].as_array().unwrap().len() > 100);
        assert!(dispatch("power.thermal_sweep", &serde_json::json!({})).unwrap().is_ok());
        assert!(dispatch("power.trade", &serde_json::json!({ "config": { "total_mass_g": 45 } }))
            .unwrap()
            .is_ok());
        assert!(dispatch("power.scenarios", &serde_json::json!({})).unwrap().is_ok());
        assert!(dispatch("power.ocv", &serde_json::json!({})).unwrap().is_ok());
        assert!(dispatch("power.budget", &serde_json::json!({"scenario": "zzz"})).unwrap().is_err());
    }

    #[test]
    fn metrics_are_fast() {
        let p = Project::iris();
        let t0 = std::time::Instant::now();
        for _ in 0..100 {
            let _ = metrics(&p);
        }
        assert!(t0.elapsed().as_millis() < 500);
    }
}

#[cfg(test)]
mod debug {
    use super::*;

    /// `cargo test -p forge-core power::debug -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn dump() {
        let p = Project::iris();
        for m in metrics(&p) {
            println!("{:28} {:10.3} {:4} {}", m.key, m.value, m.unit, m.note);
        }
        for s in budget::compare(&p) {
            println!(
                "{:32} {:7.2} mW {:7.2} h  display {:.2}",
                s.name, s.avg_mw, s.life_h, s.display_duty
            );
        }
        let b = budget::budget(&p, &UsageProfile::default());
        let d = battery::discharge(&p, b.avg_mw, b.peak_mw, &battery::DischargeConfig::default());
        for c in std::iter::once(&d.main).chain(d.comparisons.iter()) {
            println!(
                "{:28} {:6.2} h {:8} soc@cut {:.3} {:6.1} mAh {:6.1} mWh eq-usable {:.3} brownout {:?}",
                c.label,
                c.cutoff_h,
                c.end_reason,
                c.soc_at_cutoff,
                c.delivered_mah,
                c.delivered_mwh,
                c.equivalent_usable_fraction,
                c.brownout_h
            );
        }
        println!("budget life {:.2} h, max power {:.0} mW", d.budget_life_h, d.max_power_mw);
        let t = battery::trade(&p, b.avg_mw, &battery::TradeConfig::default());
        println!(
            "trade: other {:.2} g ({}), min {:.1} mAh, max {:.1} mAh, now {:.1} g",
            t.other_mass_g, t.mass_source, t.min_mah_for_life, t.max_mah_for_mass, t.current_mass_g
        );
        let s = thermal::sweep(&p, &thermal::SweepConfig::default());
        println!("max ambient {:.1} C, safe duty {:.2}", s.max_ambient_c, s.max_safe_duty);
    }
}
