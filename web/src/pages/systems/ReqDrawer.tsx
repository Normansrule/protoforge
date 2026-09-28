// Requirement detail drawer (also opened by `#/systems?req=R-012`): statement,
// rationale, the full trace chain from need to lab, the metric's history
// during this session, quality findings and an editor.
import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { Icon } from '../../components/Icon';
import {
  Badge, Drawer, Eyebrow, MarginBar, Sparkline, cap, fmtNum, labById, labForMetric, useMetricHistory, useStudio,
} from './shared';
import { ReqForm } from './ReqForm';

export function RequirementDrawer({ id, onClose }: { id: string | null; onClose: () => void }) {
  const { project, ev, studio, update, openReq, go } = useStudio();
  const [editing, setEditing] = useState(false);
  useEffect(() => setEditing(false), [id]);
  const idx = id ? project.requirements.findIndex((r) => r.id === id) : -1;
  const r = idx >= 0 ? project.requirements[idx] : null;
  const res = r ? ev?.results[idx] : undefined;
  const metric = r?.verification.metric ? ev?.metrics.find((m) => m.key === r.verification.metric) : undefined;
  const hist = useMetricHistory(r?.verification.metric);
  const lab = labForMetric(r?.verification.metric);
  if (!id) return null;
  if (!r) {
    return (
      <Drawer open onClose={onClose} title={`${id} not found`}>
        <div className="ss-empty">This project has no requirement {id}.</div>
      </Drawer>
    );
  }
  const issues = (studio?.quality ?? []).filter((q) => q.id === r.id);
  const prev = project.requirements[idx - 1]?.id;
  const next = project.requirements[idx + 1]?.id;
  const needs = r.derives_from.map((n) => project.needs.find((x) => x.id === n) ?? { id: n, text: '(unknown need)', stakeholder: '', priority: 0 });
  const subs = r.allocated_to.map((s) => project.subsystems.find((x) => x.id === s) ?? { id: s, name: '(unknown subsystem)', lab: '', domain: 'electrical' as const, description: '', functions: [] });
  const status = res?.status ?? 'unverified';
  const limitForSpark = r.verification.cmp === 'le' || r.verification.cmp === 'ge' ? r.verification.target : null;

  return (
    <Drawer
      open
      onClose={onClose}
      kicker={<>{cap(r.kind)} requirement · verified by {r.verification.method}</>}
      title={<span className="row" style={{ gap: 10 }}><span className="mono">{r.id}</span> <Badge status={status} /></span>}
    >
      {editing ? (
        <ReqForm
          initial={r}
          isNew={false}
          onCancel={() => setEditing(false)}
          onDelete={() => { if (confirm(`Delete ${r.id}?`)) { update((d) => { d.requirements.splice(idx, 1); }); onClose(); } }}
          onSave={(x) => {
            update((d) => {
              d.requirements[idx] = x;
              // keep ids consistent if it was renamed
            });
            setEditing(false);
            if (x.id !== r.id) openReq(x.id);
          }}
        />
      ) : (
        <>
          <div>
            <div style={{ fontSize: 17, fontWeight: 600, lineHeight: 1.4 }}>{r.text}</div>
            {r.rationale && <div className="muted" style={{ marginTop: 8, fontSize: 13.5 }}><b style={{ color: 'var(--text-2)' }}>Rationale.</b> {r.rationale}</div>}
          </div>
          <div className="grid grid-3" style={{ gap: 10 }}>
            <div className="stat"><div className="stat-label">Criterion</div><div className="mono" style={{ fontWeight: 700, marginTop: 4 }}>{res?.criterion ?? '—'}</div></div>
            <div className="stat"><div className="stat-label">Live value</div><div className="mono" style={{ fontWeight: 700, marginTop: 4 }}>{res?.value != null ? `${fmtNum(res.value)} ${res.unit}` : '—'}</div></div>
            <div className="stat"><div className="stat-label">Margin</div><div style={{ marginTop: 6 }}><MarginBar margin={res?.margin} width={70} /></div></div>
          </div>

          <div>
            <Eyebrow>Trace chain</Eyebrow>
            <div className="ss-chain">
              <Step color="var(--series-7)" k="Stakeholder need" glyph="N">
                {needs.length === 0 ? <span className="muted">not traced to a need</span> : needs.map((n) => <div key={n.id}><span className="ss-id">{n.id}</span> {n.text} <span className="muted small">· {n.stakeholder}</span></div>)}
              </Step>
              <Step color="var(--series-1)" k="Requirement" glyph="R">
                <span className="ss-id">{r.id}</span> {cap(r.kind)} · {r.verification.method}
              </Step>
              <Step color="var(--series-3)" k="Allocated to" glyph="S">
                {subs.length === 0 ? <span className="muted">not allocated</span> : subs.map((s) => {
                  const sl = labById(s.lab);
                  return (
                    <div key={s.id}>
                      <button className="ss-link" onClick={() => go('architecture')}>{s.name}</button> <span className="muted small">{s.id}</span>
                      {sl && <> · <Link to={sl.path} className="small">designed in {sl.title}</Link></>}
                    </div>
                  );
                })}
              </Step>
              <Step color="var(--series-4)" k="Verified by metric" glyph="M">
                {r.verification.metric ? (
                  <div>
                    <span className="mono">{r.verification.metric}</span>
                    {metric ? <> — {metric.label}: <b className="mono">{fmtNum(metric.value)} {metric.unit}</b></> : <span style={{ color: 'var(--warn)' }}> — not produced by any lab yet</span>}
                    {metric?.note && <div className="muted small" style={{ marginTop: 3 }}>{metric.note}</div>}
                  </div>
                ) : <span className="muted">no metric bound: record a {r.verification.method} result</span>}
              </Step>
              <Step color="var(--series-5)" k="Computed in lab" glyph="L">
                {lab ? <Link to={lab.path} className="btn small"><Icon name={lab.icon} size={14} /> Open {lab.title}</Link> : <span className="muted">—</span>}
              </Step>
            </div>
          </div>

          {r.verification.metric && (
            <div>
              <Eyebrow>Metric this session</Eyebrow>
              <div className="row" style={{ alignItems: 'center' }}>
                <Sparkline values={hist.map((h) => h.v)} width={280} height={56} limit={limitForSpark} />
                <div className="muted small" style={{ maxWidth: 240 }}>
                  {hist.length <= 1 ? 'Change a design parameter in a lab or here and come back — every new value is plotted.' : `${hist.length} values; dashed line = limit.`}
                </div>
              </div>
            </div>
          )}

          {issues.length > 0 && (
            <div>
              <Eyebrow>Quality findings (INCOSE Guide to Writing Requirements)</Eyebrow>
              <div className="ss-lint">
                {issues.map((q, i) => (
                  <div className="ss-lint-item" key={i}>
                    <span className={`ss-sev ${q.severity}`}>{q.severity}</span>
                    <div><b>{q.characteristic}:</b> {q.message}{q.hint && <div className="muted small">{q.hint}</div>}</div>
                  </div>
                ))}
              </div>
            </div>
          )}

          <div className="row" style={{ justifyContent: 'space-between' }}>
            <div className="row">
              <button className="btn small" disabled={!prev} onClick={() => prev && openReq(prev)}>← {prev ?? ''}</button>
              <button className="btn small" disabled={!next} onClick={() => next && openReq(next)}>{next ?? ''} →</button>
            </div>
            <button className="btn primary" onClick={() => setEditing(true)}>Edit requirement</button>
          </div>
        </>
      )}
    </Drawer>
  );
}

function Step(props: { color: string; k: string; glyph: string; children: React.ReactNode }) {
  return (
    <div className="ss-chain-step">
      <div className="ss-chain-dot" style={{ background: props.color }} aria-hidden>{props.glyph}</div>
      <div style={{ minWidth: 0, fontSize: 13 }}>
        <div className="ss-chain-k">{props.k}</div>
        {props.children}
      </div>
    </div>
  );
}
