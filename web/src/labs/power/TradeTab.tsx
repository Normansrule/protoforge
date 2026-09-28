// Battery trade: capacity sweep → life and wearable mass, feasible region,
// and the project's trade study T-3 scored with engineering numbers.
import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useMetricMap, useProject } from '../../state/project';
import { toCSV } from '../control/kit';
import type { TradeResult } from './types';
import { REFS } from './Explainers';

export function TradeTab({ scenario }: { scenario: string }) {
  const project = useProject((s) => s.project);
  const update = useProject((s) => s.update);
  const metrics = useMetricMap();
  const total = metrics.get('mass.total_g')?.value;
  const res = useCompute<TradeResult>('power.trade', { project, scenario, config: { total_mass_g: total ?? null } });
  const t = res.data;
  const [cap, setCap] = useState<number | null>(null);
  const bat = project.design.battery;
  const pick = cap ?? bat.capacity_mah;
  const tradeId = t?.trade_id ?? 'T-3';
  const study = project.trades.find((x) => x.id === tradeId);

  const apply = (c: number) => update((d) => {
    const gpm = d.design.battery.capacity_mah > 0 ? d.design.battery.mass_g / d.design.battery.capacity_mah : 0;
    d.design.battery.capacity_mah = c;
    d.design.battery.mass_g = +(gpm * c).toFixed(2);
    const cell = d.components.find((x) => x.refdes.startsWith('BT'));
    if (cell) {
      cell.mass_g = +(gpm * c).toFixed(2);
      cell.name = cell.name.replace(/\d+(\.\d+)?\s*mAh/, `${c} mAh`);
    }
  });

  if (!t) return <div className="stack"><ErrorBox error={res.error} /><Card title="Battery trade">Computing…</Card></div>;
  const life = (c: number) => (c * bat.nominal_v * bat.usable_fraction) / t.avg_mw;
  const mass = (c: number) => t.other_mass_g + c * t.g_per_mah;
  const lo = t.min_mah_for_life;
  const hi = t.max_mah_for_mass;
  const band = t.feasible ? [{ from: lo, to: hi, color: 'color-mix(in srgb, var(--good) 13%, transparent)', label: 'feasible' }] : [];
  const opts = t.options;
  const pickOk = life(pick) >= t.life_req_h && mass(pick) <= t.mass_req_g;
  const csv = () => toCSV(['capacity_mah', 'life_h', 'battery_mass_g', 'total_mass_g'], [t.capacity_mah, t.life_h, t.battery_mass_g, t.total_mass_g]);
  const weighted = study?.options.map((o) => ({ name: o.name, score: o.scores.reduce((s, v, i) => s + v * (study.criteria[i]?.weight ?? 0), 0) })) ?? [];
  const best = weighted.reduce((a, b) => (b.score > a.score ? b : a), weighted[0] ?? { name: '', score: 0 });

  return (
    <div className="stack">
      <div className="pw-stats">
        <Stat label="Feasible capacity" value={t.feasible ? `${lo.toFixed(0)}–${hi.toFixed(0)}` : 'none'} unit="mAh" tone={t.feasible ? 'pass' : 'fail'} note={`≥ ${t.life_req_h} h needs ≥ ${lo.toFixed(0)} mAh; ≤ ${t.mass_req_g} g allows ≤ ${hi.toFixed(0)} mAh`} />
        <Stat label="Current cell" value={bat.capacity_mah.toFixed(0)} unit="mAh" note={`${t.current_life_h.toFixed(1)} h · ${t.current_mass_g.toFixed(1)} g total`} tone={t.current_life_h >= t.life_req_h && t.current_mass_g <= t.mass_req_g ? 'pass' : 'fail'} />
        <Stat label="Mass without battery" value={t.other_mass_g.toFixed(1)} unit="g" note={t.mass_source.startsWith('mass.total_g') ? 'frame (CAD) + other components' : t.mass_source} />
        <Stat label="Cell mass per mAh" value={(t.g_per_mah * 1000).toFixed(1)} unit="mg/mAh" note={`≈ ${((bat.nominal_v / t.g_per_mah)).toFixed(0)} Wh/kg (${bat.mass_g} g for ${bat.capacity_mah} mAh)`} />
      </div>
      <div className="pw-main">
        <Card title="Life and mass vs battery capacity" subtitle={`Average power ${t.avg_mw.toFixed(1)} mW (${scenario === 'reference' ? 'reference profile' : 'selected scenario'}); cell mass scales with capacity at constant specific energy.`} actions={<DownloadButton small filename="iris-battery-trade.csv" content={csv} label="trade.csv" />}>
          <Plot
            series={[
              { name: 'battery life', x: t.capacity_mah, y: t.life_h, color: 'var(--series-1)', width: 2.4 },
              ...(opts.length ? [{ name: `${tradeId} options`, x: opts.map((o) => o.capacity_mah), y: opts.map((o) => o.life_h), scatter: true, color: 'var(--series-2)' }] : []),
              { name: 'selected', x: [pick], y: [life(pick)], scatter: true, color: 'var(--text)' },
            ]}
            height={230}
            xLabel="capacity (mAh)"
            yLabel="battery life (h)"
            hLines={[{ value: t.life_req_h, label: `R-002 ≥ ${t.life_req_h} h`, color: 'var(--bad)' }]}
            vLines={[{ value: bat.capacity_mah, label: 'current' }]}
            xBands={band}
            legend
          />
          <Plot
            series={[
              { name: 'total wearable mass', x: t.capacity_mah, y: t.total_mass_g, color: 'var(--series-3)', width: 2.4 },
              ...(opts.length ? [{ name: `${tradeId} options`, x: opts.map((o) => o.capacity_mah), y: opts.map((o) => o.total_mass_g), scatter: true, color: 'var(--series-2)' }] : []),
              { name: 'selected', x: [pick], y: [mass(pick)], scatter: true, color: 'var(--text)' },
            ]}
            height={200}
            xLabel="capacity (mAh)"
            yLabel="mass (g)"
            hLines={[{ value: t.mass_req_g, label: `R-001 ≤ ${t.mass_req_g} g`, color: 'var(--bad)' }]}
            vLines={[{ value: bat.capacity_mah, label: 'current' }]}
            xBands={band}
            legend={false}
          />
          <div className="small muted">Shaded: every capacity that meets both R-002 (life) and R-001 (mass). Orange points: the options in trade study {tradeId}.</div>
        </Card>
        <div className="stack">
          <Card title="Try a cell" subtitle="Preview, then apply: updates design.battery and the cell's mass in the bill of materials">
            <Slider label="Capacity" value={pick} min={40} max={500} step={5} unit="mAh" onChange={setCap} />
            <div className="row small" style={{ gap: 8 }}>
              <Chip tone={pickOk ? 'pass' : 'fail'}>{pickOk ? '✓ feasible' : '✕ infeasible'}</Chip>
              <span className="tnum">{life(pick).toFixed(1)} h · {mass(pick).toFixed(1)} g</span>
            </div>
            <div className="row" style={{ marginTop: 10 }}>
              <button className="btn primary small" disabled={pick === bat.capacity_mah} onClick={() => { apply(pick); setCap(null); }}>Apply {pick} mAh</button>
              {cap !== null && <button className="btn small" onClick={() => setCap(null)}>Cancel</button>}
            </div>
          </Card>
          {study && (
            <Card title={`Trade study ${study.id}`} subtitle={`${study.title} · decision: ${study.decision}`}>
              <div className="table-wrap">
                <table className="table" style={{ fontSize: 12.5, whiteSpace: 'nowrap' }}>
                  <thead><tr><th>Option</th><th>Life</th><th>Mass</th><th>Weighted score</th></tr></thead>
                  <tbody>
                    {study.options.map((o) => {
                      const pt = opts.find((x) => x.name === o.name);
                      const w = weighted.find((x) => x.name === o.name)?.score ?? 0;
                      return (
                        <tr key={o.name}>
                          <td><b>{o.name}</b>{o.name === study.decision && <div className="small muted">chosen</div>}</td>
                          <td className="tnum">{pt ? `${pt.life_h.toFixed(1)} h` : '—'} {pt && (pt.life_h >= t.life_req_h ? '✓' : '✕')}</td>
                          <td className="tnum">{pt ? `${pt.total_mass_g.toFixed(1)} g` : '—'} {pt && (pt.total_mass_g <= t.mass_req_g ? '✓' : '✕')}</td>
                          <td className="tnum">{w.toFixed(2)}{o.name === best.name ? ' ★' : ''}</td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
              <div className="small muted" style={{ marginTop: 6 }}>
                Weights: {study.criteria.map((c) => `${c.name} ${(c.weight * 100).toFixed(0)} %`).join(' · ')}. Scores are the team's 1–10 judgements; the life and mass columns are this lab's analysis, so the judgement can be checked.
              </div>
              <Link className="btn small" style={{ marginTop: 10 }} to={`/systems?trade=${study.id}`}>Open {study.id} in Systems Studio</Link>
            </Card>
          )}
        </div>
      </div>
      <div className="pw-explain">
        <Explainer title="A trade with two hard constraints" refs={[REFS.linden]}>
          <Formula>{`life(C) = C · V_nom · f_usable / P̄ ≥ ${t.life_req_h} h\n        ⇒ C ≥ ${lo.toFixed(0)} mAh\nmass(C) = m_rest + C · m_cell/C_cell ≤ ${t.mass_req_g} g\n        ⇒ C ≤ ${hi.toFixed(0)} mAh`}</Formula>
          <p>
            Both are straight lines in capacity, so the feasible set is an interval. A weighted-sum trade study (T-3) then chooses inside it: more capacity buys
            life margin, less buys comfort and a smaller temple pod. Real cells are not perfectly linear — very small pouch cells carry proportionally more packaging, so their specific energy is lower.
          </p>
        </Explainer>
        <Explainer title="Iris tie-in">
          <p>
            The mass line starts at {t.other_mass_g.toFixed(1)} g — the frame from the <Link to="/cad">CAD Studio</Link> plus every other component. Every gram saved elsewhere widens
            the battery window. Every milliwatt saved in the <Link to="/power">budget</Link> or by the <Link to="/rl">learned power policy</Link> moves the life bound left.
          </p>
        </Explainer>
      </div>
    </div>
  );
}
