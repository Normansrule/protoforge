// Plain-words help for the current lab: the question it answers, what you
// change, what you read and what good looks like, three steps to try, who it
// is for and whether it works on your own product. Rendered by LabPage in
// every lab header (no props; the lab comes from the route). In Simple mode it
// opens by itself the first time a lab is visited.
import { useEffect, useId, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { Link, useLocation } from 'react-router-dom';
import { Icon } from '../../components/Icon';
import { fmtNum } from '../../components/Plot';
import { useEvaluation, useMetricMap, useProject } from '../../state/project';
import { usePrefs } from '../prefs';
import { labFromPath, labIcon, labTitle, tabLabel, tabRoute } from '../data/links';
import { roleById } from '../data/roles';
import { LAB_HELP, SCOPE_LABEL, type LabHelpData } from './labHelp';
import { Term } from './Term';
import './help.css';

function HelpGlyph({ size = 16 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <circle cx="12" cy="12" r="9.5" />
      <path d="M9.4 9.3a2.7 2.7 0 015.2.9c0 1.8-2.6 2.3-2.6 3.9M12 17.2h.01" />
    </svg>
  );
}

export function LabHelp() {
  const { pathname } = useLocation();
  const lab = labFromPath(pathname);
  const data = lab ? LAB_HELP[lab] : undefined;
  const mode = usePrefs((s) => s.mode);
  const seen = usePrefs((s) => (lab ? s.helpSeen.includes(lab) : true));
  const markSeen = usePrefs((s) => s.markHelpSeen);
  const [open, setOpen] = useState(false);
  const [focusOnOpen, setFocusOnOpen] = useState(false);
  const btn = useRef<HTMLButtonElement>(null);
  const panelId = useId();

  // Simple mode: open once per lab, without stealing keyboard focus.
  useEffect(() => {
    if (!lab || !data) return;
    if (mode === 'simple' && !seen) {
      setFocusOnOpen(false);
      setOpen(true);
    }
    if (!seen) markSeen(lab);
  }, [lab, data, mode, seen, markSeen]);

  if (!lab || !data) return null;
  return (
    <>
      <button
        ref={btn}
        type="button"
        className={`btn ux-help-btn ${open ? 'on' : ''}`}
        aria-expanded={open}
        aria-controls={panelId}
        onClick={() => {
          setFocusOnOpen(true);
          setOpen((o) => !o);
        }}
      >
        <HelpGlyph /> Help
      </button>
      {open &&
        createPortal(
          <HelpPanel
            id={panelId}
            lab={lab}
            data={data}
            focus={focusOnOpen}
            onClose={() => {
              setOpen(false);
              btn.current?.focus();
            }}
          />,
          document.body,
        )}
    </>
  );
}

function HelpPanel({ id, lab, data, focus, onClose }: { id: string; lab: string; data: LabHelpData; focus: boolean; onClose: () => void }) {
  const head = useRef<HTMLHeadingElement>(null);
  const metrics = useMetricMap();
  const ev = useEvaluation();
  const project = useProject((s) => s.project);
  const headId = `${id}-h`;
  const mode = usePrefs((s) => s.mode);

  useEffect(() => {
    if (focus) head.current?.focus();
  }, [focus]);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [onClose]);

  const status = (metric?: string) => {
    if (!metric) return null;
    const req = project.requirements.find((r) => r.verification.metric === metric);
    const res = req ? ev.data?.results.find((r) => r.id === req.id) : undefined;
    // a failing stretch goal reads as an amber open goal, not a red fail
    return res ? { id: req!.id, status: res.status === 'fail' && req!.kind === 'goal' ? ('goal' as const) : res.status } : null;
  };

  return (
    <aside id={id} className="ux-help-panel" role="dialog" aria-modal="false" aria-labelledby={headId}>
      <div className="ux-help-top">
        <span className="ux-help-icon"><Icon name={labIcon(lab)} size={20} /></span>
        <div style={{ minWidth: 0 }}>
          <div className="ux-help-kicker">How to use</div>
          <h2 id={headId} ref={head} tabIndex={-1}>{labTitle(lab)}</h2>
        </div>
        <button type="button" className="btn ghost small ux-help-close" onClick={onClose} aria-label="Close help">✕</button>
      </div>
      <div className="ux-help-body">
        <span className={`ux-scope ${data.scope}`}>{SCOPE_LABEL[data.scope]}</span>
        <p className="ux-help-q">{data.question}</p>
        <p className="muted small" style={{ marginTop: 0 }}>{data.scopeNote}</p>

        <h3>What you change</h3>
        <ul className="ux-help-list">
          {data.inputs.map((x) => <li key={x}>{x}</li>)}
        </ul>

        <h3>What you read</h3>
        <ul className="ux-help-out">
          {data.outputs.map((o) => {
            const m = o.metric ? metrics.get(o.metric) : undefined;
            const st = status(o.metric);
            return (
              <li key={o.name}>
                <div className="ux-help-out-row">
                  <span className="ux-help-out-name">{o.name}</span>
                  {m && (
                    <span className={`chip ${st?.status === 'pass' ? 'pass' : st?.status === 'fail' ? 'fail' : st?.status === 'goal' ? 'warn' : ''}`} title={st ? `${st.id}: ${st.status === 'goal' ? 'open goal' : st.status}` : 'live value'}>
                      {st?.status === 'pass' ? '✓ ' : st?.status === 'fail' ? '✕ ' : st?.status === 'goal' ? '! ' : ''}
                      {fmtNum(m.value)} {m.unit}
                    </span>
                  )}
                </div>
                <div className="ux-help-out-good">Good: {o.good}</div>
              </li>
            );
          })}
        </ul>

        <h3>Try this in 3 steps</h3>
        <ol className="ux-help-steps">
          {data.try3.map((s, i) => (
            <li key={i}>
              <span className="ux-step-n" aria-hidden>{i + 1}</span>
              <span>
                {s.text}
                {s.tab && (
                  <>
                    {' '}
                    <Link to={tabRoute(lab, s.tab)} className="ux-help-tab">{tabLabel(lab, s.tab) ?? s.tab} →</Link>
                  </>
                )}
              </span>
            </li>
          ))}
        </ol>

        <h3>Who it is for</h3>
        <div className="row" style={{ gap: 6 }}>
          {data.roles.map((r) => {
            const role = roleById(r);
            return role ? <Link key={r} to={`/start?role=${r}`} className="chip info" onClick={onClose}>{role.short}</Link> : null;
          })}
        </div>

        {data.terms.length > 0 && (
          <>
            <h3>Words you will meet</h3>
            <div className="ux-help-terms">
              {data.terms.map((t) => <Term key={t} k={t} />)}
            </div>
          </>
        )}
      </div>
      <div className="ux-help-foot">
        <Link to={`/docs/labs-${lab}`} className="btn small" onClick={onClose}><Icon name="docs" size={14} /> Full lab guide</Link>
        {mode === 'simple' && <span className="muted small">Opens by itself once per lab in Simple mode.</span>}
      </div>
    </aside>
  );
}
