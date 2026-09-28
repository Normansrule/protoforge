// Systems Studio — the front door of Protoforge. Define a product (concept,
// needs, requirements, architecture, trades, risks, milestones) and watch the
// engineering labs verify every requirement live.
import { useCallback, useEffect, useMemo, type ReactNode } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useCompute } from '../engine/hooks';
import { useEvaluation, useProject } from '../state/project';
import { Icon } from '../components/Icon';
import { StudioContext, recordEvaluation, useStudio, type StudioCtx, type TabId } from './systems/shared';
import type { Studio } from './systems/types';
import { Overview } from './systems/Overview';
import { ConceptTab } from './systems/Concept';
import { RequirementsTab } from './systems/Requirements';
import { RequirementDrawer } from './systems/ReqDrawer';
import { ArchitectureTab } from './systems/Architecture';
import { TraceabilityTab } from './systems/Traceability';
import { VModelTab } from './systems/VModel';
import { TradesTab } from './systems/Trades';
import { RisksTab } from './systems/Risks';
import { BudgetsTab } from './systems/Budgets';
import { RoadmapTab } from './systems/Roadmap';
import { ReportTab } from './systems/Report';
import './systems/studio.css';

const TABS: { id: TabId; label: string; icon: string }[] = [
  { id: 'overview', label: 'Mission control', icon: 'home' },
  { id: 'concept', label: 'Concept', icon: 'learn' },
  { id: 'requirements', label: 'Requirements', icon: 'check' },
  { id: 'architecture', label: 'Architecture', icon: 'systems' },
  { id: 'traceability', label: 'Traceability', icon: 'arrow' },
  { id: 'vmodel', label: 'V-model', icon: 'control' },
  { id: 'trades', label: 'Trade studies', icon: 'rl' },
  { id: 'risks', label: 'Risks (FMEA)', icon: 'power' },
  { id: 'budgets', label: 'Budgets', icon: 'mfg' },
  { id: 'roadmap', label: 'Roadmap', icon: 'arrow' },
  { id: 'report', label: 'Report & templates', icon: 'download' },
];

export default function Systems() {
  const project = useProject((s) => s.project);
  const update = useProject((s) => s.update);
  const ev = useEvaluation();
  const [params, setParams] = useSearchParams();
  const tab = (TABS.find((t) => t.id === params.get('tab'))?.id ?? (params.get('req') ? 'requirements' : 'overview')) as TabId;
  const reqId = params.get('req');

  // derived analyses reuse the evaluation we already have (one engine call)
  const studio = useCompute<Studio>('systems.studio', { project, evaluation: ev.data }, !!ev.data);
  useMemo(() => recordEvaluation(ev.data), [ev.data]);

  const go = useCallback((t: TabId) => {
    setParams((p) => {
      const n = new URLSearchParams(p);
      n.set('tab', t);
      n.delete('req');
      return n;
    });
    window.scrollTo({ top: 0 });
  }, [setParams]);
  const openReq = useCallback((id: string) => {
    setParams((p) => {
      const n = new URLSearchParams(p);
      n.set('req', id);
      if (!n.get('tab')) n.set('tab', 'requirements');
      return n;
    });
  }, [setParams]);
  const closeReq = useCallback(() => {
    setParams((p) => {
      const n = new URLSearchParams(p);
      n.delete('req');
      return n;
    });
  }, [setParams]);

  useEffect(() => {
    const el = document.querySelector<HTMLElement>('.ss-tab.active');
    const bar = el?.parentElement;
    if (el && bar) bar.scrollTo({ left: el.offsetLeft - bar.clientWidth / 2 + el.clientWidth / 2, behavior: 'smooth' });
  }, [tab]);

  const ctx: StudioCtx = useMemo(() => ({ project, ev: ev.data, studio: studio.data, update, go, openReq }), [project, ev.data, studio.data, update, go, openReq]);
  const counts = tabCounts(ctx);

  return (
    <StudioContext.Provider value={ctx}>
      <div className="ss-root">
      <Hero />
      <nav className="ss-tabs" role="tablist" aria-label="Systems Studio sections">
        {TABS.map((t) => (
          <button key={t.id} role="tab" aria-selected={tab === t.id} className={`ss-tab ${tab === t.id ? 'active' : ''}`} onClick={() => go(t.id)}>
            <Icon name={t.icon} size={15} /> {t.label}
            {counts[t.id]}
          </button>
        ))}
      </nav>
      {(ev.error || studio.error) && <div className="callout" style={{ borderLeftColor: 'var(--bad)', color: 'var(--bad-text)', marginBottom: 12 }}>{ev.error ?? studio.error}</div>}
      <div className="stack">
        {tab === 'overview' && <Overview />}
        {tab === 'concept' && <ConceptTab />}
        {tab === 'requirements' && <RequirementsTab />}
        {tab === 'architecture' && <ArchitectureTab />}
        {tab === 'traceability' && <TraceabilityTab />}
        {tab === 'vmodel' && <VModelTab />}
        {tab === 'trades' && <TradesTab />}
        {tab === 'risks' && <RisksTab />}
        {tab === 'budgets' && <BudgetsTab />}
        {tab === 'roadmap' && <RoadmapTab />}
        {tab === 'report' && <ReportTab />}
      </div>
      <RequirementDrawer id={reqId} onClose={closeReq} />
      </div>
    </StudioContext.Provider>
  );
}

function tabCounts(c: StudioCtx): Partial<Record<TabId, ReactNode>> {
  const s = c.studio;
  const dot = (color: string, title: string) => <span className="ss-dot" style={{ background: color }} title={title} aria-label={title} />;
  const fail = c.ev?.fail ?? 0;
  const highRisks = s?.fmea.filter((r) => r.level === 'high').length ?? 0;
  const disagree = s?.trades.filter((t) => !t.decision_agrees).length ?? 0;
  const lintErr = s?.quality.filter((q) => q.severity === 'error').length ?? 0;
  return {
    requirements: (
      <>
        <span className="ss-num">{c.project.requirements.length}</span>
        {(fail > 0 || lintErr > 0) && dot('var(--bad)', `${fail} failing, ${lintErr} lint errors`)}
      </>
    ),
    risks: highRisks > 0 ? <><span className="ss-num">{c.project.risks.length}</span>{dot('var(--warn)', `${highRisks} high risks`)}</> : <span className="ss-num">{c.project.risks.length}</span>,
    trades: disagree > 0 ? dot('var(--warn)', 'a recorded decision differs from the weighted winner') : undefined,
    architecture: <span className="ss-num">{c.project.subsystems.length}</span>,
  };
}

function Hero() {
  const { project, ev, studio, go } = useStudio();
  const m = project.meta;
  const pass = ev?.pass ?? 0, fail = ev?.fail ?? 0, unv = ev?.unverified ?? 0;
  const total = pass + fail + unv;
  return (
    <header className="ss-hero">
      <div style={{ minWidth: 0 }}>
        <div className="page-kicker">Systems Studio · idea → requirements → verified design</div>
        <h1 style={{ marginTop: 6 }}>{m.name || 'Untitled product'}</h1>
        {m.tagline && <div className="ss-tagline">{m.tagline}</div>}
        <div className="ss-hero-meta">
          {m.codename && <span className="chip info">◆ {m.codename}</span>}
          <span className="chip">v{m.version || '0.0.0'}</span>
          <span className="chip">{project.needs.length} needs</span>
          <span className="chip">{project.requirements.length} requirements</span>
          <span className="chip">{project.subsystems.length} subsystems</span>
          {studio && <span className="chip" title="Traceability completeness: needs covered, requirements traced, allocated and bound to a metric">{Math.round(studio.coverage.completeness * 100)} % traced</span>}
          <button className="btn small" onClick={() => go('concept')}>Edit product</button>
        </div>
      </div>
      <div className="ss-hero-right">
        <MiniRing pass={pass} fail={fail} unv={unv} />
        <div className="ss-hero-legend" aria-label="Requirement status">
          <span className="k"><span className="ss-sw" style={{ background: 'var(--good)' }} />✓ pass <b>{pass}</b></span>
          <span className="k"><span className="ss-sw" style={{ background: 'var(--bad)' }} />✕ fail <b>{fail}</b></span>
          <span className="k"><span className="ss-sw" style={{ background: 'var(--muted)', opacity: 0.5 }} />○ unverified <b>{unv}</b></span>
          <span className="muted small">{total ? `${Math.round((pass / total) * 100)} % verified` : 'no requirements yet'}</span>
        </div>
      </div>
    </header>
  );
}

function MiniRing({ pass, fail, unv }: { pass: number; fail: number; unv: number }) {
  const size = 96, stroke = 10, r = (size - stroke) / 2, C = 2 * Math.PI * r;
  const total = Math.max(1, pass + fail + unv);
  let acc = 0;
  return (
    <svg width={size} height={size} role="img" aria-label={`${pass} of ${pass + fail + unv} pass`}>
      <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--surface-3)" strokeWidth={stroke} />
      <g transform={`rotate(-90 ${size / 2} ${size / 2})`}>
        {[{ v: pass, c: 'var(--good)' }, { v: fail, c: 'var(--bad)' }].map((s, i) => {
          const len = (s.v / total) * C;
          const el = s.v > 0 ? <circle key={i} cx={size / 2} cy={size / 2} r={r} fill="none" stroke={s.c} strokeWidth={stroke} strokeDasharray={`${Math.max(0, len - 2)} ${C}`} strokeDashoffset={-acc} /> : null;
          acc += len;
          return el;
        })}
      </g>
      <text x={size / 2} y={size / 2 + 7} textAnchor="middle" fontSize={21} fontWeight={800} fill="var(--text)">{pass}<tspan fontSize={12} fill="var(--muted)">/{pass + fail + unv}</tspan></text>
    </svg>
  );
}

