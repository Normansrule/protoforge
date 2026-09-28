//! 8-node hexahedral (brick) element for a cube of side h.
//!
//! Every voxel of the finite element model is the same cube, so the element
//! stiffness is computed **once** for a unit cube with E = 1 and scaled:
//! `K_e = E · h · K̂(ν)` (stiffness ∝ E·area/length = E·h²/h).
//!
//! ## Trilinear shape functions
//! Node a sits at natural coordinates (ξa, ηa, ζa) ∈ {−1, 1}³ and
//! `N_a = ⅛ (1 + ξ ξa)(1 + η ηa)(1 + ζ ζa)`. With x = h(ξ + 1)/2 the strain
//! matrix is `B = (2/h) B_ξ`, the Jacobian determinant h³/8, and
//! `K = ∫ Bᵀ D B dV = (h/2) Σ_gauss B_ξᵀ D B_ξ` with 2 × 2 × 2 Gauss points
//! (weights 1), exact for this element.
//!
//! ## Incompatible modes (Wilson–Taylor)
//! A plain trilinear brick cannot bend: a pure bending field needs strain
//! that varies linearly *and* zero shear, which the element can only mimic
//! with parasitic shear — **shear locking**, making thin parts far too stiff
//! unless there are many elements through the thickness. Wilson's
//! incompatible modes add three internal "bubble" displacements per
//! direction, `(1 − ξ²), (1 − η²), (1 − ζ²)`, whose 9 amplitudes α are
//! condensed out statically:
//! `K* = K_uu − K_uα K_αα⁻¹ K_αu`, `α = −K_αα⁻¹ K_αu u`.
//! For regular (parallelepiped) elements the modified element passes the
//! patch test and represents pure bending exactly, so one or two voxels
//! through a wall already give good bending stiffness.
//!
//! References: E. L. Wilson, R. L. Taylor, W. P. Doherty, J. Ghaboussi,
//! "Incompatible displacement models", in *Numerical and Computer Methods
//! in Structural Mechanics*, Academic Press, 1973; R. L. Taylor, P. J.
//! Beresford, E. L. Wilson, "A non-conforming element for stress analysis",
//! IJNME 10, 1976; O. C. Zienkiewicz, R. L. Taylor, *The Finite Element
//! Method*, 7th ed.; T. J. R. Hughes, *The Finite Element Method*, ch. 4.

/// Natural coordinates of the 8 nodes (grid offsets are (s + 1) / 2).
pub const NODES: [[f64; 3]; 8] = [
    [-1.0, -1.0, -1.0],
    [1.0, -1.0, -1.0],
    [1.0, 1.0, -1.0],
    [-1.0, 1.0, -1.0],
    [-1.0, -1.0, 1.0],
    [1.0, -1.0, 1.0],
    [1.0, 1.0, 1.0],
    [-1.0, 1.0, 1.0],
];

/// Grid offsets (0/1) of the 8 nodes.
pub const OFFS: [[usize; 3]; 8] = [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0], [0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]];

/// Isotropic elasticity matrix for E = 1 (Voigt order xx, yy, zz, xy, yz, zx
/// with engineering shear strains).
pub fn d_matrix(nu: f64) -> [[f64; 6]; 6] {
    let lam = nu / ((1.0 + nu) * (1.0 - 2.0 * nu));
    let mu = 1.0 / (2.0 * (1.0 + nu));
    let mut d = [[0.0; 6]; 6];
    for (i, row) in d.iter_mut().enumerate().take(3) {
        for (j, v) in row.iter_mut().enumerate().take(3) {
            *v = lam + if i == j { 2.0 * mu } else { 0.0 };
        }
    }
    d[3][3] = mu;
    d[4][4] = mu;
    d[5][5] = mu;
    d
}

/// Derivatives dN_a/dξ at natural point p (8 × 3).
fn dn(p: [f64; 3]) -> [[f64; 3]; 8] {
    let mut out = [[0.0; 3]; 8];
    for (a, n) in NODES.iter().enumerate() {
        let f = [1.0 + p[0] * n[0], 1.0 + p[1] * n[1], 1.0 + p[2] * n[2]];
        out[a] = [0.125 * n[0] * f[1] * f[2], 0.125 * n[1] * f[0] * f[2], 0.125 * n[2] * f[0] * f[1]];
    }
    out
}

/// Strain-displacement rows for one "node" with derivatives g (6 × 3).
fn b_block(g: [f64; 3]) -> [[f64; 3]; 6] {
    [
        [g[0], 0.0, 0.0],
        [0.0, g[1], 0.0],
        [0.0, 0.0, g[2]],
        [g[1], g[0], 0.0],
        [0.0, g[2], g[1]],
        [g[2], 0.0, g[0]],
    ]
}

/// Full B_ξ (6 × 33) at point p: 24 nodal columns followed by 9 incompatible
/// columns (mode m for component d at 24 + 3m + d).
fn b_full(p: [f64; 3]) -> Vec<[f64; 33]> {
    let mut b = vec![[0.0; 33]; 6];
    for (a, g) in dn(p).iter().enumerate() {
        let blk = b_block(*g);
        for r in 0..6 {
            for c in 0..3 {
                b[r][3 * a + c] = blk[r][c];
            }
        }
    }
    // Bubble modes P_m = 1 − s_m²: dP_m/ds_m = −2 s_m.
    for m in 0..3 {
        let mut g = [0.0; 3];
        g[m] = -2.0 * p[m];
        let blk = b_block(g);
        for r in 0..6 {
            for c in 0..3 {
                b[r][24 + 3 * m + c] = blk[r][c];
            }
        }
    }
    b
}

/// Precomputed unit-cube element: condensed stiffness and stress recovery.
#[derive(Clone, Debug)]
pub struct UnitHex {
    pub nu: f64,
    pub incompatible: bool,
    /// 24 × 24 stiffness for E = 1, h = 1 (row-major).
    pub k: Vec<f64>,
    /// Stress-recovery matrices at the 8 corners: σ = (E/h) · S_a · u_e (6 × 24 each).
    pub s_corner: Vec<[[f64; 24]; 6]>,
    /// Stress at the element centre.
    pub s_center: [[f64; 24]; 6],
}

fn gauss_points() -> Vec<[f64; 3]> {
    let g = 1.0 / 3f64.sqrt();
    let mut v = Vec::with_capacity(8);
    for &z in &[-g, g] {
        for &y in &[-g, g] {
            for &x in &[-g, g] {
                v.push([x, y, z]);
            }
        }
    }
    v
}

/// Invert a small dense SPD matrix by Gauss–Jordan elimination.
fn invert(a: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = a.len();
    let mut m: Vec<Vec<f64>> = a
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut row = r.clone();
            row.extend((0..n).map(|j| if i == j { 1.0 } else { 0.0 }));
            row
        })
        .collect();
    for c in 0..n {
        let p = (c..n).max_by(|&x, &y| m[x][c].abs().total_cmp(&m[y][c].abs())).unwrap_or(c);
        m.swap(c, p);
        let piv = m[c][c];
        for v in m[c].iter_mut() {
            *v /= piv;
        }
        for r in 0..n {
            if r != c {
                let f = m[r][c];
                if f != 0.0 {
                    let pr = m[c].clone();
                    for (v, q) in m[r].iter_mut().zip(pr.iter()) {
                        *v -= f * q;
                    }
                }
            }
        }
    }
    m.into_iter().map(|r| r[n..].to_vec()).collect()
}

impl UnitHex {
    pub fn new(nu: f64, incompatible: bool) -> UnitHex {
        let d = d_matrix(nu);
        let nd = if incompatible { 33 } else { 24 };
        // Full stiffness (h = 1, E = 1): 0.5 Σ Bξᵀ D Bξ.
        let mut kf = vec![vec![0.0; nd]; nd];
        for p in gauss_points() {
            let b = b_full(p);
            // DB (6 × nd)
            let mut db = vec![vec![0.0; nd]; 6];
            for i in 0..6 {
                for j in 0..nd {
                    db[i][j] = (0..6).map(|k| d[i][k] * b[k][j]).sum();
                }
            }
            for i in 0..nd {
                for j in 0..nd {
                    let s: f64 = (0..6).map(|k| b[k][i] * db[k][j]).sum();
                    kf[i][j] += 0.5 * s;
                }
            }
        }
        // Static condensation of the bubble modes.
        let mut g = vec![vec![0.0; 24]; 9]; // α = G u
        let mut k = vec![0.0; 576];
        if incompatible {
            let kaa: Vec<Vec<f64>> = (24..33).map(|i| kf[i][24..33].to_vec()).collect();
            let kaa_inv = invert(&kaa);
            for i in 0..9 {
                for j in 0..24 {
                    g[i][j] = -(0..9).map(|l| kaa_inv[i][l] * kf[24 + l][j]).sum::<f64>();
                }
            }
            for i in 0..24 {
                for j in 0..24 {
                    let corr: f64 = (0..9).map(|l| kf[i][24 + l] * g[l][j]).sum();
                    k[i * 24 + j] = kf[i][j] + corr;
                }
            }
        } else {
            for i in 0..24 {
                for j in 0..24 {
                    k[i * 24 + j] = kf[i][j];
                }
            }
        }
        // Symmetrise round-off.
        for i in 0..24 {
            for j in 0..i {
                let v = 0.5 * (k[i * 24 + j] + k[j * 24 + i]);
                k[i * 24 + j] = v;
                k[j * 24 + i] = v;
            }
        }
        let stress_at = |p: [f64; 3]| -> [[f64; 24]; 6] {
            let b = b_full(p);
            // Effective B (6 × 24) = Bu + Bα G, times 2 (dξ/dx for h = 1).
            let mut be = [[0.0; 24]; 6];
            for r in 0..6 {
                for c in 0..24 {
                    let mut v = b[r][c];
                    if incompatible {
                        v += (0..9).map(|l| b[r][24 + l] * g[l][c]).sum::<f64>();
                    }
                    be[r][c] = 2.0 * v;
                }
            }
            let mut s = [[0.0; 24]; 6];
            for r in 0..6 {
                for c in 0..24 {
                    s[r][c] = (0..6).map(|q| d[r][q] * be[q][c]).sum();
                }
            }
            s
        };
        let s_corner = NODES.iter().map(|n| stress_at(*n)).collect();
        let s_center = stress_at([0.0; 3]);
        UnitHex {
            nu,
            incompatible,
            k,
            s_corner,
            s_center,
        }
    }

    /// Diagonal of the unit stiffness.
    pub fn diag(&self) -> [f64; 24] {
        let mut d = [0.0; 24];
        for (i, v) in d.iter_mut().enumerate() {
            *v = self.k[i * 24 + i];
        }
        d
    }
}

/// Von Mises equivalent stress of a Voigt stress vector.
pub fn von_mises(s: &[f64; 6]) -> f64 {
    let (a, b, c) = (s[0] - s[1], s[1] - s[2], s[2] - s[0]);
    (0.5 * (a * a + b * b + c * c) + 3.0 * (s[3] * s[3] + s[4] * s[4] + s[5] * s[5])).sqrt()
}

/// Normal stress along the unit direction n: nᵀ σ n.
pub fn normal_stress(s: &[f64; 6], n: [f64; 3]) -> f64 {
    s[0] * n[0] * n[0] + s[1] * n[1] * n[1] + s[2] * n[2] * n[2] + 2.0 * (s[3] * n[0] * n[1] + s[4] * n[1] * n[2] + s[5] * n[2] * n[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rigid_modes() -> Vec<[f64; 24]> {
        let mut out = Vec::new();
        for d in 0..3 {
            let mut u = [0.0; 24];
            for a in 0..8 {
                u[3 * a + d] = 1.0;
            }
            out.push(u);
        }
        // Rotation about z: u = −y, v = x.
        let mut u = [0.0; 24];
        for (a, n) in NODES.iter().enumerate() {
            u[3 * a] = -n[1];
            u[3 * a + 1] = n[0];
        }
        out.push(u);
        out
    }

    #[test]
    fn stiffness_is_symmetric_with_rigid_null_space() {
        for inc in [false, true] {
            let e = UnitHex::new(0.3, inc);
            for i in 0..24 {
                for j in 0..24 {
                    assert!((e.k[i * 24 + j] - e.k[j * 24 + i]).abs() < 1e-12);
                }
                assert!(e.k[i * 24 + i] > 0.0);
            }
            for u in rigid_modes() {
                for i in 0..24 {
                    let f: f64 = (0..24).map(|j| e.k[i * 24 + j] * u[j]).sum();
                    assert!(f.abs() < 1e-10, "rigid mode force {f}");
                }
            }
        }
    }

    #[test]
    fn uniform_strain_patch() {
        // u = ε x: every corner stress equals E ε (σxx) for a free bar (ν = 0).
        let e = UnitHex::new(0.0, true);
        let mut u = [0.0; 24];
        for (a, o) in OFFS.iter().enumerate() {
            u[3 * a] = 0.01 * o[0] as f64;
        }
        for s in &e.s_corner {
            let sxx: f64 = (0..24).map(|c| s[0][c] * u[c]).sum();
            assert!((sxx - 0.01).abs() < 1e-12, "{sxx}");
        }
    }

    #[test]
    fn von_mises_uniaxial_and_shear() {
        assert!((von_mises(&[100.0, 0.0, 0.0, 0.0, 0.0, 0.0]) - 100.0).abs() < 1e-12);
        assert!((von_mises(&[0.0, 0.0, 0.0, 10.0, 0.0, 0.0]) - 10.0 * 3f64.sqrt()).abs() < 1e-12);
        assert!((normal_stress(&[1.0, 2.0, 3.0, 0.0, 0.0, 0.0], [0.0, 0.0, 1.0]) - 3.0).abs() < 1e-12);
    }
}
