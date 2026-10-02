// Earned value: planned value from the baseline, a scenario that slows the
// work and raises costs from an onset week, the status date sweeping
// forward (animated), CPI / SPI and earned schedule, and the estimate at
// completion (EAC) formulas compared with where the scenario really ends.
import { useMemo, useState } from 'react';
import { Card, ErrorBox, Explainer, Formula, Slider, Stat, DownloadButton } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import type { EvmResult, Scenario } from './types';
import { downloadName, num, PlayBar, useClock, useProgram, usd } from './util';

const PRESETS: { label: string; s: Omit<Scenario, 'status_week'> }[] = [
  { label: 'On plan', s: { onset_week: 0, progress_rate: 1, cost_factor: 1 } },
  { label: 'Slow EVT bring-up', s: { onset_week: 14, progress_rate: 0.85, cost_factor: 1.12 } },
  { label: 'Supplier slip, cheap fix', s: { onset_week: 20, progress_rate: 0.7, cost_factor: 1.0 } },
  { label: 'Overtime to hold the date', s: { onset_week: 10, progress_rate: 0.98, cost_factor: 1.35 } },
];

export function EvmTab() {
  const { project, cfg } = useProgram();
  const [sc, setSc] = useState<Scenario>({ status_week: 24, onset_week: 14, progress_rate: 0.85, cost_factor: 1.12 });
  const [animate, setAnimate] = useState(false);
  const full = useCompute<EvmResult>('program.evm', { project, scenario: { ...sc, status_week: 1e6 } }, !!cfg);
  const end = full.data?.true_finish ?? 50;
  const clock = useClock(10, { loop: false, autoplay: false, resetKey: `${sc.onset_week}-${sc.progress_rate}-${sc.cost_factor}` });
  const status = animate ? (clock.t / clock.duration) * end : sc.status_week;
  const r = useCompute<EvmResult>('program.evm', { project, scenario: { ...sc, status_week: status } }, !!cfg);
  const d = r.data;

  const series = useMemo(() => {
    if (!d) return [];
    return [
      { name: 'Planned value (PV)', x: d.weeks, y: d.pv, color: 'var(--series-1)' },
      { name: 'Earned value (EV)', x: d.weeks.slice(0, d.ev.length), y: d.ev, color: 'var(--series-3)', width: 2.6 },
      { name: 'Actual cost (AC)', x: d.weeks.slice(0, d.ac.length), y: d.ac, color: 'var(--series-2)', width: 2.6 },
      { name: 'EV forecast', x: d.ev_forecast.map((p) => p[0]), y: d.ev_forecast.map((p) => p[1]), color: 'var(--series-3)', dash: '5 4' },
      { name: 'AC forecast', x: d.ac_forecast.map((p) => p[0]), y: d.ac_forecast.map((p) => p[1]), color: 'var(--series-2)', dash: '5 4' },
    ];
  }, [d]);
  const idxSeries = useMemo(() => {
    if (!d) return [];
    const cut = <T extends [number, number]>(v: T[]) => v.filter((p) => p[0] <= d.at + 1e-9);
    return [
      { name: 'CPI = EV / AC', x: cut(d.cpi_t).map((p) => p[0]), y: cut(d.cpi_t).map((p) => p[1]), color: 'var(--series-2)' },
      { name: 'SPI = EV / PV', x: cut(d.spi_t).map((p) => p[0]), y: cut(d.spi_t).map((p) => p[1]), color: 'var(--series-1)' },
      { name: 'SPI(t) = ES / AT', x: cut(d.spi_es_t).map((p) => p[0]), y: cut(d.spi_es_t).map((p) => p[1]), color: 'var(--series-7)', dash: '6 3' },
    ];
  }, [d]);
  const tone = (v: number) => (v >= 0.995 ? 'pass' : v >= 0.9 ? 'warn' : 'fail');

  return (
    <div className="stack">
      <ErrorBox error={r.error} />
      <div className="grid grid-sidebar">
        <Card title="Scenario" subtitle="How the project really goes (only you know — EVM reads the symptoms).">
          <div className="row" style={{ marginBottom: 10 }}>
            {PRESETS.map((p) => <button key={p.label} className="btn small" onClick={() => setSc((s) => ({ ...s, ...p.s }))}>{p.label}</button>)}
          </div>
          <Slider label="Trouble starts at" unit="wk" value={sc.onset_week} min={0} max={Math.max(10, Math.round(full.data?.pd ?? 46))} step={0.5} onChange={(v) => setSc({ ...sc, onset_week: v })} />
          <Slider label="Progress rate after that" value={sc.progress_rate} min={0.5} max={1.2} step={0.01} onChange={(v) => setSc({ ...sc, progress_rate: v })} format={(v) => `${(v * 100).toFixed(0)} %`} />
          <Slider label="Cost per earned dollar" value={sc.cost_factor} min={0.8} max={1.6} step={0.01} onChange={(v) => setSc({ ...sc, cost_factor: v })} format={(v) => `$${v.toFixed(2)}`} />
          <Slider label="Status date" unit="wk" value={animate ? status : sc.status_week} min={0} max={Math.ceil(end)} step={0.5} onChange={(v) => { setAnimate(false); clock.setPlaying(false); setSc({ ...sc, status_week: v }); }} />
          <div className="row">
            <button className="btn small primary" onClick={() => { setAnimate(true); clock.restart(); }}>▶ Sweep the status date</button>
            {animate && <button className="btn small" onClick={() => { setAnimate(false); clock.setPlaying(false); setSc({ ...sc, status_week: status }); }}>Stop here</button>}
          </div>
          {animate && <div style={{ marginTop: 8 }}><PlayBar clock={clock} label={() => `wk ${num(status, 1)}`} speeds={[0.5, 1, 2]} /></div>}
          <p className="small muted" style={{ marginTop: 10 }}>Budgets come from the schedule: each task's labour (FTE × weeks × rate) and expenses spread over its duration.</p>
        </Card>
        <Card title="Performance measurement baseline" subtitle={d ? `BAC ${usd(d.bac)} over ${num(d.pd, 1)} weeks · status week ${num(d.at, 1)}` : ''}>
          {d && (
            <Plot series={series} height={320} xLabel="week" yLabel="USD (cumulative)" formatY={(v) => usd(v)}
              vLines={[{ value: d.at, label: 'status', color: 'var(--accent)' }, { value: d.pd, label: 'PD', color: 'var(--muted)' }]}
              hLines={[{ value: d.bac, label: 'BAC' }, { value: d.idx.eac_cpi, label: '', color: 'var(--series-2)' }]} />
          )}
        </Card>
      </div>

      <div className="grid grid-4">
        <Stat label="Cost performance index (CPI)" value={d ? num(d.idx.cpi, 3) : '…'} tone={d ? tone(d.idx.cpi) : undefined} note={d ? `CV = EV − AC = ${usd(d.idx.cv)}` : ''} />
        <Stat label="Schedule performance index (SPI)" value={d ? num(d.idx.spi, 3) : '…'} tone={d ? tone(d.idx.spi) : undefined} note={d ? `SV = EV − PV = ${usd(d.idx.sv)}` : ''} />
        <Stat label="SPI(t), earned schedule" value={d ? num(d.spi_time, 3) : '…'} tone={d ? tone(d.spi_time) : undefined} note={d ? `ES ${num(d.es, 1)} wk at AT ${num(d.at, 1)} → finish ≈ ${num(d.ieac_weeks, 1)} wk` : ''} />
        <Stat label="To-complete index (TCPI)" value={d ? num(d.idx.tcpi, 3) : '…'} tone={d ? (d.idx.tcpi <= 1.05 ? 'pass' : d.idx.tcpi <= 1.15 ? 'warn' : 'fail') : undefined} note="efficiency needed on the remaining work to hit BAC" />
      </div>

      <div className="grid grid-2">
        <Card title="Estimate at completion" subtitle={d ? `The scenario actually ends at ${usd(d.true_cost)} in week ${num(d.true_finish, 1)} — which formula saw it coming?` : ''}>
          <div className="table-wrap">
            <table className="table">
              <thead><tr><th>Formula</th><th>EAC</th><th>vs truth</th></tr></thead>
              <tbody>
                {d?.eacs.map((e) => (
                  <tr key={e.name}>
                    <td><b className="mono small">{e.name}</b><div className="small muted">{e.meaning}</div><div className="small mono muted">{e.formula}</div></td>
                    <td className="mono tnum" style={{ whiteSpace: 'nowrap' }}>{usd(e.value)}</td>
                    <td className="mono tnum">{d.true_cost > 0 ? `${((e.value / d.true_cost - 1) * 100).toFixed(1)} %` : '—'}</td>
                  </tr>
                ))}
                {d && (
                  <tr>
                    <td><b className="small">Variance at completion</b><div className="small muted">VAC = BAC − EAC(BAC/CPI)</div></td>
                    <td className="mono tnum" style={{ whiteSpace: 'nowrap' }}>{usd(d.idx.vac)}</td>
                    <td />
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </Card>
        <Card title="Indices over time" subtitle="Cost-based SPI drifts back to 1 at the end (all value is eventually earned); the time-based SPI(t) does not.">
          {d && <Plot series={idxSeries} height={250} xLabel="week" yLabel="index" hLines={[{ value: 1, label: 'on plan' }]} yDomain={[0.5, 1.3]} formatY={(v) => v.toFixed(2)} />}
        </Card>
      </div>

      <Card title="Work packages at the status date" subtitle="Planned vs earned percent complete." actions={d && <DownloadButton small filename={downloadName(project, 'earned-value', 'csv')} content={() => 'week,pv,ev,ac\n' + d.weeks.map((w, i) => `${w},${d.pv[i].toFixed(0)},${d.ev[i]?.toFixed(0) ?? ''},${d.ac[i]?.toFixed(0) ?? ''}`).join('\n') + '\n'} label="EVM CSV" />}>
        <div className="pg-bars">
          {d?.tasks.filter((t) => t.planned_pct > 0 || t.earned_pct > 0).filter((t) => t.planned_pct < 100 || t.earned_pct < 100).slice(0, 14).map((t) => (
            <div key={t.id} className="pg-bar-row" title={t.name}>
              <span className="mono small">{t.id}</span>
              <span className="small pg-ellipsis">{t.name}</span>
              <div className="pg-bar pg-bar-2">
                <div style={{ width: `${t.planned_pct}%`, background: 'var(--series-1)', opacity: 0.35 }} />
                <div style={{ width: `${t.earned_pct}%`, background: 'var(--series-3)' }} />
              </div>
              <span className="mono small tnum">{t.earned_pct.toFixed(0)} / {t.planned_pct.toFixed(0)} %</span>
            </div>
          ))}
          {d && d.tasks.filter((t) => (t.planned_pct > 0 && t.planned_pct < 100) || (t.earned_pct > 0 && t.earned_pct < 100)).length === 0 && <p className="small muted">No work package in progress at this status date.</p>}
        </div>
        <p className="small muted">Pale blue = planned, green = earned. Completed and not-yet-started packages are hidden.</p>
      </Card>

      <Explainer title="Earned value management (EVM)" refs={[{ label: 'Fleming & Koppelman, Earned Value Project Management, 4th ed.' }, { label: 'PMBOK Guide, 7th ed.' }, { label: 'Lipke 2003, “Schedule is different”' }]}>
        <p>Three curves tell the whole story: the <b>planned value</b> (PV, budgeted cost of work scheduled), the <b>earned value</b> (EV, budgeted cost of work performed) and the <b>actual cost</b> (AC).</p>
        <Formula>CPI = EV / AC · SPI = EV / PV · CV = EV − AC · SV = EV − PV</Formula>
        <Formula>EAC = BAC / CPI · EAC = AC + (BAC − EV) · EAC = AC + (BAC − EV) / (CPI · SPI) · TCPI = (BAC − EV) / (BAC − AC)</Formula>
        <p>Earned schedule (ES) asks “when should we have earned what we have earned?”: PV(ES) = EV, SPI(t) = ES / AT and the time estimate IEAC(t) = planned duration / SPI(t). Studies of defence contracts found the cumulative CPI rarely improves much after about 20 % complete (Christensen & Heise 1993), which is why BAC / CPI is the usual first forecast.</p>
      </Explainer>
    </div>
  );
}
