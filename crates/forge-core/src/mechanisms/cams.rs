//! Disk cams with a translating roller follower (Norton, *Design of
//! Machinery*, ch. 8).
//!
//! **SVAJ.** Each segment (rise, fall, dwell) of angle β and lift h uses a
//! motion law in `x = θ/β`:
//!
//! | law | s/h | peak acceleration |
//! |---|---|---|
//! | cycloidal | `x − sin(2πx)/(2π)` | `2π h/β²` |
//! | 3-4-5 polynomial | `10x³ − 15x⁴ + 6x⁵` | `5.7735 h/β²` |
//! | simple harmonic | `½(1 − cos πx)` | `π² h/(2β²)` |
//!
//! Derivatives are per radian of cam angle; multiply by ω, ω², ω³ for
//! time units. The *fundamental law of cam design*: displacement, velocity
//! and acceleration must be continuous (finite jerk) across the whole cycle
//! — the harmonic law breaks it at a dwell.
//!
//! **Geometry.** Prime circle `R_p = R_b + R_r`, offset ε,
//! `d = √(R_p² − ε²)`. The pitch-curve point in follower coordinates is
//! `(ε, d + s)`; the pressure angle is `φ = atan((v − ε)/(d + s))` (Norton
//! eq. 8.31) — keep |φ| ≤ 30° for a translating follower. The pitch curve's
//! radius of curvature comes from its first and second derivatives; the
//! cam surface is the pitch curve offset inward by the roller radius, so a
//! convex pitch radius below `R_r` undercuts (Norton recommends
//! `ρ_min ≥ 2–3 R_r`).

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

const DEG: f64 = PI / 180.0;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Segment {
    /// rise | fall | dwell
    pub kind: String,
    /// cycloidal | poly345 | harmonic
    #[serde(default = "cyc")]
    pub law: String,
    #[serde(default)]
    pub lift_mm: f64,
    pub beta_deg: f64,
}

fn cyc() -> String {
    "cycloidal".into()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct CamIn {
    pub segments: Vec<Segment>,
    pub base_r_mm: f64,
    pub roller_r_mm: f64,
    pub offset_mm: f64,
    pub rpm: f64,
    pub samples: usize,
}

impl Default for CamIn {
    fn default() -> Self {
        CamIn {
            segments: vec![
                Segment {
                    kind: "rise".into(),
                    law: "cycloidal".into(),
                    lift_mm: 20.0,
                    beta_deg: 100.0,
                },
                Segment {
                    kind: "dwell".into(),
                    law: "cycloidal".into(),
                    lift_mm: 0.0,
                    beta_deg: 80.0,
                },
                Segment {
                    kind: "fall".into(),
                    law: "poly345".into(),
                    lift_mm: 20.0,
                    beta_deg: 100.0,
                },
                Segment {
                    kind: "dwell".into(),
                    law: "cycloidal".into(),
                    lift_mm: 0.0,
                    beta_deg: 80.0,
                },
            ],
            base_r_mm: 40.0,
            roller_r_mm: 8.0,
            offset_mm: 0.0,
            rpm: 60.0,
            samples: 720,
        }
    }
}

/// Normalised law and derivatives in x ∈ [0, 1]: (s, s', s'', s''') per unit lift, per unit x.
pub fn law(name: &str, x: f64) -> [f64; 4] {
    let x = x.clamp(0.0, 1.0);
    match name {
        "poly345" => [
            10.0 * x.powi(3) - 15.0 * x.powi(4) + 6.0 * x.powi(5),
            30.0 * x * x - 60.0 * x.powi(3) + 30.0 * x.powi(4),
            60.0 * x - 180.0 * x * x + 120.0 * x.powi(3),
            60.0 - 360.0 * x + 360.0 * x * x,
        ],
        "harmonic" => [
            0.5 * (1.0 - (PI * x).cos()),
            0.5 * PI * (PI * x).sin(),
            0.5 * PI * PI * (PI * x).cos(),
            -0.5 * PI.powi(3) * (PI * x).sin(),
        ],
        _ => [
            x - (2.0 * PI * x).sin() / (2.0 * PI),
            1.0 - (2.0 * PI * x).cos(),
            2.0 * PI * (2.0 * PI * x).sin(),
            4.0 * PI * PI * (2.0 * PI * x).cos(),
        ],
    }
}

/// Peak |acceleration| coefficient C_a with a_max = C_a h/β².
pub fn peak_accel_coeff(name: &str) -> f64 {
    match name {
        "poly345" => 10.0 / 3f64.sqrt(),
        "harmonic" => PI * PI / 2.0,
        _ => 2.0 * PI,
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CamOut {
    pub theta_deg: Vec<f64>,
    /// Displacement mm; velocity mm/rad; acceleration mm/rad²; jerk mm/rad³.
    pub s: Vec<f64>,
    pub v: Vec<f64>,
    pub a: Vec<f64>,
    pub j: Vec<f64>,
    /// Same in time units at the given speed: mm/s, mm/s², mm/s³.
    pub v_t: Vec<f64>,
    pub a_t: Vec<f64>,
    pub j_t: Vec<f64>,
    pub pressure_deg: Vec<f64>,
    /// Pitch-curve and cam-surface radius of curvature (positive = convex), mm (clamped to ±1e4).
    pub rho_pitch: Vec<f64>,
    pub rho_cam: Vec<f64>,
    /// Pitch curve and cam profile in the cam frame (θ = 0), mm.
    pub pitch_x: Vec<f64>,
    pub pitch_y: Vec<f64>,
    pub cam_x: Vec<f64>,
    pub cam_y: Vec<f64>,
    pub prime_r: f64,
    pub d_mm: f64,
    pub max_pressure_deg: f64,
    pub min_rho_convex_mm: f64,
    pub min_rho_concave_mm: f64,
    pub undercut: bool,
    pub max_v: f64,
    pub max_a: f64,
    pub max_j: f64,
    /// Per segment: kind, law, β, h, peak |a| (mm/rad²) and its closed form C_a h/β².
    pub segments: Vec<(String, String, f64, f64, f64, f64)>,
    pub accel_continuous: bool,
    pub boundaries_deg: Vec<f64>,
    pub notes: Vec<String>,
}

pub fn cam(i: CamIn) -> Result<CamOut, String> {
    if i.segments.is_empty() {
        return Err("add at least one segment".into());
    }
    let total: f64 = i.segments.iter().map(|s| s.beta_deg).sum();
    if (total - 360.0).abs() > 1e-6 {
        return Err(format!("segment angles add up to {total:.1}°, not 360°"));
    }
    if i.segments.iter().any(|s| s.beta_deg <= 0.0) {
        return Err("every segment needs a positive angle".into());
    }
    let rp = i.base_r_mm + i.roller_r_mm;
    let eps = i.offset_mm;
    if rp <= eps.abs() || i.base_r_mm <= 0.0 || i.roller_r_mm < 0.0 {
        return Err("need R_b > 0 and |offset| < prime radius".into());
    }
    let d = (rp * rp - eps * eps).sqrt();
    // lift levels
    let mut levels = vec![];
    let mut lv = 0.0;
    for s in &i.segments {
        levels.push(lv);
        match s.kind.as_str() {
            "rise" => lv += s.lift_mm,
            "fall" => lv -= s.lift_mm,
            _ => {}
        }
    }
    if lv.abs() > 1e-6 {
        return Err(format!("rises and falls do not return to the start (net {lv:.2} mm)"));
    }
    let n = i.samples.clamp(90, 3600);
    let w = i.rpm * 2.0 * PI / 60.0;
    let mut o = CamOut {
        prime_r: rp,
        d_mm: d,
        ..Default::default()
    };
    let eval = |th: f64| -> [f64; 4] {
        let mut start = 0.0;
        for (k, s) in i.segments.iter().enumerate() {
            let end = start + s.beta_deg;
            if th < end - 1e-12 || k == i.segments.len() - 1 {
                let beta = s.beta_deg * DEG;
                let x = (th - start) / s.beta_deg;
                let base = levels[k];
                return match s.kind.as_str() {
                    "rise" | "fall" => {
                        let sg = if s.kind == "rise" { 1.0 } else { -1.0 };
                        let l = law(&s.law, x);
                        let h = s.lift_mm;
                        [
                            base + sg * h * l[0],
                            sg * h * l[1] / beta,
                            sg * h * l[2] / (beta * beta),
                            sg * h * l[3] / (beta * beta * beta),
                        ]
                    }
                    _ => [base, 0.0, 0.0, 0.0],
                };
            }
            start = end;
        }
        [0.0; 4]
    };
    for k in 0..n {
        let tdeg = k as f64 * 360.0 / n as f64;
        let [s, v, a, j] = eval(tdeg);
        o.theta_deg.push(tdeg);
        o.s.push(s);
        o.v.push(v);
        o.a.push(a);
        o.j.push(j);
        o.v_t.push(v * w);
        o.a_t.push(a * w * w);
        o.j_t.push(j * w * w * w);
        let pa = ((v - eps) / (d + s)).atan() / DEG;
        o.pressure_deg.push(pa);
        // derivatives of the pitch curve in follower coordinates (see module docs)
        let p1 = [d + s, v - eps];
        let p2 = [2.0 * v - eps, a - d - s];
        let cross = p1[0] * p2[1] - p1[1] * p2[0];
        let sp = p1[0].hypot(p1[1]);
        let rho_p = if cross.abs() < 1e-12 {
            1e4
        } else {
            (-(sp * sp * sp) / cross).clamp(-1e4, 1e4)
        };
        o.rho_pitch.push(rho_p);
        o.rho_cam.push((rho_p - i.roller_r_mm).clamp(-1e4, 1e4));
        // cam frame (rotate follower-frame point by −θ)
        let th = tdeg * DEG;
        let (sn, cs) = th.sin_cos();
        let q = [eps, d + s];
        let px = cs * q[0] + sn * q[1];
        let py = -sn * q[0] + cs * q[1];
        // tangent in cam frame: R(−θ)·p1, outward normal = J·t̂
        let tx = cs * p1[0] + sn * p1[1];
        let ty = -sn * p1[0] + cs * p1[1];
        let tn = tx.hypot(ty).max(1e-12);
        let (nx, ny) = (-ty / tn, tx / tn);
        o.pitch_x.push(px);
        o.pitch_y.push(py);
        o.cam_x.push(px - i.roller_r_mm * nx);
        o.cam_y.push(py - i.roller_r_mm * ny);
    }
    o.max_pressure_deg = o.pressure_deg.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    o.min_rho_convex_mm = o
        .rho_pitch
        .iter()
        .filter(|r| **r > 0.0)
        .cloned()
        .fold(f64::INFINITY, f64::min)
        .min(1e4);
    o.min_rho_concave_mm = o
        .rho_pitch
        .iter()
        .filter(|r| **r < 0.0)
        .map(|r| r.abs())
        .fold(f64::INFINITY, f64::min)
        .min(1e4);
    o.undercut = o.min_rho_convex_mm < i.roller_r_mm;
    o.max_v = o.v.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    o.max_a = o.a.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    o.max_j = o.j.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    // acceleration continuity at boundaries
    let mut start = 0.0;
    let mut cont = true;
    for (k, s) in i.segments.iter().enumerate() {
        o.boundaries_deg.push(start);
        let beta = s.beta_deg * DEG;
        let (pk, cf) = if s.kind == "dwell" {
            (0.0, 0.0)
        } else {
            let cf = peak_accel_coeff(&s.law) * s.lift_mm / (beta * beta);
            let mut pk = 0.0f64;
            for q in 0..=400 {
                pk = pk.max((law(&s.law, q as f64 / 400.0)[2] * s.lift_mm / (beta * beta)).abs());
            }
            (pk, cf)
        };
        o.segments.push((s.kind.clone(), s.law.clone(), s.beta_deg, s.lift_mm, pk, cf));
        let end_a = if s.kind == "dwell" {
            0.0
        } else {
            law(&s.law, 1.0)[2] * s.lift_mm / (beta * beta)
        };
        let next = &i.segments[(k + 1) % i.segments.len()];
        let nb = next.beta_deg * DEG;
        let start_a = if next.kind == "dwell" {
            0.0
        } else {
            law(&next.law, 0.0)[2] * next.lift_mm / (nb * nb)
        };
        let sg_end = if s.kind == "fall" { -1.0 } else { 1.0 };
        let sg_next = if next.kind == "fall" { -1.0 } else { 1.0 };
        if (sg_end * end_a - sg_next * start_a).abs() > 1e-6 * (1.0 + o.max_a) {
            cont = false;
        }
        start += s.beta_deg;
    }
    o.accel_continuous = cont;
    if !cont {
        o.notes.push("Acceleration jumps at a segment boundary (infinite jerk): the harmonic law next to a dwell violates the fundamental law of cam design — use cycloidal or 3-4-5.".into());
    }
    if o.max_pressure_deg > 30.0 {
        o.notes.push(format!(
            "Pressure angle reaches {:.1}° (> 30°): the follower may jam in its guide — enlarge the base circle or use an offset.",
            o.max_pressure_deg
        ));
    }
    if o.undercut {
        o.notes.push(format!(
            "Undercut: the pitch curve's convex radius {:.2} mm is smaller than the roller ({:.2} mm) — the cam cannot be made; enlarge the base circle or use a smaller roller.",
            o.min_rho_convex_mm, i.roller_r_mm
        ));
    } else if o.min_rho_convex_mm < 2.0 * i.roller_r_mm {
        o.notes
            .push("Convex pitch radius is under 2× the roller radius — Norton recommends 2–3× for low contact stress.".into());
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycloidal_peak_acceleration() {
        let o = cam(CamIn::default()).unwrap();
        let (h, beta) = (20.0, 100.0 * DEG);
        let rise = &o.segments[0];
        assert!((rise.5 - 2.0 * PI * h / (beta * beta)).abs() < 1e-12);
        assert!((rise.4 - rise.5).abs() / rise.5 < 1e-4, "{} vs {}", rise.4, rise.5);
        // sampled a over the rise also peaks at 2πh/β²
        let pk =
            o.a.iter()
                .zip(&o.theta_deg)
                .filter(|(_, t)| **t < 100.0)
                .fold(0.0f64, |m, (a, _)| m.max(a.abs()));
        assert!((pk - 2.0 * PI * h / (beta * beta)).abs() / pk < 1e-3);
        // 3-4-5 closed form 5.7735 h/β²
        assert!((peak_accel_coeff("poly345") - 5.773_502_7).abs() < 1e-6);
        let fall = &o.segments[2];
        assert!((fall.4 - fall.5).abs() / fall.5 < 1e-4);
        assert!(o.accel_continuous);
    }

    #[test]
    fn law_derivatives_consistent() {
        for name in ["cycloidal", "poly345", "harmonic"] {
            let h = 1e-6;
            for x in [0.1, 0.37, 0.8] {
                let a = law(name, x);
                let p = law(name, x + h);
                let m = law(name, x - h);
                for k in 0..3 {
                    let fd = (p[k] - m[k]) / (2.0 * h);
                    assert!((fd - a[k + 1]).abs() < 1e-5 * (1.0 + a[k + 1].abs()), "{name} k{k}");
                }
            }
            let e = law(name, 1.0);
            assert!((e[0] - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn pressure_angle_and_curvature_zero_offset() {
        let o = cam(CamIn::default()).unwrap();
        // Norton eq. 8.31 with ε = 0 and the explicit ρ formula (8.33)
        for k in [30, 140, 400] {
            let (s, v, a) = (o.s[k], o.v[k], o.a[k]);
            let rp = o.prime_r;
            assert!((o.pressure_deg[k] - (v / (rp + s)).atan() / DEG).abs() < 1e-9);
            let num = ((rp + s).powi(2) + v * v).powf(1.5);
            let den = (rp + s).powi(2) + 2.0 * v * v - a * (rp + s);
            assert!((o.rho_pitch[k] - num / den).abs() < 1e-6 * num / den.abs());
        }
        // A dwell is a circular arc of radius R_p + s.
        let k = o.theta_deg.iter().position(|t| *t > 120.0).unwrap();
        assert!((o.rho_pitch[k] - (o.prime_r + 20.0)).abs() < 1e-9);
        // Profile points sit R_r inside the pitch curve.
        let dx = o.pitch_x[k] - o.cam_x[k];
        let dy = o.pitch_y[k] - o.cam_y[k];
        assert!((dx.hypot(dy) - 8.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_bad_programs() {
        let mut i = CamIn::default();
        i.segments[1].beta_deg = 70.0;
        assert!(cam(i).is_err());
        let mut j = CamIn::default();
        j.segments[2].lift_mm = 15.0;
        assert!(cam(j).is_err());
    }
}
