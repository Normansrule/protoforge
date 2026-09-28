import { useMemo, useState } from 'react';
import { useCompute } from '../../engine/hooks';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat, Toggle } from '../../components/ui';
import { Plot, type Series } from '../../components/Plot';
import { PlayBar, fmt, frameAt, toCSV, useClock } from '../control/kit';
import type { JointMove, LineOut, ProfileSamples } from './types';

const COLORS = ['var(--series-1)', 'var(--series-2)', 'var(--series-3)'];

const LINE_PRESETS: Record<string, { from: [number, number]; to: [number, number]; label: string }> = {
  easy: { from: [1.2, -0.6], to: [0.4, 1.0], label: 'Across the workspace' },
  sing: { from: [1.6, -0.35], to: [1.35, 0.9], label: 'Skimming the outer singularity' },
  inner: { from: [0.7, -0.2], to: [0.1, 0.62], label: 'Near the inner boundary' },
};

export function Trajectories() {
  const [dist, setDist] = useState(90);
  const [vmax, setVmax] = useState(120);
  const [amax, setAmax] = useState(400);
  const [same, setSame] = useState(false);
  const prof = useCompute<ProfileSamples[]>('robotics.profiles', { distance: dist, v_max: vmax, a_max: amax, same_duration: same, samples: 400 });
  const P = prof.data ?? [];
  const mk = (k: 'p' | 'v' | 'a' | 'j'): Series[] => P.map((p, i) => ({ name: `${p.name} (${p.duration.toFixed(2)} s)`, x: p.t, y: p[k], color: COLORS[i], width: 2 }));
  const pS = useMemo(() => mk('p'), [prof.data]); // eslint-disable-line react-hooks/exhaustive-deps
  const vS = useMemo(() => mk('v'), [prof.data]); // eslint-disable-line react-hooks/exhaustive-deps
  const aS = useMemo(() => mk('a'), [prof.data]); // eslint-disable-line react-hooks/exhaustive-deps
  const jS = useMemo(() => mk('j').slice(1), [prof.data]); // eslint-disable-line react-hooks/exhaustive-deps

  // synchronised 2-joint move
  const [g1, setG1] = useState(120);
  const [g2, setG2] = useState(-40);
  const jm = useCompute<JointMove>('robotics.joint_move', { q_start: [0, 0], q_goal: [g1, g2], v_max: [180, 240], a_max: [720, 960], samples: 240 });
  const jmS = useMemo<Series[]>(() => (jm.data ? [
    { name: 'J1 angle', x: jm.data.t, y: jm.data.q[0], color: 'var(--series-1)' },
    { name: 'J2 angle', x: jm.data.t, y: jm.data.q[1], color: 'var(--series-2)' },
  ] : []), [jm.data]);
  const jmV = useMemo<Series[]>(() => (jm.data ? [
    { name: 'J1 speed', x: jm.data.t, y: jm.data.qd[0], color: 'var(--series-1)' },
    { name: 'J2 speed', x: jm.data.t, y: jm.data.qd[1], color: 'var(--series-2)' },
  ] : []), [jm.data]);

  // Cartesian straight line vs joint interpolation
  const [preset, setPreset] = useState('easy');
  const pr = LINE_PRESETS[preset];
  const line = useCompute<LineOut>('robotics.cartesian_line', { arm: { l1: 1, l2: 0.7, q1_min: -180, q1_max: 180, q2_min: -180, q2_max: 180 }, from: pr.from, to: pr.to, v_max: 0.8, a_max: 2.0, elbow_up: false, samples: 200 });
  const L = line.data;
  const clock = useClock(L?.duration ?? 1, { resetKey: preset });
  const qdS = useMemo<Series[]>(() => (L ? [
    { name: 'q̇1', x: L.t, y: L.q1d, color: 'var(--series-1)' },
    { name: 'q̇2', x: L.t, y: L.q2d, color: 'var(--series-2)' },
  ] : []), [L]);
  const k = L ? frameAt(L.t, clock.t) : 0;
  const R = 1.9;

  const csv = () => {
    const cols: number[][] = [];
    const head: string[] = [];
    P.forEach((p) => { head.push(`${p.name}_t`, `${p.name}_pos`, `${p.name}_vel`, `${p.name}_acc`); cols.push(p.t, p.p, p.v, p.a); });
    return toCSV(head, cols);
  };

  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Point-to-point move">
          <Slider label="Distance" value={dist} min={5} max={360} step={1} unit="°" onChange={setDist} />
          <Slider label="Velocity limit" value={vmax} min={10} max={720} log unit="°/s" onChange={setVmax} />
          <Slider label="Acceleration limit" value={amax} min={20} max={5000} log unit="°/s²" onChange={setAmax} />
          <Toggle label="Force the same duration for all three" value={same} onChange={setSame} />
          <table className="table" style={{ fontSize: 12.5 }}>
            <thead><tr><th>Profile</th><th>T (s)</th><th>peak v</th><th>peak a</th></tr></thead>
            <tbody>
              {P.map((p, i) => (
                <tr key={p.name}>
                  <td><span style={{ display: 'inline-block', width: 10, height: 3, background: COLORS[i], marginRight: 6, verticalAlign: 'middle' }} />{p.name}</td>
                  <td className="mono">{p.duration.toFixed(3)}</td>
                  <td className="mono" style={{ color: p.peak_v > vmax * 1.001 ? 'var(--bad-text)' : undefined }}>{p.peak_v.toFixed(0)}</td>
                  <td className="mono" style={{ color: p.peak_a > amax * 1.001 ? 'var(--bad-text)' : undefined }}>{p.peak_a.toFixed(0)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="small muted">{same ? 'Same T: red values break the limits — polynomials need more time for the same peak speed.' : 'Each profile gets its own minimum time under the same limits.'}</p>
          <DownloadButton filename="profiles.csv" content={csv} label="Profiles CSV" small />
          <ErrorBox error={prof.error} />
        </Card>
        <div className="grid grid-2">
          <Card title="Position"><Plot series={pS} height={200} xLabel="t (s)" yLabel="q (°)" legend={false} /></Card>
          <Card title="Velocity"><Plot series={vS} height={200} xLabel="t (s)" yLabel="q̇ (°/s)" hLines={[{ value: vmax, label: 'limit', color: 'var(--bad)' }]} legend={false} /></Card>
          <Card title="Acceleration"><Plot series={aS} height={200} xLabel="t (s)" yLabel="q̈ (°/s²)" hLines={[{ value: amax, label: 'limit', color: 'var(--bad)' }, { value: -amax, color: 'var(--bad)' }]} legend={false} /></Card>
          <Card title="Jerk (cubic & quintic)" subtitle="The trapezoid's jerk is infinite at every corner"><Plot series={jS} height={200} xLabel="t (s)" yLabel="jerk (°/s³)" legend={false} /></Card>
          <div className="plot-legend" style={{ gridColumn: '1 / -1', paddingLeft: 4 }}>
            {P.map((p, i) => <span className="lg" key={p.name}><span className="lg-swatch" style={{ background: COLORS[i] }} />{p.name} — {p.duration.toFixed(2)} s</span>)}
          </div>
        </div>
      </div>

      <div className="grid grid-2">
        <Card title="Synchronised two-joint move" subtitle={jm.data ? `Both joints arrive together at ${jm.data.duration.toFixed(3)} s (alone: J1 ${jm.data.own_min_time[0].toFixed(3)} s, J2 ${jm.data.own_min_time[1].toFixed(3)} s)` : ''}>
          <div className="grid grid-2" style={{ gap: 8 }}>
            <Slider label="J1 goal" value={g1} min={-180} max={180} step={1} unit="°" onChange={setG1} />
            <Slider label="J2 goal" value={g2} min={-180} max={180} step={1} unit="°" onChange={setG2} />
          </div>
          <Plot series={jmS} height={170} xLabel="t (s)" yLabel="angle (°)" />
          <Plot series={jmV} height={150} xLabel="t (s)" yLabel="speed (°/s)" legend={false} />
          <p className="small muted">Limits: J1 180 °/s, 720 °/s² · J2 240 °/s, 960 °/s². The faster joint solves D = v·T − v²/a for a slower cruise speed so the tool moves along one coordinated path — exactly what the SCARA does in the pick-and-place tab.</p>
        </Card>
        <Card title="Straight line in Cartesian space vs joint space" actions={<Select value={preset} onChange={setPreset} options={Object.entries(LINE_PRESETS).map(([k2, v]) => ({ value: k2, label: v.label }))} />}>
          <svg viewBox={`${-R} ${-R} ${2 * R} ${2 * R}`} style={{ width: '100%', maxHeight: 280, display: 'block' }} role="img" aria-label="Cartesian vs joint interpolated path">
            <g transform="scale(1,-1)">
              <circle r={1.7} fill="var(--accent-soft)" stroke="var(--border-strong)" strokeWidth={0.006} />
              <circle r={0.3} fill="var(--surface-1)" stroke="var(--border-strong)" strokeWidth={0.006} />
              {L && <polyline points={L.joint_x.map((x, i) => `${x},${L.joint_y[i]}`).join(' ')} fill="none" stroke="var(--series-4)" strokeWidth={0.018} strokeDasharray="0.05 0.04" />}
              {L && <polyline points={L.x.map((x, i) => `${x},${L.y[i]}`).join(' ')} fill="none" stroke="var(--series-1)" strokeWidth={0.02} />}
              {L && L.q1.length > 0 && (() => {
                const q1 = (L.q1[k] * Math.PI) / 180, q2 = (L.q2[k] * Math.PI) / 180;
                const e = [Math.cos(q1), Math.sin(q1)];
                const tip = [e[0] + 0.7 * Math.cos(q1 + q2), e[1] + 0.7 * Math.sin(q1 + q2)];
                return (
                  <g>
                    <line x1={0} y1={0} x2={e[0]} y2={e[1]} stroke="var(--text-2)" strokeWidth={0.05} strokeLinecap="round" />
                    <line x1={e[0]} y1={e[1]} x2={tip[0]} y2={tip[1]} stroke="var(--text-2)" strokeWidth={0.04} strokeLinecap="round" />
                    <circle cx={tip[0]} cy={tip[1]} r={0.035} fill="var(--series-1)" />
                    <circle r={0.06} fill="var(--text-2)" />
                  </g>
                );
              })()}
            </g>
          </svg>
          <div className="plot-legend" style={{ paddingLeft: 0 }}>
            <span className="lg"><span className="lg-swatch" style={{ background: 'var(--series-1)' }} />Cartesian line (IK at every sample)</span>
            <span className="lg"><span className="lg-swatch" style={{ background: 'var(--series-4)' }} />joint-space interpolation (curved tool path)</span>
          </div>
          <PlayBar clock={clock} />
          <Plot series={qdS} height={150} xLabel="t (s)" yLabel="joint speed (°/s)" vLines={[{ value: clock.t, color: 'var(--accent)' }]} />
          <Stat label="Peak joint speed on the Cartesian line" value={fmt(L?.peak_joint_speed_dps, 0)} unit="°/s" note="for a tool speed of only 0.8 m/s — near a singularity joint speeds explode" />
        </Card>
      </div>
      <div className="grid grid-2">
        <Explainer title="Trapezoid, cubic, quintic" refs={[{ label: 'Biagiotti & Melchiorri, Trajectory Planning for Automatic Machines and Robots (2008)' }, { label: 'Lynch & Park, Modern Robotics, ch. 9', href: 'http://modernrobotics.org' }]}>
          <Formula>{`Trapezoid: T = D/v + v/a  (triangular if D < v²/a, v_peak = √(aD))
Cubic:     q(τ) = D(3τ² − 2τ³)             v̂ = 1.5 D/T,   â = 6 D/T²
Quintic:   q(τ) = D(10τ³ − 15τ⁴ + 6τ⁵)     v̂ = 1.875 D/T, â = 5.77 D/T²`}</Formula>
          <p>The trapezoid is time-optimal for velocity and acceleration limits but has step changes in acceleration (infinite jerk) that shake a light arm. The quintic has zero velocity <i>and</i> acceleration at both ends — smooth, but ~25 % slower for the same limits. Real controllers blend them (S-curves with a jerk limit).</p>
        </Explainer>
        <Explainer title="Why the tool path matters">
          Interpolating in joint space is cheap and always feasible but the tool traces a curve; a straight Cartesian line needs IK at every sample and can demand huge joint speeds near singularities (pick the “outer singularity” preset). Pick-and-place heads therefore move in joint space between points and only go straight in Z.
        </Explainer>
      </div>
    </div>
  );
}
