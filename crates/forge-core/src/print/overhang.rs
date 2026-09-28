//! Overhang analysis for a part in its print orientation.
//!
//! A downward-facing surface whose angle from vertical exceeds roughly 45°
//! cannot be built on the layer below without support: each new bead would
//! overhang the previous one by more than about half its width (the "45°
//! rule" of material extrusion, see I. Gibson, D. Rosen, B. Stucker,
//! *Additive Manufacturing Technologies*, 2nd ed., 2015, §5.5 and
//! ISO/ASTM 52900 terminology). For a face with unit normal n, the angle
//! from vertical is α = asin(−n_z) for n_z < 0. Faces resting on the bed
//! (z ≈ 0) are supported by definition.

use serde::Serialize;

use crate::cad::mesh::Mesh;

#[derive(Clone, Debug, Serialize)]
pub struct OverhangReport {
    /// Unwelded positions (3 vertices per triangle) and matching colours.
    pub positions: Vec<f32>,
    pub colors: Vec<f32>,
    pub threshold_deg: f64,
    pub overhang_area_mm2: f64,
    pub overhang_faces: usize,
    pub downward_area_mm2: f64,
    pub bed_area_mm2: f64,
    pub max_angle_deg: f64,
    /// Area of down-facing (non-bed) surface per 10° band of angle from
    /// vertical: [0–10°, 10–20°, … 80–90°].
    pub histogram_mm2: [f64; 9],
    pub needs_support: bool,
    pub suggestion: String,
}

pub fn analyse(mesh: &Mesh, threshold_deg: f64) -> OverhangReport {
    let (lo, _) = mesh.bbox();
    let thr = threshold_deg.clamp(5.0, 89.0);
    let n = mesh.triangle_count();
    let mut positions = Vec::with_capacity(n * 9);
    let mut colors = Vec::with_capacity(n * 9);
    let mut r = OverhangReport {
        positions: vec![],
        colors: vec![],
        threshold_deg: thr,
        overhang_area_mm2: 0.0,
        overhang_faces: 0,
        downward_area_mm2: 0.0,
        bed_area_mm2: 0.0,
        max_angle_deg: 0.0,
        histogram_mm2: [0.0; 9],
        needs_support: false,
        suggestion: String::new(),
    };
    // Flat down-facing faces above the bed may rest on material below (the
    // contact face between stacked shells): test the layer just beneath.
    let idx = super::slicer::TriIndex::new(mesh);
    let mut support_cache: std::collections::HashMap<i64, Vec<Vec<crate::cad::geom2d::P2>>> = std::collections::HashMap::new();
    for t in 0..n {
        let tri = mesh.tri(t);
        let nrm = mesh.face_normal(t);
        let area = 0.5 * crate::cad::mesh::v_len(crate::cad::mesh::v_cross(crate::cad::mesh::v_sub(tri[1], tri[0]), crate::cad::mesh::v_sub(tri[2], tri[0])));
        let zmax = tri.iter().map(|p| p[2]).fold(f64::NEG_INFINITY, f64::max);
        let on_bed = zmax - lo[2] < 0.05;
        let mut col = [0.60, 0.66, 0.76];
        if nrm[2] < -1e-6 {
            let alpha = (-nrm[2]).clamp(0.0, 1.0).asin().to_degrees();
            let supported = !on_bed && nrm[2] < -0.999 && {
                let zf = tri[0][2];
                let key = (zf * 1e3).round() as i64;
                let loops = support_cache.entry(key).or_insert_with(|| idx.section(zf - 0.02));
                let c = [(tri[0][0] + tri[1][0] + tri[2][0]) / 3.0, (tri[0][1] + tri[1][1] + tri[2][1]) / 3.0];
                crate::cad::geom2d::winding(c, loops) >= 1
            };
            if on_bed {
                r.bed_area_mm2 += area;
                col = [0.18, 0.68, 0.42];
            } else if supported {
                col = [0.5, 0.56, 0.66];
            } else {
                r.downward_area_mm2 += area;
                r.histogram_mm2[((alpha / 10.0) as usize).min(8)] += area;
                r.max_angle_deg = r.max_angle_deg.max(alpha);
                if alpha > thr {
                    r.overhang_area_mm2 += area;
                    r.overhang_faces += 1;
                    let k = ((alpha - thr) / (90.0 - thr)).clamp(0.0, 1.0);
                    col = [0.92, 0.42 - 0.25 * k, 0.28 - 0.1 * k];
                } else if alpha > thr - 15.0 {
                    col = [0.96, 0.74, 0.22];
                }
            }
        }
        for p in tri {
            positions.extend(p.iter().map(|&v| v as f32));
            colors.extend(col.iter().map(|&v| v as f32));
        }
    }
    r.positions = positions;
    r.colors = colors;
    r.needs_support = r.overhang_area_mm2 > 4.0;
    r.suggestion = if r.overhang_area_mm2 <= 0.5 {
        "No unsupported overhangs: prints without support in this orientation.".into()
    } else if !r.needs_support {
        format!("{:.1} mm² of small overhangs — usually bridges fine, no support needed.", r.overhang_area_mm2)
    } else {
        format!(
            "{:.0} mm² steeper than {thr:.0}° from vertical: enable tree/organic supports (Bambu Studio / OrcaSlicer) or re-orient the part.",
            r.overhang_area_mm2
        )
    };
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::mesh::Affine;
    use crate::cad::solid::box_mesh;

    #[test]
    fn cube_on_bed_has_no_overhang_but_raised_cube_does() {
        let m = box_mesh([0.0; 3], [10.0, 10.0, 10.0]);
        let r = analyse(&m, 45.0);
        assert!(r.overhang_area_mm2 < 1e-9);
        assert!((r.bed_area_mm2 - 100.0).abs() < 1e-9);
        // A T-shape: a slab raised on a thin post overhangs, except where
        // it rests on the post.
        let mut t = box_mesh([4.0, 4.0, 0.0], [6.0, 6.0, 5.0]);
        // Subdivide the slab's underside so faces over the post are separate.
        let slab = crate::cad::solid::extrude(&crate::cad::solid::Region::new(
            vec![[0.0, 0.0], [4.0, 0.0], [6.0, 0.0], [10.0, 0.0], [10.0, 10.0], [6.0, 10.0], [4.0, 10.0], [0.0, 10.0]], vec![]), 5.0, 7.0);
        t.append(&slab);
        let r = analyse(&t, 45.0);
        assert!(r.overhang_area_mm2 > 70.0 && r.overhang_area_mm2 <= 100.0, "{}", r.overhang_area_mm2);
        assert!(r.needs_support);
    }

    #[test]
    fn tilted_face_threshold() {
        // Rotate a raised box 30° about X: its bottom face is 60° from vertical.
        let m = box_mesh([-5.0, -5.0, -5.0], [5.0, 5.0, 5.0]).transformed(&Affine::rot_x(30.0)).translated(0.0, 0.0, 20.0);
        let r = analyse(&m, 45.0);
        assert!((r.max_angle_deg - 60.0).abs() < 1e-6);
        assert!(r.overhang_area_mm2 > 99.0);
    }
}
