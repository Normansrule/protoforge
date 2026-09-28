//! Starter projects for "build anything from scratch".
//!
//! * **Blank** – an empty product with the standard stage-gate milestones.
//! * **Iris smart glasses** – the full reference project.
//! * **Pebble sensor tag** – a coin-cell Bluetooth Low Energy (BLE) tag: a
//!   small, low-power product whose requirements are verified by the
//!   systems roll-ups (battery life, mass, cost) and the PCB, robotics and
//!   manufacturing labs.
//! * **Pip desk robot** – a two-wheeled self-balancing desk robot with a
//!   display face and voice input, exercising the control, DSP, circuit,
//!   PCB and reinforcement-learning labs.
//!
//! The small templates reuse Iris reference designators for the parts they
//! share (SoC `U1`, inertial measurement unit (IMU) `U3`, decoupling and
//! pull-ups) so the PCB lab's schematic table can connect them. Their board
//! parts start with `on_pcb = false`: the PCB lab's placement is tuned for
//! the Iris board outline and routing a new outline from scratch takes
//! seconds, which would stall live verification. The PCB requirements stay
//! bound to `pcb.*` metrics and read "unverified" until the parts are placed.
//! Prices and masses are round planning numbers for learning, not quotes.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::project::Project;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Icon name understood by the web app.
    pub icon: String,
    pub tags: Vec<String>,
    pub project: Project,
}

/// Standard stage-gate milestones (concept → production), with exit
/// criteria the Systems Studio can check automatically where possible.
pub fn standard_milestones() -> Value {
    json!([
        { "id": "M-0", "title": "Concept review", "stage": "Concept",
          "deliverables": ["Needs", "Use cases", "Trade studies"],
          "exit_criteria": ["Top trades decided", "Every need traced to a requirement"] },
        { "id": "M-1", "title": "Requirements review", "stage": "Requirements",
          "deliverables": ["Verified requirement set", "Architecture"],
          "exit_criteria": ["Every requirement has a metric and an owner"] },
        { "id": "M-2", "title": "Preliminary design review", "stage": "Design",
          "deliverables": ["Simulations passing", "CAD v1", "Schematic"],
          "exit_criteria": ["All analysis requirements green", "No high risks without mitigation (RPN)"] },
        { "id": "M-3", "title": "EVT: engineering validation", "stage": "Prototype",
          "deliverables": ["First prototypes", "Assembled PCBs"],
          "exit_criteria": ["Functional prototype demonstrated"] },
        { "id": "M-4", "title": "DVT: design validation", "stage": "Validation",
          "deliverables": ["Soft-tooled parts", "Environmental and drop tests"],
          "exit_criteria": ["Tests pass", "Every need satisfied"] },
        { "id": "M-5", "title": "PVT: production validation", "stage": "Production",
          "deliverables": ["Hard tooling", "Line fixtures", "First production units"],
          "exit_criteria": ["Yield above 95 %"] }
    ])
}

fn blank() -> Project {
    let mut p = Project::blank("New product");
    p.meta.codename = "NEW".into();
    p.meta.description = "Start here: write the problem and the stakeholder needs, derive requirements, sketch subsystems and interfaces, then bind each requirement to a lab metric.".into();
    p.milestones = serde_json::from_value(standard_milestones()).expect("milestones parse");
    p
}

/// Build a project from JSON, filling unspecified design sections with defaults.
fn from_json(v: Value) -> Project {
    serde_json::from_value(v).expect("template must parse")
}

/// A BOM line. `board` marks parts meant for the PCB (see the module docs:
/// they start off the board so live verification stays fast).
#[allow(clippy::too_many_arguments)]
fn comp(refdes: &str, name: &str, class: &str, sub: &str, cost: f64, mass: f64, power: [f64; 3], footprint: &str, board: bool) -> Value {
    json!({
        "refdes": refdes, "name": name, "part_class": class, "example_part": "", "subsystem": sub,
        "qty": 1, "unit_cost_usd": cost, "mass_g": mass,
        "power": { "active_mw": power[0], "sleep_mw": power[1], "duty": power[2] },
        "footprint": footprint, "on_pcb": false, "location": if board { "board" } else { "enclosure" }
    })
}

#[allow(clippy::too_many_arguments)]
fn req(
    id: &str,
    kind: &str,
    text: &str,
    rationale: &str,
    needs: &[&str],
    subs: &[&str],
    metric: Option<&str>,
    cmp: &str,
    target: f64,
    unit: &str,
) -> Value {
    json!({
        "id": id, "text": text, "rationale": rationale, "kind": kind,
        "derives_from": needs, "allocated_to": subs,
        "verification": {
            "method": if metric.is_some() { "analysis" } else { "test" },
            "metric": metric, "cmp": cmp, "target": target, "target_hi": null, "unit": unit
        }
    })
}

const Z: [f64; 3] = [0.0, 0.0, 0.0];

/// Pebble: coin-cell BLE sensor tag.
pub fn pebble() -> Project {
    from_json(json!({
        "meta": {
            "id": "pebble-tag", "name": "Pebble Sensor Tag", "codename": "PEBBLE", "version": "0.1.0",
            "tagline": "A coin-cell Bluetooth sensor tag that clips to a bag, bike or collar",
            "description": "A 30 mm tag that advertises over Bluetooth Low Energy (BLE), wakes on motion and logs temperature. A small, low-power starter product: its requirements are verified by the power, mass and cost roll-ups and by the PCB, robotics and manufacturing labs.",
            "author": ""
        },
        "concept": {
            "problem": "People lose track of bags, bikes and pets, and cheap trackers say nothing about what the item went through.",
            "vision": "A tag the size of a coin that lasts a year on one cell, notices motion, logs temperature and is cheap enough to put on everything.",
            "users": ["Students tracking backpacks and bikes", "Pet owners", "Makers learning low-power design"],
            "use_cases": [
                { "id": "UC-1", "title": "Find my bag", "narrative": "The phone listens for the tag's advertising packets and shows how close it is from the received signal strength." },
                { "id": "UC-2", "title": "Motion log", "narrative": "The accelerometer wakes the SoC on motion; the tag counts and time-stamps events for later download." },
                { "id": "UC-3", "title": "Condition log", "narrative": "Temperature is sampled every 10 minutes and kept in a ring buffer." }
            ],
            "constraints": ["Runs on one CR2032-class coin cell", "At most 12 g with cell and enclosure", "2-layer PCB, hand-solderable prototype"]
        },
        "needs": [
            { "id": "N-1", "text": "Runs for a year without changing the battery", "stakeholder": "User", "priority": 1 },
            { "id": "N-2", "text": "Light enough to forget it is attached", "stakeholder": "User", "priority": 1 },
            { "id": "N-3", "text": "Cheap enough to put on everything", "stakeholder": "Business", "priority": 1 },
            { "id": "N-4", "text": "Notices when the item moves", "stakeholder": "User", "priority": 2 },
            { "id": "N-5", "text": "Simple to build by hand and to assemble in volume", "stakeholder": "Maker / Business", "priority": 2 }
        ],
        "requirements": [
            req("R-001", "performance", "Battery life in the reference advertising profile shall be at least 8760 h (one year).", "N-1 asks for a year; 8760 h = 365 days.", &["N-1"], &["SYS-PWR", "SYS-RF"], Some("budget.battery_life_h"), "ge", 8760.0, "h"),
            req("R-002", "performance", "Average battery power shall be at most 0.07 mW.", "A 220 mAh, 3 V cell holds about 560 mWh usable: one year needs ≤ 0.064 mW; the limit carries the allocation for the radio and the sensor.", &["N-1"], &["SYS-PWR", "SYS-RF", "SYS-SNS"], Some("budget.avg_power_mw"), "le", 0.07, "mW"),
            req("R-003", "performance", "Total tag mass including the cell and enclosure shall be at most 12 g.", "About the mass of a house key.", &["N-2"], &["SYS-ENC", "SYS-PWR"], Some("mass.components_g"), "le", 12.0, "g"),
            req("R-004", "cost", "Bill of materials cost at quantity-1 prices shall be at most 8 USD.", "Leaves room for a retail price under 20 USD.", &["N-3"], &["SYS-MFG"], Some("bom.cost_usd"), "le", 8.0, "USD"),
            req("R-005", "manufacturing", "All PCB connections shall be routed.", "An unrouted board cannot be ordered. Mark the board parts on-PCB to lay them out in PCB Studio.", &["N-5"], &["SYS-PCB"], Some("pcb.routed_pct"), "ge", 100.0, "%"),
            req("R-006", "manufacturing", "Design-rule check shall report 0 violations.", "Standard prototype-fab rules (0.15 mm trace/space).", &["N-5"], &["SYS-PCB"], Some("pcb.drc_violations"), "le", 0.0, "count"),
            req("R-007", "manufacturing", "Automated pick-and-place of one board shall take at most 20 s.", "Keeps assembly cost per tag low at volume.", &["N-5", "N-3"], &["SYS-MFG"], Some("robotics.pnp_cycle_s"), "le", 20.0, "s"),
            req("R-008", "performance", "Tilt estimate error shall be at most 3 degrees RMS during handling.", "Distinguishes a bag being picked up from vibration.", &["N-4"], &["SYS-SNS"], Some("control.head_rms_err_deg"), "le", 3.0, "deg"),
            req("R-009", "functional", "The tag shall wake from sleep within 50 ms of a motion event.", "Verified by bench test with a logic analyser on the interrupt line.", &["N-4"], &["SYS-SNS", "SYS-FW"], None, "le", 50.0, "ms")
        ],
        "subsystems": [
            { "id": "SYS-PWR", "name": "Coin cell & power", "domain": "electrical", "description": "CR2032-class cell, reverse-polarity protection, decoupling.", "functions": ["Store energy", "Supply the SoC and sensor"], "lab": "power" },
            { "id": "SYS-RF", "name": "BLE SoC & antenna", "domain": "electrical", "description": "BLE system-on-chip with chip antenna.", "functions": ["Advertise", "Serve the log over GATT"], "lab": "circuit" },
            { "id": "SYS-SNS", "name": "Motion & temperature sensing", "domain": "software", "description": "IMU with wake-on-motion and on-die temperature.", "functions": ["Detect motion", "Estimate tilt", "Measure temperature"], "lab": "control" },
            { "id": "SYS-PCB", "name": "Tag PCB", "domain": "electrical", "description": "Two-layer 0.8 mm board.", "functions": ["Interconnect electronics"], "lab": "pcb" },
            { "id": "SYS-ENC", "name": "Enclosure", "domain": "mechanical", "description": "Two printed shells with a clip and a battery door.", "functions": ["Protect the board", "Clip to a bag"], "lab": "cad" },
            { "id": "SYS-FW", "name": "Firmware", "domain": "software", "description": "Duty-cycled advertising, motion log, temperature log.", "functions": ["Schedule radio events", "Log data"], "lab": "rl" },
            { "id": "SYS-MFG", "name": "Manufacturing", "domain": "manufacturing", "description": "Turnkey PCB assembly, printed shells, hand assembly.", "functions": ["Source parts", "Assemble", "Test"], "lab": "mfg" }
        ],
        "interfaces": [
            { "id": "IF-01", "from": "SYS-PWR", "to": "SYS-RF", "kind": "POWER", "description": "3 V coin cell to the SoC's supply input." },
            { "id": "IF-02", "from": "SYS-SNS", "to": "SYS-RF", "kind": "I2C", "description": "IMU registers over I2C plus a wake interrupt line." },
            { "id": "IF-03", "from": "SYS-RF", "to": "SYS-FW", "kind": "BLE", "description": "Advertising packets and the log download service." },
            { "id": "IF-04", "from": "SYS-PCB", "to": "SYS-ENC", "kind": "MECH", "description": "Board snaps into the lower shell." },
            { "id": "IF-05", "from": "SYS-PWR", "to": "SYS-ENC", "kind": "MECH", "description": "Cell sits behind a twist-lock battery door." },
            { "id": "IF-06", "from": "SYS-PWR", "to": "SYS-SNS", "kind": "POWER", "description": "Sensor supply straight from the cell." }
        ],
        "components": [
            comp("U1", "BLE system-on-chip", "Cortex-M BLE 5 SoC", "SYS-RF", 3.6, 0.1, [9.0, 0.006, 0.002], "QFN-48_6x6", true),
            comp("U3", "Accelerometer + gyroscope (wake-on-motion)", "Low-power IMU, I2C", "SYS-SNS", 1.2, 0.02, [0.03, 0.0, 1.0], "LGA-14_3x2.5", true),
            comp("C3", "SoC decoupling 100 nF", "X7R MLCC 0402", "SYS-RF", 0.01, 0.002, Z, "0402", true),
            comp("C4", "IMU decoupling 100 nF", "X7R MLCC 0402", "SYS-SNS", 0.01, 0.002, Z, "0402", true),
            comp("R3", "I2C SCL pull-up 4.7 k", "Thick film 0402", "SYS-SNS", 0.005, 0.001, Z, "0402", true),
            comp("R4", "I2C SDA pull-up 4.7 k", "Thick film 0402", "SYS-SNS", 0.005, 0.001, Z, "0402", true),
            comp("AE1", "2.4 GHz chip antenna", "Ceramic chip antenna 3216", "SYS-RF", 0.25, 0.02, Z, "ANT_3216", true),
            comp("J3", "Cell contacts", "2-pin wire-to-board", "SYS-PWR", 0.1, 0.05, Z, "JST-SH-2", true),
            comp("BT1", "CR2032-class coin cell", "3 V lithium manganese dioxide coin cell, 220 mAh", "SYS-PWR", 0.3, 3.0, Z, "", false),
            comp("ENC1", "Printed shells (pair)", "PETG, 2 parts", "SYS-ENC", 0.4, 3.5, Z, "", false),
            comp("PCB1", "Tag PCB (bare)", "2-layer FR-4, 0.8 mm", "SYS-PCB", 0.3, 0.6, Z, "", false)
        ],
        "trades": [
            { "id": "T-1", "title": "Radio", "decision": "BLE advertising",
              "criteria": [ { "name": "Average power", "weight": 0.4 }, { "name": "Works with any phone", "weight": 0.3 }, { "name": "Range", "weight": 0.15 }, { "name": "Cost", "weight": 0.15 } ],
              "options": [
                { "name": "BLE advertising", "scores": [8, 10, 6, 8], "notes": "Every phone can listen; microamp average current." },
                { "name": "Sub-GHz radio", "scores": [7, 2, 10, 6], "notes": "Long range but needs a gateway." },
                { "name": "NFC only", "scores": [10, 7, 1, 9], "notes": "No battery drain, but only works when touched." } ] },
            { "id": "T-2", "title": "Cell", "decision": "CR2032-class coin cell",
              "criteria": [ { "name": "Energy", "weight": 0.4 }, { "name": "Mass", "weight": 0.3 }, { "name": "Availability", "weight": 0.3 } ],
              "options": [
                { "name": "CR2032-class coin cell", "scores": [6, 8, 10], "notes": "" },
                { "name": "CR2450-class coin cell", "scores": [9, 5, 8], "notes": "" },
                { "name": "Rechargeable LiPo + charger", "scores": [7, 6, 7], "notes": "Adds a charger and a connector." } ] }
        ],
        "risks": [
            { "id": "F-1", "item": "Coin cell", "failure_mode": "Voltage sag during radio bursts", "effect": "Brown-out reset, lost log", "cause": "High internal resistance when cold", "severity": 7, "occurrence": 4, "detection": 4, "mitigation": "Bulk capacitor across the cell, spread radio events, brown-out test at 0 °C." },
            { "id": "F-2", "item": "Clip", "failure_mode": "Snaps off", "effect": "Tag lost", "cause": "Thin printed section, cold", "severity": 5, "occurrence": 4, "detection": 3, "mitigation": "Fillet the root, print the clip flat, 500-cycle flex test." },
            { "id": "F-3", "item": "Cell contacts", "failure_mode": "Corrosion", "effect": "Intermittent power", "cause": "Condensation", "severity": 6, "occurrence": 3, "detection": 5, "mitigation": "Gold-plated contacts, gasket on the battery door." },
            { "id": "F-4", "item": "Radio", "failure_mode": "Packets missed in crowded 2.4 GHz", "effect": "Tag looks lost", "cause": "Interference", "severity": 3, "occurrence": 5, "detection": 4, "mitigation": "Advertise on all three channels, randomised interval." }
        ],
        "milestones": standard_milestones(),
        "design": {
            "battery": { "chemistry": "Li-MnO2 coin cell", "capacity_mah": 220, "nominal_v": 3.0, "full_v": 3.2, "cutoff_v": 2.0, "internal_resistance_ohm": 15, "mass_g": 3.0, "usable_fraction": 0.85 },
            "power_rails": { "core_v": 1.8, "buck_efficiency": 0.9, "display_rail_v": 3.0, "quiescent_ua": 0.5 },
            "pcb": { "width_mm": 24, "height_mm": 18, "layers": 2, "thickness_mm": 0.8, "min_trace_mm": 0.15, "min_clearance_mm": 0.15, "via_drill_mm": 0.3, "via_pad_mm": 0.6, "grid_mm": 0.25 },
            "manufacturing": { "quantities": [1, 10, 100, 1000, 10000], "target_quantity": 1000, "molding_material": "PC", "cavities": 2, "mold_steel": "P20", "assembly_minutes": 3, "labor_rate_usd_h": 30 },
            "extra": { "template": "pebble-tag" }
        }
    }))
}

/// Pip: self-balancing desk robot.
pub fn pip() -> Project {
    from_json(json!({
        "meta": {
            "id": "pip-robot", "name": "Pip Desk Robot", "codename": "PIP", "version": "0.1.0",
            "tagline": "A two-wheeled self-balancing desk companion with a display face and voice input",
            "description": "Pip balances on two wheels, drives around a desk, shows an animated face and listens for voice commands. It exercises the control (balance and tilt estimation), DSP (microphones), circuit (power), PCB, reinforcement-learning and manufacturing labs.",
            "author": ""
        },
        "concept": {
            "problem": "Learning robotics usually means kits that hide the engineering or cost more than a student can spend.",
            "vision": "A small robot whose every subsystem — balance loop, motor drive, voice front end, printed chassis — is open, simulated and buildable in a weekend.",
            "users": ["Students learning control and embedded systems", "Teachers running robotics labs", "Makers"],
            "use_cases": [
                { "id": "UC-1", "title": "Balance and roam", "narrative": "Pip stands up, balances with a state-feedback loop and wanders the desk, stopping at edges." },
                { "id": "UC-2", "title": "Voice command", "narrative": "\"Pip, come here\": the two-microphone front end captures speech, the phone recognises it and Pip drives toward the sound." },
                { "id": "UC-3", "title": "Study timer", "narrative": "The face shows a focus timer and reacts when the session ends." }
            ],
            "constraints": ["Printable on a 220 × 220 mm hobby printer", "Single 1-cell LiPo, USB-C charging", "Kit price under 100 USD"]
        },
        "needs": [
            { "id": "N-1", "text": "Balances and drives around a desk without falling over", "stakeholder": "User", "priority": 1 },
            { "id": "N-2", "text": "Runs for a whole study session on one charge", "stakeholder": "User", "priority": 1 },
            { "id": "N-3", "text": "Hears and responds to voice commands", "stakeholder": "User", "priority": 2 },
            { "id": "N-4", "text": "Affordable as a student kit", "stakeholder": "Business", "priority": 1 },
            { "id": "N-5", "text": "Printable and hand-assembled", "stakeholder": "Maker", "priority": 1 },
            { "id": "N-6", "text": "Safe to pick up at any time", "stakeholder": "User / Safety", "priority": 1 }
        ],
        "requirements": [
            req("R-001", "performance", "Pitch estimate error shall be at most 2 degrees RMS while driving.", "The balance loop needs an accurate tilt estimate; the Control lab's complementary filter provides it.", &["N-1"], &["SYS-CTL", "SYS-SNS"], Some("control.head_rms_err_deg"), "le", 2.0, "deg"),
            req("R-002", "performance", "Run time in the reference roam profile shall be at least 2 h.", "A study session is about two hours; the motors, SoC and face dominate the load.", &["N-2"], &["SYS-PWR", "SYS-DRV", "SYS-CMP", "SYS-FACE"], Some("budget.battery_life_h"), "ge", 2.0, "h"),
            req("R-003", "performance", "Buck converter output ripple shall be at most 30 mV peak-to-peak.", "Keeps the IMU and microphones quiet while the motors switch.", &["N-2", "N-1"], &["SYS-PWR"], Some("circuit.buck_ripple_mv"), "le", 30.0, "mV"),
            req("R-004", "performance", "Microphone array directivity index at 1 kHz shall be at least 4 dB.", "Rejects motor noise from behind the face.", &["N-3"], &["SYS-AUD"], Some("dsp.mic_di_db"), "ge", 4.0, "dB"),
            req("R-005", "performance", "PDM decimation filter stopband attenuation shall be at least 60 dB.", "Aliased ultrasonic noise from the motor drivers must stay below the speech floor.", &["N-3"], &["SYS-AUD"], Some("dsp.decim_stopband_db"), "ge", 60.0, "dB"),
            req("R-006", "performance", "Total robot mass shall be at most 350 g.", "Motor torque and fall energy both scale with mass.", &["N-1", "N-6"], &["SYS-CHS", "SYS-DRV", "SYS-PWR"], Some("mass.components_g"), "le", 350.0, "g"),
            req("R-007", "cost", "Bill of materials cost at quantity-1 prices shall be at most 60 USD.", "Leaves margin for a kit price under 100 USD.", &["N-4"], &["SYS-MFG"], Some("bom.cost_usd"), "le", 60.0, "USD"),
            req("R-008", "manufacturing", "All PCB connections shall be routed.", "An unrouted board cannot be ordered. Mark the board parts on-PCB to lay them out in PCB Studio.", &["N-5"], &["SYS-PCB"], Some("pcb.routed_pct"), "ge", 100.0, "%"),
            req("R-009", "manufacturing", "Design-rule check shall report 0 violations.", "Standard prototype-fab rules.", &["N-5"], &["SYS-PCB"], Some("pcb.drc_violations"), "le", 0.0, "count"),
            req("R-010", "goal", "A learned power policy should extend run time by at least 5 %.", "Dimming the face and sleeping the radio when idle.", &["N-2"], &["SYS-FW"], Some("rl.policy_life_gain_pct"), "ge", 5.0, "%"),
            req("R-011", "functional", "The robot shall recover from a 5 degree push without falling.", "Demonstrated on the desk with a push-stick.", &["N-1"], &["SYS-CTL", "SYS-DRV"], None, "ge", 5.0, "deg"),
            req("R-012", "safety", "Motor driver surface temperature shall stay at or below 45 C after 30 min of roaming.", "Pip is picked up by hand.", &["N-6"], &["SYS-DRV", "SYS-CHS"], None, "le", 45.0, "C")
        ],
        "subsystems": [
            { "id": "SYS-CMP", "name": "Compute & radio", "domain": "electrical", "description": "Wi-Fi + BLE SoC running the balance loop and behaviours.", "functions": ["Run the balance loop", "Drive the face", "Talk to the phone"], "lab": "circuit" },
            { "id": "SYS-PWR", "name": "Battery & power", "domain": "electrical", "description": "1-cell LiPo, USB-C charger, 3.3 V buck.", "functions": ["Store energy", "Charge", "Regulate"], "lab": "power" },
            { "id": "SYS-DRV", "name": "Motors & drive", "domain": "electrical", "description": "Two gear motors with encoders and an H-bridge driver.", "functions": ["Produce torque", "Measure wheel speed"], "lab": "robotics" },
            { "id": "SYS-SNS", "name": "IMU & encoders", "domain": "electrical", "description": "6-axis IMU on the board, quadrature encoders on the motors.", "functions": ["Sense tilt rate and acceleration", "Count wheel ticks"], "lab": "control" },
            { "id": "SYS-CTL", "name": "Balance control", "domain": "software", "description": "Complementary-filter tilt estimate + linear-quadratic regulator (LQR) state feedback.", "functions": ["Estimate tilt", "Stabilise the inverted pendulum"], "lab": "control" },
            { "id": "SYS-AUD", "name": "Voice front end", "domain": "acoustic", "description": "Two PDM microphones and a small speaker.", "functions": ["Capture speech", "Suppress motor noise"], "lab": "dsp" },
            { "id": "SYS-FACE", "name": "Display face", "domain": "optical", "description": "1.3 in round LCD showing the face.", "functions": ["Show expressions", "Show the timer"], "lab": "optics" },
            { "id": "SYS-PCB", "name": "Main board", "domain": "electrical", "description": "Two-layer board in the robot's body.", "functions": ["Interconnect electronics"], "lab": "pcb" },
            { "id": "SYS-CHS", "name": "Printed chassis", "domain": "mechanical", "description": "Body, wheel hubs and battery cradle, printed in PETG.", "functions": ["Carry the parts", "Protect in a fall"], "lab": "print" },
            { "id": "SYS-FW", "name": "Firmware & behaviours", "domain": "software", "description": "State machine for behaviours, learned power policy.", "functions": ["Choose behaviours", "Manage power"], "lab": "rl" },
            { "id": "SYS-MFG", "name": "Kit & assembly", "domain": "manufacturing", "description": "Board assembly, print farm, kit packing.", "functions": ["Source parts", "Assemble", "Test"], "lab": "mfg" }
        ],
        "interfaces": [
            { "id": "IF-01", "from": "SYS-PWR", "to": "SYS-CMP", "kind": "POWER", "description": "3.3 V rail from the buck." },
            { "id": "IF-02", "from": "SYS-PWR", "to": "SYS-DRV", "kind": "POWER", "description": "Battery rail to the H-bridge." },
            { "id": "IF-03", "from": "SYS-CMP", "to": "SYS-DRV", "kind": "PWM", "description": "Two PWM + direction pairs at 20 kHz." },
            { "id": "IF-04", "from": "SYS-SNS", "to": "SYS-CTL", "kind": "I2C", "description": "IMU samples at 200 Hz." },
            { "id": "IF-05", "from": "SYS-DRV", "to": "SYS-SNS", "kind": "GPIO", "description": "Encoder quadrature signals." },
            { "id": "IF-06", "from": "SYS-CTL", "to": "SYS-CMP", "kind": "SW", "description": "Balance loop task at 200 Hz on the SoC." },
            { "id": "IF-07", "from": "SYS-AUD", "to": "SYS-CMP", "kind": "PDM", "description": "Two microphones on one PDM data line." },
            { "id": "IF-08", "from": "SYS-CMP", "to": "SYS-FACE", "kind": "SPI", "description": "Face frames at 30 fps." },
            { "id": "IF-09", "from": "SYS-CMP", "to": "SYS-FW", "kind": "BLE", "description": "Phone app link." },
            { "id": "IF-10", "from": "SYS-PCB", "to": "SYS-CHS", "kind": "MECH", "description": "Board on four bosses in the body." },
            { "id": "IF-11", "from": "SYS-DRV", "to": "SYS-CHS", "kind": "MECH", "description": "Motor clamps and wheel hubs." },
            { "id": "IF-12", "from": "SYS-DRV", "to": "SYS-CHS", "kind": "THERMAL", "description": "Driver heat spreads into the body." }
        ],
        "components": [
            comp("U1", "Wi-Fi + BLE system-on-chip", "Dual-core Wi-Fi/BLE SoC", "SYS-CMP", 4.5, 0.2, [250.0, 1.0, 0.4], "QFN-48_6x6", true),
            comp("U3", "6-axis IMU", "Accelerometer + gyroscope, I2C", "SYS-SNS", 2.4, 0.02, [0.9, 0.01, 1.0], "LGA-14_3x2.5", true),
            comp("MK1", "PDM MEMS microphone (left)", "Bottom-port digital MEMS mic", "SYS-AUD", 0.8, 0.02, [0.9, 0.02, 0.5], "MIC_LGA_3.5x2.65", true),
            comp("MK2", "PDM MEMS microphone (right)", "Bottom-port digital MEMS mic", "SYS-AUD", 0.8, 0.02, [0.9, 0.02, 0.5], "MIC_LGA_3.5x2.65", true),
            comp("U4", "Li-ion linear charger", "Single-cell CC/CV charger", "SYS-PWR", 0.7, 0.01, [0.0, 0.004, 0.0], "SOT-23-5", true),
            comp("U5", "Buck converter 3.3 V", "Synchronous buck", "SYS-PWR", 0.9, 0.01, Z, "DFN-6_2x2", true),
            comp("U6", "Serial NOR flash 16 MB", "QSPI NOR", "SYS-CMP", 0.6, 0.02, [18.0, 0.002, 0.03], "USON-8_2x3", true),
            comp("L1", "Buck inductor 2.2 uH", "Shielded power inductor", "SYS-PWR", 0.12, 0.03, Z, "0805", true),
            comp("C1", "Buck input cap 4.7 uF", "X5R MLCC", "SYS-PWR", 0.02, 0.002, Z, "0402", true),
            comp("C2", "Buck output cap 10 uF", "X5R MLCC", "SYS-PWR", 0.03, 0.002, Z, "0402", true),
            comp("C3", "SoC decoupling 100 nF", "X7R MLCC", "SYS-CMP", 0.01, 0.002, Z, "0402", true),
            comp("C4", "IMU decoupling 100 nF", "X7R MLCC", "SYS-SNS", 0.01, 0.002, Z, "0402", true),
            comp("C5", "Charger cap 1 uF", "X5R MLCC", "SYS-PWR", 0.01, 0.002, Z, "0402", true),
            comp("R3", "I2C SCL pull-up 4.7 k", "Thick film", "SYS-SNS", 0.005, 0.001, Z, "0402", true),
            comp("R4", "I2C SDA pull-up 4.7 k", "Thick film", "SYS-SNS", 0.005, 0.001, Z, "0402", true),
            comp("R5", "Charge current set 10 k", "Thick film", "SYS-PWR", 0.005, 0.001, Z, "0402", true),
            comp("AE1", "2.4 GHz chip antenna", "Ceramic chip antenna", "SYS-CMP", 0.25, 0.02, Z, "ANT_3216", true),
            comp("J1", "USB-C receptacle (power only)", "USB Type-C 16-pin", "SYS-PWR", 0.3, 0.6, Z, "USB-C_16P", true),
            comp("J3", "Battery connector", "2-pin wire-to-board", "SYS-PWR", 0.1, 0.05, Z, "JST-SH-2", true),
            comp("BT1", "LiPo cell 1000 mAh", "3.7 V pouch cell with protection", "SYS-PWR", 5.0, 20.0, Z, "", false),
            comp("U2", "1.3 in round LCD face", "240 × 240 SPI LCD module", "SYS-FACE", 6.0, 6.0, [80.0, 0.1, 0.5], "", false),
            comp("M1", "Gear motor with encoder (left)", "N20-class 6 V gear motor", "SYS-DRV", 3.5, 10.0, [900.0, 0.0, 0.35], "", false),
            comp("M2", "Gear motor with encoder (right)", "N20-class 6 V gear motor", "SYS-DRV", 3.5, 10.0, [900.0, 0.0, 0.35], "", false),
            comp("U9", "Dual H-bridge driver module", "Dual H-bridge, 1 A per channel", "SYS-DRV", 1.5, 3.0, [5.0, 0.05, 1.0], "", false),
            comp("WH1", "Wheels with tyres (pair)", "Printed hub + O-ring tyre", "SYS-DRV", 1.0, 16.0, Z, "", false),
            comp("SP1", "Speaker 15 mm", "8 Ω micro speaker", "SYS-AUD", 1.0, 3.0, [20.0, 0.0, 0.05], "", false),
            comp("CH1", "Printed chassis set", "PETG body, cradle, covers", "SYS-CHS", 1.2, 60.0, Z, "", false),
            comp("HW1", "Screws and inserts", "M2 screws, heat-set inserts", "SYS-CHS", 0.5, 5.0, Z, "", false),
            comp("PCB1", "Main board (bare)", "2-layer FR-4, 1.6 mm", "SYS-PCB", 2.0, 4.0, Z, "", false)
        ],
        "trades": [
            { "id": "T-1", "title": "Balance controller", "decision": "LQR state feedback",
              "criteria": [ { "name": "Disturbance rejection", "weight": 0.35 }, { "name": "Ease of tuning", "weight": 0.25 }, { "name": "Compute cost", "weight": 0.2 }, { "name": "Teaching value", "weight": 0.2 } ],
              "options": [
                { "name": "Cascaded PID", "scores": [6, 8, 10, 7], "notes": "Simple, but two loops to tune." },
                { "name": "LQR state feedback", "scores": [9, 7, 9, 9], "notes": "Optimal gains from a model." },
                { "name": "Learned policy (RL)", "scores": [7, 3, 5, 8], "notes": "Needs a simulator and lots of episodes." } ] },
            { "id": "T-2", "title": "Drive motors", "decision": "Gear motors with encoders",
              "criteria": [ { "name": "Low-speed control", "weight": 0.4 }, { "name": "Cost", "weight": 0.3 }, { "name": "Noise", "weight": 0.3 } ],
              "options": [
                { "name": "Gear motors with encoders", "scores": [8, 8, 5], "notes": "" },
                { "name": "Stepper motors", "scores": [9, 6, 3], "notes": "Heavy and power-hungry when holding." },
                { "name": "Gimbal BLDC + FOC", "scores": [10, 3, 9], "notes": "Silky but needs a field-oriented control driver." } ] }
        ],
        "risks": [
            { "id": "F-1", "item": "LiPo cell", "failure_mode": "Puncture in a fall", "effect": "Fire hazard", "cause": "Drop from the desk", "severity": 10, "occurrence": 2, "detection": 5, "mitigation": "Cell in an enclosed cradle, protection board, edge-detection behaviour." },
            { "id": "F-2", "item": "Balance loop", "failure_mode": "Oscillation", "effect": "Robot falls", "cause": "IMU vibration, gain too high", "severity": 6, "occurrence": 5, "detection": 3, "mitigation": "Soft-mount the IMU, gain margin checked in the Control lab." },
            { "id": "F-3", "item": "Motor driver", "failure_mode": "Overheats", "effect": "Shutdown, hot surface", "cause": "Stall current", "severity": 6, "occurrence": 3, "detection": 4, "mitigation": "Current limit, stall detection from encoder speed." },
            { "id": "F-4", "item": "Wheel hub", "failure_mode": "Slips on the shaft", "effect": "Loss of balance", "cause": "Worn printed D-bore", "severity": 5, "occurrence": 4, "detection": 4, "mitigation": "Heat-set brass insert with set screw." },
            { "id": "F-5", "item": "Microphones", "failure_mode": "Motor noise masks speech", "effect": "Commands missed", "cause": "Gear whine", "severity": 4, "occurrence": 6, "detection": 3, "mitigation": "Beamformer null toward the motors, pause driving while listening." }
        ],
        "milestones": standard_milestones(),
        "design": {
            "battery": { "chemistry": "LiPo", "capacity_mah": 1000, "nominal_v": 3.7, "full_v": 4.2, "cutoff_v": 3.3, "internal_resistance_ohm": 0.12, "mass_g": 20.0, "usable_fraction": 0.85 },
            "power_rails": { "core_v": 3.3, "buck_efficiency": 0.9, "display_rail_v": 3.3, "quiescent_ua": 20 },
            "audio": { "mic_count": 2, "mic_spacing_mm": 20, "sample_rate_hz": 16000, "pdm_clock_hz": 1024000, "decimation": 64, "fir_taps": 31, "voice_band_lo_hz": 100, "voice_band_hi_hz": 4000 },
            "pcb": { "width_mm": 50, "height_mm": 30, "layers": 2, "thickness_mm": 1.6, "min_trace_mm": 0.15, "min_clearance_mm": 0.15, "via_drill_mm": 0.3, "via_pad_mm": 0.6, "grid_mm": 0.25 },
            "manufacturing": { "quantities": [1, 10, 100, 1000], "target_quantity": 500, "molding_material": "ABS", "cavities": 1, "mold_steel": "P20", "assembly_minutes": 25, "labor_rate_usd_h": 30 },
            "extra": { "template": "pip-robot" }
        }
    }))
}

/// Every starter template.
pub fn templates() -> Vec<Template> {
    vec![
        Template {
            id: "blank".into(),
            name: "Blank product".into(),
            description: "An empty project with the standard stage-gate milestones. Write the problem, the needs and the first requirements, then add subsystems.".into(),
            icon: "systems".into(),
            tags: vec!["empty".into()],
            project: blank(),
        },
        Template {
            id: "iris".into(),
            name: "Iris smart glasses".into(),
            description: "The reference product: monocular heads-up-display glasses with 24 requirements verified by every lab.".into(),
            icon: "optics".into(),
            tags: vec!["wearable".into(), "optics".into(), "reference".into()],
            project: Project::iris(),
        },
        Template {
            id: "pebble".into(),
            name: "Pebble sensor tag".into(),
            description: "A coin-cell Bluetooth Low Energy (BLE) tag that lasts a year: a small, low-power product to learn budgets and margins on.".into(),
            icon: "power".into(),
            tags: vec!["wearable".into(), "low power".into(), "IoT".into()],
            project: pebble(),
        },
        Template {
            id: "pip".into(),
            name: "Pip desk robot".into(),
            description: "A self-balancing two-wheeled desk robot with a display face and voice input: control, DSP, PCB and manufacturing in one product.".into(),
            icon: "robotics".into(),
            tags: vec!["robot".into(), "control".into(), "education".into()],
            project: pip(),
        },
    ]
}

/// One template by id.
pub fn template(id: &str) -> Option<Template> {
    templates().into_iter().find(|t| t.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::{evaluate, lint, Status};

    #[test]
    fn templates_parse_and_lint_clean() {
        let ts = templates();
        assert!(ts.len() >= 4);
        for t in &ts {
            let errors: Vec<_> = lint(&t.project).into_iter().filter(|i| i.severity == "error").collect();
            assert!(errors.is_empty(), "{}: {errors:?}", t.id);
            assert_eq!(t.project.milestones.len(), 6, "{}", t.id);
            if t.id != "blank" {
                let ev = evaluate(&t.project);
                let cov = crate::systems::analysis::coverage(&t.project, &ev);
                assert!(cov.empty_subsystems.is_empty(), "{}: {:?}", t.id, cov.empty_subsystems);
                assert_eq!(cov.needs_covered, cov.needs_total, "{}", t.id);
            }
        }
        assert!(template("pebble").is_some());
        assert!(template("nope").is_none());
    }

    #[test]
    fn pebble_budgets_hand_calc() {
        let p = pebble();
        // U1: 9 mW × 0.002 + 0.006 × 0.998; U3: 0.03 mW continuous
        let load = 9.0 * 0.002 + 0.006 * 0.998 + 0.03;
        assert!((crate::systems::budgets::load_power_mw(&p) - load).abs() < 1e-12);
        let m = crate::systems::all_metrics(&p);
        let life = m.iter().find(|x| x.key == "budget.battery_life_h").unwrap().value;
        let expect = 220.0 * 3.0 * 0.85 / (load / 0.9);
        assert!((life - expect).abs() < 1e-6, "{life} vs {expect}");
        assert!(life > 8760.0, "one year: {life}");
        let mass = m.iter().find(|x| x.key == "mass.components_g").unwrap().value;
        assert!(mass < 12.0, "{mass}");
    }

    #[test]
    fn small_templates_verify_their_roll_up_requirements() {
        for p in [pebble(), pip()] {
            let ev = evaluate(&p);
            for r in &p.requirements {
                let res = ev.results.iter().find(|x| x.id == r.id).unwrap();
                let m = r.verification.metric.as_deref();
                if matches!(
                    m,
                    Some("budget.battery_life_h" | "budget.avg_power_mw" | "mass.components_g" | "bom.cost_usd")
                ) {
                    assert_eq!(res.status, Status::Pass, "{} {} = {:?}", p.meta.id, r.id, res.value);
                }
                if m.is_none() {
                    assert_eq!(res.status, Status::Unverified);
                }
            }
        }
    }
}
