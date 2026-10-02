// Shared helpers for the RF & Antenna Lab: worker calls, the diverging field
// colour map, animation loop, formatting and the Iris design binding.
import { useEffect, useRef, useState } from 'react';
import { useProject } from '../../state/project';
import type { Element, IrisRf } from './types';

// ------------------------------------------------------------------ worker

let worker: Worker | null = null;
let seq = 0;
const pending = new Map<number, { resolve: (v: { data: unknown; ms: number }) => void; reject: (e: Error) => void }>();

function getWorker(): Worker {
  if (!worker) {
    worker = new Worker(new URL('./engineWorker.ts', import.meta.url), { type: 'module' });
    worker.onmessage = (e: MessageEvent<{ id: number; ok: unknown; error: string | null; ms: number }>) => {
      const p = pending.get(e.data.id);
      if (!p) return;
      pending.delete(e.data.id);
      if (e.data.error) p.reject(new Error(e.data.error));
      else p.resolve({ data: e.data.ok, ms: e.data.ms });
    };
  }
  return worker;
}

export function workerCall<T>(method: string, input: unknown): Promise<{ data: T; ms: number }> {
  const id = ++seq;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve: resolve as (v: { data: unknown; ms: number }) => void, reject });
    getWorker().postMessage({ id, method, input });
  });
}

export interface WorkerState<T> { data: T | null; error: string | null; busy: boolean; ms: number }

/** Run `method(input)` in the RF worker whenever the JSON of `input` changes; the last result stays visible while the next computes. */
export function useWorkerCompute<T>(method: string, input: unknown | null): WorkerState<T> {
  const key = input === null ? null : JSON.stringify(input);
  const [state, setState] = useState<WorkerState<T>>({ data: null, error: null, busy: false, ms: 0 });
  const latest = useRef(0);
  useEffect(() => {
    if (key === null) return;
    const ticket = ++latest.current;
    setState((s) => ({ ...s, busy: true, error: null }));
    const t = setTimeout(() => {
      workerCall<T>(method, JSON.parse(key))
        .then(({ data, ms }) => { if (ticket === latest.current) setState({ data, error: null, busy: false, ms }); })
        .catch((err: Error) => { if (ticket === latest.current) setState((s) => ({ ...s, error: err.message, busy: false })); });
    }, 80);
    return () => clearTimeout(t);
  }, [method, key]);
  return state;
}

// ------------------------------------------------------------------ colour

function hexRgb(h: string): [number, number, number] {
  const n = parseInt(h.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** Current theme (the app sets data-theme on <html>). */
export function isLight(): boolean {
  return typeof document !== 'undefined' && document.documentElement.getAttribute('data-theme') === 'light';
}

/**
 * Diverging blue ↔ red map with a neutral grey midpoint (never a hue at zero),
 * built per theme so "no field" reads as the surface. Returns a 256-entry RGBA LUT.
 */
export function divergingLut(light: boolean): Uint8ClampedArray {
  const neg = light ? ['#0d3a7a', '#2a78d6', '#8fb8ea'] : ['#9cc3f5', '#3987e5', '#173f78'];
  const pos = light ? ['#f0a3a0', '#d03b3b', '#7a1515'] : ['#7c2020', '#e66767', '#f7b9b5'];
  const mid = light ? '#f0efec' : '#383835';
  // stops from −1 → +1
  const stops = light
    ? [neg[0], neg[1], neg[2], mid, pos[0], pos[1], pos[2]]
    : [neg[0], neg[1], neg[2], mid, pos[0], pos[1], pos[2]];
  const rgb = stops.map(hexRgb);
  const lut = new Uint8ClampedArray(256 * 4);
  for (let q = 0; q < 256; q++) {
    const v = (q - 128) / 127; // −1..1
    const x = Math.max(0, Math.min(1, (v + 1) / 2)) * (rgb.length - 1);
    const i = Math.min(rgb.length - 2, Math.floor(x));
    const f = x - i;
    const a = rgb[i];
    const b = rgb[i + 1];
    lut[q * 4] = a[0] + (b[0] - a[0]) * f;
    lut[q * 4 + 1] = a[1] + (b[1] - a[1]) * f;
    lut[q * 4 + 2] = a[2] + (b[2] - a[2]) * f;
    lut[q * 4 + 3] = 255;
  }
  return lut;
}

export function lutGradient(light: boolean): string {
  const lut = divergingLut(light);
  const stops: string[] = [];
  for (let k = 0; k <= 8; k++) {
    const q = Math.round(1 + (254 * k) / 8);
    stops.push(`rgb(${lut[q * 4]}, ${lut[q * 4 + 1]}, ${lut[q * 4 + 2]})`);
  }
  return `linear-gradient(90deg, ${stops.join(', ')})`;
}

/** Sequential blue ramp for magnitudes (light → dark in light theme, reversed in dark). */
export function seqBlue(t: number, light: boolean): [number, number, number] {
  const stops = (light ? ['#dbe8f8', '#8fb8ea', '#2a78d6', '#0d3a7a'] : ['#173f78', '#2a67b8', '#3987e5', '#bcd6f7']).map(hexRgb);
  const x = Math.max(0, Math.min(1, t)) * (stops.length - 1);
  const i = Math.min(stops.length - 2, Math.floor(x));
  const f = x - i;
  return [0, 1, 2].map((c) => (stops[i][c] + (stops[i + 1][c] - stops[i][c]) * f) / 255) as [number, number, number];
}

/** Re-render when the theme attribute changes. */
export function useTheme(): boolean {
  const [light, setLight] = useState(isLight());
  useEffect(() => {
    const obs = new MutationObserver(() => setLight(isLight()));
    obs.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
    return () => obs.disconnect();
  }, []);
  return light;
}

// ------------------------------------------------------------------ misc

export function prefersReducedMotion(): boolean {
  return typeof window !== 'undefined' && !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
}

/** requestAnimationFrame loop while `active`; callback gets dt seconds. */
export function useRaf(active: boolean, cb: (dt: number) => void) {
  const ref = useRef(cb);
  ref.current = cb;
  useEffect(() => {
    if (!active) return;
    let raf = 0;
    let last = performance.now();
    const tick = (now: number) => {
      const dt = Math.max(0, Math.min(0.1, (now - last) / 1000));
      last = now;
      ref.current(dt);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [active]);
}

export function b64Bytes(s: string): Uint8Array {
  const bin = atob(s);
  const u8 = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
  return u8;
}

export function fmt(v: number | null | undefined, digits = 2): string {
  if (v == null || !isFinite(v)) return '—';
  const a = Math.abs(v);
  if (a >= 1e4) return v.toExponential(2);
  if (a >= 100) return v.toFixed(0);
  if (a >= 10) return v.toFixed(Math.max(0, digits - 1));
  return v.toFixed(digits);
}

export function toCsv(rows: (string | number)[][]): string {
  return rows.map((r) => r.map((c) => (typeof c === 'number' ? String(c) : `"${String(c).replace(/"/g, '""')}"`)).join(',')).join('\n') + '\n';
}

export function elLabel(e: Element): string {
  const l = e.kind === 'series_l' || e.kind === 'shunt_l';
  const v = l ? e.value * 1e9 : e.value * 1e12;
  const s = v >= 10 ? v.toFixed(0) : String(Number(v.toFixed(2)));
  return `${e.kind.startsWith('series') ? 'series' : 'shunt'} ${s} ${l ? 'nH' : 'pF'}`;
}

// ------------------------------------------------------------------ Iris binding

export const RF_DEFAULTS: IrisRf = {
  antenna: { r_ohm: 18, x_ohm: -30, q: 12, fc_ghz: 2.44175 },
  rad_efficiency: 0.5,
  directivity_dbi: 1.76,
  network: null,
  feed: null,
  er: 4.4,
  tan_d: 0.02,
  cu_um: 35,
  q_l: 40,
  q_c: 150,
  series: 'E24',
  link: { phy: '1m', distance_m: 5, path_exponent: 2.5, tx_dbm: 0, phone_tx_dbm: 0, phone_gain_dbi: -2, phone_sens_1m_dbm: -90, head_loss_db: 6, body_loss_db: 4, availability: 0.9 },
};

/** The stored `design.extra.rf` merged over defaults, plus a patcher. */
export function useIrisRf(): [IrisRf, (patch: Partial<IrisRf>) => void, boolean] {
  const project = useProject((s) => s.project);
  const patchDesign = useProject((s) => s.patchDesign);
  const stored = (project.design.extra?.rf ?? null) as Partial<IrisRf> | null;
  const rf: IrisRf = {
    ...RF_DEFAULTS,
    ...(stored ?? {}),
    antenna: { ...RF_DEFAULTS.antenna, ...(stored?.antenna ?? {}) },
    link: { ...RF_DEFAULTS.link, ...(stored?.link ?? {}) },
  };
  const patch = (p: Partial<IrisRf>) => {
    const cur = (useProject.getState().project.design.extra?.rf ?? {}) as Partial<IrisRf>;
    patchDesign('extra', { rf: { ...cur, ...p } } as Record<string, unknown>);
  };
  return [rf, patch, stored !== null];
}

export const PHY_LABEL: Record<string, string> = { '1m': 'LE 1M', '2m': 'LE 2M', s2: 'LE Coded S=2', s8: 'LE Coded S=8' };
