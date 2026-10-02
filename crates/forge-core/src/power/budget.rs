//! Energy budget: duty-cycle average power per component and subsystem,
//! conversion loss, quiescent drain, usable battery energy and battery life.
//!
//! For one component with active power `P_a`, sleep power `P_s` and duty
//! cycle `d` (fraction of time active), the time-average power is
//!
//! ```text
//! P̄ = qty · (P_a·d + P_s·(1 − d))
//! ```
//!
//! The loads sit behind a buck converter of efficiency `η`, so the battery
//! supplies `ΣP̄ / η`; the converter's quiescent current `I_q` is drawn
//! straight from the cell at its nominal voltage. Usable energy is
//! `E = C · V_nom · f_usable` (mAh × V = mWh) and life is `E / P_batt`.
//! This is the same definition the RL Lab uses for its fixed policy (it
//! divides the tiny quiescent term by `η` as well — a 0.002 mW difference
//! for Iris).
//!
//! Peak power is every load active at once (duty = 1): the worst case the
//! battery's internal resistance and the rail must survive.
//!
//! References: D. Linden & T. B. Reddy, *Handbook of Batteries* (4th ed.,
//! McGraw-Hill, 2011), ch. 3 (battery performance, energy from capacity ×
//! voltage); L. Benini, A. Bogliolo & G. De Micheli, "A survey of design
//! techniques for system-level dynamic power management", IEEE Trans. VLSI
//! Systems 8(3), 2000.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::project::{Component, Project};

/// Overrides of the bill-of-materials (BOM) duty cycles. Applied in order:
/// BOM duty → `subsystem_scale` (multiplier) → `subsystem_duty` (absolute)
/// → `component_duty` (absolute, by reference designator). Results are
/// clamped to 0..1.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct UsageProfile {
    pub subsystem_scale: BTreeMap<String, f64>,
    pub subsystem_duty: BTreeMap<String, f64>,
    pub component_duty: BTreeMap<String, f64>,
    /// Set the display's duty cycle (the component found by [`display_ref`]).
    pub display_duty: Option<f64>,
}

impl UsageProfile {
    pub fn duty_for(&self, p: &Project, c: &Component) -> f64 {
        let mut d = c.power.duty;
        if let Some(k) = self.subsystem_scale.get(&c.subsystem) {
            d *= k;
        }
        if let Some(v) = self.subsystem_duty.get(&c.subsystem) {
            d = *v;
        }
        if let Some(v) = self.component_duty.get(&c.refdes) {
            d = *v;
        }
        if let Some(v) = self.display_duty {
            if display_ref(p).as_deref() == Some(c.refdes.as_str()) {
                d = v;
            }
        }
        d.clamp(0.0, 1.0)
    }
}

/// A named usage scenario (Iris use cases UC-1 … UC-4 plus standby and the
/// R-008 thermal worst case).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub name: String,
    pub description: String,
    pub profile: UsageProfile,
}

fn map(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

/// Built-in scenarios. Subsystem ids follow the Iris project (`SYS-OPT`
/// display, `SYS-CMP` compute & radio, `SYS-AUD` microphones); ids that a
/// project does not have are simply ignored. The multipliers are modelling
/// assumptions, stated in each description.
pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            id: "reference".into(),
            name: "Reference mixed use".into(),
            description: "The BOM duty cycles as entered in the project — the profile requirement R-002 is written against (display on about a third of the time).".into(),
            profile: UsageProfile::default(),
        },
        Scenario {
            id: "navigation".into(),
            name: "Navigation walk (UC-1)".into(),
            description: "Turn-by-turn directions: display duty ×2 (arrows redrawn often), system-on-chip ×1.6 (phone link, map updates), microphones ×0.4.".into(),
            profile: UsageProfile {
                subsystem_scale: map(&[("SYS-OPT", 2.0), ("SYS-CMP", 1.6), ("SYS-AUD", 0.4)]),
                ..Default::default()
            },
        },
        Scenario {
            id: "captions".into(),
            name: "All-day captions (UC-2)".into(),
            description: "Live captions in a meeting: display duty ×2.67, microphones ×4 (always listening), system-on-chip ×2.8 (audio streamed to the phone over Bluetooth Low Energy).".into(),
            profile: UsageProfile {
                subsystem_scale: map(&[("SYS-OPT", 2.67), ("SYS-CMP", 2.8), ("SYS-AUD", 4.0)]),
                ..Default::default()
            },
        },
        Scenario {
            id: "notifications".into(),
            name: "Glanceable notifications (UC-3)".into(),
            description: "Short glances at messages: display duty ×0.33, system-on-chip ×0.6, microphones ×0.2.".into(),
            profile: UsageProfile {
                subsystem_scale: map(&[("SYS-OPT", 0.33), ("SYS-CMP", 0.6), ("SYS-AUD", 0.2)]),
                ..Default::default()
            },
        },
        Scenario {
            id: "standby".into(),
            name: "Standby".into(),
            description: "Display and microphones off, radio advertising slowly (system-on-chip ×0.08); the inertial sensor stays on for wake-on-gesture.".into(),
            profile: UsageProfile {
                subsystem_duty: map(&[("SYS-OPT", 0.0), ("SYS-AUD", 0.0)]),
                subsystem_scale: map(&[("SYS-CMP", 0.08)]),
                ..Default::default()
            },
        },
        Scenario {
            id: "display_on".into(),
            name: "Display always on (R-008 case)".into(),
            description: "The thermal verification case: display duty = 1, everything else at the reference duty.".into(),
            profile: UsageProfile {
                display_duty: Some(1.0),
                ..Default::default()
            },
        },
    ]
}

pub fn scenario_profile(id: &str) -> Option<UsageProfile> {
    scenarios().into_iter().find(|s| s.id == id).map(|s| s.profile)
}

/// The display component: first powered part whose name contains
/// "display" (same rule as the RL Lab), else a part in an optical subsystem.
pub fn display_ref(p: &Project) -> Option<String> {
    p.components
        .iter()
        .find(|c| c.name.to_lowercase().contains("display") && c.power.active_mw > 0.0)
        .or_else(|| {
            p.components.iter().find(|c| {
                c.power.active_mw > 0.0
                    && p.subsystems
                        .iter()
                        .any(|s| s.id == c.subsystem && matches!(s.domain, crate::project::Domain::Optical))
            })
        })
        .map(|c| c.refdes.clone())
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ComponentPower {
    pub refdes: String,
    pub name: String,
    pub subsystem: String,
    pub location: String,
    pub qty: f64,
    pub active_mw: f64,
    pub sleep_mw: f64,
    /// Duty after the usage profile.
    pub duty: f64,
    /// Duty stored in the project (BOM).
    pub bom_duty: f64,
    /// qty · P_a · d
    pub active_part_mw: f64,
    /// qty · P_s · (1 − d)
    pub sleep_part_mw: f64,
    /// Load-side average (active + sleep parts).
    pub avg_mw: f64,
    /// Share of the load-side total, 0..1.
    pub share: f64,
    /// Everything active (qty · P_a).
    pub peak_mw: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SubsystemPower {
    pub id: String,
    pub name: String,
    pub avg_mw: f64,
    pub active_part_mw: f64,
    pub sleep_part_mw: f64,
    pub share: f64,
    pub components: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Budget {
    pub components: Vec<ComponentPower>,
    pub subsystems: Vec<SubsystemPower>,
    /// Σ component averages (load side of the converter).
    pub load_mw: f64,
    pub active_mw: f64,
    pub sleep_mw: f64,
    /// load / η − load
    pub conversion_loss_mw: f64,
    /// I_q · V_nom
    pub quiescent_mw: f64,
    /// Average power drawn from the battery.
    pub avg_mw: f64,
    /// Every load active, from the battery.
    pub peak_mw: f64,
    /// C · V_nom · usable fraction.
    pub energy_mwh: f64,
    /// C · V_nom (rated).
    pub rated_energy_mwh: f64,
    pub life_h: f64,
    /// Average battery current at the nominal voltage.
    pub avg_current_ma: f64,
    pub peak_current_ma: f64,
    /// Average current as a fraction of capacity (C-rate).
    pub c_rate: f64,
    pub buck_efficiency: f64,
    pub display_ref: Option<String>,
}

fn clamp_eff(e: f64) -> f64 {
    if e.is_finite() {
        e.clamp(0.05, 1.0)
    } else {
        1.0
    }
}

/// Compute the budget for a usage profile.
pub fn budget(p: &Project, profile: &UsageProfile) -> Budget {
    let eff = clamp_eff(p.design.power_rails.buck_efficiency);
    let b = &p.design.battery;
    let mut comps: Vec<ComponentPower> = p
        .components
        .iter()
        .filter(|c| c.power.active_mw > 0.0 || c.power.sleep_mw > 0.0)
        .map(|c| {
            let d = profile.duty_for(p, c);
            let q = c.qty.max(0.0);
            let a = q * c.power.active_mw.max(0.0) * d;
            let s = q * c.power.sleep_mw.max(0.0) * (1.0 - d);
            ComponentPower {
                refdes: c.refdes.clone(),
                name: c.name.clone(),
                subsystem: c.subsystem.clone(),
                location: c.location.clone(),
                qty: q,
                active_mw: c.power.active_mw,
                sleep_mw: c.power.sleep_mw,
                duty: d,
                bom_duty: c.power.duty,
                active_part_mw: a,
                sleep_part_mw: s,
                avg_mw: a + s,
                share: 0.0,
                peak_mw: q * c.power.active_mw.max(c.power.sleep_mw).max(0.0),
            }
        })
        .collect();
    let load: f64 = comps.iter().map(|c| c.avg_mw).sum();
    for c in comps.iter_mut() {
        c.share = if load > 0.0 { c.avg_mw / load } else { 0.0 };
    }
    // subsystems in project order, then any unknown ids
    let mut subs: Vec<SubsystemPower> = Vec::new();
    let mut order: Vec<String> = p.subsystems.iter().map(|s| s.id.clone()).collect();
    for c in &comps {
        if !order.contains(&c.subsystem) {
            order.push(c.subsystem.clone());
        }
    }
    for id in order {
        let members: Vec<&ComponentPower> = comps.iter().filter(|c| c.subsystem == id).collect();
        if members.is_empty() {
            continue;
        }
        let avg: f64 = members.iter().map(|c| c.avg_mw).sum();
        subs.push(SubsystemPower {
            name: p
                .subsystems
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| id.clone()),
            id,
            avg_mw: avg,
            active_part_mw: members.iter().map(|c| c.active_part_mw).sum(),
            sleep_part_mw: members.iter().map(|c| c.sleep_part_mw).sum(),
            share: if load > 0.0 { avg / load } else { 0.0 },
            components: members.iter().map(|c| c.refdes.clone()).collect(),
        });
    }
    subs.sort_by(|a, b| b.avg_mw.total_cmp(&a.avg_mw));
    let quiescent = p.design.power_rails.quiescent_ua.max(0.0) * 1e-3 * b.nominal_v;
    let avg = load / eff + quiescent;
    let peak_load: f64 = comps.iter().map(|c| c.peak_mw).sum();
    let peak = peak_load / eff + quiescent;
    let rated = b.capacity_mah.max(0.0) * b.nominal_v.max(0.0);
    let energy = rated * b.usable_fraction.clamp(0.0, 1.0);
    let vnom = b.nominal_v.max(0.1);
    Budget {
        active_mw: comps.iter().map(|c| c.active_part_mw).sum(),
        sleep_mw: comps.iter().map(|c| c.sleep_part_mw).sum(),
        components: comps,
        subsystems: subs,
        load_mw: load,
        conversion_loss_mw: load / eff - load,
        quiescent_mw: quiescent,
        avg_mw: avg,
        peak_mw: peak,
        energy_mwh: energy,
        rated_energy_mwh: rated,
        life_h: if avg > 0.0 { energy / avg } else { f64::MAX.min(1e6) },
        avg_current_ma: avg / vnom,
        peak_current_ma: peak / vnom,
        c_rate: if b.capacity_mah > 0.0 { avg / vnom / b.capacity_mah } else { 0.0 },
        buck_efficiency: eff,
        display_ref: display_ref(p),
    }
}

/// Heat dissipated in the electronics (right-temple) pod: all battery power
/// except loads that live in the battery pod. Every electrical watt ends as
/// heat; the light leaving the display is a few milliwatts at most.
pub fn pod_heat_mw(p: &Project, profile: &UsageProfile) -> f64 {
    let bud = budget(p, profile);
    let remote: f64 = bud.components.iter().filter(|c| c.location == "left_pod").map(|c| c.avg_mw).sum();
    bud.avg_mw - remote / bud.buck_efficiency
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub id: String,
    pub name: String,
    pub description: String,
    pub avg_mw: f64,
    pub life_h: f64,
    pub display_duty: f64,
    pub subsystems: Vec<SubsystemPower>,
    pub profile: UsageProfile,
}

/// Every built-in scenario evaluated on the project.
pub fn compare(p: &Project) -> Vec<ScenarioResult> {
    let disp = display_ref(p);
    scenarios()
        .into_iter()
        .map(|s| {
            let b = budget(p, &s.profile);
            ScenarioResult {
                display_duty: b
                    .components
                    .iter()
                    .find(|c| Some(&c.refdes) == disp.as_ref())
                    .map(|c| c.duty)
                    .unwrap_or(0.0),
                id: s.id,
                name: s.name,
                description: s.description,
                avg_mw: b.avg_mw,
                life_h: b.life_h,
                subsystems: b.subsystems,
                profile: s.profile,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hand_calculated_iris_budget() {
        let p = crate::bridge::diff::iris_v05();
        let b = budget(&p, &UsageProfile::default());
        // U1 21·0.25 + 0.006·0.75, U2 140·0.3 + 0.05·0.7, U3 0.9, MK1/MK2 0.9·0.25 + 0.02·0.75,
        // U4 0.004, U6 18·0.03 + 0.002·0.97, U7 0.05
        let load = 21.0 * 0.25
            + 0.006 * 0.75
            + 140.0 * 0.3
            + 0.05 * 0.7
            + 0.9
            + 2.0 * (0.9 * 0.25 + 0.02 * 0.75)
            + 0.004
            + 18.0 * 0.03
            + 0.002 * 0.97
            + 0.05;
        assert!((b.load_mw - load).abs() < 1e-9, "{} vs {load}", b.load_mw);
        let avg = load / 0.9 + 5e-3 * 3.7;
        assert!((b.avg_mw - avg).abs() < 1e-9);
        let e = 180.0 * 3.7 * 0.85;
        assert!((b.energy_mwh - e).abs() < 1e-9);
        assert!((b.life_h - e / avg).abs() < 1e-9);
        assert!((b.life_h - 10.34).abs() < 0.05, "life {}", b.life_h);
        // peak: all active
        let peak = (21.0 + 140.0 + 0.9 + 0.9 + 0.9 + 0.004 + 18.0 + 0.05) / 0.9 + 5e-3 * 3.7;
        assert!((b.peak_mw - peak).abs() < 1e-9);
        // subsystem totals add up
        let s: f64 = b.subsystems.iter().map(|s| s.avg_mw).sum();
        assert!((s - load).abs() < 1e-9);
        assert_eq!(b.subsystems[0].id, "SYS-OPT");
    }

    #[test]
    fn consistent_with_rl_fixed_policy() {
        let p = Project::iris();
        let m = crate::rl::power::model(&p, &crate::rl::power::PowerConfig::default());
        let b = budget(&p, &UsageProfile::default());
        let life_rl = m.battery_mwh / m.p_ref_mw;
        assert!((b.life_h - life_rl).abs() < 0.01, "{} vs {}", b.life_h, life_rl);
    }

    #[test]
    fn profile_override_order() {
        let p = Project::iris();
        let u2 = p.components.iter().find(|c| c.refdes == "U2").unwrap();
        let mut prof = UsageProfile::default();
        prof.subsystem_scale.insert("SYS-OPT".into(), 2.0);
        assert!((prof.duty_for(&p, u2) - 0.6).abs() < 1e-12);
        prof.component_duty.insert("U2".into(), 0.1);
        assert!((prof.duty_for(&p, u2) - 0.1).abs() < 1e-12);
        prof.display_duty = Some(1.0);
        assert_eq!(prof.duty_for(&p, u2), 1.0);
        assert_eq!(display_ref(&p).as_deref(), Some("U2"));
    }

    #[test]
    fn scenarios_rank_sensibly() {
        let p = Project::iris();
        let r = compare(&p);
        let life = |id: &str| r.iter().find(|s| s.id == id).unwrap().life_h;
        assert!(life("standby") > life("notifications"));
        assert!(life("notifications") > life("reference"));
        assert!(life("reference") > life("navigation"));
        assert!(life("navigation") > life("captions"));
        assert!(life("captions") > life("display_on"));
        assert!(life("standby") > 100.0);
    }
}
