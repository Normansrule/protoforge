// 3-D conduction (FEA): the Iris right pod, optics housing and front voxelised
// from the real CAD, heated by the components where the PCB Studio places
// them, cooled by correlation-based convection + radiation; steady state by
// preconditioned conjugate gradients in a Web Worker.
import { useMemo, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat, Toggle } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { PresetButtons, type ViewPreset } from '../cad/viewer';
import { SliceView, type Axis } from './SliceView';
import { TempLegend, ThermalView, decodeSurfaces, type ViewOpts, type VolumeData } from './ThermalView';
import { TransientCard } from './TransientCard';
import { configOf, type PodControls } from './controls';
import type { ScenariosOut, StudyRow, SteadyOut, ThermalMaterial } from './types';
import { LOCATION_LABEL, b64Bytes, fluidsDesign, fmt, toCsv, useWorkerCompute, workerCall } from './util';

const VOXELS = [2.0, 1.5, 1.2, 1.0];

function tone(v: number, limit: number): 'pass' | 'fail' | 'warn' {
  return v <= limit - 1 ? 'pass' : v <= limit ? 'warn' : 'fail';
}
function word(v: number, limit: number) {
  const t = tone(v, limit);
  return t === 'pass' ? `✓ under the ${limit} °C skin limit` : t === 'warn' ? `! within 1 °C of ${limit} °C` : `✕ above the ${limit} °C skin limit`;
}

export function ConductionTab({ ctl, setCtl }: { ctl: PodControls; setCtl: (f: (c: PodControls) => PodControls) => void }) {
  const project = useProject((s) => s.project);
  const patchDesign = useProject((s) => s.patchDesign);
  const scen = useCompute<ScenariosOut>('fluids.scenarios', {}).data;
  const mats = useCompute<{ shell: ThermalMaterial[] }>('fluids.materials', {}).data;
  const input = useMemo(() => ({ project, config: configOf(ctl), surfaces: true, slices: true }), [project, ctl]);
  const res = useWorkerCompute<SteadyOut>('fluids.pod_steady', input);
  const r = res.data;
  const set = (p: Partial<PodControls>) => setCtl((c) => ({ ...c, ...p }));
  const [opts, setOpts] = useState<ViewOpts>({ xrayLid: false, xrayShell: false, inner: true, markers: true, wire: false });
  const [preset, setPreset] = useState<ViewPreset>('iso');
  const [nonce, setNonce] = useState(0);
  const [axis, setAxis] = useState<Axis>('x');
  const [sliceIdx, setSliceIdx] = useState<number | null>(null);
  const [showMat, setShowMat] = useState(false);
  const [range, setRange] = useState<'auto' | 'limit'>('auto');
  const [study, setStudy] = useState<{ rows: StudyRow[] | null; busy: boolean; error: string | null }>({ rows: null, busy: false, error: null });
  const surfaces = useMemo(() => (r ? decodeSurfaces(r.surfaces) : []), [r]);
  const inner = useMemo(() => (r ? decodeSurfaces(r.inner) : []), [r]);
  const volume = useMemo<VolumeData | null>(() => (r?.volume ? { ...r.volume, bytes: b64Bytes(r.volume.data) } : null), [r]);
  const limit = project.design.thermal.skin_limit_c;
  const lo = r ? r.ambient_c : 30;
  const hiAuto = r ? Math.max(r.peak_skin_c, r.board_max_c, r.display_max_c, lo + 0.5) : 40;
  const hi = range === 'limit' ? Math.max(lo + 0.5, limit) : hiAuto;
  const s = r?.slices;
  const dims = s ? { x: s.nx, y: s.ny, z: s.nz } : { x: 1, y: 1, z: 1 };
  // default slice: through the board (x) / the hotspot
  const defaultIdx = useMemo(() => {
    if (!r || !s) return 0;
    const p = axis === 'x' ? (r.pcb_box.lo[0] + r.pcb_box.hi[0]) / 2 : r.peak_pos[axis === 'y' ? 1 : 2];
    const d = axis === 'x' ? 0 : axis === 'y' ? 1 : 2;
    return Math.round((p - s.origin[d]) / s.h_mm - 0.5);
  }, [r, s, axis]);
  const idx = sliceIdx ?? defaultIdx;
  const design = fluidsDesign(project);
  const dirty =
    design.spreader !== ctl.spreader ||
    design.spreader_um !== ctl.spreader_um ||
    design.spreader_location !== ctl.spreader_location ||
    design.display_heat_in !== ctl.display_heat_in ||
    design.air_speed_m_s !== ctl.air_speed_m_s ||
    (design.emissivity ?? null) !== ctl.emissivity;
  const runStudy = () => {
    setStudy({ rows: null, busy: true, error: null });
    workerCall<StudyRow[]>('fluids.mesh_study', { project, config: configOf(ctl), sizes: VOXELS })
      .then(({ data }) => setStudy({ rows: data, busy: false, error: null }))
      .catch((e: Error) => setStudy({ rows: null, busy: false, error: e.message }));
  };
  const csvSources = () =>
    toCsv([['refdes', 'name', 'location', 'heat_mw', 'placed', 'x_mm', 'y_mm', 'z_mm'], ...(r?.sources ?? []).map((x) => [x.refdes, x.name, x.location, +x.mw.toFixed(4), x.placed, +x.pos[0].toFixed(2), +x.pos[1].toFixed(2), +x.pos[2].toFixed(2)])]);
  const vtk = () => {
    if (!s) return '';
    const t = b64Bytes(s.temp_u8);
    const m = b64Bytes(s.mat_u8);
    const n = s.nx * s.ny * s.nz;
    const lines = [
      '# vtk DataFile Version 3.0',
      `Protoforge Iris pod temperature (cell data, quantised to 1/255 of ${s.t_lo.toFixed(2)}-${s.t_hi.toFixed(2)} C)`,
      'ASCII',
      'DATASET STRUCTURED_POINTS',
      `DIMENSIONS ${s.nx + 1} ${s.ny + 1} ${s.nz + 1}`,
      `ORIGIN ${s.origin.map((v) => v.toFixed(4)).join(' ')}`,
      `SPACING ${s.h_mm} ${s.h_mm} ${s.h_mm}`,
      `CELL_DATA ${n}`,
      'SCALARS temperature_C float 1',
      'LOOKUP_TABLE default',
    ];
    const tv: string[] = [];
    for (let i = 0; i < n; i++) tv.push(m[i] === 0 ? 'nan' : (s.t_lo + (t[i] / 255) * (s.t_hi - s.t_lo)).toFixed(2));
    lines.push(tv.join('\n'));
    lines.push('SCALARS material int 1', 'LOOKUP_TABLE default');
    lines.push(Array.from(m).join('\n'));
    return lines.join('\n') + '\n';
  };
  const summary = () => {
    if (!r) return '';
    const { surfaces: _a, inner: _b, slices: _c, voxel_surface: _d, ...rest } = r;
    return JSON.stringify(rest, null, 2);
  };
  const maxRes = r ? Math.max(...r.residuals) : 1;
  return (
    <div className="stack">
      <div className="fl-main">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card
            title="Iris right side — steady temperature"
            subtitle={
              r
                ? `${r.elements.toLocaleString()} voxels of ${r.voxel_mm} mm · ${r.dofs.toLocaleString()} unknowns · ${r.precond.toUpperCase()}-preconditioned conjugate gradient, ${r.iterations_total} iterations over 3 convection updates · ${(res.ms / 1000).toFixed(2)} s in WebAssembly`
                : res.busy
                  ? 'voxelising the CAD and solving…'
                  : '—'
            }
            actions={
              <>
                {res.busy && <span className="cp-spin" aria-label="computing" />}
                <PresetButtons value={preset} onChange={(p) => { setPreset(p); setNonce((n) => n + 1); }} />
              </>
            }
          >
            <ErrorBox error={res.error} />
            {r && volume ? (
              <ThermalView
                surfaces={surfaces}
                inner={inner}
                volume={volume}
                lo={lo}
                hi={hi}
                bbox={{ min: r.bbox_min, max: r.bbox_max }}
                opts={opts}
                peak={r.peak_pos}
                sources={r.sources.filter((x) => x.placed !== 'not modelled (battery pod)').map((x) => ({ pos: x.pos, mw: x.mw }))}
                height="min(60vh, 540px)"
                preset={preset}
                nonce={nonce}
                overlay={<TempLegend lo={lo} hi={hi} limit={range === 'limit' ? undefined : limit} title={range === 'limit' ? `Temperature, °C (darkest = ${limit} °C or more)` : undefined} />}
              />
            ) : (
              <div className="scene3d" style={{ height: 'min(60vh, 540px)', display: 'grid', placeItems: 'center' }}>
                <span className="muted small">{res.busy ? 'Solving the first model…' : 'Waiting for the engine…'}</span>
              </div>
            )}
            <div className="cp-toolbar">
              <div className="cp-seg" role="group" aria-label="Colour range">
                {(['auto', 'limit'] as const).map((m) => (
                  <button key={m} className={`cp-seg-btn ${range === m ? 'active' : ''}`} onClick={() => setRange(m)}>{m === 'auto' ? 'full range' : `clip at ${limit} °C`}</button>
                ))}
              </div>
              <label className="cp-check"><input type="checkbox" checked={opts.xrayLid} onChange={(e) => setOpts({ ...opts, xrayLid: e.target.checked })} /> x-ray lid</label>
              <label className="cp-check"><input type="checkbox" checked={opts.xrayShell} onChange={(e) => setOpts({ ...opts, xrayShell: e.target.checked })} /> x-ray frame</label>
              <label className="cp-check"><input type="checkbox" checked={opts.inner} onChange={(e) => setOpts({ ...opts, inner: e.target.checked })} /> board, display, spreader</label>
              <label className="cp-check"><input type="checkbox" checked={opts.markers} onChange={(e) => setOpts({ ...opts, markers: e.target.checked })} /> heat sources + hotspot</label>
              <label className="cp-check"><input type="checkbox" checked={opts.wire} onChange={(e) => setOpts({ ...opts, wire: e.target.checked })} /> wireframe</label>
            </div>
            <p className="small muted" style={{ marginBottom: 0 }}>
              Blue dots are heat sources (size ∝ √power); the pulsing ring marks the hottest frame surface. Tick “x-ray” to see the board inside the pod.
            </p>
          </Card>
          {r && (
            <div className="grid grid-4">
              <Stat label="Peak frame surface" value={fmt(r.peak_skin_c, 1)} unit="°C" tone={tone(r.peak_skin_c, limit)} note={`${word(r.peak_skin_c, limit)} · on the ${r.peak_part.toLowerCase()}`} />
              <Stat label="Mean frame surface" value={fmt(r.mean_skin_c, 1)} unit="°C" note={`area-weighted over ${(r.lumped.area_mm2 / 100).toFixed(0)} cm²`} />
              <Stat label="Hotspot above mean" value={fmt(r.hotspot_over_mean_c, 1)} unit="°C" note="what a single-node model cannot see" />
              <Stat label="Lumped model (Power Lab)" value={fmt(r.lumped.shell_c, 1)} unit="°C" note={`same ${r.pod_heat_mw.toFixed(0)} mW, one shell node, R = ${r.lumped.r_pod_ambient} °C/W`} />
            </div>
          )}
        </div>
        <div className="stack">
          <Card title="Scenario & design">
            <Select
              label="Usage scenario (Power Lab)"
              value={ctl.scenario}
              options={(scen?.steady ?? [{ id: 'display_on', name: 'Display always on (R-008 case)', description: '' }]).map((x) => ({ value: x.id, label: x.name }))}
              onChange={(v) => set({ scenario: v })}
            />
            <Select
              label="Display heat goes into"
              value={ctl.display_heat_in}
              options={[
                { value: 'optics', label: 'the optics housing (where the BOM puts it)' },
                { value: 'pod', label: 'the pod board (as the lumped model assumes)' },
              ]}
              onChange={(v) => set({ display_heat_in: v })}
            />
            <Select
              label="Heat spreader"
              value={ctl.spreader}
              options={[
                { value: 'none', label: 'none' },
                { value: 'graphite', label: 'synthetic graphite sheet' },
                { value: 'copper', label: 'copper foil' },
                { value: 'aluminium', label: 'aluminium foil / plate' },
              ]}
              onChange={(v) => set({ spreader: v })}
            />
            {ctl.spreader !== 'none' && (
              <>
                <Select
                  label="Spreader location"
                  value={ctl.spreader_location}
                  options={Object.entries(LOCATION_LABEL).map(([value, label]) => ({ value, label }))}
                  onChange={(v) => set({ spreader_location: v })}
                />
                <Slider label="Spreader thickness" value={ctl.spreader_um} min={10} max={1000} log unit="µm" format={(v) => v.toFixed(0)} onChange={(v) => set({ spreader_um: v })} />
              </>
            )}
            <Select
              label="Frame material (thermal)"
              value={ctl.shell_material}
              options={(mats?.shell ?? []).map((m) => ({ value: m.id, label: `${m.name} · k ${m.k_in} W/m K` })).concat(mats?.shell.some((m) => m.id.toLowerCase() === ctl.shell_material.toLowerCase()) ? [] : [{ value: ctl.shell_material, label: `${ctl.shell_material} (project)` }])}
              onChange={(v) => set({ shell_material: v })}
            />
            <Slider label="Surface emissivity ε" value={ctl.emissivity ?? r?.config.emissivity ?? 0.9} min={0.05} max={1} step={0.01} onChange={(v) => set({ emissivity: v })} hint="0.9 for most plastics and paints, 0.05 for bare polished metal" />
            <Slider label="Air speed (walking ≈ 1.4)" value={ctl.air_speed_m_s} min={0} max={4} step={0.1} unit="m/s" onChange={(v) => set({ air_speed_m_s: v })} />
            <div className="field">
              <span className="field-top"><span className="field-label">Voxel size</span></span>
              <div className="cp-seg" role="group" aria-label="Voxel size">
                {VOXELS.map((v) => (
                  <button key={v} className={`cp-seg-btn ${ctl.voxel_mm === v ? 'active' : ''}`} style={{ textTransform: 'none' }} onClick={() => set({ voxel_mm: v })}>{v} mm</button>
                ))}
              </div>
            </div>
            <div className="field">
              <span className="field-top"><span className="field-label">Preconditioner</span></span>
              <div className="cp-seg" role="group" aria-label="Preconditioner">
                {['ssor', 'jacobi'].map((v) => (
                  <button key={v} className={`cp-seg-btn ${ctl.precond === v ? 'active' : ''}`} onClick={() => set({ precond: v })}>{v === 'ssor' ? 'SSOR' : 'Jacobi'}</button>
                ))}
              </div>
            </div>
            <div className="row">
              <button
                className="btn primary small"
                disabled={!dirty}
                onClick={() =>
                  patchDesign('extra', {
                    fluids: {
                      spreader: ctl.spreader,
                      spreader_um: ctl.spreader_um,
                      spreader_location: ctl.spreader_location,
                      display_heat_in: ctl.display_heat_in,
                      emissivity: ctl.emissivity,
                      air_speed_m_s: ctl.air_speed_m_s,
                    },
                  })
                }
                title="Store spreader, display-heat, emissivity and air speed at design.extra.fluids — the requirement metrics use them"
              >
                {dirty ? 'Use these choices for Iris' : '✓ Iris uses these choices'}
              </button>
            </div>
            <p className="small muted" style={{ marginBottom: 0 }}>
              Metrics always use the display-on (R-008) case on 1.5 mm voxels; the voxel size, scenario and material here are what-ifs.
            </p>
          </Card>
          {r && (
            <Card title="Where the heat is">
              <table className="table">
                <thead>
                  <tr><th>Part</th><th>Peak</th><th>Mean</th></tr>
                </thead>
                <tbody>
                  {r.parts.map((p) => (
                    <tr key={p.part}><td>{p.part}</td><td className="tnum">{fmt(p.peak_c, 1)} °C</td><td className="tnum">{fmt(p.mean_c, 1)} °C</td></tr>
                  ))}
                  <tr><td>Printed circuit board (max)</td><td className="tnum">{fmt(r.board_max_c, 1)} °C</td><td /></tr>
                  <tr><td>Micro-display (max)</td><td className="tnum">{fmt(r.display_max_c, 1)} °C</td><td /></tr>
                </tbody>
              </table>
              <p className="small muted" style={{ marginBottom: 0 }}>
                Heat in {fmt(r.total_mw, 1)} mW = heat out {fmt(r.heat_out_mw, 1)} mW (energy balance check).
              </p>
            </Card>
          )}
        </div>
      </div>

      {r && (
        <Card title="Hotspot vs the lumped model" subtitle="Same heat, two models — why the numbers differ">
          <div className="grid grid-2">
            <div className="fl-bars" role="img" aria-label="comparison of lumped and 3-D temperatures">
              {[
                { label: 'Lumped shell (one node)', v: r.lumped.shell_c, c: 'var(--series-7)' },
                { label: '3-D mean frame surface', v: r.mean_skin_c, c: 'var(--series-1)' },
                { label: '3-D pod shell peak', v: Math.max(...r.parts.filter((p) => p.part.startsWith('Pod')).map((p) => p.peak_c), r.ambient_c), c: 'var(--series-3)' },
                { label: '3-D hottest surface', v: r.peak_skin_c, c: 'var(--series-2)' },
              ].map((b) => {
                const top = Math.max(r.peak_skin_c, limit, r.lumped.shell_c) + 2;
                const w = ((b.v - r.ambient_c) / (top - r.ambient_c)) * 100;
                const lw = ((limit - r.ambient_c) / (top - r.ambient_c)) * 100;
                return (
                  <div key={b.label} className="fl-bar-row">
                    <span className="small">{b.label}</span>
                    <div className="fl-bar-track">
                      <div className="fl-bar" style={{ width: `${Math.max(1, w)}%`, background: b.c }} />
                      <div className="fl-bar-limit" style={{ left: `${lw}%` }} />
                    </div>
                    <span className="small tnum">{b.v.toFixed(1)} °C</span>
                  </div>
                );
              })}
              <div className="small muted">bars start at the {r.ambient_c} °C ambient; the dashed line is the {limit} °C skin limit</div>
            </div>
            <ul className="small" style={{ margin: 0, paddingLeft: 18 }}>
              {r.lumped.explanation.map((e, i) => <li key={i} style={{ marginBottom: 6 }}>{e}</li>)}
            </ul>
          </div>
          <Explainer
            title="Lumped versus spatially resolved"
            refs={[{ label: 'Incropera et al., Fundamentals of Heat and Mass Transfer, 7th ed., §3.6 (fins), §5.2 (lumped capacitance)' }]}
          >
            <p>
              A lumped resistor–capacitor network is valid when a body is nearly isothermal — when the Biot number <i>Bi = h L / k</i> is small. For a plastic shell (k ≈ 0.2 W/m K) with h ≈ 10 W/m² K over
              L ≈ 30 mm, Bi ≈ 1.5: the shell is <b>not</b> isothermal, so heat leaves mostly near where it enters. The distance over which a wall of thickness t spreads heat is the fin length
            </p>
            <Formula>λ = √(k t / h) = √({r.materials[0]?.k_in ?? 0.2} × {project.design.frame.wall_mm} mm ÷ {fmt(r.lumped.h_mean, 1)} W/m² K) ≈ {fmt(r.lumped.spread_len_mm, 1)} mm</Formula>
            <p>
              Anything further than a few λ from the source barely warms up. A spreader with conductance k<sub>s</sub>t<sub>s</sub> ≫ k t stretches λ — that is the whole trade in the next tab.
            </p>
          </Explainer>
        </Card>
      )}

      <TransientCard ctl={ctl} />

      {s && (
        <Card
          title="Cut slices"
          subtitle="One plane of voxel elements; outlines are material boundaries. Hover for the temperature."
          actions={
            <div className="cp-seg" role="group" aria-label="Slice axis">
              {(['x', 'y', 'z'] as Axis[]).map((a) => (
                <button key={a} className={`cp-seg-btn ${axis === a ? 'active' : ''}`} onClick={() => { setAxis(a); setSliceIdx(null); }}>
                  {a === 'x' ? 'side (x)' : a === 'y' ? 'cross-section (y)' : 'plan (z)'}
                </button>
              ))}
            </div>
          }
        >
          <div className="cp-toolbar" style={{ marginTop: 0, marginBottom: 10 }}>
            <Slider label="Slice position" value={idx} min={0} max={dims[axis] - 1} step={1} format={(v) => `${(s.origin[axis === 'x' ? 0 : axis === 'y' ? 1 : 2] + (v + 0.5) * s.h_mm).toFixed(1)} mm`} onChange={(v) => setSliceIdx(Math.round(v))} />
            <Toggle label="show materials instead of temperature" value={showMat} onChange={setShowMat} />
          </div>
          <SliceView s={s} axis={axis} index={idx} showMaterials={showMat} />
        </Card>
      )}

      {r && (
        <div className="grid grid-2">
          <Card title="Boundary: convection + radiation" subtitle="Coefficients updated from the solved surface temperature (Picard iteration)">
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr><th>Surface</th><th>Area</th><th>ΔT</th><th>Ra</th><th>h conv</th><th>h rad</th><th>Heat out</th></tr>
                </thead>
                <tbody>
                  {r.groups.filter((g) => g.area_mm2 > 0).map((g, i) => (
                    <tr key={i}>
                      <td>{g.body}, {g.class === 'vertical' ? 'side' : g.class === 'up' ? 'top' : 'bottom'} faces</td>
                      <td className="tnum">{(g.area_mm2 / 100).toFixed(1)} cm²</td>
                      <td className="tnum">{fmt(g.theta_mean, 2)} K</td>
                      <td className="tnum">{g.ra < 1 ? g.ra.toFixed(2) : g.ra.toFixed(0)}{!g.valid && <span title="outside the correlation's validated range — extrapolated"> !</span>}</td>
                      <td className="tnum">{fmt(g.h_conv, 2)}</td>
                      <td className="tnum">{fmt(g.h_rad, 2)}</td>
                      <td className="tnum">{fmt(g.q_mw, 1)} mW</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <p className="small muted">
              h in W/m² K. “!” marks a Rayleigh number below the horizontal-plate correlations' validated range (10⁴) — extrapolated; small wearables live there. Radiation (ε = {r.config.emissivity}) carries about as much heat as natural convection.
            </p>
            <Explainer title="Inside the pod: why no convection" refs={[{ label: 'Incropera et al. §9.8 (enclosures: onset at Ra ≈ 1708)' }]}>
              The sealed cavity is modelled as conducting air (k = {fmt(r.k_cavity, 4)} W/m K) plus linearised radiation between board and walls (k<sub>rad</sub> = 4σε<sub>eff</sub>T³·gap = {fmt(r.k_cavity_rad, 4)} W/m K). Natural convection needs a Rayleigh number above about 1708 in an enclosure; across the ≈ 2 mm board-to-wall gap it is only Ra ≈ {fmt(r.cavity_rayleigh, 2)}.
            </Explainer>
          </Card>
          <Card title="Heat sources" subtitle={`From the Power Lab budget for “${scen?.steady.find((x) => x.id === ctl.scenario)?.name ?? ctl.scenario}”, placed where the PCB Studio puts each part`}>
            <div className="table-wrap" style={{ maxHeight: 360, overflowY: 'auto' }}>
              <table className="table">
                <thead><tr><th>Ref</th><th>Part</th><th>Heat</th><th>Where</th></tr></thead>
                <tbody>
                  {[...r.sources].sort((a, b) => b.mw - a.mw).map((x) => (
                    <tr key={x.refdes + x.name}><td className="mono">{x.refdes}</td><td>{x.name}</td><td className="tnum">{x.mw < 0.1 ? x.mw.toFixed(3) : x.mw.toFixed(1)} mW</td><td className="small">{x.placed}</td></tr>
                  ))}
                </tbody>
              </table>
            </div>
            <p className="small muted" style={{ marginBottom: 0 }}>
              Total {fmt(r.total_mw, 1)} mW — exactly the lumped model's pod heat ({fmt(r.pod_heat_mw, 1)} mW), so the two models compare like for like.
            </p>
          </Card>
        </div>
      )}

      {r && (
        <div className="grid grid-2">
          <Card title="Solver convergence" subtitle={`${r.precond === 'ssor' ? 'Symmetric successive over-relaxation (SSOR)' : 'Jacobi'} preconditioner · ${r.nnz.toLocaleString()} non-zeros`}>
            <Plot
              series={[{ name: 'relative residual ‖b − Kθ‖/‖b‖ (last pass)', x: r.residuals.map((_, i) => i), y: r.residuals.map((v) => Math.max(v, 1e-14)) }]}
              yLog
              xLabel="conjugate-gradient iteration"
              yLabel="residual"
              height={220}
              yDomain={[Math.min(1e-9, ...r.residuals.filter((v) => v > 0)), Math.max(1, maxRes)]}
              hLines={[{ value: r.config.tol, label: 'tolerance' }]}
            />
            <div className="row" style={{ marginTop: 8 }}>
              <button className="btn small" onClick={runStudy} disabled={study.busy}>{study.busy ? 'Solving 4 meshes…' : 'Run a mesh-convergence study'}</button>
              {r.converged ? <Chip tone="pass">✓ converged</Chip> : <Chip tone="warn">! not converged</Chip>}
            </div>
            <ErrorBox error={study.error} />
            {study.rows && (
              <div className="table-wrap" style={{ marginTop: 8 }}>
                <table className="table">
                  <thead><tr><th>Voxel</th><th>Elements</th><th>Peak</th><th>Mean</th><th>Board</th><th>CG its</th></tr></thead>
                  <tbody>
                    {study.rows.map((x) => (
                      <tr key={x.voxel_mm}><td>{x.voxel_mm} mm</td><td className="tnum">{x.elements.toLocaleString()}</td><td className="tnum">{fmt(x.peak_skin_c, 2)} °C</td><td className="tnum">{fmt(x.mean_skin_c, 2)} °C</td><td className="tnum">{fmt(x.board_max_c, 2)} °C</td><td className="tnum">{x.iterations}</td></tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </Card>
          <Card title="Model & downloads">
            <ul className="small" style={{ margin: '0 0 10px', paddingLeft: 18 }}>
              {r.materials.map((m) => (
                <li key={m.id}>{m.name}: k {m.k_in === m.k_through ? fmt(m.k_in, 3) : `${fmt(m.k_in, 1)} along / ${fmt(m.k_through, 2)} across`} W/m K, ρc {(m.rho * m.cp / 1e6).toFixed(2)} MJ/m³ K <span className="muted">— {m.note} (typical)</span></li>
              ))}
              <li>Cavity air voxels: {r.cavity_cells.toLocaleString()} (≈ {Math.round(r.cavity_expected_cells).toLocaleString()} expected from the CAD cavity)</li>
            </ul>
            {r.warnings.length > 0 && (
              <details className="small" style={{ marginBottom: 10 }}>
                <summary>{r.warnings.length} model notes</summary>
                <ul style={{ paddingLeft: 18 }}>{r.warnings.map((w, i) => <li key={i}>{w}</li>)}</ul>
              </details>
            )}
            <div className="row">
              <DownloadButton filename="iris-pod-temperature.vtk" content={vtk} label="ParaView VTK (temperature + material)" small />
              <DownloadButton filename="iris-heat-sources.csv" content={csvSources} label="heat sources CSV" small />
              <DownloadButton filename="iris-thermal-summary.json" content={summary} label="summary JSON" small />
            </div>
          </Card>
        </div>
      )}

      <Explainer
        title="The finite element model in one screen"
        refs={[
          { label: 'Patankar, Numerical Heat Transfer and Fluid Flow (1980)' },
          { label: 'Incropera et al. (2011)' },
          { label: 'Elmer FEM (open-source multiphysics)', href: 'https://github.com/ElmerCSC/elmerfem' },
        ]}
      >
        <p>
          The CAD parts are voxelised on a regular grid (the Structures Lab's ray-casting voxeliser); every voxel becomes an 8-node brick with trilinear temperature. The weak form of
          −∇·(k∇T) = q with a convective boundary gives a symmetric linear system:
        </p>
        <Formula>(K + H) θ = Q,  K<sub>e</sub> = Σ<sub>d</sub> k<sub>d</sub> L · K̂<sub>d</sub>,  H = Σ<sub>faces</sub> (h<sub>conv</sub> + h<sub>rad</sub>) A / 4,  θ = T − T<sub>∞</sub></Formula>
        <p>
          K̂<sub>d</sub>[a][b] = ∫ ∂N<sub>a</sub>/∂ξ<sub>d</sub> ∂N<sub>b</sub>/∂ξ<sub>d</sub> factorises into ±1 along d times 1/3 or 1/6 along the other two axes. Partially filled voxels mix materials: along a sheet in
          parallel (k = Σ f<sub>i</sub>k<sub>i</sub>), across it in series — a 25 µm graphite sheet in a 1.5 mm voxel keeps its true conductance k·t. Natural convection h depends on the surface temperature, so
          the solve is repeated three times with updated h (Picard iteration).
        </p>
      </Explainer>
    </div>
  );
}
