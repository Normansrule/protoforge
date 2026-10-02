// "How it works": the vocabulary and ideas behind every tab, the MTBF
// misconception animated, the pipeline from data to a test plan, tools to
// graduate to and references.
import { useMemo, useState } from 'react';
import { Card, Explainer, Formula } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import type { Obs } from './types';
import { mulberry32, prefersReducedMotion, useRaf } from './util';

export const REFS = {
  abernethy: { label: 'R. B. Abernethy, The New Weibull Handbook, 5th ed., 2006' },
  meeker: { label: 'W. Q. Meeker & L. A. Escobar, Statistical Methods for Reliability Data, Wiley 1998' },
  nelson: { label: 'W. Nelson, Accelerated Testing: Statistical Models, Test Plans and Data Analyses, Wiley 1990' },
  oconnor: { label: 'P. D. T. O’Connor & A. Kleyner, Practical Reliability Engineering, 5th ed., Wiley 2012' },
  nist: { label: 'NIST/SEMATECH e-Handbook of Statistical Methods, ch. 8 “Assessing product reliability”', href: 'https://www.itl.nist.gov/div898/handbook/apr/apr.htm' },
  mil217: { label: 'MIL-HDBK-217F, Reliability Prediction of Electronic Equipment, 1991 (Notice 2, 1995)' },
  lipson: { label: 'C. Lipson & N. J. Sheth, Statistical Design and Analysis of Engineering Experiments, McGraw-Hill 1973' },
  lawless: { label: 'J. F. Lawless, Statistical Models and Methods for Lifetime Data, 2nd ed., Wiley 2003' },
  lz: { label: 'J. Lieblein & M. Zelen, “Statistical investigation of the fatigue life of deep-groove ball bearings”, J. Res. NBS 57(5), 1956' },
  rausand: { label: 'M. Rausand & A. Høyland, System Reliability Theory, 2nd ed., Wiley 2004' },
};

const TOOLS = [
  { name: 'reliability (Python)', href: 'https://github.com/MatthewReid854/reliability', what: 'Fitters for many distributions with censored data, probability plots, ALT fitters, test planners. Its plotting positions use the same Johnson + Bernard ranks as this lab; check your fits against it.' },
  { name: 'lifelines (Python)', href: 'https://github.com/CamDavidsonPilon/lifelines', what: 'Survival analysis: Kaplan–Meier, Nelson–Aalen, parametric (including Weibull) and regression models — the step up for field-return data with covariates.' },
  { name: 'SurPyval (Python)', href: 'https://github.com/derrynknife/SurPyval', what: 'Survival analysis with arbitrary censoring and truncation, several fitting methods (MLE, MPP, MOM, MSE).' },
];

export function LearnTab() {
  return (
    <div className="stack">
      <div className="grid grid-2">
        <Explainer title="The vocabulary" refs={[REFS.oconnor, REFS.nist]}>
          <Formula>{`R(t) = P(T > t)          reliability (survival)
F(t) = 1 − R(t)          unreliability (cumulative failures)
f(t) = dF/dt             probability density
h(t) = f(t) / R(t)       hazard: failure rate of the survivors
MTTF = ∫₀^∞ R(t) dt      mean time to failure
FIT  = failures per 10⁹ device-hours;  MTBF = 10⁹ / FIT`}</Formula>
          Reliability is always <em>at a time, under a use profile, with a definition of failure</em>. "99 %" alone means nothing; "99 % survive two years of 8 h/day wear with the battery above 80 %" is a requirement.
        </Explainer>
        <MtbfDemo />
        <Explainer title="Weibull: one distribution, three behaviours" refs={[REFS.abernethy]}>
          <Formula>{`F(t) = 1 − exp[−(t/η)^β]
ln(−ln(1 − F)) = β ln t − β ln η     → a straight line on Weibull paper
B_p life: t_p = η (−ln(1 − p))^(1/β)`}</Formula>
          β &lt; 1 infant mortality, β = 1 random (exponential), β &gt; 1 wear-out; η is the age at which 63.2 % have failed whatever β is. Abernethy's handbook is the practitioner's bible for small-sample Weibull work.
        </Explainer>
        <Explainer title="Censoring: units that haven't failed yet" refs={[REFS.meeker, REFS.lawless]}>
          A test that stops before every unit fails produces <em>suspensions</em> (right-censored times). Throwing them away biases life low; treating them as failures biases it lower still. Rank methods give them a share of later ranks (Johnson); maximum likelihood gives each suspension a factor R(t) in the likelihood:
          <Formula>{'L(β, η) = Π_fail f(tᵢ) · Π_susp R(tⱼ)'}</Formula>
        </Explainer>
        <Explainer title="Confidence: what the bounds mean" refs={[REFS.meeker]}>
          A 90 % two-sided bound on B10 means that the procedure, repeated on new samples, would bracket the true B10 nine times in ten. With five failures the bounds span a factor of several — the honest message of a small test. Fisher-matrix bounds assume the log-life estimate is normal; likelihood-ratio bounds follow the actual shape of the likelihood and are better with few failures.
        </Explainer>
        <Explainer title="Acceleration: compress years into weeks" refs={[REFS.nelson]}>
          Raise the stress that drives <em>one</em> failure mechanism (temperature, humidity, temperature swing, vibration, voltage) and the same failures arrive sooner by the acceleration factor. The physics model must be right for that mechanism, and the test must not create failures that never happen in use.
          <Formula>{'t_field = AF × t_test'}</Formula>
        </Explainer>
        <Explainer title="Demonstration: how many units and how long" refs={[REFS.lipson, REFS.nist]}>
          <Formula>{`success run:   n = ln(1 − C) / ln R          (R = 0.9, C = 0.9 → 22)
Lipson:        n = ln(1 − C) / (k^β ln R)    (k = 2, β = 2 → 6)
with failures: n k^β ≥ χ²(C; 2r+2) / (2(−ln R))`}</Formula>
          Demonstrating high reliability by test alone is expensive: R = 0.99 at 95 % needs 299 units for one life. That is why reliability is designed in (margins, derating, physics of failure) and tests confirm it.
        </Explainer>
        <Explainer title="System reliability" refs={[REFS.rausand, REFS.mil217]}>
          A product fails when its structure function says so: series chains multiply reliabilities, redundancy (parallel, k-of-n) adds them back. Importance measures tell you where an improvement pays: in a series system it is almost always the weakest item — for Iris, the battery.
        </Explainer>
        <Explainer title="From analysis to a test plan" refs={[REFS.oconnor]}>
          <ol style={{ margin: '4px 0 0', paddingLeft: 18 }}>
            <li><b>Model</b> — block diagram with failure rates and wear-out lives (System tab).</li>
            <li><b>Find the drivers</b> — importance; fix the top item by design.</li>
            <li><b>Plan tests</b> — accelerate each wear-out mechanism (ALT tab), size the samples (Sample size tab).</li>
            <li><b>Run DVT</b> — legs, schedule, matrix (Test plan tab).</li>
            <li><b>Analyse</b> — fit the results (Weibull tab), feed β and η back into the block diagram, repeat.</li>
          </ol>
        </Explainer>
      </div>
      <Card title="Tools to graduate to" subtitle="Open-source libraries studied while building this lab. None does the whole flow in a browser tied to a product model; this lab adds that, plus animated teaching views — use these to cross-check and for analyses beyond it.">
        <div className="rl-tools">
          {TOOLS.map((t) => (
            <a key={t.name} className="rl-tool" href={t.href} target="_blank" rel="noreferrer">
              <b>{t.name} ↗</b>
              <span className="small muted">{t.what}</span>
            </a>
          ))}
        </div>
      </Card>
      <Card title="References">
        <ul className="small" style={{ margin: 0, paddingLeft: 18 }}>
          {Object.values(REFS).map((r) => (
            <li key={r.label}>{'href' in r && r.href ? <a href={r.href} target="_blank" rel="noreferrer">{r.label}</a> : r.label}</li>
          ))}
          <li>IEC 60068-2 series (environmental testing), IEC 60529 (IP code), IEC 61000-4-2 (electrostatic discharge), IEC 62133-2 and UN 38.3 (lithium cells), ISO 12870 (spectacle frames) — cited by designation; check the current editions.</li>
          <li>JEDEC JEP122, Failure Mechanisms and Models for Semiconductor Devices — check the current revision.</li>
        </ul>
      </Card>
    </div>
  );
}

/** 100 units with constant failure rate: at t = MTBF only 37 % remain. */
function MtbfDemo() {
  const { data } = useCompute<Obs[]>('reliability.weibull_generate', { beta: 1, eta: 1000, n: 100, seed: 5 });
  const [t, setT] = useState(prefersReducedMotion() ? 1000 : 0);
  const [playing, setPlaying] = useState(!prefersReducedMotion());
  useRaf(playing, (dt) => setT((v) => (v + dt * 250 > 3000 ? 0 : v + dt * 250)));
  const alive = useMemo(() => (data ? data.filter((o) => o.t > t).length : 0), [data, t]);
  // the engine returns lifetimes sorted; scatter them over the grid so failures look random
  const shuffled = useMemo(() => {
    if (!data) return [];
    const rnd = mulberry32(3);
    const a = [...data];
    for (let i = a.length - 1; i > 0; i--) {
      const j = Math.floor(rnd() * (i + 1));
      [a[i], a[j]] = [a[j], a[i]];
    }
    return a;
  }, [data]);
  return (
    <Explainer title="Animated: what an MTBF of 1 000 h does not mean" refs={[REFS.oconnor]}>
      <div className="row" style={{ alignItems: 'center', gap: 12, flexWrap: 'wrap' }}>
        <svg viewBox="0 0 200 100" className="rl-mtbf" role="img" aria-label={`${alive} of 100 units working at ${Math.round(t)} hours`}>
          {shuffled.map((o, i) => (
            <circle key={i} cx={(i % 20) * 10 + 5} cy={Math.floor(i / 20) * 10 + 5 + 25} r={o.t > t ? 3.6 : 2} fill={o.t > t ? 'var(--good)' : 'var(--bad)'} opacity={o.t > t ? 0.85 : 0.5} />
          ))}
          <text x={0} y={12} className="rl-axis" style={{ fill: 'var(--text)' }}>t = {Math.round(t)} h · {alive} of 100 working</text>
        </svg>
        <button className="btn small" onClick={() => setPlaying((p) => !p)} aria-pressed={playing}>{playing ? '❚❚' : '▶'}</button>
      </div>
      With a constant failure rate, R(MTBF) = e⁻¹ = 36.8 %: by the time a fleet reaches its MTBF, almost two-thirds have failed. MTBF describes a rate, not a lifetime — and it says nothing about wear-out.
    </Explainer>
  );
}
