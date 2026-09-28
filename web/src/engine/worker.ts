// Promise-based client for the engine worker.
export interface WorkerResult<T> {
  data: T | null;
  error: string | null;
  ms: number;
}

let worker: Worker | null = null;
let nextId = 1;
const pending = new Map<number, (r: WorkerResult<unknown>) => void>();

function getWorker(): Worker {
  if (!worker) {
    worker = new Worker(new URL('./engineWorker.ts', import.meta.url), { type: 'module' });
    worker.onmessage = (e: MessageEvent<{ id: number; ok: unknown; error: string | null; ms: number }>) => {
      const cb = pending.get(e.data.id);
      pending.delete(e.data.id);
      cb?.({ data: e.data.ok, error: e.data.error, ms: e.data.ms });
    };
  }
  return worker;
}

/** Call an engine method in the background worker. */
export function callInWorker<T>(method: string, input?: unknown): Promise<WorkerResult<T>> {
  const id = nextId++;
  return new Promise((resolve) => {
    pending.set(id, resolve as (r: WorkerResult<unknown>) => void);
    getWorker().postMessage({ id, method, input });
  });
}
