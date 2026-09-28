//! Indexed triangle mesh shared by CAD, printing and molding.
//! Coordinates are millimetres, Z up. Triangles are counter-clockwise when
//! seen from outside (outward normals).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Mesh {
    /// Flat `[x0, y0, z0, x1, y1, z1, ...]`.
    pub positions: Vec<f64>,
    /// Flat triangle list `[a0, b0, c0, a1, ...]`.
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
    pub fn vertex(&self, i: u32) -> [f64; 3] {
        let k = i as usize * 3;
        [self.positions[k], self.positions[k + 1], self.positions[k + 2]]
    }
}

/// A named printable / moldable part of the product.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Part {
    pub name: String,
    pub mesh: Mesh,
    /// Suggested display colour (hex).
    #[serde(default)]
    pub color: String,
    /// Offset used by the exploded assembly view, mm.
    #[serde(default)]
    pub explode: [f64; 3],
}
