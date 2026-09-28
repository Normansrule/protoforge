// Requirements: the verification matrix with filters and sort, add/edit,
// the quality lint panel and the needs they derive from.
import { useDeferredValue, useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, Explainer } from '../../components/ui';
import { Icon } from '../../components/Icon';
import type { ReqKind, Requirement, Status } from '../../engine/types';
import { Badge, Drawer, KINDS, MarginBar, REFS, cap, fmtNum, labForMetric, useStudio } from './shared';
import { ReqForm, blankRequirement } from './ReqForm';
import { NeedsEditor } from './Needs';

type SortKey = 'id' | 'kind' | 'status' | 'margin' | 'lab';

const STATUS_RANK: Record<Status, number> = { fail: 0, unverified: 1, pass: 2 };

export function RequirementsTab() {
  const { project, ev, studio, update, openReq } = useStudio();
  const [q, setQ] = useState('');
  const dq = useDeferredValue(q.toLowerCase());
  const [kind, setKind] = useState<ReqKind | ''>('');
  const [status, setStatus] = useState<Status | ''>('');
  const [sub, setSub] = useState('');
  const [sort, setSort] = useState<{ key: SortKey; dir: 1 | -1 }>({ key: 'id', dir: 1 });
  const [creating, setCreating] = useState<Requirement | null>(null);

  const rows = useMemo(() => {
    const out = project.requirements.map((r, i) => ({ r, res: ev?.results[i], lab: labForMetric(r.verification.metric) }));
    const f = out.filter(({ r, res }) =>
      (!kind || r.kind === kind) &&
      (!status || (res?.status ?? 'unverified') === status) &&
      (!sub || r.allocated_to.includes(sub)) &&
      (!dq || `${r.id} ${r.text} ${r.rationale} ${r.verification.metric ?? ''}`.toLowerCase().includes(dq)));
    const key = (x: (typeof out)[number]): string | number => {
      switch (sort.key) {
        case 'id': return x.r.id;
        case 'kind': return x.r.kind;
        case 'status': return STATUS_RANK[x.res?.status ?? 'unverified'];
        case 'margin': return x.res?.margin ?? Number.POSITIVE_INFINITY;
        case 'lab': return x.lab?.title ?? '~';
      }
    };
    return f.sort((a, b) => {
      const ka = key(a), kb = key(b);
      return (ka < kb ? -1 : ka > kb ? 1 : 0) * sort.dir;
    });
  }, [project.requirements, ev, kind, status, sub, dq, sort]);

  const th = (k: SortKey, label: string) => (
    <th aria-sort={sort.key === k ? (sort.dir === 1 ? 'ascending' : 'descending') : 'none'}>
      <button className="ss-th-sort" onClick={() => setSort((s) => ({ key: k, dir: s.key === k ? (-s.dir as 1 | -1) : 1 }))}>
        {label}{sort.key === k ? (sort.dir === 1 ? ' ▲' : ' ▼') : ''}
      </button>
    </th>
  );
  const subName = (id: string) => project.subsystems.find((s) => s.id === id)?.name ?? id;
  const quality = studio?.quality ?? [];
  const cov = studio?.coverage;

  return (
    <>
      <Card
        title="Requirements & verification matrix"
        subtitle={<>{project.requirements.length} requirements · {ev?.pass ?? 0} pass · {ev?.fail ?? 0} fail · {ev?.unverified ?? 0} unverified. Click a row for the full trace; values update live as you change the design in any lab.</>}
        actions={<button className="btn primary small" onClick={() => setCreating(blankRequirement(project.requirements.map((r) => r.id)))}>+ Requirement</button>}
      >
        <div className="ss-toolbar">
          <input type="search" placeholder="Search text, id or metric…" value={q} onChange={(e) => setQ(e.target.value)} aria-label="Search requirements" />
          <select value={kind} onChange={(e) => setKind(e.target.value as ReqKind | '')} aria-label="Filter by kind">
            <option value="">All kinds</option>
            {KINDS.map((k) => <option key={k} value={k}>{cap(k)}</option>)}
          </select>
          <select value={status} onChange={(e) => setStatus(e.target.value as Status | '')} aria-label="Filter by status">
            <option value="">All statuses</option>
            <option value="pass">✓ Pass</option><option value="fail">✕ Fail</option><option value="unverified">○ Unverified</option>
          </select>
          <select value={sub} onChange={(e) => setSub(e.target.value)} aria-label="Filter by subsystem">
            <option value="">All subsystems</option>
            {project.subsystems.map((s) => <option key={s.id} value={s.id}>{s.name}</option>)}
          </select>
          {(q || kind || status || sub) && <button className="btn small ghost" onClick={() => { setQ(''); setKind(''); setStatus(''); setSub(''); }}>Clear filters</button>}
          <span className="muted small" style={{ marginLeft: 'auto' }}>{rows.length} shown</span>
        </div>
        <div className="table-wrap">
          <table className="table ss-req-table">
            <thead>
              <tr>
                {th('id', 'ID')}
                {th('kind', 'Requirement')}
                <th>Derives</th>
                <th>Allocated to</th>
                <th>Criterion</th>
                <th>Value</th>
                {th('margin', 'Margin')}
                {th('status', 'Status')}
                {th('lab', 'Lab')}
              </tr>
            </thead>
            <tbody>
              {rows.map(({ r, res, lab }) => (
                <tr key={r.id} className="clickable" onClick={() => openReq(r.id)} tabIndex={0} onKeyDown={(e) => { if (e.key === 'Enter') openReq(r.id); }}>
                  <td className="ss-id">{r.id}</td>
                  <td className="ss-req-text">{r.text}<div><span className="ss-tag">{r.kind}</span>{r.verification.metric && <span className="muted mono" style={{ fontSize: 11 }}>{r.verification.metric}</span>}</div></td>
                  <td className="small">{r.derives_from.join(', ') || <span className="muted">—</span>}</td>
                  <td className="small">{r.allocated_to.map((s) => <span key={s} className="ss-tag" title={s}>{subName(s)}</span>)}{r.allocated_to.length === 0 && <span className="muted">—</span>}</td>
                  <td className="mono small" style={{ whiteSpace: 'nowrap' }}>{res?.criterion}</td>
                  <td className="mono small" style={{ whiteSpace: 'nowrap' }}>{res?.value != null ? `${fmtNum(res.value)} ${res.unit}` : '—'}</td>
                  <td><MarginBar margin={res?.margin} width={64} /></td>
                  <td><Badge status={res?.status ?? 'unverified'} /></td>
                  <td onClick={(e) => e.stopPropagation()}>
                    {lab ? <Link to={lab.path} className="small" style={{ whiteSpace: 'nowrap' }} title={`Open ${lab.title}`}><Icon name={lab.icon} size={13} /> {lab.title.replace(/ (Lab|Studio|Hub)$/, '')}</Link> : <span className="muted small">{r.verification.method}</span>}
                  </td>
                </tr>
              ))}
              {rows.length === 0 && (
                <tr><td colSpan={9}><div className="ss-empty">{project.requirements.length === 0 ? 'No requirements yet — add the first one.' : 'Nothing matches the filters.'}</div></td></tr>
              )}
            </tbody>
          </table>
        </div>
      </Card>

      <div className="grid grid-2">
        <Card
          title="Requirement quality"
          subtitle={<>{quality.filter((x) => x.severity === 'error').length} errors · {quality.filter((x) => x.severity === 'warning').length} warnings · {quality.filter((x) => x.severity === 'info').length} notes</>}
        >
          {quality.length === 0 && <div className="ss-empty">✓ No findings.</div>}
          <div className="ss-lint">
            {quality.map((x, i) => (
              <div className="ss-lint-item" key={i}>
                <span className={`ss-sev ${x.severity}`}>{x.severity === 'error' ? '✕ ' : x.severity === 'warning' ? '! ' : 'i '}{x.severity}</span>
                <div>
                  <button className="ss-link ss-id" onClick={() => project.requirements.some((r) => r.id === x.id) && openReq(x.id)}>{x.id}</button>{' '}
                  <b>{x.characteristic}</b> — {x.message}
                  {x.hint && <div className="muted small">{x.hint}</div>}
                </div>
              </div>
            ))}
          </div>
        </Card>
        <div className="stack">
          {cov && (
            <Card title="Verification mix & coverage" subtitle="How requirements will be verified, and how complete the trace is">
              <div className="grid grid-2" style={{ gap: 10 }}>
                {cov.methods.map((m) => (
                  <div className="stat" key={m.key}><div className="stat-label">{m.key}</div><div className="stat-value tnum">{m.count}</div></div>
                ))}
              </div>
              <div className="ss-kv" style={{ marginTop: 12 }}>
                <dt>Traced to a need</dt><dd>{cov.reqs_traced}/{cov.reqs_total}</dd>
                <dt>Allocated</dt><dd>{cov.reqs_allocated}/{cov.reqs_total}</dd>
                <dt>Bound to a metric</dt><dd>{cov.reqs_with_metric}/{cov.reqs_total} ({cov.reqs_metric_found} produced by a lab)</dd>
                <dt>Needs covered</dt><dd>{cov.needs_covered}/{cov.needs_total}</dd>
              </div>
            </Card>
          )}
          <Explainer title="Writing good requirements" refs={[REFS.gtwr, REFS.incose, REFS.nasa]}>
            The INCOSE <i>Guide to Writing Requirements</i> asks each requirement to be <b>necessary</b>, <b>unambiguous</b>, <b>singular</b> (one thought),
            <b> feasible</b>, <b>verifiable</b> and <b>correct</b>, and the set to be <b>complete</b> and <b>consistent</b>. In practice: use “shall”, state a
            measurable value with a unit, avoid vague words (“fast”, “robust”, “sufficient”) and escape clauses (“if possible”), record the rationale, trace it
            to a need and allocate it to an owner. Verification methods follow the NASA handbook: <i>analysis</i> (models — what the labs do), <i>test</i>,
            <i> inspection</i> and <i>demonstration</i>.
          </Explainer>
        </div>
      </div>

      <NeedsEditor />

      <Drawer open={!!creating} onClose={() => setCreating(null)} title="New requirement" kicker="Requirements">
        {creating && (
          <ReqForm
            initial={creating}
            isNew
            onCancel={() => setCreating(null)}
            onSave={(r) => { update((d) => { d.requirements.push(r); }); setCreating(null); openReq(r.id); }}
          />
        )}
      </Drawer>
    </>
  );
}
