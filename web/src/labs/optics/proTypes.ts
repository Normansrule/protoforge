// TypeScript mirror of crates/forge-core/src/optics/{glass,seq,quality,lensdesign,pro}.rs outputs.

export interface Surface {
  label: string; radius: number; thickness: number; material: string; conic: number; aspheric: number[];
  semi_diameter: number; paraxial_f: number; fold: boolean;
}
export interface LensSystem {
  name: string; surfaces: Surface[]; pupil_diameter_mm: number; object_distance_mm: number | null;
  fields_deg: number[]; wavelengths_nm: number[]; primary: number;
}
export interface Variable { surface: number; param: string }
export interface LensPreset { id: string; title: string; summary: string; variables: Variable[]; system: LensSystem; efl: number; mass_g: number }

export interface SurfaceShape { index: number; label: string; z: number; profile: [number, number][]; semi_diameter: number; material: string; stop: boolean; fold: boolean; ideal: boolean; mirror: boolean }
export interface ElementShape { from: number; to: number; material: string; outline: [number, number][]; center_mm: number; edge_mm: number }
export interface FanRay { field: number; wave: number; py: number; pts: [number, number][]; fail: 'miss' | 'tir' | 'clipped' | null }
export interface Layout { surfaces: SurfaceShape[]; elements: ElementShape[]; rays: FanRay[]; z_image: number; image_half_height: number; z_min: number; z_max: number; y_max: number }

export interface Paraxial {
  wavelength_nm: number; efl: number; bfl: number; ffl: number; z_front_principal: number; z_rear_principal: number;
  z_front_focal: number; z_rear_focal: number; z_paraxial_image: number; chief_height_per_tan: number;
  z_exit_pupil: number | null; exit_pupil_diameter: number; marginal_nu: number; working_fnum: number; y_marginal: number[]; y_chief: number[];
}
export interface SpotField { field_deg: number; points: [number, number][][]; centroid: [number, number]; rms_mm: number; rms_per_wave_mm: number[]; geo_radius_mm: number; rms_arcmin: number; rays: number; failed: number }
export interface FanField { field_deg: number; pupil: number[]; ey: number[][]; ex: number[][]; ok: boolean[][]; opd_y: number[] }
export interface FieldCurves { field_deg: number[]; tangential: number[][]; sagittal: number[][]; distortion_pct: number[]; image_height_mm: number[] }
export interface Chromatic { longitudinal_mm: number; longitudinal_diopter: number; lateral_mm: number[]; lateral_arcmin: number[]; shift_wavelengths_nm: number[]; shift_mm: number[] }
export interface Wavefront { field_deg: number; rms_waves: number; pv_waves: number; strehl: number }
export interface MtfField { field_deg: number; tangential: number[]; sagittal: number[]; geo_tangential: number[]; geo_sagittal: number[]; at_nyquist_t: number; at_nyquist_s: number }
export interface Mtf { freq_lp_mm: number[]; diffraction: number[]; cutoff_lp_mm: number; nyquist_lp_mm: number; diffraction_at_nyquist: number; fields: MtfField[] }
export interface Quality {
  paraxial: Paraxial[]; efl: number; z_image: number; focus_shift_mm: number; image_distance_mm: number;
  spots: SpotField[]; fans: FanField[]; curves: FieldCurves | null; chromatic: Chromatic; wavefront: Wavefront[]; mtf: Mtf;
  worst_rms_mm: number; worst_rms_arcmin: number; worst_mtf_nyquist: number; worst_lateral_arcmin: number; pixel_arcmin: number;
  warnings: string[]; ideal: boolean;
}
export interface LensAnalysis {
  system: LensSystem; layout: Layout; quality: Quality; lens_efl: number; lens_mass_g: number; pixel_mm: number; nyquist_lp_mm: number;
  elements: { surfaces: [number, number]; material: string; center_mm: number; edge_mm: number }[]; project_lens: string;
}
export interface OptStep { iter: number; rms_arcmin: number; merit: number; efl: number; damping: number; values: number[]; surfaces: Surface[] }
export interface OptOut {
  history: OptStep[]; system: LensSystem; start_rms_arcmin: number; final_rms_arcmin: number; evaluations: number; converged: boolean; note: string;
  layouts: Layout[]; variables: Variable[];
}

export type Dispersion = { formula: 'sellmeier'; b: number[]; c: number[] } | { formula: 'cauchy'; a: number; b: number } | { formula: 'constant'; n: number };
export interface MaterialRow {
  id: string; name: string; kind: string; nd: number; vd: number; nf: number; nc: number; nd_catalog: number; vd_catalog: number;
  density_g_cm3: number; source: string; notes: string; curve: number[]; dispersion: Dispersion;
}
export interface Catalog { wavelengths_nm: number[]; materials: MaterialRow[]; lines_nm: [number, number, number] }
export interface AchromatOut {
  n1: number; n2: number; v1: number; v2: number; phi1: number; phi2: number; f1_mm: number; f2_mm: number;
  r1_mm: number; r2_mm: number; r3_mm: number; petzval_radius_mm: number; singlet_lca_mm: number;
}

/** Wavelength colours (F blue, d yellow-green, C red) — lab CSS variables defined for both themes. */
export const WAVE_COLORS = ['var(--op-f)', 'var(--op-d)', 'var(--op-c)'];
export const waveColor = (w: number, n: number) => (n === 3 ? WAVE_COLORS[w] : `var(--series-${(w % 8) + 1})`);
export const waveName = (nm: number) => (Math.abs(nm - 486.1327) < 0.1 ? 'F 486 nm' : Math.abs(nm - 587.5618) < 0.1 ? 'd 588 nm' : Math.abs(nm - 656.2725) < 0.1 ? 'C 656 nm' : `${nm.toFixed(1)} nm`);
