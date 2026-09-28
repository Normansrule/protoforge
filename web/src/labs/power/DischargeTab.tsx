// Discharge: an animated cell draining through a simulated day, with the
// open-circuit-voltage curve, internal-resistance sag and temperature derating.
import { useMemo, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat } from '../../components/ui';
import { Plot, type Series } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { lerpAt, PlayBar, toCSV, useClock } from '../control/kit';
import type { DischargeCurve, DischargeResult } from './types';
import { clockTime, fmtHours, Kv, reqTarget } from './util';
import { REFS } from './Explainers';

function socColor(s: number) {
  return s > 0.5 ? 'var(--good)' : s > 0.2 ? 'var(--warn)' : 'var(--bad)';
}

function BatteryIcon({ soc, v, playing, t }: { soc: number; v: number; playing: boolean; t: number }) {
  const W = 120, H = 210, x0 = 20, y0 = 26, bw = 80, bh = 170;
  const inner = { x: x0 + 7, y: y0 + 7, w: bw - 14, h: bh - 14 };
  const level = inner.y + inner.h * (1 - Math.max(0, Math.min(1, soc)));
  // gentle surface wave while playing
  const amp = playing && soc > 0.02 ? 2.2 : 0;
  const pts: string[] = [];
  for (let i = 0; i <= 20; i++) {
    const x = inner.x + (inner.w * i) / 20;
    const y = level + amp * Math.sin(i / 3 + t * 5);
    pts.push(`${x.toFixed(1)},${y.toFixed(1)}`);
  }
  const fillPath = `M${inner.x},${inner.y + inner.h} L${pts.join(' L')} L${inner.x + inner.w},${inner.y + inner.h} Z`;
  const col = socColor(soc);
  const word = soc > 0.5 ? 'good' : soc > 0.2 ? 'low' : 'critical';
  return (
    <svg className="pw-battery" viewBox={`0 0 ${W} ${H}`} width={150} role="img" aria-label={`battery ${Math.round(soc * 100)} percent, ${word}`}>
      <defs>
        <clipPath id="pw-bat-clip"><rect x={inner.x} y={inner.y} width={inner.w} height={inner.h} rx={7} /></clipPath>
        <linearGradient id="pw-bat-sheen" x1="0" x2="1">
          <stop offset="0" stopColor="#fff" stopOpacity={0.0} />
          <stop offset="0.35" stopColor="#fff" stopOpacity={0.22} />
          <stop offset="0.6" stopColor="#fff" stopOpacity={0.0} />
        </linearGradient>
      </defs>
      <rect x={x0 + bw / 2 - 16} y={y0 - 12} width={32} height={14} rx={4} fill="var(--border-strong)" />
      <rect x={x0} y={y0} width={bw} height={bh} rx={14} fill="var(--surface-1)" stroke="var(--border-strong)" strokeWidth={3} />
      <g clipPath="url(#pw-bat-clip)">
        <path d={fillPath} style={{ fill: col, transition: 'fill 0.4s' }} opacity={0.9} />
        <rect x={inner.x} y={inner.y} width={inner.w} height={inner.h} fill="url(#pw-bat-sheen)" />
        {[0.25, 0.5, 0.75].map((f) => <line key={f} x1={inner.x} x2={inner.x + 10} y1={inner.y + inner.h * f} y2={inner.y + inner.h * f} stroke="var(--text-2)" strokeWidth={1} opacity={0.5} />)}
      </g>
      <text x={x0 + bw / 2} y={y0 + bh / 2 - 4} textAnchor="middle" fontSize={24} fontWeight={800} fill="var(--text)" style={{ paintOrder: 'stroke', stroke: 'var(--surface-1)', strokeWidth: 4 }}>{Math.round(soc * 100)}%</text>
      <text x={x0 + bw / 2} y={y0 + bh / 2 + 16} textAnchor="middle" fontSize={12} fill="var(--text)" style={{ paintOrder: 'stroke', stroke: 'var(--surface-1)', strokeWidth: 3 }}>{v.toFixed(3)} V</text>
      <text x={x0 + bw / 2} y={H - 2} textAnchor="middle" fontSize={11} fill="var(--muted)">{word}</text>
    </svg>
  );
}

function RuntimeBars({ curves, budget, req, now }: { curves: DischargeCurve[]; budget: number; req: number | null; now: number | null }) {
  const max = Math.max(16, ...curves.map((c) => c.cutoff_h), budget) * 1.05;
  const pct = (h: number) => `${(h / max) * 100}%`;
  const ticks = Array.from({ length: Math.floor(max / 2) + 1 }, (_, i) => i * 2);
  return (
    <div className="pw-wf" style={{ gridTemplateColumns: 'minmax(120px, 190px) minmax(0, 1fr) 66px' }}>
      {curves.map((c, i) => (
        <div key={c.label} style={{ display: 'contents' }}>
          <div className={`pw-wf-label ${i === 0 ? 'total' : ''}`}>{c.label}</div>
          <div className="pw-wf-track">
            <div className="pw-wf-seg" style={{ left: 0, width: pct(c.cutoff_h), background: i === 0 ? 'var(--accent)' : `var(--series-${(i % 7) + 2})`, opacity: i === 0 ? 1 : 0.75 }} />
            {req !== null && <div className="pw-wf-req" style={{ left: pct(req) }} title={`requirement ${req} h`} />}
            <div className="pw-wf-req" style={{ left: pct(budget), borderLeftColor: 'var(--text-2)', borderLeftStyle: 'dotted' }} title="budget estimate" />
            {i === 0 && now !== null && <div style={{ position: 'absolute', left: pct(now), top: 0, bottom: 0, width: 2, background: 'var(--text)' }} />}
          </div>
          <div className="pw-wf-val tnum">{c.cutoff_h.toFixed(2)} h</div>
        </div>
      ))}
      <div />
      <div className="pw-wf-axis">{ticks.map((t) => <span key={t} style={{ left: pct(t) }}>{t}</span>)}</div>
      <div className="small muted" style={{ textAlign: 'right' }}>hours</div>
    </div>
  );
}

export function DischargeTab({ scenario, scenarioName }: { scenario: string; scenarioName: string }) {
  const project = useProject((s) => s.project);
  const patch = useProject((s) => s.patchDesign);
  const [temp, setTemp] = useState(25);
  const [soh, setSoh] = useState(1);
  const [peukert, setPeukert] = useState(1);
  const bat = project.design.battery;
  const res = useCompute<DischargeResult>('power.discharge', { project, scenario, config: { temp_c: temp, state_of_health: soh, peukert_k: peukert } });
  const d = res.data;
  const lifeReq = reqTarget(project, 'power.battery_life_h');
  const clock = useClock(14, { loop: true, resetKey: `${scenario}` });
  const m = d?.main;
  const tH = m ? (clock.t / clock.duration) * m.cutoff_h : 0;
  const at = (ys: number[]) => (m ? lerpAt(m.t_h, ys, tH) : 0);
  const soc = m ? at(m.soc) : 1;
  const vt = m ? at(m.v_term) : 0;
  const ima = m ? at(m.current_ma) : 0;
  const ocvNow = m ? at(m.ocv_v) : 0;

  const series = useMemo(() => {
    if (!d) return null;
    const mm = d.main;
    const volt: Series[] = [
      { name: 'open-circuit voltage', x: mm.t_h, y: mm.ocv_v, color: 'var(--series-3)', dash: '6 4' },
      { name: 'terminal voltage', x: mm.t_h, y: mm.v_term, color: 'var(--series-1)', width: 2.4 },
      { name: 'terminal at peak load', x: mm.t_h, y: mm.v_peak, color: 'var(--series-2)', dash: '2 3' },
    ];
    const socS: Series[] = [
      { name: 'this cell', x: mm.t_h, y: mm.soc.map((s) => s * 100), color: 'var(--accent)', width: 2.6 },
      ...d.comparisons.slice(1).map((c, i) => ({ name: c.label, x: c.t_h, y: c.soc.map((s) => s * 100), color: `var(--series-${i + 3})`, dash: '5 4', width: 1.5 })),
    ];
    const cur: Series[] = [{ name: 'current', x: mm.t_h, y: mm.current_ma, color: 'var(--series-5)', area: true }];
    const ocv: Series[] = [{ name: 'OCV(state of charge)', x: d.ocv_curve.map((p) => p[0] * 100), y: d.ocv_curve.map((p) => p[1]), color: 'var(--series-3)' }];
    return { volt, socS, cur, ocv };
  }, [d]);

  const csv = () => {
    if (!d) return '';
    const mm = d.main;
    return toCSV(['t_h', 'soc', 'ocv_v', 'v_term', 'current_ma', 'v_peak'], [mm.t_h, mm.soc, mm.ocv_v, mm.v_term, mm.current_ma, mm.v_peak]) +
      `# load_mw,${d.load_mw}\n# peak_mw,${d.peak_mw}\n# temp_c,${temp}\n# capacity_factor,${d.capacity_factor}\n# r_factor,${d.r_factor}\n# cutoff_h,${mm.cutoff_h}\n`;
  };

  return (
    <div className="stack">
      <ErrorBox error={res.error} />
      <div className="pw-main">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card
            title="A day on one charge"
            subtitle={d ? `${scenarioName}: constant ${d.load_mw.toFixed(1)} mW from the cell, starting full at 07:00` : 'simulating…'}
            actions={<DownloadButton small filename={`iris-discharge-${scenario}.csv`} content={csv} label="discharge.csv" />}
          >
            {m && d && (
              <>
                <div className="pw-battery-wrap">
                  <BatteryIcon soc={soc} v={vt} playing={clock.playing} t={clock.t} />
                  <div className="stack" style={{ gap: 10, width: '100%' }}>
                    <div className="pw-stats" style={{ gridTemplateColumns: 'repeat(auto-fit, minmax(150px, 1fr))' }}>
                      <Stat label="Clock" value={clockTime(tH)} note={`${tH.toFixed(2)} h elapsed`} />
                      <Stat label="Terminal" value={vt.toFixed(3)} unit="V" note={`OCV ${ocvNow.toFixed(3)} V`} />
                      <Stat label="Current" value={ima.toFixed(2)} unit="mA" note={`I·R = ${(ima * m.r_ohm).toFixed(1)} mV`} />
                      <Stat label="Remaining" value={fmtHours(Math.max(0, m.cutoff_h - tH))} note={`to ${d.cutoff_v} V cutoff`} />
                    </div>
                    <PlayBar clock={clock} label={() => clockTime(tH)} />
                  </div>
                </div>
                <div style={{ marginTop: 14 }}>
                  <div className="pw-group-title">Runtime to cutoff</div>
                  <RuntimeBars curves={[m, ...d.comparisons]} budget={d.budget_life_h} req={lifeReq?.target ?? null} now={tH} />
                  <div className="small muted" style={{ marginTop: 6 }}>
                    <span style={{ display: 'inline-block', width: 14, borderTop: '2px dashed var(--bad)', verticalAlign: 'middle' }} /> requirement {lifeReq?.target ?? 8} h ·{' '}
                    <span style={{ display: 'inline-block', width: 14, borderTop: '2px dotted var(--text-2)', verticalAlign: 'middle' }} /> budget estimate {d.budget_life_h.toFixed(2)} h (usable fraction {(d.budget_usable_fraction * 100).toFixed(0)} %)
                  </div>
                </div>
              </>
            )}
          </Card>

          {series && m && d && (
            <Card title="Voltage, state of charge and current" subtitle="The cutoff is reached on the terminal voltage, not on charge: sag (I·R) and the knee of the curve end the run.">
              <Plot series={series.volt} height={240} xLabel="time (h)" yLabel="voltage (V)" hLines={[{ value: d.cutoff_v, label: `cutoff ${d.cutoff_v} V`, color: 'var(--bad)' }]} vLines={[{ value: tH, color: 'var(--accent)' }, ...(m.brownout_h !== null && m.brownout_h < m.cutoff_h - 0.05 ? [{ value: m.brownout_h, label: 'peak load browns out' }] : [])]} legend />
              <Plot series={series.socS} height={210} xLabel="time (h)" yLabel="state of charge (%)" vLines={[{ value: tH, color: 'var(--accent)' }]} yDomain={[0, 102]} legend />
              <Plot series={series.cur} height={150} xLabel="time (h)" yLabel="current (mA)" vLines={[{ value: tH, color: 'var(--accent)' }]} legend={false} />
            </Card>
          )}
        </div>

        <div className="stack">
          <Card title="Cell conditions" subtitle="Temperature and ageing are local what-ifs; resistance and cutoff edit design.battery">
            <Slider label="Cell temperature" value={temp} min={-20} max={60} step={1} unit="°C" onChange={setTemp} />
            <Slider label="State of health" value={soh} min={0.5} max={1} step={0.01} format={(v) => `${(v * 100).toFixed(0)} %`} onChange={setSoh} hint="Remaining capacity vs new (≈80 % is a common end-of-life definition)" />
            <Slider label="Internal resistance (25 °C)" value={bat.internal_resistance_ohm} min={0.05} max={3} log unit="Ω" onChange={(v) => patch('battery', { internal_resistance_ohm: v })} />
            <Slider label="Cutoff voltage" value={bat.cutoff_v} min={2.8} max={3.6} step={0.01} unit="V" onChange={(v) => patch('battery', { cutoff_v: v })} />
            <Slider label="Peukert exponent k" value={peukert} min={1} max={1.3} step={0.01} onChange={setPeukert} hint="1 = off. Lead-acid 1.1–1.3, lithium ≈ 1.0–1.05" />
            {d && (
              <Kv rows={[
                ['Capacity factor', `${(d.capacity_factor * 100).toFixed(0)} % → ${m!.capacity_mah.toFixed(0)} mAh`],
                ['Resistance', `${m!.r_ohm.toFixed(3)} Ω (×${d.r_factor.toFixed(2)})`],
                ['Delivered', `${m!.delivered_mah.toFixed(1)} mAh`],
                ['Delivered energy', `${m!.delivered_mwh.toFixed(0)} mWh`],
                ['Charge left at cutoff', `${(m!.soc_at_cutoff * 100).toFixed(1)} %`],
                ['Usable fraction (sim)', `${(m!.equivalent_usable_fraction * 100).toFixed(0)} %`],
                ['Usable fraction (budget)', `${(d.budget_usable_fraction * 100).toFixed(0)} %`],
                ['Max power OCV²/4R', `${(d.max_power_mw / 1000).toFixed(1)} W`],
                ['Stopped by', m!.end_reason],
              ]} />
            )}
          </Card>
          {series && d && m && (
            <Card title="Open-circuit voltage curve" subtitle="Typical 4.2 V lithium-polymer cell, low rate, 25 °C">
              <Plot
                series={[...series.ocv, { name: 'now (OCV)', x: [soc * 100], y: [ocvNow], scatter: true, color: 'var(--accent)' }, { name: 'now (terminal)', x: [soc * 100], y: [vt], scatter: true, color: 'var(--series-2)' }]}
                height={220}
                xLabel="state of charge (%)"
                yLabel="volts"
                hLines={[{ value: d.cutoff_v, label: 'cutoff', color: 'var(--bad)' }]}
                legend={false}
              />
              <div className="small muted">Flat plateau = voltage tells you little about charge between 30 and 60 %; fuel gauges count coulombs instead.</div>
            </Card>
          )}
        </div>
      </div>

      <div className="pw-explain">
        <Explainer title="The cell as a voltage source and a resistor" refs={[REFS.plett, REFS.linden]}>
          <p>The simplest useful battery model is an open-circuit voltage that depends on state of charge (SoC is avoided here — it also means system-on-chip) in series with an internal resistance:</p>
          <Formula>{'V_term = OCV(s) − I·R\nconstant power:  P = V_term·I  ⇒  R·I² − OCV·I + P = 0\nI = (OCV − √(OCV² − 4RP)) / 2R        ds/dt = −I / Q'}</Formula>
          <p>
            A regulator behind the cell draws constant <i>power</i>, so as the voltage falls the current rises — a small positive feedback that ends the run early
            when the cell is cold or worn. Above OCV²/4R the equation has no real root: the voltage collapses (maximum power transfer).
            Better models add RC pairs for diffusion and hysteresis (Plett, vol. 1).
          </p>
        </Explainer>
        <Explainer title="Open-circuit voltage, Peukert and the cold" refs={[REFS.linden, REFS.peukert]}>
          <p>
            The OCV table used here is a <b>typical</b> lithium-cobalt-oxide/graphite pouch-cell curve (steep top, long plateau, knee below ≈10 %), the shape shown
            in Linden & Reddy and in cell datasheets — not a specific cell. Measure yours with a slow C/20 discharge or pulse-and-rest steps before trusting runtime to minutes.
          </p>
          <p>
            Peukert's law <span className="mono">t = H·(C/(I·H))ᵏ</span> captures the lost capacity of lead-acid cells at high current; lithium cells have k ≈ 1.0–1.05
            and their rate effect is mostly the I·R term above. Cold is worse: capacity falls to roughly 80–85 % at 0 °C and resistance roughly doubles
            (illustrative trends; real cells vary).
          </p>
        </Explainer>
        <Explainer title="Why the simulation beats the budget" refs={[REFS.linden]}>
          <p>
            A fresh cell at 25 °C delivers ≈ {m ? (m.equivalent_usable_fraction * 100).toFixed(0) : '—'} % of <span className="mono">C·V_nom</span> (its average voltage ≈ 3.8 V exceeds the 3.7 V nominal),
            so it runs {m ? m.cutoff_h.toFixed(1) : '—'} h. The budget's usable fraction of {(bat.usable_fraction * 100).toFixed(0)} % deliberately designs for an aged (≈ 80 % health), cool cell with margin — the
            requirement must hold at end of life, not on day one. Try state of health 85 % and 10 °C.
          </p>
        </Explainer>
        <Explainer title="Iris tie-in">
          <p>
            R-002 (≥ 8 h) is verified with the budget model; this simulation shows the margin. The dotted orange trace is the terminal voltage if every load switched on at
            once: that peak ({d ? d.peak_mw.toFixed(0) : '—'} mW) must stay above cutoff or the system-on-chip browns out — the reason for R-012 (rail droop) in the Circuit Lab.
          </p>
        </Explainer>
      </div>
    </div>
  );
}
