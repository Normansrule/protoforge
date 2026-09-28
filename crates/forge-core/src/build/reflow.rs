//! Reflow soldering: profile windows, a lumped oven simulation, and stencil
//! aperture checks.
//!
//! **Profile windows.** Typical ranges consistent with IPC/JEDEC J-STD-020
//! (the classification profile used to qualify components: preheat/soak
//! 150–200 °C for 60–120 s and a 217 °C liquidus for lead-free; 100–150 °C and
//! 183 °C for tin-lead; ramp-up ≤ 3 °C/s, ramp-down ≤ 6 °C/s) narrowed to
//! the windows typical solder-paste datasheets recommend for assembly (time
//! above liquidus ≈45–90 s, peak ≈20–30 °C above liquidus). Your paste's
//! datasheet always wins.
//!
//! **Oven model.** The board rides a conveyor through heated zones. Each
//! thermocouple location is a lumped thermal mass heated by convection:
//!
//! ```text
//! m·c_p · dT/dt = h·A · (T_air(x(t)) − T)   ⇒   dT/dt = (T_air − T) / τ,   τ = m·c_p / (h·A)
//! x(t) = v · t
//! ```
//!
//! integrated exactly per step (`T += (T_air − T)(1 − e^(−Δt/τ))`). A small
//! passive (short τ) follows the air closely; a large part or heavy copper
//! (long τ) lags — the difference is the board's ΔT. The zone air temperature
//! blends linearly over a short transition between zones. This is a
//! teaching model: real ovens need a measured profile (thermocouples on the
//! board).
//!
//! **Stencil.** IPC-7525: area ratio = aperture area / aperture wall area
//! = L·W / (2·(L + W)·t) ≥ 0.66 and aspect ratio W/t ≥ 1.5 for good paste
//! release.
//!
//! References: IPC/JEDEC J-STD-020; IPC-7525 *Stencil design guidelines*;
//! F. P. Incropera, D. P. DeWitt, *Fundamentals of Heat and Mass Transfer*
//! (lumped capacitance method).

use serde::{Deserialize, Serialize};

use crate::project::Project;

#[derive(Clone, Debug, Serialize)]
pub struct Alloy {
    pub id: &'static str,
    pub name: &'static str,
    pub liquidus_c: f64,
    pub soak_lo_c: f64,
    pub soak_hi_c: f64,
    pub soak_s: [f64; 2],
    pub tal_s: [f64; 2],
    pub peak_c: [f64; 2],
    pub max_ramp_c_s: f64,
    pub max_cool_c_s: f64,
    /// Maximum time from 25 °C to peak, s.
    pub max_time_to_peak_s: f64,
    pub iron_c: [f64; 2],
    pub notes: &'static str,
    pub default_zones: &'static [f64],
    pub default_speed_cm_min: f64,
}

impl Alloy {
    pub fn sac305() -> Alloy {
        Alloy {
            id: "SAC305",
            name: "SAC305 lead-free (Sn96.5 Ag3.0 Cu0.5)",
            liquidus_c: 217.0,
            soak_lo_c: 150.0,
            soak_hi_c: 200.0,
            soak_s: [60.0, 120.0],
            tal_s: [45.0, 90.0],
            peak_c: [235.0, 250.0],
            max_ramp_c_s: 3.0,
            max_cool_c_s: 6.0,
            max_time_to_peak_s: 480.0,
            iron_c: [330.0, 370.0],
            notes: "Melting range ≈217–220 °C; J-STD-020 allows up to 150 s above 217 °C and component peaks of 245–260 °C by package — pastes typically ask for 45–90 s and a 235–250 °C peak.",
            default_zones: &[110.0, 175.0, 190.0, 240.0, 250.0, 90.0],
            default_speed_cm_min: 52.0,
        }
    }
    pub fn snpb() -> Alloy {
        Alloy {
            id: "Sn63Pb37",
            name: "Sn63Pb37 tin-lead (eutectic)",
            liquidus_c: 183.0,
            soak_lo_c: 100.0,
            soak_hi_c: 150.0,
            soak_s: [60.0, 120.0],
            tal_s: [45.0, 90.0],
            peak_c: [205.0, 225.0],
            max_ramp_c_s: 3.0,
            max_cool_c_s: 6.0,
            max_time_to_peak_s: 360.0,
            iron_c: [300.0, 340.0],
            notes: "Eutectic at 183 °C. Not RoHS compliant — for repair, hobby and exempt products only; lead-free is required for products sold in the EU.",
            default_zones: &[100.0, 145.0, 160.0, 180.0, 232.0, 80.0],
            default_speed_cm_min: 48.0,
        }
    }
    pub fn by_id(id: &str) -> Alloy {
        if id.to_ascii_lowercase().contains("pb") {
            Alloy::snpb()
        } else {
            Alloy::sac305()
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ReflowIn {
    pub alloy: String,
    /// Zone setpoints, °C (last zones may be cooling zones).
    pub zones: Vec<f64>,
    pub zone_length_mm: f64,
    pub conveyor_cm_min: f64,
    /// Time constant of a small part (0402 passive), s.
    pub tau_small_s: f64,
    /// Time constant of the heaviest part (QFN / shield / heavy copper), s.
    pub tau_large_s: f64,
    pub ambient_c: f64,
    /// Width of the air-temperature blend between zones, mm.
    pub transition_mm: f64,
    /// Output sample interval, s.
    pub sample_s: f64,
}

impl Default for ReflowIn {
    fn default() -> Self {
        let a = Alloy::sac305();
        ReflowIn {
            alloy: a.id.into(),
            zones: a.default_zones.to_vec(),
            zone_length_mm: 450.0,
            conveyor_cm_min: a.default_speed_cm_min,
            tau_small_s: 30.0,
            tau_large_s: 42.0,
            ambient_c: 25.0,
            transition_mm: 60.0,
            sample_s: 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ProbeStats {
    pub peak_c: f64,
    pub tal_s: f64,
    pub soak_s: f64,
    pub max_ramp_c_s: f64,
    pub max_cool_c_s: f64,
    pub time_to_peak_s: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Check {
    pub probe: &'static str,
    pub label: String,
    pub value: f64,
    pub unit: &'static str,
    pub lo: Option<f64>,
    pub hi: Option<f64>,
    pub pass: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReflowOut {
    pub alloy: Alloy,
    pub t_s: Vec<f64>,
    pub x_mm: Vec<f64>,
    pub air_c: Vec<f64>,
    pub small_c: Vec<f64>,
    pub large_c: Vec<f64>,
    /// Time at which the board enters each zone, s.
    pub zone_entry_s: Vec<f64>,
    pub oven_length_mm: f64,
    pub dwell_per_zone_s: f64,
    pub small: ProbeStats,
    pub large: ProbeStats,
    pub checks: Vec<Check>,
    pub pass: bool,
    pub max_delta_t_c: f64,
}

/// Air temperature at conveyor position `x` (mm; 0 = oven entrance).
pub fn air_temp(zones: &[f64], zl: f64, trans: f64, ambient: f64, x: f64) -> f64 {
    let n = zones.len();
    let len = zl * n as f64;
    if x < 0.0 || x >= len || n == 0 {
        return ambient;
    }
    let i = ((x / zl) as usize).min(n - 1);
    let half = trans / 2.0;
    let xb = (i + 1) as f64 * zl;
    if xb - x < half && i + 1 < n {
        let f = (x - (xb - half)) / trans;
        return zones[i] + (zones[i + 1] - zones[i]) * f;
    }
    let xa = i as f64 * zl;
    if x - xa < half && i > 0 {
        let f = 0.5 + (x - xa) / trans;
        return zones[i - 1] + (zones[i] - zones[i - 1]) * f;
    }
    zones[i]
}

fn stats(ts: &[f64], ys: &[f64], dt: f64, a: &Alloy) -> ProbeStats {
    let mut peak = f64::MIN;
    let mut ipk = 0;
    for (k, &y) in ys.iter().enumerate() {
        if y > peak {
            peak = y;
            ipk = k;
        }
    }
    let tal = ys.iter().filter(|&&y| y >= a.liquidus_c).count() as f64 * dt;
    let soak = ys[..=ipk].iter().filter(|&&y| y >= a.soak_lo_c && y <= a.soak_hi_c).count() as f64 * dt;
    // slopes over a 1 s window
    let w = ((1.0 / dt).round() as usize).max(1);
    let (mut up, mut down) = (0.0f64, 0.0f64);
    for k in 0..ys.len().saturating_sub(w) {
        let s = (ys[k + w] - ys[k]) / (w as f64 * dt);
        up = up.max(s);
        down = down.max(-s);
    }
    let t25 = ts.iter().zip(ys).find(|(_, &y)| y > 25.5).map(|(t, _)| *t).unwrap_or(0.0);
    ProbeStats {
        peak_c: peak,
        tal_s: tal,
        soak_s: soak,
        max_ramp_c_s: up,
        max_cool_c_s: down,
        time_to_peak_s: ts[ipk] - t25,
    }
}

fn checks_for(probe: &'static str, s: &ProbeStats, a: &Alloy) -> Vec<Check> {
    let c = |label: String, value: f64, unit: &'static str, lo: Option<f64>, hi: Option<f64>| Check {
        probe,
        pass: lo.is_none_or(|l| value >= l - 1e-9) && hi.is_none_or(|h| value <= h + 1e-9),
        label,
        value,
        unit,
        lo,
        hi,
    };
    vec![
        c("Max heating ramp".into(), s.max_ramp_c_s, "°C/s", None, Some(a.max_ramp_c_s)),
        c(
            format!("Soak {:.0}–{:.0} °C", a.soak_lo_c, a.soak_hi_c),
            s.soak_s,
            "s",
            Some(a.soak_s[0]),
            Some(a.soak_s[1]),
        ),
        c(
            format!("Time above liquidus ({:.0} °C)", a.liquidus_c),
            s.tal_s,
            "s",
            Some(a.tal_s[0]),
            Some(a.tal_s[1]),
        ),
        c("Peak temperature".into(), s.peak_c, "°C", Some(a.peak_c[0]), Some(a.peak_c[1])),
        c("Max cooling rate".into(), s.max_cool_c_s, "°C/s", None, Some(a.max_cool_c_s)),
        c("Time 25 °C → peak".into(), s.time_to_peak_s, "s", None, Some(a.max_time_to_peak_s)),
    ]
}

pub fn simulate(i: &ReflowIn) -> Result<ReflowOut, String> {
    let a = Alloy::by_id(&i.alloy);
    if i.zones.is_empty() || i.zones.len() > 12 {
        return Err("between 1 and 12 zones".into());
    }
    if i.conveyor_cm_min <= 0.0 || i.zone_length_mm <= 0.0 {
        return Err("conveyor speed and zone length must be positive".into());
    }
    let v = i.conveyor_cm_min * 10.0 / 60.0; // mm/s
    let zl = i.zone_length_mm;
    let len = zl * i.zones.len() as f64;
    let dt = 0.25;
    let lead = 100.0; // start 100 mm before the entrance
    let tail = 600.0; // follow the board 600 mm past the exit
    let total = (lead + len + tail) / v;
    let n = (total / dt).ceil() as usize + 1;
    if n > 400_000 {
        return Err("conveyor too slow for the simulation window".into());
    }
    let (ks, kl) = (
        1.0 - (-dt / i.tau_small_s.max(1.0)).exp(),
        1.0 - (-dt / i.tau_large_s.max(1.0)).exp(),
    );
    let mut ts = Vec::with_capacity(n);
    let mut xs = Vec::with_capacity(n);
    let mut air = Vec::with_capacity(n);
    let mut ys = Vec::with_capacity(n);
    let mut yl = Vec::with_capacity(n);
    let (mut s, mut l) = (i.ambient_c, i.ambient_c);
    for k in 0..n {
        let t = k as f64 * dt;
        let x = -lead + v * t;
        let ta = air_temp(&i.zones, zl, i.transition_mm, i.ambient_c, x);
        s += (ta - s) * ks;
        l += (ta - l) * kl;
        ts.push(t);
        xs.push(x);
        air.push(ta);
        ys.push(s);
        yl.push(l);
    }
    let ss = stats(&ts, &ys, dt, &a);
    let sl = stats(&ts, &yl, dt, &a);
    let mut checks = checks_for("small part", &ss, &a);
    checks.extend(checks_for("large part", &sl, &a));
    let pass = checks.iter().all(|c| c.pass);
    let max_dt = ys.iter().zip(&yl).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
    // downsample for output
    let step = ((i.sample_s.max(dt) / dt).round() as usize).max(1);
    let pick = |v: &Vec<f64>| v.iter().step_by(step).copied().collect::<Vec<_>>();
    Ok(ReflowOut {
        zone_entry_s: (0..i.zones.len()).map(|z| (lead + z as f64 * zl) / v).collect(),
        oven_length_mm: len,
        dwell_per_zone_s: zl / v,
        t_s: pick(&ts),
        x_mm: pick(&xs),
        air_c: pick(&air),
        small_c: pick(&ys),
        large_c: pick(&yl),
        small: ss,
        large: sl,
        checks,
        pass,
        max_delta_t_c: max_dt,
        alloy: a,
    })
}

// ---------------------------------------------------------------------------
// Stencil
// ---------------------------------------------------------------------------

/// IPC-7525 area ratio of a rectangular aperture L × W in a stencil of thickness t.
pub fn area_ratio_rect(l: f64, w: f64, t: f64) -> f64 {
    l * w / (2.0 * (l + w) * t)
}

/// Area ratio of a circular aperture of diameter d: (π d²/4) / (π d t) = d / (4 t).
pub fn area_ratio_circle(d: f64, t: f64) -> f64 {
    d / (4.0 * t)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct StencilIn {
    pub length_mm: f64,
    pub width_mm: f64,
    pub thickness_mm: f64,
    pub circular: bool,
}

impl Default for StencilIn {
    fn default() -> Self {
        StencilIn {
            length_mm: 0.8,
            width_mm: 0.2,
            thickness_mm: 0.12,
            circular: false,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct StencilOut {
    pub area_ratio: f64,
    pub aspect_ratio: f64,
    pub area_ok: bool,
    pub aspect_ok: bool,
    /// Thickness at which the area ratio would be exactly 0.66.
    pub max_thickness_mm: f64,
    pub paste_volume_mm3: f64,
}

pub fn stencil(i: &StencilIn) -> Result<StencilOut, String> {
    if i.thickness_mm <= 0.0 || i.width_mm <= 0.0 || (!i.circular && i.length_mm <= 0.0) {
        return Err("dimensions must be positive".into());
    }
    let t = i.thickness_mm;
    let (ar, asp, area) = if i.circular {
        (
            area_ratio_circle(i.width_mm, t),
            i.width_mm / t,
            std::f64::consts::PI * i.width_mm * i.width_mm / 4.0,
        )
    } else {
        (
            area_ratio_rect(i.length_mm, i.width_mm, t),
            i.length_mm.min(i.width_mm) / t,
            i.length_mm * i.width_mm,
        )
    };
    Ok(StencilOut {
        area_ratio: ar,
        aspect_ratio: asp,
        area_ok: ar >= 0.66,
        aspect_ok: asp >= 1.5,
        max_thickness_mm: t * ar / 0.66,
        paste_volume_mm3: area * t,
    })
}

#[derive(Clone, Debug, Serialize)]
pub struct PadRow {
    pub footprint: String,
    pub pad_w_mm: f64,
    pub pad_h_mm: f64,
    pub count: usize,
    pub refdes: Vec<String>,
    pub area_ratio: f64,
    pub aspect_ratio: f64,
    pub pass: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct StencilCheck {
    pub thickness_mm: f64,
    pub rows: Vec<PadRow>,
    pub min_area_ratio: f64,
    pub worst: String,
    pub failing: usize,
    pub notes: Vec<String>,
}

/// Area ratio of every 1:1 paste aperture on the project's board.
pub fn stencil_check(p: &Project, thickness_mm: f64) -> StencilCheck {
    let t = thickness_mm.max(0.01);
    let mut rows: Vec<PadRow> = Vec::new();
    let mut windowed: Vec<String> = Vec::new();
    for c in p.components.iter().filter(|c| c.on_pcb) {
        let Some(fp) = crate::pcb::footprints::footprint(&c.footprint) else {
            continue;
        };
        for pd in fp.pads.iter().filter(|pd| pd.drill <= 0.0 && pd.paste > 0.0) {
            if pd.paste < 0.99 {
                let k = format!("{} {}", fp.name, pd.num);
                if !windowed.contains(&k) {
                    windowed.push(k);
                }
                continue;
            }
            let (w, h) = (pd.w.max(pd.h), pd.w.min(pd.h));
            let (w, h) = ((w * 1000.0).round() / 1000.0, (h * 1000.0).round() / 1000.0);
            if let Some(r) = rows
                .iter_mut()
                .find(|r| r.footprint == fp.name && (r.pad_w_mm - w).abs() < 1e-9 && (r.pad_h_mm - h).abs() < 1e-9)
            {
                r.count += 1;
                if !r.refdes.contains(&c.refdes) {
                    r.refdes.push(c.refdes.clone());
                }
                continue;
            }
            let circle = matches!(pd.shape, crate::pcb::footprints::PadShape::Circle);
            let ar = if circle {
                area_ratio_circle(w, t)
            } else {
                area_ratio_rect(w, h, t)
            };
            rows.push(PadRow {
                footprint: fp.name.clone(),
                pad_w_mm: w,
                pad_h_mm: h,
                count: 1,
                refdes: vec![c.refdes.clone()],
                area_ratio: ar,
                aspect_ratio: h / t,
                pass: ar >= 0.66,
            });
        }
    }
    rows.sort_by(|a, b| a.area_ratio.total_cmp(&b.area_ratio));
    let min = rows.first().map(|r| r.area_ratio).unwrap_or(f64::NAN);
    let worst = rows
        .first()
        .map(|r| format!("{} {:.2}×{:.2} mm pad", r.footprint, r.pad_w_mm, r.pad_h_mm))
        .unwrap_or_default();
    let failing = rows.iter().filter(|r| !r.pass).count();
    let mut notes =
        vec!["Apertures assumed 1:1 with the copper pad (IPC-7525 allows reductions — they lower the area ratio further).".to_string()];
    if !windowed.is_empty() {
        notes.push(format!(
            "Windowed thermal-pad apertures not checked (each window must be sized separately): {}.",
            windowed.join(", ")
        ));
    }
    StencilCheck {
        thickness_mm: t,
        rows,
        min_area_ratio: min,
        worst,
        failing,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_ratio_hand_calcs() {
        // 1.0 × 0.25 mm pad, 0.12 mm stencil: 0.25 / (2 · 1.25 · 0.12) = 0.8333
        assert!((area_ratio_rect(1.0, 0.25, 0.12) - 0.833_333).abs() < 1e-5);
        // 0.3 mm circle, 0.1 mm stencil: 0.3 / 0.4 = 0.75
        assert!((area_ratio_circle(0.3, 0.1) - 0.75).abs() < 1e-12);
        let o = stencil(&StencilIn::default()).unwrap();
        // QFN 0.8 × 0.2 at 0.12: 0.16 / 0.24 = 0.6667 → just passes
        assert!((o.area_ratio - 2.0 / 3.0).abs() < 1e-9 && o.area_ok);
        assert!((o.max_thickness_mm - 0.12 * (2.0 / 3.0) / 0.66).abs() < 1e-12);
    }

    #[test]
    fn default_profiles_pass_their_windows() {
        let o = simulate(&ReflowIn::default()).unwrap();
        for c in &o.checks {
            assert!(c.pass, "SAC305 {} {}: {:.2}", c.probe, c.label, c.value);
        }
        let a = Alloy::snpb();
        let o = simulate(&ReflowIn {
            alloy: a.id.into(),
            zones: a.default_zones.to_vec(),
            conveyor_cm_min: a.default_speed_cm_min,
            ..Default::default()
        })
        .unwrap();
        for c in &o.checks {
            assert!(c.pass, "SnPb {} {}: {:.2}", c.probe, c.label, c.value);
        }
        // large part lags the small part
        assert!(o.large.peak_c < o.small.peak_c && o.max_delta_t_c > 1.0);
    }

    #[test]
    fn window_checks_catch_bad_profiles() {
        // too fast a conveyor → too little time above liquidus / low peak
        let o = simulate(&ReflowIn {
            conveyor_cm_min: 120.0,
            ..Default::default()
        })
        .unwrap();
        assert!(!o.pass);
        assert!(o.checks.iter().any(|c| c.label.starts_with("Time above") && !c.pass));
        // a scorching peak zone → peak too high
        let o = simulate(&ReflowIn {
            zones: vec![110.0, 175.0, 190.0, 240.0, 300.0, 90.0],
            ..Default::default()
        })
        .unwrap();
        assert!(o.checks.iter().any(|c| c.label == "Peak temperature" && !c.pass));
    }

    #[test]
    fn lumped_step_response() {
        // one very long zone at 200 °C: after τ seconds inside, T ≈ 25 + 175·(1 − e⁻¹)
        let i = ReflowIn {
            zones: vec![200.0],
            zone_length_mm: 100_000.0,
            conveyor_cm_min: 60.0,
            tau_small_s: 30.0,
            tau_large_s: 60.0,
            transition_mm: 0.0,
            sample_s: 0.25,
            ..Default::default()
        };
        let o = simulate(&i).unwrap();
        let t_in = o.zone_entry_s[0];
        let k = o.t_s.iter().position(|&t| t >= t_in + 30.0).unwrap();
        let expect = 25.0 + 175.0 * (1.0 - (-1.0f64).exp());
        assert!((o.small_c[k] - expect).abs() < 1.5, "{} vs {}", o.small_c[k], expect);
    }

    #[test]
    fn iris_stencil_min_ratio() {
        let c = stencil_check(&Project::iris(), 0.12);
        assert!(!c.rows.is_empty());
        // QFN-48 0.2 × 0.8 pads are the limiting aperture: 0.667 at 0.12 mm
        assert!(c.min_area_ratio > 0.3 && c.min_area_ratio <= 0.667 + 1e-3, "{}", c.min_area_ratio);
        assert!(c.notes.iter().any(|n| n.contains("QFN-48_6x6 EP")));
    }
}
