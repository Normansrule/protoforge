//! "Iris v0": the breadboard prototype built from widely available
//! development boards, with a wiring model the web page animates bus by bus.
//!
//! * **XIAO build** (primary, battery powered) — Seeed Studio XIAO nRF52840
//!   Sense: the same Bluetooth Low Energy (BLE) system-on-chip family as Iris,
//!   with an on-board 6-axis inertial measurement unit (IMU, LSM6DS3TR-C on an
//!   internal I²C bus), a pulse-density-modulation (PDM) microphone and a
//!   lithium-polymer (LiPo) charger. A 7-pin SPI SSD1306 OLED stands in for the
//!   micro-OLED (Iris drives its display over SPI too, IF-01). Pin names follow
//!   Seeed's published Arduino variant (`D0`–`D10`; SPI on D8/D9/D10).
//! * **nRF52840 DK build** (Zephyr bring-up) — Nordic's development kit with
//!   external breakouts wired to exactly the pins of the generated Zephyr
//!   devicetree overlay ([`super::targets::ZEPHYR_PINS`]): SPI OLED, PDM
//!   microphone, BMI270 IMU on I²C, touch, and a potentiometer that simulates
//!   the battery-sense voltage so the learned power policy can be exercised.
//! * **ESP32-S3 build** — ESP32-S3-DevKitC-1 + MPU-6050-class IMU + I²C
//!   SSD1306 + INMP441-class I²S microphone; the variant you can simulate in
//!   Wokwi (whose SSD1306 part is I²C-only). Pin names follow Wokwi's board
//!   definition of the DevKitC-1.
//!
//! Every wire carries a `bus` tag (power, ground, battery, i2c, spi, pdm, i2s,
//! gpio) so the diagram can light one bus at a time, and the pin map is
//! derived from the project's interface list (`Project::interfaces`).
//!
//! Module power figures are typical values stated as assumptions (check the
//! datasheets of the modules you buy); the battery-life comparison runs them
//! through the Power Lab's budget (`crate::power::budget`) exactly like the
//! custom board.

use serde::{Deserialize, Serialize};

use crate::power::budget::{budget, scenario_profile};
use crate::project::{Component, PowerProfile, Project};

use super::targets::ZEPHYR_PINS;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Build {
    #[default]
    Xiao,
    Nrf52840dk,
    Esp32s3,
}

#[derive(Clone, Debug, Serialize)]
pub struct Link {
    pub label: String,
    pub url: String,
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
    /// Distributor search links (searches, not quotes).
    pub search: Vec<Link>,
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
    /// "board", "module", "battery", "onboard" (a chip on a development board).
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
    /// "power", "ground", "battery", "i2c", "spi", "pdm", "i2s", "gpio".
    pub bus: String,
    pub signal: String,
    pub note: String,
    /// A trace on the development board (drawn, not wired by you).
    pub onboard: bool,
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

/// One signal of a project interface and where it lands on each platform.
#[derive(Clone, Debug, Serialize)]
pub struct PinMapSignal {
    pub signal: String,
    /// Proposed nRF52840 pin on the custom board (same as the Zephyr overlay).
    pub custom: String,
    /// Pin on this prototype build.
    pub prototype: String,
    pub note: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PinMapRow {
    pub interface: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    pub description: String,
    /// Bus tag used by the wiring diagram ("" when the interface has no wires).
    pub bus: String,
    pub signals: Vec<PinMapSignal>,
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
    pub pinmap: Vec<PinMapRow>,
    pub power: PowerCompare,
    pub notes: Vec<String>,
}

// ------------------------------------------------------------------ geometry helpers

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
    /// Explicit corner points between the two pins.
    Pts(Vec<(f64, f64)>),
}

fn route(parts: &[Part], a: (&str, &str), b: (&str, &str), via: Via) -> Vec<(f64, f64)> {
    let (x1, y1, s1) = find(parts, a.0, a.1);
    let (x2, y2, s2) = find(parts, b.0, b.1);
    let mut r = vec![(x1, y1)];
    match via {
        Via::X(x) => {
            let p1 = stub(x1, y1, &s1, 10.0);
            let p2 = stub(x2, y2, &s2, 10.0);
            r.extend([p1, (x, p1.1), (x, p2.1), p2]);
        }
        Via::Y(y, xa, xb) => {
            let p1 = stub(x1, y1, &s1, 10.0);
            let p2 = stub(x2, y2, &s2, 10.0);
            r.extend([p1, (xa, p1.1), (xa, y), (xb, y), (xb, p2.1), p2]);
        }
        Via::Pts(v) => r.extend(v),
    }
    r.push((x2, y2));
    r.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
    r
}

struct WireSpec<'a> {
    a: (&'a str, &'a str),
    b: (&'a str, &'a str),
    color: &'a str,
    bus: &'a str,
    signal: &'a str,
    note: &'a str,
    via: Via,
}

fn ws<'a>(
    a: (&'a str, &'a str),
    b: (&'a str, &'a str),
    color: &'a str,
    bus: &'a str,
    signal: &'a str,
    note: &'a str,
    via: Via,
) -> WireSpec<'a> {
    WireSpec {
        a,
        b,
        color,
        bus,
        signal,
        note,
        via,
    }
}

fn wires_from(parts: &[Part], specs: Vec<WireSpec>, onboard: Vec<WireSpec>) -> Vec<Wire> {
    let mut out = Vec::new();
    for (k, s) in specs
        .into_iter()
        .map(|s| (s, false))
        .chain(onboard.into_iter().map(|s| (s, true)))
        .enumerate()
    {
        let (s, ob) = s;
        out.push(Wire {
            id: k,
            from_part: s.a.0.into(),
            from_pin: s.a.1.into(),
            to_part: s.b.0.into(),
            to_pin: s.b.1.into(),
            color: s.color.into(),
            bus: s.bus.into(),
            signal: s.signal.into(),
            note: s.note.into(),
            onboard: ob,
            route: route(parts, s.a, s.b, s.via),
        });
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn part(id: &str, label: &str, sub: &str, kind: &str, x: f64, y: f64, w: f64, h: f64, pins: Vec<Pin>) -> Part {
    Part {
        id: id.into(),
        label: label.into(),
        sublabel: sub.into(),
        kind: kind.into(),
        x,
        y,
        w,
        h,
        pins,
    }
}

/// Pins on one vertical edge, evenly spaced from `y0` by `dy`.
fn column(names: &[&str], x: f64, y0: f64, dy: f64, side: &str) -> Vec<Pin> {
    names.iter().enumerate().map(|(k, n)| pin(n, x, y0 + dy * k as f64, side)).collect()
}

// ------------------------------------------------------------------ BOM helpers

/// Percent-encode a search query (RFC 3986 unreserved characters kept).
pub fn url_encode(q: &str) -> String {
    q.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn search_links(q: &str) -> Vec<Link> {
    if q.is_empty() {
        return vec![];
    }
    let e = url_encode(q);
    vec![
        Link {
            label: "Octopart".into(),
            url: format!("https://octopart.com/search?q={e}"),
        },
        Link {
            label: "Digi-Key".into(),
            url: format!("https://www.digikey.com/en/products/result?keywords={e}"),
        },
    ]
}

fn bl(item: &str, qty: u32, category: &str, stands: &str, notes: &str, query: &str) -> BomLine {
    BomLine {
        item: item.into(),
        qty,
        category: category.into(),
        stands_in_for: stands.into(),
        notes: notes.into(),
        search: search_links(query),
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

/// Wire ids whose `bus` is one of `buses` (for step highlighting).
fn ids(wires: &[Wire], pred: impl Fn(&Wire) -> bool) -> Vec<usize> {
    wires.iter().filter(|w| pred(w)).map(|w| w.id).collect()
}

// ------------------------------------------------------------------ power

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
    let soc_duty = find_comp(p, "system-on-chip").map(|c| c.power.duty).unwrap_or(0.25);
    let mic_duty = duty_of(p, "microphone", 0.25);
    let core_v = p.design.power_rails.core_v.max(0.5);
    let bat_v = p.design.battery.nominal_v.max(3.3);
    // same silicon on a higher rail: same current → power × V_rail / V_core
    let scale = |key: &str, a: f64, s: f64, k: f64| {
        find_comp(p, key)
            .map(|c| (c.power.active_mw * k, c.power.sleep_mw * k))
            .unwrap_or((a, s))
    };
    let oled = comp("OLED", "SSD1306 128x64 OLED module (SPI)", "SYS-OPT", 40.0, 0.03, disp_duty);
    let touch = comp("TOUCH", "TTP223 touch module", "SYS-SNS", 0.02, 0.02, 1.0);
    let oled_note =
        "SSD1306 OLED 40 mW: ≈12 mA at 3.3 V with about a quarter of the pixels lit (typical; grows with lit pixels) — measure your module"
            .to_string();
    match build {
        Build::Xiao => {
            let k = 3.3 / core_v;
            let (sa, ss) = scale("system-on-chip", 21.0, 0.006, k);
            let (ia, is) = scale("imu", 0.9, 0.01, k);
            let (ma, ms) = scale("microphone", 0.9, 0.02, k);
            let comps = vec![
                comp("XIAO", "nRF52840 system-on-chip (on the XIAO)", "SYS-CMP", sa, ss, soc_duty),
                oled,
                comp("IMU", "LSM6DS3TR-C IMU (on the XIAO)", "SYS-SNS", ia, is, 1.0),
                comp("MIC", "PDM microphone (on the XIAO)", "SYS-AUD", ma, ms, mic_duty),
                touch,
                comp(
                    "BRD",
                    "XIAO board overhead (regulator, charger, flash standby)",
                    "SYS-PWR",
                    0.2,
                    0.2,
                    1.0,
                ),
            ];
            let notes = vec![
                format!("system-on-chip, IMU and microphone: the Iris BOM figures × {k:.2} (same current at the XIAO's 3.3 V rail instead of the {core_v:.1} V core rail)"),
                oled_note,
                "TTP223 ≈ a few µA; XIAO board overhead 0.2 mW — assumptions".into(),
            ];
            let eff = 3.3 / bat_v;
            (
                comps,
                notes,
                format!("on-board 3.3 V linear regulator (efficiency ≈ 3.3 V / {bat_v:.1} V)"),
                eff,
            )
        }
        Build::Nrf52840dk => {
            let k = 3.0 / core_v;
            let (sa, ss) = scale("system-on-chip", 21.0, 0.006, k);
            let (ia, is) = scale("imu", 0.9, 0.01, k);
            let (ma, ms) = scale("microphone", 0.9, 0.02, k);
            let comps = vec![
                comp("DK", "nRF52840 system-on-chip (on the DK)", "SYS-CMP", sa, ss, soc_duty),
                oled,
                comp("IMU", "BMI270 IMU breakout", "SYS-SNS", ia, is, 1.0),
                comp("MIC", "PDM microphone breakout", "SYS-AUD", ma, ms, mic_duty),
                touch,
                comp("POT", "10 kΩ battery-simulator potentiometer", "SYS-PWR", 0.9, 0.9, 1.0),
            ];
            let notes = vec![
                format!("nRF52840, IMU and microphone: the Iris BOM figures × {k:.2} (the DK's 3.0 V VDD instead of the {core_v:.1} V core rail)"),
                oled_note,
                "the 10 kΩ potentiometer across 3.0 V burns V²/R = 0.9 mW on its own — Iris's real divider uses 2.5 MΩ (R-014) for exactly this reason".into(),
                "the DK's interface MCU runs from USB and is not counted; measure the nRF52840 alone on the DK's current-measurement header (see the DK user guide)".into(),
            ];
            let eff = 3.0 / bat_v;
            (
                comps,
                notes,
                format!(
                    "equivalent: a LiPo feeding 3.0 V VDD through a linear regulator (≈ 3.0 V / {bat_v:.1} V); the DK itself runs from USB"
                ),
                eff,
            )
        }
        Build::Esp32s3 => {
            let comps = vec![
                comp("ESP", "ESP32-S3 module with BLE", "SYS-CMP", 100.0, 2.5, soc_duty),
                comp("OLED", "SSD1306 128x64 OLED module (I2C)", "SYS-OPT", 40.0, 0.03, disp_duty),
                comp("MPU", "MPU-6050 IMU module", "SYS-SNS", 12.9, 0.02, 1.0),
                comp("I2S", "INMP441 I2S microphone module", "SYS-AUD", 4.6, 0.04, mic_duty),
                touch,
                comp(
                    "DEVKIT",
                    "DevKit overhead (power LED, USB-UART bridge, regulator)",
                    "SYS-PWR",
                    5.0,
                    5.0,
                    1.0,
                ),
            ];
            let notes = vec![
                "ESP32-S3 100 mW active (CPU + radio events) / 2.5 mW light sleep — typical, depends heavily on firmware".into(),
                "MPU-6050 ≈ 3.9 mA and INMP441 ≈ 1.4 mA at 3.3 V (datasheet-class figures); DevKit overhead 5 mW — assumptions".into(),
                oled_note,
            ];
            let eff = 3.3 / bat_v;
            (
                comps,
                notes,
                "DevKit 3.3 V linear regulator fed from a LiPo + charger module or a USB power bank".into(),
                eff,
            )
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
        })
        .collect()
}

pub fn power_compare(p: &Project, build: Build, battery_mah: f64, scenario: &str) -> (PowerCompare, Vec<String>) {
    let prof = scenario_profile(scenario).unwrap_or_default();
    let custom = budget(p, &prof);
    let (comps, notes, reg, eff) = proto_components(p, build);
    let mut q = p.clone();
    q.components = comps;
    q.design.power_rails.buck_efficiency = eff;
    q.design.power_rails.quiescent_ua = 0.0;
    q.design.battery.capacity_mah = if battery_mah > 0.0 {
        battery_mah
    } else {
        p.design.battery.capacity_mah
    };
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

// ------------------------------------------------------------------ pin map

fn zp(name: &str) -> String {
    ZEPHYR_PINS
        .iter()
        .find(|p| p.0 == name)
        .map(|p| format!("P{}.{:02}", p.1, p.2))
        .unwrap_or_else(|| "—".into())
}

fn sig(signal: &str, custom: String, proto: &str, note: &str) -> PinMapSignal {
    PinMapSignal {
        signal: signal.into(),
        custom,
        prototype: proto.into(),
        note: note.into(),
    }
}

/// Pin map derived from the project's interfaces: each electrical interface
/// (SPI, PDM, I²C, power, BLE) becomes the signals it needs, with the proposed
/// custom-board pin and the pin on the chosen prototype build.
pub fn pinmap(p: &Project, build: Build) -> Vec<PinMapRow> {
    let mut out = Vec::new();
    for itf in &p.interfaces {
        let k = itf.kind.to_uppercase();
        let touches = |s: &str| itf.from == s || itf.to == s;
        let (bus, signals): (&str, Vec<PinMapSignal>) = match k.as_str() {
            "SPI" if touches("SYS-OPT") => (
                "spi",
                match build {
                    Build::Xiao => vec![
                        sig("SCK", zp("DISP_SCK"), "D8 (SCK)", "hardware SPI"),
                        sig("MOSI", zp("DISP_MOSI"), "D10 (MOSI)", ""),
                        sig("CS", zp("DISP_CS"), "D7", "active low"),
                        sig("DC", zp("DISP_DC"), "D2", "data / command"),
                        sig("RST", zp("DISP_RST"), "D3", "active low"),
                    ],
                    Build::Nrf52840dk => vec![
                        sig("SCK", zp("DISP_SCK"), &zp("DISP_SCK"), "same pin as the custom board"),
                        sig("MOSI", zp("DISP_MOSI"), &zp("DISP_MOSI"), ""),
                        sig("CS", zp("DISP_CS"), &zp("DISP_CS"), ""),
                        sig("DC", zp("DISP_DC"), &zp("DISP_DC"), ""),
                        sig("RST", zp("DISP_RST"), &zp("DISP_RST"), ""),
                    ],
                    Build::Esp32s3 => vec![
                        sig(
                            "SCL (I²C)",
                            zp("DISP_SCK"),
                            "GPIO 9",
                            "Wokwi's SSD1306 is I²C-only, so this build swaps SPI for I²C",
                        ),
                        sig("SDA (I²C)", zp("DISP_MOSI"), "GPIO 8", "shared with the IMU"),
                    ],
                },
            ),
            "SPI" => (
                "",
                vec![sig(
                    "SPI to co-processor",
                    "(SPIM, see Chip Lab)".into(),
                    "not prototyped",
                    "the SoC's hardware PDM decimator replaces the IRIS-DSP co-processor on every breadboard",
                )],
            ),
            "PDM" => (
                if build == Build::Esp32s3 { "i2s" } else { "pdm" },
                match build {
                    Build::Xiao => vec![
                        sig(
                            "PDM CLK",
                            zp("PDM_CLK"),
                            "on-board (PIN_PDM_CLK)",
                            "microphone soldered on the XIAO",
                        ),
                        sig(
                            "PDM DATA",
                            zp("PDM_DAT"),
                            "on-board (PIN_PDM_DIN)",
                            "one microphone: no beamforming",
                        ),
                    ],
                    Build::Nrf52840dk => vec![
                        sig("PDM CLK", zp("PDM_CLK"), &zp("PDM_CLK"), "to the breakout's CLK"),
                        sig("PDM DATA", zp("PDM_DAT"), &zp("PDM_DAT"), "SEL chooses the clock edge (channel)"),
                    ],
                    Build::Esp32s3 => vec![
                        sig("I²S SCK", zp("PDM_CLK"), "GPIO 4", "I²S microphone instead of PDM"),
                        sig("I²S WS", "—".into(), "GPIO 5", ""),
                        sig("I²S SD", zp("PDM_DAT"), "GPIO 6", "not simulated in Wokwi"),
                    ],
                },
            ),
            "I2C" => (
                "i2c",
                match build {
                    Build::Xiao => vec![
                        sig("SDA", zp("I2C_SDA"), "on-board (Wire1)", "IMU on the XIAO's internal bus"),
                        sig("SCL", zp("I2C_SCL"), "on-board (Wire1)", ""),
                        sig("IMU INT", zp("IMU_INT"), "on-board", "polled in the Arduino sketch"),
                        sig("TOUCH", zp("TOUCH_INT"), "D0", "TTP223 output (digital, not I²C)"),
                    ],
                    Build::Nrf52840dk => vec![
                        sig("SDA", zp("I2C_SDA"), &zp("I2C_SDA"), "BMI270 breakout"),
                        sig("SCL", zp("I2C_SCL"), &zp("I2C_SCL"), ""),
                        sig("IMU INT", zp("IMU_INT"), &zp("IMU_INT"), ""),
                        sig("TOUCH", zp("TOUCH_INT"), &zp("TOUCH_INT"), "TTP223 output"),
                    ],
                    Build::Esp32s3 => vec![
                        sig("SDA", zp("I2C_SDA"), "GPIO 8", "MPU-6050 + OLED"),
                        sig("SCL", zp("I2C_SCL"), "GPIO 9", ""),
                        sig("TOUCH", zp("TOUCH_INT"), "GPIO 7", "push button in Wokwi"),
                    ],
                },
            ),
            "POWER" if touches("SYS-OPT") => (
                "power",
                vec![sig(
                    "DISP_EN",
                    zp("DISP_EN"),
                    match build {
                        Build::Nrf52840dk => "P1.08 (left open)",
                        _ => "—",
                    },
                    "the OLED module runs from 3.3 V permanently; Iris switches the display rail with a load switch",
                )],
            ),
            "POWER" => (
                if build == Build::Esp32s3 { "power" } else { "battery" },
                vec![
                    sig(
                        "VBAT sense",
                        zp("VBAT_SENSE"),
                        match build {
                            Build::Xiao => "on-board divider (PIN_VBAT, enable VBAT_ENABLE low)",
                            Build::Nrf52840dk => "P0.02 ← potentiometer wiper",
                            Build::Esp32s3 => "— (no gauge)",
                        },
                        "feeds the learned power policy's state of charge",
                    ),
                    sig(
                        "Supply",
                        "1.8 V buck".into(),
                        match build {
                            Build::Xiao => "LiPo → BAT pads → 3.3 V LDO",
                            Build::Nrf52840dk => "USB → 3.0 V VDD",
                            Build::Esp32s3 => "USB → 3.3 V LDO",
                        },
                        "",
                    ),
                ],
            ),
            "BLE" => (
                "",
                vec![sig(
                    "2.4 GHz radio",
                    "AE1 chip antenna".into(),
                    match build {
                        Build::Xiao => "on-module antenna",
                        Build::Nrf52840dk => "on-board PCB antenna",
                        Build::Esp32s3 => "module antenna (Bluetooth not simulated in Wokwi)",
                    },
                    "same GATT protocol on every build",
                )],
            ),
            _ => continue,
        };
        out.push(PinMapRow {
            interface: itf.id.clone(),
            kind: itf.kind.clone(),
            from: itf.from.clone(),
            to: itf.to.clone(),
            description: itf.description.clone(),
            bus: bus.into(),
            signals,
        });
    }
    out
}

// ------------------------------------------------------------------ XIAO build

fn xiao(p: &Project, battery_mah: f64, scenario: &str) -> Breadboard {
    let (xl, xr, top) = (330.0, 490.0, 70.0);
    let mut xp = column(&["D0", "D1", "D2", "D3", "D4/SDA", "D5/SCL", "D6/TX"], xl, 100.0, 20.0, "l");
    xp.extend(column(
        &["5V", "GND", "3V3", "D10/MOSI", "D9/MISO", "D8/SCK", "D7/RX"],
        xr,
        100.0,
        20.0,
        "r",
    ));
    xp.push(pin("BAT+", 375.0, 300.0, "b"));
    xp.push(pin("BAT-", 425.0, 300.0, "b"));
    let cap = if battery_mah > 0.0 {
        battery_mah
    } else {
        p.design.battery.capacity_mah
    };
    let parts = vec![
        part(
            "xiao",
            "XIAO nRF52840 Sense",
            "BLE SoC · IMU · PDM mic · LiPo charger",
            "board",
            xl,
            top,
            xr - xl,
            230.0,
            xp,
        ),
        part(
            "soc",
            "nRF52840",
            "Cortex-M4F · BLE",
            "onboard",
            385.0,
            88.0,
            50.0,
            36.0,
            vec![pin("I2C1", 398.0, 124.0, "b"), pin("PDM", 422.0, 124.0, "b")],
        ),
        part(
            "imu",
            "LSM6DS3TR-C",
            "IMU",
            "onboard",
            346.0,
            250.0,
            56.0,
            30.0,
            vec![pin("I2C1", 374.0, 250.0, "t")],
        ),
        part(
            "mic",
            "PDM mic",
            "MEMS",
            "onboard",
            418.0,
            250.0,
            56.0,
            30.0,
            vec![pin("PDM", 446.0, 250.0, "t")],
        ),
        part(
            "oled",
            "OLED 128×64 (SPI)",
            "SSD1306 7-pin module",
            "module",
            600.0,
            80.0,
            200.0,
            170.0,
            column(&["GND", "VCC", "D0/SCK", "D1/MOSI", "RES", "DC", "CS"], 600.0, 100.0, 20.0, "l"),
        ),
        part(
            "touch",
            "TTP223 touch",
            "capacitive pad, active-high",
            "module",
            80.0,
            110.0,
            120.0,
            95.0,
            column(&["VCC", "SIG", "GND"], 200.0, 130.0, 25.0, "r"),
        ),
        part(
            "lipo",
            &format!("LiPo 3.7 V {cap:.0} mAh"),
            "with protection circuit",
            "battery",
            340.0,
            360.0,
            120.0,
            60.0,
            vec![pin("+", 375.0, 360.0, "t"), pin("-", 425.0, 360.0, "t")],
        ),
    ];
    let specs = vec![
        ws(
            ("xiao", "GND"),
            ("oled", "GND"),
            "black",
            "ground",
            "GND",
            "common ground",
            Via::X(510.0),
        ),
        ws(
            ("xiao", "3V3"),
            ("oled", "VCC"),
            "red",
            "power",
            "3V3",
            "3.3 V supply (most modules accept 3.3–5 V — check yours)",
            Via::X(522.0),
        ),
        ws(
            ("xiao", "D8/SCK"),
            ("oled", "D0/SCK"),
            "yellow",
            "spi",
            "SPI SCK",
            "SPI clock (the module labels it D0 or SCK/SCL)",
            Via::X(546.0),
        ),
        ws(
            ("xiao", "D10/MOSI"),
            ("oled", "D1/MOSI"),
            "blue",
            "spi",
            "SPI MOSI",
            "SPI data to the display (labelled D1 or SDA/MOSI)",
            Via::X(534.0),
        ),
        ws(
            ("xiao", "D7/RX"),
            ("oled", "CS"),
            "orange",
            "spi",
            "SPI CS",
            "chip select, active low",
            Via::X(558.0),
        ),
        ws(
            ("xiao", "D2"),
            ("oled", "DC"),
            "purple",
            "spi",
            "DC",
            "data / command select",
            Via::Y(20.0, 312.0, 570.0),
        ),
        ws(
            ("xiao", "D3"),
            ("oled", "RES"),
            "white",
            "spi",
            "RES",
            "display reset, active low",
            Via::Y(10.0, 304.0, 580.0),
        ),
        ws(
            ("touch", "SIG"),
            ("xiao", "D0"),
            "green",
            "gpio",
            "TOUCH",
            "digital input, sampled at 1 kHz by the gesture FSM",
            Via::X(265.0),
        ),
        ws(
            ("touch", "VCC"),
            ("xiao", "3V3"),
            "red",
            "power",
            "3V3",
            "",
            Via::Y(330.0, 225.0, 505.0),
        ),
        ws(
            ("touch", "GND"),
            ("xiao", "GND"),
            "black",
            "ground",
            "GND",
            "",
            Via::Y(342.0, 240.0, 515.0),
        ),
        ws(
            ("lipo", "+"),
            ("xiao", "BAT+"),
            "red",
            "battery",
            "VBAT",
            "solder to the BAT+ pad on the underside — mind the polarity",
            Via::X(375.0),
        ),
        ws(
            ("lipo", "-"),
            ("xiao", "BAT-"),
            "black",
            "ground",
            "GND",
            "solder to the BAT− pad",
            Via::X(425.0),
        ),
    ];
    let onboard = vec![
        ws(
            ("soc", "I2C1"),
            ("imu", "I2C1"),
            "blue",
            "i2c",
            "I²C (internal Wire1)",
            "on-board trace: SDA + SCL to the IMU",
            Via::Pts(vec![(398.0, 236.0), (374.0, 236.0)]),
        ),
        ws(
            ("soc", "PDM"),
            ("mic", "PDM"),
            "purple",
            "pdm",
            "PDM CLK + DATA",
            "on-board trace to the microphone",
            Via::Pts(vec![(422.0, 236.0), (446.0, 236.0)]),
        ),
    ];
    let wires = wires_from(&parts, specs, onboard);
    let bom = vec![
        bl(
            "Seeed Studio XIAO nRF52840 Sense",
            1,
            "Development board — Seeed Studio store and electronics distributors",
            "U1 SoC, U3 IMU, MK1 microphone, U4 charger",
            "Get the Sense variant (IMU + microphone). Headers are usually supplied loose.",
            "XIAO nRF52840 Sense",
        ),
        bl(
            "0.96 in SSD1306 128×64 OLED module, 7-pin SPI (GND VCC D0 D1 RES DC CS)",
            1,
            "Display module — hobby electronics retailers / marketplaces",
            "U2 micro-OLED (same SPI interface, IF-01)",
            "Pin order and labels vary between sellers; some 7-pin boards need a jumper change for SPI.",
            "SSD1306 128x64 OLED SPI",
        ),
        bl(
            "TTP223 capacitive touch module",
            1,
            "Sensor module — hobby electronics retailers",
            "U7 touch controller + E1 touch pad",
            "Momentary / active-high by default; the A/B jumpers change it.",
            "TTP223 touch module",
        ),
        bl(
            "3.7 V LiPo cell with protection circuit",
            1,
            "Battery — hobby / RC and electronics retailers",
            "BT1 LiPo cell",
            "Match the capacity you simulate; the XIAO charges it over USB-C.",
            "3.7V LiPo battery protection circuit",
        ),
        bl(
            "Half-size solderless breadboard",
            1,
            "Prototyping supplies",
            "SYS-PCB main board",
            "400- or 830-point.",
            "solderless breadboard",
        ),
        bl(
            "Male–male jumper wires",
            10,
            "Prototyping supplies",
            "board traces",
            "Colour code: red 3.3 V, black ground, yellow SCK, blue MOSI, orange CS, purple DC, white RES, green touch.",
            "jumper wires male male",
        ),
        bl(
            "USB-C data cable",
            1,
            "Cables",
            "J1 USB-C",
            "A charge-only cable will not program the board.",
            "",
        ),
        bl(
            "Optional: 45° acrylic or glass beam-splitter + small lens",
            1,
            "Optics — maker optics suppliers",
            "OPT1 collimating lens + combiner",
            "Look at the OLED through it to preview the heads-up effect (see the Optics Lab).",
            "",
        ),
    ];
    let steps = vec![
        st(
            "Seat the XIAO",
            "Solder the headers, press the XIAO across the breadboard's centre channel with USB-C facing outward, so each pin gets its own row. The IMU and microphone are already on the board (dashed traces).",
            &ids(&wires, |w| w.onboard),
        ),
        st(
            "Share ground and 3.3 V",
            "Run black from GND and red from 3V3 to the OLED and to the touch module. Every module must share the same ground.",
            &ids(&wires, |w| !w.onboard && w.from_part != "lipo" && (w.bus == "power" || w.bus == "ground")),
        ),
        st(
            "SPI to the OLED",
            "Yellow D8 (SCK) → D0, blue D10 (MOSI) → D1, orange D7 → CS, purple D2 → DC, white D3 → RES. SPI is what the Iris micro-OLED uses (IF-01), so the display driver code carries over.",
            &ids(&wires, |w| w.bus == "spi"),
        ),
        st("Touch input", "Green from the TTP223 SIG pin to D0. Touch the pad: its LED lights and D0 goes high.", &ids(&wires, |w| w.bus == "gpio")),
        st(
            "Battery",
            "With nothing plugged into USB, solder the LiPo leads to the BAT+ / BAT− pads on the underside (red to +). Double-check polarity — reversing a LiPo can destroy the board.",
            &ids(&wires, |w| w.from_part == "lipo"),
        ),
        st(
            "Flash the firmware",
            "Firmware tab → target \"XIAO (Arduino)\" → download the zip, open it in PlatformIO (or the Arduino IDE with the Seeed nRF52 core), upload over USB-C.",
            &[],
        ),
        st(
            "Test",
            "The clock screen appears. Tap: next screen; double tap: back to the clock; long press: display off. Open nRF Connect for Mobile, connect to \"Iris v0\" and write a Captions packet.",
            &[],
        ),
        st(
            "Measure",
            "Put a USB power meter or a power profiler between battery and board, run each screen for a minute, and compare the current with the power table — then correct the assumptions.",
            &[],
        ),
    ];
    let differences = vec![
        df(
            "System-on-chip",
            "nRF52840 on the XIAO module (3.3 V rail)",
            "nRF52840-class SoC on the custom board (1.8 V core rail from a buck)",
            "Same firmware architecture and radio; the custom rail cuts SoC power by ≈ 1.8/3.3.",
        ),
        df(
            "Display",
            "128×64 monochrome OLED over SPI, viewed directly",
            "640×400 micro-OLED over SPI through a collimating lens and 45° combiner",
            "Same bus and UI description; the lab scales the same widgets (font scale 1 vs 5).",
        ),
        df(
            "Microphones",
            "One on-board PDM microphone",
            "Two PDM microphones, 14 mm endfire pair + beamformer",
            "No directivity on the prototype (R-009 is verified by the DSP Lab, not the breadboard).",
        ),
        df(
            "IMU",
            "On-board LSM6DS3TR-C (internal I²C)",
            "BMI270-class IMU on the main I²C bus",
            "Different noise figures — re-run the Control Lab with the prototype IMU's datasheet values.",
        ),
        df(
            "Touch",
            "TTP223 module, one pad",
            "Capacitive touch controller + temple touch strip",
            "Same digital signal into the gesture FSM; thresholds identical.",
        ),
        df(
            "Power path",
            "LiPo → XIAO charger → 3.3 V linear regulator",
            "LiPo → charger → 1.8 V synchronous buck + display load switch",
            "Different efficiency and a smaller display — see the comparison table.",
        ),
        df(
            "Size and mass",
            "Breadboard, wires, loose modules",
            "Main board in the right temple pod",
            "Proves function, not wearability; the CAD and PCB labs cover packaging.",
        ),
    ];
    let (power, pnotes) = power_compare(p, Build::Xiao, battery_mah, scenario);
    Breadboard {
        build: Build::Xiao,
        title: "Iris v0 — XIAO nRF52840 Sense build".into(),
        summary: "The primary prototype: one thumb-sized board already contains Iris's SoC family, IMU, microphone and charger, so only an SPI display, a touch pad and a battery are added.".into(),
        width: 860.0,
        height: 440.0,
        parts,
        wires,
        bom,
        steps,
        differences,
        pinmap: pinmap(p, Build::Xiao),
        power,
        notes: pnotes,
    }
}

// ------------------------------------------------------------------ nRF52840 DK build

fn dk(p: &Project, battery_mah: f64, scenario: &str) -> Breadboard {
    let (dx, dw) = (340.0, 220.0);
    let z = |n: &str| zp(n);
    let left_names = [
        "VDD".to_string(),
        "GND".to_string(),
        z("DISP_SCK"),
        z("DISP_MOSI"),
        z("DISP_CS"),
        z("DISP_DC"),
        z("DISP_RST"),
    ];
    let mut dpins: Vec<Pin> = left_names
        .iter()
        .enumerate()
        .map(|(k, n)| pin(n, dx, 80.0 + 24.0 * k as f64, "l"))
        .collect();
    dpins.push(pin(&z("PDM_CLK"), dx, 296.0, "l"));
    dpins.push(pin(&z("PDM_DAT"), dx, 320.0, "l"));
    let right = [
        "VDD.2".to_string(),
        "GND.2".to_string(),
        z("I2C_SDA"),
        z("I2C_SCL"),
        z("IMU_INT"),
        z("TOUCH_INT"),
    ];
    dpins.extend(right.iter().enumerate().map(|(k, n)| pin(n, dx + dw, 80.0 + 24.0 * k as f64, "r")));
    dpins.push(pin(&z("VBAT_SENSE"), dx + dw, 296.0, "r"));
    let parts = vec![
        part(
            "dk",
            "nRF52840 DK",
            "Nordic development kit · Zephyr bring-up",
            "board",
            dx,
            50.0,
            dw,
            310.0,
            dpins,
        ),
        part(
            "oled",
            "OLED 128×64 (SPI)",
            "SSD1306 7-pin module",
            "module",
            40.0,
            60.0,
            180.0,
            190.0,
            column(&["GND", "VCC", "D0/SCK", "D1/MOSI", "RES", "DC", "CS"], 220.0, 84.0, 22.0, "r"),
        ),
        part(
            "mic",
            "PDM microphone",
            "MEMS breakout",
            "module",
            60.0,
            280.0,
            160.0,
            110.0,
            column(&["3V", "GND", "SEL", "CLK", "DAT"], 220.0, 296.0, 20.0, "r"),
        ),
        part(
            "imu",
            "BMI270 IMU",
            "I²C breakout, 0x68",
            "module",
            680.0,
            60.0,
            180.0,
            120.0,
            column(&["VCC", "GND", "SDA", "SCL", "INT"], 680.0, 80.0, 20.0, "l"),
        ),
        part(
            "touch",
            "TTP223 touch",
            "active-high output",
            "module",
            680.0,
            196.0,
            180.0,
            84.0,
            column(&["VCC", "SIG", "GND"], 680.0, 214.0, 22.0, "l"),
        ),
        part(
            "pot",
            "10 kΩ potentiometer",
            "simulated battery sense",
            "module",
            680.0,
            296.0,
            180.0,
            94.0,
            column(&["VCC", "WIPER", "GND"], 680.0, 316.0, 24.0, "l"),
        ),
    ];
    let (sck, mosi, cs, dc, rst) = (z("DISP_SCK"), z("DISP_MOSI"), z("DISP_CS"), z("DISP_DC"), z("DISP_RST"));
    let (pclk, pdat, sda, scl, int, tch, vb) = (
        z("PDM_CLK"),
        z("PDM_DAT"),
        z("I2C_SDA"),
        z("I2C_SCL"),
        z("IMU_INT"),
        z("TOUCH_INT"),
        z("VBAT_SENSE"),
    );
    let specs = vec![
        ws(("oled", "GND"), ("dk", "GND"), "black", "ground", "GND", "", Via::X(270.0)),
        ws(
            ("oled", "VCC"),
            ("dk", "VDD"),
            "red",
            "power",
            "VDD 3.0 V",
            "the DK's VDD is 3.0 V by default — fine for 3.3 V modules",
            Via::X(250.0),
        ),
        ws(
            ("oled", "D0/SCK"),
            ("dk", &sck),
            "yellow",
            "spi",
            "SPI SCK",
            "SPIM3 clock (overlay: spi3)",
            Via::X(280.0),
        ),
        ws(("oled", "D1/MOSI"), ("dk", &mosi), "blue", "spi", "SPI MOSI", "", Via::X(286.0)),
        ws(
            ("oled", "RES"),
            ("dk", &rst),
            "white",
            "spi",
            "RES",
            "disp-rst-gpios",
            Via::X(304.0),
        ),
        ws(("oled", "DC"), ("dk", &dc), "purple", "spi", "DC", "disp-dc-gpios", Via::X(298.0)),
        ws(
            ("oled", "CS"),
            ("dk", &cs),
            "orange",
            "spi",
            "CS",
            "disp-cs-gpios (active low)",
            Via::X(292.0),
        ),
        ws(("mic", "3V"), ("dk", "VDD"), "red", "power", "VDD", "", Via::X(236.0)),
        ws(("mic", "GND"), ("dk", "GND"), "black", "ground", "GND", "", Via::X(243.0)),
        ws(
            ("mic", "SEL"),
            ("dk", "GND"),
            "black",
            "ground",
            "SEL",
            "channel select — the firmware reads PDM_CHAN_LEFT; swap SEL or the channel if you record silence",
            Via::X(262.0),
        ),
        ws(
            ("mic", "CLK"),
            ("dk", &pclk),
            "purple",
            "pdm",
            "PDM CLK",
            "PDM clock from the nRF52840",
            Via::X(312.0),
        ),
        ws(
            ("mic", "DAT"),
            ("dk", &pdat),
            "green",
            "pdm",
            "PDM DATA",
            "1-bit data, decimated to 16 kHz PCM by the PDM peripheral",
            Via::X(320.0),
        ),
        ws(("imu", "VCC"), ("dk", "VDD.2"), "red", "power", "VDD", "", Via::X(590.0)),
        ws(("imu", "GND"), ("dk", "GND.2"), "black", "ground", "GND", "", Via::X(600.0)),
        ws(
            ("imu", "SDA"),
            ("dk", &sda),
            "blue",
            "i2c",
            "I²C SDA",
            "TWIM0 (overlay: i2c0); breakouts usually carry pull-ups",
            Via::X(610.0),
        ),
        ws(("imu", "SCL"), ("dk", &scl), "yellow", "i2c", "I²C SCL", "400 kHz", Via::X(620.0)),
        ws(
            ("imu", "INT"),
            ("dk", &int),
            "green",
            "gpio",
            "IMU INT",
            "data-ready interrupt",
            Via::X(630.0),
        ),
        ws(("touch", "VCC"), ("dk", "VDD.2"), "red", "power", "VDD", "", Via::X(640.0)),
        ws(
            ("touch", "SIG"),
            ("dk", &tch),
            "green",
            "gpio",
            "TOUCH",
            "touch-gpios, sampled at 1 kHz",
            Via::X(636.0),
        ),
        ws(("touch", "GND"), ("dk", "GND.2"), "black", "ground", "GND", "", Via::X(650.0)),
        ws(("pot", "VCC"), ("dk", "VDD.2"), "red", "power", "VDD", "", Via::X(660.0)),
        ws(
            ("pot", "WIPER"),
            ("dk", &vb),
            "orange",
            "battery",
            "VBAT sense",
            "AIN0: the firmware multiplies by 2.5 (Iris divider), so 1.5 V on the wiper reads as 3.75 V",
            Via::X(645.0),
        ),
        ws(("pot", "GND"), ("dk", "GND.2"), "black", "ground", "GND", "", Via::X(668.0)),
    ];
    let wires = wires_from(&parts, specs, vec![]);
    let bom = vec![
        bl(
            "Nordic Semiconductor nRF52840 DK",
            1,
            "Development kit — Nordic distributors",
            "U1 SoC (same chip as the custom board)",
            "Has an on-board debugger, so `west flash` works with just a USB cable. Check the DK user guide for header pin labels.",
            "nRF52840 DK",
        ),
        bl(
            "0.96 in SSD1306 128×64 OLED module, 7-pin SPI",
            1,
            "Display module — hobby electronics retailers",
            "U2 micro-OLED (SPI)",
            "Build with -DIRIS_PANEL_SSD1306=ON so the Zephyr skeleton drives this panel.",
            "SSD1306 128x64 OLED SPI",
        ),
        bl(
            "PDM MEMS microphone breakout (e.g. Adafruit PDM MEMS Microphone Breakout)",
            1,
            "Sensor module — Adafruit and distributors",
            "MK1 microphone (PDM, IF-02)",
            "Pins are typically 3V, GND, SEL, CLK, DAT — check your breakout.",
            "PDM MEMS microphone breakout",
        ),
        bl(
            "BMI270 6-axis IMU breakout (3.3 V I/O, I²C)",
            1,
            "Sensor module — electronics distributors",
            "U3 IMU (same part family as the overlay's bosch,bmi270 node)",
            "Address 0x68 or 0x69 depending on a jumper — the overlay uses 0x68.",
            "BMI270 breakout",
        ),
        bl(
            "TTP223 capacitive touch module",
            1,
            "Sensor module",
            "U7 touch controller",
            "",
            "TTP223 touch module",
        ),
        bl(
            "10 kΩ potentiometer (breadboard-friendly)",
            1,
            "Passive components",
            "R1/R2 battery divider",
            "Turn it to walk the learned power policy through its battery buckets.",
            "10k potentiometer breadboard",
        ),
        bl(
            "Solderless breadboard + jumper wires (male–male and male–female)",
            1,
            "Prototyping supplies",
            "board traces",
            "The DK has pin headers; male–female jumpers are convenient.",
            "jumper wires male female",
        ),
        bl(
            "USB cable for the DK",
            1,
            "Cables",
            "J1",
            "Check the connector on your DK revision.",
            "",
        ),
    ];
    let steps = vec![
        st(
            "Power the DK",
            "Plug the DK's debugger USB port into your computer and switch it on. Leave the nRF52840 at its default 3.0 V VDD.",
            &[],
        ),
        st("Rails", "Red from VDD and black from GND to every breakout.", &ids(&wires, |w| w.bus == "power" || w.bus == "ground")),
        st(
            "SPI display",
            &format!("SCK → {sck}, MOSI → {mosi}, CS → {cs}, DC → {dc}, RES → {rst} — the same pins as the overlay, so the custom board's display code runs unchanged."),
            &ids(&wires, |w| w.bus == "spi"),
        ),
        st("PDM microphone", &format!("CLK → {pclk}, DAT → {pdat}, SEL → GND."), &ids(&wires, |w| w.bus == "pdm" || w.to_pin == "GND" && w.from_pin == "SEL")),
        st("I²C IMU", &format!("SDA → {sda}, SCL → {scl}, INT → {int}."), &ids(&wires, |w| w.bus == "i2c" || w.signal == "IMU INT")),
        st("Touch", &format!("TTP223 SIG → {tch}."), &ids(&wires, |w| w.signal == "TOUCH")),
        st(
            "Battery simulator",
            &format!("Potentiometer ends to VDD and GND, wiper → {vb} (AIN0). Turn it to move the simulated state of charge."),
            &ids(&wires, |w| w.from_part == "pot"),
        ),
        st(
            "Flash Zephyr",
            "Firmware tab → \"Zephyr\" → download, then `west build -b nrf52840dk/nrf52840 . -- -DIRIS_PANEL_SSD1306=ON` and `west flash`. Watch the log on the DK's serial port.",
            &[],
        ),
        st(
            "Verify",
            "Check `west build -t ram_report` against the Resources tab, tap through the screens, stream audio to a phone, and turn the potentiometer: the power thread logs a new display/radio decision.",
            &[],
        ),
    ];
    let differences = vec![
        df(
            "Board",
            "nRF52840 DK with on-board debugger",
            "Custom board, SWD test pads",
            "Identical chip and pins: the overlay moves to the custom board unchanged.",
        ),
        df(
            "Display",
            "128×64 SSD1306 over SPI",
            "640×400 micro-OLED over SPI",
            "Same SPI pins and render path; panel init differs (IRIS_PANEL_SSD1306).",
        ),
        df(
            "Microphones",
            "One PDM breakout",
            "Two PDM microphones sharing CLK",
            "Add a second breakout with SEL high to test stereo capture.",
        ),
        df(
            "Battery",
            "Potentiometer on AIN0",
            "LiPo + 1.5 MΩ / 1 MΩ divider",
            "Exercises the policy; the pot draws far more current than the real divider.",
        ),
        df(
            "Power",
            "USB, DK regulator",
            "LiPo + 1.8 V buck",
            "Measure nRF52840 current on the DK's current-measurement header.",
        ),
    ];
    let (power, pnotes) = power_compare(p, Build::Nrf52840dk, battery_mah, scenario);
    Breadboard {
        build: Build::Nrf52840dk,
        title: "Iris v0 — nRF52840 DK build (Zephyr pin map)".into(),
        summary: "The bring-up rig for production firmware: breakouts wired to exactly the pins of the generated devicetree overlay, so the Zephyr skeleton runs unchanged and later moves to the custom board.".into(),
        width: 900.0,
        height: 410.0,
        parts,
        wires,
        bom,
        steps,
        differences,
        pinmap: pinmap(p, Build::Nrf52840dk),
        power,
        notes: pnotes,
    }
}

// ------------------------------------------------------------------ ESP32-S3 build

fn esp32(p: &Project, battery_mah: f64, scenario: &str) -> Breadboard {
    let (bx, bw, top) = (360.0, 110.0, 30.0);
    let left = [
        "3V3.1", "3V3.2", "RST", "4", "5", "6", "7", "15", "16", "17", "18", "8", "3", "46", "9", "10", "11", "12", "13", "14", "5V",
        "GND.1",
    ];
    let right = [
        "GND.2", "TX", "RX", "1", "2", "42", "41", "40", "39", "38", "37", "36", "35", "0", "45", "48", "47", "21", "20", "19", "GND.3",
        "GND.4",
    ];
    let mut ep = column(&left, bx, 50.0, 15.0, "l");
    ep.extend(column(&right, bx + bw, 50.0, 15.0, "r"));
    let m = |id: &str, label: &str, sub: &str, y: f64, h: f64, pins: &[(&str, f64)]| {
        part(
            id,
            label,
            sub,
            "module",
            40.0,
            y,
            160.0,
            h,
            pins.iter().map(|(n, py)| pin(n, 200.0, *py, "r")).collect(),
        )
    };
    let parts = vec![
        part("esp", "ESP32-S3-DevKitC-1", "Wi-Fi + BLE", "board", bx, top, bw, 360.0, ep),
        m(
            "oled",
            "OLED 128×64 (I²C)",
            "SSD1306, 0x3C",
            30.0,
            100.0,
            &[("GND", 50.0), ("VCC", 70.0), ("SCL", 90.0), ("SDA", 110.0)],
        ),
        m(
            "mic",
            "INMP441 microphone",
            "I²S, 24-bit",
            150.0,
            105.0,
            &[
                ("VDD", 160.0),
                ("GND", 175.0),
                ("L/R", 190.0),
                ("WS", 205.0),
                ("SCK", 220.0),
                ("SD", 235.0),
            ],
        ),
        m(
            "imu",
            "MPU-6050 IMU",
            "I²C 0x68",
            275.0,
            100.0,
            &[("VCC", 290.0), ("GND", 310.0), ("SCL", 330.0), ("SDA", 350.0)],
        ),
        part(
            "touch",
            "Touch / button",
            "TTP223 (hardware) · button (Wokwi)",
            "module",
            540.0,
            120.0,
            150.0,
            80.0,
            column(&["VCC", "SIG", "GND"], 540.0, 135.0, 20.0, "l"),
        ),
    ];
    type Row<'a> = (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str, &'a str, f64);
    let spec: [Row; 14] = [
        ("oled", "GND", "GND.1", "black", "ground", "GND", "", 214.0),
        ("oled", "VCC", "3V3.1", "red", "power", "3V3", "", 222.0),
        (
            "oled",
            "SCL",
            "9",
            "yellow",
            "i2c",
            "I²C SCL",
            "Wokwi's SSD1306 part is I²C-only, so this build uses I²C for the display",
            230.0,
        ),
        ("oled", "SDA", "8", "blue", "i2c", "I²C SDA", "", 238.0),
        ("mic", "VDD", "3V3.2", "red", "power", "3V3", "", 246.0),
        ("mic", "GND", "GND.1", "black", "ground", "GND", "", 254.0),
        ("mic", "L/R", "GND.1", "black", "ground", "L/R select", "low = left channel", 262.0),
        ("mic", "WS", "5", "purple", "i2s", "I²S WS", "word select", 270.0),
        ("mic", "SCK", "4", "orange", "i2s", "I²S SCK", "bit clock", 278.0),
        (
            "mic",
            "SD",
            "6",
            "white",
            "i2s",
            "I²S SD",
            "serial data (not simulated in Wokwi)",
            286.0,
        ),
        ("imu", "VCC", "3V3.2", "red", "power", "3V3", "", 294.0),
        ("imu", "GND", "GND.1", "black", "ground", "GND", "", 302.0),
        ("imu", "SCL", "9", "yellow", "i2c", "I²C SCL", "shared bus with the OLED", 310.0),
        ("imu", "SDA", "8", "blue", "i2c", "I²C SDA", "", 318.0),
    ];
    let mut specs: Vec<WireSpec> = spec
        .iter()
        .map(|(a, ap, bp, c, bus, s, n, x)| ws((a, ap), ("esp", bp), c, bus, s, n, Via::X(*x)))
        .collect();
    specs.push(ws(
        ("touch", "SIG"),
        ("esp", "7"),
        "green",
        "gpio",
        "TOUCH",
        "GPIO 7 (INPUT_PULLUP with a button in Wokwi)",
        Via::Y(14.0, 520.0, 340.0),
    ));
    specs.push(ws(("touch", "GND"), ("esp", "GND.2"), "black", "ground", "GND", "", Via::X(500.0)));
    specs.push(ws(
        ("touch", "VCC"),
        ("esp", "3V3.1"),
        "red",
        "power",
        "3V3",
        "TTP223 only (a push button needs no supply)",
        Via::Y(6.0, 510.0, 346.0),
    ));
    let wires = wires_from(&parts, specs, vec![]);
    let bom = vec![
        bl(
            "Espressif ESP32-S3-DevKitC-1 (or a compatible ESP32-S3 board)",
            1,
            "Development board — Espressif distributors and hobby retailers",
            "U1 SoC",
            "Different SoC family from Iris: the protocol and UI carry over, the power numbers do not.",
            "ESP32-S3-DevKitC-1",
        ),
        bl(
            "0.96 in SSD1306 128×64 OLED module, 4-pin I²C",
            1,
            "Display module — hobby electronics retailers",
            "U2 micro-OLED",
            "",
            "SSD1306 128x64 OLED I2C",
        ),
        bl(
            "MPU-6050 6-axis IMU breakout (GY-521 style)",
            1,
            "Sensor module — hobby electronics retailers",
            "U3 IMU",
            "I²C address 0x68 (AD0 low).",
            "MPU-6050 GY-521",
        ),
        bl(
            "INMP441 I²S MEMS microphone breakout",
            1,
            "Sensor module — hobby electronics retailers",
            "MK1 microphone",
            "I²S instead of PDM: the microphone decimates internally.",
            "INMP441 I2S microphone",
        ),
        bl(
            "TTP223 capacitive touch module (or a push button)",
            1,
            "Sensor module",
            "U7 touch controller",
            "",
            "TTP223 touch module",
        ),
        bl(
            "Breadboard + jumper wires",
            1,
            "Prototyping supplies",
            "main board",
            "",
            "solderless breadboard",
        ),
        bl(
            "USB data cable; USB power bank for untethered tests",
            1,
            "Cables / power",
            "J1, BT1",
            "The DevKit has no LiPo charger.",
            "",
        ),
    ];
    let steps = vec![
        st(
            "Try it in Wokwi first",
            "Firmware tab → target \"ESP32-S3 (Wokwi)\" → follow the \"Open in Wokwi\" steps. The same wiring (minus the I²S microphone) is simulated in the browser.",
            &[],
        ),
        st("Power rails", "Red from 3V3 and black from GND to every module.", &ids(&wires, |w| w.bus == "power" || w.bus == "ground")),
        st("I²C bus", "OLED and MPU-6050 share GPIO 8 (SDA) and GPIO 9 (SCL).", &ids(&wires, |w| w.bus == "i2c")),
        st("I²S microphone", "SCK → GPIO 4, WS → GPIO 5, SD → GPIO 6, L/R → GND (left channel).", &ids(&wires, |w| w.bus == "i2s" || w.from_pin == "L/R")),
        st("Touch", "TTP223 SIG → GPIO 7 (set TOUCH_ACTIVE_HIGH 1), or a push button to GND as in Wokwi.", &ids(&wires, |w| w.from_part == "touch")),
        st("Flash", "Set IRIS_ENABLE_BLE 1 and IRIS_SIM_AUDIO 0, then `pio run -t upload`.", &[]),
    ];
    let differences = vec![
        df(
            "System-on-chip",
            "ESP32-S3 (dual-core, Wi-Fi + BLE)",
            "nRF52840-class BLE SoC",
            "Several times the power; good for UI and protocol work, not for battery-life claims.",
        ),
        df(
            "Display bus",
            "I²C (Wokwi's SSD1306 model is I²C-only)",
            "SPI micro-OLED",
            "Slower refresh on the prototype; the UI code is identical.",
        ),
        df(
            "Microphone",
            "I²S microphone (decimation inside the mic)",
            "Two PDM microphones, decimation by the SoC or the IRIS-DSP co-processor",
            "The DSP Lab's decimation filter is only exercised on the nRF paths.",
        ),
        df(
            "IMU",
            "MPU-6050-class breakout",
            "BMI270-class IMU",
            "Noisier, higher power; fine for the complementary filter demo.",
        ),
        df(
            "Power",
            "USB or power bank; no gauge",
            "LiPo + buck + load switch + battery sense",
            "The learned policy runs, but with a constant state of charge.",
        ),
    ];
    let (power, pnotes) = power_compare(p, Build::Esp32s3, battery_mah, scenario);
    Breadboard {
        build: Build::Esp32s3,
        title: "Iris v0 — ESP32-S3 build (Wokwi-simulatable)".into(),
        summary: "For trying everything in the browser first: common modules around an ESP32-S3 DevKit. The same wiring runs in the Wokwi simulator.".into(),
        width: 720.0,
        height: 400.0,
        parts,
        wires,
        bom,
        steps,
        differences,
        pinmap: pinmap(p, Build::Esp32s3),
        power,
        notes: pnotes,
    }
}

pub fn breadboard(p: &Project, build: Build, battery_mah: f64, scenario: &str) -> Breadboard {
    match build {
        Build::Xiao => xiao(p, battery_mah, scenario),
        Build::Nrf52840dk => dk(p, battery_mah, scenario),
        Build::Esp32s3 => esp32(p, battery_mah, scenario),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wires_start_and_end_on_their_pins() {
        let p = Project::iris();
        for b in [Build::Xiao, Build::Nrf52840dk, Build::Esp32s3] {
            let bb = breadboard(&p, b, 0.0, "reference");
            for w in &bb.wires {
                let a = find(&bb.parts, &w.from_part, &w.from_pin);
                let z = find(&bb.parts, &w.to_part, &w.to_pin);
                assert!(
                    a.0 > 0.0 && z.0 > 0.0,
                    "{b:?}: {} {}→{} {}",
                    w.from_part,
                    w.from_pin,
                    w.to_part,
                    w.to_pin
                );
                assert_eq!(w.route.first().unwrap(), &(a.0, a.1));
                assert_eq!(w.route.last().unwrap(), &(z.0, z.1));
                for s in w.route.windows(2) {
                    assert!(
                        (s[0].0 - s[1].0).abs() < 1e-9 || (s[0].1 - s[1].1).abs() < 1e-9,
                        "diagonal in wire {}",
                        w.id
                    );
                }
                assert!(
                    ["power", "ground", "battery", "i2c", "spi", "pdm", "i2s", "gpio"].contains(&w.bus.as_str()),
                    "{}",
                    w.bus
                );
                // everything stays on the canvas
                assert!(w
                    .route
                    .iter()
                    .all(|&(x, y)| x >= 0.0 && x <= bb.width && y >= 0.0 && y <= bb.height));
            }
            for s in &bb.steps {
                assert!(s.wires.iter().all(|&i| i < bb.wires.len()));
            }
            // every wired step highlights something; ids are unique and dense
            assert!(bb.wires.iter().enumerate().all(|(i, w)| w.id == i));
            assert!(!bb.pinmap.is_empty());
        }
    }

    #[test]
    fn dk_wiring_matches_the_zephyr_overlay() {
        let bb = breadboard(&Project::iris(), Build::Nrf52840dk, 0.0, "reference");
        let pin_of = |sig: &str| bb.wires.iter().find(|w| w.signal == sig).map(|w| w.to_pin.clone()).unwrap();
        assert_eq!(pin_of("SPI SCK"), "P1.15");
        assert_eq!(pin_of("I²C SDA"), "P0.26");
        assert_eq!(pin_of("PDM CLK"), zp("PDM_CLK"));
        assert_eq!(pin_of("VBAT sense"), zp("VBAT_SENSE"));
        let buses: std::collections::BTreeSet<&str> = bb.wires.iter().map(|w| w.bus.as_str()).collect();
        for b in ["spi", "i2c", "pdm", "power", "ground"] {
            assert!(buses.contains(b), "{b}");
        }
    }

    #[test]
    fn pinmap_follows_the_interfaces() {
        let p = Project::iris();
        let m = pinmap(&p, Build::Xiao);
        // every electrical interface of Iris is mapped; mechanical/thermal ones are not
        let electrical = p
            .interfaces
            .iter()
            .filter(|i| ["SPI", "PDM", "I2C", "POWER", "BLE"].contains(&i.kind.to_uppercase().as_str()))
            .count();
        assert_eq!(m.len(), electrical);
        let disp = m.iter().find(|r| r.kind == "SPI" && r.bus == "spi").unwrap();
        assert_eq!(disp.signals[0].prototype, "D8 (SCK)");
        assert_eq!(disp.signals[0].custom, "P1.15");
        let e = pinmap(&p, Build::Esp32s3);
        assert!(e.iter().any(|r| r.bus == "i2s"));
    }

    #[test]
    fn search_links_are_encoded_searches() {
        assert_eq!(url_encode("XIAO nRF52840 Sense"), "XIAO%20nRF52840%20Sense");
        assert_eq!(url_encode("a/b&c"), "a%2Fb%26c");
        let bb = breadboard(&Project::iris(), Build::Xiao, 0.0, "reference");
        let l = &bb.bom[0].search;
        assert!(l[0].url.starts_with("https://octopart.com/search?q=XIAO%20nRF52840"));
    }

    #[test]
    fn power_comparison_runs_through_the_power_lab() {
        let p = Project::iris();
        let x = breadboard(&p, Build::Xiao, 0.0, "reference").power;
        let d = breadboard(&p, Build::Nrf52840dk, 0.0, "reference").power;
        let e = breadboard(&p, Build::Esp32s3, 0.0, "reference").power;
        // custom board figure is the Power Lab's own number
        let custom = crate::power::budget(&p, &crate::power::budget::UsageProfile::default()).avg_mw;
        assert!((x.custom_avg_mw - custom).abs() < 1e-9);
        // the ESP32-S3 is the hungriest build; the nRF builds draw less than the custom board
        // only because a 128×64 OLED replaces the 640×400 micro-OLED (reported, not tuned)
        assert!(e.proto_avg_mw > x.proto_avg_mw && e.proto_avg_mw > d.proto_avg_mw);
        let micro = p.components.iter().find(|c| c.refdes == "U2").unwrap();
        assert!(micro.power.active_mw > 40.0);
        // hand check of one row: OLED 40 mW × duty + 0.03 × (1 − duty)
        let r = x.proto_rows.iter().find(|r| r.name.contains("SSD1306")).unwrap();
        assert!((r.avg_mw - (40.0 * r.duty + 0.03 * (1.0 - r.duty))).abs() < 1e-9);
        // the DK's potentiometer: 3.0² / 10 kΩ = 0.9 mW, always on
        let pot = d.proto_rows.iter().find(|r| r.name.contains("potentiometer")).unwrap();
        assert!((pot.avg_mw - 0.9).abs() < 1e-9);
        assert_eq!(x.battery_mah, p.design.battery.capacity_mah);
    }
}
