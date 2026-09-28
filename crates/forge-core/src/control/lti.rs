//! Linear time-invariant (LTI) systems as transfer functions
//! `G(s) = N(s)/D(s) · e^{−sT}` (coefficients highest power first).
//!
//! * Time responses: the transfer function is converted to controllable
//!   canonical state space and discretised **exactly** for a zero-order-hold
//!   input (`Ad = e^{AΔ}`), so step and impulse responses carry no
//!   integration error beyond the ZOH assumption.
//! * Frequency responses evaluate `G(jω)` directly; dead time contributes the
//!   exact phase `−ωT` (root locus uses a Padé approximation instead).
//! * Stability margins follow the Bode definitions in Åström & Murray,
//!   *Feedback Systems* (2nd ed.), §10.3.
//!
//! References: K. J. Åström & R. M. Murray, *Feedback Systems: An
//! Introduction for Scientists and Engineers*, 2nd ed., Princeton 2021
//! (<https://fbswiki.org>); G. F. Franklin, J. D. Powell, A. Emami-Naeini,
//! *Feedback Control of Dynamic Systems*, Pearson.

use serde::{Deserialize, Serialize};

use super::mat::{self, Mat};
use crate::math::{logspace, poly_add, poly_eval, poly_mul, poly_roots, C64};

/// A SISO transfer function with optional input dead time.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Tf {
    pub num: Vec<f64>,
    pub den: Vec<f64>,
    /// Dead time T in seconds (e^{−sT}).
    #[serde(default)]
    pub delay: f64,
}

fn strip(p: &[f64]) -> Vec<f64> {
    let scale = p.iter().fold(0.0f64, |m, v| m.max(v.abs())).max(1e-300);
    let mut v: Vec<f64> = p.to_vec();
    while v.len() > 1 && v[0].abs() <= 1e-14 * scale {
        v.remove(0);
    }
    if v.is_empty() {
        v.push(0.0);
    }
    v
}

impl Tf {
    pub fn new(num: Vec<f64>, den: Vec<f64>) -> Tf {
        Tf { num, den, delay: 0.0 }.normalized()
    }

    pub fn with_delay(mut self, t: f64) -> Tf {
        self.delay = t.max(0.0);
        self
    }

    /// Strip leading zeros and make the denominator monic.
    pub fn normalized(&self) -> Tf {
        let mut den = strip(&self.den);
        let mut num = strip(&self.num);
        // cancel common factors of s (pole–zero cancellation at the origin,
        // e.g. PID integrator × pendulum zero) exactly
        while den.len() > 1 && num.len() > 1 && den[den.len() - 1] == 0.0 && num[num.len() - 1] == 0.0 {
            den.pop();
            num.pop();
        }
        let lead = if den[0].abs() > 0.0 { den[0] } else { 1.0 };
        Tf { num: num.iter().map(|v| v / lead).collect(), den: den.iter().map(|v| v / lead).collect(), delay: self.delay }
    }

    pub fn order(&self) -> usize {
        self.den.len().saturating_sub(1)
    }

    pub fn is_proper(&self) -> bool {
        self.num.len() <= self.den.len()
    }

    pub fn eval(&self, s: C64) -> C64 {
        let g = poly_eval(&self.num, s) / poly_eval(&self.den, s);
        if self.delay > 0.0 {
            g * (s.scale(-self.delay)).exp()
        } else {
            g
        }
    }

    pub fn poles(&self) -> Vec<C64> {
        poly_roots(&self.den)
    }

    pub fn zeros(&self) -> Vec<C64> {
        poly_roots(&self.num)
    }

    /// Steady-state gain G(0) (∞ for integrating plants).
    pub fn dc_gain(&self) -> f64 {
        let n = *self.num.last().unwrap_or(&0.0);
        let d = *self.den.last().unwrap_or(&1.0);
        if d.abs() < 1e-300 {
            f64::INFINITY
        } else {
            n / d
        }
    }

    pub fn series(&self, o: &Tf) -> Tf {
        Tf { num: poly_mul(&self.num, &o.num), den: poly_mul(&self.den, &o.den), delay: self.delay + o.delay }.normalized()
    }

    /// Replace the dead time with a Padé approximation of the given order
    /// (1..=4): e^{−sT} ≈ P(−s)/P(s).
    pub fn pade(&self, order: usize) -> Tf {
        if self.delay <= 0.0 {
            return self.clone();
        }
        let (n, d) = pade(self.delay, order);
        Tf { num: poly_mul(&self.num, &n), den: poly_mul(&self.den, &d), delay: 0.0 }.normalized()
    }

    /// Closed loop with unity negative feedback, `G/(1+G)` (delay must be
    /// Padé-approximated first for an exact rational result).
    pub fn feedback_unity(&self) -> Tf {
        let g = self.pade(3);
        Tf { num: g.num.clone(), den: poly_add(&g.den, &g.num), delay: 0.0 }.normalized()
    }

    /// Controllable canonical realisation (A, B, C, D).
    pub fn to_ss(&self) -> Result<(Mat, Mat, Mat, f64), String> {
        let t = self.normalized();
        if !t.is_proper() {
            return Err("transfer function must be proper (deg num ≤ deg den)".into());
        }
        let n = t.order();
        let mut num = vec![0.0; n + 1 - t.num.len()];
        num.extend_from_slice(&t.num);
        let d = num[0];
        // strictly proper remainder: num − d·den
        let b: Vec<f64> = (0..=n).map(|i| num[i] - d * t.den[i]).collect();
        let mut a = mat::zeros(n, n);
        for i in 0..n.saturating_sub(1) {
            a[i][i + 1] = 1.0;
        }
        if n > 0 {
            for j in 0..n {
                a[n - 1][j] = -t.den[n - j];
            }
        }
        let mut bm = mat::zeros(n, 1);
        if n > 0 {
            bm[n - 1][0] = 1.0;
        }
        let mut c = mat::zeros(1, n);
        for j in 0..n {
            c[0][j] = b[n - j];
        }
        Ok((a, bm, c, d))
    }

    /// A time scale that captures the interesting dynamics: ≈ 7 time
    /// constants of the slowest stable pole (bounded), plus the dead time.
    pub fn natural_horizon(&self) -> f64 {
        horizon_from_poles(&self.poles()) + self.delay * 2.0
    }
}

pub fn horizon_from_poles(poles: &[C64]) -> f64 {
    let mut slow = f64::INFINITY;
    let fastest = poles.iter().fold(0.0f64, |m, p| m.max(p.abs()));
    let mut fast: f64 = 0.0;
    for p in poles {
        let m = p.abs();
        if m < 1e-9 {
            continue;
        }
        fast = fast.max(m);
        // ignore very slow poles (typically almost cancelled by a zero, e.g. a
        // weak integrator) so the window shows the dominant dynamics
        if p.re < -1e-9 && m >= 2e-3 * fastest {
            // settling ~ 4/|σ|; oscillation period 2π/ω should also be seen
            slow = slow.min(-p.re);
        }
    }
    let mut t = if slow.is_finite() { 7.0 / slow } else if fast > 0.0 { 20.0 / fast } else { 10.0 };
    for p in poles {
        if p.im.abs() > 1e-9 {
            t = t.max(3.0 * 2.0 * std::f64::consts::PI / p.im.abs());
        }
    }
    t.clamp(1e-3, 1e5)
}

/// Padé approximant of e^{−sT}: returns (numerator, denominator).
pub fn pade(t: f64, order: usize) -> (Vec<f64>, Vec<f64>) {
    let n = order.clamp(1, 6);
    // coefficients c_k = (2n−k)! n! / ((2n)! k! (n−k)!)
    let fact = |k: usize| (1..=k).fold(1.0, |a, b| a * b as f64);
    let mut num = vec![0.0; n + 1];
    let mut den = vec![0.0; n + 1];
    for k in 0..=n {
        let c = fact(2 * n - k) * fact(n) / (fact(2 * n) * fact(k) * fact(n - k)) * t.powi(k as i32);
        // highest power first: index n−k
        den[n - k] = c;
        num[n - k] = if k % 2 == 0 { c } else { -c };
    }
    (num, den)
}

// ---------------------------------------------------------------------------
// Time responses
// ---------------------------------------------------------------------------

/// Exact ZOH discretisation of a proper transfer function.
pub struct Discrete {
    pub ad: Mat,
    pub bd: Vec<f64>,
    pub c: Vec<f64>,
    pub d: f64,
    pub x: Vec<f64>,
}

impl Discrete {
    pub fn new(tf: &Tf, h: f64) -> Result<Discrete, String> {
        let (a, b, c, d) = tf.to_ss()?;
        let n = a.len();
        if n == 0 {
            return Ok(Discrete { ad: vec![], bd: vec![], c: vec![], d, x: vec![] });
        }
        let (ad, bd) = mat::c2d(&a, &b, h);
        Ok(Discrete { ad, bd: bd.iter().map(|r| r[0]).collect(), c: c[0].clone(), d, x: vec![0.0; n] })
    }
    /// Output for the current state and input.
    pub fn output(&self, u: f64) -> f64 {
        self.c.iter().zip(&self.x).map(|(a, b)| a * b).sum::<f64>() + self.d * u
    }
    /// Output ignoring the feedthrough (used to avoid algebraic loops).
    pub fn output_state(&self) -> f64 {
        self.c.iter().zip(&self.x).map(|(a, b)| a * b).sum::<f64>()
    }
    pub fn step(&mut self, u: f64) {
        if self.x.is_empty() {
            return;
        }
        let mut nx = mat::mat_vec(&self.ad, &self.x);
        for (v, b) in nx.iter_mut().zip(&self.bd) {
            *v += b * u;
        }
        self.x = nx;
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Response {
    pub t: Vec<f64>,
    pub y: Vec<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub u: Vec<f64>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InputKind {
    Step,
    Impulse,
    Ramp,
    Sine,
    Square,
}

/// Simulate `tf` for input `kind` over `t_end` seconds with `n` samples.
/// Impulse: the state is kicked by B (exact for strictly proper systems).
pub fn simulate(tf: &Tf, kind: InputKind, t_end: f64, n: usize, freq_hz: f64) -> Result<Response, String> {
    let n = n.clamp(10, 20000);
    let h = t_end / (n - 1) as f64;
    let mut sys = Discrete::new(tf, h)?;
    let nd = (tf.delay / h).round() as usize;
    let input = |t: f64| -> f64 {
        match kind {
            InputKind::Step => 1.0,
            InputKind::Impulse => 0.0,
            InputKind::Ramp => t,
            InputKind::Sine => (2.0 * std::f64::consts::PI * freq_hz * t).sin(),
            InputKind::Square => {
                if (2.0 * std::f64::consts::PI * freq_hz * t).sin() >= 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
        }
    };
    let mut hist = vec![0.0; nd + 1];
    let mut out = Response::default();
    let mut kicked = kind != InputKind::Impulse;
    for k in 0..n {
        let t = k as f64 * h;
        let u_now = input(t);
        hist.push(u_now);
        let u_del = hist[hist.len() - 1 - nd];
        if !kicked && t + 1e-12 >= tf.delay {
            // impulse through the delay: x += B (continuous-time impulse)
            if let Ok((_, b, _, _)) = tf.to_ss() {
                for (x, bb) in sys.x.iter_mut().zip(b.iter()) {
                    *x += bb[0];
                }
            }
            kicked = true;
        }
        let y = sys.output(u_del);
        if !y.is_finite() || y.abs() > 1e12 {
            break;
        }
        out.t.push(t);
        out.y.push(y);
        out.u.push(u_now);
        sys.step(u_del);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Step-response metrics
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct StepMetrics {
    /// 10 % → 90 % rise time, s.
    pub rise_time: Option<f64>,
    pub peak: f64,
    pub peak_time: f64,
    /// Percent overshoot relative to the final value.
    pub overshoot_pct: f64,
    /// 2 % settling time, s (None if not settled within the window).
    pub settling_time: Option<f64>,
    pub final_value: f64,
    /// reference − final value.
    pub ss_error: f64,
    pub stable: bool,
}

/// Metrics of a step response `y(t)` towards reference `r`.
pub fn step_metrics(t: &[f64], y: &[f64], r: f64) -> StepMetrics {
    let n = y.len();
    if n < 3 {
        return StepMetrics { stable: false, ..Default::default() };
    }
    let tail = (n / 20).max(1);
    let yf = y[n - tail..].iter().sum::<f64>() / tail as f64;
    let spread = y[n - tail..].iter().fold(0.0f64, |m, v| m.max((v - yf).abs()));
    let t_full = t.last().copied().unwrap_or(0.0);
    let stable = n > 3 && t[n - 1] >= t_full * 0.999 && spread <= 0.02 * yf.abs().max(r.abs()).max(1e-9) * 2.0;
    let sign = if yf >= 0.0 { 1.0 } else { -1.0 };
    let (mut pk, mut pk_t) = (y[0], t[0]);
    for i in 0..n {
        if y[i] * sign > pk * sign {
            pk = y[i];
            pk_t = t[i];
        }
    }
    let cross = |level: f64| -> Option<f64> {
        for i in 1..n {
            if (y[i - 1] - level) * sign < 0.0 && (y[i] - level) * sign >= 0.0 {
                let f = (level - y[i - 1]) / (y[i] - y[i - 1]);
                return Some(t[i - 1] + f * (t[i] - t[i - 1]));
            }
        }
        None
    };
    let y0 = y[0];
    let rise = match (cross(y0 + 0.1 * (yf - y0)), cross(y0 + 0.9 * (yf - y0))) {
        (Some(a), Some(b)) if yf.abs() > 1e-12 => Some(b - a),
        _ => None,
    };
    let band = 0.02 * (yf - y0).abs().max(1e-12);
    let mut settle = None;
    if stable {
        let mut last_out = None;
        for i in 0..n {
            if (y[i] - yf).abs() > band {
                last_out = Some(i);
            }
        }
        settle = match last_out {
            None => Some(t[0]),
            Some(i) if i + 1 < n => Some(t[i + 1]),
            _ => None,
        };
    }
    let os = if (yf - y0).abs() > 1e-12 { ((pk - yf) / (yf - y0) * 100.0).max(0.0) } else { 0.0 };
    StepMetrics { rise_time: rise, peak: pk, peak_time: pk_t, overshoot_pct: os, settling_time: settle, final_value: yf, ss_error: r - yf, stable }
}

// ---------------------------------------------------------------------------
// Frequency response and margins
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Bode {
    /// rad/s
    pub w: Vec<f64>,
    pub mag_db: Vec<f64>,
    /// Unwrapped phase, degrees.
    pub phase_deg: Vec<f64>,
}

/// Frequency grid spanning the poles and zeros of `tfs` (two decades beyond).
pub fn auto_grid(tfs: &[&Tf], n: usize) -> Vec<f64> {
    let mut lo = f64::INFINITY;
    let mut hi: f64 = 0.0;
    for tf in tfs {
        for z in tf.poles().into_iter().chain(tf.zeros()) {
            let m = z.abs();
            if m > 1e-9 {
                lo = lo.min(m);
                hi = hi.max(m);
            }
        }
        if tf.delay > 0.0 {
            hi = hi.max(3.0 / tf.delay);
            lo = lo.min(0.3 / tf.delay);
        }
    }
    if !lo.is_finite() {
        lo = 1.0;
        hi = 1.0;
    }
    logspace((lo / 100.0).max(1e-5), (hi * 100.0).min(1e7), n)
}

pub fn bode(tf: &Tf, w: &[f64]) -> Bode {
    let mut out = Bode { w: w.to_vec(), ..Default::default() };
    let mut prev: Option<f64> = None;
    for &wi in w {
        let g = tf.eval(C64::new(0.0, wi));
        out.mag_db.push(g.db());
        let mut ph = g.phase_deg();
        if let Some(p) = prev {
            while ph - p > 180.0 {
                ph -= 360.0;
            }
            while ph - p < -180.0 {
                ph += 360.0;
            }
        } else {
            // start low-frequency phase near its asymptote: type-k → −90k
            while ph > 90.0 {
                ph -= 360.0;
            }
        }
        prev = Some(ph);
        out.phase_deg.push(ph);
    }
    out
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Margins {
    /// Gain margin as a factor (None = infinite).
    pub gm: Option<f64>,
    pub gm_db: Option<f64>,
    /// Phase-crossover frequency (rad/s) where ∠L = −180°.
    pub w_pc: Option<f64>,
    /// Phase margin, degrees (None = |L| never crosses 1).
    pub pm_deg: Option<f64>,
    /// Gain-crossover frequency (rad/s) where |L| = 1.
    pub w_gc: Option<f64>,
    /// Delay margin PM/ω_gc, seconds.
    pub delay_margin: Option<f64>,
}

fn refine<F: Fn(f64) -> f64>(f: F, mut a: f64, mut b: f64) -> f64 {
    // bisection in log-frequency
    let mut fa = f(a);
    for _ in 0..60 {
        let m = (a * b).sqrt();
        let fm = f(m);
        if (fm > 0.0) == (fa > 0.0) {
            a = m;
            fa = fm;
        } else {
            b = m;
        }
    }
    (a * b).sqrt()
}

/// Gain and phase margins of the loop transfer function `l`.
pub fn margins(l: &Tf) -> Margins {
    let w = auto_grid(&[l], 1600);
    let bd = bode(l, &w);
    let mut out = Margins::default();
    // gain crossovers: |L| = 1 → choose the one with the smallest PM
    for i in 1..w.len() {
        let (a, b) = (bd.mag_db[i - 1], bd.mag_db[i]);
        if a.is_finite() && b.is_finite() && (a >= 0.0) != (b >= 0.0) {
            let wc = refine(|x| l.eval(C64::new(0.0, x)).db(), w[i - 1], w[i]);
            // interpolate unwrapped phase at wc
            let f = (wc.ln() - w[i - 1].ln()) / (w[i].ln() - w[i - 1].ln());
            let ph = bd.phase_deg[i - 1] + f * (bd.phase_deg[i] - bd.phase_deg[i - 1]);
            let mut pm = 180.0 + ph;
            pm = (pm + 180.0).rem_euclid(360.0) - 180.0;
            if out.pm_deg.is_none_or(|p| pm < p) {
                out.pm_deg = Some(pm);
                out.w_gc = Some(wc);
                out.delay_margin = if pm > 0.0 { Some(pm.to_radians() / wc) } else { Some(0.0) };
            }
        }
    }
    // phase crossovers: unwrapped phase crosses −180 + 360k
    for i in 1..w.len() {
        let (a, b) = (bd.phase_deg[i - 1], bd.phase_deg[i]);
        let ka = ((a + 180.0) / 360.0).floor();
        let kb = ((b + 180.0) / 360.0).floor();
        if ka != kb {
            let level = if b < a { ka * 360.0 - 180.0 } else { kb * 360.0 - 180.0 };
            let f = (level - a) / (b - a);
            let lw = w[i - 1].ln() + f * (w[i].ln() - w[i - 1].ln());
            let wpc = lw.exp();
            let mag = l.eval(C64::new(0.0, wpc)).abs();
            if mag > 0.0 {
                let gm = 1.0 / mag;
                let crit = gm.ln().abs();
                if out.gm.is_none_or(|g: f64| crit < g.ln().abs()) {
                    out.gm = Some(gm);
                    out.gm_db = Some(20.0 * gm.log10());
                    out.w_pc = Some(wpc);
                }
            }
        }
    }
    out
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Nyquist {
    pub re: Vec<f64>,
    pub im: Vec<f64>,
}

/// Nyquist curve L(jω) for ω > 0 (the ω < 0 half is its mirror image).
pub fn nyquist(l: &Tf, w: &[f64]) -> Nyquist {
    let mut out = Nyquist::default();
    for &wi in w {
        let g = l.eval(C64::new(0.0, wi));
        // clip huge values near integrators so the plot stays readable
        let m = g.abs();
        let g = if m > 50.0 { g.scale(50.0 / m) } else { g };
        out.re.push(g.re);
        out.im.push(g.im);
    }
    out
}

// ---------------------------------------------------------------------------
// Root locus
// ---------------------------------------------------------------------------

/// Durand–Kerner iteration warm-started from `init` (keeps branch identity).
pub fn roots_warm(coeffs: &[f64], init: &[C64]) -> Vec<C64> {
    let c = strip(coeffs);
    let n = c.len() - 1;
    if n == 0 {
        return vec![];
    }
    if init.len() != n {
        return poly_roots(&c);
    }
    let a: Vec<f64> = c.iter().map(|v| v / c[0]).collect();
    let mut r: Vec<C64> = init.iter().enumerate().map(|(i, z)| *z + C64::new(1e-7 * (i as f64 + 1.0), 1.3e-7)).collect();
    for _ in 0..300 {
        let mut md: f64 = 0.0;
        for i in 0..n {
            let mut den = C64::ONE;
            for j in 0..n {
                if i != j {
                    den = den * (r[i] - r[j]);
                }
            }
            if den.abs() < 1e-300 {
                den = C64::new(1e-12, 1e-12);
            }
            let d = poly_eval(&a, r[i]) / den;
            r[i] = r[i] - d;
            md = md.max(d.abs() / (1.0 + r[i].abs()));
        }
        if md < 1e-12 {
            break;
        }
    }
    for z in r.iter_mut() {
        if z.im.abs() < 1e-7 * (1.0 + z.re.abs()) {
            z.im = 0.0;
        }
    }
    r
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RootLocus {
    pub gains: Vec<f64>,
    /// branches[b][k] = closed-loop pole of branch b at gains[k].
    pub branches: Vec<Vec<C64>>,
    pub open_poles: Vec<C64>,
    pub open_zeros: Vec<C64>,
    /// Closed-loop poles at the reference gain k = 1.
    pub current: Vec<C64>,
    /// Smallest gain factor at which a pole enters the right half plane (if any).
    pub k_unstable: Option<f64>,
}

/// Root locus of 1 + k·L(s) = 0 for k ∈ [k_min, k_max] (log-spaced), where
/// `l` is rational (Padé-approximate delays first).
pub fn root_locus(l: &Tf, k_min: f64, k_max: f64, n: usize) -> RootLocus {
    let l = l.pade(2);
    let (num, den) = (&l.num, &l.den);
    let gains: Vec<f64> = std::iter::once(0.0).chain(logspace(k_min.max(1e-9), k_max.max(k_min * 10.0), n.max(10))).collect();
    let mut out = RootLocus { open_poles: poly_roots(den), open_zeros: poly_roots(num), ..Default::default() };
    let deg = den.len() - 1;
    if deg == 0 {
        return out;
    }
    let mut prev = out.open_poles.clone();
    out.branches = vec![Vec::with_capacity(gains.len()); deg];
    let mut k_unstable = None;
    for &k in &gains {
        let p = poly_add(den, &num.iter().map(|v| v * k).collect::<Vec<_>>());
        let mut r = if k == 0.0 { prev.clone() } else { roots_warm(&p, &prev) };
        // greedy nearest matching to previous branch positions
        let mut matched = vec![C64::ZERO; deg];
        let mut used = vec![false; r.len()];
        for (b, pz) in prev.iter().enumerate() {
            let mut best = usize::MAX;
            let mut bd = f64::INFINITY;
            for (j, z) in r.iter().enumerate() {
                if !used[j] && (*z - *pz).abs() < bd {
                    bd = (*z - *pz).abs();
                    best = j;
                }
            }
            if best != usize::MAX {
                used[best] = true;
                matched[b] = r[best];
            }
        }
        r = matched;
        if k > 0.0 && k_unstable.is_none() && r.iter().any(|z| z.re > 1e-9) && !prev.iter().any(|z| z.re > 1e-9) {
            k_unstable = Some(k);
        }
        for (b, z) in r.iter().enumerate() {
            out.branches[b].push(*z);
        }
        prev = r;
    }
    let p1 = poly_add(den, num);
    out.current = poly_roots(&p1);
    out.gains = gains;
    out.k_unstable = k_unstable;
    out
}

// ---------------------------------------------------------------------------
// PID controller
// ---------------------------------------------------------------------------

/// Parallel PID with first-order derivative filter:
/// `C(s) = Kp + Ki/s + Kd·s/(Tf·s + 1)`, with `Tf = Kd/(Kp·N)`
/// (Åström & Murray §11.5: the derivative acts only up to N/Td).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Pid {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    /// Derivative filter factor N (typ. 5–20).
    pub n: f64,
}

impl Default for Pid {
    fn default() -> Self {
        Pid { kp: 1.0, ki: 0.0, kd: 0.0, n: 10.0 }
    }
}

impl Pid {
    pub fn tf_filter(&self) -> f64 {
        if self.kd.abs() < 1e-15 {
            0.0
        } else if self.kp.abs() > 1e-12 {
            (self.kd / (self.kp * self.n.max(1e-6))).abs()
        } else {
            self.kd.abs() / self.n.max(1e-6)
        }
    }

    pub fn tf(&self) -> Tf {
        let tf = self.tf_filter();
        if self.ki.abs() < 1e-15 {
            // Kp + Kd s/(Tf s + 1) = ((Kp Tf + Kd) s + Kp)/(Tf s + 1)
            Tf::new(vec![self.kp * tf + self.kd, self.kp], vec![tf, 1.0])
        } else {
            Tf::new(vec![self.kp * tf + self.kd, self.kp + self.ki * tf, self.ki], vec![tf, 1.0, 0.0])
        }
    }

    pub fn scaled(&self, k: f64) -> Pid {
        Pid { kp: self.kp * k, ki: self.ki * k, kd: self.kd * k, n: self.n }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LoopSimOptions {
    pub t_end: Option<f64>,
    pub samples: usize,
    pub setpoint: f64,
    /// Actuator limits (None = unlimited).
    pub u_min: Option<f64>,
    pub u_max: Option<f64>,
    /// Back-calculation anti-windup (Åström & Hägglund).
    pub anti_windup: bool,
    /// Differentiate −y instead of e (no derivative kick on setpoint steps).
    pub d_on_measurement: bool,
    /// Step load disturbance added at the plant input.
    pub dist_time: Option<f64>,
    pub dist_value: f64,
}

impl Default for LoopSimOptions {
    fn default() -> Self {
        LoopSimOptions {
            t_end: None,
            samples: 1200,
            setpoint: 1.0,
            u_min: None,
            u_max: None,
            anti_windup: true,
            d_on_measurement: false,
            dist_time: None,
            dist_value: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LoopSim {
    pub t: Vec<f64>,
    pub y: Vec<f64>,
    pub u: Vec<f64>,
    pub r: Vec<f64>,
    pub saturated_frac: f64,
    pub diverged: bool,
}

/// Simulate the PID loop in discrete time with a fine step (the plant is
/// discretised exactly, the controller is updated every step).
pub fn simulate_pid_loop(plant: &Tf, pid: &Pid, opt: &LoopSimOptions, t_end: f64) -> Result<LoopSim, String> {
    let n_out = opt.samples.clamp(50, 8000);
    let sub = 4usize;
    let steps = n_out * sub;
    let h = t_end / steps as f64;
    let mut sys = Discrete::new(plant, h)?;
    let nd = (plant.delay / h).round() as usize;
    let mut ubuf = vec![0.0; nd + 1];
    let tf = pid.tf_filter();
    let alpha = if tf > 0.0 { 1.0 - (-h / tf).exp() } else { 1.0 };
    let ti = if pid.ki.abs() > 1e-15 { pid.kp / pid.ki } else { f64::INFINITY };
    let td = if pid.kp.abs() > 1e-15 { pid.kd / pid.kp } else { 0.0 };
    let tt = if ti.is_finite() && ti > 0.0 {
        if td > 0.0 {
            (ti * td).sqrt()
        } else {
            ti
        }
    } else {
        1.0
    }
    .max(h);
    let (mut integ, mut xd) = (0.0, 0.0);
    let mut out = LoopSim::default();
    let mut sat_count = 0usize;
    let mut first = true;
    for k in 0..=steps {
        let t = k as f64 * h;
        let r = opt.setpoint;
        let y = sys.output_state() + sys.d * ubuf[ubuf.len() - 1 - nd];
        let e = r - y;
        let dsig = if opt.d_on_measurement { -y } else { e };
        if first {
            xd = dsig; // start the filter at rest (no spurious kick at t=0 from initial state)
            if !opt.d_on_measurement {
                xd = 0.0;
            }
            first = false;
        }
        let dterm = if tf > 0.0 { pid.kd * (dsig - xd) / tf } else { 0.0 };
        let v = pid.kp * e + integ + dterm;
        let mut u = v;
        if let Some(hi) = opt.u_max {
            u = u.min(hi);
        }
        if let Some(lo) = opt.u_min {
            u = u.max(lo);
        }
        if (u - v).abs() > 1e-12 {
            sat_count += 1;
        }
        let dist = match opt.dist_time {
            Some(td0) if t >= td0 => opt.dist_value,
            _ => 0.0,
        };
        ubuf.push(u + dist);
        if ubuf.len() > nd + 64 {
            ubuf.drain(0..ubuf.len() - nd - 1);
        }
        if k % sub == 0 {
            out.t.push(t);
            out.y.push(y);
            out.u.push(u);
            out.r.push(r);
        }
        if !y.is_finite() || y.abs() > 1e9 {
            out.diverged = true;
            break;
        }
        // updates
        let aw = if opt.anti_windup { (u - v) / tt } else { 0.0 };
        integ += h * (pid.ki * e + aw);
        xd += alpha * (dsig - xd);
        let up = ubuf[ubuf.len() - 1 - nd];
        sys.step(up);
    }
    out.saturated_frac = sat_count as f64 / (steps + 1) as f64;
    Ok(out)
}

/// Closed-loop characteristic polynomial for C·G (delay → Padé order 3).
pub fn loop_tf(plant: &Tf, pid: &Pid) -> Tf {
    let c = pid.tf();
    Tf { num: poly_mul(&c.num, &plant.num), den: poly_mul(&c.den, &plant.den), delay: plant.delay }.normalized()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_order_step_hits_63_percent_at_tau() {
        let tau = 0.5;
        let g = Tf::new(vec![1.0], vec![tau, 1.0]);
        let r = simulate(&g, InputKind::Step, 5.0 * tau, 2001, 0.0).unwrap();
        let y_tau = crate::math::interp(&r.t, &r.y, tau);
        assert!((y_tau - (1.0 - (-1f64).exp())).abs() < 1e-6, "y(τ) = {y_tau}");
    }

    #[test]
    fn second_order_overshoot_matches_formula() {
        for &zeta in &[0.2, 0.5, 0.7] {
            let wn: f64 = 3.0;
            let g = Tf::new(vec![wn * wn], vec![1.0, 2.0 * zeta * wn, wn * wn]);
            let r = simulate(&g, InputKind::Step, 12.0, 6001, 0.0).unwrap();
            let m = step_metrics(&r.t, &r.y, 1.0);
            let expected = 100.0 * (-zeta * std::f64::consts::PI / (1.0 - zeta * zeta).sqrt()).exp();
            assert!((m.overshoot_pct - expected).abs() < 0.2, "ζ={zeta}: {} vs {expected}", m.overshoot_pct);
            // peak time π/ωd
            let tp = std::f64::consts::PI / (wn * (1.0 - zeta * zeta).sqrt());
            assert!((m.peak_time - tp).abs() < 0.01);
        }
    }

    #[test]
    fn margins_of_known_loop() {
        // L = 1/(s(s+1)(s+2)) → phase crossover at √2 rad/s with GM = 6 (15.56 dB)
        let l = Tf::new(vec![1.0], poly_mul(&poly_mul(&[1.0, 0.0], &[1.0, 1.0]), &[1.0, 2.0]));
        let m = margins(&l);
        assert!((m.w_pc.unwrap() - 2f64.sqrt()).abs() < 1e-3, "{:?}", m);
        assert!((m.gm.unwrap() - 6.0).abs() < 1e-2);
        // gain crossover: |L| = 1 at ω ≈ 0.4462, PM ≈ 53.4°
        assert!((m.w_gc.unwrap() - 0.4462).abs() < 2e-3);
        assert!((m.pm_deg.unwrap() - 53.4).abs() < 0.3);
    }

    #[test]
    fn delay_adds_phase_lag() {
        let g = Tf::new(vec![1.0], vec![1.0, 1.0]).with_delay(0.5);
        let w = [2.0];
        let b = bode(&g, &w);
        let expect = -(2.0f64).atan().to_degrees() - (0.5f64 * 2.0).to_degrees();
        assert!((b.phase_deg[0] - expect).abs() < 1e-6);
    }

    #[test]
    fn root_locus_endpoints() {
        // L = (s+3)/(s(s+1)(s+2)) : k→0 poles at 0,−1,−2; k→∞ one branch → −3
        let l = Tf::new(vec![1.0, 3.0], poly_mul(&poly_mul(&[1.0, 0.0], &[1.0, 1.0]), &[1.0, 2.0]));
        let rl = root_locus(&l, 1e-3, 1e5, 300);
        let first: Vec<C64> = rl.branches.iter().map(|b| b[0]).collect();
        for p in [0.0, -1.0, -2.0] {
            assert!(first.iter().any(|z| (*z - C64::real(p)).abs() < 1e-6));
        }
        let last: Vec<C64> = rl.branches.iter().map(|b| *b.last().unwrap()).collect();
        assert!(last.iter().any(|z| (*z - C64::real(-3.0)).abs() < 0.01), "{last:?}");
        // the other two go to infinity along ±90° asymptotes (centroid (−3+3)/2 = 0)
        assert!(last.iter().filter(|z| z.abs() > 50.0).count() == 2);
    }

    #[test]
    fn pid_loop_matches_linear_closed_loop_without_saturation() {
        let plant = Tf::new(vec![1.0], vec![1.0, 3.0, 2.0]);
        let pid = Pid { kp: 4.0, ki: 2.0, kd: 0.5, n: 20.0 };
        let cl = loop_tf(&plant, &pid).feedback_unity();
        let lin = simulate(&cl, InputKind::Step, 8.0, 3201, 0.0).unwrap();
        let sim = simulate_pid_loop(&plant, &pid, &LoopSimOptions { samples: 800, ..Default::default() }, 8.0).unwrap();
        for (i, &t) in sim.t.iter().enumerate().skip(10) {
            let yl = crate::math::interp(&lin.t, &lin.y, t);
            assert!((sim.y[i] - yl).abs() < 0.03, "t={t}: {} vs {yl}", sim.y[i]);
        }
    }

    #[test]
    fn anti_windup_reduces_overshoot() {
        let plant = Tf::new(vec![1.0], vec![1.0, 1.0, 0.0]);
        let pid = Pid { kp: 2.0, ki: 1.0, kd: 1.0, n: 10.0 };
        let base = LoopSimOptions { u_min: Some(-0.3), u_max: Some(0.3), samples: 1000, ..Default::default() };
        let with = simulate_pid_loop(&plant, &pid, &base, 60.0).unwrap();
        let without = simulate_pid_loop(&plant, &pid, &LoopSimOptions { anti_windup: false, ..base.clone() }, 60.0).unwrap();
        let pk = |y: &[f64]| y.iter().cloned().fold(f64::MIN, f64::max);
        assert!(pk(&with.y) < pk(&without.y), "{} vs {}", pk(&with.y), pk(&without.y));
    }

    #[test]
    fn pade_is_allpass() {
        let (n, d) = pade(0.3, 3);
        let g = Tf::new(n, d);
        for w in [0.1, 1.0, 10.0] {
            assert!((g.eval(C64::new(0.0, w)).abs() - 1.0).abs() < 1e-9);
        }
    }
}
