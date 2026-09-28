//! Planar FDM slicer: mesh → layers of toolpaths.
//!
//! Pipeline per layer (the classic "slice, offset, hatch" approach):
//! 1. **Section** — intersect every triangle straddling the plane
//!    z = layer mid-height with that plane. Each crossing triangle yields
//!    one segment, oriented with the solid on its left (direction ∝ ẑ × n),
//!    so outer contours come out counter-clockwise and holes clockwise.
//!    Crossing points are computed from the edge's vertices in a canonical
//!    order so neighbouring triangles produce bit-identical endpoints, which
//!    are then chained into closed loops through a hash map.
//! 2. **Regions** — loops are cleaned with the winding rule (a union, which
//!    also merges overlapping shells) and grouped into islands (outer loop +
//!    the holes nested inside it, by even-odd containment).
//! 3. **Perimeters** — island offset inwards by (k + ½)·w for k = 0…P−1
//!    (w = line width). Loops that vanish are dropped; thin walls lose their
//!    inner perimeters.
//! 4. **Skin** — material within N layers of a top or bottom surface is
//!    printed solid: solid = infill ∖ (outline[i−B] ∩ outline[i+T]).
//! 5. **Infill** — rectilinear lines at spacing w/ρ (ρ = infill fraction;
//!    ρ = 1 for skin), alternating 45°/135° per layer, clipped to the
//!    infill region by scan-line even-odd crossings.
//! 6. **Ordering** — nearest-neighbour island order, perimeters inside-out,
//!    seams at the vertex nearest the nozzle, infill lines chained greedily;
//!    travel moves in between (retracted when longer than a threshold).
//!
//! References: I. Gibson, D. Rosen, B. Stucker, *Additive Manufacturing
//! Technologies*, 2nd ed., Springer 2015, ch. 5 (material extrusion);
//! M. de Berg et al., *Computational Geometry*, 3rd ed., 2008;
//! Slic3r / PrusaSlicer and CuraEngine documentation for the conventions.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::cad::clip;
use crate::cad::geom2d::{self as g, P2};
use crate::cad::mesh::Mesh;
use crate::project::PrintParams;

/// Move types, also used as the compact `k` code in the JSON output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[repr(u8)]
pub enum Kind {
    Travel = 0,
    ExternalPerimeter = 1,
    Perimeter = 2,
    SparseInfill = 3,
    SolidInfill = 4,
    Skirt = 5,
}

impl Kind {
    pub fn code(self) -> u8 {
        self as u8
    }
    pub fn label(self) -> &'static str {
        match self {
            Kind::Travel => "travel",
            Kind::ExternalPerimeter => "external perimeter",
            Kind::Perimeter => "inner perimeter",
            Kind::SparseInfill => "sparse infill",
            Kind::SolidInfill => "solid infill (top/bottom)",
            Kind::Skirt => "skirt",
        }
    }
    pub const ALL: [Kind; 6] = [
        Kind::Travel,
        Kind::ExternalPerimeter,
        Kind::Perimeter,
        Kind::SparseInfill,
        Kind::SolidInfill,
        Kind::Skirt,
    ];
}

/// Slicer settings beyond the project's `PrintParams`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SliceSettings {
    pub first_layer_mm: f64,
    pub top_layers: u32,
    pub bottom_layers: u32,
    pub infill_overlap_pct: f64,
    pub retract_mm: f64,
    pub retract_speed_mm_s: f64,
    pub retract_min_travel_mm: f64,
    pub fan_pct: f64,
    pub bed_center: [f64; 2],
    pub bed_size: [f64; 2],
    pub cost_per_kg: f64,
    pub accel_mm_s2: f64,
    /// Junction speed at sharp corners (Klipper `square_corner_velocity`).
    pub corner_speed_mm_s: f64,
    pub min_layer_time_s: f64,
    pub first_layer_speed_pct: f64,
    pub outer_wall_speed_pct: f64,
    pub infill_speed_pct: f64,
    pub relative_e: bool,
    pub skirt: bool,
    /// Filament density; `None` = the frame material density.
    pub density_g_cm3: Option<f64>,
    /// Emit G-code (can be skipped for estimates).
    pub gcode: bool,
}

impl Default for SliceSettings {
    fn default() -> Self {
        SliceSettings {
            first_layer_mm: 0.2,
            top_layers: 4,
            bottom_layers: 4,
            infill_overlap_pct: 15.0,
            retract_mm: 0.8,
            retract_speed_mm_s: 35.0,
            retract_min_travel_mm: 1.5,
            fan_pct: 40.0,
            bed_center: [128.0, 128.0],
            bed_size: [256.0, 256.0],
            cost_per_kg: 20.0,
            accel_mm_s2: 2000.0,
            corner_speed_mm_s: 5.0,
            min_layer_time_s: 6.0,
            first_layer_speed_pct: 50.0,
            outer_wall_speed_pct: 60.0,
            infill_speed_pct: 100.0,
            relative_e: false,
            skirt: true,
            density_g_cm3: None,
            gcode: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Path {
    pub kind: Kind,
    pub pts: Vec<P2>,
    /// Travel only: retract before and unretract after.
    pub retract: bool,
}

#[derive(Clone, Debug)]
pub struct Layer {
    pub index: usize,
    /// Top of the layer (nozzle height).
    pub z: f64,
    pub height: f64,
    pub paths: Vec<Path>,
    /// Area enclosed by the layer outline, mm².
    pub area_mm2: f64,
}

/// Layer tops for a part of height `zmax`.
pub fn layer_tops(zmax: f64, first: f64, h: f64) -> Vec<(f64, f64)> {
    let mut v = Vec::new();
    if zmax <= 0.0 {
        return v;
    }
    let first = first.clamp(0.05, 1.0);
    let h = h.clamp(0.04, 1.0);
    let mut z = first.min(zmax);
    v.push((z, z));
    while z < zmax - 1e-6 {
        let nz = (z + h).min(zmax.max(z + 0.5 * h));
        v.push((nz, nz - z));
        z = nz;
    }
    v
}

/// Triangles pre-sorted by their lowest vertex for fast plane sweeps.
pub struct TriIndex<'a> {
    mesh: &'a Mesh,
    order: Vec<(f64, f64, usize)>, // (zmin, zmax, tri)
}

impl<'a> TriIndex<'a> {
    pub fn new(mesh: &'a Mesh) -> TriIndex<'a> {
        let mut order: Vec<(f64, f64, usize)> = (0..mesh.triangle_count())
            .map(|t| {
                let [a, b, c] = mesh.tri(t);
                (a[2].min(b[2]).min(c[2]), a[2].max(b[2]).max(c[2]), t)
            })
            .collect();
        order.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        TriIndex { mesh, order }
    }

    /// Closed contour loops of the section at height `z`.
    pub fn section(&self, z: f64) -> Vec<Vec<P2>> {
        let m = self.mesh;
        let end = self.order.partition_point(|x| x.0 <= z);
        let mut segs: Vec<(P2, P2)> = Vec::new();
        for &(_, zmax, t) in &self.order[..end] {
            if zmax <= z {
                continue;
            }
            let idx = [m.indices[3 * t], m.indices[3 * t + 1], m.indices[3 * t + 2]];
            let p = [m.vertex(idx[0]), m.vertex(idx[1]), m.vertex(idx[2])];
            let above = [p[0][2] >= z, p[1][2] >= z, p[2][2] >= z];
            let mut pts: Vec<P2> = Vec::with_capacity(2);
            for k in 0..3 {
                let (i, j) = (k, (k + 1) % 3);
                if above[i] != above[j] {
                    // Canonical order by global vertex index.
                    let (u, v) = if idx[i] < idx[j] { (p[i], p[j]) } else { (p[j], p[i]) };
                    let s = (z - u[2]) / (v[2] - u[2]);
                    pts.push([u[0] + s * (v[0] - u[0]), u[1] + s * (v[1] - u[1])]);
                }
            }
            if pts.len() != 2 || pts[0] == pts[1] {
                continue;
            }
            let n = m.face_normal(t);
            let dir = [-n[1], n[0]];
            let d = g::sub(pts[1], pts[0]);
            if g::dot(d, dir) >= 0.0 {
                segs.push((pts[0], pts[1]));
            } else {
                segs.push((pts[1], pts[0]));
            }
        }
        chain_segments(&segs)
    }
}

fn key(p: P2) -> (u64, u64) {
    ((p[0] + 0.0).to_bits(), (p[1] + 0.0).to_bits())
}

/// Chain directed segments into closed loops (exact endpoint hashing, with a
/// small-gap fallback for meshes that are not perfectly closed).
pub fn chain_segments(segs: &[(P2, P2)]) -> Vec<Vec<P2>> {
    let mut by_start: HashMap<(u64, u64), Vec<usize>> = HashMap::with_capacity(segs.len());
    for (i, s) in segs.iter().enumerate() {
        by_start.entry(key(s.0)).or_default().push(i);
    }
    let mut used = vec![false; segs.len()];
    let mut loops = Vec::new();
    for s0 in 0..segs.len() {
        if used[s0] {
            continue;
        }
        used[s0] = true;
        let mut pts = vec![segs[s0].0];
        let mut cur = s0;
        let start = key(segs[s0].0);
        let mut closed = false;
        for _ in 0..segs.len() {
            let e = segs[cur].1;
            if key(e) == start {
                closed = true;
                break;
            }
            pts.push(e);
            let next = by_start.get(&key(e)).and_then(|c| c.iter().copied().find(|&i| !used[i]));
            match next {
                Some(i) => {
                    used[i] = true;
                    cur = i;
                }
                None => break,
            }
        }
        if !closed && pts.len() > 2 && g::dist(pts[0], *pts.last().unwrap()) < 0.05 {
            closed = true;
        }
        if closed && pts.len() >= 3 {
            let s = g::simplify(&pts, 0.005);
            if s.len() >= 3 && g::area(&s).abs() > 1e-6 {
                loops.push(s);
            }
        }
    }
    loops
}

/// Clean a layer's loops and group them into islands (outer, holes...).
pub fn islands(loops: Vec<Vec<P2>>) -> Vec<Vec<Vec<P2>>> {
    if loops.is_empty() {
        return vec![];
    }
    let clean = if clip::has_intersections(&loops) {
        clip::union(&loops)
    } else {
        clip::classify_loops(loops, |w| w >= 1)
    };
    group(clean)
}

/// Group oriented loops into islands by containment.
pub fn group(loops: Vec<Vec<P2>>) -> Vec<Vec<Vec<P2>>> {
    let (outers, holes): (Vec<_>, Vec<_>) = loops.into_iter().partition(|l| g::area(l) > 0.0);
    let mut out: Vec<Vec<Vec<P2>>> = outers.into_iter().map(|o| vec![o]).collect();
    for h in holes {
        let p = h[0];
        let mut best = None;
        let mut best_a = f64::INFINITY;
        for (i, isl) in out.iter().enumerate() {
            if g::point_in_polygon(p, &isl[0]) {
                let a = g::area(&isl[0]);
                if a < best_a {
                    best_a = a;
                    best = Some(i);
                }
            }
        }
        if let Some(i) = best {
            out[i].push(h);
        }
    }
    out
}

fn rot(p: P2, c: f64, s: f64) -> P2 {
    [p[0] * c - p[1] * s, p[0] * s + p[1] * c]
}

/// Rectilinear hatch lines of `region` at `angle_deg`, `spacing` apart on
/// a global grid (so consecutive layers line up).
pub fn hatch(region: &[Vec<P2>], angle_deg: f64, spacing: f64, min_len: f64) -> Vec<[P2; 2]> {
    let mut out = Vec::new();
    if region.is_empty() || spacing <= 0.0 {
        return out;
    }
    let (s, c) = angle_deg.to_radians().sin_cos();
    // Rotate the region by -angle so hatch lines are horizontal.
    let rloops: Vec<Vec<P2>> = region.iter().map(|l| l.iter().map(|&p| rot(p, c, -s)).collect()).collect();
    let mut ymin = f64::INFINITY;
    let mut ymax = f64::NEG_INFINITY;
    let mut edges: Vec<(P2, P2)> = Vec::new();
    for l in &rloops {
        let n = l.len();
        for i in 0..n {
            let a = l[i];
            let b = l[(i + 1) % n];
            ymin = ymin.min(a[1]);
            ymax = ymax.max(a[1]);
            if a[1] != b[1] {
                edges.push((a, b));
            }
        }
    }
    edges.sort_by(|a, b| {
        a.0[1]
            .min(a.1[1])
            .partial_cmp(&b.0[1].min(b.1[1]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let k0 = (ymin / spacing - 0.5).ceil() as i64;
    let k1 = (ymax / spacing - 0.5).floor() as i64;
    let mut xs: Vec<f64> = Vec::new();
    for k in k0..=k1 {
        let y = (k as f64 + 0.5) * spacing;
        xs.clear();
        let end = edges.partition_point(|e| e.0[1].min(e.1[1]) <= y);
        for &(a, b) in &edges[..end] {
            let (lo, hi) = if a[1] < b[1] { (a, b) } else { (b, a) };
            if lo[1] <= y && y < hi[1] {
                xs.push(lo[0] + (y - lo[1]) / (hi[1] - lo[1]) * (hi[0] - lo[0]));
            }
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        for pair in xs.chunks(2) {
            if pair.len() == 2 && pair[1] - pair[0] >= min_len {
                out.push([rot([pair[0], y], c, s), rot([pair[1], y], c, s)]);
            }
        }
    }
    out
}

/// Pre-computed geometry of one layer before ordering.
struct LayerGeom {
    z: f64,
    h: f64,
    islands: Vec<Vec<Vec<P2>>>,
    outline: Vec<Vec<P2>>,
    area: f64,
}

/// Which layers to fully process (all by default; every `stride`-th for
/// sampled estimates).
pub fn slice(mesh: &Mesh, pp: &PrintParams, st: &SliceSettings, only: Option<&[usize]>) -> Vec<Layer> {
    let (lo, hi) = mesh.bbox();
    let zmax = hi[2] - lo[2];
    let mesh_z = if lo[2].abs() > 1e-9 {
        mesh.translated(0.0, 0.0, -lo[2])
    } else {
        mesh.clone()
    };
    let idx = TriIndex::new(&mesh_z);
    let w = pp.line_width_mm.clamp(0.1, 2.0);
    let tops = layer_tops(zmax, st.first_layer_mm, pp.layer_height_mm);
    let n = tops.len();
    let want = |i: usize| -> bool { only.map(|o| o.contains(&i)).unwrap_or(true) };
    let need = |i: usize| -> bool {
        // A layer's geometry is needed if it or a skin neighbour is wanted.
        want(i)
            || (i >= st.top_layers as usize && want(i - st.top_layers as usize))
            || (i + (st.bottom_layers as usize) < n && want(i + st.bottom_layers as usize))
    };
    let mut geoms: Vec<Option<LayerGeom>> = Vec::with_capacity(n);
    for (i, &(z, h)) in tops.iter().enumerate() {
        if !need(i) {
            geoms.push(None);
            continue;
        }
        let mut zc = z - h / 2.0;
        // Nudge planes that coincide with vertex heights.
        zc += 1e-7;
        let loops = idx.section(zc);
        let isl = islands(loops);
        let outline: Vec<Vec<P2>> = isl.iter().flat_map(|x| x.iter().cloned()).collect();
        let area = clip::region_area(&outline);
        geoms.push(Some(LayerGeom {
            z,
            h,
            islands: isl,
            outline,
            area,
        }));
    }
    let perims = pp.perimeters.clamp(0, 20) as usize;
    let lh = pp.layer_height_mm.clamp(0.04, 1.0);
    let ov = (st.infill_overlap_pct / 100.0).clamp(0.0, 0.5);
    let infill_frac = (pp.infill_pct / 100.0).clamp(0.0, 1.0);
    let mut layers = Vec::new();
    let mut pos: P2 = [0.0, 0.0];
    for i in 0..n {
        if !want(i) {
            continue;
        }
        let Some(lg) = geoms[i].as_ref() else { continue };
        let mut paths: Vec<Path> = Vec::new();
        // Skirt around the first layer.
        if i == 0 && st.skirt && !lg.outline.is_empty() {
            let pts: Vec<P2> = lg.outline.iter().flat_map(|l| l.iter().copied()).collect();
            let hull = convex_hull(&pts);
            if hull.len() >= 3 {
                let (sk, _) = g::raw_offset(&hull, 3.0, g::Join::Round, 0.05);
                let sk = rotate_to_nearest(&sk, pos);
                push_travel(&mut paths, &mut pos, sk[0], st);
                let mut closed = sk.clone();
                closed.push(sk[0]);
                pos = sk[0];
                paths.push(Path {
                    kind: Kind::Skirt,
                    pts: closed,
                    retract: false,
                });
            }
        }
        // Skin detection: interior = outline[i-B] ∩ outline[i+T].
        let b = st.bottom_layers as usize;
        let t = st.top_layers as usize;
        let below = if i >= b { geoms[i - b].as_ref().map(|x| &x.outline) } else { None };
        let above = if i + t < n {
            geoms[i + t].as_ref().map(|x| &x.outline)
        } else {
            None
        };
        let mut whole_interior = false;
        let interior: Option<Vec<Vec<P2>>> = match (below, above) {
            (Some(bl), Some(ab)) if !bl.is_empty() && !ab.is_empty() => {
                if (b == 0 && t == 0) || (same_shape(bl, &lg.outline, w) && same_shape(ab, &lg.outline, w)) {
                    // Nothing changes within the skin distance (up to draft):
                    // the whole layer is interior.
                    whole_interior = true;
                    Some(lg.outline.clone())
                } else {
                    Some(clip::intersection(&[bl.as_slice(), ab.as_slice()]))
                }
            }
            _ => None,
        };
        let angle = if i % 2 == 0 { 45.0 } else { 135.0 };
        // Bead spacing that tiles the plane with rounded-rectangle beads
        // (Slic3r flow math): s = w − h·(1 − π/4).
        let sp = extrusion_spacing(w, lg.h.min(lh.max(lg.h)));
        // Island order: nearest first.
        let mut remaining: Vec<usize> = (0..lg.islands.len()).collect();
        while !remaining.is_empty() {
            let (ri, _) = remaining
                .iter()
                .enumerate()
                .map(|(k, &isl)| (k, lg.islands[isl][0].iter().map(|&p| g::dist(p, pos)).fold(f64::INFINITY, f64::min)))
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap();
            let island = &lg.islands[remaining.swap_remove(ri)];
            // Perimeters, computed from the island outline each time.
            let mut rings: Vec<(usize, Vec<Vec<P2>>)> = Vec::new();
            for k in 0..perims {
                let off = clip::offset_region(island, -(0.5 * w + k as f64 * sp), 0.02);
                if off.is_empty() {
                    break;
                }
                rings.push((k, off));
            }
            for (k, loops) in rings.iter().rev() {
                let kind = if *k == 0 { Kind::ExternalPerimeter } else { Kind::Perimeter };
                let mut order: Vec<&Vec<P2>> = loops.iter().collect();
                while !order.is_empty() {
                    let (oi, _) = order
                        .iter()
                        .enumerate()
                        .map(|(k, l)| (k, l.iter().map(|&p| g::dist(p, pos)).fold(f64::INFINITY, f64::min)))
                        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                        .unwrap();
                    let l = order.swap_remove(oi);
                    let r = rotate_to_nearest(l, pos);
                    push_travel(&mut paths, &mut pos, r[0], st);
                    let mut closed = r.clone();
                    closed.push(r[0]);
                    pos = r[0];
                    paths.push(Path {
                        kind,
                        pts: closed,
                        retract: false,
                    });
                }
            }
            if rings.len() < perims {
                continue;
            }
            let d_inf = if perims == 0 {
                0.5 * w
            } else {
                0.5 * w + perims as f64 * sp - ov * sp
            };
            let infill = clip::offset_region(island, -d_inf, 0.02);
            if infill.is_empty() {
                continue;
            }
            let (solid, sparse) = match &interior {
                None => (infill.clone(), vec![]),
                Some(int) if int.is_empty() => (infill.clone(), vec![]),
                Some(_) if whole_interior => (vec![], infill.clone()),
                Some(int) => {
                    let sparse = clip::intersection(&[infill.as_slice(), int.as_slice()]);
                    let solid = clip::difference(&infill, int);
                    // Morphological opening drops slivers narrower than a line.
                    let solid = if solid.is_empty() {
                        solid
                    } else {
                        clip::offset_region(&clip::offset_region(&solid, -0.45 * w, 0.02), 0.45 * w, 0.02)
                    };
                    (solid, sparse)
                }
            };
            for (region, kind, spacing) in [
                (&solid, Kind::SolidInfill, sp),
                (&sparse, Kind::SparseInfill, if infill_frac > 0.0 { sp / infill_frac } else { 0.0 }),
            ] {
                if region.is_empty() || spacing <= 0.0 {
                    continue;
                }
                let lines = hatch(region, angle, spacing, 0.5 * w);
                order_lines(lines, &mut pos, &mut paths, kind, st, spacing);
            }
        }
        layers.push(Layer {
            index: i,
            z: lg.z,
            height: lg.h,
            paths,
            area_mm2: lg.area,
        });
    }
    layers
}

/// Centre-to-centre spacing of adjacent beads of width `w` and height `h`.
pub fn extrusion_spacing(w: f64, h: f64) -> f64 {
    (w - h * (1.0 - std::f64::consts::FRAC_PI_4)).max(0.3 * w)
}

/// Cheap "same region up to draft" test: loop count, area and bounding box
/// all within tolerance (`tol` mm of boundary movement).
fn same_shape(a: &[Vec<P2>], b: &[Vec<P2>], tol: f64) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let (aa, ab) = (clip::region_area(a), clip::region_area(b));
    let per: f64 = b.iter().map(|l| g::perimeter(l)).sum();
    let tol = tol.max(1e-6);
    if (aa - ab).abs() > per * tol * 0.5 + 1e-6 {
        return false;
    }
    let bb = |r: &[Vec<P2>]| {
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for l in r {
            let (a, b) = g::bbox(l);
            lo = [lo[0].min(a[0]), lo[1].min(a[1])];
            hi = [hi[0].max(b[0]), hi[1].max(b[1])];
        }
        [lo[0], lo[1], hi[0], hi[1]]
    };
    let (x, y) = (bb(a), bb(b));
    x.iter().zip(y.iter()).all(|(p, q)| (p - q).abs() <= tol + 1e-6)
}

fn push_travel(paths: &mut Vec<Path>, pos: &mut P2, to: P2, st: &SliceSettings) {
    let d = g::dist(*pos, to);
    if d > 1e-6 {
        paths.push(Path {
            kind: Kind::Travel,
            pts: vec![*pos, to],
            retract: d > st.retract_min_travel_mm,
        });
        *pos = to;
    }
}

fn rotate_to_nearest(l: &[P2], pos: P2) -> Vec<P2> {
    let (k, _) = l
        .iter()
        .enumerate()
        .map(|(k, &p)| (k, g::dist(p, pos)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((0, 0.0));
    let mut v = l[k..].to_vec();
    v.extend_from_slice(&l[..k]);
    v
}

/// Greedy nearest-endpoint chaining of hatch lines. Short hops between
/// neighbouring lines stay unretracted (they lie inside the region).
fn order_lines(mut lines: Vec<[P2; 2]>, pos: &mut P2, paths: &mut Vec<Path>, kind: Kind, st: &SliceSettings, spacing: f64) {
    while !lines.is_empty() {
        let mut best = (0usize, false, f64::INFINITY);
        for (i, l) in lines.iter().enumerate() {
            let d0 = g::dist(l[0], *pos);
            let d1 = g::dist(l[1], *pos);
            if d0 < best.2 {
                best = (i, false, d0);
            }
            if d1 < best.2 {
                best = (i, true, d1);
            }
        }
        let l = lines.swap_remove(best.0);
        let (a, b) = if best.1 { (l[1], l[0]) } else { (l[0], l[1]) };
        let d = g::dist(*pos, a);
        if d > 1e-6 {
            let hop = d <= spacing * 2.5 + 1e-9;
            paths.push(Path {
                kind: Kind::Travel,
                pts: vec![*pos, a],
                retract: !hop && d > st.retract_min_travel_mm,
            });
        }
        paths.push(Path {
            kind,
            pts: vec![a, b],
            retract: false,
        });
        *pos = b;
    }
}

/// Andrew's monotone chain convex hull (counter-clockwise).
pub fn convex_hull(pts: &[P2]) -> Vec<P2> {
    let mut p: Vec<P2> = pts.to_vec();
    p.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap().then(a[1].partial_cmp(&b[1]).unwrap()));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let mut lower: Vec<P2> = Vec::new();
    for &q in &p {
        while lower.len() >= 2 && g::orient(lower[lower.len() - 2], lower[lower.len() - 1], q) <= 0.0 {
            lower.pop();
        }
        lower.push(q);
    }
    let mut upper: Vec<P2> = Vec::new();
    for &q in p.iter().rev() {
        while upper.len() >= 2 && g::orient(upper[upper.len() - 2], upper[upper.len() - 1], q) <= 0.0 {
            upper.pop();
        }
        upper.push(q);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::solid::{box_mesh, cylinder};
    use crate::math::PI;

    #[test]
    fn cube_sections_are_squares() {
        let m = box_mesh([-10.0, -10.0, 0.0], [10.0, 10.0, 20.0]);
        let idx = TriIndex::new(&m);
        for z in [0.1, 5.0, 12.345, 19.9] {
            let loops = idx.section(z);
            assert_eq!(loops.len(), 1);
            assert!((g::area(&loops[0]) - 400.0).abs() < 1e-9, "z={z}");
        }
    }

    #[test]
    fn cylinder_section_area() {
        let m = cylinder(0.0, 0.0, 8.0, 0.0, 10.0, 128);
        let loops = TriIndex::new(&m).section(3.3);
        assert_eq!(loops.len(), 1);
        let a = g::area(&loops[0]);
        assert!((a - PI * 64.0).abs() / (PI * 64.0) < 2e-3, "{a}");
    }

    #[test]
    fn tube_section_has_hole() {
        let m = crate::cad::solid::tube(0.0, 0.0, 8.0, 5.0, 0.0, 10.0, 96);
        let isl = islands(TriIndex::new(&m).section(4.0));
        assert_eq!(isl.len(), 1);
        assert_eq!(isl[0].len(), 2);
        assert!(g::area(&isl[0][1]) < 0.0);
    }

    #[test]
    fn layer_tops_cover_height() {
        let t = layer_tops(10.0, 0.3, 0.2);
        assert!((t[0].0 - 0.3).abs() < 1e-12);
        assert!((t.last().unwrap().0 - 10.0).abs() < 1e-9);
        let total: f64 = t.iter().map(|x| x.1).sum();
        assert!((total - 10.0).abs() < 1e-9);
    }

    #[test]
    fn hatch_covers_square() {
        let sq = vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]];
        let lines = hatch(&sq, 0.0, 1.0, 0.0);
        assert_eq!(lines.len(), 10);
        let total: f64 = lines.iter().map(|l| g::dist(l[0], l[1])).sum();
        assert!((total - 100.0).abs() < 1e-9);
        let diag = hatch(&sq, 45.0, 0.5, 0.0);
        let total: f64 = diag.iter().map(|l| g::dist(l[0], l[1])).sum();
        assert!((total * 0.5 - 100.0).abs() < 1.0, "{total}");
    }

    #[test]
    fn hull() {
        let h = convex_hull(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.5, 0.5]]);
        assert_eq!(h.len(), 4);
        assert!(g::area(&h) > 0.0);
    }
}
