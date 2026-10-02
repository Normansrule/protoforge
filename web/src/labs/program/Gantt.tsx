// Animated Gantt chart: phase summary bars, task bars growing as a time
// cursor sweeps the plan, total float as a hatched tail, milestones as
// diamonds and the critical path highlighted (colour + ◆ marker + label).
import { useMemo } from 'react';
import { useElementWidth } from '../../components/useElementWidth';
import type { TaskRow } from './types';
import { addWeeks, CRIT, phaseColor, shortDate } from './util';

interface Props {
  rows: TaskRow[];
  finish: number;
  start: string;
  /** Animation cursor (weeks); null = draw everything. */
  cursor: number | null;
  selected: string | null;
  onSelect: (id: string | null) => void;
  showFloat: boolean;
  showLinks: boolean;
  /** Tasks to emphasise (e.g. critical in a Monte Carlo sample). */
  emphasis?: Set<string>;
}

const RH = 21;

export function Gantt({ rows, finish, start, cursor, selected, onSelect, showFloat, showLinks }: Props) {
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const narrow = width < 620;
  const L = narrow ? 96 : 250;
  const W = Math.max(width, 320);
  const cw = W - L - 12;
  const end = Math.max(1, Math.ceil(finish + 1));
  const x = (w: number) => L + (w / end) * cw;

  // interleave phase header rows
  const lines = useMemo(() => {
    const out: ({ kind: 'phase'; phase: string; es: number; ef: number; crit: boolean } | { kind: 'task'; r: TaskRow })[] = [];
    let cur = '';
    for (const r of rows) {
      if (r.phase !== cur) {
        cur = r.phase;
        const ph = rows.filter((t) => t.phase === cur);
        out.push({ kind: 'phase', phase: cur, es: Math.min(...ph.map((t) => t.es)), ef: Math.max(...ph.map((t) => t.ef)), crit: ph.some((t) => t.critical) });
      }
      out.push({ kind: 'task', r });
    }
    return out;
  }, [rows]);
  const H = 28 + lines.length * RH + 8;
  const rowY = new Map<string, number>();
  lines.forEach((l, i) => { if (l.kind === 'task') rowY.set(l.r.id, 28 + i * RH + RH / 2); });

  const ticks: number[] = [];
  const stepW = end / Math.max(2, Math.floor(cw / (narrow ? 58 : 74)));
  const nice = stepW <= 2 ? 2 : stepW <= 4 ? 4 : stepW <= 8 ? 8 : stepW <= 13 ? 13 : 26;
  for (let w = 0; w <= end; w += nice) ticks.push(w);
  const c = cursor ?? Infinity;
  const grown = (r: TaskRow) => Math.max(0, Math.min(r.duration, c - r.es));

  const links = showLinks
    ? rows.flatMap((r) => r.preds.map(([p, lag]) => {
        const pr = rows.find((t) => t.id === p);
        if (!pr) return null;
        const y1 = rowY.get(p)!, y2 = rowY.get(r.id)!;
        const x1 = x(pr.ef), x2 = x(r.es);
        const crit = pr.critical && r.critical && Math.abs(pr.ef + lag - r.es) < 1e-6;
        const visible = c >= r.es;
        return { key: `${p}-${r.id}`, d: `M${x1},${y1} H${Math.max(x1 + 4, Math.min(x2 - 4, x1 + 6))} V${y2} H${x2 - 2}`, crit, visible, hot: selected === r.id || selected === p };
      })).filter(Boolean) as { key: string; d: string; crit: boolean; visible: boolean; hot: boolean }[]
    : [];

  return (
    <div ref={ref} className="pg-gantt">
      {width > 0 && (
        <svg width={W} height={H} role="img" aria-label="Gantt chart of the program schedule with the critical path highlighted">
          <defs>
            <pattern id="pg-hatch" width="5" height="5" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
              <line x1="0" y1="0" x2="0" y2="5" stroke="var(--muted)" strokeWidth="1.2" />
            </pattern>
            <clipPath id="pg-label-clip"><rect x={0} y={0} width={L - 8} height={H} /></clipPath>
            <marker id="pg-arrow" viewBox="0 0 6 6" refX="5" refY="3" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
              <path d="M0,0 L6,3 L0,6 z" fill="var(--muted)" />
            </marker>
            <marker id="pg-arrow-c" viewBox="0 0 6 6" refX="5" refY="3" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
              <path d="M0,0 L6,3 L0,6 z" fill={CRIT} />
            </marker>
          </defs>
          {ticks.map((w) => (
            <g key={w}>
              <line x1={x(w)} x2={x(w)} y1={22} y2={H - 6} stroke="var(--grid)" />
              <text x={x(w)} y={11} fontSize={10} textAnchor="middle" fill="var(--muted)" className="mono">{`wk ${w}`}</text>
              <text x={x(w)} y={21} fontSize={9.5} textAnchor="middle" fill="var(--muted)">{shortDate(addWeeks(start, w))}</text>
            </g>
          ))}
          {lines.map((l, i) => {
            const y = 28 + i * RH;
            if (l.kind === 'phase') {
              const col = phaseColor(l.phase);
              const f = cursor === null ? 1 : Math.max(0, Math.min(1, (c - l.es) / Math.max(1e-6, l.ef - l.es)));
              return (
                <g key={`ph-${l.phase}`}>
                  <rect x={0} y={y} width={W} height={RH} fill="var(--surface-2)" opacity={0.7} />
                  <text x={6} y={y + 14.5} fontSize={11.5} fontWeight={700} fill="var(--text)">{l.phase}</text>
                  <rect x={x(l.es)} y={y + 7} width={Math.max(0, (x(l.ef) - x(l.es)) * f)} height={7} rx={2} fill={col} opacity={f > 0 ? 0.85 : 0} />
                  <path d={`M${x(l.es)},${y + 14} l0,4 l4,-4 M${x(l.ef)},${y + 14} l0,4 l-4,-4`} stroke={col} fill="none" opacity={f >= 1 ? 1 : 0} />
                </g>
              );
            }
            const r = l.r;
            const col = r.critical ? CRIT : phaseColor(r.phase);
            const g = grown(r);
            const sel = selected === r.id;
            const label = narrow ? r.id : `${r.id}  ${r.name}`;
            return (
              <g key={r.id} className="pg-gantt-row" onClick={() => onSelect(sel ? null : r.id)} style={{ cursor: 'pointer' }}>
                <title>{`${r.id} — ${r.name}\n${r.milestone ? 'milestone' : `${r.duration} wk`} · ES ${r.es} · EF ${r.ef} · total float ${r.total_float} wk${r.critical ? ' · CRITICAL' : ''}\n${r.start_date} → ${r.finish_date}`}</title>
                <rect x={0} y={y} width={W} height={RH} fill={sel ? 'var(--accent-soft)' : 'transparent'} />
                <text clipPath="url(#pg-label-clip)" x={6} y={y + 14.5} fontSize={11} fill={r.critical ? 'var(--text)' : 'var(--text-2)'} fontWeight={r.critical ? 650 : 400} className={narrow ? 'mono' : undefined}>
                  {label.length > (narrow ? 12 : 36) ? label.slice(0, narrow ? 12 : 35) + '…' : label}
                </text>
                {r.milestone ? (
                  c >= r.ef && (
                    <g>
                      <rect x={x(r.ef) - 5.5} y={y + 5} width={11} height={11} transform={`rotate(45 ${x(r.ef)} ${y + 10.5})`} fill={col} stroke="var(--surface-1)" strokeWidth={1.2} />
                      {!narrow && <text x={x(r.ef) + 10} y={y + 14.5} fontSize={10} fill="var(--muted)">{r.id}</text>}
                    </g>
                  )
                ) : (
                  <g>
                    <rect x={x(r.es)} y={y + 4} width={Math.max(0, x(r.es + g) - x(r.es))} height={RH - 8} rx={3} fill={col} opacity={r.critical ? 0.95 : 0.8} stroke={sel ? 'var(--text)' : 'none'} />
                    {showFloat && r.total_float > 1e-6 && g >= r.duration && (
                      <rect x={x(r.ef)} y={y + 8} width={Math.max(0, x(r.ef + r.total_float) - x(r.ef))} height={RH - 16} fill="url(#pg-hatch)" stroke="var(--muted)" strokeWidth={0.6} opacity={0.8} />
                    )}
                    {r.critical && g >= r.duration && !narrow && <text x={x(r.ef) + 4} y={y + 14.5} fontSize={9.5} fill={CRIT} fontWeight={700}>◆</text>}
                  </g>
                )}
              </g>
            );
          })}
          {links.map((k) => (
            <path key={k.key} d={k.d} fill="none" stroke={k.crit ? CRIT : 'var(--muted)'} strokeWidth={k.hot ? 1.8 : k.crit ? 1.3 : 0.8} opacity={k.visible ? (k.hot ? 1 : k.crit ? 0.8 : 0.35) : 0} markerEnd={k.crit ? 'url(#pg-arrow-c)' : 'url(#pg-arrow)'} pointerEvents="none" />
          ))}
          {cursor !== null && cursor < end && (
            <g pointerEvents="none">
              <line x1={x(cursor)} x2={x(cursor)} y1={22} y2={H - 4} stroke="var(--accent)" strokeWidth={1.5} />
              <rect x={x(cursor) - 26} y={H - 18} width={52} height={14} rx={4} fill="var(--accent)" />
              <text x={x(cursor)} y={H - 7.5} fontSize={10} fill="#fff" textAnchor="middle" className="mono">{`wk ${cursor.toFixed(1)}`}</text>
            </g>
          )}
        </svg>
      )}
    </div>
  );
}
