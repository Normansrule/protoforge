// Shared state, helpers and small components for the Systems Studio tabs.
import { createContext, useContext, useEffect, useRef, useState, useSyncExternalStore, type ReactNode } from 'react';
import { LABS, type LabInfo } from '../../labs/registry';
import { fmtNum } from '../../components/Plot';
import { plural } from '../../components/reqStatus';
import type { Cmp, Domain, Evaluation, Metric, Project, ReqKind, Status, VerifyMethod } from '../../engine/types';
import type { Studio } from './types';

export { fmtNum };

// ------------------------------------------------------------------ references

export const REFS = {
  nasa: { label: 'NASA Systems Engineering Handbook, NASA/SP-2016-6105 Rev 2', href: 'https://www.nasa.gov/reference/systems-engineering-handbook/' },
  incose: { label: 'INCOSE Systems Engineering Handbook, 5th ed. (Wiley, 2023)' },
  gtwr: { label: 'INCOSE Requirements Working Group, "Guide to Writing Requirements"' },
  iso15288: { label: 'ISO/IEC/IEEE 15288:2023, Systems and software engineering — System life cycle processes' },
  vmodel: { label: 'K. Forsberg & H. Mooz, "The relationship of system engineering to the project cycle", NCOSE/ASEM 1991' },
  fmea: { label: 'AIAG & VDA, FMEA Handbook (2019)' },
  n2: { label: 'R. J. Lano, "The N² Chart", TRW Software Series (1977)' },
  dsm: { label: 'D. V. Steward, "The Design Structure System", IEEE Trans. Engineering Management EM-28 (1981)' },
  saaty: { label: 'T. L. Saaty, The Analytic Hierarchy Process (McGraw-Hill, 1980)' },
  mass: { label: 'AIAA S-120A-2015, Mass Properties Control for Space Systems' },
};

// ------------------------------------------------------------------ context

export type TabId =
  | 'overview' | 'concept' | 'requirements' | 'architecture' | 'traceability' | 'vmodel'
  | 'trades' | 'risks' | 'budgets' | 'roadmap' | 'report';

export interface StudioCtx {
  project: Project;
  ev: Evaluation | null;
  studio: Studio | null;
  update: (fn: (d: Project) => void) => void;
  go: (tab: TabId) => void;
  openReq: (id: string) => void;
}

export const StudioContext = createContext<StudioCtx | null>(null);

export function useStudio(): StudioCtx {
  const c = useContext(StudioContext);
  if (!c) throw new Error('useStudio outside the Systems Studio');
  return c;
}

// ------------------------------------------------------------------ labs

export interface LabRef { id: string; title: string; path: string; icon: string }

export const SYSTEMS_LAB: LabRef = { id: 'systems', title: 'Systems roll-up', path: '/systems?tab=budgets', icon: 'systems' };

/** The lab that computes a metric, by key prefix (registry prefixes + systems roll-ups). */
export function labForMetric(key: string | null | undefined): LabRef | null {
  if (!key) return null;
  const lab = LABS.find((l) => l.metrics.some((p) => key.startsWith(p)));
  if (lab) return lab;
  if (/^(mass|bom|fit|budget)\./.test(key)) return SYSTEMS_LAB;
  return null;
}

export function labById(id: string): LabRef | null {
  if (id === 'systems') return SYSTEMS_LAB;
  return LABS.find((l) => l.id === id) ?? null;
}

/** Labs in registry (V-model) order plus the systems roll-up. */
export const SWEEP_LABS: (LabInfo | LabRef)[] = [...LABS, SYSTEMS_LAB];

export function metricsOfLab(lab: LabRef, metrics: Metric[]): Metric[] {
  return metrics.filter((m) => labForMetric(m.key)?.id === lab.id);
}

// ------------------------------------------------------------------ enums

export const KINDS: ReqKind[] = ['performance', 'functional', 'interface', 'environmental', 'safety', 'regulatory', 'manufacturing', 'cost', 'goal'];
export const METHODS: VerifyMethod[] = ['analysis', 'test', 'inspection', 'demonstration'];
export const CMPS: { value: Cmp; label: string }[] = [
  { value: 'le', label: '≤ at most' },
  { value: 'ge', label: '≥ at least' },
  { value: 'between', label: '… between' },
  { value: 'eq', label: '= equals ± tolerance' },
];
export const DOMAINS: Domain[] = ['electrical', 'mechanical', 'optical', 'software', 'silicon', 'acoustic', 'thermal', 'manufacturing'];
export const IF_KINDS = ['POWER', 'I2C', 'SPI', 'PDM', 'UART', 'USB', 'BLE', 'PWM', 'GPIO', 'MIPI', 'SW', 'MECH', 'OPTICAL', 'THERMAL'];

export const cap = (s: string) => (s ? s[0].toUpperCase() + s.slice(1) : s);

/** Next id with a prefix, e.g. nextId('R-', ['R-001','R-024'], 3) → 'R-025'. */
export function nextId(prefix: string, ids: string[], pad = 3): string {
  let max = 0;
  for (const id of ids) {
    if (!id.startsWith(prefix)) continue;
    const n = parseInt(id.slice(prefix.length), 10);
    if (Number.isFinite(n)) max = Math.max(max, n);
  }
  return prefix + String(max + 1).padStart(pad, '0');
}

export const pct = (x: number | null | undefined, digits = 0) => (x == null || !isFinite(x) ? '—' : `${(x * 100).toFixed(digits)} %`);
export const signedPct = (x: number | null | undefined) => (x == null || !isFinite(x) ? '—' : `${x >= 0 ? '+' : '−'}${Math.abs(x * 100).toFixed(Math.abs(x) < 0.1 ? 1 : 0)} %`);

// ------------------------------------------------------------------ status

export type AnyStatus = Status | 'goal' | 'warn' | 'done' | 'partial' | 'attention' | 'empty' | 'met' | 'unmet' | 'manual' | 'todo' | 'info';

const STATUS: Record<AnyStatus, { icon: string; word: string; tone: string }> = {
  pass: { icon: '✓', word: 'pass', tone: 'pass' },
  fail: { icon: '✕', word: 'fail', tone: 'fail' },
  goal: { icon: '!', word: 'open goal', tone: 'warn' },
  unverified: { icon: '○', word: 'unverified', tone: '' },
  warn: { icon: '!', word: 'check', tone: 'warn' },
  done: { icon: '✓', word: 'done', tone: 'pass' },
  partial: { icon: '◐', word: 'in progress', tone: 'info' },
  attention: { icon: '!', word: 'attention', tone: 'fail' },
  empty: { icon: '○', word: 'not started', tone: '' },
  met: { icon: '✓', word: 'met', tone: 'pass' },
  unmet: { icon: '✕', word: 'not met', tone: 'fail' },
  manual: { icon: '✎', word: 'sign-off', tone: 'warn' },
  todo: { icon: '○', word: 'to do', tone: '' },
  info: { icon: 'i', word: 'info', tone: 'info' },
};

/** Status chip: icon + word, never colour alone. */
export function Badge({ status, label, title }: { status: AnyStatus; label?: string; title?: string }) {
  const s = STATUS[status] ?? STATUS.unverified;
  return <span className={`chip ${s.tone}`} title={title}><span aria-hidden>{s.icon}</span> {label ?? s.word}</span>;
}

export const statusColor = (s: AnyStatus) =>
  s === 'pass' || s === 'done' || s === 'met' ? 'var(--good)' : s === 'fail' || s === 'attention' || s === 'unmet' ? 'var(--bad)' : s === 'warn' || s === 'manual' || s === 'goal' ? 'var(--warn)' : s === 'partial' || s === 'info' ? 'var(--accent)' : 'var(--muted)';

// ------------------------------------------------------------------ margin bar

/** Margin against a limit as a centred bar: right/green = inside the limit, left/red = outside. */
export function MarginBar({ margin, width = 92, goal = false }: { margin: number | null | undefined; width?: number; /** an open stretch goal: outside the limit reads amber, not red */ goal?: boolean }) {
  if (margin == null || !isFinite(margin)) return <span className="muted small">—</span>;
  const clamped = Math.max(-1, Math.min(1, margin));
  const half = width / 2;
  const w = Math.max(2, Math.abs(clamped) * half);
  const good = margin >= 0;
  const out = goal ? 'var(--warn)' : 'var(--bad)';
  return (
    <span className="ss-margin" title={`margin ${signedPct(margin)} of the limit`}>
      <svg width={width} height={10} aria-hidden>
        <rect x={0} y={4} width={width} height={2} rx={1} fill="var(--surface-3)" />
        <rect x={good ? half : half - w} y={1} width={w} height={8} rx={3} fill={good ? 'var(--good)' : out} opacity={0.85} />
        <line x1={half} x2={half} y1={0} y2={10} stroke="var(--text-2)" strokeWidth={1.2} />
      </svg>
      <span className="mono small" style={{ color: good ? 'var(--good-text)' : goal ? 'var(--warn-text)' : 'var(--bad-text)' }}>{signedPct(margin)}</span>
    </span>
  );
}

// ------------------------------------------------------------------ ring

/** Donut of pass / fail / open goal / unverified with the pass count in the middle (`fail` = binding failures only). */
export function Ring({ pass, fail, goal = 0, unverified, size = 168, stroke = 16, label = 'verified', progress = 1 }: {
  pass: number; fail: number; goal?: number; unverified: number; size?: number; stroke?: number; label?: string; progress?: number;
}) {
  const n = pass + fail + goal + unverified;
  const total = Math.max(1, n);
  const r = (size - stroke) / 2;
  const C = 2 * Math.PI * r;
  const gap = n > 1 ? 3 : 0;
  const segs: { v: number; color: string; name: string }[] = [
    { v: pass, color: 'var(--good)', name: 'pass' },
    { v: fail, color: 'var(--bad)', name: 'fail' },
    { v: goal, color: 'var(--warn)', name: 'goal' },
    { v: unverified, color: 'var(--muted)', name: 'unverified' },
  ];
  let acc = 0;
  const shown = Math.round(pass * progress);
  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} role="img" aria-label={`${pass} of ${n} requirements pass, ${fail} fail, ${plural(goal, 'open goal')}, ${unverified} unverified`}>
      <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--surface-3)" strokeWidth={stroke} />
      <g transform={`rotate(-90 ${size / 2} ${size / 2})`}>
        {segs.map((s) => {
          const len = (s.v / total) * C * progress;
          const el = s.v > 0 ? (
            <circle key={s.name} cx={size / 2} cy={size / 2} r={r} fill="none" stroke={s.color} strokeWidth={stroke}
              strokeDasharray={`${Math.max(0, len - gap)} ${C}`} strokeDashoffset={-acc} strokeLinecap="butt" style={{ transition: 'stroke-dasharray 0.35s ease, stroke-dashoffset 0.35s ease' }} opacity={s.name === 'unverified' ? 0.5 : 1} />
          ) : null;
          acc += len;
          return el;
        })}
      </g>
      <text x={size / 2} y={size / 2 - 2} textAnchor="middle" fontSize={size * 0.24} fontWeight={800} fill="var(--text)" className="tnum">{shown}<tspan fontSize={size * 0.12} fill="var(--muted)" fontWeight={600}>/{n}</tspan></text>
      <text x={size / 2} y={size / 2 + size * 0.13} textAnchor="middle" fontSize={Math.max(10, size * 0.075)} fill="var(--muted)" letterSpacing="0.08em">{label.toUpperCase()}</text>
    </svg>
  );
}

// ------------------------------------------------------------------ inputs

/** Text input that commits after a pause or on blur (no engine call per keystroke). */
export function DText(props: { value: string; onCommit: (v: string) => void; placeholder?: string; area?: boolean; rows?: number; className?: string; ariaLabel?: string; mono?: boolean; list?: string; style?: React.CSSProperties; upper?: boolean }) {
  const [v, setV] = useState(props.value);
  const focused = useRef(false);
  const timer = useRef<number | null>(null);
  const latest = useRef(props.onCommit);
  latest.current = props.onCommit;
  useEffect(() => {
    if (!focused.current) setV(props.value);
  }, [props.value]);
  useEffect(() => () => { if (timer.current) window.clearTimeout(timer.current); }, []);
  const commit = (x: string) => {
    if (timer.current) window.clearTimeout(timer.current);
    timer.current = null;
    if (x !== props.value) latest.current(x);
  };
  const common = {
    value: v,
    placeholder: props.placeholder,
    'aria-label': props.ariaLabel ?? props.placeholder,
    className: `${props.className ?? ''} ${props.area && !props.mono ? 'ss-prose' : ''}`,
    onFocus: () => { focused.current = true; },
    onBlur: () => { focused.current = false; commit(v); },
    style: props.style,
    onChange: (e: { target: { value: string } }) => {
      const x = props.upper ? e.target.value.toUpperCase() : e.target.value;
      setV(x);
      if (timer.current) window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => commit(x), 600);
    },
  };
  return props.area ? <textarea rows={props.rows ?? 3} {...common} /> : <input type="text" list={props.list} {...common} />;
}

/** Number input that commits valid numbers (lets the user type "-" or "1." first). */
export function DNum(props: { value: number | null; onCommit: (v: number) => void; step?: number; min?: number; max?: number; ariaLabel?: string; className?: string; width?: number }) {
  const [v, setV] = useState(props.value == null ? '' : String(props.value));
  const focused = useRef(false);
  useEffect(() => {
    if (!focused.current) setV(props.value == null ? '' : String(props.value));
  }, [props.value]);
  return (
    <input
      type="number"
      className={props.className}
      style={props.width ? { width: props.width } : undefined}
      value={v}
      step={props.step ?? 'any'}
      min={props.min}
      max={props.max}
      aria-label={props.ariaLabel}
      onFocus={() => { focused.current = true; }}
      onBlur={() => { focused.current = false; setV(props.value == null ? '' : String(props.value)); }}
      onChange={(e) => {
        setV(e.target.value);
        const n = Number(e.target.value);
        if (e.target.value.trim() !== '' && Number.isFinite(n)) {
          const c = Math.min(props.max ?? Infinity, Math.max(props.min ?? -Infinity, n));
          if (c !== props.value) props.onCommit(c);
        }
      }}
    />
  );
}

/** Editable list of short strings. */
export function ListEditor(props: { items: string[]; onChange: (items: string[]) => void; placeholder?: string; addLabel?: string }) {
  return (
    <div className="ss-list">
      {props.items.map((it, i) => (
        <div className="ss-list-row" key={i}>
          <span className="ss-bullet" aria-hidden />
          <DText value={it} ariaLabel={`${props.placeholder ?? 'item'} ${i + 1}`} onCommit={(v) => props.onChange(props.items.map((x, k) => (k === i ? v : x)))} />
          <button className="btn ghost small ss-x" aria-label="Remove" title="Remove" onClick={() => props.onChange(props.items.filter((_, k) => k !== i))}>✕</button>
        </div>
      ))}
      <button className="btn small ss-add" onClick={() => props.onChange([...props.items, ''])}>+ {props.addLabel ?? 'Add'}</button>
    </div>
  );
}

/** Toggle chips for picking several ids. */
export function MultiPick(props: { options: { id: string; label: string; title?: string }[]; value: string[]; onChange: (v: string[]) => void }) {
  return (
    <div className="ss-pick" role="group">
      {props.options.map((o) => {
        const on = props.value.includes(o.id);
        return (
          <button type="button" key={o.id} className={`ss-pick-opt ${on ? 'on' : ''}`} aria-pressed={on} title={o.title}
            onClick={() => props.onChange(on ? props.value.filter((x) => x !== o.id) : [...props.value, o.id])}>
            {on ? '✓ ' : ''}{o.label}
          </button>
        );
      })}
      {props.options.length === 0 && <span className="muted small">none defined yet</span>}
    </div>
  );
}

// ------------------------------------------------------------------ drawer / modal

export function Drawer(props: { open: boolean; onClose: () => void; title: ReactNode; kicker?: ReactNode; children: ReactNode; wide?: boolean }) {
  useEffect(() => {
    if (!props.open) return;
    const k = (e: KeyboardEvent) => { if (e.key === 'Escape') props.onClose(); };
    window.addEventListener('keydown', k);
    return () => window.removeEventListener('keydown', k);
  }, [props.open, props.onClose]);
  if (!props.open) return null;
  return (
    <div className="ss-overlay" onMouseDown={(e) => { if (e.target === e.currentTarget) props.onClose(); }}>
      <aside className={`ss-drawer ${props.wide ? 'wide' : ''}`} role="dialog" aria-modal="true">
        <div className="ss-drawer-head">
          <div style={{ minWidth: 0 }}>
            {props.kicker && <div className="page-kicker">{props.kicker}</div>}
            <h2 style={{ marginTop: 4 }}>{props.title}</h2>
          </div>
          <button className="btn ghost" onClick={props.onClose} aria-label="Close">✕</button>
        </div>
        <div className="ss-drawer-body">{props.children}</div>
      </aside>
    </div>
  );
}

export function Modal(props: { open: boolean; onClose: () => void; title: ReactNode; children: ReactNode; actions?: ReactNode }) {
  useEffect(() => {
    if (!props.open) return;
    const k = (e: KeyboardEvent) => { if (e.key === 'Escape') props.onClose(); };
    window.addEventListener('keydown', k);
    return () => window.removeEventListener('keydown', k);
  }, [props.open, props.onClose]);
  if (!props.open) return null;
  return (
    <div className="ss-overlay center" onMouseDown={(e) => { if (e.target === e.currentTarget) props.onClose(); }}>
      <div className="ss-modal card" role="dialog" aria-modal="true">
        <h3 style={{ marginBottom: 10 }}>{props.title}</h3>
        <div>{props.children}</div>
        {props.actions && <div className="row" style={{ justifyContent: 'flex-end', marginTop: 16 }}>{props.actions}</div>}
      </div>
    </div>
  );
}

// ------------------------------------------------------------------ motion

export function useReducedMotion(): boolean {
  const [r, setR] = useState(() => typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches);
  useEffect(() => {
    const m = window.matchMedia?.('(prefers-reduced-motion: reduce)');
    if (!m) return;
    const f = () => setR(m.matches);
    m.addEventListener('change', f);
    return () => m.removeEventListener('change', f);
  }, []);
  return !!r;
}

// ------------------------------------------------------------------ metric history

/** Metric values seen during this session, keyed by metric (one point per distinct evaluation). */
const history = new Map<string, { t: number; v: number }[]>();
const seen = new WeakSet<object>();
let version = 0;
const listeners = new Set<() => void>();

export function recordEvaluation(ev: Evaluation | null) {
  if (!ev || seen.has(ev)) return;
  seen.add(ev);
  const t = Date.now();
  let changed = false;
  for (const m of ev.metrics) {
    const h = history.get(m.key) ?? [];
    const last = h[h.length - 1];
    if (!last || Math.abs(last.v - m.value) > 1e-12 * Math.max(1, Math.abs(m.value))) {
      h.push({ t, v: m.value });
      if (h.length > 80) h.shift();
      history.set(m.key, h);
      changed = true;
    }
  }
  if (changed) {
    version++;
    listeners.forEach((l) => l());
  }
}

export function useMetricHistory(key: string | null | undefined): { t: number; v: number }[] {
  useSyncExternalStore((cb) => { listeners.add(cb); return () => { listeners.delete(cb); }; }, () => version);
  return key ? history.get(key) ?? [] : [];
}

export function Sparkline({ values, width = 220, height = 44, limit }: { values: number[]; width?: number; height?: number; limit?: number | null }) {
  if (values.length === 0) return <span className="muted small">no data yet</span>;
  const all = limit != null ? [...values, limit] : values;
  let lo = Math.min(...all), hi = Math.max(...all);
  if (hi - lo < 1e-12) { lo -= 1; hi += 1; }
  const pad = 5;
  const x = (i: number) => pad + (values.length === 1 ? (width - 2 * pad) / 2 : (i / (values.length - 1)) * (width - 2 * pad));
  const y = (v: number) => height - pad - ((v - lo) / (hi - lo)) * (height - 2 * pad);
  const d = values.map((v, i) => `${i ? 'L' : 'M'}${x(i).toFixed(1)},${y(v).toFixed(1)}`).join('');
  return (
    <svg width={width} height={height} role="img" aria-label={`history of ${values.length} values, latest ${fmtNum(values[values.length - 1])}`}>
      {limit != null && <line x1={pad} x2={width - pad} y1={y(limit)} y2={y(limit)} stroke="var(--muted)" strokeDasharray="4 3" />}
      <path d={d} fill="none" stroke="var(--series-1)" strokeWidth={2} strokeLinejoin="round" strokeLinecap="round" />
      {values.map((v, i) => <circle key={i} cx={x(i)} cy={y(v)} r={i === values.length - 1 ? 4 : 2.2} fill="var(--series-1)" stroke="var(--surface-1)" strokeWidth={1.5}><title>{fmtNum(v)}</title></circle>)}
    </svg>
  );
}

/** A small "section title" used inside cards. */
export function Eyebrow({ children }: { children: ReactNode }) {
  return <div className="ss-eyebrow">{children}</div>;
}
