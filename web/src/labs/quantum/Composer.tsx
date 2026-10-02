// Circuit composer: drag gates onto qubit wires (or tap a gate, then tap a
// cell), edit rotation angles and classical conditions, step through the
// state column by column, and export OpenQASM 2.0 or open the circuit in Quirk.
import { useEffect, useMemo, useState, type DragEvent } from 'react';
import { Card, Chip, Code, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider } from '../../components/ui';
import { Icon } from '../../components/Icon';
import { tryCall } from '../../engine/engine';
import { useCompute } from '../../engine/hooks';
import { circuitToGrid, cellTitle, emptyGrid, fmtAngle, gridToCircuit, LABEL, PARAMS, withQubits, type Cell, type Grid } from './grid';
import type { Entry, ExportOut, GateInfo, Listing, ParseOut, RunOut, Snapshot } from './types';
import { AmplitudeBars, BlochSphere, csv, Histogram, PhaseWheel, PlayBar, Seg, usePlayer } from './viz';

const STORE = 'protoforge.quantum.grid.v1';

function bellGrid(): Grid {
  const g = emptyGrid(2, 8);
  g.clbits = 2;
  g.cols[0].cells[0] = { g: 'h' };
  g.cols[1].cells[0] = { g: 'ctrl' };
  g.cols[1].cells[1] = { g: 'x' };
  g.cols[2].cells[0] = { g: 'measure', c: 0 };
  g.cols[2].cells[1] = { g: 'measure', c: 1 };
  return g;
}

function loadGrid(): Grid {
  try {
    const s = localStorage.getItem(STORE);
    if (s) {
      const g = JSON.parse(s) as Grid;
      if (g && g.qubits > 0 && Array.isArray(g.cols)) return g;
    }
  } catch {
    /* private mode or blocked storage: start fresh */
  }
  return bellGrid();
}

const PALETTE: { group: string; ids: string[] }[] = [
  { group: 'Single qubit', ids: ['h', 'x', 'y', 'z', 's', 'sdg', 't', 'tdg', 'sx'] },
  { group: 'Rotations', ids: ['rx', 'ry', 'rz', 'p', 'u3'] },
  { group: 'Multi-qubit', ids: ['cnot', 'cz', 'ccx', 'swap2', 'ctrl', 'swap', 'iswap'] },
  { group: 'Other', ids: ['measure', 'reset', 'barrier'] },
];

/** Multi-cell tiles: cells placed on consecutive qubits starting at the tapped wire. */
const PATTERNS: Record<string, { name: string; cells: Cell[]; tip: string }> = {
  cnot: { name: 'CNOT', cells: [{ g: 'ctrl' }, { g: 'x' }], tip: 'CNOT (controlled-X): flips the lower qubit when the upper is |1⟩ — the entangling gate' },
  cz: { name: 'CZ', cells: [{ g: 'ctrl' }, { g: 'z' }], tip: 'CZ (controlled-Z): phase −1 on |11⟩, symmetric in its two qubits' },
  ccx: { name: 'Toffoli', cells: [{ g: 'ctrl' }, { g: 'ctrl' }, { g: 'x' }], tip: 'Toffoli (CCX): flips the third qubit when both controls are |1⟩ — a reversible AND' },
  swap2: { name: 'SWAP', cells: [{ g: 'swap' }, { g: 'swap' }], tip: 'SWAP: exchanges the states of two neighbouring qubits' },
};

function PatternGlyph({ id }: { id: string }) {
  const pat = PATTERNS[id];
  const n = pat.cells.length;
  const H = 30, dy = H / n;
  return (
    <svg width={30} height={H} viewBox={`0 0 30 ${H}`} aria-label={pat.name}>
      <line x1={15} x2={15} y1={dy / 2} y2={H - dy / 2} stroke="currentColor" strokeWidth={1.5} />
      {pat.cells.map((c, k) => {
        const y = dy / 2 + k * dy;
        if (c.g === 'ctrl') return <circle key={k} cx={15} cy={y} r={3.2} fill="currentColor" />;
        if (c.g === 'swap') return <text key={k} x={15} y={y + 4} fontSize={12} textAnchor="middle" fill="currentColor" fontWeight={700}>×</text>;
        if (c.g === 'x') return <g key={k}><circle cx={15} cy={y} r={5.5} fill="var(--surface-1)" stroke="currentColor" strokeWidth={1.4} /><line x1={9.5} x2={20.5} y1={y} y2={y} stroke="currentColor" strokeWidth={1.4} /><line x1={15} x2={15} y1={y - 5.5} y2={y + 5.5} stroke="currentColor" strokeWidth={1.4} /></g>;
        return <g key={k}><rect x={9} y={y - 6} width={12} height={12} rx={2} fill="var(--surface-1)" stroke="currentColor" strokeWidth={1.2} /><text x={15} y={y + 3.5} fontSize={9} textAnchor="middle" fill="currentColor" fontWeight={700}>Z</text></g>;
      })}
    </svg>
  );
}

const DEFAULT_P: Record<string, number[]> = { rx: [Math.PI / 2], ry: [Math.PI / 2], rz: [Math.PI / 4], p: [Math.PI / 4], u3: [Math.PI / 2, 0, Math.PI] };

type V3 = [number, number, number];
function slerp(a: V3, b: V3, t: number): V3 {
  const ra = Math.hypot(...a), rb = Math.hypot(...b);
  if (ra < 1e-6 || rb < 1e-6) return a.map((x, k) => x + (b[k] - x) * t) as V3;
  const ua = a.map((x) => x / ra), ub = b.map((x) => x / rb);
  const dot = Math.max(-1, Math.min(1, ua[0] * ub[0] + ua[1] * ub[1] + ua[2] * ub[2]));
  const om = Math.acos(dot);
  const r = ra + (rb - ra) * t;
  if (om < 1e-4) return ua.map((x, k) => (x + (ub[k] - x) * t) * r) as V3;
  if (Math.PI - om < 1e-3) {
    // antipodal: rotate through an orthogonal axis
    const ortho: V3 = Math.abs(ua[1]) < 0.9 ? [0, -1, 0] : [1, 0, 0];
    const d = ortho[0] * ua[0] + ortho[1] * ua[1] + ortho[2] * ua[2];
    const perp = ortho.map((x, k) => x - d * ua[k]);
    const pl = Math.hypot(...perp) || 1;
    const mid = perp.map((x) => (x / pl) * (ra + rb) / 2) as V3;
    return t < 0.5 ? slerp(a, mid, t * 2) : slerp(mid, b, (t - 0.5) * 2);
  }
  const s = Math.sin(om);
  const wa = Math.sin((1 - t) * om) / s, wb = Math.sin(t * om) / s;
  return ua.map((x, k) => (wa * x + wb * ub[k]) * r) as V3;
}

export function GateGlyph({ cell, small }: { cell: Cell; small?: boolean }) {
  if (cell.g === 'ctrl') return <span className="qx-ctrl" aria-label="control" />;
  if (cell.g === 'measure') {
    return (
      <span className="qx-gate meas" aria-label="measure">
        <svg width={small ? 18 : 22} height={small ? 14 : 16} viewBox="0 0 22 16" aria-hidden>
          <path d="M3 13 A8 8 0 0 1 19 13" fill="none" stroke="currentColor" strokeWidth={1.6} />
          <line x1={11} y1={13} x2={17} y2={4} stroke="currentColor" strokeWidth={1.6} />
        </svg>
        {cell.c != null && <sub>{cell.c}</sub>}
      </span>
    );
  }
  if (cell.g === 'swap') return <span className="qx-swap" aria-label="swap">×</span>;
  const isRot = PARAMS[cell.g] !== undefined;
  return (
    <span className={`qx-gate g-${cell.g} ${isRot ? 'rot' : ''}`}>
      <span>{LABEL[cell.g] ?? cell.g}</span>
      {isRot && !small && cell.p && <span className="qx-gate-p">{cell.g === 'u3' ? 'θφλ' : fmtAngle(cell.p[0] ?? 0)}</span>}
      {cell.cond && <span className="qx-cond">c{cell.cond.bits.join('')}</span>}
    </span>
  );
}

export default function Composer() {
  const [grid, setGrid] = useState<Grid>(loadGrid);
  const [armed, setArmed] = useState<string | null>(null);
  const [sel, setSel] = useState<{ col: number; q: number } | null>(null);
  const [shots, setShots] = useState(1024);
  const [seed, setSeed] = useState(1);
  const [preset, setPreset] = useState('');
  const [qasmIn, setQasmIn] = useState('');
  const [importErr, setImportErr] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [fmt, setFmt] = useState<'qasm2' | 'qasm3' | 'qiskit'>('qasm2');

  useEffect(() => {
    try {
      localStorage.setItem(STORE, JSON.stringify(grid));
    } catch {
      /* ignore */
    }
  }, [grid]);

  const gates = useCompute<GateInfo[]>('quantum.gates', {});
  const lib = useCompute<Listing[]>('quantum.library', {});
  const { circuit, errors } = useMemo(() => gridToCircuit(grid), [grid]);
  const ok = errors.length === 0;
  const run = useCompute<RunOut>('quantum.run', { circuit, shots, seed, steps: true }, ok);
  const exp = useCompute<ExportOut>('quantum.export', { circuit }, ok);
  const [lastRun, setLastRun] = useState<RunOut | null>(null);
  useEffect(() => {
    if (run.data) setLastRun(run.data);
  }, [run.data]);
  const res = run.data ?? lastRun;

  const nSteps = res?.steps.length ?? 1;
  const player = usePlayer(nSteps, 1.1, { resetKey: JSON.stringify(circuit), resetToEnd: true });
  const step: Snapshot | null = res?.steps[Math.min(player.i, nSteps - 1)] ?? null;
  const next: Snapshot | null = res?.steps[Math.min(player.i + 1, nSteps - 1)] ?? null;
  const activeCol = player.i - 1; // column just applied

  const info = (id: string) => gates.data?.find((g) => g.id === id);

  // --- editing ---------------------------------------------------------
  const edit = (f: (g: Grid) => Grid) => setGrid((g) => {
    const n = f(structuredClone(g));
    // keep one empty column at the end to drop into
    const last = n.cols[n.cols.length - 1];
    if (!last || last.barrier || last.cells.some((c) => c)) n.cols.push({ cells: Array(n.qubits).fill(null) });
    return n;
  });
  const place = (gid: string, col: number, q: number) => {
    const pat = PATTERNS[gid];
    if (pat) {
      if (grid.qubits < pat.cells.length) {
        setNotice(`${pat.name} needs ${pat.cells.length} qubits — add a qubit first.`);
        return;
      }
      const q0 = Math.min(q, grid.qubits - pat.cells.length);
      edit((g) => {
        while (g.cols.length <= col) g.cols.push({ cells: Array(g.qubits).fill(null) });
        const c = g.cols[col];
        c.barrier = false;
        // a column holds one controlled operation: clear it before placing a multi-qubit pattern
        if (c.cells.some((x) => x)) c.cells = Array(g.qubits).fill(null);
        pat.cells.forEach((cell, k) => { c.cells[q0 + k] = { ...cell }; });
        return g;
      });
      setSel({ col, q: q0 + pat.cells.length - 1 });
      return;
    }
    edit((g) => {
      while (g.cols.length <= col) g.cols.push({ cells: Array(g.qubits).fill(null) });
      const c = g.cols[col];
      if (gid === 'barrier') {
        g.cols[col] = { cells: Array(g.qubits).fill(null), barrier: true };
        return g;
      }
      c.barrier = false;
      const cell: Cell = { g: gid };
      if (PARAMS[gid]) cell.p = [...(DEFAULT_P[gid] ?? PARAMS[gid].map(() => 0))];
      if (gid === 'measure') {
        cell.c = q;
        g.clbits = Math.max(g.clbits, q + 1);
      }
      c.cells[q] = cell;
      return g;
    });
    setSel({ col, q });
  };
  const move = (from: { col: number; q: number }, col: number, q: number) => edit((g) => {
    const cell = g.cols[from.col]?.cells[from.q];
    if (!cell) return g;
    g.cols[from.col].cells[from.q] = null;
    while (g.cols.length <= col) g.cols.push({ cells: Array(g.qubits).fill(null) });
    g.cols[col].barrier = false;
    g.cols[col].cells[q] = cell;
    return g;
  });
  const remove = (col: number, q: number) => edit((g) => {
    if (g.cols[col]?.barrier) g.cols[col].barrier = false;
    else if (g.cols[col]) g.cols[col].cells[q] = null;
    return g;
  });
  const patchCell = (col: number, q: number, patch: Partial<Cell>) => edit((g) => {
    const c = g.cols[col]?.cells[q];
    if (c) g.cols[col].cells[q] = { ...c, ...patch };
    return g;
  });

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement;
      if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT')) return;
      if (sel && (e.key === 'Delete' || e.key === 'Backspace')) {
        remove(sel.col, sel.q);
        setSel(null);
      }
      if (e.key === 'Escape') {
        setSel(null);
        setArmed(null);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sel]);

  const onDrop = (e: DragEvent, col: number, q: number) => {
    e.preventDefault();
    const gid = e.dataTransfer.getData('text/gate');
    const mv = e.dataTransfer.getData('text/move');
    if (gid) place(gid, col, q);
    else if (mv) {
      const [c0, q0] = mv.split(',').map(Number);
      move({ col: c0, q: q0 }, col, q);
      setSel({ col, q });
    }
  };

  const loadPreset = (id: string) => {
    setPreset(id);
    if (!id) return;
    if (id === '__empty') {
      setGrid(emptyGrid(3, 8));
      setSel(null);
      return;
    }
    const r = tryCall<Entry>('quantum.algorithm', { id, params: {} });
    if (r.data) {
      const g = circuitToGrid(r.data.circuit);
      g.cols.push({ cells: Array(g.qubits).fill(null) });
      setGrid(g);
      setSel(null);
      setNotice(r.data.explanation);
    }
  };

  const importQasm = () => {
    const r = tryCall<ParseOut>('quantum.qasm_parse', { qasm: qasmIn });
    if (r.error) {
      setImportErr(r.error.replace(/^quantum\.qasm_parse:\s*/, ''));
      return;
    }
    if (r.data) {
      if (r.data.circuit.qubits > 14) {
        setImportErr(`${r.data.circuit.qubits} qubits is more than the simulator's 14`);
        return;
      }
      const g = circuitToGrid(r.data.circuit);
      g.cols.push({ cells: Array(g.qubits).fill(null) });
      setGrid(g);
      setImportErr(null);
      setNotice(`Imported ${r.data.ops} operations on ${r.data.circuit.qubits} qubits${r.data.warnings.length ? ' — ' + r.data.warnings.join('; ') : ''}.`);
    }
  };

  const selCell = sel ? grid.cols[sel.col]?.cells[sel.q] ?? null : null;
  const selInfo = selCell ? info(selCell.g) : undefined;
  const CELL = 44;

  // Bloch vectors (interpolated between steps while playing)
  const blochs: V3[] = step
    ? step.qubits.map((qi, k) => (next && player.frac > 0 ? slerp(qi.bloch as V3, next.qubits[k].bloch as V3, easeInOut(player.frac)) : (qi.bloch as V3)))
    : [];
  const events = step?.events ?? [];

  const ampCsv = () => {
    if (!step) return '';
    return csv(['basis', 're', 'im', 'probability', 'phase_deg'], step.labels.map((l, k) => [l, step.re[k], step.im[k], step.prob[k], ((Math.atan2(step.im[k], step.re[k]) * 180) / Math.PI).toFixed(2)]));
  };

  return (
    <div className="stack">
      <Card
        title="Circuit composer"
        subtitle="Drag a gate onto a wire — or tap a gate, then tap a cell. Control dots (•) control every gate in their column. Drag a placed gate to move it; select it and press Delete to remove it."
        actions={
          <div className="row" style={{ gap: 6 }}>
            <Select
              value={preset}
              onChange={loadPreset}
              options={[{ value: '', label: 'Load a preset…' }, { value: '__empty', label: 'Empty (3 qubits)' }, ...(lib.data ?? []).map((l) => ({ value: l.id, label: `${l.group}: ${l.title}` }))]}
            />
          </div>
        }
      >
        <div className="qx-palette" role="toolbar" aria-label="gate palette">
          {PALETTE.map((grp) => (
            <div key={grp.group} className="qx-pgroup">
              <div className="qx-pgroup-title">{grp.group}</div>
              <div className="qx-ptiles">
                {grp.ids.map((id) => (
                  <button
                    key={id}
                    className={`qx-tile ${armed === id ? 'armed' : ''}`}
                    draggable
                    onDragStart={(e) => { e.dataTransfer.setData('text/gate', id); e.dataTransfer.effectAllowed = 'copy'; }}
                    onClick={() => setArmed((a) => (a === id ? null : id))}
                    title={PATTERNS[id] ? PATTERNS[id].tip : id === 'ctrl' ? 'Control dot: the gates in this column act only when this qubit is |1⟩' : id === 'barrier' ? 'Barrier: a visual separator (no effect on the state)' : `${info(id)?.name ?? id} — ${info(id)?.description ?? ''}`}
                    aria-pressed={armed === id}
                  >
                    {PATTERNS[id] ? <PatternGlyph id={id} /> : id === 'barrier' ? <span className="qx-barrier-tile">┆</span> : <GateGlyph cell={{ g: id, p: DEFAULT_P[id] }} small />}
                  </button>
                ))}
              </div>
            </div>
          ))}
        </div>
        {armed && <div className="small" style={{ margin: '6px 0' }}><Chip tone="info">placing {PATTERNS[armed] ? PATTERNS[armed].name : armed === 'ctrl' ? 'control •' : LABEL[armed] ?? armed}</Chip> <span className="muted">tap cells to place · tap the tile again or press Esc to stop</span></div>}

        <div className="qx-gridwrap">
          <div className="qx-grid" style={{ ['--cell' as string]: `${CELL}px` }}>
            <div className="qx-wirelabels">
              {Array.from({ length: grid.qubits }, (_, q) => (
                <div key={q} className="qx-wlabel mono">q{q}<span className="muted">|0⟩</span></div>
              ))}
              <div className="qx-wlabel mono muted">c{grid.clbits}</div>
            </div>
            <div className="qx-cols">
              {grid.cols.map((col, ci) => {
                const filled = col.cells.map((c, q) => (c ? q : -1)).filter((q) => q >= 0);
                const multi = !col.barrier && filled.length > 1 && col.cells.some((c) => c?.g === 'ctrl' || c?.g === 'swap' || c?.g === 'iswap');
                const lo = Math.min(...filled), hi = Math.max(...filled);
                const meas = col.cells.map((c, q) => (c?.g === 'measure' ? q : -1)).filter((q) => q >= 0);
                return (
                  <div key={ci} className={`qx-col ${activeCol === ci ? 'active' : ''} ${sel?.col === ci ? 'selcol' : ''} ${col.barrier ? 'barrier' : ''}`}>
                    {multi && <div className="qx-vline" style={{ top: `calc(${lo + 0.5} * var(--cell))`, height: `calc(${hi - lo} * var(--cell))` }} />}
                    {meas.map((q) => <div key={q} className="qx-mline" style={{ top: `calc(${q + 0.5} * var(--cell) + 12px)`, height: `calc(${grid.qubits - q - 0.5} * var(--cell) - 6px)` }} />)}
                    {Array.from({ length: grid.qubits }, (_, q) => {
                      const cell = col.cells[q];
                      const isSel = sel?.col === ci && sel?.q === q;
                      return (
                        <div
                          key={q}
                          className={`qx-cell ${isSel ? 'sel' : ''} ${cell ? 'filled' : ''}`}
                          onDragOver={(e) => e.preventDefault()}
                          onDrop={(e) => onDrop(e, ci, q)}
                          onClick={() => {
                            if (armed) place(armed, ci, q);
                            else setSel(cell || col.barrier ? { col: ci, q } : null);
                          }}
                          role="button"
                          tabIndex={0}
                          aria-label={`column ${ci + 1}, qubit ${q}${cell ? `: ${cellTitle(cell)}` : ' (empty)'}`}
                          onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); if (armed) place(armed, ci, q); else setSel({ col: ci, q }); } }}
                        >
                          <div className="qx-wire" />
                          {cell && (
                            <div
                              className="qx-cellin"
                              draggable
                              onDragStart={(e) => { e.dataTransfer.setData('text/move', `${ci},${q}`); e.dataTransfer.effectAllowed = 'move'; }}
                              title={cellTitle(cell)}
                            >
                              <GateGlyph cell={cell} />
                            </div>
                          )}
                        </div>
                      );
                    })}
                    <div className="qx-cwire" />
                    <div className="qx-colnum small muted">{ci + 1}</div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>

        <div className="row" style={{ marginTop: 8 }}>
          <button className="btn small" onClick={() => edit((g) => withQubits(g, g.qubits + 1))} disabled={grid.qubits >= 14}>+ qubit</button>
          <button className="btn small" onClick={() => edit((g) => withQubits(g, g.qubits - 1))} disabled={grid.qubits <= 1}>− qubit</button>
          <button className="btn small" onClick={() => edit((g) => { const at = sel ? sel.col : g.cols.length; g.cols.splice(at, 0, { cells: Array(g.qubits).fill(null) }); return g; })}>+ column{sel ? ' before selected' : ''}</button>
          <button className="btn small" disabled={!sel} onClick={() => { if (sel) { edit((g) => { g.cols.splice(sel.col, 1); return g; }); setSel(null); } }}>− selected column</button>
          <button className="btn small" onClick={() => edit((g) => ({ ...g, clbits: Math.min(16, g.clbits + 1) }))}>+ classical bit</button>
          <button className="btn small" onClick={() => { setGrid(emptyGrid(grid.qubits, 8)); setSel(null); }}><Icon name="reset" size={12} /> Clear</button>
          <span className="small muted">{grid.qubits} qubits · {grid.clbits} classical bits · {circuit.cols.filter((c) => c.length).length} columns</span>
        </div>
        {notice && <div className="callout small" style={{ marginTop: 10 }}>{notice} <button className="btn ghost small" onClick={() => setNotice(null)} aria-label="dismiss">✕</button></div>}
        {errors.length > 0 && <ErrorBox error={errors.join(' · ')} />}
        {run.error && ok && <ErrorBox error={run.error.replace(/^quantum\.run:\s*/, '')} />}

        {selCell && sel && (
          <div className="qx-inspector">
            <div className="row" style={{ justifyContent: 'space-between' }}>
              <div>
                <b>{selCell.g === 'ctrl' ? 'Control dot' : selInfo?.name ?? selCell.g}</b>
                <span className="muted small"> · column {sel.col + 1}, q{sel.q}</span>
              </div>
              <button className="btn small" onClick={() => { remove(sel.col, sel.q); setSel(null); }}>Delete</button>
            </div>
            <div className="small muted" style={{ margin: '4px 0 8px' }}>{selCell.g === 'ctrl' ? 'Every gate in this column acts only on the part of the state where this qubit is |1⟩ (CNOT, CZ, Toffoli, Fredkin, controlled rotations…).' : selInfo?.description}</div>
            {PARAMS[selCell.g]?.map((pn, k) => (
              <div key={pn}>
                <Slider label={pn} value={selCell.p?.[k] ?? 0} min={-2 * Math.PI} max={2 * Math.PI} step={Math.PI / 64} format={(v) => fmtAngle(v)} onChange={(v) => patchCell(sel.col, sel.q, { p: (selCell.p ?? PARAMS[selCell.g].map(() => 0)).map((x, j) => (j === k ? v : x)) })} />
                <div className="row" style={{ gap: 4, marginTop: -6, marginBottom: 8 }}>
                  {[Math.PI / 8, Math.PI / 4, Math.PI / 2, Math.PI, -Math.PI / 2].map((v) => (
                    <button key={v} className="btn small ghost" onClick={() => patchCell(sel.col, sel.q, { p: (selCell.p ?? PARAMS[selCell.g].map(() => 0)).map((x, j) => (j === k ? v : x)) })}>{fmtAngle(v)}</button>
                  ))}
                </div>
              </div>
            ))}
            {selCell.g === 'measure' && (
              <Select label="Writes classical bit" value={String(selCell.c ?? sel.q)} options={Array.from({ length: grid.clbits }, (_, k) => ({ value: String(k), label: `c[${k}]` }))} onChange={(v) => patchCell(sel.col, sel.q, { c: Number(v) })} />
            )}
            {selCell.g !== 'ctrl' && selCell.g !== 'measure' && selCell.g !== 'reset' && (
              <Select
                label="Classical condition"
                value={selCell.cond ? `${selCell.cond.bits.join('+')}:${selCell.cond.value}` : ''}
                options={[{ value: '', label: 'always' }, ...Array.from({ length: grid.clbits }, (_, k) => ({ value: `${k}:1`, label: `only if c[${k}] = 1` })), ...(selCell.cond && selCell.cond.bits.length > 1 ? [{ value: `${selCell.cond.bits.join('+')}:${selCell.cond.value}`, label: `if register = ${selCell.cond.value}` }] : [])]}
                onChange={(v) => patchCell(sel.col, sel.q, { cond: v ? { bits: v.split(':')[0].split('+').map(Number), value: Number(v.split(':')[1]) } : null })}
              />
            )}
          </div>
        )}
      </Card>

      <div className="grid grid-2">
        <Card
          title="Step through the state"
          subtitle={step ? (step.column === 0 ? 'Initial state |0…0⟩' : `After column ${step.column}${res ? ` of ${res.columns}` : ''}`) : 'Starting the engine…'}
          actions={step && <DownloadButton small filename="amplitudes.csv" content={ampCsv} label="amplitudes.csv" />}
        >
          {res && <PlayBar p={player} label={(i) => (i === 0 ? 'start' : `column ${i}`)} />}
          {step && (
            <>
              <div className="row" style={{ alignItems: 'flex-start', gap: 12, marginTop: 8 }}>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <AmplitudeBars labels={step.labels} re={step.re} im={step.im} />
                </div>
                <div className="qx-wheel-legend small muted"><PhaseWheel />phase</div>
              </div>
              <div className="small muted" style={{ marginTop: 6 }}>
                Bars: probability |a|² of each basis state (q{grid.qubits - 1}…q0, Qiskit order). Discs: amplitude magnitude; colour and needle: phase.
                {res && res.qubits > 8 && ' Showing the 64 most probable basis states.'}
              </div>
              {events.length > 0 && (
                <div className="row small" style={{ marginTop: 6 }}>
                  {events.map((e, k) => <Chip key={k} tone="info">measured q{e.qubit} → {e.outcome}{e.clbit != null ? ` into c[${e.clbit}]` : ''} (p = {e.probability.toFixed(3)})</Chip>)}
                </div>
              )}
            </>
          )}
        </Card>
        <Card title="Bloch spheres" subtitle="One per qubit: the arrow is its reduced state. Pure: on the surface. Entangled or measured into a mixture: shorter, down to the centre. Drag to rotate.">
          <div className="qx-blochs">
            {blochs.slice(0, 8).map((v, k) => (
              <BlochSphere key={k} v={v} size={grid.qubits > 4 ? 124 : 150} label={`q${k}`} sub={step ? `|r| = ${step.qubits[k].radius.toFixed(2)} · S = ${step.qubits[k].entropy.toFixed(2)} bit` : undefined} />
            ))}
          </div>
          {grid.qubits > 8 && <div className="small muted">Showing q0–q7.</div>}
          {step && step.concurrence && grid.qubits >= 2 && (
            <div style={{ marginTop: 10 }}>
              <div className="small" style={{ fontWeight: 600, marginBottom: 4 }}>Pairwise entanglement (Wootters concurrence)</div>
              <ConcurrenceMatrix n={grid.qubits} m={step.concurrence} />
            </div>
          )}
        </Card>
      </div>

      <div className="grid grid-2">
        <Card
          title="Measurement shots"
          subtitle={res ? `${res.shots} shots over ${res.histogram_over}; ${res.sampling}.` : ''}
          actions={res && <DownloadButton small filename="counts.csv" content={() => csv(['bits', 'count', 'probability'], res.histogram.map((h) => [h.bits, h.count, h.prob]))} label="counts.csv" />}
        >
          <div className="row" style={{ gap: 12 }}>
            <div style={{ width: 150 }}><Select label="Shots" value={String(shots)} options={[64, 256, 1024, 4096, 16384].map((s) => ({ value: String(s), label: String(s) }))} onChange={(v) => setShots(Number(v))} /></div>
            <button className="btn small" onClick={() => setSeed((s) => s + 1)} style={{ marginTop: 8 }}>Re-sample (seed {seed})</button>
          </div>
          {res && <Histogram entries={res.histogram} exact={res.exact} />}
          <div className="small muted">Bars: sampled frequencies. Ticks: exact Born-rule probabilities (when all measurements are at the end). The spread between them shrinks as 1/√shots.</div>
          {res?.warnings.map((w) => <div key={w} className="small" style={{ color: 'var(--warn)' }}>! {w}</div>)}
        </Card>
        <Card title="Export and open elsewhere" subtitle="The same circuit in real tools: Open Quantum Assembly Language (OpenQASM) 2.0 or 3.0, a Qiskit script, a live Quirk link, or the IBM Quantum Composer.">
          {exp.data?.quirk_url && (
            <div className="row" style={{ marginBottom: 10 }}>
              <a className="btn primary" href={exp.data.quirk_url} target="_blank" rel="noreferrer">Open in Quirk ↗</a>
              <button className="btn small" onClick={() => navigator.clipboard?.writeText(exp.data!.quirk_url ?? '').catch(() => {})}>Copy link</button>
              <button className="btn small" onClick={() => navigator.clipboard?.writeText(exp.data!.quirk_json ?? '').catch(() => {})}>Copy Quirk JSON</button>
            </div>
          )}
          <Seg value={fmt} onChange={setFmt} options={[{ value: 'qasm2', label: 'OpenQASM 2.0' }, { value: 'qasm3', label: 'OpenQASM 3.0' }, { value: 'qiskit', label: 'Qiskit (Python)' }]} />
          {fmt === 'qasm2' && exp.data?.qasm && <Code code={exp.data.qasm} filename="circuit.qasm" maxHeight={220} />}
          {fmt === 'qasm2' && exp.data?.qasm_error && <ErrorBox error={`OpenQASM 2.0: ${exp.data.qasm_error} — the 3.0 and Qiskit exports handle it.`} />}
          {fmt === 'qasm3' && exp.data?.qasm3 && <Code code={exp.data.qasm3} filename="circuit.qasm3" maxHeight={220} />}
          {fmt === 'qasm3' && exp.data?.qasm3_error && <ErrorBox error={`OpenQASM 3.0: ${exp.data.qasm3_error}`} />}
          {fmt === 'qiskit' && exp.data?.qiskit && <Code code={exp.data.qiskit} filename="circuit.py" maxHeight={220} />}
          {exp.data?.warnings.map((w) => <div key={w} className="small" style={{ color: 'var(--warn)' }}>! {w}</div>)}
          <details style={{ marginTop: 10 }} open={fmt !== 'qiskit'}>
            <summary className="small" style={{ cursor: 'pointer', fontWeight: 600 }}>Run it in the IBM Quantum Composer</summary>
            <ol className="small qx-howto">
              <li>Sign in at IBM Quantum (a free account) and open the <b>Composer</b>; start a new, empty circuit.</li>
              <li>Open the code panel (the <b>OpenQASM</b> view beside the circuit) and choose OpenQASM 2.0 or 3.0 to match the program you copy here.</li>
              <li>Replace the code with this program. The grid redraws with the same gates; the Composer's probability and Q-sphere views should match the bars and histogram on this page.</li>
              <li>To run on hardware, <b>Set up and run</b> with a real backend: expect the noisy counts that the <b>Noise</b> tab predicts rather than the ideal ones.</li>
            </ol>
            <div className="small muted">Menu names change between releases — check IBM's Composer documentation if a step moved. Qiskit: <code>QuantumCircuit.from_qasm_str(text)</code> for 2.0, <code>qiskit.qasm3.loads(text)</code> (with the <code>qiskit-qasm3-import</code> package) for 3.0, or run <code>circuit.py</code> directly.</div>
          </details>
          <details style={{ marginTop: 10 }}>
            <summary className="small" style={{ cursor: 'pointer' }}>Import OpenQASM 2.0</summary>
            <textarea rows={7} value={qasmIn} onChange={(e) => setQasmIn(e.target.value)} placeholder={'OPENQASM 2.0;\ninclude "qelib1.inc";\nqreg q[2];\ncreg c[2];\nh q[0];\ncx q[0],q[1];\nmeasure q -> c;'} aria-label="OpenQASM input" />
            <div className="row" style={{ marginTop: 6 }}>
              <button className="btn small primary" onClick={importQasm}>Load into the composer</button>
              {exp.data?.qasm && <button className="btn small" onClick={() => setQasmIn(exp.data!.qasm!)}>Start from the current circuit</button>}
            </div>
            {importErr && <ErrorBox error={importErr} />}
          </details>
        </Card>
      </div>

      <div className="grid grid-2">
        <Explainer title="What the simulator does" refs={[{ label: 'Nielsen & Chuang (2010), ch. 4' }, { label: 'Quirk', href: 'https://algassert.com/quirk' }, { label: 'OpenQASM 2.0 (Cross et al., arXiv:1707.03429)', href: 'https://arxiv.org/abs/1707.03429' }]}>
          The state of n qubits is 2ⁿ complex amplitudes. A single-qubit gate U on qubit t mixes every pair of amplitudes whose indices differ only in bit t:
          <Formula>{`a'[i] = U00·a[i] + U01·a[j]      a'[j] = U10·a[i] + U11·a[j]      (j = i | 2^t)`}</Formula>
          Controls simply skip the pairs where a control bit is 0. Measuring qubit t gives 1 with probability P = Σ|a[i]|² over indices with bit t set, then keeps only that half and renormalises (the collapse). Up to 14 qubits (16 384 amplitudes) run live in your browser.
        </Explainer>
        <Explainer title="Reading the Bloch spheres" refs={[{ label: 'Preskill, Ph219 notes ch. 2', href: 'http://theory.caltech.edu/~preskill/ph219/' }]}>
          Each qubit's reduced density matrix ρ = ½(I + xX + yY + zZ) is drawn as the vector (x, y, z). A lone qubit in a pure state sits on the surface. After a CNOT makes a Bell pair, each qubit's vector shrinks to the centre — the information now lives only in the correlations, measured here by the entropy S = h((1 + |r|)/2) and the pairwise concurrence.
        </Explainer>
      </div>
    </div>
  );
}

function easeInOut(t: number) {
  return t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2;
}

function ConcurrenceMatrix({ n, m }: { n: number; m: number[] }) {
  return (
    <div className="qx-cmat" style={{ gridTemplateColumns: `28px repeat(${n}, 30px)` }}>
      <span />
      {Array.from({ length: n }, (_, j) => <span key={j} className="mono small muted" style={{ textAlign: 'center' }}>q{j}</span>)}
      {Array.from({ length: n }, (_, i) => (
        <div key={i} style={{ display: 'contents' }}>
          <span className="mono small muted">q{i}</span>
          {Array.from({ length: n }, (_, j) => {
            const v = i === j ? null : m[i * n + j];
            return (
              <span key={j} className="qx-cm" title={v == null ? '' : `C(q${i}, q${j}) = ${v.toFixed(3)}`} style={{ background: v == null ? 'transparent' : `color-mix(in srgb, var(--series-5) ${Math.round(v * 85)}%, var(--surface-2))` }}>
                {v == null ? '' : v > 0.005 ? v.toFixed(2) : '·'}
              </span>
            );
          })}
        </div>
      ))}
    </div>
  );
}
