// TypeScript mirror of molding.* outputs (crates/forge-core/src/molding).
export type V3 = [number, number, number];

export interface ColoredMesh { positions: number[]; indices: number[]; colors: number[] }
export interface Candidate { label: string; dir: V3; min_draft_deg: number; undercut_area_mm2: number }
export interface DraftResult {
  part: string; pull: V3; pull_label: string; min_draft_required_deg: number; min_draft_deg: number; min_face_center: V3;
  faces: number; side_faces: number; area_floor_mm2: number; area_ok_mm2: number; area_low_mm2: number; area_zero_mm2: number;
  area_undercut_mm2: number; undercut_faces: number; hist_edges: number[]; hist_area: number[]; candidates: Candidate[];
  mesh: ColoredMesh | null;
}
export interface ThicknessResult {
  part: string; min_mm: number; max_mm: number; nominal_mm: number; p95_mm: number; hist_edges: number[]; hist_area: number[];
  thick_area_mm2: number; thin_area_mm2: number; thick_center: V3 | null; sampled_faces: number; mesh: ColoredMesh | null;
}
export interface Material {
  id: string; name: string; t_melt_c: number; t_mold_c: number; t_eject_c: number; diffusivity_mm2_s: number; density_g_cm3: number;
  shrinkage_pct: number; cavity_pressure_mpa: number; price_usd_kg: number; max_flow_ratio: number; min_draft_deg: number; notes: string;
}
export interface Steel { id: string; name: string; base_usd: number; per_cm2_usd: number; side_action_usd: number; shot_life: number; lead_weeks: number; notes: string }
export interface CycleBreakdown { fill_s: number; pack_s: number; cool_s: number; open_eject_close_s: number; total_s: number }
export interface PartProcess {
  part: string; pull_label: string; pull: V3; volume_cm3: number; mass_g: number; projected_area_mm2: number; clamp_force_kn: number;
  clamp_tonnes: number; recommended_press_t: number; wall_nominal_mm: number; wall_max_mm: number; cycle: CycleBreakdown;
  min_draft_deg: number; undercut_area_mm2: number; side_actions: number; gate: V3; flow_length_mm: number; flow_ratio: number;
  flow_ratio_limit: number; shrink_scale: number; tool_cost_usd: number; shot_life: number;
  piece: { material_usd: number; machine_usd: number; total_usd: number }; warnings: string[];
}
export interface ProcessOptions {
  material: string; steel: string; cavities: number; min_draft_deg: number; clamp_allowance: number; scrap_pct: number;
  runner_fraction: number; machine_rate_usd_h: number;
}
export interface Analysis {
  material: Material; steel: Steel; options: ProcessOptions; parts: PartProcess[]; clamp_tonnes: number; cycle_s: number;
  min_draft_deg: number; min_draft_part: string; tooling_total_usd: number; quantities: number[]; unit_cost: number[];
}
export interface PartInfo { name: string; color: string; triangles: number; volume_cm3: number; bbox_min: V3; bbox_max: V3; mesh: { positions: number[]; indices: number[] } }
