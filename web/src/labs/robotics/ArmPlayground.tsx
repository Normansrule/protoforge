import { useRef, useState } from 'react';
import { useCompute } from '../../engine/hooks';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat, Toggle } from '../../components/ui';
import { KV, fmt, json } from '../control/kit';
import type { Arm2, ArmQuery, ArmSolution, Workspace } from './types';

/** Convert a pointer event to SVG user coordinates (y flipped back to world). */
function toWorld(svg: SVGSVGElement, e: React.PointerEvent): [number, number] {
  const pt = svg.createSVGPoint();
  pt.x = e.clientX;
  pt.y = e.clientY;
  const m = svg.getScreenCTM();
  if (!m) return [0, 0];
  const p = pt.matrixTransform(m.inverse());
  return [p.x, -p.y];
}

function ArmDrawing({ s, ghost, color }: { s: ArmSolution; ghost?: boolean; color: string }) {
  const w = ghost ? 0.035 : 0.06;
  return (
    <g opacity={ghost ? 0.35 : 1}>
      <line x1={0} y1={0} x2={s.elbow[0]} y2={s.elbow[1]} stroke={color} strokeWidth={w} strokeLinecap="round" strokeDasharray={ghost ? '0.06 0.05' : undefined} />
      <line x1={s.elbow[0]} y1={s.elbow[1]} x2={s.tip[0]} y2={s.tip[1]} stroke={color} strokeWidth={w * 0.8} strokeLinecap="round" strokeDasharray={ghost ? '0.06 0.05' : undefined} />
      {!ghost && (
        <>
          <circle cx={s.elbow[0]} cy={s.elbow[1]} r={0.045} fill="var(--surface-1)" stroke={color} strokeWidth={0.02} />
          <circle cx={s.tip[0]} cy={s.tip[1]} r={0.03} fill={color} />
        </>
      )}
    </g>
  );
}

export function ArmPlayground() {
  const [arm, setArm] = useState<Arm2>({ l1: 1.0, l2: 0.7, q1_min: -135, q1_max: 135, q2_min: -150, q2_max: 150 });
  const [target, setTarget] = useState<[number, number]>([1.1, 0.6]);
  const [elbowUp, setElbowUp] = useState(true);
  const [showWs, setShowWs] = useState(true);
  const [showEllipse, setShowEllipse] = useState(true);
  const [dragging, setDragging] = useState(false);
  const svgRef = useRef<SVGSVGElement>(null);
  const q = useCompute<ArmQuery>('robotics.arm', { arm, x: target[0], y: target[1], elbow_up: elbowUp });
  const ws = useCompute<Workspace>('robotics.workspace', { arm, n: 46 });
  const d = q.data;
  const s = d?.solution;
  const R = (arm.l1 + arm.l2) * 1.12;
  const ellScale = 0.25;

  const onPointer = (e: React.PointerEvent) => {
    if (!svgRef.current) return;
    const [x, y] = toWorld(svgRef.current, e);
    setTarget([Math.round(x * 1000) / 1000, Math.round(y * 1000) / 1000]);
  };

  const tickCircles = [0.5, 1, 1.5, 2].filter((r) => r < R);

  return (
    <div className="stack">
      <div className="grid grid-4">
        <Stat label="Target" value={d ? (d.reachable ? (s?.within_limits ? '✓ reachable' : '! joint limit') : '✕ out of reach') : '—'} tone={d ? (d.reachable ? (s?.within_limits ? 'pass' : 'warn') : 'fail') : undefined} note={`r = ${fmt(d?.distance, 3)} m (annulus ${fmt(d?.r_min, 2)} … ${fmt(d?.r_max, 2)})`} />
        <Stat label="Joint angles" value={s ? `${s.q_deg[0].toFixed(1)}°, ${s.q_deg[1].toFixed(1)}°` : '—'} note={`${elbowUp ? 'elbow up' : 'elbow down'} · other solution ${d?.alternate ? `${d.alternate.q_deg[0].toFixed(1)}°, ${d.alternate.q_deg[1].toFixed(1)}°` : '—'}`} />
        <Stat label="Manipulability w" value={fmt(s?.manipulability, 3)} unit="m²" note={`max l1·l2 = ${fmt(d?.max_manipulability, 3)} at q2 = ±90°`} />
        <Stat label="Condition κ(J)" value={s?.condition != null && Number.isFinite(s.condition) ? fmt(s.condition, 2) : '∞'} tone={s && (s.condition == null || s.condition > 20) ? 'warn' : undefined} note="σmax/σmin — ∞ at a singularity" />
      </div>
      <div className="grid grid-sidebar">
        <Card title="Arm">
          <Slider label="Link 1 length l1" value={arm.l1} min={0.3} max={1.5} step={0.01} unit="m" onChange={(v) => setArm({ ...arm, l1: v })} />
          <Slider label="Link 2 length l2" value={arm.l2} min={0.2} max={1.5} step={0.01} unit="m" onChange={(v) => setArm({ ...arm, l2: v })} />
          <Slider label="q1 limit ±" value={arm.q1_max} min={30} max={180} step={1} unit="°" onChange={(v) => setArm({ ...arm, q1_min: -v, q1_max: v })} />
          <Slider label="q2 limit ±" value={arm.q2_max} min={30} max={180} step={1} unit="°" onChange={(v) => setArm({ ...arm, q2_min: -v, q2_max: v })} />
          <Toggle label="Elbow up (q2 < 0)" value={elbowUp} onChange={setElbowUp} />
          <Toggle label="Shade joint-limited workspace" value={showWs} onChange={setShowWs} />
          <Toggle label="Manipulability ellipse" value={showEllipse} onChange={setShowEllipse} />
          {s && (
            <>
              <div className="field-label small" style={{ margin: '6px 0 4px' }}>Jacobian J(q) (m/rad)</div>
              <div className="formula">{`⎡${s.jacobian[0][0].toFixed(3).padStart(8)} ${s.jacobian[0][1].toFixed(3).padStart(8)} ⎤\n⎣${s.jacobian[1][0].toFixed(3).padStart(8)} ${s.jacobian[1][1].toFixed(3).padStart(8)} ⎦`}</div>
              <KV rows={[['tip (x, y)', `${s.tip[0].toFixed(3)}, ${s.tip[1].toFixed(3)} m`], ['ellipse axes', `${s.ellipse.a.toFixed(3)} / ${s.ellipse.b.toFixed(3)} m/s per rad/s`]]} />
            </>
          )}
          <DownloadButton filename="arm-solution.json" content={() => json({ arm, target, elbow_up: elbowUp, result: d })} label="Solution JSON" small />
          <ErrorBox error={q.error} />
        </Card>
        <Card title="Planar 2-link arm (SCARA top view)" subtitle="Click or drag anywhere to move the target — the arm solves inverse kinematics analytically">
          <svg
            ref={svgRef}
            viewBox={`${-R} ${-R} ${2 * R} ${2 * R}`}
            style={{ width: '100%', maxHeight: 560, touchAction: 'none', cursor: dragging ? 'grabbing' : 'crosshair', display: 'block' }}
            onPointerDown={(e) => { (e.target as Element).setPointerCapture?.(e.pointerId); setDragging(true); onPointer(e); }}
            onPointerMove={(e) => dragging && onPointer(e)}
            onPointerUp={() => setDragging(false)}
            role="img"
            aria-label="Two-link arm workspace"
          >
            <g transform="scale(1,-1)">
              {/* reachable annulus */}
              <path d={`M ${d?.r_max ?? 1.7} 0 A ${d?.r_max ?? 1.7} ${d?.r_max ?? 1.7} 0 1 0 ${-(d?.r_max ?? 1.7)} 0 A ${d?.r_max ?? 1.7} ${d?.r_max ?? 1.7} 0 1 0 ${d?.r_max ?? 1.7} 0 M ${d?.r_min ?? 0.3} 0 A ${d?.r_min ?? 0.3} ${d?.r_min ?? 0.3} 0 1 1 ${-(d?.r_min ?? 0.3)} 0 A ${d?.r_min ?? 0.3} ${d?.r_min ?? 0.3} 0 1 1 ${d?.r_min ?? 0.3} 0`} fill="var(--accent-soft)" fillRule="evenodd" stroke="var(--border-strong)" strokeWidth={0.006} />
              {tickCircles.map((r) => <circle key={r} r={r} fill="none" stroke="var(--grid)" strokeWidth={0.004} />)}
              <line x1={-R} x2={R} y1={0} y2={0} stroke="var(--grid)" strokeWidth={0.004} />
              <line y1={-R} y2={R} x1={0} x2={0} stroke="var(--grid)" strokeWidth={0.004} />
              {showWs && ws.data?.points.map((p, i) => <circle key={i} cx={p[0]} cy={p[1]} r={0.012} fill="var(--series-3)" opacity={0.35} />)}
              {d?.alternate && <ArmDrawing s={d.alternate} ghost color="var(--text-2)" />}
              {s && <ArmDrawing s={s} color={s.within_limits ? 'var(--series-1)' : 'var(--warn)'} />}
              {s && showEllipse && s.ellipse.a > 0 && (
                <ellipse cx={s.tip[0]} cy={s.tip[1]} rx={s.ellipse.a * ellScale} ry={Math.max(s.ellipse.b * ellScale, 0.003)} transform={`rotate(${s.ellipse.angle_deg} ${s.tip[0]} ${s.tip[1]})`} fill="color-mix(in srgb, var(--series-2) 18%, transparent)" stroke="var(--series-2)" strokeWidth={0.012} />
              )}
              {/* base */}
              <rect x={-0.09} y={-0.09} width={0.18} height={0.18} rx={0.03} fill="var(--surface-3)" stroke="var(--text-2)" strokeWidth={0.012} />
              <circle r={0.05} fill="var(--text-2)" />
              {/* target */}
              <g>
                <circle cx={target[0]} cy={target[1]} r={0.06} fill="none" stroke={d?.reachable ? 'var(--good)' : 'var(--bad)'} strokeWidth={0.014} />
                <line x1={target[0] - 0.09} x2={target[0] + 0.09} y1={target[1]} y2={target[1]} stroke={d?.reachable ? 'var(--good)' : 'var(--bad)'} strokeWidth={0.01} />
                <line y1={target[1] - 0.09} y2={target[1] + 0.09} x1={target[0]} x2={target[0]} stroke={d?.reachable ? 'var(--good)' : 'var(--bad)'} strokeWidth={0.01} />
              </g>
            </g>
            <text x={-R + 0.05} y={-R + 0.12} fontSize={0.075} fill="var(--muted)">Shaded ring = reachable annulus |l1−l2| ≤ r ≤ l1+l2 · green dots = within joint limits · dashed = other elbow</text>
          </svg>
        </Card>
      </div>
      <div className="grid grid-2">
        <Explainer title="Forward & inverse kinematics" refs={[{ label: 'Siciliano et al., Robotics: Modelling, Planning and Control, §2.9' }, { label: 'Lynch & Park, Modern Robotics, ch. 4–6', href: 'http://modernrobotics.org' }]}>
          <Formula>{`x = l1 cos q1 + l2 cos(q1+q2)      y = l1 sin q1 + l2 sin(q1+q2)
cos q2 = (x² + y² − l1² − l2²) / (2 l1 l2),   q2 = ±acos(…)
q1 = atan2(y, x) − atan2(l2 sin q2, l1 + l2 cos q2)`}</Formula>
          <p>Every reachable point inside the ring has two solutions (elbow up / down) that merge on its edges. Joint limits carve the green region out of the ring — some targets are only reachable with one elbow.</p>
        </Explainer>
        <Explainer title="Jacobian, singularities & manipulability" refs={[{ label: 'T. Yoshikawa, "Manipulability of robotic mechanisms", IJRR 1985' }]}>
          <Formula>{`ẋ = J(q) q̇,  J = [[−l1 s1 − l2 s12, −l2 s12], [l1 c1 + l2 c12, l2 c12]]
w = √det(J Jᵀ) = l1 l2 |sin q2|`}</Formula>
          <p>The orange ellipse is the set of tip velocities produced by unit joint speeds (semi-axes √λ(JJᵀ)). Stretch the arm straight (q2 → 0) and it collapses to a line: the tip can no longer move radially, and IK asks for infinite joint speeds. The Iris pick-and-place cell keeps every feeder and pad well inside the ring for exactly this reason.</p>
        </Explainer>
      </div>
    </div>
  );
}
