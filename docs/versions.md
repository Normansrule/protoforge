# Versions

Route: `#/versions` · Engine: `crates/forge-core/src/bridge/diff.rs`, `bridge/merge.rs` · UI: `web/src/pages/Versions.tsx`, `web/src/pages/versions/`

Onshape made version control part of computer-aided design (CAD): immutable **versions**, a mutable
**workspace**, **branches** for alternatives and a **compare** view. Protoforge
applies the same idea to the whole product model — requirements, bill of materials (BOM),
architecture, risks and every design parameter the labs read — so comparing two
versions also re-runs the verification and tells you what the change did.

## Concepts

| Onshape | Git | Protoforge | Configuration management term |
|---|---|---|---|
| Version | commit / tag | a named snapshot of `project.json` | configuration record (a *baseline* when flagged) |
| Workspace | working tree | the working state every lab edits | work in progress |
| Branch | branch | a lane in the graph started from a version | alternative / variant |
| Merge | three-way merge | **merge by choosing** (`bridge.merge`) → a merge version with two parents | approved change incorporated |
| Compare | diff | `bridge.diff` of two versions, both re-verified | configuration status accounting / design review |
| Release | signed tag / release | ⚑ baseline flag | functional, allocated, product baseline |

(Onshape's and Git's behaviour is described here in our own words; see their
own documentation for the details.)

A **baseline** is an agreed, frozen description of the product; changes are
proposed against it and recorded. Systems engineering names three classic
ones: the **functional baseline** (what the system must do — the system
requirements), the **allocated baseline** (how the requirements are split among
subsystems and interfaces, with the analyses that show the split works) and the
**product baseline** (the build-to design — drawings, board, BOM, manufacturing
files). The International Organization for Standardization (ISO) guideline
ISO 10007:2017 *Quality management — Guidelines for configuration management*
and the SAE/EIA-649 *Configuration Management Standard* describe the same four
activities — configuration identification, change control, status accounting
and audit — without prescribing a tool; this page is one lightweight way to do
them. The clause-level requirements are in the standards themselves (check the
current revisions); the *NASA Systems Engineering Handbook*
(NASA/SP-2016-6105 Rev 2) explains the process in plain language.

## Using the page

* **Create version** — name + description; the snapshot goes on the current
  branch and becomes the working state's base.
* **Version graph** — newest on top, one lane per branch with a legend; the
  dashed node is your working state (it pulses when it differs from its base).
  Each node shows the requirement verdicts of that version (✓ pass, ✕ fail,
  ○ unverified); a red dot marks a version with a failing requirement, a
  diamond a merge (with a dashed edge from the tip that was merged in) and a
  ⚑ flag a baseline. **▶ Replay history** rebuilds the graph oldest-first
  (skipped with `prefers-reduced-motion`).
* **Baseline** — in a version's detail card, flag it as a functional,
  allocated or product baseline. The **Baselines & concepts** tab lists the
  baselines with their verdicts and a one-click compare with the previous
  baseline, draws the configuration-management loop (identify → change →
  review → approve → account → audit) as an animated cycle, and maps Onshape,
  Git and Protoforge terms side by side.
* **Restore** (with confirmation) loads a version into every lab.
* **Branch from here** creates a new lane from a version and loads it.
* **Compare** any two versions — or a version and the working state. The
  **Compare (design review)** tab evaluates both with every lab.
* **Merge branches** — pick a branch to bring into the one you are on. The
  engine finds the merge base (the newest version both descend from) and
  performs a three-way merge of the whole `project.json`: a value changed on
  only one side is taken from that side; a value both sides changed
  differently is a **conflict** (default: ours). Every difference is listed
  with base / ours / theirs and an **ours | theirs** switch, so you merge by
  choosing. **Verify the merge** evaluates the result with every lab and lists
  what it changes against your working state; **Commit merge** loads it and
  saves a merge version whose second parent is the branch tip.
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

## The built-in example: Iris v0.3 → v0.4, and an experiment

The first visit seeds the history with three versions — two that tell what the
simulations caught, and one experiment on its own branch:

* **Iris v0.3** — the combiner 8 mm after the collimating lens, the
  battery-sense divider at 1 MΩ / 1.2 MΩ, no combiner-clearance requirement.
  All 24 requirements of the time pass, yet the Optics Lab puts the 45°
  combiner about 2.2 mm *into* the lens aperture, and the Circuit Lab shows the
  divider driving the analog-to-digital converter (ADC) pin to 4.2 V × 1.2 / 2.2 ≈ 2.29 V at a full cell —
  above the 1.8 V rail.
* **Iris v0.4** — combiner moved to 11 mm (+0.78 mm clearance) and requirement
  R-025 added so the interference cannot come back; divider changed to
  1.5 MΩ / 1 MΩ (4.2 V / 2.5 = 1.68 V at the ADC pin, 1.68 µA always-on).
  Flagged as the *allocated baseline*.
* **Explore: 250 mAh cell** on branch `explore/battery-250` — the cell grows
  from 180 mAh to 250 mAh, its mass scaled at the same energy density
  (4.0 g → 5.56 g, in both `design.battery` and BOM line BT1; the unit cost is
  left unchanged until a supplier quotes it). Its description states only the
  change; comparing it with v0.4 shows what the labs say: battery life rises
  from about 10.3 h to 14.4 h (R-002 still passes, with more margin — a unit
  test checks the ratio is within 8 % of 250/180), and total mass grows by
  1.56 g, which takes R-001 (≤ 50 g) to within a fraction of a gram of its
  limit. That trade — hours of battery life against the last of the mass
  margin — is the decision to make before merging it into `main` on the
  **Merge branches** tab.

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
| `bridge.merge` | `{base, ours, theirs: Project, choices?: {path: "ours" \| "theirs"}}` | `{project, error, decisions: [{path, base, ours, theirs, kind: "ours" \| "theirs" \| "conflict", default, chosen}], conflicts, from_theirs, summary}` |
| `bridge.example_history` | — | `[{name, description, branch, parent, baseline, project}]` (Iris v0.3, v0.4, the 250 mAh experiment) |

### How the merge walks the file

Objects are merged key by key; arrays of records with an `id` (or `refdes` for
BOM lines) are merged record by record — so two branches that each add a
requirement keep both, and a record deleted on one side and untouched on the
other is deleted. Any other value is a leaf. The unit tests in `merge.rs`
cover non-overlapping edits, a conflict resolved both ways, records added on
both sides, a deletion overridden by choice, and identical tips.

## References

* ISO 10007:2017, *Quality management — Guidelines for configuration management*.
* SAE/EIA-649, *Configuration Management Standard* (check the current revision).
* NASA, *NASA Systems Engineering Handbook*, NASA/SP-2016-6105 Rev 2 (2016) —
  configuration management process and baselines.
* INCOSE, *Systems Engineering Handbook*, 5th ed., Wiley, 2023.
* S. Chacon and B. Straub, *Pro Git*, 2nd ed., Apress, 2014 — ch. 3, Git
  Branching (basic merging, three-way merge): <https://git-scm.com/book/en/v2>
* V. Driessen, “A successful Git branching model”, 2010 —
  <https://nvie.com/posts/a-successful-git-branching-model/>
* Onshape Learning Center — versions, workspaces, branching and merging:
  <https://learn.onshape.com>
* G. Fowler, L. C. Noll, K.-P. Vo, *FNV hash* (Fowler–Noll–Vo), IETF draft
  `draft-eastlake-fnv`.
