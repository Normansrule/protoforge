// Optics Pro · Image quality: spot diagrams across fields and wavelengths,
// ray fans, field curvature and distortion, colour, wavefront and MTF of the
// lens being edited in the Lens designer (reversed HUD model).
import { useMemo, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Stat } from '../../components/ui';
import { Plot, type Series } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { toCSV } from '../control/kit';
import { useLensInit } from './LensTab';
import { useLens } from './lensStore';
import type { LensAnalysis, SpotField } from './proTypes';
import { waveColor, waveName } from './proTypes';
import { fmt, PRO_REFS } from './proUtil';
import { REFS, Seg } from './util';

function SpotBox({ spot, waves, only, scale, pixel, airy, title }: { spot: SpotField; waves: number; only: number | null; scale: number; pixel: number; airy: number; title: string }) {
  const S = scale;
  const rms = only === null ? spot.rms_mm : spot.rms_per_wave_mm[only] ?? 0;
  const pts = only === null ? spot.points.flatMap((p, w) => p.map((q) => [q[0], q[1], w] as const)) : (spot.points[only] ?? []).map((q) => [q[0], q[1], only] as const);
  const r = S / 90;
  return (
    <figure className="op-spot">
      <svg viewBox={`${-S} ${-S} ${2 * S} ${2 * S}`} role="img" aria-label={`spot diagram ${title}`}>
        <rect x={-S} y={-S} width={2 * S} height={2 * S} className="op-spot-bg" />
        <line x1={-S} y1={0} x2={S} y2={0} className="op-spot-axis" strokeWidth={S / 200} />
        <line x1={0} y1={-S} x2={0} y2={S} className="op-spot-axis" strokeWidth={S / 200} />
        {pixel > 0 && <rect x={-pixel / 2} y={-pixel / 2} width={pixel} height={pixel} className="op-spot-pixel" strokeWidth={1.4} />}
        {airy > 0 && <circle r={airy} className="op-spot-airy" strokeWidth={1.2} />}
        {pts.map((p, k) => <circle key={k} cx={p[0]} cy={-p[1]} r={r} fill={waveColor(p[2], waves)} />)}
      </svg>
      <figcaption>
        <span>{title}</span>
        <span className="tnum">{fmt(rms * 1000, 1)} µm</span>
      </figcaption>
    </figure>
  );
}

export function QualityTab() {
  const project = useProject((s) => s.project);
  useLensInit();
  const { system, presetId, edited } = useLens();
  const an = useCompute<LensAnalysis>('optics.lens_analyze', { project, system, config: { rings: 6, fan_points: 41, rays_per_fan: 5 } }, !!system);
  const A = an.data;
  const [fieldSel, setFieldSel] = useState(0);
  const [perWave, setPerWave] = useState(false);
  const q = A?.quality;
  const nw = system?.wavelengths_nm.length ?? 3;
  const pw = system?.primary ?? 1;
  const scale = useMemo(() => {
    if (!q) return 0.02;
    const g = Math.max(...q.spots.map((s) => s.geo_radius_mm), 1e-4);
    return Math.max(g * 1.08, (A?.pixel_mm ?? 0) * 0.8);
  }, [q, A]);
  if (!system) return <Card title="Image quality">Loading…</Card>;
  if (!q || !A) return <Card title="Image quality">{an.error ? <ErrorBox error={an.error} /> : 'Tracing…'}</Card>;
  const px = q.paraxial[pw];
  const lam = system.wavelengths_nm[pw] * 1e-6;
  const airy = 1.22 * lam * px.working_fnum;
  const pixel = q.pixel_arcmin;
  const fi = Math.min(fieldSel, q.spots.length - 1);
  const fan = q.fans[fi];
  const fanSeries = (key: 'ey' | 'ex'): Series[] =>
    fan ? fan[key].map((y, w) => ({ name: waveName(system.wavelengths_nm[w]), x: fan.pupil, y: y.map((v, k) => (fan.ok[w][k] ? v * 1000 : NaN)).map((v) => (Number.isFinite(v) ? v : 0)), color: waveColor(w, nw), width: w === pw ? 2.2 : 1.5 })) : [];
  const curves = q.curves;
  const fcSeries: Series[] = curves
    ? curves.tangential.flatMap((t, w) => [
      { name: `T ${waveName(system.wavelengths_nm[w])}`, x: t, y: curves.field_deg, color: waveColor(w, nw), width: 2 },
      { name: `S ${waveName(system.wavelengths_nm[w])}`, x: curves.sagittal[w], y: curves.field_deg, color: waveColor(w, nw), dash: '5 4', width: 1.5 },
    ])
    : [];
  const mtfSeries: Series[] = [
    ...q.mtf.fields.flatMap((f, k) => [
      { name: `${f.field_deg.toFixed(1)}° T`, x: q.mtf.freq_lp_mm, y: f.tangential, color: `var(--series-${k + 1})`, width: 2 },
      { name: `${f.field_deg.toFixed(1)}° S`, x: q.mtf.freq_lp_mm, y: f.sagittal, color: `var(--series-${k + 1})`, dash: '5 4', width: 1.4 },
    ]),
    { name: 'diffraction limit', x: q.mtf.freq_lp_mm, y: q.mtf.diffraction, color: 'var(--muted)', dash: '2 3', width: 1.6 },
  ];
  const spotsCsv = () => {
    const rows = ['field_deg,wavelength_nm,x_mm,y_mm'];
    q.spots.forEach((s) => s.points.forEach((p, w) => p.forEach((pt) => rows.push(`${s.field_deg},${system.wavelengths_nm[w]},${pt[0]},${pt[1]}`))));
    return rows.join('\n') + '\n';
  };
  const mtfCsv = () => toCSV(['freq_lp_mm', 'diffraction', ...q.mtf.fields.flatMap((f) => [`T_${f.field_deg.toFixed(2)}deg`, `S_${f.field_deg.toFixed(2)}deg`])], [q.mtf.freq_lp_mm, q.mtf.diffraction, ...q.mtf.fields.flatMap((f) => [f.tangential, f.sagittal])]);
  const tone = (ok: boolean) => (ok ? 'pass' : 'fail');
  const title = edited ? 'your edited lens' : presetId;

  return (
    <div className="stack">
      <div className="op-stats">
        <Stat label="RMS spot (worst)" value={fmt(q.worst_rms_arcmin, 2)} unit="arcmin" tone={tone(q.worst_rms_arcmin <= pixel)} note={`${q.worst_rms_arcmin <= pixel ? '✓' : '✕'} ≤ 1 pixel (${pixel.toFixed(2)}′) · ${title}`} />
        <Stat label="MTF at Nyquist (worst)" value={fmt(q.worst_mtf_nyquist, 2)} tone={tone(q.worst_mtf_nyquist >= 0.3)} note={`${q.worst_mtf_nyquist >= 0.3 ? '✓' : '✕'} rule of thumb ≥ 0.3 at ${fmt(q.mtf.nyquist_lp_mm, 1)} cycles/mm`} />
        <Stat label="Lateral colour (worst)" value={fmt(q.worst_lateral_arcmin, 2)} unit="arcmin" tone={tone(q.worst_lateral_arcmin <= pixel)} note={`colour fringes of ${fmt(q.worst_lateral_arcmin / Math.max(pixel, 1e-9), 1)} pixels at the corner`} />
        <Stat label="Distortion (edge)" value={fmt(curves ? curves.distortion_pct[curves.distortion_pct.length - 1] : 0, 2)} unit="%" note="reversed model; the eye sees the opposite sign — pre-warp the frame buffer to cancel it" />
      </div>
      <ErrorBox error={an.error} />
      <Card
        title="Spot diagrams — where a point of light lands on the display"
        subtitle={`Each dot is one real ray from a ${system.pupil_diameter_mm} mm eye pupil. Square = one pixel (${fmt(A.pixel_mm * 1000, 1)} µm); circle = Airy disc radius 1.22λN = ${fmt(airy * 1000, 1)} µm. Common scale ±${fmt(scale * 1000, 0)} µm.`}
        actions={
          <div className="row" style={{ gap: 6 }}>
            <Seg value={perWave ? 'w' : 'p'} onChange={(v) => setPerWave(v === 'w')} options={[{ value: 'p', label: 'polychromatic' }, { value: 'w', label: 'per wavelength' }]} label="spot view" />
            <DownloadButton small filename="iris-spots.csv" content={spotsCsv} label="spots.csv" />
          </div>
        }
      >
        <div className={`op-spotgrid ${perWave ? 'wide' : ''}`}>
          {q.spots.map((s) => (
            <div key={s.field_deg} className="op-spotrow">
              <div className="op-spotfield"><b>{s.field_deg.toFixed(1)}°</b><span className="muted small">{fmt(s.rms_arcmin, 2)}′ RMS{s.failed ? ` · ${s.failed} rays lost` : ''}</span></div>
              {perWave
                ? system.wavelengths_nm.map((nm, w) => <SpotBox key={w} spot={s} waves={nw} only={w} scale={scale} pixel={A.pixel_mm} airy={airy} title={waveName(nm)} />)
                : <SpotBox spot={s} waves={nw} only={null} scale={scale} pixel={A.pixel_mm} airy={airy} title="F + d + C" />}
            </div>
          ))}
        </div>
      </Card>

      <div className="grid grid-2">
        <Card title="Modulation transfer function (MTF)" subtitle="Contrast the lens passes vs spatial frequency on the display: geometric MTF of the spot × the diffraction limit." actions={<DownloadButton small filename="iris-mtf.csv" content={mtfCsv} label="mtf.csv" />}>
          <Plot series={mtfSeries} height={260} xLabel="spatial frequency (cycles/mm on the display)" yLabel="MTF" yDomain={[0, 1]} vLines={[{ value: q.mtf.nyquist_lp_mm, label: 'display Nyquist', color: 'var(--accent)' }]} hLines={[{ value: 0.3, label: 'rule of thumb 0.3', color: 'var(--bad)' }]} legend />
        </Card>
        <Card title="Field curvature and distortion" subtitle="Where each field focuses (T tangential, S sagittal) relative to the flat display, and how far the image is stretched.">
          {curves && (
            <>
              <Plot series={fcSeries} height={200} xLabel="focus relative to the display (mm)" yLabel="field (°)" legend={false} vLines={[{ value: 0, color: 'var(--muted)' }]} />
              <Plot series={[{ name: 'distortion', x: curves.distortion_pct, y: curves.field_deg, color: 'var(--series-4)', width: 2.2 }]} height={150} xLabel="distortion (%)" yLabel="field (°)" legend={false} vLines={[{ value: 0, color: 'var(--muted)' }]} />
            </>
          )}
        </Card>
      </div>

      <div className="grid grid-2">
        <Card
          title="Ray fans (transverse aberration)"
          subtitle="Ray error on the display vs pupil position, relative to the primary chief ray. Flat and on top of each other = perfect."
          actions={<Seg value={fi} onChange={setFieldSel} options={q.spots.map((s, k) => ({ value: k, label: `${s.field_deg.toFixed(1)}°` }))} label="field" />}
        >
          <Plot series={fanSeries('ey')} height={190} xLabel="tangential pupil coordinate py" yLabel="e_y (µm)" legend />
          <Plot series={fanSeries('ex')} height={170} xLabel="sagittal pupil coordinate px" yLabel="e_x (µm)" legend={false} />
          {fan && !q.ideal && <Plot series={[{ name: 'OPD', x: fan.pupil, y: fan.opd_y, color: 'var(--series-3)', width: 2 }]} height={150} xLabel="py" yLabel="OPD (waves)" legend={false} />}
        </Card>
        <Card title="Colour and wavefront" subtitle="Why a single plastic lens shows rainbow fringes — and what the achromat fixes.">
          <Plot series={[{ name: 'focal shift', x: q.chromatic.shift_wavelengths_nm, y: q.chromatic.shift_mm.map((v) => v * 1000), color: 'var(--series-2)', width: 2.2 }]} height={170} xLabel="wavelength (nm)" yLabel="focus shift (µm)" legend={false} vLines={system.wavelengths_nm.map((nm, w) => ({ value: nm, label: waveName(nm).split(' ')[0], color: waveColor(w, nw) }))} />
          <Plot series={[{ name: 'lateral colour F−C', x: q.spots.map((s) => s.field_deg), y: q.chromatic.lateral_arcmin.map(Math.abs), color: 'var(--series-5)', width: 2.2 }]} height={150} xLabel="field (°)" yLabel="|F − C| (′)" hLines={[{ value: pixel, label: '1 pixel', color: 'var(--good)' }]} legend={false} />
          <div className="table-wrap">
            <table className="table">
              <thead><tr><th>Field</th><th className="op-nocaps">RMS F / d / C (µm)</th><th>Wavefront RMS</th><th>P–V</th><th>Strehl</th></tr></thead>
              <tbody>
                {q.spots.map((s, k) => (
                  <tr key={k}>
                    <td className="tnum">{s.field_deg.toFixed(1)}°</td>
                    <td className="tnum">{s.rms_per_wave_mm.map((r) => fmt(r * 1000, 1)).join(' / ')}</td>
                    <td className="tnum">{q.ideal ? '0 (ideal)' : `${fmt(q.wavefront[k]?.rms_waves, 2)} λ`}</td>
                    <td className="tnum">{q.ideal ? '0' : `${fmt(q.wavefront[k]?.pv_waves, 1)} λ`}</td>
                    <td className="tnum">{q.ideal ? '1' : fmt(q.wavefront[k]?.strehl, 3)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="small muted" style={{ marginTop: 6 }}>Longitudinal colour {fmt(q.chromatic.longitudinal_mm * 1000, 0)} µm ≈ {fmt(Math.abs(q.chromatic.longitudinal_diopter), 2)} dioptres — the eye's depth of focus (±0.3 D or so) hides part of it; lateral colour it cannot hide.</div>
        </Card>
      </div>

      <div className="op-explain">
        <Explainer title="Spots, pixels and arcminutes" refs={[PRO_REFS.smith, REFS.kress]}>
          <Formula>{`angular blur  = RMS spot / EFL             (radians → × 3438 arcmin)\none pixel     = pitch / EFL = ${fmt(A.pixel_mm * 1000, 1)} µm / ${fmt(A.lens_efl, 1)} mm = ${pixel.toFixed(2)}′\n20/20 vision  ≈ 1′ per letter stroke`}</Formula>
          <p>
            The eye sees angles, not millimetres. A blur smaller than one pixel is invisible; at two or three pixels small text smears. The worst field is the display corner
            ({Math.max(...system.fields_deg).toFixed(1)}°), where the rays use the edge of the lens.
          </p>
        </Explainer>
        <Explainer title="MTF and the display's Nyquist frequency" refs={[PRO_REFS.goodman, PRO_REFS.smith]}>
          <Formula>{`Nyquist = 1 / (2·pitch) = ${fmt(q.mtf.nyquist_lp_mm, 1)} cycles/mm\ndiffraction: MTF(ν) = (2/π)(φ − cos φ sin φ),  φ = acos(ν/ν_c),  ν_c = 1/(λN) = ${fmt(q.mtf.cutoff_lp_mm, 0)} cy/mm\ngeometric:   MTF_g(ν) = |⟨exp(−2πiνy)⟩ over the spot|`}</Formula>
          <p>
            The finest pattern a display can show is alternating on/off pixels — its Nyquist frequency. The MTF says how much of that pattern's contrast survives the lens. The product of the geometric MTF
            with the diffraction limit is a standard estimate; the exact answer is the autocorrelation of the pupil function (an FFT MTF in Zemax or rayoptics).
          </p>
        </Explainer>
        <Explainer title="The five Seidel aberrations, as you see them here" refs={[PRO_REFS.welford, PRO_REFS.kingslake, PRO_REFS.bornwolf]}>
          <p>
            <b>Spherical</b>: S-shaped on-axis fan. <b>Coma</b>: fan curved the same way on both sides, comet-shaped off-axis spots. <b>Astigmatism</b>: T and S curves split. <b>Field curvature</b>: both curves bend
            away from the flat display (the Petzval sum, set by the powers and indices, not the bending). <b>Distortion</b>: sharp but stretched image. Colour adds a longitudinal focus shift and a lateral (image-size) change with wavelength.
          </p>
        </Explainer>
        <Explainer title="Wavefront, Strehl and the diffraction limit" refs={[PRO_REFS.bornwolf, PRO_REFS.goodman]}>
          <Formula>{`OPD = optical path to the reference sphere (piston and tilt removed)\nStrehl ≈ exp(−(2π σ_W)²)       diffraction-limited: σ_W ≤ λ/14 (Maréchal)`}</Formula>
          <p>
            With a 4 mm pupil at f/{fmt(px.working_fnum, 1)}, the Airy disc ({fmt(airy * 2000, 1)} µm across) is smaller than a pixel, so Iris is <i>aberration</i>-limited, not diffraction-limited: geometric spots tell the story.
          </p>
        </Explainer>
      </div>
    </div>
  );
}
