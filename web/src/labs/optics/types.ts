// TypeScript mirror of crates/forge-core/src/optics outputs.
import type { OpticsParams } from '../../engine/types';

export interface Extras { pupil_mm: number; lens_transmission: number; backgrounds: [string, number][]; world_m: number }
export interface ContrastRow { name: string; background_nits: number; see_through_nits: number; contrast_ratio: number; verdict: string }
export interface Abcd { a: number; b: number; c: number; d: number }

export interface Analysis {
  f_mm: number; d_mm: number; aperture_mm: number; path_mm: number; s_i_mm: number; image_kind: 'virtual' | 'collimated' | 'real';
  image_behind_lens_mm: number; virtual_image_m: number; magnification: number; image_width_mm: number; image_height_mm: number;
  hfov_deg: number; vfov_deg: number; dfov_deg: number; hfov_collimated_deg: number; ppd: number; ppd_center: number; ppd_v: number;
  arcmin_per_px: number; pixel_pitch_um: number; f_number: number; na: number;
  eyebox_mm: number; eyebox_v_mm: number; eyebox_approx_mm: number; eyebox_full_pupil_mm: number; eyebox_partial_mm: number; pupil_mm: number; k: number;
  combiner_min_mm: number; min_lens_to_combiner_mm: number; abcd: Abcd; accommodation_d: number; focus_mismatch_d: number; focus_sensitivity_d_per_mm: number;
  see_through_pct: number; image_nits: number; lens_transmission: number; contrast: ContrastRow[]; warnings: string[];
}
export type AnalyzeOut = Analysis & { optics: OpticsParams };

export interface Ray { field: number; y0: number; yl: number; points: [number, number][]; through_pupil: boolean; chief: boolean; back: [number, number][]; unfolded: [number, number][] }
export interface FieldInfo { y0: number; angle_deg: number; pupil_fill: number; virtual_point: [number, number]; virtual_unfolded: [number, number]; retina_x: number | null }
export interface Geometry {
  display: [[number, number], [number, number]]; lens_x: number; lens_half: number; combiner: [[number, number], [number, number]];
  eye_x: number; eye_y: number; pupil_half: number; eyeball_r: number; retina_y: number; lc: number; eye_relief: number;
}
export interface EyeboxSweep { offset_mm: number[]; visible_fov_deg: number[]; visible_frac: number[]; field_fill: number[][] }
export interface TraceOut { analysis: Analysis; geometry: Geometry; rays: Ray[]; fields: FieldInfo[]; sweep: EyeboxSweep; visible_fov_deg: number; eye_offset_mm: number }

export interface DesignSweep {
  f_mm: number[]; hfov_deg: number[]; ppd: number[]; eyebox_fixed_d_mm: number[]; eyebox_fixed_n_mm: number[]; px_needed: number[];
  min_hfov_deg: number; min_ppd: number; min_eyebox_mm: number; band_mm: [number, number] | null; band_eyebox_mm: [number, number] | null;
  current_f_mm: number; image_m: number; note: string;
}

export interface LegibilityRow { em_px: number; cap_px: number; cap_arcmin: number; snellen_20_over: number; chars_per_line: number; lines: number; verdict: string }
export interface Legibility { arcmin_per_px: number; ppd_center: number; rows: LegibilityRow[]; min_em_16: number; min_em_20: number; px_w: number; px_h: number; hfov_deg: number; vfov_deg: number }

export interface ContrastSweep { background_nits: number[]; contrast_ratio: number[]; nits_for_3: number[]; image_nits: number; see_through: number; bg_at_3: number; bg_at_1_5: number }
