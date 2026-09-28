// 3D stack of toolpaths: one line-segment buffer per move kind for the whole
// part, revealed layer by layer with drawRange (no re-upload while playing).
import { useEffect, useMemo } from 'react';
import * as THREE from 'three';
import { KINDS, type LayerOut } from './api';
import { cssVar, useTheme } from '../cad/viewer';

interface KindGeom {
  geom: THREE.BufferGeometry;
  /** vertex count at the end of each layer */
  ends: number[];
}

export function buildToolpaths(layers: LayerOut[]): KindGeom[] {
  const buf: number[][] = KINDS.map(() => []);
  const ends: number[][] = KINDS.map(() => []);
  for (const l of layers) {
    for (const p of l.paths) {
      const b = buf[p.k] ?? buf[2];
      for (let i = 2; i < p.p.length; i += 2) {
        b.push(p.p[i - 2] / 100, p.p[i - 1] / 100, l.z, p.p[i] / 100, p.p[i + 1] / 100, l.z);
      }
    }
    buf.forEach((b, k) => ends[k].push(b.length / 3));
  }
  return buf.map((b, k) => {
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.Float32BufferAttribute(b, 3));
    return { geom: g, ends: ends[k] };
  });
}

export function ToolpathStack(props: { geoms: KindGeom[]; upto: number; showTravel: boolean; current: LayerOut | null }) {
  const theme = useTheme();
  const colors = useMemo(() => KINDS.map((k) => cssVar(k.color)), [theme]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    for (const kg of props.geoms) {
      const end = props.upto >= 0 ? kg.ends[Math.min(props.upto, kg.ends.length - 1)] ?? 0 : 0;
      kg.geom.setDrawRange(0, end);
    }
  }, [props.geoms, props.upto]);
  useEffect(() => () => props.geoms.forEach((g) => g.geom.dispose()), [props.geoms]);
  const currentGeom = useMemo(() => {
    if (!props.current) return null;
    const b: number[] = [];
    for (const p of props.current.paths) {
      if (p.k === 0) continue;
      for (let i = 2; i < p.p.length; i += 2) b.push(p.p[i - 2] / 100, p.p[i - 1] / 100, props.current.z + 0.01, p.p[i] / 100, p.p[i + 1] / 100, props.current.z + 0.01);
    }
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.Float32BufferAttribute(b, 3));
    return g;
  }, [props.current]);
  return (
    <>
      {props.geoms.map((kg, k) =>
        k === 0 && !props.showTravel ? null : (
          <lineSegments key={k} geometry={kg.geom}>
            <lineBasicMaterial color={colors[k]} transparent opacity={k === 0 ? 0.25 : 0.55} />
          </lineSegments>
        ),
      )}
      {currentGeom && (
        <lineSegments geometry={currentGeom}>
          <lineBasicMaterial color={theme === 'light' ? '#0b0b0b' : '#ffffff'} transparent opacity={0.9} />
        </lineSegments>
      )}
    </>
  );
}
