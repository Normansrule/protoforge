// Where to send it: vendor directory with filters and an animated
// "Protoforge output → vendor → back to you" flow for the selected vendor.
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, Chip, DownloadButton, ErrorBox, Explainer } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { PlayButton, usePlaying, useTicker } from './anim';
import type { Category, Vendor } from './types';

function Flow({ v, t }: { v: Vendor; t: number }) {
  const files = v.send.length ? v.send.map((s) => s.label) : ['your files'];
  const p = (t / 4) % 1;
  const out = p < 0.5;
  const q = out ? p / 0.5 : (p - 0.5) / 0.5;
  const x = out ? 90 + q * 170 : 260 - q * 170;
  return (
    <svg viewBox="0 0 360 170" className="bd-anim bd-flow" role="img" aria-label={`Files go from Protoforge to ${v.name}; parts come back`}>
      <rect x={10} y={40} width={90} height={90} rx={12} fill="var(--accent-soft)" stroke="var(--accent)" />
      <text x={55} y={62} textAnchor="middle" className="bd-svg-label">Protoforge</text>
      {files.slice(0, 4).map((f, k) => <text key={f} x={55} y={80 + k * 12} textAnchor="middle" className="bd-svg-small">{f.length > 18 ? f.slice(0, 17) + '…' : f}</text>)}
      <rect x={260} y={40} width={90} height={90} rx={12} fill="var(--surface-2)" stroke="var(--border-strong)" />
      <text x={305} y={80} textAnchor="middle" className="bd-svg-label">{v.name.length > 14 ? v.name.slice(0, 13) + '…' : v.name}</text>
      <text x={305} y={96} textAnchor="middle" className="bd-svg-small">{v.region}</text>
      <path d="M100,70 C160,40 200,40 260,70" fill="none" stroke="var(--accent)" strokeDasharray="4 4" />
      <path d="M260,100 C200,130 160,130 100,100" fill="none" stroke="var(--series-3)" strokeDasharray="4 4" />
      <text x={180} y={34} textAnchor="middle" className="bd-svg-small">upload / order</text>
      <text x={180} y={148} textAnchor="middle" className="bd-svg-small">{v.returns ? (v.returns.length > 44 ? v.returns.slice(0, 43) + '…' : v.returns) : 'parts, reports'}</text>
      {out ? (
        <g transform={`translate(${x},${70 - Math.sin(q * Math.PI) * 22})`}>
          <path d="M-8,-10 h11 l5,5 v15 h-16 Z" fill="var(--surface-1)" stroke="var(--accent)" />
          <line x1={-5} y1={-3} x2={4} y2={-3} stroke="var(--accent)" />
          <line x1={-5} y1={1} x2={4} y2={1} stroke="var(--accent)" />
        </g>
      ) : (
        <g transform={`translate(${x},${100 + Math.sin(q * Math.PI) * 22})`}>
          <rect x={-10} y={-8} width={20} height={16} rx={2} fill="var(--series-3)" opacity={0.8} />
          <line x1={-10} y1={-2} x2={10} y2={-2} stroke="var(--surface-1)" />
        </g>
      )}
    </svg>
  );
}

export function VendorsTab() {
  const d = useCompute<{ categories: Category[]; vendors: Vendor[] }>('build.vendors', {});
  const [cat, setCat] = useState('all');
  const [q, setQ] = useState('');
  const [sel, setSel] = useState('JLCPCB');
  const [playing, setPlaying] = usePlaying();
  const t = useTicker(playing);
  const list = useMemo(() => (d.data?.vendors ?? []).filter((v) => (cat === 'all' || v.categories.includes(cat)) && (q === '' || `${v.name} ${v.services} ${v.formats.join(' ')}`.toLowerCase().includes(q.toLowerCase()))), [d.data, cat, q]);
  const selected = d.data?.vendors.find((v) => v.name === sel) ?? list[0];
  const catLabel = (id: string) => d.data?.categories.find((c) => c.id === id)?.label ?? id;
  const md = () => `# Where to send it — vendor directory\n\nFacts only: names, websites, services, accepted formats. No prices or endorsements.\n\n${(d.data?.vendors ?? []).map((v) => `## ${v.name}${v.url ? ` (${v.url})` : ''}\n\n${v.services}\n\n- Categories: ${v.categories.map(catLabel).join(', ')}\n- Accepts: ${v.formats.join('; ') || '—'}\n- Send from Protoforge: ${v.send.map((s) => s.label).join('; ') || '—'}\n${v.note ? `- Note: ${v.note}\n` : ''}`).join('\n')}`;
  return (
    <div className="stack">
      <ErrorBox error={d.error} />
      {selected && (
        <Card title={`${selected.name}: files out, parts back`} subtitle="Pick any card below to see its flow" actions={<PlayButton playing={playing} onChange={setPlaying} />}>
          <div className="bd-flow-wrap">
            <Flow v={selected} t={t} />
            <div className="small">
              <div className="muted">Accepts</div>
              <div className="row" style={{ gap: 4, margin: '4px 0 8px' }}>{selected.formats.map((f) => <Chip key={f}>{f}</Chip>)}</div>
              <div className="muted">Make these files in Protoforge</div>
              <ul className="bd-fixes">{selected.send.map((s) => <li key={s.label}><Link to={s.route}>{s.label}</Link></li>)}</ul>
            </div>
          </div>
        </Card>
      )}
      <div className="row" style={{ gap: 6 }}>
        <button className={`chip ${cat === 'all' ? 'info' : ''}`} style={{ cursor: 'pointer' }} aria-pressed={cat === 'all'} onClick={() => setCat('all')}>All</button>
        {d.data?.categories.map((c) => <button key={c.id} className={`chip ${cat === c.id ? 'info' : ''}`} style={{ cursor: 'pointer' }} aria-pressed={cat === c.id} onClick={() => setCat(c.id)}>{c.label}</button>)}
        <input className="bd-search" placeholder="Search name, service, format…" value={q} onChange={(e) => setQ(e.target.value)} aria-label="Search vendors" />
      </div>
      <div className="bd-vendors">
        {list.map((v) => (
          <div key={v.name} className={`card bd-vendor ${v.category_card ? 'cat' : ''} ${selected?.name === v.name ? 'active' : ''}`} onClick={() => setSel(v.name)} role="button" tabIndex={0} onKeyDown={(e) => { if (e.key === 'Enter') setSel(v.name); }}>
            <div className="bd-vendor-head">
              {v.url ? <a href={v.url} target="_blank" rel="noreferrer" onClick={(e) => e.stopPropagation()}><b>{v.name} ↗</b></a> : <b>{v.name}</b>}
              <span className="small muted">{v.region}</span>
            </div>
            <div className="row" style={{ gap: 4, margin: '6px 0' }}>{v.categories.map((c) => <Chip key={c} tone="info">{catLabel(c)}</Chip>)}</div>
            <div className="small">{v.services}</div>
            {v.formats.length > 0 && <div className="small muted" style={{ marginTop: 6 }}>Accepts: {v.formats.join(' · ')}</div>}
            {v.note && <div className="small" style={{ marginTop: 6 }}><Chip tone="warn">! note</Chip> {v.note}</div>}
          </div>
        ))}
      </div>
      <div className="grid grid-2">
        <div className="callout small">Facts only: names, websites, service categories and accepted formats — no prices, lead times, ratings or endorsements. Battery cells, cable assemblies and contract manufacturers are described as categories rather than naming a company. The <Link to="/mfg">Manufacturing Hub</Link> keeps the cost model, order guides and the one-click fabrication package.</div>
        <div className="row" style={{ alignItems: 'flex-start' }}><DownloadButton filename="vendor-directory.md" content={md} label="vendor-directory.md" small /></div>
      </div>
      <Explainer title="What each shop needs from you">
        Board houses want <b>Gerber + Excellon drill</b> files (one zip), assemblers also a <b>BOM</b> (bill of materials) and <b>CPL</b> (component placement list). Machining, molding and casting shops quote from <b>STEP</b> solids plus a <b>2D drawing</b> with tolerances, threads and finishes; print services take <b>STL</b> or STEP; sheet and laser services take <b>DXF</b> outlines. Test labs want a product description, block diagram, BOM, draft manual and samples — the Compliance tab produces the plan they start from.
      </Explainer>
    </div>
  );
}
