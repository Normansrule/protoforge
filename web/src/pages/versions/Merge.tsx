// Merge by choosing: bring a branch into the one you are on. The engine's
// three-way merge (bridge.merge) lists every place where the two tips differ
// with the side the rule would take; flip any of them, re-verify the result
// with every lab, then commit it as a merge version with two parents.
import { useEffect, useMemo, useState } from 'react';
import { Card, Chip, ErrorBox, Explainer } from '../../components/ui';
import type { Project } from '../../engine/types';
import { workerCall } from '../../labs/pcb/useWorker';
import { useProject } from '../../state/project';
import type { Diff } from './Compare';
import { branchTip, mergeBase, useHistory, type Summary } from './history';

type Side = 'ours' | 'theirs';
interface Decision { path: string; base: unknown; ours: unknown; theirs: unknown; kind: 'ours' | 'theirs' | 'conflict'; default: Side; chosen: Side }
interface MergeResult { project: Project | null; error: string | null; decisions: Decision[]; conflicts: number; from_theirs: number; summary: Summary | null }

const show = (v: unknown) => {
  if (v === undefined || v === null) return '— (absent)';
  const s = typeof v === 'string' ? v : JSON.stringify(v);
  return s.length > 90 ? `${s.slice(0, 88)}…` : s;
};

const KIND: Record<Decision['kind'], { tone: 'fail' | 'info' | undefined; text: string }> = {
  conflict: { tone: 'fail', text: '✕ both changed' },
  theirs: { tone: 'info', text: '→ only theirs changed' },
  ours: { tone: undefined, text: '← only ours changed' },
};

/** Small animated picture of the three-way merge: base → ours / theirs → merged. */
function MergeFlow({ ours, theirs }: { ours: string; theirs: string }) {
  const P = { base: [40, 60], ours: [200, 24], theirs: [200, 96], merged: [380, 60] } as const;
  const edge = (a: readonly number[], b: readonly number[]) => `M${a[0] + 12} ${a[1]} C${(a[0] + b[0]) / 2} ${a[1]}, ${(a[0] + b[0]) / 2} ${b[1]}, ${b[0] - 12} ${b[1]}`;
  return (
    <svg viewBox="0 0 470 130" className="vm-flow" role="img" aria-label={`Three-way merge of ${theirs} into ${ours}`}>
      {[[P.base, P.ours, 1], [P.base, P.theirs, 2], [P.ours, P.merged, 1], [P.theirs, P.merged, 2]].map(([a, b, c], i) => (
        <path key={i} d={edge(a as readonly number[], b as readonly number[])} fill="none" stroke={`var(--series-${c})`} strokeWidth={2.2} className="flow-line" />
      ))}
      {([['base', 'merge base', 'var(--muted)'], ['ours', ours, 'var(--series-1)'], ['theirs', theirs, 'var(--series-2)'], ['merged', 'merged', 'var(--accent)']] as const).map(([k, label, col]) => {
        const [x, y] = P[k];
        return (
          <g key={k}>
            {k === 'merged' ? <rect x={x - 9} y={y - 9} width={18} height={18} rx={2} transform={`rotate(45 ${x} ${y})`} fill={col} /> : <circle cx={x} cy={y} r={10} fill={col} />}
            <text x={x} y={k === 'theirs' ? y + 24 : y - 16} textAnchor="middle">{label.length > 24 ? `${label.slice(0, 23)}…` : label}</text>
          </g>
        );
      })}
    </svg>
  );
}

export function MergePanel() {
  const h = useHistory();
  const project = useProject((s) => s.project);
  const others = h.branches.map((b) => b.name).filter((b) => b !== h.branch && branchTip(h.versions, b));
  const [from, setFrom] = useState<string>('');
  const src = others.includes(from) ? from : others[0] ?? '';
  const tip = src ? branchTip(h.versions, src) : null;
  const base = tip ? mergeBase(h.versions, h.base, tip.id) : null;
  const [choices, setChoices] = useState<Record<string, Side>>({});
  const [res, setRes] = useState<{ key: string; data: MergeResult | null; error: string | null }>({ key: '', data: null, error: null });
  const [check, setCheck] = useState<{ key: string; diff: Diff | null; busy: boolean; error: string | null }>({ key: '', diff: null, busy: false, error: null });
  const [all, setAll] = useState(false);
  const [name, setName] = useState('');
  const key = `${src}|${tip?.id}|${base?.id}|${h.base}|${JSON.stringify(choices)}|${project.meta.version}`;

  useEffect(() => setChoices({}), [src]);
  useEffect(() => {
    if (!tip) return;
    const input = { base: (base ?? tip).project, ours: project, theirs: tip.project, choices };
    let alive = true;
    workerCall<MergeResult>('bridge.merge', input)
      .then((data) => alive && setRes({ key, data, error: data.error }))
      .catch((e: Error) => alive && setRes({ key, data: null, error: e.message }));
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, project]);

  const decisions = useMemo(() => {
    const d = res.data?.decisions ?? [];
    const order = { conflict: 0, theirs: 1, ours: 2 };
    return [...d].sort((a, b) => order[a.kind] - order[b.kind] || a.path.localeCompare(b.path));
  }, [res.data]);

  if (others.length === 0) {
    return (
      <Card title="Merge a branch" subtitle="Nothing to merge yet">
        <div className="small muted">Create a branch from any version (“Branch from here”), save a version on it, then come back to merge it into {h.branch}.</div>
      </Card>
    );
  }
  const merged = res.data?.project ?? null;
  const verify = () => {
    if (!merged) return;
    const k = res.key;
    setCheck({ key: k, diff: null, busy: true, error: null });
    workerCall<Diff>('bridge.diff', { a: project, b: merged })
      .then((diff) => setCheck({ key: k, diff, busy: false, error: null }))
      .catch((e: Error) => setCheck({ key: k, diff: null, busy: false, error: e.message }));
  };
  const checked = check.key === res.key ? check : null;
  const ev = checked?.diff?.b.evaluation;
  const rows = all ? decisions : decisions.slice(0, 14);
  return (
    <Card title="Merge by choosing" subtitle={`Bring a branch into ${h.branch} — every difference is listed with the side a three-way merge would take; flip any of them`}>
      <div className="row" style={{ gap: 8, marginBottom: 6 }}>
        <label className="small muted" htmlFor="vm-from">Merge</label>
        <select id="vm-from" value={src} onChange={(e) => setFrom(e.target.value)} style={{ maxWidth: 220 }}>
          {others.map((b) => <option key={b} value={b}>{b}</option>)}
        </select>
        <span className="small muted">into <b>{h.branch}</b> (your working state)</span>
      </div>
      <MergeFlow ours={h.branch} theirs={src} />
      <div className="small muted" style={{ marginBottom: 10 }}>
        Merge base: <b>{base ? base.name : 'none found — every difference counts as a change on their side'}</b> · their tip: <b>{tip?.name}</b>
      </div>
      <ErrorBox error={res.error} />
      {res.data && (
        <>
          <div className="row" style={{ gap: 6, marginBottom: 8 }}>
            <Chip tone={res.data.conflicts ? 'fail' : 'pass'}>{res.data.conflicts ? `✕ ${res.data.conflicts} conflict${res.data.conflicts > 1 ? 's' : ''}` : '✓ no conflicts'}</Chip>
            <Chip>{decisions.length} difference{decisions.length === 1 ? '' : 's'}</Chip>
            <Chip tone="info">{res.data.from_theirs} taken from {src}</Chip>
            {Object.keys(choices).length > 0 && <button className="btn small ghost" onClick={() => setChoices({})}>Reset choices</button>}
          </div>
          {decisions.length === 0 ? (
            <div className="small muted">{h.branch} already contains everything on {src}.</div>
          ) : (
            <div className="table-wrap">
              <table className="table vm-table">
                <thead><tr><th>Where</th><th>Base</th><th>Ours ({h.branch})</th><th>Theirs ({src})</th><th>Why</th><th>Take</th></tr></thead>
                <tbody>
                  {rows.map((d) => (
                    <tr key={d.path}>
                      <td className="mono small">{d.path}</td>
                      <td className="vm-val muted">{show(d.base)}</td>
                      <td className="vm-val">{show(d.ours)}</td>
                      <td className="vm-val">{show(d.theirs)}</td>
                      <td><Chip tone={KIND[d.kind].tone}>{KIND[d.kind].text}</Chip></td>
                      <td>
                        <span className="vm-side" role="group" aria-label={`Side for ${d.path}`}>
                          {(['ours', 'theirs'] as const).map((s) => (
                            <button key={s} aria-pressed={d.chosen === s} onClick={() => setChoices((c) => ({ ...c, [d.path]: s }))}>{s}</button>
                          ))}
                        </span>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
          {decisions.length > 14 && <button className="btn small ghost" onClick={() => setAll(!all)}>{all ? 'Show fewer' : `Show all ${decisions.length}`}</button>}
          <div className="row" style={{ gap: 8, marginTop: 12, alignItems: 'flex-end' }}>
            <button className="btn" disabled={!merged || !!checked?.busy} onClick={verify}>{checked?.busy ? 'Evaluating…' : 'Verify the merge'}</button>
            <label className="field" style={{ flex: '1 1 200px', marginBottom: 0 }}>
              <span className="field-top"><span className="field-label">Merge version name</span></span>
              <input type="text" value={name} placeholder={`Merge ${src} into ${h.branch}`} onChange={(e) => setName(e.target.value)} />
            </label>
            <button
              className="btn primary"
              disabled={!merged || decisions.length === 0}
              onClick={() => {
                if (!merged || !tip) return;
                const took = decisions.filter((d) => d.chosen === 'theirs').length;
                h.commitMerge(merged, tip.id, name.trim() || `Merge ${src} into ${h.branch}`, `Three-way merge of ${src} (${tip.name}) into ${h.branch}: ${took} of ${decisions.length} difference(s) taken from ${src}${res.data?.conflicts ? `, ${res.data.conflicts} conflict(s) resolved by choice` : ''}.`);
                setName('');
                setChoices({});
              }}
            >
              Commit merge
            </button>
          </div>
          <ErrorBox error={checked?.error ?? null} />
          {checked?.diff && (
            <div className="callout small" style={{ marginTop: 10 }}>
              <b>Merged result vs your working state:</b> {ev ? <>✓ {ev.pass} pass · ✕ {ev.fail} fail · ○ {ev.unverified} unverified</> : null}
              <ul className="vd-story" style={{ marginTop: 6 }}>{checked.diff.headline.map((l, i) => <li key={i}>{l}</li>)}</ul>
            </div>
          )}
        </>
      )}
      <Explainer
        title="Three-way merge, and how it maps to Onshape and Git"
        refs={[
          { label: 'S. Chacon and B. Straub, Pro Git, 2nd ed., Apress, 2014 — ch. 3 Git Branching', href: 'https://git-scm.com/book/en/v2' },
          { label: 'V. Driessen, “A successful Git branching model”, 2010', href: 'https://nvie.com/posts/a-successful-git-branching-model/' },
          { label: 'Onshape Learning Center — versions, branching and merging', href: 'https://learn.onshape.com' },
        ]}
      >
        A two-way comparison can only say that two files differ; the <b>merge base</b> — the newest version both branches descend from — says <i>who</i> changed what. If only one side moved a value away from the base, that side wins automatically; if both moved it differently, it is a conflict and a person decides. Records with an id (requirements, BOM lines, risks) are merged one by one, so two branches that each add a requirement keep both. Git applies the rule to lines of text; Onshape applies the same idea to features and parts when you merge a branch workspace; here it applies to every field of project.json, and the result is re-verified by all labs before you commit it.
      </Explainer>
    </Card>
  );
}
