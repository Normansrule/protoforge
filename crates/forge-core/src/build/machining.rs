//! Feeds and speeds for end milling.
//!
//! ```text
//! RPM   = (SFM · 12) / (π · D_in)          (imperial: SFM in ft/min, D in inches)
//! n     = (1000 · Vc) / (π · D_mm)          (metric: Vc in m/min)
//! feed  = n · z · f_z                       (mm/min or in/min)
//! MRR   = a_e · a_p · feed                  (radial × axial engagement × feed)
//! P     = MRR · u                           (u = specific cutting energy, W·s/mm³)
//! RCTF  = 1 / √(1 − (1 − 2·a_e/D)²)         (radial chip thinning, a_e < D/2)
//! ```
//!
//! With less than 50 % radial engagement the chip is thinner than the
//! programmed feed per tooth, so the feed per tooth is raised by the radial
//! chip thinning factor (RCTF) to keep the real chip thickness on target
//! (standard tool-maker guidance).
//!
//! Surface speeds and chip loads are **typical starting ranges for carbide
//! end mills** compiled from general machining references and tool-maker
//! charts; specific cutting energies for metals follow the ranges tabulated
//! by Kalpakjian & Schmid. Plastics and wood are not in that table — their
//! values are order-of-magnitude estimates and flagged as such.
//!
//! References: E. Oberg et al., *Machinery's Handbook* (Industrial Press);
//! S. Kalpakjian, S. R. Schmid, *Manufacturing Engineering and Technology*
//! (Pearson), table of approximate specific energy in cutting; Sandvik
//! Coromant and Harvey Tool published milling guidance (chip thinning).

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

pub const MM_PER_IN: f64 = 25.4;
pub const M_PER_FT: f64 = 0.3048;

#[derive(Clone, Debug, Serialize)]
pub struct MachMaterial {
    pub id: &'static str,
    pub name: &'static str,
    /// Surface speed with carbide tools, SFM [lo, hi].
    pub sfm_carbide: [f64; 2],
    /// Surface speed with high-speed-steel tools, SFM [lo, hi].
    pub sfm_hss: [f64; 2],
    /// Chip load per tooth for a 6 mm (≈¼ in) end mill, mm [lo, hi].
    pub chip_mm_6: [f64; 2],
    /// Specific cutting energy, W·s/mm³ [lo, hi].
    pub unit_power: [f64; 2],
    /// true when `unit_power` is an estimate outside the reference table.
    pub unit_power_estimated: bool,
    pub flutes: &'static str,
    pub coolant: &'static str,
    pub notes: &'static str,
}

pub const MATERIALS: &[MachMaterial] = &[
    MachMaterial {
        id: "al6061",
        name: "Aluminium 6061-T6",
        sfm_carbide: [600.0, 1200.0],
        sfm_hss: [250.0, 600.0],
        chip_mm_6: [0.025, 0.075],
        unit_power: [0.4, 1.1],
        unit_power_estimated: false,
        flutes: "2–3 flutes, polished / aluminium-specific geometry",
        coolant: "Mist, flood or air blast — clear the chips",
        notes: "Hobby spindles rarely reach the top of the surface-speed range with small tools — run the spindle's maximum and keep the chip load.",
    },
    MachMaterial {
        id: "steel",
        name: "Mild steel (e.g. 1018)",
        sfm_carbide: [350.0, 600.0],
        sfm_hss: [70.0, 120.0],
        chip_mm_6: [0.02, 0.05],
        unit_power: [2.0, 9.0],
        unit_power_estimated: false,
        flutes: "4 flutes (coated carbide)",
        coolant: "Flood coolant, or dry with coated carbide and air",
        notes: "Needs a rigid machine; hobby routers struggle.",
    },
    MachMaterial {
        id: "stainless",
        name: "Stainless steel (e.g. 304)",
        sfm_carbide: [200.0, 350.0],
        sfm_hss: [40.0, 70.0],
        chip_mm_6: [0.013, 0.04],
        unit_power: [2.0, 5.0],
        unit_power_estimated: false,
        flutes: "4–5 flutes (coated carbide)",
        coolant: "Flood coolant",
        notes: "Work-hardens: never let the tool rub — keep a real chip.",
    },
    MachMaterial {
        id: "brass",
        name: "Brass (free-machining)",
        sfm_carbide: [400.0, 800.0],
        sfm_hss: [150.0, 300.0],
        chip_mm_6: [0.025, 0.075],
        unit_power: [1.4, 3.3],
        unit_power_estimated: false,
        flutes: "2–3 flutes, low or zero helix / rake reduces grabbing",
        coolant: "Often dry",
        notes: "Machines beautifully; tools can 'grab' with high positive rake.",
    },
    MachMaterial {
        id: "pom",
        name: "Acetal / POM (Delrin)",
        sfm_carbide: [500.0, 1000.0],
        sfm_hss: [300.0, 600.0],
        chip_mm_6: [0.05, 0.15],
        unit_power: [0.05, 0.3],
        unit_power_estimated: true,
        flutes: "1–2 flutes (O-flute or up-cut)",
        coolant: "Air blast — keep it cool, avoid re-cutting chips",
        notes: "The easiest plastic to machine; great for jigs.",
    },
    MachMaterial {
        id: "acrylic",
        name: "Acrylic (cast PMMA)",
        sfm_carbide: [400.0, 800.0],
        sfm_hss: [250.0, 500.0],
        chip_mm_6: [0.05, 0.13],
        unit_power: [0.05, 0.3],
        unit_power_estimated: true,
        flutes: "1 flute (O-flute) for clean edges",
        coolant: "Air blast; melting means the chip is too thin",
        notes: "Cast acrylic machines far better than extruded.",
    },
    MachMaterial {
        id: "hdpe",
        name: "HDPE",
        sfm_carbide: [500.0, 1000.0],
        sfm_hss: [300.0, 600.0],
        chip_mm_6: [0.075, 0.2],
        unit_power: [0.05, 0.3],
        unit_power_estimated: true,
        flutes: "1–2 flutes (O-flute)",
        coolant: "Air blast",
        notes: "Soft and stringy — big chips, sharp tools, good work holding.",
    },
    MachMaterial {
        id: "wood",
        name: "Wood / plywood",
        sfm_carbide: [500.0, 1500.0],
        sfm_hss: [300.0, 800.0],
        chip_mm_6: [0.075, 0.25],
        unit_power: [0.02, 0.2],
        unit_power_estimated: true,
        flutes: "2 flutes; compression bits for plywood edges",
        coolant: "Dust extraction",
        notes: "Too small a chip scorches the wood.",
    },
];

pub fn material(id: &str) -> Option<&'static MachMaterial> {
    MATERIALS.iter().find(|m| m.id == id)
}

/// RPM from surface speed (ft/min) and diameter (in).
pub fn rpm_imperial(sfm: f64, d_in: f64) -> f64 {
    sfm * 12.0 / (PI * d_in)
}

/// RPM from cutting speed (m/min) and diameter (mm).
pub fn rpm_metric(vc_m_min: f64, d_mm: f64) -> f64 {
    1000.0 * vc_m_min / (PI * d_mm)
}

/// Radial chip-thinning factor for radial engagement `ae` on diameter `d`.
pub fn chip_thinning_factor(ae: f64, d: f64) -> f64 {
    let r = (ae / d).clamp(1e-6, 1.0);
    if r >= 0.5 {
        1.0
    } else {
        let x = 1.0 - 2.0 * r;
        1.0 / (1.0 - x * x).sqrt()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct FeedsIn {
    pub material: String,
    /// "carbide" | "hss"
    pub tool: String,
    pub diameter_mm: f64,
    pub flutes: u32,
    /// Radial depth of cut (stepover), mm.
    pub woc_mm: f64,
    /// Axial depth of cut, mm.
    pub doc_mm: f64,
    /// Where in the typical ranges to sit, 0 = conservative, 1 = aggressive.
    pub aggressiveness: f64,
    /// Spindle limits.
    pub max_rpm: f64,
    pub min_rpm: f64,
    /// Spindle / drive efficiency for motor power.
    pub efficiency: f64,
    /// Apply radial chip thinning compensation.
    pub chip_thinning: bool,
    /// Optional overrides.
    pub sfm_override: Option<f64>,
    pub chip_load_override_mm: Option<f64>,
}

impl Default for FeedsIn {
    fn default() -> Self {
        FeedsIn {
            material: "al6061".into(),
            tool: "carbide".into(),
            diameter_mm: 6.0,
            flutes: 3,
            woc_mm: 1.5,
            doc_mm: 6.0,
            aggressiveness: 0.4,
            max_rpm: 24000.0,
            min_rpm: 1000.0,
            efficiency: 0.8,
            chip_thinning: true,
            sfm_override: None,
            chip_load_override_mm: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct FeedsOut {
    pub material: &'static MachMaterial,
    pub sfm_range: [f64; 2],
    pub chip_range_mm: [f64; 2],
    /// Target surface speed before spindle limits.
    pub sfm_target: f64,
    pub vc_target_m_min: f64,
    pub rpm_ideal: f64,
    /// RPM after clamping to the spindle.
    pub rpm: f64,
    pub rpm_limited: bool,
    pub sfm_actual: f64,
    pub vc_actual_m_min: f64,
    pub chip_load_mm: f64,
    pub chip_thinning_factor: f64,
    pub chip_load_adjusted_mm: f64,
    pub feed_mm_min: f64,
    pub feed_ipm: f64,
    pub mrr_cm3_min: f64,
    pub mrr_in3_min: f64,
    pub power_spindle_w: f64,
    pub power_motor_w: f64,
    pub power_range_w: [f64; 2],
    pub torque_nm: f64,
    pub radial_engagement_pct: f64,
    pub warnings: Vec<String>,
    pub steps: Vec<String>,
}

fn lerp(a: [f64; 2], t: f64) -> f64 {
    a[0] + (a[1] - a[0]) * t.clamp(0.0, 1.0)
}

/// Chip load range scaled to the tool diameter (≈ linear with D around the 6 mm reference).
pub fn chip_range_for(m: &MachMaterial, d_mm: f64) -> [f64; 2] {
    let s = (d_mm / 6.0).clamp(0.15, 2.5);
    [m.chip_mm_6[0] * s, m.chip_mm_6[1] * s]
}

pub fn feeds_speeds(i: &FeedsIn) -> Result<FeedsOut, String> {
    let m = material(&i.material).ok_or_else(|| format!("unknown material `{}`", i.material))?;
    if i.diameter_mm <= 0.0 || i.flutes == 0 {
        return Err("diameter and flutes must be positive".into());
    }
    let d = i.diameter_mm;
    let t = i.aggressiveness;
    let sfm_range = if i.tool == "hss" { m.sfm_hss } else { m.sfm_carbide };
    let chip_range = chip_range_for(m, d);
    let sfm_target = i.sfm_override.unwrap_or_else(|| lerp(sfm_range, t));
    let vc_target = sfm_target * M_PER_FT;
    let rpm_ideal = rpm_metric(vc_target, d);
    let max_rpm = if i.max_rpm > 0.0 { i.max_rpm } else { f64::INFINITY };
    let rpm = rpm_ideal.clamp(i.min_rpm.max(0.0).min(max_rpm), max_rpm);
    let rpm_limited = (rpm - rpm_ideal).abs() > 1e-6;
    let vc_actual = PI * d * rpm / 1000.0;
    let sfm_actual = vc_actual / M_PER_FT;
    let fz = i.chip_load_override_mm.unwrap_or_else(|| lerp(chip_range, t));
    let ae = i.woc_mm.clamp(0.0, d);
    let rctf = if i.chip_thinning { chip_thinning_factor(ae, d) } else { 1.0 };
    let fz_adj = fz * rctf;
    let feed = rpm * i.flutes as f64 * fz_adj;
    let mrr_mm3_min = ae * i.doc_mm.max(0.0) * feed;
    let u_mid = 0.5 * (m.unit_power[0] + m.unit_power[1]);
    let p_spindle = mrr_mm3_min / 60.0 * u_mid;
    let eff = i.efficiency.clamp(0.1, 1.0);
    let omega = rpm * 2.0 * PI / 60.0;
    let mut warnings = Vec::new();
    if rpm_limited && rpm < rpm_ideal {
        warnings.push(format!(
            "Spindle limited to {:.0} RPM (ideal {:.0}): surface speed drops to {:.0} SFM — fine for carbide in soft materials; keep the chip load, the feed drops with RPM.",
            rpm, rpm_ideal, sfm_actual
        ));
    } else if rpm_limited {
        warnings.push(format!(
            "Spindle minimum {:.0} RPM is above the ideal {:.0} RPM: the tool runs faster than the recommended surface speed.",
            rpm, rpm_ideal
        ));
    }
    if ae / d > 0.5 && i.doc_mm > d {
        warnings.push("Slotting (more than 50 % engagement) deeper than 1 × D is hard on small tools — use adaptive / trochoidal paths or several passes.".into());
    }
    if i.doc_mm > 4.0 * d {
        warnings.push("Axial depth above ≈4 × D: expect deflection and chatter.".into());
    }
    if rctf > 2.0 {
        warnings.push(format!(
            "Very light radial engagement: chip thinning raises the programmed feed per tooth {rctf:.1}× — check the tool maker's limit."
        ));
    }
    if m.unit_power_estimated {
        warnings.push(
            "Power for this material is an order-of-magnitude estimate (plastics and wood are not in the specific-energy reference table)."
                .into(),
        );
    }
    let steps = vec![
        format!("Vc = {:.0} SFM × 0.3048 = {:.1} m/min", sfm_target, vc_target),
        format!(
            "n = 1000·Vc / (π·D) = 1000 × {:.1} / (π × {:.2}) = {:.0} RPM",
            vc_target, d, rpm_ideal
        ),
        if rpm_limited {
            format!("spindle limit → n = {rpm:.0} RPM")
        } else {
            "within spindle limits".into()
        },
        format!("f_z = {:.4} mm/tooth × chip thinning {:.2} = {:.4} mm/tooth", fz, rctf, fz_adj),
        format!(
            "feed = n·z·f_z = {:.0} × {} × {:.4} = {:.0} mm/min ({:.1} in/min)",
            rpm,
            i.flutes,
            fz_adj,
            feed,
            feed / MM_PER_IN
        ),
        format!(
            "MRR = a_e·a_p·feed = {:.2} × {:.2} × {:.0} = {:.2} cm³/min",
            ae,
            i.doc_mm,
            feed,
            mrr_mm3_min / 1000.0
        ),
        format!(
            "P = MRR·u = {:.1} mm³/s × {:.2} W·s/mm³ = {:.0} W at the cutter ({:.0} W motor at {:.0} % efficiency)",
            mrr_mm3_min / 60.0,
            u_mid,
            p_spindle,
            p_spindle / eff,
            eff * 100.0
        ),
    ];
    Ok(FeedsOut {
        material: m,
        sfm_range,
        chip_range_mm: chip_range,
        sfm_target,
        vc_target_m_min: vc_target,
        rpm_ideal,
        rpm,
        rpm_limited,
        sfm_actual,
        vc_actual_m_min: vc_actual,
        chip_load_mm: fz,
        chip_thinning_factor: rctf,
        chip_load_adjusted_mm: fz_adj,
        feed_mm_min: feed,
        feed_ipm: feed / MM_PER_IN,
        mrr_cm3_min: mrr_mm3_min / 1000.0,
        mrr_in3_min: mrr_mm3_min / MM_PER_IN.powi(3),
        power_spindle_w: p_spindle,
        power_motor_w: p_spindle / eff,
        power_range_w: [mrr_mm3_min / 60.0 * m.unit_power[0], mrr_mm3_min / 60.0 * m.unit_power[1]],
        torque_nm: if omega > 0.0 { p_spindle / omega } else { 0.0 },
        radial_engagement_pct: 100.0 * ae / d,
        warnings,
        steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpm_hand_calcs() {
        // 500 SFM, ½ in cutter: 500·12 / (π·0.5) = 3819.7 RPM
        assert!((rpm_imperial(500.0, 0.5) - 3819.72).abs() < 0.01);
        // 100 m/min, 10 mm cutter: 100000 / (π·10) = 3183.1 RPM
        assert!((rpm_metric(100.0, 10.0) - 3183.10).abs() < 0.01);
        // the two forms agree: 500 SFM = 152.4 m/min, ½ in = 12.7 mm
        assert!((rpm_metric(500.0 * M_PER_FT, 12.7) - rpm_imperial(500.0, 0.5)).abs() < 1e-6);
    }

    #[test]
    fn feed_and_mrr_hand_calc() {
        // 3820 RPM × 4 flutes × 0.002 in = 30.56 in/min; slot 0.5 × 0.25 in → MRR = 3.82 in³/min
        let i = FeedsIn {
            material: "steel".into(),
            diameter_mm: 12.7,
            flutes: 4,
            woc_mm: 12.7,
            doc_mm: 6.35,
            sfm_override: Some(500.0),
            chip_load_override_mm: Some(0.002 * 25.4),
            max_rpm: 0.0,
            min_rpm: 0.0,
            ..Default::default()
        };
        let o = feeds_speeds(&i).unwrap();
        assert!((o.rpm - 3819.72).abs() < 0.05);
        assert!((o.chip_thinning_factor - 1.0).abs() < 1e-12);
        assert!((o.feed_ipm - 3819.72 * 4.0 * 0.002).abs() < 1e-3, "{}", o.feed_ipm);
        assert!((o.mrr_in3_min - 0.5 * 0.25 * o.feed_ipm).abs() < 1e-6);
        // power = MRR (mm³/s) × 5.5 W·s/mm³ (mid of 2–9)
        let mrr_mm3_s = o.mrr_cm3_min * 1000.0 / 60.0;
        assert!((o.power_spindle_w - mrr_mm3_s * 5.5).abs() < 1e-6);
    }

    #[test]
    fn chip_thinning_values() {
        assert!((chip_thinning_factor(3.0, 6.0) - 1.0).abs() < 1e-12);
        // ae = 0.1·D → x = 0.8 → 1/√0.36 = 1.6667
        assert!((chip_thinning_factor(0.6, 6.0) - 1.0 / 0.6).abs() < 1e-9);
        // ae = 0.25·D → x = 0.5 → 1/√0.75 = 1.1547
        assert!((chip_thinning_factor(1.5, 6.0) - 1.154_700_5).abs() < 1e-6);
    }

    #[test]
    fn spindle_limit_clamps_rpm() {
        let o = feeds_speeds(&FeedsIn {
            max_rpm: 10000.0,
            ..Default::default()
        })
        .unwrap();
        assert!(o.rpm_limited && (o.rpm - 10000.0).abs() < 1e-9);
        assert!(o.sfm_actual < o.sfm_target);
        assert!(!o.warnings.is_empty());
    }

    #[test]
    fn unknown_material_errors() {
        assert!(feeds_speeds(&FeedsIn {
            material: "unobtainium".into(),
            ..Default::default()
        })
        .is_err());
    }
}
