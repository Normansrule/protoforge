# Start here

Protoforge has 23 labs. This guide shows the features that help any engineer
— electrical, mechanical, firmware, optical, radio frequency (RF), controls
and robotics, manufacturing, test and reliability, systems and program,
quantum, or a student — find their way in the first minute: **where to
start, what to change and what the numbers mean.**

Open it in the app: [Start here](#/start).

## 1. Pick your discipline (`#/start`)

The start page asks one question: *I work on…* Pick one of eleven
disciplines (you can change it any time; deep link with
`#/start?role=<id>`, for example `#/start?role=rf`).

| Role id | Discipline | Your labs (pinned first in the sidebar) |
|---|---|---|
| `electrical` | Electrical & electronics | Circuit, Power & Thermal, PCB Studio, RF & Antenna, Build Guide, Prototype & Firmware |
| `mechanical` | Mechanical & structures | CAD Studio, Structures, Mechanisms, Thermal & Fluids, Print, Human Factors |
| `firmware` | Firmware & software | Prototype & Firmware, DSP, Chip, Control, RL |
| `optical` | Optical | Optics, Human Factors, Structures, CAD Studio |
| `rf` | RF & wireless | RF & Antenna, PCB Studio, Prototype & Firmware, Power & Thermal |
| `controls` | Controls & robotics (including reinforcement learning, RL) | Control, Robotics, RL, Mechanisms, DSP |
| `manufacturing` | Manufacturing & industrial | Build Guide, Manufacturing Hub, Molding, Print, PCB Studio, CAD Studio |
| `test` | Test & reliability | Reliability & Test, Structures, Mechanisms, Thermal & Fluids, Build Guide |
| `systems` | Systems & program | Program & Business, Reliability & Test, Manufacturing Hub, Power & Thermal, Human Factors |
| `quantum` | Quantum | Quantum, Chip, DSP, Optics |
| `student` | New to engineering / student | Circuit, Power & Thermal, CAD Studio, Print, Structures, Optics |

For the chosen discipline the page shows:

* **What Protoforge does for you** — one line.
* **Your path** — 5–8 ordered steps. Each names the lab and tab (one click
  opens it), says what to do in plain words, shows the Iris v0.6 requirement
  or metric that step moves **with its live value and pass/fail status**, and
  gives a time estimate. Requirements that are open goals for Iris (R-030
  lateral colour, R-047 outdoor contrast, R-053 technology readiness) are
  marked with "!" rather than hidden. If you have replaced Iris with your own
  product, steps whose requirement your project does not have say "Iris
  example".
* **Five-minute first win** — a short quest with tick boxes (saved in your
  browser) and an *Open* link per step, for example "Match an antenna" for
  RF or "Will the temple break?" for mechanical.
* **Your labs** — the labs the sidebar shows first in Simple mode.
* **Words you will meet** — key terms; hover, focus or tap one for its full
  form and a plain definition.
* **Tools you will graduate to** — the professional and open-source tools
  from [Reference repositories](reference-repos.md) that take over where the
  labs stop.
* **Guides to read** — the lab guides in this documentation.

## 2. "I want to…" task finder

Below the disciplines, 66 everyday engineering jobs — *size a battery, check
a part is strong enough, pick 3D-print settings, match an antenna, estimate
unit cost, plan a reliability test, route a board, design a filter, tune a
proportional–integral–derivative (PID) loop, pick a spring, check fit for
different head sizes, estimate schedule risk, generate firmware, export to
KiCad or Onshape* … — each mapped to the lab tab that does it with a one-line
how. Type to search (it is forgiving about word order and typos of four
letters or more) or filter by discipline.

## 3. How Protoforge works

Three steps, shown as one flow on the start page below your path:

1. **Describe** — the [new-product wizard](#/new) or the
   [Systems Studio](#/systems) turns what the product must do into
   requirements bound to metrics.
2. **Design & simulate** — change the design in the labs; every edit
   re-checks every requirement in the Rust engine.
3. **Build & ship** — the [Build Guide](#/build), the
   [Manufacturing Hub](#/mfg) and the [Web toolchain](#/toolchain) give
   settings, vendors, files and hand-off to real tools.

## 4. Simple and Expert mode

The switch in the top bar changes the sidebar:

* **Simple** (the default on a first visit) — *Start here* and *New
  product* at the top, then **Your labs** for your discipline, and every
  lab folded under **All labs (23)**. The lab help panel opens by itself the
  first time you visit each lab.
* **Expert** — the full grouped list of labs, as before; help opens only
  when you press **Help**.

Someone who already had a saved project before this feature existed starts
in Expert mode. The choice is stored in the browser (`protoforge.prefs.v1`).

## 5. Command palette (Ctrl K / ⌘ K)

Press **Ctrl K** (⌘ K on a Mac), **/** when you are not typing in a field,
or the **Search…** button in the top bar. Type anything:

| You type | You get |
|---|---|
| a lab or tab name — `rf`, `smith`, `weibull` | the lab, then its tabs (opens `#/<lab>?tab=<id>`) |
| a requirement — `R-047` | the requirement with its live status and value (opens it in the Systems Studio) |
| a metric — `battery life`, `ppd` | the live value; opens the lab that computes it |
| a term — `MTF`, `DRC` | the full form and definition right in the list |
| a guide — `architecture` | the page in Documentation |
| a job — `size a battery` | the "I want to…" task |
| an action — `theme`, `expert`, `save` | toggle theme, switch Simple/Expert, choose your discipline, save `project.json`, new product, reset to Iris (asks first) |

↑ and ↓ move, Enter opens, Esc closes and returns focus to where you were.
With an empty query the palette lists your recent items and suggestions.

## 6. Lab help

Every lab header has a **Help** button. The panel answers, in plain words:

* **The question this lab answers.**
* **What you change** — the inputs.
* **What you read** — the outputs, each with its good/bad range and, where
  the engine computes it, the live value with pass/fail.
* **Try this in 3 steps** — with links straight to the tabs.
* **Who it is for** — disciplines (each links to that path).
* **Whether it works on your product** — honestly labelled per lab:
  *Works on your product* (Power & Thermal, Manufacturing Hub, Program &
  Business read your project's bill of materials (BOM) and plan),
  *Analyses the Iris example geometry* (CAD, Optics, Thermal & Fluids, Human
  Factors, Molding, Print, PCB Studio and Prototype & Firmware model the
  glasses), *General-purpose tools* (Circuit, Chip, Build Guide, Quantum) or
  *General tools + Iris example* (RF, DSP, Control, Robotics, RL, Structures,
  Mechanisms, Reliability).

Esc or ✕ closes it. On a phone it is a bottom sheet.

## 7. Welcome card

The first time you open the Overview, a card offers three ways in: **Pick
your discipline**, **Describe your own product** or **Explore the Iris smart
glasses**. It never shows again once dismissed or once you have chosen a
discipline.

## 8. Glossary pop-ups

Terms on the start page and in the help panels are dotted-underlined: hover,
focus with the keyboard or tap to see the full form and definition, with a
link to where it is used. They come from the shared glossary
(`web/src/pages/learn/glossary.ts`) plus a supplement for terms it does not
have yet (`web/src/ux/data/terms.ts`).

## 9. Deep links to tabs

Every lab tab has an address, `#/<lab>?tab=<id>` — for example
`#/rf?tab=matching` or `#/optics?tab=quality`. The tab ids come from
`web/src/ux/data/labTabs.ts`, generated from the labs' source:

```bash
node scripts/gen-lab-index.mjs   # re-run after a lab adds or renames a tab
```

Labs that do not read `?tab=` themselves are switched by the navigation
chrome, which presses the matching tab for you.

## Where the code lives

| Piece | File |
|---|---|
| Preferences (role, mode, quests, recent items, help seen) | `web/src/ux/prefs.ts` |
| Disciplines, paths, quests | `web/src/ux/data/roles.ts` |
| "I want to…" tasks | `web/src/ux/data/tasks.ts` |
| Glossary supplement and look-up | `web/src/ux/data/terms.ts` |
| Start page, welcome card | `web/src/ux/start/` |
| Sidebar and top bar | `web/src/ux/nav/Chrome.tsx` |
| Command palette and fuzzy scorer | `web/src/ux/palette/` |
| Lab help data and panel, `<Term>` | `web/src/ux/help/` |
