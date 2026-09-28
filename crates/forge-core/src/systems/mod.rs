//! Systems engineering engine.
//!
//! * **Verification** – runs every lab's analyses on the project, collects
//!   [`Metric`]s and checks each requirement's acceptance criterion.
//! * **Traceability** – builds the need → requirement → subsystem → metric →
//!   lab graph and answers "what is affected if this changes?".
//! * **Trade studies** – weighted-sum decision matrices with weight
//!   sensitivity.
//! * **FMEA** – risk priority numbers (RPN = S × O × D).
//! * **Lint** – structural checks (dangling ids, unverifiable requirements).
//!
//! References: NASA Systems Engineering Handbook (SP-2016-6105 Rev 2);
//! INCOSE Systems Engineering Handbook; AIAG & VDA FMEA Handbook.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Cmp, Metric, Project, ReqKind, TradeStudy};

/// Collect metrics from every analysis module.
pub fn all_metrics(p: &Project) -> Vec<Metric> {
    let mut m = Vec::new();
    m.extend(crate::power::metrics(p));
    m.extend(crate::optics::metrics(p));
    m.extend(crate::circuit::metrics(p));
    m.extend(crate::dsp::metrics(p));
    m.extend(crate::chip::metrics(p));
    m.extend(crate::control::metrics(p));
    m.extend(crate::robotics::metrics(p));
    m.extend(crate::rl::metrics(p));
    m.extend(crate::cad::metrics(p));
    m.extend(crate::print::metrics(p));
    m.extend(crate::pcb::metrics(p));
    m.extend(crate::molding::metrics(p));
    m.extend(crate::mfg::metrics(p));
    m.extend(system_metrics(p, &m));
    m
}

/// Cross-domain metrics that combine other modules' results.
fn system_metrics(p: &Project, m: &[Metric]) -> Vec<Metric> {
    let get = |k: &str| m.iter().find(|x| x.key == k).map(|x| x.value);
    let mut out = Vec::new();
    let comp = p.component_mass_g();
    if let Some(frame) = get("cad.frame_mass_g") {
        out.push(
            Metric::new("mass.total_g", "Total wearable mass", frame + comp, "g", "systems")
                .with_note(&format!("frame {frame:.1} g + components {comp:.1} g")),
        );
    } else {
        out.push(Metric::new("mass.components_g", "Component mass", comp, "g", "systems"));
    }
    out.push(Metric::new("bom.cost_usd", "BOM cost (qty 1 prices)", p.bom_cost_usd(), "USD", "systems"));
    if let Some(inner) = get("cad.pod_inner_length_mm") {
        let margin = inner - p.design.pcb.width_mm;
        out.push(
            Metric::new("fit.pcb_pod_margin_mm", "PCB-to-pod length margin", margin, "mm", "systems")
                .with_note(&format!("pod inner {inner:.1} mm vs board {:.1} mm", p.design.pcb.width_mm)),
        );
    }
    out
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pass,
    Fail,
    /// No metric bound or metric not produced.
    Unverified,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReqResult {
    pub id: String,
    pub status: Status,
    pub value: Option<f64>,
    pub unit: String,
    /// Positive = inside the limit, as a fraction of the target (e.g. 0.2 = 20 % margin).
    pub margin: Option<f64>,
    pub source: Option<String>,
    pub criterion: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evaluation {
    pub metrics: Vec<Metric>,
    pub results: Vec<ReqResult>,
    pub pass: usize,
    pub fail: usize,
    pub unverified: usize,
}

fn criterion_text(v: &crate::project::Verification) -> String {
    let u = &v.unit;
    match v.cmp {
        Cmp::Le => format!("≤ {} {u}", fmt(v.target)),
        Cmp::Ge => format!("≥ {} {u}", fmt(v.target)),
        Cmp::Eq => format!("= {} ± {} {u}", fmt(v.target), fmt(v.target_hi.unwrap_or(0.0))),
        Cmp::Between => format!("{} … {} {u}", fmt(v.target), fmt(v.target_hi.unwrap_or(v.target))),
    }
}

fn fmt(x: f64) -> String {
    if x == x.round() && x.abs() < 1e9 {
        format!("{}", x as i64)
    } else {
        format!("{x:.3}").trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Check one value against a verification criterion. Returns (pass, margin).
pub fn check(v: &crate::project::Verification, value: f64) -> (bool, f64) {
    let t = v.target;
    let scale = |d: f64, base: f64| if base.abs() > 1e-12 { d / base.abs() } else { d };
    match v.cmp {
        Cmp::Le => (value <= t + 1e-9, scale(t - value, t)),
        Cmp::Ge => (value >= t - 1e-9, scale(value - t, t)),
        Cmp::Eq => {
            let tol = v.target_hi.unwrap_or(0.0).abs();
            let d = tol - (value - t).abs();
            (d >= -1e-9, scale(d, t))
        }
        Cmp::Between => {
            let hi = v.target_hi.unwrap_or(t);
            let d = (value - t).min(hi - value);
            (d >= -1e-9, scale(d, (hi - t).max(1e-12)))
        }
    }
}

/// Run all analyses and verify every requirement.
pub fn evaluate(p: &Project) -> Evaluation {
    let metrics = all_metrics(p);
    evaluate_with(p, metrics)
}

pub fn evaluate_with(p: &Project, metrics: Vec<Metric>) -> Evaluation {
    let by_key: BTreeMap<&str, &Metric> = metrics.iter().map(|m| (m.key.as_str(), m)).collect();
    let mut results = Vec::new();
    for r in &p.requirements {
        let v = &r.verification;
        let crit = criterion_text(v);
        let found = v.metric.as_deref().and_then(|k| by_key.get(k).copied());
        let res = match found {
            Some(m) => {
                let (ok, margin) = check(v, m.value);
                ReqResult {
                    id: r.id.clone(),
                    status: if ok { Status::Pass } else { Status::Fail },
                    value: Some(m.value),
                    unit: if v.unit.is_empty() { m.unit.clone() } else { v.unit.clone() },
                    margin: Some(margin),
                    source: Some(m.source.clone()),
                    criterion: crit,
                }
            }
            None => ReqResult {
                id: r.id.clone(),
                status: Status::Unverified,
                value: None,
                unit: v.unit.clone(),
                margin: None,
                source: None,
                criterion: crit,
            },
        };
        results.push(res);
    }
    let pass = results.iter().filter(|r| r.status == Status::Pass).count();
    let fail = results.iter().filter(|r| r.status == Status::Fail).count();
    let unverified = results.len() - pass - fail;
    Evaluation { metrics, results, pass, fail, unverified }
}

// ---------------------------------------------------------------------------
// Traceability graph
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    /// need | requirement | subsystem | metric | lab
    pub kind: String,
    /// Column index for layered layout (0..4).
    pub layer: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

pub fn trace_graph(p: &Project) -> TraceGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for n in &p.needs {
        nodes.push(Node { id: n.id.clone(), label: n.text.clone(), kind: "need".into(), layer: 0 });
    }
    let mut metric_keys = BTreeSet::new();
    for r in &p.requirements {
        nodes.push(Node { id: r.id.clone(), label: r.text.clone(), kind: "requirement".into(), layer: 1 });
        for n in &r.derives_from {
            edges.push(Edge { from: n.clone(), to: r.id.clone(), kind: "derives".into() });
        }
        for s in &r.allocated_to {
            edges.push(Edge { from: r.id.clone(), to: s.clone(), kind: "allocated".into() });
        }
        if let Some(m) = &r.verification.metric {
            metric_keys.insert(m.clone());
            edges.push(Edge { from: r.id.clone(), to: format!("m:{m}"), kind: "verified_by".into() });
        }
    }
    let mut labs = BTreeSet::new();
    for s in &p.subsystems {
        nodes.push(Node { id: s.id.clone(), label: s.name.clone(), kind: "subsystem".into(), layer: 2 });
        if !s.lab.is_empty() {
            labs.insert(s.lab.clone());
            edges.push(Edge { from: s.id.clone(), to: format!("lab:{}", s.lab), kind: "designed_in".into() });
        }
    }
    for m in &metric_keys {
        nodes.push(Node { id: format!("m:{m}"), label: m.clone(), kind: "metric".into(), layer: 3 });
        let lab = metric_lab(m);
        labs.insert(lab.clone());
        edges.push(Edge { from: format!("m:{m}"), to: format!("lab:{lab}"), kind: "computed_in".into() });
    }
    for l in &labs {
        nodes.push(Node { id: format!("lab:{l}"), label: l.clone(), kind: "lab".into(), layer: 4 });
    }
    TraceGraph { nodes, edges }
}

/// Which lab computes a metric (by key prefix).
pub fn metric_lab(key: &str) -> String {
    let prefix = key.split('.').next().unwrap_or("");
    match prefix {
        "thermal" => "power".into(),
        "mass" | "fit" | "bom" => "systems".into(),
        other => other.into(),
    }
}

/// All node ids reachable downstream **and** upstream of `id` (impact set).
pub fn impact(p: &Project, id: &str) -> Vec<String> {
    let g = trace_graph(p);
    let mut out = BTreeSet::new();
    // downstream
    let mut stack = vec![id.to_string()];
    while let Some(n) = stack.pop() {
        for e in g.edges.iter().filter(|e| e.from == n) {
            if out.insert(e.to.clone()) {
                stack.push(e.to.clone());
            }
        }
    }
    // upstream
    let mut stack = vec![id.to_string()];
    while let Some(n) = stack.pop() {
        for e in g.edges.iter().filter(|e| e.to == n) {
            if out.insert(e.from.clone()) {
                stack.push(e.from.clone());
            }
        }
    }
    out.remove(id);
    out.into_iter().collect()
}

// ---------------------------------------------------------------------------
// Trade studies
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeResult {
    pub id: String,
    /// Weighted score per option (0..10).
    pub scores: Vec<f64>,
    pub winner: usize,
    /// For each criterion: how much its weight could change (relative) before the winner flips; `None` = never.
    pub weight_sensitivity: Vec<Option<f64>>,
}

pub fn score_trade(t: &TradeStudy) -> TradeResult {
    let weights: Vec<f64> = t.criteria.iter().map(|c| c.weight).collect();
    let scores = weighted(t, &weights);
    let winner = argmax(&scores);
    let mut sens = Vec::new();
    for i in 0..weights.len() {
        let mut flip = None;
        // scan relative weight changes from -100 % to +300 % in 1 % steps
        'scan: for step in 1..=300 {
            for sign in [-1.0, 1.0] {
                let rel = sign * step as f64 / 100.0;
                if rel < -1.0 {
                    continue;
                }
                let mut w = weights.clone();
                w[i] = (w[i] * (1.0 + rel)).max(0.0);
                if argmax(&weighted(t, &w)) != winner {
                    flip = Some(rel);
                    break 'scan;
                }
            }
        }
        sens.push(flip);
    }
    TradeResult { id: t.id.clone(), scores, winner, weight_sensitivity: sens }
}

fn weighted(t: &TradeStudy, w: &[f64]) -> Vec<f64> {
    let total: f64 = w.iter().sum::<f64>().max(1e-12);
    t.options
        .iter()
        .map(|o| {
            o.scores
                .iter()
                .zip(t.criteria.iter())
                .zip(w.iter())
                .map(|((s, c), wi)| if c.higher_is_better { s * wi } else { (11.0 - s) * wi })
                .sum::<f64>()
                / total
        })
        .collect()
}

fn argmax(v: &[f64]) -> usize {
    v.iter().enumerate().fold((0, f64::MIN), |(bi, bv), (i, x)| if *x > bv { (i, *x) } else { (bi, bv) }).0
}

// ---------------------------------------------------------------------------
// FMEA
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskResult {
    pub id: String,
    pub rpn: u32,
    /// low | medium | high
    pub level: String,
}

pub fn fmea(p: &Project) -> Vec<RiskResult> {
    let mut v: Vec<RiskResult> = p
        .risks
        .iter()
        .map(|r| {
            let rpn = r.severity as u32 * r.occurrence as u32 * r.detection as u32;
            let level = if rpn >= 120 || r.severity >= 9 {
                "high"
            } else if rpn >= 60 {
                "medium"
            } else {
                "low"
            };
            RiskResult { id: r.id.clone(), rpn, level: level.into() }
        })
        .collect();
    v.sort_by_key(|r| std::cmp::Reverse(r.rpn));
    v
}

// ---------------------------------------------------------------------------
// Lint
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LintIssue {
    pub severity: String,
    pub id: String,
    pub message: String,
}

pub fn lint(p: &Project) -> Vec<LintIssue> {
    let mut out = Vec::new();
    let needs: BTreeSet<&str> = p.needs.iter().map(|n| n.id.as_str()).collect();
    let subs: BTreeSet<&str> = p.subsystems.iter().map(|s| s.id.as_str()).collect();
    let mut seen = BTreeSet::new();
    for r in &p.requirements {
        if !seen.insert(r.id.as_str()) {
            out.push(LintIssue { severity: "error".into(), id: r.id.clone(), message: "duplicate requirement id".into() });
        }
        if r.derives_from.is_empty() && r.kind != ReqKind::Goal {
            out.push(LintIssue { severity: "warning".into(), id: r.id.clone(), message: "not traced to any stakeholder need".into() });
        }
        for n in &r.derives_from {
            if !needs.contains(n.as_str()) {
                out.push(LintIssue { severity: "error".into(), id: r.id.clone(), message: format!("derives from unknown need {n}") });
            }
        }
        if r.allocated_to.is_empty() {
            out.push(LintIssue { severity: "warning".into(), id: r.id.clone(), message: "not allocated to a subsystem".into() });
        }
        for s in &r.allocated_to {
            if !subs.contains(s.as_str()) {
                out.push(LintIssue { severity: "error".into(), id: r.id.clone(), message: format!("allocated to unknown subsystem {s}") });
            }
        }
        if r.verification.metric.is_none() {
            out.push(LintIssue { severity: "info".into(), id: r.id.clone(), message: "no metric bound: verify by test/inspection".into() });
        }
        let t = r.text.to_lowercase();
        for weak in ["user-friendly", "fast", "easy", "adequate", "as appropriate", "etc", "tbd"] {
            if t.split(|c: char| !c.is_alphanumeric() && c != '-').any(|w| w == weak) {
                out.push(LintIssue {
                    severity: "warning".into(),
                    id: r.id.clone(),
                    message: format!("ambiguous word \"{weak}\" (INCOSE: requirements must be verifiable)"),
                });
            }
        }
        if !t.contains("shall") && !t.contains("should") && !t.contains("will") {
            out.push(LintIssue { severity: "info".into(), id: r.id.clone(), message: "use \"shall\" for binding requirements".into() });
        }
    }
    for n in &p.needs {
        if !p.requirements.iter().any(|r| r.derives_from.contains(&n.id)) {
            out.push(LintIssue { severity: "warning".into(), id: n.id.clone(), message: "need has no derived requirement".into() });
        }
    }
    for i in &p.interfaces {
        for end in [&i.from, &i.to] {
            if !subs.contains(end.as_str()) {
                out.push(LintIssue { severity: "error".into(), id: i.id.clone(), message: format!("interface endpoint {end} is not a subsystem") });
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

/// A Markdown verification report (used by the CLI and the fab package).
pub fn report_markdown(p: &Project, ev: &Evaluation) -> String {
    let mut s = String::new();
    s += &format!("# {} — verification report\n\n", p.meta.name);
    s += &format!("Version {} · {} requirements · **{} pass**, {} fail, {} unverified\n\n", p.meta.version, ev.results.len(), ev.pass, ev.fail, ev.unverified);
    s += "| ID | Requirement | Criterion | Value | Status | Source |\n|---|---|---|---|---|---|\n";
    for (r, res) in p.requirements.iter().zip(ev.results.iter()) {
        let val = res.value.map(|v| format!("{v:.3} {}", res.unit)).unwrap_or_else(|| "—".into());
        let st = match res.status {
            Status::Pass => "✅ pass",
            Status::Fail => "❌ fail",
            Status::Unverified => "⚪ unverified",
        };
        s += &format!("| {} | {} | {} | {} | {} | {} |\n", r.id, r.text.replace('|', "/"), res.criterion, val, st, res.source.clone().unwrap_or_default());
    }
    s += "\n## All metrics\n\n| Key | Label | Value | Note |\n|---|---|---|---|\n";
    for m in &ev.metrics {
        s += &format!("| `{}` | {} | {:.4} {} | {} |\n", m.key, m.label, m.value, m.unit, m.note.replace('|', "/"));
    }
    s
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ImpactIn {
    project: Project,
    id: String,
}

pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "systems.evaluate" => handle(input, |p: Project| Ok(evaluate(&p))),
        "systems.metrics" => handle(input, |p: Project| Ok(all_metrics(&p))),
        "systems.trace" => handle(input, |p: Project| Ok(trace_graph(&p))),
        "systems.impact" => handle(input, |i: ImpactIn| Ok(impact(&i.project, &i.id))),
        "systems.trade" => handle(input, |t: TradeStudy| Ok(score_trade(&t))),
        "systems.trades" => handle(input, |p: Project| Ok(p.trades.iter().map(score_trade).collect::<Vec<_>>())),
        "systems.fmea" => handle(input, |p: Project| Ok(fmea(&p))),
        "systems.lint" => handle(input, |p: Project| Ok(lint(&p))),
        "systems.report" => handle(input, |p: Project| {
            let ev = evaluate(&p);
            Ok(report_markdown(&p, &ev))
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Verification;

    #[test]
    fn iris_parses_and_lints_clean_of_errors() {
        let p = Project::iris();
        assert!(p.requirements.len() >= 20);
        let issues = lint(&p);
        let errors: Vec<_> = issues.iter().filter(|i| i.severity == "error").collect();
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn checks_criteria() {
        let v = Verification { cmp: Cmp::Le, target: 50.0, ..Default::default() };
        assert!(check(&v, 45.0).0);
        assert!(!check(&v, 51.0).0);
        let b = Verification { cmp: Cmp::Between, target: 1.0, target_hi: Some(3.0), ..Default::default() };
        assert!(check(&b, 2.0).0);
        assert!(!check(&b, 3.5).0);
    }

    #[test]
    fn trade_winner_and_fmea() {
        let p = Project::iris();
        let r = score_trade(&p.trades[0]);
        assert_eq!(r.winner, 0);
        let f = fmea(&p);
        assert!(f[0].rpn >= f[f.len() - 1].rpn);
    }

    #[test]
    fn impact_walks_graph() {
        let p = Project::iris();
        let imp = impact(&p, "N-2");
        assert!(imp.iter().any(|x| x == "R-002"));
        assert!(imp.iter().any(|x| x == "SYS-PWR"));
    }
}
