// Molding Lab — injection-molding DFM on the CAD frame parts.
import { useMemo, useState } from 'react';
import { LabPage } from '../../components/LabPage';
import { Card, DownloadButton, EmptyState, ErrorBox, Explainer, Formula, NumberField, Select, Slider, Stat, StatusChip, Tabs } from '../../components/ui';
import { Plot, fmtNum } from '../../components/Plot';
import { EngineMesh, Scene3D, Segments } from '../../components/Scene3D';
import { useCompute } from '../../engine/hooks';
import { call } from '../../engine/engine';
import { useProject } from '../../state/project';
import { CycleAnimation } from './CycleAnimation';
import { Safe3D } from '../pcb/Safe3D';
import type { Analysis, DraftResult, Material, PartInfo, ProcessOptions, Steel, ThicknessResult, V3 } from './types';

type View = 'draft' | 'thickness';
const PULLS: { value: string; label: string; dir: V3 | null }[] = [
  { value: 'auto', label: 'Auto (best draft)', dir: null },
  { value: '+X', label: '±X', dir: [1, 0, 0] },
  { value: '+Y', label: '±Y', dir: [0, 1, 0] },
  { value: '+Z', label: '±Z', dir: [0, 0, 1] },
];

const REFS = {
  rosato: { label: 'D. V. Rosato, M. G. Rosato, Injection Molding Handbook' },
  osswald: { label: 'T. A. Osswald, Understanding Polymer Processing' },
  menges: { label: 'G. Menges, W. Michaeli, P. Mohren, How to Make Injection Molds' },
  protolabs: { label: 'Protolabs injection-molding design guidelines', href: 'https://www.protolabs.com/services/injection-molding/' },
  mt: { label: 'T. Möller, B. Trumbore, "Fast, minimum storage ray/triangle intersection", 1997' },
};

function coolingTime(s: number, m: Material) {
  const r = (8 / Math.PI ** 2) * (m.t_melt_c - m.t_mold_c) / (m.t_eject_c - m.t_mold_c);
  return r <= 1 ? 0 : (s * s) / (Math.PI ** 2 * m.diffusivity_mm2_s) * Math.log(r);
}

function Legend({ items }: { items: { color: string; label: string }[] }) {
  return (
    <div className="row small" style={{ gap: 12 }}>
      {items.map((i) => <span key={i.label} className="row" style={{ gap: 5 }}><span style={{ width: 12, height: 12, borderRadius: 3, background: i.color }} />{i.label}</span>)}
    </div>
  );
}

export default function Lab() {
  const project = useProject((s) => s.project);
  const patch = useProject((s) => s.patchDesign);
  const mfg = project.design.manufacturing;
  const parts = useCompute<PartInfo[]>('molding.parts', project);
  const mats = useCompute<{ materials: Material[]; steels: Steel[] }>('molding.materials', {});
  const [part, setPart] = useState('');
  const [pull, setPull] = useState('auto');
  const [minDraft, setMinDraft] = useState(1.0);
  const [view, setView] = useState<View>('draft');
  const [thickOn, setThickOn] = useState(false);
  const [opts, setOpts] = useState({ clamp_allowance: 1.1, scrap_pct: 3, runner_fraction: 0.15, machine_rate_usd_h: 0 });
  const partName = part || parts.data?.[0]?.name || '';
  const pullDir = PULLS.find((p) => p.value === pull)?.dir ?? null;
  const draft = useCompute<DraftResult>('molding.draft', { project, part: partName, pull: pullDir, min_draft_deg: minDraft, mesh: true }, !!partName);
  const thick = useCompute<ThicknessResult>('molding.thickness', { project, part: partName, mesh: true }, thickOn && !!partName);
  const options: ProcessOptions = { material: mfg.molding_material, steel: mfg.mold_steel, cavities: mfg.cavities, min_draft_deg: minDraft, ...opts };
  const an = useCompute<Analysis>('molding.analyze', { project, options });
  const pp = an.data?.parts.find((x) => x.part === partName) ?? an.data?.parts[0];
  const info = parts.data?.find((x) => x.name === partName);
  const mat = mats.data?.materials.find((m) => m.id === mfg.molding_material) ?? an.data?.material;
  const [wall, setWall] = useState(2);

  const bb = info ? { lo: info.bbox_min, hi: info.bbox_max } : { lo: [0, 0, 0] as V3, hi: [10, 10, 10] as V3 };
  const c: V3 = [(bb.lo[0] + bb.hi[0]) / 2, (bb.lo[1] + bb.hi[1]) / 2, (bb.lo[2] + bb.hi[2]) / 2];
  const diag = Math.hypot(bb.hi[0] - bb.lo[0], bb.hi[1] - bb.lo[1], bb.hi[2] - bb.lo[2]) || 50;
  const target: [number, number, number] = [c[0], c[2], -c[1]];
  const cam: [number, number, number] = [c[0] + diag * 0.75, c[2] + diag * 0.55, -c[1] + diag * 0.9];
  const arrow = useMemo(() => {
    const d = draft.data?.pull ?? [0, 0, 1];
    const L = diag * 0.45;
    const tip = [c[0] + d[0] * L, c[1] + d[1] * L, c[2] + d[2] * L];
    const base = [c[0] - d[0] * L * 0.2, c[1] - d[1] * L * 0.2, c[2] - d[2] * L * 0.2];
    // arrow head: two short lines perpendicular-ish
    const helper = Math.abs(d[0]) < 0.9 ? [1, 0, 0] : [0, 1, 0];
    const u = [d[1] * helper[2] - d[2] * helper[1], d[2] * helper[0] - d[0] * helper[2], d[0] * helper[1] - d[1] * helper[0]];
    const h = L * 0.12;
    const back = [tip[0] - d[0] * h * 1.6, tip[1] - d[1] * h * 1.6, tip[2] - d[2] * h * 1.6];
    return [...base, ...tip, ...tip, back[0] + u[0] * h, back[1] + u[1] * h, back[2] + u[2] * h, ...tip, back[0] - u[0] * h, back[1] - u[1] * h, back[2] - u[2] * h];
  }, [draft.data?.pull, diag, c]);

  const mesh = view === 'draft' ? draft.data?.mesh : thick.data?.mesh;
  const costCurve = useMemo(() => {
    if (!an.data) return null;
    const qs = Array.from({ length: 49 }, (_, k) => Math.round(10 ** (k / 8)));
    const unit = qs.map((q) => an.data!.parts.reduce((s, x) => s + x.piece.total_usd + (x.tool_cost_usd * Math.max(1, Math.ceil(q / x.shot_life))) / q, 0));
    const piece = qs.map(() => an.data!.parts.reduce((s, x) => s + x.piece.total_usd, 0));
    const lo = Math.min(...piece, ...unit);
    const hi = Math.max(...piece, ...unit);
    return { qs, unit, piece, dom: [lo / 2, hi * 1.5] as [number, number] };
  }, [an.data]);

  const report = () => call<string>('mfg.molding_report', project);

  return (
    <LabPage
      kicker="Production · Injection molding"
      title="Molding Lab"
      subtitle={<>Design-for-manufacture checks for injection-molding the Iris frame: draft, wall thickness and undercuts on the CAD geometry, then clamp force, cooling and cycle time, tooling and piece-price estimates for {mat?.name ?? mfg.molding_material}.</>}
      metricPrefixes={['molding.']}
      actions={<DownloadButton filename="molding-dfm-report.md" content={report} label="DFM report" />}
    >
      <ErrorBox error={parts.error || draft.error || an.error} />
      {parts.data && parts.data.length === 0 && <EmptyState>The project has no frame parts yet.</EmptyState>}
      <div className="grid grid-sidebar">
        <div className="stack">
          <Card title="Part & pull">
            <Select label="Part" value={partName} onChange={setPart} options={(parts.data ?? []).map((p) => ({ value: p.name, label: `${p.name} · ${p.triangles.toLocaleString()} triangles` }))} />
            <Select label="Pull direction" value={pull} onChange={setPull} options={PULLS.map((p) => ({ value: p.value, label: p.label }))} />
            <Slider label="Minimum draft" value={minDraft} min={0.25} max={3} step={0.25} unit="°" onChange={setMinDraft} hint="Textured surfaces need ~1.5° per 0.025 mm of texture depth" />
            {draft.data && (
              <div className="small muted">Chosen pull: <b>{draft.data.pull_label}</b>. Candidates: {draft.data.candidates.map((c) => `${c.label} ${fmtNum(c.min_draft_deg)}°`).join(' · ')}</div>
            )}
          </Card>
          <Card title="Material & tool">
            <Select label="Resin" value={mfg.molding_material} onChange={(v) => patch('manufacturing', { molding_material: v })} options={(mats.data?.materials ?? []).map((m) => ({ value: m.id, label: `${m.id} — ${m.name}` }))} />
            <Select label="Tool steel" value={mfg.mold_steel} onChange={(v) => patch('manufacturing', { mold_steel: v })} options={(mats.data?.steels ?? []).map((s) => ({ value: s.id, label: s.name }))} />
            <Select label="Cavities" value={String(mfg.cavities)} onChange={(v) => patch('manufacturing', { cavities: Number(v) })} options={['1', '2', '4', '8'].map((v) => ({ value: v, label: `${v} cavit${v === '1' ? 'y' : 'ies'}` }))} />
            <div className="grid grid-2" style={{ gap: 8 }}>
              <NumberField label="Clamp allowance" value={opts.clamp_allowance} step={0.05} onChange={(v) => setOpts({ ...opts, clamp_allowance: v })} />
              <NumberField label="Scrap" unit="%" value={opts.scrap_pct} step={0.5} onChange={(v) => setOpts({ ...opts, scrap_pct: v })} />
              <NumberField label="Runner / part mass" value={opts.runner_fraction} step={0.05} onChange={(v) => setOpts({ ...opts, runner_fraction: v })} />
              <NumberField label="Press rate (0 = auto)" unit="$/h" value={opts.machine_rate_usd_h} step={5} onChange={(v) => setOpts({ ...opts, machine_rate_usd_h: v })} />
            </div>
            {mat && <div className="small muted">{mat.notes} Typical: melt {mat.t_melt_c} °C, mold {mat.t_mold_c} °C, eject {mat.t_eject_c} °C, α = {mat.diffusivity_mm2_s} mm²/s, shrink {mat.shrinkage_pct} %, ≈ ${mat.price_usd_kg}/kg (indicative).</div>}
          </Card>
        </div>
        <div className="stack">
          <Card
            title={view === 'draft' ? 'Draft map' : 'Wall thickness map'}
            subtitle={view === 'draft' ? 'Faces coloured by draft relative to the pull (arrow); undercuts found by ray casting' : `Local thickness = shortest of five rays into the wall · nominal ${fmtNum(thick.data?.nominal_mm ?? 0)} mm`}
            actions={<Tabs<View> value={view} onChange={(v) => { setView(v); if (v === 'thickness') setThickOn(true); }} tabs={[{ id: 'draft', label: 'Draft' }, { id: 'thickness', label: 'Thickness' }]} />}
          >
            {mesh ? (
              <Safe3D height={380}>
                <Scene3D height={380} camera={cam} target={target} grid={false}>
                  <EngineMesh mesh={mesh} roughness={0.7} />
                  {view === 'draft' && <Segments points={arrow} color="#3987e5" />}
                </Scene3D>
              </Safe3D>
            ) : (
              <EmptyState>{view === 'thickness' && thick.ready && !thick.data ? 'Casting rays…' : 'Computing…'}</EmptyState>
            )}
            {view === 'draft' ? (
              <Legend items={[{ color: 'rgb(219,46,41)', label: '0° (no draft)' }, { color: 'rgb(219,158,41)', label: `below ${minDraft}°` }, { color: 'rgb(41,191,92)', label: `≥ ${2 * minDraft}°` }, { color: 'rgb(140,158,184)', label: 'floor / top (face-on)' }, { color: 'rgb(217,51,191)', label: 'undercut' }, { color: '#3987e5', label: 'pull' }]} />
            ) : (
              <Legend items={[{ color: 'rgb(230,51,51)', label: 'thin (short-shot risk)' }, { color: 'rgb(38,166,140)', label: 'within the nominal band' }, { color: 'rgb(115,77,217)', label: 'thick (sink / voids)' }]} />
            )}
          </Card>
          <div className="grid grid-2">
            {view === 'draft' && draft.data && (
              <Card title="Draft histogram" subtitle="Side-wall area per 0.5° bin">
                <Plot series={[{ name: 'area', x: draft.data.hist_edges, y: draft.data.hist_area, step: true, area: true }]} height={200} xLabel="draft (°)" yLabel="area (mm²)" vLines={[{ value: minDraft, label: 'minimum', color: 'var(--bad)' }]} />
              </Card>
            )}
            {view === 'thickness' && thick.data && (
              <Card title="Thickness histogram" subtitle={`${thick.data.sampled_faces.toLocaleString()} faces probed`}>
                <Plot series={[{ name: 'area', x: thick.data.hist_edges, y: thick.data.hist_area, step: true, area: true }]} height={200} xLabel="wall (mm)" yLabel="area (mm²)" vLines={[{ value: thick.data.nominal_mm, label: 'nominal' }]} />
              </Card>
            )}
            {view === 'draft' && draft.data && (
              <Card title="Draft check">
                <div className="grid grid-2" style={{ gap: 8 }}>
                  <Stat label="Min side-wall draft" value={fmtNum(draft.data.min_draft_deg)} unit="°" tone={draft.data.min_draft_deg >= minDraft ? 'pass' : 'fail'} />
                  <Stat label="Undercut area" value={fmtNum(draft.data.area_undercut_mm2)} unit="mm²" tone={draft.data.area_undercut_mm2 > 1 ? 'warn' : 'pass'} />
                  <Stat label="Below minimum" value={fmtNum(draft.data.area_low_mm2 + draft.data.area_zero_mm2)} unit="mm²" />
                  <Stat label="Faces" value={draft.data.faces.toLocaleString()} note={`${draft.data.side_faces.toLocaleString()} side walls`} />
                </div>
                <div className="row" style={{ marginTop: 8 }}>{draft.data.min_draft_deg >= minDraft ? <StatusChip status="pass" /> : <StatusChip status="fail" />}<span className="small muted">R-020 asks for ≥ 1° on every molded wall. The CAD Studio's draft angle tapers extruded walls.</span></div>
              </Card>
            )}
            {view === 'thickness' && thick.data && (
              <Card title="Wall check">
                <div className="grid grid-2" style={{ gap: 8 }}>
                  <Stat label="Nominal" value={fmtNum(thick.data.nominal_mm)} unit="mm" />
                  <Stat label="95th percentile" value={fmtNum(thick.data.p95_mm)} unit="mm" />
                  <Stat label="Thick area" value={fmtNum(thick.data.thick_area_mm2)} unit="mm²" tone={thick.data.thick_area_mm2 > 0 ? 'warn' : 'pass'} />
                  <Stat label="Thin area" value={fmtNum(thick.data.thin_area_mm2)} unit="mm²" tone={thick.data.thin_area_mm2 > 0 ? 'warn' : 'pass'} />
                </div>
              </Card>
            )}
          </div>
        </div>
      </div>
      {an.data && pp && (
        <>
          <div className="grid grid-4">
            <Stat label="Clamp force (largest tool)" value={fmtNum(an.data.clamp_tonnes)} unit="t" note={`${fmtNum(pp.projected_area_mm2)} mm² × ${an.data.material.cavity_pressure_mpa} MPa × ${options.clamp_allowance}`} />
            <Stat label="Cycle (slowest part)" value={fmtNum(an.data.cycle_s)} unit="s" note={`cooling ${fmtNum(pp.cycle.cool_s)} s of it`} />
            <Stat label="Tooling, all parts" value={`$${fmtNum(an.data.tooling_total_usd)}`} note={`${an.data.steel.name} · indicative`} />
            <Stat label="Piece price" value={`$${fmtNum(an.data.parts.reduce((s, x) => s + x.piece.total_usd, 0))}`} note="material + press time, per set · indicative" />
          </div>
          <div className="grid grid-2">
            <Card title="Molding cycle" subtitle={`${pp.part}: timed by the engine's estimate`}>
              <CycleAnimation cycle={pp.cycle} />
            </Card>
            <Card title="Cost per frame set vs quantity" subtitle="Tooling amortised over the run (a new tool every shot-life) — indicative, get real quotes">
              {costCurve && (
                <Plot
                  series={[{ name: 'Unit cost (tooling + piece)', x: costCurve.qs, y: costCurve.unit }, { name: 'Piece price only', x: costCurve.qs, y: costCurve.piece, dash: '5 4' }]}
                  height={250} xLog yLog yDomain={costCurve.dom} xLabel="quantity" yLabel="USD per set"
                  vLines={[{ value: mfg.target_quantity, label: `target ${mfg.target_quantity}` }]}
                />
              )}
            </Card>
          </div>
          <Card title="Per-part DFM & process summary">
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Part</th><th>Pull</th><th>Draft</th><th>Undercut</th><th>Wall nom/max</th><th>Proj. area</th><th>Clamp</th><th>Cycle</th><th>Gate → flow</th><th>Tool</th><th>Piece</th></tr></thead>
                <tbody>
                  {an.data.parts.map((x) => (
                    <tr key={x.part}>
                      <td><b>{x.part}</b><div className="small muted">{fmtNum(x.mass_g)} g · scale tool ×{x.shrink_scale.toFixed(4)}</div></td>
                      <td className="mono">{x.pull_label}</td>
                      <td className="tnum" style={{ color: x.min_draft_deg >= minDraft ? 'var(--good-text)' : 'var(--bad-text)' }}>{x.min_draft_deg >= minDraft ? '✓' : '✕'} {fmtNum(x.min_draft_deg)}°</td>
                      <td className="tnum">{fmtNum(x.undercut_area_mm2)} mm²{x.side_actions ? ` · ${x.side_actions} side action(s)` : ''}</td>
                      <td className="tnum">{fmtNum(x.wall_nominal_mm)} / {fmtNum(x.wall_max_mm)} mm</td>
                      <td className="tnum">{fmtNum(x.projected_area_mm2)} mm²</td>
                      <td className="tnum">{fmtNum(x.clamp_tonnes)} t <span className="muted">(press ≥ {x.recommended_press_t} t)</span></td>
                      <td className="tnum">{fmtNum(x.cycle.total_s)} s</td>
                      <td className="tnum">{fmtNum(x.flow_length_mm)} mm · L/t {fmtNum(x.flow_ratio)} {x.flow_ratio > x.flow_ratio_limit ? '✕' : '✓'}</td>
                      <td className="tnum">${fmtNum(x.tool_cost_usd)}</td>
                      <td className="tnum">${fmtNum(x.piece.total_usd)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {an.data.parts.some((x) => x.warnings.length) && (
              <ul className="small" style={{ marginTop: 10, paddingLeft: 18 }}>
                {an.data.parts.flatMap((x) => x.warnings.map((w, i) => <li key={x.part + i}><b>{x.part}:</b> {w}</li>))}
              </ul>
            )}
          </Card>
        </>
      )}
      <div className="grid grid-2">
        <Explainer title="Cooling time — why wall thickness rules the cycle" refs={[REFS.osswald, REFS.rosato]}>
          Heat leaves the part by conduction into the mold. Solving the 1-D heat equation for a slab of thickness s cooled from both faces and keeping the first Fourier term gives the time until the mid-plane reaches the ejection temperature:
          <Formula>{'t_c = s²/(π²α) · ln[(8/π²)·(T_m − T_w)/(T_e − T_w)]\nT_m melt, T_w mold wall, T_e ejection temperature'}</Formula>
          {mat && <>
            <Slider label="Wall thickness s" value={wall} min={0.5} max={6} step={0.1} unit="mm" onChange={setWall} />
            <div>{mat.id}: t_c = <b className="mono">{fmtNum(coolingTime(wall, mat))} s</b> — double the wall and cooling takes <b>four</b> times longer. That is why the Iris front's thick rim dominates its cycle.</div>
          </>}
        </Explainer>
        <Explainer title="Clamp force and press size" refs={[REFS.rosato, REFS.menges]}>
          Melt pressure pushes the mold halves apart over the part's shadow on the parting plane. The press must hold them shut:
          <Formula>{'F = A_projected · p_cavity · (1 + runner allowance)\n1 tonne-force = 9.807 kN'}</Formula>
          Cavity pressures are typically 30–60 MPa for commodity resins, higher for stiff PC. Choose a press with ~25 % headroom.
        </Explainer>
        <Explainer title="Design rules the lab checks" refs={[REFS.protolabs, REFS.menges]}>
          <ul style={{ margin: 0, paddingLeft: 18 }}>
            <li><b>Draft</b> ≥ 1° on walls parallel to the pull (more on textured faces) so the part releases.</li>
            <li><b>Uniform walls</b>; core out thick sections — they sink, void and cool slowly.</li>
            <li><b>Ribs ≤ 60 %</b> of the wall they stand on; generous fillets (≥ 0.5 × wall) at corners.</li>
            <li><b>Undercuts</b> need side actions or lifters (tool cost) — or a redesign.</li>
            <li><b>Gate</b> into the thickest section so melt flows thick → thin; keep flow length / wall under the resin's limit.</li>
            <li><b>Shrinkage</b>: cut the cavity oversized by 1/(1 − s).</li>
          </ul>
        </Explainer>
        <Explainer title="How the analysis works" refs={[REFS.mt]}>
          Draft = 90° − ∠(n, pull) per face; faces within 45° of face-on are floors. A side wall is an <i>undercut</i> when a ray from it in its release direction hits the part again (uniform-grid accelerated Möller–Trumbore ray casting). The pull is chosen among the world and principal axes to maximise the minimum draft, then minimise undercuts and depth. Projected area is the rasterised union of the triangles' shadows. Graduate to Moldflow, SolidWorks Plastics or FreeCAD for real fill/pack/warp simulation.
        </Explainer>
      </div>
    </LabPage>
  );
}
