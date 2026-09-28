// TypeScript mirror of crates/forge-core/src/circuit (mod.rs, presets.rs, analysis.rs).

export interface ParamSpec { name: string; label: string; value: number; min: number; max: number; unit: string; log: boolean; hint?: string }
export interface Pin { name: string; node: string; x: number; y: number }
export interface Part { el: string; sym: string; x: number; y: number; dir: string; members: string[]; lpos: string; pins: Pin[] }
export interface Wire { node: string; pts: [number, number][]; flow: [string, number][] }
export interface Label { text: string; x: number; y: number; node: string | null; port: boolean }
export interface LBox { x: number; y: number; w: number; h: number; label: string }
export interface Layout { w: number; h: number; parts: Part[]; wires: Wire[]; grounds: [number, number][]; labels: Label[]; boxes: LBox[] }
export interface Preset {
  id: string; title: string; group: string; summary: string; description: string; iris: string;
  netlist: string; params: ParamSpec[]; probes: string[]; layout: Layout; links: { lab: string; label: string }[]; metric: string | null;
}

export interface ElementInfo { name: string; kind: string; nodes: string[]; value: string; wave: string; sw: [number, number] | null; line: number }
export interface OpResult { node_v: number[]; elem_i: number[]; elem_v: number[]; elem_p: number[]; elem_note: string[]; iterations: number; method: string }
export interface TranStats { accepted: number; rejected_lte: number; rejected_nr: number; nr_iterations: number; breakpoints: number; min_step: number; max_step: number; uic: boolean }
export interface TranResult { time: number[]; v: number[][]; i: number[][]; tstep: number; tstop: number; stats: TranStats }
export interface AcResult { freq: number[]; mag: number[][]; phase: number[][] }
export interface DcResult { source: string; sweep: number[]; v: number[][]; i: number[][] }
export interface Measurement { key: string; label: string; value: number; unit: string; note: string; pass: boolean | null }
export interface Annotation { plot: 'tran' | 'ac' | 'dc'; axis: 'x' | 'y'; value: number; label: string }
export interface SolverStats { unknowns: number; nodes: number; branches: number; lu_solves: number; nr_iterations: number }
export interface SimResult {
  title: string; nodes: string[]; elements: ElementInfo[]; params: Record<string, number>; analyses: string[];
  op: OpResult | null; dc: DcResult | null; tran: TranResult | null; ac: AcResult | null;
  measurements: Measurement[]; annotations: Annotation[]; tran_window: [number, number] | null;
  stats: SolverStats; warnings: string[]; netlist: string;
}
export interface ParseError { line: number; message: string }
export interface ParseOut { ok: boolean; errors: ParseError[]; warnings: string[]; title: string; nodes: string[]; elements: ElementInfo[]; analyses: string[]; params: Record<string, number> }

/** Values of the circuit at one instant (a transient frame or the DC operating point). */
export interface Snapshot {
  /** node name → volts ("0" → 0) */
  v: (node: string) => number;
  /** element name → current (SPICE convention: into n+ / drain) */
  i: (el: string) => number;
  /** time of the frame, or null for the operating point */
  t: number | null;
}
