// Feeds & speeds calculator with a cutter that spins at a rate proportional
// to the computed RPM (slowed 1000×) and a radial-engagement diagram.
import { useMemo, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, NumberField, Select, Slider, Stat, Toggle } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { PlayButton, usePlaying, useTicker } from './anim';
import type { FeedsOut, MachMaterial } from './types';

const SLOW = 1000;

function Cutter({ t, rpm, flutes, d, woc, chip }: { t: number; rpm: number; flutes: number; d: number; woc: number; chip: number }) {
  const R = 70;
  const cx = 110, cy = 110;
  const ang = (t * rpm) / 60 / SLOW * 2 * Math.PI;
  const e = Math.min(1, woc / d);
  // engaged arc: stock on the right side (conventional view), from the cutter edge inwards by woc
  const xCut = cx + R - 2 * R * e;
  const phi = Math.acos(Math.max(-1, Math.min(1, (xCut - cx) / R)));
  const chips = [];
  for (let k = 0; k < flutes; k++) {
    const a = ang + (k * 2 * Math.PI) / flutes;
    const am = ((a % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI);
    const inCut = am < phi || am > 2 * Math.PI - phi;
    chips.push({ a, inCut });
  }
  const ct = Math.max(1.5, Math.min(10, chip * 120));
  return (
    <svg viewBox="0 0 360 220" className="bd-anim" role="img" aria-label="Top view of the end mill and the radial engagement">
      <rect x={xCut} y={10} width={360 - xCut} height={200} fill="var(--surface-3)" />
      <text x={Math.min(300, xCut + 8)} y={26} className="bd-svg-small">stock</text>
      <line x1={xCut} y1={10} x2={xCut} y2={210} stroke="var(--text-2)" strokeDasharray="4 3" />
      <path d={`M${cx},${cy} L${cx + R * Math.cos(phi)},${cy - R * Math.sin(phi)} A${R},${R} 0 ${phi > Math.PI / 2 ? 1 : 0} 1 ${cx + R * Math.cos(phi)},${cy + R * Math.sin(phi)} Z`} fill="var(--warn)" opacity={0.18} />
      <circle cx={cx} cy={cy} r={R} fill="var(--surface-2)" stroke="var(--border-strong)" />
      {chips.map(({ a, inCut }, k) => (
        <g key={k}>
          <path d={`M${cx},${cy} L${cx + R * Math.cos(a)},${cy - R * Math.sin(a)} A${R},${R} 0 0 0 ${cx + R * Math.cos(a - 0.7)},${cy - R * Math.sin(a - 0.7)} Z`} fill="var(--muted)" />
          {inCut && <path d={`M${cx + R * Math.cos(a)},${cy - R * Math.sin(a)} q${8 * Math.cos(a + 1)},${-8 * Math.sin(a + 1)} ${14 * Math.cos(a)},${-14 * Math.sin(a)}`} stroke="var(--copper)" strokeWidth={ct / 2} fill="none" strokeLinecap="round" />}
        </g>
      ))}
      <circle cx={cx} cy={cy} r={6} fill="var(--text-2)" />
      <text x={cx} y={cy + R + 22} textAnchor="middle" className="bd-svg-small">Ø{d.toFixed(1)} mm · {flutes} flutes · shown {SLOW}× slower</text>
      <g transform="translate(262,150)">
        <text x={0} y={0} className="bd-svg-small">engagement</text>
        <text x={0} y={16} className="bd-svg-label">{(100 * e).toFixed(0)} % of D</text>
        <text x={0} y={34} className="bd-svg-small">shaded arc:</text>
        <text x={0} y={46} className="bd-svg-small">flute in the cut</text>
      </g>
    </svg>
  );
}

/** Raster pocket path for the tool centre (mm), plus a finishing contour. */
function pocketPath(L: number, W: number, d: number, step: number): [number, number][] {
  const r = d / 2;
  const pts: [number, number][] = [];
  const ys: number[] = [];
  for (let y = r; y < W - r + 1e-9; y += Math.max(0.05, step)) ys.push(y);
  if (ys[ys.length - 1] < W - r - 1e-6) ys.push(W - r);
  ys.forEach((y, k) => {
    const a: [number, number] = [r, y], b: [number, number] = [L - r, y];
    if (k % 2 === 0) pts.push(a, b);
    else pts.push(b, a);
  });
  pts.push([r, W - r], [r, r], [L - r, r], [L - r, W - r], [r, W - r]);
  return pts;
}

function cum(pts: [number, number][]) {
  const c = [0];
  for (let k = 1; k < pts.length; k++) c.push(c[k - 1] + Math.hypot(pts[k][0] - pts[k - 1][0], pts[k][1] - pts[k - 1][1]));
  return c;
}

const POCKET_DEPTH = 6;

function ToolPath({ t, d, woc, doc, feed, flutes, rpm }: { t: number; d: number; woc: number; doc: number; feed: number; flutes: number; rpm: number }) {
  const L = Math.max(40, 6 * d), W = Math.max(25, 4 * d);
  const pts = useMemo(() => pocketPath(L, W, d, Math.min(woc, d)), [L, W, d, woc]);
  const c = useMemo(() => cum(pts), [pts]);
  const len = c[c.length - 1];
  const layers = Math.max(1, Math.ceil(POCKET_DEPTH / Math.max(0.05, doc)));
  const layerMin = feed > 0 ? len / feed : Infinity;
  const speedup = Number.isFinite(layerMin) ? Math.max(1, (layerMin * 60) / 7) : 1;
  const sNow = Number.isFinite(layerMin) ? ((t * speedup * feed) / 60) % len : 0;
  let k = 1;
  while (k < c.length - 1 && c[k] < sNow) k++;
  const f = (sNow - c[k - 1]) / Math.max(1e-9, c[k] - c[k - 1]);
  const cur: [number, number] = [pts[k - 1][0] + (pts[k][0] - pts[k - 1][0]) * f, pts[k - 1][1] + (pts[k][1] - pts[k - 1][1]) * f];
  const sc = Math.min(320 / L, 140 / W);
  const ox = (360 - L * sc) / 2;
  const X = (x: number) => ox + x * sc, Y = (y: number) => 26 + y * sc;
  const done = [...pts.slice(0, k), cur].map(([x, y]) => `${X(x).toFixed(1)},${Y(y).toFixed(1)}`).join(' ');
  const all = pts.map(([x, y]) => `${X(x).toFixed(1)},${Y(y).toFixed(1)}`).join(' ');
  const spin = (t * rpm) / 60 / 1000 * 2 * Math.PI;
  const R = (d / 2) * sc;
  return (
    <svg viewBox="0 0 360 220" className="bd-anim" role="img" aria-label="Top view of an end mill clearing a rectangular pocket along a raster tool path">
      <rect x={X(0)} y={Y(0)} width={L * sc} height={W * sc} fill="var(--surface-3)" stroke="var(--border-strong)" />
      <polyline points={all} fill="none" stroke="var(--muted)" strokeWidth={0.8} strokeDasharray="3 3" />
      <polyline points={done} fill="none" stroke="var(--surface-1)" strokeWidth={2 * R} strokeLinecap="round" strokeLinejoin="round" />
      <polyline points={done} fill="none" stroke="var(--accent)" strokeWidth={1} opacity={0.7} />
      <g transform={`translate(${X(cur[0])},${Y(cur[1])})`}>
        <circle r={R} fill="var(--muted)" stroke="var(--text-2)" />
        {Array.from({ length: flutes }, (_, n) => {
          const a = spin + (n * 2 * Math.PI) / flutes;
          return <line key={n} x1={0} y1={0} x2={R * Math.cos(a)} y2={R * Math.sin(a)} stroke="var(--surface-1)" strokeWidth={Math.max(1, R / 5)} />;
        })}
      </g>
      <text x={8} y={16} className="bd-svg-small">pocket {L.toFixed(0)} × {W.toFixed(0)} × {POCKET_DEPTH} mm · stepover {Math.min(woc, d).toFixed(2)} mm · {len.toFixed(0)} mm path/layer</text>
      <text x={8} y={Y(W) + 18} className="bd-svg-small">{layers} layer{layers > 1 ? 's' : ''} × {Math.min(doc, POCKET_DEPTH).toFixed(2)} mm at {feed.toFixed(0)} mm/min → {(layers * layerMin).toFixed(1)} min cutting</text>
      <text x={8} y={Y(W) + 32} className="bd-svg-small">shown {speedup.toFixed(0)}× faster · first pass is a full slot: ramp in, keep it shallow</text>
    </svg>
  );
}

export function FeedsCalc() {
  const mats = useCompute<MachMaterial[]>('build.machining_materials', {});
  const [i, setI] = useState({ material: 'al6061', tool: 'carbide', diameter_mm: 6, flutes: 3, woc_mm: 1.5, doc_mm: 6, aggressiveness: 0.4, max_rpm: 24000, min_rpm: 1000, efficiency: 0.8, chip_thinning: true });
  const r = useCompute<FeedsOut>('build.feeds_speeds', i);
  const [playing, setPlaying] = usePlaying();
  const t = useTicker(playing);
  const o = r.data;
  const set = (p: Partial<typeof i>) => setI({ ...i, ...p });
  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Cutter and cut">
          <Select label="Material" value={i.material} onChange={(v) => set({ material: v })} options={(mats.data ?? []).map((m) => ({ value: m.id, label: m.name }))} />
          <Select label="Tool" value={i.tool} onChange={(v) => set({ tool: v })} options={[{ value: 'carbide', label: 'Carbide end mill' }, { value: 'hss', label: 'High-speed steel (HSS) end mill' }]} />
          <Slider label="Diameter D" unit="mm" value={i.diameter_mm} min={0.5} max={20} log onChange={(v) => set({ diameter_mm: v, woc_mm: Math.min(i.woc_mm, v), doc_mm: Math.min(i.doc_mm, 4 * v) })} format={(v) => v.toFixed(2)} />
          <Slider label="Flutes z" value={i.flutes} min={1} max={6} step={1} onChange={(v) => set({ flutes: Math.round(v) })} format={(v) => v.toFixed(0)} />
          <Slider label="Radial depth a_e (stepover)" unit="mm" value={i.woc_mm} min={0.05 * i.diameter_mm} max={i.diameter_mm} onChange={(v) => set({ woc_mm: v })} format={(v) => v.toFixed(2)} />
          <Slider label="Axial depth a_p" unit="mm" value={i.doc_mm} min={0.1} max={4 * i.diameter_mm} onChange={(v) => set({ doc_mm: v })} format={(v) => v.toFixed(2)} />
          <Slider label="Conservative → aggressive" value={i.aggressiveness} min={0} max={1} step={0.05} onChange={(v) => set({ aggressiveness: v })} format={(v) => `${(v * 100).toFixed(0)} %`} />
          <div className="grid grid-2" style={{ gap: 8 }}>
            <NumberField label="Max spindle" unit="RPM" value={i.max_rpm} step={500} onChange={(v) => set({ max_rpm: v })} />
            <NumberField label="Min spindle" unit="RPM" value={i.min_rpm} step={500} onChange={(v) => set({ min_rpm: v })} />
          </div>
          <div className="row" style={{ gap: 6, marginBottom: 8 }}>
            {[['Trim router', 30000], ['Hobby spindle', 24000], ['Benchtop mill', 10000], ['VMC', 12000]].map(([l, v]) => <button key={l} className="btn small" onClick={() => set({ max_rpm: Number(v) })}>{l} {Number(v) / 1000}k</button>)}
          </div>
          <Toggle label="Radial chip-thinning compensation" value={i.chip_thinning} onChange={(v) => set({ chip_thinning: v })} />
        </Card>
        <div className="stack">
          <ErrorBox error={r.error} />
          {o && (
            <>
              <div className="grid grid-4">
                <Stat label="Spindle speed" value={o.rpm.toFixed(0)} unit="RPM" tone={o.rpm_limited ? 'warn' : undefined} note={o.rpm_limited ? `ideal ${o.rpm_ideal.toFixed(0)} — spindle limited` : `${o.sfm_actual.toFixed(0)} SFM · ${o.vc_actual_m_min.toFixed(0)} m/min`} />
                <Stat label="Feed" value={o.feed_mm_min.toFixed(0)} unit="mm/min" note={`${o.feed_ipm.toFixed(1)} in/min`} />
                <Stat label="Chip load" value={o.chip_load_adjusted_mm.toFixed(4)} unit="mm/tooth" note={o.chip_thinning_factor > 1.001 ? `${o.chip_load_mm.toFixed(4)} × thinning ${o.chip_thinning_factor.toFixed(2)}` : 'no thinning at ≥ 50 % engagement'} />
                <Stat label="Cutting power" value={o.power_spindle_w.toFixed(0)} unit="W" note={`${o.power_range_w[0].toFixed(0)}–${o.power_range_w[1].toFixed(0)} W range · motor ≈ ${o.power_motor_w.toFixed(0)} W`} />
              </div>
              <Card title="The cutter" subtitle={`MRR ${o.mrr_cm3_min.toFixed(2)} cm³/min (${o.mrr_in3_min.toFixed(3)} in³/min) · torque ${(o.torque_nm * 1000).toFixed(0)} mN·m`} actions={<PlayButton playing={playing} onChange={setPlaying} />}>
                <Cutter t={t} rpm={o.rpm} flutes={i.flutes} d={i.diameter_mm} woc={i.woc_mm} chip={o.chip_load_adjusted_mm} />
                {o.warnings.map((w) => <div key={w} className="small" style={{ marginTop: 6 }}><Chip tone="warn">! note</Chip> {w}</div>)}
              </Card>
              <Card title="Tool path: clearing a pocket" subtitle="Raster passes at the chosen stepover, then a finishing contour — cycle time from the computed feed" actions={<PlayButton playing={playing} onChange={setPlaying} />}>
                <ToolPath t={t} d={i.diameter_mm} woc={i.woc_mm} doc={i.doc_mm} feed={o.feed_mm_min} flutes={i.flutes} rpm={o.rpm} />
              </Card>
            </>
          )}
        </div>
      </div>
      {o && (
        <div className="grid grid-2">
          <Card title="Step by step" subtitle={`${o.material.name}: ${o.sfm_range[0]}–${o.sfm_range[1]} SFM, ${o.chip_range_mm[0].toFixed(3)}–${o.chip_range_mm[1].toFixed(3)} mm/tooth at Ø${i.diameter_mm.toFixed(1)} mm (typical)`}>
            <ol className="bd-steps">{o.steps.map((s) => <li key={s} className="mono small">{s}</li>)}</ol>
            <div className="small muted">Flutes: {o.material.flutes}. Coolant: {o.material.coolant}. {o.material.notes}</div>
            <DownloadButton filename="feeds-and-speeds.md" content={`# Feeds and speeds — ${o.material.name}\n\nTypical starting point — tune on your machine.\n\n${o.steps.map((s) => `- ${s}`).join('\n')}\n\n${o.warnings.map((w) => `> ${w}`).join('\n')}\n`} label="feeds-and-speeds.md" small />
          </Card>
          <Explainer title="Feeds and speeds" refs={[{ label: "Machinery's Handbook (Industrial Press)" }, { label: 'Kalpakjian & Schmid, Manufacturing Engineering and Technology — specific cutting energy' }, { label: 'Sandvik Coromant / Harvey Tool milling guidance (chip thinning)' }]}>
            The tool maker specifies a <b>surface speed</b> (how fast the cutting edge moves through the material) and a <b>chip load</b> (how thick a chip each flute takes):
            <Formula>{'RPM = SFM × 12 / (π · D_in)        n = 1000 · Vc / (π · D_mm)\nfeed = RPM × z × f_z\nMRR = a_e × a_p × feed\nP = MRR × u        (u = specific cutting energy, W·s/mm³)\nRCTF = 1 / √(1 − (1 − 2·a_e/D)²)   for a_e < D/2'}</Formula>
            With a small stepover each flute only grazes the stock, so the real chip is thinner than f_z — rubbing instead of cutting. Raising the feed by the radial chip-thinning factor (RCTF) restores the chip. Hobby spindles often cannot reach the surface speed for small tools in aluminium: keep the chip load and accept the lower feed.
          </Explainer>
        </div>
      )}
    </div>
  );
}
