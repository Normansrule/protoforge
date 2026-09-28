// Thin wrapper around React Three Fiber for engine-generated meshes.
// Engine meshes arrive as flat arrays: positions [x,y,z,...] (mm) and
// triangle indices [a,b,c,...]; optional per-vertex colours [r,g,b,...] 0..1.
import { Canvas } from '@react-three/fiber';
import { OrbitControls, Grid } from '@react-three/drei';
import { useMemo, type ReactNode } from 'react';
import * as THREE from 'three';

export interface MeshData {
  positions: number[];
  indices: number[];
  colors?: number[];
}

export function Scene3D(props: {
  children: ReactNode;
  height?: number;
  camera?: [number, number, number];
  target?: [number, number, number];
  grid?: boolean;
  autoRotate?: boolean;
}) {
  const { height = 420, camera = [160, 110, 180], target = [0, 0, 0], grid = true } = props;
  return (
    <div className="scene3d" style={{ height }}>
      <Canvas camera={{ position: camera, fov: 40, near: 0.5, far: 5000 }} dpr={[1, 2]} shadows>
        <ambientLight intensity={0.55} />
        <directionalLight position={[120, 200, 100]} intensity={1.3} castShadow />
        <directionalLight position={[-150, 80, -120]} intensity={0.45} />
        <hemisphereLight args={['#dfe8ff', '#1a1a1f', 0.6]} />
        {grid && (
          <Grid
            position={[0, -0.01, 0]}
            args={[400, 400]}
            cellSize={5}
            cellThickness={0.5}
            cellColor="#3a3a40"
            sectionSize={50}
            sectionThickness={1}
            sectionColor="#4a5a7a"
            fadeDistance={600}
            infiniteGrid
          />
        )}
        {props.children}
        <OrbitControls target={target} makeDefault autoRotate={props.autoRotate} autoRotateSpeed={0.8} enableDamping />
      </Canvas>
    </div>
  );
}

/** Render an engine mesh. Engine coordinates are Z-up millimetres; three.js is Y-up. */
export function EngineMesh(props: {
  mesh: MeshData;
  color?: string;
  opacity?: number;
  wireframe?: boolean;
  position?: [number, number, number];
  zUp?: boolean;
  metalness?: number;
  roughness?: number;
}) {
  const { mesh, zUp = true } = props;
  const geom = useMemo(() => {
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.Float32BufferAttribute(mesh.positions, 3));
    if (mesh.colors && mesh.colors.length === mesh.positions.length) {
      g.setAttribute('color', new THREE.Float32BufferAttribute(mesh.colors, 3));
    }
    g.setIndex(mesh.indices);
    g.computeVertexNormals();
    if (zUp) g.rotateX(-Math.PI / 2);
    return g;
  }, [mesh, zUp]);
  const vertexColors = !!mesh.colors && mesh.colors.length === mesh.positions.length;
  return (
    <mesh geometry={geom} position={props.position} castShadow receiveShadow>
      <meshStandardMaterial
        color={vertexColors ? '#ffffff' : props.color ?? '#9aa7b8'}
        vertexColors={vertexColors}
        metalness={props.metalness ?? 0.1}
        roughness={props.roughness ?? 0.55}
        transparent={(props.opacity ?? 1) < 1}
        opacity={props.opacity ?? 1}
        wireframe={props.wireframe}
        side={THREE.DoubleSide}
        flatShading={false}
      />
    </mesh>
  );
}

/** Line segments from a flat array [x0,y0,z0,x1,y1,z1,...] (pairs). */
export function Segments(props: { points: number[]; color?: string; zUp?: boolean; opacity?: number }) {
  const { zUp = true } = props;
  const geom = useMemo(() => {
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.Float32BufferAttribute(props.points, 3));
    if (zUp) g.rotateX(-Math.PI / 2);
    return g;
  }, [props.points, zUp]);
  return (
    <lineSegments geometry={geom}>
      <lineBasicMaterial color={props.color ?? '#3987e5'} transparent opacity={props.opacity ?? 1} />
    </lineSegments>
  );
}
