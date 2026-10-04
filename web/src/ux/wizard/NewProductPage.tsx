// New-product wizard: describe a product in plain words and get a starter
// project whose requirements are bound only to metrics the engine really
// computes from that project's own data (systems.wizard in Rust). The live
// preview evaluates the generated project in the Web Worker.
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { Chip, DownloadButton, Explainer, Slider } from '../../components/ui';
import { Icon } from '../../components/Icon';
import { fmtNum } from '../../components/Plot';
import { useElementWidth } from '../../components/useElementWidth';
import { useCompute } from '../../engine/hooks';
import { callInWorker } from '../../engine/worker';
import type { Evaluation, Project, ReqResult } from '../../engine/types';
import { LABS } from '../../labs/registry';
import { useProject } from '../../state/project';
import { Modal, useReducedMotion } from '../../pages/systems/shared';
import type { Template } from '../../pages/systems/types';
import { ArchDiagram, ifaceClass } from './ArchDiagram';
import { KindArt } from './KindArt';
import type { Answers, CapOption, KindOption, TargetOption, WizardOptions, WizardOut } from './types';
import '../../pages/systems/studio.css';
import './wizard.css';

const DRAFT_KEY = 'protoforge.wizard.v1';
const STEPS = ['Describe', 'Kind', 'Capabilities', 'Targets', 'Review'] as const;

const IDEAS: { kind: string; idea: string; name: string }[] = [
  { kind: 'wearable', name: 'Posture Clip', idea: 'A clip-on sensor that buzzes when you slouch' },
  { kind: 'iot_sensor', name: 'Leaf', idea: 'A plant-soil monitor that reports moisture to your phone for a year' },
  { kind: 'robot', name: 'Rover', idea: 'A two-wheeled desk robot that follows a line and says hello' },
  { kind: 'desk_instrument', name: 'Breathe', idea: 'An air-quality monitor with a colour screen for a classroom' },
];

function loadDraft(): { answers: Answers; step: number } | null {
  try {
    const raw = localStorage.getItem(DRAFT_KEY);
    if (!raw) return null;
    const d = JSON.parse(raw);
    return d && d.answers && typeof d.answers.kind === 'string' ? d : null;
  } catch {
    return null;
  }
}

function saveDraft(answers: Answers, step: number) {
  try {
    localStorage.setItem(DRAFT_KEY, JSON.stringify({ answers, step }));
  } catch {
    /* storage blocked: the wizard still works for this visit */
  }
}

function freshAnswers(o: WizardOptions, kind = 'wearable'): Answers {
  const k = o.kinds.find((x) => x.id === kind) ?? o.kinds[0];
  return { name: '', idea: '', users: '', kind: k.id, capabilities: [...k.default_capabilities], targets: { ...k.defaults } };
}

/** Does a target field apply to these answers? */
function applies(t: TargetOption, a: Answers, caps: CapOption[]): boolean {
  const has = (c: string) => a.capabilities.includes(c);
  const electronics = a.kind !== 'mechanical' || a.capabilities.some((c) => caps.find((x) => x.id === c)?.electronic);
  switch (t.applies) {
    case 'battery': return has('battery') || has('coin_cell');
    case 'radio': return has('ble') || has('wifi');
    case 'electronics': return electronics;
    case 'quantum': return a.kind === 'quantum';
    case 'mechanical': return a.kind === 'mechanical';
    case 'product': return a.kind !== 'quantum' && a.kind !== 'mechanical';
    default: return true;
  }
}

function fmtT(v: number, unit: string): string {
  if (unit === 'h' && v >= 48) return v >= 8760 ? `${fmtNum(v)} h (${fmtNum(v / 8760)} y)` : `${fmtNum(v)} h (${fmtNum(v / 24)} d)`;
  if (unit === 'USD') return `${v >= 100 ? Math.round(v).toLocaleString('en-US') : fmtNum(v)}`;
  if (unit === 'units') return Math.round(v).toLocaleString('en-US');
  return fmtNum(v);
}

/** Debounced evaluation of the preview project in the Web Worker. */
function usePreviewEvaluation(project: Project | null) {
  const key = useMemo(() => (project ? JSON.stringify(project) : ''), [project]);
  const [st, set] = useState<{ ev: Evaluation | null; key: string; error: string | null; ms: number }>({ ev: null, key: '', error: null, ms: 0 });
  useEffect(() => {
    if (!project) return;
    let live = true;
    const t = setTimeout(() => {
      callInWorker<Evaluation>('systems.evaluate', project).then((r) => {
        if (live) set({ ev: r.data, key, error: r.error, ms: r.ms });
      });
    }, 450);
    return () => {
      live = false;
      clearTimeout(t);
    };
    // the key captures every change of the project
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return { ...st, fresh: st.key === key && !!key };
}

export default function NewProductPage() {
  const opts = useCompute<WizardOptions>('systems.wizard_options', {});
  if (opts.error) return <div className="callout">{opts.error}</div>;
  if (!opts.data || !opts.data.kinds) return <div className="muted" style={{ padding: 40 }}>Loading the wizard…</div>;
  return <Wizard o={opts.data} />;
}

function Wizard({ o }: { o: WizardOptions }) {
  const draft = useMemo(loadDraft, []);
  const [answers, setAnswers] = useState<Answers>(() => draft?.answers ?? freshAnswers(o));
  const [step, setStep] = useState<number>(() => Math.min(Math.max(draft?.step ?? 0, 0), STEPS.length - 1));
  const [confirm, setConfirm] = useState<{ project: Project; label: string } | null>(null);
  const project = useProject((s) => s.project);
  const setProject = useProject((s) => s.setProject);
  const navigate = useNavigate();
  const stepRef = useRef<HTMLHeadingElement>(null);

  useEffect(() => saveDraft(answers, step), [answers, step]);

  const input = useMemo(() => ({ ...answers, name: answers.name.trim() || undefined }), [answers]);
  const out = useCompute<WizardOut>('systems.wizard', input, !!answers.kind);
  const gen = out.data;
  const evalState = usePreviewEvaluation(gen?.project ?? null);

  const kind = o.kinds.find((k) => k.id === answers.kind) ?? o.kinds[0];
  const go = useCallback((s: number) => {
    setStep(s);
    requestAnimationFrame(() => stepRef.current?.focus({ preventScroll: true }));
  }, []);

  const pickKind = (k: KindOption) => {
    setAnswers((a) => (a.kind === k.id ? a : { ...a, kind: k.id, capabilities: [...k.default_capabilities], targets: { ...k.defaults } }));
  };
  const toggleCap = (c: CapOption) => {
    setAnswers((a) => {
      const on = a.capabilities.includes(c.id);
      let caps = on ? a.capabilities.filter((x) => x !== c.id) : [...a.capabilities.filter((x) => !c.excludes.includes(x)), c.id];
      if (!on && (c.id === 'printed' || c.id === 'molded')) caps = caps.filter((x) => x === c.id || (x !== 'printed' && x !== 'molded'));
      return { ...a, capabilities: caps };
    });
  };
  const setTarget = (id: string, v: number) => setAnswers((a) => ({ ...a, targets: { ...a.targets, [id]: v } }));

  const create = () => gen && setConfirm({ project: gen.project, label: gen.project.meta.name });
  const doReplace = () => {
    if (!confirm) return;
    setProject(structuredClone(confirm.project));
    try {
      localStorage.removeItem(DRAFT_KEY); // the next visit starts a fresh product
    } catch {
      /* storage blocked */
    }
    setConfirm(null);
    navigate('/systems');
  };

  const slug = gen?.project.meta.id || 'new-product';

  return (
    <div className="stack wz">
      <div className="page-head wz-hero">
        <div style={{ flex: 1, minWidth: 260 }}>
          <div className="page-kicker">New product</div>
          <h1>Describe it. Get a project the labs can check.</h1>
          <div className="page-sub">
            Five short steps turn a plain-words idea into a starter project: needs, an architecture, a typical bill of materials (BOM), sized battery and
            board, risks, a trade study and stage gates. Every requirement the wizard binds to a number is one the engine really computes for <i>your</i> product — the rest are honest tests.
          </div>
        </div>
      </div>

      <nav className="wz-steps" aria-label="Wizard steps">
        {STEPS.map((s, i) => (
          <button key={s} className={`wz-stepbtn ${i < step ? 'done' : ''}`} aria-current={i === step ? 'step' : undefined} onClick={() => go(i)}>
            <span className="num">{i < step ? '✓' : i + 1}</span>
            <span className="lbl">{s}</span>
          </button>
        ))}
      </nav>
      <div className="wz-progress" aria-hidden="true"><span style={{ width: `${((step + 1) / STEPS.length) * 100}%` }} /></div>

      <div className="wz-layout">
        <section className="card wz-step" aria-labelledby="wz-step-title">
          <h2 id="wz-step-title" ref={stepRef} tabIndex={-1} style={{ outline: 'none' }}>
            {step + 1}. {['What are you making?', 'What kind of product is it?', 'What should it do?', 'How good must it be?', 'Review and create'][step]}
          </h2>
          {step === 0 && <Describe answers={answers} setAnswers={setAnswers} o={o} onLoad={(p, label) => setConfirm({ project: p, label })} />}
          {step === 1 && <KindStep o={o} kind={kind} pick={pickKind} />}
          {step === 2 && <CapStep o={o} answers={answers} resolved={gen?.capabilities ?? answers.capabilities} toggle={toggleCap} />}
          {step === 3 && (
            <TargetStep o={o} answers={answers} set={setTarget} reset={() => setAnswers((a) => ({ ...a, targets: { ...kind.defaults } }))} />
          )}
          {step === 4 && <Review gen={gen} ev={evalState.fresh ? evalState.ev : null} error={out.error} />}
          <div className="wz-nav">
            {step > 0 && <button className="btn" onClick={() => go(step - 1)}>← Back</button>}
            <span className="spacer" />
            {step >= 1 && gen && step < 4 && <DownloadButton small filename={`${slug}.json`} content={() => JSON.stringify(gen.project, null, 2)} label="project.json" />}
            {step < STEPS.length - 1 ? (
              <button className="btn primary" onClick={() => go(step + 1)}>Next: {STEPS[step + 1]} →</button>
            ) : (
              <>
                {gen && <DownloadButton filename={`${slug}.json`} content={() => JSON.stringify(gen.project, null, 2)} label="Download project.json" />}
                <button className="btn primary" disabled={!gen} onClick={create}><Icon name="check" size={15} /> Create project</button>
              </>
            )}
          </div>
          {out.error && <div className="callout" style={{ marginTop: 12, borderLeftColor: 'var(--bad)', color: 'var(--bad-text)' }}>{out.error}</div>}
        </section>

        <aside className="wz-preview" aria-label="Live preview">
          <Preview gen={gen} o={o} ev={evalState} />
        </aside>
      </div>

      <Explainer
        title="How the wizard decides — and what it will not pretend"
        refs={[
          { label: 'NASA Systems Engineering Handbook, SP-2016-6105 Rev 2 (2016)' },
          { label: 'D. Linden & T. B. Reddy, Handbook of Batteries, 4th ed. (2011)' },
          { label: 'F. P. Incropera et al., Fundamentals of Heat and Mass Transfer, 7th ed. (2011), ch. 9' },
          { label: 'R. C. Hibbeler, Mechanics of Materials, 10th ed. (2017)' },
          { label: "P. O'Connor & A. Kleyner, Practical Reliability Engineering, 5th ed. (2012)" },
          { label: 'IPC-7351B, land patterns and courtyards' },
        ]}
      >
        <p style={{ margin: '4px 0' }}>
          Each kind and capability adds needs, subsystems, interfaces and <b>typical</b> parts (mass, power, price class — no part numbers, no quotes).
          The wizard then sizes the design with the maths shown under “Why?”: average power from duty cycles, the battery from power × life (plus a 20 % growth
          allowance and enough capacity to survive its charge cycles), the printed circuit board (PCB) from footprint courtyards, the enclosure's mass from its
          shell, its thermal resistance from its surface area, and a reliability target from your design life.
        </p>
        <p style={{ margin: '4px 0' }}>
          Requirements are bound only to metrics that perturbation experiments showed respond to a generic project's own data (see the Labs tab). Labs that
          analyse the Iris glasses geometry — CAD, structures, optics, molding — are listed as such, and their requirements start as tests. A failing default is
          reported, never tuned away.
        </p>
      </Explainer>

      <Modal
        open={!!confirm}
        onClose={() => setConfirm(null)}
        title={confirm ? `Open “${confirm.label}” in Systems Studio?` : ''}
        actions={
          <>
            <DownloadButton small filename={`${project.meta.id || 'project'}.json`} content={() => JSON.stringify(project, null, 2)} label="Save current first" />
            <button className="btn" onClick={() => setConfirm(null)}>Cancel</button>
            <button className="btn primary" onClick={doReplace}>Replace project</button>
          </>
        }
      >
        <p className="muted">
          This replaces <b style={{ color: 'var(--text)' }}>{project.meta.name}</b> in the browser. Edits that are not saved to a project.json file are lost —
          download it first if you want to keep it.
        </p>
      </Modal>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 1 — describe
// ---------------------------------------------------------------------------

function Describe({ answers, setAnswers, o, onLoad }: { answers: Answers; setAnswers: (f: (a: Answers) => Answers) => void; o: WizardOptions; onLoad: (p: Project, label: string) => void }) {
  const templates = useCompute<Template[]>('systems.templates', {});
  const field = (k: 'name' | 'idea' | 'users') => (e: React.ChangeEvent<HTMLInputElement>) => {
    const v = e.target.value;
    setAnswers((a) => ({ ...a, [k]: v }));
  };
  return (
    <>
      <p className="lead">Plain words are fine. You can change everything later in Systems Studio.</p>
      <div className="wz-field">
        <label htmlFor="wz-name">Product name</label>
        <input id="wz-name" value={answers.name} onChange={field('name')} placeholder="e.g. Posture Clip" autoComplete="off" />
      </div>
      <div className="wz-field">
        <label htmlFor="wz-idea">One-line idea</label>
        <input id="wz-idea" value={answers.idea} onChange={field('idea')} placeholder="e.g. A clip-on sensor that buzzes when you slouch" autoComplete="off" />
        <div className="wz-examples" aria-label="Example ideas">
          {IDEAS.map((x) => (
            <button
              key={x.name}
              className="wz-example"
              onClick={() => {
                const k = o.kinds.find((y) => y.id === x.kind)!;
                setAnswers((a) => ({ ...a, name: x.name, idea: x.idea, kind: k.id, capabilities: [...k.default_capabilities], targets: { ...k.defaults } }));
              }}
            >
              {x.idea}
            </button>
          ))}
        </div>
      </div>
      <div className="wz-field">
        <label htmlFor="wz-users">Who uses it?</label>
        <input id="wz-users" value={answers.users} onChange={field('users')} placeholder="e.g. Office workers, students" autoComplete="off" />
        <span className="hint">Separate groups with commas; they become the stakeholders.</span>
      </div>
      <div className="wz-subhead" style={{ marginTop: 18 }}>Or start another way</div>
      <div className="wz-alts">
        {(templates.data ?? []).map((t) => (
          <button key={t.id} className="wz-alt" onClick={() => onLoad(structuredClone(t.project), t.name)}>
            <b>{t.id === 'blank' ? 'Blank project' : `Template: ${t.name.split(' — ')[0]}`}</b>
            <span>{t.id === 'blank' ? 'An empty project with the standard stage gates — write everything yourself.' : `${t.project.requirements.length} requirements · ${t.project.subsystems.length} subsystems`}</span>
          </button>
        ))}
      </div>
    </>
  );
}

// ---------------------------------------------------------------------------
// Step 2 — kind
// ---------------------------------------------------------------------------

function KindStep({ o, kind, pick }: { o: WizardOptions; kind: KindOption; pick: (k: KindOption) => void }) {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const idx = o.kinds.findIndex((k) => k.id === kind.id);
  const onKey = (e: React.KeyboardEvent) => {
    const d = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    const n = (idx + d + o.kinds.length) % o.kinds.length;
    pick(o.kinds[n]);
    refs.current[n]?.focus();
  };
  return (
    <>
      <p className="lead">The kind sets sensible defaults — capabilities, size, life, cost — and how warm a touchable surface may get.</p>
      <div className="wz-kinds" role="radiogroup" aria-label="Product kind" onKeyDown={onKey}>
        {o.kinds.map((k, i) => (
          <button
            key={k.id}
            ref={(el) => { refs.current[i] = el; }}
            role="radio"
            aria-checked={k.id === kind.id}
            tabIndex={k.id === kind.id ? 0 : -1}
            className="wz-kind"
            onClick={() => pick(k)}
          >
            <span className="art"><KindArt id={k.id} /></span>
            <b>{k.name}</b>
            <span>{k.blurb}</span>
          </button>
        ))}
      </div>
      <div className="wz-help" aria-live="polite">
        <b>{kind.name}.</b> {kind.help} <span className="muted">Examples: {kind.examples.join(', ')}.</span>
        {kind.scope && <div style={{ marginTop: 6, color: 'var(--warn)' }}>Scope: {kind.scope}</div>}
      </div>
    </>
  );
}

// ---------------------------------------------------------------------------
// Step 3 — capabilities
// ---------------------------------------------------------------------------

function CapStep({ o, answers, resolved, toggle }: { o: WizardOptions; answers: Answers; resolved: string[]; toggle: (c: CapOption) => void }) {
  const [focus, setFocus] = useState<CapOption | null>(null);
  const groups = useMemo(() => {
    const g: { name: string; caps: CapOption[] }[] = [];
    for (const c of o.capabilities) {
      let e = g.find((x) => x.name === c.group);
      if (!e) g.push((e = { name: c.group, caps: [] }));
      e.caps.push(c);
    }
    return g;
  }, [o.capabilities]);
  const shown = focus ?? o.capabilities.find((c) => c.id === answers.capabilities[answers.capabilities.length - 1]) ?? null;
  return (
    <>
      <p className="lead">Toggle what the product needs. Each choice adds parts, power, requirements and risks — watch the preview change.</p>
      {groups.map((g) => (
        <div className="wz-group" key={g.name}>
          <h4>{g.name}</h4>
          <div className="wz-chips">
            {g.caps.map((c) => {
              const chosen = answers.capabilities.includes(c.id);
              const auto = !chosen && resolved.includes(c.id);
              return (
                <button
                  key={c.id}
                  className={`wz-cap ${auto ? 'auto' : ''}`}
                  aria-pressed={chosen || auto}
                  aria-describedby="wz-cap-help"
                  title={c.help}
                  onClick={() => toggle(c)}
                  onFocus={() => setFocus(c)}
                  onMouseEnter={() => setFocus(c)}
                  onMouseLeave={() => setFocus(null)}
                >
                  <span className="tick" aria-hidden="true">{chosen || auto ? '✓' : ''}</span>
                  {c.name}
                  {auto && <span className="auto-tag">auto</span>}
                </button>
              );
            })}
          </div>
        </div>
      ))}
      <div className="wz-help" id="wz-cap-help" aria-live="polite">
        {shown ? (
          <>
            <b>{shown.name}.</b> {shown.help}
            {shown.excludes.length > 0 && <span className="muted"> Replaces {shown.excludes.map((x) => o.capabilities.find((c) => c.id === x)?.name.toLowerCase()).join(', ')}.</span>}
          </>
        ) : (
          <span className="muted">Point at or focus a capability to read what it adds. Dashed chips marked “auto” were added by the wizard (a power source, a charge input or an enclosure process chosen from your quantity).</span>
        )}
      </div>
    </>
  );
}

// ---------------------------------------------------------------------------
// Step 4 — targets
// ---------------------------------------------------------------------------

function TargetStep({ o, answers, set, reset }: { o: WizardOptions; answers: Answers; set: (id: string, v: number) => void; reset: () => void }) {
  const list = o.targets.filter((t) => applies(t, answers, o.capabilities));
  return (
    <>
      <p className="lead">Targets become requirements. Slide them and watch the battery, mass and verdicts move in the preview.</p>
      <div className="wz-targets">
        {list.map((t) => {
          const v = answers.targets[t.id] ?? t.default;
          return (
            <div className="wz-target" key={t.id}>
              <Slider label={t.label} value={v} min={t.min} max={t.max} step={t.step} log={t.log} unit={t.unit} format={(x) => fmtT(x, t.unit)}
                onChange={(x) => set(t.id, t.log ? Number(x.toPrecision(x >= 100 ? 3 : 2)) : Math.round(x / t.step) * t.step)} hint={t.help} />
              <p className="thelp">{t.help}</p>
            </div>
          );
        })}
      </div>
      <button className="btn small" onClick={reset}><Icon name="reset" size={14} /> Kind defaults</button>
    </>
  );
}

// ---------------------------------------------------------------------------
// Step 5 — review
// ---------------------------------------------------------------------------

function Review({ gen, ev, error }: { gen: WizardOut | undefined | null; ev: Evaluation | null; error: string | null }) {
  if (!gen) return <div className="muted">{error ?? 'Generating…'}</div>;
  const p = gen.project;
  const s = gen.summary;
  const bound = p.requirements.filter((r) => r.verification.metric);
  const pass = ev ? bound.filter((r) => ev.results.find((x) => x.id === r.id)?.status === 'pass').length : null;
  return (
    <>
      <p className="wz-review-sum">
        <b>{p.meta.name}</b> — {p.meta.tagline}. It has <b>{p.needs.length}</b> needs, <b>{p.subsystems.length}</b> subsystems,{' '}
        <b>{p.components.length}</b> part lines and <b>{p.requirements.length}</b> requirements: <b>{s.bound_requirements}</b> verified live by the labs
        {pass != null ? <> (<b style={{ color: pass === bound.length ? 'var(--good-text)' : 'var(--warn)' }}>{pass} passing now</b>)</> : <> (<span className="wz-pulse" aria-label="verifying" /> verifying)</>} and{' '}
        <b>{s.test_requirements}</b> to verify by test, inspection or demonstration. A failure mode and effects analysis (FMEA) with {p.risks.length} risks, one
        trade study (“{p.trades[0]?.title}”, decided: {p.trades[0]?.decision}) and engineering, design and production validation test (EVT, DVT, PVT) gates come with it.
      </p>
      {gen.warnings.length > 0 && (
        <div style={{ marginBottom: 10 }}>
          {gen.warnings.map((w, i) => (
            <div key={i} className={`wz-warn ${w.level}`}><span className="ico" aria-hidden="true">{w.level === 'warn' ? '!' : 'i'}</span><span>{w.text}</span></div>
          ))}
        </div>
      )}
      <div className="small muted">
        After you create it, Systems Studio opens with the new requirements already evaluated. Your current project is replaced only after you confirm.
      </div>
    </>
  );
}

// ---------------------------------------------------------------------------
// Live preview
// ---------------------------------------------------------------------------

type PvTab = 'arch' | 'bom' | 'reqs' | 'why' | 'labs';

function Preview({ gen, o, ev }: { gen: WizardOut | null | undefined; o: WizardOptions; ev: ReturnType<typeof usePreviewEvaluation> }) {
  const [tab, setTab] = useState<PvTab>('arch');
  if (!gen) return <section className="card"><div className="muted">Generating the preview…</div></section>;
  const p = gen.project;
  const s = gen.summary;
  const results = new Map<string, ReqResult>((ev.ev?.results ?? []).map((r) => [r.id, r]));
  const bound = p.requirements.filter((r) => r.verification.metric);
  const npass = ev.fresh ? bound.filter((r) => results.get(r.id)?.status === 'pass').length : null;
  const nfail = ev.fresh ? bound.filter((r) => results.get(r.id)?.status === 'fail').length : null;
  const tabs: { id: PvTab; label: string; n?: number }[] = [
    { id: 'arch', label: 'Architecture', n: p.subsystems.length },
    { id: 'bom', label: 'Parts', n: p.components.length },
    { id: 'reqs', label: 'Requirements', n: p.requirements.length },
    { id: 'why', label: 'Why?', n: gen.explanations.length },
    { id: 'labs', label: 'Labs' },
  ];
  return (
    <section className="card">
      <div className="wz-pv-head">
        <h3>{p.meta.name}</h3>
        <span className="grow" />
        {ev.fresh ? (
          <Chip tone={nfail ? 'warn' : 'pass'} title="Requirements bound to metrics, evaluated in the background">
            {nfail ? `${npass} pass · ${nfail} fail` : `✓ ${npass}/${bound.length} pass`}
          </Chip>
        ) : (
          <Chip tone="info"><span className="wz-pulse" aria-hidden="true" /> verifying…</Chip>
        )}
      </div>
      <div className="wz-stats">
        <Stat k="Mass" v={fmtNum(s.mass_g)} u="g" n={`target ${fmtNum(gen.targets.mass_g)} g`} />
        <Stat k="Average power" v={fmtNum(s.avg_power_mw)} u="mW" n={`peak ${fmtNum(s.peak_power_mw)} mW`} />
        {s.life_h != null ? (
          <Stat k="Battery life" v={fmtNum(s.life_h)} u="h" n={`${Math.round(s.battery_mah).toLocaleString('en-US')} mAh ${s.battery.includes('coin') ? 'coin cell' : 'LiPo'}`} />
        ) : (
          <Stat k="Surface temp." v={s.surface_c != null ? fmtNum(s.surface_c) : '—'} u="°C" n={`limit ${fmtNum(s.surface_limit_c)} °C`} />
        )}
        <Stat k="Parts at volume" v={fmtNum(s.bom_volume_usd)} u="USD" n={`${fmtNum(s.bom_usd)} USD at qty 1`} />
      </div>
      <div className="wz-tabs" role="tablist" aria-label="Preview">
        {tabs.map((t) => (
          <button key={t.id} role="tab" id={`wz-tab-${t.id}`} aria-controls={`wz-panel-${t.id}`} aria-selected={tab === t.id} className="wz-tab" onClick={() => setTab(t.id)}>
            {t.label}{t.n != null && <span className="cnt">{t.n}</span>}
          </button>
        ))}
      </div>
      <div role="tabpanel" id={`wz-panel-${tab}`} aria-labelledby={`wz-tab-${tab}`}>
        {tab === 'arch' && <ArchTab p={p} />}
        {tab === 'bom' && <BomTab p={p} />}
        {tab === 'reqs' && <ReqTab p={p} results={results} fresh={ev.fresh} error={ev.error} />}
        {tab === 'why' && <WhyTab gen={gen} />}
        {tab === 'labs' && <LabsTab gen={gen} o={o} />}
      </div>
    </section>
  );
}

function Stat({ k, v, u, n }: { k: string; v: ReactNode; u: string; n: string }) {
  return (
    <div className="wz-stat">
      <div className="k">{k}</div>
      <div className="v">{v}<small>{u}</small></div>
      <div className="n">{n}</div>
    </div>
  );
}

function ArchTab({ p }: { p: Project }) {
  const [ref, w] = useElementWidth<HTMLDivElement>();
  const reduced = useReducedMotion();
  const [play, setPlay] = useState(true);
  return (
    <div ref={ref}>
      <ArchDiagram subsystems={p.subsystems} interfaces={p.interfaces} width={w} animate={play && !reduced} />
      <div className="wz-legend">
        <span><i style={{ borderColor: 'var(--series-4)' }} />power</span>
        <span><i style={{ borderColor: 'var(--accent)', borderTopStyle: 'dashed' }} />data</span>
        <span><i style={{ borderColor: 'var(--muted)', borderTopStyle: 'dotted' }} />mechanical / optical</span>
        <span style={{ flex: 1 }} />
        {!reduced && (
          <button className="btn small" onClick={() => setPlay((x) => !x)} aria-pressed={play}>
            <Icon name="play" size={12} /> {play ? 'Pause flow' : 'Animate flow'}
          </button>
        )}
      </div>
      <div className="small muted" style={{ marginTop: 6 }}>
        {p.interfaces.length} interfaces: {['power', 'data', 'physical'].map((c) => `${p.interfaces.filter((i) => ifaceClass(i.kind) === c).length} ${c}`).join(', ')}. Hover a box or line for details.
      </div>
    </div>
  );
}

function BomTab({ p }: { p: Project }) {
  const sub = (id: string) => p.subsystems.find((s) => s.id === id)?.name ?? id;
  const avg = (c: Project['components'][number]) => c.qty * (c.power.active_mw * c.power.duty + c.power.sleep_mw * (1 - c.power.duty));
  const tot = p.components.reduce((a, c) => ({ m: a.m + c.mass_g * c.qty, p: a.p + avg(c), c: a.c + c.unit_cost_usd * c.qty }), { m: 0, p: 0, c: 0 });
  return (
    <>
      <div className="table-wrap wz-scroll">
        <table className="table wz-bom">
          <thead>
            <tr><th>Ref</th><th>Part (typical class)</th><th>Subsystem</th><th className="num">Qty</th><th className="num">g</th><th className="num">mW avg</th><th className="num">USD</th></tr>
          </thead>
          <tbody>
            {p.components.map((c) => (
              <tr key={c.refdes}>
                <td className="mono small">{c.refdes}</td>
                <td>{c.name}<span className="cls">{c.part_class}{c.location === 'board' ? ' · on the board' : ''}</span></td>
                <td className="small">{sub(c.subsystem)}</td>
                <td className="num">{c.qty}</td>
                <td className="num">{fmtNum(c.mass_g * c.qty)}</td>
                <td className="num">{avg(c) > 0 ? fmtNum(avg(c)) : '—'}</td>
                <td className="num">{fmtNum(c.unit_cost_usd * c.qty)}</td>
              </tr>
            ))}
          </tbody>
          <tfoot>
            <tr><td colSpan={4}>Totals (quantity 1)</td><td className="num">{fmtNum(tot.m)}</td><td className="num">{fmtNum(tot.p)}</td><td className="num">{fmtNum(tot.c)}</td></tr>
          </tfoot>
        </table>
      </div>
      <div className="small muted" style={{ marginTop: 6 }}>
        Typical planning values — mass, power and price classes, not quotes or part numbers. Average power before regulator losses; board parts start off the PCB so live verification stays fast.
      </div>
    </>
  );
}

function ReqTab({ p, results, fresh, error }: { p: Project; results: Map<string, ReqResult>; fresh: boolean; error: string | null }) {
  const bound = p.requirements.filter((r) => r.verification.metric);
  const manual = p.requirements.filter((r) => !r.verification.metric);
  const row = (r: Project['requirements'][number]) => {
    const res = results.get(r.id);
    const live = !!r.verification.metric;
    return (
      <li key={r.id} className={`wz-req ${live && !fresh ? 'stale' : ''}`}>
        <span className="id">{r.id}</span>
        <div>
          <div className="txt">{r.text}</div>
          <div className="crit">{live ? <>metric <code>{r.verification.metric}</code> · {res?.criterion ?? ''}</> : <>{r.verification.method} · {r.kind}</>}</div>
        </div>
        <div className="st">
          {live ? (
            res ? (
              <>
                <Chip tone={res.status === 'pass' ? 'pass' : res.status === 'fail' ? 'fail' : undefined}>{res.status === 'pass' ? '✓ pass' : res.status === 'fail' ? '✕ fail' : '○ —'}</Chip>
                {res.value != null && <span className="val">{fmtNum(res.value)} {res.unit}</span>}
              </>
            ) : <Chip tone="info">…</Chip>
          ) : (
            <Chip title="Verified outside the engine">○ {r.verification.method}</Chip>
          )}
        </div>
      </li>
    );
  };
  return (
    <>
      {error && <div className="callout" style={{ color: 'var(--bad-text)' }}>{error}</div>}
      <div className="wz-subhead">Verified live by the labs <Chip>{bound.length}</Chip></div>
      <ul className="wz-reqs">{bound.map(row)}</ul>
      <div className="wz-subhead">Verified by test, inspection or demonstration <Chip>{manual.length}</Chip></div>
      <ul className="wz-reqs">{manual.map(row)}</ul>
    </>
  );
}

function WhyTab({ gen }: { gen: WizardOut }) {
  return (
    <div>
      {gen.explanations.map((e, i) => (
        <details key={e.id} className="wz-why" open={i === 0 || e.id === 'battery'}>
          <summary>{e.title}<span className="res">{e.result}</span></summary>
          <div className="body">
            {e.plain}
            {e.math.length > 0 && <div className="math">{e.math.map((m, k) => <div key={k}>{m}</div>)}</div>}
          </div>
        </details>
      ))}
    </div>
  );
}

const STATUS_LABEL: Record<string, { label: string; tone: 'pass' | 'warn' | undefined }> = {
  yours: { label: 'your data', tone: 'pass' },
  partial: { label: 'partly yours', tone: 'warn' },
  iris: { label: 'Iris example', tone: undefined },
  unused: { label: 'not used', tone: undefined },
};

function LabsTab({ gen, o }: { gen: WizardOut; o: WizardOptions }) {
  const [table, setTable] = useState(false);
  const lab = (id: string) => (id === 'systems' ? { title: 'Systems Studio', path: '/systems?tab=budgets', icon: 'systems' } : LABS.find((l) => l.id === id));
  return (
    <>
      <p className="small muted" style={{ marginTop: 0 }}>
        Which labs analyse <b>this</b> product's data, and which still show the Iris smart-glasses example while you learn the method there.
      </p>
      <div className="wz-labs">
        {gen.labs.map((n) => {
          const l = lab(n.lab);
          const st = STATUS_LABEL[n.status] ?? STATUS_LABEL.iris;
          return (
            <Link key={n.lab} className="wz-lab" to={l?.path ?? '/'}>
              <span className="ic"><Icon name={l?.icon ?? 'systems'} size={16} /></span>
              <span><span className="nm">{l?.title ?? n.lab}</span><span className="nt" style={{ display: 'block' }}>{n.note}</span></span>
              <Chip tone={st.tone}>{st.label}</Chip>
            </Link>
          );
        })}
      </div>
      <button className="btn small" style={{ marginTop: 10 }} onClick={() => setTable((x) => !x)} aria-expanded={table}>
        {table ? 'Hide' : 'Show'} the generic-vs-Iris metric table
      </button>
      {table && (
        <div className="table-wrap" style={{ marginTop: 8 }}>
          <table className="table wz-mtable">
            <thead><tr><th>Metrics</th><th>Responds to</th><th>Bound?</th></tr></thead>
            <tbody>
              {o.metric_table.map((r, i) => (
                <tr key={i}>
                  <td>{r.keys.map((k) => <code key={k} style={{ display: 'block' }}>{k}</code>)}</td>
                  <td className="small">{r.responds_to}{r.note && <span className="muted" style={{ display: 'block' }}>{r.note}</span>}</td>
                  <td><Chip tone={r.bound ? 'pass' : undefined}>{r.bound ? 'yes' : r.class === 'iris' ? 'Iris' : r.class}</Chip></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </>
  );
}
