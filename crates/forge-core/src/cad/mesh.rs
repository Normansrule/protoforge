//! Indexed triangle mesh shared by CAD, printing and molding.
//! Coordinates are millimetres, Z up. Triangles are counter-clockwise when
//! seen from outside (outward normals).
//!
//! Besides storage this file holds the mesh-level maths: rigid/affine
//! transforms (with winding repair for mirrors), merging, a 2-manifold
//! validator and mass properties by the divergence theorem.
//!
//! **Volume by the divergence theorem.** For a closed, outward-oriented
//! triangle mesh, ∭ ∇·F dV = ∯ F·n dA with F = (x, y, z)/3 gives
//! V = Σ_t (v0 · (v1 × v2)) / 6 — the sum of signed tetrahedra from the
//! origin to every face. The centroid follows from the same tetrahedra,
//! each weighted by its signed volume with centroid (v0 + v1 + v2) / 4.
//! Reference: C. Zhang, T. Chen, "Efficient feature extraction for 2D/3D
//! objects in mesh representation", Proc. IEEE ICIP 2001.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub type P3 = [f64; 3];

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Mesh {
    /// Flat `[x0, y0, z0, x1, y1, z1, ...]`.
    pub positions: Vec<f64>,
    /// Flat triangle list `[a0, b0, c0, a1, ...]`.
    pub indices: Vec<u32>,
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

#[inline]
pub fn v_sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
#[inline]
pub fn v_add(a: P3, b: P3) -> P3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
#[inline]
pub fn v_scale(a: P3, k: f64) -> P3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
#[inline]
pub fn v_cross(a: P3, b: P3) -> P3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
#[inline]
pub fn v_dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
#[inline]
pub fn v_len(a: P3) -> f64 {
    v_dot(a, a).sqrt()
}
pub fn v_norm(a: P3) -> P3 {
    let l = v_len(a);
    if l > 0.0 {
        v_scale(a, 1.0 / l)
    } else {
        a
    }
}

/// Affine map p ↦ M·p + t.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    pub m: [[f64; 3]; 3],
    pub t: P3,
}

impl Default for Affine {
    fn default() -> Self {
        Affine::IDENTITY
    }
}

impl Affine {
    pub const IDENTITY: Affine = Affine {
        m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        t: [0.0; 3],
    };
    pub fn translate(x: f64, y: f64, z: f64) -> Affine {
        Affine {
            t: [x, y, z],
            ..Affine::IDENTITY
        }
    }
    pub fn linear(m: [[f64; 3]; 3]) -> Affine {
        Affine { m, t: [0.0; 3] }
    }
    pub fn scale(sx: f64, sy: f64, sz: f64) -> Affine {
        Affine::linear([[sx, 0.0, 0.0], [0.0, sy, 0.0], [0.0, 0.0, sz]])
    }
    pub fn rot_x(deg: f64) -> Affine {
        let (s, c) = deg.to_radians().sin_cos();
        Affine::linear([[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]])
    }
    pub fn rot_y(deg: f64) -> Affine {
        let (s, c) = deg.to_radians().sin_cos();
        Affine::linear([[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]])
    }
    pub fn rot_z(deg: f64) -> Affine {
        let (s, c) = deg.to_radians().sin_cos();
        Affine::linear([[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]])
    }
    /// `self ∘ other` (apply `other` first).
    pub fn then(&self, after: &Affine) -> Affine {
        let mut m = [[0.0; 3]; 3];
        for (i, row) in m.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = (0..3).map(|k| after.m[i][k] * self.m[k][j]).sum();
            }
        }
        let t = v_add(after.apply_linear(self.t), after.t);
        Affine { m, t }
    }
    pub fn apply(&self, p: P3) -> P3 {
        v_add(self.apply_linear(p), self.t)
    }
    pub fn apply_linear(&self, p: P3) -> P3 {
        let m = &self.m;
        [
            m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2],
            m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2],
            m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2],
        ]
    }
    pub fn det(&self) -> f64 {
        let m = &self.m;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }
}

/// Result of [`Mesh::validate`].
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MeshCheck {
    /// Every edge is shared by exactly two triangles …
    pub manifold: bool,
    /// … which traverse it in opposite directions (consistent winding).
    pub oriented: bool,
    /// manifold && oriented && positive volume ⇒ closed solid, normals out.
    pub watertight: bool,
    pub boundary_edges: usize,
    pub nonmanifold_edges: usize,
    pub misoriented_edges: usize,
    pub degenerate_triangles: usize,
    pub shells: usize,
    pub signed_volume_mm3: f64,
}

/// Mass properties of a closed mesh.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MassProps {
    pub volume_mm3: f64,
    pub volume_cm3: f64,
    pub area_mm2: f64,
    pub mass_g: f64,
    pub centroid: P3,
    pub bbox_min: P3,
    pub bbox_max: P3,
    pub size: P3,
    pub triangles: usize,
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
    pub fn add_vertex(&mut self, p: P3) -> u32 {
        let i = self.vertex_count() as u32;
        self.positions.extend_from_slice(&p);
        i
    }
    pub fn add_tri(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend_from_slice(&[a, b, c]);
    }
    /// Triangle `t` as three points.
    pub fn tri(&self, t: usize) -> [P3; 3] {
        let i = &self.indices[t * 3..t * 3 + 3];
        [self.vertex(i[0]), self.vertex(i[1]), self.vertex(i[2])]
    }
    /// Unit face normal of triangle `t` (zero for degenerate faces).
    pub fn face_normal(&self, t: usize) -> P3 {
        let [a, b, c] = self.tri(t);
        v_norm(v_cross(v_sub(b, a), v_sub(c, a)))
    }
    /// Append another mesh (no welding: shells stay separate).
    pub fn append(&mut self, other: &Mesh) {
        let base = self.vertex_count() as u32;
        self.positions.extend_from_slice(&other.positions);
        self.indices.extend(other.indices.iter().map(|i| i + base));
    }
    pub fn merged(meshes: &[Mesh]) -> Mesh {
        let mut m = Mesh::default();
        for x in meshes {
            m.append(x);
        }
        m
    }
    /// Apply an affine map; mirrors (det < 0) flip the winding so normals
    /// stay outward.
    pub fn transformed(&self, a: &Affine) -> Mesh {
        let mut out = Mesh {
            positions: Vec::with_capacity(self.positions.len()),
            indices: self.indices.clone(),
        };
        for k in 0..self.vertex_count() {
            let p = a.apply(self.vertex(k as u32));
            out.positions.extend_from_slice(&p);
        }
        if a.det() < 0.0 {
            out.flip();
        }
        out
    }
    pub fn translated(&self, x: f64, y: f64, z: f64) -> Mesh {
        self.transformed(&Affine::translate(x, y, z))
    }
    /// Reverse every triangle.
    pub fn flip(&mut self) {
        for t in self.indices.chunks_mut(3) {
            t.swap(1, 2);
        }
    }
    pub fn bbox(&self) -> (P3, P3) {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in self.positions.chunks(3) {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        if self.positions.is_empty() {
            return ([0.0; 3], [0.0; 3]);
        }
        (lo, hi)
    }
    /// Signed volume, Σ v0·(v1×v2)/6.
    pub fn volume(&self) -> f64 {
        (0..self.triangle_count())
            .map(|t| {
                let [a, b, c] = self.tri(t);
                v_dot(a, v_cross(b, c))
            })
            .sum::<f64>()
            / 6.0
    }
    pub fn area(&self) -> f64 {
        (0..self.triangle_count())
            .map(|t| {
                let [a, b, c] = self.tri(t);
                0.5 * v_len(v_cross(v_sub(b, a), v_sub(c, a)))
            })
            .sum()
    }
    /// Volume, area, centroid, bounding box and mass (`density` in g/cm³).
    pub fn mass_props(&self, density_g_cm3: f64) -> MassProps {
        let (lo, hi) = self.bbox();
        // Use a local origin (bbox centre) to limit cancellation error.
        let o = v_scale(v_add(lo, hi), 0.5);
        let mut vol = 0.0;
        let mut c = [0.0; 3];
        let mut area = 0.0;
        for t in 0..self.triangle_count() {
            let [a, b, cc] = self.tri(t);
            let (a, b, cc) = (v_sub(a, o), v_sub(b, o), v_sub(cc, o));
            let v = v_dot(a, v_cross(b, cc)) / 6.0;
            vol += v;
            let s = v_add(v_add(a, b), cc);
            c = v_add(c, v_scale(s, v / 4.0));
            area += 0.5 * v_len(v_cross(v_sub(b, a), v_sub(cc, a)));
        }
        let centroid = if vol.abs() > 1e-12 { v_add(v_scale(c, 1.0 / vol), o) } else { o };
        MassProps {
            volume_mm3: vol,
            volume_cm3: vol / 1000.0,
            area_mm2: area,
            mass_g: vol / 1000.0 * density_g_cm3,
            centroid,
            bbox_min: lo,
            bbox_max: hi,
            size: v_sub(hi, lo),
            triangles: self.triangle_count(),
        }
    }
    /// Number of connected triangle shells (union-find over shared vertices).
    pub fn shell_count(&self) -> usize {
        let n = self.vertex_count();
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(p: &mut [usize], mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        for t in self.indices.chunks(3) {
            let a = find(&mut parent, t[0] as usize);
            for &o in &t[1..] {
                let b = find(&mut parent, o as usize);
                if a != b {
                    parent[b] = a;
                }
            }
        }
        let mut used = vec![false; n];
        for &i in &self.indices {
            used[i as usize] = true;
        }
        let mut roots = std::collections::HashSet::new();
        for (i, &u) in used.iter().enumerate() {
            if u {
                roots.insert(find(&mut parent, i));
            }
        }
        roots.len()
    }
    /// 2-manifold check: every undirected edge used by exactly two
    /// triangles in opposite directions, plus positive signed volume.
    pub fn validate(&self) -> MeshCheck {
        let mut edges: HashMap<(u32, u32), (u32, i32)> = HashMap::with_capacity(self.indices.len());
        let mut degenerate = 0;
        for t in 0..self.triangle_count() {
            let tri = &self.indices[t * 3..t * 3 + 3];
            let [a, b, c] = self.tri(t);
            if v_len(v_cross(v_sub(b, a), v_sub(c, a))) < 1e-12 {
                degenerate += 1;
            }
            for k in 0..3 {
                let (u, v) = (tri[k], tri[(k + 1) % 3]);
                let key = (u.min(v), u.max(v));
                let e = edges.entry(key).or_insert((0, 0));
                e.0 += 1;
                e.1 += if u < v { 1 } else { -1 };
            }
        }
        let mut boundary = 0;
        let mut nonmanifold = 0;
        let mut misoriented = 0;
        for (_, (count, dir)) in edges {
            match count {
                1 => boundary += 1,
                2 => {
                    if dir != 0 {
                        misoriented += 1
                    }
                }
                _ => nonmanifold += 1,
            }
        }
        let vol = self.volume();
        let manifold = boundary == 0 && nonmanifold == 0;
        let oriented = misoriented == 0;
        MeshCheck {
            manifold,
            oriented,
            watertight: manifold && oriented && vol > 0.0,
            boundary_edges: boundary,
            nonmanifold_edges: nonmanifold,
            misoriented_edges: misoriented,
            degenerate_triangles: degenerate,
            shells: self.shell_count(),
            signed_volume_mm3: vol,
        }
    }
    /// Compact single-precision copy for the browser.
    pub fn to_f32(&self) -> MeshF32 {
        MeshF32 {
            positions: self.positions.iter().map(|&v| ((v * 1e4).round() / 1e4) as f32).collect(),
            indices: self.indices.clone(),
        }
    }
}

/// Mesh with `f32` coordinates (smaller JSON for the web view).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MeshF32 {
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
}

impl MeshF32 {
    pub fn to_mesh(&self) -> Mesh {
        Mesh {
            positions: self.positions.iter().map(|&v| v as f64).collect(),
            indices: self.indices.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn unit_cube() -> Mesh {
        let v = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ];
        let positions = v.iter().flat_map(|p: &[f64; 3]| p.iter().copied()).collect();
        let indices = vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0, 4, 3, 4, 7,
        ];
        Mesh { positions, indices }
    }

    #[test]
    fn cube_is_watertight_with_unit_volume() {
        let m = unit_cube();
        let c = m.validate();
        assert!(c.watertight, "{c:?}");
        assert_eq!(c.shells, 1);
        let mp = m.mass_props(1.0);
        assert!((mp.volume_mm3 - 1.0).abs() < 1e-12);
        assert!((mp.area_mm2 - 6.0).abs() < 1e-12);
        assert!(mp.centroid.iter().all(|&x| (x - 0.5).abs() < 1e-12));
    }

    #[test]
    fn mirror_keeps_normals_outward() {
        let m = unit_cube().transformed(&Affine::scale(-1.0, 1.0, 1.0));
        assert!(m.validate().watertight);
        assert!((m.volume() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn flipped_cube_is_detected() {
        let mut m = unit_cube();
        m.flip();
        let c = m.validate();
        assert!(c.manifold && c.oriented && !c.watertight);
        // A single reversed triangle breaks orientation.
        let mut m = unit_cube();
        m.indices.swap(1, 2);
        assert!(!m.validate().oriented);
        // Removing a triangle opens the mesh.
        let mut m = unit_cube();
        m.indices.truncate(33);
        assert!(!m.validate().manifold);
    }

    #[test]
    fn affine_composition() {
        let a = Affine::rot_z(90.0).then(&Affine::translate(1.0, 0.0, 0.0));
        let p = a.apply([1.0, 0.0, 0.0]);
        assert!((p[0] - 1.0).abs() < 1e-12 && (p[1] - 1.0).abs() < 1e-12);
    }
}
