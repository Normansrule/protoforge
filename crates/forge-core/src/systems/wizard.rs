//! New-product wizard: turns a plain-language product description (kind,
//! capabilities, targets) into a starter project whose requirements are bound
//! to metrics the engine really computes for that project.
//! (Scaffold — replaced by the wizard implementation.)
use serde_json::Value;

use crate::api::ApiResult;

pub fn dispatch(method: &str, _input: &Value) -> Option<ApiResult> {
    match method {
        "systems.wizard_options" => Some(Ok(Value::Array(vec![]))),
        _ => None,
    }
}
