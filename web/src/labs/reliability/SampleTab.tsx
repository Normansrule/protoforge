// Sample size and demonstration testing: success run, binomial with allowed
// failures, the Lipson/Weibayes extension and operating-characteristic curves,
// with an animated "run the test many times" demonstration.
import { useEffect, useMemo, useState } from 'react';
import { Card, DownloadButton, Explainer, Formula, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import type { SampleOut } from './types';
import { fmt, mulberry32, pct, prefersReducedMotion, toCsv, useRelConfig } from './util';

export function SampleTab() {
  const [cfg] = useRelConfig();
  const [r, setR] = useState(cfg.dvt_reliability);
  const [c, setC] = useState(cfg.dvt_confidence);
  const [beta, setBeta] = useState(cfg.life_beta);
  const [k, setK] = useState(cfg.life_multiple);
  const [planN, setPlanN] = useState(22);
  const [planF, setPlanF] = useState(0);
  const [design, setDesign] = useState(0.98);
  const input = useMemo(() => ({ reliability: r, confidence: c, max_failures: 3, beta, multiple: k, plan_n: planN, plan_failures: planF, design_reliability: design }), [r, c, beta, k, planN, planF, design]);
  const { data: s } = useCompute<SampleOut>('reliability.sample_size', input);
  const csv = () => (s ? toCsv([['failures_allowed', 'binomial_units', `lipson_units_${k}_lives_beta_${beta}`], ...s.rows.map((x) => [x.failures, x.binomial_n, x.lipson_n])]) : '');
  return (
    <div className="rl-layout">
      <div className="stack" style={{ minWidth: 0 }}>
        {s && (
          <div className="grid grid-4">
            <Stat label="Success run (0 failures)" value={s.success_run_n} unit="units" note={`ln(1 − ${c}) / ln ${r} = ${s.success_run_exact.toFixed(2)}`} />
            <Stat label="With 1 failure allowed" value={s.rows[1]?.binomial_n ?? '—'} unit="units" note="binomial" />
            <Stat label={`Lipson: ${k} lives, β = ${beta}`} value={s.rows[0].lipson_n} unit="units" note="zero failures" />
            <Stat label={`Plan ${planN} units / ≤ ${planF} fail proves`} value={pct(s.demonstrated_r, 2)} note={`R at ${Math.round(c * 100)} % confidence`} />
          </div>
        )}
        <Card title="Units needed" subtitle={`Demonstrate R = ${r} with ${Math.round(c * 100)} % confidence`} actions={s && <DownloadButton small filename="sample-sizes.csv" label="table CSV" content={csv} />}>
          {s && (
            <div className="grid grid-2">
              <div className="table-wrap">
                <table className="table">
                  <thead><tr><th>Failures allowed</th><th>Binomial (1 life)</th><th>Lipson ({k} lives, β = {beta})</th></tr></thead>
                  <tbody>
                    {s.rows.map((x) => (
                      <tr key={x.failures}><td className="tnum">{x.failures}</td><td className="tnum">{x.binomial_n}</td><td className="tnum">{x.lipson_n}</td></tr>
                    ))}
                  </tbody>
                </table>
                <div className="small muted" style={{ marginTop: 6 }}>Allowing a failure costs units but makes the test fairer to a good design (see the operating-characteristic curve).</div>
              </div>
              <Plot series={s.success_curves.map((cv) => ({ name: cv.label, x: cv.x, y: cv.y, step: true }))} xLabel="reliability to demonstrate" yLabel="units (0 failures)" yLog legend height={230} vLines={[{ value: r }]} />
            </div>
          )}
        </Card>
        <Card title="Lipson / Weibayes: test longer, test fewer" subtitle="Units needed (zero failures) against how many field lives each unit is tested, for several failure-mode shapes β.">
          {s && <Plot series={s.lipson_curves.map((cv) => ({ name: cv.label, x: cv.x, y: cv.y, step: true }))} xLabel="test length per unit, field lives k" yLabel="units" yLog legend height={240} vLines={[{ value: k, label: `k = ${k}` }]} />}
        </Card>
        <Card title="Operating-characteristic (OC) curve" subtitle="Probability that the test passes, for each true reliability. The plan protects the customer at R (≤ 1 − C chance of passing a design that is only R); the right side shows the producer's risk.">
          {s && (
            <>
              <Plot
                series={[
                  { name: `binomial plan: ${planN} units, ≤ ${planF} failures`, x: s.oc_r, y: s.oc_binomial },
                  { name: `Lipson plan: ${s.lipson_plan_n} units × ${k} lives, ≤ ${planF} failures (β = ${beta})`, x: s.oc_r, y: s.oc_lipson, dash: '5 3' },
                ]}
                xLabel="true reliability at one life"
                yLabel="P(pass)"
                legend
                height={250}
                yDomain={[0, 1.02]}
                vLines={[{ value: r, label: 'target R' }, { value: design, label: 'design R' }]}
                hLines={[{ value: 1 - c, label: '1 − C' }]}
              />
              <div className="grid grid-3" style={{ marginTop: 8 }}>
                <Stat label="Consumer's risk at target R" value={pct(s.consumer_risk, 1)} note="chance a marginal design passes" />
                <Stat label="Producer's risk, binomial" value={pct(s.producer_risk_binomial, 1)} tone={s.producer_risk_binomial > 0.2 ? 'warn' : undefined} note={`a design with R = ${design} fails the test`} />
                <Stat label="Producer's risk, Lipson" value={pct(s.producer_risk_lipson, 1)} tone={s.producer_risk_lipson > 0.2 ? 'warn' : undefined} note="same design" />
              </div>
            </>
          )}
        </Card>
        <Campaigns n={planN} fails={planF} />
      </div>
      <div className="stack rl-side">
        <Card title="Target">
          <Slider label="Reliability R to demonstrate" value={r} min={0.5} max={0.995} step={0.005} onChange={setR} />
          <Slider label="Confidence C" value={c} min={0.5} max={0.99} step={0.01} onChange={setC} format={(v) => `${Math.round(v * 100)} %`} />
          <Slider label="Failure-mode shape β (Lipson)" value={beta} min={0.5} max={5} step={0.1} onChange={setBeta} />
          <Slider label="Test length per unit k (lives)" value={k} min={0.5} max={3} step={0.1} onChange={setK} />
        </Card>
        <Card title="Plan on the OC curve">
          <Slider label="Units in the binomial plan" value={planN} min={2} max={80} step={1} format={(v) => String(Math.round(v))} onChange={(v) => setPlanN(Math.round(v))} />
          <Slider label="Failures allowed" value={planF} min={0} max={5} step={1} format={(v) => String(Math.round(v))} onChange={(v) => setPlanF(Math.round(v))} />
          <Slider label="Reliability you believe the design has" value={design} min={0.8} max={0.999} step={0.001} onChange={setDesign} />
          <button className="btn small" onClick={() => s && setPlanN(s.rows[Math.min(planF, s.rows.length - 1)].binomial_n)}>Use the minimum units for {planF} failure(s)</button>
        </Card>
        <Explainer title="Success run and the binomial" refs={[{ label: 'NIST/SEMATECH e-Handbook of Statistical Methods, §8.3.1', href: 'https://www.itl.nist.gov/div898/handbook/apr/section3/apr31.htm' }, { label: 'O’Connor & Kleyner, Practical Reliability Engineering, 5th ed., ch. 14' }]}>
          <Formula>{`0 failures:  R^n ≤ 1 − C  →  n = ln(1 − C)/ln R
r failures:  Σ_{i≤r} C(n,i)(1−R)^i R^(n−i) ≤ 1 − C
R = 0.9, C = 0.9  →  n = 21.85 → 22`}</Formula>
          If every one of 22 units survives a one-life test, a design whose true reliability were only 0.9 would pass that often less than 10 % of the time — so you are 90 % confident R ≥ 0.9.
        </Explainer>
        <Explainer title="Lipson extension" refs={[{ label: 'C. Lipson & N. J. Sheth, Statistical Design and Analysis of Engineering Experiments, 1973' }, { label: 'Abernethy, The New Weibull Handbook, ch. 6' }]}>
          <Formula>{`R(k·L) = R(L)^(k^β)   →   n = ln(1 − C) / (k^β ln R)`}</Formula>
          Valid only if β is known for that failure mode and the longer test does not trigger a new mechanism.
        </Explainer>
      </div>
    </div>
  );
}

/** Run the plan many times against a "true" design and watch the pass rate converge to the OC curve. */
function Campaigns({ n, fails }: { n: number; fails: number }) {
  const [trueR, setTrueR] = useState(0.95);
  const [runs, setRuns] = useState<{ failures: number; pass: boolean }[]>([]);
  const [current, setCurrent] = useState<boolean[] | null>(null);
  const [auto, setAuto] = useState(false);
  const [seed, setSeed] = useState(7);
  const rng = useMemo(() => mulberry32(seed), [seed]);
  useEffect(() => {
    setRuns([]);
    setCurrent(null);
  }, [n, fails, trueR]);
  const runOne = () => {
    const units = Array.from({ length: n }, () => rng() < trueR);
    const f = units.filter((x) => !x).length;
    setCurrent(units);
    setRuns((r) => [...r.slice(-499), { failures: f, pass: f <= fails }]);
  };
  useEffect(() => {
    if (!auto) return;
    const id = setInterval(runOne, prefersReducedMotion() ? 20 : 160);
    return () => clearInterval(id);
  });
  const passRate = runs.length ? runs.filter((x) => x.pass).length / runs.length : 0;
  const exact = binomCdf(fails, n, 1 - trueR);
  return (
    <Card
      title="Run the test many times"
      subtitle="Each campaign tests the plan's units against a design whose true reliability you choose; green units survive one life. The pass rate converges to the OC curve."
      actions={
        <div className="row">
          <button className="btn small" onClick={runOne}>Run once</button>
          <button className="btn small primary" onClick={() => setAuto((a) => !a)} aria-pressed={auto}>{auto ? '❚❚ Stop' : '▶ Run many'}</button>
          <button className="btn small" onClick={() => { setRuns([]); setCurrent(null); setSeed((x) => x + 1); }}>↺ Reset</button>
        </div>
      }
    >
      <Slider label="True reliability of the design" value={trueR} min={0.8} max={0.999} step={0.001} onChange={setTrueR} />
      <div className="rl-units" aria-label={current ? `${current.filter((x) => !x).length} of ${n} units failed` : 'no campaign yet'}>
        {(current ?? new Array(n).fill(null)).map((u, i) => (
          <span key={i} className={`rl-unit ${u == null ? '' : u ? 'ok' : 'bad'}`}>{u == null ? '' : u ? '✓' : '✕'}</span>
        ))}
      </div>
      <div className="grid grid-3" style={{ marginTop: 8 }}>
        <Stat label="Campaigns run" value={runs.length} />
        <Stat label="Pass rate (simulated)" value={runs.length ? pct(passRate, 1) : '—'} />
        <Stat label="Pass probability (exact)" value={pct(exact, 1)} note={`binomial, ≤ ${fails} of ${n} fail`} />
      </div>
      <div className="small muted">Last result: {current ? `${current.filter((x) => !x).length} failure(s) → ${current.filter((x) => !x).length <= fails ? '✓ pass' : '✕ fail'}` : '—'} · simulated with a seeded generator for illustration; the exact value is the OC curve above ({fmt(trueR)}).</div>
    </Card>
  );
}

function binomCdf(r: number, n: number, p: number) {
  let s = 0;
  let term = (1 - p) ** n;
  for (let i = 0; i <= r; i++) {
    s += term;
    term *= ((n - i) / (i + 1)) * (p / (1 - p));
  }
  return Math.min(1, s);
}
