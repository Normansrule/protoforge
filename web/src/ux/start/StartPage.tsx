// Start here: pick your engineering discipline and get a guided path through
// real lab tabs, a five-minute first win, your labs, key terms, the tools you
// graduate to, and an "I want to…" finder for everyday jobs.
import { useEffect, useMemo, useRef, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { Icon } from '../../components/Icon';
import { fmtNum } from '../../components/Plot';
import { REPOS } from '../../pages/learn/repos';
import { DOCS } from '../../pages/docs/sources';
import { IRIS, useEvaluation, useMetricMap, useProject } from '../../state/project';
import { usePrefs } from '../prefs';
import { ROLES, roleById, type Role, type PathStep } from '../data/roles';
import { TASKS } from '../data/tasks';
import { labIcon, labTitle, tabLabel, tabRoute } from '../data/links';
import { scoreFields } from '../palette/fuzzy';
import { ModeSwitch } from '../nav/Chrome';
import { Term } from '../help/Term';
import './start.css';

const ROLE_COLOR: Record<string, string> = {
  electrical: 'var(--series-1)',
  mechanical: 'var(--series-2)',
  firmware: 'var(--series-3)',
  optical: 'var(--series-4)',
  rf: 'var(--series-5)',
  controls: 'var(--series-8)',
  manufacturing: 'var(--serious)',
  test: 'var(--series-6)',
  systems: 'var(--accent-2)',
  quantum: 'var(--series-7)',
  student: 'var(--warn)',
};

function reducedMotion() {
  return typeof window !== 'undefined' && !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
}

export default function StartPage() {
  const [params, setParams] = useSearchParams();
  const roleId = usePrefs((s) => s.role);
  const setRole = usePrefs((s) => s.setRole);
  const panel = useRef<HTMLDivElement>(null);
  const urlRole = params.get('role');

  // ?role=rf deep links choose the role.
  useEffect(() => {
    if (urlRole && roleById(urlRole) && urlRole !== roleId) setRole(urlRole);
  }, [urlRole, roleId, setRole]);

  const role = roleById(roleId);
  const choose = (id: string) => {
    setRole(id);
    setParams((p) => {
      const n = new URLSearchParams(p);
      n.set('role', id);
      return n;
    }, { replace: true });
    requestAnimationFrame(() => panel.current?.scrollIntoView({ behavior: reducedMotion() ? 'auto' : 'smooth', block: 'start' }));
  };

  return (
    <div className="st">
      <header className="st-hero">
        <h1>Where do you want to start?</h1>
        <p className="st-lede">
          Protoforge takes a product from an idea to factory files in your browser. Tell it what kind of engineer you are and it shows the labs you
          need, the order to use them in, and what each number means — using the Iris smart glasses as a worked example until you describe your own product.
        </p>
      </header>

      <section aria-labelledby="st-pick-h" className="st-pick">
        <h2 id="st-pick-h">I work on…</h2>
        <div className="st-roles" role="radiogroup" aria-label="Your discipline">
          {ROLES.map((r) => (
            <button
              key={r.id}
              type="button"
              role="radio"
              aria-checked={r.id === roleId}
              className={`st-role ${r.id === roleId ? 'on' : ''}`}
              style={{ ['--role' as string]: ROLE_COLOR[r.id] }}
              onClick={() => choose(r.id)}
            >
              <span className="st-role-icon"><Icon name={r.icon} size={22} /></span>
              <span className="st-role-name">{r.title}</span>
              <span className="st-role-labs">{r.labs.map(labTitle).slice(0, 3).join(', ')}{r.labs.length > 3 ? ` +${r.labs.length - 3}` : ''}</span>
            </button>
          ))}
        </div>
      </section>

      <div ref={panel} className="st-panel-anchor">
        {role ? <RolePanel key={role.id} role={role} /> : <NoRole />}
      </div>

      <section aria-labelledby="st-how-h" className="st-how-wrap">
        <h2 id="st-how-h">How Protoforge works</h2>
        <HowItWorks />
      </section>

      <TaskFinder roleId={roleId} />
    </div>
  );
}

function HowItWorks() {
  const steps = [
    { icon: 'learn', title: 'Describe', text: 'Say what the product must do. The new-product wizard or the Systems Studio turns it into requirements bound to numbers.', links: [{ to: '/new', label: 'New product' }, { to: '/systems', label: 'Systems Studio' }] },
    { icon: 'cad', title: 'Design & simulate', text: 'Change the design in the labs. Every edit re-checks every requirement, live.', links: [{ to: '/', label: 'All 23 labs' }] },
    { icon: 'build', title: 'Build & ship', text: 'Pick processes and vendors, get files, and hand off to real tools.', links: [{ to: '/build', label: 'Build Guide' }, { to: '/mfg', label: 'Manufacturing Hub' }, { to: '/toolchain', label: 'Toolchain' }] },
  ];
  return (
    <ol className="st-how" aria-label="How Protoforge works">
      {steps.map((s, i) => (
        <li key={s.title} className="st-how-step">
          <div className="st-how-top">
            <span className="st-how-icon"><Icon name={s.icon} size={18} /></span>
            <b>{s.title}</b>
            {i < steps.length - 1 && <span className="st-how-arrow" aria-hidden />}
          </div>
          <p>{s.text}</p>
          <div className="st-how-links">
            {s.links.map((l) => <Link key={l.to} to={l.to}>{l.label}</Link>)}
          </div>
        </li>
      ))}
    </ol>
  );
}

function NoRole() {
  return (
    <div className="st-norole">
      <Icon name="arrow" size={18} />
      <span>Pick a discipline above to get your path, a five-minute first win and the labs to pin in your sidebar. You can change it any time.</span>
    </div>
  );
}

function RolePanel({ role }: { role: Role }) {
  const mode = usePrefs((s) => s.mode);
  const total = role.path.reduce((a, s) => a + s.min, 0);
  return (
    <section className="st-panel" style={{ ['--role' as string]: ROLE_COLOR[role.id] }} aria-labelledby="st-panel-h">
      <div className="st-panel-head">
        <span className="st-panel-icon"><Icon name={role.icon} size={26} /></span>
        <div>
          <h2 id="st-panel-h">{role.title}</h2>
          <p>{role.pitch}</p>
        </div>
      </div>

      <div className="st-panel-grid">
        <div className="st-path">
          <div className="st-sec-head">
            <h3>Your path</h3>
            <span className="muted small">{role.path.length} steps · about {total} minutes</span>
          </div>
          <ol className="st-steps">
            {role.path.map((s, i) => <PathRow key={i} step={s} n={i + 1} />)}
          </ol>
        </div>
        <Quest role={role} />
      </div>

      <div className="st-extras">
        <div>
          <h3>Your labs</h3>
          <p className="muted small">{mode === 'simple' ? 'Pinned first in the sidebar.' : 'Pinned first in the sidebar in Simple mode.'}</p>
          <div className="st-labs">
            {role.labs.map((l) => (
              <Link key={l} to={tabRoute(l)} className="st-lab"><Icon name={labIcon(l)} size={16} /> {labTitle(l)}</Link>
            ))}
          </div>
          {mode === 'expert' && <div style={{ marginTop: 10 }}><ModeSwitch /></div>}
        </div>
        <div>
          <h3>Words you will meet</h3>
          <p className="muted small">Hover, focus or tap for the meaning.</p>
          <ul className="st-terms">
            {role.terms.map((t) => <li key={t}><Term k={t} /></li>)}
          </ul>
        </div>
        <Graduate role={role} />
        <Guides role={role} />
      </div>
    </section>
  );
}

function useReqStatus() {
  const ev = useEvaluation();
  const project = useProject((s) => s.project);
  const metrics = useMetricMap();
  return { ev, project, metrics };
}

function PathRow({ step, n }: { step: PathStep; n: number }) {
  const { ev, project, metrics } = useReqStatus();
  // Path steps name Iris requirement ids. Match the loaded project by the metric that
  // requirement is bound to, so another product's R-001 (which may be something else
  // entirely) never shows up as the Iris step's live value.
  const irisMetric = step.req ? IRIS.requirements.find((r) => r.id === step.req)?.verification.metric : undefined;
  const req = step.req
    ? project.meta.id === IRIS.meta.id
      ? project.requirements.find((r) => r.id === step.req)
      : irisMetric
        ? project.requirements.find((r) => r.verification.metric === irisMetric)
        : undefined
    : undefined;
  const res = req ? ev.data?.results.find((r) => r.id === req.id) : undefined;
  const metric = step.metric ? metrics.get(step.metric) : undefined;
  const where = tabLabel(step.lab, step.tab);
  let chip: React.ReactNode = null;
  if (step.req && req && res) {
    const tone = res.status === 'pass' ? 'pass' : res.status === 'fail' ? (req.kind === 'goal' ? 'warn' : 'fail') : '';
    chip = (
      <Link to={`/systems?req=${req.id}`} className={`chip ${tone}`} title={req.text}>
        {res.status === 'pass' ? '✓' : res.status === 'fail' ? (req.kind === 'goal' ? '!' : '✕') : '○'} {req.id} · {res.value != null ? `${fmtNum(res.value)} ${res.unit}` : '—'} ({res.criterion})
      </Link>
    );
  } else if (step.req) {
    chip = <span className="chip" title="This requirement belongs to the Iris example; reset to Iris to see it">{step.req} · Iris example</span>;
  } else if (step.metric) {
    chip = metric ? (
      <span className="chip info" title={step.metric}>{metric.label}: {fmtNum(metric.value)} {metric.unit}</span>
    ) : (
      <span className="chip" title={step.metric}>{step.metric.startsWith('quantum.') ? 'appears once the Q-Link template is loaded' : 'no value for this project'}</span>
    );
  }
  return (
    <li className="st-step">
      <span className="st-step-n" aria-hidden>{n}</span>
      <div className="st-step-body">
        <Link to={tabRoute(step.lab, step.tab)} className="st-step-where">
          <Icon name={labIcon(step.lab)} size={15} /> {labTitle(step.lab)}{where ? ` › ${where}` : ''}
        </Link>
        <p>{step.action}</p>
        <div className="st-step-meta">
          {chip}
          <span className="st-min">{step.min} min</span>
        </div>
      </div>
    </li>
  );
}

function Quest({ role }: { role: Role }) {
  const done = usePrefs((s) => s.quests[role.id]) ?? [];
  const toggle = usePrefs((s) => s.toggleQuest);
  const resetQuest = usePrefs((s) => s.resetQuest);
  const count = role.quest.steps.filter((s) => done.includes(s.id)).length;
  const all = count === role.quest.steps.length;
  const frac = count / role.quest.steps.length;
  const R = 20;
  const C = 2 * Math.PI * R;
  return (
    <aside className={`st-quest ${all ? 'done' : ''}`} aria-labelledby="st-quest-h">
      <div className="st-quest-head">
        <svg width="52" height="52" viewBox="0 0 52 52" className="st-ring" role="img" aria-label={`${count} of ${role.quest.steps.length} steps done`}>
          <circle cx="26" cy="26" r={R} className="st-ring-bg" />
          <circle cx="26" cy="26" r={R} className="st-ring-fg" strokeDasharray={C} strokeDashoffset={C * (1 - frac)} transform="rotate(-90 26 26)" />
          <text x="26" y="30" textAnchor="middle">{all ? '✓' : `${count}/${role.quest.steps.length}`}</text>
        </svg>
        <div>
          <div className="st-quest-kicker">Five-minute first win</div>
          <h3 id="st-quest-h">{role.quest.title}</h3>
        </div>
      </div>
      <p className="st-quest-goal">{role.quest.goal}</p>
      <ul className="st-quest-list">
        {role.quest.steps.map((s) => {
          const id = `q-${role.id}-${s.id}`;
          const checked = done.includes(s.id);
          return (
            <li key={s.id} className={checked ? 'checked' : ''}>
              <input id={id} type="checkbox" checked={checked} onChange={() => toggle(role.id, s.id)} />
              <label htmlFor={id}>{s.text}</label>
              {s.lab && <Link to={tabRoute(s.lab, s.tab)} className="st-quest-go" aria-label={`Open ${labTitle(s.lab)}${s.tab ? `, ${tabLabel(s.lab, s.tab)}` : ''}`}>Open</Link>}
            </li>
          );
        })}
      </ul>
      <div className="st-quest-foot" aria-live="polite">
        {all ? (
          <>
            <span>Done — that is your first win. Carry on down your path.</span>
            <button type="button" className="btn small ghost" onClick={() => resetQuest(role.id)}>Start over</button>
          </>
        ) : (
          <span className="muted small">Ticks are saved in this browser. Reset to Iris from the top bar to undo any design change.</span>
        )}
      </div>
    </aside>
  );
}

function Graduate({ role }: { role: Role }) {
  const items = role.repoGroups.flatMap((name) => REPOS.find((g) => g.discipline === name)?.items ?? []).slice(0, 6);
  if (items.length === 0) return null;
  return (
    <div>
      <h3>Tools you will graduate to</h3>
      <p className="muted small">Professional and open-source tools that take over where the labs stop.</p>
      <ul className="st-tools">
        {items.map((r) => {
          const href = r.url ?? (r.repo ? `https://github.com/${r.repo}` : undefined);
          return (
            <li key={r.name}>
              {href ? <a href={href} target="_blank" rel="noreferrer">{r.name}</a> : <b>{r.name}</b>}
              <span>{r.learn}</span>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

function Guides({ role }: { role: Role }) {
  const docs = role.docs.map((s) => DOCS.find((d) => d.slug === s)).filter((d): d is (typeof DOCS)[number] => !!d);
  if (docs.length === 0) return null;
  return (
    <div>
      <h3>Guides to read</h3>
      <p className="muted small">The maths, the limits and the references behind each lab.</p>
      <ul className="st-guides">
        {docs.map((d) => (
          <li key={d.slug}><Link to={`/docs/${d.slug}`}><Icon name="docs" size={15} /> {d.title}</Link></li>
        ))}
      </ul>
    </div>
  );
}

function TaskFinder({ roleId }: { roleId: string | null }) {
  const [q, setQ] = useState('');
  const [filter, setFilter] = useState<string>('all');
  const results = useMemo(() => {
    const base = TASKS.filter((t) => filter === 'all' || t.role === filter);
    if (!q.trim()) return base.map((t) => ({ t, s: 0 }));
    return base
      .map((t) => ({ t, s: scoreFields(q, [{ text: t.title, w: 1 }, { text: t.keywords, w: 0.7 }, { text: t.how, w: 0.5 }, { text: labTitle(t.lab), w: 0.5 }]) }))
      .filter((x) => x.s > 0)
      .sort((a, b) => b.s - a.s);
  }, [q, filter]);
  const grouped = useMemo(() => {
    if (q.trim()) return [{ role: undefined as Role | undefined, items: results.map((r) => r.t) }];
    const order = [...ROLES].sort((a, b) => (a.id === roleId ? -1 : b.id === roleId ? 1 : 0));
    return order.map((r) => ({ role: r, items: results.filter((x) => x.t.role === r.id).map((x) => x.t) })).filter((g) => g.items.length > 0);
  }, [results, q, roleId]);

  return (
    <section className="st-finder" aria-labelledby="st-finder-h">
      <div className="st-finder-head">
        <h2 id="st-finder-h">I want to…</h2>
        <p className="muted">{TASKS.length} everyday engineering jobs, each mapped to the lab tab that does it.</p>
      </div>
      <div className="st-finder-search">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" aria-hidden>
          <circle cx="11" cy="11" r="7" />
          <path d="M20 20l-3.5-3.5" />
        </svg>
        <input type="search" value={q} onChange={(e) => setQ(e.target.value)} placeholder="size a battery, match an antenna, tune a PID loop…" aria-label="Search everyday engineering jobs" />
      </div>
      <div className="st-filter" role="group" aria-label="Filter by discipline">
        <button type="button" aria-pressed={filter === 'all'} className={filter === 'all' ? 'on' : ''} onClick={() => setFilter('all')}>All</button>
        {ROLES.map((r) => (
          <button key={r.id} type="button" aria-pressed={filter === r.id} className={filter === r.id ? 'on' : ''} style={{ ['--role' as string]: ROLE_COLOR[r.id] }} onClick={() => setFilter(r.id)}>
            {r.short}
          </button>
        ))}
      </div>
      <div aria-live="polite" className="sr-only">{results.length} jobs shown</div>
      {results.length === 0 && <p className="st-finder-empty">No job matches “{q}”. Try fewer words, or press Ctrl K to search labs, requirements and the glossary too.</p>}
      {grouped.map((g, gi) => (
        <div key={g.role?.id ?? gi} className="st-task-group">
          {g.role && (
            <h3 style={{ ['--role' as string]: ROLE_COLOR[g.role.id] }}>
              <span className="st-dot" aria-hidden /> {g.role.title}
            </h3>
          )}
          <ul className="st-tasks">
            {g.items.map((t) => (
              <li key={t.id}>
                <Link to={tabRoute(t.lab, t.tab)} className="st-task">
                  <span className="st-task-title">{t.title.charAt(0).toUpperCase() + t.title.slice(1)}</span>
                  <span className="st-task-how">{t.how}</span>
                  <span className="st-task-where"><Icon name={labIcon(t.lab)} size={13} /> {labTitle(t.lab)}{t.tab ? ` › ${tabLabel(t.lab, t.tab) ?? t.tab}` : ''}</span>
                </Link>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </section>
  );
}
