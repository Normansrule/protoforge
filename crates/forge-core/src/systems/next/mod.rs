//! "What should I do next?" — ranks everything open on a project into
//! concrete next actions: failing requirements, open goals, thin margins,
//! unverified requirements, requirement-quality and traceability gaps,
//! unmitigated high risks and the next stage gate. Each action says where to
//! go (lab + tab) and, for metric-bound requirements, which design
//! parameters move the metric — measured by re-running the owning lab
//! module ([`levers`]). [`levers::solve`] then finds the value of one
//! parameter that meets the target and lists every side effect.
//!
//! **Ranking rule** (see `docs/next-actions.md`): every action gets
//! `score = base(kind) + bonus`, with the kind bases 10 points apart
//! ([`BASES`]) and `bonus = 10 × clamp((importance × urgency − 1) / 1.5, 0, 0.99)`,
//! so the kind always decides first and importance × urgency orders actions
//! within a kind. Importance is 1, plus 0.15 for safety/regulatory
//! requirements, plus 0.10 when the requirement serves a "must"
//! (priority-1) need. Urgency is 1 plus the shortfall (capped at 1) for
//! failures, 1 plus how much of the marginal band is used up for marginal
//! passes, and 1 plus RPN / 200 for risks.

pub mod levers;
pub mod routes;

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::analysis::{coverage, gates};
use super::quality::quality_with;
use super::{evaluate, fmea, Evaluation, Status};
use crate::api::{handle, ApiResult};
use crate::project::{Cmp, Project, ReqKind, VerifyMethod};
use levers::{fmt_num, Lever, Perturber, LAZY_MODULES};
use routes::Where;

/// A pass whose margin is below this fraction of the target is "marginal".
///
/// 10 % is about the uncertainty of the first-order models the labs use
/// (closed-form estimates, lumped thermal networks, parts-count reliability)
/// — a pass inside that band could flip when the model is refined or the
/// part is measured. Margin policies differ by programme; this is a
/// default, not a standard.
pub const MARGINAL: f64 = 0.10;

/// Kind → (base score, plain name). Bases are 10 apart so the within-kind
/// bonus (0–9.9) never reorders kinds.
pub const BASES: &[(&str, f64, &str)] = &[
    ("failing", 100.0, "Failing requirement"),
    ("unverified", 90.0, "Unverified requirement"),
    ("goal", 80.0, "Open goal"),
    ("risk", 70.0, "Unmitigated high risk"),
    ("quality_error", 60.0, "Requirement error"),
    ("marginal", 50.0, "Marginal pass"),
    ("gate", 40.0, "Stage gate"),
    ("no_verification", 30.0, "No verification"),
    ("traceability", 20.0, "Traceability gap"),
    ("quality", 10.0, "Requirement wording"),
    ("quality_info", 0.0, "Wording suggestion"),
];

fn base(kind: &str) -> f64 {
    BASES.iter().find(|b| b.0 == kind).map(|b| b.1).unwrap_or(0.0)
}

fn bonus(importance: f64, urgency: f64) -> f64 {
    10.0 * ((importance * urgency - 1.0) / 1.5).clamp(0.0, 0.99)
}

/// One thing to do next.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    /// Stable id: `kind:subject`.
    pub id: String,
    /// failing | unverified | goal | risk | quality_error | marginal | gate | no_verification | traceability | quality | quality_info
    pub kind: String,
    /// critical | high | medium | low | info
    pub severity: String,
    pub score: f64,
    /// How the score was built, in words.
    pub score_note: String,
    pub title: String,
    pub why: String,
    /// Requirements (or needs, subsystems, risks, gates) involved.
    pub requirements: Vec<String>,
    pub metric: Option<String>,
    pub metric_label: Option<String>,
    pub value: Option<f64>,
    pub target: Option<f64>,
    pub target_hi: Option<f64>,
    pub cmp: Option<Cmp>,
    pub criterion: Option<String>,
    pub unit: String,
    /// Fraction of the target (positive = inside the limit).
    pub margin: Option<f64>,
    /// The metric's own note (how the lab computed it).
    pub note: String,
    #[serde(rename = "where")]
    pub location: Where,
    /// Lab module that computes the metric.
    pub module: Option<String>,
    pub levers: Vec<Lever>,
    /// measured | lazy (call `systems.next_levers`) | none (no shortlisted parameter moves it) | n/a
    pub levers_status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Summary {
    pub requirements: usize,
    pub pass: usize,
    /// Passing by at least [`MARGINAL`].
    pub pass_with_margin: usize,
    pub marginal: usize,
    /// Binding requirements failing (open goals excluded).
    pub failing: usize,
    pub open_goals: usize,
    pub unverified: usize,
    /// pass_with_margin ÷ requirements, in percent.
    pub readiness_pct: f64,
    pub marginal_threshold: f64,
    /// Action count per kind.
    pub counts: BTreeMap<String, usize>,
    pub actions: usize,
    /// Modules whose levers were measured inline / left for `systems.next_levers`.
    pub levers_measured: Vec<String>,
    pub levers_lazy: Vec<String>,
    pub headline: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NextActions {
    pub actions: Vec<Action>,
    pub summary: Summary,
}

/// Input of `systems.next_actions`.
#[derive(Deserialize)]
pub struct NextIn {
    pub project: Project,
    #[serde(default)]
    pub evaluation: Option<Evaluation>,
    /// `inline` (default: cheap modules measured now, expensive ones lazy) | `all` | `none`.
    #[serde(default)]
    pub levers: Option<String>,
    /// Keep only the first `limit` actions (the summary still counts all).
    #[serde(default)]
    pub limit: Option<usize>,
}

struct Draft {
    kind: &'static str,
    severity: &'static str,
    importance: f64,
    urgency: f64,
    title: String,
    why: String,
    subjects: Vec<String>,
    location: Where,
    req: Option<usize>,
}

/// Lower-case the first letter unless the label starts with an acronym ("RMS spot").
fn lower_first(s: &str) -> String {
    let mut it = s.chars();
    match (it.next(), it.next()) {
        (Some(a), Some(b)) if a.is_uppercase() && b.is_uppercase() => s.to_string(),
        (Some(a), _) => a.to_lowercase().collect::<String>() + &s[a.len_utf8()..],
        _ => s.to_string(),
    }
}

fn pct(x: f64) -> String {
    let a = (x * 100.0).abs();
    if a < 10.0 {
        format!("{a:.1} %")
    } else {
        format!("{a:.0} %")
    }
}

/// Whether "marginal" makes sense for a criterion: not when the target is 0
/// (the margin is then absolute, in the metric's unit) and not when the
/// target sits at the natural limit of the quantity (≥ 100 %, ≤ 0 counts).
fn margin_meaningful(v: &crate::project::Verification) -> bool {
    if v.target.abs() < 1e-12 {
        return false;
    }
    !(v.cmp == Cmp::Ge && v.unit.trim() == "%" && v.target >= 100.0)
}

/// Build the ranked action list.
pub fn next_actions(i: &NextIn) -> Result<NextActions, String> {
    let p = &i.project;
    let ev = i.evaluation.clone().unwrap_or_else(|| evaluate(p));
    let metric_by_key: BTreeMap<&str, &crate::project::Metric> = ev.metrics.iter().map(|m| (m.key.as_str(), m)).collect();
    let must_needs: BTreeSet<&str> = p.needs.iter().filter(|n| n.priority <= 1).map(|n| n.id.as_str()).collect();
    let mut drafts: Vec<Draft> = Vec::new();
    let mut marginal = 0usize;

    for (k, (r, res)) in p.requirements.iter().zip(&ev.results).enumerate() {
        let v = &r.verification;
        let goal = r.kind == ReqKind::Goal;
        let importance =
            1.0 + if matches!(r.kind, ReqKind::Safety | ReqKind::Regulatory) {
                0.15
            } else {
                0.0
            } + if r.derives_from.iter().any(|n| must_needs.contains(n.as_str())) {
                0.10
            } else {
                0.0
            };
        let mlabel = v
            .metric
            .as_deref()
            .and_then(|k| metric_by_key.get(k))
            .map(|m| lower_first(&m.label))
            .unwrap_or_else(|| v.metric.clone().unwrap_or_default());
        let loc = || match v.metric.as_deref() {
            Some(key) => routes::for_metric(key, Some(&r.id)),
            None => routes::studio("requirements", Some(&r.id)),
        };
        let now = |val: Option<f64>| match val {
            Some(x) => format!("{} {}", fmt_num(x), res.unit).trim().to_string(),
            None => "—".into(),
        };
        let aim = match v.cmp {
            Cmp::Le => format!("Bring {mlabel} down to {}", res.criterion),
            Cmp::Ge => format!("Raise {mlabel} to {}", res.criterion),
            Cmp::Eq | Cmp::Between => format!("Bring {mlabel} into {}", res.criterion),
        };
        match res.status {
            Status::Fail => {
                let short = -res.margin.unwrap_or(0.0);
                let gap = if v.target.abs() > 1e-12 {
                    format!("{} short of the target", pct(short))
                } else {
                    format!("{} {} outside the limit", fmt_num(short.abs()), res.unit)
                };
                let (kind, severity, tail) = if goal {
                    (
                        "goal",
                        "medium",
                        "It is a goal (\"should\"), so it does not block a review gate, but it is the clearest way to make the product better.",
                    )
                } else if matches!(r.kind, ReqKind::Safety | ReqKind::Regulatory) {
                    (
                        "failing",
                        "critical",
                        "A binding safety requirement (\"shall\"): it blocks the next review gate and must be fixed or formally waived.",
                    )
                } else {
                    (
                        "failing",
                        "high",
                        "A binding requirement (\"shall\"): it blocks the next review gate until the design changes or the requirement is renegotiated.",
                    )
                };
                drafts.push(Draft {
                    kind,
                    severity,
                    importance,
                    urgency: 1.0 + short.clamp(0.0, 1.0),
                    title: format!("{aim} ({})", r.id),
                    why: format!("{} Now {}: {gap}. {tail}", r.text, now(res.value)),
                    subjects: vec![r.id.clone()],
                    location: loc(),
                    req: Some(k),
                });
            }
            Status::Pass => {
                let m = res.margin.unwrap_or(1.0);
                if margin_meaningful(v) && m < MARGINAL {
                    marginal += 1;
                    drafts.push(Draft {
                        kind: "marginal",
                        severity: "low",
                        importance,
                        urgency: 1.0 + (1.0 - m / MARGINAL).clamp(0.0, 1.0),
                        title: format!("Add margin to {mlabel} ({})", r.id),
                        why: format!(
                            "{} It passes at {} against {}, but only by {} — inside the {:.0} % band where a refined model, a measured part or a design change elsewhere could flip it.",
                            r.text,
                            now(res.value),
                            res.criterion,
                            pct(m),
                            MARGINAL * 100.0
                        ),
                        subjects: vec![r.id.clone()],
                        location: loc(),
                        req: Some(k),
                    });
                }
            }
            Status::Unverified => {
                if let Some(key) = v.metric.as_deref() {
                    drafts.push(Draft {
                        kind: "unverified",
                        severity: if goal { "medium" } else { "high" },
                        importance,
                        urgency: 1.0,
                        title: format!("Get a value for {key} ({})", r.id),
                        why: format!(
                            "{} The requirement is bound to the metric {key}, but no lab produced it, so nobody knows whether it passes. Check the key against the metric list, or open the lab that should compute it.",
                            r.text
                        ),
                        subjects: vec![r.id.clone()],
                        location: loc(),
                        req: Some(k),
                    });
                } else {
                    let analysis = v.method == VerifyMethod::Analysis;
                    let method = format!("{:?}", v.method).to_lowercase();
                    drafts.push(Draft {
                        kind: "no_verification",
                        severity: if analysis { "low" } else { "info" },
                        importance,
                        urgency: if analysis { 1.2 } else { 1.0 },
                        title: if analysis {
                            format!("Decide how to verify {}", r.id)
                        } else {
                            format!("Plan the {method} for {}", r.id)
                        },
                        why: if analysis {
                            format!(
                                "{} It is verified by analysis but bound to no lab metric and has no test planned, so it can never turn green. Bind a metric a lab computes, or change the method to test, inspection or demonstration and plan it.",
                                r.text
                            )
                        } else {
                            format!(
                                "{} It is verified by {method}, which the engine cannot run: write the procedure and pass criterion, and record the result when it is done.",
                                r.text
                            )
                        },
                        subjects: vec![r.id.clone()],
                        location: routes::studio("requirements", Some(&r.id)),
                        req: Some(k),
                    });
                }
            }
        }
    }

    // requirement quality and traceability (structural lint + INCOSE checks)
    let issues = quality_with(p, Some(&ev.metrics));
    let mut trace: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut per_req: BTreeMap<String, Vec<&super::quality::QualityIssue>> = BTreeMap::new();
    let mut infos: Vec<&super::quality::QualityIssue> = Vec::new();
    for q in &issues {
        let m = q.message.as_str();
        if m.starts_with("not traced") {
            trace.entry("untraced").or_default().push(q.id.clone());
        } else if m.starts_with("not allocated") {
            trace.entry("unallocated").or_default().push(q.id.clone());
        } else if m.starts_with("need has no") {
            trace.entry("uncovered").or_default().push(q.id.clone());
        } else if m.starts_with("no metric bound") || (m.starts_with("metric ") && m.ends_with("not produced by any lab")) {
            // covered by the requirement's own unverified / no-verification action
        } else if q.severity == "info" {
            infos.push(q);
        } else {
            per_req.entry(q.id.clone()).or_default().push(q);
        }
    }
    for (id, qs) in &per_req {
        let error = qs.iter().any(|q| q.severity == "error");
        let first = qs[0];
        let more = if qs.len() > 1 {
            format!(" (+{} more)", qs.len() - 1)
        } else {
            String::new()
        };
        let hints: Vec<String> = qs
            .iter()
            .map(|q| {
                format!(
                    "{} [{}]{}",
                    q.message,
                    q.characteristic,
                    if q.hint.is_empty() {
                        String::new()
                    } else {
                        format!(": {}", q.hint)
                    }
                )
            })
            .collect();
        let is_req = p.requirements.iter().any(|r| &r.id == id);
        drafts.push(Draft {
            kind: if error { "quality_error" } else { "quality" },
            severity: if error { "medium" } else { "low" },
            importance: 1.0,
            urgency: 1.0 + 0.1 * qs.len().min(5) as f64,
            title: format!("Fix the wording of {id}: {}{more}", first.message),
            why: hints.join(" · "),
            subjects: vec![id.clone()],
            location: if is_req {
                routes::studio("requirements", Some(id))
            } else if p.interfaces.iter().any(|x| &x.id == id) {
                routes::studio("architecture", None)
            } else {
                routes::studio("traceability", None)
            },
            req: None,
        });
    }
    if !infos.is_empty() {
        let ids: BTreeSet<String> = infos.iter().map(|q| q.id.clone()).collect();
        let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
        for q in &infos {
            *kinds.entry(q.message.as_str()).or_default() += 1;
        }
        let list: Vec<String> = kinds.iter().map(|(m, n)| format!("{m} ×{n}")).collect();
        drafts.push(Draft {
            kind: "quality_info",
            severity: "info",
            importance: 1.0,
            urgency: 1.0,
            title: format!("{} small wording suggestions on {} items", infos.len(), ids.len()),
            why: format!(
                "Requirement-quality hints after the INCOSE Guide to Writing Requirements: {}. None blocks verification.",
                list.join("; ")
            ),
            subjects: ids.into_iter().collect(),
            location: routes::studio("requirements", None),
            req: None,
        });
    }
    let n = |v: &Vec<String>| v.len();
    if let Some(v) = trace.get("untraced") {
        drafts.push(Draft {
            kind: "traceability",
            severity: "low",
            importance: 1.0,
            urgency: 1.0 + 0.05 * n(v).min(10) as f64,
            title: format!(
                "Trace {} requirement{} to a stakeholder need",
                n(v),
                if n(v) == 1 { "" } else { "s" }
            ),
            why: format!(
                "{} {} not derived from any need. An untraced requirement may not be needed at all — or a need is missing from the list.",
                v.join(", "),
                if n(v) == 1 { "is" } else { "are" }
            ),
            subjects: v.clone(),
            location: routes::studio("traceability", None),
            req: None,
        });
    }
    if let Some(v) = trace.get("unallocated") {
        drafts.push(Draft {
            kind: "traceability",
            severity: "low",
            importance: 1.0,
            urgency: 1.0 + 0.05 * n(v).min(10) as f64,
            title: format!("Give {} requirement{} an owning subsystem", n(v), if n(v) == 1 { "" } else { "s" }),
            why: format!(
                "{} {} not allocated to a subsystem, so no part of the design is responsible for meeting {}.",
                v.join(", "),
                if n(v) == 1 { "is" } else { "are" },
                if n(v) == 1 { "it" } else { "them" }
            ),
            subjects: v.clone(),
            location: routes::studio("traceability", None),
            req: None,
        });
    }
    if let Some(v) = trace.get("uncovered") {
        drafts.push(Draft {
            kind: "traceability",
            severity: "low",
            importance: 1.1,
            urgency: 1.0 + 0.05 * n(v).min(10) as f64,
            title: format!(
                "Derive a requirement for {} uncovered need{}",
                n(v),
                if n(v) == 1 { "" } else { "s" }
            ),
            why: format!(
                "{} {} no requirement derived from {}, so nothing checks that the product meets {}.",
                v.join(", "),
                if n(v) == 1 { "has" } else { "have" },
                if n(v) == 1 { "it" } else { "them" },
                if n(v) == 1 { "it" } else { "them" }
            ),
            subjects: v.clone(),
            location: routes::studio("traceability", None),
            req: None,
        });
    }
    let cov = coverage(p, &ev);
    if !cov.empty_subsystems.is_empty() {
        let v = &cov.empty_subsystems;
        drafts.push(Draft {
            kind: "traceability",
            severity: "low",
            importance: 1.0,
            urgency: 1.0,
            title: format!(
                "Allocate requirements to {} empty subsystem{}",
                n(v),
                if n(v) == 1 { "" } else { "s" }
            ),
            why: format!(
                "{} {} no requirement allocated: either something the subsystem must do is unwritten, or the subsystem is not needed.",
                v.join(", "),
                if n(v) == 1 { "has" } else { "have" }
            ),
            subjects: v.clone(),
            location: routes::studio("architecture", None),
            req: None,
        });
    }

    // risks
    for rr in fmea(p) {
        let Some(risk) = p.risks.iter().find(|x| x.id == rr.id) else {
            continue;
        };
        if rr.level == "high" && risk.mitigation.trim().is_empty() {
            drafts.push(Draft {
                kind: "risk",
                severity: if risk.severity >= 9 { "high" } else { "medium" },
                importance: 1.0 + if risk.severity >= 9 { 0.15 } else { 0.0 },
                urgency: 1.0 + (rr.rpn as f64 / 200.0).min(1.0),
                title: format!(
                    "Plan a mitigation for {}: {} — {}",
                    risk.id,
                    risk.item,
                    lower_first(&risk.failure_mode)
                ),
                why: format!(
                    "Risk priority number (RPN) {} = severity {} × occurrence {} × detection {}, with no mitigation recorded. Effect: {}.",
                    rr.rpn,
                    risk.severity,
                    risk.occurrence,
                    risk.detection,
                    if risk.effect.is_empty() {
                        "not stated"
                    } else {
                        risk.effect.as_str()
                    }
                ),
                subjects: vec![risk.id.clone()],
                location: routes::studio("risks", None),
                req: None,
            });
        }
    }

    // the next stage gate
    if let Some(g) = gates(p, &ev).into_iter().find(|g| g.current) {
        let unmet: Vec<&super::analysis::CheckStatus> = g.criteria.iter().filter(|c| c.status != "met").collect();
        if !unmet.is_empty() {
            let auto = unmet.iter().filter(|c| c.auto).count();
            let manual = unmet.len() - auto;
            let list: Vec<String> = unmet
                .iter()
                .map(|c| {
                    if c.auto {
                        format!("{} ({})", c.text, c.detail)
                    } else {
                        format!("{} (needs a sign-off)", c.text)
                    }
                })
                .collect();
            drafts.push(Draft {
                kind: "gate",
                severity: "medium",
                importance: 1.0,
                urgency: 1.0 + 0.1 * unmet.len().min(5) as f64,
                title: format!(
                    "Get {} ({}) passable: {} exit criteri{} open",
                    g.title,
                    g.id,
                    unmet.len(),
                    if unmet.len() == 1 { "on" } else { "a" }
                ),
                why: format!(
                    "{} of {} exit criteria not met yet ({auto} computed, {manual} need a manual sign-off): {}.",
                    unmet.len(),
                    g.criteria.len(),
                    list.join("; ")
                ),
                subjects: vec![g.id.clone()],
                location: routes::studio("roadmap", None),
                req: None,
            });
        }
    }

    // levers
    let mode = i.levers.as_deref().unwrap_or("inline");
    let pert = if mode == "none" { None } else { Some(Perturber::new(p)?) };
    let mut measured: BTreeSet<String> = BTreeSet::new();
    let mut lazy: BTreeSet<String> = BTreeSet::new();
    let mut actions: Vec<Action> = drafts
        .into_iter()
        .map(|d| {
            let req = d.req.map(|k| (&p.requirements[k], &ev.results[k]));
            let metric = req.and_then(|(r, _)| r.verification.metric.clone());
            let m = metric.as_deref().and_then(|k| metric_by_key.get(k));
            let module = metric.as_deref().and_then(levers::owner).map(String::from);
            let wants_levers = matches!(d.kind, "failing" | "goal" | "marginal");
            let (lv, status) = match (&module, &pert, wants_levers, req) {
                (Some(md), Some(pt), true, Some((r, _))) => {
                    if LAZY_MODULES.contains(&md.as_str()) && mode != "all" {
                        lazy.insert(md.clone());
                        (vec![], "lazy")
                    } else {
                        measured.insert(md.clone());
                        let probes = levers::measure_module(p, pt, md);
                        let l = levers::levers_from(&probes, metric.as_deref().unwrap_or(""), Some(&r.verification), 6);
                        let st = if l.is_empty() { "none" } else { "measured" };
                        (l, st)
                    }
                }
                _ => (vec![], if wants_levers && mode != "none" { "none" } else { "n/a" }),
            };
            let score = base(d.kind) + bonus(d.importance, d.urgency);
            Action {
                id: format!("{}:{}", d.kind, d.subjects.first().cloned().unwrap_or_default()),
                kind: d.kind.into(),
                severity: d.severity.into(),
                score: (score * 10.0).round() / 10.0,
                score_note: format!(
                    "{} (base {:.0}) + {:.1} for importance {:.2} × urgency {:.2}",
                    BASES.iter().find(|b| b.0 == d.kind).map(|b| b.2).unwrap_or(d.kind),
                    base(d.kind),
                    bonus(d.importance, d.urgency),
                    d.importance,
                    d.urgency
                ),
                title: d.title,
                why: d.why,
                requirements: d.subjects,
                metric_label: m.map(|m| m.label.clone()),
                value: req.and_then(|(_, res)| res.value),
                target: req.map(|(r, _)| r.verification.target),
                target_hi: req.and_then(|(r, _)| r.verification.target_hi),
                cmp: req.map(|(r, _)| r.verification.cmp),
                criterion: req.map(|(_, res)| res.criterion.clone()),
                unit: req.map(|(_, res)| res.unit.clone()).unwrap_or_default(),
                margin: req.and_then(|(_, res)| res.margin),
                note: m.map(|m| m.note.clone()).unwrap_or_default(),
                metric,
                location: d.location,
                module,
                levers: lv,
                levers_status: status.into(),
            }
        })
        .collect();
    actions.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.id.cmp(&b.id))
    });

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for a in &actions {
        *counts.entry(a.kind.clone()).or_default() += 1;
    }
    let total = p.requirements.len();
    let failing = ev.fail - ev.open_goals;
    let pass_with_margin = ev.pass - marginal;
    let readiness_pct = if total > 0 {
        pass_with_margin as f64 / total as f64 * 100.0
    } else {
        0.0
    };
    let headline = match actions.first() {
        None => "Nothing open: every requirement passes with margin and the next gate is clear.".to_string(),
        Some(a) => format!("Start with: {}", a.title),
    };
    let n_actions = actions.len();
    if let Some(l) = i.limit {
        actions.truncate(l);
    }
    Ok(NextActions {
        actions,
        summary: Summary {
            requirements: total,
            pass: ev.pass,
            pass_with_margin,
            marginal,
            failing,
            open_goals: ev.open_goals,
            unverified: ev.unverified,
            readiness_pct,
            marginal_threshold: MARGINAL,
            counts,
            actions: n_actions,
            levers_measured: measured.into_iter().collect(),
            levers_lazy: lazy.into_iter().collect(),
            headline,
        },
    })
}

pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "systems.next_actions" => handle(input, |i: NextIn| next_actions(&i)),
        "systems.next_levers" => handle(input, |i: levers::LeversIn| levers::levers_for_metric(&i)),
        "systems.solve_lever" => handle(input, |i: levers::SolveIn| levers::solve(&i)),
        _ => None,
    }
}

/// Plain-text rendering used by `protoforge next [project.json] [--all] [--levers all|none]`.
/// Returns the text and whether no binding requirement fails.
pub fn render_cli(p: &Project, all: bool, levers: Option<&str>) -> Result<(String, bool), String> {
    let out = next_actions(&NextIn {
        project: p.clone(),
        evaluation: None,
        levers: levers.map(String::from),
        limit: None,
    })?;
    let s = &out.summary;
    let mut o = format!("{} v{} — what to do next\n", p.meta.name, p.meta.version);
    o += &format!(
        "{:.0} % ready: {} of {} requirements pass with ≥ {:.0} % margin · {} marginal · {} open goal(s) · {} failing · {} unverified\n\n",
        s.readiness_pct,
        s.pass_with_margin,
        s.requirements,
        s.marginal_threshold * 100.0,
        s.marginal,
        s.open_goals,
        s.failing,
        s.unverified
    );
    let shown = if all { out.actions.len() } else { out.actions.len().min(10) };
    for (k, a) in out.actions.iter().take(shown).enumerate() {
        o += &format!(
            "{:>2}. [{}] {} · score {:.1}\n",
            k + 1,
            a.severity.to_uppercase(),
            a.kind.replace('_', " "),
            a.score
        );
        o += &format!("    {}\n", a.title);
        if let (Some(v), Some(c)) = (a.value, &a.criterion) {
            o += &format!(
                "    now {} {} → {}{}\n",
                levers::fmt_num(v),
                a.unit,
                c,
                a.margin.map(|m| format!(" (margin {:+.1} %)", m * 100.0)).unwrap_or_default()
            );
        }
        o += &format!("    go to: {}  (#{})\n", a.location.label, a.location.href);
        match a.levers_status.as_str() {
            "measured" => {
                for l in &a.levers {
                    o += &format!(
                        "    {} {}: +{} {} moves it {:+.4} {}{}\n",
                        if l.direction == "increase" { "↑" } else { "↓" },
                        l.label,
                        levers::fmt_num(l.step),
                        l.unit,
                        l.delta,
                        a.unit,
                        l.estimate
                            .map(|e| format!(" · ≈ {} {} reaches the target", levers::fmt_num(e), l.unit))
                            .unwrap_or_default()
                    );
                }
            }
            "lazy" => o += "    levers: run with --levers all (re-runs a finite-element / layout model per parameter)\n",
            "none" => o += "    levers: no numeric design parameter moves this metric measurably\n",
            _ => {}
        }
        o += "\n";
    }
    if shown < out.actions.len() {
        o += &format!("… {} more (use --all)\n", out.actions.len() - shown);
    }
    Ok((o, s.failing == 0))
}

#[cfg(test)]
mod tests;
