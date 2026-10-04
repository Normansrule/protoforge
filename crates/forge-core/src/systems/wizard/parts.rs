//! Typical parts the wizard places in a starter bill of materials (BOM).
//!
//! Every number is a **typical, round planning value** for learning — mass,
//! power and price classes, not quotes and not part numbers. Each line names a
//! part *class*; `example_part` stays empty so nobody mistakes a starting
//! point for a sourcing decision.

use crate::project::{Component, PowerProfile};

/// Subsystem ids used by the wizard.
pub const STR: &str = "SYS-STR";
pub const PWR: &str = "SYS-PWR";
pub const CMP: &str = "SYS-CMP";
pub const PCB: &str = "SYS-PCB";
pub const FW: &str = "SYS-FW";
pub const UI: &str = "SYS-UI";
pub const AUD: &str = "SYS-AUD";
pub const SNS: &str = "SYS-SNS";
pub const ACT: &str = "SYS-ACT";
pub const OPT: &str = "SYS-OPT";
pub const QRNG: &str = "SYS-QRNG";
pub const QLINK: &str = "SYS-QLINK";
pub const MFG: &str = "SYS-MFG";

/// One BOM line under construction.
#[derive(Clone, Debug)]
pub struct Part {
    pub refdes: String,
    pub name: String,
    pub class: String,
    pub sub: &'static str,
    pub qty: f64,
    pub cost: f64,
    pub mass: f64,
    /// active mW, sleep mW, duty
    pub power: [f64; 3],
    /// Footprint from the PCB library (empty = none).
    pub footprint: &'static str,
    /// Board area for parts outside the footprint library, mm² (per unit).
    pub area: f64,
    /// Lives on the main board (counts toward board area).
    pub board: bool,
}

impl Part {
    pub fn new(refdes: &str, name: &str, class: &str, sub: &'static str) -> Part {
        Part {
            refdes: refdes.into(),
            name: name.into(),
            class: class.into(),
            sub,
            qty: 1.0,
            cost: 0.0,
            mass: 0.0,
            power: [0.0; 3],
            footprint: "",
            area: 0.0,
            board: false,
        }
    }
    pub fn cost(mut self, usd: f64) -> Part {
        self.cost = usd;
        self
    }
    pub fn mass(mut self, g: f64) -> Part {
        self.mass = g;
        self
    }
    pub fn qty(mut self, q: f64) -> Part {
        self.qty = q;
        self
    }
    pub fn pw(mut self, active_mw: f64, sleep_mw: f64, duty: f64) -> Part {
        self.power = [active_mw, sleep_mw, duty.clamp(0.0, 1.0)];
        self
    }
    /// On the board with a library footprint.
    pub fn fp(mut self, footprint: &'static str) -> Part {
        self.footprint = footprint;
        self.board = true;
        self
    }
    /// On the board with a typical land area (mm² per unit).
    pub fn area(mut self, mm2: f64) -> Part {
        self.area = mm2;
        self.board = true;
        self
    }

    /// Board area of one unit: the courtyard of the library footprint, else the typical area.
    pub fn unit_area_mm2(&self) -> f64 {
        if !self.footprint.is_empty() {
            if let Some(f) = crate::pcb::footprints::footprint(self.footprint) {
                let c = f.courtyard;
                return (c[2] - c[0]).abs() * (c[3] - c[1]).abs();
            }
        }
        self.area
    }

    /// Average power of the line (mW), same definition as the Systems roll-up.
    pub fn avg_mw(&self) -> f64 {
        let [a, s, d] = self.power;
        self.qty * (a * d + s * (1.0 - d))
    }

    pub fn component(&self) -> Component {
        Component {
            refdes: self.refdes.clone(),
            name: self.name.clone(),
            part_class: self.class.clone(),
            example_part: String::new(),
            subsystem: self.sub.into(),
            qty: self.qty,
            unit_cost_usd: round_to(self.cost, 3),
            mass_g: round_to(self.mass, 3),
            power: PowerProfile {
                active_mw: round_to(self.power[0], 4),
                sleep_mw: round_to(self.power[1], 5),
                duty: round_to(self.power[2], 4),
            },
            footprint: self.footprint.into(),
            // Board parts start off the PCB, like the built-in templates: PCB
            // Studio's placement is tuned for the Iris outline and routing a
            // new outline takes seconds, which would stall live verification.
            on_pcb: false,
            location: if self.board { "board".into() } else { "enclosure".into() },
        }
    }
}

/// Round to `digits` significant decimals (keeps project.json tidy).
pub fn round_to(x: f64, digits: i32) -> f64 {
    let k = 10f64.powi(digits);
    (x * k).round() / k
}

/// Round up to two significant figures (for targets derived from estimates).
pub fn nice_up(x: f64) -> f64 {
    if !(x.is_finite() && x > 0.0) {
        return x;
    }
    let mag = 10f64.powi(x.log10().floor() as i32 - 1);
    (x / mag - 1e-9).ceil() * mag
}

/// Round down to two significant figures.
pub fn nice_down(x: f64) -> f64 {
    if !(x.is_finite() && x > 0.0) {
        return x;
    }
    let mag = 10f64.powi(x.log10().floor() as i32 - 1);
    (x / mag + 1e-9).floor() * mag
}

/// Typical fraction of time a part is active in each kind's reference use.
pub fn duty(kind: &str, role: &str) -> f64 {
    let t = |w: f64, h: f64, i: f64, d: f64, r: f64, s: f64, o: f64, q: f64, b: f64| match kind {
        "wearable" => w,
        "handheld" => h,
        "iot_sensor" => i,
        "desk_instrument" => d,
        "robot" => r,
        "smart_home" => s,
        "optical" => o,
        "quantum" => q,
        "dev_board" => b,
        _ => 0.1,
    };
    match role {
        //            wear  hand  iot    desk  robot home  opt   qnt   dev
        "soc" => t(0.05, 0.2, 0.002, 0.3, 0.4, 0.05, 0.3, 1.0, 0.2),
        "wifi" => t(0.05, 0.1, 0.002, 0.08, 0.15, 0.04, 0.1, 1.0, 0.1),
        "display" => t(0.3, 0.6, 0.002, 1.0, 0.5, 0.2, 0.6, 1.0, 0.3),
        "mics" => t(0.3, 0.3, 0.1, 0.5, 0.5, 1.0, 0.3, 0.0, 0.3),
        "speaker" => t(0.03, 0.05, 0.01, 0.05, 0.05, 0.05, 0.02, 0.0, 0.05),
        "camera" => t(0.05, 0.2, 0.001, 0.1, 0.2, 0.05, 0.5, 0.0, 0.1),
        "motors" => t(0.02, 0.02, 0.001, 0.02, 0.35, 0.02, 0.02, 0.0, 0.02),
        "leds" => t(0.05, 0.1, 0.001, 0.2, 0.2, 0.1, 0.1, 0.5, 0.2),
        "illum" => t(0.3, 0.5, 0.01, 0.5, 0.3, 0.3, 0.5, 0.0, 0.3),
        "env" => t(0.01, 0.01, 0.001, 0.01, 0.01, 0.01, 0.01, 0.0, 0.01),
        _ => 0.1,
    }
}

/// A LiPo pouch size from a list of common capacities (mAh).
pub const LIPO_SIZES: &[f64] = &[
    40.0, 60.0, 80.0, 100.0, 120.0, 150.0, 180.0, 220.0, 250.0, 300.0, 400.0, 500.0, 600.0, 800.0, 1000.0, 1200.0, 1500.0, 2000.0, 2500.0,
    3000.0, 3500.0, 4000.0, 5000.0, 6000.0, 8000.0, 10000.0,
];

/// Typical gravimetric energy density of small LiPo pouch cells, mWh/g
/// (smaller cells carry relatively more packaging and protection board).
pub fn lipo_mwh_per_g(mah: f64) -> f64 {
    120.0 + 60.0 * mah / (mah + 300.0)
}

/// Typical quantity-1 price of a LiPo pouch with protection board, USD.
pub fn lipo_cost(mah: f64) -> f64 {
    1.0 + 0.0035 * mah
}

/// A primary coin cell class.
pub struct Coin {
    pub name: &'static str,
    pub mah: f64,
    pub mass_g: f64,
    pub cost: f64,
    /// Holder footprint, mm².
    pub holder_mm2: f64,
}

pub const COINS: &[Coin] = &[
    Coin {
        name: "CR2032-class",
        mah: 225.0,
        mass_g: 3.0,
        cost: 0.3,
        holder_mm2: 440.0,
    },
    Coin {
        name: "CR2450-class",
        mah: 620.0,
        mass_g: 6.8,
        cost: 0.5,
        holder_mm2: 700.0,
    },
];

/// Display module class for a product kind: (name, class, cost, mass, active mW, sleep mW).
pub fn display_for(kind: &str) -> (&'static str, &'static str, f64, f64, f64, f64) {
    match kind {
        "wearable" => (
            "Display: 1.2 in round LCD module (240 × 240)",
            "SPI TFT LCD module with LED backlight",
            5.5,
            4.0,
            60.0,
            0.05,
        ),
        "iot_sensor" => (
            "Display: 2.13 in e-paper module",
            "SPI e-paper panel (bistable)",
            6.0,
            5.0,
            30.0,
            0.0,
        ),
        "dev_board" => ("Display: 0.96 in OLED module (128 × 64)", "I2C OLED module", 3.0, 2.0, 20.0, 0.01),
        "desk_instrument" => (
            "Display: 2.8 in TFT LCD module (320 × 240)",
            "SPI TFT LCD module with LED backlight",
            9.0,
            18.0,
            180.0,
            0.1,
        ),
        _ => (
            "Display: 2.0 in TFT LCD module (320 × 240)",
            "SPI TFT LCD module with LED backlight",
            7.0,
            9.0,
            110.0,
            0.1,
        ),
    }
}
