// Shared helpers for the Mechanisms Lab: number formatting, design-check
// lists, key–value tables, SVG pointer dragging and the animation clock
// (re-exported from the Control lab kit so every lab animates the same way).
import { useCallback, useRef, type ReactNode } from 'react';
export { useClock, PlayBar, toCSV, usePrefersReducedMotion } from '../control/kit';
import type { Check } from './types';

export const DEG = Math.PI / 180;

export function fmt(v: number | null | undefined, digits = 2): string {
  if (v == null || !Number.isFinite(v)) return '—';
  const a = Math.abs(v);
  if (a !== 0 && (a >= 1e5 || a < 1e-3)) return v.toExponential(2);
  if (a >= 100) return v.toFixed(Math.max(0, digits - 2));
  if (a >= 10) return v.toFixed(Math.max(0, digits - 1));
  return v.toFixed(digits);
}

/** Cycles with a "≥ 10⁷ (runout)" style for capped values. */
export function fmtCycles(v: number, runout = false): string {
  if (runout || v >= 1e11) return '∞ (runout)';
  if (v >= 1e6) return `${(v / 1e6).toFixed(v >= 1e7 ? 0 : 1)} M`;
  if (v >= 1e3) return `${(v / 1e3).toFixed(v >= 1e4 ? 0 : 1)} k`;
  return v.toFixed(0);
}

export function sfTone(sf: number, target = 1.5): 'pass' | 'warn' | 'fail' {
  if (!Number.isFinite(sf) || sf >= target) return 'pass';
  if (sf >= 1) return 'warn';
  return 'fail';
}

/** Pass/fail list for design rules — icon + word, never colour alone. */
export function Checks({ checks }: { checks: Check[] }) {
  return (
    <ul className="mx-checks">
      {checks.map((c) => (
        <li key={c.name} className={c.ok ? 'ok' : 'bad'}>
          <span className="mx-check-icon" aria-hidden>{c.ok ? '✓' : '✕'}</span>
          <span>
            <b>{c.name}</b> <span className="muted">— {c.ok ? 'pass' : 'fail'}: {c.detail}</span>
          </span>
        </li>
      ))}
    </ul>
  );
}

export function Kv({ rows }: { rows: [ReactNode, ReactNode][] }) {
  return (
    <dl className="mx-kv">
      {rows.map(([k, v], i) => (
        <div key={i} style={{ display: 'contents' }}>
          <dt>{k}</dt>
          <dd>{v}</dd>
        </div>
      ))}
    </dl>
  );
}

export function Notes({ notes }: { notes: string[] }) {
  if (!notes.length) return null;
  return (
    <ul className="small muted mx-notes">
      {notes.map((n, i) => <li key={i}>{n}</li>)}
    </ul>
  );
}

/**
 * Pointer dragging inside an SVG with a viewBox: returns handlers that
 * report positions in SVG user units while a handle is held.
 */
export function useSvgDrag(onDrag: (id: string, x: number, y: number) => void) {
  const svgRef = useRef<SVGSVGElement | null>(null);
  const active = useRef<string | null>(null);
  const toLocal = (e: React.PointerEvent) => {
    const svg = svgRef.current;
    if (!svg) return null;
    const pt = svg.createSVGPoint();
    pt.x = e.clientX;
    pt.y = e.clientY;
    const m = svg.getScreenCTM();
    if (!m) return null;
    const q = pt.matrixTransform(m.inverse());
    return { x: q.x, y: q.y };
  };
  const start = useCallback((id: string) => (e: React.PointerEvent) => {
    active.current = id;
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
    e.preventDefault();
  }, []);
  const move = (e: React.PointerEvent) => {
    if (!active.current) return;
    const q = toLocal(e);
    if (q) onDrag(active.current, q.x, q.y);
  };
  const end = () => {
    active.current = null;
  };
  return { svgRef, start, move, end, dragging: () => active.current };
}

/** Download helper for CSV built from columns. */
export function csvFrom(header: string[], cols: number[][]): string {
  const n = Math.max(0, ...cols.map((c) => c.length));
  const rows = [header.join(',')];
  for (let i = 0; i < n; i++) rows.push(cols.map((c) => (Number.isFinite(c[i]) ? String(+c[i].toPrecision(8)) : '')).join(','));
  return rows.join('\n') + '\n';
}

export const clamp = (v: number, a: number, b: number) => Math.max(a, Math.min(b, v));

/** SVG path through points (optionally closed). */
export function pathOf(xs: number[], ys: number[], closed = false, sx = (v: number) => v, sy = (v: number) => v): string {
  let d = '';
  let pen = false;
  for (let i = 0; i < xs.length; i++) {
    const x = xs[i];
    const y = ys[i];
    if (!Number.isFinite(x) || !Number.isFinite(y)) {
      pen = false;
      continue;
    }
    d += `${pen ? 'L' : 'M'}${sx(x).toFixed(2)},${sy(y).toFixed(2)}`;
    pen = true;
  }
  return closed && d ? d + 'Z' : d;
}

/** Zig-zag spring polyline between two points (for diagrams). */
export function springPath(x0: number, y0: number, x1: number, y1: number, coils: number, amp: number): string {
  const dx = x1 - x0;
  const dy = y1 - y0;
  const L = Math.hypot(dx, dy) || 1;
  const ux = dx / L;
  const uy = dy / L;
  const nx = -uy;
  const ny = ux;
  const lead = Math.min(0.12 * L, 6);
  let d = `M${x0.toFixed(2)},${y0.toFixed(2)} L${(x0 + ux * lead).toFixed(2)},${(y0 + uy * lead).toFixed(2)}`;
  const n = Math.max(2, Math.round(coils * 2));
  const body = L - 2 * lead;
  for (let k = 1; k <= n; k++) {
    const t = lead + (body * (k - 0.5)) / n;
    const s = k % 2 ? 1 : -1;
    d += ` L${(x0 + ux * t + nx * amp * s).toFixed(2)},${(y0 + uy * t + ny * amp * s).toFixed(2)}`;
  }
  d += ` L${(x1 - ux * lead).toFixed(2)},${(y1 - uy * lead).toFixed(2)} L${x1.toFixed(2)},${y1.toFixed(2)}`;
  return d;
}
