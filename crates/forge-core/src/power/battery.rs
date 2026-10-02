//! Battery model: a single lithium-polymer (LiPo) cell as an open-circuit
//! voltage (OCV) source that depends on state of charge, in series with an
//! internal resistance — the simplest equivalent-circuit model ("Rint").
//!
//! ```text
//! V_term = OCV(s) − I·R_int            s = state of charge, 0..1
//! P      = V_term · I   (constant-power load behind the converter)
//! ⇒ R·I² − OCV·I + P = 0  ⇒  I = (OCV − √(OCV² − 4·R·P)) / (2R)
//! ds/dt  = −I / Q                      Q = capacity in coulombs (mAh·3.6)
//! ```
//!
//! If `OCV² < 4RP` the cell cannot deliver the power at all (maximum power
//! transfer is `OCV²/4R`): the terminal voltage collapses. A constant-power
//! load draws *more* current as the voltage sags, which is why runtime
//! shrinks faster than capacity when the cell is cold or aged.
//!
//! **OCV curve.** [`OCV_TABLE`] is a typical curve for a lithium-cobalt-oxide
//! / graphite pouch cell at room temperature and a low (≈ C/10) rate, of the
//! shape found in Linden & Reddy (*Handbook of Batteries*, 4th ed., ch. 26
//! on lithium-ion cells) and in countless cell datasheets: steep near full,
//! a long gentle plateau around 3.7–3.9 V, and a "knee" below ≈ 10 %. It is
//! not the curve of any particular cell — measure yours (a slow C/20
//! discharge, or a pulse/rest test) before trusting runtime to minutes. The
//! table is shifted so its top matches `battery.full_v`.
//!
//! **Temperature.** Capacity falls and resistance rises in the cold. The
//! derating table below is illustrative of the trends reported for Li-ion
//! (capacity ~80–85 % at 0 °C, ~60 % at −20 °C; resistance roughly doubling
//! by 0 °C); real cells vary widely, so the lab lets you override both.
//!
//! **Peukert.** Peukert's law `t = H (C / (I·H))^k` describes the loss of
//! capacity at high rate in lead-acid cells (k ≈ 1.1–1.3). Lithium cells have
//! k ≈ 1.0–1.05; their rate dependence is better captured by the resistance
//! term above. An optional Peukert exponent is provided for comparison.

use serde::{Deserialize, Serialize};

use crate::project::{BatteryParams, Project};

/// (state of charge, OCV in volts) for a typical 4.2 V LiPo cell.
pub const OCV_TABLE: [(f64, f64); 21] = [
    (0.00, 3.27),
    (0.05, 3.61),
    (0.10, 3.69),
    (0.15, 3.71),
    (0.20, 3.73),
    (0.25, 3.75),
    (0.30, 3.77),
    (0.35, 3.79),
    (0.40, 3.80),
    (0.45, 3.82),
    (0.50, 3.84),
    (0.55, 3.85),
    (0.60, 3.87),
    (0.65, 3.91),
    (0.70, 3.95),
    (0.75, 3.98),
    (0.80, 4.02),
    (0.85, 4.08),
    (0.90, 4.11),
    (0.95, 4.15),
    (1.00, 4.20),
];

/// OCV at state of charge `soc` (linear interpolation), shifted to `full_v`.
pub fn ocv(soc: f64, full_v: f64) -> f64 {
    let s = soc.clamp(0.0, 1.0);
    let shift = if full_v > 0.0 { full_v - 4.2 } else { 0.0 };
    for w in OCV_TABLE.windows(2) {
        let (s0, v0) = w[0];
        let (s1, v1) = w[1];
        if s <= s1 {
            return v0 + (v1 - v0) * (s - s0) / (s1 - s0) + shift;
        }
    }
    OCV_TABLE[OCV_TABLE.len() - 1].1 + shift
}

/// (temperature °C, capacity factor, resistance factor) — illustrative Li-ion trends.
pub const TEMP_TABLE: [(f64, f64, f64); 8] = [
    (-20.0, 0.60, 4.0),
    (-10.0, 0.72, 2.8),
    (0.0, 0.83, 1.9),
    (10.0, 0.93, 1.35),
    (25.0, 1.00, 1.00),
    (35.0, 1.00, 0.90),
    (45.0, 0.99, 0.82),
    (60.0, 0.97, 0.75),
];

/// Capacity and resistance multipliers at a cell temperature.
pub fn temp_factors(t_c: f64) -> (f64, f64) {
    let t = t_c.clamp(TEMP_TABLE[0].0, TEMP_TABLE[TEMP_TABLE.len() - 1].0);
    for w in TEMP_TABLE.windows(2) {
        if t <= w[1].0 {
            let f = (t - w[0].0) / (w[1].0 - w[0].0);
            return (w[0].1 + (w[1].1 - w[0].1) * f, w[0].2 + (w[1].2 - w[0].2) * f);
        }
    }
    (1.0, 1.0)
}

/// Terminal current for a constant-power load; `None` if the cell cannot
/// deliver `p_w` (OCV² < 4RP).
pub fn cp_current(ocv_v: f64, r: f64, p_w: f64) -> Option<f64> {
    if p_w <= 0.0 {
        return Some(0.0);
    }
    if r <= 1e-12 {
        return if ocv_v > 0.0 { Some(p_w / ocv_v) } else { None };
    }
    let disc = ocv_v * ocv_v - 4.0 * r * p_w;
    if disc < 0.0 {
        return None;
    }
    // smaller root = the stable operating point (V_term > OCV/2)
    Some((ocv_v - disc.sqrt()) / (2.0 * r))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DischargeConfig {
    /// Constant load power from the battery, mW (None = budget average of the profile).
    pub load_mw: Option<f64>,
    /// Peak (burst) power used for the brown-out check, mW (None = budget peak).
    pub peak_mw: Option<f64>,
    pub temp_c: f64,
    /// Manual capacity multiplier (None = from temperature).
    pub capacity_derate: Option<f64>,
    /// Manual resistance multiplier (None = from temperature).
    pub r_scale: Option<f64>,
    /// State of health: remaining fraction of rated capacity (1 = new cell).
    pub state_of_health: f64,
    /// Peukert exponent (1 = off). Reference current = C/20.
    pub peukert_k: f64,
    pub dt_s: f64,
    pub max_h: f64,
    /// Maximum number of output samples.
    pub max_points: usize,
    /// Also run the comparison cases (ideal, no resistance, cold).
    pub compare: bool,
}

impl Default for DischargeConfig {
    fn default() -> Self {
        DischargeConfig {
            load_mw: None,
            peak_mw: None,
            temp_c: 25.0,
            capacity_derate: None,
            r_scale: None,
            state_of_health: 1.0,
            peukert_k: 1.0,
            dt_s: 30.0,
            max_h: 400.0,
            max_points: 480,
            compare: true,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DischargeCurve {
    pub label: String,
    pub t_h: Vec<f64>,
    pub soc: Vec<f64>,
    pub ocv_v: Vec<f64>,
    pub v_term: Vec<f64>,
    pub current_ma: Vec<f64>,
    /// Terminal voltage if the peak (all-on) load were applied at that moment.
    pub v_peak: Vec<f64>,
    /// Time at which the terminal voltage reaches cutoff (or the run ends).
    pub cutoff_h: f64,
    /// Why the run stopped: "cutoff", "empty", "collapse" or "time limit".
    pub end_reason: String,
    pub soc_at_cutoff: f64,
    pub delivered_mah: f64,
    pub delivered_mwh: f64,
    /// First time a peak burst would pull the terminal below cutoff (brown-out risk).
    pub brownout_h: Option<f64>,
    /// Delivered energy / (rated capacity × nominal V): compare with the budget's usable fraction.
    pub equivalent_usable_fraction: f64,
    pub capacity_mah: f64,
    pub r_ohm: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DischargeResult {
    pub main: DischargeCurve,
    pub comparisons: Vec<DischargeCurve>,
    pub load_mw: f64,
    pub peak_mw: f64,
    pub cutoff_v: f64,
    pub full_v: f64,
    pub capacity_factor: f64,
    pub r_factor: f64,
    /// Budget life (energy × usable fraction / average power) for comparison.
    pub budget_life_h: f64,
    pub budget_usable_fraction: f64,
    /// OCV table (soc, V) after the full_v shift.
    pub ocv_curve: Vec<(f64, f64)>,
    /// Maximum power the fresh, full cell could deliver (OCV²/4R), mW.
    pub max_power_mw: f64,
}

/// Time-stepped constant-power discharge (forward Euler on charge; the
/// final step is interpolated so the cutoff time is not quantised by `dt`).
pub fn simulate(
    b: &BatteryParams,
    load_mw: f64,
    peak_mw: f64,
    cap_factor: f64,
    r_factor: f64,
    cfg: &DischargeConfig,
    label: &str,
) -> DischargeCurve {
    let soh = cfg.state_of_health.clamp(0.05, 1.5);
    let cap_mah = (b.capacity_mah * cap_factor * soh).max(1e-6);
    let r = (b.internal_resistance_ohm * r_factor).max(0.0);
    let p = load_mw.max(0.0) * 1e-3;
    let p_pk = peak_mw.max(load_mw).max(0.0) * 1e-3;
    let dt = cfg.dt_s.clamp(0.5, 3600.0);
    let max_steps = ((cfg.max_h.clamp(0.1, 5000.0) * 3600.0) / dt).ceil() as usize;
    // Peukert: effective capacity at current I, Q_eff = Q (I_ref / I)^(k−1), capped at Q.
    let k = cfg.peukert_k.clamp(1.0, 1.6);
    let i_ref = cap_mah / 20.0 * 1e-3; // C/20 in A
    let mut out = DischargeCurve {
        label: label.into(),
        capacity_mah: cap_mah,
        r_ohm: r,
        ..Default::default()
    };
    let stride = (max_steps / cfg.max_points.max(20)).max(1);
    let mut soc = 1.0f64;
    let mut t = 0.0f64;
    let mut q_mah = 0.0;
    let mut e_mwh = 0.0;
    let mut end = "time limit";
    let push = |o: &mut DischargeCurve, t: f64, soc: f64, oc: f64, vt: f64, i: f64, vp: f64| {
        o.t_h.push(t / 3600.0);
        o.soc.push(soc);
        o.ocv_v.push(oc);
        o.v_term.push(vt);
        o.current_ma.push(i * 1e3);
        o.v_peak.push(vp);
    };
    for step in 0..=max_steps {
        let oc = ocv(soc, b.full_v);
        let Some(i) = cp_current(oc, r, p) else {
            end = "collapse";
            push(&mut out, t, soc, oc, oc / 2.0, oc / (2.0 * r.max(1e-9)), oc / 2.0);
            break;
        };
        let vt = oc - i * r;
        let vp = match cp_current(oc, r, p_pk) {
            Some(ip) => oc - ip * r,
            None => oc / 2.0,
        };
        if out.brownout_h.is_none() && vp < b.cutoff_v {
            out.brownout_h = Some(t / 3600.0);
        }
        if vt <= b.cutoff_v || soc <= 0.0 {
            end = if soc <= 0.0 { "empty" } else { "cutoff" };
            push(&mut out, t, soc, oc, vt, i, vp);
            break;
        }
        if step % stride == 0 {
            push(&mut out, t, soc, oc, vt, i, vp);
        }
        let q_eff = if k > 1.0 && i > i_ref {
            cap_mah * (i_ref / i).powf(k - 1.0)
        } else {
            cap_mah
        };
        let ds = i * 1e3 * dt / 3600.0 / q_eff;
        // look ahead: does this step cross cutoff or empty? interpolate the fraction
        let soc2 = soc - ds;
        let oc2 = ocv(soc2.max(0.0), b.full_v);
        let vt2 = cp_current(oc2, r, p).map(|i2| oc2 - i2 * r).unwrap_or(0.0);
        let mut frac: f64 = 1.0;
        if soc2 < 0.0 {
            frac = frac.min(soc / ds);
        }
        if vt2 < b.cutoff_v && vt > b.cutoff_v {
            frac = frac.min((vt - b.cutoff_v) / (vt - vt2));
        }
        let h = dt * frac;
        q_mah += i * 1e3 * h / 3600.0;
        e_mwh += p * 1e3 * h / 3600.0;
        soc -= ds * frac;
        t += h;
        if frac < 1.0 {
            let oc = ocv(soc.max(0.0), b.full_v);
            let i = cp_current(oc, r, p).unwrap_or(0.0);
            let vp = cp_current(oc, r, p_pk).map(|ip| oc - ip * r).unwrap_or(oc / 2.0);
            if out.brownout_h.is_none() && vp < b.cutoff_v {
                out.brownout_h = Some(t / 3600.0);
            }
            end = if soc <= 1e-9 { "empty" } else { "cutoff" };
            push(&mut out, t, soc.max(0.0), oc, oc - i * r, i, vp);
            break;
        }
    }
    out.cutoff_h = t / 3600.0;
    out.end_reason = end.into();
    out.soc_at_cutoff = soc.max(0.0);
    out.delivered_mah = q_mah;
    out.delivered_mwh = e_mwh;
    let rated = b.capacity_mah * b.nominal_v;
    out.equivalent_usable_fraction = if rated > 0.0 { e_mwh / rated } else { 0.0 };
    out
}

/// Discharge for a project: the budget's average/peak power (or overrides)
/// at the configured temperature, plus comparison runs.
pub fn discharge(p: &Project, avg_mw: f64, peak_mw: f64, cfg: &DischargeConfig) -> DischargeResult {
    let b = &p.design.battery;
    let load = cfg.load_mw.unwrap_or(avg_mw);
    let peak = cfg.peak_mw.unwrap_or(peak_mw);
    let (tc, tr) = temp_factors(cfg.temp_c);
    let cf = cfg.capacity_derate.unwrap_or(tc).clamp(0.05, 1.5);
    let rf = cfg.r_scale.unwrap_or(tr).clamp(0.0, 50.0);
    let main = simulate(b, load, peak, cf, rf, cfg, "this cell");
    let mut comparisons = Vec::new();
    if cfg.compare {
        let base = DischargeConfig {
            state_of_health: 1.0,
            peukert_k: 1.0,
            ..cfg.clone()
        };
        comparisons.push(simulate(b, load, peak, 1.0, 0.0, &base, "fresh, 25 °C, no resistance"));
        comparisons.push(simulate(b, load, peak, 1.0, 1.0, &base, "fresh, 25 °C"));
        let (c0, r0) = temp_factors(0.0);
        comparisons.push(simulate(b, load, peak, c0, r0, &base, "fresh, 0 °C"));
        let (c20, r20) = temp_factors(-20.0);
        comparisons.push(simulate(b, load, peak, c20, r20, &base, "fresh, −20 °C"));
    }
    let usable = b.usable_fraction.clamp(0.0, 1.0);
    let energy = b.capacity_mah * b.nominal_v * usable;
    let r = b.internal_resistance_ohm * rf;
    let v_full = ocv(1.0, b.full_v);
    DischargeResult {
        main,
        comparisons,
        load_mw: load,
        peak_mw: peak,
        cutoff_v: b.cutoff_v,
        full_v: b.full_v,
        capacity_factor: cf,
        r_factor: rf,
        budget_life_h: if load > 0.0 { energy / load } else { 0.0 },
        budget_usable_fraction: usable,
        ocv_curve: OCV_TABLE.iter().map(|(s, _)| (*s, ocv(*s, b.full_v))).collect(),
        max_power_mw: if r > 0.0 {
            v_full * v_full / (4.0 * r) * 1e3
        } else {
            f64::MAX.min(1e12)
        },
    }
}

// ---------------------------------------------------------------- trade

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TradeConfig {
    pub min_mah: f64,
    pub max_mah: f64,
    pub points: usize,
    /// Total wearable mass of the current design (e.g. the `mass.total_g`
    /// metric); None = computed from the CAD frame + components.
    pub total_mass_g: Option<f64>,
    pub life_req_h: f64,
    pub mass_req_g: f64,
}

impl Default for TradeConfig {
    fn default() -> Self {
        TradeConfig {
            min_mah: 40.0,
            max_mah: 500.0,
            points: 93,
            total_mass_g: None,
            life_req_h: 8.0,
            mass_req_g: 50.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TradeOptionPoint {
    pub name: String,
    pub capacity_mah: f64,
    pub life_h: f64,
    pub total_mass_g: f64,
    pub feasible: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TradeResult {
    pub capacity_mah: Vec<f64>,
    pub life_h: Vec<f64>,
    pub battery_mass_g: Vec<f64>,
    pub total_mass_g: Vec<f64>,
    /// Mass without the battery (frame + other components).
    pub other_mass_g: f64,
    /// Battery mass per mAh (from `battery.mass_g / capacity_mah`).
    pub g_per_mah: f64,
    pub avg_mw: f64,
    /// Smallest capacity meeting the life requirement.
    pub min_mah_for_life: f64,
    /// Largest capacity meeting the mass requirement.
    pub max_mah_for_mass: f64,
    pub feasible: bool,
    pub current_mah: f64,
    pub current_life_h: f64,
    pub current_mass_g: f64,
    pub life_req_h: f64,
    pub mass_req_g: f64,
    pub trade_id: Option<String>,
    pub options: Vec<TradeOptionPoint>,
    pub mass_source: String,
}

fn battery_component_mass(p: &Project) -> f64 {
    p.components
        .iter()
        .filter(|c| {
            let n = c.name.to_lowercase();
            c.refdes.starts_with("BT") || n.contains("lipo cell") || n.contains("battery cell")
        })
        .map(|c| c.mass_g * c.qty)
        .sum()
}

/// Sweep battery capacity: life from the budget's average power; battery
/// mass scales linearly with capacity at the current cell's g/mAh (constant
/// specific energy — small cells are in reality a little worse).
pub fn trade(p: &Project, avg_mw: f64, cfg: &TradeConfig) -> TradeResult {
    let b = &p.design.battery;
    let g_per_mah = if b.capacity_mah > 0.0 { b.mass_g / b.capacity_mah } else { 0.0 };
    let (total_now, src) = match cfg.total_mass_g {
        Some(m) => (m, "mass.total_g (supplied)".to_string()),
        None => {
            let frame = crate::cad::metrics(p)
                .into_iter()
                .find(|m| m.key == "cad.frame_mass_g")
                .map(|m| m.value);
            match frame {
                Some(f) => (f + p.component_mass_g(), "CAD frame + components".to_string()),
                None => (p.component_mass_g(), "components only (no frame mass)".to_string()),
            }
        }
    };
    // the battery's mass sits in the component list (BT1); if not, use design.battery.mass_g
    let bat_now = {
        let m = battery_component_mass(p);
        if m > 0.0 {
            m
        } else {
            b.mass_g
        }
    };
    let other = total_now - bat_now;
    let wh_per_mah = b.nominal_v * b.usable_fraction.clamp(0.0, 1.0);
    let life = |c: f64| if avg_mw > 0.0 { c * wh_per_mah / avg_mw } else { 0.0 };
    let n = cfg.points.clamp(3, 2000);
    let caps = crate::math::linspace(cfg.min_mah.max(1.0), cfg.max_mah.max(cfg.min_mah + 1.0), n);
    let min_life = if wh_per_mah > 0.0 {
        cfg.life_req_h * avg_mw / wh_per_mah
    } else {
        f64::INFINITY
    };
    let max_mass = if g_per_mah > 0.0 {
        (cfg.mass_req_g - other) / g_per_mah
    } else {
        f64::INFINITY
    };
    let trade_id = p
        .trades
        .iter()
        .find(|t| t.title.to_lowercase().contains("battery"))
        .map(|t| t.id.clone());
    let options = p
        .trades
        .iter()
        .find(|t| Some(&t.id) == trade_id.as_ref())
        .map(|t| {
            t.options
                .iter()
                .filter_map(|o| {
                    let num: String = o.name.chars().take_while(|ch| ch.is_ascii_digit() || *ch == '.').collect();
                    let c: f64 = num.parse().ok()?;
                    let m = other + c * g_per_mah;
                    Some(TradeOptionPoint {
                        name: o.name.clone(),
                        capacity_mah: c,
                        life_h: life(c),
                        total_mass_g: m,
                        feasible: life(c) >= cfg.life_req_h && m <= cfg.mass_req_g,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    TradeResult {
        life_h: caps.iter().map(|&c| life(c)).collect(),
        battery_mass_g: caps.iter().map(|&c| c * g_per_mah).collect(),
        total_mass_g: caps.iter().map(|&c| other + c * g_per_mah).collect(),
        capacity_mah: caps,
        other_mass_g: other,
        g_per_mah,
        avg_mw,
        min_mah_for_life: min_life,
        max_mah_for_mass: if max_mass.is_finite() { max_mass } else { 1e9 },
        feasible: min_life <= max_mass,
        current_mah: b.capacity_mah,
        current_life_h: life(b.capacity_mah),
        current_mass_g: other + b.capacity_mah * g_per_mah,
        life_req_h: cfg.life_req_h,
        mass_req_g: cfg.mass_req_g,
        trade_id,
        options,
        mass_source: src,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell() -> BatteryParams {
        BatteryParams::default()
    }

    #[test]
    fn ocv_table_interpolates_and_shifts() {
        assert!((ocv(1.0, 4.2) - 4.2).abs() < 1e-12);
        assert!((ocv(0.5, 4.2) - 3.84).abs() < 1e-12);
        assert!((ocv(0.525, 4.2) - 3.845).abs() < 1e-12);
        assert!((ocv(1.0, 4.35) - 4.35).abs() < 1e-12);
        // monotone
        for w in OCV_TABLE.windows(2) {
            assert!(w[1].1 > w[0].1);
        }
    }

    #[test]
    fn constant_power_current_solves_quadratic() {
        let (v, r, p) = (3.8, 0.35, 0.2);
        let i = cp_current(v, r, p).unwrap();
        assert!(((v - i * r) * i - p).abs() < 1e-12);
        assert!(cp_current(3.8, 1.0, 3.8 * 3.8 / 4.0 + 0.01).is_none());
    }

    #[test]
    fn small_current_integrates_to_capacity() {
        // no resistance, cutoff below OCV(0): the run ends when the charge is used up
        let b = BatteryParams {
            cutoff_v: 3.0,
            internal_resistance_ohm: 0.0,
            ..cell()
        };
        let cfg = DischargeConfig {
            dt_s: 60.0,
            max_points: 100_000,
            compare: false,
            ..Default::default()
        };
        let c = simulate(&b, 10.0, 10.0, 1.0, 1.0, &cfg, "t");
        assert_eq!(c.end_reason, "empty");
        assert!((c.delivered_mah - 180.0).abs() < 0.2, "{}", c.delivered_mah);
        // energy = ∫ OCV dQ: mean of the OCV table (trapezoid) × capacity
        let mean_v: f64 = OCV_TABLE.windows(2).map(|w| (w[0].1 + w[1].1) / 2.0 * 0.05).sum();
        assert!(
            (c.delivered_mwh - 180.0 * mean_v).abs() / (180.0 * mean_v) < 0.005,
            "{} vs {}",
            c.delivered_mwh,
            180.0 * mean_v
        );
        // time = energy / power
        assert!((c.cutoff_h - c.delivered_mwh / 10.0).abs() < 0.01);
        // derated capacity delivers proportionally less
        let d = simulate(&b, 10.0, 10.0, 0.8, 1.0, &cfg, "t");
        assert!((d.delivered_mah - 144.0).abs() < 0.2);
    }

    #[test]
    fn resistance_and_cold_shorten_runtime() {
        let p = Project::iris();
        let r = discharge(&p, 54.76, 202.0, &DischargeConfig::default());
        let ideal = &r.comparisons[0];
        let warm = &r.comparisons[1];
        let cold = &r.comparisons[2];
        let frozen = &r.comparisons[3];
        assert!(ideal.cutoff_h >= warm.cutoff_h);
        assert!(warm.cutoff_h > cold.cutoff_h);
        assert!(cold.cutoff_h > frozen.cutoff_h);
        assert_eq!(warm.end_reason, "cutoff");
        // a fresh cell at 25 °C delivers more than the budget's derated usable energy
        assert!(warm.cutoff_h > r.budget_life_h);
        assert!(warm.equivalent_usable_fraction > 0.9 && warm.equivalent_usable_fraction < 1.1);
        // high resistance makes the terminal voltage hit cutoff earlier
        let hi = discharge(
            &p,
            54.76,
            202.0,
            &DischargeConfig {
                r_scale: Some(10.0),
                compare: false,
                ..Default::default()
            },
        );
        assert!(hi.main.cutoff_h < warm.cutoff_h);
        assert!(hi.main.soc_at_cutoff > warm.soc_at_cutoff);
    }

    #[test]
    fn peukert_reduces_capacity_at_high_rate() {
        let b = cell();
        let base = DischargeConfig {
            compare: false,
            ..Default::default()
        };
        let pk = DischargeConfig {
            peukert_k: 1.2,
            ..base.clone()
        };
        let a = simulate(&b, 300.0, 300.0, 1.0, 1.0, &base, "a");
        let c = simulate(&b, 300.0, 300.0, 1.0, 1.0, &pk, "b");
        assert!(c.cutoff_h < a.cutoff_h * 0.9);
    }

    #[test]
    fn trade_band_matches_hand_calc() {
        let p = crate::bridge::diff::iris_v05();
        let avg = 54.76;
        let t = trade(
            &p,
            avg,
            &TradeConfig {
                total_mass_g: Some(45.0),
                ..Default::default()
            },
        );
        let min = 8.0 * avg / (3.7 * 0.85);
        assert!((t.min_mah_for_life - min).abs() < 1e-9);
        // 45 g now, battery 4 g for 180 mAh → other 41 g, 9 g for battery → 405 mAh
        assert!((t.other_mass_g - 41.0).abs() < 1e-9);
        assert!((t.max_mah_for_mass - 405.0).abs() < 1e-6);
        assert!(t.feasible);
        assert_eq!(t.trade_id.as_deref(), Some("T-3"));
        assert_eq!(t.options.len(), 3);
        assert!(!t.options[0].feasible); // 120 mAh misses 8 h
        assert!(t.options[1].feasible);
    }
}
