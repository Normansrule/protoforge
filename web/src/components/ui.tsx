// Shared UI primitives. Keep labs visually consistent by building from these.
import { useState, type ReactNode } from 'react';

export function Card(props: { title?: ReactNode; subtitle?: ReactNode; actions?: ReactNode; children: ReactNode; className?: string; style?: React.CSSProperties }) {
  return (
    <section className={`card fade-in ${props.className ?? ''}`} style={props.style}>
      {(props.title || props.actions) && (
        <div className="card-head">
          <div>
            {props.title && <h3>{props.title}</h3>}
            {props.subtitle && <div className="card-sub">{props.subtitle}</div>}
          </div>
          {props.actions && <div className="card-actions">{props.actions}</div>}
        </div>
      )}
      {props.children}
    </section>
  );
}

export type Tone = 'pass' | 'fail' | 'warn' | 'info' | undefined;

export function Stat(props: { label: string; value: ReactNode; unit?: string; note?: ReactNode; tone?: Tone }) {
  const color = props.tone === 'pass' ? 'var(--good-text)' : props.tone === 'fail' ? 'var(--bad-text)' : props.tone === 'warn' ? 'var(--warn)' : undefined;
  return (
    <div className="stat">
      <div className="stat-label">{props.label}</div>
      <div className="stat-value" style={{ color }}>
        {props.value}
        {props.unit && <span className="stat-unit">{props.unit}</span>}
      </div>
      {props.note && <div className="stat-note">{props.note}</div>}
    </div>
  );
}

export function Chip(props: { tone?: Tone; children: ReactNode; title?: string }) {
  return <span className={`chip ${props.tone ?? ''}`} title={props.title}>{props.children}</span>;
}

/** Status chip that never relies on colour alone: icon + word. */
export function StatusChip({ status }: { status: 'pass' | 'fail' | 'unverified' | 'warn' }) {
  if (status === 'pass') return <Chip tone="pass">✓ pass</Chip>;
  if (status === 'fail') return <Chip tone="fail">✕ fail</Chip>;
  if (status === 'warn') return <Chip tone="warn">! check</Chip>;
  return <Chip>○ unverified</Chip>;
}

export function Slider(props: {
  label: string; value: number; min: number; max: number; step?: number; unit?: string;
  onChange: (v: number) => void; log?: boolean; format?: (v: number) => string; hint?: string;
}) {
  const { min, max, log } = props;
  const toPos = (v: number) => (log ? Math.log10(v / min) / Math.log10(max / min) : (v - min) / (max - min));
  const fromPos = (p: number) => (log ? min * (max / min) ** p : min + p * (max - min));
  const step = props.step ?? (max - min) / 200;
  const shown = props.format ? props.format(props.value) : formatValue(props.value);
  return (
    <label className="field">
      <span className="field-top">
        <span className="field-label" title={props.hint}>{props.label}</span>
        <span className="field-value">{shown}{props.unit ? ` ${props.unit}` : ''}</span>
      </span>
      <input
        type="range"
        min={0}
        max={1000}
        value={Math.round(toPos(props.value) * 1000)}
        onChange={(e) => {
          let v = fromPos(Number(e.target.value) / 1000);
          if (!log && step) v = Math.round(v / step) * step;
          props.onChange(Number(v.toPrecision(6)));
        }}
      />
    </label>
  );
}

function formatValue(v: number) {
  if (Math.abs(v) >= 1000) return v.toFixed(0);
  if (Math.abs(v) >= 10) return v.toFixed(1);
  if (Math.abs(v) >= 1) return v.toFixed(2);
  return v.toPrecision(3);
}

export function NumberField(props: { label: string; value: number; onChange: (v: number) => void; step?: number; unit?: string }) {
  return (
    <label className="field">
      <span className="field-top"><span className="field-label">{props.label}</span>{props.unit && <span className="muted small">{props.unit}</span>}</span>
      <input type="number" value={props.value} step={props.step ?? 'any'} onChange={(e) => props.onChange(Number(e.target.value))} />
    </label>
  );
}

export function Select<T extends string>(props: { label?: string; value: T; options: { value: T; label: string }[]; onChange: (v: T) => void }) {
  return (
    <label className="field">
      {props.label && <span className="field-top"><span className="field-label">{props.label}</span></span>}
      <select value={props.value} onChange={(e) => props.onChange(e.target.value as T)}>
        {props.options.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
      </select>
    </label>
  );
}

export function Toggle(props: { label: string; value: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="small" style={{ cursor: 'pointer', marginBottom: 10, display: 'flex', gap: 8, alignItems: 'flex-start' }}>
      <input type="checkbox" checked={props.value} onChange={(e) => props.onChange(e.target.checked)} style={{ accentColor: 'var(--accent)', marginTop: 3, flex: '0 0 auto' }} />
      <span className="field-label">{props.label}</span>
    </label>
  );
}

export function Tabs<T extends string>(props: { tabs: { id: T; label: ReactNode }[]; value: T; onChange: (t: T) => void }) {
  return (
    <div className="tabs" role="tablist">
      {props.tabs.map((t) => (
        <button key={t.id} role="tab" aria-selected={props.value === t.id} className={`tab ${props.value === t.id ? 'active' : ''}`} onClick={() => props.onChange(t.id)}>
          {t.label}
        </button>
      ))}
    </div>
  );
}

export function useTabs<T extends string>(initial: T) {
  return useState<T>(initial);
}

/** "How it works" teaching block with optional references. */
export function Explainer(props: { title: string; children: ReactNode; refs?: { label: string; href?: string }[] }) {
  return (
    <div className="callout">
      <h4>{props.title}</h4>
      <div>{props.children}</div>
      {props.refs && props.refs.length > 0 && (
        <div className="small" style={{ marginTop: 8 }}>
          <span className="muted">References: </span>
          {props.refs.map((r, i) => (
            <span key={i}>
              {i > 0 && ' · '}
              {r.href ? <a href={r.href} target="_blank" rel="noreferrer">{r.label}</a> : r.label}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

export function Formula({ children }: { children: ReactNode }) {
  return <div className="formula">{children}</div>;
}

export function Code(props: { code: string; filename?: string; maxHeight?: number }) {
  const [copied, setCopied] = useState(false);
  return (
    <div style={{ position: 'relative' }}>
      <div className="row" style={{ justifyContent: 'space-between', marginBottom: 6 }}>
        <span className="muted small mono">{props.filename}</span>
        <div className="row">
          <button
            className="btn small"
            onClick={() => {
              navigator.clipboard?.writeText(props.code).then(() => {
                setCopied(true);
                setTimeout(() => setCopied(false), 1200);
              });
            }}
          >
            {copied ? 'Copied' : 'Copy'}
          </button>
          {props.filename && <DownloadButton filename={props.filename} content={props.code} small />}
        </div>
      </div>
      <pre className="code" style={{ maxHeight: props.maxHeight }}>{props.code}</pre>
    </div>
  );
}

export function downloadBlob(filename: string, blob: Blob) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 2000);
}

export function downloadText(filename: string, content: string, mime = 'text/plain') {
  downloadBlob(filename, new Blob([content], { type: mime }));
}

export function DownloadButton(props: { filename: string; content: string | (() => string | Blob); label?: string; small?: boolean; primary?: boolean }) {
  return (
    <button
      className={`btn ${props.small ? 'small' : ''} ${props.primary ? 'primary' : ''}`}
      onClick={() => {
        const c = typeof props.content === 'function' ? props.content() : props.content;
        if (c instanceof Blob) downloadBlob(props.filename, c);
        else downloadText(props.filename, c);
      }}
    >
      ↓ {props.label ?? props.filename}
    </button>
  );
}

export function EmptyState({ children }: { children: ReactNode }) {
  return <div className="muted small" style={{ padding: 24, textAlign: 'center' }}>{children}</div>;
}

export function ErrorBox({ error }: { error: string | null }) {
  if (!error) return null;
  return <div className="callout" style={{ borderLeftColor: 'var(--bad)', color: 'var(--bad-text)' }}>{error}</div>;
}
