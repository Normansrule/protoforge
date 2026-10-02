// Types for the ergonomics.* engine methods (mirror of crates/forge-core/src/ergonomics).
import type { MeshF32 } from '../cad/api';

export type Group = 'men' | 'women' | 'combined';
export type V3 = [number, number, number];

export interface DimOverride { men?: [number, number] | null; women?: [number, number] | null }

export interface ErgoConfig {
  female_fraction: number;
  temple_splay_deg: number;
  display_offset_mm: number;
  tint_transmission: number;
  caption_cap_px: number;
  acuity_20_over: number;
  pd_tolerance_mm: number;
  ear_window_mm: [number, number];
  nose_check: boolean;
  nose_window_mm: [number, number];
  friction_mu: number;
  look_down_deg: number;
  nose_flank_deg: number;
  pad_area_mm2: number;
  temple_contact_length_mm: number;
  pressure_limit_kpa: number;
  contact_before_bend_mm: number;
  bright_scene_nits: number;
  asymmetry_sd_mm: number;
  samples: number;
  seed: number;
  overrides: Record<string, DimOverride>;
}

export interface Normal { mean: number; sd: number }

export interface Dim {
  id: 'head_breadth' | 'ipd' | 'ear_distance' | 'nose_bridge';
  label: string;
  symbol: string;
  unit: string;
  men: Normal;
  women: Normal;
  measured: boolean;
  source: string;
  definition: string;
  ansur: [number[], number[]] | null;
  overridden: boolean;
}

export interface GroupStats { group: Group; mean: number; sd: number; percentiles: number[] }
export interface DimTable { dim: Dim; groups: GroupStats[]; x: number[]; pdf: number[][] }
export interface PopulationOut {
  percentiles: number[];
  ansur_percentiles: number[];
  female_fraction: number;
  tables: DimTable[];
  correlations: number[][][];
}

export interface FrameSize { lens_width_mm: number; bridge_mm: number; temple_length_mm: number; splay_deg?: number | null }

export interface FrameGeom {
  lens_width_mm: number; bridge_mm: number; temple_length_mm: number; rim_mm: number; front_thickness_mm: number;
  lens_height_mm: number; lens_center_x: number; frame_pd_mm: number; total_width_mm: number; temple_inner_x: number;
  pin_y: number; hinge_z: number; pod_front_y: number; pod_rear_y: number; battery_pod_rear_y: number; ear_bend_y: number;
  bend_from_pin_mm: number; temple_w_mm: number; temple_h_mm: number; pod_w_mm: number; pod_h_mm: number; wall_mm: number;
  pad: V3; pad_semi_mm: [number, number]; pad_area_cad_mm2: number; material: string; e_mpa: number;
}

export interface FrameSummary { size: FrameSize; label: string; geom: FrameGeom; s0_mm: number; contact_from_pin_mm: number }

export interface Band { min_n: number; max_n: number }

export interface FitCheck {
  pd_ok: boolean; ear_ok: boolean; width_ok: boolean; nose_ok: boolean; clamp_n: number;
  decentration_mm: number; bend_minus_ear_mm: number; fits: boolean;
}

export interface Person { female: boolean; head_breadth: number; ipd: number; ear: number; nose: number; asym: number }

export interface Coverage {
  n: number; fit_pct: number; pd_pct: number; ear_pct: number; width_pct: number; nose_pct: number;
  too_tight_pct: number; too_loose_pct: number;
}

export interface FitOut {
  frame: FrameSummary;
  band: Band;
  n: number;
  combined: Coverage;
  men: Coverage;
  women: Coverage;
  people: (Person & FitCheck)[];
  clamp_p: [number, number, number];
  band_heads_mm: [number, number];
  pd_window_mm: [number, number];
  ear_window_abs_mm: [number, number];
  config: ErgoConfig;
}

export interface SizeOut {
  name: string; size: FrameSize; label: string; frame_pd_mm: number; total_width_mm: number;
  alone_pct: number; served_pct: number; head_breadth_range_mm: [number, number];
}
export interface SizesOut { sizes: SizeOut[]; union_pct: number; iris_pct: number; candidates: number; n: number; notes: string[] }

export interface HeadMesh {
  head: MeshF32; nose: MeshF32; ears: MeshF32;
  eyes: [V3, V3]; eye_radius: number; pupils: [V3, V3]; ear_tops: [V3, V3]; contacts: [V3, V3];
}
export interface HeadOut {
  mesh: HeadMesh; person: Person; check: FitCheck; band: Band; temple_angle_deg: [number, number]; pin: V3;
  deflection_mm: [number, number]; pupil_offset_mm: number; eyebox_mm: number; in_eyebox: boolean; frame: FrameSummary;
}

export interface MassItem { name: string; kind: 'frame' | 'component'; location: string; mass_g: number; pos: V3 }
export interface Loads {
  pitch_deg: number;
  reactions_gf: [number, number, number, number];
  nose_gf: number; ears_gf: number; nose_share_pct: number; right_ear_share_pct: number;
  pad_normal_n: [number, number]; pad_pressure_kpa: [number, number];
  slide_force_n: number; mu_required_no_clamp: number;
}
export interface BalanceOut {
  items: MassItem[]; mass_g: number; com: V3; pads: [V3, V3]; ears: [V3, V3];
  pad_area_mm2: number; pad_area_cad_mm2: number;
  upright: Loads; pitched: Loads; look_down: Loads; band: Band;
  pad_area_for_limit_mm2: number; pressure_limit_kpa: number;
  shift_mm: number[]; nose_gf_vs_shift: number[]; pad_area_axis: number[]; pressure_vs_area: number[];
  pitch_axis: number[]; mu_required_vs_pitch: number[]; mu_required_with_clamp: number[]; clamp_p50_n: number;
  right_g: number; left_g: number; frame: FrameSummary; notes: string[];
}

export interface Segment { name: string; s0: number; s1: number; i_mm4: number; ei: number }
export interface ClampModel {
  e_mpa: number; l_c: number; s0: number; splay_deg: number;
  right: Segment[]; left: Segment[]; front: Segment[]; front_flex: number;
  c_right: number; c_left: number; c_front: number; c_total: number;
}
export interface ClampRow { group: Group; percentile: number; head_breadth_mm: number; force_n: number; verdict: string }
export interface ClampOut {
  model: ClampModel; band: Band; frame: FrameSummary; percentiles: number[];
  force_vs_pct: number[][]; hb_vs_pct: number[][]; head_axis: number[]; force_vs_head: number[];
  rows: ClampRow[]; force_p95_n: number; head_p95_mm: number; band_coverage_pct: number;
  best_splay_deg: number; best_splay_coverage_pct: number; band_heads_mm: [number, number];
  root_stress_p95_mpa: number; deflection_p95_mm: [number, number]; yaw_p95_deg: number; material: string; notes: string[];
}

export interface SceneRow {
  name: string; description: string; background_nits: number; see_through_nits: number; image_nits: number;
  contrast_ratio: number; verdict: string; nits_for_3to1: number;
}
export interface Photometry { image_nits: number; see_through: number; lens_transmission: number; combiner_reflectance: number; tint: number }
export interface EyeboxOut {
  eyebox_mm: number; eyebox_full_pupil_mm: number; display_x_mm: number; ipd_window_mm: [number, number];
  groups: { group: Group; coverage_pct: number }[];
  best_offset_mm: number; best_coverage_pct: number; offsets: number[]; coverage_vs_offset: number[];
  eyebox_for_95_mm: number; ipd_x: number[]; ipd_pdf: number[]; ipd_stats: [[number, number], [number, number]];
}
export interface LegRow { cap_px: number; arcmin: number; snellen_20_over: number; verdict: string }
export interface DisplayOut {
  arcmin_per_px: number; ppd: number; ppd_center: number; hfov_deg: number; vfov_deg: number; px_w: number; px_h: number;
  acuity_arcmin: number; caption_px: number; caption_arcmin: number; caption_verdict: string; min_px_16: number; min_px_20: number;
  rows: LegRow[]; photometry: Photometry; scenes: SceneRow[]; bright: SceneRow; eyebox: EyeboxOut; bg_axis: number[]; acr_vs_bg: number[];
}
