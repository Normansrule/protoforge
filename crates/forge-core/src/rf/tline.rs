//! Planar transmission lines: microstrip (Hammerstad & Jensen 1980),
//! grounded coplanar waveguide (CPWG, conformal mapping as tabulated by
//! Wadell 1991), edge-coupled differential microstrip (the IPC-2141
//! approximation) and loss estimates (Pozar, *Microwave Engineering*, §3.8).
//!
//! **Microstrip** (quasi-static, Hammerstad & Jensen). With `u = w/h`:
//!
//! ```text
//! Z01(u) = η0/(2π) · ln( f(u)/u + √(1 + 4/u²) ),   f(u) = 6 + (2π − 6)·exp(−(30.666/u)^0.7528)
//! εeff(u) = (εr+1)/2 + (εr−1)/2 · (1 + 10/u)^(−a·b)
//! a = 1 + ln((u⁴ + (u/52)²)/(u⁴ + 0.432))/49 + ln(1 + (u/18.1)³)/18.7,  b = 0.564·((εr−0.9)/(εr+3))^0.053
//! Z0 = Z01(u)/√εeff
//! ```
//!
//! Strip thickness `t` widens the strip: Δu₁ = (t̄/π)·ln(1 + 4e/(t̄·coth²√(6.517u))),
//! Δu_r = ½(1 + 1/cosh√(εr−1))·Δu₁ with t̄ = t/h; then Z0 = Z01(u_r)/√εeff(u_r)
//! and εeff = εeff(u_r)·(Z01(u₁)/Z01(u_r))². Accuracy is ~0.2 % for Z01 and εeff
//! over 0.01 ≤ u ≤ 100 (quoted by the authors). Frequency dispersion
//! (Kirschning–Jansen) is neglected: on a 0.73 mm board it shifts εeff by well
//! under 1 % at 2.4 GHz.
//!
//! **Grounded coplanar waveguide** (strip `w`, gaps `s`, substrate `h` over a
//! bottom ground, top grounds assumed wide and stitched with vias):
//!
//! ```text
//! k = w/(w+2s),  k3 = tanh(πw/4h)/tanh(π(w+2s)/4h)
//! εeff = (1 + εr·K(k')/K(k)·K(k3)/K(k3')) / (1 + K(k')/K(k)·K(k3)/K(k3'))
//! Z0 = 60π/√εeff · 1/( K(k)/K(k') + K(k3)/K(k3') )
//! ```
//!
//! `K` is the complete elliptic integral of the first kind, computed with the
//! arithmetic–geometric mean: K(k) = π / (2·AGM(1, √(1−k²))).
//!
//! **Losses.** Conductor α_c ≈ R_s/(Z0·w) with R_s = √(ωμ0/2σ) (Pozar eq. 3.199
//! approximation, smooth copper); dielectric
//! α_d = k0·εr(εeff−1)·tanδ / (2√εeff·(εr−1)) Np/m (Pozar eq. 3.198).

use serde::{Deserialize, Serialize};

pub const C0: f64 = 299_792_458.0;
pub const ETA0: f64 = 376.730_313_668;
pub const MU0: f64 = 1.256_637_062_12e-6;
pub const EPS0: f64 = 8.854_187_812_8e-12;
/// Copper conductivity, S/m (annealed, 20 °C).
pub const SIGMA_CU: f64 = 5.8e7;

fn z01(u: f64) -> f64 {
    let f = 6.0 + (2.0 * std::f64::consts::PI - 6.0) * (-(30.666 / u).powf(0.7528)).exp();
    ETA0 / (2.0 * std::f64::consts::PI) * (f / u + (1.0 + 4.0 / (u * u)).sqrt()).ln()
}

fn eeff0(u: f64, er: f64) -> f64 {
    let a = 1.0 + ((u.powi(4) + (u / 52.0).powi(2)) / (u.powi(4) + 0.432)).ln() / 49.0 + (1.0 + (u / 18.1).powi(3)).ln() / 18.7;
    let b = 0.564 * ((er - 0.9) / (er + 3.0)).powf(0.053);
    (er + 1.0) / 2.0 + (er - 1.0) / 2.0 * (1.0 + 10.0 / u).powf(-a * b)
}

/// Microstrip characteristic impedance (Ω) and effective permittivity.
/// `w`, `h`, `t` in the same unit (t = 0 for an infinitely thin strip).
pub fn microstrip(w: f64, h: f64, t: f64, er: f64) -> (f64, f64) {
    let u = (w / h).max(1e-4);
    if t <= 0.0 {
        let e = eeff0(u, er);
        return (z01(u) / e.sqrt(), e);
    }
    let tb = t / h;
    let coth = 1.0 / (6.517 * u).sqrt().tanh();
    let du1 = tb / std::f64::consts::PI * (1.0 + 4.0 * std::f64::consts::E / (tb * coth * coth)).ln();
    let dur = 0.5 * (1.0 + 1.0 / (er - 1.0).max(0.0).sqrt().cosh()) * du1;
    let (u1, ur) = (u + du1, u + dur);
    let er_r = eeff0(ur, er);
    let z = z01(ur) / er_r.sqrt();
    let e = er_r * (z01(u1) / z01(ur)).powi(2);
    (z, e)
}

/// Arithmetic–geometric mean.
fn agm(mut a: f64, mut b: f64) -> f64 {
    for _ in 0..60 {
        let (an, bn) = (0.5 * (a + b), (a * b).sqrt());
        a = an;
        b = bn;
        if (a - b).abs() < 1e-15 * a {
            break;
        }
    }
    a
}

/// Complete elliptic integral of the first kind K(k) (modulus k).
pub fn ellip_k(k: f64) -> f64 {
    let k = k.clamp(0.0, 1.0 - 1e-15);
    std::f64::consts::PI / (2.0 * agm(1.0, (1.0 - k * k).sqrt()))
}

/// K(k)/K(k') with k' = √(1 − k²).
fn kratio(k: f64) -> f64 {
    ellip_k(k) / ellip_k((1.0 - k * k).sqrt())
}

/// Grounded coplanar waveguide Z0 (Ω) and εeff. Zero-thickness conductors.
pub fn cpwg(w: f64, s: f64, h: f64, er: f64) -> (f64, f64) {
    let pi = std::f64::consts::PI;
    let k = w / (w + 2.0 * s);
    let k3 = (pi * w / (4.0 * h)).tanh() / (pi * (w + 2.0 * s) / (4.0 * h)).tanh();
    let r1 = kratio(k); // K(k)/K(k')
    let r3 = kratio(k3); // K(k3)/K(k3')
    let q = r3 / r1; // K(k')/K(k) · K(k3)/K(k3')
    let e = (1.0 + er * q) / (1.0 + q);
    let z = 60.0 * pi / e.sqrt() / (r1 + r3);
    (z, e)
}

/// Edge-coupled microstrip differential impedance, IPC-2141 approximation:
/// Z_diff ≈ 2·Z0·(1 − 0.48·exp(−0.96·s/h)). Good to roughly ±10 %; a field
/// solver (or the fab's stack-up tool) should confirm before ordering.
pub fn diff_microstrip(w: f64, s: f64, h: f64, t: f64, er: f64) -> (f64, f64, f64) {
    let (z0, e) = microstrip(w, h, t, er);
    (2.0 * z0 * (1.0 - 0.48 * (-0.96 * s / h).exp()), z0, e)
}

/// Monotone (decreasing in width) bisection for the width giving `target` Ω.
pub fn synth_width(target: f64, z_of_w: impl Fn(f64) -> f64, lo: f64, hi: f64) -> f64 {
    let (mut a, mut b) = (lo.ln(), hi.ln());
    for _ in 0..80 {
        let m = 0.5 * (a + b);
        if z_of_w(m.exp()) > target {
            a = m;
        } else {
            b = m;
        }
    }
    (0.5 * (a + b)).exp()
}

/// Line type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineKind {
    Microstrip,
    #[default]
    Cpwg,
    DiffMicrostrip,
}

/// A trace on a stack-up. Lengths in millimetres.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LineIn {
    pub kind: LineKind,
    pub width_mm: f64,
    /// CPWG gap to the top ground, or the spacing of a differential pair.
    pub gap_mm: f64,
    /// Dielectric height between the trace and its reference plane.
    pub h_mm: f64,
    pub t_um: f64,
    pub er: f64,
    pub tan_d: f64,
    pub f_ghz: f64,
    pub length_mm: f64,
    /// When set, synthesise the width for this impedance (Ω; differential for pairs).
    pub target_ohm: Option<f64>,
}

impl Default for LineIn {
    fn default() -> Self {
        LineIn {
            kind: LineKind::Cpwg,
            width_mm: 1.0,
            gap_mm: 0.2,
            h_mm: 0.73,
            t_um: 35.0,
            er: 4.4,
            tan_d: 0.02,
            f_ghz: 2.44,
            length_mm: 8.0,
            target_ohm: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LineOut {
    pub kind: LineKind,
    pub width_mm: f64,
    pub gap_mm: f64,
    pub h_mm: f64,
    /// Single-ended Z0 (for a pair: each line's own Z0).
    pub z0_ohm: f64,
    /// Differential impedance (pairs only, else = z0).
    pub zdiff_ohm: f64,
    pub eeff: f64,
    pub wavelength_mm: f64,
    pub electrical_deg: f64,
    pub delay_ps: f64,
    pub alpha_c_db_m: f64,
    pub alpha_d_db_m: f64,
    pub loss_db: f64,
    /// Microstrip Z0 of the same strip (for comparison with CPWG).
    pub microstrip_z0_ohm: f64,
    /// Sweep of Z0 vs width for plotting (width mm, Z0 Ω).
    pub sweep_w: Vec<f64>,
    pub sweep_z: Vec<f64>,
    pub notes: Vec<String>,
}

fn z_of(kind: LineKind, w: f64, g: f64, h: f64, t: f64, er: f64) -> (f64, f64, f64) {
    match kind {
        LineKind::Microstrip => {
            let (z, e) = microstrip(w, h, t, er);
            (z, z, e)
        }
        LineKind::Cpwg => {
            let (z, e) = cpwg(w, g, h, er);
            (z, z, e)
        }
        LineKind::DiffMicrostrip => {
            let (zd, z, e) = diff_microstrip(w, g, h, t, er);
            (z, zd, e)
        }
    }
}

/// Analyse (or synthesise) a line.
pub fn line(i: LineIn) -> Result<LineOut, String> {
    if !(i.h_mm > 0.0 && i.er >= 1.0 && i.f_ghz > 0.0) {
        return Err("need h > 0, εr ≥ 1 and f > 0".into());
    }
    let t = i.t_um.max(0.0) * 1e-3;
    let g = i.gap_mm.max(0.01);
    let mut notes = Vec::new();
    let w = match i.target_ohm {
        Some(zt) if zt > 1.0 => {
            let pick = |w: f64| {
                let (z, zd, _) = z_of(i.kind, w, g, i.h_mm, t, i.er);
                if i.kind == LineKind::DiffMicrostrip {
                    zd
                } else {
                    z
                }
            };
            let w = synth_width(zt, pick, 0.005 * i.h_mm, 30.0 * i.h_mm);
            notes.push(format!("width synthesised for {zt:.1} Ω by bisection"));
            w
        }
        _ => i.width_mm.max(0.005),
    };
    let (z0, zd, e) = z_of(i.kind, w, g, i.h_mm, t, i.er);
    let f = i.f_ghz * 1e9;
    let lam0 = C0 / f * 1e3;
    let lam = lam0 / e.sqrt();
    let k0 = 2.0 * std::f64::consts::PI * f / C0;
    let rs = (std::f64::consts::PI * f * MU0 / SIGMA_CU).sqrt();
    let alpha_c = rs / (z0 * w * 1e-3); // Np/m
    let alpha_d = if i.er > 1.0 {
        k0 * i.er * (e - 1.0) * i.tan_d / (2.0 * e.sqrt() * (i.er - 1.0))
    } else {
        0.0
    };
    let np_db = 20.0 / std::f64::consts::LN_10;
    let (ac, ad) = (alpha_c * np_db, alpha_d * np_db);
    let (msz, _) = microstrip(w, i.h_mm, t, i.er);
    let ws = crate::math::logspace(0.05 * i.h_mm, 5.0 * i.h_mm, 60);
    let zs = ws
        .iter()
        .map(|&x| z_of(i.kind, x, g, i.h_mm, t, i.er))
        .map(|(z, zd, _)| if i.kind == LineKind::DiffMicrostrip { zd } else { z })
        .collect();
    if i.kind == LineKind::Cpwg {
        notes.push("CPWG: top grounds assumed wide and via-stitched to the bottom plane every ≤ λ/10 along the line".into());
    }
    if i.kind == LineKind::DiffMicrostrip {
        notes.push("Differential impedance from the IPC-2141 approximation (±10 %) — confirm with your fab's stack-up calculator".into());
    }
    Ok(LineOut {
        kind: i.kind,
        width_mm: w,
        gap_mm: g,
        h_mm: i.h_mm,
        z0_ohm: z0,
        zdiff_ohm: zd,
        eeff: e,
        wavelength_mm: lam,
        electrical_deg: 360.0 * i.length_mm / lam,
        delay_ps: i.length_mm * 1e-3 * e.sqrt() / C0 * 1e12,
        alpha_c_db_m: ac,
        alpha_d_db_m: ad,
        loss_db: (ac + ad) * i.length_mm * 1e-3,
        microstrip_z0_ohm: msz,
        sweep_w: ws,
        sweep_z: zs,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn microstrip_matches_pozar_example() {
        // Pozar, Microwave Engineering (3rd ed.), microstrip design example
        // (Example 3.7): εr = 2.20, d = 0.127 cm → 50 Ω at W/d = 3.081, εe = 1.87.
        // (Pozar uses the simpler Wheeler/Hammerstad closed form; HJ agrees to ~1 %.)
        let (z, e) = microstrip(3.081, 1.0, 0.0, 2.2);
        assert!((z - 50.0).abs() < 1.0, "Z0 = {z}");
        assert!((e - 1.87).abs() < 0.02, "εeff = {e}");
    }

    #[test]
    fn microstrip_fr4_textbook_width() {
        // Widely tabulated: 50 Ω on 1.6 mm FR-4 (εr ≈ 4.4) is ≈ 3.0–3.1 mm wide.
        let w = synth_width(50.0, |w| microstrip(w, 1.6, 0.0, 4.4).0, 0.01, 50.0);
        assert!(w > 2.9 && w < 3.2, "w = {w}");
    }

    #[test]
    fn thickness_lowers_impedance() {
        let (z0, _) = microstrip(1.0, 0.8, 0.0, 4.4);
        let (z1, _) = microstrip(1.0, 0.8, 0.035, 4.4);
        assert!(z1 < z0 && z0 - z1 < 3.0);
    }

    #[test]
    fn elliptic_k_known_values() {
        assert!((ellip_k(0.0) - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        // K(1/√2) = Γ(1/4)² / (4√π) = 1.854074677…
        assert!((ellip_k(std::f64::consts::FRAC_1_SQRT_2) - 1.854_074_677_301_372).abs() < 1e-10);
    }

    #[test]
    fn cpwg_tends_to_microstrip_with_wide_gaps_and_is_lower_with_narrow() {
        let (ms, _) = microstrip(1.4, 0.73, 0.0, 4.4);
        let (wide, _) = cpwg(1.4, 5.0, 0.73, 4.4);
        let (narrow, _) = cpwg(1.4, 0.15, 0.73, 4.4);
        assert!((wide - ms).abs() / ms < 0.12, "cpwg {wide} vs ms {ms}");
        assert!(narrow < wide);
    }

    #[test]
    fn synthesis_round_trips() {
        let o = line(LineIn {
            target_ohm: Some(50.0),
            ..Default::default()
        })
        .unwrap();
        assert!((o.z0_ohm - 50.0).abs() < 0.01);
        let d = line(LineIn {
            kind: LineKind::DiffMicrostrip,
            gap_mm: 0.2,
            target_ohm: Some(90.0),
            ..Default::default()
        })
        .unwrap();
        assert!((d.zdiff_ohm - 90.0).abs() < 0.01);
    }
}
