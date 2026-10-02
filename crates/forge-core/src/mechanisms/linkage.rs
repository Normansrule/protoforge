//! Planar linkages: the four-bar and the slider-crank, analysed by vector
//! loop closure (Norton, *Design of Machinery*, ch. 4, 6, 7), plus
//! three-position motion synthesis (Norton ch. 5).
//!
//! **Four-bar** (Norton's notation): ground `d` (link 1, O₂→O₄), crank `a`
//! (link 2, O₂→A), coupler `b` (link 3, A→B), rocker `c` (link 4, O₄→B). The
//! loop `a e^{iθ₂} + b e^{iθ₃} − c e^{iθ₄} − d = 0` gives Freudenstein's
//! equation `K₁ cos θ₄ − K₂ cos θ₂ + K₃ = cos(θ₂ − θ₄)` with
//! `K₁ = d/a`, `K₂ = d/c`, `K₃ = (a² − b² + c² + d²)/(2ac)` (Freudenstein
//! 1955), solved by the tangent half-angle substitution for the open and
//! crossed branches. Velocities and accelerations differentiate the loop
//! (Norton eqs. 6.18, 7.12).
//!
//! **Grashof condition**: with S the shortest, L the longest and P, Q the
//! others, `S + L ≤ P + Q` means at least one link can rotate fully. The
//! Barker-style classes: crank-rocker (S adjacent to ground is the input),
//! double crank (S is the ground), rocker-crank, double rocker (S is the
//! coupler), change point (`S + L = P + Q`) and the non-Grashof triple rockers.
//!
//! **Transmission angle** μ between coupler and rocker (keep μ ≥ 40° for
//! smooth running); **toggle** positions where the crank cannot drive.

use serde::{Deserialize, Serialize};

use crate::math::C64;

const DEG: f64 = std::f64::consts::PI / 180.0;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct FourBarIn {
    /// Crank (link 2), coupler (3), rocker (4), ground (1), mm.
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    /// Ground pivot O₂ and the ground link's angle θ₁ (O₂ → O₄), mm and degrees.
    pub o2: [f64; 2],
    pub theta1_deg: f64,
    /// Coupler point: distance from A and angle from line AB, mm / degrees.
    pub p: f64,
    pub delta3_deg: f64,
    /// "open" or "crossed".
    pub branch: String,
    /// Crank speed and angular acceleration (rad/s, rad/s²).
    pub omega2: f64,
    pub alpha2: f64,
    pub samples: usize,
}

impl Default for FourBarIn {
    fn default() -> Self {
        FourBarIn {
            a: 40.0,
            b: 120.0,
            c: 80.0,
            d: 100.0,
            o2: [0.0, 0.0],
            theta1_deg: 0.0,
            p: 60.0,
            delta3_deg: 30.0,
            branch: "open".into(),
            omega2: 10.0,
            alpha2: 0.0,
            samples: 360,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Grashof {
    pub s: f64,
    pub l: f64,
    pub p: f64,
    pub q: f64,
    pub s_plus_l: f64,
    pub p_plus_q: f64,
    pub grashof: bool,
    pub change_point: bool,
    /// GCRR, GCCC, GRCR, GRRR, GCP, RRR1..RRR4
    pub code: String,
    pub name: String,
    pub description: String,
    /// Can the input crank (link 2) turn a full revolution?
    pub crank_full_rotation: bool,
}

/// Grashof classification of a four-bar with the given crank/coupler/rocker/ground.
pub fn grashof(a: f64, b: f64, c: f64, d: f64) -> Grashof {
    let mut v = [(a, 2), (b, 3), (c, 4), (d, 1)];
    v.sort_by(|x, y| x.0.total_cmp(&y.0));
    let (s, l) = (v[0].0, v[3].0);
    let (p, q) = (v[1].0, v[2].0);
    let sum_sl = s + l;
    let sum_pq = p + q;
    let tol = 1e-9 * (sum_sl + sum_pq);
    let change = (sum_sl - sum_pq).abs() <= tol.max(1e-9);
    let g = sum_sl < sum_pq - tol;
    // which link is the shortest (ties: prefer ground, then crank)
    let shortest = v
        .iter()
        .filter(|x| (x.0 - s).abs() <= 1e-12 * s.max(1.0))
        .map(|x| x.1)
        .min_by_key(|id| match id {
            1 => 0,
            2 => 1,
            4 => 2,
            _ => 3,
        });
    let sid = shortest.unwrap_or(2);
    let (code, name, desc, crank) = if change {
        (
            "GCP",
            "Change point (S + L = P + Q)",
            "All links can line up flat: the linkage can flip between open and crossed branches unpredictably at the flat position — add a guide or avoid.",
            true,
        )
    } else if g {
        match sid {
            1 => (
                "GCCC",
                "Grashof double crank (drag link)",
                "The shortest link is the ground: both crank and rocker turn fully.",
                true,
            ),
            2 => (
                "GCRR",
                "Grashof crank-rocker",
                "The shortest link is the input crank: it turns fully and the output rocks between two toggle limits.",
                true,
            ),
            4 => (
                "GRRC",
                "Grashof rocker-crank",
                "The shortest link is the output: the input rocks, the output turns fully (drive it from the other side).",
                false,
            ),
            _ => (
                "GRCR",
                "Grashof double rocker (coupler crank)",
                "The shortest link is the coupler: it can turn fully relative to the others, but both grounded links rock.",
                false,
            ),
        }
    } else {
        let code = match v[3].1 {
            1 => "RRR1",
            2 => "RRR2",
            3 => "RRR3",
            _ => "RRR4",
        };
        (
            code,
            "Non-Grashof triple rocker",
            "No link turns fully: every grounded link rocks between toggle positions (the class number says which link is longest).",
            false,
        )
    };
    Grashof {
        s,
        l,
        p,
        q,
        s_plus_l: sum_sl,
        p_plus_q: sum_pq,
        grashof: g || change,
        change_point: change,
        code: code.into(),
        name: name.into(),
        description: desc.into(),
        crank_full_rotation: crank,
    }
}

/// Positions θ₃, θ₄ (radians) for crank angle θ₂ by Freudenstein's equation.
/// `open` selects the open branch (Norton's minus root for θ₄).
pub fn position(a: f64, b: f64, c: f64, d: f64, th2: f64, open: bool) -> Option<(f64, f64)> {
    let k1 = d / a;
    let k2 = d / c;
    let k3 = (a * a - b * b + c * c + d * d) / (2.0 * a * c);
    let k4 = d / b;
    let k5 = (c * c - d * d - a * a - b * b) / (2.0 * a * b);
    let (s2, c2) = th2.sin_cos();
    let aa = c2 - k1 - k2 * c2 + k3;
    let bb = -2.0 * s2;
    let cc = k1 - (k2 + 1.0) * c2 + k3;
    let disc4 = bb * bb - 4.0 * aa * cc;
    let dd = c2 - k1 + k4 * c2 + k5;
    let ee = -2.0 * s2;
    let ff = k1 + (k4 - 1.0) * c2 + k5;
    let disc3 = ee * ee - 4.0 * dd * ff;
    let eps = -1e-9;
    if disc4 < eps || disc3 < eps {
        return None;
    }
    let sgn = if open { -1.0 } else { 1.0 };
    let wrap = |x: f64| {
        let t = x.rem_euclid(2.0 * std::f64::consts::PI);
        if t > std::f64::consts::PI {
            t - 2.0 * std::f64::consts::PI
        } else {
            t
        }
    };
    let th4 = wrap(2.0 * (-bb + sgn * disc4.max(0.0).sqrt()).atan2(2.0 * aa));
    let th3 = wrap(2.0 * (-ee + sgn * disc3.max(0.0).sqrt()).atan2(2.0 * dd));
    Some((th3, th4))
}

/// Angular velocities (ω₃, ω₄) — Norton eq. 6.18.
pub fn velocity(a: f64, b: f64, c: f64, th2: f64, th3: f64, th4: f64, w2: f64) -> (f64, f64) {
    let w3 = a * w2 / b * (th4 - th2).sin() / (th3 - th4).sin();
    let w4 = a * w2 / c * (th2 - th3).sin() / (th4 - th3).sin();
    (w3, w4)
}

/// Angular accelerations (α₃, α₄) — Norton eq. 7.12.
#[allow(clippy::too_many_arguments)]
pub fn acceleration(a: f64, b: f64, c: f64, th: [f64; 3], w: [f64; 3], al2: f64) -> (f64, f64) {
    let [th2, th3, th4] = th;
    let [w2, w3, w4] = w;
    let aa = c * th4.sin();
    let bb = b * th3.sin();
    let cc = a * al2 * th2.sin() + a * w2 * w2 * th2.cos() + b * w3 * w3 * th3.cos() - c * w4 * w4 * th4.cos();
    let dd = c * th4.cos();
    let ee = b * th3.cos();
    let ff = a * al2 * th2.cos() - a * w2 * w2 * th2.sin() - b * w3 * w3 * th3.sin() + c * w4 * w4 * th4.sin();
    let den = aa * ee - bb * dd;
    ((cc * dd - aa * ff) / den, (cc * ee - bb * ff) / den)
}

/// Transmission angle μ ∈ [0°, 90°] between coupler and rocker, degrees.
pub fn transmission_deg(th3: f64, th4: f64) -> f64 {
    let mut mu = (th4 - th3).rem_euclid(std::f64::consts::PI);
    if mu > std::f64::consts::FRAC_PI_2 {
        mu = std::f64::consts::PI - mu;
    }
    mu / DEG
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct FourBarOut {
    pub grashof: Grashof,
    /// Crank angle θ₂ (degrees, relative to the ground link) per sample.
    pub theta2_deg: Vec<f64>,
    pub valid: Vec<bool>,
    pub theta3_deg: Vec<f64>,
    pub theta4_deg: Vec<f64>,
    pub omega3: Vec<f64>,
    pub omega4: Vec<f64>,
    pub alpha3: Vec<f64>,
    pub alpha4: Vec<f64>,
    pub mu_deg: Vec<f64>,
    /// World coordinates: crank pin A, rocker pin B, coupler point P (mm).
    pub ax: Vec<f64>,
    pub ay: Vec<f64>,
    pub bx: Vec<f64>,
    pub by: Vec<f64>,
    pub px: Vec<f64>,
    pub py: Vec<f64>,
    /// Coupler-point speed and acceleration magnitude (mm/s, mm/s²).
    pub p_speed: Vec<f64>,
    pub p_accel: Vec<f64>,
    pub o2: [f64; 2],
    pub o4: [f64; 2],
    pub mu_min_deg: f64,
    pub mu_max_deg: f64,
    /// Crank angles (deg) where the linkage toggles (coupler and rocker in line) — non-Grashof or rocker input.
    pub toggle_theta2_deg: Vec<f64>,
    /// Rocker swing (deg) and time ratio (crank-rocker only).
    pub rocker_min_deg: f64,
    pub rocker_max_deg: f64,
    pub time_ratio: f64,
    pub notes: Vec<String>,
}

fn rot(p: [f64; 2], th: f64) -> [f64; 2] {
    let (s, c) = th.sin_cos();
    [c * p[0] - s * p[1], s * p[0] + c * p[1]]
}

pub fn fourbar(i: FourBarIn) -> Result<FourBarOut, String> {
    let (a, b, c, d) = (i.a, i.b, i.c, i.d);
    for (v, n) in [(a, "a"), (b, "b"), (c, "c"), (d, "d")] {
        if v.is_nan() || v <= 0.0 {
            return Err(format!("link {n} must be positive"));
        }
    }
    let g = grashof(a, b, c, d);
    let open = i.branch != "crossed";
    let n = i.samples.clamp(36, 1440);
    let th1 = i.theta1_deg * DEG;
    let o2 = i.o2;
    let o4d = rot([d, 0.0], th1);
    let o4 = [o2[0] + o4d[0], o2[1] + o4d[1]];
    let mut o = FourBarOut {
        grashof: g,
        o2,
        o4,
        ..Default::default()
    };
    let d3 = i.delta3_deg * DEG;
    for k in 0..n {
        let t2 = k as f64 * 360.0 / n as f64;
        let th2 = t2 * DEG;
        o.theta2_deg.push(t2);
        match position(a, b, c, d, th2, open) {
            Some((th3, th4)) => {
                let (w3, w4) = velocity(a, b, c, th2, th3, th4, i.omega2);
                let (al3, al4) = acceleration(a, b, c, [th2, th3, th4], [i.omega2, w3, w4], i.alpha2);
                let pa = rot([a * th2.cos(), a * th2.sin()], th1);
                let pb = rot([a * th2.cos() + b * th3.cos(), a * th2.sin() + b * th3.sin()], th1);
                let pp_local = [a * th2.cos() + i.p * (th3 + d3).cos(), a * th2.sin() + i.p * (th3 + d3).sin()];
                let pp = rot(pp_local, th1);
                // coupler point velocity/acceleration (local frame; magnitudes are frame-free)
                let vx = -a * i.omega2 * th2.sin() - i.p * w3 * (th3 + d3).sin();
                let vy = a * i.omega2 * th2.cos() + i.p * w3 * (th3 + d3).cos();
                let ax = -a * i.alpha2 * th2.sin()
                    - a * i.omega2 * i.omega2 * th2.cos()
                    - i.p * al3 * (th3 + d3).sin()
                    - i.p * w3 * w3 * (th3 + d3).cos();
                let ay = a * i.alpha2 * th2.cos() - a * i.omega2 * i.omega2 * th2.sin() + i.p * al3 * (th3 + d3).cos()
                    - i.p * w3 * w3 * (th3 + d3).sin();
                o.valid.push(true);
                o.theta3_deg.push(th3 / DEG);
                o.theta4_deg.push(th4 / DEG);
                o.omega3.push(w3);
                o.omega4.push(w4);
                o.alpha3.push(al3);
                o.alpha4.push(al4);
                o.mu_deg.push(transmission_deg(th3, th4));
                o.ax.push(o2[0] + pa[0]);
                o.ay.push(o2[1] + pa[1]);
                o.bx.push(o2[0] + pb[0]);
                o.by.push(o2[1] + pb[1]);
                o.px.push(o2[0] + pp[0]);
                o.py.push(o2[1] + pp[1]);
                o.p_speed.push(vx.hypot(vy));
                o.p_accel.push(ax.hypot(ay));
            }
            None => {
                o.valid.push(false);
                for v in [
                    &mut o.theta3_deg,
                    &mut o.theta4_deg,
                    &mut o.omega3,
                    &mut o.omega4,
                    &mut o.alpha3,
                    &mut o.alpha4,
                    &mut o.mu_deg,
                    &mut o.ax,
                    &mut o.ay,
                    &mut o.bx,
                    &mut o.by,
                    &mut o.px,
                    &mut o.py,
                    &mut o.p_speed,
                    &mut o.p_accel,
                ] {
                    v.push(0.0);
                }
            }
        }
    }
    let mus: Vec<f64> = o.mu_deg.iter().zip(&o.valid).filter(|(_, v)| **v).map(|(m, _)| *m).collect();
    o.mu_min_deg = mus.iter().cloned().fold(f64::INFINITY, f64::min);
    o.mu_max_deg = mus.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if mus.is_empty() {
        o.mu_min_deg = 0.0;
        o.mu_max_deg = 0.0;
        o.notes
            .push("These link lengths cannot be assembled (no crank angle closes the loop).".into());
    }
    // Toggle positions of the crank: coupler and rocker collinear, |AO₄| = b ± c.
    for bc in [b + c, (b - c).abs()] {
        let cs = (a * a + d * d - bc * bc) / (2.0 * a * d);
        if cs.abs() <= 1.0 {
            let t = cs.acos() / DEG;
            for v in [t, 360.0 - t] {
                if !o.toggle_theta2_deg.iter().any(|x: &f64| (x - v).abs() < 1e-6) {
                    o.toggle_theta2_deg.push(v);
                }
            }
        }
    }
    o.toggle_theta2_deg.sort_by(|x, y| x.total_cmp(y));
    // Rocker limits (crank-rocker): crank and coupler collinear, |O₂B| = b ± a.
    if o.valid.iter().all(|v| *v) {
        let th4s: Vec<f64> = o.theta4_deg.clone();
        o.rocker_min_deg = th4s.iter().cloned().fold(f64::INFINITY, f64::min);
        o.rocker_max_deg = th4s.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        if o.grashof.code == "GCRR" {
            // time ratio from the two crank angles at the rocker extremes
            let (imin, imax) = extreme_indices(&th4s);
            let t_a = o.theta2_deg[imin];
            let t_b = o.theta2_deg[imax];
            let fwd = (t_a - t_b).rem_euclid(360.0);
            let back = 360.0 - fwd;
            o.time_ratio = fwd.max(back) / fwd.min(back).max(1e-9);
        }
    }
    if o.mu_min_deg < 40.0 && !mus.is_empty() {
        o.notes.push(format!(
            "Minimum transmission angle {:.1}° is below the ~40° guideline: the output will bind or jerk near that position.",
            o.mu_min_deg
        ));
    }
    Ok(o)
}

fn extreme_indices(v: &[f64]) -> (usize, usize) {
    let mut imin = 0;
    let mut imax = 0;
    for (k, x) in v.iter().enumerate() {
        if *x < v[imin] {
            imin = k;
        }
        if *x > v[imax] {
            imax = k;
        }
    }
    (imin, imax)
}

// ------------------------------------------------------------------ slider-crank

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct SliderIn {
    pub r: f64,
    pub l: f64,
    /// Offset of the slider line above the crank pivot, mm.
    pub e: f64,
    pub omega2: f64,
    pub samples: usize,
}

impl Default for SliderIn {
    fn default() -> Self {
        SliderIn {
            r: 30.0,
            l: 100.0,
            e: 0.0,
            omega2: 10.0,
            samples: 360,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SliderOut {
    pub theta2_deg: Vec<f64>,
    pub valid: Vec<bool>,
    /// Slider position (mm), velocity (mm/s), acceleration (mm/s²).
    pub x: Vec<f64>,
    pub v: Vec<f64>,
    pub a: Vec<f64>,
    /// Rod angle θ₃ (deg) and transmission angle (deg, between rod and slider line's normal).
    pub theta3_deg: Vec<f64>,
    pub mu_deg: Vec<f64>,
    pub stroke_mm: f64,
    pub x_min: f64,
    pub x_max: f64,
    pub time_ratio: f64,
    /// Peak acceleration from the exact analysis vs the classical
    /// approximation r ω² (1 + r/l) for zero offset.
    pub a_max: f64,
    pub a_approx: f64,
    pub notes: Vec<String>,
}

/// Offset slider-crank: `x = r cos θ₂ + √(l² − (r sin θ₂ − e)²)`, differentiated exactly.
pub fn slider_crank(i: SliderIn) -> Result<SliderOut, String> {
    if i.r.is_nan() || i.l.is_nan() || i.r <= 0.0 || i.l <= 0.0 {
        return Err("crank and rod must be positive".into());
    }
    if i.l <= i.r + i.e.abs() {
        return Err("the rod must be longer than crank + offset for a full rotation".into());
    }
    let n = i.samples.clamp(36, 1440);
    let w = i.omega2;
    let mut o = SliderOut::default();
    for k in 0..n {
        let t2 = k as f64 * 360.0 / n as f64;
        let th = t2 * DEG;
        let (s, c) = th.sin_cos();
        let h = i.r * s - i.e;
        let q = (i.l * i.l - h * h).sqrt();
        let x = i.r * c + q;
        // dx/dθ and d²x/dθ²
        let hp = i.r * c;
        let hpp = -i.r * s;
        let qp = -h * hp / q;
        let qpp = -(hp * hp + h * hpp) / q - (h * hp).powi(2) / q.powi(3);
        let xp = -i.r * s + qp;
        let xpp = -i.r * c + qpp;
        o.theta2_deg.push(t2);
        o.valid.push(true);
        o.x.push(x);
        o.v.push(xp * w);
        o.a.push(xpp * w * w);
        let th3 = (-h).atan2(q);
        o.theta3_deg.push(th3 / DEG);
        o.mu_deg.push(90.0 - th3.abs() / DEG);
    }
    o.x_min = o.x.iter().cloned().fold(f64::INFINITY, f64::min);
    o.x_max = o.x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    o.stroke_mm = o.x_max - o.x_min;
    let (imin, imax) = extreme_indices(&o.x);
    let fwd = (o.theta2_deg[imin] - o.theta2_deg[imax]).rem_euclid(360.0);
    o.time_ratio = fwd.max(360.0 - fwd) / fwd.min(360.0 - fwd).max(1e-9);
    o.a_max = o.a.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    o.a_approx = i.r * w * w * (1.0 + i.r / i.l);
    if i.e != 0.0 {
        o.notes
            .push("An offset makes the forward and return strokes take different crank angles (quick return).".into());
    }
    Ok(o)
}

// ------------------------------------------------------------------ synthesis

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Synth3In {
    /// Three coupler-point positions (mm) and coupler-line angles (deg).
    pub p: [[f64; 2]; 3],
    pub theta_deg: [f64; 3],
    /// Free choices: rotations of the input dyad (β₂, β₃) and output dyad (γ₂, γ₃), deg.
    pub beta_deg: [f64; 2],
    pub gamma_deg: [f64; 2],
}

impl Default for Synth3In {
    fn default() -> Self {
        Synth3In {
            p: [[0.0, 0.0], [40.0, 20.0], [80.0, 10.0]],
            theta_deg: [0.0, -20.0, -45.0],
            beta_deg: [40.0, 90.0],
            gamma_deg: [-25.0, -60.0],
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Synth3Out {
    pub o2: [f64; 2],
    pub o4: [f64; 2],
    /// Pivot positions in position 1.
    pub a1: [f64; 2],
    pub b1: [f64; 2],
    /// Resulting four-bar (ready for `mechanisms.fourbar`).
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub theta1_deg: f64,
    pub p: f64,
    pub delta3_deg: f64,
    /// Crank angle (relative to the ground link) at each position, deg.
    pub theta2_deg: [f64; 3],
    pub branch: String,
    pub grashof: Grashof,
    pub notes: Vec<String>,
}

fn solve_dyad(d2: C64, d3: C64, al2: f64, al3: f64, b2: f64, b3: f64) -> Option<(C64, C64)> {
    let one = C64::real(1.0);
    let a11 = C64::from_polar(1.0, b2) - one;
    let a12 = C64::from_polar(1.0, al2) - one;
    let a21 = C64::from_polar(1.0, b3) - one;
    let a22 = C64::from_polar(1.0, al3) - one;
    let det = a11 * a22 - a12 * a21;
    if det.abs() < 1e-12 {
        return None;
    }
    let w = (d2 * a22 - a12 * d3) * det.inv();
    let z = (a11 * d3 - d2 * a21) * det.inv();
    Some((w, z))
}

/// Three-position motion generation (Norton §5.8, eqs. 5.24–5.27): for each
/// dyad solve `W(e^{iβ₂} − 1) + Z(e^{iα₂} − 1) = δ₂` and
/// `W(e^{iβ₃} − 1) + Z(e^{iα₃} − 1) = δ₃`.
pub fn synth3(i: Synth3In) -> Result<Synth3Out, String> {
    let p = |k: usize| C64::new(i.p[k][0], i.p[k][1]);
    let d2 = p(1) - p(0);
    let d3 = p(2) - p(0);
    let al2 = (i.theta_deg[1] - i.theta_deg[0]) * DEG;
    let al3 = (i.theta_deg[2] - i.theta_deg[0]) * DEG;
    let (w, z) =
        solve_dyad(d2, d3, al2, al3, i.beta_deg[0] * DEG, i.beta_deg[1] * DEG).ok_or("input dyad is singular for these choices")?;
    let (u, s) =
        solve_dyad(d2, d3, al2, al3, i.gamma_deg[0] * DEG, i.gamma_deg[1] * DEG).ok_or("output dyad is singular for these choices")?;
    let a1 = p(0) - z;
    let b1 = p(0) - s;
    let o2 = a1 - w;
    let o4 = b1 - u;
    let ground = o4 - o2;
    let link3 = b1 - a1;
    let (a, b, c, d) = (w.abs(), link3.abs(), u.abs(), ground.abs());
    if [a, b, c, d].iter().any(|v| *v < 1e-9) {
        return Err("degenerate linkage (a zero-length link) — change the free choices".into());
    }
    let th1 = ground.arg();
    let delta3 = z.arg() - link3.arg();
    let th2_1 = w.arg() - th1;
    let th2 = [th2_1, th2_1 + i.beta_deg[0] * DEG, th2_1 + i.beta_deg[1] * DEG];
    // Which assembly branch is position 1 on?
    let th4_1 = u.arg() - th1;
    let branch = ["open", "crossed"]
        .iter()
        .min_by(|x, y| {
            let e = |br: &str| {
                position(a, b, c, d, th2_1, br == "open")
                    .map(|(_, t4)| {
                        let dd = (t4 - th4_1).rem_euclid(2.0 * std::f64::consts::PI);
                        dd.min(2.0 * std::f64::consts::PI - dd)
                    })
                    .unwrap_or(9.0)
            };
            e(x).total_cmp(&e(y))
        })
        .unwrap_or(&"open")
        .to_string();
    let g = grashof(a, b, c, d);
    let mut notes = vec![];
    if !g.grashof {
        notes.push("Non-Grashof result: the crank cannot turn fully — it must rock between the positions.".into());
    }
    // branch defect: do all three positions lie on the same branch?
    let open = branch == "open";
    for (k, t) in th2.iter().enumerate().skip(1) {
        match position(a, b, c, d, *t, open) {
            Some((t3, _)) => {
                let want = link3.arg() - th1 + (i.theta_deg[k] - i.theta_deg[0]) * DEG;
                let dd = (t3 - want).rem_euclid(2.0 * std::f64::consts::PI);
                if dd.min(2.0 * std::f64::consts::PI - dd) > 1e-3 {
                    notes.push(format!("Branch defect: position {} is on the other assembly branch.", k + 1));
                }
            }
            None => notes.push(format!(
                "Position {} cannot be reached by rotating the crank (toggle in between).",
                k + 1
            )),
        }
    }
    let wrap = |x: f64| (x / DEG).rem_euclid(360.0);
    Ok(Synth3Out {
        o2: [o2.re, o2.im],
        o4: [o4.re, o4.im],
        a1: [a1.re, a1.im],
        b1: [b1.re, b1.im],
        a,
        b,
        c,
        d,
        theta1_deg: th1 / DEG,
        p: z.abs(),
        delta3_deg: delta3 / DEG,
        theta2_deg: [wrap(th2[0]), wrap(th2[1]), wrap(th2[2])],
        branch,
        grashof: g,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grashof_classes() {
        assert_eq!(grashof(40.0, 120.0, 80.0, 100.0).code, "GCRR");
        assert_eq!(grashof(100.0, 120.0, 80.0, 40.0).code, "GCCC");
        assert_eq!(grashof(80.0, 40.0, 120.0, 100.0).code, "GRCR");
        assert!(grashof(50.0, 50.0, 50.0, 50.0).change_point);
        let n = grashof(70.0, 90.0, 75.0, 100.0); // 70 + 100 > 90 + 75
        assert!(!n.grashof);
        assert_eq!(n.code, "RRR1");
    }

    /// Hand check (Norton Example 4-1 dimensions a = 40, b = 120, c = 80,
    /// d = 100, θ₂ = 40°): intersect the circle about A (radius b) with the
    /// circle about O₄ (radius c) directly and compare both branches.
    #[test]
    fn position_matches_circle_intersection() {
        let (a, b, c, d) = (40.0, 120.0, 80.0, 100.0);
        let th2 = 40.0 * DEG;
        let pa = [a * th2.cos(), a * th2.sin()];
        let o4 = [d, 0.0];
        let (dx, dy) = (o4[0] - pa[0], o4[1] - pa[1]);
        let r = dx.hypot(dy);
        let x = (b * b - c * c + r * r) / (2.0 * r);
        let h = (b * b - x * x).sqrt();
        let base = [pa[0] + x * dx / r, pa[1] + x * dy / r];
        let cands = [
            [base[0] - h * dy / r, base[1] + h * dx / r],
            [base[0] + h * dy / r, base[1] - h * dx / r],
        ];
        for open in [true, false] {
            let (th3, th4) = position(a, b, c, d, th2, open).unwrap();
            let bpt = [pa[0] + b * th3.cos(), pa[1] + b * th3.sin()];
            let bpt2 = [d + c * th4.cos(), c * th4.sin()];
            assert!(
                (bpt[0] - bpt2[0]).abs() < 1e-9 && (bpt[1] - bpt2[1]).abs() < 1e-9,
                "loop does not close"
            );
            assert!(cands.iter().any(|q| (q[0] - bpt[0]).hypot(q[1] - bpt[1]) < 1e-9));
        }
        // Open branch by hand (circle intersection on paper): θ₃ ≈ 20.3°, θ₄ ≈ 57.3°.
        let (th3, th4) = position(a, b, c, d, th2, true).unwrap();
        assert!((th3 / DEG - 20.3).abs() < 0.15, "θ3 = {}", th3 / DEG);
        assert!((th4 / DEG - 57.3).abs() < 0.15, "θ4 = {}", th4 / DEG);
    }

    #[test]
    fn velocity_and_acceleration_match_finite_differences() {
        let (a, b, c, d) = (40.0, 120.0, 80.0, 100.0);
        let w2 = 10.0;
        let al2 = 3.0;
        let th2 = 70.0 * DEG;
        let h = 1e-5;
        let (t3, t4) = position(a, b, c, d, th2, true).unwrap();
        let (t3p, t4p) = position(a, b, c, d, th2 + h, true).unwrap();
        let (t3m, t4m) = position(a, b, c, d, th2 - h, true).unwrap();
        let (w3, w4) = velocity(a, b, c, th2, t3, t4, w2);
        assert!((w3 - w2 * (t3p - t3m) / (2.0 * h)).abs() < 1e-5);
        assert!((w4 - w2 * (t4p - t4m) / (2.0 * h)).abs() < 1e-5);
        // α = θ'' ω² + θ' α₂ with θ' = dθ/dθ₂
        let d3 = (t3p - t3m) / (2.0 * h);
        let dd3 = (t3p - 2.0 * t3 + t3m) / (h * h);
        let d4 = (t4p - t4m) / (2.0 * h);
        let dd4 = (t4p - 2.0 * t4 + t4m) / (h * h);
        let (al3, al4) = acceleration(a, b, c, [th2, t3, t4], [w2, w3, w4], al2);
        assert!((al3 - (dd3 * w2 * w2 + d3 * al2)).abs() < 1e-2, "{al3}");
        assert!((al4 - (dd4 * w2 * w2 + d4 * al2)).abs() < 1e-2, "{al4}");
    }

    #[test]
    fn crank_rocker_runs_full_circle() {
        let o = fourbar(FourBarIn::default()).unwrap();
        assert!(o.valid.iter().all(|v| *v));
        assert!(o.mu_min_deg > 0.0 && o.mu_min_deg <= o.mu_max_deg);
        assert!(o.time_ratio >= 1.0);
        let n = fourbar(FourBarIn {
            a: 70.0,
            b: 90.0,
            c: 75.0,
            d: 100.0,
            ..Default::default()
        })
        .unwrap();
        assert!(n.valid.iter().any(|v| !*v));
        assert_eq!(n.toggle_theta2_deg.len(), 2);
    }

    #[test]
    fn slider_crank_derivatives() {
        let o = slider_crank(SliderIn {
            e: 8.0,
            ..Default::default()
        })
        .unwrap();
        let n = o.x.len();
        let dth = 2.0 * std::f64::consts::PI / n as f64;
        for k in [10, 77, 200] {
            let fd_v = (o.x[k + 1] - o.x[k - 1]) / (2.0 * dth) * 10.0;
            assert!((fd_v - o.v[k]).abs() < 0.02 * o.v.iter().fold(0.0f64, |m, v| m.max(v.abs())), "v");
            let fd_a = (o.x[k + 1] - 2.0 * o.x[k] + o.x[k - 1]) / (dth * dth) * 100.0;
            assert!((fd_a - o.a[k]).abs() < 0.01 * o.a_max, "a {k}: {fd_a} vs {}", o.a[k]);
        }
        let z = slider_crank(SliderIn::default()).unwrap();
        assert!((z.stroke_mm - 60.0).abs() < 1e-6);
        assert!((z.a_max - z.a_approx).abs() / z.a_approx < 0.02);
    }

    #[test]
    fn three_position_synthesis_reaches_all_positions() {
        let i = Synth3In::default();
        let s = synth3(i.clone()).unwrap();
        let open = s.branch == "open";
        for k in 0..3 {
            let th2 = (s.theta2_deg[k]) * DEG;
            if let Some((th3, _)) = position(s.a, s.b, s.c, s.d, th2, open) {
                let th1 = s.theta1_deg * DEG;
                let loc = [
                    s.a * th2.cos() + s.p * (th3 + s.delta3_deg * DEG).cos(),
                    s.a * th2.sin() + s.p * (th3 + s.delta3_deg * DEG).sin(),
                ];
                let w = rot(loc, th1);
                let pt = [s.o2[0] + w[0], s.o2[1] + w[1]];
                if s.notes.iter().all(|n| !n.contains("Branch")) {
                    assert!((pt[0] - i.p[k][0]).hypot(pt[1] - i.p[k][1]) < 1e-6, "position {k}: {pt:?}");
                }
            }
        }
        // Position 1 is always reproduced.
        let th2 = s.theta2_deg[0] * DEG;
        let (th3, _) = position(s.a, s.b, s.c, s.d, th2, open).unwrap();
        let loc = [
            s.a * th2.cos() + s.p * (th3 + s.delta3_deg * DEG).cos(),
            s.a * th2.sin() + s.p * (th3 + s.delta3_deg * DEG).sin(),
        ];
        let w = rot(loc, s.theta1_deg * DEG);
        assert!((s.o2[0] + w[0] - i.p[0][0]).abs() < 1e-6 && (s.o2[1] + w[1] - i.p[0][1]).abs() < 1e-6);
    }
}
