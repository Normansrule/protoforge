// Face-fit helper: face width + PD → boxing-system size, decentration and
// an Iris-specific display-axis check, with a to-scale diagram.
import { useState } from 'react';
import { Card, Chip, Explainer, Formula, Slider, Stat } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { useMetricMap, useProject } from '../../state/project';
import type { FitResult } from './api';

export function FitTab() {
  const project = useProject((s) => s.project);
  const patch = useProject((s) => s.patchDesign);
  const metrics = useMetricMap();
  const [faceW, setFaceW] = useState(138);
  const [pd, setPd] = useState(63);
  const [sph, setSph] = useState(0);
  const eyebox = metrics.get('optics.eyebox_mm')?.value ?? 5;
  const { data: r } = useCompute<FitResult>('cad.fit', { face_width_mm: faceW, pd_mm: pd, sphere_d: sph, frame: project.design.frame, eyebox_mm: eyebox });
  const f = project.design.frame;
  const applied = r && r.lens_width_mm === f.lens_width_mm && r.bridge_mm === f.bridge_mm && r.temple_length_mm === f.temple_length_mm;
  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Wearer" subtitle="Measure with a ruler and a mirror, or use an optician's PD">
          <Slider label="Face width (temple to temple)" value={faceW} min={110} max={170} step={1} unit="mm" onChange={setFaceW} />
          <Slider label="Pupillary distance (PD)" value={pd} min={50} max={76} step={0.5} unit="mm" onChange={setPd} />
          <Slider label="Prescription sphere (optional)" value={sph} min={-8} max={6} step={0.25} unit="D" onChange={setSph} format={(v) => (v > 0 ? `+${v.toFixed(2)}` : v.toFixed(2))} />
          <div className="small muted" style={{ marginTop: 6 }}>
            Adult PDs are typically 54–74 mm (mean ≈ 63 mm). Display eyebox from the Optics Lab: {eyebox.toFixed(1)} mm.
          </div>
          {r && (
            <button className="btn primary" style={{ marginTop: 12, width: '100%', justifyContent: 'center' }} disabled={!!applied} onClick={() => patch('frame', r.patch)}>
              {applied ? '✓ Applied to Iris' : `Apply ${r.size_label} to the project`}
            </button>
          )}
        </Card>
        <div className="stack">
          {r && (
            <div className="cp-stats">
              <Stat label="Recommended size" value={r.size_label} note="A □ DBL – temple (mm)" />
              <Stat label="Frame PD" value={r.frame_pd_mm.toFixed(0)} unit="mm" note={`current ${r.current_frame_pd_mm.toFixed(0)} mm`} />
              <Stat
                label="Decentration / eye"
                value={r.decentration_per_eye_mm.toFixed(1)}
                unit="mm"
                note={r.decentration_per_eye_mm >= 0 ? 'optical centres move inwards' : 'optical centres move outwards'}
              />
              <Stat
                label="Pupil ↔ display axis"
                value={r.pupil_to_display_axis_mm.toFixed(1)}
                unit="mm"
                tone={r.within_eyebox ? 'pass' : 'fail'}
                note={`${r.within_eyebox ? '✓ inside' : '✕ outside'} half-eyebox ${(eyebox / 2).toFixed(1)} mm`}
              />
              <Stat label="Front width" value={r.total_width_mm.toFixed(0)} unit="mm" note={`${r.width_error_mm >= 0 ? '+' : ''}${r.width_error_mm.toFixed(0)} mm vs face`} />
              <Stat label="Prentice prism" value={r.prentice_prism_d.toFixed(2)} unit="Δ" note={sph === 0 ? 'plano lenses: none' : 'if not decentred'} />
            </div>
          )}
          <Card title="Boxing system, to scale" subtitle="Grey: current Iris frame · blue: recommendation · dots: the wearer's pupils">
            {r && <BoxingDiagram a0={f.lens_width_mm} b0={f.lens_height_mm} dbl0={f.bridge_mm} a1={r.lens_width_mm} dbl1={r.bridge_mm} pd={pd} />}
            {r && r.notes.length > 0 && (
              <div className="stack" style={{ gap: 6, marginTop: 10 }}>
                {r.notes.map((n) => (
                  <div key={n} className="small"><Chip tone="warn">! note</Chip> {n}</div>
                ))}
              </div>
            )}
          </Card>
        </div>
      </div>
      <div className="grid grid-2">
        <Explainer title="The boxing system (ISO 8624)" refs={[{ label: 'ISO 8624:2020 Ophthalmic optics — Spectacle frames — Measuring system and vocabulary' }]}>
          Draw the smallest box around each lens: its width is <b>A</b>, its height <b>B</b>; the gap between the two boxes is the
          bridge, <b>DBL</b>. That is what "50 □ 18 – 145" inside a temple means. The distance between the box centres is the
          frame PD:
          <Formula>{'frame PD = A + DBL        decentration per eye = (A + DBL − PD) / 2'}</Formula>
          When the frame PD is wider than the wearer's PD the lens optical centres are shifted (decentred) towards the nose.
        </Explainer>
        <Explainer title="Prentice's rule and the Iris display" refs={[{ label: 'C. Fowler & K. Latham Petre, Spectacle Lenses: Theory and Practice (2001)' }]}>
          Looking through a lens of power F dioptres at a point c centimetres from its optical centre bends the line of sight
          by a prism of
          <Formula>{'P [Δ] = c [cm] × F [D]'}</Formula>
          For Iris the display's virtual image is centred on the right lens box, so any decentration also moves the pupil off
          the display axis — it must stay within half the eyebox (requirement R-007 asks for ≥ 5 mm) or the image clips at
          the edges.
        </Explainer>
      </div>
    </div>
  );
}

function BoxingDiagram(p: { a0: number; b0: number; dbl0: number; a1: number; dbl1: number; pd: number }) {
  const W = 640;
  const H = 250;
  const span = Math.max(2 * p.a0 + p.dbl0, 2 * p.a1 + p.dbl1) + 30;
  const s = W / span;
  const cx = W / 2;
  const cy = 110;
  const box = (a: number, b: number, dbl: number, cls: string, dash?: string) => (
    <g className={cls}>
      {[-1, 1].map((side) => {
        const x0 = side < 0 ? cx - (dbl / 2 + a) * s : cx + (dbl / 2) * s;
        return <rect key={side} x={x0} y={cy - (b / 2) * s} width={a * s} height={b * s} rx={6} fill="none" strokeWidth={1.6} strokeDasharray={dash} />;
      })}
    </g>
  );
  const b1 = p.b0;
  const c1 = (p.a1 + p.dbl1) / 2;
  const dimY = cy + (Math.max(p.b0, b1) / 2) * s + 22;
  return (
    <svg className="cp-svg" viewBox={`0 0 ${W} ${H}`} width="100%" role="img" aria-label="Boxing system diagram">
      <g stroke="var(--muted)" opacity={0.7}>{box(p.a0, p.b0, p.dbl0, '', '5 4')}</g>
      <g stroke="var(--accent)">{box(p.a1, b1, p.dbl1, '')}</g>
      {/* box centres */}
      {[-1, 1].map((side) => (
        <g key={side}>
          <line x1={cx + side * c1 * s} y1={cy - 8} x2={cx + side * c1 * s} y2={cy + 8} stroke="var(--accent)" />
          <line x1={cx + side * c1 * s - 8} y1={cy} x2={cx + side * c1 * s + 8} y2={cy} stroke="var(--accent)" />
          <circle cx={cx + (side * p.pd * s) / 2} cy={cy} r={4.5} fill="var(--series-2)" />
        </g>
      ))}
      {/* A dimension */}
      <g className="dim">
        <line x1={cx + (p.dbl1 / 2) * s} y1={dimY} x2={cx + (p.dbl1 / 2 + p.a1) * s} y2={dimY} markerEnd="url(#arr)" markerStart="url(#arr)" />
        <line x1={cx - (p.dbl1 / 2) * s} y1={dimY} x2={cx + (p.dbl1 / 2) * s} y2={dimY} markerEnd="url(#arr)" markerStart="url(#arr)" />
        <line x1={cx - c1 * s} y1={dimY + 24} x2={cx + c1 * s} y2={dimY + 24} markerEnd="url(#arr)" markerStart="url(#arr)" />
        <line x1={cx - (p.pd / 2) * s} y1={26} x2={cx + (p.pd / 2) * s} y2={26} markerEnd="url(#arr)" markerStart="url(#arr)" />
      </g>
      <text x={cx + (p.dbl1 / 2 + p.a1 / 2) * s} y={dimY - 5} textAnchor="middle">A = {p.a1} mm</text>
      <text x={cx} y={dimY - 5} textAnchor="middle">DBL = {p.dbl1}</text>
      <text x={cx} y={dimY + 19} textAnchor="middle">frame PD = A + DBL = {p.a1 + p.dbl1} mm</text>
      <text x={cx} y={20} textAnchor="middle" style={{ fill: 'var(--series-2)' }}>wearer PD = {p.pd} mm</text>
      <defs>
        <marker id="arr" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
          <path d="M0 0L10 5L0 10z" fill="var(--muted)" />
        </marker>
      </defs>
    </svg>
  );
}
