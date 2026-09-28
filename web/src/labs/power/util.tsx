// Shared helpers for the Power & Thermal Lab.
import type { ReactNode } from 'react';
import type { Project } from '../../engine/types';
import { seriesColor } from '../../components/Plot';
import { mix } from '../control/kit';

export function Seg<T extends string | number>(props: { value: T; options: { value: T; label: ReactNode; title?: string }[]; onChange: (v: T) => void; label?: string }) {
  return (
    <div className="pw-seg" role="group" aria-label={props.label}>
      {props.options.map((o) => (
        <button key={String(o.value)} className={`pw-seg-btn ${o.value === props.value ? 'active' : ''}`} aria-pressed={o.value === props.value} title={o.title} onClick={() => props.onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

/** Colour of subsystem `id`, stable by its order in the project. */
export function subsystemColor(p: Project, id: string): string {
  const i = p.subsystems.findIndex((s) => s.id === id);
  return seriesColor(i < 0 ? 7 : i);
}

/** Requirement target for a metric key (first match). */
export function reqTarget(p: Project, metric: string): { id: string; target: number; cmp: string } | null {
  const r = p.requirements.find((x) => x.verification.metric === metric);
  return r ? { id: r.id, target: r.verification.target, cmp: r.verification.cmp } : null;
}

/** Hours as "10 h 21 min". */
export function fmtHours(h: number): string {
  if (!Number.isFinite(h)) return '—';
  if (h >= 100) return `${h.toFixed(0)} h`;
  const hh = Math.floor(h);
  const mm = Math.round((h - hh) * 60);
  return mm === 60 ? `${hh + 1} h 0 min` : `${hh} h ${mm} min`;
}

/** Wall-clock time for `h` hours after `startH` (e.g. 7 → "07:00"). */
export function clockTime(h: number, startH = 7): string {
  const t = startH + h;
  const day = Math.floor(t / 24);
  const hh = Math.floor(t % 24);
  const mm = Math.floor((t * 60) % 60);
  return `${String(hh).padStart(2, '0')}:${String(mm).padStart(2, '0')}${day > 0 ? ` +${day}d` : ''}`;
}

/**
 * Temperature colour: cool (series-1 blue) → warm (series-4 amber) → hot
 * (bad red), over [lo, hi] °C. Returned as a CSS color-mix string.
 */
export function tempColor(t: number, lo = 25, hi = 48): string {
  const f = Math.max(0, Math.min(1, (t - lo) / (hi - lo)));
  if (f < 0.5) return mix('var(--series-4)', 'var(--series-1)', (f / 0.5) * 100);
  return mix('var(--bad)', 'var(--series-4)', ((f - 0.5) / 0.5) * 100);
}

export const TEMP_RAMP_CSS = 'linear-gradient(90deg, var(--series-1), var(--series-4), var(--bad))';

/** Status chip content for a requirement check (never colour only). */
export function passWord(ok: boolean): string {
  return ok ? '✓ meets' : '✕ misses';
}

export function Kv({ rows }: { rows: [ReactNode, ReactNode][] }) {
  return (
    <dl className="pw-kv">
      {rows.map(([k, v], i) => (
        <div key={i} style={{ display: 'contents' }}>
          <dt>{k}</dt>
          <dd>{v}</dd>
        </div>
      ))}
    </dl>
  );
}
