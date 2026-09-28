//! Small dense linear algebra for density matrices: Hermitian eigenvalues and
//! matrix functions (via the real symmetric embedding and cyclic Jacobi),
//! Uhlmann fidelity, von Neumann entropy and Wootters' concurrence.
//!
//! A Hermitian `M = A + iB` has the real symmetric embedding
//! `S = [[A, −B], [B, A]]`; each eigenvalue of `M` appears twice in `S`, and
//! `f(S)` is the embedding of `f(M)`, so matrix square roots can be read back
//! from the blocks. Jacobi is slow (O(n³) per sweep) but robust for the
//! repeated eigenvalues that pure and Bell states produce.

use crate::math::C64;

/// Cyclic Jacobi eigen-decomposition of a real symmetric `n×n` matrix
/// (row-major). Returns (eigenvalues, eigenvectors as columns, row-major).
pub fn sym_eig(mut a: Vec<f64>, n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut v = vec![0.0; n * n];
    for i in 0..n {
        v[i * n + i] = 1.0;
    }
    for _sweep in 0..60 {
        let mut off = 0.0;
        for i in 0..n {
            for j in (i + 1)..n {
                off += a[i * n + j] * a[i * n + j];
            }
        }
        if off < 1e-26 {
            break;
        }
        for p in 0..n {
            for q in (p + 1)..n {
                let apq = a[p * n + q];
                if apq.abs() < 1e-300 {
                    continue;
                }
                let app = a[p * n + p];
                let aqq = a[q * n + q];
                let theta = (aqq - app) / (2.0 * apq);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let cs = 1.0 / (t * t + 1.0).sqrt();
                let sn = t * cs;
                for k in 0..n {
                    let akp = a[k * n + p];
                    let akq = a[k * n + q];
                    a[k * n + p] = cs * akp - sn * akq;
                    a[k * n + q] = sn * akp + cs * akq;
                }
                for k in 0..n {
                    let apk = a[p * n + k];
                    let aqk = a[q * n + k];
                    a[p * n + k] = cs * apk - sn * aqk;
                    a[q * n + k] = sn * apk + cs * aqk;
                }
                for k in 0..n {
                    let vkp = v[k * n + p];
                    let vkq = v[k * n + q];
                    v[k * n + p] = cs * vkp - sn * vkq;
                    v[k * n + q] = sn * vkp + cs * vkq;
                }
            }
        }
    }
    ((0..n).map(|i| a[i * n + i]).collect(), v)
}

fn embed(m: &[C64], d: usize) -> Vec<f64> {
    let n = 2 * d;
    let mut s = vec![0.0; n * n];
    for i in 0..d {
        for j in 0..d {
            // Hermitian-symmetrise to kill round-off asymmetry
            let z = (m[i * d + j] + m[j * d + i].conj()).scale(0.5);
            s[i * n + j] = z.re;
            s[(i + d) * n + (j + d)] = z.re;
            s[(i + d) * n + j] = z.im;
            s[i * n + (j + d)] = -z.im;
        }
    }
    s
}

/// Eigenvalues of a Hermitian `d×d` matrix, ascending.
pub fn herm_eigvals(m: &[C64], d: usize) -> Vec<f64> {
    let (mut vals, _) = sym_eig(embed(m, d), 2 * d);
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    vals.iter().step_by(2).copied().collect()
}

/// Apply a real function to the eigenvalues of a Hermitian matrix.
pub fn herm_fn(m: &[C64], d: usize, f: impl Fn(f64) -> f64) -> Vec<C64> {
    let n = 2 * d;
    let (vals, v) = sym_eig(embed(m, d), n);
    let fv: Vec<f64> = vals.iter().map(|&x| f(x)).collect();
    let mut out = vec![C64::ZERO; d * d];
    for i in 0..d {
        for j in 0..d {
            // A' = S'[i][j], B' = S'[i+d][j]
            let (mut a, mut b) = (0.0, 0.0);
            for k in 0..n {
                a += v[i * n + k] * fv[k] * v[j * n + k];
                b += v[(i + d) * n + k] * fv[k] * v[j * n + k];
            }
            out[i * d + j] = C64::new(a, b);
        }
    }
    out
}

pub fn matmul(a: &[C64], b: &[C64], d: usize) -> Vec<C64> {
    let mut out = vec![C64::ZERO; d * d];
    for i in 0..d {
        for k in 0..d {
            let x = a[i * d + k];
            if x.re == 0.0 && x.im == 0.0 {
                continue;
            }
            for j in 0..d {
                out[i * d + j] = out[i * d + j] + x * b[k * d + j];
            }
        }
    }
    out
}

/// Uhlmann fidelity F(ρ,σ) = (Tr √(√ρ σ √ρ))².
pub fn fidelity(rho: &[C64], sigma: &[C64], d: usize) -> f64 {
    let sr = herm_fn(rho, d, |x| x.max(0.0).sqrt());
    let m = matmul(&matmul(&sr, sigma, d), &sr, d);
    let t: f64 = herm_eigvals(&m, d).iter().map(|x| x.max(0.0).sqrt()).sum();
    (t * t).min(1.0)
}

/// von Neumann entropy S(ρ) = −Σ λ log₂ λ, bits.
pub fn entropy(rho: &[C64], d: usize) -> f64 {
    herm_eigvals(rho, d).iter().filter(|&&l| l > 1e-12).map(|&l| -l * l.log2()).sum()
}

/// Wootters concurrence of a two-qubit density matrix (4×4, basis |q1 q0⟩):
/// C = max(0, λ₁ − λ₂ − λ₃ − λ₄) with λᵢ the square roots of the
/// eigenvalues of √ρ ρ̃ √ρ, ρ̃ = (Y⊗Y) ρ* (Y⊗Y).
/// (W. K. Wootters, PRL 80, 2245, 1998.)
pub fn concurrence(rho: &[C64]) -> f64 {
    // (Y⊗Y) is the anti-diagonal matrix with signs [-1, 1, 1, -1] (real)
    let yy = |i: usize, j: usize| -> f64 {
        if i + j != 3 {
            0.0
        } else if i == 0 || i == 3 {
            -1.0
        } else {
            1.0
        }
    };
    let mut tilde = vec![C64::ZERO; 16];
    for i in 0..4 {
        for j in 0..4 {
            let mut s = C64::ZERO;
            for k in 0..4 {
                for l in 0..4 {
                    let w = yy(i, k) * yy(l, j);
                    if w != 0.0 {
                        s = s + rho[k * 4 + l].conj().scale(w);
                    }
                }
            }
            tilde[i * 4 + j] = s;
        }
    }
    let sr = herm_fn(rho, 4, |x| x.max(0.0).sqrt());
    let m = matmul(&matmul(&sr, &tilde, 4), &sr, 4);
    let mut l: Vec<f64> = herm_eigvals(&m, 4).iter().map(|x| x.max(0.0).sqrt()).collect();
    l.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    (l[0] - l[1] - l[2] - l[3]).max(0.0)
}

/// Binary entropy h(p) in bits.
pub fn h2(p: f64) -> f64 {
    if p <= 0.0 || p >= 1.0 {
        0.0
    } else {
        -p * p.log2() - (1.0 - p) * (1.0 - p).log2()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jacobi_diagonalises() {
        let (mut v, _) = sym_eig(vec![2.0, 1.0, 1.0, 2.0], 2);
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((v[0] - 1.0).abs() < 1e-12 && (v[1] - 3.0).abs() < 1e-12);
    }

    #[test]
    fn hermitian_sqrt_squares_back() {
        // ρ = [[0.7, 0.2−0.1i],[0.2+0.1i, 0.3]]
        let m = vec![C64::new(0.7, 0.0), C64::new(0.2, -0.1), C64::new(0.2, 0.1), C64::new(0.3, 0.0)];
        let s = herm_fn(&m, 2, |x| x.sqrt());
        let back = matmul(&s, &s, 2);
        for k in 0..4 {
            assert!((back[k] - m[k]).abs() < 1e-10);
        }
        let ev = herm_eigvals(&m, 2);
        // trace 1, det 0.21 − 0.05 = 0.16
        assert!((ev[0] + ev[1] - 1.0).abs() < 1e-12);
        assert!((ev[0] * ev[1] - 0.16).abs() < 1e-12);
    }

    #[test]
    fn bell_state_concurrence_is_one_and_product_is_zero() {
        let h = 0.5;
        let mut bell = vec![C64::ZERO; 16];
        for (i, j) in [(0, 0), (0, 3), (3, 0), (3, 3)] {
            bell[i * 4 + j] = C64::new(h, 0.0);
        }
        assert!((concurrence(&bell) - 1.0).abs() < 1e-9);
        let mut prod = vec![C64::ZERO; 16];
        prod[0] = C64::ONE;
        assert!(concurrence(&prod) < 1e-9);
        // Werner state p|Φ+⟩⟨Φ+| + (1−p) I/4 has C = max(0, (3p−1)/2)
        let p = 0.8;
        let w: Vec<C64> = (0..16)
            .map(|k| bell[k].scale(p) + if k % 5 == 0 { C64::new((1.0 - p) / 4.0, 0.0) } else { C64::ZERO })
            .collect();
        assert!((concurrence(&w) - (3.0 * p - 1.0) / 2.0).abs() < 1e-9);
    }

    #[test]
    fn fidelity_of_pure_states_is_overlap_squared() {
        let a = [C64::new(1.0, 0.0), C64::ZERO];
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let b = [C64::new(s, 0.0), C64::new(s, 0.0)];
        let dm = |v: &[C64]| (0..4).map(|k| v[k / 2] * v[k % 2].conj()).collect::<Vec<_>>();
        assert!((fidelity(&dm(&a), &dm(&b), 2) - 0.5).abs() < 1e-9);
        assert!((h2(0.11) - 0.4999).abs() < 1e-3);
    }
}
