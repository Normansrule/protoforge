//! Design verification test (DVT) plan: test legs with sample sizes from the
//! reliability/confidence target, environmental test families, a requirement
//! verification matrix, a resource-constrained schedule, and CSV/Markdown
//! exports.
//!
//! Legs (units are not reused between destructive legs):
//! * **Baseline** — every unit gets the functional and performance checks first.
//! * **A · Mechanical robustness** (attribute, pass/fail): success-run or
//!   binomial sample, `n = ln(1 − C)/ln R` for zero failures.
//! * **B · Climatic life** and **C · Wear-out life**: Lipson/Weibayes sample
//!   for a test of k field lives with an assumed β.
//! * **D · Qualification checks** (ingress, electrostatic discharge, sweat):
//!   a small fixed sample — a check against a severity, not a reliability
//!   demonstration.
//! * **E · Cell cycle life** on bare cells (not product units).
//!
//! Standards are cited by designation only (IEC 60068-2 series, IEC 60529,
//! IEC 61000-4-2, IEC 62133-2, UN 38.3, ISO 12870); severities are typical
//! choices for a wearable — **check the current edition and your product
//! standard before testing**.

use serde::{Deserialize, Serialize};

use super::iris::{Config, UseProfile};
use super::{alt, sample};
use crate::project::{Cmp, Project, VerifyMethod};

#[derive(Serialize, Clone, Debug, Default)]
pub struct Test {
    pub id: String,
    pub leg: String,
    pub name: String,
    pub standard: String,
    pub severity: String,
    pub procedure: Vec<String>,
    pub equipment: String,
    pub resource: String,
    pub units: u32,
    pub pass: String,
    pub linked: Vec<String>,
    pub duration_d: f64,
    pub start_d: f64,
    pub end_d: f64,
    /// Extra facts (acceleration factor, field equivalence …).
    pub basis: String,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Leg {
    pub id: String,
    pub name: String,
    pub sample_basis: String,
    pub units: u32,
    /// Product units (false for bare-cell legs).
    pub product_units: bool,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct MatrixRow {
    pub id: String,
    pub text: String,
    pub kind: String,
    pub method: String,
    pub dvt: String,
    pub equipment: String,
    pub units: String,
    pub pass: String,
    pub tests: Vec<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct DvtOut {
    pub legs: Vec<Leg>,
    pub tests: Vec<Test>,
    pub matrix: Vec<MatrixRow>,
    pub total_units: u32,
    /// Product legs, days.
    pub duration_d: f64,
    /// Bare-cell cycling, days (runs in parallel from EVT).
    pub cell_duration_d: f64,
    pub resources: Vec<String>,
    pub reliability: f64,
    pub confidence: f64,
    pub life_multiple: f64,
    pub life_beta: f64,
    pub csv: String,
    pub markdown: String,
    pub notes: Vec<String>,
}

fn crit(cmp: Cmp, target: f64, hi: Option<f64>, unit: &str) -> String {
    let u = if unit.is_empty() { String::new() } else { format!(" {unit}") };
    match cmp {
        Cmp::Le => format!("≤ {target}{u}"),
        Cmp::Ge => format!("≥ {target}{u}"),
        Cmp::Eq => format!("= {target} ± {}{u}", hi.unwrap_or(0.0)),
        Cmp::Between => format!("{target} to {}{u}", hi.unwrap_or(target)),
    }
}

/// How a requirement's metric is confirmed physically in DVT: (activity, equipment, linked test ids).
fn dvt_for(metric: &str) -> (&'static str, &'static str, Vec<&'static str>) {
    let m = metric;
    if m.starts_with("mass.") {
        ("Weigh complete units", "Balance, 0.01 g resolution", vec!["BL"])
    } else if m.starts_with("power.") {
        (
            "Run the reference use-profile script from full charge to cut-off at 25 °C and log battery power",
            "Source-measure unit or power analyser, scripted phone app",
            vec!["BL"],
        )
    } else if m.starts_with("thermal.") {
        (
            "Display on continuously in a 30 °C chamber; thermocouples on skin-contact surfaces until steady",
            "Climatic chamber, thermocouples, data logger",
            vec!["B3"],
        )
    } else if m == "optics.combiner_clearance_mm" {
        (
            "Inspect the optical assembly (section or X-ray one unit)",
            "Microscope, CT/X-ray or sectioning",
            vec!["BL"],
        )
    } else if m.starts_with("optics.") || m == "structures.focus_error_worst_d" {
        ("Optical bench: calibrated camera at the eye position (field of view, pixels per degree, image distance by refocus, eyebox by translation, spot and colour by slanted edge)", "Optical bench, machine-vision camera with calibrated lens, translation stage", vec!["BL"])
    } else if m.starts_with("dsp.") {
        (
            "Acoustic measurement: speaker on a turntable in a quiet room, tones and speech; decimation checked with a PDM test-tone sweep",
            "Turntable, reference microphone, audio interface",
            vec!["BL"],
        )
    } else if m.starts_with("control.") {
        (
            "Head-motion rig: glasses on a motorised gimbal against a reference encoder",
            "Gimbal or turntable with encoder",
            vec!["BL"],
        )
    } else if m.starts_with("circuit.") {
        (
            "Probe rails and sleep currents on the assembled board",
            "Oscilloscope (20 MHz limit for ripple), source-measure unit",
            vec!["BL"],
        )
    } else if m.starts_with("chip.") {
        (
            "Analysis (static timing and synthesis report); DVT runs the silicon or its FPGA prototype at speed",
            "FPGA board or packaged silicon, logic analyser",
            vec![],
        )
    } else if m.starts_with("pcb.") || m.starts_with("build.") {
        ("Inspection of fabrication data (design-rule and fab capability reports) and first-article inspection of bare and assembled boards", "Fab reports, microscope, X-ray for fine-pitch joints", vec![])
    } else if m.starts_with("fit.") || m.starts_with("structures.pod") || m.starts_with("molding.") {
        (
            "First-article inspection of frames, pods and boards; trial assembly",
            "Calipers, optical CMM, pin gauges",
            vec!["BL"],
        )
    } else if m.starts_with("structures.") {
        (
            "Frame load test: temple spread and bridge bend at the specified force, then inspect",
            "Force gauge on a test stand, fixtures",
            vec!["A4"],
        )
    } else if m.starts_with("print.") {
        (
            "Demonstrate: print a frame set and time it",
            "The printer, stopwatch or printer log",
            vec![],
        )
    } else if m.starts_with("mfg.") {
        ("Analysis against supplier quotes at the target quantity", "Quotes", vec![])
    } else if m.starts_with("robotics.") {
        ("Demonstrate on the assembly line (time study)", "Line, stopwatch", vec![])
    } else if m.starts_with("firmware.") {
        (
            "On-target measurement (linker map for memory; timestamps on GPIO or a high-speed camera for latency)",
            "Debugger, logic analyser",
            vec!["BL"],
        )
    } else if m.starts_with("reliability.") {
        (
            "Life legs of this plan (B, C, E) plus field-return tracking",
            "Chambers, cyclers",
            vec!["B1", "C1", "E1"],
        )
    } else if m.starts_with("rl.") {
        ("Analysis now; confirm with field logs", "Firmware logging", vec![])
    } else {
        ("Per the requirement's verification method", "—", vec![])
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(default)]
pub struct DvtIn {
    pub project: Option<Project>,
    pub reliability: Option<f64>,
    pub confidence: Option<f64>,
    pub failures: Option<u32>,
    pub life_multiple: Option<f64>,
    pub life_beta: Option<f64>,
    pub qual_units: Option<u32>,
}

#[allow(clippy::too_many_arguments)]
fn test(
    id: &str,
    leg: &str,
    name: &str,
    standard: &str,
    severity: &str,
    procedure: &[&str],
    equipment: &str,
    resource: &str,
    pass: &str,
    linked: &[&str],
    duration_d: f64,
    basis: String,
) -> Test {
    Test {
        id: id.into(),
        leg: leg.into(),
        name: name.into(),
        standard: standard.into(),
        severity: severity.into(),
        procedure: procedure.iter().map(|s| s.to_string()).collect(),
        equipment: equipment.into(),
        resource: resource.into(),
        pass: pass.into(),
        linked: linked.iter().map(|s| s.to_string()).collect(),
        duration_d,
        basis,
        ..Default::default()
    }
}

/// Build the plan for the project at the configured (or overridden) targets.
pub fn plan(p: &Project, cfg: &Config, u: &UseProfile, i: &DvtIn) -> DvtOut {
    let r = i.reliability.unwrap_or(cfg.dvt_reliability).clamp(0.5, 0.9999);
    let c = i.confidence.unwrap_or(cfg.dvt_confidence).clamp(0.5, 0.999);
    let fails = i.failures.unwrap_or(cfg.dvt_failures).min(5);
    let k = i.life_multiple.unwrap_or(cfg.life_multiple).clamp(0.25, 10.0);
    let beta = i.life_beta.unwrap_or(cfg.life_beta).clamp(0.3, 10.0);
    let q = i.qual_units.unwrap_or(cfg.qual_units).clamp(1, 50);
    let n_attr = sample::binomial_n(r, c, fails);
    let n_life = sample::lipson_n(r, c, beta, k, fails);
    let legs = vec![
        Leg {
            id: "BL".into(),
            name: "Baseline (all units)".into(),
            sample_basis: "every unit, non-destructive".into(),
            units: 0,
            product_units: true,
        },
        Leg {
            id: "A".into(),
            name: "Mechanical robustness".into(),
            sample_basis: if fails == 0 {
                format!("success run: ln(1 − {c}) / ln {r} → {n_attr}")
            } else {
                format!("binomial, ≤ {fails} failure(s) → {n_attr}")
            },
            units: n_attr,
            product_units: true,
        },
        Leg {
            id: "B".into(),
            name: "Climatic life".into(),
            sample_basis: format!("Lipson: {k} lives, β = {beta} → {n_life}"),
            units: n_life,
            product_units: true,
        },
        Leg {
            id: "C".into(),
            name: "Wear-out life".into(),
            sample_basis: format!("Lipson: {k} lives, β = {beta} → {n_life}"),
            units: n_life,
            product_units: true,
        },
        Leg {
            id: "D".into(),
            name: "Qualification checks".into(),
            sample_basis: format!("fixed {q} units (check against a severity, not a demonstration)"),
            units: q,
            product_units: true,
        },
        Leg {
            id: "E".into(),
            name: "Cell cycle life (bare cells)".into(),
            sample_basis: format!("Lipson: {k} lives, β = {beta} → {n_life} cells"),
            units: n_life,
            product_units: false,
        },
    ];
    let total: u32 = legs.iter().filter(|l| l.product_units).map(|l| l.units).sum();
    let bench = |units: u32, h_per_unit: f64| (units as f64 * h_per_unit / 8.0).max(0.5);

    // B1 thermal cycling: Norris–Landzberg (tin–silver–copper constants) from the daily swing.
    let prof = alt::profile(alt::ProfileIn::default()).unwrap_or_default();
    let af_tc = alt::norris_landzberg(
        u.daily_swing_c.max(1.0),
        80.0,
        2.65,
        1.0,
        prof.cycles_per_day.max(1.0),
        0.136,
        2185.0,
        u.board_c,
        60.0,
    );
    let field_cycles = u.mission_days;
    let tc_cycles = (field_cycles * k / af_tc).ceil().max(10.0);
    let tc_days = tc_cycles * prof.cycle_min / 1440.0;
    // B2 damp heat: 40 °C / 93 % RH for 10 days, Peck equivalence against the worn condition.
    let af_dh = alt::peck(0.79, 2.66, u.ambient_c, u.use_rh_pct.clamp(5.0, 99.0), 40.0, 93.0);
    let dh_field_h = 240.0 * af_dh;
    // C1 hinge cycling at 20 cycles/min; C2 USB-C mating at 6 per minute.
    let hinge_cycles = (u.hinge_cycles_per_day * u.mission_days * k).ceil();
    let usb_cycles = (u.charge_cycles_per_day * u.mission_days * k).ceil();
    // E1 cell cycling: ~2.5 h per charge/discharge cycle at about 0.5–1 C.
    let cell_cycles = (u.charge_cycles_per_day * u.mission_days * k).ceil();

    let mut tests = vec![
        test(
            "BL",
            "BL",
            "Baseline functional and performance check",
            "—",
            "all verifiable requirements",
            &["Serialise units; photograph and weigh", "Run the functional script (display, touch, IMU, microphones, Bluetooth Low Energy link, charging)", "Measure the performance requirements listed in the matrix on a subset of 5 units"],
            "Functional test fixture, bench instruments",
            "Functional bench",
            "all requirements in the matrix met",
            &[],
            bench(total, 0.5),
            format!("{total} units × 0.5 h"),
        ),
        test(
            "A1",
            "A",
            "Random vibration (transport)",
            "IEC 60068-2-64 (Test Fh)",
            "transport-representative profile, 3 axes, packaged (typical — pick from your distribution standard)",
            &["Mount packaged units on the shaker", "Run the profile on each axis", "Inspect and run the functional script"],
            "Electrodynamic shaker with controller",
            "Shaker",
            "no damage; functional",
            &[],
            1.0,
            String::new(),
        ),
        test(
            "A2",
            "A",
            "Mechanical shock",
            "IEC 60068-2-27 (Test Ea)",
            "half-sine, 3 shocks per direction, 6 directions; peak and duration per product specification",
            &["Fixture unpackaged units", "Apply the shocks", "Functional check"],
            "Shock machine, accelerometer",
            "Shock machine",
            "no damage; functional",
            &[],
            0.5,
            String::new(),
        ),
        test(
            "A3",
            "A",
            "Free-fall drop",
            "IEC 60068-2-31 (Test Ec, rough handling)",
            "1.5 m onto a hard floor, several orientations (the Structures Lab drop estimate uses 1.5 m)",
            &["Drop each unit in each orientation", "Inspect for cracks, hinge and lens retention", "Functional check"],
            "Drop tester, steel or concrete floor plate",
            "Drop tester",
            "no cracks or loose parts; functional",
            &["structures.drop_safety_factor"],
            0.5,
            String::new(),
        ),
        test(
            "A4",
            "A",
            "Frame load test",
            "ISO 12870 (spectacle frames) contains frame test methods — check which apply",
            "5 N temple spread and 10 N bridge bend (the Iris wear load cases)",
            &["Clamp at the hinge / endpieces", "Apply the load with a force gauge, hold, release", "Measure permanent set; functional check"],
            "Force gauge on a test stand",
            "Force stand",
            "no permanent set beyond limit; no cracks",
            &["R-026"],
            0.5,
            String::new(),
        ),
        test(
            "A5",
            "A",
            "Post-leg functional check",
            "—",
            "baseline script",
            &["Repeat the baseline functional script"],
            "Functional test fixture",
            "Functional bench",
            "all units functional",
            &[],
            bench(n_attr, 0.5),
            String::new(),
        ),
        test(
            "B1",
            "B",
            "Change of temperature (thermal cycling)",
            "IEC 60068-2-14 (Test Nb)",
            &format!("−20 ↔ +60 °C, 5 °C/min, 15 min dwells: {tc_cycles:.0} cycles"),
            &["Units unpowered in the chamber", "Cycle; functional check every 25 % of the cycles", "Final inspection of solder joints (microscope, X-ray sample)"],
            "Climatic chamber with ramp control",
            "Climatic chamber",
            "no loss of function; no cracked joints",
            &["PCB1"],
            tc_days,
            format!(
                "Norris–Landzberg (tin–silver–copper constants) AF {af_tc:.0}: {field_cycles:.0} daily {:.0} K swings × {k} lives ÷ AF",
                u.daily_swing_c
            ),
        ),
        test(
            "B2",
            "B",
            "Damp heat, steady state",
            "IEC 60068-2-78 (Test Cab)",
            "40 °C / 93 % RH, 10 days (typical severities 4, 10, 21 or 56 days)",
            &["Units unpowered; recover 1–2 h at lab conditions", "Functional check; inspect for corrosion, haze in optics, delamination"],
            "Climatic chamber with humidity",
            "Climatic chamber",
            "no loss of function; no visible corrosion or haze",
            &["OPT1", "OPT2"],
            10.0 + 0.2,
            format!(
                "Peck AF {af_dh:.1} vs {:.0} °C / {:.0} % RH → ≈ {:.0} worn hours ({:.0} % of the {:.0}-year worn exposure) — a robustness screen, not a full-life demonstration",
                u.ambient_c,
                u.use_rh_pct,
                dh_field_h,
                100.0 * dh_field_h / u.mission_h.max(1.0),
                u.mission_years
            ),
        ),
        test(
            "B3",
            "B",
            "Dry heat, operating (and skin temperature)",
            "IEC 60068-2-2 (Test B)",
            "30 °C then 40 °C, display on continuously, 16 h each",
            &["Thermocouples on the skin-contact surfaces and the board", "Run the display at full duty until steady", "Record skin and board temperatures; check the thermal throttle"],
            "Climatic chamber, thermocouples, data logger",
            "Climatic chamber",
            "skin ≤ 41 °C at 30 °C ambient; no shutdowns",
            &["R-008"],
            1.5,
            String::new(),
        ),
        test(
            "B4",
            "B",
            "Cold, operating",
            "IEC 60068-2-1 (Test A)",
            "−10 °C, 16 h, powered; then charge attempt (the charger must not charge below its limit)",
            &["Soak, then run the functional script at temperature", "Verify the charger's cold inhibit"],
            "Climatic chamber",
            "Climatic chamber",
            "functional; no charging below the cell's limit",
            &["BT1", "U4"],
            1.0,
            String::new(),
        ),
        test(
            "B5",
            "B",
            "Post-leg functional check",
            "—",
            "baseline script",
            &["Repeat the baseline functional script"],
            "Functional test fixture",
            "Functional bench",
            "all units functional",
            &[],
            bench(n_life, 0.5),
            String::new(),
        ),
        test(
            "C1",
            "C",
            "Hinge open/close and flex-cable fold cycling",
            "fixture test (ISO 12870 has frame endurance methods — check)",
            &format!("{hinge_cycles:.0} open/close cycles at 20 per minute, display running"),
            &["Mount units on the hinge cycler", "Monitor the display link continuously (flag any dropout)", "Measure hinge torque every 5 000 cycles"],
            "Hinge cycler with continuity monitor",
            "Hinge cycler",
            "no display dropouts; hinge torque within limits",
            &["HW1", "FPC1", "F-2"],
            hinge_cycles / 20.0 / 60.0 / 24.0,
            format!("{:.0} per day × {:.0} days × {k} lives", u.hinge_cycles_per_day, u.mission_days),
        ),
        test(
            "C2",
            "C",
            "USB-C mating cycles",
            "fixture test",
            &format!("{usb_cycles:.0} insertions at 6 per minute"),
            &["Robotic or manual insertion rig", "Check charging current every 250 cycles"],
            "Insertion rig, USB power meter",
            "USB rig",
            "charges normally; retention force within limits",
            &["J1"],
            usb_cycles / 6.0 / 60.0 / 24.0,
            format!("{:.2} charges per day × {:.0} days × {k} lives", u.charge_cycles_per_day, u.mission_days),
        ),
        test(
            "C3",
            "C",
            "Post-leg functional check",
            "—",
            "baseline script",
            &["Repeat the baseline functional script"],
            "Functional test fixture",
            "Functional bench",
            "all units functional",
            &[],
            bench(n_life, 0.5),
            String::new(),
        ),
        test(
            "D1",
            "D",
            "Ingress: splashing water",
            "IEC 60529 (IPX4)",
            "oscillating tube or spray nozzle per IPX4",
            &["Spray per the standard", "Dry the outside, functional check, open one unit for water"],
            "IPX4 spray apparatus",
            "Spray rig",
            "no harmful water ingress; functional",
            &[],
            0.5,
            String::new(),
        ),
        test(
            "D2",
            "D",
            "Electrostatic discharge",
            "IEC 61000-4-2",
            "±4 kV contact, ±8 kV air (typical consumer levels — check the product standard)",
            &["Discharge to touch surface, hinge, USB shell and seams", "Classify the performance (self-recovering or not)"],
            "ESD simulator, ground plane",
            "ESD bench",
            "no damage; self-recovering",
            &["U7", "J1"],
            1.0,
            String::new(),
        ),
        test(
            "D3",
            "D",
            "Sweat and skin-oil exposure",
            "method per your test specification (no single standard assumed)",
            "artificial sweat soak on skin-contact parts, 48 h at 40 °C",
            &["Apply, soak, rinse", "Inspect coatings, microphone ports, touch function"],
            "Oven, artificial-sweat solution",
            "Oven",
            "no corrosion, discolouration or blocked ports",
            &["MK1", "MK2", "U7"],
            2.5,
            String::new(),
        ),
        test(
            "D4",
            "D",
            "Battery safety and transport",
            "IEC 62133-2 and UN 38.3",
            "certificates for the cell (and pack, if you build one)",
            &["Obtain the test reports from the cell supplier", "Review the protection circuit against them"],
            "Document review",
            "Document review",
            "valid reports for the exact cell",
            &["BT1", "F-1"],
            1.0,
            String::new(),
        ),
        test(
            "E1",
            "E",
            "Cell cycle life (bare cells)",
            "cycling per the cell datasheet method",
            &format!("{cell_cycles:.0} equivalent full cycles, 25 °C, capacity check every 50 cycles"),
            &["Cycle cells on a battery cycler at the datasheet rate", "Fit capacity-fade data in the Weibull tab"],
            "Multi-channel battery cycler",
            "Battery cycler",
            "capacity ≥ 80 % of rated at the end",
            &["BT1"],
            cell_cycles * 2.5 / 24.0,
            format!("{:.2} charges per day × {:.0} days × {k} lives; ≈ 2.5 h per cycle — start in EVT, it sets the schedule", u.charge_cycles_per_day, u.mission_days),
        ),
    ];
    // units per test from its leg
    for t in tests.iter_mut() {
        t.units = match t.leg.as_str() {
            "BL" => total,
            l => legs.iter().find(|x| x.id == l).map(|x| x.units).unwrap_or(0),
        };
        if t.id == "D4" {
            t.units = 0;
        }
        t.duration_d = (t.duration_d * 10.0).ceil() / 10.0;
    }
    // schedule: baseline first, then legs in parallel; each resource runs one
    // test at a time (earliest gap that fits after the leg's previous test)
    let mut busy: std::collections::BTreeMap<String, Vec<(f64, f64)>> = Default::default();
    let mut leg_ready: std::collections::BTreeMap<String, f64> = Default::default();
    let base_end = tests[0].duration_d;
    for (idx, t) in tests.iter_mut().enumerate() {
        let ready = if idx == 0 {
            0.0
        } else {
            *leg_ready.entry(t.leg.clone()).or_insert(if t.leg == "E" { 0.0 } else { base_end })
        };
        let slots = busy.entry(t.resource.clone()).or_default();
        let mut start = ready;
        if t.resource != "Document review" {
            slots.sort_by(|a, b| a.0.total_cmp(&b.0));
            for &(a, b) in slots.iter() {
                if start + t.duration_d <= a + 1e-9 {
                    break;
                }
                if b > start {
                    start = b;
                }
            }
            slots.push((start, start + t.duration_d));
        }
        t.start_d = start;
        t.end_d = start + t.duration_d;
        leg_ready.insert(t.leg.clone(), t.end_d);
    }
    let duration_d = tests.iter().filter(|t| t.leg != "E").map(|t| t.end_d).fold(0.0, f64::max);
    let cell_duration_d = tests.iter().filter(|t| t.leg == "E").map(|t| t.end_d).fold(0.0, f64::max);
    let mut resources: Vec<String> = vec![];
    for t in &tests {
        if !resources.contains(&t.resource) {
            resources.push(t.resource.clone());
        }
    }

    // requirement matrix
    let matrix: Vec<MatrixRow> = p
        .requirements
        .iter()
        .map(|req| {
            let v = &req.verification;
            let metric = v.metric.clone().unwrap_or_default();
            let (dvt, equip, linked) = dvt_for(&metric);
            let method = match v.method {
                VerifyMethod::Analysis => "analysis",
                VerifyMethod::Test => "test",
                VerifyMethod::Inspection => "inspection",
                VerifyMethod::Demonstration => "demonstration",
            };
            let units = if linked.contains(&"BL") {
                if dvt.starts_with("Weigh") || dvt.starts_with("First-article") {
                    format!("all {total}")
                } else {
                    "5 (subset of baseline; variables data)".into()
                }
            } else if let Some(l) = linked.first() {
                tests
                    .iter()
                    .find(|t| t.id == *l)
                    .map(|t| t.units.to_string())
                    .unwrap_or_else(|| "—".into())
            } else {
                "—".into()
            };
            MatrixRow {
                id: req.id.clone(),
                text: req.text.clone(),
                kind: format!("{:?}", req.kind).to_lowercase(),
                method: method.into(),
                dvt: dvt.into(),
                equipment: equip.into(),
                units,
                pass: crit(v.cmp, v.target, v.target_hi, &v.unit),
                tests: linked.iter().map(|s| s.to_string()).collect(),
            }
        })
        .collect();

    let mut notes = vec![
        format!(
            "Sample sizes demonstrate R = {r} at {:.0} % confidence with {fails} failure(s) allowed; the life legs assume Weibull β = {beta} and test {k} field lives (Lipson). Change them in the sidebar.",
            c * 100.0
        ),
        "Severities are typical choices for a wearable, not requirements of the standards; check the current editions and the product standard that applies to you.".into(),
        "Units that pass destructive legs are not shipped. Baseline performance data on every unit also gives the variation for a tolerance-interval analysis.".into(),
    ];
    if n_attr > 30 {
        notes.push(format!("Leg A needs {n_attr} units — to cut it, lower the demonstrated reliability for the attribute leg, allow a failure with more units, or combine legs sequentially on the same units."));
    }
    let csv = to_csv(&tests);
    let markdown = to_markdown(p, &legs, &tests, &matrix, total, duration_d, &notes);
    DvtOut {
        legs,
        tests,
        matrix,
        total_units: total,
        duration_d,
        cell_duration_d,
        resources,
        reliability: r,
        confidence: c,
        life_multiple: k,
        life_beta: beta,
        csv,
        markdown,
        notes,
    }
}

fn q(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn to_csv(tests: &[Test]) -> String {
    let mut s = String::from(
        "leg,test_id,test,standard,severity,units,duration_days,start_day,end_day,resource,equipment,pass_criterion,linked,basis\n",
    );
    for t in tests {
        s.push_str(
            &[
                q(&t.leg),
                q(&t.id),
                q(&t.name),
                q(&t.standard),
                q(&t.severity),
                t.units.to_string(),
                format!("{:.1}", t.duration_d),
                format!("{:.1}", t.start_d),
                format!("{:.1}", t.end_d),
                q(&t.resource),
                q(&t.equipment),
                q(&t.pass),
                q(&t.linked.join(" ")),
                q(&t.basis),
            ]
            .join(","),
        );
        s.push('\n');
    }
    s
}

fn md(s: &str) -> String {
    s.replace('|', "\\|")
}

fn to_markdown(p: &Project, legs: &[Leg], tests: &[Test], matrix: &[MatrixRow], total: u32, days: f64, notes: &[String]) -> String {
    let mut s = format!(
        "# Design verification test (DVT) plan — {}\n\nGenerated by the Protoforge Reliability & Test Lab. {total} product units; the product legs take about {days:.0} days (bare-cell cycling runs separately, from EVT).\n\n## Legs\n\n| Leg | Name | Units | Sample basis |\n|---|---|---|---|\n",
        p.meta.name
    );
    for l in legs {
        s.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            l.id,
            md(&l.name),
            if l.id == "BL" {
                format!("all {total}")
            } else {
                l.units.to_string()
            },
            md(&l.sample_basis)
        ));
    }
    s.push_str("\n## Tests\n\n| ID | Test | Standard | Severity | Units | Days | Start–end (day) | Pass criterion | Linked |\n|---|---|---|---|---|---|---|---|---|\n");
    for t in tests {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.1} | {:.1}–{:.1} | {} | {} |\n",
            t.id,
            md(&t.name),
            md(&t.standard),
            md(&t.severity),
            t.units,
            t.duration_d,
            t.start_d,
            t.end_d,
            md(&t.pass),
            md(&t.linked.join(", "))
        ));
    }
    s.push_str("\n### Procedures\n\n");
    for t in tests {
        s.push_str(&format!("**{} {}** — {}; equipment: {}.", t.id, t.name, t.standard, t.equipment));
        if !t.basis.is_empty() {
            s.push_str(&format!(" Basis: {}.", t.basis));
        }
        s.push('\n');
        for (i, step) in t.procedure.iter().enumerate() {
            s.push_str(&format!("{}. {}\n", i + 1, step));
        }
        s.push('\n');
    }
    s.push_str("## Requirement verification matrix\n\n| Req | Requirement | Method | DVT activity | Units | Pass | Tests |\n|---|---|---|---|---|---|---|\n");
    for m in matrix {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            m.id,
            md(&m.text),
            m.method,
            md(&m.dvt),
            md(&m.units),
            md(&m.pass),
            m.tests.join(", ")
        ));
    }
    s.push_str("\n## Notes\n\n");
    for n in notes {
        s.push_str(&format!("- {n}\n"));
    }
    s
}
