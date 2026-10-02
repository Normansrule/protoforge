// Tab 5 — How it works: the human-factors ideas behind every tab, the
// pipeline from body statistics to requirement checks, a live Iris scorecard
// against recommended targets, two hands-on demos (the lever rule and Monte
// Carlo convergence), references and tools.
import { useState } from 'react';
import { Card, Chip, Explainer, Formula, Slider } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useElementWidth } from '../../components/useElementWidth';
import { useMetricMap } from '../../state/project';
import { f1, PlayButton, REFS, Seg, useClock, useErgo, usePlaying } from './shared';
import type { BalanceOut, ClampOut, DisplayOut, FitOut } from './types';

type TabId = 'fit' | 'balance' | 'clamp' | 'display';

const STEPS = [
  { t: 'Body statistics', s: 'ANSUR II means, SDs, correlations' },
  { t: 'Population', s: 'seeded Monte Carlo, women + men' },
  { t: 'Frame', s: 'CAD geometry, masses, optics' },
  { t: 'Physics', s: 'lever rule, cantilevers, photometry' },
  { t: 'Rules', s: 'fit, comfort band, legibility' },
  { t: 'Metrics', s: 'ergonomics.* → requirements' },
];

function Pipeline() {
  const [playing, setPlaying] = usePlaying();
  const t = useClock(playing);
  const active = playing ? Math.floor(t / 1.2) % STEPS.length : -1;
  return (
    <Card title="From body measurements to a requirement check" actions={<PlayButton playing={playing} onChange={setPlaying} />}>
      <div className="hf-pipe" role="list">
        {STEPS.map((s, i) => (
          <div key={s.t} role="listitem" className={`hf-pipe-box ${i === active ? 'on' : ''}`}>
            <b>{i + 1}. {s.t}</b>
            <span className="small muted">{s.s}</span>
          </div>
        ))}
      </div>
    </Card>
  );
}

// ---------------------------------------------------------------- scorecard

interface Live { clamp: ClampOut | null; display: DisplayOut | null; balance: BalanceOut | null; tempSplay: number }

/** Targets this lab recommends for Iris (not yet Iris requirements); fixes are recomputed live. */
const TARGETS: { key: string; cmp: '>=' | '<='; target: number; tab: TabId; why: string; fix: (l: Live) => string }[] = [
  {
    key: 'ergonomics.fit_coverage_pct', cmp: '>=', target: 30, tab: 'fit', why: 'one size fits a useful share of adults on every rule at once',
    fix: (l) => (l.clamp ? `temple splay ≈ ${f1(l.clamp.best_splay_deg)}° centres the clamp band (now ${f1(l.tempSplay)}°); S/M/L sizes do better still` : '…'),
  },
  {
    key: 'ergonomics.nose_pad_pressure_kpa', cmp: '<=', target: 4.3, tab: 'balance', why: 'sustained skin pressure below capillary pressure (≈ 32 mmHg)',
    fix: (l) => (l.balance ? `pads of ≥ ${f1(l.balance.pad_area_for_limit_mm2, 0)} mm² each (now ${f1(l.balance.pad_area_mm2, 0)} mm²)` : '…'),
  },
  {
    key: 'ergonomics.clamp_force_n', cmp: '<=', target: 1.4, tab: 'clamp', why: 'temple pressure under the same limit over a 40 mm × temple-height contact',
    fix: (l) => {
      if (!l.clamp) return '…';
      const need = (l.clamp.head_p95_mm - l.clamp.rows[6].head_breadth_mm) / Math.max(1e-9, l.clamp.band.max_n - l.clamp.band.min_n);
      return `one size over P5–P95 needs ≥ ${f1(need, 1)} mm/N of compliance (now ${f1(l.clamp.model.c_total, 1)}): spring hinges, or sizes`;
    },
  },
  {
    key: 'ergonomics.eyebox_coverage_pct', cmp: '>=', target: 85, tab: 'display', why: 'most wearers see the whole image without adjusting the display',
    fix: (l) => (l.display ? `display axis ${f1(l.display.eyebox.best_offset_mm)} mm towards the nose → ${f1(l.display.eyebox.best_coverage_pct, 0)} %` : '…'),
  },
  {
    key: 'ergonomics.outdoor_contrast', cmp: '>=', target: 1.5, tab: 'display', why: 'captions stay readable on a sunlit street',
    fix: (l) => {
      if (!l.display) return '…';
      const ph = l.display.photometry, bg = l.display.bright.background_nits;
      const tint = ph.image_nits / (0.5 * bg) / Math.max(1e-9, 1 - ph.combiner_reflectance);
      const nits = (0.5 * ph.see_through * bg) / Math.max(1e-9, ph.lens_transmission * ph.combiner_reflectance);
      return `a tint transmitting ≤ ${f1(100 * Math.min(1, tint), 0)} %, or ≈ ${Math.round(nits / 100) * 100} nits of display`;
    },
  },
  { key: 'ergonomics.caption_arcmin', cmp: '>=', target: 16, tab: 'display', why: 'comfortable character height for reading', fix: (l) => (l.display ? `≥ ${l.display.min_px_16} px cap height` : '…') },
];

function Scorecard({ onGo }: { onGo: (t: TabId) => void }) {
  const metrics = useMetricMap();
  const { project, cfg } = useErgo();
  const clamp = useCompute<ClampOut>('ergonomics.clamp', { project }, !!cfg);
  const display = useCompute<DisplayOut>('ergonomics.display', { project }, !!cfg);
  const balance = useCompute<BalanceOut>('ergonomics.balance', { project }, !!cfg);
  const live: Live = { clamp: clamp.data, display: display.data, balance: balance.data, tempSplay: cfg?.temple_splay_deg ?? 0 };
  const rows = TARGETS.map((t) => ({ ...t, m: metrics.get(t.key) }));
  const failing = rows.filter((r) => r.m && !(r.cmp === '>=' ? r.m.value >= r.target : r.m.value <= r.target)).length;
  return (
    <Card
      title="Iris scorecard: what the simulations caught"
      subtitle={`Live ergonomics.* metrics against targets this lab recommends (not yet Iris requirements) — ${failing} of ${rows.length} miss with the current design. Each fix is recomputed from the tab it links to.`}
    >
      <div className="table-wrap">
        <table className="table hf-score">
          <thead>
            <tr><th>Metric</th><th>Iris</th><th>Target</th><th /><th>Why</th><th>Fix the lab finds</th><th /></tr>
          </thead>
          <tbody>
            {rows.map((r) => {
              const ok = r.m ? (r.cmp === '>=' ? r.m.value >= r.target : r.m.value <= r.target) : null;
              return (
                <tr key={r.key}>
                  <td><div>{r.m?.label ?? r.key}</div><div className="mono small muted">{r.key}</div></td>
                  <td className="tnum" style={{ whiteSpace: 'nowrap' }}>{r.m ? `${f1(r.m.value, r.m.value < 10 ? 2 : 1)} ${r.m.unit}` : '…'}</td>
                  <td className="tnum" style={{ whiteSpace: 'nowrap' }}>{r.cmp === '>=' ? '≥' : '≤'} {r.target} {r.m?.unit ?? ''}</td>
                  <td>{ok == null ? null : ok ? <Chip tone="pass">✓ meets</Chip> : <Chip tone="fail">✕ misses</Chip>}</td>
                  <td className="small hf-wide"><span className="hf-only-phone muted">Why: </span>{r.why}</td>
                  <td className="small hf-wide"><span className="hf-only-phone muted">Fix: </span>{ok ? '—' : r.fix(live)}</td>
                  <td><button className="btn small" onClick={() => onGo(r.tab)}>Open</button></td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </Card>
  );
}

// ---------------------------------------------------------------- lever-rule demo

function LeverDemo() {
  const { project, cfg } = useErgo();
  const bal = useCompute<BalanceOut>('ergonomics.balance', { project }, !!cfg);
  const b = bal.data;
  const [playing, setPlaying] = usePlaying(false);
  const t = useClock(playing);
  const [manual, setManual] = useState<number | null>(null);
  const [ref, width] = useElementWidth<HTMLDivElement>();
  if (!b) return <Card title="Lever rule: who carries the weight?"><div className="muted small">Loading…</div></Card>;
  const yn = b.pads[0][1];
  const ye = b.ears[0][1];
  const yIris = b.com[1];
  // CoM position as a fraction of the way from the pads (0) to the ears (1).
  const fIris = (yn - yIris) / (yn - ye);
  const frac = playing ? 0.5 - 0.45 * Math.cos((t * 2 * Math.PI) / 6) : (manual ?? fIris);
  const yc = yn - frac * (yn - ye);
  const nose = (b.mass_g * (yc - ye)) / (yn - ye);
  const ears = b.mass_g - nose;
  const W = Math.max(300, width);
  const H = 210;
  const x0 = 50, x1 = W - 50;
  const X = (f: number) => x0 + f * (x1 - x0);
  const beamY = 92;
  const k = 1.4; // px per gf
  const span = Math.abs(yn - ye);
  return (
    <Card
      title="Lever rule: who carries the weight?"
      subtitle={`Iris front to back: nose pads → ear roots are ${f1(span, 0)} mm apart. Slide the centre of mass (CoM) and watch the reactions trade off.`}
      actions={<PlayButton playing={playing} onChange={setPlaying} />}
    >
      <div ref={ref} style={{ minWidth: 0 }}>
        <svg width={W} height={H} className="hf-svg" role="img" aria-label="Beam on two supports with the load split">
          <line x1={X(0)} x2={X(1)} y1={beamY} y2={beamY} className="hf-frame" />
          <path d={`M${X(0)},${beamY + 4} l-9,16 h18 Z`} className="hf-frame-fill" />
          <path d={`M${X(1)},${beamY + 4} l-9,16 h18 Z`} className="hf-frame-fill" />
          <text x={X(0)} y={beamY + 36} textAnchor="middle" className="hf-lbl">nose pads</text>
          <text x={X(1)} y={beamY + 36} textAnchor="middle" className="hf-lbl">ear roots</text>
          {/* weight */}
          <line x1={X(frac)} x2={X(frac)} y1={beamY - 4 - b.mass_g * k * 0.9} y2={beamY - 10} stroke="var(--series-2)" strokeWidth={2.6} />
          <path d={`M${X(frac)},${beamY - 3} l-6,-10 h12 Z`} fill="var(--series-2)" />
          <circle cx={X(frac)} cy={beamY} r={5} className="hf-com" />
          <text x={X(frac)} y={beamY - 10 - b.mass_g * k * 0.9} textAnchor="middle" className="hf-lbl">W {f1(b.mass_g)} gf</text>
          {/* reactions */}
          {[
            { f: 0, v: nose, c: 'var(--series-1)', a: 'start' as const },
            { f: 1, v: ears, c: 'var(--series-3)', a: 'end' as const },
          ].map((r) => (
            <g key={r.f}>
              <rect x={X(r.f) + (r.f === 0 ? 14 : -14 - 70)} y={beamY + 44} width={70} height={10} rx={5} fill="var(--surface-3)" />
              <rect x={X(r.f) + (r.f === 0 ? 14 : -14 - 70)} y={beamY + 44} width={(70 * Math.max(0, r.v)) / b.mass_g} height={10} rx={5} fill={r.c} />
              <text x={X(r.f) + (r.f === 0 ? 14 : -14)} y={beamY + 72} textAnchor={r.a} className="hf-lbl">
                {f1(r.v)} gf ({f1((100 * r.v) / b.mass_g, 0)} %)
              </text>
            </g>
          ))}
          <line x1={X(fIris)} x2={X(fIris)} y1={beamY - 8} y2={beamY + 8} stroke="var(--muted)" strokeDasharray="3 3" />
          <text x={X(fIris)} y={beamY + 22} textAnchor="middle" className="hf-tick">Iris CoM</text>
          <text x={W / 2} y={H - 10} textAnchor="middle" className="hf-lbl muted">CoM {f1(yn - yc, 0)} mm behind the pads</text>
        </svg>
      </div>
      {!playing && <Slider label="Centre of mass behind the nose pads" value={frac * span} min={0} max={span} step={1} unit="mm" onChange={(v) => setManual(v / span)} />}
      <Formula>{`N_nose = W (y_c − y_ear)/(y_pad − y_ear) = ${f1(b.mass_g)} × ${f1(yc - ye, 0)}/${f1(yn - ye, 0)} = ${f1(nose)} gf`}</Formula>
      <p className="small muted" style={{ margin: 0 }}>
        Only moving mass towards the ears (or adding a counterweight behind them) unloads the nose — the Weight &amp; balance tab does this with the battery.
      </p>
    </Card>
  );
}

// ---------------------------------------------------------------- Monte Carlo convergence

type Rule = 'pd_ok' | 'ear_ok' | 'width_ok' | 'fits';
const RULES: { value: Rule; label: string }[] = [
  { value: 'pd_ok', label: 'PD rule' },
  { value: 'ear_ok', label: 'Temple length' },
  { value: 'width_ok', label: 'Clamp band' },
  { value: 'fits', label: 'All rules' },
];

function ConvergenceDemo() {
  const { project, cfg } = useErgo();
  const N = 2000;
  const fit = useCompute<FitOut>('ergonomics.fit', { project, scatter: N }, !!cfg);
  const [rule, setRule] = useState<Rule>('pd_ok');
  const [playing, setPlaying] = usePlaying(false);
  const [nonce, setNonce] = useState(0);
  const t = useClock(playing, nonce);
  const people = fit.data?.people ?? [];
  const n = people.length;
  const shown = playing ? Math.min(n, Math.max(10, Math.floor(10 * Math.pow(1.6, t * 4)))) : n;
  const xs: number[] = [], est: number[] = [], lo: number[] = [], hi: number[] = [];
  let hits = 0;
  for (let i = 0; i < shown; i++) {
    hits += people[i][rule] ? 1 : 0;
    const m = i + 1;
    if (m < 10 && m !== shown) continue;
    const p = hits / m;
    const se = Math.sqrt(Math.max(p * (1 - p), 0.25 / m) / m);
    xs.push(m);
    est.push(100 * p);
    lo.push(100 * Math.max(0, p - 1.96 * se));
    hi.push(100 * Math.min(1, p + 1.96 * se));
  }
  const final = est.length ? est[est.length - 1] : 0;
  return (
    <Card
      title="Monte Carlo: how many simulated people are enough?"
      subtitle={`The running share of the first n people who pass a rule, with its 95 % confidence band p̂ ± 1.96 √(p̂(1 − p̂)/n). Same seed (${cfg?.seed ?? '—'}) → same answer every time.`}
      actions={<PlayButton playing={playing} onChange={(p) => { if (p) setNonce((v) => v + 1); setPlaying(p); }} />}
    >
      <Seg value={rule} onChange={setRule} options={RULES} />
      {n > 0 && (
        <Plot
          height={230}
          xLog
          xDomain={[10, N]}
          yDomain={[0, 100]}
          xLabel="people simulated, n"
          yLabel="share passing (%)"
          series={[
            { name: 'estimate', x: xs, y: est, width: 2.2 },
            { name: '95 % band (upper)', x: xs, y: hi, dash: '4 3', color: 'var(--muted)' },
            { name: '95 % band (lower)', x: xs, y: lo, dash: '4 3', color: 'var(--muted)' },
          ]}
          hLines={[{ value: final, label: `${f1(final)} % at n = ${shown}` }]}
          legend={false}
        />
      )}
      <p className="small muted" style={{ margin: 0 }}>
        The band shrinks as 1/√n: 100 people pin a share to about ±10 points, 4,000 (the metric&apos;s default) to about ±1.5. The metric uses a fixed seed so a design change, not
        sampling noise, is what moves it.
      </p>
    </Card>
  );
}

export function LearnTab({ onGo }: { onGo: (t: TabId) => void }) {
  return (
    <div className="stack">
      <Pipeline />
      <Scorecard onGo={onGo} />
      <div className="grid grid-2">
        <LeverDemo />
        <ConvergenceDemo />
      </div>
      <div className="grid grid-2">
        <Explainer title="Design for a range, not for the average" refs={[REFS.bodyspace, REFS.sanders, { label: 'G. S. Daniels, "The ‘Average Man’?", Technical Note WCRD 53-7, Wright Air Development Center (1952)' }]}>
          Daniels measured 4,063 airmen on ten dimensions and found that not one of them was within the middle 30 % on all ten: there is no "average person". Human-factors
          practice therefore designs to a range — classically the 5th-percentile woman to the 95th-percentile man — and, for products that come in one size, checks the
          <i> joint </i>coverage of every dimension that matters at once. That is what the Monte Carlo in the first tab does.
          <Formula>{'P5 = μ − 1.645σ      P50 = μ      P95 = μ + 1.645σ      (normal approximation)'}</Formula>
        </Explainer>
        <Explainer title="Where the numbers come from" refs={[REFS.ansur, REFS.ansurData, REFS.dodgson, REFS.iso7250]}>
          Head breadth and interpupillary distance (IPD) statistics were computed from the public ANSUR II files (4,082 men and 1,986 women, measured to ISO 7250-style landmark
          definitions in 2010–2012). ANSUR II is a military sample — younger and fitter than the general public — so treat the tails with care. Dodgson (2004) collects IPD data
          from many sources: adult mean ≈ 63 mm, nearly everyone within 50–75 mm. Hinge-to-ear distance and nose bridge width are not in ANSUR II; the lab labels its values as
          engineering assumptions and lets you replace them.
        </Explainer>
        <Explainer title="Comfort of head-worn products" refs={[REFS.kress2020, REFS.wickens, REFS.landis]}>
          Glasses-style displays are judged against ordinary spectacles: tens of grams, mostly on the nose. Three levers decide comfort — total mass, where the centre of mass sits
          between nose and ears (a battery behind the ear is a counterweight), and how the load is spread (pad area, clamp force). Sustained skin pressure above the ≈ 32 mmHg
          capillary pressure leaves marks and becomes painful over hours.
        </Explainer>
        <Explainer title="Seeing the display" refs={[REFS.kress2013, REFS.kress2020, REFS.hfes100, REFS.sanders]}>
          Legibility needs three things at once: big enough (angular size well above acuity), bright enough against the world (ambient contrast ratio), and visible at all (the pupil
          inside the eyebox). Optical see-through displays struggle most outdoors, where the world is thousands of cd/m² — which is why products add tints, raise luminance, or
          dim the world electronically.
          <Formula>{'ACR = (L_img + T L_bg)/(T L_bg)      θ_text = h_px · θ_px      |IPD/2 − x_d| ≤ e/2'}</Formula>
        </Explainer>
        <Explainer title="Mechanics of a temple" refs={[REFS.roark]}>
          A temple is a cantilever built in at the hinge. Deflection at the contact for a stepped section is ∫(L − s)²/EI ds; the front adds a rotation at each hinge from the
          moment F·L. Stiff electronics pods make the temples short springs, so most of the give is in the front — a stiffer front means a tighter clamp for the same head.
        </Explainer>
        <Explainer title="Limitations (read before trusting a number)">
          Normal approximations miss skew and the extreme tails; ANSUR II is a military sample; two dimensions are assumptions; clamp forces use head breadth (an upper bound) and
          linear beams without lens stiffening or spring hinges; the support model is quasi-static on equally stiff tissue; legibility ignores contrast-dependent acuity and the
          display&apos;s colour; the eyebox is horizontal only. Each of these is a place to plug in better data — and a wear test on real heads always has the last word.
        </Explainer>
      </div>
      <Card title="Tools and data to graduate to">
        <ul className="small" style={{ margin: 0, paddingLeft: 18, lineHeight: 1.7 }}>
          <li><b>ANSUR II public data</b> — the raw measurements behind the head breadth and IPD statistics (mirrored in the <span className="mono">data/</span> folder of AllenDowney/ProbablyOverthinkingIt). This lab&apos;s closest open counterpart is a notebook analysis of that data; the lab adds the fit rules, the frame physics and the requirement link.</li>
          <li><b>Digital human models</b> — commercial tools such as Siemens Jack and Human Solutions RAMSIS place manikins in CAD; for head-worn products, 3-D head-scan sets and headforms (e.g. those standardised for respirator fit) replace the parametric head here.</li>
          <li><b>Wear testing</b> — a pressure-mapping film or a small load cell under a nose pad, and a short structured comfort questionnaire over a few hours of wear, validate every number on this page.</li>
        </ul>
      </Card>
    </div>
  );
}
