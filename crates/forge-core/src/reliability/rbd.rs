//! Reliability block diagrams (RBD): series, parallel and k-out-of-n groups
//! of blocks with constant failure rates (in failures in time, FIT = failures
//! per 10⁹ device-hours) or Weibull wear-out lives (in hours or cycles).
//!
//! * Series: `R = Π R_i`; parallel: `R = 1 − Π (1 − R_i)`; k-out-of-n: the
//!   probability at least k children work (dynamic programming over the
//!   children, so they need not be identical).
//! * A block with quantity q is q identical copies in series: `R = R_1^q`.
//! * Mean time to failure (MTTF) `= ∫₀^∞ R_sys(t) dt` (adaptive Simpson). The
//!   conventional **parts-count MTBF** is the same integral over the
//!   constant-rate blocks only (wear-out items excluded), which for a pure
//!   series system is `10⁹ / Σ FIT` (MIL-HDBK-217F parts-count convention).
//! * Importance at the mission time: **Birnbaum** `I_B = R_sys(R_i=1) −
//!   R_sys(R_i=0)` and **criticality** `I_C = I_B (1 − R_i)/(1 − R_sys)` —
//!   the share of system failures that block i is responsible for.
//! * **Monte Carlo check**: sample each block's time to failure, combine by
//!   the structure function (series = min, parallel = max, k-of-n = k-th
//!   largest) and count survivors.
//!
//! The Iris builder maps each bill-of-materials line to a *typical*
//! order-of-magnitude failure rate at 40 °C (replace with supplier data),
//! scales thermally activated parts to the board temperature in use with an
//! Arrhenius factor, and models the battery (capacity fade), the hinge and
//! the hinge flex cable as Weibull wear-out items driven by the use profile.
//!
//! References: O'Connor & Kleyner, *Practical Reliability Engineering*,
//! ch. 6; MIL-HDBK-217F, *Reliability Prediction of Electronic Equipment*
//! (1991, Notice 2 1995); Z. W. Birnbaum, "On the importance of different
//! components in a multicomponent system", 1969; Rausand & Høyland, *System
//! Reliability Theory*, 2nd ed., ch. 5.

use serde::{Deserialize, Serialize};

use crate::math::Rng;

fn one() -> f64 {
    1.0
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Node {
    pub id: String,
    pub label: String,
    /// `block` | `series` | `parallel` | `kofn`
    #[serde(default)]
    pub kind: String,
    /// Children that must work (k-of-n).
    #[serde(default)]
    pub k: usize,
    #[serde(default)]
    pub children: Vec<Node>,
    /// `exp` (constant rate) | `weibull` (wear-out)
    #[serde(default)]
    pub model: String,
    /// Failure rate at use conditions, FIT.
    #[serde(default)]
    pub fit: f64,
    /// Typical failure rate at the reference temperature, FIT.
    #[serde(default)]
    pub fit_ref: f64,
    /// Arrhenius factor applied (fit = fit_ref × temp_factor).
    #[serde(default)]
    pub temp_factor: f64,
    #[serde(default)]
    pub beta: f64,
    /// Weibull characteristic life in `life_unit`.
    #[serde(default)]
    pub eta: f64,
    /// `h` | `cycles`
    #[serde(default)]
    pub life_unit: String,
    /// Life units per operating hour (1 for hours).
    #[serde(default)]
    pub per_hour: f64,
    #[serde(default = "one")]
    pub qty: f64,
    /// `typical` | `user` | `test`
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub class: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub refdes: String,
}

impl Node {
    pub fn is_block(&self) -> bool {
        self.children.is_empty() && (self.kind == "block" || self.kind.is_empty())
    }

    fn lambda_per_h(&self) -> f64 {
        self.fit.max(0.0) * 1e-9
    }

    /// Reliability of one block at operating time t (h).
    pub fn block_rel(&self, t: f64, random_only: bool) -> f64 {
        let q = self.qty.max(0.0);
        let r1 = if self.model == "weibull" {
            if random_only || self.eta <= 0.0 || self.beta <= 0.0 {
                1.0
            } else {
                let per = if self.per_hour > 0.0 { self.per_hour } else { 1.0 };
                (-(t * per / self.eta).powf(self.beta)).exp()
            }
        } else {
            (-self.lambda_per_h() * t).exp()
        };
        r1.powf(q)
    }
}

/// Probability that at least k of the independent items with reliabilities `r` work.
pub fn k_of_n(k: usize, r: &[f64]) -> f64 {
    let n = r.len();
    if k == 0 {
        return 1.0;
    }
    if k > n {
        return 0.0;
    }
    // dp[j] = P(exactly j work) after processing items
    let mut dp = vec![0.0; n + 1];
    dp[0] = 1.0;
    for (i, &ri) in r.iter().enumerate() {
        for j in (0..=i + 1).rev() {
            let keep = dp[j] * (1.0 - ri);
            let add = if j > 0 { dp[j - 1] * ri } else { 0.0 };
            dp[j] = keep + add;
        }
    }
    dp[k..].iter().sum::<f64>().clamp(0.0, 1.0)
}

/// Reliability of a node; `fix` overrides one block's reliability (for importance measures).
pub fn rel(node: &Node, t: f64, random_only: bool, fix: Option<(&str, f64)>) -> f64 {
    if node.is_block() {
        if let Some((id, v)) = fix {
            if id == node.id {
                return v;
            }
        }
        return node.block_rel(t, random_only);
    }
    let rs: Vec<f64> = node.children.iter().map(|c| rel(c, t, random_only, fix)).collect();
    match node.kind.as_str() {
        "parallel" => 1.0 - rs.iter().map(|r| 1.0 - r).product::<f64>(),
        "kofn" => k_of_n(node.k.clamp(1, rs.len().max(1)), &rs),
        _ => rs.iter().product(),
    }
}

pub fn leaves(node: &Node) -> Vec<&Node> {
    let mut out = vec![];
    fn walk<'a>(n: &'a Node, out: &mut Vec<&'a Node>) {
        if n.is_block() {
            out.push(n);
        } else {
            for c in &n.children {
                walk(c, out);
            }
        }
    }
    walk(node, &mut out);
    out
}

/// Adaptive Simpson integral of R(t) from 0 to ∞ (truncated where R < 1e-10).
pub fn mttf(node: &Node, random_only: bool) -> f64 {
    let f = |t: f64| rel(node, t, random_only, None);
    let mut end = 1000.0;
    while f(end) > 1e-10 && end < 1e12 {
        end *= 2.0;
    }
    #[allow(clippy::too_many_arguments)]
    fn simpson(f: &dyn Fn(f64) -> f64, a: f64, b: f64, fa: f64, fm: f64, fb: f64, whole: f64, tol: f64, depth: u32) -> f64 {
        let m = 0.5 * (a + b);
        let (lm, rm) = (0.5 * (a + m), 0.5 * (m + b));
        let (flm, frm) = (f(lm), f(rm));
        let left = (m - a) / 6.0 * (fa + 4.0 * flm + fm);
        let right = (b - m) / 6.0 * (fm + 4.0 * frm + fb);
        if depth == 0 || (left + right - whole).abs() <= 15.0 * tol {
            return left + right + (left + right - whole) / 15.0;
        }
        simpson(f, a, m, fa, flm, fm, left, tol / 2.0, depth - 1) + simpson(f, m, b, fm, frm, fb, right, tol / 2.0, depth - 1)
    }
    // integrate over octaves so early wear-out features are resolved
    let mut total = 0.0;
    let mut a = 0.0;
    let mut b = end / 2f64.powi(24);
    while a < end {
        let (fa, fb) = (f(a), f(b));
        let fm = f(0.5 * (a + b));
        let whole = (b - a) / 6.0 * (fa + 4.0 * fm + fb);
        total += simpson(&f, a, b, fa, fm, fb, whole, 1e-7 * (b - a).max(1.0), 30);
        a = b;
        b = (b * 2.0).min(end);
    }
    total
}

/// Time at which R_sys falls to `r` (bisection), h.
pub fn life_at(node: &Node, r: f64) -> f64 {
    let (mut lo, mut hi) = (0.0f64, 1000.0f64);
    while rel(node, hi, false, None) > r && hi < 1e13 {
        hi *= 2.0;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if rel(node, mid, false, None) > r {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

fn sample_ttf(node: &Node, rng: &mut Rng, leaf_ttf: &mut Vec<f64>) -> f64 {
    if node.is_block() {
        let q = node.qty.max(0.0);
        // q identical copies in series: minimum of q draws ≡ one draw with the q-th power survival
        let u = rng.uniform().max(1e-300);
        let t = if q <= 0.0 {
            f64::INFINITY
        } else if node.model == "weibull" {
            let per = if node.per_hour > 0.0 { node.per_hour } else { 1.0 };
            if node.eta <= 0.0 || node.beta <= 0.0 {
                f64::INFINITY
            } else {
                node.eta / per * (-(1.0 - u).ln() / q).powf(1.0 / node.beta)
            }
        } else {
            let l = node.lambda_per_h() * q;
            if l <= 0.0 {
                f64::INFINITY
            } else {
                -(1.0 - u).ln() / l
            }
        };
        leaf_ttf.push(t);
        return t;
    }
    let mut ts: Vec<f64> = node.children.iter().map(|c| sample_ttf(c, rng, leaf_ttf)).collect();
    match node.kind.as_str() {
        "parallel" => ts.iter().cloned().fold(0.0, f64::max),
        "kofn" => {
            ts.sort_by(|a, b| b.total_cmp(a));
            let k = node.k.clamp(1, ts.len().max(1));
            ts.get(k - 1).cloned().unwrap_or(0.0)
        }
        _ => ts.iter().cloned().fold(f64::INFINITY, f64::min),
    }
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct BlockOut {
    pub id: String,
    pub label: String,
    pub refdes: String,
    pub group: String,
    pub class: String,
    pub model: String,
    pub fit: f64,
    pub fit_ref: f64,
    pub temp_factor: f64,
    pub beta: f64,
    pub eta: f64,
    pub life_unit: String,
    pub per_hour: f64,
    pub qty: f64,
    pub source: String,
    pub note: String,
    pub r_mission: f64,
    pub birnbaum: f64,
    pub criticality: f64,
    /// Share of Monte Carlo system failures before the mission time that this block caused first.
    pub mc_first_share: f64,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct GroupOut {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub k: usize,
    pub n: usize,
    pub r_mission: f64,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct McOut {
    pub samples: usize,
    pub r_mission: f64,
    pub ci_lo: f64,
    pub ci_hi: f64,
    pub mttf_h: f64,
    /// Empirical R(t) at the curve times.
    pub r_curve: Vec<f64>,
    /// A few simulated units: per-leaf times to failure (same order as `blocks`) and the system time.
    pub units: Vec<McUnit>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct McUnit {
    pub leaf_ttf_h: Vec<f64>,
    pub system_ttf_h: f64,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct RbdOut {
    pub mission_h: f64,
    pub r_mission: f64,
    pub r_mission_random: f64,
    pub mttf_h: f64,
    pub mtbf_random_h: f64,
    pub fit_total: f64,
    pub b10_h: f64,
    pub t_h: Vec<f64>,
    pub r_sys: Vec<f64>,
    pub r_random: Vec<f64>,
    pub blocks: Vec<BlockOut>,
    pub groups: Vec<GroupOut>,
    pub mc: Option<McOut>,
    pub dominant: String,
    pub notes: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct RbdIn {
    pub root: Node,
    pub mission_h: f64,
    /// Monte Carlo samples (0 = skip).
    pub samples: usize,
    pub seed: u64,
    /// Curve end, h (default 3 × mission).
    pub horizon_h: f64,
}

impl Default for RbdIn {
    fn default() -> Self {
        RbdIn {
            root: Node::default(),
            mission_h: 5840.0,
            samples: 20_000,
            seed: 1,
            horizon_h: 0.0,
        }
    }
}

fn validate(n: &Node) -> Result<(), String> {
    if n.is_block() {
        if n.model == "weibull" && !(n.beta > 0.0 && n.eta > 0.0) {
            return Err(format!("block `{}`: Weibull β and η must be positive", n.label));
        }
        if n.fit < 0.0 || !n.fit.is_finite() {
            return Err(format!("block `{}`: FIT must be ≥ 0", n.label));
        }
        return Ok(());
    }
    if n.children.is_empty() {
        return Err(format!("group `{}` has no blocks", n.label));
    }
    for c in &n.children {
        validate(c)?;
    }
    Ok(())
}

/// Evaluate a block diagram at the mission time, with curves, importance and a Monte Carlo check.
pub fn evaluate(i: RbdIn) -> Result<RbdOut, String> {
    let root = &i.root;
    validate(root)?;
    let tm = i.mission_h.max(1e-9);
    let r_sys = rel(root, tm, false, None);
    let horizon = if i.horizon_h > 0.0 { i.horizon_h } else { 3.0 * tm };
    let t_h: Vec<f64> = (0..=150).map(|k| horizon * k as f64 / 150.0).collect();
    let mut groups = vec![];
    let mut group_of: Vec<(String, String)> = vec![];
    fn walk(n: &Node, tm: f64, parent: &str, groups: &mut Vec<GroupOut>, group_of: &mut Vec<(String, String)>) {
        if n.is_block() {
            group_of.push((n.id.clone(), parent.to_string()));
            return;
        }
        groups.push(GroupOut {
            id: n.id.clone(),
            label: n.label.clone(),
            kind: if n.kind.is_empty() { "series".into() } else { n.kind.clone() },
            k: n.k,
            n: n.children.len(),
            r_mission: rel(n, tm, false, None),
        });
        for c in &n.children {
            walk(c, tm, &n.label, groups, group_of);
        }
    }
    walk(root, tm, "", &mut groups, &mut group_of);
    let lv = leaves(root);
    let mut blocks: Vec<BlockOut> = lv
        .iter()
        .map(|b| {
            let hi = rel(root, tm, false, Some((&b.id, 1.0)));
            let lo = rel(root, tm, false, Some((&b.id, 0.0)));
            let rb = b.block_rel(tm, false);
            let ib = hi - lo;
            BlockOut {
                id: b.id.clone(),
                label: b.label.clone(),
                refdes: b.refdes.clone(),
                group: group_of
                    .iter()
                    .find(|(id, _)| *id == b.id)
                    .map(|(_, g)| g.clone())
                    .unwrap_or_default(),
                class: b.class.clone(),
                model: if b.model.is_empty() { "exp".into() } else { b.model.clone() },
                fit: b.fit,
                fit_ref: b.fit_ref,
                temp_factor: b.temp_factor,
                beta: b.beta,
                eta: b.eta,
                life_unit: b.life_unit.clone(),
                per_hour: b.per_hour,
                qty: b.qty,
                source: b.source.clone(),
                note: b.note.clone(),
                r_mission: rb,
                birnbaum: ib,
                criticality: if r_sys < 1.0 { ib * (1.0 - rb) / (1.0 - r_sys) } else { 0.0 },
                mc_first_share: 0.0,
            }
        })
        .collect();
    let fit_total: f64 = lv.iter().filter(|b| b.model != "weibull").map(|b| b.fit * b.qty).sum();
    let mttf_h = mttf(root, false);
    let mtbf_random_h = mttf(root, true);
    let mc = if i.samples > 0 {
        let n = i.samples.clamp(100, 200_000);
        let mut rng = Rng::new(i.seed);
        let mut sys = Vec::with_capacity(n);
        let mut first = vec![0usize; lv.len()];
        let mut units = vec![];
        let mut fails_before = 0usize;
        for s in 0..n {
            let mut lt = Vec::with_capacity(lv.len());
            let ts = sample_ttf(root, &mut rng, &mut lt);
            if ts <= tm {
                fails_before += 1;
                // the block whose failure coincides with the system failure
                if let Some((k, _)) = lt
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| (**t - ts).abs() <= 1e-9 * ts.max(1.0))
                    .min_by(|a, b| a.1.total_cmp(b.1))
                {
                    first[k] += 1;
                }
            }
            if s < 8 {
                units.push(McUnit {
                    leaf_ttf_h: lt.iter().map(|t| if t.is_finite() { *t } else { 1e15 }).collect(),
                    system_ttf_h: if ts.is_finite() { ts } else { 1e15 },
                });
            }
            sys.push(ts);
        }
        for (b, c) in blocks.iter_mut().zip(&first) {
            b.mc_first_share = if fails_before > 0 { *c as f64 / fails_before as f64 } else { 0.0 };
        }
        let surv = sys.iter().filter(|t| **t > tm).count() as f64 / n as f64;
        // Wilson score interval, 95 %
        let z = 1.959_964;
        let nn = n as f64;
        let den = 1.0 + z * z / nn;
        let c = (surv + z * z / (2.0 * nn)) / den;
        let h = z * (surv * (1.0 - surv) / nn + z * z / (4.0 * nn * nn)).sqrt() / den;
        let finite: Vec<f64> = sys.iter().filter(|t| t.is_finite()).cloned().collect();
        let mut sorted = sys.clone();
        sorted.sort_by(f64::total_cmp);
        let r_curve = t_h.iter().map(|&t| 1.0 - sorted.partition_point(|x| *x <= t) as f64 / nn).collect();
        Some(McOut {
            samples: n,
            r_mission: surv,
            ci_lo: c - h,
            ci_hi: c + h,
            mttf_h: finite.iter().sum::<f64>() / finite.len().max(1) as f64,
            r_curve,
            units,
        })
    } else {
        None
    };
    let dominant = blocks
        .iter()
        .max_by(|a, b| a.criticality.total_cmp(&b.criticality))
        .map(|b| b.label.clone())
        .unwrap_or_default();
    let mut notes = vec![];
    if lv.iter().any(|b| b.model == "weibull") {
        notes.push("MTTF integrates every block (wear-out included); the parts-count MTBF counts only the constant-rate blocks — the conventional MIL-HDBK-217 figure, which says nothing about wear-out.".into());
    }
    notes.push("An MTBF of M hours does not mean units last M hours: for a constant rate only 37 % survive to t = MTBF.".into());
    Ok(RbdOut {
        mission_h: tm,
        r_mission: r_sys,
        r_mission_random: rel(root, tm, true, None),
        mttf_h,
        mtbf_random_h,
        fit_total,
        b10_h: life_at(root, 0.9),
        r_sys: t_h.iter().map(|&t| rel(root, t, false, None)).collect(),
        r_random: t_h.iter().map(|&t| rel(root, t, true, None)).collect(),
        t_h,
        blocks,
        groups,
        mc,
        dominant,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blk(id: &str, fit: f64) -> Node {
        Node {
            id: id.into(),
            label: id.into(),
            kind: "block".into(),
            model: "exp".into(),
            fit,
            qty: 1.0,
            ..Default::default()
        }
    }

    fn grp(kind: &str, k: usize, children: Vec<Node>) -> Node {
        Node {
            id: format!("g-{kind}"),
            label: kind.into(),
            kind: kind.into(),
            k,
            children,
            qty: 1.0,
            ..Default::default()
        }
    }

    #[test]
    fn series_parallel_algebra() {
        // R_i = e^{-λt}; pick λt so R = 0.9 and 0.8
        let t = 1000.0;
        let f = |r: f64| -r.ln() / t * 1e9;
        let s = grp("series", 0, vec![blk("a", f(0.9)), blk("b", f(0.8))]);
        assert!((rel(&s, t, false, None) - 0.72).abs() < 1e-12);
        let p = grp("parallel", 0, vec![blk("a", f(0.9)), blk("b", f(0.8))]);
        assert!((rel(&p, t, false, None) - 0.98).abs() < 1e-12);
        // 2-of-3 with R = 0.9 each: 3R² − 2R³ = 0.972
        let k = grp("kofn", 2, vec![blk("a", f(0.9)), blk("b", f(0.9)), blk("c", f(0.9))]);
        assert!((rel(&k, t, false, None) - 0.972).abs() < 1e-12);
        // k = n is series, k = 1 is parallel
        assert!((k_of_n(3, &[0.9, 0.8, 0.7]) - 0.504).abs() < 1e-12);
        assert!((k_of_n(1, &[0.9, 0.8]) - 0.98).abs() < 1e-12);
    }

    #[test]
    fn series_mtbf_is_inverse_total_rate_and_parallel_is_one_and_a_half() {
        let s = grp("series", 0, vec![blk("a", 1000.0), blk("b", 3000.0)]);
        let m = mttf(&s, false);
        assert!((m / 250_000.0 - 1.0).abs() < 1e-5, "{m}");
        // two identical exponential units in parallel: MTTF = 1.5/λ
        let p = grp("parallel", 0, vec![blk("a", 1000.0), blk("b", 1000.0)]);
        let m = mttf(&p, false);
        assert!((m / 1.5e6 - 1.0).abs() < 1e-5, "{m}");
    }

    #[test]
    fn weibull_block_and_monte_carlo_agree() {
        let w = Node {
            id: "w".into(),
            label: "w".into(),
            kind: "block".into(),
            model: "weibull".into(),
            beta: 3.0,
            eta: 20_000.0,
            life_unit: "cycles".into(),
            per_hour: 2.0,
            qty: 2.0,
            ..Default::default()
        };
        let root = grp("series", 0, vec![blk("a", 5000.0), w]);
        let out = evaluate(RbdIn {
            root,
            mission_h: 5000.0,
            samples: 40_000,
            ..Default::default()
        })
        .unwrap();
        // analytic: e^{-5000·5e-6} · (e^{-(10000/20000)^3})^2
        let exact = (-0.025f64).exp() * (-(0.5f64.powi(3))).exp().powi(2);
        assert!((out.r_mission - exact).abs() < 1e-12);
        let mc = out.mc.unwrap();
        assert!(mc.ci_lo <= exact && exact <= mc.ci_hi, "{} [{}, {}]", exact, mc.ci_lo, mc.ci_hi);
        // parts-count MTBF ignores the wear-out block
        assert!((out.mtbf_random_h - 2e5).abs() / 2e5 < 1e-5);
        assert!(out.mttf_h < out.mtbf_random_h);
        // Birnbaum importance in a series system = product of the others
        let a = out.blocks.iter().find(|b| b.id == "a").unwrap();
        let w = out.blocks.iter().find(|b| b.id == "w").unwrap();
        assert!((a.birnbaum - w.r_mission).abs() < 1e-12);
    }
}
