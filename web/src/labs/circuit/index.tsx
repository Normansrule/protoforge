// Circuit Lab: a SPICE-style simulator (Rust → WebAssembly) with an animated
// schematic, live parameter sliders, waveforms, Bode plots, DC sweeps, an
// operating-point table and an ngspice-compatible netlist editor.
import { useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { LabPage } from '../../components/LabPage';
import { Card, Chip, Code, DownloadButton, ErrorBox, Explainer, Slider, Tabs } from '../../components/ui';
import { Plot, seriesColor, type Series } from '../../components/Plot';
import { Icon } from '../../components/Icon';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { NodeGraph, Schematic } from './Schematic';
import { AcExplainer, DevicesExplainer, MnaExplainer, NewtonExplainer, REFS, TransientExplainer } from './Explainers';
import type { ElementInfo, Measurement, ParseOut, Preset, SimResult } from './types';
import { acCsv, dcCsv, elemVIP, fmtMeas, frameAt, opCsv, parseErrorLines, prefersReducedMotion, RAMP_CSS, si, snapshot, tranCsv, voltageRange } from './util';
import './circuit.css';

const CUSTOM = 'custom';
const DEFAULT_CUSTOM = `My circuit (line 1 is always the title)
* Edit, then Run (Ctrl+Enter). Elements: R C L V I D M E G S.
* Analyses: .op .dc .tran .ac   Parameters: .param name=value, use {name}
.param r=1k
V1 in 0 PULSE(0 5 0 1u 1u 0.5m 1m) AC 1
R1 in out {r}
C1 out 0 100n
D1 out clamp DX
V2 clamp 0 3.3
.model DX D(IS=1e-14)
.tran 5u 3m
.ac dec 20 10 1meg
.end
`;

// ---------------------------------------------------------------------------
// Decoding compact (base64) waveform arrays
// ---------------------------------------------------------------------------

function b64bytes(s: string): ArrayBuffer {
  const bin = atob(s);
  const u8 = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
  return u8.buffer;
}
const f32 = (s: unknown): number[] => (typeof s === 'string' ? Array.from(new Float32Array(b64bytes(s))) : (s as number[]));
const f64 = (s: unknown): number[] => (typeof s === 'string' ? Array.from(new Float64Array(b64bytes(s))) : (s as number[]));

function decode(raw: SimResult | null): SimResult | null {
  if (!raw) return null;
  const r: SimResult = { ...raw };
  if (raw.tran) r.tran = { ...raw.tran, time: f64(raw.tran.time), v: raw.tran.v.map(f32), i: raw.tran.i.map(f32) };
  if (raw.ac) r.ac = { freq: f64(raw.ac.freq), mag: raw.ac.mag.map(f32), phase: raw.ac.phase.map(f32) };
  if (raw.dc) r.dc = { ...raw.dc, sweep: f64(raw.dc.sweep), v: raw.dc.v.map(f32), i: raw.dc.i.map(f32) };
  return r;
}

// ---------------------------------------------------------------------------
// Small pieces
// ---------------------------------------------------------------------------

function Gallery({ presets, value, onChange }: { presets: Preset[]; value: string; onChange: (id: string) => void }) {
  const groups = ['Iris', 'Classic'];
  return (
    <div className="stack" style={{ gap: 10 }}>
      {groups.map((g) => (
        <div key={g}>
          <div className="cl-gtitle">{g === 'Iris' ? 'Iris circuits — verified against requirements' : 'Classic circuits'}</div>
          <div className="cl-gallery">
            {presets.filter((p) => p.group === g).map((p) => (
              <button key={p.id} className={`cl-card ${value === p.id ? 'active' : ''}`} onClick={() => onChange(p.id)} aria-pressed={value === p.id}>
                <div className="cl-thumb"><Schematic layout={p.layout} thumb /></div>
                <div className="cl-ctitle">{p.title}</div>
                <div className="cl-csum">{p.summary}</div>
                {p.metric && <span className="cl-metric">{p.metric.replace('circuit.', '')}</span>}
              </button>
            ))}
            {g === 'Classic' && (
              <button className={`cl-card custom ${value === CUSTOM ? 'active' : ''}`} onClick={() => onChange(CUSTOM)} aria-pressed={value === CUSTOM}>
                <div className="cl-thumb cl-thumb-code"><pre>{'V1 in 0 PULSE(…)\nR1 in out 1k\nC1 out 0 100n\n.tran 5u 3m'}</pre></div>
                <div className="cl-ctitle">Custom netlist</div>
                <div className="cl-csum">Paste any SPICE deck — the same file runs in ngspice.</div>
              </button>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}

function MeasurementTiles({ items }: { items: Measurement[] }) {
  if (items.length === 0) return <div className="muted small">No measurements for this circuit.</div>;
  return (
    <div className="cl-meas">
      {items.map((m) => (
        <div key={m.key} className={`cl-mtile ${m.pass === true ? 'pass' : m.pass === false ? 'fail' : ''}`}>
          <div className="cl-mlabel">{m.label}</div>
          <div className="cl-mvalue tnum">
            {fmtMeas(m.value, m.unit)}
            {m.pass === true && <Chip tone="pass">✓ pass</Chip>}
            {m.pass === false && <Chip tone="fail">✕ check</Chip>}
          </div>
          {m.note && <div className="cl-mnote">{m.note}</div>}
        </div>
      ))}
    </div>
  );
}

function NetlistEditor({ value, onChange, onRun, errorLines, running }: { value: string; onChange: (v: string) => void; onRun: () => void; errorLines: Map<number, string>; running?: boolean }) {
  const gutter = useRef<HTMLDivElement>(null);
  const lines = value.split('\n').length;
  return (
    <div className="cl-editor">
      <div className="cl-gutter" ref={gutter} aria-hidden>
        {Array.from({ length: lines }, (_, k) => (
          <div key={k} className={errorLines.has(k + 1) ? 'err' : ''} title={errorLines.get(k + 1)}>{k + 1}</div>
        ))}
      </div>
      <textarea
        value={value}
        spellCheck={false}
        wrap="off"
        aria-label="SPICE netlist"
        onChange={(e) => onChange(e.target.value)}
        onScroll={(e) => { if (gutter.current) gutter.current.scrollTop = e.currentTarget.scrollTop; }}
        onKeyDown={(e) => { if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) { e.preventDefault(); onRun(); } }}
        rows={Math.min(28, Math.max(12, lines + 1))}
      />
      {running && <div className="cl-running">running…</div>}
    </div>
  );
}

function Inspector({ el, res, frame }: { el: ElementInfo | null; res: SimResult | null; frame: number | null }) {
  if (!el || !res) return <div className="cl-inspect muted small">Hover or tap a part to see its voltage, current and power.</div>;
  const s = snapshot(res, frame);
  const { v, i, p } = elemVIP(el, s);
  const k = res.elements.findIndex((e) => e.name === el.name);
  const note = res.op?.elem_note[k];
  return (
    <div className="cl-inspect">
      <b>{el.name}</b>
      <span className="muted">{el.kind} · {el.value}</span>
      <span className="muted">nodes {el.nodes.join(', ')}</span>
      <span className="tnum">V = {si(v, 'V', 4)}</span>
      <span className="tnum">I = {si(i, 'A', 4)}</span>
      <span className="tnum">P = {si(p, 'W', 3)}{p < 0 ? ' (delivering)' : ''}</span>
      {note && <span className="muted">DC: {note}</span>}
    </div>
  );
}

function probeSeries(res: SimResult, probes: string[], kind: 'v' | 'i', win: [number, number] | null): Series[] {
  const tr = res.tran!;
  let a = 0, b = tr.time.length;
  if (win) {
    a = Math.max(0, frameAt(tr.time, win[0]));
    b = Math.min(tr.time.length, frameAt(tr.time, win[1]) + 2);
  }
  const x = tr.time.slice(a, b);
  return probes
    .map((p, k) => ({ p, k }))
    .filter(({ p }) => p.startsWith(kind + '('))
    .map(({ p, k }) => {
      const name = p.slice(2, -1).toLowerCase();
      const arr = kind === 'v' ? tr.v[res.nodes.indexOf(name)] : tr.i[res.elements.findIndex((e) => e.name.toLowerCase() === name)];
      return arr ? { name: p, x, y: arr.slice(a, b), color: seriesColor(k) } : null;
    })
    .filter(Boolean) as Series[];
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

type Tab = 'wave' | 'ac' | 'dc' | 'op' | 'netlist';

export default function Lab() {
  const project = useProject((s) => s.project);
  const presetsC = useCompute<Preset[]>('circuit.presets', { project });
  const presets = useMemo(() => presetsC.data ?? [], [presetsC.data]);
  const [id, setId] = useState('rail_load_step');
  const [params, setParams] = useState<Record<string, Record<string, number>>>({});
  const [edited, setEdited] = useState<Record<string, string>>({});
  const [draft, setDraft] = useState<string | null>(null);
  const [customRun, setCustomRun] = useState(DEFAULT_CUSTOM);
  const [probeSel, setProbeSel] = useState<Record<string, string[]>>({});
  const [tab, setTab] = useState<Tab>('wave');
  const [hover, setHover] = useState<string | null>(null);
  const [picked, setPicked] = useState<string | null>(null);
  const [frame, setFrame] = useState<number | null>(null);
  const [playing, setPlaying] = useState(false);
  const [duration, setDuration] = useState(8);
  const [useWin, setUseWin] = useState(true);

  const preset = presets.find((p) => p.id === id) ?? null;
  const isCustom = id === CUSTOM;
  const myParams = params[id] ?? {};

  const method = isCustom ? 'circuit.simulate' : 'circuit.run_preset';
  const input = isCustom
    ? { netlist: customRun, compact: true }
    : { id, params: myParams, netlist: edited[id], project, compact: true };
  const sim = useCompute<SimResult>(method, input, isCustom || presets.length > 0);
  const [last, setLast] = useState<SimResult | null>(null);
  const decoded = useMemo(() => decode(sim.data), [sim.data]);
  useEffect(() => {
    if (decoded) setLast(decoded);
  }, [decoded]);
  const res = decoded ?? (sim.error ? last : null);

  // Probes: preset defaults, or every node for custom decks.
  const defaultProbes = useMemo(() => {
    if (preset && !isCustom) return preset.probes;
    return (res?.nodes ?? []).filter((n) => !n.includes('#')).slice(0, 8).map((n) => `v(${n})`);
  }, [preset, isCustom, res?.nodes]);
  const probes = probeSel[id] ?? defaultProbes;
  const toggleProbe = (p: string) => setProbeSel((s) => {
    const cur = s[id] ?? defaultProbes;
    return { ...s, [id]: cur.includes(p) ? cur.filter((x) => x !== p) : [...cur, p] };
  });

  // Layout applies when every drawn element still exists in the (possibly edited) deck.
  const layoutOk = useMemo(() => {
    if (!preset || isCustom || !res) return false;
    const names = new Set(res.elements.map((e) => e.name.toLowerCase()));
    return preset.layout.parts.every((p) => p.members.every((m) => names.has(m.toLowerCase())));
  }, [preset, isCustom, res]);

  const tr = res?.tran ?? null;
  const win: [number, number] | null = tr ? (useWin && res?.tran_window ? res.tran_window : [tr.time[0], tr.time[tr.time.length - 1]]) : null;

  // reset view when switching circuits
  const firstFrameFor = useRef<string>('');
  useEffect(() => {
    if (!res) return;
    const key = `${id}`;
    if (firstFrameFor.current !== key) {
      firstFrameFor.current = key;
      setHover(null);
      setPicked(null);
      if (res.tran) {
        const w = res.tran_window ?? [res.tran.time[0], res.tran.time[res.tran.time.length - 1]];
        setFrame(frameAt(res.tran.time, w[0]));
        setPlaying(!prefersReducedMotion());
      } else {
        setFrame(null);
        setPlaying(false);
      }
      setTab(res.tran ? 'wave' : res.ac ? 'ac' : res.dc ? 'dc' : 'op');
    } else if (res.tran && frame !== null && frame >= res.tran.time.length) {
      setFrame(res.tran.time.length - 1);
    }
  }, [res, id, frame]);

  // playback loop
  const simT = useRef(0);
  useEffect(() => {
    if (!playing || !tr || !win) return;
    const [w0, w1] = win;
    const span = w1 - w0;
    simT.current = frame !== null && tr.time[frame] >= w0 && tr.time[frame] <= w1 ? tr.time[frame] : w0;
    let lastTs = performance.now();
    let lastSet = 0;
    let raf = 0;
    const step = (now: number) => {
      const dt = (now - lastTs) / 1000;
      lastTs = now;
      simT.current += (span / duration) * dt;
      if (simT.current > w1) simT.current = w0;
      if (now - lastSet > 33) {
        lastSet = now;
        setFrame(frameAt(tr.time, simT.current));
      }
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [playing, tr, win?.[0], win?.[1], duration]);

  const snap = useMemo(() => snapshot(res, tr ? frame : null), [res, tr, frame]);
  const layoutNodes = useMemo(() => (preset && layoutOk ? [...new Set(preset.layout.wires.map((w) => w.node))] : res?.nodes ?? []), [preset, layoutOk, res]);
  const vrange = useMemo(() => voltageRange(res, layoutNodes), [res, layoutNodes]);
  const imax = useMemo(() => {
    if (!res) return 0;
    let m = 0;
    if (res.tran) for (const col of res.tran.i) for (const x of col) m = Math.max(m, Math.abs(x));
    else if (res.op) for (const x of res.op.elem_i) m = Math.max(m, Math.abs(x));
    return m;
  }, [res]);

  // editor state
  const effectiveDeck = isCustom ? customRun : edited[id] ?? res?.netlist ?? preset?.netlist ?? '';
  const editorText = draft ?? effectiveDeck;
  const parse = useCompute<ParseOut>('circuit.parse', { netlist: editorText, params: isCustom ? {} : myParams }, tab === 'netlist' || isCustom);
  const liveErrors = parse.data && !parse.data.ok ? parse.data.errors : [];
  const runErrors = parseErrorLines(sim.error);
  const errorLines = new Map<number, string>();
  for (const e of [...liveErrors, ...runErrors]) if (e.line > 0) errorLines.set(e.line, e.message);
  const runDraft = () => {
    if (isCustom) setCustomRun(editorText);
    else setEdited((s) => ({ ...s, [id]: editorText }));
    setDraft(null);
  };
  const resetDeck = () => {
    if (isCustom) setCustomRun(DEFAULT_CUSTOM);
    else setEdited((s) => { const n = { ...s }; delete n[id]; return n; });
    setDraft(null);
  };

  const select = (nid: string) => {
    setId(nid);
    setDraft(null);
  };

  const inspectEl = res?.elements.find((e) => e.name.toLowerCase() === (hover ?? picked ?? '').toLowerCase()) ?? null;
  const fileBase = isCustom ? 'custom' : id;
  const tranAnn = res?.annotations.filter((a) => a.plot === 'tran') ?? [];
  const frameOk = tr && frame !== null && frame < tr.time.length ? frame : null;
  const nowLine = tr && frameOk !== null ? [{ value: tr.time[frameOk], label: '', color: 'var(--accent)' }] : [];

  const voltSeries = res && tr && win ? probeSeries(res, probes, 'v', win) : [];
  const curSeries = res && tr && win ? probeSeries(res, probes, 'i', win) : [];

  const acSeries = (kind: 'mag' | 'phase'): Series[] => {
    if (!res?.ac) return [];
    const nodesSel = probes.filter((p) => p.startsWith('v(')).map((p) => p.slice(2, -1).toLowerCase());
    const list = nodesSel.length ? nodesSel : res.nodes.slice(0, 4);
    return list
      .map((n, k) => {
        const j = res.nodes.indexOf(n);
        if (j < 0) return null;
        const y = kind === 'mag' ? res.ac!.mag[j].map((m) => 20 * Math.log10(Math.max(m, 1e-30))) : res.ac!.phase[j];
        return { name: `v(${n})`, x: res.ac!.freq, y, color: seriesColor(probes.indexOf(`v(${n})`) >= 0 ? probes.indexOf(`v(${n})`) : k) };
      })
      .filter(Boolean) as Series[];
  };
  const acV = (res?.annotations ?? []).filter((a) => a.plot === 'ac').map((a) => ({ value: a.value, label: a.label }));
  const dcSeries: Series[] = res?.dc
    ? probes.filter((p) => p.startsWith('v(')).map((p) => {
      const j = res.nodes.indexOf(p.slice(2, -1).toLowerCase());
      return j >= 0 ? { name: p, x: res.dc!.sweep, y: res.dc!.v[j], color: seriesColor(probes.indexOf(p)) } : null;
    }).filter(Boolean) as Series[]
    : [];

  const tabs: { id: Tab; label: string }[] = [];
  if (tr) tabs.push({ id: 'wave', label: 'Waveforms' });
  if (res?.ac) tabs.push({ id: 'ac', label: 'Bode (AC)' });
  if (res?.dc) tabs.push({ id: 'dc', label: 'DC sweep' });
  tabs.push({ id: 'op', label: 'Operating point' }, { id: 'netlist', label: 'Netlist' });
  const tabNow = tabs.some((t) => t.id === tab) ? tab : tabs[0].id;

  const editorPanel = (
          <div className="stack" style={{ gap: 10 }}>
            <NetlistEditor value={editorText} onChange={setDraft} onRun={runDraft} errorLines={errorLines} />
            <div className="row">
              <button className="btn primary" onClick={runDraft}><Icon name="play" size={13} /> Run</button>
              <button className="btn" onClick={resetDeck}><Icon name="reset" size={13} /> Reset to {isCustom ? 'example' : 'preset'}</button>
              <DownloadButton filename={`${fileBase}.cir`} content={() => editorText} label="download .cir" />
              {draft !== null && <Chip tone="warn">unsaved edits — Run to simulate</Chip>}
              {parse.data?.ok && <Chip tone="pass" title={parse.data.analyses.join('  ')}>✓ parses: {parse.data.elements.length} elements, {parse.data.nodes.length} nodes</Chip>}
            </div>
            {liveErrors.length > 0 && (
              <div className="callout" style={{ borderLeftColor: 'var(--bad)' }}>
                {liveErrors.map((e, k) => <div key={k} className="small" style={{ color: 'var(--bad-text)' }}>{e.line > 0 ? `line ${e.line}: ` : ''}{e.message}</div>)}
              </div>
            )}
            {!isCustom && !layoutOk && res && <div className="small muted">The edited deck no longer matches the hand-drawn schematic, so the node graph is shown instead.</div>}
          </div>
  );

  const title = isCustom ? 'Custom netlist' : preset?.title ?? 'Loading…';
  const stats = res?.tran?.stats;

  return (
    <LabPage
      kicker="Electronics · SPICE in your browser"
      title="Circuit Lab"
      subtitle={<>A small SPICE written in Rust and compiled to WebAssembly: Modified Nodal Analysis, Newton–Raphson with junction limiting, trapezoidal transients with breakpoints and error control, and AC small-signal analysis. Pick a circuit, drag the sliders and watch the charge move.</>}
      metricPrefixes={['circuit.']}
      actions={res ? (
        <>
          <DownloadButton filename={`${fileBase}.cir`} content={() => (isCustom ? customRun : res.netlist)} label=".cir netlist" />
          {res.tran && <DownloadButton filename={`${fileBase}-tran.csv`} content={() => tranCsv(res)} label="CSV" />}
        </>
      ) : undefined}
    >
      <Gallery presets={presets} value={id} onChange={select} />

      <div className="cl-top">
        <div className="stack">
        <Card
          className="cl-stage"
          title={title}
          subtitle={isCustom ? 'Your deck, drawn as a node graph (presets come with hand-drawn schematics).' : preset?.summary}
          actions={res && (
            <div className="row small tnum" style={{ gap: 6 }}>
              <Chip title="engine call time, WebAssembly">{sim.ms.toFixed(1)} ms</Chip>
              <Chip title="MNA unknowns (node voltages + branch currents)">{res.stats.unknowns} unknowns</Chip>
            </div>
          )}
        >
          {sim.error && <ErrorBox error={sim.error.replace(/^circuit\.\w+:\s*/, '')} />}
          {res && (
            <>
              <div className="cl-canvas">
                {layoutOk && preset ? (
                  <Schematic layout={preset.layout} elements={res.elements} snap={snap} vrange={vrange} imax={imax} animate hovered={hover ?? picked} onHover={setHover} onPick={(e) => setPicked((p) => (p === e ? null : e))} maxHeight={520} />
                ) : (
                  <NodeGraph elements={res.elements} nodes={res.nodes} snap={snap} vrange={vrange} imax={imax} animate hovered={hover} onHover={setHover} />
                )}
              </div>
              <div className="cl-legend">
                <span className="small muted">node voltage</span>
                <span className="tnum small">{si(vrange[0], 'V', 3)}</span>
                <span className="cl-ramp" style={{ background: RAMP_CSS }} />
                <span className="tnum small">{si(vrange[1], 'V', 3)}</span>
                <span className="small muted cl-legend-dots"><span className="cl-dotkey" /> dots = current (speed ∝ log |I|, direction = sign)</span>
              </div>
              {tr && (
                <div className="cl-scrub">
                  <button className="btn small" onClick={() => setPlaying((p) => !p)} aria-label={playing ? 'pause' : 'play'}>
                    {playing ? '❚❚ Pause' : <><Icon name="play" size={13} /> Play</>}
                  </button>
                  <input
                    type="range"
                    min={0}
                    max={tr.time.length - 1}
                    value={frameOk ?? 0}
                    onChange={(e) => { setPlaying(false); setFrame(Number(e.target.value)); }}
                    aria-label="simulation time"
                  />
                  <span className="tnum small cl-tnow">t = {si(tr.time[frameOk ?? 0], 's', 4)}</span>
                  <select value={duration} onChange={(e) => setDuration(Number(e.target.value))} aria-label="playback speed" className="cl-speed">
                    <option value={3}>fast</option>
                    <option value={8}>normal</option>
                    <option value={20}>slow</option>
                    <option value={60}>very slow</option>
                  </select>
                </div>
              )}
              {!tr && <div className="small muted" style={{ marginTop: 6 }}>Showing the DC operating point (this deck has no .tran).</div>}
              <Inspector el={inspectEl} res={res} frame={tr ? frame : null} />
            </>
          )}
          {!res && !sim.error && <div className="muted small" style={{ padding: 30, textAlign: 'center' }}>Starting the engine…</div>}
        </Card>
          <Card title="Measurements" subtitle={isCustom ? 'Presets add .meas-style readouts; custom decks show the waveforms.' : 'Computed from the simulated waveforms.'}>
            <MeasurementTiles items={res?.measurements ?? []} />
            {res && res.warnings.length > 0 && (
              <details className="small" style={{ marginTop: 10 }}>
                <summary className="muted">{res.warnings.length} solver note{res.warnings.length > 1 ? 's' : ''}</summary>
                <ul className="cl-warn">{res.warnings.map((w, k) => <li key={k}>{w}</li>)}</ul>
              </details>
            )}
          </Card>
        </div>

        <div className="stack">
          {isCustom && <Card title="Your deck" subtitle="Ctrl+Enter runs it. Errors are reported with line numbers.">{editorPanel}</Card>}
          {!isCustom && preset && (
            <Card title="Parameters" subtitle="Every slider rewrites a .param card and re-simulates.">
              {preset.params.map((p) => (
                <Slider
                  key={p.name}
                  label={p.label}
                  value={myParams[p.name] ?? p.value}
                  min={p.min}
                  max={p.max}
                  log={p.log}
                  unit={p.unit}
                  format={(v) => si(v, '', 3)}
                  onChange={(v) => setParams((s) => ({ ...s, [id]: { ...(s[id] ?? {}), [p.name]: v } }))}
                />
              ))}
              <div className="row">
                <button className="btn small" onClick={() => setParams((s) => ({ ...s, [id]: {} }))}><Icon name="reset" size={13} /> Defaults</button>
                {edited[id] && <Chip tone="warn">edited deck</Chip>}
              </div>
            </Card>
          )}
          {!isCustom && preset && (
            <Card title="Why Iris needs this circuit">
              <p style={{ marginTop: 0 }} className="small">{preset.description}</p>
              <div className="callout" style={{ borderLeftColor: 'var(--accent-2)' }}>
                <h4>Iris tie-in</h4>
                {preset.iris}
              </div>
              {preset.links.length > 0 && (
                <div className="row" style={{ marginTop: 10 }}>
                  {preset.links.map((l) => (
                    <Link key={l.lab} to={`/${l.lab}`} className="btn small">{l.label} <Icon name="arrow" size={13} /></Link>
                  ))}
                </div>
              )}
            </Card>
          )}
        </div>
      </div>


      <Card
        title="Results"
        actions={res && (
          <div className="row" style={{ gap: 6 }}>
            {tabNow === 'wave' && res.tran && <DownloadButton small filename={`${fileBase}-tran.csv`} content={() => tranCsv(res)} label="tran.csv" />}
            {tabNow === 'ac' && res.ac && <DownloadButton small filename={`${fileBase}-ac.csv`} content={() => acCsv(res)} label="ac.csv" />}
            {tabNow === 'dc' && res.dc && <DownloadButton small filename={`${fileBase}-dc.csv`} content={() => dcCsv(res)} label="dc.csv" />}
            {tabNow === 'op' && res.op && <DownloadButton small filename={`${fileBase}-op.csv`} content={() => opCsv(res)} label="op.csv" />}
            <DownloadButton small filename={`${fileBase}-results.json`} content={() => JSON.stringify(res)} label="json" />
          </div>
        )}
      >
        <Tabs tabs={tabs} value={tabNow} onChange={setTab} />
        {res && (tabNow === 'wave' || tabNow === 'ac' || tabNow === 'dc') && (
          <div className="cl-probes">
            <span className="small muted">Probes</span>
            {res.nodes.filter((n) => !n.includes('#')).map((n) => {
              const p = `v(${n})`;
              const on = probes.includes(p);
              return <button key={p} className={`cl-probe ${on ? 'on' : ''}`} style={on ? { borderColor: seriesColor(probes.indexOf(p)) } : undefined} onClick={() => toggleProbe(p)} aria-pressed={on}>{p}</button>;
            })}
            {tabNow === 'wave' && res.elements.map((e) => {
              const p = `i(${e.name})`;
              const on = probes.includes(p);
              return <button key={p} className={`cl-probe cur ${on ? 'on' : ''}`} style={on ? { borderColor: seriesColor(probes.indexOf(p)) } : undefined} onClick={() => toggleProbe(p)} aria-pressed={on}>{p}</button>;
            })}
          </div>
        )}
        {res && tabNow === 'wave' && tr && (
          <div className="stack" style={{ gap: 8 }}>
            {res.tran_window && (
              <div className="row small">
                <span className="muted">View</span>
                <button className={`btn small ${useWin ? 'primary' : ''}`} onClick={() => setUseWin(true)}>zoom {si(res.tran_window[0], 's', 3)} – {si(res.tran_window[1], 's', 3)}</button>
                <button className={`btn small ${!useWin ? 'primary' : ''}`} onClick={() => setUseWin(false)}>full run</button>
              </div>
            )}
            {voltSeries.length > 0 && (
              <Plot series={voltSeries} height={280} xLabel="time (s)" yLabel="voltage (V)" vLines={[...nowLine, ...tranAnn.filter((a) => a.axis === 'x').map((a) => ({ value: a.value, label: a.label }))]} hLines={tranAnn.filter((a) => a.axis === 'y').map((a) => ({ value: a.value, label: a.label, color: 'var(--bad)' }))} xDomain={win ?? undefined} legend />
            )}
            {curSeries.length > 0 && <Plot series={curSeries} height={220} xLabel="time (s)" yLabel="current (A)" vLines={nowLine} xDomain={win ?? undefined} legend />}
            {voltSeries.length === 0 && curSeries.length === 0 && <div className="muted small">Pick a probe above.</div>}
            <div className="small muted">
              Currents follow the SPICE convention: positive <i>into</i> the first node (n+) or the drain, so a source that delivers power reads negative.
              {stats && <> · {stats.accepted} time points ({stats.rejected_lte} steps rejected by the error estimate, {stats.rejected_nr} by Newton), step {si(stats.min_step, 's', 2)} … {si(stats.max_step, 's', 2)}, {stats.breakpoints} breakpoints{stats.uic ? ', started from initial conditions (UIC)' : ''}.</>}
            </div>
          </div>
        )}
        {res && tabNow === 'ac' && res.ac && (
          <div className="stack" style={{ gap: 8 }}>
            <Plot series={acSeries('mag')} height={260} xLog xLabel="frequency (Hz)" yLabel="magnitude (dB)" vLines={acV} legend />
            <Plot series={acSeries('phase')} height={200} xLog xLabel="frequency (Hz)" yLabel="phase (°)" vLines={acV} legend={false} />
            <div className="small muted">AC magnitudes are relative to the source's AC amplitude (usually 1 V), so they read directly as gain.</div>
          </div>
        )}
        {res && tabNow === 'dc' && res.dc && (
          <Plot
            series={dcSeries}
            height={300}
            xLabel={`${res.dc.source} (V)`}
            yLabel="voltage (V)"
            vLines={(res.annotations ?? []).filter((a) => a.plot === 'dc').map((a) => ({ value: a.value, label: a.label }))}
            legend
          />
        )}
        {res && tabNow === 'op' && res.op && (
          <div className="grid grid-2">
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Node</th><th>Voltage</th></tr></thead>
                <tbody>{res.nodes.map((n, k) => <tr key={n}><td className="mono">{n}</td><td className="tnum">{si(res.op!.node_v[k], 'V', 5)}</td></tr>)}</tbody>
              </table>
              <div className="small muted" style={{ marginTop: 8 }}>
                Solved by {res.op.method} in {res.op.iterations} iteration{res.op.iterations === 1 ? '' : 's'} · {res.stats.unknowns} unknowns ({res.stats.nodes} nodes + {res.stats.branches} branch currents) · {res.stats.lu_solves} LU solves in total.
              </div>
            </div>
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Element</th><th>V</th><th>I</th><th>P</th></tr></thead>
                <tbody>
                  {res.elements.map((e, k) => (
                    <tr key={e.name} onMouseEnter={() => setHover(e.name)} onMouseLeave={() => setHover(null)}>
                      <td><span className="mono">{e.name}</span> <span className="muted small">{e.value}</span>{res.op!.elem_note[k] && <div className="small muted">{res.op!.elem_note[k]}</div>}</td>
                      <td className="tnum">{si(res.op!.elem_v[k], 'V', 4)}</td>
                      <td className="tnum">{si(res.op!.elem_i[k], 'A', 4)}</td>
                      <td className="tnum">{si(res.op!.elem_p[k], 'W', 3)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
        {res && tabNow === 'op' && !res.op && <div className="muted small">No DC operating point for this deck (see solver notes).</div>}
        {tabNow === 'netlist' && editorPanel}
      </Card>

      <div className="grid grid-2">
        <Card title="Netlist syntax (SPICE subset)">
          <p style={{ marginTop: 0 }}>Line 1 is the title. Values take SPICE suffixes — <code>f p n u m k meg g t</code> and <code>mil</code>; careful: <code>1M</code> is one <i>milli</i>, write <code>1meg</code> for a million. Node <code>0</code> (or <code>gnd</code>) is ground.</p>
          <Code code={`R<name> n+ n- value
C<name> n+ n- value [IC=v]      L<name> n+ n- value [IC=i]
V<name> n+ n- [DC v] [AC mag [phase]] [PULSE(v1 v2 td tr tf pw per) | SIN(vo va f td theta) | PWL(t1 v1 ...)]
I<name> n+ n- ...same as V
D<name> a k model [area]         .model DX D(IS= N= RS= CJO= TT= BV=)
M<name> d g s b model W= L=      .model NM NMOS(VTO= KP= LAMBDA= GAMMA=)
E<name> n+ n- nc+ nc- gain       G<name> n+ n- nc+ nc- gm
S<name> n+ n- nc+ nc- model      .model SW1 SW(RON= ROFF= VT= VH=)
.op  .dc src start stop step  .tran tstep tstop [tstart [tmax]] [uic]
.ac dec|oct|lin n fstart fstop  .ic v(node)=val  .param name=value  .end`} filename="syntax.txt" />
        </Card>
        <Card title="Cross-check in ngspice">
          <p style={{ marginTop: 0 }}>
            Every deck here is plain SPICE. Download the <code>.cir</code> and run the same file in <a href="https://ngspice.sourceforge.io/" target="_blank" rel="noreferrer">ngspice</a>, the open-source
            descendant of Berkeley SPICE — the answers should agree to within the solvers' tolerances.
          </p>
          <Code code={`ngspice -b ${fileBase}.cir            # batch run: prints .op results
ngspice -b -r ${fileBase}.raw ${fileBase}.cir   # save all waveforms to a rawfile
ngspice ${fileBase}.cir               # interactive, then:  run   plot v(${(probes.find((p) => p.startsWith('v(')) ?? 'v(out)').slice(2, -1)})`} filename="ngspice.sh" />
          <p className="small muted">
            When you outgrow this lab: <b>ngspice</b> (BSIM transistor models, sub-circuits, behavioural sources), <b>Xyce</b> (Sandia's parallel SPICE for very large circuits),
            <b> Qucs-S</b> (schematic front end for ngspice/Xyce) and <b>KiCad</b>'s built-in ngspice simulator, straight from your schematic.
          </p>
        </Card>
      </div>

      <h2 style={{ marginTop: 8 }}>How the simulator works</h2>
      <div className="grid grid-2 cl-explain">
        <MnaExplainer />
        <NewtonExplainer />
        <TransientExplainer />
        <div className="stack">
          <AcExplainer />
          <DevicesExplainer />
          <Explainer title="Further reading" refs={[REFS.nagel, REFS.ho, REFS.sh, REFS.vlach, REFS.ngspice]}>
            SPICE's algorithms have barely changed since Nagel's thesis: MNA assembly, sparse LU, Newton–Raphson with limiting, and
            trapezoidal/Gear integration with truncation-error control. This lab keeps the matrices dense (fine for tens of nodes) so every step stays readable.
          </Explainer>
        </div>
      </div>
    </LabPage>
  );
}
