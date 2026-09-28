import { useEffect, useMemo, useState } from 'react';
import { useCompute } from '../../engine/hooks';
import { tryCall } from '../../engine/engine';
import { useProject } from '../../state/project';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, NumberField, Select, Slider, Stat, Toggle } from '../../components/ui';
import { Plot, fmtNum, type Series } from '../../components/Plot';
import { KV, fmt, fmtComplex, json, toCSV } from './kit';
import type { AnalyzeOut, Pid, PlantInfo, Tuning } from './types';

function parsePoly(s: string): number[] {
  return s.split(/[\s,]+/).filter(Boolean).map(Number).filter((v) => Number.isFinite(v));
}

function GainSlider(props: { label: string; value: number; onChange: (v: number) => void; base: number; enabled: boolean; onToggle?: (b: boolean) => void; unit?: string }) {
  const lo = Math.max(props.base / 300, 1e-6);
  const hi = Math.max(props.base * 300, lo * 10);
  return (
    <div>
      <div className="row" style={{ justifyContent: 'space-between', gap: 6 }}>
        {props.onToggle ? <Toggle label={props.label} value={props.enabled} onChange={props.onToggle} /> : <span className="field-label small">{props.label}</span>}
        <input type="number" value={Number(props.value.toPrecision(4))} step="any" style={{ width: 96, padding: '2px 6px' }} disabled={!props.enabled} onChange={(e) => props.onChange(Math.max(0, Number(e.target.value)))} aria-label={`${props.label} value`} />
      </div>
      {props.enabled && <Slider label="" value={Math.min(hi, Math.max(lo, props.value))} min={lo} max={hi} log onChange={props.onChange} format={() => ''} />}
    </div>
  );
}

export function PidDesigner() {
  const project = useProject((s) => s.project);
  const catalog = useCompute<PlantInfo[]>('control.plants', { project });
  const [plantId, setPlantId] = useState('mass_spring_damper');
  const info = catalog.data?.find((p) => p.id === plantId);
  const [params, setParams] = useState<Record<string, number>>({});
  const [numS, setNumS] = useState('1');
  const [denS, setDenS] = useState('1 2 1');
  const [delay, setDelay] = useState(0.2);
  const [pid, setPid] = useState<Pid>({ kp: 350, ki: 300, kd: 50, n: 20 });
  const [useI, setUseI] = useState(true);
  const [useD, setUseD] = useState(true);
  const [sat, setSat] = useState(false);
  const [uMax, setUMax] = useState(100);
  const [aw, setAw] = useState(true);
  const [dMeas, setDMeas] = useState(false);
  const [dist, setDist] = useState(false);
  const [setpoint, setSetpoint] = useState(1);
  const [tauC, setTauC] = useState<number | ''>('');
  const [tuneMsg, setTuneMsg] = useState<{ ok: boolean; text: string[] } | null>(null);

  // reset to the plant's defaults when switching plants
  useEffect(() => {
    if (!info) return;
    setParams(Object.fromEntries(info.params.map((p) => [p.key, p.value])));
    setPid(info.pid);
    setUseI(info.pid.ki > 0);
    setUseD(info.pid.kd > 0);
    setUMax(info.u_max);
    setSetpoint(info.id === 'thermal_rc' ? 5 : 1);
    setTuneMsg(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [plantId, catalog.data]);

  const plantSel = useMemo(() => ({ id: plantId, params, num: parsePoly(numS), den: parsePoly(denS), delay }), [plantId, params, numS, denS, delay]);
  const effPid: Pid = { kp: pid.kp, ki: useI ? pid.ki : 0, kd: useD ? pid.kd : 0, n: pid.n };
  const lo = info && info.u_min >= 0 ? 0 : -uMax;
  const res = useCompute<AnalyzeOut>('control.analyze', {
    plant: plantSel,
    pid: effPid,
    project,
    sim: { setpoint, u_max: sat ? uMax : null, u_min: sat ? lo : null, anti_windup: aw, d_on_measurement: dMeas, dist_time: dist ? null : null, dist_value: 0, samples: 1000 },
  });
  // disturbance needs the horizon: second pass once we know t_end
  const tEnd = res.data?.t_end;
  const res2 = useCompute<AnalyzeOut>('control.analyze', {
    plant: plantSel,
    pid: effPid,
    project,
    sim: { setpoint, u_max: sat ? uMax : null, u_min: sat ? lo : null, anti_windup: aw, d_on_measurement: dMeas, dist_time: tEnd ? tEnd * 0.6 : null, dist_value: -distMag(res.data?.sim.u ?? []), samples: 1000, t_end: tEnd ? tEnd * 1.2 : null },
  }, dist && !!tEnd);
  const d = dist && res2.data ? res2.data : res.data;

  const tune = (method: string) => {
    const r = tryCall<Tuning>('control.tune', { plant: plantSel, method, tau_c: tauC === '' ? null : tauC, n: pid.n, project });
    if (r.error || !r.data) {
      setTuneMsg({ ok: false, text: [r.error?.replace(/^control\.tune: /, '') ?? 'failed'] });
      return;
    }
    setPid(r.data.pid);
    setUseI(r.data.pid.ki > 0);
    setUseD(r.data.pid.kd > 0);
    setTuneMsg({ ok: true, text: [`${r.data.method}: Kp = ${fmtNum(r.data.pid.kp)}, Ki = ${fmtNum(r.data.pid.ki)}, Kd = ${fmtNum(r.data.pid.kd)}`, ...r.data.notes] });
  };

  const m = d?.metrics;
  const mg = d?.margins;
  const stepSeries = useMemo<Series[]>(() => (d ? [
    { name: 'output y', x: d.sim.t, y: d.sim.y, color: 'var(--series-1)' },
    { name: 'reference r', x: d.sim.t, y: d.sim.r, color: 'var(--muted)', dash: '5 4', width: 1.4 },
  ] : []), [d]);
  const uSeries = useMemo<Series[]>(() => (d ? [{ name: 'control u', x: d.sim.t, y: d.sim.u, color: 'var(--series-2)' }] : []), [d]);
  const openSeries = useMemo<Series[]>(() => (d ? [{ name: 'plant step (open loop)', x: d.open_step.t, y: d.open_step.y, color: 'var(--series-3)' }] : []), [d]);
  const magSeries = useMemo<Series[]>(() => (d ? [
    { name: 'loop L = C·G', x: d.bode_loop.w, y: d.bode_loop.mag_db, color: 'var(--series-1)' },
    { name: 'plant G', x: d.bode_plant.w, y: d.bode_plant.mag_db, color: 'var(--series-3)', dash: '5 4', width: 1.4 },
    { name: 'closed loop T', x: d.bode_closed.w, y: d.bode_closed.mag_db, color: 'var(--series-4)', width: 1.4 },
  ] : []), [d]);
  const phSeries = useMemo<Series[]>(() => (d ? [
    { name: 'loop L', x: d.bode_loop.w, y: d.bode_loop.phase_deg, color: 'var(--series-1)' },
    { name: 'plant G', x: d.bode_plant.w, y: d.bode_plant.phase_deg, color: 'var(--series-3)', dash: '5 4', width: 1.4 },
  ] : []), [d]);
  const rlSeries = useMemo<Series[]>(() => {
    if (!d) return [];
    const rl = d.root_locus;
    const s: Series[] = rl.branches.map((b, i) => ({ name: `branch ${i + 1}`, x: b.map((z) => z.re), y: b.map((z) => z.im), color: 'var(--series-1)', width: 1.6 }));
    s.push({ name: 'open-loop poles (k→0)', x: rl.open_poles.map((z) => z.re), y: rl.open_poles.map((z) => z.im), color: 'var(--series-3)', scatter: true });
    if (rl.open_zeros.length) s.push({ name: 'zeros (k→∞)', x: rl.open_zeros.map((z) => z.re), y: rl.open_zeros.map((z) => z.im), color: 'var(--series-4)', scatter: true });
    s.push({ name: 'closed-loop poles now', x: rl.current.map((z) => z.re), y: rl.current.map((z) => z.im), color: 'var(--series-2)', scatter: true });
    return s;
  }, [d]);
  const rlDomain = useMemo<{ x: [number, number]; y: [number, number] } | undefined>(() => {
    if (!d) return undefined;
    const pts = [...d.root_locus.open_poles, ...d.root_locus.open_zeros, ...d.root_locus.current];
    let r = Math.max(1, ...pts.map((z) => Math.hypot(z.re, z.im)));
    r *= 1.6;
    return { x: [-r, Math.max(r * 0.35, 1)], y: [-r * 0.8, r * 0.8] };
  }, [d]);
  const nySeries = useMemo<Series[]>(() => (d ? [
    { name: 'L(jω), ω > 0', x: d.nyquist.re, y: d.nyquist.im, color: 'var(--series-1)' },
    { name: 'L(jω), ω < 0', x: d.nyquist.re, y: d.nyquist.im.map((v) => -v), color: 'var(--series-1)', dash: '4 4', width: 1.2 },
    { name: '−1 point', x: [-1], y: [0], color: 'var(--bad)', scatter: true },
  ] : []), [d]);

  const vl = [
    ...(mg?.w_gc ? [{ value: mg.w_gc, label: `ω_gc ${fmtNum(mg.w_gc)}`, color: 'var(--series-1)' }] : []),
    ...(mg?.w_pc ? [{ value: mg.w_pc, label: `ω_180 ${fmtNum(mg.w_pc)}`, color: 'var(--series-2)' }] : []),
  ];

  const stepCsv = () => (d ? toCSV(['t_s', 'y', 'u', 'r'], [d.sim.t, d.sim.y, d.sim.u, d.sim.r]) : '');
  const bodeCsv = () => (d ? toCSV(['w_rad_s', 'L_mag_db', 'L_phase_deg', 'G_mag_db', 'G_phase_deg', 'T_mag_db'], [d.bode_loop.w, d.bode_loop.mag_db, d.bode_loop.phase_deg, d.bode_plant.mag_db, d.bode_plant.phase_deg, d.bode_closed.mag_db]) : '');
  const gains = () => json({ plant: plantSel, pid: effPid, controller_tf: d?.controller, closed_loop_tf: d?.closed_loop, margins: d?.margins, step_metrics: d?.metrics, note: 'C(s) = Kp + Ki/s + Kd s/(Tf s + 1), Tf = Kd/(Kp N)' });

  return (
    <div className="grid grid-sidebar">
      <div className="stack">
        <Card title="Plant">
          <Select label="Plant" value={plantId} onChange={setPlantId} options={[...(catalog.data ?? []).map((p) => ({ value: p.id, label: p.name })), { value: 'custom', label: 'Custom transfer function' }]} />
          {info && <p className="small muted" style={{ marginTop: 0 }}>{info.description}</p>}
          {info && <div className="formula" style={{ whiteSpace: 'pre-wrap' }}>{info.formula}</div>}
          {info?.params.map((p) => (
            <Slider key={p.key} label={p.label} unit={p.unit} value={params[p.key] ?? p.value} min={p.min} max={p.max} log={p.log} onChange={(v) => setParams({ ...params, [p.key]: v })} />
          ))}
          {plantId === 'custom' && (
            <>
              <label className="field"><span className="field-top"><span className="field-label">Numerator (highest power first)</span></span><input type="text" value={numS} onChange={(e) => setNumS(e.target.value)} /></label>
              <label className="field"><span className="field-top"><span className="field-label">Denominator</span></span><input type="text" value={denS} onChange={(e) => setDenS(e.target.value)} /></label>
              <Slider label="Dead time" value={delay} min={0} max={5} step={0.01} unit="s" onChange={setDelay} />
            </>
          )}
          {info && <div className="small" style={{ color: 'var(--text-2)' }}><b>Iris:</b> {info.iris}</div>}
        </Card>
        <Card title="PID controller" subtitle="C(s) = Kp + Ki/s + Kd·s/(Tf·s + 1),  Tf = Kd/(Kp·N)">
          <GainSlider label="Kp" value={pid.kp} base={info?.pid.kp || 1} enabled onChange={(v) => setPid({ ...pid, kp: v })} />
          <GainSlider label="Ki" value={pid.ki} base={info?.pid.ki || 1} enabled={useI} onToggle={setUseI} onChange={(v) => setPid({ ...pid, ki: v })} />
          <GainSlider label="Kd" value={pid.kd} base={info?.pid.kd || 1} enabled={useD} onToggle={setUseD} onChange={(v) => setPid({ ...pid, kd: v })} />
          {useD && <Slider label="Derivative filter N" value={pid.n} min={2} max={100} log onChange={(v) => setPid({ ...pid, n: v })} />}
          <NumberField label="Set-point step" value={setpoint} onChange={setSetpoint} />
          <Toggle label="Actuator saturation" value={sat} onChange={setSat} />
          {sat && <Slider label={`Limit ${info && info.u_min >= 0 ? '0 …' : '±'}`} value={uMax} min={(info?.u_max ?? 100) / 200} max={(info?.u_max ?? 100) * 2} log onChange={setUMax} />}
          {sat && <Toggle label="Anti-windup (back-calculation)" value={aw} onChange={setAw} />}
          <Toggle label="Derivative on measurement (no kick)" value={dMeas} onChange={setDMeas} />
          <Toggle label="Load disturbance step" value={dist} onChange={setDist} />
        </Card>
        <Card title="Autotune">
          <div className="row" style={{ marginBottom: 8 }}>
            <button className="btn small" onClick={() => tune('zn_pid')}>Ziegler–Nichols PID</button>
            <button className="btn small" onClick={() => tune('zn_pi')}>ZN PI</button>
            <button className="btn small" onClick={() => tune('simc_pi')}>SIMC PI</button>
            <button className="btn small primary" onClick={() => tune('simc_pid')}>SIMC PID</button>
          </div>
          <label className="field"><span className="field-top"><span className="field-label">SIMC τc (closed-loop time constant, blank = θ)</span></span><input type="number" step="any" value={tauC} onChange={(e) => setTauC(e.target.value === '' ? '' : Number(e.target.value))} /></label>
          {tuneMsg && (
            <div className="callout small" style={{ borderLeftColor: tuneMsg.ok ? 'var(--good)' : 'var(--bad)' }}>
              {tuneMsg.text.map((t, i) => <div key={i}>{i === 0 && !tuneMsg.ok ? '✕ ' : ''}{t}</div>)}
            </div>
          )}
        </Card>
      </div>

      <div className="stack">
        <ErrorBox error={res.error} />
        <div className="grid grid-4">
          <Stat label="Closed loop" value={d ? (d.stable ? '✓ stable' : '✕ unstable') : '—'} tone={d ? (d.stable ? 'pass' : 'fail') : undefined} note={d ? `max Re(p) = ${fmt(Math.max(...d.closed_poles.map((p) => p.re)), 3)}` : ''} />
          <Stat label="Overshoot" value={fmt(m?.overshoot_pct, 1)} unit="%" note={`rise ${fmt(m?.rise_time, 3)} s · peak @ ${fmt(m?.peak_time, 3)} s`} />
          <Stat label="Settling (2 %)" value={fmt(m?.settling_time, 3)} unit="s" note={`steady-state error ${fmt(m?.ss_error, 4)}`} />
          <Stat label="Margins" value={<span>{fmt(mg?.pm_deg, 1)}°</span>} unit={`PM · GM ${mg?.gm_db != null ? fmt(mg.gm_db, 1) + ' dB' : '∞'}`} note={`Ms = ${fmt(d?.ms, 2)} · delay margin ${fmt(mg?.delay_margin, 3)} s`} />
        </div>
        <Card title="Closed-loop step response" subtitle={d?.sim.saturated_frac ? `Actuator saturated ${(d.sim.saturated_frac * 100).toFixed(0)} % of the time` : 'Exact ZOH plant + PID updated every step'} actions={<><DownloadButton filename={`step-${plantId}.csv`} content={stepCsv} label="CSV" small /><DownloadButton filename={`pid-${plantId}.json`} content={gains} label="Gains JSON" small /></>}>
          <Plot series={stepSeries} height={250} xLabel="time (s)" yLabel={info?.output ?? 'y'} hLines={m?.final_value != null ? [{ value: m.final_value * 1.02, color: 'var(--grid)' }, { value: m.final_value * 0.98, label: '±2 % band', color: 'var(--grid)' }] : []} vLines={m?.settling_time ? [{ value: m.settling_time, label: 'settled', color: 'var(--good)' }] : []} />
          <Plot series={uSeries} height={150} xLabel="time (s)" yLabel={`u (${info?.input ?? 'u'})`} legend={false} />
        </Card>
        <div className="grid grid-2">
          <Card title="Bode — magnitude" subtitle="Margins read off the loop transfer function L = C·G" actions={<DownloadButton filename={`bode-${plantId}.csv`} content={bodeCsv} label="CSV" small />}>
            <Plot series={magSeries} height={230} xLog xLabel="ω (rad/s)" yLabel="|·| (dB)" hLines={[{ value: 0, label: '0 dB' }]} vLines={vl} />
          </Card>
          <Card title="Bode — phase">
            <Plot series={phSeries} height={230} xLog xLabel="ω (rad/s)" yLabel="phase (°)" hLines={[{ value: -180, label: '−180°', color: 'var(--bad)' }]} vLines={vl} />
          </Card>
          <Card title="Root locus of 1 + k·C·G" subtitle={`Branches for k = 0 … 1000× the current gains · ● = poles at k = 1${d?.root_locus.k_unstable ? ` · unstable beyond k ≈ ${fmtNum(d.root_locus.k_unstable)}` : ''}`}>
            <Plot series={rlSeries} height={260} xLabel="Re(s)" yLabel="Im(s)" legend={false} xDomain={rlDomain?.x} yDomain={rlDomain?.y} vLines={[{ value: 0, color: 'var(--axis)' }]} hLines={[{ value: 0, color: 'var(--axis)' }]} />
            <div className="plot-legend" style={{ paddingLeft: 0 }}>
              <span className="lg"><span className="lg-swatch" style={{ background: 'var(--series-1)' }} />locus branches</span>
              <span className="lg"><span className="lg-swatch" style={{ background: 'var(--series-3)', height: 8, width: 8, borderRadius: 8 }} />open-loop poles</span>
              <span className="lg"><span className="lg-swatch" style={{ background: 'var(--series-4)', height: 8, width: 8, borderRadius: 8 }} />zeros</span>
              <span className="lg"><span className="lg-swatch" style={{ background: 'var(--series-2)', height: 8, width: 8, borderRadius: 8 }} />closed-loop poles now</span>
            </div>
          </Card>
          <Card title="Nyquist" subtitle="Stable if the curve's encirclements of −1 match the open-loop RHP poles">
            <Plot series={nySeries} height={260} xLabel="Re L(jω)" yLabel="Im L(jω)" legend={false} vLines={[{ value: 0, color: 'var(--axis)' }]} hLines={[{ value: 0, color: 'var(--axis)' }]} />
          </Card>
        </div>
        <div className="grid grid-2">
          <Card title="Plant open-loop step">
            <Plot series={openSeries} height={180} xLabel="time (s)" yLabel={info?.output ?? 'y'} legend={false} />
          </Card>
          <Card title="Poles & transfer functions">
            <KV rows={[
              ['plant poles', d?.plant_poles.map((z) => fmtComplex(z, 3)).join(', ') ?? '—'],
              ['plant zeros', d?.plant_zeros.length ? d.plant_zeros.map((z) => fmtComplex(z, 3)).join(', ') : 'none'],
              ['closed-loop poles', d?.closed_poles.map((z) => fmtComplex(z, 3)).join(', ') ?? '—'],
              ['bandwidth (−3 dB)', d?.bandwidth ? `${fmtNum(d.bandwidth)} rad/s` : '—'],
              ['plant dead time', d ? `${fmt(d.plant.delay, 3)} s` : '—'],
            ]} />
          </Card>
        </div>
        <div className="grid grid-2">
          <Explainer title="Reading the loop" refs={[{ label: 'Åström & Murray, Feedback Systems, ch. 9–11', href: 'https://fbswiki.org' }, { label: 'Franklin, Powell & Emami-Naeini, Feedback Control of Dynamic Systems' }]}>
            <p>The closed loop is <span className="mono">T = CG/(1 + CG)</span>. Its poles are the roots of <span className="mono">1 + k·C(s)G(s) = 0</span>; the root locus traces them as the overall gain k grows — they start at the open-loop poles and end at the zeros or run off along asymptotes.</p>
            <Formula>{`Phase margin  PM = 180° + ∠L(jω_gc),  |L(jω_gc)| = 1
Gain margin   GM = 1 / |L(jω_180)|,  ∠L(jω_180) = −180°
Delay margin  = PM / ω_gc   (extra dead time the loop tolerates)
2nd-order:    overshoot = exp(−ζπ/√(1−ζ²)),  PM ≈ 100·ζ  (deg)`}</Formula>
            <p>Dead time adds phase −ωθ without changing gain — watch the FOPDT plant's phase plunge. The sensitivity peak <span className="mono">Ms = max|1/(1+L)|</span> is a single robustness number: 1.2–2 is healthy.</p>
          </Explainer>
          <Explainer title="Autotuning rules" refs={[{ label: 'Ziegler & Nichols, Trans. ASME 1942' }, { label: 'S. Skogestad, J. Process Control 13 (2003)' }]}>
            <p><b>Ziegler–Nichols</b>: turn up a P controller until the loop oscillates — gain Ku (= the gain margin), period Pu = 2π/ω₁₈₀ — then Kp = 0.6Ku, Ti = Pu/2, Td = Pu/8. Fast but aggressive (~quarter-decay).</p>
            <p><b>SIMC</b>: reduce the plant to k·e^(−θs)/((τ₁s+1)(τ₂s+1)) with the half rule, then</p>
            <Formula>{`Kc = τ₁ / (k (τc + θ)),  τI = min(τ₁, 4(τc + θ)),  τD = τ₂`}</Formula>
            <p>One knob, τc (default θ), trades speed for robustness. Neither rule works for the unstable pendulum — that is what the LQR tab is for. Anti-windup matters whenever the actuator saturates: try the thermal plant (heat can't go below 0 W) with and without it.</p>
          </Explainer>
        </div>
      </div>
    </div>
  );
}

/** Load-disturbance size: half the steady control effort (or 10 % of the peak). */
function distMag(u: number[]) {
  if (u.length === 0) return 0;
  const uf = Math.abs(u[u.length - 1]);
  const pk = Math.max(...u.map(Math.abs));
  return Math.max(0.5 * uf, 0.1 * pk);
}
