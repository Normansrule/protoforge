// Thermal: an animated cross-section of the electronics pod (board → shell →
// air), the two-node RC transient with display throttling, and sweeps.
import { useMemo, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat, Toggle } from '../../components/ui';
import { Plot, type Band } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { lerpAt, PlayBar, toCSV, useClock } from '../control/kit';
import type { SweepResult, ThermalResult } from './types';
import { Kv, passWord, reqTarget, Seg, TEMP_RAMP_CSS, tempColor } from './util';
import { REFS } from './Explainers';

interface PodState { tb: number; tp: number; ta: number; qbp: number; qpa: number; p: number; throttled: boolean; duty: number }

/** Cross-section of the right-temple pod, coloured by temperature, with heat-flow particles. */
function PodSection({ s, phase, limit }: { s: PodState; phase: number; limit: number }) {
  const lo = s.ta - 1;
  const hi = Math.max(limit + 6, s.tb + 1);
  const cB = tempColor(s.tb, lo, hi);
  const cP = tempColor(s.tp, lo, hi);
  const cA = tempColor(s.ta, lo, hi);
  // dash offsets: speed ∝ heat flow
  const off1 = -(phase * (8 + s.qbp / 6)) % 1000;
  const off2 = -(phase * (8 + s.qpa / 6)) % 1000;
  const w1 = 1 + Math.min(4, s.qbp / 45);
  const w2 = 1 + Math.min(4, s.qpa / 45);
  const hot = s.tp > limit;
  return (
    <svg className="pw-pod" viewBox="0 0 640 300" role="img" aria-label={`pod cross-section: board ${s.tb.toFixed(1)} °C, shell ${s.tp.toFixed(1)} °C, ambient ${s.ta.toFixed(1)} °C`}>
      <defs>
        <radialGradient id="pw-air" cx="0.62" cy="0.55" r="0.7">
          <stop offset="0" style={{ stopColor: cP, stopOpacity: 0.35 }} />
          <stop offset="1" style={{ stopColor: cA, stopOpacity: 0.05 }} />
        </radialGradient>
        <linearGradient id="pw-skin" x1="0" x2="1">
          <stop offset="0" style={{ stopColor: 'var(--series-5)', stopOpacity: 0.25 }} />
          <stop offset="1" style={{ stopColor: cP, stopOpacity: 0.55 }} />
        </linearGradient>
        <filter id="pw-glow" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="6" /></filter>
      </defs>
      <rect x={0} y={0} width={640} height={300} fill="url(#pw-air)" />
      {/* head / skin */}
      <path d="M0,0 L118,0 C150,70 150,230 118,300 L0,300 Z" fill="url(#pw-skin)" stroke="var(--border-strong)" />
      <text x={16} y={28} fontSize={13} fontWeight={700} fill="var(--text)">wearer's temple</text>
      <text x={16} y={46} fontSize={11.5} fill="var(--text-2)">skin contact</text>
      {/* pod shell */}
      <g>
        <rect x={150} y={60} width={300} height={180} rx={42} style={{ fill: cP, transition: 'fill 0.3s' }} opacity={0.9} stroke={hot ? 'var(--bad)' : 'var(--border-strong)'} strokeWidth={hot ? 3 : 1.5} />
        <rect x={166} y={76} width={268} height={148} rx={30} fill="var(--surface-1)" opacity={0.82} />
        {/* board glow */}
        <rect x={200} y={134} width={200} height={32} rx={8} style={{ fill: cB }} filter="url(#pw-glow)" opacity={0.85} />
        {/* PCB */}
        <rect x={196} y={144} width={208} height={12} rx={3} fill="var(--mask)" stroke="var(--border-strong)" />
        {/* chips */}
        <rect x={220} y={124} width={44} height={20} rx={3} style={{ fill: cB }} stroke="var(--text-2)" strokeWidth={1} />
        <text x={242} y={138} textAnchor="middle" fontSize={10} fontWeight={700} fill="var(--text)">SoC</text>
        <rect x={280} y={130} width={26} height={14} rx={2} style={{ fill: cB }} stroke="var(--text-2)" strokeWidth={1} opacity={0.9} />
        <rect x={320} y={128} width={50} height={16} rx={2} style={{ fill: cB }} stroke="var(--text-2)" strokeWidth={1} />
        <text x={345} y={140} textAnchor="middle" fontSize={9} fill="var(--text)">display drv</text>
        <rect x={236} y={156} width={30} height={10} rx={2} fill="var(--copper)" opacity={0.8} />
        <rect x={330} y={156} width={20} height={9} rx={2} fill="var(--copper)" opacity={0.8} />
        <text x={300} y={196} textAnchor="middle" fontSize={12} fontWeight={700} fill="var(--text)">board {s.tb.toFixed(1)} °C</text>
        <text x={300} y={212} textAnchor="middle" fontSize={11} fill="var(--text-2)">{s.p.toFixed(0)} mW dissipated{s.throttled ? ' · throttled' : ''}</text>
      </g>
      {/* heat flow board → shell */}
      <g className="pw-flow" stroke="var(--series-2)" strokeDasharray="3 9" strokeDashoffset={off1} strokeWidth={w1} opacity={0.9}>
        <path d="M242,124 C242,106 240,94 238,80" />
        <path d="M345,128 C348,108 352,94 356,80" />
        <path d="M300,156 C300,176 300,200 300,222" opacity={0.0} />
        <path d="M200,150 C186,150 176,150 168,150" />
        <path d="M400,150 C414,150 424,150 432,150" />
      </g>
      {/* shell → skin and → air */}
      <g className="pw-flow" stroke="var(--bad)" strokeDasharray="4 10" strokeDashoffset={off2} strokeWidth={w2} opacity={0.75}>
        <path d="M150,150 C140,150 132,150 124,150" />
        <path d="M152,110 C140,106 134,104 128,100" />
        <path d="M152,190 C140,194 134,196 128,200" />
      </g>
      <g className="pw-flow" stroke="var(--series-4)" strokeDasharray="4 10" strokeDashoffset={off2} strokeWidth={w2} opacity={0.85}>
        <path d="M300,60 C296,40 310,24 300,4" />
        <path d="M380,64 C380,44 396,28 388,6" />
        <path d="M220,64 C220,44 206,30 214,6" />
        <path d="M450,150 C480,140 500,110 520,80 C530,64 540,50 548,30" />
        <path d="M430,226 C470,236 510,230 560,210" />
      </g>
      <text x={462} y={96} fontSize={12} fontWeight={700} fill="var(--text)">shell {s.tp.toFixed(1)} °C</text>
      <text x={462} y={112} fontSize={11} fill={hot ? 'var(--bad-text)' : 'var(--text-2)'}>{hot ? `✕ above ${limit} °C limit` : `✓ below ${limit} °C limit`}</text>
      <text x={540} y={264} fontSize={12} fontWeight={700} fill="var(--text)" textAnchor="middle">air {s.ta.toFixed(1)} °C</text>
      <text x={540} y={280} fontSize={11} fill="var(--text-2)" textAnchor="middle">convection + radiation</text>
      <text x={300} y={40} fontSize={11} fill="var(--text-2)" textAnchor="middle">{s.qpa.toFixed(0)} mW to air & skin</text>
      <text x={300} y={258} fontSize={11} fill="var(--text-2)" textAnchor="middle">{s.qbp.toFixed(0)} mW board → shell</text>
    </svg>
  );
}

/** The electrical analogy: an RC ladder driven by a current source. */
function Analogy({ r }: { r: ThermalResult }) {
  const n = r.network;
  if (!n) return null;
  return (
    <svg className="pw-analogy" viewBox="0 0 520 170" role="img" aria-label="electrical analogy RC ladder">
      {/* ground */}
      <line className="wire" x1={40} y1={140} x2={480} y2={140} />
      {/* current source */}
      <circle cx={60} cy={95} r={16} fill="none" stroke="var(--series-2)" strokeWidth={1.6} />
      <line x1={60} y1={104} x2={60} y2={86} stroke="var(--series-2)" strokeWidth={1.6} markerEnd="url(#pw-arrow)" />
      <defs><marker id="pw-arrow" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" fill="var(--series-2)" /></marker></defs>
      <line className="wire" x1={60} y1={79} x2={60} y2={40} />
      <line className="wire" x1={60} y1={111} x2={60} y2={140} />
      <text x={20} y={100}>P</text>
      <line className="wire" x1={60} y1={40} x2={170} y2={40} />
      {/* C_b */}
      <line className="wire" x1={140} y1={40} x2={140} y2={84} />
      <line x1={126} y1={84} x2={154} y2={84} stroke="var(--text-2)" strokeWidth={2} />
      <line x1={126} y1={92} x2={154} y2={92} stroke="var(--text-2)" strokeWidth={2} />
      <line className="wire" x1={140} y1={92} x2={140} y2={140} />
      <text x={100} y={115}>C_b {n.c_b} J/°C</text>
      <circle cx={140} cy={40} r={3} fill="var(--series-2)" />
      <text x={112} y={30} style={{ fill: 'var(--text)' }}>T_board</text>
      {/* R_bp */}
      <path className="wire" d="M170,40 l6,-7 l10,14 l10,-14 l10,14 l10,-14 l10,14 l6,-7" />
      <text x={176} y={62}>R_bp {n.r_bp} °C/W</text>
      <line className="wire" x1={232} y1={40} x2={330} y2={40} />
      {/* C_p */}
      <line className="wire" x1={300} y1={40} x2={300} y2={84} />
      <line x1={286} y1={84} x2={314} y2={84} stroke="var(--text-2)" strokeWidth={2} />
      <line x1={286} y1={92} x2={314} y2={92} stroke="var(--text-2)" strokeWidth={2} />
      <line className="wire" x1={300} y1={92} x2={300} y2={140} />
      <text x={260} y={115}>C_p {n.c_p} J/°C</text>
      <circle cx={300} cy={40} r={3} fill="var(--bad)" />
      <text x={272} y={30} style={{ fill: 'var(--text)' }}>T_shell</text>
      {/* R_pa */}
      <path className="wire" d="M330,40 l6,-7 l10,14 l10,-14 l10,14 l10,-14 l10,14 l6,-7" />
      <text x={336} y={62}>R_pa {n.r_pa} °C/W</text>
      <line className="wire" x1={392} y1={40} x2={450} y2={40} />
      {/* ambient source */}
      <line className="wire" x1={450} y1={40} x2={450} y2={80} />
      <line x1={436} y1={80} x2={464} y2={80} stroke="var(--text-2)" strokeWidth={2} />
      <line x1={442} y1={88} x2={458} y2={88} stroke="var(--text-2)" strokeWidth={2} />
      <line className="wire" x1={450} y1={88} x2={450} y2={140} />
      <text x={466} y={88}>T_a</text>
      <text x={40} y={162}>temperature ↔ voltage · heat flow ↔ current · °C/W ↔ Ω · J/°C ↔ F</text>
    </svg>
  );
}

function bandsFromThrottle(t: number[], thr: boolean[]): Band[] {
  const out: Band[] = [];
  let start: number | null = null;
  for (let i = 0; i < t.length; i++) {
    if (thr[i] && start === null) start = t[i];
    if (!thr[i] && start !== null) { out.push({ from: start / 60, to: t[i] / 60, color: 'color-mix(in srgb, var(--warn) 14%, transparent)' }); start = null; }
  }
  if (start !== null) out.push({ from: start / 60, to: t[t.length - 1] / 60, color: 'color-mix(in srgb, var(--warn) 14%, transparent)' });
  if (out.length) out[0].label = 'throttled';
  return out;
}

export function ThermalTab({ scenario }: { scenario: string }) {
  const project = useProject((s) => s.project);
  const patch = useProject((s) => s.patchDesign);
  const th = project.design.thermal;
  const skinReq = reqTarget(project, 'thermal.max_skin_c');
  const [ambient, setAmbient] = useState<number | null>(null);
  const amb = ambient ?? th.ambient_c;
  const [duty, setDuty] = useState(1);
  const [pulse, setPulse] = useState(0);
  const [initial, setInitial] = useState<'ambient' | 'reference'>('ambient');
  const [durMin, setDurMin] = useState(60);
  const [thrOn, setThrOn] = useState(false);
  const [thrT, setThrT] = useState(42);
  const [thrDuty, setThrDuty] = useState(0.5);
  const [hyst, setHyst] = useState(1);
  const cfg = { display_duty: duty, ambient_c: amb, duration_s: durMin * 60, dt_s: durMin > 120 ? 2 : 1, pulse_period_s: pulse, initial, throttle: { enabled: thrOn, threshold_c: thrT, hysteresis_c: hyst, throttled_duty: thrDuty } };
  const res = useCompute<ThermalResult>('power.thermal', { project, scenario, config: cfg });
  const sw = useCompute<SweepResult>('power.thermal_sweep', { project, config: { display_duty: duty, ambient_c: amb } });
  const r = res.data;
  const clock = useClock(16, { loop: true, resetKey: `${scenario}-${durMin}` });
  const tNow = r ? (clock.t / clock.duration) * r.t_s[r.t_s.length - 1] : 0;
  const state: PodState | null = r ? {
    tb: lerpAt(r.t_s, r.board_c, tNow),
    tp: lerpAt(r.t_s, r.pod_c, tNow),
    ta: r.ambient_c,
    qbp: lerpAt(r.t_s, r.q_board_pod_mw, tNow),
    qpa: lerpAt(r.t_s, r.q_pod_air_mw, tNow),
    p: lerpAt(r.t_s, r.power_mw, tNow),
    throttled: r.throttled[Math.min(r.throttled.length - 1, Math.max(0, r.t_s.findIndex((x) => x >= tNow)))] ?? false,
    duty: lerpAt(r.t_s, r.display_duty, tNow),
  } : null;
  const limit = skinReq?.target ?? th.skin_limit_c;
  const plots = useMemo(() => {
    if (!r) return null;
    const tm = r.t_s.map((t) => t / 60);
    return {
      temp: [
        { name: 'board', x: tm, y: r.board_c, color: 'var(--series-4)', width: 2.2 },
        { name: 'shell (skin contact)', x: tm, y: r.pod_c, color: 'var(--bad)', width: 2.6 },
        { name: 'ambient', x: [tm[0], tm[tm.length - 1]], y: [r.ambient_c, r.ambient_c], color: 'var(--series-1)', dash: '4 4' },
      ],
      heat: [
        { name: 'dissipated', x: tm, y: r.power_mw, color: 'var(--text-2)', step: pulse > 0 || thrOn },
        { name: 'board → shell', x: tm, y: r.q_board_pod_mw, color: 'var(--series-4)' },
        { name: 'shell → air', x: tm, y: r.q_pod_air_mw, color: 'var(--series-1)' },
      ],
      bands: bandsFromThrottle(r.t_s, r.throttled),
    };
  }, [r, pulse, thrOn]);
  const maxOk = r ? r.max_pod_c <= limit : true;
  const csv = () => r ? toCSV(['t_s', 'board_c', 'shell_c', 'power_mw', 'display_duty', 'q_board_shell_mw', 'q_shell_air_mw', 'throttled'], [r.t_s, r.board_c, r.pod_c, r.power_mw, r.display_duty, r.q_board_pod_mw, r.q_pod_air_mw, r.throttled.map((x) => (x ? '1' : '0'))]) : '';
  const suggested = r?.network ? amb + (limit - amb) * (r.network.r_pa + r.network.r_bp) / r.network.r_pa : null;

  return (
    <div className="stack">
      <ErrorBox error={res.error} />
      {r && (
        <div className="pw-stats">
          <Stat label="Max shell (this run)" value={r.max_pod_c.toFixed(1)} unit="°C" tone={maxOk ? 'pass' : 'fail'} note={`${passWord(maxOk)} ≤ ${limit} °C${skinReq ? ` (${skinReq.id})` : ''}`} />
          <Stat label="Steady shell" value={r.steady_pod_c.toFixed(1)} unit="°C" note={`board ${r.steady_board_c.toFixed(1)} °C at ${r.p_avg_mw.toFixed(0)} mW`} />
          <Stat label="Time constants" value={`${r.time_constants_s[1].toFixed(0)} s`} note={`slow (shell + air); fast ${r.time_constants_s[0].toFixed(1)} s (board)`} />
          <Stat label="Reaches limit" value={r.t_skin_limit_s !== null ? `${(r.t_skin_limit_s / 60).toFixed(1)} min` : 'never'} note={thrOn ? `throttled ${(r.throttled_fraction * 100).toFixed(0)} % of the time, mean display ${(r.mean_display_duty * 100).toFixed(0)} %` : `max safe display duty ${(r.max_safe_display_duty * 100).toFixed(0)} %`} tone={r.t_skin_limit_s !== null ? 'warn' : undefined} />
        </div>
      )}
      <div className="pw-main">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card title="Inside the temple pod" subtitle="Heat from the chips crosses the air gap to the shell, then leaves by convection and into the skin. Colours are temperatures." actions={<DownloadButton small filename="iris-thermal.csv" content={csv} label="thermal.csv" />}>
            {state && <PodSection s={state} phase={clock.t} limit={limit} />}
            <div className="pw-legend">
              <span>{(amb - 1).toFixed(0)} °C</span><span className="pw-ramp" style={{ background: TEMP_RAMP_CSS }} /><span>{Math.max(limit + 6, (r?.max_board_c ?? 0) + 1).toFixed(0)} °C</span>
              <span className="muted" style={{ marginLeft: 'auto' }}>dash speed ∝ heat flow</span>
            </div>
            <div style={{ marginTop: 10 }}>
              <PlayBar clock={clock} label={() => `${(tNow / 60).toFixed(1)} min`} />
            </div>
          </Card>
          {plots && r && (
            <Card title="Transient" subtitle={`Display at ${(duty * 100).toFixed(0)} %${pulse > 0 ? `, switching on/off every ${pulse} s` : ''} from ${initial === 'ambient' ? 'ambient (just put on)' : 'the reference-use steady state'}`}>
              <Plot series={plots.temp} height={260} xLabel="time (min)" yLabel="temperature (°C)" hLines={[{ value: limit, label: `skin limit ${limit} °C`, color: 'var(--bad)' }, ...(thrOn ? [{ value: thrT, label: `throttle at ${thrT} °C (board)`, color: 'var(--warn)' }] : [])]} vLines={[{ value: tNow / 60, color: 'var(--accent)' }]} xBands={plots.bands} legend />
              <Plot series={plots.heat} height={180} xLabel="time (min)" yLabel="heat (mW)" vLines={[{ value: tNow / 60, color: 'var(--accent)' }]} xBands={plots.bands} yZero legend />
            </Card>
          )}
          {sw.data && (
            <Card title="Steady state vs ambient and display duty" subtitle="T_shell = T_a + P·R_pa — a straight line in ambient; the display duty sets its offset.">
              <div className="grid grid-2">
                <Plot
                  series={[
                    { name: `shell, display ${(duty * 100).toFixed(0)} %`, x: sw.data.ambient_c, y: sw.data.pod_c, color: 'var(--bad)' },
                    { name: 'board', x: sw.data.ambient_c, y: sw.data.board_c, color: 'var(--series-4)', dash: '5 4' },
                    { name: 'shell, reference use', x: sw.data.ambient_c, y: sw.data.pod_ref_c, color: 'var(--series-1)' },
                  ]}
                  height={230}
                  xLabel="ambient (°C)"
                  yLabel="steady temperature (°C)"
                  hLines={[{ value: limit, label: `${limit} °C`, color: 'var(--bad)' }]}
                  vLines={[{ value: amb }, { value: sw.data.max_ambient_c, label: `max ${sw.data.max_ambient_c.toFixed(1)} °C`, color: 'var(--bad)' }]}
                  legend
                />
                <Plot
                  series={[{ name: 'shell', x: sw.data.duty.map((d) => d * 100), y: sw.data.pod_vs_duty_c, color: 'var(--bad)' }, { name: 'board', x: sw.data.duty.map((d) => d * 100), y: sw.data.board_vs_duty_c, color: 'var(--series-4)', dash: '5 4' }]}
                  height={230}
                  xLabel={`display duty (%) at ${amb} °C`}
                  yLabel="steady temperature (°C)"
                  hLines={[{ value: limit, label: `${limit} °C`, color: 'var(--bad)' }]}
                  vLines={[{ value: duty * 100 }]}
                  legend
                />
              </div>
              <div className="small muted" style={{ marginTop: 6 }}>
                With the display always on, the shell stays within {limit} °C up to <b style={{ color: 'var(--text)' }}>{sw.data.max_ambient_c.toFixed(1)} °C</b> ambient;
                at {amb} °C the display may run at most <b style={{ color: 'var(--text)' }}>{(sw.data.max_safe_duty * 100).toFixed(0)} %</b> of the time.
              </div>
            </Card>
          )}
        </div>

        <div className="stack">
          <Card title="Scenario" subtitle="Local what-ifs (R-008 is verified at the project ambient, display always on)">
            <Slider label="Ambient" value={amb} min={0} max={45} step={0.5} unit="°C" onChange={setAmbient} />
            {ambient !== null && ambient !== th.ambient_c && (
              <div className="row small" style={{ marginTop: -6, marginBottom: 8 }}>
                <button className="btn small" onClick={() => setAmbient(null)}>Back to {th.ambient_c} °C</button>
                <button className="btn small ghost" onClick={() => { patch('thermal', { ambient_c: amb }); setAmbient(null); }}>Set as project ambient</button>
              </div>
            )}
            <Slider label="Display duty" value={duty} min={0} max={1} step={0.01} format={(v) => `${(v * 100).toFixed(0)} %`} onChange={setDuty} />
            <div className="field"><span className="field-label small">Display power</span>
              <Seg value={pulse} onChange={setPulse} label="Display switching" options={[{ value: 0, label: 'average', title: 'constant average power' }, { value: 10, label: '10 s', title: 'on/off every 10 s' }, { value: 60, label: '60 s', title: 'on/off every minute' }, { value: 300, label: '5 min', title: 'on/off every 5 minutes' }]} />
            </div>
            <div className="field"><span className="field-label small">Start from</span>
              <Seg value={initial} onChange={setInitial} options={[{ value: 'ambient', label: 'ambient' }, { value: 'reference', label: 'reference use' }]} />
            </div>
            <div className="field"><span className="field-label small">Duration</span>
              <Seg value={durMin} onChange={setDurMin} options={[{ value: 20, label: '20 min' }, { value: 60, label: '1 h' }, { value: 180, label: '3 h' }]} />
            </div>
          </Card>
          <Card title="Thermal throttling" subtitle="Firmware reads the board temperature and dims or pauses the display">
            <Toggle label="Enable throttling" value={thrOn} onChange={setThrOn} />
            <Slider label="Board threshold" value={thrT} min={35} max={55} step={0.5} unit="°C" onChange={setThrT} />
            <Slider label="Throttled display duty" value={thrDuty} min={0} max={1} step={0.05} format={(v) => `${(v * 100).toFixed(0)} %`} onChange={setThrDuty} />
            <Slider label="Hysteresis" value={hyst} min={0.1} max={4} step={0.1} unit="°C" onChange={setHyst} />
            {suggested !== null && (
              <div className="small muted">
                At {amb} °C a board threshold of ≈ <b style={{ color: 'var(--text)' }}>{suggested.toFixed(1)} °C</b> corresponds to a {limit} °C shell in steady state
                (T_b − T_a = (T_p − T_a)·(R_pa + R_bp)/R_pa). Hotter days need a lower threshold — or an ambient estimate.
                <div style={{ marginTop: 6 }}><button className="btn small" onClick={() => { setThrOn(true); setThrT(Math.floor(suggested * 2) / 2); }}>Use {(Math.floor(suggested * 2) / 2).toFixed(1)} °C</button></div>
              </div>
            )}
          </Card>
          <Card title="Network (design.thermal)" subtitle="Edits the project: R-008 updates live">
            <Slider label="R shell → air" value={th.r_pod_ambient_c_per_w} min={15} max={120} step={1} unit="°C/W" onChange={(v) => patch('thermal', { r_pod_ambient_c_per_w: v })} />
            <Slider label="R board → shell" value={th.r_board_pod_c_per_w} min={2} max={80} step={1} unit="°C/W" onChange={(v) => patch('thermal', { r_board_pod_c_per_w: v })} />
            <Slider label="C shell" value={th.c_pod_j_per_c} min={1} max={30} step={0.5} unit="J/°C" onChange={(v) => patch('thermal', { c_pod_j_per_c: v })} />
            <Slider label="C board" value={th.c_board_j_per_c} min={0.2} max={10} step={0.1} unit="J/°C" onChange={(v) => patch('thermal', { c_board_j_per_c: v })} />
            <Slider label="Skin limit" value={th.skin_limit_c} min={37} max={45} step={0.5} unit="°C" onChange={(v) => patch('thermal', { skin_limit_c: v })} />
            {r && (
              <Kv rows={[
                ['Heat, display off / on', `${r.p_display_off_mw.toFixed(0)} / ${r.p_display_on_mw.toFixed(0)} mW`],
                ['A matrix (1/s)', `[${r.a_matrix[0].map((v) => v.toFixed(4)).join(', ')}; ${r.a_matrix[1].map((v) => v.toFixed(4)).join(', ')}]`],
                ['Eigenvalues', `${r.eigenvalues[0].toFixed(4)}, ${r.eigenvalues[1].toFixed(5)} 1/s`],
              ]} />
            )}
          </Card>
        </div>
      </div>

      {r && (
        <Card title="The electrical analogy" subtitle="Solve heat like a circuit: this ladder is exactly the two-node model above.">
          <Analogy r={r} />
        </Card>
      )}

      <div className="pw-explain">
        <Explainer title="Lumped thermal networks" refs={[REFS.incropera]}>
          <p>When a body is small and conducts well (Biot number Bi = hL/k ≪ 1) its temperature is nearly uniform, so it becomes one <i>node</i> with heat capacity C = m·c_p. Nodes exchange heat through thermal resistances: conduction L/(kA), convection 1/(hA).</p>
          <Formula>{'C_b dT_b/dt = P − (T_b − T_p)/R_bp\nC_p dT_p/dt = (T_b − T_p)/R_bp − (T_p − T_a)/R_pa\nsteady:  T_p = T_a + P·R_pa     T_b = T_p + P·R_bp'}</Formula>
          <p>
            For a small plastic pod in still air h ≈ 5–10 W/m²K (natural convection plus radiation), so an area of ≈ 20 cm² gives R ≈ 50–100 °C/W — the scale of Iris's {th.r_pod_ambient_c_per_w} °C/W.
          </p>
        </Explainer>
        <Explainer title="Two time constants from two eigenvalues" refs={[REFS.strang, REFS.incropera]}>
          <p>Writing x = T − T_a gives ẋ = A·x + b·P. The eigenvalues of A are real and negative; their reciprocals are the time constants:</p>
          <Formula>{r ? `λ = ${r.eigenvalues[0].toFixed(4)}, ${r.eigenvalues[1].toFixed(5)} 1/s  →  τ = ${r.time_constants_s[0].toFixed(1)} s, ${r.time_constants_s[1].toFixed(0)} s\nx(t+h) = x_ss + e^{Ah}(x(t) − x_ss)    (exact for constant P)` : ''}</Formula>
          <p>The board warms in tens of seconds; the shell — what the wearer feels — needs ≈ 5τ ≈ {r ? ((5 * r.time_constants_s[1]) / 60).toFixed(0) : '—'} minutes to settle. That lag is what makes throttling work: short bursts of full brightness are harmless.</p>
        </Explainer>
        <Explainer title="How hot may a wearable get?" refs={[REFS.iec]}>
          <p>
            Skin tolerates a warm surface for a limited time; the longer the contact, the lower the limit. Medical-device safety standards (IEC 60601-1, applied parts) are commonly cited as
            allowing 43 °C for prolonged skin contact. Iris's requirement R-008 uses <b>41 °C at 30 °C ambient with the display always on</b> for margin — the pod rests on the temple all day.
          </p>
          <p>A lumped shell temperature is an average: the spot right above the system-on-chip is hotter. Spread heat with a copper pour or graphite sheet and verify with a thermocouple or a thermal camera.</p>
        </Explainer>
        <Explainer title="Iris tie-in: design ideas">
          <p>
            {r && <>Display always on at {amb} °C: {r.p_display_on_mw.toFixed(0)} mW → shell {(r.ambient_c + r.p_display_on_mw * 1e-3 * (r.network?.r_pa ?? 0)).toFixed(1)} °C. </>}
            Levers: lower R_pa (bigger pod, more surface, vents), move heat away from the skin side, lower display power (T-2 trade: micro-OLED black pixels are free), or throttle.
            Throttling trades brightness for comfort only when it is hot — try ambient 35 °C with throttling on.
          </p>
          <div className="row" style={{ marginTop: 6 }}><Chip>R-008</Chip><Chip>SYS-PWR</Chip><Chip>SYS-FRM</Chip></div>
        </Explainer>
      </div>
    </div>
  );
}
