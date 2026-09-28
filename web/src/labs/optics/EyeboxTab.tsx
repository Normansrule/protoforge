// Eyebox: slide (or drag) the eye sideways and watch field points vignette.
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, DownloadButton, Explainer, Formula, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { toCSV, usePrefersReducedMotion } from '../control/kit';
import type { TraceOut } from './types';
import { fieldColor, RayDiagram } from './RayDiagram';
import { reqOf, REFS } from './util';

export function EyeboxTab() {
  const project = useProject((s) => s.project);
  const patch = useProject((s) => s.patchDesign);
  const o = project.design.optics;
  const reduced = usePrefersReducedMotion();
  const [off, setOff] = useState(0);
  const [pupil, setPupil] = useState(4);
  const tr = useCompute<TraceOut>('optics.trace', { project, config: { eye_offset_mm: off, rays: 9, fields: 5, extras: { pupil_mm: pupil, lens_transmission: 0.92, backgrounds: [], world_m: 10 } } });
  const t = tr.data;
  const req = reqOf(project, 'optics.eyebox_mm');
  // pupil fill across the whole display width at this eye offset (unfolded geometry)
  const strip = useMemo(() => {
    if (!t) return '';
    const a = t.analysis;
    const L = a.path_mm, d = a.d_mm, D = a.aperture_mm;
    const hw = (D * Math.abs(a.k)) / 2;
    const stops: string[] = [];
    const n = 48;
    for (let i = 0; i <= n; i++) {
      const y0 = o.display_width_mm / 2 - (o.display_width_mm * i) / n;
      const c = (-y0 * L) / d;
      const lo = Math.max(c - hw, -off - pupil / 2), hi = Math.min(c + hw, -off + pupil / 2);
      const fill = pupil > 0 ? Math.max(0, Math.min(1, (hi - lo) / pupil)) : 1;
      stops.push(`color-mix(in srgb, var(--series-3) ${(fill * 100).toFixed(0)}%, var(--surface-1)) ${((i / n) * 100).toFixed(1)}%`);
    }
    return `linear-gradient(90deg, ${stops.join(', ')})`;
  }, [t, off, pupil, o.display_width_mm]);
  if (!t) return <Card title="Eyebox">Tracing…</Card>;
  const a = t.analysis;
  const s = t.sweep;
  const half = a.eyebox_mm / 2;
  const csv = () => toCSV(['eye_offset_mm', 'visible_fov_deg', 'visible_frac', ...t.fields.map((f) => `fill_field_${f.angle_deg.toFixed(1)}deg`)], [s.offset_mm, s.visible_fov_deg, s.visible_frac, ...s.field_fill]);
  return (
    <div className="stack">
      <div className="op-stats">
        <Stat label="Eyebox (pupil centre)" value={a.eyebox_mm.toFixed(2)} unit="mm" tone={req ? (a.eyebox_mm >= req.target ? 'pass' : 'fail') : undefined} note={req ? `${a.eyebox_mm >= req.target ? '✓' : '✕'} ≥ ${req.target} mm (${req.id}) · approx. ${a.eyebox_approx_mm.toFixed(2)} mm` : ''} />
        <Stat label="Whole pupil lit" value={a.eyebox_full_pupil_mm.toFixed(2)} unit="mm" note={`every field fills the ${pupil} mm pupil`} />
        <Stat label="Some light" value={a.eyebox_partial_mm.toFixed(2)} unit="mm" note="every field reaches part of the pupil" />
        <Stat label="Visible field now" value={t.visible_fov_deg.toFixed(1)} unit="°" tone={t.visible_fov_deg >= a.hfov_deg - 0.05 ? 'pass' : 'warn'} note={`of ${a.hfov_deg.toFixed(1)}° at ${off >= 0 ? '+' : ''}${off.toFixed(1)} mm`} />
      </div>
      <div className="op-main">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card title="Move the eye" subtitle="Drag the eye (or use the slider). Rays that miss the pupil are drawn faint and marked ×; the edge of the field goes dark first.">
            <RayDiagram trace={t} playing={!reduced} sparse showWorld={false} onDragEye={(v) => setOff(Math.round(v * 10) / 10)} maxHeight={560} />
            <div style={{ marginTop: 10 }}>
              <div className="small muted" style={{ marginBottom: 4 }}>What this eye sees across the display width (green = lit pupil fraction)</div>
              <div className="op-strip" style={{ background: strip }} role="img" aria-label="visible field strip" />
            </div>
          </Card>
          <Card title="Visible field vs eye position" subtitle="Shaded: the eyebox — the pupil centre sees every field point" actions={<DownloadButton small filename="iris-eyebox.csv" content={csv} label="eyebox.csv" />}>
            <Plot series={[{ name: 'visible horizontal FOV', x: s.offset_mm, y: s.visible_fov_deg, color: 'var(--accent)', width: 2.4, area: true }]} height={200} xLabel="eye offset (mm)" yLabel="visible FOV (°)" xBands={[{ from: -half, to: half, color: 'color-mix(in srgb, var(--good) 13%, transparent)', label: 'eyebox' }]} vLines={[{ value: off, color: 'var(--series-2)' }]} yZero legend={false} />
            <Plot series={t.fields.map((f, i) => ({ name: `${f.angle_deg >= 0 ? '+' : ''}${f.angle_deg.toFixed(1)}°`, x: s.offset_mm, y: s.field_fill[i].map((v) => v * 100), color: fieldColor(i, t.fields.length) }))} height={200} xLabel="eye offset (mm)" yLabel="pupil lit (%)" xBands={[{ from: -half, to: half, color: 'color-mix(in srgb, var(--good) 13%, transparent)' }]} vLines={[{ value: off, color: 'var(--series-2)' }]} yDomain={[0, 105]} legend />
          </Card>
        </div>
        <div className="stack">
          <Card title="Eye & aperture">
            <Slider label="Eye offset" value={off} min={-12} max={12} step={0.1} unit="mm" onChange={setOff} />
            <Slider label="Pupil diameter" value={pupil} min={1.5} max={8} step={0.1} unit="mm" onChange={setPupil} hint="≈2 mm in sunlight, 4 mm indoors, 7 mm in the dark" />
            <Slider label="Lens aperture D (design)" value={o.lens_diameter_mm} min={4} max={30} step={0.5} unit="mm" onChange={(v) => patch('optics', { lens_diameter_mm: v })} />
            <Slider label="Eye relief (design)" value={o.eye_relief_mm} min={8} max={30} step={0.5} unit="mm" onChange={(v) => patch('optics', { eye_relief_mm: v })} />
            <button className="btn small" onClick={() => setOff(0)}>Centre the eye</button>
          </Card>
          <Card title="Pupil fill per field point">
            <div className="stack" style={{ gap: 6 }}>
              {t.fields.map((f, i) => (
                <div className="op-fillbar" key={i}>
                  <span className="mono"><span className="op-sw" style={{ background: fieldColor(i, t.fields.length) }} /> {f.angle_deg >= 0 ? '+' : ''}{f.angle_deg.toFixed(1)}°</span>
                  <div className="track"><div style={{ width: `${f.pupil_fill * 100}%`, background: fieldColor(i, t.fields.length) }} /></div>
                  <span className="mono tnum" style={{ textAlign: 'right' }}>{(f.pupil_fill * 100).toFixed(0)} %{f.pupil_fill < 0.5 ? ' ✕' : ''}</span>
                </div>
              ))}
            </div>
          </Card>
        </div>
      </div>
      <div className="op-explain">
        <Explainer title="Where the eyebox comes from" refs={[REFS.rolland, REFS.kress]}>
          <p>The lens rim is the aperture stop. Each field point leaves the lens as a beam about D wide that tilts with its field angle, so at the eye the beams from the two edges of the field only overlap in a window:</p>
          <Formula>{`eyebox = D·|k| − w·L/d,  k = 1 + L/d − L/f\n≈ D − 2·L·tan(FOV/2) = ${a.aperture_mm} − 2·${a.path_mm.toFixed(0)}·tan(${(a.hfov_deg / 2).toFixed(1)}°) = ${a.eyebox_approx_mm.toFixed(2)} mm (collimated)\nexact here: ${a.eyebox_mm.toFixed(2)} mm;  with pupil p: eyebox ∓ p`}</Formula>
          <p>Double the eye relief or the field of view and the eyebox shrinks fast; the only cure is a bigger (heavier) lens — or pupil replication in waveguide displays (Kress 2020).</p>
        </Explainer>
        <Explainer title="Iris tie-in">
          <p>
            A {a.eyebox_mm.toFixed(1)} mm eyebox tolerates about ±{half.toFixed(1)} mm of frame slip or pupillary-distance (PD) mismatch before the edge text dims. The <Link to="/cad">CAD Studio</Link>'s face-fit tab
            checks that the wearer's pupil lands inside it, and R-007 requires ≥ {req?.target ?? 5} mm. The eye also rotates: looking 10° to the side moves the pupil ≈ 2 mm, which eats into the same budget.
          </p>
        </Explainer>
      </div>
    </div>
  );
}
