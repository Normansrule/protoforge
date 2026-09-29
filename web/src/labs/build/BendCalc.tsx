// Sheet-metal bend calculator: animated forming from the flat pattern, with
// bend allowance, setback and deduction drawn to scale.
import { useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat, Toggle } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { PlayButton, usePlaying, useTicker } from './anim';
import type { BendOut, SheetMaterial } from './types';

function Bend({ prog, i, o }: { prog: number; i: { thickness_mm: number; radius_mm: number; angle_deg: number; flange1_mm: number; flange2_mm: number }; o: BendOut }) {
  // scale the part to fit a 360 × 230 box
  const total = o.flat_length_mm;
  const s = Math.min(300 / Math.max(1, total), 12);
  const T = i.thickness_mm * s;
  const Ri = i.radius_mm * s;
  const th = ((i.angle_deg * Math.PI) / 180) * prog;
  const K = o.k_factor;
  const a1 = Math.max(0, o.flat_straight1_mm * s);
  const a2 = Math.max(0, o.flat_straight2_mm * s);
  // Build the formed centreline of each surface: straight a1 along +x, arc of radius r turning up by th, straight a2
  const path = (r: number) => {
    const x0 = 30, y0 = 150;
    const cx = x0 + a1, cy = y0 - Ri; // bend centre (inside radius measured from the inner surface at y0 - 0)
    const pts: string[] = [`M${x0},${cy + r}`, `L${cx},${cy + r}`];
    // start of arc is directly below the centre at distance r
    const ex = cx + r * Math.sin(th), ey = cy + r * Math.cos(th);
    pts.push(`A${r},${r} 0 0 0 ${ex},${ey}`);
    pts.push(`L${ex + a2 * Math.cos(th)},${ey - a2 * Math.sin(th)}`);
    return pts.join(' ');
  };
  // the inner surface sits at radius Ri from the centre; outer at Ri + T; neutral at Ri + K·T
  // (the centre is above the sheet; the sheet bends upwards)
  const inner = path(Ri);
  const outer = path(Ri + T);
  const neutral = path(Ri + K * T);
  const flatY = 200;
  return (
    <svg viewBox="0 0 360 230" className="bd-anim" role="img" aria-label="Sheet bending from flat to the bend angle">
      <path d={outer} fill="none" stroke="var(--series-1)" strokeWidth={1.5} />
      <path d={inner} fill="none" stroke="var(--series-1)" strokeWidth={1.5} />
      <path d={neutral} fill="none" stroke="var(--warn)" strokeDasharray="4 3" />
      <text x={30} y={20} className="bd-svg-small">formed: {(i.angle_deg * prog).toFixed(0)}° · dashed = neutral axis at K·T = {(K * i.thickness_mm).toFixed(2)} mm</text>
      {/* flat pattern */}
      <rect x={30} y={flatY} width={a1} height={Math.max(2, T)} fill="var(--series-1)" opacity={0.35} />
      <rect x={30 + a1} y={flatY} width={Math.max(0, o.bend_allowance_mm * s)} height={Math.max(2, T)} fill="var(--warn)" opacity={0.6} />
      <rect x={30 + a1 + o.bend_allowance_mm * s} y={flatY} width={a2} height={Math.max(2, T)} fill="var(--series-1)" opacity={0.35} />
      <text x={30} y={flatY - 6} className="bd-svg-small">flat pattern {o.flat_length_mm.toFixed(2)} mm — amber = bend allowance {o.bend_allowance_mm.toFixed(2)} mm</text>
    </svg>
  );
}

export function BendCalc() {
  const mats = useCompute<SheetMaterial[]>('build.sheet_materials', {});
  const [i, setI] = useState({ thickness_mm: 1.5, radius_mm: 1.5, angle_deg: 90, flange1_mm: 30, flange2_mm: 20, material: 'al5052' });
  const [autoK, setAutoK] = useState(true);
  const [k, setK] = useState(0.44);
  const r = useCompute<BendOut>('build.bend', { ...i, k_factor: autoK ? null : k });
  const [playing, setPlaying] = usePlaying();
  const t = useTicker(playing);
  const ph = (t / 4) % 1;
  const prog = playing ? (ph < 0.5 ? ph * 2 : ph < 0.8 ? 1 : 1 - (ph - 0.8) * 5) : 1;
  const o = r.data;
  const set = (p: Partial<typeof i>) => setI({ ...i, ...p });
  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Bend">
          <Select label="Material" value={i.material} onChange={(v) => set({ material: v })} options={(mats.data ?? []).map((m) => ({ value: m.id, label: m.name }))} />
          <Slider label="Thickness T" unit="mm" value={i.thickness_mm} min={0.3} max={6} onChange={(v) => set({ thickness_mm: v })} format={(v) => v.toFixed(2)} />
          <Slider label="Inside radius R" unit="mm" value={i.radius_mm} min={0} max={12} onChange={(v) => set({ radius_mm: v })} format={(v) => v.toFixed(2)} />
          <Slider label="Bend angle θ" unit="°" value={i.angle_deg} min={10} max={170} step={1} onChange={(v) => set({ angle_deg: v })} format={(v) => v.toFixed(0)} />
          <Slider label="Flange 1 (to virtual sharp)" unit="mm" value={i.flange1_mm} min={2} max={150} onChange={(v) => set({ flange1_mm: v })} format={(v) => v.toFixed(1)} />
          <Slider label="Flange 2 (to virtual sharp)" unit="mm" value={i.flange2_mm} min={2} max={150} onChange={(v) => set({ flange2_mm: v })} format={(v) => v.toFixed(1)} />
          <Toggle label="Estimate K-factor from R/T (DIN 6935)" value={autoK} onChange={setAutoK} />
          {!autoK && <Slider label="K-factor" value={k} min={0.2} max={0.5} step={0.01} onChange={setK} format={(v) => v.toFixed(2)} />}
        </Card>
        <div className="stack">
          <ErrorBox error={r.error} />
          {o && (
            <>
              <div className="grid grid-4">
                <Stat label="Flat length" value={o.flat_length_mm.toFixed(2)} unit="mm" note={`${i.flange1_mm} + ${i.flange2_mm} − BD`} />
                <Stat label="Bend allowance" value={o.bend_allowance_mm.toFixed(3)} unit="mm" note={`K = ${o.k_factor.toFixed(3)} (${o.k_source})`} />
                <Stat label="Bend deduction" value={o.bend_deduction_mm.toFixed(3)} unit="mm" />
                <Stat label="Outside setback" value={o.outside_setback_mm.toFixed(3)} unit="mm" />
              </div>
              <Card title="Forming and flat pattern" actions={<PlayButton playing={playing} onChange={setPlaying} />}>
                <Bend prog={prog} i={i} o={o} />
                <div className="stack" style={{ gap: 4, marginTop: 8 }}>
                  {o.checks.map(([l, ok]) => <div key={l} className="small"><Chip tone={ok ? 'pass' : 'warn'}>{ok ? '✓ ok' : '! check'}</Chip> {l}</div>)}
                  <div className="small muted">Holes ≳ {o.hole_to_bend_mm.toFixed(1)} mm from the bend (2.5·T + R); bend relief width ≥ T, depth ≥ R + T. Typical rules — ask your shop.</div>
                </div>
                <DownloadButton filename="bend.md" content={`# Bend calculation\n\nT = ${i.thickness_mm} mm, R = ${i.radius_mm} mm, θ = ${i.angle_deg}°, K = ${o.k_factor.toFixed(3)} (${o.k_source})\n\n- BA = θ·(R + K·T) = ${o.bend_allowance_mm.toFixed(4)} mm\n- OSSB = tan(θ/2)·(R + T) = ${o.outside_setback_mm.toFixed(4)} mm\n- BD = 2·OSSB − BA = ${o.bend_deduction_mm.toFixed(4)} mm\n- Flat = ${i.flange1_mm} + ${i.flange2_mm} − BD = ${o.flat_length_mm.toFixed(4)} mm\n`} label="bend.md" small />
              </Card>
            </>
          )}
        </div>
      </div>
      <Explainer title="Bend allowance, setback and deduction" refs={[{ label: "Machinery's Handbook — sheet-metal bending" }, { label: 'DIN 6935 — cold bending of flat steel products' }]}>
        When sheet bends, the inside compresses and the outside stretches; somewhere between lies the <b>neutral axis</b>, which keeps its length. Its position is the K-factor (fraction of T from the inside). The flat blank is the neutral-axis length:
        <Formula>{'BA = θ · (R + K·T)\nOSSB = tan(θ/2) · (R + T)\nBD = 2·OSSB − BA\nflat = L1 + L2 − BD      (L1, L2 to the virtual sharp)\nDIN 6935: K = ½ · (0.65 + ½·log₁₀(R/T)),  K = 0.5 for R/T ≥ 5'}</Formula>
        Tight bends (small R/T) shift the neutral axis towards the inside (K ≈ 0.3–0.4); gentle bends approach the middle (K → 0.5). Shops calibrate K from test bends on their own press brake.
      </Explainer>
    </div>
  );
}
