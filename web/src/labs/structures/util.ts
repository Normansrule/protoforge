// Shared helpers for the Structures Lab: worker calls, colour ramps, formatting.
import { useEffect, useRef, useState } from 'react';

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

/** Call an engine method in the Structures worker. */
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
        .then(({ data, ms }) => {
          if (ticket === latest.current) setState({ data, error: null, busy: false, ms });
        })
        .catch((err: Error) => {
          if (ticket === latest.current) setState((s) => ({ ...s, error: err.message, busy: false }));
        });
    }, 60);
    return () => clearTimeout(t);
  }, [method, key]);
  return state;
}

// ------------------------------------------------------------------ colour

/** Sequential single-hue ramp (orange, light → dark) for stress magnitude. */
export const HEAT: string[] = ['#fde6d9', '#f9c9ae', '#f4a57e', '#ec8052', '#d95926', '#b4461b', '#8a3413', '#62240c'];
/** Status colour for "beyond yield" (reserved; always shown with a label). */
export const CRITICAL = '#d03b3b';

function hexRgb(h: string): [number, number, number] {
  const n = parseInt(h.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
const HEAT_RGB = HEAT.map(hexRgb);

/** Colour for t ∈ [0, 1] on the heat ramp, as 0..1 RGB. */
export function heat(t: number): [number, number, number] {
  const x = Math.max(0, Math.min(1, t)) * (HEAT_RGB.length - 1);
  const i = Math.min(HEAT_RGB.length - 2, Math.floor(x));
  const f = x - i;
  const a = HEAT_RGB[i];
  const b = HEAT_RGB[i + 1];
  return [(a[0] + (b[0] - a[0]) * f) / 255, (a[1] + (b[1] - a[1]) * f) / 255, (a[2] + (b[2] - a[2]) * f) / 255];
}

export const HEAT_GRADIENT = `linear-gradient(90deg, ${HEAT.join(', ')})`;

// ------------------------------------------------------------------ misc

export function prefersReducedMotion(): boolean {
  return typeof window !== 'undefined' && !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
}

export function fmt(v: number | null | undefined, digits = 2): string {
  if (v == null || !isFinite(v)) return v === Infinity ? '∞' : '—';
  const a = Math.abs(v);
  if (a >= 1e4) return v.toExponential(2);
  if (a >= 100) return v.toFixed(0);
  if (a >= 10) return v.toFixed(Math.max(0, digits - 1));
  return v.toFixed(digits);
}

export function sfTone(sf: number, target = 2): 'pass' | 'warn' | 'fail' {
  if (!isFinite(sf) || sf >= target) return 'pass';
  if (sf >= 1) return 'warn';
  return 'fail';
}

export function sfWord(sf: number, target = 2): string {
  const t = sfTone(sf, target);
  return t === 'pass' ? `✓ ≥ ${target}` : t === 'warn' ? `! below ${target}` : '✕ below 1 — yields';
}

export function b64Bytes(s: string): Uint8Array {
  const bin = atob(s);
  const u8 = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
  return u8;
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

export const PART_LABEL: Record<string, string> = {
  front: 'Front (rims + bridge)',
  temple_r: 'Right temple + electronics pod',
  temple_l: 'Left temple + battery pod',
};

export function toCsv(rows: (string | number)[][]): string {
  return rows.map((r) => r.map((c) => (typeof c === 'number' ? String(c) : `"${String(c).replace(/"/g, '""')}"`)).join(',')).join('\n') + '\n';
}
