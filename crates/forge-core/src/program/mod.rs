//! # Program & Business Lab engine — Program and business: schedule, risk, earned value, unit economics, readiness
//!
//! Scaffold: replace with the real module.

use serde_json::Value;

use crate::api::ApiResult;
use crate::project::{Metric, Project};

/// JSON API for the Program & Business Lab.
pub fn dispatch(_method: &str, _input: &Value) -> Option<ApiResult> {
    None
}

/// Metrics feeding requirement verification.
pub fn metrics(_p: &Project) -> Vec<Metric> {
    Vec::new()
}
