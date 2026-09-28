// "How it works": the one-function JSON API that joins the React UI, the
// WebAssembly bridge and the Rust engine, plus a live console that calls it.
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { call, engineVersion, tryCall } from '../../engine/engine';
import { LABS } from '../../labs/registry';
import { METHOD_CATALOG } from './methods';

const EXAMPLES: { method: string; input: string; note: string }[] = [
  { method: 'pcb.ipc2221', input: '{ "current_a": 0.5, "temp_rise_c": 10, "copper_oz": 1, "external": true }', note: 'IPC-2221 trace width for a current' },
  { method: 'project.blank', input: '{ "name": "My robot lamp" }', note: 'a fresh product template' },
  { method: 'mfg.assumptions', input: '', note: 'the cost model’s default assumptions' },
  { method: 'api.methods', input: '', note: 'every engine module' },
];

/** Module → method names: from the engine when it offers `api.catalog`, else the static snapshot. */
function useCatalog(modules: string[]): Record<string, string[]> {
  return useMemo(() => {
    const live = tryCall<Record<string, string[]>>('api.catalog');
    if (live.data && typeof live.data === 'object' && !Array.isArray(live.data)) return live.data;
    const out: Record<string, string[]> = {};
    for (const m of modules) out[m] = METHOD_CATALOG[m] ?? [];
    return out;
  }, [modules]);
}

function short(json: string, max = 2400): string {
  return json.length > max ? `${json.slice(0, max)}\n… (${(json.length / 1024).toFixed(1)} kB total)` : json;
}

export function Architecture() {
  const modules = useMemo(() => tryCall<string[]>('api.methods').data ?? [], []);
  const catalog = useCatalog(modules);
  const methodCount = Object.values(catalog).reduce((a, v) => a + v.length, 0);
  const [open, setOpen] = useState<string | null>('dsp');

  const [ex, setEx] = useState(0);
  const [input, setInput] = useState(EXAMPLES[0].input);
  const [out, setOut] = useState<{ text: string; ms: number; ok: boolean } | null>(null);
  const run = (method = EXAMPLES[ex].method, text = input) => {
    const t0 = performance.now();
    try {
      const v = call(method, text.trim() ? JSON.parse(text) : undefined);
      setOut({ text: short(JSON.stringify({ ok: v }, null, 2)), ms: performance.now() - t0, ok: true });
    } catch (e) {
      setOut({ text: JSON.stringify({ error: e instanceof Error ? e.message : String(e) }, null, 2), ms: performance.now() - t0, ok: false });
    }
  };

  return (
    <div className="hm-arch">
      <div className="hm-arch-diagram" role="img" aria-label="Architecture: React user interface calls one JSON function, bridged by WebAssembly to the Rust engine; the command-line tool and the REST server call the same engine">
        <div className="hm-tier hm-tier-ui">
          <div className="hm-tier-kicker">web/ · React 19 + TypeScript</div>
          <div className="hm-tier-title">Browser user interface</div>
          <div className="hm-tier-body">{LABS.length} labs · Systems Studio · Three.js viewers · SVG plots</div>
          <code className="hm-code">useCompute('dsp.fir_design', input)</code>
        </div>
        <div className="hm-link" aria-hidden>
          <span className="hm-link-line" />
          <span className="hm-link-label">JSON in · JSON out</span>
        </div>
        <div className="hm-tier hm-tier-bridge">
          <div className="hm-tier-kicker">crates/forge-wasm · wasm-bindgen</div>
          <div className="hm-tier-title">WebAssembly bridge</div>
          <div className="hm-tier-body">one export: <code>call(method, json) → {'{ok}'} | {'{error}'}</code> — synchronous, no server</div>
        </div>
        <div className="hm-link" aria-hidden>
          <span className="hm-link-line" />
          <span className="hm-link-label">api::call_value</span>
        </div>
        <div className="hm-tier hm-tier-core">
          <div className="hm-tier-kicker">crates/forge-core · Rust · serde only</div>
          <div className="hm-tier-title">Engine: {modules.length} modules · {methodCount} methods</div>
          <div className="hm-mods">
            {modules.map((m) => (
              <button key={m} className={`hm-mod ${open === m ? 'on' : ''}`} onClick={() => setOpen(open === m ? null : m)} aria-expanded={open === m}>
                {m}
                <span className="hm-mod-n">{catalog[m]?.length ?? 0}</span>
              </button>
            ))}
          </div>
          {open && (
            <div className="hm-methods mono" aria-live="polite">
              {(catalog[open] ?? []).length === 0 ? (
                <span className="muted">{open} contributes verification metrics only</span>
              ) : (
                catalog[open].map((v) => <span key={v}>{open}.{v}</span>)
              )}
            </div>
          )}
        </div>
        <div className="hm-siblings">
          <div className="hm-tier hm-tier-side">
            <div className="hm-tier-kicker">crates/forge-cli</div>
            <div className="hm-tier-title">protoforge command line</div>
            <code className="hm-code">protoforge evaluate · package · call</code>
          </div>
          <div className="hm-tier hm-tier-side">
            <div className="hm-tier-kicker">crates/forge-server · axum</div>
            <div className="hm-tier-title">Local REST server</div>
            <code className="hm-code">POST /api/systems.evaluate</code>
          </div>
        </div>
      </div>

      <div className="hm-console card">
        <div className="card-head">
          <div>
            <h3>Try the engine</h3>
            <div className="card-sub">Runs in this tab — engine v{engineVersion()} compiled to WebAssembly.</div>
          </div>
        </div>
        <div className="hm-ex">
          {EXAMPLES.map((e, i) => (
            <button
              key={e.method}
              className={`hm-ex-btn ${ex === i ? 'on' : ''}`}
              onClick={() => {
                setEx(i);
                setInput(e.input);
                run(e.method, e.input);
              }}
              title={e.note}
            >
              {e.method}
            </button>
          ))}
        </div>
        <label className="small muted" htmlFor="hm-input">input for <code>{EXAMPLES[ex].method}</code> — {EXAMPLES[ex].note}</label>
        <textarea id="hm-input" rows={3} value={input} onChange={(e) => setInput(e.target.value)} spellCheck={false} placeholder="(no input)" />
        <div className="row" style={{ marginTop: 8 }}>
          <button className="btn primary small" onClick={() => run()}>▶ call('{EXAMPLES[ex].method}')</button>
          {out && <span className={`chip ${out.ok ? 'pass' : 'fail'}`}>{out.ok ? '✓ ok' : '✕ error'} · {out.ms < 1 ? '<1' : out.ms.toFixed(1)} ms</span>}
        </div>
        {out && <pre className="code hm-out">{out.text}</pre>}
        <div className="small muted" style={{ marginTop: 8 }}>
          The same call from a shell: <code>protoforge call {EXAMPLES[ex].method}{EXAMPLES[ex].input ? ` '${EXAMPLES[ex].input.replace(/\s+/g, ' ')}'` : ''}</code> — or over HTTP with the REST server. <Link to="/learn">Architecture notes →</Link>
        </div>
      </div>
    </div>
  );
}
