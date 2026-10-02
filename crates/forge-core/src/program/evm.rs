//! Earned value management (EVM).
//!
//! The baseline is the CPM early-start schedule with each task's budget
//! spread linearly over its duration: the **planned value** curve
//! `PV(t) = Σ Bᵢ · clamp((t − ESᵢ)/dᵢ, 0, 1)`, ending at the budget at
//! completion (BAC). A scenario then "runs" the project: until an onset week
//! everything goes to plan; after it, work progresses at a fraction `r` of
//! the planned rate (so `EV(t) = PV(τ(t))`, `τ = t₀ + r·(t − t₀)`) and every
//! earned dollar costs `k` dollars (`AC` grows `k` times faster than `EV`).
//!
//! At the status date: cost variance `CV = EV − AC`, schedule variance
//! `SV = EV − PV`, cost performance index `CPI = EV/AC`, schedule performance
//! index `SPI = EV/PV`, and estimates at completion (EAC):
//! `BAC/CPI` (current cost efficiency continues), `AC + (BAC − EV)` (the
//! overrun was a one-off), `AC + (BAC − EV)/(CPI·SPI)` (cost and schedule
//! pressure both persist); to-complete performance index
//! `TCPI = (BAC − EV)/(BAC − AC)`. **Earned schedule** (Lipke 2003) measures
//! the schedule in time: `ES` is the time at which PV equalled today's EV,
//! `SPI(t) = ES/AT`, and the independent time estimate `IEAC(t) = PD/SPI(t)`
//! — unlike the cost-based SPI it does not drift back to 1 at the end.
//!
//! References: Q. W. Fleming & J. M. Koppelman, *Earned Value Project
//! Management*, 4th ed. (PMI, 2010); PMI, *PMBOK Guide*, 7th ed. (2021);
//! W. Lipke, "Schedule is different", *The Measurable News* (2003).

use serde::{Deserialize, Serialize};

use super::cpm::{build_net, cpm, durations};
use super::wbs::{task_budget, Config, Task};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Scenario {
    /// Status date, weeks from the start.
    pub status_week: f64,
    /// Week at which trouble starts.
    pub onset_week: f64,
    /// Progress rate after the onset (1 = as planned, 0.8 = 20 % slower).
    pub progress_rate: f64,
    /// Actual cost per earned dollar after the onset (1.15 = 15 % overrun).
    pub cost_factor: f64,
}

impl Default for Scenario {
    fn default() -> Self {
        Scenario {
            status_week: 24.0,
            onset_week: 12.0,
            progress_rate: 0.85,
            cost_factor: 1.12,
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Indices {
    pub cv: f64,
    pub sv: f64,
    pub cpi: f64,
    pub spi: f64,
    pub eac_cpi: f64,
    pub eac_atypical: f64,
    pub eac_cpi_spi: f64,
    pub etc: f64,
    pub vac: f64,
    pub tcpi: f64,
}

/// The standard EVM indices from BAC, PV, EV and AC.
pub fn indices(bac: f64, pv: f64, ev: f64, ac: f64) -> Indices {
    let cpi = if ac > 0.0 { ev / ac } else { 1.0 };
    let spi = if pv > 0.0 { ev / pv } else { 1.0 };
    let eac_cpi = if cpi > 0.0 { bac / cpi } else { f64::INFINITY };
    let eac_cpi_spi = if cpi * spi > 0.0 {
        ac + (bac - ev) / (cpi * spi)
    } else {
        f64::INFINITY
    };
    Indices {
        cv: ev - ac,
        sv: ev - pv,
        cpi,
        spi,
        eac_cpi,
        eac_atypical: ac + (bac - ev),
        eac_cpi_spi,
        etc: eac_cpi - ac,
        vac: bac - eac_cpi,
        tcpi: if bac - ac != 0.0 { (bac - ev) / (bac - ac) } else { f64::INFINITY },
    }
}

/// Planned value of the baseline at time t.
pub fn pv_at(es: &[f64], d: &[f64], b: &[f64], t: f64) -> f64 {
    let mut s = 0.0;
    for i in 0..es.len() {
        if d[i] <= 1e-12 {
            if t >= es[i] {
                s += b[i];
            }
        } else {
            s += b[i] * ((t - es[i]) / d[i]).clamp(0.0, 1.0);
        }
    }
    s
}

/// Earned schedule: the time at which PV reached `ev` (bisection; PV is non-decreasing).
pub fn earned_schedule(es: &[f64], d: &[f64], b: &[f64], ev: f64, pd: f64) -> f64 {
    if ev <= 0.0 {
        return 0.0;
    }
    let (mut lo, mut hi) = (0.0, pd);
    if pv_at(es, d, b, hi) <= ev {
        return pd;
    }
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if pv_at(es, d, b, mid) < ev {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    hi
}

#[derive(Clone, Debug, Serialize)]
pub struct Eac {
    pub name: String,
    pub formula: String,
    pub value: f64,
    pub meaning: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TaskEv {
    pub id: String,
    pub name: String,
    pub budget: f64,
    pub planned_pct: f64,
    pub earned_pct: f64,
    pub pv: f64,
    pub ev: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct EvmResult {
    pub bac: f64,
    /// Planned duration (weeks).
    pub pd: f64,
    pub scenario: Scenario,
    pub weeks: Vec<f64>,
    pub pv: Vec<f64>,
    /// EV and AC up to the status date (NaN-free: shorter arrays).
    pub ev: Vec<f64>,
    pub ac: Vec<f64>,
    /// Forecast continuation after the status date under the scenario.
    pub ev_forecast: Vec<(f64, f64)>,
    pub ac_forecast: Vec<(f64, f64)>,
    pub cpi_t: Vec<(f64, f64)>,
    pub spi_t: Vec<(f64, f64)>,
    pub spi_es_t: Vec<(f64, f64)>,
    pub at: f64,
    pub pv_now: f64,
    pub ev_now: f64,
    pub ac_now: f64,
    pub idx: Indices,
    pub es: f64,
    pub spi_time: f64,
    pub ieac_weeks: f64,
    /// What the scenario really ends at (for comparing the EAC formulas).
    pub true_finish: f64,
    pub true_cost: f64,
    pub eacs: Vec<Eac>,
    pub tasks: Vec<TaskEv>,
}

pub fn run(tasks: &[Task], cfg: &Config, sc: &Scenario) -> Result<EvmResult, String> {
    let net = build_net(tasks)?;
    let d = durations(tasks);
    let pass = cpm(&net, &d);
    let b: Vec<f64> = tasks.iter().map(|t| task_budget(t, cfg)).collect();
    let bac: f64 = b.iter().sum();
    let pd = pass.finish.max(1e-6);
    let r = sc.progress_rate.clamp(0.2, 2.0);
    let k = sc.cost_factor.clamp(0.3, 3.0);
    let t0 = sc.onset_week.clamp(0.0, pd);
    let tau = |t: f64| if t <= t0 { t } else { t0 + r * (t - t0) };
    let ev_at = |t: f64| pv_at(&pass.es, &d, &b, tau(t).min(pd));
    let ev0 = pv_at(&pass.es, &d, &b, t0);
    let ac_at = |t: f64| {
        let e = ev_at(t);
        if t <= t0 {
            e
        } else {
            ev0 + k * (e - ev0)
        }
    };
    // real end of the scenario: τ(t) = PD
    let true_finish = t0 + (pd - t0) / r;
    let true_cost = if t0 >= pd { bac } else { ev0 + k * (bac - ev0) };
    let at = sc.status_week.clamp(0.0, true_finish);
    let horizon = true_finish.max(pd).ceil();
    let step = 0.5;
    let nstep = (horizon / step).round() as usize;
    let weeks: Vec<f64> = (0..=nstep).map(|i| i as f64 * step).collect();
    let pv: Vec<f64> = weeks.iter().map(|&t| pv_at(&pass.es, &d, &b, t)).collect();
    let upto: Vec<f64> = weeks.iter().cloned().filter(|&t| t <= at + 1e-9).collect();
    let ev: Vec<f64> = upto.iter().map(|&t| ev_at(t)).collect();
    let ac: Vec<f64> = upto.iter().map(|&t| ac_at(t)).collect();
    let mut fut: Vec<f64> = vec![at];
    fut.extend(weeks.iter().cloned().filter(|&t| t > at + 1e-9 && t <= true_finish + 1e-9));
    if (fut.last().copied().unwrap_or(0.0) - true_finish).abs() > 1e-6 {
        fut.push(true_finish);
    }
    let ev_forecast = fut.iter().map(|&t| (t, ev_at(t))).collect();
    let ac_forecast = fut.iter().map(|&t| (t, ac_at(t))).collect();
    let mut cpi_t = Vec::new();
    let mut spi_t = Vec::new();
    let mut spi_es_t = Vec::new();
    for &t in weeks.iter().filter(|&&t| t > 0.5 && t <= true_finish + 1e-9) {
        let (p, e, a) = (pv_at(&pass.es, &d, &b, t), ev_at(t), ac_at(t));
        if a > 0.0 {
            cpi_t.push((t, e / a));
        }
        if p > 0.0 {
            spi_t.push((t, e / p));
        }
        spi_es_t.push((t, earned_schedule(&pass.es, &d, &b, e, pd) / t));
    }
    let (pv_now, ev_now, ac_now) = (pv_at(&pass.es, &d, &b, at), ev_at(at), ac_at(at));
    let idx = indices(bac, pv_now, ev_now, ac_now);
    let es = earned_schedule(&pass.es, &d, &b, ev_now, pd);
    let spi_time = if at > 0.0 { es / at } else { 1.0 };
    let ieac_weeks = if spi_time > 0.0 { pd / spi_time } else { f64::INFINITY };
    let eacs = vec![
        Eac {
            name: "EAC = BAC / CPI".into(),
            formula: format!("{bac:.0} / {:.3}", idx.cpi),
            value: idx.eac_cpi,
            meaning: "Typical: today's cost efficiency continues to the end.".into(),
        },
        Eac {
            name: "EAC = AC + (BAC − EV)".into(),
            formula: format!("{ac_now:.0} + ({bac:.0} − {ev_now:.0})"),
            value: idx.eac_atypical,
            meaning: "Atypical: the overrun so far was a one-off; the rest goes to plan.".into(),
        },
        Eac {
            name: "EAC = AC + (BAC − EV) / (CPI × SPI)".into(),
            formula: format!("{ac_now:.0} + {:.0} / ({:.3} × {:.3})", bac - ev_now, idx.cpi, idx.spi),
            value: idx.eac_cpi_spi,
            meaning: "Schedule pressure also costs money (overtime, expediting, standing army).".into(),
        },
    ];
    let tasks_ev = tasks
        .iter()
        .enumerate()
        .filter(|(i, _)| b[*i] > 0.0)
        .map(|(i, t)| {
            let frac = |x: f64| {
                if d[i] <= 1e-12 {
                    if x >= pass.es[i] {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    ((x - pass.es[i]) / d[i]).clamp(0.0, 1.0)
                }
            };
            let (pp, ee) = (frac(at), frac(tau(at).min(pd)));
            TaskEv {
                id: t.id.clone(),
                name: t.name.clone(),
                budget: b[i],
                planned_pct: pp * 100.0,
                earned_pct: ee * 100.0,
                pv: pp * b[i],
                ev: ee * b[i],
            }
        })
        .collect();
    Ok(EvmResult {
        bac,
        pd,
        scenario: sc.clone(),
        weeks,
        pv,
        ev,
        ac,
        ev_forecast,
        ac_forecast,
        cpi_t,
        spi_t,
        spi_es_t,
        at,
        pv_now,
        ev_now,
        ac_now,
        idx,
        es,
        spi_time,
        ieac_weeks,
        true_finish,
        true_cost,
        eacs,
        tasks: tasks_ev,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// BAC 200, PV 100, EV 80, AC 90 (hand calculation):
    /// CPI = 0.8889, SPI = 0.8, CV = −10, SV = −20, EAC(BAC/CPI) = 225,
    /// EAC(atypical) = 90 + 120 = 210, EAC(CPI·SPI) = 90 + 120/(0.8889·0.8) = 258.75,
    /// TCPI = 120/110 = 1.0909, VAC = −25.
    #[test]
    fn evm_indices_hand_calc() {
        let i = indices(200.0, 100.0, 80.0, 90.0);
        assert!((i.cpi - 80.0 / 90.0).abs() < 1e-12);
        assert!((i.spi - 0.8).abs() < 1e-12);
        assert_eq!((i.cv, i.sv), (-10.0, -20.0));
        assert!((i.eac_cpi - 225.0).abs() < 1e-9);
        assert!((i.eac_atypical - 210.0).abs() < 1e-9);
        assert!((i.eac_cpi_spi - 258.75).abs() < 1e-9);
        assert!((i.tcpi - 120.0 / 110.0).abs() < 1e-12);
        assert!((i.vac + 25.0).abs() < 1e-9);
    }

    #[test]
    fn scenario_on_plan_and_late() {
        let mk = |id: &str, d: f64, p: &[&str]| Task {
            id: id.into(),
            name: id.into(),
            optimistic: d,
            likely: d,
            pessimistic: d,
            fte: 1.0,
            owner: "X".into(),
            preds: p.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        let ts = vec![mk("A", 4.0, &[]), mk("B", 6.0, &["A"])];
        let cfg = Config {
            eng_rate_usd_h: 25.0,
            ..Default::default()
        };
        // BAC = 10 weeks × 40 h × 25 = 10 000
        let on = run(
            &ts,
            &cfg,
            &Scenario {
                status_week: 5.0,
                onset_week: 0.0,
                progress_rate: 1.0,
                cost_factor: 1.0,
            },
        )
        .unwrap();
        assert!((on.bac - 10000.0).abs() < 1e-9);
        assert!((on.idx.cpi - 1.0).abs() < 1e-12 && (on.idx.spi - 1.0).abs() < 1e-12);
        assert!((on.es - 5.0).abs() < 1e-6);
        // 20 % slower and 25 % more expensive from the start
        let late = run(
            &ts,
            &cfg,
            &Scenario {
                status_week: 5.0,
                onset_week: 0.0,
                progress_rate: 0.8,
                cost_factor: 1.25,
            },
        )
        .unwrap();
        assert!((late.ev_now - 4000.0).abs() < 1e-6, "EV = PV(4 wk) = 4000, got {}", late.ev_now);
        assert!((late.idx.cpi - 0.8).abs() < 1e-9);
        assert!((late.idx.spi - 0.8).abs() < 1e-9);
        assert!((late.es - 4.0).abs() < 1e-6 && (late.spi_time - 0.8).abs() < 1e-6);
        assert!((late.ieac_weeks - 12.5).abs() < 1e-6 && (late.true_finish - 12.5).abs() < 1e-9);
        assert!((late.true_cost - 12500.0).abs() < 1e-6 && (late.idx.eac_cpi - 12500.0).abs() < 1e-6);
    }
}
