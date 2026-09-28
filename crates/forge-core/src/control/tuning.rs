//! PID autotuning.
//!
//! * **Ziegler–Nichols ultimate-gain method** (J. G. Ziegler & N. B. Nichols,
//!   "Optimum settings for automatic controllers", Trans. ASME 64, 1942):
//!   the ultimate gain `Ku` is the plant's gain margin and the ultimate
//!   period `Pu = 2π/ω₁₈₀` comes from the phase-crossover frequency — i.e.
//!   the frequency-domain equivalent of turning up a P controller until the
//!   loop oscillates. PID: `Kp = 0.6 Ku, Ti = Pu/2, Td = Pu/8`.
//! * **SIMC** (S. Skogestad, "Simple analytic rules for model reduction and
//!   PID controller tuning", J. Process Control 13, 2003): reduce the plant
//!   to first/second order plus dead time with the *half rule*, then
//!   `Kc = τ1 / (k (τc + θ))`, `τI = min(τ1, 4(τc + θ))`, `τD = τ2`.
//!   The single tuning knob `τc` (the desired closed-loop time constant,
//!   a.k.a. λ in lambda tuning) defaults to `τc = θ` ("tight control").
//!   Plants with zeros or complex poles are first fitted by a two-point
//!   FOPDT step-response method (Sundaresan & Krishnaswamy, 1977).

use serde::{Deserialize, Serialize};

use super::lti::{margins, simulate, InputKind, Pid, Tf};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReducedModel {
    pub k: f64,
    pub tau1: f64,
    pub tau2: f64,
    pub theta: f64,
    pub integrating: bool,
    pub method: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Tuning {
    pub pid: Pid,
    pub method: String,
    pub notes: Vec<String>,
    pub ku: Option<f64>,
    pub pu: Option<f64>,
    pub model: Option<ReducedModel>,
    pub tau_c: Option<f64>,
}

/// Ziegler–Nichols closed-loop (ultimate sensitivity) rules.
/// `kind`: "p", "pi", "pid", "pessen" (Pessen integral rule), "no_overshoot".
pub fn ziegler_nichols(plant: &Tf, kind: &str, n: f64) -> Result<Tuning, String> {
    if plant.poles().iter().any(|p| p.re > 1e-9) {
        return Err("Ziegler–Nichols needs a stable (or integrating) plant: a P controller cannot find the ultimate gain of an unstable one".into());
    }
    let m = margins(plant);
    let (ku, wpc) = match (m.gm, m.w_pc) {
        (Some(g), Some(w)) if g > 0.0 && w > 0.0 => (g, w),
        _ => return Err("the plant's phase never reaches −180°, so there is no ultimate gain (infinite gain margin). Try SIMC, or add dead time / more poles".into()),
    };
    let pu = 2.0 * std::f64::consts::PI / wpc;
    let (kp, ti, td, label) = match kind {
        "p" => (0.5 * ku, f64::INFINITY, 0.0, "Ziegler–Nichols P"),
        "pi" => (0.45 * ku, pu / 1.2, 0.0, "Ziegler–Nichols PI"),
        "pessen" => (0.7 * ku, pu / 2.5, 0.15 * pu, "Pessen integral rule"),
        "no_overshoot" => (0.2 * ku, pu / 2.0, pu / 3.0, "ZN 'no overshoot' variant"),
        _ => (0.6 * ku, pu / 2.0, pu / 8.0, "Ziegler–Nichols PID"),
    };
    let ki = if ti.is_finite() { kp / ti } else { 0.0 };
    Ok(Tuning {
        pid: Pid { kp, ki, kd: kp * td, n },
        method: label.into(),
        notes: vec![
            format!("Ultimate gain Ku = {ku:.4} (the plant's gain margin), ultimate period Pu = {pu:.4} s"),
            "ZN targets a quarter-amplitude decay ratio — aggressive, expect ~25–60 % overshoot.".into(),
        ],
        ku: Some(ku),
        pu: Some(pu),
        model: None,
        tau_c: None,
    })
}

/// Reduce a plant to (S)OPDT form with Skogestad's half rule when all poles
/// are real and there are no finite zeros, else fit FOPDT from the step.
pub fn reduce(plant: &Tf) -> Result<ReducedModel, String> {
    let poles = plant.poles();
    let zeros = plant.zeros();
    if poles.iter().any(|p| p.re > 1e-9) {
        return Err("SIMC needs a stable or integrating plant (this one has a right-half-plane pole) — use LQR or loop-shaping instead".into());
    }
    let n_int = poles.iter().filter(|p| p.abs() < 1e-9).count();
    if n_int > 1 {
        return Err("more than one integrator: SIMC rules do not apply".into());
    }
    let all_real = poles.iter().all(|p| p.im.abs() < 1e-9);
    let integrating = n_int == 1;
    if all_real && zeros.is_empty() {
        let mut taus: Vec<f64> = poles.iter().filter(|p| p.abs() >= 1e-9).map(|p| -1.0 / p.re).collect();
        taus.sort_by(|a, b| b.partial_cmp(a).unwrap());
        // steady-state gain of the non-integrating part
        let k = if integrating {
            // k' in k'/s: lim s→0 s·G(s)
            let den_wo: Vec<f64> = plant.den[..plant.den.len() - 1].to_vec();
            plant.num.last().copied().unwrap_or(0.0) / den_wo.last().copied().unwrap_or(1.0)
        } else {
            plant.dc_gain()
        };
        let theta0 = plant.delay;
        if integrating {
            // integrator retained (τ1 = ∞); the largest remaining τ is split
            let t2 = taus.first().copied().unwrap_or(0.0);
            let rest: f64 = taus.iter().skip(1).sum();
            return Ok(ReducedModel { k, tau1: f64::INFINITY, tau2: t2, theta: theta0 + rest, integrating, method: "half rule (integrating + SOPDT)".into() });
        }
        let t1 = taus.first().copied().unwrap_or(0.0);
        let t2 = taus.get(1).copied().unwrap_or(0.0);
        let t3 = taus.get(2).copied().unwrap_or(0.0);
        let rest: f64 = taus.iter().skip(3).sum();
        // SOPDT reduction (for PID): τ2' = τ2 + τ3/2, θ = θ0 + τ3/2 + Σ rest
        return Ok(ReducedModel { k, tau1: t1, tau2: t2 + t3 / 2.0, theta: theta0 + t3 / 2.0 + rest, integrating, method: "half rule (SOPDT)".into() });
    }
    if integrating {
        return Err("integrating plant with zeros or complex poles: fit a model by hand (custom plant) first".into());
    }
    // two-point FOPDT fit from the open-loop step response
    let horizon = plant.natural_horizon() * 1.5;
    let r = simulate(plant, InputKind::Step, horizon, 6000, 0.0)?;
    let yf = *r.y.last().unwrap_or(&0.0);
    if yf.abs() < 1e-12 {
        return Err("zero steady-state gain — cannot fit FOPDT".into());
    }
    let cross = |f: f64| -> Option<f64> {
        let lvl = f * yf;
        (1..r.y.len()).find(|&i| (r.y[i] - lvl) * yf.signum() >= 0.0).map(|i| {
            let (a, b) = (r.y[i - 1], r.y[i]);
            let fr = if (b - a).abs() > 0.0 { (lvl - a) / (b - a) } else { 0.0 };
            r.t[i - 1] + fr * (r.t[i] - r.t[i - 1])
        })
    };
    let (t1, t2) = match (cross(0.353), cross(0.853)) {
        (Some(a), Some(b)) => (a, b),
        _ => return Err("step response did not rise enough to fit FOPDT".into()),
    };
    let tau = 0.67 * (t2 - t1);
    let theta = (1.3 * t1 - 0.29 * t2).max(0.0);
    Ok(ReducedModel { k: yf, tau1: tau, tau2: 0.0, theta, integrating: false, method: "two-point FOPDT fit (Sundaresan & Krishnaswamy)".into() })
}

/// SIMC PI/PID tuning. `tau_c` = desired closed-loop time constant (None → θ).
pub fn simc(plant: &Tf, pid_mode: bool, tau_c: Option<f64>, n: f64) -> Result<Tuning, String> {
    let m = reduce(plant)?;
    let mut notes = vec![format!("Model ({}) : k = {:.4}, τ1 = {}, τ2 = {:.4} s, θ = {:.4} s", m.method, m.k, if m.tau1.is_finite() { format!("{:.4} s", m.tau1) } else { "∞ (integrator)".into() }, m.tau2, m.theta)];
    // For PI the second time constant is split by the half rule as well
    let (tau1, tau2, theta) = if pid_mode {
        (m.tau1, m.tau2, m.theta)
    } else if m.tau1.is_finite() {
        (m.tau1 + m.tau2 / 2.0, 0.0, m.theta + m.tau2 / 2.0)
    } else {
        (m.tau1, 0.0, m.theta + m.tau2 / 2.0)
    };
    let tc = tau_c.filter(|v| *v > 0.0).unwrap_or_else(|| {
        if theta > 1e-9 {
            theta
        } else if tau1.is_finite() {
            // no dead time: θ-based rule undefined, pick a fast but sane target
            (tau1 / 4.0).max(1e-4)
        } else {
            tau2.max(1e-3)
        }
    });
    let (kc, ti) = if m.integrating {
        (1.0 / (m.k * (tc + theta)), 4.0 * (tc + theta))
    } else {
        (tau1 / (m.k * (tc + theta)), tau1.min(4.0 * (tc + theta)))
    };
    let td = if pid_mode { tau2 } else { 0.0 };
    // series (cascade) → parallel form
    let f = 1.0 + td / ti;
    let kp = kc * f;
    let ki = kp / (ti + td);
    let kd = kp * (ti * td / (ti + td));
    notes.push(format!("τc = {tc:.4} s{}; series form Kc = {kc:.4}, τI = {ti:.4} s, τD = {td:.4} s", if tau_c.is_none() { " (default τc = θ)" } else { "" }));
    if pid_mode && m.tau2 <= 0.0 {
        notes.push("Model has no second time constant, so SIMC gives a PI controller (τD = 0).".into());
    }
    Ok(Tuning {
        pid: Pid { kp, ki, kd, n },
        method: if pid_mode { "SIMC PID (Skogestad 2003)".into() } else { "SIMC PI (Skogestad 2003)".into() },
        notes,
        ku: None,
        pu: None,
        model: Some(m),
        tau_c: Some(tc),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::poly_mul;

    #[test]
    fn zn_on_third_order_plant() {
        // G = 1/(s+1)^3: phase −180° at ω = √3, |G| = 1/8 → Ku = 8, Pu = 2π/√3
        let g = Tf::new(vec![1.0], poly_mul(&poly_mul(&[1.0, 1.0], &[1.0, 1.0]), &[1.0, 1.0]));
        let t = ziegler_nichols(&g, "pid", 10.0).unwrap();
        assert!((t.ku.unwrap() - 8.0).abs() < 1e-3);
        assert!((t.pu.unwrap() - 2.0 * std::f64::consts::PI / 3f64.sqrt()).abs() < 1e-3);
        assert!((t.pid.kp - 4.8).abs() < 1e-2);
    }

    #[test]
    fn simc_fopdt_matches_paper() {
        // k=1, τ=1, θ=0.3, τc=θ → Kc = 1/(0.6) , τI = min(1, 2.4) = 1
        let g = Tf::new(vec![1.0], vec![1.0, 1.0]).with_delay(0.3);
        let t = simc(&g, false, None, 10.0).unwrap();
        assert!((t.pid.kp - 1.0 / 0.6).abs() < 1e-6);
        assert!((t.pid.ki - 1.0 / 0.6).abs() < 1e-6);
    }

    #[test]
    fn simc_half_rule_example() {
        // Skogestad example E1-like: 1/((s+1)(0.2s+1)) PI → τ1 = 1.1, θ = 0.1
        let g = Tf::new(vec![1.0], poly_mul(&[1.0, 1.0], &[0.2, 1.0]));
        let t = simc(&g, false, None, 10.0).unwrap();
        let kc = 1.1 / (1.0 * 0.2);
        assert!((t.pid.kp - kc).abs() < 1e-6, "{}", t.pid.kp);
    }

    #[test]
    fn simc_rejects_unstable() {
        let g = Tf::new(vec![1.0], vec![1.0, -1.0]);
        assert!(simc(&g, false, None, 10.0).is_err());
    }
}
