// Compliance & certification planner for the current project: checklist
// traced to components and interfaces, an animated Gantt-style timeline,
// explanations and downloads.
import { useEffect, useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Select, Stat } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { PlayButton, usePlaying, useTicker } from './anim';
import type { ComplianceItem, FileOut, Plan } from './types';

const MARKETS = [
  { id: 'US', label: 'United States' },
  { id: 'EU', label: 'European Union' },
  { id: 'UK', label: 'Great Britain' },
  { id: 'CA', label: 'Canada' },
];

const STAGE_TONE: Record<string, string> = { EVT: 'var(--series-3)', DVT: 'var(--series-1)', PVT: 'var(--series-7)' };

function money(c: [number, number] | null) {
  if (!c) return 'fees vary';
  if (c[1] <= 0) return 'in-house';
  return `$${(c[0] / 1000).toFixed(1)}k–$${(c[1] / 1000).toFixed(1)}k`;
}

function useDone(key: string) {
  const [done, setDone] = useState<Record<string, boolean>>(() => {
    try { return JSON.parse(localStorage.getItem(key) ?? '{}'); } catch { return {}; }
  });
  useEffect(() => { try { localStorage.setItem(key, JSON.stringify(done)); } catch { /* storage unavailable */ } }, [key, done]);
  return [done, setDone] as const;
}

function Row({ it, open, onToggle, done, onDone }: { it: ComplianceItem; open: boolean; onToggle: () => void; done: boolean; onDone: (v: boolean) => void }) {
  return (
    <div className={`bd-comp ${open ? 'open' : ''} ${done ? 'done' : ''}`}>
      <div className="bd-comp-head">
        <input type="checkbox" checked={done} onChange={(e) => onDone(e.target.checked)} aria-label={`Mark ${it.title} done`} style={{ accentColor: 'var(--accent)' }} />
        <button className="bd-comp-title" onClick={onToggle} aria-expanded={open}>
          <span className="bd-stage-dot" style={{ background: STAGE_TONE[it.stage] }} />
          <span>{it.title}</span>
        </button>
        <span className="bd-comp-meta">
          <Chip>{it.stage}</Chip>
          <Chip tone={it.obligation === 'mandatory' ? 'fail' : it.obligation === 'conditional' ? 'warn' : undefined}>{it.obligation === 'mandatory' ? '● required' : it.obligation === 'conditional' ? '◐ if applicable' : '○ voluntary'}</Chip>
          <span className="small muted tnum">{money(it.cost_usd)} · {it.weeks[0]}–{it.weeks[1]} wk</span>
        </span>
      </div>
      <div className="small bd-comp-why">{it.why}{it.traces.length > 0 && ' ← '}{it.traces.slice(0, 6).map((t) => <Chip key={t.kind + t.id} title={t.label}>{t.id}</Chip>)}</div>
      {open && (
        <div className="bd-comp-body">
          <p>{it.explain}</p>
          <div className="small"><b>Standards / rules:</b> {it.standards.join(' · ')}</div>
          <div className="small"><b>Route:</b> {it.route}</div>
          <div className="small"><b>Markets:</b> {it.markets.join(', ')}</div>
          {it.notes && <div className="small"><Chip tone="info">i</Chip> {it.notes}</div>}
          {it.traces.length > 0 && (
            <table className="table" style={{ marginTop: 6 }}>
              <tbody>{it.traces.map((t) => <tr key={t.kind + t.id}><td className="muted small">{t.kind}</td><td className="mono small">{t.id}</td><td className="small">{t.label}</td></tr>)}</tbody>
            </table>
          )}
        </div>
      )}
    </div>
  );
}

function Gantt({ plan, t }: { plan: Plan; t: number }) {
  const end = Math.max(34, ...plan.timeline.map((b) => b.start_week + b.weeks)) + 1;
  const cursor = (t * 2.5) % end;
  const x = (w: number) => `${(100 * w) / end}%`;
  return (
    <div className="bd-gantt">
      <div className="bd-gantt-row bd-gantt-phase">
        <div className="bd-gantt-label small muted">week →</div>
        <div className="bd-gantt-track">
          {plan.phases.map((p) => (
            <div key={p.id} className="bd-gantt-ph" style={{ left: x(p.start_week), width: x(p.end_week - p.start_week), borderColor: STAGE_TONE[p.id] }} title={p.milestones.join(', ')}>
              <b>{p.id}</b> <span className="small muted">{p.milestones.map((m) => m.split(' — ')[0]).join(', ')}</span>
            </div>
          ))}
        </div>
      </div>
      {plan.timeline.map((b) => {
        const active = cursor >= b.start_week && cursor <= b.start_week + b.weeks;
        return (
          <div key={b.id} className="bd-gantt-row">
            <div className={`bd-gantt-label small ${b.activity ? 'muted' : ''}`} title={b.label}>{b.activity ? '◆ ' : ''}{b.label}</div>
            <div className="bd-gantt-track">
              {plan.phases.map((p) => <div key={p.id} className="bd-gantt-bg" style={{ left: x(p.start_week), width: x(p.end_week - p.start_week), background: STAGE_TONE[p.id] }} />)}
              <div className={`bd-gantt-bar ${b.activity ? 'act' : ''} ${active ? 'on' : ''}`} style={{ left: x(b.start_week), width: x(Math.max(0.4, b.weeks)), background: STAGE_TONE[b.stage], color: STAGE_TONE[b.stage] }} />
              <div className="bd-gantt-now" style={{ left: x(cursor) }} />
            </div>
          </div>
        );
      })}
      <div className="bd-gantt-row">
        <div className="bd-gantt-label" />
        <div className="bd-gantt-track bd-gantt-axis">
          {Array.from({ length: Math.floor(end / 4) + 1 }, (_, k) => <span key={k} style={{ left: x(k * 4) }} className="small muted tnum">{k * 4}</span>)}
          <div className="bd-gantt-now" style={{ left: x(cursor), bottom: 14 }} />
        </div>
      </div>
    </div>
  );
}

export function ComplianceTab() {
  const project = useProject((s) => s.project);
  const extraMarkets = (project.design.extra as Record<string, unknown>)?.markets;
  const [markets, setMarkets] = useState<string[]>(Array.isArray(extraMarkets) && extraMarkets.length ? (extraMarkets as string[]) : ['US', 'EU']);
  const [module, setModule] = useState<'auto' | 'yes' | 'no'>('auto');
  const options = { markets, radio_module: module === 'auto' ? null : module === 'yes' };
  const r = useCompute<Plan>('build.compliance', { project, options });
  const md = useCompute<FileOut>('build.compliance_md', { project, options });
  const [open, setOpen] = useState<string | null>('fcc15c');
  const [group, setGroup] = useState<'stage' | 'category'>('stage');
  const [done, setDone] = useDone(`protoforge.build.compliance.${project.meta.id}`);
  const [playing, setPlaying] = usePlaying();
  const t = useTicker(playing);
  const plan = r.data;
  const groups = useMemo(() => {
    if (!plan) return [];
    const keys = group === 'stage' ? ['EVT', 'DVT', 'PVT'] : Array.from(new Set(plan.items.map((i) => i.category)));
    return keys.map((k) => ({ k, items: plan.items.filter((i) => (group === 'stage' ? i.stage : i.category) === k) })).filter((g) => g.items.length);
  }, [plan, group]);
  const f = plan?.features;
  const nDone = plan ? plan.items.filter((i) => done[i.id]).length : 0;
  const csv = () => ['id,title,stage,obligation,category,markets,standards,cost_low_usd,cost_high_usd,weeks_low,weeks_high,traces', ...(plan?.items ?? []).map((i) => [i.id, `"${i.title}"`, i.stage, i.obligation, i.category, i.markets.join(' '), `"${i.standards.join('; ')}"`, i.cost_usd?.[0] ?? '', i.cost_usd?.[1] ?? '', i.weeks[0], i.weeks[1], i.traces.map((x) => x.id).join(' ')].join(','))].join('\n');
  return (
    <div className="stack">
      <ErrorBox error={r.error} />
      <div className="grid grid-sidebar">
        <Card title="Where will you sell?" subtitle="The plan is derived from the project's components, interfaces and requirements">
          <div className="row" style={{ gap: 6 }}>
            {MARKETS.map((m) => {
              const on = markets.includes(m.id);
              return <button key={m.id} className={`chip ${on ? 'info' : ''}`} style={{ cursor: 'pointer' }} aria-pressed={on} onClick={() => setMarkets(on ? markets.filter((x) => x !== m.id) : [...markets, m.id])}>{on ? '✓ ' : ''}{m.label}</button>;
            })}
          </div>
          <Select label="Radio" value={module} onChange={(v) => setModule(v)} options={[{ value: 'auto', label: `Detect from the BOM (${f?.radio_module ? 'module' : 'chip-down'})` }, { value: 'yes', label: 'Pre-certified radio module' }, { value: 'no', label: 'Chip-down radio (own antenna design)' }]} />
          {f && (
            <div className="stack" style={{ gap: 4, marginTop: 6 }}>
              <div className="bd-gtitle">Detected in {project.meta.name}</div>
              <div className="row" style={{ gap: 4 }}>
                {([['radio', `radio ${(f.radio_kinds as string[]).join('/')}`], ['lithium', 'lithium cell'], ['light_source', 'display / light source'], ['eyewear', 'eyewear'], ['wearable', 'skin contact'], ['usb', 'USB charging'], ['polycarbonate', 'polycarbonate']] as [string, string][]).map(([k, l]) => (
                  <Chip key={k} tone={f[k] ? 'info' : undefined}>{f[k] ? '✓' : '○'} {l}</Chip>
                ))}
              </div>
              <div className="small muted">Change the components or interfaces in the <Link to="/systems">Systems Studio</Link> and the plan follows.</div>
            </div>
          )}
        </Card>
        {plan && (
          <div className="grid grid-2" style={{ alignContent: 'start' }}>
            <Stat label="Compliance items" value={plan.items.length} note={`${plan.mandatory} required · ${nDone} checked off`} />
            <Stat label="Indicative budget" value={`$${(plan.total_cost_usd[0] / 1000).toFixed(0)}k–${(plan.total_cost_usd[1] / 1000).toFixed(0)}k`} note="rough order of magnitude — confirm with a lab" tone="warn" />
            <div className="callout small" style={{ gridColumn: '1 / -1' }}>{plan.notes.map((n) => <div key={n}>• {n}</div>)}</div>
            <div className="row" style={{ gridColumn: '1 / -1' }}>
              {md.data && <DownloadButton filename={md.data.filename} content={md.data.content} label="Compliance plan (.md)" small primary />}
              <DownloadButton filename="compliance-checklist.csv" content={csv} label="checklist.csv" small />
              <Link className="btn small" to="/systems">→ Requirements</Link>
            </div>
          </div>
        )}
      </div>
      {plan && (
        <Card title="Timeline" subtitle="Pre-compliance at DVT, formal testing on production-intent units at PVT · phases carry the project milestones" actions={<PlayButton playing={playing} onChange={setPlaying} />}>
          <Gantt plan={plan} t={t} />
        </Card>
      )}
      {plan && (
        <Card title="Checklist" subtitle="Click a line for the explanation, route and full traceability · ticks are saved in this browser" actions={<Select value={group} onChange={(v) => setGroup(v)} options={[{ value: 'stage', label: 'Group by stage' }, { value: 'category', label: 'Group by area' }]} />}>
          {groups.map((g) => (
            <div key={g.k} style={{ marginBottom: 12 }}>
              <div className="bd-gtitle">{g.k} · {g.items.length}</div>
              {g.items.map((it) => <Row key={it.id} it={it} open={open === it.id} onToggle={() => setOpen(open === it.id ? null : it.id)} done={!!done[it.id]} onDone={(v) => setDone({ ...done, [it.id]: v })} />)}
            </div>
          ))}
        </Card>
      )}
      <div className="grid grid-2">
        <Explainer title="Radio, EMC and safety" refs={[{ label: 'FCC 47 CFR Part 15', href: 'https://www.ecfr.gov/current/title-47/chapter-I/subchapter-A/part-15' }, { label: 'Radio Equipment Directive 2014/53/EU', href: 'https://eur-lex.europa.eu/eli/dir/2014/53/oj' }, { label: 'IEC 62368-1' }]}>
          Anything with a clock radiates, so every electronic product meets emissions limits (FCC Part 15 Subpart B in the US). A product that transmits on purpose also needs a radio authorisation (Part 15 Subpart C, FCC ID) — or reuses one by embedding a pre-certified module. In the EU the Radio Equipment Directive (RED) bundles radio spectrum, electromagnetic compatibility (EMC), safety and RF exposure; applying the harmonised standards lets you self-declare and CE-mark. Safety for audio/video and ICT equipment is IEC 62368-1 — for glasses mostly battery fire and hot surfaces on skin (requirement R-008 in the Iris set).
        </Explainer>
        <Explainer title="Batteries, materials and the body" refs={[{ label: 'UN Manual of Tests and Criteria, section 38.3' }, { label: 'IEC 62133-2' }, { label: 'RoHS 2011/65/EU', href: 'https://eur-lex.europa.eu/eli/dir/2011/65/oj' }, { label: 'IEC 62471' }, { label: 'ISO 12870' }]}>
          Lithium cells need UN 38.3 transport tests before anything ships and IEC 62133-2 for safety — buy cells whose maker supplies both. RoHS limits lead and other substances (assemble with lead-free SAC305 — see the reflow calculator), REACH asks you to disclose substances of very high concern, WEEE pays for recycling. A bright display a few centimetres from the eye is checked for photobiological safety (IEC 62471). Frames have their own standard (ISO 12870), and all-day skin contact is worth a biocompatibility screen (ISO 10993 concepts — consult a lab; may not be required for consumer eyewear).
        </Explainer>
      </div>
    </div>
  );
}
