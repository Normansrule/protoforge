import { useMemo, useState } from 'react';
import { useCompute } from '../../engine/hooks';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat, Toggle } from '../../components/ui';
import { Plot, type Series } from '../../components/Plot';
import { PlayBar, divColor, fmt, json, toCSV, useClock } from '../control/kit';
import type { GridSpec, TdConfig, TdResult } from './types';

const ARROWS = ['↑', '→', '↓', '←'];

interface Preset { label: string; grid: GridSpec; td?: Partial<TdConfig> }
const PRESETS: Record<string, Preset> = {
  office: {
    label: 'Office floor (default)',
    grid: { layout: ['......#...G', '.##...#.#..', '..#.....#P.', '..#.PP..#..', 'S.......#..', '....##.....'], slip: 0.2, step_reward: -0.04, goal_reward: 1, pit_reward: -1, gamma: 0.95 },
  },
  aima: {
    label: 'Russell & Norvig 4×3',
    grid: { layout: ['...G', '.#.P', 'S...'], slip: 0.2, step_reward: -0.04, goal_reward: 1, pit_reward: -1, gamma: 1 },
    td: { episodes: 1500, alpha: 0.1 },
  },
  cliff: {
    label: 'Cliff walking (Sutton & Barto 6.6)',
    grid: { layout: ['............', '............', '............', 'SPPPPPPPPPPG'], slip: 0, step_reward: -0.01, goal_reward: 1, pit_reward: -1, gamma: 0.99 },
    td: { eps_start: 0.1, eps_end: 0.1, eps_decay: 1, episodes: 500, alpha: 0.5 },
  },
  maze: {
    label: 'Maze',
    grid: { layout: ['S.#.......', '..#.####..', '..#....#..', '.####..#.#', '......#...', '.####.#.#.', '....#...#G'], slip: 0.1, step_reward: -0.02, goal_reward: 1, pit_reward: -1, gamma: 0.97 },
    td: { episodes: 1200, q_init: 0.5 },
  },
};

const DEFAULT_TD: TdConfig = { algorithm: 'q_learning', episodes: 600, max_steps: 200, alpha: 0.2, eps_start: 1, eps_end: 0.05, eps_decay: 0.993, seed: 7, frames: 48, q_init: 0 };

type Tool = '#' | 'P' | 'G' | 'S' | '.';

function GridView(props: {
  w: number; h: number; layout: string[]; values?: number[]; policy?: number[]; path?: number[]; agent?: number; maxAbs?: number;
  onCell?: (r: number, c: number) => void; showValues?: boolean; title?: string;
}) {
  const { w, h } = props;
  const cs = 40;
  const pathPts = (props.path ?? []).map((s) => `${(s % w) * cs + cs / 2},${Math.floor(s / w) * cs + cs / 2}`).join(' ');
  return (
    <svg viewBox={`0 0 ${w * cs} ${h * cs}`} style={{ width: '100%', display: 'block', maxHeight: 420, cursor: props.onCell ? 'pointer' : 'default' }} role="img" aria-label={props.title ?? 'grid world'}>
      {Array.from({ length: h }, (_, r) =>
        Array.from({ length: w }, (_, c) => {
          const s = r * w + c;
          const ch = props.layout[r]?.[c] ?? '.';
          const v = props.values?.[s] ?? 0;
          let fill = 'var(--surface-2)';
          if (ch === '#') fill = 'color-mix(in srgb, var(--text-2) 55%, var(--surface-1))';
          else if (ch === 'G') fill = 'color-mix(in srgb, var(--good) 55%, var(--surface-2))';
          else if (ch === 'P') fill = 'color-mix(in srgb, var(--bad) 55%, var(--surface-2))';
          else if (props.values) fill = divColor(v, props.maxAbs ?? 1);
          const pol = props.policy?.[s] ?? -1;
          return (
            <g key={s} onClick={() => props.onCell?.(r, c)}>
              <title>{ch === '#' ? 'wall' : ch === 'G' ? 'goal' : ch === 'P' ? 'pit' : `V = ${v.toFixed(3)}${pol >= 0 ? `, best ${ARROWS[pol]}` : ''}`}</title>
              <rect x={c * cs + 1} y={r * cs + 1} width={cs - 2} height={cs - 2} rx={5} fill={fill} />
              {ch === 'G' && <text x={c * cs + cs / 2} y={r * cs + cs / 2 + 5} textAnchor="middle" fontSize={15} fill="var(--text)">★</text>}
              {ch === 'P' && <text x={c * cs + cs / 2} y={r * cs + cs / 2 + 5} textAnchor="middle" fontSize={14} fill="var(--text)">✕</text>}
              {ch === 'S' && <rect x={c * cs + 4} y={r * cs + 4} width={cs - 8} height={cs - 8} rx={4} fill="none" stroke="var(--accent)" strokeWidth={2} strokeDasharray="4 3" />}
              {pol >= 0 && ch !== '#' && ch !== 'G' && ch !== 'P' && (
                <text x={c * cs + cs / 2} y={r * cs + cs / 2 + (props.showValues ? -1 : 6)} textAnchor="middle" fontSize={props.showValues ? 13 : 18} fill="var(--text)" fontWeight={700}>{ARROWS[pol]}</text>
              )}
              {props.showValues && props.values && ch !== '#' && ch !== 'G' && ch !== 'P' && (
                <text x={c * cs + cs / 2} y={r * cs + cs - 6} textAnchor="middle" fontSize={9} fill="var(--text)" style={{ fontFamily: 'var(--mono)' }}>{v.toFixed(2)}</text>
              )}
            </g>
          );
        }),
      )}
      {pathPts && <polyline points={pathPts} fill="none" stroke="var(--series-4)" strokeWidth={3} strokeLinejoin="round" strokeLinecap="round" opacity={0.85} />}
      {props.agent !== undefined && props.agent >= 0 && (
        <circle cx={(props.agent % w) * cs + cs / 2} cy={Math.floor(props.agent / w) * cs + cs / 2} r={11} fill="var(--series-4)" stroke="var(--surface-1)" strokeWidth={3} />
      )}
    </svg>
  );
}

export function Gridworld() {
  const [presetId, setPresetId] = useState('office');
  const [grid, setGrid] = useState<GridSpec>(PRESETS.office.grid);
  const [td, setTd] = useState<TdConfig>(DEFAULT_TD);
  const [tool, setTool] = useState<Tool>('#');
  const [showValues, setShowValues] = useState(true);
  const [trained, setTrained] = useState<{ grid: GridSpec; td: TdConfig }>({ grid: PRESETS.office.grid, td: DEFAULT_TD });
  const [phase, setPhase] = useState<'train' | 'rollout'>('train');
  const res = useCompute<TdResult>('rl.gridworld', trained);
  const d = res.data;
  const dirty = JSON.stringify(trained) !== JSON.stringify({ grid, td });
  const nSnap = d?.snapshots.length ?? 1;
  const trainClock = useClock(Math.max(1, nSnap * 0.22), { loop: false, resetKey: res.data });
  const rollClock = useClock(Math.max(1, (d?.rollout.length ?? 1) * 0.25), { loop: true, autoplay: true, resetKey: res.data });
  const snapIdx = Math.min(nSnap - 1, Math.floor(trainClock.t / 0.22));
  const snap = d?.snapshots[snapIdx];
  const agent = d ? d.rollout[Math.min(d.rollout.length - 1, Math.floor(rollClock.t / 0.25))] : -1;
  const w = Math.max(...grid.layout.map((r) => r.length));
  const h = grid.layout.length;
  const [sweep, setSweep] = useState(1000);

  const maxAbs = useMemo(() => {
    if (!d) return 1;
    return Math.max(1e-6, ...d.vi.v.map(Math.abs), ...d.v.map(Math.abs));
  }, [d]);

  const choosePreset = (id: string) => {
    const p = PRESETS[id];
    setPresetId(id);
    setGrid(p.grid);
    const nt = { ...DEFAULT_TD, ...(p.td ?? {}), algorithm: td.algorithm };
    setTd(nt);
    setTrained({ grid: p.grid, td: nt });
  };
  const edit = (r: number, c: number) => {
    const rows = grid.layout.map((row) => row.padEnd(w, '.').split(''));
    if (tool === 'S') for (const row of rows) for (let k = 0; k < row.length; k++) if (row[k] === 'S') row[k] = '.';
    rows[r][c] = rows[r][c] === tool && tool !== 'S' ? '.' : tool;
    setGrid({ ...grid, layout: rows.map((x) => x.join('')) });
  };

  const curve = useMemo<Series[]>(() => {
    if (!d) return [];
    const x = d.returns.map((_, i) => i + 1);
    const ma = d.returns.map((_, i) => { const lo = Math.max(0, i - 49); const s = d.returns.slice(lo, i + 1); return s.reduce((a, b) => a + b, 0) / s.length; });
    const succ = d.successes.map((_, i) => { const lo = Math.max(0, i - 49); const s = d.successes.slice(lo, i + 1); return s.filter(Boolean).length / s.length; });
    return [
      { name: 'episode return', x, y: d.returns, color: 'var(--series-1)', width: 0.8, scatter: false },
      { name: '50-episode average', x, y: ma, color: 'var(--series-2)', width: 2.2 },
      { name: 'success rate (50 ep)', x, y: succ, color: 'var(--series-3)', width: 1.6, dash: '5 3' },
    ];
  }, [d]);
  const viRes = useMemo<Series[]>(() => (d ? [{ name: 'log₁₀ max |ΔV|', x: d.vi.residuals.map((_, i) => i + 1), y: d.vi.residuals.map((v) => Math.log10(Math.max(v, 1e-16))), color: 'var(--series-7)' }] : []), [d]);
  const sweepIdx = d ? Math.min(d.vi.sweeps.length - 1, sweep) : 0;
  const viValues = d ? (sweep >= d.vi.sweeps.length ? d.vi.v : d.vi.sweeps[sweepIdx]) : undefined;

  const qJson = () => json({ algorithm: d?.algorithm, grid: trained.grid, config: trained.td, w: d?.w, h: d?.h, actions: ['up', 'right', 'down', 'left'], q: d?.q, greedy_policy: d?.policy, value_iteration: { v: d?.vi.v, policy: d?.vi.policy } });
  const curveCsv = () => (d ? toCSV(['episode', 'return', 'steps', 'reached_goal'], [d.returns.map((_, i) => i + 1), d.returns, d.steps, d.successes.map((b) => (b ? 1 : 0))]) : '');

  return (
    <div className="stack">
      <div className="grid grid-4">
        <Stat label="Policy agreement with DP" value={fmt((d?.policy_agreement ?? NaN) * 100, 0)} unit="%" note="cells where the learned action is optimal" />
        <Stat label="Value error" value={fmt(d?.v_rms_error, 3)} note="RMS |max Q − V*| over free cells" />
        <Stat label="Greedy rollout" value={d ? (d.rollout_reached_goal ? '✓ goal' : '✕ no goal') : '—'} tone={d ? (d.rollout_reached_goal ? 'pass' : 'fail') : undefined} note={`${d ? d.rollout.length - 1 : 0} steps (slip sampled)`} />
        <Stat label="Value iteration" value={d?.vi.iterations ?? '—'} unit="sweeps" note="to max |ΔV| < 1e-10" />
      </div>
      <div className="grid grid-sidebar">
        <Card title="World & learner">
          <Select label="Preset" value={presetId} onChange={choosePreset} options={Object.entries(PRESETS).map(([k, p]) => ({ value: k, label: p.label }))} />
          <div className="field-label small" style={{ marginBottom: 6 }}>Click cells to paint</div>
          <div className="row" style={{ marginBottom: 10 }}>
            {([['#', 'Wall'], ['P', 'Pit'], ['G', 'Goal'], ['S', 'Start'], ['.', 'Erase']] as [Tool, string][]).map(([t, l]) => (
              <button key={t} className={`btn small ${tool === t ? 'primary' : ''}`} onClick={() => setTool(t)}>{l}</button>
            ))}
          </div>
          <Slider label="Slip probability" value={grid.slip} min={0} max={0.6} step={0.01} onChange={(v) => setGrid({ ...grid, slip: v })} />
          <Slider label="Step reward" value={grid.step_reward} min={-0.3} max={0} step={0.005} onChange={(v) => setGrid({ ...grid, step_reward: v })} />
          <Slider label="Discount γ" value={grid.gamma} min={0.5} max={1} step={0.005} onChange={(v) => setGrid({ ...grid, gamma: v })} />
          <Select label="Algorithm" value={td.algorithm} onChange={(v) => setTd({ ...td, algorithm: v })} options={[{ value: 'q_learning', label: 'Q-learning (off-policy)' }, { value: 'sarsa', label: 'SARSA (on-policy)' }]} />
          <Slider label="Episodes" value={td.episodes} min={50} max={5000} log step={1} onChange={(v) => setTd({ ...td, episodes: Math.round(v) })} format={(v) => Math.round(v).toString()} />
          <Slider label="Learning rate α" value={td.alpha} min={0.01} max={1} log onChange={(v) => setTd({ ...td, alpha: v })} />
          <Slider label="ε decay per episode" value={td.eps_decay} min={0.95} max={1} step={0.0005} format={(v) => v.toFixed(4)} onChange={(v) => setTd({ ...td, eps_decay: v })} />
          <Slider label="Final ε" value={td.eps_end} min={0} max={0.5} step={0.01} onChange={(v) => setTd({ ...td, eps_end: v })} />
          <Slider label="Seed" value={td.seed} min={1} max={100} step={1} onChange={(v) => setTd({ ...td, seed: Math.round(v) })} format={(v) => String(Math.round(v))} />
          <div className="row">
            <button className="btn primary" onClick={() => { setTrained({ grid, td }); setPhase('train'); }}>▶ Train{dirty ? ' (changes pending)' : ''}</button>
            <DownloadButton filename="gridworld-q-table.json" content={qJson} label="Q-table" small />
            <DownloadButton filename="gridworld-learning-curve.csv" content={curveCsv} label="Curve CSV" small />
          </div>
          <div className="muted small" style={{ marginTop: 6 }}>{res.ms.toFixed(0)} ms for {trained.td.episodes} episodes in WebAssembly</div>
          <ErrorBox error={res.error} />
        </Card>
        <Card
          title={phase === 'train' ? `Learning: episode ${snap?.episode ?? 0} of ${trained.td.episodes}` : 'Greedy agent after training'}
          subtitle={phase === 'train' ? `max_a Q(s,a) heat-map + greedy arrows · path of that episode · ε = ${fmt(snap?.epsilon, 3)}` : 'Following argmax Q from the start (moves still slip)'}
          actions={
            <div className="row">
              <button className={`btn small ${phase === 'train' ? 'primary' : ''}`} onClick={() => setPhase('train')}>Training</button>
              <button className={`btn small ${phase === 'rollout' ? 'primary' : ''}`} onClick={() => setPhase('rollout')}>Rollout</button>
              <Toggle label="values" value={showValues} onChange={setShowValues} />
            </div>
          }
        >
          {dirty && <div className="small" style={{ color: 'var(--warn)', marginBottom: 6 }}>! Editing — the picture shows the grid you are painting; press Train to learn on it.</div>}
          {dirty ? (
            <GridView w={w} h={h} layout={grid.layout.map((r) => r.padEnd(w, '.'))} onCell={edit} title="grid editor" />
          ) : d && (
            <GridView
              w={d.w} h={d.h} layout={trained.grid.layout.map((r) => r.padEnd(d.w, '.'))} onCell={edit}
              values={phase === 'train' ? snap?.v : d.v} policy={phase === 'train' ? snap?.policy : d.policy}
              path={phase === 'train' ? snap?.path : d.rollout} agent={phase === 'rollout' ? agent : undefined} maxAbs={maxAbs} showValues={showValues}
            />
          )}
          <div style={{ marginTop: 8 }}>
            {phase === 'train' ? <PlayBar clock={trainClock} label={() => `ep ${snap?.episode ?? 0}`} /> : <PlayBar clock={rollClock} label={(t) => `step ${Math.floor(t / 0.25)}`} />}
          </div>
          <div className="row small muted" style={{ marginTop: 6 }}>
            <span style={{ display: 'inline-block', width: 60, height: 8, borderRadius: 4, background: 'linear-gradient(90deg, color-mix(in srgb, var(--series-2) 92%, var(--surface-3)), var(--surface-3), color-mix(in srgb, var(--series-1) 92%, var(--surface-3)))' }} />
            <span>value: negative (orange) → 0 (grey) → positive (blue)</span>
          </div>
        </Card>
      </div>
      <div className="grid grid-2">
        <Card title="Learning curve" subtitle="Discounted return per episode — noisy because of ε-exploration and slippery moves">
          <Plot series={curve} height={240} xLabel="episode" yLabel="return / success" />
        </Card>
        <Card title="Ground truth: value iteration" subtitle={`Sweep ${sweep >= (d?.vi.sweeps.length ?? 0) ? `${d?.vi.iterations ?? 0} (converged)` : sweepIdx + 1} · V* heat-map with the optimal policy`}>
          {d && <GridView w={d.w} h={d.h} layout={trained.grid.layout.map((r) => r.padEnd(d.w, '.'))} values={viValues} policy={sweep >= d.vi.sweeps.length ? d.vi.policy : undefined} maxAbs={maxAbs} showValues={showValues} title="value iteration" />}
          <Slider label="Show sweep" value={Math.min(sweep, d?.vi.sweeps.length ?? 60)} min={1} max={d?.vi.sweeps.length ?? 60} step={1} onChange={(v) => setSweep(v >= (d?.vi.sweeps.length ?? 60) ? 1000 : v - 1)} format={(v) => (v >= (d?.vi.sweeps.length ?? 60) ? 'converged' : String(Math.round(v)))} />
          <Plot series={viRes} height={140} xLabel="sweep" yLabel="log₁₀ max |ΔV|" legend={false} />
        </Card>
      </div>
      <div className="grid grid-2">
        <Explainer title="Bellman equations, planned and learned" refs={[{ label: 'Sutton & Barto, Reinforcement Learning: An Introduction (2nd ed.), ch. 4 & 6', href: 'http://incompleteideas.net/book/the-book-2nd.html' }, { label: 'Watkins & Dayan, "Q-learning", Machine Learning 1992' }]}>
          <Formula>{`Value iteration:  V(s) ← max_a Σ_s' P(s'|s,a) [r + γ V(s')]
Q-learning:       Q(s,a) ← Q(s,a) + α [r + γ max_a' Q(s',a') − Q(s,a)]
SARSA:            Q(s,a) ← Q(s,a) + α [r + γ Q(s',a')        − Q(s,a)]`}</Formula>
          <p>Value iteration needs the model P(s'|s,a) and sweeps every state; Q-learning only sees sampled transitions yet converges to the same Q* (Watkins & Dayan) if every pair is visited often enough and α decays. The Russell & Norvig preset reproduces the textbook utilities (0.705 at the start).</p>
        </Explainer>
        <Explainer title="Try this">
          <ul style={{ margin: 0, paddingLeft: 18 }}>
            <li><b>Cliff walking</b>: with constant ε = 0.1, SARSA learns the safe path away from the cliff while Q-learning hugs the edge (optimal, but it falls in while exploring).</li>
            <li>Raise <b>slip</b>: the optimal policy starts avoiding corridors next to pits.</li>
            <li>Make the <b>step reward</b> very negative: the agent prefers a quick pit over a long walk.</li>
            <li>Paint walls to split the world and watch value propagate backwards from the goal over the sweeps.</li>
          </ul>
        </Explainer>
      </div>
    </div>
  );
}
