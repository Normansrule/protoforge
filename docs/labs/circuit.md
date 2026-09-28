# Circuit Lab

A small SPICE written in Rust, compiled to WebAssembly and driven from the
browser. It reads a practical subset of the SPICE3/ngspice netlist language,
solves it with Modified Nodal Analysis and Newton–Raphson, and runs DC
operating-point, DC-sweep, transient and AC analyses. The web page draws the
circuit, colours every net by its voltage and animates the current flowing
through the wires, frame by frame through the transient.

Everything the lab produces is downloadable: the `.cir` deck (runs unchanged
in ngspice), CSV files of the transient, AC, DC-sweep and operating-point
results, and the full result as JSON.

| Piece | Location |
|---|---|
| Engine | `crates/forge-core/src/circuit/` (`parser.rs`, `mna.rs`, `devices.rs`, `analysis.rs`, `measure.rs`, `presets.rs`, `schematic.rs`, `mod.rs`, `tests.rs`) |
| Page | `web/src/labs/circuit/` (`index.tsx`, `Schematic.tsx`, `Explainers.tsx`, `util.ts`, `types.ts`, `circuit.css`; schematic editor: `SchEditor.tsx`, `SchSymbols.tsx`, `SchResults.tsx`, `schTypes.ts`, `schematic.css`) |

The page has two top-level tabs: **Simulator & presets** (everything below up to
"Schematic editor") and **Schematic editor** — drag-and-drop circuit capture in the
spirit of Falstad's circuit simulator and EasyEDA, whose drawing is turned into a
SPICE netlist and simulated by the same engine.

## What it does

* **Netlist parser.** Elements `R C L V I D M E G S`; sources with `DC`,
  `AC mag [phase]`, `PULSE`, `SIN`, `PWL` and `EXP`; directives `.op`,
  `.dc`, `.tran` (with `uic`), `.ac dec|oct|lin`, `.model` (D, NMOS/PMOS
  level 1, SW), `.ic`, `.nodeset`, `.param` with `{expr}` substitution,
  `.options` (`reltol abstol vntol gmin itl1 itl4 trtol`) and `.end`.
  Comments (`*`, `;`, ` $ `), `+` continuation lines, case-insensitive
  names, `0`/`gnd` as ground, and engineering suffixes
  `f p n u m k meg g t mil`. Errors are reported with their line numbers;
  `.control` blocks and output cards (`.save`, `.print`, `.meas` …) are
  skipped with a note.
* **Solver.** Dense MNA with an in-place LU (reused when the matrix is
  unchanged), Newton–Raphson with SPICE `pnjlim` junction limiting and
  simplified `fetlim`/`limvds` MOSFET limiting, 1 pS gmin across every
  junction, and a fallback chain when the operating point fails: plain
  Newton → gmin stepping (10 mS → 1 pS shunts) → source stepping (0 → 100 %,
  adaptive). Floating nodes get 1 pS shunts with a warning.
* **Transient.** Trapezoidal companion models; backward Euler for the first
  step and after every source corner (PULSE/PWL/EXP breakpoints) and switch
  event; steps land exactly on breakpoints; local-truncation-error control
  (predictor–corrector difference) grows or shrinks the step; Newton failures
  cut the step by 8×. Hard limits (250 000 points, 2 M solves) keep the tab
  responsive.
* **AC.** Complex MNA linearised at the DC operating point, one solve per
  frequency, magnitude and unwrapped phase for every node.
* **Outputs.** Every node voltage and every element current at every time /
  sweep point; operating-point table with element V, I, P and device state
  (MOSFET region, gm, gds, Vth; diode rd); solver statistics.

## The maths

**Modified Nodal Analysis** (Ho, Ruehli & Brennan 1975). KCL at every node
with node voltages as unknowns gives `G·v = i`. Elements whose current is
not a function of node voltages (voltage sources, inductors, VCVS) add their
branch current as an unknown and their voltage law as an extra row:

```text
resistor a–b:   A[a][a] += G   A[b][b] += G   A[a][b] −= G   A[b][a] −= G
V source (k):   A[a][k] += 1   A[b][k] −= 1   A[k][a] += 1   A[k][b] −= 1   b[k] = V
current a→b:    b[a] −= I      b[b] += I
VCCS:           A[a][c+] += gm, A[a][c−] −= gm, A[b][c+] −= gm, A[b][c−] += gm
```

Currents follow the SPICE convention: positive *into* the first node (n+),
through the element, out of n−; a MOSFET's current is the drain current. A
source that delivers power therefore reads negative.

**Newton–Raphson** (Nagel 1975). A nonlinear device is replaced, at the
present guess, by its tangent: a conductance and a current source.

```text
diode:  I = IS·(e^(V/nV_T) − 1),  g_d = dI/dV,  I_eq = I(V_k) − g_d·V_k
converged when |x_{k+1} − x_k| ≤ reltol·|x| + vntol  (1e-3, 1 µV) and no device limited
pnjlim: if V_new > V_crit and |ΔV| > 2nV_T:  V_new = V_old + nV_T·ln(1 + ΔV/nV_T)
```

Without limiting, a diode fed from 5 V through 1 kΩ takes ~170 iterations
from 0 V (the page's Newton demo plots both); with `pnjlim` it takes ~11.

**Level-1 MOSFET** (Shichman & Hodges 1968), β = KP·W/L:

```text
cutoff      V_GS ≤ V_T:          I_D = 0
triode      V_DS < V_GS − V_T:   I_D = β(V_GS − V_T − V_DS/2)·V_DS·(1 + λV_DS)
saturation                       I_D = (β/2)(V_GS − V_T)²·(1 + λV_DS)
V_T = VTO + γ(√(φ − V_BS) − √φ)
```

PMOS is handled by sign normalisation and drain/source interchange when
V_DS < 0; the full 4-terminal Jacobian is stamped.

**Transient companion models.**

```text
backward Euler:  i₊ = (C/h)·v₊ − (C/h)·v
trapezoidal:     i₊ = (2C/h)·v₊ − [(2C/h)·v + i]
inductor:        v₊ = (2L/h)·i₊ − [(2L/h)·i + v]
nonlinear charge (diode CJ, TT):  i₊ = a0·q(v₊) − hist, linearised with C(v)
LTE ≈ |x_corrector − x_predictor| / 13  (quadratic predictor), step ∝ (tol/LTE)^(1/3)
```

The trapezoidal rule is second-order and exact for linear ramps but rings
when h ≫ τ; backward Euler is first-order and heavily damped, which is why
it is used right after discontinuities.

**AC.** `Y_C = jωC`, `Z_L = jωL`, devices replaced by g_d, gm, gds at the
bias point; `|H|dB = 20·log10|V_out/V_in|`; the −3 dB point is where
`|H| = |H(0)|/√2`.

## Using the page

1. Pick a circuit in the gallery (Iris circuits first, then classics, then
   **Custom netlist**).
2. Drag the sliders: each one rewrites a `.param` card and re-simulates
   (typically 2–15 ms in WebAssembly).
3. Watch the schematic: nets are coloured by voltage (legend below), dots
   move along wires at a speed ∝ log |I| in the direction of positive
   current. **Play/Pause** and the scrubber step through the transient; hover
   or tap a part for its V, I, P and DC state.
4. **Measurements** are computed from the waveforms (like ngspice `.meas`),
   with pass/fail against the Iris requirement where one applies.
5. **Results** tabs: waveforms (choose probes, zoom window vs. full run),
   Bode plot, DC sweep, operating-point table and the netlist editor (live
   parse errors with line numbers, Ctrl+Enter to run). Editing a preset's
   deck keeps its measurements; if the element names no longer match the
   drawing, the page switches to an automatic node graph.

### API (`circuit.*`)

| Method | Input | Output |
|---|---|---|
| `circuit.presets` | `{project?}` | preset list: id, title, group, description, Iris tie-in, netlist, slider specs, default probes, schematic layout (with resolved pins), links, verified metric |
| `circuit.parse` | `{netlist, params?}` | `{ok, errors[{line,message}], warnings, title, nodes, elements, analyses, params}` |
| `circuit.simulate` | `{netlist, params?, compact?}` | `SimResult`: nodes, elements, op, dc, tran, ac, measurements, annotations, stats, warnings, effective netlist |
| `circuit.run_preset` | `{id, params?, netlist?, project?, compact?}` | `SimResult` plus the preset's measurements, plot annotations and suggested time window |
| `circuit.schematic_netlist` (alias `circuit.schematic_to_netlist`) | `{schematic}` | `Extracted`: SPICE netlist, nets (pins, labels, probe flag), per-part pin positions / nets / emitted elements / netlist line numbers, wire pieces with their current `flow`, net of every drawn wire, junction dots, probed nets, node count, warnings (unconnected pins, shorted labels) |
| `circuit.schematic_simulate` | `{schematic, compact?}` | `{extracted, result?: SimResult, error?}` — extraction always returns; a simulation error (with netlist line numbers) comes back in `error` |
| `circuit.schematic_from_preset` | `{id, project?}` | an editable `Schematic` drawn from any preset's hand-made layout; each part keeps its original SPICE card(s) as a template with `<p0>`, `<p1>` … for its pins, other deck lines become directives |
| `circuit.schematic_examples` | – | starter schematics: RC low-pass (pulse + AC), LED driver (NMOS low-side switch), buck power stage (PWM switch, Schottky, LC, load) |

`compact: true` returns waveform arrays as base64 little-endian `f32`
(`f64` for time/frequency/sweep) instead of JSON number lists — 4–6× smaller;
the web page uses it.

The schematic layout format is documented at the top of `presets.rs`: parts
(symbol, position, pin-1→pin-2 direction, label side), wire polylines tagged
with their net and a signed list of element currents flowing along them,
ground symbols, labels/ports and grouping boxes. A unit test checks that
every pin and wire end of every preset is connected on the right net.

## Schematic editor

A second tab turns the lab into schematic capture: draw the circuit, and the engine
extracts the netlist and simulates it.

**Drawing.** A palette of parts — resistor, capacitor, inductor, voltage source (DC,
PULSE or SIN, optional AC magnitude), current source, battery, diode, LED, NMOS, PMOS,
switch (closed / open / PWM-driven), op-amp (single-pole macro-model), ground, net label
and voltage probe. Drag a part onto the sheet (or click it, then click the sheet); **R**
rotates, **F** mirrors a MOSFET, **Del** deletes. **W** draws wires on a half-unit snap
grid with automatic orthogonal (L-shaped) routing: click to add corners, and the wire ends
when it lands on a pin or another wire (or on double-click / Enter / right-click). Dragging
a part rubber-bands the wires attached to it. Junction dots appear wherever three or more
wire ends or pins meet; wires that merely cross are *not* connected, exactly as in KiCad
and EasyEDA. Hovering a wire highlights its whole net. Undo/redo (Ctrl+Z / Ctrl+Shift+Z,
100 steps), pan (drag the background), zoom (wheel or buttons) and fit.

**Editing.** The property panel edits name and value with engineering suffixes (live
parsed: `4.7k`, `100n`, `1meg`; `m` is milli), source waveforms, capacitor/inductor
initial conditions, diode and MOSFET model parameters, switch state and PWM, op-amp gain
and gain–bandwidth, and — for parts imported from a preset — the raw SPICE card(s). The
analyses panel sets `.op`, `.tran` (step, stop, `uic`), `.ac` (points per decade, range)
and `.dc` (source, range, step).

**Simulating.** Every edit re-extracts the netlist; with auto-run on, the circuit is
re-simulated 250 ms after the last change (typically a few milliseconds). Nets are
coloured by voltage and dots move along *the wires as drawn* at a speed ∝ log |I|, frame by
frame through the transient (play/pause, scrubber, speed; still under
`prefers-reduced-motion`). LEDs glow with their current, switches show their state, labels
and probes show their voltage, hovering a part gives V, I and P. A simulation error marks
the offending part in red. Results reuse the lab's plots: waveforms (probes default to the
probed and labelled nets), Bode, DC sweep and the operating-point table, each with CSV.

**Starting points and files.** Blank sheet, three starter examples, or **import any
preset** of the simulator tab (its hand-drawn layout and SPICE cards become an editable
schematic; the import is unit-tested to reproduce every preset's node set and operating
point). Save/open the schematic as JSON (also kept in the browser between visits), export
the netlist (`.cir`, runs unchanged in ngspice) and the drawing (`.svg`, theme colours
resolved).

**How the netlist is extracted** (`schematic.rs`):

1. Every pin, wire vertex, ground and label anchor is a grid point (rounded to 1/1000 of
   a unit). Wire segments are split at every point lying on them (T-junctions, pins
   touching a wire).
2. **Union–find** (Tarjan 1975) joins the ends of every segment; labels with the same
   name and every ground are joined by virtual links; ground is node `0`.
3. Nets are named ground → `0`, then by label, then by the node name an imported preset
   used, else `n1, n2 …`. Unconnected pins and differently named labels wired together are
   reported.
4. Each part emits its SPICE card(s): `R C L V I D M S` cards with generated `.model`
   lines (diode: IS 2.52 nA, N 1.752, RS 0.568 Ω; LED fitted to ≈ 1.9 V at 20 mA; MOSFET
   level 1 with bulk tied to source; switch `SW` driven by a hidden control source); the
   op-amp becomes `E` (gain A₀) → R·C pole at GBW/A₀ → unity `E` buffer.

**Current on the drawn wires.** The simulator only knows element currents. Each pin
injects a signed element current (SPICE convention: `i(X)` flows into pin 1, so pin 1
injects −i and pin 2 +i); on a spanning tree of each net (real segments first, then the
virtual label/ground links) the current along a tree edge is the sum of the injections on
one side — Kirchhoff's current law — stored as a signed list of element currents, the same
`flow` format the preset layouts use. Chains of plain wire bends are merged into one
animated path. A unit test checks a divider drawn with extra segments and a mid-wire T:
the probe branch carries 0 A and every top-rail piece carries the full 250 µA.

## How Iris uses it

| Metric | Preset | Iris value | Requirement |
|---|---|---|---|
| `circuit.rail_droop_mv` | `rail_load_step` | **7.5 mV** | R-012 ≤ 50 mV |
| `circuit.buck_ripple_mv` | `buck_converter` | **1.5 mV** p-p | R-013 ≤ 30 mV |
| `circuit.vbat_divider_ua` | `vbat_divider` | **1.91 µA** | R-014 ≤ 2 µA |

* **1.8 V rail load step.** The buck is modelled by its closed-loop output
  impedance (`Rreg + Lloop‖Rdamp`, with `Lloop = 1/((2π·fc)²·Cout)` so |Zout|
  meets the capacitor's impedance at the loop crossover fc = 100 kHz), C2 with
  ESR/ESL, the PCB trace and the SoC decoupling; the display steps the load
  1 mA → 60 mA in 1 µs. Hand estimate ΔI/(2π·fc·C) ≈ 9.4 mV; simulated 7.5 mV
  including 3 mV of static IR drop. `vnom` comes from
  `design.power_rails.core_v`, C2 from the BOM.
* **Buck converter.** 3.7 V (battery nominal) → 1.8 V at 2 MHz, 2.2 µH (BOM
  L1), 10 µF (BOM C2) with 5 mΩ ESR, 50 mΩ switches, 5 ns dead time with
  body diodes, 150 mA load. It starts from its steady-state inductor current
  and capacitor voltage (`uic`) because the LC filter has Q ≈ 40 and would
  take ~1.6 ms to settle open-loop. Ripple is the worst single-cycle
  peak-to-peak of the last four cycles; the inductor ripple (212 mA) matches
  (Vin − Vout)·D/(L·f) within a few percent.
* **Battery-sense divider.** 1 MΩ / 1.2 MΩ (BOM R1/R2) at a full 4.2 V cell:
  1.909 µA (≈ 17 mAh per year of storage). The preset also simulates C6
  feeding the ADC's sample-and-hold (kick-back ≈ 114 µV per sample) and the
  54.5 ms settling after power-up.
  **Finding:** at 4.2 V the ADC pin sits at 2.29 V, above the 1.8 V rail the
  SoC runs from, so the measurement is flagged. A ratio such as R1 = 1.5 MΩ,
  R2 = 1.0 MΩ keeps the pin at 1.68 V and the current at 1.68 µA.
* **IR LED driver** (wear detection): 1.8 V GPIO → logic-level NMOS → IR LED
  → 33 Ω from the 3.3 V rail: ≈ 59 mA pulses, 5.9 mA average at 10 % duty.
* **Capacitive touch:** a finger adds Cf‖Cbody ≈ 9.4 pF → the pad crosses
  0.7·VDD 11.3 µs later (180 counts of a 16 MHz timer).
* Classics (CMOS inverter, RC anti-alias filter, op-amp preamp, RLC,
  half-wave rectifier) link to the Chip, DSP and Control labs.

## Limitations

* Dense matrices: fine for tens of nodes, slow beyond a few hundred.
* No sub-circuits (`.subckt`), `.include`/`.lib`, behavioural sources
  (`B`, `E … VALUE=`), BJTs/JFETs, coupled inductors (`K`), transmission
  lines, noise, distortion, `.four`, `.meas`, temperature sweeps.
* MOSFETs are level 1 only (constant overlap/junction capacitances,
  no mobility degradation, no subthreshold conduction).
* Diode breakdown uses BV directly (no IBV fitting); junction capacitance
  uses the SPICE depletion formula with FC linearisation.
* The LTE test is voltage-based (a stand-in for SPICE's charge-based test)
  and trapezoidal only (no Gear/BDF option).
* Switches change state abruptly (like ngspice's `SW`), with hysteresis.
* The buck is simulated open-loop; closing the loop belongs to the Control Lab.
* Schematic editor: one sheet, no hierarchy or buses, no multi-select or copy/paste,
  rotation does not drag the attached wires along (move the part to rubber-band them),
  and the part library is generic models, not manufacturer parts. Touch screens place
  parts by tap (tap the palette, then the sheet) since HTML drag-and-drop needs a mouse.

## Graduating to professional tools

* **ngspice** — the open-source SPICE3 descendant: BSIM transistor models,
  sub-circuits, behavioural sources, `.meas`, scripting. Every deck here runs
  with `ngspice -b deck.cir` (or `ngspice -b -r out.raw deck.cir`).
* **Xyce** (Sandia) — parallel SPICE for very large circuits.
* **Qucs-S** — schematic capture front end for ngspice/Xyce.
* **KiCad** — its schematic editor has ngspice built in, so the Iris
  schematic itself can be simulated.
* **Falstad circuit simulator / CircuitJS** — the browser classic for animated
  current and voltage colouring that the schematic editor's animation follows.
* **EasyEDA** — web schematic capture and PCB layout tied to a parts library and a fab.

## References

* L. W. Nagel, "SPICE2: A Computer Program to Simulate Semiconductor
  Circuits", UC Berkeley ERL Memo M520, 1975.
* C.-W. Ho, A. E. Ruehli, P. A. Brennan, "The modified nodal approach to
  network analysis", IEEE Transactions on Circuits and Systems, CAS-22(6),
  1975.
* H. Shichman and D. A. Hodges, "Modeling and simulation of insulated-gate
  field-effect transistor switching circuits", IEEE Journal of Solid-State
  Circuits, SC-3(3), 1968.
* J. Vlach and K. Singhal, *Computer Methods for Circuit Analysis and
  Design*, 2nd ed., Van Nostrand Reinhold, 1994.
* ngspice User's Manual, <https://ngspice.sourceforge.io/docs.html>.
* P. Falstad, Circuit Simulator applet (2005–) and its JavaScript port CircuitJS1 by
  I. Sharp, <https://www.falstad.com/circuit/>.
* R. E. Tarjan, "Efficiency of a good but not linear set union algorithm", *Journal of
  the ACM* 22(2), 215–225, 1975.
* KiCad and EasyEDA documentation on schematic capture and netlist export.
