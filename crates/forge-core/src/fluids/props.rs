//! Thermophysical properties: dry air at 1 atm and the solids the Iris pod is
//! made of.
//!
//! Air: linear interpolation in Incropera, DeWitt, Bergman & Lavine,
//! *Fundamentals of Heat and Mass Transfer* (7th ed., Wiley, 2011),
//! Table A.4 (250–400 K). Solids: **typical** handbook / datasheet values —
//! polymers vary by grade and filler, graphite sheets by product family.
//! Every value is labelled "typical" in the UI; check the supplier's
//! datasheet before relying on a number.

use serde::{Deserialize, Serialize};

/// Stefan–Boltzmann constant, W/(m² K⁴).
pub const SIGMA: f64 = 5.670_374_419e-8;
/// Standard gravity, m/s².
pub const G: f64 = 9.806_65;
/// 0 °C in kelvin.
pub const K0: f64 = 273.15;

/// Dry air at atmospheric pressure (SI units).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Air {
    pub t_k: f64,
    /// Density, kg/m³.
    pub rho: f64,
    /// Specific heat, J/(kg K).
    pub cp: f64,
    /// Kinematic viscosity, m²/s.
    pub nu: f64,
    /// Thermal conductivity, W/(m K).
    pub k: f64,
    /// Thermal diffusivity, m²/s.
    pub alpha: f64,
    pub pr: f64,
    /// Expansion coefficient 1/T (ideal gas), 1/K.
    pub beta: f64,
    /// True when `t_k` was outside the tabulated 250–400 K range (clamped).
    pub clamped: bool,
}

// T (K), ρ, cp (kJ/kg K), ν·1e6, k·1e3, α·1e6, Pr — Incropera Table A.4.
const TABLE: [[f64; 7]; 4] = [
    [250.0, 1.3947, 1.006, 11.44, 22.3, 15.9, 0.720],
    [300.0, 1.1614, 1.007, 15.89, 26.3, 22.5, 0.707],
    [350.0, 0.9950, 1.009, 20.92, 30.0, 29.9, 0.700],
    [400.0, 0.8711, 1.014, 26.41, 33.8, 38.3, 0.690],
];

/// Air properties at temperature `t_k` (kelvin).
pub fn air(t_k: f64) -> Air {
    let lo = TABLE[0][0];
    let hi = TABLE[TABLE.len() - 1][0];
    let clamped = !(lo..=hi).contains(&t_k);
    let t = t_k.clamp(lo, hi);
    let mut i = 0;
    while i + 2 < TABLE.len() && t > TABLE[i + 1][0] {
        i += 1;
    }
    let (a, b) = (TABLE[i], TABLE[i + 1]);
    let f = (t - a[0]) / (b[0] - a[0]);
    let l = |j: usize| a[j] + f * (b[j] - a[j]);
    Air {
        t_k,
        rho: l(1),
        cp: l(2) * 1e3,
        nu: l(3) * 1e-6,
        k: l(4) * 1e-3,
        alpha: l(5) * 1e-6,
        pr: l(6),
        beta: 1.0 / t_k.max(1.0),
        clamped,
    }
}

/// A solid (or sheet) for the conduction model. Conductivities in W/(m K);
/// sheets are anisotropic: `k_in` along the sheet, `k_through` across it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ThermalMaterial {
    pub id: String,
    pub name: String,
    pub k_in: f64,
    pub k_through: f64,
    /// Density, kg/m³.
    pub rho: f64,
    /// Specific heat, J/(kg K).
    pub cp: f64,
    /// Typical total hemispherical emissivity of the surface.
    pub emissivity: f64,
    pub note: String,
}

impl ThermalMaterial {
    /// Volumetric heat capacity ρc, J/(m³ K).
    pub fn rho_c(&self) -> f64 {
        self.rho * self.cp
    }
}

fn m(id: &str, name: &str, k_in: f64, k_through: f64, rho: f64, cp: f64, eps: f64, note: &str) -> ThermalMaterial {
    ThermalMaterial {
        id: id.into(),
        name: name.into(),
        k_in,
        k_through,
        rho,
        cp,
        emissivity: eps,
        note: note.into(),
    }
}

/// Frame / shell materials (isotropic). Typical values.
pub fn shell_materials() -> Vec<ThermalMaterial> {
    vec![
        m(
            "PETG",
            "PETG (printed / molded)",
            0.20,
            0.20,
            1270.0,
            1200.0,
            0.90,
            "typical 0.19–0.24 W/m K; printed parts lower across layers",
        ),
        m("PC", "Polycarbonate", 0.20, 0.20, 1200.0, 1200.0, 0.90, "typical 0.19–0.22 W/m K"),
        m("ABS", "ABS", 0.17, 0.17, 1050.0, 1400.0, 0.90, "typical 0.14–0.21 W/m K"),
        m("PLA", "PLA", 0.13, 0.13, 1240.0, 1800.0, 0.90, "typical 0.12–0.16 W/m K"),
        m(
            "PA12",
            "Nylon PA12 (SLS / MJF)",
            0.23,
            0.23,
            1010.0,
            1700.0,
            0.90,
            "typical 0.2–0.3 W/m K",
        ),
        m(
            "TPC",
            "Thermally conductive polymer (filled)",
            3.0,
            3.0,
            1600.0,
            1100.0,
            0.90,
            "filled grades typically 1–10 W/m K in-flow; check the datasheet",
        ),
        m(
            "Mg",
            "Magnesium alloy (AZ91D)",
            72.0,
            72.0,
            1810.0,
            1050.0,
            0.40,
            "typical; emissivity depends on finish (painted ≈ 0.9)",
        ),
        m(
            "Al6061",
            "Aluminium 6061 (anodised)",
            167.0,
            167.0,
            2700.0,
            896.0,
            0.80,
            "anodised finish ε ≈ 0.8; bare metal ≈ 0.05–0.1",
        ),
        m("Ti64", "Titanium Ti-6Al-4V", 6.7, 6.7, 4430.0, 526.0, 0.50, "typical"),
    ]
}

/// Look up a shell material by id (case-insensitive); unknown ids map to
/// PETG with `false` in the second slot so callers can warn.
pub fn shell_material(id: &str) -> (ThermalMaterial, bool) {
    let all = shell_materials();
    let key = id.trim().to_lowercase();
    // accept a few aliases from the structures material table
    let key = match key.as_str() {
        "pc_abs" | "pc-abs" => "pc".to_string(),
        "pa12_gf" | "pa12-gf" | "pa12gf" => "pa12".to_string(),
        "aluminium" | "aluminum" | "al" | "6061" | "al-6061" => "al6061".to_string(),
        "titanium" => "ti64".to_string(),
        k => k.to_string(),
    };
    match all.iter().find(|x| x.id.to_lowercase() == key) {
        Some(x) => (x.clone(), true),
        None => (all[0].clone(), false),
    }
}

/// Heat-spreader sheets. In-plane conductivities are typical datasheet ranges.
pub fn spreader(kind: &str) -> Option<ThermalMaterial> {
    match kind {
        "graphite" => Some(m(
            "graphite",
            "Synthetic graphite sheet",
            1500.0,
            10.0,
            1900.0,
            850.0,
            0.85,
            "typical in-plane 700–1900 W/m K, through-plane 5–20 W/m K; thin sheets are the most conductive",
        )),
        "copper" => Some(m(
            "copper",
            "Copper foil",
            390.0,
            390.0,
            8960.0,
            385.0,
            0.05,
            "rolled/electrodeposited copper ≈ 385–400 W/m K",
        )),
        "aluminium" => Some(m(
            "aluminium",
            "Aluminium foil",
            230.0,
            230.0,
            2700.0,
            900.0,
            0.05,
            "typical 205–235 W/m K",
        )),
        _ => None,
    }
}

/// FR-4 board with copper layers: in-plane conductivity by the rule of
/// mixtures (layers conduct in parallel), through-plane by the laminate.
/// `layers` copper layers of `cu_um` thickness with `coverage` fill fraction.
pub fn pcb_material(thickness_mm: f64, layers: u32, cu_um: f64, coverage: f64) -> ThermalMaterial {
    let t = thickness_mm.max(0.1);
    let cu_frac = (layers as f64 * cu_um * 1e-3 * coverage.clamp(0.0, 1.0) / t).min(0.5);
    let k_fr4 = 0.3;
    let k_in = k_fr4 * (1.0 - cu_frac) + 390.0 * cu_frac;
    // through-plane: laminate dominates; thermal vias raise it modestly
    let k_through = 0.35;
    let rho = 1900.0 * (1.0 - cu_frac) + 8960.0 * cu_frac;
    let rc = 1900.0 * 1150.0 * (1.0 - cu_frac) + 8960.0 * 385.0 * cu_frac;
    m(
        "pcb",
        "FR-4 printed circuit board (PCB) with copper",
        k_in,
        k_through,
        rho,
        rc / rho,
        0.9,
        "in-plane by rule of mixtures of FR-4 (0.3 W/m K) and copper; through-plane ≈ 0.35 W/m K",
    )
}

/// Micro-display module (silicon backplane + glass): an assumed effective value.
pub fn display_material() -> ThermalMaterial {
    m(
        "display",
        "Micro-display module (silicon + glass, effective)",
        2.0,
        2.0,
        2400.0,
        800.0,
        0.9,
        "assumed effective value for a silicon backplane under cover glass",
    )
}

/// Effective conductivity of the sealed cavity air: molecular conduction
/// plus linearised enclosure radiation across a gap `gap_m`
/// (`k_rad = 4 σ ε_eff T³ · gap`, ε_eff for two parallel grey surfaces of
/// emissivity ε: 1/(2/ε − 1)). Natural convection is absent while the
/// cavity Rayleigh number stays far below the 1708 onset (Incropera §9.8).
pub fn cavity_air_k(t_k: f64, emissivity: f64, gap_m: f64) -> (f64, f64) {
    let a = air(t_k);
    let e = emissivity.clamp(0.02, 1.0);
    let e_eff = 1.0 / (2.0 / e - 1.0);
    let k_rad = 4.0 * SIGMA * e_eff * t_k.powi(3) * gap_m.max(0.0);
    (a.k, k_rad)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn air_table_points_and_interpolation() {
        let a = air(300.0);
        assert!((a.nu - 15.89e-6).abs() < 1e-10);
        assert!((a.k - 0.0263).abs() < 1e-9);
        assert!((a.pr - 0.707).abs() < 1e-9);
        let b = air(325.0);
        assert!((b.k - 0.5 * (0.0263 + 0.0300)).abs() < 1e-9);
        assert!(!b.clamped);
        assert!(air(500.0).clamped);
    }

    #[test]
    fn pcb_mixture() {
        // 2 layers of 35 µm at 50 % on a 0.8 mm board → 4.4 % copper → ≈ 17 W/m K
        let p = pcb_material(0.8, 2, 35.0, 0.5);
        assert!((p.k_in - (0.3 * (1.0 - 0.04375) + 390.0 * 0.04375)).abs() < 1e-9);
        assert!(p.k_in > 15.0 && p.k_in < 20.0);
        assert!(p.k_through < 1.0);
    }

    #[test]
    fn cavity_radiation_is_comparable_to_conduction() {
        let (k, kr) = cavity_air_k(310.0, 0.9, 2e-3);
        assert!(k > 0.026 && k < 0.028);
        // 4σ ε_eff T³ gap ≈ 4·5.67e-8·0.818·2.98e7·0.002 ≈ 0.011 W/m K
        assert!((kr - 0.011).abs() < 0.001, "{kr}");
    }
}
