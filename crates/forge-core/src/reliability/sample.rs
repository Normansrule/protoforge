//! Sample sizes for reliability demonstration tests.
//!
//! * **Success run** (zero failures, one life): `n = ln(1 − C) / ln R`,
//!   e.g. R = 0.9, C = 0.9 → n = 21.85 → **22** units.
//! * **Binomial with r allowed failures**: the smallest n with
//!   `Σ_{i≤r} C(n,i) (1−R)^i R^{n−i} ≤ 1 − C` (the Clopper–Pearson lower bound
//!   on R reaches the target).
//! * **Weibayes / Lipson extension**: testing each unit for k lives when the
//!   failure mode's Weibull shape β is known trades units for time,
//!   `n = χ²(C; 2r + 2) / (2 k^β (−ln R))` (= ln(1−C)/(k^β ln R) for r = 0).
//! * **Operating characteristic (OC) curve**: probability of passing the plan
//!   given the true reliability; for the Lipson plan a unit survives the test
//!   with probability `R_true^{k^β}`.
//!
//! References: C. Lipson & N. J. Sheth, *Statistical Design and Analysis of
//! Engineering Experiments*, McGraw-Hill 1973; D. Kececioglu, *Reliability &
//! Life Testing Handbook*; O'Connor & Kleyner, *Practical Reliability
//! Engineering*, ch. 14; NIST/SEMATECH e-Handbook of Statistical Methods,
//! §8.3.1.

use serde::{Deserialize, Serialize};

use super::stats;

/// Success-run sample size (zero failures, one life).
pub fn success_run(r: f64, c: f64) -> u32 {
    ((1.0 - c).ln() / r.ln() - 1e-9).ceil().max(1.0) as u32
}

/// Smallest n that demonstrates R at confidence C with at most `fails` failures.
pub fn binomial_n(r: f64, c: f64, fails: u32) -> u32 {
    let mut n = (fails + 1).max(success_run(r, c));
    while stats::binom_cdf(fails as u64, n as u64, 1.0 - r) > 1.0 - c {
        n += 1;
        if n > 1_000_000 {
            break;
        }
    }
    n
}

/// Lipson/Weibayes units for a test of `k` lives per unit.
pub fn lipson_n(r: f64, c: f64, beta: f64, k: f64, fails: u32) -> u32 {
    let req = stats::chi2_quantile(c, 2.0 * fails as f64 + 2.0) / (2.0 * -r.ln());
    (req / k.powf(beta) - 1e-9).ceil().max(1.0 + fails as f64) as u32
}

/// Lower confidence bound on R after `fails` failures in n trials (Clopper–Pearson).
pub fn demonstrated_r(n: u32, fails: u32, c: f64) -> f64 {
    if fails >= n {
        return 0.0;
    }
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..100 {
        let mid = 0.5 * (lo + hi);
        // P(X ≤ fails | p = 1 − mid) rises with mid
        if stats::binom_cdf(fails as u64, n as u64, 1.0 - mid) < 1.0 - c {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct SampleIn {
    pub reliability: f64,
    pub confidence: f64,
    pub max_failures: u32,
    pub beta: f64,
    /// Lives each unit is tested for (Lipson).
    pub multiple: f64,
    /// Plan evaluated on the OC curve.
    pub plan_n: u32,
    pub plan_failures: u32,
    /// Reliability the design is believed to have (producer's risk point).
    pub design_reliability: f64,
}

impl Default for SampleIn {
    fn default() -> Self {
        SampleIn {
            reliability: 0.9,
            confidence: 0.9,
            max_failures: 3,
            beta: 2.0,
            multiple: 2.0,
            plan_n: 22,
            plan_failures: 0,
            design_reliability: 0.98,
        }
    }
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Row {
    pub failures: u32,
    pub binomial_n: u32,
    pub lipson_n: u32,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Curve {
    pub label: String,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct SampleOut {
    pub success_run_n: u32,
    pub success_run_exact: f64,
    pub rows: Vec<Row>,
    /// Units vs test multiple k for several β (zero failures).
    pub lipson_curves: Vec<Curve>,
    /// Units vs R for C = 0.8, 0.9, 0.95, 0.99 (success run).
    pub success_curves: Vec<Curve>,
    pub oc_r: Vec<f64>,
    pub oc_binomial: Vec<f64>,
    pub oc_lipson: Vec<f64>,
    pub lipson_plan_n: u32,
    pub consumer_risk: f64,
    pub producer_risk_binomial: f64,
    pub producer_risk_lipson: f64,
    pub demonstrated_r: f64,
    pub notes: Vec<String>,
}

/// Every sample-size answer for one (R, C) target.
pub fn analyze(i: SampleIn) -> Result<SampleOut, String> {
    let (r, c) = (i.reliability, i.confidence);
    if !(r > 0.0 && r < 1.0 && c > 0.0 && c < 1.0) {
        return Err("reliability and confidence must be between 0 and 1".into());
    }
    if !(i.beta > 0.0 && i.multiple > 0.0) {
        return Err("β and the test multiple must be positive".into());
    }
    let rows: Vec<Row> = (0..=i.max_failures.min(10))
        .map(|f| Row {
            failures: f,
            binomial_n: binomial_n(r, c, f),
            lipson_n: lipson_n(r, c, i.beta, i.multiple, f),
        })
        .collect();
    let ks: Vec<f64> = (0..=50).map(|k| 0.5 + 2.5 * k as f64 / 50.0).collect();
    let lipson_curves = [1.0, 1.5, 2.0, 3.0]
        .iter()
        .map(|&b| Curve {
            label: format!("β = {b}"),
            x: ks.clone(),
            y: ks.iter().map(|&k| lipson_n(r, c, b, k, 0) as f64).collect(),
        })
        .collect();
    let rs: Vec<f64> = (0..=60).map(|k| 0.8 + 0.195 * k as f64 / 60.0).collect();
    let success_curves = [0.8, 0.9, 0.95, 0.99]
        .iter()
        .map(|&cc| Curve {
            label: format!("C = {:.0} %", cc * 100.0),
            x: rs.clone(),
            y: rs.iter().map(|&rr| success_run(rr, cc) as f64).collect(),
        })
        .collect();
    let pn = i.plan_n.max(1);
    let pf = i.plan_failures.min(pn.saturating_sub(1));
    let lip_n = lipson_n(r, c, i.beta, i.multiple, pf);
    let kb = i.multiple.powf(i.beta);
    let lo = (r - 3.0 * (1.0 - r)).max(0.5);
    let oc_r: Vec<f64> = (0..=120).map(|k| lo + (0.9999 - lo) * k as f64 / 120.0).collect();
    let acc_bin = |rt: f64| stats::binom_cdf(pf as u64, pn as u64, 1.0 - rt);
    let acc_lip = |rt: f64| stats::binom_cdf(pf as u64, lip_n as u64, 1.0 - rt.powf(kb));
    let mut notes = vec![];
    if i.multiple > 3.0 {
        notes.push("Test multiples above about 3 lives risk failure modes that never happen in the field.".into());
    }
    notes.push(format!(
        "Consumer's risk: a design with exactly R = {r} passes with probability ≤ {:.0} % (1 − C). The OC curve shows the producer's side: a good design can still fail the test.",
        (1.0 - c) * 100.0
    ));
    Ok(SampleOut {
        success_run_n: success_run(r, c),
        success_run_exact: (1.0 - c).ln() / r.ln(),
        rows,
        lipson_curves,
        success_curves,
        oc_binomial: oc_r.iter().map(|&x| acc_bin(x)).collect(),
        oc_lipson: oc_r.iter().map(|&x| acc_lip(x)).collect(),
        oc_r,
        lipson_plan_n: lip_n,
        consumer_risk: acc_bin(r),
        producer_risk_binomial: 1.0 - acc_bin(i.design_reliability),
        producer_risk_lipson: 1.0 - acc_lip(i.design_reliability),
        demonstrated_r: demonstrated_r(pn, pf, c),
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_run_theorem() {
        // n = ln(1 − C) / ln R: R = 0.9, C = 0.9 → 21.85 → 22
        assert_eq!(success_run(0.9, 0.9), 22);
        assert_eq!(success_run(0.95, 0.9), 45); // 44.89
        assert_eq!(success_run(0.99, 0.95), 299); // 298.07
        assert_eq!(binomial_n(0.9, 0.9, 0), 22);
    }

    #[test]
    fn binomial_with_failures() {
        // R = 0.9, C = 0.9, one failure allowed: n = 38 (standard table value)
        let n = binomial_n(0.9, 0.9, 1);
        assert_eq!(n, 38);
        assert!(stats::binom_cdf(1, 38, 0.1) <= 0.1 && stats::binom_cdf(1, 37, 0.1) > 0.1);
    }

    #[test]
    fn lipson_extension() {
        // β = 2, two lives: 21.85 / 4 = 5.46 → 6; β = 1 (exponential): 21.85/2 → 11
        assert_eq!(lipson_n(0.9, 0.9, 2.0, 2.0, 0), 6);
        assert_eq!(lipson_n(0.9, 0.9, 1.0, 2.0, 0), 11);
        // one life reproduces the success run
        assert_eq!(lipson_n(0.9, 0.9, 3.0, 1.0, 0), 22);
    }

    #[test]
    fn demonstrated_reliability_inverts_the_plan() {
        let rd = demonstrated_r(22, 0, 0.9);
        // 0.1^(1/22) = 0.9006
        assert!((rd - 0.1f64.powf(1.0 / 22.0)).abs() < 1e-9);
        let o = analyze(SampleIn::default()).unwrap();
        assert!(o.consumer_risk <= 0.1 + 1e-9);
        assert!(o.oc_binomial.windows(2).all(|w| w[1] >= w[0] - 1e-12));
    }
}
