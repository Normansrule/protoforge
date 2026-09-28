// V-model: decomposition down the left leg, implementation at the bottom,
// integration and verification up the right leg, with each level's
// verification link — every stage's status derived from the live evaluation.
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Card, Explainer } from '../../components/ui';
import { Badge, Eyebrow, REFS, statusColor, useReducedMotion, useStudio, type TabId } from './shared';
import type { VStage } from './types';

const W = 1040, BW = 202, BH = 74, DY = 104, DX = 70, TOP = 26;

function stageXY(s: VStage): [number, number] {
  if (s.leg === 'bottom') return [W / 2 - BW / 2, TOP + 4 * DY];
  const x = s.leg === 'left' ? 24 + s.level * DX : W - 24 - BW - s.level * DX;
  return [x, TOP + s.level * DY];
}

export function useLinkHandler() {
  const { go, openReq } = useStudio();
  const nav = useNavigate();
  return (link: string | null | undefined) => {
    if (!link) return;
    const [k, v] = [link.slice(0, link.indexOf(':')), link.slice(link.indexOf(':') + 1)];
    if (k === 'tab') go(v as TabId);
    else if (k === 'req') openReq(v);
    else if (k === 'lab') nav(v);
  };
}

export function VModelTab() {
  const { studio } = useStudio();
  const reduced = useReducedMotion();
  const [sel, setSel] = useState<string>('system');
  const follow = useLinkHandler();
  const v = studio?.vmodel;
  if (!v) return <Card title="V-model"><div className="muted small">computing…</div></Card>;
  const byId = new Map(v.stages.map((s) => [s.id, s]));
  const center = (s: VStage) => { const [x, y] = stageXY(s); return [x + BW / 2, y + BH / 2]; };
  const spine = ['conops', 'requirements', 'architecture', 'design', 'implementation', 'unit', 'integration', 'system', 'validation']
    .map((id) => byId.get(id)).filter(Boolean).map((s) => center(s!));
  const spinePath = spine.map(([x, y], i) => `${i ? 'L' : 'M'}${x},${y}`).join('');
  const H = TOP + 4 * DY + BH + 16;
  const selected = byId.get(sel);
  const pairs = v.stages.filter((s) => s.leg === 'left' && s.pairs_with);
  const pairLabel: Record<string, string> = { conops: 'validation', requirements: 'system verification', architecture: 'integration test', design: 'unit verification' };

  return (
    <>
      <Card
        title="The V-model — where this product is in its life cycle"
        subtitle={<>Down the left leg the product is decomposed and designed; at the bottom it is built; up the right leg it is integrated and verified against each level on the left. Overall {Math.round(v.overall * 100)} % — click a stage for its evidence.</>}
      >
        <div className="ss-canvas ss-v" style={{ overflowX: 'auto' }}>
          <svg viewBox={`0 0 ${W} ${H}`} style={{ minWidth: 760 }} role="img" aria-label="V-model diagram">
            <defs>
              <marker id="v-arr" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" fill="var(--accent)" /></marker>
            </defs>
            <path d={spinePath} fill="none" stroke="var(--accent-soft)" strokeWidth={34} strokeLinejoin="round" strokeLinecap="round" />
            <path d={spinePath} fill="none" stroke="var(--accent)" strokeWidth={1.5} strokeDasharray="4 6" className="ss-flow" opacity={0.6} />
            {!reduced && (
              <circle r={6} fill="var(--accent)" stroke="var(--surface-1)" strokeWidth={2}>
                <animateMotion dur="7s" repeatCount="indefinite" path={spinePath} />
              </circle>
            )}
            <text x={40} y={H - 30} fontSize={11} fill="var(--muted)" fontWeight={700} letterSpacing="0.1em">DECOMPOSITION & DEFINITION ↓</text>
            <text x={W - 40} y={H - 30} fontSize={11} fill="var(--muted)" fontWeight={700} letterSpacing="0.1em" textAnchor="end">↑ INTEGRATION & VERIFICATION</text>
            {pairs.map((l) => {
              const r = byId.get(l.pairs_with!);
              if (!r) return null;
              const [lx, ly] = stageXY(l), [rx] = stageXY(r);
              const y = ly + BH / 2;
              return (
                <g key={l.id}>
                  <line x1={rx} x2={lx + BW} y1={y} y2={y} stroke="var(--accent)" strokeDasharray="6 5" strokeWidth={1.3} markerEnd="url(#v-arr)" opacity={0.75} />
                  <rect x={(rx + lx + BW) / 2 - 62} y={y - 9} width={124} height={18} rx={9} fill="var(--surface-1)" stroke="var(--border-strong)" />
                  <text x={(rx + lx + BW) / 2} y={y + 4} textAnchor="middle" fontSize={10} fill="var(--text-2)" fontWeight={600}>{pairLabel[l.id] ?? 'verifies'}</text>
                </g>
              );
            })}
            {v.stages.map((s) => {
              const [x, y] = stageXY(s);
              const c = statusColor(s.status);
              const icon = s.status === 'done' ? '✓' : s.status === 'attention' ? '!' : s.status === 'partial' ? '◐' : '○';
              const word = s.status === 'done' ? 'done' : s.status === 'attention' ? 'attention' : s.status === 'partial' ? 'in progress' : 'not started';
              return (
                <g key={s.id} className={`ss-vstage ${sel === s.id ? 'sel' : ''}`} transform={`translate(${x},${y})`} onClick={() => setSel(s.id)}
                  role="button" tabIndex={0} aria-label={`${s.title}: ${word}, ${Math.round(s.progress * 100)} %`} onKeyDown={(e) => { if (e.key === 'Enter') setSel(s.id); }}>
                  <rect className="body" width={BW} height={BH} rx={12} />
                  <text x={12} y={21} fontSize={12.5} fontWeight={700} fill="var(--text)">{s.title.length > 25 ? s.title.slice(0, 24) + '…' : s.title}</text>
                  <text x={12} y={39} fontSize={11} fill={c === 'var(--muted)' ? 'var(--muted)' : 'var(--text-2)'} fontWeight={700}>
                    <tspan fill={c}>{icon}</tspan> {word} · {Math.round(s.progress * 100)} %
                  </text>
                  <rect x={12} y={48} width={BW - 24} height={4} rx={2} fill="var(--surface-3)" />
                  <rect x={12} y={48} width={(BW - 24) * s.progress} height={4} rx={2} fill={c} style={{ transition: 'width 0.4s' }} />
                  <text x={12} y={66} fontSize={10} fill="var(--muted)">{s.summary.length > 34 ? s.summary.slice(0, 33) + '…' : s.summary}</text>
                </g>
              );
            })}
          </svg>
        </div>
      </Card>

      {selected && (
        <Card
          title={<span className="row" style={{ gap: 8 }}>{selected.title} <Badge status={selected.status} /></span>}
          subtitle={selected.summary}
          actions={<button className="btn small" onClick={() => follow(selected.link)}>Go to {selected.link.startsWith('lab:') ? 'lab' : selected.link.replace('tab:', '')} →</button>}
        >
          <div className="grid grid-2" style={{ gap: 8 }}>
            {selected.items.map((it, i) => (
              <div key={i} className="ss-check" style={{ borderBottom: '1px solid var(--border)', paddingBottom: 6 }}>
                <span className="ic" style={{ background: `color-mix(in srgb, ${statusColor(it.status === 'todo' ? 'unverified' : it.status)} 22%, transparent)`, color: 'var(--text)' }} aria-hidden>
                  {it.status === 'pass' ? '✓' : it.status === 'fail' ? '✕' : it.status === 'info' ? 'i' : '○'}
                </span>
                <div style={{ minWidth: 0 }}>
                  {it.link ? <button className="ss-link" style={{ fontWeight: 600 }} onClick={() => follow(it.link)}>{it.label}</button> : <b>{it.label}</b>}
                  <span className="muted small"> — {it.status === 'pass' ? 'pass' : it.status === 'fail' ? 'fail' : it.status === 'info' ? 'info' : 'to do'}</span>
                  <div className="muted small" style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>{it.detail}</div>
                </div>
              </div>
            ))}
            {selected.items.length === 0 && <div className="ss-empty">Nothing at this stage yet.</div>}
          </div>
        </Card>
      )}

      <div className="grid grid-2">
        <Explainer title="The V-model" refs={[REFS.vmodel, REFS.nasa, REFS.iso15288]}>
          Forsberg and Mooz drew the project cycle as a “V” in 1991 to tie each definition activity to the verification that checks it. The left leg
          <i> decomposes</i>: stakeholder needs and a concept of operations become system requirements, then an architecture, then detailed designs of each
          subsystem. The bottom is <i>implementation</i> — printing, board fabrication, silicon and firmware. The right leg <i>integrates</i> upward: units are
          verified against their designs, assemblies against the architecture's interfaces, the system against its requirements, and finally the product is
          <i> validated</i> against the stakeholders' needs. ISO/IEC/IEEE 15288 names these the technical processes of the life cycle.
        </Explainer>
        <Explainer title="How Protoforge fills in the V" refs={[REFS.nasa, REFS.incose]}>
          <Eyebrow>Verification vs validation</Eyebrow>
          <b>Verification</b> asks “did we build it right?” — does the design meet each requirement? <b>Validation</b> asks “did we build the right thing?” — does it
          meet the needs? Here, <i>unit verification</i> is every requirement owned by a single subsystem, <i>integration</i> covers requirements spanning subsystems
          and interface requirements, <i>system verification</i> is the whole set, and a need is <i>validated</i> once every requirement derived from it passes.
          Analysis-based evidence comes from the labs; tests and demonstrations on real prototypes come later (EVT and DVT on the Roadmap).
        </Explainer>
      </div>
    </>
  );
}
