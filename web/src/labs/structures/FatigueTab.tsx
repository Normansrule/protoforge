// Fatigue & vibration: the Iris hinge usage spectrum through Basquin S–N
// curves, Goodman mean-stress correction and Miner's rule; the temple's
// natural frequencies and animated mode shapes; a generic beam-mode
// calculator checked against the analytical cantilever.
import { useEffect, useMemo, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import type { BeamModesOut, FatigueOut, IrisOut, Section, StructMaterial } from './types';
import { fmt, prefersReducedMotion, sfTone, toCsv, useRaf } from './util';

interface Block {
  name: string;
  max_mpa: number;
  min_mpa: number;
  cycles_per_day: number;
}

function defaultBlocks(perN: number): Block[] {
  return [
    { name: 'Put on / take off (≈2.5 N spread)', max_mpa: 2.5 * perN, min_mpa: 0, cycles_per_day: 10 },
    { name: 'One-handed removal, wide spread (5 N)', max_mpa: 5 * perN, min_mpa: 0, cycles_per_day: 2 },
    { name: 'Walking bounce (±0.1 N, 1 h/day at 1.8 Hz)', max_mpa: 0.1 * perN, min_mpa: -0.1 * perN, cycles_per_day: 6480 },
  ];
}

export function FatigueTab({ iris }: { iris: IrisOut | null }) {
  return (
    <div className="stack">
      <HingeFatigue iris={iris} />
      <TempleModes iris={iris} />
      <BeamModes />
    </div>
  );
}

// ------------------------------------------------------------------ fatigue

function HingeFatigue({ iris }: { iris: IrisOut | null }) {
  const mats = useCompute<StructMaterial[]>('structures.materials', {}).data ?? [];
  const worse = useMemo(() => {
    const spread = iris?.cases.filter((c) => c.case === 'temple_spread') ?? [];
    return spread.reduce<(typeof spread)[number] | null>((a, c) => (!a || c.safety_factor < a.safety_factor ? c : a), null);
  }, [iris]);
  const perN = worse ? worse.max_vm_mpa / Math.max(1e-9, worse.force_n) : 4;
  const [blocks, setBlocks] = useState<Block[] | null>(null);
  const [material, setMaterial] = useState<string | null>(null);
  const [years, setYears] = useState(3);
  const [kf, setKf] = useState(1);
  // Seed the spectrum from the FEA once it arrives (and when the design changes).
  useEffect(() => {
    if (worse) setBlocks(defaultBlocks(perN));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [perN]);
  const mat = material ?? iris?.material ?? 'PETG';
  const input = useMemo(() => (blocks ? { material: mat, years, blocks, strength_factor: 1 / kf } : null), [blocks, mat, years, kf]);
  const { data: r, error } = useCompute<FatigueOut>('structures.fatigue', input ?? {}, !!input);
  const setBlock = (i: number, q: Partial<Block>) => setBlocks((b) => (b ? b.map((x, k) => (k === i ? { ...x, ...q } : x)) : b));
  const cyclesPerDay = blocks ? blocks.slice(0, 2).reduce((a, b) => a + b.cycles_per_day, 0) : 12;
  const csv = () =>
    r
      ? toCsv([
          ['block', 'amplitude_mpa', 'mean_mpa', 'equivalent_mpa', 'cycles', 'cycles_to_failure', 'damage'],
          ...r.blocks.map((b) => [b.name, +b.amplitude_mpa.toFixed(4), +b.mean_mpa.toFixed(4), +b.equivalent_mpa.toFixed(4), b.cycles, b.cycles_to_failure, b.damage]),
        ])
      : '';
  return (
    <div className="st-fea">
      <div className="stack" style={{ minWidth: 0 }}>
        <Card
          title="Hinge fatigue: S–N curve and Miner's rule"
          subtitle={worse ? `Stresses from the FEA: ${fmt(perN, 2)} MPa per newton of temple spread (${worse.label.toLowerCase()})` : 'Waiting for the FEA…'}
          actions={r && <DownloadButton small filename="iris-hinge-fatigue.csv" label="blocks CSV" content={csv} />}
        >
          <ErrorBox error={error} />
          {r && !r.has_data && (
            <div className="callout" style={{ borderLeftColor: 'var(--warn)' }}>
              <h4>No fatigue data for this material</h4>
              There is no S–N (stress–life) curve for this material in the library, so fatigue life can't be predicted. Pick a material with fatigue data
              (e.g. PETG, PA12, PC or aluminium) or test coupons of your own.
            </div>
          )}
          {r && r.has_data && (
            <>
              <div className="grid grid-4">
                <Stat label="Miner damage D" value={fmt(r.damage, 3)} tone={r.damage <= 0.3 ? 'pass' : r.damage <= 1 ? 'warn' : 'fail'} note={`over ${years} years (aim ≤ 0.1–0.3)`} />
                <Stat label="Predicted life" value={r.life_years > 1e6 ? '> 10⁶' : fmt(r.life_years, 1)} unit="years" />
                <Stat label="Open/close cycles to failure" value={fmt(r.life_years * 365 * cyclesPerDay, 0)} note={`${cyclesPerDay} per day`} />
                <Stat label="Stress margin" value={fmt(r.stress_margin, 2)} unit="×" tone={sfTone(r.stress_margin, 1.5)} note="stress scale to reach D = 1" />
              </div>
              <div className="grid grid-2" style={{ marginTop: 12 }}>
                <Plot
                  series={[
                    { name: `S–N (Basquin${kf > 1 ? `, ÷ K_f ${kf}` : ''})`, x: r.sn_n, y: r.sn_s },
                    { name: 'blocks (equivalent amplitude)', x: r.blocks.map((b) => Math.min(1e9, Math.max(1, b.cycles_to_failure))), y: r.blocks.map((b) => b.equivalent_mpa), scatter: true, color: 'var(--series-2)' },
                  ]}
                  xLog
                  yLog
                  xLabel="cycles to failure N"
                  yLabel="stress amplitude, MPa"
                  legend
                  height={230}
                />
                <div>
                  <GoodmanDiagram r={r} />
                  <div className="small muted" style={{ marginTop: 4 }}>Goodman diagram: blocks inside the shaded triangle survive the design cycle count.</div>
                </div>
              </div>
            </>
          )}
        </Card>
        {blocks && r && (
          <Card title="Usage spectrum" subtitle="Edit the loads and how often they happen — damage adds up block by block">
            <div className="table-wrap st-table-input">
              <table className="table">
                <thead>
                  <tr>
                    <th>Block</th>
                    <th>σ max, MPa</th>
                    <th>σ min, MPa</th>
                    <th>Cycles / day</th>
                    <th>N_f</th>
                    <th>Damage</th>
                  </tr>
                </thead>
                <tbody>
                  {blocks.map((b, i) => (
                    <tr key={i}>
                      <td style={{ minWidth: 160 }}>{b.name}</td>
                      <td>
                        <input type="number" value={+b.max_mpa.toFixed(3)} step={0.1} onChange={(e) => setBlock(i, { max_mpa: Number(e.target.value) })} aria-label={`${b.name} maximum stress`} />
                      </td>
                      <td>
                        <input type="number" value={+b.min_mpa.toFixed(3)} step={0.1} onChange={(e) => setBlock(i, { min_mpa: Number(e.target.value) })} aria-label={`${b.name} minimum stress`} />
                      </td>
                      <td>
                        <input type="number" value={b.cycles_per_day} step={1} min={0} onChange={(e) => setBlock(i, { cycles_per_day: Math.max(0, Number(e.target.value) || 0) })} aria-label={`${b.name} cycles per day`} />
                      </td>
                      <td className="tnum">{isFinite(r.blocks[i]?.cycles_to_failure ?? Infinity) ? fmt(r.blocks[i].cycles_to_failure, 2) : '∞'}</td>
                      <td className="tnum">{fmt(r.blocks[i]?.damage ?? 0, 4)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="row" style={{ marginTop: 8 }}>
              <button className="btn small" onClick={() => setBlocks(defaultBlocks(perN))}>
                Reset to the FEA spectrum
              </button>
            </div>
          </Card>
        )}
      </div>
      <div className="stack cp-sticky">
        <Card title="Material & design life">
          <Select label="Material" value={mat} options={mats.map((m) => ({ value: m.id, label: m.name }))} onChange={setMaterial} />
          <Slider label="Design life" value={years} min={0.5} max={10} step={0.5} unit="years" onChange={setYears} />
          <Slider label="Fatigue notch factor K_f" value={kf} min={1} max={3} step={0.05} onChange={setKf} hint="Divide the S-N strength by K_f for notches, layer lines or sharp corners" />
          <div className="small muted">
            K_f = 1 uses the design stress as is. The sharp pod-to-arm corner of the default Iris temple is a notch: try K_f ≈ 1.5–2.5 and watch the life fall — then fillet the corner.
          </div>
          {r && (
            <ul className="small muted" style={{ margin: '10px 0 0', paddingLeft: 18 }}>
              {r.notes.map((n, i) => (
                <li key={i}>{n}</li>
              ))}
            </ul>
          )}
        </Card>
        <Explainer
          title="Basquin, Goodman, Miner"
          refs={[
            { label: 'O. H. Basquin, "The exponential law of endurance tests", Proc. ASTM 10, 1910' },
            { label: 'M. A. Miner, "Cumulative damage in fatigue", J. Appl. Mech. 12, 1945' },
            { label: 'Budynas & Nisbett, "Shigley’s Mechanical Engineering Design", ch. 6' },
          ]}
        >
          <Formula>{`S–N:      σ_a = σ_f′ (2N)^b     (through UTS at ½ cycle and the quoted fatigue point)
Goodman:  σ_ar = σ_a / (1 − σ_m / S_u)
Miner:    D = Σ n_i / N_i,   life = duration / D`}</Formula>
          <p className="small">Polymers have no true endurance limit and fatigue data scatter by a factor of 3–10 in life: treat this as a ranking tool and confirm with a hinge cycling test.</p>
        </Explainer>
      </div>
    </div>
  );
}

function GoodmanDiagram({ r }: { r: FatigueOut }) {
  const W = 360;
  const H = 240;
  const pad = 44;
  const su = r.uts_mpa;
  const se = r.goodman_se_mpa;
  const amax = Math.max(se, ...r.blocks.map((b) => b.amplitude_mpa)) * 1.15 || 1;
  const X = (m: number) => pad + (Math.max(0, m) / su) * (W - pad - 14);
  const Y = (a: number) => H - pad + 10 - (a / amax) * (H - pad - 20);
  return (
    <svg className="st-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Goodman diagram with the usage blocks">
      <line className="ground" x1={pad} y1={Y(0)} x2={W - 10} y2={Y(0)} />
      <line className="ground" x1={pad} y1={Y(0)} x2={pad} y2={10} />
      <text x={W - 12} y={Y(0) + 28} textAnchor="end">mean stress σ_m →</text>
      <text x={pad + 4} y={18}>amplitude σ_a</text>
      <polygon points={`${X(0)},${Y(0)} ${X(0)},${Y(se)} ${X(su)},${Y(0)}`} fill="color-mix(in srgb, var(--good) 14%, transparent)" stroke="none" />
      <line x1={X(0)} y1={Y(se)} x2={X(su)} y2={Y(0)} stroke="var(--series-1)" strokeWidth={2} />
      <text x={X(0) + 4} y={Y(se) - 5}>S_e = {fmt(se, 1)} MPa at {fmt(r.design_cycles, 2)} cycles</text>
      <text x={X(su)} y={Y(0) + 14} textAnchor="end">S_u = {fmt(su, 0)}</text>
      {r.blocks.map((b, i) => (
        <g key={i}>
          <circle cx={X(b.mean_mpa)} cy={Y(b.amplitude_mpa)} r={5} fill={`var(--series-${(i % 7) + 2})`} stroke="var(--surface-1)" />
          <text x={X(b.mean_mpa) + 8} y={Y(b.amplitude_mpa) + 4}>{i + 1}</text>
        </g>
      ))}
    </svg>
  );
}

// ------------------------------------------------------------------ modes

function ModeBeam({ stations, shape, playing, freq }: { stations: number[]; shape: number[]; playing: boolean; freq: number }) {
  const [t, setT] = useState(0.25);
  // Slowed down so the eye can follow: one cycle per 1.6 s whatever the real frequency.
  useRaf(playing, (dt) => setT((v) => (v + dt / 1.6) % 1));
  const W = 380;
  const H = 140;
  const L = stations[stations.length - 1] || 1;
  const x0 = 34;
  const w = W - x0 - 20;
  const amp = Math.sin(2 * Math.PI * t) * 36;
  const pts = stations.map((s, i) => `${x0 + (s / L) * w},${70 - amp * (shape[i] ?? 0)}`).join(' ');
  const ghost = stations.map((s, i) => `${x0 + (s / L) * w},${70 + 36 * (shape[i] ?? 0)}`).join(' ');
  return (
    <svg className="st-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Animated mode shape of the temple">
      <rect x={x0 - 20} y={40} width={20} height={60} fill="var(--surface-3)" stroke="var(--text-2)" />
      <line className="ghost" x1={x0} y1={70} x2={x0 + w} y2={70} />
      <polyline className="ghost" points={ghost} fill="none" />
      <polyline points={pts} fill="none" stroke="var(--series-1)" strokeWidth={5} strokeLinecap="round" />
      <text x={x0} y={H - 10}>hinge (clamped)</text>
      <text x={x0 + w} y={H - 10} textAnchor="end">ear tip · {fmt(freq, 1)} Hz (animation slowed)</text>
    </svg>
  );
}

function TempleModes({ iris }: { iris: IrisOut | null }) {
  const m = iris?.modal;
  const [dir, setDir] = useState<'lateral' | 'vertical'>('lateral');
  const [mode, setMode] = useState(0);
  const [playing, setPlaying] = useState(!prefersReducedMotion());
  const freqs = m ? (dir === 'lateral' ? m.lateral_hz : m.vertical_hz) : [];
  const shapes = m ? (dir === 'lateral' ? m.lateral_shapes : m.vertical_shapes) : [];
  const shape = shapes[Math.min(mode, shapes.length - 1)] ?? [];
  return (
    <div className="grid grid-2">
      <Card
        title="Temple natural frequencies"
        subtitle="The right temple as an Euler–Bernoulli beam built from voxel slices of the CAD part, clamped at the hinge, electronics and lid as added mass"
        actions={
          <button className="btn small" onClick={() => setPlaying(!playing)} aria-pressed={playing}>
            {playing ? '❚❚ Pause' : '▶ Animate'}
          </button>
        }
      >
        {!m || freqs.length === 0 ? (
          <div className="muted small">Waiting for the FEA grid…</div>
        ) : (
          <>
            <ModeBeam stations={m.stations_mm} shape={shape} playing={playing} freq={freqs[mode] ?? 0} />
            <div className="cp-toolbar">
              <div className="cp-seg" role="group" aria-label="Bending direction">
                {(['lateral', 'vertical'] as const).map((d) => (
                  <button key={d} className={`cp-seg-btn ${dir === d ? 'active' : ''}`} onClick={() => setDir(d)}>
                    {d === 'lateral' ? 'sideways (spread)' : 'up–down'}
                  </button>
                ))}
              </div>
              <div className="cp-seg" role="group" aria-label="Mode">
                {freqs.map((f, i) => (
                  <button key={i} className={`cp-seg-btn ${mode === i ? 'active' : ''}`} onClick={() => setMode(i)}>
                    mode {i + 1}: {fmt(f, 0)} Hz
                  </button>
                ))}
              </div>
            </div>
            <div className="grid grid-3" style={{ marginTop: 12 }}>
              <Stat label="First mode" value={fmt(m.first_mode_hz, 1)} unit="Hz" note="lowest of the two directions" />
              <Stat label="Temple mass" value={fmt(m.total_mass_g, 1)} unit="g" note={`incl. ${fmt(m.added_mass_g, 1)} g in the pod`} />
              <Stat label="Beam segments" value={m.ei_lateral.length} note="from voxel slices" />
            </div>
          </>
        )}
      </Card>
      <Card title="Stiffness and mass along the temple" subtitle="Why the modes look like they do: a stiff, heavy pod near the hinge and a slim, light arm">
        {m && m.ei_lateral.length > 0 ? (
          <>
            <Plot
              series={[
                { name: 'EI sideways', x: m.stations_mm.slice(1), y: m.ei_lateral, step: true },
                { name: 'EI up–down', x: m.stations_mm.slice(1), y: m.ei_vertical, step: true },
              ]}
              yLog
              xLabel="distance from the hinge, mm"
              yLabel="EI, N·mm²"
              legend
              height={180}
            />
            <Plot series={[{ name: 'mass per length', x: m.stations_mm.slice(1), y: m.mass_g_mm, step: true, color: 'var(--series-3)' }]} xLabel="distance from the hinge, mm" yLabel="g/mm" height={150} yZero />
            <ul className="small muted" style={{ margin: 0, paddingLeft: 18 }}>
              {m.notes.map((n, i) => (
                <li key={i}>{n}</li>
              ))}
              <li>Walking excites 1–2 Hz and its low harmonics; a first mode far above that means the display does not visibly shake from temple flex.</li>
            </ul>
          </>
        ) : (
          <div className="muted small">—</div>
        )}
      </Card>
    </div>
  );
}

function BeamModes() {
  const [p, setP] = useState({ length_mm: 80, section: { shape: 'rect', b: 8, h: 4, t: 0 } as Section, material: 'PETG', tip_mass_g: 0, segments: 24 });
  const set = (q: Partial<typeof p>) => setP((o) => ({ ...o, ...q }));
  const mats = useCompute<StructMaterial[]>('structures.materials', {}).data ?? [];
  const { data: r, error } = useCompute<BeamModesOut>('structures.beam_modes', p);
  const [mode, setMode] = useState(0);
  const [playing, setPlaying] = useState(!prefersReducedMotion());
  return (
    <div className="st-calc">
      <Card title="Beam-mode calculator">
        <Select label="Material" value={p.material} options={mats.map((m) => ({ value: m.id, label: m.name }))} onChange={(v) => set({ material: v })} />
        <Slider label="Length" value={p.length_mm} min={10} max={200} step={1} unit="mm" onChange={(v) => set({ length_mm: v })} />
        <Slider label="Width (along the bending axis)" value={p.section.b} min={1} max={20} step={0.5} unit="mm" onChange={(v) => set({ section: { ...p.section, b: v } })} />
        <Slider label="Thickness (bending direction)" value={p.section.h} min={0.5} max={10} step={0.1} unit="mm" onChange={(v) => set({ section: { ...p.section, h: v } })} />
        <Slider label="Tip mass" value={p.tip_mass_g} min={0} max={20} step={0.5} unit="g" onChange={(v) => set({ tip_mass_g: v })} />
      </Card>
      <Card
        title="Clamped–free beam modes"
        subtitle="Finite element beam (Hermite cubics, consistent mass) against the analytical β_n L = 1.875, 4.694, 7.855"
        actions={
          <button className="btn small" onClick={() => setPlaying(!playing)} aria-pressed={playing}>
            {playing ? '❚❚ Pause' : '▶ Animate'}
          </button>
        }
      >
        <ErrorBox error={error} />
        {r && (
          <>
            <ModeBeam stations={r.stations_mm} shape={r.shapes[mode] ?? []} playing={playing} freq={r.freqs_hz[mode] ?? 0} />
            <div className="cp-toolbar">
              <div className="cp-seg" role="group" aria-label="Mode">
                {r.freqs_hz.map((_, i) => (
                  <button key={i} className={`cp-seg-btn ${mode === i ? 'active' : ''}`} onClick={() => setMode(i)}>
                    mode {i + 1}
                  </button>
                ))}
              </div>
            </div>
            <div className="table-wrap" style={{ marginTop: 10 }}>
              <table className="table">
                <thead>
                  <tr>
                    <th>Mode</th>
                    <th>Finite elements</th>
                    <th>Analytical (no tip mass)</th>
                  </tr>
                </thead>
                <tbody>
                  {r.freqs_hz.map((f, i) => (
                    <tr key={i}>
                      <td>{i + 1}</td>
                      <td className="tnum">{fmt(f, 1)} Hz</td>
                      <td className="tnum">{p.tip_mass_g > 0 ? <span className="muted">{fmt(r.analytic_hz[i], 1)} Hz</span> : `${fmt(r.analytic_hz[i], 1)} Hz`}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <Formula>{`K φ = ω² M φ        f_n = (β_n L)² / (2π) · √(E I / (μ L⁴))`}</Formula>
          </>
        )}
      </Card>
    </div>
  );
}
