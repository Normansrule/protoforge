import { useMemo, useState } from 'react';
import { useCompute } from '../../engine/hooks';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat } from '../../components/ui';
import { Plot, type Series } from '../../components/Plot';
import { useElementWidth } from '../../components/useElementWidth';
import { KV, PlayBar, fmt, fmtComplex, frameAt, json, toCSV, useClock } from './kit';
import type { CartPoleResult } from './types';

interface Push { t: number; impulse: number }

function CartPoleSvg({ r, t, pushes }: { r: CartPoleResult; t: number; pushes: Push[] }) {
  const [ref, w] = useElementWidth<HTMLDivElement>();
  const W = Math.max(w, 280);
  const H = 330;
  const i = frameAt(r.t, t);
  const x = r.x[i] ?? 0;
  const th = r.theta[i] ?? 0;
  const f = r.force[i] ?? 0;
  const xr = r.xref[i] ?? 0;
  const span = 3.4;
  const s = W / span;
  const cx = (v: number) => W / 2 + v * s;
  const trackY = H - 60;
  const cartW = 0.34 * s;
  const cartH = 0.14 * s;
  const L = 0.6 * s; // pole length 2l
  const px = cx(x) + L * Math.sin(th);
  const py = trackY - cartH - L * Math.cos(th);
  const recentPush = pushes.find((p) => t >= p.t && t - p.t < 0.35);
  const fScale = Math.min(1, Math.abs(f) / 20);
  return (
    <div ref={ref} style={{ width: '100%' }}>
      <svg width={W} height={H} role="img" aria-label="Cart-pole animation" style={{ display: 'block' }}>
        <defs>
          <linearGradient id="cp-track" x1="0" x2="0" y1="0" y2="1">
            <stop offset="0" stopColor="var(--surface-3)" />
            <stop offset="1" stopColor="var(--surface-2)" />
          </linearGradient>
        </defs>
        {/* ticks */}
        {[-1.5, -1, -0.5, 0, 0.5, 1, 1.5].map((v) => (
          <g key={v}>
            <line x1={cx(v)} x2={cx(v)} y1={trackY + 8} y2={trackY + 14} stroke="var(--axis)" />
            <text x={cx(v)} y={trackY + 28} fontSize={10.5} textAnchor="middle" fill="var(--muted)" className="mono">{v} m</text>
          </g>
        ))}
        <rect x={cx(-1.6)} y={trackY} width={cx(1.6) - cx(-1.6)} height={8} rx={4} fill="url(#cp-track)" stroke="var(--border-strong)" />
        {/* set-point marker */}
        <path d={`M${cx(xr)},${trackY + 8} l-6,10 h12 z`} fill="var(--accent)" opacity={0.8} />
        {/* cart */}
        <rect x={cx(x) - cartW / 2} y={trackY - cartH} width={cartW} height={cartH} rx={6} fill="var(--surface-3)" stroke="var(--text-2)" strokeWidth={1.5} />
        {[-1, 1].map((k) => <circle key={k} cx={cx(x) + k * cartW * 0.3} cy={trackY} r={6} fill="var(--text-2)" />)}
        {/* force arrow */}
        {Math.abs(f) > 0.05 && (
          <g opacity={0.9}>
            <line x1={cx(x) - Math.sign(f) * (cartW / 2 + 4 + 60 * fScale)} x2={cx(x) - Math.sign(f) * (cartW / 2 + 4)} y1={trackY - cartH / 2} y2={trackY - cartH / 2} stroke="var(--series-2)" strokeWidth={3} markerEnd="url(#cp-arrow)" />
            <text x={cx(x) - Math.sign(f) * (cartW / 2 + 36)} y={trackY - cartH / 2 - 8} fontSize={10.5} textAnchor="middle" fill="var(--text-2)" className="mono">{f.toFixed(1)} N</text>
          </g>
        )}
        <defs>
          <marker id="cp-arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
            <path d="M0,0 L10,5 L0,10 z" fill="var(--series-2)" />
          </marker>
        </defs>
        {/* pole */}
        <line x1={cx(x)} y1={trackY - cartH} x2={px} y2={py} stroke="var(--series-1)" strokeWidth={7} strokeLinecap="round" />
        <circle cx={px} cy={py} r={9} fill="var(--series-1)" stroke="var(--surface-1)" strokeWidth={2} />
        <circle cx={cx(x)} cy={trackY - cartH} r={4} fill="var(--surface-1)" stroke="var(--text-2)" />
        {recentPush && (
          <g>
            <circle cx={px} cy={py} r={14 + 40 * (t - recentPush.t)} fill="none" stroke="var(--warn)" strokeWidth={2} opacity={1 - (t - recentPush.t) / 0.35} />
            <text x={px + (recentPush.impulse > 0 ? -30 : 30)} y={py - 14} fontSize={12} fill="var(--warn)" textAnchor="middle">push!</text>
          </g>
        )}
        <text x={12} y={20} fontSize={12} fill="var(--text-2)" className="mono">θ = {(th * 180 / Math.PI).toFixed(1)}°   x = {x.toFixed(2)} m   t = {t.toFixed(2)} s</text>
        {r.fell && r.fell_at !== null && t >= r.fell_at && <text x={W / 2} y={60} fontSize={18} fontWeight={700} textAnchor="middle" fill="var(--bad-text)">✕ fell over — more force or weights?</text>}
      </svg>
    </div>
  );
}

export function Pendulum() {
  const [q, setQ] = useState<[number, number, number, number]>([10, 1, 100, 1]);
  const [rw, setR] = useState(0.1);
  const [theta0, setTheta0] = useState(10);
  const [fMax, setFMax] = useState(20);
  const [dt, setDt] = useState(0.01);
  const [impulse, setImpulse] = useState(0.25);
  const [pushes, setPushes] = useState<Push[]>([]);
  const [refs, setRefs] = useState<[number, number][]>([]);
  const lastEvent = Math.max(0, ...pushes.map((p) => p.t), ...refs.map((r) => r[0]));
  const tEnd = Math.max(10, Math.ceil(lastEvent + 6));
  const res = useCompute<CartPoleResult>('control.cartpole', { q, r: rw, theta0_deg: theta0, f_max: fMax, dt, t_end: tEnd, pushes, x_ref: refs, fps: 100 });
  const d = res.data;
  const clock = useClock(tEnd, { loop: false, resetKey: `${q}|${rw}|${theta0}|${fMax}|${dt}` });

  const addPush = (dir: number) => {
    const t = Math.round(clock.t * 100) / 100 + 0.01;
    setPushes([...pushes, { t, impulse: dir * impulse }]);
    if (!clock.playing) clock.setPlaying(true);
  };
  const moveCart = () => {
    const cur = refs.length ? refs[refs.length - 1][1] : 0;
    setRefs([...refs, [Math.round(clock.t * 100) / 100 + 0.01, cur >= 0.4 ? -0.5 : cur <= -0.4 ? 0 : 0.5]]);
    if (!clock.playing) clock.setPlaying(true);
  };

  const now = [{ value: clock.t, color: 'var(--accent)' }];
  const pushLines = pushes.map((p) => ({ value: p.t, color: 'var(--warn)' }));
  const thS = useMemo<Series[]>(() => (d ? [{ name: 'θ (deg)', x: d.t, y: d.theta.map((v) => (v * 180) / Math.PI), color: 'var(--series-1)' }] : []), [d]);
  const xS = useMemo<Series[]>(() => (d ? [{ name: 'cart x', x: d.t, y: d.x, color: 'var(--series-3)' }, { name: 'set-point', x: d.t, y: d.xref, color: 'var(--muted)', dash: '5 4', width: 1.3, step: true }] : []), [d]);
  const fS = useMemo<Series[]>(() => (d ? [{ name: 'force F', x: d.t, y: d.force, color: 'var(--series-2)' }] : []), [d]);
  const csv = () => (d ? toCSV(['t_s', 'x_m', 'xdot_m_s', 'theta_rad', 'thetadot_rad_s', 'force_N', 'x_ref_m'], [d.t, d.x, d.xdot, d.theta, d.thetadot, d.force, d.xref]) : '');
  const gains = () => json({ K: d?.k, law: 'u = −K (x − x_ref), x = [x, ẋ, θ, θ̇]', Q_diag: q, R: rw, dt_s: dt, A: d?.a, B: d?.b, P: d?.p, closed_loop_poles: d?.closed_poles });

  return (
    <div className="stack">
      <div className="grid grid-4">
        <Stat label="Outcome" value={d ? (d.fell ? '✕ fell' : '✓ balanced') : '—'} tone={d ? (d.fell ? 'fail' : 'pass') : undefined} note={d?.fell ? `at t = ${fmt(d.fell_at, 2)} s` : `max |x| ${fmt(d?.max_abs_x, 2)} m`} />
        <Stat label="Peak force" value={fmt(d?.max_abs_force, 1)} unit="N" note={`saturated ${fmt((d?.saturated_frac ?? 0) * 100, 0)} % of steps`} />
        <Stat label="Open-loop unstable pole" value={fmt(Math.max(...(d?.open_poles.map((p) => p.re) ?? [0])), 2)} unit="1/s" note="time-to-double ≈ ln2/λ" />
        <Stat label="LQR cost J" value={fmt(d?.cost, 2)} note="Σ (xᵀQx + Ru²) Δt" />
      </div>
      <div className="grid grid-sidebar">
        <Card title="LQR design">
          <Slider label="Q · cart position" value={q[0]} min={0.01} max={1000} log onChange={(v) => setQ([v, q[1], q[2], q[3]])} />
          <Slider label="Q · cart velocity" value={q[1]} min={0.01} max={1000} log onChange={(v) => setQ([q[0], v, q[2], q[3]])} />
          <Slider label="Q · pole angle" value={q[2]} min={0.01} max={10000} log onChange={(v) => setQ([q[0], q[1], v, q[3]])} />
          <Slider label="Q · pole rate" value={q[3]} min={0.01} max={1000} log onChange={(v) => setQ([q[0], q[1], q[2], v])} />
          <Slider label="R · force" value={rw} min={0.0005} max={100} log onChange={setR} />
          <Slider label="Initial tilt θ₀" value={theta0} min={-40} max={40} step={0.5} unit="°" onChange={setTheta0} />
          <Slider label="Force limit ±F" value={fMax} min={1} max={100} log unit="N" onChange={setFMax} />
          <Slider label="Controller period" value={dt} min={0.002} max={0.08} log format={(v) => (v * 1000).toFixed(1) + ' ms'} onChange={setDt} />
          <Slider label="Push impulse" value={impulse} min={0.02} max={1} log unit="N·s" onChange={setImpulse} />
          <KV rows={[
            ['K (x, ẋ, θ, θ̇)', d ? d.k.map((v) => v.toFixed(2)).join(', ') : '—'],
            ['closed-loop poles', d ? d.closed_poles.map((z) => fmtComplex(z, 2)).join(', ') : '—'],
          ]} />
          <ErrorBox error={res.error} />
        </Card>
        <Card
          title="Nonlinear cart–pole under LQR"
          subtitle="Integrated with RK4 at 1 kHz; the controller runs at the chosen period with a zero-order hold"
          actions={<><DownloadButton filename="cartpole-trajectory.csv" content={csv} label="CSV" small /><DownloadButton filename="lqr-gains.json" content={gains} label="Gains" small /></>}
        >
          {d && <CartPoleSvg r={d} t={clock.t} pushes={pushes} />}
          <div className="row" style={{ margin: '8px 0' }}>
            <button className="btn" onClick={() => addPush(-1)}>← Push pole</button>
            <button className="btn" onClick={() => addPush(1)}>Push pole →</button>
            <button className="btn" onClick={moveCart}>Move cart set-point</button>
            <button className="btn ghost" onClick={() => { setPushes([]); setRefs([]); clock.restart(); }}>Reset</button>
          </div>
          <PlayBar clock={clock} />
        </Card>
      </div>
      <div className="grid grid-3">
        <Card title="Pole angle"><Plot series={thS} height={190} xLabel="t (s)" yLabel="θ (°)" legend={false} vLines={[...pushLines, ...now]} hLines={[{ value: 0 }]} /></Card>
        <Card title="Cart position"><Plot series={xS} height={190} xLabel="t (s)" yLabel="x (m)" vLines={now} /></Card>
        <Card title="Control force"><Plot series={fS} height={190} xLabel="t (s)" yLabel="F (N)" legend={false} vLines={now} hLines={[{ value: fMax, label: 'limit', color: 'var(--bad)' }, { value: -fMax, color: 'var(--bad)' }]} /></Card>
      </div>
      <div className="grid grid-2">
        <Explainer title="Linear-quadratic regulator" refs={[{ label: 'Åström & Murray, Feedback Systems (state feedback & LQR)', href: 'https://fbswiki.org' }, { label: 'Anderson & Moore, Optimal Control: Linear Quadratic Methods (1990)' }]}>
          <p>Linearise the cart–pole about upright, discretise at the controller period (ZOH, <span className="mono">A_d = e^(AΔt)</span>) and choose the state feedback that minimises</p>
          <Formula>{`J = Σ (xₖᵀ Q xₖ + uₖᵀ R uₖ)
P = Q + AᵀPA − AᵀPB (R + BᵀPB)⁻¹ BᵀPA     (discrete Riccati)
K = (R + BᵀPB)⁻¹ BᵀPA,   u = −K x`}</Formula>
          <p>The Riccati equation is solved with the structure-preserving doubling algorithm (quadratic convergence). Raising Q on θ makes the pole stiffer; raising R makes the controller gentler — and eventually too weak to catch a push under the force limit. Notice the cart first moves the <i>wrong</i> way when you change its set-point: the pendulum is non-minimum-phase in x.</p>
        </Explainer>
        <Explainer title="The nonlinear model" refs={[{ label: 'CTMS inverted pendulum parameters (M = 0.5 kg, m = 0.2 kg, l = 0.3 m)' }]}>
          <Formula>{`(M+m) ẍ + m l cosθ θ̈ − m l sinθ θ̇² = F − b ẋ
m l cosθ ẍ + (I + m l²) θ̈ − m g l sinθ = 0`}</Formula>
          <p>The controller was designed on the linear model but here drives the full nonlinear dynamics with saturation — so large initial tilts or big pushes can still drop the pole. A push is a horizontal impulse at the tip: it changes ẋ and θ̇ instantly via the mass matrix. On Iris the same state-space thinking powers display stabilisation: estimate the head state (Head-tracking tab), then act on it.</p>
        </Explainer>
      </div>
    </div>
  );
}
