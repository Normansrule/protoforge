## What does this change?

<!-- One or two sentences. Link the issue if there is one. -->

## Which labs / crates are affected?

- [ ] Engine (`crates/forge-core`) — module: 
- [ ] Web (`web/`) — page/lab: 
- [ ] CLI / REST server
- [ ] Docs / CI

## How was it verified?

- [ ] `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `./scripts/build-wasm.sh && (cd web && npm run build)`
- [ ] `cargo run --release -p forge-cli -- evaluate` still passes every Iris requirement
- [ ] Checked the page in dark and light theme and at phone width (390 px)

## Numbers and references

<!-- New or changed physics/maths: hand-calculated check, cross-check against a
professional tool, and the reference (author, year, title). -->

## Screenshots

<!-- Before / after for UI changes. -->
