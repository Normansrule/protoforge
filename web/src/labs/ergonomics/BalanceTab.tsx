// Tab 2 — Weight & balance: where Iris's mass sits, how it loads the nose and
// the ears, nose-pad pressure and whether the glasses slip when you look down.
import { useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useElementWidth } from '../../components/useElementWidth';
import { csv, f1, Kv, OkChip, PlayButton, REFS, useClock, useErgo, usePlaying } from './shared';
import type { BalanceOut, ErgoConfig } from './types';

const G = 9.80665;

export function BalanceTab() {
  const { project, cfg, setCfg } = useErgo();
  const [shift, setShift] = useState(0);
  const bal = useCompute<BalanceOut>('ergonomics.balance', { project, battery_shift_mm: shift }, !!cfg);
  const b = bal.data;
  if (!cfg) return <div className="muted small">Loading the engine…</div>;
  const pmax = b ? Math.max(...b.upright.pad_pressure_kpa) : 0;
  return (
    <div className="stack">
      <ErrorBox error={bal.error} />
      {b && (
        <div className="grid grid-4 hf-stats">
          <Stat label="Wearable mass" value={f1(b.mass_g)} unit="g" note={`frame (CAD) + ${project.components.length} BOM lines`} />
          <Stat label="Load on the nose" value={f1(b.upright.nose_gf)} unit="gf" note={`${f1(b.upright.nose_share_pct, 0)} % of the weight · ergonomics.nose_load_g`} tone={b.upright.nose_gf <= 25 ? 'pass' : 'warn'} />
          <Stat label="Nose-pad pressure" value={f1(pmax)} unit="kPa" tone={pmax <= b.pressure_limit_kpa ? 'pass' : 'fail'} note={`limit ≈ ${f1(b.pressure_limit_kpa)} kPa (32 mmHg) · ${f1(b.pad_area_mm2, 0)} mm² pads`} />
          <Stat label="Right ear share" value={f1(b.upright.right_ear_share_pct, 0)} unit="%" note={`right side ${f1(b.right_g)} g · left ${f1(b.left_g)} g`} />
        </div>
      )}
      {b && <ForceCard b={b} cfg={cfg} />}
      {b && (
        <div className="grid grid-2">
          <Card title="What if the battery moved back?" subtitle="Slide the left-pod parts along the temple towards (or behind) the ear — a counterweight unloads the nose.">
            <Slider label="Battery shift towards the ear" value={shift} min={0} max={80} step={1} unit="mm" onChange={setShift} />
            <Plot
              height={220}
              xLabel="battery shift (mm)"
              yLabel="nose load (gf)"
              series={[{ name: 'nose load', x: b.shift_mm, y: b.nose_gf_vs_shift }]}
              vLines={[{ value: shift, label: `${f1(b.upright.nose_gf)} gf` }]}
              hLines={[{ value: 25, label: 'suggested 25 gf' }]}
            />
            <p className="small muted" style={{ margin: 0 }}>A what-if only: the CAD battery pod does not move. Behind-the-ear battery housings are a common smart-glasses layout for exactly this reason.</p>
          </Card>
          <Card title="Nose-pad pressure vs pad area" subtitle="Normal force per pad over its contact area, against the ≈ 4.3 kPa sustained-pressure limit.">
            <Plot
              height={220}
              xLog
              yLog
              xLabel="pad contact area per pad (mm²)"
              yLabel="pressure (kPa)"
              series={[{ name: 'pressure', x: b.pad_area_axis, y: b.pressure_vs_area }]}
              hLines={[{ value: b.pressure_limit_kpa, label: 'capillary limit', color: 'var(--bad)' }]}
              vLines={[
                { value: b.pad_area_cad_mm2, label: 'CAD pad' },
                { value: b.pad_area_for_limit_mm2, label: `${f1(b.pad_area_for_limit_mm2, 0)} mm² needed` },
              ]}
            />
            <Slider label="Pad area override (0 = CAD pad)" value={cfg.pad_area_mm2} min={0} max={300} step={5} unit="mm²" onChange={(v) => setCfg({ pad_area_mm2: v })} />
            <Slider label="Nose flank angle from vertical" value={cfg.nose_flank_deg} min={10} max={60} step={1} unit="°" onChange={(v) => setCfg({ nose_flank_deg: v })} hint="steeper flanks (small angle) need more normal force for the same vertical support" />
          </Card>
        </div>
      )}
      {b && <MassCard b={b} />}
      <div className="grid grid-3">
        <Explainer title="Four supports, one rigid frame" refs={[REFS.roark, REFS.bodyspace]}>
          The glasses rest on two nose pads and two ear roots: four supports, but a rigid body has only three equilibrium equations. Treating the soft tissue under each support as
          an equally stiff spring gives reactions that vary linearly with position — the bolt-group / pile-group distribution:
          <Formula>{'Rᵢ = W/4 + W(y_c − ȳ)(yᵢ − ȳ)/Σ(yⱼ − ȳ)² + W x_c xᵢ/Σxⱼ²   →   N_nose = W (y_c − y_ear)/(y_pad − y_ear)'}</Formula>
          The fore–aft split is the lever rule; the ears, 130 mm apart, take almost all of any left/right imbalance.
        </Explainer>
        <Explainer title="Nose-pad pressure" refs={[REFS.landis]}>
          A pad on a nose flank tilted β from the vertical needs N = V / sin β to carry a vertical load V (frictionless wedge, an upper bound). The contact pressure p = N / A is compared
          with the capillary blood pressure of skin, ≈ 32 mmHg = 4.3 kPa (Landis 1930): sustained pressure above it squeezes the capillaries shut, the classic cause of red marks and
          soreness. Larger, softer (silicone) pads spread the load.
        </Explainer>
        <Explainer title="Does it slip when you look down?" refs={[REFS.derler]}>
          Flexing the head by α tilts the weight: a component W sin α pushes the frame down the nose and friction must hold it.
          <Formula>{'μ (N_pads + V_ears + 2 F_clamp) ≥ W sin α   →   F_min = (W sin α / μ − N_pads − V_ears) / 2'}</Formula>
          That F_min (at {cfg.look_down_deg}° and μ = {cfg.friction_mu}) is the lower edge of the clamp comfort band. Skin friction depends strongly on moisture: dry skin on plastic
          sits near the low end of the published range, sweat raises it.
        </Explainer>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------- animated force diagram

function Arrow({ x, y, dx, dy, color, w = 2.4 }: { x: number; y: number; dx: number; dy: number; color: string; w?: number }) {
  const len = Math.hypot(dx, dy);
  if (len < 0.5) return null;
  const ux = dx / len, uy = dy / len;
  const hx = x + dx, hy = y + dy;
  const s = Math.min(8, len * 0.45);
  return (
    <g>
      <line x1={x} y1={y} x2={hx - ux * s * 0.6} y2={hy - uy * s * 0.6} stroke={color} strokeWidth={w} strokeLinecap="round" />
      <path d={`M${hx},${hy}L${hx - ux * s - uy * s * 0.55},${hy - uy * s + ux * s * 0.55}L${hx - ux * s + uy * s * 0.55},${hy - uy * s - ux * s * 0.55}Z`} fill={color} />
    </g>
  );
}

function ForceCard({ b, cfg }: { b: BalanceOut; cfg: ErgoConfig }) {
  const [playing, setPlaying] = usePlaying();
  const [manual, setManual] = useState(cfg.look_down_deg);
  const t = useClock(playing);
  const pitch = playing ? cfg.look_down_deg * 0.5 * (1 - Math.cos((t * 2 * Math.PI) / 5)) : manual;
  const a = (pitch * Math.PI) / 180;
  const W = (b.mass_g * G) / 1000;
  const c = Math.cos(a);
  const r = b.upright.reactions_gf.map((v) => v * c);
  const beta = (cfg.nose_flank_deg * Math.PI) / 180;
  const padN = (Math.max(0, r[0]) + Math.max(0, r[1])) * (G / 1000) / Math.sin(beta);
  const earV = Math.max(0, r[2] + r[3]) * (G / 1000);
  const slide = W * Math.sin(a);
  const cap = cfg.friction_mu * (padN + earV + 2 * b.clamp_p50_n);
  const holds = cap >= slide - 1e-12;
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const Wd = Math.max(300, width);
  const side = Wd >= 720;
  const sw = side ? Wd * 0.58 : Wd;
  const sh = side ? 400 : 340;
  // side view: engine y → svg x (forward = right), engine z → svg −y
  const yMin = -180, yMax = 40, zMax = 160, zMin = -150;
  const k = Math.min((sw - 20) / (yMax - yMin), (sh - 30) / (zMax - zMin));
  const X = (y: number) => (sw - (yMax - yMin) * k) / 2 + (y - yMin) * k;
  const Y = (z: number) => 24 + (zMax - z) * k;
  const pivot = [X(-95), Y(-140)];
  const fs = 2.6; // px per gf
  const pad = b.pads[0], ear = b.ears[0], com = b.com;
  const headPath = `M${X(-8)},${Y(95)} C${X(-40)},${Y(150)} ${X(-170)},${Y(140)} ${X(-172)},${Y(40)} C${X(-176)},${Y(-40)} ${X(-150)},${Y(-90)} ${X(-110)},${Y(-120)} L${X(-60)},${Y(-125)} C${X(-30)},${Y(-120)} ${X(-12)},${Y(-110)} ${X(-10)},${Y(-95)} L${X(-14)},${Y(-72)} L${X(-4)},${Y(-60)} L${X(-12)},${Y(-45)} L${X(10)},${Y(-35)} L${X(-8)},${Y(8)} L${X(-12)},${Y(25)} C${X(-10)},${Y(55)} ${X(-5)},${Y(80)} ${X(-8)},${Y(95)} Z`;
  const fw = Wd - sw - 16;
  const fh = side ? 400 : 320;
  const front = (
    <svg width={side ? fw : Wd} height={fh} className="hf-svg" role="img" aria-label="Front view: load on each nose pad and ear">
      {(() => {
        const w0 = side ? fw : Wd;
        const sc = Math.min(w0 / 190, (fh - 50) / 250);
        const cx = (x: number) => w0 / 2 + x * sc;
        const cy = (z: number) => fh * 0.55 - z * sc;
        return (
          <g>
            <ellipse cx={cx(0)} cy={cy(10)} rx={74 * sc} ry={100 * sc} className="hf-head" />
            <line x1={cx(-70)} x2={cx(70)} y1={cy(ear[2])} y2={cy(ear[2])} className="hf-frame" />
            <rect x={cx(-60)} y={cy(20)} width={52 * sc} height={40 * sc} rx={10} className="hf-frame-fill" />
            <rect x={cx(8)} y={cy(20)} width={52 * sc} height={40 * sc} rx={10} className="hf-frame-fill" />
            {[0, 1].map((i) => (
              <g key={`p${i}`}>
                <Arrow x={cx(b.pads[i][0])} y={cy(pad[2]) + Math.max(0, r[i]) * fs * 0.8 + 6} dx={0} dy={-Math.max(0, r[i]) * fs * 0.8} color="var(--series-1)" />
                <text x={cx(b.pads[i][0]) + (i === 0 ? 6 : -6)} y={cy(pad[2]) + 34} textAnchor={i === 0 ? 'start' : 'end'} className="hf-lbl">{f1(r[i])} gf</text>
              </g>
            ))}
            {[2, 3].map((i) => (
              <g key={`e${i}`}>
                <Arrow x={cx(b.ears[i - 2][0])} y={cy(ear[2]) + Math.max(0, r[i]) * fs * 0.8 + 6} dx={0} dy={-Math.max(0, r[i]) * fs * 0.8} color="var(--series-3)" />
                <text x={cx(b.ears[i - 2][0])} y={cy(ear[2]) + Math.max(0, r[i]) * fs * 0.8 + 20} textAnchor="middle" className="hf-lbl">{f1(r[i])} gf</text>
              </g>
            ))}
            <circle cx={cx(com[0])} cy={cy(com[2])} r={5} className="hf-com" />
            <Arrow x={cx(com[0])} y={cy(com[2])} dx={0} dy={b.mass_g * fs * 0.8 * 0.5} color="var(--series-2)" />
            <text x={cx(com[0]) + 8} y={cy(com[2]) - 6} className="hf-lbl">CoM x = {f1(com[0])} mm</text>
            <text x={6} y={16} className="hf-lbl">← left (battery)</text>
            <text x={w0 - 6} y={32} textAnchor="end" className="hf-lbl">right (electronics, display) →</text>
          </g>
        );
      })()}
    </svg>
  );
  return (
    <Card
      title="Forces on the head"
      subtitle="Side view (left) rotates with the head as the wearer looks down; arrow lengths are proportional to load. Front view shows the left/right split."
      actions={<PlayButton playing={playing} onChange={setPlaying} />}
    >
      <div className="row" style={{ marginBottom: 6 }}>
        {!playing && (
          <div style={{ flex: '1 1 220px' }}>
            <Slider label="Head pitch (looking down)" value={manual} min={0} max={80} step={1} unit="°" onChange={setManual} />
          </div>
        )}
        <OkChip ok={holds} yes={`holds at ${f1(pitch, 0)}°`} no={`slips at ${f1(pitch, 0)}°`} />
        <span className="small muted">friction capacity {f1(cap, 3)} N vs {f1(slide, 3)} N down the nose (median-head clamp {f1(b.clamp_p50_n, 2)} N per side, μ = {cfg.friction_mu})</span>
      </div>
      <div ref={ref} className="hf-force" style={{ flexDirection: side ? 'row' : 'column' }}>
        <svg width={sw} height={sh} className="hf-svg" role="img" aria-label="Side view of the head with load arrows">
          <g transform={`rotate(${pitch} ${pivot[0]} ${pivot[1]})`}>
            <path d={headPath} className="hf-head" />
            <ellipse cx={X(-104)} cy={Y(-15)} rx={12 * k} ry={30 * k} className="hf-ear" />
            {/* temple and front */}
            <polyline points={`${X(0)},${Y(0)} ${X(-8)},${Y(ear[2])} ${X(ear[1])},${Y(ear[2])} ${X(ear[1] - 28)},${Y(ear[2] - 22)}`} className="hf-frame" />
            <rect x={X(b.frame.geom.pod_rear_y)} y={Y(ear[2] + b.frame.geom.pod_h_mm / 2)} width={(b.frame.geom.pod_front_y - b.frame.geom.pod_rear_y) * k} height={b.frame.geom.pod_h_mm * k} rx={3} className="hf-frame-fill" />
            <line x1={X(0)} y1={Y(24)} x2={X(0)} y2={Y(-24)} className="hf-frame" />
            {/* reactions in the head frame */}
            <Arrow x={X(pad[1])} y={Y(pad[2]) + (r[0] + r[1]) * fs + 8} dx={0} dy={-(r[0] + r[1]) * fs} color="var(--series-1)" />
            <text x={X(pad[1]) + 6} y={Y(pad[2]) + (r[0] + r[1]) * fs + 8} className="hf-lbl">nose {f1(r[0] + r[1])} gf</text>
            <Arrow x={X(ear[1])} y={Y(ear[2]) + (r[2] + r[3]) * fs + 8} dx={0} dy={-(r[2] + r[3]) * fs} color="var(--series-3)" />
            <text x={X(ear[1]) + 6} y={Y(ear[2]) + (r[2] + r[3]) * fs + 8} className="hf-lbl">ears {f1(r[2] + r[3])} gf</text>
            {slide > 1e-4 && <Arrow x={X(pad[1])} y={Y(pad[2]) - 10} dx={(slide / G) * 1000 * fs} dy={0} color="var(--series-4)" />}
            <circle cx={X(com[1])} cy={Y(com[2])} r={5} className="hf-com" />
          </g>
          {/* gravity stays vertical in the world */}
          {(() => {
            const px = X(com[1]), py = Y(com[2]);
            const ang = (pitch * Math.PI) / 180;
            const dx = px - pivot[0], dy = py - pivot[1];
            const wx = pivot[0] + dx * Math.cos(ang) - dy * Math.sin(ang);
            const wy = pivot[1] + dx * Math.sin(ang) + dy * Math.cos(ang);
            return (
              <g>
                <Arrow x={wx} y={wy} dx={0} dy={b.mass_g * fs * 0.8} color="var(--series-2)" />
                <text x={wx + 8} y={wy + b.mass_g * fs * 0.8} className="hf-lbl">W = {f1(b.mass_g)} gf</text>
              </g>
            );
          })()}
          <text x={10} y={16} className="hf-lbl">pitch {f1(pitch, 0)}° · CoM {f1(-com[1], 0)} mm behind the front</text>
        </svg>
        {front}
      </div>
      <div className="hf-legend">
        <span><i style={{ background: 'var(--series-2)' }} /> weight</span>
        <span><i style={{ background: 'var(--series-1)' }} /> nose pads</span>
        <span><i style={{ background: 'var(--series-3)' }} /> ears</span>
        <span><i style={{ background: 'var(--series-4)' }} /> slide force down the nose</span>
      </div>
      <div className="grid grid-2" style={{ marginTop: 8 }}>
        <Plot
          height={200}
          xLabel="head pitch (deg)"
          yLabel="friction needed μ"
          series={[
            { name: 'no clamp', x: b.pitch_axis, y: b.mu_required_vs_pitch },
            { name: 'median-head clamp', x: b.pitch_axis, y: b.mu_required_with_clamp },
          ]}
          hLines={[{ value: cfg.friction_mu, label: `μ = ${cfg.friction_mu}` }]}
          vLines={[{ value: pitch, label: '' }]}
          yDomain={[0, 1.5]}
        />
        <Kv
          rows={[
            ['Pad normal force (each)', `${f1(b.upright.pad_normal_n[0], 3)} / ${f1(b.upright.pad_normal_n[1], 3)} N`],
            ['Pad pressure R / L', `${f1(b.upright.pad_pressure_kpa[0])} / ${f1(b.upright.pad_pressure_kpa[1])} kPa`],
            ['Ear loads R / L', `${f1(b.upright.reactions_gf[2])} / ${f1(b.upright.reactions_gf[3])} gf`],
            ['Centre of mass (x, y, z)', `${f1(com[0])}, ${f1(com[1])}, ${f1(com[2])} mm`],
            ['Clamp band from this model', `${f1(b.band.min_n, 2)}–${f1(b.band.max_n, 2)} N per side`],
          ]}
        />
      </div>
      {b.notes.map((n) => <div key={n} className="hf-note">{n}</div>)}
    </Card>
  );
}

// ---------------------------------------------------------------- mass table

function MassCard({ b }: { b: BalanceOut }) {
  const rows = [...b.items].sort((x, y) => y.mass_g - x.mass_g);
  const max = rows[0]?.mass_g ?? 1;
  return (
    <Card
      title="Mass breakdown"
      subtitle="Frame parts from the CAD volumes × material density; components at their project locations."
      actions={<DownloadButton small filename="iris-mass-properties.csv" label="CSV" content={() => csv(['item', 'kind', 'location', 'mass_g', 'x_mm', 'y_mm', 'z_mm'], rows.map((i) => [i.name, i.kind, i.location, i.mass_g, ...i.pos]))} />}
    >
      <div className="hf-mass">
        {rows.filter((i) => i.mass_g >= 0.05).map((i) => (
          <div className="hf-bar" key={i.name + i.location}>
            <span className="small" title={i.location}>{i.name}</span>
            <div className="hf-bar-track"><div style={{ width: `${(100 * i.mass_g) / max}%`, background: i.kind === 'frame' ? 'var(--series-1)' : 'var(--series-4)' }} /></div>
            <span className="small tnum">{f1(i.mass_g, 2)} g</span>
          </div>
        ))}
      </div>
      <div className="small muted">Blue: frame parts · amber: components. Items under 0.05 g are in the CSV.</div>
    </Card>
  );
}
