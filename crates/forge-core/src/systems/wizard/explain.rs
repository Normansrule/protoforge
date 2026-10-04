//! Explanations that need the engine's own analyses of the generated project
//! (thermal network, RF link, reliability, volume prices), the honest
//! lab-applicability notes, and the summary.

use super::{fmt, Ctx, LabNote, Source, Summary};
use crate::project::Project;

impl Ctx {
    /// Add the analysis-backed explanations and warnings; return the lab notes.
    pub fn finish(&mut self, p: &Project) -> Vec<LabNote> {
        self.explain_mass_cost(p);
        if self.electronics {
            self.explain_thermal(p);
            self.explain_reliability(p);
        }
        if self.has("ble") {
            self.explain_radio(p);
        }
        if self.has("microphones") {
            let s = p.design.audio.mic_spacing_mm;
            self.explain(
                "audio",
                "Microphone spacing",
                "Two microphones a little apart can subtract sound arriving from behind (a hypercardioid pair). Wider spacing improves low-frequency sensitivity; too wide and high frequencies alias.".into(),
                vec![
                    format!("Spacing d = 0.4 × envelope length, clamped to 8–40 mm = {} mm", fmt(s)),
                    format!("Spatial aliasing above f = c / (2d) = 343 m/s ÷ (2 × {} mm) = {} kHz", fmt(s), fmt(343.0 / (2.0 * s * 1e-3) / 1000.0)),
                ],
                format!("{} mm pair; the DSP Lab computes the directivity index", fmt(s)),
            );
        }
        if self.is("quantum") {
            let km = self.tv("fiber_km");
            let loss = 0.2 * km + 3.0;
            let m = crate::quantum::metrics(p);
            let get = |k: &str| m.iter().find(|x| x.key == k).map(|x| x.value);
            let rate = get("quantum.qkd_key_rate_bps").unwrap_or(0.0);
            self.explain(
                "quantum",
                "Quantum link budget",
                "Every kilometre of fiber absorbs about 4.5 % of the photons; the key rate falls with the transmission and collapses when detector noise catches up with the signal.".into(),
                vec![
                    format!("Channel loss = 0.2 dB/km × {} km + 3 dB (connectors, receiver) = {} dB", fmt(km), fmt(loss)),
                    format!("Transmission η_ch = 10^(−{}/10) = {}", fmt(loss), fmt(10f64.powf(-loss / 10.0))),
                    format!("Decoy-state BB84 key rate (Quantum Lab) = {} bit/s", fmt(rate)),
                    format!("QBER = {} %, maximum distance = {} km", fmt(get("quantum.qkd_qber_pct").unwrap_or(0.0)), fmt(get("quantum.qkd_max_distance_km").unwrap_or(0.0))),
                ],
                format!("{} bit/s of secret key at {} km", fmt(rate), fmt(km)),
            );
            if rate < 1000.0 {
                self.warn(format!(
                    "At {} km the key rate is {} bit/s, below the 1000 bit/s requirement: shorten the link or plan for higher-efficiency detectors (trade T-1).",
                    fmt(km),
                    fmt(rate)
                ));
            }
        }
        self.lab_notes()
    }

    fn explain_mass_cost(&mut self, p: &Project) {
        let mut groups: Vec<(String, f64)> = vec![];
        for s in &p.subsystems {
            let m: f64 = p.components.iter().filter(|c| c.subsystem == s.id).map(|c| c.mass_g * c.qty).sum();
            if m > 0.0 {
                groups.push((s.name.clone(), m));
            }
        }
        groups.sort_by(|a, b| b.1.total_cmp(&a.1));
        let mass = p.component_mass_g();
        let target = self.tv("mass_g");
        let mut math: Vec<String> = groups.iter().map(|(n, m)| format!("{n}: {} g", fmt(*m))).collect();
        math.push(format!(
            "Total = {} g vs {} g target ⇒ margin {} %",
            fmt(mass),
            fmt(target),
            fmt(100.0 * (target - mass) / target)
        ));
        self.explain(
            "mass",
            "Mass budget",
            "The mass is the sum of every part × its quantity, grouped by subsystem.".into(),
            math,
            format!("{} g of {} g", fmt(mass), fmt(target)),
        );
        if mass > target {
            let top = groups.first().map(|g| g.0.clone()).unwrap_or_default();
            self.warn(format!(
                "The parts weigh {} g, over the {} g target — the heaviest subsystem is {top}.",
                fmt(mass),
                fmt(target)
            ));
        }
        // volume-priced BOM, same learning curve as the Manufacturing Hub
        let q = self.tv("quantity");
        let a = crate::mfg::Assumptions::default();
        let qty1 = p.bom_cost_usd();
        let vol: f64 = p
            .components
            .iter()
            .filter(|c| !c.refdes.starts_with("PCB"))
            .map(|c| crate::mfg::price_at(c.unit_cost_usd, q * c.qty, &a) * c.qty)
            .sum();
        let mut lines: Vec<(String, f64)> = p
            .components
            .iter()
            .map(|c| (format!("{} {}", c.refdes, c.name), c.unit_cost_usd * c.qty))
            .collect();
        lines.sort_by(|x, y| y.1.total_cmp(&x.1));
        let mut math: Vec<String> = lines.iter().take(4).map(|(n, c)| format!("{n}: {} USD", fmt(*c))).collect();
        math.push(format!("Quantity-1 BOM = {} USD ({} lines)", fmt(qty1), p.components.len()));
        math.push(format!(
            "Volume price p(q) = p₁ (f + (1 − f) q^−b), f = {}, b = {} (Manufacturing Hub learning curve) at q = {} ⇒ {} USD (bare board excluded: the Hub prices it from the outline)",
            a.bom_floor_fraction,
            a.bom_exponent,
            fmt(q),
            fmt(vol)
        ));
        let retail = self.tv("retail_usd");
        math.push(format!(
            "Parts ÷ retail = {} ÷ {} = {} %",
            fmt(vol),
            fmt(retail),
            fmt(100.0 * vol / retail)
        ));
        self.explain(
            "cost",
            "Cost: prototype vs volume",
            "Prices fall with quantity. The first number is what one prototype's parts cost; the second is the parts cost per unit at your production quantity.".into(),
            math,
            format!("{} USD at qty 1, {} USD at {} units", fmt(qty1), fmt(vol), fmt(q)),
        );
        if vol > self.tv("cost_usd") {
            self.warn(format!(
                "Parts at {} units cost {} USD, over the {} USD target: the cost requirement will fail until parts get cheaper or the target moves.",
                fmt(q),
                fmt(vol),
                fmt(self.tv("cost_usd"))
            ));
        }
        if vol > retail / 3.0 && !self.is("quantum") {
            self.info(format!(
                "Parts are {} % of the retail price; consumer hardware usually needs parts near 20–30 % to cover assembly, channel margin and returns.",
                fmt(100.0 * vol / retail)
            ));
        }
    }

    fn explain_thermal(&mut self, p: &Project) {
        let th = &p.design.thermal;
        let (skin, board, heat) = crate::power::thermal::worst_case(p);
        let skin_contact = self.kind.contact == super::catalog::Contact::Skin;
        let area = if self.enclosure.process == "none" {
            2.0 * self.board.w * self.board.h
        } else {
            self.enclosure.surface_mm2
        };
        let math = vec![
            format!(
                "Exposed area A = {} mm²{} = {} m²",
                fmt(area),
                if skin_contact { " × 0.7 (rest against skin or strap)" } else { "" },
                fmt(area * 1e-6 * if skin_contact { 0.7 } else { 1.0 })
            ),
            format!(
                "Shell to air R = 1 / (h A), h ≈ 10 W/m²K (natural convection ≈ 5 + radiation ≈ 5, typical) ⇒ {} °C/W",
                fmt(th.r_pod_ambient_c_per_w)
            ),
            if self.is("quantum") {
                format!(
                    "Modules to enclosure air R = {} °C/W (fan-cooled rack, typical)",
                    fmt(th.r_board_pod_c_per_w)
                )
            } else if self.enclosure.process == "none" {
                format!("Parts to board R = {} °C/W (bare board, typical)", fmt(th.r_board_pod_c_per_w))
            } else {
                format!(
                    "Board to shell R ≈ 20 °C/W × √(20 cm² / board area) = {} °C/W (typical spreading and contact)",
                    fmt(th.r_board_pod_c_per_w)
                )
            },
            format!("Heat with the display always on = {} mW", fmt(heat)),
            format!(
                "Surface = {} °C + {} W × {} °C/W = {} °C (limit {} °C)",
                fmt(th.ambient_c),
                fmt(heat / 1000.0),
                fmt(th.r_pod_ambient_c_per_w),
                fmt(skin),
                fmt(th.skin_limit_c)
            ),
            format!("Board = {} °C", fmt(board)),
        ];
        self.explain(
            "thermal",
            "How hot does it get?",
            "Every milliwatt ends up as heat that must leave through the enclosure's surface. A small case has a high thermal resistance to the air, so it warms up more per watt.".into(),
            math,
            format!("{} °C surface at {} °C ambient", fmt(skin), fmt(th.ambient_c)),
        );
        if skin > th.skin_limit_c {
            self.warn(format!(
                "Predicted surface temperature {} °C exceeds the {} °C limit: lower the peak duty, spread heat to a bigger surface or enlarge the enclosure.",
                fmt(skin),
                fmt(th.skin_limit_c)
            ));
        }
    }

    fn explain_reliability(&mut self, p: &Project) {
        let m = crate::reliability::metrics(p);
        let get = |k: &str| m.iter().find(|x| x.key == k);
        let (Some(mtbf), Some(b10)) = (get("reliability.mtbf_h"), get("reliability.b10_years")) else {
            return;
        };
        let years = self.tv("design_years");
        let hours = years * 365.0 * self.tv("hours_per_day");
        let need = hours / -(0.9f64.ln());
        let math = vec![
            format!(
                "Design life t = {} years × 365 × {} h/day = {} operating hours",
                fmt(years),
                fmt(self.tv("hours_per_day")),
                fmt(hours)
            ),
            format!("R(t) = exp(−t / MTBF) ≥ 0.9 ⇒ MTBF ≥ t ÷ 0.105 = {} h", fmt(need)),
            format!(
                "Parts-count MTBF (Reliability Lab, typical FIT at the board temperature) = {} h",
                fmt(mtbf.value)
            ),
            format!("B10 life including wear-out = {} years", fmt(b10.value)),
            mtbf.note.clone(),
        ];
        self.explain(
            "reliability",
            "Will it last?",
            "Random failures are summed from typical failure rates of each part class (failures in time, FIT = failures per 10⁹ hours). Wear-out items such as a rechargeable cell add a second, ageing failure mode.".into(),
            math,
            format!("MTBF {} h, B10 {} years", fmt(mtbf.value), fmt(b10.value)),
        );
        if b10.value < years {
            let why = if self.has("battery") {
                format!(
                    " The rechargeable cell is charged {} times a day; a bigger cell or a replaceable one stretches it.",
                    fmt(self.tv("hours_per_day") / (self.battery.mah * 3.7 * 0.85 / self.avg_mw.max(1e-9)).max(1e-9))
                )
            } else {
                String::new()
            };
            self.warn(format!(
                "B10 life is {} years, short of the {} year design life — the B10 requirement will fail.{why}",
                fmt(b10.value),
                fmt(years)
            ));
        }
        if mtbf.value < need {
            self.warn(format!(
                "Parts-count MTBF {} h is below the {} h target.",
                fmt(mtbf.value),
                fmt(super::parts::nice_up(need))
            ));
        }
    }

    fn explain_radio(&mut self, p: &Project) {
        let Ok(o) = crate::rf::iris(p) else { return };
        let get = |k: &str| o.metrics.iter().find(|x| x.key == k).map(|x| x.value);
        let d = self.tv("range_m");
        let fspl1 = 20.0 * (4.0 * std::f64::consts::PI * 1.0 / (0.299792458 / 2.44)).log10();
        let rf = crate::rf::IrisRf::from_project(p);
        let margin = get("rf.link_margin_db").unwrap_or(f64::NAN);
        let math = vec![
            format!("Free-space loss at 1 m, 2.44 GHz: 20 log₁₀(4π · 1 m / λ) = {} dB", fmt(fspl1)),
            format!(
                "Indoors, n = {}: PL({} m) = {} + 10 · {} · log₁₀({}) = {} dB",
                rf.link.path_exponent,
                fmt(d),
                fmt(fspl1),
                rf.link.path_exponent,
                fmt(d),
                fmt(fspl1 + 10.0 * rf.link.path_exponent * d.log10())
            ),
            format!(
                "Antenna: radiation efficiency {} % (typical chip antenna on a {} board), matched with the RF lab's {} network",
                fmt(rf.rad_efficiency * 100.0),
                if rf.rad_efficiency >= 0.5 { "roomy" } else { "small" },
                o.recommended_labels.join(" + ")
            ),
            format!(
                "Losses: head {} dB, body {} dB, fading allowance for 90 % availability ≈ 10 dB",
                fmt(rf.link.head_loss_db),
                fmt(rf.link.body_loss_db)
            ),
            format!(
                "Link margin (RF & Antenna Lab) = {} dB; maximum range = {} m",
                fmt(margin),
                fmt(get("rf.range_m").unwrap_or(f64::NAN))
            ),
        ];
        self.explain(
            "radio",
            "Will the Bluetooth link reach?",
            "Radio power falls with distance; the link works while the received power stays above the phone's sensitivity plus a fading allowance.".into(),
            math,
            format!("{} dB margin at {} m", fmt(margin), fmt(d)),
        );
        if margin < 3.0 {
            self.warn(format!(
                "BLE margin at {} m is {} dB, under the 3 dB requirement: shorten the range target, raise transmit power or improve the antenna (RF & Antenna Lab).",
                fmt(d),
                fmt(margin)
            ));
        }
    }

    /// Which labs analyse this product's own data.
    fn lab_notes(&self) -> Vec<LabNote> {
        let note = |lab: &str, status: &str, note: String| LabNote {
            lab: lab.into(),
            status: status.into(),
            note,
        };
        let mut v = vec![];
        if self.electronics {
            v.push(note(
                "power",
                "yours",
                "Power budget, battery life, peak power and the lumped thermal model use this project's parts and enclosure.".into(),
            ));
            v.push(note(
                "reliability",
                "yours",
                "Parts-count mean time between failures (MTBF), B10 life and the design-validation test (DVT) plan from this bill of materials (BOM) and use profile (typical failure rates).".into(),
            ));
        }
        v.push(note(
            "systems",
            "yours",
            "Mass, power and cost budgets, traceability, the failure mode and effects analysis (FMEA) and the trade study.".into(),
        ));
        v.push(note(
            "mfg",
            "partial",
            "Parts cost at volume is yours; the unit cost adds the Iris frame's printing/molding cost.".into(),
        ));
        v.push(note(
            "program",
            "partial",
            "Schedule is the standard hardware task network; economics use your retail price but include the Iris frame cost.".into(),
        ));
        v.push(note(
            "build",
            "partial",
            "Compliance plan and process settings; counts follow the BOM's radios and batteries.".into(),
        ));
        if self.has("microphones") {
            v.push(note(
                "dsp",
                "yours",
                "Directivity and decimation from this project's microphone spacing and audio chain.".into(),
            ));
        }
        if self.has("battery") {
            v.push(note(
                "circuit",
                "partial",
                "Buck ripple uses your battery voltage, rail and L1/C2 values; the other presets are Iris circuits.".into(),
            ));
        }
        if self.has("ble") {
            v.push(note("rf", "yours", "Link budget from this project's range, losses, antenna efficiency and matching network (antenna impedance is an assumed chip-antenna model).".into()));
        }
        if self.is("quantum") {
            v.push(note(
                "quantum",
                "yours",
                "Key rate, quantum bit error rate (QBER) and quantum random number generator (QRNG) entropy from this project's link parameters.".into(),
            ));
        }
        if self.has("imu") {
            v.push(note(
                "control",
                "partial",
                "Uses your inertial measurement unit (IMU) parameters but scores the Iris head-motion scenario.".into(),
            ));
        }
        v.push(note(
            "cad",
            "iris",
            "CAD Studio, Structures, Thermal & Fluids, Mechanisms, Human Factors, Molding and Print analyse the Iris glasses frame — model your own part in your CAD tool and use those labs to learn the methods.".into(),
        ));
        v.push(note("optics", "iris", "The Optics Lab ray-traces the Iris head-up display.".into()));
        v.push(note(
            "pcb",
            "iris",
            "Board parts start off the printed circuit board (PCB); PCB Studio places them on your outline but nets come from the Iris schematic table.".into(),
        ));
        v.push(note(
            "proto",
            "iris",
            "Firmware metrics and the Chip Lab describe the Iris firmware and filter.".into(),
        ));
        v
    }

    pub fn summary(&self, p: &Project) -> Summary {
        let bound = p.requirements.iter().filter(|r| r.verification.metric.is_some()).count();
        let surface = if self.electronics {
            Some(crate::power::thermal::worst_case(p).0)
        } else {
            None
        };
        let q = self.tv("quantity");
        let a = crate::mfg::Assumptions::default();
        let vol: f64 = p
            .components
            .iter()
            .filter(|c| !c.refdes.starts_with("PCB"))
            .map(|c| crate::mfg::price_at(c.unit_cost_usd, q * c.qty, &a) * c.qty)
            .sum();
        Summary {
            mass_g: p.component_mass_g(),
            avg_power_mw: self.avg_mw,
            peak_power_mw: self.peak_mw,
            battery: match self.battery.source {
                Source::None => p.design.battery.chemistry.clone(),
                _ => self.battery.name.clone(),
            },
            battery_mah: self.battery.mah,
            life_h: if self.has_battery() && self.avg_mw > 0.0 {
                Some(self.battery.mah * self.battery.params.nominal_v * self.battery.params.usable_fraction / self.avg_mw)
            } else {
                None
            },
            bom_usd: p.bom_cost_usd(),
            bom_volume_usd: vol,
            board_mm: [self.board.w, self.board.h],
            board_layers: self.board.layers,
            enclosure: self.enclosure.process.clone(),
            surface_c: surface,
            surface_limit_c: p.design.thermal.skin_limit_c,
            bound_requirements: bound,
            test_requirements: p.requirements.len() - bound,
        }
    }
}
