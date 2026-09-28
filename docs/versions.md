# Versions

Route: `#/versions` · Engine: `crates/forge-core/src/bridge/diff.rs` · UI: `web/src/pages/Versions.tsx`, `web/src/pages/versions/`

Onshape made version control part of computer-aided design (CAD): immutable **versions**, a mutable
**workspace**, **branches** for alternatives and a **compare** view. Protoforge
applies the same idea to the whole product model — requirements, bill of materials (BOM),
architecture, risks and every design parameter the labs read — so comparing two
versions also re-runs the verification and tells you what the change did.

## Concepts

| Onshape | Protoforge | Configuration management term |
|---|---|---|
| Version | a named snapshot of `project.json` | baseline |
| Workspace | the working state every lab edits | work in progress |
| Branch | a lane in the graph started from a version | alternative / variant |
| Compare | `bridge.diff` of two versions | configuration status accounting |

A **baseline** is an agreed, frozen description of the product (functional,
allocated and product baselines as a project matures); changes are proposed
against it and recorded. See the International Organization for Standardization (ISO) guideline ISO 10007:2017 *Quality management — Guidelines
for configuration management* and the configuration management process of the
*NASA Systems Engineering Handbook* (NASA/SP-2016-6105 Rev 2).

## Using the page

* **Create version** — name + description; the snapshot goes on the current
  branch and becomes the working state's base.
* **Version graph** — newest on top, one lane per branch; the dashed node is
  your working state (it pulses when it differs from its base). Each node shows
  the requirement verdicts of that version (✓ pass, ✕ fail, ○ unverified).
* **Restore** (with confirmation) loads a version into every lab.
* **Branch from here** creates a new lane from a version and loads it.
* **Compare** any two versions — or a version and the working state.
* **Auto-version** (toggle): 4 s after an edit, if the change is *significant*
  — a requirement verdict count, the number of requirements / BOM lines /
  subsystems / interfaces, or BOM cost or component mass by more than 1 % —
  and at least a minute after the last automatic snapshot, a version called
  `Auto · hh:mm` is created (the 25 most recent are kept). It runs once the
  Versions page has been opened in the session.
* **Export / Import history** as JSON (`"format": "protoforge-versions"`).

The history is stored in the browser (`localStorage`, key
`protoforge.versions.v1`), never inside `project.json`: the project file stays
one configuration, the history is the list of baselines around it. Export it to
keep it or to share it with a teammate.

## The built-in example: Iris v0.3 → v0.4

The first visit seeds the history with two versions that tell what the
simulations caught:

* **Iris v0.3** — the combiner 8 mm after the collimating lens, the
  battery-sense divider at 1 MΩ / 1.2 MΩ, no combiner-clearance requirement.
  All 24 requirements of the time pass, yet the Optics Lab puts the 45°
  combiner about 2.2 mm *into* the lens aperture, and the Circuit Lab shows the
  divider driving the analog-to-digital converter (ADC) pin to 4.2 V × 1.2 / 2.2 ≈ 2.29 V at a full cell —
  above the 1.8 V rail.
* **Iris v0.4** — combiner moved to 11 mm (+0.78 mm clearance) and requirement
  R-025 added so the interference cannot come back; divider changed to
  1.5 MΩ / 1 MΩ (4.2 V / 2.5 = 1.68 V at the ADC pin, 1.68 µA always-on).

Comparing them shows the parameter change, the two BOM lines, R-025 added and
passing, R-014's value moving from 1.91 µA to 1.68 µA, the combiner clearance
crossing zero, and the Circuit Lab's ADC check going from ✕ to ✓. The
descriptions quote numbers computed by the labs when the history is seeded.

## What a comparison contains

`bridge.diff {a, b}` returns:

* `params` — every changed leaf under `design.*` (path, old, new, Δ, %);
* `sections` — added / removed / changed requirements, BOM lines, needs,
  subsystems, interfaces, risks, trade studies and milestones, with the changed
  fields of each;
* `requirements_status` — both versions evaluated by every lab
  (`systems.evaluate`): status and value in A and B, classified as improved,
  regressed, added, removed, value change or same;
* `metrics` — every lab metric that moved, sorted by relative change;
* `circuit_checks` — Circuit Lab presets whose inputs come from a changed
  component value are re-run and their checks compared;
* `headline` — a plain-language summary of the above.

`bridge.summary {project, evaluate?}` gives the fingerprint (Fowler–Noll–Vo, FNV-1a 64-bit hash
of the canonical project JSON, shown as a 7-digit short id like a commit) and
key counts.

```
h₀ = 14695981039346656037
h ← (h ⊕ byte) · 1099511628211  (mod 2⁶⁴)   for every byte of project.json
```

## API

| Method | Input | Output |
|---|---|---|
| `bridge.diff` | `{a: Project, b: Project}` | structured differences + verdict changes |
| `bridge.summary` | `{project, evaluate?: bool}` | `{hash, short, counts, bom_cost_usd, component_mass_g, evaluation?}` |
| `bridge.example_history` | — | `[{name, description, project}]` (Iris v0.3, v0.4) |

## References

* ISO 10007:2017, *Quality management — Guidelines for configuration management*.
* NASA, *NASA Systems Engineering Handbook*, NASA/SP-2016-6105 Rev 2 (2016) —
  configuration management process and baselines.
* Onshape Learning Center — versions, workspaces, branching and merging:
  <https://learn.onshape.com>
* G. Fowler, L. C. Noll, K.-P. Vo, *FNV hash* (Fowler–Noll–Vo), IETF draft
  `draft-eastlake-fnv`.
