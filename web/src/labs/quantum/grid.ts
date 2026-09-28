// The composer's grid model (Quirk / IBM Composer style) and its conversion
// to and from the engine's circuit format.
//
// A grid column holds one cell per qubit. As in Quirk, control dots (•) in a
// column control every gate in that column; two SWAP (or iSWAP) cells in a
// column form one two-qubit gate; a barrier column holds nothing else.
import type { Circuit, Cond, Creg, Op } from './types';

export interface Cell {
  g: string; // gate id (h x y z s sdg t tdg sx rx ry rz p u3 swap iswap measure reset) or 'ctrl'
  p?: number[];
  c?: number | null; // classical bit written by a measurement
  cond?: Cond | null; // classical condition
  label?: string | null;
}
export interface Col { cells: (Cell | null)[]; barrier?: boolean }
export interface Grid { qubits: number; clbits: number; cols: Col[]; cregs?: Creg[] }

export const PARAMS: Record<string, string[]> = { rx: ['θ'], ry: ['θ'], rz: ['θ'], p: ['λ'], u3: ['θ', 'φ', 'λ'] };

export const LABEL: Record<string, string> = {
  h: 'H', x: 'X', y: 'Y', z: 'Z', s: 'S', sdg: 'S†', t: 'T', tdg: 'T†', sx: '√X', sxdg: '√X†', rx: 'RX', ry: 'RY', rz: 'RZ', p: 'P', u3: 'U',
  swap: '×', iswap: 'iSW', measure: 'M', reset: '|0⟩', ctrl: '•', id: 'I',
};

export function emptyGrid(qubits = 3, cols = 8): Grid {
  return { qubits, clbits: qubits, cols: Array.from({ length: cols }, () => ({ cells: Array(qubits).fill(null) })) };
}

export function gridToCircuit(g: Grid): { circuit: Circuit; errors: string[] } {
  const errors: string[] = [];
  const cols: Op[][] = [];
  g.cols.forEach((col, ci) => {
    if (col.barrier) {
      cols.push([{ gate: 'barrier', targets: Array.from({ length: g.qubits }, (_, k) => k) }]);
      return;
    }
    const cells = col.cells.slice(0, g.qubits);
    const controls = cells.map((c, q) => (c?.g === 'ctrl' ? q : -1)).filter((q) => q >= 0);
    const ops: Op[] = [];
    for (const two of ['swap', 'iswap']) {
      const qs = cells.map((c, q) => (c?.g === two ? q : -1)).filter((q) => q >= 0);
      if (qs.length === 0) continue;
      if (qs.length !== 2) {
        errors.push(`column ${ci + 1}: ${two === 'swap' ? 'SWAP' : 'iSWAP'} needs exactly two cells in one column (found ${qs.length})`);
        continue;
      }
      const first = cells[qs[0]]!;
      ops.push({ gate: two, targets: qs, controls, cond: first.cond ?? null });
    }
    cells.forEach((c, q) => {
      if (!c || c.g === 'ctrl' || c.g === 'swap' || c.g === 'iswap') return;
      if ((c.g === 'measure' || c.g === 'reset') && controls.length > 0) {
        errors.push(`column ${ci + 1}: a measurement or reset cannot share a column with control dots`);
        return;
      }
      const op: Op = { gate: c.g, targets: [q], controls, params: c.p ?? [], cond: c.cond ?? null };
      if (c.g === 'measure') op.clbit = c.c ?? (q < g.clbits ? q : null);
      if (c.label) op.label = c.label;
      ops.push(op);
    });
    if (controls.length > 0 && ops.length === 0) errors.push(`column ${ci + 1}: control dots need a gate in the same column`);
    cols.push(ops); // keep grid and engine columns aligned for step-through
  });
  // drop trailing empty columns
  while (cols.length > 0 && cols[cols.length - 1].length === 0) cols.pop();
  return { circuit: { qubits: g.qubits, clbits: g.clbits, cols, cregs: g.cregs ?? [] }, errors };
}

/** Map from grid column index → engine column index (engine drops trailing empties). */
export function circuitToGrid(c: Circuit): Grid {
  const cols: Col[] = [];
  for (const ops of c.cols) {
    if (ops.length === 1 && ops[0].gate === 'barrier') {
      cols.push({ cells: Array(c.qubits).fill(null), barrier: true });
      continue;
    }
    let cur: Col = { cells: Array(c.qubits).fill(null) };
    let curCtrl: number[] | null = null;
    const flush = () => {
      cols.push(cur);
      cur = { cells: Array(c.qubits).fill(null) };
      curCtrl = null;
    };
    for (const op of ops) {
      if (op.gate === 'barrier') continue;
      const ctl = [...(op.controls ?? [])].sort();
      const qs = [...op.targets, ...ctl];
      const busy = qs.some((q) => cur.cells[q] !== null) || (curCtrl !== null && JSON.stringify(curCtrl) !== JSON.stringify(ctl)) || (curCtrl === null && ctl.length > 0 && cur.cells.some((x) => x));
      if (busy) flush();
      for (const t of op.targets) cur.cells[t] = { g: op.gate, p: op.params ?? [], c: op.clbit ?? null, cond: op.cond ?? null, label: op.label ?? null };
      for (const q of ctl) cur.cells[q] = { g: 'ctrl' };
      curCtrl = ctl;
    }
    flush();
  }
  return { qubits: c.qubits, clbits: Math.max(c.clbits, 0), cols, cregs: c.cregs ?? [] };
}

export function withQubits(g: Grid, n: number): Grid {
  const q = Math.max(1, Math.min(14, n));
  return {
    ...g,
    qubits: q,
    clbits: Math.max(g.clbits, q),
    cols: g.cols.map((c) => ({ ...c, cells: Array.from({ length: q }, (_, k) => (k < c.cells.length ? c.cells[k] : null)) })),
  };
}

/** Pretty angle: multiples of π where exact. */
export function fmtAngle(x: number): string {
  if (Math.abs(x) < 1e-9) return '0';
  for (const den of [1, 2, 3, 4, 6, 8, 16]) {
    const k = (x / Math.PI) * den;
    if (Math.abs(k - Math.round(k)) < 1e-6) {
      const n = Math.round(k);
      const s = n < 0 ? '−' : '';
      const num = Math.abs(n) === 1 ? 'π' : `${Math.abs(n)}π`;
      return den === 1 ? `${s}${num}` : `${s}${num}/${den}`;
    }
  }
  return x.toFixed(3);
}

export function cellTitle(c: Cell): string {
  const name = LABEL[c.g] ?? c.g;
  const ps = c.p && c.p.length ? `(${c.p.map(fmtAngle).join(', ')})` : '';
  const cond = c.cond ? ` if c[${c.cond.bits.join(',')}] = ${c.cond.value}` : '';
  return `${name}${ps}${cond}`;
}
