//! # Program & Business Lab engine — schedule, schedule risk, earned value, unit economics, readiness
//!
//! Turns a Protoforge product into a program you could run and a business
//! you could launch:
//!
//! * [`wbs`] — the work breakdown structure (WBS) task network, derived from
//!   the project's milestones, the Build Guide roadmap (EVT / DVT / PVT), the
//!   compliance plan and the Manufacturing Hub cost model, or edited on the
//!   page and stored in `design.extra.program`.
//! * [`cpm`] — critical path method (CPM): forward / backward pass, total and
//!   free float, driving path, CSV and Mermaid `gantt` exports.
//! * [`risk`] — three-point estimates, classic PERT, seeded Monte Carlo of
//!   the network: completion histogram, S-curve, P50 / P80, criticality
//!   index, rank-correlation sensitivity and a tornado chart.
//! * [`evm`] — earned value management: PV / EV / AC curves, CPI / SPI,
//!   estimate-at-completion (EAC) variants and earned schedule.
//! * [`econ`] — landed unit cost, price waterfall, gross margin, break-even,
//!   quarterly cash flow with NPV / IRR and a sensitivity tornado.
//! * [`readiness`] — technology and manufacturing readiness levels (TRL /
//!   MRL) per subsystem from project evidence, and SRR / PDR / CDR / TRR / PRR
//!   review gates linked to live requirement status.
//!
//! ## REFERENCES
//! * Project Management Institute, *A Guide to the Project Management Body of Knowledge (PMBOK Guide)*, 7th ed. (2021).
//! * NASA, *Systems Engineering Handbook*, SP-2016-6105 Rev 2 (2016).
//! * NASA, *Cost Estimating Handbook*, version 4.0 (2015).
//! * U.S. DoD, *Technology Readiness Assessment Guidebook*; *Manufacturing Readiness Level (MRL) Deskbook*.
//! * J. E. Kelley & M. R. Walker, "Critical-path planning and scheduling", Proc. Eastern Joint Computer Conference (1959).
//! * D. G. Malcolm, J. H. Roseboom, C. E. Clark & W. Fazar, "Application of a technique for research and development program evaluation", *Operations Research* 7(5) (1959).
//! * D. T. Hulett, *Practical Schedule Risk Analysis* (Gower, 2009).
//! * Q. W. Fleming & J. M. Koppelman, *Earned Value Project Management*, 4th ed. (PMI, 2010).
//! * W. Lipke, "Schedule is different", *The Measurable News* (2003).
//! * K. T. Ulrich & S. D. Eppinger, *Product Design and Development* (McGraw-Hill).
//! * J. C. Mankins, "Technology readiness levels: a white paper", NASA (1995).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Metric, Project};
use crate::systems::Status;

pub mod cpm;
pub mod econ;
pub mod evm;
pub mod readiness;
pub mod risk;
pub mod wbs;

pub use wbs::{Config, Econ, Evidence, Task};

// ---------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ProjectIn {
    project: Project,
    /// Edited tasks (preview before saving); None = the project's.
    #[serde(default)]
    tasks: Option<Vec<Task>>,
    #[serde(default)]
    start_date: Option<String>,
}

#[derive(Deserialize)]
struct RiskIn {
    project: Project,
    #[serde(default)]
    tasks: Option<Vec<Task>>,
    #[serde(default)]
    iterations: Option<usize>,
    #[serde(default)]
    seed: Option<u64>,
    #[serde(default)]
    dist: Option<String>,
}

#[derive(Deserialize)]
struct EvmIn {
    project: Project,
    #[serde(default)]
    tasks: Option<Vec<Task>>,
    #[serde(default)]
    scenario: evm::Scenario,
}

#[derive(Deserialize)]
struct EconIn {
    project: Project,
    #[serde(default)]
    tasks: Option<Vec<Task>>,
    #[serde(default)]
    economics: Option<Econ>,
}

#[derive(Deserialize)]
struct ResultIn {
    id: String,
    status: Status,
}

#[derive(Deserialize)]
struct ReadinessIn {
    project: Project,
    /// Live requirement results (from `systems.evaluate`); computed when absent.
    #[serde(default)]
    results: Option<Vec<ResultIn>>,
}

#[derive(Deserialize)]
#[serde(default)]
struct CpmIn {
    tasks: Vec<Task>,
    start_date: String,
}

impl Default for CpmIn {
    fn default() -> Self {
        CpmIn {
            tasks: textbook_network(),
            start_date: "2026-10-05".into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct PertIn {
    a: f64,
    m: f64,
    b: f64,
}

impl Default for PertIn {
    fn default() -> Self {
        PertIn { a: 2.0, m: 4.0, b: 12.0 }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct NpvIn {
    cashflows: Vec<f64>,
    rate: f64,
}

impl Default for NpvIn {
    fn default() -> Self {
        NpvIn {
            cashflows: vec![-100.0, 60.0, 60.0],
            rate: 0.10,
        }
    }
}

/// The hand-calculated six-activity network used in the tests and the "How it works" tab.
pub fn textbook_network() -> Vec<Task> {
    let mk = |id: &str, name: &str, d: [f64; 3], p: &[&str]| Task {
        id: id.into(),
        wbs: id.into(),
        name: name.into(),
        phase: "Example".into(),
        owner: "Team".into(),
        optimistic: d[0],
        likely: d[1],
        pessimistic: d[2],
        preds: p.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    };
    vec![
        mk("A", "Specify", [2.0, 3.0, 5.0], &[]),
        mk("B", "Order long-lead parts", [3.0, 4.0, 8.0], &[]),
        mk("C", "Design board", [1.0, 2.0, 4.0], &["A"]),
        mk("D", "Design enclosure", [4.0, 5.0, 8.0], &["A"]),
        mk("E", "Assemble electronics", [1.0, 1.0, 3.0], &["B", "C"]),
        mk("F", "Integrate and test", [1.0, 2.0, 4.0], &["D", "E"]),
    ]
}

fn resolve(p: &Project, tasks: Option<Vec<Task>>, start: Option<String>) -> (Config, Vec<Task>) {
    let mut cfg = Config::from_project(p);
    if let Some(s) = start {
        cfg.start_date = s;
    }
    let t = match tasks {
        Some(t) if !t.is_empty() => t,
        _ => cfg.tasks(p),
    };
    (cfg, t)
}

// ---------------------------------------------------------------------------
// Whole-program summary (metrics, reviews, economics launch date)
// ---------------------------------------------------------------------------

/// Cumulative planned value by week (index = week), baseline most-likely schedule.
pub fn pv_weekly(s: &cpm::Schedule) -> Vec<f64> {
    let es: Vec<f64> = s.tasks.iter().map(|t| t.es).collect();
    let d: Vec<f64> = s.tasks.iter().map(|t| t.duration).collect();
    let b: Vec<f64> = s.tasks.iter().map(|t| t.budget_usd).collect();
    let n = s.finish_weeks.ceil().max(1.0) as usize;
    (0..=n).map(|w| evm::pv_at(&es, &d, &b, w as f64)).collect()
}

#[derive(Clone, Debug, Serialize)]
pub struct Summary {
    pub cpm_weeks: f64,
    pub p50_weeks: f64,
    pub p80_weeks: f64,
    pub bac_usd: f64,
    pub launch_week: f64,
    pub gross_margin_pct: f64,
    pub breakeven_units: f64,
    pub lifetime_units: f64,
    pub npv_usd: f64,
    pub landed_usd: f64,
    pub net_price_usd: f64,
    pub min_trl: u8,
    pub min_trl_subsystems: Vec<String>,
    pub min_mrl: u8,
    pub milestone_weeks: BTreeMap<String, f64>,
    pub compliance_items: usize,
    pub critical_path: Vec<String>,
    pub error: Option<String>,
}

/// Launch week for the business case.
fn launch_week(cfg: &Config, sched: &cpm::Schedule, sorted_finishes: &[f64]) -> f64 {
    match cfg.economics.launch.to_ascii_lowercase().as_str() {
        "cpm" | "plan" => sched.finish_weeks,
        "p50" => risk::percentile(sorted_finishes, 0.5),
        _ => risk::percentile(sorted_finishes, 0.8),
    }
}

pub fn summary(p: &Project, cfg: &Config, tasks: &[Task]) -> Result<(Summary, cpm::Schedule, econ::EconResult), String> {
    let sched = cpm::schedule(Some(p), tasks, cfg)?;
    let fins = risk::finishes(tasks, cfg.iterations.clamp(200, 5000), cfg.seed, &cfg.dist)?;
    let launch = launch_week(cfg, &sched, &fins);
    let pvw = pv_weekly(&sched);
    let ec = econ::analyse(p, cfg, tasks, &pvw, sched.finish_weeks, launch);
    let sub = readiness::assess(p, cfg, None);
    let min_trl = sub.iter().map(|s| s.trl).min().unwrap_or(1);
    let min_mrl = sub.iter().map(|s| s.mrl).min().unwrap_or(1);
    let plan = crate::build::compliance::plan(p, &crate::build::compliance::project_options(p));
    let s = Summary {
        cpm_weeks: sched.finish_weeks,
        p50_weeks: risk::percentile(&fins, 0.5),
        p80_weeks: risk::percentile(&fins, 0.8),
        bac_usd: sched.bac_usd,
        launch_week: launch,
        gross_margin_pct: ec.outcome.gm_pct,
        breakeven_units: econ::finite(ec.outcome.breakeven_units),
        lifetime_units: ec.outcome.lifetime_units,
        npv_usd: ec.outcome.npv,
        landed_usd: ec.outcome.landed,
        net_price_usd: ec.outcome.net_price,
        min_trl,
        min_trl_subsystems: sub.iter().filter(|s| s.trl == min_trl).map(|s| s.id.clone()).collect(),
        min_mrl,
        milestone_weeks: sched.tasks.iter().filter(|t| t.milestone).map(|t| (t.id.clone(), t.ef)).collect(),
        compliance_items: plan.items.len(),
        critical_path: sched.critical_path.clone(),
        error: None,
    };
    Ok((s, sched, ec))
}

#[derive(Serialize)]
struct ConfigOut {
    config: Config,
    tasks: Vec<Task>,
    default_tasks: Vec<Task>,
    edited: bool,
    evidence_defaults: BTreeMap<String, Evidence>,
    stages: Vec<&'static str>,
}

#[derive(Serialize)]
struct ReadinessOut {
    subsystems: Vec<readiness::SubsystemReadiness>,
    reviews: Vec<readiness::Review>,
    min_trl: u8,
    min_mrl: u8,
    summary: Summary,
    trl_names: Vec<&'static str>,
    mrl_names: Vec<&'static str>,
}

#[derive(Serialize)]
struct PertOut {
    mean: f64,
    variance: f64,
    sd: f64,
    tri_mean: f64,
    tri_sd: f64,
    x: Vec<f64>,
    pert_pdf: Vec<f64>,
    tri_pdf: Vec<f64>,
    alpha: f64,
    beta: f64,
}

fn pert_curves(a: f64, m: f64, b: f64) -> PertOut {
    let (a, b) = (a.min(b), a.max(b));
    let m = m.clamp(a, b);
    let (mean, variance, sd) = risk::pert(a, m, b);
    let (tm, tv) = risk::tri_moments(a, m, b);
    let w = (b - a).max(1e-9);
    let al = 1.0 + 4.0 * (m - a) / w;
    let be = 1.0 + 4.0 * (b - m) / w;
    // beta pdf normalised numerically (no gamma function needed)
    let n = 121;
    let x: Vec<f64> = (0..n).map(|k| a + w * k as f64 / (n - 1) as f64).collect();
    let raw: Vec<f64> = x
        .iter()
        .map(|&xx| {
            let u = ((xx - a) / w).clamp(0.0, 1.0);
            u.powf(al - 1.0) * (1.0 - u).powf(be - 1.0)
        })
        .collect();
    let area: f64 = raw.windows(2).map(|p| 0.5 * (p[0] + p[1]) * w / (n - 1) as f64).sum();
    let pert_pdf = raw.iter().map(|v| v / area.max(1e-12)).collect();
    let tri_pdf = x
        .iter()
        .map(|&xx| {
            if xx < m {
                if m - a > 1e-12 {
                    2.0 * (xx - a) / (w * (m - a))
                } else {
                    0.0
                }
            } else if b - m > 1e-12 {
                2.0 * (b - xx) / (w * (b - m))
            } else {
                2.0 / w
            }
        })
        .collect();
    PertOut {
        mean,
        variance,
        sd,
        tri_mean: tm,
        tri_sd: tv.sqrt(),
        x,
        pert_pdf,
        tri_pdf,
        alpha: al,
        beta: be,
    }
}

// ---------------------------------------------------------------------------
// API
// ---------------------------------------------------------------------------

/// JSON API for the Program & Business Lab.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    // calculators accept a missing input (their defaults are the worked examples)
    let empty = Value::Object(Default::default());
    let input = if input.is_null() { &empty } else { input };
    match method {
        "program.config" => handle(input, |i: ProjectIn| {
            let (cfg, tasks) = resolve(&i.project, i.tasks, i.start_date);
            Ok(ConfigOut {
                edited: cfg.tasks.as_ref().map(|t| !t.is_empty()).unwrap_or(false),
                default_tasks: wbs::default_tasks(&i.project, &cfg),
                evidence_defaults: readiness::default_evidence(&i.project),
                stages: readiness::STAGES.to_vec(),
                config: cfg,
                tasks,
            })
        }),
        "program.schedule" => handle(input, |i: ProjectIn| {
            let (cfg, tasks) = resolve(&i.project, i.tasks, i.start_date);
            cpm::schedule(Some(&i.project), &tasks, &cfg)
        }),
        "program.cpm" => handle(input, |i: CpmIn| {
            let cfg = Config {
                start_date: i.start_date,
                ..Default::default()
            };
            cpm::schedule(None, &i.tasks, &cfg)
        }),
        "program.risk" => handle(input, |i: RiskIn| {
            let (cfg, tasks) = resolve(&i.project, i.tasks, None);
            risk::simulate(
                &tasks,
                i.iterations.unwrap_or(cfg.iterations),
                i.seed.unwrap_or(cfg.seed),
                i.dist.as_deref().unwrap_or(&cfg.dist),
            )
        }),
        "program.evm" => handle(input, |i: EvmIn| {
            let (cfg, tasks) = resolve(&i.project, i.tasks, None);
            evm::run(&tasks, &cfg, &i.scenario)
        }),
        "program.economics" => handle(input, |i: EconIn| {
            let (mut cfg, tasks) = resolve(&i.project, i.tasks, None);
            if let Some(e) = i.economics {
                cfg.economics = e;
            }
            summary(&i.project, &cfg, &tasks).map(|(_, _, ec)| ec)
        }),
        "program.readiness" => handle(input, |i: ReadinessIn| {
            let p = &i.project;
            let (cfg, tasks) = resolve(p, None, None);
            let status: BTreeMap<String, Status> = match i.results {
                Some(r) => r.into_iter().map(|x| (x.id, x.status)).collect(),
                None => crate::systems::evaluate(p).results.into_iter().map(|x| (x.id, x.status)).collect(),
            };
            let sub = readiness::assess(p, &cfg, Some(&status));
            let (s, _, _) = summary(p, &cfg, &tasks)?;
            let facts = readiness::ProgramFacts {
                cpm_weeks: s.cpm_weeks,
                p80_weeks: s.p80_weeks,
                gross_margin_pct: s.gross_margin_pct,
                breakeven_units: s.breakeven_units,
                lifetime_units: s.lifetime_units,
                npv_usd: s.npv_usd,
                milestone_weeks: s.milestone_weeks.clone(),
                compliance_items: s.compliance_items,
            };
            let reviews = readiness::reviews(p, &cfg, &status, &sub, &facts);
            Ok(ReadinessOut {
                min_trl: sub.iter().map(|x| x.trl).min().unwrap_or(1),
                min_mrl: sub.iter().map(|x| x.mrl).min().unwrap_or(1),
                subsystems: sub,
                reviews,
                summary: s,
                trl_names: readiness::TRL_NAMES.to_vec(),
                mrl_names: readiness::MRL_NAMES.to_vec(),
            })
        }),
        "program.summary" => handle(input, |i: ProjectIn| {
            let (cfg, tasks) = resolve(&i.project, i.tasks, i.start_date);
            summary(&i.project, &cfg, &tasks).map(|x| x.0)
        }),
        "program.pert" => handle(input, |i: PertIn| Ok(pert_curves(i.a, i.m, i.b))),
        "program.npv" => handle(input, |i: NpvIn| {
            Ok(serde_json::json!({
                "npv": econ::npv(&i.cashflows, i.rate),
                "irr": econ::irr(&i.cashflows),
            }))
        }),
        _ => None,
    }
}

thread_local! {
    static METRICS: crate::memo::Slot<Vec<Metric>> = const { std::cell::RefCell::new(None) };
}

/// Metrics feeding requirement verification (memoised on the whole project:
/// the plan reads milestones, the roadmap, the compliance plan, the cost
/// model and the requirements).
pub fn metrics(p: &Project) -> Vec<Metric> {
    crate::memo::memo(&METRICS, crate::memo::key(p), || metrics_uncached(p))
}

fn metrics_uncached(p: &Project) -> Vec<Metric> {
    let cfg = Config::from_project(p);
    let tasks = cfg.tasks(p);
    let Ok((s, sched, ec)) = summary(p, &cfg, &tasks) else {
        return vec![];
    };
    let src = "program";
    let path = s
        .critical_path
        .iter()
        .filter(|id| !id.starts_with("M-"))
        .take(8)
        .cloned()
        .collect::<Vec<_>>()
        .join(" → ");
    vec![
        Metric::new("program.p80_weeks", "80th-percentile weeks to PVT exit", s.p80_weeks, "weeks", src).with_note(&format!(
            "seeded Monte Carlo, {} {} iterations of the {}-task network; plan (most likely) {:.1} wk, P50 {:.1} wk; critical path {path}",
            cfg.iterations.clamp(200, 5000),
            if cfg.dist == "triangular" { "triangular" } else { "beta-PERT" },
            tasks.len(),
            s.cpm_weeks,
            s.p50_weeks
        )),
        Metric::new("program.cpm_weeks", "Planned weeks to PVT exit (critical path)", s.cpm_weeks, "weeks", src)
            .with_note("critical path method on most-likely durations"),
        Metric::new("program.gross_margin_pct", "Gross margin at the stated retail price", s.gross_margin_pct, "%", src).with_note(&format!(
            "${:.0} retail → ${:.2} net per unit (channel mix); landed ${:.2} at {:.0} units = Manufacturing Hub ${:.2} + logistics, warranty, amortised certification and fixtures (assumptions — edit)",
            cfg.economics.retail_price_usd, s.net_price_usd, s.landed_usd, ec.target_qty, ec.mfg_unit_usd
        )),
        Metric::new("program.breakeven_units", "Break-even volume", s.breakeven_units, "units", src).with_note(&format!(
            "development budget ${:.0}k ÷ ${:.2} contribution per unit (net − variable cost − {:.0} % sales & marketing)",
            ec.outcome.fixed / 1000.0,
            ec.outcome.contribution,
            cfg.economics.opex_pct
        )),
        Metric::new("program.npv_usd", "Net present value (development + 3 sales years)", s.npv_usd, "USD", src).with_note(&format!(
            "{:.0} % discount rate, launch at week {:.1} ({}), {:.0} lifetime units",
            cfg.economics.discount_rate_pct,
            s.launch_week,
            cfg.economics.launch,
            s.lifetime_units
        )),
        Metric::new("program.bac_usd", "Development budget at completion", sched.bac_usd, "USD", src)
            .with_note("labour at the blended rate + prototype builds + indicative certification fees (assumptions)"),
        Metric::new("program.min_trl", "Lowest technology readiness level", s.min_trl as f64, "TRL", src).with_note(&format!(
            "suggested from evidence (allocated requirements, verification method, heritage, builds demonstrated); lowest: {}",
            s.min_trl_subsystems.join(", ")
        )),
        Metric::new("program.min_mrl", "Lowest manufacturing readiness level", s.min_mrl as f64, "MRL", src)
            .with_note("suggested from evidence: cost-model coverage, manufacturability requirements, builds demonstrated"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_metrics_are_sensible() {
        let m = metrics(&Project::iris());
        let get = |k: &str| m.iter().find(|x| x.key == k).unwrap_or_else(|| panic!("{k}")).value;
        let (cpm, p80) = (get("program.cpm_weeks"), get("program.p80_weeks"));
        assert!(cpm > 30.0 && cpm < 80.0, "cpm {cpm}");
        assert!(p80 > cpm, "P80 {p80} should exceed the most-likely plan {cpm}");
        let gm = get("program.gross_margin_pct");
        assert!(gm > 20.0 && gm < 90.0, "{gm}");
        let be = get("program.breakeven_units");
        assert!(be > 100.0 && be < 1e6, "{be}");
        assert_eq!(get("program.min_trl"), 3.0);
        for x in &m {
            assert!(x.value.is_finite(), "{}", x.key);
        }
    }

    #[test]
    fn api_round_trip() {
        let p = serde_json::to_value(Project::iris()).unwrap();
        for method in ["program.config", "program.schedule", "program.summary", "program.economics"] {
            let r = crate::api::call_value(method, &serde_json::json!({ "project": p }));
            assert!(r.is_ok(), "{method}: {:?}", r.err());
        }
        let r = crate::api::call_value("program.risk", &serde_json::json!({ "project": p, "iterations": 300 })).unwrap();
        assert_eq!(r["finishes"].as_array().unwrap().len(), 300);
        let r = crate::api::call_value("program.evm", &serde_json::json!({ "project": p })).unwrap();
        assert!(r["idx"]["cpi"].as_f64().unwrap() < 1.0);
        let r = crate::api::call_value("program.cpm", &Value::Null).unwrap();
        assert_eq!(r["finish_weeks"].as_f64().unwrap(), 10.0);
        let r = crate::api::call_value("program.npv", &Value::Null).unwrap();
        assert!((r["npv"].as_f64().unwrap() - 4.1322314).abs() < 1e-6);
        let r = crate::api::call_value("program.pert", &Value::Null).unwrap();
        assert_eq!(r["mean"].as_f64().unwrap(), 5.0);
    }

    #[test]
    fn readiness_reviews_link_requirement_status() {
        let p = Project::iris();
        let mut results = vec![];
        for r in &p.requirements {
            results.push(serde_json::json!({ "id": r.id, "status": if r.id == "R-022" { "fail" } else { "pass" } }));
        }
        let out = crate::api::call_value(
            "program.readiness",
            &serde_json::json!({ "project": serde_json::to_value(&p).unwrap(), "results": results }),
        )
        .unwrap();
        let revs = out["reviews"].as_array().unwrap();
        assert_eq!(revs.len(), 5);
        let pdr = revs.iter().find(|r| r["id"] == "PDR").unwrap();
        let cost = pdr["criteria"].as_array().unwrap().iter().find(|c| c["key"] == "PDR.cost").unwrap();
        assert_eq!(cost["met"], false);
        assert!(cost["reqs"].to_string().contains("R-022"));
        assert_eq!(out["min_trl"], 3);
    }

    #[test]
    fn edited_config_round_trips_through_extra() {
        let mut p = Project::iris();
        let mut cfg = Config::default();
        let mut tasks = cfg.tasks(&p);
        tasks.iter_mut().find(|t| t.id == "T05").unwrap().likely = 7.0;
        cfg.tasks = Some(tasks);
        p.design.extra.insert("program".into(), serde_json::to_value(&cfg).unwrap());
        let back = Config::from_project(&p);
        assert_eq!(back, cfg);
        let s = cpm::schedule(Some(&p), &back.tasks(&p), &back).unwrap();
        assert!(s.tasks.iter().find(|t| t.id == "T05").unwrap().likely == 7.0);
    }
}
