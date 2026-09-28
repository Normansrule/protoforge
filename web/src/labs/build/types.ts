// Types mirroring crates/forge-core/src/build/*.rs (serde output).

export interface Range { label: string; lo: number; hi: number; unit: string; scale: [number, number]; note?: string }
export interface Fact { label: string; value: string }
export interface MaterialCard { id: string; name: string; ranges: Range[]; facts: Fact[]; notes: string[] }
export interface Rule { title: string; value: string; detail: string; diagram: string }
export interface Failure { name: string; symptom: string; causes: string[]; fixes: string[]; diagram: string }
export interface LabLink { label: string; route: string }
export interface ProjectCheck { label: string; value: number; lo: number; hi: number | null; unit: string; status: 'pass' | 'warn'; note: string }
export interface Process {
  id: string; name: string; group: string; summary: string; caveat: string; anim: string;
  materials: MaterialCard[]; rules: Rule[]; failures: Failure[]; tips: string[]; links: LabLink[]; refs: string[]; project_checks: ProjectCheck[];
}
export interface LaserMaterial { name: string; verdict: 'safe' | 'caution' | 'never'; why: string }
export interface FileOut { filename: string; content: string }

export interface MachMaterial {
  id: string; name: string; sfm_carbide: [number, number]; sfm_hss: [number, number]; chip_mm_6: [number, number];
  unit_power: [number, number]; unit_power_estimated: boolean; flutes: string; coolant: string; notes: string;
}
export interface FeedsOut {
  material: MachMaterial; sfm_range: [number, number]; chip_range_mm: [number, number]; sfm_target: number; vc_target_m_min: number;
  rpm_ideal: number; rpm: number; rpm_limited: boolean; sfm_actual: number; vc_actual_m_min: number; chip_load_mm: number;
  chip_thinning_factor: number; chip_load_adjusted_mm: number; feed_mm_min: number; feed_ipm: number; mrr_cm3_min: number; mrr_in3_min: number;
  power_spindle_w: number; power_motor_w: number; power_range_w: [number, number]; torque_nm: number; radial_engagement_pct: number;
  warnings: string[]; steps: string[];
}
export interface SheetMaterial { id: string; name: string; min_r_over_t: [number, number]; notes: string }
export interface BendOut {
  k_factor: number; k_source: string; bend_allowance_mm: number; outside_setback_mm: number; bend_deduction_mm: number; flat_length_mm: number;
  flat_straight1_mm: number; flat_straight2_mm: number; min_radius_mm: [number, number]; min_flange_mm: number; hole_to_bend_mm: number; checks: [string, boolean][];
}
export interface Alloy {
  id: string; name: string; liquidus_c: number; soak_lo_c: number; soak_hi_c: number; soak_s: [number, number]; tal_s: [number, number];
  peak_c: [number, number]; max_ramp_c_s: number; max_cool_c_s: number; max_time_to_peak_s: number; iron_c: [number, number]; notes: string;
  default_zones: number[]; default_speed_cm_min: number;
}
export interface ProbeStats { peak_c: number; tal_s: number; soak_s: number; max_ramp_c_s: number; max_cool_c_s: number; time_to_peak_s: number }
export interface ReflowCheck { probe: string; label: string; value: number; unit: string; lo: number | null; hi: number | null; pass: boolean }
export interface ReflowOut {
  alloy: Alloy; t_s: number[]; x_mm: number[]; air_c: number[]; small_c: number[]; large_c: number[]; zone_entry_s: number[];
  oven_length_mm: number; dwell_per_zone_s: number; small: ProbeStats; large: ProbeStats; checks: ReflowCheck[]; pass: boolean; max_delta_t_c: number;
}
export interface StencilOut { area_ratio: number; aspect_ratio: number; area_ok: boolean; aspect_ok: boolean; max_thickness_mm: number; paste_volume_mm3: number }
export interface PadRow { footprint: string; pad_w_mm: number; pad_h_mm: number; count: number; refdes: string[]; area_ratio: number; aspect_ratio: number; pass: boolean }
export interface StencilCheck { thickness_mm: number; rows: PadRow[]; min_area_ratio: number; worst: string; failing: number; notes: string[] }

export interface Tier {
  id: string; name: string; trace_mm: number; space_mm: number; drill_mm: number; annular_mm: number; edge_mm: number; mask_web_mm: number;
  max_layers: number; min_thickness_mm: number; max_thickness_mm: number; finishes: string[]; via_in_pad: boolean; impedance: string; notes: string;
}
export interface DfmItem { rule: string; design: number; limit: number; unit: string; status: 'pass' | 'fail' | 'warn' | 'info'; source: string; note: string }
export interface Dfm { tier: Tier; items: DfmItem[]; violations: number; warnings: number; cheapest_ok: string | null; has_layout: boolean; min_pitch_mm: number | null }

export interface ProcessCap {
  id: string; name: string; materials: string[]; geometry: number[]; qty: [number, number]; tol_mm: number; finish: number; strength: number;
  lead_days: [number, number]; setup: number; unit: number; max_size_mm: number; vendor_categories: string[]; upload: string; lab_route: string; lab_label: string; note: string;
}
export interface Weights { quantity: number; cost: number; tolerance: number; finish: number; strength: number; lead_time: number; size: number; geometry: number }
export interface Sub { criterion: string; score: number; weight: number; reason: string }
export interface Ranked { process: ProcessCap; score: number; feasible: boolean; blockers: string[]; subs: Sub[]; cost_index: number; cost_band: string; lead_band: string; reasons: string[] }
export interface WizardOut { ranked: Ranked[]; weights: Weights; notes: string[] }

export interface Output { label: string; route: string }
export interface Vendor {
  name: string; url: string; categories: string[]; services: string; formats: string[]; send: Output[]; returns: string; region: string; category_card: boolean; note: string;
}
export interface Category { id: string; label: string }

export interface Trace { kind: string; id: string; label: string }
export interface ComplianceItem {
  id: string; title: string; category: string; markets: string[]; standards: string[]; why: string; traces: Trace[]; stage: 'EVT' | 'DVT' | 'PVT';
  obligation: 'mandatory' | 'conditional' | 'voluntary'; route: string; cost_usd: [number, number] | null; weeks: [number, number]; notes: string; explain: string;
}
export interface Phase { id: string; label: string; start_week: number; end_week: number; milestones: string[] }
export interface Bar { id: string; label: string; stage: string; start_week: number; weeks: number; activity: boolean }
export interface Plan {
  markets: string[]; features: Record<string, boolean | string[]>; items: ComplianceItem[]; phases: Phase[]; timeline: Bar[]; total_cost_usd: [number, number]; mandatory: number; notes: string[];
}
export interface BuildStep { what: string; process: string; vendor_category: string; route: string }
export interface RoadStage {
  id: string; title: string; question: string; quantity: string; builds: BuildStep[]; tests: string[]; exit: string[]; milestones: string[];
  compliance: string[]; risks: string[]; typical_weeks: [number, number];
}
export interface GridOut { svg: string; cells: { row: number; col: number; power_pct: number; speed: number; color: string }[]; width_mm: number; height_mm: number; csv: string }

export interface ScaleOut {
  energy_ratio: number; speed_same_power_mm_s: number; power_same_speed_pct: number; passes: number; speed_per_pass_mm_s: number;
  grid_speed: [number, number]; grid_power: [number, number]; steps: string[];
}
export interface LaserType { name: string; wavelength: string; cuts: string; cannot: string; where: string }
export interface ResinFit { dp_mm: number; ec_mj_cm2: number; r2: number; n: number }
export interface ResinOut {
  dp_mm: number; ec_mj_cm2: number; fitted: ResinFit | null; cure_mm: number; exposure_mj_cm2: number; time_s: number; bottom_time_s: number; bottom_cure_mm: number;
  curve_e: number[]; curve_cd: number[]; table: { layer_mm: number; cure_mm: number; exposure_mj_cm2: number; time_s: number }[]; steps: string[]; warnings: string[];
}
export interface IpcClass { class: number; name: string; typical: string; effect: string }
