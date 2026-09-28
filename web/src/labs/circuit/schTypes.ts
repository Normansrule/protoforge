// Schematic editor model — mirrors crates/forge-core/src/circuit/schematic.rs.
import type { SimResult } from './types';

export interface SchPart {
  id: string; kind: string; name: string; x: number; y: number; rot: number; flip: boolean;
  value: string; props: Record<string, string>; template?: string | null; pin_hint?: string[];
}
export interface SchWire { pts: [number, number][] }
export interface AnalysisSpec {
  op: boolean; tran: boolean; tstep: string; tstop: string; tmax: string; uic: boolean;
  ac: boolean; ac_points: number; fstart: string; fstop: string;
  dc: boolean; dc_source: string; dc_start: string; dc_stop: string; dc_step: string;
}
export interface SchDoc { title: string; parts: SchPart[]; wires: SchWire[]; analysis: AnalysisSpec; directives: string[] }

export interface PinPos { name: string; x: number; y: number }
export interface FlowWire { net: string; pts: [number, number][]; flow: [string, number][] }
export interface NetInfo { name: string; pins: string[]; labels: string[]; probe: boolean }
export interface PartNets { id: string; name: string; pins: PinPos[]; nets: string[]; elements: string[]; lines: number[] }
export interface Extracted { netlist: string; nets: NetInfo[]; parts: PartNets[]; wires: FlowWire[]; wire_nets: string[]; junctions: [number, number][]; probes: string[]; node_count: number; warnings: string[] }
export interface SchSimOut { extracted: Extracted; result: SimResult | null; error: string | null }
export interface SchExample { id: string; title: string; summary: string; schematic: SchDoc }

export const DEFAULT_ANALYSIS: AnalysisSpec = {
  op: true, tran: true, tstep: '10u', tstop: '5m', tmax: '', uic: false,
  ac: false, ac_points: 20, fstart: '10', fstop: '1meg',
  dc: false, dc_source: 'V1', dc_start: '0', dc_stop: '5', dc_step: '0.05',
};

export const EMPTY_DOC: SchDoc = { title: 'My schematic', parts: [], wires: [], analysis: DEFAULT_ANALYSIS, directives: [] };

/** Local pin offsets at rotation 0 (pin 1 up, pin 2 down) — same table as the engine. */
export function localPins(kind: string): [string, number, number][] {
  switch (kind) {
    case 'nmos': case 'pmos': return [['top', 0, -2], ['g', -2, 0], ['bottom', 0, 2]];
    case 'opamp': return [['in+', -3, -1], ['in-', -3, 1], ['out', 3, 0]];
    case 'ground': case 'label': case 'probe': return [['n', 0, 0]];
    case 'text': return [];
    case 'diode': case 'led': return [['a', 0, -2], ['k', 0, 2]];
    case 'vsource': case 'battery': case 'isource': return [['+', 0, -2], ['-', 0, 2]];
    default: return [['1', 0, -2], ['2', 0, 2]];
  }
}

export function transform(dx: number, dy: number, rot: number, flip: boolean): [number, number] {
  let x = flip ? -dx : dx;
  let y = dy;
  for (let k = 0; k < ((rot % 4) + 4) % 4; k++) { const nx = -y; y = x; x = nx; }
  return [x + 0, y + 0];
}

export function pinsOf(p: SchPart): PinPos[] {
  return localPins(p.kind).map(([name, dx, dy]) => { const [tx, ty] = transform(dx, dy, p.rot, p.flip); return { name, x: p.x + tx, y: p.y + ty }; });
}

export interface PaletteItem { kind: string; label: string; prefix: string; value: string; props?: Record<string, string>; hint: string }
export const PALETTE: PaletteItem[] = [
  { kind: 'resistor', label: 'Resistor', prefix: 'R', value: '1k', hint: 'R — ohms, e.g. 4.7k' },
  { kind: 'capacitor', label: 'Capacitor', prefix: 'C', value: '100n', hint: 'C — farads, e.g. 10u' },
  { kind: 'inductor', label: 'Inductor', prefix: 'L', value: '10u', hint: 'L — henries, e.g. 2.2u' },
  { kind: 'vsource', label: 'Voltage source', prefix: 'V', value: '5', props: { wave: 'dc' }, hint: 'DC, pulse or sine' },
  { kind: 'isource', label: 'Current source', prefix: 'I', value: '1m', props: { wave: 'dc' }, hint: 'current flows through it from + to −' },
  { kind: 'battery', label: 'Battery', prefix: 'VB', value: '3.7', hint: 'a DC voltage source' },
  { kind: 'diode', label: 'Diode', prefix: 'D', value: '', hint: 'silicon small-signal diode model' },
  { kind: 'led', label: 'LED', prefix: 'D', value: '', hint: 'generic red LED (~1.9 V at 20 mA)' },
  { kind: 'nmos', label: 'NMOS', prefix: 'M', value: '', props: { w: '1m', l: '1u' }, hint: 'level-1 MOSFET, bulk tied to source' },
  { kind: 'pmos', label: 'PMOS', prefix: 'M', value: '', props: { w: '1m', l: '1u' }, hint: 'level-1 MOSFET, bulk tied to source' },
  { kind: 'switch', label: 'Switch', prefix: 'S', value: '', props: { state: 'closed', freq: '100k', duty: '0.5' }, hint: 'closed / open / PWM' },
  { kind: 'opamp', label: 'Op-amp', prefix: 'U', value: '', props: { a0: '100k', gbw: '1meg' }, hint: 'single-pole macro-model' },
  { kind: 'ground', label: 'Ground', prefix: 'GND', value: '', hint: 'node 0' },
  { kind: 'label', label: 'Net label', prefix: 'net', value: 'net', hint: 'same name = same net' },
  { kind: 'probe', label: 'Probe', prefix: 'P', value: '', hint: 'plot v(net) of what it touches' },
];

/** Parse a SPICE number with an engineering suffix (m = milli, meg = mega). */
export function spiceNum(s: string): number | null {
  const t = s.trim().replace('µ', 'u').replace('Ω', '').toLowerCase();
  const m = /^([+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)([a-z]*)$/.exec(t);
  if (!m) return null;
  const v = Number(m[1]);
  const suf = m[2];
  const k = suf.startsWith('meg') ? 1e6 : ({ t: 1e12, g: 1e9, k: 1e3, m: 1e-3, u: 1e-6, n: 1e-9, p: 1e-12, f: 1e-15 } as Record<string, number>)[suf[0]] ?? 1;
  return v * k;
}
