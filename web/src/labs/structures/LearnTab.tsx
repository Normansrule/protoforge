// How it works: stiffness-matrix assembly (animated), shape functions and the
// incompatible "bubble" modes, shear locking (live solve), von Mises and the
// yield ellipse, SIMP penalisation and root-sum-square (RSS) statistics — plus
// the professional tools to graduate to and the references.
import { useMemo, useRef, useState } from 'react';
import { Card, Explainer, Formula, Select, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import type { LockOut } from './types';
import { fmt, heat, prefersReducedMotion, useRaf, useWorkerCompute } from './util';

export function LearnTab() {
  return (
    <div className="stack">
      <AssemblyCard />
      <div className="grid grid-2">
        <ShapeFunctionCard />
        <LockingCard />
      </div>
      <div className="grid grid-2">
        <VonMisesCard />
        <Card title="SIMP: why grey material is a bad deal" subtitle="Stiffness ∝ x^p but volume ∝ x — with p = 3 half-density material gives only 1/8 of the stiffness">
          <Plot
            series={[1, 2, 3, 4].map((p) => ({ name: `p = ${p}`, x: Array.from({ length: 51 }, (_, i) => i / 50), y: Array.from({ length: 51 }, (_, i) => (i / 50) ** p) }))}
            xLabel="density x"
            yLabel="E(x) / E₀"
            legend
            height={220}
          />
          <Formula>{`E(x) = E_min + x^p (E₀ − E_min)      c(x) = fᵀ u(x)      ∂c/∂x_e = −p x_e^(p−1) (E₀ − E_min) u_eᵀ k₀ u_e`}</Formula>
          <div className="small muted">
            The optimiser moves material to where the strain energy density is highest; the filter over radius r_min stops checkerboards. See the Topology tab for the full 88-line-style loop.
          </div>
        </Card>
      </div>
      <div className="grid grid-2">
        <RssCard />
        <Explainer
          title="From the Iris CAD to a stress map, step by step"
          refs={[
            { label: 'O. C. Zienkiewicz & R. L. Taylor, "The Finite Element Method", 7th ed., Butterworth-Heinemann, 2013' },
            { label: 'T. J. R. Hughes, "The Finite Element Method: Linear Static and Dynamic Finite Element Analysis", Dover, 2000' },
            { label: 'K.-J. Bathe, "Finite Element Procedures", 2nd ed., 2014' },
          ]}
        >
          <ol className="cp-steps">
            <li>Voxelise the real CAD triangle mesh by ray casting (winding numbers), 27 samples per voxel for a partial-volume fraction; the grid is aligned with the temple's faces.</li>
            <li>Every voxel is the same cube, so one 24 × 24 brick stiffness (with Wilson's incompatible modes, statically condensed) is computed once and scaled by E·h·ρ.</li>
            <li>Clamp the voxels at the hinge stop, spread the load case over the loaded voxels' nodes.</li>
            <li>Solve K u = f without ever forming K: matrix-free conjugate gradients, preconditioned by a geometric multigrid V-cycle (Galerkin coarse levels, Chebyshev smoothing).</li>
            <li>Recover the six stresses at every element corner, average them at the nodes, take von Mises; the design stress skips one voxel around sharp inside edges (singular) and the clamps.</li>
            <li>Safety factor = yield ÷ design stress, with the print-layer check: stress across the layers is compared with the weaker interlayer strength.</li>
          </ol>
        </Explainer>
      </div>
      <GraduateCard />
      <ReferencesCard />
    </div>
  );
}

// ------------------------------------------------------------------ assembly

/** Three springs in a row: element matrices scatter into the global K one by one. */
function AssemblyCard() {
  const ks = [3, 2, 1];
  const [step, setStep] = useState(prefersReducedMotion() ? 4 : 0);
  const [playing, setPlaying] = useState(!prefersReducedMotion());
  const acc = useRef(0);
  useRaf(playing, (dt) => {
    acc.current += dt;
    if (acc.current > 1.4) {
      acc.current = 0;
      setStep((s) => (s >= 4 ? 0 : s + 1));
    }
  });
  const K = useMemo(() => {
    const m = Array.from({ length: 4 }, () => [0, 0, 0, 0]);
    for (let e = 0; e < Math.min(step, 3); e++) {
      m[e][e] += ks[e];
      m[e][e + 1] -= ks[e];
      m[e + 1][e] -= ks[e];
      m[e + 1][e + 1] += ks[e];
    }
    return m;
  }, [step]);
  const active = step >= 1 && step <= 3 ? step - 1 : -1;
  const fixed = step >= 4;
  // Solve the fixed-at-node-0 chain with 1 N at node 3: u_i = Σ 1/k.
  const u = [0, 1 / 3, 1 / 3 + 1 / 2, 1 / 3 + 1 / 2 + 1];
  const W = 560;
  const H = 150;
  return (
    <Card
      title="Stiffness-matrix assembly"
      subtitle="Each element contributes its small matrix k_e = k [1 −1; −1 1] to the rows and columns of its two nodes; supports remove rows; the solve gives displacements"
      actions={
        <div className="row" style={{ gap: 6 }}>
          <button className="btn small" onClick={() => setPlaying(!playing)} aria-pressed={playing}>
            {playing ? '❚❚ Pause' : '▶ Play'}
          </button>
          <button className="btn small" onClick={() => { setPlaying(false); setStep((s) => (s >= 4 ? 0 : s + 1)); }}>
            Step ▶
          </button>
        </div>
      }
    >
      <div className="grid grid-2">
        <svg className="st-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Spring chain being assembled">
          {[0, 1, 2].map((e) => {
            const x1 = 60 + e * 150;
            const on = e < step;
            const pts: string[] = [];
            for (let i = 0; i <= 12; i++) pts.push(`${x1 + 12 + (i * 126) / 12},${70 + (i === 0 || i === 12 ? 0 : i % 2 ? -12 : 12)}`);
            return (
              <g key={e} opacity={on ? 1 : 0.35}>
                <polyline points={pts.join(' ')} fill="none" stroke={e === active ? 'var(--series-2)' : 'var(--series-1)'} strokeWidth={e === active ? 3 : 2} />
                <text x={x1 + 75} y={40} textAnchor="middle" className={e === active ? 'lbl' : ''}>k{e + 1} = {ks[e]}</text>
              </g>
            );
          })}
          {[0, 1, 2, 3].map((n) => (
            <g key={n}>
              <circle cx={60 + n * 150 + (fixed ? u[n] * 22 : 0)} cy={70} r={9} fill="var(--surface-1)" stroke="var(--text)" strokeWidth={1.5} />
              <text x={60 + n * 150 + (fixed ? u[n] * 22 : 0)} y={74} textAnchor="middle" className="lbl">{n}</text>
            </g>
          ))}
          <rect x={30} y={45} width={14} height={50} fill="var(--surface-3)" stroke="var(--text-2)" opacity={fixed ? 1 : 0.3} />
          {fixed && (
            <g className="load">
              <line x1={510 + u[3] * 22 + 12} y1={70} x2={545} y2={70} />
            </g>
          )}
          <text x={20} y={H - 12}>
            {step === 0 ? 'empty 4 × 4 global matrix' : step <= 3 ? `adding element ${step}` : 'node 0 clamped, 1 N at node 3 → u = [0, 0.33, 0.83, 1.83]'}
          </text>
        </svg>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 4, alignContent: 'center' }} role="table" aria-label="Global stiffness matrix">
          {K.flatMap((row, i) =>
            row.map((v, j) => {
              const inBlock = active >= 0 && (i === active || i === active + 1) && (j === active || j === active + 1);
              const struck = fixed && (i === 0 || j === 0);
              return (
                <div
                  key={`${i}${j}`}
                  role="cell"
                  className="tnum"
                  style={{
                    textAlign: 'center',
                    padding: '10px 0',
                    borderRadius: 8,
                    border: '1px solid var(--border)',
                    background: inBlock ? 'color-mix(in srgb, var(--series-2) 22%, var(--surface-1))' : 'var(--surface-1)',
                    color: struck ? 'var(--muted)' : v === 0 ? 'var(--muted)' : 'var(--text)',
                    textDecoration: struck ? 'line-through' : undefined,
                    fontWeight: 600,
                  }}
                >
                  {v}
                </div>
              );
            }),
          )}
        </div>
      </div>
      <Formula>{`K = Σ_e  L_eᵀ k_e L_e   (scatter-add),     K u = f,     3D brick: k_e = ∫ Bᵀ D B dV  (24 × 24)`}</Formula>
      <div className="small muted">The voxel solver never stores K: it multiplies element by element (matrix-free), which is exactly this scatter-add done on the fly inside every conjugate-gradient iteration.</div>
    </Card>
  );
}

// ------------------------------------------------------------------ shape functions

function ShapeFunctionCard() {
  const [node, setNode] = useState(2);
  const [mode, setMode] = useState<'node' | 'bubble'>('node');
  const nodes = [
    [-1, -1],
    [1, -1],
    [1, 1],
    [-1, 1],
  ];
  const n = 24;
  const cells = [];
  for (let i = 0; i < n; i++) {
    for (let j = 0; j < n; j++) {
      const xi = -1 + (2 * (i + 0.5)) / n;
      const eta = -1 + (2 * (j + 0.5)) / n;
      const v = mode === 'node' ? 0.25 * (1 + xi * nodes[node][0]) * (1 + eta * nodes[node][1]) : 1 - xi * xi;
      const [r, g, b] = heat(v);
      cells.push(<rect key={`${i}-${j}`} x={20 + i * 8} y={20 + (n - 1 - j) * 8} width={8.4} height={8.4} fill={`rgb(${r * 255},${g * 255},${b * 255})`} />);
    }
  }
  return (
    <Card title="Shape functions" subtitle="Inside an element the displacement is interpolated from the nodes: u(ξ, η) = Σ N_a u_a">
      <div className="row" style={{ alignItems: 'flex-start', gap: 16 }}>
        <svg className="st-diagram" viewBox="0 0 232 232" style={{ maxWidth: 232 }} role="img" aria-label="Shape function value over the element">
          {cells}
          {nodes.map((p, a) => (
            <circle key={a} cx={20 + ((p[0] + 1) / 2) * 192} cy={212 - ((p[1] + 1) / 2) * 192} r={a === node && mode === 'node' ? 8 : 5} fill="var(--surface-1)" stroke="var(--text)" strokeWidth={2} onClick={() => { setNode(a); setMode('node'); }} style={{ cursor: 'pointer' }} />
          ))}
        </svg>
        <div className="stack" style={{ flex: 1, minWidth: 180, gap: 8 }}>
          <Select
            label="Show"
            value={mode === 'bubble' ? 'bubble' : String(node)}
            options={[...nodes.map((_, a) => ({ value: String(a), label: `N_${a + 1} (node ${a + 1})` })), { value: 'bubble', label: 'Incompatible mode 1 − ξ²' }]}
            onChange={(v) => {
              if (v === 'bubble') setMode('bubble');
              else {
                setMode('node');
                setNode(Number(v));
              }
            }}
          />
          <Formula>{`N_a = ¼ (1 + ξ ξ_a)(1 + η η_a)
3D brick: N_a = ⅛ (1 + ξ ξ_a)(1 + η η_a)(1 + ζ ζ_a)
bubble modes (Wilson): 1 − ξ², 1 − η², 1 − ζ²`}</Formula>
          <div className="small muted">
            Each N_a is 1 at its own node and 0 at the others, so the element reproduces nodal values exactly. A bilinear brick cannot bend without parasitic shear; the bubble modes (condensed out) give it the curved displacement that bending needs.
          </div>
        </div>
      </div>
    </Card>
  );
}

// ------------------------------------------------------------------ locking demo

function LockingCard() {
  const [layers, setLayers] = useState(2);
  const [element, setElement] = useState<'h8i' | 'h8'>('h8');
  const { data: r, busy } = useWorkerCompute<LockOut>('structures.locking_demo', { layers, element });
  const other = useWorkerCompute<LockOut>('structures.locking_demo', { layers, element: element === 'h8' ? 'h8i' : 'h8' });
  return (
    <Card title="Shear locking, live" subtitle="A 40 × 4 × 4 mm cantilever, 10 N at the tip, solved by the same voxel engine" actions={busy ? <span className="cp-spin" aria-label="solving" /> : undefined}>
      <Slider label="Voxels through the thickness" value={layers} min={1} max={6} step={1} onChange={(v) => setLayers(Math.round(v))} />
      <Select label="Element" value={element} options={[{ value: 'h8', label: 'Plain trilinear brick (H8)' }, { value: 'h8i', label: 'Brick + incompatible modes (H8i)' }]} onChange={setElement} />
      {r && (
        <>
          <div className="grid grid-3">
            <Stat label="FEA tip deflection" value={fmt(r.fea_tip_mm, 3)} unit="mm" tone={Math.abs(r.error_pct) < 5 ? 'pass' : 'fail'} note={`${r.error_pct > 0 ? '+' : ''}${fmt(r.error_pct, 1)} % vs Timoshenko`} />
            <Stat label="Beam theory" value={fmt(r.timoshenko_tip_mm, 3)} unit="mm" note={`Euler–Bernoulli ${fmt(r.beam_tip_mm, 3)} mm`} />
            <Stat label="Other element" value={other.data ? fmt(other.data.fea_tip_mm, 3) : '…'} unit="mm" note={other.data ? `${other.data.element.toUpperCase()}: ${fmt(other.data.error_pct, 1)} %` : ''} />
          </div>
          <Plot
            series={[
              { name: `${r.element.toUpperCase()} (${r.layers} layers)`, x: r.x, y: r.w.map((v) => -v) },
              ...(other.data ? [{ name: `${other.data.element.toUpperCase()}`, x: other.data.x, y: other.data.w.map((v) => -v), dash: '4 3' }] : []),
              { name: 'Euler–Bernoulli', x: r.x, y: r.x.map((x) => -((10 * x * x * (3 * 40 - x)) / (6 * 2000 * ((4 * 4 ** 3) / 12)))), color: 'var(--muted)', dash: '2 3' },
            ]}
            xLabel="x, mm"
            yLabel="deflection, mm"
            legend
            height={200}
          />
        </>
      )}
    </Card>
  );
}

// ------------------------------------------------------------------ von Mises

function VonMisesCard() {
  const [sx, setSx] = useState(30);
  const [sy, setSy] = useState(-10);
  const [txy, setTxy] = useState(10);
  const [sy0, setSy0] = useState(45);
  const c = (sx + sy) / 2;
  const R = Math.hypot((sx - sy) / 2, txy);
  const s1 = c + R;
  const s2 = c - R;
  const vm = Math.sqrt(sx * sx - sx * sy + sy * sy + 3 * txy * txy);
  const W = 260;
  const S = 110 / Math.max(sy0 * 1.25, Math.abs(s1), Math.abs(s2));
  const ellipse = Array.from({ length: 121 }, (_, i) => {
    const t = (i / 120) * 2 * Math.PI;
    // σ1² − σ1σ2 + σ2² = Sy²  ⇒ rotate by 45°: axes √2 Sy and √(2/3) Sy.
    const a = Math.SQRT2 * sy0;
    const b = Math.sqrt(2 / 3) * sy0;
    const u = a * Math.cos(t);
    const v = b * Math.sin(t);
    const p1 = (u - v) / Math.SQRT2;
    const p2 = (u + v) / Math.SQRT2;
    return `${W / 2 + p1 * S},${W / 2 - p2 * S}`;
  }).join(' ');
  return (
    <Card title="Von Mises stress and the yield ellipse" subtitle="Six stress components → one number to compare with the tensile yield strength">
      <div className="row" style={{ alignItems: 'flex-start', gap: 16 }}>
        <svg className="st-diagram" viewBox={`0 0 ${W} ${W}`} style={{ maxWidth: 260 }} role="img" aria-label="Principal stress point inside the von Mises ellipse">
          <line className="dim" x1={10} x2={W - 10} y1={W / 2} y2={W / 2} />
          <line className="dim" x1={W / 2} x2={W / 2} y1={10} y2={W - 10} />
          <polygon points={ellipse} fill="color-mix(in srgb, var(--good) 12%, transparent)" stroke="var(--series-1)" strokeWidth={2} />
          <circle cx={W / 2 + s1 * S} cy={W / 2 - s2 * S} r={6} fill={vm <= sy0 ? 'var(--series-2)' : 'var(--bad)'} stroke="var(--surface-1)" />
          <text x={W - 14} y={W / 2 - 6} textAnchor="end">σ₁</text>
          <text x={W / 2 + 6} y={18}>σ₂</text>
        </svg>
        <div className="stack" style={{ flex: 1, minWidth: 180, gap: 4 }}>
          <Slider label="σx" value={sx} min={-60} max={60} step={1} unit="MPa" onChange={setSx} />
          <Slider label="σy" value={sy} min={-60} max={60} step={1} unit="MPa" onChange={setSy} />
          <Slider label="τxy" value={txy} min={-40} max={40} step={1} unit="MPa" onChange={setTxy} />
          <Slider label="Yield S_y" value={sy0} min={10} max={80} step={1} unit="MPa" onChange={setSy0} />
          <div className="grid grid-2">
            <Stat label="von Mises" value={fmt(vm, 1)} unit="MPa" tone={vm <= sy0 ? 'pass' : 'fail'} note={vm <= sy0 ? '✓ inside: elastic' : '✕ outside: yields'} />
            <Stat label="Safety factor" value={fmt(sy0 / Math.max(1e-9, vm), 2)} unit="×" note={`σ₁ ${fmt(s1, 1)}, σ₂ ${fmt(s2, 1)} MPa`} />
          </div>
        </div>
      </div>
      <Formula>{`σ_vM = √( ½[(σx−σy)² + (σy−σz)² + (σz−σx)²] + 3(τxy² + τyz² + τzx²) )      plane stress: √(σx² − σx σy + σy² + 3 τxy²)`}</Formula>
    </Card>
  );
}

// ------------------------------------------------------------------ RSS

function RssCard() {
  const [t, setT] = useState(0.1);
  const n = Array.from({ length: 12 }, (_, i) => i + 1);
  return (
    <Card title="Why RSS beats worst case for long chains" subtitle="n parts, each ±t at 3σ: the worst case grows like n, the statistical spread like √n">
      <Slider label="Tolerance per part ±t" value={t} min={0.02} max={0.3} step={0.01} unit="mm" onChange={setT} />
      <Plot
        series={[
          { name: 'worst case  n·t', x: n, y: n.map((k) => k * t) },
          { name: 'RSS  √n·t (99.73 %)', x: n, y: n.map((k) => Math.sqrt(k) * t) },
        ]}
        xLabel="number of parts in the chain"
        yLabel="± gap variation, mm"
        legend
        height={210}
      />
      <Formula>{`worst case: ±Σ t_i      RSS: ±√(Σ t_i²)      (independent, centred, normal; 3σ ↔ 99.73 %)`}</Formula>
    </Card>
  );
}

// ------------------------------------------------------------------ graduate + references

function GraduateCard() {
  const tools = [
    ['CalculiX', 'Open-source Abaqus-style FEA solver (ccx) with a pre/post-processor (cgx); nonlinear, contact, modal.'],
    ['FreeCAD FEM', 'The FEM workbench of FreeCAD: mesh with Gmsh or Netgen, solve with CalculiX or Elmer, all from your CAD model.'],
    ['Elmer', 'Open-source multiphysics finite element suite (CSC, Finland) — structures, heat, electromagnetics coupled.'],
    ['Code_Aster', 'EDF’s open-source structural solver, industrial verification pedigree; usually driven from Salome-Meca.'],
    ['ToPy / TopOpt apps', 'Python topology optimisation (ToPy) and the DTU TopOpt group’s interactive apps and codes — the step up from the 88-line method.'],
    ['Onshape & SimScale', 'Cloud CAD (Onshape) with browser-based simulation (SimScale, and Onshape Simulation): no install, real meshes and contact.'],
  ];
  return (
    <Card title="Graduate to professional tools" subtitle="When a part matters, re-run it with real tetrahedral meshes, fillets, contact and nonlinear materials">
      <div className="st-grad">
        {tools.map(([n, d]) => (
          <div key={n}>
            <b>{n}</b>
            <span className="muted">{d}</span>
          </div>
        ))}
      </div>
      <div className="small muted" style={{ marginTop: 10 }}>
        Export the Iris part as STL from the CAD Studio, the stress results as CSV from the Strength tab and the topology outline as SVG, and compare. Recommended English-language video series: the free finite element and structures lectures on MIT OpenCourseWare (search "Finite Element Analysis of Solids and Fluids", K.-J. Bathe) — check availability on the MIT OpenCourseWare site.
      </div>
    </Card>
  );
}

function ReferencesCard() {
  const refs = [
    'O. C. Zienkiewicz & R. L. Taylor, "The Finite Element Method" (7th ed., 2013) — isoparametric elements, patch test, incompatible modes.',
    'T. J. R. Hughes, "The Finite Element Method: Linear Static and Dynamic Finite Element Analysis" (Dover, 2000).',
    'K.-J. Bathe, "Finite Element Procedures" (2nd ed., 2014) — element matrices, eigenproblems.',
    'E. L. Wilson, R. L. Taylor, W. P. Doherty & J. Ghaboussi, "Incompatible displacement models" (1973).',
    'J. R. Shewchuk, "An Introduction to the Conjugate Gradient Method Without the Agonizing Pain" (Carnegie Mellon University, 1994).',
    'W. L. Briggs, V. E. Henson & S. F. McCormick, "A Multigrid Tutorial" (2nd ed., SIAM, 2000).',
    'E. Andreassen, A. Clausen, M. Schevenels, B. S. Lazarov & O. Sigmund, "Efficient topology optimization in MATLAB using 88 lines of code", Struct. Multidisc. Optim. 43 (2011).',
    'M. P. Bendsøe & O. Sigmund, "Topology Optimization: Theory, Methods and Applications" (Springer, 2003).',
    'W. C. Young & R. G. Budynas, "Roark’s Formulas for Stress and Strain" (7th ed., 2002).',
    'R. G. Budynas & J. K. Nisbett, "Shigley’s Mechanical Engineering Design" (10th ed.).',
    'Bayer MaterialScience (Covestro), "Snap-Fit Joints for Plastics — A Design Guide".',
    'W. D. Pilkey & D. F. Pilkey, "Peterson’s Stress Concentration Factors" (3rd ed., 2008).',
    'ISO 286-1 / ISO 286-2 — ISO code system for tolerances on linear sizes (limits and fits).',
    'ISO 12870 — Ophthalmic optics, spectacle frames, requirements and test methods (the load cases here are inspired by this kind of test, not an implementation of it).',
    'J. R. Davis (ed.), ASM handbooks — typical material property values; always use your supplier’s data sheet.',
  ];
  return (
    <Card title="References">
      <ol className="small" style={{ margin: 0, paddingLeft: 20, display: 'grid', gap: 4 }}>
        {refs.map((r) => (
          <li key={r}>{r}</li>
        ))}
      </ol>
    </Card>
  );
}
