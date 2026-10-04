# Thermal & Fluids Lab (`#/fluids`, engine module `fluids`)

Where does the heat really go? The Power Lab answers "how warm does the pod
get?" with a two-node lumped resistor–capacitor network. This lab is its
spatially resolved companion: a **3-D finite element analysis (FEA) of heat
conduction** on the real Iris CAD — right pod, lid, display-optics housing
and the right half of the front — with every component's power placed where
the printed circuit board (PCB) Studio puts it, cooled by natural convection
and radiation from textbook correlations. It compares the result honestly
with the lumped model on exactly the same heat, trades graphite and copper
heat spreaders, sizes convection coefficients with the Churchill–Chu and
Churchill–Bernstein correlations, and runs a **lattice-Boltzmann method
(LBM)** wind tunnel that grows a Kármán vortex street in the browser.
Everything runs in the Rust engine compiled to WebAssembly; the heavy
conduction solves use the lab's own Web Worker.

## Tabs

| Tab | What you get |
|---|---|
| 3-D conduction (FEA) | The voxelised right side of the frame, temperature-coloured on the CAD surfaces (subdivided so a hotspot can show), the board, display and spreader inside (x-ray the lid or the frame), heat-source markers sized by power and a pulsing ring at the hottest surface. Controls: Power Lab usage scenario, where the display's heat goes (optics housing as the bill of materials (BOM) says, or the pod board as the lumped model assumes), spreader type / thickness / location, frame material (thermal table), emissivity, air speed (walking), voxel size, preconditioner; "Use these choices for Iris" writes them to `design.extra.fluids`. Stat tiles (peak, mean, hotspot above mean, lumped value), per-part table and energy balance, a **hotspot vs lumped** comparison with a plain-language explanation and the spreading-length formula, an animated **transient** (implicit Euler) for three use scenarios with a time scrubber next to the lumped network on the same schedule, **cut slices** (side, cross-section, plan; temperature or material, hover readout), the convection/radiation coefficient table with Rayleigh numbers and validity flags, the heat-source table, the conjugate-gradient convergence plot and a one-click mesh-convergence study. Downloads: ParaView VTK (temperature + material cell data), heat sources CSV, summary JSON, transient CSV. |
| Heat spreading trade | Nine full 3-D solves: none; 25 µm graphite on the pod's skin-side wall or under the lid; 35 µm copper; and 25/70 µm graphite, 70 µm copper, 0.5 and 1 mm aluminium **behind the display** — peak, gain, hotspot above mean, board temperature, added mass, sheet conductance k·t, with "Use for Iris". Peak vs sheet thickness for graphite and copper at the hotspot. An interactive 1-D fin model shows why conductance k·t, not conductivity, decides. Download: trade CSV. |
| Convection | Calculator for the Iris pod and temple (or a textbook 0.3 m plate): film properties, Ra/Re, Nu and h for Churchill–Chu (vertical plate, full range and laminar), Churchill–Chu horizontal cylinder (temple), horizontal plates up/down, Churchill–Bernstein cross-flow, laminar flat plate, radiation; validity chips; still-air and walking totals; h against ΔT for every correlation; an animated boundary-layer / cross-flow diagram. Download: curves CSV. |
| Wind tunnel (LBM) | A D2Q9 BGK lattice-Boltzmann session kept in the engine and streamed frame by frame: vorticity, speed, pressure or streamwise velocity with particle tracers; presets (Kármán street at Re 150, Schäfer–Turek 2D-1 and 2D-2 benchmarks, Iris temple and pod cross-sections in a side wind, NACA 2412 at 6°, flat plate, empty tunnel); Reynolds-number, angle, NACA-section, resolution, speed and contrast controls; **tap or drag to draw / erase obstacles**; live drag and lift coefficients by momentum exchange, Strouhal number from the probe spectrum, stability and Reynolds-cap warnings, the real Re of the Iris shape at walking speed. Downloads: force CSV, frame PNG. |
| How it works | Animated heat-path diagram, the dimensionless numbers evaluated on Iris (Biot, Rayleigh, Reynolds, Nusselt, Prandtl, Strouhal, Mach), the FEA pipeline and preconditioned conjugate gradients, correlations vs computational fluid dynamics (CFD), the D2Q9 lattice, tools to graduate to, references. |

## Engine API

| Method | Input | Output |
|---|---|---|
| `fluids.pod_steady` | `{project?, config: PodConfig, surfaces?, voxels?, slices?}` | peak / mean / hotspot / board / display temperatures and positions, per-part stats, convection groups (area, ΔT, Ra, h_conv, h_rad, heat out), sources, solver stats and residual history, materials, cavity checks, lumped comparison (R_eff, h_mean, h needed for the lumped R, spreading lengths, explanation), CAD + inner surfaces (base64 f32 positions, u32 indices, f32 temperatures), optional voxel surface, slices (u8 temperature + material grids) |
| `fluids.pod_transient` | `{project?, config, preset: r008 \| commute \| captions, segments?, dt_s?, frames?}` | time series (peak, mean, board, display, power, duty, lumped shell and board), u8 surface frames, surfaces, steady peak, time to the skin limit |
| `fluids.spreader_trade` | `{project?, config, thicknesses_um?}` | trade rows, thickness sweeps at the hotspot, baseline, hotspot location, notes |
| `fluids.mesh_study` | `{project?, config, sizes?}` | per voxel size: elements, unknowns, iterations, peak, mean, hotspot, board |
| `fluids.convection` | `{project?, t_surface_c?, t_ambient_c?, emissivity?, plate_height_mm?, plate_ap_mm?, diameter_mm?, flow_length_mm?, speed_m_s?, dt_max?}` | film properties, correlation rows (Ra/Re, Nu, h, validity, equation), radiation, still/walking totals, h-vs-ΔT curves, notes |
| `fluids.air` | `{t_c}` | air properties (Incropera Table A.4 interpolation) |
| `fluids.materials`, `fluids.scenarios` | – | thermal material table (typical); steady scenarios (Power Lab) and transient schedules |
| `fluids.lbm_presets` | – | wind-tunnel presets with reference values |
| `fluids.lbm_init` | `{config: LbmConfig, replace?}` | session id, info (τ, ν, actual Re, Mach, blockage, probe, warnings, Iris Re at walking speed), obstacle mask |
| `fluids.lbm_step` | `{session, steps, field, contrast, tracers, frame, history_points, spectrum}` | step, u8 frame (base64), C_D / C_L now and averaged, lift amplitude, mass, max speed, tracer positions, optional history and spectrum (Strouhal number) |
| `fluids.lbm_paint` | `{session, points, radius, solid, clear}` | updated obstacle mask |
| `fluids.lbm_run` | `{config, steps, frame_every, field}` | stateless batch run: frames, final snapshot with history and spectrum |

`PodConfig` fields (all optional): `voxel_mm` (0.6–2), `scenario`, `display_duty`,
`spreader` (none, graphite, copper, aluminium), `spreader_um`, `spreader_k_in`,
`spreader_location` (floor, lid, optics), `display_heat_in` (optics, pod),
`shell_material`, `emissivity`, `air_speed_m_s`, `h_override`, `ambient_c`,
`precond` (ssor, jacobi), `tol`, `max_iter`, `cu_coverage`.

## The maths

**Weak form.** Steady conduction `−∇·(k∇T) = q` with `−k ∂T/∂n = h (T − T∞)` on
exposed faces. In the excess temperature θ = T − T∞ the boundary is
homogeneous and the Galerkin form with trilinear 8-node bricks is

```
(K + H) θ = Q,   K_e = Σ_d k_d L K̂_d,   K̂_d[a][b] = ∫ ∂N_a/∂ξ_d ∂N_b/∂ξ_d dΩ̂
```

`K̂_d` factorises into ±1 along axis d and the 1-D mass factors 1/3 or 1/6 along
the other two; each element carries its own `[k_x, k_y, k_z]`. The boundary
and capacity matrices are lumped (h A/4 and ρc V/8 per node).

**Geometry and materials.** The CAD parts are voxelised by the Structures
Lab's ray-casting voxeliser (27 sub-samples per voxel → fractions). The PCB
(the box the CAD layout reserves for it), two plastic mounting ribs at its
ends (assumed — the CAD has no mounting features yet), the USB-C receptacle
plugging the slot in the pod wall, the micro-display at the far end of the
optics housing and the spreader sheet are axis-aligned boxes with exact
overlap fractions. Outside air is flood-filled from the grid edge; what is
left inside is the sealed cavity air, with `k = k_air + 4σε_eff T³·gap` (the
cavity Rayleigh number is far below the 1708 onset of convection). Mixed
voxels: parallel along sheets `k = Σ fᵢkᵢ`, series across them — a 25 µm
sheet in a 1.5 mm voxel keeps its true conductance k·t. FR-4 in-plane
conductivity is the rule of mixtures of FR-4 and the copper layers
(`design.pcb.layers` × 35 µm × 50 % fill → ≈ 17 W/m K); through-plane 0.35 W/m K.

**Sources.** Each component's average power for the scenario comes from
`power::budget`, converter losses (load/η − load) plus quiescent power go to
the buck converter, and passives add heat capacity only. Board parts sit
where `pcb::placement` puts them (board x mapped so the USB-C receptacle is
at the pod's USB slot), spread over their package body. The placed total
equals `power::budget::pod_heat_mw` exactly (unit-tested), so the lumped and
3-D models see the same heat.

**Boundary.** Each exposed face belongs to a body (pod, optics housing, front)
and a class (side, top, bottom). Side faces: Churchill–Chu vertical plate with
the body height; top/bottom: horizontal-plate fits with L = A/P; with air
speed V, blended with the laminar flat-plate value, `Nu³ = Nu_N³ + Nu_F³`;
plus linearised radiation `h_r = εσ(T_s + T∞)(T_s² + T∞²)`. h depends on the
mean excess of its group, so the system is solved three times with updated
h (Picard iteration).

**Solver.** `K` in compressed sparse rows (≤ 27 per row), the boundary and
capacity terms as a separate diagonal; preconditioned conjugate gradients
with Jacobi or symmetric successive over-relaxation (SSOR, ω = 1.4).
Transients use implicit (backward) Euler `(C/Δt + K + H)θⁿ⁺¹ = C/Δt θⁿ + Qⁿ⁺¹`,
warm-started, with h linearised about the hottest steady state of the
schedule; the lumped network is stepped exactly (`power::thermal::Network`).

**Convection correlations.** See the table in
`crates/forge-core/src/fluids/convection.rs`: Churchill–Chu plate and
cylinder, horizontal plates (Incropera eq. 9.30–9.32), Churchill–Bernstein,
laminar flat plate, mixed convection, radiation; properties at the film
temperature from Incropera Table A.4.

**Lattice Boltzmann.** D2Q9 with BGK collision `f_i* = f_i − (f_i − f_i^eq)/τ`,
`f_i^eq = w_i ρ[1 + 3c·u + 9/2 (c·u)² − 3/2 u²]`, `ν = (τ − ½)/3`. Obstacles
and no-slip walls: half-way bounce-back; free-slip walls: specular
reflection; inlet/outlet: non-equilibrium extrapolation (velocity in,
density out); driven channels: Guo forcing. Forces by momentum exchange
`F = Σ_links 2 c f*`, `C_D = F_x/(½ρU²D)`; Strouhal number from a
Hann-windowed FFT of the transverse velocity at a probe two diameters behind
the obstacle. Re is set through τ = 3UD/Re + ½ and capped at τ ≥ 0.505 with a
warning (raise the resolution for higher Re).

## Verification (unit tests)

* 1-D slab, heat flux in / convection out: nodal temperatures exact to 10⁻⁸.
* Pin fin with an adiabatic tip: base and mid-length temperatures within 2 % of `θ_b = Q/(√(hPkA) tanh mL)`, `cosh` profile (Incropera §3.6).
* Lumped capacitance: a copper cube (Bi ≪ 0.1) cools as `e^{−t/τ}`, τ = ρcV/(hA), within 1 %; the backward-Euler discrete solution within 0.2 %.
* SSOR needs fewer iterations than Jacobi for the same answer.
* Spreader trade at the page's settings (display heat in the optics housing, 1.5 mm voxels): all nine rows solve, every sheet behind the display gains > 5 °C, the 1 mm bracket at least as much as the 0.5 mm one — a regression test for brackets that add voxels (the baseline warm start is then skipped).
* Iris pod: placed heat = Power Lab pod heat; heat out = heat in to 10⁻⁴; cavity sealed; like-for-like (display heat in the pod) the 3-D mean is far below the lumped 39 °C while the hotspot reaches about it; a graphite sheet lowers the pod hotspot; the transient rises monotonically to the steady state while the lumped network reaches its own 39 °C.
* Correlations: Churchill–Chu Nu = 122.8 at Ra = 10⁹ (hand calculation) and its conduction limits; within 12 % of Holman's simplified air equation for a 0.3 m plate; cylinder within 12 % of Morgan's power law and 20 % of Holman's; Churchill–Bernstein 53.3 at Re = 10⁴ and within 10 % of Hilpert; radiation `4σT³ = 6.12 W/m² K` at 300 K; air properties at the Table A.4 points.
* LBM: equilibrium moments; Poiseuille channel within 1 % (L2) of `u = F y(H − y)/(2ν)`; mass conserved to 10⁻¹² with bounce-back and free-slip walls; Schäfer–Turek 2D-1 (Re 20) C_D within 10 % of 5.58 at only 10 cells per diameter (≈ 6.0 — the staircase cylinder reads high); FFT peak of a test sine; NACA 0012 thickness; session step / paint round trip.

## Iris tie-in and metrics

Metrics run on every project edit, memoised on every input they read
(frame, optics, PCB, thermal, power rails, battery, components, subsystems,
`design.extra.fluids`), on 1.5 mm voxels, display always on (the R-008 case).

| Metric | Iris v0.6 | Meaning |
|---|---|---|
| `fluids.peak_skin_c` | **40.37 °C** (optics housing) | hottest exposed frame surface — R-039 (≤ 41 °C), a marginal pass (1.5 %) |
| `fluids.hotspot_over_mean_c` | **8.92 °C** | peak − area-mean frame surface (31.5 °C over 100 cm²) — R-040 (≤ 10 °C) |
| `fluids.spreader_gain_c` | **21.6 °C** | what the design's 25 µm graphite sheet behind the display (TH1) gains against no spreader |
| `fluids.pod_peak_c` | 31.37 °C | pod shell and lid only (the surfaces R-008 describes) |

**What the simulation caught (v0.5 → v0.6).** The lumped model passes R-008
(39.0 °C ≤ 41 °C) because it smears all 164 mW over one shell node with
R = 55 °C/W. The BOM, however, puts the display and its driver (140 mW with the
display always on) in the small plastic optics housing at the front, not in
the pod. There the heat enters a 1.2 mm PETG wall over the display's
8.4 × 5.3 mm footprint and cannot spread — the spreading length √(k t / h) is
only ≈ 4.6 mm — so in v0.5, which had no spreader, the end wall of the optics
housing reached **62.9 °C** (31.6 °C above the frame mean) while the pod itself
stayed near 31.5 °C. Even like for like (all heat in the pod, as the lumped
model assumes) the 3-D mean is ≈ 31.4 °C and the hotspot over the board's warm
parts reaches ≈ 38.9 °C — close to the lumped value only by coincidence of the
chosen R (the 3-D mean corresponds to ≈ 8 °C/W at h ≈ 12 W/m² K; the lumped
55 °C/W would need h ≈ 1.8 W/m² K).

v0.6 adds a 25 µm synthetic graphite sheet behind the display on the
optics-housing end wall (BOM line TH1, `design.extra.fluids` =
`{ "spreader": "graphite", "spreader_um": 25, "spreader_location": "optics" }`):
the spreading length grows to ≈ 58 mm and the peak falls to **40.37 °C**
(R-039 passes with 0.6 °C of margin) for 0.014 g. The trade tab on the v0.6
design (1.5 mm voxels, baseline without a sheet 62.0 °C): 70 µm graphite or a
0.5 mm aluminium bracket ≈ 39.8 °C, a 1 mm bracket ≈ 39.6 °C, 70 µm copper
40.6 °C. Sheets inside the pod do nothing for this hotspot (≈ 0 °C gain).
Wrapping the sheet further round the housing, lowering the display's duty or
power, or moving the display driver onto the main board would add the margin
R-039 still lacks — it is one of the marginal passes on
[What should I do next?](../next-actions.md).

Lab defaults when a project has no `design.extra.fluids`:
`{ "spreader": "none", "spreader_um": 25, "spreader_location": "floor", "display_heat_in": "optics", "air_speed_m_s": 0 }`.

## Limitations

* Voxels stair-step the drafted, tilted walls; the mesh study shows peak
  temperatures moving by about ±1.5 °C between 2 and 1 mm voxels.
* Convection comes from correlations for isolated plates and cylinders, not
  from a flow solve around the head; up/down faces of these small parts sit
  below the horizontal-plate correlations' validated Rayleigh range (flagged,
  extrapolated). Correlations differ by 10–20 % among themselves.
* The head is not modelled: the skin side of the temple sees the same
  free-air convection as the outside (contact with skin at ≈ 34 °C would
  change the inner-face temperature).
* Board mounting ribs, the display module's effective conductivity
  (2 W/m K) and its bond to the housing wall, the USB-C receptacle (5 W/m K)
  and all material values are typical assumptions — check the datasheets.
* Contact resistances (board to ribs, sheet adhesive) are not modelled; a
  real graphite sheet's adhesive adds through-plane resistance.
* The transient linearises h about the hottest steady state.
* The metrics cost two 3-D steady solves (baseline and spreader variant, ≈ 8.5k
  unknowns each), measured at ≈ 150–300 ms in WebAssembly on a shared machine —
  well above the ~20 ms budget for other labs' metrics. They are memoised on
  every input they read, so they rerun only when the frame, optics, PCB,
  thermal, power, battery, components or `design.extra.fluids` change. A 2 mm
  voxel would roughly halve the cost but moves the peak by ≈ +1.7 °C.
* The LBM tunnel is 2-D with BGK collisions and staircase obstacles: τ ≥ 0.505
  caps Re for a given resolution, C_D reads ≈ 8 % high at 10 cells per
  diameter, side walls (blockage) raise C_D and St. Real wakes above
  Re ≈ 200 are three-dimensional.

## Graduate to

* **OpenFOAM** — finite-volume CFD; `chtMultiRegionFoam` solves conjugate heat
  transfer (air and solids together), replacing the convection correlations.
* **Elmer FEM** (ElmerCSC/elmerfem) — multiphysics finite elements with
  radiation view factors on real tetrahedral meshes of the CAD.
* **Palabos** — parallel lattice-Boltzmann library with curved boundaries and
  multi-relaxation-time collisions.
* **FluidX3D** (ProjectPhysX/FluidX3D) — GPU lattice Boltzmann in 3-D.

No open tool runs the whole loop in a browser from the product file; this lab
takes geometry, placements and duty cycles straight from the Iris project,
compares the 3-D result with the lumped model on the same heat, and verifies
requirements live. Export the VTK file to ParaView to post-process it like
the output of the tools above.

## References

* F. P. Incropera, D. P. DeWitt, T. L. Bergman & A. S. Lavine, *Fundamentals of Heat and Mass Transfer*, 7th ed., Wiley, 2011.
* S. W. Churchill & H. H. S. Chu, "Correlating equations for laminar and turbulent free convection from a vertical plate", *Int. J. Heat Mass Transfer* 18 (1975) 1323–1329; "… from a horizontal cylinder", 18 (1975) 1049–1053.
* S. W. Churchill & M. Bernstein, "A correlating equation for forced convection from gases and liquids to a circular cylinder in crossflow", *J. Heat Transfer* 99 (1977) 300–306.
* S. V. Patankar, *Numerical Heat Transfer and Fluid Flow*, Hemisphere, 1980.
* J. P. Holman, *Heat Transfer*, 10th ed., McGraw-Hill, 2010.
* T. J. R. Hughes, *The Finite Element Method*, Dover, 2000.
* Y. Saad, *Iterative Methods for Sparse Linear Systems*, 2nd ed., SIAM, 2003.
* T. Krüger, H. Kusumaatmaja, A. Kuzmin, O. Shardt, G. Silva & E. M. Viggen, *The Lattice Boltzmann Method: Principles and Practice*, Springer, 2017.
* S. Succi, *The Lattice Boltzmann Equation for Fluid Dynamics and Beyond*, Oxford University Press, 2001.
* A. J. C. Ladd, "Numerical simulations of particulate suspensions via a discretized Boltzmann equation", *J. Fluid Mech.* 271 (1994).
* Z. Guo, C. Zheng & B. Shi, "Non-equilibrium extrapolation method for velocity and pressure boundary conditions in the lattice Boltzmann method", *Chinese Physics* 11 (2002); "Discrete lattice effects on the forcing term in the lattice Boltzmann method", *Phys. Rev. E* 65 (2002) 046308.
* M. Schäfer & S. Turek, "Benchmark computations of laminar flow around a cylinder", Notes on Numerical Fluid Mechanics 52, Vieweg, 1996.
* C. H. K. Williamson, "Vortex dynamics in the cylinder wake", *Annu. Rev. Fluid Mech.* 28 (1996) 477–539.
* Repositories: OpenFOAM (openfoam.org), Elmer FEM (github.com/ElmerCSC/elmerfem), Palabos, FluidX3D (github.com/ProjectPhysX/FluidX3D).
