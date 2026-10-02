//! Critical path method (CPM) on an activity-on-node network.
//!
//! **Forward pass** (earliest times): `ES_j = max_i (EF_i + lag_ij)` over the
//! predecessors i of j (0 when j has none), `EF_j = ES_j + d_j`. The project
//! finishes at `T = max EF`. **Backward pass** (latest times):
//! `LF_i = min_j (LS_j − lag_ij)` over the successors j of i (T when i has
//! none), `LS_i = LF_i − d_i`. **Total float** `TF = LS − ES` is how far an
//! activity can slip without delaying the project; **free float**
//! `FF_i = min_j (ES_j − lag_ij) − EF_i` is how far it can slip without
//! delaying any successor. Activities with zero total float form the
//! **critical path** — the longest path through the network.
//!
//! References: J. E. Kelley Jr. & M. R. Walker, "Critical-path planning and
//! scheduling", *Proc. Eastern Joint Computer Conference* (1959); Project
//! Management Institute, *A Guide to the Project Management Body of
//! Knowledge (PMBOK Guide)*, 7th ed. (2021).

use serde::Serialize;

use super::wbs::{parse_pred, task_budget, Config, Task};

/// Index form of a task network, topologically ordered.
#[derive(Clone, Debug)]
pub struct Net {
    pub n: usize,
    pub preds: Vec<Vec<(usize, f64)>>,
    pub succs: Vec<Vec<(usize, f64)>>,
    /// Topological order (predecessors first).
    pub order: Vec<usize>,
    /// Topological level (longest chain of predecessors), for drawing.
    pub level: Vec<usize>,
}

/// Build the network; errors on unknown predecessors, duplicate ids and cycles.
pub fn build_net(tasks: &[Task]) -> Result<Net, String> {
    let n = tasks.len();
    for (i, t) in tasks.iter().enumerate() {
        if t.id.trim().is_empty() {
            return Err(format!("task #{} has no id", i + 1));
        }
        if tasks[..i].iter().any(|u| u.id == t.id) {
            return Err(format!("duplicate task id `{}`", t.id));
        }
    }
    let idx = |s: &str| tasks.iter().position(|t| t.id == s);
    let known = |s: &str| idx(s).is_some();
    let mut preds = vec![Vec::new(); n];
    let mut succs = vec![Vec::new(); n];
    for (j, t) in tasks.iter().enumerate() {
        for p in &t.preds {
            let Some((id, lag)) = parse_pred(p, &known) else { continue };
            let i = idx(&id).ok_or_else(|| format!("task `{}` depends on unknown task `{id}`", t.id))?;
            if i == j {
                return Err(format!("task `{}` depends on itself", t.id));
            }
            if !lag.is_finite() {
                return Err(format!("task `{}`: bad lag in `{p}`", t.id));
            }
            preds[j].push((i, lag));
            succs[i].push((j, lag));
        }
    }
    // Kahn's algorithm, stable (keeps the listed order among ready tasks)
    let mut indeg: Vec<usize> = preds.iter().map(|v| v.len()).collect();
    let mut order = Vec::with_capacity(n);
    let mut done = vec![false; n];
    while order.len() < n {
        let Some(k) = (0..n).find(|&k| !done[k] && indeg[k] == 0) else {
            let cyc: Vec<&str> = (0..n).filter(|&k| !done[k]).map(|k| tasks[k].id.as_str()).collect();
            return Err(format!("dependency cycle among: {}", cyc.join(", ")));
        };
        done[k] = true;
        order.push(k);
        for &(s, _) in &succs[k] {
            indeg[s] -= 1;
        }
    }
    let mut level = vec![0usize; n];
    for &k in &order {
        level[k] = preds[k].iter().map(|&(i, _)| level[i] + 1).max().unwrap_or(0);
    }
    Ok(Net {
        n,
        preds,
        succs,
        order,
        level,
    })
}

/// Result of one forward + backward pass.
#[derive(Clone, Debug, Default)]
pub struct Pass {
    pub es: Vec<f64>,
    pub ef: Vec<f64>,
    pub ls: Vec<f64>,
    pub lf: Vec<f64>,
    pub finish: f64,
}

impl Pass {
    pub fn total_float(&self, i: usize) -> f64 {
        self.ls[i] - self.es[i]
    }
}

/// Floats below this (weeks) count as zero.
pub const EPS: f64 = 1e-6;

/// Forward pass only (earliest start/finish, project finish). Fast path for Monte Carlo.
pub fn forward(net: &Net, d: &[f64], es: &mut [f64], ef: &mut [f64]) -> f64 {
    let mut finish: f64 = 0.0;
    for &k in &net.order {
        let s = net.preds[k].iter().map(|&(i, lag)| ef[i] + lag).fold(0.0f64, f64::max);
        es[k] = s;
        ef[k] = s + d[k];
        finish = finish.max(ef[k]);
    }
    finish
}

/// Backward pass (latest start/finish) against a project finish `finish`.
pub fn backward(net: &Net, d: &[f64], finish: f64, ls: &mut [f64], lf: &mut [f64]) {
    for &k in net.order.iter().rev() {
        let f = net.succs[k].iter().map(|&(j, lag)| ls[j] - lag).fold(finish, f64::min);
        lf[k] = f;
        ls[k] = f - d[k];
    }
}

/// Full CPM pass.
pub fn cpm(net: &Net, d: &[f64]) -> Pass {
    let n = net.n;
    let mut p = Pass {
        es: vec![0.0; n],
        ef: vec![0.0; n],
        ls: vec![0.0; n],
        lf: vec![0.0; n],
        finish: 0.0,
    };
    p.finish = forward(net, d, &mut p.es, &mut p.ef);
    backward(net, d, p.finish, &mut p.ls, &mut p.lf);
    p
}

/// Free float of task i.
pub fn free_float(net: &Net, p: &Pass, i: usize) -> f64 {
    net.succs[i].iter().map(|&(j, lag)| p.es[j] - lag).fold(p.finish, f64::min) - p.ef[i]
}

/// The driving chain that sets the finish: walk back from the latest-finishing
/// task through the predecessor whose EF + lag equals the ES.
pub fn driving_path(net: &Net, p: &Pass) -> Vec<usize> {
    if net.n == 0 {
        return vec![];
    }
    // Start from a task that finishes the project; when several tie, prefer one
    // with no successors (a lead can let a successor finish together with its
    // predecessor, and the path must then run through to that last task).
    let finish = (0..net.n).map(|i| p.ef[i]).fold(f64::NEG_INFINITY, f64::max);
    let mut k = (0..net.n)
        .filter(|&i| (p.ef[i] - finish).abs() < EPS)
        .min_by_key(|&i| (!net.succs[i].is_empty(), i))
        .unwrap();
    let mut path = vec![k];
    loop {
        let next = net.preds[k]
            .iter()
            .filter(|&&(i, lag)| (p.ef[i] + lag - p.es[k]).abs() < EPS)
            .map(|&(i, _)| i)
            .max_by(|&a, &b| {
                // prefer the longer (non-milestone) driver when several tie
                (p.ef[a] - p.es[a]).partial_cmp(&(p.ef[b] - p.es[b])).unwrap()
            });
        match next {
            Some(i) => {
                path.push(i);
                k = i;
            }
            None => break,
        }
    }
    path.reverse();
    path
}

// ---------------------------------------------------------------------------
// Calendar dates (no external crates): days from civil, Howard Hinnant's algorithm.
// ---------------------------------------------------------------------------

pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Parse `YYYY-MM-DD` → day number (falls back to 2026-10-05).
pub fn parse_date(s: &str) -> i64 {
    let v: Vec<i64> = s.trim().split('-').filter_map(|x| x.parse().ok()).collect();
    match v.as_slice() {
        [y, m, d] if (1..=12).contains(m) && (1..=31).contains(d) => days_from_civil(*y, *m, *d),
        _ => days_from_civil(2026, 10, 5),
    }
}

pub fn fmt_date(day: i64) -> String {
    let (y, m, d) = civil_from_days(day);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Date `weeks` after the start (calendar weeks, rounded to whole days).
pub fn date_after(start: i64, weeks: f64) -> String {
    fmt_date(start + (weeks * 7.0).round() as i64)
}

// ---------------------------------------------------------------------------
// Schedule output
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct TaskRow {
    pub id: String,
    pub wbs: String,
    pub name: String,
    pub phase: String,
    pub owner: String,
    pub milestone: bool,
    pub optimistic: f64,
    pub likely: f64,
    pub pessimistic: f64,
    pub duration: f64,
    pub es: f64,
    pub ef: f64,
    pub ls: f64,
    pub lf: f64,
    pub total_float: f64,
    pub free_float: f64,
    pub critical: bool,
    pub level: usize,
    pub preds: Vec<(String, f64)>,
    pub succs: Vec<String>,
    pub budget_usd: f64,
    pub expense_usd: f64,
    pub fte: f64,
    pub start_date: String,
    pub finish_date: String,
    pub source: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct StageCheck {
    pub stage: String,
    pub from: String,
    pub to: String,
    pub weeks: f64,
    pub typical: [f64; 2],
    pub verdict: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Schedule {
    pub tasks: Vec<TaskRow>,
    pub finish_weeks: f64,
    pub finish_date: String,
    pub start_date: String,
    /// The driving (critical) chain, in order.
    pub critical_path: Vec<String>,
    pub critical_count: usize,
    pub bac_usd: f64,
    pub labour_usd: f64,
    pub expense_usd: f64,
    pub levels: usize,
    pub stage_checks: Vec<StageCheck>,
    pub owners: Vec<OwnerLoad>,
    pub csv: String,
    pub mermaid: String,
    pub notes: Vec<String>,
}

/// Weekly full-time-equivalent load per owner (early-start schedule).
#[derive(Clone, Debug, Serialize)]
pub struct OwnerLoad {
    pub owner: String,
    pub weeks: Vec<f64>,
    pub peak: f64,
}

pub fn durations(tasks: &[Task]) -> Vec<f64> {
    tasks.iter().map(|t| t.three_point().1).collect()
}

/// CPM schedule with floats, budgets, exports and sanity checks.
pub fn schedule(p: Option<&crate::project::Project>, tasks: &[Task], cfg: &Config) -> Result<Schedule, String> {
    let net = build_net(tasks)?;
    let d = durations(tasks);
    let pass = cpm(&net, &d);
    let start = parse_date(&cfg.start_date);
    let mut rows = Vec::with_capacity(tasks.len());
    for (i, t) in tasks.iter().enumerate() {
        let tf = pass.total_float(i);
        rows.push(TaskRow {
            id: t.id.clone(),
            wbs: t.wbs.clone(),
            name: t.name.clone(),
            phase: t.phase.clone(),
            owner: t.owner.clone(),
            milestone: t.milestone,
            optimistic: t.three_point().0,
            likely: d[i],
            pessimistic: t.three_point().2,
            duration: d[i],
            es: pass.es[i],
            ef: pass.ef[i],
            ls: pass.ls[i],
            lf: pass.lf[i],
            total_float: clean(tf),
            free_float: clean(free_float(&net, &pass, i)),
            critical: tf.abs() < EPS,
            level: net.level[i],
            preds: net.preds[i].iter().map(|&(k, lag)| (tasks[k].id.clone(), lag)).collect(),
            succs: net.succs[i].iter().map(|&(k, _)| tasks[k].id.clone()).collect(),
            budget_usd: task_budget(t, cfg),
            expense_usd: t.expense_usd,
            fte: t.fte,
            start_date: date_after(start, pass.es[i]),
            finish_date: date_after(start, pass.ef[i]),
            source: t.source.clone(),
        });
    }
    let path: Vec<String> = driving_path(&net, &pass).into_iter().map(|i| tasks[i].id.clone()).collect();
    let bac: f64 = rows.iter().map(|r| r.budget_usd).sum();
    let expense: f64 = tasks.iter().map(|t| t.expense_usd.max(0.0)).sum();
    // owner load, weekly buckets
    let nweeks = pass.finish.ceil().max(1.0) as usize;
    let mut owners: Vec<OwnerLoad> = Vec::new();
    for (i, t) in tasks.iter().enumerate() {
        if t.milestone || d[i] <= 0.0 {
            continue;
        }
        let o = match owners.iter_mut().position(|o| o.owner == t.owner) {
            Some(k) => k,
            None => {
                owners.push(OwnerLoad {
                    owner: t.owner.clone(),
                    weeks: vec![0.0; nweeks],
                    peak: 0.0,
                });
                owners.len() - 1
            }
        };
        for (w, slot) in owners[o].weeks.iter_mut().enumerate() {
            let (a, b) = (w as f64, w as f64 + 1.0);
            let overlap = (b.min(pass.ef[i]) - a.max(pass.es[i])).max(0.0);
            *slot += overlap * t.fte;
        }
    }
    for o in &mut owners {
        o.peak = o.weeks.iter().cloned().fold(0.0, f64::max);
    }
    let mut notes = vec![
        "Durations are three-point estimates in weeks (optimistic / most likely / pessimistic) — assumptions to edit. CPM uses the most likely value; the Schedule risk tab samples all three.".to_string(),
        format!(
            "Budget at completion (BAC) = labour (FTE × weeks × 40 h × ${:.0}/h blended rate — assumption) + expenses (prototype builds priced by the Manufacturing Hub, indicative certification fees from the compliance plan).",
            cfg.eng_rate_usd_h
        ),
    ];
    let mut checks = Vec::new();
    if let Some(p) = p {
        checks = stage_checks(p, tasks, &pass);
        let fc = super::wbs::frame_choice(p);
        if !super::wbs::plans_mold(p, cfg) && !fc.molded {
            notes.push(format!(
                "The Build Guide roadmap plans a hardened steel mold at PVT, but the Manufacturing Hub costs the frame by {} at the {:.0}-unit target (molding pays off above {} units, tooling ≈ ${:.0}k). The default plan follows the cost model; set frame tooling to “mold” to plan the hard tool.",
                fc.process,
                p.design.manufacturing.target_quantity,
                fc.crossover.map(|q| format!("{q:.0}")).unwrap_or_else(|| "—".into()),
                fc.tooling_usd / 1000.0
            ));
        }
    }
    let csv = to_csv(&rows);
    let title = p.map(|p| p.meta.name.clone()).unwrap_or_else(|| "Program".into());
    let mermaid = to_mermaid(&title, &rows, start);
    Ok(Schedule {
        finish_weeks: pass.finish,
        finish_date: date_after(start, pass.finish),
        start_date: fmt_date(start),
        critical_count: rows.iter().filter(|r| r.critical).count(),
        critical_path: path,
        bac_usd: bac,
        labour_usd: bac - expense,
        expense_usd: expense,
        levels: net.level.iter().cloned().max().unwrap_or(0) + 1,
        tasks: rows,
        stage_checks: checks,
        owners,
        csv,
        mermaid,
        notes,
    })
}

fn clean(x: f64) -> f64 {
    if x.abs() < EPS {
        0.0
    } else {
        (x * 1e6).round() / 1e6
    }
}

/// Stage spans of the plan vs the Build Guide roadmap's typical weeks.
fn stage_checks(p: &crate::project::Project, tasks: &[Task], pass: &Pass) -> Vec<StageCheck> {
    let road = crate::build::roadmap::roadmap(p);
    let ms_ef = |stage: &str| -> Option<(String, f64)> {
        tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                t.milestone
                    && p.milestones
                        .iter()
                        .any(|m| m.id == t.id && crate::build::roadmap::stage_of(&m.stage, &m.title) == stage)
            })
            .map(|(i, t)| (t.id.clone(), pass.ef[i]))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
    };
    let mut out = Vec::new();
    let mut prev: (String, f64) = ("start".into(), 0.0);
    for s in road.iter().filter(|s| s.id != "mp") {
        if let Some(end) = ms_ef(s.id) {
            let w = end.1 - prev.1;
            let [lo, hi] = s.typical_weeks;
            let verdict = if w < lo {
                "shorter than typical — check the estimates".to_string()
            } else if w > hi {
                "longer than typical — look at the critical tasks in this stage".to_string()
            } else {
                "within the typical range".to_string()
            };
            out.push(StageCheck {
                stage: s.title.to_string(),
                from: prev.0.clone(),
                to: end.0.clone(),
                weeks: w,
                typical: s.typical_weeks,
                verdict,
            });
            prev = end;
        }
    }
    out
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn to_csv(rows: &[TaskRow]) -> String {
    let mut s = String::from("id,wbs,name,phase,owner,milestone,optimistic_wk,likely_wk,pessimistic_wk,es_wk,ef_wk,ls_wk,lf_wk,total_float_wk,free_float_wk,critical,predecessors,start_date,finish_date,budget_usd\n");
    for r in rows {
        let preds: Vec<String> = r
            .preds
            .iter()
            .map(|(id, lag)| if lag.abs() < EPS { id.clone() } else { format!("{id}{lag:+}") })
            .collect();
        s += &format!(
            "{},{},{},{},{},{},{},{},{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{},{},{},{},{:.0}\n",
            csv_field(&r.id),
            csv_field(&r.wbs),
            csv_field(&r.name),
            csv_field(&r.phase),
            csv_field(&r.owner),
            r.milestone,
            r.optimistic,
            r.likely,
            r.pessimistic,
            r.es,
            r.ef,
            r.ls,
            r.lf,
            r.total_float,
            r.free_float,
            r.critical,
            csv_field(&preds.join(" ")),
            r.start_date,
            r.finish_date,
            r.budget_usd
        );
    }
    s
}

fn mermaid_id(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

fn mermaid_text(s: &str) -> String {
    s.replace([':', ';', '#'], " ").replace("  ", " ")
}

/// Mermaid `gantt` block with explicit CPM dates (critical tasks tagged `crit`).
pub fn to_mermaid(title: &str, rows: &[TaskRow], start: i64) -> String {
    let mut s = format!(
        "gantt\n    title {} — CPM schedule\n    dateFormat YYYY-MM-DD\n    axisFormat %b %Y\n",
        mermaid_text(title)
    );
    let mut phases: Vec<&str> = Vec::new();
    for r in rows {
        if !phases.contains(&r.phase.as_str()) {
            phases.push(&r.phase);
        }
    }
    for ph in phases {
        s += &format!("    section {}\n", mermaid_text(ph));
        for r in rows.iter().filter(|r| r.phase == ph) {
            let mut tags: Vec<&str> = Vec::new();
            if r.milestone {
                tags.push("milestone");
            }
            if r.critical {
                tags.push("crit");
            }
            let days = (r.duration * 7.0).round() as i64;
            s += &format!(
                "    {} :{}{}, {}, {}d\n",
                mermaid_text(&r.name),
                if tags.is_empty() {
                    String::new()
                } else {
                    format!("{}, ", tags.join(", "))
                },
                mermaid_id(&r.id),
                date_after(start, r.es),
                days
            );
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, d: f64, preds: &[&str]) -> Task {
        Task {
            id: id.into(),
            name: id.into(),
            optimistic: d,
            likely: d,
            pessimistic: d,
            preds: preds.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    /// Hand-calculated activity-on-node network:
    /// A(3), B(4), C(2)←A, D(5)←A, E(1)←B,C, F(2)←D,E.
    /// Forward: A 0–3, B 0–4, C 3–5, D 3–8, E 5–6, F 8–10 → T = 10.
    /// Backward: F 8–10, D 3–8, E 7–8, C 5–7, B 3–7, A 0–3.
    /// TF: A 0, B 3, C 2, D 0, E 2, F 0; FF: A 0, B 1, C 0, D 0, E 2, F 0.
    #[test]
    fn cpm_textbook_network() {
        let ts = vec![
            task("A", 3.0, &[]),
            task("B", 4.0, &[]),
            task("C", 2.0, &["A"]),
            task("D", 5.0, &["A"]),
            task("E", 1.0, &["B", "C"]),
            task("F", 2.0, &["D", "E"]),
        ];
        let s = schedule(None, &ts, &Config::default()).unwrap();
        assert!((s.finish_weeks - 10.0).abs() < 1e-9);
        let get = |id: &str| s.tasks.iter().find(|r| r.id == id).unwrap();
        let expect = [
            ("A", 0.0, 3.0, 0.0, 3.0, 0.0, 0.0),
            ("B", 0.0, 4.0, 3.0, 7.0, 3.0, 1.0),
            ("C", 3.0, 5.0, 5.0, 7.0, 2.0, 0.0),
            ("D", 3.0, 8.0, 3.0, 8.0, 0.0, 0.0),
            ("E", 5.0, 6.0, 7.0, 8.0, 2.0, 2.0),
            ("F", 8.0, 10.0, 8.0, 10.0, 0.0, 0.0),
        ];
        for (id, es, ef, ls, lf, tf, ff) in expect {
            let r = get(id);
            assert_eq!(
                (r.es, r.ef, r.ls, r.lf, r.total_float, r.free_float),
                (es, ef, ls, lf, tf, ff),
                "{id}"
            );
        }
        assert_eq!(s.critical_path, vec!["A", "D", "F"]);
        assert_eq!(s.critical_count, 3);
        assert!(s.mermaid.contains("crit, A, 2026-10-05, 21d"), "{}", s.mermaid);
    }

    #[test]
    fn critical_path_runs_through_a_task_that_ties_the_finish() {
        // D finishes at 10; E starts 1 week before D ends (lead) and also ends at 10.
        let t = |id: &str, d: f64, preds: &[&str]| Task {
            id: id.into(),
            optimistic: d,
            likely: d,
            pessimistic: d,
            preds: preds.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        let tasks = vec![
            t("A", 3.0, &[]),
            t("B", 2.0, &["A"]),
            t("C", 4.0, &["A"]),
            t("D", 2.0, &["B+3", "C"]),
            t("E", 1.0, &["D-1"]),
        ];
        let s = schedule(None, &tasks, &Config::default()).unwrap();
        assert!((s.finish_weeks - 10.0).abs() < 1e-9);
        assert_eq!(s.critical_path, vec!["A", "B", "D", "E"]);
    }

    #[test]
    fn lags_and_cycles() {
        // B starts 2 weeks after A finishes; C leads by 1 week
        let ts = vec![task("A", 3.0, &[]), task("B", 1.0, &["A+2"]), task("C", 4.0, &["A-1"])];
        let s = schedule(None, &ts, &Config::default()).unwrap();
        let b = s.tasks.iter().find(|r| r.id == "B").unwrap();
        let c = s.tasks.iter().find(|r| r.id == "C").unwrap();
        assert_eq!((b.es, c.es, s.finish_weeks), (5.0, 2.0, 6.0));
        assert_eq!(b.total_float, 0.0);
        assert_eq!(c.total_float, 0.0);
        let cyc = vec![task("A", 1.0, &["B"]), task("B", 1.0, &["A"])];
        assert!(build_net(&cyc).unwrap_err().contains("cycle"));
        assert!(build_net(&[task("A", 1.0, &["Z"])]).unwrap_err().contains("unknown"));
    }

    #[test]
    fn calendar_round_trip() {
        let d = parse_date("2026-10-05");
        assert_eq!(fmt_date(d), "2026-10-05");
        assert_eq!(date_after(d, 1.0), "2026-10-12");
        assert_eq!(date_after(d, 13.0), "2027-01-04");
        assert_eq!(fmt_date(days_from_civil(2024, 2, 29) + 1), "2024-03-01");
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }
}
