// Shared pieces for the Optics Lab: references, parameter sliders, stat tiles.
import type { ReactNode } from 'react';
import { Slider, Stat } from '../../components/ui';
import { useProject } from '../../state/project';
import type { OpticsParams, Project } from '../../engine/types';
import type { Analysis } from './types';

export const REFS = {
  hecht: { label: 'E. Hecht, Optics, 5th ed. (Pearson, 2017)' },
  smith: { label: 'W. J. Smith, Modern Optical Engineering, 4th ed. (McGraw-Hill, 2008)' },
  kress: { label: 'B. C. Kress, Optical Architectures for Augmented-, Virtual-, and Mixed-Reality Headsets (SPIE Press, 2020)' },
  rolland: { label: 'J. P. Rolland & H. Hua, "Head-mounted display systems", Encyclopedia of Optical Engineering (2005)' },
  hfes: { label: 'ANSI/HFES 100-2007, Human Factors Engineering of Computer Workstations (character height)' },
  rayoptics: { label: 'rayoptics (open-source Python optical design)', href: 'https://github.com/mjhoptics/ray-optics' },
};

export function reqOf(p: Project, metric: string) {
  const r = p.requirements.find((x) => x.verification.metric === metric);
  return r ? { id: r.id, target: r.verification.target, hi: r.verification.target_hi, cmp: r.verification.cmp } : null;
}

function check(p: Project, metric: string, v: number): { ok: boolean | null; text: string } {
  const r = reqOf(p, metric);
  if (!r) return { ok: null, text: '' };
  let ok = false;
  let crit = '';
  if (r.cmp === 'ge') { ok = v >= r.target; crit = `≥ ${r.target}`; }
  else if (r.cmp === 'le') { ok = v <= r.target; crit = `≤ ${r.target}`; }
  else if (r.cmp === 'between') { ok = v >= r.target && v <= (r.hi ?? Infinity); crit = `${r.target}–${r.hi}`; }
  else { ok = Math.abs(v - r.target) <= (r.hi ?? 0); crit = `= ${r.target}`; }
  return { ok, text: `${ok ? '✓' : '✕'} ${crit} (${r.id})` };
}

export function OpticsStats({ a, project }: { a: Analysis; project: Project }) {
  const tone = (c: { ok: boolean | null }) => (c.ok === null ? undefined : c.ok ? 'pass' : 'fail');
  const f = check(project, 'optics.hfov_deg', a.hfov_deg);
  const p = check(project, 'optics.ppd', a.ppd);
  const v = check(project, 'optics.virtual_image_m', a.virtual_image_m);
  const e = check(project, 'optics.eyebox_mm', a.eyebox_mm);
  const vi = a.image_kind === 'virtual' ? a.virtual_image_m.toFixed(2) : a.image_kind === 'collimated' ? '∞' : 'real!';
  return (
    <div className="op-stats">
      <Stat label="Horizontal FOV" value={a.hfov_deg.toFixed(1)} unit="°" tone={tone(f)} note={`${f.text} · V ${a.vfov_deg.toFixed(1)}° · D ${a.dfov_deg.toFixed(1)}°`} />
      <Stat label="Pixels per degree" value={a.ppd.toFixed(1)} unit="px/°" tone={tone(p)} note={`${p.text} · ${a.arcmin_per_px.toFixed(2)}′ per pixel`} />
      <Stat label="Virtual image" value={vi} unit={a.image_kind === 'virtual' ? 'm' : ''} tone={tone(v)} note={`${v.text} · ${a.accommodation_d.toFixed(2)} D to focus`} />
      <Stat label="Eyebox" value={a.eyebox_mm.toFixed(1)} unit="mm" tone={tone(e)} note={`${e.text} · ${a.eyebox_full_pupil_mm.toFixed(1)} mm with the full ${a.pupil_mm} mm pupil`} />
    </div>
  );
}

/** Sliders that write design.optics. Moving f keeps the display the same distance inside f. */
export function OpticsSliders({ compact }: { compact?: boolean }) {
  const o = useProject((s) => s.project.design.optics);
  const patch = useProject((s) => s.patchDesign);
  const set = (p: Partial<OpticsParams>) => patch('optics', p);
  const inside = o.lens_focal_mm - o.display_to_lens_mm;
  const aspect = o.px_h / Math.max(1, o.px_w);
  return (
    <div>
      <div className="op-group">
        <div className="op-group-title">Lens & focus</div>
        <Slider label="Focal length f" value={o.lens_focal_mm} min={8} max={40} step={0.1} unit="mm" onChange={(v) => set({ lens_focal_mm: v, display_to_lens_mm: +(v - inside).toFixed(3) })} hint="The display follows so it stays the same distance inside f" />
        <Slider label="Display inside f by" value={inside} min={-0.3} max={2.5} step={0.01} unit="mm" format={(v) => v.toFixed(2)} onChange={(v) => set({ display_to_lens_mm: +(o.lens_focal_mm - v).toFixed(3) })} hint={`display-to-lens = ${o.display_to_lens_mm.toFixed(2)} mm; negative = beyond f (real image)`} />
        <Slider label="Lens aperture D" value={o.lens_diameter_mm} min={4} max={30} step={0.5} unit="mm" onChange={(v) => set({ lens_diameter_mm: v })} />
      </div>
      <div className="op-group">
        <div className="op-group-title">Fold & eye</div>
        <Slider label="Lens to combiner" value={o.lens_to_combiner_mm} min={3} max={20} step={0.5} unit="mm" onChange={(v) => set({ lens_to_combiner_mm: v })} />
        <Slider label="Eye relief" value={o.eye_relief_mm} min={8} max={30} step={0.5} unit="mm" onChange={(v) => set({ eye_relief_mm: v })} />
      </div>
      {!compact && (
        <div className="op-group">
          <div className="op-group-title">Micro-display</div>
          <Slider label="Active width" value={o.display_width_mm} min={3} max={20} step={0.1} unit="mm" onChange={(v) => set({ display_width_mm: v, display_height_mm: +(v * aspect).toFixed(3) })} hint="Height follows the pixel aspect ratio" />
          <Slider label="Horizontal pixels" value={o.px_w} min={160} max={1920} step={16} unit="px" format={(v) => v.toFixed(0)} onChange={(v) => { const w = Math.round(v / 16) * 16; set({ px_w: w, px_h: Math.round(w * aspect) }); }} />
          <Slider label="Luminance" value={o.display_luminance_nits} min={200} max={20000} log unit="nits" format={(v) => v.toFixed(0)} onChange={(v) => set({ display_luminance_nits: Math.round(v) })} />
          <Slider label="Combiner reflectance R" value={o.combiner_reflectance} min={0.05} max={0.9} step={0.01} format={(v) => `${(v * 100).toFixed(0)} %`} onChange={(v) => set({ combiner_reflectance: v })} />
        </div>
      )}
    </div>
  );
}

export function Seg<T extends string | number>(props: { value: T; options: { value: T; label: ReactNode; title?: string }[]; onChange: (v: T) => void; label?: string }) {
  return (
    <div className="op-seg" role="group" aria-label={props.label}>
      {props.options.map((o) => (
        <button key={String(o.value)} className={`op-seg-btn ${o.value === props.value ? 'active' : ''}`} aria-pressed={o.value === props.value} title={o.title} onClick={() => props.onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function Kv({ rows }: { rows: [ReactNode, ReactNode][] }) {
  return (
    <dl className="op-kv">
      {rows.map(([k, v], i) => (
        <div key={i} style={{ display: 'contents' }}>
          <dt>{k}</dt>
          <dd>{v}</dd>
        </div>
      ))}
    </dl>
  );
}
