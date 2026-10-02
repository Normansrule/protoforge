// Tab 5 — How it works: the human-factors ideas behind every tab, the
// pipeline from body statistics to requirement checks, references and tools.
import { Card, Explainer, Formula } from '../../components/ui';
import { REFS, PlayButton, useClock, usePlaying } from './shared';

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

export function LearnTab() {
  return (
    <div className="stack">
      <Pipeline />
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
          <li><b>ANSUR II public data</b> — the raw measurements behind the head breadth and IPD statistics (mirrored in AllenDowney/ProbablyOverthinkingIt, <span className="mono">data/</span> on the <span className="mono">book</span> branch). This lab&apos;s closest open counterpart is a notebook analysis of that data; the lab adds the fit rules, the frame physics and the requirement link.</li>
          <li><b>Digital human models</b> — commercial tools such as Siemens Jack and Human Solutions RAMSIS place manikins in CAD; for head-worn products, 3-D head-scan sets and headforms (e.g. those standardised for respirator fit) replace the parametric head here.</li>
          <li><b>Wear testing</b> — a pressure-mapping film or a small load cell under a nose pad, and a short structured comfort questionnaire over a few hours of wear, validate every number on this page.</li>
        </ul>
      </Card>
    </div>
  );
}
