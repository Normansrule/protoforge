// Traceability: layered need → requirement → subsystem → metric → lab graph
// with impact highlighting (systems.impact), and a requirements × subsystems
// allocation matrix with coverage gaps.
import { useMemo, useState } from 'react';
import { Card, Explainer } from '../../components/ui';
import { call } from '../../engine/engine';
import { Badge, REFS, labById, useStudio } from './shared';
import type { TraceGraph, TraceNode } from './types';

const COLS = ['Needs', 'Requirements', 'Subsystems', 'Metrics', 'Labs'];
const KIND_COLOR: Record<TraceNode['kind'], string> = {
  need: 'var(--series-7)', requirement: 'var(--series-1)', subsystem: 'var(--series-3)', metric: 'var(--series-4)', lab: 'var(--series-5)',
};
const COL_W = 196, ROW_H = 24, TOP = 34, NODE_W = 150;

function useLayered(g: TraceGraph | undefined) {
  return useMemo(() => {
    if (!g) return null;
    const layers: TraceNode[][] = [[], [], [], [], []];
    g.nodes.forEach((n) => layers[Math.min(4, n.layer)].push(n));
    const pos = new Map<string, number>();
    const setPos = (L: TraceNode[]) => L.forEach((n, i) => pos.set(n.id, i));
    setPos(layers[0]);
    const neighbours = (id: string) => g.edges.filter((e) => e.from === id || e.to === id).map((e) => (e.from === id ? e.to : e.from));
    const bary = (L: TraceNode[], refLayers: number[]) => {
      const ids = new Set(refLayers.flatMap((l) => layers[l].map((n) => n.id)));
      const score = (n: TraceNode) => {
        const ps = neighbours(n.id).filter((x) => ids.has(x)).map((x) => pos.get(x) ?? 0);
        return ps.length ? ps.reduce((a, b) => a + b, 0) / ps.length : 1e6;
      };
      const s = new Map(L.map((n) => [n.id, score(n)]));
      L.sort((a, b) => s.get(a.id)! - s.get(b.id)!);
      setPos(L);
    };
    bary(layers[1], [0]);
    bary(layers[2], [1]);
    bary(layers[3], [1]);
    bary(layers[4], [2, 3]);
    // second sweep upward for needs
    bary(layers[0], [1]);
    bary(layers[1], [0, 2]);
    const H = TOP + Math.max(...layers.map((l) => l.length), 1) * ROW_H + 10;
    const xy = new Map<string, [number, number]>();
    layers.forEach((L, li) => {
      const offset = ((H - TOP) - L.length * ROW_H) / 2;
      L.forEach((n, i) => xy.set(n.id, [10 + li * COL_W, TOP + offset + i * ROW_H]));
    });
    return { layers, xy, H, W: 20 + 4 * COL_W + NODE_W };
  }, [g]);
}

export function TraceabilityTab() {
  const [view, setView] = useState<'graph' | 'matrix'>('graph');
  const { studio } = useStudio();
  const cov = studio?.coverage;
  return (
    <>
      <Card
        title="Traceability"
        subtitle="Every requirement should come from a need, belong to a subsystem and be verified by a metric that a lab computes. Hover or click any node to see what it affects."
        actions={
          <div className="ss-seg" role="group" aria-label="View">
            <button className={view === 'graph' ? 'on' : ''} onClick={() => setView('graph')}>Graph</button>
            <button className={view === 'matrix' ? 'on' : ''} onClick={() => setView('matrix')}>Matrix</button>
          </div>
        }
      >
        {cov && (
          <div className="grid grid-4" style={{ marginBottom: 14 }}>
            <div className="stat"><div className="stat-label">Needs covered</div><div className="stat-value tnum">{cov.needs_covered}<span className="stat-unit">/ {cov.needs_total}</span></div></div>
            <div className="stat"><div className="stat-label">Traced to needs</div><div className="stat-value tnum">{cov.reqs_traced}<span className="stat-unit">/ {cov.reqs_total}</span></div></div>
            <div className="stat"><div className="stat-label">Allocated</div><div className="stat-value tnum">{cov.reqs_allocated}<span className="stat-unit">/ {cov.reqs_total}</span></div></div>
            <div className="stat"><div className="stat-label">Completeness</div><div className="stat-value tnum">{Math.round(cov.completeness * 100)}<span className="stat-unit">%</span></div><div className="stat-note">mean of the four coverage ratios</div></div>
          </div>
        )}
        {view === 'graph' ? <Graph /> : <Matrix />}
      </Card>
      <div className="grid grid-2">
        <NeedsCoverage />
        <Explainer title="Why trace everything?" refs={[REFS.nasa, REFS.incose, REFS.iso15288]}>
          Bidirectional traceability answers two review questions. <b>Downward</b>: is every need realised by requirements, subsystems and a verification?
          (a gap means a need nobody designs for). <b>Upward</b>: does every requirement and part exist because a need asks for it? (an orphan is gold-plating).
          During change control the same graph gives the <i>impact set</i>: change a need or a subsystem and you can see exactly which requirements, metrics and
          labs must be re-checked — here that re-check is automatic.
        </Explainer>
      </div>
    </>
  );
}

function Graph() {
  const { project, studio, openReq, go } = useStudio();
  const g = studio?.trace;
  const L = useLayered(g);
  const [hover, setHover] = useState<string | null>(null);
  const [locked, setLocked] = useState<string | null>(null);
  const active = hover ?? locked;
  const impactCache = useMemo(() => new Map<string, Set<string>>(), [project]);
  const impactSet = useMemo(() => {
    if (!active) return null;
    let s = impactCache.get(active);
    if (!s) {
      try { s = new Set([active, ...call<string[]>('systems.impact', { project, id: active })]); } catch { s = new Set([active]); }
      impactCache.set(active, s);
    }
    return s;
  }, [active, impactCache, project]);
  if (!g || !L) return <div className="muted small">computing…</div>;
  return (
    <GraphInner L={L} g={g} impactSet={impactSet} active={active} locked={locked}
      onHover={setHover} onClick={(id) => setLocked(locked === id ? null : id)}
      onOpen={(n) => {
        if (n.kind === 'requirement') openReq(n.id);
        else if (n.kind === 'subsystem') go('architecture');
        else if (n.kind === 'lab') { const l = labById(n.id.replace(/^lab:/, '')); if (l) window.location.hash = l.path; }
      }} />
  );
}

function GraphInner(props: {
  L: NonNullable<ReturnType<typeof useLayered>>; g: TraceGraph; impactSet: Set<string> | null; active: string | null; locked: string | null;
  onHover: (id: string | null) => void; onClick: (id: string) => void; onOpen: (n: TraceNode) => void;
}) {
  const { project, ev } = useStudio();
  const { L, g, impactSet } = props;
  const status = new Map(project.requirements.map((r, i) => [r.id, ev?.results[i]?.status ?? 'unverified']));
  const label = (n: TraceNode) => {
    if (n.kind === 'requirement') return `${n.id} ${n.label}`;
    if (n.kind === 'need') return `${n.id} ${n.label}`;
    if (n.kind === 'metric') return n.label;
    if (n.kind === 'lab') return labById(n.label)?.title ?? n.label;
    return n.label;
  };
  const activeNode = props.active ? g.nodes.find((n) => n.id === props.active) : null;
  return (
    <>
      <div className="row muted small" style={{ marginBottom: 8, minHeight: 20 }}>
        {activeNode ? (
          <>
            <b style={{ color: 'var(--text)' }}>{activeNode.kind}</b> {activeNode.id.replace(/^(m|lab):/, '')} — impact set of {(impactSet?.size ?? 1) - 1} nodes
            {props.locked && <button className="btn small ghost" onClick={() => props.onClick(props.locked!)}>Clear</button>}
            {activeNode.kind === 'requirement' && <button className="btn small" onClick={() => props.onOpen(activeNode)}>Open {activeNode.id}</button>}
          </>
        ) : <>Hover a node to highlight everything upstream and downstream of it; click to pin; double-click to open.</>}
      </div>
      <div className="ss-canvas">
        <svg className="ss-trace" width={L.W} height={L.H} style={{ minWidth: L.W }} role="img" aria-label="Traceability graph">
          {COLS.map((c, i) => (
            <text key={c} x={10 + i * COL_W} y={18} fontSize={10.5} fontWeight={700} letterSpacing="0.1em" fill="var(--muted)">{c.toUpperCase()} · {L.layers[i].length}</text>
          ))}
          <g>
            {g.edges.map((e, i) => {
              const a = L.xy.get(e.from), b = L.xy.get(e.to);
              if (!a || !b) return null;
              const x1 = a[0] + NODE_W, y1 = a[1] + 9, x2 = b[0], y2 = b[1] + 9;
              const dx = (x2 - x1) * 0.5;
              const on = impactSet ? impactSet.has(e.from) && impactSet.has(e.to) : false;
              const fromNode = g.nodes.find((n) => n.id === e.from);
              const c = fromNode ? KIND_COLOR[fromNode.kind] : 'var(--muted)';
              return (
                <path key={i} d={`M${x1},${y1} C${x1 + dx},${y1} ${x2 - dx},${y2} ${x2},${y2}`} fill="none" stroke={c}
                  className={`ss-tedge ${impactSet ? (on ? 'hl' : 'dim') : ''}`} strokeWidth={1.1} opacity={0.35} />
              );
            })}
          </g>
          {L.layers.flat().map((n) => {
            const [x, y] = L.xy.get(n.id)!;
            const dim = impactSet && !impactSet.has(n.id);
            const st = n.kind === 'requirement' ? status.get(n.id) : undefined;
            const text = label(n);
            const t = text.length > 23 ? text.slice(0, 22) + '…' : text;
            return (
              <g key={n.id} className={`ss-node ${dim ? 'dim' : ''}`} transform={`translate(${x},${y})`}
                onMouseEnter={() => props.onHover(n.id)} onMouseLeave={() => props.onHover(null)}
                onClick={() => props.onClick(n.id)} onDoubleClick={() => props.onOpen(n)}>
                <title>{`${n.kind}: ${n.id.replace(/^(m|lab):/, '')}\n${n.label}`}</title>
                <rect width={NODE_W} height={18} rx={9} fill="var(--surface-1)" stroke={props.active === n.id ? 'var(--accent)' : 'var(--border-strong)'} strokeWidth={props.active === n.id ? 2 : 1} />
                <circle cx={9} cy={9} r={4} fill={KIND_COLOR[n.kind]} />
                <text x={18} y={12.5} fontSize={10.5} fill="var(--text)" fontFamily={n.kind === 'metric' ? 'var(--mono)' : undefined}>{t}</text>
                {st && (
                  <text x={NODE_W - 8} y={12.5} fontSize={10.5} textAnchor="end" fontWeight={800} fill={st === 'pass' ? 'var(--good-text)' : st === 'fail' ? 'var(--bad-text)' : 'var(--muted)'}>
                    {st === 'pass' ? '✓' : st === 'fail' ? '✕' : '○'}
                  </text>
                )}
              </g>
            );
          })}
        </svg>
      </div>
      <div className="ss-legend">
        {(Object.keys(KIND_COLOR) as TraceNode['kind'][]).map((k) => <span className="i" key={k}><span className="ss-sw" style={{ background: KIND_COLOR[k], borderRadius: '50%' }} />{k}</span>)}
        <span className="i">✓ pass · ✕ fail · ○ unverified</span>
      </div>
    </>
  );
}

function Matrix() {
  const { project, ev, studio, openReq } = useStudio();
  const empty = new Set(studio?.coverage.empty_subsystems ?? []);
  return (
    <>
      <div className="table-wrap">
        <table className="ss-matrix" aria-label="Requirement allocation matrix">
          <thead>
            <tr>
              <th style={{ textAlign: 'left' }}>Req.</th>
              <th style={{ textAlign: 'left' }}>Needs</th>
              {project.subsystems.map((s) => (
                <th key={s.id} className={`rot ${empty.has(s.id) ? 'gap' : ''}`} title={empty.has(s.id) ? `${s.name}: no requirement allocated` : s.name}>
                  <div>{s.name}{empty.has(s.id) ? ' !' : ''}</div>
                </th>
              ))}
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {project.requirements.map((r, i) => {
              const gap = r.allocated_to.length === 0 || r.derives_from.length === 0;
              return (
                <tr key={r.id}>
                  <td style={{ textAlign: 'left' }} className={gap ? 'gap' : ''}><button className="ss-link ss-id" onClick={() => openReq(r.id)} title={r.text}>{r.id}</button></td>
                  <td style={{ textAlign: 'left' }} className={r.derives_from.length === 0 ? 'gap' : ''}>{r.derives_from.join(', ') || '— none'}</td>
                  {project.subsystems.map((s) => (
                    <td key={s.id} className={`${r.allocated_to.includes(s.id) ? 'mark' : ''} ${empty.has(s.id) ? 'gap' : ''}`}>{r.allocated_to.includes(s.id) ? '●' : ''}</td>
                  ))}
                  <td><Badge status={ev?.results[i]?.status ?? 'unverified'} /></td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <div className="ss-legend">
        <span className="i">● allocated</span>
        <span className="i"><span className="ss-sw" style={{ background: 'color-mix(in srgb, var(--warn) 40%, transparent)' }} />! gap: requirement without a need or owner, or subsystem without requirements</span>
      </div>
    </>
  );
}

function NeedsCoverage() {
  const { studio, openReq } = useStudio();
  const needs = studio?.coverage.needs ?? [];
  const map = { satisfied: 'pass', failing: 'fail', partial: 'partial', unverified: 'unverified', uncovered: 'todo' } as const;
  return (
    <Card title="Need coverage" subtitle="Each need, the requirements derived from it and whether they all pass">
      {needs.length === 0 && <div className="ss-empty">No needs.</div>}
      <div className="stack" style={{ gap: 8 }}>
        {needs.map((n) => (
          <div key={n.id} style={{ borderBottom: '1px solid var(--border)', paddingBottom: 6 }}>
            <div className="row" style={{ justifyContent: 'space-between', flexWrap: 'nowrap' }}>
              <span style={{ fontSize: 13 }}><span className="ss-id">{n.id}</span> {n.text}</span>
              <Badge status={map[n.status]} label={n.status} />
            </div>
            <div className="small" style={{ marginTop: 3 }}>
              {n.requirements.length === 0 ? <span style={{ color: 'var(--warn)' }}>! no requirement derives from this need</span> : n.requirements.map((r) => (
                <button key={r} className="ss-link ss-id" style={{ marginRight: 8 }} onClick={() => openReq(r)}>{r}</button>
              ))}
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
}
