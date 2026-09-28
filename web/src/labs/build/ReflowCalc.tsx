// Reflow profile designer: zone setpoints + conveyor speed → lumped board
// temperatures, animated board travelling through the oven, live plot vs
// the alloy's windows, pass/fail checks.
import { useMemo, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { PlayBar, frameAt, toCSV, useClock } from '../control/kit';
import { heat } from './illustrations';
import type { Alloy, ReflowOut } from './types';

function Oven({ o, t, zones, zoneLen }: { o: ReflowOut; t: number; zones: number[]; zoneLen: number }) {
  const lead = 100, tail = 600;
  const total = lead + o.oven_length_mm + tail;
  const W = 360;
  const sx = (x: number) => 8 + ((x + lead) / total) * (W - 16);
  const k = frameAt(o.t_s, t);
  const x = o.x_mm[k] ?? 0;
  const ts = o.small_c[k] ?? 25, tl = o.large_c[k] ?? 25;
  return (
    <svg viewBox={`0 0 ${W} 150`} className="bd-anim" role="img" aria-label="Reflow oven zones with the board on the conveyor">
      {zones.map((z, i) => (
        <g key={i}>
          <rect x={sx(i * zoneLen)} y={30} width={sx((i + 1) * zoneLen) - sx(i * zoneLen) - 1} height={70} fill={heat(z)} opacity={0.8} />
          <text x={(sx(i * zoneLen) + sx((i + 1) * zoneLen)) / 2} y={24} textAnchor="middle" className="bd-svg-small">Z{i + 1} {Math.round(z)}°</text>
        </g>
      ))}
      <rect x={sx(0)} y={28} width={sx(o.oven_length_mm) - sx(0)} height={74} fill="none" stroke="var(--border-strong)" />
      <line x1={8} y1={96} x2={W - 8} y2={96} stroke="var(--text-2)" strokeWidth={2} />
      <g transform={`translate(${sx(x)},0)`}>
        <rect x={-14} y={86} width={28} height={6} fill="var(--mask)" />
        <circle cx={-7} cy={83} r={3.2} fill={heat(ts)} stroke="var(--text)" strokeWidth={0.6} />
        <rect x={3} y={79} width={8} height={7} fill={heat(tl)} stroke="var(--text)" strokeWidth={0.6} />
      </g>
      <text x={8} y={122} className="bd-svg-small">t = {t.toFixed(0)} s · small part {ts.toFixed(0)} °C · large part {tl.toFixed(0)} °C · air {(o.air_c[k] ?? 25).toFixed(0)} °C</text>
      <text x={8} y={140} className="bd-svg-small">oven {(o.oven_length_mm / 1000).toFixed(2)} m · {o.dwell_per_zone_s.toFixed(0)} s per zone</text>
    </svg>
  );
}

export function ReflowCalc() {
  const alloys = useCompute<Alloy[]>('build.reflow_alloys', {});
  const [alloy, setAlloy] = useState('SAC305');
  const [zones, setZones] = useState<number[]>([110, 175, 190, 240, 250, 90]);
  const [speed, setSpeed] = useState(52);
  const [zoneLen, setZoneLen] = useState(450);
  const [tauS, setTauS] = useState(30);
  const [tauL, setTauL] = useState(42);
  const r = useCompute<ReflowOut>('build.reflow', { alloy, zones, conveyor_cm_min: speed, zone_length_mm: zoneLen, tau_small_s: tauS, tau_large_s: tauL, sample_s: 1 });
  const o = r.data;
  const dur = o ? o.t_s[o.t_s.length - 1] : 400;
  const clock = useClock(dur, { speed: 16, resetKey: `${alloy}` });
  const a = o?.alloy;
  const pickAlloy = (id: string) => {
    const al = alloys.data?.find((x) => x.id === id);
    setAlloy(id);
    if (al) {
      setZones(al.default_zones);
      setSpeed(al.default_speed_cm_min);
    }
  };
  const series = useMemo(() => {
    if (!o) return [];
    const k = frameAt(o.t_s, clock.t) + 1;
    return [
      { name: 'oven air at the board', x: o.t_s, y: o.air_c, dash: '4 3', color: 'var(--muted)' },
      { name: 'small part (0402)', x: o.t_s.slice(0, k), y: o.small_c.slice(0, k), color: 'var(--series-2)' },
      { name: 'large part (QFN)', x: o.t_s.slice(0, k), y: o.large_c.slice(0, k), color: 'var(--series-1)' },
    ];
  }, [o, clock.t]);
  const labels = o ? Array.from(new Set(o.checks.map((c) => c.label))) : [];
  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Oven recipe" subtitle="Air setpoints per zone">
          <Select label="Solder paste alloy" value={alloy} onChange={pickAlloy} options={(alloys.data ?? []).map((x) => ({ value: x.id, label: x.name }))} />
          {zones.map((z, i) => (
            <Slider key={i} label={`Zone ${i + 1}${i === zones.length - 1 ? ' (cooling)' : ''}`} unit="°C" value={z} min={25} max={320} step={1} onChange={(v) => setZones(zones.map((q, j) => (j === i ? v : q)))} format={(v) => v.toFixed(0)} />
          ))}
          <div className="row" style={{ gap: 6, marginBottom: 8 }}>
            <button className="btn small" disabled={zones.length <= 4} onClick={() => setZones(zones.slice(0, -2).concat(zones.slice(-1)))}>− zone</button>
            <button className="btn small" disabled={zones.length >= 8} onClick={() => setZones([...zones.slice(0, -1), zones[zones.length - 2] ?? 200, zones[zones.length - 1]])}>+ zone</button>
            <button className="btn small" onClick={() => pickAlloy(alloy)}>Reset</button>
          </div>
          <Slider label="Conveyor speed" unit="cm/min" value={speed} min={20} max={140} step={1} onChange={setSpeed} format={(v) => v.toFixed(0)} />
          <Slider label="Zone length" unit="mm" value={zoneLen} min={200} max={700} step={10} onChange={setZoneLen} format={(v) => v.toFixed(0)} />
          <Slider label="τ small part" unit="s" value={tauS} min={10} max={80} step={1} onChange={setTauS} format={(v) => v.toFixed(0)} hint="m·c_p/(h·A)" />
          <Slider label="τ large part / heavy copper" unit="s" value={tauL} min={15} max={120} step={1} onChange={setTauL} format={(v) => v.toFixed(0)} />
        </Card>
        <div className="stack">
          <ErrorBox error={r.error} />
          {o && a && (
            <>
              <Card title="Board through the oven" subtitle={o.pass ? '✓ every window met on both thermocouples' : '✕ some windows missed — see the checks'} actions={<Chip tone={o.pass ? 'pass' : 'fail'}>{o.pass ? '✓ profile passes' : '✕ profile fails'}</Chip>}>
                <Oven o={o} t={clock.t} zones={zones} zoneLen={zoneLen} />
                <PlayBar clock={clock} speeds={[4, 8, 16, 32]} label={(t) => `${t.toFixed(0)} s`} />
              </Card>
              <Card title="Temperature vs time" subtitle={`${a.name} · liquidus ${a.liquidus_c} °C · max ΔT across the board ${o.max_delta_t_c.toFixed(1)} °C`}>
                <Plot
                  series={series}
                  height={280}
                  xLabel="time (s)"
                  yLabel="°C"
                  xDomain={[0, dur]}
                  yDomain={[0, 300]}
                  legend
                  hLines={[
                    { value: a.liquidus_c, label: `liquidus ${a.liquidus_c}`, color: 'var(--series-8)' },
                    { value: a.soak_lo_c, label: `soak ${a.soak_lo_c}`, color: 'var(--series-4)' },
                    { value: a.soak_hi_c, label: `soak ${a.soak_hi_c}`, color: 'var(--series-4)' },
                    { value: a.peak_c[1], label: `peak max ${a.peak_c[1]}`, color: 'var(--bad)' },
                  ]}
                  vLines={[{ value: clock.t, label: '' }, ...o.zone_entry_s.map((z, i) => ({ value: z, label: `Z${i + 1}`, color: 'var(--grid)' }))]}
                />
              </Card>
            </>
          )}
        </div>
      </div>
      {o && a && (
        <div className="grid grid-2">
          <Card title="Profile checks" subtitle="Windows consistent with J-STD-020 and typical paste datasheets — your paste's datasheet wins">
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Window</th><th>Target</th><th>Small part</th><th>Large part</th></tr></thead>
                <tbody>
                  {labels.map((l) => {
                    const cs = o.checks.filter((c) => c.label === l);
                    const c0 = cs[0];
                    const tgt = c0.lo != null && c0.hi != null ? `${c0.lo}–${c0.hi}` : c0.hi != null ? `≤ ${c0.hi}` : `≥ ${c0.lo}`;
                    return (
                      <tr key={l}>
                        <td>{l}</td>
                        <td className="tnum small">{tgt} {c0.unit}</td>
                        {cs.map((c) => <td key={c.probe} className="tnum"><Chip tone={c.pass ? 'pass' : 'fail'}>{c.pass ? '✓' : '✕'} {c.value.toFixed(c.unit === '°C/s' ? 2 : 0)}</Chip></td>)}
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
            <div className="small muted" style={{ marginTop: 6 }}>{a.notes}</div>
            <DownloadButton filename={`reflow-${alloy}.csv`} content={() => toCSV(['t_s', 'x_mm', 'air_c', 'small_c', 'large_c'], [o.t_s, o.x_mm, o.air_c, o.small_c, o.large_c])} label={`reflow-${alloy}.csv`} small />
          </Card>
          <Explainer title="A lumped model of the board" refs={[{ label: 'IPC/JEDEC J-STD-020 (classification reflow profile)' }, { label: 'Incropera & DeWitt, Fundamentals of Heat and Mass Transfer — lumped capacitance' }, { label: 'Solder-paste datasheet profiles' }]}>
            Each part is a thermal mass heated by hot air. With a heat-transfer coefficient h over area A:
            <Formula>{'m·c_p · dT/dt = h·A · (T_air − T)\n⇒ T(t + Δt) = T_air + (T − T_air) · e^(−Δt/τ),   τ = m·c_p / (h·A)\nx(t) = v_conveyor · t   → T_air = zone setpoint at x'}</Formula>
            A tiny 0402 resistor (short τ) tracks the air; the big QFN of the Iris system-on-chip (long τ) lags. The profile must satisfy both: the small part must not overheat or ramp too fast, the large part must still spend enough time above liquidus. Air setpoints sit well above the board temperatures you want because the board never catches up in the time it spends in a zone. Measure the real profile with thermocouples on the heaviest and lightest parts before production.
          </Explainer>
        </div>
      )}
    </div>
  );
}
