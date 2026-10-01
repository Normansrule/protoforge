// Cams: displacement / velocity / acceleration / jerk (SVAJ) for cycloidal,
// 3-4-5 polynomial and harmonic motion; a translating roller follower's
// cam profile with pressure-angle and radius-of-curvature checks; animated
// cam and follower; DXF/SVG profile and CSV exports.
import { useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { tryCall } from '../../engine/engine';
import type { CamIn, CamOut, CamSegment, ExportOut } from './types';
import { DEG, Kv, Notes, PlayBar, csvFrom, fmt, useClock } from './util';

const PRESETS: { id: string; label: string; segs: CamSegment[] }[] = [
  {
    id: 'rdfd',
    label: 'Rise–dwell–fall–dwell (cycloidal + 3-4-5)',
    segs: [
      { kind: 'rise', law: 'cycloidal', lift_mm: 20, beta_deg: 100 },
      { kind: 'dwell', law: 'cycloidal', lift_mm: 0, beta_deg: 80 },
      { kind: 'fall', law: 'poly345', lift_mm: 20, beta_deg: 100 },
      { kind: 'dwell', law: 'cycloidal', lift_mm: 0, beta_deg: 80 },
    ],
  },
  {
    id: 'harm',
    label: 'Harmonic with dwells (breaks the fundamental law)',
    segs: [
      { kind: 'rise', law: 'harmonic', lift_mm: 20, beta_deg: 100 },
      { kind: 'dwell', law: 'harmonic', lift_mm: 0, beta_deg: 80 },
      { kind: 'fall', law: 'harmonic', lift_mm: 20, beta_deg: 100 },
      { kind: 'dwell', law: 'harmonic', lift_mm: 0, beta_deg: 80 },
    ],
  },
  {
    id: 'rf',
    label: 'Rise–fall, no dwell (harmonic is fine here)',
    segs: [
      { kind: 'rise', law: 'harmonic', lift_mm: 15, beta_deg: 180 },
      { kind: 'fall', law: 'harmonic', lift_mm: 15, beta_deg: 180 },
    ],
  },
];

const LAW_LABEL: Record<string, string> = { cycloidal: 'Cycloidal', poly345: '3-4-5 polynomial', harmonic: 'Simple harmonic' };

export function CamsTab() {
  const [c, setC] = useState<CamIn>({ segments: PRESETS[0].segs, base_r_mm: 40, roller_r_mm: 8, offset_mm: 0, rpm: 60 });
  const set = (q: Partial<CamIn>) => setC({ ...c, ...q });
  const { data: o, error } = useCompute<CamOut>('mechanisms.cam', { ...c, samples: 720 });
  const clock = useClock(6, { loop: true });
  const total = c.segments.reduce((a, s) => a + s.beta_deg, 0);
  const setSeg = (i: number, q: Partial<CamSegment>) => set({ segments: c.segments.map((s, k) => (k === i ? { ...s, ...q } : s)) });
  const idx = o ? Math.min(o.theta_deg.length - 1, Math.floor((clock.t / 6) * o.theta_deg.length)) : 0;
  const th = o ? o.theta_deg[idx] : 0;
  const vlines = o ? [...o.boundaries_deg.map((b) => ({ value: b, color: 'var(--grid)' })), { value: th, label: `${fmt(th, 0)}°` }] : [];
  const csv = () =>
    o ? csvFrom(['theta_deg', 's_mm', 'v_mm_per_rad', 'a_mm_per_rad2', 'j_mm_per_rad3', 'v_mm_s', 'a_mm_s2', 'j_mm_s3', 'pressure_deg', 'rho_pitch_mm', 'rho_cam_mm', 'cam_x_mm', 'cam_y_mm'], [o.theta_deg, o.s, o.v, o.a, o.j, o.v_t, o.a_t, o.j_t, o.pressure_deg, o.rho_pitch, o.rho_cam, o.cam_x, o.cam_y]) : '';
  const exp = (kind: 'svg' | 'dxf') => () => {
    const r = tryCall<ExportOut>('mechanisms.cam_export', { cam: { ...c, samples: 1440 }, bore_mm: Math.round(c.base_r_mm * 0.3) });
    if (!r.data) return r.error ?? 'export failed';
    return kind === 'svg' ? new Blob([r.data.svg], { type: 'image/svg+xml' }) : r.data.dxf;
  };
  const w = (c.rpm * 2 * Math.PI) / 60;
  return (
    <div className="mx-split">
      <div className="stack" style={{ minWidth: 0 }}>
        <Card
          title="Disk cam with a translating roller follower"
          subtitle="The cam turns counter-clockwise; the follower rides in a vertical guide"
          actions={
            o && (
              <>
                <DownloadButton small filename="cam-svaj.csv" label="SVAJ CSV" content={csv} />
                <DownloadButton small filename="cam-profile.dxf" label="profile DXF" content={exp('dxf')} />
                <DownloadButton small filename="cam-profile.svg" label="SVG" content={exp('svg')} />
              </>
            )
          }
        >
          <ErrorBox error={error} />
          {o && (
            <div className="grid grid-2">
              <div>
                <CamView o={o} c={c} i={idx} />
                <PlayBar clock={clock} label={() => `θ ${fmt(th, 0)}°`} />
              </div>
              <div className="stack" style={{ gap: 8 }}>
                <Plot series={[{ name: 's', x: o.theta_deg, y: o.s }]} yLabel="s, mm" vLines={vlines} height={120} />
                <Plot series={[{ name: 'v', x: o.theta_deg, y: o.v_t.map((v) => v / 1000) }]} yLabel="v, m/s" vLines={vlines} height={120} />
                <Plot series={[{ name: 'a', x: o.theta_deg, y: o.a_t.map((v) => v / 1000) }]} yLabel="a, m/s²" vLines={vlines} height={120} />
                <Plot series={[{ name: 'j', x: o.theta_deg, y: o.j_t.map((v) => v / 1000) }]} yLabel="j, m/s³" xLabel="cam angle θ, deg" vLines={vlines} height={130} />
              </div>
            </div>
          )}
          {o && (
            <div className="grid grid-4" style={{ marginTop: 12 }}>
              <Stat label="Max pressure angle" value={fmt(o.max_pressure_deg, 1)} unit="°" tone={o.max_pressure_deg <= 30 ? 'pass' : 'fail'} note={o.max_pressure_deg <= 30 ? '✓ ≤ 30°' : '✕ > 30° — jams'} />
              <Stat label="Min convex ρ (pitch)" value={fmt(o.min_rho_convex_mm, 1)} unit="mm" tone={o.undercut ? 'fail' : o.min_rho_convex_mm >= 2 * c.roller_r_mm ? 'pass' : 'warn'} note={o.undercut ? '✕ undercut' : `${fmt(o.min_rho_convex_mm / c.roller_r_mm, 1)} × R_r`} />
              <Stat label="Peak acceleration" value={fmt((o.max_a * w * w) / 1000, 2)} unit="m/s²" note={`at ${c.rpm} rpm`} />
              <Stat label="Fundamental law" value={o.accel_continuous ? 'obeyed' : 'broken'} tone={o.accel_continuous ? 'pass' : 'fail'} note={o.accel_continuous ? '✓ finite jerk' : '✕ acceleration jumps'} />
            </div>
          )}
          {o && <Notes notes={o.notes} />}
        </Card>
        <Card title="Motion program" subtitle="Rise, dwell and fall segments; the angles must add up to 360°">
          <Select label="Preset" value="" options={[{ value: '', label: 'Choose a preset…' }, ...PRESETS.map((p) => ({ value: p.id, label: p.label }))]} onChange={(id) => { const p = PRESETS.find((x) => x.id === id); if (p) set({ segments: p.segs }); }} />
          <div className="table-wrap mx-table-input">
            <table className="table">
              <thead><tr><th>Kind</th><th>Law</th><th>h, mm</th><th>β, °</th><th /></tr></thead>
              <tbody>
                {c.segments.map((s, i) => (
                  <tr key={i}>
                    <td>
                      <select value={s.kind} aria-label={`segment ${i + 1} kind`} onChange={(e) => setSeg(i, { kind: e.target.value as CamSegment['kind'] })}>
                        <option value="rise">rise</option>
                        <option value="dwell">dwell</option>
                        <option value="fall">fall</option>
                      </select>
                    </td>
                    <td>
                      <select value={s.law} aria-label={`segment ${i + 1} law`} disabled={s.kind === 'dwell'} onChange={(e) => setSeg(i, { law: e.target.value as CamSegment['law'] })}>
                        <option value="cycloidal">cycloidal</option>
                        <option value="poly345">3-4-5</option>
                        <option value="harmonic">harmonic</option>
                      </select>
                    </td>
                    <td><input type="number" value={s.lift_mm} step={0.5} disabled={s.kind === 'dwell'} aria-label={`segment ${i + 1} lift`} onChange={(e) => setSeg(i, { lift_mm: Number(e.target.value) || 0 })} /></td>
                    <td><input type="number" value={s.beta_deg} step={5} aria-label={`segment ${i + 1} angle`} onChange={(e) => setSeg(i, { beta_deg: Number(e.target.value) || 0 })} /></td>
                    <td><button className="btn small" aria-label={`remove segment ${i + 1}`} disabled={c.segments.length <= 1} onClick={() => set({ segments: c.segments.filter((_, k) => k !== i) })}>✕</button></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="row" style={{ marginTop: 6 }}>
            <button className="btn small" onClick={() => set({ segments: [...c.segments, { kind: 'dwell', law: 'cycloidal', lift_mm: 0, beta_deg: Math.max(0, 360 - total) || 30 }] })}>+ segment</button>
            <Chip tone={Math.abs(total - 360) < 1e-6 ? 'pass' : 'fail'}>{Math.abs(total - 360) < 1e-6 ? '✓' : '✕'} Σβ = {fmt(total, 1)}°</Chip>
          </div>
        </Card>
        {o && (
          <div className="grid grid-2">
            <Card title="Pressure angle" subtitle="φ = atan((v − ε)/(d + s)); keep |φ| ≤ 30°">
              <Plot series={[{ name: 'φ', x: o.theta_deg, y: o.pressure_deg }]} xLabel="θ, deg" yLabel="deg" hLines={[{ value: 30, label: '+30°' }, { value: -30, label: '−30°' }]} vLines={[{ value: th }]} height={200} />
            </Card>
            <Card title="Radius of curvature" subtitle="Pitch curve (convex > 0); must stay above the roller radius">
              <Plot
                series={[
                  { name: 'ρ pitch', x: o.theta_deg, y: o.rho_pitch.map((r) => Math.max(-4 * o.prime_r, Math.min(4 * o.prime_r, r))) },
                  { name: 'ρ cam', x: o.theta_deg, y: o.rho_cam.map((r) => Math.max(-4 * o.prime_r, Math.min(4 * o.prime_r, r))) },
                ]}
                xLabel="θ, deg"
                yLabel="mm (clipped ±4 R_p)"
                hLines={[{ value: c.roller_r_mm, label: 'R_r' }]}
                vLines={[{ value: th }]}
                legend
                height={200}
              />
            </Card>
          </div>
        )}
        {o && (
          <Card title="Peak acceleration: engine vs closed form" subtitle="Cycloidal 2πh/β², 3-4-5 5.7735 h/β², harmonic π²h/(2β²) (per radian²)">
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Segment</th><th>Law</th><th>β</th><th>h</th><th>peak |a| (sampled)</th><th>closed form</th></tr></thead>
                <tbody>
                  {o.segments.map((sg, i) => (
                    <tr key={i}>
                      <td>{i + 1}. {sg[0]}</td>
                      <td>{sg[0] === 'dwell' ? '—' : LAW_LABEL[sg[1]]}</td>
                      <td className="tnum">{fmt(sg[2], 0)}°</td>
                      <td className="tnum">{fmt(sg[3], 1)} mm</td>
                      <td className="tnum">{fmt(sg[4], 3)} mm/rad²</td>
                      <td className="tnum">{fmt(sg[5], 3)} mm/rad²</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
        )}
      </div>
      <div className="stack mx-side">
        <Card title="Geometry">
          <Slider label="Base circle R_b" value={c.base_r_mm} min={5} max={120} step={0.5} unit="mm" onChange={(base_r_mm) => set({ base_r_mm })} />
          <Slider label="Roller radius R_r" value={c.roller_r_mm} min={0.5} max={30} step={0.5} unit="mm" onChange={(roller_r_mm) => set({ roller_r_mm })} />
          <Slider label="Follower offset ε" value={c.offset_mm} min={-30} max={30} step={0.5} unit="mm" onChange={(offset_mm) => set({ offset_mm })} />
          <Slider label="Cam speed" value={c.rpm} min={1} max={3000} step={1} unit="rpm" log onChange={(rpm) => set({ rpm })} />
          {o && <Kv rows={[['Prime circle R_p', `${fmt(o.prime_r, 2)} mm`], ['d = √(R_p² − ε²)', `${fmt(o.d_mm, 2)} mm`], ['Max |v|, |a|, |j| per rad', `${fmt(o.max_v, 2)}, ${fmt(o.max_a, 1)}, ${fmt(o.max_j, 0)}`]]} />}
        </Card>
        <Explainer title="SVAJ and the fundamental law of cam design" refs={[{ label: 'R. L. Norton, "Design of Machinery", ch. 8' }, { label: 'R. L. Norton, "Cam Design and Manufacturing Handbook", 2nd ed., Industrial Press, 2009' }]}>
          <Formula>{`cycloidal: s = h[θ/β − sin(2πθ/β)/(2π)],  a_max = 2πh/β²
3-4-5:     s = h[10x³ − 15x⁴ + 6x⁵],        a_max = 5.7735 h/β²
harmonic:  s = (h/2)(1 − cos πx),           a_max = π²h/(2β²)
pressure:  φ = atan((v − ε)/(s + √(R_p² − ε²)))`}</Formula>
          <p className="small">The cam function must have continuous displacement, velocity and acceleration across the full 360° — finite jerk. Harmonic motion next to a dwell jumps in acceleration; cycloidal and 3-4-5 do not.</p>
        </Explainer>
      </div>
    </div>
  );
}

function CamView({ o, c, i }: { o: CamOut; c: CamIn; i: number }) {
  const th = o.theta_deg[i] * DEG;
  const cs = Math.cos(th);
  const sn = Math.sin(th);
  // follower frame = R(θ)·cam frame; SVG y is flipped
  const rot = (x: number, y: number) => [cs * x - sn * y, sn * x + cs * y];
  const prof = o.cam_x.map((x, k) => rot(x, o.cam_y[k]));
  const d = prof.map((p, k) => `${k ? 'L' : 'M'}${p[0].toFixed(2)},${(-p[1]).toFixed(2)}`).join('') + 'Z';
  const yRoll = o.d_mm + o.s[i];
  const maxS = Math.max(...o.s);
  const R = o.prime_r + maxS + c.roller_r_mm;
  const top = yRoll + c.roller_r_mm + 0.6 * R;
  const vb = [-R * 1.15, -(o.d_mm + maxS + c.roller_r_mm + 0.65 * R), 2.3 * R, R * 1.2 + o.d_mm + maxS + c.roller_r_mm + 0.65 * R];
  const u = vb[2] / 100;
  const ex = c.offset_mm;
  const phi = (o.pressure_deg[i] ?? 0) * DEG;
  const nlen = 0.5 * R;
  const mark = rot(0, o.prime_r * 0.6);
  return (
    <svg className="mx-canvas" viewBox={vb.join(' ')} role="img" aria-label="Animated cam and roller follower">
      <path d={d} className="mx-cam" strokeWidth={0.4 * u} />
      <circle cx={0} cy={0} r={c.base_r_mm} className="mx-ghost" strokeWidth={0.25 * u} />
      <line x1={0} y1={0} x2={mark[0]} y2={-mark[1]} className="mx-spoke" strokeWidth={0.6 * u} />
      <circle cx={0} cy={0} r={1.4 * u} className="mx-pin" />
      {/* guide */}
      <rect x={ex - c.roller_r_mm * 0.9} y={-top} width={c.roller_r_mm * 1.8} height={0.35 * R} className="mx-guide" strokeWidth={0.3 * u} />
      <line x1={ex} y1={-yRoll} x2={ex} y2={-top + 0.02 * R} className="mx-stem" strokeWidth={1.4 * u} />
      <circle cx={ex} cy={-yRoll} r={c.roller_r_mm} className="mx-roller" strokeWidth={0.4 * u} />
      <circle cx={ex} cy={-yRoll} r={0.8 * u} className="mx-pin" />
      {/* contact normal at the pressure angle */}
      <line x1={ex} y1={-yRoll} x2={ex - nlen * Math.sin(phi)} y2={-yRoll - nlen * Math.cos(phi)} className="mx-normal" strokeWidth={0.4 * u} />
      <line x1={ex} y1={-yRoll} x2={ex} y2={-yRoll - nlen} className="mx-ghost" strokeWidth={0.3 * u} />
      <text x={ex + 2 * u} y={-yRoll - nlen * 0.7} fontSize={3.4 * u} className="mx-svgtext">φ {fmt(o.pressure_deg[i], 1)}°</text>
      <text x={-R * 1.1} y={vb[1] + vb[3] - 2 * u} fontSize={3.4 * u} className="mx-svgtext">s = {fmt(o.s[i], 2)} mm</text>
    </svg>
  );
}
