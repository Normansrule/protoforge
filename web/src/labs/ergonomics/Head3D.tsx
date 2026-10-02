// 3-D fit view: the parametric head from ergonomics.head wearing the Iris CAD
// parts from cad.parts. Temples swing open about their hinge pins until they
// meet the head (or stay at their splay if the head is narrower).
import { CadCanvas, PartMesh, PresetButtons, cssVar, useAnimatedValue, useTheme, type ViewPreset } from '../cad/viewer';
import type { PartsOut } from '../cad/api';
import type { HeadOut, V3 } from './types';
import { useState } from 'react';

const SKIN = '#c79f86';

function Ball({ p, r, color, opacity = 1 }: { p: V3; r: number; color: string; opacity?: number }) {
  return (
    <mesh position={p}>
      <sphereGeometry args={[r, 24, 16]} />
      <meshStandardMaterial color={color} roughness={0.35} transparent={opacity < 1} opacity={opacity} />
    </mesh>
  );
}

/** Rotate children about the vertical (engine Z) axis through `pivot`. */
function Hinged({ pivot, deg, children }: { pivot: [number, number]; deg: number; children: React.ReactNode }) {
  return (
    <group position={[pivot[0], pivot[1], 0]} rotation={[0, 0, (deg * Math.PI) / 180]}>
      <group position={[-pivot[0], -pivot[1], 0]}>{children}</group>
    </group>
  );
}

export function Head3D({ head, parts, height = 460 }: { head: HeadOut; parts: PartsOut | null; height?: number }) {
  const [preset, setPreset] = useState<ViewPreset>('iso');
  const [nonce, setNonce] = useState(0);
  const [xray, setXray] = useState(true);
  const angR = useAnimatedValue(head.temple_angle_deg[0]);
  const angL = useAnimatedValue(head.temple_angle_deg[1]);
  const pin = head.pin;
  useTheme(); // re-render (and re-read the colour tokens) when the theme flips
  const bounds = { min: [-100, -235, -135] as V3, max: [100, 25, 150] as V3 };
  const goodC = cssVar('--good', '#0ca30c');
  const badC = cssVar('--bad', '#d03b3b');
  return (
    <div className="stack" style={{ gap: 8 }}>
      <div className="row" style={{ justifyContent: 'space-between' }}>
        <PresetButtons value={preset} onChange={(p) => { setPreset(p); setNonce((n) => n + 1); }} />
        <label className="small row" style={{ gap: 6, cursor: 'pointer' }}>
          <input type="checkbox" checked={xray} onChange={(e) => setXray(e.target.checked)} style={{ accentColor: 'var(--accent)' }} /> see-through head
        </label>
      </div>
      <CadCanvas bounds={bounds} preset={preset} nonce={nonce} height={height} grid={false} shadows={false} fit={0.72}>
        <PartMesh mesh={head.mesh.head} color={SKIN} opacity={xray ? 0.45 : 1} roughness={0.8} clearcoat={0} />
        <PartMesh mesh={head.mesh.ears} color={SKIN} opacity={xray ? 0.6 : 1} roughness={0.8} clearcoat={0} />
        <PartMesh mesh={head.mesh.nose} color={SKIN} opacity={xray ? 0.7 : 1} roughness={0.8} clearcoat={0} />
        {head.mesh.eyes.map((e, i) => <Ball key={`e${i}`} p={e} r={head.mesh.eye_radius} color="#f2f0ea" />)}
        {head.mesh.pupils.map((p, i) => <Ball key={`p${i}`} p={p} r={2.2} color="#1d2733" />)}
        {head.mesh.contacts.map((p, i) => <Ball key={`c${i}`} p={p} r={2.6} color={head.check.width_ok ? goodC : badC} />)}
        {head.mesh.ear_tops.map((p, i) => <Ball key={`t${i}`} p={p} r={2.2} color={head.check.ear_ok ? goodC : badC} />)}
        {parts && (
          <>
            {parts.parts.filter((p) => p.group === 'front').map((p) => <PartMesh key={p.name} mesh={p.mesh} color={p.color} />)}
            <Hinged pivot={[pin[0], pin[1]]} deg={angR}>
              {parts.parts.filter((p) => p.group === 'right').map((p) => <PartMesh key={p.name} mesh={p.mesh} color={p.color} />)}
            </Hinged>
            <Hinged pivot={[-pin[0], pin[1]]} deg={-angL}>
              {parts.parts.filter((p) => p.group === 'left').map((p) => <PartMesh key={p.name} mesh={p.mesh} color={p.color} />)}
            </Hinged>
            {parts.display.filter((d) => d.kind !== 'pcb').map((d) => <PartMesh key={d.name} mesh={d.mesh} color={d.color} opacity={Math.min(0.5, d.opacity)} />)}
          </>
        )}
      </CadCanvas>
    </div>
  );
}
