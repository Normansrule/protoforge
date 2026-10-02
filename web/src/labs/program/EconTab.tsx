// Unit economics: landed cost from the Manufacturing Hub model plus
// logistics, warranty and amortised certification / fixtures; the price
// waterfall to gross margin; break-even; a quarterly cash flow (J-curve)
// with NPV and IRR; and a sensitivity tornado. Every assumption is editable
// and labelled as one.
import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, NumberField, Select, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import type { Econ, EconResult } from './types';
import { useElementWidth } from '../../components/useElementWidth';
import { Assume, downloadName, int, num, useClock, useProgram, usd } from './util';

export function EconTab() {
  const { project, cfg, save } = useProgram();
  const r = useCompute<EconResult>('program.economics', { project }, !!cfg);
  const d = r.data;
  const e = cfg?.config.economics;
  const set = (patch: Partial<Econ>) => e && save({ economics: { ...e, ...patch } });
  const o = d?.outcome;
  const clock = useClock(4, { loop: false, resetKey: d ? Math.round(d.outcome.npv) : 0 });
  const grow = Math.min(1, clock.t / clock.duration);
  const vols = d?.econ.volumes ?? [];

  return (
    <div className="stack">
      <ErrorBox error={r.error} />
      <div className="grid grid-4">
        <Stat label="Gross margin" value={o ? num(o.gm_pct, 1) : '…'} unit="%" tone={o ? (o.gm_pct >= 40 ? 'pass' : o.gm_pct >= 25 ? 'warn' : 'fail') : undefined} note={o ? `${usd(o.net_price, 2)} net − ${usd(o.landed, 2)} landed` : ''} />
        <Stat label="Break-even volume" value={o ? (o.breakeven_units === null ? 'never' : int(o.breakeven_units)) : '…'} unit="units" tone={o ? (o.breakeven_units !== null && o.breakeven_units <= o.lifetime_units ? 'pass' : 'fail') : undefined} note={o ? `vs ${int(o.lifetime_units)} units planned over ${vols.length} years` : ''} />
        <Stat label="Net present value (NPV)" value={o ? usd(o.npv) : '…'} tone={o ? (o.npv >= 0 ? 'pass' : 'fail') : undefined} note={o ? `internal rate of return (IRR) ${o.irr_annual === null ? '—' : num(o.irr_annual * 100, 1) + ' %'} · ${e?.discount_rate_pct ?? 12} % discount` : ''} />
        <Stat label="Peak cash needed" value={o ? usd(o.peak_cash_need) : '…'} note={o ? `payback ${o.payback_weeks === null ? 'not within the horizon' : `week ${num(o.payback_weeks, 0)}`}` : ''} />
      </div>

      <div className="grid grid-sidebar">
        <Card title="Assumptions" subtitle="Every number here is a guess to replace with your own — none are quotes.">
          {e && (
            <>
              <Slider label="Retail price" unit="USD" value={e.retail_price_usd} min={49} max={999} step={1} onChange={(v) => set({ retail_price_usd: v })} hint="assumption — edit" />
              <Slider label="Direct-to-consumer share" unit="%" value={e.direct_share_pct} min={0} max={100} step={1} onChange={(v) => set({ direct_share_pct: v })} />
              <Slider label="Retailer margin" unit="%" value={e.retail_margin_pct} min={0} max={60} step={1} onChange={(v) => set({ retail_margin_pct: v })} />
              <Slider label="Payment and shop fees (direct)" unit="%" value={e.payment_fee_pct} min={0} max={10} step={0.1} onChange={(v) => set({ payment_fee_pct: v })} />
              <Slider label="Logistics per unit" unit="USD" value={e.logistics_usd} min={0} max={30} step={0.5} onChange={(v) => set({ logistics_usd: v })} />
              <Slider label="Warranty returns" unit="%" value={e.warranty_return_pct} min={0} max={20} step={0.5} onChange={(v) => set({ warranty_return_pct: v })} />
              <Slider label="Sales, marketing and support" unit="% of net" value={e.opex_pct} min={0} max={40} step={1} onChange={(v) => set({ opex_pct: v })} />
              <Slider label="Discount rate" unit="%/yr" value={e.discount_rate_pct} min={0} max={30} step={0.5} onChange={(v) => set({ discount_rate_pct: v })} />
              <div className="grid grid-3" style={{ gap: 8 }}>
                {[0, 1, 2].map((k) => (
                  <NumberField key={k} label={`Year ${k + 1} units`} value={vols[k] ?? 0} onChange={(v) => { const nv = [...vols]; nv[k] = Math.max(0, v); set({ volumes: nv }); }} />
                ))}
              </div>
              <NumberField label="Line fixtures and test jigs" unit="USD" value={e.fixtures_usd} onChange={(v) => set({ fixtures_usd: Math.max(0, v) })} />
              <Select label="Launch after" value={(e.launch || 'p80') as 'cpm' | 'p50' | 'p80'} options={[{ value: 'cpm', label: 'the plan (most likely)' }, { value: 'p50', label: 'P50 schedule' }, { value: 'p80', label: 'P80 schedule (prudent)' }]} onChange={(v) => set({ launch: v })} />
              <div className="row small muted"><Assume /> all of the above · <button className="btn small ghost" onClick={() => save({ economics: undefined })}>Reset</button></div>
            </>
          )}
        </Card>
        <div className="stack">
          <Card title="Landed unit cost" subtitle={d ? `${usd(o?.landed, 2)} per unit at ${int(d.target_qty)} units — frame by ${d.frame_process}` : ''}>
            {d && <LandedBar d={d} grow={grow} />}
            {d && (
              <table className="table" style={{ marginTop: 10 }}>
                <tbody>
                  {d.lines.map((l) => (
                    <tr key={l.name}>
                      <td><span className="pg-dot" style={{ background: lineColor(l.kind, l.name) }} />{l.name}<div className="small muted">{l.note}</div></td>
                      <td className="mono tnum" style={{ textAlign: 'right' }}>{usd(l.usd, 2)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
            <p className="small muted">Manufacturing lines come straight from the <Link to="/mfg">Manufacturing Hub</Link> (requirement R-022 uses the same number).</p>
          </Card>
        </div>
      </div>

      <div className="grid grid-2">
        <Card title="Price waterfall" subtitle="From the shelf price to what you keep per unit." actions={<button className="btn small" onClick={clock.restart} aria-label="Replay the animation">↺ Replay</button>}>
          {d && <Waterfall d={d} grow={grow} />}
        </Card>
        <Card title="Break-even" subtitle="Cumulative profit before tax vs units sold (constant contribution at the target-quantity cost).">
          {d && o && (
            <Plot
              series={[{ name: 'Cumulative profit', x: d.breakeven_curve.map((p) => p[0]), y: d.breakeven_curve.map((p) => p[1]), area: true }]}
              height={250} xLabel="units sold" yLabel="USD" formatY={(v) => usd(v)}
              hLines={[{ value: 0, label: 'break-even' }]}
              vLines={[...(o.breakeven_units !== null ? [{ value: o.breakeven_units, label: `${int(o.breakeven_units)} units`, color: 'var(--good)' }] : []), { value: o.lifetime_units, label: 'plan', color: 'var(--muted)' }]}
            />
          )}
        </Card>
      </div>

      <Card title="Cash flow: the J-curve" subtitle={d ? `Development spend until launch (week ${num(d.inputs.launch_week, 1)}), then ${vols.length} years of sales. Bars: quarterly net cash; line: cumulative.` : ''}
        actions={d && <DownloadButton small filename={downloadName(project, 'cash-flow', 'csv')} content={() => cashCsv(d)} label="Cash flow CSV" />}>
        {d && <CashChart d={d} grow={grow} />}
      </Card>

      <div className="grid grid-2">
        <Card title="What moves the business case" subtitle="NPV with one assumption at its low / high value, the rest at base.">
          {d && <NpvTornado d={d} />}
        </Card>
        <Card title="Notes" actions={d && <DownloadButton small filename={downloadName(project, 'business-case', 'md')} content={() => businessMd(project.meta.name, d)} label="Business case (Markdown)" />}>
          {d?.notes.map((n, i) => <p key={i} className="small">{n}</p>)}
          {d && <p className="small">Certification in the plan: {usd(d.plan_cert_usd)} (compliance-plan midpoints, indicative). Hard-tool cost from the Molding engine: {usd(d.tooling_usd)}; molding beats printing above {d.crossover_qty ? int(d.crossover_qty) : '—'} units.</p>}
        </Card>
      </div>

      <Explainer title="Unit economics and the business case" refs={[{ label: 'Ulrich & Eppinger, Product Design and Development (product development economics)' }, { label: 'NASA Cost Estimating Handbook v4.0 (2015)' }, { label: 'Brealey, Myers & Allen, Principles of Corporate Finance' }]}>
        <Formula>net = P · [s (1 − f) + (1 − s)(1 − m)] · GM = (net − landed) / net</Formula>
        <Formula>Q* = F / (net − variable − opex · net) · NPV = Σ CFₜ / (1 + r)ᵗ · IRR: NPV(r) = 0</Formula>
        <p>P is the retail price, s the direct-to-consumer share, f the payment fees, m the retailer margin; F is the development budget from the schedule (it already holds prototype builds, certification fees and fixtures, so they are not counted twice). Cash flows are quarterly; r is the quarterly equivalent of the annual discount rate. A common hardware-startup rule of thumb puts the retail price at roughly four to five times the bill of materials — compare Iris's price with its parts cost.</p>
      </Explainer>
    </div>
  );
}

function lineColor(kind: string, name: string) {
  if (kind === 'amortised') return 'var(--series-7)';
  if (kind === 'added') return 'var(--series-4)';
  if (name.startsWith('Frame')) return 'var(--series-3)';
  if (name.startsWith('Yield')) return 'var(--series-8)';
  if (name.startsWith('PCB')) return 'var(--copper)';
  return 'var(--series-1)';
}

function LandedBar({ d, grow }: { d: EconResult; grow: number }) {
  const total = d.lines.reduce((s, l) => s + Math.max(0, l.usd), 0) || 1;
  let acc = 0;
  return (
    <div className="pg-stack" role="img" aria-label="Landed unit cost by category">
      {d.lines.map((l) => {
        const w = (Math.max(0, l.usd) / total) * 100 * grow;
        const left = acc;
        acc += w;
        return <div key={l.name} title={`${l.name}: $${l.usd.toFixed(2)}`} style={{ left: `${left}%`, width: `${w}%`, background: lineColor(l.kind, l.name) }} />;
      })}
    </div>
  );
}

function Waterfall({ d, grow }: { d: EconResult; grow: number }) {
  const W = 560, H = 260, mb = 74, mt = 14, ml = 46;
  const max = d.waterfall[0][1] || 1;
  const n = d.waterfall.length;
  const bw = (W - ml - 10) / n;
  const y = (v: number) => mt + (H - mt - mb) * (1 - v / max);
  let run = 0;
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="pg-svg" role="img" aria-label="Price waterfall from retail price to gross profit">
      {[0, 0.25, 0.5, 0.75, 1].map((f) => (
        <g key={f}><line x1={ml} x2={W - 6} y1={y(max * f)} y2={y(max * f)} stroke="var(--grid)" /><text x={ml - 6} y={y(max * f) + 4} fontSize={10.5} textAnchor="end" fill="var(--muted)" className="mono">${(max * f).toFixed(0)}</text></g>
      ))}
      {d.waterfall.map(([name, v], i) => {
        const total = i === 0 || name.startsWith('Net') || name.startsWith('Gross');
        const k = Math.max(0, Math.min(1, grow * n - i));
        let top: number, bot: number;
        if (total) { top = v; bot = 0; run = v; } else { top = run; bot = run + v; run = bot; }
        const hi = Math.max(top, bot), lo = Math.min(top, bot);
        const y0 = y(hi), y1 = y(lo);
        const col = total ? (name.startsWith('Gross') ? 'var(--good)' : 'var(--series-1)') : 'var(--series-2)';
        return (
          <g key={name} opacity={k > 0 ? 1 : 0.15}>
            <rect x={ml + i * bw + 6} y={y1 - (y1 - y0) * k} width={bw - 12} height={(y1 - y0) * k} fill={col} rx={3} />
            <text x={ml + i * bw + bw / 2} y={y0 - 4} fontSize={11} textAnchor="middle" fill="var(--text)" className="mono">{v < 0 ? '−' : ''}${Math.abs(v).toFixed(0)}</text>
            <foreignObject x={ml + i * bw + 2} y={H - mb + 4} width={bw - 4} height={mb - 4}>
              <div className="pg-wf-label">{name}</div>
            </foreignObject>
          </g>
        );
      })}
    </svg>
  );
}

function CashChart({ d, grow }: { d: EconResult; grow: number }) {
  const q = d.outcome.quarters;
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const W = Math.max(520, width), H = 280, ml = 58, mb = 34, mt = 12;
  const vals = q.flatMap((x) => [x.net, x.cumulative]);
  const max = Math.max(...vals, 1), min = Math.min(...vals, -1);
  const y = (v: number) => mt + (H - mt - mb) * (1 - (v - min) / (max - min));
  const bw = (W - ml - 10) / q.length;
  const shown = Math.ceil(grow * q.length);
  const pts = q.slice(0, shown).map((x, i) => `${ml + i * bw + bw / 2},${y(x.cumulative)}`).join(' ');
  const launchQ = d.inputs.launch_week / 13;
  const ticks = useMemo(() => {
    const out: number[] = [];
    const step = (max - min) / 5;
    const mag = 10 ** Math.floor(Math.log10(step));
    const s = Math.ceil(step / mag) * mag;
    for (let v = Math.ceil(min / s) * s; v <= max; v += s) out.push(v);
    return out;
  }, [max, min]);
  return (
    <div className="pg-scroll-x" ref={ref}>
      <svg width={W} height={H} style={{ display: 'block' }} role="img" aria-label="Quarterly cash flow and cumulative cash">
        {ticks.map((t) => <g key={t}><line x1={ml} x2={W - 6} y1={y(t)} y2={y(t)} stroke={Math.abs(t) < 1e-9 ? 'var(--axis)' : 'var(--grid)'} /><text x={ml - 6} y={y(t) + 4} fontSize={10.5} textAnchor="end" fill="var(--muted)" className="mono">{usd(t)}</text></g>)}
        <rect x={ml} y={mt} width={Math.max(0, launchQ * bw)} height={H - mt - mb} fill="var(--accent-soft)" opacity={0.6} />
        <text x={ml + 4} y={mt + 12} fontSize={10.5} fill="var(--text-2)">development</text>
        {q.slice(0, shown).map((x, i) => (
          <g key={x.q}>
            <title>{`${x.label}: net ${usd(x.net)} (revenue ${usd(x.revenue)}, cost of goods ${usd(x.cogs)}, opex ${usd(x.opex)}, development ${usd(x.dev)}); cumulative ${usd(x.cumulative)}`}</title>
            <rect x={ml + i * bw + 3} y={Math.min(y(0), y(x.net))} width={bw - 6} height={Math.abs(y(x.net) - y(0))} fill={x.net >= 0 ? 'var(--series-3)' : 'var(--series-2)'} opacity={0.85} rx={2} />
          </g>
        ))}
        <polyline points={pts} fill="none" stroke="var(--series-1)" strokeWidth={2.4} />
        {q.slice(0, shown).map((x, i) => <circle key={x.q} cx={ml + i * bw + bw / 2} cy={y(x.cumulative)} r={2.6} fill="var(--series-1)" />)}
        {q.map((x, i) => (i % 2 === 0 ? <text key={x.q} x={ml + i * bw + bw / 2} y={H - 14} fontSize={10} textAnchor="middle" fill="var(--muted)">{x.label}</text> : null))}
        <text x={W - 8} y={H - 2} fontSize={10.5} textAnchor="end" fill="var(--muted)">green/orange bars: quarterly net cash · blue line: cumulative</text>
      </svg>
    </div>
  );
}

function NpvTornado({ d }: { d: EconResult }) {
  const base = d.outcome.npv;
  const rows = d.tornado;
  const lo = Math.min(base, ...rows.flatMap((r) => [r.npv_low, r.npv_high]));
  const hi = Math.max(base, ...rows.flatMap((r) => [r.npv_low, r.npv_high]));
  const W = 560, L = 170, rh = 22, H = rows.length * rh + 24;
  const x = (v: number) => L + ((v - lo) / (hi - lo || 1)) * (W - L - 16);
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="pg-svg" role="img" aria-label="Net present value sensitivity tornado">
      <line x1={x(0)} x2={x(0)} y1={0} y2={H - 18} stroke="var(--axis)" />
      <line x1={x(base)} x2={x(base)} y1={0} y2={H - 18} stroke="var(--text-2)" strokeDasharray="4 3" />
      {rows.map((r, i) => {
        const a = Math.min(r.npv_low, r.npv_high), b = Math.max(r.npv_low, r.npv_high);
        return (
          <g key={r.name}>
            <title>{`${r.name}: ${r.low_label} → ${usd(r.npv_low)}, ${r.high_label} → ${usd(r.npv_high)}`}</title>
            <text x={L - 6} y={i * rh + 15} fontSize={11} textAnchor="end" fill="var(--text-2)">{r.name}</text>
            <rect x={x(a)} y={i * rh + 5} width={Math.max(1, x(b) - x(a))} height={rh - 9} fill="var(--surface-3)" />
            <rect x={Math.min(x(r.npv_low), x(base))} y={i * rh + 5} width={Math.abs(x(r.npv_low) - x(base))} height={rh - 9} fill="var(--series-7)" opacity={0.85} />
            <rect x={Math.min(x(r.npv_high), x(base))} y={i * rh + 5} width={Math.abs(x(r.npv_high) - x(base))} height={rh - 9} fill="var(--series-4)" opacity={0.85} />
          </g>
        );
      })}
      <text x={x(base)} y={H - 4} fontSize={10.5} textAnchor="middle" fill="var(--muted)">{`base NPV ${usd(base)} · purple = low value, amber = high value`}</text>
    </svg>
  );
}

function cashCsv(d: EconResult) {
  return 'quarter,units,revenue_usd,cogs_usd,opex_usd,development_usd,net_usd,cumulative_usd\n' +
    d.outcome.quarters.map((q) => [q.label, q.units.toFixed(1), q.revenue.toFixed(0), q.cogs.toFixed(0), q.opex.toFixed(0), q.dev.toFixed(0), q.net.toFixed(0), q.cumulative.toFixed(0)].join(',')).join('\n') + '\n';
}

function businessMd(name: string, d: EconResult) {
  const o = d.outcome;
  let s = `# ${name} — first-pass business case\n\nGenerated by Protoforge (Program & Business Lab). **Every price, rate and volume is an assumption; costs are indicative estimates, not quotes.**\n\n`;
  s += `| Metric | Value |\n|---|---|\n| Retail price | $${d.econ.retail_price_usd.toFixed(0)} |\n| Net revenue per unit | $${o.net_price.toFixed(2)} |\n| Landed unit cost (${d.target_qty} units) | $${o.landed.toFixed(2)} |\n| Gross margin | ${o.gm_pct.toFixed(1)} % |\n| Break-even | ${o.breakeven_units === null ? 'never' : Math.round(o.breakeven_units)} units |\n| Lifetime volume | ${Math.round(o.lifetime_units)} units |\n| NPV @ ${d.econ.discount_rate_pct} % | $${o.npv.toFixed(0)} |\n| IRR | ${o.irr_annual === null ? '—' : (o.irr_annual * 100).toFixed(1) + ' %'} |\n| Peak cash need | $${o.peak_cash_need.toFixed(0)} |\n\n## Landed cost\n\n| Line | USD/unit | Basis |\n|---|---|---|\n`;
  for (const l of d.lines) s += `| ${l.name} | ${l.usd.toFixed(2)} | ${l.note} |\n`;
  s += `\n## Sensitivity (NPV)\n\n| Assumption | Low | NPV | High | NPV |\n|---|---|---|---|---|\n`;
  for (const t of d.tornado) s += `| ${t.name} | ${t.low_label} | $${t.npv_low.toFixed(0)} | ${t.high_label} | $${t.npv_high.toFixed(0)} |\n`;
  s += `\n## Notes\n\n${d.notes.map((n) => `- ${n}`).join('\n')}\n`;
  return s;
}
