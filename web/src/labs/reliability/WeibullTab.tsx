// Weibull analysis: failure data with suspensions → median ranks, rank
// regression and maximum likelihood, bounds on Weibull paper, pdf / hazard /
// reliability curves, an estimator study and the bathtub curve.
import { useEffect, useMemo, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { WeibullPaper, type PaperLine } from './WeibullPaper';
import { Bathtub } from './Bathtub';
import type { FitOut, IrisOut, SampleSet, StudyOut } from './types';
import { fmt, prefersReducedMotion, toCsv, useRelConfig } from './util';

const METHOD_COLORS: Record<string, string> = { rrx: 'var(--series-3)', rry: 'var(--series-4)', mle: 'var(--series-1)', weibayes: 'var(--series-7)' };

export function WeibullTab() {
  const sets = useCompute<SampleSet[]>('reliability.weibull_sets', {}).data ?? [];
  const [setId, setSetId] = useState('hinge');
  const [text, setText] = useState<string | null>(null);
  const [unit, setUnit] = useState<string | null>(null);
  const [method, setMethod] = useState('mle');
  const [bounds, setBounds] = useState('fisher');
  const [cl, setCl] = useState(0.9);
  const [betaAssumed, setBetaAssumed] = useState(2);
  const [compare, setCompare] = useState(true);
  const [curve, setCurve] = useState<'rel' | 'pdf' | 'hazard'>('rel');
  const set = sets.find((s) => s.id === setId);
  const dataText = text ?? set?.text ?? '';
  const u = unit ?? set?.unit ?? 'hours';
  const input = useMemo(() => ({ text: dataText, method, bounds, cl, beta_assumed: betaAssumed, unit: u }), [dataText, method, bounds, cl, betaAssumed, u]);
  const { data: r, error, ms } = useCompute<FitOut>('reliability.weibull_fit', input, dataText.trim().length > 0);

  // build-up animation of the points
  const nPts = r?.rows.filter((x) => x.failed).length ?? 0;
  const [reveal, setReveal] = useState<number | null>(null);
  useEffect(() => {
    if (reveal == null) return;
    if (reveal >= nPts) {
      const t = setTimeout(() => setReveal(null), 600);
      return () => clearTimeout(t);
    }
    const t = setTimeout(() => setReveal((v) => (v == null ? v : v + 1)), 380);
    return () => clearTimeout(t);
  }, [reveal, nPts]);

  const points = r ? r.rows.filter((x) => x.failed && x.median_rank != null).map((x) => ({ t: x.t, f: x.median_rank! })) : [];
  const susp = r ? r.rows.filter((x) => !x.failed).map((x) => x.t) : [];
  const lines: PaperLine[] = r
    ? [
        { name: r.fits.find((f) => f.method === r.method)?.label ?? r.method, beta: r.beta, eta: r.eta, color: METHOD_COLORS[r.method] ?? 'var(--series-1)' },
        ...(compare ? r.fits.filter((f) => f.ok && f.method !== r.method && f.method !== 'weibayes').map((f) => ({ name: f.label, beta: f.beta, eta: f.eta, color: METHOD_COLORS[f.method], dash: '2 3' })) : []),
      ]
    : [];
  const ranksCsv = () =>
    r ? toCsv([['time', 'status', 'reverse_rank', 'adjusted_rank', 'median_rank_F'], ...r.rows.map((x) => [x.t, x.failed ? 'F' : 'S', x.reverse_rank, x.adjusted_rank, x.median_rank])]) : '';
  const fitJson = () => (r ? JSON.stringify({ unit: u, method: r.method, beta: r.beta, eta: r.eta, cl: r.cl, bounds: r.bounds_method, beta_bounds: r.beta_bounds, eta_bounds: r.eta_bounds, b_lives: r.b_lives, fits: r.fits }, null, 2) : '');
  const curveSeries = r
    ? curve === 'rel'
      ? [{ name: 'R(t) reliability', x: r.curves.t, y: r.curves.rel }, { name: 'F(t) unreliability', x: r.curves.t, y: r.curves.cdf, dash: '4 3' }]
      : curve === 'pdf'
        ? [{ name: 'f(t) probability density', x: r.curves.t, y: r.curves.pdf, area: true }]
        : [{ name: 'h(t) hazard rate', x: r.curves.t, y: r.curves.hazard }]
    : [];
  return (
    <div className="rl-layout">
      <div className="stack" style={{ minWidth: 0 }}>
        <ErrorBox error={error} />
        {r && (
          <div className="grid grid-4">
            <Stat label="Shape β" value={fmt(r.beta, 3)} note={r.beta_bounds ? `${Math.round(r.cl * 100)} %: ${fmt(r.beta_bounds[0])} – ${fmt(r.beta_bounds[1])}` : r.method === 'weibayes' ? 'assumed' : undefined} />
            <Stat label="Characteristic life η" value={fmt(r.eta, 4)} unit={u} note={r.eta_bounds ? `${Math.round(r.cl * 100)} %: ${fmt(r.eta_bounds[0])} – ${fmt(r.eta_bounds[1])}` : undefined} />
            <Stat label="B10 life (10 % failed)" value={fmt(r.b_lives[2].t, 4)} unit={u} note={r.b_lives[2].lo != null ? `lower bound ${fmt(r.b_lives[2].lo)}` : undefined} />
            <Stat label="Mean life (MTTF)" value={fmt(r.mttf, 4)} unit={u} note={`${r.failures} failures, ${r.suspensions} suspensions`} />
          </div>
        )}
        <Card
          title="Weibull probability plot"
          subtitle="On Weibull paper a Weibull distribution is a straight line: slope = β, and it crosses 63.2 % at t = η. Hover to read F and R."
          actions={
            <div className="row">
              <button className="btn small" onClick={() => setReveal(prefersReducedMotion() ? nPts : 0)} disabled={!r}>▶ Build the plot</button>
              {r && <DownloadButton small filename="weibull-fit.json" label="fit JSON" content={fitJson} />}
            </div>
          }
        >
          {r && (
            <WeibullPaper
              points={points}
              suspensions={susp}
              lines={lines}
              bounds={r.bound_lo.length ? { f: r.line_f, lo: r.bound_lo, hi: r.bound_hi } : null}
              unit={u}
              reveal={reveal ?? undefined}
              highlightB={0.1}
            />
          )}
          {reveal != null && r && (
            <div className="small muted" aria-live="polite">
              {reveal < nPts
                ? `Point ${reveal + 1}: t = ${fmt(points[reveal]?.t)} ${u}, adjusted rank ${fmt(r.rows.filter((x) => x.failed)[reveal]?.adjusted_rank ?? 0, 4)} → F = (rank − 0.3)/(N + 0.4) = ${((points[reveal]?.f ?? 0) * 100).toFixed(1)} %`
                : 'All points placed — now the line through them.'}
            </div>
          )}
          {r && <div className="rl-verdict" style={{ marginTop: 8 }}>{r.interpretation}</div>}
          {r?.notes.map((n, i) => (
            <div key={i} className="small muted" style={{ marginTop: 4 }}>• {n}</div>
          ))}
        </Card>
        {r && (
          <Card title="Three fits of the same data" subtitle="Rank regression fits a line on the paper; maximum likelihood uses the exact failure times and the suspensions. They differ most for small samples.">
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr><th>Method</th><th>β</th><th>η ({u})</th><th>B10</th><th>B1</th><th>ρ²</th><th>log-likelihood</th></tr>
                </thead>
                <tbody>
                  {r.fits.map((f) => (
                    <tr key={f.method} className={f.method === r.method ? 'rl-row-active' : undefined}>
                      <td><i className="rl-line" style={{ background: METHOD_COLORS[f.method] }} /> {f.label}{f.method === r.method && ' ✓'}</td>
                      {f.ok ? (
                        <>
                          <td className="tnum">{fmt(f.beta, 3)}</td>
                          <td className="tnum">{fmt(f.eta, 4)}</td>
                          <td className="tnum">{fmt(f.b10, 3)}</td>
                          <td className="tnum">{fmt(f.b1, 3)}</td>
                          <td className="tnum">{f.r2 != null ? f.r2.toFixed(4) : '—'}</td>
                          <td className="tnum">{f.loglik.toFixed(2)}</td>
                        </>
                      ) : (
                        <td colSpan={6} className="muted small">{f.error}</td>
                      )}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="small muted" style={{ marginTop: 6 }}>Computed in {ms.toFixed(1)} ms. Log-likelihoods omit the constant Σ ln t over failures, so compare them only within one data set.</div>
          </Card>
        )}
        {r && (
          <Card
            title="Life curves"
            actions={
              <div className="rl-seg" role="tablist">
                {(['rel', 'pdf', 'hazard'] as const).map((c) => (
                  <button key={c} className={curve === c ? 'active' : ''} onClick={() => setCurve(c)} aria-pressed={curve === c}>
                    {c === 'rel' ? 'R(t) and F(t)' : c === 'pdf' ? 'pdf f(t)' : 'hazard h(t)'}
                  </button>
                ))}
              </div>
            }
          >
            <Plot series={curveSeries} xLabel={`time (${u})`} yLabel={curve === 'rel' ? 'probability' : curve === 'pdf' ? `density, 1/${u}` : `failures per ${u}`} legend height={230} yZero
              vLines={[{ value: r.b_lives[2].t, label: 'B10' }, { value: r.eta, label: 'η' }]} />
          </Card>
        )}
        {r && (
          <Card title="Ranks table" subtitle="Johnson's adjusted ranks spread each suspension's share over the later failures; Bernard's approximation turns a rank into a median rank F." actions={<DownloadButton small filename="weibull-ranks.csv" label="ranks CSV" content={ranksCsv} />}>
            <div className="table-wrap" style={{ maxHeight: 280 }}>
              <table className="table">
                <thead><tr><th>#</th><th>Time</th><th>Status</th><th>Reverse rank</th><th>Adjusted rank</th><th>Median rank F</th></tr></thead>
                <tbody>
                  {r.rows.map((x, i) => (
                    <tr key={i}>
                      <td className="tnum">{i + 1}</td>
                      <td className="tnum">{fmt(x.t, 5)}</td>
                      <td>{x.failed ? '✕ failure' : '+ suspension'}</td>
                      <td className="tnum">{x.reverse_rank}</td>
                      <td className="tnum">{x.adjusted_rank != null ? x.adjusted_rank.toFixed(3) : '—'}</td>
                      <td className="tnum">{x.median_rank != null ? `${(x.median_rank * 100).toFixed(2)} %` : '—'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
        )}
        <EstimatorStudy />
        <Bathtub />
      </div>
      <div className="stack rl-side">
        <Card title="Failure data">
          <div className="rl-sets">
            {sets.map((s) => (
              <button key={s.id} className={`rl-set ${s.id === setId && text == null ? 'active' : ''}`} onClick={() => { setSetId(s.id); setText(null); setUnit(null); }}>
                <b>{s.title}</b>
                <span className="small muted">{s.unit}</span>
              </button>
            ))}
          </div>
          {set && text == null && (
            <div className="small muted" style={{ margin: '8px 0' }}>
              {set.description} <em>{set.source}</em>
            </div>
          )}
          <label className="field">
            <span className="field-top"><span className="field-label">Times (one per line; 123+ or "123,S" = suspension)</span></span>
            <textarea className="rl-textarea mono" rows={9} value={dataText} onChange={(e) => setText(e.target.value)} spellCheck={false} aria-label="Failure and suspension times" />
          </label>
          <div className="row" style={{ flexWrap: 'wrap' }}>
            <label className="btn small">
              ↑ CSV file
              <input
                type="file"
                accept=".csv,.txt,text/csv,text/plain"
                className="sr-only"
                onChange={(e) => {
                  const f = e.target.files?.[0];
                  if (f) f.text().then((t) => setText(t));
                  e.target.value = '';
                }}
              />
            </label>
            <DownloadButton small filename="weibull-data.csv" label="data" content={() => dataText} />
            <label className="field" style={{ flex: 1, minWidth: 110, marginBottom: 0 }}>
              <input type="text" value={u} onChange={(e) => setUnit(e.target.value)} aria-label="Time unit" placeholder="unit" />
            </label>
          </div>
          <div className="small muted" style={{ marginTop: 6 }}>CSV: a time column and an optional status column (F/S, 1/0) and count column; a header row is skipped.</div>
        </Card>
        <Card title="Analysis">
          <Select label="Fit method" value={method} onChange={setMethod} options={[
            { value: 'mle', label: 'Maximum likelihood (MLE)' },
            { value: 'rrx', label: 'Rank regression on X (RRX)' },
            { value: 'rry', label: 'Rank regression on Y (RRY)' },
            { value: 'weibayes', label: 'Weibayes (β assumed)' },
          ]} />
          {method === 'weibayes' && <Slider label="Assumed β" value={betaAssumed} min={0.5} max={6} step={0.1} onChange={setBetaAssumed} hint="From prior tests of the same failure mode" />}
          <Select label="Confidence bounds" value={bounds} onChange={setBounds} options={[
            { value: 'fisher', label: 'Fisher matrix' },
            { value: 'lr', label: 'Likelihood ratio' },
            { value: 'none', label: 'None' },
          ]} />
          <Slider label="Confidence level (two-sided)" value={cl} min={0.8} max={0.99} step={0.01} onChange={setCl} format={(v) => `${Math.round(v * 100)} %`} />
          <label className="small" style={{ display: 'flex', gap: 8, cursor: 'pointer' }}>
            <input type="checkbox" checked={compare} onChange={(e) => setCompare(e.target.checked)} style={{ accentColor: 'var(--accent)' }} /> Show the other fits on the plot
          </label>
        </Card>
        {r && <UseForBlock beta={r.beta} eta={r.eta} unit={u} method={r.method} />}
        <Explainer title="Median ranks and suspensions" refs={[{ label: 'R. B. Abernethy, The New Weibull Handbook, 5th ed., 2006, ch. 2–5' }, { label: 'L. G. Johnson, The Statistical Treatment of Fatigue Experiments, Elsevier 1964' }]}>
          <Formula>{`adjusted rank = (RR·prev + N + 1) / (RR + 1)
F ≈ (rank − 0.3) / (N + 0.4)        (Bernard)
y = ln(−ln(1 − F)) = β ln t − β ln η`}</Formula>
          RR is the reverse rank (items still on test, counting this one). A suspension (a unit removed or still running) never gets a point but pushes the later failures' ranks up.
        </Explainer>
        <Explainer title="Maximum likelihood and bounds" refs={[{ label: 'W. Q. Meeker & L. A. Escobar, Statistical Methods for Reliability Data, Wiley 1998, ch. 8' }, { label: 'NIST/SEMATECH e-Handbook of Statistical Methods, §8.4', href: 'https://www.itl.nist.gov/div898/handbook/apr/section4/apr4.htm' }]}>
          <Formula>{`ln L = Σ_fail ln f(tᵢ) + Σ_susp ln R(tⱼ)
η̂^β = Σ tᵢ^β / r,   1/β + mean ln t_f − Σ t^β ln t / Σ t^β = 0
Fisher: Var(ln t_p) ≈ ∇u · I⁻¹ · ∇u,  u = ln η + y_p/β
LR: {t_p : 2[ℓ̂ − ℓ*(t_p)] ≤ χ²₁(CL)}`}</Formula>
          MLE uses every exact time and handles heavy censoring well, but its β is biased high for small samples (run the study below). Likelihood-ratio bounds are more accurate than Fisher bounds when failures are few.
        </Explainer>
      </div>
    </div>
  );
}

function UseForBlock({ beta, eta, unit, method }: { beta: number; eta: number; unit: string; method: string }) {
  const project = useProject((s) => s.project);
  const iris = useCompute<IrisOut>('reliability.iris', { project, samples: 0 }).data;
  const [cfg, setCfg] = useRelConfig();
  const wear = iris?.rbd.blocks.filter((b) => b.model === 'weibull') ?? [];
  const [ref, setRef] = useState('');
  const target = wear.find((b) => b.refdes === ref) ?? wear[0];
  if (!target) return null;
  const mismatch = target.life_unit && unit && !unit.toLowerCase().includes(target.life_unit.replace(/s$/, '').toLowerCase()) && !(target.life_unit === 'folds' && unit.includes('cycle')) && !(target.life_unit === 'cycles' && unit.includes('fold'));
  const ov = cfg.blocks[target.refdes];
  return (
    <Card title="Use this fit in the Iris block diagram" subtitle="Wear-out items in the System tab take their β and η from here">
      <Select label="Block" value={target.refdes} onChange={setRef} options={wear.map((b) => ({ value: b.refdes, label: `${b.refdes} · ${b.label} (${b.life_unit})` }))} />
      <div className="small">
        Now: β = {fmt(target.beta)}, η = {fmt(target.eta, 4)} {target.life_unit} ({target.source}). This fit: β = {fmt(beta)}, η = {fmt(eta, 4)} {unit}.
      </div>
      {mismatch && <div className="small" style={{ color: 'var(--warn)', marginTop: 4 }}>! The data are in {unit}; the block counts {target.life_unit}. Check the units before applying.</div>}
      <div className="row" style={{ marginTop: 8, flexWrap: 'wrap' }}>
        <button className="btn small primary" onClick={() => setCfg({ blocks: { ...cfg.blocks, [target.refdes]: { beta, eta, source: `Weibull tab (${method.toUpperCase()})` } } })}>Apply to {target.refdes}</button>
        {ov && <button className="btn small" onClick={() => { const b = { ...cfg.blocks }; delete b[target.refdes]; setCfg({ blocks: b }); }}>Back to typical</button>}
      </div>
    </Card>
  );
}

function EstimatorStudy() {
  const [n, setN] = useState(8);
  const [ff, setFf] = useState(1);
  const [beta, setBeta] = useState(2);
  const input = useMemo(() => ({ beta, eta: 1000, n, fail_fraction: ff, reps: 400, seed: 11 }), [beta, n, ff]);
  const { data } = useCompute<StudyOut>('reliability.weibull_study', input);
  const hist = useMemo(() => {
    if (!data) return [];
    const lo = 0;
    const hi = beta * 3;
    const bins = 36;
    const w = (hi - lo) / bins;
    return data.methods.map((m) => {
      const c = new Array(bins).fill(0);
      m.betas.forEach((b) => {
        const k = Math.floor((b - lo) / w);
        if (k >= 0 && k < bins) c[k]++;
      });
      return { name: m.method.toUpperCase(), x: c.map((_, i) => lo + (i + 0.5) * w), y: c.map((v) => v / m.betas.length), step: true };
    });
  }, [data, beta]);
  return (
    <Card title="Which estimator should I trust? A Monte Carlo study" subtitle="400 synthetic tests drawn from a known Weibull (η = 1000): how spread out and how biased is the fitted β?">
      <div className="grid grid-3">
        <Slider label="Units per test" value={n} min={3} max={40} step={1} onChange={(v) => setN(Math.round(v))} />
        <Slider label="Fraction failed when the test stops" value={ff} min={0.2} max={1} step={0.05} onChange={setFf} format={(v) => `${Math.round(v * 100)} %`} />
        <Slider label="True β" value={beta} min={0.5} max={5} step={0.1} onChange={setBeta} />
      </div>
      {data && (
        <>
          <Plot series={hist.map((h, i) => ({ ...h, color: ['var(--series-3)', 'var(--series-4)', 'var(--series-1)'][i] }))} xLabel="fitted β" yLabel="fraction of tests" legend height={210} vLines={[{ value: beta, label: 'true β' }]} yZero />
          <div className="table-wrap">
            <table className="table">
              <thead><tr><th>Method</th><th>median β̂</th><th>mean β̂</th><th>90 % of tests between</th><th>median B10 (true {fmt(data.true_b10)})</th></tr></thead>
              <tbody>
                {data.methods.map((m) => (
                  <tr key={m.method}>
                    <td>{m.method.toUpperCase()}</td>
                    <td className="tnum">{fmt(m.beta_median)}</td>
                    <td className="tnum">{fmt(m.beta_mean)}</td>
                    <td className="tnum">{fmt(m.beta_p05)} – {fmt(m.beta_p95)}</td>
                    <td className="tnum">{fmt(m.b10_median)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="small muted" style={{ marginTop: 6 }}>
            With {n} units the fitted β scatters widely whatever the method; MLE's β is biased high for small samples, which is why Abernethy recommends rank regression (RRX) below about 20 failures and
            MLE for heavy censoring. {data.skipped > 0 && `${data.skipped} simulated tests had fewer than two failures and were skipped.`}
          </div>
        </>
      )}
    </Card>
  );
}
