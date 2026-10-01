// Gears: rack-generated involute spur gears with profile shift and the true
// trochoidal root fillet, animated meshing along the line of action,
// contact ratio, undercut and interference checks, Lewis + Barth bending,
// SVG/DXF export, and gear-train and planetary ratio designers.
import { useMemo, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, NumberField, Select, Slider, Stat, Toggle } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { tryCall } from '../../engine/engine';
import type { ExportOut, GearProfile, PairOut, PlanetOut, TrainOut } from './types';
import { DEG, Kv, Notes, PlayBar, fmt, sfTone, useClock } from './util';

const inv = (a: number) => Math.tan(a) - a;

/** Closed SVG path of a whole gear (y flipped), tooth 0 centred at `rot`. */
export function gearPath(g: GearProfile, rot: number, cx: number, cy: number): string {
  const z = g.teeth;
  const pitch = (2 * Math.PI) / z;
  const n = g.flank_r.length;
  const psiTop = g.flank_psi[n - 1];
  const psiRoot = g.flank_psi[0];
  const P = (r: number, a: number) => `${(cx + r * Math.cos(a)).toFixed(3)},${(cy - r * Math.sin(a)).toFixed(3)}`;
  let d = '';
  for (let k = 0; k < z; k++) {
    const c = rot + k * pitch;
    for (let j = 0; j < n; j++) d += (d ? 'L' : 'M') + P(g.flank_r[j], c - g.flank_psi[j]);
    if (psiTop > 0) d += `A${g.flank_r[n - 1].toFixed(3)},${g.flank_r[n - 1].toFixed(3)} 0 0 0 ${P(g.flank_r[n - 1], c + psiTop)}`;
    for (let j = n - 2; j >= 0; j--) d += 'L' + P(g.flank_r[j], c + g.flank_psi[j]);
    const nextStart = c + pitch - psiRoot;
    d += `A${g.root_r.toFixed(3)},${g.root_r.toFixed(3)} 0 0 0 ${P(g.root_r, nextStart)}`;
  }
  return d + 'Z';
}

interface PairState {
  module_mm: number; z1: number; z2: number; pressure_deg: number; x1: number; x2: number; root_radius: number;
  shorten_tips: boolean; backlash_mm: number; face_width_mm: number; torque_nm: number; rpm1: number; finish: string;
  material1: string; material2: string;
}

const MATS = ['POM', 'PA12', 'PETG', 'PLA', 'PA6-GF30', 'AL6061-T6', 'SS304'];

export function GearsTab() {
  const [s, setS] = useState<PairState>({ module_mm: 1, z1: 18, z2: 36, pressure_deg: 20, x1: 0, x2: 0, root_radius: 0.38, shorten_tips: true, backlash_mm: 0.05, face_width_mm: 8, torque_nm: 0.3, rpm1: 600, finish: 'hobbed', material1: 'POM', material2: 'POM' });
  const set = (q: Partial<PairState>) => setS({ ...s, ...q });
  const { data: o, error } = useCompute<PairOut>('mechanisms.gear_pair', s);
  return (
    <div className="stack">
      <div className="mx-split">
        <div className="stack" style={{ minWidth: 0 }}>
          <MeshCard s={s} o={o} error={error} />
          {o && <ToothCard o={o} />}
        </div>
        <div className="stack mx-side">
          <Card title="Gear pair">
            <Slider label="Module m" value={s.module_mm} min={0.3} max={5} step={0.05} unit="mm" onChange={(module_mm) => set({ module_mm })} />
            <Slider label="Pinion teeth z₁" value={s.z1} min={6} max={80} step={1} onChange={(z1) => set({ z1: Math.round(z1) })} />
            <Slider label="Gear teeth z₂" value={s.z2} min={6} max={200} step={1} onChange={(z2) => set({ z2: Math.round(z2) })} />
            <Select label="Pressure angle" value={String(s.pressure_deg)} options={[{ value: '14.5', label: '14.5°' }, { value: '20', label: '20° (ISO 53)' }, { value: '25', label: '25°' }]} onChange={(v) => set({ pressure_deg: Number(v) })} />
            <Slider label="Pinion profile shift x₁" value={s.x1} min={-0.5} max={1} step={0.01} onChange={(x1) => set({ x1 })} />
            <Slider label="Gear profile shift x₂" value={s.x2} min={-0.5} max={1} step={0.01} onChange={(x2) => set({ x2 })} />
            <Slider label="Rack tip radius ρ_fP / m" value={s.root_radius} min={0} max={0.45} step={0.01} onChange={(root_radius) => set({ root_radius })} />
            <Slider label="Backlash (circular)" value={s.backlash_mm} min={0} max={0.5} step={0.01} unit="mm" onChange={(backlash_mm) => set({ backlash_mm })} />
            <Toggle label="Shorten tips to restore clearance (shifted pairs)" value={s.shorten_tips} onChange={(shorten_tips) => set({ shorten_tips })} />
            <button className="btn small" onClick={() => o && set({ x1: Math.max(0, +o.g1.x_min.toFixed(2)) })}>Set x₁ to avoid undercut</button>
          </Card>
          <Card title="Load (Lewis + Barth)">
            <Slider label="Pinion torque" value={s.torque_nm} min={0.01} max={50} step={0.01} unit="N·m" log onChange={(torque_nm) => set({ torque_nm })} />
            <Slider label="Pinion speed" value={s.rpm1} min={1} max={10000} step={1} unit="rpm" log onChange={(rpm1) => set({ rpm1 })} />
            <Slider label="Face width b" value={s.face_width_mm} min={1} max={60} step={0.5} unit="mm" onChange={(face_width_mm) => set({ face_width_mm })} />
            <Select label="Tooth finish (velocity factor)" value={s.finish} options={[{ value: 'cut', label: 'Cut or milled — Barth (6.1 + V)/6.1' }, { value: 'hobbed', label: 'Hobbed or shaped' }, { value: 'shaved', label: 'Shaved or ground' }, { value: 'cast', label: 'Cast iron, cast profile' }]} onChange={(finish) => set({ finish })} />
            <Select label="Pinion material" value={s.material1} options={MATS.map((m) => ({ value: m, label: m }))} onChange={(material1) => set({ material1 })} />
            <Select label="Gear material" value={s.material2} options={MATS.map((m) => ({ value: m, label: m }))} onChange={(material2) => set({ material2 })} />
            {o && (
              <Kv
                rows={[
                  ['Pitch-line speed V', `${fmt(o.pitch_velocity_m_s, 2)} m/s`],
                  ['Tangential load W_t', `${fmt(o.tangential_load_n, 1)} N`],
                  ['Velocity factor K_v', fmt(o.kv, 3)],
                  ['Lewis Y (profile / table)', `${fmt(o.g1.lewis_y, 3)} / ${o.lewis_y_table1 != null ? fmt(o.lewis_y_table1, 3) : '—'}`],
                ]}
              />
            )}
          </Card>
          <Explainer
            title="Involute geometry"
            refs={[
              { label: 'F. L. Litvin & A. Fuentes, "Gear Geometry and Applied Theory", 2004' },
              { label: 'ISO 53 (basic rack); ISO 21771 (geometry)' },
              { label: 'Shigley ch. 13–14' },
            ]}
          >
            <Formula>{`r = m z/2,   r_b = r cos α,   p_b = π m cos α
ψ(ρ) = π/(2z) + 2x tan α/z + inv α − inv α_ρ
inv α_w = inv α + 2 tan α (x₁+x₂)/(z₁+z₂)
ε_α = [√(r_a1²−r_b1²) + √(r_a2²−r_b2²) − a_w sin α_w] / p_b
σ = K_v W_t / (b m Y),  K_v = (6.1 + V)/6.1`}</Formula>
          </Explainer>
        </div>
      </div>
      <div className="grid grid-2">
        <TrainCard />
        <PlanetCard />
      </div>
    </div>
  );
}

function exportGear(g: GearProfile, s: PairState, which: 1 | 2, fmtKind: 'svg' | 'dxf') {
  const r = tryCall<ExportOut>('mechanisms.gear_export', {
    gear: { module_mm: g.module_mm, teeth: g.teeth, pressure_deg: g.pressure_deg, shift: g.shift, root_radius: s.root_radius, thinning_mm: s.backlash_mm / 2, samples: 120 },
    bore_mm: Math.max(1, Math.round(g.root_r * 0.5)),
    title: which === 1 ? 'Pinion' : 'Gear',
  });
  if (!r.data) return r.error ?? 'export failed';
  return fmtKind === 'svg' ? new Blob([r.data.svg], { type: 'image/svg+xml' }) : r.data.dxf;
}

function MeshCard({ s, o, error }: { s: PairState; o: PairOut | null; error: string | null }) {
  const [zoom, setZoom] = useState(false);
  const clock = useClock(12, { loop: true });
  if (!o) return <Card title="Meshing"><ErrorBox error={error} /></Card>;
  const { g1, g2 } = o;
  const aw = o.center_mm;
  // six pinion teeth per loop: both gears look identical at the loop point, so it is seamless
  const th1 = (clock.t / 12) * ((2 * Math.PI * 6) / g1.teeth);
  const th2 = Math.PI + Math.PI / g2.teeth - (th1 * g1.teeth) / g2.teeth;
  const al = s.pressure_deg * DEG;
  const rb1 = g1.base_r;
  const pb = (2 * Math.PI * rb1) / g1.teeth;
  const psib = Math.PI / (2 * g1.teeth) + (2 * s.x1 * Math.tan(al)) / g1.teeth + inv(al) - s.backlash_mm / (4 * g1.pitch_r);
  const alw = o.working_pressure_deg * DEG;
  const dir = [Math.sin(alw), Math.cos(alw)];
  const sStart = Math.hypot(o.loa_start[0] - o.t1[0], o.loa_start[1] - o.t1[1]) * Math.sign((o.loa_start[0] - o.t1[0]) * dir[0] + (o.loa_start[1] - o.t1[1]) * dir[1] || 1);
  const sEnd = Math.hypot(o.loa_end[0] - o.t1[0], o.loa_end[1] - o.t1[1]);
  const contacts: [number, number][] = [];
  const s0 = rb1 * (th1 + psib + alw);
  for (let k = -3; k <= 3; k++) {
    const sk = (((s0 % pb) + pb) % pb) + k * pb;
    if (sk >= sStart - 1e-6 && sk <= sEnd + 1e-6) contacts.push([o.t1[0] + sk * dir[0], o.t1[1] + sk * dir[1]]);
  }
  const R1 = g1.tip_r;
  const R2 = g2.tip_r;
  const full = [-R1 * 1.08, -Math.max(R1, R2) * 1.08, aw + R2 + R1 * 0.08 + R2 * 0.08, 2 * Math.max(R1, R2) * 1.08];
  const pp = (aw * g1.teeth) / (g1.teeth + g2.teeth);
  const zr = Math.max(4 * s.module_mm, 0.35 * Math.min(R1, R2));
  const close = [pp - 1.6 * zr, -1.1 * zr, 3.2 * zr, 2.2 * zr];
  const vb = zoom ? close : full;
  const u = vb[2] / 100;
  return (
    <Card
      title="Meshing"
      subtitle="Contact points (dots) travel along the line of action between the tip circles; the tooth pair count averages the contact ratio"
      actions={
        <>
          <DownloadButton small filename={`pinion-z${g1.teeth}.svg`} label="pinion SVG" content={() => exportGear(g1, s, 1, 'svg')} />
          <DownloadButton small filename={`pinion-z${g1.teeth}.dxf`} label="DXF" content={() => exportGear(g1, s, 1, 'dxf')} />
          <DownloadButton small filename={`gear-z${g2.teeth}.svg`} label="gear SVG" content={() => exportGear(g2, s, 2, 'svg')} />
          <DownloadButton small filename={`gear-z${g2.teeth}.dxf`} label="DXF" content={() => exportGear(g2, s, 2, 'dxf')} />
        </>
      }
    >
      <ErrorBox error={error} />
      <svg className="mx-canvas" viewBox={vb.join(' ')} role="img" aria-label="Two involute gears meshing with the line of action">
        <circle cx={0} cy={0} r={g1.pitch_r} className="mx-ghost" strokeWidth={0.25 * u} />
        <circle cx={aw} cy={0} r={g2.pitch_r} className="mx-ghost" strokeWidth={0.25 * u} />
        <circle cx={0} cy={0} r={g1.base_r} className="mx-base" strokeWidth={0.25 * u} />
        <circle cx={aw} cy={0} r={g2.base_r} className="mx-base" strokeWidth={0.25 * u} />
        <path d={gearPath(g1, th1, 0, 0)} className="mx-gear g1" strokeWidth={0.3 * u} />
        <path d={gearPath(g2, th2, aw, 0)} className="mx-gear g2" strokeWidth={0.3 * u} />
        <line x1={o.t1[0]} y1={-o.t1[1]} x2={o.t2[0]} y2={-o.t2[1]} className="mx-loa" strokeWidth={0.3 * u} />
        <line x1={o.loa_start[0]} y1={-o.loa_start[1]} x2={o.loa_end[0]} y2={-o.loa_end[1]} className="mx-loa-active" strokeWidth={0.9 * u} />
        {contacts.map((c, i) => <circle key={i} cx={c[0]} cy={-c[1]} r={1.1 * u} className="mx-contact" />)}
        <line x1={0} y1={0} x2={g1.pitch_r * Math.cos(th1)} y2={-g1.pitch_r * Math.sin(th1)} className="mx-spoke" strokeWidth={0.5 * u} />
        <line x1={aw} y1={0} x2={aw + g2.pitch_r * Math.cos(th2 - Math.PI / g2.teeth)} y2={-g2.pitch_r * Math.sin(th2 - Math.PI / g2.teeth)} className="mx-spoke" strokeWidth={0.5 * u} />
        <circle cx={0} cy={0} r={0.9 * u} className="mx-pin" />
        <circle cx={aw} cy={0} r={0.9 * u} className="mx-pin" />
      </svg>
      <div className="row" style={{ marginTop: 6 }}>
        <div style={{ flex: 1, minWidth: 220 }}><PlayBar clock={clock} label={() => `${contacts.length} pair${contacts.length === 1 ? '' : 's'} in contact`} /></div>
        <Toggle label="Close-up of the mesh" value={zoom} onChange={setZoom} />
      </div>
      <div className="grid grid-4" style={{ marginTop: 12 }}>
        <Stat label="Ratio z₂/z₁" value={fmt(o.ratio, 3)} note={`centre distance ${fmt(o.center_mm, 3)} mm`} />
        <Stat label="Contact ratio ε_α" value={fmt(o.contact_ratio, 3)} tone={o.contact_ratio >= 1.4 ? 'pass' : o.contact_ratio >= 1.2 ? 'warn' : 'fail'} note={o.contact_ratio >= 1.2 ? '✓ ≥ 1.2' : '✕ < 1.2 — rough'} />
        <Stat label="Pinion bending σ" value={fmt(o.sigma1_mpa, 1)} unit="MPa" tone={sfTone(o.sf1, 1.5)} note={`SF ${fmt(o.sf1, 2)} vs ${fmt(o.allowable1_mpa, 0)} MPa`} />
        <Stat label="Gear bending σ" value={fmt(o.sigma2_mpa, 1)} unit="MPa" tone={sfTone(o.sf2, 1.5)} note={`SF ${fmt(o.sf2, 2)} vs ${fmt(o.allowable2_mpa, 0)} MPa`} />
      </div>
      <div className="row" style={{ marginTop: 10 }}>
        <Chip tone={g1.undercut ? 'warn' : 'pass'}>{g1.undercut ? '! pinion undercut' : '✓ pinion not undercut'}</Chip>
        <Chip tone={g2.undercut ? 'warn' : 'pass'}>{g2.undercut ? '! gear undercut' : '✓ gear not undercut'}</Chip>
        <Chip tone={o.interference ? 'fail' : 'pass'}>{o.interference ? '✕ interference' : '✓ no interference'}</Chip>
        <Chip>α_w {fmt(o.working_pressure_deg, 2)}°</Chip>
        {o.tip_shortening > 1e-4 && <Chip>tip shortening k = {fmt(o.tip_shortening, 3)}</Chip>}
      </div>
      <Notes notes={[...g1.notes.map((n) => `Pinion: ${n}`), ...g2.notes.map((n) => `Gear: ${n}`), ...o.notes]} />
      <div className="small muted">Allowables are the fatigue strengths in the Structures material table (fully reversed, about 10⁷ cycles — conservative for one-way tooth bending). Check your supplier's gear data.</div>
    </Card>
  );
}

function ToothCard({ o }: { o: PairOut }) {
  const [which, setWhich] = useState<'1' | '2'>('1');
  const g = which === '1' ? o.g1 : o.g2;
  const n = g.flank_r.length;
  const xs = g.flank_r.map((r, i) => r * Math.sin(g.flank_psi[i]));
  const ys = g.flank_r.map((r, i) => r * Math.cos(g.flank_psi[i]));
  const pitch = (2 * Math.PI) / g.teeth;
  const halfW = g.root_r * Math.sin(pitch * 0.75) + 0.5 * g.module_mm;
  const y0 = g.root_r * Math.cos(pitch * 0.75) - 0.6 * g.module_mm;
  const vb = [-halfW, -(g.tip_r + 0.5 * g.module_mm), 2 * halfW, g.tip_r + 0.5 * g.module_mm - y0];
  const u = vb[2] / 100;
  const pts = (a: number[], b: number[], sign: number, from: number, to: number) =>
    a.slice(from, to).map((x, i) => `${(sign * x).toFixed(4)},${(-b[from + i]).toFixed(4)}`).join(' ');
  const fi = Math.max(1, g.form_index);
  // Lewis parabola
  const t = g.lewis_thickness_mm;
  const l = g.lewis_height_mm;
  const ysec = Math.sqrt(Math.max(0, g.lewis_section_r ** 2 - (t / 2) ** 2));
  const hL = ysec + l;
  const par = Array.from({ length: 41 }, (_, i) => {
    const x = -t / 2 + (t * i) / 40;
    return `${x.toFixed(4)},${(-(hL - (l / ((t / 2) ** 2)) * x * x)).toFixed(4)}`;
  }).join(' ');
  const arc = (r: number, cls: string, label: string, k: number) => (
    <g key={label}>
      <path d={`M${(-r * Math.sin(pitch * 0.75)).toFixed(3)},${(-r * Math.cos(pitch * 0.75)).toFixed(3)} A${r},${r} 0 0 1 ${(r * Math.sin(pitch * 0.75)).toFixed(3)},${(-r * Math.cos(pitch * 0.75)).toFixed(3)}`} className={cls} strokeWidth={0.25 * u} strokeDasharray={`${1.2 * u} ${0.9 * u}`} fill="none" />
      <text x={halfW - 1.5 * u} y={-r - 0.6 * u - k * 0} fontSize={3 * u} textAnchor="end" className="mx-svgtext">{label}</text>
    </g>
  );
  return (
    <Card title="One tooth, up close" subtitle="True involute above the form circle, trochoidal fillet below it (generated by the rounded rack tip), Lewis parabola at the critical section">
      <div className="grid grid-2">
        <svg className="mx-diagram tall" viewBox={vb.join(' ')} role="img" aria-label="Tooth profile with involute and fillet">
          {arc(g.root_r, 'mx-ghost', 'root', 0)}
          {arc(g.base_r, 'mx-base', 'base', 1)}
          {arc(g.form_r, 'mx-form', 'form', 2)}
          {arc(g.pitch_r, 'mx-ghost', 'pitch', 3)}
          {arc(g.tip_r, 'mx-ghost', 'tip', 4)}
          <polyline points={pts(xs, ys, 1, fi - 1, n)} className="mx-invo" strokeWidth={0.8 * u} fill="none" />
          <polyline points={pts(xs, ys, -1, fi - 1, n)} className="mx-invo" strokeWidth={0.8 * u} fill="none" />
          <polyline points={pts(xs, ys, 1, 0, fi)} className="mx-fillet" strokeWidth={0.8 * u} fill="none" />
          <polyline points={pts(xs, ys, -1, 0, fi)} className="mx-fillet" strokeWidth={0.8 * u} fill="none" />
          {t > 0 && <polyline points={par} className="mx-lewis" strokeWidth={0.35 * u} fill="none" />}
          {t > 0 && <line x1={-t / 2} y1={-ysec} x2={t / 2} y2={-ysec} className="mx-lewis" strokeWidth={0.5 * u} />}
        </svg>
        <div>
          <Select label="Show" value={which} options={[{ value: '1', label: `Pinion (z = ${o.g1.teeth})` }, { value: '2', label: `Gear (z = ${o.g2.teeth})` }]} onChange={setWhich} />
          <div className="row small" style={{ marginBottom: 8 }}>
            <span><span className="mx-swatch invo" /> involute</span>
            <span><span className="mx-swatch fillet" /> fillet (trochoid)</span>
            <span><span className="mx-swatch lewis" /> Lewis parabola</span>
          </div>
          <Kv
            rows={[
              ['Pitch / base radius', `${fmt(g.pitch_r, 3)} / ${fmt(g.base_r, 3)} mm`],
              ['Tip / root radius', `${fmt(g.tip_r, 3)} / ${fmt(g.root_r, 3)} mm`],
              ['Form radius (involute starts)', `${fmt(g.form_r, 3)} mm`],
              ['Thickness at pitch / tip', `${fmt(g.pitch_thickness_mm, 3)} / ${fmt(g.tip_thickness_mm, 3)} mm`],
              ['Base pitch p_b', `${fmt(g.base_pitch_mm, 3)} mm`],
              ['z_min (no undercut) / x_min', `${fmt(g.z_min, 1)} / ${fmt(g.x_min, 3)}`],
              ['Lewis Y (from this profile)', fmt(g.lewis_y, 3)],
            ]}
          />
          <p className="small muted" style={{ marginTop: 8 }}>
            {g.undercut ? 'Undercut: the cutter tip has eaten into the flank below the base circle — note the fillet climbing above it.' : 'No undercut: the involute reaches down to the form circle and blends into the fillet.'}
          </p>
        </div>
      </div>
    </Card>
  );
}

// ------------------------------------------------------------------ trains

function TrainCard() {
  const [stages, setStages] = useState<[number, number][]>([[12, 36], [15, 45]]);
  const [target, setTarget] = useState(9);
  const { data: o } = useCompute<TrainOut>('mechanisms.gear_train', { stages, target_ratio: target, min_teeth: 12, max_teeth: 80 });
  return (
    <Card title="Gear-train designer" subtitle="Compound train: ratio = Π (driven / driver); suggestions use 12–80 teeth">
      <div className="table-wrap mx-table-input">
        <table className="table">
          <thead><tr><th>Stage</th><th>Driver</th><th>Driven</th><th>Ratio</th><th /></tr></thead>
          <tbody>
            {stages.map((st, i) => (
              <tr key={i}>
                <td>{i + 1}</td>
                <td><input type="number" min={6} value={st[0]} aria-label={`stage ${i + 1} driver teeth`} onChange={(e) => setStages(stages.map((x, k) => (k === i ? [Math.max(1, Number(e.target.value) || 1), x[1]] : x)))} /></td>
                <td><input type="number" min={6} value={st[1]} aria-label={`stage ${i + 1} driven teeth`} onChange={(e) => setStages(stages.map((x, k) => (k === i ? [x[0], Math.max(1, Number(e.target.value) || 1)] : x)))} /></td>
                <td className="tnum">{fmt(st[1] / st[0], 3)}</td>
                <td><button className="btn small" aria-label={`remove stage ${i + 1}`} onClick={() => setStages(stages.filter((_, k) => k !== i))} disabled={stages.length <= 1}>✕</button></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="row" style={{ marginTop: 8 }}>
        <button className="btn small" onClick={() => setStages([...stages, [12, 24]])} disabled={stages.length >= 6}>+ stage</button>
        {o && <Chip>total {fmt(o.ratio, 4)} : 1 · output {o.direction > 0 ? 'same direction' : 'reversed'}</Chip>}
      </div>
      <div style={{ marginTop: 12 }}>
        <NumberField label="Find a train for ratio" value={target} step={0.1} onChange={setTarget} />
        {o && o.suggestions.length > 0 && (
          <ul className="mx-sugg">
            {o.suggestions.map((sg, i) => (
              <li key={i}>
                <button className="btn small" onClick={() => setStages(sg.stages)}>use</button>{' '}
                <span className="mono small">{sg.stages.map((x) => `${x[0]}→${x[1]}`).join(' · ')}</span>{' '}
                <span className="muted small">= {fmt(sg.ratio, 4)} ({sg.error_pct >= 0 ? '+' : ''}{fmt(sg.error_pct, 3)} %)</span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </Card>
  );
}

function PlanetCard() {
  const [p, setP] = useState({ sun: 18, ring: 72, planets: 3, module_mm: 1, fixed: 'ring', target_ratio: 0 });
  const { data: o, error } = useCompute<PlanetOut>('mechanisms.planetary', p);
  const clock = useClock(10, { loop: true });
  const view = useMemo(() => {
    if (!o) return null;
    const R = o.ring_r * 1.12;
    return { R, u: (2 * R) / 100 };
  }, [o]);
  // kinematics for the held member: carrier angle drives the animation
  const zs = p.sun;
  const zr = p.ring;
  const wc = (2 * Math.PI * clock.t) / 10;
  let ws = 0, wr = 0, wcarr = wc;
  if (p.fixed === 'ring') { wr = 0; ws = wc * (1 + zr / zs); }
  else if (p.fixed === 'sun') { ws = 0; wr = wc * (1 + zs / zr); }
  else { wcarr = 0; ws = wc; wr = -wc * (zs / zr); }
  const zp = o ? o.planet : 1;
  // planet spin (absolute) from rolling on the sun: ω_p = ω_c − (ω_s − ω_c) z_s / z_p
  const wp = wcarr - ((ws - wcarr) * zs) / Math.max(1, zp);
  const ticks = (r: number, z: number, rot: number, cx: number, cy: number, out: boolean, key: string) =>
    Array.from({ length: Math.min(z, 120) }, (_, k) => {
      const a = rot + (2 * Math.PI * k) / z;
      const r2 = out ? r + (view?.u ?? 1) * 1.6 : r - (view?.u ?? 1) * 1.6;
      return <line key={`${key}${k}`} x1={cx + r * Math.cos(a)} y1={cy - r * Math.sin(a)} x2={cx + r2 * Math.cos(a)} y2={cy - r2 * Math.sin(a)} strokeWidth={(view?.u ?? 1) * 0.5} />;
    });
  return (
    <Card title="Planetary set" subtitle="Willis: (ω_s − ω_c)/(ω_r − ω_c) = −Z_r/Z_s. Pitch circles with tooth marks, animated.">
      <ErrorBox error={error} />
      {o && view && (
        <>
          <svg className="mx-diagram square" viewBox={`${-view.R} ${-view.R} ${2 * view.R} ${2 * view.R}`} role="img" aria-label="Animated planetary gear set">
            <g className="mx-ring">{ticks(o.ring_r, zr, wr, 0, 0, false, 'r')}<circle cx={0} cy={0} r={o.ring_r} fill="none" strokeWidth={view.u * 0.6} /></g>
            <g className="mx-sun">{ticks(o.sun_r, zs, ws, 0, 0, true, 's')}<circle cx={0} cy={0} r={o.sun_r} strokeWidth={view.u * 0.6} /></g>
            <circle cx={0} cy={0} r={o.carrier_r} className="mx-ghost" strokeWidth={view.u * 0.3} />
            {Array.from({ length: p.planets }, (_, k) => {
              const a = wcarr + (2 * Math.PI * k) / p.planets;
              const cx = o.carrier_r * Math.cos(a);
              const cy = -o.carrier_r * Math.sin(a);
              return (
                <g key={k} className="mx-planet">
                  <line x1={0} y1={0} x2={cx} y2={cy} className="mx-carrier" strokeWidth={view.u * 1.2} />
                  <circle cx={cx} cy={cy} r={o.planet_r} strokeWidth={view.u * 0.6} />
                  {ticks(o.planet_r, Math.max(1, Math.round(zp)), wp, cx, cy, true, `p${k}`)}
                </g>
              );
            })}
          </svg>
          <PlayBar clock={clock} label={() => `carrier ${fmt((wcarr / (2 * Math.PI)) * 360, 0)}°`} />
        </>
      )}
      <div className="grid grid-2" style={{ marginTop: 10 }}>
        <div>
          <Slider label="Sun teeth Z_s" value={p.sun} min={8} max={80} step={1} onChange={(sun) => setP({ ...p, sun: Math.round(sun) })} />
          <Slider label="Ring teeth Z_r" value={p.ring} min={30} max={200} step={1} onChange={(ring) => setP({ ...p, ring: Math.round(ring) })} />
          <Slider label="Planets" value={p.planets} min={1} max={8} step={1} onChange={(planets) => setP({ ...p, planets: Math.round(planets) })} />
          <Select label="Held member" value={p.fixed} options={[{ value: 'ring', label: 'Ring held (sun → carrier)' }, { value: 'sun', label: 'Sun held (ring → carrier)' }, { value: 'carrier', label: 'Carrier held (sun → ring)' }]} onChange={(fixed) => setP({ ...p, fixed })} />
        </div>
        <div>
          {o && (
            <>
              <Stat label="Ratio" value={fmt(o.ratio, 3)} note={o.ratio_label} />
              <div className="row" style={{ marginTop: 8 }}>
                <Chip tone={o.planet_integer ? 'pass' : 'fail'}>{o.planet_integer ? `✓ planet z = ${o.planet}` : '✕ Z_r − Z_s odd'}</Chip>
                <Chip tone={o.equal_spacing ? 'pass' : 'warn'}>{o.equal_spacing ? '✓ equal spacing' : '! unequal spacing'}</Chip>
                <Chip tone={o.adjacency_ok ? 'pass' : 'fail'}>{o.adjacency_ok ? '✓ planets clear' : '✕ planets collide'}</Chip>
              </div>
              <Notes notes={o.notes} />
              <NumberField label="Suggest Z_s/Z_r for ratio (ring held)" value={p.target_ratio} step={0.5} onChange={(target_ratio) => setP({ ...p, target_ratio })} />
              {o.suggestions.length > 0 && (
                <ul className="mx-sugg">
                  {o.suggestions.map((sg, i) => (
                    <li key={i}>
                      <button className="btn small" onClick={() => setP({ ...p, sun: sg[0], ring: sg[2], fixed: 'ring' })}>use</button>{' '}
                      <span className="mono small">Z_s {sg[0]} · Z_p {sg[1]} · Z_r {sg[2]}</span> <span className="muted small">= {fmt(1 + sg[2] / sg[0], 3)}</span>
                    </li>
                  ))}
                </ul>
              )}
            </>
          )}
        </div>
      </div>
      {o && (
        <Plot
          series={[{ name: 'ratio (ring held)', x: Array.from({ length: 40 }, (_, i) => 10 + i * 2), y: Array.from({ length: 40 }, (_, i) => 1 + p.ring / (10 + i * 2)) }]}
          xLabel="sun teeth Z_s"
          yLabel="1 + Z_r/Z_s"
          vLines={[{ value: p.sun }]}
          height={150}
        />
      )}
    </Card>
  );
}
