// Circuit Lab · Schematic editor: an EasyEDA/Falstad-style drawing canvas
// (snap grid, palette, orthogonal wires with junction dots, rubber-banding,
// pan/zoom, undo/redo, property editor) whose drawing is turned into a SPICE
// netlist by connectivity extraction in the engine (circuit.schematic_*),
// simulated by the same engine as the rest of the lab, and animated with
// current flowing along the wires exactly as drawn.
import { useCallback, useEffect, useMemo, useRef, useState, type PointerEvent as RPointerEvent } from 'react';
import { Card, Chip, Code, DownloadButton, ErrorBox, Explainer, Formula, Tabs, downloadText } from '../../components/ui';
import { Icon } from '../../components/Icon';
import { useCompute } from '../../engine/hooks';
import { tryCall } from '../../engine/engine';
import { useElementWidth } from '../../components/useElementWidth';
import { useProject } from '../../state/project';
import { REFS } from './Explainers';
import { SchResults, decodeSim } from './SchResults';
import { labelPos, SymbolBody, valueText } from './SchSymbols';
import { DEFAULT_ANALYSIS, EMPTY_DOC, PALETTE, pinsOf, spiceNum, type Extracted, type SchDoc, type SchExample, type SchPart, type SchSimOut, type SchWire } from './schTypes';
import type { Preset, SimResult } from './types';
import { elemVIP, frameAt, prefersReducedMotion, rampColor, si, snapshot, switchOn, voltageRange } from './util';
import './schematic.css';

const PX = 20; // px per grid unit at zoom 1
const STORE = 'protoforge.schematic.doc';
type Tool = { t: 'select' } | { t: 'wire' } | { t: 'place'; kind: string; rot: number };
type Sel = { type: 'part'; id: string } | { type: 'wire'; idx: number } | null;
type Pt = [number, number];

const snap = (v: number) => Math.round(v * 2) / 2;
const same = (a: Pt, b: Pt) => Math.abs(a[0] - b[0]) < 1e-6 && Math.abs(a[1] - b[1]) < 1e-6;

function loadDoc(): SchDoc | null {
  try {
    const s = localStorage.getItem(STORE);
    if (!s) return null;
    const d = JSON.parse(s) as SchDoc;
    return d && Array.isArray(d.parts) ? { ...EMPTY_DOC, ...d, analysis: { ...DEFAULT_ANALYSIS, ...d.analysis } } : null;
  } catch { return null; }
}

/** Remove duplicate and collinear points; insert elbows so every segment is horizontal or vertical. */
function tidy(pts: Pt[]): Pt[] {
  const out: Pt[] = [];
  for (const p of pts) {
    const last = out[out.length - 1];
    if (last && !same(last, p) && last[0] !== p[0] && last[1] !== p[1]) out.push([p[0], last[1]]);
    if (!last || !same(last, p)) out.push(p);
  }
  const res: Pt[] = [];
  for (const p of out) {
    res.push(p);
    while (res.length >= 3) {
      const [a, b, c] = res.slice(-3);
      if ((a[0] === b[0] && b[0] === c[0]) || (a[1] === b[1] && b[1] === c[1])) res.splice(res.length - 2, 1);
      else break;
    }
  }
  return res;
}

function movePartsRubber(doc: SchDoc, ids: Set<string>, dx: number, dy: number): SchDoc {
  if (!dx && !dy) return doc;
  const oldPins: Pt[] = doc.parts.filter((p) => ids.has(p.id)).flatMap((p) => pinsOf(p).map((q) => [q.x, q.y] as Pt));
  const hit = (p: Pt) => oldPins.some((q) => same(p, q));
  const parts = doc.parts.map((p) => (ids.has(p.id) ? { ...p, x: p.x + dx, y: p.y + dy } : p));
  const wires = doc.wires.map((w) => {
    const pts = w.pts.map((p) => [p[0], p[1]] as Pt);
    const n = pts.length;
    const ends = [0, n - 1].filter((k, i, a) => a.indexOf(k) === i && hit(w.pts[k] as Pt));
    if (!ends.length) return w;
    if (ends.length === 2 || n === 2) {
      for (const k of ends) { pts[k] = [pts[k][0] + dx, pts[k][1] + dy]; }
      if (ends.length === 2) {
        // both ends move: move the whole wire
        for (let k = 1; k < n - 1; k++) pts[k] = [pts[k][0] + dx, pts[k][1] + dy];
      }
      return { pts: tidy(pts) };
    }
    const k = ends[0];
    const nb = k === 0 ? 1 : n - 2;
    const horiz = w.pts[k][1] === w.pts[nb][1];
    pts[k] = [pts[k][0] + dx, pts[k][1] + dy];
    if (nb !== 0 && nb !== n - 1) pts[nb] = horiz ? [pts[nb][0], pts[nb][1] + dy] : [pts[nb][0] + dx, pts[nb][1]];
    return { pts: tidy(pts) };
  });
  return { ...doc, parts, wires };
}

function nextName(doc: SchDoc, prefix: string): string {
  const used = new Set(doc.parts.map((p) => p.name.toUpperCase()));
  for (let k = 1; k < 999; k++) { const n = `${prefix}${k}`; if (!used.has(n.toUpperCase())) return n; }
  return `${prefix}${Date.now()}`;
}

let uid = 0;
const newId = () => `p${Date.now().toString(36)}${(uid++).toString(36)}`;

function bbox(doc: SchDoc): [number, number, number, number] {
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  const add = (x: number, y: number) => { x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x); y1 = Math.max(y1, y); };
  for (const p of doc.parts) { add(p.x - 3, p.y - 3); add(p.x + 4, p.y + 3); }
  for (const w of doc.wires) for (const q of w.pts) add(q[0], q[1]);
  if (!Number.isFinite(x0)) return [0, 0, 30, 20];
  return [x0 - 1, y0 - 1, x1 + 1, y1 + 1];
}

// ---------------------------------------------------------------------------
// Flow dots (same idea as the lab's Schematic: speed ∝ log |I|, direction = sign)
// ---------------------------------------------------------------------------

interface Geom { pts: Pt[]; cum: number[]; len: number }
function geom(pts: Pt[]): Geom {
  const cum = [0];
  for (let k = 1; k < pts.length; k++) cum.push(cum[k - 1] + Math.hypot(pts[k][0] - pts[k - 1][0], pts[k][1] - pts[k - 1][1]));
  return { pts, cum, len: cum[cum.length - 1] || 1e-9 };
}
function pointAt(g: Geom, s: number): Pt {
  let k = 1;
  while (k < g.cum.length - 1 && g.cum[k] < s) k++;
  const seg = g.cum[k] - g.cum[k - 1] || 1;
  const u = (s - g.cum[k - 1]) / seg;
  const a = g.pts[k - 1], b = g.pts[k];
  return [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u];
}
const SPACING = 1.1;
function speedOf(i: number, imax: number): number {
  const a = Math.abs(i);
  const imin = Math.max(imax * 1e-4, 1e-12);
  if (!(a > imin) || !(imax > 0)) return 0;
  const f = Math.log10(a / imin) / Math.log10(imax / imin);
  return Math.sign(i) * (0.7 + 6.3 * Math.max(0, Math.min(1, f)));
}

function FlowDots({ paths, currents, imax, active }: { paths: Geom[]; currents: number[]; imax: number; active: boolean }) {
  const gref = useRef<SVGGElement>(null);
  const cur = useRef(currents);
  const im = useRef(imax);
  cur.current = currents;
  im.current = imax;
  useEffect(() => {
    const g = gref.current;
    if (!g) return;
    const reduce = prefersReducedMotion();
    const off = paths.map(() => Math.random() * SPACING);
    let last = performance.now();
    let raf = 0;
    const draw = (now: number) => {
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      for (let w = 0; w < paths.length; w++) {
        const grp = g.children[w] as SVGGElement | undefined;
        if (!grp) continue;
        const sp = speedOf(cur.current[w] ?? 0, im.current);
        if (!reduce && active) off[w] = (off[w] + sp * dt) % SPACING;
        grp.setAttribute('opacity', sp === 0 ? '0' : '1');
        const gm = paths[w];
        for (let k = 0; k < grp.children.length; k++) {
          let s = k * SPACING + off[w];
          s = ((s % gm.len) + gm.len) % gm.len;
          const [x, y] = pointAt(gm, s);
          grp.children[k].setAttribute('cx', x.toFixed(3));
          grp.children[k].setAttribute('cy', y.toFixed(3));
        }
      }
      if (!reduce && active) raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(raf);
  }, [paths, active]);
  return (
    <g ref={gref} className="cl-dots" pointerEvents="none">
      {paths.map((p, w) => <g key={w}>{Array.from({ length: Math.max(1, Math.floor(p.len / SPACING)) }, (_, k) => <circle key={k} r={0.14} />)}</g>)}
    </g>
  );
}

// ---------------------------------------------------------------------------
// Property editor
// ---------------------------------------------------------------------------

function Field({ label, value, onChange, hint, numeric = true }: { label: string; value: string; onChange: (v: string) => void; hint?: string; numeric?: boolean }) {
  const bad = numeric && value.trim() !== '' && spiceNum(value) === null && !value.includes('{');
  return (
    <label className="field sch-field">
      <span className="field-top"><span className="field-label" title={hint}>{label}</span>{numeric && value.trim() !== '' && !bad && spiceNum(value) !== null && <span className="small muted tnum">{si(spiceNum(value)!, '', 4)}</span>}{bad && <span className="small" style={{ color: 'var(--bad-text)' }}>✕ not a number</span>}</span>
      <input value={value} onChange={(e) => onChange(e.target.value)} spellCheck={false} />
    </label>
  );
}

function PartProps({ p, onChange, onRotate, onFlip, onDelete, info }: { p: SchPart; onChange: (p: SchPart) => void; onRotate: () => void; onFlip: () => void; onDelete: () => void; info: React.ReactNode }) {
  const pr = p.props ?? {};
  const setP = (k: string, v: string) => onChange({ ...p, props: { ...pr, [k]: v } });
  const fields: [string, string, string?][] = [];
  const src = p.kind === 'vsource' || p.kind === 'isource';
  if (src) {
    const wave = pr.wave ?? 'dc';
    if (wave === 'dc') fields.push(['value', 'DC value']);
    if (wave === 'pulse') fields.push(['v1', 'initial v1'], ['v2', 'pulsed v2'], ['td', 'delay'], ['tr', 'rise'], ['tf', 'fall'], ['pw', 'width'], ['per', 'period']);
    if (wave === 'sin') fields.push(['vo', 'offset'], ['va', 'amplitude'], ['freq', 'frequency']);
    fields.push(['ac', 'AC magnitude (for .ac)']);
  }
  if (p.kind === 'capacitor' || p.kind === 'inductor') fields.push(['ic', p.kind === 'capacitor' ? 'initial voltage (IC, with uic)' : 'initial current (IC, with uic)']);
  if (p.kind === 'diode' || p.kind === 'led') fields.push(['is', 'saturation current IS'], ['n', 'emission coefficient N'], ['rs', 'series resistance RS'], ['bv', 'breakdown voltage BV']);
  if (p.kind === 'nmos' || p.kind === 'pmos') fields.push(['w', 'width W'], ['l', 'length L'], ['vto', 'threshold VTO'], ['kp', 'transconductance KP'], ['lambda', 'channel-length modulation λ']);
  if (p.kind === 'switch') { if (pr.state === 'pwm') fields.push(['freq', 'PWM frequency'], ['duty', 'duty cycle 0–1']); fields.push(['ron', 'on resistance'], ['roff', 'off resistance']); }
  if (p.kind === 'opamp') fields.push(['a0', 'open-loop gain A0'], ['gbw', 'gain-bandwidth GBW']);
  const hasValue = ['resistor', 'capacitor', 'inductor', 'battery'].includes(p.kind);
  return (
    <div className="stack" style={{ gap: 6 }}>
      <div className="row" style={{ gap: 6 }}>
        <b className="mono">{p.name}</b><span className="small muted">{p.kind}</span><span style={{ flex: 1 }} />
        {p.kind !== 'label' && p.kind !== 'ground' && <button className="btn small" onClick={onRotate} title="rotate (R)">⟳ R</button>}
        {(p.kind === 'nmos' || p.kind === 'pmos') && <button className="btn small" onClick={onFlip} title="mirror (F)">⇋ F</button>}
        <button className="btn small" onClick={onDelete} title="delete (Del)">✕</button>
      </div>
      {info}
      {!['ground', 'probe', 'text'].includes(p.kind) && <Field label={p.kind === 'label' ? 'Net name' : 'Name'} value={p.kind === 'label' ? p.value : p.name} numeric={false} onChange={(v) => onChange(p.kind === 'label' ? { ...p, value: v, name: v } : { ...p, name: v.replace(/\s+/g, '') })} />}
      {hasValue && <Field label="Value" value={p.value} onChange={(v) => onChange({ ...p, value: v })} hint="engineering suffixes: f p n u m k meg g — careful, M means milli" />}
      {src && (
        <label className="field sch-field">
          <span className="field-top"><span className="field-label">Waveform</span></span>
          <select value={pr.wave ?? 'dc'} onChange={(e) => setP('wave', e.target.value)}><option value="dc">DC</option><option value="pulse">Pulse</option><option value="sin">Sine</option></select>
        </label>
      )}
      {p.kind === 'switch' && (
        <label className="field sch-field">
          <span className="field-top"><span className="field-label">State</span></span>
          <select value={pr.state ?? 'closed'} onChange={(e) => setP('state', e.target.value)}><option value="closed">closed</option><option value="open">open</option><option value="pwm">PWM (driven)</option></select>
        </label>
      )}
      {fields.map(([k, label]) => <Field key={k} label={label} value={k === 'value' ? p.value : pr[k] ?? ''} onChange={(v) => (k === 'value' ? onChange({ ...p, value: v }) : setP(k, v))} />)}
      {p.template && (
        <label className="field sch-field">
          <span className="field-top"><span className="field-label">SPICE card(s) — imported; &lt;p0&gt;… are the pins</span></span>
          <textarea className="sch-template" rows={Math.min(5, p.template.split('\n').length + 1)} value={p.template} spellCheck={false} onChange={(e) => onChange({ ...p, template: e.target.value })} />
        </label>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Editor
// ---------------------------------------------------------------------------

export function SchematicEditor() {
  const project = useProject((s) => s.project);
  const examples = useCompute<SchExample[]>('circuit.schematic_examples', {});
  const presets = useCompute<Preset[]>('circuit.presets', { project });
  const [hist, setHist] = useState<{ past: SchDoc[]; doc: SchDoc; future: SchDoc[] }>(() => ({ past: [], doc: loadDoc() ?? EMPTY_DOC, future: [] }));
  const doc = hist.doc;
  const [tool, setTool] = useState<Tool>({ t: 'select' });
  const [sel, setSel] = useState<Sel>(null);
  const [cur, setCur] = useState<Pt | null>(null);
  const [draft, setDraft] = useState<Pt[] | null>(null);
  const [hoverNet, setHoverNet] = useState<string | null>(null);
  const [hoverPart, setHoverPart] = useState<string | null>(null);
  const [wrapRef, width] = useElementWidth<HTMLDivElement>();
  const H = width > 700 ? 540 : 420;
  const [view, setView] = useState({ x: -2, y: -2, k: 1 });
  const svgRef = useRef<SVGSVGElement>(null);
  const drag = useRef<null | { kind: 'move'; start: Pt; doc0: SchDoc; ids: Set<string>; moved: boolean } | { kind: 'pan'; sx: number; sy: number; v0: { x: number; y: number; k: number } }>(null);
  const initDone = useRef(false);

  // first visit: open the RC example
  useEffect(() => {
    if (initDone.current || !examples.data) return;
    initDone.current = true;
    if (!loadDoc()) setHist({ past: [], doc: examples.data[0].schematic, future: [] });
  }, [examples.data]);
  useEffect(() => { try { localStorage.setItem(STORE, JSON.stringify(doc)); } catch { /* ignore */ } }, [doc]);

  const commit = useCallback((next: SchDoc) => setHist((h) => ({ past: [...h.past.slice(-100), h.doc], doc: next, future: [] })), []);
  const undo = useCallback(() => setHist((h) => (h.past.length ? { past: h.past.slice(0, -1), doc: h.past[h.past.length - 1], future: [h.doc, ...h.future] } : h)), []);
  const redo = useCallback(() => setHist((h) => (h.future.length ? { past: [...h.past, h.doc], doc: h.future[0], future: h.future.slice(1) } : h)), []);

  const fit = useCallback((d: SchDoc = doc) => {
    if (!width) return;
    const [x0, y0, x1, y1] = bbox(d);
    const k = Math.max(0.35, Math.min(2.2, Math.min(width / ((x1 - x0) * PX), H / ((y1 - y0) * PX))));
    const vw = width / (PX * k), vh = H / (PX * k);
    setView({ x: (x0 + x1) / 2 - vw / 2, y: (y0 + y1) / 2 - vh / 2, k });
  }, [doc, width, H]);
  const fitted = useRef(false);
  useEffect(() => { if (width && !fitted.current && doc.parts.length) { fitted.current = true; fit(); } }, [width, doc, fit]);

  const openDoc = (d: SchDoc) => { commit({ ...EMPTY_DOC, ...d, analysis: { ...DEFAULT_ANALYSIS, ...d.analysis } }); setSel(null); setRes(null); setTimeout(() => fit(d), 0); };

  // ---- extraction (live) and simulation ------------------------------------
  const ex = useCompute<Extracted>('circuit.schematic_netlist', { schematic: doc });
  const X = ex.data;
  const [res, setRes] = useState<SimResult | null>(null);
  const [simErr, setSimErr] = useState<string | null>(null);
  const [simMs, setSimMs] = useState(0);
  const [simX, setSimX] = useState<Extracted | null>(null);
  const [auto, setAuto] = useState(true);
  const run = useCallback((d: SchDoc) => {
    const t0 = performance.now();
    const { data, error } = tryCall<SchSimOut>('circuit.schematic_simulate', { schematic: d, compact: true });
    setSimMs(performance.now() - t0);
    if (error) { setSimErr(error.replace(/^circuit\.\w+:\s*/, '')); return; }
    if (!data) return;
    setSimX(data.extracted);
    setSimErr(data.error);
    if (data.result) setRes(decodeSim(data.result));
  }, []);
  const dragging = drag.current?.kind === 'move';
  useEffect(() => {
    if (!auto || dragging || !doc.parts.length) return;
    const id = window.setTimeout(() => run(doc), 250);
    return () => window.clearTimeout(id);
  }, [doc, auto, dragging, run]);

  // playback
  const tr = res?.tran ?? null;
  const [frame, setFrame] = useState<number | null>(null);
  const [playing, setPlaying] = useState(!prefersReducedMotion());
  const [duration, setDuration] = useState(8);
  useEffect(() => { if (tr && (frame === null || frame >= tr.time.length)) setFrame(0); if (!tr) setFrame(null); }, [tr, frame]);
  const simT = useRef(0);
  useEffect(() => {
    if (!playing || !tr) return;
    const w0 = tr.time[0], w1 = tr.time[tr.time.length - 1];
    simT.current = frame !== null ? tr.time[Math.min(frame, tr.time.length - 1)] : w0;
    let last = performance.now(), lastSet = 0, raf = 0;
    const step = (now: number) => {
      simT.current += ((w1 - w0) / duration) * ((now - last) / 1000);
      last = now;
      if (simT.current > w1) simT.current = w0;
      if (now - lastSet > 33) { lastSet = now; setFrame(frameAt(tr.time, simT.current)); }
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [playing, tr, duration]);
  const snapNow = useMemo(() => snapshot(res, tr ? frame : null), [res, tr, frame]);
  const flowX = simX ?? X;
  const vrange = useMemo(() => voltageRange(res, res?.nodes.filter((n) => !n.startsWith('_')) ?? []), [res]);
  const imax = useMemo(() => {
    if (!res) return 0;
    let m = 0;
    if (res.tran) for (const col of res.tran.i) for (const x of col) m = Math.max(m, Math.abs(x));
    else if (res.op) for (const x of res.op.elem_i) m = Math.max(m, Math.abs(x));
    return m;
  }, [res]);
  const flowPaths = useMemo(() => (flowX?.wires ?? []).map((w) => geom(w.pts)), [flowX]);
  const currents = (flowX?.wires ?? []).map((w) => (res ? w.flow.reduce((a, [el, s]) => a + s * snapNow.i(el), 0) : 0));
  const netColor = (net: string | undefined) => (res && net ? rampColor(snapNow.v(net), vrange[0], vrange[1]) : 'var(--text-2)');
  const partEls = useMemo(() => new Map((X?.parts ?? []).map((p) => [p.id, p])), [X]);
  const elInfo = (name: string | undefined) => (name && res ? res.elements.find((e) => e.name.toLowerCase() === name.toLowerCase()) ?? null : null);
  const errLines = useMemo(() => {
    const m = new Map<number, string>();
    for (const l of (simErr ?? '').split('\n')) { const r = /^line (\d+): (.*)$/.exec(l.trim()); if (r) m.set(Number(r[1]), r[2]); }
    return m;
  }, [simErr]);
  const badParts = new Set((X?.parts ?? []).filter((p) => p.lines.some((l) => errLines.has(l))).map((p) => p.id));

  // ---- geometry helpers -------------------------------------------------------
  const toWorld = (e: { clientX: number; clientY: number }): Pt => {
    const r = svgRef.current!.getBoundingClientRect();
    return [view.x + (e.clientX - r.left) / (PX * view.k), view.y + (e.clientY - r.top) / (PX * view.k)];
  };
  const connectables = useMemo(() => {
    const pts: Pt[] = [];
    for (const p of doc.parts) for (const q of pinsOf(p)) pts.push([q.x, q.y]);
    return pts;
  }, [doc.parts]);
  const onAnyWire = (p: Pt) => doc.wires.some((w) => w.pts.some((q, k) => {
    if (k === 0) return false;
    const a = w.pts[k - 1], b = q;
    return (a[0] === b[0] && p[0] === a[0] && p[1] >= Math.min(a[1], b[1]) && p[1] <= Math.max(a[1], b[1])) || (a[1] === b[1] && p[1] === a[1] && p[0] >= Math.min(a[0], b[0]) && p[0] <= Math.max(a[0], b[0]));
  }));
  const route = (a: Pt, b: Pt): Pt[] => (a[0] === b[0] || a[1] === b[1] ? [b] : Math.abs(b[0] - a[0]) >= Math.abs(b[1] - a[1]) ? [[b[0], a[1]], b] : [[a[0], b[1]], b]);

  const finishWire = (pts: Pt[]) => {
    const t = tidy(pts);
    if (t.length >= 2) commit({ ...doc, wires: [...doc.wires, { pts: t }] });
    setDraft(null);
  };

  const placePart = (kind: string, at: Pt, rot: number) => {
    const item = PALETTE.find((p) => p.kind === kind)!;
    const name = kind === 'label' ? 'net' : nextName(doc, item.prefix);
    const p: SchPart = { id: newId(), kind, name, x: at[0], y: at[1], rot, flip: false, value: kind === 'label' ? nextName(doc, 'net').toLowerCase() : item.value, props: { ...(item.props ?? {}) } };
    if (kind === 'label') p.name = p.value;
    commit({ ...doc, parts: [...doc.parts, p] });
    setSel({ type: 'part', id: p.id });
  };

  // ---- pointer handling ------------------------------------------------------------
  const onDown = (e: RPointerEvent<SVGSVGElement>) => {
    if (e.button === 2) return;
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
    const w = toWorld(e);
    const s: Pt = [snap(w[0]), snap(w[1])];
    const target = (e.target as Element).closest('[data-part],[data-wire]');
    if (tool.t === 'place') { placePart(tool.kind, s, tool.rot); return; }
    if (tool.t === 'wire') {
      if (!draft) { setDraft([s]); return; }
      const last = draft[draft.length - 1];
      if (same(last, s)) { if (draft.length >= 2) finishWire(draft); return; }
      const next = [...draft, ...route(last, s)];
      const hitsSomething = connectables.some((q) => same(q, s)) || onAnyWire(s);
      if (hitsSomething) finishWire(next); else setDraft(next);
      return;
    }
    // select tool
    const pid = target?.getAttribute('data-part');
    const wid = target?.getAttribute('data-wire');
    if (pid) {
      setSel({ type: 'part', id: pid });
      drag.current = { kind: 'move', start: s, doc0: doc, ids: new Set([pid]), moved: false };
      return;
    }
    if (wid !== null && wid !== undefined) { setSel({ type: 'wire', idx: Number(wid) }); return; }
    setSel(null);
    drag.current = { kind: 'pan', sx: e.clientX, sy: e.clientY, v0: view };
  };
  const onMove = (e: RPointerEvent<SVGSVGElement>) => {
    const w = toWorld(e);
    const s: Pt = [snap(w[0]), snap(w[1])];
    if (!cur || !same(cur, s)) setCur(s);
    const d = drag.current;
    if (!d) return;
    if (d.kind === 'pan') {
      setView({ ...d.v0, x: d.v0.x - (e.clientX - d.sx) / (PX * d.v0.k), y: d.v0.y - (e.clientY - d.sy) / (PX * d.v0.k) });
      return;
    }
    const dx = s[0] - d.start[0], dy = s[1] - d.start[1];
    if (dx || dy || d.moved) {
      d.moved = true;
      setHist((h) => ({ ...h, doc: movePartsRubber(d.doc0, d.ids, dx, dy) }));
    }
  };
  const onUp = () => {
    const d = drag.current;
    drag.current = null;
    if (d?.kind === 'move' && d.moved) setHist((h) => ({ past: [...h.past.slice(-100), d.doc0], doc: h.doc, future: [] }));
  };
  const onWheel = (e: React.WheelEvent<SVGSVGElement>) => {
    const w = toWorld(e);
    const k = Math.max(0.3, Math.min(4, view.k * (e.deltaY < 0 ? 1.12 : 1 / 1.12)));
    setView({ k, x: w[0] - (w[0] - view.x) * (view.k / k), y: w[1] - (w[1] - view.y) * (view.k / k) });
  };
  // non-passive wheel so the page does not scroll while zooming
  useEffect(() => {
    const el = svgRef.current;
    if (!el) return;
    const f = (e: WheelEvent) => e.preventDefault();
    el.addEventListener('wheel', f, { passive: false });
    return () => el.removeEventListener('wheel', f);
  }, []);
  const zoom = (f: number) => {
    const cx = view.x + width / (2 * PX * view.k), cy = view.y + H / (2 * PX * view.k);
    const k = Math.max(0.3, Math.min(4, view.k * f));
    setView({ k, x: cx - width / (2 * PX * k), y: cy - H / (2 * PX * k) });
  };

  const selPart = sel?.type === 'part' ? doc.parts.find((p) => p.id === sel.id) ?? null : null;
  const updatePart = (p: SchPart) => commit({ ...doc, parts: doc.parts.map((q) => (q.id === p.id ? p : q)) });
  const rotateSel = () => {
    if (tool.t === 'place') { setTool({ ...tool, rot: (tool.rot + 1) % 4 }); return; }
    if (!selPart) return;
    // rotate about the part centre, keeping attached wires (rubber-band via move of pins is not needed: pins change)
    updatePart({ ...selPart, rot: (selPart.rot + 1) % 4 });
  };
  const deleteSel = () => {
    if (sel?.type === 'part') commit({ ...doc, parts: doc.parts.filter((p) => p.id !== sel.id) });
    if (sel?.type === 'wire') commit({ ...doc, wires: doc.wires.filter((_, k) => k !== sel.idx) });
    setSel(null);
  };

  // keyboard
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement;
      if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT' || t.isContentEditable)) return;
      if (!wrapRef.current?.closest('.sch-root')?.contains(document.activeElement) && document.activeElement !== document.body) return;
      const k = e.key.toLowerCase();
      if ((e.ctrlKey || e.metaKey) && k === 'z') { e.preventDefault(); if (e.shiftKey) redo(); else undo(); return; }
      if ((e.ctrlKey || e.metaKey) && k === 'y') { e.preventDefault(); redo(); return; }
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      if (k === 'r') { e.preventDefault(); rotateSel(); }
      else if (k === 'f' && selPart && (selPart.kind === 'nmos' || selPart.kind === 'pmos')) updatePart({ ...selPart, flip: !selPart.flip });
      else if (k === 'delete' || k === 'backspace') { if (sel) { e.preventDefault(); deleteSel(); } }
      else if (k === 'w') { setTool({ t: 'wire' }); setDraft(null); }
      else if (k === 'escape') { if (draft) { if (draft.length >= 2) finishWire(draft); else setDraft(null); } else setTool({ t: 'select' }); }
      else if (k === 'enter' && draft) finishWire(draft);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  // ---- exports ------------------------------------------------------------------
  const exportSvg = () => {
    const svg = svgRef.current;
    if (!svg) return '';
    const clone = svg.cloneNode(true) as SVGSVGElement;
    clone.querySelectorAll('.sch-ui').forEach((n) => n.remove());
    const [x0, y0, x1, y1] = bbox(doc);
    clone.setAttribute('viewBox', `${x0} ${y0} ${x1 - x0} ${y1 - y0}`);
    clone.setAttribute('width', String((x1 - x0) * PX));
    clone.setAttribute('height', String((y1 - y0) * PX));
    clone.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
    const cs = getComputedStyle(document.documentElement);
    let s = new XMLSerializer().serializeToString(clone);
    s = s.replace(/var\((--[a-z0-9-]+)\)/gi, (_, n: string) => cs.getPropertyValue(n).trim() || '#888');
    const style = `<style>.cl-sym{stroke:${cs.getPropertyValue('--text').trim()};stroke-width:0.13;fill:none;stroke-linecap:round}text{font-family:sans-serif}.sch-name{font-size:0.62px;font-weight:700;fill:${cs.getPropertyValue('--text').trim()}}.sch-val{font-size:0.55px;fill:${cs.getPropertyValue('--muted').trim()}}.sch-net{font-size:0.6px;font-weight:650;fill:${cs.getPropertyValue('--text').trim()}}</style>`;
    return s.replace(/<svg([^>]*)>/, `<svg$1>${style}<rect x="${x0}" y="${y0}" width="${x1 - x0}" height="${y1 - y0}" fill="${cs.getPropertyValue('--surface-2').trim()}"/>`);
  };
  const fileInput = useRef<HTMLInputElement>(null);
  const loadFile = (f: File) => {
    f.text().then((t) => {
      try {
        const d = JSON.parse(t) as SchDoc;
        if (!Array.isArray(d.parts) || !Array.isArray(d.wires)) throw new Error('not a schematic file');
        openDoc(d);
      } catch (err) { setSimErr(`could not open ${f.name}: ${(err as Error).message}`); }
    });
  };
  const importPreset = (id: string) => {
    const { data, error } = tryCall<SchDoc>('circuit.schematic_from_preset', { id, project });
    if (data) openDoc(data); else setSimErr(error);
  };

  // ---- render -------------------------------------------------------------------
  const vb = `${view.x} ${view.y} ${width / (PX * view.k) || 40} ${H / (PX * view.k)}`;
  const gridStep = PX * view.k;
  const ghost: SchPart | null = tool.t === 'place' && cur ? { id: 'ghost', kind: tool.kind, name: '', x: cur[0], y: cur[1], rot: tool.rot, flip: false, value: '', props: PALETTE.find((p) => p.kind === tool.kind)?.props ?? {} } : null;
  const draftPts: Pt[] | null = draft && cur ? [...draft, ...route(draft[draft.length - 1], cur)] : draft;
  const hoverInfo = (() => {
    const id = hoverPart ?? (selPart?.id ?? null);
    if (!id || !res) return null;
    const pn = partEls.get(id);
    const el = elInfo(pn?.elements[0]);
    if (!el) return null;
    const { v, i, p } = elemVIP(el, snapNow);
    return <div className="small tnum sch-vip"><b>{pn!.name}</b> V = {si(v, 'V', 4)} · I = {si(i, 'A', 4)} · P = {si(p, 'W', 3)}</div>;
  })();

  return (
    <div className="stack sch-root">
      <Card
        title={<input className="sch-title" value={doc.title} onChange={(e) => setHist((h) => ({ ...h, doc: { ...h.doc, title: e.target.value } }))} aria-label="schematic title" />}
        subtitle="Click a part in the palette, click the canvas to place it (R rotates). W draws wires: click to add corners, click a pin or wire (or double-click / Enter) to finish. Drag parts — wires follow. Wheel or buttons zoom, drag the background to pan."
        actions={
          <div className="row" style={{ gap: 6, flexWrap: 'wrap' }}>
            <select className="sch-select" value="" onChange={(e) => { const x = examples.data?.find((q) => q.id === e.target.value); if (x) openDoc(x.schematic); }} aria-label="open a starter example">
              <option value="">Examples…</option>
              {(examples.data ?? []).map((x) => <option key={x.id} value={x.id}>{x.title}</option>)}
            </select>
            <select className="sch-select" value="" onChange={(e) => e.target.value && importPreset(e.target.value)} aria-label="import a Circuit Lab preset">
              <option value="">Import preset…</option>
              {(presets.data ?? []).map((p) => <option key={p.id} value={p.id}>{p.title}</option>)}
            </select>
            <button className="btn small" onClick={() => { commit({ ...EMPTY_DOC }); setRes(null); setSel(null); }}>New</button>
          </div>
        }
      >
        <div className="sch-toolbar" role="toolbar" aria-label="schematic tools">
          <button className={`sch-tool ${tool.t === 'select' ? 'on' : ''}`} onClick={() => { setTool({ t: 'select' }); setDraft(null); }} title="select / move (Esc)" aria-pressed={tool.t === 'select'}>⬚ Select</button>
          <button className={`sch-tool ${tool.t === 'wire' ? 'on' : ''}`} onClick={() => { setTool({ t: 'wire' }); setDraft(null); }} title="draw wires (W)" aria-pressed={tool.t === 'wire'}>╱ Wire</button>
          <span className="sch-sep" />
          {PALETTE.map((it) => (
            <button key={it.kind} className={`sch-tool pal ${tool.t === 'place' && tool.kind === it.kind ? 'on' : ''}`} onClick={() => { setTool({ t: 'place', kind: it.kind, rot: it.kind === 'resistor' || it.kind === 'inductor' ? 3 : 0 }); setDraft(null); }} title={`${it.label} — ${it.hint}`} aria-label={it.label}>
              <svg viewBox="-3.4 -3.4 6.8 6.8" width="26" height="26"><g className="cl-sym"><SymbolBody kind={it.kind} props={it.props} /></g></svg>
              <span>{it.label}</span>
            </button>
          ))}
          <span className="sch-sep" />
          <button className="sch-tool" onClick={undo} disabled={!hist.past.length} title="undo (Ctrl+Z)">↶</button>
          <button className="sch-tool" onClick={redo} disabled={!hist.future.length} title="redo (Ctrl+Shift+Z)">↷</button>
          <button className="sch-tool" onClick={() => zoom(1.25)} title="zoom in">＋</button>
          <button className="sch-tool" onClick={() => zoom(0.8)} title="zoom out">−</button>
          <button className="sch-tool" onClick={() => fit()} title="fit to drawing">⤢</button>
        </div>

        <div className="sch-main">
          <div ref={wrapRef} className="sch-canvas cl-top" style={{ height: H }}>
            <svg
              ref={svgRef}
              className={`cl-schem sch-svg tool-${tool.t}`}
              viewBox={vb}
              preserveAspectRatio="xMinYMin meet"
              width="100%"
              height={H}
              onPointerDown={onDown}
              onPointerMove={onMove}
              onPointerUp={onUp}
              onPointerLeave={() => setCur(null)}
              onWheel={onWheel}
              onDoubleClick={() => { if (draft && draft.length >= 2) finishWire(draft); }}
              onContextMenu={(e) => { e.preventDefault(); if (draft) { if (draft.length >= 2) finishWire(draft); else setDraft(null); } else setTool({ t: 'select' }); }}
              role="application"
              aria-label="schematic canvas"
              style={{ touchAction: 'none' }}
            >
              <defs>
                <pattern id="sch-grid" x={0} y={0} width={1} height={1} patternUnits="userSpaceOnUse">
                  <circle cx={0} cy={0} r={gridStep > 14 ? 0.05 : 0.07} className="sch-dot" />
                </pattern>
              </defs>
              <rect className="sch-ui" x={view.x - 1} y={view.y - 1} width={width / (PX * view.k) + 2} height={H / (PX * view.k) + 2} fill="url(#sch-grid)" />
              {/* wires */}
              <g>
                {doc.wires.map((w, k) => {
                  const net = X?.wire_nets[k];
                  const hi = hoverNet && net === hoverNet;
                  const selected = sel?.type === 'wire' && sel.idx === k;
                  return (
                    <g key={k} data-wire={k} onPointerEnter={() => setHoverNet(net ?? null)} onPointerLeave={() => setHoverNet(null)}>
                      <polyline points={w.pts.map((p) => p.join(',')).join(' ')} fill="none" stroke="transparent" strokeWidth={0.8} className="sch-hit" />
                      <polyline points={w.pts.map((p) => p.join(',')).join(' ')} fill="none" stroke={selected ? 'var(--accent)' : netColor(net)} strokeWidth={hi || selected ? 0.3 : 0.2} strokeLinecap="round" strokeLinejoin="round" />
                    </g>
                  );
                })}
              </g>
              {res && <FlowDots paths={flowPaths} currents={currents} imax={imax} active={playing || !tr} />}
              {(X?.junctions ?? []).map((j, k) => <circle key={k} cx={j[0]} cy={j[1]} r={0.22} fill={netColor(undefined)} className="sch-junction" />)}
              {/* parts */}
              {doc.parts.map((p) => {
                const pn = partEls.get(p.id);
                const el = elInfo(pn?.elements[0]);
                let glow = 0;
                if (p.kind === 'led' && el) glow = imax > 0 ? Math.max(0, Math.min(1, elemVIP(el, snapNow).i / (imax * 0.8))) : 0;
                const on = p.kind === 'switch' && el ? switchOn(el, snapNow) : undefined;
                const selected = selPart?.id === p.id;
                const lp = labelPos(p);
                const net = pn?.nets[0];
                const bad = badParts.has(p.id);
                return (
                  <g key={p.id} data-part={p.id} className={`sch-part ${selected ? 'sel' : ''} ${bad ? 'bad' : ''}`} onPointerEnter={() => setHoverPart(p.id)} onPointerLeave={() => setHoverPart(null)}>
                    <rect x={p.x - (p.kind === 'opamp' ? 3.2 : 2.2)} y={p.y - 2.2} width={p.kind === 'opamp' ? 6.4 : 4.4} height={4.4} rx={0.6} className="sch-box" />
                    {p.kind === 'text' ? (
                      <text x={p.x} y={p.y} className="sch-note">{p.value}</text>
                    ) : (
                      <g transform={`translate(${p.x} ${p.y}) rotate(${p.rot * 90}) scale(${p.flip ? -1 : 1} 1)`} className="cl-sym"><SymbolBody kind={p.kind} props={p.props} glow={glow} on={on} /></g>
                    )}
                    {p.kind === 'label' && (
                      <text x={lp.x} y={lp.y} className="sch-net">{p.value}{res && net ? <tspan className="cl-lval" dx={0.3}>{si(snapNow.v(net), 'V', 3)}</tspan> : null}</text>
                    )}
                    {p.kind === 'probe' && (
                      <text x={lp.x} y={lp.y} className="sch-net">{net ? `v(${net})` : 'probe'}{res && net ? <tspan className="cl-lval" dx={0.3}>{si(snapNow.v(net), 'V', 3)}</tspan> : null}</text>
                    )}
                    {!['label', 'probe', 'ground', 'text'].includes(p.kind) && (
                      <text x={lp.x} y={lp.y} textAnchor={lp.anchor}>
                        <tspan className="sch-name">{p.name}</tspan>
                        <tspan className="sch-val" dx={0.3}>{valueText(p)}</tspan>
                      </text>
                    )}
                  </g>
                );
              })}
              {/* pins of the hovered/selected part and connection hints while wiring */}
              <g className="sch-ui">
                {tool.t === 'wire' && connectables.map((q, k) => <circle key={k} cx={q[0]} cy={q[1]} r={0.16} className="sch-pin" />)}
                {draftPts && <polyline points={draftPts.map((p) => p.join(',')).join(' ')} fill="none" className="sch-draft" strokeWidth={0.18} />}
                {ghost && <g transform={`translate(${ghost.x} ${ghost.y}) rotate(${ghost.rot * 90})`} className="cl-sym sch-ghost"><SymbolBody kind={ghost.kind} props={ghost.props} /></g>}
                {tool.t !== 'select' && cur && <circle cx={cur[0]} cy={cur[1]} r={0.12} className="sch-cursor" />}
              </g>
            </svg>
            {!doc.parts.length && <div className="sch-empty">Empty sheet — open an example, import a preset, or pick a part from the palette.</div>}
          </div>

          <div className="stack sch-side">
            <Card title={selPart ? 'Properties' : sel?.type === 'wire' ? 'Wire' : 'Selection'} className="sch-card">
              {selPart ? (
                <PartProps
                  p={selPart}
                  onChange={updatePart}
                  onRotate={rotateSel}
                  onFlip={() => updatePart({ ...selPart, flip: !selPart.flip })}
                  onDelete={deleteSel}
                  info={partEls.get(selPart.id) ? <div className="small muted">pins → {partEls.get(selPart.id)!.pins.map((q, k) => `${q.name}: ${partEls.get(selPart.id)!.nets[k] ?? '?'}`).join(' · ')}</div> : null}
                />
              ) : sel?.type === 'wire' ? (
                <div className="stack" style={{ gap: 6 }}>
                  <div className="small">Net <b className="mono">{X?.wire_nets[sel.idx] ?? '?'}</b>, {doc.wires[sel.idx]?.pts.length ?? 0} points.</div>
                  <button className="btn small" onClick={deleteSel}>✕ Delete wire</button>
                </div>
              ) : (
                <div className="small muted">Click a part or wire to edit it. Hover a wire to highlight its whole net.</div>
              )}
              {hoverInfo}
            </Card>
            <AnalysisPanel doc={doc} commit={commit} />
            <Card title="Simulate" className="sch-card">
              <div className="row" style={{ gap: 6 }}>
                <button className="btn primary" onClick={() => run(doc)}><Icon name="play" size={13} /> Run</button>
                <label className="op-check small" style={{ display: 'inline-flex', gap: 6, alignItems: 'center' }}><input type="checkbox" checked={auto} onChange={(e) => setAuto(e.target.checked)} /> auto-run on edit</label>
              </div>
              <div className="row small" style={{ gap: 6, marginTop: 8, flexWrap: 'wrap' }}>
                {X && <Chip tone={X.warnings.length ? 'warn' : 'pass'}>{X.warnings.length ? '!' : '✓'} {doc.parts.filter((p) => !['ground', 'label', 'probe', 'text'].includes(p.kind)).length} parts · {X.node_count} nodes</Chip>}
                {res && <Chip title="engine call time">{simMs.toFixed(0)} ms</Chip>}
              </div>
              {X && X.warnings.length > 0 && <ul className="small sch-warn">{X.warnings.map((w, k) => <li key={k}>{w}</li>)}</ul>}
              <ErrorBox error={ex.error ? ex.error.replace(/^circuit\.\w+:\s*/, '') : null} />
              {simErr && <ErrorBox error={simErr} />}
            </Card>
          </div>
        </div>

        <div className="row" style={{ gap: 6, marginTop: 10, flexWrap: 'wrap' }}>
          {tr && (
            <div className="cl-scrub" style={{ flex: '1 1 320px', marginTop: 0 }}>
              <button className="btn small" onClick={() => setPlaying((p) => !p)} aria-label={playing ? 'pause' : 'play'}>{playing ? '❚❚ Pause' : <><Icon name="play" size={13} /> Play</>}</button>
              <input type="range" min={0} max={tr.time.length - 1} value={Math.min(frame ?? 0, tr.time.length - 1)} onChange={(e) => { setPlaying(false); setFrame(Number(e.target.value)); }} aria-label="simulation time" />
              <span className="tnum small cl-tnow">t = {si(tr.time[Math.min(frame ?? 0, tr.time.length - 1)], 's', 4)}</span>
              <select value={duration} onChange={(e) => setDuration(Number(e.target.value))} aria-label="playback speed" className="cl-speed"><option value={3}>fast</option><option value={8}>normal</option><option value={20}>slow</option><option value={60}>very slow</option></select>
            </div>
          )}
          <span style={{ flex: 1 }} />
          <DownloadButton small filename={`${slug(doc.title)}.json`} content={() => JSON.stringify(doc, null, 1)} label="save .json" />
          <button className="btn small" onClick={() => fileInput.current?.click()}>open .json</button>
          <input ref={fileInput} type="file" accept=".json,application/json" style={{ display: 'none' }} onChange={(e) => { const f = e.target.files?.[0]; if (f) loadFile(f); e.target.value = ''; }} />
          {X && <DownloadButton small filename={`${slug(doc.title)}.cir`} content={() => X.netlist} label="export .cir" />}
          <button className="btn small" onClick={() => downloadText(`${slug(doc.title)}.svg`, exportSvg(), 'image/svg+xml')}>export .svg</button>
        </div>
      </Card>

      {res && X && <SchResults res={res} extracted={simX ?? X} frame={tr ? frame : null} fileBase={slug(doc.title)} />}

      <Card title="Extracted netlist" subtitle="What the connectivity pass hands to the simulator — plain SPICE, runs unchanged in ngspice.">
        {X ? <Code code={X.netlist} filename={`${slug(doc.title)}.cir`} maxHeight={260} /> : <div className="muted small">Extracting…</div>}
        {X && (
          <div className="table-wrap" style={{ marginTop: 10 }}>
            <table className="table">
              <thead><tr><th>Net</th><th>Pins</th><th>Labels</th></tr></thead>
              <tbody>{X.nets.map((n) => <tr key={n.name} onMouseEnter={() => setHoverNet(n.name)} onMouseLeave={() => setHoverNet(null)}><td className="mono">{n.name === '0' ? '0 (ground)' : n.name}{n.probe ? ' ◉' : ''}</td><td className="small">{n.pins.join(', ') || '—'}</td><td className="small">{n.labels.join(', ')}</td></tr>)}</tbody>
            </table>
          </div>
        )}
      </Card>

      <div className="grid grid-2 cl-explain">
        <Explainer title="From a drawing to a netlist" refs={[REFS.ngspice, { label: 'R. E. Tarjan, "Efficiency of a good but not linear set union algorithm", J. ACM 22 (1975)' }]}>
          <Formula>{`points  = pins ∪ wire vertices ∪ label and ground anchors\nunion(a, b) for every wire segment and every point lying on it\nlabels with the same name → one net;  ground → node 0\nnet name = label | preset node name | n1, n2 …`}</Formula>
          <p>
            That is all a schematic capture program's netlister does: a union–find over coordinates. Two wires that merely cross do <i>not</i> connect — only an end on a wire (drawn with a junction dot) or a
            shared point does, exactly as in KiCad and EasyEDA. Each part then becomes a SPICE card with its pins' net names.
          </p>
        </Explainer>
        <Explainer title="Current on the wires you drew" refs={[REFS.vlach]}>
          <Formula>{`pin 1 of X injects −i(X), pin 2 injects +i(X)   (SPICE: i flows into pin 1)\ncurrent along a wire = Σ injections on one side of it   (KCL on the net's tree)`}</Formula>
          <p>
            The simulator only knows currents through <i>elements</i>. To animate every wire segment, the engine builds a spanning tree of each net's wires and sums the element currents entering on one side —
            the same bookkeeping the hand-drawn preset schematics carry, done automatically. Dots move faster for bigger currents (log scale) and reverse with the sign.
          </p>
        </Explainer>
        <Explainer title="Where to go next" refs={[{ label: 'KiCad (schematic + ngspice)', href: 'https://www.kicad.org' }, { label: 'EasyEDA', href: 'https://easyeda.com' }, { label: 'Falstad circuit simulator', href: 'https://www.falstad.com/circuit/' }, REFS.ngspice]}>
          <p>
            Export the <code>.cir</code> to cross-check in ngspice, or redraw the circuit in <b>KiCad</b> (whose schematic feeds ngspice and the PCB layout the Iris board is built from) or <b>EasyEDA</b> (web, with a parts
            library tied to a fab). <b>Falstad</b>'s simulator is the classic for building intuition with animated currents.
          </p>
        </Explainer>
        <Explainer title="Models are simple on purpose">
          <p>
            Diodes and LEDs use the SPICE junction model with generic parameters (the LED is fitted to ~1.9 V at 20 mA), MOSFETs are level-1 Shichman–Hodges with the bulk tied to the source, the switch is a
            voltage-controlled resistor, and the op-amp is a single-pole macro-model with no output-swing limits. Replace them with datasheet models (a <code>.model</code> line in an imported preset shows how) before trusting margins.
          </p>
        </Explainer>
      </div>
    </div>
  );
}

function slug(s: string) {
  return (s || 'schematic').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 40) || 'schematic';
}

function AnalysisPanel({ doc, commit }: { doc: SchDoc; commit: (d: SchDoc) => void }) {
  const a = doc.analysis;
  const set = (p: Partial<typeof a>) => commit({ ...doc, analysis: { ...a, ...p } });
  const sources = doc.parts.filter((p) => ['vsource', 'isource', 'battery'].includes(p.kind)).map((p) => p.name);
  return (
    <Card title="Analyses" className="sch-card">
      <div className="sch-an">
        <label className="op-check"><input type="checkbox" checked={a.op} onChange={(e) => set({ op: e.target.checked })} /> .op operating point</label>
        <label className="op-check"><input type="checkbox" checked={a.tran} onChange={(e) => set({ tran: e.target.checked })} /> .tran transient</label>
        {a.tran && (
          <div className="sch-an-row">
            <Field label="step" value={a.tstep} onChange={(v) => set({ tstep: v })} />
            <Field label="stop" value={a.tstop} onChange={(v) => set({ tstop: v })} />
            <label className="op-check small"><input type="checkbox" checked={a.uic} onChange={(e) => set({ uic: e.target.checked })} /> uic</label>
          </div>
        )}
        <label className="op-check"><input type="checkbox" checked={a.ac} onChange={(e) => set({ ac: e.target.checked })} /> .ac Bode (set a source's AC magnitude)</label>
        {a.ac && (
          <div className="sch-an-row">
            <Field label="pts/dec" value={String(a.ac_points)} onChange={(v) => set({ ac_points: Math.max(1, Math.round(Number(v) || 1)) })} />
            <Field label="from" value={a.fstart} onChange={(v) => set({ fstart: v })} />
            <Field label="to" value={a.fstop} onChange={(v) => set({ fstop: v })} />
          </div>
        )}
        <label className="op-check"><input type="checkbox" checked={a.dc} onChange={(e) => set({ dc: e.target.checked })} /> .dc sweep</label>
        {a.dc && (
          <div className="sch-an-row">
            <label className="field sch-field"><span className="field-top"><span className="field-label">source</span></span>
              <select value={a.dc_source} onChange={(e) => set({ dc_source: e.target.value })}>{[...new Set([a.dc_source, ...sources])].map((s) => <option key={s} value={s}>{s}</option>)}</select>
            </label>
            <Field label="from" value={a.dc_start} onChange={(v) => set({ dc_start: v })} />
            <Field label="to" value={a.dc_stop} onChange={(v) => set({ dc_stop: v })} />
            <Field label="step" value={a.dc_step} onChange={(v) => set({ dc_step: v })} />
          </div>
        )}
        {doc.directives.length > 0 && (
          <details className="small"><summary className="muted">{doc.directives.length} extra deck line{doc.directives.length > 1 ? 's' : ''} (from the imported preset)</summary><pre className="code" style={{ maxHeight: 160 }}>{doc.directives.join('\n')}</pre></details>
        )}
      </div>
    </Card>
  );
}

export { Tabs };
