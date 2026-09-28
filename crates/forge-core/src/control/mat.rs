//! Tiny dense-matrix helpers (row-major `Vec<Vec<f64>>`) for the control,
//! robotics and RL modules: products, inverse, matrix exponential and the
//! characteristic polynomial. Sizes here are ≤ 8×8, so clarity beats speed.

use crate::math::{poly_roots, C64};

pub type Mat = Vec<Vec<f64>>;

pub fn zeros(r: usize, c: usize) -> Mat {
    vec![vec![0.0; c]; r]
}

pub fn eye(n: usize) -> Mat {
    let mut m = zeros(n, n);
    for (i, row) in m.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    m
}

pub fn rows(a: &Mat) -> usize {
    a.len()
}

pub fn cols(a: &Mat) -> usize {
    a.first().map_or(0, |r| r.len())
}

pub fn mul(a: &Mat, b: &Mat) -> Mat {
    let (n, k, m) = (rows(a), cols(a), cols(b));
    let mut out = zeros(n, m);
    for i in 0..n {
        for p in 0..k {
            let v = a[i][p];
            if v == 0.0 {
                continue;
            }
            for j in 0..m {
                out[i][j] += v * b[p][j];
            }
        }
    }
    out
}

pub fn add(a: &Mat, b: &Mat) -> Mat {
    a.iter()
        .zip(b)
        .map(|(x, y)| x.iter().zip(y).map(|(p, q)| p + q).collect())
        .collect()
}

pub fn sub(a: &Mat, b: &Mat) -> Mat {
    a.iter()
        .zip(b)
        .map(|(x, y)| x.iter().zip(y).map(|(p, q)| p - q).collect())
        .collect()
}

pub fn scale(a: &Mat, k: f64) -> Mat {
    a.iter().map(|r| r.iter().map(|v| v * k).collect()).collect()
}

pub fn transpose(a: &Mat) -> Mat {
    let (n, m) = (rows(a), cols(a));
    let mut out = zeros(m, n);
    for i in 0..n {
        for j in 0..m {
            out[j][i] = a[i][j];
        }
    }
    out
}

pub fn mat_vec(a: &Mat, x: &[f64]) -> Vec<f64> {
    a.iter().map(|r| r.iter().zip(x).map(|(p, q)| p * q).sum()).collect()
}

/// Inverse by Gauss–Jordan elimination with partial pivoting.
pub fn inv(a: &Mat) -> Option<Mat> {
    let n = rows(a);
    let mut m: Mat = a.iter().zip(eye(n)).map(|(r, e)| r.iter().copied().chain(e).collect()).collect();
    for k in 0..n {
        let p = (k..n).max_by(|&i, &j| m[i][k].abs().partial_cmp(&m[j][k].abs()).unwrap())?;
        if m[p][k].abs() < 1e-300 {
            return None;
        }
        m.swap(p, k);
        let piv = m[k][k];
        for v in m[k].iter_mut() {
            *v /= piv;
        }
        for i in 0..n {
            if i != k {
                let f = m[i][k];
                if f != 0.0 {
                    for j in 0..2 * n {
                        let t = m[k][j];
                        m[i][j] -= f * t;
                    }
                }
            }
        }
    }
    Some(m.into_iter().map(|r| r[n..].to_vec()).collect())
}

pub fn norm_inf(a: &Mat) -> f64 {
    a.iter().map(|r| r.iter().map(|v| v.abs()).sum::<f64>()).fold(0.0, f64::max)
}

/// Matrix exponential by scaling and squaring with a 12-term Taylor series
/// (Moler & Van Loan, "Nineteen dubious ways to compute the exponential of a
/// matrix", SIAM Review 1978/2003 — method 3). Accurate for the small,
/// well-scaled matrices used here.
pub fn expm(a: &Mat) -> Mat {
    let n = rows(a);
    let nrm = norm_inf(a);
    let s = if nrm > 0.5 { (nrm / 0.5).log2().ceil() as i32 } else { 0 };
    let a_s = scale(a, 0.5f64.powi(s));
    let mut term = eye(n);
    let mut sum = eye(n);
    for k in 1..=12 {
        term = scale(&mul(&term, &a_s), 1.0 / k as f64);
        sum = add(&sum, &term);
    }
    for _ in 0..s {
        sum = mul(&sum, &sum);
    }
    sum
}

/// Zero-order-hold discretisation of `ẋ = A x + B u` with step `dt`, via the
/// block exponential exp([[A, B], [0, 0]]·dt) = [[Ad, Bd], [0, I]].
pub fn c2d(a: &Mat, b: &Mat, dt: f64) -> (Mat, Mat) {
    let n = rows(a);
    let m = cols(b);
    let mut big = zeros(n + m, n + m);
    for i in 0..n {
        for j in 0..n {
            big[i][j] = a[i][j] * dt;
        }
        for j in 0..m {
            big[i][n + j] = b[i][j] * dt;
        }
    }
    let e = expm(&big);
    let ad = (0..n).map(|i| e[i][..n].to_vec()).collect();
    let bd = (0..n).map(|i| e[i][n..].to_vec()).collect();
    (ad, bd)
}

/// Characteristic polynomial det(sI − A) (highest power first) by the
/// Faddeev–LeVerrier recursion.
pub fn charpoly(a: &Mat) -> Vec<f64> {
    let n = rows(a);
    let mut c = vec![1.0];
    let mut m = zeros(n, n);
    let mut ck = 1.0;
    for k in 1..=n {
        // M_k = A M_{k-1} + c_{k-1} I
        m = mul(a, &m);
        for (i, row) in m.iter_mut().enumerate() {
            row[i] += ck;
        }
        let am = mul(a, &m);
        let tr: f64 = (0..n).map(|i| am[i][i]).sum();
        ck = -tr / k as f64;
        c.push(ck);
    }
    c
}

pub fn eig(a: &Mat) -> Vec<C64> {
    poly_roots(&charpoly(a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_and_expm() {
        let a = vec![vec![4.0, 7.0], vec![2.0, 6.0]];
        let ai = inv(&a).unwrap();
        let i = mul(&a, &ai);
        assert!((i[0][0] - 1.0).abs() < 1e-12 && i[0][1].abs() < 1e-12);
        // exp of diag(1, -2)
        let e = expm(&vec![vec![1.0, 0.0], vec![0.0, -2.0]]);
        assert!((e[0][0] - 1f64.exp()).abs() < 1e-10 && (e[1][1] - (-2f64).exp()).abs() < 1e-10);
        // rotation generator
        let r = expm(&vec![vec![0.0, -1.0], vec![1.0, 0.0]]);
        assert!((r[0][0] - 1f64.cos()).abs() < 1e-10 && (r[1][0] - 1f64.sin()).abs() < 1e-10);
    }

    #[test]
    fn charpoly_matches() {
        // companion of s^2 + 3 s + 2
        let a = vec![vec![0.0, 1.0], vec![-2.0, -3.0]];
        let c = charpoly(&a);
        assert!((c[1] - 3.0).abs() < 1e-12 && (c[2] - 2.0).abs() < 1e-12);
    }
}
