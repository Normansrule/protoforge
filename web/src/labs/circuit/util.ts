import type { ElementInfo, SimResult, Snapshot } from './types';

const PREFIX: [number, string][] = [[1e12, 'T'], [1e9, 'G'], [1e6, 'M'], [1e3, 'k'], [1, ''], [1e-3, 'm'], [1e-6, 'µ'], [1e-9, 'n'], [1e-12, 'p'], [1e-15, 'f']];

/** 4.7e-6, 'F' → "4.7 µF" */
export function si(v: number | null | undefined, unit = '', digits = 3): string {
  if (typeof v !== 'number' || !isFinite(v)) return '—';
  if (v === 0) return `0 ${unit}`.trim();
  const a = Math.abs(v);
  const [s, p] = PREFIX.find(([s]) => a >= s * 0.9995) ?? [1e-15, 'f'];
  const m = v / s;
  let txt = m.toPrecision(digits);
  if (txt.includes('e')) txt = String(Number(txt));
  if (txt.includes('.')) txt = txt.replace(/0+$/, '').replace(/\.$/, '');
  return `${txt} ${p}${unit}`.trim();
}

/** Format a measurement value with its unit. */
export function fmtMeas(v: number | null, unit: string): string {
  if (typeof v !== 'number' || !isFinite(v)) return '—';
  const a = Math.abs(v);
  const txt = a >= 1000 ? v.toFixed(0) : a >= 100 ? v.toFixed(1) : a >= 10 ? v.toFixed(2) : a >= 0.01 || a === 0 ? v.toFixed(3) : v.toExponential(2);
  return unit ? `${txt} ${unit}` : txt;
}

/** Index of the last sample with time ≤ t (binary search). */
export function frameAt(time: number[], t: number): number {
  let lo = 0, hi = time.length - 1;
  if (hi < 0) return 0;
  if (t <= time[0]) return 0;
  if (t >= time[hi]) return hi;
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    if (time[mid] <= t) lo = mid; else hi = mid;
  }
  return lo;
}

/** Build a snapshot accessor for a transient frame (or the operating point). */
export function snapshot(res: SimResult | null, frame: number | null): Snapshot {
  const nodeIdx = new Map<string, number>();
  const elIdx = new Map<string, number>();
  res?.nodes.forEach((n, k) => nodeIdx.set(n.toLowerCase(), k));
  res?.elements.forEach((e, k) => elIdx.set(e.name.toLowerCase(), k));
  const tr = res?.tran;
  if (res && tr && frame !== null && tr.time.length > 0) {
    const f = Math.max(0, Math.min(frame, tr.time.length - 1));
    return {
      t: tr.time[f],
      v: (n) => { const k = nodeIdx.get(n.toLowerCase()); return k === undefined ? 0 : tr.v[k][f]; },
      i: (e) => { const k = elIdx.get(e.toLowerCase()); return k === undefined ? 0 : tr.i[k][f]; },
    };
  }
  const op = res?.op;
  return {
    t: null,
    v: (n) => { const k = nodeIdx.get(n.toLowerCase()); return k === undefined || !op ? 0 : op.node_v[k]; },
    i: (e) => { const k = elIdx.get(e.toLowerCase()); return k === undefined || !op ? 0 : op.elem_i[k]; },
  };
}

/** Voltage range over the whole run (for a stable colour scale). */
export function voltageRange(res: SimResult | null, nodes: string[]): [number, number] {
  let lo = 0, hi = 0;
  if (!res) return [0, 1];
  const want = new Set(nodes.map((n) => n.toLowerCase()));
  res.nodes.forEach((n, k) => {
    if (!want.has(n)) return;
    const arr = res.tran ? res.tran.v[k] : res.op ? [res.op.node_v[k]] : [];
    for (const x of arr) { if (x < lo) lo = x; if (x > hi) hi = x; }
  });
  // ignore numerical dust (e.g. −470 nV on a grounded node)
  if (Math.abs(lo) < 1e-3 * Math.max(Math.abs(hi), 1e-12)) lo = 0;
  if (hi - lo < 1e-9) hi = lo + 1;
  return [lo, hi];
}

// Sequential voltage ramp (perceptually ordered, readable on dark and light).
const RAMP: [number, number, number][] = [
  [72, 86, 178], [55, 132, 214], [38, 176, 190], [96, 196, 120], [232, 190, 60], [238, 118, 52],
];

export function rampColor(v: number, lo: number, hi: number): string {
  let f = (v - lo) / (hi - lo || 1);
  f = Math.max(0, Math.min(1, f));
  const x = f * (RAMP.length - 1);
  const k = Math.min(RAMP.length - 2, Math.floor(x));
  const u = x - k;
  const c = RAMP[k].map((a, j) => Math.round(a + (RAMP[k + 1][j] - a) * u));
  return `rgb(${c[0]},${c[1]},${c[2]})`;
}

export const RAMP_CSS = `linear-gradient(90deg, ${RAMP.map((c) => `rgb(${c.join(',')})`).join(', ')})`;

/** Element voltage (n+ − n−; MOSFET d − s) and power at a snapshot. */
export function elemVIP(e: ElementInfo, s: Snapshot) {
  const [a, b] = e.kind === 'nmos' || e.kind === 'pmos' ? [e.nodes[0], e.nodes[2]] : [e.nodes[0], e.nodes[1]];
  const v = s.v(a) - s.v(b);
  const i = s.i(e.name);
  return { v, i, p: v * i };
}

/** Is a switch closed at this snapshot? (|V/I| below √(Ron·Roff)). */
export function switchOn(e: ElementInfo, s: Snapshot): boolean {
  if (!e.sw) return false;
  const { v, i } = elemVIP(e, s);
  if (Math.abs(i) < 1e-15) return false;
  return Math.abs(v / i) < Math.sqrt(e.sw[0] * e.sw[1]);
}

/** CSV of the transient waveforms. */
export function tranCsv(res: SimResult): string {
  const tr = res.tran!;
  const head = ['time', ...res.nodes.map((n) => `v(${n})`), ...res.elements.map((e) => `i(${e.name})`)];
  const rows = [head.join(',')];
  for (let k = 0; k < tr.time.length; k++) {
    rows.push([tr.time[k].toExponential(9), ...tr.v.map((c) => c[k].toPrecision(7)), ...tr.i.map((c) => c[k].toPrecision(7))].join(','));
  }
  return rows.join('\n') + '\n';
}

export function acCsv(res: SimResult): string {
  const ac = res.ac!;
  const head = ['frequency', ...res.nodes.flatMap((n) => [`mag(${n})`, `db(${n})`, `phase_deg(${n})`])];
  const rows = [head.join(',')];
  for (let k = 0; k < ac.freq.length; k++) {
    rows.push([ac.freq[k].toExponential(9), ...res.nodes.flatMap((_, j) => [ac.mag[j][k].toPrecision(7), (20 * Math.log10(Math.max(ac.mag[j][k], 1e-30))).toFixed(4), ac.phase[j][k].toFixed(4)])].join(','));
  }
  return rows.join('\n') + '\n';
}

export function dcCsv(res: SimResult): string {
  const dc = res.dc!;
  const head = [dc.source, ...res.nodes.map((n) => `v(${n})`), ...res.elements.map((e) => `i(${e.name})`)];
  const rows = [head.join(',')];
  for (let k = 0; k < dc.sweep.length; k++) {
    rows.push([dc.sweep[k].toPrecision(9), ...dc.v.map((c) => c[k].toPrecision(7)), ...dc.i.map((c) => c[k].toPrecision(7))].join(','));
  }
  return rows.join('\n') + '\n';
}

export function opCsv(res: SimResult): string {
  const op = res.op!;
  const rows = ['kind,name,value,voltage_V,current_A,power_W,note'];
  res.nodes.forEach((n, k) => rows.push(`node,${n},,${op.node_v[k]},,,`));
  res.elements.forEach((e, k) => rows.push(`${e.kind},${e.name},"${e.value}",${op.elem_v[k]},${op.elem_i[k]},${op.elem_p[k]},"${op.elem_note[k]}"`));
  return rows.join('\n') + '\n';
}

/** Parse "line N: message" errors from the engine. */
export function parseErrorLines(err: string | null): { line: number; message: string }[] {
  if (!err) return [];
  const body = err.replace(/^circuit\.\w+:\s*/, '');
  return body.split('\n').filter(Boolean).map((l) => {
    const m = /^line (\d+): (.*)$/.exec(l.trim());
    return m ? { line: Number(m[1]), message: m[2] } : { line: 0, message: l.trim() };
  });
}

export const prefersReducedMotion = () =>
  typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
