// Optics Pro · Materials: dispersion curves n(λ), the Abbe diagram (nd vs Vd)
// of the built-in glasses and optical plastics, and an achromat calculator
// that traces a singlet against the doublet the achromat condition gives.
import { useMemo, useRef, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { toCSV } from '../control/kit';
import { svgForDownload } from './RayDiagram';
import { LensLayout } from './LensLayout';
import type { AchromatOut, Catalog, LensAnalysis, LensSystem, MaterialRow, Surface } from './proTypes';
import { fmt, PRO_REFS } from './proUtil';

const kindColor = (k: string) => (k === 'plastic' ? 'var(--series-2)' : 'var(--series-1)');

function AbbeDiagram({ mats, hi, onHover, pick }: { mats: MaterialRow[]; hi: string | null; onHover: (id: string | null) => void; pick: string[] }) {
  const W = 520, H = 330, m = { l: 48, r: 14, t: 14, b: 40 };
  const vx = (v: number) => m.l + ((85 - v) / (85 - 18)) * (W - m.l - m.r); // Vd decreases to the right (glass-map convention)
  const ny = (n: number) => H - m.b - ((n - 1.4) / (1.85 - 1.4)) * (H - m.t - m.b);
  const svg = useRef<SVGSVGElement>(null);
  // SCHOTT crown/flint line: Vd = 50 for nd < 1.60, Vd = 55 above
  const line = `M${vx(50)},${ny(1.4)} L${vx(50)},${ny(1.6)} L${vx(55)},${ny(1.6)} L${vx(55)},${ny(1.85)}`;
  return (
    <div>
      <svg ref={svg} viewBox={`0 0 ${W} ${H}`} className="op-abbe" role="img" aria-label="Abbe diagram: refractive index against Abbe number">
        {[20, 30, 40, 50, 60, 70, 80].map((v) => (
          <g key={v}>
            <line x1={vx(v)} y1={m.t} x2={vx(v)} y2={H - m.b} className="op-grid" />
            <text x={vx(v)} y={H - m.b + 16} textAnchor="middle" className="op-ax">{v}</text>
          </g>
        ))}
        {[1.4, 1.5, 1.6, 1.7, 1.8].map((n) => (
          <g key={n}>
            <line x1={m.l} y1={ny(n)} x2={W - m.r} y2={ny(n)} className="op-grid" />
            <text x={m.l - 6} y={ny(n) + 4} textAnchor="end" className="op-ax">{n.toFixed(1)}</text>
          </g>
        ))}
        <path d={line} fill="none" stroke="var(--muted)" strokeDasharray="5 4" strokeWidth={1.2} />
        <text x={vx(72)} y={ny(1.82)} className="op-ax" textAnchor="middle">crowns (low dispersion)</text>
        <text x={vx(30)} y={ny(1.42)} className="op-ax" textAnchor="middle">flints (high dispersion)</text>
        <text x={(W + m.l) / 2} y={H - 6} textAnchor="middle" className="op-ax">Abbe number Vd  (← less dispersion)</text>
        <text x={12} y={(H - m.b) / 2} textAnchor="middle" className="op-ax" transform={`rotate(-90 12 ${(H - m.b) / 2})`}>refractive index nd</text>
        {pick.length === 2 && (() => {
          const a = mats.find((x) => x.id === pick[0]);
          const b = mats.find((x) => x.id === pick[1]);
          return a && b ? <line x1={vx(a.vd)} y1={ny(a.nd)} x2={vx(b.vd)} y2={ny(b.nd)} stroke="var(--accent)" strokeWidth={2} strokeDasharray="3 3" /> : null;
        })()}
        {mats.map((x) => {
          const X = vx(x.vd), Y = ny(x.nd);
          const on = hi === x.id || pick.includes(x.id);
          return (
            <g key={x.id} onMouseEnter={() => onHover(x.id)} onMouseLeave={() => onHover(null)} style={{ cursor: 'default' }}>
              {x.kind === 'plastic'
                ? <rect x={X - 6} y={Y - 6} width={12} height={12} rx={2} fill={kindColor(x.kind)} stroke={on ? 'var(--text)' : 'var(--surface-1)'} strokeWidth={on ? 2 : 1} />
                : <circle cx={X} cy={Y} r={6.5} fill={kindColor(x.kind)} stroke={on ? 'var(--text)' : 'var(--surface-1)'} strokeWidth={on ? 2 : 1} />}
              <text x={X + 9} y={Y - 7} className="op-abbe-lbl">{x.id}</text>
            </g>
          );
        })}
      </svg>
      <div className="row small" style={{ justifyContent: 'space-between' }}>
        <span className="op-legend"><span><svg width="12" height="12"><circle cx="6" cy="6" r="5" fill={kindColor('glass')} /></svg> glass (Sellmeier)</span><span><svg width="12" height="12"><rect x="1" y="1" width="10" height="10" rx="2" fill={kindColor('plastic')} /></svg> plastic (Cauchy from nd, Vd)</span></span>
        <DownloadButton small filename="abbe-diagram.svg" content={() => svgForDownload(svg.current)} label="SVG" />
      </div>
    </div>
  );
}

function sagSphere(R: number, h: number) {
  if (!R) return 0;
  const a = Math.abs(R);
  const hh = Math.min(h, a * 0.98);
  return Math.sign(R) * (a - Math.sqrt(a * a - hh * hh));
}

function surf(p: Partial<Surface>): Surface {
  return { label: '', radius: 0, thickness: 0, material: '', conic: 0, aspheric: [], semi_diameter: 0, paraxial_f: 0, fold: false, ...p };
}

export function MaterialsTab() {
  const cat = useCompute<Catalog>('optics.materials', {});
  const [hi, setHi] = useState<string | null>(null);
  const [crown, setCrown] = useState('PMMA');
  const [flint, setFlint] = useState('PC');
  const [f, setF] = useState(50);
  const ach = useCompute<AchromatOut>('optics.achromat', { f_mm: f, crown, flint });
  const a = ach.data;
  const C = cat.data;
  // singlet vs achromat, collimated input, f/4
  const systems = useMemo(() => {
    if (!a) return null;
    const sd = f / 8;
    const base = { name: '', pupil_diameter_mm: f / 4, object_distance_mm: null, fields_deg: [0, 2], wavelengths_nm: [486.1327, 587.5618, 656.2725], primary: 1 };
    const rs = 2 * (a.n1 - 1) * f; // equiconvex singlet of the crown
    const ts = Math.max(1, sagSphere(rs, sd) - sagSphere(-rs, sd) + 1);
    const singlet: LensSystem = { ...base, surfaces: [surf({ label: 'stop', thickness: 2 }), surf({ radius: rs, thickness: ts, material: crown, semi_diameter: sd }), surf({ radius: -rs, thickness: f - ts / 2, semi_diameter: sd })] };
    const t1 = Math.max(1.5, sagSphere(a.r1_mm, sd) - sagSphere(a.r2_mm, sd) + 1);
    const t2 = Math.max(1, sagSphere(a.r2_mm, sd) - sagSphere(a.r3_mm, sd) + 1);
    const doublet: LensSystem = { ...base, surfaces: [surf({ label: 'stop', thickness: 2 }), surf({ radius: a.r1_mm, thickness: t1, material: crown, semi_diameter: sd }), surf({ radius: a.r2_mm, thickness: t2, material: flint, semi_diameter: sd }), surf({ radius: a.r3_mm, thickness: f - t1, semi_diameter: sd })] };
    return { singlet, doublet };
  }, [a, f, crown, flint]);
  const sA = useCompute<LensAnalysis>('optics.lens_analyze', { system: systems?.singlet, config: { rings: 4, rays_per_fan: 7 } }, !!systems);
  const dA = useCompute<LensAnalysis>('optics.lens_analyze', { system: systems?.doublet, config: { rings: 4, rays_per_fan: 7 } }, !!systems);
  if (!C) return <Card title="Materials">{cat.error ? <ErrorBox error={cat.error} /> : 'Loading the catalogue…'}</Card>;
  const opts = C.materials.map((m) => ({ value: m.id, label: `${m.id} (nd ${m.nd.toFixed(4)}, Vd ${m.vd.toFixed(1)})` }));
  const csv = () => toCSV(['wavelength_nm', ...C.materials.map((m) => m.id)], [C.wavelengths_nm, ...C.materials.map((m) => m.curve)]);
  return (
    <div className="stack">
      <div className="grid grid-2">
        <Card title="Dispersion n(λ)" subtitle="Every transparent material bends blue more than red. How much more is the dispersion." actions={<DownloadButton small filename="dispersion.csv" content={csv} label="CSV" />}>
          <Plot
            series={C.materials.map((m, k) => ({ name: m.id, x: C.wavelengths_nm, y: m.curve, color: `var(--series-${(k % 8) + 1})`, width: hi === m.id ? 3 : 1.8, dash: m.kind === 'plastic' ? '6 3' : undefined }))}
            height={330}
            xLabel="wavelength (nm)"
            yLabel="refractive index n"
            vLines={[{ value: C.lines_nm[0], label: 'F', color: 'var(--op-f)' }, { value: C.lines_nm[1], label: 'd', color: 'var(--op-d)' }, { value: C.lines_nm[2], label: 'C', color: 'var(--op-c)' }]}
            legend
          />
        </Card>
        <Card title="Abbe diagram (glass map)" subtitle="Index against Abbe number. Pairs far apart horizontally make good achromats.">
          <AbbeDiagram mats={C.materials} hi={hi} onHover={setHi} pick={[crown, flint]} />
        </Card>
      </div>

      <Card title="Achromat calculator" subtitle="Pick a low-dispersion crown and a high-dispersion flint: the thin-lens achromat condition gives the two powers; both lenses are then traced with real rays (f/4, collimated light).">
        <div className="op-main">
          <div className="stack" style={{ minWidth: 0 }}>
            <div className="grid grid-2">
              <div className="op-diagram">
                <div className="small muted" style={{ padding: '2px 4px' }}>Singlet ({crown}) — colours focus apart</div>
                {sA.data && <LensLayout layout={sA.data.layout} waves={3} compact maxHeight={220} />}
              </div>
              <div className="op-diagram">
                <div className="small muted" style={{ padding: '2px 4px' }}>Achromat ({crown} + {flint}) — F and C meet</div>
                {dA.data && <LensLayout layout={dA.data.layout} waves={3} compact maxHeight={220} />}
              </div>
            </div>
            <div className="op-stats">
              <Stat label="Singlet colour F−C" value={fmt((sA.data?.quality.chromatic.longitudinal_mm ?? 0) * 1000, 0)} unit="µm" note={`≈ f / V = ${fmt((a?.singlet_lca_mm ?? 0) * 1000, 0)} µm`} />
              <Stat label="Achromat colour F−C" value={fmt((dA.data?.quality.chromatic.longitudinal_mm ?? 0) * 1000, 0)} unit="µm" tone="pass" note="thick lenses and spherical aberration leave a residual (plus the secondary spectrum at d)" />
              <Stat label="Element focal lengths" value={a ? `${fmt(a.f1_mm, 1)} / ${fmt(a.f2_mm, 1)}` : '—'} unit="mm" note="the crown is stronger than the whole doublet" />
              <Stat label="Petzval radius" value={fmt(a?.petzval_radius_mm, 0)} unit="mm" note="field curvature is not fixed by an achromat" />
            </div>
          </div>
          <div className="stack">
            <Select label="Crown (positive element)" value={crown} options={opts} onChange={setCrown} />
            <Select label="Flint (negative element)" value={flint} options={opts} onChange={setFlint} />
            <Slider label="Focal length f" value={f} min={10} max={200} step={1} unit="mm" onChange={setF} />
            <ErrorBox error={ach.error} />
            {a && (
              <dl className="op-kv">
                <dt>n₁ / V₁</dt><dd>{a.n1.toFixed(4)} / {a.v1.toFixed(2)}</dd>
                <dt>n₂ / V₂</dt><dd>{a.n2.toFixed(4)} / {a.v2.toFixed(2)}</dd>
                <dt>φ₁ = φV₁/(V₁−V₂)</dt><dd>{fmt(a.phi1, 5)} mm⁻¹</dd>
                <dt>φ₂ = −φV₂/(V₁−V₂)</dt><dd>{fmt(a.phi2, 5)} mm⁻¹</dd>
                <dt>R₁ / R₂ / R₃</dt><dd>{fmt(a.r1_mm, 2)} / {fmt(a.r2_mm, 2)} / {fmt(a.r3_mm, 2)}</dd>
              </dl>
            )}
          </div>
        </div>
      </Card>

      <Card title="Catalogue" subtitle="Computed nd and Vd from the dispersion formulae, checked against the datasheet or typical catalogue values (unit-tested).">
        <div className="table-wrap">
          <table className="table">
            <thead><tr><th>Material</th><th>nd</th><th>Vd</th><th>catalogue nd / Vd</th><th>ρ (g/cm³)</th><th>Data</th></tr></thead>
            <tbody>
              {C.materials.map((m) => (
                <tr key={m.id} onMouseEnter={() => setHi(m.id)} onMouseLeave={() => setHi(null)} className={hi === m.id ? 'hi' : ''}>
                  <td><b>{m.id}</b> <span className="muted small">{m.name}</span><div className="small muted">{m.notes}</div></td>
                  <td className="tnum">{m.nd.toFixed(5)}</td>
                  <td className="tnum">{m.vd.toFixed(2)}</td>
                  <td className="tnum">{m.nd_catalog} / {m.vd_catalog}</td>
                  <td className="tnum">{m.density_g_cm3}</td>
                  <td className="small">{m.source}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      <div className="op-explain">
        <Explainer title="Sellmeier and Cauchy" refs={[PRO_REFS.schott, PRO_REFS.malitson, PRO_REFS.rii, PRO_REFS.bornwolf]}>
          <Formula>{`Sellmeier:  n²(λ) − 1 = Σ Bᵢ λ² / (λ² − Cᵢ)      (λ in µm)\nCauchy:     n(λ) = A + B / λ²\nAbbe:       Vd = (nd − 1) / (nF − nC)     F 486.1 · d 587.6 · C 656.3 nm`}</Formula>
          <p>
            The Sellmeier form comes from modelling the material as oscillators with ultraviolet and infrared resonances; glass makers publish three pairs of coefficients per glass. Optical plastics vary by
            grade, so here they are two-term Cauchy fits to typical nd and Vd — check the resin supplier's datasheet before cutting a mould.
          </p>
        </Explainer>
        <Explainer title="The achromat condition" refs={[PRO_REFS.kingslake, PRO_REFS.smith]}>
          <Formula>{`φ = φ₁ + φ₂        φ₁/V₁ + φ₂/V₂ = 0\nφ₁ = φ·V₁/(V₁ − V₂)      φ₂ = −φ·V₂/(V₁ − V₂)`}</Formula>
          <p>
            A lens's focal length changes by about f/V between the F and C lines. A strong low-dispersion positive lens and a weaker high-dispersion negative lens cancel that change while keeping net power —
            Chester Moor Hall's and John Dollond's 18th-century trick. PMMA (V ≈ 57) with polycarbonate (V ≈ 30) is the moulded-plastic version used in cheap eyepieces and phone cameras.
          </p>
        </Explainer>
      </div>
    </div>
  );
}
