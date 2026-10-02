// A dedicated WebAssembly engine instance for the RF & Antenna Lab, so the
// finite-difference time-domain field solver never blocks the page (or the
// shared requirement-evaluation worker).
import init, { call } from '../../wasm/forge_wasm.js';

const ready = init();

self.onmessage = async (e: MessageEvent<{ id: number; method: string; input: unknown }>) => {
  const { id, method, input } = e.data;
  await ready;
  const t0 = performance.now();
  try {
    const raw = call(method, JSON.stringify(input ?? null));
    const env = JSON.parse(raw) as { ok?: unknown; error?: string };
    (self as unknown as Worker).postMessage({ id, ok: env.ok, error: env.error ?? null, ms: performance.now() - t0 });
  } catch (err) {
    (self as unknown as Worker).postMessage({ id, ok: null, error: String(err), ms: performance.now() - t0 });
  }
};
