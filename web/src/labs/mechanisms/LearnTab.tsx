// How it works: mobility (Gruebler–Kutzbach), why gears use involutes
// (animated string unwinding), spring curvature factors, cam laws side by
// side, the tools to graduate to and the references.
import { useState } from 'react';
import { Card, Explainer, Formula, NumberField, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { PlayBar, fmt, useClock } from './util';

export function LearnTab() {
  return (
    <div className="stack">
      <div className="grid grid-2">
        <InvoluteCard />
        <MobilityCard />
      </div>
      <div className="grid grid-2">
        <CamLawsCard />
        <SpringFactorCard />
      </div>
      <Explainer
        title="From a sketch to a working mechanism, step by step"
        refs={[
          { label: 'R. L. Norton, "Design of Machinery", 6th ed., 2019' },
          { label: 'J. J. Uicker, G. R. Pennock & J. E. Shigley, "Theory of Machines and Mechanisms", 5th ed., 2017' },
        ]}
      >
        <ol className="mx-steps">
          <li><b>Type synthesis</b> — linkage, cam, gears or a spring? Count degrees of freedom (Gruebler) so the mechanism moves exactly as many ways as you drive it.</li>
          <li><b>Dimensional synthesis</b> — two- or three-position synthesis (Linkages tab), cam program (Cams tab), ratios (Gears tab).</li>
          <li><b>Analysis</b> — positions, velocities, accelerations; transmission and pressure angles; contact ratio; undercut.</li>
          <li><b>Forces and strength</b> — Lewis bending, spring stresses with curvature factors, fatigue on a Goodman diagram for the duty cycle you expect.</li>
          <li><b>Make it</b> — export DXF/SVG (laser, waterjet, CAD import), CSV of motion data, and prototype on the Bambu printer: printed POM-like or nylon parts for gears, PETG for brackets.</li>
          <li><b>Test</b> — cycle a prototype (a servo and a counter is enough for a hinge) and compare the failure count with the predicted life; fatigue scatters by a factor of 3–10.</li>
        </ol>
      </Explainer>
      <GraduateCard />
      <ReferencesCard />
    </div>
  );
}

// ------------------------------------------------------------------ involute

function InvoluteCard() {
  const clock = useClock(6, { loop: true });
  const R = 30;
  const th = (clock.t / 6) * 1.05 * Math.PI;
  const trail: string[] = [];
  for (let k = 0; k <= 80; k++) {
    const t = (th * k) / 80;
    const x = R * Math.cos(t) + R * t * Math.sin(t);
    const y = R * Math.sin(t) - R * t * Math.cos(t);
    trail.push(`${x.toFixed(2)},${(-y).toFixed(2)}`);
  }
  const T = [R * Math.cos(th), R * Math.sin(th)];
  const P = [T[0] + R * th * Math.sin(th), T[1] - R * th * Math.cos(th)];
  const arc = `M${R},0 A${R},${R} 0 ${th > Math.PI ? 1 : 0} 0 ${T[0].toFixed(2)},${(-T[1]).toFixed(2)}`;
  return (
    <Card title="Why gear teeth are involutes" subtitle="Unwind a taut string from the base circle: its end traces an involute, and the string is always normal to the curve">
      <svg className="mx-diagram square" viewBox="-120 -120 240 240" role="img" aria-label="String unwinding from a base circle tracing an involute">
        <circle cx={0} cy={0} r={R} className="mx-base" strokeWidth={1} />
        <path d={arc} stroke="var(--series-3)" strokeWidth={2.5} fill="none" />
        <polyline points={trail.join(' ')} fill="none" stroke="var(--series-1)" strokeWidth={2} />
        <line x1={T[0]} y1={-T[1]} x2={P[0]} y2={-P[1]} stroke="var(--series-2)" strokeWidth={1.6} />
        <circle cx={T[0]} cy={-T[1]} r={2.5} fill="var(--series-3)" />
        <circle cx={P[0]} cy={-P[1]} r={3} fill="var(--series-2)" />
        <text x={-114} y={112} fontSize={10}>string length = arc length = R θ = {fmt(R * th, 1)}</text>
      </svg>
      <PlayBar clock={clock} label={() => `θ ${fmt((th * 180) / Math.PI, 0)}°`} />
      <p className="small">
        Two involutes in contact touch on the common tangent to both base circles — the <b>line of action</b>. The contact force always acts along it, so the speed ratio is
        exactly r_b2/r_b1 at every instant (conjugate action), and it stays exact if the centre distance drifts: only the pressure angle and backlash change. That
        tolerance is why involutes won over cycloidal teeth.
      </p>
      <Formula>{`x = r_b (cos θ + θ sin θ),  y = r_b (sin θ − θ cos θ)
inv α = tan α − α   (the polar angle of the involute at pressure angle α)`}</Formula>
    </Card>
  );
}

// ------------------------------------------------------------------ mobility

function MobilityCard() {
  const [L, setL] = useState(4);
  const [J1, setJ1] = useState(4);
  const [J2, setJ2] = useState(0);
  const M = 3 * (L - 1) - 2 * J1 - J2;
  const examples: [string, number, number, number][] = [
    ['Four-bar', 4, 4, 0],
    ['Slider-crank', 4, 4, 0],
    ['Five-bar', 5, 5, 0],
    ['Stephenson / Watt six-bar', 6, 7, 0],
    ['Cam + flat-faced follower', 3, 2, 1],
    ['Cam + roller follower (roller spin is idle)', 4, 3, 1],
    ['Gear pair on a frame', 3, 2, 1],
    ['Truss (triangle)', 3, 3, 0],
  ];
  return (
    <Card title="Degrees of freedom (Gruebler–Kutzbach)" subtitle="Planar: M = 3(L − 1) − 2J₁ − J₂">
      <div className="grid grid-3">
        <NumberField label="Links L (incl. ground)" value={L} step={1} onChange={(v) => setL(Math.max(1, Math.round(v)))} />
        <NumberField label="Full joints J₁ (pin, slider)" value={J1} step={1} onChange={(v) => setJ1(Math.max(0, Math.round(v)))} />
        <NumberField label="Half joints J₂ (cam, gear)" value={J2} step={1} onChange={(v) => setJ2(Math.max(0, Math.round(v)))} />
      </div>
      <Stat label="Mobility M" value={M} tone={M === 1 ? 'pass' : M < 1 ? 'fail' : 'warn'} note={M === 1 ? '✓ one input drives it' : M === 0 ? '✕ a structure (rigid)' : M < 0 ? '✕ over-constrained (preloaded)' : `! needs ${M} inputs`} />
      <div className="table-wrap" style={{ marginTop: 10 }}>
        <table className="table">
          <thead><tr><th>Example</th><th>L</th><th>J₁</th><th>J₂</th><th>M</th><th /></tr></thead>
          <tbody>
            {examples.map(([n, l, j1, j2]) => (
              <tr key={n}>
                <td>{n}</td><td>{l}</td><td>{j1}</td><td>{j2}</td><td className="tnum">{3 * (l - 1) - 2 * j1 - j2}</td>
                <td><button className="btn small" onClick={() => { setL(l); setJ1(j1); setJ2(j2); }}>load</button></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <p className="small muted">The formula ignores special geometry: parallel links (a pantograph) can move even when M = 0 — Norton calls these paradoxes.</p>
    </Card>
  );
}

// ------------------------------------------------------------------ cams & springs

function CamLawsCard() {
  const xs = Array.from({ length: 101 }, (_, i) => i / 100);
  const laws = {
    cycloidal: (x: number) => [x - Math.sin(2 * Math.PI * x) / (2 * Math.PI), 2 * Math.PI * Math.sin(2 * Math.PI * x)],
    '3-4-5': (x: number) => [10 * x ** 3 - 15 * x ** 4 + 6 * x ** 5, 60 * x - 180 * x ** 2 + 120 * x ** 3],
    harmonic: (x: number) => [0.5 * (1 - Math.cos(Math.PI * x)), 0.5 * Math.PI ** 2 * Math.cos(Math.PI * x)],
  } as const;
  return (
    <Card title="Three rise laws, one lift" subtitle="Same displacement end points, very different acceleration (h = 1, β = 1)">
      <div className="grid grid-2">
        <Plot series={Object.entries(laws).map(([n, f]) => ({ name: n, x: xs, y: xs.map((x) => f(x)[0]) }))} xLabel="θ/β" yLabel="s/h" legend height={190} />
        <Plot series={Object.entries(laws).map(([n, f]) => ({ name: n, x: xs, y: xs.map((x) => f(x)[1]) }))} xLabel="θ/β" yLabel="a β²/h" legend height={190} hLines={[{ value: 0, color: 'var(--axis)' }]} />
      </div>
      <p className="small">
        Harmonic has the lowest peak (π²/2 ≈ 4.93) but starts and ends with non-zero acceleration — next to a dwell that is a jump, an infinite jerk spike and noise. Cycloidal
        (2π ≈ 6.28) and 3-4-5 (5.77) start and end at zero: smooth.
      </p>
    </Card>
  );
}

function SpringFactorCard() {
  const cs = Array.from({ length: 81 }, (_, i) => 3 + (i * 13) / 80);
  return (
    <Card title="Why small coils are stressed more" subtitle="The inner fibre of a tightly wound coil sees direct shear plus curvature: Wahl and Bergsträsser factors">
      <Plot
        series={[
          { name: 'Wahl K_W', x: cs, y: cs.map((c) => (4 * c - 1) / (4 * c - 4) + 0.615 / c) },
          { name: 'Bergsträsser K_B', x: cs, y: cs.map((c) => (4 * c + 2) / (4 * c - 3)), dash: '5 3' },
          { name: 'direct shear only 1 + 0.5/C', x: cs, y: cs.map((c) => 1 + 0.5 / c), dash: '2 3' },
        ]}
        xLabel="spring index C = D/d"
        yLabel="stress factor"
        legend
        xBands={[{ from: 4, to: 12, label: 'preferred C' }]}
        height={210}
      />
      <Formula>{`τ = K 8 F D / (π d³)
K_W = (4C − 1)/(4C − 4) + 0.615/C     K_B = (4C + 2)/(4C − 3)`}</Formula>
      <p className="small muted">The Iris hinge spring has C ≈ 4.4 — near the tight end, because it must fit inside a 4 mm temple.</p>
    </Card>
  );
}

// ------------------------------------------------------------------ graduate & refs

function GraduateCard() {
  const tools = [
    ['Pyslvs-UI', 'Open-source planar linkage simulation and dimensional synthesis (Python + Qt). GitHub: KmolYuan/Pyslvs-UI — the closest open tool to the Linkages tab; it adds multi-loop linkages and evolutionary path synthesis.'],
    ['Linkage (David Rector)', 'Free Windows linkage designer and simulator with drag-and-drop links, sliders and gears; good for quick sketches with many links.'],
    ['MotionGen', 'Planar mechanism synthesis app from Stony Brook University (A. Purwar): fit a four- or six-bar to sketched poses.'],
    ['BOSL2 gears.scad', 'OpenSCAD library (GitHub: BelfrySCAD/BOSL2) with spur, ring, rack, bevel and worm gears, profile shift and auto-undercut — studied for this lab; here the root fillet comes from the rounded ISO 53 rack envelope instead.'],
    ['FreeCAD Gears workbench', 'Parametric involute, cycloid, bevel and worm gears inside FreeCAD (GitHub: looooo/freecad.gears).'],
    ['SolveSpace', 'Constraint-based 2D/3D CAD (GitHub: solvespace/solvespace) that can animate linkages by dragging; exports DXF.'],
    ['Spring makers’ calculators', 'Associated Spring, Lee Spring and others publish online spring calculators and stock catalogs — check stock wire sizes and tolerances against this lab.'],
    ['AGMA / ISO 6336 software', 'For load-rated gears use a full rating method (AGMA 2001 or ISO 6336) — e.g. KISSsoft or the calculators in commercial CAD.'],
  ];
  return (
    <Card title="Graduate to professional tools" subtitle="Where to go when the mechanism matters">
      <div className="mx-grad">
        {tools.map(([n, d]) => (
          <div key={n}><b>{n}</b><span className="muted">{d}</span></div>
        ))}
      </div>
      <div className="small muted" style={{ marginTop: 10 }}>
        English-language learning: MIT OpenCourseWare's 2.007 "Design and Manufacturing I" covers mechanisms, gears and springs in a design-build course (check the OCW site for current
        materials); R. L. Norton's "Design of Machinery" ships with linkage, cam and gear programs and its examples are the ones checked here.
      </div>
    </Card>
  );
}

function ReferencesCard() {
  const refs = [
    'R. L. Norton, "Design of Machinery: An Introduction to the Synthesis and Analysis of Mechanisms and Machines", 6th ed., McGraw-Hill, 2019.',
    'J. J. Uicker, G. R. Pennock & J. E. Shigley, "Theory of Machines and Mechanisms", 5th ed., Oxford University Press, 2017.',
    'R. G. Budynas & J. K. Nisbett, "Shigley’s Mechanical Engineering Design", 10th ed., McGraw-Hill, 2015 — ch. 10 springs, ch. 13–14 gears.',
    'A. M. Wahl, "Mechanical Springs", 2nd ed., McGraw-Hill, 1963.',
    'F. L. Litvin & A. Fuentes, "Gear Geometry and Applied Theory", 2nd ed., Cambridge University Press, 2004.',
    'F. Freudenstein, "Approximate synthesis of four-bar linkages", Transactions of the ASME 77, 1955.',
    'G. N. Sandor & A. G. Erdman, "Advanced Mechanism Design: Analysis and Synthesis", vol. 2, Prentice-Hall, 1984.',
    'R. L. Norton, "Cam Design and Manufacturing Handbook", 2nd ed., Industrial Press, 2009.',
    'ISO 53 — cylindrical gears for general and heavy engineering: standard basic rack tooth profile.',
    'ISO 21771 — gears: cylindrical involute gears and gear pairs, concepts and geometry.',
    'AGMA 2001 and ISO 6336 — calculation of load capacity of spur and helical gears (what the Lewis + Barth check here leaves out).',
    'ISO 12870 — ophthalmic optics, spectacle frames, requirements and test methods.',
  ];
  return (
    <Card title="References">
      <ol className="small" style={{ margin: 0, paddingLeft: 20, display: 'grid', gap: 4 }}>
        {refs.map((r) => <li key={r}>{r}</li>)}
      </ol>
    </Card>
  );
}
