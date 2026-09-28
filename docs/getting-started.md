# Getting started

This guide sets up Protoforge on **Ubuntu** — natively or under the Windows
Subsystem for Linux 2 (WSL2) — builds the Rust engine to WebAssembly, runs the
web app, the command-line interface (CLI) and the local representational
state transfer (REST) server, and deploys the site to GitHub Pages.

Just want to use it? Open the published site — everything runs in the
browser, nothing to install.

## 1. Prerequisites

| Tool | Version | Why |
|---|---|---|
| Rust (via rustup) | stable (1.85 or newer) | the engine, CLI and server |
| `wasm32-unknown-unknown` target | — | compile the engine to WebAssembly |
| `wasm-bindgen-cli` | **exactly 0.2.129** | generate the JavaScript bindings (must match the crate) |
| Node.js | 22 LTS | build and serve the React front end |
| git, build-essential, pkg-config | — | compilers and linkers for native crates |
| ngspice (optional) | any recent | cross-check Circuit Lab decks through the REST server |

### WSL2 notes

* Install Ubuntu from a Windows terminal with `wsl --install -d Ubuntu`, then
  open the Ubuntu shell for everything below.
* Keep the repository **inside the Linux file system** (e.g. `~/code/protoforge`),
  not under `/mnt/c/…` — builds and `npm ci` are many times faster there.
* Servers started in WSL2 on `localhost` are reachable from the Windows
  browser at the same `http://localhost:<port>` address.
* Visual Studio Code with the WSL extension (`code .` from the Ubuntu shell)
  edits the Linux files directly.

### Install the toolchain

```bash
sudo apt update
sudo apt install -y build-essential pkg-config git curl python3

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup target add wasm32-unknown-unknown
cargo install --locked wasm-bindgen-cli --version 0.2.129

# Node.js 22 via nvm (no sudo needed)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash
source "$HOME/.nvm/nvm.sh"
nvm install 22

# optional: a real SPICE for cross-checking
sudo apt install -y ngspice
```

Check: `rustc --version`, `wasm-bindgen --version` (must print 0.2.129),
`node --version` (v22.x).

## 2. Build and run the web app

```bash
git clone https://github.com/Normansrule/protoforge.git
cd protoforge

./scripts/build-wasm.sh          # Rust engine → web/src/wasm (release, ~1 min the first time)
cd web
npm ci                           # exact dependency versions from package-lock.json
npm run dev                      # Vite dev server → http://localhost:5173
```

The dev server hot-reloads TypeScript and CSS. After changing **Rust** code,
re-run `./scripts/build-wasm.sh` — Vite picks up the new `.wasm`.

Production build and preview:

```bash
npm run build                    # type-check (tsc) + vite build → web/dist
npx vite preview --port 4173     # or: python3 -m http.server 4173 -d dist
```

## 3. Run the command-line interface

The `protoforge` CLI runs the same analyses natively. Without a project path it
uses the built-in Iris smart-glasses project.

```bash
cargo run --release -p forge-cli -- evaluate          # verify every requirement (exit 1 on any FAIL)
cargo run --release -p forge-cli -- metrics           # every computed metric with its note
cargo run --release -p forge-cli -- report > report.md
cargo run --release -p forge-cli -- package --out out/iris   # the complete fabrication package
cargo run --release -p forge-cli -- call pcb.ipc2221 '{"current_a":0.5,"temp_rise_c":10,"copper_oz":1,"external":true}'
cargo run --release -p forge-cli -- new "Robot lamp" > robot-lamp.json
cargo run --release -p forge-cli -- evaluate robot-lamp.json
```

Install it on your `PATH` with `cargo install --path crates/forge-cli`, then
simply `protoforge evaluate`.

## 4. Run the REST server

```bash
cargo run --release -p forge-server -- --port 8787                    # API only
cargo run --release -p forge-server -- --port 8787 --static web/dist  # API + the built web app
curl localhost:8787/api/health
curl -o iris-fab-package.zip localhost:8787/api/package
```

All routes, options and more examples (including Python) are in
[crates/forge-server/README.md](../crates/forge-server/README.md).

## 5. Test and lint

```bash
cargo fmt --all                                   # rustfmt.toml: max_width = 140
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                            # engine, CLI and server tests
cd web && npm run typecheck
```

These are exactly the checks continuous integration (CI) runs on every push
(`.github/workflows/ci.yml`), plus `protoforge evaluate` and the fabrication
package artifact.

## 6. Deploy to GitHub Pages

1. Push the repository to GitHub (e.g. `Normansrule/protoforge`).
2. In the repository: **Settings → Pages → Build and deployment → Source:
   GitHub Actions**. (This is a one-time switch; without it the deploy job
   fails with a "Pages not enabled" error.)
3. Push to `main` — or run **Actions → Deploy to GitHub Pages → Run
   workflow**. The workflow builds the engine to WebAssembly, builds the web
   app and publishes `web/dist`.
4. The site appears at `https://<user>.github.io/<repo>/`. Hash routing
   (`#/circuit`) and Vite's relative `base: './'` mean no further
   configuration is needed for a project sub-path.

Every push and pull request also runs **CI**; download the Iris fabrication
package from the run's **Artifacts → iris-fab-package**.

## Troubleshooting

| Symptom | Fix |
|---|---|
| `wasm-bindgen` schema/version mismatch error | Install exactly 0.2.129: `cargo install --locked --force wasm-bindgen-cli --version 0.2.129`. |
| `can't find crate for core` / target missing | `rustup target add wasm32-unknown-unknown` (offline toolchains: the script falls back to `-Z build-std`, which needs `rustup component add rust-src`). |
| The web app says "loading engine…" forever | `web/src/wasm/` is missing — run `./scripts/build-wasm.sh`. |
| `npm ci` is very slow on WSL2 | Move the checkout from `/mnt/c/…` into your Linux home directory. |
| `POST /api/ngspice` returns 501 | Install ngspice (`sudo apt install ngspice`) and restart the server. |

Next: read [architecture.md](architecture.md), then a lab guide in
[docs/labs/](labs/), and the in-app **Guide & references** page.
