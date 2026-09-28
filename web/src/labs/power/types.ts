// TypeScript mirror of crates/forge-core/src/power/{budget,battery,thermal}.rs outputs.

export interface UsageProfile {
  subsystem_scale: Record<string, number>;
  subsystem_duty: Record<string, number>;
  component_duty: Record<string, number>;
  display_duty: number | null;
}

export interface ComponentPower {
  refdes: string; name: string; subsystem: string; location: string; qty: number;
  active_mw: number; sleep_mw: number; duty: number; bom_duty: number;
  active_part_mw: number; sleep_part_mw: number; avg_mw: number; share: number; peak_mw: number;
}

export interface SubsystemPower { id: string; name: string; avg_mw: number; active_part_mw: number; sleep_part_mw: number; share: number; components: string[] }

export interface Budget {
  components: ComponentPower[];
  subsystems: SubsystemPower[];
  load_mw: number; active_mw: number; sleep_mw: number; conversion_loss_mw: number; quiescent_mw: number;
  avg_mw: number; peak_mw: number; energy_mwh: number; rated_energy_mwh: number; life_h: number;
  avg_current_ma: number; peak_current_ma: number; c_rate: number; buck_efficiency: number;
  display_ref: string | null;
  profile: UsageProfile;
  scenario: string;
}

export interface ScenarioResult {
  id: string; name: string; description: string; avg_mw: number; life_h: number; display_duty: number;
  subsystems: SubsystemPower[]; profile: UsageProfile;
}

export interface DischargeCurve {
  label: string; t_h: number[]; soc: number[]; ocv_v: number[]; v_term: number[]; current_ma: number[]; v_peak: number[];
  cutoff_h: number; end_reason: string; soc_at_cutoff: number; delivered_mah: number; delivered_mwh: number;
  brownout_h: number | null; equivalent_usable_fraction: number; capacity_mah: number; r_ohm: number;
}

export interface DischargeResult {
  main: DischargeCurve; comparisons: DischargeCurve[]; load_mw: number; peak_mw: number; cutoff_v: number; full_v: number;
  capacity_factor: number; r_factor: number; budget_life_h: number; budget_usable_fraction: number;
  ocv_curve: [number, number][]; max_power_mw: number;
}

export interface OcvOut { soc: number[]; ocv_v: number[]; temp_c: number[]; capacity_factor: number[]; r_factor: number[] }

export interface Throttle { enabled: boolean; threshold_c: number; hysteresis_c: number; throttled_duty: number }

export interface ThermalResult {
  t_s: number[]; board_c: number[]; pod_c: number[]; power_mw: number[]; display_duty: number[];
  q_board_pod_mw: number[]; q_pod_air_mw: number[]; throttled: boolean[];
  ambient_c: number; skin_limit_c: number; p_display_off_mw: number; p_display_on_mw: number; p_avg_mw: number;
  steady_board_c: number; steady_pod_c: number; max_board_c: number; max_pod_c: number; t_skin_limit_s: number | null;
  eigenvalues: [number, number]; time_constants_s: [number, number]; a_matrix: [[number, number], [number, number]];
  throttled_fraction: number; mean_display_duty: number; max_safe_display_duty: number;
  network: { r_bp: number; r_pa: number; c_b: number; c_p: number } | null;
}

export interface SweepResult {
  ambient_c: number[]; pod_c: number[]; board_c: number[]; pod_ref_c: number[]; max_ambient_c: number;
  duty: number[]; pod_vs_duty_c: number[]; board_vs_duty_c: number[]; max_safe_duty: number; sweep_ambient_c: number; skin_limit_c: number;
}

export interface TradeOptionPoint { name: string; capacity_mah: number; life_h: number; total_mass_g: number; feasible: boolean }

export interface TradeResult {
  capacity_mah: number[]; life_h: number[]; battery_mass_g: number[]; total_mass_g: number[];
  other_mass_g: number; g_per_mah: number; avg_mw: number; min_mah_for_life: number; max_mah_for_mass: number;
  feasible: boolean; current_mah: number; current_life_h: number; current_mass_g: number;
  life_req_h: number; mass_req_g: number; trade_id: string | null; options: TradeOptionPoint[]; mass_source: string;
}
