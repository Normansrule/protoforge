//! Small numerical toolkit shared by every module: complex numbers, dense
//! linear solves (LU with partial pivoting), polynomial roots and grids.
//!
//! Kept dependency-free so the engine compiles to WebAssembly. For large
//! problems a real project would reach for `nalgebra`/`faer` (dense) or a
//! sparse LU such as KLU (what ngspice and Xyce use).

use serde::{Deserialize, Serialize};
use std::ops::{Add, Div, Mul, Neg, Sub};

pub const PI: f64 = std::f64::consts::PI;

/// A complex number `re + j·im`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct C64 {
    pub re: f64,
    pub im: f64,
}

impl C64 {
    pub const ZERO: C64 = C64 { re: 0.0, im: 0.0 };
    pub const ONE: C64 = C64 { re: 1.0, im: 0.0 };
    pub const J: C64 = C64 { re: 0.0, im: 1.0 };
    pub fn new(re: f64, im: f64) -> Self {
        C64 { re, im }
    }
    pub fn real(re: f64) -> Self {
        C64 { re, im: 0.0 }
    }
    pub fn from_polar(r: f64, th: f64) -> Self {
        C64 { re: r * th.cos(), im: r * th.sin() }
    }
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }
    pub fn conj(self) -> Self {
        C64 { re: self.re, im: -self.im }
    }
    pub fn norm_sqr(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
    pub fn scale(self, k: f64) -> Self {
        C64 { re: self.re * k, im: self.im * k }
    }
    pub fn exp(self) -> Self {
        C64::from_polar(self.re.exp(), self.im)
    }
    pub fn inv(self) -> Self {
        let d = self.norm_sqr();
        C64 { re: self.re / d, im: -self.im / d }
    }
    pub fn db(self) -> f64 {
        20.0 * self.abs().max(1e-300).log10()
    }
    pub fn phase_deg(self) -> f64 {
        self.arg().to_degrees()
    }
}

impl Add for C64 {
    type Output = C64;
    fn add(self, o: C64) -> C64 {
        C64 { re: self.re + o.re, im: self.im + o.im }
    }
}
impl Sub for C64 {
    type Output = C64;
    fn sub(self, o: C64) -> C64 {
        C64 { re: self.re - o.re, im: self.im - o.im }
    }
}
impl Mul for C64 {
    type Output = C64;
    fn mul(self, o: C64) -> C64 {
        C64 { re: self.re * o.re - self.im * o.im, im: self.re * o.im + self.im * o.re }
    }
}
impl Div for C64 {
    type Output = C64;
    fn div(self, o: C64) -> C64 {
        self * o.inv()
    }
}
impl Neg for C64 {
    type Output = C64;
    fn neg(self) -> C64 {
        C64 { re: -self.re, im: -self.im }
    }
}
impl Add<f64> for C64 {
    type Output = C64;
    fn add(self, o: f64) -> C64 {
        C64 { re: self.re + o, im: self.im }
    }
}
impl Mul<f64> for C64 {
    type Output = C64;
    fn mul(self, o: f64) -> C64 {
        self.scale(o)
    }
}

/// Numeric type usable by the generic LU solver.
pub trait Field:
    Copy + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Div<Output = Self> + Neg<Output = Self>
{
    fn zero() -> Self;
    fn magnitude(self) -> f64;
}
impl Field for f64 {
    fn zero() -> Self {
        0.0
    }
    fn magnitude(self) -> f64 {
        self.abs()
    }
}
impl Field for C64 {
    fn zero() -> Self {
        C64::ZERO
    }
    fn magnitude(self) -> f64 {
        self.abs()
    }
}

/// Solve `A x = b` in place with Gaussian elimination + partial pivoting.
/// `a` is row-major `n×n`. Returns `None` if the matrix is singular.
pub fn solve<T: Field>(mut a: Vec<Vec<T>>, mut b: Vec<T>) -> Option<Vec<T>> {
    let n = b.len();
    for k in 0..n {
        // pivot
        let mut p = k;
        let mut best = a[k][k].magnitude();
        for (i, row) in a.iter().enumerate().skip(k + 1) {
            let m = row[k].magnitude();
            if m > best {
                best = m;
                p = i;
            }
        }
        if best < 1e-300 {
            return None;
        }
        if p != k {
            a.swap(p, k);
            b.swap(p, k);
        }
        let pivot = a[k][k];
        for i in (k + 1)..n {
            let f = a[i][k] / pivot;
            if f.magnitude() == 0.0 {
                continue;
            }
            for j in k..n {
                let v = a[k][j];
                a[i][j] = a[i][j] - f * v;
            }
            let bk = b[k];
            b[i] = b[i] - f * bk;
        }
    }
    let mut x = vec![T::zero(); n];
    for i in (0..n).rev() {
        let mut s = b[i];
        for j in (i + 1)..n {
            s = s - a[i][j] * x[j];
        }
        x[i] = s / a[i][i];
    }
    Some(x)
}

/// Roots of a real polynomial given highest-order-first coefficients,
/// using the Durand–Kerner (Weierstrass) simultaneous iteration.
pub fn poly_roots(coeffs: &[f64]) -> Vec<C64> {
    let mut c: Vec<f64> = coeffs.to_vec();
    while c.len() > 1 && c[0].abs() < 1e-300 {
        c.remove(0);
    }
    let n = c.len().saturating_sub(1);
    if n == 0 {
        return vec![];
    }
    let lead = c[0];
    let a: Vec<f64> = c.iter().map(|v| v / lead).collect();
    let eval = |z: C64| -> C64 {
        let mut acc = C64::ZERO;
        for &ai in &a {
            acc = acc * z + ai;
        }
        acc
    };
    // Cauchy bound for initial radius
    let radius = 1.0 + a.iter().skip(1).fold(0.0f64, |m, v| m.max(v.abs()));
    let mut roots: Vec<C64> = (0..n)
        .map(|k| C64::from_polar(radius * 0.9, 2.0 * PI * k as f64 / n as f64 + 0.4))
        .collect();
    for _ in 0..500 {
        let mut max_delta: f64 = 0.0;
        for i in 0..n {
            let mut denom = C64::ONE;
            for j in 0..n {
                if i != j {
                    denom = denom * (roots[i] - roots[j]);
                }
            }
            if denom.abs() < 1e-300 {
                denom = C64::new(1e-12, 1e-12);
            }
            let delta = eval(roots[i]) / denom;
            roots[i] = roots[i] - delta;
            max_delta = max_delta.max(delta.abs());
        }
        if max_delta < 1e-13 {
            break;
        }
    }
    for r in roots.iter_mut() {
        if r.im.abs() < 1e-9 * (1.0 + r.re.abs()) {
            r.im = 0.0;
        }
    }
    roots
}

/// Evaluate a real polynomial (highest order first) at a complex point.
pub fn poly_eval(coeffs: &[f64], z: C64) -> C64 {
    coeffs.iter().fold(C64::ZERO, |acc, &c| acc * z + c)
}

/// Multiply two polynomials (highest order first).
pub fn poly_mul(a: &[f64], b: &[f64]) -> Vec<f64> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let mut out = vec![0.0; a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] += x * y;
        }
    }
    out
}

/// Add two polynomials (highest order first), aligning the constant terms.
pub fn poly_add(a: &[f64], b: &[f64]) -> Vec<f64> {
    let n = a.len().max(b.len());
    let mut out = vec![0.0; n];
    for (i, v) in a.iter().enumerate() {
        out[n - a.len() + i] += v;
    }
    for (i, v) in b.iter().enumerate() {
        out[n - b.len() + i] += v;
    }
    out
}

/// `n` evenly spaced points from `a` to `b` inclusive.
pub fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    if n <= 1 {
        return vec![a];
    }
    (0..n).map(|i| a + (b - a) * i as f64 / (n - 1) as f64).collect()
}

/// `n` log-spaced points from `a` to `b` inclusive (both > 0).
pub fn logspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    linspace(a.log10(), b.log10(), n).into_iter().map(|e| 10f64.powf(e)).collect()
}

/// Linear interpolation of `(xs, ys)` at `x` (xs ascending). Clamps at the ends.
pub fn interp(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    if x <= xs[0] {
        return ys[0];
    }
    if x >= xs[xs.len() - 1] {
        return ys[ys.len() - 1];
    }
    let i = xs.partition_point(|v| *v <= x).max(1);
    let (x0, x1, y0, y1) = (xs[i - 1], xs[i], ys[i - 1], ys[i]);
    y0 + (y1 - y0) * (x - x0) / (x1 - x0)
}

/// Root-mean-square of a slice.
pub fn rms(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    (v.iter().map(|x| x * x).sum::<f64>() / v.len() as f64).sqrt()
}

/// Deterministic pseudo-random generator (xorshift64*), so simulations are
/// reproducible in the browser, the CLI and in tests.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    /// Uniform in [0, 1).
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Standard normal via Box–Muller.
    pub fn normal(&mut self) -> f64 {
        let u1 = self.uniform().max(1e-300);
        let u2 = self.uniform();
        (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos()
    }
    /// Uniform integer in [0, n).
    pub fn below(&mut self, n: usize) -> usize {
        (self.uniform() * n as f64) as usize % n.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_real_system() {
        let a = vec![vec![2.0, 1.0], vec![1.0, 3.0]];
        let x = solve(a, vec![3.0, 5.0]).unwrap();
        assert!((x[0] - 0.8).abs() < 1e-12 && (x[1] - 1.4).abs() < 1e-12);
    }

    #[test]
    fn solves_complex_system() {
        let a = vec![vec![C64::new(1.0, 1.0), C64::ZERO], vec![C64::ZERO, C64::new(0.0, 2.0)]];
        let x = solve(a, vec![C64::new(2.0, 0.0), C64::new(2.0, 0.0)]).unwrap();
        assert!((x[0] - C64::new(1.0, -1.0)).abs() < 1e-12);
        assert!((x[1] - C64::new(0.0, -1.0)).abs() < 1e-12);
    }

    #[test]
    fn finds_polynomial_roots() {
        // (s+1)(s+2)(s^2+2s+5) roots: -1, -2, -1±2j
        let p = poly_mul(&poly_mul(&[1.0, 1.0], &[1.0, 2.0]), &[1.0, 2.0, 5.0]);
        let mut r = poly_roots(&p);
        r.sort_by(|a, b| a.re.partial_cmp(&b.re).unwrap().then(a.im.partial_cmp(&b.im).unwrap()));
        assert!((r[0] - C64::new(-2.0, 0.0)).abs() < 1e-8);
        assert!(r.iter().any(|z| (*z - C64::new(-1.0, 2.0)).abs() < 1e-8));
    }

    #[test]
    fn rng_is_deterministic() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        assert_eq!(a.next_u64(), b.next_u64());
        let m: f64 = (0..20000).map(|_| a.normal()).sum::<f64>() / 20000.0;
        assert!(m.abs() < 0.05);
    }
}
