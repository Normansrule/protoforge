// A WebAssembly engine instance inside a Web Worker. Long-running calls —
// above all `systems.evaluate`, which runs every lab's analysis — execute
// here so the page never blocks while you drag a slider.
import init, { call } from '../wasm/forge_wasm.js';

const ready = init();

self.onmessage = async (e: MessageEvent<{ id: number; method: string; input: unknown }>) => {
  const { id, method, input } = e.data;
  await ready;
  const t0 = performance.now();
  try {
    const env = JSON.parse(call(method, JSON.stringify(input ?? null))) as { ok?: unknown; error?: string };
    (self as unknown as Worker).postMessage({ id, ok: env.ok ?? null, error: env.error ?? null, ms: performance.now() - t0 });
  } catch (err) {
    (self as unknown as Worker).postMessage({ id, ok: null, error: String(err), ms: performance.now() - t0 });
  }
};
