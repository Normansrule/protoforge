// Budgets: mass, power / battery life and cost roll-ups per subsystem with
// margins against their requirements, plus a what-if parameter sweep.
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, Explainer, Formula, Stat } from '../../components/ui';
import { Plot, seriesColor } from '../../components/Plot';
import { Icon } from '../../components/Icon';
import { call } from '../../engine/engine';
import { useCompute } from '../../engine/hooks';
import { Badge, REFS, fmtNum, signedPct, useStudio } from './shared';
import type { Budget, Param, Sensitivity } from './types';

interface Seg { id: string; label: string; value: number; color: string }

/** Top 7 subsystems keep a colour (assigned in project order); the rest fold into "Other". */
function segmentsOf(b: Budget, order: string[]): Seg[] {
  const shares = [...b.by_subsystem].filter((s) => s.value > 0);
  const top = [...shares].sort((a, c) => c.value - a.value).slice(0, 7);
  const keep = new Set(top.map((s) => s.id));
  const ordered = shares.filter((s) => keep.has(s.id)).sort((a, c) => order.indexOf(a.id) - order.indexOf(c.id));
  const out: Seg[] = ordered.map((s, i) => ({ id: s.id, label: s.name, value: s.value, color: seriesColor(i) }));
  const other = shares.filter((s) => !keep.has(s.id)).reduce((a, s) => a + s.value, 0);
  if (other > 0) out.push({ id: 'other', label: 'Other', value: other, color: 'var(--muted)' });
  return out;
}

function StackBar({ b }: { b: Budget }) {
  const { project } = useStudio();
  const [hover, setHover] = useState<Seg | null>(null);
  const segs = segmentsOf(b, project.subsystems.map((s) => s.id));
  const ge = b.cmp === 'ge';
  const scale = Math.max(b.total, b.limit ?? 0, 1e-9) * 1.08;
  return (
    <div>
      <div className="ss-stack-wrap" style={{ marginTop: 22 }}>
        <div className="ss-stack" style={{ width: `${(b.total / scale) * 100}%`, minWidth: 4 }}>
          {segs.map((s) => (
            <span key={s.id} style={{ width: `${(s.value / Math.max(b.total, 1e-12)) * 100}%`, background: s.color, opacity: hover && hover.id !== s.id ? 0.45 : 1 }}
              onMouseEnter={() => setHover(s)} onMouseLeave={() => setHover(null)} aria-label={`${s.label} ${fmtNum(s.value)} ${b.unit}`} />
          ))}
        </div>
        {b.limit != null && !ge && (
          <div className="ss-stack-limit" style={{ left: `${(b.limit / scale) * 100}%` }}><span>limit {fmtNum(b.limit)} {b.unit}</span></div>
        )}
      </div>
      <div className="muted small" style={{ minHeight: 18, marginTop: 6 }}>
        {hover ? <><b style={{ color: 'var(--text)' }}>{hover.label}</b>: {fmtNum(hover.value)} {b.unit} ({Math.round((hover.value / Math.max(b.total, 1e-12)) * 100)} %)</> : 'Hover a segment for its share.'}
      </div>
      <div className="ss-legend" style={{ marginTop: 2 }}>
        {segs.map((s) => <span className="i" key={s.id}><span className="ss-sw" style={{ background: s.color }} />{s.label} <span className="muted tnum">{fmtNum(s.value)}</span></span>)}
      </div>
    </div>
  );
}

function ItemsTable({ b, top = 10 }: { b: Budget; top?: number }) {
  const { project } = useStudio();
  const items = [...b.items].sort((a, c) => c.value - a.value);
  const shown = items.slice(0, top);
  const rest = items.slice(top).reduce((a, x) => a + x.value, 0);
  const sub = (id: string) => project.subsystems.find((s) => s.id === id)?.name ?? id;
  return (
    <div className="table-wrap" style={{ marginTop: 10 }}>
      <table className="table">
        <thead><tr><th>Line</th><th>Subsystem</th><th style={{ textAlign: 'right' }}>{b.unit}</th><th style={{ textAlign: 'right' }}>Share</th></tr></thead>
        <tbody>
          {shown.map((x) => (
            <tr key={x.id + x.label}>
              <td className="small"><span className="ss-id">{x.kind === 'component' ? x.id : ''}</span> {x.label}</td>
              <td className="small muted">{sub(x.subsystem)}</td>
              <td className="mono small" style={{ textAlign: 'right' }}>{fmtNum(x.value)}</td>
              <td className="small" style={{ textAlign: 'right', width: 110 }}>
                <span style={{ display: 'inline-block', width: 50, height: 5, borderRadius: 3, background: 'var(--surface-3)', marginRight: 6, verticalAlign: 'middle' }}>
                  <span style={{ display: 'block', width: `${(x.value / Math.max(b.total, 1e-12)) * 100}%`, height: '100%', borderRadius: 3, background: 'var(--series-1)' }} />
                </span>
                {Math.round((x.value / Math.max(b.total, 1e-12)) * 100)} %
              </td>
            </tr>
          ))}
          {rest > 0 && <tr><td className="small muted" colSpan={2}>{items.length - top} more lines</td><td className="mono small" style={{ textAlign: 'right' }}>{fmtNum(rest)}</td><td /></tr>}
        </tbody>
      </table>
    </div>
  );
}

function marginNote(b: Budget) {
  if (b.limit == null) return 'no requirement bounds this budget';
  const abs = b.margin_abs ?? 0;
  return `${b.cmp === 'ge' ? '≥' : '≤'} ${fmtNum(b.limit)} ${b.unit} (${b.requirement}) · margin ${signedPct(b.margin)} = ${abs >= 0 ? '' : '−'}${fmtNum(Math.abs(abs))} ${b.unit}`;
}
const tone = (b: Budget) => (b.margin == null ? undefined : b.margin >= 0 ? 'pass' : 'fail') as 'pass' | 'fail' | undefined;

export function BudgetsTab() {
  const { studio, openReq } = useStudio();
  if (!studio) return <Card title="Budgets"><div className="muted small">computing…</div></Card>;
  const { mass, power, battery_life, cost } = studio.budgets;
  const all = [mass, battery_life, power, cost];
  return (
    <>
      <div className="grid grid-4">
        {all.map((b) => (
          <Stat key={b.key} label={b.label} value={fmtNum(b.total)} unit={b.unit} tone={tone(b)}
            note={<>{b.margin != null ? (b.margin >= 0 ? '✓ ' : '✕ ') : ''}{marginNote(b)}</>} />
        ))}
      </div>

      <div className="grid grid-2">
        <Card title="Mass budget" subtitle={mass.note} actions={<>{mass.requirement && <button className="btn small" onClick={() => openReq(mass.requirement!)}>{mass.requirement}</button>}<Link className="btn small" to="/cad"><Icon name="cad" size={14} /> CAD</Link></>}>
          <StackBar b={mass} />
          <ItemsTable b={mass} />
        </Card>
        <Card title="Power budget" subtitle={power.note} actions={<>{power.requirement && <button className="btn small" onClick={() => openReq(power.requirement!)}>{power.requirement}</button>}<Link className="btn small" to="/power"><Icon name="power" size={14} /> Power lab</Link></>}>
          <StackBar b={power} />
          <div className="row" style={{ marginTop: 10, gap: 8 }}>
            <Badge status={battery_life.margin == null ? 'unverified' : battery_life.margin >= 0 ? 'pass' : 'fail'} label={`battery life ${fmtNum(battery_life.total)} h`} />
            <span className="muted small">{battery_life.note}{battery_life.limit != null && ` · need ≥ ${fmtNum(battery_life.limit)} h (${battery_life.requirement})`}</span>
          </div>
          {power.verified_value != null && <div className="muted small" style={{ marginTop: 6 }}>Power lab verified value: <b className="mono">{fmtNum(power.verified_value)} mW</b> (the lab models each rail; this roll-up is the planning estimate).</div>}
          <ItemsTable b={power} top={8} />
        </Card>
      </div>

      <div className="grid grid-2">
        <Card title="Cost budget" subtitle={cost.note} actions={<>{cost.requirement && <button className="btn small" onClick={() => openReq(cost.requirement!)}>{cost.requirement}</button>}<Link className="btn small" to="/mfg"><Icon name="mfg" size={14} /> Manufacturing</Link></>}>
          <StackBar b={cost} />
          <ItemsTable b={cost} top={8} />
        </Card>
        <Card title="Per-subsystem roll-up" subtitle="Bill of materials (BOM) totals by subsystem">
          <div className="table-wrap">
            <table className="table">
              <thead><tr><th>Subsystem</th><th style={{ textAlign: 'right' }}>Parts</th><th style={{ textAlign: 'right' }}>Mass g</th><th style={{ textAlign: 'right' }}>Cost USD</th><th style={{ textAlign: 'right' }}>Avg mW</th></tr></thead>
              <tbody>
                {studio.budgets.subsystems.map((s) => (
                  <tr key={s.id}>
                    <td className="small">{s.name}</td>
                    <td className="mono small" style={{ textAlign: 'right' }}>{s.components}</td>
                    <td className="mono small" style={{ textAlign: 'right' }}>{fmtNum(s.mass_g)}</td>
                    <td className="mono small" style={{ textAlign: 'right' }}>{fmtNum(s.cost_usd)}</td>
                    <td className="mono small" style={{ textAlign: 'right' }}>{fmtNum(s.power_mw)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      </div>

      <SensitivitySweep />

      <div className="grid grid-2">
        <Explainer title="Technical budgets and margins" refs={[REFS.nasa, REFS.mass]}>
          A budget allocates a system resource — mass, power, cost — down to subsystems and tracks the <b>margin</b> to the limit a requirement sets.
          The NASA handbook tracks these as technical performance measures at every review. Early estimates are uncertain, so programmes hold larger reserves
          at concept and let them shrink as estimates become calculations and then measured parts (the mass-growth allowances of AIAA S-120A formalise this).
          <Formula>{'margin = (limit − estimate) / limit        (≤ limits)\nmargin = (estimate − limit) / limit        (≥ limits)'}</Formula>
        </Explainer>
        <Explainer title="How these roll-ups are computed" refs={[REFS.nasa]}>
          Mass = Σ qty × mass of every BOM line, plus the frame mass from the CAD Studio's mesh volume when the requirement bounds the total wearable mass. Average power =
          Σ (P<sub>active</sub>·duty + P<sub>sleep</sub>·(1 − duty)) ÷ regulator efficiency; battery life = capacity × nominal voltage × usable fraction ÷ average power.
          Cost is the Manufacturing Hub's unit cost at the target quantity when the requirement bounds it, split into volume-priced BOM and conversion (frame, assembly, test).
        </Explainer>
      </div>
    </>
  );
}

const SUGGESTED = ['battery.capacity_mah', 'frame.density_g_cm3', 'frame.wall_mm', 'power_rails.buck_efficiency', 'audio.mic_spacing_mm', 'imu.fusion_alpha', 'chip.clock_mhz', 'optics.lens_focal_mm', 'manufacturing.target_quantity'];

function SensitivitySweep() {
  const { project, openReq } = useStudio();
  const params = useCompute<Param[]>('systems.parameters', project);
  const [path, setPath] = useState('battery.capacity_mah');
  const [range, setRange] = useState<{ lo: string; hi: string }>({ lo: '', hi: '' });
  const [steps, setSteps] = useState(7);
  const [out, setOut] = useState<{ s: Sensitivity; ms: number } | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const p = params.data?.find((x) => x.path === path);
  const sorted = useMemo(() => {
    const ps = params.data ?? [];
    return [...ps.filter((x) => SUGGESTED.includes(x.path)), ...ps.filter((x) => !SUGGESTED.includes(x.path))];
  }, [params.data]);
  const run = () => {
    setBusy(true);
    setErr(null);
    // yield a frame so the button shows its busy state before the synchronous sweep
    window.setTimeout(() => {
      try {
        const t0 = performance.now();
        const lo = range.lo.trim() === '' ? undefined : Number(range.lo);
        const hi = range.hi.trim() === '' ? undefined : Number(range.hi);
        const s = call<Sensitivity>('systems.sensitivity', { project, param: path, lo, hi, steps });
        setOut({ s, ms: performance.now() - t0 });
      } catch (e) {
        setErr(e instanceof Error ? e.message : String(e));
      }
      setBusy(false);
    }, 30);
  };
  const s = out?.s;
  const moving = s ? s.rows.filter((r) => r.moves || r.flips) : [];
  const plotted = moving.filter((r) => r.margins.some((m) => m != null)).slice(0, 8);
  return (
    <Card
      title="What if? — design parameter sweep"
      subtitle="Vary one design parameter, re-verify every requirement at each value, and see which requirements move or flip. Each point is a full evaluation, so the sweep is capped at 15 values."
    >
      <div className="ss-toolbar">
        <select value={path} onChange={(e) => { setPath(e.target.value); setRange({ lo: '', hi: '' }); setOut(null); }} aria-label="Parameter" style={{ maxWidth: 320 }}>
          {sorted.map((x) => <option key={x.path} value={x.path}>{x.path} = {fmtNum(x.value)}</option>)}
        </select>
        <label className="small row" style={{ gap: 6 }}>from <input type="text" inputMode="decimal" style={{ width: 90 }} placeholder={p ? fmtNum(p.value * 0.5) : ''} value={range.lo} onChange={(e) => setRange((r) => ({ ...r, lo: e.target.value }))} /></label>
        <label className="small row" style={{ gap: 6 }}>to <input type="text" inputMode="decimal" style={{ width: 90 }} placeholder={p ? fmtNum(p.value * 1.5) : ''} value={range.hi} onChange={(e) => setRange((r) => ({ ...r, hi: e.target.value }))} /></label>
        <label className="small row" style={{ gap: 6 }}>steps
          <select value={steps} onChange={(e) => setSteps(Number(e.target.value))} style={{ width: 'auto' }}>{[3, 5, 7, 9, 11, 15].map((n) => <option key={n} value={n}>{n}</option>)}</select>
        </label>
        <button className="btn primary small" onClick={run} disabled={busy || !p}><Icon name="play" size={13} /> {busy ? 'Sweeping…' : 'Run sweep'}</button>
        {out && <span className="muted small">{s!.values.length} evaluations in {out.ms.toFixed(0)} ms</span>}
      </div>
      {err && <div className="callout" style={{ borderLeftColor: 'var(--bad)', color: 'var(--bad-text)' }}>{err}</div>}
      {!s && !err && <div className="ss-empty">Pick a parameter (default range ±50 % around the current value) and run the sweep. Try the battery capacity or the frame wall thickness.</div>}
      {s && (
        <div className="grid grid-2">
          <div>
            {plotted.length > 0 ? (
              <Plot
                height={260}
                xLabel={s.param}
                yLabel="margin (%)"
                series={plotted.map((r) => ({ name: r.id, x: s.values, y: r.margins.map((m) => (m == null ? NaN : m * 100)) }))}
                hLines={[{ value: 0, label: 'limit', color: 'var(--bad)' }]}
                vLines={[{ value: s.baseline, label: 'now' }]}
              />
            ) : <div className="ss-empty">No requirement's metric moves with {s.param} — its effect may live in a lab that is not bound to any requirement.</div>}
          </div>
          <div>
            <div className="small" style={{ marginBottom: 8 }}>
              {s.flipped.length > 0 ? <><Badge status="warn" label={`${s.flipped.length} flip`} /> {s.flipped.join(', ')} change status within this range.</> : <><Badge status="pass" label="no flips" /> Every requirement keeps its status across the range.</>}
            </div>
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr><th>Req.</th>{s.values.map((v, i) => <th key={i} className="mono" style={{ textAlign: 'center', fontSize: 10.5, color: i === s.baseline_index ? 'var(--text)' : undefined }}>{fmtNum(v)}</th>)}</tr>
                </thead>
                <tbody>
                  {moving.map((r) => (
                    <tr key={r.id}>
                      <td><button className="ss-link ss-id" onClick={() => openReq(r.id)}>{r.id}</button></td>
                      {r.statuses.map((st, i) => (
                        <td key={i} style={{ textAlign: 'center', fontWeight: 800, color: st === 'pass' ? 'var(--good-text)' : st === 'fail' ? 'var(--bad-text)' : 'var(--muted)', background: i === s.baseline_index ? 'var(--accent-soft)' : undefined }}
                          title={`${r.metric} = ${r.values[i] != null ? fmtNum(r.values[i]!) : '—'} → ${st}`}>
                          {st === 'pass' ? '✓' : st === 'fail' ? '✕' : '○'}
                        </td>
                      ))}
                    </tr>
                  ))}
                  <tr>
                    <td className="muted small">pass</td>
                    {s.pass.map((n, i) => <td key={i} className="mono small" style={{ textAlign: 'center' }}>{n}</td>)}
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}
    </Card>
  );
}
