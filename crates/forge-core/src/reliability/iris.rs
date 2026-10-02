//! Iris (or any project): the use profile, the configuration read from
//! `design.extra.reliability`, and the reliability block diagram built from
//! the bill of materials.
//!
//! **Failure rates are typical order-of-magnitude values at 40 °C for
//! commercial parts — placeholders to replace with supplier reliability
//! reports (most semiconductor vendors publish FIT rates per product family)
//! or your own test data.** Thermally activated parts are scaled to the board
//! temperature in use with an Arrhenius factor (MIL-HDBK-217F uses a similar
//! temperature factor π_T).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{alt, rbd::Node};
use crate::power::budget::{self, UsageProfile};
use crate::power::thermal::Network;
use crate::project::{Component, Project};

#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct BlockOverride {
    pub fit: Option<f64>,
    pub beta: Option<f64>,
    pub eta: Option<f64>,
    /// Where the numbers came from (e.g. "supplier report", "Weibull tab fit").
    pub source: Option<String>,
}

/// Lab settings stored in `design.extra.reliability` (every field optional).
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Config {
    pub mission_years: f64,
    /// Hours per day the glasses are worn and powered.
    pub hours_per_day: f64,
    /// Temple open/close cycles per day (each also folds the hinge flex cable).
    pub hinge_cycles_per_day: f64,
    /// Ambient in use; defaults to `design.thermal.ambient_c`.
    pub use_ambient_c: Option<f64>,
    pub use_rh_pct: f64,
    /// Daily temperature swing of the electronics (on/off, indoor/outdoor), K.
    pub daily_swing_c: f64,
    pub ea_ev: f64,
    pub fit_ref_c: f64,
    /// Microphones that must work (2 = the beamformer needs both).
    pub mic_k: usize,
    /// Count the battery reaching 80 % capacity as a failure.
    pub battery_counts: bool,
    pub blocks: BTreeMap<String, BlockOverride>,
    pub dvt_reliability: f64,
    pub dvt_confidence: f64,
    pub dvt_failures: u32,
    pub life_multiple: f64,
    pub life_beta: f64,
    pub qual_units: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            mission_years: 2.0,
            hours_per_day: 8.0,
            hinge_cycles_per_day: 12.0,
            use_ambient_c: None,
            use_rh_pct: 60.0,
            daily_swing_c: 20.0,
            ea_ev: 0.7,
            fit_ref_c: 40.0,
            mic_k: 2,
            battery_counts: true,
            blocks: BTreeMap::new(),
            dvt_reliability: 0.9,
            dvt_confidence: 0.9,
            dvt_failures: 0,
            life_multiple: 2.0,
            life_beta: 2.0,
            qual_units: 3,
        }
    }
}

impl Config {
    pub fn from_project(p: &Project) -> Config {
        p.design
            .extra
            .get("reliability")
            .and_then(|v| serde_json::from_value::<Config>(v.clone()).ok())
            .unwrap_or_default()
    }
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct UseProfile {
    pub mission_years: f64,
    pub hours_per_day: f64,
    pub mission_h: f64,
    pub mission_days: f64,
    pub battery_life_h: f64,
    pub charge_cycles_per_day: f64,
    pub hinge_cycles_per_day: f64,
    pub ambient_c: f64,
    pub board_c: f64,
    pub pod_heat_mw: f64,
    pub temp_factor: f64,
    pub use_rh_pct: f64,
    pub daily_swing_c: f64,
}

/// The use profile the reliability numbers are computed at.
pub fn use_profile(p: &Project, c: &Config) -> UseProfile {
    let b = budget::budget(p, &UsageProfile::default());
    let heat = budget::pod_heat_mw(p, &UsageProfile::default());
    let net = Network::from_params(&p.design.thermal);
    let ambient = c.use_ambient_c.unwrap_or(p.design.thermal.ambient_c);
    let board = ambient + net.steady(heat * 1e-3)[0];
    let hpd = c.hours_per_day.clamp(0.1, 24.0);
    let life = if b.life_h.is_finite() && b.life_h > 0.0 { b.life_h } else { 24.0 };
    let years = c.mission_years.max(0.01);
    UseProfile {
        mission_years: years,
        hours_per_day: hpd,
        mission_h: years * 365.0 * hpd,
        mission_days: years * 365.0,
        battery_life_h: life,
        charge_cycles_per_day: hpd / life,
        hinge_cycles_per_day: c.hinge_cycles_per_day.max(0.0),
        ambient_c: ambient,
        board_c: board,
        pod_heat_mw: heat,
        temp_factor: alt::arrhenius(c.ea_ev, c.fit_ref_c, board),
        use_rh_pct: c.use_rh_pct,
        daily_swing_c: c.daily_swing_c,
    }
}

/// A part class in the typical failure-rate library.
#[derive(Serialize, Clone, Debug)]
pub struct LibEntry {
    pub class: &'static str,
    /// Typical FIT at 40 °C (0 for pure wear-out items).
    pub fit: f64,
    pub thermal: bool,
    /// Wear-out: (β, η, unit, driver) where driver ∈ {charge, hinge}.
    pub wear: Option<(f64, f64, &'static str, &'static str)>,
    pub note: &'static str,
    pub excluded: bool,
}

/// Typical order-of-magnitude failure rates (FIT at 40 °C) — replace with supplier data.
pub fn classify(c: &Component) -> LibEntry {
    let s = format!("{} {}", c.name, c.part_class).to_lowercase();
    let has = |k: &[&str]| k.iter().any(|w| s.contains(w));
    let e = |class, fit, thermal, note| LibEntry {
        class,
        fit,
        thermal,
        wear: None,
        note,
        excluded: false,
    };
    if has(&["demo lens", "plano lens"]) {
        return LibEntry {
            excluded: true,
            ..e(
                "Passive lens (not a functional failure item here)",
                0.0,
                false,
                "Cracks and scratches are covered by the drop and abrasion tests, not a failure rate.",
            )
        };
    }
    if has(&["graphite", "heat spreader", "nose pad"]) {
        return LibEntry {
            excluded: true,
            ..e(
                "Passive comfort / thermal part (not a functional failure item)",
                0.0,
                false,
                "Delamination of a spreader or a lost nose pad shows up in the drop, sweat and abrasion tests, not as a failure rate.",
            )
        };
    }
    if has(&["thick film", "pull-down"]) {
        return e("Chip resistor", 0.5, false, "");
    }
    if has(&["flex cable", "ffc"]) {
        return LibEntry {
            wear: Some((2.2, 60_000.0, "folds", "hinge")),
            ..e(
                "Dynamic flex cable (fold fatigue)",
                0.0,
                false,
                "Conductor fatigue in the hinge (Iris risk F-2); one fold per open/close. Fit your fold-test data in the Weibull tab.",
            )
        };
    }
    if has(&["hinge"]) {
        return LibEntry {
            wear: Some((3.0, 60_000.0, "cycles", "hinge")),
            ..e(
                "Spring hinge (wear and spring fatigue)",
                0.0,
                false,
                "Open/close wear-out; fit your hinge-cycling data in the Weibull tab.",
            )
        };
    }
    if has(&["connector", "receptacle", "usb"]) {
        return if has(&["usb"]) {
            e(
                "USB-C receptacle",
                20.0,
                false,
                "Mechanical: solder-joint and contact wear; mating-cycle life belongs in the DVT wear leg.",
            )
        } else {
            e("Board-to-board / FPC connector", 10.0, false, "Contact fretting and latch damage.")
        };
    }
    if has(&["thick film", "resistor", "divider", "pull-up"]) {
        return e("Chip resistor", 0.5, false, "");
    }
    if has(&["touch"]) {
        return e("Touch controller IC", 10.0, true, "");
    }
    if has(&["mlcc", "capacitor", " cap "]) || s.contains(" cap") {
        return e(
            "Ceramic capacitor (MLCC)",
            1.0,
            false,
            "Flex cracking dominates in thin boards — keep MLCCs away from board edges and screw bosses.",
        );
    }
    if has(&["quartz crystal", "mhz crystal"]) {
        return e(
            "Quartz crystal",
            10.0,
            false,
            "Shock can crack the blank or shift frequency — part of the drop test; check the start-up margin (negative resistance) at temperature.",
        );
    }
    if has(&["rf chip inductor", "rf match"]) {
        return e("RF chip inductor", 1.0, false, "");
    }
    if has(&["inductor"]) {
        return e("Power inductor", 2.0, false, "");
    }
    if has(&["charger"]) {
        return e("Battery charger IC", 10.0, true, "");
    }
    if has(&["lipo", "li-ion cell", "pouch cell"]) || (has(&["cell"]) && !has(&["focus cell"])) {
        return LibEntry {
            wear: Some((4.0, 800.0, "cycles", "charge")),
            ..e("Lithium-polymer cell (capacity fade)", 0.0, false, "Failure = capacity below 80 % of rated; η in equivalent full cycles (typical assumption — use the cell maker's cycle-life data or your own).")
        };
    }
    if has(&["system-on-chip", "soc", "microcontroller", "mcu"]) {
        return e("Microcontroller / radio SoC", 25.0, true, "");
    }
    if has(&["oled", "display"]) && !has(&["load switch", "fpc"]) {
        return e(
            "Micro-display module (panel, driver, flex)",
            250.0,
            true,
            "Emitter luminance decay is a separate wear-out; check the panel's rated luminance lifetime.",
        );
    }
    if has(&["imu", "accelerometer", "gyroscope"]) {
        return e("MEMS inertial sensor", 15.0, true, "");
    }
    if has(&["microphone", "mems mic"]) {
        return e(
            "MEMS microphone",
            10.0,
            true,
            "Port blockage by sweat and dust is the practical failure — protect the port mesh.",
        );
    }
    if has(&["buck", "converter", "regulator", "ldo"]) {
        return e("Power converter IC", 10.0, true, "");
    }
    if has(&["load switch"]) {
        return e("Load switch", 5.0, true, "");
    }
    if has(&["flash", "memory", "eeprom"]) {
        return e(
            "Serial flash",
            10.0,
            true,
            "Write endurance and data retention are separate wear-out limits.",
        );
    }
    if has(&["antenna"]) {
        return e("Chip antenna", 1.0, false, "");
    }
    if has(&["lens", "combiner", "optic", "focus cell", "beam-splitter"]) {
        return e(
            "Bonded optics",
            2.0,
            false,
            "Adhesive creep, delamination or haze; covered by damp heat and thermal cycling.",
        );
    }
    if has(&["pcb", "printed circuit"]) {
        return e(
            "Printed circuit board and solder joints",
            10.0,
            false,
            "Solder-joint fatigue under thermal cycling is estimated in the ALT tab (Norris–Landzberg).",
        );
    }
    e("Unclassified part", 10.0, false, "No class matched — generic value; replace it.")
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Excluded {
    pub refdes: String,
    pub name: String,
    pub reason: String,
}

/// Build the Iris block diagram: a series chain of subsystems, each a series
/// of its parts (the microphones as k-of-2 when `mic_k` < 2).
pub fn tree(p: &Project, c: &Config, u: &UseProfile) -> (Node, Vec<Excluded>) {
    let mut excluded = vec![];
    let mut groups: Vec<Node> = vec![];
    let mut order: Vec<(String, String)> = p.subsystems.iter().map(|s| (s.id.clone(), s.name.clone())).collect();
    for comp in &p.components {
        if !order.iter().any(|(id, _)| *id == comp.subsystem) {
            order.push((
                comp.subsystem.clone(),
                if comp.subsystem.is_empty() {
                    "Other".into()
                } else {
                    comp.subsystem.clone()
                },
            ));
        }
    }
    for (sid, sname) in order {
        let mut kids: Vec<Node> = vec![];
        let mut mics: Vec<Node> = vec![];
        for comp in p.components.iter().filter(|x| x.subsystem == sid) {
            let lib = classify(comp);
            if lib.excluded {
                excluded.push(Excluded {
                    refdes: comp.refdes.clone(),
                    name: comp.name.clone(),
                    reason: lib.class.into(),
                });
                continue;
            }
            if lib.wear.is_some_and(|w| w.3 == "charge") && !c.battery_counts {
                excluded.push(Excluded {
                    refdes: comp.refdes.clone(),
                    name: comp.name.clone(),
                    reason: "Battery treated as a consumable service item (setting)".into(),
                });
                continue;
            }
            let ov = c.blocks.get(&comp.refdes).cloned().unwrap_or_default();
            let mut n = Node {
                id: comp.refdes.clone(),
                label: comp.name.clone(),
                kind: "block".into(),
                refdes: comp.refdes.clone(),
                class: lib.class.into(),
                qty: comp.qty.max(0.0),
                note: lib.note.into(),
                source: ov.source.clone().unwrap_or_else(|| "typical".into()),
                ..Default::default()
            };
            if let Some((beta, eta, unit, driver)) = lib.wear {
                n.model = "weibull".into();
                n.beta = ov.beta.unwrap_or(beta);
                n.eta = ov.eta.unwrap_or(eta);
                n.life_unit = unit.into();
                n.per_hour = if driver == "charge" {
                    1.0 / u.battery_life_h
                } else {
                    u.hinge_cycles_per_day / u.hours_per_day
                };
            } else {
                n.model = "exp".into();
                n.fit_ref = lib.fit;
                n.temp_factor = if lib.thermal { u.temp_factor } else { 1.0 };
                n.fit = ov.fit.unwrap_or(lib.fit * n.temp_factor);
                if ov.fit.is_some() {
                    n.temp_factor = 1.0;
                    n.fit_ref = n.fit;
                }
            }
            if (ov.fit.is_some() || ov.beta.is_some() || ov.eta.is_some()) && ov.source.is_none() {
                n.source = "user".into();
            }
            if lib.class == "MEMS microphone" {
                mics.push(n);
            } else {
                kids.push(n);
            }
        }
        if !mics.is_empty() {
            let k = c.mic_k.clamp(1, mics.len());
            if k < mics.len() {
                kids.insert(
                    0,
                    Node {
                        id: format!("{sid}-mics"),
                        label: format!("Microphones ({k} of {} needed)", mics.len()),
                        kind: "kofn".into(),
                        k,
                        children: mics,
                        qty: 1.0,
                        ..Default::default()
                    },
                );
            } else {
                for (i, m) in mics.into_iter().enumerate() {
                    kids.insert(i, m);
                }
            }
        }
        if kids.is_empty() {
            continue;
        }
        groups.push(Node {
            id: sid.clone(),
            label: sname,
            kind: "series".into(),
            children: kids,
            qty: 1.0,
            ..Default::default()
        });
    }
    (
        Node {
            id: "system".into(),
            label: p.meta.name.clone(),
            kind: "series".into(),
            children: groups,
            qty: 1.0,
            ..Default::default()
        },
        excluded,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_classes_and_profile() {
        let p = Project::iris();
        let c = Config::default();
        let u = use_profile(&p, &c);
        assert!(u.board_c > u.ambient_c && u.board_c < u.ambient_c + 20.0, "{:?}", u);
        assert!(u.battery_life_h > 5.0 && u.battery_life_h < 30.0);
        let (t, ex) = tree(&p, &c, &u);
        assert!(ex.iter().any(|e| e.refdes == "LN1"));
        let leaves = super::super::rbd::leaves(&t);
        let find = |r: &str| leaves.iter().find(|b| b.refdes == r).unwrap_or_else(|| panic!("{r}"));
        assert_eq!(find("BT1").model, "weibull");
        assert_eq!(find("FPC1").model, "weibull");
        assert_eq!(find("HW1").model, "weibull");
        assert_eq!(find("HW1").qty, 2.0);
        assert_eq!(find("U7").class, "Touch controller IC");
        assert_eq!(find("C1").class, "Ceramic capacitor (MLCC)");
        assert_eq!(find("J3").class, "Board-to-board / FPC connector");
        assert_eq!(find("R1").class, "Chip resistor");
        assert_eq!(find("OPT2").class, "Bonded optics");
        assert_eq!(find("R6").class, "Chip resistor");
        assert_eq!(find("C7").class, "Ceramic capacitor (MLCC)");
        assert_eq!(find("Y1").class, "Quartz crystal");
        assert_eq!(find("L2").class, "RF chip inductor");
        assert!(ex.iter().any(|e| e.refdes == "TH1") && ex.iter().any(|e| e.refdes == "NP1"));
        assert!(leaves.iter().all(|b| b.class != "Unclassified part"), "unclassified parts in Iris");
    }

    #[test]
    fn mic_redundancy_setting() {
        let p = Project::iris();
        let c = Config {
            mic_k: 1,
            ..Default::default()
        };
        let u = use_profile(&p, &c);
        let (t, _) = tree(&p, &c, &u);
        let aud = t.children.iter().find(|g| g.id == "SYS-AUD").unwrap();
        assert_eq!(aud.children[0].kind, "kofn");
        assert_eq!(aud.children[0].k, 1);
    }
}
