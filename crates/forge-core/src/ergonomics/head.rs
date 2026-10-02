//! A parametric head for the 3-D fit view.
//!
//! This is a *simple procedural mesh*, not a scanned head form: an ellipsoidal
//! cranium whose breadth, length and height follow the chosen head breadth
//! (length and height scale with ANSUR II ratios: head length ≈ 1.29 ×
//! breadth, tragion-to-top ≈ 0.85 × breadth), a face that narrows below the
//! cheekbones towards the chin, a wedge nose that passes through the frame's
//! nose pads, two ears whose roots sit at the chosen hinge-to-ear distance
//! and two eyes at the chosen interpupillary distance. It is placed in the
//! frame's assembly coordinates (mm; X right, Y forward, Z up) so the Iris CAD
//! parts can be drawn on it unchanged.
//!
//! For real head forms see the NIOSH/ISO headforms used for respirator and
//! helmet fitting (e.g. ISO/TS 16976-2) or open 3-D morphable models; this
//! toy exists to make the percentile sliders tangible.

use serde::Serialize;

use super::model::FrameGeom;
use crate::cad::mesh::{Mesh, MeshF32};

#[derive(Clone, Debug, Serialize)]
pub struct HeadMesh {
    pub head: MeshF32,
    pub nose: MeshF32,
    pub ears: MeshF32,
    /// Eye centres (right, left) and eyeball radius.
    pub eyes: [[f64; 3]; 2],
    pub eye_radius: f64,
    /// Pupil (corneal apex) positions (right, left).
    pub pupils: [[f64; 3]; 2],
    /// Ear-root tops (right, left).
    pub ear_tops: [[f64; 3]; 2],
    /// Where the temples press (right, left) on this head.
    pub contacts: [[f64; 3]; 2],
}

fn lerp_table(tab: &[(f64, f64)], x: f64) -> f64 {
    if x <= tab[0].0 {
        return tab[0].1;
    }
    for w in tab.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        if x <= x1 {
            return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
        }
    }
    tab[tab.len() - 1].1
}

fn grid_mesh(nu: usize, nv: usize, f: impl Fn(f64, f64) -> [f64; 3]) -> Mesh {
    // u: longitude 0..2π (wraps), v: latitude −π/2..π/2 (poles collapsed)
    let mut m = Mesh::default();
    for j in 0..=nv {
        let v = -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * j as f64 / nv as f64;
        for i in 0..nu {
            let u = 2.0 * std::f64::consts::PI * i as f64 / nu as f64;
            m.add_vertex(f(u, v));
        }
    }
    for j in 0..nv {
        for i in 0..nu {
            let a = (j * nu + i) as u32;
            let b = (j * nu + (i + 1) % nu) as u32;
            let c = ((j + 1) * nu + (i + 1) % nu) as u32;
            let d = ((j + 1) * nu + i) as u32;
            m.add_tri(a, b, c);
            m.add_tri(a, c, d);
        }
    }
    m
}

/// Build the head for one person. `contact_y` is where the temples touch.
pub fn head_mesh(fg: &FrameGeom, head_breadth: f64, ipd: f64, ear_distance: f64, nose_width: f64, contact_y: f64) -> HeadMesh {
    let hb = head_breadth.clamp(110.0, 200.0);
    let a_x = hb / 2.0;
    let a_y = 1.29 * hb / 2.0;
    let z0 = 20.0;
    let a_up = 0.85 * hb - z0 + 2.0;
    let a_dn = 115.0;
    let face_front = -fg.front_thickness_mm - 9.0; // forehead just behind the frame
    let y0 = face_front - a_y;
    let cornea_y = -fg.front_thickness_mm - 12.0; // ≈ 12 mm vertex distance
                                                  // Width factor with height: cheekbones, jaw, chin.
    let width_tab = [
        (-125.0, 0.30),
        (-110.0, 0.42),
        (-80.0, 0.70),
        (-40.0, 0.86),
        (-15.0, 0.93),
        (12.0, 1.0),
    ];
    // Face surface (max forward y) with height, before lateral curvature.
    let face_tab = [
        (-125.0, face_front - 16.0),
        (-108.0, face_front - 6.0),
        (-85.0, face_front - 14.0),
        (-60.0, face_front - 8.0),
        (-30.0, face_front - 12.0),
        (-8.0, face_front - 14.0),
        (8.0, face_front - 10.0),
        (22.0, face_front),
        (60.0, face_front - 12.0),
    ];
    let head = grid_mesh(64, 40, |u, v| {
        let (cu, su, cv, sv) = (u.cos(), u.sin(), v.cos(), v.sin());
        let dz = if sv >= 0.0 { a_up } else { a_dn };
        let z = z0 + dz * sv;
        let w = lerp_table(&width_tab, z);
        let x = a_x * cv * su * w;
        let mut y = y0 + a_y * cv * cu;
        // flatten the face: nothing in front of the face surface (curved around)
        let limit = lerp_table(&face_tab, z) - 0.0045 * x * x;
        if y > limit {
            y = limit + (y - limit) * 0.15;
        }
        [x, y, z]
    });
    // Nose: a triangular wedge through the nose pads.
    let pad_in = (fg.pad[0] - fg.pad_semi_mm[0]).max(2.0);
    let tan_g = (35f64).to_radians().tan();
    let ridge_at_pad = fg.pad[1] + pad_in / tan_g;
    let z_pad = fg.pad[2];
    let nose_half = |z: f64| -> f64 {
        // half-width grows from the root to the alae; scaled so it matches the
        // chosen nose bridge width at pad height
        let base = lerp_table(
            &[
                (-40.0, 17.0),
                (-30.0, 12.0),
                (z_pad, nose_width / 2.0),
                (14.0, nose_width / 2.0 - 2.0),
            ],
            z,
        );
        base.max(2.0)
    };
    let ridge = |z: f64| -> f64 {
        lerp_table(
            &[
                (-42.0, ridge_at_pad + 12.0),
                (-36.0, ridge_at_pad + 15.0),
                (z_pad, ridge_at_pad),
                (14.0, ridge_at_pad - 8.0),
            ],
            z,
        )
    };
    let mut nose = Mesh::default();
    let levels: Vec<f64> = crate::math::linspace(-42.0, 14.0, 15);
    for &z in &levels {
        let yr = ridge(z);
        let hw = nose_half(z);
        let yb = yr - hw / tan_g - 4.0;
        nose.add_vertex([0.0, yr, z]);
        nose.add_vertex([hw, yb, z]);
        nose.add_vertex([-hw, yb, z]);
    }
    for k in 0..levels.len() - 1 {
        let a = (3 * k) as u32;
        let b = a + 3;
        // right flank
        nose.add_tri(a, a + 1, b + 1);
        nose.add_tri(a, b + 1, b);
        // left flank
        nose.add_tri(a, b, b + 2);
        nose.add_tri(a, b + 2, a + 2);
        // underside
        nose.add_tri(a + 1, a + 2, b + 2);
        nose.add_tri(a + 1, b + 2, b + 1);
    }
    let last = (3 * (levels.len() - 1)) as u32;
    nose.add_tri(0, 2, 1);
    nose.add_tri(last, last + 1, last + 2);
    // Head half-width at (y, z) on the ellipsoid.
    let half_width = |y: f64, z: f64| -> f64 {
        let dz = if z >= z0 { a_up } else { a_dn };
        let r = 1.0 - ((y - y0) / a_y).powi(2) - ((z - z0) / dz).powi(2);
        a_x * r.max(0.0).sqrt() * lerp_table(&width_tab, z)
    };
    // Ears: flattened ellipsoids, top of the root at the ear distance.
    let ear_top_y = fg.pin_y - ear_distance;
    let ear_top_z = fg.hinge_z;
    let (el, eb, et) = (62.0, 34.0, 9.0);
    let mut ears = Mesh::default();
    let mut ear_tops = [[0.0; 3]; 2];
    for (k, side) in [1.0, -1.0].into_iter().enumerate() {
        let cy = ear_top_y - eb * 0.35;
        let cz = ear_top_z - el * 0.45;
        let cx = side * (half_width(cy, cz) + et * 0.4);
        let e = grid_mesh(24, 14, |u, v| {
            let (cu, su, cv, sv) = (u.cos(), u.sin(), v.cos(), v.sin());
            // tilt the ear back ~15°
            let (ly, lz) = (eb / 2.0 * cv * cu, el / 2.0 * sv);
            let t = 15f64.to_radians();
            [
                cx + side * et / 2.0 * cv * su,
                cy + ly * t.cos() - lz * t.sin(),
                cz + ly * t.sin() + lz * t.cos(),
            ]
        });
        let off = ears.vertex_count() as u32;
        ears.positions.extend_from_slice(&e.positions);
        ears.indices.extend(e.indices.iter().map(|i| i + off));
        ear_tops[k] = [side * half_width(ear_top_y, ear_top_z), ear_top_y, ear_top_z];
    }
    let eye_r = 12.0;
    let eyes = [[ipd / 2.0, cornea_y - eye_r, 0.0], [-ipd / 2.0, cornea_y - eye_r, 0.0]];
    let pupils = [[ipd / 2.0, cornea_y, 0.0], [-ipd / 2.0, cornea_y, 0.0]];
    let cw = half_width(contact_y, fg.hinge_z).max(hb / 2.0 * 0.9);
    let contacts = [[cw, contact_y, fg.hinge_z], [-cw, contact_y, fg.hinge_z]];
    HeadMesh {
        head: head.to_f32(),
        nose: nose.to_f32(),
        ears: ears.to_f32(),
        eyes,
        eye_radius: eye_r,
        pupils,
        ear_tops,
        contacts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Project;

    #[test]
    fn head_is_sized_by_breadth_and_well_formed() {
        let p = Project::iris();
        let fg = super::super::model::frame_geom(&p);
        let h = head_mesh(&fg, 154.0, 64.0, 103.0, 18.0, -100.0);
        let m = h.head.to_mesh();
        let (lo, hi) = m.bbox();
        let width = hi[0] - lo[0];
        assert!((width - 154.0).abs() < 3.0, "{width}");
        assert!(m.indices.len().is_multiple_of(3) && m.indices.iter().all(|&i| (i as usize) < m.vertex_count()));
        assert!((h.pupils[0][0] - h.pupils[1][0] - 64.0).abs() < 1e-9);
        assert!((fg.pin_y - h.ear_tops[0][1] - 103.0).abs() < 1e-9);
    }
}
