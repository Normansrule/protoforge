// Ray diagram tab: folded optical path with animated light, live metrics,
// the ray-transfer matrix and a to-scale view of the virtual image.
import { useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, DownloadButton, ErrorBox, Explainer, Formula } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { IRIS, useProject } from '../../state/project';
import { usePrefersReducedMotion } from '../control/kit';
import type { TraceOut } from './types';
import { FarView, fieldColor, RayDiagram, svgForDownload } from './RayDiagram';
import { Kv, OpticsSliders, OpticsStats, REFS } from './util';

export function RayTab() {
  const project = useProject((s) => s.project);
  const patch = useProject((s) => s.patchDesign);
  const reduced = usePrefersReducedMotion();
  const [playing, setPlaying] = useState(!reduced);
  const [rays, setRays] = useState(5);
  const [back, setBack] = useState(true);
  const [world, setWorld] = useState(true);
  const [hi, setHi] = useState<number | null>(null);
  const svg = useRef<SVGSVGElement>(null);
  const tr = useCompute<TraceOut>('optics.trace', { project, config: { rays, fields: 5 } });
  const t = tr.data;
  const a = t?.analysis;
  const vi = project.requirements.find((r) => r.verification.metric === 'optics.virtual_image_m')?.verification;
  return (
    <div className="stack">
      <ErrorBox error={tr.error} />
      {a && <OpticsStats a={a} project={project} />}
      <div className="op-main">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card
            title="The folded light path (top view, to scale)"
            subtitle="Each colour is one point on the display; the lens bends its rays almost parallel, the combiner folds them into the eye, and the eye focuses each colour to one spot on the retina."
            actions={t && (
              <div className="row" style={{ gap: 6 }}>
                <button className="btn small" onClick={() => setPlaying((p) => !p)} aria-label={playing ? 'pause light pulses' : 'play light pulses'}>{playing ? '❚❚ Pause' : '▶ Play'}</button>
                <DownloadButton small filename="iris-ray-diagram.svg" content={() => svgForDownload(svg.current)} label="SVG" />
                <DownloadButton small filename="iris-rays.json" content={() => JSON.stringify({ optics: project.design.optics, geometry: t.geometry, fields: t.fields, rays: t.rays, analysis: t.analysis }, null, 1)} label="rays.json" />
              </div>
            )}
          >
            {t && <RayDiagram ref={svg} trace={t} playing={playing} showBack={back} showWorld={world} highlightField={hi} maxHeight={620} />}
            {t && (
              <div className="op-toolbar">
                <span className="small muted">Field points</span>
                {t.fields.map((f, i) => (
                  <button key={i} className="btn small" style={{ borderColor: fieldColor(i, t.fields.length), background: hi === i ? 'var(--surface-3)' : undefined }} onMouseEnter={() => setHi(i)} onMouseLeave={() => setHi(null)} onFocus={() => setHi(i)} onBlur={() => setHi(null)} aria-label={`field ${f.angle_deg.toFixed(1)} degrees`}>
                    <span className="op-sw" style={{ background: fieldColor(i, t.fields.length) }} /> {f.angle_deg >= 0 ? '+' : ''}{f.angle_deg.toFixed(1)}°
                  </button>
                ))}
                <span style={{ flex: 1 }} />
                <label className="op-check"><input type="checkbox" checked={back} onChange={(e) => setBack(e.target.checked)} /> virtual rays</label>
                <label className="op-check"><input type="checkbox" checked={world} onChange={(e) => setWorld(e.target.checked)} /> world light</label>
                <select value={rays} onChange={(e) => setRays(Number(e.target.value))} aria-label="rays per field" style={{ width: 'auto', padding: '3px 6px', fontSize: 12 }}>
                  {[3, 5, 7, 11].map((n) => <option key={n} value={n}>{n} rays / field</option>)}
                </select>
              </div>
            )}
            {a && a.warnings.length > 0 && <div className="callout op-warn" style={{ marginTop: 10 }}>{a.warnings.map((w, i) => <div key={i} className="small">{w}</div>)}</div>}
          </Card>
          {t && (
            <Card title="Unfolded, to scale: the eye and its virtual image" subtitle="The combiner only folds the path; unfold it and the display becomes a large image floating in the world.">
              <FarView trace={t} reqLo={vi?.target ?? 1} reqHi={vi?.target_hi ?? 3} />
            </Card>
          )}
        </div>
        <div className="stack">
          <Card title="Optics (design.optics)" subtitle="Every slider re-traces and re-verifies R-004…R-007">
            <OpticsSliders />
            <button className="btn small" onClick={() => patch('optics', IRIS.design.optics)}>Reset to Iris</button>
          </Card>
          {a && (
            <Card title="First-order numbers">
              <Kv rows={[
                ['Image distance sᵢ', a.image_kind === 'collimated' ? '∞' : `${a.s_i_mm.toFixed(0)} mm (${a.image_kind})`],
                ['Magnification m', a.image_kind === 'virtual' ? `${a.magnification.toFixed(1)}×` : '—'],
                ['Image size', a.image_kind === 'virtual' ? `${(a.image_width_mm / 10).toFixed(0)} × ${(a.image_height_mm / 10).toFixed(0)} cm` : '—'],
                ['Collimated FOV 2·atan(w/2f)', `${a.hfov_collimated_deg.toFixed(2)}°`],
                ['Centre PPD', `${a.ppd_center.toFixed(1)} px/°`],
                ['f-number / NA', `f/${a.f_number.toFixed(2)} · ${a.na.toFixed(2)}`],
                ['Focus sensitivity', `${a.focus_sensitivity_d_per_mm.toFixed(2)} D per mm`],
                ['Min combiner length', `${a.combiner_min_mm.toFixed(1)} mm`],
                ['Min lens-to-combiner', `${a.min_lens_to_combiner_mm.toFixed(1)} mm (now ${project.design.optics.lens_to_combiner_mm})`],
                ['Image luminance', `${a.image_nits.toFixed(0)} nits`],
              ]} />
              <div className="small muted" style={{ marginTop: 8 }}>Ray-transfer matrix, display → eye (mm, rad):</div>
              <div className="row" style={{ marginTop: 4, gap: 8 }}>
                <span className="mono small">M =</span>
                <div className="op-matrix"><span>{a.abcd.a.toFixed(3)}</span><span>{a.abcd.b.toFixed(2)}</span><span>{a.abcd.c.toFixed(4)}</span><span>{a.abcd.d.toFixed(3)}</span></div>
                <span className="mono small">det = {(a.abcd.a * a.abcd.d - a.abcd.b * a.abcd.c).toFixed(3)}</span>
              </div>
            </Card>
          )}
        </div>
      </div>
      {a && (
        <div className="op-explain">
          <Explainer title="A magnifier that floats text in the world" refs={[REFS.hecht, REFS.smith]}>
            <p>Put an object just inside the focal length of a converging lens and the thin-lens equation gives a large, upright, <i>virtual</i> image far behind it:</p>
            <Formula>{`1/sₒ + 1/sᵢ = 1/f   →   sᵢ = f·d/(d − f) = ${a.f_mm.toFixed(1)}·${a.d_mm.toFixed(2)}/(${(a.d_mm - a.f_mm).toFixed(2)}) = ${a.image_kind === 'collimated' ? '∞' : a.s_i_mm.toFixed(0) + ' mm'}\nm = −sᵢ/sₒ = ${a.image_kind === 'virtual' ? a.magnification.toFixed(1) : '—'}      image distance from the eye = |sᵢ| + L = ${a.image_kind === 'virtual' ? a.virtual_image_m.toFixed(2) + ' m' : '—'}`}</Formula>
            <p>Moving the display by only {(1 / a.focus_sensitivity_d_per_mm).toFixed(2)} mm changes the image vergence by a full dioptre — which is why HUD optics need precise, stable spacers.</p>
          </Explainer>
          <Explainer title="Field of view and pixels per degree" refs={[REFS.kress, REFS.rolland]}>
            <Formula>{`FOV = 2·atan((m·w/2)/(|sᵢ| + L)) → 2·atan(w/2f) = ${a.hfov_collimated_deg.toFixed(1)}° (collimated)\nPPD = pixels / FOV = ${project.design.optics.px_w} / ${a.hfov_deg.toFixed(1)}° = ${a.ppd.toFixed(1)} px/°`}</Formula>
            <p>
              Field of view (FOV) depends only on display width and focal length — not on the lens diameter. 60 pixels per degree (PPD) matches 20/20 acuity (1 arcmin per pixel);
              Iris's {a.ppd.toFixed(0)} px/° is ≈ 20/{(20 * 60 / a.ppd).toFixed(0)}: fine for text, not for photos.
            </p>
          </Explainer>
          <Explainer title="Ray-transfer (ABCD) matrices" refs={[REFS.hecht, REFS.smith]}>
            <p>Paraxial rays are vectors [height y, angle u]; free space and thin lenses are 2 × 2 matrices, and a system is their product:</p>
            <Formula>{`T(t) = [1 t; 0 1]    Λ(f) = [1 0; −1/f 1]\nM = T(L)·Λ(f)·T(d) = [${a.abcd.a.toFixed(3)} ${a.abcd.b.toFixed(2)}; ${a.abcd.c.toFixed(4)} ${a.abcd.d.toFixed(3)}]   det M = 1`}</Formula>
            <p>The trace here refracts each ray with exactly this lens matrix, reflects it in the tilted mirror and follows it into a <i>reduced eye</i> (one refracting surface of 60 D plus accommodation, index 1.336, retina 22.3 mm behind): all rays of one colour meet at one retinal spot.</p>
          </Explainer>
          <Explainer title="Iris tie-in and where to go next" refs={[REFS.rayoptics]}>
            <p>
              The fold mirrors the image left–right, so firmware flips the frame buffer. The combiner must be at least {a.combiner_min_mm.toFixed(0)} mm long to catch every ray — it sets the size of the optics housing in the <Link to="/cad">CAD Studio</Link>.
              Paraxial optics ignores aberrations: at ±{(a.hfov_deg / 2).toFixed(0)}° a single lens shows field curvature, distortion and lateral colour. Graduate to Zemax OpticStudio or the open-source <b>rayoptics</b> for real surfaces and glasses.
            </p>
          </Explainer>
        </div>
      )}
    </div>
  );
}
