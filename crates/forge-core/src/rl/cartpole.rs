//! CartPole with tabular Q-learning on a discretised state.
//!
//! Physics and termination follow Gymnasium's `CartPole-v1`, which in turn
//! implements Barto, Sutton & Anderson, "Neuronlike adaptive elements that
//! can solve difficult learning control problems", IEEE Trans. SMC 13
//! (1983): g = 9.8, cart 1.0 kg, pole 0.1 kg, half-length 0.5 m, force
//! ±10 N, explicit Euler with τ = 0.02 s; the episode ends when
//! |x| > 2.4 m or |θ| > 12°, or after 500 steps; reward +1 per step.
//!
//! ```text
//! temp  = (F + m l θ̇² sinθ) / (M + m)
//! θ̈     = (g sinθ − cosθ·temp) / (l (4/3 − m cos²θ / (M + m)))
//! ẍ     = temp − m l θ̈ cosθ / (M + m)
//! ```
//! The continuous state is binned (Barto et al. used 162 "boxes"); the
//! learning rate and exploration decay logarithmically with the episode
//! index, a common schedule for this problem.

use serde::{Deserialize, Serialize};

use super::grid::argmax;
use crate::math::Rng;

const GRAVITY: f64 = 9.8;
const MASSCART: f64 = 1.0;
const MASSPOLE: f64 = 0.1;
const TOTAL: f64 = MASSCART + MASSPOLE;
const LENGTH: f64 = 0.5;
const PML: f64 = MASSPOLE * LENGTH;
const FORCE: f64 = 10.0;
const TAU: f64 = 0.02;
pub const X_LIMIT: f64 = 2.4;
pub const THETA_LIMIT: f64 = 12.0 * 2.0 * std::f64::consts::PI / 360.0;

pub fn step(s: [f64; 4], action: usize) -> [f64; 4] {
    let f = if action == 1 { FORCE } else { -FORCE };
    let (x, xd, th, thd) = (s[0], s[1], s[2], s[3]);
    let (st, ct) = th.sin_cos();
    let temp = (f + PML * thd * thd * st) / TOTAL;
    let thacc = (GRAVITY * st - ct * temp) / (LENGTH * (4.0 / 3.0 - MASSPOLE * ct * ct / TOTAL));
    let xacc = temp - PML * thacc * ct / TOTAL;
    [x + TAU * xd, xd + TAU * xacc, th + TAU * thd, thd + TAU * thacc]
}

pub fn failed(s: &[f64; 4]) -> bool {
    s[0].abs() > X_LIMIT || s[2].abs() > THETA_LIMIT
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CartPoleRl {
    pub episodes: usize,
    pub max_steps: usize,
    /// Bins for x, ẋ, θ, θ̇.
    pub bins: [usize; 4],
    /// Clipping bounds for ẋ and θ̇ (x and θ use the failure limits).
    pub xdot_bound: f64,
    pub thetadot_bound: f64,
    pub gamma: f64,
    pub alpha_min: f64,
    pub eps_min: f64,
    /// Episode scale of the log decay: rate = clamp(1 − log10((e+1)/k)).
    pub decay_k: f64,
    pub seed: u64,
}

impl Default for CartPoleRl {
    fn default() -> Self {
        CartPoleRl {
            episodes: 600,
            max_steps: 500,
            bins: [1, 3, 6, 12],
            xdot_bound: 0.5,
            thetadot_bound: 50f64.to_radians(),
            gamma: 0.99,
            alpha_min: 0.1,
            eps_min: 0.02,
            decay_k: 25.0,
            seed: 3,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CartPoleRlResult {
    pub returns: Vec<f64>,
    pub moving_avg: Vec<f64>,
    /// Greedy evaluation (mean of 3 episodes) during training: (episode, steps).
    pub greedy_curve: Vec<(usize, f64)>,
    /// Best mean greedy evaluation (3 episodes) and when it happened; the
    /// returned Q-table and replay use that checkpoint.
    pub best_greedy: f64,
    pub best_episode: usize,
    /// Greedy evaluation of the final table.
    pub final_greedy: f64,
    pub q: Vec<[f64; 2]>,
    pub bins: [usize; 4],
    /// Greedy replay after training: [x, ẋ, θ, θ̇, action] per step.
    pub replay: Vec<[f64; 5]>,
    pub replay_len: usize,
    pub states: usize,
}

struct Disc {
    bins: [usize; 4],
    lo: [f64; 4],
    hi: [f64; 4],
}

impl Disc {
    fn index(&self, s: &[f64; 4]) -> usize {
        let mut idx = 0;
        for i in 0..4 {
            let n = self.bins[i].max(1);
            let b = if n == 1 {
                0
            } else {
                let f = ((s[i] - self.lo[i]) / (self.hi[i] - self.lo[i])).clamp(0.0, 0.999_999);
                (f * n as f64) as usize
            };
            idx = idx * n + b;
        }
        idx
    }
    fn size(&self) -> usize {
        self.bins.iter().map(|b| (*b).max(1)).product()
    }
}

fn episode_greedy(q: &[[f64; 2]], d: &Disc, rng: &mut Rng, max_steps: usize, record: bool) -> (usize, Vec<[f64; 5]>) {
    let mut s = [0.0; 4];
    for v in s.iter_mut() {
        *v = rng.uniform() * 0.1 - 0.05;
    }
    let mut frames = Vec::new();
    for k in 0..max_steps {
        let a = argmax(&q[d.index(&s)]);
        if record {
            frames.push([s[0], s[1], s[2], s[3], a as f64]);
        }
        s = step(s, a);
        if failed(&s) {
            if record {
                frames.push([s[0], s[1], s[2], s[3], a as f64]);
            }
            return (k + 1, frames);
        }
    }
    (max_steps, frames)
}

pub fn train(cfg: &CartPoleRl) -> CartPoleRlResult {
    let d = Disc {
        bins: cfg.bins.map(|b| b.clamp(1, 30)),
        lo: [-X_LIMIT, -cfg.xdot_bound, -THETA_LIMIT, -cfg.thetadot_bound],
        hi: [X_LIMIT, cfg.xdot_bound, THETA_LIMIT, cfg.thetadot_bound],
    };
    let mut q = vec![[0.0f64; 2]; d.size()];
    let mut rng = Rng::new(cfg.seed);
    let mut eval_rng = Rng::new(cfg.seed ^ 0x5151);
    let episodes = cfg.episodes.clamp(1, 20_000);
    let max_steps = cfg.max_steps.clamp(10, 2000);
    let mut out = CartPoleRlResult {
        bins: d.bins,
        states: d.size(),
        ..Default::default()
    };
    let eval_every = (episodes / 60).max(1);
    let k = cfg.decay_k.max(1.0);
    let mut best_q = q.clone();
    for e in 0..episodes {
        let rate = (1.0 - ((e as f64 + 1.0) / k).log10()).clamp(0.0, 1.0);
        let alpha = rate.max(cfg.alpha_min);
        let eps = rate.max(cfg.eps_min);
        let mut s = [0.0; 4];
        for v in s.iter_mut() {
            *v = rng.uniform() * 0.1 - 0.05;
        }
        let mut steps = 0;
        for _ in 0..max_steps {
            let i = d.index(&s);
            let a = if rng.uniform() < eps { rng.below(2) } else { argmax(&q[i]) };
            let s2 = step(s, a);
            steps += 1;
            let done = failed(&s2);
            let r = 1.0;
            let target = if done {
                r
            } else {
                r + cfg.gamma * q[d.index(&s2)].iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            };
            q[i][a] += alpha * (target - q[i][a]);
            s = s2;
            if done {
                break;
            }
        }
        out.returns.push(steps as f64);
        let lo = out.returns.len().saturating_sub(50);
        let w = &out.returns[lo..];
        out.moving_avg.push(w.iter().sum::<f64>() / w.len() as f64);
        if e % eval_every == 0 || e + 1 == episodes {
            // greedy evaluation: mean of 3 episodes; keep the best table
            // (checkpointing — tabular CartPole policies often degrade later)
            let g = (0..3)
                .map(|_| episode_greedy(&q, &d, &mut eval_rng, max_steps, false).0 as f64)
                .sum::<f64>()
                / 3.0;
            out.greedy_curve.push((e + 1, g));
            if g >= out.best_greedy {
                out.best_greedy = g;
                out.best_episode = e + 1;
                best_q = q.clone();
            }
        }
    }
    out.final_greedy = out.greedy_curve.last().map(|p| p.1).unwrap_or(0.0);
    let mut rr = Rng::new(cfg.seed ^ 0x77);
    let (n, frames) = episode_greedy(&best_q, &d, &mut rr, max_steps, true);
    out.replay = frames;
    out.replay_len = n;
    out.q = best_q;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physics_matches_gym_single_step() {
        // push right from rest: ẋ grows, θ̇ becomes negative
        let s = step([0.0, 0.0, 0.0, 0.0], 1);
        let temp = FORCE / TOTAL;
        let thacc = -temp / (LENGTH * (4.0 / 3.0 - MASSPOLE / TOTAL));
        let xacc = temp - PML * thacc / TOTAL;
        assert!((s[1] - TAU * xacc).abs() < 1e-12 && (s[3] - TAU * thacc).abs() < 1e-12);
    }

    #[test]
    fn learning_curve_improves() {
        let r = train(&CartPoleRl::default());
        let n = r.returns.len();
        let early: f64 = r.returns[..50].iter().sum::<f64>() / 50.0;
        let late: f64 = r.returns[n - 100..].iter().sum::<f64>() / 100.0;
        assert!(late > 2.0 * early, "early {early} late {late}");
        assert!(r.best_greedy >= 150.0, "best greedy {}", r.best_greedy);
    }
}

#[cfg(test)]
mod debug {
    use super::*;
    #[test]
    #[ignore]
    fn sweep_bins() {
        for bins in [[1, 1, 6, 12], [1, 3, 6, 12], [3, 3, 6, 12], [1, 1, 8, 12], [3, 3, 8, 12]] {
            for seed in [1u64, 2, 3] {
                let r = train(&CartPoleRl {
                    bins,
                    seed,
                    ..Default::default()
                });
                println!(
                    "{bins:?} seed {seed}: best {:.0} @ {} final {:.0} replay {}",
                    r.best_greedy, r.best_episode, r.final_greedy, r.replay_len
                );
            }
        }
    }
}
