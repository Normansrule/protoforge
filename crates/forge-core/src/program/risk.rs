//! Schedule risk analysis: three-point estimates, classic PERT and a seeded
//! Monte Carlo simulation of the whole network.
//!
//! **PERT** (Malcolm, Roseboom, Clark & Fazar 1959) approximates an activity
//! with optimistic a, most likely m and pessimistic b by a beta distribution
//! with mean `tₑ = (a + 4m + b)/6` and standard deviation `σ = (b − a)/6`.
//! Classic PERT then adds the means and variances along the critical path
//! and reads a normal distribution — which ignores **merge bias**: where
//! parallel paths join, the later of several uncertain finishes is later on
//! average than any one of them. The Monte Carlo simulation fixes that by
//! sampling every activity (beta-PERT `α = 1 + 4(m − a)/(b − a)`,
//! `β = 1 + 4(b − m)/(b − a)`, or triangular) and re-running the CPM forward
//! and backward passes per iteration. Beta-PERT samples come from a per-task
//! inverse-CDF table (the density integrated once on a fine grid); the
//! Marsaglia–Tsang Gamma sampler is kept as the reference it is tested against.
//!
//! Outputs: completion histogram and S-curve (cumulative probability), P50
//! and P80, the **criticality index** (share of iterations in which a task is
//! critical), Spearman rank correlation of each duration with the finish
//! (Hulett's schedule sensitivity) and a deterministic tornado (finish with a
//! task at its optimistic vs pessimistic value).
//!
//! References: D. G. Malcolm, J. H. Roseboom, C. E. Clark & W. Fazar,
//! "Application of a technique for research and development program
//! evaluation", *Operations Research* 7(5) (1959); D. T. Hulett, *Practical
//! Schedule Risk Analysis* (Gower, 2009); G. Marsaglia & W. W. Tsang, "A simple
//! method for generating gamma variables", *ACM TOMS* 26(3) (2000).

use serde::Serialize;

use super::cpm::{backward, build_net, cpm, driving_path, forward, EPS};
use super::wbs::Task;
use crate::math::Rng;

/// PERT mean, variance and standard deviation of (a, m, b).
pub fn pert(a: f64, m: f64, b: f64) -> (f64, f64, f64) {
    let mean = (a + 4.0 * m + b) / 6.0;
    let sd = (b - a) / 6.0;
    (mean, sd * sd, sd)
}

/// Triangular mean and variance.
pub fn tri_moments(a: f64, m: f64, b: f64) -> (f64, f64) {
    let mean = (a + m + b) / 3.0;
    let var = (a * a + m * m + b * b - a * m - a * b - m * b) / 18.0;
    (mean, var)
}

/// Inverse-CDF triangular sample from a uniform u.
pub fn triangular(a: f64, m: f64, b: f64, u: f64) -> f64 {
    if b - a < 1e-12 {
        return m;
    }
    let fc = (m - a) / (b - a);
    if u < fc {
        a + (u * (b - a) * (m - a)).sqrt()
    } else {
        b - ((1.0 - u) * (b - a) * (b - m)).sqrt()
    }
}

/// Gamma(shape ≥ 1, scale 1) by Marsaglia–Tsang; shape < 1 via the boost u^(1/k).
pub fn gamma(rng: &mut Rng, k: f64) -> f64 {
    if k < 1.0 {
        let u = rng.uniform().max(1e-300);
        return gamma(rng, k + 1.0) * u.powf(1.0 / k);
    }
    let d = k - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();
    loop {
        let x = rng.normal();
        let v = 1.0 + c * x;
        if v <= 0.0 {
            continue;
        }
        let v = v * v * v;
        let u = rng.uniform().max(1e-300);
        if u < 1.0 - 0.0331 * x * x * x * x || u.ln() < 0.5 * x * x + d * (1.0 - v + v.ln()) {
            return d * v;
        }
    }
}

/// Beta-PERT sample of (a, m, b).
pub fn pert_sample(rng: &mut Rng, a: f64, m: f64, b: f64) -> f64 {
    if b - a < 1e-12 {
        return m;
    }
    let al = 1.0 + 4.0 * (m - a) / (b - a);
    let be = 1.0 + 4.0 * (b - m) / (b - a);
    let x = gamma(rng, al);
    let y = gamma(rng, be);
    a + (b - a) * x / (x + y)
}

/// Beta-PERT inverse-CDF table: the density u^(α−1)(1−u)^(β−1) is
/// integrated on a fine grid once per task, so a sample is one uniform
/// number and a table lookup (≈ 10× faster than two Gamma variates, which
/// keeps the requirement metric inside its time budget).
#[derive(Clone, Debug)]
pub struct PertTable {
    a: f64,
    w: f64,
    q: Vec<f64>,
}

impl PertTable {
    const N: usize = 1024;
    pub fn new(a: f64, m: f64, b: f64) -> PertTable {
        let w = b - a;
        if w < 1e-12 {
            return PertTable { a: m, w: 0.0, q: vec![0.0, 0.0] };
        }
        let al = 1.0 + 4.0 * (m - a) / w;
        let be = 1.0 + 4.0 * (b - m) / w;
        let g = 4096usize;
        let pdf = |u: f64| u.powf(al - 1.0) * (1.0 - u).powf(be - 1.0);
        let mut cdf = vec![0.0; g + 1];
        let mut prev = pdf(0.0);
        for k in 1..=g {
            let u = k as f64 / g as f64;
            let cur = pdf(u);
            cdf[k] = cdf[k - 1] + 0.5 * (prev + cur);
            prev = cur;
        }
        let tot = cdf[g].max(1e-300);
        let mut q = Vec::with_capacity(Self::N + 1);
        let mut k = 0usize;
        for i in 0..=Self::N {
            let p = i as f64 / Self::N as f64 * tot;
            while k < g && cdf[k + 1] < p {
                k += 1;
            }
            let (c0, c1) = (cdf[k], cdf[(k + 1).min(g)]);
            let f = if c1 > c0 { ((p - c0) / (c1 - c0)).clamp(0.0, 1.0) } else { 0.0 };
            q.push(((k as f64 + f) / g as f64).min(1.0));
        }
        PertTable { a, w, q }
    }
    pub fn sample(&self, u: f64) -> f64 {
        if self.w == 0.0 {
            return self.a;
        }
        let x = u.clamp(0.0, 1.0) * Self::N as f64;
        let i = (x as usize).min(Self::N - 1);
        let f = x - i as f64;
        self.a + self.w * (self.q[i] + f * (self.q[i + 1] - self.q[i]))
    }
}

/// Per-task sampler for the chosen distribution.
enum Sampler {
    Tri(f64, f64, f64),
    Pert(PertTable),
}

impl Sampler {
    fn new(tp: (f64, f64, f64), tri: bool) -> Sampler {
        if tri {
            Sampler::Tri(tp.0, tp.1, tp.2)
        } else {
            Sampler::Pert(PertTable::new(tp.0, tp.1, tp.2))
        }
    }
    fn draw(&self, rng: &mut Rng) -> f64 {
        match self {
            Sampler::Tri(a, m, b) => triangular(*a, *m, *b, rng.uniform()),
            Sampler::Pert(t) => t.sample(rng.uniform()),
        }
    }
}

/// Linear-interpolated percentile of sorted data (q in 0..1).
pub fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let h = (sorted.len() - 1) as f64 * q.clamp(0.0, 1.0);
    let lo = h.floor() as usize;
    let hi = h.ceil() as usize;
    sorted[lo] + (h - lo as f64) * (sorted[hi] - sorted[lo])
}

fn ranks(v: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..v.len()).collect();
    idx.sort_by(|&a, &b| v[a].partial_cmp(&v[b]).unwrap());
    let mut r = vec![0.0; v.len()];
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && (v[idx[j + 1]] - v[idx[i]]).abs() < 1e-12 {
            j += 1;
        }
        let avg = (i + j) as f64 / 2.0;
        for k in i..=j {
            r[idx[k]] = avg;
        }
        i = j + 1;
    }
    r
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    if n < 2.0 {
        return 0.0;
    }
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        sxy += (a - mx) * (b - my);
        sxx += (a - mx) * (a - mx);
        syy += (b - my) * (b - my);
    }
    if sxx < 1e-18 || syy < 1e-18 {
        0.0
    } else {
        sxy / (sxx * syy).sqrt()
    }
}

/// Spearman rank correlation.
pub fn spearman(x: &[f64], y: &[f64]) -> f64 {
    pearson(&ranks(x), &ranks(y))
}

#[derive(Clone, Debug, Serialize)]
pub struct TaskRisk {
    pub id: String,
    pub name: String,
    pub mean: f64,
    pub sd: f64,
    /// Share of iterations in which the task was critical (0..1).
    pub criticality: f64,
    /// Spearman rank correlation of its duration with the finish.
    pub correlation: f64,
    /// Criticality × correlation (Hulett's "cruciality").
    pub cruciality: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Tornado {
    pub id: String,
    pub name: String,
    pub low: f64,
    pub high: f64,
    pub swing: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct MilestoneRisk {
    pub id: String,
    pub name: String,
    pub deterministic: f64,
    pub p50: f64,
    pub p80: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Preview {
    pub finish: f64,
    /// Indices (into `tasks`) critical in this iteration.
    pub critical: Vec<u16>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Histogram {
    pub lo: f64,
    pub width: f64,
    pub counts: Vec<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RiskResult {
    pub iterations: usize,
    pub dist: String,
    pub seed: u64,
    /// Every iteration's finish (weeks), in sampling order (for the animation).
    pub finishes: Vec<f64>,
    pub histogram: Histogram,
    /// S-curve: (weeks, cumulative probability), 101 points.
    pub s_curve: Vec<(f64, f64)>,
    pub mean: f64,
    pub sd: f64,
    pub p10: f64,
    pub p50: f64,
    pub p80: f64,
    pub p90: f64,
    pub min: f64,
    pub max: f64,
    /// CPM finish with most likely durations.
    pub deterministic: f64,
    /// Probability of finishing by the deterministic date.
    pub prob_deterministic: f64,
    /// Classic PERT along the critical path of the mean-duration network.
    pub pert_mean: f64,
    pub pert_sd: f64,
    pub pert_p80: f64,
    pub pert_path: Vec<String>,
    pub tasks: Vec<TaskRisk>,
    pub tornado: Vec<Tornado>,
    pub milestones: Vec<MilestoneRisk>,
    pub preview: Vec<Preview>,
    pub ids: Vec<String>,
    pub notes: Vec<String>,
}

/// Quick percentile-only run (used by the metrics): finishes sorted.
pub fn finishes(tasks: &[Task], iterations: usize, seed: u64, dist: &str) -> Result<Vec<f64>, String> {
    let net = build_net(tasks)?;
    let tp: Vec<(f64, f64, f64)> = tasks.iter().map(|t| t.three_point()).collect();
    let n = tasks.len();
    let mut rng = Rng::new(seed);
    let (mut es, mut ef, mut d) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    let tri = dist.eq_ignore_ascii_case("triangular");
    let samplers: Vec<Sampler> = tp.iter().map(|&x| Sampler::new(x, tri)).collect();
    let mut out = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        for (k, s) in samplers.iter().enumerate() {
            d[k] = s.draw(&mut rng);
        }
        out.push(forward(&net, &d, &mut es, &mut ef));
    }
    out.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Ok(out)
}

/// Full Monte Carlo schedule risk analysis.
pub fn simulate(tasks: &[Task], iterations: usize, seed: u64, dist: &str) -> Result<RiskResult, String> {
    let iterations = iterations.clamp(10, 20000);
    let net = build_net(tasks)?;
    let n = tasks.len();
    let tp: Vec<(f64, f64, f64)> = tasks.iter().map(|t| t.three_point()).collect();
    let tri = dist.eq_ignore_ascii_case("triangular");
    let mut rng = Rng::new(seed);
    let (mut es, mut ef, mut ls, mut lf) = (vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    let mut d = vec![0.0; n];
    let mut dur: Vec<Vec<f64>> = vec![Vec::with_capacity(iterations); n];
    let mut crit = vec![0usize; n];
    let mut fin = Vec::with_capacity(iterations);
    let ms: Vec<usize> = (0..n).filter(|&k| tasks[k].milestone).collect();
    let mut ms_ef: Vec<Vec<f64>> = vec![Vec::with_capacity(iterations); ms.len()];
    let mut preview = Vec::new();
    let samplers: Vec<Sampler> = tp.iter().map(|&x| Sampler::new(x, tri)).collect();
    for it in 0..iterations {
        for (k, s) in samplers.iter().enumerate() {
            d[k] = s.draw(&mut rng);
            dur[k].push(d[k]);
        }
        let f = forward(&net, &d, &mut es, &mut ef);
        backward(&net, &d, f, &mut ls, &mut lf);
        let mut cr = Vec::new();
        for k in 0..n {
            if (ls[k] - es[k]).abs() < EPS {
                crit[k] += 1;
                if it < 60 {
                    cr.push(k as u16);
                }
            }
        }
        for (j, &k) in ms.iter().enumerate() {
            ms_ef[j].push(ef[k]);
        }
        if it < 60 {
            preview.push(Preview { finish: f, critical: cr });
        }
        fin.push(f);
    }
    let mut sorted = fin.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let nf = iterations as f64;
    let mean = fin.iter().sum::<f64>() / nf;
    let sd = (fin.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (nf - 1.0).max(1.0)).sqrt();
    let (lo, hi) = (sorted[0], sorted[sorted.len() - 1]);
    let bins = 32usize;
    let width = ((hi - lo) / bins as f64).max(1e-9);
    let mut counts = vec![0u32; bins];
    for &x in &fin {
        let b = (((x - lo) / width) as usize).min(bins - 1);
        counts[b] += 1;
    }
    let s_curve: Vec<(f64, f64)> = (0..=100)
        .map(|k| (percentile(&sorted, k as f64 / 100.0), k as f64 / 100.0))
        .collect();
    // deterministic and classic PERT
    let dm: Vec<f64> = tp.iter().map(|x| x.1).collect();
    let det = cpm(&net, &dm).finish;
    let prob_det = sorted.iter().filter(|&&x| x <= det + EPS).count() as f64 / nf;
    let te: Vec<f64> = tp.iter().map(|&(a, m, b)| pert(a, m, b).0).collect();
    let pp = cpm(&net, &te);
    let path = driving_path(&net, &pp);
    let pvar: f64 = path.iter().map(|&k| pert(tp[k].0, tp[k].1, tp[k].2).1).sum();
    let psd = pvar.sqrt();
    // per-task statistics
    let fin_ranks = ranks(&fin);
    let mut trisk = Vec::new();
    for k in 0..n {
        if tasks[k].milestone {
            continue;
        }
        let (a, m, b) = tp[k];
        let (mu, s) = if tri {
            let (mu, v) = tri_moments(a, m, b);
            (mu, v.sqrt())
        } else {
            let (mu, _, s) = pert(a, m, b);
            (mu, s)
        };
        let ci = crit[k] as f64 / nf;
        let rho = if b - a > 1e-12 { pearson(&ranks(&dur[k]), &fin_ranks) } else { 0.0 };
        trisk.push(TaskRisk {
            id: tasks[k].id.clone(),
            name: tasks[k].name.clone(),
            mean: mu,
            sd: s,
            criticality: ci,
            correlation: rho,
            cruciality: ci * rho,
        });
    }
    // deterministic tornado
    let mut tor = Vec::new();
    for k in 0..n {
        let (a, m, b) = tp[k];
        if b - a < 1e-9 {
            continue;
        }
        let mut dd = dm.clone();
        dd[k] = a;
        let low = cpm(&net, &dd).finish;
        dd[k] = b;
        let high = cpm(&net, &dd).finish;
        let _ = m;
        tor.push(Tornado {
            id: tasks[k].id.clone(),
            name: tasks[k].name.clone(),
            low,
            high,
            swing: high - low,
        });
    }
    tor.sort_by(|x, y| y.swing.partial_cmp(&x.swing).unwrap().then(x.id.cmp(&y.id)));
    tor.retain(|t| t.swing > 1e-9);
    tor.truncate(14);
    let det_pass = cpm(&net, &dm);
    let milestones = ms
        .iter()
        .enumerate()
        .map(|(j, &k)| {
            let mut v = ms_ef[j].clone();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            MilestoneRisk {
                id: tasks[k].id.clone(),
                name: tasks[k].name.clone(),
                deterministic: det_pass.ef[k],
                p50: percentile(&v, 0.5),
                p80: percentile(&v, 0.8),
            }
        })
        .collect();
    let p80 = percentile(&sorted, 0.8);
    let notes = vec![
        format!(
            "The plan's most-likely finish ({det:.1} wk) has only a {:.0} % chance — three-point estimates are skewed to the right and parallel paths add merge bias.",
            prob_det * 100.0
        ),
        format!(
            "Classic PERT on the mean-duration critical path gives P80 ≈ {:.1} wk; the Monte Carlo P80 is {:.1} wk. The difference is merge bias, which PERT ignores.",
            te_p80(pp.finish, psd),
            p80
        ),
        "Durations are sampled independently. Real risks are correlated (one supplier, one team); Hulett recommends modelling risk drivers — treat these percentiles as optimistic.".into(),
    ];
    Ok(RiskResult {
        iterations,
        dist: if tri { "triangular".into() } else { "pert".into() },
        seed,
        finishes: fin,
        histogram: Histogram { lo, width, counts },
        s_curve,
        mean,
        sd,
        p10: percentile(&sorted, 0.1),
        p50: percentile(&sorted, 0.5),
        p80,
        p90: percentile(&sorted, 0.9),
        min: lo,
        max: hi,
        deterministic: det,
        prob_deterministic: prob_det,
        pert_mean: pp.finish,
        pert_sd: psd,
        pert_p80: te_p80(pp.finish, psd),
        pert_path: path.iter().map(|&k| tasks[k].id.clone()).collect(),
        tasks: trisk,
        tornado: tor,
        milestones,
        preview,
        ids: tasks.iter().map(|t| t.id.clone()).collect(),
        notes,
    })
}

/// Normal 80th percentile: μ + 0.8416 σ.
fn te_p80(mean: f64, sd: f64) -> f64 {
    mean + 0.841_621_233_6 * sd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pert_moments_hand_calc() {
        // a = 2, m = 4, b = 12: tₑ = (2 + 16 + 12)/6 = 5, σ = 10/6, σ² = 2.7778
        let (m, v, s) = pert(2.0, 4.0, 12.0);
        assert!((m - 5.0).abs() < 1e-12);
        assert!((v - 100.0 / 36.0).abs() < 1e-12);
        assert!((s - 10.0 / 6.0).abs() < 1e-12);
        let (tm, tv) = tri_moments(2.0, 4.0, 12.0);
        assert!((tm - 6.0).abs() < 1e-12);
        assert!((tv - (4.0 + 16.0 + 144.0 - 8.0 - 24.0 - 48.0) / 18.0).abs() < 1e-12);
    }

    #[test]
    fn samplers_match_their_means() {
        let mut rng = Rng::new(3);
        let n = 40000;
        let s: f64 = (0..n).map(|_| pert_sample(&mut rng, 2.0, 4.0, 12.0)).sum::<f64>() / n as f64;
        assert!((s - 5.0).abs() < 0.05, "beta-PERT mean {s}");
        // the inverse-CDF table matches the beta-PERT mean and variance (α = 1.8, β = 4.2: var = 100·αβ/((α+β)²(α+β+1)) = 3.0)
        let t = PertTable::new(2.0, 4.0, 12.0);
        let xs: Vec<f64> = (0..n).map(|_| t.sample(rng.uniform())).collect();
        let mu = xs.iter().sum::<f64>() / n as f64;
        let var = xs.iter().map(|x| (x - mu) * (x - mu)).sum::<f64>() / n as f64;
        assert!((mu - 5.0).abs() < 0.05, "table mean {mu}");
        assert!((var - 3.0).abs() < 0.12, "table variance {var}");
        assert_eq!(t.sample(0.0), 2.0);
        assert!((t.sample(1.0) - 12.0).abs() < 1e-9);
        let s: f64 = (0..n).map(|_| triangular(2.0, 4.0, 12.0, rng.uniform())).sum::<f64>() / n as f64;
        assert!((s - 6.0).abs() < 0.05, "triangular mean {s}");
        let g: f64 = (0..n).map(|_| gamma(&mut rng, 2.5)).sum::<f64>() / n as f64;
        assert!((g - 2.5).abs() < 0.05);
    }

    #[test]
    fn spearman_and_percentiles() {
        assert!((spearman(&[1.0, 2.0, 3.0, 4.0], &[10.0, 20.0, 25.0, 100.0]) - 1.0).abs() < 1e-12);
        assert!((spearman(&[1.0, 2.0, 3.0], &[3.0, 2.0, 1.0]) + 1.0).abs() < 1e-12);
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0, 5.0], 0.5), 3.0);
        assert!((percentile(&[0.0, 10.0], 0.8) - 8.0).abs() < 1e-12);
    }

    /// Two equal parallel paths: merge bias pushes the Monte Carlo mean above
    /// the PERT mean, and each path is critical about half the time.
    #[test]
    fn merge_bias_and_criticality() {
        let mk = |id: &str, p: &[&str]| Task {
            id: id.into(),
            name: id.into(),
            optimistic: 2.0,
            likely: 4.0,
            pessimistic: 10.0,
            preds: p.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        let ts = vec![
            mk("A", &[]),
            mk("B", &[]),
            Task {
                id: "END".into(),
                milestone: true,
                preds: vec!["A".into(), "B".into()],
                ..Default::default()
            },
        ];
        let r = simulate(&ts, 4000, 11, "pert").unwrap();
        assert!((r.pert_mean - 14.0 / 3.0).abs() < 1e-9);
        assert!(r.mean > r.pert_mean + 0.4, "merge bias: {} vs {}", r.mean, r.pert_mean);
        let a = r.tasks.iter().find(|t| t.id == "A").unwrap();
        assert!((a.criticality - 0.5).abs() < 0.05, "{}", a.criticality);
        assert!(r.p80 > r.p50 && r.p50 > r.p10);
        // deterministic, seeded
        let r2 = simulate(&ts, 4000, 11, "pert").unwrap();
        assert_eq!(r.p80, r2.p80);
    }
}
