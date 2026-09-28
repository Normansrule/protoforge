// Design space: the field-of-view vs resolution trade over focal length.
import { useMemo } from 'react';
import { Card, DownloadButton, Explainer, Formula, Stat } from '../../components/ui';
import { Plot, type Band } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { toCSV } from '../control/kit';
import type { DesignSweep } from './types';
import { OpticsSliders, REFS } from './util';

export function DesignTab() {
  const project = useProject((s) => s.project);
  const patch = useProject((s) => s.patchDesign);
  const o = project.design.optics;
  const res = useCompute<DesignSweep>('optics.design_sweep', { project });
  const s = res.data;
  const frontier = useMemo(() => {
    const fovs = Array.from({ length: 91 }, (_, i) => 5 + i * 0.5);
    return [o.px_w, 1280, 1920].filter((v, i, arr) => arr.indexOf(v) === i).map((px) => ({ px, fov: fovs, ppd: fovs.map((f) => px / f) }));
  }, [o.px_w]);
  if (!s) return <Card title="Design space">Computing…</Card>;
  const band: Band[] = s.band_mm ? [{ from: s.band_mm[0], to: s.band_mm[1], color: 'color-mix(in srgb, var(--good) 13%, transparent)', label: 'FOV & PPD met' }] : [];
  const bandQuiet: Band[] = band.map((b) => ({ ...b, label: undefined }));
  const mid = s.band_eyebox_mm ? (s.band_eyebox_mm[0] + s.band_eyebox_mm[1]) / 2 : null;
  const inside = o.lens_focal_mm - o.display_to_lens_mm;
  const setF = (f: number) => {
    // keep the virtual image distance: 1/d = 1/f + 1/s
    const sImg = s.image_m * 1000 - (o.lens_to_combiner_mm + o.eye_relief_mm);
    const d = sImg > 0 ? 1 / (1 / f + 1 / sImg) : f - inside;
    patch('optics', { lens_focal_mm: +f.toFixed(2), display_to_lens_mm: +d.toFixed(3) });
  };
  const csv = () => toCSV(['f_mm', 'hfov_deg', 'ppd', 'eyebox_fixed_d_mm', 'eyebox_fixed_n_mm', 'px_needed'], [s.f_mm, s.hfov_deg, s.ppd, s.eyebox_fixed_d_mm, s.eyebox_fixed_n_mm, s.px_needed]);
  const pixels = o.px_w;
  return (
    <div className="stack">
      <div className="op-stats">
        <Stat label="Feasible focal length" value={s.band_mm ? `${s.band_mm[0].toFixed(1)}–${s.band_mm[1].toFixed(1)}` : 'none'} unit="mm" tone={s.band_mm ? 'pass' : 'fail'} note={`FOV ≥ ${s.min_hfov_deg}° and PPD ≥ ${s.min_ppd}`} />
        <Stat label="…with eyebox too" value={s.band_eyebox_mm ? `${s.band_eyebox_mm[0].toFixed(1)}–${s.band_eyebox_mm[1].toFixed(1)}` : 'none'} unit="mm" tone={s.band_eyebox_mm ? 'pass' : 'fail'} note={`eyebox ≥ ${s.min_eyebox_mm} mm at D = ${o.lens_diameter_mm} mm`} />
        <Stat label="Current f" value={s.current_f_mm.toFixed(1)} unit="mm" tone={s.band_eyebox_mm && s.current_f_mm >= s.band_eyebox_mm[0] && s.current_f_mm <= s.band_eyebox_mm[1] ? 'pass' : 'fail'} note={mid !== null ? `band centre ${mid.toFixed(1)} mm` : ''} />
        <Stat label="Pixel budget" value={`${pixels}`} unit="px" note={`${s.min_ppd} px/° × ${s.min_hfov_deg}° needs ≥ ${(s.min_ppd * s.min_hfov_deg).toFixed(0)} px`} tone={pixels >= s.min_ppd * s.min_hfov_deg ? 'pass' : 'fail'} />
      </div>
      <div className="op-main">
        <Card title="Field of view vs pixels per degree" subtitle={s.note} actions={<DownloadButton small filename="iris-optics-design-space.csv" content={csv} label="sweep.csv" />}>
          <Plot series={[{ name: 'horizontal FOV', x: s.f_mm, y: s.hfov_deg, color: 'var(--series-1)', width: 2.4 }]} height={210} xLabel="focal length f (mm)" yLabel="HFOV (°)" hLines={[{ value: s.min_hfov_deg, label: `R-004 ≥ ${s.min_hfov_deg}°`, color: 'var(--bad)' }]} vLines={[{ value: s.current_f_mm }]} xBands={band} legend={false} />
          <Plot series={[{ name: 'pixels per degree', x: s.f_mm, y: s.ppd, color: 'var(--series-2)', width: 2.4 }]} height={190} xLabel="focal length f (mm)" yLabel="PPD (px/°)" hLines={[{ value: s.min_ppd, label: `R-005 ≥ ${s.min_ppd}`, color: 'var(--bad)' }]} vLines={[{ value: s.current_f_mm }]} xBands={bandQuiet} legend={false} />
          <Plot series={[{ name: `eyebox, D fixed ${o.lens_diameter_mm} mm`, x: s.f_mm, y: s.eyebox_fixed_d_mm, color: 'var(--series-3)', width: 2.2 }, { name: 'eyebox, f-number fixed', x: s.f_mm, y: s.eyebox_fixed_n_mm, color: 'var(--series-7)', dash: '5 4' }]} height={190} xLabel="focal length f (mm)" yLabel="eyebox (mm)" hLines={[{ value: s.min_eyebox_mm, label: `R-007 ≥ ${s.min_eyebox_mm} mm`, color: 'var(--bad)' }]} vLines={[{ value: s.current_f_mm }]} xBands={bandQuiet} legend />
          {mid !== null && Math.abs(mid - s.current_f_mm) > 0.2 && <button className="btn small" style={{ marginTop: 8 }} onClick={() => setF(mid)}>Move Iris to the band centre (f = {mid.toFixed(1)} mm)</button>}
        </Card>
        <div className="stack">
          <Card title="The resolution frontier" subtitle="PPD × FOV = pixels: each display is one hyperbola">
            <Plot series={frontier.map((f, i) => ({ name: `${f.px} px`, x: f.fov, y: f.ppd, color: `var(--series-${i + 1})`, width: f.px === o.px_w ? 2.6 : 1.4, dash: f.px === o.px_w ? undefined : '5 4' }))} height={240} xLabel="HFOV (°)" yLabel="PPD" yDomain={[0, 100]} xBands={[{ from: s.min_hfov_deg, to: 50, color: 'color-mix(in srgb, var(--good) 10%, transparent)', label: `≥ ${s.min_hfov_deg}°` }]} hLines={[{ value: s.min_ppd, label: `${s.min_ppd} px/°`, color: 'var(--bad)' }, { value: 60, label: '60 ≈ 20/20', color: 'var(--muted)' }]} legend />
          </Card>
          <Card title="Optics" subtitle="Same controls as the ray diagram">
            <OpticsSliders compact />
          </Card>
        </div>
      </div>
      <div className="op-explain">
        <Explainer title="The classic HMD trade" refs={[REFS.kress, REFS.rolland]}>
          <Formula>{`FOV ≈ 2·atan(w / 2f)        PPD ≈ N / FOV\n⇒ PPD · FOV ≈ N  (pixels across)\nFOV ≥ ${s.min_hfov_deg}°  →  f ≤ ${s.band_mm ? s.band_mm[1].toFixed(1) : '—'} mm      PPD ≥ ${s.min_ppd}  →  f ≥ ${s.band_mm ? s.band_mm[0].toFixed(1) : '—'} mm`}</Formula>
          <p>
            A fixed micro-display has a fixed pixel count, so field of view and angular resolution pull against each other: a shorter focal length widens the view and blurs the text.
            The only ways out are more pixels (bigger or denser panels, more power) or foveation tricks. A retina-matched 60 PPD over Iris's {s.min_hfov_deg}° would need {60 * s.min_hfov_deg} pixels across.
          </p>
        </Explainer>
        <Explainer title="Eyebox and lens size" refs={[REFS.smith]}>
          <p>
            With the lens diameter fixed, a longer focal length narrows the field and so <i>grows</i> the eyebox (D − 2L·tan(FOV/2)). Holding the f-number fixed instead, the lens grows with f — bigger eyebox, but more glass mass on the temple (R-001).
            Low f-numbers (Iris: f/{(o.lens_focal_mm / o.lens_diameter_mm).toFixed(1)}) are hard to make sharp with one element; real HUDs use aspheres or doublets.
          </p>
        </Explainer>
      </div>
    </div>
  );
}
