//! Design-parameter sensitivity: sweep one number in `project.design` and
//! re-verify every requirement at each value.
//!
//! This is the "what if?" question a systems engineer asks before a design
//! review — *how far can the battery shrink before the battery-life
//! requirement fails, and what else moves with it?* Each step is a full
//! [`super::evaluate`] on a modified copy of the project, so the sweep is
//! bounded to [`MAX_STEPS`] values. Modules cache their last result, so
//! parameters that only feed cheap analyses sweep in milliseconds; the
//! baseline is evaluated last so those caches end up warm for the real
//! project again.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{evaluate, Evaluation, Status};
use crate::project::Project;

/// Upper bound on sweep points (each is a full evaluation).
pub const MAX_STEPS: usize = 15;

/// A numeric design parameter the sweep can vary.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Param {
    /// Dotted path under `design`, e.g. `battery.capacity_mah`.
    pub path: String,
    pub section: String,
    pub name: String,
    pub value: f64,
}

/// Every numeric leaf of `project.design` (one level of sections, plus
/// numeric `extra` entries), in file order.
pub fn parameters(p: &Project) -> Vec<Param> {
    let mut out = Vec::new();
    let Ok(Value::Object(design)) = serde_json::to_value(&p.design) else {
        return out;
    };
    for (section, v) in &design {
        if let Value::Object(fields) = v {
            for (k, x) in fields {
                if let Some(n) = x.as_f64() {
                    out.push(Param {
                        path: format!("{section}.{k}"),
                        section: section.clone(),
                        name: k.clone(),
                        value: n,
                    });
                }
            }
        }
    }
    out
}

/// Read a design parameter by dotted path.
pub fn get_param(p: &Project, path: &str) -> Option<f64> {
    let v = serde_json::to_value(&p.design).ok()?;
    v.pointer(&format!("/{}", path.replace('.', "/")))?.as_f64()
}

/// Copy of `p` with one design parameter replaced.
pub fn with_param(p: &Project, path: &str, value: f64) -> Result<Project, String> {
    let mut v = serde_json::to_value(p).map_err(|e| e.to_string())?;
    let ptr = format!("/design/{}", path.replace('.', "/"));
    let slot = v.pointer_mut(&ptr).ok_or_else(|| format!("unknown design parameter `{path}`"))?;
    if !slot.is_number() {
        return Err(format!("design parameter `{path}` is not a number"));
    }
    // keep integer fields integral so they still deserialise as u32
    *slot = if slot.is_u64() || slot.is_i64() {
        Value::from(value.round() as i64)
    } else {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or("value must be finite")?
    };
    serde_json::from_value(v).map_err(|e| format!("cannot apply {path} = {value}: {e}"))
}

#[derive(Clone, Debug, Deserialize)]
pub struct SensitivityIn {
    pub project: Project,
    pub param: String,
    /// Explicit values to try (overrides lo/hi/steps).
    #[serde(default)]
    pub values: Vec<f64>,
    #[serde(default)]
    pub lo: Option<f64>,
    #[serde(default)]
    pub hi: Option<f64>,
    #[serde(default)]
    pub steps: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensitivityRow {
    pub id: String,
    pub metric: Option<String>,
    pub values: Vec<Option<f64>>,
    pub statuses: Vec<Status>,
    pub margins: Vec<Option<f64>>,
    /// Status differs somewhere in the sweep.
    pub flips: bool,
    /// Metric value changes by more than 0.1 % across the sweep.
    pub moves: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sensitivity {
    pub param: String,
    pub baseline: f64,
    pub values: Vec<f64>,
    /// Index of the value closest to the baseline.
    pub baseline_index: usize,
    pub rows: Vec<SensitivityRow>,
    pub pass: Vec<usize>,
    pub fail: Vec<usize>,
    /// Requirement ids whose status flips somewhere in the sweep.
    pub flipped: Vec<String>,
}

/// Sweep values: explicit list, or `steps` points from lo..hi (default
/// ±50 % around the baseline), always including the baseline.
pub fn sweep_values(baseline: f64, i: &SensitivityIn) -> Vec<f64> {
    let mut v: Vec<f64> = if !i.values.is_empty() {
        i.values.iter().copied().filter(|x| x.is_finite()).collect()
    } else {
        let span = if baseline.abs() > 1e-12 { baseline.abs() * 0.5 } else { 1.0 };
        let lo = i.lo.unwrap_or(baseline - span);
        let hi = i.hi.unwrap_or(baseline + span);
        let n = i.steps.unwrap_or(9).clamp(2, MAX_STEPS);
        (0..n).map(|k| lo + (hi - lo) * k as f64 / (n - 1) as f64).collect()
    };
    if !v.iter().any(|x| (x - baseline).abs() <= 1e-9 * baseline.abs().max(1.0)) {
        v.push(baseline);
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v.dedup_by(|a, b| (*a - *b).abs() <= 1e-12 * a.abs().max(1.0));
    if v.len() > MAX_STEPS {
        // thin evenly, keeping both ends and the baseline
        let keep: Vec<f64> = (0..MAX_STEPS - 1).map(|k| v[k * (v.len() - 1) / (MAX_STEPS - 2)]).collect();
        let mut t = keep;
        if !t.iter().any(|x| (x - baseline).abs() <= 1e-9 * baseline.abs().max(1.0)) {
            t.push(baseline);
        }
        t.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        t.dedup();
        v = t;
    }
    v
}

pub fn sensitivity(i: &SensitivityIn) -> Result<Sensitivity, String> {
    let p = &i.project;
    let baseline = get_param(p, &i.param).ok_or_else(|| format!("unknown design parameter `{}`", i.param))?;
    let values = sweep_values(baseline, i);
    let baseline_index = values
        .iter()
        .enumerate()
        .min_by(|a, b| {
            (a.1 - baseline)
                .abs()
                .partial_cmp(&(b.1 - baseline).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|x| x.0)
        .unwrap_or(0);
    let mut evals: Vec<Option<Evaluation>> = vec![None; values.len()];
    for (k, v) in values.iter().enumerate() {
        if k == baseline_index {
            continue;
        }
        evals[k] = Some(evaluate(&with_param(p, &i.param, *v)?));
    }
    // baseline last: leaves every module's memo cache holding the real project
    evals[baseline_index] = Some(evaluate(p));
    let evals: Vec<Evaluation> = evals.into_iter().map(|e| e.expect("filled")).collect();
    let rows: Vec<SensitivityRow> = p
        .requirements
        .iter()
        .enumerate()
        .map(|(ri, r)| {
            let res: Vec<_> = evals.iter().map(|e| &e.results[ri]).collect();
            let statuses: Vec<Status> = res.iter().map(|x| x.status).collect();
            let vals: Vec<Option<f64>> = res.iter().map(|x| x.value).collect();
            let finite: Vec<f64> = vals.iter().flatten().copied().collect();
            let moves = match (
                finite.iter().cloned().fold(f64::NAN, f64::min),
                finite.iter().cloned().fold(f64::NAN, f64::max),
            ) {
                (lo, hi) if lo.is_finite() && hi.is_finite() => (hi - lo) > 1e-3 * lo.abs().max(hi.abs()).max(1e-9),
                _ => false,
            };
            SensitivityRow {
                id: r.id.clone(),
                metric: r.verification.metric.clone(),
                flips: statuses.windows(2).any(|w| w[0] != w[1]),
                moves,
                values: vals,
                margins: res.iter().map(|x| x.margin).collect(),
                statuses,
            }
        })
        .collect();
    Ok(Sensitivity {
        param: i.param.clone(),
        baseline,
        pass: evals.iter().map(|e| e.pass).collect(),
        fail: evals.iter().map(|e| e.fail).collect(),
        flipped: rows.iter().filter(|r| r.flips).map(|r| r.id.clone()).collect(),
        values,
        baseline_index,
        rows,
    })
}

/// Dispatcher adapter.
pub fn run(i: SensitivityIn) -> Result<Sensitivity, String> {
    sensitivity(&i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_and_sets_parameters() {
        let p = Project::iris();
        let ps = parameters(&p);
        assert!(ps.iter().any(|x| x.path == "battery.capacity_mah" && x.value == 180.0));
        assert!(ps.iter().any(|x| x.path == "optics.px_w"));
        let q = with_param(&p, "battery.capacity_mah", 120.0).unwrap();
        assert_eq!(q.design.battery.capacity_mah, 120.0);
        let q = with_param(&p, "optics.px_w", 800.4).unwrap();
        assert_eq!(q.design.optics.px_w, 800);
        assert!(with_param(&p, "battery.nope", 1.0).is_err());
        assert!(with_param(&p, "frame.material", 1.0).is_err());
    }

    #[test]
    fn sweep_values_include_baseline_and_are_bounded() {
        let p = Project::iris();
        let i = SensitivityIn {
            project: p,
            param: "battery.capacity_mah".into(),
            values: vec![],
            lo: Some(50.0),
            hi: Some(400.0),
            steps: Some(100),
        };
        let v = sweep_values(180.0, &i);
        assert!(v.len() <= MAX_STEPS + 1);
        assert!(v.contains(&180.0));
        assert_eq!(v[0], 50.0);
    }

    #[test]
    fn battery_sweep_flips_battery_life() {
        let mut p = Project::iris();
        // a requirement on the component roll-up so the test does not depend on the Power lab
        p.requirements.push(crate::project::Requirement {
            id: "R-X".into(),
            text: "Battery life shall be at least 8 h.".into(),
            derives_from: vec!["N-2".into()],
            allocated_to: vec!["SYS-PWR".into()],
            verification: crate::project::Verification {
                metric: Some("budget.battery_life_h".into()),
                cmp: crate::project::Cmp::Ge,
                target: 8.0,
                unit: "h".into(),
                ..Default::default()
            },
            ..Default::default()
        });
        let i = SensitivityIn {
            project: p.clone(),
            param: "battery.capacity_mah".into(),
            values: vec![60.0, 180.0, 300.0],
            lo: None,
            hi: None,
            steps: None,
        };
        let s = sensitivity(&i).unwrap();
        assert_eq!(s.values, vec![60.0, 180.0, 300.0]);
        assert_eq!(s.baseline_index, 1);
        let rx = s.rows.iter().find(|r| r.id == "R-X").unwrap();
        assert!(rx.flips && rx.moves);
        assert_eq!(rx.statuses, vec![Status::Fail, Status::Pass, Status::Pass]);
        assert!(s.flipped.contains(&"R-X".to_string()));
        // battery life ∝ capacity
        let v: Vec<f64> = rx.values.iter().map(|x| x.unwrap()).collect();
        assert!((v[2] / v[0] - 5.0).abs() < 1e-9);
        // requirements unrelated to the battery do not move
        let r18 = s.rows.iter().find(|r| r.id == "R-018").unwrap();
        assert!(!r18.moves);
    }
}
