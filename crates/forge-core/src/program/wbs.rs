//! Work breakdown structure (WBS): the task network the schedule, risk and
//! earned-value tabs run on, and the lab's per-project configuration read
//! from `design.extra.program`.
//!
//! The default network is **derived from the project**: the milestones
//! (`project.milestones`, M-0 … M-5 for Iris) become zero-duration milestone
//! tasks, the Build Guide roadmap (`crate::build::roadmap`) names the EVT /
//! DVT / PVT build tasks and their quantities, the compliance plan
//! (`crate::build::compliance`) contributes certification work packages
//! (grouped by stage and category, durations and fees from the plan), and
//! the Manufacturing Hub cost model (`crate::mfg`) prices every prototype
//! build (`n × unit cost at n`). Every duration is a three-point estimate
//! (optimistic, most likely, pessimistic) in weeks — an *assumption to edit*.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::project::Project;

/// One activity (or milestone) of the network.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Task {
    pub id: String,
    /// Work breakdown structure code, e.g. `4.2`.
    pub wbs: String,
    pub name: String,
    /// Concept, Requirements, Design, EVT, DVT, PVT …
    pub phase: String,
    /// Role responsible (Systems, Optics, Electrical, …).
    pub owner: String,
    /// Three-point estimate, weeks.
    pub optimistic: f64,
    pub likely: f64,
    pub pessimistic: f64,
    /// Finish-to-start predecessors: `"T05"`, `"T05+2"` (2-week lag) or `"T05-1"` (1-week lead).
    pub preds: Vec<String>,
    /// Zero-duration milestone.
    pub milestone: bool,
    /// People working on it full time (labour budget = fte × weeks × 40 h × rate).
    pub fte: f64,
    /// Non-labour cost: builds, test-lab fees, tooling (USD).
    pub expense_usd: f64,
    /// What the task was derived from (milestone id, roadmap stage, compliance items …).
    pub source: String,
}

impl Default for Task {
    fn default() -> Self {
        Task {
            id: String::new(),
            wbs: String::new(),
            name: String::new(),
            phase: String::new(),
            owner: String::new(),
            optimistic: 0.0,
            likely: 0.0,
            pessimistic: 0.0,
            preds: Vec::new(),
            milestone: false,
            fte: 1.0,
            expense_usd: 0.0,
            source: String::new(),
        }
    }
}

impl Task {
    /// Durations sanitised: non-negative and ordered a ≤ m ≤ b; milestones are 0.
    pub fn three_point(&self) -> (f64, f64, f64) {
        if self.milestone {
            return (0.0, 0.0, 0.0);
        }
        let m = self.likely.max(0.0);
        let a = self.optimistic.max(0.0).min(m);
        let b = self.pessimistic.max(m);
        (a, m, b)
    }
}

/// Unit-economics assumptions (all editable on the page).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Econ {
    pub retail_price_usd: f64,
    /// Share of units sold direct to consumer (DTC); the rest through retailers.
    pub direct_share_pct: f64,
    /// Retailer's margin as a share of the retail price.
    pub retail_margin_pct: f64,
    /// Payment processing and web-shop fees on DTC sales, % of price.
    pub payment_fee_pct: f64,
    /// Freight, duties and fulfilment per unit.
    pub logistics_usd: f64,
    /// Units returned or replaced under warranty, %.
    pub warranty_return_pct: f64,
    /// Reverse logistics and handling per return.
    pub return_handling_usd: f64,
    /// Certification total; None = midpoint of the compliance plan's indicative range.
    pub certification_usd: Option<f64>,
    /// Line fixtures and test jigs (one-off).
    pub fixtures_usd: f64,
    /// Development budget; None = the schedule's budget at completion (BAC).
    pub dev_cost_usd: Option<f64>,
    /// Units sold per year after launch; empty = target quantity × [1, 2.5, 5].
    pub volumes: Vec<f64>,
    /// Sales, marketing and support, % of net revenue.
    pub opex_pct: f64,
    /// Annual discount rate for NPV, %.
    pub discount_rate_pct: f64,
    /// Launch after the deterministic plan (`cpm`) or the Monte Carlo `p50` / `p80` finish.
    pub launch: String,
}

impl Default for Econ {
    fn default() -> Self {
        Econ {
            retail_price_usd: 249.0,
            direct_share_pct: 50.0,
            retail_margin_pct: 40.0,
            payment_fee_pct: 3.5,
            logistics_usd: 6.0,
            warranty_return_pct: 5.0,
            return_handling_usd: 12.0,
            certification_usd: None,
            fixtures_usd: 15000.0,
            dev_cost_usd: None,
            volumes: Vec::new(),
            opex_pct: 15.0,
            discount_rate_pct: 12.0,
            launch: "p80".into(),
        }
    }
}

/// Readiness evidence for one subsystem.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Evidence {
    /// `new` | `modified` | `cots` (commercial off-the-shelf); empty = suggest from the domain.
    pub heritage: String,
    /// Highest build demonstrated: none | breadboard | brassboard | evt | dvt | pvt | production.
    pub stage: String,
    pub trl_override: Option<u8>,
    pub mrl_override: Option<u8>,
    pub note: String,
}

impl Default for Evidence {
    fn default() -> Self {
        Evidence {
            heritage: String::new(),
            stage: "none".into(),
            trl_override: None,
            mrl_override: None,
            note: String::new(),
        }
    }
}

/// `design.extra.program`: everything this lab stores in the project.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    /// Project start date (ISO 8601), used for calendar dates in exports.
    pub start_date: String,
    /// Edited task network; None = the default derived from the project.
    pub tasks: Option<Vec<Task>>,
    /// Blended, fully loaded engineering labour rate, USD/h.
    pub eng_rate_usd_h: f64,
    /// Per-owner overrides of the labour rate.
    pub rates: BTreeMap<String, f64>,
    /// Monte Carlo sampling: `pert` (beta-PERT) or `triangular`.
    pub dist: String,
    pub iterations: usize,
    pub seed: u64,
    /// Frame tooling at PVT: `auto` (follow the Manufacturing Hub's cheapest process) or `mold` (hard tool, per the Build Guide roadmap).
    pub frame_tooling: String,
    pub economics: Econ,
    /// Subsystem id → readiness evidence.
    pub readiness: BTreeMap<String, Evidence>,
    /// Manual review-criterion check marks, `"PDR.files"` → true.
    pub reviews: BTreeMap<String, bool>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            start_date: "2026-10-05".into(),
            tasks: None,
            eng_rate_usd_h: 75.0,
            rates: BTreeMap::new(),
            dist: "pert".into(),
            iterations: 2000,
            seed: 7,
            frame_tooling: "auto".into(),
            economics: Econ::default(),
            readiness: BTreeMap::new(),
            reviews: BTreeMap::new(),
        }
    }
}

impl Config {
    /// Read `design.extra.program`, falling back to defaults field by field.
    pub fn from_project(p: &Project) -> Config {
        p.design
            .extra
            .get("program")
            .and_then(|v| serde_json::from_value::<Config>(v.clone()).ok())
            .unwrap_or_default()
    }

    /// The task network: the edited one if present, otherwise the project default.
    pub fn tasks(&self, p: &Project) -> Vec<Task> {
        match &self.tasks {
            Some(t) if !t.is_empty() => t.clone(),
            _ => default_tasks(p, self),
        }
    }

    pub fn rate(&self, owner: &str) -> f64 {
        self.rates.get(owner).copied().unwrap_or(self.eng_rate_usd_h)
    }
}

/// Labour + expense budget of one task, USD.
pub fn task_budget(t: &Task, cfg: &Config) -> f64 {
    if t.milestone {
        return t.expense_usd.max(0.0);
    }
    t.fte.max(0.0) * t.likely.max(0.0) * 40.0 * cfg.rate(&t.owner) + t.expense_usd.max(0.0)
}

/// Parse `"T05"`, `"T05+2"`, `"T05-1.5"` → (id, lag weeks). `known` tells
/// whether a string is an existing task id, so ids containing a dash
/// (`M-3`) are never mistaken for a lead (`M` − 3 weeks).
pub fn parse_pred(s: &str, known: &dyn Fn(&str) -> bool) -> Option<(String, f64)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if known(s) {
        return Some((s.to_string(), 0.0));
    }
    for (i, c) in s.char_indices().rev() {
        if (c == '+' || c == '-') && i > 0 {
            let head = s[..i].trim();
            if let Ok(v) = s[i + 1..].trim().parse::<f64>() {
                if known(head) {
                    return Some((head.to_string(), if c == '-' { -v } else { v }));
                }
            }
        }
    }
    Some((s.to_string(), 0.0))
}

/// Which milestone of the project plays each role (concept, requirements,
/// design, EVT, DVT, PVT); missing ones get generic names.
fn milestone_ids(p: &Project) -> [(String, String); 6] {
    let keys: [(&str, &[&str], &str, &str); 6] = [
        ("concept", &["concept"], "M-0", "Concept review"),
        ("req", &["requirement"], "M-1", "Requirements review"),
        ("design", &["preliminary", "design review", "design"], "M-2", "Preliminary design"),
        ("evt", &["evt", "engineering validation"], "M-3", "EVT: engineering validation"),
        ("dvt", &["dvt", "design validation"], "M-4", "DVT: design validation"),
        ("pvt", &["pvt", "production validation"], "M-5", "PVT: production validation"),
    ];
    let mut used: Vec<String> = Vec::new();
    let mut out: Vec<(String, String)> = Vec::new();
    for (k, words, did, dtitle) in keys {
        let found = p.milestones.iter().find(|m| {
            if used.contains(&m.id) {
                return false;
            }
            let s = format!("{} {}", m.stage, m.title).to_ascii_lowercase();
            let st = crate::build::roadmap::stage_of(&m.stage, &m.title);
            match k {
                "evt" | "dvt" | "pvt" => st == k,
                _ => st == "proto" && words.iter().any(|w| s.contains(w)),
            }
        });
        match found {
            Some(m) => {
                used.push(m.id.clone());
                out.push((m.id.clone(), m.title.clone()));
            }
            None => {
                let id = if p.milestones.iter().any(|m| m.id == did) {
                    format!("{did}*")
                } else {
                    did.to_string()
                };
                out.push((id, dtitle.to_string()));
            }
        }
    }
    [
        out[0].clone(),
        out[1].clone(),
        out[2].clone(),
        out[3].clone(),
        out[4].clone(),
        out[5].clone(),
    ]
}

/// Mid-point of a roadmap quantity string such as "5–20 units".
fn qty_mid(s: &str, fallback: f64) -> f64 {
    let nums: Vec<f64> = s
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .filter_map(|x| x.parse().ok())
        .collect();
    match nums.as_slice() {
        [a, b, ..] => ((a + b) / 2.0).round(),
        [a] => *a,
        _ => fallback,
    }
}

/// Cost of building `n` units at quantity n (Manufacturing Hub model, indicative).
pub fn build_cost(p: &Project, n: f64) -> f64 {
    let inp = crate::mfg::CostInputs::from_project(p);
    let a = crate::mfg::Assumptions::default();
    let (cats, _, _) = crate::mfg::unit_cost_breakdown(p, &inp, &a, n.max(1.0));
    n.max(1.0) * cats.iter().map(|c| c.1).sum::<f64>()
}

/// Frame process the Manufacturing Hub picks at the target quantity, crossover and total tooling.
#[derive(Clone, Debug)]
pub struct FrameChoice {
    pub process: String,
    pub molded: bool,
    pub crossover: Option<f64>,
    pub tooling_usd: f64,
}

thread_local! {
    static FRAME: crate::memo::Slot<FrameChoice> = const { std::cell::RefCell::new(None) };
}

/// Memoised on the inputs the cost model reads (the crossover search is the slow part).
pub fn frame_choice(p: &Project) -> FrameChoice {
    let d = &p.design;
    let key = crate::memo::key(&(&p.components, &d.frame, &d.optics, &d.pcb, &d.manufacturing, &d.extra));
    crate::memo::memo(&FRAME, key, || frame_choice_uncached(p))
}

fn frame_choice_uncached(p: &Project) -> FrameChoice {
    let inp = crate::mfg::CostInputs::from_project(p);
    let a = crate::mfg::Assumptions::default();
    let q = p.design.manufacturing.target_quantity.max(1.0);
    let (_, fname, _) = crate::mfg::unit_cost_breakdown(p, &inp, &a, q);
    FrameChoice {
        molded: fname.starts_with("Injection"),
        process: fname,
        crossover: crate::mfg::crossover(p, &inp, &a),
        tooling_usd: inp.molded.iter().map(|x| x.tool_cost_usd).sum(),
    }
}

/// Whether the plan builds a hard steel mold at PVT.
pub fn plans_mold(p: &Project, cfg: &Config) -> bool {
    cfg.frame_tooling.eq_ignore_ascii_case("mold") || frame_choice(p).molded
}

#[allow(clippy::too_many_arguments)]
fn t(id: &str, wbs: &str, name: impl Into<String>, phase: &str, owner: &str, abc: [f64; 3], preds: &[&str], source: &str) -> Task {
    Task {
        id: id.into(),
        wbs: wbs.into(),
        name: name.into(),
        phase: phase.into(),
        owner: owner.into(),
        optimistic: abc[0],
        likely: abc[1],
        pessimistic: abc[2],
        preds: preds.iter().map(|s| s.to_string()).collect(),
        milestone: false,
        fte: 1.0,
        expense_usd: 0.0,
        source: source.into(),
    }
}

fn ms(id: &str, wbs: &str, name: &str, phase: &str, preds: &[String], source: &str) -> Task {
    Task {
        id: id.into(),
        wbs: wbs.into(),
        name: name.into(),
        phase: phase.into(),
        owner: "Program".into(),
        preds: preds.to_vec(),
        milestone: true,
        fte: 0.0,
        source: source.into(),
        ..Default::default()
    }
}

/// The default task network for a project (Iris: 36 tasks, milestones M-0 … M-5).
pub fn default_tasks(p: &Project, cfg: &Config) -> Vec<Task> {
    let [m0, m1, m2, m3, m4, m5] = milestone_ids(p);
    let road = crate::build::roadmap::roadmap(p);
    let stage = |id: &str| road.iter().find(|s| s.id == id);
    let build_name = |sid: &str, k: usize, fallback: &str| -> String {
        stage(sid)
            .and_then(|s| s.builds.get(k))
            .map(|b| b.what.clone())
            .unwrap_or_else(|| fallback.to_string())
    };
    let q_evt = stage("evt").map(|s| qty_mid(&s.quantity, 12.0)).unwrap_or(12.0);
    let q_dvt = stage("dvt").map(|s| qty_mid(&s.quantity, 60.0)).unwrap_or(60.0);
    let q_pvt = stage("pvt").map(|s| qty_mid(&s.quantity, 125.0)).unwrap_or(125.0);
    let has_silicon = p.subsystems.iter().any(|s| s.domain == crate::project::Domain::Silicon);
    let fc = frame_choice(p);
    let mold = plans_mold(p, cfg);

    let mut v: Vec<Task> = Vec::new();
    // 1 Concept
    v.push(t(
        "T01",
        "1.1",
        "Stakeholder needs, use cases and concept of operations",
        "Concept",
        "Systems",
        [1.0, 2.0, 3.0],
        &[],
        "project.needs",
    ));
    v.push(t(
        "T02",
        "1.2",
        "Trade studies: display, radio, battery, frame process",
        "Concept",
        "Systems",
        [2.0, 3.0, 5.0],
        &["T01"],
        "project.trades",
    ));
    v.push(ms(&m0.0, "1.3", &m0.1, "Concept", &["T02".into()], &m0.0));
    // 2 Requirements
    v.push(t(
        "T03",
        "2.1",
        "Requirements with verification metrics and owners",
        "Requirements",
        "Systems",
        [2.0, 3.0, 5.0],
        &[&m0.0],
        "project.requirements",
    ));
    v.push(t(
        "T04",
        "2.2",
        "Architecture, interfaces, mass and power budgets",
        "Requirements",
        "Systems",
        [1.0, 2.0, 4.0],
        &[&m0.0],
        "project.interfaces",
    ));
    v.push(ms(
        &m1.0,
        "2.3",
        &format!("{} (SRR)", m1.1),
        "Requirements",
        &["T03".into(), "T04".into()],
        &m1.0,
    ));
    // 3 Preliminary design
    v.push(t(
        "T05",
        "3.1",
        "Optics design: collimator, combiner and tolerances",
        "Design",
        "Optics",
        [3.0, 4.0, 8.0],
        &[&m1.0],
        "/optics",
    ));
    v.push(t(
        "T06",
        "3.2",
        "Schematic, power tree and circuit simulation",
        "Design",
        "Electrical",
        [2.0, 4.0, 6.0],
        &[&m1.0],
        "/circuit",
    ));
    v.push(t(
        "T07",
        "3.3",
        "Frame CAD v1, structures and test prints",
        "Design",
        "Mechanical",
        [2.0, 4.0, 6.0],
        &[&m1.0],
        "/cad",
    ));
    v.push(t(
        "T08",
        "3.4",
        "Firmware on development kits",
        "Design",
        "Firmware",
        [3.0, 5.0, 8.0],
        &[&m1.0],
        "/proto",
    ));
    if has_silicon {
        v.push(t(
            "T09",
            "3.5",
            "IRIS-DSP co-processor: RTL and FPGA prototype (optional block)",
            "Design",
            "Silicon",
            [4.0, 6.0, 10.0],
            &[&m1.0],
            "/chip",
        ));
    }
    v.push(ms(
        &m2.0,
        "3.6",
        &format!("{} (PDR)", m2.1),
        "Design",
        &["T05".into(), "T06".into(), "T07".into(), "T08".into()],
        &m2.0,
    ));
    // 4 EVT
    let mut t10 = t(
        "T10",
        "4.1",
        format!("{} — layout, DRC, order", build_name("evt", 0, "Main PCB assembled")),
        "EVT",
        "Electrical",
        [2.0, 3.0, 5.0],
        &[&m2.0],
        "roadmap:evt",
    );
    t10.source = "roadmap:evt · /pcb".into();
    v.push(t10);
    v.push(t(
        "T11",
        "4.2",
        build_name("evt", 1, "Printed frames"),
        "EVT",
        "Mechanical",
        [1.0, 2.0, 3.0],
        &[&m2.0],
        "roadmap:evt · /print",
    ));
    v.push(t(
        "T12",
        "4.3",
        build_name("evt", 2, "Optics module"),
        "EVT",
        "Optics",
        [2.0, 3.0, 5.0],
        &[&m2.0],
        "roadmap:evt · /optics",
    ));
    let mut t13 = t(
        "T13",
        "4.4",
        format!("EVT assembly and board bring-up ({q_evt:.0} units)"),
        "EVT",
        "Electrical",
        [1.0, 2.0, 4.0],
        &["T10", "T11", "T12"],
        "roadmap:evt · /mfg",
    );
    t13.expense_usd = build_cost(p, q_evt).round();
    v.push(t13);
    v.push(t(
        "T14",
        "4.5",
        "EVT tests: power, battery life, skin temperature, optics, radio link",
        "EVT",
        "Test",
        [2.0, 3.0, 5.0],
        &["T13", "T08"],
        "roadmap:evt",
    ));
    v.push(ms(&m3.0, "4.6", &format!("{} (CDR)", m3.1), "EVT", &["T14".into()], &m3.0));
    // 5 DVT
    let mut t15 = t(
        "T15",
        "5.1",
        "Design updates from EVT → production intent",
        "DVT",
        "Systems",
        [2.0, 3.0, 5.0],
        &[&m3.0],
        "CDR actions",
    );
    t15.fte = 2.0;
    if has_silicon {
        t15.preds.push("T09".into());
    }
    v.push(t15);
    v.push(t(
        "T16",
        "5.2",
        build_name("dvt", 0, "Frames from soft tooling"),
        "DVT",
        "Manufacturing",
        [3.0, 4.0, 6.0],
        &["T15"],
        "roadmap:dvt · /molding",
    ));
    v.push(t(
        "T17",
        "5.3",
        build_name("dvt", 1, "Production-intent PCBA"),
        "DVT",
        "Electrical",
        [2.0, 3.0, 5.0],
        &["T15"],
        "roadmap:dvt · /pcb",
    ));
    v.push(t(
        "T18",
        "5.4",
        build_name("dvt", 2, "Hinge flex cables"),
        "DVT",
        "Mechanical",
        [3.0, 4.0, 8.0],
        &["T15"],
        "roadmap:dvt",
    ));
    let mut t19 = t(
        "T19",
        "5.5",
        format!("DVT build ({q_dvt:.0} units)"),
        "DVT",
        "Manufacturing",
        [1.0, 1.0, 2.0],
        &["T16", "T17", "T18"],
        "roadmap:dvt · /mfg",
    );
    t19.expense_usd = build_cost(p, q_dvt).round();
    v.push(t19);
    v.push(ms("TRR", "5.6", "Test readiness review (TRR)", "DVT", &["T19".into()], "review"));
    v.push(t(
        "T20",
        "5.7",
        "Reliability tests: drop, hinge-fold cycling, sweat, temperature",
        "DVT",
        "Test",
        [3.0, 4.0, 6.0],
        &["TRR"],
        "roadmap:dvt · /structures",
    ));
    v.push(t(
        "T21",
        "5.8",
        "Pre-compliance EMC / radio scan",
        "DVT",
        "Compliance",
        [1.0, 1.0, 2.0],
        &["TRR"],
        "compliance:prescan",
    ));
    let mut m4_preds: Vec<String> = vec!["T20".into(), "T21".into()];

    // compliance work packages, grouped by stage and category
    let plan = crate::build::compliance::plan(p, &crate::build::compliance::project_options(p));
    let mut groups: Vec<(&str, &str, Vec<&crate::build::compliance::Item>)> = Vec::new();
    for it in &plan.items {
        match groups.iter_mut().find(|g| g.0 == it.stage && g.1 == it.category) {
            Some(g) => g.2.push(it),
            None => groups.push((it.stage, it.category, vec![it])),
        }
    }
    let mut n_dvt = 8usize;
    let mut n_pvt = 7usize;
    let mut pvt_groups: Vec<Task> = Vec::new();
    for (st, cat, items) in &groups {
        let lo = items.iter().map(|i| i.weeks[0]).fold(0.0, f64::max);
        let mid = items.iter().map(|i| 0.5 * (i.weeks[0] + i.weeks[1])).fold(0.0, f64::max);
        let hi = items.iter().map(|i| i.weeks[1]).fold(0.0, f64::max);
        let fee: f64 = items.iter().filter_map(|i| i.cost_usd).map(|c| 0.5 * (c[0] + c[1])).sum();
        let ids: Vec<&str> = items.iter().map(|i| i.id).collect();
        let name = format!(
            "{cat} certification: {}",
            items.iter().map(|i| short_title(&i.title)).collect::<Vec<_>>().join("; ")
        );
        let (phase, pred) = match *st {
            "PVT" => ("PVT", "T26".to_string()),
            "EVT" => ("EVT", m2.0.clone()),
            _ => ("DVT", "TRR".to_string()),
        };
        let wbs = match phase {
            "PVT" => {
                n_pvt += 1;
                format!("6.{n_pvt}")
            }
            "EVT" => "4.7".into(),
            _ => {
                n_dvt += 1;
                format!("5.{n_dvt}")
            }
        };
        let mut task = t(
            &format!("C-{}{}", &phase[..1], cat.chars().take(3).collect::<String>().to_ascii_uppercase()),
            &wbs,
            name,
            phase,
            "Compliance",
            [lo, mid, hi],
            &[&pred],
            &format!("compliance:{}", ids.join(",")),
        );
        task.fte = 0.5;
        task.expense_usd = fee.round();
        match phase {
            "PVT" => pvt_groups.push(task),
            "EVT" => {
                // feeds the EVT exit
                if let Some(m) = v.iter_mut().find(|x| x.id == m3.0) {
                    m.preds.push(task.id.clone());
                }
                v.insert(v.iter().position(|x| x.id == m3.0).unwrap_or(v.len()), task);
            }
            _ => {
                m4_preds.push(task.id.clone());
                v.push(task);
            }
        }
    }
    v.push(ms(&m4.0, "5.9", &format!("{} — design freeze", m4.1), "DVT", &m4_preds, &m4.0));
    // 6 PVT
    let tool = if mold {
        let mut x = t(
            "T23",
            "6.1",
            build_name("pvt", 0, "Hardened steel mold, T1 → T2 samples"),
            "PVT",
            "Manufacturing",
            [6.0, 8.0, 12.0],
            &[&m4.0],
            "roadmap:pvt · /molding",
        );
        x.expense_usd = fc.tooling_usd.round();
        x
    } else {
        let short = fc.process.split(' ').next().unwrap_or("printing").to_string();
        let mut x = t(
            "T23",
            "6.1",
            format!(
                "Production frames by {short} — capacity, fixtures and cosmetic finish (molding pays off above {})",
                fc.crossover
                    .map(|q| format!("{q:.0} units"))
                    .unwrap_or_else(|| "the modelled range".into())
            ),
            "PVT",
            "Manufacturing",
            [2.0, 3.0, 5.0],
            &[&m4.0],
            "/mfg cost model",
        );
        x.fte = 1.0;
        x
    };
    v.push(tool);
    let mut t24 = t(
        "T24",
        "6.2",
        build_name("pvt", 1, "Line fixtures, test jigs, work instructions"),
        "PVT",
        "Manufacturing",
        [3.0, 4.0, 6.0],
        &[&m4.0],
        "roadmap:pvt · /mfg",
    );
    t24.expense_usd = cfg.economics.fixtures_usd.max(0.0);
    v.push(t24);
    v.push(t(
        "T25",
        "6.3",
        build_name("pvt", 2, "Production PCBA with test"),
        "PVT",
        "Electrical",
        [2.0, 3.0, 5.0],
        &[&m4.0],
        "roadmap:pvt · /pcb",
    ));
    v.push(ms(
        "PRR",
        "6.4",
        "Production readiness review (PRR)",
        "PVT",
        &["T23".into(), "T24".into(), "T25".into()],
        "review",
    ));
    let mut t26 = t(
        "T26",
        "6.5",
        format!("PVT pilot build ({q_pvt:.0} units)"),
        "PVT",
        "Manufacturing",
        [1.0, 2.0, 3.0],
        &["PRR"],
        "roadmap:pvt · /mfg",
    );
    t26.expense_usd = build_cost(p, q_pvt).round();
    t26.fte = 2.0;
    v.push(t26);
    v.push(t(
        "T27",
        "6.6",
        "Yield, cycle time and first article inspection",
        "PVT",
        "Test",
        [1.0, 2.0, 3.0],
        &["T26"],
        "roadmap:pvt",
    ));
    let mut m5_preds: Vec<String> = vec!["T27".into()];
    for g in pvt_groups {
        m5_preds.push(g.id.clone());
        v.push(g);
    }
    v.push(ms(&m5.0, "6.9", &m5.1, "PVT", &m5_preds, &m5.0));
    v
}

/// Short form of a compliance item title ("FCC Part 15 Subpart B — …" → "FCC Part 15 Subpart B").
fn short_title(s: &str) -> String {
    s.split(" — ").next().unwrap_or(s).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pred_parsing() {
        let known = |s: &str| ["T05", "M-3", "M"].contains(&s);
        assert_eq!(parse_pred("T05", &known), Some(("T05".into(), 0.0)));
        assert_eq!(parse_pred("T05+2", &known), Some(("T05".into(), 2.0)));
        assert_eq!(parse_pred(" M-3 - 1.5 ", &known), Some(("M-3".into(), -1.5)));
        assert_eq!(parse_pred("M-3", &known), Some(("M-3".into(), 0.0)));
        assert_eq!(parse_pred("M-1", &known), Some(("M".into(), -1.0)));
        assert_eq!(parse_pred("", &known), None);
    }

    #[test]
    fn iris_default_network_links_milestones_roadmap_and_compliance() {
        let p = Project::iris();
        let v = default_tasks(&p, &Config::default());
        for id in ["M-0", "M-1", "M-2", "M-3", "M-4", "M-5"] {
            assert!(v.iter().any(|t| t.id == id && t.milestone), "{id}");
        }
        assert!(v.iter().any(|t| t.name.contains("2-layer")), "roadmap build names used");
        assert!(
            v.iter().any(|t| t.id.starts_with("C-P") && t.expense_usd > 0.0),
            "certification fees"
        );
        assert!(v.iter().find(|t| t.id == "T13").unwrap().expense_usd > 100.0);
        // every predecessor exists
        let known = |s: &str| v.iter().any(|x| x.id == s);
        for t in &v {
            for pr in &t.preds {
                let (id, _) = parse_pred(pr, &known).unwrap();
                assert!(v.iter().any(|x| x.id == id), "{} → {}", t.id, id);
            }
        }
    }
}
