// Types for the cad.* engine methods (mirror of crates/forge-core/src/cad).
import JSZip from 'jszip';
import { call } from '../../engine/engine';
import type { Project } from '../../engine/types';
import { downloadBlob } from '../../components/ui';

export type V3 = [number, number, number];

export interface MeshF32 {
  positions: number[];
  indices: number[];
}

export interface MassProps {
  volume_mm3: number;
  volume_cm3: number;
  area_mm2: number;
  mass_g: number;
  centroid: V3;
  bbox_min: V3;
  bbox_max: V3;
  size: V3;
  triangles: number;
}

export interface MeshCheck {
  manifold: boolean;
  oriented: boolean;
  watertight: boolean;
  boundary_edges: number;
  nonmanifold_edges: number;
  misoriented_edges: number;
  degenerate_triangles: number;
  shells: number;
  signed_volume_mm3: number;
}

export interface PartOut {
  name: string;
  label: string;
  color: string;
  explode: V3;
  group: 'front' | 'left' | 'right';
  mesh: MeshF32;
  mass: MassProps;
  check: MeshCheck;
  shells: number;
}

export interface DisplayItem {
  name: string;
  kind: 'lens' | 'combiner' | 'pcb';
  mesh: MeshF32;
  color: string;
  opacity: number;
  group: 'front' | 'left' | 'right';
}

export interface FrameLayout {
  lens_center_x: number;
  frame_pd_mm: number;
  total_width_mm: number;
  front_height_mm: number;
  hinge_z: number;
  pin_x: number;
  pin_y: number;
  temple_inner_x: number;
  pod_front_y: number;
  pod_rear_y: number;
  pod_cavity_len_rim_mm: number;
  pod_cavity_len_floor_mm: number;
  pod_cavity_height_floor_mm: number;
  pod_cavity_depth_mm: number;
  pod_cavity_corner_r_mm: number;
  pod_usable_len_mm: number;
  pcb_fits_height: boolean;
  battery_cavity_len_mm: number;
  battery_cavity_height_mm: number;
  battery_cavity_depth_mm: number;
  pcb_center: V3;
  pcb_size: V3;
  usb_slot_y: [number, number];
  ear_bend_y: number;
  tilt_deg: number;
  draft_deg: number;
  min_wall_mm: number;
}

export interface PartsOut {
  orientation: 'assembly' | 'print';
  parts: PartOut[];
  display: DisplayItem[];
  layout: FrameLayout;
  total: MassProps;
  frame_mass_g: number;
  components_mass_g: number;
  wearable_mass_g: number;
  material: string;
  density_g_cm3: number;
  warnings: string[];
}

export interface FileOut {
  filename: string;
  base64: string | null;
  text: string | null;
  bytes: number;
  triangles: number;
}

export interface FitResult {
  size_label: string;
  lens_width_mm: number;
  bridge_mm: number;
  temple_length_mm: number;
  frame_pd_mm: number;
  total_width_mm: number;
  width_error_mm: number;
  decentration_per_eye_mm: number;
  prentice_prism_d: number;
  pupil_to_display_axis_mm: number;
  within_eyebox: boolean;
  notes: string[];
  patch: { lens_width_mm: number; bridge_mm: number; temple_length_mm: number };
  current_frame_pd_mm: number;
  current_decentration_mm: number;
}

export interface Cutout {
  face: 'x+' | 'x-' | 'y+' | 'y-';
  offset_mm: number;
  z_mm: number;
  width_mm: number;
  height_mm: number;
  label: string;
}

export interface EnclosureParams {
  inner_length_mm: number;
  inner_width_mm: number;
  inner_height_mm: number;
  wall_mm: number;
  floor_mm: number;
  lid_mm: number;
  corner_radius_mm: number;
  fillet_mm: number;
  draft_deg: number;
  clearance_mm: number;
  bosses: boolean;
  boss_d_mm: number;
  boss_hole_mm: number;
  screw_clear_mm: number;
  lip_mm: number;
  pcb: {
    enabled: boolean;
    length_mm: number;
    width_mm: number;
    thickness_mm: number;
    hole_inset_mm: number;
    standoff_h_mm: number;
    standoff_d_mm: number;
    standoff_hole_mm: number;
  };
  cutouts: Cutout[];
  density_g_cm3: number;
}

export interface EnclosurePart {
  name: string;
  color: string;
  explode: V3;
  assembly: MeshF32;
  mass: MassProps;
  watertight: boolean;
}

export interface EnclosureOut {
  parts: EnclosurePart[];
  outer_size_mm: V3;
  interior_mm: V3;
  pcb_center: V3;
  pcb_size: V3;
  total_mass_g: number;
  warnings: string[];
}

export interface TriOut {
  points: [number, number][];
  loop_sizes: number[];
  triangles: [number, number, number][];
  area_mm2: number;
}

export function base64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

export function fileToBlob(f: FileOut): Blob {
  if (f.base64 !== null) {
    const bytes = base64ToBytes(f.base64);
    return new Blob([bytes.buffer as ArrayBuffer], { type: 'model/stl' });
  }
  return new Blob([f.text ?? ''], { type: 'text/plain' });
}

export function downloadPartStl(project: Project, part: string, orientation: 'print' | 'assembly', binary = true) {
  const f = call<FileOut>('cad.stl', { project, part, orientation, binary });
  downloadBlob(f.filename, fileToBlob(f));
}

export async function downloadAllZip(project: Project, parts: string[], orientation: 'print' | 'assembly') {
  const zip = new JSZip();
  const lines: string[] = [];
  for (const part of parts) {
    const f = call<FileOut>('cad.stl', { project, part, orientation, binary: true });
    zip.file(f.filename, base64ToBytes(f.base64 ?? ''));
    lines.push(`${f.filename}  (${f.triangles} triangles)`);
  }
  const f = project.design.frame;
  zip.file(
    'README.txt',
    [
      `${project.meta.name} — frame parts (${orientation} orientation, millimetres)`,
      `Size ${f.lens_width_mm} □ ${f.bridge_mm} – ${f.temple_length_mm}, material ${f.material}`,
      '',
      ...lines,
      '',
      'Print orientation: every part lies flat on its largest face, min Z = 0, no supports needed.',
      'Open in Bambu Studio / OrcaSlicer / PrusaSlicer, pick your printer + filament profile and slice.',
      'Generated by Protoforge CAD Studio.',
    ].join('\n'),
  );
  const blob = await zip.generateAsync({ type: 'blob' });
  downloadBlob(`iris_frame_${orientation}_stl.zip`, blob);
}

export async function download3mf(project: Project) {
  const out = call<{ files: [string, string][]; filename: string }>('cad.3mf', { project });
  const zip = new JSZip();
  for (const [path, content] of out.files) zip.file(path, content);
  const blob = await zip.generateAsync({ type: 'blob', mimeType: 'model/3mf', compression: 'DEFLATE' });
  downloadBlob(out.filename, blob);
}

export const MATERIALS: { name: string; density: number; note: string }[] = [
  { name: 'PETG', density: 1.27, note: 'tough, slightly flexible, good for hinges' },
  { name: 'PLA', density: 1.24, note: 'easy to print, brittle, softens ~55 °C' },
  { name: 'ABS', density: 1.04, note: 'light, heat resistant, needs an enclosure' },
  { name: 'ASA', density: 1.07, note: 'UV-stable ABS alternative' },
  { name: 'PC', density: 1.2, note: 'injection-molding grade for the production frame' },
  { name: 'PA12', density: 1.01, note: 'nylon (e.g. SLS/MJF); light and tough' },
  { name: 'TPU', density: 1.21, note: 'flexible — nose pads, ear tips' },
];
