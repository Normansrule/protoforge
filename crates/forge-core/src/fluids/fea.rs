//! Voxel finite element method (FEM) for heat conduction.
//!
//! ## Problem
//! Steady conduction with volumetric sources and a convective / radiative
//! boundary, `−∇·(k ∇T) = q` in Ω, `−k ∂T/∂n = h (T − T∞)` on the exposed
//! surface. Working in the excess temperature θ = T − T∞ the boundary
//! condition is homogeneous. The Galerkin weak form with trilinear 8-node
//! bricks (the same regular voxel grid the Structures Lab uses) gives
//!
//! ```text
//!   (K + H) θ = Q,      K = Σ_e Σ_d k_d,e  L · K̂_d,     H = Σ_faces h A/4 (per face node)
//! ```
//!
//! with the unit-cube matrices `K̂_d[a][b] = ∫ ∂N_a/∂ξ_d ∂N_b/∂ξ_d dΩ̂`. For
//! `N_a = φ_{ia}(ξ)φ_{ja}(η)φ_{ka}(ζ)`, `φ₀ = 1 − t`, `φ₁ = t`, the integral
//! factorises: along axis d it is ±1 (same / different node), along the other
//! two axes it is the 1-D mass factor 1/3 (same) or 1/6 (different). The
//! conductivity may differ per axis (a graphite sheet conducts ~150× better
//! along the sheet than across it), so every element carries `[k_x, k_y, k_z]`.
//! Voxels only partly filled with a material get rule-of-mixtures values
//! (parallel along a sheet, series across it) — see `pod.rs`.
//!
//! Face terms use a lumped (row-sum) boundary matrix: each of the 4 face
//! nodes gets `h A / 4`, which keeps the system an M-matrix and is exact for
//! the uniform faces of the analytic tests. The capacity matrix is lumped the
//! same way (`ρc V / 8` per node).
//!
//! ## Solver
//! `K` is assembled once into compressed sparse rows (CSR, ≤ 27 entries per
//! row); the boundary and capacity terms are carried as a separate diagonal
//! so the convection coefficients can be updated (natural convection and
//! radiation depend on the surface temperature — a Picard iteration) and the
//! transient matrix `K + H + C/Δt` formed without re-assembly. The system is
//! symmetric positive definite and solved by the preconditioned conjugate
//! gradient (PCG) method with either Jacobi (diagonal) or symmetric
//! successive over-relaxation (SSOR) preconditioning.
//!
//! ## Transient
//! Implicit (backward) Euler, unconditionally stable and first-order:
//! `(C/Δt + K + H) θⁿ⁺¹ = C/Δt θⁿ + Qⁿ⁺¹` — one PCG solve per step, warm-started
//! from θⁿ, so each step costs a handful of iterations.
//!
//! References: S. V. Patankar, *Numerical Heat Transfer and Fluid Flow*
//! (Hemisphere, 1980); T. J. R. Hughes, *The Finite Element Method* (Dover,
//! 2000), ch. 7 (transient algorithms); Y. Saad, *Iterative Methods for
//! Sparse Linear Systems*, 2nd ed. (SIAM, 2003), §10.2 (SSOR preconditioning);
//! J. R. Shewchuk, "An Introduction to the Conjugate Gradient Method Without
//! the Agonizing Pain" (CMU, 1994).

pub const NONE: u32 = u32::MAX;

/// Node offsets of a brick: a = i + 2 j + 4 k.
pub const OFFS: [[usize; 3]; 8] = [
    [0, 0, 0],
    [1, 0, 0],
    [0, 1, 0],
    [1, 1, 0],
    [0, 0, 1],
    [1, 0, 1],
    [0, 1, 1],
    [1, 1, 1],
];

/// Face directions: −x, +x, −y, +y, −z, +z.
pub const DIRS: [[i32; 3]; 6] = [[-1, 0, 0], [1, 0, 0], [0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1]];

/// Unit-cube conduction matrices K̂_d (d = x, y, z).
pub fn unit_k() -> [[[f64; 8]; 8]; 3] {
    let mut out = [[[0.0; 8]; 8]; 3];
    for (d, kd) in out.iter_mut().enumerate() {
        for a in 0..8 {
            for b in 0..8 {
                let mut v = 1.0;
                for ax in 0..3 {
                    let same = OFFS[a][ax] == OFFS[b][ax];
                    v *= if ax == d {
                        if same {
                            1.0
                        } else {
                            -1.0
                        }
                    } else if same {
                        1.0 / 3.0
                    } else {
                        1.0 / 6.0
                    };
                }
                kd[a][b] = v;
            }
        }
    }
    out
}

/// Local node indices (into OFFS) of face `dir`.
pub fn face_nodes(dir: usize) -> [usize; 4] {
    let ax = dir / 2;
    let side = dir % 2;
    let mut out = [0; 4];
    let mut n = 0;
    for (a, o) in OFFS.iter().enumerate() {
        if o[ax] == side {
            out[n] = a;
            n += 1;
        }
    }
    out
}

/// One active voxel.
#[derive(Clone, Debug)]
pub struct Elem {
    /// Grid element index i + nx (j + ny k).
    pub e: u32,
    /// Conductivity along x, y, z, W/(m K).
    pub k: [f64; 3],
    /// Volumetric heat capacity, J/(m³ K).
    pub rho_c: f64,
    /// Material tag (caller-defined).
    pub mat: u8,
    /// Part / body tag (caller-defined).
    pub part: u8,
}

/// An exposed (convective) element face.
#[derive(Clone, Copy, Debug)]
pub struct Face {
    /// Active element index.
    pub elem: u32,
    pub dir: u8,
    /// Coefficient group (index into the `h` table).
    pub group: u16,
}

/// Voxel conduction model on a regular grid (h in mm).
#[derive(Clone, Debug)]
pub struct Model {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    pub h_mm: f64,
    pub origin: [f64; 3],
    pub elems: Vec<Elem>,
    /// Grid element → active index (NONE if inactive).
    pub elem_of: Vec<u32>,
    /// Grid node → dof (NONE if not touched by an active element).
    pub dof_of: Vec<u32>,
    /// Dof → grid node.
    pub node_of: Vec<u32>,
    pub faces: Vec<Face>,
    /// Faces on the grid boundary (model cut): adiabatic, kept for drawing.
    pub cut_faces: Vec<Face>,
}

impl Model {
    #[inline]
    pub fn eid(&self, i: usize, j: usize, k: usize) -> usize {
        i + self.nx * (j + self.ny * k)
    }
    #[inline]
    pub fn ecoord(&self, e: usize) -> [usize; 3] {
        [e % self.nx, (e / self.nx) % self.ny, e / (self.nx * self.ny)]
    }
    #[inline]
    pub fn nid(&self, i: usize, j: usize, k: usize) -> usize {
        i + (self.nx + 1) * (j + (self.ny + 1) * k)
    }
    pub fn ncoord(&self, n: usize) -> [usize; 3] {
        let nx1 = self.nx + 1;
        let ny1 = self.ny + 1;
        [n % nx1, (n / nx1) % ny1, n / (nx1 * ny1)]
    }
    pub fn node_pos(&self, n: usize) -> [f64; 3] {
        let c = self.ncoord(n);
        std::array::from_fn(|d| self.origin[d] + self.h_mm * c[d] as f64)
    }
    pub fn elem_center(&self, e: usize) -> [f64; 3] {
        let c = self.ecoord(e);
        std::array::from_fn(|d| self.origin[d] + self.h_mm * (c[d] as f64 + 0.5))
    }
    pub fn ndof(&self) -> usize {
        self.node_of.len()
    }
    /// Grid node ids of element `e` (grid index), OFFS order.
    pub fn elem_nodes(&self, e: usize) -> [usize; 8] {
        let [i, j, k] = self.ecoord(e);
        std::array::from_fn(|a| self.nid(i + OFFS[a][0], j + OFFS[a][1], k + OFFS[a][2]))
    }
    /// Dofs of active element `ae`.
    pub fn elem_dofs(&self, ae: usize) -> [u32; 8] {
        let nodes = self.elem_nodes(self.elems[ae].e as usize);
        std::array::from_fn(|a| self.dof_of[nodes[a]])
    }
    /// Dofs of a face.
    pub fn face_dofs(&self, f: &Face) -> [u32; 4] {
        let d = self.elem_dofs(f.elem as usize);
        let fl = face_nodes(f.dir as usize);
        std::array::from_fn(|a| d[fl[a]])
    }
    /// Neighbour grid element across face `dir`, None outside the grid.
    pub fn neighbour(&self, e: usize, dir: usize) -> Option<usize> {
        let c = self.ecoord(e);
        let d = DIRS[dir];
        let n = [c[0] as i32 + d[0], c[1] as i32 + d[1], c[2] as i32 + d[2]];
        if n[0] < 0 || n[1] < 0 || n[2] < 0 || n[0] >= self.nx as i32 || n[1] >= self.ny as i32 || n[2] >= self.nz as i32 {
            return None;
        }
        Some(self.eid(n[0] as usize, n[1] as usize, n[2] as usize))
    }

    /// Build the model from active elements. `group_of(active_elem, dir)`
    /// returns the coefficient group of an exposed face (None = adiabatic).
    pub fn new(
        nx: usize,
        ny: usize,
        nz: usize,
        h_mm: f64,
        origin: [f64; 3],
        elems: Vec<Elem>,
        group_of: impl Fn(&Elem, usize) -> Option<u16>,
    ) -> Model {
        let mut m = Model {
            nx,
            ny,
            nz,
            h_mm,
            origin,
            elems,
            elem_of: vec![NONE; nx * ny * nz],
            dof_of: vec![NONE; (nx + 1) * (ny + 1) * (nz + 1)],
            node_of: Vec::new(),
            faces: Vec::new(),
            cut_faces: Vec::new(),
        };
        for (ae, el) in m.elems.iter().enumerate() {
            m.elem_of[el.e as usize] = ae as u32;
        }
        let mut used = vec![false; m.dof_of.len()];
        for el in &m.elems {
            for n in m.elem_nodes(el.e as usize) {
                used[n] = true;
            }
        }
        for (n, u) in used.iter().enumerate() {
            if *u {
                m.dof_of[n] = m.node_of.len() as u32;
                m.node_of.push(n as u32);
            }
        }
        let mut faces = Vec::new();
        let mut cuts = Vec::new();
        for (ae, el) in m.elems.iter().enumerate() {
            for dir in 0..6 {
                match m.neighbour(el.e as usize, dir) {
                    None => cuts.push(Face {
                        elem: ae as u32,
                        dir: dir as u8,
                        group: 0,
                    }),
                    Some(nb) => {
                        if m.elem_of[nb] == NONE {
                            if let Some(g) = group_of(el, dir) {
                                faces.push(Face {
                                    elem: ae as u32,
                                    dir: dir as u8,
                                    group: g,
                                });
                            }
                        }
                    }
                }
            }
        }
        m.faces = faces;
        m.cut_faces = cuts;
        m
    }

    /// Lumped heat capacity per dof, J/K.
    pub fn capacity(&self) -> Vec<f64> {
        let hm = self.h_mm * 1e-3;
        let v8 = hm * hm * hm / 8.0;
        let mut c = vec![0.0; self.ndof()];
        for (ae, el) in self.elems.iter().enumerate() {
            for d in self.elem_dofs(ae) {
                c[d as usize] += el.rho_c * v8;
            }
        }
        c
    }

    /// Boundary conductance per dof for face-group coefficients `h` (W/m² K), W/K.
    pub fn face_diag(&self, h: &[f64]) -> Vec<f64> {
        let hm = self.h_mm * 1e-3;
        let a4 = hm * hm / 4.0;
        let mut d = vec![0.0; self.ndof()];
        for f in &self.faces {
            let hc = h.get(f.group as usize).copied().unwrap_or(0.0);
            for n in self.face_dofs(f) {
                d[n as usize] += hc * a4;
            }
        }
        d
    }

    /// Mean of nodal field over the faces of each group, area-weighted: (sum, area m²).
    pub fn face_group_means(&self, theta: &[f64], groups: usize) -> Vec<(f64, f64)> {
        let hm = self.h_mm * 1e-3;
        let a = hm * hm;
        let mut out = vec![(0.0, 0.0); groups];
        for f in &self.faces {
            let g = f.group as usize;
            if g >= groups {
                continue;
            }
            let t: f64 = self.face_dofs(f).iter().map(|&n| theta[n as usize]).sum::<f64>() / 4.0;
            out[g].0 += t * a;
            out[g].1 += a;
        }
        out
    }

    /// Assemble the conduction matrix K (W/K) in CSR form.
    pub fn assemble(&self) -> Csr {
        let n = self.ndof();
        // Row pattern: every active node within the 3×3×3 neighbourhood, in
        // grid order (so columns come out sorted).
        let mut ptr = Vec::with_capacity(n + 1);
        let mut col: Vec<u32> = Vec::with_capacity(n * 20);
        let mut slot = vec![NONE; n * 27];
        let mut diag = vec![0u32; n];
        ptr.push(0u32);
        for (dof, &node) in self.node_of.iter().enumerate() {
            let [i, j, k] = self.ncoord(node as usize);
            for dk in -1i32..=1 {
                for dj in -1i32..=1 {
                    for di in -1i32..=1 {
                        let (ii, jj, kk) = (i as i32 + di, j as i32 + dj, k as i32 + dk);
                        if ii < 0 || jj < 0 || kk < 0 || ii > self.nx as i32 || jj > self.ny as i32 || kk > self.nz as i32 {
                            continue;
                        }
                        let m = self.dof_of[self.nid(ii as usize, jj as usize, kk as usize)];
                        if m == NONE {
                            continue;
                        }
                        let o = ((dk + 1) * 9 + (dj + 1) * 3 + (di + 1)) as usize;
                        slot[dof * 27 + o] = col.len() as u32;
                        if m as usize == dof {
                            diag[dof] = col.len() as u32;
                        }
                        col.push(m);
                    }
                }
            }
            ptr.push(col.len() as u32);
        }
        let mut val = vec![0.0; col.len()];
        let uk = unit_k();
        let hm = self.h_mm * 1e-3;
        for (ae, el) in self.elems.iter().enumerate() {
            let dofs = self.elem_dofs(ae);
            let nodes = self.elem_nodes(el.e as usize);
            let s = [el.k[0] * hm, el.k[1] * hm, el.k[2] * hm];
            for a in 0..8 {
                let ca = self.ncoord(nodes[a]);
                let ra = dofs[a] as usize;
                for b in 0..8 {
                    let cb = self.ncoord(nodes[b]);
                    let o = ((cb[2] as i32 - ca[2] as i32 + 1) * 9
                        + (cb[1] as i32 - ca[1] as i32 + 1) * 3
                        + (cb[0] as i32 - ca[0] as i32 + 1)) as usize;
                    let p = slot[ra * 27 + o] as usize;
                    val[p] += s[0] * uk[0][a][b] + s[1] * uk[1][a][b] + s[2] * uk[2][a][b];
                }
            }
        }
        Csr { n, ptr, col, val, diag }
    }

    /// Interpolation stencil at a point: trilinear weights over the active
    /// nodes of the cell containing it (renormalised), or the nearest active
    /// node within two cells. Empty when nothing is near.
    pub fn stencil(&self, p: [f64; 3]) -> Vec<(u32, f64)> {
        let f: [f64; 3] = std::array::from_fn(|d| (p[d] - self.origin[d]) / self.h_mm);
        let dims = [self.nx, self.ny, self.nz];
        let c: [usize; 3] = std::array::from_fn(|d| (f[d].floor().max(0.0) as usize).min(dims[d].saturating_sub(1)));
        let t: [f64; 3] = std::array::from_fn(|d| (f[d] - c[d] as f64).clamp(0.0, 1.0));
        let mut out = Vec::with_capacity(8);
        let mut sw = 0.0;
        for o in OFFS {
            let dof = self.dof_of[self.nid(c[0] + o[0], c[1] + o[1], c[2] + o[2])];
            if dof == NONE {
                continue;
            }
            let w: f64 = (0..3).map(|d| if o[d] == 1 { t[d] } else { 1.0 - t[d] }).product::<f64>() + 1e-9;
            sw += w;
            out.push((dof, w));
        }
        if sw > 0.0 {
            for x in out.iter_mut() {
                x.1 /= sw;
            }
            return out;
        }
        let mut best: Option<(f64, u32)> = None;
        for dk in -2i32..=3 {
            for dj in -2i32..=3 {
                for di in -2i32..=3 {
                    let (i, j, k) = (c[0] as i32 + di, c[1] as i32 + dj, c[2] as i32 + dk);
                    if i < 0 || j < 0 || k < 0 || i > self.nx as i32 || j > self.ny as i32 || k > self.nz as i32 {
                        continue;
                    }
                    let n = self.nid(i as usize, j as usize, k as usize);
                    let dof = self.dof_of[n];
                    if dof == NONE {
                        continue;
                    }
                    let q = self.node_pos(n);
                    let d2 = (0..3).map(|d| (q[d] - p[d]).powi(2)).sum::<f64>();
                    if best.is_none_or(|(b, _)| d2 < b) {
                        best = Some((d2, dof));
                    }
                }
            }
        }
        best.map(|(_, d)| vec![(d, 1.0)]).unwrap_or_default()
    }

    /// Nodal field at a point (see [`Model::stencil`]).
    pub fn sample(&self, theta: &[f64], p: [f64; 3]) -> Option<f64> {
        let st = self.stencil(p);
        if st.is_empty() {
            return None;
        }
        Some(st.iter().map(|(d, w)| theta[*d as usize] * w).sum())
    }

    /// Spread `value` at point `p` onto nearby active nodes (stencil weights);
    /// returns false if the point is outside the grid or nothing is near.
    pub fn deposit(&self, out: &mut [f64], p: [f64; 3], value: f64) -> bool {
        let dims = [self.nx, self.ny, self.nz];
        if (0..3).any(|d| p[d] < self.origin[d] || p[d] > self.origin[d] + dims[d] as f64 * self.h_mm) {
            return false;
        }
        let st = self.stencil(p);
        for (d, w) in &st {
            out[*d as usize] += value * w;
        }
        !st.is_empty()
    }
}

/// Symmetric matrix in compressed sparse row form (full rows stored).
#[derive(Clone, Debug)]
pub struct Csr {
    pub n: usize,
    pub ptr: Vec<u32>,
    pub col: Vec<u32>,
    pub val: Vec<f64>,
    /// Position of the diagonal entry in each row.
    pub diag: Vec<u32>,
}

impl Csr {
    /// y = (A + diag(d)) x
    pub fn mul(&self, d: &[f64], x: &[f64], y: &mut [f64]) {
        for i in 0..self.n {
            let (a, b) = (self.ptr[i] as usize, self.ptr[i + 1] as usize);
            let mut s = d[i] * x[i];
            for p in a..b {
                s += self.val[p] * x[self.col[p] as usize];
            }
            y[i] = s;
        }
    }
    pub fn nnz(&self) -> usize {
        self.val.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Precond {
    Jacobi,
    /// Symmetric successive over-relaxation with factor ω ∈ (0, 2).
    Ssor(f64),
}

impl Precond {
    pub fn parse(s: &str) -> Precond {
        match s {
            "jacobi" => Precond::Jacobi,
            _ => Precond::Ssor(1.4),
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Precond::Jacobi => "jacobi",
            Precond::Ssor(_) => "ssor",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct PcgStats {
    pub iterations: usize,
    pub converged: bool,
    /// Relative residual ‖b − A x‖ / ‖b‖ per iteration (first entry: start).
    pub history: Vec<f64>,
}

fn apply_pc(a: &Csr, d: &[f64], pc: Precond, r: &[f64], z: &mut [f64]) {
    match pc {
        Precond::Jacobi => {
            for i in 0..a.n {
                z[i] = r[i] / (a.val[a.diag[i] as usize] + d[i]);
            }
        }
        Precond::Ssor(w) => {
            // M = ω/(2−ω) · (D/ω + L) (D/ω)⁻¹ (D/ω + U)
            let dw = |i: usize| (a.val[a.diag[i] as usize] + d[i]) / w;
            for i in 0..a.n {
                let mut s = r[i];
                for p in a.ptr[i] as usize..a.diag[i] as usize {
                    s -= a.val[p] * z[a.col[p] as usize];
                }
                z[i] = s / dw(i);
            }
            for i in 0..a.n {
                z[i] *= dw(i);
            }
            for i in (0..a.n).rev() {
                let mut s = z[i];
                for p in a.diag[i] as usize + 1..a.ptr[i + 1] as usize {
                    s -= a.val[p] * z[a.col[p] as usize];
                }
                z[i] = s / dw(i);
            }
            let k = (2.0 - w) / w;
            for zi in z.iter_mut() {
                *zi *= k;
            }
        }
    }
}

/// Solve (A + diag(d)) x = b by preconditioned conjugate gradients; `x` is
/// the initial guess and receives the solution.
pub fn pcg(a: &Csr, d: &[f64], b: &[f64], x: &mut [f64], pc: Precond, tol: f64, max_iter: usize) -> PcgStats {
    let n = a.n;
    let bn = b.iter().map(|v| v * v).sum::<f64>().sqrt();
    let mut st = PcgStats::default();
    if bn == 0.0 {
        x.iter_mut().for_each(|v| *v = 0.0);
        st.converged = true;
        st.history.push(0.0);
        return st;
    }
    let mut r = vec![0.0; n];
    a.mul(d, x, &mut r);
    for i in 0..n {
        r[i] = b[i] - r[i];
    }
    let mut z = vec![0.0; n];
    apply_pc(a, d, pc, &r, &mut z);
    let mut p = z.clone();
    let mut rz: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
    let mut q = vec![0.0; n];
    let mut rn = r.iter().map(|v| v * v).sum::<f64>().sqrt() / bn;
    st.history.push(rn);
    while st.iterations < max_iter && rn > tol {
        a.mul(d, &p, &mut q);
        let pq: f64 = p.iter().zip(&q).map(|(a, b)| a * b).sum();
        if pq <= 0.0 {
            break;
        }
        let alpha = rz / pq;
        for i in 0..n {
            x[i] += alpha * p[i];
            r[i] -= alpha * q[i];
        }
        rn = r.iter().map(|v| v * v).sum::<f64>().sqrt() / bn;
        st.iterations += 1;
        st.history.push(rn);
        if rn <= tol {
            break;
        }
        apply_pc(a, d, pc, &r, &mut z);
        let rz_new: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
        let beta = rz_new / rz;
        rz = rz_new;
        for i in 0..n {
            p[i] = z[i] + beta * p[i];
        }
    }
    st.converged = rn <= tol;
    st
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bar of nx × ny × nz voxels of one material.
    fn bar(nx: usize, ny: usize, nz: usize, h: f64, k: f64, rho_c: f64, group_of: impl Fn(&Elem, usize) -> Option<u16>) -> Model {
        // one voxel of empty margin on every side so exposed faces exist
        let elems = (0..nx * ny * nz)
            .map(|e| {
                let (i, j, kk) = (e % nx, (e / nx) % ny, e / (nx * ny));
                Elem {
                    e: (i + 1 + (nx + 2) * (j + 1 + (ny + 2) * (kk + 1))) as u32,
                    k: [k; 3],
                    rho_c,
                    mat: 0,
                    part: 0,
                }
            })
            .collect();
        Model::new(nx + 2, ny + 2, nz + 2, h, [-h; 3], elems, group_of)
    }

    #[test]
    fn unit_matrices_have_zero_row_sums() {
        let uk = unit_k();
        for kd in &uk {
            for row in kd {
                assert!(row.iter().sum::<f64>().abs() < 1e-14);
            }
        }
        // diagonal = 1 · 1/3 · 1/3 = 1/9
        assert!((uk[0][0][0] - 1.0 / 9.0).abs() < 1e-15);
        assert_eq!(face_nodes(1), [1, 3, 5, 7]);
    }

    #[test]
    fn slab_1d_matches_analytic() {
        // 10 mm slab along x (k = 0.2), 1 W/m² … heat flux q'' in at x = 0
        // (deposited on the x = 0 nodes), convection h = 10 on x = L only.
        // T(0) − T∞ = q''(L/k + 1/h); T(L) − T∞ = q''/h; linear in between.
        let (nx, h) = (10, 1.0);
        let m = bar(nx, 3, 3, h, 0.2, 1e6, |_, dir| if dir == 1 { Some(0) } else { None });
        let a = m.assemble();
        let hd = m.face_diag(&[10.0]);
        let qflux = 500.0; // W/m²
        let area = (3.0 * h * 1e-3) * (3.0 * h * 1e-3);
        let mut b = vec![0.0; m.ndof()];
        let nodes_x0: Vec<usize> = (0..m.ndof()).filter(|&d| m.ncoord(m.node_of[d] as usize)[0] == 1).collect();
        // x=0 face of the bar is grid node plane i = 1; weight like a lumped face
        let mut wsum = 0.0;
        let ws: Vec<f64> = nodes_x0
            .iter()
            .map(|&d| {
                let c = m.ncoord(m.node_of[d] as usize);
                let edge = |v: usize| if v == 1 || v == 4 { 0.5 } else { 1.0 };
                let w = edge(c[1]) * edge(c[2]);
                wsum += w;
                w
            })
            .collect();
        for (&d, w) in nodes_x0.iter().zip(&ws) {
            b[d] = qflux * area * w / wsum;
        }
        let mut x = vec![0.0; m.ndof()];
        let st = pcg(&a, &hd, &b, &mut x, Precond::Ssor(1.4), 1e-12, 500);
        assert!(st.converged);
        let l = nx as f64 * h * 1e-3;
        for d in 0..m.ndof() {
            let c = m.ncoord(m.node_of[d] as usize);
            let xpos = (c[0] as f64 - 1.0) * h * 1e-3;
            let exact = qflux * ((l - xpos) / 0.2 + 1.0 / 10.0);
            assert!((x[d] - exact).abs() / exact < 1e-8, "x={xpos} {} vs {exact}", x[d]);
        }
    }

    #[test]
    fn fin_with_convection_matches_cosh_solution() {
        // Pin fin 40 × 2 × 2 mm, k = 20 W/m K, h = 25 W/m²K on the sides,
        // adiabatic tip, heat Q injected at the base. Classic result
        // (Incropera §3.6.4): θ_b = Q / (√(hPkA_c) tanh(mL)), θ(x) = θ_b cosh(m(L−x))/cosh(mL).
        let (nx, h) = (40, 1.0);
        let m = bar(nx, 2, 2, h, 20.0, 1e6, |_, dir| if dir >= 2 { Some(0) } else { None });
        let a = m.assemble();
        let hc = 25.0;
        let hd = m.face_diag(&[hc]);
        let q = 0.05;
        let mut b = vec![0.0; m.ndof()];
        let base: Vec<usize> = (0..m.ndof()).filter(|&d| m.ncoord(m.node_of[d] as usize)[0] == 1).collect();
        // base face: 3 × 3 nodes, lumped face weights 1/4 1/2 1 pattern
        let mut ws = Vec::new();
        for &d in &base {
            let c = m.ncoord(m.node_of[d] as usize);
            let edge = |v: usize| if v == 2 { 1.0 } else { 0.5 };
            ws.push(edge(c[1]) * edge(c[2]));
        }
        let wsum: f64 = ws.iter().sum();
        for (&d, w) in base.iter().zip(&ws) {
            b[d] = q * w / wsum;
        }
        let mut x = vec![0.0; m.ndof()];
        let st = pcg(&a, &hd, &b, &mut x, Precond::Jacobi, 1e-12, 5000);
        assert!(st.converged);
        let (w, l) = (2e-3, 40e-3);
        let (pp, ac) = (4.0 * w, w * w);
        let mm = (hc * pp / (20.0 * ac)).sqrt();
        let theta_b = q / ((hc * pp * 20.0 * ac).sqrt() * (mm * l).tanh());
        let mean_at = |ix: usize| {
            let v: Vec<f64> = (0..m.ndof())
                .filter(|&d| m.ncoord(m.node_of[d] as usize)[0] == ix)
                .map(|d| x[d])
                .collect();
            v.iter().sum::<f64>() / v.len() as f64
        };
        let tb = mean_at(1);
        assert!((tb - theta_b).abs() / theta_b < 0.02, "{tb} vs {theta_b}");
        let x_mid = 20e-3;
        let exact_mid = theta_b * (mm * (l - x_mid)).cosh() / (mm * l).cosh();
        let mid = mean_at(21);
        assert!((mid - exact_mid).abs() / exact_mid < 0.02, "{mid} vs {exact_mid}");
    }

    #[test]
    fn lumped_capacitance_transient() {
        // Small copper cube (Bi ≪ 0.1) cooling: θ(t) = θ₀ e^{−t/τ}, τ = ρcV/(hA).
        let h = 1.0;
        let rho_c = 8960.0 * 385.0;
        let m = bar(4, 4, 4, h, 390.0, rho_c, |_, _| Some(0));
        let a = m.assemble();
        let hc = 50.0;
        let hd = m.face_diag(&[hc]);
        let c = m.capacity();
        let side: f64 = 4e-3;
        let tau = rho_c * side.powi(3) / (hc * 6.0 * side * side);
        let dt = tau / 200.0;
        let mut theta = vec![10.0; m.ndof()];
        let diag: Vec<f64> = hd.iter().zip(&c).map(|(h, c)| h + c / dt).collect();
        let steps = 200;
        for _ in 0..steps {
            let rhs: Vec<f64> = c.iter().zip(&theta).map(|(c, t)| c / dt * t).collect();
            let mut x = theta.clone();
            pcg(&a, &diag, &rhs, &mut x, Precond::Ssor(1.2), 1e-12, 200);
            theta = x;
        }
        // backward Euler: θ = θ₀ (1 + Δt/τ)^(−n)
        let be = 10.0 * (1.0 + dt / tau).powi(-steps);
        let exact = 10.0 * (-1.0f64).exp();
        let mean = theta.iter().sum::<f64>() / theta.len() as f64;
        assert!((mean - be).abs() / be < 2e-3, "{mean} vs {be}");
        assert!((mean - exact).abs() / exact < 0.01, "{mean} vs {exact}");
        // Biot number is tiny → the cube is isothermal
        let spread = theta.iter().cloned().fold(f64::MIN, f64::max) - theta.iter().cloned().fold(f64::MAX, f64::min);
        assert!(spread < 1e-3 * mean);
    }

    #[test]
    fn ssor_beats_jacobi() {
        let m = bar(30, 6, 6, 1.0, 0.2, 1e6, |_, _| Some(0));
        let a = m.assemble();
        let hd = m.face_diag(&[10.0]);
        let mut b = vec![0.0; m.ndof()];
        b[0] = 0.01;
        let mut x1 = vec![0.0; m.ndof()];
        let j = pcg(&a, &hd, &b, &mut x1, Precond::Jacobi, 1e-8, 5000);
        let mut x2 = vec![0.0; m.ndof()];
        let s = pcg(&a, &hd, &b, &mut x2, Precond::Ssor(1.4), 1e-8, 5000);
        assert!(j.converged && s.converged);
        assert!(s.iterations < j.iterations, "{} vs {}", s.iterations, j.iterations);
        for (p, q) in x1.iter().zip(&x2) {
            assert!((p - q).abs() < 1e-5 * x1[0].abs().max(1e-12));
        }
    }
}
