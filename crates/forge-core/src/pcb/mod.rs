//! `pcb` module (stub — to be implemented). The public signature
//! [`placement`] is used by the robotics pick-and-place planner and must be kept.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::ApiResult;
use crate::project::{Metric, Project};

/// A placed component (board coordinates in mm, origin bottom-left).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Placed {
    pub refdes: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub rot_deg: f64,
    /// "top" or "bottom"
    pub side: String,
}

/// Component placement for every `on_pcb` component. Placeholder: a row.
pub fn placement(p: &Project) -> Vec<Placed> {
    let parts: Vec<_> = p.components.iter().filter(|c| c.on_pcb).collect();
    let w = p.design.pcb.width_mm;
    let n = parts.len().max(1) as f64;
    parts
        .iter()
        .enumerate()
        .map(|(i, c)| Placed {
            refdes: c.refdes.clone(),
            x_mm: w * (i as f64 + 0.5) / n,
            y_mm: p.design.pcb.height_mm / 2.0,
            rot_deg: 0.0,
            side: "top".into(),
        })
        .collect()
}

/// JSON API entry point for `pcb.*` methods.
pub fn dispatch(_method: &str, _input: &Value) -> Option<ApiResult> {
    None
}

/// Metrics this module contributes to requirement verification.
pub fn metrics(_p: &Project) -> Vec<Metric> {
    vec![]
}
