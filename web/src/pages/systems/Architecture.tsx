// Architecture: block diagram of subsystems grouped by domain with animated
// interface connectors, an N² interface matrix, and editors for both.
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, Explainer } from '../../components/ui';
import { Icon } from '../../components/Icon';
import type { Domain, Interface, Project, Subsystem } from '../../engine/types';
import { LABS } from '../../labs/registry';
import { Badge, DOMAINS, DText, Eyebrow, IF_KINDS, ListEditor, REFS, cap, fmtNum, labById, nextId, useStudio } from './shared';
import type { N2 } from './types';

const KIND_COLOR: Record<string, string> = {
  POWER: 'var(--series-2)', I2C: 'var(--series-1)', SPI: 'var(--series-3)', PDM: 'var(--series-5)', BLE: 'var(--series-7)',
  MECH: 'var(--series-4)', THERMAL: 'var(--series-8)', PWM: 'var(--series-6)', GPIO: 'var(--series-6)', SW: 'var(--series-7)', USB: 'var(--series-1)',
};
export const kindColor = (k: string) => KIND_COLOR[k.toUpperCase()] ?? 'var(--muted)';

const BW = 184, BH = 58, GAP_Y = 14, PAD = 14, HEAD = 22, COL_GAP = 130;

interface Placed { s: Subsystem; x: number; y: number; col: number }
interface GroupBox { domain: Domain; x: number; y: number; w: number; h: number }

function layout(p: Project) {
  const groups = DOMAINS.map((d) => ({ d, subs: p.subsystems.filter((s) => s.domain === d) })).filter((g) => g.subs.length > 0);
  const degree = (g: (typeof groups)[number]) => p.interfaces.filter((i) => g.subs.some((s) => s.id === i.from || s.id === i.to)).length;
  const ncol = groups.length >= 3 ? 3 : Math.max(1, groups.length);
  const cols: (typeof groups)[] = Array.from({ length: ncol }, () => []);
  const heights = new Array(ncol).fill(0);
  const sorted = [...groups].sort((a, b) => degree(b) - degree(a));
  const center = Math.floor(ncol / 2);
  sorted.forEach((g, i) => {
    let c: number;
    if (i === 0) c = center;
    else {
      // pick the column with the least stacked height, preferring the sides
      c = 0;
      let best = Infinity;
      for (let k = 0; k < ncol; k++) {
        const score = heights[k] + (k === center ? 1.5 : 0);
        if (score < best) { best = score; c = k; }
      }
    }
    cols[c].push(g);
    heights[c] += g.subs.length + 0.6;
  });
  const placed: Placed[] = [];
  const boxes: GroupBox[] = [];
  let H = 0;
  cols.forEach((col, ci) => {
    const x = PAD + ci * (BW + 2 * PAD + COL_GAP);
    let y = PAD;
    col.forEach((g) => {
      const h = HEAD + g.subs.length * (BH + GAP_Y) - GAP_Y + PAD * 2;
      boxes.push({ domain: g.d, x, y, w: BW + 2 * PAD, h });
      g.subs.forEach((s, k) => placed.push({ s, x: x + PAD, y: y + HEAD + PAD + k * (BH + GAP_Y), col: ci }));
      y += h + 18;
    });
    H = Math.max(H, y);
  });
  const W = PAD * 2 + ncol * (BW + 2 * PAD) + (ncol - 1) * COL_GAP;
  return { placed, boxes, W: Math.max(W, 420), H: Math.max(H, 120) };
}

function edgePath(a: Placed, b: Placed, k: number, n: number) {
  const off = (k - (n - 1) / 2) * 9;
  if (a.col !== b.col) {
    const lr = a.col < b.col;
    const x1 = lr ? a.x + BW : a.x, x2 = lr ? b.x : b.x + BW;
    const y1 = a.y + BH / 2 + off, y2 = b.y + BH / 2 + off;
    const dx = Math.max(50, Math.abs(x2 - x1) * 0.45) * (lr ? 1 : -1);
    return { d: `M${x1},${y1} C${x1 + dx},${y1} ${x2 - dx},${y2} ${x2},${y2}`, mid: bez([x1, y1], [x1 + dx, y1], [x2 - dx, y2], [x2, y2], Math.abs(a.col - b.col) > 1 ? 0.13 : 0.5) };
  }
  // same column: loop out to the right
  const x = a.x + BW, y1 = a.y + BH / 2 + off, y2 = b.y + BH / 2 + off;
  const bulge = 40 + Math.abs(y2 - y1) * 0.18;
  return { d: `M${x},${y1} C${x + bulge},${y1} ${x + bulge},${y2} ${x},${y2}`, mid: bez([x, y1], [x + bulge, y1], [x + bulge, y2], [x, y2], 0.5) };
}

function bez(p0: number[], p1: number[], p2: number[], p3: number[], t: number): [number, number] {
  const u = 1 - t;
  return [0, 1].map((i) => u * u * u * p0[i] + 3 * u * u * t * p1[i] + 3 * u * t * t * p2[i] + t * t * t * p3[i]) as [number, number];
}

export function ArchitectureTab() {
  const { project, studio } = useStudio();
  const [view, setView] = useState<'diagram' | 'n2'>('diagram');
  const [sel, setSel] = useState<string | null>(null);
  return (
    <>
      <Card
        title="System architecture"
        subtitle={<>{project.subsystems.length} subsystems in {new Set(project.subsystems.map((s) => s.domain)).size} domains, {project.interfaces.length} interfaces. Click a block for its functions, requirements, parts and lab.</>}
        actions={
          <div className="ss-seg" role="group" aria-label="View">
            <button className={view === 'diagram' ? 'on' : ''} onClick={() => setView('diagram')}>Block diagram</button>
            <button className={view === 'n2' ? 'on' : ''} onClick={() => setView('n2')}>N² matrix</button>
          </div>
        }
      >
        {project.subsystems.length === 0 ? <div className="ss-empty">No subsystems yet — add one below.</div> : view === 'diagram' ? <BlockDiagram sel={sel} setSel={setSel} /> : studio && <N2View n2={studio.n2} />}
      </Card>
      <div className="grid grid-2">
        <SubsystemPanel sel={sel} setSel={setSel} />
        <InterfaceEditor sel={sel} />
      </div>
      <div className="grid grid-2">
        <Explainer title="Functional and physical architecture" refs={[REFS.nasa, REFS.incose]}>
          Architecture design allocates the <i>functions</i> the product must perform to physical <i>subsystems</i>, and defines the <i>interfaces</i> between them
          (electrical buses, power rails, mechanical mounts, heat paths). Interfaces are where integration problems hide, so each one is written down with its type and
          owner. Requirements are then <i>allocated</i> to the subsystems responsible for meeting them.
        </Explainer>
        <Explainer title="Reading an N² chart" refs={[REFS.n2, REFS.dsm]}>
          Subsystems sit on the diagonal; an interface from <i>i</i> to <i>j</i> sits in row <i>i</i>, column <i>j</i>. Marks above the diagonal flow forward; marks below the
          diagonal are feedback loops that force iteration between teams. Read as a design structure matrix (DSM), the rows can be re-sequenced so that producers come before consumers,
          minimising feedback marks — the “Sequenced” view does this with a greedy ordering plus local swaps.
        </Explainer>
      </div>
    </>
  );
}

function BlockDiagram({ sel, setSel }: { sel: string | null; setSel: (s: string | null) => void }) {
  const { project, ev } = useStudio();
  const L = useMemo(() => layout(project), [project]);
  const [hoverIf, setHoverIf] = useState<string | null>(null);
  const pos = new Map(L.placed.map((p) => [p.s.id, p]));
  const edges = useMemo(() => {
    const groups = new Map<string, Interface[]>();
    for (const i of project.interfaces) {
      const k = [i.from, i.to].sort().join('|');
      groups.set(k, [...(groups.get(k) ?? []), i]);
    }
    const out: { i: Interface; d: string; mid: [number, number] }[] = [];
    groups.forEach((list) => list.forEach((i, k) => {
      const a = pos.get(i.from), b = pos.get(i.to);
      if (!a || !b || a === b) return;
      const e = edgePath(a, b, k, list.length);
      out.push({ i, ...e });
    }));
    return out;
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [project.interfaces, L]);
  const connected = (id: string) => project.interfaces.some((i) => (i.from === sel && i.to === id) || (i.to === sel && i.from === id));
  const statusOf = (sid: string) => {
    const reqs = project.requirements.map((r, k) => ({ r, res: ev?.results[k] })).filter(({ r }) => r.allocated_to.includes(sid));
    const f = reqs.filter((x) => x.res?.status === 'fail' && x.r.kind !== 'goal').length;
    const g = reqs.filter((x) => x.res?.status === 'fail' && x.r.kind === 'goal').length; // open stretch goals
    const p = reqs.filter((x) => x.res?.status === 'pass').length;
    return { n: reqs.length, p, f, g };
  };
  const kinds = [...new Set(project.interfaces.map((i) => i.kind.toUpperCase()))];
  return (
    <>
      <div className="ss-canvas">
        <svg width={L.W} height={L.H} viewBox={`0 0 ${L.W} ${L.H}`} style={{ minWidth: L.W }} role="img" aria-label="Architecture block diagram">
          <defs>
            {kinds.map((k) => (
              <marker key={k} id={`arr-${k}`} viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
                <path d="M0,0 L10,5 L0,10 z" fill={kindColor(k)} />
              </marker>
            ))}
          </defs>
          {L.boxes.map((b) => (
            <g key={b.domain}>
              <rect x={b.x} y={b.y} width={b.w} height={b.h} rx={14} fill="color-mix(in srgb, var(--surface-1) 45%, transparent)" stroke="var(--border-strong)" strokeDasharray="5 4" />
              <text x={b.x + 12} y={b.y + 17} fontSize={10.5} fill="var(--muted)" letterSpacing="0.1em" fontWeight={700}>{b.domain.toUpperCase()}</text>
            </g>
          ))}
          {edges.map(({ i, d }) => {
            const active = !sel || i.from === sel || i.to === sel;
            const hl = hoverIf === i.id;
            const c = kindColor(i.kind);
            return (
              <g key={i.id} className={`ss-edge ${active ? '' : 'dim'}`} onMouseEnter={() => setHoverIf(i.id)} onMouseLeave={() => setHoverIf(null)}>
                <path d={d} fill="none" stroke="transparent" strokeWidth={12} />
                <path d={d} fill="none" stroke={c} strokeWidth={hl ? 3 : 1.8} opacity={0.55} markerEnd={`url(#arr-${i.kind.toUpperCase()})`} />
                <path d={d} fill="none" stroke={c} strokeWidth={hl ? 3 : 2} className="ss-flow" strokeLinecap="round" />
                <title>{`${i.id} ${i.kind}: ${i.from} → ${i.to}. ${i.description}`}</title>
              </g>
            );
          })}
          {L.placed.map((p) => {
            const st = statusOf(p.s.id);
            const dim = sel && sel !== p.s.id && !connected(p.s.id);
            const lab = labById(p.s.lab);
            return (
              <g key={p.s.id} className={`ss-block ${sel === p.s.id ? 'sel' : ''} ${dim ? 'dim' : ''}`} transform={`translate(${p.x},${p.y})`}
                onClick={() => setSel(sel === p.s.id ? null : p.s.id)} role="button" tabIndex={0} aria-label={`${p.s.name}, ${st.p} of ${st.n} requirements pass`}
                onKeyDown={(e) => { if (e.key === 'Enter') setSel(sel === p.s.id ? null : p.s.id); }}>
                <rect className="body" width={BW} height={BH} rx={10} />
                <rect width={4} height={BH - 16} x={0} y={8} rx={2} fill={st.f > 0 ? 'var(--bad)' : st.g > 0 ? 'var(--warn)' : st.n > 0 && st.p === st.n ? 'var(--good)' : 'var(--muted)'} />
                <text x={14} y={21} fontSize={13} fontWeight={650} fill="var(--text)">{p.s.name.length > 21 ? p.s.name.slice(0, 20) + '…' : p.s.name}</text>
                <text x={14} y={38} fontSize={10.5} fill="var(--muted)" fontFamily="var(--mono)">{p.s.id}</text>
                <text x={14} y={51} fontSize={10.5} fill="var(--text-2)">{lab ? lab.title : 'no lab'}</text>
                <text x={BW - 10} y={38} fontSize={11} textAnchor="end" fill={st.f > 0 ? 'var(--bad-text)' : st.g > 0 ? 'var(--warn-text)' : 'var(--text-2)'} fontWeight={700}>
                  {st.n ? `${st.f > 0 ? '✕' : st.g > 0 ? '!' : st.p === st.n ? '✓' : '○'} ${st.p}/${st.n}` : '—'}
                </text>
              </g>
            );
          })}
          {edges.map(({ i, mid }) => {
            const active = !sel || i.from === sel || i.to === sel;
            const w = i.kind.length * 6.6 + 12;
            return (
              <g key={`l${i.id}`} className={`ss-edge-label ${active ? '' : 'ss-edge dim'}`} transform={`translate(${mid[0] - w / 2},${mid[1] - 8})`} onMouseEnter={() => setHoverIf(i.id)} onMouseLeave={() => setHoverIf(null)}>
                <rect width={w} height={16} rx={8} style={{ stroke: kindColor(i.kind) }} />
                <text x={w / 2} y={11.5} textAnchor="middle" fill="var(--text)">{i.kind}</text>
                <title>{`${i.id}: ${i.description}`}</title>
              </g>
            );
          })}
        </svg>
      </div>
      <div className="ss-legend">
        {kinds.map((k) => <span className="i" key={k}><span className="ss-sw" style={{ background: kindColor(k), height: 3, width: 16 }} />{k}</span>)}
        <span className="i muted">▮ left bar: ✓ all allocated requirements pass · ✕ one fails</span>
        {hoverIf && <span className="i" style={{ color: 'var(--text)' }}>{(() => { const i = project.interfaces.find((x) => x.id === hoverIf); return i ? `${i.id} ${i.from} → ${i.to}: ${i.description}` : ''; })()}</span>}
      </div>
    </>
  );
}

function N2View({ n2 }: { n2: N2 }) {
  const [seq, setSeq] = useState(false);
  const order = seq ? n2.suggested_order : n2.ids.map((_, i) => i);
  const cell = (r: number, c: number) => n2.cells.find((x) => x.row === r && x.col === c);
  return (
    <>
      <div className="ss-toolbar">
        <div className="ss-seg" role="group" aria-label="Order">
          <button className={!seq ? 'on' : ''} onClick={() => setSeq(false)}>As listed · {n2.feedback} feedback</button>
          <button className={seq ? 'on' : ''} onClick={() => setSeq(true)}>Sequenced (DSM) · {n2.suggested_feedback} feedback</button>
        </div>
        <span className="ss-legend" style={{ marginTop: 0 }}>
          <span className="i"><span className="ss-sw" style={{ background: 'color-mix(in srgb, var(--series-3) 45%, transparent)' }} />↘ forward (above diagonal)</span>
          <span className="i"><span className="ss-sw" style={{ background: 'color-mix(in srgb, var(--series-2) 45%, transparent)' }} />↖ feedback (below diagonal)</span>
        </span>
        {n2.dangling.length > 0 && <Badge status="fail" label={`dangling: ${n2.dangling.join(', ')}`} />}
      </div>
      <div className="table-wrap">
        <table className="ss-n2" aria-label="N-squared interface matrix">
          <tbody>
            {order.map((r, ri) => (
              <tr key={r}>
                {order.map((c, ci) => {
                  if (r === c) return <td key={c} className="diag" title={`${n2.ids[r]} · out ${n2.fan_out[r]} / in ${n2.fan_in[r]}`}>{n2.names[r]}</td>;
                  const x = cell(r, c);
                  if (!x) return <td key={c} className="off" />;
                  const fwd = ci > ri;
                  return (
                    <td key={c} className={fwd ? 'fwd' : 'back'} title={x.links.map((l) => `${l.id} ${l.kind}: ${n2.names[r]} → ${n2.names[c]}. ${l.description}`).join('\n')}>
                      {x.links.map((l) => <span className="k" key={l.id}>{fwd ? '' : '↖ '}{l.kind}</span>)}
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}

function SubsystemPanel({ sel, setSel }: { sel: string | null; setSel: (s: string | null) => void }) {
  const { project, ev, studio, update, openReq } = useStudio();
  const idx = project.subsystems.findIndex((s) => s.id === sel);
  const s = idx >= 0 ? project.subsystems[idx] : null;
  const add = () => {
    const id = nextId('SYS-', project.subsystems.map((x) => x.id), 2);
    update((d) => { d.subsystems.push({ id, name: 'New subsystem', domain: 'electrical', description: '', functions: [], lab: '' }); });
    setSel(id);
  };
  if (!s) {
    return (
      <Card title="Subsystems" subtitle="Select a block, or pick one here" actions={<button className="btn small" onClick={add}>+ Subsystem</button>}>
        <div className="stack" style={{ gap: 6 }}>
          {project.subsystems.map((x) => {
            const r = studio?.budgets.subsystems.find((b) => b.id === x.id);
            return (
              <button key={x.id} className="ss-sub" onClick={() => setSel(x.id)} style={{ display: 'flex', justifyContent: 'space-between', gap: 10 }}>
                <span><b>{x.name}</b> <span className="muted small">{x.id} · {x.domain}</span></span>
                <span className="muted small tnum">{r ? `${r.components} parts · ${fmtNum(r.mass_g)} g` : ''}</span>
              </button>
            );
          })}
        </div>
      </Card>
    );
  }
  const reqs = project.requirements.map((r, k) => ({ r, res: ev?.results[k] })).filter(({ r }) => r.allocated_to.includes(s.id));
  const parts = project.components.filter((c) => c.subsystem === s.id);
  const lab = labById(s.lab);
  const roll = studio?.budgets.subsystems.find((b) => b.id === s.id);
  const setS = (fn: (x: Subsystem) => void) => update((d) => { fn(d.subsystems[idx]); });
  return (
    <Card
      title={<span className="row" style={{ gap: 8 }}><span>{s.name}</span><span className="ss-id muted">{s.id}</span></span>}
      subtitle={s.description || 'Subsystem'}
      actions={<><button className="btn small" onClick={() => setSel(null)}>All</button><button className="btn small" onClick={add}>+ Subsystem</button></>}
    >
      <div className="ss-form">
        <div className="two">
          <label className="f">Name<DText value={s.name} onCommit={(v) => setS((x) => { x.name = v; })} /></label>
          <label className="f">Domain
            <select value={s.domain} onChange={(e) => setS((x) => { x.domain = e.target.value as Domain; })}>
              {DOMAINS.map((d) => <option key={d} value={d}>{cap(d)}</option>)}
            </select>
          </label>
        </div>
        <div className="two">
          <label className="f">Designed in lab
            <select value={s.lab} onChange={(e) => setS((x) => { x.lab = e.target.value; })}>
              <option value="">— none —</option>
              {LABS.map((l) => <option key={l.id} value={l.id}>{l.title}</option>)}
            </select>
          </label>
          <div className="f" style={{ display: 'grid', alignContent: 'end' }}>
            {lab && <Link className="btn small" to={lab.path}><Icon name={lab.icon} size={14} /> Open {lab.title}</Link>}
          </div>
        </div>
        <label className="f">Description<DText area rows={2} value={s.description} onCommit={(v) => setS((x) => { x.description = v; })} /></label>
        <div>
          <Eyebrow>Functions</Eyebrow>
          <ListEditor items={s.functions} placeholder="function" addLabel="Function" onChange={(v) => setS((x) => { x.functions = v; })} />
        </div>
        <div>
          <Eyebrow>Allocated requirements ({reqs.length})</Eyebrow>
          {reqs.length === 0 && <div className="muted small">None — allocate requirements in the Requirements tab.</div>}
          <div className="stack" style={{ gap: 4 }}>
            {reqs.map(({ r, res }) => (
              <div key={r.id} className="row" style={{ flexWrap: 'nowrap', justifyContent: 'space-between', gap: 8 }}>
                <span className="small" style={{ minWidth: 0 }}><button className="ss-link ss-id" onClick={() => openReq(r.id)}>{r.id}</button> {r.text}</span>
                <Badge status={res?.status ?? 'unverified'} />
              </div>
            ))}
          </div>
        </div>
        <div>
          <Eyebrow>Parts (bill of materials) {roll && `· ${fmtNum(roll.mass_g)} g · ${fmtNum(roll.cost_usd)} USD · ${fmtNum(roll.power_mw)} mW`}</Eyebrow>
          {parts.length === 0 ? <div className="muted small">No parts assigned.</div> : (
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Ref</th><th>Part</th><th>Qty</th><th>USD</th><th>g</th><th>avg mW</th></tr></thead>
                <tbody>
                  {parts.map((c) => (
                    <tr key={c.refdes}>
                      <td className="ss-id">{c.refdes}</td><td className="small">{c.name}<div className="muted">{c.part_class}</div></td>
                      <td className="tnum">{c.qty}</td><td className="tnum">{fmtNum(c.unit_cost_usd * c.qty)}</td><td className="tnum">{fmtNum(c.mass_g * c.qty)}</td>
                      <td className="tnum">{fmtNum(c.qty * ((c.power?.active_mw ?? 0) * (c.power?.duty ?? 0) + (c.power?.sleep_mw ?? 0) * (1 - (c.power?.duty ?? 0))))}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
        <div className="row" style={{ justifyContent: 'flex-end' }}>
          <button className="btn small" style={{ color: 'var(--bad-text)' }} onClick={() => {
            if (!confirm(`Delete ${s.name}? Interfaces to it are removed; requirement allocations and parts keep a dangling id until you fix them.`)) return;
            update((d) => { d.subsystems.splice(idx, 1); d.interfaces = d.interfaces.filter((i) => i.from !== s.id && i.to !== s.id); });
            setSel(null);
          }}>Delete subsystem</button>
        </div>
      </div>
    </Card>
  );
}

function InterfaceEditor({ sel }: { sel: string | null }) {
  const { project, update } = useStudio();
  const list = project.interfaces.map((i, k) => ({ i, k })).filter(({ i }) => !sel || i.from === sel || i.to === sel);
  const subs = project.subsystems;
  return (
    <Card
      title={sel ? `Interfaces of ${subs.find((s) => s.id === sel)?.name ?? sel}` : 'Interfaces'}
      subtitle="Producer → consumer, with the bus, rail or mechanical joint that connects them"
      actions={
        <button className="btn small" disabled={subs.length < 2} onClick={() => update((d) => {
          const from = sel ?? d.subsystems[0]?.id ?? '';
          const to = d.subsystems.find((s) => s.id !== from)?.id ?? '';
          d.interfaces.push({ id: nextId('IF-', d.interfaces.map((x) => x.id), 2), from, to, kind: 'SPI', description: '' });
        })}>+ Interface</button>
      }
    >
      {list.length === 0 && <div className="ss-empty">No interfaces{sel ? ' on this subsystem' : ''}.</div>}
      <datalist id="ss-if-kinds">{IF_KINDS.map((k) => <option key={k} value={k} />)}</datalist>
      <div className="stack" style={{ gap: 8 }}>
        {list.map(({ i, k }) => (
          <div key={k} className="stat" style={{ padding: 10, display: 'grid', gap: 6 }}>
            <div className="row" style={{ flexWrap: 'wrap', gap: 6 }}>
              <span className="ss-id" style={{ minWidth: 42 }}>{i.id}</span>
              <select style={{ width: 'auto', flex: 1, minWidth: 110 }} value={i.from} aria-label="From" onChange={(e) => update((d) => { d.interfaces[k].from = e.target.value; })}>
                {!subs.some((s) => s.id === i.from) && <option value={i.from}>{i.from} (missing)</option>}
                {subs.map((s) => <option key={s.id} value={s.id}>{s.name}</option>)}
              </select>
              <span className="muted">→</span>
              <select style={{ width: 'auto', flex: 1, minWidth: 110 }} value={i.to} aria-label="To" onChange={(e) => update((d) => { d.interfaces[k].to = e.target.value; })}>
                {!subs.some((s) => s.id === i.to) && <option value={i.to}>{i.to} (missing)</option>}
                {subs.map((s) => <option key={s.id} value={s.id}>{s.name}</option>)}
              </select>
              <DText list="ss-if-kinds" upper value={i.kind} ariaLabel="Kind" style={{ width: 104, borderColor: kindColor(i.kind) }} onCommit={(v) => update((d) => { d.interfaces[k].kind = v; })} />
              <button className="btn ghost small ss-x" aria-label={`Delete ${i.id}`} onClick={() => update((d) => { d.interfaces.splice(k, 1); })}>✕</button>
            </div>
            <DText value={i.description} placeholder="What crosses this interface (signals, rate, voltage, fixings)" onCommit={(v) => update((d) => { d.interfaces[k].description = v; })} />
          </div>
        ))}
      </div>
    </Card>
  );
}
