//! # Reinforcement-learning engine (`rl.*`)
//!
//! * [`grid`] — gridworld MDP: value iteration (ground truth), tabular
//!   Q-learning and SARSA with ε-greedy exploration, training snapshots.
//! * [`cartpole`] — Barto–Sutton–Anderson / Gymnasium CartPole with
//!   discretised-state Q-learning.
//! * [`power`] — the Iris power-management MDP (battery × user context ×
//!   radio state), Q-learning vs the fixed BOM duty-cycle policy and a
//!   dynamic-programming reference. Produces `rl.policy_life_gain_pct`.
//!
//! Everything is seeded and deterministic. References: Sutton & Barto,
//! *Reinforcement Learning: An Introduction* 2nd ed. (2018,
//! incompleteideas.net/book/the-book-2nd.html); Watkins & Dayan,
//! "Q-learning", Machine Learning 8 (1992); Barto, Sutton & Anderson, IEEE
//! SMC 1983; Gymnasium (gymnasium.farama.org).

#![allow(clippy::needless_range_loop)] // index-heavy numeric code reads better with explicit indices

pub mod cartpole;
pub mod grid;
pub mod power;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Metric, Project};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct GridIn {
    grid: grid::GridSpec,
    td: grid::TdConfig,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct FrontierIn {
    project: Option<Project>,
    config: power::PowerConfig,
    lambdas: Vec<f64>,
}

#[derive(Serialize)]
struct FrontierPoint {
    lambda: f64,
    life_h: f64,
    served: f64,
    day_utility: f64,
    fixed_life_h: f64,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct PowerIn {
    project: Option<Project>,
    config: power::PowerConfig,
}

/// JSON API entry point for `rl.*` methods.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "rl.gridworld" => handle(input, |i: GridIn| {
            let g = grid::Grid::parse(&i.grid)?;
            Ok(grid::train_td(&g, &i.td))
        }),
        "rl.value_iteration" => handle(input, |i: GridIn| {
            let g = grid::Grid::parse(&i.grid)?;
            Ok(grid::value_iteration(&g, 1e-10, 10_000))
        }),
        "rl.cartpole" => handle(input, |i: cartpole::CartPoleRl| Ok(cartpole::train(&i))),
        "rl.power_policy" => handle(input, |i: PowerIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            Ok(power::train(&p, &i.config))
        }),
        "rl.power_frontier" => handle(input, |i: FrontierIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            let lambdas = if i.lambdas.is_empty() { vec![0.0, 0.02, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4] } else { i.lambdas };
            Ok(lambdas
                .iter()
                .map(|&l| {
                    let r = power::train(&p, &power::PowerConfig { lambda: l, with_dp: false, ..i.config.clone() });
                    FrontierPoint { lambda: l, life_h: r.learned.life_h, served: r.learned.served, day_utility: r.learned.day_utility, fixed_life_h: r.fixed.life_h }
                })
                .collect::<Vec<_>>())
        }),
        _ => None,
    }
}

/// Metrics this module contributes to requirement verification.
pub fn metrics(p: &Project) -> Vec<Metric> {
    if p.design.battery.capacity_mah <= 0.0 {
        return vec![];
    }
    // the lab stores its MDP settings in design.extra.rl_power (defaults otherwise)
    let cfg: power::PowerConfig = p.design.extra.get("rl_power").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
    let r = power::train(p, &power::PowerConfig { with_dp: false, ..cfg.clone() });
    let note = format!(
        "Q-learning ({} episodes) vs fixed BOM duty-cycle policy over simulated days: life {:.2} h → {:.2} h; utility served {:.1} % vs {:.1} %{}",
        cfg.episodes,
        r.fixed.life_h,
        r.learned.life_h,
        r.learned.served * 100.0,
        r.fixed.served * 100.0,
        if r.utility_ok { "" } else { " — learned policy lost utility, gain reported as 0" }
    );
    vec![Metric::new("rl.policy_life_gain_pct", "Battery-life gain of the learned power policy", r.metric_value, "%", "rl").with_note(&note)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_metric_meets_goal() {
        let t0 = std::time::Instant::now();
        let m = metrics(&Project::iris());
        println!("{} = {:.2} ({:?}) — {}", m[0].key, m[0].value, t0.elapsed(), m[0].note);
        assert!(m[0].value >= 5.0);
    }
}
