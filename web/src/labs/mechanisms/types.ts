// Types mirroring crates/forge-core/src/mechanisms/*.rs outputs.

export interface Check { name: string; ok: boolean; detail: string }

export interface Grashof {
  s: number; l: number; p: number; q: number; s_plus_l: number; p_plus_q: number;
  grashof: boolean; change_point: boolean; code: string; name: string; description: string; crank_full_rotation: boolean;
}

export interface FourBarIn {
  a: number; b: number; c: number; d: number; o2: [number, number]; theta1_deg: number;
  p: number; delta3_deg: number; branch: 'open' | 'crossed'; omega2: number; alpha2: number; samples: number;
}

export interface FourBarOut {
  grashof: Grashof;
  theta2_deg: number[]; valid: boolean[]; theta3_deg: number[]; theta4_deg: number[];
  omega3: number[]; omega4: number[]; alpha3: number[]; alpha4: number[]; mu_deg: number[];
  ax: number[]; ay: number[]; bx: number[]; by: number[]; px: number[]; py: number[];
  p_speed: number[]; p_accel: number[];
  o2: [number, number]; o4: [number, number];
  mu_min_deg: number; mu_max_deg: number; toggle_theta2_deg: number[];
  rocker_min_deg: number; rocker_max_deg: number; time_ratio: number; notes: string[];
}

export interface SliderOut {
  theta2_deg: number[]; valid: boolean[]; x: number[]; v: number[]; a: number[]; theta3_deg: number[]; mu_deg: number[];
  stroke_mm: number; x_min: number; x_max: number; time_ratio: number; a_max: number; a_approx: number; notes: string[];
}

export interface Synth3Out {
  o2: [number, number]; o4: [number, number]; a1: [number, number]; b1: [number, number];
  a: number; b: number; c: number; d: number; theta1_deg: number; p: number; delta3_deg: number;
  theta2_deg: [number, number, number]; branch: 'open' | 'crossed'; grashof: Grashof; notes: string[];
}

export interface WireMaterial {
  id: string; name: string; spec: string; a_mpa: number; m: number; d_min_mm: number; d_max_mm: number;
  ssy_frac: number; sy_bend_frac: number; density_g_cm3: number; max_temp_c: number; notes: string;
}

export interface LifeEstimate { cycles: number; runout: boolean; extrapolated: boolean; r0_frac: number }

export interface CompressionOut {
  material: string; sut_mpa: number; ssy_mpa: number; ssu_mpa: number; e_mpa: number; g_mpa: number;
  index_c: number; k_b: number; k_w: number; od_mm: number; id_mm: number; rate_n_mm: number; total_coils: number;
  solid_len_mm: number; pitch_mm: number; len_at_min_mm: number; len_at_max_mm: number; defl_max_mm: number;
  f_solid_n: number; tau_solid_mpa: number; ns_solid: number; tau_min_mpa: number; tau_max_mpa: number;
  tau_a_mpa: number; tau_m_mpa: number; ns_static: number; sse_mpa: number; nf_goodman: number; life: LifeEstimate;
  overrun: number; buckling_alpha: number; slenderness: number; buckling_stable_absolute: boolean; buckling_defl_mm: number;
  surge_hz: number; mass_g: number; curve_y: number[]; curve_f: number[]; checks: Check[]; notes: string[];
}

export interface ExtensionOut {
  sut_mpa: number; index_c: number; k_b: number; active_coils: number; rate_n_mm: number; free_len_mm: number;
  tau_i_mpa: number; tau_i_pref_lo_mpa: number; tau_i_pref_hi_mpa: number; defl_max_mm: number;
  tau_body_max_mpa: number; sigma_hook_a_max_mpa: number; tau_hook_b_max_mpa: number;
  ns_body: number; ns_hook_bending: number; ns_hook_torsion: number; nf_body: number; nf_hook_bending: number;
  curve_y: number[]; curve_f: number[]; checks: Check[]; notes: string[];
}

export interface TorsionOut {
  sut_mpa: number; sy_mpa: number; index_c: number; k_i: number; active_turns: number;
  rate_nmm_per_turn: number; rate_nmm_per_rad: number; defl_max_deg: number; sigma_max_mpa: number; sigma_min_mpa: number;
  ns_static: number; nf_goodman: number; mean_d_loaded_mm: number; id_loaded_mm: number; arbor_clearance_mm: number;
  curve_deg: number[]; curve_m: number[]; checks: Check[]; notes: string[];
}

export interface GearIn {
  module_mm: number; teeth: number; pressure_deg: number; shift: number; addendum?: number; dedendum?: number;
  root_radius?: number; tip_shorten?: number; thinning_mm?: number; samples?: number;
}

export interface GearProfile {
  teeth: number; module_mm: number; pressure_deg: number; shift: number;
  pitch_r: number; base_r: number; tip_r: number; root_r: number; form_r: number; rack_tip_r: number;
  undercut: boolean; pointed: boolean; tip_thickness_mm: number; pitch_thickness_mm: number; base_pitch_mm: number;
  z_min: number; x_min: number; flank_r: number[]; flank_psi: number[]; form_index: number;
  lewis_y: number; lewis_section_r: number; lewis_thickness_mm: number; lewis_height_mm: number; notes: string[];
}

export interface PairOut {
  g1: GearProfile; g2: GearProfile; ratio: number; std_center_mm: number; center_mm: number; working_pressure_deg: number;
  tip_shortening: number; contact_ratio: number; loa_start: [number, number]; loa_end: [number, number];
  t1: [number, number]; t2: [number, number]; interference: boolean; pitch_velocity_m_s: number; tangential_load_n: number;
  kv: number; kv_all: [string, number][]; lewis_y_table1: number | null; lewis_y_table2: number | null;
  sigma1_mpa: number; sigma2_mpa: number; allowable1_mpa: number; allowable2_mpa: number; sf1: number; sf2: number; notes: string[];
}

export interface TrainOut {
  ratio: number; stage_ratios: number[]; direction: number;
  suggestions: { stages: [number, number][]; ratio: number; error_pct: number }[];
}

export interface PlanetOut {
  planet: number; planet_integer: boolean; equal_spacing: boolean; adjacency_ok: boolean; ratio: number; ratio_label: string;
  arrangements: [string, string, string, number][]; sun_r: number; planet_r: number; ring_r: number; carrier_r: number;
  suggestions: [number, number, number][]; notes: string[];
}

export interface ExportOut { svg: string; dxf: string; points: number; notes: string[] }

export interface CamSegment { kind: 'rise' | 'fall' | 'dwell'; law: 'cycloidal' | 'poly345' | 'harmonic'; lift_mm: number; beta_deg: number }

export interface CamIn { segments: CamSegment[]; base_r_mm: number; roller_r_mm: number; offset_mm: number; rpm: number; samples?: number }

export interface CamOut {
  theta_deg: number[]; s: number[]; v: number[]; a: number[]; j: number[]; v_t: number[]; a_t: number[]; j_t: number[];
  pressure_deg: number[]; rho_pitch: number[]; rho_cam: number[];
  pitch_x: number[]; pitch_y: number[]; cam_x: number[]; cam_y: number[];
  prime_r: number; d_mm: number; max_pressure_deg: number; min_rho_convex_mm: number; min_rho_concave_mm: number; undercut: boolean;
  max_v: number; max_a: number; max_j: number;
  segments: [string, string, number, number, number, number][];
  accel_continuous: boolean; boundaries_deg: number[]; notes: string[];
}

export interface HingeParams {
  material: string; wire_d_mm: number; mean_d_mm: number; active_coils: number; ends: string; free_len_mm: number;
  preload_mm: number; peened: boolean; stop_offset_mm: number | null; inner_mm: number | null; outer_mm: number | null;
  inner_corner_r_mm: number; outer_corner_r_mm: number; max_overtravel_deg: number; head_width_mm: number;
  friction: number; pin_d_mm: number; uses_per_day: number; wide_pulls_per_day: number; design_years: number;
}

export interface HingeBlock {
  name: string; per_day: number; travel_mm: number; f_min_n: number; f_max_n: number;
  tau_min_mpa: number; tau_max_mpa: number; tau_a_mpa: number; tau_m_mpa: number; nf: number;
  cycles_to_failure: number; runout: boolean; damage_per_day: number;
}

export interface HingeOut {
  params: HingeParams;
  geom: {
    stop_offset_mm: number; inner_mm: number; outer_mm: number; temple_width_mm: number; temple_height_mm: number; temple_length_mm: number;
    pin_x_mm: number; temple_inner_x_mm: number; contact_len_mm: number; pod_len_mm: number; arm_len_mm: number; temple_stiffness_n_mm: number; arm_stiffness_n_mm: number; rest_half_spread_mm: number; frame_material: string;
  };
  spring: {
    material: string; sut_mpa: number; ssy_mpa: number; ssu_mpa: number; sse_mpa: number; g_mpa: number; index_c: number; k_b: number;
    rate_n_mm: number; preload_n: number; installed_len_mm: number; solid_len_mm: number; travel_to_solid_mm: number; tau_per_n: number; od_mm: number;
  };
  phi_deg: number[]; travel_mm: number[]; force_n: number[]; torque_nmm: number[]; open_torque_nmm: number[]; close_torque_nmm: number[]; energy_nmm: number[];
  contact_start_deg: number; hump_deg: number; hump_travel_mm: number; detent_open_torque_nmm: number; detent_close_torque_nmm: number;
  overtravel_start_torque_nmm: number; overtravel_rate_nmm_deg: number; max_travel_mm: number; max_tau_mpa: number; ns_static: number;
  wear: { spread_mm: number; overtravel_deg: number; arm_bend_mm: number; clamp_n: number; clamp_rigid_n: number; hinge_share_pct: number; hits_stop: boolean };
  blocks: HingeBlock[]; nf_min: number; damage_per_day: number; life_uses: number; life_years: number; runout: boolean; design_uses: number;
  checks: Check[]; notes: string[];
}
