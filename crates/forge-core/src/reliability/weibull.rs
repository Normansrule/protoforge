//! Two-parameter Weibull life-data analysis with right-censored data
//! (suspensions).
//!
//! * **Median ranks** — Johnson's adjusted ranks for suspensions,
//!   `rank_i = (RR_i · rank_{i−1} + N + 1) / (RR_i + 1)` with RR the reverse
//!   rank, turned into plotting positions with Bernard's approximation
//!   `F ≈ (rank − 0.3) / (N + 0.4)` (Abernethy, *The New Weibull Handbook*, ch. 2).
//! * **Rank regression** on Weibull paper, `y = ln(−ln(1 − F))` against
//!   `x = ln t`: RRX regresses x on y (Abernethy's recommendation for small
//!   samples), RRY regresses y on x. `β` is the slope and `η` the time at
//!   F = 63.2 %.
//! * **Maximum likelihood (MLE)** — failures contribute ln f(t), suspensions
//!   ln R(t). With `a = ln η` the score equation in `a` gives
//!   `η^β = Σ t^β / r` (r failures) and β solves the profile equation
//!   `1/β + mean_f(ln t) − Σ t^β ln t / Σ t^β = 0` (monotone, bisection).
//! * **Weibayes** — β assumed known: `η = (Σ t^β / r)^{1/β}`; with no failures
//!   one failure is assumed imminent (a 63.2 % lower confidence bound).
//! * **Confidence bounds on time** at each unreliability F: **Fisher matrix**
//!   (inverse observed information in (ln η, β), delta method on
//!   `u = ln t_p = ln η + y_p/β`, normal on u) or **likelihood ratio**
//!   (profile likelihood: all t_p with `2(ℓ̂ − ℓ*(t_p)) ≤ χ²₁(CL)`), two-sided at
//!   the stated confidence level (Meeker & Escobar, *Statistical Methods for
//!   Reliability Data*, ch. 8).
//!
//! Checked against the Lieblein–Zelen ball-bearing data (23 failures,
//! millions of revolutions), whose MLE is β = 2.102, η = 81.88 (Lawless,
//! *Statistical Models and Methods for Lifetime Data*).

use serde::{Deserialize, Serialize};

use super::stats;
use crate::math::Rng;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Obs {
    pub t: f64,
    /// true = failure, false = suspension (right-censored).
    #[serde(default = "yes")]
    pub failed: bool,
}

fn yes() -> bool {
    true
}

/// Parse pasted text or CSV into observations.
///
/// Accepted per line: `123` (failure), `123+` / `123 s` / `123,S` /
/// `123,0` / `123,suspended` (suspension), `123,1` / `123,F` (failure), an
/// optional third column with a count (grouped data), `#` comments and a
/// non-numeric header line. Several bare numbers on one line are all failures.
pub fn parse_text(s: &str) -> Result<Vec<Obs>, String> {
    let mut out = Vec::new();
    for (ln, raw) in s.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let toks: Vec<&str> = line
            .split(|c: char| c == ',' || c == ';' || c == '\t' || c.is_whitespace())
            .filter(|t| !t.is_empty())
            .collect();
        if toks.is_empty() {
            continue;
        }
        let first = toks[0].trim_end_matches('+');
        let t: f64 = match first.parse() {
            Ok(v) => v,
            Err(_) => {
                if out.is_empty() && ln < 3 {
                    continue; // header
                }
                return Err(format!("line {}: can't read a time from \"{}\"", ln + 1, raw.trim()));
            }
        };
        let status = |tok: &str| -> Option<bool> {
            match tok.to_ascii_lowercase().as_str() {
                "1" | "f" | "fail" | "failed" | "failure" | "x" => Some(true),
                "0" | "s" | "c" | "susp" | "suspended" | "suspension" | "censored" | "r" | "+" => Some(false),
                _ => None,
            }
        };
        let plus = toks[0].ends_with('+');
        if toks.len() >= 2 {
            if let Some(f) = status(toks[1]) {
                let count = toks
                    .get(2)
                    .and_then(|c| c.parse::<f64>().ok())
                    .unwrap_or(1.0)
                    .round()
                    .clamp(1.0, 10_000.0) as usize;
                for _ in 0..count {
                    out.push(Obs { t, failed: f && !plus });
                }
                continue;
            }
            // several numbers: each a failure (a trailing + marks a suspension)
            for tok in &toks {
                let v: f64 = tok
                    .trim_end_matches('+')
                    .parse()
                    .map_err(|_| format!("line {}: can't read \"{}\"", ln + 1, tok))?;
                out.push(Obs {
                    t: v,
                    failed: !tok.ends_with('+'),
                });
            }
            continue;
        }
        out.push(Obs { t, failed: !plus });
        if out.len() > 20_000 {
            return Err("more than 20 000 observations".into());
        }
    }
    Ok(out)
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct RankRow {
    pub t: f64,
    pub failed: bool,
    pub reverse_rank: usize,
    pub adjusted_rank: Option<f64>,
    /// Bernard median rank (plotting position F).
    pub median_rank: Option<f64>,
}

/// Sort (failures before suspensions at equal times) and compute Johnson's
/// adjusted ranks and Bernard's median ranks.
pub fn ranks(data: &[Obs]) -> Vec<RankRow> {
    let mut d: Vec<&Obs> = data.iter().collect();
    d.sort_by(|a, b| a.t.total_cmp(&b.t).then(b.failed.cmp(&a.failed)));
    let n = d.len();
    let nf = n as f64;
    let mut prev = 0.0;
    d.iter()
        .enumerate()
        .map(|(i, o)| {
            let rr = n - i;
            if o.failed {
                let adj = (rr as f64 * prev + nf + 1.0) / (rr as f64 + 1.0);
                prev = adj;
                RankRow {
                    t: o.t,
                    failed: true,
                    reverse_rank: rr,
                    adjusted_rank: Some(adj),
                    median_rank: Some((adj - 0.3) / (nf + 0.4)),
                }
            } else {
                RankRow {
                    t: o.t,
                    failed: false,
                    reverse_rank: rr,
                    adjusted_rank: None,
                    median_rank: None,
                }
            }
        })
        .collect()
}

/// Weibull paper ordinate y = ln(−ln(1 − F)).
pub fn y_of_f(f: f64) -> f64 {
    (-(1.0 - f).ln()).ln()
}

pub fn cdf(t: f64, beta: f64, eta: f64) -> f64 {
    if t <= 0.0 {
        0.0
    } else {
        1.0 - (-(t / eta).powf(beta)).exp()
    }
}

/// Time at unreliability F.
pub fn quantile(f: f64, beta: f64, eta: f64) -> f64 {
    eta * (-(1.0 - f).ln()).powf(1.0 / beta)
}

/// Log-likelihood (up to the constant Σ_f ln t) in (β, a = ln η).
pub fn loglik(data: &[Obs], beta: f64, a: f64) -> f64 {
    let mut l = 0.0;
    for o in data {
        let s = o.t.ln() - a;
        let w = (beta * s).exp();
        if o.failed {
            l += beta.ln() - a + (beta - 1.0) * s;
        }
        l -= w;
    }
    l
}

/// Least-squares fit on Weibull paper. Returns (β, η, ρ²).
pub fn rank_regression(pts: &[(f64, f64)], x_on_y: bool) -> Result<(f64, f64, f64), String> {
    if pts.len() < 2 {
        return Err("rank regression needs at least two failures".into());
    }
    let xs: Vec<f64> = pts.iter().map(|p| p.0.ln()).collect();
    let ys: Vec<f64> = pts.iter().map(|p| y_of_f(p.1)).collect();
    let n = xs.len() as f64;
    let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    for (x, y) in xs.iter().zip(&ys) {
        sxx += (x - mx) * (x - mx);
        syy += (y - my) * (y - my);
        sxy += (x - mx) * (y - my);
    }
    if sxx <= 0.0 || syy <= 0.0 {
        return Err("rank regression needs failures at two or more different times".into());
    }
    let r2 = sxy * sxy / (sxx * syy);
    let (beta, a) = if x_on_y {
        // x = a + y/β
        let m = sxy / syy;
        (1.0 / m, mx - m * my)
    } else {
        // y = β x − β a
        let b = sxy / sxx;
        (b, mx - my / b)
    };
    if !(beta > 0.0 && beta.is_finite()) {
        return Err("rank regression gave a non-positive slope (failures don't increase with time)".into());
    }
    Ok((beta, a.exp(), r2))
}

/// Maximum-likelihood estimate (β, η) for right-censored data.
pub fn mle(data: &[Obs]) -> Result<(f64, f64), String> {
    let r = data.iter().filter(|o| o.failed).count();
    if r < 1 {
        return Err("maximum likelihood needs at least one failure (use Weibayes with an assumed β)".into());
    }
    let tmax = data.iter().map(|o| o.t).fold(0.0, f64::max);
    let lx: Vec<f64> = data.iter().map(|o| (o.t / tmax).ln()).collect();
    let mean_f = data.iter().zip(&lx).filter(|(o, _)| o.failed).map(|(_, l)| l).sum::<f64>() / r as f64;
    let g = |b: f64| -> f64 {
        let (mut s0, mut s1) = (0.0, 0.0);
        for l in &lx {
            let w = (b * l).exp();
            s0 += w;
            s1 += w * l;
        }
        1.0 / b + mean_f - s1 / s0
    };
    let (mut lo, mut hi) = (1e-3f64.ln(), 500f64.ln());
    if g(lo.exp()) < 0.0 || g(hi.exp()) > 0.0 {
        return Err("maximum likelihood did not converge (all failures at the same time as the longest run?)".into());
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if g(mid.exp()) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi - lo < 1e-14 {
            break;
        }
    }
    let beta = (0.5 * (lo + hi)).exp();
    let s: f64 = lx.iter().map(|l| (beta * l).exp()).sum();
    let eta = tmax * (s / r as f64).powf(1.0 / beta);
    Ok((beta, eta))
}

/// Weibayes η for an assumed β. Returns (η, failures used, assumed-one flag).
pub fn weibayes(data: &[Obs], beta: f64) -> (f64, usize, bool) {
    let r = data.iter().filter(|o| o.failed).count();
    let s: f64 = data.iter().map(|o| o.t.powf(beta)).sum();
    let used = r.max(1);
    ((s / used as f64).powf(1.0 / beta), r, r == 0)
}

/// Covariance of (a = ln η, β) from the inverse observed Fisher information.
pub fn fisher_cov(data: &[Obs], beta: f64, eta: f64) -> Option<[[f64; 2]; 2]> {
    let a = eta.ln();
    let r = data.iter().filter(|o| o.failed).count() as f64;
    let (mut sw, mut ssw, mut s2w) = (0.0, 0.0, 0.0);
    for o in data {
        let s = o.t.ln() - a;
        let w = (beta * s).exp();
        sw += w;
        ssw += s * w;
        s2w += s * s * w;
    }
    // information = −Hessian
    let iaa = beta * beta * sw;
    let iab = r - sw - beta * ssw;
    let ibb = r / (beta * beta) + s2w;
    let det = iaa * ibb - iab * iab;
    if !(det > 0.0 && iaa > 0.0 && ibb > 0.0) {
        return None;
    }
    Some([[ibb / det, -iab / det], [-iab / det, iaa / det]])
}

/// Fisher-matrix two-sided bounds on the time at unreliability F.
pub fn fisher_time_bounds(v: &[[f64; 2]; 2], beta: f64, eta: f64, f: f64, z: f64) -> (f64, f64) {
    let y = y_of_f(f);
    let u = eta.ln() + y / beta;
    let d = -y / (beta * beta);
    let var = (v[0][0] + d * d * v[1][1] + 2.0 * d * v[0][1]).max(0.0);
    let s = var.sqrt();
    ((u - z * s).exp(), (u + z * s).exp())
}

/// Profile log-likelihood at fixed u = ln t_F (maximised over β by golden section on ln β).
fn profile_at_u(data: &[Obs], y: f64, u: f64, beta_hat: f64) -> f64 {
    let f = |lb: f64| {
        let b = lb.exp();
        loglik(data, b, u - y / b)
    };
    let (mut lo, mut hi) = (beta_hat.ln() - 3.5, beta_hat.ln() + 3.5);
    let gr = 0.618_033_988_749_895;
    let mut c = hi - gr * (hi - lo);
    let mut d = lo + gr * (hi - lo);
    let (mut fc, mut fd) = (f(c), f(d));
    for _ in 0..60 {
        if fc > fd {
            hi = d;
            d = c;
            fd = fc;
            c = hi - gr * (hi - lo);
            fc = f(c);
        } else {
            lo = c;
            c = d;
            fc = fd;
            d = lo + gr * (hi - lo);
            fd = f(d);
        }
        if hi - lo < 1e-7 {
            break;
        }
    }
    fc.max(fd)
}

/// Likelihood-ratio two-sided bounds on the time at unreliability F around the MLE.
pub fn lr_time_bounds(data: &[Obs], beta: f64, eta: f64, f: f64, cl: f64, sd_hint: f64) -> (f64, f64) {
    let lmax = loglik(data, beta, eta.ln());
    let crit = lmax - 0.5 * stats::chi2_quantile(cl, 1.0);
    let y = y_of_f(f);
    let u0 = eta.ln() + y / beta;
    let g = |u: f64| profile_at_u(data, y, u, beta) - crit;
    let step = sd_hint.clamp(1e-3, 5.0);
    let find = |dir: f64| -> f64 {
        let mut inner = u0;
        let mut outer = u0 + dir * step;
        let mut k = 0;
        while g(outer) > 0.0 && k < 40 {
            inner = outer;
            outer = u0 + dir * step * 2f64.powi(k + 1);
            k += 1;
        }
        for _ in 0..50 {
            let mid = 0.5 * (inner + outer);
            if g(mid) > 0.0 {
                inner = mid;
            } else {
                outer = mid;
            }
            if (outer - inner).abs() < 1e-6 {
                break;
            }
        }
        (0.5 * (inner + outer)).exp()
    };
    (find(-1.0), find(1.0))
}

/// Likelihood-ratio bounds on β (profile over η, closed form).
fn lr_beta_bounds(data: &[Obs], beta: f64, eta: f64, cl: f64) -> (f64, f64) {
    let r = data.iter().filter(|o| o.failed).count() as f64;
    let lmax = loglik(data, beta, eta.ln());
    let crit = lmax - 0.5 * stats::chi2_quantile(cl, 1.0);
    let prof = |b: f64| {
        let s: f64 = data.iter().map(|o| o.t.powf(b)).sum();
        let a = (s / r).ln() / b;
        loglik(data, b, a) - crit
    };
    let solve = |mut inner: f64, mut outer: f64| {
        for _ in 0..80 {
            let mid = (inner * outer).sqrt();
            if prof(mid) > 0.0 {
                inner = mid;
            } else {
                outer = mid;
            }
        }
        (inner * outer).sqrt()
    };
    let lo = if prof(beta / 50.0) < 0.0 {
        solve(beta, beta / 50.0)
    } else {
        beta / 50.0
    };
    let hi = if prof(beta * 50.0) < 0.0 {
        solve(beta, beta * 50.0)
    } else {
        beta * 50.0
    };
    (lo, hi)
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct Fit {
    pub method: String,
    pub label: String,
    pub ok: bool,
    pub error: String,
    pub beta: f64,
    pub eta: f64,
    /// Coefficient of determination on Weibull paper (rank regression only).
    pub r2: Option<f64>,
    pub loglik: f64,
    pub mttf: f64,
    pub b1: f64,
    pub b10: f64,
    pub b50: f64,
}

fn make_fit(method: &str, label: &str, data: &[Obs], res: Result<(f64, f64, Option<f64>), String>) -> Fit {
    match res {
        Ok((beta, eta, r2)) => Fit {
            method: method.into(),
            label: label.into(),
            ok: true,
            error: String::new(),
            beta,
            eta,
            r2,
            loglik: loglik(data, beta, eta.ln()),
            mttf: eta * stats::gamma(1.0 + 1.0 / beta),
            b1: quantile(0.01, beta, eta),
            b10: quantile(0.10, beta, eta),
            b50: quantile(0.50, beta, eta),
        },
        Err(e) => Fit {
            method: method.into(),
            label: label.into(),
            ok: false,
            error: e,
            ..Default::default()
        },
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct FitIn {
    /// Observations; used when `text` is empty.
    pub data: Vec<Obs>,
    /// Pasted text or CSV (see [`parse_text`]).
    pub text: String,
    /// `rrx` | `rry` | `mle` | `weibayes`
    pub method: String,
    /// `fisher` | `lr` | `none`
    pub bounds: String,
    /// Two-sided confidence level, e.g. 0.9.
    pub cl: f64,
    /// β for Weibayes.
    pub beta_assumed: f64,
    pub unit: String,
}

impl Default for FitIn {
    fn default() -> Self {
        FitIn {
            data: vec![],
            text: sample_sets()[0].text.clone(),
            method: "mle".into(),
            bounds: "fisher".into(),
            cl: 0.9,
            beta_assumed: 2.0,
            unit: String::new(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct BLife {
    pub p: f64,
    pub t: f64,
    pub lo: Option<f64>,
    pub hi: Option<f64>,
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct Curves {
    pub t: Vec<f64>,
    pub pdf: Vec<f64>,
    pub cdf: Vec<f64>,
    pub rel: Vec<f64>,
    pub hazard: Vec<f64>,
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct FitOut {
    pub n: usize,
    pub failures: usize,
    pub suspensions: usize,
    pub unit: String,
    pub rows: Vec<RankRow>,
    pub fits: Vec<Fit>,
    pub method: String,
    pub beta: f64,
    pub eta: f64,
    pub beta_bounds: Option<[f64; 2]>,
    pub eta_bounds: Option<[f64; 2]>,
    pub cl: f64,
    pub bounds_method: String,
    pub b_lives: Vec<BLife>,
    /// Fit line and bounds on Weibull paper at these unreliabilities.
    pub line_f: Vec<f64>,
    pub line_t: Vec<f64>,
    pub bound_lo: Vec<f64>,
    pub bound_hi: Vec<f64>,
    pub curves: Curves,
    pub mttf: f64,
    pub interpretation: String,
    pub notes: Vec<String>,
}

/// Fit a Weibull distribution and compute bounds and curves.
pub fn fit(i: FitIn) -> Result<FitOut, String> {
    let data = if i.text.trim().is_empty() {
        i.data.clone()
    } else {
        parse_text(&i.text)?
    };
    if data.is_empty() {
        return Err("enter at least two failure times".into());
    }
    if data.iter().any(|o| !(o.t > 0.0 && o.t.is_finite())) {
        return Err("times must be positive numbers".into());
    }
    let cl = i.cl.clamp(0.5, 0.999);
    let z = stats::norm_quantile(0.5 + 0.5 * cl);
    let rows = ranks(&data);
    let pts: Vec<(f64, f64)> = rows.iter().filter_map(|r| r.median_rank.map(|f| (r.t, f))).collect();
    let failures = pts.len();
    let mut notes = Vec::new();
    let rrx = make_fit(
        "rrx",
        "Rank regression on X (RRX)",
        &data,
        rank_regression(&pts, true).map(|(b, e, r)| (b, e, Some(r))),
    );
    let rry = make_fit(
        "rry",
        "Rank regression on Y (RRY)",
        &data,
        rank_regression(&pts, false).map(|(b, e, r)| (b, e, Some(r))),
    );
    let mle_fit = make_fit(
        "mle",
        "Maximum likelihood (MLE)",
        &data,
        if failures >= 2 {
            mle(&data).map(|(b, e)| (b, e, None))
        } else {
            Err("maximum likelihood needs at least two failures".into())
        },
    );
    let wb_beta = i.beta_assumed.clamp(0.1, 20.0);
    let (wb_eta, wb_r, wb_assumed) = weibayes(&data, wb_beta);
    let wb = make_fit(
        "weibayes",
        &format!("Weibayes (β = {wb_beta} assumed)"),
        &data,
        Ok((wb_beta, wb_eta, None)),
    );
    if wb_assumed && i.method == "weibayes" {
        notes.push(
            "No failures: Weibayes assumes one failure is imminent, so η is a 63.2 % lower confidence bound (Abernethy ch. 6).".into(),
        );
    }
    let fits = vec![rrx, rry, mle_fit, wb];
    let sel = fits
        .iter()
        .find(|f| f.method == i.method && f.ok)
        .or_else(|| fits.iter().find(|f| f.method == "mle" && f.ok))
        .or_else(|| fits.iter().find(|f| f.ok))
        .cloned()
        .ok_or_else(|| "no fit possible".to_string())?;
    if sel.method != i.method {
        notes.push(format!(
            "{} not possible here ({}); showing {}.",
            i.method.to_uppercase(),
            fits.iter()
                .find(|f| f.method == i.method)
                .map(|f| f.error.clone())
                .unwrap_or_default(),
            sel.label
        ));
    }
    let (beta, eta) = (sel.beta, sel.eta);

    // Line on Weibull paper and bounds.
    let line_f: Vec<f64> = (0..=60)
        .map(|k| 1.0 - (-(10f64.powf(-3.0 + 3.84 * k as f64 / 60.0))).exp())
        .map(|f: f64| f.clamp(1e-4, 0.9999))
        .collect();
    let line_t: Vec<f64> = line_f.iter().map(|&f| quantile(f, beta, eta)).collect();
    let mut bound_lo = vec![];
    let mut bound_hi = vec![];
    let mut beta_bounds = None;
    let mut eta_bounds = None;
    let mut bounds_method = i.bounds.clone();
    let mle_ok = fits[2].ok;
    let mut cov = None;
    if sel.method == "weibayes" {
        bounds_method = "chi2".into();
        let s: f64 = data.iter().map(|o| o.t.powf(beta)).sum();
        let lo = (2.0 * s / stats::chi2_quantile(0.5 + 0.5 * cl, 2.0 * wb_r as f64 + 2.0)).powf(1.0 / beta);
        let hi = if wb_r > 0 {
            (2.0 * s / stats::chi2_quantile(0.5 - 0.5 * cl, 2.0 * wb_r as f64)).powf(1.0 / beta)
        } else {
            f64::INFINITY
        };
        eta_bounds = Some([lo, hi]);
        if i.bounds != "none" {
            bound_lo = line_f.iter().map(|&f| quantile(f, beta, lo)).collect();
            bound_hi = line_f
                .iter()
                .map(|&f| if hi.is_finite() { quantile(f, beta, hi) } else { f64::NAN })
                .collect();
        }
        notes.push("Weibayes bounds: η from the chi-square distribution with 2r + 2 degrees of freedom (β treated as exact).".into());
    } else if i.bounds == "lr" && mle_ok && data.len() > 400 {
        notes.push("Likelihood-ratio bounds are limited to 400 observations in the browser; Fisher-matrix bounds are drawn instead.".into());
        bounds_method = "fisher".into();
        cov = fisher_cov(&data, beta, eta);
        if let Some(v) = cov {
            for &f in &line_f {
                let (lo, hi) = fisher_time_bounds(&v, beta, eta, f, z);
                bound_lo.push(lo);
                bound_hi.push(hi);
            }
        }
    } else if i.bounds == "lr" && mle_ok {
        let (mb, me) = (fits[2].beta, fits[2].eta);
        let v = fisher_cov(&data, mb, me);
        cov = v;
        for &f in &line_f {
            let sd = v.map(|v| {
                let y = y_of_f(f);
                let d = -y / (mb * mb);
                (v[0][0] + d * d * v[1][1] + 2.0 * d * v[0][1]).max(1e-12).sqrt()
            });
            let (lo, hi) = lr_time_bounds(&data, mb, me, f, cl, sd.unwrap_or(0.3));
            bound_lo.push(lo);
            bound_hi.push(hi);
        }
        let bb = lr_beta_bounds(&data, mb, me, cl);
        beta_bounds = Some([bb.0, bb.1]);
        let e63 = lr_time_bounds(&data, mb, me, 1.0 - (-1f64).exp(), cl, v.map(|v| v[0][0].sqrt()).unwrap_or(0.3));
        eta_bounds = Some([e63.0, e63.1]);
        if sel.method != "mle" {
            notes.push("Likelihood-ratio bounds belong to the maximum-likelihood fit, so they are drawn around the MLE line.".into());
        }
    } else if i.bounds != "none" {
        bounds_method = "fisher".into();
        cov = fisher_cov(&data, beta, eta);
        match cov {
            Some(v) => {
                for &f in &line_f {
                    let (lo, hi) = fisher_time_bounds(&v, beta, eta, f, z);
                    bound_lo.push(lo);
                    bound_hi.push(hi);
                }
                let sb = v[1][1].sqrt();
                beta_bounds = Some([beta * (-z * sb / beta).exp(), beta * (z * sb / beta).exp()]);
                let sa = v[0][0].sqrt();
                eta_bounds = Some([eta * (-z * sa).exp(), eta * (z * sa).exp()]);
                if sel.method != "mle" {
                    notes.push("Fisher-matrix bounds use the likelihood's information evaluated at the rank-regression estimates (as commercial tools do).".into());
                }
            }
            None => notes.push("The Fisher information is not positive definite at these estimates, so no Fisher bounds are drawn.".into()),
        }
    } else {
        bounds_method = "none".into();
    }
    if i.bounds == "lr" && !mle_ok && sel.method != "weibayes" {
        notes.push("Likelihood-ratio bounds need a maximum-likelihood fit (two or more failures).".into());
    }

    // B-lives with bounds.
    let b_lives = [0.001, 0.01, 0.1, 0.5]
        .iter()
        .map(|&p| {
            let t = quantile(p, beta, eta);
            let (lo, hi) = match bounds_method.as_str() {
                "fisher" => cov
                    .map(|v| fisher_time_bounds(&v, beta, eta, p, z))
                    .map(|(a, b)| (Some(a), Some(b)))
                    .unwrap_or((None, None)),
                "lr" => {
                    let (lo, hi) = lr_time_bounds(&data, fits[2].beta, fits[2].eta, p, cl, 0.3);
                    (Some(lo), Some(hi))
                }
                "chi2" => eta_bounds
                    .map(|e| {
                        (
                            Some(quantile(p, beta, e[0])),
                            if e[1].is_finite() { Some(quantile(p, beta, e[1])) } else { None },
                        )
                    })
                    .unwrap_or((None, None)),
                _ => (None, None),
            };
            BLife { p, t, lo, hi }
        })
        .collect();

    // pdf / hazard / reliability curves up to the 99.9 % life.
    let tmax = quantile(0.999, beta, eta).max(data.iter().map(|o| o.t).fold(0.0, f64::max));
    let mut curves = Curves::default();
    for k in 0..=200 {
        let t = tmax * k as f64 / 200.0;
        let x = t / eta;
        let rel = (-x.powf(beta)).exp();
        let h = if t > 0.0 {
            beta / eta * x.powf(beta - 1.0)
        } else if beta < 1.0 {
            f64::NAN
        } else if beta == 1.0 {
            1.0 / eta
        } else {
            0.0
        };
        curves.t.push(t);
        curves.rel.push(rel);
        curves.cdf.push(1.0 - rel);
        curves.hazard.push(h);
        curves.pdf.push(if h.is_finite() { h * rel } else { f64::NAN });
    }
    let interpretation = if beta < 0.95 {
        format!("β = {beta:.2} < 1: failure rate falls with age — infant mortality (manufacturing or assembly defects). Burn-in or better process control helps; more testing time does not prove wear-out life.")
    } else if beta <= 1.1 {
        format!("β = {beta:.2} ≈ 1: constant failure rate — random failures (overstress events, mixed modes). Here MTBF = η is meaningful.")
    } else if beta <= 4.0 {
        format!("β = {beta:.2} > 1: failure rate rises with age — wear-out (fatigue, wear, corrosion). Plan replacement or design margin before the B10 life.")
    } else {
        format!("β = {beta:.2} ≫ 1: rapid wear-out — the population fails in a narrow band (typical of some fatigue and capacity-fade modes). Check whether a three-parameter (failure-free period) model fits better.")
    };
    if failures < 5 {
        notes.push("Fewer than five failures: β is very uncertain. Abernethy recommends rank regression (RRX) or Weibayes with a known β for small samples.".into());
    }
    Ok(FitOut {
        n: data.len(),
        failures,
        suspensions: data.len() - failures,
        unit: i.unit,
        rows,
        fits,
        method: sel.method.clone(),
        beta,
        eta,
        beta_bounds,
        eta_bounds,
        cl,
        bounds_method,
        b_lives,
        line_f,
        line_t,
        bound_lo,
        bound_hi,
        curves,
        mttf: sel.mttf,
        interpretation,
        notes,
    })
}

/// Draw `n` Weibull lifetimes; units still running at `censor_at` are suspended.
pub fn generate(beta: f64, eta: f64, n: usize, censor_at: Option<f64>, seed: u64) -> Vec<Obs> {
    let mut rng = Rng::new(seed);
    let mut v: Vec<Obs> = (0..n)
        .map(|_| {
            let u = rng.uniform().max(1e-300);
            let t = eta * (-(1.0 - u).ln()).powf(1.0 / beta);
            match censor_at {
                Some(c) if t > c => Obs { t: c, failed: false },
                _ => Obs {
                    t: t.max(1e-9),
                    failed: true,
                },
            }
        })
        .collect();
    v.sort_by(|a, b| a.t.total_cmp(&b.t));
    v
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct StudyIn {
    pub beta: f64,
    pub eta: f64,
    pub n: usize,
    /// Fraction expected to fail before the test stops (1 = run to failure).
    pub fail_fraction: f64,
    pub reps: usize,
    pub seed: u64,
}

impl Default for StudyIn {
    fn default() -> Self {
        StudyIn {
            beta: 2.0,
            eta: 1000.0,
            n: 8,
            fail_fraction: 1.0,
            reps: 300,
            seed: 1,
        }
    }
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct MethodStats {
    pub method: String,
    pub betas: Vec<f64>,
    pub etas: Vec<f64>,
    pub beta_median: f64,
    pub beta_mean: f64,
    pub beta_p05: f64,
    pub beta_p95: f64,
    pub eta_median: f64,
    pub b10_median: f64,
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct StudyOut {
    pub true_beta: f64,
    pub true_eta: f64,
    pub true_b10: f64,
    pub censor_at: Option<f64>,
    pub example: Vec<Obs>,
    pub methods: Vec<MethodStats>,
    pub skipped: usize,
}

fn pct(v: &[f64], p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    let x = p * (s.len() - 1) as f64;
    let i = x.floor() as usize;
    let f = x - i as f64;
    s[i] * (1.0 - f) + s[(i + 1).min(s.len() - 1)] * f
}

/// Monte Carlo study: how well do RRX, RRY and MLE recover a known Weibull
/// from small, possibly censored samples?
pub fn study(i: StudyIn) -> Result<StudyOut, String> {
    if !(i.beta > 0.0 && i.eta > 0.0) {
        return Err("β and η must be positive".into());
    }
    let n = i.n.clamp(3, 200);
    let reps = i.reps.clamp(10, 2000);
    let ff = i.fail_fraction.clamp(0.05, 1.0);
    let censor_at = if ff < 0.999 { Some(quantile(ff, i.beta, i.eta)) } else { None };
    let names = ["rrx", "rry", "mle"];
    let mut res: Vec<MethodStats> = names
        .iter()
        .map(|m| MethodStats {
            method: m.to_string(),
            ..Default::default()
        })
        .collect();
    let mut skipped = 0;
    let mut example = vec![];
    for k in 0..reps {
        let d = generate(i.beta, i.eta, n, censor_at, i.seed.wrapping_mul(7919).wrapping_add(k as u64));
        if k == 0 {
            example = d.clone();
        }
        let rows = ranks(&d);
        let pts: Vec<(f64, f64)> = rows.iter().filter_map(|r| r.median_rank.map(|f| (r.t, f))).collect();
        let (a, b, c) = (
            rank_regression(&pts, true),
            rank_regression(&pts, false),
            if pts.len() >= 2 { mle(&d) } else { Err(String::new()) },
        );
        match (a, b, c) {
            (Ok(a), Ok(b), Ok(c)) => {
                for (m, (be, et)) in res.iter_mut().zip([(a.0, a.1), (b.0, b.1), c]) {
                    m.betas.push(be);
                    m.etas.push(et);
                }
            }
            _ => skipped += 1,
        }
    }
    for m in res.iter_mut() {
        m.beta_median = pct(&m.betas, 0.5);
        m.beta_mean = m.betas.iter().sum::<f64>() / m.betas.len().max(1) as f64;
        m.beta_p05 = pct(&m.betas, 0.05);
        m.beta_p95 = pct(&m.betas, 0.95);
        m.eta_median = pct(&m.etas, 0.5);
        let b10: Vec<f64> = m.betas.iter().zip(&m.etas).map(|(b, e)| quantile(0.1, *b, *e)).collect();
        m.b10_median = pct(&b10, 0.5);
    }
    Ok(StudyOut {
        true_beta: i.beta,
        true_eta: i.eta,
        true_b10: quantile(0.1, i.beta, i.eta),
        censor_at,
        example,
        methods: res,
        skipped,
    })
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct BathtubIn {
    pub infant_beta: f64,
    pub infant_eta: f64,
    pub random_eta: f64,
    pub wear_beta: f64,
    pub wear_eta: f64,
    pub horizon: f64,
    pub units: usize,
    pub seed: u64,
}

impl Default for BathtubIn {
    fn default() -> Self {
        BathtubIn {
            infant_beta: 0.5,
            infant_eta: 40_000.0,
            random_eta: 60_000.0,
            wear_beta: 5.0,
            wear_eta: 30_000.0,
            horizon: 40_000.0,
            units: 240,
            seed: 3,
        }
    }
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct BathtubUnit {
    pub t: f64,
    /// 0 infant, 1 random, 2 wear-out
    pub mode: u8,
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct BathtubOut {
    pub t: Vec<f64>,
    pub h_infant: Vec<f64>,
    pub h_random: Vec<f64>,
    pub h_wear: Vec<f64>,
    pub h_total: Vec<f64>,
    pub rel: Vec<f64>,
    pub units: Vec<BathtubUnit>,
}

/// The bathtub curve as three competing Weibull modes (β < 1, β = 1, β > 1):
/// the total hazard is their sum; each simulated unit fails at the earliest
/// of its three mode lifetimes.
pub fn bathtub(i: BathtubIn) -> Result<BathtubOut, String> {
    if [i.infant_beta, i.infant_eta, i.random_eta, i.wear_beta, i.wear_eta, i.horizon]
        .iter()
        .any(|v| v.is_nan() || *v <= 0.0)
    {
        return Err("all parameters must be positive".into());
    }
    let hz = |t: f64, b: f64, e: f64| b / e * (t / e).powf(b - 1.0);
    let mut o = BathtubOut::default();
    for k in 1..=240 {
        let t = i.horizon * k as f64 / 240.0;
        let (a, b, c) = (
            hz(t, i.infant_beta, i.infant_eta),
            1.0 / i.random_eta,
            hz(t, i.wear_beta, i.wear_eta),
        );
        o.t.push(t);
        o.h_infant.push(a);
        o.h_random.push(b);
        o.h_wear.push(c);
        o.h_total.push(a + b + c);
        o.rel
            .push((-(t / i.infant_eta).powf(i.infant_beta) - t / i.random_eta - (t / i.wear_eta).powf(i.wear_beta)).exp());
    }
    let mut rng = Rng::new(i.seed);
    for _ in 0..i.units.clamp(10, 2000) {
        let mut draw = |b: f64, e: f64| e * (-(1.0 - rng.uniform()).max(1e-300).ln()).powf(1.0 / b);
        let ts = [
            draw(i.infant_beta, i.infant_eta),
            draw(1.0, i.random_eta),
            draw(i.wear_beta, i.wear_eta),
        ];
        let (mode, t) = ts
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(b.1))
            .map(|(m, t)| (m as u8, *t))
            .unwrap_or((0, 0.0));
        o.units.push(BathtubUnit { t, mode });
    }
    Ok(o)
}

#[derive(Serialize, Debug, Clone)]
pub struct SampleSet {
    pub id: String,
    pub title: String,
    pub unit: String,
    pub source: String,
    pub description: String,
    pub text: String,
}

/// Sample data sets. Only the ball-bearing set is real published data; the
/// Iris sets are synthetic (drawn from a known Weibull) and labelled so.
pub fn sample_sets() -> Vec<SampleSet> {
    let mk = |id: &str, title: &str, unit: &str, source: &str, description: &str, text: &str| SampleSet {
        id: id.into(),
        title: title.into(),
        unit: unit.into(),
        source: source.into(),
        description: description.into(),
        text: text.into(),
    };
    vec![
        mk(
            "hinge",
            "Iris spring hinge — open/close cycle test",
            "cycles",
            "Illustrative, synthetic data (drawn from β = 3, η = 60 000 cycles; seed fixed). Replace with your own rig results.",
            "Ten hinges cycled on a fixture; the test stopped at 50 000 cycles with six hinges still working (suspensions, marked +).",
            "# cycles to failure; + = still working when the test stopped\n28700\n34900\n36400\n43700\n50000+\n50000+\n50000+\n50000+\n50000+\n50000+\n",
        ),
        mk(
            "battery",
            "LiPo cell — capacity fade to 80 %",
            "cycles",
            "Illustrative, synthetic data (drawn from β = 4, η = 800 cycles; seed fixed). Real cells: fit your own cycling data.",
            "Eight 180 mAh cells charge/discharge cycled; 'failure' = capacity below 80 % of rated. Three cells were pulled at 700 cycles still above 80 %.",
            "time,status\n365,F\n375,F\n570,F\n605,F\n660,F\n700,S\n700,S\n700,S\n",
        ),
        mk(
            "flex",
            "Hinge flex cable — fold test",
            "cycles",
            "Illustrative, synthetic data (drawn from β = 2.2, η = 65 000 folds; seed fixed).",
            "Twelve flat flex cables folded through the hinge until an open conductor; three survived to 60 000 folds. Iris risk F-2 asks for this test in DVT.",
            "21700\n22700\n24900\n29400\n35900\n47300\n50200\n53500\n56200\n60000+\n60000+\n60000+\n",
        ),
        mk(
            "infant",
            "Early field returns — infant mortality",
            "hours",
            "Illustrative, synthetic data (drawn from β = 0.6, η = 4 000 h; seed fixed).",
            "Sixteen units on a 3 000 h reliability-growth run; failures cluster early — the signature of assembly defects (β < 1).",
            "44\n49\n108\n148\n280\n306\n481\n812\n1698\n3000+\n3000+\n3000+\n3000+\n3000+\n3000+\n3000+\n",
        ),
        mk(
            "bearings",
            "Deep-groove ball bearings (Lieblein & Zelen 1956)",
            "million revolutions",
            "Published data: J. Lieblein and M. Zelen, J. Res. NBS 57(5), 1956 — the classic complete Weibull sample (MLE β ≈ 2.10, η ≈ 81.9).",
            "23 bearings run to fatigue failure.",
            "17.88\n28.92\n33.00\n41.52\n42.12\n45.60\n48.48\n51.84\n51.96\n54.12\n55.56\n67.80\n68.64\n68.64\n68.88\n84.12\n93.12\n98.64\n105.12\n105.84\n127.92\n128.04\n173.40\n",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bearings() -> Vec<Obs> {
        parse_text(&sample_sets().into_iter().find(|s| s.id == "bearings").unwrap().text).unwrap()
    }

    #[test]
    fn mle_matches_published_bearing_fit() {
        let (b, e) = mle(&bearings()).unwrap();
        assert!((b - 2.102).abs() < 2e-3, "β = {b}");
        assert!((e - 81.88).abs() < 0.02, "η = {e}");
    }

    #[test]
    fn mle_matches_scipy_on_censored_hinge_set() {
        // Independent check: scipy Nelder–Mead on the same censored likelihood gives β = 3.440, η = 59 797.
        let d = parse_text(&sample_sets()[0].text).unwrap();
        assert_eq!(d.iter().filter(|o| !o.failed).count(), 6);
        let (b, e) = mle(&d).unwrap();
        assert!((b - 3.44).abs() < 0.01, "β = {b}");
        assert!((e - 59_797.0).abs() < 60.0, "η = {e}");
    }

    #[test]
    fn johnson_adjusted_ranks_textbook() {
        // 5 items: F 10, S 20, F 30, S 40, F 50.  Adjusted ranks 1, 2.25, 4.125 (Abernethy's method).
        let d = parse_text("10\n20+\n30\n40+\n50").unwrap();
        let r: Vec<f64> = ranks(&d).iter().filter_map(|r| r.adjusted_rank).collect();
        assert!((r[0] - 1.0).abs() < 1e-12);
        assert!((r[1] - 2.25).abs() < 1e-12);
        assert!((r[2] - 4.125).abs() < 1e-12);
        // Bernard: (1 − 0.3)/(5.4)
        assert!((ranks(&d)[0].median_rank.unwrap() - 0.7 / 5.4).abs() < 1e-12);
    }

    #[test]
    fn fits_recover_known_parameters_from_generated_data() {
        // Large complete sample: every method lands near the truth.
        let d = generate(2.5, 1000.0, 2000, None, 42);
        let (b, e) = mle(&d).unwrap();
        assert!((b - 2.5).abs() < 0.12, "MLE β {b}");
        assert!((e - 1000.0).abs() < 25.0, "MLE η {e}");
        let rows = ranks(&d);
        let pts: Vec<(f64, f64)> = rows.iter().filter_map(|r| r.median_rank.map(|f| (r.t, f))).collect();
        let (b, e, r2) = rank_regression(&pts, true).unwrap();
        assert!((b - 2.5).abs() < 0.15 && (e - 1000.0).abs() < 30.0 && r2 > 0.98, "RRX {b} {e} {r2}");
        // Heavily censored (≈ 40 % fail): MLE still recovers it.
        let c = quantile(0.4, 1.8, 500.0);
        let d = generate(1.8, 500.0, 3000, Some(c), 7);
        let (b, e) = mle(&d).unwrap();
        assert!((b - 1.8).abs() < 0.1 && (e - 500.0).abs() < 20.0, "censored MLE {b} {e}");
    }

    #[test]
    fn bounds_contain_the_estimate_and_lr_is_close_to_fisher() {
        let out = fit(FitIn {
            text: sample_sets().into_iter().find(|s| s.id == "bearings").unwrap().text,
            method: "mle".into(),
            bounds: "fisher".into(),
            ..Default::default()
        })
        .unwrap();
        for k in 0..out.line_t.len() {
            assert!(out.bound_lo[k] < out.line_t[k] && out.line_t[k] < out.bound_hi[k]);
        }
        let lr = fit(FitIn {
            text: sample_sets().into_iter().find(|s| s.id == "bearings").unwrap().text,
            method: "mle".into(),
            bounds: "lr".into(),
            ..Default::default()
        })
        .unwrap();
        // B10 bounds by the two methods agree within ~15 % for n = 23
        let (f, l) = (&out.b_lives[2], &lr.b_lives[2]);
        assert!((f.lo.unwrap() / l.lo.unwrap() - 1.0).abs() < 0.15, "{:?} {:?}", f, l);
        assert!(l.lo.unwrap() < l.t && l.t < l.hi.unwrap());
        let bb = lr.beta_bounds.unwrap();
        assert!(bb[0] < lr.beta && lr.beta < bb[1]);
    }

    #[test]
    fn weibayes_zero_failures() {
        // 5 units × 1000 h, β = 2: η = (5·1000²)^{1/2} = 2236 h
        let d = parse_text("1000+\n1000+\n1000+\n1000+\n1000+").unwrap();
        let (e, r, assumed) = weibayes(&d, 2.0);
        assert!((e - 2236.07).abs() < 0.01 && r == 0 && assumed);
    }

    #[test]
    fn parse_formats() {
        let d = parse_text("time,status\n5,F\n6,S\n7+\n8 1\n9 0 3\n").unwrap();
        assert_eq!(d.len(), 7);
        assert_eq!(d.iter().filter(|o| !o.failed).count(), 5);
        assert!(parse_text("abc\n5\nxyz").is_err());
    }

    #[test]
    fn small_sample_study_shows_mle_bias() {
        let s = study(StudyIn::default()).unwrap();
        let mle = s.methods.iter().find(|m| m.method == "mle").unwrap();
        // MLE β is biased high for n = 8 (well documented; Abernethy ch. 5)
        assert!(mle.beta_median > 2.0, "{}", mle.beta_median);
        assert!(s.methods.iter().all(|m| m.betas.len() > 250));
    }
}
