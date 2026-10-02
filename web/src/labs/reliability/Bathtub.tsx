// The bathtub curve as three competing Weibull modes, with a population of
// units that fail as the clock runs (play/pause, reduced-motion aware).
import { useMemo, useState } from 'react';
import { Card, Explainer, Formula, Slider } from '../../components/ui';
import { useElementWidth } from '../../components/useElementWidth';
import { useCompute } from '../../engine/hooks';
import type { BathtubOut } from './types';
import { MODE_COLORS, MODE_NAMES, fmt, prefersReducedMotion, useRaf } from './util';

export function Bathtub() {
  const [infantBeta, setInfantBeta] = useState(0.5);
  const [wearEta, setWearEta] = useState(30000);
  const [wearBeta, setWearBeta] = useState(5);
  const input = useMemo(() => ({ infant_beta: infantBeta, wear_eta: wearEta, wear_beta: wearBeta, horizon: 40000, units: 240 }), [infantBeta, wearEta, wearBeta]);
  const { data } = useCompute<BathtubOut>('reliability.bathtub', input);
  const [t, setT] = useState(prefersReducedMotion() ? 40000 : 0);
  const [playing, setPlaying] = useState(!prefersReducedMotion());
  useRaf(playing, (dt) =>
    setT((v) => {
      const n = v + dt * 4000;
      if (n > 40000) {
        return 0;
      }
      return n;
    }),
  );
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const W = Math.max(width, 280);
  const H = 190;
  const m = { l: 40, r: 10, t: 10, b: 26 };
  const counts = [0, 0, 0];
  data?.units.forEach((u) => {
    if (u.t <= t) counts[u.mode]++;
  });
  const alive = (data?.units.length ?? 0) - counts[0] - counts[1] - counts[2];
  const hmax = useMemo(() => {
    if (!data) return 1;
    const mid = data.h_total.slice(8);
    return Math.min(Math.max(...mid) * 1.1, (data.h_random[0] ?? 1e-5) * 12);
  }, [data]);
  const sx = (x: number) => m.l + (x / 40000) * (W - m.l - m.r);
  const sy = (h: number) => m.t + (H - m.t - m.b) * (1 - Math.min(h, hmax) / hmax);
  const path = (ys: number[]) => (data ? data.t.map((x, i) => `${i ? 'L' : 'M'}${sx(x).toFixed(1)},${sy(ys[i]).toFixed(1)}`).join('') : '');
  const cols = 24;
  return (
    <Card
      title="The bathtub curve, animated"
      subtitle="Three Weibull failure modes compete in every unit: infant mortality (β < 1), random (β = 1) and wear-out (β > 1). The total hazard is their sum."
      actions={
        <div className="row">
          <button className="btn small" onClick={() => setPlaying((p) => !p)} aria-pressed={playing}>{playing ? '❚❚ Pause' : '▶ Play'}</button>
          <button className="btn small" onClick={() => setT(0)}>↺ Reset</button>
        </div>
      }
    >
      <div className="rl-bath">
        <div ref={ref} style={{ minWidth: 0 }}>
          <svg width={W} height={H} role="img" aria-label="Hazard rate against time: the bathtub curve">
            <line x1={m.l} x2={W - m.r} y1={H - m.b} y2={H - m.b} stroke="var(--axis)" />
            <line x1={m.l} x2={m.l} y1={m.t} y2={H - m.b} stroke="var(--axis)" />
            {[0, 10000, 20000, 30000, 40000].map((x) => (
              <text key={x} x={sx(x)} y={H - 8} textAnchor="middle" className="rl-axis">{fmt(x)} h</text>
            ))}
            <text x={14} y={m.t + (H - m.t - m.b) / 2} transform={`rotate(-90 14 ${m.t + (H - m.t - m.b) / 2})`} textAnchor="middle" className="rl-axis-label">hazard h(t)</text>
            {data && (
              <>
                <path d={path(data.h_infant)} fill="none" stroke={MODE_COLORS[0]} strokeWidth={1.3} strokeDasharray="4 3" />
                <path d={path(data.h_random)} fill="none" stroke={MODE_COLORS[1]} strokeWidth={1.3} strokeDasharray="4 3" />
                <path d={path(data.h_wear)} fill="none" stroke={MODE_COLORS[2]} strokeWidth={1.3} strokeDasharray="4 3" />
                <path d={path(data.h_total)} fill="none" stroke="var(--text)" strokeWidth={2.4} />
              </>
            )}
            <line x1={sx(t)} x2={sx(t)} y1={m.t} y2={H - m.b} stroke="var(--accent)" strokeWidth={1.5} />
            <text x={Math.min(sx(t) + 4, W - 70)} y={m.t + 12} className="rl-axis" style={{ fill: 'var(--accent)' }}>t = {fmt(t)} h</text>
          </svg>
          <div className="rl-legend small">
            {MODE_NAMES.map((n, i) => (
              <span key={n}><i className="rl-line dashed" style={{ borderColor: MODE_COLORS[i] }} /> {n}</span>
            ))}
            <span><i className="rl-line" style={{ background: 'var(--text)' }} /> total hazard</span>
          </div>
        </div>
        <div>
          <svg viewBox={`0 0 ${cols * 10} ${Math.ceil((data?.units.length ?? 240) / cols) * 10}`} className="rl-pop" role="img" aria-label={`Population: ${alive} of ${data?.units.length ?? 0} units still working`}>
            {data?.units.map((u, i) => {
              const dead = u.t <= t;
              return (
                <circle
                  key={i}
                  cx={(i % cols) * 10 + 5}
                  cy={Math.floor(i / cols) * 10 + 5}
                  r={dead ? 2.2 : 3.6}
                  fill={dead ? MODE_COLORS[u.mode] : 'var(--good)'}
                  opacity={dead ? 0.9 : 0.75}
                />
              );
            })}
          </svg>
          <div className="small rl-counts">
            <span>● working {alive}</span>
            {MODE_NAMES.map((n, i) => (
              <span key={n} style={{ color: MODE_COLORS[i] }}>● {n} {counts[i]}</span>
            ))}
          </div>
        </div>
      </div>
      <div className="grid grid-3" style={{ marginTop: 10 }}>
        <Slider label="Infant-mortality shape β" value={infantBeta} min={0.2} max={0.95} step={0.05} onChange={setInfantBeta} hint="Smaller β = steeper early failures (latent assembly defects)" />
        <Slider label="Wear-out characteristic life η" value={wearEta} min={10000} max={60000} step={1000} unit="h" onChange={setWearEta} />
        <Slider label="Wear-out shape β" value={wearBeta} min={1.5} max={10} step={0.5} onChange={setWearBeta} />
      </div>
      <Explainer title="Why three regions" refs={[{ label: 'O’Connor & Kleyner, Practical Reliability Engineering, 5th ed., ch. 2' }, { label: 'Abernethy, The New Weibull Handbook, ch. 2' }]}>
        <Formula>{'h(t) = Σ βᵢ/ηᵢ · (t/ηᵢ)^(βᵢ−1)        R(t) = Π exp[−(t/ηᵢ)^βᵢ]'}</Formula>
        Early failures (β &lt; 1) come from escapes in manufacturing — burn-in and process control remove them. The flat bottom is random overstress (β = 1), where a constant
        failure rate and an MTBF make sense. Wear-out (β &gt; 1) — hinge fatigue, battery capacity fade, flex-cable cracking — sets the useful life. Real products rarely show a
        clean bathtub; each mode deserves its own Weibull, which is what the tabs of this lab do.
      </Explainer>
    </Card>
  );
}
