# CAD Studio (`#/cad`, engine module `cad`)

A small, dependency-free parametric modeller written in Rust and compiled to
WebAssembly. It generates the **Iris smart-glasses frame** from the
boxing-system sizes in `design.frame`, checks every part is a watertight
solid, weighs it, and exports STL / OBJ / 3MF. A second generator builds a
**parametric electronics enclosure** for anything else you make.

## What it does

| Tab | What you get |
|---|---|
| Frame designer | 3D assembly (explode, fold the temples, x-ray, wireframe, edges, PCB and lens toggles, view presets, auto-rotate) or the print-bed layout; sliders for every `FrameParams` field grouped as Fit / Front / Temples / Pods / Molding & material. Every change is written to the project, so requirements R-001 (mass) and R-019 (PCB fit), the Print Lab and the Molding Lab update live. |
| Face fit | Face width + PD (+ optional prescription) → recommended `A □ DBL – temple`, frame PD, decentration, Prentice prism, and whether the pupil stays inside the display eyebox; one click applies it. To-scale boxing diagram. |
| Mass & export | Mass budget (frame parts + components vs the 50 g limit), per-part mass properties (volume, area, mass, centroid, bounding box, triangles, shell count, mesh check), downloads: STL per part (print or as-worn orientation), all parts as a zip, OBJ with part groups, and a 3MF print plate. |
| Enclosure generator | Box + lid with corner radius, bottom fillet, optional draft, M3 screw bosses, PCB standoffs and rectangular cut-outs (USB, LED, vents) on any wall; presets (dev-board case, Iris PCB bench case, sensor puck); STL / zip export; "Send to Print Lab" stores the parameters in `design.extra.enclosure`. |
| How it works | Animated ear-clipping of the real Iris front outline, an interactive draft cross-section, and explainers on B-rep vs mesh, 2-manifold meshes and sweeps. |

## Engine API

| Method | Input | Output |
|---|---|---|
| `cad.parts` | `{project, orientation: "assembly" \| "print", display?}` | parts (name, label, colour, explode offset, group, f32 mesh, mass properties, mesh check, shell count), display-only items (demo lenses, combiner, PCB board), `layout` (derived dimensions), totals, frame / component / wearable mass, warnings |
| `cad.stl` | `{project, part, binary, orientation}` or `{enclosure, part: "body" \| "lid"}` | `{filename, base64 \| text, bytes, triangles}` |
| `cad.obj` | `{project, orientation}` | OBJ text with one `o`/`g` group per part |
| `cad.3mf` | `{project}` | the three XML files of a 3MF package, parts laid out on a 256 mm plate (the page zips them with JSZip) |
| `cad.mass` | `{project}` | per-part and total mass properties, components, wearable mass, layout |
| `cad.layout` | `{project}` | derived dimensions only (cheap) |
| `cad.fit` | `{face_width_mm, pd_mm, sphere_d?, frame?, eyebox_mm?}` | recommendation + `patch` for `design.frame` |
| `cad.enclosure` | `{params}` | body + lid meshes, mass, interior, PCB placement, warnings |
| `cad.enclosure_defaults` | – | default `EnclosureParams` |
| `cad.triangulate` | `{outer, holes}` or `{project}` | points, loop sizes, triangles in clipping order |
| `cad.validate` | `{mesh, density_g_cm3?}` | 2-manifold check + mass properties of any mesh |

Rust entry points used by other modules: `cad::frame_parts(p)` (assembly
coordinates, kept stable), `cad::frame_parts_print(p)` (print / mold
orientation, pull = +Z), `cad::frame_pull_directions(p)` and
`cad::frame_geom(p)` (cached full geometry).

**Coordinates.** Assembly: X = wearer's right (display and electronics
side), Y = forward (the wearer looks along +Y; temples run towards −Y),
Z = up; origin at the bridge centre on the front face. Print frame: each part
lies on its largest face, centred on X/Y, min Z = 0, and **+Z is the mold
pull direction** — every side wall is drafted relative to it.

### Metrics

| Key | Meaning | Iris (defaults) |
|---|---|---|
| `cad.frame_mass_g` | all six printed/molded parts, divergence-theorem volume × density | **31.4 g** (PETG 1.27 g/cm³) |
| `cad.pod_inner_length_mm` | longest board of `pcb.height_mm` that lies flat on the electronics-pod floor (draft and corner radii included) | **59.5 mm** → PCB margin 1.52 mm (R-019 ≥ 1 mm ✓) |
| `cad.pod_inner_width_mm` | pod depth, floor to lid | 9.8 mm |
| `cad.pod_inner_height_mm` | cavity height at the floor | 10.5 mm |
| `cad.frame_volume_cm3` | material volume | 24.7 cm³ |

With 16.7 g of components the wearable total is **48.2 g** (R-001 ≤ 50 g ✓,
1.8 g margin). The frame is slightly above the 15–28 g band typical of
printed frames: the 5 mm × 5 mm rim front (11.5 g) and the housing for the
18 mm collimating lens (4.2 g) dominate. A 4 mm rim or a 4 mm front, or ABS /
PA12, each bring it back into the band.

## The maths

* **Superellipse lenses** — |x/a|ⁿ + |y/b|ⁿ = 1 (Lamé curve), area
  4ab·Γ(1+1/n)²/Γ(1+2/n); resampled to equal arc length.
* **Boxing system (ISO 8624)** — frame PD = A + DBL; decentration per eye =
  (A + DBL − PD)/2; Prentice's rule P = c[cm]·F[D].
* **Offsets** — vertex-wise mitre offsets (vertex count preserved, used for
  draft) and general offsets whose raw curves are cleaned by the positive
  winding-number rule (Chen & McMains); region booleans (union,
  intersection, difference) use the same rule.
* **Triangulation** — ear clipping with hole bridging through mutually
  visible vertices (Eberly); n + 2h − 2 triangles.
* **Extrusion with draft** — a region lofted between levels, each level the
  base outline offset by −z·tan α. Stacked shells share the same reference,
  so walls continue exactly. The pod pocket is sized so the wall equals
  `wall_mm` at the rim (thicker at the floor).
* **Sweeps** — profile placed in rotation-minimising frames (double
  reflection, Wang et al. 2008) with mitre scaling at bends.
* **Validation** — every undirected edge used exactly twice, in opposite
  directions; positive signed volume.
* **Mass properties** — V = Σ v0·(v1×v2)/6, centroid = Σ Vₜ(v0+v1+v2)/4 / V
  (Zhang & Chen 2001), area = Σ |e1×e2|/2, mass = V·ρ.

## How the Iris frame is built

* **Front** — rim outline = each lens offset by the rim width, with an
  endpiece bulge at hinge height, joined by a bridge with a nose arch; lens
  openings are holes; upper and lower hinge knuckles and two nose pads are
  stacked on the rear face.
* **Temples** — side profile = forward middle knuckle + pod (rounded
  rectangle) + arm band swept along a centre line (straight, 22 mm-radius
  bend through 38° behind the ear, tapering tail). Three stacked slabs: a
  solid floor, the profile with the pocket, and the pod walls. The right
  (electronics) pod has a 9.4 mm USB-C slot in its lower wall; the left pod
  holds the battery.
* **Lids** — plate + locating lip; the right lid carries a tongue that
  closes the USB slot above the receptacle.
* **Optics housing** — tray above the right lens sized from
  `design.optics` (lens diameter, display-to-lens distance), open towards
  the combiner.
* **Placement** — pantoscopic tilt rotates the front about the hinge line.

## Limitations

* Meshes, not B-reps: no exact fillets or feature history; the bottom
  fillet of the enclosure is a stepped loft.
* Parts are stacks of drafted prisms that touch on flat faces rather than
  one fused shell (volume is exact; slicers union them). Hinge pin holes
  are not modelled (drill them, or use a spring-hinge insert — the BOM has
  one); with pantoscopic tilt the front knuckles meet the temple knuckle at
  the tilt angle.
* The pod usable length assumes the board lies flat on the pocket floor.
* The fit helper uses population-style rules of thumb, not a fitting by an
  optician.
* Ear clipping gives thin triangles; production kernels use constrained
  Delaunay triangulation.

## Graduating to professional tools

Use the numbers from *Mass & export* to rebuild the frame with feature
history in **Onshape** (browser, Parasolid kernel), **FreeCAD**, or code-CAD
with **build123d** / **CadQuery** (Python, Open CASCADE) or **OpenSCAD**
(CSG). Keep the STL/3MF exports for slicing, and move to proper
B-rep STEP files for injection-mold tooling quotes.

## References

* M. de Berg, O. Cheong, M. van Kreveld, M. Overmars, *Computational Geometry: Algorithms and Applications*, 3rd ed., Springer, 2008.
* D. Eberly, "Triangulation by Ear Clipping", Geometric Tools — https://www.geometrictools.com/Documentation/TriangulationByEarClipping.pdf
* X. Chen, S. McMains, "Polygon Offsetting by Computing Winding Numbers", ASME IDETC/CIE 2005.
* C. Zhang, T. Chen, "Efficient feature extraction for 2D/3D objects in mesh representation", IEEE ICIP 2001.
* W. Wang, B. Jüttler, D. Zheng, Y. Liu, "Computation of rotation minimizing frames", ACM Transactions on Graphics 27(1), 2008.
* M. Botsch, L. Kobbelt, M. Pauly, P. Alliez, B. Lévy, *Polygon Mesh Processing*, A K Peters, 2010.
* ISO 8624:2020, *Ophthalmic optics — Spectacle frames — Measuring system and vocabulary*.
* C. Fowler, K. Latham Petre, *Spectacle Lenses: Theory and Practice*, Butterworth-Heinemann, 2001.
* 3MF Consortium, *3D Manufacturing Format Core Specification* v1.3.
* I. Gibson, D. Rosen, B. Stucker, *Additive Manufacturing Technologies*, 2nd ed., Springer, 2015 (design for AM).
