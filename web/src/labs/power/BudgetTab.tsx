// Energy budget: scenarios, a waterfall from component power to battery
// power, duty-cycle sliders that edit the project, battery & rail settings.
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, Chip, DownloadButton, Explainer, Formula, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useElementWidth } from '../../components/useElementWidth';
import { IRIS, useProject } from '../../state/project';
import type { Project } from '../../engine/types';
import { toCSV } from '../control/kit';
import type { Budget, ScenarioResult } from './types';
import { fmtHours, Kv, passWord, reqTarget, Seg, subsystemColor } from './util';
import { REFS } from './Explainers';

export function ScenarioStrip({ scenarios, value, onChange, lifeReq }: { scenarios: ScenarioResult[]; value: string; onChange: (id: string) => void; lifeReq: number }) {
  const maxLife = Math.max(lifeReq * 2.5, 16);
  return (
    <div className="pw-scen" role="radiogroup" aria-label="Usage scenario">
      {scenarios.map((s) => {
        const ok = s.life_h >= lifeReq;
        return (
          <button key={s.id} role="radio" aria-checked={s.id === value} className={`pw-scen-card ${s.id === value ? 'active' : ''}`} onClick={() => onChange(s.id)} title={s.description}>
            <div className="pw-scen-name">{s.name}</div>
            <div className="pw-scen-life tnum">
              {s.life_h >= 100 ? s.life_h.toFixed(0) : s.life_h.toFixed(1)} h
              <small>{s.avg_mw.toFixed(1)} mW</small>
            </div>
            <div className="pw-meter" aria-hidden>
              <div className="fill" style={{ width: `${Math.min(100, (s.life_h / maxLife) * 100)}%`, background: ok ? 'var(--good)' : 'var(--series-2)' }} />
              <div className="mark" style={{ left: `${(lifeReq / maxLife) * 100}%` }} />
            </div>
            <div className="small muted" style={{ marginTop: 4 }}>{ok ? '✓' : '✕'} {lifeReq} h · display {(s.display_duty * 100).toFixed(0)} %{s.life_h > maxLife ? ' · off scale' : ''}</div>
          </button>
        );
      })}
    </div>
  );
}

type WfMode = 'subsystem' | 'component';

const fmtMw = (v: number) => (v >= 10 ? v.toFixed(1) : v >= 1 ? v.toFixed(2) : v >= 0.01 ? v.toFixed(3) : v.toPrecision(2));

interface WfRow { key: string; label: string; color: string; start: number; active: number; sleep: number; total?: boolean; note?: string }

function Waterfall({ rows, max, reqLine }: { rows: WfRow[]; max: number; reqLine?: { value: number; label: string } }) {
  const [ref, w] = useElementWidth<HTMLDivElement>();
  const pct = (v: number) => `${(v / max) * 100}%`;
  const step = max > 150 ? 50 : max > 60 ? 20 : max > 25 ? 10 : 5;
  const ticks: number[] = [];
  for (let v = 0; v <= max + 1e-9; v += step) ticks.push(v);
  return (
    <div className="pw-wf" ref={ref}>
      {rows.map((r) => (
        <div key={r.key} style={{ display: 'contents' }}>
          <div className={`pw-wf-label ${r.total ? 'total' : ''}`} title={r.note ?? r.label}>
            {!r.total && <span className="pw-dot" style={{ background: r.color }} />}
            {r.label}
          </div>
          <div className="pw-wf-track">
            <div className="pw-wf-seg" style={{ left: pct(r.start), width: pct(r.active), background: r.color }} />
            {r.sleep > 0 && <div className="pw-wf-seg sleep" style={{ left: pct(r.start + r.active), width: pct(r.sleep), background: r.color, opacity: 0.55 }} />}
            {reqLine && r.total && <div className="pw-wf-req" style={{ left: pct(reqLine.value) }} title={reqLine.label} />}
          </div>
          <div className="pw-wf-val tnum">{fmtMw(r.active + r.sleep)}</div>
        </div>
      ))}
      <div />
      <div className="pw-wf-axis" aria-hidden>
        {w > 0 && ticks.map((t) => <span key={t} style={{ left: pct(t) }}>{t}</span>)}
      </div>
      <div className="small muted" style={{ textAlign: 'right' }}>mW</div>
    </div>
  );
}

function batteryWaterfall(p: Project, b: Budget, mode: WfMode): WfRow[] {
  const rows: WfRow[] = [];
  let cum = 0;
  if (mode === 'subsystem') {
    for (const s of b.subsystems) {
      rows.push({ key: s.id, label: s.name, color: subsystemColor(p, s.id), start: cum, active: s.active_part_mw, sleep: s.sleep_part_mw, note: `${s.components.join(', ')} · ${(s.share * 100).toFixed(1)} % of the load` });
      cum += s.avg_mw;
    }
  } else {
    const comps = [...b.components].sort((a, c) => c.avg_mw - a.avg_mw);
    for (const c of comps) {
      rows.push({ key: c.refdes, label: `${c.refdes} ${c.name}`, color: subsystemColor(p, c.subsystem), start: cum, active: c.active_part_mw, sleep: c.sleep_part_mw });
      cum += c.avg_mw;
    }
  }
  rows.push({ key: 'load', label: 'Loads (after the regulator)', color: 'var(--text-2)', start: 0, active: b.load_mw, sleep: 0, total: true });
  rows.push({ key: 'loss', label: `Conversion loss (η ${(b.buck_efficiency * 100).toFixed(0)} %)`, color: 'var(--series-8)', start: b.load_mw, active: b.conversion_loss_mw, sleep: 0 });
  rows.push({ key: 'q', label: 'Quiescent current', color: 'var(--series-7)', start: b.load_mw + b.conversion_loss_mw, active: b.quiescent_mw, sleep: 0 });
  rows.push({ key: 'bat', label: 'From the battery', color: 'var(--accent)', start: 0, active: b.avg_mw, sleep: 0, total: true });
  return rows;
}

export function BudgetTab({ budget, scenarios, scenario, setScenario }: { budget: Budget | null; scenarios: ScenarioResult[]; scenario: string; setScenario: (s: string) => void }) {
  const project = useProject((s) => s.project);
  const update = useProject((s) => s.update);
  const patch = useProject((s) => s.patchDesign);
  const [mode, setMode] = useState<WfMode>('subsystem');
  const [hot, setHot] = useState<string | null>(null);
  const lifeReq = reqTarget(project, 'power.battery_life_h');
  const powReq = reqTarget(project, 'power.avg_power_mw');
  const b = budget;
  const bat = project.design.battery;
  const rails = project.design.power_rails;

  // battery life as a function of display duty (linear in duty)
  const lifeVsDuty = useMemo(() => {
    if (!b || !b.display_ref) return null;
    const c = b.components.find((x) => x.refdes === b.display_ref);
    if (!c) return null;
    const slope = (c.qty * (c.active_mw - c.sleep_mw)) / b.buck_efficiency;
    const xs = Array.from({ length: 101 }, (_, i) => i / 100);
    const avg = (d: number) => b.avg_mw + slope * (d - c.duty);
    const life = xs.map((d) => b.energy_mwh / avg(d));
    const target = lifeReq?.target ?? 8;
    const dMax = slope > 0 ? (b.energy_mwh / target - b.avg_mw) / slope + c.duty : 1;
    return { xs, life, now: c.duty, dMax: Math.max(0, Math.min(1, dMax)), name: c.name };
  }, [b, lifeReq?.target]);

  if (!b) return <Card title="Energy budget">Starting the engine…</Card>;
  const maxWf = Math.max(b.avg_mw, powReq?.target ?? 0) * 1.08;
  const lifeOk = lifeReq ? b.life_h >= lifeReq.target : true;
  const powOk = powReq ? b.avg_mw <= powReq.target : true;
  const isRef = scenario === 'reference';
  const setDuty = (refdes: string, v: number) => update((d) => {
    const c = d.components.find((x) => x.refdes === refdes);
    if (c) c.power.duty = v;
  });
  const resetDuties = () => update((d) => {
    for (const c of d.components) {
      const orig = IRIS.components.find((x) => x.refdes === c.refdes);
      if (orig?.power) c.power.duty = orig.power.duty;
    }
  });
  const budgetCsv = () =>
    toCSV(
      ['refdes', 'name', 'subsystem', 'qty', 'active_mw', 'sleep_mw', 'duty', 'active_part_mw', 'sleep_part_mw', 'avg_mw', 'share'],
      [b.components.map((c) => c.refdes), b.components.map((c) => c.name), b.components.map((c) => c.subsystem), b.components.map((c) => c.qty), b.components.map((c) => c.active_mw), b.components.map((c) => c.sleep_mw), b.components.map((c) => c.duty), b.components.map((c) => c.active_part_mw), b.components.map((c) => c.sleep_part_mw), b.components.map((c) => c.avg_mw), b.components.map((c) => c.share)],
    ) + `\n# loads_mw,${b.load_mw}\n# conversion_loss_mw,${b.conversion_loss_mw}\n# quiescent_mw,${b.quiescent_mw}\n# battery_avg_mw,${b.avg_mw}\n# usable_energy_mwh,${b.energy_mwh}\n# life_h,${b.life_h}\n# scenario,${scenario}\n`;
  const scenCsv = () => toCSV(['id', 'name', 'avg_mw', 'life_h', 'display_duty'], [scenarios.map((s) => s.id), scenarios.map((s) => s.name), scenarios.map((s) => s.avg_mw), scenarios.map((s) => s.life_h), scenarios.map((s) => s.display_duty)]);

  return (
    <div className="stack">
      <Card title="Usage scenario" subtitle="Duty cycles are the whole story: the same hardware lasts 4 h or 2 weeks depending on how it is used. Click to compare." actions={<DownloadButton small filename="iris-power-scenarios.csv" content={scenCsv} label="scenarios.csv" />}>
        <ScenarioStrip scenarios={scenarios} value={scenario} onChange={setScenario} lifeReq={lifeReq?.target ?? 8} />
        {!isRef && <div className="small muted" style={{ marginTop: 8 }}>{scenarios.find((s) => s.id === scenario)?.description} {lifeReq || powReq ? <>The requirements ({[lifeReq?.id, powReq?.id].filter(Boolean).join(', ')}) are</> : 'Requirements are'} always verified against the <b>reference</b> profile.</div>}
      </Card>

      <div className="pw-stats">
        <div className="pw-stat-big"><Stat label="Battery life" value={b.life_h.toFixed(2)} unit="h" tone={lifeOk ? 'pass' : 'fail'} note={`${fmtHours(b.life_h)} · ${lifeReq ? `${passWord(lifeOk)} ≥ ${lifeReq.target} h (${lifeReq.id})` : ''}`} /></div>
        <div className="pw-stat-big"><Stat label="Average power" value={b.avg_mw.toFixed(1)} unit="mW" tone={powOk ? 'pass' : 'fail'} note={powReq ? `${passWord(powOk)} ≤ ${powReq.target} mW (${powReq.id}) · ${b.avg_current_ma.toFixed(1)} mA` : ''} /></div>
        <div className="pw-stat-big"><Stat label="Usable energy" value={b.energy_mwh.toFixed(0)} unit="mWh" note={`${bat.capacity_mah} mAh × ${bat.nominal_v} V × ${(bat.usable_fraction * 100).toFixed(0)} %`} /></div>
        <div className="pw-stat-big"><Stat label="Peak power" value={b.peak_mw.toFixed(0)} unit="mW" note={`everything on · ${b.peak_current_ma.toFixed(0)} mA${bat.capacity_mah > 0 ? ` · ${(b.peak_current_ma / bat.capacity_mah).toFixed(2)} C` : ''}`} /></div>
      </div>

      <div className="pw-main">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card
            title="Where the power goes"
            subtitle="Each bar is qty × (P_active·duty + P_sleep·(1 − duty)); hatched = sleep power. The loads then pass through the regulator."
            actions={<Seg value={mode} onChange={setMode} label="Group by" options={[{ value: 'subsystem', label: 'Subsystems' }, { value: 'component', label: 'Components' }]} />}
          >
            <Waterfall rows={batteryWaterfall(project, b, mode)} max={maxWf} reqLine={powReq ? { value: powReq.target, label: `${powReq.id}: ≤ ${powReq.target} mW` } : undefined} />
            <div className="row small muted" style={{ marginTop: 8, gap: 14 }}>
              <span>Solid = active share · hatched = sleep share</span>
              {powReq && <span><span style={{ display: 'inline-block', width: 14, borderTop: '2px dashed var(--bad)', verticalAlign: 'middle' }} /> {powReq.id} limit {powReq.target} mW</span>}
              <span>Display alone: {((b.components.find((c) => c.refdes === b.display_ref)?.share ?? 0) * 100).toFixed(0)} % of the load</span>
            </div>
          </Card>

          <Card
            title="Duty cycles (reference profile)"
            subtitle="Drag a slider: it edits the component's duty in the project, so R-002/R-003, the RL Lab's fixed policy and the thermal case all update."
            actions={
              <div className="row" style={{ gap: 6 }}>
                <button className="btn small" onClick={resetDuties}>Reset to Iris</button>
                <DownloadButton small filename={`iris-power-budget-${scenario}.csv`} content={budgetCsv} label="budget.csv" />
              </div>
            }
          >
            <div className="table-wrap">
              <table className="pw-ctable">
                <thead>
                  <tr><th>Part</th><th className="pw-hide-md">Active</th><th className="pw-hide-md">Sleep</th><th>Duty</th><th>Average</th><th className="pw-hide-md">Share</th></tr>
                </thead>
                <tbody>
                  {[...b.components].sort((a, c) => c.avg_mw - a.avg_mw).map((c) => {
                    const pc = project.components.find((x) => x.refdes === c.refdes);
                    const bom = pc?.power.duty ?? c.bom_duty;
                    return (
                      <tr key={c.refdes} className={hot === c.refdes ? 'hot' : ''} onMouseEnter={() => setHot(c.refdes)} onMouseLeave={() => setHot(null)}>
                        <td>
                          <div className="row" style={{ gap: 6, flexWrap: 'nowrap' }}>
                            <span className="pw-dot" style={{ background: subsystemColor(project, c.subsystem) }} />
                            <span><b className="mono">{c.refdes}</b> <span className="small">{c.name}</span></span>
                          </div>
                        </td>
                        <td className="tnum pw-hide-md">{c.active_mw} mW</td>
                        <td className="tnum pw-hide-md">{c.sleep_mw} mW</td>
                        <td>
                          <div className="pw-duty">
                            <input type="range" min={0} max={1000} value={Math.round(bom * 1000)} aria-label={`${c.refdes} duty cycle`} onChange={(e) => setDuty(c.refdes, Number(e.target.value) / 1000)} />
                            <span className="mono small tnum">{(bom * 100).toFixed(bom < 0.1 ? 1 : 0)} %</span>
                          </div>
                          {!isRef && Math.abs(c.duty - bom) > 1e-9 && <div className="small muted">scenario: {(c.duty * 100).toFixed(0)} %</div>}
                        </td>
                        <td className="tnum mono">{c.avg_mw.toFixed(c.avg_mw < 1 ? 3 : 2)} mW</td>
                        <td className="pw-hide-md">
                          <div className="pw-share"><div style={{ width: `${c.share * 100}%`, background: subsystemColor(project, c.subsystem) }} /></div>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </Card>
        </div>

        <div className="stack">
          <Card title="Battery & power rails" subtitle="Written to design.battery and design.power_rails">
            <div className="pw-group">
              <div className="pw-group-title">Cell ({bat.chemistry})</div>
              <Slider label="Capacity" value={bat.capacity_mah} min={60} max={500} step={5} unit="mAh" onChange={(v) => patch('battery', { capacity_mah: v })} />
              <Slider label="Nominal voltage" value={bat.nominal_v} min={3.6} max={3.85} step={0.01} unit="V" onChange={(v) => patch('battery', { nominal_v: v })} />
              <Slider label="Usable fraction" value={bat.usable_fraction} min={0.5} max={1} step={0.01} format={(v) => `${(v * 100).toFixed(0)} %`} onChange={(v) => patch('battery', { usable_fraction: v })} hint="Ageing, temperature and cutoff margin" />
            </div>
            <div className="pw-group">
              <div className="pw-group-title">Regulator</div>
              <Slider label="Buck efficiency η" value={rails.buck_efficiency} min={0.6} max={0.98} step={0.01} format={(v) => `${(v * 100).toFixed(0)} %`} onChange={(v) => patch('power_rails', { buck_efficiency: v })} />
              <Slider label="Quiescent current" value={rails.quiescent_ua} min={0.1} max={200} log unit="µA" onChange={(v) => patch('power_rails', { quiescent_ua: v })} />
            </div>
            <Kv rows={[
              ['Rated energy', `${b.rated_energy_mwh.toFixed(0)} mWh`],
              ['Average current', `${b.avg_current_ma.toFixed(2)} mA (${b.c_rate.toFixed(3)} C)`],
              ['Conversion loss', `${b.conversion_loss_mw.toFixed(2)} mW`],
              ['Quiescent drain', `${(b.quiescent_mw * 1000).toFixed(1)} µW`],
            ]} />
          </Card>

          {lifeVsDuty && (
            <Card title="Life vs display duty" subtitle={`The ${lifeVsDuty.name.toLowerCase()} dominates: every other load held at its duty`}>
              <Plot
                series={[{ name: 'battery life', x: lifeVsDuty.xs.map((x) => x * 100), y: lifeVsDuty.life, color: 'var(--series-1)' }, { name: 'now', x: [lifeVsDuty.now * 100], y: [b.energy_mwh / (b.avg_mw)], scatter: true, color: 'var(--series-2)' }]}
                height={200}
                xLabel="display duty (%)"
                yLabel="life (h)"
                yDomain={[0, Math.max(24, (lifeReq?.target ?? 8) * 3)]}
                hLines={lifeReq ? [{ value: lifeReq.target, label: `${lifeReq.id} ${lifeReq.target} h`, color: 'var(--bad)' }] : []}
                vLines={[{ value: lifeVsDuty.dMax * 100, label: `max ${(lifeVsDuty.dMax * 100).toFixed(0)} %` }]}
                legend={false}
              />
              <div className="small muted">
                Keeping ≥ {lifeReq?.target ?? 8} h allows the display on up to <b style={{ color: 'var(--text)' }}>{(lifeVsDuty.dMax * 100).toFixed(0)} %</b> of the time.
                The <Link to="/rl">RL Lab</Link> learns when to dim or pause it.
              </div>
            </Card>
          )}
        </div>
      </div>

      <div className="pw-explain">
        <Explainer title="An energy budget in three lines" refs={[REFS.linden, REFS.benini]}>
          <p>Every part alternates between an active and a sleep power. Its time-average is the duty-cycle weighted mean:</p>
          <Formula>{'P̄ᵢ = qtyᵢ · (P_active,ᵢ · dᵢ + P_sleep,ᵢ · (1 − dᵢ))\nP_battery = Σ P̄ᵢ / η  +  I_q · V_nom\nlife = C · V_nom · f_usable / P_battery'}</Formula>
          <p>
            For Iris: {b.load_mw.toFixed(1)} mW of loads ÷ {(b.buck_efficiency * 100).toFixed(0)} % + {(b.quiescent_mw * 1000).toFixed(0)} µW = <b>{b.avg_mw.toFixed(1)} mW</b>;{' '}
            {bat.capacity_mah} mAh × {bat.nominal_v} V × {(bat.usable_fraction * 100).toFixed(0)} % = {b.energy_mwh.toFixed(0)} mWh → <b>{b.life_h.toFixed(2)} h</b>.
            mAh × V = mWh, so capacity alone is not energy: a 3.7 V cell stores 3.7 mWh per mAh.
          </p>
        </Explainer>
        <Explainer title="Why the display dominates — and what to do about it" refs={[REFS.benini]}>
          <p>
            A 140 mW micro-OLED on 30 % of the time is {((b.components.find((c) => c.refdes === b.display_ref)?.avg_mw ?? 0)).toFixed(0)} mW —
            about {((b.components.find((c) => c.refdes === b.display_ref)?.share ?? 0) * 100).toFixed(0)} % of everything. Halving its duty buys more life than removing every other chip.
            Dynamic power management (turn off what the wearer is not looking at, lower refresh when content is static) is the lever;
            the <Link to="/rl">RL Lab</Link> learns such a policy on this very budget.
          </p>
          <p>
            Sleep power matters for standby: {scenarios.find((s) => s.id === 'standby') ? <>with the display and microphones off Iris draws {scenarios.find((s) => s.id === 'standby')!.avg_mw.toFixed(1)} mW and lasts {fmtHours(scenarios.find((s) => s.id === 'standby')!.life_h)}.</> : null}
            {' '}Always-on leakage such as the battery-sense divider (R-014, ≤ 2 µA) and regulator quiescent current sets the shelf life.
          </p>
        </Explainer>
      </div>
      <div className="small muted">
        <Chip>Tip</Chip> Budget numbers are only as good as the datasheet powers in the bill of materials. Validate on hardware with a source-measure unit or a
        power profiler that integrates current over a scripted use session.
      </div>
    </div>
  );
}
