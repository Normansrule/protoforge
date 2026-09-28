// Optics Pro · Lens designer: a Lens Data Editor (surface table), a live
// meridional layout with real refracted rays in three wavelengths, presets
// (ideal lens, PMMA plano-convex, aspheric PMMA, PMMA+PC and glass
// achromats) and a damped-least-squares optimiser whose iterations animate.
import { useEffect, useMemo, useRef, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { tryCall } from '../../engine/engine';
import { useProject } from '../../state/project';
import { usePrefersReducedMotion } from '../control/kit';
import { svgForDownload } from './RayDiagram';
import { LensLayout } from './LensLayout';
import { LENS_START, useLens } from './lensStore';
import type { Catalog, LensAnalysis, LensPreset, LensSystem, OptOut, Surface, Variable } from './proTypes';
import { waveColor, waveName } from './proTypes';
import { PRO_REFS, NumCell, prescriptionCsv, rayopticsScript, fmt } from './proUtil';
import { REFS } from './util';

const PARAMS: { key: string; label: string }[] = [
  { key: 'curvature', label: 'c' },
  { key: 'conic', label: 'k' },
  { key: 'a4', label: 'A4' },
  { key: 'a6', label: 'A6' },
];

export function useLensInit() {
  const project = useProject((s) => s.project);
  const presetsC = useCompute<LensPreset[]>('optics.lens_presets', { optics: project.design.optics });
  const { system, load } = useLens();
  const presets = presetsC.data;
  useEffect(() => {
    if (system || !presets?.length) return;
    const want = (project.design.extra?.optics_lens as string) || 'achromat_cop_pc';
    const p = presets.find((x) => x.id === want) ?? presets.find((x) => x.id === 'achromat_cop_pc') ?? presets[0];
    const rx = project.design.extra?.optics_lens_rx as Surface[] | undefined;
    if (want === 'custom' && Array.isArray(rx) && rx.length) {
      load('custom', { ...p.system, surfaces: [...p.system.surfaces.slice(0, LENS_START), ...rx] }, p.variables);
    } else {
      load(p.id, p.system, p.variables);
    }
  }, [presets, system, load, project.design.extra]);
  return presetsC;
}

export function LensTab() {
  const project = useProject((s) => s.project);
  const patch = useProject((s) => s.patchDesign);
  const reduced = usePrefersReducedMotion();
  const presetsC = useLensInit();
  const presets = presetsC.data ?? [];
  const { presetId, system, variables, edited, load, setSystem, setVariables } = useLens();
  const catalog = useCompute<Catalog>('optics.materials', {});
  const an = useCompute<LensAnalysis>('optics.lens_analyze', { project, system, config: { rings: 5, rays_per_fan: 7 } }, !!system);
  const [last, setLast] = useState<LensAnalysis | null>(null);
  useEffect(() => { if (an.data) setLast(an.data); }, [an.data]);
  const A = an.data ?? last;
  const [hi, setHi] = useState<number | null>(null);
  const [playing, setPlaying] = useState(!reduced);
  const [showWave, setShowWave] = useState([true, true, true]);
  const svg = useRef<SVGSVGElement>(null);

  // optimiser run + animation
  const [opt, setOpt] = useState<OptOut | null>(null);
  const [optErr, setOptErr] = useState<string | null>(null);
  const [step, setStep] = useState(0);
  const [animating, setAnimating] = useState(false);
  const before = useRef<LensSystem | null>(null);
  const runOpt = () => {
    if (!system) return;
    const { data, error } = tryCall<OptOut>('optics.lens_optimize', { project, system, variables, max_iter: 40 });
    setOptErr(error);
    if (!data) return;
    before.current = system;
    setOpt(data);
    if (reduced) {
      setStep(data.history.length - 1);
      setSystem(data.system);
    } else {
      setStep(0);
      setAnimating(true);
    }
  };
  useEffect(() => {
    if (!animating || !opt) return;
    const n = opt.history.length;
    const id = window.setInterval(() => {
      setStep((s) => {
        if (s + 1 >= n) {
          window.clearInterval(id);
          setAnimating(false);
          setSystem(opt.system);
          return n - 1;
        }
        return s + 1;
      });
    }, Math.max(90, Math.min(260, 3200 / n)));
    return () => window.clearInterval(id);
  }, [animating, opt, setSystem]);

  const shownLayout = animating && opt ? opt.layouts[Math.min(step, opt.layouts.length - 1)] : A?.layout;
  const q = A?.quality;
  const px = q?.paraxial[system?.primary ?? 1];
  const pixel = q?.pixel_arcmin ?? 0;
  const mats = useMemo(() => catalog.data?.materials.map((m) => m.id) ?? [], [catalog.data]);

  const editSurface = (i: number, p: Partial<Surface>) => {
    if (!system) return;
    const surfaces = system.surfaces.map((s, k) => (k === i ? { ...s, ...p } : s));
    setSystem({ ...system, surfaces });
    setOpt(null);
  };
  const addSurface = (after: number) => {
    if (!system) return;
    const surfaces = [...system.surfaces];
    surfaces.splice(after + 1, 0, { label: 'new', radius: 0, thickness: 1, material: '', conic: 0, aspheric: [], semi_diameter: 0, paraxial_f: 0, fold: false });
    setSystem({ ...system, surfaces });
    setVariables(variables.map((v) => (v.surface + LENS_START > after ? { ...v, surface: v.surface + 1 } : v)));
  };
  const removeSurface = (i: number) => {
    if (!system || i < LENS_START || system.surfaces.length <= LENS_START + 1) return;
    const surfaces = system.surfaces.filter((_, k) => k !== i);
    setSystem({ ...system, surfaces });
    setVariables(variables.filter((v) => v.surface + LENS_START !== i).map((v) => (v.surface + LENS_START > i ? { ...v, surface: v.surface - 1 } : v)));
  };
  const isVar = (i: number, p: string) => variables.some((v) => v.surface + LENS_START === i && v.param === p);
  const toggleVar = (i: number, p: string) => {
    const v: Variable = { surface: i - LENS_START, param: p };
    setVariables(isVar(i, p) ? variables.filter((x) => !(x.surface === v.surface && x.param === p)) : [...variables, v]);
  };
  const choosePreset = (id: string) => {
    const p = presets.find((x) => x.id === id);
    if (p) {
      load(p.id, p.system, p.variables);
      setOpt(null);
    }
  };
  const projectLens = (project.design.extra?.optics_lens as string) || 'achromat_cop_pc';
  const inUse = edited ? false : projectLens === presetId;
  const useForIris = () => {
    if (!system) return;
    if (!edited && presetId !== 'custom') patch('extra', { optics_lens: presetId, optics_lens_rx: undefined } as Record<string, unknown>);
    else patch('extra', { optics_lens: 'custom', optics_lens_rx: system.surfaces.slice(LENS_START) } as Record<string, unknown>);
  };
  const preset = presets.find((p) => p.id === presetId);
  const tone = (ok: boolean) => (ok ? 'pass' : 'fail');

  if (!system) return <Card title="Lens designer">{presetsC.error ? <ErrorBox error={presetsC.error} /> : 'Loading presets…'}</Card>;
  const hist = opt ? opt.history.slice(0, step + 1) : [];

  return (
    <div className="stack">
      <ErrorBox error={an.error} />
      {q && px && (
        <div className="op-stats">
          <Stat label="Effective focal length (EFL)" value={fmt(A!.lens_efl, 3)} unit="mm" note={`back focal length ${fmt(px.bfl, 2)} mm · working f/${fmt(px.working_fnum, 2)} (4 mm eye pupil)`} />
          <Stat label="RMS spot, worst field" value={fmt(q.worst_rms_arcmin, 2)} unit="arcmin" tone={tone(q.worst_rms_arcmin <= pixel)} note={`${q.worst_rms_arcmin <= pixel ? '✓' : '✕'} target ≤ 1 pixel = ${pixel.toFixed(2)}′ · ${fmt(q.worst_rms_mm * 1000, 1)} µm on the display`} />
          <Stat label="MTF at Nyquist, worst" value={fmt(q.worst_mtf_nyquist, 2)} tone={tone(q.worst_mtf_nyquist >= 0.3)} note={`${q.worst_mtf_nyquist >= 0.3 ? '✓' : '✕'} rule of thumb ≥ 0.3 at ${fmt(A!.nyquist_lp_mm, 1)} cycles/mm · on-axis ${fmt(q.mtf.fields[0] ? Math.min(q.mtf.fields[0].at_nyquist_t, q.mtf.fields[0].at_nyquist_s) : 0, 2)} · diffraction limit ${fmt(q.mtf.diffraction_at_nyquist, 2)}`} />
          <Stat label="Lateral colour F−C" value={fmt(q.worst_lateral_arcmin, 2)} unit="arcmin" tone={tone(q.worst_lateral_arcmin <= pixel)} note={`longitudinal ${fmt(q.chromatic.longitudinal_mm * 1000, 0)} µm (${fmt(q.chromatic.longitudinal_diopter, 2)} D) · lens mass ${fmt(A!.lens_mass_g, 2)} g`} />
        </div>
      )}
      <div className="op-main">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card
            title="Layout — reversed, from the eye to the display"
            subtitle={preset ? preset.summary : 'Your edited lens.'}
            actions={
              <div className="row" style={{ gap: 6 }}>
                <button className="btn small" onClick={() => setPlaying((p) => !p)} aria-label={playing ? 'pause light pulses' : 'play light pulses'}>{playing ? '❚❚ Pause' : '▶ Play'}</button>
                <DownloadButton small filename="iris-lens-layout.svg" content={() => svgForDownload(svg.current)} label="SVG" />
              </div>
            }
          >
            <div className="op-diagram">
              {shownLayout && <LensLayout ref={svg} layout={shownLayout} waves={animating ? 1 : system.wavelengths_nm.length} showWave={animating ? [true] : showWave} highlight={hi} onPick={setHi} playing={playing && !animating && !reduced} hud maxHeight={440} />}
            </div>
            <div className="op-toolbar">
              {system.wavelengths_nm.map((nm, w) => (
                <label key={w} className="op-check">
                  <input type="checkbox" checked={showWave[w] ?? true} onChange={(e) => setShowWave((s) => s.map((x, k) => (k === w ? e.target.checked : x)))} />
                  <span className="op-sw" style={{ background: waveColor(w, system.wavelengths_nm.length) }} /> {waveName(nm)}
                </label>
              ))}
              <span style={{ flex: 1 }} />
              <span className="small muted">fields {system.fields_deg.map((f) => `${f.toFixed(1)}°`).join(' · ')} · display {fmt(q?.image_distance_mm ?? 0, 2)} mm behind the lens (best focus)</span>
            </div>
            {q && q.warnings.length > 0 && <div className="callout op-warn small" style={{ marginTop: 8 }}>{q.warnings.join(' · ')}</div>}
          </Card>

          <Card
            title="Lens Data Editor"
            subtitle="Radius R (0 = flat), thickness to the next surface, material after the surface, conic k and even aspheric terms. Tick c / k / A4 / A6 to let the optimiser vary them."
            actions={
              <div className="row" style={{ gap: 6 }}>
                <DownloadButton small filename="iris-lens.json" content={() => JSON.stringify(system, null, 1)} label="system.json" />
                <DownloadButton small filename="iris-lens-prescription.csv" content={() => prescriptionCsv(system)} label="CSV" />
                <DownloadButton small filename="iris_lens_rayoptics.py" content={() => rayopticsScript(system)} label="rayoptics.py" />
              </div>
            }
          >
            <div className="table-wrap op-lde-wrap">
              <table className="table op-lde">
                <thead>
                  <tr><th>#</th><th>Surface</th><th>Radius</th><th>Thickness</th><th>Material</th><th>Conic k</th><th>A4</th><th>A6</th><th>Semi-dia.</th><th>Vary</th><th /></tr>
                </thead>
                <tbody>
                  {system.surfaces.map((s, i) => {
                    const lens = i >= LENS_START;
                    const lastRow = i === system.surfaces.length - 1;
                    return (
                      <tr key={i} className={hi === i ? 'hi' : ''} onMouseEnter={() => setHi(i)} onMouseLeave={() => setHi(null)}>
                        <td className="mono">{i === 0 ? 'STO' : i}</td>
                        <td className="op-lde-label">{s.label || (s.paraxial_f ? 'ideal lens' : '—')}</td>
                        <td>{s.paraxial_f ? <span className="muted small">f {fmt(s.paraxial_f, 3)}</span> : <NumCell value={s.radius} disabled={!lens} onChange={(v) => editSurface(i, { radius: v })} />}</td>
                        <td><NumCell value={s.thickness} disabled={lastRow} onChange={(v) => editSurface(i, { thickness: v })} title={lastRow ? 'display distance: set by best focus' : undefined} /></td>
                        <td>
                          {lens && !s.paraxial_f ? (
                            <select className="op-cell" value={s.material} onChange={(e) => editSurface(i, { material: e.target.value })} aria-label={`material after surface ${i}`}>
                              <option value="">air</option>
                              {mats.map((m) => <option key={m} value={m}>{m}</option>)}
                              <option value="MIRROR">MIRROR</option>
                              {s.material && !mats.includes(s.material) && s.material !== 'MIRROR' && <option value={s.material}>{s.material}</option>}
                            </select>
                          ) : <span className="muted small">air</span>}
                        </td>
                        <td><NumCell value={s.conic} disabled={!lens || !!s.paraxial_f} onChange={(v) => editSurface(i, { conic: v })} /></td>
                        <td><NumCell value={s.aspheric[0] ?? 0} disabled={!lens || !!s.paraxial_f} onChange={(v) => editSurface(i, { aspheric: [v, s.aspheric[1] ?? 0, ...s.aspheric.slice(2)] })} /></td>
                        <td><NumCell value={s.aspheric[1] ?? 0} disabled={!lens || !!s.paraxial_f} onChange={(v) => editSurface(i, { aspheric: [s.aspheric[0] ?? 0, v, ...s.aspheric.slice(2)] })} /></td>
                        <td><NumCell value={s.semi_diameter} disabled={!lens} onChange={(v) => editSurface(i, { semi_diameter: Math.max(0, v) })} /></td>
                        <td className="op-vary">
                          {lens && !s.paraxial_f && PARAMS.map((p) => (
                            <button key={p.key} className={`op-vbtn ${isVar(i, p.key) ? 'on' : ''}`} aria-pressed={isVar(i, p.key)} onClick={() => toggleVar(i, p.key)} title={`vary ${p.key}`}>{p.label}</button>
                          ))}
                        </td>
                        <td className="op-rowbtns">
                          {i >= LENS_START - 1 && <button className="btn small ghost" onClick={() => addSurface(i)} title="insert a surface after this one" aria-label={`insert after ${i}`}>＋</button>}
                          {lens && system.surfaces.length > LENS_START + 1 && <button className="btn small ghost" onClick={() => removeSurface(i)} title="delete this surface" aria-label={`delete surface ${i}`}>✕</button>}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
            {A && A.elements.length > 0 && (
              <div className="row small" style={{ marginTop: 8, gap: 6 }}>
                {A.elements.map((e, k) => (
                  <Chip key={k} tone={e.edge_mm < 0.3 ? 'fail' : undefined} title="centre / edge thickness">
                    {e.material} S{e.surfaces[0]}–S{e.surfaces[1]}: centre {fmt(e.center_mm, 2)} mm · edge {fmt(e.edge_mm, 2)} mm{e.edge_mm < 0.3 ? ' ✕ too thin' : ''}
                  </Chip>
                ))}
              </div>
            )}
          </Card>
        </div>

        <div className="stack">
          <Card title="Design" subtitle="Presets are designed at f = 20 mm and scaled to the Iris focal length and aperture.">
            <label className="field">
              <span className="field-top"><span className="field-label">Preset</span>{edited && <span className="small muted">edited</span>}</span>
              <select value={presetId} onChange={(e) => choosePreset(e.target.value)}>
                {presetId === 'custom' && <option value="custom">Custom (saved in the project)</option>}
                {presets.map((p) => <option key={p.id} value={p.id}>{p.title}</option>)}
              </select>
            </label>
            <div className="row" style={{ gap: 6, flexWrap: 'wrap' }}>
              <button className="btn small" onClick={() => choosePreset(presetId === 'custom' ? 'achromat_cop_pc' : presetId)}>Reset</button>
              <button className={`btn small ${inUse ? '' : 'primary'}`} onClick={useForIris} disabled={inUse}>{inUse ? '✓ Iris uses this lens' : 'Use for Iris metrics'}</button>
            </div>
            <div className="small muted" style={{ marginTop: 8 }}>
              The informational metrics <code>optics.rms_spot_um</code>, <code>optics.rms_spot_arcmin</code>, <code>optics.mtf_at_nyquist</code> and <code>optics.lateral_color_arcmin</code> use the lens chosen here (now: <b>{projectLens}</b>). The first-order metrics (field of view, pixels per degree, eyebox) stay paraxial.
            </div>
          </Card>
          <Card
            title="Optimiser (damped least squares)"
            subtitle={`${variables.length} variable${variables.length === 1 ? '' : 's'} · merit = RMS spot over ${system.fields_deg.length} fields × ${system.wavelengths_nm.length} wavelengths, EFL held at ${project.design.optics.lens_focal_mm} mm`}
          >
            <div className="row" style={{ gap: 6 }}>
              <button className="btn primary" onClick={runOpt} disabled={animating || variables.length === 0}>{animating ? 'Optimising…' : '▶ Optimise'}</button>
              {opt && before.current && !animating && <button className="btn small" onClick={() => { setSystem(before.current!); setOpt(null); }}>Undo</button>}
            </div>
            <ErrorBox error={optErr} />
            {opt && (
              <>
                <Plot
                  series={[{ name: 'RMS spot (arcmin)', x: hist.map((h) => h.iter), y: hist.map((h) => h.rms_arcmin), color: 'var(--series-1)', width: 2.2 }]}
                  height={170}
                  xLabel="iteration"
                  yLabel="RMS spot (′)"
                  xDomain={[0, Math.max(1, opt.history.length - 1)]}
                  yDomain={[0, Math.max(...opt.history.map((h) => h.rms_arcmin)) * 1.05]}
                  hLines={pixel ? [{ value: pixel, label: '1 pixel', color: 'var(--good)' }] : []}
                  legend={false}
                />
                <div className="small muted">
                  Step {hist[hist.length - 1]?.iter ?? 0}/{opt.history.length - 1}: {fmt(hist[hist.length - 1]?.rms_arcmin ?? 0, 2)}′, EFL {fmt(hist[hist.length - 1]?.efl ?? 0, 3)} mm, damping λ {(hist[hist.length - 1]?.damping ?? 0).toExponential(1)}. {!animating && opt.note}
                </div>
              </>
            )}
            {variables.length === 0 && <div className="small muted">Tick at least one c / k / A4 / A6 box in the table.</div>}
          </Card>
          {q && px && (
            <Card title="First-order data (paraxial y–nu trace)">
              <dl className="op-kv stacked">
                <dt>EFL (lens)</dt><dd>{fmt(A!.lens_efl, 4)} mm</dd>
                <dt>Front / rear principal planes</dt><dd>z = {fmt(px.z_front_principal, 2)} / {fmt(px.z_rear_principal, 2)} mm</dd>
                <dt>Paraxial image of the virtual object</dt><dd>z = {fmt(px.z_paraxial_image, 3)} mm</dd>
                <dt>Best-focus shift</dt><dd>{fmt(q.focus_shift_mm * 1000, 0)} µm</dd>
                <dt>Exit pupil (image of the eye)</dt><dd>{px.z_exit_pupil === null ? '∞' : `z = ${fmt(px.z_exit_pupil, 1)} mm`}</dd>
                <dt>Working f-number</dt><dd>f/{fmt(px.working_fnum, 2)}</dd>
                <dt>Pixel pitch / angular size</dt><dd>{fmt(A!.pixel_mm * 1000, 2)} µm / {pixel.toFixed(2)}′</dd>
              </dl>
            </Card>
          )}
        </div>
      </div>

      <div className="op-explain">
        <Explainer title="Real rays: vector Snell's law on aspheric surfaces" refs={[PRO_REFS.bornwolf, PRO_REFS.welford, PRO_REFS.smith]}>
          <Formula>{`z(r) = c r² / (1 + √(1 − (1+k) c² r²)) + A4 r⁴ + A6 r⁶ …\nd' = μ d + (√(1 − μ²(1 − cos²I)) − μ cos I) n,   μ = n₁/n₂\n(total internal reflection when the root is imaginary)`}</Formula>
          <p>
            Every ray is intersected exactly with each surface (a quadratic for the conic, Newton's method for the polynomial terms), refracted in 3D and carried on. The first-order numbers come
            from the paraxial y–nu trace, <i>n′u′ = nu − yφ</i> with φ = (n′ − n)c. The Iris combiner is a flat mirror, so it is drawn <i>unfolded</i>.
          </p>
        </Explainer>
        <Explainer title="Why design it backwards?" refs={[REFS.kress, PRO_REFS.kingslake]}>
          <p>
            In a head-mounted display the eye pupil is the aperture stop. Designers therefore trace from the eye (a {system.pupil_diameter_mm} mm pupil) through the lens to the display — the direction of a camera lens —
            and convert blur on the display to an angle with the focal length: one {fmt((A?.pixel_mm ?? 0) * 1000, 1)} µm pixel subtends {pixel.toFixed(2)} arcmin. A lens that keeps the RMS spot below one pixel at every field
            shows every pixel sharply. The object is the virtual image, {system.object_distance_mm ? `${fmt(-system.object_distance_mm / 1000, 2)} m` : 'at infinity'} in front of the eye.
          </p>
        </Explainer>
        <Explainer title="Damped least squares" refs={[PRO_REFS.kingslake, PRO_REFS.smith]}>
          <Formula>{`minimise  Φ = Σ wᵢ fᵢ(x)²      (ray errors + EFL target)\n(JᵀJ + λ·diag JᵀJ) Δx = −Jᵀf     λ ↓ on success, ↑ on failure`}</Formula>
          <p>
            The same Levenberg–Marquardt local optimiser Zemax OpticStudio and CODE V use by default. The Jacobian J comes from finite differences; the display is refocused in closed form at every step,
            and centre thicknesses follow the curvatures so element edges stay ≥ 0.8 mm. It finds the nearest local minimum — try different starting presets (global search is what the big tools add).
          </p>
        </Explainer>
        <Explainer title="Graduate to the real tools" refs={[PRO_REFS.rayoptics, PRO_REFS.phydemo, PRO_REFS.zemax, PRO_REFS.codev]}>
          <p>
            Export the prescription (<code>system.json</code>, CSV, or a starting <code>rayoptics</code> script) and re-enter it in <b>Zemax OpticStudio</b> or <b>CODE V</b> for tolerancing, stray light and
            FFT/Huygens MTF; <b>rayoptics</b> is the free Python route and <b>Ray Optics Simulation</b> is a browser playground for building intuition. Numbers here are a teaching-grade sequential model: no coatings,
            no polarisation, no tolerances, and the plastic indices are typical catalogue values.
          </p>
        </Explainer>
      </div>
    </div>
  );
}
