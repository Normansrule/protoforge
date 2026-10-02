// 3-D temperature view: the real CAD surfaces of the right side of the frame
// (pod, lid, optics housing, front) and the board / display / spreader boxes,
// coloured per pixel from a 3-D texture of the solved nodal temperatures (the
// shader samples the voxel field at each fragment's position, trilinearly).
// Engine coordinates (mm, Z up) inside the CAD Studio's canvas.
import { useEffect, useMemo, useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { CadCanvas, useEngineGeometry, useTheme, type ViewPreset } from '../cad/viewer';
import type { Surface, V3 } from './types';
import { TEMP_STOPS, b64F32, b64U32, prefersReducedMotion } from './util';

export interface DecodedSurface {
  name: string;
  label: string;
  pos: Float32Array;
  idx: Uint32Array;
}

export function decodeSurfaces(s: Surface[]): DecodedSurface[] {
  return s.map((x) => ({ name: x.name, label: x.label, pos: b64F32(x.positions), idx: b64U32(x.indices) }));
}

/** Nodal temperature volume (node counts, origin of node 0, pitch, u8 values over [t_lo, t_hi]). */
export interface VolumeData {
  nx: number;
  ny: number;
  nz: number;
  origin: V3;
  h_mm: number;
  t_lo: number;
  t_hi: number;
  bytes: Uint8Array;
}

const RAMP_LINEAR = TEMP_STOPS.map((h) => new THREE.Color(h)); // THREE.Color converts sRGB hex to linear

/** Shared uniforms + 3-D texture for one volume; data updates in place for animation. */
function useVolumeUniforms(vol: VolumeData, lo: number, hi: number) {
  const tex = useMemo(() => {
    const t = new THREE.Data3DTexture(new Uint8Array(vol.bytes), vol.nx, vol.ny, vol.nz);
    t.format = THREE.RedFormat;
    t.type = THREE.UnsignedByteType;
    t.minFilter = THREE.LinearFilter;
    t.magFilter = THREE.LinearFilter;
    t.wrapS = t.wrapT = t.wrapR = THREE.ClampToEdgeWrapping;
    t.unpackAlignment = 1;
    t.needsUpdate = true;
    return t;
    // a new texture only when the grid changes
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [vol.nx, vol.ny, vol.nz]);
  useEffect(() => () => tex.dispose(), [tex]);
  useEffect(() => {
    const d = tex.image.data as Uint8Array;
    if (d.length === vol.bytes.length) {
      d.set(vol.bytes);
      tex.needsUpdate = true;
    }
  }, [tex, vol.bytes]);
  const uniforms = useMemo(
    () => ({
      uVol: { value: tex },
      uOrigin: { value: new THREE.Vector3() },
      uExtent: { value: new THREE.Vector3(1, 1, 1) },
      uDims: { value: new THREE.Vector3(1, 1, 1) },
      uMap: { value: new THREE.Vector2(1, 0) },
      uRamp: { value: RAMP_LINEAR.map((c) => new THREE.Vector3(c.r, c.g, c.b)) },
    }),
    [tex],
  );
  uniforms.uOrigin.value.set(vol.origin[0], vol.origin[1], vol.origin[2]);
  uniforms.uExtent.value.set((vol.nx - 1) * vol.h_mm, (vol.ny - 1) * vol.h_mm, (vol.nz - 1) * vol.h_mm);
  uniforms.uDims.value.set(vol.nx, vol.ny, vol.nz);
  const span = Math.max(1e-6, hi - lo);
  uniforms.uMap.value.set((vol.t_hi - vol.t_lo) / span, (vol.t_lo - lo) / span);
  return uniforms;
}

type Uniforms = ReturnType<typeof useVolumeUniforms>;

function makeMaterial(u: Uniforms, opacity: number, wire: boolean): THREE.MeshStandardMaterial {
  const transparent = opacity < 1;
  const m = new THREE.MeshStandardMaterial({
    roughness: 0.6,
    metalness: 0.04,
    transparent,
    opacity,
    depthWrite: !transparent,
    side: THREE.DoubleSide,
    wireframe: wire,
  });
  m.onBeforeCompile = (shader) => {
    Object.assign(shader.uniforms, u);
    shader.vertexShader = shader.vertexShader
      .replace('#include <common>', '#include <common>\nvarying vec3 vEng;')
      .replace('#include <begin_vertex>', '#include <begin_vertex>\nvEng = position;');
    shader.fragmentShader = shader.fragmentShader
      .replace(
        '#include <common>',
        `#include <common>
varying vec3 vEng;
uniform highp sampler3D uVol;
uniform vec3 uOrigin;
uniform vec3 uExtent;
uniform vec3 uDims;
uniform vec2 uMap;
uniform vec3 uRamp[8];
vec3 fluidsRamp(float t) {
  t = clamp(t, 0.0, 1.0) * 7.0;
  int i = int(min(floor(t), 6.0));
  return mix(uRamp[i], uRamp[i + 1], t - float(i));
}`,
      )
      .replace(
        'vec4 diffuseColor = vec4( diffuse, opacity );',
        `vec3 fluidsUvw = (vEng - uOrigin) / uExtent * (uDims - 1.0) / uDims + 0.5 / uDims;
float fluidsV = texture(uVol, fluidsUvw).r;
vec4 diffuseColor = vec4( fluidsRamp(fluidsV * uMap.x + uMap.y), opacity );`,
      );
  };
  m.customProgramCacheKey = () => 'fluids-volume-v1';
  return m;
}

function VolumeMesh({ s, u, opacity, wire }: { s: DecodedSurface; u: Uniforms; opacity: number; wire: boolean }) {
  const geom = useEngineGeometry(useMemo(() => ({ positions: s.pos, indices: s.idx }), [s]), 30);
  const mat = useMemo(() => makeMaterial(u, opacity, wire), [u, opacity, wire]);
  useEffect(() => () => mat.dispose(), [mat]);
  return <mesh geometry={geom} material={mat} renderOrder={opacity < 1 ? 2 : 0} />;
}

function Pulse({ pos, r, color }: { pos: V3; r: number; color: string }) {
  const ref = useRef<THREE.Mesh>(null);
  const t = useRef(0);
  const still = prefersReducedMotion();
  useFrame((_, dt) => {
    if (still) return;
    t.current += dt;
    ref.current?.scale.setScalar(1 + 0.35 * Math.sin(t.current * 4));
  });
  return (
    <mesh ref={ref} position={pos}>
      <sphereGeometry args={[r, 18, 12]} />
      <meshBasicMaterial color={color} wireframe />
    </mesh>
  );
}

export interface ViewOpts {
  xrayLid: boolean;
  xrayShell: boolean;
  inner: boolean;
  markers: boolean;
  wire: boolean;
}

export function ThermalView(props: {
  surfaces: DecodedSurface[];
  inner: DecodedSurface[];
  volume: VolumeData;
  lo: number;
  hi: number;
  bbox: { min: V3; max: V3 };
  opts: ViewOpts;
  peak?: V3 | null;
  sources?: { pos: V3; mw: number }[];
  height: number | string;
  preset: ViewPreset;
  nonce: number;
  overlay?: React.ReactNode;
}) {
  const theme = useTheme();
  const { opts } = props;
  const u = useVolumeUniforms(props.volume, props.lo, props.hi);
  const markColor = theme === 'light' ? '#0b0b0b' : '#ffffff';
  const srcColor = theme === 'light' ? '#1c5cab' : '#86b6ef';
  const maxMw = Math.max(1e-9, ...(props.sources ?? []).map((s) => s.mw));
  return (
    <CadCanvas bounds={{ min: props.bbox.min, max: props.bbox.max }} height={props.height} preset={props.preset} nonce={props.nonce} fit={1.15} shadows={false} overlay={props.overlay}>
      {props.surfaces.map((s) => {
        const isLid = s.name === 'lid_r';
        const op = (isLid && opts.xrayLid) || (!isLid && opts.xrayShell) ? 0.22 : 1;
        return <VolumeMesh key={s.name} s={s} u={u} opacity={op} wire={opts.wire} />;
      })}
      {opts.inner && props.inner.map((s) => <VolumeMesh key={s.name} s={s} u={u} opacity={1} wire={false} />)}
      {opts.markers && props.peak && <Pulse pos={props.peak} r={1.6} color={markColor} />}
      {opts.markers &&
        (props.sources ?? [])
          .filter((s) => s.mw > 0.05)
          .map((s, i) => (
            <mesh key={i} position={s.pos} renderOrder={5}>
              <sphereGeometry args={[0.35 + 1.1 * Math.sqrt(s.mw / maxMw), 14, 10]} />
              <meshBasicMaterial color={srcColor} transparent opacity={0.85} depthTest={false} />
            </mesh>
          ))}
    </CadCanvas>
  );
}

/** Colour legend overlay (temperature ramp with ticks). */
export function TempLegend({ lo, hi, limit, title }: { lo: number; hi: number; limit?: number; title?: string }) {
  const ticks = 5;
  const lim = limit != null && limit > lo && limit < hi ? (limit - lo) / (hi - lo) : null;
  return (
    <div className="fl-legend" aria-label="temperature colour scale">
      <div className="small" style={{ fontWeight: 600 }}>{title ?? 'Temperature, °C'}</div>
      <div className="fl-legend-bar" style={{ position: 'relative' }}>
        {lim != null && <div className="fl-legend-limit" style={{ left: `${lim * 100}%` }} title={`skin limit ${limit} °C`} />}
      </div>
      <div className="fl-legend-ticks">
        {Array.from({ length: ticks }, (_, i) => (
          <span key={i}>{(lo + ((hi - lo) * i) / (ticks - 1)).toFixed(1)}</span>
        ))}
      </div>
      {lim != null && <div className="small muted">┆ marks the {limit} °C skin limit</div>}
    </div>
  );
}
