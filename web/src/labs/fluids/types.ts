// Engine types for the Thermal & Fluids Lab (crates/forge-core/src/fluids).
export type V3 = [number, number, number];

export interface PodConfig {
  voxel_mm?: number;
  scenario?: string;
  display_duty?: number;
  spreader?: string;
  spreader_um?: number;
  spreader_k_in?: number;
  spreader_location?: string;
  display_heat_in?: string;
  shell_material?: string;
  emissivity?: number;
  air_speed_m_s?: number;
  h_override?: number;
  ambient_c?: number;
  precond?: string;
  tol?: number;
  max_iter?: number;
  cu_coverage?: number;
}

export interface Resolved extends Required<Omit<PodConfig, 'display_duty' | 'spreader_k_in' | 'h_override'>> {
  display_duty: number | null;
  spreader_k_in: number | null;
  h_override: number | null;
}

export interface FluidsDesign {
  spreader: string;
  spreader_um: number;
  spreader_location: string;
  display_heat_in: string;
  emissivity?: number | null;
  air_speed_m_s: number;
}

export interface Source {
  refdes: string;
  name: string;
  location: string;
  mw: number;
  pos: V3;
  placed: string;
}

export interface HGroup {
  body: string;
  class: 'vertical' | 'up' | 'down';
  area_mm2: number;
  theta_mean: number;
  h_conv: number;
  h_rad: number;
  h_total: number;
  ra: number;
  length_mm: number;
  valid: boolean;
  q_mw: number;
}

export interface PartStat {
  part: string;
  peak_c: number;
  mean_c: number;
  area_mm2: number;
}

export interface Surface {
  name: string;
  label: string;
  positions: string;
  indices: string;
  temps: string;
  vertices: number;
  triangles: number;
}

export interface Volume {
  nx: number;
  ny: number;
  nz: number;
  origin: V3;
  h_mm: number;
  data: string;
  t_lo: number;
  t_hi: number;
}

export interface Slices {
  nx: number;
  ny: number;
  nz: number;
  h_mm: number;
  origin: V3;
  temp_u8: string;
  mat_u8: string;
  t_lo: number;
  t_hi: number;
}

export interface ThermalMaterial {
  id: string;
  name: string;
  k_in: number;
  k_through: number;
  rho: number;
  cp: number;
  emissivity: number;
  note: string;
}

export interface Lumped {
  shell_c: number;
  board_c: number;
  r_pod_ambient: number;
  r_board_pod: number;
  r_eff_mean: number;
  r_eff_peak: number;
  area_mm2: number;
  h_mean: number;
  h_equiv_lumped: number;
  spread_len_mm: number;
  spread_len_spreader_mm: number;
  explanation: string[];
}

export interface Aabb {
  lo: V3;
  hi: V3;
}

export interface SteadyOut {
  config: Resolved;
  ambient_c: number;
  skin_limit_c: number;
  total_mw: number;
  pod_heat_mw: number;
  peak_skin_c: number;
  peak_part: string;
  peak_pos: V3;
  mean_skin_c: number;
  hotspot_over_mean_c: number;
  board_max_c: number;
  board_max_pos: V3;
  display_max_c: number;
  air_max_c: number;
  parts: PartStat[];
  groups: HGroup[];
  heat_out_mw: number;
  sources: Source[];
  elements: number;
  dofs: number;
  nnz: number;
  voxel_mm: number;
  cavity_cells: number;
  cavity_expected_cells: number;
  iterations: number;
  iterations_total: number;
  converged: boolean;
  residuals: number[];
  precond: string;
  materials: ThermalMaterial[];
  k_cavity: number;
  k_cavity_rad: number;
  cavity_rayleigh: number;
  lumped: Lumped;
  surfaces: Surface[];
  inner: Surface[];
  voxel_surface: Surface | null;
  volume: Volume | null;
  slices: Slices | null;
  bbox_min: V3;
  bbox_max: V3;
  pcb_box: Aabb;
  display_box: Aabb;
  spreader_box: Aabb | null;
  warnings: string[];
}

export interface Segment {
  duration_s: number;
  display_duty: number;
  label: string;
}

export interface TransientOut {
  t_s: number[];
  peak_skin_c: number[];
  mean_skin_c: number[];
  board_max_c: number[];
  display_max_c: number[];
  power_mw: number[];
  duty: number[];
  lumped_shell_c: number[];
  lumped_board_c: number[];
  segments: Segment[];
  frame_t_s: number[];
  frames: string[];
  volume_dims: [number, number, number];
  volume_origin: V3;
  volume_h_mm: number;
  t_lo: number;
  t_hi: number;
  surfaces: Surface[];
  inner: Surface[];
  steady_peak_c: number;
  time_to_limit_s: number | null;
  skin_limit_c: number;
  ambient_c: number;
  iterations_per_step: number;
  dt_s: number;
  bbox_min: V3;
  bbox_max: V3;
  note: string;
}

export interface TradeRow {
  id: string;
  label: string;
  spreader: string;
  thickness_um: number;
  location: string;
  k_in: number;
  peak_skin_c: number;
  mean_skin_c: number;
  hotspot_over_mean_c: number;
  board_max_c: number;
  gain_c: number;
  added_mass_g: number;
  sheet_conductance_mw_k: number;
  note: string;
}

export interface TradeOut {
  rows: TradeRow[];
  sweeps: { spreader: string; location: string; thickness_um: number[]; peak_skin_c: number[] }[];
  baseline_peak_c: number;
  skin_limit_c: number;
  ambient_c: number;
  voxel_mm: number;
  sheet_area_mm2: number;
  hotspot_location: string;
  notes: string[];
}

export interface StudyRow {
  voxel_mm: number;
  elements: number;
  dofs: number;
  iterations: number;
  peak_skin_c: number;
  mean_skin_c: number;
  hotspot_over_mean_c: number;
  board_max_c: number;
  peak_part: string;
}

export interface Air {
  t_k: number;
  rho: number;
  cp: number;
  nu: number;
  k: number;
  alpha: number;
  pr: number;
  beta: number;
  clamped: boolean;
}

export interface CorrRow {
  id: string;
  name: string;
  kind: 'natural' | 'forced';
  length_mm: number;
  group: number;
  group_name: string;
  nu: number;
  h: number;
  valid: boolean;
  range: string;
  equation: string;
}

export interface ConvOut {
  t_surface_c: number;
  t_ambient_c: number;
  t_film_c: number;
  film: Air;
  emissivity: number;
  speed_m_s: number;
  rows: CorrRow[];
  h_rad: number;
  h_still_total: number;
  h_walking_total: number;
  dt: number[];
  curves: { id: string; name: string; h: number[] }[];
  h_rad_curve: number[];
  q_still_w_m2: number;
  q_walking_w_m2: number;
  notes: string[];
}

export interface ScenariosOut {
  steady: { id: string; name: string; description: string }[];
  transient: [string, string, Segment[]][];
}

// ---------------------------------------------------------------- LBM

export interface LbmConfig {
  nx: number;
  ny: number;
  obstacle: string;
  size_cells: number;
  x_frac: number;
  y_frac: number;
  angle_deg: number;
  naca: string;
  aspect: number;
  re: number;
  u_in: number;
  walls: string;
  inlet: string;
  periodic_x: boolean;
  body_force: number;
  tracers: number;
  seed: number;
  mask?: string | null;
  perturb: number;
}

export interface LbmPreset {
  id: string;
  label: string;
  description: string;
  config: LbmConfig;
  reference: string | null;
}

export interface LbmInfo {
  nx: number;
  ny: number;
  tau: number;
  nu: number;
  re: number;
  re_requested: number;
  u_ref: number;
  d_cells: number;
  mach: number;
  blockage: number;
  probe: [number, number];
  solid_cells: number;
  re_walking: number | null;
  real_size_mm: number | null;
  warnings: string[];
}

export interface LbmInitOut {
  session: number;
  info: LbmInfo;
  mask: string;
}

export interface SpectrumOut {
  st: number | null;
  freq_per_step: number | null;
  st_axis: number[];
  power: number[];
  samples: number;
  periods: number;
  note: string;
}

export interface LbmStepOut {
  step: number;
  frame: string | null;
  lo: number;
  hi: number;
  cd: number;
  cl: number;
  cd_mean: number;
  cl_mean: number;
  cl_amp: number;
  mass: number;
  max_speed: number;
  unstable: boolean;
  tracers: number[];
  history: { step: number[]; cd: number[]; cl: number[]; probe: number[] } | null;
  spectrum: SpectrumOut | null;
}
