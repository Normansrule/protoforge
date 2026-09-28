// Onshape-style version graph: a vertical timeline (newest on top) with one
// lane per branch, named versions as nodes and the working state as a moving
// head attached to the version it started from.
import type { Branch, Summary, Version } from './history';

const ROW = 58;
const LANE = 22;
const X0 = 16;

export interface GraphProps {
  versions: Version[];
  branches: Branch[];
  branch: string;
  base: string | null;
  working: Summary | null;
  modified: boolean;
  selected: string | null;
  compare: [string | null, string | null];
  onSelect: (id: string) => void;
}

function laneColor(i: number) {
  return `var(--series-${(i % 8) + 1})`;
}

function when(t: number) {
  const d = new Date(t);
  return d.toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
}

export function VersionGraph(p: GraphProps) {
  const lanes = p.branches.map((b) => b.name);
  const laneOf = (name: string) => Math.max(0, lanes.indexOf(name));
  const sorted = [...p.versions].sort((a, b) => b.created - a.created);
  // row 0 = working state (head), then versions newest first
  const rowOf = new Map<string, number>();
  sorted.forEach((v, i) => rowOf.set(v.id, i + 1));
  rowOf.set('HEAD', 0);
  const nodes = [{ id: 'HEAD', lane: laneOf(p.branch) }, ...sorted.map((v) => ({ id: v.id, lane: laneOf(v.branch) }))];
  const x = (lane: number) => X0 + lane * LANE;
  const y = (row: number) => row * ROW + ROW / 2;
  const edges: { d: string; lane: number; dashed: boolean }[] = [];
  const link = (x1: number, y1: number, x2: number, y2: number) =>
    x1 === x2 ? `M${x1} ${y1} L${x2} ${y2}` : `M${x1} ${y1} L${x1} ${y2 - ROW / 2} Q${x1} ${y2} ${x2 + (x1 > x2 ? 10 : -10)} ${y2} L${x2} ${y2}`;
  for (const v of sorted) {
    if (!v.parent || !rowOf.has(v.parent)) continue;
    const pv = p.versions.find((q) => q.id === v.parent)!;
    const [x1, y1, x2, y2] = [x(laneOf(v.branch)), y(rowOf.get(v.id)!), x(laneOf(pv.branch)), y(rowOf.get(pv.id)!)];
    edges.push({ d: link(x1, y1, x2, y2), lane: laneOf(v.branch), dashed: false });
  }
  if (p.base && rowOf.has(p.base)) {
    const bv = p.versions.find((q) => q.id === p.base)!;
    const [x1, y1, x2, y2] = [x(laneOf(p.branch)), y(0), x(laneOf(bv.branch)), y(rowOf.get(bv.id)!)];
    edges.push({ d: link(x1, y1, x2, y2), lane: laneOf(p.branch), dashed: true });
  }
  const W = X0 + Math.max(1, lanes.length) * LANE;
  const H = nodes.length * ROW;
  const ev = (s: Summary) => s.evaluation;
  return (
    <div className="vg" style={{ minHeight: H }}>
      <svg className="vg-svg" width={W} height={H} aria-hidden>
        {edges.map((e, i) => <path key={i} d={e.d} fill="none" stroke={laneColor(e.lane)} strokeWidth={2.2} strokeDasharray={e.dashed ? '4 4' : undefined} className={e.dashed ? 'flow-line' : undefined} />)}
        {nodes.map((n) => {
          const row = rowOf.get(n.id)!;
          const v = p.versions.find((q) => q.id === n.id);
          const sel = p.selected === n.id || p.compare.includes(n.id);
          if (n.id === 'HEAD') {
            return <circle key={n.id} cx={x(n.lane)} cy={y(row)} r={8} fill="var(--surface-1)" stroke={laneColor(n.lane)} strokeWidth={2.5} strokeDasharray="3 2" className={p.modified ? 'vg-pulse' : undefined} />;
          }
          const fail = v && ev(v.summary) ? ev(v.summary)!.fail > 0 : false;
          return (
            <g key={n.id}>
              {sel && <circle cx={x(n.lane)} cy={y(row)} r={12} fill="none" stroke="var(--accent)" strokeWidth={2} />}
              <circle cx={x(n.lane)} cy={y(row)} r={7.5} fill={laneColor(n.lane)} stroke="var(--surface-1)" strokeWidth={2} />
              {fail && <circle cx={x(n.lane) + 6} cy={y(row) - 6} r={3.5} fill="var(--bad)" />}
            </g>
          );
        })}
      </svg>
      <div className="vg-rows" style={{ marginLeft: W + 8 }}>
        <button className={`vg-row ${p.selected === 'HEAD' ? 'sel' : ''}`} style={{ height: ROW }} onClick={() => p.onSelect('HEAD')}>
          <div className="vg-title">Working state <span className="chip info" style={{ padding: '0 7px' }}>{p.branch}</span> {p.modified ? <span className="chip warn" style={{ padding: '0 7px' }}>● modified</span> : <span className="chip" style={{ padding: '0 7px' }}>= base</span>}</div>
          <div className="vg-meta">{p.working ? <>#{p.working.short} · {p.working.requirements} requirements · {p.working.components} BOM lines</> : '…'}</div>
        </button>
        {sorted.map((v) => {
          const e = ev(v.summary);
          return (
            <button key={v.id} className={`vg-row ${p.selected === v.id ? 'sel' : ''}`} style={{ height: ROW }} onClick={() => p.onSelect(v.id)}>
              <div className="vg-title">
                {v.name}
                {v.kind !== 'manual' && <span className="chip" style={{ padding: '0 7px' }}>{v.kind}</span>}
                {v.branch !== 'main' && <span className="chip" style={{ padding: '0 7px', color: laneColor(laneOf(v.branch)) }}>{v.branch}</span>}
                {p.compare[0] === v.id && <span className="chip info" style={{ padding: '0 7px' }}>A</span>}
                {p.compare[1] === v.id && <span className="chip info" style={{ padding: '0 7px' }}>B</span>}
              </div>
              <div className="vg-meta">
                #{v.summary.short} · {when(v.created)} · {e ? <><span style={{ color: 'var(--good-text)' }}>✓ {e.pass}</span>{e.fail > 0 && <span style={{ color: 'var(--bad-text)' }}> ✕ {e.fail}</span>}{e.unverified > 0 && <> ○ {e.unverified}</>}</> : 'evaluating…'}
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}
