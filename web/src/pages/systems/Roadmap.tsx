// Roadmap: stage-gate milestones with deliverables and exit criteria. The
// engine checks the criteria it can tie to project state (requirement
// status, coverage, trades, risks); the rest take a manual sign-off stored in
// design.extra.studio.gates.
import { useState } from 'react';
import { Card, Explainer } from '../../components/ui';
import type { Project } from '../../engine/types';
import { Badge, DText, Eyebrow, ListEditor, REFS, nextId, useStudio } from './shared';
import type { CheckStatus } from './types';

function setFlag(d: Project, key: string, on: boolean) {
  const extra = d.design.extra as Record<string, unknown>;
  const studio = (extra.studio ??= {}) as Record<string, unknown>;
  const gates = (studio.gates ??= {}) as Record<string, boolean>;
  if (on) gates[key] = true; else delete gates[key];
}

function Check({ c, flagKey }: { c: CheckStatus; flagKey: string }) {
  const { update } = useStudio();
  const color = c.status === 'met' ? 'var(--good)' : c.status === 'unmet' ? 'var(--bad)' : 'var(--warn)';
  const icon = c.status === 'met' ? '✓' : c.status === 'unmet' ? '✕' : '✎';
  return (
    <div className="ss-check">
      <span className="ic" style={{ background: `color-mix(in srgb, ${color} 22%, transparent)`, color: 'var(--text)' }} aria-hidden>{icon}</span>
      <div style={{ minWidth: 0 }}>
        <div>{c.text}</div>
        <div className="muted small">
          {c.auto ? <>auto · {c.status === 'met' ? 'met' : 'not met'} — {c.detail}</> : (
            <label style={{ display: 'inline-flex', gap: 6, alignItems: 'center', cursor: 'pointer' }}>
              <input type="checkbox" checked={c.status === 'met'} onChange={(e) => update((d) => setFlag(d, flagKey, e.target.checked))} style={{ accentColor: 'var(--accent)' }} />
              {c.status === 'met' ? 'signed off' : 'sign off manually'}
            </label>
          )}
        </div>
      </div>
    </div>
  );
}

export function RoadmapTab() {
  const { project, studio, update } = useStudio();
  const [editing, setEditing] = useState(false);
  const gates = studio?.gates ?? [];
  const passed = gates.filter((g) => g.passed).length;
  const cur = gates.findIndex((g) => g.current);
  const n = project.milestones.length;
  const pos = (i: number) => (n <= 1 ? 50 : (i / (n - 1)) * 100);
  return (
    <>
      <Card
        title="Roadmap — stage gates from concept to production"
        subtitle={<>{passed}/{n} gates passed{cur >= 0 ? <> · current gate: <b>{gates[cur].id} {gates[cur].title}</b></> : n ? ' · all gates passed' : ''}</>}
        actions={
          <>
            <button className={`btn small ${editing ? 'primary' : ''}`} onClick={() => setEditing((e) => !e)}>{editing ? 'Done editing' : 'Edit milestones'}</button>
            <button className="btn small" onClick={() => update((d) => { d.milestones.push({ id: nextId('M-', d.milestones.map((m) => m.id), 1), title: 'New milestone', stage: 'Stage', deliverables: [], exit_criteria: [] }); })}>+ Milestone</button>
          </>
        }
      >
        {n === 0 && <div className="ss-empty">No milestones. Load a template (Report & templates) for the standard concept → production gates, or add your own.</div>}
        {n > 0 && (
          <div className="ss-rail" aria-hidden>
            <div className="ss-rail-line" />
            <div className="ss-rail-fill" style={{ width: `${cur < 0 ? 100 : pos(cur)}%` }} />
            {project.milestones.map((m, i) => {
              const g = gates[i];
              return <div key={m.id} className={`ss-diamond ${g?.passed ? 'passed' : ''} ${g?.current ? 'current' : ''}`} style={{ left: `${pos(i)}%` }} title={`${m.id} ${m.title}`} />;
            })}
          </div>
        )}
        <div className="ss-road">
          {project.milestones.map((m, i) => {
            const g = gates[i];
            return (
              <div key={i} className={`card ss-gate ${g?.current ? 'current' : ''}`} style={{ padding: 14 }}>
                <div className="row" style={{ justifyContent: 'space-between', flexWrap: 'nowrap' }}>
                  <span className="page-kicker" style={{ fontSize: 11 }}>{m.id} · {m.stage}</span>
                  {g && <Badge status={g.passed ? 'pass' : g.current ? 'partial' : 'todo'} label={g.passed ? 'passed' : g.current ? 'current gate' : 'ahead'} />}
                </div>
                {editing ? (
                  <div className="ss-form" style={{ marginTop: 8 }}>
                    <DText value={m.title} ariaLabel="Milestone title" onCommit={(v) => update((d) => { d.milestones[i].title = v; })} />
                    <DText value={m.stage} ariaLabel="Stage" onCommit={(v) => update((d) => { d.milestones[i].stage = v; })} />
                    <Eyebrow>Deliverables</Eyebrow>
                    <ListEditor items={m.deliverables} placeholder="deliverable" addLabel="Deliverable" onChange={(v) => update((d) => { d.milestones[i].deliverables = v; })} />
                    <Eyebrow>Exit criteria</Eyebrow>
                    <ListEditor items={m.exit_criteria} placeholder="criterion" addLabel="Criterion" onChange={(v) => update((d) => { d.milestones[i].exit_criteria = v; })} />
                    <button className="btn small" style={{ color: 'var(--bad-text)', justifySelf: 'start' }} onClick={() => { if (confirm(`Delete ${m.id}?`)) update((d) => { d.milestones.splice(i, 1); }); }}>Delete milestone</button>
                  </div>
                ) : (
                  <>
                    <h3 style={{ margin: '6px 0 10px' }}>{m.title}</h3>
                    <Eyebrow>Exit criteria</Eyebrow>
                    {(g?.criteria ?? []).map((c, k) => <Check key={k} c={c} flagKey={`${m.id}#${k}`} />)}
                    {m.exit_criteria.length === 0 && <div className="muted small">none — add some in edit mode</div>}
                    <div style={{ height: 8 }} />
                    <Eyebrow>Deliverables</Eyebrow>
                    {(g?.deliverables ?? []).map((c, k) => <Check key={k} c={c} flagKey={`${m.id}#d${k}`} />)}
                  </>
                )}
              </div>
            );
          })}
        </div>
      </Card>
      <div className="grid grid-2">
        <Explainer title="Stage gates and technical reviews" refs={[REFS.nasa, REFS.iso15288, { label: 'R. G. Cooper, "Stage-gate systems: a new tool for managing new products", Business Horizons 33(3), 1990' }]}>
          A gate is a go / no-go review between stages: the project may only continue when the exit criteria are met. NASA's life cycle uses the same idea with
          technical reviews — the system requirements review (SRR), preliminary design review (PDR) and critical design review (CDR). Hardware companies add
          build stages: EVT (engineering validation test: does the design work?), DVT (design validation test: does it pass environmental, drop and compliance
          tests on production-intent parts?) and PVT (production validation test: can the line build it at yield?).
        </Explainer>
        <Explainer title="Automatic exit criteria" refs={[REFS.nasa]}>
          The engine recognises criteria it can check from the project itself — e.g. “All analysis requirements green”, “Every requirement has a metric and
          an owner”, “Top trades decided”, “Every need satisfied”, “No high risks” — and marks them met or not met live. Anything that needs a physical
          prototype (a day of wear, a yield figure) is a manual sign-off, saved with the project.
        </Explainer>
      </div>
    </>
  );
}
