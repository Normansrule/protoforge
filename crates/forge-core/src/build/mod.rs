//! # Build Guide engine — ideal settings, calculators, where to send each
//! part, and the certification path from prototype to product
//!
//! | File | What |
//! |---|---|
//! | [`library`] | Process settings library: FDM per filament, resin, powder bed, laser, CNC, sheet metal, electronics assembly, PCB capability tiers, injection-molding windows — **typical starting points, tune on your machine** — with design rules, common failures and settings-sheet export |
//! | [`machining`] | Feeds and speeds: RPM from surface speed, feed from chip load, radial chip thinning, material-removal rate and cutting power from specific cutting energy |
//! | [`sheet`] | Bend allowance / deduction with the K-factor (DIN 6935 estimate) |
//! | [`reflow`] | Reflow profile windows (SAC305, Sn63Pb37), a lumped oven simulation with pass/fail checks, IPC-7525 stencil area ratios for every Iris pad |
//! | [`laser`] | Laser power × speed test-grid SVG and kerf strip |
//! | [`pcbcap`] | PCB capability tiers and a DFM checker on the routed Iris layout |
//! | [`wizard`] | Transparent weighted process-selection model |
//! | [`vendors`] | The Manufacturing Hub directory extended: categories, accepted formats, which Protoforge file to upload |
//! | [`compliance`] | Certification planner traced to components/interfaces, with a timeline |
//! | [`roadmap`] | Proof of concept → EVT → DVT → PVT → mass production build recipe |
//!
//! Metrics: `build.compliance_items` (count of applicable compliance items
//! for the project's markets, `design.extra.markets`, default US + EU),
//! `build.pcb_capability_violations` (DFM violations against the tier in
//! `design.extra.pcb_tier`, default "standard"), and the informational
//! `build.stencil_min_area_ratio` (smallest IPC-7525 area ratio at the
//! stencil thickness in `design.extra.stencil_thickness_mm`, default 0.12 mm).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, to_value, ApiResult};
use crate::project::{Metric, Project};

pub mod compliance;
pub mod laser;
pub mod library;
pub mod machining;
pub mod pcbcap;
pub mod reflow;
pub mod roadmap;
pub mod sheet;
pub mod vendors;
pub mod wizard;

/// References shown on the page and in the guide.
pub const REFERENCES: &[(&str, &str)] = &[
    (
        "E. Oberg, F. D. Jones, H. L. Horton, H. H. Ryffel, Machinery's Handbook (Industrial Press)",
        "",
    ),
    (
        "S. Kalpakjian, S. R. Schmid, Manufacturing Engineering and Technology (Pearson) — specific cutting energy",
        "",
    ),
    (
        "Sandvik Coromant / Harvey Tool published milling guidance (chip thinning, speeds and feeds)",
        "",
    ),
    ("IPC-7525 — Stencil design guidelines", ""),
    (
        "IPC/JEDEC J-STD-020 — Moisture/reflow sensitivity classification for non-hermetic SMDs",
        "",
    ),
    ("IPC-A-610 — Acceptability of electronic assemblies", ""),
    ("IPC-2221 — Generic standard on printed board design", ""),
    ("ISO/ASTM 52900 — Additive manufacturing: general principles and vocabulary", ""),
    (
        "I. Gibson, D. Rosen, B. Stucker, Additive Manufacturing Technologies (3rd ed., Springer, 2021)",
        "",
    ),
    ("D. V. Rosato, M. G. Rosato, Injection Molding Handbook (3rd ed., 2000)", ""),
    ("DIN 6935 — Cold bending of flat rolled steel products", ""),
    (
        "G. Boothroyd, P. Dewhurst, W. Knight, Product Design for Manufacture and Assembly (3rd ed., 2010)",
        "",
    ),
    (
        "FCC 47 CFR Part 15 — Radio frequency devices",
        "https://www.ecfr.gov/current/title-47/chapter-I/subchapter-A/part-15",
    ),
    (
        "EU Radio Equipment Directive 2014/53/EU",
        "https://eur-lex.europa.eu/eli/dir/2014/53/oj",
    ),
    ("EU RoHS Directive 2011/65/EU", "https://eur-lex.europa.eu/eli/dir/2011/65/oj"),
    ("UN Manual of Tests and Criteria, section 38.3 (lithium batteries)", ""),
    (
        "IEC 62368-1 — Audio/video, information and communication technology equipment: safety",
        "",
    ),
    ("IEC 62133-2 — Safety of portable sealed secondary lithium cells and batteries", ""),
    ("IEC 62471 — Photobiological safety of lamps and lamp systems", ""),
    ("ISO 12870 — Ophthalmic optics: spectacle frames, requirements and test methods", ""),
    ("Prusa Knowledge Base", "https://help.prusa3d.com"),
    ("Bambu Lab Wiki", "https://wiki.bambulab.com"),
];

#[derive(Deserialize, Default)]
#[serde(default)]
struct ProjIn {
    project: Option<Project>,
}

#[derive(Deserialize)]
struct SheetIn {
    #[serde(default)]
    project: Option<Project>,
    process: String,
    #[serde(default)]
    material: Option<String>,
    #[serde(default)]
    format: Option<String>,
}

#[derive(Serialize)]
struct FileOut {
    filename: String,
    content: String,
}

#[derive(Deserialize)]
#[serde(default)]
struct KerfIn {
    nominal_mm: f64,
    measured_mm: f64,
    slices: usize,
}

impl Default for KerfIn {
    fn default() -> Self {
        KerfIn {
            nominal_mm: 50.0,
            measured_mm: 48.5,
            slices: 10,
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct StencilCheckIn {
    project: Option<Project>,
    thickness_mm: Option<f64>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct DfmIn {
    project: Option<Project>,
    tier: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ComplianceIn {
    project: Option<Project>,
    options: Option<compliance::Options>,
}

fn proj(p: Option<Project>) -> Project {
    p.unwrap_or_else(Project::iris)
}

fn opts(p: &Project, o: Option<compliance::Options>) -> compliance::Options {
    o.unwrap_or_else(|| compliance::project_options(p))
}

/// JSON API entry point for `build.*` methods.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    // Methods whose whole input is optional accept `null`.
    let obj = if input.is_null() {
        Value::Object(Default::default())
    } else {
        input.clone()
    };
    let input = &obj;
    match method {
        "build.library" => handle(input, |i: ProjIn| Ok(library::library(&proj(i.project)))),
        "build.settings_sheet" => handle(input, |i: SheetIn| {
            let p = proj(i.project);
            let csv = i.format.as_deref() == Some("csv");
            library::sheet_text(&p, &i.process, i.material.as_deref(), csv).map(|(filename, content)| FileOut { filename, content })
        }),
        "build.laser_materials" => Some(to_value(library::LASER_MATERIALS)),
        "build.laser_grid" => handle(input, |i: laser::GridIn| laser::grid(&i)),
        "build.kerf" => handle(input, |i: KerfIn| {
            let k = laser::kerf_from_strip(i.nominal_mm, i.measured_mm, i.slices);
            Ok(serde_json::json!({ "kerf_mm": k, "offset_mm": k / 2.0 }))
        }),
        "build.machining_materials" => Some(to_value(machining::MATERIALS)),
        "build.feeds_speeds" => handle(input, |i: machining::FeedsIn| machining::feeds_speeds(&i)),
        "build.sheet_materials" => Some(to_value(sheet::SHEET_MATERIALS)),
        "build.bend" => handle(input, |i: sheet::BendIn| sheet::bend(&i)),
        "build.reflow_alloys" => Some(to_value([reflow::Alloy::sac305(), reflow::Alloy::snpb()])),
        "build.reflow" => handle(input, |i: reflow::ReflowIn| reflow::simulate(&i)),
        "build.stencil" => handle(input, |i: reflow::StencilIn| reflow::stencil(&i)),
        "build.stencil_check" => handle(input, |i: StencilCheckIn| {
            let p = proj(i.project);
            let t = i.thickness_mm.unwrap_or_else(|| stencil_thickness(&p));
            Ok(reflow::stencil_check(&p, t))
        }),
        "build.pcb_tiers" => Some(to_value(pcbcap::TIERS)),
        "build.pcb_dfm" => handle(input, |i: DfmIn| {
            let p = proj(i.project);
            let t = i.tier.unwrap_or_else(|| pcbcap::project_tier(&p).to_string());
            Ok(pcbcap::check(&p, &t))
        }),
        "build.wizard" => handle(input, |i: wizard::WizardIn| Ok(wizard::rank(&i))),
        "build.wizard_processes" => Some(to_value(wizard::PROCESSES)),
        "build.vendors" => Some(to_value(
            serde_json::json!({ "categories": vendors::CATEGORIES, "vendors": vendors::directory() }),
        )),
        "build.compliance" => handle(input, |i: ComplianceIn| {
            let p = proj(i.project);
            let o = opts(&p, i.options);
            Ok(compliance::plan(&p, &o))
        }),
        "build.compliance_md" => handle(input, |i: ComplianceIn| {
            let p = proj(i.project);
            let o = opts(&p, i.options);
            let plan = compliance::plan(&p, &o);
            let name = if p.meta.codename.is_empty() {
                p.meta.id.clone()
            } else {
                p.meta.codename.to_lowercase()
            };
            Ok(FileOut {
                filename: format!("{name}-compliance-plan.md"),
                content: compliance::markdown(&p, &plan),
            })
        }),
        "build.roadmap" => handle(input, |i: ProjIn| Ok(roadmap::roadmap(&proj(i.project)))),
        "build.references" => Some(to_value(
            REFERENCES
                .iter()
                .map(|(l, u)| serde_json::json!({ "label": l, "href": u }))
                .collect::<Vec<_>>(),
        )),
        _ => None,
    }
}

/// Stencil thickness for the metric: `design.extra.stencil_thickness_mm`, default 0.12 mm.
pub fn stencil_thickness(p: &Project) -> f64 {
    p.design
        .extra
        .get("stencil_thickness_mm")
        .and_then(|v| v.as_f64())
        .filter(|t| *t > 0.0)
        .unwrap_or(0.12)
}

/// Metrics this module contributes to requirement verification.
pub fn metrics(p: &Project) -> Vec<Metric> {
    let mut v = Vec::new();
    let o = compliance::project_options(p);
    let plan = compliance::plan(p, &o);
    v.push(
        Metric::new(
            "build.compliance_items",
            "Compliance items to plan",
            plan.items.len() as f64,
            "count",
            "build",
        )
        .with_note(&format!(
            "{} markets; {} mandatory; indicative ${:.0}k–${:.0}k (rough order of magnitude)",
            plan.markets.join(" + "),
            plan.mandatory,
            plan.total_cost_usd[0] / 1000.0,
            plan.total_cost_usd[1] / 1000.0
        )),
    );
    let tier = pcbcap::project_tier(p);
    let m = pcbcap::measure(p);
    let d = pcbcap::check_with(p, &m, pcbcap::tier(tier));
    let fails: Vec<String> = d.items.iter().filter(|i| i.status == "fail").map(|i| i.rule.clone()).collect();
    v.push(
        Metric::new(
            "build.pcb_capability_violations",
            "PCB rules beyond the fab tier",
            d.violations as f64,
            "count",
            "build",
        )
        .with_note(&if fails.is_empty() {
            format!("{} tier; {} warning(s)", d.tier.name, d.warnings)
        } else {
            format!("{} tier: {}", d.tier.name, fails.join(", "))
        }),
    );
    if p.components.iter().any(|c| c.on_pcb) {
        let t = stencil_thickness(p);
        let s = reflow::stencil_check(p, t);
        if s.min_area_ratio.is_finite() {
            v.push(
                Metric::new(
                    "build.stencil_min_area_ratio",
                    "Smallest stencil area ratio",
                    s.min_area_ratio,
                    "ratio",
                    "build",
                )
                .with_note(&format!("{:.2} mm stencil, worst: {} (IPC-7525 wants ≥ 0.66)", t, s.worst)),
            );
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_metrics() {
        let m = metrics(&Project::iris());
        let get = |k: &str| m.iter().find(|x| x.key == k).unwrap_or_else(|| panic!("missing {k}"));
        let n = get("build.compliance_items").value;
        assert!((15.0..=30.0).contains(&n), "{n}");
        assert_eq!(get("build.pcb_capability_violations").value, 0.0);
        let ar = get("build.stencil_min_area_ratio").value;
        assert!(ar > 0.5 && ar < 1.0, "{ar}");
        for x in &m {
            eprintln!("{} = {} {} ({})", x.key, x.value, x.unit, x.note);
        }
    }

    #[test]
    fn dispatch_methods() {
        for (m, input) in [
            ("build.library", Value::Null),
            ("build.settings_sheet", serde_json::json!({"process": "cnc", "format": "csv"})),
            ("build.laser_grid", Value::Null),
            ("build.kerf", Value::Null),
            ("build.feeds_speeds", Value::Null),
            ("build.bend", Value::Null),
            ("build.reflow", Value::Null),
            ("build.stencil", Value::Null),
            ("build.stencil_check", Value::Null),
            ("build.pcb_dfm", serde_json::json!({"tier": "basic"})),
            ("build.wizard", Value::Null),
            ("build.vendors", Value::Null),
            ("build.compliance", serde_json::json!({"options": {"markets": ["US"]}})),
            ("build.compliance_md", Value::Null),
            ("build.roadmap", Value::Null),
            ("build.references", Value::Null),
        ] {
            let r = dispatch(m, &input).unwrap();
            assert!(r.is_ok(), "{m}: {r:?}");
        }
        assert!(dispatch("build.nope", &Value::Null).is_none());
        let k = dispatch("build.kerf", &Value::Null).unwrap().unwrap();
        assert!((k["kerf_mm"].as_f64().unwrap() - 0.15).abs() < 1e-12);
    }
}
