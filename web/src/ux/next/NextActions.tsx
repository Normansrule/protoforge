// "What should I do next?" — the open issues on the current project, ranked
// by the engine (systems.next_actions), each with the lab tab to open, the
// design parameters that measurably move its metric, and a one-parameter
// "Try fixing this" solver (systems.solve_lever) whose side effects you see
// before you apply anything.
import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { callInWorker } from '../../engine/worker';
import { useProject } from '../../state/project';
import type { Project } from '../../engine/types';
import { fmtNum } from '../../components/Plot';
import { MarginBar } from '../../pages/systems/shared';
import type { Action, ActionKind, Change, Lever, LeversOut, NextActionsOut, Severity, Solve, Where } from './types';
import './next.css';

// ------------------------------------------------------------------ engine

interface Run { data: NextActionsOut | null; error: string | null; ms: number; running: boolean }

/** Last result, shared by every mounted list (Overview and Systems Studio). */
const shared: { project: Project | null; data: NextActionsOut | null; ms: number } = { project: null, data: null, ms: 0 };
let seq = 0;

function useNextActions(project: Project): Run {
  const [run, setRun] = useState<Run>(() => ({ data: shared.data, error: null, ms: shared.ms, running: shared.project !== project }));
  useEffect(() => {
    if (shared.project === project && shared.data) {
      setRun({ data: shared.data, error: null, ms: shared.ms, running: false });
      return;
    }
    setRun((r) => ({ ...r, running: true }));
    const my = ++seq;
    const t = window.setTimeout(() => {
      callInWorker<NextActionsOut>('systems.next_actions', { project }).then((r) => {
        if (r.data) {
          shared.project = project;
          shared.data = r.data;
          shared.ms = r.ms;
        }
        // a newer edit supersedes this answer, but any answer beats a skeleton
        if (my === seq || !shared.data) setRun({ data: r.data ?? shared.data, error: r.error, ms: r.ms, running: my !== seq });
        else setRun((s) => ({ ...s, data: shared.data }));
      });
    }, 350);
    return () => window.clearTimeout(t);
  }, [project]);
  return run;
}

/** Lazily measured levers (expensive modules), per project and metric. */
const leverCache = new WeakMap<Project, Map<string, LeversOut>>();

// ------------------------------------------------------------------ vocabulary

const SEVERITY: Record<Severity, { word: string; icon: string; tone: string }> = {
  critical: { word: 'Critical', icon: '‼', tone: 'fail' },
  high: { word: 'High', icon: '!', tone: 'fail' },
  medium: { word: 'Medium', icon: '◆', tone: 'warn' },
  low: { word: 'Low', icon: '•', tone: 'info' },
  info: { word: 'Info', icon: 'i', tone: '' },
};

const KIND: Record<ActionKind, string> = {
  failing: 'Failing requirement',
  unverified: 'Unverified',
  goal: 'Open goal',
  risk: 'High risk',
  quality_error: 'Requirement error',
  marginal: 'Marginal pass',
  gate: 'Stage gate',
  no_verification: 'No verification',
  traceability: 'Traceability gap',
  quality: 'Wording',
  quality_info: 'Wording',
};

type Filter = 'all' | 'failing' | 'goals' | 'marginal' | 'gaps';
const FILTERS: { id: Filter; label: string; kinds: ActionKind[] | null }[] = [
  { id: 'all', label: 'All', kinds: null },
  { id: 'failing', label: 'Failing', kinds: ['failing', 'unverified'] },
  { id: 'goals', label: 'Goals', kinds: ['goal'] },
  { id: 'marginal', label: 'Marginal', kinds: ['marginal'] },
  { id: 'gaps', label: 'Gaps', kinds: ['risk', 'quality_error', 'gate', 'no_verification', 'traceability', 'quality', 'quality_info'] },
];

const EMPTY: Record<Filter, string> = {
  all: 'Nothing open: every requirement passes with margin and the next gate is clear.',
  failing: 'Nothing failing: every binding requirement passes.',
  goals: 'No open goals: every stretch target is met.',
  marginal: 'No thin margins: every pass clears its target comfortably.',
  gaps: 'No gaps: requirements are verifiable, traced, allocated and well worded.',
};

const num = (v: number) => (Number.isInteger(v) && Math.abs(v) < 1e4 ? String(v) : fmtNum(v));
const fmtV = (v: number | null | undefined, unit = '') => (v == null || !isFinite(v) ? '—' : `${num(v)}${unit ? ` ${unit}` : ''}`);
const signed = (v: number) => (v >= 0 ? '+' : '−') + num(Math.abs(v));

/** Split a trailing "(R-012)" off a title so the id never wraps mid-way. */
function Title({ text }: { text: string }) {
  const m = /^(.*)\s(\([^()]+\))$/.exec(text);
  if (!m) return <>{text}</>;
  return <>{m[1]} <span className="na-rid">{m[2]}</span></>;
}

// ------------------------------------------------------------------ navigation

/** Open a lab tab. Labs that read `?tab=` select it themselves; for the
 *  others the matching tab button is clicked once the lazy page mounts. */
function selectTabLater(where: Where) {
  if (!where.tab_label || where.route === '/systems') return;
  const want = where.tab_label;
  const t0 = performance.now();
  const tick = () => {
    const btn = Array.from(document.querySelectorAll<HTMLButtonElement>('main [role="tab"], [role="tab"]')).find((b) => b.textContent?.trim() === want);
    if (btn) {
      if (btn.getAttribute('aria-selected') !== 'true') btn.click();
      return;
    }
    if (performance.now() - t0 < 5000) window.setTimeout(tick, 120);
  };
  window.setTimeout(tick, 60);
}

function GoLink({ where, compact }: { where: Where; compact?: boolean }) {
  return (
    <Link className="na-go" to={where.href} onClick={() => selectTabLater(where)} title={`Open ${where.label}`}>
      Go to {compact ? where.page : where.label} <span aria-hidden>→</span>
    </Link>
  );
}

// ------------------------------------------------------------------ project edits

function setPath(obj: Record<string, unknown>, path: string, value: number) {
  const segs = path.split('.');
  let cur: Record<string, unknown> | unknown[] = obj;
  for (let i = 0; i < segs.length - 1; i++) {
    const k = segs[i];
    const next = Array.isArray(cur) ? cur[Number(k)] : (cur as Record<string, unknown>)[k];
    if (next == null || typeof next !== 'object') return;
    cur = next as Record<string, unknown>;
  }
  const last = segs[segs.length - 1];
  if (Array.isArray(cur)) cur[Number(last)] = value;
  else (cur as Record<string, unknown>)[last] = value;
}

function applyChanges(changes: Change[], which: 'before' | 'after') {
  useProject.getState().update((d) => {
    for (const c of changes) {
      const v = which === 'after' ? c.after : c.before;
      if (c.kind === 'design') setPath(d.design as unknown as Record<string, unknown>, c.path, v);
      else {
        const comp = d.components.find((x) => x.refdes === c.refdes);
        if (comp) setPath(comp as unknown as Record<string, unknown>, c.path, v);
      }
    }
  });
}

interface Applied { solve: Solve; changes: Change[] }

// ------------------------------------------------------------------ pieces

function SeverityPill({ s }: { s: Severity }) {
  const v = SEVERITY[s];
  return <span className={`chip na-sev ${v.tone}`}><span aria-hidden>{v.icon}</span> {v.word}</span>;
}

function Readiness({ out, compact }: { out: NextActionsOut; compact?: boolean }) {
  const s = out.summary;
  const n = Math.max(1, s.requirements);
  const segs = [
    { key: 'ok', n: s.pass_with_margin, label: 'pass with margin', cls: 'ok' },
    { key: 'thin', n: s.marginal, label: 'marginal', cls: 'thin' },
    { key: 'goal', n: s.open_goals, label: s.open_goals === 1 ? 'open goal' : 'open goals', cls: 'goal' },
    { key: 'fail', n: s.failing, label: 'failing', cls: 'fail' },
    { key: 'unv', n: s.unverified, label: 'unverified', cls: 'unv' },
  ];
  return (
    <div className={`na-ready ${compact ? 'compact' : ''}`}>
      <div className="na-ready-head">
        <div>
          <span className="na-ready-pct">{Math.round(s.readiness_pct)} %</span>
          <span className="na-ready-label"> ready — {s.pass_with_margin} of {s.requirements} requirements pass with at least {Math.round(s.marginal_threshold * 100)} % margin</span>
        </div>
      </div>
      <div className="na-meter" role="img" aria-label={segs.map((g) => `${g.n} ${g.label}`).join(', ')}>
        {segs.filter((g) => g.n > 0).map((g) => (
          <span key={g.key} className={`na-seg ${g.cls}`} style={{ width: `${(g.n / n) * 100}%` }} />
        ))}
      </div>
      <ul className="na-legend" aria-hidden>
        {segs.filter((g) => g.n > 0).map((g) => (
          <li key={g.key}><i className={`na-dot ${g.cls}`} /> {g.n} {g.label}</li>
        ))}
      </ul>
    </div>
  );
}

function LeverRow({ lever, action, onTry, busy }: { lever: Lever; action: Action; onTry?: () => void; busy?: boolean }) {
  const up = lever.direction === 'increase';
  const stepTxt = `${signed(lever.step)}${lever.unit ? ` ${lever.unit}` : ''}`;
  return (
    <li className="na-lever">
      <div className="na-lever-main">
        <span className={`na-arrow ${up ? 'up' : 'down'}`} aria-hidden>{up ? '↑' : '↓'}</span>
        <div className="na-lever-text">
          <div>
            <b>{up ? 'Increase' : 'Decrease'}</b> {lever.label}{' '}
            <span className="muted">(now {fmtV(lever.value, lever.unit)})</span>
          </div>
          <div className="small muted">
            Measured: {stepTxt} moves {action.metric_label ? action.metric_label.toLowerCase() : action.metric} by {signed(lever.delta)}{action.unit ? ` ${action.unit}` : ''} ({signed(lever.effect_pct)} %)
            {lever.estimate != null && (
              <> · about <b className="na-num">{fmtV(lever.estimate, lever.unit)}</b> reaches the target (straight-line estimate)</>
            )}
          </div>
        </div>
      </div>
      {onTry && (
        <button className="btn small primary na-try" onClick={onTry} disabled={busy} aria-label={`Try fixing ${action.requirements[0]} by changing ${lever.label}`}>
          {busy ? 'Solving…' : 'Try fixing this'}
        </button>
      )}
    </li>
  );
}

function StatusWord({ s }: { s: 'pass' | 'fail' | 'unverified' }) {
  const icon = s === 'pass' ? '✓' : s === 'fail' ? '✕' : '○';
  return <span className={`na-st ${s}`}><span aria-hidden>{icon}</span> {s}</span>;
}

function SolvePanel(props: {
  solve: Solve | null; error: string | null; busy: boolean; couple: boolean; onCouple: (v: boolean) => void;
  onApply: () => void; onCancel: () => void;
}) {
  const { solve: s } = props;
  const [all, setAll] = useState(false);
  const titleId = useId();
  if (props.error) return <div className="na-solve" role="alert"><div className="na-bad small">Could not solve: {props.error}</div><button className="btn small" onClick={props.onCancel}>Close</button></div>;
  if (!s) return <div className="na-solve" aria-busy="true"><div className="na-skel-line" style={{ width: '60%' }} /><div className="na-skel-line" style={{ width: '85%' }} /><div className="muted small">Searching the parameter range and re-verifying every requirement…</div></div>;
  const verdicts = s.side_effects.filter((x) => x.verdict_changed);
  const moves = s.side_effects.filter((x) => !x.verdict_changed);
  const shown = all ? moves : moves.slice(0, 4);
  const coupled = s.changes.filter((c) => c.assumption);
  const hasCoupling = coupled.length > 0 || !props.couple;
  return (
    <section className="na-solve" aria-labelledby={titleId} aria-busy={props.busy}>
      <h4 id={titleId}>Solve for target: {s.label}</h4>
      <div className="na-ba">
        <div className="na-ba-cell">
          <div className="na-ba-k">{s.label}</div>
          <div className="na-ba-v"><span className="na-num">{fmtV(s.before_param, s.unit)}</span> <span aria-hidden>→</span><span className="sr-only">to</span> <b className="na-num">{fmtV(s.after_param, s.unit)}</b></div>
        </div>
        <div className="na-ba-cell">
          <div className="na-ba-k">{s.requirement_id} · {s.criterion}</div>
          <div className="na-ba-v"><span className="na-num">{fmtV(s.before_value, s.metric_unit)}</span> <span aria-hidden>→</span><span className="sr-only">to</span> <b className="na-num">{fmtV(s.after_value, s.metric_unit)}</b></div>
        </div>
        <div className="na-ba-cell">
          <div className="na-ba-k">Requirements passing</div>
          <div className="na-ba-v"><span className="na-num">{s.pass_before}</span> <span aria-hidden>→</span><span className="sr-only">to</span> <b className="na-num">{s.pass_after}</b>{s.open_goals_after !== s.open_goals_before && <span className="muted small"> · open goals {s.open_goals_before} → {s.open_goals_after}</span>}</div>
        </div>
      </div>
      <p className={`small ${s.found ? '' : 'na-bad'}`} style={{ margin: '8px 0' }}>{s.found ? '✓ ' : '✕ '}{s.message}</p>
      {s.found && s.after_param !== s.before_param && (
        <>
          <div className="na-se-head small">
            {verdicts.length > 0 ? (
              <b>
                {verdicts.filter((x) => x.better).length > 0 && <span className="na-good">{verdicts.filter((x) => x.better).map((x) => x.id).join(', ')} would pass</span>}
                {verdicts.some((x) => x.better) && verdicts.some((x) => !x.better) && ' · '}
                {verdicts.some((x) => !x.better) && <span className="na-bad">{verdicts.filter((x) => !x.better).length} requirement{verdicts.filter((x) => !x.better).length === 1 ? '' : 's'} would start failing</span>}
              </b>
            ) : <b>No requirement changes verdict</b>}
            {moves.length > 0 && <span className="muted"> · {moves.length} more move inside their limits</span>}
          </div>
          <ul className="na-se">
            {[...verdicts, ...shown].map((x) => (
              <li key={x.id} className={`na-se-row ${x.verdict_changed ? 'flip' : ''} ${x.better ? 'better' : 'worse'}`}>
                <Link to={`/systems?req=${x.id}`} className="na-se-id">{x.id}</Link>
                <span className="na-se-text" title={`${x.text}${x.metric ? ` (${x.metric})` : ''}`}>{x.text}</span>
                <span className="na-se-val na-num">{fmtV(x.before_value, x.unit)} → {fmtV(x.after_value, x.unit)}</span>
                <span className="na-se-st">
                  {x.verdict_changed ? <><StatusWord s={x.before_status} /> <span aria-hidden>→</span> <StatusWord s={x.after_status} /></> : <span className={x.better ? 'na-good' : 'na-warn'}>{x.better ? '▲ more margin' : '▼ less margin'}</span>}
                </span>
              </li>
            ))}
          </ul>
          {moves.length > 4 && <button className="btn ghost small" onClick={() => setAll(!all)} aria-expanded={all}>{all ? 'Show fewer' : `Show all ${moves.length} margin moves`}</button>}
          {hasCoupling && (
            <div className="na-assume">
              <label className="na-check">
                <input type="checkbox" checked={props.couple} onChange={(e) => props.onCouple(e.target.checked)} disabled={props.busy} />
                <span>Include coupled effects the labs do not model <span className="chip warn small-chip">assumption</span></span>
              </label>
              {coupled.map((c) => (
                <div key={c.label} className="small" style={{ marginTop: 4 }}>
                  <b>{c.label}</b>: <span className="na-num">{fmtV(c.before, c.unit)} → {fmtV(c.after, c.unit)}</span>
                  <div className="muted">{c.assumption}</div>
                </div>
              ))}
              {!props.couple && <div className="small muted" style={{ marginTop: 4 }}>Coupling off: only the solved parameter changes, exactly as the labs model it.</div>}
            </div>
          )}
        </>
      )}
      <div className="row" style={{ marginTop: 10 }}>
        {s.found && s.after_param !== s.before_param && <button className="btn primary small" onClick={props.onApply} disabled={props.busy}>Apply {s.changes.length > 1 ? `${s.changes.length} changes` : 'change'}</button>}
        <button className="btn small" onClick={props.onCancel}>{s.found && s.after_param !== s.before_param ? 'Cancel' : 'Close'}</button>
        <span className="muted small">{s.runs} {s.module} runs + 1 full verification</span>
      </div>
    </section>
  );
}

function ActionCard({ a, project, compact, threshold, onApplied }: { a: Action; project: Project; compact?: boolean; threshold: number; onApplied: (x: Applied) => void }) {
  const [open, setOpen] = useState(false);
  const [more, setMore] = useState(!compact);
  const [lazy, setLazy] = useState<LeversOut | null>(() => (a.metric ? leverCache.get(project)?.get(a.metric) ?? null : null));
  const [lazyErr, setLazyErr] = useState<string | null>(null);
  const [solving, setSolving] = useState<{ param: string; couple: boolean } | null>(null);
  const [solve, setSolve] = useState<Solve | null>(null);
  const [solveErr, setSolveErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const headId = useId();
  const leverId = useId();
  const whyRef = useRef<HTMLParagraphElement>(null);
  const [clipped, setClipped] = useState(false);
  useLayoutEffect(() => {
    const el = whyRef.current;
    if (!el || more) return;
    const check = () => setClipped(el.scrollHeight > el.clientHeight + 2);
    check();
    const ro = new ResizeObserver(check);
    ro.observe(el);
    return () => ro.disconnect();
  }, [more, a.why]);

  const levers = a.levers_status === 'lazy' ? lazy?.levers ?? [] : a.levers;
  const canLever = a.levers_status !== 'n/a';
  const canSolve = !!a.metric && ['failing', 'goal', 'marginal'].includes(a.kind);

  useEffect(() => {
    if (!open || a.levers_status !== 'lazy' || lazy || !a.metric) return;
    let live = true;
    callInWorker<LeversOut>('systems.next_levers', { project, metric: a.metric, requirement_id: a.requirements[0] }).then((r) => {
      if (!live) return;
      if (r.data) {
        let m = leverCache.get(project);
        if (!m) leverCache.set(project, (m = new Map()));
        m.set(a.metric!, r.data);
        setLazy(r.data);
      } else setLazyErr(r.error);
    });
    return () => { live = false; };
  }, [open, a.levers_status, a.metric, a.requirements, lazy, project]);

  const runSolve = (param: string, couple: boolean) => {
    setSolving({ param, couple });
    setBusy(true);
    setSolveErr(null);
    if (!solve || solve.param !== param) setSolve(null);
    callInWorker<Solve>('systems.solve_lever', {
      project, requirement_id: a.requirements[0], param, couple,
      ...(a.kind === 'marginal' ? { margin: threshold } : {}),
    }).then((r) => {
      setBusy(false);
      if (r.data) setSolve(r.data);
      else setSolveErr(r.error);
    });
  };
  const cancel = () => { setSolving(null); setSolve(null); setSolveErr(null); };
  const apply = () => {
    if (!solve) return;
    const changes = solve.changes;
    applyChanges(changes, 'after');
    onApplied({ solve, changes });
    cancel();
  };

  const sev = SEVERITY[a.severity];
  return (
    <li className={`na-card sev-${a.severity} ${compact ? 'compact' : ''}`} aria-labelledby={headId}>
      <div className="na-card-top">
        <SeverityPill s={a.severity} />
        <span className="na-kind">{KIND[a.kind]}</span>
        {!compact && <span className="na-score muted small" title={a.score_note}>score {a.score.toFixed(1)}</span>}
      </div>
      <h3 id={headId} className="na-title"><Title text={a.title} /></h3>
      <p ref={whyRef} className={`na-why ${more ? '' : 'clamp'}`}>{a.why}</p>
      {compact && (clipped || more) && (
        <button className="btn ghost small na-more" onClick={() => setMore(!more)} aria-expanded={more}>{more ? 'Less' : 'More'}</button>
      )}
      {a.value != null && a.criterion && (
        <div className="na-vt">
          <span className="na-vt-text"><span className="na-num">{fmtV(a.value, a.unit)}</span> <span aria-hidden>→</span><span className="sr-only">target</span> <span className="na-num">{a.criterion}</span></span>
          <MarginBar margin={a.margin} width={compact ? 80 : 110} />
        </div>
      )}
      {!compact && a.note && <div className="na-note small muted" title="How the lab computed this metric">{a.note}</div>}
      <div className="na-foot">
        <GoLink where={a.where} compact={compact} />
        {canLever && (
          <button className="btn ghost small na-lever-btn" aria-expanded={open} aria-controls={leverId} onClick={() => setOpen(!open)}>
            <span aria-hidden>{open ? '▾' : '▸'}</span> Levers{a.levers_status === 'measured' ? ` (${a.levers.length})` : ''}
          </button>
        )}
        <span className="sr-only">Severity {sev.word}</span>
      </div>
      {open && (
        <div id={leverId} className="na-levers">
          {a.levers_status === 'lazy' && !lazy && !lazyErr && (
            <div className="muted small" aria-busy="true"><span className="na-spin" aria-hidden /> Measuring levers — this lab runs a finite-element or layout model once per candidate parameter…</div>
          )}
          {lazyErr && <div className="na-bad small">Could not measure levers: {lazyErr}</div>}
          {(a.levers_status === 'none' || (a.levers_status === 'lazy' && lazy && levers.length === 0)) && (
            <div className="muted small">No numeric design parameter on this lab's shortlist moves this metric measurably — it changes with the inputs in {a.where.label} (evidence, schedule, test plan or a design choice such as a material or lens type).</div>
          )}
          {levers.length > 0 && (
            <>
              <div className="small muted na-lever-intro">Each parameter was nudged by 5 % and the {a.module} model re-run; strongest first.</div>
              <ul className="na-lever-list">
                {levers.map((l) => (
                  <LeverRow key={l.param} lever={l} action={a} busy={busy && solving?.param === l.param} onTry={canSolve ? () => runSolve(l.param, true) : undefined} />
                ))}
              </ul>
            </>
          )}
          {solving && (
            <SolvePanel solve={solve} error={solveErr} busy={busy} couple={solving.couple} onCouple={(c) => runSolve(solving.param, c)} onApply={apply} onCancel={cancel} />
          )}
        </div>
      )}
    </li>
  );
}

function Skeleton({ rows }: { rows: number }) {
  return (
    <div className="na-skel" aria-busy="true" aria-label="Working out what to do next">
      <div className="na-skel-line" style={{ width: '40%', height: 18 }} />
      <div className="na-skel-line" style={{ width: '100%', height: 10 }} />
      {Array.from({ length: rows }).map((_, i) => (
        <div key={i} className="na-skel-card">
          <div className="na-skel-line" style={{ width: '22%' }} />
          <div className="na-skel-line" style={{ width: '70%', height: 14 }} />
          <div className="na-skel-line" style={{ width: '90%' }} />
        </div>
      ))}
    </div>
  );
}

// ------------------------------------------------------------------ main

export function NextActions({ limit = 8, compact = false }: { limit?: number; compact?: boolean }) {
  const project = useProject((s) => s.project);
  const run = useNextActions(project);
  const [filter, setFilter] = useState<Filter>('all');
  const [showAll, setShowAll] = useState(false);
  const [applied, setApplied] = useState<Applied | null>(null);
  const [undone, setUndone] = useState(false);
  const bannerRef = useRef<HTMLDivElement>(null);

  const out = run.data;
  const counts = useMemo(() => {
    const c: Record<Filter, number> = { all: 0, failing: 0, goals: 0, marginal: 0, gaps: 0 };
    for (const a of out?.actions ?? []) {
      c.all++;
      for (const f of FILTERS) if (f.kinds && f.kinds.includes(a.kind)) c[f.id]++;
    }
    return c;
  }, [out]);
  const list = useMemo(() => {
    const f = FILTERS.find((x) => x.id === filter)!;
    return (out?.actions ?? []).filter((a) => !f.kinds || f.kinds.includes(a.kind));
  }, [out, filter]);

  useEffect(() => { if (applied) bannerRef.current?.focus(); }, [applied]);

  if (!out) {
    if (run.error) return <div className="na-bad small" role="alert">Could not rank the next actions: {run.error}</div>;
    return <Skeleton rows={compact ? 3 : 4} />;
  }
  const visible = showAll ? list : list.slice(0, limit);
  const undo = () => {
    if (!applied) return;
    applyChanges(applied.changes, 'before');
    setUndone(true);
  };
  return (
    <div className={`na ${compact ? 'compact' : ''}`} aria-busy={run.running}>
      <Readiness out={out} compact={compact} />
      {applied && (
        <div className="na-applied" role="status" tabIndex={-1} ref={bannerRef}>
          <span>
            {undone ? 'Undone: ' : 'Applied: '}
            <b>{applied.solve.label}</b> {fmtV(applied.solve.before_param, applied.solve.unit)} → {fmtV(applied.solve.after_param, applied.solve.unit)}
            {applied.changes.length > 1 && <> and {applied.changes.length - 1} coupled change{applied.changes.length > 2 ? 's' : ''}</>}
            {undone ? ' — back to the previous design.' : ` — ${applied.solve.requirement_id} ${applied.solve.after_margin != null && applied.solve.after_margin >= 0 ? 'now passes' : 'moves'}; the engine re-verifies everything.`}
          </span>
          <span className="row">
            {!undone && <button className="btn small" onClick={undo}>Undo</button>}
            <button className="btn ghost small" onClick={() => { setApplied(null); setUndone(false); }} aria-label="Dismiss">✕</button>
          </span>
        </div>
      )}
      <div className="na-filters" role="group" aria-label="Filter actions">
        {FILTERS.map((f) => (
          <button key={f.id} className={`na-filter ${filter === f.id ? 'active' : ''}`} aria-pressed={filter === f.id} onClick={() => { setFilter(f.id); setShowAll(false); }}>
            {f.label} <span className="na-count">{counts[f.id]}</span>
          </button>
        ))}
        {run.running && <span className="muted small na-updating"><span className="na-spin" aria-hidden /> updating…</span>}
      </div>
      {visible.length === 0 ? (
        <div className="na-empty small">✓ {EMPTY[filter]}</div>
      ) : (
        <ol className="na-list">
          {visible.map((a) => (
            <ActionCard key={a.id} a={a} project={project} compact={compact} threshold={out.summary.marginal_threshold} onApplied={(x) => { setApplied(x); setUndone(false); }} />
          ))}
        </ol>
      )}
      <div className="na-tail small">
        {list.length > limit && (
          <button className="btn ghost small" onClick={() => setShowAll(!showAll)} aria-expanded={showAll}>
            {showAll ? 'Show the top ' + limit : `Show all ${list.length}`}
          </button>
        )}
        {compact && <Link to="/systems?tab=overview">Full list with levers in Systems Studio →</Link>}
        {!compact && (
          <details className="na-how">
            <summary>How is this ranked?</summary>
            <p>
              Every action gets a score: a base for its kind — failing requirement 100, unverified 90, open goal 80, unmitigated high risk 70,
              requirement error 60, marginal pass 50, stage gate 40, no verification 30, traceability gap 20, wording 10 — plus up to 10 points
              for importance × urgency. Importance rises for safety and regulatory requirements and for requirements that serve a
              must-have need; urgency is how far a requirement misses its target, or how much of the {Math.round(out.summary.marginal_threshold * 100)} % margin band a
              pass has used up. So the kind always decides first, and the worst miss leads within a kind.
            </p>
            <p>
              Levers are measured, not looked up: each candidate design parameter is nudged by 5 % and only the lab model that computes the
              metric is re-run. "Try fixing this" searches one parameter for the value that meets the target with a 2 % margin
              (10 % for marginal passes), then re-verifies every requirement so you see the side effects before applying anything.
              Engine time for this list: {run.ms.toFixed(0)} ms.
            </p>
          </details>
        )}
      </div>
    </div>
  );
}
