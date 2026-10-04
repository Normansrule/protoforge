// Tolerance stack-ups: the Iris PCB-in-pod fit, the display focus chain and a
// custom chain — worst case, root-sum-square (RSS) and Monte Carlo, with a
// to-scale stack diagram, an animated Monte Carlo histogram and a Pareto of
// the contributors.
import { useEffect, useMemo, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat, Toggle } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { Dim, FocusStackOut, PodStackOut, StackOut } from './types';
import { fmt, prefersReducedMotion, toCsv, useRaf } from './util';

type StackId = 'pod' | 'focus' | 'custom';

const CUSTOM_DEFAULT: Dim[] = [
  { name: 'Knuckle slot width (front)', nominal: 3.2, plus: 0.1, minus: 0.1, dir: 1, dist: 'normal', sigmas: 3 },
  { name: 'Temple lug thickness', nominal: 3.0, plus: 0.1, minus: 0.1, dir: -1, dist: 'normal', sigmas: 3 },
  { name: 'Washer', nominal: 0.1, plus: 0.02, minus: 0.02, dir: -1, dist: 'uniform', sigmas: 3 },
];

export function TolerancesTab() {
  const project = useProject((s) => s.project);
  const [which, setWhich] = useState<StackId>('pod');
  // PCB in pod.
  const [process, setProcess] = useState<'fdm' | 'molded'>('fdm');
  const [podTol, setPodTol] = useState<number | null>(null);
  const [pcbTol, setPcbTol] = useState(0.15);
  const [thermal, setThermal] = useState(true);
  const [tMin, setTMin] = useState(-10);
  const pod = useCompute<PodStackOut>('structures.pod_stack', { project, process, pod_tol_mm: podTol ?? undefined, pcb_tol_mm: pcbTol, thermal, t_min_c: tMin, samples: 20000 }, which === 'pod');
  // Focus chain.
  const [fProcess, setFProcess] = useState<'fdm' | 'molded'>('fdm');
  const [allow, setAllow] = useState(0.25);
  const [housingTol, setHousingTol] = useState<number | null>(null);
  const focus = useCompute<FocusStackOut>('structures.focus_stack', { project, process: fProcess, housing_tol_mm: housingTol ?? undefined, allowable_d: allow, samples: 20000 }, which === 'focus');
  // Custom.
  const [dims, setDims] = useState<Dim[]>(CUSTOM_DEFAULT);
  const [lower, setLower] = useState(0);
  const [upper, setUpper] = useState<number | null>(0.3);
  const custom = useCompute<StackOut>('structures.stackup', { dims, lower, upper, samples: 20000, seed: 3 }, which === 'custom');

  const data: { dims: Dim[]; stack: StackOut } | null =
    which === 'pod' ? (pod.data ? { dims: pod.data.dims, stack: pod.data.stack } : null) : which === 'focus' ? (focus.data ? { dims: focus.data.dims, stack: focus.data.stack } : null) : custom.data ? { dims, stack: custom.data } : null;
  const error = which === 'pod' ? pod.error : which === 'focus' ? focus.error : custom.error;
  const title = which === 'pod' ? 'PCB in the right-temple pod (length)' : which === 'focus' ? 'Display focus chain (display-to-lens spacing)' : 'Custom stack';
  const unitNote = which === 'focus' ? 'gap = display-to-lens spacing' : 'gap = clearance';
  const csv = () => (data ? stackCsv(title, data.dims, data.stack) : '');
  return (
    <div className="stack">
      <div className="st-presets" role="tablist" aria-label="Stack">
        {(
          [
            ['pod', 'PCB-in-pod fit', 'Does the board always drop in? → structures.pod_fit_worst_mm'],
            ['focus', 'Optics alignment chain', 'Is the virtual image in focus on every unit?'],
            ['custom', 'Custom stack', 'Build your own chain (hinge knuckle play by default)'],
          ] as const
        ).map(([id, label, sub]) => (
          <button key={id} role="tab" aria-selected={which === id} className={`st-preset ${which === id ? 'active' : ''}`} onClick={() => setWhich(id)}>
            <b>{label}</b>
            <span className="small muted">{sub}</span>
          </button>
        ))}
      </div>
      <div className="st-fea">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card title={title} subtitle={`Each bar is one dimension, drawn to scale left (−) or right (+); ${unitNote}`} actions={data && <DownloadButton small filename={`stackup-${which}.csv`} label="stack-up CSV" content={csv} />}>
            <ErrorBox error={error} />
            {data ? <StackDiagram dims={data.dims} stack={data.stack} /> : <div className="muted small">Computing…</div>}
          </Card>
          {data && <Results stack={data.stack} focus={which === 'focus' ? focus.data : null} />}
          {data && <MonteCarlo stack={data.stack} key={which} />}
        </div>
        <div className="stack cp-sticky">
          {which === 'pod' && (
            <Card title="Pod stack inputs">
              <Select label="Pod process" value={process} options={[{ value: 'fdm', label: 'FDM print (±0.20 mm typical)' }, { value: 'molded', label: 'Injection molded (±0.10 mm typical)' }]} onChange={(v) => { setProcess(v); setPodTol(null); }} />
              <Slider label="Pod inner length tolerance ±" value={podTol ?? (process === 'molded' ? 0.1 : 0.2)} min={0.02} max={0.5} step={0.01} unit="mm" onChange={setPodTol} />
              <Slider label="PCB outline tolerance ±" value={pcbTol} min={0.05} max={0.3} step={0.01} unit="mm" onChange={setPcbTol} />
              <Toggle label="Include cold-temperature shrink of the pod" value={thermal} onChange={setThermal} />
              {thermal && <Slider label="Cold limit" value={tMin} min={-40} max={20} step={1} unit="°C" onChange={setTMin} />}
              {pod.data && (
                <ul className="small muted" style={{ margin: '6px 0 0', paddingLeft: 18 }}>
                  {pod.data.notes.map((n, i) => (
                    <li key={i}>{n}</li>
                  ))}
                </ul>
              )}
            </Card>
          )}
          {which === 'focus' && (
            <Card title="Focus chain inputs">
              <Select label="Optics housing process" value={fProcess} options={[{ value: 'fdm', label: 'Printed housing (±0.10 mm)' }, { value: 'molded', label: 'Molded housing (±0.03 mm)' }]} onChange={(v) => { setFProcess(v); setHousingTol(null); }} />
              <Slider label="Housing seat-to-seat tolerance ±" value={housingTol ?? (fProcess === 'molded' ? 0.03 : 0.1)} min={0.01} max={0.3} step={0.005} unit="mm" onChange={setHousingTol} />
              <Slider label="Allowed focus error ±" value={allow} min={0.05} max={1} step={0.05} unit="D" onChange={setAllow} />
              {focus.data && (
                <ul className="small muted" style={{ margin: '6px 0 0', paddingLeft: 18 }}>
                  {focus.data.notes.map((n, i) => (
                    <li key={i}>{n}</li>
                  ))}
                </ul>
              )}
            </Card>
          )}
          {which === 'custom' && <CustomEditor dims={dims} setDims={setDims} lower={lower} setLower={setLower} upper={upper} setUpper={setUpper} />}
          <Explainer
            title="Three ways to add tolerances"
            refs={[
              { label: 'Budynas & Nisbett, "Shigley’s Mechanical Engineering Design", ch. 1 (tolerance stacks)' },
              { label: 'C. M. Creveling, "Tolerance Design: A Handbook for Developing Optimal Specifications", Addison-Wesley, 1997' },
            ]}
          >
            <Formula>{`worst case:  G_min = Σ (+ dims at min) − Σ (− dims at max)
RSS:         σ_G = √Σ (t_i / k_i)²,   G = Ḡ ± 3 σ_G  (99.73 %)
Monte Carlo: sample every dimension, count assemblies inside the limits`}</Formula>
            <p className="small">
              Worst case guarantees fit but over-tightens parts; RSS assumes independent, centred processes; Monte Carlo handles uniform and skewed distributions and gives the assembly yield directly.
            </p>
          </Explainer>
        </div>
      </div>
      {which === 'focus' && focus.data && <FocusCurve f={focus.data} />}
    </div>
  );
}

function stackCsv(title: string, dims: Dim[], s: StackOut): string {
  const rows: (string | number)[][] = [[`# ${title}`], ['dimension', 'nominal_mm', 'plus_mm', 'minus_mm', 'direction', 'distribution', 'sigmas', 'variance_pct']];
  dims.forEach((d, i) => rows.push([d.name, d.nominal, d.plus, d.minus, d.dir, d.dist, d.sigmas, +(s.contributions[i]?.variance_pct ?? 0).toFixed(3)]));
  rows.push([]);
  rows.push(['result', 'value_mm']);
  rows.push(['nominal', +s.nominal.toFixed(5)], ['worst_min', +s.worst_min.toFixed(5)], ['worst_max', +s.worst_max.toFixed(5)], ['rss_sigma', +s.rss_sigma.toFixed(5)], ['rss_min_3sigma', +s.rss_min.toFixed(5)], ['rss_max_3sigma', +s.rss_max.toFixed(5)], ['mc_mean', +s.mc_mean.toFixed(5)], ['mc_sigma', +s.mc_sigma.toFixed(5)], ['mc_yield_pct', +s.yield_pct.toFixed(3)], ['cpk', +s.cpk.toFixed(3)]);
  return toCsv(rows);
}

function StackDiagram({ dims, stack }: { dims: Dim[]; stack: StackOut }) {
  const W = 560;
  const rowH = 30;
  const H = 40 + rowH * (dims.length + 1) + 20;
  // Walk the chain: + dims go right, − dims go left.
  let pos = 0;
  const bars = dims.map((d) => {
    const a = pos;
    pos += (d.dir < 0 ? -1 : 1) * d.nominal;
    return { d, a, b: pos };
  });
  const xs = bars.flatMap((b) => [b.a, b.b]).concat([0, pos]);
  const lo = Math.min(...xs);
  const hi = Math.max(...xs);
  const span = Math.max(1e-9, hi - lo);
  const X = (v: number) => 24 + ((v - lo) / span) * (W - 48);
  const tolPx = (t: number) => Math.max(2, (t / span) * (W - 48));
  return (
    <div className="st-scroll-x">
      <svg className="st-diagram" viewBox={`0 0 ${W} ${H}`} style={{ minWidth: 420 }} role="img" aria-label="Tolerance stack diagram">
        <line className="dim" x1={X(0)} x2={X(0)} y1={16} y2={H - 16} />
        {bars.map(({ d, a, b }, i) => {
          const y = 30 + i * rowH;
          const x1 = Math.min(X(a), X(b));
          const w = Math.abs(X(b) - X(a));
          const right = d.dir >= 0;
          return (
            <g key={i}>
              <rect className={right ? 'part' : 'part2'} x={x1} y={y} width={Math.max(1, w)} height={14} rx={3} />
              <rect x={X(b) - tolPx(d.minus)} y={y - 3} width={tolPx(d.minus) + tolPx(d.plus)} height={20} fill="none" stroke="var(--series-2)" strokeWidth={1.5} />
              {(() => {
                const label = `${right ? '+' : '−'} ${fmt(d.nominal, 3)}  ${d.name} (+${fmt(d.plus, 3)}/−${fmt(d.minus, 3)}${d.dist === 'uniform' ? ', uniform' : ''})`;
                const est = label.length * 6.2;
                if (w > est + 12) return <text x={x1 + 6} y={y + 11} className="lbl">{label}</text>;
                const after = Math.max(X(a), X(b)) + 10;
                if (after + est < W - 8) return <text x={after} y={y + 11}>{label}</text>;
                return <text x={Math.max(8, x1 - 8)} y={y + 11} textAnchor={x1 - 8 > est ? 'end' : 'start'}>{label}</text>;
              })()}
            </g>
          );
        })}
        {(() => {
          const y = 30 + dims.length * rowH;
          return (
            <g>
              <rect x={Math.min(X(0), X(pos))} y={y} width={Math.max(2, Math.abs(X(pos) - X(0)))} height={14} rx={3} fill="var(--series-2)" opacity={0.85} />
              <text x={Math.max(X(0), X(pos)) + 8} y={y + 11} className="lbl">
                gap {fmt(stack.nominal, 3)} mm (worst {fmt(stack.worst_min, 3)} … {fmt(stack.worst_max, 3)})
              </text>
            </g>
          );
        })()}
        <text x={24} y={H - 6}>the orange frames show each dimension's tolerance band at the end of its bar</text>
      </svg>
    </div>
  );
}

function Results({ stack: s, focus }: { stack: StackOut; focus: FocusStackOut | null }) {
  const pareto = [...s.contributions].map((c, i) => ({ ...c, i })).sort((a, b) => b.variance_pct - a.variance_pct);
  const passWorst = (s.lower == null || s.worst_min >= s.lower) && (s.upper == null || s.worst_max <= s.upper);
  return (
    <div className="grid grid-2">
      <Card title="Results" subtitle={`Limits: ${s.lower != null ? `≥ ${fmt(s.lower, 3)} mm` : ''}${s.lower != null && s.upper != null ? ' and ' : ''}${s.upper != null ? `≤ ${fmt(s.upper, 3)} mm` : ''}`}>
        <div className="grid grid-2">
          <Stat label="Worst case" value={`${fmt(s.worst_min, 3)} … ${fmt(s.worst_max, 3)}`} unit="mm" tone={passWorst ? 'pass' : 'fail'} note={passWorst ? '✓ always inside the limits' : '✕ some combinations fall outside'} />
          <Stat label="RSS ±3σ" value={`${fmt(s.rss_min, 3)} … ${fmt(s.rss_max, 3)}`} unit="mm" note={`σ = ${fmt(s.rss_sigma * 1000, 1)} µm, yield ${fmt(s.rss_yield_pct, 3)} %${s.rss_capped ? ' · capped at the worst case' : ''}`} />
          <Stat label="Monte Carlo yield" value={fmt(s.yield_pct, 3)} unit="%" tone={s.yield_pct >= 99.73 ? 'pass' : s.yield_pct >= 95 ? 'warn' : 'fail'} note={`${fmt(s.mc_mean, 3)} ± ${fmt(s.mc_sigma * 1000, 1)} µm (1σ)`} />
          <Stat label="Process capability C_pk" value={fmt(s.cpk, 2)} tone={s.cpk >= 1.33 ? 'pass' : s.cpk >= 1 ? 'warn' : 'fail'} note="≥ 1.33 is a common target" />
        </div>
        {focus && (
          <div className="grid grid-2" style={{ marginTop: 12 }}>
            <Stat label="Focus error, worst case" value={fmt(focus.worst_diopters, 2)} unit="D" tone={focus.worst_diopters <= focus.allowable_d ? 'pass' : 'fail'} note={`allowance ±${fmt(focus.allowable_d, 2)} D`} />
            <Stat label="Sensitivity" value={fmt(focus.sensitivity_d_per_mm, 2)} unit="D/mm" note={`image at ${fmt(focus.image_distance_m, 2)} m; RSS ${fmt(focus.rss_diopters, 2)} D`} />
          </div>
        )}
      </Card>
      <Card title="Who contributes? (Pareto of the RSS variance)" subtitle="Tighten the top bar first — it buys the most">
        <div className="st-pareto">
          {pareto.map((c) => (
            <div key={c.i} className="st-pareto-row">
              <span>{c.name}</span>
              <div className="st-pareto-bar" aria-hidden>
                <div style={{ width: `${Math.max(1, c.variance_pct)}%`, background: `var(--series-${(c.i % 8) + 1})` }} />
              </div>
              <span className="tnum small">{fmt(c.variance_pct, 1)} %</span>
            </div>
          ))}
        </div>
        <div className="small muted" style={{ marginTop: 8 }}>
          Share of the worst-case range: {pareto.map((c) => `${c.name.split(' ')[0]} ${fmt(c.range_pct, 0)} %`).join(' · ')}
        </div>
      </Card>
    </div>
  );
}

/** Histogram that fills sample by sample (the first 4 000 draws), then shows all of them. */
function MonteCarlo({ stack: s }: { stack: StackOut }) {
  const [n, setN] = useState(prefersReducedMotion() ? s.samples_head.length : 0);
  const [playing, setPlaying] = useState(!prefersReducedMotion());
  useEffect(() => {
    setN(prefersReducedMotion() ? s.samples_head.length : 0);
    setPlaying(!prefersReducedMotion());
  }, [s]);
  useRaf(playing, (dt) =>
    setN((v) => {
      const next = v + Math.max(20, Math.round(dt * 1600));
      if (next >= s.samples_head.length) {
        setPlaying(false);
        return s.samples_head.length;
      }
      return next;
    }),
  );
  const done = n >= s.samples_head.length;
  const edges = s.hist_edges;
  const bins = edges.length - 1;
  const counts = useMemo(() => {
    if (done) return s.hist_counts;
    const c = new Array(bins).fill(0);
    const lo = edges[0];
    const w = (edges[bins] - lo) / bins;
    for (let i = 0; i < n; i++) {
      const k = Math.min(bins - 1, Math.max(0, Math.floor((s.samples_head[i] - lo) / w)));
      c[k]++;
    }
    return c;
  }, [n, done, s, bins, edges]);
  const total = done ? s.hist_counts.reduce((a, b) => a + b, 0) : n;
  const W = 560;
  const H = 230;
  const pad = 36;
  const cmax = Math.max(1, ...counts);
  const X = (v: number) => pad + ((v - edges[0]) / (edges[bins] - edges[0])) * (W - 2 * pad);
  const Y = (c: number) => H - pad - (c / cmax) * (H - 2 * pad - 10);
  const inside = (v: number) => (s.lower == null || v >= s.lower) && (s.upper == null || v <= s.upper);
  return (
    <Card
      title="Monte Carlo: build the assembly thousands of times"
      subtitle={`${total.toLocaleString()} assemblies drawn — each draws every dimension from its distribution`}
      actions={
        <button className="btn small" onClick={() => { if (done) setN(0); setPlaying(!playing); }} aria-pressed={playing}>
          {playing ? '❚❚ Pause' : done ? '↺ Replay' : '▶ Play'}
        </button>
      }
    >
      <div className="st-scroll-x">
        <svg className="st-diagram" viewBox={`0 0 ${W} ${H}`} style={{ minWidth: 420 }} role="img" aria-label="Histogram of the gap with limits, worst-case and RSS bands">
          <rect x={X(s.worst_min)} y={14} width={Math.max(1, X(s.worst_max) - X(s.worst_min))} height={H - pad - 14} fill="color-mix(in srgb, var(--series-5) 10%, transparent)" />
          <rect x={X(s.rss_min)} y={14} width={Math.max(1, X(s.rss_max) - X(s.rss_min))} height={H - pad - 14} fill="color-mix(in srgb, var(--series-1) 12%, transparent)" />
          {counts.map((c, i) => {
            const mid = 0.5 * (edges[i] + edges[i + 1]);
            return <rect key={i} x={X(edges[i]) + 0.5} y={Y(c)} width={Math.max(1, X(edges[i + 1]) - X(edges[i]) - 1)} height={H - pad - Y(c)} fill={inside(mid) ? 'var(--series-1)' : 'var(--bad)'} opacity={0.85} />;
          })}
          {[s.lower, s.upper].map((v, i) =>
            v == null || v < edges[0] || v > edges[bins] ? null : (
              <g key={i}>
                <line x1={X(v)} x2={X(v)} y1={10} y2={H - pad} stroke="var(--text)" strokeWidth={1.5} strokeDasharray="5 3" />
                <text x={X(v) + 4} y={22}>{i === 0 ? 'lower limit' : 'upper limit'}</text>
              </g>
            ),
          )}
          <line className="ground" x1={pad} x2={W - pad} y1={H - pad} y2={H - pad} />
          <text x={pad} y={H - 12}>{fmt(edges[0], 3)} mm</text>
          <text x={W - pad} y={H - 12} textAnchor="end">{fmt(edges[bins], 3)} mm</text>
          <text x={W / 2} y={H - 12} textAnchor="middle">gap</text>
        </svg>
      </div>
      <div className="cp-legend">
        <span><i className="cp-legend-dot" style={{ background: 'var(--series-1)' }} /> inside the limits</span>
        <span><i className="cp-legend-dot" style={{ background: 'var(--bad)' }} /> ✕ outside (rejects)</span>
        <span><i className="cp-legend-dot" style={{ background: 'color-mix(in srgb, var(--series-1) 40%, transparent)' }} /> RSS ±3σ band</span>
        <span><i className="cp-legend-dot" style={{ background: 'color-mix(in srgb, var(--series-5) 40%, transparent)' }} /> worst-case range</span>
      </div>
    </Card>
  );
}

function FocusCurve({ f }: { f: FocusStackOut }) {
  return (
    <Card title="Focus error vs display spacing" subtitle={`V = 1000/f − 1000/d: at d = ${fmt(f.nominal_mm, 2)} mm and f = ${fmt(f.focal_mm, 1)} mm the image moves ${fmt(f.sensitivity_d_per_mm, 2)} D per millimetre`}>
      <Plot
        series={[{ name: 'focus error', x: f.curve_mm, y: f.curve_d }]}
        xLabel="display-to-lens spacing, mm"
        yLabel="focus error, D"
        height={200}
        hLines={[{ value: f.allowable_d, label: `+${f.allowable_d} D` }, { value: -f.allowable_d, label: `−${f.allowable_d} D` }]}
        vLines={[{ value: f.stack.worst_min, label: 'worst min' }, { value: f.stack.worst_max, label: 'worst max' }]}
      />
      <div className="small muted">
        Ties to the Optics Lab: the same lens equation sets the virtual image distance (requirement on <code>optics.virtual_image_m</code>). If the worst case exceeds the allowance, a focus adjustment is cheaper than tighter parts.
      </div>
    </Card>
  );
}

function CustomEditor({ dims, setDims, lower, setLower, upper, setUpper }: { dims: Dim[]; setDims: (d: Dim[]) => void; lower: number; setLower: (v: number) => void; upper: number | null; setUpper: (v: number | null) => void }) {
  const set = (i: number, q: Partial<Dim>) => setDims(dims.map((d, k) => (k === i ? { ...d, ...q } : d)));
  return (
    <Card title="Custom chain">
      <div className="stack" style={{ gap: 10 }}>
        {dims.map((d, i) => (
          <div key={i} className="st-table-input" style={{ borderBottom: '1px dashed var(--border)', paddingBottom: 8 }}>
            <input value={d.name} onChange={(e) => set(i, { name: e.target.value })} aria-label="Dimension name" style={{ marginBottom: 6 }} />
            <div className="grid grid-4" style={{ gap: 6 }}>
              <input type="number" step="0.01" value={d.nominal} onChange={(e) => set(i, { nominal: Number(e.target.value) })} aria-label="Nominal" title="nominal, mm" />
              <input type="number" step="0.005" value={d.plus} onChange={(e) => set(i, { plus: Number(e.target.value), minus: Number(e.target.value) })} aria-label="Tolerance plus/minus" title="± tolerance, mm" />
              <select value={d.dir} onChange={(e) => set(i, { dir: Number(e.target.value) })} aria-label="Direction">
                <option value={1}>+ adds</option>
                <option value={-1}>− subtracts</option>
              </select>
              <select value={d.dist} onChange={(e) => set(i, { dist: e.target.value as Dim['dist'] })} aria-label="Distribution">
                <option value="normal">normal</option>
                <option value="uniform">uniform</option>
              </select>
            </div>
          </div>
        ))}
        <div className="row">
          <button className="btn small" onClick={() => setDims([...dims, { name: `Dimension ${dims.length + 1}`, nominal: 1, plus: 0.05, minus: 0.05, dir: -1, dist: 'normal', sigmas: 3 }])}>
            + Add dimension
          </button>
          {dims.length > 1 && (
            <button className="btn small" onClick={() => setDims(dims.slice(0, -1))}>
              − Remove last
            </button>
          )}
        </div>
        <Slider label="Lower limit on the gap" value={lower} min={-0.5} max={1} step={0.01} unit="mm" onChange={setLower} />
        <Toggle label="Upper limit" value={upper != null} onChange={(v) => setUpper(v ? 0.3 : null)} />
        {upper != null && <Slider label="Upper limit on the gap" value={upper} min={-0.5} max={2} step={0.01} unit="mm" onChange={setUpper} />}
        <div className="small muted">Example values for the axial play of a hinge knuckle (slot − lug − washer) — replace them with the dimensions from your drawings.</div>
      </div>
    </Card>
  );
}
