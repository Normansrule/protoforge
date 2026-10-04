# Program & Business Lab (`#/program`, engine module `program`)

A product is not finished when the simulations pass: someone has to plan the
work, commit to a date, keep the budget honest, price the thing and decide
when it is ready to build. The Program & Business Lab turns the **Iris smart
glasses** — and any other Protoforge project — into a program you could run
and a business you could launch, entirely in the browser: a work breakdown
structure (WBS) derived from the project, the critical path method (CPM)
with animated Gantt and network diagrams, a seeded Monte Carlo schedule-risk
analysis, earned value management (EVM), unit economics with net present
value (NPV), and technology / manufacturing readiness levels (TRL / MRL)
with the systems-engineering review gates tied to live requirement status.

## What it does

| Tab | What you get |
|---|---|
| Schedule | The Iris WBS (46 activities and milestones) **derived from the project**: milestones M-0 … M-5, the Build Guide roadmap's EVT / DVT / PVT builds and quantities, the compliance plan's certification work packages (grouped by stage and category, with its indicative fees and weeks) and prototype builds priced by the Manufacturing Hub. CPM forward / backward pass with total and free float. An **animated Gantt chart** (bars grow as a cursor sweeps the plan; phase summary bars; critical path in orange with ◆; hatched total float; dependency links; click a bar to see and edit its estimate) and an **activity-on-node network diagram** whose forward pass, backward pass and critical-path highlight play node by node with the arithmetic spelled out. Stage spans checked against the roadmap's typical weeks; full-time-equivalent load per owner; an editable task table (three-point estimates, predecessors with lags such as `T05+2`, owners, FTE, expenses) saved in `design.extra.program`; start date, blended labour rate and frame tooling strategy. Downloads: schedule CSV, Mermaid `gantt`, tasks JSON. |
| Schedule risk | Beta-PERT or triangular sampling of every activity, 200–10 000 seeded iterations of the full network: an **animated completion histogram** filling up sample by sample, the S-curve with P50 / P80, the probability of hitting the deterministic plan, classic PERT alongside (to show merge bias), milestone P50 / P80 dates, a **criticality strip** (which tasks were critical in each of the first 60 sampled runs), criticality index with Spearman rank correlation, and a deterministic tornado. Download: samples CSV. |
| Earned value | The planned value (PV) curve from the baseline budget; a scenario (trouble onset, progress rate, cost per earned dollar, four presets) produces earned value (EV) and actual cost (AC); **sweep the status date** to watch the curves, CPI, SPI and earned-schedule SPI(t) evolve; three estimate-at-completion (EAC) formulas compared with where the scenario really ends; to-complete performance index (TCPI); work-package progress bars. Download: PV / EV / AC CSV. |
| Unit economics | Landed unit cost at the target quantity (the Manufacturing Hub's categories plus logistics, a warranty / returns reserve, and certification and fixtures amortised over the lifetime volume); an animated **price waterfall** from retail price to gross profit; break-even chart; quarterly **cash-flow J-curve** with NPV, internal rate of return (IRR), payback and peak cash need; an NPV sensitivity tornado over eleven assumptions. Every assumption is a slider or field labelled "assumption — edit". Downloads: cash-flow CSV, business case (Markdown). |
| Readiness & reviews | TRL (1–9) and MRL (1–10) per subsystem **suggested from evidence** (requirements allocated, verification method, metric bound, heritage, highest build demonstrated), an animated radar chart, climbing ladders that say why each level stops where it does, evidence editors and assessor overrides; the five review gates — system requirements review (SRR), preliminary design review (PDR), critical design review (CDR), test readiness review (TRR), production readiness review (PRR) — placed on the schedule, each with entry / exit checklists: criteria that can be checked from the project are ticked automatically from **live requirement status** (with requirement chips linking to the Systems Studio), the rest are manual check marks saved with the project. Download: readiness report (Markdown). |
| How it works | The CPM passes animated on a six-activity network you can check by hand (with its table of ES / EF / LS / LF / floats), a PERT vs triangular distribution explorer, an NPV / IRR calculator, the TRL and MRL scales, how the Iris plan is derived, tools to graduate to and references. |

## Engine API

| Method | Input | Output |
|---|---|---|
| `program.config` | `{project, tasks?}` | resolved `design.extra.program` (defaults filled), the active task list, the project-default tasks, whether the plan was edited, default readiness evidence, build stages |
| `program.schedule` | `{project, tasks?, start_date?}` | per task: three-point estimate, ES / EF / LS / LF, total and free float, critical flag, topological level, predecessors (with lag) and successors, budget, calendar dates, source; finish (weeks, date), driving critical path, BAC (labour + expenses), stage checks vs the roadmap, owner load, CSV, Mermaid, notes |
| `program.cpm` | `{tasks, start_date}` (default: the six-activity textbook network) | the same schedule for any network, no project needed |
| `program.risk` | `{project, tasks?, iterations?, seed?, dist?: pert \| triangular}` | every iteration's finish, histogram, S-curve, mean / sd / P10 / P50 / P80 / P90, deterministic finish and its probability, classic PERT mean / sd / P80 and path, per-task criticality / Spearman correlation / cruciality, tornado, milestone P50 / P80, first 60 iterations' critical sets |
| `program.evm` | `{project, tasks?, scenario: {status_week, onset_week, progress_rate, cost_factor}}` | PV / EV / AC curves and forecasts, CPI / SPI / SPI(t) over time, indices at the status date (CV, SV, CPI, SPI, EAC ×3, ETC, VAC, TCPI), earned schedule, IEAC(t), the scenario's true finish and cost, per-task planned / earned % |
| `program.economics` | `{project, tasks?, economics?}` | landed-cost lines, price waterfall, outcome (net price, landed, variable, GM %, contribution, fixed, break-even, lifetime units, NPV, IRR, payback, peak cash, quarterly cash flow), break-even curve, NPV / GM / break-even tornado, resolved inputs, frame process and molding crossover, notes |
| `program.readiness` | `{project, results?: [{id, status}]}` (live results from `systems.evaluate`; computed when absent) | per subsystem: heritage, stage, suggested and final TRL / MRL, both ladders with reasons, allocated requirements with method / metric / status, warnings; the five reviews with criteria (auto / manual, met, detail, requirement ids with status, route); program summary |
| `program.summary` | `{project, tasks?}` | CPM / P50 / P80 weeks, BAC, launch week, GM, break-even, NPV, landed cost, net price, min TRL (and which subsystems), min MRL, milestone weeks, compliance item count, critical path |
| `program.pert` | `{a, m, b}` | PERT mean / variance / sd, triangular mean / sd, α, β, both probability density curves |
| `program.npv` | `{cashflows, rate}` | NPV and IRR (per period) |

## The maths

**Critical path method** (Kelley & Walker 1959). Finish-to-start links with
lag ℓ: forward pass `ES_j = max_i (EF_i + ℓ_ij)`, `EF = ES + d`; backward
pass `LF_i = min_j (LS_j − ℓ_ij)` (the project finish T for end activities),
`LS = LF − d`; total float `TF = LS − ES`; free float
`FF_i = min_j (ES_j − ℓ_ij) − EF_i`. Zero total float is the critical path;
the *driving path* is traced back from the latest finish through the
predecessor whose `EF + ℓ` equals the successor's ES. Networks are
topologically sorted (Kahn) and cycles and unknown predecessors are reported.

**Three-point estimates and Monte Carlo** (Malcolm et al. 1959; Hulett
2009). PERT: `tₑ = (a + 4m + b)/6`, `σ = (b − a)/6`. Sampling uses the
beta-PERT distribution `α = 1 + 4(m − a)/(b − a)`,
`β = 1 + 4(b − m)/(b − a)` (a per-task inverse-CDF table of the beta
density, checked against a Marsaglia–Tsang Gamma-ratio sampler) or the
triangular inverse CDF; each iteration re-runs both CPM passes, so the
criticality index (share of runs in which a task has zero float) comes for
free. Sensitivity is the Spearman rank correlation between each duration
and the finish; cruciality = criticality × correlation. Classic PERT (sum
of tₑ and σ² along the mean-duration critical path, normal P80 = μ + 0.8416σ)
is shown beside the simulation to expose **merge bias**.

**Earned value** (Fleming & Koppelman; PMBOK). `PV(t) = Σ Bᵢ·clamp((t − ESᵢ)/dᵢ, 0, 1)`
with each task budget `B = FTE × weeks × 40 h × rate + expenses`. Scenario:
`EV(t) = PV(τ(t))`, `τ = t₀ + r (t − t₀)` after the onset t₀, and AC grows k
times faster than EV after t₀. `CPI = EV/AC`, `SPI = EV/PV`,
`EAC₁ = BAC/CPI`, `EAC₂ = AC + (BAC − EV)`, `EAC₃ = AC + (BAC − EV)/(CPI·SPI)`,
`TCPI = (BAC − EV)/(BAC − AC)`. Earned schedule (Lipke 2003): `PV(ES) = EV`,
`SPI(t) = ES/AT`, `IEAC(t) = PD/SPI(t)`.

**Unit economics** (Ulrich & Eppinger, product development economics).
Net revenue per unit `P [s(1 − f) + (1 − s)(1 − m)]` (s direct share, f
payment fees, m retailer margin). Landed cost = Manufacturing Hub unit cost
at the target quantity + logistics + warranty reserve
`w (unit + logistics + handling)` + (certification + fixtures) / lifetime
units. `GM = (net − landed)/net`. Break-even `Q* = F/(net − variable − opex·net)`
with F the development budget (BAC — it already contains certification fees,
fixtures and prototype builds, so nothing is counted twice). Cash flow is
built week by week (development spend following the PV curve, stretched to
the launch week; sales at each year's volume and that volume's unit cost)
and summed into quarters; `NPV = Σ CFₜ/(1 + r_q)ᵗ` with
`r_q = (1 + r)^¼ − 1`; IRR by bisection, annualised `(1 + IRR_q)⁴ − 1`.
When the plan buys a hard mold up front, the per-unit mold amortisation is
removed from the Manufacturing Hub's frame line.

**Readiness ladders** (this lab's reading of the DoD / NASA definitions — an
assessor decides). TRL: 1 always; 2 functions described; 3 at least one
allocated requirement bound to a lab metric (analytical proof of concept);
4 a breadboard, or commercial off-the-shelf (COTS) parts whose evaluation
kits validate them on the bench; 5 a brassboard in the real form factor; 6 an
EVT prototype **and** at least one requirement verified by test or
demonstration; 7 DVT units in real use; 8 PVT and every requirement verified
beyond analysis; 9 production. MRL: 2 a lab or BOM describes how it is made;
3 the cost model covers it (software: the toolchain and the programming step
of final test); 4 manufacturability / cost requirements with metrics, or a
lab-built part; 5–9 brassboard, EVT, DVT, PVT pilot line, low-rate
production; 10 by override only.

## Iris tie-in and metrics

`metrics()` derives the plan, runs CPM, a 2 000-iteration seeded beta-PERT
Monte Carlo, the business case and the readiness ladders, memoised on the
whole project (the plan reads milestones, roadmap, compliance, cost model
and requirements); it runs in the verification worker (≈ 20 ms in WebAssembly with the Manufacturing Hub's caches warm). Values for Iris
v0.6:

| Key | Meaning | Iris (v0.6) |
|---|---|---|
| `program.p80_weeks` | 80th-percentile weeks from start to PVT exit (M-5) | **50.9 weeks** (plan 45.5, P50 49.3; only ≈ 3 % of runs meet the most-likely plan; classic PERT P80 49.2) — R-051 (≤ 52 weeks), a marginal pass |
| `program.cpm_weeks` | critical-path weeks with most-likely durations | 45.5 weeks |
| `program.gross_margin_pct` | gross margin at the stated retail price and target quantity | **60.7 %** ($249 retail → $194.84 net; landed $76.65 = Manufacturing Hub $54.58 + logistics $6 + warranty $3.63 + certification $10.68 + fixtures $1.76) — R-052 (≥ 50 %) |
| `program.breakeven_units` | units to recover the development budget | **4 341 units** ($440k budget ÷ $101.4 contribution) |
| `program.npv_usd` | NPV of development + three sales years at 12 % | ≈ $226k (IRR ≈ 35 %, launch at P80, peak cash need ≈ $438k, payback ≈ week 163) |
| `program.bac_usd` | development budget at completion | ≈ $440k |
| `program.min_trl` | lowest suggested TRL | **3** — optics, sensing, the IRIS-DSP block, PCB, frame, firmware and manufacturing have analysis-only evidence (COTS power, compute and audio sit at 4) — R-053 (≥ 6, a goal) stays open: readiness rises with evidence (builds, tests), not with a design parameter |
| `program.min_mrl` | lowest suggested MRL | 2 — the optional IRIS-DSP co-processor has no costed fabrication route in the Manufacturing Hub (PCB, frame and manufacturing are at 4, the rest at 3) |

What the plan shows about Iris: the critical path runs through the trade
studies and requirements, firmware on development kits, the EVT build and
tests, the hinge flex cables, **battery certification** (IEC 62133-2 / UN 38.3
supplier documents) at DVT and the **radio certification** package at PVT
(the bare BLE system-on-chip needs its own FCC / RED radio approvals — a
pre-certified module would shorten it). The proof-of-concept stage (13 weeks)
is longer than the roadmap's typical 4–12. The roadmap plans a hardened steel
mold at PVT, but the Manufacturing Hub costs the frame by in-house FDM at the
1 000-unit target (molding pays off above ≈ 10 200 units) — the default plan
follows the cost model and the switch "frame tooling: hard steel mold" plans
the $60k tool instead.

## Limitations

* Durations, rates, prices, volumes and fees are assumptions; certification
  fees and weeks are the compliance plan's indicative ranges, build costs the
  Manufacturing Hub's indicative model — none are quotes.
* Finish-to-start links only (no start-to-start / finish-to-finish), no
  calendars or holidays, no resource levelling (the owner-load chart shows
  over-allocation but does not fix it).
* The Monte Carlo samples durations independently; real risks are
  correlated, so the percentiles are optimistic. No discrete risk events yet.
* The EVM tab simulates a scenario; it does not ingest real timesheets.
* The business case is pre-tax with no working capital, inventory, price
  erosion or financing; break-even uses a constant contribution at the
  target-quantity cost.
* Readiness levels are suggestions from the evidence the project holds — a
  real technology readiness assessment needs an independent review.

## Graduate to

GanttProject (`bardsoftware/ganttproject`) and ProjectLibre for desktop
Gantt / PERT; OpenProject (`opf/openproject`) for team project management;
TaskJuggler (`taskjuggler/TaskJuggler`) for resource levelling; MPXJ
(`joniles/mpxj`) to move plans into Microsoft Project or Primavera formats;
frappe/gantt (`frappe/gantt`) and Mermaid (`mermaid-js/mermaid`) for Gantt
charts in your own pages. The lab exports CSV and Mermaid to bridge to them.
The closest open tools draw Gantt bars and dependencies; this lab adds the
float arithmetic, schedule risk, earned value, the business case and the
readiness gates, all tied to the product's requirements.

## References

* Project Management Institute, *A Guide to the Project Management Body of Knowledge (PMBOK Guide)*, 7th ed., 2021.
* NASA, *Systems Engineering Handbook*, SP-2016-6105 Rev 2, 2016.
* NASA, *Cost Estimating Handbook*, version 4.0, 2015.
* U.S. Department of Defense, *Technology Readiness Assessment Guidebook*; *Manufacturing Readiness Level (MRL) Deskbook* (check the current editions).
* J. C. Mankins, "Technology readiness levels: a white paper", NASA, 1995.
* J. E. Kelley & M. R. Walker, "Critical-path planning and scheduling", *Proc. Eastern Joint Computer Conference*, 1959.
* D. G. Malcolm, J. H. Roseboom, C. E. Clark & W. Fazar, "Application of a technique for research and development program evaluation", *Operations Research* 7(5), 1959.
* D. T. Hulett, *Practical Schedule Risk Analysis*, Gower, 2009.
* Q. W. Fleming & J. M. Koppelman, *Earned Value Project Management*, 4th ed., PMI, 2010.
* W. Lipke, "Schedule is different", *The Measurable News*, 2003.
* D. S. Christensen & S. R. Heise, "Cost performance index stability", *National Contract Management Journal*, 1993.
* K. T. Ulrich & S. D. Eppinger, *Product Design and Development*, McGraw-Hill.
* R. A. Brealey, S. C. Myers & F. Allen, *Principles of Corporate Finance*, McGraw-Hill.
* G. Marsaglia & W. W. Tsang, "A simple method for generating gamma variables", *ACM Transactions on Mathematical Software* 26(3), 2000.
