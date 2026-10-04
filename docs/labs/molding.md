# Molding Lab

Route: `#/molding` · Engine: `crates/forge-core/src/molding/` · UI: `web/src/labs/molding/`

The Molding Lab asks "could this frame be injection-molded, and what would
that take?" It reads the frame parts produced by the CAD Studio
(`crate::cad::frame_parts`) and runs design-for-manufacture (DFM) checks —
draft, undercuts, wall thickness, flow length — then estimates clamp force,
cycle time, tooling and piece cost. All costs are **indicative — get real
quotes**.

## What it does

* **Draft map** — every triangle coloured by its draft angle relative to the
  pull direction: red 0°, amber below the minimum, green ≥ 2°, grey for
  floors/tops, magenta for undercuts. The pull arrow is drawn on the part.
* **Pull direction** — automatic (best of ±X/±Y/±Z and the part's principal
  axes) or forced by the user.
* **Undercuts** — faces that would be trapped by the part itself, found by
  ray casting.
* **Wall thickness map** — per-triangle thickness; nominal, 95th percentile
  and maximum; warnings for thick sections that will sink and cool slowly.
* **Process** — projected area, clamp force and recommended press, gate at
  the thickest section, flow-length/wall ratio vs the resin's limit, shrink
  scale for the tool, cycle breakdown (clamp → fill → pack → cool → eject,
  animated), tool cost, piece price and a cost-vs-quantity curve.
* **Materials** — PC, ABS, PC/ABS, PP, PA66, POM, TPU, PETG with typical
  processing temperatures, thermal diffusivity, density, shrinkage, cavity
  pressure, flow-ratio limit and indicative $/kg; tool steels Aluminium 7075,
  P20 and H13 with indicative base cost, $/cm², side-action cost, shot life
  and lead time.
* **DFM report** — Markdown download (also included in the Manufacturing Hub
  package).

## The maths

**Draft.** For a face with unit normal **n** and pull direction **d**,

```
draft = 90° − ∠(n, d) = asin(|n · d|)
```

Faces with `|n · d| ≥ cos 45°` are floors/tops (face-on); the rest are side
walls and must have `draft ≥ min_draft` (default 1°, R-020).

**Undercuts.** From each side-wall triangle's centroid a ray is cast in the
release direction (+d for the core side, −d for the cavity side, whichever the
face points toward). If it hits the part again, the face is an undercut.
Ray/triangle tests use Möller–Trumbore, accelerated with a uniform 3-D grid.

**Wall thickness.** From each triangle, five rays in a 45° cone into the
material; thickness is the shortest hit distance (the cone avoids blow-ups on
edge rays).

**Projected area and clamp force.** Triangles are rasterised onto the plane
normal to the pull (union of shadows, not a sum of areas):

```
F = A_projected · p_cavity · (1 + runner allowance)      (1 tonne-force = 9.807 kN)
```

Hand check (unit test): a 40 × 20 mm box, 55 MPa, allowance 1.1 →
800 mm² × 55 MPa × 1.1 = 48.4 kN = **4.93 t**.

**Cooling time.** One-dimensional conduction in a slab of thickness s cooled
from both faces, first Fourier term:

```
t_c = s² / (π² α) · ln[ (8/π²) · (T_melt − T_mold) / (T_eject − T_mold) ]
```

Hand check (unit test): PC, 2 mm, α = 0.13 mm²/s, 300/90/127 °C →
4 / (π² · 0.13) · ln(0.8106 · 210 / 37) = 3.118 · 1.5285 ≈ **4.77 s**; doubling
the wall quadruples it.

**Cycle.** `fill = 0.4 + shot_cm³ / 25`, `pack = max(1, 0.25 · t_c)` (≈ gate
freeze), `open/eject/close = 2.5 + 0.02 · depth_mm` seconds.

**Costs (indicative).**

```
tool  = (base + $/cm² · A_proj + side_actions · $/action) · (1 + 0.65 · (cavities − 1))
press rate = 35 + 0.25 · tonnes   $/h
piece = mass · (1 + runner) · $/kg / (1 − scrap) + rate · cycle / cavities / (1 − scrap)
unit(q) = piece + tool · ⌈q / shot_life⌉ / q
```

## How to use it

1. Pick a **part** (front, temples…) and a **pull direction** (auto picks the
   one with the largest minimum draft, then least undercut, then least depth).
2. Read the draft map, histogram and draft check; switch to the thickness map.
3. Choose **resin**, **tool steel** and **cavities** — these are written to the
   project (`design.manufacturing`) and flow into the Manufacturing Hub.
4. Adjust clamp allowance, scrap, runner fraction or press rate if you have
   better numbers from a molder.
5. Download the **DFM report** and send it with the STEP file when asking
   for a molding quote.

## Engine API

| Method | Input | Output |
|---|---|---|
| `molding.materials` | – | `{ materials, steels }` |
| `molding.parts` | `Project` | `[{ name, color, triangles, volume_cm3, bbox_min, bbox_max, mesh }]` |
| `molding.draft` | `{ project, part?, pull?, min_draft_deg = 1, mesh = true }` | Draft analysis: pull, candidates, per-face draft/class, areas, histogram, min draft, undercut area, coloured mesh |
| `molding.thickness` | `{ project, part?, mesh = true }` | Thickness per face, nominal / p95 / max, thick area, histogram, coloured mesh |
| `molding.analyze` | `{ project, options? }` | Per-part process (clamp, cycle, gate, flow ratio, tool, piece, warnings) + totals |
| `molding.cooling` | `{ wall_mm, material = "PC" }` | `{ cool_s, material }` |

Metrics (feed R-020):

| Key | Iris value (v0.6) |
|---|---|
| `molding.min_draft_deg` | **1.5°** (right lid, pull +Z) — R-020 (≥ 1°) ✓; the two temples carry 7 810 mm² of undercut that needs side actions |
| `molding.clamp_tonnage_t` | **9.98 t** (front: 1 617 mm² × 55 MPa × 1.1) |
| `molding.cycle_time_s` | **28.4 s** (front: fill 0.8 + pack 5.0 + cool 20.0 at a 4.10 mm wall + open/eject 2.7) |

## How Iris uses it

Iris's frame is specified in polycarbonate (P20 tool, one cavity per part).
The CAD Studio frame is drafted (1.5° on every side wall) and cored to a 1.2 mm
pod wall, so R-020 passes; the clamp is set by the front's projected area
(≈ 10 t, a 13 t press). What molding still needs is visible on the page: the
temples, pulled along their length, have 5 065 mm² (right) and 2 744 mm²
(left) of undercut surface — four side actions each — and the front has
thick sections up to ≈ 3.3 mm (nominal 2.4 mm) that should be cored out to
avoid sink. With six P20 tools the Analyse tab totals ≈ 60 k USD of tooling,
so the Manufacturing Hub keeps fused deposition modelling (FDM) printing (≈ 7.0 USD per frame set) at the 1 000-unit
target and finds molding cheaper only above ≈ 10 300 units.

Note: the metric's cycle time samples fewer faces for wall thickness than the
Analyse tab (fast metric pass), so it reads the front's maximum wall as
4.10 mm and gives 28.4 s, while the Analyse tab on the same front finds
3.33 mm and ≈ 20.0 s. Use the tab's number for a quote request.

## Limitations

* No flow simulation: no fill pattern, weld lines, air traps, pressure drop,
  warpage or fibre orientation. The gate is a heuristic (thickest section).
* Cooling uses the 1-D slab formula with the maximum wall; real cycles depend
  on cooling-line layout, mold material and ribs/bosses.
* Undercut detection is per-face centroid ray casting — very small features
  can be missed, and it does not design side actions or lifters.
* Parting line is implied by the pull direction; no parting-surface design,
  shut-offs or ejector layout.
* Cost figures (tool $/cm², press rates, resin $/kg) are rough indicative
  numbers for comparison, not quotes. Real tooling cost depends on the
  toolmaker, region, surface finish and tolerances.

## Graduating to professional tools

* **Autodesk Moldflow**, **SolidWorks Plastics** or Moldex3D for fill, pack,
  cool and warp simulation.
* **FreeCAD** (Part / Draft analysis) and the draft-analysis tools in
  Fusion, SolidWorks or Onshape to add and check draft in CAD.
* Upload the STEP to a molding portal (Protolabs, Xometry, Fictiv) for an
  automated DFM report and a real quote.

## References

* T. A. Osswald, *Understanding Polymer Processing*, Hanser.
* D. V. Rosato, M. G. Rosato, *Injection Molding Handbook*, Springer.
* G. Menges, W. Michaeli, P. Mohren, *How to Make Injection Molds*, Hanser.
* T. Möller, B. Trumbore, "Fast, minimum storage ray/triangle intersection",
  *Journal of Graphics Tools*, 1997.
* Protolabs injection-molding design guidelines —
  <https://www.protolabs.com/resources/design-tips/>.
