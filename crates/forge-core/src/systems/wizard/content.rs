//! Project assembly: needs, requirements (bound only to responsive metrics),
//! subsystems, interfaces, FMEA risks, one computed trade study, stage gates
//! and the design parameters the labs read.

use serde_json::json;

use super::catalog::{self, Contact};
use super::parts::{nice_down, nice_up, round_to, ACT, AUD, CMP, FW, MFG, OPT, PCB, PWR, QLINK, QRNG, SNS, STR, UI};
use super::{fmt, process_cost, Ctx, Source};
use crate::project::*;

/// A requirement under construction.
struct R {
    kind: ReqKind,
    text: String,
    rationale: String,
    needs: Vec<&'static str>,
    subs: Vec<&'static str>,
    metric: Option<&'static str>,
    cmp: Cmp,
    target: f64,
    unit: &'static str,
    method: VerifyMethod,
}

#[allow(clippy::too_many_arguments)]
fn bound(
    kind: ReqKind,
    text: String,
    rationale: String,
    needs: &[&'static str],
    subs: &[&'static str],
    metric: &'static str,
    cmp: Cmp,
    target: f64,
    unit: &'static str,
) -> R {
    debug_assert!(catalog::bindable(metric), "{metric} is not a responsive metric");
    R {
        kind,
        text,
        rationale,
        needs: needs.to_vec(),
        subs: subs.to_vec(),
        metric: Some(metric),
        cmp,
        target,
        unit,
        method: VerifyMethod::Analysis,
    }
}

fn manual(kind: ReqKind, method: VerifyMethod, text: String, rationale: String, needs: &[&'static str], subs: &[&'static str]) -> R {
    R {
        kind,
        text,
        rationale,
        needs: needs.to_vec(),
        subs: subs.to_vec(),
        metric: None,
        cmp: Cmp::Ge,
        target: 1.0,
        unit: "pass",
        method,
    }
}

impl R {
    /// The acceptance criterion of a test/inspection requirement.
    fn crit(mut self, cmp: Cmp, target: f64, unit: &'static str) -> R {
        self.cmp = cmp;
        self.target = target;
        self.unit = unit;
        self
    }
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for ch in s.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let s = out.trim_end_matches('-').to_string();
    if s.is_empty() {
        "new-product".into()
    } else {
        s
    }
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) if !s.starts_with("I ") && !s.chars().nth(1).is_some_and(|x| x.is_uppercase()) => {
            f.to_lowercase().collect::<String>() + c.as_str()
        }
        Some(f) => f.to_string() + c.as_str(),
        None => String::new(),
    }
}

/// Score 1..10 where `best` maps to 9 and `worst` to 3 (linear).
fn score(x: f64, best: f64, worst: f64) -> f64 {
    if (best - worst).abs() < 1e-12 {
        return 6.0;
    }
    round_to((3.0 + 6.0 * (x - worst) / (best - worst)).clamp(1.0, 10.0), 1)
}

impl Ctx {
    fn subsystems(&self) -> Vec<Subsystem> {
        let s = |id: &str, name: &str, domain: Domain, desc: String, functions: &[&str], lab: &str| Subsystem {
            id: id.into(),
            name: name.into(),
            domain,
            description: desc,
            functions: functions.iter().map(|x| x.to_string()).collect(),
            lab: lab.into(),
        };
        let mut v = vec![];
        let e = &self.enclosure;
        if self.is("quantum") {
            v.push(s(
                STR,
                "Rack enclosures",
                Domain::Mechanical,
                "Two interlocked 19-inch enclosures, one per node.".into(),
                &["House each node", "Interlock the laser"],
                "cad",
            ));
            v.push(s(
                PWR,
                "Bench power",
                Domain::Electrical,
                "Certified low-voltage bench supplies.".into(),
                &["Supply every module"],
                "power",
            ));
            v.push(s(
                QRNG,
                "Quantum random number generator",
                Domain::Electrical,
                "Diode avalanche noise, ADC and Toeplitz conditioning with health tests.".into(),
                &["Generate raw entropy", "Condition and health-test"],
                "quantum",
            ));
            v.push(s(
                QLINK,
                "Photon source, fiber channel and detectors",
                Domain::Optical,
                format!(
                    "Weak coherent pulses, phase encoding, {} km of single-mode fiber and gated InGaAs detectors.",
                    fmt(self.tv("fiber_km"))
                ),
                &["Emit and encode weak pulses", "Carry single photons", "Detect and decode"],
                "quantum",
            ));
            v.push(s(
                CMP,
                "Timing and post-processing",
                Domain::Software,
                "FPGA time tagging, sifting, error correction and privacy amplification.".into(),
                &["Time-tag detections", "Distil secret keys"],
                "quantum",
            ));
            v.push(s(
                MFG,
                "Integration and test",
                Domain::Manufacturing,
                "Bench integration, calibration and acceptance tests.".into(),
                &["Source modules", "Integrate", "Calibrate"],
                "mfg",
            ));
            return v;
        }
        if e.process != "none" {
            let (name, lab) = if self.is("mechanical") {
                ("Bracket", "structures")
            } else if self.is("robot") {
                ("Chassis and covers", if e.process == "molded" { "molding" } else { "print" })
            } else {
                ("Enclosure", if e.process == "molded" { "molding" } else { "print" })
            };
            v.push(s(
                STR,
                name,
                Domain::Mechanical,
                format!(
                    "{} {} part{}, {}.",
                    if e.process == "molded" { "Injection-molded" } else { "3-D printed" },
                    e.material,
                    if self.is("mechanical") { "" } else { "s" },
                    if self.has("waterproof") {
                        "gasket-sealed"
                    } else {
                        "screw-assembled"
                    }
                ),
                &["Protect the contents", "Carry loads and mounting"],
                lab,
            ));
        }
        if self.electronics {
            let src = match self.battery.source {
                Source::Lipo => format!("{} with charger and buck converter", self.battery.name),
                Source::Coin(_) => self.battery.name.clone(),
                Source::None => {
                    if self.has("dc_adapter") {
                        "12 V adapter input with buck converter".into()
                    } else {
                        "USB-C 5 V input with buck converter".into()
                    }
                }
            };
            v.push(s(
                PWR,
                "Power",
                Domain::Electrical,
                format!("{src}."),
                &["Store or receive energy", "Regulate the rails"],
                "power",
            ));
            let (cname, lab) = if self.has("wifi") || self.has("camera") {
                ("Compute and Wi-Fi radio", "circuit")
            } else if self.has("ble") {
                ("Compute and BLE radio", "rf")
            } else {
                ("Compute", "circuit")
            };
            v.push(s(
                CMP,
                cname,
                Domain::Electrical,
                "System-on-chip or microcontroller, crystal, memory and antenna.".into(),
                &["Run the firmware", "Talk to the phone or network"],
                lab,
            ));
            v.push(s(
                PCB,
                "Main board",
                Domain::Electrical,
                format!(
                    "{}-layer board, {} × {} mm.",
                    self.board.layers,
                    fmt(self.board.w),
                    fmt(self.board.h)
                ),
                &["Interconnect the electronics"],
                "pcb",
            ));
            v.push(s(
                FW,
                "Firmware",
                Domain::Software,
                "Power states, sensing, user interface and updates.".into(),
                &["Schedule work and sleep", "Handle the user interface", "Update in the field"],
                "proto",
            ));
        }
        if self.has("display") || self.has("buttons") || self.has("leds") {
            v.push(s(
                UI,
                "User interface",
                Domain::Electrical,
                "Display, buttons or touch, status light.".into(),
                &["Show information", "Take input"],
                "proto",
            ));
        }
        if self.has("microphones") || self.has("speaker") {
            v.push(s(
                AUD,
                "Audio",
                Domain::Acoustic,
                "Microphone pair and/or speaker with amplifier.".into(),
                &["Capture voice", "Play prompts"],
                "dsp",
            ));
        }
        if self.has("imu") || self.has("env_sensors") || self.has("camera") {
            v.push(s(
                SNS,
                "Sensing",
                Domain::Electrical,
                "Motion, environment and/or camera sensors.".into(),
                &["Measure the world"],
                "control",
            ));
        }
        if self.has("motors") {
            v.push(s(
                ACT,
                if self.is("robot") { "Motors and drive" } else { "Actuator" },
                Domain::Mechanical,
                "Gear motors and an H-bridge driver.".into(),
                &["Produce motion"],
                if self.is("robot") { "robotics" } else { "mechanisms" },
            ));
        }
        if self.is("optical") {
            v.push(s(
                OPT,
                "Optical train",
                Domain::Optical,
                "LED illuminator, collimating lens and photodiode receiver.".into(),
                &["Illuminate the target", "Collect and detect light"],
                "optics",
            ));
        }
        v.push(s(
            MFG,
            "Manufacturing",
            Domain::Manufacturing,
            "Sourcing, assembly and end-of-line test.".into(),
            &["Source parts", "Assemble", "Test"],
            "mfg",
        ));
        v
    }

    fn interfaces(&self, subs: &[Subsystem]) -> Vec<Interface> {
        let has = |id: &str| subs.iter().any(|s| s.id == id);
        let mut v: Vec<(&str, &str, &str, String)> = vec![];
        if self.is("quantum") {
            v.push((QRNG, QLINK, "LVDS", "Random bit, basis and decoy choices, one per pulse.".into()));
            v.push((
                QLINK,
                CMP,
                "GPIO",
                "Detection pulses to the time tagger; gates back to the detectors.".into(),
            ));
            v.push((QRNG, CMP, "USB", "Seeds for privacy amplification.".into()));
            v.push((PWR, CMP, "POWER", "Bench supply rails.".into()));
            v.push((PWR, QLINK, "POWER", "Laser driver, modulators and detector coolers.".into()));
            v.push((
                QLINK,
                STR,
                "MECH",
                "Modules on rack shelves; laser inside the interlocked enclosure.".into(),
            ));
        } else {
            if has(PWR) {
                v.push((PWR, CMP, "POWER", format!("{} V rail from the regulator.", self.core_v())));
                for (sub, what) in [
                    (UI, "Display and LED supply."),
                    (AUD, "Microphone and amplifier supply."),
                    (SNS, "Sensor supply."),
                    (ACT, "Battery rail to the motor driver."),
                    (OPT, "LED driver supply."),
                ] {
                    if has(sub) {
                        v.push((PWR, sub, "POWER", what.into()));
                    }
                }
            }
            if has(SNS) {
                v.push((
                    SNS,
                    CMP,
                    if self.has("camera") { "DVP" } else { "I2C" },
                    if self.has("camera") {
                        "Camera pixels on the parallel bus; sensors on I2C.".into()
                    } else {
                        "Sensor registers over I2C plus an interrupt line.".into()
                    },
                ));
            }
            if has(UI) {
                v.push((
                    CMP,
                    UI,
                    if self.has("display") { "SPI" } else { "GPIO" },
                    if self.has("display") {
                        "Frames to the display; buttons and LED on GPIO.".into()
                    } else {
                        "Buttons and LED on GPIO.".into()
                    },
                ));
            }
            if self.has("microphones") {
                v.push((AUD, CMP, "PDM", "Two microphones on one PDM data line.".into()));
            }
            if self.has("speaker") {
                v.push((CMP, AUD, "I2S", "Audio samples to the amplifier.".into()));
            }
            if has(ACT) {
                v.push((CMP, ACT, "PWM", "PWM and direction to the H-bridge; encoder pulses back.".into()));
                v.push((ACT, STR, "MECH", "Motor clamps and wheel hubs or linkage.".into()));
            }
            if has(OPT) {
                v.push((CMP, OPT, "GPIO", "LED dimming (PWM) and the photodiode signal to the ADC.".into()));
                v.push((
                    OPT,
                    STR,
                    "OPTICAL",
                    "Lens and LED aligned in the bench insert; window in the enclosure.".into(),
                ));
            }
            if has(FW) {
                v.push((
                    CMP,
                    FW,
                    if self.has("ble") { "BLE" } else { "SW" },
                    if self.has("ble") {
                        "Firmware services over BLE to the phone app.".into()
                    } else {
                        "Firmware runs on the SoC.".into()
                    },
                ));
            }
            if has(PCB) && has(STR) {
                v.push((PCB, STR, "MECH", "Board on bosses inside the enclosure.".into()));
            }
            if has(STR) && matches!(self.battery.source, Source::Lipo | Source::Coin(_)) {
                v.push((PWR, STR, "MECH", "Battery in a cradle or behind a door.".into()));
            }
            if has(UI) && has(STR) && self.has("display") {
                v.push((UI, STR, "MECH", "Display behind a window in the enclosure.".into()));
            }
            if has(STR) && self.electronics && !has(PCB) {
                v.push((PWR, STR, "MECH", "Electronics mounted in the part.".into()));
            }
        }
        v.into_iter()
            .enumerate()
            .map(|(i, (f, t, k, d))| Interface {
                id: format!("IF-{:02}", i + 1),
                from: f.into(),
                to: t.into(),
                kind: k.into(),
                description: d,
            })
            .collect()
    }

    /// (key, text, stakeholder, priority)
    fn need_list(&self) -> Vec<(&'static str, String, &'static str, u8)> {
        let mut v = vec![];
        let idea = if self.idea.is_empty() {
            self.kind.blurb.trim_end_matches('.').to_string()
        } else {
            self.idea.clone()
        };
        if self.is("quantum") {
            v.push((
                "core",
                "Shares secret keys between two nodes over fiber, with eavesdropping detectable".to_string(),
                "Researcher",
                1,
            ));
            v.push((
                "random",
                "Draws every random choice from a physical entropy source".to_string(),
                "Security",
                1,
            ));
        } else if self.is("mechanical") {
            v.push(("core", format!("Does its job: {}", lower_first(&idea)), "User", 1));
            v.push((
                "load",
                format!("Holds a {} N load without breaking or sagging", fmt(self.tv("load_n"))),
                "User",
                1,
            ));
        } else {
            v.push(("core", format!("Does its job: {}", lower_first(&idea)), "User", 1));
        }
        let carry = match self.kind.id {
            "wearable" => "Light and small enough to wear all day",
            "handheld" | "optical" => "Comfortable to hold and carry in a bag",
            "iot_sensor" => "Small enough to stick anywhere",
            "robot" => "Light enough to lift and safe if it falls off a desk",
            "dev_board" => "Fits a breadboard bench and a student's budget",
            "quantum" => "Fits a lab bench and a teaching-lab budget",
            _ => "Fits where it is used without getting in the way",
        };
        v.push(("carry", carry.to_string(), "User", 2));
        if self.has_battery() {
            let life = self.tv("life_h");
            let t = if life >= 24.0 * 300.0 {
                format!(
                    "Runs about {} on one cell",
                    if life >= 8760.0 {
                        format!("{} year{}", fmt(life / 8760.0), if life >= 2.0 * 8760.0 { "s" } else { "" })
                    } else {
                        format!("{} days", fmt(life / 24.0))
                    }
                )
            } else {
                format!("Runs {} h of typical use per charge", fmt(life))
            };
            v.push(("power", t, "User", 1));
        }
        if self.electronics {
            v.push((
                "safe",
                format!("Safe to touch and use, {}", self.kind.contact.words()),
                "User / Safety",
                1,
            ));
            v.push((
                "durable",
                format!("Keeps working for {} years of everyday use", fmt(self.tv("design_years"))),
                "User",
                2,
            ));
        }
        if self.radio() {
            v.push((
                "connect",
                if self.has("wifi") {
                    "Connects to the home network and a phone".into()
                } else {
                    "Talks to a phone".into()
                },
                "User",
                1,
            ));
        }
        if self.has("microphones") {
            v.push(("voice", "Hears its user in a noisy room".into(), "User", 2));
        }
        if self.has("waterproof") {
            v.push(("env", "Survives rain, sweat and a drop into water".into(), "User", 2));
        }
        v.push((
            "cost",
            format!("Affordable at about {} USD", fmt(self.tv("retail_usd"))),
            "Business",
            1,
        ));
        v.push(("build", "Buildable by hand now and in volume later".into(), "Maker / Business", 2));
        v
    }

    fn reqs(&self) -> Vec<R> {
        use Cmp::*;
        use ReqKind as K;
        use VerifyMethod as M;
        let mut v: Vec<R> = vec![];
        let (l, w, h) = (self.tv("length_mm"), self.tv("width_mm"), self.tv("height_mm"));
        let size_sub: &'static str = if self.enclosure.process == "none" { PCB } else { STR };
        let ambient = self.tv("ambient_c");
        // ---- envelope, mass, cost --------------------------------------------
        v.push(manual(
            K::Interface,
            M::Inspection,
            format!("The product shall fit within a {} × {} × {} mm envelope.", fmt(l), fmt(w), fmt(h)),
            "From the size target. CAD Studio's parametric model is the Iris frame, so this is checked on the product's own CAD by inspection.".into(),
            &["carry", "core"],
            &[size_sub],
        ).crit(Le, l, "mm"));
        let mass_subs: Vec<&'static str> = [STR, PWR, ACT].into_iter().filter(|s| self.sub_exists(s)).collect();
        v.push(bound(
            K::Performance,
            format!(
                "Total product mass, energy source included, shall be at most {} g.",
                fmt(self.tv("mass_g"))
            ),
            "From the mass target; verified by the BOM mass roll-up (every part × quantity, enclosure included).".into(),
            &["carry"],
            &mass_subs,
            "mass.components_g",
            Le,
            self.tv("mass_g"),
            "g",
        ));
        v.push(bound(
            K::Cost,
            format!("Parts cost per unit at {} units shall be at most {} USD.", fmt(self.tv("quantity")), fmt(self.tv("cost_usd"))),
            format!(
                "Keeps the parts near a quarter of the {} USD retail price (a common hardware rule of thumb). Verified by the Manufacturing Hub's volume-priced BOM (learning-curve price breaks on the quantity-1 prices).",
                fmt(self.tv("retail_usd"))
            ),
            &["cost", "build"],
            &[MFG],
            "mfg.bom_cost_usd",
            Le,
            self.tv("cost_usd"),
            "USD",
        ));
        // ---- energy -----------------------------------------------------------
        if self.has_battery() {
            let life = self.tv("life_h");
            let b = &self.battery.params;
            let alloc = nice_down(b.capacity_mah * b.nominal_v * b.usable_fraction / life);
            v.push(bound(
                K::Performance,
                format!("Battery life in the reference use profile shall be at least {} h.", fmt(life)),
                "From the life target; the reference profile is the BOM duty cycles. Verified by the power roll-up: usable energy ÷ average power.".into(),
                &["power"],
                &[PWR, CMP, FW],
                "budget.battery_life_h",
                Ge,
                life,
                "h",
            ));
            v.push(bound(
                K::Performance,
                format!(
                    "Average battery power in the reference use profile shall be at most {} mW.",
                    fmt(alloc)
                ),
                format!(
                    "The power allocation: {} mAh × {} V × {} usable ÷ {} h = {} mW, rounded down.",
                    fmt(b.capacity_mah),
                    fmt(b.nominal_v),
                    fmt(b.usable_fraction),
                    fmt(life),
                    fmt(b.capacity_mah * b.nominal_v * b.usable_fraction / life)
                ),
                &["power"],
                &[PWR, CMP, FW],
                "budget.avg_power_mw",
                Le,
                alloc,
                "mW",
            ));
        }
        if self.electronics {
            let lim = self.peak_limit_mw();
            let why = match self.battery.source {
                Source::Lipo => format!(
                    "2C typical continuous rating of a {} mAh pouch × 3.7 V — check the cell's data sheet.",
                    fmt(self.battery.mah)
                ),
                Source::Coin(_) => {
                    "A coin cell delivers pulses of about 15 mA with a bulk capacitor (typical) × 3 V; check the cell maker's pulse data."
                        .into()
                }
                Source::None => {
                    if self.is("quantum") {
                        "Both nodes on lab circuits with headroom.".into()
                    } else if self.has("dc_adapter") {
                        "A 12 V 1 A adapter derated to 80 %.".into()
                    } else {
                        "USB's default budget: 5 V × 500 mA without Power Delivery negotiation.".into()
                    }
                }
            };
            v.push(bound(
                K::Safety,
                format!("Peak power with every load active shall be at most {} mW.", fmt(lim)),
                why,
                &["safe"],
                &[PWR],
                "power.peak_power_mw",
                Le,
                lim,
                "mW",
            ));
            let limit = self.kind.contact.surface_limit_c();
            v.push(bound(
                K::Safety,
                format!("The hottest touchable surface shall stay at or below {} °C at {} °C ambient with the display at full duty.", fmt(limit), fmt(ambient)),
                format!(
                    "A typical design limit for a product {} (check the touch-temperature limits of IEC 62368-1 for your material and contact time). Verified by the lumped thermal model.",
                    self.kind.contact.words()
                ),
                &["safe"],
                &[if self.sub_exists(STR) { STR } else { PCB }, PWR],
                "thermal.max_skin_c",
                Le,
                limit,
                "C",
            ));
            let years = self.tv("design_years");
            let hours = years * 365.0 * self.tv("hours_per_day");
            let mtbf = nice_up(hours / -(0.9f64.ln()));
            let rel_subs: Vec<&'static str> = [CMP, PCB, PWR].into_iter().filter(|s| self.sub_exists(s)).collect();
            v.push(bound(
                K::Performance,
                format!("Parts-count mean time between failures (MTBF) shall be at least {} h.", fmt(mtbf)),
                format!(
                    "At most 10 % random failures over the design life: R = exp(−t/MTBF) ≥ 0.9 with t = {} years × 365 × {} h/day = {} h ⇒ MTBF ≥ t ÷ 0.105.",
                    fmt(years),
                    fmt(self.tv("hours_per_day")),
                    fmt(hours)
                ),
                &["durable"],
                &rel_subs,
                "reliability.mtbf_h",
                Ge,
                mtbf,
                "h",
            ));
            v.push(bound(
                K::Performance,
                format!("System B10 life (10 % of units failed) shall be at least {} years.", fmt(years)),
                "From the design life. Includes wear-out: a rechargeable cell reaching 80 % capacity counts as a failure.".into(),
                &["durable"],
                &rel_subs,
                "reliability.b10_years",
                Ge,
                years,
                "years",
            ));
        }
        if self.has("battery") {
            v.push(bound(
                K::Performance,
                format!("Regulated {} V rail ripple shall be at most 30 mV peak-to-peak.", self.core_v()),
                "Keeps sensors and the radio quiet; verified by the Circuit Lab's buck-converter simulation with the battery voltage and the L1/C2 values in the BOM.".into(),
                &["safe"],
                &[PWR],
                "circuit.buck_ripple_mv",
                Le,
                30.0,
                "mV",
            ));
            if self.has("usb_c") {
                let i = (0.5 * self.battery.mah).min(500.0);
                let t = nice_up(1.2 * self.battery.mah / i);
                v.push(
                    manual(
                        K::Functional,
                        M::Test,
                        format!("A full charge from USB-C shall take at most {} h.", fmt(t)),
                        format!(
                        "Charging at {} mA (min of 0.5 C and USB's 500 mA default) plus the constant-voltage taper. Measure on 3 units.",
                        fmt(i)
                    ),
                        &["power"],
                        &[PWR],
                    )
                    .crit(Le, t, "h"),
                );
            }
            v.push(manual(
                K::Safety,
                M::Inspection,
                "The battery shall carry protection against over-charge, over-discharge and short circuit.".into(),
                "Lithium cells can vent or burn when abused; a protection circuit and a cell tested to IEC 62133-2 (check the current edition) are expected by retailers and test labs.".into(),
                &["safe"],
                &[PWR],
            ));
        }
        if self.has("coin_cell") {
            v.push(manual(
                K::Safety,
                M::Inspection,
                "The coin-cell compartment shall need a tool or two independent motions to open.".into(),
                "Swallowed coin cells cause severe injuries to children; many markets now regulate button-cell products (for example Reese's Law in the US) — check the rules where you sell.".into(),
                &["safe"],
                &[PWR, if self.sub_exists(STR) { STR } else { PCB }],
            ));
        }
        // ---- connectivity -------------------------------------------------------
        if self.has("ble") {
            v.push(bound(
                K::Performance,
                format!("The Bluetooth Low Energy link to a phone shall close with at least 3 dB margin at {} m.", fmt(self.tv("range_m"))),
                "Margin after a 10 dB fading allowance (90 % availability), indoor path-loss exponent 2.5. Verified by the RF & Antenna Lab's link budget with this product's antenna efficiency, losses and matching network.".into(),
                &["connect"],
                &[CMP],
                "rf.link_margin_db",
                Ge,
                3.0,
                "dB",
            ));
        }
        if self.has("wifi") {
            v.push(
                manual(
                    K::Functional,
                    M::Test,
                    format!(
                        "The device shall stay connected to a WPA2 Wi-Fi access point at {} m through one interior wall.",
                        fmt(self.tv("range_m"))
                    ),
                    "No Wi-Fi link metric exists in the engine yet; verify with a throughput test.".into(),
                    &["connect"],
                    &[CMP],
                )
                .crit(Ge, self.tv("range_m"), "m"),
            );
        }
        if self.radio() {
            v.push(manual(
                K::Regulatory,
                M::Inspection,
                "The radio shall be certified for every target market (FCC Part 15 in the US, the Radio Equipment Directive in the EU)."
                    .into(),
                "Using a pre-certified radio module shortens this; the Build Guide lists the compliance path.".into(),
                &["connect", "build"],
                &[CMP],
            ));
        }
        // ---- interaction, audio, sensing ---------------------------------------
        if self.has("display") {
            v.push(
                manual(
                    K::Functional,
                    M::Test,
                    "Display text shall be legible at 40 cm in a 500 lux room.".into(),
                    "Office lighting level; test with 5 users.".into(),
                    &["core"],
                    &[UI],
                )
                .crit(Ge, 40.0, "cm"),
            );
        }
        if self.has("buttons") {
            v.push(
                manual(
                    K::Functional,
                    M::Test,
                    if self.is("wearable") {
                        "A touch shall register within 50 ms with a sweaty or gloved finger.".into()
                    } else {
                        "Each button shall register a press within 50 ms and survive 100,000 presses.".into()
                    },
                    "Responsiveness and wear; test on a cycling rig.".into(),
                    &["core"],
                    &[UI],
                )
                .crit(Le, 50.0, "ms"),
            );
        }
        if self.has("leds") {
            v.push(
                manual(
                    K::Functional,
                    M::Demonstration,
                    "The status LED shall be visible at 2 m in a 500 lux room.".into(),
                    "Glanceable state.".into(),
                    &["core"],
                    &[UI],
                )
                .crit(Ge, 2.0, "m"),
            );
        }
        if self.has("microphones") {
            v.push(bound(
                K::Performance,
                "Microphone-array directivity index at 1 kHz shall be at least 4 dB.".into(),
                "Rejects noise from behind; verified by the DSP Lab's delay-and-subtract model with this product's microphone spacing."
                    .into(),
                &["voice"],
                &[AUD],
                "dsp.mic_di_db",
                Ge,
                4.0,
                "dB",
            ));
            v.push(bound(
                K::Performance,
                "PDM decimation alias attenuation shall be at least 60 dB.".into(),
                "Keeps aliased ultrasonic noise below the speech floor; verified by the DSP Lab's decimation chain.".into(),
                &["voice"],
                &[AUD],
                "dsp.decim_stopband_db",
                Ge,
                60.0,
                "dB",
            ));
        }
        if self.has("speaker") {
            v.push(
                manual(
                    K::Functional,
                    M::Test,
                    "Spoken prompts shall reach at least 70 dBA at 30 cm.".into(),
                    "Audible over room noise; measure with a sound-level meter.".into(),
                    &["core"],
                    &[AUD],
                )
                .crit(Ge, 70.0, "dBA"),
            );
        }
        if self.has("imu") {
            v.push(manual(
                K::Performance,
                M::Test,
                "Tilt error shall be at most 2 degrees RMS during typical handling.".into(),
                "The Control Lab's tilt metric scores a synthetic head-motion scenario (the Iris use case), so this is verified on the bench with a reference.".into(),
                &["core"],
                &[SNS],
            ).crit(Le, 2.0, "deg"));
        }
        if self.has("env_sensors") {
            v.push(
                manual(
                    K::Performance,
                    M::Test,
                    "Temperature readings shall be within 0.5 °C of a reference thermometer from 0 to 40 °C.".into(),
                    "Self-heating of the board biases the reading: test in a climate chamber.".into(),
                    &["core"],
                    &[SNS],
                )
                .crit(Le, 0.5, "C"),
            );
        }
        if self.has("camera") {
            v.push(
                manual(
                    K::Functional,
                    M::Test,
                    "The camera shall capture a 1600 × 1200 still image within 1 s of a request.".into(),
                    "2 MP class sensor (UXGA).".into(),
                    &["core"],
                    &[SNS],
                )
                .crit(Le, 1.0, "s"),
            );
        }
        if self.has("motors") {
            if self.is("robot") {
                v.push(
                    manual(
                        K::Functional,
                        M::Demonstration,
                        "The robot shall drive at 0.3 m/s on a desk and stop within 50 mm.".into(),
                        "The motor sizing assumes this speed.".into(),
                        &["core"],
                        &[ACT],
                    )
                    .crit(Ge, 0.3, "m/s"),
                );
            } else {
                v.push(
                    manual(
                        K::Functional,
                        M::Test,
                        "The actuator shall complete a full stroke within 2 s.".into(),
                        "Sets the gear ratio; test with the design load.".into(),
                        &["core"],
                        &[ACT],
                    )
                    .crit(Le, 2.0, "s"),
                );
            }
        }
        if self.is("optical") {
            v.push(
                manual(
                    K::Performance,
                    M::Test,
                    "Illuminance shall be uniform within 20 % over a 20 mm field at 100 mm.".into(),
                    "The Optics Lab models the Iris head-up display, so optical performance is measured on the bench.".into(),
                    &["core"],
                    &[OPT],
                )
                .crit(Le, 20.0, "%"),
            );
            v.push(
                manual(
                    K::Performance,
                    M::Test,
                    "Detector signal-to-noise ratio shall be at least 40 dB on the reference target.".into(),
                    "Sets the gain of the transimpedance amplifier.".into(),
                    &["core"],
                    &[OPT],
                )
                .crit(Ge, 40.0, "dB"),
            );
        }
        // ---- environment, structure, manufacturing -----------------------------
        if self.has("waterproof") {
            v.push(manual(
                K::Environmental,
                M::Test,
                "The enclosure shall meet IP67 (dust-tight, 30 min at 1 m immersion) per IEC 60529.".into(),
                "Test 5 DVT units; check the test conditions in the standard.".into(),
                &["env"],
                &[STR],
            ));
        }
        if matches!(self.kind.id, "wearable" | "handheld" | "robot" | "optical" | "iot_sensor") {
            v.push(
                manual(
                    K::Environmental,
                    M::Test,
                    "The product shall keep working after 1 m drops onto concrete on each of its 6 faces.".into(),
                    "Everyday drops; the Structures Lab's drop analysis models the Iris frame, so this is a DVT test.".into(),
                    &["carry"],
                    &[if self.sub_exists(STR) { STR } else { PCB }],
                )
                .crit(Ge, 1.0, "m"),
            );
        }
        if self.is("mechanical") {
            let f = self.tv("load_n");
            v.push(manual(
                K::Performance,
                M::Test,
                format!("The bracket shall carry {} N at its tip for 1 h without cracking or permanent set.", fmt(f)),
                format!("Sized for a safety factor of 2 against yield ({} mm section); the Structures Lab's finite-element model is the Iris frame, so this is a load test.", fmt(self.enclosure.bracket_t_mm)),
                &["load"],
                &[STR],
            ).crit(Ge, f, "N"));
            v.push(manual(
                K::Performance,
                M::Test,
                format!("Tip deflection at {} N shall be at most {} mm.", fmt(f), fmt(nice_up((self.tv("length_mm") * 0.02).max(0.5)))),
                "2 % of the arm length; the beam estimate in the sizing notes sets the thickness to meet it (printed parts also creep: test after 24 h under load).".into(),
                &["load"],
                &[STR],
            ).crit(Le, nice_up((self.tv("length_mm") * 0.02).max(0.5)), "mm"));
        }
        if self.is("quantum") {
            let km = self.tv("fiber_km");
            v.push(bound(K::Performance, format!("The secret key rate at {} km shall be at least 1000 bit/s.", fmt(km)), "Rekeying AES-256 every second needs 256 bit/s; the rest is margin for finite-key effects and authentication. Verified by the Quantum Lab's decoy-state BB84 model.".into(), &["core"], &[QLINK, CMP, QRNG], "quantum.qkd_key_rate_bps", Ge, 1000.0, "bit/s"));
            v.push(bound(
                K::Performance,
                format!("The quantum bit error rate at {} km shall be at most 5 %.", fmt(km)),
                "A wide margin below the 11 % BB84 security threshold (Shor & Preskill 2000).".into(),
                &["core"],
                &[QLINK],
                "quantum.qkd_qber_pct",
                Le,
                5.0,
                "%",
            ));
            v.push(bound(
                K::Performance,
                format!(
                    "The maximum distance with a positive key rate shall be at least {} km.",
                    fmt(2.0 * km)
                ),
                "Twice the design distance covers splices, repairs and ageing.".into(),
                &["core"],
                &[QLINK],
                "quantum.qkd_max_distance_km",
                Ge,
                2.0 * km,
                "km",
            ));
            v.push(bound(
                K::Performance,
                "The QRNG shall deliver at least 3 bits of min-entropy per 4-bit sample.".into(),
                "Sets the Toeplitz extractor's output length (NIST SP 800-90B most-common-value estimate).".into(),
                &["random"],
                &[QRNG],
                "quantum.qrng_min_entropy_bits",
                Ge,
                3.0,
                "bit",
            ));
            v.push(bound(
                K::Performance,
                "The conditioned QRNG stream shall fail 0 of the NIST SP 800-22 tests run by the lab.".into(),
                "Statistical tests catch broken sources and wiring faults.".into(),
                &["random"],
                &[QRNG],
                "quantum.qrng_nist_failures",
                Le,
                0.0,
                "count",
            ));
            v.push(manual(
                K::Safety,
                M::Inspection,
                "The unattenuated laser shall stay inside an interlocked enclosure.".into(),
                "Laser safety and security: μ ≫ 1 breaks both.".into(),
                &["core"],
                &[STR],
            ));
        }
        if self.electronics && !self.is("quantum") {
            v.push(
                manual(
                    K::Manufacturing,
                    M::Inspection,
                    format!(
                        "The main board shall fit a {} × {} mm outline with {} copper layers.",
                        fmt(self.board.w),
                        fmt(self.board.h),
                        self.board.layers
                    ),
                    "From the board-area estimate (courtyards × routing factor).".into(),
                    &["build"],
                    &[PCB],
                )
                .crit(Le, self.board.w, "mm"),
            );
            let how = if self.has("wifi") {
                "Wi-Fi"
            } else if self.has("ble") {
                "BLE"
            } else {
                "USB"
            };
            v.push(manual(
                K::Functional,
                M::Demonstration,
                format!("Firmware shall be updatable in the field over {how} without opening the product."),
                "Bugs ship; recovering from a failed update must be demonstrated too.".into(),
                &["core", "build"],
                &[FW],
            ));
        }
        if self.is("dev_board") {
            v.push(manual(
                K::Interface,
                M::Inspection,
                "Every microcontroller GPIO shall be broken out to 2.54 mm headers.".into(),
                "Breadboard-friendly pitch.".into(),
                &["core"],
                &[PCB],
            ));
        }
        if self.enclosure.process == "printed" || self.enclosure.process == "molded" {
            let what = if self.enclosure.process == "printed" {
                "3-D printed PETG"
            } else {
                "injection-molded PC/ABS"
            };
            v.push(manual(
                K::Manufacturing,
                M::Inspection,
                format!(
                    "The {} shall be {what}, assembled with screws and no glue.",
                    if self.is("mechanical") { "part" } else { "enclosure" }
                ),
                "Serviceable and recyclable; the trade study compares the processes at the target quantity.".into(),
                &["build"],
                &[STR],
            ));
        }
        v
    }

    fn sub_exists(&self, id: &str) -> bool {
        match id {
            STR => self.enclosure.process != "none",
            PWR | CMP | PCB | FW => self.electronics && !(self.is("quantum") && (id == PCB || id == FW)),
            UI => self.has("display") || self.has("buttons") || self.has("leds"),
            AUD => self.has("microphones") || self.has("speaker"),
            SNS => self.has("imu") || self.has("env_sensors") || self.has("camera"),
            ACT => self.has("motors"),
            OPT => self.is("optical"),
            QRNG | QLINK => self.is("quantum"),
            MFG => true,
            _ => false,
        }
    }

    fn risks(&self) -> Vec<Risk> {
        type Row<'a> = (&'a str, &'a str, &'a str, &'a str, u8, u8, u8, &'a str);
        let mut v: Vec<Row> = vec![];
        match self.battery.source {
            Source::Lipo => v.push((
                "LiPo cell",
                "Swelling or thermal runaway",
                "Fire, burns",
                "Over-charge, puncture in a drop, heat",
                10,
                2,
                4,
                "Protected cell tested to IEC 62133-2, charger temperature cut-off, cradle that spreads crush loads.",
            )),
            Source::Coin(_) => v.push((
                "Coin cell",
                "Voltage sag during radio bursts",
                "Brown-out reset, lost data",
                "High internal resistance when cold",
                6,
                4,
                4,
                "Bulk capacitor across the cell, spread radio events, brown-out test at 0 °C.",
            )),
            Source::None => {}
        }
        if self.has("usb_c") {
            v.push((
                "USB-C receptacle",
                "Connector cracks off the board",
                "No power or charging",
                "Lever forces on the plug",
                6,
                3,
                4,
                "Through-hole shell tabs, receptacle backed by the enclosure, 5,000-cycle insertion test.",
            ));
        }
        if self.has("dc_adapter") {
            v.push((
                "DC input",
                "Wrong adapter plugged in",
                "Damaged board",
                "Reverse polarity or over-voltage",
                6,
                3,
                3,
                "Reverse-polarity MOSFET and a TVS diode; label the input voltage.",
            ));
        }
        if self.radio() {
            v.push((
                "Radio",
                "Connection drops in crowded 2.4 GHz",
                "Lost data, angry users",
                "Interference, body blocking, detuned antenna",
                5,
                5,
                4,
                "Keep metal and batteries clear of the antenna, tune the match on the real enclosure, retry with back-off.",
            ));
        }
        if self.has("display") {
            v.push((
                "Display",
                "Cracked panel",
                "Product unusable",
                "Drop on a corner",
                7,
                3,
                3,
                "Recess the panel below the bezel, foam backing, corner drop test.",
            ));
        }
        if self.has("motors") {
            v.push((
                "Motor driver",
                "Overheats at stall",
                "Shutdown, hot surface",
                "Jammed wheel or linkage",
                6,
                3,
                4,
                "Current limit, stall detection from encoder speed, thermal pad to the chassis.",
            ));
        }
        if self.has("microphones") {
            v.push((
                "Microphones",
                "Port blocked by dust or sweat",
                "Voice commands missed",
                "No mesh over the port",
                4,
                4,
                5,
                "Acoustic mesh, port away from the grip.",
            ));
        }
        if self.has("camera") {
            v.push((
                "Camera",
                "Images captured without consent",
                "Privacy complaint, recall",
                "No capture indicator",
                8,
                3,
                6,
                "Hard-wired capture LED, privacy shutter, on-device processing by default.",
            ));
        }
        if self.has("waterproof") {
            v.push((
                "Gasket",
                "Leak after a drop",
                "Corroded board",
                "Gasket pinched or housing distorted",
                7,
                3,
                5,
                "Gasket groove to the supplier's compression range, vacuum leak test at end of line.",
            ));
        }
        if self.enclosure.process == "printed" {
            v.push((
                "Printed part",
                "Layers split under load",
                if self.is("mechanical") { "Load falls" } else { "Housing opens" },
                "Load across the layer lines, under-extrusion",
                if self.is("mechanical") { 8 } else { 5 },
                4,
                4,
                "Orient so loads run along the layers, 4+ perimeters, load test every print batch.",
            ));
        } else if self.enclosure.process == "molded" {
            v.push((
                "Molded part",
                "Sink marks, warp or short shots",
                "Cosmetic rejects, poor fit",
                "Uneven walls, no draft",
                4,
                4,
                3,
                "Uniform walls, 1–2° draft, Molding Lab DFM check before cutting steel.",
            ));
        }
        if self.electronics && !self.is("quantum") {
            v.push((
                "Firmware",
                "Update bricks the device",
                "Product dead in the field",
                "Power loss during update",
                7,
                3,
                3,
                "A/B firmware slots with rollback; test power-cut during update.",
            ));
            if self.kind.contact == Contact::Skin {
                v.push((
                    "Skin contact",
                    "Surface gets uncomfortably warm",
                    "Discomfort, returns",
                    "Radio or display at full duty in a hot room",
                    5,
                    4,
                    4,
                    "Thermal model in the Power & Thermal lab, heat spreader, duty limits when hot.",
                ));
            }
        }
        if self.is("mechanical") {
            v.push((
                "Mounting screws",
                "Pull out of the wall",
                "Load falls",
                "Wrong anchor for the wall material",
                8,
                3,
                5,
                "Specify anchors per wall type in the instructions; proof-load test.",
            ));
        }
        if self.is("quantum") {
            v.push((
                "Single-photon detectors",
                "Afterpulsing and dark counts raise the QBER",
                "Lower key rate, possible abort",
                "Too short dead time or too warm",
                6,
                5,
                3,
                "Characterise afterpulsing, set dead time, monitor dark counts every session.",
            ));
            v.push((
                "Interferometers",
                "Phase drift between the nodes",
                "Error rate rises",
                "Temperature changes",
                6,
                6,
                3,
                "Temperature stabilisation and active phase tracking.",
            ));
            v.push((
                "QRNG",
                "Entropy source degrades silently",
                "Predictable basis choices break security",
                "Diode ageing, ADC clipping",
                9,
                3,
                4,
                "Continuous health tests (NIST SP 800-90B) and a stop on failure.",
            ));
        }
        v.into_iter()
            .enumerate()
            .map(|(i, (item, fm, eff, cause, s, o, d, mit))| Risk {
                id: format!("F-{}", i + 1),
                item: item.into(),
                failure_mode: fm.into(),
                effect: eff.into(),
                cause: cause.into(),
                severity: s,
                occurrence: o,
                detection: d,
                mitigation: mit.into(),
            })
            .collect()
    }

    fn trade(&self) -> TradeStudy {
        let crit = |n: &str, w: f64| Criterion {
            name: n.into(),
            weight: w,
            higher_is_better: true,
        };
        let mut t = if self.is("quantum") {
            TradeStudy {
                id: "T-1".into(),
                title: "Single-photon detector technology".into(),
                decision: String::new(),
                criteria: vec![
                    crit("Detection efficiency", 0.25),
                    crit("Dark counts and afterpulsing", 0.2),
                    crit("Cost", 0.25),
                    crit("Power, size and operating effort", 0.3),
                ],
                options: vec![
                    TradeOption {
                        name: "Gated InGaAs APD".into(),
                        scores: vec![5.0, 5.0, 8.0, 8.0],
                        notes: "Typical 10–30 % efficiency; thermoelectric cooling.".into(),
                    },
                    TradeOption {
                        name: "Superconducting nanowire (SNSPD)".into(),
                        scores: vec![10.0, 10.0, 2.0, 2.0],
                        notes: "Typical > 80 % efficiency, needs a ~2–3 K cryostat.".into(),
                    },
                    TradeOption {
                        name: "Free-running InGaAs APD".into(),
                        scores: vec![4.0, 4.0, 7.0, 7.0],
                        notes: "No gating electronics; higher dark counts.".into(),
                    },
                ],
            }
        } else if self.has("battery") {
            // three neighbouring pouch sizes, scored from computed life, mass, cost and charge time
            let sizes = super::parts::LIPO_SIZES;
            let i = sizes.iter().position(|s| *s == self.battery.mah).unwrap_or(0);
            let picks: Vec<f64> = [i.saturating_sub(1), i, (i + 1).min(sizes.len() - 1)]
                .iter()
                .map(|&k| sizes[k])
                .collect::<Vec<_>>();
            let mut picks = picks;
            picks.dedup();
            let base_mass: f64 = self.parts.iter().filter(|p| p.refdes != "BT1").map(|p| p.mass * p.qty).sum();
            let life_t = self.tv("life_h");
            let rows: Vec<(f64, f64, f64, f64, f64)> = picks
                .iter()
                .map(|&c| {
                    let life = c * 3.7 * 0.85 / self.avg_mw.max(1e-9);
                    let mass = base_mass + c * 3.7 / super::parts::lipo_mwh_per_g(c);
                    let cost = super::parts::lipo_cost(c);
                    let chg = 1.2 * c / (0.5 * c).min(500.0);
                    (c, life, mass, cost, chg)
                })
                .collect();
            // must meet the requirement (life × 1.2 and cycle life) to score well
            let need_h = (life_t * 1.2).max(self.cycle_life_h());
            let lifescore = |life: f64| {
                let r = life / need_h;
                round_to(
                    if r >= 1.0 {
                        10.0
                    } else {
                        (1.0 + 4.0 * (r - 0.6) / 0.4).clamp(1.0, 5.0)
                    },
                    1,
                )
            };
            let (mmin, mmax) = rows.iter().fold((f64::MAX, f64::MIN), |a, r| (a.0.min(r.2), a.1.max(r.2)));
            let (cmin, cmax) = rows.iter().fold((f64::MAX, f64::MIN), |a, r| (a.0.min(r.3), a.1.max(r.3)));
            let (hmin, hmax) = rows.iter().fold((f64::MAX, f64::MIN), |a, r| (a.0.min(r.4), a.1.max(r.4)));
            TradeStudy {
                id: "T-1".into(),
                title: "Battery capacity".into(),
                decision: String::new(),
                criteria: vec![
                    crit("Meets life and cycle-life need", 0.45),
                    crit("Product mass", 0.3),
                    crit("Cell cost", 0.15),
                    crit("Charge time", 0.1),
                ],
                options: rows
                    .iter()
                    .map(|&(c, life, mass, cost, chg)| TradeOption {
                        name: format!("{} mAh LiPo", fmt(c)),
                        scores: vec![
                            lifescore(life),
                            score(mass, mmin, mmax),
                            score(cost, cmin, cmax),
                            score(chg, hmin, hmax),
                        ],
                        notes: format!(
                            "{} h life, {} g product, {} USD cell, {} h charge (computed)",
                            fmt(life),
                            fmt(mass),
                            fmt(cost),
                            fmt(chg)
                        ),
                    })
                    .collect(),
            }
        } else if self.has("coin_cell") {
            // the same coin classes (and usable fraction) the sizing picks from
            let mut rows: Vec<(String, f64, f64, f64)> = super::parts::COINS
                .iter()
                .map(|c| (format!("{} coin cell", c.name), c.mah * 3.0 * 0.85, c.mass_g, c.cost))
                .collect();
            rows.push(("2 × AAA alkaline cells".into(), 1100.0 * 3.0 * 0.8, 23.0, 0.8));
            let life_t = self.tv("life_h");
            let lives: Vec<f64> = rows.iter().map(|r| r.1 / self.avg_mw.max(1e-9)).collect();
            let lifescore = |life: f64| {
                let r = life / (life_t * 1.2);
                round_to(
                    if r >= 1.0 {
                        10.0
                    } else {
                        (1.0 + 4.0 * (r - 0.6) / 0.4).clamp(1.0, 5.0)
                    },
                    1,
                )
            };
            TradeStudy {
                id: "T-1".into(),
                title: "Primary cell".into(),
                decision: String::new(),
                criteria: vec![
                    crit("Meets the life target (with 20 % allowance)", 0.45),
                    crit("Mass and size", 0.35),
                    crit("Cost", 0.2),
                ],
                options: rows
                    .iter()
                    .zip(&lives)
                    .map(|(r, life)| TradeOption {
                        name: r.0.clone(),
                        scores: vec![lifescore(*life), score(r.2, 3.0, 23.0), score(r.3, 0.3, 0.8)],
                        notes: format!(
                            "{} h at {} mW (computed); {} g, {} USD typical",
                            fmt(*life),
                            fmt(self.avg_mw),
                            fmt(r.2),
                            fmt(r.3)
                        ),
                    })
                    .collect(),
            }
        } else {
            // enclosure / part process at the target quantity
            let q = self.tv("quantity");
            let e = &self.enclosure;
            let (l, w, h) = (self.tv("length_mm"), self.tv("width_mm"), self.tv("height_mm"));
            let vol = if e.volume_cm3 > 0.0 {
                e.volume_cm3
            } else {
                2.0 * (l * w + l * h + w * h) * 1.5 * 1.15 / 1000.0
            };
            let (pc, _) = process_cost("printed", vol, vol * 1.27, l.max(w), q);
            let (mc, tool) = process_cost("molded", vol, vol * 1.15, l.max(w), q);
            let molded_unit = mc + tool / q.max(1.0);
            let cnc = 25.0 + vol * 0.5;
            let costs = [pc, molded_unit, cnc];
            let (lo, hi) = costs.iter().fold((f64::MAX, f64::MIN), |a, c| (a.0.min(*c), a.1.max(*c)));
            let s = |c: f64| score(c, lo, hi);
            TradeStudy {
                id: "T-1".into(),
                title: if self.is("mechanical") {
                    "Manufacturing process".into()
                } else {
                    "Enclosure process".into()
                },
                decision: String::new(),
                criteria: vec![
                    crit(&format!("Unit cost at {} units", fmt(q)), 0.45),
                    crit("Lead time to first parts", 0.2),
                    crit("Finish and strength", 0.2),
                    crit("Freedom to change the design", 0.15),
                ],
                options: vec![
                    TradeOption {
                        name: "3-D printing (PETG)".into(),
                        scores: vec![s(pc), 9.0, 5.0, 10.0],
                        notes: format!("≈ {} USD per set, no tooling (computed, typical rates)", fmt(pc)),
                    },
                    TradeOption {
                        name: "Injection molding (PC/ABS)".into(),
                        scores: vec![s(molded_unit), 3.0, 9.0, 2.0],
                        notes: format!(
                            "≈ {} USD piece + {} USD tooling ÷ {} units = {} USD (computed, typical rates)",
                            fmt(mc),
                            fmt(tool),
                            fmt(q),
                            fmt(molded_unit)
                        ),
                    },
                    TradeOption {
                        name: "CNC-machined aluminium".into(),
                        scores: vec![s(cnc), 6.0, 10.0, 7.0],
                        notes: format!("≈ {} USD per set (typical job-shop rate)", fmt(cnc)),
                    },
                ],
            }
        };
        let r = crate::systems::score_trade(&t);
        t.decision = t.options[r.winner].name.clone();
        t
    }

    fn milestones(&self) -> Vec<Milestone> {
        let mut evt = vec!["Functional prototype demonstrated".to_string()];
        let mut dvt = vec!["Every test requirement passed on DVT units".to_string()];
        if self.has_battery() {
            evt.push(format!("Battery life measured on 3 units ≥ {} h", fmt(self.tv("life_h"))));
        }
        if self.radio() {
            dvt.push("Radiated-emissions pre-scan passed".into());
        }
        if self.has("waterproof") {
            dvt.push("IP67 immersion passed on 5 units".into());
        }
        if matches!(self.kind.id, "wearable" | "handheld" | "robot" | "optical" | "iot_sensor") {
            dvt.push("Drop test passed".into());
        }
        if self.is("mechanical") {
            dvt.push(format!("{} N proof-load test passed on 5 parts", fmt(self.tv("load_n"))));
        }
        let pilot = (self.tv("quantity") * 0.05).clamp(1.0, 100.0).round();
        let ms = |id: &str, title: &str, stage: &str, del: &[&str], exit: Vec<String>| Milestone {
            id: id.into(),
            title: title.into(),
            stage: stage.into(),
            deliverables: del.iter().map(|s| s.to_string()).collect(),
            exit_criteria: exit,
        };
        vec![
            ms(
                "M-0",
                "Concept review",
                "Concept",
                &["Needs", "Use cases", "Trade study"],
                vec!["Top trade decided".into(), "Every need traced to a requirement".into()],
            ),
            ms(
                "M-1",
                "Requirements review",
                "Requirements",
                &["Requirement set", "Architecture"],
                vec!["Every requirement has a verification method and an owner".into()],
            ),
            ms(
                "M-2",
                "Preliminary design review",
                "Design",
                &["Simulations passing", "CAD v1", "Schematic"],
                vec![
                    "All analysis requirements green".into(),
                    "No high risks without mitigation (RPN)".into(),
                ],
            ),
            ms(
                "M-3",
                "EVT: engineering validation",
                "Prototype",
                &["First prototypes", "Assembled boards"],
                evt,
            ),
            ms(
                "M-4",
                "DVT: design validation",
                "Validation",
                &["Production-intent parts", "Test reports"],
                dvt,
            ),
            ms(
                "M-5",
                "PVT: production validation",
                "Production",
                &["Production tooling", "Line fixtures", "Pilot run"],
                vec![format!("Yield above 95 % on a {}-unit pilot run", fmt(pilot))],
            ),
        ]
    }

    fn concept(&self) -> Concept {
        let users = if self.users.is_empty() {
            match self.kind.id {
                "quantum" => vec!["Researchers and students".to_string(), "Lab technicians".into()],
                "dev_board" => vec!["Students".to_string(), "Makers".into()],
                "mechanical" => vec!["Installers".to_string(), "Makers".into()],
                _ => vec!["Primary user".to_string()],
            }
        } else {
            self.users.clone()
        };
        let idea = if self.idea.is_empty() {
            self.kind.blurb.trim_end_matches('.').to_string()
        } else {
            self.idea.clone()
        };
        let mut uc = vec![(
            "Everyday use".to_string(),
            format!("{} uses {} as intended: {}.", users[0], self.name, lower_first(&idea)),
        )];
        if self.has_battery() {
            uc.push((
                if self.has("battery") { "Charge" } else { "Change the cell" }.into(),
                if self.has("battery") {
                    format!(
                        "Plug into USB-C; full again in about {} h.",
                        fmt(1.2 * self.battery.mah / (0.5 * self.battery.mah).min(500.0))
                    )
                } else {
                    "Open the tool-secured door and swap the coin cell once its life is up.".into()
                },
            ));
        }
        if self.radio() {
            uc.push((
                "Connect".into(),
                if self.has("wifi") {
                    "Join the home network once; data and updates flow over Wi-Fi.".into()
                } else {
                    "Pair with the phone app over Bluetooth Low Energy.".into()
                },
            ));
        }
        let mut constraints = vec![
            format!(
                "Fits within {} × {} × {} mm",
                fmt(self.tv("length_mm")),
                fmt(self.tv("width_mm")),
                fmt(self.tv("height_mm"))
            ),
            format!("At most {} g", fmt(self.tv("mass_g"))),
        ];
        if !self.kind.scope.is_empty() {
            constraints.push(self.kind.scope.to_string());
        }
        constraints.push("Part values are typical planning numbers — replace them with data-sheet values".into());
        Concept {
            problem: format!("{} want {}.", users.join(", "), lower_first(&idea)),
            vision: format!("{}: {}.", self.name, idea),
            users,
            use_cases: uc
                .into_iter()
                .enumerate()
                .map(|(i, (t, n))| UseCase {
                    id: format!("UC-{}", i + 1),
                    title: t,
                    narrative: n,
                })
                .collect(),
            constraints,
        }
    }

    fn design(&self) -> Design {
        let mut d = Design::default();
        if !self.electronics {
            d.battery = self.battery.params.clone();
            d.power_rails.quiescent_ua = 0.0;
        } else {
            d.battery = self.battery.params.clone();
            d.power_rails = PowerRails {
                core_v: self.core_v(),
                buck_efficiency: self.regulator_eff(),
                display_rail_v: 3.3,
                quiescent_ua: self.quiescent_ua(),
            };
        }
        if self.board.w > 0.0 {
            d.pcb = PcbParams {
                width_mm: self.board.w,
                height_mm: self.board.h,
                layers: self.board.layers,
                thickness_mm: self.board.thickness,
                ..PcbParams::default()
            };
        }
        if self.has("microphones") {
            d.audio.mic_count = 2;
            d.audio.mic_spacing_mm = round_to((self.tv("length_mm") * 0.4).clamp(8.0, 40.0), 1);
        }
        // lumped thermal network from the enclosure (or the bare board)
        let skin = self.kind.contact == Contact::Skin;
        let area_m2 = if self.enclosure.process == "none" {
            2.0 * self.board.w * self.board.h * 1e-6
        } else {
            self.enclosure.surface_mm2 * 1e-6
        } * if skin { 0.7 } else { 1.0 };
        let r_pa = (1.0 / (10.0 * area_m2.max(1e-6))).clamp(0.05, 500.0);
        let board_cm2 = (self.board.w * self.board.h / 100.0).max(1.0);
        let r_bp = if self.is("quantum") {
            0.1
        } else if self.enclosure.process == "none" {
            2.0
        } else {
            (20.0 * (20.0 / board_cm2).sqrt()).clamp(1.0, 40.0)
        };
        let shell = self.enclosure.mass_g + self.battery.params.mass_g;
        let pcb_g = self.parts.iter().find(|p| p.refdes == "PCB1").map(|p| p.mass).unwrap_or(2.0);
        d.thermal = ThermalParams {
            ambient_c: self.tv("ambient_c"),
            r_pod_ambient_c_per_w: round_to(r_pa, 2),
            r_board_pod_c_per_w: round_to(r_bp, 2),
            c_pod_j_per_c: round_to((self.enclosure.mass_g * 1.3 + self.battery.params.mass_g).max(2.0), 2),
            c_board_j_per_c: round_to(pcb_g * 1.1 + 0.5, 2),
            skin_limit_c: self.kind.contact.surface_limit_c(),
        };
        let _ = shell;
        let q = self.tv("quantity");
        let mut qs = vec![1.0, 10.0, 100.0, 1000.0, 10000.0];
        if !qs.contains(&q) {
            qs.push(q);
            qs.sort_by(|a, b| a.total_cmp(b));
        }
        let offboard = self.parts.iter().filter(|p| !p.board).count() as f64;
        d.manufacturing = MfgParams {
            quantities: qs,
            target_quantity: q,
            molding_material: if self.enclosure.process == "molded" {
                "PC".into()
            } else {
                MfgParams::default().molding_material
            },
            cavities: if q >= 20000.0 { 2 } else { 1 },
            assembly_minutes: round_to(4.0 + 0.75 * offboard, 1),
            ..MfgParams::default()
        };
        let mut extra = serde_json::Map::new();
        extra.insert("template".into(), json!("wizard"));
        extra.insert(
            "wizard".into(),
            json!({
                "version": 1,
                "kind": self.kind.id,
                "capabilities": self.caps,
                "targets": self.t,
            }),
        );
        if self.electronics {
            extra.insert(
                "reliability".into(),
                json!({
                    "mission_years": self.tv("design_years"),
                    "hours_per_day": self.tv("hours_per_day"),
                    "hinge_cycles_per_day": 0.0,
                    "mic_k": if self.has("microphones") { 2 } else { 1 },
                }),
            );
        }
        extra.insert(
            "program".into(),
            json!({ "economics": { "retail_price_usd": self.tv("retail_usd") } }),
        );
        if self.has("ble") {
            extra.insert("rf".into(), serde_json::to_value(self.rf_design_with(&d)).unwrap_or_default());
        }
        if self.is("quantum") {
            extra.insert(
                "qkd".into(),
                json!({
                    "distance_km": self.tv("fiber_km"), "fiber_loss_db_km": 0.2, "extra_loss_db": 3.0, "detector_efficiency": 0.2,
                    "dark_count_prob": 1e-6, "misalignment": 0.015, "ec_efficiency": 1.16, "source_rate_hz": 1e7,
                    "mu": -1.0, "sifting": 0.5, "dead_time_s": 1e-6, "random_bits_per_pulse": 3.0
                }),
            );
            extra.insert(
                "qrng".into(),
                json!({
                    "source": "diode", "samples": 20000, "seed": 42,
                    "noise_rms_mv": 40.0, "flicker_rms_mv": 4.0, "offset_mv": 3.0, "hum_mv": 1.0, "hum_hz": 60.0, "bandwidth_hz": 0.0,
                    "sample_rate_hz": 2e7, "adc_bits": 8, "adc_vpp_mv": 400.0, "digitize": "lsb", "lsb_bits": 4,
                    "von_neumann": false, "xor_fold": 1, "toeplitz": true, "toeplitz_block": 1024, "security_bits": 64.0
                }),
            );
        }
        d.extra = extra;
        d
    }

    /// RF design block: link at the range target, losses by how the product is used,
    /// antenna efficiency by board size, the RF lab's own matching network and 50 Ω feed.
    pub fn rf_design_with(&self, d: &Design) -> crate::rf::IrisRf {
        let mut rf = crate::rf::IrisRf::default();
        rf.link.distance_m = self.tv("range_m");
        let (head, body) = match self.kind.id {
            "wearable" => (6.0, 4.0),
            "handheld" | "optical" => (0.0, 3.0),
            _ => (0.0, 0.0),
        };
        rf.link.head_loss_db = head;
        rf.link.body_loss_db = body;
        rf.rad_efficiency = if self.board.w * self.board.h >= 1500.0 { 0.5 } else { 0.3 };
        rf.network = Some(self.rf_network());
        let mut probe = Project::blank("rf");
        probe.design.pcb = d.pcb.clone();
        rf.feed = Some(crate::rf::recommended_feed(&probe, &rf));
        rf
    }

    /// Build the project.
    pub fn assemble(&self) -> Project {
        let needs_raw = self.need_list();
        let need_id = |key: &str| needs_raw.iter().position(|n| n.0 == key).map(|i| format!("N-{}", i + 1));
        let subsystems = self.subsystems();
        let interfaces = self.interfaces(&subsystems);
        let reqs = self.reqs();
        let requirements: Vec<Requirement> = reqs
            .into_iter()
            .enumerate()
            .map(|(i, r)| Requirement {
                id: format!("R-{:03}", i + 1),
                text: r.text,
                rationale: r.rationale,
                kind: r.kind,
                derives_from: r.needs.iter().filter_map(|k| need_id(k)).collect(),
                allocated_to: r
                    .subs
                    .iter()
                    .filter(|s| subsystems.iter().any(|x| x.id == **s))
                    .map(|s| s.to_string())
                    .collect(),
                verification: Verification {
                    method: r.method,
                    metric: r.metric.map(String::from),
                    cmp: r.cmp,
                    target: r.target,
                    target_hi: None,
                    unit: r.unit.into(),
                },
            })
            .collect();
        let needs: Vec<Need> = needs_raw
            .iter()
            .enumerate()
            .map(|(i, (_, text, who, prio))| Need {
                id: format!("N-{}", i + 1),
                text: text.clone(),
                stakeholder: who.to_string(),
                priority: *prio,
            })
            .collect();
        let caps_text = self
            .caps
            .iter()
            .filter_map(|c| catalog::cap(c))
            .map(|c| c.name.to_lowercase())
            .collect::<Vec<_>>()
            .join(", ");
        Project {
            meta: Meta {
                id: slug(&self.name),
                name: self.name.clone(),
                codename: self.name.split_whitespace().next().unwrap_or("NEW").chars().filter(|c| c.is_alphanumeric()).take(12).collect::<String>().to_uppercase(),
                version: "0.1.0".into(),
                tagline: if self.idea.is_empty() { self.kind.blurb.into() } else { self.idea.clone() },
                description: format!(
                    "A {} starter project generated by the new-product wizard ({caps_text}). Requirements bound to metrics are verified live by the labs that compute them from this project's own data; the rest are verified by test, inspection or demonstration. Part values are typical — replace them with data-sheet numbers.",
                    self.kind.name.to_lowercase()
                ),
                author: self.author.clone(),
            },
            concept: self.concept(),
            needs,
            requirements,
            subsystems,
            interfaces,
            components: self.parts.iter().map(|p| p.component()).collect(),
            trades: vec![self.trade()],
            risks: self.risks(),
            milestones: self.milestones(),
            design: self.design(),
        }
    }
}
