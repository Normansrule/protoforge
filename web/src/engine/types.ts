// TypeScript mirror of crates/forge-core/src/project.rs and systems/mod.rs.

export type ReqKind =
  | 'performance' | 'functional' | 'interface' | 'environmental' | 'safety'
  | 'regulatory' | 'manufacturing' | 'cost' | 'goal';
export type VerifyMethod = 'analysis' | 'test' | 'inspection' | 'demonstration';
export type Cmp = 'le' | 'ge' | 'eq' | 'between';
export type Domain = 'electrical' | 'mechanical' | 'optical' | 'software' | 'silicon' | 'acoustic' | 'thermal' | 'manufacturing';

export interface Meta { id: string; name: string; codename: string; version: string; tagline: string; description: string; author: string }
export interface UseCase { id: string; title: string; narrative: string }
export interface Concept { problem: string; vision: string; users: string[]; use_cases: UseCase[]; constraints: string[] }
export interface Need { id: string; text: string; stakeholder: string; priority: number }
export interface Verification { method: VerifyMethod; metric: string | null; cmp: Cmp; target: number; target_hi: number | null; unit: string }
export interface Requirement {
  id: string; text: string; rationale: string; kind: ReqKind;
  derives_from: string[]; allocated_to: string[]; verification: Verification;
}
export interface Subsystem { id: string; name: string; domain: Domain; description: string; functions: string[]; lab: string }
export interface Interface { id: string; from: string; to: string; kind: string; description: string }
export interface PowerProfile { active_mw: number; sleep_mw: number; duty: number }
export interface Component {
  refdes: string; name: string; part_class: string; example_part: string; subsystem: string;
  qty: number; unit_cost_usd: number; mass_g: number; power: PowerProfile; footprint: string;
  on_pcb: boolean; location: string;
}
export interface Criterion { name: string; weight: number; higher_is_better?: boolean }
export interface TradeOption { name: string; scores: number[]; notes: string }
export interface TradeStudy { id: string; title: string; decision: string; criteria: Criterion[]; options: TradeOption[] }
export interface Risk {
  id: string; item: string; failure_mode: string; effect: string; cause: string;
  severity: number; occurrence: number; detection: number; mitigation: string;
}
export interface Milestone { id: string; title: string; stage: string; deliverables: string[]; exit_criteria: string[] }

export interface FrameParams {
  lens_width_mm: number; lens_height_mm: number; bridge_mm: number; temple_length_mm: number;
  front_thickness_mm: number; rim_width_mm: number; lens_shape_exponent: number;
  temple_width_mm: number; temple_height_mm: number; pod_length_mm: number; pod_width_mm: number;
  pod_height_mm: number; battery_pod_length_mm: number; wall_mm: number; draft_deg: number;
  pantoscopic_tilt_deg: number; material: string; density_g_cm3: number;
}
export interface BatteryParams { chemistry: string; capacity_mah: number; nominal_v: number; full_v: number; cutoff_v: number; internal_resistance_ohm: number; mass_g: number; usable_fraction: number }
export interface PowerRails { core_v: number; buck_efficiency: number; display_rail_v: number; quiescent_ua: number }
export interface OpticsParams {
  display_width_mm: number; display_height_mm: number; px_w: number; px_h: number; lens_focal_mm: number;
  lens_diameter_mm: number; display_to_lens_mm: number; lens_to_combiner_mm: number; eye_relief_mm: number;
  combiner_reflectance: number; display_luminance_nits: number;
}
export interface AudioParams { mic_count: number; mic_spacing_mm: number; sample_rate_hz: number; pdm_clock_hz: number; decimation: number; fir_taps: number; voice_band_lo_hz: number; voice_band_hi_hz: number }
export interface ImuParams { rate_hz: number; gyro_noise_dps_rthz: number; gyro_bias_dps: number; accel_noise_mg_rthz: number; fusion_alpha: number }
export interface PcbParams { width_mm: number; height_mm: number; layers: number; thickness_mm: number; min_trace_mm: number; min_clearance_mm: number; via_drill_mm: number; via_pad_mm: number; grid_mm: number }
export interface ThermalParams { ambient_c: number; r_pod_ambient_c_per_w: number; r_board_pod_c_per_w: number; c_pod_j_per_c: number; c_board_j_per_c: number; skin_limit_c: number }
export interface MfgParams { quantities: number[]; target_quantity: number; molding_material: string; cavities: number; mold_steel: string; assembly_minutes: number; labor_rate_usd_h: number }
export interface PrintParams { nozzle_mm: number; layer_height_mm: number; line_width_mm: number; perimeters: number; infill_pct: number; print_speed_mm_s: number; travel_speed_mm_s: number; filament_diameter_mm: number; nozzle_temp_c: number; bed_temp_c: number }
export interface ChipParams { clock_mhz: number; coeff_bits: number; data_bits: number; gate_delay_ns: number; cell_budget: number }
export interface NamedNetlist { id: string; title: string; description: string; netlist: string }

export interface Design {
  frame: FrameParams; battery: BatteryParams; power_rails: PowerRails; optics: OpticsParams;
  audio: AudioParams; imu: ImuParams; pcb: PcbParams; thermal: ThermalParams;
  manufacturing: MfgParams; print: PrintParams; chip: ChipParams; circuits: NamedNetlist[];
  extra: Record<string, unknown>;
}

export interface Project {
  meta: Meta; concept: Concept; needs: Need[]; requirements: Requirement[]; subsystems: Subsystem[];
  interfaces: Interface[]; components: Component[]; trades: TradeStudy[]; risks: Risk[];
  milestones: Milestone[]; design: Design;
}

export interface Metric { key: string; label: string; value: number; unit: string; source: string; note: string }
export type Status = 'pass' | 'fail' | 'unverified';
export interface ReqResult { id: string; status: Status; value: number | null; unit: string; margin: number | null; source: string | null; criterion: string }
export interface Evaluation { metrics: Metric[]; results: ReqResult[]; pass: number; fail: number; unverified: number; /** failing requirements of kind "goal" (included in fail) */ open_goals?: number }
