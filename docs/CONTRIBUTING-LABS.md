# Building a Protoforge lab

Protoforge is a monorepo: a **Rust engine** (`crates/forge-core`) compiled to
**WebAssembly** (`crates/forge-wasm`) and driven by a **React + TypeScript**
front end (`web/`). A *lab* is one engineering discipline (circuits, DSP,
CAD, ...) and consists of:

| Piece | Location |
|---|---|
| Engine module | `crates/forge-core/src/<module>/` (`mod.rs` + any extra files) |
| Lab page | `web/src/labs/<lab>/index.tsx` (+ any extra files in that folder) |
| Guide | `docs/labs/<lab>.md` |

## Engine conventions

* **No new crates.** `forge-core` depends only on `serde` and `serde_json`
  so it builds for `wasm32-unknown-unknown`. Shared numerics live in
  `crate::math` (complex numbers `C64`, dense `solve`, `poly_roots`,
  `linspace`/`logspace`, `interp`, `rms`, deterministic `Rng`).
* Every module exposes
  ```rust
  pub fn dispatch(method: &str, input: &serde_json::Value) -> Option<crate::api::ApiResult>;
  pub fn metrics(p: &crate::project::Project) -> Vec<crate::project::Metric>;
  ```
  `dispatch` returns `None` for unknown methods. Use
  `crate::api::handle(input, |i: MyInput| Ok(my_fn(i)))` to parse input,
  run and serialise. Method names are `"<module>.<verb>"`, e.g.
  `dsp.fir_design`. Input/output are serde structs with `snake_case`
  fields; prefer `#[serde(default)]` on inputs so the UI can omit fields.
  Inputs that need the product take `project: Project` as a field.
* `metrics(p)` feeds requirement verification (`systems.evaluate`). It is
  re-run on **every project edit** in the browser, so keep each module's
  `metrics` under ~20 ms in release WebAssembly. Emit the exact metric keys
  and units listed for your module. Build metrics with
  `Metric::new(key, label, value, unit, source).with_note(...)`.
* Deterministic: seed randomness with `crate::math::Rng::new(seed)`.
* Unit tests in each module (`#[cfg(test)]`) — test the physics/maths
  against hand-calculable cases, and assert that the Iris reference project
  (`Project::iris()`) produces sensible metric values.
* Doc comments explain the maths and cite real references (books/papers by
  author, year, title; URLs only when certain). Never invent part numbers,
  prices presented as quotes, or vendor facts.
* `cargo test -p forge-core <module>` and `cargo clippy -p forge-core` should
  be clean.

## Front-end conventions

* Default-export a component from `web/src/labs/<lab>/index.tsx`, wrapped in
  `<LabPage kicker title subtitle metricPrefixes={['<module>.']}>` — this
  shows the live status of the requirements your metrics verify.
* Call the engine with `useCompute<T>('module.method', input)` (from
  `web/src/engine/hooks.ts`, synchronous WebAssembly under the hood, returns
  `{data, error, ms}`) or `call<T>()` from `web/src/engine/engine.ts` for
  event handlers.
* Read the product with `useProject((s) => s.project)`; edit design
  parameters with `useProject((s) => s.patchDesign)('frame', { bridge_mm: 20 })`.
  Types are in `web/src/engine/types.ts`.
* Build UI from `web/src/components`: `Card`, `Stat`, `Slider`,
  `NumberField`, `Select`, `Toggle`, `Tabs`, `Explainer` (teaching block
  with references), `Formula`, `Code` (copy + download), `DownloadButton`,
  `ErrorBox`, `Plot` (SVG line/step/scatter plots with hover, legend, log
  axes, reference lines) and `Scene3D` / `EngineMesh` / `Segments`
  (React Three Fiber). Layout helpers: `.grid .grid-2/3/4 .grid-sidebar
  .stack .row`.
* Colours only through CSS variables (`var(--series-1..8)`, `var(--text)`,
  `var(--muted)`, `var(--accent)`, `var(--good)`, `var(--bad)`,
  `var(--copper)` ...) so both dark and light themes work. Series colours
  are assigned in fixed order; status is never colour-only (icon + word).
* Animations use `requestAnimationFrame` with play/pause controls and
  respect `prefers-reduced-motion`.
* Every lab teaches: include `Explainer` blocks with the key equations and
  references, and tie the lab back to the Iris smart glasses.
* Offer downloads of what the lab produces (netlists, CSV, Verilog, STL,
  G-code, Gerbers ...).
* No new npm dependencies (available: react 19, react-router 7, three,
  @react-three/fiber, @react-three/drei, zustand, jszip).

## Build & check

```bash
./scripts/build-wasm.sh              # Rust -> web/src/wasm
cd web && npm ci && npm run build    # typecheck + production build
npx vite preview --port 4173         # or: python3 -m http.server 4173 -d dist
```
