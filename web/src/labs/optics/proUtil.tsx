// Shared helpers for the Optics Pro tabs: references, number cells for the
// Lens Data Editor, number formatting and prescription exports.
import { useEffect, useState } from 'react';
import type { LensSystem } from './proTypes';

export const PRO_REFS = {
  smith: { label: 'W. J. Smith, Modern Optical Engineering, 4th ed. (McGraw-Hill, 2008)' },
  kingslake: { label: 'R. Kingslake & R. B. Johnson, Lens Design Fundamentals, 2nd ed. (Academic Press / SPIE, 2010)' },
  bornwolf: { label: 'M. Born & E. Wolf, Principles of Optics, 7th ed. (Cambridge, 1999)' },
  welford: { label: 'W. T. Welford, Aberrations of Optical Systems (Adam Hilger, 1986)' },
  goodman: { label: 'J. W. Goodman, Introduction to Fourier Optics, 3rd ed. (Roberts & Co., 2005)' },
  schott: { label: 'SCHOTT optical glass datasheets and TIE-29 (Sellmeier formula)', href: 'https://www.schott.com/en-gb/products/optical-glass-p1000267' },
  malitson: { label: 'I. H. Malitson, "Interspecimen comparison of the refractive index of fused silica", J. Opt. Soc. Am. 55, 1205 (1965)' },
  rii: { label: 'refractiveindex.info — refractive index database', href: 'https://refractiveindex.info' },
  kress: { label: 'B. C. Kress, Optical Architectures for Augmented-, Virtual-, and Mixed-Reality Headsets (SPIE Press, 2020)' },
  rayoptics: { label: 'rayoptics (open-source Python lens design)', href: 'https://github.com/mjhoptics/ray-optics' },
  phydemo: { label: 'Ray Optics Simulation (web)', href: 'https://phydemo.app/ray-optics' },
  zemax: { label: 'Ansys Zemax OpticStudio' },
  codev: { label: 'Synopsys CODE V' },
};

export function fmt(v: number | null | undefined, digits = 3): string {
  if (typeof v !== 'number' || !Number.isFinite(v)) return '—';
  if (Math.abs(v) >= 1e6) return '∞';
  if (v !== 0 && Math.abs(v) < 1e-3) return v.toExponential(2);
  return v.toFixed(digits);
}

function show(v: number): string {
  if (!Number.isFinite(v)) return '';
  if (v === 0) return '0';
  const a = Math.abs(v);
  if (a < 1e-3 || a >= 1e6) return v.toExponential(4);
  return String(+v.toPrecision(7));
}

/** A table cell that edits a number; commits on blur or Enter so partial input ("-", "1e-") is allowed. */
export function NumCell(props: { value: number; onChange: (v: number) => void; disabled?: boolean; title?: string }) {
  const [text, setText] = useState(show(props.value));
  const [focus, setFocus] = useState(false);
  useEffect(() => {
    if (!focus) setText(show(props.value));
  }, [props.value, focus]);
  const commit = () => {
    const v = Number(text.trim());
    if (text.trim() !== '' && Number.isFinite(v)) {
      if (v !== props.value) props.onChange(v);
    } else setText(show(props.value));
  };
  return (
    <input
      className="op-cell tnum"
      value={text}
      disabled={props.disabled}
      title={props.title}
      inputMode="decimal"
      onFocus={() => setFocus(true)}
      onBlur={() => { setFocus(false); commit(); }}
      onChange={(e) => setText(e.target.value)}
      onKeyDown={(e) => { if (e.key === 'Enter') (e.target as HTMLInputElement).blur(); }}
    />
  );
}

export function prescriptionCsv(sys: LensSystem): string {
  const rows = ['surface,label,radius_mm,thickness_mm,material,conic,A4,A6,A8,A10,semi_diameter_mm,ideal_f_mm'];
  sys.surfaces.forEach((s, i) => {
    const a = [0, 1, 2, 3].map((k) => s.aspheric[k] ?? 0);
    rows.push([i === 0 ? 'STO' : i, `"${s.label}"`, s.radius, s.thickness, s.material || 'air', s.conic, ...a, s.semi_diameter, s.paraxial_f].join(','));
  });
  rows.push(`# pupil ${sys.pupil_diameter_mm} mm; object distance ${sys.object_distance_mm ?? 'infinity'} mm; fields ${sys.fields_deg.join(' ')} deg; wavelengths ${sys.wavelengths_nm.join(' ')} nm`);
  return rows.join('\n') + '\n';
}

/** Starting script for rayoptics (https://github.com/mjhoptics/ray-optics). */
export function rayopticsScript(sys: LensSystem, nd: (m: string) => [number, number] | null = () => null): string {
  const lines = [
    '# Iris HUD lens exported from Protoforge Optics Lab (reversed model: eye pupil = stop, display = image).',
    '# A starting point for rayoptics — check the API against the rayoptics documentation.',
    '# Materials are entered as (nd, Vd) model glasses; conic/aspheric terms are listed as comments',
    '# (add them with an EvenPolynomial surface profile in rayoptics).',
    'from rayoptics.environment import *',
    '',
    'opm = OpticalModel()',
    "sm = opm['seq_model']",
    "osp = opm['optical_spec']",
    `osp['pupil'] = PupilSpec(osp, key=['object', 'epd'], value=${sys.pupil_diameter_mm})`,
    `osp['fov'] = FieldSpec(osp, key=['object', 'angle'], value=${Math.max(...sys.fields_deg).toFixed(3)}, flds=[${sys.fields_deg.map((f) => (f / Math.max(1e-9, Math.max(...sys.fields_deg))).toFixed(4)).join(', ')}], is_relative=True)`,
    `osp['wvls'] = WvlSpec([${sys.wavelengths_nm.map((w, k) => `(${w}, ${k === sys.primary ? '1.0' : '0.5'})`).join(', ')}], ref_wl=${sys.primary})`,
    'opm.radius_mode = True',
    `sm.gaps[0].thi = 1e10  # object: the virtual image (${sys.object_distance_mm ? `${(-sys.object_distance_mm / 1000).toFixed(2)} m` : 'infinity'}); traced here from infinity`,
  ];
  sys.surfaces.forEach((s, i) => {
    const m = s.material.trim();
    const glass = m && m.toUpperCase() !== 'AIR' && m.toUpperCase() !== 'MIRROR';
    const g = glass ? nd(m) : null;
    const note = s.conic || s.aspheric.some((a) => a) ? `  # conic ${s.conic}, aspheric ${JSON.stringify(s.aspheric)}` : '';
    if (s.paraxial_f) {
      lines.push(`# surface ${i}: ideal lens f = ${s.paraxial_f} mm (use a thin paraxial element in rayoptics)`);
      return;
    }
    const data = glass ? (g ? `[${s.radius}, ${s.thickness}, ${g[0]}, ${g[1]}]` : `[${s.radius}, ${s.thickness}, '${m}']`) : `[${s.radius}, ${s.thickness}]`;
    lines.push(`sm.add_surface(${data})${note}  # ${s.label}`);
    if (i === 0) lines.push('sm.set_stop()');
  });
  lines.push('opm.update_model()', "listobj(sm)", '');
  return lines.join('\n');
}
