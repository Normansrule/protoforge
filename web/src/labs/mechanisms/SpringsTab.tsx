// Springs: helical compression, extension and torsion springs per Shigley
// ch. 10 — wire table (A, m), rate, Bergsträsser/Wahl, solid height,
// buckling, Goodman–Zimmerli fatigue and life, with an animated 3-D helix.
import { useEffect, useMemo, useState } from 'react';
import * as THREE from 'three';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat, Toggle } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { Scene3D } from '../../components/Scene3D';
import { useCompute } from '../../engine/hooks';
import type { CompressionOut, ExtensionOut, TorsionOut, WireMaterial } from './types';
import { Checks, Kv, Notes, PlayBar, fmt, fmtCycles, sfTone, springPath, useClock } from './util';

type Kind = 'compression' | 'extension' | 'torsion';

export function SpringsTab() {
  const [kind, setKind] = useState<Kind>('compression');
  const mats = useCompute<WireMaterial[]>('mechanisms.spring_materials', {}).data ?? [];
  return (
    <div className="stack">
      <div className="mx-seg" role="tablist" aria-label="Spring type">
        {([
          ['compression', 'Compression'],
          ['extension', 'Extension'],
          ['torsion', 'Torsion'],
        ] as [Kind, string][]).map(([id, l]) => (
          <button key={id} role="tab" aria-selected={kind === id} className={`btn small ${kind === id ? 'primary' : ''}`} onClick={() => setKind(id)}>
            {l}
          </button>
        ))}
      </div>
      {kind === 'compression' && <Compression mats={mats} />}
      {kind === 'extension' && <Extension mats={mats} />}
      {kind === 'torsion' && <Torsion mats={mats} />}
      <WireTable mats={mats} />
    </div>
  );
}

const matOptions = (mats: WireMaterial[]) => mats.map((m) => ({ value: m.id, label: `${m.name} (${m.spec})` }));

// ------------------------------------------------------------------ compression

interface CompState {
  material: string; d_mm: number; mean_d_mm: number; active_coils: number; ends: string; free_len_mm: number;
  f_min_n: number; f_max_n: number; peened: boolean; support: string;
}

function Compression({ mats }: { mats: WireMaterial[] }) {
  const [s, setS] = useState<CompState>({ material: 'music_wire', d_mm: 1.2, mean_d_mm: 10.0, active_coils: 8, ends: 'squared_ground', free_len_mm: 28, f_min_n: 10, f_max_n: 25, peened: false, support: 'fixed' });
  const set = (q: Partial<CompState>) => setS({ ...s, ...q });
  const { data: o, error } = useCompute<CompressionOut>('mechanisms.compression_spring', s);
  const json = () => JSON.stringify({ input: s, result: o }, null, 2);
  return (
    <div className="mx-split">
      <div className="stack" style={{ minWidth: 0 }}>
        <Card title="Helical compression spring" subtitle="Animated between the working loads; drag to orbit" actions={o && <DownloadButton small filename="compression-spring.json" label="design JSON" content={json} />}>
          <ErrorBox error={error} />
          {o && (
            <>
              <HelixScene d={s.d_mm} D={s.mean_d_mm} na={s.active_coils} ends={s.ends} L0={s.free_len_mm} Lmin={o.len_at_max_mm} Lmax={o.len_at_min_mm} Ls={o.solid_len_mm} />
              <div className="grid grid-4" style={{ marginTop: 12 }}>
                <Stat label="Rate k" value={fmt(o.rate_n_mm, 3)} unit="N/mm" note={`C = ${fmt(o.index_c, 2)}, K_B = ${fmt(o.k_b, 3)}`} />
                <Stat label="τ at F_max" value={fmt(o.tau_max_mpa, 0)} unit="MPa" note={`S_sy = ${fmt(o.ssy_mpa, 0)} MPa → n_s ${fmt(o.ns_static, 2)}`} tone={sfTone(o.ns_static, 1.2)} />
                <Stat label="Fatigue n_f (Goodman)" value={fmt(o.nf_goodman, 2)} tone={sfTone(o.nf_goodman, 1.5)} note={`life ${fmtCycles(o.life.cycles, o.life.runout)}${o.life.extrapolated ? ' (extrapolated)' : ''}`} />
                <Stat label="Solid length" value={fmt(o.solid_len_mm, 2)} unit="mm" note={`overrun ξ = ${fmt(o.overrun, 2)} (≥ 0.15)`} tone={o.overrun >= 0.15 ? 'pass' : 'warn'} />
              </div>
            </>
          )}
        </Card>
        {o && (
          <div className="grid grid-2">
            <Card title="Force vs deflection" subtitle="Linear to solid; shaded band = working range">
              <Plot
                series={[{ name: 'F', x: o.curve_y, y: o.curve_f }]}
                xLabel="deflection y, mm"
                yLabel="force, N"
                xBands={[{ from: s.f_min_n / o.rate_n_mm, to: s.f_max_n / o.rate_n_mm, label: 'working' }]}
                vLines={o.buckling_stable_absolute ? [] : [{ value: o.buckling_defl_mm, label: 'buckles', color: 'var(--bad)' }]}
                height={220}
              />
            </Card>
            <Card title="Goodman diagram (Zimmerli data)" subtitle="Torsional mean vs amplitude; the point must sit under the line">
              <GoodmanSvg ssu={o.ssu_mpa} sse={o.sse_mpa} ssy={o.ssy_mpa} ta={o.tau_a_mpa} tm={o.tau_m_mpa} peened={s.peened} />
            </Card>
          </div>
        )}
        {o && (
          <Card title="Design checks">
            <Checks checks={o.checks} />
            <div className="grid grid-2" style={{ marginTop: 10 }}>
              <Kv
                rows={[
                  ['S_ut = A/d^m', `${fmt(o.sut_mpa, 0)} MPa`],
                  ['G, E', `${fmt(o.g_mpa / 1000, 1)}, ${fmt(o.e_mpa / 1000, 1)} GPa`],
                  ['OD / ID', `${fmt(o.od_mm, 2)} / ${fmt(o.id_mm, 2)} mm`],
                  ['Total coils, pitch', `${fmt(o.total_coils, 1)}, ${fmt(o.pitch_mm, 2)} mm`],
                  ['Length at F_min / F_max', `${fmt(o.len_at_min_mm, 2)} / ${fmt(o.len_at_max_mm, 2)} mm`],
                ]}
              />
              <Kv
                rows={[
                  ['Wahl K_W (for comparison)', fmt(o.k_w, 3)],
                  ['Force to solid', `${fmt(o.f_solid_n, 1)} N (n_s ${fmt(o.ns_solid, 2)})`],
                  ['Buckling', o.buckling_stable_absolute ? `stable (λ_eff = ${fmt(o.slenderness, 2)})` : `y_cr = ${fmt(o.buckling_defl_mm, 1)} mm`],
                  ['Surge frequency', `${fmt(o.surge_hz, 0)} Hz`],
                  ['Spring mass', `${fmt(o.mass_g, 2)} g`],
                ]}
              />
            </div>
            <Notes notes={o.notes} />
          </Card>
        )}
      </div>
      <div className="stack mx-side">
        <Card title="Spring">
          <Select label="Wire" value={s.material} options={matOptions(mats)} onChange={(material) => set({ material })} />
          <Slider label="Wire diameter d" value={s.d_mm} min={0.1} max={6} step={0.01} unit="mm" onChange={(d_mm) => set({ d_mm })} />
          <Slider label="Mean coil diameter D" value={s.mean_d_mm} min={0.5} max={60} step={0.1} unit="mm" onChange={(mean_d_mm) => set({ mean_d_mm })} />
          <Slider label="Active coils N_a" value={s.active_coils} min={2} max={30} step={0.25} onChange={(active_coils) => set({ active_coils })} />
          <Slider label="Free length L₀" value={s.free_len_mm} min={3} max={250} step={0.5} unit="mm" onChange={(free_len_mm) => set({ free_len_mm })} />
          <Select label="Ends" value={s.ends} options={[{ value: 'plain', label: 'Plain' }, { value: 'plain_ground', label: 'Plain and ground' }, { value: 'squared', label: 'Squared (closed)' }, { value: 'squared_ground', label: 'Squared and ground' }]} onChange={(ends) => set({ ends })} />
          <Slider label="F_min" value={s.f_min_n} min={0} max={500} step={0.5} unit="N" onChange={(f_min_n) => set({ f_min_n })} />
          <Slider label="F_max" value={s.f_max_n} min={0.5} max={1000} step={0.5} unit="N" onChange={(f_max_n) => set({ f_max_n })} />
          <Select label="End support (buckling)" value={s.support} options={[{ value: 'fixed', label: 'Flat parallel plates (α 0.5)' }, { value: 'fixed_pivot', label: 'One plate, one pivot (α 0.707)' }, { value: 'pivoted', label: 'Both pivoted (α 1)' }, { value: 'clamped_free', label: 'Clamped–free (α 2)' }]} onChange={(support) => set({ support })} />
          <Toggle label="Shot peened (raises the Zimmerli endurance)" value={s.peened} onChange={(peened) => set({ peened })} />
        </Card>
        <Explainer title="Shigley's spring equations" refs={[{ label: 'Budynas & Nisbett, "Shigley’s Mechanical Engineering Design", 10th ed., ch. 10' }, { label: 'A. M. Wahl, "Mechanical Springs", 1963' }]}>
          <Formula>{`S_ut = A / d^m          S_sy ≈ 0.45 S_ut (music wire), 0.35 (302)
k = d⁴ G / (8 D³ N_a)    C = D/d
K_B = (4C + 2)/(4C − 3)  τ = K_B 8 F D / (π d³)
S_se = S_sa / (1 − S_sm/S_su),  S_su = 0.67 S_ut
n_f = 1 / (τ_a/S_se + τ_m/S_su)`}</Formula>
          <p className="small">Zimmerli found that for wire under 10 mm the fatigue limit barely depends on the steel: S_sa = 241 MPa and S_sm = 379 MPa unpeened (398 / 534 MPa peened).</p>
        </Explainer>
      </div>
    </div>
  );
}

function GoodmanSvg({ ssu, sse, ssy, ta, tm, peened }: { ssu: number; sse: number; ssy: number; ta: number; tm: number; peened: boolean }) {
  const W = 380, H = 240, pad = 42;
  const mx = ssu * 1.02;
  const my = Math.max(sse, ta, ssy * 0.6) * 1.15;
  const X = (v: number) => pad + (v / mx) * (W - pad - 12);
  const Y = (v: number) => H - pad + 8 - (v / my) * (H - pad - 18);
  const [za, zm] = peened ? [398, 534] : [241, 379];
  return (
    <svg className="mx-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Goodman diagram for the spring">
      <line className="mx-axis" x1={pad} y1={Y(0)} x2={W - 8} y2={Y(0)} />
      <line className="mx-axis" x1={pad} y1={Y(0)} x2={pad} y2={8} />
      <polygon points={`${X(0)},${Y(0)} ${X(0)},${Y(sse)} ${X(ssu)},${Y(0)}`} className="mx-safe" />
      <line x1={X(0)} y1={Y(sse)} x2={X(ssu)} y2={Y(0)} stroke="var(--series-1)" strokeWidth={2} />
      <line x1={X(0)} y1={Y(ssy)} x2={X(ssy)} y2={Y(0)} stroke="var(--series-4)" strokeWidth={1.4} strokeDasharray="5 3" />
      <circle cx={X(zm)} cy={Y(za)} r={4} fill="var(--series-3)" />
      <text x={X(zm) + 6} y={Y(za) - 4}>Zimmerli ({zm}, {za})</text>
      <line x1={X(0)} y1={Y(0)} x2={X(tm * 3)} y2={Y(ta * 3)} stroke="var(--muted)" strokeDasharray="3 3" />
      <circle cx={X(tm)} cy={Y(ta)} r={5} fill="var(--series-2)" stroke="var(--surface-1)" />
      <text x={X(tm) - 8} y={Y(ta) - 8} textAnchor="end">you: τ_m {fmt(tm, 0)}, τ_a {fmt(ta, 0)}</text>
      <text x={X(0) + 4} y={Y(sse) - 5}>S_se = {fmt(sse, 0)} MPa</text>
      <text x={X(ssu) - 2} y={Y(0) + 16} textAnchor="end">S_su = {fmt(ssu, 0)}</text>
      <text x={X(ssy * 0.5) + 4} y={Y(ssy * 0.5) - 4} fill="var(--series-4)">yield τ_a + τ_m = S_sy</text>
      <text x={W - 10} y={H - 6} textAnchor="end">τ_m, MPa →</text>
      <text x={pad + 4} y={16}>τ_a, MPa</text>
    </svg>
  );
}

// ------------------------------------------------------------------ 3-D helix

class HelixCurve extends THREE.Curve<THREE.Vector3> {
  constructor(private R: number, private turns: number, private len: number, private endTurns: number, private dwire: number) {
    super();
  }
  getPoint(t: number, target = new THREE.Vector3()) {
    // closed end turns at pitch ≈ d, the active body spread over the rest
    const total = this.turns + 2 * this.endTurns;
    const th = t * total * 2 * Math.PI;
    const turn = t * total;
    const endLen = this.endTurns * this.dwire;
    const bodyLen = Math.max(0, this.len - 2 * endLen - this.dwire);
    let y: number;
    if (turn < this.endTurns) y = turn * this.dwire;
    else if (turn > this.endTurns + this.turns) y = endLen + bodyLen + (turn - this.endTurns - this.turns) * this.dwire;
    else y = endLen + ((turn - this.endTurns) / Math.max(1e-6, this.turns)) * bodyLen;
    return target.set(this.R * Math.cos(th), y + this.dwire / 2, this.R * Math.sin(th));
  }
}

function HelixScene(props: { d: number; D: number; na: number; ends: string; L0: number; Lmin: number; Lmax: number; Ls: number }) {
  const clock = useClock(3, { loop: true });
  // scale so the free length is ~60 scene units
  const sc = 60 / Math.max(props.L0, props.D * 2);
  const phase = (1 - Math.cos((2 * Math.PI * clock.t) / 3)) / 2;
  const L = Math.max(props.Ls, props.Lmax + (props.Lmin - props.Lmax) * phase);
  const endTurns = props.ends.startsWith('squared') ? 1 : props.ends === 'plain_ground' ? 0.5 : 0;
  const geom = useMemo(() => {
    const curve = new HelixCurve((props.D / 2) * sc, props.na, L * sc, endTurns, props.d * sc);
    return new THREE.TubeGeometry(curve, Math.min(1600, Math.round((props.na + 2) * 48)), (props.d / 2) * sc, 10, false);
  }, [props.D, props.d, props.na, L, endTurns, sc]);
  useEffect(() => () => geom.dispose(), [geom]);
  const plate = (props.D + props.d) * sc * 0.75;
  return (
    <div>
      <Scene3D height={300} camera={[52, 42, 52]} target={[0, 24, 0]} grid={false}>
        <mesh geometry={geom} castShadow>
          <meshStandardMaterial color="#9aa7b8" metalness={0.75} roughness={0.32} />
        </mesh>
        <mesh position={[0, -0.6, 0]}>
          <cylinderGeometry args={[plate, plate, 1.2, 40]} />
          <meshStandardMaterial color="#3987e5" transparent opacity={0.35} />
        </mesh>
        <mesh position={[0, L * sc + 0.6, 0]}>
          <cylinderGeometry args={[plate, plate, 1.2, 40]} />
          <meshStandardMaterial color="#d95926" transparent opacity={0.35} />
        </mesh>
      </Scene3D>
      <PlayBar clock={clock} label={() => `L = ${fmt(L, 1)} mm`} />
    </div>
  );
}

// ------------------------------------------------------------------ extension

function Extension({ mats }: { mats: WireMaterial[] }) {
  const [s, setS] = useState({ material: 'music_wire', d_mm: 1.0, mean_d_mm: 7, body_coils: 14, initial_tension_n: 4, r1_mm: 3.5, r2_mm: 2.5, f_min_n: 6, f_max_n: 20, peened: false });
  const set = (q: Partial<typeof s>) => setS({ ...s, ...q });
  const { data: o, error } = useCompute<ExtensionOut>('mechanisms.extension_spring', s);
  const clock = useClock(3, { loop: true });
  const phase = (1 - Math.cos((2 * Math.PI * clock.t) / 3)) / 2;
  return (
    <div className="mx-split">
      <div className="stack" style={{ minWidth: 0 }}>
        <Card title="Helical extension spring" subtitle="Initial tension holds the coils together until it is overcome; the hooks usually fail first">
          <ErrorBox error={error} />
          {o && (
            <>
              <ExtensionSketch L0={o.free_len_mm} y={o.defl_max_mm * phase} D={s.mean_d_mm} coils={s.body_coils} />
              <PlayBar clock={clock} label={() => `F = ${fmt(s.initial_tension_n + o.rate_n_mm * o.defl_max_mm * phase, 1)} N`} />
              <div className="grid grid-4" style={{ marginTop: 12 }}>
                <Stat label="Rate k" value={fmt(o.rate_n_mm, 3)} unit="N/mm" note={`N_a = N_b + G/E = ${fmt(o.active_coils, 2)}`} />
                <Stat label="Initial-tension stress" value={fmt(o.tau_i_mpa, 0)} unit="MPa" note={`preferred ${fmt(o.tau_i_pref_lo_mpa, 0)}–${fmt(o.tau_i_pref_hi_mpa, 0)}`} tone={o.tau_i_mpa >= o.tau_i_pref_lo_mpa && o.tau_i_mpa <= o.tau_i_pref_hi_mpa ? 'pass' : 'warn'} />
                <Stat label="Hook bending σ_A" value={fmt(o.sigma_hook_a_max_mpa, 0)} unit="MPa" note={`n = ${fmt(o.ns_hook_bending, 2)}, n_f = ${fmt(o.nf_hook_bending, 2)}`} tone={sfTone(o.nf_hook_bending, 1.5)} />
                <Stat label="Body τ" value={fmt(o.tau_body_max_mpa, 0)} unit="MPa" note={`n_s ${fmt(o.ns_body, 2)}, n_f ${fmt(o.nf_body, 2)}`} tone={sfTone(o.ns_body, 1.2)} />
              </div>
            </>
          )}
        </Card>
        {o && (
          <div className="grid grid-2">
            <Card title="Force vs extension">
              <Plot series={[{ name: 'F', x: o.curve_y, y: o.curve_f }]} xLabel="extension, mm" yLabel="force, N" hLines={[{ value: s.initial_tension_n, label: 'initial tension' }]} height={210} />
            </Card>
            <Card title="Design checks">
              <Checks checks={o.checks} />
              <Notes notes={o.notes} />
            </Card>
          </div>
        )}
      </div>
      <div className="stack mx-side">
        <Card title="Spring">
          <Select label="Wire" value={s.material} options={matOptions(mats)} onChange={(material) => set({ material })} />
          <Slider label="Wire diameter d" value={s.d_mm} min={0.2} max={5} step={0.01} unit="mm" onChange={(d_mm) => set({ d_mm })} />
          <Slider label="Mean coil diameter D" value={s.mean_d_mm} min={1} max={40} step={0.1} unit="mm" onChange={(mean_d_mm) => set({ mean_d_mm })} />
          <Slider label="Body coils N_b" value={s.body_coils} min={3} max={60} step={0.5} onChange={(body_coils) => set({ body_coils })} />
          <Slider label="Initial tension F_i" value={s.initial_tension_n} min={0} max={50} step={0.1} unit="N" onChange={(initial_tension_n) => set({ initial_tension_n })} />
          <Slider label="Hook bend radius r₁" value={s.r1_mm} min={0.5} max={20} step={0.1} unit="mm" onChange={(r1_mm) => set({ r1_mm })} />
          <Slider label="Hook side radius r₂" value={s.r2_mm} min={0.5} max={20} step={0.1} unit="mm" onChange={(r2_mm) => set({ r2_mm })} />
          <Slider label="F_min" value={s.f_min_n} min={0} max={200} step={0.5} unit="N" onChange={(f_min_n) => set({ f_min_n })} />
          <Slider label="F_max" value={s.f_max_n} min={0.5} max={400} step={0.5} unit="N" onChange={(f_max_n) => set({ f_max_n })} />
        </Card>
        <Explainer title="Hooks and initial tension" refs={[{ label: 'Shigley, §10-11 (extension springs)' }]}>
          <Formula>{`σ_A = F [(K)_A 16D/(πd³) + 4/(πd²)]
(K)_A = (4C₁² − C₁ − 1)/(4C₁(C₁ − 1)),  C₁ = 2r₁/d
τ_B = (K)_B 8FD/(πd³),  (K)_B = (4C₂ − 1)/(4C₂ − 4)
τ_i preferred ≈ 231 e^(−0.105C) ± 6.9(4 − (C − 3)/6.5) MPa`}</Formula>
        </Explainer>
      </div>
    </div>
  );
}

function ExtensionSketch({ L0, y, D, coils }: { L0: number; y: number; D: number; coils: number }) {
  const W = 420, H = 110;
  const total = L0 + y;
  const sc = (W - 60) / Math.max(total * 1.25, 1);
  const x0 = 30;
  const x1 = x0 + total * sc;
  const amp = Math.min(30, (D / 2) * sc);
  const hook = Math.min(26, D * sc);
  return (
    <svg className="mx-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Extension spring stretching">
      <rect x={4} y={20} width={8} height={70} className="mx-wall" />
      <path d={`M12,55 A${hook / 2},${hook / 2} 0 1 1 ${12 + hook},55`} className="mx-wire" />
      <path d={springPath(x0 + hook * 0.4, 55, x1 - hook * 0.4, 55, Math.min(coils, 30), amp)} className="mx-wire" />
      <path d={`M${x1},55 A${hook / 2},${hook / 2} 0 1 0 ${x1 + hook},55`} className="mx-wire" />
      <line x1={x1 + hook} y1={55} x2={x1 + hook + 18} y2={55} className="mx-force" markerEnd="url(#mx-arrow)" />
      <defs>
        <marker id="mx-arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
          <path d="M0,0 L10,5 L0,10 z" fill="var(--series-2)" />
        </marker>
      </defs>
      <text x={x0} y={104}>L₀ = {fmt(L0, 1)} mm inside hooks, +{fmt(y, 1)} mm</text>
    </svg>
  );
}

// ------------------------------------------------------------------ torsion

function Torsion({ mats }: { mats: WireMaterial[] }) {
  const [s, setS] = useState({ material: 'music_wire', d_mm: 1.0, mean_d_mm: 8, body_turns: 6.25, leg1_mm: 20, leg2_mm: 20, m_min_nmm: 50, m_max_nmm: 250, arbor_d_mm: 5, peened: false });
  const set = (q: Partial<typeof s>) => setS({ ...s, ...q });
  const { data: o, error } = useCompute<TorsionOut>('mechanisms.torsion_spring', s);
  const clock = useClock(3, { loop: true });
  const phase = (1 - Math.cos((2 * Math.PI * clock.t) / 3)) / 2;
  return (
    <div className="mx-split">
      <div className="stack" style={{ minWidth: 0 }}>
        <Card title="Helical torsion spring" subtitle="Load it so it winds up: the coil diameter shrinks onto the arbor">
          <ErrorBox error={error} />
          {o && (
            <>
              <TorsionSketch angle={o.defl_max_deg * phase} D={s.mean_d_mm} leg={s.leg1_mm} />
              <PlayBar clock={clock} label={() => `M = ${fmt(o.rate_nmm_per_turn * (o.defl_max_deg * phase) / 360, 0)} N·mm`} />
              <div className="grid grid-4" style={{ marginTop: 12 }}>
                <Stat label="Rate k′" value={fmt(o.rate_nmm_per_turn, 1)} unit="N·mm/turn" note={`${fmt(o.rate_nmm_per_rad, 2)} N·mm/rad`} />
                <Stat label="Wind-up at M_max" value={fmt(o.defl_max_deg, 1)} unit="°" />
                <Stat label="Bending σ (inner fibre)" value={fmt(o.sigma_max_mpa, 0)} unit="MPa" note={`S_y ≈ ${fmt(o.sy_mpa, 0)} → n ${fmt(o.ns_static, 2)}`} tone={sfTone(o.ns_static, 1.2)} />
                <Stat label="Fatigue n_f" value={fmt(o.nf_goodman, 2)} tone={sfTone(o.nf_goodman, 1.5)} note={`ID wound ${fmt(o.id_loaded_mm, 2)} mm`} />
              </div>
            </>
          )}
        </Card>
        {o && (
          <div className="grid grid-2">
            <Card title="Moment vs wind-up">
              <Plot series={[{ name: 'M', x: o.curve_deg, y: o.curve_m }]} xLabel="wind-up, deg" yLabel="moment, N·mm" hLines={[{ value: s.m_max_nmm, label: 'M_max' }]} height={210} />
            </Card>
            <Card title="Design checks">
              <Checks checks={o.checks} />
              <Notes notes={o.notes} />
            </Card>
          </div>
        )}
      </div>
      <div className="stack mx-side">
        <Card title="Spring">
          <Select label="Wire" value={s.material} options={matOptions(mats)} onChange={(material) => set({ material })} />
          <Slider label="Wire diameter d" value={s.d_mm} min={0.2} max={5} step={0.01} unit="mm" onChange={(d_mm) => set({ d_mm })} />
          <Slider label="Mean coil diameter D" value={s.mean_d_mm} min={1} max={40} step={0.1} unit="mm" onChange={(mean_d_mm) => set({ mean_d_mm })} />
          <Slider label="Body turns N_b" value={s.body_turns} min={1} max={30} step={0.25} onChange={(body_turns) => set({ body_turns })} />
          <Slider label="Leg 1 length" value={s.leg1_mm} min={0} max={80} step={0.5} unit="mm" onChange={(leg1_mm) => set({ leg1_mm })} />
          <Slider label="Leg 2 length" value={s.leg2_mm} min={0} max={80} step={0.5} unit="mm" onChange={(leg2_mm) => set({ leg2_mm })} />
          <Slider label="M_min" value={s.m_min_nmm} min={0} max={2000} step={5} unit="N·mm" onChange={(m_min_nmm) => set({ m_min_nmm })} />
          <Slider label="M_max" value={s.m_max_nmm} min={5} max={4000} step={5} unit="N·mm" onChange={(m_max_nmm) => set({ m_max_nmm })} />
          <Slider label="Arbor diameter" value={s.arbor_d_mm} min={0} max={30} step={0.1} unit="mm" onChange={(arbor_d_mm) => set({ arbor_d_mm })} />
          <Toggle label="Shot peened" value={s.peened} onChange={(peened) => set({ peened })} />
        </Card>
        <Explainer title="Torsion springs bend, they don't twist" refs={[{ label: 'Shigley, §10-12 (helical torsion springs)' }]}>
          <Formula>{`k′ = d⁴ E / (10.8 D N_a)   [N·mm per turn]
N_a = N_b + (l₁ + l₂)/(3πD)
σ = K_i 32 M / (π d³),  K_i = (4C² − C − 1)/(4C(C − 1))
D′ = D N_b / (N_b + θ_turns)`}</Formula>
        </Explainer>
      </div>
    </div>
  );
}

function TorsionSketch({ angle, D, leg }: { angle: number; D: number; leg: number }) {
  const W = 300, H = 200;
  const cx = 150, cy = 110;
  const sc = 70 / Math.max(D / 2 + leg, 1);
  const R = (D / 2) * sc;
  const L = leg * sc;
  const a = (-angle * Math.PI) / 180;
  return (
    <svg className="mx-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Torsion spring winding up">
      {[0, 1, 2, 3].map((k) => <circle key={k} cx={cx} cy={cy} r={R - k * 0.6} className="mx-wire" opacity={0.35 + 0.15 * k} />)}
      <circle cx={cx} cy={cy} r={Math.max(2, R * 0.55)} className="mx-arbor" />
      <line x1={cx + R} y1={cy} x2={cx + R} y2={cy - L} className="mx-wire" />
      <line x1={cx - R * Math.cos(a)} y1={cy + R * Math.sin(a)} x2={cx - R * Math.cos(a) - L * Math.sin(-a)} y2={cy + R * Math.sin(a) - L * Math.cos(a)} className="mx-wire" />
      <rect x={cx + R - 6} y={cy - L - 8} width={12} height={8} className="mx-wall" />
      <text x={10} y={H - 10}>wind-up {fmt(angle, 0)}° (leg 1 fixed)</text>
    </svg>
  );
}

// ------------------------------------------------------------------ wire table

function WireTable({ mats }: { mats: WireMaterial[] }) {
  const ds = Array.from({ length: 60 }, (_, i) => 0.1 * Math.pow(100, i / 59));
  return (
    <div className="grid grid-2">
      <Card title="Spring wire table" subtitle="Shigley Tables 10-4 to 10-6: S_ut = A / d^m (A in MPa·mm^m)">
        <div className="table-wrap">
          <table className="table">
            <thead>
              <tr><th>Wire</th><th>A</th><th>m</th><th>d range</th><th>S_sy / S_ut</th><th>Notes</th></tr>
            </thead>
            <tbody>
              {mats.map((m) => (
                <tr key={m.id}>
                  <td><b>{m.name}</b><div className="muted small">{m.spec}</div></td>
                  <td className="tnum">{m.a_mpa}</td>
                  <td className="tnum">{m.m}</td>
                  <td className="tnum">{m.d_min_mm}–{m.d_max_mm} mm</td>
                  <td className="tnum">{m.ssy_frac}</td>
                  <td className="small muted" style={{ minWidth: 200 }}>{m.notes}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>
      <Card title="Thinner wire is stronger" subtitle="Cold drawing work-hardens small wire: tensile strength vs diameter">
        <Plot
          series={mats.map((m) => ({ name: m.name, x: ds, y: ds.map((d) => (d >= m.d_min_mm * 0.999 && d <= m.d_max_mm * 1.001 ? m.a_mpa / Math.pow(d, m.m) : NaN)) }))}
          xLog
          xLabel="wire diameter d, mm"
          yLabel="S_ut, MPa"
          legend
          height={230}
        />
      </Card>
    </div>
  );
}
