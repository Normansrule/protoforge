//! "What should I do next?": ranks failing, marginal and unverified
//! requirements and lint issues into concrete next actions, each pointing at
//! the lab tab and the design parameters that move it.
//! (Scaffold — replaced by the next-actions implementation.)
use serde_json::Value;

use crate::api::ApiResult;

pub fn dispatch(method: &str, _input: &Value) -> Option<ApiResult> {
    match method {
        "systems.next_actions" => Some(Ok(Value::Array(vec![]))),
        _ => None,
    }
}
