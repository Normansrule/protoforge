# Manufacturing Hub

Route: `#/mfg` · Engine: `crates/forge-core/src/mfg/` · UI: `web/src/labs/mfg/`

The Manufacturing Hub answers four questions about the product:

1. **What will one unit cost** at 1, 100, 1 000 or 100 000 units, and where
   does the money go?
2. **Which frame process wins when** — in-house FDM printing, an MJF/SLS
   service, urethane casting or injection molding — and at what quantity does
   molding take over?
3. **Who can make it and how do I order?** A vendor directory (facts only)
   and step-by-step order guides for PCBA, printing and molding.
4. **One button for all the files** — a zip with Gerbers, drill, BOM, CPL,
   the molding DFM report, cost reports, order guides, the verification report
   and the project file.

Every number here is an **indicative parametric estimate — get real quotes**.
No vendor prices are scraped or quoted, and no distributor part numbers are
invented.

## The cost model

Each category splits into a fixed cost spread over the quantity q and a
variable cost per unit:

```
unit(q) = Σ fixed / q + Σ variable(q)
```

| Category | Model (defaults editable in the UI) |
|---|---|
| Electronics (on board) | Σ p₁ · qty, each at its learning-curve price for q · qty parts |
| Display, battery, optics & hardware | same, for off-board parts (the bare-PCB BOM line is replaced by the PCB model) |
| PCB fabrication | `setup / q + area_cm² · $/cm² / panel efficiency` (25 $, 0.02 $/cm², 0.75) |
| PCB assembly | `(setup + stencil + unique parts · loading) / q + joints · $/joint` (50 $, 10 $, 3 $/part, 0.0017 $/SMD joint, 0.02 $/THT joint) |
| Frame | cheapest of the four processes below at that quantity (or the one you force) |
| Final assembly & test | `(assembly minutes + test minutes) / 60 · labour rate` (from the project) |
| Packaging | fixed per unit (2.50 $) |
| Yield loss | `subtotal / yield − subtotal` (95 %) |

**Component price breaks** follow a learning-curve fit to typical
distributor price breaks, anchored at the quantity-1 price p₁ in the project
BOM:

```
p(q) = p₁ · (f + (1 − f) · q^(−b))        f = 0.35 (floor), b = 0.18
```

At q = 1 000 a part costs ≈ 0.35 + 0.65 · 1000^−0.18 ≈ 0.54 × p₁. (Wright's
learning curve, the basis of this form, predicts a constant fractional drop
per doubling of cumulative quantity: 2^−0.18 ≈ 0.88, i.e. an 88 % curve on the
variable part.)

**Frame processes** (per frame set, volume V from the CAD Studio):

```
FDM       = filament g · $/kg + hours · $/h + post-processing minutes · labour
            hours = V / 12 cm³/h + 0.25 (unless the Print lab supplies a time)
MJF/SLS   = V · 0.35 $/cm³ + 20 $ order fee / q
Urethane  = (master 1 500 $ + new silicone mould every 20 shots at 30 %) / q
            + V · 0.25 $/cm³ + 15 min labour
Molding   = Molding Lab piece price + tool · ⌈q / shot life⌉ / q + 300 $ setup / q
```

The **crossover** is found by bisection on log q: the smallest quantity at
which injection molding is cheaper than the cheaper of the two printing
processes.

## How to use it

1. **Cost vs quantity** — edit the assumptions (or reset them), hover the
   stacked chart for the breakdown at any quantity, compare frame processes on
   the log–log chart, read the table view at the target quantity, download
   `cost-vs-quantity.csv`.
2. **Vendors** — filter by category (PCB, parts, printing, molding, CNC);
   each entry lists what the vendor does, where it is and which files it
   consumes. Facts only — no prices, ratings or endorsements.
3. **Order guides** — Markdown walkthroughs generated from the project
   (board size, layer count, finish, part counts, material, tool steel).
4. **Fab package** — preview and download the zip. Files are produced by a
   list of producer functions in the engine (`PRODUCERS`), so other labs can
   add outputs with one line.
5. **Prototype → production** — EVT/DVT/PVT/MP stage checklists, linked to
   the project's milestones; ticks are saved in your browser.

## Engine API

| Method | Input | Output |
|---|---|---|
| `mfg.cost` | `{ project, assumptions? }` | `CostCurve`: quantities, categories, per-unit matrix, totals, frame options per quantity, frame choice, crossover, target quantity, breakdown at target, BOM cost, board stats |
| `mfg.assumptions` | – | default `Assumptions` |
| `mfg.vendors` | – | `[{ name, url, categories, services, consumes, region }]` |
| `mfg.guides` | `Project` | `[{ id, title, markdown }]` (pcba, print, molding) |
| `mfg.stages` | `Project` | `[{ id, title, goal, checklist, milestones }]` |
| `mfg.package` | `Project` | `[{ path, content, encoding }]` — every file of the fabrication zip |
| `mfg.molding_report` | `Project` | Markdown DFM report |
| `mfg.producers` | – | producer names (`pcb`, `molding`, `cost`, `guides`, `verification`, `project`) |

Package layout:

```
README.md
pcb/gerbers/iris-main-{F_Cu,B_Cu,F_Mask,B_Mask,F_Paste,F_Silkscreen,Edge_Cuts}.gbr
pcb/gerbers/iris-main-{PTH,NPTH}.drl
pcb/assembly/iris-main-{BOM,CPL,netlist}.csv, iris-main-assembly-top.svg
pcb/iris-main-FAB-README.md
molding/molding-dfm-report.md
cost/cost-vs-quantity.csv, cost/cost-report.md
guides/order-{pcba,print,molding}.md
verification-report.md
project.json
```

Metrics (feed R-022):

| Key | Iris value |
|---|---|
| `mfg.unit_cost_usd` | **54.58 USD** at 1 000 units (R-022 ≤ 120 USD ✓) — frame by FDM |
| `mfg.bom_cost_usd` | 34.11 USD of parts at 1 000 units (64.12 USD at quantity 1) |
| `mfg.crossover_qty` | ≈ 10 200 units (molding beats printing above) |

## How Iris uses it

At the 1 000-unit target the Iris v0.6 estimate is ≈ 54.6 USD per unit,
dominated by the off-board parts (display module, battery, optics and
hardware ≈ 27.5 USD), final assembly and test (7.5 USD), the FDM frame set
(≈ 7.0 USD) and on-board electronics (≈ 6.6 USD). The drafted CAD frame molds
with ≈ 60 k USD of P20 tooling for its six parts (the temples need side
actions for their undercuts), so molding wins only above ≈ 10 200 units. The
Program & Business Lab adds logistics, warranty, certification and fixtures
to this figure for a landed cost of ≈ 76.7 USD.

## Limitations

* Parametric estimates only: no live distributor pricing, no MOQ/reel
  effects, no attrition, no shipping, duties, tax, certification, NRE for
  test fixtures or firmware.
* The learning-curve price model is one curve for every part; real price
  breaks differ widely (commodity passives vs custom displays).
* Frame costs use the CAD volume; FDM time is a crude volume rate unless the
  Print lab provides a slicer estimate.
* Vendor entries are facts (services, regions, accepted files) — not
  recommendations and not guaranteed current.

## Graduating to professional tools

* Real quote portals: JLCPCB / PCBWay / Eurocircuits / Aisler for boards and
  assembly; Protolabs, Xometry, Fictiv for molding and printing; Craftcloud
  to compare print services.
* BOM tools with live pricing: Octopart, the distributors' BOM managers
  (Digi-Key, Mouser, LCSC).
* DFMA software (Boothroyd Dewhurst DFM Concurrent Costing, aPriori) for
  should-cost models.

## References

* G. Boothroyd, P. Dewhurst, W. Knight, *Product Design for Manufacture and
  Assembly*, 3rd ed., CRC Press, 2010.
* T. P. Wright, "Factors affecting the cost of airplanes", *Journal of the
  Aeronautical Sciences* 3, 1936.
* JLCPCB help: BOM and CPL files — <https://jlcpcb.com/help>.
* Engineering validation stages (EVT/DVT/PVT): e.g. A. "bunnie" Huang,
  *The Hardware Hacker*, No Starch Press, 2017.
