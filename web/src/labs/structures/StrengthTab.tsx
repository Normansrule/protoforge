// Strength (3D FEA): the real Iris CAD parts under named load cases, solved by
// the voxel finite element engine in a Web Worker.
import { useMemo, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Select, Slider, Stat, StatusChip } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { PresetButtons, type ViewPreset } from '../cad/viewer';
import { FeaView, type ColorMode, type ViewOpts, autoScale } from './FeaView';
import type { CaseInfo, FeaResult, IrisOut, StructMaterial, StudyRow } from './types';
import { CRITICAL, HEAT_GRADIENT, PART_LABEL, fmt, prefersReducedMotion, sfTone, sfWord, toCsv, useWorkerCompute, workerCall } from './util';
import { DropCard } from './DropCard';

const VOXELS = [2.0, 1.33, 1.0, 0.8];
const FINE_VOXEL = 0.67;

type IrisState = { data: IrisOut | null; busy: boolean; error: string | null; ms: number };

export function StrengthTab({ iris, fidelity, setFidelity }: { iris: IrisState; fidelity: 'metric' | 'fine'; setFidelity: (f: 'metric' | 'fine') => void }) {
  const project = useProject((s) => s.project);
  const patchDesign = useProject((s) => s.patchDesign);
  const mats = useCompute<StructMaterial[]>('structures.materials', {}).data ?? [];
  const cases = useCompute<CaseInfo[]>('structures.cases', {}).data ?? [];
  const [caseId, setCaseId] = useState('temple_spread');
  const [part, setPart] = useState('temple_r');
  const [material, setMaterial] = useState<string>('');
  const [voxel, setVoxel] = useState(1.33);
  const [force, setForce] = useState<number | null>(null);
  const [precond, setPrecond] = useState<'multigrid' | 'jacobi'>('multigrid');
  const [element, setElement] = useState<'h8i' | 'h8'>('h8i');
  const [fine, setFine] = useState(false);
  const info = cases.find((c) => c.id === caseId);
  const parts = info?.parts ?? ['temple_r'];
  const effPart = parts.includes(part) ? part : parts[0];
  const matId = material || project.design.frame.material;
  const input = useMemo(
    () => ({
      project,
      part: effPart,
      case: caseId,
      material: matId,
      voxel_mm: fine ? FINE_VOXEL : voxel,
      max_elements: fine ? 30000 : 12000,
      force_n: force ?? info?.default_force_n ?? 5,
      precond,
      element,
      tol: 1e-6,
      max_iter: precond === 'jacobi' ? 6000 : 600,
      surface: 'both',
    }),
    [project, effPart, caseId, matId, voxel, fine, force, info, precond, element],
  );
  const res = useWorkerCompute<FeaResult>('structures.fea', cases.length ? input : null);
  const r = res.data;
  const [opts, setOpts] = useState<ViewOpts>({
    surface: 'cad',
    mode: 'vm',
    exaggeration: 1,
    playing: !prefersReducedMotion(),
    showLoads: true,
    showFixed: true,
    showCrit: true,
    wire: false,
  });
  const [preset, setPreset] = useState<ViewPreset>('iso');
  const [nonce, setNonce] = useState(0);
  const set = (p: Partial<ViewOpts>) => setOpts((o) => ({ ...o, ...p }));
  const mat = mats.find((m) => m.id.toLowerCase() === matId.toLowerCase());
  const projMat = mats.find((m) => m.id.toLowerCase() === project.design.frame.material.toLowerCase());
  const sf = r?.safety_factor ?? NaN;
  return (
    <div className="stack">
      <div className="st-fea">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card
            title={`${PART_LABEL[effPart] ?? effPart} — ${info?.label ?? caseId}`}
            subtitle={
              r
                ? `${r.elements.toLocaleString()} voxels of ${fmt(r.h_mm, 2)} mm · ${r.free_dofs.toLocaleString()} unknowns · ${r.precond === 'multigrid' ? `multigrid (${r.levels} levels)` : 'Jacobi'} PCG, ${r.iterations} iterations · ${(res.ms / 1000).toFixed(2)} s in WebAssembly`
                : res.busy
                  ? 'voxelising and solving…'
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
            <div style={{ position: 'relative' }}>
              {r ? (
                <FeaView r={r} opts={opts} height="min(62vh, 560px)" nonce={nonce} preset={preset as 'iso'} />
              ) : (
                <div className="scene3d" style={{ height: 'min(62vh, 560px)', display: 'grid', placeItems: 'center' }}>
                  <span className="muted small">{res.busy ? 'Solving the first model…' : 'Waiting for the engine…'}</span>
                </div>
              )}
              {r && <Legend r={r} mode={opts.mode} />}
            </div>
            <div className="cp-toolbar">
              <button className="btn small" onClick={() => set({ playing: !opts.playing })} aria-pressed={opts.playing}>
                {opts.playing ? '❚❚ Pause' : '▶ Animate load'}
              </button>
              <Slider
                label="Deformation exaggeration"
                value={opts.exaggeration}
                min={0}
                max={4}
                step={0.05}
                format={(v) => (r ? `×${fmt(v * autoScale(r), 1)}` : `${v.toFixed(2)}`)}
                onChange={(v) => set({ exaggeration: v })}
              />
              <div className="cp-seg" role="group" aria-label="Colour by">
                {(['vm', 'util'] as ColorMode[]).map((m) => (
                  <button key={m} className={`cp-seg-btn ${opts.mode === m ? 'active' : ''}`} onClick={() => set({ mode: m })}>
                    {m === 'vm' ? 'von Mises' : 'utilisation'}
                  </button>
                ))}
              </div>
              <div className="cp-seg" role="group" aria-label="Surface">
                {(['cad', 'voxels'] as const).map((m) => (
                  <button key={m} className={`cp-seg-btn ${opts.surface === m ? 'active' : ''}`} onClick={() => set({ surface: m })}>
                    {m === 'cad' ? 'CAD surface' : 'voxels'}
                  </button>
                ))}
              </div>
              <label className="cp-check"><input type="checkbox" checked={opts.showLoads} onChange={(e) => set({ showLoads: e.target.checked })} /> loads</label>
              <label className="cp-check"><input type="checkbox" checked={opts.showFixed} onChange={(e) => set({ showFixed: e.target.checked })} /> supports</label>
              <label className="cp-check"><input type="checkbox" checked={opts.showCrit} onChange={(e) => set({ showCrit: e.target.checked })} /> critical point</label>
              <label className="cp-check"><input type="checkbox" checked={opts.wire} onChange={(e) => set({ wire: e.target.checked })} /> wireframe</label>
            </div>
          </Card>
          {r && (
            <div className="grid grid-4">
              <Stat label="Design stress (von Mises)" value={fmt(r.max_vm_mpa, 1)} unit="MPa" note={`sharp-corner peak ${fmt(r.peak_vm_mpa, 1)} MPa (×${fmt(r.peak_vm_mpa / Math.max(1e-9, r.max_vm_mpa), 2)})`} />
              <Stat
                label="Safety factor vs yield"
                value={fmt(sf, 2)}
                unit="×"
                tone={sfTone(sf)}
                note={`${sfTone(sf) === 'pass' ? '✓' : sfTone(sf) === 'warn' ? '!' : '✕'} ${r.governing}${r.printed ? `, Z × ${r.z_factor}` : ''} · isotropic ${fmt(r.sf_von_mises, 2)}×`}
              />
              <Stat label="Load-point deflection" value={fmt(r.load_disp_mm, 2)} unit="mm" note={`max anywhere ${fmt(r.max_disp_mm, 2)} mm`} />
              <Stat label="Stiffness" value={fmt(r.stiffness_n_mm, 2)} unit="N/mm" note={`${fmt(r.force_n, 1)} N ÷ deflection`} />
            </div>
          )}
          {r && (
            <div className="row" style={{ gap: 8 }}>
              <Chip tone={sfTone(sf)} title="Recommended requirement: minimum safety factor ≥ 2">{sfWord(sf)} · SF {fmt(sf, 2)}×</Chip>
              <Chip tone={sfTone(r.yield_mpa / Math.max(1e-9, r.peak_vm_mpa), 1)}>
                {r.peak_vm_mpa < r.yield_mpa ? '✓ corner peak below yield' : '! corner peak above yield — fillet it'}
              </Chip>
              <Chip>{r.converged ? `✓ solver converged (${r.iterations} it)` : '! not converged — raise the iteration limit'}</Chip>
            </div>
          )}
          {r && (
            <Card title={sf >= 2 ? 'Where is it working hardest?' : 'Why is it failing here?'} subtitle={`Critical point: ${r.hint.region} — the pulsing marker in the view`}>
              <div className="stack" style={{ gap: 8 }}>
                <div className="small">{r.hint.mechanism}</div>
                <div className="callout small">{r.hint.advice}</div>
              </div>
            </Card>
          )}
        </div>
        <div className="stack cp-sticky">
          <Card title="Load case">
            <Select
              label="Case"
              value={caseId}
              options={cases.map((c) => ({ value: c.id, label: c.label }))}
              onChange={(v) => {
                setCaseId(v);
                setForce(null);
              }}
            />
            {parts.length > 1 && (
              <Select label="Part" value={effPart} options={parts.map((p) => ({ value: p, label: PART_LABEL[p] ?? p }))} onChange={setPart} />
            )}
            <div className="small muted" style={{ marginBottom: 10 }}>{info?.description}</div>
            <Slider
              label={caseId === 'drop_side' ? 'Unit load (scaled by the drop model)' : caseId === 'pod_squeeze' ? 'Pinch force (each side)' : 'Force'}
              value={force ?? info?.default_force_n ?? 5}
              min={0.5}
              max={caseId === 'drop_side' ? 5 : 40}
              step={0.5}
              unit="N"
              onChange={setForce}
            />
          </Card>
          <Card title="Material & mesh">
            <Select
              label="Material"
              value={mat?.id ?? matId}
              options={mats.map((m) => ({ value: m.id, label: m.name }))}
              onChange={(v) => setMaterial(v)}
            />
            {mat && (
              <div className="small muted" style={{ marginBottom: 10 }}>
                E {fmt(mat.e_gpa, 2)} GPa · yield {fmt(mat.yield_mpa, 0)} MPa · {mat.density_g_cm3} g/cm³
                {mat.printed ? ` · typical, orientation dependent (Z ≈ ${Math.round(mat.z_strength_factor * 100)} %)` : ''}
              </div>
            )}
            {mat && projMat?.id !== mat.id && (
              <button
                className="btn small primary"
                style={{ marginBottom: 12 }}
                onClick={() => patchDesign('frame', { material: mat.id, density_g_cm3: mat.density_g_cm3 })}
                title="Updates mass, print and molding analyses everywhere"
              >
                Use {mat.id} for the Iris frame
              </button>
            )}
            <Select
              label="Voxel size"
              value={String(voxel)}
              options={VOXELS.map((v) => ({ value: String(v), label: `${v} mm${v === 2 ? ' (metric: 2 through the temple)' : v === 1.33 ? ' (3 through the temple)' : v === 1 ? ' (4 through the temple)' : ''}` }))}
              onChange={(v) => {
                setVoxel(Number(v));
                setFine(false);
              }}
            />
            <div className="row" style={{ marginBottom: 10 }}>
              <button className={`btn small ${fine ? 'primary' : ''}`} onClick={() => setFine((f) => !f)} aria-pressed={fine}>
                {fine ? `✓ Fine run (${FINE_VOXEL} mm, ≤ 30 k voxels)` : `Run finer (${FINE_VOXEL} mm, a few seconds)`}
              </button>
            </div>
            <Select
              label="Preconditioner"
              value={precond}
              options={[
                { value: 'multigrid', label: 'Geometric multigrid (fast)' },
                { value: 'jacobi', label: 'Jacobi (diagonal, textbook)' },
              ]}
              onChange={setPrecond}
            />
            <Select
              label="Element"
              value={element}
              options={[
                { value: 'h8i', label: 'Brick + incompatible modes (bends correctly)' },
                { value: 'h8', label: 'Plain trilinear brick (locks in bending)' },
              ]}
              onChange={setElement}
            />
          </Card>
          {r && (
            <Card title="Downloads">
              <div className="row">
                <DownloadButton
                  small
                  filename={`iris-${r.part}-${r.case}-nodal.csv`}
                  label="nodal results CSV"
                  content={() => fieldCsv(r)}
                />
                <DownloadButton small filename={`iris-${r.part}-${r.case}-summary.json`} label="summary JSON" content={() => JSON.stringify(summaryOf(r), null, 2)} />
              </div>
            </Card>
          )}
        </div>
      </div>
      <div className="grid grid-2">
        <Card title="Conjugate-gradient convergence" subtitle="Relative residual ‖f − K u‖ / ‖f‖ each iteration (log scale)">
          {r ? (
            <Plot
              series={[{ name: `${r.precond === 'multigrid' ? 'Multigrid' : 'Jacobi'}-preconditioned CG`, x: r.history.map((_, i) => i), y: r.history.map((v) => Math.max(v, 1e-12)) }]}
              yLog
              xLabel="iteration"
              yLabel="relative residual"
              hLines={[{ value: 1e-6, label: 'tolerance 1e-6' }]}
              height={230}
            />
          ) : (
            <div className="muted small">—</div>
          )}
          <CompareCg input={input} />
        </Card>
        <IrisCases iris={iris} fidelity={fidelity} setFidelity={setFidelity} />
      </div>
      <div className="grid grid-2">
        <MeshStudy part={effPart} caseId={caseId} material={matId} />
        <DropCard iris={iris.data} />
      </div>
      {r && (
        <Explainer title="Reading this result">
          <ul className="small" style={{ margin: 0, paddingLeft: 18 }}>
            {r.notes.map((n, i) => (
              <li key={i}>{n}</li>
            ))}
          </ul>
        </Explainer>
      )}
    </div>
  );
}

function Legend({ r, mode }: { r: FeaResult; mode: ColorMode }) {
  return (
    <div className="st-legend" aria-label="Colour scale">
      <div className="small" style={{ fontWeight: 600 }}>{mode === 'vm' ? 'von Mises stress, MPa' : 'Utilisation (stress ÷ allowable)'}</div>
      {mode === 'vm' && <div className="small muted">scale tops out at the design stress</div>}
      <div className="st-legend-bar" style={{ background: HEAT_GRADIENT }} />
      <div className="st-legend-ticks tnum">
        <span>0</span>
        <span>{mode === 'vm' ? fmt(r.max_vm_mpa / 2, 1) : '0.5'}</span>
        <span>{mode === 'vm' ? `≥ ${fmt(r.max_vm_mpa, 1)}` : '1.0'}</span>
      </div>
      {mode === 'util' && (
        <div className="small row" style={{ gap: 6, marginTop: 4 }}>
          <span style={{ width: 12, height: 12, borderRadius: 3, background: CRITICAL, display: 'inline-block' }} /> ✕ ≥ 1: beyond yield
        </div>
      )}
    </div>
  );
}

function fieldCsv(r: FeaResult): string {
  const f = r.voxels ?? r.cad;
  if (!f) return '';
  const rows: (string | number)[][] = [['x_mm', 'y_mm', 'z_mm', 'ux_mm', 'uy_mm', 'uz_mm', 'von_mises_mpa', 'utilisation']];
  for (let i = 0; i < f.positions.length / 3; i++) {
    rows.push([
      +f.positions[3 * i].toFixed(3),
      +f.positions[3 * i + 1].toFixed(3),
      +f.positions[3 * i + 2].toFixed(3),
      +f.disp[3 * i].toExponential(4),
      +f.disp[3 * i + 1].toExponential(4),
      +f.disp[3 * i + 2].toExponential(4),
      +f.vm[i].toFixed(3),
      +f.util[i].toFixed(4),
    ]);
  }
  return toCsv(rows);
}

function summaryOf(r: FeaResult) {
  const { voxels: _v, cad: _c, history, ...rest } = r;
  void _v;
  void _c;
  return { ...rest, iterations: history.length - 1 };
}

/** Run the same model with the other preconditioner and overlay the residuals. */
function CompareCg({ input }: { input: Record<string, unknown> }) {
  const [out, setOut] = useState<{ jac: number[]; mg: number[]; msJ: number; msM: number } | null>(null);
  const [busy, setBusy] = useState(false);
  const run = async () => {
    setBusy(true);
    try {
      const base = { ...input, surface: 'none', voxel_mm: Math.max(1.5, Number(input.voxel_mm)), max_elements: 4000 };
      const a = await workerCall<FeaResult>('structures.fea', { ...base, precond: 'jacobi', max_iter: 8000 });
      const b = await workerCall<FeaResult>('structures.fea', { ...base, precond: 'multigrid' });
      setOut({ jac: a.data.history, mg: b.data.history, msJ: a.ms, msM: b.ms });
    } finally {
      setBusy(false);
    }
  };
  return (
    <div style={{ marginTop: 10 }}>
      <div className="row">
        <button className="btn small" onClick={run} disabled={busy}>
          {busy ? 'Solving twice…' : 'Compare Jacobi vs multigrid (1.5 mm)'}
        </button>
        {out && (
          <span className="small muted">
            Jacobi {out.jac.length - 1} it · {(out.msJ / 1000).toFixed(2)} s — multigrid {out.mg.length - 1} it · {(out.msM / 1000).toFixed(2)} s
          </span>
        )}
      </div>
      {out && (
        <Plot
          series={[
            { name: 'Jacobi PCG', x: out.jac.map((_, i) => i), y: out.jac.map((v) => Math.max(v, 1e-12)) },
            { name: 'Multigrid PCG', x: out.mg.map((_, i) => i), y: out.mg.map((v) => Math.max(v, 1e-12)) },
          ]}
          yLog
          xLog
          xDomain={[1, Math.max(out.jac.length, out.mg.length)]}
          xLabel="iteration (log)"
          yLabel="relative residual"
          legend
          height={210}
        />
      )}
    </div>
  );
}

function IrisCases({ iris, fidelity, setFidelity }: { iris: IrisState; fidelity: 'metric' | 'fine'; setFidelity: (f: 'metric' | 'fine') => void }) {
  const o = iris.data;
  return (
    <Card
      title="Iris structural verification"
      subtitle={
        fidelity === 'metric'
          ? 'Every load case on the current design — the coarse pass the requirement metrics use (2 voxels through the temple)'
          : 'Fine pass: 4 voxels through the temple, plus the pod squeeze (not used by the metrics)'
      }
      actions={
        <>
          {iris.busy && <span className="cp-spin" aria-label="computing" />}
          <div className="cp-seg" role="group" aria-label="Fidelity">
            {(['metric', 'fine'] as const).map((f) => (
              <button key={f} className={`cp-seg-btn ${fidelity === f ? 'active' : ''}`} onClick={() => setFidelity(f)} aria-pressed={fidelity === f}>
                {f === 'metric' ? 'metric pass' : 'fine pass'}
              </button>
            ))}
          </div>
        </>
      }
    >
      <ErrorBox error={iris.error} />
      {!o ? (
        <div className="muted small">{iris.busy ? 'Solving every load case in the worker…' : 'Waiting for the engine…'}</div>
      ) : (
        <>
          <div className="table-wrap">
            <table className="table">
              <thead>
                <tr>
                  <th>Case</th>
                  <th>Force</th>
                  <th>Design stress</th>
                  <th>Corner peak</th>
                  <th>Deflection</th>
                  <th>SF</th>
                </tr>
              </thead>
              <tbody>
                {o.cases.map((c) => (
                  <tr key={c.label}>
                    <td>
                      {c.label}
                      <div className="small muted">
                        {c.region} · {c.elements.toLocaleString()} voxels of {fmt(c.h_mm, 2)} mm
                      </div>
                    </td>
                    <td className="tnum">{fmt(c.force_n, 1)} N</td>
                    <td className="tnum">{fmt(c.max_vm_mpa, c.max_vm_mpa < 1 ? 3 : 1)} MPa</td>
                    <td className="tnum muted">{fmt(c.peak_vm_mpa, c.peak_vm_mpa < 1 ? 3 : 1)} MPa</td>
                    <td className="tnum">{fmt(c.deflection_mm, 2)} mm</td>
                    <td className="tnum">
                      {c.counts_for_min ? (
                        <Chip tone={sfTone(c.safety_factor)}>
                          {sfTone(c.safety_factor) === 'pass' ? '✓' : sfTone(c.safety_factor) === 'warn' ? '!' : '✕'} {fmt(c.safety_factor, 2)}×
                        </Chip>
                      ) : (
                        <span className="muted small">unit load</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="row small" style={{ marginTop: 10 }}>
            <StatusChip status={o.min_safety_factor >= 2 ? 'pass' : o.min_safety_factor >= 1 ? 'warn' : 'fail'} />
            <span>
              Minimum safety factor <b className="tnum">{fmt(o.min_safety_factor, 2)}×</b> — {o.worst_case}. Recommended requirement ≥ 2 for a consumer product
              {fidelity === 'fine' ? ' (the metric uses the coarse pass).' : '.'}
            </span>
          </div>
          <ul className="small muted" style={{ margin: '8px 0 0', paddingLeft: 18 }}>
            {o.notes.map((n, i) => (
              <li key={i}>{n}</li>
            ))}
            {o.solid_fraction_note && <li>{o.solid_fraction_note}</li>}
          </ul>
          <div className="row" style={{ marginTop: 8 }}>
            <DownloadButton small filename={`iris-structures-${o.fidelity}.csv`} label="load cases CSV" content={() => casesCsv(o)} />
            <DownloadButton small filename={`iris-structures-${o.fidelity}.json`} label="full verification JSON" content={() => JSON.stringify(o, null, 2)} />
          </div>
        </>
      )}
    </Card>
  );
}

function casesCsv(o: IrisOut): string {
  const rows: (string | number)[][] = [['case', 'part', 'label', 'force_n', 'voxel_mm', 'elements', 'iterations', 'design_vm_mpa', 'corner_peak_mpa', 'safety_factor', 'governing', 'deflection_mm', 'stiffness_n_mm', 'region']];
  for (const c of o.cases) {
    rows.push([c.case, c.part, c.label, c.force_n, c.h_mm, c.elements, c.iterations, +c.max_vm_mpa.toFixed(4), +c.peak_vm_mpa.toFixed(4), +c.safety_factor.toFixed(4), c.governing, +c.deflection_mm.toFixed(5), +c.stiffness_n_mm.toFixed(4), c.region]);
  }
  return toCsv(rows);
}

function MeshStudy({ part, caseId, material }: { part: string; caseId: string; material: string }) {
  const project = useProject((s) => s.project);
  const [rows, setRows] = useState<StudyRow[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const run = async () => {
    setBusy(true);
    setErr(null);
    try {
      const { data } = await workerCall<StudyRow[]>('structures.mesh_study', { project, part, case: caseId, material, sizes: [2.0, 1.33, 1.0, 0.8, 0.67] });
      setRows(data);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Card
      title="Mesh-convergence study"
      subtitle="Does the answer change when the voxels shrink? Deflection settles; the sharp-corner peak keeps climbing (a singularity) and the design stress creeps up as its exclusion zone shrinks."
      actions={<button className="btn small" onClick={run} disabled={busy}>{busy ? 'Running 5 models…' : rows ? 'Re-run' : 'Run study'}</button>}
    >
      <ErrorBox error={err} />
      {rows ? (
        <>
          <Plot
            series={[
              { name: 'Design stress (MPa)', x: rows.map((r) => r.h_mm), y: rows.map((r) => r.max_vm_mpa) },
              { name: 'Sharp-corner peak (MPa)', x: rows.map((r) => r.h_mm), y: rows.map((r) => r.peak_vm_mpa), dash: '4 3' },
            ]}
            xLabel="voxel size, mm"
            yLabel="stress, MPa"
            legend
            yZero
            height={200}
          />
          <Plot
            series={[{ name: 'Load-point deflection (mm)', x: rows.map((r) => r.h_mm), y: rows.map((r) => r.deflection_mm), color: 'var(--series-3)' }]}
            xLabel="voxel size, mm"
            yLabel="deflection, mm"
            yZero
            height={170}
          />
          <div className="small muted">
            {rows.map((r) => `${r.h_mm} mm: ${r.elements.toLocaleString()} voxels, ${r.iterations} it`).join(' · ')}
          </div>
        </>
      ) : (
        <div className="muted small">{busy ? 'Solving the same case at 2.0 → 0.67 mm…' : 'Five solves from 2 mm to 0.67 mm voxels (several seconds in the worker).'}</div>
      )}
    </Card>
  );
}
