//! Technical budgets: mass, power, battery life and cost.
//!
//! A *technical budget* allocates a system-level resource (mass, power,
//! cost, ...) down to subsystems and tracks the **margin** between the
//! current estimate and the limit set by a requirement. The NASA Systems
//! Engineering Handbook (SP-2016-6105 Rev 2, §6.7 Technical Assessment and
//! Appendix on technical performance measures) treats budgets with margins as
//! *technical performance measures* (TPMs) reviewed at every milestone;
//! margins are expected to shrink as estimates turn into measurements (see
//! also AIAA S-120A-2015, *Mass Properties Control for Space Systems*).
//!
//! Everything here is a roll-up of `project.components` plus a few lab
//! metrics (the CAD frame mass, the manufacturing unit cost), so the budgets
//! update live with the design.
//!
//! * Mass: Σ qty × mass per component (+ the frame from the CAD lab when the
//!   mass requirement bounds the *total* wearable mass).
//! * Power: each load's average power `P = P_active · d + P_sleep · (1 − d)`
//!   summed and divided by the regulator efficiency to get battery power.
//! * Battery life: `t = C · V_nom · f_usable / P_battery`.
//! * Cost: bill of materials (BOM) at quantity-1 prices, or, when the cost
//!   requirement bounds the manufactured unit cost, the Manufacturing Hub's
//!   unit cost split into volume-priced BOM and conversion (frame, assembly,
//!   test, overhead).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Evaluation, Status};
use crate::project::{Cmp, Component, Project};

/// One line of a budget.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BudgetItem {
    pub id: String,
    pub label: String,
    pub subsystem: String,
    pub value: f64,
    /// component | frame | regulator | conversion
    pub kind: String,
}

/// A budget's share per subsystem.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubsystemShare {
    pub id: String,
    pub name: String,
    pub value: f64,
    /// Fraction of the budget total (0..1).
    pub fraction: f64,
}

/// One technical budget with its margin against a requirement.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Budget {
    pub key: String,
    pub label: String,
    pub unit: String,
    /// Roll-up estimate.
    pub total: f64,
    /// Limit from the bounding requirement.
    pub limit: Option<f64>,
    /// `le` when the total must stay below the limit, `ge` when above.
    pub cmp: Option<Cmp>,
    pub requirement: Option<String>,
    /// Metric the requirement is verified by.
    pub metric: Option<String>,
    /// Value of that metric in the live evaluation (the lab's own number).
    pub verified_value: Option<f64>,
    /// Status of the bounding requirement in the live evaluation.
    pub status: Status,
    /// Margin of the roll-up against the limit, as a fraction of the limit
    /// (positive = inside the limit).
    pub margin: Option<f64>,
    /// Same margin in the budget's unit.
    pub margin_abs: Option<f64>,
    pub items: Vec<BudgetItem>,
    pub by_subsystem: Vec<SubsystemShare>,
    pub note: String,
}

/// Per-subsystem roll-up of the component list.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubsystemRollup {
    pub id: String,
    pub name: String,
    pub components: usize,
    pub mass_g: f64,
    pub cost_usd: f64,
    pub power_mw: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Budgets {
    pub mass: Budget,
    pub power: Budget,
    pub battery_life: Budget,
    pub cost: Budget,
    pub subsystems: Vec<SubsystemRollup>,
}

/// Average power drawn by one component (mW).
pub fn component_power_mw(c: &Component) -> f64 {
    let d = c.power.duty.clamp(0.0, 1.0);
    c.qty * (c.power.active_mw * d + c.power.sleep_mw * (1.0 - d))
}

/// Σ average load power of every component (mW), before regulator losses.
pub fn load_power_mw(p: &Project) -> f64 {
    p.components.iter().map(component_power_mw).sum()
}

/// Regulator efficiency used for the roll-up (clamped to a sane range).
pub fn regulator_efficiency(p: &Project) -> f64 {
    let e = p.design.power_rails.buck_efficiency;
    if e.is_finite() && e > 0.0 {
        e.clamp(0.3, 1.0)
    } else {
        0.9
    }
}

const UNALLOCATED: &str = "(unallocated)";

fn subsystem_name(p: &Project, id: &str) -> String {
    p.subsystems.iter().find(|s| s.id == id).map(|s| s.name.clone()).unwrap_or_else(|| {
        if id.is_empty() {
            UNALLOCATED.to_string()
        } else {
            id.to_string()
        }
    })
}

fn sub_id(c: &Component) -> String {
    if c.subsystem.is_empty() {
        UNALLOCATED.into()
    } else {
        c.subsystem.clone()
    }
}

/// The first requirement verified by one of `metrics`, with its criterion.
fn bounding_requirement<'a>(p: &'a Project, metrics: &[&str]) -> Option<&'a crate::project::Requirement> {
    for m in metrics {
        if let Some(r) = p.requirements.iter().find(|r| r.verification.metric.as_deref() == Some(*m)) {
            return Some(r);
        }
    }
    None
}

/// Subsystem designed in `lab`, if any.
fn subsystem_for_lab(p: &Project, labs: &[&str]) -> Option<String> {
    labs.iter()
        .find_map(|l| p.subsystems.iter().find(|s| s.lab == *l).map(|s| s.id.clone()))
}

fn shares(p: &Project, items: &[BudgetItem], total: f64) -> Vec<SubsystemShare> {
    let mut by: BTreeMap<String, f64> = BTreeMap::new();
    for it in items {
        *by.entry(it.subsystem.clone()).or_default() += it.value;
    }
    // keep the project's subsystem order, then anything else
    let mut out = Vec::new();
    for s in &p.subsystems {
        if let Some(v) = by.remove(&s.id) {
            out.push(SubsystemShare {
                id: s.id.clone(),
                name: s.name.clone(),
                value: v,
                fraction: if total > 0.0 { v / total } else { 0.0 },
            });
        }
    }
    for (id, v) in by {
        out.push(SubsystemShare {
            name: subsystem_name(p, &id),
            id,
            value: v,
            fraction: if total > 0.0 { v / total } else { 0.0 },
        });
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn finish(
    p: &Project,
    ev: &Evaluation,
    key: &str,
    label: &str,
    unit: &str,
    total: f64,
    req: Option<&crate::project::Requirement>,
    items: Vec<BudgetItem>,
    note: String,
) -> Budget {
    let (limit, cmp, metric) = match req {
        Some(r) => {
            let v = &r.verification;
            let lim = match v.cmp {
                Cmp::Between => v.target_hi.unwrap_or(v.target),
                _ => v.target,
            };
            let cmp = match v.cmp {
                Cmp::Ge => Cmp::Ge,
                _ => Cmp::Le,
            };
            (Some(lim), Some(cmp), v.metric.clone())
        }
        None => (None, None, None),
    };
    let (margin, margin_abs) = match (limit, cmp) {
        (Some(l), Some(Cmp::Ge)) => (
            Some(if l.abs() > 1e-12 { (total - l) / l.abs() } else { total - l }),
            Some(total - l),
        ),
        (Some(l), Some(_)) => (
            Some(if l.abs() > 1e-12 { (l - total) / l.abs() } else { l - total }),
            Some(l - total),
        ),
        _ => (None, None),
    };
    let res = req.and_then(|r| ev.results.iter().find(|x| x.id == r.id));
    let status = res.map(|r| r.status).unwrap_or(Status::Unverified);
    let verified_value = res.and_then(|r| r.value);
    let by_subsystem = shares(p, &items, total);
    Budget {
        key: key.into(),
        label: label.into(),
        unit: unit.into(),
        total,
        limit,
        cmp,
        requirement: req.map(|r| r.id.clone()),
        metric,
        verified_value,
        status,
        margin,
        margin_abs,
        items,
        by_subsystem,
        note,
    }
}

fn metric(ev: &Evaluation, key: &str) -> Option<f64> {
    ev.metrics.iter().find(|m| m.key == key).map(|m| m.value)
}

/// Mass budget.
pub fn mass_budget(p: &Project, ev: &Evaluation) -> Budget {
    let req = bounding_requirement(p, &["mass.total_g", "mass.components_g"]);
    let include_frame = match req {
        Some(r) => r.verification.metric.as_deref() == Some("mass.total_g"),
        None => p.subsystems.iter().any(|s| s.lab == "cad"),
    };
    let mut items = Vec::new();
    let mut note = String::new();
    if include_frame {
        if let Some(frame) = metric(ev, "cad.frame_mass_g") {
            items.push(BudgetItem {
                id: "frame".into(),
                label: "Frame (CAD Studio mass properties)".into(),
                subsystem: subsystem_for_lab(p, &["cad", "print", "molding"]).unwrap_or_else(|| UNALLOCATED.into()),
                value: frame,
                kind: "frame".into(),
            });
            note = format!("frame {frame:.1} g from the CAD lab's divergence-theorem volume × density; ");
        }
    }
    for c in &p.components {
        let m = c.mass_g * c.qty;
        if m > 0.0 {
            items.push(BudgetItem {
                id: c.refdes.clone(),
                label: if c.qty != 1.0 {
                    format!("{} × {}", c.name, c.qty)
                } else {
                    c.name.clone()
                },
                subsystem: sub_id(c),
                value: m,
                kind: "component".into(),
            });
        }
    }
    let total: f64 = items.iter().map(|i| i.value).sum();
    note += &format!("{} component lines", p.components.len());
    finish(p, ev, "mass", "Mass", "g", total, req, items, note)
}

/// Average battery power budget.
pub fn power_budget(p: &Project, ev: &Evaluation) -> Budget {
    let req = bounding_requirement(p, &["power.avg_power_mw", "budget.avg_power_mw"]);
    let mut items = Vec::new();
    for c in &p.components {
        let pw = component_power_mw(c);
        if pw > 0.0 {
            items.push(BudgetItem {
                id: c.refdes.clone(),
                label: format!("{} ({:.0} % duty)", c.name, c.power.duty.clamp(0.0, 1.0) * 100.0),
                subsystem: sub_id(c),
                value: pw,
                kind: "component".into(),
            });
        }
    }
    let load: f64 = items.iter().map(|i| i.value).sum();
    let eff = regulator_efficiency(p);
    let loss = load / eff - load;
    if loss > 0.0 {
        items.push(BudgetItem {
            id: "reg".into(),
            label: format!("Regulator losses (η = {:.0} %)", eff * 100.0),
            subsystem: subsystem_for_lab(p, &["power", "circuit"]).unwrap_or_else(|| UNALLOCATED.into()),
            value: loss,
            kind: "regulator".into(),
        });
    }
    let total = load + loss.max(0.0);
    let note = format!("{load:.2} mW of loads ÷ {eff:.2} efficiency; the Power lab refines this with rail-by-rail models");
    finish(p, ev, "power", "Average power", "mW", total, req, items, note)
}

/// Battery-life budget (a "bigger is better" budget).
pub fn battery_life_budget(p: &Project, ev: &Evaluation, avg_mw: f64) -> Budget {
    let req = bounding_requirement(p, &["power.battery_life_h", "budget.battery_life_h"]);
    let b = &p.design.battery;
    let energy = b.capacity_mah * b.nominal_v * b.usable_fraction;
    let life = if avg_mw > 0.0 { energy / avg_mw } else { 0.0 };
    let note = format!(
        "{:.0} mAh × {:.2} V × {:.0} % usable = {energy:.0} mWh at {avg_mw:.2} mW",
        b.capacity_mah,
        b.nominal_v,
        b.usable_fraction * 100.0
    );
    finish(p, ev, "battery_life", "Battery life", "h", life, req, vec![], note)
}

/// Cost budget.
pub fn cost_budget(p: &Project, ev: &Evaluation) -> Budget {
    let req = bounding_requirement(p, &["mfg.unit_cost_usd", "bom.cost_usd", "mfg.bom_cost_usd"]);
    let unit = metric(ev, "mfg.unit_cost_usd");
    let volume_bom = metric(ev, "mfg.bom_cost_usd");
    let bom1 = p.bom_cost_usd();
    let use_unit = matches!(req.and_then(|r| r.verification.metric.as_deref()), Some("mfg.unit_cost_usd")) && unit.is_some();
    let scale = match (use_unit, volume_bom) {
        (true, Some(v)) if bom1 > 0.0 => v / bom1,
        _ => 1.0,
    };
    let mut items: Vec<BudgetItem> = p
        .components
        .iter()
        .filter(|c| c.unit_cost_usd * c.qty > 0.0)
        .map(|c| BudgetItem {
            id: c.refdes.clone(),
            label: c.name.clone(),
            subsystem: sub_id(c),
            value: c.unit_cost_usd * c.qty * scale,
            kind: "component".into(),
        })
        .collect();
    let note = if use_unit {
        let u = unit.unwrap_or(0.0);
        let bom_part: f64 = items.iter().map(|i| i.value).sum();
        let conv = u - bom_part;
        if conv > 0.0 {
            items.push(BudgetItem {
                id: "conversion".into(),
                label: "Frame, assembly, test & overhead".into(),
                subsystem: subsystem_for_lab(p, &["mfg", "molding"]).unwrap_or_else(|| UNALLOCATED.into()),
                value: conv,
                kind: "conversion".into(),
            });
        }
        format!(
            "Manufacturing Hub unit cost at {:.0} units: BOM at volume prices ({:.0} % of quantity-1) + conversion",
            p.design.manufacturing.target_quantity,
            scale * 100.0
        )
    } else {
        "bill of materials at quantity-1 prices".into()
    };
    let total: f64 = items.iter().map(|i| i.value).sum();
    finish(p, ev, "cost", "Unit cost", "USD", total, req, items, note)
}

/// Per-subsystem roll-up of mass, cost and power.
pub fn subsystem_rollup(p: &Project) -> Vec<SubsystemRollup> {
    let mut out: Vec<SubsystemRollup> = p
        .subsystems
        .iter()
        .map(|s| SubsystemRollup {
            id: s.id.clone(),
            name: s.name.clone(),
            components: 0,
            mass_g: 0.0,
            cost_usd: 0.0,
            power_mw: 0.0,
        })
        .collect();
    for c in &p.components {
        let id = sub_id(c);
        let idx = match out.iter().position(|r| r.id == id) {
            Some(i) => i,
            None => {
                out.push(SubsystemRollup {
                    name: subsystem_name(p, &id),
                    id: id.clone(),
                    components: 0,
                    mass_g: 0.0,
                    cost_usd: 0.0,
                    power_mw: 0.0,
                });
                out.len() - 1
            }
        };
        let r = &mut out[idx];
        r.components += 1;
        r.mass_g += c.mass_g * c.qty;
        r.cost_usd += c.unit_cost_usd * c.qty;
        r.power_mw += component_power_mw(c);
    }
    out
}

/// All budgets.
pub fn budgets(p: &Project, ev: &Evaluation) -> Budgets {
    let power = power_budget(p, ev);
    let battery_life = battery_life_budget(p, ev, power.total);
    Budgets {
        mass: mass_budget(p, ev),
        battery_life,
        power,
        cost: cost_budget(p, ev),
        subsystems: subsystem_rollup(p),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::evaluate;

    #[test]
    fn iris_mass_budget_matches_total_metric() {
        let p = Project::iris();
        let ev = evaluate(&p);
        let b = mass_budget(&p, &ev);
        let total = ev.metrics.iter().find(|m| m.key == "mass.total_g").unwrap().value;
        assert!((b.total - total).abs() < 1e-9, "{} vs {total}", b.total);
        assert_eq!(b.requirement.as_deref(), Some("R-001"));
        assert_eq!(b.limit, Some(50.0));
        // margin = (50 − total)/50
        assert!((b.margin.unwrap() - (50.0 - total) / 50.0).abs() < 1e-12);
        let s: f64 = b.by_subsystem.iter().map(|x| x.value).sum();
        assert!((s - b.total).abs() < 1e-9);
        assert!(b.items.iter().any(|i| i.kind == "frame" && i.subsystem == "SYS-FRM"));
    }

    #[test]
    fn power_rollup_hand_calc() {
        let p = crate::bridge::diff::iris_v05();
        // U1: 21 mW × 0.25 + 0.006 × 0.75
        let u1 = p.components.iter().find(|c| c.refdes == "U1").unwrap();
        assert!((component_power_mw(u1) - (21.0 * 0.25 + 0.006 * 0.75)).abs() < 1e-12);
        let ev = evaluate(&p);
        let b = budgets(&p, &ev);
        let load = load_power_mw(&p);
        assert!((b.power.total - load / 0.9).abs() < 1e-9);
        // 180 mAh × 3.7 V × 0.85 = 566.1 mWh
        assert!((b.battery_life.total - 566.1 / b.power.total).abs() < 1e-6);
        assert!(
            b.battery_life.total > 8.0 && b.battery_life.total < 14.0,
            "{}",
            b.battery_life.total
        );
        assert_eq!(b.battery_life.cmp, Some(Cmp::Ge));
        assert_eq!(b.power.requirement.as_deref(), Some("R-003"));
    }

    #[test]
    fn cost_budget_uses_unit_cost_when_bounded() {
        let p = Project::iris();
        let ev = evaluate(&p);
        let b = cost_budget(&p, &ev);
        assert_eq!(b.requirement.as_deref(), Some("R-022"));
        let unit = ev.metrics.iter().find(|m| m.key == "mfg.unit_cost_usd").map(|m| m.value);
        if let Some(u) = unit {
            assert!((b.total - u).abs() < 1e-6, "{} vs {u}", b.total);
            assert!(b.items.iter().any(|i| i.kind == "conversion"));
        }
        let sub = subsystem_rollup(&p);
        let c: f64 = sub.iter().map(|s| s.cost_usd).sum();
        assert!((c - p.bom_cost_usd()).abs() < 1e-9);
    }

    #[test]
    fn roll_up_metrics_are_emitted() {
        let p = Project::iris();
        let m = crate::systems::all_metrics(&p);
        for k in ["mass.components_g", "budget.avg_power_mw", "budget.battery_life_h"] {
            assert!(m.iter().any(|x| x.key == k), "missing {k}");
        }
    }
}
