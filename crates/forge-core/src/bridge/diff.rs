//! Version comparison for Onshape-style history: structured differences
//! between two projects (design parameters, requirements, BOM lines,
//! subsystems, interfaces, risks, trades …), requirement **verdict changes**
//! from evaluating both versions with every lab (`systems::evaluate`),
//! metric changes, and Circuit Lab checks that changed because a component
//! value changed.
//!
//! In configuration-management terms a saved version is a *baseline* and
//! this report is the *configuration status accounting* between two
//! baselines: what changed, and what the change did to verified performance
//! (ISO 10007:2017 *Quality management — Guidelines for configuration
//! management*; NASA/SP-2016-6105 Rev 2, *NASA Systems Engineering
//! Handbook*, configuration management process).

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use super::util::fnv1a;
use crate::project::Project;
use crate::systems::{self, Evaluation, Status};

#[derive(Clone, Debug, Default, Serialize)]
pub struct EvalCounts {
    pub pass: usize,
    pub fail: usize,
    pub unverified: usize,
}

/// Short fingerprint and key statistics of one project version.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Summary {
    /// FNV-1a 64-bit hash of the canonical project JSON (16 hex digits).
    pub hash: String,
    /// First 7 hex digits, like a short commit id.
    pub short: String,
    pub name: String,
    pub version: String,
    pub needs: usize,
    pub requirements: usize,
    pub subsystems: usize,
    pub interfaces: usize,
    pub components: usize,
    pub risks: usize,
    pub trades: usize,
    pub bom_cost_usd: f64,
    pub component_mass_g: f64,
    pub evaluation: Option<EvalCounts>,
}

pub fn summary(p: &Project, ev: Option<&Evaluation>) -> Summary {
    let json = serde_json::to_string(p).unwrap_or_default();
    let h = format!("{:016x}", fnv1a(json.as_bytes()));
    Summary {
        short: h[..7].to_string(),
        hash: h,
        name: p.meta.name.clone(),
        version: p.meta.version.clone(),
        needs: p.needs.len(),
        requirements: p.requirements.len(),
        subsystems: p.subsystems.len(),
        interfaces: p.interfaces.len(),
        components: p.components.len(),
        risks: p.risks.len(),
        trades: p.trades.len(),
        bom_cost_usd: p.bom_cost_usd(),
        component_mass_g: p.component_mass_g(),
        evaluation: ev.map(|e| EvalCounts {
            pass: e.pass,
            fail: e.fail,
            unverified: e.unverified,
        }),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ValueChange {
    pub path: String,
    pub old: Option<Value>,
    pub new: Option<Value>,
    /// new − old for numbers.
    pub delta: Option<f64>,
    /// relative change in percent for numbers (old ≠ 0).
    pub rel_pct: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ItemChange {
    /// "added", "removed" or "changed"
    pub kind: String,
    pub key: String,
    pub label: String,
    pub fields: Vec<ValueChange>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Section {
    pub id: String,
    pub title: String,
    pub items: Vec<ItemChange>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReqRow {
    pub id: String,
    pub text: String,
    pub criterion: String,
    pub status_a: Option<String>,
    pub status_b: Option<String>,
    pub value_a: Option<f64>,
    pub value_b: Option<f64>,
    /// Signed margin as a fraction of the target (positive = inside the limit).
    pub margin_a: Option<f64>,
    pub margin_b: Option<f64>,
    pub unit: String,
    /// "same", "value", "improved", "regressed", "added", "removed"
    pub change: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct MetricChange {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub source: String,
    pub old: Option<f64>,
    pub new: Option<f64>,
    pub delta: Option<f64>,
    pub rel_pct: Option<f64>,
    pub note: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CheckChange {
    pub preset: String,
    pub preset_title: String,
    pub key: String,
    pub label: String,
    pub unit: String,
    pub old: Option<f64>,
    pub new: Option<f64>,
    pub pass_old: Option<bool>,
    pub pass_new: Option<bool>,
    pub note: String,
    pub changed: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct DiffCounts {
    pub params: usize,
    pub items_added: usize,
    pub items_removed: usize,
    pub items_changed: usize,
    pub verdicts_changed: usize,
    pub metrics_changed: usize,
    pub checks_changed: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Diff {
    pub a: Summary,
    pub b: Summary,
    pub identical: bool,
    pub meta: Vec<ValueChange>,
    pub concept: Vec<ValueChange>,
    /// Changed design parameters (`design.*` leaves).
    pub params: Vec<ValueChange>,
    pub sections: Vec<Section>,
    pub requirements_status: Vec<ReqRow>,
    pub metrics: Vec<MetricChange>,
    pub circuit_checks: Vec<CheckChange>,
    pub counts: DiffCounts,
    /// Plain-language story of what changed and what it did.
    pub headline: Vec<String>,
}

/// Flatten JSON into `path → leaf`. Arrays of objects with an `id` or
/// `refdes` field are keyed by it; other arrays are leaves.
fn flatten(v: &Value, path: &str, out: &mut BTreeMap<String, Value>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                let p = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                flatten(x, &p, out);
            }
        }
        Value::Array(a)
            if !a.is_empty()
                && a.iter()
                    .all(|x| x.get("id").or_else(|| x.get("refdes")).is_some_and(|k| k.is_string())) =>
        {
            for x in a {
                let k = x.get("id").or_else(|| x.get("refdes")).and_then(|k| k.as_str()).unwrap_or("");
                flatten(x, &format!("{path}[{k}]"), out);
            }
        }
        _ => {
            out.insert(path.to_string(), v.clone());
        }
    }
}

fn changes(a: &Value, b: &Value, prefix: &str) -> Vec<ValueChange> {
    let (mut fa, mut fb) = (BTreeMap::new(), BTreeMap::new());
    flatten(a, prefix, &mut fa);
    flatten(b, prefix, &mut fb);
    let mut keys: Vec<&String> = fa.keys().chain(fb.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut out = Vec::new();
    for k in keys {
        let (x, y) = (fa.get(k), fb.get(k));
        if x == y {
            continue;
        }
        // numerically equal (e.g. 8 vs 8.0) is not a change
        if let (Some(Value::Number(p)), Some(Value::Number(q))) = (x, y) {
            if p.as_f64() == q.as_f64() {
                continue;
            }
        }
        let nums = (x.and_then(|v| v.as_f64()), y.and_then(|v| v.as_f64()));
        let (delta, rel) = match nums {
            (Some(p), Some(q)) => (Some(q - p), (p != 0.0).then(|| 100.0 * (q - p) / p.abs())),
            _ => (None, None),
        };
        out.push(ValueChange {
            path: k.clone(),
            old: x.cloned(),
            new: y.cloned(),
            delta,
            rel_pct: rel,
        });
    }
    out
}

fn items(a: &[Value], b: &[Value], key: &str, label: impl Fn(&Value) -> String) -> Vec<ItemChange> {
    let k = |v: &Value| v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let mut out = Vec::new();
    for x in a {
        match b.iter().find(|y| k(y) == k(x)) {
            None => out.push(ItemChange {
                kind: "removed".into(),
                key: k(x),
                label: label(x),
                fields: vec![],
            }),
            Some(y) => {
                let f = changes(x, y, "");
                if !f.is_empty() {
                    out.push(ItemChange {
                        kind: "changed".into(),
                        key: k(x),
                        label: label(y),
                        fields: f,
                    });
                }
            }
        }
    }
    for y in b {
        if !a.iter().any(|x| k(x) == k(y)) {
            out.push(ItemChange {
                kind: "added".into(),
                key: k(y),
                label: label(y),
                fields: changes(&Value::Null, y, "").into_iter().filter(|c| c.new.is_some()).collect(),
            });
        }
    }
    out
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn status_word(st: Status) -> &'static str {
    match st {
        Status::Pass => "pass",
        Status::Fail => "fail",
        Status::Unverified => "unverified",
    }
}

fn rank(s: Option<&str>) -> i32 {
    match s {
        Some("pass") => 2,
        Some("unverified") => 1,
        Some("fail") => 0,
        _ => -1,
    }
}

pub fn fmt(v: f64) -> String {
    let a = v.abs();
    if a == 0.0 {
        "0".into()
    } else if a >= 100.0 {
        format!("{v:.0}")
    } else if a >= 10.0 {
        format!("{v:.1}")
    } else if a >= 0.1 {
        format!("{v:.2}")
    } else {
        format!("{v:.3}")
    }
}

/// Compare version `a` (older) with version `b` (newer).
pub fn diff(a: &Project, b: &Project) -> Diff {
    let ja = serde_json::to_value(a).unwrap_or(Value::Null);
    let jb = serde_json::to_value(b).unwrap_or(Value::Null);
    let eva = systems::evaluate(a);
    let evb = systems::evaluate(b);
    let meta = changes(&ja["meta"], &jb["meta"], "meta");
    let concept = changes(&ja["concept"], &jb["concept"], "concept");
    let params = changes(&ja["design"], &jb["design"], "design");
    let arr = |j: &Value, k: &str| j[k].as_array().cloned().unwrap_or_default();
    let sections = vec![
        Section {
            id: "requirements".into(),
            title: "Requirements".into(),
            items: items(&arr(&ja, "requirements"), &arr(&jb, "requirements"), "id", |v| s(v, "text")),
        },
        Section {
            id: "components".into(),
            title: "Bill of materials".into(),
            items: items(&arr(&ja, "components"), &arr(&jb, "components"), "refdes", |v| s(v, "name")),
        },
        Section {
            id: "needs".into(),
            title: "Stakeholder needs".into(),
            items: items(&arr(&ja, "needs"), &arr(&jb, "needs"), "id", |v| s(v, "text")),
        },
        Section {
            id: "subsystems".into(),
            title: "Subsystems".into(),
            items: items(&arr(&ja, "subsystems"), &arr(&jb, "subsystems"), "id", |v| s(v, "name")),
        },
        Section {
            id: "interfaces".into(),
            title: "Interfaces".into(),
            items: items(&arr(&ja, "interfaces"), &arr(&jb, "interfaces"), "id", |v| {
                format!("{} → {} ({})", s(v, "from"), s(v, "to"), s(v, "kind"))
            }),
        },
        Section {
            id: "risks".into(),
            title: "Risks (FMEA)".into(),
            items: items(&arr(&ja, "risks"), &arr(&jb, "risks"), "id", |v| {
                format!("{}: {}", s(v, "item"), s(v, "failure_mode"))
            }),
        },
        Section {
            id: "trades".into(),
            title: "Trade studies".into(),
            items: items(&arr(&ja, "trades"), &arr(&jb, "trades"), "id", |v| s(v, "title")),
        },
        Section {
            id: "milestones".into(),
            title: "Milestones".into(),
            items: items(&arr(&ja, "milestones"), &arr(&jb, "milestones"), "id", |v| s(v, "title")),
        },
    ];
    // Requirement verdicts.
    let mut ids: Vec<String> = a.requirements.iter().map(|r| r.id.clone()).collect();
    for r in &b.requirements {
        if !ids.contains(&r.id) {
            ids.push(r.id.clone());
        }
    }
    let mut rows = Vec::new();
    for id in &ids {
        let ra = eva.results.iter().find(|r| &r.id == id);
        let rb = evb.results.iter().find(|r| &r.id == id);
        let text = b
            .requirements
            .iter()
            .chain(a.requirements.iter())
            .find(|r| &r.id == id)
            .map(|r| r.text.clone())
            .unwrap_or_default();
        let sa = ra.map(|r| status_word(r.status).to_string());
        let sb = rb.map(|r| status_word(r.status).to_string());
        let (va, vb) = (ra.and_then(|r| r.value), rb.and_then(|r| r.value));
        let change = match (&sa, &sb) {
            (None, Some(_)) => "added",
            (Some(_), None) => "removed",
            _ if rank(sa.as_deref()) < rank(sb.as_deref()) => "improved",
            _ if rank(sa.as_deref()) > rank(sb.as_deref()) => "regressed",
            _ => match (va, vb) {
                (Some(x), Some(y)) if (x - y).abs() > 1e-9 * (1.0 + x.abs()) => "value",
                _ => "same",
            },
        };
        rows.push(ReqRow {
            id: id.clone(),
            text,
            criterion: rb.or(ra).map(|r| r.criterion.clone()).unwrap_or_default(),
            status_a: sa,
            status_b: sb,
            value_a: va,
            value_b: vb,
            margin_a: ra.and_then(|r| r.margin),
            margin_b: rb.and_then(|r| r.margin),
            unit: rb.or(ra).map(|r| r.unit.clone()).unwrap_or_default(),
            change: change.into(),
        });
    }
    // Metrics.
    let mut metrics = Vec::new();
    let mut keys: Vec<&str> = eva.metrics.iter().chain(evb.metrics.iter()).map(|m| m.key.as_str()).collect();
    keys.sort();
    keys.dedup();
    for k in keys {
        let ma = eva.metrics.iter().find(|m| m.key == k);
        let mb = evb.metrics.iter().find(|m| m.key == k);
        let (x, y) = (ma.map(|m| m.value), mb.map(|m| m.value));
        let changed = match (x, y) {
            (Some(p), Some(q)) => (p - q).abs() > 1e-9 * (1.0 + p.abs()),
            (None, None) => false,
            _ => true,
        };
        if !changed {
            continue;
        }
        let m = mb.or(ma).unwrap();
        metrics.push(MetricChange {
            key: k.to_string(),
            label: m.label.clone(),
            unit: m.unit.clone(),
            source: m.source.clone(),
            old: x,
            new: y,
            delta: x.zip(y).map(|(p, q)| q - p),
            rel_pct: x.zip(y).and_then(|(p, q)| (p != 0.0).then(|| 100.0 * (q - p) / p.abs())),
            note: mb.map(|m| m.note.clone()).unwrap_or_default(),
        });
    }
    metrics.sort_by(|p, q| {
        let r = |m: &MetricChange| m.rel_pct.map(|x| x.abs()).unwrap_or(f64::INFINITY);
        r(q).partial_cmp(&r(p)).unwrap_or(std::cmp::Ordering::Equal)
    });
    // Circuit Lab checks whose inputs changed.
    let mut checks = Vec::new();
    for pr in crate::circuit::presets::list(None) {
        let (pa, pb) = (
            crate::circuit::presets::project_params(&pr.id, a),
            crate::circuit::presets::project_params(&pr.id, b),
        );
        if pa == pb {
            continue;
        }
        let empty = BTreeMap::new();
        let (Ok(xa), Ok(xb)) = (
            crate::circuit::presets::run(&pr.id, &empty, None, Some(a)),
            crate::circuit::presets::run(&pr.id, &empty, None, Some(b)),
        ) else {
            continue;
        };
        for mb in &xb.measurements {
            let ma = xa.measurements.iter().find(|m| m.key == mb.key);
            let changed = ma
                .map(|m| (m.value - mb.value).abs() > 1e-9 * (1.0 + m.value.abs()) || m.pass != mb.pass)
                .unwrap_or(true);
            checks.push(CheckChange {
                preset: pr.id.clone(),
                preset_title: pr.title.clone(),
                key: mb.key.clone(),
                label: mb.label.clone(),
                unit: mb.unit.clone(),
                old: ma.map(|m| m.value),
                new: Some(mb.value),
                pass_old: ma.and_then(|m| m.pass),
                pass_new: mb.pass,
                note: if ma.and_then(|m| m.pass) == Some(false) {
                    ma.map(|m| m.note.clone()).unwrap_or_default()
                } else {
                    mb.note.clone()
                },
                changed,
            });
        }
    }
    let counts = DiffCounts {
        params: params.len(),
        items_added: sections.iter().flat_map(|s| &s.items).filter(|i| i.kind == "added").count(),
        items_removed: sections.iter().flat_map(|s| &s.items).filter(|i| i.kind == "removed").count(),
        items_changed: sections.iter().flat_map(|s| &s.items).filter(|i| i.kind == "changed").count(),
        verdicts_changed: rows.iter().filter(|r| !matches!(r.change.as_str(), "same" | "value")).count(),
        metrics_changed: metrics.len(),
        checks_changed: checks.iter().filter(|c| c.changed).count(),
    };
    // Story.
    let mut headline = Vec::new();
    for r in rows.iter().filter(|r| !matches!(r.change.as_str(), "same" | "value")) {
        let glyph = |s: &Option<String>| match s.as_deref() {
            Some("pass") => "✓ pass",
            Some("fail") => "✕ fail",
            Some("unverified") => "○ unverified",
            _ => "—",
        };
        let val = r
            .value_b
            .or(r.value_a)
            .map(|v| format!(" ({} {})", fmt(v), r.unit))
            .unwrap_or_default();
        headline.push(match r.change.as_str() {
            "added" => format!("{} added: {} → {}{val}", r.id, r.text, glyph(&r.status_b)),
            "removed" => format!("{} removed ({} before)", r.id, glyph(&r.status_a)),
            _ => format!("{}: {} → {}{val}", r.id, glyph(&r.status_a), glyph(&r.status_b)),
        });
    }
    // Same verdict, but the margin to the limit halved or doubled: worth a line in a review.
    for r in rows.iter().filter(|r| r.change == "value") {
        if let (Some(ma), Some(mb)) = (r.margin_a, r.margin_b) {
            if ma > 0.0 && mb > 0.0 && (mb < 0.5 * ma || mb > 2.0 * ma) {
                headline.push(format!(
                    "{}: margin {} from {:.1} % to {:.1} % of the target ({} → {} {}, still {})",
                    r.id,
                    if mb < ma { "shrank" } else { "grew" },
                    100.0 * ma,
                    100.0 * mb,
                    r.value_a.map(fmt).unwrap_or_default(),
                    r.value_b.map(fmt).unwrap_or_default(),
                    r.unit,
                    if r.status_b.as_deref() == Some("pass") {
                        "✓ pass"
                    } else {
                        "the same verdict"
                    }
                ));
            }
        }
    }
    for c in checks.iter().filter(|c| c.pass_old != c.pass_new && c.pass_old.is_some()) {
        let g = |p: Option<bool>| match p {
            Some(true) => "✓",
            Some(false) => "✕",
            None => "·",
        };
        headline.push(format!(
            "Circuit Lab · {}: {} {} {} {} → {} {} {}",
            c.preset_title,
            c.label.to_lowercase(),
            c.old.map(fmt).unwrap_or_default(),
            c.unit,
            g(c.pass_old),
            c.new.map(fmt).unwrap_or_default(),
            c.unit,
            g(c.pass_new)
        ));
    }
    for m in metrics
        .iter()
        .filter(|m| matches!((m.old, m.new), (Some(x), Some(y)) if x < 0.0 && y >= 0.0 || x >= 0.0 && y < 0.0))
    {
        headline.push(format!(
            "{} crossed zero: {} → {} {} ({})",
            m.label,
            fmt(m.old.unwrap()),
            fmt(m.new.unwrap()),
            m.unit,
            m.source
        ));
    }
    if !params.is_empty() {
        let list: Vec<String> = params
            .iter()
            .take(4)
            .map(|p| {
                let v = |x: &Option<Value>| x.as_ref().map(|v| v.to_string()).unwrap_or("—".into());
                format!("{} {} → {}", p.path.trim_start_matches("design."), v(&p.old), v(&p.new))
            })
            .collect();
        headline.push(format!(
            "{} design parameter{} changed: {}{}",
            params.len(),
            if params.len() == 1 { "" } else { "s" },
            list.join(", "),
            if params.len() > 4 { " …" } else { "" }
        ));
    }
    let bom = &sections[1].items;
    if !bom.is_empty() {
        headline.push(format!(
            "BOM: {} line{} changed ({})",
            bom.len(),
            if bom.len() == 1 { "" } else { "s" },
            bom.iter().map(|i| format!("{} {}", i.key, i.kind)).collect::<Vec<_>>().join(", ")
        ));
    }
    let identical = ja == jb;
    if identical {
        headline.push("The two versions are identical.".into());
    }
    Diff {
        a: summary(a, Some(&eva)),
        b: summary(b, Some(&evb)),
        identical,
        meta,
        concept,
        params,
        sections,
        requirements_status: rows,
        metrics,
        circuit_checks: checks,
        counts,
        headline,
    }
}

/// A named version for the built-in example history.
#[derive(Clone, Debug, Serialize)]
pub struct ExampleVersion {
    pub name: String,
    pub description: String,
    /// Branch the version was saved on.
    pub branch: String,
    /// Index of the parent version in the list.
    pub parent: Option<usize>,
    /// Configuration-management baseline label, if this version is one.
    pub baseline: Option<String>,
    pub project: Project,
}

/// Reconstruct Iris v0.3 from the shipped v0.4: the combiner 8 mm after the
/// lens, the battery-sense divider at 1 MΩ / 1.2 MΩ and no combiner-clearance
/// requirement (R-025). The descriptions quote numbers computed by the labs.
/// A third, exploratory version on its own branch (a 250 mAh cell) gives the
/// branch, compare and merge tools something to work on; its description
/// states only the change — the labs report what it does when it is compared.
pub fn iris_history() -> Vec<ExampleVersion> {
    let v5 = Project::iris();
    // v0.4: before the round of mechanical / firmware / manufacturing analyses
    let mut v4 = v5.clone();
    v4.meta.version = "0.4.0".into();
    let v05_reqs = [
        "R-026", "R-027", "R-028", "R-029", "R-030", "R-031", "R-032", "R-033", "R-034", "R-035",
    ];
    v4.requirements.retain(|r| !v05_reqs.contains(&r.id.as_str()));
    v4.components.retain(|c| c.refdes != "OPT2");
    v4.design.extra.remove("focus_stack");
    v4.design.extra.insert("stencil_thickness_mm".into(), serde_json::json!(0.12));
    let metric = |p: &Project, key: &str| {
        let mut m = crate::structures::metrics(p);
        m.extend(crate::build::metrics(p));
        m.into_iter().find(|x| x.key == key).map(|x| x.value).unwrap_or(f64::NAN)
    };
    let (f4, f5) = (
        metric(&v4, "structures.focus_error_worst_d"),
        metric(&v5, "structures.focus_error_worst_d"),
    );
    let (s4, s5) = (
        metric(&v4, "build.stencil_min_area_ratio"),
        metric(&v5, "build.stencil_min_area_ratio"),
    );
    let mut v3 = v4.clone();
    v3.meta.version = "0.3.0".into();
    v3.design.optics.lens_to_combiner_mm = 8.0;
    for c in v3.components.iter_mut() {
        match c.refdes.as_str() {
            "R1" => c.name = "VBAT divider top 1 M".into(),
            "R2" => c.name = "VBAT divider bottom 1.2 M".into(),
            _ => {}
        }
    }
    v3.requirements.retain(|r| r.id != "R-025");
    let clearance = |p: &Project| {
        crate::optics::metrics(p)
            .into_iter()
            .find(|m| m.key == "optics.combiner_clearance_mm")
            .map(|m| m.value)
            .unwrap_or(f64::NAN)
    };
    let adc = |p: &Project| {
        crate::circuit::presets::run("vbat_divider", &BTreeMap::new(), None, Some(p))
            .ok()
            .and_then(|r| r.measurements.into_iter().find(|m| m.key == "adc_peak_v"))
            .map(|m| m.value)
            .unwrap_or(f64::NAN)
    };
    let (c3, c4, a3, a4) = (clearance(&v3), clearance(&v4), adc(&v3), adc(&v4));
    let rail = v4.design.power_rails.core_v;
    vec![
        ExampleVersion {
            name: "Iris v0.3 — first integrated design".into(),
            description: format!(
                "Every requirement of the time passed, but the labs flagged two physical problems no requirement covered: the Optics Lab put the 45° combiner {:.1} mm into the collimating lens (lens-to-combiner 8 mm), and the Circuit Lab showed the 1 MΩ / 1.2 MΩ battery-sense divider driving the ADC pin to {:.2} V at a full cell — above the {rail:.1} V rail.",
                -c3, a3
            ),
            branch: "main".into(),
            parent: None,
            baseline: None,
            project: v3,
        },
        ExampleVersion {
            name: "Iris v0.4 — simulation fixes".into(),
            description: format!(
                "Combiner moved to 11 mm after the lens ({c4:+.2} mm clearance) and requirement R-025 added so the interference can never come back; divider changed to 1.5 MΩ / 1 MΩ ({a4:.2} V at the ADC pin, less always-on current)."
            ),
            branch: "main".into(),
            parent: Some(0),
            baseline: Some("Allocated baseline".into()),
            project: v4.clone(),
        },
        {
            let mut big = v4.clone();
            let b = &mut big.design.battery;
            let k = 250.0 / b.capacity_mah;
            b.capacity_mah = 250.0;
            b.mass_g = (b.mass_g * k * 100.0).round() / 100.0;
            big.meta.version = "0.4.1-exp".into();
            let mass = big.design.battery.mass_g;
            // the BOM line carries the mass the mass budget adds up; cost is left
            // as it was (a real change would need a supplier quote)
            for c in big.components.iter_mut().filter(|c| c.refdes == "BT1") {
                c.name = "LiPo cell 250 mAh".into();
                c.example_part = format!("3.7 V 250 mAh LiPo, ~{mass:.1} g");
                c.mass_g = mass;
            }
            ExampleVersion {
                name: "Explore: 250 mAh cell".into(),
                description: format!(
                    "What if the cell grows from 180 mAh to 250 mAh? Battery mass scaled at the same energy density ({mass:.2} g, design parameter and BOM line BT1); unit cost left unchanged until a supplier quotes it. Compare with v0.4 to see what every lab says about battery life, mass and skin temperature before deciding whether to merge it."
                ),
                branch: "explore/battery-250".into(),
                parent: Some(1),
                baseline: None,
                project: big,
            }
        },
        ExampleVersion {
            name: "Iris v0.5 — strength, firmware and factory checks".into(),
            description: format!(
                "Ten requirements added from the new Structures, Prototype & Firmware, Build and Optics Pro analyses. The Structures Lab found the printed optics housing alone gives a ±{f4:.2} D focus error against ±0.25 D, so a threaded focus cell set at assembly was added ({f5:.2} D); the Build Guide found the 0.12 mm stencil's area ratio at {s4:.3}, right on the IPC-7525 limit, so the stencil became 0.10 mm ({s5:.2}). One goal stays open: lateral colour of the moulded achromat (R-030)."
            ),
            branch: "main".into(),
            parent: Some(1),
            baseline: Some("Product baseline (candidate)".into()),
            project: v5,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_projects_have_no_changes() {
        let p = Project::iris();
        let d = diff(&p, &p);
        assert!(d.identical);
        assert!(d.params.is_empty() && d.metrics.is_empty() && d.circuit_checks.is_empty());
        assert_eq!(d.counts.verdicts_changed, 0);
        assert_eq!(d.a.hash, d.b.hash);
    }

    #[test]
    fn iris_v03_to_v04_tells_the_story() {
        let h = iris_history();
        let d = diff(&h[0].project, &h[1].project);
        assert!(!d.identical);
        assert_ne!(d.a.hash, d.b.hash);
        // design parameter
        let p = d
            .params
            .iter()
            .find(|c| c.path == "design.optics.lens_to_combiner_mm")
            .expect("optics change");
        assert_eq!(p.old.as_ref().unwrap().as_f64(), Some(8.0));
        assert_eq!(p.new.as_ref().unwrap().as_f64(), Some(11.0));
        assert!((p.delta.unwrap() - 3.0).abs() < 1e-12);
        // BOM lines
        let bom = &d.sections.iter().find(|s| s.id == "components").unwrap().items;
        assert!(bom.iter().any(|i| i.key == "R1" && i.kind == "changed"));
        assert!(bom.iter().any(|i| i.key == "R2" && i.kind == "changed"));
        // requirement added and verified
        let reqs = &d.sections.iter().find(|s| s.id == "requirements").unwrap().items;
        assert!(reqs.iter().any(|i| i.key == "R-025" && i.kind == "added"));
        let r25 = d.requirements_status.iter().find(|r| r.id == "R-025").unwrap();
        assert_eq!(r25.change, "added");
        assert_eq!(r25.status_b.as_deref(), Some("pass"));
        // divider current changed value but still passes
        let r14 = d.requirements_status.iter().find(|r| r.id == "R-014").unwrap();
        assert_eq!(r14.change, "value");
        assert!((r14.value_a.unwrap() - 4.2 / 2.2).abs() < 1e-3);
        assert!((r14.value_b.unwrap() - 4.2 / 2.5).abs() < 1e-3);
        // the optics metric crossed zero
        let m = d.metrics.iter().find(|m| m.key == "optics.combiner_clearance_mm").unwrap();
        assert!(m.old.unwrap() < 0.0 && m.new.unwrap() > 0.0);
        assert!((m.delta.unwrap() - 3.0).abs() < 1e-9);
        // Circuit Lab: ADC pin over-voltage fixed (hand calc 4.2·1.2/2.2 = 2.29 V → 4.2·1/2.5 = 1.68 V)
        let adc = d.circuit_checks.iter().find(|c| c.key == "adc_peak_v").unwrap();
        assert_eq!(adc.pass_old, Some(false));
        assert_eq!(adc.pass_new, Some(true));
        assert!((adc.old.unwrap() - 4.2 * 1.2 / 2.2).abs() < 0.01);
        assert!((adc.new.unwrap() - 4.2 / 2.5).abs() < 0.01);
        assert!(d.headline.iter().any(|h| h.contains("R-025")));
        assert!(d.headline.iter().any(|h| h.contains("crossed zero")));
        assert!(
            h[0].description.contains("2.2 mm") && h[0].description.contains("2.29 V"),
            "{}",
            h[0].description
        );
    }

    #[test]
    fn exploratory_branch_changes_battery_life_in_proportion() {
        let h = iris_history();
        assert_eq!(h[2].branch, "explore/battery-250");
        assert_eq!(h[2].parent, Some(1));
        let d = diff(&h[1].project, &h[2].project);
        let life = d
            .metrics
            .iter()
            .find(|m| m.key == "power.battery_life_h")
            .expect("battery life moved");
        // Capacity ×250/180 with the same load: life scales by about the same ratio
        // (the discharge model's internal-resistance loss keeps it slightly off).
        let ratio = life.new.unwrap() / life.old.unwrap();
        assert!((ratio - 250.0 / 180.0).abs() < 0.08, "ratio {ratio}");
        assert!(d.params.iter().any(|p| p.path == "design.battery.mass_g"));
        // the heavier cell shows up in the mass budget: +1.56 g
        let m = d.metrics.iter().find(|m| m.key == "mass.total_g").expect("total mass moved");
        assert!((m.delta.unwrap() - 1.56).abs() < 1e-6, "{:?}", m.delta);
        assert!(d.sections[1].items.iter().any(|i| i.key == "BT1" && i.kind == "changed"));
        // R-001 keeps passing but most of its mass margin is gone — the review says so
        assert!(d.headline.iter().any(|h| h.starts_with("R-001: margin shrank")), "{:?}", d.headline);
    }

    #[test]
    fn v05_fixes_focus_and_stencil() {
        let h = iris_history();
        assert_eq!(h[3].parent, Some(1));
        let d = diff(&h[1].project, &h[3].project);
        let focus = d
            .metrics
            .iter()
            .find(|m| m.key == "structures.focus_error_worst_d")
            .expect("focus moved");
        assert!(focus.old.unwrap() > 0.25 && focus.new.unwrap() <= 0.25, "{focus:?}");
        assert!(h[3].description.contains("0.10 mm"));
    }

    #[test]
    fn summary_hash_is_stable_and_sensitive() {
        let p = Project::iris();
        let a = summary(&p, None);
        assert_eq!(a.hash, summary(&p, None).hash);
        let mut q = p.clone();
        q.design.frame.bridge_mm += 0.5;
        assert_ne!(a.hash, summary(&q, None).hash);
        assert_eq!(a.short.len(), 7);
        assert_eq!(a.requirements, p.requirements.len());
    }

    #[test]
    fn flatten_keys_arrays_by_id() {
        let a = serde_json::json!({ "c": [{ "id": "x", "v": 1 }, { "id": "y", "v": 2 }], "q": [1, 2] });
        let b = serde_json::json!({ "c": [{ "id": "y", "v": 3 }, { "id": "x", "v": 1 }], "q": [1, 2] });
        let ch = changes(&a, &b, "");
        assert_eq!(ch.len(), 1);
        assert_eq!(ch[0].path, "c[y].v");
    }
}
