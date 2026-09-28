# Print Lab (`#/print`, engine module `print`)

A planar FDM slicer written in Rust and running in the browser. It slices the
CAD Studio's parts in their print orientation into layers of toolpaths,
writes generic Marlin/Klipper G-code, estimates time, filament and cost, and
flags overhangs that would need support.

## What it does

* **Slice a part** — pick any Iris frame part (or the enclosure body/lid
  after "Send to Print Lab" in the CAD Studio), adjust settings, slice.
  Tiles show time, filament (m / g / cm³), cost, layer count and fill ratio.
* **Layers & toolpaths** — 2D layer preview with beads drawn at their real
  width, coloured by move type (outer wall, inner wall, sparse infill,
  solid skin, skirt, travel dashed, retracting travel in red), previous
  layer ghosted, zoom/pan, and a *trace* animation that replays the print
  order. A 3D toolpath stack builds up layer by layer with play/pause.
* **Time & material** — time per layer (with the minimum-layer-time
  slow-down), cumulative time, time and length by move type, retractions,
  per-layer CSV download.
* **Overhangs** — heat map of down-facing faces vs a threshold (default
  45° from vertical), area histogram, support suggestion; compare the print
  orientation with the as-worn orientation.
* **G-code** — preview and download.
* **Whole frame set** — fast estimate for all six parts (what requirement
  R-021 uses) and an on-demand full slice for comparison, with STL downloads.
* Settings (layer height, line width, perimeters, infill, speeds, temps,
  nozzle, filament) are stored in `design.print`; slicer-only settings
  (first layer, skins, acceleration, minimum layer time, fan, retraction,
  price) live on the page. Presets: hobby i3-class (the Iris default), fast
  CoreXY-class, draft, fine.

## Engine API

| Method | Input | Output |
|---|---|---|
| `print.slice` | `{project, part}` or `{mesh}`, `params?`, `settings?` | `layers` (per layer: z, height, paths `{k: kind, r: retract, p: [x,y… in 1/100 mm]}`), `per_layer` stats, totals, G-code, the print-oriented mesh, bbox |
| `print.estimate` | `{project, params?, settings?, method?: "fast" \| "sampled" \| "full"}` | per-part and total time / filament / cost |
| `print.overhangs` | `{project, part, threshold_deg, orientation: "print" \| "assembly"}` or `{mesh}` | per-face colours, overhang / bed / down-facing areas, angle histogram, suggestion |
| `print.settings_defaults` | – | default `SliceSettings` |

Move kinds: 0 travel, 1 external perimeter, 2 inner perimeter, 3 sparse
infill, 4 solid infill, 5 skirt.

### Metrics

| Key | Meaning | Iris (defaults) |
|---|---|---|
| `print.total_time_h` | all six frame parts, print orientation | **1.83 h** (full slice 1.81 h) — R-021 ≤ 8 h ✓ |
| `print.filament_g` | filament for the set | **26.4 g** (full slice 25.9 g) |
| `print.cost_usd` | at 20 USD/kg (informational) | 0.53 USD |

Because `metrics()` runs on every project edit, the metrics use the fast
analytic estimator (≈ 13 ms in the browser for all parts, cached per
parameter set). The test `iris_parts_slice_and_estimate_agrees` compares it
with full slices: every part within 10 % in time and filament (largest
deviation: right temple +7 % time), totals within 5 % (currently +1.3 %
time, +2 % filament).

## The algorithm

1. **Sectioning.** At each layer's mid-height every straddling triangle
   gives one segment, oriented along ẑ × n so outer contours run
   counter-clockwise. Crossings are computed from each edge's vertices in a
   canonical order, so endpoints match bit-for-bit and chain through a hash
   map into closed loops.
2. **Regions.** Loops are cleaned by the winding-number rule (merging
   overlapping or touching shells) and grouped into islands by containment.
3. **Perimeters.** Island offset inwards by w/2 + k·s, with bead spacing
   s = w − h(1 − π/4) (Slic3r flow maths), k = 0…P−1; loops that vanish are
   dropped — thin walls lose their inner perimeters.
4. **Skins.** Solid = infill ∖ (outline[i−B] ∩ outline[i+T]), then a
   morphological opening removes slivers narrower than a line.
5. **Infill.** Rectilinear lines on a global grid, spacing s (solid) or
   s/ρ (sparse), alternating 45°/135°, clipped by scan-line crossings.
6. **Ordering.** Nearest island first, perimeters inside-out with the seam
   at the vertex nearest the nozzle, infill chained greedily; travel moves
   longer than 1.5 mm retract.
7. **Extrusion.** A_bead = (w − h)·h + π(h/2)², E = A_bead·L / (π d²/4).
8. **Time.** Trapezoidal velocity profile per run (runs split at corners
   sharper than 30°): t = L/v + (v − v_j)²/(a·v), or the triangular profile
   for short moves; plus retractions, layer changes and a minimum layer
   time for cooling.
9. **Fast estimator.** Each layer is only sectioned and summarised by area
   A, boundary length L and χ = outer loops − holes; Steiner's formulas
   A(d) = A − L·d + πχd², L(d) = L − 2πχd give perimeter lengths and infill
   area, and Cauchy–Crofton (a curve of length L meets lines spaced s apart
   ≈ 2L/(πs) times) gives the number of infill segments.
10. **G-code.** Header with temperatures (`M140/M104/M190/M109`), `G28`,
    `G90`, `M82`/`M83`, purge line, `M106` after layer 1, retraction,
    end script. The tests check that every line parses, that E never
    decreases on extrusion moves and that every unretract restores E.

## How Iris uses it

Every Iris part is generated to lie flat on its largest face, so none needs
support (tested); the overhang tab shows the temples would need ~1500 mm² of
support if printed as worn. The frame set prints in under two hours on a
hobby printer at 60 mm/s, far inside R-021's 8 h.

## Handoff to a real slicer

The G-code here is generic and centred at X128 Y128 on a 256 mm bed. For
real prints — and always for **Bambu Lab** printers — export the 3MF plate or
STL zip from the CAD Studio, open it in **Bambu Studio** or **OrcaSlicer**,
pick your printer and a PETG profile, match 0.2 mm layers / 3 walls / 25 %
infill / 4 top and bottom layers, supports off, and slice. **PrusaSlicer**
and **Cura (CuraEngine)** work the same way for other printers.

## Limitations

* No gap fill or variable-width perimeters (Arachne); walls thinner than one
  line are not printed.
* Rectilinear infill only; no gyroid, ironing, supports, bridges detection,
  seam hiding, pressure advance or multi-material.
* Travel moves do not avoid crossing perimeters (combing).
* The time model ignores firmware look-ahead details; expect ±10–20 % vs a
  real printer.
* Overhang analysis is per face (flat faces resting on material below are
  recognised as supported).

## References

* I. Gibson, D. Rosen, B. Stucker, *Additive Manufacturing Technologies*, 2nd ed., Springer, 2015.
* ISO/ASTM 52900:2021, *Additive manufacturing — General principles — Fundamentals and vocabulary*.
* RepRap wiki, "G-code" — https://reprap.org/wiki/G-code
* Marlin firmware, G-code index — https://marlinfw.org/meta/gcode/
* Klipper documentation, "Kinematics" (trapezoid generator, square corner velocity).
* Slic3r manual, "Flow math".
* X. Chen, S. McMains, "Polygon Offsetting by Computing Winding Numbers", ASME IDETC/CIE 2005.
* L. A. Santaló, *Integral Geometry and Geometric Probability*, Addison-Wesley, 1976.
* M. de Berg et al., *Computational Geometry: Algorithms and Applications*, 3rd ed., Springer, 2008.
