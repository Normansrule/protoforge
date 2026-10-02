// TypeScript mirror of crates/forge-core/src/program/*.rs outputs.

export interface Task {
  id: string; wbs: string; name: string; phase: string; owner: string;
  optimistic: number; likely: number; pessimistic: number;
  preds: string[]; milestone: boolean; fte: number; expense_usd: number; source: string;
}

export interface Econ {
  retail_price_usd: number; direct_share_pct: number; retail_margin_pct: number; payment_fee_pct: number;
  logistics_usd: number; warranty_return_pct: number; return_handling_usd: number;
  certification_usd: number | null; fixtures_usd: number; dev_cost_usd: number | null;
  volumes: number[]; opex_pct: number; discount_rate_pct: number; launch: string;
}

export interface Evidence { heritage: string; stage: string; trl_override: number | null; mrl_override: number | null; note: string }

export interface Config {
  start_date: string; tasks: Task[] | null; eng_rate_usd_h: number; rates: Record<string, number>;
  dist: string; iterations: number; seed: number; frame_tooling: string; economics: Econ;
  readiness: Record<string, Evidence>; reviews: Record<string, boolean>;
}

export interface ConfigOut { config: Config; tasks: Task[]; default_tasks: Task[]; edited: boolean; evidence_defaults: Record<string, Evidence>; stages: string[] }

export interface TaskRow {
  id: string; wbs: string; name: string; phase: string; owner: string; milestone: boolean;
  optimistic: number; likely: number; pessimistic: number; duration: number;
  es: number; ef: number; ls: number; lf: number; total_float: number; free_float: number; critical: boolean;
  level: number; preds: [string, number][]; succs: string[]; budget_usd: number; expense_usd: number; fte: number;
  start_date: string; finish_date: string; source: string;
}

export interface StageCheck { stage: string; from: string; to: string; weeks: number; typical: [number, number]; verdict: string }
export interface OwnerLoad { owner: string; weeks: number[]; peak: number }

export interface Schedule {
  tasks: TaskRow[]; finish_weeks: number; finish_date: string; start_date: string; critical_path: string[]; critical_count: number;
  bac_usd: number; labour_usd: number; expense_usd: number; levels: number; stage_checks: StageCheck[]; owners: OwnerLoad[];
  csv: string; mermaid: string; notes: string[];
}

export interface TaskRisk { id: string; name: string; mean: number; sd: number; criticality: number; correlation: number; cruciality: number }
export interface Tornado { id: string; name: string; low: number; high: number; swing: number }
export interface MilestoneRisk { id: string; name: string; deterministic: number; p50: number; p80: number }
export interface RiskResult {
  iterations: number; dist: string; seed: number; finishes: number[];
  histogram: { lo: number; width: number; counts: number[] }; s_curve: [number, number][];
  mean: number; sd: number; p10: number; p50: number; p80: number; p90: number; min: number; max: number;
  deterministic: number; prob_deterministic: number; pert_mean: number; pert_sd: number; pert_p80: number; pert_path: string[];
  tasks: TaskRisk[]; tornado: Tornado[]; milestones: MilestoneRisk[]; preview: { finish: number; critical: number[] }[]; ids: string[]; notes: string[];
}

export interface Scenario { status_week: number; onset_week: number; progress_rate: number; cost_factor: number }
export interface Indices { cv: number; sv: number; cpi: number; spi: number; eac_cpi: number; eac_atypical: number; eac_cpi_spi: number; etc: number; vac: number; tcpi: number }
export interface EvmResult {
  bac: number; pd: number; scenario: Scenario; weeks: number[]; pv: number[]; ev: number[]; ac: number[];
  ev_forecast: [number, number][]; ac_forecast: [number, number][]; cpi_t: [number, number][]; spi_t: [number, number][]; spi_es_t: [number, number][];
  at: number; pv_now: number; ev_now: number; ac_now: number; idx: Indices; es: number; spi_time: number; ieac_weeks: number;
  true_finish: number; true_cost: number; eacs: { name: string; formula: string; value: number; meaning: string }[];
  tasks: { id: string; name: string; budget: number; planned_pct: number; earned_pct: number; pv: number; ev: number }[];
}

export interface Quarter { q: number; label: string; units: number; revenue: number; cogs: number; opex: number; dev: number; net: number; cumulative: number }
export interface Outcome {
  net_price: number; landed: number; variable: number; gm_pct: number; gross_profit: number; contribution: number; fixed: number;
  breakeven_units: number | null; lifetime_units: number; npv: number; irr_annual: number | null; payback_weeks: number | null; peak_cash_need: number; quarters: Quarter[];
}
export interface Sens { name: string; low_label: string; high_label: string; npv_low: number; npv_high: number; gm_low: number; gm_high: number; be_low: number; be_high: number }
export interface EconResult {
  target_qty: number; lines: { name: string; usd: number; kind: string; note: string }[]; mfg_unit_usd: number;
  waterfall: [string, number][]; outcome: Outcome; breakeven_curve: [number, number][]; tornado: Sens[];
  inputs: { launch_week: number; plan_weeks: number; cert_usd: number; fixtures_usd: number; volumes: number[]; mfg_years: number[] };
  econ: Econ; frame_process: string; frame_molded: boolean; crossover_qty: number | null; tooling_usd: number; plan_cert_usd: number; notes: string[];
}

export interface Rung { level: number; name: string; met: boolean; why: string }
export interface SubsystemReadiness {
  id: string; name: string; domain: string; lab: string; heritage: string; heritage_suggested: boolean; stage: string;
  trl_suggested: number; trl: number; mrl_suggested: number; mrl: number; trl_ladder: Rung[]; mrl_ladder: Rung[];
  reqs: { id: string; method: string; kind: string; metric: string | null; status: string | null }[]; warnings: string[]; note: string;
}
export interface Crit { key: string; text: string; phase: 'entry' | 'exit'; auto: boolean; met: boolean; detail: string; reqs: [string, string][]; route: string }
export interface Review { id: string; name: string; milestone: string; week: number | null; purpose: string; criteria: Crit[]; met: number; total: number; ready: boolean }
export interface Summary {
  cpm_weeks: number; p50_weeks: number; p80_weeks: number; bac_usd: number; launch_week: number; gross_margin_pct: number; breakeven_units: number;
  lifetime_units: number; npv_usd: number; landed_usd: number; net_price_usd: number; min_trl: number; min_trl_subsystems: string[]; min_mrl: number;
  milestone_weeks: Record<string, number>; compliance_items: number; critical_path: string[];
}
export interface ReadinessOut { subsystems: SubsystemReadiness[]; reviews: Review[]; min_trl: number; min_mrl: number; summary: Summary; trl_names: string[]; mrl_names: string[] }

export interface PertOut { mean: number; variance: number; sd: number; tri_mean: number; tri_sd: number; x: number[]; pert_pdf: number[]; tri_pdf: number[]; alpha: number; beta: number }
