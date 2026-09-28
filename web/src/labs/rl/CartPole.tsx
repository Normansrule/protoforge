import { useMemo, useState } from 'react';
import { useCompute } from '../../engine/hooks';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat } from '../../components/ui';
import { Plot, type Series } from '../../components/Plot';
import { useElementWidth } from '../../components/useElementWidth';
import { PlayBar, fmt, json, mix, toCSV, useClock } from '../control/kit';
import type { CartPoleRlResult } from './types';

interface Cfg { episodes: number; bins: [number, number, number, number]; gamma: number; seed: number; alpha_min: number; eps_min: number; decay_k: number }
const DEFAULT: Cfg = { episodes: 600, bins: [1, 3, 6, 12], gamma: 0.99, seed: 3, alpha_min: 0.1, eps_min: 0.02, decay_k: 25 };
const TAU = 0.02;

function Replay({ frame, step }: { frame: [number, number, number, number, number] | undefined; step: number }) {
  const [ref, w] = useElementWidth<HTMLDivElement>();
  const W = Math.max(w, 260);
  const H = 230;
  const s = W / 5.6;
  const cx = (x: number) => W / 2 + x * s;
  const ty = H - 50;
  const x = frame?.[0] ?? 0;
  const th = frame?.[2] ?? 0;
  const a = frame?.[4] ?? 0;
  const L = 1.0 * s; // pole length 2 × 0.5 m
  return (
    <div ref={ref}>
      <svg width={W} height={H} role="img" aria-label="CartPole replay" style={{ display: 'block' }}>
        <rect x={cx(-2.4)} y={ty} width={cx(2.4) - cx(-2.4)} height={6} rx={3} fill="var(--surface-3)" />
        {[-2.4, 2.4].map((v) => <line key={v} x1={cx(v)} x2={cx(v)} y1={ty - 30} y2={ty + 14} stroke="var(--bad)" strokeDasharray="4 3" />)}
        <text x={cx(-2.4)} y={ty + 28} fontSize={10.5} fill="var(--muted)" textAnchor="middle" className="mono">−2.4 m</text>
        <text x={cx(2.4)} y={ty + 28} fontSize={10.5} fill="var(--muted)" textAnchor="middle" className="mono">+2.4 m</text>
        {/* ±12° failure wedge */}
        <path d={`M${cx(x)},${ty - 14} L${cx(x) + L * Math.sin(-0.2094)},${ty - 14 - L * Math.cos(0.2094)} A ${L} ${L} 0 0 1 ${cx(x) + L * Math.sin(0.2094)},${ty - 14 - L * Math.cos(0.2094)} Z`} fill="var(--accent-soft)" />
        <rect x={cx(x) - 0.25 * s} y={ty - 14} width={0.5 * s} height={16} rx={4} fill="var(--surface-3)" stroke="var(--text-2)" />
        <line x1={cx(x)} y1={ty - 14} x2={cx(x) + L * Math.sin(th)} y2={ty - 14 - L * Math.cos(th)} stroke="var(--series-1)" strokeWidth={6} strokeLinecap="round" />
        <path d={a > 0.5 ? `M${cx(x) + 0.25 * s + 6},${ty - 6} l14,0 l-5,-5 m5,5 l-5,5` : `M${cx(x) - 0.25 * s - 6},${ty - 6} l-14,0 l5,-5 m-5,5 l5,5`} stroke="var(--series-2)" strokeWidth={2.5} fill="none" />
        <text x={10} y={18} fontSize={12} fill="var(--text-2)" className="mono">step {step} · θ = {((th * 180) / Math.PI).toFixed(1)}° · x = {x.toFixed(2)} m · push {a > 0.5 ? 'right' : 'left'}</text>
      </svg>
    </div>
  );
}

export function CartPole() {
  const [cfg, setCfg] = useState<Cfg>(DEFAULT);
  const [trained, setTrained] = useState<Cfg>(DEFAULT);
  const res = useCompute<CartPoleRlResult>('rl.cartpole', { ...trained, max_steps: 500 });
  const d = res.data;
  const n = d?.replay.length ?? 1;
  const clock = useClock(Math.max(0.1, n * TAU), { loop: true, resetKey: d });
  const k = Math.min(n - 1, Math.floor(clock.t / TAU));
  const dirty = JSON.stringify(cfg) !== JSON.stringify(trained);

  const curve = useMemo<Series[]>(() => (d ? [
    { name: 'steps balanced (training, ε-greedy)', x: d.returns.map((_, i) => i + 1), y: d.returns, color: 'var(--series-1)', width: 0.8 },
    { name: '50-episode average', x: d.moving_avg.map((_, i) => i + 1), y: d.moving_avg, color: 'var(--series-2)', width: 2.2 },
    { name: 'greedy evaluation', x: d.greedy_curve.map((p) => p[0]), y: d.greedy_curve.map((p) => p[1]), color: 'var(--series-3)', scatter: true },
  ] : []), [d]);
  const thS = useMemo<Series[]>(() => (d ? [{ name: 'θ (°)', x: d.replay.map((_, i) => i * TAU), y: d.replay.map((f) => (f[2] * 180) / Math.PI), color: 'var(--series-1)' }, { name: 'x (m ×10)', x: d.replay.map((_, i) => i * TAU), y: d.replay.map((f) => f[0] * 10), color: 'var(--series-3)' }] : []), [d]);

  // policy map over (θ, θ̇) bins for the middle x/ẋ bins
  const [nx, nxd, nth, nthd] = d?.bins ?? [1, 1, 6, 12];
  const polCells = useMemo(() => {
    if (!d) return [];
    const ix = Math.floor(nx / 2), ixd = Math.floor(nxd / 2);
    const out: { i: number; j: number; a: number; conf: number; visited: boolean }[] = [];
    let maxDiff = 1e-9;
    for (let i = 0; i < nth; i++) for (let j = 0; j < nthd; j++) {
      const idx = ((ix * nxd + ixd) * nth + i) * nthd + j;
      const q = d.q[idx];
      maxDiff = Math.max(maxDiff, Math.abs(q[1] - q[0]));
    }
    for (let i = 0; i < nth; i++) for (let j = 0; j < nthd; j++) {
      const idx = ((ix * nxd + ixd) * nth + i) * nthd + j;
      const q = d.q[idx];
      out.push({ i, j, a: q[1] > q[0] ? 1 : 0, conf: Math.abs(q[1] - q[0]) / maxDiff, visited: q[0] !== 0 || q[1] !== 0 });
    }
    return out;
  }, [d, nx, nxd, nth, nthd]);

  const qJson = () => json({ env: 'CartPole-v1 physics (Gymnasium)', bins: d?.bins, bounds: { x: 2.4, x_dot: 0.5, theta_rad: 0.2094, theta_dot_rad_s: 0.8727 }, actions: ['push left', 'push right'], index: '((((ix·n_xd)+ixd)·n_th)+ith)·n_thd + ithd', q: d?.q, config: trained });
  const csv = () => (d ? toCSV(['episode', 'steps', 'moving_avg_50'], [d.returns.map((_, i) => i + 1), d.returns, d.moving_avg]) : '');

  return (
    <div className="stack">
      <div className="grid grid-4">
        <Stat label="Best greedy (checkpoint)" value={fmt(d?.best_greedy, 0)} unit="/ 500 steps" tone={d ? (d.best_greedy >= 475 ? 'pass' : undefined) : undefined} note={`episode ${d?.best_episode ?? '—'} · final table ${fmt(d?.final_greedy, 0)} · ≥ 475 counts as solved`} />
        <Stat label="Replay length" value={d?.replay_len ?? '—'} unit="steps" note={`${fmt((d?.replay_len ?? 0) * TAU, 1)} s of balancing`} />
        <Stat label="Last 100 episodes" value={d ? fmt(d.returns.slice(-100).reduce((a, b) => a + b, 0) / Math.min(100, d.returns.length), 0) : '—'} unit="avg steps" note="with exploration still on" />
        <Stat label="Q-table" value={d?.states ?? '—'} unit="states × 2" note={`${res.ms.toFixed(0)} ms to train in WebAssembly`} />
      </div>
      <div className="grid grid-sidebar">
        <Card title="Discretised Q-learning">
          <Slider label="Episodes" value={cfg.episodes} min={50} max={4000} log step={1} format={(v) => String(Math.round(v))} onChange={(v) => setCfg({ ...cfg, episodes: Math.round(v) })} />
          <Slider label="Bins: cart position" value={cfg.bins[0]} min={1} max={6} step={1} format={(v) => String(Math.round(v))} onChange={(v) => setCfg({ ...cfg, bins: [Math.round(v), cfg.bins[1], cfg.bins[2], cfg.bins[3]] })} />
          <Slider label="Bins: cart velocity" value={cfg.bins[1]} min={1} max={6} step={1} format={(v) => String(Math.round(v))} onChange={(v) => setCfg({ ...cfg, bins: [cfg.bins[0], Math.round(v), cfg.bins[2], cfg.bins[3]] })} />
          <Slider label="Bins: pole angle" value={cfg.bins[2]} min={2} max={16} step={1} format={(v) => String(Math.round(v))} onChange={(v) => setCfg({ ...cfg, bins: [cfg.bins[0], cfg.bins[1], Math.round(v), cfg.bins[3]] })} />
          <Slider label="Bins: pole rate" value={cfg.bins[3]} min={2} max={24} step={1} format={(v) => String(Math.round(v))} onChange={(v) => setCfg({ ...cfg, bins: [cfg.bins[0], cfg.bins[1], cfg.bins[2], Math.round(v)] })} />
          <Slider label="Discount γ" value={cfg.gamma} min={0.8} max={1} step={0.001} onChange={(v) => setCfg({ ...cfg, gamma: v })} />
          <Slider label="Decay scale k (episodes)" value={cfg.decay_k} min={5} max={200} log onChange={(v) => setCfg({ ...cfg, decay_k: v })} />
          <Slider label="Seed" value={cfg.seed} min={1} max={50} step={1} format={(v) => String(Math.round(v))} onChange={(v) => setCfg({ ...cfg, seed: Math.round(v) })} />
          <div className="row">
            <button className="btn primary" onClick={() => setTrained(cfg)}>▶ Train{dirty ? ' (changes pending)' : ''}</button>
            <DownloadButton filename="cartpole-q-table.json" content={qJson} label="Q-table" small />
            <DownloadButton filename="cartpole-learning-curve.csv" content={csv} label="Curve CSV" small />
          </div>
          <ErrorBox error={res.error} />
        </Card>
        <Card title="Greedy replay of the best checkpoint" subtitle="Same physics as Gymnasium CartPole-v1: ±10 N pushes every 20 ms, fail beyond ±12° or ±2.4 m">
          <Replay frame={d?.replay[k]} step={k} />
          <PlayBar clock={clock} speeds={[0.25, 0.5, 1, 2, 4]} />
          <Plot series={thS} height={150} xLabel="t (s)" yLabel="θ (°), x (dm)" vLines={[{ value: clock.t, color: 'var(--accent)' }]} />
        </Card>
      </div>
      <div className="grid grid-2">
        <Card title="Learning curve"><Plot series={curve} height={250} xLabel="episode" yLabel="steps balanced" hLines={[{ value: 500, label: 'episode cap', color: 'var(--muted)' }]} /></Card>
        <Card title="Learned policy over pole angle × pole rate" subtitle="Blue = push right, orange = push left; saturation = how strongly Q prefers it (middle cart bins)">
          <svg viewBox={`0 0 ${nthd * 30 + 40} ${nth * 30 + 36}`} style={{ width: '100%', maxHeight: 280, display: 'block' }} role="img" aria-label="policy map">
            {polCells.map((c) => (
              <g key={`${c.i}-${c.j}`}>
                <title>{`θ bin ${c.i}, θ̇ bin ${c.j}: ${c.a ? 'right' : 'left'}${c.visited ? '' : ' (never visited)'}`}</title>
                <rect x={40 + c.j * 30} y={(nth - 1 - c.i) * 30} width={28} height={28} rx={4} fill={c.visited ? mix(c.a ? 'var(--series-1)' : 'var(--series-2)', 'var(--surface-2)', 25 + 70 * c.conf) : 'var(--surface-2)'} />
                {c.visited && <text x={40 + c.j * 30 + 14} y={(nth - 1 - c.i) * 30 + 19} fontSize={13} textAnchor="middle" fill="var(--text)">{c.a ? '→' : '←'}</text>}
              </g>
            ))}
            <text x={40 + (nthd * 30) / 2} y={nth * 30 + 22} fontSize={11} textAnchor="middle" fill="var(--muted)">pole rate θ̇ → (−50 … +50 °/s)</text>
            <text x={12} y={(nth * 30) / 2} fontSize={11} textAnchor="middle" fill="var(--muted)" transform={`rotate(-90 12 ${(nth * 30) / 2})`}>pole angle θ ↑</text>
          </svg>
        </Card>
      </div>
      <div className="grid grid-2">
        <Explainer title="The benchmark" refs={[{ label: 'Barto, Sutton & Anderson, IEEE Trans. SMC 13 (1983)' }, { label: 'Gymnasium CartPole-v1', href: 'https://gymnasium.farama.org' }]}>
          <Formula>{`temp = (F + m l θ̇² sinθ) / (M + m)
θ̈ = (g sinθ − cosθ · temp) / (l (4/3 − m cos²θ / (M + m)))
ẍ = temp − m l θ̈ cosθ / (M + m)          (Euler, τ = 0.02 s)`}</Formula>
          <p>Barto et al. split the state into 162 "boxes" and learned with an actor–critic; here each box holds two Q-values updated by Q-learning, with learning and exploration rates decaying as 1 − log₁₀((e+1)/k).</p>
        </Explainer>
        <Explainer title="Tabular RL vs the LQR tab">
          The Control Lab balances a pole with LQR using the <i>model</i>; here the agent never sees the equations and learns only from +1 per surviving step. It needs hundreds of episodes and a coarse table — which is why real systems graduate to function approximation (DQN, PPO in Stable-Baselines3) or, when a model exists, to optimal control.
        </Explainer>
      </div>
    </div>
  );
}
