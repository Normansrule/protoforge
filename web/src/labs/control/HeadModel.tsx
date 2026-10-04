// Stylised head wearing the Iris glasses, posed by NED roll/pitch/yaw.
import { useMemo, type ReactNode } from 'react';
import * as THREE from 'three';
import { Canvas } from '@react-three/fiber';
import { useInView } from '../../components/Scene3D';
import { OrbitControls } from '@react-three/drei';

/** Convert NED body Euler angles (deg, ZYX) to a three.js quaternion.
 *  Mapping: NED x (forward) → +z, NED y (right) → −x, NED z (down) → −y. */
export function nedQuaternion(rollDeg: number, pitchDeg: number, yawDeg: number): THREE.Quaternion {
  const d = Math.PI / 180;
  const qz = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, -1, 0), yawDeg * d);
  const qy = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(-1, 0, 0), pitchDeg * d);
  const qx = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 0, 1), rollDeg * d);
  return qz.multiply(qy).multiply(qx);
}

function Glasses({ color, ghost, podColor }: { color: string; ghost?: boolean; podColor: string }) {
  const mat = (c: string) => (
    <meshStandardMaterial color={c} metalness={0.2} roughness={0.45} transparent={ghost} opacity={ghost ? 0.5 : 1} wireframe={ghost} />
  );
  // frame front sits ~ 78 mm in front of the neck pivot, at eye height
  return (
    <group position={[0, 18, 78]}>
      {[-1, 1].map((s) => (
        <mesh key={s} position={[s * 34, 0, 0]}>
          <torusGeometry args={[22, 3, 10, 40]} />
          {mat(color)}
        </mesh>
      ))}
      {!ghost && [-1, 1].map((s) => (
        <mesh key={`l${s}`} position={[s * 34, 0, -0.5]}>
          <circleGeometry args={[20, 32]} />
          <meshStandardMaterial color="#8fb7e8" transparent opacity={0.22} side={THREE.DoubleSide} />
        </mesh>
      ))}
      <mesh position={[0, 4, 0]}>
        <boxGeometry args={[24, 4, 4]} />
        {mat(color)}
      </mesh>
      {/* temples */}
      {[-1, 1].map((s) => (
        <mesh key={`t${s}`} position={[s * 57, 2, -50]}>
          <boxGeometry args={[4, 7, 100]} />
          {mat(color)}
        </mesh>
      ))}
      {/* electronics pod on the wearer's right temple (−x in three) */}
      <mesh position={[-59, 1, -28]}>
        <boxGeometry args={[11, 14, 58]} />
        {mat(podColor)}
      </mesh>
      {/* display prism in front of the right eye */}
      <mesh position={[-34, 12, 6]}>
        <boxGeometry args={[14, 9, 9]} />
        <meshStandardMaterial color="#5ad1e6" transparent opacity={ghost ? 0.3 : 0.75} wireframe={ghost} />
      </mesh>
    </group>
  );
}

export function HeadModel(props: { truth: [number, number, number]; est?: [number, number, number]; estColor?: string; skin?: string; frame?: string; accent?: string }) {
  const qT = useMemo(() => nedQuaternion(...props.truth), [props.truth]);
  const qE = useMemo(() => (props.est ? nedQuaternion(...props.est) : null), [props.est]);
  return (
    <group position={[0, 120, 0]}>
      <group quaternion={qT}>
        <mesh castShadow>
          <sphereGeometry args={[62, 40, 30]} />
          <meshStandardMaterial color={props.skin ?? '#c9b8a6'} roughness={0.8} />
        </mesh>
        {/* nose shows which way is forward */}
        <mesh position={[0, 2, 64]} rotation={[Math.PI / 2, 0, 0]}>
          <coneGeometry args={[9, 22, 16]} />
          <meshStandardMaterial color={props.skin ?? '#c9b8a6'} roughness={0.8} />
        </mesh>
        {/* ears */}
        {[-1, 1].map((s) => (
          <mesh key={s} position={[s * 60, 8, -6]} scale={[0.4, 1, 0.7]}>
            <sphereGeometry args={[16, 16, 12]} />
            <meshStandardMaterial color={props.skin ?? '#c9b8a6'} roughness={0.8} />
          </mesh>
        ))}
        <Glasses color={props.frame ?? '#2b2f36'} podColor={props.accent ?? '#3987e5'} />
      </group>
      {qE && (
        <group quaternion={qE}>
          <Glasses color={props.estColor ?? '#d95926'} podColor={props.estColor ?? '#d95926'} ghost />
        </group>
      )}
      {/* neck */}
      <mesh position={[0, -80, -10]}>
        <cylinderGeometry args={[26, 30, 60, 24]} />
        <meshStandardMaterial color={props.skin ?? '#c9b8a6'} roughness={0.8} />
      </mesh>
    </group>
  );
}

/** Self-contained 3D viewport (no HDR environment download, so it also works offline). */
export function MiniScene(props: { children: ReactNode; height?: number; camera?: [number, number, number]; target?: [number, number, number]; floor?: string }) {
  const { height = 330, camera = [230, 210, 300], target = [0, 110, 0] } = props;
  const [host, visible] = useInView<HTMLDivElement>();
  return (
    <div ref={host} className="scene3d" style={{ height }}>
      <Canvas frameloop={visible ? 'always' : 'never'} camera={{ position: camera, fov: 40, near: 1, far: 5000 }} dpr={[1, 2]}>
        <hemisphereLight args={['#ffffff', '#444455', 0.9]} />
        <directionalLight position={[150, 260, 180]} intensity={1.4} />
        <directionalLight position={[-200, 120, -150]} intensity={0.5} />
        <gridHelper args={[600, 24, props.floor ?? '#4a5a7a', props.floor ?? '#33363d']} />
        {props.children}
        <OrbitControls target={target} makeDefault enableDamping />
      </Canvas>
    </div>
  );
}
