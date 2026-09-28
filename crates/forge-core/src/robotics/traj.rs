//! Point-to-point trajectory generation.
//!
//! * **Trapezoidal velocity** (a.k.a. linear segments with parabolic
//!   blends): accelerate at `a`, cruise at `v`, decelerate. If
//!   `D < v²/a` the cruise phase vanishes (triangular profile,
//!   `v_peak = √(aD)`). Duration `T = D/v + v/a`.
//! * **Cubic** rest-to-rest: `q(τ) = q0 + D(3τ² − 2τ³)`, τ = t/T —
//!   peak velocity 1.5 D/T, peak acceleration 6 D/T², jumps in
//!   acceleration (infinite jerk) at both ends.
//! * **Quintic** rest-to-rest: `q(τ) = q0 + D(10τ³ − 15τ⁴ + 6τ⁵)` — zero
//!   velocity *and* acceleration at both ends; peak velocity 1.875 D/T,
//!   peak acceleration 10/√3 · D/T² ≈ 5.774 D/T².
//! * **Synchronised multi-joint moves**: every joint finishes at the
//!   slowest joint's time `T` by solving `D = v T − v²/a` for its cruise
//!   speed with its own acceleration limit.
//!
//! References: L. Biagiotti & C. Melchiorri, *Trajectory Planning for
//! Automatic Machines and Robots*, Springer 2008, Ch. 2–3; Siciliano et al.,
//! *Robotics: Modelling, Planning and Control*, §4.2; Lynch & Park,
//! *Modern Robotics*, Ch. 9.

use serde::{Deserialize, Serialize};

/// Trapezoidal (or triangular) velocity profile for a signed distance.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Trapezoid {
    pub distance: f64,
    pub v: f64,
    pub a: f64,
    pub t_acc: f64,
    pub t_total: f64,
}

impl Trapezoid {
    /// Minimum-time profile under |v| ≤ vmax, |a| ≤ amax.
    pub fn min_time(distance: f64, vmax: f64, amax: f64) -> Trapezoid {
        let d = distance.abs();
        if d < 1e-15 || vmax <= 0.0 || amax <= 0.0 {
            return Trapezoid { distance, ..Default::default() };
        }
        let (v, ta) = if d < vmax * vmax / amax {
            let v = (amax * d).sqrt();
            (v, v / amax)
        } else {
            (vmax, vmax / amax)
        };
        Trapezoid { distance, v, a: amax, t_acc: ta, t_total: d / v + ta }
    }

    /// Profile with a prescribed duration `t` (≥ the minimum time) using
    /// acceleration `amax`: solve D = v T − v²/a for v.
    pub fn with_duration(distance: f64, t: f64, amax: f64) -> Trapezoid {
        let d = distance.abs();
        if d < 1e-15 || t <= 0.0 {
            return Trapezoid { distance, t_total: t.max(0.0), ..Default::default() };
        }
        let disc = (amax * amax * t * t - 4.0 * amax * d).max(0.0);
        let v = (amax * t - disc.sqrt()) / 2.0;
        Trapezoid { distance, v, a: amax, t_acc: v / amax, t_total: t }
    }

    /// (position, velocity, acceleration) at time t (position from 0).
    pub fn eval(&self, t: f64) -> (f64, f64, f64) {
        let sgn = if self.distance < 0.0 { -1.0 } else { 1.0 };
        let d = self.distance.abs();
        if self.t_total <= 0.0 || d == 0.0 {
            return (0.0, 0.0, 0.0);
        }
        let t = t.clamp(0.0, self.t_total);
        let (ta, tt, v, a) = (self.t_acc, self.t_total, self.v, self.a);
        let (p, vv, aa) = if t < ta {
            (0.5 * a * t * t, a * t, a)
        } else if t <= tt - ta {
            (0.5 * a * ta * ta + v * (t - ta), v, 0.0)
        } else {
            let r = tt - t;
            (d - 0.5 * a * r * r, a * r, -a)
        };
        (sgn * p, sgn * vv, sgn * aa)
    }
}

/// Synchronise several joints: each moves `d[i]` with limits (v[i], a[i])
/// and all finish together at T = max minimum time.
pub fn synchronized(d: &[f64], vmax: &[f64], amax: &[f64]) -> (f64, Vec<Trapezoid>) {
    let t = d.iter().zip(vmax).zip(amax).map(|((d, v), a)| Trapezoid::min_time(*d, *v, *a).t_total).fold(0.0, f64::max);
    let profs = d.iter().zip(amax).map(|(d, a)| Trapezoid::with_duration(*d, t, *a)).collect();
    (t, profs)
}

/// Cubic rest-to-rest polynomial: (p, v, a, jerk) for distance d over T.
pub fn cubic(d: f64, t_total: f64, t: f64) -> (f64, f64, f64, f64) {
    let tt = t_total.max(1e-12);
    let s = (t / tt).clamp(0.0, 1.0);
    (d * (3.0 * s * s - 2.0 * s * s * s), d * (6.0 * s - 6.0 * s * s) / tt, d * (6.0 - 12.0 * s) / (tt * tt), -12.0 * d / (tt * tt * tt))
}

/// Quintic rest-to-rest polynomial: (p, v, a, jerk).
pub fn quintic(d: f64, t_total: f64, t: f64) -> (f64, f64, f64, f64) {
    let tt = t_total.max(1e-12);
    let s = (t / tt).clamp(0.0, 1.0);
    let (s2, s3, s4, s5) = (s * s, s * s * s, s.powi(4), s.powi(5));
    (
        d * (10.0 * s3 - 15.0 * s4 + 6.0 * s5),
        d * (30.0 * s2 - 60.0 * s3 + 30.0 * s4) / tt,
        d * (60.0 * s - 180.0 * s2 + 120.0 * s3) / (tt * tt),
        d * (60.0 - 360.0 * s + 360.0 * s2) / (tt * tt * tt),
    )
}

/// Minimum durations of cubic / quintic under velocity & acceleration limits.
pub fn poly_min_time(d: f64, vmax: f64, amax: f64) -> (f64, f64) {
    let d = d.abs();
    let cubic = (1.5 * d / vmax).max((6.0 * d / amax).sqrt());
    let quintic = (1.875 * d / vmax).max((10.0 / 3f64.sqrt() * d / amax).sqrt());
    (cubic, quintic)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfileSamples {
    pub name: String,
    pub duration: f64,
    pub t: Vec<f64>,
    pub p: Vec<f64>,
    pub v: Vec<f64>,
    pub a: Vec<f64>,
    pub j: Vec<f64>,
    pub peak_v: f64,
    pub peak_a: f64,
    pub peak_j: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ProfilesIn {
    pub distance: f64,
    pub v_max: f64,
    pub a_max: f64,
    pub samples: usize,
    /// true: all three use the trapezoid's duration; false: each its own
    /// minimum time under the same limits.
    pub same_duration: bool,
}

impl Default for ProfilesIn {
    fn default() -> Self {
        ProfilesIn { distance: 90.0, v_max: 120.0, a_max: 400.0, samples: 300, same_duration: false }
    }
}

pub fn profiles(i: &ProfilesIn) -> Vec<ProfileSamples> {
    let tr = Trapezoid::min_time(i.distance, i.v_max, i.a_max);
    let (tc, tq) = if i.same_duration { (tr.t_total, tr.t_total) } else { poly_min_time(i.distance, i.v_max, i.a_max) };
    let horizon = tr.t_total.max(tc).max(tq);
    let n = i.samples.clamp(20, 5000);
    let mut out = Vec::new();
    for (name, dur) in [("Trapezoid", tr.t_total), ("Cubic", tc), ("Quintic", tq)] {
        let mut s = ProfileSamples { name: name.into(), duration: dur, ..Default::default() };
        let mut prev_a = 0.0;
        let dt = horizon / (n - 1) as f64;
        for k in 0..n {
            let t = k as f64 * dt;
            let (p, v, a, j) = match name {
                "Trapezoid" => {
                    let (p, v, a) = tr.eval(t);
                    let j = if k == 0 { 0.0 } else { (a - prev_a) / dt };
                    (p, v, a, j)
                }
                "Cubic" => {
                    if t <= dur {
                        cubic(i.distance, dur, t)
                    } else {
                        (i.distance, 0.0, 0.0, 0.0)
                    }
                }
                _ => {
                    if t <= dur {
                        quintic(i.distance, dur, t)
                    } else {
                        (i.distance, 0.0, 0.0, 0.0)
                    }
                }
            };
            prev_a = a;
            s.t.push(t);
            s.p.push(p);
            s.v.push(v);
            s.a.push(a);
            s.j.push(j);
            s.peak_v = s.peak_v.max(v.abs());
            s.peak_a = s.peak_a.max(a.abs());
            if name != "Trapezoid" {
                s.peak_j = s.peak_j.max(j.abs());
            }
        }
        if name == "Trapezoid" {
            s.peak_j = f64::INFINITY;
        }
        out.push(s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trapezoid_area_equals_distance() {
        for &(d, v, a) in &[(10.0, 3.0, 2.0), (1.0, 3.0, 2.0), (-5.0, 1.0, 10.0)] {
            let tr = Trapezoid::min_time(d, v, a);
            // integrate velocity numerically
            let n = 20000;
            let dt = tr.t_total / n as f64;
            let area: f64 = (0..n).map(|k| tr.eval((k as f64 + 0.5) * dt).1 * dt).sum();
            assert!((area - d).abs() < 1e-6 * d.abs().max(1.0), "{area} vs {d}");
            assert!((tr.eval(tr.t_total).0 - d).abs() < 1e-12);
        }
    }

    #[test]
    fn quintic_boundary_conditions() {
        let (d, t) = (2.0, 1.5);
        let s0 = quintic(d, t, 0.0);
        let s1 = quintic(d, t, t);
        assert!(s0.0.abs() < 1e-12 && s0.1.abs() < 1e-12 && s0.2.abs() < 1e-12);
        assert!((s1.0 - d).abs() < 1e-12 && s1.1.abs() < 1e-12 && s1.2.abs() < 1e-12);
        // peak velocity 1.875 D/T at the midpoint
        assert!((quintic(d, t, t / 2.0).1 - 1.875 * d / t).abs() < 1e-12);
    }

    #[test]
    fn synchronized_joints_finish_together() {
        let (t, p) = synchronized(&[1.0, 0.2, -0.5], &[1.0, 1.0, 2.0], &[2.0, 2.0, 1.0]);
        for (tr, d) in p.iter().zip([1.0, 0.2, -0.5]) {
            assert!((tr.eval(t).0 - d).abs() < 1e-9);
            assert!(tr.v <= 2.0 + 1e-9);
        }
    }
}
