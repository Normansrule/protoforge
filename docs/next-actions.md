# What should I do next?

A project with 54 requirements and 23 labs answers a lot of questions — but
the first question anyone opening it has is simpler: **what should I work on
now?** The next-actions engine answers it for any project, the way a senior
systems engineer would: it points at the most important open issue, says in
plain words why it matters, opens the lab tab where you can change it, lists
the design parameters that *measurably* move it, and offers to try a fix —
showing every side effect before anything is applied.

It appears in two places:

* **Overview** (`/`) — a compact list of the top 6 actions under the welcome card.
* **Systems Studio → Mission control** (`/systems`) — the full list (top 8, "Show all"),
  with metric notes, scores and the "How is this ranked?" explainer.

Engine code: `crates/forge-core/src/systems/next/` (`mod.rs` ranking,
`levers.rs` lever measurement and the solver, `routes.rs` metric → lab tab).
Web: `web/src/ux/next/NextActions.tsx`.

## What counts as an action

| Kind | Base score | Severity | Raised when |
|---|---|---|---|
| Failing requirement | 100 | critical (safety/regulatory) or high | a binding ("shall") requirement fails its criterion |
| Unverified | 90 | high (medium for goals) | a requirement is bound to a metric that no lab produced |
| Open goal | 80 | medium | a goal ("should") misses its target |
| Unmitigated high risk | 70 | high (severity ≥ 9) or medium | a failure mode and effects analysis (FMEA) row is *high* (risk priority number (RPN) ≥ 120 or severity ≥ 9) with no mitigation written |
| Requirement error | 60 | medium | a requirement-quality *error* (duplicate id, dangling reference, range without an upper bound) |
| Marginal pass | 50 | low | a requirement passes with less than 10 % margin (see below) |
| Stage gate | 40 | medium | the current gate (first one not passed) has exit criteria not met |
| No verification | 30 | low (info for test/inspection) | a requirement has no metric: by analysis it can never turn green; by test, inspection or demonstration it needs a plan and a recorded result |
| Traceability gap | 20 | low | requirements not traced to a need, not allocated to a subsystem, needs without requirements, empty subsystems (grouped into one action each) |
| Wording | 10 | low | requirement-quality warnings after the International Council on Systems Engineering (INCOSE) *Guide to Writing Requirements* (grouped per requirement) |
| Wording suggestion | 0 | info | all info-level wording hints, grouped into one action |

## The ranking rule

```
score = base(kind) + bonus
bonus = 10 × clamp((importance × urgency − 1) / 1.5, 0, 0.99)
importance = 1 + 0.15 [safety or regulatory requirement] + 0.10 [serves a priority-1 "must" need]
urgency    = 1 + min(shortfall, 1)                        failing requirements and open goals
           = 1 + (1 − margin / 10 %)                       marginal passes
           = 1 + min(RPN / 200, 1)                         risks
           = 1 + 0.1 × open items (max 5)                  gates and wording
```

The bases are 10 points apart and the bonus stays below 10, so **the kind
always decides first** (a failing binding requirement always outranks any
goal, a goal always outranks any marginal pass), and importance × urgency
orders actions within a kind. Every action carries its `score_note`, e.g.
"Open goal (base 80) + 4.3 for importance 1.10 × urgency 1.50".

**Why 10 % for "marginal".** It is about the uncertainty of the first-order
models the labs use — closed-form optics, lumped thermal networks,
parts-count reliability, Monte Carlo schedules with a few hundred runs. A
pass inside that band can flip when a model is refined, a part is measured
or another subsystem changes. Programmes set their own margin policy; this
is a default, not a standard. Two kinds of criteria are never marginal: a
target of 0 (the margin is then absolute, in the metric's unit — "≤ 0 DRC
violations", "≥ 0 mm clearance") and a target at the natural limit of the
quantity ("≥ 100 % routed").

### Iris v0.6, as computed

54 requirements: 51 pass, 3 open goals, 0 failing. Nine passes are marginal,
so **42 of 54 (78 %) pass with margin**. The engine lists 14 actions
(`./target/debug/protoforge next --all`):

| # | Score | Kind | Action | Where |
|---|---|---|---|---|
| 1 | 84.3 | Open goal | Raise lowest technology readiness level (TRL) to ≥ 6 (R-053; now 3, 50 % short) | Program & Business Lab → Readiness & reviews |
| 2 | 82.8 | Open goal | Bring lateral colour down to ≤ 4.5 arcmin (R-030; now 5.79, 29 % short) | Optics Lab → Image quality |
| 3 | 82.0 | Open goal | Raise ambient contrast ratio to ≥ 1.5 :1 (R-047; now 1.24, 18 % short) | Human Factors Lab → Legibility & eyebox |
| 4 | 58.6 | Marginal | Peak frame surface temperature 40.32 °C vs ≤ 41 °C (R-039, safety, 1.6 %) | Thermal & Fluids Lab → 3-D conduction |
| 5 | 58.4 | Marginal | Structural safety factor 2.04 vs ≥ 2 (R-026, safety, 1.9 %) | Structures Lab → Strength |
| 6 | 57.7 | Marginal | 2-year reliability 90.38 % vs ≥ 90 % (R-048, 0.4 %) | Reliability & Test Lab → System reliability |
| 7 | 56.7 | Marginal | Total mass 49.09 g vs ≤ 50 g (R-001, 1.8 %) | CAD Studio → Mass & export |
| 8 | 56.5 | Marginal | 80th-percentile schedule 50.9 weeks vs ≤ 52 (R-051, 2.0 %) | Program & Business Lab → Schedule risk |
| 9 | 55.9 | Marginal | Pod skin temperature 39.0 °C vs ≤ 41 °C (R-008, safety, 4.9 %) | Power & Thermal → Thermal |
| 10 | 54.0 | Marginal | Eyebox coverage 89.6 % vs ≥ 85 % (R-046, 5.4 %) | Human Factors Lab → Legibility & eyebox |
| 11 | 52.5 | Marginal | Design validation test (DVT) plan 37 units vs ≤ 40 (R-050, 7.5 %) | Reliability & Test Lab → Test plan (DVT) |
| 12 | 51.7 | Marginal | Angular resolution 27.2 px/deg vs ≥ 25 (R-005, 8.6 %) | Optics Lab → Ray diagram |
| 13 | 40.7 | Stage gate | Preliminary design (M-2): "all analysis requirements green" — 51/54 | Systems Studio → Roadmap |
| 14 | 10.7 | Wording | R-037: the text says "50 Ω ± 5 Ω" while the criterion is stored as the range 45…55 (the checker cannot match them) | Systems Studio → Requirements |

Note how R-039 (1.6 %, safety) leads the marginal passes although R-048 has
the thinnest margin (0.4 %): safety adds importance.

## Levers: measured, not claimed

For every failing, goal or marginal action whose metric belongs to a
lab module, the engine measures which design parameters move it:

1. **Owner.** The metric's prefix names the module (`thermal.*` → `power`,
   `mass.*`/`fit.*`/`bom.*`/`budget.*` → the systems roll-up, which re-runs CAD first).
2. **Candidates.** A curated shortlist of the parameters each module reads
   (`SHORTLIST` in `levers.rs`): e.g. `optics.*` for Optics; `extra.ergonomics.**`
   plus display luminance, combiner reflectance, eye relief and frame sizes for
   Human Factors; `extra.rf.**` plus board thickness for RF. Only positive
   numbers are candidates (zero and negative values are often "auto" sentinels).
   Environment and use conditions — ambient temperature, the skin limit, uses per
   day, design life, head width, the build quantity a cost requirement is stated
   at, material densities — are **not** levers (`DENY`).
3. **Probe.** Each candidate is nudged by **+5 %** (integers by at least 1; a
   fraction near 1 is probed downwards instead) and *only the owning module's
   metrics* are re-run — the same per-module functions `METRIC_MODULES`
   lists for `systems.evaluate`.
4. **Report.** For each lever: the metric change per step, the percentage
   effect, which way to move the parameter to gain margin (relative to the
   requirement's criterion, so "≤" and "≥" and ranges all read correctly), and
   a straight-line estimate of the value that reaches the target with 2 % margin
   (shown only when today's margin is below that and the estimate lies within
   ÷4…×4 of today's value). "Try fixing this" on a *marginal* pass aims instead
   for the 10 % margin band, so its answer goes further than the estimate. Levers that move the margin by
   less than 0.01 % of the target per step are dropped; the six strongest are kept.

Probes are memoised per module and project, and each module is re-run once on
the real project afterwards so its own result cache stays warm for the next
verification. The six modules with expensive analyses — Structures (3-D
finite element analysis (FEA)), Thermal & Fluids (3-D FEA), Robotics, PCB
Studio (autorouter), Manufacturing and Molding — are measured **on demand**:
their actions say `levers_status: "lazy"` and the web page calls
`systems.next_levers` when you open "Levers".

**Speed.** On Iris the whole list (14 actions, levers measured inline for the
Optics, Human Factors, Power, Program, Reliability and systems roll-up
modules) takes 1.8 s cold and 53 ms memoised in an unoptimised debug build
(`cargo test -p forge-core iris_list_is_interactive -- --nocapture`); the
release WebAssembly engine in the browser (headless Chromium, shared 2-CPU machine) took
**0.75–0.93 s** for the same list, including the evaluation it runs first (the worker reports
the time; it is shown under "How is this ranked?"). "Try fixing this" on R-047 took 1.4–1.6 s
(32 module runs + one full verification), and the on-demand Structures levers for R-026
2.2–6.3 s (eight 3-D finite element runs).

### Iris levers, as measured

R-047, ambient contrast ratio (ACR) against a 5,000 cd/m² sunlit street (now 1.237 :1):

| Lever | Step | ACR change | Straight-line estimate |
|---|---|---|---|
| ↑ Optics · combiner reflectance (0.30) | +0.015 | +0.0173 | 0.55 |
| ↑ Optics · display luminance (3,000 cd/m²) | +150 cd/m² | +0.0118 | 6,721 cd/m² |

Hand check: the Human Factors model is ACR = 1 + L·τ·R / (L_bg·(1 − R)) with
τ = 0.92 lens transmission: 1 + 3000·0.92·0.3 / (5000·0.7) = 1.2366 ✓, and a 5 %
step in L moves ACR by 0.05 × 0.2366 = 0.0118 ✓ (unit-tested). The ↑ reflectance lever is
measured on the Human Factors model alone; it does not know that see-through is 1 − R, so the
lever list shows it as a clean gain — the solver's full verification (below) is what reveals that
it fails R-054.

R-030, lateral colour (now 5.79 arcmin): ↓ lens focal length is the strongest
lever (−0.71 arcmin per +1 mm), then lens diameter, display-to-lens distance
and display width. R-001, total mass (49.09 g): ↓ frame wall thickness
(+0.53 g per +0.06 mm), rim width, front thickness, temple height and width.
R-002, battery life: ↑ battery capacity and ↑ regulator efficiency (unit-tested
signs). R-053 (TRL) has **no** numeric lever — readiness climbs with evidence
(passing requirements, built prototypes, sign-offs), which the Program lab's
Readiness tab shows — and the card says so instead of inventing one.

## "Try fixing this": solve for target

`systems.solve_lever` takes one requirement and one parameter and finds the
value that meets the target with a **2 % margin** (10 % for marginal passes,
so "Add margin" really adds it; it falls back to the bare target when the
margin is out of reach):

1. **Range.** ÷4 … ×4 of today's value (or `lo`/`hi`), clipped to physical
   bounds: fractions inside (0, 1), percentages ≤ 100, counts ≥ 1, everything else positive.
2. **Scan.** 13 points (geometric spacing) plus today's value, each a run of the owning module only.
3. **Bracket and bisect.** The passing point nearest today's value and its failing
   neighbour are bisected (geometric midpoints, integers in whole steps) until the
   bracket is below 10⁻⁶ relative; the result is rounded to three significant
   figures on the passing side and re-checked.
4. **Verify.** One full `systems.evaluate` of the changed project; every requirement
   whose verdict, value or margin moved is returned, verdict changes first.
5. **Honesty.** If nothing in range meets the target, the answer is "No value of …
   between … and … meets …: the best in range is …" — never a made-up fix.

**Coupled effects.** Two first-order couplings the labs do not model are applied
by default and labelled *assumption* in the panel (untick to see the labs'
model alone): a micro-display's active power scales with its luminance (an
upper bound — real modules have a fixed driver overhead), and a cell's mass
scales with its capacity at the same specific energy.

### Iris examples, as computed

* **R-047 by display luminance**: 3,000 → **6,730 cd/m²** gives ACR 1.531 (≥ 1.5 :1 with 2 %
  margin), 32 module runs. With the coupling, the display line U2 goes from 140 to 314 mW
  active and **six requirements flip to fail**: battery life 13.8 → 6.7 h (R-002), average
  power 54.8 → 112.8 mW (R-003), pod skin temperature 39.0 → 49.6 °C (R-008), peak frame
  temperature 40.4 → 52.1 °C (R-039), hotspot rise 8.9 → 19.2 °C (R-040) and 2-year reliability
  90.4 → 23.7 % (R-048); passing count 51 → 46. Without the coupling only R-047 changes.
* **R-047 by combiner reflectance**: 0.30 → **0.49** gives ACR 1.530, but see-through
  transmission falls from 70 % to 51 % (1 − R) and **R-054 (see-through ≥ 60 %, safety) flips
  to fail**; passing count stays 51 → 51 (R-047 gains, R-054 loses), 33 module runs. Before
  R-054 was added this lever looked free — no other requirement moved — which is exactly
  why R-054 was added: the solver's side-effect list showed a cost no requirement was watching.
* **So both R-047 levers break something.** A brighter display trips power, battery life, skin
  temperature and reliability (six requirements); a more reflective combiner trips see-through
  (R-054, 70 % → 51 %). With R = 0.40 — the most R-054 allows — the same hand formula gives
  ACR 1 + 3000·0.92·0.4 / (5000·0.6) = 1.37 :1, still short of 1.5. That is the honest reason
  R-047 stays an open goal: no single parameter closes it without failing a binding requirement.
  The way out is architectural — a display with higher luminous efficacy (more nits per milliwatt,
  so the power coupling does not apply), or a photochromic outer lens that darkens only in
  sunlight (then R-054 must say under which light it applies) — and is a trade study for Systems
  Studio, not a slider.
* **R-030 by lens focal length**: 20 → **24.1 mm** fixes lateral colour (5.79 → 4.38 arcmin)
  but breaks the field of view (R-004, 23.6° → 19.1°), the virtual-image distance (R-006,
  1.34 → 0.14 m) and the spot size (R-029, 19.4 → 27.1 µm) — a one-parameter fix is the wrong
  tool here; the Optics Lab's Lens designer (an achromat) is the real answer.
* **R-001 by frame wall thickness**: 1.20 → **1.19 mm** brings total mass to 49.00 g (2 % margin)
  and nudges ten other requirements the right way (pod fit, hotspot, nose-pad pressure, cost).
* **R-053 by battery capacity**: "No value of Battery · capacity between 60 and 960 mAh meets
  program.min_trl ≥ 6 TRL" — correctly, since readiness does not depend on the battery.

## API

| Method | Input | Output |
|---|---|---|
| `systems.next_actions` | `{ project, evaluation?, levers?: "inline" \| "all" \| "none", limit? }` | `{ actions: Action[], summary }` |
| `systems.next_levers` | `{ project, metric, requirement_id?, limit? }` | `{ metric, module, requirement_id, candidates, levers }` |
| `systems.solve_lever` | `{ project, requirement_id, param, lo?, hi?, couple? = true, margin?, evaluation? }` | `{ found, message, before_param, after_param, before_value, after_value, changes, side_effects, pass_before/after, … }` |

`Action` = `{ id, kind, severity, score, score_note, title, why, requirements, metric,
metric_label, value, target, target_hi, cmp, criterion, unit, margin, note,
where: { route, tab, label, page, tab_label, href }, module, levers, levers_status }`.
`Lever` = `{ param, label, unit, value, step, integer, delta, effect_pct, direction,
margin_per_step, estimate, lo, hi }`. `changes` is the list "Apply" writes: design paths
under `design`, or a component's field by reference designator.

## Limitations

* **One parameter at a time, one step.** Levers are one-at-a-time finite differences
  around today's design; interactions between parameters and strong non-linearity far
  from today's value are not captured (that is what the Systems Studio sensitivity sweep
  and a design-of-experiments are for). The straight-line estimate is only a hint; the
  solver's value is the real one.
* **The solver finds the crossing nearest today's value** on a 13-point grid; a narrow
  passing window between grid points can be missed, and a non-monotonic metric may have
  other solutions further away. It changes one parameter and reports the side effects —
  it never trades requirements off for you.
* **Shortlists are curated.** A parameter a module reads but the shortlist omits is not
  offered; bill of materials (BOM) entries (component power, mass, cost) and string choices
  (material, lens type, fab tier) are not numeric levers. Custom metric prefixes without a
  module get no levers.
* **Couplings are only the two above**, both first-order and labelled; everything else is
  exactly what the labs compute.
* **Lazy modules** take a few seconds in the browser because each probe re-runs a
  finite-element, layout or planning model. The Structures FEA uses 2 mm voxels, so a 5 %
  step of a 4 mm temple can change the voxelisation and give a jumpy, mesh-driven lever
  (Iris: temple width −0.57 in safety factor per +0.2 mm); treat such levers as a prompt to
  run the Structures Lab at fine fidelity, not as a gradient.
* **Tabs**: labs that read `?tab=` open on the right tab; for the others the page clicks the
  tab button once the lab has loaded (see the suggestion in the round report).
* The 10 % marginal band and the 2 % solver margin are defaults, not programme policy.

## References

* INCOSE Requirements Working Group, *Guide to Writing Requirements* (INCOSE-TP-2010-006).
* National Aeronautics and Space Administration (NASA), *Systems Engineering Handbook*,
  SP-2016-6105 Rev 2 (2016) — technical performance measures and margin management.
* A. Saltelli et al., *Global Sensitivity Analysis: The Primer*, Wiley (2008) — one-at-a-time
  sensitivity and its limits.
* R. L. Burden and J. D. Faires, *Numerical Analysis* — bisection and bracketing.
* AIAG & VDA, *FMEA Handbook* (2019) — note it replaces RPN ranking with "action priority";
  Protoforge's FMEA tab still uses S × O × D, and so does this ranking.
