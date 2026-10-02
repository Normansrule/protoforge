//! Special functions for reliability statistics: log-gamma and gamma
//! (Lanczos), the standard normal distribution, the regularised incomplete
//! gamma function and chi-square quantiles, and the binomial distribution.
//!
//! References: W. H. Press et al., *Numerical Recipes*, 3rd ed., 2007
//! (§6.1 gamma, §6.2 incomplete gamma and erfc); P. J. Acklam's rational
//! approximation of the normal quantile (refined with one Halley step);
//! C. Lanczos, "A precision approximation of the gamma function", SIAM J.
//! Numer. Anal. B 1, 1964.

use std::f64::consts::PI;

const LANCZOS: [f64; 9] = [
    0.999_999_999_999_809_9,
    676.520_368_121_885_1,
    -1_259.139_216_722_402_8,
    771.323_428_777_653_1,
    -176.615_029_162_140_6,
    12.507_343_278_686_905,
    -0.138_571_095_265_720_12,
    9.984_369_578_019_572e-6,
    1.505_632_735_149_311_6e-7,
];

/// ln Γ(x) for x > 0 (Lanczos, g = 7), ~15 significant digits.
pub fn ln_gamma(x: f64) -> f64 {
    if x < 0.5 {
        // reflection
        return (PI / (PI * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = LANCZOS[0];
    let t = x + 7.5;
    for (i, c) in LANCZOS.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    0.5 * (2.0 * PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// Γ(x) for x > 0.
pub fn gamma(x: f64) -> f64 {
    ln_gamma(x).exp()
}

/// Complementary error function, erfc(x) = Q(½, x²) for x ≥ 0 (full double precision).
pub fn erfc(x: f64) -> f64 {
    let q = gamma_pq(0.5, x * x).1;
    if x >= 0.0 {
        q
    } else {
        2.0 - q
    }
}

/// Standard normal cumulative distribution Φ(x).
pub fn norm_cdf(x: f64) -> f64 {
    0.5 * erfc(-x / std::f64::consts::SQRT_2)
}

/// Standard normal quantile Φ⁻¹(p) (Acklam + one Halley refinement).
pub fn norm_quantile(p: f64) -> f64 {
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
    // one Halley step on Φ(x) − p (erfc is accurate to double precision)
    let e = norm_cdf(x) - p;
    let u = e * (2.0 * PI).sqrt() * (0.5 * x * x).exp();
    x - u / (1.0 + 0.5 * x * u)
}

/// Regularised lower incomplete gamma P(a, x) = γ(a, x) / Γ(a).
pub fn gamma_p(a: f64, x: f64) -> f64 {
    gamma_pq(a, x).0
}

/// (P(a, x), Q(a, x) = 1 − P) each computed without cancellation: the series
/// for x < a + 1, the continued fraction (modified Lentz) otherwise.
pub fn gamma_pq(a: f64, x: f64) -> (f64, f64) {
    if x <= 0.0 || a <= 0.0 {
        return (0.0, 1.0);
    }
    let gln = ln_gamma(a);
    if x < a + 1.0 {
        let mut ap = a;
        let mut sum = 1.0 / a;
        let mut del = sum;
        for _ in 0..2000 {
            ap += 1.0;
            del *= x / ap;
            sum += del;
            if del.abs() < sum.abs() * 1e-16 {
                break;
            }
        }
        let p = (sum.ln() - x + a * x.ln() - gln).exp().min(1.0);
        (p, 1.0 - p)
    } else {
        let tiny = 1e-300;
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / tiny;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..2000 {
            let an = -(i as f64) * (i as f64 - a);
            b += 2.0;
            d = an * d + b;
            if d.abs() < tiny {
                d = tiny;
            }
            c = b + an / c;
            if c.abs() < tiny {
                c = tiny;
            }
            d = 1.0 / d;
            let del = d * c;
            h *= del;
            if (del - 1.0).abs() < 1e-16 {
                break;
            }
        }
        let q = ((-x + a * x.ln() - gln).exp() * h).clamp(0.0, 1.0);
        (1.0 - q, q)
    }
}

/// Chi-square cumulative distribution with `dof` degrees of freedom.
pub fn chi2_cdf(x: f64, dof: f64) -> f64 {
    gamma_p(0.5 * dof, 0.5 * x)
}

/// Chi-square quantile: the x with P(χ²_dof ≤ x) = p (bisection on a log scale, ~1e-12 relative).
pub fn chi2_quantile(p: f64, dof: f64) -> f64 {
    if p <= 0.0 || dof <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    let (mut lo, mut hi) = (1e-12f64, dof.max(1.0));
    while chi2_cdf(hi, dof) < p {
        hi *= 2.0;
        if hi > 1e7 {
            break;
        }
    }
    for _ in 0..200 {
        let mid = (lo * hi).sqrt();
        if chi2_cdf(mid, dof) < p {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi / lo - 1.0 < 1e-13 {
            break;
        }
    }
    (lo * hi).sqrt()
}

/// ln C(n, k).
pub fn ln_choose(n: f64, k: f64) -> f64 {
    ln_gamma(n + 1.0) - ln_gamma(k + 1.0) - ln_gamma(n - k + 1.0)
}

/// Binomial CDF: P(X ≤ r) for X ~ Bin(n, p).
pub fn binom_cdf(r: u64, n: u64, p: f64) -> f64 {
    if p <= 0.0 {
        return 1.0;
    }
    if p >= 1.0 {
        return if r >= n { 1.0 } else { 0.0 };
    }
    let (lp, lq) = (p.ln(), (1.0 - p).ln());
    let mut s = 0.0;
    for i in 0..=r.min(n) {
        let (i, nf) = (i as f64, n as f64);
        s += (ln_choose(nf, i) + i * lp + (nf - i) * lq).exp();
    }
    s.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gamma_values() {
        assert!((gamma(5.0) - 24.0).abs() < 1e-10);
        assert!((gamma(0.5) - PI.sqrt()).abs() < 1e-12);
        assert!((gamma(1.5) - 0.5 * PI.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn normal_quantiles() {
        assert!((norm_quantile(0.975) - 1.959_963_985).abs() < 1e-7);
        assert!((norm_quantile(0.95) - 1.644_853_627).abs() < 1e-7);
        assert!((norm_quantile(0.5)).abs() < 1e-9);
        assert!((norm_cdf(1.0) - 0.841_344_746_068_5).abs() < 1e-12);
        assert!((norm_cdf(-5.0) - 2.866_515_718_791e-7).abs() < 1e-15);
    }

    #[test]
    fn chi_square_table_values() {
        // textbook table values
        assert!((chi2_quantile(0.90, 2.0) - 4.605_170).abs() < 1e-5); // = −2 ln 0.1
        assert!((chi2_quantile(0.95, 1.0) - 3.841_459).abs() < 1e-5);
        assert!((chi2_quantile(0.90, 4.0) - 7.779_440).abs() < 1e-5);
        assert!((chi2_quantile(0.05, 10.0) - 3.940_299).abs() < 1e-5);
    }

    #[test]
    fn binomial() {
        // P(X ≤ 1), n = 10, p = 0.1: 0.9^10 + 10·0.1·0.9^9
        let v = 0.9f64.powi(10) + 0.9f64.powi(9);
        assert!((binom_cdf(1, 10, 0.1) - v).abs() < 1e-12);
    }
}
