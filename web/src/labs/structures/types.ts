// Types for the structures.* engine methods (mirror of crates/forge-core/src/structures).
export type V3 = [number, number, number];

export interface StructMaterial {
  id: string;
  name: string;
  process: string;
  printed: boolean;
  e_gpa: number;
  nu: number;
  density_g_cm3: number;
  yield_mpa: number;
  uts_mpa: number;
  elongation_pct: number;
  fatigue_mpa: number;
  fatigue_cycles: number;
  max_service_c: number;
  cte_ppm_k: number;
  z_strength_factor: number;
  snap_strain_pct: number;
  notes: string;
}

export interface CaseInfo {
  id: string;
  label: string;
  parts: string[];
  default_force_n: number;
  description: string;
}

export interface SurfaceField {
  positions: number[];
  indices: number[];
  disp: number[];
  vm: number[];
  util: number[];
}

export interface FeaResult {
  part: string;
  case: string;
  material: string;
  printed: boolean;
  h_mm: number;
  dims: [number, number, number];
  elements: number;
  nodes: number;
  free_dofs: number;
  removed_voxels: number;
  levels: number;
  precond: string;
  element: string;
  iterations: number;
  converged: boolean;
  history: number[];
  force_n: number;
  max_vm_mpa: number;
  max_vm_pos: V3;
  peak_vm_mpa: number;
  peak_vm_pos: V3;
  max_disp_mm: number;
  load_disp_mm: number;
  stiffness_n_mm: number;
  yield_mpa: number;
  z_factor: number;
  sf_von_mises: number;
  safety_factor: number;
  governing: string;
  crit_pos: V3;
  max_across_layers_mpa: number;
  hint: { region: string; mechanism: string; advice: string };
  build_dir: V3;
  fixed: V3[];
  loads: { pos: V3; dir: V3 }[];
  bbox_min: V3;
  bbox_max: V3;
  voxels: SurfaceField | null;
  cad: SurfaceField | null;
  notes: string[];
}

export interface CaseSummary {
  case: string;
  part: string;
  label: string;
  force_n: number;
  h_mm: number;
  elements: number;
  iterations: number;
  max_vm_mpa: number;
  peak_vm_mpa: number;
  safety_factor: number;
  sf_von_mises: number;
  governing: string;
  deflection_mm: number;
  stiffness_n_mm: number;
  region: string;
  counts_for_min: boolean;
}

export interface DropResult {
  height_m: number;
  impact_speed_m_s: number;
  energy_j: number;
  moving_mass_g: number;
  stiffness_n_mm: number;
  peak_force_n: number;
  deflection_mm: number;
  contact_time_ms: number;
  peak_accel_g: number;
  stress_per_n_mpa: number;
  peak_stress_mpa: number;
  safety_factor: number;
  governing: string;
  assumptions: string[];
}

export interface Dim {
  name: string;
  nominal: number;
  plus: number;
  minus: number;
  dir: number;
  dist: 'normal' | 'uniform';
  sigmas: number;
  note?: string;
}

export interface StackOut {
  nominal: number;
  mean: number;
  worst_min: number;
  worst_max: number;
  rss_sigma: number;
  rss_min: number;
  rss_max: number;
  mc_mean: number;
  mc_sigma: number;
  mc_min: number;
  mc_max: number;
  yield_pct: number;
  rss_yield_pct: number;
  cpk: number;
  hist_edges: number[];
  hist_counts: number[];
  samples_head: number[];
  contributions: { name: string; variance_pct: number; range_pct: number }[];
  lower: number | null;
  upper: number | null;
}

export interface PodStackOut {
  dims: Dim[];
  stack: StackOut;
  pod_inner_mm: number;
  pcb_len_mm: number;
  thermal: { pod_cte_ppm_k: number; pcb_cte_ppm_k: number; temps_c: number[]; delta_mm: number[]; delta_cold_mm: number; delta_hot_mm: number };
  process: string;
  notes: string[];
}

export interface ModalOut {
  part: string;
  stations_mm: number[];
  lateral_hz: number[];
  vertical_hz: number[];
  lateral_shapes: number[][];
  vertical_shapes: number[][];
  ei_lateral: number[];
  ei_vertical: number[];
  mass_g_mm: number[];
  total_mass_g: number;
  added_mass_g: number;
  first_mode_hz: number;
  notes: string[];
}

export interface FatigueBlock {
  name: string;
  amplitude_mpa: number;
  mean_mpa: number;
  equivalent_mpa: number;
  cycles: number;
  cycles_to_failure: number;
  damage: number;
}

export interface FatigueOut {
  material: string;
  has_data: boolean;
  uts_mpa: number;
  sf_prime_mpa: number;
  basquin_b: number;
  fatigue_point: [number, number];
  blocks: FatigueBlock[];
  damage: number;
  life_years: number;
  stress_margin: number;
  sn_n: number[];
  sn_s: number[];
  goodman_se_mpa: number;
  design_cycles: number;
  notes: string[];
}

export interface IrisOut {
  fidelity: string;
  material: string;
  material_name: string;
  printed: boolean;
  yield_mpa: number;
  cases: CaseSummary[];
  min_safety_factor: number;
  worst_case: string;
  temple_deflection_mm: number;
  drop: DropResult;
  pod: PodStackOut;
  focus: FocusStackOut;
  modal: ModalOut;
  fatigue: FatigueOut;
  hinge_cycles: number;
  solid_fraction_note: string;
  notes: string[];
}

export interface TopoFrame {
  iter: number;
  rho_b64: string;
  compliance: number;
}

export interface TopoSupport {
  i0: number;
  j0: number;
  i1: number;
  j1: number;
  fix_x: boolean;
  fix_y: boolean;
}

export interface TopoOut {
  preset: string;
  nelx: number;
  nely: number;
  volfrac: number;
  penal: number;
  rmin: number;
  filter: string;
  elem_mm: number;
  iterations: number;
  converged: boolean;
  compliance: number[];
  volume: number[];
  change: number[];
  frames: TopoFrame[];
  final_rho: number[];
  final_vm: number[];
  outline: [number, number][][];
  svg: string;
  openscad: string;
  supports: TopoSupport[];
  loads: { i: number; j: number; fx: number; fy: number }[];
  passive: number[];
}

export interface TopoPreset {
  id: string;
  label: string;
  description: string;
  nelx: number;
  nely: number;
  volfrac: number;
  rmin: number;
  elem_mm: number;
}

export interface StudyRow {
  h_mm: number;
  elements: number;
  iterations: number;
  max_vm_mpa: number;
  peak_vm_mpa: number;
  safety_factor: number;
  deflection_mm: number;
  volume_mm3: number;
}

export interface LockOut {
  layers: number;
  element: string;
  fea_tip_mm: number;
  beam_tip_mm: number;
  timoshenko_tip_mm: number;
  error_pct: number;
  iterations: number;
  history: number[];
  x: number[];
  w: number[];
}

export interface FitOut {
  nominal_mm: number;
  hole: string;
  shaft: string;
  hole_es_um: number;
  hole_ei_um: number;
  shaft_es_um: number;
  shaft_ei_um: number;
  hole_max_mm: number;
  hole_min_mm: number;
  shaft_max_mm: number;
  shaft_min_mm: number;
  max_clearance_um: number;
  min_clearance_um: number;
  fit_type: string;
  description: string;
}

export interface FocusStackOut {
  dims: Dim[];
  stack: StackOut;
  nominal_mm: number;
  focal_mm: number;
  nominal_vergence_d: number;
  image_distance_m: number;
  sensitivity_d_per_mm: number;
  allowable_d: number;
  lower_mm: number;
  upper_mm: number;
  worst_diopters: number;
  rss_diopters: number;
  curve_mm: number[];
  curve_d: number[];
  notes: string[];
}

export interface Section {
  shape: 'rect' | 'round' | 'tube';
  b: number;
  h: number;
  t: number;
}

export interface SectionProps {
  area_mm2: number;
  i_mm4: number;
  c_mm: number;
  z_mm3: number;
  r_gyr_mm: number;
}

export interface BeamOut {
  section: SectionProps;
  e_mpa: number;
  max_moment_nmm: number;
  max_moment_x_mm: number;
  max_stress_mpa: number;
  max_deflection_mm: number;
  stiffness_n_mm: number;
  yield_mpa: number;
  safety_factor: number;
  x: number[];
  deflection: number[];
  moment: number[];
  formula: string;
}

export interface SnapOut {
  strain_pct: number;
  allowable_strain_pct: number;
  utilisation: number;
  max_undercut_mm: number;
  deflection_force_n: number;
  mating_force_n: number;
  separation_force_n: number;
  self_locking: boolean;
  e_mpa: number;
  root_stress_mpa: number;
  shape_x: number[];
  shape_y: number[];
  verdict: string;
}

export interface BoltOut {
  d_mm: number;
  pitch_mm: number;
  stress_area_mm2: number;
  proof_mpa: number;
  proof_load_n: number;
  preload_n: number;
  torque_nm: number;
  bolt_stiffness_n_mm: number;
  member_stiffness_n_mm: number;
  joint_constant: number;
  bolt_load_n: number;
  member_load_n: number;
  separation_load_n: number;
  load_factor: number;
  separation_factor: number;
  bolt_stress_mpa: number;
  strip_area_internal_mm2: number;
  strip_area_external_mm2: number;
  strip_force_internal_n: number;
  strip_force_external_n: number;
  tensile_capacity_n: number;
  first_failure: string;
  min_engagement_mm: number;
  nut_material: string;
  notes: string[];
}

export interface BossOut {
  pilot_hole_mm: number;
  boss_od_mm: number;
  boss_od_range_mm: [number, number];
  min_engagement_mm: number;
  max_boss_wall_at_base_mm: number;
  base_fillet_mm: number;
  pullout_force_n: number;
  shear_strength_mpa: number;
  safety_factor: number;
  notes: string[];
  inserts: { thread: string; hole_mm: number; insert_od_mm: number; length_mm: number; min_boss_od_mm: number }[];
}

export interface PressOut {
  pressure_mpa: number;
  hub_hoop_mpa: number;
  hub_von_mises_mpa: number;
  shaft_hoop_mpa: number;
  hub_yield_mpa: number;
  hub_safety_factor: number;
  axial_force_n: number;
  torque_nm: number;
  hub_radial_growth_um: number;
  r: number[];
  hoop: number[];
  radial: number[];
}

export interface BuckleOut {
  k_factor: number;
  slenderness: number;
  transition_slenderness: number;
  euler_load_n: number;
  johnson_load_n: number;
  critical_load_n: number;
  regime: string;
  safety_factor: number;
  plate_k: number;
  plate_stress_mpa: number;
  plate_load_n: number;
  applied_stress_mpa: number;
  curve_l: number[];
  curve_p: number[];
}

export interface HingeOut {
  strain_pct: number;
  bend_radius_mm: number;
  guidance_limit_pct: number;
  verdict: string;
  min_length_mm: number;
}

export interface ThermalOut {
  alpha_outer_ppm: number;
  alpha_inner_ppm: number;
  dclear_um_per_k: number;
  clearance_cold_mm: number;
  clearance_hot_mm: number;
  closing_temp_c: number | null;
  constrained_stress_mpa: number;
  outer_yield_mpa: number;
  bimetal_curvature_per_k: number;
  bimetal_bow_cold_mm: number;
  bimetal_bow_hot_mm: number;
  temps_c: number[];
  clearance_mm: number[];
  notes: string[];
}

export interface KtOut {
  kt: number;
  kf: number;
  nominal_mpa: number;
  peak_mpa: number;
  yield_mpa: number;
  sf_nominal: number;
  sf_peak: number;
  basis: string;
  curve_x: number[];
  curve_kt: number[];
  curve_label: string;
  notes: string[];
}

export interface BeamModesOut {
  freqs_hz: number[];
  analytic_hz: number[];
  stations_mm: number[];
  shapes: number[][];
  ei_nmm2: number;
  mass_g: number;
}
