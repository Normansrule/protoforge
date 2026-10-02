// Accelerated life testing: acceleration factors (Arrhenius, Peck,
// Coffin–Manson, Norris–Landzberg, inverse power law), a zero/few-failure
// test planner and an animated climatic-chamber profile.
import { useMemo, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, NumberField, Select, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useElementWidth } from '../../components/useElementWidth';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { AltOut, IrisOut, PlanOut, ProfileOut } from './types';
import { fmt, prefersReducedMotion, toCsv, useRaf } from './util';

type Model = 'arrhenius' | 'peck' | 'coffin_manson' | 'norris_landzberg' | 'ipl';
const MODELS: { id: Model; label: string }[] = [
  { id: 'arrhenius', label: 'Arrhenius (temperature)' },
  { id: 'peck', label: 'Peck (temperature + humidity)' },
  { id: 'coffin_manson', label: 'Coffin–Manson (cycling)' },
  { id: 'norris_landzberg', label: 'Norris–Landzberg (solder)' },
  { id: 'ipl', label: 'Inverse power law' },
];

interface AltInput {
  model: Model; ea_ev: number; t_use_c: number; t_test_c: number; rh_use: number; rh_test: number; rh_exp: number;
  dt_use: number; dt_test: number; cm_exp: number; f_use: number; f_test: number; freq_exp: number; nl_c: number;
  tmax_use_c: number; tmax_test_c: number; s_use: number; s_test: number; ipl_exp: number;
}

export function AltTab() {
  const project = useProject((s) => s.project);
  const iris = useCompute<IrisOut>('reliability.iris', { project, samples: 0 }).data;
  const board = iris?.profile.board_c ?? 35;
  const [a, setA] = useState<AltInput>({
    model: 'arrhenius', ea_ev: 0.7, t_use_c: 35, t_test_c: 85, rh_use: 60, rh_test: 85, rh_exp: 2.66, dt_use: 20, dt_test: 80, cm_exp: 2,
    f_use: 1, f_test: 23, freq_exp: 0.136, nl_c: 2185, tmax_use_c: 35, tmax_test_c: 60, s_use: 1, s_test: 2, ipl_exp: 6.4,
  });
  const set = (p: Partial<AltInput>) => setA((x) => ({ ...x, ...p }));
  const { data: r, error } = useCompute<AltOut>('reliability.alt', a);
  const presets: { label: string; apply: Partial<AltInput> }[] = [
    { label: `Iris board: ${board.toFixed(0)} °C use → 85 °C bake`, apply: { model: 'arrhenius', ea_ev: 0.7, t_use_c: +board.toFixed(1), t_test_c: 85 } },
    { label: 'Worn 30 °C / 60 % RH → 40 °C / 93 % RH damp heat', apply: { model: 'peck', ea_ev: 0.79, rh_exp: 2.66, t_use_c: 30, rh_use: 60, t_test_c: 40, rh_test: 93 } },
    { label: 'Parts: 30 °C / 60 % → 85 °C / 85 % RH', apply: { model: 'peck', ea_ev: 0.79, rh_exp: 2.66, t_use_c: 30, rh_use: 60, t_test_c: 85, rh_test: 85 } },
    { label: 'Solder (SAC): daily 20 K swing → −20/+60 °C', apply: { model: 'norris_landzberg', dt_use: 20, dt_test: 80, cm_exp: 2.65, f_use: 1, f_test: 23, freq_exp: 0.136, nl_c: 2185, tmax_use_c: +board.toFixed(1), tmax_test_c: 60 } },
    { label: 'Solder (tin–lead constants)', apply: { model: 'norris_landzberg', dt_use: 20, dt_test: 80, cm_exp: 1.9, f_use: 1, f_test: 23, freq_exp: 1 / 3, nl_c: 1414, tmax_use_c: +board.toFixed(1), tmax_test_c: 60 } },
    { label: 'Random vibration: 2× Grms', apply: { model: 'ipl', s_use: 1, s_test: 2, ipl_exp: 6.4 } },
  ];
  const num = (label: string, key: keyof AltInput, unit?: string, step = 1) => (
    <NumberField label={label} value={a[key] as number} unit={unit} step={step} onChange={(v) => set({ [key]: v } as Partial<AltInput>)} />
  );
  return (
    <div className="stack">
      <div className="rl-layout">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card title="Acceleration factor" subtitle="How many hours (or cycles) of field use one hour (or cycle) of test represents — for one failure mechanism at a time.">
            <div className="rl-chips" role="tablist">
              {MODELS.map((m) => (
                <button key={m.id} className={`chip ${a.model === m.id ? 'active' : ''}`} onClick={() => set({ model: m.id })} aria-pressed={a.model === m.id}>{m.label}</button>
              ))}
            </div>
            <ErrorBox error={error} />
            {r && (
              <>
                <div className="grid grid-3" style={{ marginTop: 10 }}>
                  <Stat label="Acceleration factor" value={fmt(r.af, 4)} unit="×" />
                  <div className="stat" style={{ gridColumn: 'span 2' }}>
                    <div className="stat-label">Equivalence</div>
                    <div style={{ fontSize: 15, fontWeight: 600, marginTop: 4 }}>{r.equivalence}</div>
                  </div>
                </div>
                <div className="rl-factors">
                  {r.factors.map((f, i) => (
                    <div key={i} className="rl-factor">
                      <div className="small muted">{f.name}</div>
                      <div className="mono small">{f.formula}</div>
                      <div className="tnum" style={{ fontWeight: 600 }}>× {fmt(f.value, 4)}</div>
                    </div>
                  ))}
                </div>
                <Plot series={[{ name: 'acceleration factor', x: r.sweep_x, y: r.sweep_af }]} xLabel={r.sweep_label} yLabel="AF (log)" yLog height={220} hLines={[{ value: r.af, label: `current ${fmt(r.af)}×` }]} />
                {r.notes.map((n, i) => (
                  <div key={i} className="small muted" style={{ marginTop: 4 }}>• {n}</div>
                ))}
              </>
            )}
          </Card>
          <Planner af={r?.af ?? 1} model={a.model} iris={iris} />
        </div>
        <div className="stack rl-side">
          <Card title="Presets tied to Iris">
            <div className="rl-sets">
              {presets.map((p) => (
                <button key={p.label} className="rl-set" onClick={() => set(p.apply)}><b>{p.label}</b></button>
              ))}
            </div>
            <div className="small muted" style={{ marginTop: 6 }}>Iris board temperature in use: {board.toFixed(1)} °C (thermal model at the average power, from the Power lab).</div>
          </Card>
          <Card title="Model inputs">
            {a.model === 'arrhenius' && (
              <>
                <Slider label="Activation energy Ea" value={a.ea_ev} min={0.2} max={1.2} step={0.05} unit="eV" onChange={(v) => set({ ea_ev: v })} />
                {num('Use temperature', 't_use_c', '°C')}
                {num('Test temperature', 't_test_c', '°C')}
              </>
            )}
            {a.model === 'peck' && (
              <>
                <Slider label="Activation energy Ea" value={a.ea_ev} min={0.2} max={1.2} step={0.01} unit="eV" onChange={(v) => set({ ea_ev: v })} />
                <Slider label="Humidity exponent n" value={a.rh_exp} min={1} max={4} step={0.02} onChange={(v) => set({ rh_exp: v })} />
                <div className="grid grid-2">
                  {num('Use temperature', 't_use_c', '°C')}
                  {num('Use humidity', 'rh_use', '% RH')}
                  {num('Test temperature', 't_test_c', '°C')}
                  {num('Test humidity', 'rh_test', '% RH')}
                </div>
              </>
            )}
            {(a.model === 'coffin_manson' || a.model === 'norris_landzberg') && (
              <>
                <Slider label="Cycling exponent" value={a.cm_exp} min={1} max={6} step={0.05} onChange={(v) => set({ cm_exp: v })} />
                <div className="grid grid-2">
                  {num('Use swing ΔT', 'dt_use', 'K')}
                  {num('Test swing ΔT', 'dt_test', 'K')}
                </div>
                {a.model === 'norris_landzberg' && (
                  <div className="grid grid-2">
                    {num('Use cycles/day', 'f_use', '', 0.1)}
                    {num('Test cycles/day', 'f_test', '', 1)}
                    {num('Frequency exponent', 'freq_exp', '', 0.01)}
                    {num('Ea/k constant', 'nl_c', 'K', 1)}
                    {num('Use peak T', 'tmax_use_c', '°C')}
                    {num('Test peak T', 'tmax_test_c', '°C')}
                  </div>
                )}
              </>
            )}
            {a.model === 'ipl' && (
              <>
                <Slider label="Exponent n" value={a.ipl_exp} min={1} max={12} step={0.1} onChange={(v) => set({ ipl_exp: v })} />
                <div className="grid grid-2">
                  {num('Use stress', 's_use', '', 0.1)}
                  {num('Test stress', 's_test', '', 0.1)}
                </div>
              </>
            )}
          </Card>
          <Explainer title="The models" refs={[{ label: 'W. Nelson, Accelerated Testing, Wiley 1990' }, { label: 'D. S. Peck, "Comprehensive model for humidity testing correlation", Proc. IRPS 1986' }, { label: 'K. C. Norris & A. H. Landzberg, IBM J. Res. Dev. 13(3), 1969' }, { label: 'JEDEC JEP122 — failure mechanisms and models (check the current revision)' }]}>
            <Formula>{`Arrhenius  AF = exp[(Ea/k)(1/T_u − 1/T_t)]
Peck       AF = (RH_t/RH_u)^n · Arrhenius
C–M        AF = (ΔT_t/ΔT_u)^m
N–L        AF = C–M · (f_u/f_t)^p · exp[C(1/Tmax_u − 1/Tmax_t)]
IPL        AF = (S_t/S_u)^n          k = 8.617×10⁻⁵ eV/K`}</Formula>
            One model per failure mechanism: never multiply a temperature AF onto a mechanism it doesn't drive, and never push the stress past the point where a new mechanism starts (a lithium cell above about 60 °C, a plastic past its glass transition).
          </Explainer>
        </div>
      </div>
      <Chamber />
    </div>
  );
}

function Planner({ af, model, iris }: { af: number; model: Model; iris: IrisOut | null }) {
  const cycling = model === 'coffin_manson' || model === 'norris_landzberg';
  const prof = iris?.profile;
  const defaultLife = cycling ? (prof ? prof.mission_days : 730) : prof?.mission_h ?? 5840;
  const [life, setLife] = useState<number | null>(null);
  const [rel, setRel] = useState(0.9);
  const [conf, setConf] = useState(0.9);
  const [beta, setBeta] = useState(2);
  const [fails, setFails] = useState(0);
  const [units, setUnits] = useState(10);
  const [dur, setDur] = useState(500);
  const L = life ?? defaultLife;
  const perDay = cycling ? 23 : 24;
  const input = useMemo(() => ({ af, life: L, reliability: rel, confidence: conf, beta, failures: fails, units, test_duration: dur, per_day: perDay }), [af, L, rel, conf, beta, fails, units, dur, perDay]);
  const { data: p, error } = useCompute<PlanOut>('reliability.alt_plan', input);
  const tu = cycling ? 'cycles' : 'h';
  const csv = () => (p ? toCsv([['units', `test_${tu}`, 'days', 'lives'], ...p.table.map((r) => [r.units, +r.test_duration.toFixed(2), +r.days.toFixed(2), +r.lives.toFixed(3)])]) : '');
  return (
    <Card
      title="Test planner: prove a field life with few units"
      subtitle={`Demonstrate R = ${rel} at the field life with ${Math.round(conf * 100)} % confidence (Weibayes/Lipson with a known β), using the acceleration factor above (${fmt(af)}×).`}
      actions={p && <DownloadButton small filename="alt-test-plan.csv" label="plan CSV" content={csv} />}
    >
      <div className="grid grid-4">
        <NumberField label={`Field life to prove (${tu})`} value={+L.toFixed(1)} onChange={(v) => setLife(v)} />
        <Slider label="Reliability R" value={rel} min={0.5} max={0.99} step={0.01} onChange={setRel} />
        <Slider label="Confidence C" value={conf} min={0.5} max={0.99} step={0.01} onChange={setConf} />
        <Slider label="Weibull β of the mechanism" value={beta} min={0.5} max={6} step={0.1} onChange={setBeta} />
        <Slider label="Failures allowed" value={fails} min={0} max={3} step={1} format={(v) => String(Math.round(v))} onChange={(v) => setFails(Math.round(v))} />
        <Slider label="Units on test" value={units} min={1} max={60} step={1} format={(v) => String(Math.round(v))} onChange={(v) => setUnits(Math.round(v))} />
        <NumberField label={`Test length per unit (${tu})`} value={dur} onChange={setDur} />
      </div>
      <div className="small muted">Default field life: {cycling ? `${fmt(defaultLife)} daily thermal cycles` : `${fmt(defaultLife)} operating hours`} = Iris's {prof?.mission_years ?? 2}-year use profile.</div>
      <ErrorBox error={error} />
      {p && (
        <>
          <div className="grid grid-3" style={{ marginTop: 10 }}>
            <Stat label={`${units} units must each run`} value={fmt(p.duration_for_units, 4)} unit={tu} note={`${fmt(p.days_for_units, 3)} days · ${fmt(p.lives_for_units, 3)} field lives each`} />
            <Stat label={`A ${fmt(dur)} ${tu} test needs`} value={p.units_for_duration} unit="units" note={`${fmt(p.lives_for_duration, 3)} field lives each`} />
            <Stat label="Success run (1 life, no β)" value={p.success_run_units} unit="units" note="the upper bound without β" />
          </div>
          <Plot
            series={[{ name: `test length per unit (${tu})`, x: p.table.map((r) => r.units), y: p.table.map((r) => r.test_duration), scatter: false }]}
            xLabel="units on test"
            yLabel={`test ${tu} per unit`}
            yLog
            height={200}
            vLines={[{ value: units, label: `${units} units` }]}
          />
          {p.notes.map((n, i) => (
            <div key={i} className="small muted">• {n}</div>
          ))}
        </>
      )}
      <Explainer title="Trading units for time" refs={[{ label: 'C. Lipson & N. J. Sheth, Statistical Design and Analysis of Engineering Experiments, 1973' }, { label: 'R. B. Abernethy, The New Weibull Handbook, ch. 6 (Weibayes)' }]}>
        <Formula>{`n · k^β ≥ χ²(C; 2r+2) / (2 (−ln R)),   k = t_test · AF / L
r = 0:  n = ln(1 − C) / (k^β · ln R)`}</Formula>
        If a mechanism wears out (β &gt; 1), running each unit longer is worth more than adding units: with β = 2, testing two lives cuts the sample by four.
      </Explainer>
    </Card>
  );
}

function Chamber() {
  const [kind, setKind] = useState<'thermal_cycle' | 'damp_heat' | 'dry_heat' | 'cold'>('thermal_cycle');
  const [lo, setLo] = useState(-20);
  const [hi, setHi] = useState(60);
  const [ramp, setRamp] = useState(5);
  const [dwell, setDwell] = useState(15);
  const input = useMemo(() => ({ kind, t_low_c: lo, t_high_c: kind === 'damp_heat' ? 40 : hi, ramp_c_per_min: ramp, dwell_min: dwell, cycles: 3, rh: 93, hold_h: kind === 'damp_heat' ? 4 : 2 }), [kind, lo, hi, ramp, dwell]);
  const { data: pr, error } = useCompute<ProfileOut>('reliability.chamber_profile', input);
  const total = pr ? pr.t_min[pr.t_min.length - 1] : 1;
  const [t, setT] = useState(0);
  const [playing, setPlaying] = useState(!prefersReducedMotion());
  useRaf(playing && !!pr, (dt) => setT((v) => (v + dt * total / 12) % total));
  const temp = pr ? interp(pr.t_min, pr.temp_c, t) : 25;
  const rh = pr && pr.rh.some((x) => x != null) ? interp(pr.t_min, pr.rh.map((x) => x ?? 50), t) : null;
  const seg = pr?.segments.find((s) => t >= s.start_min && t <= s.end_min)?.label ?? '';
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const W = Math.max(width, 260);
  const H = 180;
  const tmin = Math.min(lo, 20) - 5;
  const tmax = Math.max(kind === 'damp_heat' ? 40 : hi, 30) + 5;
  const sx = (x: number) => 36 + (x / total) * (W - 46);
  const sy = (c: number) => 10 + (H - 36) * (1 - (c - tmin) / (tmax - tmin));
  // warm/cold tint of the chamber (series-2 = hot, series-1 = cold) mixed by temperature
  const hot = Math.max(0, Math.min(1, (temp - 25) / 60));
  const cold = Math.max(0, Math.min(1, (25 - temp) / 45));
  return (
    <Card
      title="Climatic chamber profile"
      subtitle="The set-point the chamber follows; the units inside see it with a lag. Profiles feed the DVT schedule."
      actions={
        <div className="row">
          <button className="btn small" onClick={() => setPlaying((p) => !p)} aria-pressed={playing}>{playing ? '❚❚ Pause' : '▶ Play'}</button>
          {pr && <DownloadButton small filename={`chamber-${kind}.csv`} label="profile CSV" content={() => toCsv([['minute', 'temp_c', 'rh_pct'], ...pr.t_min.map((x, i) => [x, pr.temp_c[i], pr.rh[i]])])} />}
        </div>
      }
    >
      <ErrorBox error={error} />
      <div className="rl-chamber-wrap">
        <svg viewBox="0 0 200 180" className="rl-chamber" role="img" aria-label={`Chamber at ${temp.toFixed(0)} °C${rh != null ? `, ${rh.toFixed(0)} % RH` : ''}`}>
          <rect x={8} y={8} width={184} height={164} rx={10} fill="var(--surface-3)" stroke="var(--border-strong)" />
          <rect x={20} y={20} width={130} height={140} rx={6} fill="var(--surface-1)" stroke="var(--border-strong)" />
          <rect x={20} y={20} width={130} height={140} rx={6} fill="var(--series-2)" opacity={hot * 0.45} />
          <rect x={20} y={20} width={130} height={140} rx={6} fill="var(--series-1)" opacity={cold * 0.45} />
          {[0, 1, 2].map((row) =>
            [0, 1, 2].map((col) => (
              <g key={`${row}${col}`} transform={`translate(${34 + col * 38} ${42 + row * 40})`}>
                <rect width={30} height={10} rx={3} fill="var(--surface-2)" stroke="var(--text-2)" />
                <circle cx={6} cy={5} r={2} fill="var(--good)" />
              </g>
            )),
          )}
          <line x1={20} y1={78} x2={150} y2={78} stroke="var(--border-strong)" />
          <line x1={20} y1={118} x2={150} y2={118} stroke="var(--border-strong)" />
          <rect x={160} y={30} width={22} height={60} rx={4} fill="var(--surface-2)" stroke="var(--border-strong)" />
          <text x={171} y={48} textAnchor="middle" className="rl-axis" style={{ fill: 'var(--text)' }}>{temp.toFixed(0)}°</text>
          {rh != null && <text x={171} y={66} textAnchor="middle" className="rl-axis" style={{ fill: 'var(--text)' }}>{rh.toFixed(0)}%</text>}
          <circle cx={171} cy={110} r={9} fill="none" stroke="var(--text-2)" />
          <line x1={171} y1={110} x2={171 + 8 * Math.cos(t / 3)} y2={110 + 8 * Math.sin(t / 3)} stroke="var(--text-2)" strokeWidth={2} />
          <text x={171} y={134} textAnchor="middle" className="rl-axis">fan</text>
        </svg>
        <div ref={ref} style={{ minWidth: 0 }}>
          <svg width={W} height={H} role="img" aria-label="Chamber temperature profile against time">
            {[tmin + 5, 0, 25, tmax - 5].filter((v, i, arr) => arr.indexOf(v) === i).map((c) => (
              <g key={c}>
                <line x1={36} x2={W - 10} y1={sy(c)} y2={sy(c)} stroke="var(--grid)" />
                <text x={32} y={sy(c) + 4} textAnchor="end" className="rl-axis">{c.toFixed(0)}°</text>
              </g>
            ))}
            {pr && <path d={pr.t_min.map((x, i) => `${i ? 'L' : 'M'}${sx(x).toFixed(1)},${sy(pr.temp_c[i]).toFixed(1)}`).join('')} fill="none" stroke="var(--series-2)" strokeWidth={2.2} />}
            <line x1={sx(t)} x2={sx(t)} y1={10} y2={H - 26} stroke="var(--accent)" />
            <circle cx={sx(t)} cy={sy(temp)} r={5} fill="var(--accent)" />
            <text x={36} y={H - 8} className="rl-axis">0</text>
            <text x={W - 10} y={H - 8} textAnchor="end" className="rl-axis">{(total / 60).toFixed(1)} h</text>
            <text x={(W + 26) / 2} y={H - 8} textAnchor="middle" className="rl-axis-label">{seg}</text>
          </svg>
          {pr && (
            <div className="small muted">
              {kind === 'thermal_cycle' ? `One cycle: ${pr.cycle_min.toFixed(0)} min → ${pr.cycles_per_day.toFixed(1)} cycles per day.` : `Ramp, hold and recover shown with a short hold; the DVT plan holds for the full severity.`}
            </div>
          )}
        </div>
      </div>
      <div className="grid grid-4" style={{ marginTop: 10 }}>
        <Select label="Profile" value={kind} onChange={(v) => setKind(v)} options={[
          { value: 'thermal_cycle', label: 'Change of temperature (IEC 60068-2-14)' },
          { value: 'damp_heat', label: 'Damp heat 40 °C / 93 % (IEC 60068-2-78)' },
          { value: 'dry_heat', label: 'Dry heat (IEC 60068-2-2)' },
          { value: 'cold', label: 'Cold (IEC 60068-2-1)' },
        ]} />
        <Slider label="Low temperature" value={lo} min={-40} max={10} step={1} format={(v) => String(Math.round(v))} unit="°C" onChange={setLo} />
        <Slider label="High temperature" value={hi} min={30} max={85} step={1} format={(v) => String(Math.round(v))} unit="°C" onChange={setHi} />
        <Slider label="Ramp rate" value={ramp} min={1} max={15} step={0.5} unit="°C/min" onChange={setRamp} />
        {kind === 'thermal_cycle' && <Slider label="Dwell" value={dwell} min={5} max={60} step={1} format={(v) => String(Math.round(v))} unit="min" onChange={setDwell} />}
      </div>
      <div className="small muted">Standard designations for orientation only — pick severities from your product specification and check the current editions.</div>
    </Card>
  );
}

function interp(xs: number[], ys: number[], x: number) {
  if (!xs.length) return 0;
  if (x <= xs[0]) return ys[0];
  for (let i = 1; i < xs.length; i++) {
    if (x <= xs[i]) {
      const f = (x - xs[i - 1]) / (xs[i] - xs[i - 1] || 1);
      return ys[i - 1] + f * (ys[i] - ys[i - 1]);
    }
  }
  return ys[ys.length - 1];
}
