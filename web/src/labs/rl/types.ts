// TypeScript mirrors of crates/forge-core/src/rl/*.rs outputs.
export type Cell = 'Empty' | 'Wall' | 'Pit' | 'Goal';
export interface GridSpec { layout: string[]; slip: number; step_reward: number; goal_reward: number; pit_reward: number; gamma: number }
export interface TdConfig { algorithm: 'q_learning' | 'sarsa'; episodes: number; max_steps: number; alpha: number; eps_start: number; eps_end: number; eps_decay: number; seed: number; frames: number; q_init: number }
export interface Snapshot { episode: number; v: number[]; policy: number[]; epsilon: number; path: number[] }
export interface ViResult { v: number[]; q: number[][]; policy: number[]; iterations: number; sweeps: number[][]; residuals: number[] }
export interface TdResult {
  algorithm: string; q: number[][]; v: number[]; policy: number[]; returns: number[]; steps: number[]; successes: boolean[];
  snapshots: Snapshot[]; rollout: number[]; rollout_reached_goal: boolean; vi: ViResult; v_rms_error: number; policy_agreement: number;
  w: number; h: number; cells: Cell[]; start: number;
}

export interface CartPoleRlResult {
  returns: number[]; moving_avg: number[]; greedy_curve: [number, number][]; best_greedy: number; best_episode: number; final_greedy: number; q: number[][];
  bins: [number, number, number, number]; replay: [number, number, number, number, number][]; replay_len: number; states: number;
}

export interface PowerConfig {
  soc_buckets: number; step_min: number; active_split: [number, number, number]; dwell_min: [number, number, number];
  low_refresh_factor: number; soc_weight_fast: number[]; soc_weight_slow: number[]; utility_low: number[]; slow_penalty: number[];
  lambda: number; beta: number; empty_penalty: number; episodes: number; horizon: number; gamma: number; alpha_min: number;
  eps_start: number; eps_end: number; seed: number; eval_runs: number; with_dp: boolean; all_actions: boolean;
}
export interface PowerModel {
  display_ref: string; soc_ref: string; display_mw: [number, number, number]; soc_mw: [number, number][]; soc_duty: [number, number][];
  const_mw: number; buck_efficiency: number; battery_mwh: number; shares: number[]; trans: number[][]; p_ref_mw: number; notes: string[];
}
export interface Timeline { t_h: number[]; ctx: number[]; display: number[]; radio: number[]; soc: number[]; power_mw: number[] }
export interface EvalSummary { name: string; life_h: number; served: number; day_utility: number; avg_power_mw: number; display_frac: [number, number, number]; slow_radio_frac: number; timeline: Timeline }
export interface PowerResult {
  contexts: string[]; displays: string[]; radios: string[]; model: PowerModel; soc_buckets: number; q: number[][]; policy: number[];
  visits: number[]; dp_policy: number[]; dp_agreement: number | null; training_curve: number[]; fixed: EvalSummary; learned: EvalSummary;
  dp: EvalSummary | null; gain_pct: number; utility_ok: boolean; metric_value: number;
}
