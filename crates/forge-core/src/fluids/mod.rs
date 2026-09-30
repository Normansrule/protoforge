//! # Thermal & Fluids Lab engine — Heat transfer and fluid flow: 3-D conduction, convection, lattice-Boltzmann flow
//!
//! Scaffold: replace with the real module.

use serde_json::Value;

use crate::api::ApiResult;
use crate::project::{Metric, Project};

/// JSON API for the Thermal & Fluids Lab.
pub fn dispatch(_method: &str, _input: &Value) -> Option<ApiResult> {
    None
}

/// Metrics feeding requirement verification.
pub fn metrics(_p: &Project) -> Vec<Metric> {
    Vec::new()
}
