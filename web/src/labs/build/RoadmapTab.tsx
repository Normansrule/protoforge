// Prototype → product roadmap as an animated journey along a road with five
// stations; the product morphs from breadboard to boxed units.
import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, Chip, DownloadButton, ErrorBox, Explainer } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { PlayButton, usePlaying, useTicker } from './anim';
import type { RoadStage } from './types';

const W = 720;
const XS = [70, 215, 360, 505, 650];

function Product({ k }: { k: number }) {
  switch (k) {
    case 0:
      return (
        <g>
          <rect x={-18} y={-10} width={36} height={20} rx={2} fill="var(--surface-3)" stroke="var(--border-strong)" />
          {[-12, -4, 4, 12].map((x) => <circle key={x} cx={x} cy={0} r={1.6} fill="var(--muted)" />)}
          <path d="M-12,-10 q6,-10 12,0 M0,-10 q6,-8 12,0" stroke="var(--series-2)" fill="none" />
        </g>
      );
    case 1:
      return (
        <g>
          <rect x={-20} y={-6} width={16} height={12} rx={5} fill="none" stroke="var(--series-3)" strokeWidth={3} />
          <rect x={4} y={-6} width={16} height={12} rx={5} fill="none" stroke="var(--series-3)" strokeWidth={3} />
          <rect x={-3} y={4} width={10} height={6} fill="var(--mask)" />
        </g>
      );
    case 2:
      return (
        <g>
          <rect x={-20} y={-6} width={16} height={12} rx={5} fill="none" stroke="var(--series-1)" strokeWidth={3} />
          <rect x={4} y={-6} width={16} height={12} rx={5} fill="none" stroke="var(--series-1)" strokeWidth={3} />
          <path d="M-24,-12 l4,4 M24,-12 l-4,4" stroke="var(--warn)" />
        </g>
      );
    case 3:
      return (
        <g>
          <rect x={-22} y={-12} width={44} height={24} rx={3} fill="var(--surface-2)" stroke="var(--series-7)" />
          <rect x={-16} y={-6} width={12} height={10} rx={4} fill="none" stroke="var(--series-7)" strokeWidth={2} />
          <rect x={4} y={-6} width={12} height={10} rx={4} fill="none" stroke="var(--series-7)" strokeWidth={2} />
        </g>
      );
    default:
      return (
        <g>
          {[[-16, 0], [0, 0], [16, 0], [-8, -12], [8, -12]].map(([x, y], n) => <rect key={n} x={x - 7} y={y - 5} width={14} height={11} fill="var(--copper)" opacity={0.7} stroke="var(--border-strong)" />)}
        </g>
      );
  }
}

function Road({ stages, pos, sel, onSel }: { stages: RoadStage[]; pos: number; sel: number; onSel: (k: number) => void }) {
  const k0 = Math.min(stages.length - 1, Math.floor(pos));
  const f = pos - k0;
  const x = k0 < stages.length - 1 ? XS[k0] + (XS[k0 + 1] - XS[k0]) * f : XS[k0];
  const y = 96 - Math.sin(f * Math.PI) * 10;
  return (
    <svg viewBox={`0 0 ${W} 170`} className="bd-road" role="img" aria-label="Roadmap from proof of concept to mass production">
      <path d={`M20,110 C150,110 150,100 ${W / 2},105 S${W - 150},110 ${W - 20},110`} stroke="var(--surface-3)" strokeWidth={20} fill="none" strokeLinecap="round" />
      <path d={`M20,110 C150,110 150,100 ${W / 2},105 S${W - 150},110 ${W - 20},110`} stroke="var(--text-2)" strokeWidth={1.5} strokeDasharray="10 8" fill="none" />
      {stages.map((s, k) => (
        <g key={s.id} transform={`translate(${XS[k]},0)`} className="bd-station" onClick={() => onSel(k)} role="button" tabIndex={0} onKeyDown={(e) => { if (e.key === 'Enter') onSel(k); }}>
          <circle cx={0} cy={108} r={sel === k ? 13 : 10} fill={k <= pos + 0.001 ? 'var(--accent)' : 'var(--surface-2)'} stroke="var(--accent)" strokeWidth={2} />
          <text x={0} y={112} textAnchor="middle" className="bd-svg-label" fill={k <= pos + 0.001 ? '#fff' : 'var(--text)'}>{k + 1}</text>
          <text x={0} y={144} textAnchor="middle" className="bd-svg-label">{s.title.split(' — ')[0]}</text>
          <text x={0} y={160} textAnchor="middle" className="bd-svg-small">{s.quantity}</text>
        </g>
      ))}
      <g transform={`translate(${x},${y - 34})`}>
        <Product k={Math.round(pos) > k0 && f > 0.5 ? k0 + 1 : k0} />
      </g>
    </svg>
  );
}

export function RoadmapTab() {
  const project = useProject((s) => s.project);
  const r = useCompute<RoadStage[]>('build.roadmap', { project });
  const [playing, setPlaying] = usePlaying();
  const t = useTicker(playing);
  const [sel, setSel] = useState(1);
  const [manual, setManual] = useState<number | null>(null);
  const stages = r.data ?? [];
  const n = stages.length || 5;
  // 3 s per leg + 2 s pause at each station
  const cyc = (t % (n * 5)) / 5;
  const leg = Math.floor(cyc);
  const within = cyc - leg;
  const pos = manual ?? Math.min(n - 1, leg + Math.max(0, (within - 0.4) / 0.6) * (leg < n - 1 ? 1 : 0));
  useEffect(() => {
    if (manual === null && playing) setSel(Math.min(n - 1, Math.round(pos)));
  }, [pos, manual, playing, n]);
  const s = stages[sel];
  const md = () => `# ${project.meta.name} — prototype to product roadmap\n\n${stages.map((x) => `## ${x.title}\n\n_${x.question}_ — ${x.quantity}\n\n**Build**\n${x.builds.map((b) => `- ${b.what} (${b.process}; vendors: ${b.vendor_category})`).join('\n')}\n\n**Test**\n${x.tests.map((q) => `- ${q}`).join('\n')}\n\n**Exit criteria**\n${x.exit.map((q) => `- ${q}`).join('\n')}\n${x.milestones.length ? `\nMilestones: ${x.milestones.join(', ')}\n` : ''}${x.compliance.length ? `\nCompliance: ${x.compliance.join('; ')}\n` : ''}${x.risks.length ? `\nRisks: ${x.risks.join('; ')}\n` : ''}`).join('\n')}`;
  return (
    <div className="stack">
      <ErrorBox error={r.error} />
      {stages.length > 0 && (
        <Card title="The journey" subtitle="Click a station, or play to travel from proof of concept to mass production" actions={<><PlayButton playing={playing} onChange={(p) => { setPlaying(p); if (p) setManual(null); }} /><DownloadButton filename="roadmap.md" content={md} label="roadmap.md" small /></>}>
          <Road stages={stages} pos={pos} sel={sel} onSel={(k) => { setSel(k); setManual(k); setPlaying(false); }} />
        </Card>
      )}
      {s && (
        <div className="grid grid-2">
          <Card title={s.title} subtitle={`${s.question} · ${s.quantity}${s.typical_weeks[1] > 0 ? ` · typically ${s.typical_weeks[0]}–${s.typical_weeks[1]} weeks` : ''}`}>
            <div className="bd-gtitle">Build</div>
            {s.builds.map((b) => (
              <div key={b.what} className="bd-build">
                <div>{b.what}</div>
                <div className="small muted">{b.process} · vendors: {b.vendor_category} · <Link to={b.route}>files from {b.route.replace('/', '')}</Link></div>
              </div>
            ))}
            <div className="bd-gtitle" style={{ marginTop: 10 }}>Test</div>
            <ul className="bd-fixes">{s.tests.map((x) => <li key={x}>{x}</li>)}</ul>
            <div className="bd-gtitle" style={{ marginTop: 10 }}>Exit criteria</div>
            <ul className="bd-fixes">{s.exit.map((x) => <li key={x}>✓ {x}</li>)}</ul>
          </Card>
          <div className="stack">
            <Card title="Linked to the project">
              <div className="small muted">Milestones</div>
              <div className="row" style={{ gap: 4, margin: '4px 0 10px' }}>{s.milestones.length ? s.milestones.map((m) => <Chip key={m} tone="info">{m}</Chip>) : <span className="small muted">none mapped</span>}</div>
              <div className="small muted">Compliance at this stage</div>
              <ul className="bd-fixes">{s.compliance.length ? s.compliance.map((x) => <li key={x}>{x}</li>) : <li className="muted">—</li>}</ul>
              <div className="small muted" style={{ marginTop: 8 }}>FMEA risks to retire here</div>
              <ul className="bd-fixes">{s.risks.length ? s.risks.map((x) => <li key={x}>{x}</li>) : <li className="muted">—</li>}</ul>
            </Card>
            <Explainer title="EVT → DVT → PVT">
              Each build answers one question before money is committed to the next: <b>EVT</b> (engineering validation) — does it work? <b>DVT</b> (design validation) — does it survive and pass pre-compliance, in production materials? <b>PVT</b> (production validation) — can the factory build it at yield with hard tools and certified units? The <Link to="/mfg">Manufacturing Hub</Link> keeps a checklist per stage and the cost model; this roadmap is the build recipe.
            </Explainer>
          </div>
        </div>
      )}
    </div>
  );
}
