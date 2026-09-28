import { useMemo } from 'react';
import { Card, EmptyState } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { Scene3D, EngineMesh } from '../../components/Scene3D';
import type { Footprint, Layout } from './types';

function FootprintSvg({ f }: { f: Footprint }) {
  const c = f.courtyard;
  const pad = 0.6;
  const vb = [c[0] - pad, -(c[3] + pad), c[2] - c[0] + 2 * pad, c[3] - c[1] + 2 * pad];
  const sw = Math.max(vb[2], vb[3]) / 250;
  return (
    <svg viewBox={vb.join(' ')} width="100%" height={130} style={{ display: 'block', background: 'var(--surface-2)', borderRadius: 8 }} role="img" aria-label={f.name}>
      <rect x={c[0]} y={-c[3]} width={c[2] - c[0]} height={c[3] - c[1]} fill="none" stroke="var(--muted)" strokeWidth={sw} strokeDasharray={`${sw * 4} ${sw * 3}`} />
      <rect x={f.body[0]} y={-f.body[3]} width={f.body[2] - f.body[0]} height={f.body[3] - f.body[1]} fill="none" stroke="var(--text-2)" strokeWidth={sw * 1.5} />
      {f.pads.map((p, i) => p.shape === 'circle'
        ? <g key={i}><circle cx={p.x} cy={-p.y} r={p.w / 2} fill={p.mech ? 'var(--muted)' : 'var(--copper)'} />{p.drill > 0 && <circle cx={p.x} cy={-p.y} r={p.drill / 2} fill="var(--surface-2)" />}</g>
        : <rect key={i} x={p.x - p.w / 2} y={-p.y - p.h / 2} width={p.w} height={p.h} rx={p.shape === 'round_rect' ? Math.min(p.w, p.h) * 0.2 : 0} fill={p.mech ? 'var(--muted)' : 'var(--copper)'} />)}
      {f.npth.map((h, i) => <circle key={`n${i}`} cx={h[0]} cy={-h[1]} r={h[2] / 2} fill="none" stroke="var(--series-8)" strokeWidth={sw * 1.5} />)}
      <circle cx={f.pin1[0]} cy={-f.pin1[1]} r={Math.max(0.15, sw * 4)} fill="var(--bad)" />
    </svg>
  );
}

export function FootprintsTab() {
  const lib = useCompute<{ footprints: Footprint[]; features: Footprint[] }>('pcb.footprints', {});
  if (!lib.data) return <EmptyState>Loading library…</EmptyState>;
  return (
    <div className="stack">
      <div className="callout small">Typical IPC-7351-style land patterns, approximated for teaching. <b>Verify every footprint against the part datasheet's recommended land pattern</b> (or KiCad's library / an IPC-7351 calculator) before ordering. Copper pads in copper, mechanical pads grey, dashed = courtyard, red dot = pin 1, red ring = non-plated hole.</div>
      <div className="grid grid-4">
        {[...lib.data.footprints, ...lib.data.features].map((f) => (
          <Card key={f.name} title={<span className="mono">{f.name}</span>} subtitle={f.description}>
            <FootprintSvg f={f} />
            <div className="small muted" style={{ marginTop: 6 }}>{f.pads.length} pads · courtyard {(f.courtyard[2] - f.courtyard[0]).toFixed(2)} × {(f.courtyard[3] - f.courtyard[1]).toFixed(2)} mm</div>
            {f.note && <div className="small" style={{ marginTop: 4 }}>{f.note}</div>}
          </Card>
        ))}
      </div>
    </div>
  );
}

/** Box mesh helper (engine-style flat arrays, Z up, mm). */
function box(x0: number, y0: number, z0: number, x1: number, y1: number, z1: number) {
  const v = [[x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0], [x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]];
  return { positions: v.flat(), indices: [0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0, 4, 3, 4, 7] };
}

export function Board3D({ layout }: { layout: Layout }) {
  const W = layout.board.width, H = layout.board.height, T = layout.board.thickness;
  const cx = W / 2, cy = H / 2;
  const parts = useMemo(() => layout.parts.filter((p) => !p.feature).map((p) => {
    const b = p.body;
    const ic = p.footprint.startsWith('QFN') || p.footprint.startsWith('LGA') || p.footprint.startsWith('SOT') || p.footprint.startsWith('DFN') || p.footprint.startsWith('USON') || p.footprint.startsWith('MIC');
    const conn = p.footprint.startsWith('USB') || p.footprint.startsWith('FPC') || p.footprint.startsWith('JST');
    const color = conn ? '#b9bcc4' : ic ? '#2b2b30' : p.footprint.startsWith('ANT') ? '#e6e1d3' : '#a88963';
    return { key: p.refdes, mesh: box(b[0] - cx, b[1] - cy, T / 2, b[2] - cx, b[3] - cy, T / 2 + Math.max(p.height, 0.2)), color };
  }), [layout.parts, cx, cy, T]);
  const copper = useMemo(() => layout.pads.filter((p) => p.top).map((p, i) => ({ key: i, mesh: box(p.x - p.w / 2 - cx, p.y - p.h / 2 - cy, T / 2, p.x + p.w / 2 - cx, p.y + p.h / 2 - cy, T / 2 + 0.035) })), [layout.pads, cx, cy, T]);
  return (
    <Scene3D height={360} camera={[20, 45, 45]} grid={false}>
      <EngineMesh mesh={box(-cx, -cy, -T / 2, cx, cy, T / 2)} color="#1f6f3f" roughness={0.6} />
      {copper.map((c) => <EngineMesh key={c.key} mesh={c.mesh} color="#d9a55a" metalness={0.6} roughness={0.35} />)}
      {parts.map((p) => <EngineMesh key={p.key} mesh={p.mesh} color={p.color} metalness={p.color === '#b9bcc4' ? 0.7 : 0.1} roughness={0.45} />)}
    </Scene3D>
  );
}
