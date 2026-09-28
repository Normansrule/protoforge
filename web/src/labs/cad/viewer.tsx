// 3D viewer used by the CAD Studio and the Print Lab. It extends the shared
// Scene3D idea with camera presets, crease-angle normals (crisp edges on the
// drafted prisms, smooth curved walls), x-ray / wireframe materials, a local
// light-former environment (no network HDRs) and soft contact shadows.
// All children are placed in ENGINE coordinates (mm, Z up) inside a group
// rotated into three.js' Y-up world.
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import { ContactShadows, Environment, Grid, Lightformer, OrbitControls } from '@react-three/drei';
import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import * as THREE from 'three';
import { toCreasedNormals } from 'three/examples/jsm/utils/BufferGeometryUtils.js';

export type ViewPreset = 'iso' | 'front' | 'side' | 'top' | 'back' | 'below';

/** true when the user asked the OS for reduced motion. */
export function prefersReducedMotion(): boolean {
  return typeof window !== 'undefined' && !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
}

/** Current theme ('dark' | 'light'), live-updated when the toggle flips. */
export function useTheme(): 'dark' | 'light' {
  const read = () => (document.documentElement.dataset.theme === 'light' ? 'light' : 'dark');
  const [t, setT] = useState<'dark' | 'light'>(read);
  useEffect(() => {
    const obs = new MutationObserver(() => setT(read()));
    obs.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
    return () => obs.disconnect();
  }, []);
  return t;
}

/** Read a CSS custom property (theme token) as a colour string. */
export function cssVar(name: string, fallback = '#888'): string {
  if (typeof window === 'undefined') return fallback;
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return v || fallback;
}

/** Engine (x, y, z) → three.js (x, z, -y). */
export const toThree = (p: [number, number, number]): [number, number, number] => [p[0], p[2], -p[1]];

export interface Bounds {
  min: [number, number, number];
  max: [number, number, number];
}

function presetDir(p: ViewPreset): THREE.Vector3 {
  switch (p) {
    // Engine +Y (forward) is three.js −Z, so "front" looks from −Z.
    case 'front':
      return new THREE.Vector3(0, 0.05, -1);
    case 'back':
      return new THREE.Vector3(0, 0.15, 1);
    case 'side':
      return new THREE.Vector3(1, 0.08, 0);
    case 'top':
      return new THREE.Vector3(0, 1, 0.0001);
    case 'below':
      return new THREE.Vector3(0.7, -0.75, -0.9);
    default:
      return new THREE.Vector3(0.8, 0.55, -1);
  }
}

/** Animates camera + orbit target to a preset whenever `nonce` changes. */
function CameraRig(props: { bounds: Bounds; preset: ViewPreset; nonce: number; fit: number }) {
  const { camera, controls, size } = useThree() as unknown as { camera: THREE.PerspectiveCamera; controls: { target: THREE.Vector3; update: () => void } | null; size: { width: number; height: number } };
  const aspect = size.width / Math.max(1, size.height);
  const goal = useRef<{ pos: THREE.Vector3; target: THREE.Vector3; t: number } | null>(null);
  useEffect(() => {
    const { min, max } = props.bounds;
    const c = new THREE.Vector3(...toThree([(min[0] + max[0]) / 2, (min[1] + max[1]) / 2, (min[2] + max[2]) / 2]));
    const r = 0.5 * Math.hypot(max[0] - min[0], max[1] - min[1], max[2] - min[2]);
    // Fit the bounding sphere into the narrower of the two fields of view.
    const vHalf = ((camera.fov / 2) * Math.PI) / 180;
    const hHalf = Math.atan(Math.tan(vHalf) * aspect);
    const dist = (r / Math.sin(Math.min(vHalf, hHalf))) * props.fit * (aspect < 1 ? 1.2 : 1);
    const pos = c.clone().add(presetDir(props.preset).normalize().multiplyScalar(dist));
    goal.current = { pos, target: c, t: 0 };
    if (prefersReducedMotion() || props.nonce === 0) {
      camera.position.copy(pos);
      controls?.target.copy(c);
      controls?.update();
      goal.current = null;
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.nonce, controls, aspect > 1]);
  useFrame((_, dt) => {
    const g = goal.current;
    if (!g || !controls) return;
    g.t += dt;
    const k = 1 - Math.exp(-dt * 7);
    camera.position.lerp(g.pos, k);
    controls.target.lerp(g.target, k);
    controls.update();
    if (g.t > 1.2) goal.current = null;
  });
  return null;
}

export function CadCanvas(props: {
  children: ReactNode;
  height?: number | string;
  bounds: Bounds;
  preset?: ViewPreset;
  nonce?: number;
  autoRotate?: boolean;
  grid?: boolean;
  shadows?: boolean;
  fit?: number;
  overlay?: ReactNode;
}) {
  const theme = useTheme();
  const { bounds, height = 520, grid = true, shadows = true } = props;
  const floorY = bounds.min[2] - 0.5;
  const size = Math.max(bounds.max[0] - bounds.min[0], bounds.max[1] - bounds.min[1], 40);
  const cell = theme === 'light' ? '#cfcfc8' : '#34343a';
  const section = theme === 'light' ? '#a9b7cc' : '#465676';
  return (
    <div className="scene3d cad-canvas" style={{ height, position: 'relative' }}>
      <Canvas camera={{ position: [180, 120, 200], fov: 35, near: 0.5, far: 6000 }} dpr={[1, 2]} gl={{ antialias: true, preserveDrawingBuffer: true }}>
        <ambientLight intensity={theme === 'light' ? 0.55 : 0.4} />
        <directionalLight position={[120, 220, 140]} intensity={1.4} />
        <directionalLight position={[-160, 90, -120]} intensity={0.5} />
        <Environment resolution={128} frames={1}>
          <Lightformer form="rect" intensity={2.2} position={[0, 4, 6]} scale={[10, 4, 1]} />
          <Lightformer form="rect" intensity={1.2} position={[-6, 2, -4]} scale={[6, 6, 1]} />
          <Lightformer form="ring" intensity={1.6} position={[6, 5, 2]} scale={3} />
          <Lightformer form="rect" intensity={0.6} color="#9ecbff" position={[0, -4, 0]} rotation-x={Math.PI / 2} scale={[20, 20, 1]} />
        </Environment>
        {grid && (
          <Grid
            position={[0, floorY - 0.05, 0]}
            args={[size * 4, size * 4]}
            cellSize={5}
            cellThickness={0.5}
            cellColor={cell}
            sectionSize={50}
            sectionThickness={1}
            sectionColor={section}
            fadeDistance={size * 5}
            infiniteGrid
          />
        )}
        {shadows && <ContactShadows key={`${bounds.min.map((v) => v.toFixed(1)).join()}|${bounds.max.map((v) => v.toFixed(1)).join()}`} position={[0, floorY, 0]} scale={size * 2.2} blur={2.4} opacity={theme === 'light' ? 0.35 : 0.55} far={size} resolution={512} frames={1} />}
        <group rotation={[-Math.PI / 2, 0, 0]}>{props.children}</group>
        <OrbitControls makeDefault enableDamping autoRotate={props.autoRotate} autoRotateSpeed={0.7} />
        <CameraRig bounds={bounds} preset={props.preset ?? 'iso'} nonce={props.nonce ?? 0} fit={props.fit ?? 1.05} />
      </Canvas>
      {props.overlay}
    </div>
  );
}

export interface MeshLike {
  positions: ArrayLike<number>;
  indices?: ArrayLike<number>;
  colors?: ArrayLike<number>;
}

/** Build a BufferGeometry with crease-angle normals (crisp 90° edges). */
export function useEngineGeometry(mesh: MeshLike, crease = 32) {
  return useMemo(() => {
    let g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.Float32BufferAttribute(Array.from(mesh.positions), 3));
    if (mesh.colors) g.setAttribute('color', new THREE.Float32BufferAttribute(Array.from(mesh.colors), 3));
    if (mesh.indices && mesh.indices.length) g.setIndex(Array.from(mesh.indices));
    if (!mesh.colors) {
      g = toCreasedNormals(g, (crease * Math.PI) / 180);
    } else {
      g = g.index ? g.toNonIndexed() : g;
      g.computeVertexNormals();
    }
    g.computeBoundingBox();
    return g;
  }, [mesh, crease]);
}

export function PartMesh(props: {
  mesh: MeshLike;
  color?: string;
  opacity?: number;
  wireframe?: boolean;
  xray?: boolean;
  highlight?: boolean;
  vertexColors?: boolean;
  roughness?: number;
  metalness?: number;
  clearcoat?: number;
  onHover?: (on: boolean) => void;
  onClick?: () => void;
  renderOrder?: number;
}) {
  const geom = useEngineGeometry(props.mesh);
  const transparent = props.xray || (props.opacity ?? 1) < 1;
  const opacity = props.xray ? 0.28 : props.opacity ?? 1;
  return (
    <mesh
      geometry={geom}
      renderOrder={props.renderOrder}
      onPointerOver={props.onHover ? (e) => { e.stopPropagation(); props.onHover!(true); } : undefined}
      onPointerOut={props.onHover ? () => props.onHover!(false) : undefined}
      onClick={props.onClick ? (e) => { e.stopPropagation(); props.onClick!(); } : undefined}
    >
      <meshPhysicalMaterial
        color={props.vertexColors ? '#ffffff' : props.color ?? '#9aa7b8'}
        vertexColors={!!props.vertexColors}
        roughness={props.vertexColors ? 0.7 : props.roughness ?? 0.38}
        metalness={props.metalness ?? 0.05}
        clearcoat={props.vertexColors ? 0 : props.clearcoat ?? 0.6}
        clearcoatRoughness={0.25}
        transparent={transparent}
        opacity={opacity}
        depthWrite={!transparent}
        wireframe={props.wireframe}
        side={transparent ? THREE.DoubleSide : THREE.FrontSide}
        emissive={props.highlight ? '#3987e5' : '#000000'}
        emissiveIntensity={props.highlight ? 0.35 : 0}
      />
    </mesh>
  );
}

/** Mesh edges (feature lines) for a technical-drawing look. */
export function PartEdges(props: { mesh: MeshLike; color: string; opacity?: number; angle?: number }) {
  const geom = useEngineGeometry(props.mesh);
  const edges = useMemo(() => new THREE.EdgesGeometry(geom, props.angle ?? 35), [geom, props.angle]);
  return (
    <lineSegments geometry={edges}>
      <lineBasicMaterial color={props.color} transparent opacity={props.opacity ?? 0.5} />
    </lineSegments>
  );
}

/** Animate a number towards a target (exponential smoothing), honouring
 * prefers-reduced-motion. Returns the live value via a ref-driven state. */
export function useAnimatedValue(target: number, speed = 6): number {
  const [v, setV] = useState(target);
  const cur = useRef(target);
  useEffect(() => {
    if (prefersReducedMotion()) {
      cur.current = target;
      setV(target);
      return;
    }
    let raf = 0;
    let last = performance.now();
    const tick = (now: number) => {
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      const k = 1 - Math.exp(-dt * speed);
      cur.current += (target - cur.current) * k;
      if (Math.abs(target - cur.current) < 1e-3) {
        cur.current = target;
        setV(target);
        return;
      }
      setV(cur.current);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [target, speed]);
  return v;
}

export function PresetButtons(props: { value: ViewPreset; onChange: (p: ViewPreset) => void; below?: boolean }) {
  const opts: ViewPreset[] = props.below ? ['iso', 'front', 'side', 'top', 'below'] : ['iso', 'front', 'side', 'top', 'back'];
  return (
    <div className="cp-seg" role="group" aria-label="View preset">
      {opts.map((o) => (
        <button key={o} className={`cp-seg-btn ${props.value === o ? 'active' : ''}`} onClick={() => props.onChange(o)}>
          {o}
        </button>
      ))}
    </div>
  );
}
