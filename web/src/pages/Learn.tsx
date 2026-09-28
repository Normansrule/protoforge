// Guide & references: a guided walkthrough of the Protoforge workflow using
// Iris, a searchable glossary, the consolidated bibliography, open-source
// repositories to study and free learning resources.
import { useDeferredValue, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { Link } from 'react-router-dom';
import { Icon } from '../components/Icon';
import { useEvaluation, useProject } from '../state/project';
import { LABS } from '../labs/registry';
import { AREAS, GLOSSARY, type Term } from './learn/glossary';
import { REFERENCES } from './learn/references';
import { REPOS } from './learn/repos';
import { RESOURCES } from './learn/resources';
import './learn/learn.css';

const labById = Object.fromEntries(LABS.map((l) => [l.id, l]));
const labName = (id: string) => (id === 'systems' ? 'Systems Studio' : labById[id]?.title ?? id);
const labPath = (id: string) => (id === 'systems' ? '/systems' : labById[id]?.path ?? '/');

// ------------------------------------------------------------------ workflow

interface Step {
  title: string;
  phase: string;
  labs: string[];
  reqs: string[];
  do: ReactNode;
  look: string[];
}

const STEPS: Step[] = [
  {
    title: 'Frame the problem',
    phase: 'Concept',
    labs: ['systems'],
    reqs: [],
    do: <>Open the Systems Studio and read Iris’s concept: the problem (glanceable information without pulling out a phone), the users and the use cases. For your own product, start from a template and write these first — every later number traces back here.</>,
    look: ['Problem and vision statements', 'Users and use-case narratives', 'Constraints (mass, budget, hobby-grade tooling)'],
  },
  {
    title: 'Capture needs, derive requirements',
    phase: 'Requirements',
    labs: ['systems'],
    reqs: ['R-001', 'R-002'],
    do: <>Each stakeholder need (priority must / should / could) derives one or more “shall” requirements. The key Protoforge habit: <b>bind every requirement to a metric key and a criterion</b>, e.g. <code>mass.total_g ≤ 50 g</code>, so it can be verified automatically.</>,
    look: ['Traceability: need → requirement → subsystem → metric → lab', 'Verification method (analysis, test, inspection, demonstration)', 'Lint warnings for unverifiable or orphaned requirements'],
  },
  {
    title: 'Architecture, interfaces and trades',
    phase: 'Architecture',
    labs: ['systems'],
    reqs: [],
    do: <>Allocate functions to subsystems, draw their interfaces (the N-squared (N²) diagram) and settle open decisions with weighted trade studies. Start a failure mode and effects analysis (FMEA): the risk priority number (RPN) tells you which risks to mitigate first.</>,
    look: ['Subsystems and the N² interface matrix', 'Trade-study winner and its weight sensitivity', 'Top FMEA risks by RPN and their mitigations'],
  },
  {
    title: 'Electronics: circuits, power, board',
    phase: 'Design · Electronics',
    labs: ['circuit', 'power', 'pcb'],
    reqs: ['R-012', 'R-013', 'R-014', 'R-002', 'R-003', 'R-008', 'R-017', 'R-018'],
    do: <>In the Circuit Lab pick the <i>Display turn-on</i> preset and watch the 1.8 V rail droop during the load step; check the buck ripple and the battery-sense divider current. Then size the energy budget and skin temperature in Power & Thermal, and place, route and design-rule check (DRC) the board in the PCB Studio.</>,
    look: ['Transient waveform of the rail droop vs the 50 mV limit', 'Battery life and average power per operating mode', 'Routed percentage, DRC violations, Gerber preview'],
  },
  {
    title: 'Signals and silicon',
    phase: 'Design · Signals & Silicon',
    labs: ['dsp', 'chip'],
    reqs: ['R-009', 'R-010', 'R-015', 'R-016'],
    do: <>The two microphones produce pulse-density modulation (PDM) bit streams. In the DSP Lab follow the cascaded integrator–comb (CIC) plus finite impulse response (FIR) decimation chain and the delay-and-subtract beamformer. The Chip Lab turns that FIR into Verilog, simulates it and estimates timing and cell count for a Tiny Tapeout tile.</>,
    look: ['Stopband attenuation of the decimation chain (≥ 60 dB)', 'Beam pattern and directivity index at 1 kHz', 'Critical path vs the 20 ns clock period; standard-cell estimate'],
  },
  {
    title: 'Motion and intelligence',
    phase: 'Design · Motion & Intelligence',
    labs: ['control', 'robotics', 'rl'],
    reqs: ['R-011', 'R-023', 'R-024'],
    do: <>Tune the complementary filter that fuses the inertial measurement unit (IMU) into a head-orientation estimate. The Robotics Lab plans the pick-and-place cycle that assembles the Iris board; the reinforcement learning (RL) Lab learns a power-management policy that beats the fixed duty cycle.</>,
    look: ['Head-tracking root-mean-square (RMS) error during the scripted scenario', 'Pick-and-place cycle time and path', 'Learned policy vs baseline battery life'],
  },
  {
    title: 'Optics and mechanics',
    phase: 'Design · Optics & Mechanics',
    labs: ['optics', 'cad', 'print'],
    reqs: ['R-004', 'R-005', 'R-006', 'R-007', 'R-001', 'R-019', 'R-021'],
    do: <>Ray-trace the display path for field of view (FOV), pixels per degree (PPD), image distance and eyebox. Shape the frame in the CAD Studio — watch the total mass and the board-to-pod fit margin — then slice it in the Print Lab and download G-code or the 3D Manufacturing Format (3MF) plate for Bambu Studio / OrcaSlicer.</>,
    look: ['FOV and PPD against the requirement', 'Exploded assembly, mass breakdown, fit margin', 'Layer preview, print time and filament'],
  },
  {
    title: 'Production',
    phase: 'Design · Production',
    labs: ['molding', 'mfg'],
    reqs: ['R-020', 'R-022'],
    do: <>Check draft angles, wall thickness, clamp force and cycle time in the Molding Lab. The Manufacturing Hub compares printing and molding cost across quantities, shows where molding wins, and builds the complete fabrication package.</>,
    look: ['Draft map (every wall ≥ 1°) and undercuts', 'Unit cost at the target quantity and the crossover quantity', 'Package contents: Gerbers, drill, bill of materials (BOM), component placement list (CPL), stereolithography (STL) meshes, 3MF, G-code, guides'],
  },
  {
    title: 'Verify, iterate, release',
    phase: 'Verification',
    labs: ['systems', 'mfg'],
    reqs: [],
    do: <>Return to the Systems Studio: every requirement is re-verified on each edit, and the verification report is one click away. The same check runs in continuous integration (CI) with <code>protoforge evaluate</code>. Plan the engineering, design and production validation builds (EVT, DVT, PVT) — then graduate each design to professional tools.</>,
    look: ['Verification matrix — all green?', 'Impact analysis: what else changes if a parameter moves', 'Markdown report and the downloadable fabrication package'],
  },
];

function Workflow() {
  const project = useProject((s) => s.project);
  const ev = useEvaluation();
  const status = (id: string) => ev.data?.results.find((r) => r.id === id)?.status;
  const text = (id: string) => project.requirements.find((r) => r.id === id)?.text ?? id;
  return (
    <ol className="ln-steps">
      {STEPS.map((s, i) => (
        <li key={s.title} className="ln-step">
          <div className="ln-step-rail" aria-hidden>
            <span className="ln-step-n">{i + 1}</span>
          </div>
          <div className="ln-step-body">
            <div className="ln-step-phase">{s.phase}</div>
            <h3>{s.title}</h3>
            <p>{s.do}</p>
            <div className="ln-step-cols">
              <div>
                <div className="ln-mini">What to look at</div>
                <ul>{s.look.map((l) => <li key={l}>{l}</li>)}</ul>
              </div>
              <div>
                <div className="ln-mini">Open</div>
                <div className="ln-step-labs">
                  {s.labs.map((id) => (
                    <Link key={id} to={labPath(id)} className="btn small">
                      <Icon name={id === 'systems' ? 'systems' : labById[id]?.icon ?? 'systems'} size={14} /> {labName(id)}
                    </Link>
                  ))}
                </div>
                {s.reqs.length > 0 && (
                  <>
                    <div className="ln-mini" style={{ marginTop: 10 }}>Requirements verified here</div>
                    <div className="ln-reqs">
                      {s.reqs.map((id) => {
                        const st = status(id);
                        return (
                          <Link key={id} to={`/systems?req=${id}`} className={`chip ${st === 'pass' ? 'pass' : st === 'fail' ? 'fail' : ''}`} title={text(id)}>
                            {st === 'pass' ? '✓' : st === 'fail' ? '✕' : '○'} {id}
                          </Link>
                        );
                      })}
                    </div>
                  </>
                )}
              </div>
            </div>
          </div>
        </li>
      ))}
    </ol>
  );
}

// ------------------------------------------------------------------ glossary

function mark(text: string, q: string): ReactNode {
  if (!q) return text;
  const i = text.toLowerCase().indexOf(q.toLowerCase());
  if (i < 0) return text;
  return (
    <>
      {text.slice(0, i)}
      <mark>{text.slice(i, i + q.length)}</mark>
      {text.slice(i + q.length)}
    </>
  );
}

function Glossary() {
  const [q, setQ] = useState('');
  const [area, setArea] = useState<Term['area'] | 'All'>('All');
  const [all, setAll] = useState(false);
  const dq = useDeferredValue(q.trim());
  const terms = useMemo(() => {
    const ql = dq.toLowerCase();
    return GLOSSARY.filter((t) => (area === 'All' || t.area === area) && (!ql || `${t.term} ${t.full} ${t.def}`.toLowerCase().includes(ql))).sort((a, b) =>
      a.term.localeCompare(b.term, 'en', { sensitivity: 'base' }),
    );
  }, [dq, area]);
  const shown = all || dq || area !== 'All' ? terms : terms.slice(0, 24);
  return (
    <div>
      <div className="ln-toolbar">
        <label className="ln-search">
          <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden><path d="M11 4a7 7 0 105 12l4 4M11 4a7 7 0 010 14" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" /></svg>
          <input type="search" placeholder={`Search ${GLOSSARY.length} terms — try “FIR”, “draft” or “Kalman”`} value={q} onChange={(e) => setQ(e.target.value)} aria-label="Search the glossary" />
        </label>
        <div className="ln-pills" role="tablist" aria-label="Filter by discipline">
          {(['All', ...AREAS] as const).map((a) => (
            <button key={a} role="tab" aria-selected={area === a} className={`ln-pill ${area === a ? 'on' : ''}`} onClick={() => setArea(a)}>
              {a}
            </button>
          ))}
        </div>
      </div>
      <div className="ln-count small muted" aria-live="polite">
        {shown.length < terms.length ? `showing ${shown.length} of ${terms.length} terms — search or pick a discipline to narrow down` : `${terms.length} ${terms.length === 1 ? 'term' : 'terms'}`}
      </div>
      {terms.length === 0 ? (
        <div className="callout">No match for “{dq}”. Missing a term? <a href="https://github.com/Normansrule/protoforge/issues/new/choose" target="_blank" rel="noreferrer">Suggest it on GitHub</a>.</div>
      ) : (
        <>
          <dl className="ln-terms">
            {shown.map((t) => (
              <div key={t.term} className="ln-term">
                <dt>
                  <span className="ln-term-abbr">{mark(t.term, dq)}</span>
                  <span className="ln-term-full">{mark(t.full, dq)}</span>
                </dt>
                <dd>
                  <span className="ln-term-def">{mark(t.def, dq)}</span>
                  <span className="ln-term-meta">
                    <span className="ln-term-area">{t.area}</span>
                    {t.lab && (
                      <Link to={t.lab} className="ln-term-lab">
                        {t.lab === '/systems' ? 'Systems Studio' : LABS.find((l) => l.path === t.lab)?.title} →
                      </Link>
                    )}
                  </span>
                </dd>
              </div>
            ))}
          </dl>
          {shown.length < terms.length && (
            <button className="btn ln-more" onClick={() => setAll(true)}>
              Show all {terms.length} terms
            </button>
          )}
        </>
      )}
    </div>
  );
}

// ------------------------------------------------------------------ references

/** Render `code` spans in short descriptions. */
function rich(text: string): ReactNode {
  return text.split(/(`[^`]+`)/g).map((p, i) => (p.startsWith('`') && p.endsWith('`') ? <code key={i}>{p.slice(1, -1)}</code> : p));
}

/** Render *italic* spans in citation strings. */
function cite(text: string): ReactNode {
  return text.split(/(\*[^*]+\*)/g).map((p, i) => (p.startsWith('*') && p.endsWith('*') ? <i key={i}>{p.slice(1, -1)}</i> : p));
}

function References() {
  const [q, setQ] = useState('');
  const dq = useDeferredValue(q.trim().toLowerCase());
  const groups = useMemo(
    () =>
      REFERENCES.map((g) => ({ ...g, refs: g.refs.filter((r) => !dq || `${r.text} ${g.discipline} ${r.labs.join(' ')}`.toLowerCase().includes(dq)) })).filter(
        (g) => g.refs.length > 0,
      ),
    [dq],
  );
  const total = REFERENCES.reduce((a, g) => a + g.refs.length, 0);
  let n = 0;
  return (
    <div>
      <div className="ln-toolbar">
        <label className="ln-search">
          <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden><path d="M11 4a7 7 0 105 12l4 4M11 4a7 7 0 010 14" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" /></svg>
          <input type="search" placeholder={`Filter ${total} references by author, title or lab`} value={q} onChange={(e) => setQ(e.target.value)} aria-label="Filter references" />
        </label>
      </div>
      <div className="ln-refs">
        {groups.map((g) => (
          <section key={g.discipline} className="ln-ref-group card">
            <h3>{g.discipline}</h3>
            <ol>
              {g.refs.map((r) => {
                n += 1;
                return (
                  <li key={r.text} value={n}>
                    <span className="ln-ref-text">{cite(r.text)}</span>{' '}
                    {r.href && (
                      <a href={r.href} target="_blank" rel="noreferrer" className="ln-ref-link">
                        {r.href.replace(/^https?:\/\//, '').replace(/\/$/, '')} ↗
                      </a>
                    )}
                    <span className="ln-ref-labs">
                      {r.labs.map((l) => (
                        <Link key={l} to={labPath(l)} className="ln-tag">{labName(l)}</Link>
                      ))}
                    </span>
                  </li>
                );
              })}
            </ol>
          </section>
        ))}
      </div>
    </div>
  );
}

// ------------------------------------------------------------------ repositories & resources

function Repositories() {
  return (
    <div className="ln-repos">
      {REPOS.map((g) => (
        <section key={g.discipline} className="ln-repo-group card">
          <h3>{g.discipline}</h3>
          <ul>
            {g.items.map((r) => {
              const href = r.url ?? (r.repo ? `https://github.com/${r.repo}` : undefined);
              return (
                <li key={r.name}>
                  <div className="ln-repo-head">
                    {href ? <a href={href} target="_blank" rel="noreferrer" className="ln-repo-name">{r.name}</a> : <span className="ln-repo-name">{r.name}</span>}
                    {r.repo && <code className="ln-repo-slug">{r.repo}</code>}
                  </div>
                  <div className="ln-repo-learn">{rich(r.learn)}</div>
                </li>
              );
            })}
          </ul>
        </section>
      ))}
    </div>
  );
}

function Resources() {
  const order = ['Free book', 'Free course', 'Practice', 'Handbook', 'Textbook'];
  const sorted = [...RESOURCES].sort((a, b) => order.indexOf(a.kind) - order.indexOf(b.kind));
  return (
    <div className="ln-res">
      {sorted.map((r) => (
        <div key={r.title} className={`ln-res-card ${r.kind.startsWith('Free') || r.kind === 'Practice' ? 'free' : ''}`}>
          <div className="ln-res-kind">{r.kind}</div>
          <div className="ln-res-title">{r.url ? <a href={r.url} target="_blank" rel="noreferrer">{r.title} ↗</a> : r.title}</div>
          <div className="ln-res-who">{r.who}</div>
          <p>{r.why}</p>
          <div className="ln-ref-labs">
            {r.labs.map((l) => (
              <Link key={l} to={labPath(l)} className="ln-tag">{labName(l)}</Link>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

// ------------------------------------------------------------------ page

const SECTIONS = [
  { id: 'workflow', label: 'Workflow guide' },
  { id: 'glossary', label: 'Glossary' },
  { id: 'references', label: 'References' },
  { id: 'repos', label: 'Repositories' },
  { id: 'resources', label: 'Learning resources' },
];

function useScrollSpy(ids: string[]): string {
  const [active, setActive] = useState(ids[0]);
  useEffect(() => {
    if (typeof IntersectionObserver === 'undefined') return;
    const io = new IntersectionObserver(
      (es) => {
        const vis = es.filter((e) => e.isIntersecting).sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top);
        if (vis[0]) setActive(vis[0].target.id.replace('ln-', ''));
      },
      { rootMargin: '-90px 0px -60% 0px' },
    );
    ids.forEach((id) => {
      const el = document.getElementById(`ln-${id}`);
      if (el) io.observe(el);
    });
    return () => io.disconnect();
  }, [ids]);
  return active;
}

function Section(props: { id: string; kicker: string; title: string; sub: ReactNode; children: ReactNode }) {
  return (
    <section id={`ln-${props.id}`} className="ln-section" aria-labelledby={`ln-${props.id}-h`}>
      <div className="page-kicker">{props.kicker}</div>
      <h2 id={`ln-${props.id}-h`}>{props.title}</h2>
      <p className="ln-sub">{props.sub}</p>
      {props.children}
    </section>
  );
}

export default function Learn() {
  const ids = useRef(SECTIONS.map((s) => s.id)).current;
  const active = useScrollSpy(ids);
  const go = (id: string) => document.getElementById(`ln-${id}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  const refCount = REFERENCES.reduce((a, g) => a + g.refs.length, 0);
  const repoCount = REPOS.reduce((a, g) => a + g.items.length, 0);
  return (
    <div className="ln">
      <header className="ln-hero">
        <div className="page-kicker">Learn</div>
        <h1>Guide & references</h1>
        <p className="page-sub">
          How to take a product through Protoforge — using the Iris smart glasses as the worked example — plus every acronym spelled out, the
          textbooks and papers behind each lab, open-source projects to study next and free courses to go deeper.
        </p>
        <div className="ln-stats">
          <span><b>{STEPS.length}</b> workflow steps</span>
          <span><b>{GLOSSARY.length}</b> glossary terms</span>
          <span><b>{refCount}</b> references</span>
          <span><b>{repoCount}</b> repositories</span>
          <span><b>{RESOURCES.length}</b> learning resources</span>
        </div>
      </header>
      <nav className="ln-toc" aria-label="On this page">
        {SECTIONS.map((s) => (
          <button key={s.id} className={`ln-toc-item ${active === s.id ? 'on' : ''}`} onClick={() => go(s.id)} aria-current={active === s.id ? 'true' : undefined}>
            {s.label}
          </button>
        ))}
      </nav>

      <Section id="workflow" kicker="Workflow guide" title="Designing a product with Protoforge" sub={<>Nine steps down and back up the systems-engineering “V”. Live chips show which Iris requirements each step verifies right now.</>}>
        <Workflow />
      </Section>

      <Section id="glossary" kicker="Glossary" title="Every acronym, spelled out" sub="Terms used across the labs, with their full form and a one-line definition. Search matches the abbreviation, expansion and definition.">
        <Glossary />
      </Section>

      <Section id="references" kicker="References" title="Consolidated bibliography" sub={<>Gathered from every lab guide (<code>docs/labs/*.md</code>) and the engine’s documentation, deduplicated and grouped by discipline. Links appear only where the source is freely available online.</>}>
        <References />
      </Section>

      <Section id="repos" kicker="Reference repositories" title="Open-source projects to study" sub="Mature tools that do what each lab teaches, at professional scale. Read their code, compare their numbers with Protoforge’s, and graduate your design to them.">
        <Repositories />
      </Section>

      <Section id="resources" kicker="Learning resources" title="Go deeper — free first" sub="Free books and courses in English, then the textbooks the labs cite most often.">
        <Resources />
      </Section>
    </div>
  );
}
