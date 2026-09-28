// Add / edit a requirement: text, rationale, kind, trace links, verification
// method, metric (from everything the engine currently produces) and criterion.
import { useEffect, useMemo, useState } from 'react';
import type { Cmp, Metric, Requirement } from '../../engine/types';
import { CMPS, KINDS, METHODS, MultiPick, cap, fmtNum, labForMetric, nextId, signedPct, useStudio } from './shared';

/** Same rule as `systems::check` in Rust: (pass, margin as a fraction of the target). */
export function checkCriterion(cmp: Cmp, t: number, hi: number | null, v: number): [boolean, number] {
  const scale = (d: number, base: number) => (Math.abs(base) > 1e-12 ? d / Math.abs(base) : d);
  switch (cmp) {
    case 'le': return [v <= t + 1e-9, scale(t - v, t)];
    case 'ge': return [v >= t - 1e-9, scale(v - t, t)];
    case 'eq': { const tol = Math.abs(hi ?? 0); const d = tol - Math.abs(v - t); return [d >= -1e-9, scale(d, t)]; }
    case 'between': { const h = hi ?? t; const d = Math.min(v - t, h - v); return [d >= -1e-9, scale(d, Math.max(h - t, 1e-12))]; }
  }
}

export function blankRequirement(ids: string[]): Requirement {
  return {
    id: nextId('R-', ids, 3), text: 'The system shall ', rationale: '', kind: 'performance', derives_from: [], allocated_to: [],
    verification: { method: 'analysis', metric: null, cmp: 'le', target: 0, target_hi: null, unit: '' },
  };
}

export function ReqForm(props: { initial: Requirement; isNew: boolean; onSave: (r: Requirement) => void; onCancel: () => void; onDelete?: () => void }) {
  const { project, ev } = useStudio();
  const [r, setR] = useState<Requirement>(() => structuredClone(props.initial));
  const set = (patch: Partial<Requirement>) => setR((x) => ({ ...x, ...patch }));
  const setV = (patch: Partial<Requirement['verification']>) => setR((x) => ({ ...x, verification: { ...x.verification, ...patch } }));
  const metrics = ev?.metrics ?? [];
  const groups = useMemo(() => {
    const g = new Map<string, Metric[]>();
    for (const m of metrics) {
      const lab = labForMetric(m.key)?.title ?? m.source;
      g.set(lab, [...(g.get(lab) ?? []), m]);
    }
    return [...g.entries()];
  }, [metrics]);
  const metric = metrics.find((m) => m.key === r.verification.metric) ?? null;
  const idTaken = project.requirements.some((x) => x.id === r.id && x.id !== props.initial.id);
  const v = r.verification;
  const preview = metric ? checkCriterion(v.cmp, v.target, v.target_hi, metric.value) : null;
  const valid = r.id.trim() !== '' && !idTaken && r.text.trim() !== '';

  return (
    <form className="ss-form" onSubmit={(e) => { e.preventDefault(); if (valid) props.onSave(r); }}>
      <div className="two">
        <label className="f">ID
          <input type="text" value={r.id} onChange={(e) => set({ id: e.target.value.trim() })} aria-invalid={idTaken} />
          {idTaken && <span className="small" style={{ color: 'var(--bad-text)' }}>✕ id already used</span>}
        </label>
        <label className="f">Kind
          <select value={r.kind} onChange={(e) => set({ kind: e.target.value as Requirement['kind'] })}>
            {KINDS.map((k) => <option key={k} value={k}>{cap(k)}</option>)}
          </select>
        </label>
      </div>
      <label className="f">Requirement statement
        <textarea className="ss-prose" rows={3} value={r.text} onChange={(e) => set({ text: e.target.value })} placeholder="The <entity> shall <action> <measure>." />
        <span className="muted small">Pattern: “The &lt;entity&gt; shall &lt;action&gt; &lt;measurable value + unit&gt;”. One thought per requirement; avoid vague words.</span>
      </label>
      <label className="f">Rationale
        <textarea className="ss-prose" rows={2} value={r.rationale} onChange={(e) => set({ rationale: e.target.value })} placeholder="Why it exists and where the number came from" />
      </label>
      <div className="f" style={{ display: 'grid', gap: 4, fontSize: 12.5, color: 'var(--text-2)' }}>
        Derives from (needs)
        <MultiPick options={project.needs.map((n) => ({ id: n.id, label: n.id, title: n.text }))} value={r.derives_from} onChange={(x) => set({ derives_from: x })} />
      </div>
      <div className="f" style={{ display: 'grid', gap: 4, fontSize: 12.5, color: 'var(--text-2)' }}>
        Allocated to (subsystems)
        <MultiPick options={project.subsystems.map((s) => ({ id: s.id, label: s.name, title: s.id }))} value={r.allocated_to} onChange={(x) => set({ allocated_to: x })} />
      </div>

      <div className="card" style={{ padding: 12, boxShadow: 'none', background: 'var(--surface-2)' }}>
        <div className="ss-eyebrow">Verification</div>
        <div className="two">
          <label className="f">Method
            <select value={v.method} onChange={(e) => setV({ method: e.target.value as Requirement['verification']['method'] })}>
              {METHODS.map((k) => <option key={k} value={k}>{cap(k)}</option>)}
            </select>
          </label>
          <label className="f">Metric (computed by a lab)
            <select value={v.metric ?? ''} onChange={(e) => {
              const key = e.target.value || null;
              const m = metrics.find((x) => x.key === key);
              setV({
                metric: key,
                unit: m ? m.unit : v.unit,
                target: m && v.target === 0 ? Number(m.value.toPrecision(2)) : v.target,
              });
            }}>
              <option value="">— none (test / inspection / demonstration) —</option>
              {groups.map(([lab, ms]) => (
                <optgroup key={lab} label={lab}>
                  {ms.map((m) => <option key={m.key} value={m.key}>{m.label} — {fmtNum(m.value)} {m.unit} ({m.key})</option>)}
                </optgroup>
              ))}
              {v.metric && !metric && <option value={v.metric}>{v.metric} (not produced)</option>}
            </select>
          </label>
        </div>
        <div className="three">
          <label className="f">Criterion
            <select value={v.cmp} onChange={(e) => setV({ cmp: e.target.value as Cmp, target_hi: e.target.value === 'between' || e.target.value === 'eq' ? v.target_hi ?? v.target : null })}>
              {CMPS.map((c) => <option key={c.value} value={c.value}>{c.label}</option>)}
            </select>
          </label>
          <label className="f">{v.cmp === 'between' ? 'Low' : 'Target'}
            <NumText value={v.target} onChange={(n) => setV({ target: n })} />
          </label>
          {(v.cmp === 'between' || v.cmp === 'eq') ? (
            <label className="f">{v.cmp === 'between' ? 'High' : '± tolerance'}
              <NumText value={v.target_hi ?? 0} onChange={(n) => setV({ target_hi: n })} />
            </label>
          ) : (
            <label className="f">Unit<input type="text" value={v.unit} onChange={(e) => setV({ unit: e.target.value })} /></label>
          )}
        </div>
        {(v.cmp === 'between' || v.cmp === 'eq') && <label className="f">Unit<input type="text" value={v.unit} onChange={(e) => setV({ unit: e.target.value })} /></label>}
        {metric && preview && (
          <div className="ss-metric-preview" style={{ marginTop: 10 }}>
            <b>{metric.label}</b> is currently <b className="mono">{fmtNum(metric.value)} {metric.unit}</b> → would{' '}
            <span style={{ color: preview[0] ? 'var(--good-text)' : 'var(--bad-text)', fontWeight: 700 }}>{preview[0] ? '✓ pass' : '✕ fail'}</span> with margin {signedPct(preview[1])}.
            {metric.note && <div className="muted small" style={{ marginTop: 4 }}>{metric.note}</div>}
          </div>
        )}
        {!v.metric && <div className="muted small" style={{ marginTop: 8 }}>Without a metric the requirement stays “unverified” until you record a test, inspection or demonstration result.</div>}
      </div>

      <div className="row" style={{ justifyContent: 'space-between' }}>
        <div>{props.onDelete && <button type="button" className="btn" style={{ color: 'var(--bad-text)' }} onClick={props.onDelete}>Delete</button>}</div>
        <div className="row">
          <button type="button" className="btn" onClick={props.onCancel}>Cancel</button>
          <button type="submit" className="btn primary" disabled={!valid}>{props.isNew ? 'Add requirement' : 'Save'}</button>
        </div>
      </div>
    </form>
  );
}

/** Number input that keeps what you type ("-", "", "47.") and only reports valid numbers. */
function NumText({ value, onChange }: { value: number; onChange: (n: number) => void }) {
  const [text, setText] = useState(String(value));
  useEffect(() => {
    if (Number(text) !== value) setText(String(value));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value]);
  return (
    <input
      type="text"
      inputMode="decimal"
      value={text}
      onChange={(e) => {
        setText(e.target.value);
        const n = Number(e.target.value);
        if (e.target.value.trim() !== '' && Number.isFinite(n)) onChange(n);
      }}
    />
  );
}
