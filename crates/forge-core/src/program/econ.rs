//! Unit economics and business case.
//!
//! * **Landed unit cost** at the target quantity: the Manufacturing Hub's
//!   indicative cost model (`crate::mfg` — parts with learning-curve price
//!   breaks, PCB fabrication and assembly, the frame by its cheapest process
//!   with any tooling amortised, final assembly and test, packaging, yield
//!   loss) plus logistics, a warranty/returns reserve and the certification
//!   and line-fixture spend amortised over the lifetime volume.
//! * **Price waterfall**: retail price → retailer margin on channel units and
//!   payment fees on direct-to-consumer (DTC) units → net revenue per unit →
//!   gross profit and gross margin `GM = (net − landed)/net`.
//! * **Break-even volume** `Q* = F / (net − variable − opex·net)` with F the
//!   development budget (the schedule's budget at completion, which includes
//!   prototype builds, certification fees and fixtures).
//! * **Cash flow**: development spend follows the planned-value curve until
//!   launch, then sales for the stated number of years; net present value
//!   `NPV = Σ CFₜ/(1 + r)ᵗ` (quarterly periods, r the quarterly equivalent of
//!   the annual rate) and internal rate of return (IRR, the r that makes NPV
//!   zero, found by bisection and annualised).
//! * **Sensitivity** tornado of NPV and gross margin over every assumption.
//!
//! Every number here is an assumption to edit — not a quote or a forecast.
//!
//! References: K. T. Ulrich & S. D. Eppinger, *Product Design and
//! Development* (McGraw-Hill; product development economics chapter); NASA,
//! *Cost Estimating Handbook*, version 4.0 (2015); R. A. Brealey, S. C. Myers & F. Allen,
//! *Principles of Corporate Finance* (NPV and IRR).

use serde::Serialize;

use super::wbs::{frame_choice, plans_mold, Config, Econ, Task};
use crate::project::Project;

/// Net present value of per-period cash flows (period 0 undiscounted).
pub fn npv(cfs: &[f64], rate: f64) -> f64 {
    cfs.iter().enumerate().map(|(t, c)| c / (1.0 + rate).powi(t as i32)).sum()
}

/// Internal rate of return per period by bisection; None without a sign change.
pub fn irr(cfs: &[f64]) -> Option<f64> {
    let (mut lo, mut hi) = (-0.95f64, 10.0f64);
    let (flo, fhi) = (npv(cfs, lo), npv(cfs, hi));
    if !(flo.is_finite() && fhi.is_finite()) || flo.signum() == fhi.signum() {
        return None;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        let f = npv(cfs, mid);
        if f.signum() == flo.signum() {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(0.5 * (lo + hi))
}

/// Everything the business case needs, as plain numbers (so the tornado can perturb them).
#[derive(Clone, Debug, Serialize)]
pub struct Inputs {
    pub price: f64,
    pub direct_share: f64,
    pub retail_margin: f64,
    pub payment_fee: f64,
    pub logistics: f64,
    pub warranty_rate: f64,
    pub return_handling: f64,
    pub cert_usd: f64,
    pub fixtures_usd: f64,
    pub mfg_target: f64,
    /// Manufactured unit cost at each year's volume.
    pub mfg_years: Vec<f64>,
    pub unit_cost_scale: f64,
    pub volumes: Vec<f64>,
    pub volume_scale: f64,
    /// Baseline planned value by week (cumulative development spend), index = week.
    pub dev_curve: Vec<f64>,
    pub dev_scale: f64,
    pub dev_override: Option<f64>,
    pub plan_weeks: f64,
    pub launch_week: f64,
    pub opex: f64,
    pub rate_annual: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Quarter {
    pub q: usize,
    pub label: String,
    pub units: f64,
    pub revenue: f64,
    pub cogs: f64,
    pub opex: f64,
    pub dev: f64,
    pub net: f64,
    pub cumulative: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Outcome {
    pub net_price: f64,
    pub landed: f64,
    pub variable: f64,
    pub gm_pct: f64,
    pub gross_profit: f64,
    pub contribution: f64,
    pub fixed: f64,
    pub breakeven_units: f64,
    pub lifetime_units: f64,
    pub npv: f64,
    pub irr_annual: Option<f64>,
    pub payback_weeks: Option<f64>,
    pub peak_cash_need: f64,
    pub quarters: Vec<Quarter>,
}

impl Inputs {
    pub fn net_price(&self) -> f64 {
        let ds = self.direct_share.clamp(0.0, 1.0);
        self.price * (ds * (1.0 - self.payment_fee) + (1.0 - ds) * (1.0 - self.retail_margin))
    }
    pub fn lifetime_units(&self) -> f64 {
        self.volumes.iter().sum::<f64>() * self.volume_scale
    }
    fn warranty(&self, mfg: f64) -> f64 {
        self.warranty_rate * (mfg + self.logistics + self.return_handling)
    }
    /// Variable cost per unit at a manufactured cost `mfg`.
    fn variable_at(&self, mfg: f64) -> f64 {
        let m = mfg * self.unit_cost_scale;
        m + self.logistics + self.warranty(m)
    }
    pub fn dev_total(&self) -> f64 {
        self.dev_override.unwrap_or_else(|| self.dev_curve.last().copied().unwrap_or(0.0)) * self.dev_scale
    }
    fn dev_spent_by(&self, w: f64) -> f64 {
        let l = self.launch_week.max(1.0);
        let x = (w / l).clamp(0.0, 1.0) * self.plan_weeks;
        let n = self.dev_curve.len();
        if n == 0 {
            return 0.0;
        }
        let i = (x.floor() as usize).min(n - 1);
        let j = (i + 1).min(n - 1);
        let f = x - i as f64;
        let base = self.dev_curve[i] + f * (self.dev_curve[j] - self.dev_curve[i]);
        let total = self.dev_curve[n - 1].max(1e-9);
        base / total * self.dev_total()
    }

    pub fn outcome(&self) -> Outcome {
        let net = self.net_price();
        let life = self.lifetime_units().max(1.0);
        let variable = self.variable_at(self.mfg_target);
        let landed = variable + (self.cert_usd + self.fixtures_usd) / life;
        let gp = net - landed;
        let contribution = net - variable - self.opex * net;
        let fixed = self.dev_total();
        let be = if contribution > 0.0 { fixed / contribution } else { f64::INFINITY };
        // weekly cash flow, aggregated to quarters of 13 weeks
        let years = self.volumes.len().max(1);
        let end = self.launch_week + 52.0 * years as f64;
        let nq = (end / 13.0).ceil() as usize;
        let rq = (1.0 + self.rate_annual).powf(0.25) - 1.0;
        let mut quarters = Vec::with_capacity(nq);
        let mut cum = 0.0;
        let mut payback: Option<f64> = None;
        let mut peak: f64 = 0.0;
        let mut prev_dev = 0.0;
        let mut cum_w = 0.0;
        for q in 0..nq {
            let mut qq = Quarter {
                q,
                label: format!("Y{} Q{}", q / 4 + 1, q % 4 + 1),
                units: 0.0,
                revenue: 0.0,
                cogs: 0.0,
                opex: 0.0,
                dev: 0.0,
                net: 0.0,
                cumulative: 0.0,
            };
            for k in 0..13 {
                let w0 = (q * 13 + k) as f64;
                let w1 = w0 + 1.0;
                let dv = self.dev_spent_by(w1);
                let dev = dv - prev_dev;
                prev_dev = dv;
                // selling weeks overlap [w0, w1)
                let s0 = w0.max(self.launch_week);
                let s1 = w1.min(end);
                let (mut units, mut cogs) = (0.0, 0.0);
                if s1 > s0 {
                    let y = (((s0 - self.launch_week) / 52.0).floor() as usize).min(years - 1);
                    let vol = self.volumes.get(y).copied().unwrap_or(0.0) * self.volume_scale;
                    units = vol / 52.0 * (s1 - s0);
                    let mfg = self.mfg_years.get(y).copied().unwrap_or(self.mfg_target);
                    cogs = units * self.variable_at(mfg);
                }
                let rev = units * net;
                let ox = self.opex * rev;
                let cf = rev - cogs - ox - dev;
                let before = cum_w;
                cum_w += cf;
                if payback.is_none() && before < 0.0 && cum_w >= 0.0 && w0 >= self.launch_week {
                    payback = Some(w0 + (-before / cf.max(1e-9)).clamp(0.0, 1.0));
                }
                peak = peak.min(cum_w);
                qq.units += units;
                qq.revenue += rev;
                qq.cogs += cogs;
                qq.opex += ox;
                qq.dev += dev;
                qq.net += cf;
            }
            cum += qq.net;
            qq.cumulative = cum;
            quarters.push(qq);
        }
        let cfs: Vec<f64> = quarters.iter().map(|q| q.net).collect();
        Outcome {
            net_price: net,
            landed,
            variable,
            gm_pct: if net > 0.0 { 100.0 * gp / net } else { f64::NEG_INFINITY },
            gross_profit: gp,
            contribution,
            fixed,
            breakeven_units: be,
            lifetime_units: life,
            npv: npv(&cfs, rq),
            irr_annual: irr(&cfs).map(|r| (1.0 + r).powi(4) - 1.0),
            payback_weeks: payback,
            peak_cash_need: -peak,
            quarters,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Line {
    pub name: String,
    pub usd: f64,
    /// mfg (Manufacturing Hub) | added | amortised
    pub kind: String,
    pub note: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Sens {
    pub name: String,
    pub low_label: String,
    pub high_label: String,
    pub npv_low: f64,
    pub npv_high: f64,
    pub gm_low: f64,
    pub gm_high: f64,
    pub be_low: f64,
    pub be_high: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct EconResult {
    pub target_qty: f64,
    pub lines: Vec<Line>,
    pub mfg_unit_usd: f64,
    pub waterfall: Vec<(String, f64)>,
    pub outcome: Outcome,
    /// Break-even chart: (units, cumulative profit before tax).
    pub breakeven_curve: Vec<(f64, f64)>,
    pub tornado: Vec<Sens>,
    pub inputs: Inputs,
    pub econ: Econ,
    pub frame_process: String,
    pub frame_molded: bool,
    pub crossover_qty: Option<f64>,
    pub tooling_usd: f64,
    pub plan_cert_usd: f64,
    pub notes: Vec<String>,
}

/// Resolve the economics inputs from the project, the plan and the launch week.
pub fn inputs(p: &Project, cfg: &Config, tasks: &[Task], pv_weekly: &[f64], plan_weeks: f64, launch_week: f64) -> (Inputs, Vec<Line>, f64) {
    let e = &cfg.economics;
    let q = p.design.manufacturing.target_quantity.max(1.0);
    let vols: Vec<f64> = if e.volumes.is_empty() {
        vec![q, 2.5 * q, 5.0 * q]
    } else {
        e.volumes.iter().map(|v| v.max(0.0)).collect()
    };
    let inp = crate::mfg::CostInputs::from_project(p);
    let mut a = crate::mfg::Assumptions::default();
    let mold = plans_mold(p, cfg);
    let fc = frame_choice(p);
    if mold && !fc.molded {
        a.frame_process = "molding".into();
    }
    let y = (a.yield_pct / 100.0).clamp(0.5, 1.0);
    // the plan pays for the hard tool up front (a PVT task), so take the per-unit mold amortisation out
    let tool_amort = |qq: f64| -> f64 {
        if !mold {
            return 0.0;
        }
        inp.molded
            .iter()
            .map(|pp| pp.tool_cost_usd * (qq / pp.shot_life.max(1.0)).ceil().max(1.0) / qq)
            .sum::<f64>()
            / y
    };
    let unit = |qq: f64| -> (Vec<(String, f64)>, f64) {
        let (cats, _, _) = crate::mfg::unit_cost_breakdown(p, &inp, &a, qq);
        let tot: f64 = cats.iter().map(|c| c.1).sum();
        (cats, tot - tool_amort(qq))
    };
    let (cats, mfg_target) = unit(q);
    let mfg_years: Vec<f64> = vols.iter().map(|&v| unit(v.max(1.0)).1).collect();
    let plan_cert: f64 = tasks
        .iter()
        .filter(|t| t.source.starts_with("compliance:"))
        .map(|t| t.expense_usd)
        .sum();
    let cert = e.certification_usd.unwrap_or(plan_cert);
    let life: f64 = vols.iter().sum::<f64>().max(1.0);
    let ii = Inputs {
        price: e.retail_price_usd.max(0.0),
        direct_share: e.direct_share_pct / 100.0,
        retail_margin: e.retail_margin_pct / 100.0,
        payment_fee: e.payment_fee_pct / 100.0,
        logistics: e.logistics_usd.max(0.0),
        warranty_rate: e.warranty_return_pct / 100.0,
        return_handling: e.return_handling_usd.max(0.0),
        cert_usd: cert.max(0.0),
        fixtures_usd: e.fixtures_usd.max(0.0),
        mfg_target,
        mfg_years,
        unit_cost_scale: 1.0,
        volumes: vols,
        volume_scale: 1.0,
        dev_curve: pv_weekly.to_vec(),
        dev_scale: 1.0,
        dev_override: e.dev_cost_usd,
        plan_weeks,
        launch_week,
        opex: e.opex_pct / 100.0,
        rate_annual: e.discount_rate_pct / 100.0,
    };
    let mut lines: Vec<Line> = cats
        .iter()
        .map(|(n, v)| {
            let mut v = *v;
            let mut note = "Manufacturing Hub cost model at the target quantity (indicative)".to_string();
            if n.starts_with("Frame") && mold {
                let amort = tool_amort(q) * y;
                v -= amort;
                note = format!(
                    "mold paid up front in the PVT plan (${:.0}k), so its per-unit amortisation is removed here",
                    fc.tooling_usd / 1000.0
                );
            }
            if n.starts_with("Yield") && mold {
                v -= tool_amort(q) * (1.0 - y);
            }
            Line {
                name: n.clone(),
                usd: v,
                kind: "mfg".into(),
                note,
            }
        })
        .collect();
    let m = mfg_target;
    lines.push(Line {
        name: "Logistics: freight, duties, fulfilment".into(),
        usd: ii.logistics,
        kind: "added".into(),
        note: "assumption — edit".into(),
    });
    lines.push(Line {
        name: "Warranty and returns reserve".into(),
        usd: ii.warranty(m),
        kind: "added".into(),
        note: format!(
            "{:.1} % of units × (unit ${m:.2} + logistics + ${:.0} handling) — assumption — edit",
            e.warranty_return_pct, ii.return_handling
        ),
    });
    lines.push(Line {
        name: "Certification, amortised".into(),
        usd: ii.cert_usd / life,
        kind: "amortised".into(),
        note: format!(
            "${:.0}k {} over {life:.0} lifetime units",
            ii.cert_usd / 1000.0,
            if e.certification_usd.is_some() {
                "(your figure)"
            } else {
                "(compliance plan midpoints in the schedule — indicative, not quotes)"
            }
        ),
    });
    lines.push(Line {
        name: if mold {
            "Line fixtures and test jigs, amortised (mold in the PVT plan)".into()
        } else {
            "Line fixtures and test jigs, amortised".into()
        },
        usd: ii.fixtures_usd / life,
        kind: "amortised".into(),
        note: format!("${:.0}k over {life:.0} units — assumption — edit", ii.fixtures_usd / 1000.0),
    });
    (ii, lines, plan_cert)
}

/// Full business case.
pub fn analyse(p: &Project, cfg: &Config, tasks: &[Task], pv_weekly: &[f64], plan_weeks: f64, launch_week: f64) -> EconResult {
    let (ii, lines, plan_cert) = inputs(p, cfg, tasks, pv_weekly, plan_weeks, launch_week);
    let o = ii.outcome();
    let e = &cfg.economics;
    let price = ii.price;
    let ds = ii.direct_share.clamp(0.0, 1.0);
    let waterfall = vec![
        ("Retail price".to_string(), price),
        (
            format!(
                "Retailer margin ({:.0} % on {:.0} % of units)",
                e.retail_margin_pct,
                100.0 - e.direct_share_pct
            ),
            -price * (1.0 - ds) * ii.retail_margin,
        ),
        (
            format!("Payment and shop fees ({:.1} % on direct sales)", e.payment_fee_pct),
            -price * ds * ii.payment_fee,
        ),
        ("Net revenue per unit".to_string(), o.net_price),
        ("Landed unit cost".to_string(), -o.landed),
        ("Gross profit per unit".to_string(), o.gross_profit),
    ];
    let be = o.breakeven_units;
    let qmax = if be.is_finite() {
        (be * 2.0).max(o.lifetime_units)
    } else {
        o.lifetime_units * 2.0
    };
    let breakeven_curve = (0..=40)
        .map(|k| {
            let qq = qmax * k as f64 / 40.0;
            (qq, qq * o.contribution - o.fixed)
        })
        .collect();
    // tornado
    type Mutator = Box<dyn Fn(&mut Inputs, bool)>;
    let base_launch = ii.launch_week;
    let muts: Vec<(&str, String, String, Mutator)> = vec![
        (
            "Retail price",
            format!("${:.0}", price * 0.85),
            format!("${:.0}", price * 1.15),
            Box::new(|i: &mut Inputs, h: bool| i.price *= if h { 1.15 } else { 0.85 }),
        ),
        (
            "Sales volume",
            "−40 %".into(),
            "+40 %".into(),
            Box::new(|i: &mut Inputs, h: bool| i.volume_scale = if h { 1.4 } else { 0.6 }),
        ),
        (
            "Manufactured unit cost",
            "−15 %".into(),
            "+15 %".into(),
            Box::new(|i: &mut Inputs, h: bool| i.unit_cost_scale = if h { 1.15 } else { 0.85 }),
        ),
        (
            "Direct-to-consumer share",
            format!("{:.0} %", (e.direct_share_pct - 20.0).max(0.0)),
            format!("{:.0} %", (e.direct_share_pct + 20.0).min(100.0)),
            Box::new(|i: &mut Inputs, h: bool| i.direct_share = (i.direct_share + if h { 0.2 } else { -0.2 }).clamp(0.0, 1.0)),
        ),
        (
            "Retailer margin",
            format!("{:.0} %", e.retail_margin_pct - 10.0),
            format!("{:.0} %", e.retail_margin_pct + 10.0),
            Box::new(|i: &mut Inputs, h: bool| i.retail_margin = (i.retail_margin + if h { 0.1 } else { -0.1 }).clamp(0.0, 0.95)),
        ),
        (
            "Development cost",
            "−30 %".into(),
            "+30 %".into(),
            Box::new(|i: &mut Inputs, h: bool| i.dev_scale = if h { 1.3 } else { 0.7 }),
        ),
        (
            "Launch date",
            "13 wk earlier".into(),
            "13 wk later".into(),
            Box::new(move |i: &mut Inputs, h: bool| i.launch_week = (base_launch + if h { 13.0 } else { -13.0 }).max(1.0)),
        ),
        (
            "Discount rate",
            format!("{:.0} %", (e.discount_rate_pct - 4.0).max(0.0)),
            format!("{:.0} %", e.discount_rate_pct + 4.0),
            Box::new(|i: &mut Inputs, h: bool| i.rate_annual = (i.rate_annual + if h { 0.04 } else { -0.04 }).max(0.0)),
        ),
        (
            "Warranty returns",
            format!("{:.0} %", (e.warranty_return_pct - 3.0).max(0.0)),
            format!("{:.0} %", e.warranty_return_pct + 5.0),
            Box::new(|i: &mut Inputs, h: bool| i.warranty_rate = (i.warranty_rate + if h { 0.05 } else { -0.03 }).max(0.0)),
        ),
        (
            "Sales, marketing and support",
            format!("{:.0} %", (e.opex_pct - 5.0).max(0.0)),
            format!("{:.0} %", e.opex_pct + 5.0),
            Box::new(|i: &mut Inputs, h: bool| i.opex = (i.opex + if h { 0.05 } else { -0.05 }).max(0.0)),
        ),
        (
            "Logistics per unit",
            format!("${:.1}", ii.logistics * 0.5),
            format!("${:.1}", ii.logistics * 1.5),
            Box::new(|i: &mut Inputs, h: bool| i.logistics *= if h { 1.5 } else { 0.5 }),
        ),
    ];
    let mut tornado: Vec<Sens> = muts
        .into_iter()
        .map(|(name, ll, hl, f)| {
            let mut lo = ii.clone();
            f(&mut lo, false);
            let mut hi = ii.clone();
            f(&mut hi, true);
            let (a, b) = (lo.outcome(), hi.outcome());
            Sens {
                name: name.into(),
                low_label: ll,
                high_label: hl,
                npv_low: a.npv,
                npv_high: b.npv,
                gm_low: a.gm_pct,
                gm_high: b.gm_pct,
                be_low: finite(a.breakeven_units),
                be_high: finite(b.breakeven_units),
            }
        })
        .collect();
    tornado.sort_by(|x, y| (y.npv_high - y.npv_low).abs().partial_cmp(&(x.npv_high - x.npv_low).abs()).unwrap());
    let fc = frame_choice(p);
    let mut notes = vec![
        "Every price, rate and volume on this tab is an assumption — edit it. Costs come from the Manufacturing Hub's indicative model; none of these numbers are quotes.".to_string(),
        "Gross margin uses the landed cost at the target quantity; break-even and net present value (NPV) treat the development budget as the fixed investment (it already contains certification fees, fixtures and prototype builds), so the amortised lines are not counted twice.".to_string(),
        "Pre-tax, no working capital (inventory, payment terms) and no price erosion — a first-pass business case in the style of Ulrich & Eppinger.".to_string(),
    ];
    if !fc.molded && !plans_mold(p, cfg) {
        notes.push(format!(
            "Frames are costed by {} (the Manufacturing Hub's cheapest process at {:.0} units); a {:.0}-unit lifetime volume {} the {}-unit molding crossover.",
            fc.process,
            p.design.manufacturing.target_quantity,
            o.lifetime_units,
            if fc.crossover.map(|c| o.lifetime_units >= c).unwrap_or(false) { "passes" } else { "stays below" },
            fc.crossover.map(|c| format!("{c:.0}")).unwrap_or_else(|| "—".into())
        ));
    }
    EconResult {
        target_qty: p.design.manufacturing.target_quantity,
        mfg_unit_usd: ii.mfg_target,
        lines,
        waterfall,
        breakeven_curve,
        tornado,
        econ: {
            let mut e2 = e.clone();
            e2.volumes = ii.volumes.clone();
            e2
        },
        frame_process: if plans_mold(p, cfg) && !fc.molded {
            "Injection molding (hard tool planned)".into()
        } else {
            fc.process.clone()
        },
        frame_molded: fc.molded || plans_mold(p, cfg),
        crossover_qty: fc.crossover,
        tooling_usd: fc.tooling_usd,
        plan_cert_usd: plan_cert,
        outcome: o,
        inputs: ii,
        notes,
    }
}

/// Finite stand-in for "never breaks even" (JSON has no infinity).
pub fn finite(x: f64) -> f64 {
    if x.is_finite() {
        x
    } else {
        1e9
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// NPV of [−100, 60, 60] at 10 % = −100 + 54.545 + 49.587 = 4.132;
    /// IRR solves 60x + 60x² = 100 with x = 1/(1 + r): x = (−1 + √(1 + 20/3))/2 → r = 13.066 %.
    #[test]
    fn npv_irr_hand_calc() {
        let cf = [-100.0, 60.0, 60.0];
        assert!((npv(&cf, 0.10) - 4.1322314).abs() < 1e-6);
        let x = (-1.0 + (1.0f64 + 20.0 / 3.0).sqrt()) / 2.0;
        let r = irr(&cf).unwrap();
        assert!((r - (1.0 / x - 1.0)).abs() < 1e-9, "{r}");
        assert!((r - 0.130662).abs() < 1e-5);
        assert!(npv(&cf, r).abs() < 1e-9);
        assert_eq!(irr(&[100.0, 50.0]), None);
    }

    fn simple() -> Inputs {
        Inputs {
            price: 200.0,
            direct_share: 0.5,
            retail_margin: 0.4,
            payment_fee: 0.0,
            logistics: 10.0,
            warranty_rate: 0.0,
            return_handling: 0.0,
            cert_usd: 10000.0,
            fixtures_usd: 0.0,
            mfg_target: 50.0,
            mfg_years: vec![50.0],
            unit_cost_scale: 1.0,
            volumes: vec![1000.0],
            volume_scale: 1.0,
            dev_curve: vec![0.0, 50000.0, 100000.0],
            dev_scale: 1.0,
            dev_override: None,
            plan_weeks: 2.0,
            launch_week: 2.0,
            opex: 0.0,
            rate_annual: 0.0,
        }
    }

    /// Net price = 200·(0.5·1 + 0.5·0.6) = 160; landed = 50 + 10 + 10 000/1000 = 70;
    /// GM = 90/160 = 56.25 %; contribution = 160 − 60 = 100 → break-even 100 000/100 = 1000 units;
    /// cash: −100 000 development + 1000 × 100 = 0 at a 0 % discount rate.
    #[test]
    fn unit_economics_hand_calc() {
        let o = simple().outcome();
        assert!((o.net_price - 160.0).abs() < 1e-9);
        assert!((o.landed - 70.0).abs() < 1e-9);
        assert!((o.gm_pct - 56.25).abs() < 1e-9);
        assert!((o.breakeven_units - 1000.0).abs() < 1e-9);
        assert!(o.npv.abs() < 1e-6, "{}", o.npv);
        assert!((o.peak_cash_need - 100000.0).abs() < 1e-6);
        let total_units: f64 = o.quarters.iter().map(|q| q.units).sum();
        assert!((total_units - 1000.0).abs() < 1e-6);
    }
}
