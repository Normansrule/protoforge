//! Prototype-to-product roadmap: what to build at each validation stage,
//! how many, with which processes and vendors, what to test, and the exit
//! criteria — tied to the project's milestones, compliance plan and FMEA
//! risks.
//!
//! The stage names follow common hardware practice: engineering validation
//! test (EVT), design validation test (DVT), production validation test
//! (PVT), then mass production (MP). The Manufacturing Hub
//! (`crate::mfg::stages`) keeps the checkbox list; this roadmap adds the
//! build recipe.

use serde::Serialize;

use crate::project::Project;

#[derive(Clone, Debug, Serialize)]
pub struct Build {
    pub what: String,
    pub process: &'static str,
    pub vendor_category: &'static str,
    pub route: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct Stage {
    pub id: &'static str,
    pub title: &'static str,
    pub question: &'static str,
    pub quantity: String,
    pub builds: Vec<Build>,
    pub tests: Vec<String>,
    pub exit: Vec<String>,
    pub milestones: Vec<String>,
    pub compliance: Vec<String>,
    pub risks: Vec<String>,
    pub typical_weeks: [f64; 2],
}

fn b(what: impl Into<String>, process: &'static str, vendor_category: &'static str, route: &'static str) -> Build {
    Build {
        what: what.into(),
        process,
        vendor_category,
        route,
    }
}

/// Which roadmap stage a milestone belongs to (by its stage name and title).
pub fn stage_of(stage: &str, title: &str) -> &'static str {
    let s = format!("{stage} {title}").to_ascii_lowercase();
    if s.contains("pvt") || s.contains("production validation") {
        "pvt"
    } else if s.contains("dvt") || s.contains("design validation") {
        "dvt"
    } else if s.contains("evt") || s.contains("engineering validation") {
        "evt"
    } else if s.contains("mass") || s.contains("launch") || s.split_whitespace().any(|w| w == "mp") {
        "mp"
    } else {
        "proto"
    }
}

/// Milestones (and their exit criteria) mapped to a stage id.
pub fn milestones_for(p: &Project, stage: &str) -> (Vec<String>, Vec<String>) {
    let ms: Vec<_> = p.milestones.iter().filter(|m| stage_of(&m.stage, &m.title) == stage).collect();
    (
        ms.iter().map(|m| format!("{} — {}", m.id, m.title)).collect(),
        ms.iter().flat_map(|m| m.exit_criteria.iter().cloned()).collect(),
    )
}

pub fn roadmap(p: &Project) -> Vec<Stage> {
    let plan = crate::build::compliance::plan(p, &crate::build::compliance::project_options(p));
    let comp = |stage: &str| -> Vec<String> { plan.items.iter().filter(|i| i.stage == stage).map(|i| i.title.clone()).collect() };
    let risks = |keys: &[&str]| -> Vec<String> {
        p.risks
            .iter()
            .filter(|r| has(&format!("{} {}", r.item, r.failure_mode), keys))
            .map(|r| {
                format!(
                    "{} {} — {} (S{}·O{}·D{})",
                    r.id, r.item, r.failure_mode, r.severity, r.occurrence, r.detection
                )
            })
            .collect()
    };
    let q = p.design.manufacturing.target_quantity;
    let mat = &p.design.frame.material;
    let mold = &p.design.manufacturing.molding_material;
    let layers = p.design.pcb.layers;

    let (m0, e0) = milestones_for(p, "proto");
    let (m1, e1) = milestones_for(p, "evt");
    let (m2, e2) = milestones_for(p, "dvt");
    let (m3, e3) = milestones_for(p, "pvt");
    let (m4, e4) = milestones_for(p, "mp");
    let with = |mut v: Vec<String>, extra: &[&str]| {
        for x in extra {
            v.push((*x).to_string());
        }
        v
    };
    vec![
        Stage {
            id: "proto",
            title: "Proof of concept",
            question: "Can it work at all?",
            quantity: "1–3 benches".into(),
            builds: vec![
                b(
                    "Development boards + breadboard wiring",
                    "Off-the-shelf dev kits",
                    "parts",
                    "/proto",
                ),
                b(format!("Looks-like frame in {mat}"), "FDM 3D printing", "print", "/print"),
                b("Optics mock-up on a breadboard", "Catalogue optics", "optics", "/optics"),
            ],
            tests: vec![
                "Simulations green in every lab".into(),
                "Firmware on dev kits".into(),
                "Wear the printed frame for comfort".into(),
            ],
            exit: with(e0, &["Architecture and top trades decided"]),
            milestones: m0,
            compliance: vec![
                "Pick radio module vs chip-down (drives certification cost)".into(),
                "Choose a cell with UN 38.3 + IEC 62133-2 documents".into(),
            ],
            risks: risks(&["supply"]),
            typical_weeks: [4.0, 12.0],
        },
        Stage {
            id: "evt",
            title: "EVT — engineering validation",
            question: "Does the design meet its requirements?",
            quantity: "5–20 units".into(),
            builds: vec![
                b(
                    format!("Main PCB, {layers}-layer, assembled (PCBA)"),
                    "PCB fab + SMT assembly",
                    "pcba",
                    "/pcb",
                ),
                b(format!("Frames in {mat} (FDM) or PA12 (SLS/MJF)"), "3D printing", "print", "/print"),
                b(
                    "Optics module from catalogue lenses + printed mount",
                    "SLA + catalogue optics",
                    "optics",
                    "/optics",
                ),
            ],
            tests: vec![
                "Board bring-up: rails, current, radio link".into(),
                "Power and battery life vs requirement".into(),
                "Thermal: skin temperature at 30 °C ambient".into(),
                "Optics: field of view, eyebox, image distance".into(),
            ],
            exit: with(e1, &["Every analysis requirement measured on hardware"]),
            milestones: m1,
            compliance: comp("EVT"),
            risks: risks(&["thermal", "hot", "misalign", "ble", "link"]),
            typical_weeks: [6.0, 10.0],
        },
        Stage {
            id: "dvt",
            title: "DVT — design validation",
            question: "Does it survive the real world?",
            quantity: "20–100 units".into(),
            builds: vec![
                b(
                    format!("Frames in production material ({mold}) from soft (aluminium) tooling or urethane casting"),
                    "Urethane casting / bridge tooling",
                    "urethane",
                    "/molding",
                ),
                b("PCBA from the production-intent layout", "PCB fab + SMT assembly", "pcba", "/pcb"),
                b("Hinge flex cables from the cable supplier", "Flex circuit", "cable", "/systems"),
            ],
            tests: vec![
                "Drop, hinge-fold cycling, sweat and temperature cycling".into(),
                "Pre-compliance EMC / radio scan".into(),
                "Battery abuse checks with the cell supplier's data".into(),
                "Frame tests per ISO 12870 (bridge deformation, hinge endurance)".into(),
            ],
            exit: with(e2, &["Design frozen", "Pre-compliance margins acceptable"]),
            milestones: m2,
            compliance: comp("DVT"),
            risks: risks(&["crack", "fatigue", "swell", "runaway", "port", "sweat"]),
            typical_weeks: [8.0, 14.0],
        },
        Stage {
            id: "pvt",
            title: "PVT — production validation",
            question: "Can the factory build it repeatably?",
            quantity: format!("{}–{} units", 100.0f64.min(q).round(), (q * 0.1).clamp(100.0, 1000.0).round()),
            builds: vec![
                b(
                    format!("Hardened steel mold ({}), T1 → T2 samples", p.design.manufacturing.mold_steel),
                    "Injection molding",
                    "molding",
                    "/molding",
                ),
                b("Line fixtures, test jigs, work instructions", "Contract manufacturer", "cm", "/mfg"),
                b("Production PCBA with in-circuit / functional test", "PCB assembly", "pcba", "/pcb"),
            ],
            tests: vec![
                "Formal certification testing on production units".into(),
                "Yield, cycle time and test coverage on the line".into(),
                "Dimensional reports (first article inspection)".into(),
            ],
            exit: with(e3, &["Certifications granted", "Yield target met"]),
            milestones: m3,
            compliance: comp("PVT"),
            risks: risks(&["supply", "end-of-life"]),
            typical_weeks: [8.0, 12.0],
        },
        Stage {
            id: "mp",
            title: "Mass production",
            question: "Ship at volume",
            quantity: format!("{q:.0}+ units"),
            builds: vec![b("Ramp with the contract manufacturer", "Production line", "cm", "/mfg")],
            tests: vec![
                "Incoming inspection and statistical process control".into(),
                "Ongoing reliability testing".into(),
                "Field-return analysis".into(),
            ],
            exit: with(e4, &["Sustaining engineering loop running"]),
            milestones: m4,
            compliance: vec!["Keep the technical file, declarations and producer registrations current".into()],
            risks: vec![],
            typical_weeks: [0.0, 0.0],
        },
    ]
}

fn has(s: &str, keys: &[&str]) -> bool {
    let l = s.to_ascii_lowercase();
    keys.iter().any(|k| l.contains(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_roadmap_links_milestones() {
        let r = roadmap(&Project::iris());
        assert_eq!(r.len(), 5);
        let evt = &r[1];
        assert!(evt.milestones.iter().any(|m| m.starts_with("M-3")), "{:?}", evt.milestones);
        assert!(r[2].milestones.iter().any(|m| m.starts_with("M-4")));
        assert!(r[3].milestones.iter().any(|m| m.starts_with("M-5")));
        assert!(r[3].compliance.iter().any(|c| c.contains("FCC")));
        assert!(r[2].risks.iter().any(|x| x.starts_with("F-7")));
        assert!(r[3].quantity.contains("100"));
        assert!(r[0].milestones.iter().any(|m| m.starts_with("M-2")) && !r[2].milestones.iter().any(|m| m.starts_with("M-3")));
    }
}
