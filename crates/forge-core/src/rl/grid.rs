//! Gridworld MDP with slippery moves, solved exactly by **value iteration**
//! and learned by tabular **Q-learning** (off-policy TD control) and
//! **SARSA** (on-policy TD control).
//!
//! ```text
//! Value iteration:  V(s) ← max_a Σ_s' P(s'|s,a) [ r(s,a,s') + γ V(s') ]
//! Q-learning:       Q(s,a) ← Q(s,a) + α [ r + γ max_a' Q(s',a') − Q(s,a) ]
//! SARSA:            Q(s,a) ← Q(s,a) + α [ r + γ Q(s',a')        − Q(s,a) ]
//! ```
//! Moves succeed with probability `1 − slip` and veer left/right with
//! `slip/2` each; bumping into a wall or the border leaves the agent in
//! place. Entering a goal or pit ends the episode.
//!
//! With the layout of Russell & Norvig's 4×3 world, step reward −0.04,
//! slip 0.2 and γ = 1, value iteration reproduces the textbook utilities
//! (e.g. 0.705 at the start). References: Sutton & Barto, *Reinforcement
//! Learning: An Introduction*, 2nd ed. (2018), Ch. 4 & 6; Watkins & Dayan,
//! "Q-learning", Machine Learning 8 (1992); S. Russell & P. Norvig,
//! *Artificial Intelligence: A Modern Approach*, Ch. 17.

use serde::{Deserialize, Serialize};

use crate::math::Rng;

pub const ACTIONS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)]; // up, right, down, left (row 0 = top)

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cell {
    Empty,
    Wall,
    Pit,
    Goal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GridSpec {
    /// Rows top→bottom; `.` empty, `#` wall, `P` pit, `G` goal, `S` start.
    pub layout: Vec<String>,
    pub slip: f64,
    pub step_reward: f64,
    pub goal_reward: f64,
    pub pit_reward: f64,
    pub gamma: f64,
}

impl Default for GridSpec {
    fn default() -> Self {
        GridSpec {
            layout: vec![
                "......#...G".into(),
                ".##...#.#..".into(),
                "..#.....#P.".into(),
                "..#.PP..#..".into(),
                "S.......#..".into(),
                "....##.....".into(),
            ],
            slip: 0.2,
            step_reward: -0.04,
            goal_reward: 1.0,
            pit_reward: -1.0,
            gamma: 0.95,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Grid {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<Cell>,
    pub start: usize,
    pub spec: GridSpec,
}

impl Grid {
    pub fn parse(spec: &GridSpec) -> Result<Grid, String> {
        let h = spec.layout.len();
        if h == 0 {
            return Err("empty layout".into());
        }
        let w = spec.layout.iter().map(|r| r.chars().count()).max().unwrap_or(0);
        if w == 0 || w * h > 400 {
            return Err("grid must have between 1 and 400 cells".into());
        }
        let mut cells = vec![Cell::Empty; w * h];
        let mut start = None;
        for (r, row) in spec.layout.iter().enumerate() {
            for (c, ch) in row.chars().enumerate() {
                cells[r * w + c] = match ch {
                    '#' => Cell::Wall,
                    'P' | 'p' => Cell::Pit,
                    'G' | 'g' => Cell::Goal,
                    'S' | 's' => {
                        start = Some(r * w + c);
                        Cell::Empty
                    }
                    _ => Cell::Empty,
                };
            }
        }
        if !cells.contains(&Cell::Goal) {
            return Err("the grid needs at least one goal".into());
        }
        let start = start
            .or_else(|| cells.iter().position(|c| *c == Cell::Empty))
            .ok_or("no free cell for the start")?;
        Ok(Grid {
            w,
            h,
            cells,
            start,
            spec: spec.clone(),
        })
    }

    pub fn n(&self) -> usize {
        self.w * self.h
    }

    pub fn terminal(&self, s: usize) -> bool {
        matches!(self.cells[s], Cell::Goal | Cell::Pit)
    }

    fn step_to(&self, s: usize, a: usize) -> usize {
        let (x, y) = ((s % self.w) as i32, (s / self.w) as i32);
        let (dx, dy) = ACTIONS[a];
        let (nx, ny) = (x + dx, y + dy);
        if nx < 0 || ny < 0 || nx >= self.w as i32 || ny >= self.h as i32 {
            return s;
        }
        let ns = ny as usize * self.w + nx as usize;
        if self.cells[ns] == Cell::Wall {
            s
        } else {
            ns
        }
    }

    pub fn reward(&self, s2: usize) -> f64 {
        self.spec.step_reward
            + match self.cells[s2] {
                Cell::Goal => self.spec.goal_reward,
                Cell::Pit => self.spec.pit_reward,
                _ => 0.0,
            }
    }

    /// Outcome distribution of action a in s: (prob, next state).
    pub fn outcomes(&self, s: usize, a: usize) -> [(f64, usize); 3] {
        let slip = self.spec.slip.clamp(0.0, 1.0);
        [
            (1.0 - slip, self.step_to(s, a)),
            (slip / 2.0, self.step_to(s, (a + 1) % 4)),
            (slip / 2.0, self.step_to(s, (a + 3) % 4)),
        ]
    }

    pub fn sample(&self, s: usize, a: usize, rng: &mut Rng) -> usize {
        let u = rng.uniform();
        let o = self.outcomes(s, a);
        if u < o[0].0 {
            o[0].1
        } else if u < o[0].0 + o[1].0 {
            o[1].1
        } else {
            o[2].1
        }
    }

    fn active(&self, s: usize) -> bool {
        self.cells[s] == Cell::Empty
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub episode: usize,
    pub v: Vec<f64>,
    /// Greedy action per cell (−1 for walls/terminals).
    pub policy: Vec<i8>,
    pub epsilon: f64,
    /// The path of the most recent episode.
    pub path: Vec<usize>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ViResult {
    pub v: Vec<f64>,
    pub q: Vec<[f64; 4]>,
    pub policy: Vec<i8>,
    pub iterations: usize,
    /// Value after each of the first sweeps (for animation).
    pub sweeps: Vec<Vec<f64>>,
    pub residuals: Vec<f64>,
}

pub fn value_iteration(g: &Grid, tol: f64, max_iter: usize) -> ViResult {
    let n = g.n();
    let gamma = g.spec.gamma.clamp(0.0, 1.0);
    let mut v = vec![0.0; n];
    let mut out = ViResult::default();
    for it in 0..max_iter {
        let mut nv = v.clone();
        let mut delta: f64 = 0.0;
        for s in 0..n {
            if !g.active(s) {
                continue;
            }
            let best = (0..4)
                .map(|a| {
                    g.outcomes(s, a)
                        .iter()
                        .map(|(p, s2)| p * (g.reward(*s2) + if g.terminal(*s2) { 0.0 } else { gamma * v[*s2] }))
                        .sum::<f64>()
                })
                .fold(f64::NEG_INFINITY, f64::max);
            delta = delta.max((best - v[s]).abs());
            nv[s] = best;
        }
        v = nv;
        out.residuals.push(delta);
        if it < 60 {
            out.sweeps.push(v.clone());
        }
        out.iterations = it + 1;
        if delta < tol {
            break;
        }
    }
    out.q = (0..n)
        .map(|s| {
            let mut q = [0.0; 4];
            if g.active(s) {
                for (a, qa) in q.iter_mut().enumerate() {
                    *qa = g
                        .outcomes(s, a)
                        .iter()
                        .map(|(p, s2)| p * (g.reward(*s2) + if g.terminal(*s2) { 0.0 } else { gamma * v[*s2] }))
                        .sum();
                }
            }
            q
        })
        .collect();
    out.policy = (0..n).map(|s| if g.active(s) { argmax(&out.q[s]) as i8 } else { -1 }).collect();
    out.v = v;
    out
}

pub fn argmax(q: &[f64]) -> usize {
    let mut b = 0;
    for i in 1..q.len() {
        if q[i] > q[b] + 1e-12 {
            b = i;
        }
    }
    b
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TdConfig {
    /// "q_learning" or "sarsa"
    pub algorithm: String,
    pub episodes: usize,
    pub max_steps: usize,
    pub alpha: f64,
    pub eps_start: f64,
    pub eps_end: f64,
    /// Multiplicative ε decay per episode.
    pub eps_decay: f64,
    pub seed: u64,
    pub frames: usize,
    /// Optimistic initial Q value (encourages exploration).
    pub q_init: f64,
}

impl Default for TdConfig {
    fn default() -> Self {
        TdConfig {
            algorithm: "q_learning".into(),
            episodes: 600,
            max_steps: 200,
            alpha: 0.2,
            eps_start: 1.0,
            eps_end: 0.05,
            eps_decay: 0.993,
            seed: 7,
            frames: 40,
            q_init: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TdResult {
    pub algorithm: String,
    pub q: Vec<[f64; 4]>,
    pub v: Vec<f64>,
    pub policy: Vec<i8>,
    pub returns: Vec<f64>,
    pub steps: Vec<usize>,
    pub successes: Vec<bool>,
    pub snapshots: Vec<Snapshot>,
    /// Greedy rollout from the start (slip sampled with a fixed seed).
    pub rollout: Vec<usize>,
    pub rollout_reached_goal: bool,
    pub vi: ViResult,
    /// RMS |V_learned − V*| over free cells.
    pub v_rms_error: f64,
    /// Fraction of free cells where the greedy action matches value iteration.
    pub policy_agreement: f64,
    pub w: usize,
    pub h: usize,
    pub cells: Vec<Cell>,
    pub start: usize,
}

fn eps_greedy(q: &[f64; 4], eps: f64, rng: &mut Rng) -> usize {
    if rng.uniform() < eps {
        rng.below(4)
    } else {
        // random tie-breaking keeps early exploration unbiased
        let m = q.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let ties: Vec<usize> = (0..4).filter(|&a| q[a] >= m - 1e-12).collect();
        ties[rng.below(ties.len())]
    }
}

pub fn train_td(g: &Grid, cfg: &TdConfig) -> TdResult {
    let n = g.n();
    let gamma = g.spec.gamma.clamp(0.0, 1.0);
    let mut rng = Rng::new(cfg.seed);
    let mut q = vec![[cfg.q_init; 4]; n];
    for (s, qs) in q.iter_mut().enumerate() {
        if !g.active(s) {
            *qs = [0.0; 4];
        }
    }
    let episodes = cfg.episodes.clamp(1, 50_000);
    let max_steps = cfg.max_steps.clamp(1, 10_000);
    let sarsa = cfg.algorithm == "sarsa";
    let every = (episodes / cfg.frames.clamp(1, 200)).max(1);
    let mut out = TdResult {
        algorithm: if sarsa { "sarsa".into() } else { "q_learning".into() },
        w: g.w,
        h: g.h,
        cells: g.cells.clone(),
        start: g.start,
        ..Default::default()
    };
    let mut eps = cfg.eps_start.clamp(0.0, 1.0);
    for ep in 0..episodes {
        let mut s = g.start;
        let mut a = eps_greedy(&q[s], eps, &mut rng);
        let mut ret = 0.0;
        let mut disc = 1.0;
        let mut path = vec![s];
        let mut success = false;
        let mut steps = 0;
        for _ in 0..max_steps {
            let s2 = g.sample(s, a, &mut rng);
            let r = g.reward(s2);
            ret += disc * r;
            disc *= gamma;
            steps += 1;
            path.push(s2);
            let term = g.terminal(s2);
            let a2 = eps_greedy(&q[s2], eps, &mut rng);
            let target = if term {
                r
            } else if sarsa {
                r + gamma * q[s2][a2]
            } else {
                r + gamma * q[s2].iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            };
            q[s][a] += cfg.alpha * (target - q[s][a]);
            if term {
                success = g.cells[s2] == Cell::Goal;
                break;
            }
            s = s2;
            a = a2;
        }
        out.returns.push(ret);
        out.steps.push(steps);
        out.successes.push(success);
        if ep % every == 0 || ep + 1 == episodes {
            out.snapshots.push(Snapshot {
                episode: ep + 1,
                v: (0..n)
                    .map(|s| {
                        if g.active(s) {
                            q[s].iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                        } else {
                            0.0
                        }
                    })
                    .collect(),
                policy: (0..n).map(|s| if g.active(s) { argmax(&q[s]) as i8 } else { -1 }).collect(),
                epsilon: eps,
                path: path.clone(),
            });
        }
        eps = (eps * cfg.eps_decay).max(cfg.eps_end);
    }
    out.v = (0..n)
        .map(|s| {
            if g.active(s) {
                q[s].iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            } else {
                0.0
            }
        })
        .collect();
    out.policy = (0..n).map(|s| if g.active(s) { argmax(&q[s]) as i8 } else { -1 }).collect();
    // greedy rollout
    let mut rr = Rng::new(cfg.seed ^ 0xABCDEF);
    let mut s = g.start;
    out.rollout.push(s);
    for _ in 0..(4 * n).max(50) {
        let s2 = g.sample(s, argmax(&q[s]), &mut rr);
        out.rollout.push(s2);
        s = s2;
        if g.terminal(s) {
            out.rollout_reached_goal = g.cells[s] == Cell::Goal;
            break;
        }
    }
    out.vi = value_iteration(g, 1e-10, 10_000);
    let free: Vec<usize> = (0..n).filter(|&s| g.active(s)).collect();
    let mut se = 0.0;
    let mut agree = 0usize;
    for &s in &free {
        se += (out.v[s] - out.vi.v[s]).powi(2);
        // count as agreeing if the learned action is (near-)optimal under V*
        let best = out.vi.q[s].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        if out.vi.q[s][out.policy[s] as usize] >= best - 1e-6 {
            agree += 1;
        }
    }
    out.v_rms_error = (se / free.len().max(1) as f64).sqrt();
    out.policy_agreement = agree as f64 / free.len().max(1) as f64;
    out.q = q;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aima() -> Grid {
        Grid::parse(&GridSpec {
            layout: vec!["...G".into(), ".#.P".into(), "S...".into()],
            slip: 0.2,
            step_reward: -0.04,
            goal_reward: 1.0,
            pit_reward: -1.0,
            gamma: 1.0,
        })
        .unwrap()
    }

    #[test]
    fn value_iteration_matches_russell_norvig() {
        let g = aima();
        let vi = value_iteration(&g, 1e-12, 10_000);
        // Russell & Norvig Fig. 17.3 utilities (row 0 = top)
        let expect = [
            (0, 0.812),
            (1, 0.868),
            (2, 0.918),
            (4, 0.762),
            (6, 0.660),
            (8, 0.705),
            (9, 0.655),
            (10, 0.611),
            (11, 0.388),
        ];
        for (s, v) in expect {
            assert!((vi.v[s] - v).abs() < 1e-3, "cell {s}: {} vs {v}", vi.v[s]);
        }
        // optimal action at (row 2, col 3) is "left" (away from the pit)
        assert_eq!(vi.policy[11], 3);
    }

    #[test]
    fn q_learning_reaches_goal() {
        let g = Grid::parse(&GridSpec::default()).unwrap();
        let r = train_td(&g, &TdConfig::default());
        assert!(r.rollout_reached_goal);
        let n = r.successes.len();
        let late = r.successes[n - 100..].iter().filter(|b| **b).count();
        assert!(late > 80, "late successes {late}");
        let early: f64 = r.returns[..50].iter().sum::<f64>() / 50.0;
        let lat: f64 = r.returns[n - 50..].iter().sum::<f64>() / 50.0;
        assert!(lat > early);
        assert!(r.policy_agreement > 0.6, "agreement {}", r.policy_agreement);
    }

    #[test]
    fn sarsa_learns_too() {
        let g = Grid::parse(&GridSpec::default()).unwrap();
        let r = train_td(
            &g,
            &TdConfig {
                algorithm: "sarsa".into(),
                ..Default::default()
            },
        );
        assert!(r.rollout_reached_goal);
    }
}
