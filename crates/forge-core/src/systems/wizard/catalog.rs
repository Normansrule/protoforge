//! The wizard's vocabulary: product kinds (archetypes), capabilities, target
//! fields, and the table of which metrics the engine really computes from a
//! generic project's own data (found by perturbation experiments — see
//! `docs/new-product.md` and the `metric_table_*` tests).

use serde::Serialize;
use serde_json::{json, Value};

/// How the product touches people — sets the surface-temperature limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Contact {
    /// Worn against the skin for hours.
    Skin,
    /// Held in the hand while in use.
    Hand,
    /// Touched briefly (desk, wall, robot).
    Touch,
}

impl Contact {
    /// Typical design limit for the hottest touchable surface, °C. These
    /// are conservative engineering choices, not quotes from a standard:
    /// check the touch-temperature tables of IEC 62368-1 for your contact
    /// time and surface material.
    pub fn surface_limit_c(self) -> f64 {
        match self {
            Contact::Skin => 41.0,
            Contact::Hand => 45.0,
            Contact::Touch => 50.0,
        }
    }
    pub fn words(self) -> &'static str {
        match self {
            Contact::Skin => "worn against the skin for hours",
            Contact::Hand => "held in the hand while in use",
            Contact::Touch => "touched briefly",
        }
    }
}

pub struct KindDef {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub blurb: &'static str,
    pub help: &'static str,
    pub examples: &'static [&'static str],
    pub default_caps: &'static [&'static str],
    /// Target defaults that differ from the generic ones.
    pub defaults: &'static [(&'static str, f64)],
    pub contact: Contact,
    /// Out-of-scope note shown with the kind (empty = none).
    pub scope: &'static str,
}

pub const KINDS: &[KindDef] = &[
    KindDef {
        id: "wearable",
        name: "Wearable",
        icon: "ergonomics",
        blurb: "Worn on the wrist, head, chest or clothing all day.",
        help: "Small, light and battery-powered, against the skin for hours: mass, battery life and surface temperature drive the design.",
        examples: &["fitness band", "smart ring charger", "posture clip", "hearing-aid accessory"],
        default_caps: &["battery", "usb_c", "ble", "imu", "buttons"],
        defaults: &[
            ("mass_g", 45.0),
            ("life_h", 16.0),
            ("cost_usd", 25.0),
            ("quantity", 1000.0),
            ("retail_usd", 129.0),
            ("length_mm", 45.0),
            ("width_mm", 38.0),
            ("height_mm", 14.0),
            ("range_m", 5.0),
            ("hours_per_day", 12.0),
            ("ambient_c", 35.0),
        ],
        contact: Contact::Skin,
        scope: "",
    },
    KindDef {
        id: "handheld",
        name: "Handheld gadget",
        icon: "proto",
        blurb: "Held in one hand: a remote, a meter, a game, a scanner.",
        help: "A pocketable, battery-powered device with a screen or buttons. Grip size, run time and drop survival matter most.",
        examples: &["handheld game", "label scanner", "multimeter", "smart remote"],
        default_caps: &["battery", "usb_c", "ble", "display", "buttons", "leds"],
        defaults: &[
            ("mass_g", 200.0),
            ("life_h", 8.0),
            ("cost_usd", 40.0),
            ("quantity", 1000.0),
            ("retail_usd", 149.0),
            ("length_mm", 120.0),
            ("width_mm", 65.0),
            ("height_mm", 25.0),
            ("hours_per_day", 2.0),
            ("ambient_c", 35.0),
        ],
        contact: Contact::Hand,
        scope: "",
    },
    KindDef {
        id: "iot_sensor",
        name: "IoT sensor node",
        icon: "rf",
        blurb: "A small node that measures something and reports it wirelessly for months.",
        help: "Runs unattended on a primary cell for a year or more: microwatt average power, a radio that sleeps almost all the time.",
        examples: &["room climate sensor", "asset tag", "plant-soil monitor", "door sensor"],
        default_caps: &["coin_cell", "ble", "env_sensors"],
        defaults: &[
            ("mass_g", 30.0),
            ("life_h", 8760.0),
            ("cost_usd", 10.0),
            ("quantity", 5000.0),
            ("retail_usd", 29.0),
            ("length_mm", 40.0),
            ("width_mm", 40.0),
            ("height_mm", 15.0),
            ("range_m", 15.0),
            ("hours_per_day", 24.0),
            ("ambient_c", 40.0),
            ("design_years", 3.0),
        ],
        contact: Contact::Touch,
        scope: "",
    },
    KindDef {
        id: "desk_instrument",
        name: "Desk instrument",
        icon: "dsp",
        blurb: "Sits on a bench or desk, powered from USB or an adapter.",
        help: "A powered instrument with a screen: an air-quality monitor, a lab meter, a clock. Run time is not an issue; heat and the supply budget are.",
        examples: &["air-quality monitor", "bench meter", "desk clock", "macro pad"],
        default_caps: &["usb_c", "wifi", "display", "buttons", "env_sensors"],
        defaults: &[
            ("mass_g", 500.0),
            ("cost_usd", 60.0),
            ("quantity", 500.0),
            ("retail_usd", 199.0),
            ("length_mm", 150.0),
            ("width_mm", 100.0),
            ("height_mm", 60.0),
            ("hours_per_day", 10.0),
            ("ambient_c", 30.0),
            ("design_years", 5.0),
        ],
        contact: Contact::Touch,
        scope: "",
    },
    KindDef {
        id: "robot",
        name: "Robot / mechatronic device",
        icon: "robotics",
        blurb: "Moves: wheels, arms, pan-tilt heads, small vehicles.",
        help: "Motors dominate the power budget, and mass and power chase each other (heavier robot → more motor power → bigger battery → heavier robot). The wizard iterates that loop for you.",
        examples: &["desk robot", "line follower", "camera pan-tilt", "robot arm kit"],
        default_caps: &["battery", "usb_c", "ble", "imu", "motors", "buttons"],
        defaults: &[
            ("mass_g", 400.0),
            ("life_h", 2.0),
            ("cost_usd", 60.0),
            ("quantity", 500.0),
            ("retail_usd", 149.0),
            ("length_mm", 150.0),
            ("width_mm", 110.0),
            ("height_mm", 120.0),
            ("hours_per_day", 2.0),
            ("ambient_c", 30.0),
        ],
        contact: Contact::Touch,
        scope: "",
    },
    KindDef {
        id: "smart_home",
        name: "Smart-home device (low-voltage)",
        icon: "home",
        blurb: "Lives on a wall or a shelf, always connected, powered by USB or a low-voltage adapter.",
        help: "Always-on Wi-Fi, sensors and lights. Powered from USB-C or a certified low-voltage adapter only.",
        examples: &["room sensor hub", "smart button", "night light", "doorbell chime"],
        default_caps: &["usb_c", "wifi", "env_sensors", "leds", "buttons"],
        defaults: &[
            ("mass_g", 250.0),
            ("cost_usd", 25.0),
            ("quantity", 5000.0),
            ("retail_usd", 79.0),
            ("length_mm", 90.0),
            ("width_mm", 90.0),
            ("height_mm", 30.0),
            ("hours_per_day", 24.0),
            ("ambient_c", 35.0),
            ("design_years", 5.0),
        ],
        contact: Contact::Touch,
        scope: "Mains (line-voltage) design is out of scope: the wizard only designs products powered from USB or an external, certified low-voltage adapter. Anything wired to the mains needs a qualified designer and a safety certification body.",
    },
    KindDef {
        id: "optical",
        name: "Optical device",
        icon: "optics",
        blurb: "Makes, shapes or measures light: illuminators, spectrometers, microscopes.",
        help: "A light source, a lens train and a detector around a small controller. The Optics Lab's ray model is the Iris head-up display, so optical requirements start as tests.",
        examples: &["pocket spectrometer", "USB microscope", "inspection light", "colour meter"],
        default_caps: &["battery", "usb_c", "display", "buttons"],
        defaults: &[
            ("mass_g", 300.0),
            ("life_h", 4.0),
            ("cost_usd", 80.0),
            ("quantity", 200.0),
            ("retail_usd", 249.0),
            ("length_mm", 140.0),
            ("width_mm", 60.0),
            ("height_mm", 50.0),
            ("hours_per_day", 2.0),
            ("ambient_c", 30.0),
        ],
        contact: Contact::Hand,
        scope: "Laser products need a laser-safety classification (IEC 60825-1); the wizard assumes an LED source.",
    },
    KindDef {
        id: "quantum",
        name: "Quantum / photonics testbed",
        icon: "quantum",
        blurb: "A benchtop quantum key distribution (QKD) link with a quantum random number generator (QRNG).",
        help: "Two nodes joined by optical fiber, verified by the Quantum Lab's decoy-state key-rate model and QRNG simulation. Bench-powered from certified lab supplies.",
        examples: &["fiber QKD teaching link", "QRNG characterisation bench", "single-photon detection bench"],
        default_caps: &["dc_adapter"],
        defaults: &[
            ("mass_g", 25000.0),
            ("cost_usd", 40000.0),
            ("quantity", 2.0),
            ("retail_usd", 60000.0),
            ("length_mm", 600.0),
            ("width_mm", 450.0),
            ("height_mm", 200.0),
            ("hours_per_day", 8.0),
            ("ambient_c", 25.0),
            ("design_years", 5.0),
            ("fiber_km", 10.0),
        ],
        contact: Contact::Touch,
        scope: "Lab mains wiring and laser safety (IEC 60825-1) are out of scope; the nodes run from certified bench supplies and the laser stays inside an interlocked enclosure.",
    },
    KindDef {
        id: "dev_board",
        name: "Bare PCB / dev board",
        icon: "pcb",
        blurb: "A board without a case: a breakout, a dev kit, a teaching board.",
        help: "The product is the printed circuit board (PCB) itself. Board size, cost and the USB power budget matter; there is no enclosure unless you add one.",
        examples: &["microcontroller dev board", "sensor breakout", "teaching kit board"],
        default_caps: &["usb_c", "leds", "buttons"],
        defaults: &[
            ("mass_g", 15.0),
            ("cost_usd", 8.0),
            ("quantity", 1000.0),
            ("retail_usd", 19.0),
            ("length_mm", 70.0),
            ("width_mm", 30.0),
            ("height_mm", 10.0),
            ("hours_per_day", 8.0),
            ("ambient_c", 30.0),
            ("design_years", 5.0),
        ],
        contact: Contact::Touch,
        scope: "",
    },
    KindDef {
        id: "mechanical",
        name: "Mechanical part / fixture",
        icon: "structures",
        blurb: "A bracket, a jig, a mount: no electronics unless you add some.",
        help: "A load-bearing printed or molded part. The wizard sizes a cantilever bracket for your design load with visible beam maths.",
        examples: &["shelf bracket", "camera mount", "drill jig", "cable clip"],
        default_caps: &["printed"],
        defaults: &[
            ("mass_g", 120.0),
            ("cost_usd", 8.0),
            ("quantity", 100.0),
            ("retail_usd", 25.0),
            ("length_mm", 80.0),
            ("width_mm", 40.0),
            ("height_mm", 30.0),
            ("hours_per_day", 24.0),
            ("ambient_c", 30.0),
            ("design_years", 5.0),
            ("load_n", 50.0),
        ],
        contact: Contact::Touch,
        scope: "",
    },
];

pub struct CapDef {
    pub id: &'static str,
    pub name: &'static str,
    pub group: &'static str,
    pub help: &'static str,
    pub excludes: &'static [&'static str],
    /// true when the capability needs electronics (a controller and power).
    pub electronic: bool,
}

pub const CAPS: &[CapDef] = &[
    CapDef {
        id: "battery",
        name: "Rechargeable battery",
        group: "Power",
        help: "A lithium-polymer (LiPo) cell sized from your power budget and target life, with a charger and a buck converter.",
        excludes: &["coin_cell"],
        electronic: true,
    },
    CapDef {
        id: "coin_cell",
        name: "Coin cell",
        group: "Power",
        help: "A primary lithium coin cell (CR2032 or CR2450 class) for months-to-years of microwatt operation. Not for radios that stream, screens or motors.",
        excludes: &["battery"],
        electronic: true,
    },
    CapDef {
        id: "usb_c",
        name: "USB-C power / charging",
        group: "Power",
        help: "A USB Type-C receptacle at the default 5 V, 500 mA budget (no USB Power Delivery negotiation).",
        excludes: &[],
        electronic: true,
    },
    CapDef {
        id: "dc_adapter",
        name: "Low-voltage DC adapter",
        group: "Power",
        help: "A barrel jack fed by an external, certified 5–24 V adapter. Mains design stays out of scope.",
        excludes: &[],
        electronic: true,
    },
    CapDef {
        id: "ble",
        name: "Bluetooth Low Energy",
        group: "Connectivity",
        help: "A BLE system-on-chip (SoC) with a chip antenna; its link budget to a phone is computed in the RF & Antenna Lab.",
        excludes: &[],
        electronic: true,
    },
    CapDef {
        id: "wifi",
        name: "Wi-Fi",
        group: "Connectivity",
        help: "A 2.4 GHz Wi-Fi + BLE SoC. Wi-Fi peaks at hundreds of milliwatts, so it needs USB, an adapter or a rechargeable battery.",
        excludes: &[],
        electronic: true,
    },
    CapDef {
        id: "display",
        name: "Display",
        group: "Interaction",
        help: "A small screen sized for the product kind: a round liquid-crystal display (LCD) on wearables, e-paper on sensors, a thin-film-transistor (TFT) LCD on the rest.",
        excludes: &[],
        electronic: true,
    },
    CapDef {
        id: "buttons",
        name: "Buttons / touch",
        group: "Interaction",
        help: "Tactile buttons (or a capacitive touch pad on wearables).",
        excludes: &[],
        electronic: true,
    },
    CapDef {
        id: "leds",
        name: "Status LEDs / light",
        group: "Interaction",
        help: "An RGB status LED with its current-limiting resistors.",
        excludes: &[],
        electronic: true,
    },
    CapDef {
        id: "microphones",
        name: "Microphones (voice)",
        group: "Audio",
        help: "Two digital MEMS microphones as a beamforming pair; the DSP Lab computes their directivity and decimation filter.",
        excludes: &[],
        electronic: true,
    },
    CapDef {
        id: "speaker",
        name: "Speaker",
        group: "Audio",
        help: "A micro speaker with a class-D amplifier for prompts and chimes.",
        excludes: &[],
        electronic: true,
    },
    CapDef {
        id: "imu",
        name: "Motion sensing",
        group: "Sensing",
        help: "An inertial measurement unit (IMU): accelerometer + gyroscope, or a wake-on-motion accelerometer on coin-cell products.",
        excludes: &[],
        electronic: true,
    },
    CapDef {
        id: "env_sensors",
        name: "Environmental sensors",
        group: "Sensing",
        help: "Temperature, humidity and pressure in one small sensor.",
        excludes: &[],
        electronic: true,
    },
    CapDef {
        id: "camera",
        name: "Camera (2 MP class)",
        group: "Sensing",
        help: "A low-resolution camera module for snapshots or barcodes. Needs a Wi-Fi-class SoC with a camera interface and extra memory; not for coin cells.",
        excludes: &[],
        electronic: true,
    },
    CapDef {
        id: "motors",
        name: "Motors / actuators",
        group: "Motion",
        help: "Two drive gear motors on robots (one actuator otherwise) with a motor driver. Motor power is sized from the mass.",
        excludes: &[],
        electronic: true,
    },
    CapDef {
        id: "printed",
        name: "3-D printed enclosure",
        group: "Enclosure",
        help: "PETG shells printed on a hobby or farm printer: no tooling, best below a few thousand units.",
        excludes: &["molded"],
        electronic: false,
    },
    CapDef {
        id: "molded",
        name: "Injection-molded enclosure",
        group: "Enclosure",
        help: "PC/ABS shells from a steel or aluminium mold: cheap per part, but the tool costs thousands and takes weeks.",
        excludes: &["printed"],
        electronic: false,
    },
    CapDef {
        id: "waterproof",
        name: "Waterproof (IP67 class)",
        group: "Enclosure",
        help: "Gaskets, a pressure-equalising vent and sealed buttons, tested to IP67 under IEC 60529.",
        excludes: &[],
        electronic: false,
    },
];

/// When a target field applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Applies {
    Always,
    /// Battery or coin cell.
    Battery,
    /// BLE or Wi-Fi.
    Radio,
    Electronics,
    Quantum,
    Mechanical,
    /// Every kind except the quantum testbed and the mechanical part.
    Product,
}

pub struct TargetDef {
    pub id: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub default: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub log: bool,
    pub help: &'static str,
    pub applies: Applies,
}

pub const TARGETS: &[TargetDef] = &[
    TargetDef {
        id: "mass_g",
        label: "Maximum mass",
        unit: "g",
        default: 100.0,
        min: 1.0,
        max: 50000.0,
        step: 1.0,
        log: true,
        help: "Everything in the product, battery included.",
        applies: Applies::Always,
    },
    TargetDef {
        id: "life_h",
        label: "Battery life (reference use)",
        unit: "h",
        default: 8.0,
        min: 0.5,
        max: 50000.0,
        step: 0.5,
        log: true,
        help: "Hours from full to empty in the typical mix of use the duty cycles describe (8760 h is one year).",
        applies: Applies::Battery,
    },
    TargetDef {
        id: "cost_usd",
        label: "Parts cost per unit at volume",
        unit: "USD",
        default: 30.0,
        min: 0.5,
        max: 100000.0,
        step: 0.5,
        log: true,
        help: "Bill of materials (BOM) at your production quantity. A common rule of thumb keeps it near a quarter of the retail price.",
        applies: Applies::Always,
    },
    TargetDef {
        id: "quantity",
        label: "Production quantity",
        unit: "units",
        default: 1000.0,
        min: 1.0,
        max: 1000000.0,
        step: 1.0,
        log: true,
        help: "Units in the first production run; it sets volume prices and the enclosure process.",
        applies: Applies::Always,
    },
    TargetDef {
        id: "retail_usd",
        label: "Retail price",
        unit: "USD",
        default: 99.0,
        min: 1.0,
        max: 200000.0,
        step: 1.0,
        log: true,
        help: "What a customer pays; stored for the Program & Business Lab's unit economics.",
        applies: Applies::Always,
    },
    TargetDef {
        id: "length_mm",
        label: "Envelope length",
        unit: "mm",
        default: 80.0,
        min: 5.0,
        max: 2000.0,
        step: 1.0,
        log: true,
        help: "Longest outside dimension.",
        applies: Applies::Always,
    },
    TargetDef {
        id: "width_mm",
        label: "Envelope width",
        unit: "mm",
        default: 50.0,
        min: 5.0,
        max: 2000.0,
        step: 1.0,
        log: true,
        help: "Second outside dimension.",
        applies: Applies::Always,
    },
    TargetDef {
        id: "height_mm",
        label: "Envelope height",
        unit: "mm",
        default: 20.0,
        min: 2.0,
        max: 2000.0,
        step: 1.0,
        log: true,
        help: "Thickness.",
        applies: Applies::Always,
    },
    TargetDef {
        id: "range_m",
        label: "Radio range to a phone",
        unit: "m",
        default: 10.0,
        min: 1.0,
        max: 100.0,
        step: 0.5,
        log: true,
        help: "Indoor distance at which the Bluetooth link must still work (path-loss exponent 2.5, 90 % availability).",
        applies: Applies::Radio,
    },
    TargetDef {
        id: "hours_per_day",
        label: "Hours used per day",
        unit: "h/day",
        default: 8.0,
        min: 0.1,
        max: 24.0,
        step: 0.1,
        log: false,
        help: "Sets charge cycles per day and operating hours for the reliability estimate.",
        applies: Applies::Electronics,
    },
    TargetDef {
        id: "ambient_c",
        label: "Hottest ambient in use",
        unit: "°C",
        default: 30.0,
        min: 15.0,
        max: 60.0,
        step: 1.0,
        log: false,
        help: "Surface temperature = ambient + temperature rise, so design for the hottest room or car you expect.",
        applies: Applies::Electronics,
    },
    TargetDef {
        id: "design_years",
        label: "Design life",
        unit: "years",
        default: 2.0,
        min: 0.5,
        max: 15.0,
        step: 0.5,
        log: false,
        help: "How long it must keep working: sets the reliability targets (B10 life, MTBF).",
        applies: Applies::Electronics,
    },
    TargetDef {
        id: "fiber_km",
        label: "Fiber link distance",
        unit: "km",
        default: 10.0,
        min: 1.0,
        max: 150.0,
        step: 1.0,
        log: true,
        help: "Length of single-mode fiber between the two nodes (0.2 dB/km at 1550 nm).",
        applies: Applies::Quantum,
    },
    TargetDef {
        id: "load_n",
        label: "Design load",
        unit: "N",
        default: 50.0,
        min: 1.0,
        max: 5000.0,
        step: 1.0,
        log: true,
        help: "Force at the tip of the bracket (10 N ≈ the weight of 1 kg).",
        applies: Applies::Mechanical,
    },
];

/// How a metric responds to a project's own data (from the perturbation experiments).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricClass {
    /// Computed from this project's bill of materials, battery and design parameters.
    Generic,
    /// Computed from a parameter block the project carries (the wizard writes it when the capability is chosen).
    Conditional,
    /// Responds to some of the project's data but mixes in Iris geometry or an Iris scenario.
    Partial,
    /// Analyses the Iris glasses geometry, netlist, firmware or optics whatever the project is.
    Iris,
    /// A template result that does not depend on the product (same as Iris unless edited in the lab).
    Template,
}

pub struct MetricRow {
    pub keys: &'static [&'static str],
    pub lab: &'static str,
    pub class: MetricClass,
    pub responds_to: &'static str,
    pub note: &'static str,
}

/// Which metrics respond to a generic project's own data. Each row was
/// established by perturbing a generic project (component mass, power and
/// cost; battery; thermal network; frame; optics; PCB; audio; IMU; chip;
/// regulator; quantity; retail price; RF link; reliability use profile) and
/// recording which metrics moved, and by comparing against Iris.
pub const METRIC_TABLE: &[MetricRow] = &[
    MetricRow {
        keys: &["budget.avg_power_mw", "budget.battery_life_h"],
        lab: "power",
        class: MetricClass::Generic,
        responds_to: "component power × duty, regulator efficiency, quiescent current, battery capacity",
        note: "Systems roll-up; identical definition to the Power lab.",
    },
    MetricRow {
        keys: &[
            "power.avg_power_mw",
            "power.battery_life_h",
            "power.peak_power_mw",
            "power.energy_mwh",
        ],
        lab: "power",
        class: MetricClass::Generic,
        responds_to: "component power × duty, regulator efficiency, battery",
        note: "Peak = every load active at once.",
    },
    MetricRow {
        keys: &["thermal.max_skin_c", "thermal.board_max_c", "thermal.time_constant_s"],
        lab: "power",
        class: MetricClass::Generic,
        responds_to: "heat from the power budget (display always on), the lumped thermal network in design.thermal",
        note: "Two-node resistor–capacitor model; the wizard derives R and C from the enclosure size.",
    },
    MetricRow {
        keys: &["mass.components_g", "bom.cost_usd"],
        lab: "systems",
        class: MetricClass::Generic,
        responds_to: "bill of materials (BOM) mass and quantity-1 price",
        note: "The enclosure is a BOM line, so its mass and cost are included.",
    },
    MetricRow {
        keys: &["mfg.bom_cost_usd"],
        lab: "mfg",
        class: MetricClass::Generic,
        responds_to: "BOM prices with learning-curve price breaks at the target quantity",
        note: "Parts only — the frame and conversion costs are not in this number.",
    },
    MetricRow {
        keys: &["reliability.mtbf_h", "reliability.r_2yr_pct", "reliability.b10_years"],
        lab: "reliability",
        class: MetricClass::Generic,
        responds_to: "part classes in the BOM, board temperature from the thermal model, battery charge cycles, hours used per day",
        note: "Typical failure rates (FIT) — replace with supplier data.",
    },
    MetricRow {
        keys: &["dsp.mic_di_db", "dsp.decim_stopband_db"],
        lab: "dsp",
        class: MetricClass::Conditional,
        responds_to: "design.audio: microphone count and spacing, decimation chain",
        note: "Bound when the product has microphones.",
    },
    MetricRow {
        keys: &["circuit.buck_ripple_mv"],
        lab: "circuit",
        class: MetricClass::Conditional,
        responds_to: "battery voltage, core rail, the L1 inductor and C2 output-capacitor values parsed from the BOM",
        note: "Bound when a rechargeable battery feeds a buck converter.",
    },
    MetricRow {
        keys: &[
            "rf.link_margin_db",
            "rf.range_m",
            "rf.antenna_efficiency_pct",
            "rf.match_return_loss_db",
            "rf.feed_z0_ohm",
        ],
        lab: "rf",
        class: MetricClass::Conditional,
        responds_to: "design.extra.rf: link distance, losses, antenna efficiency, matching network, feed; board thickness",
        note: "The antenna impedance is the lab's assumed chip-antenna model; measure yours with a vector network analyser (VNA).",
    },
    MetricRow {
        keys: &[
            "quantum.qkd_key_rate_bps",
            "quantum.qkd_qber_pct",
            "quantum.qkd_max_distance_km",
            "quantum.qkd_link_loss_db",
            "quantum.qkd_plob_fraction_pct",
            "quantum.qrng_min_entropy_bits",
            "quantum.qrng_nist_failures",
            "quantum.qrng_supply_ratio",
        ],
        lab: "quantum",
        class: MetricClass::Conditional,
        responds_to: "design.extra.qkd and design.extra.qrng",
        note: "Absent unless the project carries those blocks (Iris has none).",
    },
    MetricRow {
        keys: &["control.head_rms_err_deg"],
        lab: "control",
        class: MetricClass::Partial,
        responds_to: "design.imu noise, bias, rate and filter blend",
        note: "Scores a synthetic 60 s head-motion scenario (the Iris use case), so it is not bound for other products.",
    },
    MetricRow {
        keys: &["rl.policy_life_gain_pct"],
        lab: "rl",
        class: MetricClass::Partial,
        responds_to: "component power and battery",
        note: "The user model's contexts (glance, navigate, caption) are the Iris use cases.",
    },
    MetricRow {
        keys: &["mfg.unit_cost_usd", "mfg.crossover_qty"],
        lab: "mfg",
        class: MetricClass::Partial,
        responds_to: "BOM, quantity, PCB size — plus the Iris frame's printing/molding cost",
        note: "Includes conversion of the Iris frame geometry.",
    },
    MetricRow {
        keys: &[
            "program.gross_margin_pct",
            "program.breakeven_units",
            "program.npv_usd",
            "program.bac_usd",
        ],
        lab: "program",
        class: MetricClass::Partial,
        responds_to: "retail price (design.extra.program), manufactured cost from the Manufacturing Hub",
        note: "Inherits the Iris frame cost through mfg.unit_cost_usd.",
    },
    MetricRow {
        keys: &[
            "build.compliance_items",
            "build.pcb_capability_violations",
            "build.stencil_min_area_ratio",
        ],
        lab: "build",
        class: MetricClass::Partial,
        responds_to: "markets (design.extra.markets), radios and batteries in the BOM, on-PCB parts",
        note: "Counts, useful for planning; not bound.",
    },
    MetricRow {
        keys: &["program.cpm_weeks", "program.p80_weeks", "program.min_trl", "program.min_mrl"],
        lab: "program",
        class: MetricClass::Template,
        responds_to: "the standard hardware task network (silicon subsystems add tasks)",
        note: "Same schedule for every product until you edit the tasks in the Program lab.",
    },
    MetricRow {
        keys: &["reliability.dvt_units"],
        lab: "reliability",
        class: MetricClass::Template,
        responds_to: "the reliability/confidence targets of the DVT plan",
        note: "",
    },
    MetricRow {
        keys: &["cad.*", "mass.total_g", "fit.pcb_pod_margin_mm"],
        lab: "cad",
        class: MetricClass::Iris,
        responds_to: "design.frame (the Iris glasses frame)",
        note: "mass.total_g adds the Iris frame to the BOM mass — use mass.components_g instead.",
    },
    MetricRow {
        keys: &["structures.*", "ergonomics.*", "fluids.*", "mechanisms.*", "molding.*", "print.*"],
        lab: "structures",
        class: MetricClass::Iris,
        responds_to: "the Iris frame, pod and hinge geometry",
        note: "Finite-element, fit, heat-conduction, hinge, molding and slicing analyses of the glasses.",
    },
    MetricRow {
        keys: &["optics.*"],
        lab: "optics",
        class: MetricClass::Iris,
        responds_to: "design.optics (a birdbath head-up display)",
        note: "Only meaningful for a near-eye display of that architecture.",
    },
    MetricRow {
        keys: &["chip.*", "firmware.*"],
        lab: "chip",
        class: MetricClass::Iris,
        responds_to: "the Iris FIR filter and firmware",
        note: "Same values for every product.",
    },
    MetricRow {
        keys: &["circuit.rail_droop_mv", "circuit.vbat_divider_ua"],
        lab: "circuit",
        class: MetricClass::Iris,
        responds_to: "Iris circuit presets (display load step, R1/R2 divider)",
        note: "",
    },
    MetricRow {
        keys: &["pcb.*", "robotics.pnp_cycle_s"],
        lab: "pcb",
        class: MetricClass::Iris,
        responds_to: "parts marked on-PCB, routed with the Iris schematic table",
        note: "Absent while board parts are off the PCB (the wizard leaves them off so live verification stays fast).",
    },
];

/// Metric keys the wizard may bind requirements to.
pub fn bindable(key: &str) -> bool {
    METRIC_TABLE
        .iter()
        .any(|r| matches!(r.class, MetricClass::Generic | MetricClass::Conditional) && r.keys.contains(&key))
}

pub fn kind(id: &str) -> Option<&'static KindDef> {
    KINDS.iter().find(|k| k.id == id)
}

pub fn cap(id: &str) -> Option<&'static CapDef> {
    CAPS.iter().find(|c| c.id == id)
}

pub fn target(id: &str) -> Option<&'static TargetDef> {
    TARGETS.iter().find(|t| t.id == id)
}

/// Default value of a target for a kind.
pub fn target_default(k: &KindDef, id: &str) -> f64 {
    k.defaults
        .iter()
        .find(|(t, _)| *t == id)
        .map(|(_, v)| *v)
        .or_else(|| target(id).map(|t| t.default))
        .unwrap_or(0.0)
}

/// `systems.wizard_options`: the vocabulary with help text.
pub fn options() -> Value {
    json!({
        "kinds": KINDS.iter().map(|k| json!({
            "id": k.id, "name": k.name, "icon": k.icon, "blurb": k.blurb, "help": k.help,
            "examples": k.examples, "default_capabilities": k.default_caps,
            "defaults": TARGETS.iter().map(|t| (t.id.to_string(), json!(target_default(k, t.id)))).collect::<serde_json::Map<_, _>>(),
            "contact": k.contact, "surface_limit_c": k.contact.surface_limit_c(), "scope": k.scope,
        })).collect::<Vec<_>>(),
        "capabilities": CAPS.iter().map(|c| json!({
            "id": c.id, "name": c.name, "group": c.group, "help": c.help, "excludes": c.excludes, "electronic": c.electronic,
        })).collect::<Vec<_>>(),
        "targets": TARGETS.iter().map(|t| json!({
            "id": t.id, "label": t.label, "unit": t.unit, "default": t.default, "min": t.min, "max": t.max,
            "step": t.step, "log": t.log, "help": t.help, "applies": t.applies,
        })).collect::<Vec<_>>(),
        "metric_table": METRIC_TABLE.iter().map(|r| json!({
            "keys": r.keys, "lab": r.lab, "class": r.class, "responds_to": r.responds_to, "note": r.note,
            "bound": matches!(r.class, MetricClass::Generic | MetricClass::Conditional),
        })).collect::<Vec<_>>(),
    })
}
