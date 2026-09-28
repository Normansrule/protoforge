// 3D view of a voxel FEA result: heat-mapped surface (CAD mesh or voxel
// boundary), an animated exaggerated deformation, load arrows, clamped-support
// markers and a pulsing marker at the critical point. Everything is placed in
// engine coordinates (mm, Z up) inside the CAD Studio's canvas.
import { useEffect, useMemo, useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { CadCanvas, cssVar, useTheme } from '../cad/viewer';
import type { FeaResult, SurfaceField, V3 } from './types';
import { CRITICAL, heat } from './util';

export type ColorMode = 'vm' | 'util';

export interface ViewOpts {
  surface: 'cad' | 'voxels';
  mode: ColorMode;
  exaggeration: number;
  playing: boolean;
  showLoads: boolean;
  showFixed: boolean;
  showCrit: boolean;
  wire: boolean;
}

/** Automatic scale so the peak displacement reads as ~8 % of the part size. */
export function autoScale(r: FeaResult): number {
  const size = Math.hypot(r.bbox_max[0] - r.bbox_min[0], r.bbox_max[1] - r.bbox_min[1], r.bbox_max[2] - r.bbox_min[2]);
  return r.max_disp_mm > 0 ? (0.08 * size) / r.max_disp_mm : 1;
}

function colorsFor(f: SurfaceField, mode: ColorMode, vmax: number): Float32Array {
  const n = f.positions.length / 3;
  const out = new Float32Array(n * 3);
  const crit = new THREE.Color(CRITICAL);
  for (let i = 0; i < n; i++) {
    let rgb: [number, number, number];
    if (mode === 'util') {
      const u = f.util[i];
      rgb = u >= 1 ? [crit.r, crit.g, crit.b] : heat(u);
    } else {
      rgb = heat(vmax > 0 ? f.vm[i] / vmax : 0);
    }
    // three.js expects linear colours for vertex colours.
    const c = new THREE.Color().setRGB(rgb[0], rgb[1], rgb[2], THREE.SRGBColorSpace);
    out[3 * i] = c.r;
    out[3 * i + 1] = c.g;
    out[3 * i + 2] = c.b;
  }
  return out;
}

function DeformingMesh({ field, mode, vmax, scale, opts }: { field: SurfaceField; mode: ColorMode; vmax: number; scale: number; opts: ViewOpts }) {
  const phase = useRef(opts.playing ? 0 : 1);
  const time = useRef(0);
  const base = useMemo(() => new Float32Array(field.positions), [field]);
  const geom = useMemo(() => {
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.BufferAttribute(new Float32Array(field.positions), 3));
    g.setIndex(field.indices);
    g.computeBoundingSphere();
    return g;
  }, [field]);
  useEffect(() => {
    geom.setAttribute('color', new THREE.BufferAttribute(colorsFor(field, mode, vmax), 3));
  }, [geom, field, mode, vmax]);
  useEffect(() => () => geom.dispose(), [geom]);
  const apply = (ph: number) => {
    const pos = geom.getAttribute('position') as THREE.BufferAttribute;
    const arr = pos.array as Float32Array;
    const k = ph * scale;
    for (let i = 0; i < arr.length; i++) arr[i] = base[i] + k * field.disp[i];
    pos.needsUpdate = true;
  };
  useEffect(() => {
    if (!opts.playing) {
      phase.current = 1;
      apply(1);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [opts.playing, scale, geom]);
  useFrame((_, dt) => {
    if (!opts.playing) return;
    time.current += dt;
    // Load → hold → unload, 2.4 s period.
    const t = (time.current % 2.4) / 2.4;
    phase.current = 0.5 - 0.5 * Math.cos(2 * Math.PI * t);
    apply(phase.current);
  });
  return (
    <mesh geometry={geom}>
      <meshStandardMaterial vertexColors flatShading roughness={0.65} metalness={0.05} side={THREE.DoubleSide} wireframe={opts.wire} />
    </mesh>
  );
}

function Arrow({ pos, dir, len, color }: { pos: V3; dir: V3; len: number; color: string }) {
  // Arrow pointing along dir, tip at pos.
  const quat = useMemo(() => new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0, 1, 0), new THREE.Vector3(...dir).normalize()), [dir]);
  const tail = new THREE.Vector3(...pos).addScaledVector(new THREE.Vector3(...dir).normalize(), -len);
  const head = len * 0.3;
  const r = len * 0.05;
  return (
    <group position={tail} quaternion={quat}>
      <mesh position={[0, (len - head) / 2, 0]}>
        <cylinderGeometry args={[r, r, len - head, 10]} />
        <meshStandardMaterial color={color} />
      </mesh>
      <mesh position={[0, len - head / 2, 0]}>
        <coneGeometry args={[r * 2.6, head, 14]} />
        <meshStandardMaterial color={color} />
      </mesh>
    </group>
  );
}

function FixedMarkers({ pts, size, color }: { pts: V3[]; size: number; color: string }) {
  const ref = useRef<THREE.InstancedMesh>(null);
  useEffect(() => {
    const m = ref.current;
    if (!m) return;
    const o = new THREE.Object3D();
    pts.forEach((p, i) => {
      o.position.set(p[0], p[1], p[2]);
      o.updateMatrix();
      m.setMatrixAt(i, o.matrix);
    });
    m.instanceMatrix.needsUpdate = true;
  }, [pts]);
  return (
    <instancedMesh ref={ref} args={[undefined, undefined, pts.length]} key={pts.length}>
      <boxGeometry args={[size, size, size]} />
      <meshStandardMaterial color={color} transparent opacity={0.35} depthWrite={false} />
    </instancedMesh>
  );
}

function CritMarker({ pos, r, color }: { pos: V3; r: number; color: string }) {
  const ref = useRef<THREE.Mesh>(null);
  const t = useRef(0);
  useFrame((_, dt) => {
    t.current += dt;
    const s = 1 + 0.35 * Math.sin(t.current * 4);
    ref.current?.scale.setScalar(s);
  });
  return (
    <mesh ref={ref} position={pos}>
      <sphereGeometry args={[r, 20, 14]} />
      <meshBasicMaterial color={color} wireframe />
    </mesh>
  );
}

export function FeaView({ r, opts, height, nonce, preset }: { r: FeaResult; opts: ViewOpts; height: number | string; nonce: number; preset: 'iso' | 'front' | 'side' | 'top' | 'back' }) {
  const theme = useTheme();
  const field = opts.surface === 'voxels' ? r.voxels : r.cad;
  const scale = autoScale(r) * opts.exaggeration;
  const size = Math.hypot(r.bbox_max[0] - r.bbox_min[0], r.bbox_max[1] - r.bbox_min[1], r.bbox_max[2] - r.bbox_min[2]);
  const vmax = r.max_vm_mpa > 0 ? r.max_vm_mpa : 1;
  const loadColor = theme === 'light' ? '#1c5cab' : '#86b6ef';
  const fixColor = cssVar('--muted', '#888');
  // Show at most 6 arrows.
  const arrows = r.loads.filter((_, i) => i % Math.max(1, Math.ceil(r.loads.length / 6)) === 0);
  return (
    <CadCanvas bounds={{ min: r.bbox_min, max: r.bbox_max }} height={height} preset={preset} nonce={nonce} fit={1.1} shadows={false}>
      {field && <DeformingMesh field={field} mode={opts.mode} vmax={vmax} scale={scale} opts={opts} />}
      {opts.showLoads && arrows.map((a, i) => <Arrow key={i} pos={a.pos} dir={a.dir} len={Math.max(6, size * 0.12)} color={loadColor} />)}
      {opts.showFixed && r.fixed.length > 0 && <FixedMarkers pts={r.fixed} size={r.h_mm * 1.05} color={fixColor} />}
      {opts.showCrit && <CritMarker pos={r.crit_pos} r={Math.max(1.2, size * 0.025)} color={theme === 'light' ? '#0b0b0b' : '#ffffff'} />}
    </CadCanvas>
  );
}
