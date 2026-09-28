//! Fatigue: S-N curves (Basquin), mean-stress correction (Goodman) and
//! cumulative damage (Palmgren–Miner) for a usage spectrum.
//!
//! * **Basquin**: σ_a = σ_f′ (2N)^b. Here σ_f′ ≈ the tensile strength and b is
//!   fixed by the material's quoted fatigue point (σ_a at N_f cycles), so the
//!   line passes through (½ cycle, UTS) and that point.
//! * **Goodman**: a cycle with mean σ_m and amplitude σ_a is as damaging as a
//!   fully reversed amplitude σ_ar = σ_a / (1 − σ_m / σ_u).
//! * **Miner's rule**: damage D = Σ n_i / N_i; failure is predicted at D = 1,
//!   so life = duration / D. Scatter in fatigue is large (a factor of 3–10 in
//!   life is normal), so designs usually target D ≤ 0.1–0.3.
//!
//! Polymers have no true endurance limit and are sensitive to frequency
//! (self-heating), temperature, notches, moisture and — for printed parts —
//! layer orientation; treat these results as a ranking tool.
//!
//! References: O. H. Basquin, "The exponential law of endurance tests",
//! Proc. ASTM 10, 1910; J. Goodman, *Mechanics Applied to Engineering*, 1899;
//! M. A. Miner, "Cumulative damage in fatigue", J. Appl. Mech. 12, 1945;
//! Budynas & Nisbett, *Shigley's Mechanical Engineering Design*, ch. 6.

use serde::{Deserialize, Serialize};

use super::materials::{self, StructMaterial};

/// Cycles to failure at fully reversed amplitude σ_ar (∞ if below the
/// curve's reach or no data).
pub fn cycles_to_failure(m: &StructMaterial, sigma_ar: f64) -> f64 {
    match m.basquin() {
        Some((sf, b)) if sigma_ar > 0.0 => {
            if sigma_ar >= sf {
                return 0.5;
            }
            0.5 * (sigma_ar / sf).powf(1.0 / b)
        }
        _ => f64::INFINITY,
    }
}

/// Goodman equivalent fully reversed amplitude.
pub fn goodman(sigma_a: f64, sigma_m: f64, uts: f64) -> f64 {
    if sigma_m <= 0.0 {
        return sigma_a; // compressive mean: conservatively ignore its benefit
    }
    if sigma_m >= uts {
        return f64::INFINITY;
    }
    sigma_a / (1.0 - sigma_m / uts)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub name: String,
    /// Peak stress of the cycle, MPa.
    pub max_mpa: f64,
    /// Minimum stress of the cycle, MPa (0 for a load that goes on and off).
    pub min_mpa: f64,
    pub cycles_per_day: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct FatigueIn {
    pub material: String,
    pub years: f64,
    pub blocks: Vec<Block>,
    /// Optional knock-down on the S-N curve (surface, notch, layer lines…), 0–1.
    pub strength_factor: f64,
}

impl Default for FatigueIn {
    fn default() -> Self {
        FatigueIn {
            material: "PETG".into(),
            years: 3.0,
            blocks: vec![Block {
                name: "Put on / take off".into(),
                max_mpa: 10.0,
                min_mpa: 0.0,
                cycles_per_day: 10.0,
            }],
            strength_factor: 1.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct BlockOut {
    pub name: String,
    pub amplitude_mpa: f64,
    pub mean_mpa: f64,
    pub equivalent_mpa: f64,
    pub cycles: f64,
    pub cycles_to_failure: f64,
    pub damage: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct FatigueOut {
    pub material: String,
    pub has_data: bool,
    pub uts_mpa: f64,
    pub sf_prime_mpa: f64,
    pub basquin_b: f64,
    pub fatigue_point: [f64; 2],
    pub blocks: Vec<BlockOut>,
    pub damage: f64,
    pub life_years: f64,
    /// Safety factor on stress for the design life (how much the stresses
    /// could grow before D reaches 1), found by bisection.
    pub stress_margin: f64,
    /// S-N curve: cycles and amplitude (MPa).
    pub sn_n: Vec<f64>,
    pub sn_s: Vec<f64>,
    /// Goodman line endpoints: (0, S_e) and (S_u, 0), with S_e the S-N
    /// strength at the design-life cycle count of the largest block.
    pub goodman_se_mpa: f64,
    pub design_cycles: f64,
    pub notes: Vec<String>,
}

fn damage_of(m: &StructMaterial, blocks: &[Block], years: f64, scale: f64, kf: f64) -> (f64, Vec<BlockOut>) {
    let days = years * 365.0;
    let mut d = 0.0;
    let mut out = Vec::new();
    for b in blocks {
        let (mx, mn) = (b.max_mpa * scale, b.min_mpa * scale);
        let amp = 0.5 * (mx - mn).abs();
        let mean = 0.5 * (mx + mn);
        let eq = goodman(amp, mean, m.uts_mpa * kf) / kf;
        let n = b.cycles_per_day.max(0.0) * days;
        let nf = cycles_to_failure(m, eq);
        let di = if nf.is_finite() && nf > 0.0 { n / nf } else { 0.0 };
        d += di;
        out.push(BlockOut {
            name: b.name.clone(),
            amplitude_mpa: amp,
            mean_mpa: mean,
            equivalent_mpa: eq,
            cycles: n,
            cycles_to_failure: nf,
            damage: di,
        });
    }
    (d, out)
}

pub fn analyze(i: FatigueIn) -> Result<FatigueOut, String> {
    let m = materials::get(&i.material);
    let kf = if i.strength_factor > 0.0 { i.strength_factor.min(1.0) } else { 1.0 };
    let (d, blocks) = damage_of(m, &i.blocks, i.years, 1.0, kf);
    let mut notes = vec![
        "Miner's rule ignores load order and fatigue scatter is large; aim for damage D ≤ 0.1–0.3 on a design life.".into(),
        "Polymers have no true endurance limit; S-N data depend on frequency, temperature, humidity and notches.".into(),
    ];
    if m.printed {
        notes.push(
            "Printed parts: layer lines act as notches — use the strength factor (e.g. 0.5–0.7) when stress crosses the layers.".into(),
        );
    }
    let (sfp, b) = m.basquin().unwrap_or((m.uts_mpa, 0.0));
    let has = m.basquin().is_some();
    if !has {
        notes.push(format!(
            "No fatigue data for {} in the database — life cannot be estimated.",
            m.name
        ));
    }
    // Stress margin: scale all stresses until D = 1.
    let margin = if has && d > 0.0 {
        let (mut lo, mut hi) = (0.0, 1.0);
        while damage_of(m, &i.blocks, i.years, hi, kf).0 < 1.0 && hi < 1e6 {
            lo = hi;
            hi *= 2.0;
        }
        for _ in 0..60 {
            let mid = 0.5 * (lo + hi);
            if damage_of(m, &i.blocks, i.years, mid, kf).0 < 1.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    } else {
        f64::INFINITY
    };
    let design_cycles = blocks.iter().map(|x| x.cycles).fold(0.0, f64::max).max(1.0);
    let se = if has { sfp * (2.0 * design_cycles).powf(b) * kf } else { 0.0 };
    let sn_n: Vec<f64> = (0..=40).map(|k| 10f64.powf(k as f64 * 9.0 / 40.0)).collect();
    let sn_s: Vec<f64> = sn_n.iter().map(|&n| if has { sfp * (2.0 * n).powf(b) * kf } else { 0.0 }).collect();
    Ok(FatigueOut {
        material: m.id.into(),
        has_data: has,
        uts_mpa: m.uts_mpa,
        sf_prime_mpa: sfp,
        basquin_b: b,
        fatigue_point: [m.fatigue_cycles, m.fatigue_mpa],
        blocks,
        damage: d,
        life_years: if d > 0.0 { i.years / d } else { f64::INFINITY },
        stress_margin: margin,
        sn_n,
        sn_s,
        goodman_se_mpa: se,
        design_cycles,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basquin_passes_through_its_anchor_points() {
        let m = materials::get("AL6061-T6");
        let (sf, b) = m.basquin().unwrap();
        let n = cycles_to_failure(m, m.fatigue_mpa);
        assert!((n / m.fatigue_cycles - 1.0).abs() < 1e-9);
        assert!((sf * (2.0 * 0.5f64).powf(b) - m.uts_mpa).abs() < 1e-9);
    }

    #[test]
    fn goodman_and_miner() {
        assert!((goodman(10.0, 25.0, 50.0) - 20.0).abs() < 1e-12);
        let m = materials::get("PETG");
        // One block at exactly the 10^7-cycle strength (fully reversed), 10^7 cycles → D = 1.
        let o = analyze(FatigueIn {
            material: "PETG".into(),
            years: 1.0,
            blocks: vec![Block {
                name: "x".into(),
                max_mpa: m.fatigue_mpa,
                min_mpa: -m.fatigue_mpa,
                cycles_per_day: 1e7 / 365.0,
            }],
            strength_factor: 1.0,
        })
        .unwrap();
        assert!((o.damage - 1.0).abs() < 1e-9, "{}", o.damage);
        assert!((o.stress_margin - 1.0).abs() < 1e-6);
    }
}
