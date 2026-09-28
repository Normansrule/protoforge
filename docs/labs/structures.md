# Structures Lab (`#/structures`, engine module `structures`)

Will it break, bend, wear out or not fit? The Structures Lab answers that for
the **Iris smart glasses** — and for anything else you build — in the
browser: a voxel finite element analysis (FEA) solver runs the real CAD parts
under named load cases, SIMP (Solid Isotropic Material with Penalisation)
topology optimisation grows lighter parts, closed-form calculators size snap
fits, screws, beams, fits and hinges, fatigue and vibration analyses estimate
hinge life and temple modes, and worst-case, root-sum-square (RSS) and Monte
Carlo stack-ups check the printed circuit board (PCB) and the display optics
fit every unit. Heavy solves run in the lab's own Web Worker, so the page
never blocks.

## What it does

| Tab | What you get |
|---|---|
| Strength (3D FEA) | The voxelised Iris part (right or left temple, or the front) under a load case, solved in WebAssembly: a stress heat map on the CAD surface or the voxels, colour legend, animated exaggerated deformation (play/pause, scale slider), load arrows, clamped-support markers, a pulsing marker at the critical point and a plain-language "why here" hint. Load-case, force, material (with "use this material for Iris"), voxel size, preconditioner (multigrid or Jacobi) and element (plain or incompatible-mode brick) pickers. Stat tiles (design stress and corner peak, safety factor, deflection, stiffness) and requirement chips. Conjugate-gradient convergence plot and a Jacobi-vs-multigrid race, the full Iris verification table (metric pass or fine pass), a mesh-convergence study, and the drop estimate with an animated mass–spring impact. Downloads: nodal results CSV, summary JSON, load cases CSV, full verification JSON. |
| Topology optimisation | SIMP in the style of the 88-line code on MBB beam, cantilever, L-bracket, the **Iris hinge endpiece (bracket)** and the Iris temple arm; the density field animates iteration by iteration (play, step, scrub), compliance history, final stress view. Downloads: outline SVG (mm), OpenSCAD extrusion, density CSV. |
| Calculators | Snap fit (Bayer/Covestro), bolted joint (preload, torque, joint diagram, **thread stripping**), screw boss and heat-set inserts, Euler–Bernoulli beam, column and plate buckling, ISO 286 limits and fits, Lamé press fit, **thermal expansion mismatch** (clearance vs temperature, bonded bi-material strip), **stress concentration** (hole, ellipse, notch; K_t and K_f) and living hinge — each with an SVG diagram (most animated), results, formula and references. |
| Fatigue & vibration | Hinge fatigue from the FEA stress per newton: S–N (Basquin) curve with the usage blocks, Goodman diagram, editable Miner usage spectrum, notch factor, life in years and in open/close cycles. Temple natural frequencies with animated mode shapes (sideways and up–down) and EI / mass distributions along the temple; a beam-mode calculator checked against the analytical cantilever. |
| Tolerances | The PCB-in-pod stack, the **display focus chain** (optics alignment) and a custom chain: to-scale stack diagram, worst case, RSS ±3σ, Monte Carlo yield, C_pk, an animated Monte Carlo histogram, a Pareto of contributors, focus error vs spacing. Download: stack-up CSV. |
| How it works | Animated stiffness-matrix assembly, shape functions and incompatible modes, a live shear-locking demo, the von Mises yield ellipse, SIMP penalisation, RSS vs worst case, the pipeline from CAD to stress map, tools to graduate to and references. |

## Engine API

| Method | Input | Output |
|---|---|---|
| `structures.fea` | `{project, part: temple_r \| temple_l \| front, case, material?, voxel_mm, max_elements, force_n?, precond: multigrid \| jacobi, element: h8i \| h8, tol, max_iter, surface: voxels \| cad \| both \| none}` | mesh size, solver stats and residual history, design and peak von Mises, safety factors (von Mises and with the print-layer check), displacement, stiffness, critical point and hint, load/support markers, per-vertex fields (displacement, von Mises, utilisation) on the voxel surface and on the CAD mesh |
| `structures.cases` | – | load cases (id, label, parts, default force, description) |
| `structures.materials` | – | the material table (typical values; printed flags and Z knock-down) |
| `structures.iris` | `{project, fidelity: metric \| fine}` | every Iris case, min safety factor, drop, pod and focus stacks, temple modes, hinge fatigue, notes |
| `structures.mesh_study` | `{project, part, case, material?, sizes[]}` | per voxel size: elements, iterations, design and peak stress, SF, deflection, volume |
| `structures.drop` | `{project, height_m, part, material?, voxel_mm, mass_g?}` | 1-DOF impact: speed, energy, force, deflection, contact time, stress, SF |
| `structures.locking_demo` | `{layers, element}` | 40 × 4 × 4 mm cantilever: FEA vs Euler–Bernoulli and Timoshenko, centre line |
| `structures.topo` / `topo_presets` | `{preset, nelx, nely, volfrac, penal, rmin, filter, max_iter, every, …}` | frames (base64 densities), compliance/volume/change history, final density and stress, outline loops, SVG, OpenSCAD |
| `structures.plane_stress` | 2D plane-stress model | displacements and von Mises |
| `structures.beam`, `buckling`, `snap_fit`, `boss`, `bolt`, `press_fit`, `living_hinge`, `thermal`, `stress_conc` | calculator inputs (all fields have defaults) | results, curves for plotting, notes |
| `structures.fit`, `structures.fits` | `{nominal_mm, hole, shaft}` | ISO 286 deviations, limits, clearance range, fit type; preferred fits |
| `structures.stackup` | `{dims[], lower?, upper?, samples, seed, bins}` | worst case, RSS, Monte Carlo (histogram, first samples), yield, C_pk, contributions |
| `structures.pod_stack`, `structures.focus_stack` | `{project, …options}` | the Iris stacks (dims + `stackup` result; focus adds diopters) |
| `structures.fatigue`, `structures.iris_fatigue` | `{material, years, blocks[], strength_factor}` | per-block damage, life, stress margin, S–N curve, Goodman line |
| `structures.modal`, `structures.beam_modes` | `{project}` / `{length_mm, section, material, tip_mass_g}` | natural frequencies and mode shapes |

## The maths

**Voxel model.** The CAD triangle mesh is voxelised by ray casting with a
winding number, 27 sub-samples per voxel giving a material fraction ρ; voxels
with ρ ≥ 0.26 become 8-node bricks whose stiffness is scaled by ρ. The grid
is **aligned with the temple's faces** so a 4 mm temple on a 2 mm grid is two
full voxels thick (partial voxels are right for membranes but over-stiff in
bending). Every voxel is the same cube, so one 24 × 24 stiffness `K_e = E h K̂(ν)`
is computed once, with trilinear shape functions `N_a = ⅛(1+ξξa)(1+ηηa)(1+ζζa)`
and Wilson's incompatible modes (1 − ξ², 1 − η², 1 − ζ²) condensed out so the
brick bends without shear locking.

**Solver.** `K u = f` is solved matrix-free by the preconditioned conjugate
gradient (PCG) method: `K·p` loops over the voxels (576 multiply-adds each).
The default preconditioner is one geometric multigrid V-cycle (2 × 2 × 2
coarsening, Galerkin coarse operators `Pᵀ K P`, Chebyshev smoothing, dense
Cholesky on the coarsest level); Jacobi is available to show why it matters
(≈ 2 600 iterations vs ≈ 50 on a temple).

**Stresses and the safety factor.** Corner stresses (including the condensed
modes) are averaged at the nodes; von Mises is
`√(½[(σx−σy)² + (σy−σz)² + (σz−σx)²] + 3(τxy² + τyz² + τzx²))`. Sharp
re-entrant edges are **singular** — their nodal stress grows without bound as
the voxels shrink — so the **design stress** is the largest nodal von Mises
outside a one-voxel zone around every re-entrant edge (nodes shared by 6 or 7
voxels) and the clamps. At the metric resolution (two voxels through the
member) that zone is half a member thick: the design stress is the nominal
stress beside a sharp corner, the right basis for static strength of ductile
thermoplastics (local yielding redistributes the peak — Shigley). The
excluded corner peak is always shown beside it. Printed materials are also
checked across the layers: tension normal to the layers against
`z_strength_factor × yield`.

**Iris load cases** (inspired by the kind of frame tests in ISO 12870, not an
implementation of any clause): temple spread (5 N outward at the tip, hinge
end clamped), bridge bend (10 N on the bridge, endpieces clamped), pod
squeeze (10 N finger pinch, fine pass only — it needs voxels no larger than
the 1.2 mm wall), and a lateral unit load on the pod for the drop model
`F = v √(k m)`, `v = √(2 g h)` (an upper bound: rigid floor, all energy into
the temple).

**Topology optimisation.** `E(x) = E_min + x^p (E₀ − E_min)`, minimise
compliance `c = fᵀu` at a volume fraction, sensitivity or density filter of
radius r_min, optimality-criteria update with bisection on the Lagrange
multiplier; plane-stress bilinear quads (the 88-line element matrix) solved
by banded Cholesky. The MBB benchmark (60 × 20, 50 %, p = 3, r_min = 1.5)
converges in 94 iterations, as in the 88-line paper's setting.

**Calculators.** Snap fit: `ε = 1.5 y t / L²`, `P = b t² E ε / (6L)`,
`W = P (μ + tan α)/(1 − μ tan α)`. Bolts: `A_t = π/4 (d − 0.9382p)²`,
`T = K F_i d`, Wileman member stiffness, joint constant, and thread stripping
on the basic profile (`A_int ≈ π d L_e · 7/8`, `A_ext ≈ π D₁ L_e · 3/4`,
`τ = 0.577 S_y`). Press fits: Lamé. Buckling: Euler/Johnson and plate
buckling `σ_cr = k π² E/(12(1−ν²)) (t/b)²`. Thermal: `Δc = Δα L ΔT` and
Timoshenko's bi-material strip. K_t: Heywood's `2 + (1 − d/W)³` (net
section), Inglis `1 + 2a/b`, notch `1 + 2√(a/ρ)`; `K_f = 1 + q (K_t − 1)`.
Fatigue: Basquin through (½ cycle, UTS) and the material's quoted fatigue
point, Goodman, Miner. Modes: Hermite beam elements with consistent mass,
`K φ = ω² M φ`, sections taken from voxel slices of the temple.

**Stack-ups.** Worst case `Σ` limits; RSS `σ_G = √Σ(t_i/k_i)²` (uniform
dimensions use `t/√3`); Monte Carlo with a seeded generator. The focus chain
converts spacing error to diopters with the thin-lens equation
`V = 1000/f − 1000/d` (dV/dd ≈ 2.6 D/mm at Iris's 19.7 mm spacing).

## Iris tie-in and metrics

The metric pass uses two voxels through the temple (h = temple width / 2 =
2 mm) and front (h = 2.5 mm), grid-aligned, multigrid PCG to 1e-3; the finite
element part is memoised on the frame, optics and PCB parameters (≈ 0.24 s
native cold, a few milliseconds warm when only components change; the whole
metric list is memoised too). Values for the Iris defaults (PETG):

| Key | Meaning | Iris |
|---|---|---|
| `structures.min_safety_factor` | min SF over temple spread (both) and bridge bend | **2.04×** (left temple, 22 MPa at the battery-pod-to-arm junction; right 2.58×, bridge 8.2×). Hand check: right arm root σ = F L / (h w²/6) = 5 × 70.6 / 21.3 ≈ 16.5 MPa vs 17.5 MPa FEA. Recommend **≥ 2** — a pass with little margin |
| `structures.pod_fit_worst_mm` | worst-case clearance of the PCB in the pod incl. −10 °C shrink | **0.97 mm** (recommend **≥ 0**) |
| `structures.temple_deflection_mm` | larger tip deflection at 5 N spread | 25.9 mm (left temple) |
| `structures.temple_stiffness_n_mm` | right temple lateral stiffness | 0.30 N/mm |
| `structures.drop_safety_factor` | 1.5 m drop on the pod, energy upper bound | 0.30× (294 N peak) — see limitations |
| `structures.pod_fit_yield_pct` | Monte Carlo assembly yield | 100 % |
| `structures.first_mode_hz` | first bending mode of the right temple | ≈ 101 Hz |
| `structures.hinge_fatigue_cycles` | open/close cycles to predicted failure | ≈ 1.3 × 10⁷ (no notch factor) |
| `structures.focus_error_worst_d` | worst-case display focus error | 0.60 D vs a ±0.25 D allowance — a focus adjustment (shim or threaded lens cell) or a molded housing is needed |

The sharp step where each slim temple arm leaves its pod is a stress
concentration: its nodal peak (≈ 30 MPa at 2 mm voxels, ≈ 60 MPa at 1 mm) is
reported with every result. A fillet there is the single most useful change;
in the Fatigue tab, a notch factor of 1.5–2.5 shows how much it matters for
hinge life.

## Limitations

* Linear, small-displacement, isotropic elasticity (printed parts are
  orthotropic; only strength gets the layer check). The 25 mm temple
  deflection at 5 N is beyond "small" — stresses are fine, deflections are
  indicative.
* Voxels stair-step curved surfaces; stresses at sharp corners are singular;
  the design stress depends mildly on the voxel size (see the mesh study).
* The drop estimate ignores floor/contact compliance, rotation and rate
  effects — it ranks designs; a physical drop test decides.
* Fatigue data for polymers are rough (no endurance limit, big scatter); the
  S–N curve is anchored at a typical fatigue point, not a tested curve.
* Material values are typical, not a data sheet; tolerances in the stacks are
  typical process capabilities — check your vendor.
* The hinge is modelled by clamping; no contact, pin or screw preload in the
  3D model.

## Graduate to

CalculiX (with FreeCAD FEM or PrePoMax), FreeCAD FEM (Gmsh/Netgen meshes,
CalculiX/Elmer solvers), Elmer (multiphysics), Code_Aster (with Salome-Meca),
ToPy and the DTU TopOpt group's codes and apps for topology optimisation,
Onshape with SimScale (or Onshape Simulation) in the browser. Export the part
as STL from the CAD Studio and the results as CSV here to compare.

## References

* O. C. Zienkiewicz & R. L. Taylor, *The Finite Element Method*, 7th ed., 2013.
* T. J. R. Hughes, *The Finite Element Method: Linear Static and Dynamic Finite Element Analysis*, Dover, 2000.
* K.-J. Bathe, *Finite Element Procedures*, 2nd ed., 2014.
* E. L. Wilson, R. L. Taylor, W. P. Doherty & J. Ghaboussi, "Incompatible displacement models", 1973.
* J. R. Shewchuk, "An Introduction to the Conjugate Gradient Method Without the Agonizing Pain", CMU, 1994.
* W. L. Briggs, V. E. Henson & S. F. McCormick, *A Multigrid Tutorial*, 2nd ed., SIAM, 2000.
* E. Andreassen, A. Clausen, M. Schevenels, B. S. Lazarov & O. Sigmund, "Efficient topology optimization in MATLAB using 88 lines of code", *Struct. Multidisc. Optim.* 43, 2011.
* O. Sigmund, "A 99 line topology optimization code written in Matlab", *Struct. Multidisc. Optim.* 21, 2001.
* M. P. Bendsøe & O. Sigmund, *Topology Optimization: Theory, Methods and Applications*, Springer, 2003.
* W. C. Young & R. G. Budynas, *Roark's Formulas for Stress and Strain*, 7th ed., 2002.
* R. G. Budynas & J. K. Nisbett, *Shigley's Mechanical Engineering Design*, 10th ed.
* Bayer MaterialScience (Covestro), *Snap-Fit Joints for Plastics — A Design Guide*.
* W. D. Pilkey & D. F. Pilkey, *Peterson's Stress Concentration Factors*, 3rd ed., 2008.
* S. Timoshenko, "Analysis of bi-metal thermostats", *J. Opt. Soc. Am.* 11, 1925.
* O. H. Basquin, 1910; M. A. Miner, "Cumulative damage in fatigue", *J. Appl. Mech.* 12, 1945.
* ISO 286-1 / ISO 286-2 (limits and fits); ISO 12870 (spectacle frames — the load cases are inspired by, not an implementation of, its tests).
* J. R. Davis (ed.), ASM handbooks — typical material values.
