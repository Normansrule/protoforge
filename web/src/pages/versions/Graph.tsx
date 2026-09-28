// Onshape-style version graph: a vertical timeline (newest on top) with one
// lane per branch, named versions as nodes, merge edges from the branch tip
// that was merged in, baseline flags, and the working state as a dashed head
// attached to the version it started from. "Replay" rebuilds the history
// oldest-first (requestAnimationFrame-free: a short interval; skipped with
// prefers-reduced-motion).
import { useEffect, useState } from 'react';
import { useReducedMotion } from '../home/motion';
import { useElementWidth } from '../../components/useElementWidth';
import type { Branch, Summary, Version } from './history';

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
  const reduced = useReducedMotion();
  const [wrapRef, width] = useElementWidth<HTMLDivElement>();
  const narrow = width > 0 && width < 520;
  const ROW = narrow ? 88 : 66;
  // lanes: branches that have versions or are current, in creation order
  const lanes = p.branches.map((b) => b.name);
  const laneOf = (name: string) => Math.max(0, lanes.indexOf(name));
  const sorted = [...p.versions].sort((a, b) => b.created - a.created);
  const n = sorted.length;
  // replay: number of versions revealed, oldest first (n = all)
  const [shown, setShown] = useState(n);
  const [replaying, setReplaying] = useState(false);
  useEffect(() => {
    if (!replaying) setShown(n);
  }, [n, replaying]);
  useEffect(() => {
    if (!replaying) return;
    const id = setInterval(() => {
      setShown((s) => {
        if (s >= n) {
          setReplaying(false);
          return n;
        }
        return s + 1;
      });
    }, 650);
    return () => clearInterval(id);
  }, [replaying, n]);
  const visible = (v: Version) => sorted.indexOf(v) >= n - shown;

  const rowOf = new Map<string, number>();
  sorted.forEach((v, i) => rowOf.set(v.id, i + 1));
  rowOf.set('HEAD', 0);
  const x = (lane: number) => X0 + lane * LANE;
  const y = (row: number) => row * ROW + ROW / 2;
  const edges: { key: string; d: string; lane: number; dashed: boolean; merge: boolean; show: boolean }[] = [];
  const link = (x1: number, y1: number, x2: number, y2: number) =>
    x1 === x2 ? `M${x1} ${y1} L${x2} ${y2}` : `M${x1} ${y1} L${x1} ${y2 - ROW / 2} Q${x1} ${y2} ${x2 + (x1 > x2 ? 10 : -10)} ${y2} L${x2} ${y2}`;
  // a merge edge leaves the merged tip upward and joins the merge node
  const mergeLink = (xm: number, ym: number, xt: number, yt: number) => `M${xt} ${yt} L${xt} ${ym + ROW / 2} Q${xt} ${ym} ${xm + (xt > xm ? 10 : -10)} ${ym} L${xm} ${ym}`;
  for (const v of sorted) {
    if (v.parent && rowOf.has(v.parent)) {
      const pv = p.versions.find((q) => q.id === v.parent)!;
      edges.push({ key: `${v.id}-p`, d: link(x(laneOf(v.branch)), y(rowOf.get(v.id)!), x(laneOf(pv.branch)), y(rowOf.get(pv.id)!)), lane: laneOf(v.branch), dashed: false, merge: false, show: visible(v) });
    }
    if (v.merged && rowOf.has(v.merged)) {
      const mv = p.versions.find((q) => q.id === v.merged)!;
      edges.push({ key: `${v.id}-m`, d: mergeLink(x(laneOf(v.branch)), y(rowOf.get(v.id)!), x(laneOf(mv.branch)), y(rowOf.get(mv.id)!)), lane: laneOf(mv.branch), dashed: false, merge: true, show: visible(v) });
    }
  }
  if (p.base && rowOf.has(p.base)) {
    const bv = p.versions.find((q) => q.id === p.base)!;
    edges.push({ key: 'head', d: link(x(laneOf(p.branch)), y(0), x(laneOf(bv.branch)), y(rowOf.get(bv.id)!)), lane: laneOf(p.branch), dashed: true, merge: false, show: shown >= n });
  }
  const W = X0 + Math.max(1, lanes.length) * LANE;
  const H = (n + 1) * ROW;
  const ev = (s: Summary) => s.evaluation;
  return (
    <div ref={wrapRef}>
      <div className="row vg-legend small muted">
        {lanes.map((b, i) => (
          <span key={b} className="row" style={{ gap: 4 }}>
            <svg width="12" height="12" aria-hidden><circle cx="6" cy="6" r="5" fill={laneColor(i)} /></svg>
            {b}
          </span>
        ))}
        <span className="row" style={{ gap: 4 }}><span aria-hidden>⚑</span> baseline</span>
        <span style={{ flex: 1 }} />
        {!reduced && n > 1 && (
          <button className="btn small" onClick={() => { setShown(0); setReplaying(true); }} disabled={replaying}>
            {replaying ? 'Replaying…' : '▶ Replay history'}
          </button>
        )}
      </div>
      <div className="vg" style={{ minHeight: H }}>
        <svg className="vg-svg" width={W} height={H} aria-hidden>
          {edges.map((e) => (
            <path
              key={e.key}
              d={e.d}
              fill="none"
              stroke={laneColor(e.lane)}
              strokeWidth={e.merge ? 2 : 2.2}
              strokeDasharray={e.dashed ? '4 4' : e.merge ? '7 3' : undefined}
              className={`${e.dashed ? 'flow-line' : ''} vg-edge ${e.show ? '' : 'vg-hidden'}`}
            />
          ))}
          <circle cx={x(laneOf(p.branch))} cy={y(0)} r={8} fill="var(--surface-1)" stroke={laneColor(laneOf(p.branch))} strokeWidth={2.5} strokeDasharray="3 2" className={`${p.modified ? 'vg-pulse' : ''} ${shown >= n ? '' : 'vg-hidden'}`} />
          {sorted.map((v) => {
            const row = rowOf.get(v.id)!;
            const lane = laneOf(v.branch);
            const sel = p.selected === v.id || p.compare.includes(v.id);
            const fail = ev(v.summary) ? ev(v.summary)!.fail > 0 : false;
            return (
              <g key={v.id} className={`vg-node ${visible(v) ? '' : 'vg-hidden'}`} style={{ transformOrigin: `${x(lane)}px ${y(row)}px` }}>
                {sel && <circle cx={x(lane)} cy={y(row)} r={12} fill="none" stroke="var(--accent)" strokeWidth={2} />}
                {v.kind === 'merge' ? (
                  <rect x={x(lane) - 7} y={y(row) - 7} width={14} height={14} rx={2} transform={`rotate(45 ${x(lane)} ${y(row)})`} fill={laneColor(lane)} stroke="var(--surface-1)" strokeWidth={2} />
                ) : (
                  <circle cx={x(lane)} cy={y(row)} r={7.5} fill={laneColor(lane)} stroke="var(--surface-1)" strokeWidth={2} />
                )}
                {v.baseline && <path d={`M${x(lane) - 3} ${y(row) - 9} L${x(lane) - 3} ${y(row) - 22} L${x(lane) + 8} ${y(row) - 18} L${x(lane) - 3} ${y(row) - 14}`} fill="var(--warn)" stroke="var(--warn)" strokeWidth={1.4} strokeLinejoin="round" />}
                {fail && <circle cx={x(lane) + 6} cy={y(row) + 6} r={3.5} fill="var(--bad)" />}
              </g>
            );
          })}
        </svg>
        <div className="vg-rows" style={{ marginLeft: W + 8 }}>
          <button className={`vg-row ${p.selected === 'HEAD' ? 'sel' : ''} ${shown >= n ? '' : 'vg-hidden'}`} style={{ height: ROW }} onClick={() => p.onSelect('HEAD')}>
            <div className="vg-title">Working state <span className="chip info" style={{ padding: '0 7px' }}>{p.branch}</span> {p.modified ? <span className="chip warn" style={{ padding: '0 7px' }}>● modified</span> : <span className="chip" style={{ padding: '0 7px' }}>= base</span>}</div>
            <div className="vg-meta">{p.working ? <>#{p.working.short} · {p.working.requirements} requirements · {p.working.components} BOM lines</> : '…'}</div>
          </button>
          {sorted.map((v) => {
            const e = ev(v.summary);
            return (
              <button key={v.id} className={`vg-row ${p.selected === v.id ? 'sel' : ''} ${visible(v) ? '' : 'vg-hidden'}`} style={{ height: ROW }} onClick={() => p.onSelect(v.id)}>
                <div className="vg-title">
                  {v.name}
                  {v.baseline && <span className="chip warn" style={{ padding: '0 7px' }}>⚑ {v.baseline}</span>}
                  {v.kind !== 'manual' && !narrow && <span className="chip" style={{ padding: '0 7px' }}>{v.kind}</span>}
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
    </div>
  );
}
