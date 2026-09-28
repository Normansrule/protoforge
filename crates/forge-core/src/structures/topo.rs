//! 2D plane-stress finite elements and SIMP topology optimisation.
//!
//! ## Plane stress with bilinear quads
//! Square elements of side 1 (scaled by `elem_mm`), 4 nodes, 2 DOFs each.
//! The element stiffness is the closed form used by the 88-line code (for
//! E = 1, unit thickness):
//! `KE = 1/(1−ν²)/24 · ([A11 A12; A12ᵀ A11] + ν [B11 B12; B12ᵀ B11])`.
//! In 2D the stiffness of a square element does not depend on its size, so
//! physical units only enter through E·t (N/mm) and the element size for
//! stresses. The global system is banded (nodes numbered column by column),
//! so it is solved directly with a banded Cholesky factorisation.
//!
//! ## SIMP
//! Solid Isotropic Material with Penalisation: each element has a density
//! x ∈ [0, 1] and stiffness `E(x) = Emin + x^p (E0 − Emin)`. Minimise the
//! compliance `c = fᵀu = Σ E(x_e) u_eᵀ KE u_e` subject to a volume fraction,
//! with the optimality-criteria (OC) update
//! `x_new = clamp(x · √(−∂c/∂x / (λ ∂V/∂x)), x ± move)`, λ found by
//! bisection so the volume constraint holds. A filter of radius r_min
//! (sensitivity or density filter) removes checkerboards and mesh
//! dependence. The penalty p = 3 pushes intermediate densities towards 0/1
//! because they are "expensive" (stiffness ∝ x³ but volume ∝ x).
//!
//! References: O. Sigmund, "A 99 line topology optimization code written in
//! Matlab", Struct. Multidisc. Optim. 21, 2001; E. Andreassen, A. Clausen,
//! M. Schevenels, B. S. Lazarov, O. Sigmund, "Efficient topology optimization
//! in MATLAB using 88 lines of code", Struct. Multidisc. Optim. 43, 2011;
//! M. P. Bendsøe, O. Sigmund, *Topology Optimization: Theory, Methods and
//! Applications*, Springer, 2003.

use serde::{Deserialize, Serialize};

/// 8 × 8 plane-stress stiffness of a unit square for E = 1 (88-line form).
/// Node order: lower-left, lower-right, upper-right, upper-left; DOFs (x, y).
pub fn ke(nu: f64) -> [[f64; 8]; 8] {
    let k = [
        0.5 - nu / 6.0,
        0.125 + nu / 8.0,
        -0.25 - nu / 12.0,
        -0.125 + 3.0 * nu / 8.0,
        -0.25 + nu / 12.0,
        -0.125 - nu / 8.0,
        nu / 6.0,
        0.125 - 3.0 * nu / 8.0,
    ];
    let f = 1.0 / (1.0 - nu * nu);
    let idx = [
        [0, 1, 2, 3, 4, 5, 6, 7],
        [1, 0, 7, 6, 5, 4, 3, 2],
        [2, 7, 0, 5, 6, 3, 4, 1],
        [3, 6, 5, 0, 7, 2, 1, 4],
        [4, 5, 6, 7, 0, 1, 2, 3],
        [5, 4, 3, 2, 1, 0, 7, 6],
        [6, 3, 4, 1, 2, 7, 0, 5],
        [7, 2, 1, 4, 3, 6, 5, 0],
    ];
    let mut m = [[0.0; 8]; 8];
    for i in 0..8 {
        for j in 0..8 {
            m[i][j] = f * k[idx[i][j]];
        }
    }
    m
}

/// Plane-stress mesh of nelx × nely unit squares. Node (i, j) has i = 0..nelx
/// left→right and j = 0..nely bottom→top; node id = (nely + 1) i + j.
#[derive(Clone, Debug)]
pub struct Mesh2 {
    pub nelx: usize,
    pub nely: usize,
}

impl Mesh2 {
    pub fn nid(&self, i: usize, j: usize) -> usize {
        (self.nely + 1) * i + j
    }
    pub fn ndof(&self) -> usize {
        2 * (self.nelx + 1) * (self.nely + 1)
    }
    /// Element (ex, ey) with ey counted from the bottom; index ey + nely·ex.
    pub fn eid(&self, ex: usize, ey: usize) -> usize {
        ey + self.nely * ex
    }
    pub fn edofs(&self, ex: usize, ey: usize) -> [usize; 8] {
        let ll = self.nid(ex, ey);
        let lr = self.nid(ex + 1, ey);
        let ur = self.nid(ex + 1, ey + 1);
        let ul = self.nid(ex, ey + 1);
        [2 * ll, 2 * ll + 1, 2 * lr, 2 * lr + 1, 2 * ur, 2 * ur + 1, 2 * ul, 2 * ul + 1]
    }
    pub fn bandwidth(&self) -> usize {
        2 * (self.nely + 1) + 3
    }
}

/// Banded symmetric matrix (lower band) with Cholesky solve.
struct Band {
    n: usize,
    bw: usize,
    a: Vec<f64>, // a[i * (bw + 1) + (i - j)] for j ≤ i, i − j ≤ bw
}

impl Band {
    fn new(n: usize, bw: usize) -> Band {
        Band {
            n,
            bw,
            a: vec![0.0; n * (bw + 1)],
        }
    }
    #[inline]
    fn at(&mut self, i: usize, j: usize) -> &mut f64 {
        &mut self.a[i * (self.bw + 1) + (i - j)]
    }
    fn get(&self, i: usize, j: usize) -> f64 {
        if i < j || i - j > self.bw {
            0.0
        } else {
            self.a[i * (self.bw + 1) + (i - j)]
        }
    }
    /// In-place Cholesky; returns false if not positive definite.
    fn factor(&mut self) -> bool {
        let (n, bw) = (self.n, self.bw);
        let w = bw + 1;
        for i in 0..n {
            let j0 = i.saturating_sub(bw);
            for j in j0..=i {
                let k0 = j0.max(j.saturating_sub(bw));
                let mut s = self.a[i * w + (i - j)];
                for k in k0..j {
                    s -= self.a[i * w + (i - k)] * self.a[j * w + (j - k)];
                }
                if i == j {
                    if s <= 0.0 {
                        return false;
                    }
                    self.a[i * w] = s.sqrt();
                } else {
                    self.a[i * w + (i - j)] = s / self.a[j * w];
                }
            }
        }
        true
    }
    fn solve(&self, b: &mut [f64]) {
        let (n, bw) = (self.n, self.bw);
        let w = bw + 1;
        for i in 0..n {
            let mut s = b[i];
            for k in i.saturating_sub(bw)..i {
                s -= self.a[i * w + (i - k)] * b[k];
            }
            b[i] = s / self.a[i * w];
        }
        for i in (0..n).rev() {
            let mut s = b[i];
            for k in i + 1..(i + bw + 1).min(n) {
                s -= self.a[k * w + (k - i)] * b[k];
            }
            b[i] = s / self.a[i * w];
        }
    }
}

/// Solve K(x) u = f for element stiffness factors `e_el` (E·t per element).
pub fn solve_plane(m: &Mesh2, ke: &[[f64; 8]; 8], e_el: &[f64], fixed: &[bool], f: &[f64]) -> Option<Vec<f64>> {
    let n = m.ndof();
    let mut k = Band::new(n, m.bandwidth());
    for ex in 0..m.nelx {
        for ey in 0..m.nely {
            let s = e_el[m.eid(ex, ey)];
            if s == 0.0 {
                continue;
            }
            let d = m.edofs(ex, ey);
            for a in 0..8 {
                for b in 0..8 {
                    let (i, j) = (d[a], d[b]);
                    if i >= j && !fixed[i] && !fixed[j] {
                        *k.at(i, j) += s * ke[a][b];
                    }
                }
            }
        }
    }
    let mut u = f.to_vec();
    for i in 0..n {
        if fixed[i] || k.get(i, i) == 0.0 {
            *k.at(i, i) = 1.0;
            u[i] = 0.0;
        }
    }
    if !k.factor() {
        return None;
    }
    k.solve(&mut u);
    Some(u)
}

/// Von Mises stress at the centre of each element (plane stress), for
/// element size `h` mm and modulus E (MPa) scaled by `e_scale[e]`.
pub fn plane_von_mises(m: &Mesh2, u: &[f64], e_mpa: &[f64], nu: f64, h: f64) -> Vec<f64> {
    // B at the centre of a unit square: dN/dx = ±1/2, dN/dy = ±1/2 (per unit length).
    let dnx = [-0.5, 0.5, 0.5, -0.5];
    let dny = [-0.5, -0.5, 0.5, 0.5];
    let mut out = vec![0.0; m.nelx * m.nely];
    for ex in 0..m.nelx {
        for ey in 0..m.nely {
            let e = m.eid(ex, ey);
            let d = m.edofs(ex, ey);
            let (mut exx, mut eyy, mut gxy) = (0.0, 0.0, 0.0);
            for a in 0..4 {
                let (ux, uy) = (u[d[2 * a]], u[d[2 * a + 1]]);
                exx += dnx[a] * ux;
                eyy += dny[a] * uy;
                gxy += dny[a] * ux + dnx[a] * uy;
            }
            let (exx, eyy, gxy) = (exx / h, eyy / h, gxy / h);
            let c = e_mpa[e] / (1.0 - nu * nu);
            let sx = c * (exx + nu * eyy);
            let sy = c * (eyy + nu * exx);
            let txy = c * 0.5 * (1.0 - nu) * gxy;
            out[e] = (sx * sx - sx * sy + sy * sy + 3.0 * txy * txy).sqrt();
        }
    }
    out
}

// ---------------------------------------------------------------- problem spec

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Support {
    /// Node range (inclusive), node coordinates: i left→right, j bottom→top.
    pub i0: usize,
    pub j0: usize,
    pub i1: usize,
    pub j1: usize,
    pub fix_x: bool,
    pub fix_y: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Load {
    pub i: usize,
    pub j: usize,
    pub fx: f64,
    pub fy: f64,
}

/// Passive region: an axis-aligned element box or a circle (element units).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Passive {
    /// "void" or "solid".
    pub kind: String,
    /// "rect" (x0, y0, x1, y1) or "circle" (cx, cy, r) in element units.
    pub shape: String,
    pub a: [f64; 4],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TopoIn {
    pub preset: String,
    pub nelx: usize,
    pub nely: usize,
    pub volfrac: f64,
    pub penal: f64,
    pub rmin: f64,
    /// "sensitivity" or "density".
    pub filter: String,
    pub max_iter: usize,
    pub tol_change: f64,
    pub move_limit: f64,
    /// Store every Nth iteration's density field.
    pub every: usize,
    pub nu: f64,
    /// Physical element size for the outline and stress export, mm.
    pub elem_mm: f64,
    /// Custom problem (used when `preset` = "custom").
    pub supports: Vec<Support>,
    pub loads: Vec<Load>,
    pub passive: Vec<Passive>,
}

impl Default for TopoIn {
    fn default() -> Self {
        TopoIn {
            preset: "mbb".into(),
            nelx: 0,
            nely: 0,
            volfrac: 0.0,
            penal: 3.0,
            rmin: 0.0,
            filter: "sensitivity".into(),
            max_iter: 100,
            tol_change: 0.01,
            move_limit: 0.2,
            every: 2,
            nu: 0.3,
            elem_mm: 0.0,
            supports: vec![],
            loads: vec![],
            passive: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PresetInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub nelx: usize,
    pub nely: usize,
    pub volfrac: f64,
    pub rmin: f64,
    pub elem_mm: f64,
}

pub fn presets() -> Vec<PresetInfo> {
    vec![
        PresetInfo {
            id: "mbb",
            label: "MBB beam (half, symmetric)",
            description: "The classic benchmark from Sigmund's 99-line paper: a simply supported beam with a central load, modelled as the right half with a symmetry edge on the left.",
            nelx: 60,
            nely: 20,
            volfrac: 0.5,
            rmin: 1.5,
            elem_mm: 1.0,
        },
        PresetInfo {
            id: "cantilever",
            label: "Cantilever",
            description: "Left edge clamped, downward load at the middle of the right edge — the answer is a Michell-like truss.",
            nelx: 60,
            nely: 30,
            volfrac: 0.4,
            rmin: 1.5,
            elem_mm: 1.0,
        },
        PresetInfo {
            id: "bracket",
            label: "L-bracket",
            description: "An L-shaped domain (upper-right quadrant void) clamped along its top edge, loaded downward at the tip of the lower arm.",
            nelx: 40,
            nely: 40,
            volfrac: 0.4,
            rmin: 1.5,
            elem_mm: 1.0,
        },
        PresetInfo {
            id: "iris_hinge",
            label: "Iris hinge bracket (endpiece, top view)",
            description: "The endpiece joining an Iris rim (clamped, left) to the hinge knuckle (pin hole with a solid ring, right). Opening the temple pulls the knuckle rearward and outward. 0.25 mm elements: a 12 × 6 mm design space.",
            nelx: 48,
            nely: 24,
            volfrac: 0.45,
            rmin: 1.5,
            elem_mm: 0.25,
        },
        PresetInfo {
            id: "temple_arm",
            label: "Iris temple arm (side view)",
            description: "The temple arm behind the pod, clamped at the pod (left) and loaded downward at the ear (right): what would the stiffest 50 % arm look like?",
            nelx: 90,
            nely: 15,
            volfrac: 0.5,
            rmin: 1.5,
            elem_mm: 0.8,
        },
    ]
}

/// Fill in the preset's geometry, supports, loads and passive regions.
fn setup(i: &mut TopoIn) -> Result<(), String> {
    let p = presets().into_iter().find(|p| p.id == i.preset);
    if let Some(p) = &p {
        if i.nelx == 0 {
            i.nelx = p.nelx;
        }
        if i.nely == 0 {
            i.nely = p.nely;
        }
        if i.volfrac <= 0.0 {
            i.volfrac = p.volfrac;
        }
        if i.rmin <= 0.0 {
            i.rmin = p.rmin;
        }
        if i.elem_mm <= 0.0 {
            i.elem_mm = p.elem_mm;
        }
    } else if i.preset != "custom" {
        return Err(format!("unknown preset `{}`", i.preset));
    }
    i.nelx = i.nelx.clamp(4, 160);
    i.nely = i.nely.clamp(4, 80);
    i.volfrac = i.volfrac.clamp(0.05, 0.95);
    i.rmin = if i.rmin > 0.0 { i.rmin.clamp(1.0, 8.0) } else { 1.5 };
    i.penal = i.penal.clamp(1.0, 5.0);
    if i.elem_mm <= 0.0 {
        i.elem_mm = 1.0;
    }
    let (nx, ny) = (i.nelx, i.nely);
    let full = |i0, j0, i1, j1, fx, fy| Support {
        i0,
        j0,
        i1,
        j1,
        fix_x: fx,
        fix_y: fy,
    };
    match i.preset.as_str() {
        "mbb" => {
            i.supports = vec![full(0, 0, 0, ny, true, false), full(nx, 0, nx, 0, false, true)];
            i.loads = vec![Load {
                i: 0,
                j: ny,
                fx: 0.0,
                fy: -1.0,
            }];
            i.passive = vec![];
        }
        "cantilever" => {
            i.supports = vec![full(0, 0, 0, ny, true, true)];
            i.loads = vec![Load {
                i: nx,
                j: ny / 2,
                fx: 0.0,
                fy: -1.0,
            }];
            i.passive = vec![];
        }
        "bracket" => {
            let (cx, cy) = (nx * 2 / 5, ny * 2 / 5);
            i.supports = vec![full(0, ny, cx, ny, true, true)];
            i.loads = vec![Load {
                i: nx,
                j: cy / 2,
                fx: 0.0,
                fy: -1.0,
            }];
            i.passive = vec![Passive {
                kind: "void".into(),
                shape: "rect".into(),
                a: [cx as f64, cy as f64, nx as f64, ny as f64],
            }];
        }
        "iris_hinge" => {
            let (cx, cy, r) = (nx as f64 - 7.0, ny as f64 / 2.0, 2.2);
            i.supports = vec![full(0, 0, 0, ny, true, true)];
            // Load spread over the right edge of the hole ring (rearward + outward pull).
            let ic = (cx + r + 1.0).round() as usize;
            let jc = cy.round() as usize;
            i.loads = vec![
                Load {
                    i: ic,
                    j: jc,
                    fx: 0.35,
                    fy: -0.5,
                },
                Load {
                    i: ic - 2,
                    j: jc - 2,
                    fx: 0.35,
                    fy: -0.5,
                },
            ];
            i.passive = vec![
                Passive {
                    kind: "solid".into(),
                    shape: "circle".into(),
                    a: [cx, cy, r + 2.0, 0.0],
                },
                Passive {
                    kind: "void".into(),
                    shape: "circle".into(),
                    a: [cx, cy, r, 0.0],
                },
            ];
        }
        "temple_arm" => {
            i.supports = vec![full(0, 0, 0, ny, true, true)];
            i.loads = vec![Load {
                i: nx,
                j: ny / 2,
                fx: 0.0,
                fy: -1.0,
            }];
            i.passive = vec![];
        }
        _ => {
            if i.supports.is_empty() || i.loads.is_empty() {
                return Err("a custom problem needs supports and loads".into());
            }
        }
    }
    Ok(())
}

/// Passive flag per element: 0 free, −1 void, +1 solid (later entries win).
fn passive_map(i: &TopoIn) -> Vec<i8> {
    let m = Mesh2 {
        nelx: i.nelx,
        nely: i.nely,
    };
    let mut out = vec![0i8; i.nelx * i.nely];
    for ex in 0..i.nelx {
        for ey in 0..i.nely {
            let (cx, cy) = (ex as f64 + 0.5, ey as f64 + 0.5);
            for p in &i.passive {
                let inside = match p.shape.as_str() {
                    "circle" => (cx - p.a[0]).powi(2) + (cy - p.a[1]).powi(2) <= p.a[2] * p.a[2],
                    _ => cx >= p.a[0] && cx <= p.a[2] && cy >= p.a[1] && cy <= p.a[3],
                };
                if inside {
                    out[m.eid(ex, ey)] = if p.kind == "solid" { 1 } else { -1 };
                }
            }
        }
    }
    out
}

fn bcs(i: &TopoIn, m: &Mesh2) -> (Vec<bool>, Vec<f64>) {
    let mut fixed = vec![false; m.ndof()];
    for s in &i.supports {
        for a in s.i0.min(m.nelx)..=s.i1.min(m.nelx) {
            for b in s.j0.min(m.nely)..=s.j1.min(m.nely) {
                let n = m.nid(a, b);
                if s.fix_x {
                    fixed[2 * n] = true;
                }
                if s.fix_y {
                    fixed[2 * n + 1] = true;
                }
            }
        }
    }
    let mut f = vec![0.0; m.ndof()];
    for l in &i.loads {
        let n = m.nid(l.i.min(m.nelx), l.j.min(m.nely));
        f[2 * n] += l.fx;
        f[2 * n + 1] += l.fy;
    }
    (fixed, f)
}

#[derive(Clone, Debug, Serialize)]
pub struct Frame {
    pub iter: usize,
    /// Densities quantised to 0..255, base64, element index ey + nely·ex.
    pub rho_b64: String,
    pub compliance: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TopoOut {
    pub preset: String,
    pub nelx: usize,
    pub nely: usize,
    pub volfrac: f64,
    pub penal: f64,
    pub rmin: f64,
    pub filter: String,
    pub elem_mm: f64,
    pub iterations: usize,
    pub converged: bool,
    pub compliance: Vec<f64>,
    pub volume: Vec<f64>,
    pub change: Vec<f64>,
    pub frames: Vec<Frame>,
    /// Final physical densities (0..1).
    pub final_rho: Vec<f64>,
    /// Von Mises stress (MPa per unit load, E = 1 GPa, t = 1 mm) of the final design.
    pub final_vm: Vec<f64>,
    /// Outline loops at ρ = 0.5 in mm (y up), outer loops counter-clockwise.
    pub outline: Vec<Vec<[f64; 2]>>,
    pub svg: String,
    pub openscad: String,
    pub supports: Vec<Support>,
    pub loads: Vec<Load>,
    pub passive: Vec<i8>,
}

fn b64(rho: &[f64]) -> String {
    let bytes: Vec<u8> = rho.iter().map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8).collect();
    crate::cad::export::base64(&bytes)
}

/// Filter neighbourhoods: for each element the list of (neighbour, weight).
fn filter_weights(nelx: usize, nely: usize, rmin: f64) -> (Vec<Vec<(u32, f64)>>, Vec<f64>) {
    let r = rmin.ceil() as i64 - 1;
    let mut hs = vec![0.0; nelx * nely];
    let mut hv = vec![Vec::new(); nelx * nely];
    for ex in 0..nelx as i64 {
        for ey in 0..nely as i64 {
            let e = (ey + nely as i64 * ex) as usize;
            let mut s = 0.0;
            for kx in (ex - r).max(0)..=(ex + r).min(nelx as i64 - 1) {
                for ky in (ey - r).max(0)..=(ey + r).min(nely as i64 - 1) {
                    let w = rmin - (((ex - kx).pow(2) + (ey - ky).pow(2)) as f64).sqrt();
                    if w > 0.0 {
                        hv[e].push(((ky + nely as i64 * kx) as u32, w));
                        s += w;
                    }
                }
            }
            hs[e] = s;
        }
    }
    (hv, hs)
}

/// Run SIMP topology optimisation.
pub fn optimize(mut i: TopoIn) -> Result<TopoOut, String> {
    setup(&mut i)?;
    let m = Mesh2 {
        nelx: i.nelx,
        nely: i.nely,
    };
    let ne = i.nelx * i.nely;
    let kk = ke(i.nu);
    let (fixed, f) = bcs(&i, &m);
    let pas = passive_map(&i);
    let (h, hs) = filter_weights(i.nelx, i.nely, i.rmin);
    let density = i.filter == "density";
    let (e0, emin, p) = (1.0, 1e-9, i.penal);
    let mut x: Vec<f64> = pas
        .iter()
        .map(|&q| {
            if q < 0 {
                0.0
            } else if q > 0 {
                1.0
            } else {
                i.volfrac
            }
        })
        .collect();
    let mut xphys = x.clone();
    let mut out = TopoOut {
        preset: i.preset.clone(),
        nelx: i.nelx,
        nely: i.nely,
        volfrac: i.volfrac,
        penal: p,
        rmin: i.rmin,
        filter: if density { "density".into() } else { "sensitivity".into() },
        elem_mm: i.elem_mm,
        ..Default::default()
    };
    let every = i.every.max(1);
    let max_iter = i.max_iter.clamp(1, 400);
    let mut u = vec![0.0; m.ndof()];
    for it in 1..=max_iter {
        let e_el: Vec<f64> = xphys.iter().map(|&v| emin + v.powf(p) * (e0 - emin)).collect();
        u = solve_plane(&m, &kk, &e_el, &fixed, &f).ok_or("stiffness matrix is singular — check the supports")?;
        // Compliance and sensitivities.
        let mut c = 0.0;
        let mut dc = vec![0.0; ne];
        for ex in 0..m.nelx {
            for ey in 0..m.nely {
                let e = m.eid(ex, ey);
                let d = m.edofs(ex, ey);
                let mut ce = 0.0;
                for a in 0..8 {
                    let mut s = 0.0;
                    for b in 0..8 {
                        s += kk[a][b] * u[d[b]];
                    }
                    ce += u[d[a]] * s;
                }
                c += e_el[e] * ce;
                dc[e] = -p * xphys[e].powf(p - 1.0) * (e0 - emin) * ce;
            }
        }
        let mut dv = vec![1.0; ne];
        // Filtering.
        if density {
            dc = (0..ne)
                .map(|e| h[e].iter().map(|&(k, w)| w * dc[k as usize] / hs[k as usize]).sum())
                .collect();
            dv = (0..ne).map(|e| h[e].iter().map(|&(k, w)| w / hs[k as usize]).sum()).collect();
        } else {
            let xd: Vec<f64> = (0..ne).map(|e| x[e] * dc[e]).collect();
            dc = (0..ne)
                .map(|e| h[e].iter().map(|&(k, w)| w * xd[k as usize]).sum::<f64>() / hs[e] / x[e].max(1e-3))
                .collect();
        }
        // Optimality criteria with bisection on the Lagrange multiplier.
        let (mut l1, mut l2) = (0.0, 1e9);
        let mv = i.move_limit.clamp(0.01, 1.0);
        let free_n = pas.iter().filter(|q| **q == 0).count().max(1) as f64;
        let solid_n = pas.iter().filter(|q| **q > 0).count() as f64;
        let target = i.volfrac * free_n + solid_n;
        let mut xnew = x.clone();
        while (l2 - l1) / (l1 + l2) > 1e-4 {
            let lmid = 0.5 * (l2 + l1);
            for e in 0..ne {
                xnew[e] = match pas[e] {
                    -1 => 0.0,
                    1 => 1.0,
                    _ => {
                        let be = (-dc[e] / dv[e] / lmid).max(0.0).sqrt();
                        (x[e] * be).clamp((x[e] - mv).max(0.0), (x[e] + mv).min(1.0))
                    }
                };
            }
            let phys: Vec<f64> = if density {
                (0..ne)
                    .map(|e| match pas[e] {
                        -1 => 0.0,
                        1 => 1.0,
                        _ => h[e].iter().map(|&(k, w)| w * xnew[k as usize]).sum::<f64>() / hs[e],
                    })
                    .collect()
            } else {
                xnew.clone()
            };
            if phys.iter().sum::<f64>() > target {
                l1 = lmid;
            } else {
                l2 = lmid;
            }
            if l2 < 1e-40 {
                break;
            }
            xphys = phys;
        }
        let change = x.iter().zip(&xnew).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
        x = xnew;
        let vol = xphys.iter().sum::<f64>() / ne as f64;
        out.compliance.push(c);
        out.volume.push(vol);
        out.change.push(change);
        out.iterations = it;
        if it == 1 || it % every == 0 {
            out.frames.push(Frame {
                iter: it,
                rho_b64: b64(&xphys),
                compliance: c,
            });
        }
        if change < i.tol_change && it > 5 {
            out.converged = true;
            break;
        }
    }
    if out.frames.last().map(|f| f.iter) != Some(out.iterations) {
        out.frames.push(Frame {
            iter: out.iterations,
            rho_b64: b64(&xphys),
            compliance: *out.compliance.last().unwrap_or(&0.0),
        });
    }
    // Stress in the final design (E = 1 GPa, unit load, t = 1 mm).
    let e_mpa: Vec<f64> = xphys.iter().map(|&v| 1000.0 * (emin + v.powf(p) * (e0 - emin))).collect();
    // u was computed for E0 = 1 (dimensionless); scale to E = 1000 MPa.
    let u_mm: Vec<f64> = u.iter().map(|v| v / 1000.0).collect();
    out.final_vm = plane_von_mises(&m, &u_mm, &e_mpa, i.nu, i.elem_mm)
        .iter()
        .zip(&xphys)
        .map(|(s, &r)| if r > 0.3 { *s } else { 0.0 })
        .collect();
    out.outline = outline(&m, &xphys, 0.5, i.elem_mm);
    out.svg = svg(&out.outline, i.nelx as f64 * i.elem_mm, i.nely as f64 * i.elem_mm);
    out.openscad = openscad(&out.outline, &i.preset);
    out.final_rho = xphys;
    out.supports = i.supports.clone();
    out.loads = i.loads.clone();
    out.passive = pas;
    Ok(out)
}

/// Marching squares on node-averaged densities (with a zero border so all
/// contours close). Returns loops in mm with y up.
pub fn outline(m: &Mesh2, rho: &[f64], level: f64, scale: f64) -> Vec<Vec<[f64; 2]>> {
    let (nx, ny) = (m.nelx, m.nely);
    // Node values on a padded (nx + 3) × (ny + 3) lattice; node (a, b) sits at
    // (a − 1, b − 1) element units.
    let (w, hgt) = (nx + 3, ny + 3);
    let mut v = vec![0.0; w * hgt];
    for a in 1..w - 1 {
        for b in 1..hgt - 1 {
            let (i, j) = (a - 1, b - 1);
            let mut s = 0.0;
            let mut c = 0.0;
            for (ex, ey) in [
                (i as i64 - 1, j as i64 - 1),
                (i as i64, j as i64 - 1),
                (i as i64 - 1, j as i64),
                (i as i64, j as i64),
            ] {
                if ex >= 0 && ey >= 0 && (ex as usize) < nx && (ey as usize) < ny {
                    s += rho[m.eid(ex as usize, ey as usize)];
                }
                c += 1.0;
            }
            v[a + w * b] = s / c;
        }
    }
    // Segments per cell, oriented so the solid (> level) is on the left.
    let pt = |a: usize, b: usize| -> [f64; 2] { [a as f64 - 1.0, b as f64 - 1.0] };
    let lerp = |p: [f64; 2], q: [f64; 2], vp: f64, vq: f64| -> [f64; 2] {
        let t = if (vq - vp).abs() < 1e-12 { 0.5 } else { (level - vp) / (vq - vp) };
        [p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]
    };
    let mut segs: Vec<([f64; 2], [f64; 2])> = Vec::new();
    for a in 0..w - 1 {
        for b in 0..hgt - 1 {
            let c = [pt(a, b), pt(a + 1, b), pt(a + 1, b + 1), pt(a, b + 1)];
            let val = [v[a + w * b], v[a + 1 + w * b], v[a + 1 + w * (b + 1)], v[a + w * (b + 1)]];
            let inside: Vec<bool> = val.iter().map(|x| *x > level).collect();
            // Edge crossings e_k between corner k and k+1.
            let cross = |k: usize| lerp(c[k], c[(k + 1) % 4], val[k], val[(k + 1) % 4]);
            // Walk edges: for every edge going from inside to outside (ccw),
            // the boundary leaves the cell; pair entries with exits.
            let mut exits = Vec::new();
            let mut entries = Vec::new();
            for k in 0..4 {
                let (i0, i1) = (inside[k], inside[(k + 1) % 4]);
                if i0 && !i1 {
                    exits.push(k);
                }
                if !i0 && i1 {
                    entries.push(k);
                }
            }
            if exits.is_empty() {
                continue;
            }
            if exits.len() == 1 {
                // Solid on the left going from the entry edge crossing to the exit edge crossing
                // (counter-clockwise around the solid).
                segs.push((cross(exits[0]), cross(entries[0])));
            } else {
                // Saddle (codes 5 and 10): resolve with the centre value.
                let centre = val.iter().sum::<f64>() / 4.0;
                if centre > level {
                    for &ex in &exits {
                        segs.push((cross(ex), cross((ex + 1) % 4)));
                    }
                } else {
                    for &ex in &exits {
                        segs.push((cross(ex), cross((ex + 3) % 4)));
                    }
                }
            }
        }
    }
    chain(segs, scale)
}

/// Join segments into closed loops by matching endpoints.
fn chain(segs: Vec<([f64; 2], [f64; 2])>, scale: f64) -> Vec<Vec<[f64; 2]>> {
    use std::collections::HashMap;
    let key = |p: [f64; 2]| ((p[0] * 1e6).round() as i64, (p[1] * 1e6).round() as i64);
    let mut from: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (k, s) in segs.iter().enumerate() {
        from.entry(key(s.0)).or_default().push(k);
    }
    let mut used = vec![false; segs.len()];
    let mut loops = Vec::new();
    for start in 0..segs.len() {
        if used[start] {
            continue;
        }
        let mut lp = vec![segs[start].0];
        used[start] = true;
        let mut cur = segs[start].1;
        let mut guard = 0;
        while guard < segs.len() + 2 {
            guard += 1;
            if key(cur) == key(lp[0]) {
                break;
            }
            lp.push(cur);
            let next = from.get(&key(cur)).and_then(|v| v.iter().copied().find(|&k| !used[k]));
            match next {
                Some(k) => {
                    used[k] = true;
                    cur = segs[k].1;
                }
                None => break,
            }
        }
        if lp.len() >= 3 {
            loops.push(lp.into_iter().map(|p| [p[0] * scale, p[1] * scale]).collect());
        }
    }
    loops
}

fn svg(loops: &[Vec<[f64; 2]>], w: f64, h: f64) -> String {
    let mut d = String::new();
    for lp in loops {
        for (k, p) in lp.iter().enumerate() {
            d.push_str(&format!("{}{:.3},{:.3} ", if k == 0 { "M" } else { "L" }, p[0], h - p[1]));
        }
        d.push_str("Z ");
    }
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.2}mm\" height=\"{h:.2}mm\" viewBox=\"0 0 {w:.3} {h:.3}\">\n  <!-- Protoforge Structures Lab: SIMP topology optimisation outline (rho = 0.5), units mm -->\n  <path d=\"{}\" fill=\"#3987e5\" fill-rule=\"evenodd\" stroke=\"#1d2735\" stroke-width=\"0.05\"/>\n</svg>\n",
        d.trim()
    )
}

fn openscad(loops: &[Vec<[f64; 2]>], name: &str) -> String {
    let mut pts = Vec::new();
    let mut paths = Vec::new();
    for lp in loops {
        let start = pts.len();
        pts.extend(lp.iter().map(|p| format!("[{:.3},{:.3}]", p[0], p[1])));
        paths.push(format!(
            "[{}]",
            (start..pts.len()).map(|k| k.to_string()).collect::<Vec<_>>().join(",")
        ));
    }
    format!(
        "// Protoforge Structures Lab — topology-optimised outline ({name}).\n// Extrude to the part thickness; smooth the edges in your CAD tool before use.\nthickness = 4; // mm\nlinear_extrude(height = thickness)\n  polygon(points = [{}],\n          paths = [{}]);\n",
        pts.join(","),
        paths.join(",")
    )
}

// ---------------------------------------------------------------- standalone solver

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct PlaneIn {
    pub nelx: usize,
    pub nely: usize,
    pub elem_mm: f64,
    pub thickness_mm: f64,
    pub e_mpa: f64,
    pub nu: f64,
    /// Solid mask per element (index ey + nely·ex); empty = all solid.
    pub solid: Vec<u8>,
    pub supports: Vec<Support>,
    /// Loads in newtons.
    pub loads: Vec<Load>,
}

impl Default for PlaneIn {
    fn default() -> Self {
        PlaneIn {
            nelx: 40,
            nely: 10,
            elem_mm: 1.0,
            thickness_mm: 1.0,
            e_mpa: 2000.0,
            nu: 0.3,
            solid: vec![],
            supports: vec![Support {
                i0: 0,
                j0: 0,
                i1: 0,
                j1: 10,
                fix_x: true,
                fix_y: true,
            }],
            loads: vec![Load {
                i: 40,
                j: 5,
                fx: 0.0,
                fy: -10.0,
            }],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PlaneOut {
    pub nelx: usize,
    pub nely: usize,
    pub ux: Vec<f64>,
    pub uy: Vec<f64>,
    pub von_mises: Vec<f64>,
    pub max_vm_mpa: f64,
    pub max_disp_mm: f64,
}

pub fn plane_stress(i: PlaneIn) -> Result<PlaneOut, String> {
    let (nx, ny) = (i.nelx.clamp(1, 200), i.nely.clamp(1, 100));
    let m = Mesh2 { nelx: nx, nely: ny };
    let ne = nx * ny;
    let solid: Vec<bool> = if i.solid.len() == ne {
        i.solid.iter().map(|v| *v != 0).collect()
    } else {
        vec![true; ne]
    };
    let et = i.e_mpa * i.thickness_mm;
    let e_el: Vec<f64> = solid.iter().map(|s| if *s { et } else { et * 1e-9 }).collect();
    let spec = TopoIn {
        nelx: nx,
        nely: ny,
        supports: i.supports.clone(),
        loads: i.loads.clone(),
        ..Default::default()
    };
    let (fixed, f) = bcs(&spec, &m);
    let u = solve_plane(&m, &ke(i.nu), &e_el, &fixed, &f).ok_or("singular stiffness — add supports")?;
    // Unit-square stiffness: u is already in mm because KE is size-independent in 2D.
    let e_mpa: Vec<f64> = solid.iter().map(|s| if *s { i.e_mpa } else { 0.0 }).collect();
    let vm = plane_von_mises(&m, &u, &e_mpa, i.nu, i.elem_mm);
    let ux: Vec<f64> = (0..u.len() / 2).map(|n| u[2 * n]).collect();
    let uy: Vec<f64> = (0..u.len() / 2).map(|n| u[2 * n + 1]).collect();
    let max_disp = ux.iter().zip(&uy).map(|(a, b)| (a * a + b * b).sqrt()).fold(0.0, f64::max);
    Ok(PlaneOut {
        nelx: nx,
        nely: ny,
        max_vm_mpa: vm.iter().cloned().fold(0.0, f64::max),
        ux,
        uy,
        von_mises: vm,
        max_disp_mm: max_disp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ke_is_symmetric_and_singular_for_rigid_motion() {
        let k = ke(0.3);
        for i in 0..8 {
            for j in 0..8 {
                assert!((k[i][j] - k[j][i]).abs() < 1e-14);
            }
            // translation in x
            let fx: f64 = (0..4).map(|a| k[i][2 * a]).sum();
            assert!(fx.abs() < 1e-12);
        }
    }

    #[test]
    fn plane_cantilever_matches_beam_theory() {
        // 80 × 8 elements of 0.5 mm: L = 40, h = 4, t = 1; P = 1 N at the tip.
        let (nx, ny, s) = (80, 8, 0.5);
        let p = PlaneIn {
            nelx: nx,
            nely: ny,
            elem_mm: s,
            thickness_mm: 1.0,
            e_mpa: 2000.0,
            nu: 0.3,
            solid: vec![],
            supports: vec![Support {
                i0: 0,
                j0: 0,
                i1: 0,
                j1: ny,
                fix_x: true,
                fix_y: true,
            }],
            loads: (0..=ny)
                .map(|j| Load {
                    i: nx,
                    j,
                    fx: 0.0,
                    fy: -1.0 / (ny + 1) as f64,
                })
                .collect(),
        };
        let o = plane_stress(p).unwrap();
        let tip = -o.uy[(ny + 1) * nx + ny / 2];
        let (l, h) = (40.0, 4.0);
        let i = h * h * h / 12.0;
        let exact = l * l * l / (3.0 * 2000.0 * i);
        // Bilinear quads lock a little in bending; 8 elements deep keeps it ~5 %.
        let rel = (tip - exact) / exact;
        assert!(rel < 0.02 && rel > -0.10, "tip {tip} vs {exact}");
    }

    #[test]
    fn mbb_converges_to_a_truss() {
        let o = optimize(TopoIn {
            preset: "mbb".into(),
            nelx: 60,
            nely: 20,
            max_iter: 120,
            ..Default::default()
        })
        .unwrap();
        // Volume constraint satisfied and compliance decreasing overall.
        let v = *o.volume.last().unwrap();
        assert!((v - 0.5).abs() < 0.01, "volume {v}");
        let c0 = o.compliance[0];
        let c1 = *o.compliance.last().unwrap();
        assert!(c1 < 0.7 * c0, "compliance {c0} -> {c1}");
        // The 88-line setting (60 × 20, r_min = 1.5, sensitivity filter)
        // converges in about 94 iterations to a mostly black-and-white truss;
        // the filter leaves a one-element grey rim along every member.
        assert!(o.converged && o.iterations < 110, "iterations {}", o.iterations);
        let grey = o.final_rho.iter().filter(|r| **r > 0.1 && **r < 0.9).count() as f64 / o.final_rho.len() as f64;
        assert!(grey < 0.3, "grey fraction {grey}");
        assert!(!o.outline.is_empty() && o.svg.contains("<path"));
        assert!(o.frames.len() >= 2);
    }

    #[test]
    fn presets_run() {
        for p in presets() {
            let o = optimize(TopoIn {
                preset: p.id.into(),
                max_iter: 3,
                ..Default::default()
            })
            .unwrap();
            assert!(o.compliance.iter().all(|c| c.is_finite() && *c > 0.0), "{}", p.id);
        }
    }
}
