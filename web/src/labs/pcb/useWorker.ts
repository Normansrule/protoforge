// Promise-based access to the engine worker plus a React hook.
import { useEffect, useRef, useState } from 'react';

let worker: Worker | null = null;
let seq = 0;
const pending = new Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void }>();

function getWorker(): Worker {
  if (!worker) {
    worker = new Worker(new URL('./engineWorker.ts', import.meta.url), { type: 'module' });
    worker.onmessage = (e: MessageEvent<{ id: number; ok: unknown; error: string | null }>) => {
      const p = pending.get(e.data.id);
      if (!p) return;
      pending.delete(e.data.id);
      if (e.data.error) p.reject(new Error(e.data.error));
      else p.resolve(e.data.ok);
    };
  }
  return worker;
}

/** Call an engine method in the worker. */
export function workerCall<T>(method: string, input: unknown): Promise<T> {
  const id = ++seq;
  return new Promise<T>((resolve, reject) => {
    pending.set(id, { resolve: resolve as (v: unknown) => void, reject });
    getWorker().postMessage({ id, method, input });
  });
}

export interface WorkerState<T> {
  data: T | null;
  error: string | null;
  busy: boolean;
  ms: number;
}

/** Run `method(input)` in the worker whenever the JSON of `input` changes (null input = idle). */
export function useWorkerCompute<T>(method: string, input: unknown | null): WorkerState<T> {
  const key = input === null ? null : JSON.stringify(input);
  const [state, setState] = useState<WorkerState<T>>({ data: null, error: null, busy: false, ms: 0 });
  const latest = useRef(0);
  useEffect(() => {
    if (key === null) return;
    const ticket = ++latest.current;
    const t0 = performance.now();
    setState((s) => ({ ...s, busy: true, error: null }));
    workerCall<T>(method, JSON.parse(key))
      .then((data) => {
        if (ticket === latest.current) setState({ data, error: null, busy: false, ms: performance.now() - t0 });
      })
      .catch((err: Error) => {
        if (ticket === latest.current) setState((s) => ({ ...s, error: err.message, busy: false }));
      });
  }, [method, key]);
  return state;
}
