//! Antenna first-cut sizing and analytic radiation patterns (Balanis,
//! *Antenna Theory*, chs. 4 and 6).
//!
//! **Thin linear dipole** of total length L along z (Balanis eq. 4-62a):
//!
//! ```text
//! E(θ) ∝ [cos(kL/2·cosθ) − cos(kL/2)] / sinθ
//! ```
//!
//! which gives sinθ for a short (Hertzian) dipole (D = 1.5 = 1.76 dBi) and
//! cos(π/2·cosθ)/sinθ for a half-wave dipole (D = 1.64 = 2.15 dBi).
//!
//! **Uniform linear array** of N elements on the x axis, spacing d,
//! progressive phase β (Balanis §6.3): ψ = k·d·cosγ + β with cosγ = sinθ·cosφ,
//! AF = sin(Nψ/2) / (N·sin(ψ/2)). Pattern multiplication: F = E_element·AF.
//!
//! **Directivity** D = 4π·U_max / ∮U dΩ, integrated numerically on a θ–φ grid
//! (midpoint rule), so any element/array combination is handled the same way.
//!
//! **Small-antenna limit** (Chu 1948; McLean 1996): an antenna that fits in
//! a sphere of radius a has radiation Q ≥ 1/(ka)³ + 1/(ka), and fractional
//! bandwidth ≈ 1/Q. A 3.2 mm chip on its own would have Q in the thousands —
//! the board's ground plane is what really radiates, which is why chip
//! antenna notes specify ground length and clearance.

use serde::{Deserialize, Serialize};

use super::tline::C0;

const PI: f64 = std::f64::consts::PI;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementKind {
    Isotropic,
    Short,
    #[default]
    HalfWave,
    /// Dipole of arbitrary length `length_wl` (in wavelengths).
    Dipole,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PatternIn {
    pub element: ElementKind,
    pub length_wl: f64,
    /// Number of array elements (1 = single element).
    pub n: usize,
    pub spacing_wl: f64,
    pub phase_deg: f64,
    /// Radiation efficiency (0..1) and extra losses for realised gain.
    pub rad_efficiency: f64,
    pub mismatch_db: f64,
    pub body_loss_db: f64,
    /// 3-D mesh resolution (degrees).
    pub mesh_step_deg: f64,
    /// Dynamic range of the 3-D mesh (dB).
    pub mesh_range_db: f64,
}

impl Default for PatternIn {
    fn default() -> Self {
        PatternIn {
            element: ElementKind::HalfWave,
            length_wl: 0.5,
            n: 1,
            spacing_wl: 0.5,
            phase_deg: 0.0,
            rad_efficiency: 0.5,
            mismatch_db: 0.0,
            body_loss_db: 0.0,
            mesh_step_deg: 5.0,
            mesh_range_db: 30.0,
        }
    }
}

/// Element field magnitude (not squared) at polar angle θ.
fn element(kind: ElementKind, l_wl: f64, th: f64) -> f64 {
    let s = th.sin();
    match kind {
        ElementKind::Isotropic => 1.0,
        ElementKind::Short => s.abs(),
        ElementKind::HalfWave | ElementKind::Dipole => {
            let l = if kind == ElementKind::HalfWave { 0.5 } else { l_wl.max(1e-3) };
            let kl2 = PI * l; // k·L/2 with L in wavelengths
            if s.abs() < 1e-9 {
                return 0.0;
            }
            ((kl2 * th.cos()).cos() - kl2.cos()).abs() / s.abs()
        }
    }
}

fn array_factor(n: usize, d_wl: f64, beta: f64, th: f64, ph: f64) -> f64 {
    if n <= 1 {
        return 1.0;
    }
    let psi = 2.0 * PI * d_wl * th.sin() * ph.cos() + beta;
    let den = (n as f64) * (psi / 2.0).sin();
    if den.abs() < 1e-9 {
        return 1.0;
    }
    ((n as f64) * psi / 2.0).sin().abs() / den.abs()
}

/// Normalised field magnitude F(θ, φ).
pub fn field(i: &PatternIn, th: f64, ph: f64) -> f64 {
    element(i.element, i.length_wl, th) * array_factor(i.n, i.spacing_wl, i.phase_deg.to_radians(), th, ph)
}

/// Directivity (linear) by numerical integration of |F|².
pub fn directivity(i: &PatternIn) -> (f64, f64, f64) {
    let (nt, np) = (180, 360);
    let (dt, dp) = (PI / nt as f64, 2.0 * PI / np as f64);
    let mut total = 0.0;
    let mut umax = 0.0;
    let (mut bt, mut bp) = (0.0, 0.0);
    for a in 0..nt {
        let th = (a as f64 + 0.5) * dt;
        let st = th.sin();
        for b in 0..np {
            let ph = (b as f64 + 0.5) * dp;
            let u = field(i, th, ph).powi(2);
            total += u * st * dt * dp;
            if u > umax {
                umax = u;
                bt = th;
                bp = ph;
            }
        }
    }
    (4.0 * PI * umax / total.max(1e-300), bt.to_degrees(), bp.to_degrees())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternOut {
    pub directivity: f64,
    pub directivity_dbi: f64,
    pub realized_gain_dbi: f64,
    pub total_efficiency_pct: f64,
    pub beam_theta_deg: f64,
    pub beam_phi_deg: f64,
    /// Half-power beamwidth in the E-plane (xz) and H-plane (xy) cuts, degrees.
    /// Half-power beamwidths of the strongest lobe in each cut.
    pub hpbw_e_deg: f64,
    pub hpbw_h_deg: f64,
    /// Cuts: angle (deg, 0..360) → normalised dB. E-plane is the xz plane
    /// (angle measured from +z), H-plane the xy plane (from +x).
    pub cut_angle_deg: Vec<f64>,
    pub e_plane_db: Vec<f64>,
    pub h_plane_db: Vec<f64>,
    /// 3-D mesh (Z-up, radius 60 at the peak): positions, indices, level 0..1.
    pub mesh_positions: Vec<f64>,
    pub mesh_indices: Vec<u32>,
    pub mesh_level: Vec<f64>,
}

fn db_norm(v: f64, max: f64) -> f64 {
    (20.0 * (v / max.max(1e-300)).max(1e-10).log10()).max(-60.0)
}

fn hpbw(angles: &[f64], db: &[f64]) -> f64 {
    // width of the main lobe of this cut at 3 dB below the cut's own maximum, degrees
    let (imax, mx) = db
        .iter()
        .enumerate()
        .fold((0, f64::NEG_INFINITY), |acc, (k, &v)| if v > acc.1 { (k, v) } else { acc });
    let n = db.len();
    let step = angles[1] - angles[0];
    let mut w = 0.0;
    for dir in [1isize, -1] {
        let mut k = imax as isize;
        let mut span = 0.0;
        loop {
            let nk = (k + dir).rem_euclid(n as isize);
            if db[nk as usize] < mx - 3.0 {
                // interpolate
                let a = db[k as usize] - mx;
                let b = db[nk as usize] - mx;
                span += step * (a + 3.0) / (a - b).max(1e-12);
                break;
            }
            span += step;
            k = nk;
            if span >= 360.0 {
                return 360.0;
            }
        }
        w += span;
    }
    w.min(360.0)
}

pub fn pattern(i: PatternIn) -> Result<PatternOut, String> {
    if i.n > 64 {
        return Err("at most 64 elements".into());
    }
    let (d, bt, bp) = directivity(&i);
    let ddb = 10.0 * d.log10();
    let eff = i.rad_efficiency.clamp(1e-4, 1.0);
    let total = eff * 10f64.powf(-(i.mismatch_db.max(0.0) + i.body_loss_db.max(0.0)) / 10.0);
    // peak field for normalisation
    let fmax = i_field_max(&i);
    let angles: Vec<f64> = (0..=360).map(|k| k as f64).collect();
    let mut e = Vec::with_capacity(angles.len());
    let mut h = Vec::with_capacity(angles.len());
    for &a in &angles {
        let r = a.to_radians();
        // E-plane xz: θ = |angle| folded, φ = 0 (x>0) or π (x<0)
        let (th, ph) = if r <= PI { (r, 0.0) } else { (2.0 * PI - r, PI) };
        e.push(db_norm(field(&i, th, ph), fmax));
        h.push(db_norm(field(&i, PI / 2.0, r), fmax));
    }
    // 3-D mesh
    let step = i.mesh_step_deg.clamp(2.0, 15.0);
    let nt = (180.0 / step).round() as usize;
    let np = (360.0 / step).round() as usize;
    let range = i.mesh_range_db.clamp(10.0, 60.0);
    let mut pos = Vec::new();
    let mut lvl = Vec::new();
    for a in 0..=nt {
        let th = PI * a as f64 / nt as f64;
        for b in 0..np {
            let ph = 2.0 * PI * b as f64 / np as f64;
            let db = db_norm(field(&i, th, ph), fmax);
            let t = ((db + range) / range).clamp(0.0, 1.0);
            let r = 60.0 * t;
            pos.extend_from_slice(&[r * th.sin() * ph.cos(), r * th.sin() * ph.sin(), r * th.cos()]);
            lvl.push(t);
        }
    }
    let mut idx = Vec::new();
    for a in 0..nt {
        for b in 0..np {
            let b1 = (b + 1) % np;
            let p00 = (a * np + b) as u32;
            let p01 = (a * np + b1) as u32;
            let p10 = ((a + 1) * np + b) as u32;
            let p11 = ((a + 1) * np + b1) as u32;
            idx.extend_from_slice(&[p00, p10, p11, p00, p11, p01]);
        }
    }
    Ok(PatternOut {
        directivity: d,
        directivity_dbi: ddb,
        realized_gain_dbi: ddb + 10.0 * total.log10(),
        total_efficiency_pct: 100.0 * total,
        beam_theta_deg: bt,
        beam_phi_deg: bp,
        hpbw_e_deg: hpbw(&angles[..360], &e[..360]),
        hpbw_h_deg: hpbw(&angles[..360], &h[..360]),
        cut_angle_deg: angles,
        e_plane_db: e,
        h_plane_db: h,
        mesh_positions: pos,
        mesh_indices: idx,
        mesh_level: lvl,
    })
}

fn i_field_max(i: &PatternIn) -> f64 {
    let mut m: f64 = 0.0;
    for a in 0..=180 {
        let th = PI * a as f64 / 180.0;
        for b in 0..360 {
            m = m.max(field(i, th, 2.0 * PI * b as f64 / 360.0));
        }
    }
    m
}

// ------------------------------------------------------------------ sizing

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SizeIn {
    pub f_ghz: f64,
    /// Substrate permittivity for printed antennas.
    pub er: f64,
    /// Board (ground) length and width, mm.
    pub ground_len_mm: f64,
    pub ground_w_mm: f64,
    /// Chip antenna body length, mm (sphere radius for the Chu limit).
    pub chip_len_mm: f64,
}

impl Default for SizeIn {
    fn default() -> Self {
        SizeIn {
            f_ghz: 2.44,
            er: 4.4,
            ground_len_mm: 58.0,
            ground_w_mm: 10.0,
            chip_len_mm: 3.2,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SizeRow {
    pub name: String,
    pub length_mm: f64,
    pub eeff: f64,
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SizeOut {
    pub lambda0_mm: f64,
    pub rows: Vec<SizeRow>,
    pub ground_len_wl: f64,
    pub chu_q_chip: f64,
    pub chu_q_board: f64,
    pub chu_bw_board_pct: f64,
    pub ka_chip: f64,
    pub ka_board: f64,
    pub notes: Vec<String>,
}

/// Chu–McLean minimum radiation Q of an antenna inside a sphere of radius a.
pub fn chu_q(ka: f64) -> f64 {
    1.0 / ka.powi(3) + 1.0 / ka
}

pub fn sizing(i: SizeIn) -> Result<SizeOut, String> {
    if !(i.f_ghz > 0.0 && i.er >= 1.0) {
        return Err("need f > 0 and εr ≥ 1".into());
    }
    let lam = C0 / (i.f_ghz * 1e9) * 1e3;
    let e_pcb = (i.er + 1.0) / 2.0;
    let rows = vec![
        SizeRow {
            name: "Half-wave dipole in air".into(),
            length_mm: 0.475 * lam,
            eeff: 1.0,
            note: "0.47–0.48 λ0: the end effect of a finite-thickness wire shortens it from λ/2 (Balanis §4.6)".into(),
        },
        SizeRow {
            name: "Quarter-wave monopole on a large ground".into(),
            length_mm: 0.2375 * lam,
            eeff: 1.0,
            note: "half of the dipole by image theory; input resistance ≈ 36.5 Ω".into(),
        },
        SizeRow {
            name: "Printed dipole on the board edge".into(),
            length_mm: 0.475 * lam / e_pcb.sqrt(),
            eeff: e_pcb,
            note: "εeff ≈ (εr + 1)/2 for a trace with air on one side (first cut; tune on the board)".into(),
        },
        SizeRow {
            name: "Printed inverted-F antenna (IFA), L + H".into(),
            length_mm: 0.25 * lam / e_pcb.sqrt(),
            eeff: e_pcb,
            note: "resonant length of the arm plus the shorting leg ≈ λg/4; the feed tap sets the resistance (TI small-antenna notes)"
                .into(),
        },
        SizeRow {
            name: "Meandered / folded IFA".into(),
            length_mm: 0.25 * lam / e_pcb.sqrt() * 1.1,
            eeff: e_pcb,
            note: "meanders add coupling, so the unfolded length is typically ~10 % longer than a straight IFA".into(),
        },
    ];
    let ka_chip = 2.0 * PI / lam * (i.chip_len_mm / 2.0);
    let ka_board = 2.0 * PI / lam * (0.5 * (i.ground_len_mm.powi(2) + i.ground_w_mm.powi(2)).sqrt());
    let qb = chu_q(ka_board);
    Ok(SizeOut {
        lambda0_mm: lam,
        rows,
        ground_len_wl: i.ground_len_mm / lam,
        chu_q_chip: chu_q(ka_chip),
        chu_q_board: qb,
        chu_bw_board_pct: 100.0 / qb,
        ka_chip,
        ka_board,
        notes: vec![
            format!(
                "The Iris ground is {:.0} × {:.0} mm = {:.2} λ long: long enough to act as the other half of the antenna, but only {:.2} λ wide — expect the chip antenna to be detuned from its datasheet board and plan a matching network.",
                i.ground_len_mm,
                i.ground_w_mm,
                i.ground_len_mm / lam,
                i.ground_w_mm / lam
            ),
            "Chip antennas are dielectric-loaded radiators: the ceramic shortens the element, but the currents on the ground plane do much of the radiating, so vendors specify the ground size, the clearance area and a tuning network.".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d_of(el: ElementKind) -> f64 {
        directivity(&PatternIn {
            element: el,
            ..Default::default()
        })
        .0
    }

    #[test]
    fn dipole_directivities_match_balanis() {
        assert!((d_of(ElementKind::Short) - 1.5).abs() < 0.005);
        assert!((d_of(ElementKind::HalfWave) - 1.643).abs() < 0.005);
        assert!((d_of(ElementKind::Isotropic) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn broadside_array_directivity_about_2n_d_over_lambda() {
        // Isotropic broadside array, N = 8, d = λ/2: D ≈ 2N·d/λ = 8 (Balanis §6.3)
        let i = PatternIn {
            element: ElementKind::Isotropic,
            n: 8,
            spacing_wl: 0.5,
            ..Default::default()
        };
        let d = directivity(&i).0;
        assert!((d - 8.0).abs() < 0.2, "D = {d}");
    }

    #[test]
    fn half_wave_hpbw_is_78_degrees() {
        let o = pattern(PatternIn::default()).unwrap();
        assert!((o.hpbw_e_deg - 78.0).abs() < 1.5, "{}", o.hpbw_e_deg);
        assert!(o.hpbw_h_deg >= 359.0);
        // 4-element broadside array along x: H-plane beam ≈ 26°, E-plane cut finite and positive
        let a = pattern(PatternIn {
            n: 4,
            ..Default::default()
        })
        .unwrap();
        assert!(a.hpbw_e_deg > 0.0 && a.hpbw_e_deg < 360.0, "{}", a.hpbw_e_deg);
        assert!((a.hpbw_h_deg - 26.3).abs() < 1.5, "{}", a.hpbw_h_deg);
    }

    #[test]
    fn chu_limit_and_sizes() {
        let s = sizing(SizeIn::default()).unwrap();
        assert!((s.lambda0_mm - 122.87).abs() < 0.1);
        assert!(s.chu_q_chip > 1000.0 && s.chu_q_board < 2.0);
    }
}
