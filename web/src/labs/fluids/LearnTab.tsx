// How it works: heat paths, dimensionless numbers on Iris, the finite element
// and lattice-Boltzmann pipelines, tools to graduate to, references.
import { useEffect, useRef, useState } from 'react';
import { Card, Explainer, Formula } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { ConvOut } from './types';
import { fmt, prefersReducedMotion } from './util';

function HeatPaths() {
  const [play, setPlay] = useState(!prefersReducedMotion());
  const svg = useRef<SVGSVGElement>(null);
  useEffect(() => {
    if (play) svg.current?.unpauseAnimations();
    else svg.current?.pauseAnimations();
  }, [play]);
  // dots travelling along each path
  const paths = [
    { d: 'M150,150 C150,120 150,110 150,96', label: '' },
    { d: 'M150,96 C120,80 90,70 70,58', label: '' },
    { d: 'M150,96 C180,80 210,70 230,58', label: '' },
    { d: 'M70,58 C60,40 50,30 40,14', label: '' },
    { d: 'M230,58 C240,40 250,30 262,14', label: '' },
    { d: 'M70,58 C54,58 30,62 12,66', label: '' },
    { d: 'M230,58 C246,58 270,62 290,66', label: '' },
  ];
  return (
    <div>
      <svg ref={svg} viewBox="0 0 300 194" className="fl-diagram" role="img" aria-label="heat path from the chip through the board, cavity air and shell into the surrounding air">
        <rect x="40" y="52" width="220" height="110" rx="14" className="fl-d-shell" />
        <rect x="52" y="64" width="196" height="86" rx="9" className="fl-d-air" />
        <rect x="70" y="128" width="160" height="8" rx="2" className="fl-d-pcb" />
        <rect x="136" y="118" width="28" height="10" rx="2" className="fl-d-chip" />
        {paths.map((p, i) => (
          <g key={i}>
            <path d={p.d} className="fl-d-path" id={`fl-hp-${i}`} />
            {[0, 0.33, 0.66].map((o) => (
              <circle key={o} r="2.4" className="fl-d-dot">
                <animateMotion dur="2.4s" repeatCount="indefinite" begin={`${-o * 2.4}s`} path={p.d} />
              </circle>
            ))}
          </g>
        ))}
        <text x="150" y="30" textAnchor="middle" className="fl-d-sm">shell k ≈ 0.2 W/m K</text>
        <text x="150" y="40" textAnchor="middle" className="fl-d-sm">heat crosses it, barely spreads</text>
        <text x="150" y="114" textAnchor="middle" className="fl-d-sm">sealed cavity air</text>
        <text x="150" y="176" textAnchor="middle" className="lbl fl-d-sm">chip → board (copper spreads it)</text>
        <text x="150" y="187" textAnchor="middle" className="lbl fl-d-sm">→ cavity air (conduction + radiation) → shell</text>
        <text x="6" y="80" className="fl-d-sm">convection</text>
        <text x="294" y="80" textAnchor="end" className="fl-d-sm">convection</text>
        <text x="8" y="10" className="fl-d-sm">radiation</text>
        <text x="292" y="10" textAnchor="end" className="fl-d-sm">radiation</text>
      </svg>
      <button className="btn small" onClick={() => setPlay((p) => !p)}>{play ? '❚❚ Pause' : '▶ Animate'}</button>
    </div>
  );
}

function sci(v: number) {
  const e = Math.floor(Math.log10(Math.abs(v)));
  const sup = String(e).split('').map((c) => '⁰¹²³⁴⁵⁶⁷⁸⁹'[+c] ?? '⁻').join('');
  return `${(v / 10 ** e).toFixed(1)} × 10${sup}`;
}

function Lattice() {
  const cx = [0, 1, 0, -1, 0, 1, -1, -1, 1];
  const cy = [0, 0, 1, 0, -1, 1, 1, -1, -1];
  const w = ['4/9', '1/9', '1/9', '1/9', '1/9', '1/36', '1/36', '1/36', '1/36'];
  return (
    <svg viewBox="-70 -70 140 140" className="fl-diagram" style={{ maxWidth: 260 }} role="img" aria-label="the nine D2Q9 lattice velocities">
      <defs>
        <marker id="fl-arr" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
          <path d="M0,0 L10,5 L0,10 z" fill="var(--series-1)" />
        </marker>
      </defs>
      {[-40, 0, 40].map((v) => (
        <g key={v}>
          <line x1={-60} y1={v} x2={60} y2={v} stroke="var(--grid)" />
          <line x1={v} y1={-60} x2={v} y2={60} stroke="var(--grid)" />
        </g>
      ))}
      {cx.map((x, i) =>
        i === 0 ? null : <line key={i} x1={0} y1={0} x2={x * 38} y2={-cy[i] * 38} stroke="var(--series-1)" strokeWidth="2" markerEnd="url(#fl-arr)" />,
      )}
      <circle r="5" fill="var(--series-2)" />
      {cx.map((x, i) => (
        <text key={i} x={x * 54 + (i === 0 ? 14 : 0)} y={-cy[i] * 54 + (i === 0 ? -8 : 3)} textAnchor="middle" style={{ fontSize: 7 }}>
          {i}:{w[i]}
        </text>
      ))}
    </svg>
  );
}

export function LearnTab() {
  const project = useProject((s) => s.project);
  const f = project.design.frame;
  const conv = useCompute<ConvOut>('fluids.convection', { project }).data;
  const h = conv?.h_still_total ?? 10;
  const air = conv?.film;
  const bi = (h * f.pod_length_mm * 1e-3) / 0.2;
  const ra = conv?.rows[0]?.group ?? 0;
  const re = air ? (1.4 * f.pod_height_mm * 1e-3) / air.nu : 0;
  return (
    <div className="stack">
      <div className="grid grid-2">
        <Card title="Where the heat goes" subtitle="Every milliwatt the electronics use ends up as heat on the skin side or in the air">
          <HeatPaths />
          <p className="small muted">
            Inside the sealed pod the air is too thin a layer to convect, so heat crosses it by conduction and radiation. Outside, natural convection and radiation share the job almost equally at Iris's
            small temperature rises.
          </p>
        </Card>
        <Card title="The numbers that decide (on Iris)">
          <div className="table-wrap">
          <table className="table">
            <thead><tr><th>Number</th><th>Meaning</th><th>Iris</th></tr></thead>
            <tbody>
              <tr><td>Biot <i>Bi = h L / k</i></td><td>surface vs internal resistance; lumped models need Bi ≪ 0.1</td><td><span className="tnum">{fmt(bi, 1)}</span><div className="small muted">pod length, PETG</div></td></tr>
              <tr><td>Rayleigh <i>Ra = gβΔT L³/(να)</i></td><td>buoyancy vs diffusion; sets natural-convection Nu</td><td><span className="tnum">{ra ? sci(ra) : '—'}</span><div className="small muted">pod side at the skin limit</div></td></tr>
              <tr><td>Reynolds <i>Re = V D / ν</i></td><td>inertia vs viscosity; above ≈ 47 a cylinder sheds vortices</td><td><span className="tnum">{fmt(re, 0)}</span><div className="small muted">pod at 1.4 m/s</div></td></tr>
              <tr><td>Nusselt <i>Nu = h L / k</i></td><td>convective vs conductive transfer in the air</td><td className="tnum">{fmt(conv?.rows[0]?.nu, 2)}</td></tr>
              <tr><td>Prandtl <i>Pr = ν / α</i></td><td>momentum vs heat diffusion in the fluid</td><td className="tnum">{fmt(air?.pr, 3)}</td></tr>
              <tr><td>Strouhal <i>St = f D / U</i></td><td>vortex-shedding frequency, dimensionless</td><td><span className="tnum">≈ 0.2</span><div className="small muted">bluff bodies</div></td></tr>
              <tr><td>Mach <i>Ma = U / c_s</i></td><td>LBM accuracy needs Ma ≲ 0.1–0.2 (c_s = 1/√3)</td><td><span className="tnum">0.17</span><div className="small muted">at U = 0.1</div></td></tr>
            </tbody>
          </table>
          </div>
        </Card>
      </div>

      <div className="grid grid-2">
        <Explainer title="1. Conduction by finite elements" refs={[{ label: 'Patankar (1980)' }, { label: 'Hughes, The Finite Element Method (2000)' }, { label: 'Saad, Iterative Methods for Sparse Linear Systems (2003)' }]}>
          <ol className="small" style={{ paddingLeft: 18 }}>
            <li>Voxelise the CAD parts (ray casting, 27 sub-samples per voxel → material fractions).</li>
            <li>Add the board, display, spreader and USB-C receptacle as boxes; flood-fill the outside air from the grid edge — what stays is the sealed cavity air.</li>
            <li>Mix materials per voxel (parallel along sheets, series across), assemble K in compressed sparse rows.</li>
            <li>Put each component's average power from the Power Lab budget at its PCB Studio position.</li>
            <li>Solve (K + H)θ = Q by conjugate gradients; update h from the surface temperature; repeat three times.</li>
          </ol>
          <Formula>θ<sub>k+1</sub> = θ<sub>k</sub> + α p<sub>k</sub>,  r<sub>k+1</sub> = r<sub>k</sub> − α A p<sub>k</sub>,  p<sub>k+1</sub> = M⁻¹r<sub>k+1</sub> + β p<sub>k</sub></Formula>
          <p className="small">
            The preconditioner M approximates A cheaply. Jacobi uses its diagonal; symmetric successive over-relaxation (SSOR) does a forward and a backward Gauss–Seidel sweep and typically halves the
            iterations here.
          </p>
        </Explainer>
        <Explainer title="2. Convection: correlations, not CFD" refs={[{ label: 'Churchill & Chu (1975)' }, { label: 'Churchill & Bernstein (1977)' }, { label: 'Incropera et al. (2011)' }]}>
          <p className="small">
            Resolving the air boundary layer around the whole head would need a 3-D flow solve. Engineering practice — and this lab — uses experimental correlations for the heat-transfer coefficient h on each
            face, evaluated at the film temperature, and blends natural and forced convection with Nu³ = Nu<sub>N</sub>³ + Nu<sub>F</sub>³. Radiation is linearised:
          </p>
          <Formula>h<sub>r</sub> = ε σ (T<sub>s</sub> + T<sub>∞</sub>)(T<sub>s</sub>² + T<sub>∞</sub>²) ≈ 4 ε σ T³ ≈ {fmt(conv?.h_rad, 1)} W/m² K</Formula>
          <p className="small">The wind tunnel tab shows the flow those correlations summarise.</p>
        </Explainer>
      </div>

      <div className="grid grid-2">
        <Card title="3. The D2Q9 lattice">
          <div className="row" style={{ alignItems: 'flex-start' }}>
            <Lattice />
            <div className="small" style={{ flex: 1, minWidth: 200 }}>
              <p>Nine velocities per cell with weights w<sub>i</sub>. Each step: <b>collide</b> (relax towards equilibrium with time τ), then <b>stream</b> (move each population one cell along its link).</p>
              <Formula>ν = (τ − ½) / 3,  p = ρ / 3</Formula>
              <p>Boundaries are simple — that is LBM's strength: a solid cell just reflects populations (bounce-back), so drawing an obstacle is flipping a bit.</p>
            </div>
          </div>
        </Card>
        <Card title="Graduate to these tools">
          <div className="st-grad">
            <div><b>OpenFOAM</b>Finite-volume computational fluid dynamics (CFD): conjugate heat transfer (chtMultiRegionFoam) solves air and solids together, including the convection this lab takes from correlations.</div>
            <div><b>Elmer FEM</b>Open-source multiphysics finite elements (ElmerCSC/elmerfem): heat equation with radiation view factors and real tetrahedral meshes of the CAD.</div>
            <div><b>Palabos</b>Lattice-Boltzmann library (C++, MPI) with curved-boundary schemes and multi-relaxation-time collisions for higher Re.</div>
            <div><b>FluidX3D</b>GPU lattice-Boltzmann (ProjectPhysX/FluidX3D): billions of cells in 3-D, the same D2Q9 ideas on D3Q19.</div>
          </div>
          <p className="small muted">
            No open tool runs the whole loop in a browser from the product file. What this lab adds: the geometry, heat sources and duty cycles come straight from the Iris project, the 3-D result is
            compared with the lumped model on the same heat, and the metrics verify requirements live.
          </p>
        </Card>
      </div>

      <Card title="References">
        <ul className="small" style={{ paddingLeft: 18, margin: 0 }}>
          <li>F. P. Incropera, D. P. DeWitt, T. L. Bergman, A. S. Lavine, <i>Fundamentals of Heat and Mass Transfer</i>, 7th ed., Wiley, 2011 — air properties (Table A.4), fins (§3.6), lumped capacitance (§5.2), correlations (ch. 7, 9).</li>
          <li>S. W. Churchill, H. H. S. Chu, “Correlating equations for laminar and turbulent free convection from a vertical plate”, Int. J. Heat Mass Transfer 18 (1975) 1323–1329; “… from a horizontal cylinder”, 18 (1975) 1049–1053.</li>
          <li>S. W. Churchill, M. Bernstein, “A correlating equation for forced convection from gases and liquids to a circular cylinder in crossflow”, J. Heat Transfer 99 (1977) 300–306.</li>
          <li>S. V. Patankar, <i>Numerical Heat Transfer and Fluid Flow</i>, Hemisphere, 1980.</li>
          <li>J. P. Holman, <i>Heat Transfer</i>, 10th ed., McGraw-Hill, 2010.</li>
          <li>T. Krüger, H. Kusumaatmaja, A. Kuzmin, O. Shardt, G. Silva, E. M. Viggen, <i>The Lattice Boltzmann Method: Principles and Practice</i>, Springer, 2017.</li>
          <li>S. Succi, <i>The Lattice Boltzmann Equation for Fluid Dynamics and Beyond</i>, Oxford University Press, 2001.</li>
          <li>A. J. C. Ladd, “Numerical simulations of particulate suspensions via a discretized Boltzmann equation”, J. Fluid Mech. 271 (1994).</li>
          <li>Z. Guo, C. Zheng, B. Shi, “Non-equilibrium extrapolation method for velocity and pressure boundary conditions in the lattice Boltzmann method”, Chinese Physics 11 (2002); “Discrete lattice effects on the forcing term in the lattice Boltzmann method”, Phys. Rev. E 65 (2002) 046308.</li>
          <li>M. Schäfer, S. Turek, “Benchmark computations of laminar flow around a cylinder”, Notes on Numerical Fluid Mechanics 52, Vieweg, 1996.</li>
          <li>C. H. K. Williamson, “Vortex dynamics in the cylinder wake”, Annu. Rev. Fluid Mech. 28 (1996) 477–539.</li>
          <li>Y. Saad, <i>Iterative Methods for Sparse Linear Systems</i>, 2nd ed., SIAM, 2003.</li>
        </ul>
      </Card>
    </div>
  );
}
