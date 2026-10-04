//! Architecture and verification analyses for the Systems Studio.
//!
//! * [`n2`] – the N-squared (N²) interface matrix (R. J. Lano, "The N² Chart",
//!   TRW, 1977): subsystems on the diagonal, an interface from *i* to *j* in
//!   row *i*, column *j*. Marks above the diagonal flow forward, marks below
//!   are feedback loops. The same matrix read as a design structure matrix
//!   (D. V. Steward, "The Design Structure System", IEEE Trans. Engineering
//!   Management, 1981) can be *sequenced* to minimise feedback marks.
//! * [`coverage`] – need → requirement coverage, verification-method mix and
//!   traceability completeness.
//! * [`vmodel`] – status of each stage of the systems-engineering "V"
//!   (K. Forsberg & H. Mooz, 1991), derived from the live evaluation.
//! * [`gates`] – stage-gate milestone exit criteria that can be checked
//!   automatically against requirement status.
//! * [`trade_detail`] – weighted-sum decision matrix with contributions,
//!   the recorded decision vs the computed winner and a tornado of weight
//!   swings.
//! * [`studio`] – everything above in one call for the web app.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::budgets::{budgets, Budgets};
use super::quality::{quality_with, QualityIssue};
use super::{argmax, fmea, metric_lab, score_trade, trace_graph, weighted, Evaluation, RiskResult, Status, TraceGraph, TradeResult};
use crate::project::{Project, ReqKind, TradeStudy, VerifyMethod};

// ---------------------------------------------------------------------------
// N² matrix
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct N2Link {
    pub id: String,
    pub kind: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct N2Cell {
    /// Index of the producing subsystem (row).
    pub row: usize,
    /// Index of the receiving subsystem (column).
    pub col: usize,
    pub links: Vec<N2Link>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct N2 {
    pub ids: Vec<String>,
    pub names: Vec<String>,
    pub domains: Vec<String>,
    pub cells: Vec<N2Cell>,
    /// Interfaces leaving / entering each subsystem.
    pub fan_out: Vec<usize>,
    pub fan_in: Vec<usize>,
    /// Number of occupied cells below the diagonal in the project's order.
    pub feedback: usize,
    /// A sequencing of the subsystems (indices) that reduces feedback marks.
    pub suggested_order: Vec<usize>,
    pub suggested_feedback: usize,
    /// Interfaces whose endpoints are not subsystems.
    pub dangling: Vec<String>,
}

fn feedback_count(order: &[usize], pairs: &BTreeSet<(usize, usize)>) -> usize {
    let mut pos = vec![0usize; order.len()];
    for (k, &i) in order.iter().enumerate() {
        pos[i] = k;
    }
    pairs.iter().filter(|(a, b)| pos[*a] > pos[*b]).count()
}

/// Greedy design-structure-matrix sequencing followed by adjacent-swap
/// improvement: repeatedly schedule the subsystem with the fewest inputs
/// from still-unscheduled subsystems (ties: most outputs), which puts
/// producers before consumers and leaves the unavoidable loops as feedback.
fn sequence(n: usize, pairs: &BTreeSet<(usize, usize)>) -> Vec<usize> {
    let mut placed = vec![false; n];
    let mut order = Vec::with_capacity(n);
    for _ in 0..n {
        let mut best: Option<(usize, (usize, i64, usize))> = None;
        for i in (0..n).filter(|i| !placed[*i]) {
            let inputs = pairs.iter().filter(|(a, b)| *b == i && !placed[*a] && *a != i).count();
            let outputs = pairs.iter().filter(|(a, b)| *a == i && !placed[*b] && *b != i).count() as i64;
            let key = (inputs, -outputs, i);
            if best.as_ref().is_none_or(|(_, k)| key < *k) {
                best = Some((i, key));
            }
        }
        let (i, _) = best.expect("n > 0");
        placed[i] = true;
        order.push(i);
    }
    // local improvement: swap neighbours while it helps
    let mut cur = feedback_count(&order, pairs);
    let mut improved = true;
    let mut guard = 0;
    while improved && guard < 50 {
        improved = false;
        guard += 1;
        for k in 0..n.saturating_sub(1) {
            order.swap(k, k + 1);
            let f = feedback_count(&order, pairs);
            if f < cur {
                cur = f;
                improved = true;
            } else {
                order.swap(k, k + 1);
            }
        }
    }
    order
}

fn domain_name(d: crate::project::Domain) -> String {
    serde_json::to_value(d)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

pub fn n2(p: &Project) -> N2 {
    let ids: Vec<String> = p.subsystems.iter().map(|s| s.id.clone()).collect();
    let idx = |id: &str| ids.iter().position(|x| x == id);
    let n = ids.len();
    let mut cells: Vec<N2Cell> = Vec::new();
    let mut dangling = Vec::new();
    let mut pairs = BTreeSet::new();
    for i in &p.interfaces {
        let (Some(a), Some(b)) = (idx(&i.from), idx(&i.to)) else {
            dangling.push(i.id.clone());
            continue;
        };
        let link = N2Link {
            id: i.id.clone(),
            kind: i.kind.clone(),
            description: i.description.clone(),
        };
        if a != b {
            pairs.insert((a, b));
        }
        match cells.iter_mut().find(|c| c.row == a && c.col == b) {
            Some(c) => c.links.push(link),
            None => cells.push(N2Cell {
                row: a,
                col: b,
                links: vec![link],
            }),
        }
    }
    cells.sort_by_key(|c| (c.row, c.col));
    let fan_out = (0..n)
        .map(|k| cells.iter().filter(|c| c.row == k).map(|c| c.links.len()).sum())
        .collect();
    let fan_in = (0..n)
        .map(|k| cells.iter().filter(|c| c.col == k).map(|c| c.links.len()).sum())
        .collect();
    let natural: Vec<usize> = (0..n).collect();
    let feedback = feedback_count(&natural, &pairs);
    let suggested_order = if n > 0 { sequence(n, &pairs) } else { vec![] };
    let suggested_feedback = feedback_count(&suggested_order, &pairs);
    N2 {
        names: p.subsystems.iter().map(|s| s.name.clone()).collect(),
        domains: p.subsystems.iter().map(|s| domain_name(s.domain)).collect(),
        ids,
        cells,
        fan_out,
        fan_in,
        feedback,
        suggested_order,
        suggested_feedback,
        dangling,
    }
}

// ---------------------------------------------------------------------------
// Coverage
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NeedCoverage {
    pub id: String,
    pub text: String,
    pub stakeholder: String,
    pub priority: u8,
    pub requirements: Vec<String>,
    pub pass: usize,
    pub fail: usize,
    pub unverified: usize,
    /// satisfied | failing | partial | unverified | uncovered
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubsystemCoverage {
    pub id: String,
    pub name: String,
    pub lab: String,
    pub requirements: Vec<String>,
    pub pass: usize,
    pub fail: usize,
    pub unverified: usize,
    pub interfaces: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Count {
    pub key: String,
    pub count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Coverage {
    pub needs: Vec<NeedCoverage>,
    pub subsystems: Vec<SubsystemCoverage>,
    pub needs_total: usize,
    pub needs_covered: usize,
    pub needs_satisfied: usize,
    pub reqs_total: usize,
    pub reqs_traced: usize,
    pub reqs_allocated: usize,
    pub reqs_with_metric: usize,
    /// Requirements whose bound metric is actually produced by a lab.
    pub reqs_metric_found: usize,
    pub methods: Vec<Count>,
    pub kinds: Vec<Count>,
    /// Requirements not derived from any need.
    pub untraced: Vec<String>,
    pub unallocated: Vec<String>,
    pub no_metric: Vec<String>,
    /// Bound to a metric that no lab produces.
    pub missing_metric: Vec<String>,
    /// Subsystems with no allocated requirement.
    pub empty_subsystems: Vec<String>,
    /// Mean of need coverage, tracing, allocation and metric binding (0..1).
    pub completeness: f64,
}

fn status_of(ev: &Evaluation, id: &str) -> Status {
    ev.results
        .iter()
        .find(|r| r.id == id)
        .map(|r| r.status)
        .unwrap_or(Status::Unverified)
}

fn tally(ev: &Evaluation, ids: &[String]) -> (usize, usize, usize) {
    let mut t = (0, 0, 0);
    for id in ids {
        match status_of(ev, id) {
            Status::Pass => t.0 += 1,
            Status::Fail => t.1 += 1,
            Status::Unverified => t.2 += 1,
        }
    }
    t
}

fn enum_name<T: Serialize>(v: T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

fn frac(a: usize, b: usize) -> f64 {
    if b == 0 {
        0.0
    } else {
        a as f64 / b as f64
    }
}

pub fn coverage(p: &Project, ev: &Evaluation) -> Coverage {
    let needs: Vec<NeedCoverage> = p
        .needs
        .iter()
        .map(|n| {
            let reqs: Vec<String> = p
                .requirements
                .iter()
                .filter(|r| r.derives_from.contains(&n.id))
                .map(|r| r.id.clone())
                .collect();
            let (pass, fail, unverified) = tally(ev, &reqs);
            let status = if reqs.is_empty() {
                "uncovered"
            } else if fail > 0 {
                "failing"
            } else if pass == reqs.len() {
                "satisfied"
            } else if unverified == reqs.len() {
                "unverified"
            } else {
                "partial"
            };
            NeedCoverage {
                id: n.id.clone(),
                text: n.text.clone(),
                stakeholder: n.stakeholder.clone(),
                priority: n.priority,
                requirements: reqs,
                pass,
                fail,
                unverified,
                status: status.into(),
            }
        })
        .collect();
    let subsystems: Vec<SubsystemCoverage> = p
        .subsystems
        .iter()
        .map(|s| {
            let reqs: Vec<String> = p
                .requirements
                .iter()
                .filter(|r| r.allocated_to.contains(&s.id))
                .map(|r| r.id.clone())
                .collect();
            let (pass, fail, unverified) = tally(ev, &reqs);
            SubsystemCoverage {
                id: s.id.clone(),
                name: s.name.clone(),
                lab: s.lab.clone(),
                interfaces: p.interfaces.iter().filter(|i| i.from == s.id || i.to == s.id).count(),
                requirements: reqs,
                pass,
                fail,
                unverified,
            }
        })
        .collect();
    let keys: BTreeSet<&str> = ev.metrics.iter().map(|m| m.key.as_str()).collect();
    let untraced: Vec<String> = p
        .requirements
        .iter()
        .filter(|r| r.derives_from.is_empty())
        .map(|r| r.id.clone())
        .collect();
    let unallocated: Vec<String> = p
        .requirements
        .iter()
        .filter(|r| r.allocated_to.is_empty())
        .map(|r| r.id.clone())
        .collect();
    let no_metric: Vec<String> = p
        .requirements
        .iter()
        .filter(|r| r.verification.metric.is_none())
        .map(|r| r.id.clone())
        .collect();
    let missing_metric: Vec<String> = p
        .requirements
        .iter()
        .filter(|r| r.verification.metric.as_deref().is_some_and(|m| !keys.contains(m)))
        .map(|r| r.id.clone())
        .collect();
    let mut methods = Vec::new();
    for m in [
        VerifyMethod::Analysis,
        VerifyMethod::Test,
        VerifyMethod::Inspection,
        VerifyMethod::Demonstration,
    ] {
        let count = p.requirements.iter().filter(|r| r.verification.method == m).count();
        methods.push(Count { key: enum_name(m), count });
    }
    let mut kinds = Vec::new();
    for k in [
        ReqKind::Performance,
        ReqKind::Functional,
        ReqKind::Interface,
        ReqKind::Environmental,
        ReqKind::Safety,
        ReqKind::Regulatory,
        ReqKind::Manufacturing,
        ReqKind::Cost,
        ReqKind::Goal,
    ] {
        let count = p.requirements.iter().filter(|r| r.kind == k).count();
        if count > 0 {
            kinds.push(Count { key: enum_name(k), count });
        }
    }
    let n = p.requirements.len();
    let needs_covered = needs.iter().filter(|x| !x.requirements.is_empty()).count();
    let needs_satisfied = needs.iter().filter(|x| x.status == "satisfied").count();
    let reqs_traced = n - untraced.len();
    let reqs_allocated = n - unallocated.len();
    let reqs_with_metric = n - no_metric.len();
    let completeness = if n == 0 && p.needs.is_empty() {
        0.0
    } else {
        (frac(needs_covered, p.needs.len()) + frac(reqs_traced, n) + frac(reqs_allocated, n) + frac(reqs_with_metric, n)) / 4.0
    };
    Coverage {
        empty_subsystems: subsystems
            .iter()
            .filter(|s| s.requirements.is_empty())
            .map(|s| s.id.clone())
            .collect(),
        needs_total: p.needs.len(),
        needs_covered,
        needs_satisfied,
        reqs_total: n,
        reqs_traced,
        reqs_allocated,
        reqs_with_metric,
        reqs_metric_found: reqs_with_metric - missing_metric.len(),
        needs,
        subsystems,
        methods,
        kinds,
        untraced,
        unallocated,
        no_metric,
        missing_metric,
        completeness,
    }
}

// ---------------------------------------------------------------------------
// V-model
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VItem {
    pub label: String,
    /// pass | fail | todo | info
    pub status: String,
    pub detail: String,
    /// `tab:<id>`, `lab:<path>` or `req:<id>` for the web app to link.
    #[serde(default)]
    pub link: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VStage {
    pub id: String,
    pub title: String,
    /// left | bottom | right
    pub leg: String,
    /// 0 = top (concept / validation) … 4 = bottom (implementation).
    pub level: u8,
    /// done | partial | attention | empty
    pub status: String,
    pub progress: f64,
    pub summary: String,
    pub items: Vec<VItem>,
    pub pairs_with: Option<String>,
    pub link: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VModel {
    pub stages: Vec<VStage>,
    /// Mean progress over all stages (0..1).
    pub overall: f64,
}

fn item(label: &str, ok: bool, detail: String, link: Option<String>) -> VItem {
    VItem {
        label: label.into(),
        status: if ok { "pass" } else { "todo" }.into(),
        detail,
        link,
    }
}

fn left_status(progress: f64, attention: bool) -> String {
    if attention {
        "attention"
    } else if progress >= 0.999 {
        "done"
    } else if progress <= 0.0 {
        "empty"
    } else {
        "partial"
    }
    .into()
}

#[allow(clippy::too_many_arguments)]
fn right_stage(
    id: &str,
    title: &str,
    level: u8,
    pairs: &str,
    link: &str,
    ev: &Evaluation,
    ids: &[String],
    what: &str,
    p: &Project,
) -> VStage {
    let (pass, fail, unv) = tally(ev, ids);
    let n = ids.len();
    let status = if n == 0 {
        "empty"
    } else if fail > 0 {
        "attention"
    } else if pass == n {
        "done"
    } else {
        "partial"
    };
    let items = ids
        .iter()
        .map(|id| {
            let r = p.requirements.iter().find(|r| &r.id == id);
            let res = ev.results.iter().find(|x| &x.id == id);
            let st = match res.map(|x| x.status) {
                Some(Status::Pass) => "pass",
                Some(Status::Fail) => "fail",
                _ => "todo",
            };
            VItem {
                label: id.clone(),
                status: st.into(),
                detail: r.map(|r| r.text.clone()).unwrap_or_default(),
                link: Some(format!("req:{id}")),
            }
        })
        .collect();
    VStage {
        id: id.into(),
        title: title.into(),
        leg: "right".into(),
        level,
        status: status.into(),
        progress: frac(pass, n),
        summary: if n == 0 {
            format!("no {what} yet")
        } else {
            format!("{pass}/{n} {what} pass · {fail} fail · {unv} unverified")
        },
        items,
        pairs_with: Some(pairs.into()),
        link: link.into(),
    }
}

/// Implementation evidence: (label, metric keys that prove the artefact
/// exists, lab path, labs whose use by a subsystem makes the item relevant).
type ImplItem = (&'static str, &'static [&'static str], &'static str, &'static [&'static str]);
const IMPLEMENTATION: &[ImplItem] = &[
    (
        "3D-printed parts (slicer toolpaths, G-code)",
        &["print.total_time_h"],
        "/print",
        &["print", "cad"],
    ),
    (
        "PCB fabrication files (routed, DRC-clean)",
        &["pcb.routed_pct", "pcb.drc_violations"],
        "/pcb",
        &["pcb"],
    ),
    (
        "Silicon / register-transfer level (RTL) closure",
        &["chip.critical_path_ns", "chip.est_cells"],
        "/chip",
        &["chip"],
    ),
    (
        "Firmware & signal processing chain",
        &["dsp.decim_stopband_db", "control.head_rms_err_deg", "rl.policy_life_gain_pct"],
        "/dsp",
        &["dsp", "control", "rl"],
    ),
    (
        "Injection-molded parts (draft, clamp force)",
        &["molding.min_draft_deg"],
        "/molding",
        &["molding"],
    ),
    (
        "Factory costing & assembly",
        &["mfg.unit_cost_usd", "robotics.pnp_cycle_s"],
        "/mfg",
        &["mfg", "robotics"],
    ),
];

pub fn vmodel(p: &Project, ev: &Evaluation) -> VModel {
    let cov = coverage(p, ev);
    let q = quality_with(p, Some(&ev.metrics));
    let errors = q.iter().filter(|i| i.severity == "error").count();
    let c = &p.concept;
    let mut stages = Vec::new();

    // --- left leg ---------------------------------------------------------
    let conops_items = vec![
        item(
            "Problem statement",
            !c.problem.trim().is_empty(),
            trunc(&c.problem),
            Some("tab:concept".into()),
        ),
        item("Vision", !c.vision.trim().is_empty(), trunc(&c.vision), Some("tab:concept".into())),
        item(
            "Users",
            !c.users.is_empty(),
            format!("{} user groups", c.users.len()),
            Some("tab:concept".into()),
        ),
        item(
            "Use cases / operational scenarios",
            !c.use_cases.is_empty(),
            format!("{} use cases", c.use_cases.len()),
            Some("tab:concept".into()),
        ),
        item(
            "Stakeholder needs",
            !p.needs.is_empty(),
            format!("{} needs", p.needs.len()),
            Some("tab:requirements".into()),
        ),
    ];
    let prog = frac(conops_items.iter().filter(|i| i.status == "pass").count(), conops_items.len());
    stages.push(VStage {
        id: "conops".into(),
        title: "Concept of operations".into(),
        leg: "left".into(),
        level: 0,
        status: left_status(prog, false),
        progress: prog,
        summary: format!("{} needs · {} use cases", p.needs.len(), c.use_cases.len()),
        items: conops_items,
        pairs_with: Some("validation".into()),
        link: "tab:concept".into(),
    });

    let n = cov.reqs_total;
    let req_items = vec![
        item(
            "Requirements written",
            n > 0,
            format!("{n} requirements"),
            Some("tab:requirements".into()),
        ),
        item(
            "Traced to needs",
            n > 0 && cov.reqs_traced == n,
            format!("{}/{n} derive from a need", cov.reqs_traced),
            Some("tab:traceability".into()),
        ),
        item(
            "Every need covered",
            cov.needs_total > 0 && cov.needs_covered == cov.needs_total,
            format!("{}/{} needs have requirements", cov.needs_covered, cov.needs_total),
            Some("tab:traceability".into()),
        ),
        item(
            "Verification criterion bound",
            n > 0 && cov.reqs_with_metric == n,
            format!("{}/{n} bound to a lab metric", cov.reqs_with_metric),
            Some("tab:requirements".into()),
        ),
        item(
            "Well-formed (no lint errors)",
            n > 0 && errors == 0,
            format!("{errors} errors"),
            Some("tab:requirements".into()),
        ),
    ];
    let prog = if n == 0 {
        0.0
    } else {
        (frac(cov.reqs_traced, n)
            + frac(cov.needs_covered, cov.needs_total.max(1))
            + frac(cov.reqs_with_metric, n)
            + if errors == 0 { 1.0 } else { 0.0 })
            / 4.0
    };
    stages.push(VStage {
        id: "requirements".into(),
        title: "System requirements".into(),
        leg: "left".into(),
        level: 1,
        status: left_status(prog, errors > 0),
        progress: prog,
        summary: format!("{n} requirements · {:.0} % traced", frac(cov.reqs_traced, n) * 100.0),
        items: req_items,
        pairs_with: Some("system".into()),
        link: "tab:requirements".into(),
    });

    let ns = p.subsystems.len();
    let n2m = n2(p);
    let with_lab = p.subsystems.iter().filter(|s| !s.lab.is_empty()).count();
    let arch_items = vec![
        item(
            "Subsystems defined",
            ns > 0,
            format!("{ns} subsystems"),
            Some("tab:architecture".into()),
        ),
        item(
            "Interfaces defined",
            !p.interfaces.is_empty() && n2m.dangling.is_empty(),
            format!("{} interfaces, {} dangling", p.interfaces.len(), n2m.dangling.len()),
            Some("tab:architecture".into()),
        ),
        item(
            "Requirements allocated",
            n > 0 && cov.reqs_allocated == n,
            format!("{}/{n} allocated", cov.reqs_allocated),
            Some("tab:traceability".into()),
        ),
        item(
            "Every subsystem owns a requirement",
            ns > 0 && cov.empty_subsystems.is_empty(),
            format!("{} without requirements", cov.empty_subsystems.len()),
            Some("tab:traceability".into()),
        ),
        item(
            "Each subsystem has a design lab",
            ns > 0 && with_lab == ns,
            format!("{with_lab}/{ns} mapped to a lab"),
            Some("tab:architecture".into()),
        ),
    ];
    let prog = frac(arch_items.iter().filter(|i| i.status == "pass").count(), arch_items.len());
    stages.push(VStage {
        id: "architecture".into(),
        title: "Architecture".into(),
        leg: "left".into(),
        level: 2,
        status: left_status(prog, !n2m.dangling.is_empty()),
        progress: prog,
        summary: format!("{ns} subsystems · {} interfaces", p.interfaces.len()),
        items: arch_items,
        pairs_with: Some("integration".into()),
        link: "tab:architecture".into(),
    });

    // labs used by the architecture, in first-use order
    let mut labs: Vec<String> = Vec::new();
    for s in &p.subsystems {
        if !s.lab.is_empty() && !labs.contains(&s.lab) {
            labs.push(s.lab.clone());
        }
    }
    let lab_metrics = |lab: &str| ev.metrics.iter().filter(|m| metric_lab(&m.key) == lab).count();
    let design_items: Vec<VItem> = labs
        .iter()
        .map(|l| {
            let k = lab_metrics(l);
            item(l, k > 0, format!("{k} metrics computed"), Some(format!("lab:/{l}")))
        })
        .collect();
    let prog = frac(design_items.iter().filter(|i| i.status == "pass").count(), design_items.len());
    stages.push(VStage {
        id: "design".into(),
        title: "Detailed design (the labs)".into(),
        leg: "left".into(),
        level: 3,
        status: left_status(prog, false),
        progress: prog,
        summary: format!("{} metrics from {} labs", ev.metrics.len(), labs.len()),
        items: design_items,
        pairs_with: Some("unit".into()),
        link: "tab:overview".into(),
    });

    // --- bottom -----------------------------------------------------------
    let keys: BTreeSet<&str> = ev.metrics.iter().map(|m| m.key.as_str()).collect();
    let mut impl_items = Vec::new();
    for (label, ms, path, item_labs) in IMPLEMENTATION {
        let present = ms.iter().filter(|m| keys.contains(**m)).count();
        let used = p.subsystems.iter().any(|s| item_labs.contains(&s.lab.as_str()))
            || p.requirements
                .iter()
                .any(|r| r.verification.metric.as_deref().is_some_and(|m| ms.contains(&m)));
        if present == 0 || !used {
            continue;
        }
        // any requirement on these metrics failing?
        let reqs: Vec<String> = p
            .requirements
            .iter()
            .filter(|r| r.verification.metric.as_deref().is_some_and(|m| ms.contains(&m)))
            .map(|r| r.id.clone())
            .collect();
        let (pass, fail, _) = tally(ev, &reqs);
        let status = if fail > 0 {
            "fail"
        } else if !reqs.is_empty() && pass == reqs.len() {
            "pass"
        } else {
            "info"
        };
        impl_items.push(VItem {
            label: (*label).into(),
            status: status.into(),
            detail: if reqs.is_empty() {
                format!("{present} artefact metrics available")
            } else {
                format!("{pass}/{} related requirements pass", reqs.len())
            },
            link: Some(format!("lab:{path}")),
        });
    }
    let done = impl_items.iter().filter(|i| i.status != "fail").count();
    let prog = frac(done, impl_items.len());
    stages.push(VStage {
        id: "implementation".into(),
        title: "Implementation".into(),
        leg: "bottom".into(),
        level: 4,
        status: left_status(prog, impl_items.iter().any(|i| i.status == "fail")),
        progress: prog,
        summary: "print · PCB fab · silicon · firmware · tooling".into(),
        items: impl_items,
        pairs_with: None,
        link: "lab:/mfg".into(),
    });

    // --- right leg --------------------------------------------------------
    let unit_ids: Vec<String> = p
        .requirements
        .iter()
        .filter(|r| r.allocated_to.len() == 1 && r.kind != ReqKind::Interface)
        .map(|r| r.id.clone())
        .collect();
    stages.push(right_stage(
        "unit",
        "Unit verification (lab metrics)",
        3,
        "design",
        "tab:requirements",
        ev,
        &unit_ids,
        "single-subsystem requirements",
        p,
    ));
    let int_ids: Vec<String> = p
        .requirements
        .iter()
        .filter(|r| r.allocated_to.len() > 1 || r.kind == ReqKind::Interface)
        .map(|r| r.id.clone())
        .collect();
    stages.push(right_stage(
        "integration",
        "Integration & interfaces",
        2,
        "architecture",
        "tab:architecture",
        ev,
        &int_ids,
        "cross-subsystem requirements",
        p,
    ));
    let all_ids: Vec<String> = p.requirements.iter().map(|r| r.id.clone()).collect();
    stages.push(right_stage(
        "system",
        "System verification",
        1,
        "requirements",
        "tab:requirements",
        ev,
        &all_ids,
        "requirements",
        p,
    ));

    let val_items: Vec<VItem> = cov
        .needs
        .iter()
        .map(|nc| VItem {
            label: nc.id.clone(),
            status: match nc.status.as_str() {
                "satisfied" => "pass",
                "failing" => "fail",
                _ => "todo",
            }
            .into(),
            detail: format!("{} — {}", nc.text, nc.status),
            link: Some("tab:traceability".into()),
        })
        .collect();
    let nn = cov.needs_total;
    let failing = cov.needs.iter().any(|x| x.status == "failing");
    stages.push(VStage {
        id: "validation".into(),
        title: "Validation (stakeholder needs)".into(),
        leg: "right".into(),
        level: 0,
        status: if nn == 0 {
            "empty".into()
        } else if failing {
            "attention".into()
        } else if cov.needs_satisfied == nn {
            "done".into()
        } else {
            "partial".into()
        },
        progress: frac(cov.needs_satisfied, nn),
        summary: format!("{}/{nn} needs satisfied by passing requirements", cov.needs_satisfied),
        items: val_items,
        pairs_with: Some("conops".into()),
        link: "tab:traceability".into(),
    });
    let overall = stages.iter().map(|s| s.progress).sum::<f64>() / stages.len() as f64;
    VModel { stages, overall }
}

fn trunc(s: &str) -> String {
    let t: String = s.chars().take(90).collect();
    if s.chars().count() > 90 {
        format!("{t}…")
    } else {
        t
    }
}

// ---------------------------------------------------------------------------
// Stage-gate milestones
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckStatus {
    pub text: String,
    /// met | unmet | manual
    pub status: String,
    /// true when the engine could evaluate it from the project.
    pub auto: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateStatus {
    pub id: String,
    pub title: String,
    pub stage: String,
    pub criteria: Vec<CheckStatus>,
    pub deliverables: Vec<CheckStatus>,
    /// Every exit criterion met.
    pub passed: bool,
    /// The first gate that has not passed.
    pub current: bool,
}

/// Manual sign-offs live in `design.extra.studio.gates["M-3#0"] = true`
/// (criteria) and `["M-3#d0"]` (deliverables).
fn manual_flag(p: &Project, key: &str) -> bool {
    p.design
        .extra
        .get("studio")
        .and_then(|s| s.get("gates"))
        .and_then(|g| g.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn all_pass(ev: &Evaluation, ids: &[String]) -> (bool, String) {
    let (pass, fail, unv) = tally(ev, ids);
    (
        !ids.is_empty() && pass == ids.len(),
        format!("{pass}/{} pass, {fail} fail, {unv} unverified", ids.len()),
    )
}

fn auto_criterion(p: &Project, ev: &Evaluation, text: &str) -> Option<(bool, String)> {
    let t = text.to_lowercase();
    let has = |w: &str| t.contains(w);
    if has("requirement") && (has("green") || has("pass") || has("verified")) && !has("metric") {
        let ids: Vec<String> = if has("analysis") {
            p.requirements
                .iter()
                .filter(|r| r.verification.method == VerifyMethod::Analysis)
                .map(|r| r.id.clone())
                .collect()
        } else {
            p.requirements.iter().map(|r| r.id.clone()).collect()
        };
        return Some(all_pass(ev, &ids));
    }
    if has("requirement") && has("metric") {
        let n = p.requirements.len();
        let need_owner = has("owner") || has("allocat");
        let ok = p
            .requirements
            .iter()
            .filter(|r| r.verification.metric.is_some() && (!need_owner || !r.allocated_to.is_empty()))
            .count();
        return Some((
            n > 0 && ok == n,
            format!(
                "{ok}/{n} requirements have a metric{}",
                if need_owner { " and an owner" } else { "" }
            ),
        ));
    }
    if has("trade") && (has("decided") || has("decision")) {
        let n = p.trades.len();
        let decided = p.trades.iter().filter(|t| !t.decision.trim().is_empty()).count();
        let disagree: Vec<String> = p
            .trades
            .iter()
            .filter(|t| {
                let r = score_trade(t);
                !t.decision.trim().is_empty() && t.options.get(r.winner).is_some_and(|o| !decision_names(&t.decision, &o.name))
            })
            .map(|t| t.id.clone())
            .collect();
        let mut d = format!("{decided}/{n} trade studies decided");
        if !disagree.is_empty() {
            d += &format!("; recorded decision differs from the weighted winner in {}", disagree.join(", "));
        }
        return Some((n > 0 && decided == n, d));
    }
    if has("need") && (has("cover") || has("trace") || has("satisf")) {
        let cov = coverage(p, ev);
        return Some(if has("satisf") {
            (
                cov.needs_total > 0 && cov.needs_satisfied == cov.needs_total,
                format!("{}/{} needs satisfied", cov.needs_satisfied, cov.needs_total),
            )
        } else {
            (
                cov.needs_total > 0 && cov.needs_covered == cov.needs_total,
                format!("{}/{} needs covered", cov.needs_covered, cov.needs_total),
            )
        });
    }
    if has("risk") && (has("high") || has("rpn")) {
        let high = fmea(p).iter().filter(|r| r.level == "high").count();
        return Some((high == 0, format!("{high} high risks open")));
    }
    if has("test") && has("pass") {
        let ids: Vec<String> = p
            .requirements
            .iter()
            .filter(|r| r.verification.method == VerifyMethod::Test)
            .map(|r| r.id.clone())
            .collect();
        if ids.is_empty() {
            return None;
        }
        return Some(all_pass(ev, &ids));
    }
    None
}

fn auto_deliverable(p: &Project, ev: &Evaluation, text: &str) -> Option<(bool, String)> {
    let t = text.to_lowercase();
    let has = |w: &str| t.contains(w);
    let metric = |k: &str| ev.metrics.iter().any(|m| m.key.starts_with(k));
    if has("use case") {
        return Some((!p.concept.use_cases.is_empty(), format!("{} use cases", p.concept.use_cases.len())));
    }
    if has("need") {
        return Some((!p.needs.is_empty(), format!("{} needs", p.needs.len())));
    }
    if has("trade") {
        return Some((!p.trades.is_empty(), format!("{} trade studies", p.trades.len())));
    }
    if has("requirement") {
        let n = p.requirements.len();
        let bound = p.requirements.iter().filter(|r| r.verification.metric.is_some()).count();
        return Some((n > 0 && bound == n, format!("{n} requirements, {bound} with metrics")));
    }
    if has("architecture") {
        return Some((
            !p.subsystems.is_empty() && !p.interfaces.is_empty(),
            format!("{} subsystems, {} interfaces", p.subsystems.len(), p.interfaces.len()),
        ));
    }
    if has("simulation") {
        let ids: Vec<String> = p
            .requirements
            .iter()
            .filter(|r| r.verification.method == VerifyMethod::Analysis)
            .map(|r| r.id.clone())
            .collect();
        return Some(all_pass(ev, &ids));
    }
    if has("cad") {
        return Some((metric("cad."), "CAD Studio mass properties".into()));
    }
    if has("schematic") {
        return Some((metric("circuit."), "Circuit Lab simulations".into()));
    }
    if has("pcb") || has("gerber") {
        return Some((metric("pcb.routed_pct"), "PCB Studio layout".into()));
    }
    if has("frame") && has("print") {
        return Some((metric("print."), "Print Lab toolpaths".into()));
    }
    None
}

pub fn gates(p: &Project, ev: &Evaluation) -> Vec<GateStatus> {
    let mut out: Vec<GateStatus> = p
        .milestones
        .iter()
        .map(|m| {
            let criteria: Vec<CheckStatus> = m
                .exit_criteria
                .iter()
                .enumerate()
                .map(|(i, c)| match auto_criterion(p, ev, c) {
                    Some((ok, detail)) => CheckStatus {
                        text: c.clone(),
                        status: if ok { "met" } else { "unmet" }.into(),
                        auto: true,
                        detail,
                    },
                    None => {
                        let ok = manual_flag(p, &format!("{}#{i}", m.id));
                        CheckStatus {
                            text: c.clone(),
                            status: if ok { "met" } else { "manual" }.into(),
                            auto: false,
                            detail: if ok {
                                "signed off".into()
                            } else {
                                "needs a manual sign-off".into()
                            },
                        }
                    }
                })
                .collect();
            let deliverables = m
                .deliverables
                .iter()
                .enumerate()
                .map(|(i, d)| match auto_deliverable(p, ev, d) {
                    Some((ok, detail)) => CheckStatus {
                        text: d.clone(),
                        status: if ok { "met" } else { "unmet" }.into(),
                        auto: true,
                        detail,
                    },
                    None => {
                        let ok = manual_flag(p, &format!("{}#d{i}", m.id));
                        CheckStatus {
                            text: d.clone(),
                            status: if ok { "met" } else { "manual" }.into(),
                            auto: false,
                            detail: if ok {
                                "signed off".into()
                            } else {
                                "needs a manual sign-off".into()
                            },
                        }
                    }
                })
                .collect();
            GateStatus {
                id: m.id.clone(),
                title: m.title.clone(),
                stage: m.stage.clone(),
                passed: !criteria.is_empty() && criteria.iter().all(|c| c.status == "met"),
                criteria,
                deliverables,
                current: false,
            }
        })
        .collect();
    if let Some(g) = out.iter_mut().find(|g| !g.passed) {
        g.current = true;
    }
    out
}

// ---------------------------------------------------------------------------
// Trade studies
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TornadoBar {
    pub criterion: usize,
    pub name: String,
    /// Winner's lead over the best other option at the baseline weights.
    pub base_lead: f64,
    /// Lead of the baseline winner when this weight is scaled by (1 − swing).
    pub lo_lead: f64,
    /// … and by (1 + swing). Negative = another option would win.
    pub hi_lead: f64,
    pub lo_winner: usize,
    pub hi_winner: usize,
    /// Relative weight change that flips the winner (from `score_trade`).
    pub flip: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeDetail {
    pub result: TradeResult,
    pub title: String,
    /// Weights normalised to sum to 1.
    pub weights: Vec<f64>,
    /// Per option, per criterion: normalised weight × (direction-adjusted) score.
    pub contributions: Vec<Vec<f64>>,
    /// Options best-first.
    pub ranking: Vec<usize>,
    /// Winner's lead over the runner-up (score points, 0..10 scale).
    pub lead: f64,
    pub decision_index: Option<usize>,
    pub decision_agrees: bool,
    pub swing: f64,
    pub tornado: Vec<TornadoBar>,
}

fn lead_of(scores: &[f64], winner: usize) -> f64 {
    let best_other = scores
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != winner)
        .map(|(_, s)| *s)
        .fold(f64::NEG_INFINITY, f64::max);
    if best_other.is_finite() {
        scores[winner] - best_other
    } else {
        0.0
    }
}

pub fn trade_detail(t: &TradeStudy) -> TradeDetail {
    let result = score_trade(t);
    let raw: Vec<f64> = t.criteria.iter().map(|c| c.weight).collect();
    let total: f64 = raw.iter().sum::<f64>().max(1e-12);
    let weights: Vec<f64> = raw.iter().map(|w| w / total).collect();
    let contributions = t
        .options
        .iter()
        .map(|o| {
            t.criteria
                .iter()
                .enumerate()
                .map(|(j, c)| {
                    let s = o.scores.get(j).copied().unwrap_or(0.0);
                    let s = if c.higher_is_better { s } else { 11.0 - s };
                    weights[j] * s
                })
                .collect()
        })
        .collect();
    let mut ranking: Vec<usize> = (0..t.options.len()).collect();
    ranking.sort_by(|a, b| {
        result.scores[*b]
            .partial_cmp(&result.scores[*a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let winner = result.winner;
    let lead = if t.options.is_empty() {
        0.0
    } else {
        lead_of(&result.scores, winner)
    };
    let decision_index = t
        .options
        .iter()
        .position(|o| o.name.trim() == t.decision.trim() && !t.decision.trim().is_empty());
    let swing = 0.5;
    let mut tornado: Vec<TornadoBar> = if t.options.is_empty() {
        vec![]
    } else {
        (0..raw.len())
            .map(|i| {
                let mut lo = raw.clone();
                lo[i] *= 1.0 - swing;
                let mut hi = raw.clone();
                hi[i] *= 1.0 + swing;
                let slo = weighted(t, &lo);
                let shi = weighted(t, &hi);
                TornadoBar {
                    criterion: i,
                    name: t.criteria[i].name.clone(),
                    base_lead: lead,
                    lo_lead: slo[winner]
                        - slo
                            .iter()
                            .enumerate()
                            .filter(|(k, _)| *k != winner)
                            .map(|(_, s)| *s)
                            .fold(f64::NEG_INFINITY, f64::max),
                    hi_lead: shi[winner]
                        - shi
                            .iter()
                            .enumerate()
                            .filter(|(k, _)| *k != winner)
                            .map(|(_, s)| *s)
                            .fold(f64::NEG_INFINITY, f64::max),
                    lo_winner: argmax(&slo),
                    hi_winner: argmax(&shi),
                    flip: result.weight_sensitivity.get(i).copied().flatten(),
                }
            })
            .collect()
    };
    for b in &mut tornado {
        if !b.lo_lead.is_finite() {
            b.lo_lead = 0.0;
        }
        if !b.hi_lead.is_finite() {
            b.hi_lead = 0.0;
        }
    }
    tornado.sort_by(|a, b| {
        (b.hi_lead - b.lo_lead)
            .abs()
            .partial_cmp(&(a.hi_lead - a.lo_lead).abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    TradeDetail {
        decision_agrees: decision_index.is_none_or(|d| d == winner),
        title: t.title.clone(),
        result,
        weights,
        contributions,
        ranking,
        lead,
        decision_index,
        swing,
        tornado,
    }
}

// ---------------------------------------------------------------------------
// Everything for the studio in one call
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Studio {
    pub budgets: Budgets,
    pub n2: N2,
    pub coverage: Coverage,
    pub vmodel: VModel,
    pub gates: Vec<GateStatus>,
    pub quality: Vec<QualityIssue>,
    pub fmea: Vec<RiskResult>,
    pub trades: Vec<TradeDetail>,
    pub trace: TraceGraph,
}

pub fn studio(p: &Project, ev: &Evaluation) -> Studio {
    Studio {
        budgets: budgets(p, ev),
        n2: n2(p),
        coverage: coverage(p, ev),
        vmodel: vmodel(p, ev),
        gates: gates(p, ev),
        quality: quality_with(p, Some(&ev.metrics)),
        fmea: fmea(p),
        trades: p.trades.iter().map(trade_detail).collect(),
        trace: trace_graph(p),
    }
}

/// A recorded decision names an option when it is that option's name, or starts
/// with it followed by a non-alphanumeric character ("240 mAh — chosen because …").
fn decision_names(decision: &str, option: &str) -> bool {
    let d = decision.trim();
    d == option
        || d.strip_prefix(option)
            .is_some_and(|rest| rest.chars().next().is_some_and(|c| !c.is_alphanumeric()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn decision_text_may_explain_itself() {
        assert!(super::decision_names("240 mAh", "240 mAh"));
        assert!(super::decision_names("240 mAh — chosen for reliability", "240 mAh"));
        assert!(!super::decision_names("2400 mAh", "240"));
        assert!(!super::decision_names("180 mAh", "240 mAh"));
    }

    use super::*;
    use crate::project::{Criterion, Interface, TradeOption};
    use crate::systems::evaluate;

    #[test]
    fn n2_places_interfaces_and_sequences() {
        let p = Project::iris();
        let m = n2(&p);
        assert_eq!(m.ids.len(), p.subsystems.len());
        let total: usize = m.cells.iter().map(|c| c.links.len()).sum();
        assert_eq!(total, p.interfaces.len());
        assert!(m.dangling.is_empty());
        // IF-02: SYS-AUD → SYS-CMP sits in row AUD, column CMP
        let aud = m.ids.iter().position(|x| x == "SYS-AUD").unwrap();
        let cmp = m.ids.iter().position(|x| x == "SYS-CMP").unwrap();
        assert!(m.cells.iter().any(|c| c.row == aud && c.col == cmp && c.links[0].kind == "PDM"));
        assert!(m.suggested_feedback <= m.feedback);
        let mut o = m.suggested_order.clone();
        o.sort();
        assert_eq!(o, (0..m.ids.len()).collect::<Vec<_>>());
    }

    #[test]
    fn sequencing_removes_feedback_in_a_chain() {
        // chain C → B → A declared in order A, B, C: 2 feedback marks, 0 after sequencing
        let mut p = Project::blank("chain");
        for id in ["A", "B", "C"] {
            p.subsystems.push(crate::project::Subsystem {
                id: id.into(),
                name: id.into(),
                ..Default::default()
            });
        }
        for (i, (f, t)) in [("C", "B"), ("B", "A")].iter().enumerate() {
            p.interfaces.push(Interface {
                id: format!("IF-{i}"),
                from: (*f).into(),
                to: (*t).into(),
                kind: "SPI".into(),
                description: String::new(),
            });
        }
        let m = n2(&p);
        assert_eq!(m.feedback, 2);
        assert_eq!(m.suggested_feedback, 0);
        assert_eq!(m.suggested_order, vec![2, 1, 0]);
    }

    #[test]
    fn coverage_counts_iris() {
        let p = Project::iris();
        let ev = evaluate(&p);
        let c = coverage(&p, &ev);
        assert_eq!(c.needs_total, 8);
        assert_eq!(c.needs_covered, 8);
        assert_eq!(c.reqs_total, p.requirements.len());
        assert_eq!(c.reqs_traced, c.reqs_total);
        let analysis = c.methods.iter().find(|m| m.key == "analysis").unwrap().count;
        assert_eq!(analysis, p.requirements.len());
        assert!(c.completeness > 0.99, "{}", c.completeness);
        // N-7 derives R-016..R-021, R-023
        let n7 = c.needs.iter().find(|n| n.id == "N-7").unwrap();
        assert!(n7.requirements.contains(&"R-017".to_string()));
    }

    #[test]
    fn vmodel_has_nine_stages_with_pairs() {
        let p = Project::iris();
        let ev = evaluate(&p);
        let v = vmodel(&p, &ev);
        assert_eq!(v.stages.len(), 9);
        for s in &v.stages {
            if let Some(pw) = &s.pairs_with {
                let other = v.stages.iter().find(|x| &x.id == pw).unwrap();
                assert_eq!(other.level, s.level, "{} ↔ {}", s.id, pw);
            }
            assert!((0.0..=1.0).contains(&s.progress));
        }
        assert_eq!(v.stages[0].status, "done");
        let blank = Project::blank("x");
        let vb = vmodel(&blank, &evaluate(&blank));
        assert_eq!(vb.stages.iter().find(|s| s.id == "system").unwrap().status, "empty");
        // lab metrics nobody uses are not implementation progress
        assert_eq!(vb.stages.iter().find(|s| s.id == "implementation").unwrap().status, "empty");
    }

    #[test]
    fn gates_auto_evaluate_known_criteria() {
        let p = Project::iris();
        let ev = evaluate(&p);
        let g = gates(&p, &ev);
        assert_eq!(g.len(), p.milestones.len());
        let m1 = &g[1];
        assert!(m1.criteria[0].auto, "{:?}", m1.criteria[0]);
        assert_eq!(m1.criteria[0].status, "met");
        let m2 = &g[2];
        assert!(m2.criteria[0].auto);
        // "Yield above 95 %" can only be signed off by hand
        assert_eq!(g[5].criteria[0].status, "manual");
        assert_eq!(g.iter().filter(|x| x.current).count(), 1);
        // manual sign-off flips it
        let mut q = p.clone();
        q.design
            .extra
            .insert("studio".into(), serde_json::json!({ "gates": { "M-5#0": true } }));
        assert_eq!(gates(&q, &ev)[5].criteria[0].status, "met");
    }

    #[test]
    fn trade_detail_tornado_and_decision() {
        let t = TradeStudy {
            id: "T".into(),
            title: "t".into(),
            decision: "B".into(),
            criteria: vec![
                Criterion {
                    name: "x".into(),
                    weight: 0.6,
                    higher_is_better: true,
                },
                Criterion {
                    name: "y".into(),
                    weight: 0.4,
                    higher_is_better: true,
                },
            ],
            options: vec![
                TradeOption {
                    name: "A".into(),
                    scores: vec![9.0, 3.0],
                    notes: String::new(),
                },
                TradeOption {
                    name: "B".into(),
                    scores: vec![5.0, 8.0],
                    notes: String::new(),
                },
            ],
        };
        let d = trade_detail(&t);
        // A = 0.6·9 + 0.4·3 = 6.6, B = 0.6·5 + 0.4·8 = 6.2
        assert_eq!(d.result.winner, 0);
        assert!((d.lead - 0.4).abs() < 1e-9);
        assert_eq!(d.decision_index, Some(1));
        assert!(!d.decision_agrees);
        let s: f64 = d.contributions[0].iter().sum();
        assert!((s - d.result.scores[0]).abs() < 1e-9);
        // halving x's weight: w = (0.3, 0.4)/0.7 → A = 5.57, B = 6.71 → B wins
        let bx = d.tornado.iter().find(|b| b.name == "x").unwrap();
        assert!(bx.lo_lead < 0.0 && bx.lo_winner == 1);
        assert!(bx.hi_lead > d.lead);
    }

    #[test]
    fn studio_bundle_serialises() {
        let p = Project::iris();
        let ev = evaluate(&p);
        let s = studio(&p, &ev);
        let v = serde_json::to_value(&s).unwrap();
        assert!(v.get("budgets").is_some() && v.get("vmodel").is_some());
        let out = crate::api::call("systems.studio", &serde_json::json!({ "project": p, "evaluation": ev }).to_string());
        assert!(out.starts_with("{\"ok\""), "{}", &out[..200.min(out.len())]);
    }
}
