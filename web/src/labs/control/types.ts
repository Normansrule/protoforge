// TypeScript mirrors of crates/forge-core/src/control/*.rs outputs.
import type { Complex } from './kit';

export interface Tf { num: number[]; den: number[]; delay: number }
export interface Pid { kp: number; ki: number; kd: number; n: number }

export interface ParamSpec { key: string; label: string; unit: string; value: number; min: number; max: number; log: boolean }
export interface PlantInfo {
  id: string; name: string; description: string; formula: string; input: string; output: string;
  params: ParamSpec[]; pid: Pid; u_max: number; u_min: number; iris: string;
}

export interface StepMetrics {
  rise_time: number | null; peak: number; peak_time: number; overshoot_pct: number;
  settling_time: number | null; final_value: number; ss_error: number; stable: boolean;
}
export interface Bode { w: number[]; mag_db: number[]; phase_deg: number[] }
export interface Margins { gm: number | null; gm_db: number | null; w_pc: number | null; pm_deg: number | null; w_gc: number | null; delay_margin: number | null }
export interface RootLocus { gains: number[]; branches: Complex[][]; open_poles: Complex[]; open_zeros: Complex[]; current: Complex[]; k_unstable: number | null }
export interface LoopSim { t: number[]; y: number[]; u: number[]; r: number[]; saturated_frac: number; diverged: boolean }
export interface Response { t: number[]; y: number[]; u?: number[] }

export interface AnalyzeOut {
  plant: Tf; controller: Tf; loop_tf: Tf; closed_loop: Tf;
  plant_poles: Complex[]; plant_zeros: Complex[]; closed_poles: Complex[]; stable: boolean;
  open_step: Response; open_metrics: StepMetrics; sim: LoopSim; metrics: StepMetrics; t_end: number;
  bode_plant: Bode; bode_loop: Bode; bode_closed: Bode; margins: Margins;
  nyquist: { re: number[]; im: number[] }; root_locus: RootLocus; ms: number; bandwidth: number | null;
}

export interface Tuning {
  pid: Pid; method: string; notes: string[]; ku: number | null; pu: number | null;
  model: { k: number; tau1: number | null; tau2: number; theta: number; integrating: boolean; method: string } | null;
  tau_c: number | null;
}

export interface Estimate {
  id: string; name: string; roll: number[]; pitch: number[]; yaw: (number | null)[];
  rms_roll: number; rms_pitch: number; rms_tilt: number; max_tilt_err: number; final_yaw_err: number | null; has_yaw: boolean;
}
export interface HeadResult {
  t: number[]; true_roll: number[]; true_pitch: number[]; true_yaw: number[];
  accel_norm_g: number[]; gyro_y_dps: number[]; estimates: Estimate[];
  events: { t0: number; t1: number; label: string }[];
  cf_crossover_hz: number; cf_tau_s: number; kf_bias_est: [number, number];
  gyro_sigma_dps: number; accel_sigma_mg: number; samples: number; sweep: [number, number][];
}

export interface CartPoleParams { cart_mass: number; pole_mass: number; l: number; inertia: number; friction: number; g: number }
export interface CartPoleResult {
  a: number[][]; b: number[]; open_poles: Complex[]; k: number[]; p: number[][]; closed_poles: Complex[];
  t: number[]; x: number[]; xdot: number[]; theta: number[]; thetadot: number[]; force: number[]; xref: number[];
  fell: boolean; fell_at: number | null; max_abs_x: number; max_abs_force: number; saturated_frac: number; cost: number;
}
