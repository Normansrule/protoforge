//! Requirement quality checks after the INCOSE *Guide to Writing
//! Requirements* (GtWR, INCOSE Requirements Working Group).
//!
//! The guide lists characteristics of a well-formed requirement — among them
//! *necessary*, *appropriate*, *unambiguous*, *complete*, *singular*,
//! *feasible*, *verifiable*, *correct* and *conforming* — and rules that help
//! achieve them (avoid vague terms and escape clauses, one thought per
//! requirement, state the rationale, ...). [`quality`] runs the structural
//! checks of [`super::lint`] and tags each finding with the characteristic
//! it threatens plus a hint for fixing it, then adds text-level checks that
//! a machine can do reliably. It cannot judge feasibility or necessity — a
//! reviewer still has to.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::lint;
use crate::project::{Cmp, Metric, Project};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityIssue {
    /// error | warning | info
    pub severity: String,
    /// Requirement, need or interface id.
    pub id: String,
    pub message: String,
    /// INCOSE characteristic (or attribute) the finding threatens.
    pub characteristic: String,
    pub hint: String,
}

/// Vague words that make a requirement unverifiable.
const VAGUE: &[&str] = &[
    "adequate",
    "appropriate",
    "approximately",
    "easy",
    "easily",
    "efficient",
    "etc",
    "fast",
    "flexible",
    "maximize",
    "minimize",
    "optimize",
    "quickly",
    "reasonable",
    "robust",
    "sufficient",
    "support",
    "tbd",
    "tbc",
    "user-friendly",
    "state-of-the-art",
    "various",
    "several",
];

/// Escape clauses that make a requirement optional in disguise.
const ESCAPES: &[&str] = &[
    "if possible",
    "as far as possible",
    "where possible",
    "if practical",
    "as appropriate",
    "as required",
    "but not limited to",
];

fn classify(message: &str) -> (&'static str, &'static str) {
    if message.starts_with("duplicate") {
        (
            "Conforming",
            "Give every requirement a unique identifier so it can be traced and changed safely.",
        )
    } else if message.starts_with("not traced") {
        (
            "Necessary (traceable)",
            "Link it to the stakeholder need it serves; an untraced requirement may not be needed at all.",
        )
    } else if message.starts_with("derives from unknown")
        || message.starts_with("allocated to unknown")
        || message.starts_with("interface endpoint")
    {
        ("Correct", "Fix the dangling reference or create the missing need / subsystem.")
    } else if message.starts_with("not allocated") {
        (
            "Allocated",
            "Allocate it to the subsystem(s) responsible for meeting it (INCOSE: every requirement has an owner).",
        )
    } else if message.starts_with("no metric") {
        (
            "Verifiable",
            "Bind a lab metric and criterion, or plan a test, inspection or demonstration and record its result.",
        )
    } else if message.starts_with("ambiguous word") {
        (
            "Unambiguous",
            "Replace the vague word with a measurable quantity, unit and tolerance.",
        )
    } else if message.starts_with("use \"shall\"") {
        (
            "Conforming",
            "Write binding requirements as \"The <entity> shall <action> <object> <measure>\".",
        )
    } else if message.starts_with("need has no") {
        (
            "Complete (set)",
            "Derive at least one verifiable requirement from this need, or remove the need.",
        )
    } else {
        ("Conforming", "")
    }
}

fn words(t: &str) -> Vec<&str> {
    t.split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|w| !w.is_empty())
        .collect()
}

/// Numbers written in the requirement text ("1,000" → 1000).
pub fn numbers_in(text: &str) -> Vec<f64> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let comma_in_number = c == ',' && !cur.is_empty() && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit());
        if c.is_ascii_digit() || (c == '.' && !cur.is_empty()) {
            cur.push(c);
        } else if comma_in_number {
            // thousands separator
        } else if !cur.is_empty() {
            if let Ok(v) = cur.trim_end_matches('.').parse() {
                out.push(v);
            }
            cur.clear();
        }
    }
    if let Ok(v) = cur.trim_end_matches('.').parse() {
        out.push(v);
    }
    out
}

fn norm_unit(u: &str) -> String {
    u.trim()
        .to_lowercase()
        .replace('°', "")
        .replace("µ", "u")
        .replace("deg", "")
        .replace(' ', "")
}

/// Requirement-quality findings (structural lint + text checks).
pub fn quality(p: &Project) -> Vec<QualityIssue> {
    quality_with(p, None)
}

/// Like [`quality`], also checking bound metrics exist and units agree when
/// the computed metrics are supplied.
pub fn quality_with(p: &Project, metrics: Option<&[Metric]>) -> Vec<QualityIssue> {
    let mut out: Vec<QualityIssue> = lint(p)
        .into_iter()
        .map(|i| {
            let (c, h) = classify(&i.message);
            QualityIssue {
                severity: i.severity,
                id: i.id,
                message: i.message,
                characteristic: c.into(),
                hint: h.into(),
            }
        })
        .collect();
    let mut push = |sev: &str, id: &str, msg: String, ch: &str, hint: &str| {
        out.push(QualityIssue {
            severity: sev.into(),
            id: id.into(),
            message: msg,
            characteristic: ch.into(),
            hint: hint.into(),
        })
    };
    let already = ["user-friendly", "fast", "easy", "adequate", "as appropriate", "etc", "tbd"];
    let mut need_ids = BTreeSet::new();
    for n in &p.needs {
        if !need_ids.insert(n.id.as_str()) {
            push(
                "error",
                &n.id,
                "duplicate need id".into(),
                "Conforming",
                "Give every need a unique identifier.",
            );
        }
    }
    for r in &p.requirements {
        let t = r.text.to_lowercase();
        let ws = words(&t);
        for v in VAGUE {
            if !already.contains(v) && ws.contains(v) {
                push(
                    "warning",
                    &r.id,
                    format!("vague term \"{v}\""),
                    "Unambiguous",
                    "State the measurable property and its limit instead (e.g. \"at least 8 h\" rather than \"sufficient\").",
                );
            }
        }
        for e in ESCAPES {
            if t.contains(e) {
                push(
                    "warning",
                    &r.id,
                    format!("escape clause \"{e}\""),
                    "Verifiable",
                    "Remove the escape clause; if the requirement is optional, make it a goal instead.",
                );
            }
        }
        if t.contains("and/or") {
            push(
                "warning",
                &r.id,
                "\"and/or\" is ambiguous".into(),
                "Unambiguous",
                "Say \"and\" or \"or\", or split the requirement.",
            );
        }
        let shalls = ws.iter().filter(|w| **w == "shall").count();
        if shalls > 1 {
            push(
                "warning",
                &r.id,
                format!("{shalls} \"shall\" statements in one requirement"),
                "Singular",
                "Split it so each requirement states one capability or constraint.",
            );
        }
        if t.contains("shall not") {
            push(
                "info",
                &r.id,
                "negative requirement (\"shall not\")".into(),
                "Verifiable",
                "Prefer a positive, bounded statement: proving an absence is hard to verify.",
            );
        }
        if r.rationale.trim().is_empty() {
            push(
                "info",
                &r.id,
                "no rationale recorded".into(),
                "Necessary",
                "Record why the requirement exists and where its number came from.",
            );
        }
        if r.text.chars().count() > 240 {
            push(
                "info",
                &r.id,
                "long requirement text".into(),
                "Comprehensible",
                "Keep one requirement to one short sentence; move context to the rationale.",
            );
        }
        let v = &r.verification;
        if v.metric.is_some() {
            if v.cmp == Cmp::Between && v.target_hi.is_none() {
                push(
                    "error",
                    &r.id,
                    "range criterion without an upper bound".into(),
                    "Complete",
                    "Set both ends of the range.",
                );
            }
            if v.cmp == Cmp::Eq && v.target_hi.unwrap_or(0.0) == 0.0 {
                push(
                    "info",
                    &r.id,
                    "equality with zero tolerance".into(),
                    "Feasible",
                    "Give a tolerance: real values never equal a target exactly.",
                );
            }
            let nums = numbers_in(&r.text);
            let targets = [Some(v.target), v.target_hi];
            let close = |a: f64, b: f64| (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.0);
            if !nums.is_empty() && !targets.iter().flatten().any(|t| nums.iter().any(|n| close(*n, *t))) {
                push(
                    "warning",
                    &r.id,
                    format!("text numbers {nums:?} do not include the criterion target {}", v.target),
                    "Consistent",
                    "Make the statement and the verification criterion say the same thing.",
                );
            }
        }
        if let (Some(ms), Some(key)) = (metrics, v.metric.as_deref()) {
            match ms.iter().find(|m| m.key == key) {
                None => push(
                    "warning",
                    &r.id,
                    format!("metric {key} is not produced by any lab"),
                    "Verifiable",
                    "Pick a metric from the list the labs produce, or open the lab that should compute it.",
                ),
                Some(m) => {
                    if !v.unit.is_empty() && !m.unit.is_empty() && norm_unit(&v.unit) != norm_unit(&m.unit) {
                        push(
                            "warning",
                            &r.id,
                            format!("criterion unit \"{}\" differs from metric unit \"{}\"", v.unit, m.unit),
                            "Correct",
                            "Use the metric's unit so the comparison is meaningful.",
                        );
                    }
                }
            }
        }
    }
    let rank = |s: &str| match s {
        "error" => 0,
        "warning" => 1,
        _ => 2,
    };
    out.sort_by(|a, b| rank(&a.severity).cmp(&rank(&b.severity)).then(a.id.cmp(&b.id)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Requirement, Verification};

    #[test]
    fn numbers_are_parsed() {
        assert_eq!(
            numbers_in("Unit cost at 1,000 units shall be at most 120 USD."),
            vec![1000.0, 120.0]
        );
        assert_eq!(numbers_in("between 1 m and 3.5 m."), vec![1.0, 3.5]);
    }

    #[test]
    fn flags_vague_compound_and_inconsistent() {
        let mut p = Project::blank("q");
        p.requirements.push(Requirement {
            id: "R-1".into(),
            text: "The device shall be robust and shall weigh 40 g if possible.".into(),
            verification: Verification {
                metric: Some("mass.total_g".into()),
                target: 50.0,
                unit: "g".into(),
                ..Default::default()
            },
            ..Default::default()
        });
        let q = quality(&p);
        let has = |s: &str| q.iter().any(|i| i.message.contains(s));
        assert!(has("vague term \"robust\""));
        assert!(has("escape clause"));
        assert!(has("2 \"shall\""));
        assert!(has("do not include the criterion target"));
        assert!(q.iter().any(|i| i.characteristic == "Allocated"));
    }

    #[test]
    fn iris_has_no_quality_errors_and_units_agree() {
        let p = Project::iris();
        let ev = crate::systems::evaluate(&p);
        let q = quality_with(&p, Some(&ev.metrics));
        let errs: Vec<_> = q.iter().filter(|i| i.severity == "error").collect();
        assert!(errs.is_empty(), "{errs:?}");
        let unit: Vec<_> = q.iter().filter(|i| i.message.contains("differs from metric unit")).collect();
        assert!(unit.is_empty(), "{unit:?}");
    }
}
