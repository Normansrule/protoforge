//! `cad` module (stub — to be implemented). The public signatures
//! [`Mesh`], [`Part`] and [`frame_parts`] are used by `print` and `molding`
//! and must be kept.

use serde_json::Value;

use crate::api::ApiResult;
use crate::project::{Metric, Project};

pub mod mesh;
pub use mesh::{Mesh, Part};

/// All printable/moldable parts of the product, positioned in assembly
/// coordinates (mm, Z up). Placeholder: a single box.
pub fn frame_parts(p: &Project) -> Vec<Part> {
    let f = &p.design.frame;
    let w = f.lens_width_mm * 2.0 + f.bridge_mm + 2.0 * f.rim_width_mm;
    let (h, t) = (f.lens_height_mm + 2.0 * f.rim_width_mm, f.front_thickness_mm);
    let v = [
        [0.0, 0.0, 0.0], [w, 0.0, 0.0], [w, t, 0.0], [0.0, t, 0.0],
        [0.0, 0.0, h], [w, 0.0, h], [w, t, h], [0.0, t, h],
    ];
    let positions = v.iter().flat_map(|p| p.iter().copied()).collect();
    let indices = vec![0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0, 4, 3, 4, 7];
    vec![Part { name: "front".into(), mesh: Mesh { positions, indices }, color: "#9aa7b8".into(), explode: [0.0; 3] }]
}

/// JSON API entry point for `cad.*` methods.
pub fn dispatch(_method: &str, _input: &Value) -> Option<ApiResult> {
    None
}

/// Metrics this module contributes to requirement verification.
pub fn metrics(_p: &Project) -> Vec<Metric> {
    vec![]
}
