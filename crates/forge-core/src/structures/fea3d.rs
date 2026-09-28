//! Voxel finite element solver: 8-node bricks on a regular grid, matrix-free
//! preconditioned conjugate gradient (PCG).
//!
//! ## Problem
//! Linear elasticity, small strains: find nodal displacements u with
//! `K u = f`, `K = Σ_e ρ_e K_e` assembled from identical cube elements
//! ([`super::hex8`]); ρ_e ∈ (0, 1] is the fraction of the voxel inside the
//! part (partial-volume voxels keep walls thinner than a voxel at their true
//! membrane stiffness). Fixed degrees of freedom (DOFs) are removed by
//! masking: every vector the solver touches is zero on them.
//!
//! ## Matrix-free conjugate gradient
//! K is never assembled. `K·p` loops over the active voxels, gathers the 24
//! nodal values, multiplies by the 24 × 24 element matrix and scatters the
//! result back — memory stays O(nodes) and the product costs 576 multiply-adds
//! per element. CG minimises the energy ½uᵀKu − fᵀu over growing Krylov
//! spaces; each iteration needs one product, two dot products and three
//! vector updates, and the residual ‖f − Ku‖/‖f‖ is recorded every
//! iteration (the convergence history the lab plots). Reference:
//! J. R. Shewchuk, "An Introduction to the Conjugate Gradient Method Without
//! the Agonizing Pain", Carnegie Mellon University, 1994.
//!
//! ## Preconditioners
//! * **Jacobi** — divide by diag(K). Cheap, but the iteration count grows
//!   with the square root of the condition number, which for slender parts
//!   (a 145 mm temple, 4 mm thick) runs into the thousands.
//! * **Geometric multigrid** (default) — one symmetric V-cycle per CG
//!   iteration: Chebyshev-polynomial smoothing (Jacobi-preconditioned) on each
//!   level, coarse levels built by merging 2 × 2 × 2 voxels with *Galerkin*
//!   coarse operators `K_c = Pᵀ K P` (P = trilinear interpolation, assembled
//!   per coarse element so it stays exact for any voxel pattern), and a dense
//!   Cholesky solve on the coarsest level. Iteration counts become almost
//!   independent of mesh size. References: O. Amir, N. Aage, B. S. Lazarov,
//!   "On multigrid-CG for efficient topology optimization", Struct.
//!   Multidisc. Optim. 49, 2014; M. Adams, M. Brezina, J. Hu, R. Tuminaro,
//!   "Parallel multigrid smoothing: polynomial versus Gauss–Seidel", J.
//!   Comput. Phys. 188, 2003; W. L. Briggs, V. E. Henson, S. F. McCormick,
//!   *A Multigrid Tutorial*, 2nd ed., SIAM, 2000.

use std::collections::HashMap;

use super::hex8::{normal_stress, von_mises, UnitHex, OFFS};

/// Regular voxel grid. Element (i, j, k) spans
/// `origin + h·[i, i+1] × [j, j+1] × [k, k+1]`.
#[derive(Clone, Debug)]
pub struct Grid {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    pub h: f64,
    pub origin: [f64; 3],
    /// Solid flag per element, index `i + nx (j + ny k)`.
    pub solid: Vec<bool>,
    /// Material fraction per element (1 for fully inside).
    pub rho: Vec<f32>,
}

impl Grid {
    pub fn new(nx: usize, ny: usize, nz: usize, h: f64, origin: [f64; 3]) -> Grid {
        Grid {
            nx,
            ny,
            nz,
            h,
            origin,
            solid: vec![false; nx * ny * nz],
            rho: vec![1.0; nx * ny * nz],
        }
    }
    /// Mark every voxel solid with full density.
    pub fn fill_all(&mut self) {
        self.solid.iter_mut().for_each(|s| *s = true);
        self.rho.iter_mut().for_each(|r| *r = 1.0);
    }
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
    pub fn nnodes(&self) -> usize {
        (self.nx + 1) * (self.ny + 1) * (self.nz + 1)
    }
    pub fn node_pos(&self, n: usize) -> [f64; 3] {
        let i = n % (self.nx + 1);
        let j = (n / (self.nx + 1)) % (self.ny + 1);
        let k = n / ((self.nx + 1) * (self.ny + 1));
        [
            self.origin[0] + self.h * i as f64,
            self.origin[1] + self.h * j as f64,
            self.origin[2] + self.h * k as f64,
        ]
    }
    pub fn center(&self, e: usize) -> [f64; 3] {
        let [i, j, k] = self.ecoord(e);
        [
            self.origin[0] + self.h * (i as f64 + 0.5),
            self.origin[1] + self.h * (j as f64 + 0.5),
            self.origin[2] + self.h * (k as f64 + 0.5),
        ]
    }
    pub fn elem_nodes(&self, e: usize) -> [u32; 8] {
        let [i, j, k] = self.ecoord(e);
        let mut out = [0u32; 8];
        for (a, o) in OFFS.iter().enumerate() {
            out[a] = self.nid(i + o[0], j + o[1], k + o[2]) as u32;
        }
        out
    }
    pub fn count(&self) -> usize {
        self.solid.iter().filter(|s| **s).count()
    }
    /// Material volume Σ ρ h³, mm³.
    pub fn volume(&self) -> f64 {
        let v: f64 = self.solid.iter().zip(&self.rho).filter(|(s, _)| **s).map(|(_, r)| *r as f64).sum();
        v * self.h * self.h * self.h
    }
    /// Number of solid voxels sharing each node (0..8).
    pub fn node_valence(&self) -> Vec<u8> {
        let mut v = vec![0u8; self.nnodes()];
        for e in 0..self.solid.len() {
            if self.solid[e] {
                for n in self.elem_nodes(e) {
                    v[n as usize] += 1;
                }
            }
        }
        v
    }
    /// Keep only voxels face-connected to a seed voxel; returns how many were removed.
    pub fn keep_connected(&mut self, seeds: &[usize]) -> usize {
        let mut seen = vec![false; self.solid.len()];
        let mut stack: Vec<usize> = seeds.iter().copied().filter(|&e| self.solid[e]).collect();
        for &e in &stack {
            seen[e] = true;
        }
        while let Some(e) = stack.pop() {
            let [i, j, k] = self.ecoord(e);
            let mut push = |ii: usize, jj: usize, kk: usize| {
                let n = self.eid(ii, jj, kk);
                if self.solid[n] && !seen[n] {
                    seen[n] = true;
                    stack.push(n);
                }
            };
            if i > 0 {
                push(i - 1, j, k);
            }
            if i + 1 < self.nx {
                push(i + 1, j, k);
            }
            if j > 0 {
                push(i, j - 1, k);
            }
            if j + 1 < self.ny {
                push(i, j + 1, k);
            }
            if k > 0 {
                push(i, j, k - 1);
            }
            if k + 1 < self.nz {
                push(i, j, k + 1);
            }
        }
        let mut removed = 0;
        for (s, v) in self.solid.iter_mut().zip(seen.iter()) {
            if *s && !*v {
                *s = false;
                removed += 1;
            }
        }
        removed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Precond {
    Jacobi,
    Multigrid,
}

#[derive(Clone, Debug)]
pub struct SolveOpts {
    pub e_mpa: f64,
    pub nu: f64,
    pub incompatible: bool,
    pub precond: Precond,
    pub tol: f64,
    pub max_iter: usize,
}

impl Default for SolveOpts {
    fn default() -> Self {
        SolveOpts {
            e_mpa: 2000.0,
            nu: 0.35,
            incompatible: true,
            precond: Precond::Multigrid,
            tol: 1e-6,
            max_iter: 2000,
        }
    }
}

/// Boundary conditions on grid nodes: per-node bitmask of fixed components
/// (bit d = axis d) and the nodal force vector (N), length 3 × nodes.
#[derive(Clone, Debug)]
pub struct Bcs {
    pub fixed: Vec<u8>,
    pub force: Vec<f64>,
}

impl Bcs {
    pub fn new(g: &Grid) -> Bcs {
        Bcs {
            fixed: vec![0; g.nnodes()],
            force: vec![0.0; 3 * g.nnodes()],
        }
    }
    /// Clamp the given components of every node of the listed voxels.
    pub fn fix_voxels(&mut self, g: &Grid, voxels: &[usize], mask: u8) {
        for &e in voxels {
            for n in g.elem_nodes(e) {
                self.fixed[n as usize] |= mask;
            }
        }
    }
    /// Spread a total force equally over the unique nodes of the listed voxels.
    pub fn load_voxels(&mut self, g: &Grid, voxels: &[usize], total: [f64; 3]) -> usize {
        let mut nodes: Vec<u32> = voxels.iter().flat_map(|&e| g.elem_nodes(e)).collect();
        nodes.sort_unstable();
        nodes.dedup();
        if nodes.is_empty() {
            return 0;
        }
        let k = 1.0 / nodes.len() as f64;
        for n in &nodes {
            for d in 0..3 {
                self.force[3 * *n as usize + d] += total[d] * k;
            }
        }
        nodes.len()
    }
}

// ---------------------------------------------------------------- kernels

type Mat24 = [f64; 576];

/// ye = K ue for a symmetric 24 × 24 matrix (column-wise axpy form, which
/// vectorises well).
#[inline(always)]
fn kmul(k: &Mat24, ue: &[f64; 24]) -> [f64; 24] {
    let mut ye = [0.0f64; 24];
    for c in 0..24 {
        let uc = ue[c];
        let col: &[f64; 24] = k[c * 24..c * 24 + 24].try_into().unwrap();
        for r in 0..24 {
            ye[r] += col[r] * uc;
        }
    }
    ye
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    let mut s = [0.0f64; 4];
    let n4 = a.len() / 4 * 4;
    for i in (0..n4).step_by(4) {
        s[0] += a[i] * b[i];
        s[1] += a[i + 1] * b[i + 1];
        s[2] += a[i + 2] * b[i + 2];
        s[3] += a[i + 3] * b[i + 3];
    }
    let mut t = s[0] + s[1] + s[2] + s[3];
    for i in n4..a.len() {
        t += a[i] * b[i];
    }
    t
}

// ---------------------------------------------------------------- levels

enum Mats {
    /// Element e is the unit hex scaled by `scale[e]` (ρ E h).
    Uniform(Vec<f64>),
    /// Explicit 24 × 24 matrix per active element.
    Explicit(Vec<Mat24>),
}

/// One level of the hierarchy with compact node numbering (only nodes that
/// belong to an active element are stored).
struct Level {
    dims: [usize; 3],
    /// Grid coordinates of each active element.
    ecoord: Vec<[u32; 3]>,
    /// Compact node indices of each active element.
    enodes: Vec<[u32; 8]>,
    /// Grid node id of each compact node.
    gnode: Vec<u32>,
    mats: Mats,
    free: Vec<bool>,
    inv_diag: Vec<f64>,
    lmax: f64,
}

impl Level {
    fn ndof(&self) -> usize {
        3 * self.gnode.len()
    }
    fn gcoord(&self, g: usize) -> [usize; 3] {
        let (a, b) = (self.dims[0] + 1, self.dims[1] + 1);
        [g % a, (g / a) % b, g / (a * b)]
    }
    #[inline]
    fn mat<'a>(&'a self, e: usize, unit: &'a Mat24) -> (&'a Mat24, f64) {
        match &self.mats {
            Mats::Uniform(s) => (unit, s[e]),
            Mats::Explicit(m) => (&m[e], 1.0),
        }
    }
    /// y = K x on free DOFs (y is zero elsewhere).
    fn apply(&self, unit: &Mat24, x: &[f64], y: &mut [f64]) {
        y.iter_mut().for_each(|v| *v = 0.0);
        let mut ue = [0.0; 24];
        for (e, nodes) in self.enodes.iter().enumerate() {
            for (a, &n) in nodes.iter().enumerate() {
                let b = 3 * n as usize;
                ue[3 * a] = x[b];
                ue[3 * a + 1] = x[b + 1];
                ue[3 * a + 2] = x[b + 2];
            }
            let (k, s) = self.mat(e, unit);
            let ye = kmul(k, &ue);
            for (a, &n) in nodes.iter().enumerate() {
                let b = 3 * n as usize;
                y[b] += s * ye[3 * a];
                y[b + 1] += s * ye[3 * a + 1];
                y[b + 2] += s * ye[3 * a + 2];
            }
        }
        for (v, f) in y.iter_mut().zip(self.free.iter()) {
            if !*f {
                *v = 0.0;
            }
        }
    }
    fn diag(&self, unit: &Mat24) -> Vec<f64> {
        let mut dg = vec![0.0; self.ndof()];
        for (e, nodes) in self.enodes.iter().enumerate() {
            let (k, s) = self.mat(e, unit);
            for (a, &n) in nodes.iter().enumerate() {
                for d in 0..3 {
                    let r = 3 * a + d;
                    dg[3 * n as usize + d] += s * k[r * 24 + r];
                }
            }
        }
        dg
    }
    /// Chebyshev smoother (degree `deg`) on D⁻¹K for the interval
    /// [λmax/ratio, 1.1 λmax]; improves x in place.
    fn smooth(&self, unit: &Mat24, b: &[f64], x: &mut [f64], deg: usize, from_zero: bool, work: &mut [Vec<f64>; 3]) {
        let upper = 1.1 * self.lmax;
        let lower = self.lmax / 8.0;
        let theta = 0.5 * (upper + lower);
        let delta = 0.5 * (upper - lower);
        let n = b.len();
        let [r, d, kd] = work;
        if from_zero {
            r.copy_from_slice(b);
        } else {
            self.apply(unit, x, r);
            for i in 0..n {
                r[i] = b[i] - r[i];
            }
        }
        for i in 0..n {
            d[i] = self.inv_diag[i] * r[i] / theta;
            x[i] += d[i];
        }
        let mut rho_old = delta / theta;
        for _ in 1..deg {
            let rho = 1.0 / (2.0 * theta / delta - rho_old);
            self.apply(unit, d, kd);
            for i in 0..n {
                r[i] -= kd[i];
            }
            for i in 0..n {
                d[i] = rho * rho_old * d[i] + 2.0 * rho / delta * self.inv_diag[i] * r[i];
                x[i] += d[i];
            }
            rho_old = rho;
        }
    }
}

/// Dense Cholesky factor of the coarsest operator.
struct Coarse {
    map: Vec<usize>,
    n: usize,
    l: Vec<f64>,
}

impl Coarse {
    fn solve(&self, b: &[f64], x: &mut [f64]) {
        let n = self.n;
        let mut y = vec![0.0; n];
        for (dof, &m) in self.map.iter().enumerate() {
            if m != usize::MAX {
                y[m] = b[dof];
            }
        }
        for i in 0..n {
            let row = &self.l[i * n..i * n + i];
            let s = y[i] - dot(row, &y[..i]);
            y[i] = s / self.l[i * n + i];
        }
        for i in (0..n).rev() {
            let mut s = y[i];
            for k in i + 1..n {
                s -= self.l[k * n + i] * y[k];
            }
            y[i] = s / self.l[i * n + i];
        }
        x.iter_mut().for_each(|v| *v = 0.0);
        for (dof, &m) in self.map.iter().enumerate() {
            if m != usize::MAX {
                x[dof] = y[m];
            }
        }
    }
}

/// Transfer operator between a fine level and the next coarser one: for
/// each fine compact node, up to 8 (coarse compact node, weight) pairs.
struct Transfer {
    off: Vec<u32>,
    col: Vec<u32>,
    w: Vec<f64>,
}

/// Maximum free DOFs of the level solved directly.
const COARSE_MAX: usize = 240;
const MAX_LEVELS: usize = 10;
/// Chebyshev smoother degree (pre and post).
const SMOOTH_DEG: usize = 2;

/// Trilinear interpolation weights of fine node index i onto coarse nodes.
fn axis_w(i: usize) -> ([usize; 2], [f64; 2], usize) {
    if i.is_multiple_of(2) {
        ([i / 2, 0], [1.0, 0.0], 1)
    } else {
        ([i / 2, i / 2 + 1], [0.5, 0.5], 2)
    }
}

pub struct Operator {
    unit: Box<Mat24>,
    levels: Vec<Level>,
    transfers: Vec<Transfer>,
    coarse: Option<Coarse>,
    pub precond: Precond,
    /// Grid node → level-0 compact node (u32::MAX when unattached).
    node_map: Vec<u32>,
}

impl Operator {
    pub fn new(g: &Grid, fixed: &[u8], opts: &SolveOpts, hex: &UnitHex) -> Operator {
        let mut unit = Box::new([0.0; 576]);
        unit.copy_from_slice(&hex.k);
        // ---- level 0 with compact nodes
        let mut node_map = vec![u32::MAX; g.nnodes()];
        let mut gnode = Vec::new();
        let mut ecoord = Vec::new();
        let mut enodes = Vec::new();
        let mut scale = Vec::new();
        for (e, s) in g.solid.iter().enumerate() {
            if !*s {
                continue;
            }
            let [i, j, k] = g.ecoord(e);
            ecoord.push([i as u32, j as u32, k as u32]);
            let mut en = [0u32; 8];
            for (a, gn) in g.elem_nodes(e).iter().enumerate() {
                let m = &mut node_map[*gn as usize];
                if *m == u32::MAX {
                    *m = gnode.len() as u32;
                    gnode.push(*gn);
                }
                en[a] = *m;
            }
            enodes.push(en);
            scale.push(opts.e_mpa * g.h * g.rho[e] as f64);
        }
        let free: Vec<bool> = (0..3 * gnode.len())
            .map(|i| fixed[gnode[i / 3] as usize] & (1 << (i % 3)) == 0)
            .collect();
        let mut l0 = Level {
            dims: [g.nx, g.ny, g.nz],
            ecoord,
            enodes,
            gnode,
            mats: Mats::Uniform(scale),
            free,
            inv_diag: vec![],
            lmax: 1.0,
        };
        finish_level(&mut l0, &unit);
        let mut levels = vec![l0];
        let mut transfers = Vec::new();
        let mut coarse = None;
        if opts.precond == Precond::Multigrid {
            loop {
                let last = levels.last().unwrap();
                let nfree = last.free.iter().filter(|f| **f).count();
                if nfree <= COARSE_MAX || levels.len() >= MAX_LEVELS || last.dims.iter().all(|&d| d <= 1) {
                    break;
                }
                let (next, tr) = coarsen(last, &unit);
                levels.push(next);
                transfers.push(tr);
            }
            coarse = Some(factor(levels.last().unwrap(), &unit));
        }
        Operator {
            unit,
            levels,
            transfers,
            coarse,
            precond: opts.precond,
            node_map,
        }
    }

    pub fn levels(&self) -> usize {
        self.levels.len()
    }

    pub fn free_dofs(&self) -> usize {
        self.levels[0].free.iter().filter(|f| **f).count()
    }

    fn precondition(&self, r: &[f64], z: &mut [f64]) {
        match self.precond {
            Precond::Jacobi => {
                for ((zi, ri), di) in z.iter_mut().zip(r).zip(&self.levels[0].inv_diag) {
                    *zi = ri * di;
                }
            }
            Precond::Multigrid => self.vcycle(0, r, z),
        }
    }

    fn vcycle(&self, l: usize, b: &[f64], x: &mut [f64]) {
        if l + 1 == self.levels.len() {
            match &self.coarse {
                Some(c) => c.solve(b, x),
                None => x.iter_mut().for_each(|v| *v = 0.0),
            }
            return;
        }
        let lv = &self.levels[l];
        let n = b.len();
        let mut work = [vec![0.0; n], vec![0.0; n], vec![0.0; n]];
        x.iter_mut().for_each(|v| *v = 0.0);
        lv.smooth(&self.unit, b, x, SMOOTH_DEG, true, &mut work);
        let mut ax = vec![0.0; n];
        lv.apply(&self.unit, x, &mut ax);
        for i in 0..n {
            ax[i] = if lv.free[i] { b[i] - ax[i] } else { 0.0 };
        }
        let cl = &self.levels[l + 1];
        let tr = &self.transfers[l];
        let mut rc = vec![0.0; cl.ndof()];
        for fnode in 0..lv.gnode.len() {
            for t in tr.off[fnode] as usize..tr.off[fnode + 1] as usize {
                let (c, w) = (3 * tr.col[t] as usize, tr.w[t]);
                for d in 0..3 {
                    rc[c + d] += w * ax[3 * fnode + d];
                }
            }
        }
        for (v, f) in rc.iter_mut().zip(cl.free.iter()) {
            if !*f {
                *v = 0.0;
            }
        }
        let mut ec = vec![0.0; cl.ndof()];
        self.vcycle(l + 1, &rc, &mut ec);
        for fnode in 0..lv.gnode.len() {
            let mut v = [0.0; 3];
            for t in tr.off[fnode] as usize..tr.off[fnode + 1] as usize {
                let (c, w) = (3 * tr.col[t] as usize, tr.w[t]);
                for d in 0..3 {
                    v[d] += w * ec[c + d];
                }
            }
            for d in 0..3 {
                if lv.free[3 * fnode + d] {
                    x[3 * fnode + d] += v[d];
                }
            }
        }
        lv.smooth(&self.unit, b, x, SMOOTH_DEG, false, &mut work);
    }

    /// Preconditioned conjugate gradient on grid-node vectors; returns
    /// (u on grid nodes, relative residual history, converged).
    pub fn pcg(&self, f_grid: &[f64], tol: f64, max_iter: usize) -> (Vec<f64>, Vec<f64>, bool) {
        let l0 = &self.levels[0];
        let n = l0.ndof();
        let b: Vec<f64> = (0..n)
            .map(|i| {
                if l0.free[i] {
                    f_grid[3 * l0.gnode[i / 3] as usize + i % 3]
                } else {
                    0.0
                }
            })
            .collect();
        let bnorm = dot(&b, &b).sqrt();
        let mut x = vec![0.0; n];
        let mut hist = vec![1.0];
        let mut converged = bnorm == 0.0;
        if !converged {
            let mut r = b.clone();
            let mut z = vec![0.0; n];
            self.precondition(&r, &mut z);
            let mut p = z.clone();
            let mut rz = dot(&r, &z);
            let mut q = vec![0.0; n];
            for _ in 0..max_iter {
                l0.apply(&self.unit, &p, &mut q);
                let pq = dot(&p, &q);
                if pq <= 0.0 || !pq.is_finite() {
                    break;
                }
                let alpha = rz / pq;
                for i in 0..n {
                    x[i] += alpha * p[i];
                    r[i] -= alpha * q[i];
                }
                let rel = dot(&r, &r).sqrt() / bnorm;
                hist.push(rel);
                if rel < tol {
                    converged = true;
                    break;
                }
                self.precondition(&r, &mut z);
                let rz_new = dot(&r, &z);
                let beta = rz_new / rz;
                rz = rz_new;
                for i in 0..n {
                    p[i] = z[i] + beta * p[i];
                }
            }
        }
        let mut u = vec![0.0; 3 * self.node_map.len()];
        for (c, &gn) in l0.gnode.iter().enumerate() {
            for d in 0..3 {
                u[3 * gn as usize + d] = x[3 * c + d];
            }
        }
        (u, hist, converged)
    }
}

/// Diagonal, free-DOF cleanup and a power-iteration estimate of λmax(D⁻¹K).
fn finish_level(l: &mut Level, unit: &Mat24) {
    let dg = l.diag(unit);
    let maxd = dg.iter().cloned().fold(0.0, f64::max);
    for (f, d) in l.free.iter_mut().zip(dg.iter()) {
        if *d <= 1e-12 * maxd {
            *f = false;
        }
    }
    l.inv_diag = dg.iter().zip(l.free.iter()).map(|(d, f)| if *f { 1.0 / d } else { 0.0 }).collect();
    let n = dg.len();
    let mut rng = crate::math::Rng::new(11);
    let mut v: Vec<f64> = (0..n).map(|i| if l.free[i] { rng.uniform() - 0.5 } else { 0.0 }).collect();
    let mut w = vec![0.0; n];
    let mut lam: f64 = 1.0;
    for _ in 0..12 {
        let nv = dot(&v, &v).sqrt();
        if nv == 0.0 {
            break;
        }
        v.iter_mut().for_each(|x| *x /= nv);
        l.apply(unit, &v, &mut w);
        for i in 0..n {
            w[i] *= l.inv_diag[i];
        }
        lam = dot(&v, &w);
        std::mem::swap(&mut v, &mut w);
    }
    l.lmax = lam.max(1e-6);
}

/// Build the next coarser level with Galerkin element matrices and the
/// fine → coarse transfer.
fn coarsen(f: &Level, unit: &Mat24) -> (Level, Transfer) {
    let cd = [f.dims[0].div_ceil(2), f.dims[1].div_ceil(2), f.dims[2].div_ceil(2)];
    let cgid = |i: usize, j: usize, k: usize| i + (cd[0] + 1) * (j + (cd[1] + 1) * k);
    // Coarse elements.
    let mut cidx: HashMap<usize, usize> = HashMap::new();
    let mut ecoord: Vec<[u32; 3]> = Vec::new();
    for c in &f.ecoord {
        let (i, j, k) = (c[0] as usize / 2, c[1] as usize / 2, c[2] as usize / 2);
        let id = i + cd[0] * (j + cd[1] * k);
        cidx.entry(id).or_insert_with(|| {
            ecoord.push([i as u32, j as u32, k as u32]);
            ecoord.len() - 1
        });
    }
    // Coarse compact nodes.
    let mut cmap: HashMap<usize, u32> = HashMap::new();
    let mut gnode: Vec<u32> = Vec::new();
    let enodes: Vec<[u32; 8]> = ecoord
        .iter()
        .map(|c| {
            let mut o = [0u32; 8];
            for (a, of) in OFFS.iter().enumerate() {
                let g = cgid(c[0] as usize + of[0], c[1] as usize + of[1], c[2] as usize + of[2]);
                o[a] = *cmap.entry(g).or_insert_with(|| {
                    gnode.push(g as u32);
                    (gnode.len() - 1) as u32
                });
            }
            o
        })
        .collect();
    // Interpolation weights W[child pos][fine local node][coarse local node].
    let mut wts = [[[0.0f64; 8]; 8]; 8];
    for (cp, co) in OFFS.iter().enumerate() {
        for (a, ao) in OFFS.iter().enumerate() {
            let fpos = [co[0] + ao[0], co[1] + ao[1], co[2] + ao[2]];
            for (m, mo) in OFFS.iter().enumerate() {
                let mut w = 1.0;
                for d in 0..3 {
                    let dist = (fpos[d] as f64 - 2.0 * mo[d] as f64).abs();
                    w *= (1.0 - dist / 2.0).max(0.0);
                }
                wts[cp][a][m] = w;
            }
        }
    }
    let mut mats: Vec<Mat24> = vec![[0.0; 576]; ecoord.len()];
    let mut t = [0.0f64; 576];
    for (e, c) in f.ecoord.iter().enumerate() {
        let (i, j, k) = (c[0] as usize, c[1] as usize, c[2] as usize);
        let cpos = OFFS
            .iter()
            .position(|o| o[0] == i % 2 && o[1] == j % 2 && o[2] == k % 2)
            .unwrap_or(0);
        let ce = cidx[&(i / 2 + cd[0] * (j / 2 + cd[1] * (k / 2)))];
        let (km, s) = f.mat(e, unit);
        let w = &wts[cpos];
        let nodes = &f.enodes[e];
        let mask: [f64; 24] = std::array::from_fn(|r| if f.free[3 * nodes[r / 3] as usize + r % 3] { 1.0 } else { 0.0 });
        // T = K P̃ (24 × 24): T[r][3m+d] = Σ_a K[r][3a+d] W[a][m] mask[3a+d]
        for r in 0..24 {
            for m in 0..8 {
                for d in 0..3 {
                    let mut acc = 0.0;
                    for a in 0..8 {
                        let wv = w[a][m];
                        if wv != 0.0 {
                            acc += km[r * 24 + 3 * a + d] * wv * mask[3 * a + d];
                        }
                    }
                    t[r * 24 + 3 * m + d] = s * acc;
                }
            }
        }
        let out = &mut mats[ce];
        for m in 0..8 {
            for d in 0..3 {
                let row = 3 * m + d;
                for a in 0..8 {
                    let wv = w[a][m] * mask[3 * a + d];
                    if wv != 0.0 {
                        let fr = 3 * a + d;
                        for col in 0..24 {
                            out[row * 24 + col] += wv * t[fr * 24 + col];
                        }
                    }
                }
            }
        }
    }
    // Transfer: fine compact node → coarse compact nodes.
    let mut off = vec![0u32];
    let mut col = Vec::new();
    let mut wv = Vec::new();
    for &gn in &f.gnode {
        let [i, j, k] = f.gcoord(gn as usize);
        let (ci, wi, ni) = axis_w(i);
        let (cj, wj, nj) = axis_w(j);
        let (ck, wk, nk) = axis_w(k);
        for a in 0..nk {
            for b in 0..nj {
                for c in 0..ni {
                    if let Some(&m) = cmap.get(&cgid(ci[c], cj[b], ck[a])) {
                        col.push(m);
                        wv.push(wi[c] * wj[b] * wk[a]);
                    }
                }
            }
        }
        off.push(col.len() as u32);
    }
    let nd = 3 * gnode.len();
    let mut lv = Level {
        dims: cd,
        ecoord,
        enodes,
        gnode,
        mats: Mats::Explicit(mats),
        free: vec![true; nd],
        inv_diag: vec![],
        lmax: 1.0,
    };
    finish_level(&mut lv, unit);
    (lv, Transfer { off, col, w: wv })
}

fn factor(l: &Level, unit: &Mat24) -> Coarse {
    let nd = l.ndof();
    let mut map = vec![usize::MAX; nd];
    let mut n = 0;
    for (i, f) in l.free.iter().enumerate() {
        if *f {
            map[i] = n;
            n += 1;
        }
    }
    let mut a = vec![0.0; n * n];
    for (e, nodes) in l.enodes.iter().enumerate() {
        let (k, s) = l.mat(e, unit);
        for r in 0..24 {
            let gr = map[3 * nodes[r / 3] as usize + r % 3];
            if gr == usize::MAX {
                continue;
            }
            for c in 0..24 {
                let gc = map[3 * nodes[c / 3] as usize + c % 3];
                if gc != usize::MAX {
                    a[gr * n + gc] += s * k[r * 24 + c];
                }
            }
        }
    }
    // Cholesky, lower triangle in place; tiny regularisation guards
    // against near-mechanisms on very coarse levels.
    let maxd = (0..n).map(|i| a[i * n + i]).fold(0.0, f64::max);
    for i in 0..n {
        a[i * n + i] += 1e-10 * maxd;
    }
    for j in 0..n {
        let d = a[j * n + j] - dot(&a[j * n..j * n + j], &a[j * n..j * n + j]);
        let d = if d > 1e-14 * maxd { d.sqrt() } else { (1e-7 * maxd).sqrt() };
        a[j * n + j] = d;
        for i in j + 1..n {
            let s = a[i * n + j] - dot(&a[i * n..i * n + j], &a[j * n..j * n + j]);
            a[i * n + j] = s / d;
        }
    }
    Coarse { map, n, l: a }
}

// ---------------------------------------------------------------- results

/// Nodal (averaged) stress results on grid nodes.
#[derive(Clone, Debug, Default)]
pub struct Stresses {
    /// Averaged von Mises stress per node (0 for unattached nodes), MPa.
    pub node_vm: Vec<f64>,
    /// Averaged normal stress along the build (layer-normal) direction, MPa.
    pub node_sbuild: Vec<f64>,
    /// Von Mises stress at each element centre, MPa (per grid element; 0 = void).
    /// The centre value is the stress averaged over the voxel — bounded at
    /// sharp re-entrant corners where the nodal (corner) value is singular.
    pub elem_vm: Vec<f64>,
    /// Normal stress along the build direction at each element centre, MPa.
    pub elem_sbuild: Vec<f64>,
    /// Maximum von Mises of the averaged nodal field, MPa.
    pub max_vm: f64,
    pub max_node: usize,
}

/// Recover stresses: corner stresses per element (including the condensed
/// incompatible modes), averaged at shared nodes. The material stress uses
/// the full modulus E (a partial voxel is a thinner layer of real material
/// with the same strain).
pub fn stresses(g: &Grid, hex: &UnitHex, e_mpa: f64, u: &[f64], build_dir: [f64; 3]) -> Stresses {
    let nn = g.nnodes();
    let mut acc = vec![[0.0f64; 6]; nn];
    let mut cnt = vec![0u16; nn];
    let mut elem_vm = vec![0.0; g.solid.len()];
    let mut elem_sbuild = vec![0.0; g.solid.len()];
    let scale = e_mpa / g.h;
    let mut ue = [0.0; 24];
    for (e, s) in g.solid.iter().enumerate() {
        if !*s {
            continue;
        }
        let nodes = g.elem_nodes(e);
        for (a, &n) in nodes.iter().enumerate() {
            for d in 0..3 {
                ue[3 * a + d] = u[3 * n as usize + d];
            }
        }
        for (a, &n) in nodes.iter().enumerate() {
            let sm = &hex.s_corner[a];
            let n = n as usize;
            for r in 0..6 {
                let v: f64 = (0..24).map(|c| sm[r][c] * ue[c]).sum();
                acc[n][r] += scale * v;
            }
            cnt[n] += 1;
        }
        let mut sc = [0.0; 6];
        for (r, v) in sc.iter_mut().enumerate() {
            *v = scale * (0..24).map(|c| hex.s_center[r][c] * ue[c]).sum::<f64>();
        }
        elem_vm[e] = von_mises(&sc);
        elem_sbuild[e] = normal_stress(&sc, build_dir);
    }
    let mut node_vm = vec![0.0; nn];
    let mut node_sbuild = vec![0.0; nn];
    let mut max_vm = 0.0;
    let mut max_node = 0;
    for n in 0..nn {
        if cnt[n] == 0 {
            continue;
        }
        let k = 1.0 / cnt[n] as f64;
        let s: [f64; 6] = std::array::from_fn(|r| acc[n][r] * k);
        node_vm[n] = von_mises(&s);
        node_sbuild[n] = normal_stress(&s, build_dir);
        if node_vm[n] > max_vm {
            max_vm = node_vm[n];
            max_node = n;
        }
    }
    Stresses {
        node_vm,
        node_sbuild,
        elem_vm,
        elem_sbuild,
        max_vm,
        max_node,
    }
}

/// Solve a voxel model end to end.
pub struct Solved {
    pub u: Vec<f64>,
    pub history: Vec<f64>,
    pub converged: bool,
    pub levels: usize,
    pub free_dofs: usize,
}

pub fn solve(g: &Grid, bcs: &Bcs, opts: &SolveOpts, hex: &UnitHex) -> Solved {
    let op = Operator::new(g, &bcs.fixed, opts, hex);
    let (u, history, converged) = op.pcg(&bcs.force, opts.tol, opts.max_iter);
    Solved {
        u,
        history,
        converged,
        levels: op.levels(),
        free_dofs: op.free_dofs(),
    }
}

/// Solve several load vectors with one operator (one multigrid setup).
pub fn solve_many(g: &Grid, fixed: &[u8], forces: &[&[f64]], opts: &SolveOpts, hex: &UnitHex) -> Vec<Solved> {
    let op = Operator::new(g, fixed, opts, hex);
    forces
        .iter()
        .map(|f| {
            let (u, history, converged) = op.pcg(f, opts.tol, opts.max_iter);
            Solved {
                u,
                history,
                converged,
                levels: op.levels(),
                free_dofs: op.free_dofs(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cantilever bar along x: length L, width b (y), height t (z); clamped at
    /// x = 0, tip load P in −z on the last voxel layer.
    fn cantilever(l: f64, b: f64, t: f64, h: f64, inc: bool, pc: Precond) -> (Grid, Solved, f64) {
        let (nx, ny, nz) = ((l / h).round() as usize, (b / h).round() as usize, (t / h).round() as usize);
        let mut g = Grid::new(nx, ny, nz, h, [0.0; 3]);
        g.fill_all();
        let mut bcs = Bcs::new(&g);
        // Clamp the x = 0 face nodes.
        for k in 0..=nz {
            for j in 0..=ny {
                bcs.fixed[g.nid(0, j, k)] = 7;
            }
        }
        let p = 10.0;
        // Tip load spread equally over the end-face nodes.
        let n = (ny + 1) * (nz + 1);
        for k in 0..=nz {
            for j in 0..=ny {
                bcs.force[3 * g.nid(nx, j, k) + 2] = -p / n as f64;
            }
        }
        let opts = SolveOpts {
            e_mpa: 2000.0,
            nu: 0.3,
            incompatible: inc,
            precond: pc,
            tol: 1e-8,
            max_iter: 5000,
        };
        let hex = UnitHex::new(opts.nu, inc);
        let s = solve(&g, &bcs, &opts, &hex);
        // Mean tip deflection.
        let mut tip = 0.0;
        for k in 0..=nz {
            for j in 0..=ny {
                tip += s.u[3 * g.nid(nx, j, k) + 2];
            }
        }
        (g, s, -tip / n as f64)
    }

    #[test]
    fn cantilever_matches_euler_bernoulli() {
        let (l, b, t) = (40.0, 4.0, 4.0);
        let e = 2000.0;
        let i = b * t * t * t / 12.0;
        let exact = 10.0 * l * l * l / (3.0 * e * i);
        // Timoshenko shear correction (κ = 5/6) — a few tenths of a percent here.
        let g_mod = e / (2.0 * 1.3);
        let shear = 10.0 * l / (5.0 / 6.0 * g_mod * b * t);
        let (_, s, tip) = cantilever(l, b, t, 1.0, true, Precond::Multigrid);
        assert!(s.converged, "MG-PCG did not converge");
        let err = (tip - (exact + shear)) / (exact + shear);
        assert!(
            err.abs() < 0.04,
            "incompatible-mode tip {tip:.4} vs {:.4} ({:.1} %)",
            exact + shear,
            err * 100.0
        );
        assert!(s.history.len() < 60, "multigrid iterations {}", s.history.len());
        // With only two voxels through the thickness plain trilinear bricks
        // lock in bending (far too stiff) while the incompatible-mode brick
        // stays accurate.
        let (_, _, tip_h8) = cantilever(l, b, t, 2.0, false, Precond::Multigrid);
        let (_, _, tip_h8i) = cantilever(l, b, t, 2.0, true, Precond::Multigrid);
        assert!(tip_h8 < 0.92 * exact, "H8 {tip_h8} vs exact {exact}");
        assert!(((tip_h8i - exact - shear) / exact).abs() < 0.05, "H8i coarse {tip_h8i} vs {exact}");
    }

    #[test]
    fn jacobi_and_multigrid_agree() {
        let (_, a, tip_a) = cantilever(16.0, 2.0, 2.0, 1.0, true, Precond::Jacobi);
        let (_, b, tip_b) = cantilever(16.0, 2.0, 2.0, 1.0, true, Precond::Multigrid);
        assert!(a.converged && b.converged);
        assert!((tip_a - tip_b).abs() < 1e-5 * tip_b.abs());
        assert!(
            b.history.len() < a.history.len(),
            "MG {} vs Jacobi {}",
            b.history.len(),
            a.history.len()
        );
    }

    #[test]
    fn bending_stress_matches_beam_theory() {
        let (l, b, t) = (40.0, 4.0, 4.0);
        let (g, s, _) = cantilever(l, b, t, 1.0, true, Precond::Multigrid);
        let hex = UnitHex::new(0.3, true);
        let st = stresses(&g, &hex, 2000.0, &s.u, [0.0, 0.0, 1.0]);
        // Surface node at mid-span x = 20: σ = M c / I with M = P (L − x).
        let i = b * t * t * t / 12.0;
        let exact = 10.0 * (l - 20.0) * (t / 2.0) / i;
        let n = g.nid(20, 2, 4); // top surface, centre of width
        let rel = (st.node_vm[n] - exact) / exact;
        assert!(rel.abs() < 0.05, "mid-span σ {:.3} vs {:.3}", st.node_vm[n], exact);
    }

    #[test]
    fn uniaxial_bar_stress_is_f_over_a() {
        // Bar 20 × 3 × 3, x-rollers at x = 0 (plus a pinned corner), pulled with F.
        let (nx, ny, nz) = (20, 3, 3);
        let mut g = Grid::new(nx, ny, nz, 1.0, [0.0; 3]);
        g.fill_all();
        let mut bcs = Bcs::new(&g);
        for k in 0..=nz {
            for j in 0..=ny {
                bcs.fixed[g.nid(0, j, k)] |= 1;
            }
        }
        bcs.fixed[g.nid(0, 0, 0)] |= 7;
        bcs.fixed[g.nid(0, ny, 0)] |= 4;
        bcs.fixed[g.nid(0, 0, nz)] |= 2;
        let f = 90.0;
        // Consistent nodal loads for a uniform traction on a 3 × 3 face of unit
        // squares: corner ¼, edge ½, interior 1 (× traction area per square).
        let q = f / 9.0;
        for k in 0..=nz {
            for j in 0..=ny {
                let wj = if j == 0 || j == ny { 0.5 } else { 1.0 };
                let wk = if k == 0 || k == nz { 0.5 } else { 1.0 };
                bcs.force[3 * g.nid(nx, j, k)] = q * wj * wk;
            }
        }
        let opts = SolveOpts {
            e_mpa: 2000.0,
            nu: 0.35,
            tol: 1e-10,
            ..Default::default()
        };
        let hex = UnitHex::new(opts.nu, true);
        let s = solve(&g, &bcs, &opts, &hex);
        assert!(s.converged);
        let st = stresses(&g, &hex, opts.e_mpa, &s.u, [1.0, 0.0, 0.0]);
        let exact = f / 9.0;
        for n in [g.nid(10, 1, 1), g.nid(5, 0, 3), g.nid(15, 3, 2)] {
            assert!((st.node_vm[n] - exact).abs() < 1e-6 * exact, "σ {} vs {exact}", st.node_vm[n]);
            assert!((st.node_sbuild[n] - exact).abs() < 1e-6 * exact);
        }
        // Elongation δ = F L / (E A).
        let dl = s.u[3 * g.nid(nx, 1, 1)];
        let exact_dl = f * 20.0 / (2000.0 * 9.0);
        assert!((dl - exact_dl).abs() < 1e-6 * exact_dl, "{dl} vs {exact_dl}");
    }
}
