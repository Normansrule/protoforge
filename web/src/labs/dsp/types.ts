// TypeScript mirrors of crates/forge-core/src/dsp/*.rs outputs.

export type WindowKind = 'rectangular' | 'hann' | 'hamming' | 'blackman' | 'kaiser';
export type ComponentKind = 'tone' | 'noise' | 'chirp' | 'square' | 'saw' | 'voice' | 'impulse';

export interface SigComponent {
  kind: ComponentKind;
  freq: number;
  freq_end: number;
  amp: number;
  phase_deg: number;
  bandlimited: boolean;
  vowel: string;
  seed: number;
}

export interface SignalSpec {
  fs: number;
  n: number;
  components: SigComponent[];
}

export interface Signal { fs: number; x: number[]; rms: number; peak: number }

export interface Spectrum {
  freq: number[]; amp: number[]; db: number[]; nfft: number; bin_hz: number;
  coherent_gain: number; enbw_bins: number; peaks: [number, number][];
}

export interface AnalyzeOut { signal: Signal; spectrum: Spectrum }

export interface WindowInfo {
  kind: WindowKind; n: number; beta: number; w: number[]; coherent_gain: number; enbw_bins: number;
  scalloping_loss_db: number; peak_sidelobe_db: number; mainlobe_width_bins: number; bins: number[]; response_db: number[];
}

export interface LeakageTrace { window: WindowKind; bins: number[]; db: number[]; peak_db: number; leakage_db: number }

export interface Biquad { b: [number, number, number]; a: [number, number, number] }

export interface FilterReport {
  b: number[]; a: number[]; sections: Biquad[]; fs: number;
  freq: number[]; mag_db: number[]; phase_deg: number[]; group_delay: number[];
  impulse: number[]; step: number[]; poles: [number, number][]; zeros: [number, number][];
  origin_poles: number; stable: boolean; max_pole_radius: number; taps: number; order: number;
  beta: number | null; notes: string[];
}

export interface QFormat { bits: number; int_bits: number; frac_bits: number; codes: number[]; values: number[]; max_error: number }

export interface FixedOut {
  coeff_bits: number; data_bits: number; b_q: QFormat; a_q: QFormat | null; sections_q: Biquad[];
  freq: number[]; float_db: number[]; fixed_db: number[]; stable_fixed: boolean; max_pole_radius_fixed: number;
  y_float: number[]; y_fixed: number[]; output_snr_db: number; coeff_snr_db: number; theory_snr_db: number;
}

export interface ChainDesign {
  pdm_hz: number; fs_out: number; decimation: number; cic_order: number; cic_r: number; cic_m: number;
  cic_out_hz: number; cic_register_bits: number; cic_dc_gain: number; halfband: number[]; fir: number[];
  fir_in_hz: number; fir_decim: number; pass_hz: number; stop_hz: number; notes: string[];
}

export interface Trace { freq: number[]; db: number[] }

export interface ChainSim {
  design: ChainDesign; stopband_db: number; stopband_worst_hz: number; cic_floor_db: number;
  min_taps_60db: number | null; passband_ripple_db: number; alias_bands: [number, number][];
  resp_freq: number[]; resp_cic_db: number[]; resp_hb_db: number[]; resp_fir_db: number[]; resp_total_db: number[];
  pass_freq: number[]; pass_cic_db: number[]; pass_total_db: number[];
  strip_bits: number[]; strip_input: number[]; pdm_spectrum: Trace; ntf_db: number[];
  cic_spectrum: Trace; out_spectrum: Trace; out_t: number[]; out_x: number[]; sndr_db: number; enob: number;
  coeff_bits: number; stopband_quantized_db: number;
}

export type Pattern = 'omni' | 'dipole' | 'cardioid' | 'hypercardioid' | 'supercardioid' | 'custom' | 'delay_sum';

export interface Polar { f: number; angle_deg: number[]; gain_db: number[]; gain: number[] }

export interface PatternRow {
  pattern: Pattern; tau_ratio: number; tau_us: number; di_1k_db: number; di_ideal_db: number;
  null_deg: number | null; front_back_db: number; wng_1k_db: number;
}

export interface BeamResult {
  spacing_mm: number; pattern: Pattern; tau_ratio: number; tau_us: number; alpha: number; null_deg: number | null;
  polar: Polar[]; freq: number[]; di_db: number[]; di_das_db: number[]; wng_db: number[]; wng_das_db: number[];
  onaxis_db: number[]; eq_db: number[]; equalized_db: number[]; noise_boost_db: number[];
  di_1k_db: number; wng_1k_db: number; spatial_alias_hz: number; table: PatternRow[];
}

export interface FieldResult {
  nx: number; ny: number; x: number[]; y: number[]; level_db: number[];
  probe_db: number[]; probe_angle_deg: number[]; mics: [number, number][];
}

export const PATTERN_LABEL: Record<Pattern, string> = {
  omni: 'Omni (one mic)',
  dipole: 'Dipole (figure-8)',
  cardioid: 'Cardioid',
  hypercardioid: 'Hypercardioid',
  supercardioid: 'Supercardioid',
  custom: 'Custom delay',
  delay_sum: 'Delay-and-sum',
};

export const WINDOW_LABEL: Record<WindowKind, string> = {
  rectangular: 'Rectangular',
  hann: 'Hann',
  hamming: 'Hamming',
  blackman: 'Blackman',
  kaiser: 'Kaiser',
};
