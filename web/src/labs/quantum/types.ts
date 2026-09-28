// Mirrors of the quantum.* engine outputs (crates/forge-core/src/quantum).

export interface Cond { bits: number[]; value: number }
export interface Op {
  gate: string;
  targets: number[];
  controls?: number[];
  params?: number[];
  clbit?: number | null;
  cond?: Cond | null;
  label?: string | null;
}
export interface Creg { name: string; size: number }
export interface Circuit { qubits: number; clbits: number; cols: Op[][]; cregs?: Creg[] }

export interface GateInfo {
  id: string; label: string; name: string; targets: number; params: string[]; group: string; qasm: string; description: string;
}

export interface QubitInfo { bloch: [number, number, number]; radius: number; p1: number; purity: number; entropy: number }
export interface MeasureEvent { qubit: number; clbit: number | null; outcome: number; probability: number }
export interface Snapshot {
  column: number;
  basis: number[];
  labels: string[];
  re: number[];
  im: number[];
  prob: number[];
  qubits: QubitInfo[];
  concurrence: number[] | null;
  clbits: number[];
  events: MeasureEvent[];
}
export interface HistEntry { bits: string; count: number; prob: number }
export interface RunOut {
  qubits: number; clbits: number; columns: number;
  state: Snapshot; steps: Snapshot[];
  histogram: HistEntry[]; histogram_over: string; shots: number; exact: HistEntry[];
  sampling: string; warnings: string[];
}
export interface ExportOut { qasm: string | null; qasm_error: string | null; qasm3: string | null; qasm3_error: string | null; qiskit: string | null; quirk_url: string | null; quirk_json: string | null; warnings: string[] }
export interface ParseOut { circuit: Circuit; qregs: Creg[]; warnings: string[]; ops: number }

export interface Listing { id: string; title: string; group: string; summary: string }
export interface Expected { label: string; value: string }
export interface Entry {
  id: string; title: string; group: string; summary: string; explanation: string;
  circuit: Circuit; expected: Expected[];
  extra?: {
    iterations: number[]; p_marked_sim: number[]; p_marked_theory: number[]; distributions: number[][];
    theta: number; optimal: number; marked: number;
  };
}

export interface ChshSetting { a: number; b: number; e_exact: number; e_sampled: number; same: number; diff: number }
export interface ChshOut {
  settings: ChshSetting[]; s_exact: number; s_sampled: number; s_std: number; classical_bound: number; tsirelson: number;
  violated: boolean; v_threshold: number; sweep_x: number[]; sweep_s: number[];
}

export interface Noise {
  depolarizing: number; bit_flip: number; phase_flip: number; amplitude_damping: number; phase_damping: number; readout: number; idle: boolean;
}
export interface DmStep { column: number; qubits: QubitInfo[]; fidelity: number | null; purity: number; probs: number[] }
export interface DmOut {
  qubits: number; steps: DmStep[]; labels: string[]; probs_measured: number[]; probs_ideal: number[];
  fidelity: number | null; purity: number; entropy: number; rho_re: number[] | null; rho_im: number[] | null;
  concurrence01: number | null; concurrence01_ideal: number | null; total_variation: number; notes: string[];
}
export interface SweepOut { channel: string; p: number[]; fidelity: number[]; purity: number[]; success: number[]; target: string }
export interface ChannelOut { m: number[][]; t: number[]; trajectory: [number, number, number][]; purity: number[] }

export interface Pulse {
  alice_bit: number; alice_basis: number; eve_basis: number | null; eve_bit: number | null; detected: boolean;
  bob_basis: number; bob_bit: number | null; sifted: boolean; error: boolean; sampled: boolean;
}
export interface Bb84Out {
  pulses: Pulse[]; sent: number; detected: number; sifted: number; errors: number; qber: number; qber_estimate: number; qber_expected: number;
  sampled: number; secret_fraction: number; final_key_bits: number; leak_ec_bits: number; pa_removed_bits: number; threshold: number;
  abort: boolean; curve_q: number[]; curve_r: number[];
}
export interface LinkParams {
  distance_km: number; fiber_loss_db_km: number; extra_loss_db: number; detector_efficiency: number; dark_count_prob: number;
  misalignment: number; ec_efficiency: number; source_rate_hz: number; mu: number; sifting: number; dead_time_s: number;
}
export interface LinkPoint { distance_km: number; eta: number; mu: number; gain: number; qber: number; y1: number; q1: number; e1: number; rate_per_pulse: number; key_rate_bps: number }
export interface LinkOut {
  params: LinkParams; distance_km: number[]; decoy_bps: number[]; gllp_bps: number[]; plob_bps: number[]; mu_opt: number[]; qber_pct: number[];
  max_distance_km: number; max_distance_gllp_km: number; design: LinkPoint; design_plob_bps: number; design_gllp_bps: number; plob_ratio: number; loss_db: number;
  budget: { label: string; rate: number; note: string }[];
}
export interface E91Pair { a: number; b: number; x: number; y: number; key: boolean; eve: boolean }
export interface E91Out {
  alice_deg: number[]; bob_deg: number[]; pairs: E91Pair[]; s: number; s_expected: number; correlations: number[];
  key_bits: number; qber: number; secret_fraction: number; secure: boolean;
}

export interface TestResult { name: string; section: string; p_value: number; p_value2: number | null; pass: boolean; statistic: string; note: string; applicable: boolean }
export interface Stage {
  id: string; name: string; bits: number; ones_fraction: number; efficiency: number; rate_bps: number; min_entropy_per_bit: number;
  tests: TestResult[]; passed: number; applicable: number; preview: string; hex: string; autocorr: number[];
}
export interface QrngIn {
  source: 'diode' | 'photon'; samples: number; seed: number;
  split_ratio: number; eff1: number; eff0: number; photon_rate_hz: number;
  noise_rms_mv: number; flicker_rms_mv: number; offset_mv: number; hum_mv: number; hum_hz: number; bandwidth_hz: number;
  sample_rate_hz: number; adc_bits: number; adc_vpp_mv: number; digitize: 'comparator' | 'lsb'; lsb_bits: number;
  von_neumann: boolean; xor_fold: number; toeplitz: boolean; toeplitz_block: number; security_bits: number;
}
export interface QrngOut {
  source: string; analog_mv: number[]; codes: number[]; hist_x: number[]; hist_counts: number[]; hist_expected: number[];
  bits_per_sample: number; clipped_fraction: number; total_rms_mv: number; lsb_mv: number; min_entropy_per_sample: number;
  min_entropy_theory: number; raw_bias: number; photon_p1: number | null; sample_rate_hz: number; stages: Stage[]; output_rate_bps: number; notes: string[];
}
export interface NistOut { n: number; ones_fraction: number; tests: TestResult[]; min_entropy_per_bit: number; min_entropy_per_byte: number | null; autocorr: number[] }
