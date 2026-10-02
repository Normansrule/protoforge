// TypeScript mirror of crates/forge-core/src/reliability/*.rs outputs.

export interface Obs { t: number; failed: boolean }

export interface RankRow { t: number; failed: boolean; reverse_rank: number; adjusted_rank: number | null; median_rank: number | null }

export interface Fit {
  method: string; label: string; ok: boolean; error: string; beta: number; eta: number;
  r2: number | null; loglik: number; mttf: number; b1: number; b10: number; b50: number;
}

export interface BLife { p: number; t: number; lo: number | null; hi: number | null }

export interface FitOut {
  n: number; failures: number; suspensions: number; unit: string; rows: RankRow[]; fits: Fit[];
  method: string; beta: number; eta: number; beta_bounds: [number, number] | null; eta_bounds: [number, number] | null;
  cl: number; bounds_method: string; b_lives: BLife[]; line_f: number[]; line_t: number[]; bound_lo: number[]; bound_hi: number[];
  curves: { t: number[]; pdf: number[]; cdf: number[]; rel: number[]; hazard: number[] };
  mttf: number; interpretation: string; notes: string[];
}

export interface SampleSet { id: string; title: string; unit: string; source: string; description: string; text: string }

export interface MethodStats { method: string; betas: number[]; etas: number[]; beta_median: number; beta_mean: number; beta_p05: number; beta_p95: number; eta_median: number; b10_median: number }
export interface StudyOut { true_beta: number; true_eta: number; true_b10: number; censor_at: number | null; example: Obs[]; methods: MethodStats[]; skipped: number }

export interface BathtubOut { t: number[]; h_infant: number[]; h_random: number[]; h_wear: number[]; h_total: number[]; rel: number[]; units: { t: number; mode: number }[] }

export interface AltFactor { name: string; value: number; formula: string }
export interface AltOut { model: string; af: number; factors: AltFactor[]; sweep_x: number[]; sweep_af: number[]; sweep_label: string; equivalence: string; notes: string[] }
export interface PlanRow { units: number; test_duration: number; days: number; lives: number }
export interface PlanOut {
  required: number; field_per_test_unit: number; duration_for_units: number; days_for_units: number; lives_for_units: number;
  units_for_duration: number; lives_for_duration: number; success_run_units: number; table: PlanRow[]; notes: string[];
}
export interface ProfileOut { t_min: number[]; temp_c: number[]; rh: (number | null)[]; segments: { label: string; start_min: number; end_min: number }[]; cycle_min: number; cycles_per_day: number; total_h: number }

export interface Curve { label: string; x: number[]; y: number[] }
export interface SampleOut {
  success_run_n: number; success_run_exact: number; rows: { failures: number; binomial_n: number; lipson_n: number }[];
  lipson_curves: Curve[]; success_curves: Curve[]; oc_r: number[]; oc_binomial: number[]; oc_lipson: number[]; lipson_plan_n: number;
  consumer_risk: number; producer_risk_binomial: number; producer_risk_lipson: number; demonstrated_r: number; notes: string[];
}

export interface RbdNode {
  id: string; label: string; kind: string; k: number; children: RbdNode[];
  model: string; fit: number; fit_ref: number; temp_factor: number; beta: number; eta: number; life_unit: string; per_hour: number;
  qty: number; source: string; class: string; note: string; refdes: string;
}
export interface BlockOut {
  id: string; label: string; refdes: string; group: string; class: string; model: string; fit: number; fit_ref: number; temp_factor: number;
  beta: number; eta: number; life_unit: string; per_hour: number; qty: number; source: string; note: string;
  r_mission: number; birnbaum: number; criticality: number; mc_first_share: number;
}
export interface RbdOut {
  mission_h: number; r_mission: number; r_mission_random: number; mttf_h: number; mtbf_random_h: number; fit_total: number; b10_h: number;
  t_h: number[]; r_sys: number[]; r_random: number[]; blocks: BlockOut[];
  groups: { id: string; label: string; kind: string; k: number; n: number; r_mission: number }[];
  mc: { samples: number; r_mission: number; ci_lo: number; ci_hi: number; mttf_h: number; r_curve: number[]; units: { leaf_ttf_h: number[]; system_ttf_h: number }[] } | null;
  dominant: string; notes: string[];
}
export interface RelConfig {
  mission_years: number; hours_per_day: number; hinge_cycles_per_day: number; use_ambient_c: number | null; use_rh_pct: number; daily_swing_c: number;
  ea_ev: number; fit_ref_c: number; mic_k: number; battery_counts: boolean;
  blocks: Record<string, { fit?: number | null; beta?: number | null; eta?: number | null; source?: string | null }>;
  dvt_reliability: number; dvt_confidence: number; dvt_failures: number; life_multiple: number; life_beta: number; qual_units: number;
}
export interface UseProfile {
  mission_years: number; hours_per_day: number; mission_h: number; mission_days: number; battery_life_h: number; charge_cycles_per_day: number;
  hinge_cycles_per_day: number; ambient_c: number; board_c: number; pod_heat_mw: number; temp_factor: number; use_rh_pct: number; daily_swing_c: number;
}
export interface IrisOut { config: RelConfig; profile: UseProfile; tree: RbdNode; rbd: RbdOut; excluded: { refdes: string; name: string; reason: string }[]; r_2yr: number }

export interface DvtTest {
  id: string; leg: string; name: string; standard: string; severity: string; procedure: string[]; equipment: string; resource: string;
  units: number; pass: string; linked: string[]; duration_d: number; start_d: number; end_d: number; basis: string;
}
export interface DvtLeg { id: string; name: string; sample_basis: string; units: number; product_units: boolean }
export interface MatrixRow { id: string; text: string; kind: string; method: string; dvt: string; equipment: string; units: string; pass: string; tests: string[] }
export interface DvtOut {
  legs: DvtLeg[]; tests: DvtTest[]; matrix: MatrixRow[]; total_units: number; duration_d: number; cell_duration_d: number; resources: string[];
  reliability: number; confidence: number; life_multiple: number; life_beta: number; csv: string; markdown: string; notes: string[];
}
