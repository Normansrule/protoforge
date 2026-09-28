// Types for the bridge.* engine methods (crates/forge-core/src/bridge).

export interface BundleFile { path: string; content: string; encoding: string }

export interface KicadStats {
  footprints: number; pads: number; segments: number; vias: number; zones: number; keepouts: number; nets: number;
  width_mm: number; height_mm: number; layers: number;
}
export interface KicadBoard { filename: string; content: string; stats: KicadStats; notes: string[] }

export interface StepPartStats { name: string; shells: number; closed_shells: number; faces: number; points: number; volume_mm3: number; euler: number[] }
export interface StepStats { parts: StepPartStats[]; shells: number; closed_shells: number; faces: number; points: number; entities: number; bytes: number }
export interface StepFile { filename: string; content: string; stats: StepStats; notes: string[] }

export interface FalstadCircuit {
  id: string; title: string; group: string; text: string; url: string;
  support: 'full' | 'approximate' | 'partial' | 'none'; elements: number; unsupported: string[]; notes: string[];
  connectivity_ok: boolean; netlist: string;
}
