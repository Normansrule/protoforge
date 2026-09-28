//! A full systems-engineering report in Markdown: concept, needs,
//! requirement verification, coverage, budgets, architecture, trade
//! studies, Failure Mode and Effects Analysis (FMEA) and milestone gates —
//! the document a design review reads. The shorter verification-only report
//! is [`super::report_markdown`].

use super::analysis::{coverage, gates, n2, trade_detail};
use super::budgets::{budgets, Budget};
use super::{fmea, Evaluation, Status};
use crate::project::{Cmp, Project};

fn esc(s: &str) -> String {
    s.replace('|', "/").replace('\n', " ")
}

fn num(x: f64) -> String {
    let a = x.abs();
    if a >= 1000.0 {
        format!("{x:.0}")
    } else if a >= 10.0 {
        format!("{x:.1}")
    } else if a >= 0.1 {
        format!("{x:.2}")
    } else {
        format!("{x:.3}")
    }
}

fn status_word(s: Status) -> &'static str {
    match s {
        Status::Pass => "✅ pass",
        Status::Fail => "❌ fail",
        Status::Unverified => "⚪ unverified",
    }
}

fn budget_row(b: &Budget) -> String {
    let limit = match (b.limit, b.cmp) {
        (Some(l), Some(Cmp::Ge)) => format!("≥ {} {}", num(l), b.unit),
        (Some(l), _) => format!("≤ {} {}", num(l), b.unit),
        _ => "—".into(),
    };
    let margin = b.margin.map(|m| format!("{:+.1} %", m * 100.0)).unwrap_or_else(|| "—".into());
    format!(
        "| {} | {} {} | {} | {} | {} | {} |\n",
        b.label,
        num(b.total),
        b.unit,
        limit,
        margin,
        b.requirement.clone().unwrap_or_else(|| "—".into()),
        status_word(b.status)
    )
}

/// The full systems-engineering report.
pub fn report_full_markdown(p: &Project, ev: &Evaluation) -> String {
    let mut s = String::new();
    let m = &p.meta;
    s += &format!("# {} — systems engineering report\n\n", m.name);
    if !m.tagline.is_empty() {
        s += &format!("*{}*\n\n", m.tagline);
    }
    s += &format!(
        "Codename **{}** · version {} · {} requirements: **{} pass**, {} fail, {} unverified\n\n",
        if m.codename.is_empty() { "—" } else { &m.codename },
        m.version,
        ev.results.len(),
        ev.pass,
        ev.fail,
        ev.unverified
    );
    if !m.description.is_empty() {
        s += &format!("{}\n\n", m.description);
    }

    // concept
    let c = &p.concept;
    s += "## 1. Concept\n\n";
    if !c.problem.is_empty() {
        s += &format!("**Problem.** {}\n\n", c.problem);
    }
    if !c.vision.is_empty() {
        s += &format!("**Vision.** {}\n\n", c.vision);
    }
    if !c.users.is_empty() {
        s += "**Users**\n\n";
        for u in &c.users {
            s += &format!("- {u}\n");
        }
        s += "\n";
    }
    if !c.use_cases.is_empty() {
        s += "**Use cases**\n\n";
        for u in &c.use_cases {
            s += &format!("- **{} {}** — {}\n", u.id, u.title, u.narrative);
        }
        s += "\n";
    }
    if !c.constraints.is_empty() {
        s += "**Constraints**\n\n";
        for u in &c.constraints {
            s += &format!("- {u}\n");
        }
        s += "\n";
    }

    // needs & coverage
    let cov = coverage(p, ev);
    s += "## 2. Stakeholder needs and coverage\n\n| Need | Statement | Priority | Requirements | Status |\n|---|---|---|---|---|\n";
    for n in &cov.needs {
        s += &format!(
            "| {} | {} | {} | {} | {} |\n",
            n.id,
            esc(&n.text),
            match n.priority {
                1 => "must",
                2 => "should",
                _ => "could",
            },
            if n.requirements.is_empty() {
                "—".into()
            } else {
                n.requirements.join(", ")
            },
            n.status
        );
    }
    s += &format!(
        "\nCoverage: {}/{} needs covered, {}/{} satisfied · {}/{} requirements traced, {} allocated, {} bound to a metric · traceability completeness {:.0} %\n\n",
        cov.needs_covered,
        cov.needs_total,
        cov.needs_satisfied,
        cov.needs_total,
        cov.reqs_traced,
        cov.reqs_total,
        cov.reqs_allocated,
        cov.reqs_with_metric,
        cov.completeness * 100.0
    );

    // requirements
    s += "## 3. Requirement verification\n\n| ID | Requirement | Criterion | Value | Margin | Status | Source |\n|---|---|---|---|---|---|---|\n";
    for (r, res) in p.requirements.iter().zip(ev.results.iter()) {
        s += &format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            r.id,
            esc(&r.text),
            res.criterion,
            res.value.map(|v| format!("{} {}", num(v), res.unit)).unwrap_or_else(|| "—".into()),
            res.margin.map(|m| format!("{:+.0} %", m * 100.0)).unwrap_or_else(|| "—".into()),
            status_word(res.status),
            res.source.clone().unwrap_or_default()
        );
    }
    s += "\n";

    // budgets
    let b = budgets(p, ev);
    s += "## 4. Technical budgets\n\n| Budget | Roll-up | Limit | Margin | Requirement | Status |\n|---|---|---|---|---|---|\n";
    for x in [&b.mass, &b.power, &b.battery_life, &b.cost] {
        s += &budget_row(x);
    }
    s += "\n| Subsystem | Parts | Mass (g) | Cost (USD, qty 1) | Average power (mW) |\n|---|---|---|---|---|\n";
    for r in &b.subsystems {
        s += &format!(
            "| {} | {} | {} | {} | {} |\n",
            esc(&r.name),
            r.components,
            num(r.mass_g),
            num(r.cost_usd),
            num(r.power_mw)
        );
    }
    s += "\n";

    // architecture
    s += "## 5. Architecture\n\n| Subsystem | Domain | Functions | Lab |\n|---|---|---|---|\n";
    for x in &p.subsystems {
        s += &format!(
            "| {} {} | {} | {} | {} |\n",
            x.id,
            esc(&x.name),
            serde_json::to_value(x.domain)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            esc(&x.functions.join("; ")),
            x.lab
        );
    }
    let nn = n2(p);
    s += &format!(
        "\n{} interfaces · N² feedback marks: {} as listed, {} after design-structure sequencing\n\n| ID | From | To | Kind | Description |\n|---|---|---|---|---|\n",
        p.interfaces.len(),
        nn.feedback,
        nn.suggested_feedback
    );
    for i in &p.interfaces {
        s += &format!("| {} | {} | {} | {} | {} |\n", i.id, i.from, i.to, i.kind, esc(&i.description));
    }
    s += "\n";

    // trades
    if !p.trades.is_empty() {
        s += "## 6. Trade studies\n\n| Study | Weighted winner | Score | Lead | Recorded decision |\n|---|---|---|---|---|\n";
        for t in &p.trades {
            let d = trade_detail(t);
            let w = t.options.get(d.result.winner).map(|o| o.name.clone()).unwrap_or_default();
            s += &format!(
                "| {} {} | {} | {} | {} | {}{} |\n",
                t.id,
                esc(&t.title),
                esc(&w),
                d.result.scores.get(d.result.winner).map(|x| format!("{x:.2}")).unwrap_or_default(),
                format_args!("{:.2}", d.lead),
                esc(&t.decision),
                if d.decision_agrees { "" } else { " ⚠ differs" }
            );
        }
        s += "\n";
    }

    // FMEA
    let f = fmea(p);
    if !f.is_empty() {
        s += "## 7. Failure Mode and Effects Analysis (FMEA)\n\n| ID | Item | Failure mode | S | O | D | RPN | Level | Mitigation |\n|---|---|---|---|---|---|---|---|---|\n";
        for r in &f {
            if let Some(x) = p.risks.iter().find(|x| x.id == r.id) {
                s += &format!(
                    "| {} | {} | {} | {} | {} | {} | **{}** | {} | {} |\n",
                    x.id,
                    esc(&x.item),
                    esc(&x.failure_mode),
                    x.severity,
                    x.occurrence,
                    x.detection,
                    r.rpn,
                    r.level,
                    esc(&x.mitigation)
                );
            }
        }
        s += "\n";
    }

    // gates
    let g = gates(p, ev);
    if !g.is_empty() {
        s += "## 8. Milestones and gates\n\n";
        for x in &g {
            s += &format!(
                "### {} {} ({}){}\n\n",
                x.id,
                x.title,
                x.stage,
                if x.passed {
                    " — passed"
                } else if x.current {
                    " — current gate"
                } else {
                    ""
                }
            );
            for c in &x.criteria {
                let mark = match c.status.as_str() {
                    "met" => "[x]",
                    _ => "[ ]",
                };
                s += &format!("- {mark} {} — {}\n", c.text, c.detail);
            }
            s += "\n";
        }
    }

    s += "## 9. All metrics\n\n| Key | Label | Value | Note |\n|---|---|---|---|\n";
    for x in &ev.metrics {
        s += &format!("| `{}` | {} | {} {} | {} |\n", x.key, x.label, num(x.value), x.unit, esc(&x.note));
    }
    s += "\n---\nGenerated by the Protoforge Systems Studio. References: NASA Systems Engineering Handbook (NASA/SP-2016-6105 Rev 2); INCOSE Systems Engineering Handbook (5th ed., 2023); ISO/IEC/IEEE 15288:2023; AIAG & VDA FMEA Handbook (2019).\n";
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::evaluate;

    #[test]
    fn full_report_has_every_section() {
        let p = Project::iris();
        let ev = evaluate(&p);
        let r = report_full_markdown(&p, &ev);
        for h in [
            "## 1. Concept",
            "## 2.",
            "## 3.",
            "## 4. Technical budgets",
            "## 5.",
            "## 6.",
            "## 7. Failure Mode",
            "## 8.",
            "## 9.",
        ] {
            assert!(r.contains(h), "missing {h}");
        }
        assert!(r.contains("R-024"));
        let blank = Project::blank("x");
        let rb = report_full_markdown(&blank, &evaluate(&blank));
        assert!(rb.contains("## 3."));
    }
}
