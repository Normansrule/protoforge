# Systems Studio

The Systems Studio (`#/systems`) is the front door of Protoforge. It is where a
product is defined — concept, stakeholder needs, requirements, architecture,
trade studies, risks and milestones — and where every requirement is verified
live against the numbers the engineering labs compute. Change the battery in
the Power lab, the frame in CAD Studio or the filter in the DSP Lab and the
verification matrix, budgets, traceability and V-model update immediately.

The worked example is **Iris**, a pair of monocular heads-up-display smart
glasses (`examples/smart-glasses/project.json`). At v0.6 it has 54
requirements: 51 pass and 3 are open goals — R-030 (lateral colour), R-047
(outdoor contrast) and R-053 (technology readiness); none fails and none is
unverified. Three smaller starter products ship as templates: the **Pebble
sensor tag**, the **Pip desk robot** and **Q-Link**, a two-node fiber quantum
key distribution (QKD) testbed. For a product of your own, the
[new-product wizard](new-product.md) (`#/new`) builds a starter project from a
plain-language description.

## How verification works

Every requirement carries a verification record: a method (analysis, test,
inspection or demonstration), optionally a **metric key** such as
`power.battery_life_h`, a comparator (≤, ≥, between, equals ± tolerance) and
target values. On every project edit the Rust engine (compiled to
WebAssembly) runs every lab's `metrics()` function, collects the metrics
(107 for Iris v0.6 — `./target/debug/protoforge metrics`) and checks each
requirement:

| Status | Meaning |
|---|---|
| ✓ pass | the bound metric meets the criterion |
| ✕ fail | the bound metric violates the criterion; for a requirement of kind *goal* ("should") this is an **open goal** — reported, counted separately, never a binding failure |
| ○ unverified | no metric bound (verify by test, inspection or demonstration), or no lab produces it yet |

The **margin** is the distance to the limit as a fraction of the target:
`(limit − value) / limit` for "at most" criteria and `(value − limit) / limit`
for "at least" criteria. Status is always shown as an icon plus a word, never
by colour alone.

## The tabs

1. **Mission control** — **"What should I do next?"** at the top (the eight
   highest-ranked actions, with the lab tab to open, the design levers that
   move each metric and a *Try fixing this* solver; see
   [What should I do next?](next-actions.md)), the verification ring (pass / fail / unverified), a
   *Run full verification* button that re-runs the engine and then reveals
   the results lab by lab in V-model order (each lab tile lights up as its
   requirements are checked, with a console log), requirement status as small
   multiples per subsystem, budget margins, the top risks by risk priority
   number (RPN), life-cycle progress and need status.
2. **Concept** — name, codename, version, tagline, description, problem,
   vision, users, use cases (the concept of operations (ConOps)),
   constraints and the stakeholder needs. Text fields commit after a short
   pause or on blur, so typing does not re-run the engine per keystroke.
3. **Requirements** — the verification matrix: ID, statement, kind, the
   needs it derives from, the subsystems it is allocated to, criterion, live
   value, a margin bar, status and a link to the lab that computes the
   metric. Filter by kind, status, subsystem or free text; sort by ID, kind,
   margin, status or lab. *+ Requirement* opens a form with a metric picker
   listing every metric the engine currently produces (grouped by lab, with
   its current value and unit — picking one pre-fills the unit) and a live
   preview of whether the criterion would pass. The **requirement quality**
   panel runs structural lint plus text checks after the International
   Council on Systems Engineering (INCOSE) *Guide to Writing Requirements*
   (vague terms, escape clauses, compound "shall" statements, text numbers
   that disagree with the criterion, unit mismatches, missing rationale) and
   tags each finding with the characteristic it threatens (unambiguous,
   singular, verifiable, …).
   Clicking a row opens the **requirement drawer** (also reachable at
   `#/systems?req=R-012`, which the requirement chips on every lab page link
   to): statement and rationale, criterion / live value / margin, the full
   trace chain need → requirement → subsystems → metric (with the lab's note)
   → lab with an *Open lab* button, a sparkline of the metric's values during
   this session, the quality findings and an editor.
4. **Architecture** — a block diagram of the subsystems grouped by domain,
   with interfaces drawn as labelled, colour-coded connectors (I2C, SPI, PDM,
   POWER, MECH, THERMAL …) and animated flow. Each block shows the pass count
   of its allocated requirements. Click a block for its functions, allocated
   requirements, parts from the bill of materials (BOM) with their mass, cost
   and average power, and its lab. The **N² matrix** view puts subsystems on
   the diagonal and interfaces in row *producer*, column *consumer*; feedback
   marks (below the diagonal) are highlighted, and a *Sequenced* view reorders
   the subsystems as a design structure matrix (DSM) to minimise feedback.
   Subsystems and interfaces are editable.
5. **Traceability** — a layered graph (needs → requirements → subsystems →
   metrics → labs) with curved edges, ordered by a barycentre heuristic to
   reduce crossings. Hover a node to highlight its impact set (everything
   upstream and downstream, from `systems.impact`); click to pin, double-click
   to open. The **matrix** view shows requirements × subsystems allocation
   with coverage gaps (untraced or unallocated requirements, subsystems that
   own no requirement) highlighted, and a need-coverage list.
6. **V-model** — an animated "V": concept of operations → system
   requirements → architecture → detailed design (the labs) down the left leg,
   implementation (printing, PCB fabrication, silicon, firmware, tooling) at
   the bottom, and unit verification → integration → system verification →
   validation up the right leg, with horizontal verification links between
   matching levels. Each stage's status and progress is derived from the live
   evaluation; click a stage for its evidence, with links to the relevant tab
   or lab.
7. **Trade studies** — editable weighted decision matrices: weight sliders,
   1–10 scores, "higher / lower is better" per criterion, add or remove
   options and criteria. Scores update live (edits are held in a local draft
   and committed after a pause so dragging stays smooth). Charts: weighted
   scores split into per-criterion contributions, a **tornado** of the
   winner's lead over the runner-up when each weight is scaled ±50 % (a bar
   crossing zero flips the decision; the engine also reports the smallest
   weight change that flips it), and a **radar** of the option profiles. A
   warning appears when the recorded decision differs from the weighted winner.
8. **Risks** — the Failure Mode and Effects Analysis (FMEA) table sorted by
   RPN = severity × occurrence × detection, with editable ratings, a 10 × 10
   severity × occurrence heat map with the risks as markers, and a risk
   editor (item, failure mode, effect, cause, mitigation). A risk is *high*
   when RPN ≥ 120 or severity ≥ 9, *medium* when RPN ≥ 60.
9. **Budgets** — mass, average power, battery life and unit cost roll-ups:
   stacked bars by subsystem against the limit from the bounding requirement,
   margins, line items, a per-subsystem table, and a **what-if sweep** that
   varies one design parameter (e.g. `battery.capacity_mah`), re-verifies
   every requirement at up to 15 values and plots the margins of the
   requirements that move, marking those whose status flips.
10. **Roadmap** — stage-gate milestones (M-0 concept review … M-5 production
    validation) with deliverables and exit criteria. Criteria the engine can
    check from the project — "All analysis requirements green", "Every
    requirement has a metric and an owner", "Top trades decided", "Every need
    satisfied", "No high risks" — are marked automatically; the rest take a
    manual sign-off stored in `design.extra.studio.gates`.
11. **Report & templates** — the full systems-engineering report rendered
    from Markdown (concept, needs coverage, verification matrix, budgets,
    architecture, trades, FMEA, gates, all metrics) with downloads
    (`*-systems-report.md`, the verification-only report, `project.json`), and
    the template gallery: *Blank*, *Iris smart glasses*, *Pebble sensor tag*,
    *Pip desk robot*, *Q-Link*. Loading a template asks for confirmation and
    replaces the current project; to describe a new product in plain words,
    use the new-product wizard (`#/new`) instead.

## Building a new product

1. **Start**: open the new-product wizard (`#/new`, or *New product* in the
   command palette, Ctrl/⌘ + K) and describe the product — kind, capabilities,
   targets — to get a starter project whose requirements are bound only to
   metrics the engine computes for it; or start from *Blank* or a template in
   **Report & templates**. Save the old project first if you need it.
2. **Concept**: write the problem and vision, list users and use cases, note
   constraints, and write the stakeholder needs in the users' own words.
3. **Requirements**: for each need, add measurable "shall" requirements.
   Bind each to a metric from the picker, pick the comparator and target,
   and fix what the quality panel flags.
4. **Architecture**: add subsystems (choose a domain and the lab that designs
   each), connect them with interfaces, and allocate every requirement.
   Check the traceability matrix for gaps.
5. **Trade studies and risks**: decide the big choices with weighted
   matrices; run the FMEA and turn mitigations into requirements.
6. **Design in the labs**: every change re-verifies the requirements. Watch
   the budgets and the V-model fill in; let **What should I do next?** pick
   the next issue and its levers, and use the what-if sweep to find margins.
   Check the solver's side effects before applying a fix: on Iris, raising
   combiner reflectance closes R-047 but fails R-054 (see-through 70 % → 51 %).
7. **Roadmap**: pass each gate; export the report, then the fabrication
   package from the Manufacturing Hub.

## Engine API added for the studio

All methods live in `crates/forge-core/src/systems/` and are available from
the browser (`call('systems.…', input)`), the CLI (`protoforge call …`) and
the REST server. Methods marked *(+ev)* take `{ project, evaluation? }` and
reuse the caller's evaluation when given.

| Method | Returns |
|---|---|
| `systems.studio` *(+ev)* | everything below in one call (budgets, N², coverage, V-model, gates, quality, FMEA, trade details, trace graph) |
| `systems.budgets` *(+ev)* | mass, power, battery-life and cost budgets with items, per-subsystem shares, limit, margin and status |
| `systems.n2` | N² matrix: cells, fan-in/out, feedback marks, DSM-sequenced order |
| `systems.coverage` *(+ev)* | need coverage, requirement tracing/allocation/metric binding, verification-method and kind mix, gaps, completeness |
| `systems.vmodel` *(+ev)* | nine V-model stages with status, progress, evidence items and links |
| `systems.gates` *(+ev)* | milestone exit criteria and deliverables, auto-checked where possible |
| `systems.quality` | requirement-quality findings (lint + INCOSE-style text checks) |
| `systems.trade_detail` | a trade study's scores, normalised weights, contributions, ranking, lead, decision agreement and tornado |
| `systems.parameters` | every numeric design parameter (`battery.capacity_mah`, …) |
| `systems.sensitivity` | sweep one parameter (`{project, param, lo?, hi?, steps?, values?}`, ≤ 15 points) and report per-requirement values, margins, statuses and flips |
| `systems.templates` / `systems.template` | starter projects (`blank`, `iris`, `pebble`, `pip`, `qlink`) |
| `systems.report_full` *(+ev)* | the full Markdown systems-engineering report |
| `systems.next_actions` | ranked "what should I do next?" actions with lab routes and measured levers (`{project, evaluation?, levers?, limit?}`) |
| `systems.next_levers` | the design parameters that measurably move one metric (on demand for the expensive labs) |
| `systems.solve_lever` | one-parameter "solve for target" with a full re-verification and every side effect |
| `systems.wizard` / `systems.wizard_options` | the new-product wizard: answers → starter project (10 kinds, 18 capabilities) / the option lists |

The verification roll-ups also add three metrics that any project can bind
requirements to: `mass.components_g` (always emitted now),
`budget.avg_power_mw` (Σ component average power ÷ regulator efficiency) and
`budget.battery_life_h` (capacity × nominal voltage × usable fraction ÷
average power). Existing functions (`all_metrics`, `evaluate`, `lint`,
`report_markdown`, …) keep their behaviour.

## Concepts, briefly

* **Needs vs requirements.** A need is the stakeholder's statement ("last a
  full working day"); a requirement is the engineer's measurable, verifiable
  translation ("battery life in the reference profile shall be at least 8 h").
* **Verification vs validation.** Verification: did we build it right (does
  the design meet each requirement)? Validation: did we build the right thing
  (does it meet the needs)? Here a need is validated when every requirement
  derived from it passes.
* **Traceability.** Every requirement traces up to a need and down to an owner
  (subsystem) and a verification; the impact set tells you what to re-check
  after a change.
* **Technical budgets.** Resources allocated to subsystems with margins that
  shrink as estimates become measurements.
* **Trade studies.** Weighted-sum decision matrices; always test how robust
  the winner is to the weights.
* **FMEA.** Failure mode, effect, cause; severity, occurrence, detection; RPN
  and mitigations that become design changes and requirements.
* **Stage gates.** Go / no-go reviews between life-cycle stages (concept,
  requirements, design, engineering / design / production validation tests —
  EVT, DVT, PVT).

## Limitations

* The engine is synchronous: *Run full verification* re-runs the evaluation
  (tens to hundreds of milliseconds) and then paces the reveal for people; it
  is not streaming results from the labs.
* The power roll-up is a planning estimate (average load ÷ one regulator
  efficiency); the Power lab's rail-by-rail model is authoritative where it
  exists.
* The PCB lab's placement heuristics are tuned for the Iris board outline and
  a new outline routes from scratch in seconds, so the Pebble and Pip
  templates list their board parts with `on_pcb = false`; their PCB
  requirements read "unverified" until the parts are placed.
* The metric history sparkline is kept in memory for the current session only.
* Quality checks are heuristics; they cannot judge whether a requirement is
  necessary or feasible — a reviewer still has to.

## References

* NASA, *Systems Engineering Handbook*, NASA/SP-2016-6105 Rev 2 —
  <https://www.nasa.gov/reference/systems-engineering-handbook/>
* INCOSE, *Systems Engineering Handbook*, 5th ed., Wiley, 2023.
* INCOSE Requirements Working Group, *Guide to Writing Requirements*.
* ISO/IEC/IEEE 15288:2023, *Systems and software engineering — System life
  cycle processes*.
* K. Forsberg & H. Mooz, "The relationship of system engineering to the
  project cycle", NCOSE / ASEM joint symposium, 1991 (the V-model).
* AIAG & VDA, *FMEA Handbook*, 2019.
* R. J. Lano, "The N² Chart", TRW Software Series, 1977.
* D. V. Steward, "The Design Structure System: a method for managing the
  design of complex systems", *IEEE Transactions on Engineering Management*
  EM-28, 1981.
* T. L. Saaty, *The Analytic Hierarchy Process*, McGraw-Hill, 1980.
* R. G. Cooper, "Stage-gate systems: a new tool for managing new products",
  *Business Horizons* 33(3), 1990.
* AIAA S-120A-2015, *Mass Properties Control for Space Systems*.
