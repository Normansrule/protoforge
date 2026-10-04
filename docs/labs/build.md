# Build Guide

Route: `#/build` · Engine: `crates/forge-core/src/build/` · UI: `web/src/labs/build/`

The Build Guide answers the questions between "the simulations pass" and
"a box on a shelf":

1. **What settings do I start from?** A settings library for fused
   deposition modelling (FDM) 3D printing per filament, resin printing
   (stereolithography, SLA / MSLA), powder-bed fusion (selective laser
   sintering, SLS, and Multi Jet Fusion, MJF), laser cutting, computer
   numerical control (CNC) milling, sheet metal, electronics assembly,
   printed circuit board (PCB) fabrication tiers and injection molding —
   always labelled **typical starting points — tune on your machine**.
2. **What are the numbers?** Calculators for feeds and speeds, sheet-metal
   bends, stencil area ratios, reflow profiles and a PCB design-for-manufacture
   (DFM) check against fab capability tiers.
3. **Which process should make this part?** A transparent weighted
   process-selection wizard.
4. **Where do I send it, and which file?** A vendor directory by category
   with accepted file formats and the Protoforge output to upload.
5. **What certifications does it need?** A compliance planner derived from
   the project's components and interfaces, with a timeline.
6. **How does a prototype become a product?** An engineering validation test
   (EVT) → design validation test (DVT) → production validation test (PVT)
   → mass production roadmap tied to the project milestones.

It extends the Manufacturing Hub (`#/mfg`: cost model, order guides, fab
package) and links to the Print Lab, Molding Lab and PCB Studio rather than
repeating them. Honesty rules: settings are typical windows, never a vendor's
profile; vendor entries are facts only (no prices); compliance costs and
durations are rough orders of magnitude to confirm with a lab; nothing is
legal advice.

## Tabs

| Tab | What you do |
|---|---|
| **Settings** | Pick a process and material → animated illustration (nozzle laying lines with the Print lab's layer height and line width, a UV-cured resin layer and peel, a recoater and laser on a powder bed, a laser kerf with top-view compensation, an end mill throwing chips, a press brake, an injection-molding cycle, a PCB annular ring with drill wander), recommended windows as range bars (the Iris print settings are marked on the FDM bars), facts per material (enclosure, drying, bed surface, first layer, speed), Bambu Lab notes, design rules and common failures with small diagrams, settings sheet download (Markdown / CSV). The laser card adds a **thickness scaler** (known cut → new thickness, with an animated tapered kerf and multi-pass cross-section), a laser-type table (diode / CO₂ / fibre), the safe / never material list, a power × speed **test-grid SVG generator** and a kerf-strip calculator. |
| **Calculators** | Reflow profile (animated board through the oven with a live temperature plot, CSV), stencil area ratio (animated print-and-release plus every Iris pad), PCB fab DFM (tier cards, rule-by-rule table, IPC Class 1/2/3 explained), feeds and speeds (a cutter spinning 1000× slower than the computed RPM with the engaged arc shaded, plus an **animated pocket tool path** whose cycle time comes from the computed feed), sheet-metal bend (animated forming above the flat pattern), resin exposure (Jacobs working curve, animated cure front, staircase-test fit). |
| **Where to send it** | The process-selection wizard: sliders for quantity, material, geometry, size, tolerance, finish, strength and lead time; editable weights; ranked processes with stacked sub-score bars, reasons, "send to" vendor links and which lab produces the file to upload. |
| **Vendors** | Filterable vendor cards (16 categories: PCB fabrication and assembly, parts, 3D printing, CNC, sheet metal, laser cutting, urethane casting, molding, optics, enclosures, batteries, cables, test and certification labs, contract manufacturing, fulfilment) with accepted formats; an animated "Protoforge output → vendor → back to you" flow for the selected vendor. |
| **Compliance** | Market toggles (US, EU, Great Britain, Canada), radio module vs chip-down, detected product features, checklist traced to components / interfaces / requirements (ticks saved in the browser), animated Gantt timeline across EVT/DVT/PVT with a moving "now" cursor, Markdown and CSV checklist downloads. |
| **Roadmap** | Animated journey through five stations; each shows what to build, how many, with which process and vendor category, tests, exit criteria, linked project milestones (`p.milestones`, matched by stage name), compliance items and FMEA risks. |

## The maths

**Feeds and speeds** (`machining.rs`)

```
RPM  = SFM · 12 / (π · D_in)          n = 1000 · Vc / (π · D_mm)
feed = n · z · f_z                     MRR = a_e · a_p · feed
P    = MRR · u                         u = specific cutting energy (W·s/mm³)
RCTF = 1 / √(1 − (1 − 2·a_e/D)²)      for a_e < D/2 (radial chip thinning)
```

Surface speeds and chip loads are typical carbide / high-speed-steel ranges
per material (aluminium 6061, mild steel, stainless, brass, acetal, acrylic,
HDPE, wood); chip load scales linearly with diameter around a 6 mm
reference. Specific energies for metals follow Kalpakjian & Schmid's table;
plastics and wood are flagged as order-of-magnitude estimates. The RPM is
clamped to the spindle's limits with a warning. Hand checks in the tests:
500 SFM on a ½ in cutter = 3819.7 RPM; 100 m/min on 10 mm = 3183.1 RPM;
3820 RPM × 4 × 0.002 in = 30.56 in/min; chip thinning at 10 % engagement =
1/0.6.

**Sheet-metal bend** (`sheet.rs`)

```
BA = θ · (R + K·T)     OSSB = tan(θ/2) · (R + T)     BD = 2·OSSB − BA
flat = L1 + L2 − BD    (flanges to the virtual sharp)
DIN 6935: K = ½ · (0.65 + ½·log₁₀(R/T)), K = 0.5 for R/T ≥ 5
```

Test: T = R = 2 mm, K = 0.44, 90° → BA = 4.5239, BD = 3.4761 mm.

**Stencil** (`reflow.rs`, IPC-7525): area ratio = L·W / (2·(L+W)·t) ≥ 0.66,
aspect ratio W/t ≥ 1.5, circle d/(4t). Every 1:1 paste aperture on the Iris
board is checked; windowed thermal-pad apertures are listed separately.

**Reflow oven** (`reflow.rs`): each thermocouple location is a lumped mass
heated by convection, `dT/dt = (T_air(x(t)) − T)/τ`, integrated exactly per
0.25 s step with `x(t) = v·t`; zone air blends linearly over 60 mm between
zones. Two probes — a small part (τ ≈ 30 s) and the heaviest part
(τ ≈ 42 s) — must both meet the windows:

| Window | SAC305 (lead-free) | Sn63Pb37 |
|---|---|---|
| Heating ramp | ≤ 3 °C/s | ≤ 3 °C/s |
| Soak | 150–200 °C for 60–120 s | 100–150 °C for 60–120 s |
| Time above liquidus | 45–90 s above 217 °C | 45–90 s above 183 °C |
| Peak | 235–250 °C | 205–225 °C |
| Cooling | ≤ 6 °C/s | ≤ 6 °C/s |
| 25 °C → peak | ≤ 480 s | ≤ 360 s |

These are consistent with IPC/JEDEC J-STD-020 and typical paste datasheets
(the paste's datasheet wins). The default recipes (SAC305: 110/175/190/240/250/90 °C
in 450 mm zones at 52 cm/min; Sn63Pb37: 100/145/160/180/232/80 °C at
48 cm/min) pass on both probes; tests check that a fast conveyor fails
time-above-liquidus and a 300 °C zone fails the peak, and that a single long
zone reproduces the analytic step response `25 + 175·(1 − e⁻¹)` after τ.

**Resin exposure** (`resin.rs`) — Jacobs' working curve

```
C_d = D_p · ln(E / E_c)      E = I · t
t   = (E_c / I) · exp(C_d / D_p)        C_d = overcure × layer height
bottom layers: m × E → C_d + D_p · ln m
staircase fit: least squares of C_d on ln(I·t) → slope D_p, E_c = exp(−intercept / slope)
```

Defaults (D_p = 0.12 mm, E_c = 6 mJ/cm², I = 3 mW/cm²) are illustrative,
not a real resin. Tests: D_p = 0.1, E_c = 10, E = 10·e → C_d = 0.1 mm; the
defaults give C_d = 0.075 mm, E = 6·e^0.625 = 11.2 mJ/cm², t = 3.74 s; a
synthetic staircase recovers D_p and E_c exactly.

**Laser thickness scaling** (`laser.rs`): the line energy `P/v` needed to cut
through grows roughly with thickness, so `v = v₀·t₀/t` at the same power or
`P = P₀·t/t₀` at the same speed; below the slowest clean speed the cut is
split into passes. The result centres a narrow test grid. Test: 3 mm at
70 %, 12 mm/s → 6 mm needs 6 mm/s or 140 % (impossible in one pass); 15 mm →
2.4 mm/s is below 3 mm/s → 2 passes at 4.8 mm/s.

**CNC tool path** (web): a raster pocket (stepover = a_e, tool centre
D/2 from the walls) plus a finishing contour; path length ÷ computed feed =
minutes per layer, × ⌈pocket depth / a_p⌉ layers.

**Process wizard** (`wizard.rs`): `score = 100 · Σ wᵢ·sᵢ / Σ wᵢ`; relative
cost `setup/q + unit` (unitless, palm-sized part), `s_cost = 1 −
log₁₀(cost/cheapest)/2`; quantity fit falls off over 1.5 decades outside the
sweet spot; material or geometry mismatch scores 0. Sanity tests: one loose
plastic prototype → FDM; 100 000 plastic housings → injection molding; ten
flat metal plates → laser cutting; five tight-tolerance metal blocks → CNC;
fifty bent brackets → sheet metal.

**PCB DFM** (`pcbcap.rs`): reads the routed layout from the PCB Studio
(`crate::pcb::layout`, read-only) — finest track, via drill and annular ring,
plated holes, pad-to-pad gaps (solder-mask web), vias inside pads — plus the
design rules (clearance, layers, thickness) and compares them with four
generic tiers (basic prototype, standard, advanced, high-density interconnect
— typical values, check your vendor). Vias in large exposed pads (thermal
vias) are a warning, not a violation. The IPC performance classes
(IPC-6011 / IPC-6012, IPC-A-600 / IPC-A-610: Class 1 general electronic
products, Class 2 dedicated service electronic products, Class 3 high
reliability electronic products) are explained separately — a class is an
inspection and acceptance level ordered from the fab, not a capability tier.

**Compliance** (`compliance.rs`): features are derived from the project — a
radio from `BLE`/Wi-Fi interfaces or radio parts, a lithium cell from the BOM
(or the battery chemistry when the product has electronics), a light source
from display / LED parts, eyewear and skin contact from the product
description and skin-temperature requirements, polycarbonate from the frame /
molding material. Items for Iris (US + EU) include FCC Part 15 Subparts B and
C, FCC RF exposure, the Radio Equipment Directive's EN 300 328, EN 301 489,
EN 62479 and cybersecurity articles, the CE declaration, RoHS, REACH, WEEE,
the EU responsible economic operator, the EU Batteries Regulation,
IEC 62368-1, IEC 62133-2, UN 38.3, IEC 62471, ISO 12870, ISO 12312-1
(conditional: only for tinted / sun lenses; sunglasses are personal
protective equipment under Regulation (EU) 2016/425), lens impact
(21 CFR 801.410), skin-contact screening (ISO 10993 concepts — consult a
lab; may not be required for consumer eyewear), Bluetooth SIG listing,
charging-interface notes, California Proposition 65 and labels / markings.
Each item carries why it applies, the traced components / interfaces /
requirements, the stage, a route and indicative cost / time bands. Every
item is phrased as commonly applicable — confirm with an accredited test lab.

## Metrics

| Key | Meaning | Iris value (v0.6) |
|---|---|---|
| `build.compliance_items` | applicable compliance items for `design.extra.markets` (default US + EU) | 25 (18 mandatory; indicative $38k–$143k) |
| `build.pcb_capability_violations` | DFM violations against `design.extra.pcb_tier` (default `standard`) | **0** — R-034 (1 warning: thermal vias in the exposed pads) |
| `build.stencil_min_area_ratio` | smallest IPC-7525 area ratio at `design.extra.stencil_thickness_mm` (default 0.12 mm; Iris stores 0.10 mm) | **0.80** — R-035 (≥ 0.66). At the 0.12 mm default the QFN-48 0.8 × 0.2 mm pads gave 0.667, right on the limit, which is why Iris v0.5 moved to a 0.10 mm stencil |

## API

| Method | Input → output |
|---|---|
| `build.library` | `{project?}` → processes with materials (range bars, facts), rules, failures, tips, links, project checks |
| `build.settings_sheet` | `{project?, process, material?, format: "md"\|"csv"}` → `{filename, content}` |
| `build.laser_materials` | → safe / caution / never list |
| `build.laser_grid` | rows, cols, power and speed ranges, cell, mode → `{svg, cells, csv}` |
| `build.laser_scale` | known thickness / power / speed, target thickness, slowest clean speed → speed at same power, power at same speed, passes, grid window, steps |
| `build.laser_types` | → diode / CO₂ / fibre: wavelength, what each cuts and cannot |
| `build.resin_exposure` | `{dp_mm, ec_mj_cm2, irradiance_mw_cm2, layer_mm, cure_ratio, bottom_multiple, points?}` → dose, time, bottom time, curve, layer table, fit |
| `build.kerf` | `{nominal_mm, measured_mm, slices}` → `{kerf_mm, offset_mm}` |
| `build.machining_materials`, `build.feeds_speeds` | cutter + cut + spindle → RPM, feed, chip thinning, MRR, power, torque, steps, warnings |
| `build.sheet_materials`, `build.bend` | T, R, θ, K?, flanges → BA, OSSB, BD, flat length, checks |
| `build.reflow_alloys`, `build.reflow` | alloy, zones, conveyor, zone length, τ → time series, stats, pass/fail checks |
| `build.stencil`, `build.stencil_check` | aperture → area / aspect ratio; `{project, thickness_mm}` → every pad |
| `build.pcb_classes` | → IPC Class 1 / 2 / 3 descriptions |
| `build.pcb_tiers`, `build.pcb_dfm` | `{project, tier}` → rule table, violations, warnings, cheapest passing tier |
| `build.wizard`, `build.wizard_processes` | part + weights → ranked processes with sub-scores and reasons |
| `build.vendors` | → categories and the extended vendor directory |
| `build.compliance`, `build.compliance_md` | `{project, options: {markets, radio_module}}` → plan (items, traces, phases, timeline) / Markdown checklist |
| `build.roadmap` | `{project}` → five stages with builds, tests, exit criteria, milestones, compliance, risks |
| `build.references` | → reference list |

## References

- E. Oberg et al., *Machinery's Handbook* (Industrial Press).
- S. Kalpakjian, S. R. Schmid, *Manufacturing Engineering and Technology* (Pearson) — specific energy in cutting.
- Sandvik Coromant, Harvey Tool and Kennametal published milling formulas and chip-thinning guidance (vendor technical guides).
- IPC-7525 *Stencil design guidelines*; IPC/JEDEC J-STD-020 (moisture/reflow sensitivity classification); IPC-A-610 *Acceptability of electronic assemblies*; IPC-2221 *Generic standard on printed board design*; IPC-6012 *Qualification and performance specification for rigid printed boards*; IPC-A-600 *Acceptability of printed boards*.
- ISO/ASTM 52900 *Additive manufacturing — general principles and vocabulary*.
- I. Gibson, D. Rosen, B. Stucker, *Additive Manufacturing Technologies* (3rd ed., Springer, 2021).
- P. F. Jacobs, *Rapid Prototyping & Manufacturing: Fundamentals of Stereolithography* (Society of Manufacturing Engineers, 1992).
- W. M. Steen, J. Mazumder, *Laser Material Processing* (4th ed., Springer, 2010).
- D. V. Rosato, M. G. Rosato, *Injection Molding Handbook* (3rd ed., 2000).
- DIN 6935 *Cold bending of flat rolled steel products*.
- G. Boothroyd, P. Dewhurst, W. Knight, *Product Design for Manufacture and Assembly* (3rd ed., 2010); M. F. Ashby, *Materials Selection in Mechanical Design*.
- F. P. Incropera, D. P. DeWitt, *Fundamentals of Heat and Mass Transfer* (lumped capacitance).
- Protolabs and Xometry design guides for CNC machining, sheet metal, 3D printing and injection molding (vendor design guides).
- Prusa Knowledge Base (<https://help.prusa3d.com>), Bambu Lab Wiki (<https://wiki.bambulab.com>), UltiMaker material guides — vendor material guides.
- FCC 47 CFR Part 15 (<https://www.ecfr.gov/current/title-47/chapter-I/subchapter-A/part-15>); EU Radio Equipment Directive 2014/53/EU (<https://eur-lex.europa.eu/eli/dir/2014/53/oj>); RoHS 2011/65/EU (<https://eur-lex.europa.eu/eli/dir/2011/65/oj>); REACH Regulation (EC) No 1907/2006; WEEE Directive 2012/19/EU; Batteries Regulation (EU) 2023/1542; PPE Regulation (EU) 2016/425.
- UN *Manual of Tests and Criteria*, section 38.3; IEC 62368-1; IEC 62133-2; IEC 62471; ISO 12870; ISO 12312-1; Bluetooth SIG Qualification Program (<https://www.bluetooth.com>).

## Limitations

- Settings are generic windows; they do not know your printer, resin, laser
  tube or spindle. Calibrate.
- The oven model is lumped (one time constant per probe, no radiation or
  board conduction); real profiles need thermocouples.
- Wizard capability data and relative costs are indicative; use the
  Manufacturing Hub and quotes for decisions.
- Compliance scope, editions and dates change; the planner is a starting
  checklist for a conversation with an accredited lab, not legal advice.
- The resin calculator needs the resin's D_p and E_c (from its maker or a
  staircase test); the defaults are illustrative, and light bleed and
  oxygen inhibition are not modelled.
- Laser thickness scaling is a first-order energy balance; focus depth,
  taper and charring break it for thick sheet — always run the grid.
- The pocket tool path is a simple raster with no ramp-in, rest machining or
  acceleration limits; real CAM cycle times are longer.
