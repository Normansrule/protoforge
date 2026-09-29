// Baselines and the vocabulary behind the Versions page: which versions are
// baselines, the configuration-management loop drawn as an animated cycle,
// and a concept map Onshape ↔ Git ↔ Protoforge (described in our own words).
import { useEffect, useState } from 'react';
import { Card, Chip, Explainer } from '../../components/ui';
import { useReducedMotion } from '../home/motion';
import { useHistory } from './history';

const LOOP = [
  { k: 'Identify', t: 'Name the configuration items and freeze a baseline (a version)' },
  { k: 'Change', t: 'Propose a change on a branch; the labs re-verify it' },
  { k: 'Review', t: 'Compare against the baseline: parameters, BOM, verdicts' },
  { k: 'Approve', t: 'Merge it and save the next baseline' },
  { k: 'Account', t: 'Status accounting: the history says what changed and why' },
  { k: 'Audit', t: 'Check the product matches its baseline (all requirements verified)' },
];

function CmLoop() {
  const reduced = useReducedMotion();
  const [on, setOn] = useState(0);
  const [play, setPlay] = useState(true);
  useEffect(() => {
    if (reduced || !play) return;
    const id = setInterval(() => setOn((i) => (i + 1) % LOOP.length), 1800);
    return () => clearInterval(id);
  }, [reduced, play]);
  const R = 92;
  const cx = 130;
  const cy = 118;
  const pt = (i: number) => {
    const a = -Math.PI / 2 + (2 * Math.PI * i) / LOOP.length;
    return [cx + R * Math.cos(a), cy + R * Math.sin(a)];
  };
  const [dx, dy] = pt(on);
  return (
    <div className="row" style={{ alignItems: 'center', gap: 16 }}>
      <svg viewBox="0 0 260 236" style={{ width: 260, maxWidth: '100%' }} role="img" aria-label="Configuration management loop">
        <circle cx={cx} cy={cy} r={R} fill="none" stroke="var(--border-strong)" strokeWidth={2} className="flow-line" />
        {!reduced && <circle cx={dx} cy={dy} r={16} fill="var(--accent-soft)" style={{ transition: 'cx 0.6s, cy 0.6s' }} />}
        {LOOP.map((s, i) => {
          const [x, y] = pt(i);
          const active = i === on && !reduced;
          return (
            <g key={s.k} onClick={() => { setOn(i); setPlay(false); }} style={{ cursor: 'pointer' }}>
              <circle cx={x} cy={y} r={9} fill={active ? 'var(--accent)' : 'var(--surface-3)'} stroke="var(--border-strong)" />
              <text x={x} y={y + (y > cy ? 24 : -14)} textAnchor="middle" fontSize={11.5} fill={active ? 'var(--text)' : 'var(--text-2)'} fontWeight={active ? 700 : 500}>{s.k}</text>
            </g>
          );
        })}
        <text x={cx} y={cy - 4} textAnchor="middle" fontSize={12} fill="var(--muted)">baseline</text>
        <text x={cx} y={cy + 12} textAnchor="middle" fontSize={12} fill="var(--muted)">→ change → baseline</text>
      </svg>
      <div className="stack" style={{ gap: 6, flex: '1 1 220px' }}>
        {LOOP.map((s, i) => (
          <div key={s.k} className="small" style={{ color: i === on && !reduced ? 'var(--text)' : 'var(--text-2)', fontWeight: i === on && !reduced ? 650 : 400 }}>
            <b>{s.k}.</b> {s.t}
          </div>
        ))}
        {!reduced && <button className="btn small" style={{ alignSelf: 'flex-start' }} onClick={() => setPlay((p) => !p)}>{play ? '❚❚ Pause' : '▶ Play'}</button>}
      </div>
    </div>
  );
}

const MAP: [string, string, string, string][] = [
  ['Mutable state you edit', 'Workspace', 'Working tree', 'Working state (the project every lab reads)'],
  ['Immutable, named snapshot', 'Version', 'Commit / tag', 'Version (a frozen project.json + fingerprint)'],
  ['Parallel line of work', 'Branch (a new workspace from a version)', 'Branch', 'Branch (lane in the graph)'],
  ['Combine two lines', 'Merge a workspace into another', 'Merge (three-way)', 'Merge by choosing → merge version with two parents'],
  ['See what differs', 'Compare', 'diff', 'Compare: parameters, BOM, requirement verdicts, metrics'],
  ['Go back', 'Restore a version into the workspace', 'checkout / revert', 'Restore'],
  ['Formal release', 'Release management (named, approved revisions)', 'Signed tag / release', 'Baseline flag (functional, allocated, product)'],
];

export function Concepts({ onCompare }: { onCompare: (a: string, b: string) => void }) {
  const h = useHistory();
  const baselines = h.versions.filter((v) => v.baseline).sort((a, b) => a.created - b.created);
  return (
    <div className="stack">
      <div className="grid grid-2">
        <Card title="Baselines in this history" subtitle="Flag a version as a baseline from its detail card">
          {baselines.length === 0 ? (
            <div className="small muted">No baseline yet — select a version and choose a baseline type.</div>
          ) : (
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Baseline</th><th>Version</th><th>Verdicts</th><th /></tr></thead>
                <tbody>
                  {baselines.map((v, i) => {
                    const e = v.summary.evaluation;
                    const prev = baselines[i - 1];
                    return (
                      <tr key={v.id}>
                        <td><Chip tone="warn">⚑ {v.baseline}</Chip></td>
                        <td className="small">{v.name}<div className="muted mono">#{v.summary.short} · {v.branch}</div></td>
                        <td className="small">{e ? <>✓ {e.pass}{e.fail ? ` · ✕ ${e.fail}` : ''}{e.unverified ? ` · ○ ${e.unverified}` : ''}</> : '…'}</td>
                        <td>{prev ? <button className="btn small" onClick={() => onCompare(prev.id, v.id)}>Compare ↔ previous</button> : <button className="btn small" onClick={() => onCompare(v.id, 'HEAD')}>Compare ↔ working</button>}</td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
          <Explainer
            title="Functional, allocated and product baselines"
            refs={[
              { label: 'ISO 10007:2017 Quality management — Guidelines for configuration management' },
              { label: 'SAE/EIA-649, Configuration Management Standard (check the current revision)' },
              { label: 'NASA/SP-2016-6105 Rev 2, NASA Systems Engineering Handbook (2016), configuration management' },
              { label: 'INCOSE Systems Engineering Handbook, 5th ed., Wiley, 2023' },
            ]}
          >
            Systems engineering names three classic baselines. The <b>functional baseline</b> freezes what the system must do (the system requirements — Iris's requirement set). The <b>allocated baseline</b> freezes how those requirements are split among subsystems and interfaces, with the analyses that show the split works. The <b>product baseline</b> freezes the build-to design: drawings, the board, the BOM and the manufacturing files. ISO 10007 and EIA-649 describe the same four activities — identification, change control, status accounting and audit — without prescribing a tool; this page is one lightweight way to do them for a student project. Clause-level requirements are in the standards themselves.
          </Explainer>
        </Card>
        <Card title="The configuration-management loop" subtitle="What each part of this page is for">
          <CmLoop />
        </Card>
      </div>
      <Card title="Same ideas, three tools" subtitle="Onshape, Git and Protoforge side by side (described in our words, not quoted)">
        <div className="table-wrap">
          <table className="table">
            <thead><tr><th>Idea</th><th>Onshape</th><th>Git</th><th>Protoforge Versions</th></tr></thead>
            <tbody>{MAP.map((r) => <tr key={r[0]}>{r.map((c, i) => <td key={i} className={i === 0 ? 'small' : 'small'} style={i === 0 ? { fontWeight: 650 } : undefined}>{c}</td>)}</tr>)}</tbody>
          </table>
        </div>
        <div className="grid grid-2" style={{ marginTop: 12 }}>
          <Explainer title="Onshape's model: workspaces and versions" refs={[{ label: 'Onshape Learning Center — Version and history management courses', href: 'https://learn.onshape.com' }, { label: 'Onshape', href: 'https://www.onshape.com/' }]}>
            Onshape keeps every edit in a document's history automatically, so nothing is ever unsaved. A <b>workspace</b> is the editable branch you work in; a <b>version</b> is a named, read-only snapshot of the whole document at one point; creating a <b>branch</b> starts a new workspace from any version, and a <b>merge</b> brings a workspace's changes into another. Because every part, assembly and drawing lives in one document, a version captures them together — the same reason Protoforge versions the whole project.json instead of single files. For the full workflow and current behaviour, follow the Onshape Learning Center courses.
          </Explainer>
          <Explainer title="Git's model: commits, branches, merges" refs={[{ label: 'S. Chacon and B. Straub, Pro Git, 2nd ed., Apress, 2014', href: 'https://git-scm.com/book/en/v2' }, { label: 'V. Driessen, “A successful Git branching model”, 2010', href: 'https://nvie.com/posts/a-successful-git-branching-model/' }]}>
            Git stores snapshots (commits) that point to their parents, so history is a directed acyclic graph; a branch is just a movable name for one commit. A common model keeps a stable <i>main</i> line and does each feature or experiment on its own branch, merged back only when it is ready — here, the <span className="mono">explore/battery-250</span> branch is such an experiment. Each Protoforge version stores its parent (and, for a merge, a second parent), which is exactly what the graph on this page draws.
          </Explainer>
        </div>
      </Card>
    </div>
  );
}
