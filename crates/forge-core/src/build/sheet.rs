//! Sheet-metal bend allowance and bend deduction.
//!
//! ```text
//! BA   = θ · (R + K·T)                 bend allowance (arc length of the neutral axis)
//! OSSB = tan(θ/2) · (R + T)            outside setback
//! BD   = 2·OSSB − BA                   bend deduction
//! flat = L1 + L2 − BD                  L1, L2 measured to the outside mold line (virtual sharp)
//! ```
//!
//! θ is the bend angle (how far the flange is bent from flat; 90° for an L),
//! R the inside radius, T the thickness and K the position of the neutral
//! axis as a fraction of T. When K is not given it is estimated with the
//! DIN 6935 correction factor k = 0.65 + ½·log₁₀(R/T) (capped at 1 for
//! R/T ≥ 5), K = k/2.
//!
//! References: E. Oberg et al., *Machinery's Handbook* (sheet-metal bending);
//! DIN 6935, *Cold bending of flat rolled steel products*.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct SheetMaterial {
    pub id: &'static str,
    pub name: &'static str,
    /// Typical minimum inside bend radius as a multiple of thickness [lo, hi].
    pub min_r_over_t: [f64; 2],
    pub notes: &'static str,
}

pub const SHEET_MATERIALS: &[SheetMaterial] = &[
    SheetMaterial {
        id: "al5052",
        name: "Aluminium 5052-H32",
        min_r_over_t: [0.5, 1.5],
        notes: "The usual bendable aluminium for enclosures and brackets.",
    },
    SheetMaterial {
        id: "al6061",
        name: "Aluminium 6061-T6",
        min_r_over_t: [2.0, 4.0],
        notes: "Strong but cracks in tight bends; prefer 5052 for bent parts.",
    },
    SheetMaterial {
        id: "crs",
        name: "Mild steel (cold-rolled)",
        min_r_over_t: [0.5, 1.0],
        notes: "Forgiving; needs plating or paint against rust.",
    },
    SheetMaterial {
        id: "ss304",
        name: "Stainless 304",
        min_r_over_t: [0.5, 1.5],
        notes: "Springs back more — shops over-bend to compensate.",
    },
    SheetMaterial {
        id: "brass",
        name: "Brass (half-hard)",
        min_r_over_t: [1.0, 2.0],
        notes: "Good for springs and contacts.",
    },
    SheetMaterial {
        id: "copper",
        name: "Copper (soft)",
        min_r_over_t: [0.0, 1.0],
        notes: "Very ductile; bus bars and shields.",
    },
];

/// DIN 6935 K-factor estimate from R/T.
pub fn k_factor_din6935(r: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return 0.5;
    }
    let rt = (r / t).max(1e-6);
    if rt >= 5.0 {
        0.5
    } else {
        ((0.65 + 0.5 * rt.log10()) / 2.0).clamp(0.0, 0.5)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct BendIn {
    pub thickness_mm: f64,
    pub radius_mm: f64,
    pub angle_deg: f64,
    /// None → DIN 6935 estimate.
    pub k_factor: Option<f64>,
    /// Outside flange lengths (to the virtual sharp).
    pub flange1_mm: f64,
    pub flange2_mm: f64,
    pub material: String,
}

impl Default for BendIn {
    fn default() -> Self {
        BendIn {
            thickness_mm: 1.5,
            radius_mm: 1.5,
            angle_deg: 90.0,
            k_factor: None,
            flange1_mm: 30.0,
            flange2_mm: 20.0,
            material: "al5052".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct BendOut {
    pub k_factor: f64,
    pub k_source: &'static str,
    pub bend_allowance_mm: f64,
    pub outside_setback_mm: f64,
    pub bend_deduction_mm: f64,
    pub flat_length_mm: f64,
    /// Straight lengths either side of the bend zone in the flat pattern.
    pub flat_straight1_mm: f64,
    pub flat_straight2_mm: f64,
    pub min_radius_mm: [f64; 2],
    pub min_flange_mm: f64,
    pub hole_to_bend_mm: f64,
    pub checks: Vec<(String, bool)>,
}

pub fn bend(i: &BendIn) -> Result<BendOut, String> {
    let t = i.thickness_mm;
    let rr = i.radius_mm.max(0.0);
    if t <= 0.0 {
        return Err("thickness must be positive".into());
    }
    if !(0.0..180.0).contains(&i.angle_deg) {
        return Err("bend angle must be between 0° and 180°".into());
    }
    let th = i.angle_deg.to_radians();
    let (k, src) = match i.k_factor {
        Some(k) => (k.clamp(0.0, 0.5), "user"),
        None => (k_factor_din6935(rr, t), "DIN 6935 estimate"),
    };
    let ba = th * (rr + k * t);
    let ossb = (th / 2.0).tan() * (rr + t);
    let bd = 2.0 * ossb - ba;
    let flat = i.flange1_mm + i.flange2_mm - bd;
    let mat = SHEET_MATERIALS.iter().find(|m| m.id == i.material).unwrap_or(&SHEET_MATERIALS[0]);
    let min_r = [mat.min_r_over_t[0] * t, mat.min_r_over_t[1] * t];
    let min_flange = 4.0 * t;
    let checks = vec![
        (
            format!(
                "Both flanges ({:.1} / {:.1} mm) are longer than the outside setback {:.1} mm, so a straight section remains (sharp bends open the setback: tan(θ/2)·(R+T))",
                i.flange1_mm, i.flange2_mm, ossb
            ),
            i.flange1_mm > ossb && i.flange2_mm > ossb && flat > 0.0,
        ),
        (
            format!("Inside radius {:.2} mm ≥ typical minimum {:.2} mm for {}", rr, min_r[0], mat.name),
            rr >= min_r[0] - 1e-9,
        ),
        (
            format!("Flange 1 ({:.1} mm) ≥ ≈4 × T = {:.1} mm", i.flange1_mm, min_flange),
            i.flange1_mm >= min_flange - 1e-9,
        ),
        (
            format!("Flange 2 ({:.1} mm) ≥ ≈4 × T = {:.1} mm", i.flange2_mm, min_flange),
            i.flange2_mm >= min_flange - 1e-9,
        ),
    ];
    Ok(BendOut {
        k_factor: k,
        k_source: src,
        bend_allowance_mm: ba,
        outside_setback_mm: ossb,
        bend_deduction_mm: bd,
        flat_length_mm: flat.max(0.0),
        flat_straight1_mm: (i.flange1_mm - ossb).max(0.0),
        flat_straight2_mm: (i.flange2_mm - ossb).max(0.0),
        min_radius_mm: min_r,
        min_flange_mm: min_flange,
        hole_to_bend_mm: 2.5 * t + rr,
        checks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bend_allowance_hand_calc() {
        // T = 2, R = 2, K = 0.44, 90°: BA = π/2 · (2 + 0.88) = 4.5239;
        // OSSB = tan 45° · 4 = 4; BD = 8 − 4.5239 = 3.4761; flat = 50 + 30 − 3.4761
        let o = bend(&BendIn {
            thickness_mm: 2.0,
            radius_mm: 2.0,
            angle_deg: 90.0,
            k_factor: Some(0.44),
            flange1_mm: 50.0,
            flange2_mm: 30.0,
            material: "crs".into(),
        })
        .unwrap();
        assert!((o.bend_allowance_mm - 4.523_893).abs() < 1e-5, "{}", o.bend_allowance_mm);
        assert!((o.outside_setback_mm - 4.0).abs() < 1e-12);
        assert!((o.bend_deduction_mm - 3.476_107).abs() < 1e-5);
        assert!((o.flat_length_mm - 76.523_893).abs() < 1e-5);
        // flat = straight1 + BA + straight2
        assert!((o.flat_straight1_mm + o.bend_allowance_mm + o.flat_straight2_mm - o.flat_length_mm).abs() < 1e-9);
    }

    #[test]
    fn din6935_k_factor() {
        // R/T = 1 → k = 0.65 → K = 0.325; R/T ≥ 5 → K = 0.5
        assert!((k_factor_din6935(1.0, 1.0) - 0.325).abs() < 1e-12);
        assert!((k_factor_din6935(10.0, 1.0) - 0.5).abs() < 1e-12);
        let k = k_factor_din6935(2.0, 1.0);
        assert!(k > 0.325 && k < 0.5);
    }

    #[test]
    fn obtuse_and_invalid() {
        // a 45° bend has a smaller setback than 90°
        let a = bend(&BendIn {
            angle_deg: 45.0,
            ..Default::default()
        })
        .unwrap();
        let b = bend(&BendIn::default()).unwrap();
        assert!(a.outside_setback_mm < b.outside_setback_mm);
        assert!(bend(&BendIn {
            thickness_mm: 0.0,
            ..Default::default()
        })
        .is_err());
    }
}
