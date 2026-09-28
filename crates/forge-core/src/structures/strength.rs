//! Strength analysis of the real Iris CAD parts: named load cases, voxel
//! models, the solve, stress/safety-factor post-processing and data for the
//! 3D view (heat map, exaggerated deformation, load and support markers).
//!
//! ## Load cases (assumptions)
//! Frame deformation tests for spectacle frames exist in ISO 12870
//! (*Ophthalmic optics — Spectacle frames — Requirements and test methods*);
//! the cases here are *inspired by* that kind of test, not an implementation
//! of any clause:
//!
//! * **temple_spread** — putting the glasses on spreads each temple. The
//!   temple's hinge end (knuckle lug and the first 1.5 mm of the pod, which
//!   butts against the front's hinge stop) is clamped; 5 N acts outward on the
//!   last 4 mm of the temple tip.
//! * **bridge_bend** — both endpieces (the outer 2 mm of the rim at the
//!   hinges plus the knuckles) are clamped; 10 N pushes the bridge towards the
//!   face (a finger pushing the glasses up the nose, or pressing on the
//!   bridge).
//! * **pod_squeeze** — a finger and thumb pinch the electronics pod: 10 N on
//!   a 10 mm patch at mid-length of the top wall and 10 N on the bottom wall,
//!   towards each other (self-equilibrated); hinge end clamped. The lid is a
//!   separate snap-in part and is conservatively assumed to carry no load.
//! * **drop** — a 1.5 m drop landing on the outer face of the electronics
//!   pod, handled in [`drop_analysis`] with a 1-degree-of-freedom spring
//!   model built on the stiffness this solver computes for a lateral load at
//!   the pod.
//!
//! The analysis frame is the assembly frame (mm; X to the wearer's right, Y
//! forward, Z up); the front is first rotated back by the pantoscopic tilt so
//! its thickness lies along Y and the voxels align with its faces.

use serde::{Deserialize, Serialize};

use super::fea3d::{self, Bcs, Grid, Precond, SolveOpts};
use super::hex8::UnitHex;
use super::materials::{self, StructMaterial};
use super::voxel;
use crate::cad::frame::FrameLayout;
use crate::cad::mesh::{Affine, Mesh, P3};
use crate::project::Project;

/// Named load case metadata (for the UI).
#[derive(Clone, Debug, Serialize)]
pub struct CaseInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub parts: Vec<&'static str>,
    pub default_force_n: f64,
    pub description: &'static str,
}

pub fn cases() -> Vec<CaseInfo> {
    vec![
        CaseInfo {
            id: "temple_spread",
            label: "Temple spread (putting the glasses on)",
            parts: vec!["temple_r", "temple_l"],
            default_force_n: 5.0,
            description: "Hinge end clamped (knuckle lug + pod front wall against the hinge stop); outward force on the last 4 mm of the temple tip.",
        },
        CaseInfo {
            id: "bridge_bend",
            label: "Bridge bend (pushing on the bridge)",
            parts: vec!["front"],
            default_force_n: 10.0,
            description: "Both endpieces and hinge knuckles clamped; the bridge is pushed towards the face.",
        },
        CaseInfo {
            id: "pod_squeeze",
            label: "Pod squeeze (finger pinch)",
            parts: vec!["temple_r", "temple_l"],
            default_force_n: 10.0,
            description: "Finger and thumb pinch the pod's top and bottom walls at mid-length (10 mm patches, equal and opposite); hinge end clamped; lid assumed not to help.",
        },
        CaseInfo {
            id: "drop_side",
            label: "Drop on the pod (stiffness for the impact model)",
            parts: vec!["temple_r", "temple_l"],
            default_force_n: 1.0,
            description: "Hinge end clamped; lateral force on the middle third of the pod's outer rim (where the lid lands). The drop model scales this unit-load result by the impact force.",
        },
    ]
}

/// A ready-to-solve voxel model of one part under one load case.
pub struct Model {
    pub part: String,
    pub case: String,
    pub grid: Grid,
    pub bcs: Bcs,
    pub fixed_voxels: Vec<usize>,
    pub load_groups: Vec<(Vec<usize>, [f64; 3])>,
    pub removed_voxels: usize,
    /// Print build direction (layer normal) in the analysis frame.
    pub build_dir: P3,
    /// The CAD part in the analysis frame.
    pub mesh: Mesh,
    pub layout: FrameLayout,
    pub total_force_n: f64,
}

/// Rotation that removes the front's pantoscopic tilt (about the hinge line).
pub fn untilt(l: &FrameLayout) -> Affine {
    Affine::translate(0.0, -l.pin_y, -l.hinge_z)
        .then(&Affine::rot_x(l.tilt_deg))
        .then(&Affine::translate(0.0, l.pin_y, l.hinge_z))
}

fn part_mesh(p: &Project, part: &str) -> Result<(Mesh, P3, FrameLayout), String> {
    let geom = crate::cad::frame_geom(p);
    let l = geom.layout.clone();
    let b = geom
        .parts
        .iter()
        .find(|b| b.name == part)
        .ok_or_else(|| format!("unknown part `{part}`"))?;
    if part == "front" {
        let t = untilt(&l);
        Ok((b.assembly.transformed(&t), crate::cad::mesh::v_norm(t.apply_linear(b.pull)), l))
    } else {
        Ok((b.assembly.clone(), b.pull, l))
    }
}

/// Crop a mesh-voxel grid: clear voxels whose centre fails `keep`.
fn crop(g: &mut Grid, keep: impl Fn([f64; 3]) -> bool) {
    for e in 0..g.solid.len() {
        if g.solid[e] && !keep(g.center(e)) {
            g.solid[e] = false;
        }
    }
}

fn select(g: &Grid, pred: impl Fn([f64; 3]) -> bool) -> Vec<usize> {
    (0..g.solid.len()).filter(|&e| g.solid[e] && pred(g.center(e))).collect()
}

/// Grid anchors that align voxel faces with the part's slender members: the
/// temple arm's inner face (X) and the front's back face (Y, after untilt).
fn anchors(mesh: &Mesh, part: &str, l: &FrameLayout) -> [Option<f64>; 3] {
    let mut lo = f64::INFINITY;
    for p in mesh.positions.chunks(3) {
        match part {
            "front" => {
                if p[0].abs() < 4.0 {
                    lo = lo.min(p[1]);
                }
            }
            _ => {
                if p[1] < l.pod_rear_y - 2.0 {
                    lo = lo.min(p[0]);
                }
            }
        }
    }
    if !lo.is_finite() {
        return [None; 3];
    }
    match part {
        "front" => [None, Some(lo), None],
        _ => [Some(lo), None, None],
    }
}

/// Choose the voxel size: the requested one, coarsened until the estimated
/// element count fits the budget.
pub fn pick_h(volume_mm3: f64, requested: f64, max_elems: usize) -> f64 {
    let floor = (volume_mm3 / max_elems.max(100) as f64).cbrt();
    requested.max(floor).max(0.2)
}

/// Build the voxel model of `part` for load case `case` with total force `force_n`.
pub fn build_model(p: &Project, part: &str, case: &str, h_req: f64, max_elems: usize, force_n: f64) -> Result<Model, String> {
    let info = cases()
        .into_iter()
        .find(|c| c.id == case)
        .ok_or_else(|| format!("unknown load case `{case}`"))?;
    if !info.parts.contains(&part) {
        return Err(format!("load case `{case}` applies to {:?}, not `{part}`", info.parts));
    }
    let (mesh, build_dir, l) = part_mesh(p, part)?;
    let vol = mesh.volume().abs();
    let squeeze = case == "pod_squeeze";
    // The pod-squeeze model keeps only the pod (+3 mm of arm): scale the budget estimate.
    let vol_est = if squeeze { vol * 0.55 } else { vol };
    let mut h = pick_h(vol_est, h_req, max_elems);
    let side = if part == "temple_l" { -1.0 } else { 1.0 };
    let mut attempt = 0;
    let anchor = anchors(&mesh, part, &l);
    loop {
        let mut g = voxel::voxelize_aligned(&mesh, h, anchor);
        if squeeze {
            let y_cut = l.pod_rear_y - 3.0;
            crop(&mut g, |c| c[1] >= y_cut);
        }
        if g.count() > max_elems + max_elems / 8 && attempt < 6 {
            h *= 1.08 * (g.count() as f64 / max_elems as f64).cbrt();
            attempt += 1;
            continue;
        }
        let pin = [side * l.pin_x, l.pin_y, l.hinge_z];
        // ---- supports
        let fixed = match part {
            "front" => {
                let xf = l.temple_inner_x - 2.0;
                select(&g, |c| c[0].abs() >= xf)
            }
            _ => {
                let yf = l.pod_front_y - 1.5;
                select(&g, |c| c[1] >= yf)
            }
        };
        if fixed.is_empty() {
            return Err("no voxels in the support region — voxel size too coarse".into());
        }
        let removed = g.keep_connected(&fixed);
        let fixed: Vec<usize> = fixed.into_iter().filter(|&e| g.solid[e]).collect();
        // ---- loads
        let mut groups: Vec<(Vec<usize>, [f64; 3])> = Vec::new();
        let (lo, hi) = extent(&g);
        let y_mid = 0.5 * (l.pod_front_y + l.pod_rear_y);
        match case {
            "temple_spread" => {
                let dist = |c: [f64; 3]| ((c[1] - pin[1]).powi(2) + (c[2] - pin[2]).powi(2)).sqrt();
                let dmax = (0..g.solid.len())
                    .filter(|&e| g.solid[e])
                    .map(|e| dist(g.center(e)))
                    .fold(0.0, f64::max);
                let tip = select(&g, |c| dist(c) >= dmax - 4.0);
                groups.push((tip, [side * force_n, 0.0, 0.0]));
            }
            "bridge_bend" => {
                let br = select(&g, |c| c[0].abs() <= 3.0);
                groups.push((br, [0.0, -force_n, 0.0]));
            }
            "pod_squeeze" => {
                let top = select(&g, |c| c[2] >= hi[2] - 1.6 * h.max(1.0) && (c[1] - y_mid).abs() <= 5.0);
                let bot = select(&g, |c| c[2] <= lo[2] + 1.6 * h.max(1.0) && (c[1] - y_mid).abs() <= 5.0);
                groups.push((top, [0.0, 0.0, -force_n]));
                groups.push((bot, [0.0, 0.0, force_n]));
            }
            "drop_side" => {
                let pod_len = (l.pod_front_y - l.pod_rear_y).abs();
                let outer = if side > 0.0 { hi[0] } else { lo[0] };
                let patch = select(&g, |c| {
                    (c[0] - outer).abs() <= 1.6 * h.max(1.0) && (c[1] - y_mid).abs() <= pod_len / 6.0
                });
                groups.push((patch, [-side * force_n, 0.0, 0.0]));
            }
            _ => unreachable!(),
        }
        if groups.iter().any(|(v, _)| v.is_empty()) {
            return Err("no voxels in the load region — voxel size too coarse".into());
        }
        let mut bcs = Bcs::new(&g);
        bcs.fix_voxels(&g, &fixed, 7);
        for (v, f) in &groups {
            bcs.load_voxels(&g, v, *f);
        }
        return Ok(Model {
            part: part.into(),
            case: case.into(),
            grid: g,
            bcs,
            fixed_voxels: fixed,
            load_groups: groups,
            removed_voxels: removed,
            build_dir,
            mesh,
            layout: l,
            total_force_n: force_n,
        });
    }
}

fn extent(g: &Grid) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for e in 0..g.solid.len() {
        if g.solid[e] {
            let c = g.center(e);
            for d in 0..3 {
                lo[d] = lo[d].min(c[d]);
                hi[d] = hi[d].max(c[d]);
            }
        }
    }
    (lo, hi)
}

// ---------------------------------------------------------------- results

#[derive(Clone, Debug, Default, Serialize)]
pub struct SurfaceField {
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
    /// Displacement per vertex (mm), 3 per vertex.
    pub disp: Vec<f32>,
    /// Averaged von Mises stress per vertex (MPa).
    pub vm: Vec<f32>,
    /// Layer-aware utilisation per vertex (stress ÷ allowable; 1 = yield).
    pub util: Vec<f32>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Marker {
    pub pos: P3,
    pub dir: P3,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Hint {
    pub region: String,
    pub mechanism: String,
    pub advice: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct FeaResult {
    pub part: String,
    pub case: String,
    pub material: String,
    pub printed: bool,
    pub h_mm: f64,
    pub dims: [usize; 3],
    pub elements: usize,
    pub nodes: usize,
    pub free_dofs: usize,
    pub removed_voxels: usize,
    pub levels: usize,
    pub precond: String,
    pub element: String,
    pub iterations: usize,
    pub converged: bool,
    pub history: Vec<f64>,
    pub force_n: f64,
    /// Design stress: max nodal von Mises outside the sharp-corner zones and the clamps.
    pub max_vm_mpa: f64,
    pub max_vm_pos: P3,
    /// Raw nodal peak including stair-step corners.
    pub peak_vm_mpa: f64,
    pub peak_vm_pos: P3,
    pub max_disp_mm: f64,
    /// Mean displacement of the loaded nodes along the load direction, mm.
    pub load_disp_mm: f64,
    /// Force ÷ load-point displacement, N/mm.
    pub stiffness_n_mm: f64,
    pub yield_mpa: f64,
    pub z_factor: f64,
    /// Safety factor on von Mises vs yield (isotropic).
    pub sf_von_mises: f64,
    /// Safety factor including the print-layer (Z) knock-down.
    pub safety_factor: f64,
    /// "in-plane" or "across layers".
    pub governing: String,
    pub crit_pos: P3,
    pub max_across_layers_mpa: f64,
    pub hint: Hint,
    pub build_dir: P3,
    pub fixed: Vec<P3>,
    pub loads: Vec<Marker>,
    pub bbox_min: P3,
    pub bbox_max: P3,
    pub voxels: Option<SurfaceField>,
    pub cad: Option<SurfaceField>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct FeaIn {
    pub project: Option<Project>,
    pub part: String,
    pub case: String,
    pub material: Option<String>,
    pub voxel_mm: f64,
    pub max_elements: usize,
    pub force_n: Option<f64>,
    pub precond: String,
    pub element: String,
    pub tol: f64,
    pub max_iter: usize,
    /// "voxels", "cad", "both" or "none".
    pub surface: String,
}

impl Default for FeaIn {
    fn default() -> Self {
        FeaIn {
            project: None,
            part: "temple_r".into(),
            case: "temple_spread".into(),
            material: None,
            voxel_mm: 1.0,
            max_elements: 12_000,
            force_n: None,
            precond: "multigrid".into(),
            element: "h8i".into(),
            tol: 1e-6,
            max_iter: 3000,
            surface: "both".into(),
        }
    }
}

/// Everything needed to post-process a solved model.
pub struct Solved {
    pub model: Model,
    pub u: Vec<f64>,
    pub st: fea3d::Stresses,
    pub hist: Vec<f64>,
    pub converged: bool,
    pub levels: usize,
    pub free_dofs: usize,
    pub mat: &'static StructMaterial,
}

pub fn solve_model(model: Model, mat: &'static StructMaterial, precond: Precond, incompatible: bool, tol: f64, max_iter: usize) -> Solved {
    let opts = SolveOpts {
        e_mpa: mat.e_mpa(),
        nu: mat.nu,
        incompatible,
        precond,
        tol,
        max_iter,
    };
    let hex = UnitHex::new(mat.nu, incompatible);
    let s = fea3d::solve(&model.grid, &model.bcs, &opts, &hex);
    let st = fea3d::stresses(&model.grid, &hex, opts.e_mpa, &s.u, model.build_dir);
    Solved {
        model,
        u: s.u,
        st,
        hist: s.history,
        converged: s.converged,
        levels: s.levels,
        free_dofs: s.free_dofs,
        mat,
    }
}

/// Summary numbers of a solved model (cheap; no surface data).
///
/// **Design stress.** Nodal (corner-extrapolated) stresses are accurate on
/// smooth surfaces — they carry the outer-fibre bending stress — but at sharp
/// re-entrant edges they are *singular*: they grow without bound as the
/// voxels shrink. The design stress is the largest nodal von Mises stress
/// outside a one-voxel zone around every re-entrant edge (nodes shared by 6
/// or 7 solid voxels) and away from the clamps. At the metric resolution
/// (two voxels through the member) that zone is half a member thick, so the
/// design stress is the *nominal* stress beside a sharp corner — the right
/// basis for the static strength of ductile thermoplastics, where local
/// yielding redistributes the peak (Shigley ch. 3 & 5). The excluded peak is
/// reported separately as the stress-concentration warning; fatigue and
/// brittle materials need a notch factor on top (see `structures.stress_conc`).
pub struct Summary {
    /// Design von Mises stress (nodal, singular corners excluded), MPa.
    pub max_vm: f64,
    pub max_vm_pos: P3,
    /// Raw nodal peak including sharp corners, MPa.
    pub peak_vm: f64,
    pub peak_node: usize,
    /// Safety factor on yield including the print-layer check.
    pub sf: f64,
    /// Safety factor on von Mises only (isotropic).
    pub sf_vm: f64,
    /// Governing node and its stresses.
    pub crit_pos: P3,
    pub crit_vm: f64,
    pub crit_sb: f64,
    pub across: bool,
    pub max_across: f64,
    pub load_disp: f64,
    pub max_disp: f64,
}

/// Nodes that count for the design stress: attached, not clamped, and not
/// within one voxel of a re-entrant edge.
pub fn design_nodes(g: &Grid, fixed: &[u8]) -> Vec<bool> {
    let val = g.node_valence();
    let mut ok: Vec<bool> = (0..g.nnodes()).map(|n| val[n] > 0 && fixed[n] == 0).collect();
    for e in 0..g.solid.len() {
        if !g.solid[e] {
            continue;
        }
        let nodes = g.elem_nodes(e);
        if nodes.iter().any(|&n| val[n as usize] == 6 || val[n as usize] == 7) {
            for n in nodes {
                ok[n as usize] = false;
            }
        }
    }
    // Tiny or all-corner parts: fall back to every attached, free node.
    if !ok.iter().any(|v| *v) {
        ok = (0..g.nnodes()).map(|n| val[n] > 0 && fixed[n] == 0).collect();
    }
    ok
}

pub fn summarize(s: &Solved) -> Summary {
    let g = &s.model.grid;
    let y = s.mat.yield_mpa;
    let zf = s.mat.z_strength_factor;
    let ok = design_nodes(g, &s.model.bcs.fixed);
    let mut worst = 0.0;
    let mut crit = usize::MAX;
    let mut across = false;
    let mut max_across: f64 = 0.0;
    let mut max_design: f64 = 0.0;
    let mut max_design_n = usize::MAX;
    for n in 0..g.nnodes() {
        let vm = s.st.node_vm[n];
        if !ok[n] || vm == 0.0 {
            continue;
        }
        if vm > max_design {
            max_design = vm;
            max_design_n = n;
        }
        let sb = s.st.node_sbuild[n].max(0.0);
        max_across = max_across.max(sb);
        let (u_in, u_z) = (vm / y, sb / (zf * y));
        let u = u_in.max(u_z);
        if u > worst {
            worst = u;
            crit = n;
            across = u_z > u_in;
        }
    }
    // Load-point displacement along the load direction:
    // work-conjugate displacement fᵀu / Σ|F_group| per group.
    let ld: f64 = s.model.bcs.force.iter().zip(&s.u).map(|(f, u)| f * u).sum();
    let ftot: f64 = s
        .model
        .load_groups
        .iter()
        .map(|(_, f)| (f[0] * f[0] + f[1] * f[1] + f[2] * f[2]).sqrt())
        .sum();
    let load_disp = if ftot > 0.0 {
        ld / ftot * s.model.load_groups.len() as f64
    } else {
        0.0
    };
    let max_disp =
        s.u.chunks(3)
            .map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
            .fold(0.0, f64::max);
    let pos = |n: usize| if n == usize::MAX { [0.0; 3] } else { g.node_pos(n) };
    Summary {
        max_vm: max_design,
        max_vm_pos: pos(max_design_n),
        peak_vm: s.st.max_vm.max(max_design),
        peak_node: s.st.max_node,
        sf: if worst > 0.0 { 1.0 / worst } else { f64::INFINITY },
        sf_vm: if max_design > 0.0 { y / max_design } else { f64::INFINITY },
        crit_pos: pos(crit),
        crit_vm: if crit == usize::MAX { 0.0 } else { s.st.node_vm[crit] },
        crit_sb: if crit == usize::MAX { 0.0 } else { s.st.node_sbuild[crit] },
        across,
        max_across,
        load_disp,
        max_disp,
    }
}

fn region_name(part: &str, p: P3, l: &FrameLayout) -> (&'static str, bool) {
    // Returns (region, is re-entrant junction).
    if part == "front" {
        if p[0].abs() < 12.0 {
            return ("bridge (between the lenses)", false);
        }
        if p[0].abs() > l.temple_inner_x - 5.0 {
            return ("endpiece next to the hinge knuckles", true);
        }
        return ("rim around the lens", false);
    }
    let y = p[1];
    if y > l.pod_front_y - 2.5 {
        return ("hinge end of the pod (next to the clamped hinge stop)", true);
    }
    if (y - l.pod_rear_y).abs() < 4.0 {
        return ("pod-to-arm junction (where the slim temple leaves the pod)", true);
    }
    if y > l.pod_rear_y {
        return ("electronics pod wall", false);
    }
    if (y - l.ear_bend_y).abs() < 10.0 {
        return ("ear bend of the temple arm", false);
    }
    ("temple arm", false)
}

pub fn hint_for(s: &Solved, sm: &Summary) -> Hint {
    let (region, junction) = region_name(&s.model.part, sm.crit_pos, &s.model.layout);
    let mat = s.mat;
    let mechanism = if sm.across {
        format!(
            "Tension across the print layers: {:.1} MPa pulls the layers apart where the interlayer strength is only ≈ {:.0} % of the in-plane {:.0} MPa — layer-line cracking starts here.",
            sm.crit_sb,
            mat.z_strength_factor * 100.0,
            mat.yield_mpa
        )
    } else {
        format!(
            "Bending stress peaks here: von Mises {:.1} MPa vs {:.0} MPa yield. Stress in a bent section grows with the distance from the neutral axis, so the outer fibres of the thinnest section see it first.",
            sm.crit_vm, mat.yield_mpa
        )
    };
    let mut advice = String::new();
    if junction {
        advice.push_str("It is a sharp inside corner: the local peak there is a stress concentration (singular in a sharp model — the raw nodal peak keeps rising as voxels shrink). Add a fillet (radius ≥ half the wall) before trusting it in fatigue. ");
    }
    match (s.model.case.as_str(), sm.across) {
        (_, true) => advice.push_str("Re-orient the part on the bed so the layers run along the load path, switch to an isotropic process (MJF PA12 or molding), or thicken the section."),
        ("temple_spread", _) => advice.push_str("Bending stress ∝ 1/(h·w²): a thicker temple (temple_width_mm) helps most; a taller one (temple_height_mm) helps linearly and barely changes the look."),
        ("bridge_bend", _) => advice.push_str("Widen the rim band (rim_width_mm) or thicken the front (front_thickness_mm) — the bridge section is what bends."),
        ("pod_squeeze", _) => advice.push_str("The open pod walls bend like cantilevers from the floor: thicken the wall (wall_mm), add an internal rib, or let the lid's lip support the wall edges."),
        _ => advice.push_str("Increase the section at this location or choose a tougher material."),
    }
    Hint {
        region: region.into(),
        mechanism,
        advice,
    }
}

/// Voxel boundary surface with per-vertex fields.
fn voxel_surface(s: &Solved, util: &[f64]) -> SurfaceField {
    let g = &s.model.grid;
    let mut map = vec![u32::MAX; g.nnodes()];
    let mut out = SurfaceField::default();
    let solid = |i: isize, j: isize, k: isize| -> bool {
        i >= 0
            && j >= 0
            && k >= 0
            && (i as usize) < g.nx
            && (j as usize) < g.ny
            && (k as usize) < g.nz
            && g.solid[g.eid(i as usize, j as usize, k as usize)]
    };
    let mut vid = |n: usize, out: &mut SurfaceField| -> u32 {
        if map[n] == u32::MAX {
            map[n] = (out.positions.len() / 3) as u32;
            let p = g.node_pos(n);
            out.positions.extend(p.iter().map(|v| *v as f32));
            out.disp.extend(s.u[3 * n..3 * n + 3].iter().map(|v| *v as f32));
            out.vm.push(s.st.node_vm[n] as f32);
            out.util.push(util[n] as f32);
        }
        map[n]
    };
    for e in 0..g.solid.len() {
        if !g.solid[e] {
            continue;
        }
        let [i, j, k] = g.ecoord(e);
        let (ii, jj, kk) = (i as isize, j as isize, k as isize);
        let n = |a: usize, b: usize, c: usize| g.nid(i + a, j + b, k + c);
        let faces: [(bool, [usize; 4]); 6] = [
            (!solid(ii - 1, jj, kk), [n(0, 0, 0), n(0, 0, 1), n(0, 1, 1), n(0, 1, 0)]),
            (!solid(ii + 1, jj, kk), [n(1, 0, 0), n(1, 1, 0), n(1, 1, 1), n(1, 0, 1)]),
            (!solid(ii, jj - 1, kk), [n(0, 0, 0), n(1, 0, 0), n(1, 0, 1), n(0, 0, 1)]),
            (!solid(ii, jj + 1, kk), [n(0, 1, 0), n(0, 1, 1), n(1, 1, 1), n(1, 1, 0)]),
            (!solid(ii, jj, kk - 1), [n(0, 0, 0), n(0, 1, 0), n(1, 1, 0), n(1, 0, 0)]),
            (!solid(ii, jj, kk + 1), [n(0, 0, 1), n(1, 0, 1), n(1, 1, 1), n(0, 1, 1)]),
        ];
        for (open, q) in faces {
            if open {
                let v: Vec<u32> = q.iter().map(|&x| vid(x, &mut out)).collect();
                out.indices.extend_from_slice(&[v[0], v[1], v[2], v[0], v[2], v[3]]);
            }
        }
    }
    out
}

/// Map nodal fields onto the original CAD mesh by trilinear interpolation
/// over the attached nodes of the voxel containing each vertex.
fn cad_surface(s: &Solved, util: &[f64]) -> SurfaceField {
    let g = &s.model.grid;
    let mut attached = vec![false; g.nnodes()];
    for e in 0..g.solid.len() {
        if g.solid[e] {
            for n in g.elem_nodes(e) {
                attached[n as usize] = true;
            }
        }
    }
    let m = &s.model.mesh;
    let nv = m.vertex_count();
    let mut out = SurfaceField {
        positions: m.positions.iter().map(|v| *v as f32).collect(),
        indices: m.indices.clone(),
        disp: Vec::with_capacity(3 * nv),
        vm: Vec::with_capacity(nv),
        util: Vec::with_capacity(nv),
    };
    for vi in 0..nv {
        let p = m.vertex(vi as u32);
        let f: [f64; 3] = std::array::from_fn(|d| ((p[d] - g.origin[d]) / g.h).clamp(0.0, [g.nx, g.ny, g.nz][d] as f64 - 1e-9));
        let base = [f[0] as usize, f[1] as usize, f[2] as usize];
        let t = [f[0] - base[0] as f64, f[1] - base[1] as f64, f[2] - base[2] as f64];
        let mut acc = [0.0f64; 5];
        let mut wsum = 0.0;
        for c in 0..8 {
            let o = [c & 1, (c >> 1) & 1, (c >> 2) & 1];
            let (i, j, k) = (base[0] + o[0], base[1] + o[1], base[2] + o[2]);
            let n = g.nid(i, j, k);
            if !attached[n] {
                continue;
            }
            let w = (if o[0] == 1 { t[0] } else { 1.0 - t[0] })
                * (if o[1] == 1 { t[1] } else { 1.0 - t[1] })
                * (if o[2] == 1 { t[2] } else { 1.0 - t[2] })
                + 1e-9;
            acc[0] += w * s.u[3 * n];
            acc[1] += w * s.u[3 * n + 1];
            acc[2] += w * s.u[3 * n + 2];
            acc[3] += w * s.st.node_vm[n];
            acc[4] += w * util[n];
            wsum += w;
        }
        if wsum == 0.0 {
            // Search the 5 × 5 × 5 neighbourhood for the nearest attached node.
            let mut best = (f64::INFINITY, usize::MAX);
            for dk in -2i64..=3 {
                for dj in -2i64..=3 {
                    for di in -2i64..=3 {
                        let (i, j, k) = (base[0] as i64 + di, base[1] as i64 + dj, base[2] as i64 + dk);
                        if i < 0 || j < 0 || k < 0 || i > g.nx as i64 || j > g.ny as i64 || k > g.nz as i64 {
                            continue;
                        }
                        let n = g.nid(i as usize, j as usize, k as usize);
                        if attached[n] {
                            let q = g.node_pos(n);
                            let d = (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) + (q[2] - p[2]).powi(2);
                            if d < best.0 {
                                best = (d, n);
                            }
                        }
                    }
                }
            }
            if best.1 != usize::MAX {
                let n = best.1;
                acc = [s.u[3 * n], s.u[3 * n + 1], s.u[3 * n + 2], s.st.node_vm[n], util[n]];
                wsum = 1.0;
            }
        }
        let k = if wsum > 0.0 { 1.0 / wsum } else { 0.0 };
        out.disp.extend([(acc[0] * k) as f32, (acc[1] * k) as f32, (acc[2] * k) as f32]);
        out.vm.push((acc[3] * k) as f32);
        out.util.push((acc[4] * k) as f32);
    }
    out
}

/// Full result for the UI.
pub fn result(s: &Solved, surface: &str, precond: Precond, incompatible: bool) -> FeaResult {
    let g = &s.model.grid;
    let sm = summarize(s);
    let y = s.mat.yield_mpa;
    let zf = s.mat.z_strength_factor;
    let util: Vec<f64> = (0..g.nnodes())
        .map(|n| {
            let vm = s.st.node_vm[n];
            if vm == 0.0 {
                0.0
            } else {
                (vm / y).max(s.st.node_sbuild[n].max(0.0) / (zf * y))
            }
        })
        .collect();
    let mut fixed = Vec::new();
    let stride = (s.model.fixed_voxels.len() / 350).max(1);
    for (i, &e) in s.model.fixed_voxels.iter().enumerate() {
        if i % stride == 0 {
            fixed.push(g.center(e));
        }
    }
    let mut loads = Vec::new();
    for (vox, f) in &s.model.load_groups {
        let fm = (f[0] * f[0] + f[1] * f[1] + f[2] * f[2]).sqrt().max(1e-12);
        let dir = [f[0] / fm, f[1] / fm, f[2] / fm];
        let stride = (vox.len() / 6).max(1);
        for (i, &e) in vox.iter().enumerate() {
            if i % stride == 0 {
                loads.push(Marker { pos: g.center(e), dir });
            }
        }
    }
    let (lo, hi) = extent(g);
    let want_vox = surface == "voxels" || surface == "both";
    let want_cad = surface == "cad" || surface == "both";
    let mut notes = vec![
        "Linear elastic, small displacement, isotropic stiffness. The heat map shows nodal averages of corner stresses; the design stress and safety factor skip a one-voxel zone around sharp inside edges and the clamps.".to_string(),
        "Sharp inside corners are stress singularities: the raw nodal peak there grows as voxels shrink — compare trends, and fillet real corners.".to_string(),
    ];
    if s.mat.printed {
        notes.push(format!(
            "{} values are typical for FDM/powder prints and strongly orientation dependent (Z is weakest): stress across the layers is checked against {:.0} % of the in-plane strength.",
            s.mat.name,
            zf * 100.0
        ));
    }
    if s.model.removed_voxels > 0 {
        notes.push(format!(
            "{} voxels not connected to the supports (thin features below the voxel size) were dropped.",
            s.model.removed_voxels
        ));
    }
    FeaResult {
        part: s.model.part.clone(),
        case: s.model.case.clone(),
        material: s.mat.id.into(),
        printed: s.mat.printed,
        h_mm: g.h,
        dims: [g.nx, g.ny, g.nz],
        elements: g.count(),
        nodes: g.nnodes(),
        free_dofs: s.free_dofs,
        removed_voxels: s.model.removed_voxels,
        levels: s.levels,
        precond: if precond == Precond::Jacobi {
            "jacobi".into()
        } else {
            "multigrid".into()
        },
        element: if incompatible { "h8i".into() } else { "h8".into() },
        iterations: s.hist.len().saturating_sub(1),
        converged: s.converged,
        history: s.hist.clone(),
        force_n: s.model.total_force_n,
        max_vm_mpa: sm.max_vm,
        max_vm_pos: sm.max_vm_pos,
        peak_vm_mpa: sm.peak_vm,
        peak_vm_pos: g.node_pos(sm.peak_node),
        max_disp_mm: sm.max_disp,
        load_disp_mm: sm.load_disp,
        stiffness_n_mm: if sm.load_disp > 0.0 {
            s.model.total_force_n / sm.load_disp
        } else {
            0.0
        },
        yield_mpa: y,
        z_factor: zf,
        sf_von_mises: sm.sf_vm,
        safety_factor: sm.sf,
        governing: if sm.across { "across layers".into() } else { "in-plane".into() },
        crit_pos: sm.crit_pos,
        max_across_layers_mpa: sm.max_across,
        hint: hint_for(s, &sm),
        build_dir: s.model.build_dir,
        fixed,
        loads,
        bbox_min: lo,
        bbox_max: hi,
        voxels: want_vox.then(|| voxel_surface(s, &util)),
        cad: want_cad.then(|| cad_surface(s, &util)),
        notes,
    }
}

pub fn run(i: FeaIn) -> Result<FeaResult, String> {
    let p = i.project.unwrap_or_else(Project::iris);
    let mat = materials::get(i.material.as_deref().unwrap_or(&p.design.frame.material));
    let default_f = cases()
        .into_iter()
        .find(|c| c.id == i.case)
        .map(|c| c.default_force_n)
        .unwrap_or(1.0);
    let force = i.force_n.unwrap_or(default_f);
    if !(force.is_finite() && force > 0.0) {
        return Err("force_n must be positive".into());
    }
    let h = if i.voxel_mm.is_finite() && i.voxel_mm > 0.0 {
        i.voxel_mm
    } else {
        1.0
    };
    let max_el = i.max_elements.clamp(200, 60_000);
    let model = build_model(&p, &i.part, &i.case, h, max_el, force)?;
    let pc = if i.precond == "jacobi" {
        Precond::Jacobi
    } else {
        Precond::Multigrid
    };
    let inc = i.element != "h8";
    let tol = if i.tol > 0.0 { i.tol.clamp(1e-12, 1e-2) } else { 1e-6 };
    let s = solve_model(model, mat, pc, inc, tol, i.max_iter.clamp(10, 20_000));
    Ok(result(&s, &i.surface, pc, inc))
}

// ---------------------------------------------------------------- drop

#[derive(Clone, Debug, Default, Serialize)]
pub struct DropResult {
    pub height_m: f64,
    pub impact_speed_m_s: f64,
    pub energy_j: f64,
    /// Mass that decelerates through the struck pod and hinge, g.
    pub moving_mass_g: f64,
    pub stiffness_n_mm: f64,
    pub peak_force_n: f64,
    pub deflection_mm: f64,
    pub contact_time_ms: f64,
    pub peak_accel_g: f64,
    pub stress_per_n_mpa: f64,
    pub peak_stress_mpa: f64,
    pub safety_factor: f64,
    pub governing: String,
    pub assumptions: Vec<String>,
}

/// 1-degree-of-freedom impact: the moving mass m hits at v = √(2gh) and is
/// stopped by the structural spring k found by FEA. All kinetic energy goes
/// into strain energy, ½ m v² = ½ k δ², so δ = v √(m/k), F = k δ = v √(k m),
/// contact time ≈ π √(m/k) (half a period). Linear FEA scales with load, so
/// stress = (stress per newton) × F. This is an **upper bound** on force:
/// contact and floor compliance, rotation, rebound and material damping all
/// lower it; polymers are also somewhat stronger at impact strain rates.
pub fn drop_analysis(height_m: f64, moving_mass_g: f64, k_n_mm: f64, stress_per_n: f64, sf_per_n: f64, governing: &str) -> DropResult {
    let v = (2.0 * 9.81 * height_m.max(0.0)).sqrt();
    let m = moving_mass_g / 1000.0;
    let k = k_n_mm * 1000.0; // N/m
    let f = v * (k * m).sqrt();
    let delta_mm = if k > 0.0 { f / k * 1000.0 } else { 0.0 };
    let t = if k > 0.0 {
        std::f64::consts::PI * (m / k).sqrt() * 1000.0
    } else {
        0.0
    };
    let peak = stress_per_n * f;
    DropResult {
        height_m,
        impact_speed_m_s: v,
        energy_j: 0.5 * m * v * v,
        moving_mass_g,
        stiffness_n_mm: k_n_mm,
        peak_force_n: f,
        deflection_mm: delta_mm,
        contact_time_ms: t,
        peak_accel_g: if m > 0.0 { f / m / 9.81 } else { 0.0 },
        stress_per_n_mpa: stress_per_n,
        peak_stress_mpa: peak,
        safety_factor: if f > 0.0 { sf_per_n / f } else { f64::INFINITY },
        governing: governing.into(),
        assumptions: vec![
            "Energy method: all impact energy becomes elastic strain energy of the struck temple (no floor/contact compliance, no rebound or rotation) — an upper bound on the force.".into(),
            "Moving mass = everything that decelerates through the struck pod and hinge (front, other temple, lids, optics housing and the components not inside the struck pod).".into(),
            "Static linear FEA scaled to the peak force; strain-rate strengthening of polymers is ignored (conservative).".into(),
            "Verify with a physical drop test on the finished product; this estimate ranks designs, it does not certify them.".into(),
        ],
    }
}

/// Mass (g) that decelerates through the right pod when it lands.
pub fn moving_mass_g(p: &Project, struck: &str) -> f64 {
    let geom = crate::cad::frame_geom(p);
    let dens = p.design.frame.density_g_cm3;
    let side_parts: &[&str] = if struck == "temple_l" {
        &["temple_l", "lid_l"]
    } else {
        &["temple_r", "lid_r"]
    };
    let frame: f64 = geom
        .parts
        .iter()
        .filter(|b| !side_parts.contains(&b.name.as_str()))
        .map(|b| b.assembly.volume().abs() * dens / 1000.0)
        .sum();
    let pod = if struck == "temple_l" { "left_pod" } else { "right_pod" };
    let comps: f64 = p.components.iter().filter(|c| c.location != pod).map(|c| c.mass_g * c.qty).sum();
    frame + comps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temple_arm_is_aligned_with_the_grid() {
        // The 4 mm temple on a 2 mm grid must be exactly two full voxels thick.
        let p = Project::iris();
        let m = build_model(&p, "temple_r", "temple_spread", 2.0, 20_000, 5.0).unwrap();
        let g = &m.grid;
        let l = &m.layout;
        let j = ((l.pod_rear_y - 30.0 - g.origin[1]) / g.h) as usize;
        let mut cols = std::collections::BTreeMap::new();
        for k in 0..g.nz {
            for i in 0..g.nx {
                let e = g.eid(i, j, k);
                if g.solid[e] {
                    let r = cols.entry(i).or_insert(0.0f32);
                    *r = r.max(g.rho[e]);
                }
            }
        }
        // Two columns across the width, each full (partial voxels only at the
        // tapered top and bottom edges).
        assert_eq!(cols.len(), 2, "arm columns {cols:?}");
        assert!(cols.values().all(|r| *r > 0.99), "{cols:?}");
    }

    #[test]
    fn drop_model_is_an_upper_bound_energy_balance() {
        // ½ m v² = ½ k δ² and F = k δ.
        let d = drop_analysis(1.0, 30.0, 50.0, 2.0, 40.0, "in-plane");
        let v = (2.0 * 9.81f64).sqrt();
        let f = v * (50_000.0f64 * 0.03).sqrt();
        assert!((d.peak_force_n - f).abs() < 1e-9 * f);
        assert!((0.5 * 0.03 * v * v - 0.5 * 50_000.0 * (d.deflection_mm / 1000.0).powi(2)).abs() < 1e-9);
        assert!((d.peak_stress_mpa - 2.0 * f).abs() < 1e-9 * f);
        assert!((d.safety_factor - 40.0 / f).abs() < 1e-12);
    }
}
