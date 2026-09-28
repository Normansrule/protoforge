//! Lumped thermal network for the electronics pod: two nodes, the board
//! (with the chips on it) and the pod shell (the plastic that touches the
//! skin), connected to the ambient air.
//!
//! ```text
//!   P(t) ──► [board, C_b] ──R_bp──► [shell, C_p] ──R_pa──► ambient T_a
//!
//!   C_b dT_b/dt = P − (T_b − T_p)/R_bp
//!   C_p dT_p/dt = (T_b − T_p)/R_bp − (T_p − T_a)/R_pa
//! ```
//!
//! This is exactly an electrical RC ladder: temperature ↔ voltage, heat flow
//! (W) ↔ current, thermal resistance (°C/W) ↔ resistance, heat capacity
//! (J/°C) ↔ capacitance, ambient ↔ ground. With `x = [T_b − T_a, T_p − T_a]`
//! it is the linear system `ẋ = A x + b P` with
//!
//! ```text
//! A = [ −1/(R_bp C_b)        1/(R_bp C_b)            ]      b = [1/C_b, 0]ᵀ
//!     [  1/(R_bp C_p)  −(1/R_bp + 1/R_pa)/C_p        ]
//! ```
//!
//! * Steady state (ẋ = 0): `T_p = T_a + P R_pa`, `T_b = T_a + P (R_pa + R_bp)`.
//! * The eigenvalues of `A` are real and negative (the network is passive and
//!   `A` is similar to a symmetric matrix); the time constants are
//!   `τᵢ = −1/λᵢ`. The slow one (shell + air) sets how long a wearer takes to
//!   feel the pod warm up; the fast one is the board heating its own mass.
//! * For piecewise-constant power the solution over a step is exact:
//!   `x(t+h) = x_ss + e^{A h}(x(t) − x_ss)`, with the 2 × 2 matrix
//!   exponential from Sylvester's formula
//!   `e^{Ah} = (e^{λ₁h}(A − λ₂I) − e^{λ₂h}(A − λ₁I)) / (λ₁ − λ₂)`.
//!   A classical Runge–Kutta (RK4) integrator is provided as a cross-check.
//!
//! References: F. P. Incropera, D. P. DeWitt, T. L. Bergman & A. S. Lavine,
//! *Fundamentals of Heat and Mass Transfer* (7th ed., Wiley, 2011), §3.2
//! (thermal resistance) and §5.2 (lumped capacitance); skin-contact limits:
//! IEC 60601-1 applied-part temperature limits, commonly cited as 43 °C for
//! prolonged skin contact — Iris designs to 41 °C for margin.

use serde::{Deserialize, Serialize};

use super::budget::{pod_heat_mw, UsageProfile};
use crate::project::{Project, ThermalParams};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Network {
    pub r_bp: f64,
    pub r_pa: f64,
    pub c_b: f64,
    pub c_p: f64,
}

impl Network {
    pub fn from_params(t: &ThermalParams) -> Network {
        Network {
            r_bp: t.r_board_pod_c_per_w.max(1e-6),
            r_pa: t.r_pod_ambient_c_per_w.max(1e-6),
            c_b: t.c_board_j_per_c.max(1e-6),
            c_p: t.c_pod_j_per_c.max(1e-6),
        }
    }

    pub fn a(&self) -> [[f64; 2]; 2] {
        let g1 = 1.0 / self.r_bp;
        let g2 = 1.0 / self.r_pa;
        [[-g1 / self.c_b, g1 / self.c_b], [g1 / self.c_p, -(g1 + g2) / self.c_p]]
    }

    /// Eigenvalues (λ_fast, λ_slow), both negative, |λ_fast| ≥ |λ_slow|.
    pub fn eigenvalues(&self) -> (f64, f64) {
        let a = self.a();
        let tr = a[0][0] + a[1][1];
        let det = a[0][0] * a[1][1] - a[0][1] * a[1][0];
        let disc = (tr * tr - 4.0 * det).max(0.0).sqrt();
        ((tr - disc) / 2.0, (tr + disc) / 2.0)
    }

    /// Time constants (fast, slow), seconds.
    pub fn time_constants(&self) -> (f64, f64) {
        let (l1, l2) = self.eigenvalues();
        (-1.0 / l1, -1.0 / l2)
    }

    /// Steady-state rise above ambient (board, shell) for power `p` in W.
    pub fn steady(&self, p: f64) -> [f64; 2] {
        [p * (self.r_pa + self.r_bp), p * self.r_pa]
    }

    /// e^{A h} by Sylvester's formula (distinct eigenvalues).
    pub fn expm(&self, h: f64) -> [[f64; 2]; 2] {
        let a = self.a();
        let (l1, l2) = self.eigenvalues();
        if (l1 - l2).abs() < 1e-12 * l1.abs().max(1e-30) {
            // repeated eigenvalue: e^{λh}(I + (A − λI)h)
            let e = (l1 * h).exp();
            return [
                [e * (1.0 + (a[0][0] - l1) * h), e * a[0][1] * h],
                [e * a[1][0] * h, e * (1.0 + (a[1][1] - l1) * h)],
            ];
        }
        let (e1, e2) = ((l1 * h).exp(), (l2 * h).exp());
        let mut m = [[0.0; 2]; 2];
        for i in 0..2 {
            for j in 0..2 {
                let id = if i == j { 1.0 } else { 0.0 };
                m[i][j] = (e1 * (a[i][j] - l2 * id) - e2 * (a[i][j] - l1 * id)) / (l1 - l2);
            }
        }
        m
    }

    /// Exact step with constant power `p` (W).
    pub fn step_exact(&self, x: [f64; 2], p: f64, phi: &[[f64; 2]; 2]) -> [f64; 2] {
        let ss = self.steady(p);
        let d = [x[0] - ss[0], x[1] - ss[1]];
        [
            ss[0] + phi[0][0] * d[0] + phi[0][1] * d[1],
            ss[1] + phi[1][0] * d[0] + phi[1][1] * d[1],
        ]
    }

    /// One RK4 step with constant power `p` (W).
    pub fn step_rk4(&self, x: [f64; 2], p: f64, h: f64) -> [f64; 2] {
        let a = self.a();
        let f = |x: [f64; 2]| [a[0][0] * x[0] + a[0][1] * x[1] + p / self.c_b, a[1][0] * x[0] + a[1][1] * x[1]];
        let add = |x: [f64; 2], k: [f64; 2], s: f64| [x[0] + s * k[0], x[1] + s * k[1]];
        let k1 = f(x);
        let k2 = f(add(x, k1, h / 2.0));
        let k3 = f(add(x, k2, h / 2.0));
        let k4 = f(add(x, k3, h));
        [
            x[0] + h / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]),
            x[1] + h / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]),
        ]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Throttle {
    pub enabled: bool,
    /// Board temperature at which firmware throttles the display, °C.
    pub threshold_c: f64,
    /// Resume full duty once the board is this far below the threshold.
    pub hysteresis_c: f64,
    /// Display duty while throttled (e.g. dimmer / lower refresh), 0..1.
    pub throttled_duty: f64,
}

impl Default for Throttle {
    fn default() -> Self {
        Throttle {
            enabled: false,
            threshold_c: 42.0,
            hysteresis_c: 1.0,
            throttled_duty: 0.5,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ThermalConfig {
    /// Usage profile for the non-display loads (and display unless overridden).
    pub profile: UsageProfile,
    /// Display duty during the run (None = profile's). R-008 case = 1.
    pub display_duty: Option<f64>,
    /// Ambient override, °C.
    pub ambient_c: Option<f64>,
    pub duration_s: f64,
    pub dt_s: f64,
    /// 0 = average power; >0 = the display switches fully on/off with this period.
    pub pulse_period_s: f64,
    /// "ambient" (just put on) or "reference" (steady state at the reference profile).
    pub initial: String,
    /// "exact" or "rk4".
    pub integrator: String,
    pub throttle: Throttle,
    pub max_points: usize,
}

impl Default for ThermalConfig {
    fn default() -> Self {
        ThermalConfig {
            profile: UsageProfile::default(),
            display_duty: Some(1.0),
            ambient_c: None,
            duration_s: 3600.0,
            dt_s: 1.0,
            pulse_period_s: 0.0,
            initial: "ambient".into(),
            integrator: "exact".into(),
            throttle: Throttle::default(),
            max_points: 720,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ThermalResult {
    pub t_s: Vec<f64>,
    pub board_c: Vec<f64>,
    pub pod_c: Vec<f64>,
    pub power_mw: Vec<f64>,
    pub display_duty: Vec<f64>,
    /// Heat flow board → shell and shell → air, mW.
    pub q_board_pod_mw: Vec<f64>,
    pub q_pod_air_mw: Vec<f64>,
    pub throttled: Vec<bool>,
    pub ambient_c: f64,
    pub skin_limit_c: f64,
    /// Heat with the display off / fully on / at the requested duty, mW.
    pub p_display_off_mw: f64,
    pub p_display_on_mw: f64,
    pub p_avg_mw: f64,
    pub steady_board_c: f64,
    pub steady_pod_c: f64,
    pub max_board_c: f64,
    pub max_pod_c: f64,
    /// First time the shell reaches the skin limit (None = never in this run).
    pub t_skin_limit_s: Option<f64>,
    pub eigenvalues: [f64; 2],
    pub time_constants_s: [f64; 2],
    pub a_matrix: [[f64; 2]; 2],
    pub throttled_fraction: f64,
    pub mean_display_duty: f64,
    /// Largest display duty that keeps the steady shell ≤ skin limit.
    pub max_safe_display_duty: f64,
    pub network: Option<Network>,
}

/// Heat into the pod with the display at duty `d` (other loads per profile), mW.
pub fn heat_at_duty(p: &Project, profile: &UsageProfile, d: f64) -> f64 {
    let pr = UsageProfile {
        display_duty: Some(d.clamp(0.0, 1.0)),
        ..profile.clone()
    };
    pod_heat_mw(p, &pr)
}

/// Run the transient.
pub fn transient(p: &Project, cfg: &ThermalConfig) -> ThermalResult {
    let th = &p.design.thermal;
    let net = Network::from_params(th);
    let ta = cfg.ambient_c.unwrap_or(th.ambient_c);
    let p_off = heat_at_duty(p, &cfg.profile, 0.0);
    let p_on = heat_at_duty(p, &cfg.profile, 1.0);
    let duty_req = cfg
        .display_duty
        .unwrap_or_else(|| {
            let b = super::budget::budget(p, &cfg.profile);
            b.components
                .iter()
                .find(|c| Some(&c.refdes) == b.display_ref.as_ref())
                .map(|c| c.duty)
                .unwrap_or(0.0)
        })
        .clamp(0.0, 1.0);
    // heat is linear in display duty
    let heat = |d: f64| p_off + (p_on - p_off) * d;
    let dt = cfg.dt_s.clamp(0.01, 60.0);
    let n = (cfg.duration_s.clamp(1.0, 7.0 * 86400.0) / dt).ceil() as usize;
    let stride = (n / cfg.max_points.max(50)).max(1);
    let phi = net.expm(dt);
    let rk4 = cfg.integrator == "rk4";
    let mut x = if cfg.initial == "reference" {
        net.steady(super::budget::budget(p, &UsageProfile::default()).avg_mw * 1e-3)
    } else {
        [0.0, 0.0]
    };
    let mut out = ThermalResult {
        ambient_c: ta,
        skin_limit_c: th.skin_limit_c,
        p_display_off_mw: p_off,
        p_display_on_mw: p_on,
        p_avg_mw: heat(duty_req),
        network: Some(net),
        a_matrix: net.a(),
        ..Default::default()
    };
    let (l1, l2) = net.eigenvalues();
    out.eigenvalues = [l1, l2];
    let (t1, t2) = net.time_constants();
    out.time_constants_s = [t1, t2];
    let ss = net.steady(heat(duty_req) * 1e-3);
    out.steady_board_c = ta + ss[0];
    out.steady_pod_c = ta + ss[1];
    let allowed = (th.skin_limit_c - ta) / net.r_pa * 1e3;
    out.max_safe_display_duty = if p_on > p_off {
        ((allowed - p_off) / (p_on - p_off)).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let mut throttled = false;
    let (mut max_b, mut max_p) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut thr_steps = 0usize;
    let mut duty_sum = 0.0;
    for k in 0..=n {
        let t = k as f64 * dt;
        // firmware decision from the board temperature at the start of the step
        if cfg.throttle.enabled {
            let tb = ta + x[0];
            if !throttled && tb >= cfg.throttle.threshold_c {
                throttled = true;
            } else if throttled && tb < cfg.throttle.threshold_c - cfg.throttle.hysteresis_c.max(0.0) {
                throttled = false;
            }
        }
        let base = if throttled {
            duty_req.min(cfg.throttle.throttled_duty.clamp(0.0, 1.0))
        } else {
            duty_req
        };
        let d = if cfg.pulse_period_s > 0.0 {
            let ph = (t % cfg.pulse_period_s) / cfg.pulse_period_s;
            if ph < base {
                1.0
            } else {
                0.0
            }
        } else {
            base
        };
        let pw = heat(d);
        let (tb, tp) = (ta + x[0], ta + x[1]);
        max_b = max_b.max(tb);
        max_p = max_p.max(tp);
        if out.t_skin_limit_s.is_none() && tp >= th.skin_limit_c {
            out.t_skin_limit_s = Some(t);
        }
        if k % stride == 0 || k == n {
            out.t_s.push(t);
            out.board_c.push(tb);
            out.pod_c.push(tp);
            out.power_mw.push(pw);
            out.display_duty.push(d);
            out.q_board_pod_mw.push((x[0] - x[1]) / net.r_bp * 1e3);
            out.q_pod_air_mw.push(x[1] / net.r_pa * 1e3);
            out.throttled.push(throttled);
        }
        if k == n {
            break;
        }
        if throttled {
            thr_steps += 1;
        }
        duty_sum += d;
        x = if rk4 {
            net.step_rk4(x, pw * 1e-3, dt)
        } else {
            net.step_exact(x, pw * 1e-3, &phi)
        };
    }
    out.max_board_c = max_b;
    out.max_pod_c = max_p;
    out.throttled_fraction = thr_steps as f64 / n.max(1) as f64;
    out.mean_display_duty = duty_sum / n.max(1) as f64;
    out
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SweepConfig {
    pub profile: UsageProfile,
    pub display_duty: f64,
    pub ambient_c: Option<f64>,
    pub ambient_min: f64,
    pub ambient_max: f64,
    pub points: usize,
}

impl Default for SweepConfig {
    fn default() -> Self {
        SweepConfig {
            profile: UsageProfile::default(),
            display_duty: 1.0,
            ambient_c: None,
            ambient_min: 0.0,
            ambient_max: 45.0,
            points: 46,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SweepResult {
    pub ambient_c: Vec<f64>,
    /// Steady shell / board with the display at `display_duty`.
    pub pod_c: Vec<f64>,
    pub board_c: Vec<f64>,
    /// Steady shell at the reference (BOM) profile.
    pub pod_ref_c: Vec<f64>,
    /// Hottest ambient at which the shell stays ≤ the skin limit (display at duty).
    pub max_ambient_c: f64,
    pub duty: Vec<f64>,
    pub pod_vs_duty_c: Vec<f64>,
    pub board_vs_duty_c: Vec<f64>,
    pub max_safe_duty: f64,
    pub sweep_ambient_c: f64,
    pub skin_limit_c: f64,
}

/// Steady-state sweeps over ambient temperature and display duty.
pub fn sweep(p: &Project, cfg: &SweepConfig) -> SweepResult {
    let th = &p.design.thermal;
    let net = Network::from_params(th);
    let ta = cfg.ambient_c.unwrap_or(th.ambient_c);
    let p_off = heat_at_duty(p, &cfg.profile, 0.0) * 1e-3;
    let p_on = heat_at_duty(p, &cfg.profile, 1.0) * 1e-3;
    let p_ref = pod_heat_mw(p, &UsageProfile::default()) * 1e-3;
    let heat = |d: f64| p_off + (p_on - p_off) * d;
    let n = cfg.points.clamp(2, 1000);
    let amb = crate::math::linspace(cfg.ambient_min, cfg.ambient_max.max(cfg.ambient_min + 1.0), n);
    let pd = heat(cfg.display_duty.clamp(0.0, 1.0));
    let duty = crate::math::linspace(0.0, 1.0, 51);
    let allowed = (th.skin_limit_c - ta) / net.r_pa;
    SweepResult {
        pod_c: amb.iter().map(|a| a + pd * net.r_pa).collect(),
        board_c: amb.iter().map(|a| a + pd * (net.r_pa + net.r_bp)).collect(),
        pod_ref_c: amb.iter().map(|a| a + p_ref * net.r_pa).collect(),
        max_ambient_c: th.skin_limit_c - pd * net.r_pa,
        ambient_c: amb,
        pod_vs_duty_c: duty.iter().map(|&d| ta + heat(d) * net.r_pa).collect(),
        board_vs_duty_c: duty.iter().map(|&d| ta + heat(d) * (net.r_pa + net.r_bp)).collect(),
        duty,
        max_safe_duty: if p_on > p_off {
            ((allowed - p_off) / (p_on - p_off)).clamp(0.0, 1.0)
        } else {
            1.0
        },
        sweep_ambient_c: ta,
        skin_limit_c: th.skin_limit_c,
    }
}

/// Analytic worst case for R-008: steady state with the display always on.
pub fn worst_case(p: &Project) -> (f64, f64, f64) {
    let th = &p.design.thermal;
    let net = Network::from_params(th);
    let pw = heat_at_duty(p, &UsageProfile::default(), 1.0) * 1e-3;
    let ss = net.steady(pw);
    (th.ambient_c + ss[1], th.ambient_c + ss[0], pw * 1e3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_node_reaches_63_percent_at_tau() {
        // make the board node negligible: tiny C_b and R_bp → one RC with τ = R_pa C_p
        let net = Network {
            r_bp: 1e-4,
            r_pa: 50.0,
            c_b: 1e-4,
            c_p: 4.0,
        };
        let tau = 200.0;
        let (_, slow) = net.time_constants();
        assert!((slow - tau).abs() / tau < 1e-3, "{slow}");
        let p = 0.1;
        let phi = net.expm(1.0);
        let mut x = [0.0, 0.0];
        for _ in 0..200 {
            x = net.step_exact(x, p, &phi);
        }
        let frac = x[1] / (p * net.r_pa);
        assert!((frac - (1.0 - (-1.0f64).exp())).abs() < 2e-3, "{frac}");
    }

    #[test]
    fn steady_state_is_p_times_r() {
        let net = Network::from_params(&ThermalParams::default());
        let phi = net.expm(5.0);
        let mut x = [0.0, 0.0];
        for _ in 0..2000 {
            x = net.step_exact(x, 0.2, &phi);
        }
        assert!((x[1] - 0.2 * 55.0).abs() < 1e-6);
        assert!((x[0] - 0.2 * 75.0).abs() < 1e-6);
    }

    #[test]
    fn eigenvalues_match_analytic_and_expm() {
        let net = Network::from_params(&ThermalParams::default());
        let (l1, l2) = net.eigenvalues();
        // analytic: trace and determinant of A
        let a = net.a();
        let tr = a[0][0] + a[1][1];
        let det = a[0][0] * a[1][1] - a[0][1] * a[1][0];
        assert!((l1 + l2 - tr).abs() < 1e-12);
        assert!((l1 * l2 - det).abs() < 1e-15);
        // hand numbers for the Iris defaults: τ ≈ 23.6 s and 419 s
        let (t1, t2) = net.time_constants();
        assert!((t1 - 23.6).abs() < 0.2, "{t1}");
        assert!((t2 - 419.0).abs() < 1.5, "{t2}");
        // det(A) = 1/(R_bp R_pa C_b C_p)
        assert!((det - 1.0 / (20.0 * 55.0 * 1.5 * 6.0)).abs() < 1e-15);
        // e^{A h} vs a Taylor series
        let h = 7.0;
        let m = net.expm(h);
        let mut term = [[1.0, 0.0], [0.0, 1.0]];
        let mut sum = term;
        for k in 1..60 {
            let mut nt = [[0.0; 2]; 2];
            for i in 0..2 {
                for j in 0..2 {
                    nt[i][j] = (term[i][0] * a[0][j] + term[i][1] * a[1][j]) * h / k as f64;
                }
            }
            term = nt;
            for i in 0..2 {
                for j in 0..2 {
                    sum[i][j] += term[i][j];
                }
            }
        }
        for i in 0..2 {
            for j in 0..2 {
                assert!((m[i][j] - sum[i][j]).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn exact_and_rk4_agree() {
        let p = Project::iris();
        let a = transient(&p, &ThermalConfig::default());
        let b = transient(
            &p,
            &ThermalConfig {
                integrator: "rk4".into(),
                ..Default::default()
            },
        );
        for (x, y) in a.pod_c.iter().zip(&b.pod_c) {
            assert!((x - y).abs() < 1e-6);
        }
    }

    #[test]
    fn iris_worst_case_and_throttle() {
        let p = Project::iris();
        let (skin, board, pw) = worst_case(&p);
        assert!((pw - 163.6).abs() < 0.5, "{pw}");
        assert!((skin - (30.0 + pw * 1e-3 * 55.0)).abs() < 1e-9);
        assert!(skin > 38.0 && skin < 41.0, "{skin}");
        assert!(board > skin);
        // the transient approaches the steady state from below and never exceeds it
        let r = transient(
            &p,
            &ThermalConfig {
                duration_s: 4.0 * 3600.0,
                ..Default::default()
            },
        );
        assert!(r.max_pod_c <= skin + 1e-9);
        assert!((r.pod_c.last().unwrap() - skin).abs() < 0.01);
        // hot day: 35 °C ambient exceeds 41 °C unless throttled
        let hot = ThermalConfig {
            ambient_c: Some(35.0),
            duration_s: 2.0 * 3600.0,
            ..Default::default()
        };
        let free = transient(&p, &hot);
        assert!(free.max_pod_c > 41.0);
        let thr = transient(
            &p,
            &ThermalConfig {
                throttle: Throttle {
                    enabled: true,
                    threshold_c: 42.5,
                    hysteresis_c: 0.5,
                    throttled_duty: 0.3,
                },
                ..hot
            },
        );
        assert!(thr.max_pod_c < 41.0, "{}", thr.max_pod_c);
        assert!(thr.throttled_fraction > 0.1);
        assert!(thr.mean_display_duty < 1.0);
    }

    #[test]
    fn pulsed_load_averages_out() {
        let p = Project::iris();
        let r = transient(
            &p,
            &ThermalConfig {
                display_duty: Some(0.5),
                pulse_period_s: 20.0,
                duration_s: 4.0 * 3600.0,
                ..Default::default()
            },
        );
        let avg = transient(
            &p,
            &ThermalConfig {
                display_duty: Some(0.5),
                duration_s: 4.0 * 3600.0,
                ..Default::default()
            },
        );
        // shell ripple is tiny (filtered by the slow pole) and centred on the average
        assert!((r.pod_c.last().unwrap() - avg.steady_pod_c).abs() < 0.2);
    }

    #[test]
    fn sweep_consistent_with_worst_case() {
        let p = Project::iris();
        let s = sweep(&p, &SweepConfig::default());
        let (skin, _, _) = worst_case(&p);
        let i30 = s.ambient_c.iter().position(|a| (*a - 30.0).abs() < 1e-9).unwrap();
        assert!((s.pod_c[i30] - skin).abs() < 1e-9);
        assert!((s.pod_vs_duty_c[50] - skin).abs() < 1e-9);
        assert!(s.max_ambient_c > 30.0 && s.max_ambient_c < 35.0);
        assert!((s.max_safe_duty - 1.0).abs() < 1e-12);
    }
}
