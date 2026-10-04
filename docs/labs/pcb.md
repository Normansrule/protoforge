# PCB Studio

Route: `#/pcb` · Engine: `crates/forge-core/src/pcb/` · UI: `web/src/labs/pcb/`

PCB Studio turns the project's electronics (`components` with `on_pcb: true`
plus the `design.pcb` block) into a routed two-layer board, checks it with an
independent design-rule check (DRC), and writes the files a board house
needs: Gerber X2, Excellon drill files, a JLCPCB-style BOM and pick-and-place
(CPL) file, an assembly drawing and a README.

Everything runs in the Rust engine (WebAssembly in the browser). The heavy
calls — full routing, the annealer, file generation — run in a Web Worker so
the page stays responsive.

## What it does

| Stage | What happens | Where |
|---|---|---|
| Footprints | 14 IPC-7351-style land patterns (QFN-48, LGA-14, MEMS mic LGA, SOT-23-5/6, DFN-6, USON-8, 0402/0603/0805, USB-C, FPC-24, JST-SH-2, chip antenna) with pads, courtyards and 3-D body sizes | `footprints.rs` |
| Netlist | Pin table for every Iris part (power, I²C, SPI display, PDM mics, QSPI flash, touch, charger), decoupling and board features (touch electrode E1, test pads TP1/TP2) → nets | `netlist.rs` |
| Placement | Constructive placement driven by the enclosure: USB-C at the rear, display FPC at the hinge, SoC next to it, antenna at the edge with its keep-out, microphones spaced along the edge; zones, halos and a RUDY congestion estimate; GPIO signals are assigned to SoC pins with the Hungarian algorithm | `place.rs` |
| Annealing | Optional simulated annealing on half-perimeter wire length (HPWL) with a ramped overlap penalty — shown live in the UI | `place.rs` |
| Routing | Two-layer grid maze router (Lee / A\*) with PathFinder negotiated congestion, pin swapping on SoC GPIOs, ground backbone + drops, ground pours on both layers and union–find ground stitching | `route.rs`, `pour.rs` |
| DRC | Re-extracts connectivity (union–find over copper + pours) and checks clearances, widths, edge distance, drills, annular rings, hole spacing, antenna keep-out and courtyard overlaps from exact geometry — it does not trust the router | `drc.rs` |
| Fab files | Gerber RS-274X with X2 attributes (copper, mask, paste, silkscreen, edge), Excellon PTH/NPTH, BOM/CPL CSV, netlist, assembly SVG, README; plus a parser that re-reads the Gerbers to render them | `fab.rs` |

## The maths

**Trace width (IPC-2221).** For external layers

```
I = 0.048 · ΔT^0.44 · A^0.725        (I in A, ΔT in °C, A in mil²)
width = A / (1.378 mil · oz)
```

Hand check (unit test `ipc2221_hand_calc`): 1 A, 10 °C rise, 1 oz copper →
A = (1 / (0.048 · 10^0.44))^(1/0.725) ≈ 16.3 mil², width ≈ 11.8 mil → **0.300 mm**.

**HPWL.** For every net, the half perimeter of the bounding box of its pins;
summed over nets it is the standard fast estimate of routed length.

**Simulated annealing.** Random moves / rotations / swaps; accept a worse
layout with probability `exp(−ΔC / T)`, `T ← αT`. The cost is
`HPWL + λ · overlap`, with λ ramped up so parts can pass through each other
early and must be legal at the end. On Iris v0.6 it takes the constructive
placement from ≈ 387 mm to ≈ 373 mm HPWL with zero overlap.

**Maze routing.** The board is a grid (default 0.25 mm, routed on a half
grid of 0.125 mm) on two layers. Each cell and each grid edge carries an
owner: free, blocked, or "within clearance of net N" — only net N may run a
centre-line there. A search floods from the net's pins with costs
`1` per top-layer step, more on the bottom layer (which carries the ground
pour), an extra cost per bend and per via, and stops at any copper already on
the net (a rectilinear Steiner heuristic).

**Negotiated congestion (PathFinder).** Nets route even when they overlap,
but overlap is priced:

```
cost(cell) = base + p_pres · (other nets here) + h_hist(cell)
p_pres ← 1.35 · p_pres + 2   (capped),   h_hist += 16 on each contested cell
```

Every net is ripped up and re-routed each iteration; the best iteration is
kept and a strict repair pass removes any remaining conflict.

**Ground.** Thermal vias go into exposed pads; a ground backbone joins the
connector ground pins and thermal pads; every other ground pad drops to the
bottom pour through the cheapest legal via. Pours are rasterised at 0.1 mm,
islands removed and merged into rectangles; a union–find over pours + copper
adds stitching vias or short bridges until ground is one island.

## How to use it

1. Open **PCB Studio**. The Iris board is pre-routed by this engine (a
   baked route keyed by a hash of the PCB inputs, re-verified by a unit test)
   so the page appears instantly; any change to the PCB inputs triggers a
   fresh route in the worker (a few seconds).
2. **Board & DRC** — toggle layers, zoom/pan, click a pad or track to
   highlight its net, read the net table, the DRC list and the SoC pin
   assignment the router chose. Edit **Design rules** (trace, clearance,
   via, grid, board size) and press *Re-route*; if it routes cleanly and
   *Apply* is ticked, the rules are written back to the project.
3. **Placement annealing** — watch HPWL and overlap fall as the annealer
   runs; ghost parts and ratsnest lines show what moved.
4. **Lee wavefront** — pick a net and connection and watch the search flood
   the grid, then back-trace the path.
5. **Gerbers & files** — every file is rendered by parsing the generated
   Gerber/Excellon text; download the zip, BOM, CPL, README or the assembly
   drawing.
6. **Footprints** / **3-D** — inspect the land patterns and the assembled board.

## Engine API

| Method | Input | Output |
|---|---|---|
| `pcb.layout` | `{ project }` | Layout: board, parts, pads, nets, tracks, vias, pours (both layers), keep-outs, DRC list, stats, notes, silkscreen |
| `pcb.adopt` | `{ project, layout }` | `true` — seeds the engine cache with a layout computed in the worker |
| `pcb.route_trace` | `{ project, net, placement? }` | `WaveTrace { net, nx, ny, grid_mm, waves: [{ from_pin, expanded, path, found }] }` |
| `pcb.anneal` | `{ project, iterations = 20000, seed = 42 }` | Trace of HPWL / overlap / temperature and the final placement |
| `pcb.placement` | `Project` | Constructive placement (`Placed[]`: refdes, x, y, rotation, size) — used by the robotics lab |
| `pcb.fab_files` | `{ project, placement? }` | `FabFile[]` `{ path, content, kind }` |
| `pcb.footprints` | – | Footprint library |
| `pcb.ipc2221` | `{ current_a, temp_rise_c, copper_oz, external }` | `{ width_mm, current_for_0_15mm_a }` |

Metrics (feed requirements R-017, R-018):

| Key | Iris value (v0.6) |
|---|---|
| `pcb.routed_pct` | **100 %** (108/108 connections, 34 nets, 32 parts) |
| `pcb.drc_violations` | **0** |
| `pcb.via_count` | 101 |
| `pcb.track_length_mm` | 530 mm (352 top / 178 bottom) |
| `pcb.hpwl_mm` | 381 mm |

## How Iris uses it

Iris's main board is a 58 × 10 × 0.8 mm two-layer board in the right temple
pod. The routed layout proves R-017 (all nets routed) and R-018 (zero DRC
violations); its outline feeds the pod length check in CAD (R-019, ≥ 1 mm
clearance); the robotics lab plans pick-and-place from `pcb.placement`; the
Manufacturing Hub zips these files with the BOM/CPL and costs the board.

The engine also reports **BOM gaps** it notices while routing. Up to v0.5 it
flagged three that no requirement covered: the USB-C sink needed 5.1 kΩ Rd
resistors on CC1/CC2, the BLE SoC needed a crystal, and the antenna needed a
radio-frequency (RF) matching network. **All three are on the v0.6 board** — R6/R7
(5.1 kΩ configuration-channel (CC) pull-downs), Y1 (32 MHz crystal) with C7/C8 (12 pF load capacitors), and L2/L3
(the 1.8 nH + 4.3 nH match the [RF & Antenna Lab](rf.md) synthesised) — and the
board still routes 100 % with 0 DRC violations. The remaining notes: CHG_STAT
is open-drain (enable the SoC's internal pull-up on that GPIO). The antenna
feed `RF_ANT` (matching network → antenna) has its own net-class width: the
router lays it first, at the 0.91 mm grounded coplanar waveguide (CPWG) width
the RF & Antenna Lab stored in `design.extra.rf.feed`, and freezes it before
the other nets negotiate (the KiCad export carries it as the RF net class). The
SoC side of the match (`RF`) stays at the minimum width because the 0.4 mm-pitch
QFN pad cannot take a wide trace. If the wide feed cannot be routed it falls
back to the minimum width and the board notes say so. `rf.feed_z0_ohm` (R-037)
reads the width actually routed on `RF_ANT`, so it checks the board, not just
the stored design.

## Limitations

* The SoC pinout is **illustrative**: power, RF and debug pins are fixed,
  GPIO signals are pin-swapped to the nearest free pads. Re-map to your SoC's
  real pin-mux table.
* Land patterns are typical approximations — verify every footprint against
  the part datasheet or an IPC-7351 calculator.
* Grid router: no any-angle or 45° routing, no length matching, no
  differential pairs, no impedance control, no teardrops. USB D+/D− are
  routed as ordinary nets.
* Two layers only, parts on top only. No blind/buried vias.
* DRC checks copper geometry, drills and courtyards, not mask slivers, paste
  ratios, silkscreen-on-pad or acid traps.
* A change to PCB inputs (component list, rules, board size) re-routes the
  whole board: a few seconds in the worker, and the metrics call blocks for
  the same time on the main thread until the worker result is adopted.
* LCSC part numbers are deliberately left blank in the BOM — choose real parts.

## Graduating to professional tools

* **KiCad** (schematic + layout) with **FreeRouting** for autorouting;
  import this project's netlist and placement as a starting point.
* **gerbv** or **tracespace** to view the Gerbers independently before
  ordering; the fab's own online Gerber viewer and DFM check.
* Altium Designer / Cadence OrCAD / Allegro for impedance-controlled,
  multi-layer production boards.

## References

* IPC-2221, *Generic Standard on Printed Board Design*.
* IPC-7351, *Generic Requirements for Surface Mount Design and Land Pattern Standard*.
* C. Y. Lee, "An algorithm for path connections and its applications",
  *IRE Trans. Electronic Computers*, 1961.
* L. McMurchie, C. Ebeling, "PathFinder: a negotiation-based
  performance-driven router for FPGAs", FPGA 1995.
* S. Kirkpatrick, C. D. Gelatt, M. P. Vecchi, "Optimization by simulated
  annealing", *Science* 220, 1983.
* Ucamco, *The Gerber Layer Format Specification* — <https://www.ucamco.com/en/gerber>.
* JLCPCB help: BOM and CPL files — <https://jlcpcb.com/help>.
