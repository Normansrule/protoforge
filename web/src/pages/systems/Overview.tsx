// Mission control: the verification ring, the "run full verification" sweep
// through every lab, status by subsystem, budget margins and top risks.
import { useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, Explainer } from '../../components/ui';
import { Icon } from '../../components/Icon';
import { callInWorker } from '../../engine/worker';
import type { Evaluation, ReqResult } from '../../engine/types';
import {
  Badge, Eyebrow, REFS, Ring, SWEEP_LABS, fmtNum, labForMetric, metricsOfLab, signedPct, statusColor, useReducedMotion, useStudio,
} from './shared';
import type { Budget } from './types';

const STEP_MS = 420;

interface LabRow {
  id: string; title: string; path: string; icon: string;
  metrics: number; results: ReqResult[];
}

export function Overview() {
  const { project, ev, studio, openReq, go } = useStudio();
  const reduced = useReducedMotion();
  const [step, setStep] = useState<number | null>(null); // null = idle (show everything)
  const [runMs, setRunMs] = useState<number | null>(null);
  const timer = useRef<number | null>(null);

  const rows: LabRow[] = useMemo(() => {
    if (!ev) return [];
    return SWEEP_LABS.map((l) => {
      const ms = metricsOfLab(l, ev.metrics);
      const results = project.requirements
        .map((r, i) => ({ r, res: ev.results[i] }))
        .filter(({ r }) => labForMetric(r.verification.metric)?.id === l.id)
        .map(({ res }) => res)
        .filter(Boolean);
      return { id: l.id, title: l.title, path: l.path, icon: l.icon, metrics: ms.length, results };
    });
  }, [ev, project.requirements]);

  useEffect(() => () => { if (timer.current) window.clearTimeout(timer.current); }, []);

  const run = () => {
    if (timer.current) window.clearTimeout(timer.current);
    // re-run the whole engine for real and time it; the reveal below is paced for humans
    // (in the background worker, so the page never freezes)
    callInWorker<Evaluation>('systems.evaluate', project).then((r) => setRunMs(r.ms));
    if (reduced) { setStep(rows.length); return; }
    setStep(0);
    const tick = (k: number) => {
      timer.current = window.setTimeout(() => {
        setStep(k);
        if (k < rows.length) tick(k + 1);
      }, STEP_MS);
    };
    tick(1);
  };

  const running = step !== null && step < rows.length;
  const revealedLabs = step === null ? rows.length : step;
  const revealed = rows.slice(0, revealedLabs).flatMap((r) => r.results);
  const revealedIds = new Set(revealed.map((r) => r.id));
  const all = ev?.results ?? [];
  const done = step === null || step >= rows.length;
  const pass = done ? ev?.pass ?? 0 : revealed.filter((r) => r.status === 'pass').length;
  const fail = done ? ev?.fail ?? 0 : revealed.filter((r) => r.status === 'fail').length;
  const unv = all.length - pass - fail;

  const lines = useMemo(() => {
    const out: { text: string; tone: string }[] = [];
    rows.slice(0, revealedLabs).forEach((r) => {
      if (r.metrics === 0 && r.results.length === 0) return;
      const p = r.results.filter((x) => x.status === 'pass').length;
      const f = r.results.filter((x) => x.status === 'fail').length;
      const reqs = r.results.map((x) => `${x.id} ${x.status === 'pass' ? '✓' : x.status === 'fail' ? '✕' : '○'}`).join('  ');
      out.push({ text: `${f ? '✕' : r.metrics === 0 ? '○' : '✓'} ${r.title.padEnd(18, ' ')} ${String(r.metrics).padStart(2, ' ')} metrics  ${reqs || (p ? '' : '—')}`, tone: f ? 'bad' : 'ok' });
    });
    if (done && step !== null) out.push({ text: `== ${ev?.pass ?? 0} pass · ${ev?.fail ?? 0} fail · ${ev?.unverified ?? 0} unverified${runMs != null ? ` · engine ${runMs.toFixed(0)} ms` : ''}`, tone: (ev?.fail ?? 0) > 0 ? 'bad' : 'ok' });
    return out.slice(-6);
  }, [rows, revealedLabs, done, step, ev, runMs]);

  if (!ev) return <Card title="Verifying…"><div className="muted small">The Rust engine is running every lab's analyses.</div></Card>;

  return (
    <>
      <Card
        title="Verification — every requirement, checked by the labs"
        subtitle={<>Each requirement is bound to a metric a lab computes. Press run to watch the engine sweep the labs in V-model order and light each one up as its results come in.</>}
        actions={
          <button className="btn primary" onClick={run} disabled={running}>
            <Icon name="play" size={14} /> {running ? 'Verifying…' : 'Run full verification'}
          </button>
        }
      >
        <div className="ss-grid-hero">
          <div className="row" style={{ alignItems: 'center', gap: 22, flexWrap: 'wrap' }}>
            <Ring pass={pass} fail={fail} unverified={unv} size={188} stroke={18} label={running ? 'verifying' : 'pass'} />
            <div className="stack" style={{ gap: 8, minWidth: 200, flex: 1 }}>
              <div className="row" style={{ gap: 6 }}>
                <Badge status="pass" label={`pass ${pass}`} />
                <Badge status="fail" label={`fail ${fail}`} />
                <Badge status="unverified" label={`${running ? 'pending' : 'unverified'} ${unv}`} />
              </div>
              <div className="ss-console" aria-live="polite">
                {lines.length === 0 && <div className="muted">$ protoforge evaluate — press “Run full verification”</div>}
                {lines.map((l, i) => <div key={i} className={l.tone} style={{ whiteSpace: 'pre', overflow: 'hidden', textOverflow: 'ellipsis' }}>{l.text}</div>)}
              </div>
              <div className="muted small">
                {ev.metrics.length} metrics from {rows.filter((r) => r.metrics > 0).length} labs · failing: {' '}
                {all.filter((r) => r.status === 'fail').map((r, i) => (
                  <span key={r.id}>{i > 0 && ', '}<button className="ss-link" onClick={() => openReq(r.id)}>{r.id}</button></span>
                ))}
                {ev.fail === 0 && 'none'}
              </div>
            </div>
          </div>
          <div>
            <Eyebrow>Labs</Eyebrow>
            <div className="ss-sweep">
              {rows.map((r, i) => {
                const state = step === null ? 'lit' : i < revealedLabs ? 'lit' : i === revealedLabs && running ? 'scanning' : 'dim';
                const p = r.results.filter((x) => x.status === 'pass').length;
                const f = r.results.filter((x) => x.status === 'fail').length;
                const u = r.results.length - p - f;
                const tone = f > 0 ? 'fail' : p > 0 ? 'pass' : '';
                const n = Math.max(1, r.results.length);
                return (
                  <Link key={r.id} to={r.path} className={`ss-lab ${state} ${state === 'lit' ? tone : ''}`} title={`Open ${r.title}`}>
                    {state === 'scanning' && <span className="ss-scanline" />}
                    <div className="ss-lab-top"><Icon name={r.icon} size={16} /><span className="ss-lab-title">{r.title}</span></div>
                    <div className="ss-lab-stat">
                      <span>{r.metrics ? `${r.metrics} metrics` : 'no metrics yet'}</span>
                      <span>{r.results.length ? `${f ? '✕' : p === r.results.length ? '✓' : '○'} ${p}/${r.results.length}` : ''}</span>
                    </div>
                    <div className="ss-lab-bar" aria-hidden>
                      {state === 'lit' && r.results.length > 0 && (
                        <>
                          {p > 0 && <span style={{ width: `${(p / n) * 100}%`, background: 'var(--good)' }} />}
                          {f > 0 && <span style={{ width: `${(f / n) * 100}%`, background: 'var(--bad)' }} />}
                          {u > 0 && <span style={{ width: `${(u / n) * 100}%`, background: 'var(--muted)', opacity: 0.4 }} />}
                        </>
                      )}
                    </div>
                  </Link>
                );
              })}
            </div>
            {project.requirements.some((r) => !revealedIds.has(r.id) && !labForMetric(r.verification.metric)) && done && (
              <div className="muted small" style={{ marginTop: 8 }}>
                ○ {project.requirements.filter((r) => !labForMetric(r.verification.metric)).length} requirements have no lab metric — verify them by test, inspection or demonstration.
              </div>
            )}
          </div>
        </div>
      </Card>

      <div className="grid grid-2">
        <SubsystemMultiples />
        <Card title="Budget margins" subtitle="Roll-ups of the bill of materials and lab metrics against the requirements that bound them" actions={<button className="btn small" onClick={() => go('budgets')}>Budgets →</button>}>
          {studio ? (
            <div>
              {[studio.budgets.mass, studio.budgets.battery_life, studio.budgets.power, studio.budgets.cost].map((b) => <BudgetGauge key={b.key} b={b} />)}
            </div>
          ) : <div className="muted small">computing…</div>}
        </Card>
      </div>

      <div className="grid grid-3">
        <TopRisks />
        <VProgress />
        <NeedsPanel />
      </div>

      <Explainer title="What the Systems Studio does" refs={[REFS.nasa, REFS.incose, REFS.iso15288]}>
        Systems engineering turns a problem into a product that provably solves it. You write down who the product is for and what they <i>need</i>,
        derive measurable <i>requirements</i> from those needs, split the product into <i>subsystems</i> connected by <i>interfaces</i>, and then
        <i> verify</i> every requirement by analysis, test, inspection or demonstration. The International Council on Systems Engineering (INCOSE) and the
        NASA handbook describe this as a set of technical processes running through the life cycle (ISO/IEC/IEEE 15288). Here, every requirement is bound to a number a lab computes
        — battery life from the power model, mass from the CAD geometry, timing from the chip synthesiser — so the verification matrix stays current while you design.
      </Explainer>
    </>
  );
}

function SubsystemMultiples() {
  const { project, ev, openReq, go } = useStudio();
  if (!ev) return null;
  const byId = new Map(ev.results.map((r) => [r.id, r]));
  return (
    <Card title="Requirement status by subsystem" subtitle="One square per allocated requirement — click one to open it" actions={<button className="btn small" onClick={() => go('traceability')}>Trace →</button>}>
      {project.subsystems.length === 0 && <div className="ss-empty">No subsystems yet — add them in Architecture.</div>}
      <div className="ss-subs">
        {project.subsystems.map((s) => {
          const reqs = project.requirements.filter((r) => r.allocated_to.includes(s.id));
          const f = reqs.filter((r) => byId.get(r.id)?.status === 'fail').length;
          const p = reqs.filter((r) => byId.get(r.id)?.status === 'pass').length;
          return (
            <div key={s.id} className="ss-sub">
              <div className="ss-sub-name"><span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{s.name}</span><span className="muted small tnum">{p}/{reqs.length}</span></div>
              <div className="muted small">{s.id} · {s.domain}</div>
              <div className="ss-cells">
                {reqs.length === 0 && <span className="muted small">no requirements</span>}
                {reqs.map((r) => {
                  const st = byId.get(r.id)?.status ?? 'unverified';
                  return (
                    <button key={r.id} className={`ss-cell ${st}`} title={`${r.id} ${st}: ${r.text}`} aria-label={`${r.id} ${st}`} onClick={() => openReq(r.id)} style={{ border: 'none', cursor: 'pointer', padding: 0 }}>
                      {st === 'pass' ? '✓' : st === 'fail' ? '✕' : '○'}
                    </button>
                  );
                })}
              </div>
              {f > 0 && <div className="small" style={{ color: 'var(--bad-text)', marginTop: 6 }}>✕ {f} failing</div>}
            </div>
          );
        })}
      </div>
    </Card>
  );
}

export function BudgetGauge({ b }: { b: Budget }) {
  const { openReq } = useStudio();
  const limit = b.limit;
  const scale = Math.max(limit ?? 0, b.total, 1e-9) * 1.15;
  const ge = b.cmp === 'ge';
  const ok = b.margin == null ? null : b.margin >= 0;
  const color = ok == null ? 'var(--series-1)' : ok ? 'var(--good)' : 'var(--bad)';
  return (
    <div className="ss-budget-row">
      <div>
        <div style={{ fontWeight: 650, fontSize: 13 }}>{b.label}</div>
        <div className="muted small">{b.requirement ? <button className="ss-link" onClick={() => openReq(b.requirement!)}>{b.requirement}</button> : 'no requirement'}</div>
      </div>
      <div>
        <div className="ss-gauge" title={`${fmtNum(b.total)} ${b.unit}${limit != null ? ` vs ${ge ? '≥' : '≤'} ${fmtNum(limit)} ${b.unit}` : ''}`}>
          <div className="fill" style={{ width: `${Math.min(100, (b.total / scale) * 100)}%`, background: color, opacity: 0.85 }} />
          {limit != null && <div className="limit" style={{ left: `${(limit / scale) * 100}%` }} title={`limit ${fmtNum(limit)} ${b.unit}`} />}
        </div>
        <div className="muted small" style={{ marginTop: 4 }}>
          <b className="tnum" style={{ color: 'var(--text)' }}>{fmtNum(b.total)} {b.unit}</b>
          {limit != null && <> · limit {ge ? '≥' : '≤'} {fmtNum(limit)} {b.unit}</>}
        </div>
      </div>
      <div style={{ textAlign: 'right' }}>
        {b.margin != null ? (
          <span className="small" style={{ color: ok ? 'var(--good-text)' : 'var(--bad-text)', fontWeight: 700 }}>{ok ? '✓' : '✕'} {signedPct(b.margin)}</span>
        ) : <span className="muted small">no limit</span>}
        <div className="muted small">margin</div>
      </div>
    </div>
  );
}

function TopRisks() {
  const { project, studio, go } = useStudio();
  const top = (studio?.fmea ?? []).slice(0, 5);
  return (
    <Card title="Top risks" subtitle="By risk priority number (RPN = S × O × D)" actions={<button className="btn small" onClick={() => go('risks')}>FMEA →</button>}>
      {top.length === 0 && <div className="ss-empty">No risks recorded.</div>}
      {top.map((r) => {
        const risk = project.risks.find((x) => x.id === r.id);
        const c = r.level === 'high' ? 'var(--bad)' : r.level === 'medium' ? 'var(--warn)' : 'var(--good)';
        return (
          <div className="ss-risk" key={r.id}>
            <div className="ss-rpn" style={{ background: `color-mix(in srgb, ${c} 18%, transparent)`, color: 'var(--text)' }}>{r.rpn}</div>
            <div style={{ minWidth: 0 }}>
              <div style={{ fontWeight: 600, fontSize: 13 }}>{risk?.item} — {risk?.failure_mode}</div>
              <div className="muted small" style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{risk?.effect}</div>
            </div>
            <Badge status={r.level === 'high' ? 'fail' : r.level === 'medium' ? 'warn' : 'pass'} label={r.level} />
          </div>
        );
      })}
    </Card>
  );
}

function VProgress() {
  const { studio, go } = useStudio();
  const stages = studio?.vmodel.stages ?? [];
  return (
    <Card title="Life-cycle progress" subtitle="Status of each V-model stage" actions={<button className="btn small" onClick={() => go('vmodel')}>V-model →</button>}>
      <div className="stack" style={{ gap: 7 }}>
        {stages.map((s) => (
          <div key={s.id} title={s.summary}>
            <div className="row" style={{ justifyContent: 'space-between', fontSize: 12.5 }}>
              <span>{s.title}</span>
              <span className="muted tnum">{Math.round(s.progress * 100)} % {s.status === 'attention' ? '!' : s.status === 'done' ? '✓' : ''}</span>
            </div>
            <div style={{ height: 5, borderRadius: 3, background: 'var(--surface-3)', overflow: 'hidden' }}>
              <div style={{ width: `${s.progress * 100}%`, height: '100%', borderRadius: 3, background: statusColor(s.status), transition: 'width 0.4s' }} />
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
}

function NeedsPanel() {
  const { studio, go } = useStudio();
  const needs = studio?.coverage.needs ?? [];
  const map: Record<string, 'pass' | 'fail' | 'partial' | 'unverified' | 'todo'> = { satisfied: 'pass', failing: 'fail', partial: 'partial', unverified: 'unverified', uncovered: 'todo' };
  return (
    <Card title="Stakeholder needs" subtitle="Validated when every derived requirement passes" actions={<button className="btn small" onClick={() => go('traceability')}>Coverage →</button>}>
      {needs.length === 0 && <div className="ss-empty">No needs yet — start in Concept.</div>}
      <div className="stack" style={{ gap: 6 }}>
        {needs.map((n) => (
          <div key={n.id} className="row" style={{ justifyContent: 'space-between', flexWrap: 'nowrap', gap: 8 }}>
            <span style={{ fontSize: 12.5, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}><span className="ss-id">{n.id}</span> {n.text}</span>
            <Badge status={map[n.status]} label={n.status} />
          </div>
        ))}
      </div>
    </Card>
  );
}
