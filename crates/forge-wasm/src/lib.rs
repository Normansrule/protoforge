//! WebAssembly bridge: one generic entry point keeps the JS <-> Rust surface
//! tiny. The front end calls `call("dsp.fft", "{...}")` and gets a JSON
//! envelope back (`{"ok": ...}` or `{"error": "..."}`).

use wasm_bindgen::prelude::*;

/// Call any forge-core method. See `forge_core::api`.
#[wasm_bindgen]
pub fn call(method: &str, input_json: &str) -> String {
    forge_core::api::call(method, input_json)
}

/// Engine version string.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
