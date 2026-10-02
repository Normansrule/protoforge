//! Technology and manufacturing readiness, and systems-engineering review gates.
//!
//! **Technology readiness level (TRL)** 1–9 and **manufacturing readiness
//! level (MRL)** 1–10 are *suggested* per subsystem from evidence already in
//! the project — which requirements are allocated to it, whether they are
//! bound to a lab metric (analysis) or verified by test / demonstration /
//! inspection, the subsystem's heritage (new, modified, commercial
//! off-the-shelf) and the highest build it has demonstrated (breadboard →
//! brassboard → EVT → DVT → PVT → production). The ladder rules below are
//! this lab's reading of the level definitions — an assessor decides, and
//! every level can be overridden with a note.
//!
//! Review gates follow the NASA / U.S. Department of Defense (DoD) sequence:
//! system requirements review (SRR), preliminary design review (PDR),
//! critical design review (CDR), test readiness review (TRR) and production
//! readiness review (PRR), each with entry and exit criteria; criteria that
//! can be checked from the project (requirement status, readiness levels,
//! schedule and business metrics) are ticked automatically, the rest are
//! manual check marks saved in `design.extra.program.reviews`.
//!
//! References: U.S. DoD, *Technology Readiness Assessment (TRA) Guidebook*
//! (Office of the Under Secretary of Defense for Research and Engineering);
//! DoD, *Manufacturing Readiness Level (MRL) Deskbook* (OSD Manufacturing
//! Technology Program — check the current edition); NASA,
//! *Systems Engineering Handbook*, SP-2016-6105 Rev 2 (2016); J. C. Mankins,
//! "Technology readiness levels: a white paper", NASA (1995).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::wbs::{Config, Evidence};
use crate::project::{Domain, Project, ReqKind, VerifyMethod};
use crate::systems::Status;

pub const STAGES: [&str; 7] = ["none", "breadboard", "brassboard", "evt", "dvt", "pvt", "production"];

pub fn stage_rank(s: &str) -> usize {
    STAGES.iter().position(|x| x.eq_ignore_ascii_case(s.trim())).unwrap_or(0)
}

pub const TRL_NAMES: [&str; 9] = [
    "Basic principles observed",
    "Technology concept formulated",
    "Analytical proof of concept",
    "Validated in a laboratory environment",
    "Validated in a relevant environment",
    "Prototype demonstrated in a relevant environment",
    "Prototype demonstrated in an operational environment",
    "System complete and qualified",
    "Proven in operation",
];

pub const MRL_NAMES: [&str; 10] = [
    "Basic manufacturing implications identified",
    "Manufacturing concepts identified",
    "Manufacturing proof of concept developed",
    "Capability to produce in a laboratory environment",
    "Prototype components in a production-relevant environment",
    "Prototype system in a production-relevant environment",
    "System or components in a production-representative environment",
    "Pilot line demonstrated; ready for low-rate production",
    "Low-rate production demonstrated",
    "Full-rate production demonstrated",
];

#[derive(Clone, Debug, Serialize)]
pub struct Rung {
    pub level: u8,
    pub name: String,
    pub met: bool,
    pub why: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReqEvidence {
    pub id: String,
    pub method: VerifyMethod,
    pub kind: ReqKind,
    pub metric: Option<String>,
    pub status: Option<Status>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SubsystemReadiness {
    pub id: String,
    pub name: String,
    pub domain: Domain,
    pub lab: String,
    pub heritage: String,
    pub heritage_suggested: bool,
    pub stage: String,
    pub trl_suggested: u8,
    pub trl: u8,
    pub mrl_suggested: u8,
    pub mrl: u8,
    pub trl_ladder: Vec<Rung>,
    pub mrl_ladder: Vec<Rung>,
    pub reqs: Vec<ReqEvidence>,
    pub warnings: Vec<String>,
    pub note: String,
}

/// Heritage suggested from the domain: catalogue electronics are commercial
/// off-the-shelf (COTS); boards, optics, frames, firmware and silicon are new.
pub fn suggest_heritage(p: &Project, sid: &str) -> String {
    let Some(s) = p.subsystems.iter().find(|s| s.id == sid) else {
        return "new".into();
    };
    let name = s.name.to_ascii_lowercase();
    let board = name.contains("pcb") || name.contains("board");
    match s.domain {
        Domain::Electrical | Domain::Acoustic if !board => "cots".into(),
        _ => "new".into(),
    }
}

fn rung(level: u8, names: &[&str], met: bool, why: impl Into<String>) -> Rung {
    Rung {
        level,
        name: names[level as usize - 1].to_string(),
        met,
        why: why.into(),
    }
}

/// Highest level whose rung and all lower rungs are met.
fn climb(ladder: &[Rung]) -> u8 {
    let mut l = 0;
    for r in ladder {
        if r.met {
            l = r.level;
        } else {
            break;
        }
    }
    l.max(1)
}

/// Per-subsystem readiness. `status` (requirement id → pass/fail) is optional
/// and only adds warnings; the suggested levels depend on evidence, not on
/// today's pass/fail (so the metric does not have to run every analysis).
pub fn assess(p: &Project, cfg: &Config, status: Option<&BTreeMap<String, Status>>) -> Vec<SubsystemReadiness> {
    let mut out = Vec::new();
    for s in &p.subsystems {
        let ev = cfg.readiness.get(&s.id).cloned().unwrap_or_default();
        let heritage_suggested = ev.heritage.trim().is_empty();
        let heritage = if heritage_suggested {
            suggest_heritage(p, &s.id)
        } else {
            ev.heritage.to_ascii_lowercase()
        };
        let st = stage_rank(&ev.stage);
        let reqs: Vec<ReqEvidence> = p
            .requirements
            .iter()
            .filter(|r| r.allocated_to.iter().any(|a| a == &s.id))
            .map(|r| ReqEvidence {
                id: r.id.clone(),
                method: r.verification.method,
                kind: r.kind,
                metric: r.verification.metric.clone(),
                status: status.and_then(|m| m.get(&r.id).copied()),
            })
            .collect();
        let n = reqs.len();
        let analysed = reqs.iter().filter(|r| r.metric.is_some()).count();
        let tested = reqs
            .iter()
            .filter(|r| matches!(r.method, VerifyMethod::Test | VerifyMethod::Demonstration))
            .count();
        let beyond_analysis = reqs.iter().filter(|r| r.method != VerifyMethod::Analysis).count();
        let mfg_reqs = reqs
            .iter()
            .filter(|r| matches!(r.kind, ReqKind::Manufacturing | ReqKind::Cost) && r.metric.is_some())
            .count();
        let cots = heritage == "cots";
        // software is "manufactured" by the build toolchain and the programming step of final test
        let costed = p.components.iter().any(|c| c.subsystem == s.id && c.unit_cost_usd > 0.0)
            || matches!(s.domain, Domain::Mechanical | Domain::Manufacturing | Domain::Software);
        let tn = &TRL_NAMES;
        let trl_ladder = vec![
            rung(1, tn, true, "always — the physics is known"),
            rung(
                2,
                tn,
                !s.description.is_empty() || !s.functions.is_empty(),
                format!("{} function(s) described", s.functions.len()),
            ),
            rung(
                3,
                tn,
                analysed > 0,
                format!("{analysed} of {n} allocated requirement(s) have a lab metric (analytical proof of concept)"),
            ),
            rung(
                4,
                tn,
                st >= 1 || cots,
                if cots {
                    "commercial off-the-shelf parts: vendor evaluation kits validate the components in a laboratory environment".to_string()
                } else {
                    format!("demonstrated build: {} (needs a breadboard)", STAGES[st])
                },
            ),
            rung(
                5,
                tn,
                st >= 2,
                format!("demonstrated build: {} (needs a brassboard in the real form factor)", STAGES[st]),
            ),
            rung(
                6,
                tn,
                st >= 3 && tested > 0,
                format!(
                    "build {} (needs EVT) and {tested} requirement(s) verified by test or demonstration (needs ≥ 1)",
                    STAGES[st]
                ),
            ),
            rung(
                7,
                tn,
                st >= 4 && tested > 0,
                format!("build {} (needs DVT, worn in real use)", STAGES[st]),
            ),
            rung(
                8,
                tn,
                st >= 5 && beyond_analysis == n && n > 0,
                format!(
                    "build {} (needs PVT) and {beyond_analysis} of {n} requirements verified beyond analysis (needs all)",
                    STAGES[st]
                ),
            ),
            rung(
                9,
                tn,
                st >= 6,
                format!("build {} (needs production units in customers' hands)", STAGES[st]),
            ),
        ];
        let mn = &MRL_NAMES;
        let mrl_ladder = vec![
            rung(1, mn, true, "always"),
            rung(2, mn, !s.lab.is_empty() || costed, "a lab or bill-of-materials lines describe how it is made"),
            rung(3, mn, costed, "the Manufacturing Hub cost model covers its parts or process (software: the build toolchain and the programming step of final test)"),
            rung(
                4,
                mn,
                mfg_reqs > 0 || st >= 1,
                format!("{mfg_reqs} manufacturability / cost requirement(s) with a metric (producibility analysed) or a lab-built part"),
            ),
            rung(5, mn, st >= 2, format!("build {} (needs brassboard parts made with production-relevant processes)", STAGES[st])),
            rung(6, mn, st >= 3, format!("build {} (needs an EVT system built with production-relevant processes)", STAGES[st])),
            rung(7, mn, st >= 4, format!("build {} (needs DVT parts from production-representative tooling)", STAGES[st])),
            rung(8, mn, st >= 5, format!("build {} (needs a PVT pilot line)", STAGES[st])),
            rung(9, mn, st >= 6, format!("build {} (needs low-rate production)", STAGES[st])),
            rung(10, mn, false, "full-rate production — set by override once demonstrated"),
        ];
        let trl_s = climb(&trl_ladder);
        let mrl_s = climb(&mrl_ladder);
        let mut warnings = Vec::new();
        for r in &reqs {
            if r.status == Some(Status::Fail) && r.kind != ReqKind::Goal {
                warnings.push(format!("{} fails today — the analysis does not yet close", r.id));
            }
        }
        if n == 0 {
            warnings.push("no requirements allocated — nothing to prove readiness against".into());
        }
        if tested == 0 && n > 0 {
            warnings.push(
                "every allocated requirement is verified by analysis only — TRL 6 needs at least one verified by test or demonstration"
                    .into(),
            );
        }
        let (trl, mrl) = (
            ev.trl_override.map(|x| x.clamp(1, 9)).unwrap_or(trl_s),
            ev.mrl_override.map(|x| x.clamp(1, 10)).unwrap_or(mrl_s),
        );
        if mrl > trl + 2 {
            warnings.push("manufacturing readiness far ahead of technology readiness — check the evidence".into());
        }
        out.push(SubsystemReadiness {
            id: s.id.clone(),
            name: s.name.clone(),
            domain: s.domain,
            lab: s.lab.clone(),
            heritage,
            heritage_suggested,
            stage: STAGES[st].into(),
            trl_suggested: trl_s,
            trl,
            mrl_suggested: mrl_s,
            mrl,
            trl_ladder,
            mrl_ladder,
            reqs,
            warnings,
            note: ev.note.clone(),
        });
    }
    out
}

// ---------------------------------------------------------------------------
// Review gates
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Crit {
    pub key: String,
    pub text: String,
    /// entry | exit
    pub phase: String,
    pub auto: bool,
    pub met: bool,
    pub detail: String,
    /// Requirement ids this criterion looked at (with their live status).
    pub reqs: Vec<(String, String)>,
    pub route: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Review {
    pub id: String,
    pub name: String,
    pub milestone: String,
    pub week: Option<f64>,
    pub purpose: String,
    pub criteria: Vec<Crit>,
    pub met: usize,
    pub total: usize,
    pub ready: bool,
}

/// Program numbers the reviews look at.
#[derive(Clone, Debug, Default)]
pub struct ProgramFacts {
    pub cpm_weeks: f64,
    pub p80_weeks: f64,
    pub gross_margin_pct: f64,
    pub breakeven_units: f64,
    pub lifetime_units: f64,
    pub npv_usd: f64,
    /// Milestone id → planned finish week.
    pub milestone_weeks: BTreeMap<String, f64>,
    pub compliance_items: usize,
}

fn st_str(s: Option<Status>) -> String {
    match s {
        Some(Status::Pass) => "pass",
        Some(Status::Fail) => "fail",
        _ => "unverified",
    }
    .into()
}

#[allow(clippy::too_many_arguments)]
fn crit(
    key: &str,
    text: &str,
    phase: &str,
    auto: Option<bool>,
    detail: impl Into<String>,
    reqs: Vec<(String, String)>,
    route: &str,
    manual: &BTreeMap<String, bool>,
) -> Crit {
    let (is_auto, met) = match auto {
        Some(m) => (true, m),
        None => (false, manual.get(key).copied().unwrap_or(false)),
    };
    Crit {
        key: key.into(),
        text: text.into(),
        phase: phase.into(),
        auto: is_auto,
        met,
        detail: detail.into(),
        reqs,
        route: route.into(),
    }
}

pub fn reviews(p: &Project, cfg: &Config, status: &BTreeMap<String, Status>, sub: &[SubsystemReadiness], f: &ProgramFacts) -> Vec<Review> {
    let man = &cfg.reviews;
    let binding: Vec<&crate::project::Requirement> = p.requirements.iter().filter(|r| r.kind != ReqKind::Goal).collect();
    let with_metric = p.requirements.iter().filter(|r| r.verification.metric.is_some()).count();
    let n_req = p.requirements.len();
    let unalloc: Vec<(String, String)> = p
        .requirements
        .iter()
        .filter(|r| r.allocated_to.is_empty())
        .map(|r| (r.id.clone(), st_str(status.get(&r.id).copied())))
        .collect();
    let untraced: Vec<(String, String)> = p
        .requirements
        .iter()
        .filter(|r| r.derives_from.is_empty())
        .map(|r| (r.id.clone(), st_str(status.get(&r.id).copied())))
        .collect();
    let unverified: Vec<(String, String)> = p
        .requirements
        .iter()
        .filter(|r| status.get(&r.id).copied().unwrap_or(Status::Unverified) == Status::Unverified)
        .map(|r| (r.id.clone(), "unverified".into()))
        .collect();
    let failing: Vec<(String, String)> = binding
        .iter()
        .filter(|r| status.get(&r.id).copied() == Some(Status::Fail))
        .map(|r| (r.id.clone(), "fail".into()))
        .collect();
    let open_goals: Vec<(String, String)> = p
        .requirements
        .iter()
        .filter(|r| r.kind == ReqKind::Goal && status.get(&r.id).copied() == Some(Status::Fail))
        .map(|r| (r.id.clone(), "fail".into()))
        .collect();
    let of_kind = |k: &[ReqKind]| -> Vec<(String, String)> {
        p.requirements
            .iter()
            .filter(|r| k.contains(&r.kind))
            .map(|r| (r.id.clone(), st_str(status.get(&r.id).copied())))
            .collect()
    };
    let all_pass = |v: &[(String, String)]| v.iter().all(|x| x.1 == "pass");
    let by_test: Vec<(String, String)> = p
        .requirements
        .iter()
        .filter(|r| matches!(r.verification.method, VerifyMethod::Test | VerifyMethod::Demonstration))
        .map(|r| (r.id.clone(), st_str(status.get(&r.id).copied())))
        .collect();
    let min_trl = sub.iter().map(|s| s.trl).min().unwrap_or(1);
    let min_mrl = sub.iter().map(|s| s.mrl).min().unwrap_or(1);
    let low = |lv: u8, trl: bool| -> String {
        sub.iter()
            .filter(|s| if trl { s.trl < lv } else { s.mrl < lv })
            .map(|s| format!("{} ({})", s.id, if trl { s.trl } else { s.mrl }))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let trades_open: Vec<String> = p
        .trades
        .iter()
        .filter(|t| t.decision.trim().is_empty())
        .map(|t| t.id.clone())
        .collect();
    let risks_unmitigated: Vec<String> = p
        .risks
        .iter()
        .filter(|r| r.mitigation.trim().is_empty())
        .map(|r| r.id.clone())
        .collect();
    let mid = |k: &str| -> String {
        p.milestones
            .iter()
            .find(|m| {
                let s = format!("{} {}", m.stage, m.title).to_ascii_lowercase();
                s.contains(k)
            })
            .map(|m| m.id.clone())
            .unwrap_or_default()
    };
    let wk = |id: &str| f.milestone_weeks.get(id).copied();
    let cost_reqs = of_kind(&[ReqKind::Cost]);
    let mfg_reqs = of_kind(&[ReqKind::Manufacturing]);
    let safety_reqs = of_kind(&[ReqKind::Safety]);

    let srr_m = mid("requirement");
    let pdr_m = mid("preliminary");
    let cdr_m = {
        let x = mid("evt");
        if x.is_empty() {
            mid("engineering validation")
        } else {
            x
        }
    };
    let prr_m = mid("dvt");
    let mut out = vec![
        Review {
            id: "SRR".into(),
            name: "System requirements review".into(),
            milestone: srr_m.clone(),
            week: wk(&srr_m),
            purpose: "Are the requirements complete, verifiable and traced, so design can start?".into(),
            criteria: vec![
                crit(
                    "SRR.needs",
                    "Stakeholder needs and concept of operations documented",
                    "entry",
                    Some(!p.needs.is_empty() && !p.concept.use_cases.is_empty()),
                    format!("{} needs, {} use cases", p.needs.len(), p.concept.use_cases.len()),
                    vec![],
                    "/systems?tab=needs",
                    man,
                ),
                crit(
                    "SRR.trades",
                    "Top trade studies decided",
                    "entry",
                    Some(trades_open.is_empty() && !p.trades.is_empty()),
                    if trades_open.is_empty() {
                        format!("{} trade studies decided", p.trades.len())
                    } else {
                        format!("open: {}", trades_open.join(", "))
                    },
                    vec![],
                    "/systems?tab=trades",
                    man,
                ),
                crit(
                    "SRR.metrics",
                    "Every requirement has a verification method and a metric",
                    "exit",
                    Some(with_metric == n_req && n_req > 0),
                    format!("{with_metric} of {n_req} bound to a lab metric"),
                    vec![],
                    "/systems?tab=requirements",
                    man,
                ),
                crit(
                    "SRR.alloc",
                    "Every requirement is allocated to a subsystem",
                    "exit",
                    Some(unalloc.is_empty() && n_req > 0),
                    if unalloc.is_empty() {
                        "all allocated".to_string()
                    } else {
                        format!("{} unallocated", unalloc.len())
                    },
                    unalloc,
                    "/systems?tab=traceability",
                    man,
                ),
                crit(
                    "SRR.trace",
                    "Every requirement traces to a stakeholder need",
                    "exit",
                    Some(untraced.is_empty() && n_req > 0),
                    if untraced.is_empty() {
                        "all traced".to_string()
                    } else {
                        format!("{} without a parent need", untraced.len())
                    },
                    untraced,
                    "/systems?tab=traceability",
                    man,
                ),
                crit(
                    "SRR.risks",
                    "Risks identified with mitigations (FMEA)",
                    "exit",
                    Some(!p.risks.is_empty() && risks_unmitigated.is_empty()),
                    format!("{} risks, {} without mitigation", p.risks.len(), risks_unmitigated.len()),
                    vec![],
                    "/systems?tab=risks",
                    man,
                ),
                crit(
                    "SRR.plan",
                    "Program plan baselined: schedule and budget",
                    "exit",
                    Some(f.cpm_weeks > 0.0),
                    format!("CPM {:.1} wk, P80 {:.1} wk", f.cpm_weeks, f.p80_weeks),
                    vec![],
                    "/program?tab=schedule",
                    man,
                ),
            ],
            met: 0,
            total: 0,
            ready: false,
        },
        Review {
            id: "PDR".into(),
            name: "Preliminary design review".into(),
            milestone: pdr_m.clone(),
            week: wk(&pdr_m),
            purpose: "Does the preliminary design meet the requirements with acceptable risk, so detailed design can start?".into(),
            criteria: vec![
                crit("PDR.srr", "SRR actions closed", "entry", None, "manual", vec![], "", man),
                crit(
                    "PDR.computed",
                    "Every requirement has a computed value (no unverified)",
                    "exit",
                    Some(unverified.is_empty() && !status.is_empty()),
                    if status.is_empty() {
                        "waiting for verification".to_string()
                    } else {
                        format!("{} unverified", unverified.len())
                    },
                    unverified.clone(),
                    "/systems?tab=requirements",
                    man,
                ),
                crit(
                    "PDR.safety",
                    "Safety requirements pass",
                    "exit",
                    Some(all_pass(&safety_reqs) && !safety_reqs.is_empty()),
                    format!("{} safety requirement(s)", safety_reqs.len()),
                    safety_reqs.clone(),
                    "/systems?tab=requirements",
                    man,
                ),
                crit(
                    "PDR.trl",
                    "Every subsystem at TRL ≥ 4 (components validated on the bench)",
                    "exit",
                    Some(min_trl >= 4),
                    if min_trl >= 4 {
                        "all ≥ 4".to_string()
                    } else {
                        format!("below: {}", low(4, true))
                    },
                    vec![],
                    "/program?tab=readiness",
                    man,
                ),
                crit(
                    "PDR.cost",
                    "Cost requirement within target",
                    "exit",
                    Some(all_pass(&cost_reqs) && !cost_reqs.is_empty()),
                    format!("{} cost requirement(s)", cost_reqs.len()),
                    cost_reqs.clone(),
                    "/mfg",
                    man,
                ),
                crit(
                    "PDR.icd",
                    "Interfaces defined (interface control)",
                    "exit",
                    Some(!p.interfaces.is_empty()),
                    format!("{} interfaces", p.interfaces.len()),
                    vec![],
                    "/systems?tab=architecture",
                    man,
                ),
            ],
            met: 0,
            total: 0,
            ready: false,
        },
        Review {
            id: "CDR".into(),
            name: "Critical design review".into(),
            milestone: cdr_m.clone(),
            week: wk(&cdr_m),
            purpose: "Is the design mature enough to build production-intent hardware and start qualification?".into(),
            criteria: vec![
                crit("CDR.pdr", "PDR actions closed", "entry", None, "manual", vec![], "", man),
                crit(
                    "CDR.pass",
                    "All binding requirements pass (goals may stay open)",
                    "exit",
                    Some(failing.is_empty() && !status.is_empty()),
                    if failing.is_empty() {
                        format!("{} open goal(s)", open_goals.len())
                    } else {
                        format!("{} failing", failing.len())
                    },
                    {
                        let mut v = failing.clone();
                        v.extend(open_goals.clone());
                        v
                    },
                    "/systems?tab=requirements",
                    man,
                ),
                crit(
                    "CDR.mfg",
                    "Manufacturability requirements pass (DFM)",
                    "exit",
                    Some(all_pass(&mfg_reqs) && !mfg_reqs.is_empty()),
                    format!("{} manufacturing requirement(s)", mfg_reqs.len()),
                    mfg_reqs.clone(),
                    "/build",
                    man,
                ),
                crit(
                    "CDR.trl",
                    "Every subsystem at TRL ≥ 6 (EVT prototype tested)",
                    "exit",
                    Some(min_trl >= 6),
                    if min_trl >= 6 {
                        "all ≥ 6".to_string()
                    } else {
                        format!("below: {}", low(6, true))
                    },
                    vec![],
                    "/program?tab=readiness",
                    man,
                ),
                crit(
                    "CDR.files",
                    "Design files released: CAD, schematic, PCB, BOM",
                    "exit",
                    None,
                    "manual — fab package from the Manufacturing Hub",
                    vec![],
                    "/mfg",
                    man,
                ),
            ],
            met: 0,
            total: 0,
            ready: false,
        },
        Review {
            id: "TRR".into(),
            name: "Test readiness review".into(),
            milestone: "TRR".into(),
            week: wk("TRR"),
            purpose: "Are the units, procedures, fixtures and people ready to start formal verification?".into(),
            criteria: vec![
                crit(
                    "TRR.units",
                    "DVT units built and inspected",
                    "entry",
                    None,
                    "manual",
                    vec![],
                    "",
                    man,
                ),
                crit(
                    "TRR.methods",
                    "Requirements to be verified by test are identified",
                    "entry",
                    Some(!by_test.is_empty()),
                    format!("{} requirement(s) verified by test or demonstration", by_test.len()),
                    by_test.clone(),
                    "/systems?tab=requirements",
                    man,
                ),
                crit(
                    "TRR.procs",
                    "Test procedures and pass criteria written for each",
                    "exit",
                    None,
                    "manual",
                    vec![],
                    "",
                    man,
                ),
                crit(
                    "TRR.equip",
                    "Test equipment calibrated; fixtures ready",
                    "exit",
                    None,
                    "manual",
                    vec![],
                    "",
                    man,
                ),
                crit(
                    "TRR.prescan",
                    "Pre-compliance scan planned",
                    "exit",
                    Some(f.compliance_items > 0),
                    format!("{} compliance items in the plan", f.compliance_items),
                    vec![],
                    "/build?tab=compliance",
                    man,
                ),
            ],
            met: 0,
            total: 0,
            ready: false,
        },
        Review {
            id: "PRR".into(),
            name: "Production readiness review".into(),
            milestone: prr_m.clone(),
            week: wk(&prr_m),
            purpose: "Can the factory build it at rate, quality and cost — and is the business case sound?".into(),
            criteria: vec![
                crit(
                    "PRR.mrl",
                    "Every subsystem at MRL ≥ 8 (pilot line)",
                    "entry",
                    Some(min_mrl >= 8),
                    if min_mrl >= 8 {
                        "all ≥ 8".to_string()
                    } else {
                        format!("below: {}", low(8, false))
                    },
                    vec![],
                    "/program?tab=readiness",
                    man,
                ),
                crit(
                    "PRR.cost",
                    "Unit cost within target",
                    "exit",
                    Some(all_pass(&cost_reqs) && !cost_reqs.is_empty()),
                    "",
                    cost_reqs.clone(),
                    "/mfg",
                    man,
                ),
                crit(
                    "PRR.margin",
                    "Gross margin ≥ 40 % at the stated retail price",
                    "exit",
                    Some(f.gross_margin_pct >= 40.0),
                    format!("{:.1} %", f.gross_margin_pct),
                    vec![],
                    "/program?tab=economics",
                    man,
                ),
                crit(
                    "PRR.breakeven",
                    "Break-even inside the planned lifetime volume",
                    "exit",
                    Some(f.breakeven_units <= f.lifetime_units),
                    format!("{:.0} vs {:.0} units", f.breakeven_units, f.lifetime_units),
                    vec![],
                    "/program?tab=economics",
                    man,
                ),
                crit(
                    "PRR.yield",
                    "Line yield and cycle time demonstrated",
                    "exit",
                    None,
                    "manual — PVT pilot",
                    vec![],
                    "",
                    man,
                ),
                crit(
                    "PRR.certs",
                    "Certifications granted",
                    "exit",
                    None,
                    "manual",
                    vec![],
                    "/build?tab=compliance",
                    man,
                ),
            ],
            met: 0,
            total: 0,
            ready: false,
        },
    ];
    for r in &mut out {
        r.total = r.criteria.len();
        r.met = r.criteria.iter().filter(|c| c.met).count();
        r.ready = r.criteria.iter().filter(|c| c.phase == "entry").all(|c| c.met);
    }
    out
}

/// Default evidence for Iris-like projects (nothing built yet): heritage by domain.
pub fn default_evidence(p: &Project) -> BTreeMap<String, Evidence> {
    p.subsystems
        .iter()
        .map(|s| {
            (
                s.id.clone(),
                Evidence {
                    heritage: suggest_heritage(p, &s.id),
                    ..Default::default()
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_readiness_from_evidence() {
        let p = Project::iris();
        let cfg = Config::default();
        let s = assess(&p, &cfg, None);
        assert_eq!(s.len(), p.subsystems.len());
        let get = |id: &str| s.iter().find(|x| x.id == id).unwrap();
        // analysis-only, nothing built: new subsystems sit at TRL 3, COTS electronics at 4
        assert_eq!(get("SYS-OPT").trl, 3);
        assert_eq!(get("SYS-CMP").trl, 4);
        assert_eq!(get("SYS-PCB").heritage, "new");
        assert!(get("SYS-PCB").mrl >= 4, "PCB has DFM requirements");
        assert_eq!(s.iter().map(|x| x.trl).min(), Some(3));
        // building an EVT prototype is not enough for TRL 6 without a test-verified requirement
        let mut c2 = cfg.clone();
        c2.readiness.insert(
            "SYS-OPT".into(),
            Evidence {
                stage: "evt".into(),
                ..Default::default()
            },
        );
        let o = assess(&p, &c2, None).into_iter().find(|x| x.id == "SYS-OPT").unwrap();
        assert_eq!(o.trl, 5);
        assert_eq!(o.mrl, 6);
        let mut p2 = p.clone();
        for r in p2
            .requirements
            .iter_mut()
            .filter(|r| r.allocated_to.contains(&"SYS-OPT".to_string()))
            .take(1)
        {
            r.verification.method = VerifyMethod::Test;
        }
        let o = assess(&p2, &c2, None).into_iter().find(|x| x.id == "SYS-OPT").unwrap();
        assert_eq!(o.trl, 6);
    }
}
