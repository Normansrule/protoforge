//! Linear-quadratic regulator (LQR) for the inverted pendulum on a cart.
//!
//! 1. The **nonlinear** cart–pole equations (Lagrangian, pole angle θ
//!    measured from upright, positive towards +x):
//!    ```text
//!    (M+m) ẍ + m l cosθ θ̈ − m l sinθ θ̇² = F − b ẋ
//!    m l cosθ ẍ + (I + m l²) θ̈ − m g l sinθ = 0
//!    ```
//! 2. are linearised about upright by central differences → `ẋ = A x + B u`,
//! 3. discretised with a zero-order hold at the controller rate,
//! 4. and the **discrete algebraic Riccati equation**
//!    `P = Q + AᵀPA − AᵀPB (R + BᵀPB)⁻¹ BᵀPA` is solved with the
//!    structure-preserving doubling algorithm (quadratic convergence), giving
//!    `u = −K x` with `K = (R + BᵀPB)⁻¹ BᵀPA`.
//! 5. The controller then balances the *nonlinear* plant (RK4) with force
//!    saturation and impulse pushes applied to the pole tip.
//!
//! References: Åström & Murray, *Feedback Systems* 2nd ed. (LQR in the
//! state-feedback chapter, the "balance systems" examples); B. D. O. Anderson & J. B. Moore, *Optimal
//! Control: Linear Quadratic Methods*, 1990; E. K.-W. Chu et al.,
//! "Structure-preserving algorithms for periodic discrete-time algebraic
//! Riccati equations", Int. J. Control 2004 (doubling algorithm).

use serde::{Deserialize, Serialize};

use super::mat::{self, Mat};
use crate::math::C64;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CartPoleParams {
    /// Cart mass, kg.
    pub cart_mass: f64,
    /// Pole mass, kg.
    pub pole_mass: f64,
    /// Pivot → centre of mass, m (pole is a uniform rod of length 2l).
    pub l: f64,
    /// Pole inertia about its centre of mass, kg·m².
    pub inertia: f64,
    /// Cart viscous friction, N·s/m.
    pub friction: f64,
    pub g: f64,
}

impl Default for CartPoleParams {
    fn default() -> Self {
        // Control Tutorials for MATLAB & Simulink (CTMS) inverted pendulum
        CartPoleParams {
            cart_mass: 0.5,
            pole_mass: 0.2,
            l: 0.3,
            inertia: 0.006,
            friction: 0.1,
            g: 9.81,
        }
    }
}

/// State derivative of the nonlinear cart–pole; state = [x, ẋ, θ, θ̇].
pub fn dynamics(p: &CartPoleParams, s: &[f64; 4], f: f64) -> [f64; 4] {
    let (m_c, m, l, i) = (p.cart_mass, p.pole_mass, p.l, p.inertia);
    let (st, ct) = s[2].sin_cos();
    let a11 = m_c + m;
    let a12 = m * l * ct;
    let a22 = i + m * l * l;
    let b1 = f - p.friction * s[1] + m * l * st * s[3] * s[3];
    let b2 = m * p.g * l * st;
    let det = a11 * a22 - a12 * a12;
    let xdd = (b1 * a22 - a12 * b2) / det;
    let tdd = (a11 * b2 - a12 * b1) / det;
    [s[1], xdd, s[3], tdd]
}

fn rk4(p: &CartPoleParams, s: &[f64; 4], f: f64, h: f64) -> [f64; 4] {
    let add = |a: &[f64; 4], k: &[f64; 4], c: f64| [a[0] + c * k[0], a[1] + c * k[1], a[2] + c * k[2], a[3] + c * k[3]];
    let k1 = dynamics(p, s, f);
    let k2 = dynamics(p, &add(s, &k1, h / 2.0), f);
    let k3 = dynamics(p, &add(s, &k2, h / 2.0), f);
    let k4 = dynamics(p, &add(s, &k3, h), f);
    let mut o = *s;
    for j in 0..4 {
        o[j] += h / 6.0 * (k1[j] + 2.0 * k2[j] + 2.0 * k3[j] + k4[j]);
    }
    o
}

/// Linearisation about the upright equilibrium (central differences).
pub fn linearize(p: &CartPoleParams) -> (Mat, Mat) {
    let eps = 1e-6;
    let x0 = [0.0; 4];
    let mut a = mat::zeros(4, 4);
    for j in 0..4 {
        let mut xp = x0;
        let mut xm = x0;
        xp[j] += eps;
        xm[j] -= eps;
        let fp = dynamics(p, &xp, 0.0);
        let fm = dynamics(p, &xm, 0.0);
        for i in 0..4 {
            a[i][j] = (fp[i] - fm[i]) / (2.0 * eps);
        }
    }
    let fp = dynamics(p, &x0, eps);
    let fm = dynamics(p, &x0, -eps);
    let b = (0..4).map(|i| vec![(fp[i] - fm[i]) / (2.0 * eps)]).collect();
    (a, b)
}

/// Solve the DARE with the structure-preserving doubling algorithm.
pub fn dare(a: &Mat, b: &Mat, q: &Mat, r: &Mat) -> Result<Mat, String> {
    let n = a.len();
    let rinv = mat::inv(r).ok_or("R must be invertible")?;
    let mut ak = a.clone();
    let mut gk = mat::mul(&mat::mul(b, &rinv), &mat::transpose(b));
    let mut hk = q.clone();
    for _ in 0..80 {
        let w = mat::inv(&mat::add(&mat::eye(n), &mat::mul(&gk, &hk))).ok_or("doubling step singular")?;
        let aw = mat::mul(&ak, &w);
        let a_next = mat::mul(&aw, &ak);
        let g_next = mat::add(&gk, &mat::mul(&mat::mul(&aw, &gk), &mat::transpose(&ak)));
        let h_next = mat::add(&hk, &mat::mul(&mat::mul(&mat::transpose(&ak), &hk), &mat::mul(&w, &ak)));
        let diff = mat::norm_inf(&mat::sub(&h_next, &hk)) / (1.0 + mat::norm_inf(&h_next));
        ak = a_next;
        gk = g_next;
        hk = h_next;
        if diff < 1e-13 {
            return Ok(hk);
        }
        if !hk.iter().flatten().all(|v| v.is_finite()) {
            break;
        }
    }
    if hk.iter().flatten().all(|v| v.is_finite()) {
        Ok(hk)
    } else {
        Err("Riccati iteration diverged (is (A, B) stabilisable and Q ≥ 0?)".into())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Push {
    /// Time of the push, s.
    pub t: f64,
    /// Horizontal impulse applied at the pole tip, N·s (+ pushes towards +x).
    pub impulse: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CartPoleInput {
    pub params: CartPoleParams,
    /// Diagonal of Q for [x, ẋ, θ, θ̇].
    pub q: [f64; 4],
    pub r: f64,
    /// Controller sample time, s.
    pub dt: f64,
    pub t_end: f64,
    pub theta0_deg: f64,
    pub x0: f64,
    /// Actuator limit ±F, N.
    pub f_max: f64,
    pub pushes: Vec<Push>,
    /// Cart position set-point steps: (time, x_ref).
    pub x_ref: Vec<(f64, f64)>,
    /// Frames per second of the returned trajectory.
    pub fps: f64,
}

impl Default for CartPoleInput {
    fn default() -> Self {
        CartPoleInput {
            params: CartPoleParams::default(),
            q: [10.0, 1.0, 100.0, 1.0],
            r: 0.1,
            dt: 0.01,
            t_end: 10.0,
            theta0_deg: 10.0,
            x0: 0.0,
            f_max: 20.0,
            pushes: vec![],
            x_ref: vec![],
            fps: 100.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CartPoleResult {
    pub a: Mat,
    pub b: Vec<f64>,
    /// Open-loop continuous poles.
    pub open_poles: Vec<C64>,
    /// State-feedback gain K (u = −K (x − x_ref)).
    pub k: Vec<f64>,
    /// Riccati solution P.
    pub p: Mat,
    /// Closed-loop poles mapped to continuous time, s = ln(z)/dt.
    pub closed_poles: Vec<C64>,
    pub t: Vec<f64>,
    pub x: Vec<f64>,
    pub xdot: Vec<f64>,
    pub theta: Vec<f64>,
    pub thetadot: Vec<f64>,
    pub force: Vec<f64>,
    pub xref: Vec<f64>,
    pub fell: bool,
    pub fell_at: Option<f64>,
    pub max_abs_x: f64,
    pub max_abs_force: f64,
    pub saturated_frac: f64,
    /// Integrated quadratic cost Σ (xᵀQx + uᵀRu) dt.
    pub cost: f64,
}

/// (A, B, K, P, closed-loop poles)
pub type LqrDesign = (Mat, Mat, Vec<f64>, Mat, Vec<C64>);

pub fn lqr_gain(p: &CartPoleParams, q: [f64; 4], r: f64, dt: f64) -> Result<LqrDesign, String> {
    let (a, b) = linearize(p);
    let (ad, bd) = mat::c2d(&a, &b, dt);
    let mut qm = mat::zeros(4, 4);
    for i in 0..4 {
        qm[i][i] = q[i].max(0.0);
    }
    let rm = vec![vec![r.max(1e-9)]];
    let pm = dare(&ad, &bd, &qm, &rm)?;
    let bt = mat::transpose(&bd);
    let s = mat::add(&rm, &mat::mul(&mat::mul(&bt, &pm), &bd));
    let kmat = mat::mul(&mat::inv(&s).ok_or("singular R + BᵀPB")?, &mat::mul(&mat::mul(&bt, &pm), &ad));
    let k: Vec<f64> = kmat[0].clone();
    let bk: Mat = (0..4).map(|i| (0..4).map(|j| bd[i][0] * k[j]).collect()).collect();
    let acl = mat::sub(&ad, &bk);
    let zc = mat::eig(&acl);
    let sc = zc
        .iter()
        .map(|z| {
            // s = ln(z)/dt
            let lnz = C64::new(z.abs().max(1e-300).ln(), z.arg());
            lnz.scale(1.0 / dt)
        })
        .collect();
    Ok((a, b, k, pm, sc))
}

pub fn simulate_cartpole(inp: &CartPoleInput) -> Result<CartPoleResult, String> {
    let p = inp.params;
    let dt = inp.dt.clamp(0.001, 0.1);
    let (a, b, k, pm, closed) = lqr_gain(&p, inp.q, inp.r, dt)?;
    let open = mat::eig(&a);
    let sub = 10usize;
    let h = dt / sub as f64;
    let n_ctrl = (inp.t_end.clamp(0.5, 60.0) / dt).round() as usize;
    let frame_every = ((1.0 / inp.fps.clamp(10.0, 1000.0)) / dt).round().max(1.0) as usize;
    let mut s = [inp.x0, 0.0, inp.theta0_deg.to_radians(), 0.0];
    let mut out = CartPoleResult {
        a,
        b: b.iter().map(|r| r[0]).collect(),
        open_poles: open,
        k: k.clone(),
        p: pm,
        closed_poles: closed,
        ..Default::default()
    };
    let mut pushes = inp.pushes.clone();
    pushes.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap());
    let mut push_i = 0;
    let mut xr_sched = inp.x_ref.clone();
    xr_sched.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut sat = 0usize;
    for step in 0..=n_ctrl {
        let t = step as f64 * dt;
        // apply pushes due in this control period (instantaneous impulse at the tip)
        while push_i < pushes.len() && pushes[push_i].t <= t + 1e-12 {
            let pu = pushes[push_i].impulse;
            let (m_c, m, l, i) = (p.cart_mass, p.pole_mass, p.l, p.inertia);
            let ct = s[2].cos();
            let (a11, a12, a22) = (m_c + m, m * l * ct, i + m * l * l);
            let (j1, j2) = (pu, pu * 2.0 * l * ct);
            let det = a11 * a22 - a12 * a12;
            s[1] += (j1 * a22 - a12 * j2) / det;
            s[3] += (a11 * j2 - a12 * j1) / det;
            push_i += 1;
        }
        let xr = xr_sched.iter().rev().find(|(tt, _)| *tt <= t + 1e-12).map(|v| v.1).unwrap_or(0.0);
        let err = [s[0] - xr, s[1], s[2], s[3]];
        let mut f = -(0..4).map(|j| k[j] * err[j]).sum::<f64>();
        if out.fell {
            f = 0.0;
        }
        let f_sat = f.clamp(-inp.f_max, inp.f_max);
        if (f_sat - f).abs() > 1e-9 {
            sat += 1;
        }
        out.cost += dt * ((0..4).map(|j| inp.q[j] * err[j] * err[j]).sum::<f64>() + inp.r * f_sat * f_sat);
        if step % frame_every == 0 {
            out.t.push(t);
            out.x.push(s[0]);
            out.xdot.push(s[1]);
            out.theta.push(s[2]);
            out.thetadot.push(s[3]);
            out.force.push(f_sat);
            out.xref.push(xr);
        }
        out.max_abs_x = out.max_abs_x.max(s[0].abs());
        out.max_abs_force = out.max_abs_force.max(f_sat.abs());
        if !out.fell && s[2].abs() > std::f64::consts::FRAC_PI_2 {
            out.fell = true;
            out.fell_at = Some(t);
        }
        for _ in 0..sub {
            s = rk4(&p, &s, f_sat, h);
            if out.fell {
                // pole lies on the track: clamp to horizontal
                if s[2].abs() > std::f64::consts::FRAC_PI_2 {
                    s[2] = std::f64::consts::FRAC_PI_2 * s[2].signum();
                    s[3] = 0.0;
                }
            }
        }
    }
    out.saturated_frac = sat as f64 / (n_ctrl + 1) as f64;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linearisation_matches_textbook() {
        // θ̈ = m g l (M+m) θ / q  for b = 0 at upright (q = (M+m)(I+ml²) − (ml)²)
        let p = CartPoleParams {
            friction: 0.0,
            ..Default::default()
        };
        let (a, _) = linearize(&p);
        let q = (p.cart_mass + p.pole_mass) * (p.inertia + p.pole_mass * p.l * p.l) - (p.pole_mass * p.l).powi(2);
        let expect = p.pole_mass * p.g * p.l * (p.cart_mass + p.pole_mass) / q;
        assert!((a[3][2] - expect).abs() < 1e-5, "{} vs {expect}", a[3][2]);
    }

    #[test]
    fn dare_satisfies_riccati() {
        let p = CartPoleParams::default();
        let (a, b) = linearize(&p);
        let (ad, bd) = mat::c2d(&a, &b, 0.01);
        let mut q = mat::zeros(4, 4);
        for (i, v) in [10.0, 1.0, 100.0, 1.0].iter().enumerate() {
            q[i][i] = *v;
        }
        let r = vec![vec![0.1]];
        let pm = dare(&ad, &bd, &q, &r).unwrap();
        // residual of P = Q + AᵀPA − AᵀPB (R + BᵀPB)⁻¹ BᵀPA
        let at = mat::transpose(&ad);
        let bt = mat::transpose(&bd);
        let s = mat::add(&r, &mat::mul(&mat::mul(&bt, &pm), &bd));
        let t = mat::mul(&mat::mul(&at, &pm), &bd);
        let corr = mat::mul(&mat::mul(&t, &mat::inv(&s).unwrap()), &mat::transpose(&t));
        let rhs = mat::sub(&mat::add(&q, &mat::mul(&mat::mul(&at, &pm), &ad)), &corr);
        let res = mat::norm_inf(&mat::sub(&rhs, &pm)) / mat::norm_inf(&pm);
        assert!(res < 1e-9, "residual {res}");
    }

    #[test]
    fn lqr_stabilises_nonlinear_cartpole() {
        let inp = CartPoleInput {
            theta0_deg: 15.0,
            pushes: vec![Push { t: 4.0, impulse: 0.3 }],
            ..Default::default()
        };
        let r = simulate_cartpole(&inp).unwrap();
        assert!(!r.fell);
        assert!(r.closed_poles.iter().all(|s| s.re < 0.0));
        let n = r.theta.len();
        assert!(r.theta[n - 1].abs() < 0.5f64.to_radians(), "final θ {}", r.theta[n - 1]);
        assert!(r.x[n - 1].abs() < 0.05);
        assert!(r.open_poles.iter().any(|s| s.re > 0.0));
    }

    #[test]
    fn saturation_can_drop_the_pole() {
        let inp = CartPoleInput {
            theta0_deg: 35.0,
            f_max: 2.0,
            ..Default::default()
        };
        let r = simulate_cartpole(&inp).unwrap();
        assert!(r.fell);
    }
}
