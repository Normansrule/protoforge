//! JSON API shared by the WebAssembly bridge, the REST server and the CLI.
//!
//! Every engine capability is addressed by a dotted method name such as
//! `circuit.simulate` or `dsp.fir_design`. Input and output are JSON so the
//! exact same call works from JavaScript (`engine.call("dsp.fft", {...})`),
//! over HTTP (`POST /api/dsp.fft`) or from Rust.
//!
//! Each module owns a `dispatch(method, input)` function that returns
//! `None` when it does not recognise the method. New modules plug in by
//! adding one line to [`DISPATCHERS`].

use serde::Serialize;
use serde_json::{json, Value};

/// Result type used by module dispatchers.
pub type ApiResult = Result<Value, String>;
/// Signature every module dispatcher implements.
pub type Dispatcher = fn(&str, &Value) -> Option<ApiResult>;

/// Ordered list of module dispatchers. The prefix before the first `.` of a
/// method name selects the module.
pub const DISPATCHERS: &[(&str, Dispatcher)] = &[
    ("project", crate::project::dispatch),
    ("systems", crate::systems::dispatch),
    ("circuit", crate::circuit::dispatch),
    ("power", crate::power::dispatch),
    ("dsp", crate::dsp::dispatch),
    ("chip", crate::chip::dispatch),
    ("control", crate::control::dispatch),
    ("robotics", crate::robotics::dispatch),
    ("rl", crate::rl::dispatch),
    ("optics", crate::optics::dispatch),
    ("cad", crate::cad::dispatch),
    ("print", crate::print::dispatch),
    ("pcb", crate::pcb::dispatch),
    ("molding", crate::molding::dispatch),
    ("mfg", crate::mfg::dispatch),
];

include!(concat!(env!("OUT_DIR"), "/catalog.rs"));

/// Module name → method names, from the generated [`METHODS`] list.
pub fn catalog() -> serde_json::Map<String, Value> {
    let mut map = serde_json::Map::new();
    for (name, _) in DISPATCHERS {
        let ms: Vec<Value> = METHODS
            .iter()
            .filter(|m| m.split('.').next() == Some(name))
            .map(|m| json!(m))
            .collect();
        map.insert((*name).to_string(), Value::Array(ms));
    }
    map
}

/// Call an engine method with a JSON value and get a JSON value back.
pub fn call_value(method: &str, input: &Value) -> ApiResult {
    if method == "api.methods" {
        return Ok(json!(list_modules()));
    }
    if method == "api.catalog" {
        return Ok(Value::Object(catalog()));
    }
    let module = method.split('.').next().unwrap_or("");
    for (name, dispatcher) in DISPATCHERS {
        if *name == module {
            return match dispatcher(method, input) {
                Some(r) => r,
                None => Err(format!("unknown method `{method}`")),
            };
        }
    }
    Err(format!("unknown module `{module}` (method `{method}`)"))
}

/// String-in / string-out wrapper. Always returns a JSON envelope:
/// `{"ok": <value>}` or `{"error": "<message>"}`.
pub fn call(method: &str, input_json: &str) -> String {
    let input: Value = if input_json.trim().is_empty() {
        Value::Null
    } else {
        match serde_json::from_str(input_json) {
            Ok(v) => v,
            Err(e) => return json!({ "error": format!("invalid JSON input: {e}") }).to_string(),
        }
    };
    match call_value(method, &input) {
        Ok(v) => json!({ "ok": v }).to_string(),
        Err(e) => json!({ "error": e }).to_string(),
    }
}

fn list_modules() -> Vec<&'static str> {
    DISPATCHERS.iter().map(|(n, _)| *n).collect()
}

/// Helper: deserialize dispatcher input into a typed struct.
pub fn parse<T: serde::de::DeserializeOwned>(input: &Value) -> Result<T, String> {
    serde_json::from_value(input.clone()).map_err(|e| format!("bad input: {e}"))
}

/// Helper: serialize any result into a JSON value.
pub fn to_value<T: Serialize>(v: T) -> ApiResult {
    serde_json::to_value(v).map_err(|e| format!("serialize error: {e}"))
}

/// Helper used by dispatchers: parse input as `I`, run `f`, serialize output.
pub fn handle<I, O, F>(input: &Value, f: F) -> Option<ApiResult>
where
    I: serde::de::DeserializeOwned,
    O: Serialize,
    F: FnOnce(I) -> Result<O, String>,
{
    Some(parse::<I>(input).and_then(f).and_then(to_value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_module_is_an_error() {
        let out = call("nope.method", "{}");
        assert!(out.contains("error"));
    }

    #[test]
    fn catalog_lists_real_methods() {
        let c = catalog();
        assert!(c["systems"].as_array().unwrap().iter().any(|m| m == "systems.evaluate"));
        assert!(c["circuit"].as_array().unwrap().len() >= 3);
        // every catalogued method is actually recognised by its dispatcher
        for m in METHODS {
            let r = call_value(m, &Value::Null);
            if let Err(e) = r {
                assert!(!e.starts_with("unknown method"), "{m}: {e}");
            }
        }
    }

    #[test]
    fn lists_modules() {
        let out = call("api.methods", "");
        assert!(out.contains("circuit"));
    }
}
