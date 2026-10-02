// Shared helpers for the Thermal & Fluids Lab: a dedicated engine worker for
// the heavy conduction solves, base64 decoding, colour ramps and formatting.
import { useEffect, useRef, useState } from 'react';
import type { Project } from '../../engine/types';
import type { FluidsDesign } from './types';

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

/** Call an engine method in the Thermal & Fluids worker. */
export function workerCall<T>(method: string, input: unknown): Promise<{ data: T; ms: number }> {
  const id = ++seq;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve: resolve as (v: { data: unknown; ms: number }) => void, reject });
    getWorker().postMessage({ id, method, input });
  });
}

export interface WorkerState<T> {
  data: T | null;
  error: string | null;
  busy: boolean;
  ms: number;
}

/** Run `method(input)` in the worker whenever the JSON of `input` changes (null = idle).
 * The last result stays visible while a newer one computes. */
export function useWorkerCompute<T>(method: string, input: unknown | null, delay = 80): WorkerState<T> {
  const key = input === null ? null : JSON.stringify(input);
  const [state, setState] = useState<WorkerState<T>>({ data: null, error: null, busy: false, ms: 0 });
  const latest = useRef(0);
  useEffect(() => {
    if (key === null) return;
    const ticket = ++latest.current;
    setState((s) => ({ ...s, busy: true, error: null }));
    const t = setTimeout(() => {
      workerCall<T>(method, JSON.parse(key))
        .then(({ data, ms }) => {
          if (ticket === latest.current) setState({ data, error: null, busy: false, ms });
        })
        .catch((err: Error) => {
          if (ticket === latest.current) setState((s) => ({ ...s, error: err.message, busy: false }));
        });
    }, delay);
    return () => clearTimeout(t);
  }, [method, key, delay]);
  return state;
}

// ------------------------------------------------------------------ base64

export function b64Bytes(s: string): Uint8Array {
  const bin = atob(s);
  const u8 = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
  return u8;
}
export function b64F32(s: string): Float32Array {
  const b = b64Bytes(s);
  return new Float32Array(b.buffer, b.byteOffset, b.byteLength / 4);
}
export function b64U32(s: string): Uint32Array {
  const b = b64Bytes(s);
  return new Uint32Array(b.buffer, b.byteOffset, b.byteLength / 4);
}

// ------------------------------------------------------------------ colour

function hexRgb(h: string): [number, number, number] {
  const n = parseInt(h.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
function ramp(stops: string[]) {
  const rgb = stops.map(hexRgb);
  return (t: number): [number, number, number] => {
    const x = Math.max(0, Math.min(1, t)) * (rgb.length - 1);
    const i = Math.min(rgb.length - 2, Math.floor(x));
    const f = x - i;
    const a = rgb[i];
    const b = rgb[i + 1];
    return [(a[0] + (b[0] - a[0]) * f) / 255, (a[1] + (b[1] - a[1]) * f) / 255, (a[2] + (b[2] - a[2]) * f) / 255];
  };
}

/** Sequential single-hue temperature ramp (light → dark orange-red), same family as the Structures heat map. */
export const TEMP_STOPS = ['#fdeee4', '#f9cfb5', '#f4a57e', '#ec8052', '#d95926', '#b4461b', '#8a3413', '#5a200a'];
export const temp = ramp(TEMP_STOPS);
export const TEMP_GRADIENT = `linear-gradient(90deg, ${TEMP_STOPS.join(', ')})`;

/** Speed: single-hue blue, dark-theme version runs from the background up to bright. */
export const SPEED_LIGHT = ['#f4f7fb', '#cfe0f6', '#9fc2ee', '#6aa1e6', '#3987e5', '#256bc4', '#174f97', '#0d3366'];
export const SPEED_DARK = ['#121418', '#132741', '#16406e', '#1d5aa0', '#3987e5', '#6aa6ee', '#a8cdf6', '#e6f1fd'];
/** Diverging blue ↔ neutral ↔ orange (vorticity, pressure). */
export const DIV_LIGHT = ['#174f97', '#3987e5', '#9fc2ee', '#ecebe7', '#f4a57e', '#d95926', '#8a3413'];
export const DIV_DARK = ['#6aa6ee', '#3987e5', '#1d3c63', '#26262a', '#6b3420', '#d95926', '#f4a57e'];

/** 256-entry RGBA lookup table for a ramp. */
export function lut(stops: string[]): Uint8ClampedArray {
  const f = ramp(stops);
  const out = new Uint8ClampedArray(256 * 4);
  for (let i = 0; i < 256; i++) {
    const [r, g, b] = f(i / 255);
    out[4 * i] = r * 255;
    out[4 * i + 1] = g * 255;
    out[4 * i + 2] = b * 255;
    out[4 * i + 3] = 255;
  }
  return out;
}

export function gradientCss(stops: string[]) {
  return `linear-gradient(90deg, ${stops.join(', ')})`;
}

// ------------------------------------------------------------------ misc

export function prefersReducedMotion(): boolean {
  return typeof window !== 'undefined' && !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
}

export function fmt(v: number | null | undefined, digits = 2): string {
  if (v == null || !isFinite(v)) return '—';
  const a = Math.abs(v);
  if (a >= 1e5) return v.toExponential(2);
  if (a >= 100) return v.toFixed(0);
  if (a >= 10) return v.toFixed(Math.max(0, digits - 1));
  return v.toFixed(digits);
}

export function fmtTime(s: number): string {
  if (s < 90) return `${s.toFixed(0)} s`;
  if (s < 5400) return `${(s / 60).toFixed(s < 600 ? 1 : 0)} min`;
  return `${(s / 3600).toFixed(1)} h`;
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
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      ref.current(dt);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [active]);
}

export function toCsv(rows: (string | number)[][]): string {
  return rows.map((r) => r.map((c) => (typeof c === 'number' ? String(c) : `"${String(c).replace(/"/g, '""')}"`)).join(',')).join('\n') + '\n';
}

export const DESIGN_DEFAULTS: FluidsDesign = {
  spreader: 'none',
  spreader_um: 25,
  spreader_location: 'floor',
  display_heat_in: 'optics',
  emissivity: null,
  air_speed_m_s: 0,
};

/** The lab's design choices stored at design.extra.fluids (with defaults). */
export function fluidsDesign(p: Project): FluidsDesign {
  const raw = (p.design.extra?.fluids ?? {}) as Partial<FluidsDesign>;
  return { ...DESIGN_DEFAULTS, ...raw };
}

export const LOCATION_LABEL: Record<string, string> = {
  floor: 'pod skin-side wall',
  lid: 'under the pod lid',
  optics: 'behind the display (optics housing)',
};
