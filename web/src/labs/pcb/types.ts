// TypeScript mirror of the pcb.* engine outputs (crates/forge-core/src/pcb).

export type PadShape = 'rect' | 'round_rect' | 'circle';
export type NetClass = 'ground' | 'power' | 'signal' | 'rf';

export interface Rules {
  trace_mm: number; clearance_mm: number; via_drill_mm: number; via_pad_mm: number; grid_mm: number;
  edge_clearance_mm: number; pour_clearance_mm: number; min_drill_mm: number; min_annular_mm: number; hole_clearance_mm: number;
}
export interface PlacedPart {
  refdes: string; footprint: string; value: string; x: number; y: number; rot: number; side: string; feature: boolean;
  courtyard: number[]; body: number[]; height: number; pin1: number[];
}
export interface BoardPad {
  refdes: string; pin: string; net: string; x: number; y: number; w: number; h: number; shape: PadShape;
  drill: number; top: boolean; bottom: boolean; mech: boolean; paste: number;
}
export interface Track { net: string; layer: number; x0: number; y0: number; x1: number; y1: number; width: number }
export interface Via { net: string; x: number; y: number; drill: number; pad: number }
export interface Hole { x: number; y: number; d: number; refdes: string }
export interface Keepout { name: string; rect: number[]; top: boolean; bottom: boolean; allow_net: string; no_pour: boolean }
export interface Pour { layer: number; cell_mm: number; rects: number[][]; area_mm2: number; islands_removed: number; regions: number }
export interface Violation { kind: string; message: string; x: number; y: number; items: string[] }
export interface NetView {
  name: string; class: NetClass; pins: string[]; description: string; current_a: number; routed: boolean;
  length_mm: number; vias: number; unconnected: string[]; ripups: number; ipc_width_mm: number;
}
export interface Stats {
  routed_pct: number; connections: number; routed: number; drc_violations: number; via_count: number;
  track_length_mm: number; track_length_top_mm: number; track_length_bottom_mm: number; hpwl_mm: number;
  pour_area_mm2: number; pour_islands_removed: number; ripups: number; expansions: number; parts: number; pads: number;
  nets: number; smd_joints: number; tht_joints: number; unique_parts: number; strategy: string; strategy_index: number;
  attempts: number; from_cache: boolean;
}
export interface SilkLabel { text: string; x: number; y: number; h: number }
export interface Silk { lines: number[][]; dots: number[][]; labels: SilkLabel[] }
export interface Layout {
  board: { width: number; height: number; thickness: number; layers: number };
  rules: Rules; parts: PlacedPart[]; pads: BoardPad[]; nets: NetView[]; tracks: Track[]; vias: Via[];
  holes: Hole[]; keepouts: Keepout[]; pour: Pour; pour_top: Pour; drc: Violation[];
  connectivity: { connections: number; routed: number };
  stats: Stats; gpio: [string, string][]; notes: string[]; silk: Silk;
}
export interface PartPos { refdes: string; x: number; y: number; rot: number; side: string }
export interface AnnealFrame { iteration: number; temperature: number; hpwl: number; overlap: number; cost: number; parts: PartPos[] }
export interface AnnealResult {
  iterations: number[]; hpwl: number[]; best_hpwl: number[]; temperature: number[]; acceptance: number[];
  frames: AnnealFrame[]; initial_hpwl: number; final_hpwl: number; constructive_hpwl: number;
  final_parts: PartPos[]; movable: string[]; final_overlap: number;
}
export interface Wave { from_pin: string; expanded: number[]; path: number[]; found: boolean }
export interface WaveTrace { net: string; nx: number; ny: number; grid_mm: number; waves: Wave[] }
export interface FabFile { path: string; content: string; kind: string }
export interface PadDef { num: string; x: number; y: number; w: number; h: number; shape: PadShape; drill: number; paste: number; mech: boolean }
export interface Footprint {
  name: string; description: string; pads: PadDef[]; courtyard: number[]; body: number[]; height: number;
  pin1: number[]; npth: number[][]; note: string;
}
