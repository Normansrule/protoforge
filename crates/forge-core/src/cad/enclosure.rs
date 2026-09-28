//! Generic parametric enclosure: a box with rounded corners, filleted
//! bottom edge, wall cut-outs (USB, buttons, LEDs), M3 screw bosses in the
//! corners, PCB standoffs and a screw-on (or lip-located) lid.
//!
//! Anything you build needs a case; this is the "build anything" companion
//! to the Iris frame generator. The body is a stack of drafted extrusions:
//! a floor slab (with a quarter-circle fillet profile), then wall-ring
//! slabs split at every cut-out's bottom/top so each cut-out is a clean
//! rectangular window (region booleans by the winding rule, see
//! [`super::clip`]). Bosses and standoffs are tubes standing on the floor.
//!
//! Design rules of thumb (FDM): walls ≥ 2 line widths (≈ 1.2 mm with a
//! 0.4 mm nozzle; 2 mm is sturdy), M3 thread-forming screws want a
//! ≈ 2.5 mm pilot hole in a ≥ 6 mm boss, keep ≥ 0.2 mm clearance between
//! mating printed parts. Reference: I. Gibson, D. Rosen, B. Stucker,
//! *Additive Manufacturing Technologies*, 2nd ed., Springer 2015, ch. 17
//! (design for AM).

use serde::{Deserialize, Serialize};

use super::clip;
use super::geom2d::{self as g, P2};
use super::mesh::{Affine, MassProps, Mesh, P3};
use super::solid::{extrude_draft, loft_region, tube, Level, Region};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Cutout {
    /// Which wall: "x+", "x-", "y+", "y-".
    pub face: String,
    /// Centre offset along the wall from its middle, mm.
    pub offset_mm: f64,
    /// Bottom of the opening above the floor's top surface, mm.
    pub z_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub label: String,
}

impl Default for Cutout {
    fn default() -> Self {
        Cutout { face: "x-".into(), offset_mm: 0.0, z_mm: 6.0, width_mm: 9.5, height_mm: 3.8, label: "USB-C".into() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PcbMount {
    pub enabled: bool,
    pub length_mm: f64,
    pub width_mm: f64,
    pub thickness_mm: f64,
    /// Mounting holes inset from the board corners, mm.
    pub hole_inset_mm: f64,
    pub standoff_h_mm: f64,
    pub standoff_d_mm: f64,
    pub standoff_hole_mm: f64,
}

impl Default for PcbMount {
    fn default() -> Self {
        PcbMount { enabled: true, length_mm: 60.0, width_mm: 40.0, thickness_mm: 1.6, hole_inset_mm: 3.5, standoff_h_mm: 4.0, standoff_d_mm: 5.5, standoff_hole_mm: 2.2 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EnclosureParams {
    pub inner_length_mm: f64,
    pub inner_width_mm: f64,
    pub inner_height_mm: f64,
    pub wall_mm: f64,
    pub floor_mm: f64,
    pub lid_mm: f64,
    pub corner_radius_mm: f64,
    pub fillet_mm: f64,
    pub draft_deg: f64,
    pub clearance_mm: f64,
    pub bosses: bool,
    pub boss_d_mm: f64,
    pub boss_hole_mm: f64,
    pub screw_clear_mm: f64,
    pub lip_mm: f64,
    pub pcb: PcbMount,
    pub cutouts: Vec<Cutout>,
    pub density_g_cm3: f64,
}

impl Default for EnclosureParams {
    fn default() -> Self {
        EnclosureParams {
            inner_length_mm: 72.0,
            inner_width_mm: 52.0,
            inner_height_mm: 22.0,
            wall_mm: 2.0,
            floor_mm: 2.0,
            lid_mm: 2.0,
            corner_radius_mm: 5.0,
            fillet_mm: 1.5,
            draft_deg: 0.0,
            clearance_mm: 0.2,
            bosses: true,
            boss_d_mm: 6.5,
            boss_hole_mm: 2.5,
            screw_clear_mm: 3.4,
            lip_mm: 2.0,
            pcb: PcbMount::default(),
            cutouts: vec![
                Cutout::default(),
                Cutout { face: "y-".into(), offset_mm: 18.0, z_mm: 10.0, width_mm: 3.2, height_mm: 3.2, label: "LED".into() },
            ],
            density_g_cm3: 1.24,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct EnclosurePart {
    pub name: String,
    pub color: String,
    pub explode: P3,
    pub assembly: super::mesh::MeshF32,
    pub mass: MassProps,
    pub watertight: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct EnclosureOut {
    pub parts: Vec<EnclosurePart>,
    pub outer_size_mm: P3,
    pub interior_mm: P3,
    pub pcb_center: P3,
    pub pcb_size: P3,
    pub total_mass_g: f64,
    pub warnings: Vec<String>,
}

/// Built enclosure meshes (print frames and assembly frames).
pub struct Built {
    pub body_print: Mesh,
    pub lid_print: Mesh,
    pub body: Mesh,
    pub lid: Mesh,
    pub outer: P3,
    pub pcb_center: P3,
    pub pcb_size: P3,
    pub warnings: Vec<String>,
}

fn fillet_levels(z0: f64, fillet: f64, z1: f64, tan: f64, steps: usize) -> Vec<Level> {
    // Quarter-circle offset profile from z0 up to z0 + fillet, then straight.
    let mut lv = Vec::new();
    if fillet > 1e-6 {
        for i in 0..=steps {
            let a = std::f64::consts::FRAC_PI_2 * i as f64 / steps as f64;
            let z = z0 + fillet * (1.0 - a.cos());
            lv.push(Level { z, d_outer: -(fillet - fillet * a.sin()) - z * tan, d_holes: -z * tan });
        }
    } else {
        lv.push(Level { z: z0, d_outer: -z0 * tan, d_holes: -z0 * tan });
    }
    if z1 > lv.last().unwrap().z + 1e-9 {
        lv.push(Level { z: z1, d_outer: -z1 * tan, d_holes: -z1 * tan });
    }
    lv
}

/// Group boolean output loops into regions (each ccw loop with the cw
/// loops inside it).
fn regions_from_loops(loops: Vec<Vec<P2>>) -> Vec<Region> {
    let (outers, holes): (Vec<_>, Vec<_>) = loops.into_iter().partition(|l| g::area(l) > 0.0);
    let mut regs: Vec<Region> = outers.into_iter().map(|o| Region { outer: o, holes: vec![] }).collect();
    for h in holes {
        let p = h[0];
        let mut best: Option<usize> = None;
        let mut best_a = f64::INFINITY;
        for (i, r) in regs.iter().enumerate() {
            if g::point_in_polygon(p, &r.outer) {
                let a = g::area(&r.outer);
                if a < best_a {
                    best_a = a;
                    best = Some(i);
                }
            }
        }
        if let Some(i) = best {
            regs[i].holes.push(h);
        }
    }
    regs
}

pub fn build(p: &EnclosureParams) -> Built {
    let mut warnings = Vec::new();
    let w = p.wall_mm.clamp(0.8, 6.0);
    let fl = p.floor_mm.clamp(0.8, 6.0);
    let lid_t = p.lid_mm.clamp(0.8, 6.0);
    let li = p.inner_length_mm.clamp(10.0, 250.0);
    let wi = p.inner_width_mm.clamp(10.0, 250.0);
    let hi = p.inner_height_mm.clamp(3.0, 150.0);
    let r = p.corner_radius_mm.clamp(0.0, 0.5 * (li + 2.0 * w).min(wi + 2.0 * w) - 0.5);
    let tan = p.draft_deg.clamp(0.0, 5.0).to_radians().tan();
    let lo = li + 2.0 * w;
    let wo = wi + 2.0 * w;
    let h_body = fl + hi;
    let seg = 8;
    let outer = g::rounded_rect(0.0, 0.0, lo, wo, r, seg);
    // Pocket reference at z = 0 so the wall is `w` at the rim after draft.
    let inset = w + 2.0 * h_body * tan;
    let pocket = g::rounded_rect(0.0, 0.0, lo - 2.0 * inset, wo - 2.0 * inset, (r - inset).max(0.0), seg);
    let fil = p.fillet_mm.clamp(0.0, fl.min(r.max(0.0)) - 0.05).max(0.0);
    let mut body = Mesh::default();
    body.append(&loft_region(&Region::simple(outer.clone()), &fillet_levels(0.0, fil, fl, tan, 6)));
    // Cut-out rectangles in plan, as (loop, z0, z1) with z from the bed.
    let mut cuts: Vec<(Vec<P2>, f64, f64)> = Vec::new();
    for c in &p.cutouts {
        let (along, half_len) = match c.face.as_str() {
            "x+" | "x-" => ('y', wi / 2.0),
            _ => ('x', li / 2.0),
        };
        let straight = half_len - (r - w).max(0.0);
        let cw = c.width_mm.max(0.5);
        if c.offset_mm.abs() + cw / 2.0 > straight + 1e-9 {
            warnings.push(format!("cut-out '{}' runs into a rounded corner; skipped", c.label));
            continue;
        }
        let z0 = fl + c.z_mm.max(0.0);
        let z1 = (z0 + c.height_mm.max(0.5)).min(h_body);
        if z0 >= h_body - 0.2 {
            warnings.push(format!("cut-out '{}' is above the wall; skipped", c.label));
            continue;
        }
        let e = w + 1.0; // reach through the wall
        let rect = match (c.face.as_str(), along) {
            ("x+", _) => [lo / 2.0 - e, c.offset_mm - cw / 2.0, lo / 2.0 + 1.0, c.offset_mm + cw / 2.0],
            ("x-", _) => [-lo / 2.0 - 1.0, c.offset_mm - cw / 2.0, -lo / 2.0 + e, c.offset_mm + cw / 2.0],
            ("y+", _) => [c.offset_mm - cw / 2.0, wo / 2.0 - e, c.offset_mm + cw / 2.0, wo / 2.0 + 1.0],
            _ => [c.offset_mm - cw / 2.0, -wo / 2.0 - 1.0, c.offset_mm + cw / 2.0, -wo / 2.0 + e],
        };
        let lp = vec![[rect[0], rect[1]], [rect[2], rect[1]], [rect[2], rect[3]], [rect[0], rect[3]]];
        cuts.push((lp, z0, z1));
    }
    // Wall slabs between breakpoints.
    let mut zs: Vec<f64> = vec![fl, h_body];
    for (_, a, b) in &cuts {
        zs.push(*a);
        zs.push(*b);
    }
    zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    zs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    let ring = vec![outer.clone(), g::reversed(&pocket)];
    for win in zs.windows(2) {
        let (z0, z1) = (win[0], win[1]);
        let mid = 0.5 * (z0 + z1);
        let active: Vec<Vec<P2>> = cuts.iter().filter(|(_, a, b)| *a <= mid && mid <= *b).map(|(l, _, _)| l.clone()).collect();
        if active.is_empty() {
            body.append(&extrude_draft(&Region::new(outer.clone(), vec![pocket.clone()]), z0, z1, p.draft_deg, 0.0));
        } else {
            for reg in regions_from_loops(clip::difference(&ring, &active)) {
                body.append(&extrude_draft(&reg, z0, z1, p.draft_deg, 0.0));
            }
        }
    }
    // Corner screw bosses (tangent to both walls) and PCB standoffs.
    let bd = p.boss_d_mm.clamp(3.0, 20.0);
    let mut boss_xy: Vec<P2> = Vec::new();
    if p.bosses {
        // Tangent to both walls; if the boss is smaller than the inner
        // corner radius, slide it along the diagonal until it touches the arc.
        let rho = bd / 2.0;
        let ri = (r - w).max(0.0);
        let (bx, by) = if rho >= ri {
            (li / 2.0 - rho, wi / 2.0 - rho)
        } else {
            let k = (ri - rho) * std::f64::consts::FRAC_1_SQRT_2;
            (li / 2.0 - ri + k, wi / 2.0 - ri + k)
        };
        for (sx, sy) in [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)] {
            boss_xy.push([sx * bx, sy * by]);
            body.append(&tube(sx * bx, sy * by, bd / 2.0, p.boss_hole_mm.clamp(1.0, bd - 1.0) / 2.0, fl, h_body, 40));
        }
    }
    let pcb = &p.pcb;
    let pcb_z = fl + pcb.standoff_h_mm.max(0.0);
    if pcb.enabled {
        if pcb.length_mm > li - 1.0 || pcb.width_mm > wi - 1.0 {
            warnings.push("PCB is larger than the interior".into());
        }
        let hx = pcb.length_mm / 2.0 - pcb.hole_inset_mm;
        let hy = pcb.width_mm / 2.0 - pcb.hole_inset_mm;
        let sd = pcb.standoff_d_mm.clamp(2.0, 12.0);
        for (sx, sy) in [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)] {
            let c = [sx * hx, sy * hy];
            if boss_xy.iter().any(|b| g::dist(*b, c) < (bd + sd) / 2.0 + 0.2) {
                warnings.push("a PCB standoff collides with a screw boss; skipped".into());
                continue;
            }
            if pcb.standoff_h_mm > 0.2 {
                body.append(&tube(c[0], c[1], sd / 2.0, pcb.standoff_hole_mm.clamp(0.8, sd - 1.0) / 2.0, fl, pcb_z, 32));
            }
        }
        if pcb_z + pcb.thickness_mm > h_body {
            warnings.push("PCB on standoffs is taller than the walls".into());
        }
    }
    // Lid: plate (outer outline at the rim) with screw clearance holes;
    // print frame has the outer face on the bed with the edge fillet there.
    let rim_outer = g::rounded_rect(0.0, 0.0, lo - 2.0 * h_body * tan, wo - 2.0 * h_body * tan, (r - h_body * tan).max(0.0), seg);
    let holes: Vec<Vec<P2>> = if p.bosses {
        boss_xy.iter().map(|b| g::circle(b[0], b[1], p.screw_clear_mm.clamp(1.0, bd - 0.8) / 2.0, 28)).collect()
    } else {
        vec![]
    };
    let lid_fil = p.fillet_mm.clamp(0.0, lid_t.min(r.max(0.0)) - 0.05).max(0.0);
    let mut lid = loft_region(&Region::new(rim_outer, holes), &fillet_levels(0.0, lid_fil, lid_t, 0.0, 6));
    if !p.bosses && p.lip_mm > 0.2 {
        let rim_in = w + h_body * tan + p.clearance_mm;
        let lw = 1.2f64.min(w);
        let l_out = g::rounded_rect(0.0, 0.0, lo - 2.0 * rim_in, wo - 2.0 * rim_in, (r - rim_in).max(0.0), seg);
        let l_in = g::rounded_rect(0.0, 0.0, lo - 2.0 * (rim_in + lw), wo - 2.0 * (rim_in + lw), (r - rim_in - lw).max(0.0), seg);
        lid.append(&extrude_draft(&Region::new(l_out, vec![l_in]), lid_t, lid_t + p.lip_mm, p.draft_deg, lid_t));
    }
    // Assembly: body as printed; lid flipped onto the rim.
    let lid_asm = lid.transformed(&Affine::rot_x(180.0).then(&Affine::translate(0.0, 0.0, h_body + lid_t)));
    let pcb_center = [0.0, 0.0, pcb_z + pcb.thickness_mm / 2.0];
    Built {
        body_print: body.clone(),
        lid_print: lid,
        body,
        lid: lid_asm,
        outer: [lo, wo, h_body + lid_t],
        pcb_center,
        pcb_size: [pcb.length_mm, pcb.width_mm, pcb.thickness_mm],
        warnings,
    }
}

pub fn evaluate(p: &EnclosureParams) -> EnclosureOut {
    let b = build(p);
    let mk = |name: &str, color: &str, m: &Mesh, explode: P3| EnclosurePart {
        name: name.into(),
        color: color.into(),
        explode,
        assembly: m.to_f32(),
        mass: m.mass_props(p.density_g_cm3),
        watertight: {
            let c = m.validate();
            c.manifold && c.oriented && c.signed_volume_mm3 > 0.0
        },
    };
    let parts = vec![mk("body", "#4f6d9a", &b.body, [0.0, 0.0, 0.0]), mk("lid", "#c98f4a", &b.lid, [0.0, 0.0, 30.0])];
    let total = parts.iter().map(|x| x.mass.mass_g).sum();
    EnclosureOut {
        parts,
        outer_size_mm: b.outer,
        interior_mm: [p.inner_length_mm, p.inner_width_mm, p.inner_height_mm],
        pcb_center: b.pcb_center,
        pcb_size: b.pcb_size,
        total_mass_g: total,
        warnings: b.warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_enclosure_is_closed_and_sane() {
        let p = EnclosureParams::default();
        let b = build(&p);
        assert!(b.warnings.is_empty(), "{:?}", b.warnings);
        for m in [&b.body, &b.lid, &b.body_print, &b.lid_print] {
            let c = m.validate();
            assert!(c.manifold && c.oriented && c.signed_volume_mm3 > 0.0, "{c:?}");
        }
        // Volume sanity: between the wall-shell estimate with and without bosses.
        let lo = 76.0;
        let wo = 56.0;
        let shell = g::rounded_rect_area(lo, wo, 5.0) * 24.0 - g::rounded_rect_area(72.0, 52.0, 3.0) * 22.0;
        let v = b.body.volume();
        assert!(v > 0.9 * shell && v < 1.4 * shell, "{v} vs {shell}");
    }

    #[test]
    fn cutout_removes_the_window_volume() {
        let mut p = EnclosureParams { bosses: false, fillet_mm: 0.0, cutouts: vec![], ..Default::default() };
        p.pcb.enabled = false;
        let v0 = build(&p).body.volume();
        p.cutouts = vec![Cutout { face: "x+".into(), offset_mm: 0.0, z_mm: 5.0, width_mm: 10.0, height_mm: 4.0, label: "w".into() }];
        let b = build(&p);
        assert!(b.body.validate().manifold);
        let removed = v0 - b.body.volume();
        assert!((removed - 10.0 * 4.0 * 2.0).abs() < 1e-6, "removed {removed}");
    }
}
