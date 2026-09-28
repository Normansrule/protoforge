//! `print` module (stub — to be implemented).

use serde_json::Value;

use crate::api::ApiResult;
use crate::project::{Metric, Project};

/// JSON API entry point for `print.*` methods.
pub fn dispatch(_method: &str, _input: &Value) -> Option<ApiResult> {
    None
}

/// Metrics this module contributes to requirement verification.
pub fn metrics(_p: &Project) -> Vec<Metric> {
    vec![]
}
