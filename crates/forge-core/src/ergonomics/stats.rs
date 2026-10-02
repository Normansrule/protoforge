//! Normal-distribution toolkit for anthropometry.
//!
//! * `norm_cdf` uses the complementary error function of W. J. Cody's
//!   rational Chebyshev approximations as tabulated in *Numerical Recipes*
//!   (`erfcc`, fractional error < 1.2 × 10⁻⁷ everywhere) — plenty for
//!   percentiles quoted to 0.1 mm.
//! * `norm_inv` is P. J. Acklam's rational approximation (relative error
//!   1.15 × 10⁻⁹) polished with one Halley step.
//! * A *population* of men and women is a two-component normal **mixture**
//!   `F(x) = w·Φ((x − μ_w)/σ_w) + (1 − w)·Φ((x − μ_m)/σ_m)`; its percentiles
//!   are found by bisection on `F`.
//! * Correlated body dimensions are sampled as a multivariate normal
//!   `x = μ + σ ⊙ (L z)` with `L` the Cholesky factor of the correlation
//!   matrix and `z` independent standard normals.

use serde::{Deserialize, Serialize};

/// A normal distribution `N(mean, sd²)`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Normal {
    pub mean: f64,
    pub sd: f64,
}

impl Normal {
    pub const fn new(mean: f64, sd: f64) -> Self {
        Normal { mean, sd }
    }
    pub fn cdf(&self, x: f64) -> f64 {
        norm_cdf((x - self.mean) / self.sd.max(1e-12))
    }
    pub fn pdf(&self, x: f64) -> f64 {
        let s = self.sd.max(1e-12);
        norm_pdf((x - self.mean) / s) / s
    }
    pub fn quantile(&self, p: f64) -> f64 {
        self.mean + self.sd * norm_inv(p)
    }
    /// Probability of `lo ≤ X ≤ hi`.
    pub fn prob_between(&self, lo: f64, hi: f64) -> f64 {
        if hi <= lo {
            return 0.0;
        }
        (self.cdf(hi) - self.cdf(lo)).max(0.0)
    }
}

/// Complementary error function (Numerical Recipes `erfcc`, |ε| < 1.2e-7).
pub fn erfc(x: f64) -> f64 {
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.5 * z);
    let poly = -z * z - 1.265_512_23
        + t * (1.000_023_68
            + t * (0.374_091_96
                + t * (0.096_784_18
                    + t * (-0.186_288_06
                        + t * (0.278_868_07 + t * (-1.135_203_98 + t * (1.488_515_87 + t * (-0.822_152_23 + t * 0.170_872_77))))))));
    let r = t * poly.exp();
    if x >= 0.0 {
        r
    } else {
        2.0 - r
    }
}

/// Standard normal cumulative distribution Φ(z).
pub fn norm_cdf(z: f64) -> f64 {
    0.5 * erfc(-z / std::f64::consts::SQRT_2)
}

/// Standard normal density φ(z).
pub fn norm_pdf(z: f64) -> f64 {
    (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

/// Inverse standard normal Φ⁻¹(p) (Acklam + one Halley refinement).
pub fn norm_inv(p: f64) -> f64 {
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    let pl = 0.024_25;
    let x = if p < pl {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5]) / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= 1.0 - pl {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5]) / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };
    // One Halley step on Φ(x) − p.
    let e = norm_cdf(x) - p;
    let u = e * (2.0 * std::f64::consts::PI).sqrt() * (0.5 * x * x).exp();
    x - u / (1.0 + 0.5 * x * u)
}

/// A weighted mixture of normals (e.g. women and men).
#[derive(Clone, Debug)]
pub struct Mixture(pub Vec<(f64, Normal)>);

impl Mixture {
    fn total_w(&self) -> f64 {
        self.0.iter().map(|(w, _)| *w).sum::<f64>().max(1e-12)
    }
    pub fn cdf(&self, x: f64) -> f64 {
        self.0.iter().map(|(w, n)| w * n.cdf(x)).sum::<f64>() / self.total_w()
    }
    pub fn pdf(&self, x: f64) -> f64 {
        self.0.iter().map(|(w, n)| w * n.pdf(x)).sum::<f64>() / self.total_w()
    }
    pub fn mean(&self) -> f64 {
        self.0.iter().map(|(w, n)| w * n.mean).sum::<f64>() / self.total_w()
    }
    /// Standard deviation of the mixture (law of total variance).
    pub fn sd(&self) -> f64 {
        let m = self.mean();
        let v = self.0.iter().map(|(w, n)| w * (n.sd * n.sd + (n.mean - m).powi(2))).sum::<f64>() / self.total_w();
        v.sqrt()
    }
    pub fn prob_between(&self, lo: f64, hi: f64) -> f64 {
        if hi <= lo {
            return 0.0;
        }
        (self.cdf(hi) - self.cdf(lo)).max(0.0)
    }
    /// Percentile by bisection on the mixture CDF.
    pub fn quantile(&self, p: f64) -> f64 {
        let p = p.clamp(1e-9, 1.0 - 1e-9);
        let lo0 = self.0.iter().map(|(_, n)| n.mean - 9.0 * n.sd).fold(f64::INFINITY, f64::min);
        let hi0 = self.0.iter().map(|(_, n)| n.mean + 9.0 * n.sd).fold(f64::NEG_INFINITY, f64::max);
        let (mut lo, mut hi) = (lo0, hi0);
        for _ in 0..80 {
            let mid = 0.5 * (lo + hi);
            if self.cdf(mid) < p {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    }
}

/// Lower-triangular Cholesky factor of a symmetric positive-definite matrix.
/// Returns `None` if the matrix is not positive definite.
pub fn cholesky(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let s: f64 = (0..j).map(|k| l[i][k] * l[j][k]).sum();
            if i == j {
                let d = a[i][i] - s;
                if d <= 0.0 {
                    return None;
                }
                l[i][j] = d.sqrt();
            } else {
                l[i][j] = (a[i][j] - s) / l[j][j];
            }
        }
    }
    Some(l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_cdf_matches_tables() {
        // Standard normal table values.
        assert!((norm_cdf(0.0) - 0.5).abs() < 1e-7);
        assert!((norm_cdf(1.0) - 0.841_344_7).abs() < 2e-7);
        assert!((norm_cdf(1.96) - 0.975_002_1).abs() < 2e-7);
        assert!((norm_cdf(-1.644_853_6) - 0.05).abs() < 2e-7);
        assert!((norm_cdf(2.326_347_9) - 0.99).abs() < 2e-7);
    }

    #[test]
    fn inverse_normal_round_trips() {
        assert!((norm_inv(0.975) - 1.959_964).abs() < 1e-5);
        assert!((norm_inv(0.05) + 1.644_854).abs() < 1e-5);
        assert!((norm_inv(0.5)).abs() < 1e-6);
        for p in [0.001, 0.01, 0.1, 0.3, 0.7, 0.9, 0.99, 0.999] {
            assert!((norm_cdf(norm_inv(p)) - p).abs() < 3e-7, "{p}");
        }
    }

    #[test]
    fn mixture_of_equal_normals_is_the_normal() {
        let n = Normal::new(64.0, 3.4);
        let m = Mixture(vec![(0.5, n), (0.5, n)]);
        assert!((m.quantile(0.95) - n.quantile(0.95)).abs() < 1e-6);
        assert!((m.sd() - 3.4).abs() < 1e-12);
    }

    #[test]
    fn mixture_variance_by_hand() {
        // 50/50 mix of N(0,1) and N(2,1): mean 1, var = 1 + 1 = 2.
        let m = Mixture(vec![(0.5, Normal::new(0.0, 1.0)), (0.5, Normal::new(2.0, 1.0))]);
        assert!((m.mean() - 1.0).abs() < 1e-12);
        assert!((m.sd() - 2f64.sqrt()).abs() < 1e-12);
        // symmetric → median = mean
        assert!((m.quantile(0.5) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cholesky_two_by_two() {
        // [[1, r], [r, 1]] → L = [[1, 0], [r, √(1 − r²)]]
        let r = 0.3;
        let l = cholesky(&[vec![1.0, r], vec![r, 1.0]]).unwrap();
        assert!((l[1][0] - r).abs() < 1e-12);
        assert!((l[1][1] - (1.0 - r * r).sqrt()).abs() < 1e-12);
        assert!(cholesky(&[vec![1.0, 2.0], vec![2.0, 1.0]]).is_none());
    }
}
