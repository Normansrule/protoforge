// Overview — the landing page on GitHub Pages. Everything numeric on this page
// is computed live by the Rust engine from the current project (Iris by
// default), so the page doubles as a dashboard.
import { Suspense, lazy, useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { Icon } from '../components/Icon';
import { useEvaluation, useProject } from '../state/project';
import type { Computed } from '../engine/hooks';
import type { Evaluation } from '../engine/types';
import { tryCall } from '../engine/engine';
import { Pipeline } from './home/Pipeline';
import { LabGrid } from './home/LabGrid';
import { Architecture } from './home/Architecture';
import { GROUPS, LABS, fmt } from './home/status';
import { useCountUp, useNearViewport } from './home/motion';
import './home/home.css';
import { WelcomeCard } from '../ux/start/WelcomeCard';
import { NextActions } from '../ux/next/NextActions';

const HeroModel = lazy(() => import('./home/HeroModel'));

const REPO = 'https://github.com/Normansrule/protoforge';

function GlassesSketch() {
  // Lightweight placeholder drawn while three.js loads.
  return (
    <svg viewBox="0 0 320 140" className="hm-sketch" aria-hidden>
      <path d="M20 58 C20 30 60 26 100 28 C132 30 142 40 146 58 C150 86 134 108 98 110 C58 112 22 100 20 58Z" />
      <path d="M300 58 C300 30 260 26 220 28 C188 30 178 40 174 58 C170 86 186 108 222 110 C262 112 298 100 300 58Z" />
      <path d="M146 52 C152 44 168 44 174 52" />
      <path d="M20 50 L6 46 M300 50 L314 46" />
      <rect x="232" y="18" width="62" height="16" rx="5" className="hm-sketch-pod" />
    </svg>
  );
}

function Hero({ ev }: { ev: Computed<Evaluation> }) {
  const project = useProject((s) => s.project);
  const metrics = useMemo(() => new Map((ev.data?.metrics ?? []).map((m) => [m.key, m])), [ev.data]);
  const [load3d, setLoad3d] = useState(false);
  const [ready, setReady] = useState(false);
  const pass = useCountUp(ev.data?.pass ?? 0);
  // Let the page paint first, then pull in three.js + the model.
  useEffect(() => {
    const w = window as Window & { requestIdleCallback?: (cb: () => void, o?: { timeout: number }) => number };
    if (w.requestIdleCallback) {
      w.requestIdleCallback(() => setLoad3d(true), { timeout: 600 });
    } else {
      const t = setTimeout(() => setLoad3d(true), 120);
      return () => clearTimeout(t);
    }
  }, []);
  const mass = metrics.get('mass.total_g');
  const cost = metrics.get('mfg.unit_cost_usd');
  const n = ev.data?.results.length ?? project.requirements.length;
  const modules = tryCall<string[]>('api.methods').data?.length ?? 0;

  return (
    <section className="hm-hero">
      <div className="hm-hero-text">
        <div className="hm-kicker">Open-source systems-engineering workbench</div>
        <h1 className="hm-title">Protoforge</h1>
        <div className="hm-tagline">
          Idea <span className="hm-arrow" aria-hidden>→</span><span className="hm-sr">to</span> atoms
        </div>
        <p className="hm-pitch">
          Take a product from a one-line idea to factory-ready files, entirely in your browser: stakeholder needs, requirements bound to live
          metrics, then circuits, digital signal processing (DSP), chip logic, control, robotics, reinforcement learning (RL), optics,
          computer-aided design (CAD), 3D printing, printed circuit boards (PCB) and injection molding — ending in a fabrication package with
          Gerbers, a bill of materials (BOM), stereolithography (STL) meshes and G-code. A Rust engine compiled to WebAssembly does the maths
          locally; nothing is uploaded.
        </p>
        <div className="hm-cta">
          <Link to="/systems" className="btn primary hm-btn"><Icon name="systems" size={16} /> Open Systems Studio</Link>
          <a href="#labs" className="btn hm-btn" onClick={(e) => { e.preventDefault(); document.getElementById('hm-labs')?.scrollIntoView({ behavior: 'smooth' }); }}>
            <Icon name="cad" size={16} /> Explore the labs
          </a>
          <a href={REPO} className="btn ghost hm-btn" target="_blank" rel="noreferrer"><Icon name="github" size={16} /> View on GitHub</a>
        </div>
        <dl className="hm-facts">
          <div><dt>Labs</dt><dd className="tnum">{LABS.length}</dd></div>
          <div><dt>Requirements verified</dt><dd className="tnum">{Math.round(pass)}<span>/{n}</span></dd></div>
          <div><dt>Engine modules</dt><dd className="tnum">{modules}</dd></div>
          <div><dt>Servers needed</dt><dd className="tnum">0</dd></div>
        </dl>
      </div>

      <div className="hm-stage">
        <div className="hm-stage-glow" aria-hidden />
        {!ready && <GlassesSketch />}
        {load3d && (
          <Suspense fallback={null}>
            <HeroModel onReady={() => setReady(true)} />
          </Suspense>
        )}
        <div className="hm-stage-top">
          <span className="chip info">● live model</span>
          <span className="hm-stage-name">{project.meta.name} <span className="muted">v{project.meta.version}</span></span>
          <span className="hm-stage-hint muted">{ready ? 'generated by the Rust engine · drag to orbit' : 'loading 3D…'}</span>
        </div>
        <div className="hm-stage-chips">
          {ev.data && (
            <Link to="/systems" className={`hm-float ${ev.data.fail - (ev.data.open_goals ?? 0) ? 'fail' : 'pass'}`}>
              <b className="tnum">{ev.data.fail - (ev.data.open_goals ?? 0) ? '✕' : '✓'} {ev.data.pass}/{ev.data.results.length}</b>
              <span>requirements verified{ev.data.fail - (ev.data.open_goals ?? 0) ? ` · ${ev.data.fail - (ev.data.open_goals ?? 0)} failing` : ''}{ev.data.open_goals ? ` · ${ev.data.open_goals} open goal` : ''}</span>
            </Link>
          )}
          {mass && (
            <Link to="/cad" className="hm-float">
              <b className="tnum">{fmt(mass.value)} g</b>
              <span>wearable mass</span>
            </Link>
          )}
          {cost && (
            <Link to="/mfg" className="hm-float">
              <b className="tnum">${cost.value.toFixed(2)}</b>
              <span>unit cost @ {project.design.manufacturing.target_quantity.toLocaleString('en-US')}</span>
            </Link>
          )}
        </div>
      </div>
    </section>
  );
}

function Section(props: { id: string; kicker: string; title: string; sub?: React.ReactNode; children: React.ReactNode; actions?: React.ReactNode }) {
  const ref = useRef<HTMLElement>(null);
  const seen = useNearViewport(ref, '-60px');
  return (
    <section id={props.id} ref={ref} className={`hm-section ${seen ? 'in' : ''}`} aria-labelledby={`${props.id}-h`}>
      <div className="hm-section-head">
        <div>
          <div className="page-kicker">{props.kicker}</div>
          <h2 id={`${props.id}-h`}>{props.title}</h2>
          {props.sub && <p className="hm-section-sub">{props.sub}</p>}
        </div>
        {props.actions}
      </div>
      {props.children}
    </section>
  );
}

const STEPS = [
  {
    n: 1, icon: 'learn', title: 'Describe your product', to: '/systems', link: 'Systems Studio',
    text: 'Start from a template or a blank project: the problem, users, use cases, stakeholder needs and constraints. Everything lives in one project.json you own.',
  },
  {
    n: 2, icon: 'systems', title: 'Write requirements bound to metrics', to: '/systems', link: 'Requirements',
    text: 'Each “shall” statement names a metric key (e.g. circuit.rail_droop_mv) and a criterion (≤ 50 mV). Trace it back to a need and allocate it to a subsystem.',
  },
  {
    n: 3, icon: 'control', title: 'Iterate in the labs until green', to: '/circuit', link: 'Open a lab',
    text: 'Every slider re-runs the analyses and re-verifies every requirement in milliseconds. Trade studies and the failure mode and effects analysis (FMEA) keep the decisions honest.',
  },
  {
    n: 4, icon: 'download', title: 'Download the fabrication package', to: '/mfg', link: 'Manufacturing Hub',
    text: 'One click zips Gerbers, drill files, BOM and component placement list (CPL), STL and 3MF parts, G-code, a molding design-for-manufacturing (DFM) report, cost curves and ordering guides.',
  },
];

const SCOPE_REAL = [
  ['Circuit Lab', 'modified nodal analysis (MNA) with Newton–Raphson; DC, transient and AC analyses — decks run unchanged in ngspice'],
  ['DSP Lab', 'fast Fourier transform (FFT), windowed and least-squares FIR, biquad IIR, cascaded integrator–comb (CIC) decimation, beamforming'],
  ['Chip Lab', 'Quine–McCluskey minimisation, event-driven gate simulation, finite-state machines, synthesisable Verilog'],
  ['Control / RL', 'state-space simulation, Bode and root locus, linear–quadratic regulator (LQR), tabular Q-learning and value iteration'],
  ['CAD / Print / PCB', 'watertight meshes with divergence-theorem mass, a real slicer emitting G-code, a Lee maze router with design rule checks (DRC) and RS-274X Gerbers'],
];
const SCOPE_APPROX = [
  'Thermal behaviour is a lumped resistor–capacitor network, not finite-element or computational fluid dynamics.',
  'Chip cell counts and timing are estimates — no synthesis against a real standard-cell library.',
  'Molding cycle time and clamp force come from textbook formulas, not mold-flow simulation.',
  'Costs are indicative learning-curve models. Vendor names are real; prices are not quotes.',
  'No signal-integrity, electromagnetic-compatibility or tolerance (Monte Carlo) analysis yet.',
];
const GRADUATE = [
  ['Schematic & PCB', 'KiCad (+ FreeRouting)'],
  ['Circuit simulation', 'ngspice · Xyce'],
  ['Digital design', 'Yosys · Verilator · cocotb'],
  ['Chip layout', 'OpenROAD / OpenLane 2 · Tiny Tapeout'],
  ['Mechanical CAD', 'Onshape · FreeCAD · build123d'],
  ['Slicing', 'OrcaSlicer · Bambu Studio · PrusaSlicer'],
  ['Control & robotics', 'python-control · Drake · MuJoCo · ROS 2'],
  ['Cost', 'real quotes: JLCPCB, PCBWay, Protolabs, Xometry'],
];

export default function Home() {
  const project = useProject((s) => s.project);
  const ev = useEvaluation();
  const [filter, setFilter] = useState('All');
  return (
    <div className="hm">
      <WelcomeCard />
      <Hero ev={ev} />

      <Section id="hm-next" kicker="Next steps" title="What should I do next?" sub="The open issues on this product, most important first — each links to the lab and tab that fixes it.">
        <NextActions limit={6} compact />
      </Section>

      <Section
        id="pipeline"
        kicker="The pipeline"
        title="From idea to factory, verified at every step"
        sub={<>The systems-engineering “V” as a live flow. Each lab group shows how many of its requirements currently pass; click any node to open it.</>}
      >
        <div className="card hm-pipe-card">
          <Pipeline project={project} ev={ev.data} />
          <div className="hm-legend small muted">
            <span><i className="hm-dot pass" /> ✓ all verified</span>
            <span><i className="hm-dot partial" /> … some unverified</span>
            <span><i className="hm-dot fail" /> ✕ failing</span>
            <span><i className="hm-dot none" /> ○ no bound requirement</span>
            {ev.data && <span className="hm-legend-ms">re-verified in {ev.ms < 1 ? '<1' : ev.ms.toFixed(0)} ms</span>}
          </div>
        </div>
      </Section>

      <Section
        id="hm-labs"
        kicker="The labs"
        title={`${LABS.length} labs, one product model`}
        sub="Every lab reads the same project, teaches the maths behind it with references, and publishes metrics that verify requirements. Numbers below are live."
        actions={
          <div className="hm-filter" role="tablist" aria-label="Filter labs by group">
            {['All', ...GROUPS].map((g) => (
              <button key={g} role="tab" aria-selected={filter === g} className={`hm-pill ${filter === g ? 'on' : ''}`} onClick={() => setFilter(g)}>
                {g}
              </button>
            ))}
          </div>
        }
      >
        <LabGrid project={project} ev={ev.data} filter={filter} />
      </Section>

      <Section
        id="how"
        kicker="How it works"
        title="One engine, one function, three front doors"
        sub={<>The whole engine is reached through a single JSON call, <code>call(method, input)</code>. The browser runs it as WebAssembly; the command-line interface (CLI) and the representational state transfer (REST) server link the very same Rust crate.</>}
      >
        <Architecture />
      </Section>

      <Section id="build" kicker="Build anything" title="Your product, same workflow" sub={<>Iris is the worked example, not a limit. Templates for new products live in the <Link to="/systems">Systems Studio</Link>.</>}>
        <ol className="hm-steps">
          {STEPS.map((s) => (
            <li key={s.n} className="hm-step">
              <div className="hm-step-n">{s.n}</div>
              <div className="hm-step-icon"><Icon name={s.icon} size={20} /></div>
              <h3>{s.title}</h3>
              <p>{s.text}</p>
              <Link to={s.to} className="hm-step-link">{s.link} <Icon name="arrow" size={14} /></Link>
            </li>
          ))}
        </ol>
      </Section>

      <Section id="scope" kicker="Honest scope" title="What is simulated, what is approximated" sub="Protoforge is a teaching and early-design tool. Use it to learn, explore and size a design — then graduate to professional tools before you spend money.">
        <div className="hm-scope">
          <div className="hm-scope-col">
            <h3><span className="chip pass">✓ real</span> Genuine algorithms</h3>
            <ul>
              {SCOPE_REAL.map(([k, v]) => (
                <li key={k}><b>{k}:</b> {v}.</li>
              ))}
            </ul>
          </div>
          <div className="hm-scope-col">
            <h3><span className="chip warn">≈ approx</span> Educational approximations</h3>
            <ul>
              {SCOPE_APPROX.map((v) => <li key={v}>{v}</li>)}
            </ul>
          </div>
          <div className="hm-scope-col hm-grad">
            <h3><span className="chip info">→ next</span> Graduate to</h3>
            <table className="table">
              <tbody>
                {GRADUATE.map(([k, v]) => (
                  <tr key={k}><td className="muted">{k}</td><td>{v}</td></tr>
                ))}
              </tbody>
            </table>
            <Link to="/learn" className="small">Reference repositories to study →</Link>
          </div>
        </div>
      </Section>

      <footer className="hm-footer">
        <div className="hm-footer-brand">
          <img src="./favicon.svg" width={28} height={28} alt="" />
          <div>
            <b>Protoforge</b>
            <div className="muted small">idea → atoms · MIT License · © {new Date().getFullYear()} Aleksander Norman</div>
          </div>
        </div>
        <nav className="hm-footer-links small" aria-label="Footer">
          <a href={REPO} target="_blank" rel="noreferrer">Source on GitHub</a>
          <a href={`${REPO}/blob/main/docs/getting-started.md`} target="_blank" rel="noreferrer">Getting started</a>
          <a href={`${REPO}/blob/main/docs/architecture.md`} target="_blank" rel="noreferrer">Architecture</a>
          <Link to="/learn">Guide & references</Link>
          <a href={`${REPO}/issues/new/choose`} target="_blank" rel="noreferrer">Report an issue</a>
        </nav>
      </footer>
    </div>
  );
}
