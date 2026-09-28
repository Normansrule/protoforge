//! Parametric generator for the Iris smart-glasses frame.
//!
//! ## Sizing (boxing system, ISO 8624)
//! Frames are labelled `A □ DBL – temple`: **A** is the horizontal width of
//! the box around one lens, **DBL** the distance between lenses (the
//! bridge), and the temple length is measured along the side from the hinge
//! to the tip. The distance between the two lens (box) centres — the
//! *frame PD* — is therefore `A + DBL`.
//!
//! ## Construction
//! Every part is built in its own **print / mold frame**: lying flat on
//! the bed with the mold pull direction along +Z. Each part is a stack of
//! drafted extrusions ([`solid::extrude_draft`]) that meet on flat faces,
//! so every side wall tapers by `draft_deg` relative to +Z (what the
//! molding lab checks) and no shells overlap (so mass by the divergence
//! theorem is exact). A placement transform then puts each part into the
//! assembly frame:
//!
//! * assembly X → wearer's right (display + electronics side), Y → forward
//!   (the wearer looks along +Y, so the temples run towards −Y), Z → up —
//!   a right-handed frame; origin at the bridge centre on the front face,
//!   Z = 0 at the lens-box centre line. (Parts are laid out internally with
//!   the temples along +y and mirrored once at the end.)
//!
//! Parts: `front` (rims, bridge, endpieces, upper/lower hinge knuckles, nose
//! pads), `temple_r` (middle knuckle, hollow electronics pod open to the
//! outside, arm with ear bend, USB-C slot in the pod floor-side wall),
//! `temple_l` (battery pod), `lid_r`, `lid_l` (pod covers with a locating
//! lip; the right lid carries a tongue that closes the USB-C slot above the
//! receptacle) and `optics_housing` (display + collimator enclosure above
//! the right lens, open underneath towards the combiner).

use serde::Serialize;

use super::geom2d::{self as g, P2};
use super::mesh::{Affine, Mesh, P3};
use super::solid::{extrude_draft, Region};
use crate::project::{FrameParams, Project};

/// Clearance between mating molded parts, mm.
pub const CLEARANCE: f64 = 0.15;
/// Length of the hinge knuckles along the temple, mm.
pub const KNUCKLE_LEN: f64 = 5.0;
/// Gap between interleaved knuckles, mm.
pub const KNUCKLE_GAP: f64 = 0.4;
/// USB-C receptacle opening width (8.94 mm shell + clearance), mm.
pub const USB_SLOT_W: f64 = 9.4;
/// Height of a top-mount USB-C receptacle above the board, mm.
pub const USB_RECEPTACLE_H: f64 = 3.3;

/// One generated part in both coordinate frames.
#[derive(Clone, Debug)]
pub struct BuiltPart {
    pub name: String,
    pub label: String,
    pub color: String,
    pub explode: P3,
    /// Part in its print/mold frame (flat on the bed, pull = +Z), centred
    /// on X/Y with min Z = 0.
    pub print: Mesh,
    /// Part in assembly coordinates.
    pub assembly: Mesh,
    /// Number of stacked extrusion shells the part is made of.
    pub shells: usize,
    /// Whether the part belongs to the moving temple group (for folding).
    pub group: &'static str,
    /// Mold pull direction (print-frame +Z) expressed in assembly coordinates.
    pub pull: P3,
}

/// Transparent, non-printed items drawn with the assembly.
#[derive(Clone, Debug, Serialize)]
pub struct DisplayItem {
    pub name: String,
    pub kind: String,
    pub mesh: super::mesh::MeshF32,
    pub color: String,
    pub opacity: f64,
    pub group: String,
}

/// Derived dimensions of the generated frame (all mm, assembly frame).
#[derive(Clone, Debug, Default, Serialize)]
pub struct FrameLayout {
    pub lens_center_x: f64,
    pub frame_pd_mm: f64,
    pub total_width_mm: f64,
    pub front_height_mm: f64,
    pub hinge_z: f64,
    pub pin_x: f64,
    pub pin_y: f64,
    pub temple_inner_x: f64,
    pub pod_front_y: f64,
    pub pod_rear_y: f64,
    /// Electronics-pod cavity (right temple).
    pub pod_cavity_len_rim_mm: f64,
    pub pod_cavity_len_floor_mm: f64,
    pub pod_cavity_height_floor_mm: f64,
    pub pod_cavity_depth_mm: f64,
    pub pod_cavity_corner_r_mm: f64,
    /// Longest board of the project's PCB height that lies flat on the
    /// cavity floor (corner radii taken into account).
    pub pod_usable_len_mm: f64,
    pub pcb_fits_height: bool,
    pub battery_cavity_len_mm: f64,
    pub battery_cavity_height_mm: f64,
    pub battery_cavity_depth_mm: f64,
    pub pcb_center: P3,
    pub pcb_size: P3,
    pub usb_slot_y: [f64; 2],
    pub ear_bend_y: f64,
    pub tilt_deg: f64,
    pub draft_deg: f64,
    pub min_wall_mm: f64,
}

#[derive(Clone, Debug)]
pub struct FrameGeom {
    pub parts: Vec<BuiltPart>,
    pub display: Vec<DisplayItem>,
    pub layout: FrameLayout,
    pub warnings: Vec<String>,
}

/// Parameters after clamping to buildable ranges.
struct Dims {
    a: f64,
    b: f64,
    n: f64,
    dbl: f64,
    rim: f64,
    t: f64,
    temple_len: f64,
    temple_w: f64,
    temple_h: f64,
    pod_len: f64,
    pod_w: f64,
    pod_h: f64,
    batt_len: f64,
    wall: f64,
    draft: f64,
    tilt: f64,
}

fn clampw(v: f64, lo: f64, hi: f64, name: &str, warn: &mut Vec<String>) -> f64 {
    if !v.is_finite() {
        warn.push(format!("{name} is not a number; using {lo}"));
        return lo;
    }
    if v < lo || v > hi {
        let c = v.clamp(lo, hi);
        warn.push(format!("{name} = {v} clamped to {c}"));
        return c;
    }
    v
}

fn dims(f: &FrameParams, warn: &mut Vec<String>) -> Dims {
    let wall = clampw(f.wall_mm, 0.6, 3.0, "wall_mm", warn);
    let temple_w = clampw(f.temple_width_mm, 2.5, 8.0, "temple_width_mm", warn);
    let temple_h = clampw(f.temple_height_mm, 4.0, 16.0, "temple_height_mm", warn);
    let pod_w_min = temple_w + wall + 1.0;
    let pod_w = clampw(f.pod_width_mm, pod_w_min, 20.0, "pod_width_mm", warn);
    let pod_h = clampw(f.pod_height_mm, temple_h.max(2.0 * wall + 4.0), 30.0, "pod_height_mm", warn);
    Dims {
        a: clampw(f.lens_width_mm, 30.0, 70.0, "lens_width_mm", warn) / 2.0,
        b: clampw(f.lens_height_mm, 20.0, 60.0, "lens_height_mm", warn) / 2.0,
        n: clampw(f.lens_shape_exponent, 2.0, 8.0, "lens_shape_exponent", warn),
        dbl: clampw(f.bridge_mm, 10.0, 28.0, "bridge_mm", warn),
        rim: clampw(f.rim_width_mm, 1.5, 12.0, "rim_width_mm", warn),
        t: clampw(f.front_thickness_mm, 2.0, 12.0, "front_thickness_mm", warn),
        temple_len: clampw(f.temple_length_mm, 110.0, 170.0, "temple_length_mm", warn),
        temple_w,
        temple_h,
        pod_len: clampw(f.pod_length_mm, 15.0, 90.0, "pod_length_mm", warn),
        pod_w,
        pod_h,
        batt_len: clampw(f.battery_pod_length_mm, 15.0, 90.0, "battery_pod_length_mm", warn),
        wall,
        draft: clampw(f.draft_deg, 0.0, 10.0, "draft_deg", warn),
        tilt: clampw(f.pantoscopic_tilt_deg, -5.0, 20.0, "pantoscopic_tilt_deg", warn),
    }
}

/// First crossing of the horizontal line `v = level`; picks the crossing
/// with the smallest (or largest) u. Returns (edge index, point).
fn crossing(poly: &[P2], level: f64, min_u: bool) -> Option<(usize, P2)> {
    let n = poly.len();
    let mut best: Option<(usize, P2)> = None;
    for k in 0..n {
        let a = poly[k];
        let b = poly[(k + 1) % n];
        if (a[1] - level) * (b[1] - level) > 0.0 || a[1] == b[1] {
            continue;
        }
        let t = (level - a[1]) / (b[1] - a[1]);
        let p = [a[0] + t * (b[0] - a[0]), level];
        let better = match best {
            None => true,
            Some((_, q)) => {
                if min_u {
                    p[0] < q[0]
                } else {
                    p[0] > q[0]
                }
            }
        };
        if better {
            best = Some((k, p));
        }
    }
    best
}

/// Walk a loop forwards from a point on edge `ka` to a point on edge `kb`.
fn walk(poly: &[P2], from: (usize, P2), to: (usize, P2)) -> Vec<P2> {
    let n = poly.len();
    let mut out = vec![from.1];
    let mut i = (from.0 + 1) % n;
    let mut guard = 0;
    loop {
        out.push(poly[i]);
        if i == to.0 || guard > n {
            break;
        }
        i = (i + 1) % n;
        guard += 1;
    }
    out.push(to.1);
    out
}

fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Front outline in (u = x, v = z): rims around both lenses, the bridge
/// with a nose arch, and endpiece bulges at hinge height.
fn front_outline(d: &Dims, c: f64, v_h: f64) -> (Vec<P2>, Vec<P2>, Vec<P2>) {
    let lens = g::superellipse(d.a, d.b, d.n, 112);
    let rim_right: Vec<P2> = g::offset_vertexwise(&lens, d.rim, 3.0)
        .into_iter()
        .map(|p| {
            // Endpiece: bulge the outer flank around hinge height.
            let reach = d.a + d.rim;
            let wu = smoothstep((p[0] - 0.55 * reach) / (0.45 * reach));
            let he = 0.55 * (d.b + d.rim);
            let dv = (p[1] - v_h) / he;
            let wv = if dv.abs() < 1.0 {
                (0.5 * std::f64::consts::PI * dv).cos().powi(2)
            } else {
                0.0
            };
            [p[0] + c + 3.0 * wu * wv, p[1]]
        })
        .collect();
    let rim_right = g::resample_closed(&rim_right, 128);
    let rim_left: Vec<P2> = g::reversed(&rim_right.iter().map(|p| [-p[0], p[1]]).collect::<Vec<_>>());
    // Bridge levels.
    let top_reach = d.b + d.rim;
    let v_top = (0.62 * top_reach).min(top_reach - 1.0);
    let t_b = (0.9 * d.rim).max(3.5);
    let v_c = v_top - t_b;
    let v_be = (v_c - 0.35 * d.b).max(-(d.b + d.rim) + 2.0);
    let tr = crossing(&rim_right, v_top, true).expect("bridge top meets right rim");
    let br = crossing(&rim_right, v_be, true).expect("bridge bottom meets right rim");
    let tl = crossing(&rim_left, v_top, false).expect("bridge top meets left rim");
    let bl = crossing(&rim_left, v_be, false).expect("bridge bottom meets left rim");
    let mut out: Vec<P2> = Vec::new();
    // Top line, right to left.
    let steps = 6;
    for k in 0..steps {
        out.push(g::lerp(tr.1, tl.1, k as f64 / steps as f64));
    }
    // Left rim, ccw from its top-inner point to its bottom-inner point.
    let lw = walk(&rim_left, tl, bl);
    out.extend_from_slice(&lw[..lw.len() - 1]);
    // Nose arch, left to right: v = v_c + (v_be - v_c)(u/u_be)^2.
    let u_be = br.1[0];
    let ns = 24;
    for k in 0..ns {
        let u = -u_be + 2.0 * u_be * k as f64 / ns as f64;
        let s = u / u_be;
        out.push([u, v_c + (v_be - v_c) * s * s]);
    }
    let rw = walk(&rim_right, br, tr);
    out.extend_from_slice(&rw[..rw.len() - 1]);
    let out = g::simplify(&out, 1e-6);
    let lens_r = g::translate(&lens, c, 0.0);
    let lens_l = g::translate(&lens, -c, 0.0);
    (g::oriented(out, true), lens_l, lens_r)
}

/// The front's 2D profile (outer outline, left and right lens openings) in
/// (x, z) — used by the web page's triangulation walkthrough.
pub fn front_profile(p: &Project) -> (Vec<P2>, Vec<P2>, Vec<P2>) {
    let mut warn = Vec::new();
    let d = dims(&p.design.frame, &mut warn);
    let c = (2.0 * d.a + d.dbl) / 2.0;
    front_outline(&d, c, 0.3 * d.b)
}

/// Longest centred rectangle of height `h` inside a rounded rectangle.
fn usable_length(len: f64, height: f64, r: f64, h: f64) -> f64 {
    if h > height + 1e-9 {
        return 0.0;
    }
    let over = h / 2.0 - (height / 2.0 - r);
    if over <= 0.0 {
        return len;
    }
    let inset = r - (r * r - over * over).max(0.0).sqrt();
    (len - 2.0 * inset).max(0.0)
}

/// Build the whole frame.
pub fn build(p: &Project) -> FrameGeom {
    let f = &p.design.frame;
    let mut warnings = Vec::new();
    let d = dims(f, &mut warnings);
    let tan = d.draft.to_radians().tan();
    let c = (2.0 * d.a + d.dbl) / 2.0;
    let v_h = 0.3 * d.b;
    let (outline, lens_l, lens_r) = front_outline(&d, c, v_h);

    // ---------------- hinge placement
    let hk = d.temple_h.min(d.pod_h);
    let kh = ((hk - 2.0 * KNUCKLE_GAP) / 3.0).max(0.8);
    let mut u_edge = f64::INFINITY;
    for i in 0..=8 {
        let v = v_h - hk / 2.0 + hk * i as f64 / 8.0;
        if let Some((_, q)) = crossing(&outline, v, false) {
            u_edge = u_edge.min(q[0]);
        }
    }
    let u_k1 = u_edge - (d.t * tan + 0.3);
    let u_k0 = u_k1 - d.temple_w;
    let x_in = u_k0;
    let pin_y = d.t + (KNUCKLE_LEN + KNUCKLE_GAP) / 2.0;
    let pod_y0 = d.t + KNUCKLE_LEN + KNUCKLE_GAP;

    // ---------------- front (print frame: u, v, h = thickness)
    let mut front = Mesh::default();
    let front_region = Region::new(outline.clone(), vec![lens_l.clone(), lens_r.clone()]);
    front.append(&extrude_draft(&front_region, 0.0, d.t, d.draft, 0.0));
    let mut front_shells = 1;
    for (v0, v1) in [
        (v_h + kh / 2.0 + KNUCKLE_GAP, v_h + hk / 2.0),
        (v_h - hk / 2.0, v_h - kh / 2.0 - KNUCKLE_GAP),
    ] {
        for side in [1.0, -1.0] {
            let (ua, ub) = if side > 0.0 { (u_k0, u_k1) } else { (-u_k1, -u_k0) };
            let r = g::rounded_rect((ua + ub) / 2.0, (v0 + v1) / 2.0, ub - ua, v1 - v0, 0.4, 3);
            front.append(&extrude_draft(&Region::simple(r), d.t, d.t + KNUCKLE_LEN, d.draft, d.t));
            front_shells += 1;
        }
    }
    // Nose pads on the rear face, in the middle of the rim band.
    if d.rim >= 3.0 {
        let vp = -0.3 * d.b;
        let lens_u = d.a * (1.0 - (vp.abs() / d.b).powf(d.n)).max(0.0).powf(1.0 / d.n);
        let u_pad = c - lens_u - d.rim / 2.0;
        let (ra, rb) = ((d.rim / 2.0 - 0.6).min(1.8), 3.5);
        if u_pad - ra > 0.5 {
            for side in [1.0, -1.0] {
                let e = g::superellipse(ra, rb, 2.0, 32);
                let e = g::translate(&e, side * u_pad, vp);
                front.append(&extrude_draft(&Region::simple(e), d.t, d.t + 3.0, d.draft, d.t));
                front_shells += 1;
            }
        }
    }
    // (u, v, h) -> (x, y, z) = (u, h, v): a mirror, fixed by `transformed`.
    let front_place = Affine::linear([[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]]);

    // ---------------- temples (print frame: p = y, q = z, w = outward from x_in)
    let z_c = v_h;
    let r_p = ((d.pod_h - d.temple_h) / 2.0)
        .clamp(0.0, 3.0)
        .min(d.pod_h / 2.0 - d.wall - 0.3)
        .max(0.0);
    let pocket_inset = d.wall + 2.0 * d.pod_w * tan;
    let temple_for = |pod_len: f64, usb: bool| -> (Mesh, Mesh, usize, usize, [f64; 2]) {
        let pod_y1 = pod_y0 + pod_len;
        // Exact inward offsets of the pod's rounded rectangle.
        let rr = |inset: f64| {
            g::rounded_rect(
                pod_y0 + pod_len / 2.0,
                z_c,
                pod_len - 2.0 * inset,
                d.pod_h - 2.0 * inset,
                (r_p - inset).max(0.0),
                6,
            )
        };
        let pod = rr(0.0);
        // Arm centre line: straight to the ear bend, arc down, tail.
        let l_ear = (0.25 * d.temple_len).clamp(25.0, 45.0);
        let bend_r = 22.0;
        let bend_deg: f64 = 38.0;
        let arc_len = bend_r * bend_deg.to_radians();
        let tail = (l_ear - arc_len).max(5.0);
        let y_bend = pin_y + d.temple_len - l_ear;
        let mut path: Vec<P2> = vec![[pod_y1, z_c]];
        let y_start = pod_y1.max(pin_y + 1.0);
        let straight_to = y_bend.max(y_start + 5.0);
        let nstraight = (((straight_to - y_start) / 12.0).ceil() as usize).max(1);
        for k in 1..=nstraight {
            path.push([y_start + (straight_to - y_start) * k as f64 / nstraight as f64, z_c]);
        }
        let narc = 10;
        for k in 1..=narc {
            let th = bend_deg.to_radians() * k as f64 / narc as f64;
            path.push([straight_to + bend_r * th.sin(), z_c - bend_r * (1.0 - th.cos())]);
        }
        let dir = [bend_deg.to_radians().cos(), -bend_deg.to_radians().sin()];
        let last = *path.last().unwrap();
        for k in 1..=3 {
            path.push(g::add(last, g::scale(dir, tail * k as f64 / 3.0)));
        }
        // Half heights, tapering to 80 % at the tip.
        let mut cum = vec![0.0];
        for i in 1..path.len() {
            cum.push(cum[i - 1] + g::dist(path[i], path[i - 1]));
        }
        let total = *cum.last().unwrap();
        let half: Vec<f64> = cum.iter().map(|s| d.temple_h / 2.0 * (1.0 - 0.2 * s / total)).collect();
        let band = g::band_outline(&path, &half, 12);
        // band = right edge (bottom) → cap → left edge (top) reversed: its
        // first point is the bottom edge at the pod, its last the top edge.
        // Temple outline S: pod + forward lug + band.
        let lug = [[d.t + KNUCKLE_GAP, z_c - kh / 2.0], [pod_y0, z_c + kh / 2.0]];
        let mut s: Vec<P2> = Vec::new();
        // Pod loop starts on the bottom edge at the right end (see rounded_rect).
        // Walk the pod from its bottom-right corner up to the rear face at the
        // band bottom, insert the band, continue to the front face, insert lug.
        let n = pod.len();
        // rounded_rect emits four corners with the same number of points,
        // starting with the bottom-right corner at the bottom edge.
        let seg = n / 4;
        s.extend_from_slice(&pod[0..seg]); // bottom-right corner, ends on rear face
        s.push([pod_y1, z_c - half[0]]);
        s.extend_from_slice(&band[1..band.len() - 1]);
        s.push([pod_y1, z_c + half[0]]);
        s.extend_from_slice(&pod[seg..3 * seg]); // top-right + top-left corners
                                                 // front face going down: insert lug
        s.push([pod_y0, lug[1][1]]);
        s.push([lug[0][0], lug[1][1]]);
        s.push([lug[0][0], lug[0][1]]);
        s.push([pod_y0, lug[0][1]]);
        s.extend_from_slice(&pod[3 * seg..n]);
        let s = g::simplify(&s, 1e-7);
        // Pocket reference outline (at z = 0) chosen so the wall is exactly
        // `wall` at the rim after both sides taper.
        let pocket = rr(pocket_inset);
        let mut m = Mesh::default();
        let s_region = Region::new(s.clone(), vec![]);
        m.append(&extrude_draft(&s_region, 0.0, d.wall, d.draft, 0.0));
        m.append(&extrude_draft(
            &Region::new(s, vec![pocket.clone()]),
            d.wall,
            d.temple_w,
            d.draft,
            0.0,
        ));
        let mut slot = [0.0, 0.0];
        let ring = if usb {
            // Ring with a slot through the bottom wall near the rear.
            let s1 = pod_y1 - pocket_inset - 3.0;
            let s0 = (s1 - USB_SLOT_W).max(pod_y0 + r_p + pocket_inset);
            slot = [s0, s1];
            let q_out = z_c - d.pod_h / 2.0;
            let q_in = q_out + pocket_inset;
            let mut r: Vec<P2> = vec![[s1, q_out]];
            r.extend_from_slice(&pod);
            r.push([s0, q_out]);
            r.push([s0, q_in]);
            // Pocket traversed clockwise, starting just left of s0 on its bottom edge.
            r.extend(pocket.iter().rev().copied());
            r.push([s1, q_in]);
            Region::new(g::simplify(&r, 1e-7), vec![])
        } else {
            Region::new(pod.clone(), vec![pocket.clone()])
        };
        m.append(&extrude_draft(&ring, d.temple_w, d.pod_w, d.draft, 0.0));
        // Lid (own print frame: p, -q, z' from its outer face).
        let wl = d.wall;
        let rim_outline = rr(d.pod_w * tan);
        let rim_inset = pocket_inset - d.pod_w * tan;
        let mirror = |poly: &[P2]| -> Vec<P2> { g::reversed(&poly.iter().map(|p| [p[0], -p[1]]).collect::<Vec<_>>()) };
        let mut lid = Mesh::default();
        lid.append(&extrude_draft(&Region::simple(mirror(&rim_outline)), 0.0, wl, d.draft, 0.0));
        let lip_depth = 1.5f64.min(d.pod_w - d.temple_w - 0.5);
        let lip_w = 0.9f64.min(0.25 * d.pod_h);
        let lip_outer = rr(rim_inset + CLEARANCE);
        let lip_inner = rr(rim_inset + CLEARANCE + lip_w);
        lid.append(&extrude_draft(
            &Region::new(mirror(&lip_outer), vec![mirror(&lip_inner)]),
            wl,
            wl + lip_depth,
            d.draft,
            wl,
        ));
        let mut lid_shells = 2;
        if usb {
            let receptacle_top = d.temple_w + USB_RECEPTACLE_H + 0.2;
            let depth = d.pod_w - receptacle_top;
            if depth > 0.5 {
                let q_out = z_c - d.pod_h / 2.0 + d.pod_w * tan;
                let q_in = q_out + d.wall;
                let tongue = vec![
                    [slot[0] + CLEARANCE, q_out + 0.05],
                    [slot[1] - CLEARANCE, q_out + 0.05],
                    [slot[1] - CLEARANCE, q_in - 0.02],
                    [slot[0] + CLEARANCE, q_in - 0.02],
                ];
                lid.append(&extrude_draft(&Region::simple(mirror(&tongue)), wl, wl + depth, d.draft, wl));
                lid_shells += 1;
            }
        }
        (m, lid, 3, lid_shells, slot)
    };
    let (temple_r, lid_r, tr_shells, lr_shells, usb_slot) = temple_for(d.pod_len, true);
    let (temple_l, lid_l, tl_shells, ll_shells, _) = temple_for(d.batt_len, false);
    // (p, q, w) -> (x_in + w, p, q)   and the mirrored left side.
    let temple_place_r = Affine {
        m: [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        t: [x_in, 0.0, 0.0],
    };
    let temple_place_l = Affine {
        m: [[0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        t: [-x_in, 0.0, 0.0],
    };
    // Lid frame: (p, q', z') -> (x_in + pod_w + wl - z', p, -q')
    let wl = d.wall;
    let lid_place_r = Affine {
        m: [[0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0]],
        t: [x_in + d.pod_w + wl, 0.0, 0.0],
    };
    let lid_place_l = Affine {
        m: [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0]],
        t: [-(x_in + d.pod_w + wl), 0.0, 0.0],
    };

    // ---------------- optics housing (tray open downwards)
    let o = &p.design.optics;
    let ow = d.wall;
    let bore = o.lens_diameter_mm.max(o.display_height_mm).clamp(4.0, 30.0);
    let oy = bore + 2.0 * ow; // depth along Y
    let oz = bore + 2.0 * ow; // height
    let x0 = c - 3.0;
    let x1 = (x0 + o.display_to_lens_mm.clamp(5.0, 60.0) + 6.0).min(x_in - 1.0);
    let ox = (x1 - x0).max(10.0);
    let o_outline = g::rounded_rect(0.0, 0.0, ox, oy, 2.0, 5);
    let o_in = ow + 2.0 * oz * tan;
    let o_pocket = g::rounded_rect(0.0, 0.0, ox - 2.0 * o_in, oy - 2.0 * o_in, (2.0 - o_in).max(0.0), 5);
    let mut optics = Mesh::default();
    optics.append(&extrude_draft(&Region::simple(o_outline.clone()), 0.0, ow, d.draft, 0.0));
    optics.append(&extrude_draft(&Region::new(o_outline, vec![o_pocket]), ow, oz, d.draft, 0.0));
    let top_reach = d.b + d.rim;
    let o_top = top_reach - 1.0;
    // (x', y', z') -> (xc + x', yc - y', o_top - z')
    let optics_place = Affine {
        m: [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]],
        t: [x0 + ox / 2.0, d.t + oy / 2.0, o_top],
    };

    // ---------------- pantoscopic tilt about the hinge pin line
    let tilt = Affine::translate(0.0, -pin_y, -v_h)
        .then(&Affine::rot_x(d.tilt))
        .then(&Affine::translate(0.0, pin_y, v_h));

    let mut parts = Vec::new();
    // Internal layout has the temples along +y; mirror y so +Y is forward
    // and +X really is the wearer's right.
    let mirror = Affine::scale(1.0, -1.0, 1.0);
    let mut push = |name: &str, label: &str, color: &str, explode: P3, local: Mesh, place: Affine, shells: usize, group: &'static str| {
        let world = place.then(&mirror);
        let assembly = local.transformed(&world);
        let pull = super::mesh::v_norm(world.apply_linear([0.0, 0.0, 1.0]));
        let explode = [explode[0], -explode[1], explode[2]];
        let (lo, hi) = local.bbox();
        let print = local.translated(-(lo[0] + hi[0]) / 2.0, -(lo[1] + hi[1]) / 2.0, -lo[2]);
        parts.push(BuiltPart {
            name: name.into(),
            label: label.into(),
            color: color.into(),
            explode,
            print,
            assembly,
            shells,
            group,
            pull,
        });
    };
    push(
        "front",
        "Front (rims, bridge, knuckles, nose pads)",
        "#5d7aa8",
        [0.0, -22.0, 0.0],
        front,
        front_place.then(&tilt),
        front_shells,
        "front",
    );
    push(
        "temple_r",
        "Right temple + electronics pod",
        "#3e4c64",
        [26.0, 8.0, 0.0],
        temple_r,
        temple_place_r,
        tr_shells,
        "right",
    );
    push(
        "temple_l",
        "Left temple + battery pod",
        "#3e4c64",
        [-26.0, 8.0, 0.0],
        temple_l,
        temple_place_l,
        tl_shells,
        "left",
    );
    push(
        "lid_r",
        "Electronics pod lid",
        "#c98f4a",
        [52.0, 8.0, 0.0],
        lid_r,
        lid_place_r,
        lr_shells,
        "right",
    );
    push(
        "lid_l",
        "Battery pod lid",
        "#c98f4a",
        [-52.0, 8.0, 0.0],
        lid_l,
        lid_place_l,
        ll_shells,
        "left",
    );
    push(
        "optics_housing",
        "Display optics housing",
        "#8b9db5",
        [0.0, 14.0, 28.0],
        optics,
        optics_place.then(&tilt),
        2,
        "front",
    );

    // ---------------- layout numbers
    let pod_y1 = pod_y0 + d.pod_len;
    let floor_inset = pocket_inset - d.wall * tan; // pocket at z = wall
    let cav_len_floor = d.pod_len - 2.0 * floor_inset;
    let cav_h_floor = d.pod_h - 2.0 * floor_inset;
    let cav_r = (r_p - floor_inset).max(0.0);
    let pcb = &p.design.pcb;
    let usable = usable_length(cav_len_floor, cav_h_floor, cav_r, pcb.height_mm);
    let pcb_t = pcb.thickness_mm.clamp(0.2, 3.0);
    let pcb_center = [x_in + d.temple_w - pcb_t / 2.0, -(pod_y0 + pod_y1) / 2.0, z_c];
    let pcb_size = [pcb_t, pcb.width_mm, pcb.height_mm];
    let (lo, hi) = parts[0].assembly.bbox();
    let layout = FrameLayout {
        lens_center_x: c,
        frame_pd_mm: 2.0 * c,
        total_width_mm: hi[0] - lo[0],
        front_height_mm: hi[2] - lo[2],
        hinge_z: v_h,
        pin_x: x_in + d.temple_w / 2.0,
        pin_y: -pin_y,
        temple_inner_x: x_in,
        pod_front_y: -pod_y0,
        pod_rear_y: -pod_y1,
        pod_cavity_len_rim_mm: d.pod_len - 2.0 * (d.wall + d.pod_w * tan),
        pod_cavity_len_floor_mm: cav_len_floor,
        pod_cavity_height_floor_mm: cav_h_floor,
        pod_cavity_depth_mm: d.pod_w - d.wall,
        pod_cavity_corner_r_mm: cav_r,
        pod_usable_len_mm: usable,
        pcb_fits_height: pcb.height_mm <= cav_h_floor,
        battery_cavity_len_mm: d.batt_len - 2.0 * floor_inset,
        battery_cavity_height_mm: cav_h_floor,
        battery_cavity_depth_mm: d.pod_w - d.wall,
        pcb_center,
        pcb_size,
        usb_slot_y: [-usb_slot[1], -usb_slot[0]],
        ear_bend_y: -(pin_y + d.temple_len - (0.25 * d.temple_len).clamp(25.0, 45.0)),
        tilt_deg: d.tilt,
        draft_deg: d.draft,
        min_wall_mm: d.wall,
    };
    if !layout.pcb_fits_height {
        warnings.push(format!(
            "PCB height {:.1} mm exceeds the pod cavity height {:.1} mm at the floor",
            pcb.height_mm, cav_h_floor
        ));
    }

    // ---------------- display-only items: demo lenses, combiner, PCB
    let mut display = Vec::new();
    for (name, lens) in [("lens_l", &lens_l), ("lens_r", &lens_r)] {
        let lm = extrude_draft(&Region::simple(g::offset_vertexwise(lens, 0.3, 3.0)), 0.0, 1.8, 0.0, 0.0)
            .translated(0.0, 0.0, d.t / 2.0 - 0.9)
            .transformed(&front_place.then(&tilt).then(&mirror));
        display.push(DisplayItem {
            name: name.into(),
            kind: "lens".into(),
            mesh: lm.to_f32(),
            color: "#9ecbff".into(),
            opacity: 0.22,
            group: "front".into(),
        });
    }
    // 45° combiner plate hanging below the optics housing, in front of the pupil.
    let comb_w = bore * 1.2;
    let comb = super::solid::box_mesh([-comb_w / 2.0, -0.6, -comb_w / 2.0], [comb_w / 2.0, 0.6, comb_w / 2.0])
        .transformed(&Affine::rot_z(45.0))
        .translated(c, d.t + oy / 2.0, o_top - oz - comb_w / 2.0 - 0.5)
        .transformed(&tilt.then(&mirror));
    display.push(DisplayItem {
        name: "combiner".into(),
        kind: "combiner".into(),
        mesh: comb.to_f32(),
        color: "#7fd7ff".into(),
        opacity: 0.35,
        group: "front".into(),
    });
    let board = super::solid::box_mesh(
        [
            pcb_center[0] - pcb_size[0] / 2.0,
            pcb_center[1] - pcb_size[1] / 2.0,
            pcb_center[2] - pcb_size[2] / 2.0,
        ],
        [
            pcb_center[0] + pcb_size[0] / 2.0,
            pcb_center[1] + pcb_size[1] / 2.0,
            pcb_center[2] + pcb_size[2] / 2.0,
        ],
    );
    display.push(DisplayItem {
        name: "pcb".into(),
        kind: "pcb".into(),
        mesh: board.to_f32(),
        color: "#2fbf71".into(),
        opacity: 0.55,
        group: "right".into(),
    });

    FrameGeom {
        parts,
        display,
        layout,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usable_length_geometry() {
        // Board lower than the straight part of the wall: full length.
        assert_eq!(usable_length(60.0, 12.0, 2.0, 8.0), 60.0);
        // Board taller than the cavity: nothing fits.
        assert_eq!(usable_length(60.0, 12.0, 2.0, 13.0), 0.0);
        // Board reaching into the corner arcs.
        let u = usable_length(60.0, 12.0, 2.0, 11.0);
        let over: f64 = 5.5 - 4.0;
        let exact = 60.0 - 2.0 * (2.0 - (4.0 - over * over).sqrt());
        assert!((u - exact).abs() < 1e-12);
    }

    #[test]
    fn iris_parts_are_watertight() {
        let geom = build(&Project::iris());
        assert!(geom.warnings.is_empty(), "{:?}", geom.warnings);
        for part in &geom.parts {
            let c = part.print.validate();
            assert!(c.manifold && c.oriented && c.signed_volume_mm3 > 0.0, "{}: {c:?}", part.name);
            assert_eq!(c.shells, part.shells, "{}", part.name);
            let a = part.assembly.validate();
            assert!(a.watertight, "{} assembly {a:?}", part.name);
            assert!((a.signed_volume_mm3 - c.signed_volume_mm3).abs() < 1e-6 * c.signed_volume_mm3);
            let (lo, _) = part.print.bbox();
            assert!(lo[2].abs() < 1e-9);
        }
    }

    #[test]
    fn frame_pd_follows_boxing_system() {
        let p = Project::iris();
        let geom = build(&p);
        let f = &p.design.frame;
        assert!((geom.layout.frame_pd_mm - (f.lens_width_mm + f.bridge_mm)).abs() < 1e-9);
        assert!(geom.layout.total_width_mm > 2.0 * f.lens_width_mm + f.bridge_mm);
    }

    #[test]
    fn front_volume_matches_analytic_estimate() {
        // With zero draft the front plate is a prism: volume = area × t.
        let mut p = Project::iris();
        p.design.frame.draft_deg = 0.0;
        let mut warn = vec![];
        let d = dims(&p.design.frame, &mut warn);
        let c = (2.0 * d.a + d.dbl) / 2.0;
        let (outline, ll, lr) = front_outline(&d, c, 0.3 * d.b);
        let area = g::area(&outline) + g::area(&g::oriented(ll, false)) + g::area(&g::oriented(lr, false));
        let plate = extrude_draft(&Region::new(outline, vec![]), 0.0, d.t, 0.0, 0.0);
        assert!(plate.validate().watertight);
        assert!(area > 0.0);
        // Lens openings: superellipse area.
        let lens_area = g::superellipse_area(d.a, d.b, d.n);
        let ring_area = g::area(&Region::new(g::superellipse(d.a, d.b, d.n, 112), vec![]).outer);
        assert!((ring_area - lens_area).abs() / lens_area < 1e-3);
    }
}
