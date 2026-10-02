// Activity-on-node (AON) network diagram with an animated critical path
// method: the forward pass fills early start / finish node by node, the
// backward pass fills late start / finish in reverse, then the zero-float
// chain lights up. Each box is the textbook layout:
//   ES | duration | EF
//   id  name
//   LS | total float | LF
import { useEffect, useMemo, useRef } from 'react';
import type { TaskRow } from './types';
import { CRIT, num, phaseColor } from './util';

const NW = 116;
const NH = 60;
const GX = 34;
const GY = 12;

export interface Layout { pos: Map<string, { x: number; y: number }>; width: number; height: number; order: string[] }

/** Columns by topological level; rows by the barycentre of predecessors (fewer crossings). */
export function layoutNetwork(rows: TaskRow[]): Layout {
  const byLevel = new Map<number, TaskRow[]>();
  for (const r of rows) byLevel.set(r.level, [...(byLevel.get(r.level) ?? []), r]);
  const levels = [...byLevel.keys()].sort((a, b) => a - b);
  const rowOf = new Map<string, number>();
  let maxRows = 0;
  for (const lv of levels) {
    const items = byLevel.get(lv)!;
    const bary = (r: TaskRow) => (r.preds.length ? r.preds.reduce((s, [p]) => s + (rowOf.get(p) ?? 0), 0) / r.preds.length : 0);
    items.sort((a, b) => bary(a) - bary(b) || (b.critical ? 1 : 0) - (a.critical ? 1 : 0));
    // keep the critical node near its predecessors' row when possible
    items.forEach((r, i) => rowOf.set(r.id, i));
    maxRows = Math.max(maxRows, items.length);
  }
  const pos = new Map<string, { x: number; y: number }>();
  for (const lv of levels) {
    const items = byLevel.get(lv)!;
    const off = (maxRows - items.length) / 2;
    items.forEach((r, i) => pos.set(r.id, { x: 8 + lv * (NW + GX), y: 8 + (i + off) * (NH + GY) }));
  }
  const order = levels.flatMap((lv) => byLevel.get(lv)!.map((r) => r.id));
  return { pos, width: 16 + levels.length * (NW + GX) - GX, height: 16 + maxRows * (NH + GY) - GY, order };
}

/** Number of animation steps: forward (n) + backward (n) + highlight (1). */
export const networkSteps = (n: number) => 2 * n + 1;

export function Network({ rows, step, follow, onSelect, selected, caption }: {
  rows: TaskRow[];
  /** Animation step; ≥ 2n + 1 shows everything. */
  step: number;
  follow: boolean;
  onSelect?: (id: string | null) => void;
  selected?: string | null;
  caption?: boolean;
}) {
  const lay = useMemo(() => layoutNetwork(rows), [rows]);
  const n = rows.length;
  const byId = useMemo(() => new Map(rows.map((r) => [r.id, r])), [rows]);
  const fwdIdx = new Map(lay.order.map((id, i) => [id, i]));
  const s = Math.floor(step);
  const shownF = (id: string) => (fwdIdx.get(id) ?? 0) < Math.min(s, n);
  const shownB = (id: string) => n - 1 - (fwdIdx.get(id) ?? 0) < Math.max(0, s - n);
  const highlight = s >= 2 * n;
  const current = s < n ? lay.order[s] : s < 2 * n ? lay.order[2 * n - 1 - s] : null;
  const phase = s < n ? 'forward' : s < 2 * n ? 'backward' : 'done';
  const wrap = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!follow || !current || !wrap.current) return;
    const p = lay.pos.get(current);
    if (!p) return;
    const el = wrap.current;
    const target = p.x + NW / 2 - el.clientWidth / 2;
    el.scrollLeft += (target - el.scrollLeft) * 0.35;
  }, [current, follow, lay]);

  const cur = current ? byId.get(current) : null;
  let explain = '';
  if (cur && phase === 'forward') {
    explain = cur.preds.length
      ? `Forward pass · ES(${cur.id}) = max(${cur.preds.map(([p, lag]) => `EF(${p})${lag ? (lag > 0 ? '+' : '') + lag : ''}`).join(', ')}) = ${num(cur.es, 1)}; EF = ES + ${num(cur.duration, 1)} = ${num(cur.ef, 1)}`
      : `Forward pass · ${cur.id} has no predecessors: ES = 0, EF = ${num(cur.ef, 1)}`;
  } else if (cur && phase === 'backward') {
    explain = cur.succs.length
      ? `Backward pass · LF(${cur.id}) = min(${cur.succs.map((x) => `LS(${x})`).join(', ')}) = ${num(cur.lf, 1)}; LS = LF − ${num(cur.duration, 1)} = ${num(cur.ls, 1)}; total float = LS − ES = ${num(cur.total_float, 1)}`
      : `Backward pass · ${cur.id} ends the project: LF = ${num(cur.lf, 1)}, LS = ${num(cur.ls, 1)}`;
  } else if (phase === 'done') {
    explain = `Critical path: every activity with zero total float — ${rows.filter((r) => r.critical).length} of ${n}. Delay any of them and the finish moves.`;
  }

  return (
    <div>
      {caption !== false && <div className="pg-net-caption small mono" aria-live="polite">{explain || ' '}</div>}
      <div className="pg-net" ref={wrap}>
        <svg width={lay.width} height={lay.height} role="img" aria-label="Activity-on-node network diagram">
          <defs>
            <marker id="pg-net-a" viewBox="0 0 6 6" refX="5.5" refY="3" markerWidth="6" markerHeight="6" orient="auto">
              <path d="M0,0 L6,3 L0,6 z" fill="var(--muted)" />
            </marker>
            <marker id="pg-net-c" viewBox="0 0 6 6" refX="5.5" refY="3" markerWidth="6" markerHeight="6" orient="auto">
              <path d="M0,0 L6,3 L0,6 z" fill={CRIT} />
            </marker>
          </defs>
          {rows.flatMap((r) => r.preds.map(([p, lag]) => {
            const a = lay.pos.get(p), b = lay.pos.get(r.id);
            const pr = byId.get(p);
            if (!a || !b || !pr) return null;
            const x1 = a.x + NW, y1 = a.y + NH / 2, x2 = b.x - 2, y2 = b.y + NH / 2;
            const mx = (x1 + x2) / 2;
            const crit = highlight && pr.critical && r.critical && Math.abs(pr.ef + lag - r.es) < 1e-6;
            const active = (phase === 'forward' && current === r.id) || (phase === 'backward' && current === p);
            return (
              <path key={`${p}>${r.id}`} d={`M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}`} fill="none"
                stroke={crit ? CRIT : active ? 'var(--accent)' : 'var(--border-strong)'} strokeWidth={crit ? 2.2 : active ? 2 : 1.1}
                markerEnd={crit ? 'url(#pg-net-c)' : 'url(#pg-net-a)'} />
            );
          }))}
          {rows.map((r) => {
            const p = lay.pos.get(r.id)!;
            const f = shownF(r.id), b = shownB(r.id);
            const crit = highlight && r.critical;
            const isCur = current === r.id;
            const sel = selected === r.id;
            return (
              <g key={r.id} transform={`translate(${p.x} ${p.y})`} onClick={() => onSelect?.(sel ? null : r.id)} style={{ cursor: onSelect ? 'pointer' : undefined }}>
                <title>{`${r.id} — ${r.name}`}</title>
                <rect width={NW} height={NH} rx={r.milestone ? 14 : 6} fill={crit ? 'color-mix(in srgb, var(--series-2) 16%, var(--surface-1))' : 'var(--surface-1)'}
                  stroke={isCur ? 'var(--accent)' : crit ? CRIT : sel ? 'var(--text)' : 'var(--border-strong)'} strokeWidth={isCur || crit ? 2.2 : 1} />
                <rect x={0} y={0} width={4} height={NH} rx={2} fill={phaseColor(r.phase)} />
                <line x1={0} x2={NW} y1={18} y2={18} stroke="var(--border)" />
                <line x1={0} x2={NW} y1={NH - 18} y2={NH - 18} stroke="var(--border)" />
                <line x1={NW / 3} x2={NW / 3} y1={0} y2={18} stroke="var(--border)" />
                <line x1={(2 * NW) / 3} x2={(2 * NW) / 3} y1={0} y2={18} stroke="var(--border)" />
                <line x1={NW / 3} x2={NW / 3} y1={NH - 18} y2={NH} stroke="var(--border)" />
                <line x1={(2 * NW) / 3} x2={(2 * NW) / 3} y1={NH - 18} y2={NH} stroke="var(--border)" />
                <g fontSize={10.5} className="mono" textAnchor="middle">
                  <text x={NW / 6} y={13} fill={f ? 'var(--text)' : 'var(--muted)'}>{f ? num(r.es, 1) : 'ES'}</text>
                  <text x={NW / 2} y={13} fill="var(--text-2)">{r.milestone ? '◆' : num(r.duration, 1)}</text>
                  <text x={(5 * NW) / 6} y={13} fill={f ? 'var(--text)' : 'var(--muted)'}>{f ? num(r.ef, 1) : 'EF'}</text>
                  <text x={NW / 6} y={NH - 5} fill={b ? 'var(--text)' : 'var(--muted)'}>{b ? num(r.ls, 1) : 'LS'}</text>
                  <text x={NW / 2} y={NH - 5} fill={b ? (r.total_float < 1e-6 ? CRIT : 'var(--good-text)') : 'var(--muted)'} fontWeight={b ? 700 : 400}>{b ? num(r.total_float, 1) : 'TF'}</text>
                  <text x={(5 * NW) / 6} y={NH - 5} fill={b ? 'var(--text)' : 'var(--muted)'}>{b ? num(r.lf, 1) : 'LF'}</text>
                </g>
                <text x={9} y={NH / 2 + 4} fontSize={10.5} fill="var(--text)">
                  <tspan fontWeight={700}>{r.id}</tspan>
                  <tspan fill="var(--text-2)"> {r.name.length > 12 ? r.name.slice(0, 11) + '…' : r.name}</tspan>
                </text>
              </g>
            );
          })}
        </svg>
      </div>
    </div>
  );
}
