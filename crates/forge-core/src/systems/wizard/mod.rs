//! New-product wizard: turns a plain-language product description (kind,
//! capabilities, targets) into a starter project whose requirements are bound
//! to metrics the engine really computes for that project.
//!
//! * `systems.wizard_options` → the vocabulary: product kinds, capabilities,
//!   target fields (units, defaults, ranges) and the generic-vs-Iris metric
//!   table ([`catalog`]).
//! * `systems.wizard` → answers → `{ project, explanations, warnings, labs,
//!   summary }`.
//!
//! **Which metrics may a generated requirement be bound to?** Only those the
//! perturbation experiments showed respond to a generic project's own data
//! ([`catalog::METRIC_TABLE`], classes *generic* and *conditional*): the
//! bill-of-materials (BOM) mass and cost roll-ups, the power budget and
//! battery life, the lumped thermal network, BOM-driven reliability, the
//! microphone-array and decimation analysis, the buck-converter ripple, the
//! Bluetooth Low Energy (BLE) link budget and the quantum key distribution
//! (QKD) / quantum random number generator (QRNG) models. Everything else
//! analyses the Iris glasses geometry, netlist or firmware whatever the
//! project is, so requirements in those areas are verified by test,
//! inspection or demonstration — honestly "unverified" until done.
//!
//! **Sizing with visible maths** ([`Explanation`]): average power from duty
//! cycles; the battery from the power budget and target life (with a 20 %
//! growth allowance); the robot's motor power and battery mass iterated to a
//! fixed point; printed circuit board (PCB) area from footprint courtyards; the
//! enclosure's shell mass; the thermal resistance from its surface area;
//! a bracket's thickness from beam bending; a reliability target from the
//! design life.
//!
//! Part values are **typical** planning numbers, not quotes or part numbers.
//!
//! References: NASA Systems Engineering Handbook SP-2016-6105 Rev 2 (§4.2
//! technical requirements, §6.7 technical performance measures and margins);
//! D. Linden & T. B. Reddy, *Handbook of Batteries* (4th ed., 2011);
//! F. P. Incropera et al., *Fundamentals of Heat and Mass Transfer* (7th ed.,
//! 2011) ch. 9 (natural convection); R. C. Hibbeler, *Mechanics of Materials*
//! (10th ed., 2017) ch. 6 and 12 (bending stress, cantilever deflection);
//! P. D. T. O'Connor & A. Kleyner, *Practical Reliability Engineering*
//! (5th ed., 2012); IPC-7351B (land patterns and courtyards).

pub mod catalog;
mod content;
mod explain;
pub mod parts;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{BatteryParams, Project};
use catalog::{KindDef, CAPS, TARGETS};
use parts::{duty, Part, ACT, AUD, CMP, OPT, PCB, PWR, QLINK, QRNG, SNS, STR, UI};

/// What the user told the wizard.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Answers {
    pub name: String,
    /// One-line idea, e.g. "a clip-on sensor that buzzes when you slouch".
    pub idea: String,
    /// Who uses it (comma-separated).
    pub users: String,
    pub kind: String,
    /// Capability ids; `None` = the kind's defaults.
    pub capabilities: Option<Vec<String>>,
    /// Target values by id (see `systems.wizard_options`); missing = the kind's default.
    pub targets: BTreeMap<String, f64>,
    pub author: String,
}

/// One sizing step with its arithmetic shown.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Explanation {
    pub id: String,
    pub title: String,
    /// Plain-words answer first.
    pub plain: String,
    /// The arithmetic, one line each.
    pub math: Vec<String>,
    pub result: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Warning {
    /// `warn` (a target is at risk or something was changed) or `info`.
    pub level: String,
    pub text: String,
}

/// Whether a lab analyses this product's own data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LabNote {
    pub lab: String,
    /// `yours` | `partial` | `iris` | `unused`
    pub status: String,
    pub note: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Summary {
    pub mass_g: f64,
    pub avg_power_mw: f64,
    pub peak_power_mw: f64,
    pub battery: String,
    pub battery_mah: f64,
    pub life_h: Option<f64>,
    pub bom_usd: f64,
    pub bom_volume_usd: f64,
    pub board_mm: [f64; 2],
    pub board_layers: u32,
    pub enclosure: String,
    pub surface_c: Option<f64>,
    pub surface_limit_c: f64,
    pub bound_requirements: usize,
    pub test_requirements: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WizardOut {
    pub project: Project,
    pub explanations: Vec<Explanation>,
    pub warnings: Vec<Warning>,
    pub labs: Vec<LabNote>,
    pub summary: Summary,
    /// Capabilities after defaults, exclusions and additions.
    pub capabilities: Vec<String>,
    /// Targets after defaults and clamping.
    pub targets: BTreeMap<String, f64>,
}

/// Energy source chosen for the product.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    None,
    Lipo,
    Coin(usize),
}

#[derive(Clone, Debug)]
pub struct Battery {
    pub source: Source,
    pub mah: f64,
    pub params: BatteryParams,
    pub cost: f64,
    pub name: String,
    pub class: String,
    /// Required capacity before rounding to a standard size.
    pub required_mah: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Enclosure {
    /// `printed` | `molded` | `rack` | `none`
    pub process: String,
    pub wall_mm: f64,
    pub density: f64,
    pub material: String,
    pub surface_mm2: f64,
    pub volume_cm3: f64,
    pub mass_g: f64,
    pub cost_usd: f64,
    pub tooling_usd: f64,
    pub auto: bool,
    /// Mechanical kind: the bracket's thickness (mm).
    pub bracket_t_mm: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Board {
    pub parts_mm2: f64,
    pub needed_mm2: f64,
    pub factor: f64,
    pub w: f64,
    pub h: f64,
    pub layers: u32,
    pub thickness: f64,
    pub fits: bool,
    pub double_sided: bool,
    pub inner: [f64; 2],
}

/// The wizard's working state.
pub struct Ctx {
    pub kind: &'static KindDef,
    pub caps: Vec<&'static str>,
    pub t: BTreeMap<&'static str, f64>,
    pub name: String,
    pub idea: String,
    pub users: Vec<String>,
    pub author: String,
    pub electronics: bool,
    pub enclosure: Enclosure,
    pub battery: Battery,
    pub motor_mw: f64,
    pub parts: Vec<Part>,
    pub board: Board,
    pub avg_mw: f64,
    pub peak_mw: f64,
    pub iterations: Vec<(f64, f64, f64)>,
    pub warnings: Vec<Warning>,
    pub explanations: Vec<Explanation>,
}

pub fn fmt(x: f64) -> String {
    if !x.is_finite() {
        return "n/a".into();
    }
    let a = x.abs();
    if a >= 1000.0 {
        let s = format!("{:.0}", x);
        // thousands separators (thin spaces read badly in JSON; use commas)
        let neg = s.starts_with('-');
        let digits: Vec<char> = s.trim_start_matches('-').chars().collect();
        let mut out = String::new();
        for (i, c) in digits.iter().enumerate() {
            if i > 0 && (digits.len() - i).is_multiple_of(3) {
                out.push(',');
            }
            out.push(*c);
        }
        if neg {
            format!("-{out}")
        } else {
            out
        }
    } else if a >= 100.0 {
        format!("{:.0}", x)
    } else if a >= 10.0 {
        trim(format!("{:.1}", x))
    } else if a >= 1.0 {
        trim(format!("{:.2}", x))
    } else if a == 0.0 {
        "0".into()
    } else {
        let d = (-(a.log10().floor()) as usize + 1).min(6);
        trim(format!("{:.*}", d, x))
    }
}

fn trim(s: String) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

impl Ctx {
    pub fn has(&self, cap: &str) -> bool {
        self.caps.contains(&cap)
    }
    pub fn tv(&self, id: &str) -> f64 {
        self.t.get(id).copied().unwrap_or(0.0)
    }
    pub fn warn(&mut self, text: impl Into<String>) {
        self.warnings.push(Warning {
            level: "warn".into(),
            text: text.into(),
        });
    }
    pub fn info(&mut self, text: impl Into<String>) {
        self.warnings.push(Warning {
            level: "info".into(),
            text: text.into(),
        });
    }
    pub fn explain(&mut self, id: &str, title: &str, plain: String, math: Vec<String>, result: String) {
        self.explanations.push(Explanation {
            id: id.into(),
            title: title.into(),
            plain,
            math,
            result,
        });
    }
    pub fn is(&self, kind: &str) -> bool {
        self.kind.id == kind
    }
    pub fn has_battery(&self) -> bool {
        self.battery.source != Source::None
    }
    pub fn radio(&self) -> bool {
        self.has("ble") || self.has("wifi")
    }

    /// Resolve kind, capabilities and targets.
    fn new(a: &Answers) -> Result<Ctx, String> {
        let kind = catalog::kind(a.kind.trim()).ok_or_else(|| {
            format!(
                "unknown product kind `{}`; one of {}",
                a.kind,
                catalog::KINDS.iter().map(|k| k.id).collect::<Vec<_>>().join(", ")
            )
        })?;
        let mut c = Ctx {
            kind,
            caps: vec![],
            t: BTreeMap::new(),
            name: if a.name.trim().is_empty() {
                format!("New {}", kind.name.to_lowercase())
            } else {
                a.name.trim().to_string()
            },
            idea: a.idea.trim().trim_end_matches('.').to_string(),
            users: a
                .users
                .split([',', ';'])
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            author: a.author.trim().to_string(),
            electronics: false,
            enclosure: Enclosure::default(),
            battery: Battery {
                source: Source::None,
                mah: 0.0,
                params: BatteryParams::default(),
                cost: 0.0,
                name: String::new(),
                class: String::new(),
                required_mah: 0.0,
            },
            motor_mw: 0.0,
            parts: vec![],
            board: Board::default(),
            avg_mw: 0.0,
            peak_mw: 0.0,
            iterations: vec![],
            warnings: vec![],
            explanations: vec![],
        };
        if !kind.scope.is_empty() {
            c.info(kind.scope);
        }
        // --- capabilities: defaults, unknown ids, exclusions -------------
        let asked: Vec<String> = match &a.capabilities {
            Some(v) => v.iter().map(|s| s.trim().to_string()).collect(),
            None => kind.default_caps.iter().map(|s| s.to_string()).collect(),
        };
        let mut chosen: Vec<&'static str> = vec![];
        for id in &asked {
            let Some(cd) = catalog::cap(id) else {
                c.warn(format!("Unknown capability `{id}` ignored."));
                continue;
            };
            if chosen.contains(&cd.id) {
                continue;
            }
            if let Some(x) = chosen.iter().find(|x| cd.excludes.contains(x)) {
                let other = catalog::cap(x).map(|o| o.name).unwrap_or("");
                c.warn(format!("“{}” and “{other}” exclude each other: kept “{other}”.", cd.name));
                continue;
            }
            chosen.push(cd.id);
        }
        c.caps = chosen;
        c.electronics = !c.is("mechanical") || c.caps.iter().any(|x| catalog::cap(x).is_some_and(|d| d.electronic));
        // --- targets -------------------------------------------------------
        for td in TARGETS {
            let def = catalog::target_default(kind, td.id);
            let v = match a.targets.get(td.id) {
                Some(v) if v.is_finite() => {
                    let cl = v.clamp(td.min, td.max);
                    if (cl - v).abs() > 1e-9 {
                        c.warn(format!(
                            "{} {} {} is outside {}–{}: clamped to {}.",
                            td.label,
                            fmt(*v),
                            td.unit,
                            fmt(td.min),
                            fmt(td.max),
                            fmt(cl)
                        ));
                    }
                    cl
                }
                _ => def,
            };
            c.t.insert(td.id, v);
        }
        for k in a.targets.keys() {
            if catalog::target(k).is_none() {
                c.warn(format!("Unknown target `{k}` ignored."));
            }
        }
        // --- additions that keep the product buildable ---------------------
        if c.electronics && !c.is("quantum") {
            let powered = ["battery", "coin_cell", "usb_c", "dc_adapter"].iter().any(|x| c.has(x));
            if !powered {
                c.add_cap("usb_c", "No power source was chosen: added USB-C power.");
            }
            if c.has("battery") && !c.has("usb_c") && !c.has("dc_adapter") {
                c.add_cap("usb_c", "A rechargeable battery needs a way to charge: added USB-C charging.");
            }
        }
        if c.is("quantum") && !c.has("dc_adapter") {
            c.add_cap(
                "dc_adapter",
                "The quantum testbed runs from certified bench supplies (low-voltage DC).",
            );
        }
        if !c.is("dev_board") && !c.is("quantum") && !c.has("printed") && !c.has("molded") {
            let q = c.tv("quantity");
            let pick = if q >= 2000.0 { "molded" } else { "printed" };
            c.caps.push(pick);
            c.enclosure.auto = true;
        }
        let order = |id: &&str| CAPS.iter().position(|x| x.id == *id).unwrap_or(usize::MAX);
        c.caps.sort_by_key(order);
        // --- feasibility warnings -----------------------------------------
        if c.has("coin_cell") {
            for (cap, why) in [
                ("wifi", "Wi-Fi transmits at a few hundred milliamps"),
                ("camera", "a camera draws over 100 mW while it captures"),
                ("motors", "motors draw hundreds of milliamps"),
                ("speaker", "a speaker amplifier draws tens of milliamps"),
            ] {
                if c.has(cap) {
                    let n = catalog::cap(cap).map(|x| x.name).unwrap_or(cap);
                    c.warn(format!("Coin cell + {n}: {why}, far above the 10–15 mA pulses a coin cell delivers. Expect the peak-power requirement to fail — use a rechargeable battery."));
                }
            }
        }
        if c.has("camera") && !c.has("wifi") {
            c.info("A camera needs a Wi-Fi-class SoC with a camera interface and frame memory: the compute part was upgraded.");
        }
        if c.is("dev_board") && (c.has("printed") || c.has("molded")) {
            c.info("Dev board with an enclosure: the board size target is the envelope; the board is sized to fit inside it.");
        }
        Ok(c)
    }

    fn add_cap(&mut self, id: &'static str, why: &str) {
        if !self.has(id) {
            self.caps.push(id);
            self.info(why);
        }
    }

    // -----------------------------------------------------------------------
    // Enclosure and bracket
    // -----------------------------------------------------------------------

    fn size_enclosure(&mut self) {
        let (l, w, h) = (self.tv("length_mm"), self.tv("width_mm"), self.tv("height_mm"));
        let q = self.tv("quantity");
        let surface = 2.0 * (l * w + l * h + w * h);
        let (process, wall, rho, material) = if self.is("quantum") {
            ("rack", 1.5, 2.7, "Aluminium sheet")
        } else if self.has("molded") {
            ("molded", 1.5, 1.15, "PC/ABS")
        } else if self.has("printed") {
            ("printed", 2.0, 1.27, "PETG")
        } else {
            ("none", 0.0, 0.0, "")
        };
        let mut e = Enclosure {
            process: process.into(),
            wall_mm: wall,
            density: rho,
            material: material.into(),
            surface_mm2: surface,
            auto: self.enclosure.auto,
            ..Default::default()
        };
        if self.is("mechanical") {
            self.size_bracket(&mut e);
        } else if process != "none" {
            // shell volume ≈ surface × wall, +15 % for bosses, ribs and lips
            e.volume_cm3 = surface * wall * 1.15 / 1000.0;
            e.mass_g = e.volume_cm3 * rho;
        }
        let (cost, tooling) = process_cost(process, e.volume_cm3, e.mass_g, l.max(w), q);
        e.cost_usd = cost;
        e.tooling_usd = tooling;
        let mut math = vec![];
        if !self.is("mechanical") && process != "none" {
            math.push(format!(
                "Shell surface A = 2(LW + LH + WH) = 2({l:.0}·{w:.0} + {l:.0}·{h:.0} + {w:.0}·{h:.0}) = {} mm²",
                fmt(surface)
            ));
            math.push(format!(
                "Shell volume V = A × wall × 1.15 (bosses, ribs) = {} × {wall} mm × 1.15 = {} cm³",
                fmt(surface),
                fmt(e.volume_cm3)
            ));
            math.push(format!(
                "Mass m = V × ρ = {} cm³ × {rho} g/cm³ ({material}) = {} g",
                fmt(e.volume_cm3),
                fmt(e.mass_g)
            ));
        }
        if process == "printed" {
            let hours = e.volume_cm3 / 12.0 + 0.3;
            math.push(format!(
                "Print cost ≈ filament {} g × 1.15 waste × 25 USD/kg + {} h × 1.5 USD/h machine time = {} USD (typical)",
                fmt(e.mass_g),
                fmt(hours),
                fmt(cost)
            ));
        } else if process == "molded" {
            math.push(format!(
                "Molded piece ≈ material {} g × 3.5 USD/kg × 1.1 + 2 shots × 30 s × 40 USD/h = {} USD per set (typical)",
                fmt(e.mass_g),
                fmt(cost)
            ));
            math.push(format!(
                "Tooling (not in the BOM) ≈ 2 molds × (5,000 + 40 × {:.0} mm) = {} USD — rough order of magnitude; get quotes",
                l.max(w),
                fmt(tooling)
            ));
        }
        if !self.is("mechanical") && process != "none" {
            let why = if e.auto {
                format!(
                    "You did not pick a process, so the wizard chose {} for {} units: printing needs no tooling and wins below roughly 2,000 units; molding wins above (see the trade study).",
                    if process == "molded" { "injection molding" } else { "3-D printing" },
                    fmt(q)
                )
            } else {
                format!(
                    "A {} {material} shell with {wall} mm walls around the {l:.0} × {w:.0} × {h:.0} mm envelope.",
                    process
                )
            };
            self.explain(
                "enclosure",
                "Enclosure: mass and cost",
                why,
                math,
                format!("{} g, {} USD per set", fmt(e.mass_g), fmt(e.cost_usd)),
            );
        }
        self.enclosure = e;
    }

    /// Cantilever L-bracket sized by bending stress (mechanical kind).
    fn size_bracket(&mut self, e: &mut Enclosure) {
        let (l, b, h) = (self.tv("length_mm"), self.tv("width_mm"), self.tv("height_mm"));
        let f = self.tv("load_n");
        let molded = self.has("molded");
        let (sy, k, emod) = if molded { (60.0, 1.0, 2300.0) } else { (50.0, 0.6, 2000.0) };
        let sf = 2.0;
        let allow = sy * k / sf;
        let t_req = (6.0 * f * l / (b * allow)).sqrt();
        let d_allow = parts::nice_up((l * 0.02).max(0.5));
        let t_stiff = (4.0 * f * l.powi(3) / (emod * d_allow * b)).cbrt();
        let t = (t_req.max(t_stiff) * 2.0).ceil() / 2.0;
        let t = t.max(2.0);
        let i = b * t.powi(3) / 12.0;
        let defl = f * l.powi(3) / (3.0 * emod * i);
        let stress = 6.0 * f * l / (b * t * t);
        e.bracket_t_mm = t;
        e.volume_cm3 = (l * b * t + h * b * t) * 1.2 / 1000.0;
        e.mass_g = e.volume_cm3 * e.density;
        let math = vec![
            format!(
                "Bending moment at the root M = F × L = {} N × {} mm = {} N·mm",
                fmt(f),
                fmt(l),
                fmt(f * l)
            ),
            format!(
                "Allowable stress σ_allow = σ_y × k / SF = {sy} MPa × {k} {} / {sf} = {} MPa",
                if molded { "(molded)" } else { "(across printed layers, typical)" },
                fmt(allow)
            ),
            format!(
                "Rectangular section σ = 6M / (b t²) ⇒ t ≥ √(6 F L / (b σ_allow)) = √(6 × {} × {} / ({} × {})) = {} mm",
                fmt(f),
                fmt(l),
                fmt(b),
                fmt(allow),
                fmt(t_req)
            ),
            format!(
                "Stiffness: δ = F L³ / (3 E I) with I = b t³/12 ≤ {} mm (2 % of L) ⇒ t ≥ ∛(4 F L³ / (E δ b)) = {} mm (E = {} MPa, typical)",
                fmt(d_allow),
                fmt(t_stiff),
                fmt(emod)
            ),
            format!(
                "Chosen t = {} mm (the larger, rounded up to 0.5 mm, at least 2 mm) ⇒ σ = {} MPa, I = {} mm⁴, δ = {} mm",
                fmt(t),
                fmt(stress),
                fmt(i),
                fmt(defl)
            ),
            format!(
                "Mass ≈ (L + H) × b × t × 1.2 (fillets, holes) × ρ = ({} + {}) × {} × {} × 1.2 × {} = {} g",
                fmt(l),
                fmt(h),
                fmt(b),
                fmt(t),
                e.density,
                fmt(e.mass_g)
            ),
        ];
        if t > h * 0.6 {
            self.warn(format!(
                "The bracket needs {} mm of thickness, more than 60 % of the {} mm height: widen it, shorten the arm or use a stiffer material.",
                fmt(t),
                fmt(h)
            ));
        }
        self.explain(
            "bracket",
            "Why this thickness?",
            format!(
                "A {} N load at the tip of a {} mm arm bends the root the most. The arm is made thick enough that the bending stress stays at half of the material's typical strength and the tip sags at most 2 % of the arm length — whichever needs more material ({}).",
                fmt(f),
                fmt(l),
                if t_stiff > t_req { "here stiffness" } else { "here strength" }
            ),
            math,
            format!("t = {} mm, σ = {} MPa, δ = {} mm, {} g", fmt(t), fmt(stress), fmt(defl), fmt(e.mass_g)),
        );
    }

    // -----------------------------------------------------------------------
    // Battery
    // -----------------------------------------------------------------------

    /// Size the energy source for an average power (mW). Returns the battery.
    fn battery_for(&self, avg_mw: f64) -> Battery {
        let life = self.tv("life_h");
        let margin = 1.2;
        if self.has("battery") {
            let usable = 0.85;
            let v = 3.7;
            let hours = (life * margin).max(self.cycle_life_h());
            let req = avg_mw * hours / (v * usable);
            let mah = parts::LIPO_SIZES
                .iter()
                .copied()
                .find(|s| *s >= req)
                .unwrap_or(*parts::LIPO_SIZES.last().unwrap());
            let mass = mah * v / parts::lipo_mwh_per_g(mah);
            Battery {
                source: Source::Lipo,
                mah,
                params: BatteryParams {
                    chemistry: "LiPo".into(),
                    capacity_mah: mah,
                    nominal_v: v,
                    full_v: 4.2,
                    cutoff_v: 3.3,
                    internal_resistance_ohm: parts::round_to((0.35 * 180.0 / mah).clamp(0.03, 1.5), 3),
                    mass_g: parts::round_to(mass, 2),
                    usable_fraction: usable,
                },
                cost: parts::lipo_cost(mah),
                name: format!("LiPo cell {} mAh", fmt(mah)),
                class: "3.7 V lithium-polymer pouch cell with protection circuit".into(),
                required_mah: req,
            }
        } else if self.has("coin_cell") {
            let usable = 0.85;
            let v = 3.0;
            let req = avg_mw * life * margin / (v * usable);
            let idx = parts::COINS.iter().position(|c| c.mah >= req).unwrap_or(parts::COINS.len() - 1);
            let coin = &parts::COINS[idx];
            Battery {
                source: Source::Coin(idx),
                mah: coin.mah,
                params: BatteryParams {
                    chemistry: "Li-MnO2 coin cell".into(),
                    capacity_mah: coin.mah,
                    nominal_v: v,
                    full_v: 3.2,
                    cutoff_v: 2.0,
                    internal_resistance_ohm: if idx == 0 { 15.0 } else { 10.0 },
                    mass_g: coin.mass_g,
                    usable_fraction: usable,
                },
                cost: coin.cost,
                name: format!("{} coin cell", coin.name),
                class: format!("3 V lithium manganese dioxide coin cell, {} mAh typical", fmt(coin.mah)),
                required_mah: req,
            }
        } else {
            let (v, chem) = if self.is("quantum") {
                (12.0, "none (certified bench supplies)")
            } else if self.has("dc_adapter") {
                (12.0, "none (external 12 V adapter)")
            } else if self.electronics {
                (5.0, "none (USB 5 V supply)")
            } else {
                (0.0, "none")
            };
            Battery {
                source: Source::None,
                mah: 0.0,
                params: BatteryParams {
                    chemistry: chem.into(),
                    capacity_mah: 0.0,
                    nominal_v: v,
                    full_v: v,
                    cutoff_v: v * 0.9,
                    internal_resistance_ohm: 0.1,
                    mass_g: 0.0,
                    usable_fraction: 1.0,
                },
                cost: 0.0,
                name: String::new(),
                class: String::new(),
                required_mah: 0.0,
            }
        }
    }

    /// Weibull wear-out of a rechargeable cell from the Reliability Lab's
    /// part library: (β, η in equivalent full cycles).
    pub fn cell_wear(&self) -> (f64, f64) {
        let c = crate::project::Component {
            name: "LiPo cell".into(),
            part_class: "lithium-polymer pouch cell".into(),
            ..Default::default()
        };
        crate::reliability::iris::classify(&c)
            .wear
            .map(|w| (w.0, w.1))
            .unwrap_or((4.0, 800.0))
    }

    /// Cycles to 10 % of cells worn out: B10 = η (−ln 0.9)^(1/β).
    pub fn cell_b10_cycles(&self) -> f64 {
        let (beta, eta) = self.cell_wear();
        eta * (-(0.9f64.ln())).powf(1.0 / beta)
    }

    /// Hours per charge so the cell's B10 wear-out outlasts the design life
    /// (15 % allowance for the other failure modes in the system B10).
    pub fn cycle_life_h(&self) -> f64 {
        if !self.has("battery") {
            return 0.0;
        }
        self.tv("design_years") * 365.0 * self.tv("hours_per_day") * 1.15 / self.cell_b10_cycles()
    }

    pub fn core_v(&self) -> f64 {
        if self.has("coin_cell") {
            3.0
        } else if ["wifi", "camera", "display", "speaker", "motors"].iter().any(|x| self.has(x)) || !self.has("battery") {
            3.3
        } else {
            1.8
        }
    }

    pub fn regulator_eff(&self) -> f64 {
        if self.has("battery") || self.has("coin_cell") {
            0.9
        } else {
            0.88
        }
    }

    pub fn quiescent_ua(&self) -> f64 {
        if !self.electronics || self.is("quantum") {
            0.0
        } else if self.has("coin_cell") {
            0.5
        } else if self.has("battery") {
            5.0
        } else {
            30.0
        }
    }

    // -----------------------------------------------------------------------
    // Parts list
    // -----------------------------------------------------------------------

    /// Every BOM line for the current battery and motor power (no bare PCB yet).
    fn build_parts(&self, bat: &Battery, motor_mw: f64) -> Vec<Part> {
        let k = self.kind.id;
        let mut v: Vec<Part> = vec![];
        let e = &self.enclosure;
        if self.is("quantum") {
            return self.quantum_parts();
        }
        // ---- compute & radio ------------------------------------------------
        if self.electronics {
            let soc_d = duty(k, "soc");
            if self.has("wifi") || self.has("camera") {
                let name = if self.has("camera") {
                    "Wi-Fi + BLE system-on-chip (camera interface)"
                } else {
                    "Wi-Fi + BLE system-on-chip"
                };
                v.push(
                    Part::new("U1", name, "2.4 GHz Wi-Fi/BLE SoC, dual-core", CMP)
                        .cost(3.5)
                        .mass(0.2)
                        .pw(250.0, 3.0, duty(k, "wifi"))
                        .fp("QFN-48_6x6"),
                );
            } else if self.has("ble") {
                v.push(
                    Part::new("U1", "BLE system-on-chip", "Arm Cortex-M BLE 5 SoC with integrated DC-DC", CMP)
                        .cost(3.0)
                        .mass(0.1)
                        .pw(9.0, 0.006, soc_d)
                        .fp("QFN-48_6x6"),
                );
            } else {
                v.push(
                    Part::new("U1", "Low-power microcontroller (MCU)", "32-bit Arm Cortex-M MCU", CMP)
                        .cost(1.6)
                        .mass(0.1)
                        .pw(5.0, 0.005, soc_d)
                        .fp("QFN-48_6x6"),
                );
            }
            v.push(
                Part::new("Y1", "32 MHz quartz crystal", "SMD quartz crystal 2016", CMP)
                    .cost(0.15)
                    .mass(0.01)
                    .fp("XTAL-4_2x1.6"),
            );
            v.push(
                Part::new("C3", "SoC decoupling 100 nF", "X7R MLCC 0402", CMP)
                    .qty(4.0)
                    .cost(0.01)
                    .mass(0.002)
                    .fp("0402"),
            );
            if ["wifi", "camera", "microphones"].iter().any(|x| self.has(x)) || (self.has("display") && !self.is("iot_sensor")) {
                v.push(
                    Part::new("U6", "Serial NOR flash 16 MB", "QSPI NOR flash", CMP)
                        .cost(0.6)
                        .mass(0.02)
                        .pw(18.0, 0.002, 0.03)
                        .fp("USON-8_2x3"),
                );
            }
            if self.radio() {
                v.push(
                    Part::new("AE1", "2.4 GHz chip antenna", "Ceramic chip antenna 3216", CMP)
                        .cost(0.25)
                        .mass(0.02)
                        .fp("ANT_3216"),
                );
                if self.has("ble") {
                    for (i, el) in self.rf_network().iter().enumerate() {
                        let is_l = el.kind.is_l();
                        let refdes = format!("{}{}", if is_l { "L" } else { "C" }, 10 + i);
                        let what = if el.kind.is_series() { "series" } else { "shunt" };
                        let label = el.label();
                        v.push(
                            Part::new(
                                &refdes,
                                &format!(
                                    "RF match {what} {} ({})",
                                    if is_l { "inductor" } else { "capacitor" },
                                    label.replace("series ", "").replace("shunt ", "")
                                ),
                                if is_l {
                                    "RF multilayer chip inductor 0402"
                                } else {
                                    "C0G RF capacitor 0402"
                                },
                                CMP,
                            )
                            .cost(0.03)
                            .mass(0.001)
                            .fp("0402"),
                        );
                    }
                }
            }
        }
        // ---- power -------------------------------------------------------------
        match bat.source {
            Source::Lipo => {
                v.push(Part::new("BT1", &bat.name, &bat.class, PWR).cost(bat.cost).mass(bat.params.mass_g));
                v.push(
                    Part::new("J3", "Battery connector", "2-pin wire-to-board connector", PWR)
                        .cost(0.1)
                        .mass(0.05)
                        .fp("JST-SH-2"),
                );
                v.push(
                    Part::new("U4", "Li-ion linear charger", "Single-cell CC/CV charger", PWR)
                        .cost(0.7)
                        .mass(0.01)
                        .pw(0.0, 0.004, 0.0)
                        .fp("SOT-23-5"),
                );
                v.push(
                    Part::new("C5", "Charger cap 1 uF", "X5R MLCC 0402", PWR)
                        .cost(0.01)
                        .mass(0.002)
                        .fp("0402"),
                );
                v.push(
                    Part::new("R5", "Charge current set resistor", "Thick film 0402", PWR)
                        .cost(0.005)
                        .mass(0.001)
                        .fp("0402"),
                );
            }
            Source::Coin(i) => {
                let c = &parts::COINS[i];
                v.push(Part::new("BT1", &bat.name, &bat.class, PWR).cost(bat.cost).mass(c.mass_g));
                v.push(
                    Part::new("BH1", &format!("{} coin-cell holder", c.name), "SMD coin-cell retainer", PWR)
                        .cost(0.3)
                        .mass(1.0)
                        .area(c.holder_mm2),
                );
                v.push(
                    Part::new("C7", "Coin-cell bulk capacitor 47 uF", "X5R MLCC 0805", PWR)
                        .cost(0.08)
                        .mass(0.01)
                        .fp("0805"),
                );
            }
            Source::None => {}
        }
        if self.electronics && !self.has("coin_cell") {
            let vin = if self.has("battery") {
                "battery"
            } else if self.has("dc_adapter") {
                "12 V"
            } else {
                "5 V"
            };
            v.push(
                Part::new(
                    "U5",
                    &format!("Buck converter {vin} → {} V", self.core_v()),
                    "Synchronous buck converter",
                    PWR,
                )
                .cost(0.9)
                .mass(0.01)
                .fp("DFN-6_2x2"),
            );
            v.push(
                Part::new("L1", "Buck inductor 2.2 uH", "Shielded power inductor", PWR)
                    .cost(0.12)
                    .mass(0.03)
                    .fp("0805"),
            );
            v.push(
                Part::new("C1", "Buck input cap 4.7 uF", "X5R MLCC 0402", PWR)
                    .cost(0.02)
                    .mass(0.002)
                    .fp("0402"),
            );
            v.push(
                Part::new("C2", "Buck output cap 10 uF", "X5R MLCC 0603", PWR)
                    .cost(0.03)
                    .mass(0.003)
                    .fp("0603"),
            );
        }
        if self.has("usb_c") {
            v.push(
                Part::new("J1", "USB-C receptacle (power)", "USB Type-C 16-pin receptacle", PWR)
                    .cost(0.3)
                    .mass(0.6)
                    .fp("USB-C_16P"),
            );
            v.push(
                Part::new("R6", "USB-C CC pull-down 5.1 k", "Thick film 0402", PWR)
                    .qty(2.0)
                    .cost(0.005)
                    .mass(0.001)
                    .fp("0402"),
            );
            v.push(
                Part::new("D2", "USB ESD protection diode array", "TVS diode array", PWR)
                    .cost(0.08)
                    .mass(0.01)
                    .fp("SOT-23-6"),
            );
        }
        if self.has("dc_adapter") {
            v.push(
                Part::new("J2", "DC barrel jack 5.5 × 2.1 mm", "Through-hole DC power jack", PWR)
                    .cost(0.35)
                    .mass(2.0)
                    .area(126.0),
            );
            v.push(
                Part::new("Q1", "Reverse-polarity protection MOSFET", "P-channel MOSFET, SOT-23", PWR)
                    .cost(0.08)
                    .mass(0.01)
                    .area(9.0),
            );
            v.push(
                Part::new(
                    "PS1",
                    "External 12 V DC adapter (certified, in the box)",
                    "Wall adapter, 12 V 1 A, with safety certification",
                    PWR,
                )
                .cost(4.0)
                .mass(0.0),
            );
        }
        // ---- user interface --------------------------------------------------
        if self.has("display") {
            let (name, class, cost, mass, act, sleep) = parts::display_for(k);
            v.push(
                Part::new("U2", name, class, UI)
                    .cost(cost)
                    .mass(mass)
                    .pw(act, sleep, duty(k, "display")),
            );
            v.push(
                Part::new("J4", "Display FPC connector", "0.5 mm pitch FPC connector", UI)
                    .cost(0.25)
                    .mass(0.1)
                    .fp("FPC-24_0.5"),
            );
        }
        if self.has("buttons") {
            if self.is("wearable") {
                v.push(
                    Part::new("U11", "Capacitive touch controller", "Self-capacitance touch IC, 2 channels", UI)
                        .cost(0.9)
                        .mass(0.01)
                        .pw(0.05, 0.005, 1.0)
                        .fp("DFN-6_2x2"),
                );
            } else {
                let n = if self.is("handheld") { 4.0 } else { 2.0 };
                v.push(
                    Part::new("SW1", "Tactile push button", "SMD tactile switch 4 × 4 mm", UI)
                        .qty(n)
                        .cost(0.12)
                        .mass(0.3)
                        .area(30.0),
                );
            }
        }
        if self.has("leds") {
            v.push(
                Part::new("D1", "RGB status LED", "PLCC-4 RGB LED", UI)
                    .cost(0.12)
                    .mass(0.01)
                    .pw(8.0, 0.0, duty(k, "leds"))
                    .area(6.0),
            );
            v.push(
                Part::new("R7", "LED current-limit resistor", "Thick film 0402", UI)
                    .qty(3.0)
                    .cost(0.005)
                    .mass(0.001)
                    .fp("0402"),
            );
        }
        // ---- audio -------------------------------------------------------------
        if self.has("microphones") {
            for (r, side) in [("MK1", "left"), ("MK2", "right")] {
                v.push(
                    Part::new(
                        r,
                        &format!("PDM MEMS microphone ({side})"),
                        "Bottom-port digital MEMS microphone",
                        AUD,
                    )
                    .cost(0.8)
                    .mass(0.02)
                    .pw(0.9, 0.02, duty(k, "mics"))
                    .fp("MIC_LGA_3.5x2.65"),
                );
            }
        }
        if self.has("speaker") {
            let big = matches!(k, "desk_instrument" | "smart_home" | "robot");
            v.push(
                Part::new("U7", "Class-D audio amplifier", "Mono class-D amplifier", AUD)
                    .cost(0.6)
                    .mass(0.01)
                    .pw(if big { 300.0 } else { 150.0 }, 0.01, duty(k, "speaker"))
                    .area(6.0),
            );
            if big {
                v.push(
                    Part::new("SP1", "Speaker 28 mm 4 Ω", "Micro speaker with enclosure", AUD)
                        .cost(1.5)
                        .mass(8.0),
                );
            } else {
                v.push(Part::new("SP1", "Speaker 15 mm 8 Ω", "Micro speaker", AUD).cost(1.0).mass(3.0));
            }
        }
        // ---- sensing -----------------------------------------------------------
        if self.has("imu") {
            if self.has("coin_cell") {
                v.push(
                    Part::new(
                        "U3",
                        "3-axis accelerometer (wake-on-motion)",
                        "Ultra-low-power accelerometer, I2C",
                        SNS,
                    )
                    .cost(0.8)
                    .mass(0.02)
                    .pw(0.03, 0.0, 1.0)
                    .fp("LGA-14_3x2.5"),
                );
            } else {
                v.push(
                    Part::new("U3", "6-axis IMU (accelerometer + gyroscope)", "MEMS IMU, I2C/SPI", SNS)
                        .cost(2.4)
                        .mass(0.02)
                        .pw(0.9, 0.01, 1.0)
                        .fp("LGA-14_3x2.5"),
                );
            }
            v.push(
                Part::new("C4", "IMU decoupling 100 nF", "X7R MLCC 0402", SNS)
                    .cost(0.01)
                    .mass(0.002)
                    .fp("0402"),
            );
        }
        if self.has("env_sensors") {
            v.push(
                Part::new(
                    "U8",
                    "Environmental sensor (temperature, humidity, pressure)",
                    "Digital T/RH/P sensor, I2C",
                    SNS,
                )
                .cost(3.0)
                .mass(0.01)
                .pw(1.0, 0.0003, duty(k, "env"))
                .area(9.0),
            );
        }
        if self.has("imu") || self.has("env_sensors") || (self.has("buttons") && self.is("wearable")) {
            let sub = if self.has("imu") || self.has("env_sensors") { SNS } else { UI };
            v.push(
                Part::new("R3", "I2C SCL pull-up 4.7 k", "Thick film 0402", sub)
                    .cost(0.005)
                    .mass(0.001)
                    .fp("0402"),
            );
            v.push(
                Part::new("R4", "I2C SDA pull-up 4.7 k", "Thick film 0402", sub)
                    .cost(0.005)
                    .mass(0.001)
                    .fp("0402"),
            );
        }
        if self.has("camera") {
            v.push(
                Part::new(
                    "U10",
                    "Camera module 2 MP",
                    "2 MP CMOS camera module with lens, parallel (DVP) interface",
                    SNS,
                )
                .cost(5.0)
                .mass(2.0)
                .pw(130.0, 0.05, duty(k, "camera")),
            );
            v.push(
                Part::new("J5", "Camera FPC connector", "0.5 mm pitch FPC connector", SNS)
                    .cost(0.25)
                    .mass(0.1)
                    .fp("FPC-24_0.5"),
            );
            v.push(
                Part::new("U12", "PSRAM 8 MB (camera frame buffer)", "QSPI pseudo-static RAM", SNS)
                    .cost(1.2)
                    .mass(0.02)
                    .pw(30.0, 0.05, duty(k, "camera"))
                    .fp("USON-8_2x3"),
            );
        }
        // ---- motion -----------------------------------------------------------
        if self.has("motors") {
            if self.is("robot") {
                for (r, side) in [("M1", "left"), ("M2", "right")] {
                    v.push(
                        Part::new(
                            r,
                            &format!("Gear motor with encoder ({side})"),
                            "N20-class 6 V brushed gear motor with Hall encoder",
                            ACT,
                        )
                        .cost(3.5)
                        .mass(10.0)
                        .pw(motor_mw, 0.0, duty(k, "motors")),
                    );
                }
                v.push(
                    Part::new("U9", "Dual H-bridge motor driver", "Dual H-bridge, 1 A per channel", ACT)
                        .cost(1.2)
                        .mass(0.05)
                        .pw(5.0, 0.05, 1.0)
                        .area(25.0),
                );
                v.push(
                    Part::new("WH1", "Wheels with tyres (pair)", "Printed hub + O-ring tyre", ACT)
                        .cost(1.0)
                        .mass(16.0),
                );
                v.push(Part::new("CAS1", "Ball caster", "Steel ball caster", ACT).cost(0.5).mass(4.0));
            } else {
                v.push(
                    Part::new("M1", "Gear motor actuator", "N20-class 6 V brushed gear motor", ACT)
                        .cost(3.5)
                        .mass(10.0)
                        .pw(600.0, 0.0, duty(k, "motors")),
                );
                v.push(
                    Part::new("U9", "Single H-bridge motor driver", "H-bridge, 1 A", ACT)
                        .cost(0.8)
                        .mass(0.02)
                        .pw(3.0, 0.02, 1.0)
                        .area(12.0),
                );
            }
        }
        // ---- optics (optical device) -----------------------------------------
        if self.is("optical") {
            v.push(
                Part::new(
                    "LED1",
                    "Illumination LED (1 W class, white)",
                    "High-power LED on a metal-core star",
                    OPT,
                )
                .cost(1.2)
                .mass(1.0)
                .pw(1000.0, 0.0, duty(k, "illum")),
            );
            v.push(
                Part::new("U13", "Constant-current LED driver", "Buck LED driver, 350 mA", OPT)
                    .cost(0.6)
                    .mass(0.02)
                    .pw(30.0, 0.01, duty(k, "illum"))
                    .area(9.0),
            );
            v.push(
                Part::new("LNS1", "Collimating lens (PMMA, 20 mm)", "Molded aspheric lens", OPT)
                    .cost(1.5)
                    .mass(2.0),
            );
            v.push(
                Part::new("PD1", "Photodiode + transimpedance amplifier", "Si photodiode with op-amp TIA", OPT)
                    .cost(2.5)
                    .mass(0.2)
                    .pw(5.0, 0.01, duty(k, "illum"))
                    .area(20.0),
            );
            v.push(
                Part::new("OB1", "Optical bench insert", "Printed lens/LED carrier", OPT)
                    .cost(0.5)
                    .mass(10.0),
            );
        }
        // ---- structure ----------------------------------------------------------
        let encl_sub = STR;
        match e.process.as_str() {
            "printed" | "molded" => {
                let printed = e.process == "printed";
                if self.is("mechanical") {
                    v.push(
                        Part::new(
                            "ENC1",
                            &format!(
                                "{} bracket (L-shape, {} mm thick)",
                                if printed { "Printed" } else { "Injection-molded" },
                                fmt(e.bracket_t_mm)
                            ),
                            &format!("{}, solid infill in the arm", e.material),
                            encl_sub,
                        )
                        .cost(e.cost_usd)
                        .mass(e.mass_g),
                    );
                    v.push(
                        Part::new("HW1", "Mounting screws (M4)", "Steel screws", encl_sub)
                            .qty(4.0)
                            .cost(0.05)
                            .mass(1.5),
                    );
                } else {
                    let name = if self.is("robot") {
                        if printed {
                            "Printed chassis and covers (set)"
                        } else {
                            "Injection-molded chassis and covers (set)"
                        }
                    } else if printed {
                        "Printed enclosure shells (set)"
                    } else {
                        "Injection-molded enclosure shells (set)"
                    };
                    v.push(
                        Part::new("ENC1", name, &format!("{}, 2 parts, {} mm walls", e.material, e.wall_mm), encl_sub)
                            .cost(e.cost_usd)
                            .mass(e.mass_g),
                    );
                    if printed {
                        v.push(
                            Part::new("HW1", "Screws and heat-set inserts", "M2 screws, brass inserts", encl_sub)
                                .qty(4.0)
                                .cost(0.1)
                                .mass(0.5),
                        );
                    } else {
                        v.push(
                            Part::new("HW1", "Screws (M2, self-tapping)", "Steel screws", encl_sub)
                                .qty(4.0)
                                .cost(0.03)
                                .mass(0.25),
                        );
                    }
                }
            }
            _ => {}
        }
        if self.has("waterproof") && e.process != "none" {
            let perim = 2.0 * (self.tv("length_mm") + self.tv("width_mm"));
            v.push(
                Part::new("GSK1", "Silicone gasket", "Molded silicone O-ring/gasket", encl_sub)
                    .cost(0.3)
                    .mass(parts::round_to(perim * 2.0 * 2.0 * 1.1e-3, 2)),
            );
            v.push(
                Part::new("VNT1", "Pressure-equalising vent membrane", "ePTFE adhesive vent", encl_sub)
                    .cost(0.4)
                    .mass(0.1),
            );
        }
        match k {
            "wearable" => v.push(
                Part::new("STP1", "Strap or clip", "Silicone strap / spring clip", STR)
                    .cost(1.0)
                    .mass(6.0),
            ),
            "desk_instrument" => v.push(
                Part::new("FT1", "Rubber feet (set of 4)", "Adhesive bumpers", STR)
                    .cost(0.2)
                    .mass(2.0),
            ),
            "smart_home" => v.push(
                Part::new("MNT1", "Wall-mount plate", "Printed or molded plate with screws", STR)
                    .cost(0.3)
                    .mass(8.0),
            ),
            "dev_board" => v.push(
                Part::new("HDR1", "2.54 mm pin headers (2 × 20)", "Break-away male header", PCB)
                    .qty(2.0)
                    .cost(0.15)
                    .mass(1.0)
                    .area(129.0),
            ),
            _ => {}
        }
        v
    }

    /// The two-node QKD + QRNG testbed (values after the Q-Link template).
    fn quantum_parts(&self) -> Vec<Part> {
        let km = self.tv("fiber_km");
        let p = |mw: f64| [mw, mw, 1.0];
        let mk = |r: &str, n: &str, c: &str, s: &'static str, cost: f64, mass: f64, pw: [f64; 3]| {
            Part::new(r, n, c, s).cost(cost).mass(mass).pw(pw[0], pw[1], pw[2])
        };
        vec![
            mk(
                "QRNG1",
                "Diode-noise QRNG board",
                "Reverse-biased diode + amplifier + 8-bit ADC, hand-solderable",
                QRNG,
                60.0,
                40.0,
                p(600.0),
            ),
            mk(
                "LD1",
                "Pulsed 1550 nm DFB laser + driver",
                "Gain-switched distributed-feedback laser diode with pulse driver",
                QLINK,
                1500.0,
                250.0,
                p(2500.0),
            ),
            mk(
                "VOA1",
                "Variable optical attenuator",
                "Electronically controlled attenuator, 0–60 dB",
                QLINK,
                600.0,
                120.0,
                p(100.0),
            ),
            mk(
                "PM1",
                "Phase modulator + driver",
                "LiNbO3 phase modulator with RF driver",
                QLINK,
                2000.0,
                300.0,
                p(3000.0),
            ),
            mk(
                "INT1",
                "Alice unbalanced interferometer",
                "Fiber Mach–Zehnder with delay line, temperature-stabilised",
                QLINK,
                800.0,
                400.0,
                p(1000.0),
            ),
            mk(
                "FIB1",
                &format!("Fiber spool {} km", fmt(km)),
                "G.652 single-mode fiber on a spool",
                QLINK,
                48.0 * km,
                120.0 * km,
                [0.0, 0.0, 0.0],
            ),
            mk(
                "SPD1",
                "Gated InGaAs single-photon detector",
                "InGaAs/InP SPAD module with TEC cooling, gated",
                QLINK,
                12000.0,
                1500.0,
                p(12000.0),
            ),
            mk(
                "SPD2",
                "Gated InGaAs single-photon detector",
                "InGaAs/InP SPAD module with TEC cooling, gated",
                QLINK,
                12000.0,
                1500.0,
                p(12000.0),
            ),
            mk(
                "INT2",
                "Bob decoding interferometer",
                "Matched fiber Mach–Zehnder with phase tracking",
                QLINK,
                800.0,
                400.0,
                p(1000.0),
            ),
            mk(
                "FPGA1",
                "Alice FPGA timing board",
                "FPGA board with high-speed serial outputs",
                CMP,
                400.0,
                200.0,
                p(5000.0),
            ),
            mk(
                "FPGA2",
                "Bob FPGA time tagger",
                "FPGA time-to-digital converter board",
                CMP,
                400.0,
                200.0,
                p(5000.0),
            ),
            mk(
                "PC1",
                "Alice post-processing computer",
                "Small-form-factor PC",
                CMP,
                700.0,
                1200.0,
                p(25000.0),
            ),
            mk(
                "PC2",
                "Bob post-processing computer",
                "Small-form-factor PC",
                CMP,
                700.0,
                1200.0,
                p(25000.0),
            ),
            mk(
                "PS1",
                "Certified bench power supplies (pair)",
                "Low-voltage DC bench supplies",
                PWR,
                300.0,
                3000.0,
                [0.0, 0.0, 0.0],
            ),
            mk(
                "ENC1",
                "19-inch rack enclosure with laser interlock",
                "Aluminium sheet-metal enclosure, one per node",
                STR,
                self.enclosure.cost_usd,
                self.enclosure.mass_g,
                [0.0, 0.0, 0.0],
            )
            .qty(2.0),
        ]
    }

    /// The RF lab's recommended matching network for the assumed chip antenna.
    pub fn rf_network(&self) -> Vec<crate::rf::matching::Element> {
        let rf = crate::rf::IrisRf::default();
        crate::rf::matching::recommend(&rf.antenna, &rf.series, rf.q_l, rf.q_c)
    }

    // -----------------------------------------------------------------------
    // The sizing loop: power ↔ battery ↔ mass ↔ motors
    // -----------------------------------------------------------------------

    fn size(&mut self) {
        self.size_enclosure();
        let eff = self.regulator_eff();
        let iq_mw = self.quiescent_ua() * 1e-3 * if self.has("coin_cell") { 3.0 } else { 3.7 };
        let mut bat = self.battery_for(0.0);
        let mut motor = if self.is("robot") && self.has("motors") {
            self.motor_mw_for(self.tv("mass_g"))
        } else {
            0.0
        };
        let mut prev_mass = f64::NAN;
        for _ in 0..12 {
            let ps = self.build_parts(&bat, motor);
            let mass: f64 = ps.iter().map(|p| p.mass * p.qty).sum();
            let load: f64 = ps.iter().map(Part::avg_mw).sum();
            let avg = if load > 0.0 { load / eff + iq_mw } else { 0.0 };
            self.iterations.push((mass, avg, bat.mah));
            if self.is("robot") && self.has("motors") {
                motor = self.motor_mw_for(mass + self.pcb_mass_guess());
            }
            let next = self.battery_for(avg);
            let done = (mass - prev_mass).abs() < 0.01 && next.mah == bat.mah;
            bat = next;
            prev_mass = mass;
            if done {
                break;
            }
        }
        self.battery = bat;
        self.motor_mw = motor;
        self.parts = self.build_parts(&self.battery.clone(), motor);
        self.size_board();
        if self.is("robot") && self.has("motors") {
            // final pass with the real board mass in the total
            let mass: f64 = self.parts.iter().map(|p| p.mass * p.qty).sum();
            motor = self.motor_mw_for(mass);
            self.motor_mw = motor;
            for p in self.parts.iter_mut().filter(|p| p.refdes == "M1" || p.refdes == "M2") {
                p.power[0] = motor;
            }
        }
        let load: f64 = self.parts.iter().map(Part::avg_mw).sum();
        self.avg_mw = if load > 0.0 { load / eff + iq_mw } else { 0.0 };
        self.peak_mw = self.parts.iter().map(|p| p.qty * p.power[0]).sum::<f64>() / eff;
    }

    fn pcb_mass_guess(&self) -> f64 {
        if self.board.w > 0.0 {
            self.board.w * self.board.h * self.board.thickness * 1.85e-3 * 1.1
        } else {
            4.0
        }
    }

    /// Electrical power of one drive motor while driving, mW.
    pub fn motor_mw_for(&self, mass_g: f64) -> f64 {
        let (p0, crr, v, eta) = (300.0, 0.03, 0.3, 0.35);
        let m = mass_g / 1000.0;
        p0 + m * 9.81 * crr * v / (2.0 * eta) * 1000.0
    }

    // -----------------------------------------------------------------------
    // Board area
    // -----------------------------------------------------------------------

    fn size_board(&mut self) {
        if !self.electronics || self.is("quantum") {
            return;
        }
        let parts_mm2: f64 = self.parts.iter().filter(|p| p.board).map(|p| p.unit_area_mm2() * p.qty).sum();
        let four = self.has("wifi") || self.has("camera");
        let thickness = if matches!(self.kind.id, "wearable" | "iot_sensor") {
            0.8
        } else {
            1.6
        };
        let (l, w) = (self.tv("length_mm"), self.tv("width_mm"));
        let clearance = self.enclosure.wall_mm + 1.0;
        let inner = if self.is("dev_board") && self.enclosure.process == "none" {
            [l, w]
        } else {
            [(l - 2.0 * clearance).max(5.0), (w - 2.0 * clearance).max(5.0)]
        };
        let factor_for = |layers: u32| if layers >= 4 { 1.8 } else { 2.5 };
        let options: Vec<(u32, bool)> = [(2, false), (2, true), (4, false), (4, true)]
            .into_iter()
            .filter(|(l, _)| !four || *l == 4)
            .collect();
        let fits_in = |(l, two): (u32, bool)| parts_mm2 * factor_for(l) / if two { 2.0 } else { 1.0 } <= inner[0] * inner[1];
        let pick = options.iter().copied().find(|o| fits_in(*o)).unwrap_or(*options.last().unwrap());
        let (layers, double) = pick;
        let factor = factor_for(layers);
        let need = parts_mm2 * factor / if double { 2.0 } else { 1.0 };
        let fits = fits_in(pick);
        let (bw, bh) = if self.is("dev_board") && self.enclosure.process == "none" {
            (l, w)
        } else if fits {
            let mut bw = (need * inner[0] / inner[1]).sqrt().min(inner[0]);
            let mut bh = need / bw;
            if bh > inner[1] {
                bh = inner[1];
                bw = need / bh;
            }
            ((bw * 2.0).ceil() / 2.0, (bh * 2.0).ceil() / 2.0)
        } else {
            (inner[0], inner[1])
        };
        let (bw, bh) = (bw.max(10.0), bh.max(10.0));
        self.board = Board {
            parts_mm2,
            needed_mm2: need,
            factor,
            w: bw,
            h: bh,
            layers,
            thickness,
            fits,
            double_sided: double,
            inner,
        };
        let area_cm2 = bw * bh / 100.0;
        let cost = (0.3 + 0.03 * area_cm2) * if layers >= 4 { 1.8 } else { 1.0 };
        let mass = area_cm2 * thickness / 10.0 * 1.85 * 1.1;
        self.parts.push(
            Part::new(
                "PCB1",
                "Main board (bare)",
                &format!("{layers}-layer FR-4, {thickness} mm, {} × {} mm", fmt(bw), fmt(bh)),
                PCB,
            )
            .cost(cost)
            .mass(mass),
        );
        let n_board = self.parts.iter().filter(|p| p.board).count();
        let mut math = vec![
            format!(
                "Σ courtyard areas of {n_board} board lines (IPC-7351 courtyards from the PCB library, typical land areas for modules) = {} mm²",
                fmt(parts_mm2)
            ),
            format!(
                "Routing factor {} for a {layers}-layer board (typical 2–3× component area on 2 layers, less with inner ground/power planes) ⇒ {} mm²",
                factor,
                fmt(need)
            ),
        ];
        if self.is("dev_board") && self.enclosure.process == "none" {
            math.push(format!(
                "The board is the product: {} × {} mm = {} mm² available",
                fmt(l),
                fmt(w),
                fmt(l * w)
            ));
        } else {
            math.push(format!(
                "Inside the enclosure: ({} − 2 × {}) × ({} − 2 × {}) = {} × {} mm",
                fmt(l),
                fmt(clearance),
                fmt(w),
                fmt(clearance),
                fmt(inner[0]),
                fmt(inner[1])
            ));
            math.push(format!(
                "Board outline keeps the enclosure's aspect ratio: {} × {} mm",
                fmt(bw),
                fmt(bh)
            ));
        }
        if double {
            math.push(
                "Too big for one side: parts on both sides halves the outline (common for coin-cell holders and connectors on the back)"
                    .into(),
            );
        }
        if four {
            math.push("Wi-Fi/camera: 4 layers for a solid ground plane under the 2.4 GHz and high-speed signals (typical practice)".into());
        }
        let available = if self.is("dev_board") && self.enclosure.process == "none" {
            l * w
        } else {
            inner[0] * inner[1]
        };
        if need > available {
            self.warn(format!(
                "The parts need about {} mm² of board but only {} mm² fit: grow the envelope, place parts on both sides, or use smaller modules.",
                fmt(need),
                fmt(available)
            ));
        }
        self.explain(
            "board",
            "Why this board size?",
            format!(
                "The board must hold every part plus room for routing, and fit inside the {}.",
                if self.is("dev_board") && self.enclosure.process == "none" {
                    "board envelope"
                } else {
                    "enclosure walls"
                }
            ),
            math,
            format!(
                "{} × {} mm, {layers} layers{}, {thickness} mm thick",
                fmt(bw),
                fmt(bh),
                if double { ", parts on both sides" } else { "" }
            ),
        );
    }

    /// Battery and power explanations (after sizing).
    fn explain_power(&mut self) {
        if !self.electronics {
            return;
        }
        let eff = self.regulator_eff();
        let mut loads: Vec<(String, f64)> = self
            .parts
            .iter()
            .filter(|p| p.avg_mw() > 0.0)
            .map(|p| (format!("{} {}", p.refdes, p.name), p.avg_mw()))
            .collect();
        loads.sort_by(|a, b| b.1.total_cmp(&a.1));
        let load: f64 = loads.iter().map(|x| x.1).sum();
        let iq = self.quiescent_ua();
        let vq = if self.has("coin_cell") { 3.0 } else { 3.7 };
        let mut math: Vec<String> = loads
            .iter()
            .take(6)
            .map(|(n, w)| {
                let p = self.parts.iter().find(|p| n.starts_with(&format!("{} ", p.refdes))).unwrap();
                format!(
                    "{n}: {} × ({} mW × {} + {} mW × {}) = {} mW",
                    fmt(p.qty),
                    fmt(p.power[0]),
                    fmt(p.power[2]),
                    fmt(p.power[1]),
                    fmt(1.0 - p.power[2]),
                    fmt(*w)
                )
            })
            .collect();
        if loads.len() > 6 {
            let rest: f64 = loads.iter().skip(6).map(|x| x.1).sum();
            math.push(format!("{} smaller loads: {} mW", loads.len() - 6, fmt(rest)));
        }
        math.push(format!(
            "P_avg = Σ loads ÷ η + I_q V = {} ÷ {eff} + {iq} µA × {vq} V = {} mW",
            fmt(load),
            fmt(self.avg_mw)
        ));
        let top = loads.first().map(|x| x.0.clone()).unwrap_or_default();
        self.explain(
            "power",
            "Where the power goes",
            format!(
                "Each part's average is its active power × the fraction of time it is active, plus its sleep power the rest of the time. The biggest load is {top}."
            ),
            math,
            format!("{} mW average, {} mW peak (everything on)", fmt(self.avg_mw), fmt(self.peak_mw)),
        );
        let life = self.tv("life_h");
        match self.battery.source.clone() {
            Source::Lipo => {
                let b = self.battery.clone();
                let e_req = self.avg_mw * life * 1.2;
                let real = b.mah * b.params.nominal_v * b.params.usable_fraction / self.avg_mw;
                let (beta, eta) = self.cell_wear();
                let cyc_h = self.cycle_life_h();
                let hours = (life * 1.2).max(cyc_h);
                let mut math = vec![
                    format!("Energy per charge E = P_avg × t × 1.2 growth allowance = {} mW × {} h × 1.2 = {} mWh", fmt(self.avg_mw), fmt(life), fmt(e_req)),
                    format!(
                        "Cycle life: cell wear-out Weibull β = {}, η = {} cycles (Reliability Lab, typical) ⇒ B10 = η (−ln 0.9)^(1/β) = {} cycles",
                        fmt(beta),
                        fmt(eta),
                        fmt(self.cell_b10_cycles())
                    ),
                    format!(
                        "To outlast {} years at {} h/day: hours per charge ≥ {} × 365 × {} × 1.15 ÷ {} = {} h",
                        fmt(self.tv("design_years")),
                        fmt(self.tv("hours_per_day")),
                        fmt(self.tv("design_years")),
                        fmt(self.tv("hours_per_day")),
                        fmt(self.cell_b10_cycles()),
                        fmt(cyc_h)
                    ),
                    format!(
                        "Capacity C = P_avg × max({} h, {} h) ÷ (V_nom × usable fraction) = {} × {} ÷ (3.7 V × 0.85) = {} mAh",
                        fmt(life * 1.2),
                        fmt(cyc_h),
                        fmt(self.avg_mw),
                        fmt(hours),
                        fmt(b.required_mah)
                    ),
                    format!("Next common pouch size: {} mAh", fmt(b.mah)),
                    format!(
                        "Mass ≈ C × V ÷ energy density = {} mAh × 3.7 V ÷ {} mWh/g = {} g (typical small pouch)",
                        fmt(b.mah),
                        fmt(parts::lipo_mwh_per_g(b.mah)),
                        fmt(b.params.mass_g)
                    ),
                    format!("Predicted life = {} mAh × 3.7 V × 0.85 ÷ {} mW = {} h", fmt(b.mah), fmt(self.avg_mw), fmt(real)),
                ];
                if self.has("usb_c") {
                    let i = (0.5 * b.mah).min(500.0);
                    math.push(format!(
                        "Charging at min(0.5 C, 500 mA USB default) = {} mA ⇒ ≈ 1.2 × C ÷ I = {} h (constant-voltage taper ×1.2)",
                        fmt(i),
                        fmt(1.2 * b.mah / i)
                    ));
                }
                if self.iterations.len() > 2 && self.is("robot") {
                    let it = self
                        .iterations
                        .iter()
                        .map(|(m, p, c)| format!("{} g → {} mW → {} mAh", fmt(*m), fmt(*p), fmt(*c)))
                        .collect::<Vec<_>>();
                    math.push(format!("Mass ↔ power ↔ battery iterations: {}", it.join("; ")));
                }
                if b.required_mah > *parts::LIPO_SIZES.last().unwrap() {
                    self.warn(format!(
                        "The target life needs {} mAh, more than the largest cell the wizard places ({} mAh): lower the power or the life target.",
                        fmt(b.required_mah),
                        fmt(b.mah)
                    ));
                }
                self.explain(
                    "battery",
                    "Why this battery?",
                    format!(
                        "The battery must store the average power × the target life, plus a 20 % allowance because early power estimates grow — and be big enough that it is not recharged so often that it wears out before the design life ({}). Only about 85 % of a lithium cell's rated capacity is usable over its life.",
                        if cyc_h > life * 1.2 { "here cycle life sets the size" } else { "here the life target sets the size" }
                    ),
                    math,
                    format!("{} mAh LiPo, {} g, {} h predicted", fmt(b.mah), fmt(b.params.mass_g), fmt(real)),
                );
            }
            Source::Coin(i) => {
                let b = self.battery.clone();
                let real = b.mah * 3.0 * 0.85 / self.avg_mw;
                let c = &parts::COINS[i];
                let math = vec![
                    format!(
                        "Energy needed E = {} mW × {} h × 1.2 = {} mWh",
                        fmt(self.avg_mw),
                        fmt(life),
                        fmt(self.avg_mw * life * 1.2)
                    ),
                    format!("Capacity C = E ÷ (3.0 V × 0.85) = {} mAh", fmt(b.required_mah)),
                    format!(
                        "Coin-cell classes: CR2032 ≈ 225 mAh, CR2450 ≈ 620 mAh (typical) ⇒ {} ({} mAh, {} g)",
                        c.name,
                        fmt(c.mah),
                        fmt(c.mass_g)
                    ),
                    format!(
                        "Predicted life = {} mAh × 3.0 V × 0.85 ÷ {} mW = {} h ({} days)",
                        fmt(c.mah),
                        fmt(self.avg_mw),
                        fmt(real),
                        fmt(real / 24.0)
                    ),
                    format!(
                        "Peak power {} mW vs ≈ 45 mW a coin cell delivers in pulses with a bulk capacitor (15 mA × 3 V, typical)",
                        fmt(self.peak_mw)
                    ),
                ];
                if b.required_mah > c.mah {
                    self.warn(format!(
                        "Even a {} cannot reach {} h at {} mW: the battery-life requirement will fail. Lower the duty cycles, drop power-hungry capabilities or switch to a rechargeable battery.",
                        c.name,
                        fmt(life),
                        fmt(self.avg_mw)
                    ));
                }
                self.explain(
                    "battery",
                    "Why this coin cell?",
                    "A primary coin cell cannot be recharged, so its capacity alone sets the life. The smallest common size that meets the target with a 20 % allowance wins.".into(),
                    math,
                    format!("{} {} mAh, {} h predicted", c.name, fmt(c.mah), fmt(real)),
                );
            }
            Source::None => {
                let lim = self.peak_limit_mw();
                let src = if self.is("quantum") {
                    "the lab's certified bench supplies (one 1.5 kW lab circuit with headroom)"
                } else if self.has("dc_adapter") {
                    "a 12 V 1 A adapter derated to 80 %"
                } else {
                    "USB's default 5 V × 500 mA budget"
                };
                self.explain(
                    "battery",
                    "Power supply",
                    format!(
                        "No battery: the product runs from {src}. The design limit is on peak power, the moment every load is on at once."
                    ),
                    vec![format!(
                        "Peak power = Σ active power ÷ η = {} mW, limit {} mW",
                        fmt(self.peak_mw),
                        fmt(lim)
                    )],
                    format!("{} mW peak of {} mW available", fmt(self.peak_mw), fmt(lim)),
                );
            }
        }
        if self.is("robot") && self.has("motors") {
            let mass: f64 = self.parts.iter().map(|p| p.mass * p.qty).sum();
            let m = mass / 1000.0;
            let torque = (m * 9.81 * 0.03 + m * 0.5) * 0.02 / 2.0;
            let math = vec![
                format!(
                    "Rolling resistance F = m g C_rr = {} kg × 9.81 × 0.03 = {} N",
                    fmt(m),
                    fmt(m * 9.81 * 0.03)
                ),
                format!(
                    "Mechanical power at v = 0.3 m/s: P = F v = {} W, shared by 2 motors",
                    fmt(m * 9.81 * 0.03 * 0.3)
                ),
                format!(
                    "Electrical per motor = P₀ + P/(2 η) = 0.3 W no-load + {} W ÷ (2 × 0.35) = {} mW (typical N20-class gear motor)",
                    fmt(m * 9.81 * 0.03 * 0.3),
                    fmt(self.motor_mw)
                ),
                format!(
                    "Wheel torque to accelerate at 0.5 m/s² with 20 mm wheels: T = (m g C_rr + m a) r / 2 = {} N·m per motor",
                    fmt(torque)
                ),
            ];
            if torque > 0.15 {
                self.warn(format!(
                    "Each drive motor needs {} N·m — above what small N20-class gear motors typically deliver; choose a larger gear motor.",
                    fmt(torque)
                ));
            }
            self.explain(
                "motors",
                "Motor power for this mass",
                "A heavier robot needs more motor power, which needs a bigger battery, which adds mass. The wizard repeats the calculation until the mass stops changing.".into(),
                math,
                format!("{} mW per motor while driving, {} g total", fmt(self.motor_mw), fmt(mass)),
            );
        }
    }

    /// Peak-power limit of the energy source, mW.
    pub fn peak_limit_mw(&self) -> f64 {
        match self.battery.source {
            Source::Lipo => 2.0 * self.battery.mah * 3.7,
            Source::Coin(_) => 45.0,
            Source::None => {
                if self.is("quantum") {
                    150_000.0
                } else if self.has("dc_adapter") {
                    9600.0
                } else {
                    2500.0
                }
            }
        }
    }
}

/// (unit cost, tooling) of an enclosure process for a part of the given volume/mass.
pub fn process_cost(process: &str, volume_cm3: f64, mass_g: f64, longest_mm: f64, _qty: f64) -> (f64, f64) {
    match process {
        "printed" => {
            let hours = volume_cm3 / 12.0 + 0.3;
            (mass_g * 1.15 * 0.025 + hours * 1.5, 0.0)
        }
        "molded" => {
            let piece = mass_g * 0.0035 * 1.1 + 2.0 * 30.0 / 3600.0 * 40.0;
            (piece, 2.0 * (5000.0 + 40.0 * longest_mm))
        }
        "rack" => (600.0, 0.0),
        _ => (0.0, 0.0),
    }
}

/// Answers → starter project with explanations.
pub fn generate(a: &Answers) -> Result<WizardOut, String> {
    let mut c = Ctx::new(a)?;
    c.size();
    c.explain_power();
    let project = c.assemble();
    let labs = c.finish(&project);
    let summary = c.summary(&project);
    Ok(WizardOut {
        capabilities: c.caps.iter().map(|s| s.to_string()).collect(),
        targets: c.t.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        project,
        explanations: c.explanations,
        warnings: c.warnings,
        labs,
        summary,
    })
}

pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "systems.wizard_options" => Some(Ok(catalog::options())),
        "systems.wizard" => handle(input, |a: Answers| generate(&a)),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
