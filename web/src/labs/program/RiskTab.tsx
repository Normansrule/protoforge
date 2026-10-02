// Schedule risk: three-point estimates sampled by a seeded Monte Carlo run
// of the whole network — animated histogram filling up, S-curve with P50 and
// P80, a criticality strip of the first sampled iterations, criticality
// index and tornado, milestone confidence dates and classic PERT alongside.
import { useMemo, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useElementWidth } from '../../components/useElementWidth';
import type { RiskResult } from './types';
import { addWeeks, CRIT, downloadName, num, PlayBar, useClock, useProgram } from './util';

export function RiskTab() {
  const { project, cfg, save } = useProgram();
  const [iters, setIters] = useState(2000);
  const [seed, setSeed] = useState<number | null>(null);
  const dist = cfg?.config.dist ?? 'pert';
  const r = useCompute<RiskResult>('program.risk', { project, iterations: iters, seed: seed ?? cfg?.config.seed ?? 7, dist }, !!cfg);
  const d = r.data;
  const clock = useClock(6, { loop: false, resetKey: d ? `${d.iterations}-${d.seed}-${d.dist}-${d.p80}` : 0 });
  const shown = d ? Math.max(1, Math.round((clock.t / clock.duration) * d.iterations)) : 0;
  const start = cfg?.config.start_date ?? '2026-10-05';

  return (
    <div className="stack">
      <ErrorBox error={r.error} />
      <div className="grid grid-4">
        <Stat label="Plan (most likely)" value={d ? num(d.deterministic, 1) : '…'} unit="wk" note={d ? `only ${num(d.prob_deterministic * 100, 0)} % of runs finish by then` : ''} tone="fail" />
        <Stat label="P50" value={d ? num(d.p50, 1) : '…'} unit="wk" note={d ? addWeeks(start, d.p50) : ''} />
        <Stat label="P80 — commit to this" value={d ? num(d.p80, 1) : '…'} unit="wk" note={d ? `${addWeeks(start, d.p80)} · program.p80_weeks` : ''} tone="warn" />
        <Stat label="Classic PERT P80" value={d ? num(d.pert_p80, 1) : '…'} unit="wk" note={d ? `mean ${num(d.pert_mean, 1)} ± ${num(d.pert_sd, 1)} wk on one path — misses merge bias` : ''} />
      </div>

      <div className="grid grid-sidebar">
        <Card title="Simulation">
          <Select label="Distribution" value={dist as 'pert' | 'triangular'} options={[{ value: 'pert', label: 'Beta-PERT (default)' }, { value: 'triangular', label: 'Triangular (heavier tails)' }]} onChange={(v) => save({ dist: v })} />
          <Slider label="Iterations" value={iters} min={200} max={10000} step={100} onChange={(v) => setIters(Math.round(v))} format={(v) => v.toFixed(0)} />
          <div className="row">
            <button className="btn small" onClick={() => setSeed(Math.floor(Math.random() * 1e6))}>New seed</button>
            <button className="btn small" disabled={seed === null} onClick={() => setSeed(null)}>Project seed</button>
            <span className="small muted mono">seed {d?.seed}</span>
          </div>
          <p className="small muted">Seeded, so the same inputs give the same answer — the requirement metric uses the project seed and {cfg?.config.iterations ?? 2000} iterations.</p>
          {d && <DownloadButton filename={downloadName(project, 'schedule-risk', 'csv')} content={() => 'iteration,finish_weeks\n' + d.finishes.map((f, i) => `${i + 1},${f.toFixed(3)}`).join('\n') + '\n'} label="Samples CSV" small />}
          <div className="pg-notes">{d?.notes.map((n, i) => <p key={i} className="small">{n}</p>)}</div>
        </Card>
        <Card title="Completion histogram" subtitle={d ? `${shown.toLocaleString()} of ${d.iterations.toLocaleString()} sampled projects` : ''}>
          <PlayBar clock={clock} label={() => `${shown}`} speeds={[0.5, 1, 2]} />
          {d && <Histogram d={d} shown={shown} />}
        </Card>
      </div>

      <div className="grid grid-2">
        <Card title="S-curve" subtitle="Probability of finishing PVT by a given week.">
          {d && (
            <Plot
              series={[{ name: 'Monte Carlo', x: d.s_curve.map((p) => p[0]), y: d.s_curve.map((p) => p[1] * 100), area: true }]}
              height={250}
              xLabel="weeks from start"
              yLabel="probability %"
              hLines={[{ value: 50, label: 'P50' }, { value: 80, label: 'P80', color: 'var(--warn)' }]}
              vLines={[{ value: d.deterministic, label: 'plan', color: 'var(--bad)' }, { value: d.p80, label: 'P80', color: 'var(--warn)' }]}
              formatY={(v) => v.toFixed(0)}
            />
          )}
        </Card>
        <Card title="Milestone confidence" subtitle="Plan vs P50 / P80 dates for every milestone and review.">
          <div className="table-wrap">
            <table className="table">
              <thead><tr><th>Milestone</th><th>Plan</th><th>P50</th><th>P80</th><th>P80 date</th></tr></thead>
              <tbody>
                {d?.milestones.map((m) => (
                  <tr key={m.id}>
                    <td><b className="mono small">{m.id}</b> <span className="small">{m.name}</span></td>
                    <td className="mono tnum">{num(m.deterministic, 1)}</td>
                    <td className="mono tnum">{num(m.p50, 1)}</td>
                    <td className="mono tnum"><b>{num(m.p80, 1)}</b></td>
                    <td className="mono small">{addWeeks(start, m.p80)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      </div>

      <Card title="Which tasks drive the finish?" subtitle="Criticality strip: one column per sampled project (first 60) — a filled cell means that task was on the critical path in that run. The critical path moves.">
        {d && <CritStrip d={d} upto={Math.min(60, shown)} />}
      </Card>

      <div className="grid grid-2">
        <Card title="Criticality index and sensitivity" subtitle="Share of runs in which the task is critical, and the Spearman rank correlation of its duration with the finish.">
          {d && <CritBars d={d} />}
        </Card>
        <Card title="Tornado" subtitle="Finish when one task is at its optimistic vs pessimistic duration, the rest at most likely.">
          {d && <TornadoChart d={d} />}
        </Card>
      </div>

      <Explainer title="Three-point estimates, PERT and Monte Carlo" refs={[{ label: 'Malcolm, Roseboom, Clark & Fazar 1959 (PERT)' }, { label: 'Hulett, Practical Schedule Risk Analysis (2009)' }, { label: 'PMBOK Guide, 7th ed.' }]}>
        <p>Each task has optimistic a, most likely m and pessimistic b. PERT approximates it with a beta distribution:</p>
        <Formula>tₑ = (a + 4m + b) / 6 · σ = (b − a) / 6 · α = 1 + 4(m − a)/(b − a), β = 1 + 4(b − m)/(b − a)</Formula>
        <p>Classic PERT adds tₑ and σ² along one critical path and reads a normal curve (P80 = μ + 0.84 σ). That misses <b>merge bias</b>: where parallel paths join, the join waits for the latest one, so the real finish is later than any single path suggests. The Monte Carlo run samples every task, repeats the CPM passes {d?.iterations.toLocaleString()} times and reads percentiles straight from the results. The <b>criticality index</b> is the share of runs in which a task is critical; <b>cruciality</b> (criticality × correlation) ranks where to spend management attention.</p>
        <p>Commit externally to P80, manage internally to the plan. The gap (≈ {d ? num(d.p80 - d.deterministic, 1) : '…'} weeks for Iris) is your schedule contingency.</p>
      </Explainer>
      {d && d.pert_path.length > 0 && <p className="small muted"><Chip tone="info">i</Chip> Classic PERT path: {d.pert_path.join(' → ')}</p>}
    </div>
  );
}

function Histogram({ d, shown }: { d: RiskResult; shown: number }) {
  const { counts, bins, max } = useMemo(() => {
    const n = d.histogram.counts.length;
    const c = new Array(n).fill(0);
    for (let i = 0; i < shown && i < d.finishes.length; i++) {
      const b = Math.min(n - 1, Math.max(0, Math.floor((d.finishes[i] - d.histogram.lo) / d.histogram.width)));
      c[b]++;
    }
    return { counts: c, bins: n, max: Math.max(...d.histogram.counts, 1) };
  }, [d, shown]);
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const W = Math.max(300, Math.min(760, width || 640)), H = W < 480 ? 210 : 240, ml = 36, mb = 30, mt = 10;
  const iw = W - ml - 10, ih = H - mb - mt;
  const lo = d.histogram.lo, hi = lo + bins * d.histogram.width;
  const x = (v: number) => ml + ((v - lo) / (hi - lo || 1)) * iw;
  const bw = iw / bins;
  const last = shown > 0 ? d.finishes[Math.min(shown, d.finishes.length) - 1] : null;
  const ticks = [];
  for (let t = Math.ceil(lo); t <= hi; t += Math.max(1, Math.round((hi - lo) / 7))) ticks.push(t);
  return (
    <div ref={ref}>
    <svg viewBox={`0 0 ${W} ${H}`} className="pg-svg" role="img" aria-label="Histogram of simulated completion weeks">
      {counts.map((c, i) => {
        const h = (c / max) * ih;
        const mid = lo + (i + 0.5) * d.histogram.width;
        const fill = mid <= d.p50 ? 'var(--series-3)' : mid <= d.p80 ? 'var(--series-4)' : 'var(--series-2)';
        return <rect key={i} x={ml + i * bw + 0.5} y={mt + ih - h} width={Math.max(0, bw - 1)} height={h} fill={fill} opacity={0.85} />;
      })}
      <line x1={ml} x2={ml + iw} y1={mt + ih} y2={mt + ih} stroke="var(--axis)" />
      {ticks.map((t) => <text key={t} x={x(t)} y={H - 12} fontSize={11} textAnchor="middle" fill="var(--muted)" className="mono">{t}</text>)}
      <text x={ml} y={H - 1} fontSize={11} fill="var(--text-2)">weeks to PVT exit</text>
      {[
        { v: d.deterministic, label: 'plan', c: 'var(--bad)' },
        { v: d.p50, label: 'P50', c: 'var(--text-2)' },
        { v: d.p80, label: 'P80', c: 'var(--warn)' },
      ].map((l, i) => (
        <g key={l.label}>
          <line x1={x(l.v)} x2={x(l.v)} y1={mt} y2={mt + ih} stroke={l.c} strokeDasharray="5 4" strokeWidth={1.4} />
          <text x={x(l.v) + 3} y={mt + 12 + i * 13} fontSize={11} fill={l.c} fontWeight={700} stroke="var(--surface-1)" strokeWidth={3} paintOrder="stroke">{l.label} {l.v.toFixed(1)}</text>
        </g>
      ))}
      {last !== null && shown < d.iterations && <circle cx={x(last)} cy={mt + 6} r={4} fill="var(--accent)" />}
    </svg>
    <div className="row small muted" style={{ marginTop: 4 }}>
      <span><span className="pg-swatch" style={{ background: 'var(--series-3)' }} />≤ P50</span>
      <span><span className="pg-swatch" style={{ background: 'var(--series-4)' }} />P50–P80</span>
      <span><span className="pg-swatch" style={{ background: 'var(--series-2)' }} />beyond P80</span>
    </div>
    </div>
  );
}

function CritStrip({ d, upto }: { d: RiskResult; upto: number }) {
  const ever = useMemo(() => {
    const set = new Set<number>();
    d.preview.forEach((p) => p.critical.forEach((k) => set.add(k)));
    return [...set].filter((k) => !d.ids[k].startsWith('M-') && !['TRR', 'PRR'].includes(d.ids[k])).sort((a, b) => a - b);
  }, [d]);
  const cols = d.preview.length;
  const cw = 9, rh = 13, L = 64;
  const W = L + cols * cw + 4, H = ever.length * rh + 18;
  const crit = new Map(d.tasks.map((t) => [t.id, t.criticality]));
  return (
    <div className="pg-scroll-x">
      <svg width={W + 60} height={H} role="img" aria-label="Which tasks were critical in each sampled run">
        {ever.map((k, row) => (
          <g key={k}>
            <text x={2} y={row * rh + 10} fontSize={10} className="mono" fill="var(--text-2)">{d.ids[k]}</text>
            {d.preview.slice(0, upto).map((p, c) => (
              <rect key={c} x={L + c * cw} y={row * rh + 1} width={cw - 1.5} height={rh - 2} rx={1.5}
                fill={p.critical.includes(k) ? CRIT : 'var(--surface-3)'} opacity={p.critical.includes(k) ? 0.9 : 0.6} />
            ))}
            <text x={L + cols * cw + 6} y={row * rh + 10} fontSize={10} className="mono" fill="var(--muted)">{((crit.get(d.ids[k]) ?? 0) * 100).toFixed(0)} %</text>
          </g>
        ))}
        <text x={L} y={H - 3} fontSize={10} fill="var(--muted)">sampled run → (right column: criticality index over all runs)</text>
      </svg>
    </div>
  );
}

function CritBars({ d }: { d: RiskResult }) {
  const rows = [...d.tasks].filter((t) => t.criticality > 0.01).sort((a, b) => b.cruciality - a.cruciality).slice(0, 14);
  return (
    <div className="pg-bars">
      {rows.map((t) => (
        <div key={t.id} className="pg-bar-row" title={t.name}>
          <span className="mono small">{t.id}</span>
          <span className="small pg-ellipsis">{t.name}</span>
          <div className="pg-bar"><div style={{ width: `${t.criticality * 100}%`, background: t.criticality > 0.8 ? CRIT : 'var(--series-1)' }} /></div>
          <span className="mono small tnum">{(t.criticality * 100).toFixed(0)} % · ρ {t.correlation.toFixed(2)}</span>
        </div>
      ))}
    </div>
  );
}

function TornadoChart({ d }: { d: RiskResult }) {
  const rows = d.tornado;
  const lo = Math.min(...rows.map((r) => r.low), d.deterministic) - 0.5;
  const hi = Math.max(...rows.map((r) => r.high), d.deterministic) + 0.5;
  const W = 560, L = 150, rh = 20, H = rows.length * rh + 26;
  const x = (v: number) => L + ((v - lo) / (hi - lo)) * (W - L - 40);
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="pg-svg" role="img" aria-label="Tornado chart of schedule sensitivity">
      <line x1={x(d.deterministic)} x2={x(d.deterministic)} y1={0} y2={H - 18} stroke="var(--text-2)" strokeDasharray="4 3" />
      {rows.map((r, i) => (
        <g key={r.id}>
          <title>{`${r.id} ${r.name}: ${r.low.toFixed(1)} → ${r.high.toFixed(1)} wk`}</title>
          <text x={L - 6} y={i * rh + 14} fontSize={10.5} textAnchor="end" fill="var(--text-2)">{`${r.id} ${r.name.length > 16 ? r.name.slice(0, 15) + '…' : r.name}`}</text>
          <rect x={x(r.low)} y={i * rh + 4} width={Math.max(1, x(d.deterministic) - x(r.low))} height={rh - 7} fill="var(--series-3)" opacity={0.85} />
          <rect x={x(d.deterministic)} y={i * rh + 4} width={Math.max(1, x(r.high) - x(d.deterministic))} height={rh - 7} fill="var(--series-2)" opacity={0.85} />
          <text x={x(r.high) + 4} y={i * rh + 14} fontSize={10} className="mono" fill="var(--muted)">+{(r.high - d.deterministic).toFixed(1)}</text>
        </g>
      ))}
      <text x={x(d.deterministic)} y={H - 4} fontSize={10.5} textAnchor="middle" fill="var(--muted)">{`plan ${d.deterministic.toFixed(1)} wk · green optimistic, orange pessimistic`}</text>
    </svg>
  );
}
