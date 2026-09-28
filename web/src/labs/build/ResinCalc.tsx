// Resin (MSLA) exposure from Jacobs' working curve: dose → cure depth, an
// animated cure front growing from the vat film during one exposure, and a
// staircase-test fit that recovers the resin's D_p and E_c.
import { useMemo, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat } from '../../components/ui';
import { Plot, type Series } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { PlayBar, useClock } from '../control/kit';
import type { ResinOut } from './types';

interface In { dp_mm: number; ec_mj_cm2: number; irradiance_mw_cm2: number; layer_mm: number; cure_ratio: number; bottom_multiple: number }

/** Side view: plate and previous layers above, film and LCD below; the cure front rises from the film. */
function CureFront({ o, i, t }: { o: ResinOut; i: In; t: number }) {
  const W = 360, H = 200;
  const film = 150;
  const s = 70 / Math.max(o.cure_mm, i.layer_mm * 1.2); // px per mm, fits the cure depth
  const dose = i.irradiance_mw_cm2 * Math.min(t, o.time_s);
  const cd = dose > o.ec_mj_cm2 ? o.dp_mm * Math.log(dose / o.ec_mj_cm2) : 0;
  const on = t < o.time_s;
  const layerTop = film - i.layer_mm * s;
  const tGel = o.ec_mj_cm2 / i.irradiance_mw_cm2;
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="bd-anim" role="img" aria-label="Cure front growing from the vat film during one layer exposure">
      {/* previous layers hanging from the plate */}
      <rect x={70} y={12} width={220} height={12} fill="var(--surface-3)" stroke="var(--border-strong)" />
      <text x={296} y={22} className="bd-svg-small">build plate</text>
      {Array.from({ length: 3 }, (_, k) => (
        <rect key={k} x={110} y={24 + k * ((layerTop - 24) / 3)} width={140} height={(layerTop - 24) / 3 - 1} fill="var(--series-5)" opacity={0.55 + 0.1 * k} />
      ))}
      <text x={8} y={(24 + layerTop) / 2 + 4} className="bd-svg-small">previous layers</text>
      {/* liquid resin gap = new layer */}
      <rect x={60} y={layerTop} width={240} height={film - layerTop} fill="var(--accent-soft)" />
      {/* cured part of the new layer */}
      <rect x={110} y={film - Math.min(cd, o.cure_mm * 1.3) * s} width={140} height={Math.min(cd, o.cure_mm * 1.3) * s} fill="var(--series-5)" opacity={0.9} />
      <line x1={100} x2={260} y1={film - o.cure_mm * s} y2={film - o.cure_mm * s} stroke="var(--warn)" strokeDasharray="4 3" />
      <text x={264} y={film - o.cure_mm * s + 4} className="bd-svg-small">target {o.cure_mm.toFixed(3)} mm</text>
      <line x1={60} x2={300} y1={layerTop} y2={layerTop} stroke="var(--text-2)" strokeDasharray="2 3" />
      <text x={8} y={(layerTop + film) / 2 + 4} className="bd-svg-small">layer {i.layer_mm.toFixed(3)}</text>
      {/* film + LCD + UV */}
      <line x1={50} x2={310} y1={film} y2={film} stroke="var(--text-2)" />
      <rect x={50} y={film + 4} width={260} height={10} fill="var(--surface-3)" />
      <rect x={110} y={film + 4} width={140} height={10} fill={on ? 'var(--series-7)' : 'var(--surface-3)'} />
      {on && Array.from({ length: 8 }, (_, k) => <line key={k} x1={116 + k * 18} x2={116 + k * 18} y1={film + 34} y2={film + 16} stroke="var(--series-7)" strokeWidth={2} opacity={0.35 + 0.5 * Math.abs(Math.sin(t * 9 + k))} />)}
      <text x={8} y={film + 13} className="bd-svg-small">LCD mask</text>
      <text x={8} y={H - 6} className="bd-svg-small">
        t = {Math.min(t, o.time_s).toFixed(2)} / {o.time_s.toFixed(2)} s · dose {dose.toFixed(1)} mJ/cm² · {dose < o.ec_mj_cm2 ? `no gel until E_c (t = ${tGel.toFixed(2)} s)` : `cured ${cd.toFixed(3)} mm`}
      </text>
    </svg>
  );
}

export function ResinCalc() {
  const [i, setI] = useState<In>({ dp_mm: 0.12, ec_mj_cm2: 6, irradiance_mw_cm2: 3, layer_mm: 0.05, cure_ratio: 1.5, bottom_multiple: 8 });
  const [pts, setPts] = useState<[number, number][]>([]);
  const set = (p: Partial<In>) => setI({ ...i, ...p });
  const r = useCompute<ResinOut>('build.resin_exposure', { ...i, points: pts });
  const o = r.data;
  const clock = useClock(o ? o.time_s * 1.35 : 5, { speed: 1, resetKey: o ? o.time_s.toFixed(3) : '' });
  const synth = () => {
    // synthetic staircase from the current D_p / E_c with ±4 % measurement scatter — replace with your own readings
    const tc = i.ec_mj_cm2 / i.irradiance_mw_cm2;
    setPts([1.5, 2.2, 3.2, 4.7, 7].map((m, k) => {
      const tt = +(tc * m).toFixed(2);
      const cd = i.dp_mm * Math.log((i.irradiance_mw_cm2 * tt) / i.ec_mj_cm2) * (1 + 0.04 * Math.sin(k * 2.3 + 1));
      return [tt, +cd.toFixed(3)];
    }));
  };
  const series = useMemo(() => {
    if (!o) return [];
    const s: Series[] = [{ name: 'working curve C_d = D_p·ln(E/E_c)', x: o.curve_e, y: o.curve_cd, color: 'var(--series-1)' }];
    if (pts.length) s.push({ name: 'your staircase', x: pts.map((p) => p[0] * i.irradiance_mw_cm2), y: pts.map((p) => p[1]), color: 'var(--series-4)', scatter: true });
    return s;
  }, [o, pts, i.irradiance_mw_cm2]);
  const md = () =>
    o
      ? `# Resin exposure — working curve\n\nTypical starting point — confirm with your printer maker's exposure calibration print.\n\nD_p = ${o.dp_mm.toFixed(4)} mm, E_c = ${o.ec_mj_cm2.toFixed(2)} mJ/cm²${o.fitted ? ` (fitted from ${o.fitted.n} staircase points, R² = ${o.fitted.r2.toFixed(4)})` : ' (entered)'}, irradiance ${i.irradiance_mw_cm2} mW/cm².\n\n${o.steps.map((s) => `- ${s}`).join('\n')}\n\n| Layer (mm) | Cure depth (mm) | Dose (mJ/cm²) | Exposure (s) |\n|---|---|---|---|\n${o.table.map((row) => `| ${row.layer_mm} | ${row.cure_mm.toFixed(4)} | ${row.exposure_mj_cm2.toFixed(2)} | ${row.time_s.toFixed(2)} |`).join('\n')}\n`
      : '';
  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Resin and printer" subtitle="Illustrative defaults — use your resin's data or a staircase test">
          <Slider label="Penetration depth D_p" unit="mm" value={i.dp_mm} min={0.03} max={0.4} step={0.005} onChange={(v) => set({ dp_mm: v })} format={(v) => v.toFixed(3)} hint={pts.length >= 2 ? 'replaced by the staircase fit' : undefined} />
          <Slider label="Critical exposure E_c" unit="mJ/cm²" value={i.ec_mj_cm2} min={1} max={40} step={0.5} onChange={(v) => set({ ec_mj_cm2: v })} format={(v) => v.toFixed(1)} />
          <Slider label="Irradiance at the film I" unit="mW/cm²" value={i.irradiance_mw_cm2} min={0.5} max={10} step={0.1} onChange={(v) => set({ irradiance_mw_cm2: v })} format={(v) => v.toFixed(1)} hint="UV meter, or the printer maker's figure" />
          <Slider label="Layer height" unit="mm" value={i.layer_mm} min={0.01} max={0.15} step={0.005} onChange={(v) => set({ layer_mm: v })} format={(v) => v.toFixed(3)} />
          <Slider label="Cure depth / layer (overcure)" value={i.cure_ratio} min={1} max={3} step={0.05} onChange={(v) => set({ cure_ratio: v })} format={(v) => `${v.toFixed(2)}×`} />
          <Slider label="Bottom-layer exposure multiple" value={i.bottom_multiple} min={1} max={15} step={0.5} onChange={(v) => set({ bottom_multiple: v })} format={(v) => `${v.toFixed(1)}×`} />
          <div className="bd-gtitle" style={{ marginTop: 10 }}>Staircase test (optional)</div>
          <div className="small muted" style={{ marginBottom: 6 }}>Cure a strip of steps on the film at increasing exposure times, peel it, measure each step with a micrometer. Two or more rows replace D_p and E_c with the fitted values.</div>
          {pts.length > 0 && (
            <table className="table bd-stair">
              <thead><tr><th>Exposure (s)</th><th>Thickness (mm)</th><th /></tr></thead>
              <tbody>
                {pts.map((p, k) => (
                  <tr key={k}>
                    <td><input type="number" className="bd-num" aria-label={`Exposure step ${k + 1} (s)`} value={p[0]} step={0.1} onChange={(e) => setPts(pts.map((q, j) => (j === k ? [Number(e.target.value), q[1]] : q)))} /></td>
                    <td><input type="number" className="bd-num" aria-label={`Thickness step ${k + 1} (mm)`} value={p[1]} step={0.005} onChange={(e) => setPts(pts.map((q, j) => (j === k ? [q[0], Number(e.target.value)] : q)))} /></td>
                    <td><button className="btn small" aria-label={`Remove row ${k + 1}`} onClick={() => setPts(pts.filter((_, j) => j !== k))}>✕</button></td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          <div className="row" style={{ gap: 6, marginTop: 6 }}>
            <button className="btn small" onClick={() => setPts([...pts, [pts.length ? +(pts[pts.length - 1][0] * 1.5).toFixed(2) : 3, pts.length ? +(pts[pts.length - 1][1] + 0.04).toFixed(3) : 0.08]])}>+ row</button>
            <button className="btn small" onClick={synth} title="Synthetic readings generated from the sliders with ±4 % scatter">Example (synthetic)</button>
            {pts.length > 0 && <button className="btn small" onClick={() => setPts([])}>Clear</button>}
          </div>
        </Card>
        <div className="stack">
          <ErrorBox error={r.error} />
          {o && (
            <>
              <div className="grid grid-4">
                <Stat label="Layer exposure" value={o.time_s.toFixed(2)} unit="s" note={`${o.exposure_mj_cm2.toFixed(1)} mJ/cm²`} />
                <Stat label="Cure depth" value={o.cure_mm.toFixed(3)} unit="mm" note={`${i.cure_ratio.toFixed(2)} × ${i.layer_mm.toFixed(3)} mm layer`} />
                <Stat label="Bottom layers" value={o.bottom_time_s.toFixed(1)} unit="s" note={`cure ${o.bottom_cure_mm.toFixed(3)} mm`} />
                <Stat label="Resin D_p" value={`${o.dp_mm.toFixed(3)} mm`} note={o.fitted ? `fitted: E_c ${o.ec_mj_cm2.toFixed(2)} mJ/cm² · R² ${o.fitted.r2.toFixed(3)}` : `entered: E_c ${o.ec_mj_cm2.toFixed(1)} mJ/cm²`} tone={o.fitted ? (o.fitted.r2 >= 0.95 ? 'pass' : 'warn') : undefined} />
              </div>
              <Card title="One layer, in real time" subtitle="Nothing gels until the dose reaches E_c; then the cure front rises logarithmically">
                <CureFront o={o} i={i} t={clock.t} />
                <PlayBar clock={clock} speeds={[0.25, 0.5, 1, 2]} label={(t) => `${Math.min(t, o.time_s).toFixed(2)} s`} />
                {o.warnings.map((w) => <div key={w} className="small" style={{ marginTop: 6 }}><Chip tone="warn">! note</Chip> {w}</div>)}
              </Card>
            </>
          )}
        </div>
      </div>
      {o && (
        <div className="grid grid-2">
          <Card title="Working curve" subtitle="Cure depth vs dose (log axis) — a straight line; slope = D_p, zero crossing = E_c">
            <Plot series={series} height={250} xLog xLabel="dose E (mJ/cm²)" yLabel="C_d (mm)" legend yZero vLines={[{ value: o.exposure_mj_cm2, label: 'layer', color: 'var(--series-2)' }, { value: o.exposure_mj_cm2 * i.bottom_multiple, label: 'bottom', color: 'var(--series-3)' }]} hLines={[{ value: o.cure_mm, label: 'target C_d', color: 'var(--warn)' }]} />
          </Card>
          <Card title="Exposure by layer height" subtitle={`Same resin, same ${i.cure_ratio.toFixed(2)}× overcure`} actions={<DownloadButton filename="resin-exposure.md" content={md} label="resin-exposure.md" small />}>
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th style={{ textAlign: 'right' }}>Layer (mm)</th><th style={{ textAlign: 'right' }}>Cure depth (mm)</th><th style={{ textAlign: 'right' }}>Dose (mJ/cm²)</th><th style={{ textAlign: 'right' }}>Exposure (s)</th></tr></thead>
                <tbody>
                  {o.table.map((row) => (
                    <tr key={row.layer_mm} style={Math.abs(row.layer_mm - i.layer_mm) < 1e-6 ? { background: 'var(--accent-soft)' } : undefined}>
                      <td className="tnum" style={{ textAlign: 'right' }}>{row.layer_mm.toFixed(3)}</td>
                      <td className="tnum" style={{ textAlign: 'right' }}>{row.cure_mm.toFixed(4)}</td>
                      <td className="tnum" style={{ textAlign: 'right' }}>{row.exposure_mj_cm2.toFixed(1)}</td>
                      <td className="tnum" style={{ textAlign: 'right' }}>{row.time_s.toFixed(2)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <ol className="bd-steps" style={{ marginTop: 8 }}>{o.steps.map((s) => <li key={s} className="mono small">{s}</li>)}</ol>
          </Card>
        </div>
      )}
      <Explainer title="Why exposure grows exponentially with layer height" refs={[{ label: 'P. F. Jacobs, Rapid Prototyping & Manufacturing: Fundamentals of Stereolithography (SME, 1992)' }, { label: 'I. Gibson, D. Rosen, B. Stucker, Additive Manufacturing Technologies (3rd ed., Springer, 2021) — vat photopolymerisation' }]}>
        Light is absorbed as it travels into the resin (Beer–Lambert), so the dose at depth z is E·e^(−z/D_p). The resin gels where that dose exceeds the critical exposure E_c:
        <Formula>{'E(z) = E · exp(−z / D_p)    gel where E(z) ≥ E_c\n⇒ C_d = D_p · ln(E / E_c)      E = I · t\nt = (E_c / I) · exp(C_d / D_p)'}</Formula>
        Doubling the layer height multiplies the exposure by e^(C_d/D_p), not by two. Each layer must cure a little deeper than it is thick (overcure) to bond to the one above; the bottom layers get several times the dose so the part grips the plate. Resin makers publish exposure settings per printer — start there, and use a calibration print to confirm dimensions, since light bleed also grows with dose.
      </Explainer>
    </div>
  );
}
