//! Iris power management as a Markov decision process, solved with tabular
//! Q-learning and checked against dynamic programming.
//!
//! **State** `s = (battery state-of-charge bucket, user context)`, context
//! ∈ {idle, glance, navigate, caption}. **Action** `a = (display mode ∈
//! {off, low-refresh, full}) × (BLE radio interval ∈ {fast, slow})`.
//!
//! **User model**: a Markov chain (1-minute steps). Non-idle contexts
//! return to idle after a mean dwell time; idle jumps to context j with a
//! probability chosen so the stationary shares match the project — the
//! non-idle share equals the display's duty cycle in `p.components`.
//!
//! **Energy** comes from the project: display = the component whose name
//! contains "display" (full = active_mw, low = active_mw × factor, off =
//! sleep_mw); SoC = the "system-on-chip" component, whose duty in each
//! (context, radio) is a relative weight scaled so that the *fixed
//! reference policy* (display full whenever the user needs it, radio always
//! fast) reproduces the SoC duty in the BOM exactly; all other parts use
//! their BOM duty-cycle average. Battery power = rail power / buck
//! efficiency; usable energy = capacity × nominal V × usable fraction.
//!
//! **Reward** `r = u(context, display, radio) − λ(soc) · P/P_ref` with a
//! battery-aware price `λ(soc) = λ₀ (1 + β (1 − soc)²)` and a penalty when
//! the battery empties. The relative weights and utilities are modelling
//! assumptions exposed in [`PowerConfig`] so students can challenge them.
//!
//! References: Sutton & Barto, *Reinforcement Learning* 2nd ed., Ch. 3–6;
//! Watkins & Dayan 1992; L. Benini, A. Bogliolo, G. De Micheli, "A survey of
//! design techniques for system-level dynamic power management", IEEE
//! Trans. VLSI 8(3), 2000 (DPM as a stochastic control problem).

use serde::{Deserialize, Serialize};

use super::grid::argmax;
use crate::math::Rng;
use crate::project::{Component, Project};

pub const CONTEXTS: [&str; 4] = ["idle", "glance", "navigate", "caption"];
pub const DISPLAY: [&str; 3] = ["off", "low", "full"];
pub const RADIO: [&str; 2] = ["fast", "slow"];
pub const NA: usize = 6;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PowerConfig {
    pub soc_buckets: usize,
    /// Minutes per decision step.
    pub step_min: f64,
    /// Split of the non-idle time between glance / navigate / caption.
    pub active_split: [f64; 3],
    /// Mean dwell (minutes) of glance / navigate / caption before returning to idle.
    pub dwell_min: [f64; 3],
    /// Low-refresh display power as a fraction of full power.
    pub low_refresh_factor: f64,
    /// Relative SoC activity per context with fast / slow radio.
    pub soc_weight_fast: [f64; 4],
    pub soc_weight_slow: [f64; 4],
    /// Utility of low-refresh display per context (full = 1 when needed).
    pub utility_low: [f64; 4],
    /// Utility penalty per context when the radio interval is slow.
    pub slow_penalty: [f64; 4],
    pub lambda: f64,
    /// Battery-aware energy price: λ(soc) = λ (1 + β (1 − soc)²).
    pub beta: f64,
    pub empty_penalty: f64,
    pub episodes: usize,
    pub horizon: usize,
    pub gamma: f64,
    pub alpha_min: f64,
    pub eps_start: f64,
    pub eps_end: f64,
    pub seed: u64,
    pub eval_runs: usize,
    /// Also solve the approximate MDP exactly with value iteration.
    pub with_dp: bool,
    /// Update every action with the same sampled next context (the user's
    /// context evolves independently of our actions, so the counterfactual
    /// energy and utility of each action are known: a post-decision-state
    /// trick that removes most of the sampling noise). false = plain
    /// one-action-per-step Q-learning.
    pub all_actions: bool,
}

impl Default for PowerConfig {
    fn default() -> Self {
        PowerConfig {
            soc_buckets: 6,
            step_min: 1.0,
            active_split: [0.40, 0.27, 0.33],
            dwell_min: [1.5, 8.0, 10.0],
            low_refresh_factor: 0.7,
            soc_weight_fast: [1.0, 1.6, 1.8, 3.5],
            soc_weight_slow: [0.35, 1.3, 1.5, 3.0],
            utility_low: [0.0, 1.0, 1.0, 0.6],
            slow_penalty: [0.0, 0.05, 0.02, 0.5],
            lambda: 0.05,
            beta: 1.0,
            empty_penalty: 5.0,
            episodes: 400,
            horizon: 240,
            gamma: 0.97,
            alpha_min: 0.002,
            eps_start: 1.0,
            eps_end: 0.1,
            seed: 11,
            eval_runs: 24,
            with_dp: false,
            all_actions: true,
        }
    }
}

/// Everything derived from the project for the MDP.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PowerModel {
    pub display_ref: String,
    pub soc_ref: String,
    pub display_mw: [f64; 3],
    /// SoC power [context][radio], mW.
    pub soc_mw: [[f64; 2]; 4],
    pub soc_duty: [[f64; 2]; 4],
    pub const_mw: f64,
    pub buck_efficiency: f64,
    pub battery_mwh: f64,
    pub shares: [f64; 4],
    /// Context transition matrix P[i][j] per step.
    pub trans: [[f64; 4]; 4],
    /// Fixed-policy (BOM duty cycle) average battery power, mW.
    pub p_ref_mw: f64,
    pub notes: Vec<String>,
}

fn find<'a>(p: &'a Project, keys: &[&str], fallback: &str) -> Option<&'a Component> {
    p.components
        .iter()
        .find(|c| {
            let n = c.name.to_lowercase();
            keys.iter().any(|k| n.contains(k)) && c.power.active_mw > 0.0
        })
        .or_else(|| p.components.iter().find(|c| c.refdes == fallback))
}

fn avg_mw(c: &Component) -> f64 {
    (c.power.active_mw * c.power.duty + c.power.sleep_mw * (1.0 - c.power.duty)) * c.qty
}

pub fn model(p: &Project, cfg: &PowerConfig) -> PowerModel {
    let mut m = PowerModel { buck_efficiency: p.design.power_rails.buck_efficiency.clamp(0.3, 1.0), ..Default::default() };
    let disp = find(p, &["display"], "U2");
    let soc = find(p, &["system-on-chip", "soc", "mcu", "microcontroller"], "U1");
    let (d_act, d_sleep, d_duty) = disp.map(|c| (c.power.active_mw * c.qty, c.power.sleep_mw * c.qty, c.power.duty)).unwrap_or((140.0, 0.05, 0.3));
    let (s_act, s_sleep, s_duty) = soc.map(|c| (c.power.active_mw * c.qty, c.power.sleep_mw * c.qty, c.power.duty)).unwrap_or((21.0, 0.006, 0.25));
    m.display_ref = disp.map(|c| c.refdes.clone()).unwrap_or_else(|| "(default)".into());
    m.soc_ref = soc.map(|c| c.refdes.clone()).unwrap_or_else(|| "(default)".into());
    m.display_mw = [d_sleep, d_act * cfg.low_refresh_factor.clamp(0.0, 1.0), d_act];
    // user model: stationary non-idle share = display duty
    let active = d_duty.clamp(0.01, 0.9);
    let split_sum: f64 = cfg.active_split.iter().sum::<f64>().max(1e-9);
    m.shares = [1.0 - active, active * cfg.active_split[0] / split_sum, active * cfg.active_split[1] / split_sum, active * cfg.active_split[2] / split_sum];
    let step = cfg.step_min.clamp(0.1, 60.0);
    let mut trans = [[0.0; 4]; 4];
    let mut out_of_idle = 0.0;
    for j in 1..4 {
        let dwell_steps = (cfg.dwell_min[j - 1] / step).max(1.0);
        // flow balance: π_idle p_j = π_j / dwell_j
        let pj = m.shares[j] / (m.shares[0] * dwell_steps);
        trans[0][j] = pj;
        out_of_idle += pj;
        trans[j][j] = 1.0 - 1.0 / dwell_steps;
        trans[j][0] = 1.0 / dwell_steps;
    }
    if out_of_idle >= 1.0 {
        for tj in trans[0].iter_mut().skip(1) {
            *tj /= out_of_idle * 1.05;
        }
        m.notes.push("idle dwell too short for the requested shares — rescaled".into());
        out_of_idle = trans[0][1..].iter().sum();
    }
    trans[0][0] = 1.0 - out_of_idle;
    m.trans = trans;
    // SoC duty calibrated so the fixed policy (always fast) matches the BOM duty
    let wavg: f64 = (0..4).map(|c| m.shares[c] * cfg.soc_weight_fast[c]).sum::<f64>().max(1e-9);
    let scale = s_duty / wavg;
    for c in 0..4 {
        for r in 0..2 {
            let w = if r == 0 { cfg.soc_weight_fast[c] } else { cfg.soc_weight_slow[c] };
            let d = (w * scale).clamp(0.0, 1.0);
            m.soc_duty[c][r] = d;
            m.soc_mw[c][r] = s_act * d + s_sleep * (1.0 - d);
        }
    }
    let skip = [disp.map(|c| c.refdes.clone()), soc.map(|c| c.refdes.clone())];
    m.const_mw = p.components.iter().filter(|c| !skip.contains(&Some(c.refdes.clone()))).map(avg_mw).sum::<f64>()
        + p.design.power_rails.quiescent_ua * 1e-3 * p.design.battery.nominal_v;
    let b = &p.design.battery;
    m.battery_mwh = b.capacity_mah * b.nominal_v * b.usable_fraction;
    m.p_ref_mw = (0..4).map(|c| m.shares[c] * battery_mw(&m, c, fixed_display(c), 0)).sum();
    m
}

pub fn battery_mw(m: &PowerModel, ctx: usize, display: usize, radio: usize) -> f64 {
    (m.display_mw[display] + m.soc_mw[ctx][radio] + m.const_mw) / m.buck_efficiency
}

fn fixed_display(ctx: usize) -> usize {
    if ctx == 0 {
        0
    } else {
        2
    }
}

pub fn utility(cfg: &PowerConfig, ctx: usize, display: usize, radio: usize) -> f64 {
    let base = match display {
        0 => 0.0,
        1 => cfg.utility_low[ctx],
        _ => {
            if ctx == 0 {
                0.0
            } else {
                1.0
            }
        }
    };
    base - if radio == 1 { cfg.slow_penalty[ctx] } else { 0.0 }
}

fn max_utility(ctx: usize) -> f64 {
    if ctx == 0 {
        0.0
    } else {
        1.0
    }
}

fn state_index(cfg: &PowerConfig, soc: f64, ctx: usize) -> usize {
    let nb = cfg.soc_buckets.clamp(2, 40);
    let b = ((soc.clamp(0.0, 0.999_999)) * nb as f64) as usize;
    b * 4 + ctx
}

fn sample_ctx(m: &PowerModel, ctx: usize, rng: &mut Rng) -> usize {
    let u = rng.uniform();
    let mut acc = 0.0;
    for j in 0..4 {
        acc += m.trans[ctx][j];
        if u < acc {
            return j;
        }
    }
    3
}

fn lambda(cfg: &PowerConfig, soc: f64) -> f64 {
    cfg.lambda * (1.0 + cfg.beta * (1.0 - soc.clamp(0.0, 1.0)).powi(2))
}

fn reward(cfg: &PowerConfig, m: &PowerModel, soc: f64, ctx: usize, display: usize, radio: usize) -> f64 {
    utility(cfg, ctx, display, radio) - lambda(cfg, soc) * battery_mw(m, ctx, display, radio) / m.p_ref_mw.max(1e-9)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Timeline {
    pub t_h: Vec<f64>,
    pub ctx: Vec<u8>,
    pub display: Vec<u8>,
    pub radio: Vec<u8>,
    pub soc: Vec<f64>,
    pub power_mw: Vec<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EvalSummary {
    pub name: String,
    pub life_h: f64,
    /// Utility delivered / utility demanded while the battery lasts.
    pub served: f64,
    /// Utility delivered in a 16 h waking day / demanded.
    pub day_utility: f64,
    pub avg_power_mw: f64,
    /// Fraction of time in each display mode.
    pub display_frac: [f64; 3],
    pub slow_radio_frac: f64,
    pub timeline: Timeline,
}

/// Evaluate a policy (soc, context) → action over `runs` seeded days that
/// share the same user behaviour for every policy (common random numbers).
fn evaluate(cfg: &PowerConfig, m: &PowerModel, name: &str, policy: &dyn Fn(f64, usize) -> usize) -> EvalSummary {
    let dt_h = cfg.step_min.clamp(0.1, 60.0) / 60.0;
    let runs = cfg.eval_runs.clamp(1, 50);
    let mut out = EvalSummary { name: name.into(), ..Default::default() };
    let day_steps = (16.0 / dt_h) as usize;
    let max_steps = (72.0 / dt_h) as usize;
    let mut steps_total = 0.0;
    for run in 0..runs {
        let mut rng = Rng::new(cfg.seed.wrapping_mul(31).wrapping_add(1000 + run as u64));
        let mut ctxs = Vec::with_capacity(max_steps);
        let mut c = 0usize;
        for _ in 0..max_steps {
            ctxs.push(c);
            c = sample_ctx(m, c, &mut rng);
        }
        let mut soc = 1.0;
        let (mut u, mut umax, mut e, mut steps) = (0.0, 0.0, 0.0, 0.0);
        let (mut du, mut dumax) = (0.0, 0.0);
        let mut life = 72.0;
        for (k, &ctx) in ctxs.iter().enumerate() {
            let a = policy(soc, ctx);
            let (disp, radio) = (a / 2, a % 2);
            let p = battery_mw(m, ctx, disp, radio);
            let de = p * dt_h / m.battery_mwh;
            let uk = utility(cfg, ctx, disp, radio);
            let frac = if soc - de <= 0.0 { soc / de } else { 1.0 };
            if run == 0 {
                let tl = &mut out.timeline;
                tl.t_h.push(k as f64 * dt_h);
                tl.ctx.push(ctx as u8);
                tl.display.push(disp as u8);
                tl.radio.push(radio as u8);
                tl.soc.push(soc);
                tl.power_mw.push(p);
            }
            u += uk * frac;
            umax += max_utility(ctx) * frac;
            e += p * frac;
            steps += frac;
            out.display_frac[disp] += frac;
            out.slow_radio_frac += if radio == 1 { frac } else { 0.0 };
            if k < day_steps {
                du += uk * frac;
                dumax += max_utility(ctx);
            }
            if frac < 1.0 {
                life = (k as f64 + frac) * dt_h;
                if run == 0 {
                    let tl = &mut out.timeline;
                    tl.t_h.push(life);
                    tl.ctx.push(ctx as u8);
                    tl.display.push(disp as u8);
                    tl.radio.push(radio as u8);
                    tl.soc.push(0.0);
                    tl.power_mw.push(p);
                }
                // utility demanded for the rest of the waking day goes unserved
                for &cc in ctxs.iter().take(day_steps).skip(k + 1) {
                    dumax += max_utility(cc);
                }
                break;
            }
            soc -= de;
        }
        steps_total += steps;
        out.life_h += life / runs as f64;
        out.served += u / umax.max(1e-9) / runs as f64;
        out.day_utility += du / dumax.max(1e-9) / runs as f64;
        out.avg_power_mw += e / steps.max(1e-9) / runs as f64;
    }
    for v in out.display_frac.iter_mut() {
        *v /= steps_total.max(1e-9);
    }
    out.slow_radio_frac /= steps_total.max(1e-9);
    out
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PowerResult {
    pub contexts: Vec<String>,
    pub displays: Vec<String>,
    pub radios: Vec<String>,
    pub model: PowerModel,
    pub soc_buckets: usize,
    /// Q[state][action], state = bucket·4 + context, action = display·2 + radio.
    pub q: Vec<[f64; NA]>,
    pub policy: Vec<u8>,
    pub visits: Vec<u32>,
    pub dp_policy: Vec<u8>,
    /// Fraction of visited states where Q-learning picks the DP action.
    pub dp_agreement: Option<f64>,
    /// Mean reward per step during training (per episode).
    pub training_curve: Vec<f64>,
    pub fixed: EvalSummary,
    pub learned: EvalSummary,
    pub dp: Option<EvalSummary>,
    pub gain_pct: f64,
    pub utility_ok: bool,
    /// Value used for rl.policy_life_gain_pct (0 if utility dropped).
    pub metric_value: f64,
}

pub fn train(p: &Project, cfg: &PowerConfig) -> PowerResult {
    let m = model(p, cfg);
    let nb = cfg.soc_buckets.clamp(2, 40);
    let ns = nb * 4;
    let dt_h = cfg.step_min.clamp(0.1, 60.0) / 60.0;
    let mut q = vec![[0.0f64; NA]; ns];
    let mut visits = vec![0u32; ns];
    let mut n_sa = vec![[0u32; NA]; ns];
    let mut rng = Rng::new(cfg.seed);
    let episodes = cfg.episodes.clamp(1, 20_000);
    let horizon = cfg.horizon.clamp(10, 5000);
    let mut curve = Vec::with_capacity(episodes);
    for ep in 0..episodes {
        let frac = ep as f64 / (episodes.max(2) - 1) as f64;
        let eps = cfg.eps_start + (cfg.eps_end - cfg.eps_start) * frac.min(1.0);
        // exploring starts: any battery level, context drawn from the stationary shares
        let mut soc = 0.02 + 0.98 * rng.uniform();
        let u = rng.uniform();
        let mut ctx = if u < m.shares[0] {
            0
        } else if u < m.shares[0] + m.shares[1] {
            1
        } else if u < 1.0 - m.shares[3] {
            2
        } else {
            3
        };
        let mut total = 0.0;
        let mut steps = 0;
        for _ in 0..horizon {
            let s = state_index(cfg, soc, ctx);
            visits[s] += 1;
            let a = if rng.uniform() < eps { rng.below(NA) } else { argmax(&q[s]) };
            let ctx2 = sample_ctx(&m, ctx, &mut rng);
            let mut r_taken = 0.0;
            let mut soc_taken = soc;
            // compute every target from the Q-table *before* updating (the
            // next state is often this same state)
            let mut targets = [f64::NAN; NA];
            for (aa, tg) in targets.iter_mut().enumerate() {
                if !cfg.all_actions && aa != a {
                    continue;
                }
                let (disp, radio) = (aa / 2, aa % 2);
                let pw = battery_mw(&m, ctx, disp, radio);
                let mut r = reward(cfg, &m, soc, ctx, disp, radio);
                let soc2 = soc - pw * dt_h / m.battery_mwh;
                *tg = if soc2 <= 0.0 {
                    r -= cfg.empty_penalty;
                    r
                } else {
                    r + cfg.gamma * q[state_index(cfg, soc2, ctx2)].iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                };
                if aa == a {
                    r_taken = r;
                    soc_taken = soc2;
                }
            }
            for (aa, tg) in targets.iter().enumerate() {
                if tg.is_nan() {
                    continue;
                }
                n_sa[s][aa] += 1;
                let alpha = (1.0 / (1.0 + 0.02 * n_sa[s][aa] as f64)).max(cfg.alpha_min);
                q[s][aa] += alpha * (tg - q[s][aa]);
            }
            total += r_taken;
            steps += 1;
            if soc_taken <= 0.0 {
                break;
            }
            soc = soc_taken;
            ctx = ctx2;
        }
        curve.push(total / steps.max(1) as f64);
    }
    // unvisited states fall back to the fixed policy's action
    let policy: Vec<u8> = (0..ns).map(|s| if visits[s] == 0 { (fixed_display(s % 4) * 2) as u8 } else { argmax(&q[s]) as u8 }).collect();
    let fixed = evaluate(cfg, &m, "Fixed (BOM duty cycle)", &|_, c| fixed_display(c) * 2);
    let pol = policy.clone();
    let learned = evaluate(cfg, &m, "Learned (Q-learning)", &|soc, c| pol[state_index(cfg, soc, c)] as usize);
    let (dp_policy, dp, dp_agreement) = if cfg.with_dp {
        let dpp = value_iteration(cfg, &m);
        let dpc = dpp.clone();
        let ev = evaluate(cfg, &m, "Dynamic programming", &|soc, c| dpc[state_index(cfg, soc, c)] as usize);
        let mut agree = 0usize;
        let mut cnt = 0usize;
        for s in 0..ns {
            if visits[s] > 20 {
                cnt += 1;
                if policy[s] == dpp[s] {
                    agree += 1;
                }
            }
        }
        (dpp, Some(ev), Some(agree as f64 / cnt.max(1) as f64))
    } else {
        (vec![], None, None)
    };
    let gain = (learned.life_h / fixed.life_h.max(1e-9) - 1.0) * 100.0;
    let utility_ok = learned.served >= fixed.served - 0.005;
    PowerResult {
        contexts: CONTEXTS.iter().map(|s| s.to_string()).collect(),
        displays: DISPLAY.iter().map(|s| s.to_string()).collect(),
        radios: RADIO.iter().map(|s| s.to_string()).collect(),
        model: m,
        soc_buckets: nb,
        q,
        policy,
        visits,
        dp_policy,
        dp_agreement,
        training_curve: curve,
        fixed,
        learned,
        dp,
        gain_pct: gain,
        utility_ok,
        metric_value: if utility_ok { gain } else { 0.0 },
    }
}

/// Value iteration on the bucketed model: the battery drops one bucket with
/// probability E_step / E_bucket (a standard way to keep a discretised
/// energy state Markov).
fn value_iteration(cfg: &PowerConfig, m: &PowerModel) -> Vec<u8> {
    let nb = cfg.soc_buckets.clamp(2, 40);
    let ns = nb * 4;
    let dt_h = cfg.step_min.clamp(0.1, 60.0) / 60.0;
    let e_bucket = m.battery_mwh / nb as f64;
    let mut v = vec![0.0f64; ns];
    let mut pol = vec![0u8; ns];
    for _ in 0..5000 {
        let mut delta: f64 = 0.0;
        let mut nv = v.clone();
        for b in 0..nb {
            let soc_mid = (b as f64 + 0.5) / nb as f64;
            for ctx in 0..4 {
                let s = b * 4 + ctx;
                let mut best = f64::NEG_INFINITY;
                let mut ba = 0;
                for a in 0..NA {
                    let (disp, radio) = (a / 2, a % 2);
                    let pw = battery_mw(m, ctx, disp, radio);
                    let pdrop = (pw * dt_h / e_bucket).min(1.0);
                    let r = reward(cfg, m, soc_mid, ctx, disp, radio);
                    let mut ev = 0.0;
                    for c2 in 0..4 {
                        let pc = m.trans[ctx][c2];
                        if pc == 0.0 {
                            continue;
                        }
                        let stay = v[b * 4 + c2];
                        let drop = if b == 0 { -cfg.empty_penalty / cfg.gamma } else { v[(b - 1) * 4 + c2] };
                        ev += pc * ((1.0 - pdrop) * stay + pdrop * drop);
                    }
                    let qv = r + cfg.gamma * ev;
                    if qv > best + 1e-12 {
                        best = qv;
                        ba = a;
                    }
                }
                delta = delta.max((best - v[s]).abs());
                nv[s] = best;
                pol[s] = ba as u8;
            }
        }
        v = nv;
        if delta < 1e-10 {
            break;
        }
    }
    pol
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_policy_reproduces_bom_average_power() {
        let p = Project::iris();
        let cfg = PowerConfig::default();
        let m = model(&p, &cfg);
        let bom: f64 = p.components.iter().map(avg_mw).sum::<f64>() + p.design.power_rails.quiescent_ua * 1e-3 * p.design.battery.nominal_v;
        let bom_batt = bom / p.design.power_rails.buck_efficiency;
        assert!((m.p_ref_mw - bom_batt).abs() < 1e-9, "{} vs {}", m.p_ref_mw, bom_batt);
        // stationary distribution of the chain equals the shares
        let mut pi = [0.25; 4];
        for _ in 0..5000 {
            let mut n = [0.0; 4];
            for i in 0..4 {
                for j in 0..4 {
                    n[j] += pi[i] * m.trans[i][j];
                }
            }
            pi = n;
        }
        for i in 0..4 {
            assert!((pi[i] - m.shares[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn learned_policy_keeps_utility_and_saves_energy() {
        let p = Project::iris();
        let r = train(&p, &PowerConfig { with_dp: true, ..Default::default() });
        assert!(r.utility_ok, "served learned {} fixed {}", r.learned.served, r.fixed.served);
        assert!(r.gain_pct >= 5.0, "gain {}", r.gain_pct);
        // fixed-policy life ≈ battery energy / BOM average power
        let expect = r.model.battery_mwh / r.model.p_ref_mw;
        assert!((r.fixed.life_h - expect).abs() / expect < 0.08, "{} vs {expect}", r.fixed.life_h);
        let dp = r.dp.unwrap();
        assert!(r.learned.life_h > 0.9 * dp.life_h || r.learned.served > dp.served);
    }

    #[test]
    fn deterministic() {
        let p = Project::iris();
        let a = train(&p, &PowerConfig::default());
        let b = train(&p, &PowerConfig::default());
        assert_eq!(a.policy, b.policy);
        assert_eq!(a.gain_pct, b.gain_pct);
    }
}

#[cfg(test)]
mod debug {
    use super::*;

    /// `cargo test -p forge-core rl::power::debug -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn dump() {
        let p = Project::iris();
        let cfg = PowerConfig { with_dp: true, ..Default::default() };
        let r = train(&p, &cfg);
        println!("model {:?}", r.model);
        for (name, pol) in [("learned", &r.policy), ("dp", &r.dp_policy)] {
            println!("== {name}");
            for c in 0..4 {
                let row: Vec<String> = (0..r.soc_buckets).map(|b| {
                    let a = pol[b * 4 + c] as usize;
                    format!("{}{}", &DISPLAY[a / 2][..1], &RADIO[a % 2][..1])
                }).collect();
                println!("{:9}: {}", CONTEXTS[c], row.join(" "));
            }
        }
        for e in [&r.fixed, &r.learned, r.dp.as_ref().unwrap()] {
            println!("{:30.30} life {:.2} served {:.4} day {:.3} P {:.2} disp {:?} slow {:.3}", e.name, e.life_h, e.served, e.day_utility, e.avg_power_mw, e.display_frac, e.slow_radio_frac);
        }
        println!("agreement {:?}", r.dp_agreement);
    }
}
