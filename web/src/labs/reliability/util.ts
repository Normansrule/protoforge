// Shared helpers for the Reliability & Test Lab: formatting, animation, the
// lab's settings in design.extra.reliability, CSV.
import { useCallback, useEffect, useRef } from 'react';
import { useProject } from '../../state/project';
import type { RelConfig } from './types';

export function prefersReducedMotion(): boolean {
  return typeof window !== 'undefined' && !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
}

/** requestAnimationFrame loop while `active`; the callback gets dt in seconds. */
export function useRaf(active: boolean, cb: (dt: number) => void) {
  const ref = useRef(cb);
  ref.current = cb;
  useEffect(() => {
    if (!active) return;
    let raf = 0;
    let last = performance.now();
    const tick = (now: number) => {
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      ref.current(dt);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [active]);
}

export function fmt(v: number | null | undefined, digits = 3): string {
  if (v == null || Number.isNaN(v)) return '—';
  if (!isFinite(v)) return '∞';
  const a = Math.abs(v);
  if (a >= 1e6) return v.toExponential(2);
  if (a >= 1000) return Math.round(v).toLocaleString('en-US');
  if (a >= 100) return v.toFixed(0);
  if (a === 0) return '0';
  return Number(v.toPrecision(digits)).toString();
}

export function pct(v: number | null | undefined, digits = 1): string {
  if (v == null || !isFinite(v)) return '—';
  return `${(v * 100).toFixed(digits)} %`;
}

export function hoursToYears(h: number, hoursPerDay: number): number {
  return h / (365 * hoursPerDay);
}

export function toCsv(rows: (string | number | null)[][]): string {
  return rows.map((r) => r.map((c) => (c == null ? '' : typeof c === 'number' ? String(c) : /[",\n]/.test(c) ? `"${c.replace(/"/g, '""')}"` : c)).join(',')).join('\n') + '\n';
}

export const DEFAULT_CONFIG: RelConfig = {
  mission_years: 2, hours_per_day: 8, hinge_cycles_per_day: 12, use_ambient_c: null, use_rh_pct: 60, daily_swing_c: 20,
  ea_ev: 0.7, fit_ref_c: 40, mic_k: 2, battery_counts: true, blocks: {},
  dvt_reliability: 0.9, dvt_confidence: 0.9, dvt_failures: 0, life_multiple: 2, life_beta: 2, qual_units: 3,
};

/** The lab's settings (design.extra.reliability) with defaults, and a patcher that writes them to the project. */
export function useRelConfig(): [RelConfig, (patch: Partial<RelConfig>) => void] {
  const extra = useProject((s) => s.project.design.extra) as Record<string, unknown>;
  const patchDesign = useProject((s) => s.patchDesign);
  const stored = (extra?.reliability ?? {}) as Partial<RelConfig>;
  const cfg: RelConfig = { ...DEFAULT_CONFIG, ...stored, blocks: { ...(stored.blocks ?? {}) } };
  const set = useCallback(
    (patch: Partial<RelConfig>) => {
      const cur = (useProject.getState().project.design.extra ?? {}) as Record<string, unknown>;
      const prev = (cur.reliability ?? {}) as Partial<RelConfig>;
      patchDesign('extra', { ...cur, reliability: { ...prev, ...patch } } as never);
    },
    [patchDesign],
  );
  return [cfg, set];
}

/** Weibull paper transform y = ln(−ln(1 − F)). */
export const wy = (f: number) => Math.log(-Math.log(1 - f));

/** Small deterministic PRNG for visual demonstrations (mulberry32). */
export function mulberry32(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export const MODE_COLORS = ['var(--series-5)', 'var(--series-1)', 'var(--series-2)'];
export const MODE_NAMES = ['infant mortality', 'random', 'wear-out'];
