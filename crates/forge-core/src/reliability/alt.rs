//! Accelerated life testing (ALT): acceleration factors, a zero/few-failure
//! test planner and climatic-chamber profiles.
//!
//! * **Arrhenius** (temperature): `AF = exp[(Ea/k)(1/T_use − 1/T_test)]`,
//!   k = 8.617 333 262 × 10⁻⁵ eV/K, temperatures in kelvin.
//! * **Peck** (temperature–humidity): `AF = (RH_test/RH_use)^n · Arrhenius`,
//!   Peck's fit gives n ≈ 2.66 and Ea ≈ 0.79 eV for epoxy-packaged
//!   aluminium corrosion (D. S. Peck, IRPS 1986) — typical, check the mechanism.
//! * **Coffin–Manson** (thermal cycling): `AF = (ΔT_test/ΔT_use)^m`
//!   (ductile solder fatigue m ≈ 2–3; brittle cracking much higher).
//! * **Norris–Landzberg** (solder thermal cycling):
//!   `AF = (ΔT_t/ΔT_u)^n · (f_u/f_t)^p · exp[C (1/T_max,u − 1/T_max,t)]`;
//!   tin–lead constants n = 1.9, p = 1/3, C = 1414 K (Norris & Landzberg,
//!   IBM J. Res. Dev. 1969); tin–silver–copper constants n = 2.65, p = 0.136,
//!   C = 2185 K are commonly quoted from Pan et al. (2005) — check the source.
//! * **Inverse power law**: `AF = (S_test/S_use)^n` for non-thermal stresses
//!   (voltage, vibration level …).
//!
//! **Planner.** With a known Weibull shape β, n units tested for an
//! equivalent k = t·AF/L field lives with at most r failures demonstrate
//! reliability R at life L with confidence C when
//! `n · k^β ≥ χ²(C; 2r + 2) / (2 (−ln R))` — the Weibayes/Lipson relation
//! (Lipson & Sheth; Kececioglu; Abernethy ch. 6). For r = 0 this is
//! `n = ln(1 − C) / (k^β ln R)`.
//!
//! References: W. Nelson, *Accelerated Testing*, Wiley 1990; JEDEC JEP122
//! (failure mechanisms and models — check the current revision); O'Connor &
//! Kleyner, *Practical Reliability Engineering*, 5th ed., ch. 14.

use serde::{Deserialize, Serialize};

use super::stats;

/// Boltzmann constant, eV/K.
pub const K_EV: f64 = 8.617_333_262e-5;

fn kelvin(c: f64) -> f64 {
    c + 273.15
}

pub fn arrhenius(ea_ev: f64, t_use_c: f64, t_test_c: f64) -> f64 {
    (ea_ev / K_EV * (1.0 / kelvin(t_use_c) - 1.0 / kelvin(t_test_c))).exp()
}

pub fn peck(ea_ev: f64, n: f64, t_use_c: f64, rh_use: f64, t_test_c: f64, rh_test: f64) -> f64 {
    (rh_test / rh_use).powf(n) * arrhenius(ea_ev, t_use_c, t_test_c)
}

pub fn coffin_manson(dt_use: f64, dt_test: f64, m: f64) -> f64 {
    (dt_test / dt_use).powf(m)
}

#[allow(clippy::too_many_arguments)]
pub fn norris_landzberg(
    dt_use: f64,
    dt_test: f64,
    n: f64,
    f_use: f64,
    f_test: f64,
    p: f64,
    c_k: f64,
    tmax_use_c: f64,
    tmax_test_c: f64,
) -> f64 {
    coffin_manson(dt_use, dt_test, n) * (f_use / f_test).powf(p) * (c_k * (1.0 / kelvin(tmax_use_c) - 1.0 / kelvin(tmax_test_c))).exp()
}

pub fn inverse_power(s_use: f64, s_test: f64, n: f64) -> f64 {
    (s_test / s_use).powf(n)
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct AltIn {
    /// `arrhenius` | `peck` | `coffin_manson` | `norris_landzberg` | `ipl`
    pub model: String,
    pub ea_ev: f64,
    pub t_use_c: f64,
    pub t_test_c: f64,
    pub rh_use: f64,
    pub rh_test: f64,
    pub rh_exp: f64,
    pub dt_use: f64,
    pub dt_test: f64,
    pub cm_exp: f64,
    /// Thermal cycles per day in use and in test.
    pub f_use: f64,
    pub f_test: f64,
    pub freq_exp: f64,
    /// Norris–Landzberg Arrhenius constant C = Ea/k, K.
    pub nl_c: f64,
    pub tmax_use_c: f64,
    pub tmax_test_c: f64,
    pub s_use: f64,
    pub s_test: f64,
    pub ipl_exp: f64,
}

impl Default for AltIn {
    fn default() -> Self {
        AltIn {
            model: "arrhenius".into(),
            ea_ev: 0.7,
            t_use_c: 35.0,
            t_test_c: 85.0,
            rh_use: 60.0,
            rh_test: 85.0,
            rh_exp: 2.66,
            dt_use: 20.0,
            dt_test: 80.0,
            cm_exp: 2.0,
            f_use: 1.0,
            f_test: 16.0,
            freq_exp: 0.136,
            nl_c: 2185.0,
            tmax_use_c: 35.0,
            tmax_test_c: 60.0,
            s_use: 1.0,
            s_test: 2.0,
            ipl_exp: 6.4,
        }
    }
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Factor {
    pub name: String,
    pub value: f64,
    pub formula: String,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct AltOut {
    pub model: String,
    pub af: f64,
    pub factors: Vec<Factor>,
    /// AF against the swept test stress (x in the model's stress unit).
    pub sweep_x: Vec<f64>,
    pub sweep_af: Vec<f64>,
    pub sweep_label: String,
    pub equivalence: String,
    pub notes: Vec<String>,
}

fn pos(v: f64, name: &str) -> Result<f64, String> {
    if v > 0.0 && v.is_finite() {
        Ok(v)
    } else {
        Err(format!("{name} must be positive"))
    }
}

/// Acceleration factor with its factor breakdown and a sweep over the test stress.
pub fn acceleration(i: AltIn) -> Result<AltOut, String> {
    let mut o = AltOut {
        model: i.model.clone(),
        ..Default::default()
    };
    let ar = |tu: f64, tt: f64| arrhenius(i.ea_ev, tu, tt);
    match i.model.as_str() {
        "arrhenius" => {
            if i.t_use_c <= -273.0 || i.t_test_c <= -273.0 {
                return Err("temperatures must be above absolute zero".into());
            }
            o.af = ar(i.t_use_c, i.t_test_c);
            o.factors.push(Factor {
                name: "Temperature (Arrhenius)".into(),
                value: o.af,
                formula: format!(
                    "exp[{:.2} eV / k · (1/{:.1} K − 1/{:.1} K)]",
                    i.ea_ev,
                    kelvin(i.t_use_c),
                    kelvin(i.t_test_c)
                ),
            });
            o.sweep_label = "test temperature, °C".into();
            for k in 0..=60 {
                let t = i.t_use_c + (150.0 - i.t_use_c) * k as f64 / 60.0;
                o.sweep_x.push(t);
                o.sweep_af.push(ar(i.t_use_c, t));
            }
            o.equivalence = format!("1 h at {:.0} °C ≈ {:.1} h at {:.0} °C", i.t_test_c, o.af, i.t_use_c);
            o.notes.push("Ea depends on the failure mechanism (typical 0.3–1.1 eV); 0.7 eV is a common default for semiconductor mechanisms when nothing better is known — check JEDEC JEP122 for your mechanism.".into());
        }
        "peck" => {
            pos(i.rh_use, "use humidity")?;
            pos(i.rh_test, "test humidity")?;
            let h = (i.rh_test / i.rh_use).powf(i.rh_exp);
            let t = ar(i.t_use_c, i.t_test_c);
            o.af = h * t;
            o.factors.push(Factor {
                name: "Humidity".into(),
                value: h,
                formula: format!("({:.0} % / {:.0} %)^{:.2}", i.rh_test, i.rh_use, i.rh_exp),
            });
            o.factors.push(Factor {
                name: "Temperature (Arrhenius)".into(),
                value: t,
                formula: format!(
                    "exp[{:.2} eV / k · (1/{:.1} K − 1/{:.1} K)]",
                    i.ea_ev,
                    kelvin(i.t_use_c),
                    kelvin(i.t_test_c)
                ),
            });
            o.sweep_label = "test relative humidity, % (at the test temperature)".into();
            for k in 0..=60 {
                let rh = i.rh_use + (98.0 - i.rh_use).max(1.0) * k as f64 / 60.0;
                o.sweep_x.push(rh);
                o.sweep_af.push((rh / i.rh_use).powf(i.rh_exp) * t);
            }
            o.equivalence = format!(
                "1 h at {:.0} °C / {:.0} % RH ≈ {:.0} h at {:.0} °C / {:.0} % RH",
                i.t_test_c, i.rh_test, o.af, i.t_use_c, i.rh_use
            );
            o.notes.push("Peck's model was fitted to corrosion of plastic-encapsulated parts (n ≈ 2.66, Ea ≈ 0.79 eV). A whole product with a lithium cell cannot go to 85 °C: test the assembly at a milder damp-heat severity and parts separately.".into());
        }
        "coffin_manson" => {
            pos(i.dt_use, "use ΔT")?;
            pos(i.dt_test, "test ΔT")?;
            o.af = coffin_manson(i.dt_use, i.dt_test, i.cm_exp);
            o.factors.push(Factor {
                name: "Temperature swing".into(),
                value: o.af,
                formula: format!("({:.0} K / {:.0} K)^{:.2}", i.dt_test, i.dt_use, i.cm_exp),
            });
            o.sweep_label = "test temperature swing ΔT, K".into();
            for k in 0..=60 {
                let dt = i.dt_use + (165.0 - i.dt_use).max(1.0) * k as f64 / 60.0;
                o.sweep_x.push(dt);
                o.sweep_af.push(coffin_manson(i.dt_use, dt, i.cm_exp));
            }
            o.equivalence = format!(
                "1 test cycle of ΔT {:.0} K ≈ {:.1} field cycles of ΔT {:.0} K",
                i.dt_test, o.af, i.dt_use
            );
            o.notes.push("Typical exponents: ductile solder fatigue ≈ 2–3, hard alloys and intermetallics higher, brittle cracking much higher (see JEDEC JEP122 — check the current revision).".into());
        }
        "norris_landzberg" => {
            pos(i.dt_use, "use ΔT")?;
            pos(i.dt_test, "test ΔT")?;
            pos(i.f_use, "use cycles per day")?;
            pos(i.f_test, "test cycles per day")?;
            let a = coffin_manson(i.dt_use, i.dt_test, i.cm_exp);
            let b = (i.f_use / i.f_test).powf(i.freq_exp);
            let c = (i.nl_c * (1.0 / kelvin(i.tmax_use_c) - 1.0 / kelvin(i.tmax_test_c))).exp();
            o.af = a * b * c;
            o.factors.push(Factor {
                name: "Temperature swing".into(),
                value: a,
                formula: format!("({:.0} / {:.0})^{:.2}", i.dt_test, i.dt_use, i.cm_exp),
            });
            o.factors.push(Factor {
                name: "Cycling frequency".into(),
                value: b,
                formula: format!("({:.2} / {:.1} per day)^{:.3}", i.f_use, i.f_test, i.freq_exp),
            });
            o.factors.push(Factor {
                name: "Peak temperature".into(),
                value: c,
                formula: format!(
                    "exp[{:.0} K · (1/{:.1} − 1/{:.1})]",
                    i.nl_c,
                    kelvin(i.tmax_use_c),
                    kelvin(i.tmax_test_c)
                ),
            });
            o.sweep_label = "test temperature swing ΔT, K".into();
            for k in 0..=60 {
                let dt = i.dt_use + (165.0 - i.dt_use).max(1.0) * k as f64 / 60.0;
                o.sweep_x.push(dt);
                o.sweep_af.push(coffin_manson(i.dt_use, dt, i.cm_exp) * b * c);
            }
            o.equivalence = format!("1 test cycle ≈ {:.1} field cycles", o.af);
            o.notes.push("Constants depend on the solder: tin–lead n = 1.9, p = 1/3, C = 1414 K (Norris & Landzberg 1969); tin–silver–copper values such as n = 2.65, p = 0.136, C = 2185 K are commonly quoted from Pan et al. 2005 — check the source for your alloy and package.".into());
        }
        "ipl" => {
            pos(i.s_use, "use stress")?;
            pos(i.s_test, "test stress")?;
            o.af = inverse_power(i.s_use, i.s_test, i.ipl_exp);
            o.factors.push(Factor {
                name: "Stress ratio".into(),
                value: o.af,
                formula: format!("({} / {})^{:.2}", i.s_test, i.s_use, i.ipl_exp),
            });
            o.sweep_label = "test stress (same unit as the use stress)".into();
            for k in 0..=60 {
                let s = i.s_use * (1.0 + 4.0 * k as f64 / 60.0);
                o.sweep_x.push(s);
                o.sweep_af.push(inverse_power(i.s_use, s, i.ipl_exp));
            }
            o.equivalence = format!("1 h at stress {} ≈ {:.1} h at stress {}", i.s_test, o.af, i.s_use);
            o.notes.push("Inverse power law for non-thermal stresses. For random vibration an exponent around 6.4 on the Grms ratio is commonly quoted (Steinberg); for capacitor voltage it depends on the dielectric — check your part's data.".into());
        }
        m => return Err(format!("unknown model `{m}`")),
    }
    if !o.af.is_finite() {
        return Err("acceleration factor overflowed — check the inputs".into());
    }
    if o.af < 1.0 {
        o.notes.push("AF < 1: the test is milder than use.".into());
    }
    Ok(o)
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct PlanIn {
    pub af: f64,
    /// Field life to demonstrate, in field units (hours or cycles).
    pub life: f64,
    pub reliability: f64,
    pub confidence: f64,
    pub beta: f64,
    pub failures: u32,
    /// Units on test (for the duration answer).
    pub units: u32,
    /// Test duration per unit in test units (for the units answer).
    pub test_duration: f64,
    /// Test units per day (24 for chamber hours, cycles per day for cyclers).
    pub per_day: f64,
}

impl Default for PlanIn {
    fn default() -> Self {
        PlanIn {
            af: 20.0,
            life: 5840.0,
            reliability: 0.9,
            confidence: 0.9,
            beta: 2.0,
            failures: 0,
            units: 10,
            test_duration: 500.0,
            per_day: 24.0,
        }
    }
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct PlanRow {
    pub units: u32,
    pub test_duration: f64,
    pub days: f64,
    pub lives: f64,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct PlanOut {
    /// χ²(C; 2r+2) / (2(−ln R)) — the "unit-lives^β" the test must accumulate.
    pub required: f64,
    pub field_per_test_unit: f64,
    /// Test duration per unit for `units` on test.
    pub duration_for_units: f64,
    pub days_for_units: f64,
    pub lives_for_units: f64,
    /// Units for `test_duration`.
    pub units_for_duration: u32,
    pub lives_for_duration: f64,
    /// Success-run sample (test exactly one life, β irrelevant).
    pub success_run_units: u32,
    pub table: Vec<PlanRow>,
    pub notes: Vec<String>,
}

/// Zero/few-failure demonstration test plan with acceleration.
pub fn plan(i: PlanIn) -> Result<PlanOut, String> {
    if !(i.reliability > 0.0 && i.reliability < 1.0) {
        return Err("reliability must be between 0 and 1".into());
    }
    if !(i.confidence > 0.0 && i.confidence < 1.0) {
        return Err("confidence must be between 0 and 1".into());
    }
    pos(i.af, "acceleration factor")?;
    pos(i.life, "field life")?;
    pos(i.beta, "β")?;
    let required = stats::chi2_quantile(i.confidence, 2.0 * i.failures as f64 + 2.0) / (2.0 * -i.reliability.ln());
    let per_day = i.per_day.max(1e-9);
    let dur = |n: f64| i.life / i.af * (required / n).powf(1.0 / i.beta);
    let units_for = |t: f64| -> u32 {
        let k = t * i.af / i.life;
        (required / k.powf(i.beta) - 1e-9).ceil().max(1.0 + i.failures as f64) as u32
    };
    let n = i.units.max(1) as f64;
    let d = dur(n);
    let table = [1u32, 2, 3, 4, 5, 6, 8, 10, 12, 15, 20, 25, 30, 40, 50]
        .iter()
        .filter(|&&u| u > i.failures)
        .map(|&u| {
            let t = dur(u as f64);
            PlanRow {
                units: u,
                test_duration: t,
                days: t / per_day,
                lives: t * i.af / i.life,
            }
        })
        .collect();
    let mut notes = vec![];
    if i.failures > 0 {
        notes.push(format!(
            "Allowing {} failure(s) assumes every unit still runs the full duration (failures near the end); plan margin.",
            i.failures
        ));
    }
    let lives = d * i.af / i.life;
    if lives > 3.0 {
        notes.push("Testing more than about three lives risks triggering failure modes that never occur in the field — keep the stress and the test multiple realistic.".into());
    }
    notes.push("The shape β must come from prior data (Weibull tab, supplier data or the literature); a β that is too optimistic makes the plan non-conservative.".into());
    Ok(PlanOut {
        required,
        field_per_test_unit: i.af,
        duration_for_units: d,
        days_for_units: d / per_day,
        lives_for_units: lives,
        units_for_duration: units_for(i.test_duration.max(1e-9)),
        lives_for_duration: i.test_duration * i.af / i.life,
        success_run_units: ((1.0 - i.confidence).ln() / i.reliability.ln() - 1e-9).ceil() as u32,
        table,
        notes,
    })
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct ProfileIn {
    /// `thermal_cycle` | `damp_heat` | `dry_heat` | `cold`
    pub kind: String,
    pub t_low_c: f64,
    pub t_high_c: f64,
    pub ramp_c_per_min: f64,
    pub dwell_min: f64,
    pub cycles: u32,
    pub rh: f64,
    /// Hold duration for steady tests, hours.
    pub hold_h: f64,
    pub ambient_c: f64,
}

impl Default for ProfileIn {
    fn default() -> Self {
        ProfileIn {
            kind: "thermal_cycle".into(),
            t_low_c: -20.0,
            t_high_c: 60.0,
            ramp_c_per_min: 5.0,
            dwell_min: 15.0,
            cycles: 3,
            rh: 93.0,
            hold_h: 240.0,
            ambient_c: 25.0,
        }
    }
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Segment {
    pub label: String,
    pub start_min: f64,
    pub end_min: f64,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct ProfileOut {
    pub t_min: Vec<f64>,
    pub temp_c: Vec<f64>,
    /// Relative humidity where controlled, else NaN (serialised as null).
    pub rh: Vec<Option<f64>>,
    pub segments: Vec<Segment>,
    pub cycle_min: f64,
    pub cycles_per_day: f64,
    pub total_h: f64,
}

/// Chamber set-point profile (piecewise linear) for animation and scheduling.
pub fn profile(i: ProfileIn) -> Result<ProfileOut, String> {
    let ramp = i.ramp_c_per_min.clamp(0.1, 60.0);
    let mut o = ProfileOut::default();
    let mut t = 0.0;
    let push = |o: &mut ProfileOut, t: f64, c: f64, rh: Option<f64>| {
        o.t_min.push(t);
        o.temp_c.push(c);
        o.rh.push(rh);
    };
    let seg = |o: &mut ProfileOut, label: &str, a: f64, b: f64| {
        o.segments.push(Segment {
            label: label.into(),
            start_min: a,
            end_min: b,
        })
    };
    match i.kind.as_str() {
        "thermal_cycle" => {
            if i.t_high_c <= i.t_low_c {
                return Err("the high temperature must exceed the low one".into());
            }
            let span = i.t_high_c - i.t_low_c;
            let r = span / ramp;
            let dwell = i.dwell_min.max(0.0);
            push(&mut o, 0.0, i.ambient_c, None);
            let first = (i.ambient_c - i.t_low_c).abs() / ramp;
            t += first;
            push(&mut o, t, i.t_low_c, None);
            seg(&mut o, "ramp to cold", 0.0, t);
            for _ in 0..i.cycles.clamp(1, 20) {
                let s = t;
                t += dwell;
                push(&mut o, t, i.t_low_c, None);
                seg(&mut o, "cold dwell", s, t);
                let s = t;
                t += r;
                push(&mut o, t, i.t_high_c, None);
                seg(&mut o, "ramp up", s, t);
                let s = t;
                t += dwell;
                push(&mut o, t, i.t_high_c, None);
                seg(&mut o, "hot dwell", s, t);
                let s = t;
                t += r;
                push(&mut o, t, i.t_low_c, None);
                seg(&mut o, "ramp down", s, t);
            }
            o.cycle_min = 2.0 * (r + dwell);
        }
        "damp_heat" | "dry_heat" | "cold" => {
            let target = if i.kind == "cold" { i.t_low_c } else { i.t_high_c };
            let rh = if i.kind == "damp_heat" {
                Some(i.rh.clamp(10.0, 98.0))
            } else {
                None
            };
            let hold = i.hold_h.max(0.1) * 60.0;
            push(&mut o, 0.0, i.ambient_c, rh.map(|_| 50.0));
            t += (target - i.ambient_c).abs() / ramp;
            push(&mut o, t, target, rh);
            seg(&mut o, "ramp", 0.0, t);
            let s = t;
            t += hold;
            push(&mut o, t, target, rh);
            seg(&mut o, "hold", s, t);
            let s = t;
            t += (target - i.ambient_c).abs() / ramp;
            push(&mut o, t, i.ambient_c, rh.map(|_| 50.0));
            seg(&mut o, "recover", s, t);
            o.cycle_min = t;
        }
        k => return Err(format!("unknown profile `{k}`")),
    }
    o.cycles_per_day = if o.cycle_min > 0.0 { 1440.0 / o.cycle_min } else { 0.0 };
    o.total_h = t / 60.0;
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrhenius_hand_calculation() {
        // Ea = 0.7 eV, 55 °C → 125 °C:
        // 0.7 / 8.617333e-5 = 8123.19 K; 1/328.15 − 1/398.15 = 5.3578e-4; AF = exp(4.3523) = 77.7
        let af = arrhenius(0.7, 55.0, 125.0);
        assert!((af - 77.66).abs() < 0.1, "{af}");
        // equal temperatures → 1
        assert!((arrhenius(0.7, 40.0, 40.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn peck_and_cycling_models() {
        // humidity term alone: (85/60)^2.66
        let h = peck(0.79, 2.66, 85.0, 60.0, 85.0, 85.0);
        assert!((h - (85.0f64 / 60.0).powf(2.66)).abs() < 1e-9);
        assert!((coffin_manson(20.0, 80.0, 2.0) - 16.0).abs() < 1e-12);
        // Norris–Landzberg with equal frequency and peak temperature reduces to Coffin–Manson
        let nl = norris_landzberg(20.0, 80.0, 1.9, 1.0, 1.0, 1.0 / 3.0, 1414.0, 60.0, 60.0);
        assert!((nl - 4f64.powf(1.9)).abs() < 1e-9);
        assert!((inverse_power(1.0, 2.0, 3.0) - 8.0).abs() < 1e-12);
    }

    #[test]
    fn planner_zero_failure_matches_lipson() {
        // R = 0.9, C = 0.9, β = 2, AF = 1, test 2 lives → n = ln 0.1 / (4 ln 0.9) = 5.46 → 6
        let p = plan(PlanIn {
            af: 1.0,
            life: 1000.0,
            test_duration: 2000.0,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(p.units_for_duration, 6);
        assert_eq!(p.success_run_units, 22);
        // units = 22 → exactly one life
        let p = plan(PlanIn {
            af: 1.0,
            life: 1000.0,
            units: 22,
            ..Default::default()
        })
        .unwrap();
        assert!(
            (p.lives_for_units - (21.854_345 / 22.0f64).sqrt()).abs() < 1e-4,
            "{}",
            p.lives_for_units
        );
    }

    #[test]
    fn profile_cycle_time() {
        // −20 ↔ 60 °C at 5 °C/min with 15 min dwells: 2 × (16 + 15) = 62 min
        let p = profile(ProfileIn::default()).unwrap();
        assert!((p.cycle_min - 62.0).abs() < 1e-9);
        assert!(p.temp_c.iter().all(|t| (-20.0..=60.0).contains(t) || (*t - 25.0).abs() < 1e-9));
    }
}
