//! Schematic connectivity (pin → net) for the Iris main board.
//!
//! The Iris project file lists *components*; the schematic lives here as a
//! table so the PCB lab can place, route and verify a real board. Pins of
//! components that are missing from the project (or no longer `on_pcb`) are
//! dropped, so editing the BOM never breaks the lab.
//!
//! The SoC pinout is **illustrative**: power, RF and debug pins are fixed,
//! while every general-purpose signal is assigned to a free GPIO *after*
//! placement ("pin swapping" — see [`crate::pcb::place::assign_gpio`]), which
//! is what designers do with pin-muxed microcontrollers such as BLE SoCs.
//! Other pinouts follow the common conventions of their package families
//! (e.g. the 25-series SPI flash pinout) — always check your actual parts.

use serde::{Deserialize, Serialize};

use crate::project::Project;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetClass {
    Ground,
    Power,
    Signal,
    Rf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetDef {
    pub name: String,
    pub class: NetClass,
    /// (refdes, pin) pairs.
    pub pins: Vec<(String, String)>,
    /// Expected DC current (A), used for the IPC-2221 width check.
    pub current_a: f64,
    pub description: String,
}

/// Static pin table: (refdes, pin, net).
pub const PINS: &[(&str, &str, &str)] = &[
    // USB-C (power only)
    ("J1", "A4B9", "VBUS"),
    ("J1", "B4A9", "VBUS"),
    ("J1", "A1B12", "GND"),
    ("J1", "B1A12", "GND"),
    ("J1", "S1", "GND"),
    ("J1", "S2", "GND"),
    ("J1", "S3", "GND"),
    ("J1", "S4", "GND"),
    ("J1", "A5", "CC1"),
    ("J1", "B5", "CC2"),
    // optional CC pull-downs (5.1 kΩ Rd) — used only if the BOM has them
    ("R6", "1", "CC1"),
    ("R6", "2", "GND"),
    ("R7", "1", "CC2"),
    ("R7", "2", "GND"),
    // Li-ion charger (SOT-23-5): STAT, VSS, VBAT, VDD, PROG
    ("U4", "1", "CHG_STAT"),
    ("U4", "2", "GND"),
    ("U4", "3", "VBAT"),
    ("U4", "4", "VBUS"),
    ("U4", "5", "PROG"),
    ("C5", "1", "VBUS"),
    ("C5", "2", "GND"),
    ("R5", "1", "PROG"),
    ("R5", "2", "GND"),
    // battery connector
    ("J3", "1", "VBAT"),
    ("J3", "2", "GND"),
    // 1.8 V buck (DFN-6): SW, GND, VIN, EN, NC, VOUT/FB, EP
    ("U5", "1", "SW"),
    ("U5", "2", "GND"),
    ("U5", "3", "VBAT"),
    ("U5", "4", "VBAT"),
    ("U5", "6", "1V8"),
    ("U5", "EP", "GND"),
    ("L1", "1", "SW"),
    ("L1", "2", "1V8"),
    ("C1", "1", "VBAT"),
    ("C1", "2", "GND"),
    ("C2", "1", "1V8"),
    ("C2", "2", "GND"),
    // display load switch (SOT-23-5): VIN, GND, ON, NC, VOUT
    ("U8", "1", "VBAT"),
    ("U8", "2", "GND"),
    ("U8", "3", "DISP_EN"),
    ("U8", "5", "3V3_DISP"),
    // battery-sense divider
    ("R1", "1", "VBAT"),
    ("R1", "2", "VBAT_SENSE"),
    ("R2", "1", "VBAT_SENSE"),
    ("R2", "2", "GND"),
    ("C6", "1", "VBAT_SENSE"),
    ("C6", "2", "GND"),
    // SoC fixed pins (GPIOs are assigned after placement)
    ("U1", "27", "1V8"),
    ("U1", "34", "1V8"),
    ("U1", "EP", "GND"),
    ("U1", "2", "RF"),
    ("U1", "9", "SWDCLK"),
    ("U1", "10", "SWDIO"),
    ("C3", "1", "1V8"),
    ("C3", "2", "GND"),
    ("AE1", "1", "RF"),
    // optional RF matching network (series L2, shunt L3 at the antenna) — when
    // L2 is fitted the antenna moves to its own net RF_ANT
    ("L2", "1", "RF"),
    ("L2", "2", "RF_ANT"),
    ("L3", "1", "RF_ANT"),
    ("L3", "2", "GND"),
    // optional 32 MHz crystal + load capacitors on the SoC's XC1/XC2 pins
    ("U1", "11", "XC1"),
    ("U1", "12", "XC2"),
    ("Y1", "1", "XC1"),
    ("Y1", "2", "GND"),
    ("Y1", "3", "XC2"),
    ("Y1", "4", "GND"),
    ("C7", "1", "XC1"),
    ("C7", "2", "GND"),
    ("C8", "1", "XC2"),
    ("C8", "2", "GND"),
    // IMU (LGA-14, I²C mode)
    ("U3", "1", "GND"),
    ("U3", "4", "IMU_INT"),
    ("U3", "5", "1V8"),
    ("U3", "6", "GND"),
    ("U3", "7", "GND"),
    ("U3", "8", "1V8"),
    ("U3", "12", "1V8"),
    ("U3", "13", "I2C_SCL"),
    ("U3", "14", "I2C_SDA"),
    ("C4", "1", "1V8"),
    ("C4", "2", "GND"),
    ("R3", "1", "1V8"),
    ("R3", "2", "I2C_SCL"),
    ("R4", "1", "1V8"),
    ("R4", "2", "I2C_SDA"),
    // capacitive touch controller (SOT-23-6, I²C): SCL, GND, SDA, INT, VDD, CS1
    ("U7", "1", "I2C_SCL"),
    ("U7", "2", "GND"),
    ("U7", "3", "I2C_SDA"),
    ("U7", "4", "TOUCH_INT"),
    ("U7", "5", "1V8"),
    ("U7", "6", "TOUCH"),
    ("E1", "1", "TOUCH"),
    // PDM microphones: VDD, SELECT, CLK, DATA, GND/port — MK1 = left (SEL→GND), MK2 = right (SEL→VDD)
    ("MK1", "1", "1V8"),
    ("MK1", "2", "GND"),
    ("MK1", "3", "PDM_CLK"),
    ("MK1", "4", "PDM_DAT"),
    ("MK1", "5", "GND"),
    ("MK2", "1", "1V8"),
    ("MK2", "2", "1V8"),
    ("MK2", "3", "PDM_CLK"),
    ("MK2", "4", "PDM_DAT"),
    ("MK2", "5", "GND"),
    // SPI NOR flash (25-series pinout): /CS, DO, /WP, GND, DI, CLK, /HOLD, VCC
    ("U6", "1", "FL_CS"),
    ("U6", "2", "FL_IO1"),
    ("U6", "3", "1V8"),
    ("U6", "4", "GND"),
    ("U6", "5", "FL_IO0"),
    ("U6", "6", "FL_CLK"),
    ("U6", "7", "1V8"),
    ("U6", "8", "1V8"),
    // display FPC
    ("J2", "1", "GND"),
    ("J2", "14", "GND"),
    ("J2", "15", "3V3_DISP"),
    ("J2", "16", "3V3_DISP"),
    ("J2", "17", "1V8"),
    ("J2", "18", "GND"),
    ("J2", "19", "DISP_RST"),
    ("J2", "20", "DISP_DC"),
    ("J2", "21", "DISP_CS"),
    ("J2", "22", "DISP_SCK"),
    ("J2", "23", "DISP_MOSI"),
    ("J2", "24", "GND"),
    // SWD programming test pads (bottom side)
    ("TP1", "1", "SWDIO"),
    ("TP2", "1", "SWDCLK"),
];

/// Signals that need one SoC GPIO each (assigned after placement).
pub const GPIO_NETS: &[&str] = &[
    "I2C_SCL",
    "I2C_SDA",
    "IMU_INT",
    "TOUCH_INT",
    "PDM_CLK",
    "PDM_DAT",
    "DISP_SCK",
    "DISP_MOSI",
    "DISP_CS",
    "DISP_DC",
    "DISP_RST",
    "DISP_EN",
    "FL_CS",
    "FL_CLK",
    "FL_IO0",
    "FL_IO1",
    "VBAT_SENSE",
    "CHG_STAT",
];

/// SoC pins that are not available as GPIO (power, RF, crystal, debug).
pub const SOC_RESERVED: &[&str] = &["1", "2", "3", "9", "10", "11", "12", "27", "34", "EP"];

/// Decoupling relationships: (capacitor, IC, IC pin) — the placer puts the
/// capacitor right next to that pin.
pub const DECOUPLING: &[(&str, &str, &str)] = &[
    ("C1", "U5", "3"),
    ("C2", "U5", "6"),
    ("C3", "U1", "27"),
    ("C4", "U3", "8"),
    ("C5", "U4", "4"),
    ("C6", "R2", "1"),
    ("Y1", "U1", "11"),
    ("C7", "U1", "11"),
    ("C8", "U1", "12"),
    ("L2", "AE1", "1"),
    ("L3", "AE1", "1"),
];

/// Board-level copper features that are not BOM components.
pub const BOARD_FEATURES: &[&str] = &["E1", "TP1", "TP2"];

pub fn net_class(name: &str) -> NetClass {
    match name {
        "GND" => NetClass::Ground,
        "VBUS" | "VBAT" | "1V8" | "3V3_DISP" | "SW" => NetClass::Power,
        "RF" | "RF_ANT" => NetClass::Rf,
        _ => NetClass::Signal,
    }
}

fn net_current(name: &str) -> f64 {
    match name {
        "VBUS" => 0.12,
        "VBAT" => 0.15,
        "SW" => 0.2,
        "1V8" => 0.06,
        "3V3_DISP" => 0.08,
        _ => 0.005,
    }
}

fn net_description(name: &str) -> &'static str {
    match name {
        "VBUS" => "USB 5 V input to the charger",
        "VBAT" => "Battery node: charger output, buck and load-switch input, sense divider",
        "SW" => "Buck switch node — keep short (radiates)",
        "1V8" => "Main 1.8 V logic rail from the buck",
        "3V3_DISP" => "Switched display supply through the load switch",
        "GND" => "Ground — bottom-layer pour stitched with vias",
        "RF" => "2.4 GHz feed from the SoC antenna pin (to the matching network, if fitted)",
        "RF_ANT" => "2.4 GHz feed from the matching network to the chip antenna",
        "XC1" | "XC2" => "32 MHz crystal — keep short, ground underneath, away from the switch node",
        "I2C_SCL" | "I2C_SDA" => "I²C bus: IMU + touch controller, 4.7 kΩ pull-ups",
        "PDM_CLK" | "PDM_DAT" => "Shared PDM bus for the two microphones (L/R by SELECT strap)",
        "VBAT_SENSE" => "Divided battery voltage into the SoC ADC",
        "CHG_STAT" => "Charger status (open drain, SoC pull-up)",
        "PROG" => "Charge-current programming resistor",
        "TOUCH" => "Self-capacitance electrode",
        "SWDIO" | "SWDCLK" => "Serial-wire debug to bottom-side test pads",
        "CC1" | "CC2" => "USB-C configuration channel",
        n if n.starts_with("DISP") => "Display SPI / control over the hinge flex",
        n if n.starts_with("FL_") => "SPI NOR flash",
        _ => "",
    }
}

/// Which board features exist (the electrode and SWD pads are always added
/// when the SoC / touch controller they serve is on the board).
pub fn features_for(p: &Project) -> Vec<&'static str> {
    let has = |r: &str| p.components.iter().any(|c| c.on_pcb && c.refdes == r);
    let mut v = Vec::new();
    if has("U7") {
        v.push("E1");
    }
    if has("U1") {
        v.push("TP1");
        v.push("TP2");
    }
    v
}

/// Build the netlist for the components present on the board. `gpio` maps
/// GPIO net name → SoC pin (from [`crate::pcb::place::assign_gpio`]).
pub fn build_nets(p: &Project, present: &dyn Fn(&str, &str) -> bool, gpio: &[(String, String)]) -> Vec<NetDef> {
    let soc = "U1";
    let mut table: Vec<(String, String, String)> = PINS
        .iter()
        .map(|(r, pin, n)| (r.to_string(), pin.to_string(), n.to_string()))
        .collect();
    for (net, pin) in gpio {
        table.push((soc.to_string(), pin.clone(), net.clone()));
    }
    // GPIO nets whose far end lives on the SoC must include the SoC pin.
    let mut nets: Vec<NetDef> = Vec::new();
    // With the series matching inductor fitted, the antenna sits on its own net.
    let matched = present("L2", "1");
    for (r, pin, mut n) in table {
        if !present(&r, &pin) {
            continue;
        }
        if matched && r == "AE1" && n == "RF" {
            n = "RF_ANT".to_string();
        }
        if let Some(nd) = nets.iter_mut().find(|x| x.name == n) {
            nd.pins.push((r, pin));
        } else {
            nets.push(NetDef {
                class: net_class(&n),
                current_a: net_current(&n),
                description: net_description(&n).to_string(),
                name: n,
                pins: vec![(r, pin)],
            });
        }
    }
    let _ = p;
    // a net needs two pins (e.g. the SoC's crystal pins without a crystal fitted)
    nets.retain(|n| n.pins.len() >= 2);
    nets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_gpio_net_has_a_peripheral_pin() {
        for g in GPIO_NETS {
            assert!(PINS.iter().any(|(_, _, n)| n == g), "{g}");
        }
    }

    #[test]
    fn decoupling_caps_share_a_power_net_with_their_pin() {
        for (cap, ic, pin) in DECOUPLING {
            let cap_net = PINS.iter().find(|(r, p, _)| r == cap && *p == "1").map(|x| x.2).unwrap();
            let ic_net = PINS.iter().find(|(r, p, _)| r == ic && p == pin).map(|x| x.2).unwrap();
            // the antenna pad moves to RF_ANT when the matching network is fitted
            let same = cap_net == ic_net || (cap_net == "RF_ANT" && ic_net == "RF");
            assert!(same, "{cap}: {cap_net} vs {ic_net}");
        }
    }
}
