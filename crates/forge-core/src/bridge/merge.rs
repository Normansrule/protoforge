//! Three-way merge of two project versions, Git-style, with every decision
//! visible so the user can *merge by choosing*.
//!
//! Given the common ancestor `base` and the two tips `ours` (the branch you
//! are on) and `theirs` (the branch being merged in), the merge walks the
//! three JSON trees together:
//!
//! | base vs ours | base vs theirs | result |
//! |---|---|---|
//! | same | same | unchanged |
//! | same | changed | take **theirs** (only they changed it) |
//! | changed | same | keep **ours** (only we changed it) |
//! | changed | changed, equal | take it (both made the same edit) |
//! | changed | changed, different | **conflict** — the user chooses |
//!
//! Objects are merged key by key and arrays of records with an `id` (or
//! `refdes`) are merged record by record, so two branches that each add a
//! different requirement both keep theirs. Every place where the tips differ
//! becomes a [`Decision`] with a default side; `choices` overrides any of
//! them by path. This is the classic three-way merge used by version control
//! systems (see S. Chacon and B. Straub, *Pro Git*, 2nd ed., 2014, ch. 3
//! "Git Branching — Basic Merging"); the result is then re-verified by every
//! lab like any other version.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::diff::{summary, Summary};
use crate::project::Project;

/// Which side a decision takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Ours,
    Theirs,
}

/// One place where the two branch tips differ.
#[derive(Clone, Debug, Serialize)]
pub struct Decision {
    /// Path in the diff notation, e.g. `design.optics.lens_to_combiner_mm`
    /// or `components[R1].name`.
    pub path: String,
    pub base: Option<Value>,
    pub ours: Option<Value>,
    pub theirs: Option<Value>,
    /// "ours" (only we changed it), "theirs" (only they changed it) or
    /// "conflict" (both changed it differently).
    pub kind: String,
    /// The side the three-way rule picks (ours for conflicts).
    pub default: Side,
    /// The side actually used (default unless overridden by `choices`).
    pub chosen: Side,
}

#[derive(Clone, Debug, Serialize)]
pub struct MergeResult {
    pub project: Option<Project>,
    /// Set when the merged JSON is not a valid project.
    pub error: Option<String>,
    pub decisions: Vec<Decision>,
    pub conflicts: usize,
    pub from_theirs: usize,
    pub summary: Option<Summary>,
}

fn id_of(v: &Value) -> Option<&str> {
    v.get("id").or_else(|| v.get("refdes")).and_then(|k| k.as_str())
}

fn is_id_array(v: Option<&Value>) -> bool {
    match v {
        Some(Value::Array(a)) => a.iter().all(|x| id_of(x).is_some()),
        None => true,
        _ => false,
    }
}

fn same(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (Some(Value::Number(p)), Some(Value::Number(q))) => p.as_f64() == q.as_f64(),
        _ => a == b,
    }
}

struct Ctx<'a> {
    choices: &'a BTreeMap<String, Side>,
    decisions: Vec<Decision>,
}

impl Ctx<'_> {
    /// Merge three optional values at `path`; `None` = absent.
    fn merge(&mut self, path: &str, base: Option<&Value>, ours: Option<&Value>, theirs: Option<&Value>) -> Option<Value> {
        if same(ours, theirs) {
            return ours.cloned();
        }
        // Both present and structured the same way: recurse.
        match (ours, theirs) {
            (Some(Value::Object(o)), Some(Value::Object(t))) => {
                let b = base.and_then(|b| b.as_object());
                let mut out = Map::new();
                let mut keys: Vec<&String> = o.keys().collect();
                for k in t.keys() {
                    if !o.contains_key(k) {
                        keys.push(k);
                    }
                }
                for k in keys {
                    let p = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                    if let Some(v) = self.merge(&p, b.and_then(|b| b.get(k)), o.get(k), t.get(k)) {
                        out.insert(k.clone(), v);
                    }
                }
                return Some(Value::Object(out));
            }
            (Some(Value::Array(o)), Some(Value::Array(t)))
                if is_id_array(ours) && is_id_array(theirs) && is_id_array(base) && !(o.is_empty() && t.is_empty()) =>
            {
                let empty = Vec::new();
                let b = base.and_then(|b| b.as_array()).unwrap_or(&empty);
                let find = |a: &'_ [Value], id: &str| a.iter().find(|x| id_of(x) == Some(id)).cloned();
                // order: ours, then items only theirs has
                let mut ids: Vec<String> = o.iter().filter_map(|x| id_of(x).map(String::from)).collect();
                for x in t {
                    if let Some(id) = id_of(x) {
                        if !ids.iter().any(|i| i == id) {
                            ids.push(id.to_string());
                        }
                    }
                }
                let mut out = Vec::new();
                for id in ids {
                    let (bx, ox, tx) = (find(b, &id), find(o, &id), find(t, &id));
                    if let Some(v) = self.merge(&format!("{path}[{id}]"), bx.as_ref(), ox.as_ref(), tx.as_ref()) {
                        out.push(v);
                    }
                }
                return Some(Value::Array(out));
            }
            _ => {}
        }
        // A leaf (or a whole added/removed record) that differs.
        let (kind, default) = if same(base, ours) {
            ("theirs", Side::Theirs)
        } else if same(base, theirs) {
            ("ours", Side::Ours)
        } else {
            ("conflict", Side::Ours)
        };
        let chosen = self.choices.get(path).copied().unwrap_or(default);
        self.decisions.push(Decision {
            path: path.to_string(),
            base: base.cloned(),
            ours: ours.cloned(),
            theirs: theirs.cloned(),
            kind: kind.into(),
            default,
            chosen,
        });
        match chosen {
            Side::Ours => ours.cloned(),
            Side::Theirs => theirs.cloned(),
        }
    }
}

/// Three-way merge of `ours` and `theirs` against their common ancestor.
pub fn merge3(base: &Project, ours: &Project, theirs: &Project, choices: &BTreeMap<String, Side>) -> MergeResult {
    let jb = serde_json::to_value(base).unwrap_or(Value::Null);
    let jo = serde_json::to_value(ours).unwrap_or(Value::Null);
    let jt = serde_json::to_value(theirs).unwrap_or(Value::Null);
    let mut ctx = Ctx {
        choices,
        decisions: Vec::new(),
    };
    let merged = ctx.merge("", Some(&jb), Some(&jo), Some(&jt)).unwrap_or(Value::Null);
    let decisions = ctx.decisions;
    let conflicts = decisions.iter().filter(|d| d.kind == "conflict").count();
    let from_theirs = decisions.iter().filter(|d| d.chosen == Side::Theirs).count();
    match serde_json::from_value::<Project>(merged) {
        Ok(p) => MergeResult {
            summary: Some(summary(&p, None)),
            project: Some(p),
            error: None,
            decisions,
            conflicts,
            from_theirs,
        },
        Err(e) => MergeResult {
            project: None,
            error: Some(format!("the merged file is not a valid project: {e}")),
            decisions,
            conflicts,
            from_theirs,
            summary: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(v: &[(&str, Side)]) -> BTreeMap<String, Side> {
        v.iter().map(|(k, s)| (k.to_string(), *s)).collect()
    }

    #[test]
    fn non_overlapping_edits_merge_cleanly() {
        let base = Project::iris();
        let mut ours = base.clone();
        ours.design.frame.bridge_mm = 20.0;
        let mut theirs = base.clone();
        theirs.design.battery.capacity_mah = 220.0;
        let r = merge3(&base, &ours, &theirs, &BTreeMap::new());
        assert_eq!(r.conflicts, 0);
        let p = r.project.unwrap();
        assert_eq!(p.design.frame.bridge_mm, 20.0);
        assert_eq!(p.design.battery.capacity_mah, 220.0);
        assert_eq!(r.decisions.len(), 2);
        assert_eq!(r.from_theirs, 1);
    }

    #[test]
    fn conflicts_default_to_ours_and_can_be_chosen() {
        let base = Project::iris();
        let mut ours = base.clone();
        ours.design.frame.bridge_mm = 20.0;
        let mut theirs = base.clone();
        theirs.design.frame.bridge_mm = 16.0;
        let r = merge3(&base, &ours, &theirs, &BTreeMap::new());
        assert_eq!(r.conflicts, 1);
        assert_eq!(r.decisions[0].path, "design.frame.bridge_mm");
        assert_eq!(r.project.unwrap().design.frame.bridge_mm, 20.0);
        let r = merge3(&base, &ours, &theirs, &ch(&[("design.frame.bridge_mm", Side::Theirs)]));
        assert_eq!(r.project.unwrap().design.frame.bridge_mm, 16.0);
        assert_eq!(r.decisions[0].chosen, Side::Theirs);
    }

    #[test]
    fn records_added_on_both_sides_are_kept() {
        let base = Project::iris();
        let mut ours = base.clone();
        let mut theirs = base.clone();
        let mut r1 = base.requirements[0].clone();
        r1.id = "R-900".into();
        ours.requirements.push(r1);
        let mut r2 = base.requirements[1].clone();
        r2.id = "R-901".into();
        theirs.requirements.push(r2);
        theirs.requirements.retain(|r| r.id != "R-003");
        let r = merge3(&base, &ours, &theirs, &BTreeMap::new());
        let p = r.project.unwrap();
        let ids: Vec<&str> = p.requirements.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"R-900") && ids.contains(&"R-901"));
        assert!(!ids.contains(&"R-003"), "their deletion is taken");
        assert_eq!(r.conflicts, 0);
        // keep R-003 by choosing ours at the record path
        let r = merge3(&base, &ours, &theirs, &ch(&[("requirements[R-003]", Side::Ours)]));
        assert!(r.project.unwrap().requirements.iter().any(|q| q.id == "R-003"));
    }

    #[test]
    fn identical_tips_have_no_decisions() {
        let base = Project::iris();
        let mut tip = base.clone();
        tip.design.optics.lens_to_combiner_mm = 12.0;
        let r = merge3(&base, &tip, &tip, &BTreeMap::new());
        assert!(r.decisions.is_empty());
        assert_eq!(r.summary.unwrap().hash, summary(&tip, None).hash);
    }
}
