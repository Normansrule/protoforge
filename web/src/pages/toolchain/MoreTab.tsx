// Optics (Ray Optics Simulation scene), chips (Verilog → EDA Playground /
// DigitalJS), quantum and firmware hand-offs.
import { Card, Chip, Explainer, Formula } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { useMetricMap, useProject } from '../../state/project';
import { CodeView } from './CodeView';
import { downloadFiles, type useBundle } from './bundle';

interface P { x: number; y: number }
interface SceneObj { type: string; p1?: P; p2?: P; x?: number; y?: number; text?: string; focalLength?: number; transRatio?: number }
interface Scene { version: number; name: string; objs: SceneObj[] }

/** Paraxial rays through the exported scene: ideal lens, then the 45° splitter, then to the eye plane. */
function rays(s: Scene) {
  const lens = s.objs.find((o) => o.type === 'IdealLens');
  const bs = s.objs.find((o) => o.type === 'BeamSplitter');
  const det = s.objs.find((o) => o.type === 'Detector');
  const srcs = s.objs.filter((o) => o.type === 'PointSource');
  if (!lens?.p1 || !lens.p2 || !bs?.p1 || !bs.p2 || !det?.p1) return [];
  const xl = lens.p1.x;
  const f = lens.focalLength ?? 1;
  const out: { pts: [number, number][]; k: number }[] = [];
  srcs.forEach((src, k) => {
    for (const t of [0.12, 0.5, 0.88]) {
      const yl = lens.p1!.y + (lens.p2!.y - lens.p1!.y) * t;
      const x0 = src.x!;
      const y0 = src.y!;
      const u = (yl - y0) / (xl - x0) - yl / f; // slope after the ideal lens (paraxial, y measured from the axis)
      // splitter line: points p1 + s (p2 − p1); ray: (xl, yl) + τ (1, u)
      const [ax, ay, bx, by] = [bs.p1!.x, bs.p1!.y, bs.p2!.x, bs.p2!.y];
      const dx = bx - ax;
      const dy = by - ay;
      const den = 1 * dy - u * dx;
      if (Math.abs(den) < 1e-9) continue;
      const tau = ((ax - xl) * dy - (ay - yl) * dx) / den;
      const hx = xl + tau;
      const hy = yl + tau * u;
      // reflect direction (1, u) about the mirror line
      const ml = Math.hypot(dx, dy);
      const mx = dx / ml;
      const my = dy / ml;
      const dn = 1 * mx + u * my;
      const rx = 2 * dn * mx - 1;
      const ry = 2 * dn * my - u;
      const ty = (det.p1!.y - hy) / ry;
      out.push({ pts: [[x0, y0], [xl, yl], [hx, hy], [hx + rx * ty, det.p1!.y]], k });
    }
  });
  return out;
}

function SceneView({ scene }: { scene: Scene }) {
  const pts = scene.objs.flatMap((o) => (o.p1 && o.p2 ? [o.p1, o.p2] : o.x !== undefined ? [{ x: o.x, y: o.y! }] : []));
  const xs = pts.map((p) => p.x);
  const ys = pts.map((p) => p.y);
  const pad = 40;
  const vb = [Math.min(...xs) - pad, Math.min(...ys) - pad, Math.max(...xs) - Math.min(...xs) + 2 * pad + 160, Math.max(...ys) - Math.min(...ys) + 2 * pad];
  return (
    <svg viewBox={vb.join(' ')} style={{ width: '100%', maxHeight: 360, display: 'block' }} role="img" aria-label="Ray Optics scene preview">
      {rays(scene).map((r, i) => <polyline key={i} points={r.pts.map((p) => p.join(',')).join(' ')} fill="none" stroke={`var(--series-${(r.k % 8) + 3})`} strokeWidth={2} opacity={0.75} className="flow-line" />)}
      {scene.objs.map((o, i) => {
        if (o.type === 'Blocker') return <line key={i} x1={o.p1!.x} y1={o.p1!.y} x2={o.p2!.x} y2={o.p2!.y} stroke="var(--muted)" strokeWidth={6} />;
        if (o.type === 'IdealLens') return <g key={i}><line x1={o.p1!.x} y1={o.p1!.y} x2={o.p2!.x} y2={o.p2!.y} stroke="var(--accent)" strokeWidth={4} /><path d={`M${o.p1!.x - 9} ${o.p1!.y + 12} L${o.p1!.x} ${o.p1!.y} L${o.p1!.x + 9} ${o.p1!.y + 12} M${o.p2!.x - 9} ${o.p2!.y - 12} L${o.p2!.x} ${o.p2!.y} L${o.p2!.x + 9} ${o.p2!.y - 12}`} stroke="var(--accent)" strokeWidth={3} fill="none" /></g>;
        if (o.type === 'BeamSplitter') return <line key={i} x1={o.p1!.x} y1={o.p1!.y} x2={o.p2!.x} y2={o.p2!.y} stroke="var(--series-2)" strokeWidth={4} strokeDasharray="10 5" />;
        if (o.type === 'Detector') return <line key={i} x1={o.p1!.x} y1={o.p1!.y} x2={o.p2!.x} y2={o.p2!.y} stroke="var(--good)" strokeWidth={6} />;
        if (o.type === 'PointSource') return <circle key={i} cx={o.x} cy={o.y} r={7} fill="var(--series-3)" />;
        if (o.type === 'TextLabel') return <text key={i} x={o.x} y={o.y} fontSize={17} fill="var(--text-2)">{o.text}</text>;
        return null;
      })}
    </svg>
  );
}

export function MoreTab({ bundle }: { bundle: ReturnType<typeof useBundle> }) {
  const project = useProject((s) => s.project);
  const scene = useCompute<Scene>('bridge.ray_optics', { project });
  const json = scene.data ? JSON.stringify(scene.data, null, 2) : '';
  const o = project.design.optics;
  const clearance = useMetricMap().get('optics.combiner_clearance_mm')?.value;
  const minLc = clearance !== undefined ? o.lens_to_combiner_mm - clearance : null;
  const verilog = ['tt_um_iris_dsp.v', 'iris_cic.v', 'iris_fir.v', 'iris_gesture_fsm.v', 'tb_iris_fir.v', 'tb_iris_gesture_fsm.v', 'tb_tt_um_iris_dsp.v'];
  return (
    <div className="stack">
      <div className="grid grid-sidebar tc-more">
        <Card title="Ray Optics Simulation scene" subtitle="format version 5 · open with the app’s Open button" actions={<a className="btn small primary" href="https://phydemo.app/ray-optics/" target="_blank" rel="noreferrer">Ray Optics ↗</a>}>
          {json && <CodeView code={json} filename={`${project.meta.codename?.toLowerCase() || 'hud'}-hud.ray-optics.json`} lang="plain" maxLines={60} maxHeight={260} mime="application/json" />}
          <ol className="small" style={{ marginTop: 10 }}>
            <li>Download the scene, open phydemo.app/ray-optics and load it with Open.</li>
            <li>Drag the beam splitter (the combiner) towards the lens: {minLc !== null ? <>closer than {minLc.toFixed(1)} mm (the Optics Lab's limit; it sits at {o.lens_to_combiner_mm} mm)</> : 'too close'} and its upper end enters the lens aperture — requirement R-025.</li>
            <li>Switch the app to its image-observing modes to see the virtual image the eye sees.</li>
          </ol>
        </Card>
        <Card title="Preview of the exported scene" subtitle="Paraxial rays through the scene’s own objects: display → ideal lens → 45° combiner → pupil">
          {scene.data && <SceneView scene={scene.data} />}
          <Explainer title="What the scene encodes" refs={[{ label: 'Ray Optics Simulation (Y.-T. Tu)', href: 'https://github.com/ricktu288/ray-optics' }, { label: 'E. Hecht, Optics, 5th ed., 2017' }]}>
            The display pixels become point sources, the collimating lens an ideal thin lens (f = {o.lens_focal_mm} mm, drawn at 20 px/mm), the combiner a beam splitter with transmission 1 − R = {(1 - o.combiner_reflectance).toFixed(2)}, the eye pupil a detector {o.eye_relief_mm} mm below it. Behind the thin lens every ray bends by
            <Formula>{'u′ = u − y/f     (paraxial thin lens)\nd′ = 2(d·m̂)m̂ − d     (reflection off the combiner along m̂)'}</Formula>
          </Explainer>
        </Card>
      </div>
      <div className="grid grid-3">
        <Card title="Verilog → EDA Playground · DigitalJS" subtitle="The IRIS-DSP co-processor from the Chip Lab">
          <div className="row" style={{ gap: 5, marginBottom: 8 }}>{verilog.map((v) => <Chip key={v}>{v}</Chip>)}</div>
          <ol className="small">
            <li>EDA Playground: testbench (e.g. tb_iris_fir.v) left, design (iris_fir.v) right, Icarus Verilog, Run.</li>
            <li>DigitalJS: paste iris_gesture_fsm.v and synthesise to watch the gates.</li>
          </ol>
          <div className="row">
            <button className="btn small" disabled={bundle.busy} onClick={() => bundle.get().then((f) => downloadFiles(f, { label: 'Verilog', filename: 'iris-verilog.zip', match: (p) => p.startsWith('chip/'), zip: true }))}>↓ Verilog + testbenches</button>
            <a className="btn small" href="https://www.edaplayground.com/" target="_blank" rel="noreferrer">EDA Playground ↗</a>
            <a className="btn small" href="https://digitaljs.tilk.eu/" target="_blank" rel="noreferrer">DigitalJS ↗</a>
            <a className="btn small ghost" href="#/chip">Chip Lab →</a>
          </div>
        </Card>
        <Card title="OpenQASM → IBM Quantum · Quirk" subtitle="Built by the Quantum Lab">
          <div className="small">The Quantum Lab writes OpenQASM for its circuits (teleportation, Bell tests, key distribution) and one-click Quirk links. Paste the OpenQASM into the IBM Quantum Composer’s code editor to run it on a simulator or real hardware.</div>
          <div className="row" style={{ marginTop: 10 }}>
            <a className="btn small primary" href="#/quantum">Quantum Lab →</a>
            <a className="btn small" href="https://algassert.com/quirk" target="_blank" rel="noreferrer">Quirk ↗</a>
          </div>
        </Card>
        <Card title="Firmware → Wokwi" subtitle="Built by Prototype & Firmware">
          <div className="small">The breadboard version of Iris (development board, IMU, display, microphones) and its generated firmware live on the Prototype & Firmware page; Wokwi simulates ESP32 and Arduino boards with those parts in the browser.</div>
          <div className="row" style={{ marginTop: 10 }}>
            <a className="btn small primary" href="#/proto">Prototype & Firmware →</a>
            <a className="btn small" href="https://wokwi.com/" target="_blank" rel="noreferrer">Wokwi ↗</a>
          </div>
        </Card>
      </div>
    </div>
  );
}
