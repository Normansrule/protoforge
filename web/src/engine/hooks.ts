import { useDeferredValue, useEffect, useMemo, useState } from 'react';
import { engineLoaded, loadEngine, tryCall } from './engine';

/** true once the WebAssembly engine is ready. */
export function useEngineReady(): boolean {
  const [ready, setReady] = useState(engineLoaded());
  useEffect(() => {
    if (!ready) loadEngine().then(() => setReady(true));
  }, [ready]);
  return ready;
}

export interface Computed<T> {
  data: T | null;
  error: string | null;
  /** Milliseconds the engine call took. */
  ms: number;
  ready: boolean;
}

/**
 * Run an engine method whenever `input` changes (deep-compared via JSON).
 * Input is passed through `useDeferredValue`, so dragging a slider stays
 * smooth even when a simulation takes a few milliseconds.
 */
export function useCompute<T>(method: string, input: unknown, enabled = true): Computed<T> {
  const ready = useEngineReady();
  const key = JSON.stringify(input ?? null);
  const deferredKey = useDeferredValue(key);
  return useMemo(() => {
    if (!ready || !enabled) return { data: null, error: null, ms: 0, ready };
    const t0 = performance.now();
    const { data, error } = tryCall<T>(method, JSON.parse(deferredKey));
    return { data, error, ms: performance.now() - t0, ready };
  }, [ready, enabled, method, deferredKey]);
}
