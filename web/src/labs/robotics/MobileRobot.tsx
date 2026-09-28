import { useMemo, useRef, useState } from 'react';
import { useCompute } from '../../engine/hooks';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat, Toggle } from '../../components/ui';
import { Plot, type Series } from '../../components/Plot';
import { PlayBar, fmt, frameAt, toCSV, useClock } from '../control/kit';
import type { PursuitOut } from './types';

const DEFAULT_WP: [number, number][] = [[0, 0], [2, -0.4], [3.6, 0.6], [3.4, 2.4], [1.8, 2.2], [0.8, 3.2], [-1, 2.8], [-1.6, 1]];

export function MobileRobot() {
  const [wp, setWp] = useState<[number, number][]>(DEFAULT_WP);
  const [la, setLa] = useState(0.5);
  const [v, setV] = useState(0.6);
  const [b, setB] = useState(0.25);
  const [vmax, setVmax] = useState(0.9);
  const [offY, setOffY] = useState(-0.4);
  const [offTh, setOffTh] = useState(20);
  const [slow, setSlow] = useState(false);
  const [drag, setDrag] = useState<number | null>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const res = useCompute<PursuitOut>('robotics.pursuit', { waypoints: wp, lookahead: la, speed: v, wheelbase: b, wheel_vmax: vmax, start_offset: [0, offY, offTh], a_lat_max: slow ? 0.6 : 0, laps: 1, dt: 0.02 });
  const d = res.data;
  const clock = useClock(d ? d.t[d.t.length - 1] : 1, { speed: 2, resetKey: JSON.stringify([wp, la, v, b, vmax, offY, offTh, slow]) });
  const k = d ? frameAt(d.t, clock.t) : 0;

  const xs = [...wp.map((p) => p[0]), ...(d?.path_x ?? [])];
  const ys = [...wp.map((p) => p[1]), ...(d?.path_y ?? [])];
  const x0 = Math.min(...xs) - 0.8, x1 = Math.max(...xs) + 0.8, y0 = Math.min(...ys) - 0.8, y1 = Math.max(...ys) + 0.8;

  const toWorld = (e: React.PointerEvent): [number, number] => {
    const svg = svgRef.current!;
    const pt = svg.createSVGPoint();
    pt.x = e.clientX;
    pt.y = e.clientY;
    const p = pt.matrixTransform(svg.getScreenCTM()!.inverse());
    return [Math.round(p.x * 100) / 100, Math.round(-p.y * 100) / 100];
  };

  const cte = useMemo<Series[]>(() => (d ? [{ name: 'cross-track error', x: d.t, y: d.cross_track, color: 'var(--series-1)' }] : []), [d]);
  const wheels = useMemo<Series[]>(() => (d ? [
    { name: 'left wheel', x: d.t, y: d.v_left, color: 'var(--series-2)' },
    { name: 'right wheel', x: d.t, y: d.v_right, color: 'var(--series-3)' },
  ] : []), [d]);
  const csv = () => (d ? toCSV(['t_s', 'x_m', 'y_m', 'theta_rad', 'goal_x', 'goal_y', 'v_left', 'v_right', 'curvature', 'cross_track_m'], [d.t, d.x, d.y, d.theta, d.goal_x, d.goal_y, d.v_left, d.v_right, d.curvature, d.cross_track]) : '');

  const rx = d?.x[k] ?? 0, ry = d?.y[k] ?? 0, th = d?.theta[k] ?? 0;
  const deg = (th * 180) / Math.PI;
  const trail = d ? d.x.slice(0, k + 1).map((x, i) => `${x},${d.y[i]}`).join(' ') : '';

  return (
    <div className="stack">
      <div className="grid grid-4">
        <Stat label="RMS cross-track" value={fmt((d?.rms_cross_track ?? NaN) * 100, 1)} unit="cm" note="after the first second" />
        <Stat label="Max cross-track" value={fmt((d?.max_cross_track ?? NaN) * 100, 1)} unit="cm" />
        <Stat label="Lap time" value={fmt(d?.lap_time, 1)} unit="s" note={`path length ${fmt(d?.path_len, 2)} m`} />
        <Stat label="Now" value={d ? `${(d.curvature[k] ?? 0).toFixed(2)}` : '—'} unit="1/m κ" note={d ? `wheels ${d.v_left[k].toFixed(2)} / ${d.v_right[k].toFixed(2)} m/s` : ''} />
      </div>
      <div className="grid grid-sidebar">
        <Card title="Robot & controller">
          <Slider label="Lookahead distance L" value={la} min={0.1} max={2} step={0.01} unit="m" onChange={setLa} />
          <Slider label="Cruise speed" value={v} min={0.1} max={1.5} step={0.01} unit="m/s" onChange={setV} />
          <Slider label="Wheel base b" value={b} min={0.1} max={0.8} step={0.01} unit="m" onChange={setB} />
          <Slider label="Wheel speed limit" value={vmax} min={0.2} max={2} step={0.01} unit="m/s" onChange={setVmax} />
          <Slider label="Start offset (lateral)" value={offY} min={-1} max={1} step={0.01} unit="m" onChange={setOffY} />
          <Slider label="Start heading error" value={offTh} min={-90} max={90} step={1} unit="°" onChange={setOffTh} />
          <Toggle label="Slow down in curves (a_lat ≤ 0.6 m/s²)" value={slow} onChange={setSlow} />
          <div className="row">
            <button className="btn small" onClick={() => setWp(DEFAULT_WP)}>Reset path</button>
            <DownloadButton filename="pure-pursuit.csv" content={csv} label="Trajectory CSV" small />
          </div>
          <ErrorBox error={res.error} />
        </Card>
        <Card title="Pure pursuit on a spline" subtitle="Drag the waypoints · dashed circle = lookahead · dot = goal point">
          <svg
            ref={svgRef}
            viewBox={`${x0} ${-y1} ${x1 - x0} ${y1 - y0}`}
            style={{ width: '100%', maxHeight: 460, display: 'block', touchAction: 'none' }}
            onPointerMove={(e) => { if (drag !== null) { const p = toWorld(e); setWp(wp.map((q, i) => (i === drag ? p : q))); } }}
            onPointerUp={() => setDrag(null)}
            onPointerLeave={() => setDrag(null)}
            role="img"
            aria-label="Pure pursuit path following"
          >
            <g transform="scale(1,-1)">
              {Array.from({ length: Math.ceil(x1 - x0) + 1 }, (_, i) => Math.floor(x0) + i).map((gx) => <line key={`gx${gx}`} x1={gx} x2={gx} y1={y0} y2={y1} stroke="var(--grid)" strokeWidth={0.01} />)}
              {Array.from({ length: Math.ceil(y1 - y0) + 1 }, (_, i) => Math.floor(y0) + i).map((gy) => <line key={`gy${gy}`} y1={gy} y2={gy} x1={x0} x2={x1} stroke="var(--grid)" strokeWidth={0.01} />)}
              {d && <polyline points={d.path_x.map((x, i) => `${x},${d.path_y[i]}`).join(' ')} fill="none" stroke="var(--border-strong)" strokeWidth={0.09} strokeLinejoin="round" />}
              {d && <polyline points={d.path_x.map((x, i) => `${x},${d.path_y[i]}`).join(' ')} fill="none" stroke="var(--muted)" strokeWidth={0.015} strokeDasharray="0.08 0.06" />}
              <polyline points={trail} fill="none" stroke="var(--series-1)" strokeWidth={0.035} strokeLinejoin="round" />
              {d && (
                <>
                  <circle cx={rx} cy={ry} r={la} fill="none" stroke="var(--series-2)" strokeWidth={0.015} strokeDasharray="0.06 0.05" />
                  <line x1={rx} y1={ry} x2={d.goal_x[k]} y2={d.goal_y[k]} stroke="var(--series-2)" strokeWidth={0.015} />
                  <circle cx={d.goal_x[k]} cy={d.goal_y[k]} r={0.05} fill="var(--series-2)" />
                  <g transform={`translate(${rx} ${ry}) rotate(${deg})`}>
                    <rect x={-0.16} y={-b / 2 - 0.02} width={0.32} height={b + 0.04} rx={0.05} fill="var(--surface-3)" stroke="var(--text)" strokeWidth={0.015} />
                    <rect x={-0.07} y={b / 2 - 0.01} width={0.14} height={0.05} rx={0.015} fill="var(--text-2)" />
                    <rect x={-0.07} y={-b / 2 - 0.04} width={0.14} height={0.05} rx={0.015} fill="var(--text-2)" />
                    <path d="M0.16,0 L0.05,0.06 L0.05,-0.06 Z" fill="var(--accent)" />
                  </g>
                </>
              )}
              {wp.map((p, i) => (
                <circle key={i} cx={p[0]} cy={p[1]} r={0.09} fill={drag === i ? 'var(--accent)' : 'var(--surface-1)'} stroke="var(--accent)" strokeWidth={0.025} style={{ cursor: 'grab' }} onPointerDown={(e) => { (e.target as Element).setPointerCapture?.(e.pointerId); setDrag(i); }} />
              ))}
            </g>
          </svg>
          <PlayBar clock={clock} speeds={[0.5, 1, 2, 4, 8]} />
        </Card>
      </div>
      <div className="grid grid-2">
        <Card title="Cross-track error"><Plot series={cte} height={190} xLabel="t (s)" yLabel="error (m)" legend={false} vLines={[{ value: clock.t, color: 'var(--accent)' }]} hLines={[{ value: 0 }]} /></Card>
        <Card title="Wheel speeds"><Plot series={wheels} height={190} xLabel="t (s)" yLabel="m/s" vLines={[{ value: clock.t, color: 'var(--accent)' }]} hLines={[{ value: vmax, label: 'limit', color: 'var(--bad)' }]} /></Card>
      </div>
      <div className="grid grid-2">
        <Explainer title="Pure pursuit" refs={[{ label: 'R. C. Coulter, CMU-RI-TR-92-01 (1992)' }, { label: 'Lynch & Park, Modern Robotics, ch. 13', href: 'http://modernrobotics.org' }]}>
          <Formula>{`goal point: L metres ahead along the path from the closest point
y_r = −sinθ·(x_g − x) + cosθ·(y_g − y)      (lateral offset in the robot frame)
κ = 2 y_r / L²,   ω = v κ,   v_{L,R} = v ∓ ω b / 2`}</Formula>
          <p>The robot steers onto the unique circle through itself and the goal point. Small L tracks tightly but oscillates (high gain); large L is smooth but cuts corners. When a wheel hits its speed limit both are scaled to keep the curvature — the robot slows instead of leaving the arc.</p>
        </Explainer>
        <Explainer title="Kinematics">
          <Formula>{`ẋ = v cosθ,  ẏ = v sinθ,  θ̇ = ω       (integrated exactly along arcs)`}</Formula>
          <p>A differential drive cannot move sideways (a non-holonomic constraint), which is why the path follower must turn towards the path rather than slide onto it. For real robots, graduate to ROS 2 Nav2 (its "Regulated Pure Pursuit" controller builds on this algorithm).</p>
        </Explainer>
      </div>
    </div>
  );
}
