//! "Iris v0": the breadboard prototype built from widely available
//! development boards, with a wiring model the web page animates.
//!
//! * **XIAO build** (primary) — Seeed Studio XIAO nRF52840 Sense: the same
//!   Bluetooth Low Energy system-on-chip family as Iris, with an on-board
//!   6-axis inertial measurement unit (IMU), a pulse-density-modulation (PDM)
//!   microphone and a lithium-polymer (LiPo) charger. Pin names are those of
//!   Seeed's published board variant (`D0`–`D10`, `BAT+`/`BAT−` pads).
//! * **ESP32-S3 build** (alternative) — ESP32-S3-DevKitC-1 + MPU-6050-class
//!   IMU + INMP441-class I²S microphone, the one you can simulate in Wokwi.
//!   Pin names follow the Wokwi board definition of the DevKitC-1.
//!
//! Module power figures are typical values stated as assumptions (check the
//! datasheets of the modules you buy); the battery-life comparison runs them
//! through the Power Lab's budget (`crate::power::budget`) exactly like the
//! custom board.

use serde::{Deserialize, Serialize};

use crate::power::budget::{budget, scenario_profile, UsageProfile};
use crate::project::{Component, PowerProfile, Project};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Build {
    #[default]
    Xiao,
    Esp32s3,
}

#[derive(Clone, Debug, Serialize)]
pub struct BomLine {
    pub item: String,
    pub qty: u32,
    /// Where to buy: vendor category, not a quote.
    pub category: String,
    /// What it stands in for on the custom Iris board.
    pub stands_in_for: String,
    pub notes: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Pin {
    pub name: String,
    pub x: f64,
    pub y: f64,
    /// Side the wire leaves from: "l", "r", "t", "b".
    pub side: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Part {
    pub id: String,
    pub label: String,
    pub sublabel: String,
    /// "board", "module", "battery".
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub pins: Vec<Pin>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Wire {
    pub id: usize,
    pub from_part: String,
    pub from_pin: String,
    pub to_part: String,
    pub to_pin: String,
    pub color: String,
    pub signal: String,
    pub note: String,
    pub route: Vec<(f64, f64)>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Step {
    pub title: String,
    pub detail: String,
    /// Wire ids this step connects (for highlighting).
    pub wires: Vec<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Difference {
    pub aspect: String,
    pub prototype: String,
    pub custom: String,
    pub consequence: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PowerRow {
    pub name: String,
    pub active_mw: f64,
    pub sleep_mw: f64,
    pub duty: f64,
    pub avg_mw: f64,
    pub basis: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PowerCompare {
    pub scenario: String,
    pub battery_mah: f64,
    pub proto_rows: Vec<PowerRow>,
    pub proto_avg_mw: f64,
    pub proto_life_h: f64,
    pub proto_regulator: String,
    pub proto_efficiency: f64,
    pub custom_rows: Vec<PowerRow>,
    pub custom_avg_mw: f64,
    pub custom_life_h: f64,
    pub custom_efficiency: f64,
    pub ratio: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Breadboard {
    pub build: Build,
    pub title: String,
    pub summary: String,
    pub width: f64,
    pub height: f64,
    pub parts: Vec<Part>,
    pub wires: Vec<Wire>,
    pub bom: Vec<BomLine>,
    pub steps: Vec<Step>,
    pub differences: Vec<Difference>,
    pub power: PowerCompare,
    pub notes: Vec<String>,
}

fn pin(name: &str, x: f64, y: f64, side: &str) -> Pin {
    Pin {
        name: name.into(),
        x,
        y,
        side: side.into(),
    }
}

fn find(parts: &[Part], id: &str, pin: &str) -> (f64, f64, String) {
    parts
        .iter()
        .find(|p| p.id == id)
        .and_then(|p| p.pins.iter().find(|q| q.name == pin))
        .map(|q| (q.x, q.y, q.side.clone()))
        .unwrap_or((0.0, 0.0, "r".into()))
}

fn stub(x: f64, y: f64, side: &str, d: f64) -> (f64, f64) {
    match side {
        "l" => (x - d, y),
        "t" => (x, y - d),
        "b" => (x, y + d),
        _ => (x + d, y),
    }
}

/// Orthogonal route: pin → short stub → (via x or y lane) → stub → pin.
enum Via {
    /// Vertical run at this x.
    X(f64),
    /// Horizontal run at this y, entered and left through the given x columns.
    Y(f64, f64, f64),
}

fn route(parts: &[Part], a: (&str, &str), b: (&str, &str), via: Via) -> Vec<(f64, f64)> {
    let (x1, y1, s1) = find(parts, a.0, a.1);
    let (x2, y2, s2) = find(parts, b.0, b.1);
    let p1 = stub(x1, y1, &s1, 10.0);
    let p2 = stub(x2, y2, &s2, 10.0);
    let mut r = vec![(x1, y1), p1];
    match via {
        Via::X(x) => {
            r.push((x, p1.1));
            r.push((x, p2.1));
        }
        Via::Y(y, xa, xb) => {
            r.push((xa, p1.1));
            r.push((xa, y));
            r.push((xb, y));
            r.push((xb, p2.1));
        }
    }
    r.push(p2);
    r.push((x2, y2));
    // drop consecutive duplicates
    r.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
    r
}

#[allow(clippy::too_many_arguments)]
fn wire(id: usize, parts: &[Part], a: (&str, &str), b: (&str, &str), color: &str, signal: &str, note: &str, via: Via) -> Wire {
    Wire {
        id,
        from_part: a.0.into(),
        from_pin: a.1.into(),
        to_part: b.0.into(),
        to_pin: b.1.into(),
        color: color.into(),
        signal: signal.into(),
        note: note.into(),
        route: route(parts, a, b, via),
    }
}

fn comp(refdes: &str, name: &str, sub: &str, active: f64, sleep: f64, duty: f64) -> Component {
    Component {
        refdes: refdes.into(),
        name: name.into(),
        subsystem: sub.into(),
        qty: 1.0,
        power: PowerProfile {
            active_mw: active,
            sleep_mw: sleep,
            duty,
        },
        ..Default::default()
    }
}

fn find_comp<'a>(p: &'a Project, key: &str) -> Option<&'a Component> {
    p.components.iter().find(|c| c.name.to_lowercase().contains(key))
}

fn duty_of(p: &Project, key: &str, fallback: f64) -> f64 {
    find_comp(p, key).map(|c| c.power.duty).unwrap_or(fallback)
}

/// Prototype power components (typical module figures, assumptions) → (components, basis notes, regulator, efficiency).
fn proto_components(p: &Project, build: Build) -> (Vec<Component>, Vec<String>, String, f64) {
    let disp_duty = crate::power::budget::display_ref(p)
        .and_then(|r| p.components.iter().find(|c| c.refdes == r))
        .map(|c| c.power.duty)
        .unwrap_or(0.3);
    let soc = find_comp(p, "system-on-chip");
    let soc_duty = soc.map(|c| c.power.duty).unwrap_or(0.25);
    let mic_duty = duty_of(p, "microphone", 0.25);
    // same silicon, 3.3 V rail instead of 1.8 V: same current → power × 3.3/1.8
    let k = 3.3 / p.design.power_rails.core_v.max(0.5);
    let scale = |key: &str, a: f64, s: f64| {
        find_comp(p, key)
            .map(|c| (c.power.active_mw * k, c.power.sleep_mw * k))
            .unwrap_or((a, s))
    };
    let oled = comp("OLED", "SSD1306 0.96 in OLED display 128x64", "SYS-OPT", 40.0, 0.03, disp_duty);
    let touch = comp("TOUCH", "TTP223 touch module", "SYS-SNS", 0.02, 0.02, 1.0);
    match build {
        Build::Xiao => {
            let (sa, ss) = scale("system-on-chip", 21.0, 0.006);
            let (ia, is) = scale("imu", 0.9, 0.01);
            let (ma, ms) = scale("microphone", 0.9, 0.02);
            let comps = vec![
                comp("XIAO", "nRF52840 system-on-chip (on the XIAO)", "SYS-CMP", sa, ss, soc_duty),
                oled,
                comp("IMU", "LSM6DS3TR-C IMU (on the XIAO)", "SYS-SNS", ia, is, 1.0),
                comp("MIC", "PDM microphone (on the XIAO)", "SYS-AUD", ma, ms, mic_duty),
                touch,
                comp("BRD", "XIAO board overhead (regulator, charger, flash standby)", "SYS-PWR", 0.2, 0.2, 1.0),
            ];
            let notes = vec![
                format!("system-on-chip, IMU and microphone: the Iris BOM figures × {k:.2} (same current at the XIAO's 3.3 V rail instead of the {:.1} V core rail)", p.design.power_rails.core_v),
                "SSD1306 OLED 40 mW: ≈12 mA at 3.3 V with about a quarter of the pixels lit (typical; grows with lit pixels) — measure your module".into(),
                "TTP223 ≈ a few µA; XIAO board overhead 0.2 mW — assumptions".into(),
            ];
            let eff = 3.3 / p.design.battery.nominal_v.max(3.3);
            (comps, notes, format!("on-board 3.3 V linear regulator (efficiency ≈ 3.3 V / {:.1} V)", p.design.battery.nominal_v), eff)
        }
        Build::Esp32s3 => {
            let comps = vec![
                comp("ESP", "ESP32-S3 module with BLE", "SYS-CMP", 100.0, 2.5, soc_duty),
                oled,
                comp("MPU", "MPU-6050 IMU module", "SYS-SNS", 12.9, 0.02, 1.0),
                comp("I2S", "INMP441 I2S microphone module", "SYS-AUD", 4.6, 0.04, mic_duty),
                touch,
                comp("DEVKIT", "DevKit overhead (power LED, USB-UART bridge, regulator)", "SYS-PWR", 5.0, 5.0, 1.0),
            ];
            let notes = vec![
                "ESP32-S3 100 mW active (CPU + radio events) / 2.5 mW light sleep — typical, depends heavily on firmware".into(),
                "MPU-6050 ≈ 3.9 mA and INMP441 ≈ 1.4 mA at 3.3 V (datasheet-class figures); DevKit overhead 5 mW — assumptions".into(),
                "SSD1306 OLED 40 mW typical (see the XIAO build)".into(),
            ];
            let eff = 3.3 / p.design.battery.nominal_v.max(3.3);
            (comps, notes, "DevKit 3.3 V linear regulator fed from a LiPo + charger module or a USB power bank".into(), eff)
        }
    }
}

fn rows(b: &crate::power::Budget) -> Vec<PowerRow> {
    b.components
        .iter()
        .map(|c| PowerRow {
            name: c.name.clone(),
            active_mw: c.active_mw,
            sleep_mw: c.sleep_mw,
            duty: c.duty,
            avg_mw: c.avg_mw,
            basis: String::new(),
        })
        .collect()
}

pub fn power_compare(p: &Project, build: Build, battery_mah: f64, scenario: &str) -> (PowerCompare, Vec<String>) {
    let prof = scenario_profile(scenario).unwrap_or_else(UsageProfile::default);
    let custom = budget(p, &prof);
    let (comps, notes, reg, eff) = proto_components(p, build);
    let mut q = p.clone();
    q.components = comps;
    q.design.power_rails.buck_efficiency = eff;
    q.design.power_rails.quiescent_ua = 0.0;
    q.design.battery.capacity_mah = if battery_mah > 0.0 { battery_mah } else { p.design.battery.capacity_mah };
    let proto = budget(&q, &prof);
    let pc = PowerCompare {
        scenario: if scenario.is_empty() { "reference".into() } else { scenario.into() },
        battery_mah: q.design.battery.capacity_mah,
        proto_rows: rows(&proto),
        proto_avg_mw: proto.avg_mw,
        proto_life_h: proto.life_h,
        proto_regulator: reg,
        proto_efficiency: eff,
        custom_rows: rows(&custom),
        custom_avg_mw: custom.avg_mw,
        custom_life_h: custom.life_h,
        custom_efficiency: custom.buck_efficiency,
        ratio: proto.avg_mw / custom.avg_mw.max(1e-9),
    };
    (pc, notes)
}

fn bl(item: &str, qty: u32, category: &str, stands: &str, notes: &str) -> BomLine {
    BomLine {
        item: item.into(),
        qty,
        category: category.into(),
        stands_in_for: stands.into(),
        notes: notes.into(),
    }
}

fn st(title: &str, detail: &str, wires: &[usize]) -> Step {
    Step {
        title: title.into(),
        detail: detail.into(),
        wires: wires.to_vec(),
    }
}

fn df(a: &str, p: &str, c: &str, k: &str) -> Difference {
    Difference {
        aspect: a.into(),
        prototype: p.into(),
        custom: c.into(),
        consequence: k.into(),
    }
}

fn xiao(p: &Project, battery_mah: f64, scenario: &str) -> Breadboard {
    let xl = 330.0;
    let xr = 470.0;
    let left = ["D0", "D1", "D2", "D3", "D4/SDA", "D5/SCL", "D6/TX"];
    let right = ["5V", "GND", "3V3", "D10/MOSI", "D9/MISO", "D8/SCK", "D7/RX"];
    let mut xp: Vec<Pin> = Vec::new();
    for (k, n) in left.iter().enumerate() {
        xp.push(pin(n, xl, 95.0 + 20.0 * k as f64, "l"));
    }
    for (k, n) in right.iter().enumerate() {
        xp.push(pin(n, xr, 95.0 + 20.0 * k as f64, "r"));
    }
    xp.push(pin("BAT+", 375.0, 240.0, "b"));
    xp.push(pin("BAT-", 425.0, 240.0, "b"));
    let parts = vec![
        Part {
            id: "xiao".into(),
            label: "XIAO nRF52840 Sense".into(),
            sublabel: "BLE SoC · 6-axis IMU · PDM mic · LiPo charger".into(),
            kind: "board".into(),
            x: xl,
            y: 60.0,
            w: xr - xl,
            h: 180.0,
            pins: xp,
        },
        Part {
            id: "oled".into(),
            label: "OLED 128×64".into(),
            sublabel: "SSD1306 / SH1106, I²C 0x3C".into(),
            kind: "module".into(),
            x: 590.0,
            y: 70.0,
            w: 200.0,
            h: 130.0,
            pins: vec![
                pin("GND", 590.0, 100.0, "l"),
                pin("VCC", 590.0, 120.0, "l"),
                pin("SCL", 590.0, 140.0, "l"),
                pin("SDA", 590.0, 160.0, "l"),
            ],
        },
        Part {
            id: "touch".into(),
            label: "TTP223 touch".into(),
            sublabel: "capacitive pad, active-high".into(),
            kind: "module".into(),
            x: 80.0,
            y: 95.0,
            w: 120.0,
            h: 95.0,
            pins: vec![pin("VCC", 200.0, 120.0, "r"), pin("SIG", 200.0, 145.0, "r"), pin("GND", 200.0, 170.0, "r")],
        },
        Part {
            id: "lipo".into(),
            label: format!("LiPo 3.7 V {:.0} mAh", if battery_mah > 0.0 { battery_mah } else { p.design.battery.capacity_mah }),
            sublabel: "with protection circuit".into(),
            kind: "battery".into(),
            x: 340.0,
            y: 330.0,
            w: 120.0,
            h: 60.0,
            pins: vec![pin("+", 375.0, 330.0, "t"), pin("-", 425.0, 330.0, "t")],
        },
    ];
    let wires = vec![
        wire(0, &parts, ("xiao", "GND"), ("oled", "GND"), "black", "GND", "common ground", Via::X(520.0)),
        wire(1, &parts, ("xiao", "3V3"), ("oled", "VCC"), "red", "3V3", "3.3 V supply (most modules accept 3.3–5 V — check)", Via::X(535.0)),
        wire(2, &parts, ("xiao", "D5/SCL"), ("oled", "SCL"), "yellow", "I2C SCL", "I²C clock; modules usually carry pull-ups", Via::Y(275.0, 310.0, 550.0)),
        wire(3, &parts, ("xiao", "D4/SDA"), ("oled", "SDA"), "blue", "I2C SDA", "I²C data", Via::Y(290.0, 300.0, 565.0)),
        wire(4, &parts, ("touch", "SIG"), ("xiao", "D1"), "green", "TOUCH", "digital input, sampled at 1 kHz by the gesture FSM", Via::X(265.0)),
        wire(5, &parts, ("touch", "VCC"), ("xiao", "3V3"), "red", "3V3", "", Via::Y(40.0, 225.0, 495.0)),
        wire(6, &parts, ("touch", "GND"), ("xiao", "GND"), "black", "GND", "", Via::Y(305.0, 240.0, 505.0)),
        wire(7, &parts, ("lipo", "+"), ("xiao", "BAT+"), "red", "VBAT", "solder to the BAT+ pad on the underside — mind the polarity", Via::X(375.0)),
        wire(8, &parts, ("lipo", "-"), ("xiao", "BAT-"), "black", "GND", "solder to the BAT− pad", Via::X(425.0)),
    ];
    let bom = vec![
        bl("Seeed Studio XIAO nRF52840 Sense", 1, "Development board — Seeed Studio store and electronics distributors", "U1 SoC, U3 IMU, MK1 microphone, U4 charger", "Get the *Sense* variant (IMU + microphone). Headers are usually supplied loose."),
        bl("0.96 in SSD1306 (or 1.3 in SH1106) I²C OLED module, 128×64", 1, "Display module — hobby electronics retailers / marketplaces", "U2 micro-OLED + combiner", "4-pin I²C version (GND, VCC, SCL, SDA — pin order varies)."),
        bl("TTP223 capacitive touch module", 1, "Sensor module — hobby electronics retailers", "U7 touch controller + E1 touch pad", "Momentary / active-high by default; the A/B jumpers change it."),
        bl("3.7 V LiPo cell with protection circuit and JST lead", 1, "Battery — hobby / RC and electronics retailers", "BT1 LiPo cell", "Match the capacity you simulate; the XIAO charges it over USB-C."),
        bl("Half-size solderless breadboard", 1, "Prototyping supplies", "SYS-PCB main board", "830- or 400-point."),
        bl("Male–male jumper wires (red, black, yellow, blue, green)", 9, "Prototyping supplies", "board traces", "Colour code: red 3.3 V, black ground, yellow SCL, blue SDA, green signal."),
        bl("USB-C data cable", 1, "Cables", "J1 USB-C", "A charge-only cable will not program the board."),
        bl("Optional: 45° acrylic or glass beam-splitter + small lens", 1, "Optics — maker optics suppliers", "OPT1 collimating lens + combiner", "Look at the OLED through it to preview the heads-up effect (see the Optics Lab)."),
    ];
    let steps = vec![
        st("Seat the XIAO", "Solder the headers, press the XIAO across the breadboard's centre channel with USB-C facing outward, so each pin gets its own row.", &[]),
        st("Share ground and 3.3 V", "Run black from the XIAO's GND and red from its 3V3 pin to the OLED and to the touch module. Every module must share the same ground.", &[0, 1, 5, 6]),
        st("I²C to the OLED", "Yellow D5 (SCL) → OLED SCL, blue D4 (SDA) → OLED SDA. The on-board IMU sits on a separate internal I²C bus, so this bus only carries the display.", &[2, 3]),
        st("Touch input", "Green from the TTP223 SIG pin to D1. Touch the pad: its LED lights and D1 goes high.", &[4]),
        st("Battery", "With nothing plugged into USB, solder the LiPo leads to the BAT+ / BAT− pads on the underside (red to +). Double-check polarity — reversing a LiPo can destroy the board.", &[7, 8]),
        st("Flash the firmware", "Firmware tab → target \"XIAO (Arduino)\" → download the zip, open it in PlatformIO or the Arduino IDE (Seeed nRF52 core), upload over USB-C.", &[]),
        st("Test", "The clock screen appears. Tap: next screen; double tap: back to the clock; long press: display off. Open nRF Connect for Mobile, connect to \"Iris v0\" and write a Captions packet.", &[]),
        st("Measure", "Put a USB power meter or a power profiler between battery and board, run each screen for a minute, and compare the current with the power table below — then correct the assumptions.", &[]),
    ];
    let differences = vec![
        df("System-on-chip", "nRF52840 on the XIAO module (3.3 V rail)", "nRF52840-class SoC on the custom board (1.8 V core rail from a buck)", "Same firmware architecture and radio; the custom rail cuts SoC power by ≈ 1.8/3.3."),
        df("Display", "128×64 monochrome OLED, I²C, viewed directly", "640×400 micro-OLED over SPI through a collimating lens and 45° combiner", "The HUD renders at 1 bpp on both; the lab scales the same UI description (font scale 1 vs 5)."),
        df("Microphones", "One on-board PDM microphone", "Two PDM microphones, 14 mm endfire pair + beamformer", "No directivity on the prototype (R-009 is verified by the DSP Lab, not the breadboard)."),
        df("IMU", "On-board LSM6DS3TR-C", "BMI270-class IMU on the main board", "Different noise figures — re-run the Control Lab with the prototype IMU's datasheet values."),
        df("Touch", "TTP223 module, one pad", "AT42QT1011-class controller + temple touch strip", "Same digital signal into the gesture FSM; thresholds identical."),
        df("Power path", "LiPo → XIAO charger → linear regulator 3.3 V", "LiPo → charger → 1.8 V synchronous buck (90 %) + display load switch", "The prototype's battery life is shorter — see the comparison table."),
        df("Size and mass", "Breadboard, wires, loose modules", "58 × 10 mm board in the right temple pod", "Proves function, not wearability; the CAD and PCB labs cover packaging."),
    ];
    let (power, pnotes) = power_compare(p, Build::Xiao, battery_mah, scenario);
    Breadboard {
        build: Build::Xiao,
        title: "Iris v0 — XIAO nRF52840 Sense build".into(),
        summary: "The primary prototype: one thumb-sized board already contains Iris's SoC family, IMU, microphone and charger, so only a display, a touch pad and a battery are added.".into(),
        width: 860.0,
        height: 410.0,
        parts,
        wires,
        bom,
        steps,
        differences,
        power,
        notes: pnotes,
    }
}

fn esp32(p: &Project, battery_mah: f64, scenario: &str) -> Breadboard {
    let (bx, bw, top) = (360.0, 110.0, 30.0);
    let left = [
        "3V3.1", "3V3.2", "RST", "4", "5", "6", "7", "15", "16", "17", "18", "8", "3", "46", "9", "10", "11", "12", "13", "14", "5V", "GND.1",
    ];
    let right = [
        "GND.2", "TX", "RX", "1", "2", "42", "41", "40", "39", "38", "37", "36", "35", "0", "45", "48", "47", "21", "20", "19", "GND.3", "GND.4",
    ];
    let mut ep = Vec::new();
    for (k, n) in left.iter().enumerate() {
        ep.push(pin(n, bx, 50.0 + 15.0 * k as f64, "l"));
    }
    for (k, n) in right.iter().enumerate() {
        ep.push(pin(n, bx + bw, 50.0 + 15.0 * k as f64, "r"));
    }
    let m = |id: &str, label: &str, sub: &str, y: f64, h: f64, pins: &[(&str, f64)]| Part {
        id: id.into(),
        label: label.into(),
        sublabel: sub.into(),
        kind: "module".into(),
        x: 40.0,
        y,
        w: 160.0,
        h,
        pins: pins.iter().map(|(n, py)| pin(n, 200.0, *py, "r")).collect(),
    };
    let parts = vec![
        Part {
            id: "esp".into(),
            label: "ESP32-S3-DevKitC-1".into(),
            sublabel: "Wi-Fi + BLE".into(),
            kind: "board".into(),
            x: bx,
            y: top,
            w: bw,
            h: 360.0,
            pins: ep,
        },
        m("oled", "OLED 128×64", "SSD1306, I²C 0x3C", 30.0, 100.0, &[("GND", 50.0), ("VCC", 70.0), ("SCL", 90.0), ("SDA", 110.0)]),
        m(
            "mic",
            "INMP441 microphone",
            "I²S, 24-bit",
            150.0,
            105.0,
            &[("VDD", 160.0), ("GND", 175.0), ("L/R", 190.0), ("WS", 205.0), ("SCK", 220.0), ("SD", 235.0)],
        ),
        m("imu", "MPU-6050 IMU", "I²C 0x68", 275.0, 100.0, &[("VCC", 290.0), ("GND", 310.0), ("SCL", 330.0), ("SDA", 350.0)]),
        Part {
            id: "touch".into(),
            label: "Touch / button".into(),
            sublabel: "TTP223 (hardware) · push button (Wokwi)".into(),
            kind: "module".into(),
            x: 540.0,
            y: 120.0,
            w: 150.0,
            h: 80.0,
            pins: vec![pin("VCC", 540.0, 135.0, "l"), pin("SIG", 540.0, 155.0, "l"), pin("GND", 540.0, 175.0, "l")],
        },
    ];
    let mut wires = Vec::new();
    let spec: [(&str, &str, &str, &str, &str, &str, f64); 14] = [
        ("oled", "GND", "esp", "GND.1", "black", "GND", 214.0),
        ("oled", "VCC", "esp", "3V3.1", "red", "3V3", 222.0),
        ("oled", "SCL", "esp", "9", "yellow", "I2C SCL", 230.0),
        ("oled", "SDA", "esp", "8", "blue", "I2C SDA", 238.0),
        ("mic", "VDD", "esp", "3V3.2", "red", "3V3", 246.0),
        ("mic", "GND", "esp", "GND.1", "black", "GND", 254.0),
        ("mic", "L/R", "esp", "GND.1", "black", "L/R select (left)", 262.0),
        ("mic", "WS", "esp", "5", "purple", "I2S WS", 270.0),
        ("mic", "SCK", "esp", "4", "orange", "I2S SCK", 278.0),
        ("mic", "SD", "esp", "6", "white", "I2S SD", 286.0),
        ("imu", "VCC", "esp", "3V3.2", "red", "3V3", 294.0),
        ("imu", "GND", "esp", "GND.1", "black", "GND", 302.0),
        ("imu", "SCL", "esp", "9", "yellow", "I2C SCL", 310.0),
        ("imu", "SDA", "esp", "8", "blue", "I2C SDA", 318.0),
    ];
    for (k, (a, ap, b, bp, c, s, x)) in spec.iter().enumerate() {
        wires.push(wire(k, &parts, (*a, *ap), (*b, *bp), c, s, "", Via::X(*x)));
    }
    let n = wires.len();
    wires.push(wire(n, &parts, ("touch", "SIG"), ("esp", "7"), "green", "TOUCH", "GPIO 7 (INPUT_PULLUP with a button in Wokwi)", Via::Y(14.0, 520.0, 340.0)));
    wires.push(wire(n + 1, &parts, ("touch", "GND"), ("esp", "GND.2"), "black", "GND", "", Via::X(500.0)));
    wires.push(wire(n + 2, &parts, ("touch", "VCC"), ("esp", "3V3.1"), "red", "3V3", "TTP223 only (a push button needs no supply)", Via::Y(6.0, 510.0, 346.0)));
    let bom = vec![
        bl("ESP32-S3-DevKitC-1 (or compatible ESP32-S3 dev board)", 1, "Development board — Espressif distributors and hobby retailers", "U1 SoC", "Different SoC family from Iris: the protocol and UI carry over, the power numbers do not."),
        bl("0.96 in SSD1306 I²C OLED module, 128×64", 1, "Display module — hobby electronics retailers", "U2 micro-OLED", ""),
        bl("MPU-6050-class 6-axis IMU breakout (GY-521 style)", 1, "Sensor module — hobby electronics retailers", "U3 IMU", "I²C address 0x68 (AD0 low)."),
        bl("INMP441-class I²S MEMS microphone breakout", 1, "Sensor module — hobby electronics retailers", "MK1 microphone", "I²S instead of PDM: the chip decimates internally."),
        bl("TTP223 capacitive touch module (or a push button)", 1, "Sensor module", "U7 touch controller", ""),
        bl("Breadboard + jumper wires", 1, "Prototyping supplies", "main board", ""),
        bl("USB-C (or micro-USB, depending on the board) data cable; USB power bank for untethered tests", 1, "Cables / power", "J1, BT1", "The DevKit has no LiPo charger."),
    ];
    let steps = vec![
        st("Try it in Wokwi first", "Firmware tab → target \"ESP32-S3 (Wokwi)\" → follow the \"Open in Wokwi\" steps. The same wiring is simulated in the browser.", &[]),
        st("Power rails", "Red from 3V3 and black from GND to every module.", &[0, 1, 4, 5, 10, 11]),
        st("I²C bus", "OLED and MPU-6050 share GPIO 8 (SDA) and GPIO 9 (SCL).", &[2, 3, 12, 13]),
        st("I²S microphone", "SCK → GPIO 4, WS → GPIO 5, SD → GPIO 6, L/R → GND (left channel).", &[6, 7, 8, 9]),
        st("Touch", "TTP223 SIG → GPIO 7 (set TOUCH_ACTIVE_HIGH 1), or a push button to GND as in Wokwi.", &[14, 15, 16]),
        st("Flash", "Set IRIS_ENABLE_BLE 1 and IRIS_SIM_AUDIO 0, then `pio run -t upload`.", &[]),
    ];
    let differences = vec![
        df("System-on-chip", "ESP32-S3 (dual-core, Wi-Fi + BLE)", "nRF52840-class BLE SoC", "Several times the power; good for UI and protocol work, not for battery-life claims."),
        df("Microphone", "I²S microphone (decimation inside the mic)", "Two PDM microphones, decimation by the SoC or the IRIS-DSP co-processor", "The DSP Lab's decimation filter is only exercised on the custom path."),
        df("IMU", "MPU-6050-class breakout", "BMI270-class IMU", "Noisier, higher power; fine for the complementary filter demo."),
        df("Display", "128×64 OLED over I²C", "640×400 micro-OLED over SPI + optics", "Same UI description, rendered at a different font scale."),
        df("Power", "USB or power bank; no gauge", "LiPo + buck + load switch + battery sense", "The learned policy runs, but with a constant state of charge."),
    ];
    let (power, pnotes) = power_compare(p, Build::Esp32s3, battery_mah, scenario);
    Breadboard {
        build: Build::Esp32s3,
        title: "Iris v0 — ESP32-S3 build (Wokwi-simulatable)".into(),
        summary: "For builders without the XIAO: common modules around an ESP32-S3 DevKit. The same wiring runs in the Wokwi simulator in your browser.".into(),
        width: 720.0,
        height: 400.0,
        parts,
        wires,
        bom,
        steps,
        differences,
        power,
        notes: pnotes,
    }
}

pub fn breadboard(p: &Project, build: Build, battery_mah: f64, scenario: &str) -> Breadboard {
    match build {
        Build::Xiao => xiao(p, battery_mah, scenario),
        Build::Esp32s3 => esp32(p, battery_mah, scenario),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wires_start_and_end_on_their_pins() {
        let p = Project::iris();
        for b in [Build::Xiao, Build::Esp32s3] {
            let bb = breadboard(&p, b, 0.0, "reference");
            for w in &bb.wires {
                let a = find(&bb.parts, &w.from_part, &w.from_pin);
                let z = find(&bb.parts, &w.to_part, &w.to_pin);
                assert!(a.0 > 0.0 && z.0 > 0.0, "{} {}→{} {}", w.from_part, w.from_pin, w.to_part, w.to_pin);
                assert_eq!(w.route.first().unwrap(), &(a.0, a.1));
                assert_eq!(w.route.last().unwrap(), &(z.0, z.1));
                // orthogonal segments only
                for s in w.route.windows(2) {
                    assert!((s[0].0 - s[1].0).abs() < 1e-9 || (s[0].1 - s[1].1).abs() < 1e-9, "diagonal in wire {}", w.id);
                }
            }
            for s in &bb.steps {
                assert!(s.wires.iter().all(|&i| i < bb.wires.len()));
            }
        }
    }

    #[test]
    fn prototype_draws_more_power_than_the_custom_board() {
        let p = Project::iris();
        let x = breadboard(&p, Build::Xiao, 0.0, "reference").power;
        let e = breadboard(&p, Build::Esp32s3, 0.0, "reference").power;
        // custom board figure is the Power Lab's own number
        let custom = crate::power::budget(&p, &UsageProfile::default()).avg_mw;
        assert!((x.custom_avg_mw - custom).abs() < 1e-9);
        assert!(x.proto_avg_mw > x.custom_avg_mw, "{} vs {}", x.proto_avg_mw, x.custom_avg_mw);
        assert!(e.proto_avg_mw > x.proto_avg_mw);
        // hand check of one row: OLED 40 mW × duty + 0.03 × (1 − duty)
        let d = x.proto_rows.iter().find(|r| r.name.contains("SSD1306")).unwrap();
        assert!((d.avg_mw - (40.0 * d.duty + 0.03 * (1.0 - d.duty))).abs() < 1e-9);
        assert_eq!(x.battery_mah, p.design.battery.capacity_mah);
    }
}
