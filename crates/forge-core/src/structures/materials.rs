//! Structural materials database.
//!
//! Values are **typical mid-range figures** compiled from generic sources:
//! MatWeb-style datasheet summaries, M. F. Ashby, *Materials Selection in
//! Mechanical Design* (4th ed., 2011) property charts, and manufacturers'
//! generic technical data sheets for filaments, powders and resins. Real
//! grades vary widely (±20 % is normal for polymers) — always use the data
//! sheet of the exact grade you buy.
//!
//! **3D-printed materials** (`printed == true`) are quoted for the in-plane
//! (XY, along the extrusion roads) direction of a well-tuned print with 100 %
//! walls. They are *typical and strongly orientation dependent — Z (across
//! the layers) is weakest*: `z_strength_factor` is the typical ratio of
//! interlayer tensile strength to in-plane strength that the structural
//! solver applies to stress acting across the layers.
//!
//! Fatigue: polymers have no true endurance limit. Where a figure is given it
//! is a *rough estimate* expressed as the fully reversed stress amplitude at
//! `fatigue_cycles` cycles; for polymers it is estimated as a fraction of the
//! tensile strength (typically 0.2–0.4, lower for notch-sensitive grades).

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct StructMaterial {
    pub id: &'static str,
    pub name: &'static str,
    /// Process family: `FDM`, `MJF/SLS`, `molded`, `wrought`, `laminate`.
    pub process: &'static str,
    pub printed: bool,
    /// Young's modulus, GPa.
    pub e_gpa: f64,
    /// Poisson's ratio.
    pub nu: f64,
    pub density_g_cm3: f64,
    /// Yield strength (0.2 % offset for metals; tensile strength at yield or
    /// a strain-limited design stress for polymers without a clear yield), MPa.
    pub yield_mpa: f64,
    /// Ultimate tensile strength, MPa.
    pub uts_mpa: f64,
    /// Elongation at break, %.
    pub elongation_pct: f64,
    /// Fully reversed fatigue strength amplitude at `fatigue_cycles`, MPa
    /// (0 = not meaningful / not given).
    pub fatigue_mpa: f64,
    pub fatigue_cycles: f64,
    /// Maximum continuous service temperature (typical), °C.
    pub max_service_c: f64,
    /// Linear coefficient of thermal expansion, ppm/K (1e-6/K).
    pub cte_ppm_k: f64,
    /// Interlayer (Z) strength ÷ in-plane strength; 1 for isotropic processes.
    pub z_strength_factor: f64,
    /// Typical allowable strain for a single snap-fit assembly, %.
    pub snap_strain_pct: f64,
    pub notes: &'static str,
}

impl StructMaterial {
    /// Young's modulus in MPa (N/mm²).
    pub fn e_mpa(&self) -> f64 {
        self.e_gpa * 1000.0
    }
    /// Shear modulus G = E / (2(1 + ν)), MPa.
    pub fn g_mpa(&self) -> f64 {
        self.e_mpa() / (2.0 * (1.0 + self.nu))
    }
    /// Basquin exponent b in σa = σf′ (2N)^b, with σf′ ≈ UTS anchored at the
    /// quoted fatigue point. Returns `None` when no fatigue data is given.
    pub fn basquin(&self) -> Option<(f64, f64)> {
        if self.fatigue_mpa <= 0.0 || self.fatigue_cycles <= 1.0 {
            return None;
        }
        let sf = self.uts_mpa;
        let b = (self.fatigue_mpa / sf).ln() / (2.0 * self.fatigue_cycles).ln();
        Some((sf, b))
    }
}

pub const MATERIALS: &[StructMaterial] = &[
    StructMaterial {
        id: "PLA",
        name: "PLA (FDM)",
        process: "FDM",
        printed: true,
        e_gpa: 3.2,
        nu: 0.36,
        density_g_cm3: 1.24,
        yield_mpa: 45.0,
        uts_mpa: 50.0,
        elongation_pct: 4.0,
        fatigue_mpa: 12.0,
        fatigue_cycles: 1e7,
        max_service_c: 50.0,
        cte_ppm_k: 68.0,
        z_strength_factor: 0.6,
        snap_strain_pct: 1.5,
        notes: "Stiff and easy to print but brittle; creeps and softens above ~50 °C (a car dashboard in summer). Poor for snap fits and hinges.",
    },
    StructMaterial {
        id: "PETG",
        name: "PETG (FDM)",
        process: "FDM",
        printed: true,
        e_gpa: 2.0,
        nu: 0.38,
        density_g_cm3: 1.27,
        yield_mpa: 45.0,
        uts_mpa: 50.0,
        elongation_pct: 6.0,
        fatigue_mpa: 12.5,
        fatigue_cycles: 1e7,
        max_service_c: 65.0,
        cte_ppm_k: 65.0,
        z_strength_factor: 0.55,
        snap_strain_pct: 2.5,
        notes: "Tougher than PLA, good layer bonding, chemically resistant; the Iris prototype default.",
    },
    StructMaterial {
        id: "ABS",
        name: "ABS (FDM)",
        process: "FDM",
        printed: true,
        e_gpa: 2.1,
        nu: 0.35,
        density_g_cm3: 1.04,
        yield_mpa: 38.0,
        uts_mpa: 40.0,
        elongation_pct: 4.0,
        fatigue_mpa: 10.0,
        fatigue_cycles: 1e7,
        max_service_c: 85.0,
        cte_ppm_k: 90.0,
        z_strength_factor: 0.5,
        snap_strain_pct: 2.0,
        notes: "Heat resistant and machinable; warps without an enclosure; acetone-smoothable.",
    },
    StructMaterial {
        id: "ASA",
        name: "ASA (FDM)",
        process: "FDM",
        printed: true,
        e_gpa: 2.0,
        nu: 0.35,
        density_g_cm3: 1.07,
        yield_mpa: 40.0,
        uts_mpa: 42.0,
        elongation_pct: 5.0,
        fatigue_mpa: 10.5,
        fatigue_cycles: 1e7,
        max_service_c: 90.0,
        cte_ppm_k: 95.0,
        z_strength_factor: 0.5,
        snap_strain_pct: 2.0,
        notes: "ABS-like with much better UV stability — the outdoor choice for printed parts.",
    },
    StructMaterial {
        id: "PA12",
        name: "PA12 (MJF / SLS)",
        process: "MJF/SLS",
        printed: true,
        e_gpa: 1.8,
        nu: 0.39,
        density_g_cm3: 1.01,
        yield_mpa: 44.0,
        uts_mpa: 48.0,
        elongation_pct: 18.0,
        fatigue_mpa: 12.0,
        fatigue_cycles: 1e7,
        max_service_c: 100.0,
        cte_ppm_k: 100.0,
        z_strength_factor: 0.9,
        snap_strain_pct: 4.0,
        notes: "Powder-bed nylon: nearly isotropic, tough, good for snap fits and living features; slightly porous and absorbs moisture.",
    },
    StructMaterial {
        id: "PC",
        name: "Polycarbonate (molded)",
        process: "molded",
        printed: false,
        e_gpa: 2.35,
        nu: 0.37,
        density_g_cm3: 1.20,
        yield_mpa: 62.0,
        uts_mpa: 65.0,
        elongation_pct: 100.0,
        fatigue_mpa: 10.0,
        fatigue_cycles: 1e7,
        max_service_c: 115.0,
        cte_ppm_k: 68.0,
        z_strength_factor: 1.0,
        snap_strain_pct: 4.0,
        notes: "Very tough and clear; notch sensitive and weak in fatigue — use generous radii. The Iris production resin.",
    },
    StructMaterial {
        id: "PC/ABS",
        name: "PC/ABS blend (molded)",
        process: "molded",
        printed: false,
        e_gpa: 2.3,
        nu: 0.36,
        density_g_cm3: 1.15,
        yield_mpa: 55.0,
        uts_mpa: 50.0,
        elongation_pct: 50.0,
        fatigue_mpa: 12.0,
        fatigue_cycles: 1e7,
        max_service_c: 95.0,
        cte_ppm_k: 75.0,
        z_strength_factor: 1.0,
        snap_strain_pct: 2.5,
        notes: "The standard consumer-electronics housing blend; easier to mold than PC.",
    },
    StructMaterial {
        id: "TPU95A",
        name: "TPU 95A (FDM)",
        process: "FDM",
        printed: true,
        e_gpa: 0.026,
        nu: 0.45,
        density_g_cm3: 1.21,
        yield_mpa: 5.2,
        uts_mpa: 30.0,
        elongation_pct: 500.0,
        fatigue_mpa: 0.0,
        fatigue_cycles: 0.0,
        max_service_c: 80.0,
        cte_ppm_k: 150.0,
        z_strength_factor: 0.8,
        snap_strain_pct: 20.0,
        notes: "Rubber-like and hyperelastic: no distinct yield — the 'yield' here is a 20 % strain design limit (E × 0.2). Linear FEA is only indicative.",
    },
    StructMaterial {
        id: "POM",
        name: "Acetal POM (molded)",
        process: "molded",
        printed: false,
        e_gpa: 2.8,
        nu: 0.35,
        density_g_cm3: 1.41,
        yield_mpa: 65.0,
        uts_mpa: 65.0,
        elongation_pct: 35.0,
        fatigue_mpa: 26.0,
        fatigue_cycles: 1e7,
        max_service_c: 95.0,
        cte_ppm_k: 110.0,
        z_strength_factor: 1.0,
        snap_strain_pct: 5.0,
        notes: "Springy, low friction and good in fatigue — clips, gears, hinge bushings. Hard to glue or paint.",
    },
    StructMaterial {
        id: "PA6-GF30",
        name: "Glass-filled nylon PA6-GF30 (molded, dry)",
        process: "molded",
        printed: false,
        e_gpa: 9.5,
        nu: 0.35,
        density_g_cm3: 1.36,
        yield_mpa: 170.0,
        uts_mpa: 180.0,
        elongation_pct: 3.0,
        fatigue_mpa: 50.0,
        fatigue_cycles: 1e7,
        max_service_c: 120.0,
        cte_ppm_k: 30.0,
        z_strength_factor: 1.0,
        snap_strain_pct: 1.5,
        notes: "Dry-as-molded values; conditioned (moisture-saturated) parts are roughly 20–40 % weaker and less stiff. Fibre orientation makes it anisotropic too.",
    },
    StructMaterial {
        id: "PP",
        name: "Polypropylene copolymer (molded)",
        process: "molded",
        printed: false,
        e_gpa: 1.3,
        nu: 0.42,
        density_g_cm3: 0.905,
        yield_mpa: 28.0,
        uts_mpa: 30.0,
        elongation_pct: 200.0,
        fatigue_mpa: 10.0,
        fatigue_cycles: 1e7,
        max_service_c: 100.0,
        cte_ppm_k: 110.0,
        z_strength_factor: 1.0,
        snap_strain_pct: 6.0,
        notes: "The living-hinge material: molecular orientation in a thin molded hinge survives millions of folds.",
    },
    StructMaterial {
        id: "AL6061-T6",
        name: "Aluminium 6061-T6",
        process: "wrought",
        printed: false,
        e_gpa: 68.9,
        nu: 0.33,
        density_g_cm3: 2.70,
        yield_mpa: 276.0,
        uts_mpa: 310.0,
        elongation_pct: 12.0,
        fatigue_mpa: 96.5,
        fatigue_cycles: 5e8,
        max_service_c: 150.0,
        cte_ppm_k: 23.6,
        z_strength_factor: 1.0,
        snap_strain_pct: 0.3,
        notes: "Machinable, anodisable; loses temper above ~150 °C. Hinge brackets, heat spreaders.",
    },
    StructMaterial {
        id: "SS304",
        name: "Stainless steel 304 (annealed)",
        process: "wrought",
        printed: false,
        e_gpa: 193.0,
        nu: 0.29,
        density_g_cm3: 8.0,
        yield_mpa: 215.0,
        uts_mpa: 505.0,
        elongation_pct: 70.0,
        fatigue_mpa: 210.0,
        fatigue_cycles: 1e7,
        max_service_c: 400.0,
        cte_ppm_k: 17.3,
        z_strength_factor: 1.0,
        snap_strain_pct: 0.1,
        notes: "Hinge pins, screws, springs (in cold-worked or spring temper, much stronger). Fatigue figure is an estimate (~0.4 × UTS).",
    },
    StructMaterial {
        id: "Ti-6Al-4V",
        name: "Titanium Ti-6Al-4V (annealed)",
        process: "wrought",
        printed: false,
        e_gpa: 113.8,
        nu: 0.342,
        density_g_cm3: 4.43,
        yield_mpa: 880.0,
        uts_mpa: 950.0,
        elongation_pct: 14.0,
        fatigue_mpa: 510.0,
        fatigue_cycles: 1e7,
        max_service_c: 350.0,
        cte_ppm_k: 8.6,
        z_strength_factor: 1.0,
        snap_strain_pct: 0.6,
        notes: "Premium eyewear frames and hinges: strong, light, hypoallergenic; expensive to machine.",
    },
    StructMaterial {
        id: "FR-4",
        name: "FR-4 glass-epoxy laminate (in-plane)",
        process: "laminate",
        printed: false,
        e_gpa: 22.0,
        nu: 0.14,
        density_g_cm3: 1.85,
        yield_mpa: 300.0,
        uts_mpa: 300.0,
        elongation_pct: 2.0,
        fatigue_mpa: 0.0,
        fatigue_cycles: 0.0,
        max_service_c: 130.0,
        cte_ppm_k: 15.0,
        z_strength_factor: 1.0,
        snap_strain_pct: 0.8,
        notes: "The PCB itself. Brittle — no yield; the 'yield' is the in-plane tensile strength. Through-thickness (Z) CTE is ~50–70 ppm/K.",
    },
];

/// Look a material up by id or name (case-insensitive, tolerant of spacing).
pub fn find(id: &str) -> Option<&'static StructMaterial> {
    let norm = |s: &str| s.to_ascii_lowercase().replace([' ', '_'], "");
    let k = norm(id);
    MATERIALS
        .iter()
        .find(|m| norm(m.id) == k)
        .or_else(|| MATERIALS.iter().find(|m| norm(m.name).starts_with(&k)))
        .or_else(|| match k.as_str() {
            "tpu" => find("TPU95A"),
            "pa" | "nylon" | "pa12mjf" | "pa12sls" => find("PA12"),
            "aluminium" | "aluminum" | "6061" => find("AL6061-T6"),
            "steel" | "stainless" | "304" => find("SS304"),
            "titanium" | "ti" => find("Ti-6Al-4V"),
            "pc-abs" | "pcabs" => find("PC/ABS"),
            "gfnylon" | "pa6gf30" | "pa66" => find("PA6-GF30"),
            _ => None,
        })
}

/// Material by id with PETG as the fallback (the Iris default).
pub fn get(id: &str) -> &'static StructMaterial {
    find(id).unwrap_or(&MATERIALS[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_and_fallback() {
        assert_eq!(get("petg").id, "PETG");
        assert_eq!(get("pc/abs").id, "PC/ABS");
        assert_eq!(get("Aluminium").id, "AL6061-T6");
        assert_eq!(get("unobtainium").id, "PETG");
    }

    #[test]
    fn properties_are_physical() {
        for m in MATERIALS {
            assert!(m.e_gpa > 0.0 && m.nu > 0.0 && m.nu < 0.5, "{}", m.id);
            assert!(m.yield_mpa > 0.0 && m.uts_mpa > 0.0, "{}", m.id);
            assert!(m.z_strength_factor > 0.0 && m.z_strength_factor <= 1.0, "{}", m.id);
            assert!(m.printed == (m.process == "FDM" || m.process == "MJF/SLS"), "{}", m.id);
            if let Some((sf, b)) = m.basquin() {
                assert!(sf > 0.0 && b < 0.0 && b > -0.3, "{} b = {b}", m.id);
            }
        }
        // Shear modulus of aluminium ≈ 26 GPa.
        assert!((get("AL6061-T6").g_mpa() - 25_900.0).abs() < 300.0);
    }
}
