# RL Lab

Route: `#/rl` · Engine: `crates/forge-core/src/rl/` · Metric: `rl.policy_life_gain_pct`

## Tabs

| Tab | What you do | What you learn |
|---|---|---|
| Iris power policy | Set the energy price λ, battery awareness β, the low-refresh display power assumption, episodes and seed (saved in `design.extra.rl_power`); inspect the learned policy table, simulated days for the fixed and learned policies, the λ trade-off curve, the energy model and the wearer model. | Formulating a product decision as an MDP, reward design, off-policy learning, checking RL against dynamic programming, honest evaluation. |
| Gridworld | Paint walls, pits, goal and start; set slip, step reward, γ; train Q-learning or SARSA and watch the value heat-map and greedy arrows evolve, then the greedy agent; compare with value iteration sweep by sweep. | Bellman equations, TD learning vs planning, exploration, on- vs off-policy (cliff walking). |
| CartPole | Choose the state discretisation, episodes, γ and decay; train; watch the greedy replay and the learned policy over pole angle × rate. | The classic control benchmark, tabular limits, why function approximation exists. |

Downloads: power policy + Q-table + model (JSON), simulated day (CSV); gridworld Q-table
(JSON) and learning curve (CSV); CartPole Q-table (JSON) and learning curve (CSV).

## The maths
* **Value iteration** `V(s) ← max_a Σ P(s'|s,a)[r + γV(s')]`; **Q-learning**
  `Q ← Q + α[r + γ max Q(s',·) − Q]`; **SARSA** `Q ← Q + α[r + γ Q(s',a') − Q]`, ε-greedy with
  multiplicative decay and random tie-breaking. Moves succeed with 1 − slip and veer left/right
  with slip/2. Test: Russell & Norvig's 4×3 world (step −0.04, slip 0.2, γ = 1) reproduces the
  textbook utilities (0.705 at the start, 0.388 next to the pit, …).
* **CartPole** — Gymnasium `CartPole-v1` physics (Barto, Sutton & Anderson 1983): explicit
  Euler at 20 ms, ±10 N, failure at ±12° / ±2.4 m, 500-step cap. Default bins (1, 3, 6, 12),
  log-decaying α and ε; the best greedy table (mean of 3 evaluation episodes) is kept as a
  checkpoint because tabular CartPole policies often degrade late in training. Test: learning curve improves and a greedy episode reaches ≥ 150 steps.
* **Iris power MDP** (`power.rs`)
  * state = (battery bucket, context ∈ {idle, glance, navigate, caption}); action = display
    {off, low-refresh, full} × BLE interval {fast, slow}.
  * wearer: Markov chain with 1-min steps; non-idle share = the display's BOM duty cycle
    (30 % for Iris), mean dwell 1.5/8/10 min, idle jump probabilities from flow balance so the
    stationary distribution is exact (tested).
  * energy from `p.components`: display U2 (full = active, low = active × factor, off = sleep);
    SoC U1 with a per-(context, radio) activity weight scaled so the fixed policy reproduces the
    BOM duty cycle exactly; all other parts at their BOM average; ÷ buck efficiency; battery =
    capacity × nominal V × usable fraction. Test: fixed-policy power = BOM duty-cycle power.
  * reward `u(context, a) − λ(1 + β(1−soc)²)·P/P_ref`, penalty when the battery empties.
  * learning: Q-learning with exploring starts and visit-count step sizes; since the wearer's
    next context does not depend on the action, all six actions are updated with the same
    sampled next context (post-decision-state trick; switchable). A value-iteration solution
    of the bucketed model is the reference.
  * evaluation: 24 seeded days with identical wearer behaviour for every policy (common random
    numbers), run until the battery is empty.
* **Metric** `rl.policy_life_gain_pct` = (learned life / fixed life − 1) × 100, reported as 0
  if the learned policy serves less utility than the fixed one (0.5 % tolerance).
  **Iris v0.6 (240 mAh): 13.73 h → 17.13 h = +24.8 % (R-024, goal ≥ 5 % → pass), utility
  served 100.0 % vs 100.0 %, 96 % of states identical to the DP optimum.** The learned policy switches the display to
  low-refresh for glances and navigation (UC-1 redraws only on change) and slows BLE while
  idle; it keeps full refresh and fast BLE for captions.

## How honest is it?
The fixed policy is tied exactly to the BOM, and every milliwatt is traceable to
`p.components`. The gain, however, rests on assumptions that must be measured on hardware:
the low-refresh power fraction (default 70 % of full — the gain is most sensitive to this),
SoC activity per context, and the utility weights. The metric is deterministic (fixed seeds)
and ~9 ms natively.

## Limitations
* Tabular methods only; tiny state spaces; the wearer model is synthetic.
* The power model ignores display wake-up transients, BLE connection-update latency and
  battery voltage sag (see the Power & Thermal lab for the electrical side).
* CartPole with a coarse table rarely reaches the 500-step cap every time.

## Graduate to
* **Gymnasium** (environment API; turn the power MDP into an `Env`) and
  **Stable-Baselines3** (DQN, PPO, SAC).
* **MuJoCo** / **Drake** environments for continuous control.
* Real usage logs to fit the wearer model, and on-device measurements for the power table.

## References
* R. S. Sutton & A. G. Barto, *Reinforcement Learning: An Introduction*, 2nd ed., 2018 — http://incompleteideas.net/book/the-book-2nd.html
* C. J. C. H. Watkins & P. Dayan, "Q-learning", Machine Learning 8, 1992.
* A. G. Barto, R. S. Sutton, C. W. Anderson, "Neuronlike adaptive elements that can solve difficult learning control problems", IEEE Trans. SMC 13, 1983.
* S. Russell & P. Norvig, *Artificial Intelligence: A Modern Approach* (sequential decision problems chapter).
* L. Benini, A. Bogliolo, G. De Micheli, "A survey of design techniques for system-level dynamic power management", IEEE Trans. VLSI Systems 8(3), 2000.
* W. B. Powell, *Approximate Dynamic Programming*, 2nd ed., Wiley 2011 (post-decision states).
* Gymnasium — https://gymnasium.farama.org
