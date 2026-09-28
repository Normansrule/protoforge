// Bridge to the Rust engine compiled to WebAssembly (crates/forge-wasm).
//
// Every capability is one call:  call('dsp.fft', { signal, sample_rate })
// The same method names work against the optional local Rust server:
//   POST http://localhost:8787/api/dsp.fft
import init, { call as wasmCall, version as wasmVersion } from '../wasm/forge_wasm.js';

let ready: Promise<void> | null = null;
let loaded = false;

export class EngineError extends Error {}

/** Load and instantiate the WebAssembly module (idempotent). */
export function loadEngine(): Promise<void> {
  if (!ready) {
    ready = init().then(() => {
      loaded = true;
    });
  }
  return ready;
}

export function engineLoaded(): boolean {
  return loaded;
}

export function engineVersion(): string {
  return loaded ? wasmVersion() : '…';
}

/** Synchronously call an engine method. Throws EngineError on failure. */
export function call<T = unknown>(method: string, input?: unknown): T {
  if (!loaded) throw new EngineError('engine not loaded yet');
  const raw = wasmCall(method, input === undefined ? '' : JSON.stringify(input));
  const env = JSON.parse(raw) as { ok?: T; error?: string };
  if (env.error !== undefined) throw new EngineError(`${method}: ${env.error}`);
  return env.ok as T;
}

/** Like `call` but returns `{ data, error }` instead of throwing. */
export function tryCall<T = unknown>(method: string, input?: unknown): { data: T | null; error: string | null } {
  try {
    return { data: call<T>(method, input), error: null };
  } catch (e) {
    return { data: null, error: e instanceof Error ? e.message : String(e) };
  }
}
