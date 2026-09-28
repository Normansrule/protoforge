//! Voxelisation of a closed triangle mesh by ray casting, with a partial-volume
//! material fraction per voxel.
//!
//! For every column of voxel centres (y_j, z_k) a ray is shot along +x.
//! Each triangle whose (y, z) projection contains the ray origin is a
//! crossing at some x; crossings are sorted and swept while summing a
//! **winding number** — +1 where the ray enters a surface (outward normal
//! against the ray, nₓ < 0), −1 where it leaves. A voxel centre with a
//! positive winding number is inside. With a single closed shell this is the
//! classic parity (even–odd) test; the winding form also handles the stacked,
//! face-sharing extrusion shells the CAD generator produces and overlapping
//! solids. Rays are nudged by an irrational fraction of a voxel so they never
//! pass exactly through a mesh edge or vertex.
//!
//! Each voxel is sampled at SUB³ points (SUB rays per column direction, SUB
//! points along each ray), giving a material fraction ρ. Voxels with
//! ρ ≥ [`MIN_RHO`] become elements whose stiffness is scaled by ρ, so a 1.2 mm
//! wall on a 1.5 mm grid keeps (approximately) its real membrane stiffness
//! instead of vanishing or doubling.
//!
//! Triangles are bucketed by the columns their bounding box covers, so the
//! cost is O(triangles × columns-per-triangle + voxels) instead of
//! O(triangles × columns).
//! Reference: F. S. Nooruddin, G. Turk, "Simplification and repair of polygonal
//! models using volumetric techniques", IEEE TVCG 9(2), 2003 (ray-stabbing voxelisation).

use super::fea3d::Grid;
use crate::cad::Mesh;

/// Axis-aligned bounds of a mesh.
pub fn bounds(mesh: &Mesh) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in mesh.positions.chunks(3) {
        for d in 0..3 {
            lo[d] = lo[d].min(p[d]);
            hi[d] = hi[d].max(p[d]);
        }
    }
    (lo, hi)
}

/// Voxelise `mesh` with cubes of side `h`. The grid is fitted to the mesh
/// bounds (one voxel of margin is not needed: centres outside stay empty).
pub fn voxelize(mesh: &Mesh, h: f64) -> Grid {
    let (lo, hi) = bounds(mesh);
    let dims: Vec<usize> = (0..3).map(|d| (((hi[d] - lo[d]) / h).ceil() as usize).max(1)).collect();
    // Centre the grid on the part so the stair-stepping is symmetric.
    let origin: [f64; 3] = std::array::from_fn(|d| 0.5 * (lo[d] + hi[d]) - 0.5 * dims[d] as f64 * h);
    let mut g = Grid::new(dims[0], dims[1], dims[2], h, origin);
    fill(&mut g, mesh);
    g
}

/// Voxelise with the grid snapped so that, on every axis with an anchor,
/// the anchor coordinate lies exactly on a voxel face. Aligning the grid
/// with a slender member's faces (a 4 mm temple on a 2 mm grid) makes it
/// exactly two full voxels thick instead of three partial ones — partial
/// voxels scale stiffness with ρ, which is right for membranes but
/// over-stiffens thin walls in bending (∝ t³, not ∝ t).
pub fn voxelize_aligned(mesh: &Mesh, h: f64, anchor: [Option<f64>; 3]) -> Grid {
    let (lo, hi) = bounds(mesh);
    let mut origin = [0.0; 3];
    let mut dims = [1usize; 3];
    for d in 0..3 {
        match anchor[d] {
            Some(a) => {
                let o = a - h * ((a - lo[d]) / h - 1e-9).ceil();
                origin[d] = o;
                dims[d] = (((hi[d] - o) / h - 1e-9).ceil() as usize).max(1);
            }
            None => {
                dims[d] = (((hi[d] - lo[d]) / h).ceil() as usize).max(1);
                origin[d] = 0.5 * (lo[d] + hi[d]) - 0.5 * dims[d] as f64 * h;
            }
        }
    }
    let mut g = Grid::new(dims[0], dims[1], dims[2], h, origin);
    fill(&mut g, mesh);
    g
}

/// Sub-samples per axis for the partial-volume fraction (3 → 27 per voxel).
pub const SUB: usize = 3;
/// Minimum material fraction for a voxel to become an element.
pub const MIN_RHO: f32 = 0.26;

/// Mark voxels of an existing grid that are inside `mesh` and record their
/// material fraction ρ (share of SUB³ sample points inside).
pub fn fill(g: &mut Grid, mesh: &Mesh) {
    let (nx, ny, nz, h) = (g.nx, g.ny, g.nz, g.h);
    let o = g.origin;
    let jy = 0.123_456_7 * std::f64::consts::FRAC_1_SQRT_2 * 1e-3 * h;
    let jz = 0.314_159_2 * 1e-3 * h;
    let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); ny * nz];
    let ntri = mesh.indices.len() / 3;
    for t in 0..ntri {
        let [a, b, c] = mesh.tri(t);
        let ymin = a[1].min(b[1]).min(c[1]);
        let ymax = a[1].max(b[1]).max(c[1]);
        let zmin = a[2].min(b[2]).min(c[2]);
        let zmax = a[2].max(b[2]).max(c[2]);
        let j0 = ((ymin - o[1]) / h).floor().max(0.0) as usize;
        let j1 = ((ymax - o[1]) / h).floor().min(ny as f64 - 1.0);
        let k0 = ((zmin - o[2]) / h).floor().max(0.0) as usize;
        let k1 = ((zmax - o[2]) / h).floor().min(nz as f64 - 1.0);
        if j1 < 0.0 || k1 < 0.0 {
            continue;
        }
        for k in k0..=k1 as usize {
            for j in j0..=j1 as usize {
                buckets[j + ny * k].push(t as u32);
            }
        }
    }
    let mut count = vec![0u16; nx * ny * nz];
    let mut hits: Vec<(f64, i32)> = Vec::new();
    let sub = SUB as f64;
    for k in 0..nz {
        for j in 0..ny {
            let bucket = &buckets[j + ny * k];
            if bucket.is_empty() {
                continue;
            }
            for sb in 0..SUB {
                for sa in 0..SUB {
                    let py = o[1] + h * (j as f64 + (sa as f64 + 0.5) / sub) + jy;
                    let pz = o[2] + h * (k as f64 + (sb as f64 + 0.5) / sub) + jz;
                    hits.clear();
                    for &t in bucket {
                        let [a, b, c] = mesh.tri(t as usize);
                        // 2D barycentric in (y, z).
                        let d = (b[1] - a[1]) * (c[2] - a[2]) - (c[1] - a[1]) * (b[2] - a[2]);
                        if d.abs() < 1e-18 {
                            continue; // triangle parallel to the ray
                        }
                        let w1 = ((py - a[1]) * (c[2] - a[2]) - (c[1] - a[1]) * (pz - a[2])) / d;
                        let w2 = ((b[1] - a[1]) * (pz - a[2]) - (py - a[1]) * (b[2] - a[2])) / d;
                        let w0 = 1.0 - w1 - w2;
                        if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                            continue;
                        }
                        let x = w0 * a[0] + w1 * b[0] + w2 * c[0];
                        // The normal's x-component has the sign of d (projected area).
                        hits.push((x, if d < 0.0 { 1 } else { -1 }));
                    }
                    if hits.is_empty() {
                        continue;
                    }
                    hits.sort_by(|p, q| p.0.total_cmp(&q.0));
                    let mut wind = 0;
                    let mut hi = 0;
                    for i in 0..nx {
                        for sc in 0..SUB {
                            let x = o[0] + h * (i as f64 + (sc as f64 + 0.5) / sub);
                            while hi < hits.len() && hits[hi].0 <= x {
                                wind += hits[hi].1;
                                hi += 1;
                            }
                            if wind > 0 {
                                count[i + nx * (j + ny * k)] += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    let full = (SUB * SUB * SUB) as f32;
    for (e, c) in count.iter().enumerate() {
        let r = *c as f32 / full;
        g.rho[e] = r;
        g.solid[e] = r >= MIN_RHO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::solid::box_mesh;

    #[test]
    fn box_voxel_count_matches_volume() {
        let m = box_mesh([0.0, 0.0, 0.0], [10.0, 4.0, 6.0]);
        let g = voxelize(&m, 1.0);
        assert_eq!(g.count(), 240);
        let g = voxelize(&m, 0.5);
        assert_eq!(g.count(), 1920);
    }

    #[test]
    fn aligned_grid_puts_the_anchor_on_a_voxel_face() {
        // A 4 mm thick plate starting at x = 0.3: centred 2 mm voxels straddle
        // it (partial voxels), aligned ones fit it exactly.
        let m = box_mesh([0.3, 0.0, 0.0], [4.3, 10.0, 6.0]);
        let g = voxelize_aligned(&m, 2.0, [Some(0.3), None, None]);
        assert!((g.origin[0] - 0.3).abs() < 1e-12, "{:?}", g.origin);
        assert_eq!(g.nx, 2);
        assert_eq!(g.count(), 2 * 5 * 3);
        assert!(g.rho.iter().all(|r| (*r - 1.0).abs() < 1e-6));
        let c = voxelize(&m, 2.0);
        assert_eq!(c.nx, 2);
        let off = voxelize_aligned(&m, 1.5, [Some(0.3), None, None]);
        assert!(off.origin[0] <= 0.3 + 1e-9 && off.origin[0] + 1.5 * off.nx as f64 >= 4.3 - 1e-9);
    }

    #[test]
    fn iris_parts_voxelise_close_to_their_volume() {
        let p = crate::project::Project::iris();
        for part in crate::cad::frame_parts(&p) {
            if part.name != "front" && part.name != "temple_r" {
                continue;
            }
            let vol = part.mesh.mass_props(1.0).volume_mm3;
            for h in [0.5, 1.5] {
                let g = voxelize(&part.mesh, h);
                // Partial-volume voxels keep the material volume even when
                // walls (1.2 mm) are thinner than a voxel.
                let rel = (g.volume() - vol) / vol;
                assert!(
                    rel.abs() < 0.06,
                    "{} h={h}: voxel {:.0} vs mesh {vol:.0} mm³",
                    part.name,
                    g.volume()
                );
            }
        }
    }
}
