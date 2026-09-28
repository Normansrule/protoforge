//! 3D solid builders on top of the 2D kernel. Everything returned here is a
//! closed, outward-oriented 2-manifold (checked by the tests with
//! [`Mesh::validate`]).
//!
//! * [`loft_region`] — the workhorse: a polygon-with-holes stacked through a
//!   list of levels, each level an offset of the base outline. Two levels
//!   with a linear offset give an **extrusion with draft** (tapered walls for
//!   injection molding); a quarter-circle offset profile gives a fillet.
//! * [`sweep`] — a 2D profile swept along a 3D polyline with rotation-
//!   minimising frames (double-reflection method of W. Wang, B. Jüttler,
//!   D. Zheng, Y. Liu, "Computation of rotation minimizing frames", ACM TOG
//!   27(1), 2008).
//! * boxes, rounded boxes, cylinders, tubes and a closed double-walled
//!   hollow shell.
//!
//! Draft: a wall tapered by angle α shrinks the outline by `Δz·tan α` per
//! height `Δz`; the part then releases from the mold when pulled along +Z.

use super::geom2d::{self as g, P2};
use super::mesh::{v_add, v_cross, v_dot, v_norm, v_scale, v_sub, Mesh, P3};

/// A planar region: one counter-clockwise outer loop and clockwise holes.
#[derive(Clone, Debug, Default)]
pub struct Region {
    pub outer: Vec<P2>,
    pub holes: Vec<Vec<P2>>,
}

impl Region {
    pub fn new(outer: Vec<P2>, holes: Vec<Vec<P2>>) -> Region {
        Region {
            outer: g::oriented(outer, true),
            holes: holes.into_iter().map(|h| g::oriented(h, false)).collect(),
        }
    }
    pub fn simple(outer: Vec<P2>) -> Region {
        Region::new(outer, vec![])
    }
    /// Net area (outer minus holes).
    pub fn area(&self) -> f64 {
        g::area(&self.outer) + self.holes.iter().map(|h| g::area(h)).sum::<f64>()
    }
    /// Vertex-wise offset: `d_outer` for the outer loop, `d_holes` for the
    /// holes (positive grows the material region).
    pub fn offset(&self, d_outer: f64, d_holes: f64) -> Region {
        Region {
            outer: g::offset_vertexwise(&self.outer, d_outer, 3.0),
            holes: self.holes.iter().map(|h| g::offset_vertexwise(h, d_holes, 3.0)).collect(),
        }
    }
    pub fn loops(&self) -> Vec<Vec<P2>> {
        let mut v = vec![self.outer.clone()];
        v.extend(self.holes.iter().cloned());
        v
    }
}

/// One loft level: height, outer-loop offset, hole offset.
#[derive(Clone, Copy, Debug)]
pub struct Level {
    pub z: f64,
    pub d_outer: f64,
    pub d_holes: f64,
}

impl Level {
    pub fn new(z: f64, d: f64) -> Level {
        Level { z, d_outer: d, d_holes: d }
    }
}

/// Stack a region through `levels` (z increasing) into a closed solid.
pub fn loft_region(region: &Region, levels: &[Level]) -> Mesh {
    let mut mesh = Mesh::default();
    if levels.len() < 2 || region.outer.len() < 3 {
        return mesh;
    }
    let loops = region.loops();
    let sizes: Vec<usize> = loops.iter().map(|l| l.len()).collect();
    let per_level: usize = sizes.iter().sum();
    // Rings
    let mut first_ring: Vec<P2> = Vec::with_capacity(per_level);
    for (li, lev) in levels.iter().enumerate() {
        for (k, lp) in loops.iter().enumerate() {
            let d = if k == 0 { lev.d_outer } else { lev.d_holes };
            let ring = g::offset_vertexwise(lp, d, 3.0);
            for p in &ring {
                mesh.add_vertex([p[0], p[1], lev.z]);
                if li == 0 {
                    first_ring.push(*p);
                }
            }
        }
    }
    // Side walls
    for li in 0..levels.len() - 1 {
        let b0 = (li * per_level) as u32;
        let t0 = ((li + 1) * per_level) as u32;
        let mut off = 0u32;
        for &n in &sizes {
            let n = n as u32;
            for k in 0..n {
                let k1 = (k + 1) % n;
                let (a, b) = (b0 + off + k, b0 + off + k1);
                let (c, d) = (t0 + off + k1, t0 + off + k);
                mesh.add_tri(a, b, c);
                mesh.add_tri(a, c, d);
            }
            off += n;
        }
    }
    // Caps: each triangulated from its own ring (offsets move vertices, so
    // reusing one triangulation could flip thin triangles).
    let last = levels.len() - 1;
    let top_ring: Vec<P2> = (0..per_level)
        .map(|k| {
            let v = (last * per_level + k) as u32;
            let q = mesh.vertex(v);
            [q[0], q[1]]
        })
        .collect();
    let cap = |ring: &[P2]| -> Vec<[u32; 3]> {
        let mut holes: Vec<Vec<P2>> = Vec::new();
        let mut off = sizes[0];
        for &n in &sizes[1..] {
            holes.push(ring[off..off + n].to_vec());
            off += n;
        }
        g::triangulate(&ring[..sizes[0]], &holes)
    };
    for t in cap(&first_ring) {
        mesh.add_tri(t[0], t[2], t[1]); // bottom faces -Z
    }
    let top = (last * per_level) as u32;
    for t in cap(&top_ring) {
        mesh.add_tri(top + t[0], top + t[1], top + t[2]);
    }
    mesh
}

/// Straight extrusion between `z0` and `z1`.
pub fn extrude(region: &Region, z0: f64, z1: f64) -> Mesh {
    loft_region(region, &[Level::new(z0, 0.0), Level::new(z1, 0.0)])
}

/// Extrusion with draft: the region is offset by `-(z - z_ref)·tan(draft)`
/// at every height, so walls taper inwards going up and holes open up.
/// Stacked shells that share `z_ref` continue each other's walls exactly.
pub fn extrude_draft(region: &Region, z0: f64, z1: f64, draft_deg: f64, z_ref: f64) -> Mesh {
    let t = draft_deg.to_radians().tan();
    loft_region(region, &[Level::new(z0, -(z0 - z_ref) * t), Level::new(z1, -(z1 - z_ref) * t)])
}

/// Axis-aligned box.
pub fn box_mesh(min: P3, max: P3) -> Mesh {
    let r = Region::simple(vec![[min[0], min[1]], [max[0], min[1]], [max[0], max[1]], [min[0], max[1]]]);
    extrude(&r, min[2], max[2])
}

/// Box with rounded vertical edges and an approximate fillet of radius
/// `fillet` on the top and bottom edges (quarter-circle offset profile).
pub fn rounded_box(center: P3, size: P3, r_corner: f64, fillet: f64, seg: usize) -> Mesh {
    let outline = g::rounded_rect(center[0], center[1], size[0], size[1], r_corner, seg);
    let reg = Region::simple(outline);
    let f = fillet.clamp(0.0, (0.5 * size[2]).min(r_corner.max(0.0)));
    let z0 = center[2] - 0.5 * size[2];
    let z1 = center[2] + 0.5 * size[2];
    if f <= 1e-6 {
        return extrude(&reg, z0, z1);
    }
    let mut levels = Vec::new();
    let k = seg.clamp(2, 8);
    for i in 0..=k {
        let a = std::f64::consts::FRAC_PI_2 * i as f64 / k as f64;
        levels.push(Level::new(z0 + f - f * a.cos(), -(f - f * a.sin())));
    }
    for i in 0..=k {
        let a = std::f64::consts::FRAC_PI_2 * i as f64 / k as f64;
        levels.push(Level::new(z1 - f + f * a.sin(), -(f - f * a.cos())));
    }
    levels.dedup_by(|b, a| (b.z - a.z).abs() < 1e-12);
    loft_region(&reg, &levels)
}

/// Solid cylinder along Z.
pub fn cylinder(cx: f64, cy: f64, r: f64, z0: f64, z1: f64, n: usize) -> Mesh {
    extrude(&Region::simple(g::circle(cx, cy, r, n)), z0, z1)
}

/// Tube (cylinder with a coaxial hole).
pub fn tube(cx: f64, cy: f64, r_out: f64, r_in: f64, z0: f64, z1: f64, n: usize) -> Mesh {
    extrude(&Region::new(g::circle(cx, cy, r_out, n), vec![g::circle(cx, cy, r_in, n)]), z0, z1)
}

/// Closed hollow shell: the solid lofted from `region` minus an internal
/// cavity inset by `wall` on every side, built as a double-walled mesh (outer
/// skin with outward normals + inner skin with inward normals). Volume
/// by the divergence theorem is then the material volume.
pub fn hollow_shell(region: &Region, z0: f64, z1: f64, wall: f64) -> Mesh {
    let mut outer = extrude(region, z0, z1);
    let inner_region = region.offset(-wall, wall);
    let mut inner = extrude(&inner_region, z0 + wall, z1 - wall);
    inner.flip();
    outer.append(&inner);
    outer
}

/// Sweep a counter-clockwise 2D profile (coordinates along the frame's
/// normal and binormal) along a 3D polyline. Frames are rotation-minimising
/// (double reflection); both ends are capped.
pub fn sweep(profile: &[P2], path: &[P3]) -> Mesh {
    let mut mesh = Mesh::default();
    let n = path.len();
    if n < 2 || profile.len() < 3 {
        return mesh;
    }
    let profile = g::oriented(profile.to_vec(), true);
    // Tangents
    let tangent = |i: usize| -> P3 {
        if i == 0 {
            v_norm(v_sub(path[1], path[0]))
        } else if i == n - 1 {
            v_norm(v_sub(path[n - 1], path[n - 2]))
        } else {
            v_norm(v_add(v_norm(v_sub(path[i], path[i - 1])), v_norm(v_sub(path[i + 1], path[i]))))
        }
    };
    let t0 = tangent(0);
    let helper = if t0[2].abs() < 0.9 { [0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0] };
    let mut nrm = v_norm(v_cross(helper, t0));
    let mut frames = Vec::with_capacity(n);
    let mut t_prev = t0;
    for i in 0..n {
        let ti = tangent(i);
        if i > 0 {
            // Double reflection (Wang et al. 2008).
            let v1 = v_sub(path[i], path[i - 1]);
            let c1 = v_dot(v1, v1);
            if c1 > 1e-18 {
                let r_l = v_sub(nrm, v_scale(v1, 2.0 / c1 * v_dot(v1, nrm)));
                let t_l = v_sub(t_prev, v_scale(v1, 2.0 / c1 * v_dot(v1, t_prev)));
                let v2 = v_sub(ti, t_l);
                let c2 = v_dot(v2, v2);
                nrm = if c2 > 1e-18 {
                    v_sub(r_l, v_scale(v2, 2.0 / c2 * v_dot(v2, r_l)))
                } else {
                    r_l
                };
                nrm = v_norm(v_sub(nrm, v_scale(ti, v_dot(nrm, ti))));
            }
        }
        let b = v_cross(ti, nrm);
        // Mitre scale keeps the section size at bends.
        let s = if i > 0 && i < n - 1 {
            let tin = v_norm(v_sub(path[i], path[i - 1]));
            1.0 / v_dot(tin, ti).max(0.5)
        } else {
            1.0
        };
        frames.push((nrm, b, s, ti));
        t_prev = ti;
    }
    let m = profile.len() as u32;
    for (i, &(nn, bb, s, t)) in frames.iter().enumerate() {
        for p in &profile {
            // Stretch only the in-plane component along the bend direction.
            let off = v_add(v_scale(nn, p[0]), v_scale(bb, p[1]));
            let along = if i > 0 && i < n - 1 {
                let tin = v_norm(v_sub(path[i], path[i - 1]));
                let bend = v_norm(v_sub(tin, v_scale(t, v_dot(tin, t))));
                let k = v_dot(off, bend);
                v_add(off, v_scale(bend, k * (s - 1.0)))
            } else {
                off
            };
            mesh.add_vertex(v_add(path[i], along));
        }
    }
    for i in 0..(n as u32 - 1) {
        let b0 = i * m;
        let t0 = (i + 1) * m;
        for k in 0..m {
            let k1 = (k + 1) % m;
            mesh.add_tri(b0 + k, b0 + k1, t0 + k1);
            mesh.add_tri(b0 + k, t0 + k1, t0 + k);
        }
    }
    let tris = g::triangulate(&profile, &[]);
    let last = (n as u32 - 1) * m;
    for t in &tris {
        mesh.add_tri(t[0], t[2], t[1]);
        mesh.add_tri(last + t[0], last + t[1], last + t[2]);
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::PI;

    fn check(m: &Mesh) {
        let c = m.validate();
        assert!(c.watertight, "{c:?}");
    }

    #[test]
    fn box_volume() {
        let m = box_mesh([0.0, 0.0, 0.0], [2.0, 3.0, 4.0]);
        check(&m);
        assert!((m.volume() - 24.0).abs() < 1e-9);
    }

    #[test]
    fn cylinder_and_tube_volumes() {
        let c = cylinder(0.0, 0.0, 5.0, 0.0, 10.0, 256);
        check(&c);
        assert!((c.volume() - PI * 25.0 * 10.0).abs() / (PI * 250.0) < 1e-3);
        let t = tube(0.0, 0.0, 5.0, 3.0, 0.0, 10.0, 256);
        check(&t);
        assert!((t.volume() - PI * 16.0 * 10.0).abs() / (PI * 160.0) < 1e-3);
    }

    #[test]
    fn draft_extrusion_is_a_frustum() {
        // Square 20x20, height 10, 5° draft: exact frustum volume.
        let r = Region::simple(vec![[-10.0, -10.0], [10.0, -10.0], [10.0, 10.0], [-10.0, 10.0]]);
        let m = extrude_draft(&r, 0.0, 10.0, 5.0, 0.0);
        check(&m);
        let t = 5f64.to_radians().tan();
        let a0 = 400.0;
        let s1 = 20.0 - 2.0 * 10.0 * t;
        let a1 = s1 * s1;
        let exact = 10.0 / 3.0 * (a0 + a1 + (a0 * a1).sqrt());
        assert!((m.volume() - exact).abs() < 1e-9, "{} vs {exact}", m.volume());
        // Every side face leans in by exactly the draft angle.
        for t in 0..m.triangle_count() {
            let nz = m.face_normal(t)[2];
            if nz.abs() < 0.99 {
                assert!((nz.asin().to_degrees() - 5.0).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn region_with_holes_extrudes_watertight() {
        let outer = g::rounded_rect(0.0, 0.0, 60.0, 30.0, 5.0, 8);
        let h1 = g::superellipse(8.0, 6.0, 3.0, 48);
        let h1 = g::translate(&h1, -15.0, 0.0);
        let h2 = g::translate(&g::circle(0.0, 0.0, 6.0, 40), 15.0, 0.0);
        let r = Region::new(outer, vec![h1, h2]);
        let m = extrude_draft(&r, 0.0, 5.0, 1.5, 0.0);
        check(&m);
        assert!(m.volume() > 0.0);
        let flat = extrude(&r, 0.0, 5.0);
        assert!((flat.volume() - 5.0 * r.area()).abs() < 1e-6);
    }

    #[test]
    fn hollow_shell_volume() {
        let r = Region::simple(vec![[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [0.0, 10.0]]);
        let m = hollow_shell(&r, 0.0, 8.0, 1.0);
        let c = m.validate();
        assert!(c.watertight && c.shells == 2, "{c:?}");
        let exact = 20.0 * 10.0 * 8.0 - 18.0 * 8.0 * 6.0;
        assert!((m.volume() - exact).abs() < 1e-9);
    }

    #[test]
    fn rounded_box_with_fillet_is_closed() {
        let m = rounded_box([0.0, 0.0, 5.0], [30.0, 20.0, 10.0], 4.0, 2.0, 8);
        check(&m);
        let full = g::rounded_rect_area(30.0, 20.0, 4.0) * 10.0;
        assert!(m.volume() < full && m.volume() > 0.9 * full);
    }

    #[test]
    fn sweep_straight_and_bent() {
        let prof = g::circle(0.0, 0.0, 2.0, 64);
        let a = g::area(&prof);
        let m = sweep(&prof, &[[0.0, 0.0, 0.0], [0.0, 50.0, 0.0]]);
        check(&m);
        assert!((m.volume() - a * 50.0).abs() / (a * 50.0) < 1e-9);
        // Quarter arc of radius 30: Pappus V = A · (π/2 · 30).
        let path: Vec<P3> = (0..=48)
            .map(|i| {
                let t = PI / 2.0 * i as f64 / 48.0;
                [0.0, 30.0 * t.sin(), 30.0 * (1.0 - t.cos())]
            })
            .collect();
        let m = sweep(&prof, &path);
        check(&m);
        let exact = a * PI / 2.0 * 30.0;
        assert!((m.volume() - exact).abs() / exact < 2e-3, "{} vs {exact}", m.volume());
    }
}
