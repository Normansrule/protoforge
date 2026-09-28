// Types for the print.* engine methods (mirror of crates/forge-core/src/print).
import type { MeshF32 } from '../cad/api';
import type { PrintParams } from '../../engine/types';

export interface PathOut {
  /** move kind code, see KINDS */
  k: number;
  /** travel with retraction */
  r?: boolean;
  /** flat [x0, y0, x1, y1, …] in 1/100 mm */
  p: number[];
}

export interface LayerOut {
  i: number;
  z: number;
  h: number;
  paths: PathOut[];
}

export interface LayerStats {
  index: number;
  z: number;
  height: number;
  time_s: number;
  raw_time_s: number;
  slowed: boolean;
  length_mm: number[];
  time_by_kind_s: number[];
  e_mm: number;
  volume_mm3: number;
  retracts: number;
  area_mm2: number;
}

export interface SliceStats {
  layers: number;
  time_s: number;
  time_h: number;
  time_text: string;
  cooling_s: number;
  slowed_layers: number;
  time_by_kind_s: number[];
  length_by_kind_mm: number[];
  filament_mm: number;
  filament_m: number;
  filament_cm3: number;
  filament_g: number;
  cost_usd: number;
  part_volume_mm3: number;
  fill_ratio: number;
  retracts: number;
  density_g_cm3: number;
  max_z_mm: number;
}

export interface SliceSettings {
  first_layer_mm: number;
  top_layers: number;
  bottom_layers: number;
  infill_overlap_pct: number;
  retract_mm: number;
  retract_speed_mm_s: number;
  retract_min_travel_mm: number;
  fan_pct: number;
  bed_center: [number, number];
  bed_size: [number, number];
  cost_per_kg: number;
  accel_mm_s2: number;
  corner_speed_mm_s: number;
  min_layer_time_s: number;
  first_layer_speed_pct: number;
  outer_wall_speed_pct: number;
  infill_speed_pct: number;
  relative_e: boolean;
  skirt: boolean;
  density_g_cm3: number | null;
  gcode: boolean;
}

export interface SliceOut {
  part: string;
  layers: LayerOut[];
  per_layer: LayerStats[];
  stats: SliceStats;
  gcode: string;
  gcode_lines: number;
  mesh: MeshF32;
  bbox: [[number, number, number], [number, number, number]];
  params: PrintParams;
  settings: SliceSettings;
  kinds: [number, string][];
  ms: number;
}

export interface OverhangReport {
  positions: number[];
  colors: number[];
  threshold_deg: number;
  overhang_area_mm2: number;
  overhang_faces: number;
  downward_area_mm2: number;
  bed_area_mm2: number;
  max_angle_deg: number;
  histogram_mm2: number[];
  needs_support: boolean;
  suggestion: string;
}

export interface PartEstimate {
  name: string;
  stats: SliceStats;
  volume_mm3: number;
}

export interface EstimateOut {
  parts: PartEstimate[];
  total_time_h: number;
  total_time_text: string;
  total_filament_g: number;
  total_filament_m: number;
  total_cost_usd: number;
  method: string;
}

/** Move kinds in engine order with their theme colour token. */
export const KINDS: { code: number; label: string; color: string; short: string }[] = [
  { code: 0, label: 'Travel', color: '--muted', short: 'travel' },
  { code: 1, label: 'External perimeter', color: '--series-2', short: 'outer wall' },
  { code: 2, label: 'Inner perimeter', color: '--series-1', short: 'inner wall' },
  { code: 3, label: 'Sparse infill', color: '--series-3', short: 'infill' },
  { code: 4, label: 'Solid infill (top/bottom)', color: '--series-4', short: 'solid' },
  { code: 5, label: 'Skirt', color: '--series-7', short: 'skirt' },
];

export function defaultSettings(): SliceSettings {
  return {
    first_layer_mm: 0.2,
    top_layers: 4,
    bottom_layers: 4,
    infill_overlap_pct: 15,
    retract_mm: 0.8,
    retract_speed_mm_s: 35,
    retract_min_travel_mm: 1.5,
    fan_pct: 40,
    bed_center: [128, 128],
    bed_size: [256, 256],
    cost_per_kg: 20,
    accel_mm_s2: 2000,
    corner_speed_mm_s: 5,
    min_layer_time_s: 6,
    first_layer_speed_pct: 50,
    outer_wall_speed_pct: 60,
    infill_speed_pct: 100,
    relative_e: false,
    skirt: true,
    density_g_cm3: null,
    gcode: true,
  };
}

export function fmtTime(s: number): string {
  const m = Math.round(s / 60);
  if (m >= 60) return `${Math.floor(m / 60)} h ${String(m % 60).padStart(2, '0')} m`;
  if (s >= 60) return `${m} min`;
  return `${s.toFixed(0)} s`;
}

export function layerCsv(per: LayerStats[]): string {
  const head = 'layer,z_mm,height_mm,time_s,raw_time_s,cooling_slowdown,outer_wall_mm,inner_wall_mm,sparse_infill_mm,solid_infill_mm,skirt_mm,travel_mm,filament_mm,extruded_mm3,retracts,area_mm2';
  const rows = per.map((l) =>
    [
      l.index, l.z.toFixed(3), l.height.toFixed(3), l.time_s.toFixed(2), l.raw_time_s.toFixed(2), l.slowed ? 1 : 0,
      l.length_mm[1].toFixed(1), l.length_mm[2].toFixed(1), l.length_mm[3].toFixed(1), l.length_mm[4].toFixed(1),
      l.length_mm[5].toFixed(1), l.length_mm[0].toFixed(1), l.e_mm.toFixed(3), l.volume_mm3.toFixed(2), l.retracts, l.area_mm2.toFixed(2),
    ].join(','),
  );
  return [head, ...rows].join('\n') + '\n';
}
