#!/usr/bin/env bash
# Build the Rust engine to WebAssembly and generate JS bindings into web/src/wasm.
#
#   ./scripts/build-wasm.sh            # release build
#
# Needs: rustup target add wasm32-unknown-unknown
#        cargo install wasm-bindgen-cli --version 0.2.129
# If the wasm32 target cannot be installed (offline toolchains), the script
# falls back to building std from source (needs the rust-src component).
set -euo pipefail
cd "$(dirname "$0")/.."

EXTRA=()
if ! rustup target list --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown$'; then
  echo "note: wasm32 target not installed, building std from source (-Z build-std)"
  export RUSTC_BOOTSTRAP=1
  EXTRA=(-Z build-std=std,panic_abort)
fi

cargo build -p forge-wasm --release --target wasm32-unknown-unknown "${EXTRA[@]}"
wasm-bindgen --target web --out-dir web/src/wasm \
  target/wasm32-unknown-unknown/release/forge_wasm.wasm
ls -lh web/src/wasm/*.wasm
