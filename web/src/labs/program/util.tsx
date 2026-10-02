// Shared helpers for the Program & Business Lab: the lab's configuration in
// design.extra.program, number formatting, and the engine hooks.
import { useCallback } from 'react';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { Config, ConfigOut, Task } from './types';

import { useClock as useKitClock, usePrefersReducedMotion, type Clock } from '../control/kit';

export { PlayBar, usePrefersReducedMotion } from '../control/kit';

/**
 * The shared animation clock, with one change for this lab: when the viewer
 * prefers reduced motion the clock does not autoplay, so a chart that would
 * otherwise sit on its empty first frame shows its finished state instead.
 * Play / replay still animate on request.
 */
export function useClock(duration: number, opts: Parameters<typeof useKitClock>[1] = {}): Clock {
  const c = useKitClock(duration, opts);
  const reduced = usePrefersReducedMotion();
  if (reduced && opts.autoplay !== false && !c.playing && c.t === 0) return { ...c, t: duration };
  return c;
}

/** The resolved configuration (defaults filled in by the engine) and a saver. */
export function useProgram() {
  const project = useProject((s) => s.project);
  const patchDesign = useProject((s) => s.patchDesign);
  const out = useCompute<ConfigOut>('program.config', { project });
  const save = useCallback(
    (patch: Partial<Config>) => {
      const p = useProject.getState().project;
      const extra = (p.design.extra ?? {}) as Record<string, unknown>;
      const cur = (extra.program ?? {}) as Partial<Config>;
      patchDesign('extra', { ...extra, program: { ...cur, ...patch } });
    },
    [patchDesign],
  );
  const setTasks = useCallback((tasks: Task[] | null) => save({ tasks }), [save]);
  return { project, cfg: out.data, error: out.error, save, setTasks };
}

export function usd(v: number | null | undefined, digits = 0): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return '—';
  const a = Math.abs(v);
  const sign = v < 0 ? '−' : '';
  if (a >= 1e6) return `${sign}$${(a / 1e6).toFixed(a >= 1e7 ? 1 : 2)}M`;
  if (a >= 1e4) return `${sign}$${(a / 1e3).toFixed(a >= 1e5 ? 0 : 1)}k`;
  return `${sign}$${a.toFixed(digits)}`;
}

export function num(v: number | null | undefined, digits = 1): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return '—';
  return v.toFixed(digits);
}

export function int(v: number | null | undefined): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return '—';
  return Math.round(v).toLocaleString('en-US');
}

export function addWeeks(iso: string, w: number): string {
  const [y, m, d] = iso.split('-').map(Number);
  const t = Date.UTC(y || 2026, (m || 10) - 1, d || 5) + Math.round(w * 7) * 86400000;
  return new Date(t).toISOString().slice(0, 10);
}

export function shortDate(iso: string): string {
  const [y, m] = iso.split('-').map(Number);
  const names = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  return `${names[(m || 1) - 1]} '${String(y).slice(2)}`;
}

/** Colour slot per phase (fixed order so charts stay consistent). */
export const PHASES = ['Concept', 'Requirements', 'Design', 'EVT', 'DVT', 'PVT'];
export function phaseColor(phase: string): string {
  const i = PHASES.indexOf(phase);
  const slots = ['var(--series-7)', 'var(--series-5)', 'var(--series-1)', 'var(--series-3)', 'var(--series-4)', 'var(--series-6)'];
  return i >= 0 ? slots[i] : 'var(--series-8)';
}
export const CRIT = 'var(--series-2)';

/** "Assumption — edit" marker, per the lab's honesty rule. */
export function Assume() {
  return <span className="pg-assume" title="An assumption — edit it for your product">assumption — edit</span>;
}

export function downloadName(project: { meta: { id: string } }, what: string, ext: string) {
  return `${project.meta.id || 'project'}-${what}.${ext}`;
}
