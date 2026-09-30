//! # Mechanisms Lab engine — Mechanisms: linkages, springs, gears, cams, hinge design
//!
//! Scaffold: replace with the real module.

use serde_json::Value;

use crate::api::ApiResult;
use crate::project::{Metric, Project};

/// JSON API for the Mechanisms Lab.
pub fn dispatch(_method: &str, _input: &Value) -> Option<ApiResult> {
    None
}

/// Metrics feeding requirement verification.
pub fn metrics(_p: &Project) -> Vec<Metric> {
    Vec::new()
}
