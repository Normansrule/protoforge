//! Quantum key distribution (QKD).
//!
//! * **BB84** Monte Carlo (Bennett & Brassard 1984): random bits and bases,
//!   an optional intercept-resend eavesdropper, channel bit-flip noise and
//!   loss, sifting, the quantum bit error rate (QBER) and the asymptotic
//!   secret fraction of Shor & Preskill (PRL 85, 441, 2000) with an
//!   error-correction inefficiency f:
//!   `r = 1 − f·h(Q) − h(Q)`. With f = 1 it is zero at Q ≈ 11.0 %.
//!   Intercept-resend on a fraction `e` of photons gives
//!   `Q = (1−e)·p + e·(p/2 + 1/4)` for channel flip probability p, so a full
//!   attack shows 25 %.
//! * **Fiber link model**: decoy-state BB84 with weak coherent pulses in the
//!   asymptotic (infinite-decoy) limit of H.-K. Lo, X. Ma & K. Chen, "Decoy
//!   state quantum key distribution", PRL 94, 230504 (2005), eqs. (1)–(4)
//!   with the channel model of their "GYS" example:
//!   ```text
//!   η   = η_Bob · 10^(−(αL + L_extra)/10)          overall transmittance
//!   Y₀  = 2 p_dark                                  background yield (two detectors)
//!   Yᵢ  = Y₀ + 1 − (1 − η)^i ,  Y₁ ≈ Y₀ + η          i-photon yield
//!   Q_μ = Y₀ + 1 − e^(−ημ)                          gain
//!   E_μ = [e₀Y₀ + e_d(1 − e^(−ημ))] / Q_μ           QBER, e₀ = 1/2
//!   Q₁  = Y₁ μ e^(−μ),  e₁ = (e₀Y₀ + e_d η) / Y₁      single-photon gain and error
//!   R   = q { −Q_μ f h(E_μ) + Q₁ [1 − h(e₁)] }       bits per pulse
//!   ```
//!   with q = 1/2 for standard BB84 sifting. Without decoys the worst-case
//!   bound of Gottesman, Lo, Lütkenhaus & Preskill (GLLP, Quantum Inf.
//!   Comput. 4, 325, 2004) assumes every multi-photon pulse is attacked:
//!   `Δ = (1 − (1+μ)e^(−μ)) / Q_μ`, `R = q{−Q_μ f h(E_μ) + Q_μ(1−Δ)[1 − h(E_μ/(1−Δ))]}`.
//!   The repeaterless PLOB bound (Pirandola, Laurenza, Ottaviani & Banchi,
//!   Nat. Commun. 8, 15043, 2017) caps any point-to-point protocol at
//!   `−log₂(1 − η_ch)` secret bits per channel use.
//! * **E91** (Ekert 1991): entangled pairs, three measurement angles per
//!   side, key from matching angles and a CHSH test from the others.

use serde::{Deserialize, Serialize};

use super::linalg::h2;
use crate::math::{Rng, PI};

// ---------------------------------------------------------------------------
// BB84 Monte Carlo
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Bb84In {
    pub pulses: usize,
    /// Probability that the channel flips a bit (misalignment, noise).
    pub noise: f64,
    /// Fraction of photons Eve intercepts and resends.
    pub eve: f64,
    /// Channel transmittance (1 = lossless) for the animation.
    pub transmittance: f64,
    /// Error-correction inefficiency f ≥ 1.
    pub ec_efficiency: f64,
    /// Fraction of sifted bits disclosed to estimate the QBER.
    pub sample_fraction: f64,
    /// How many pulses to return in detail.
    pub show: usize,
    pub seed: u64,
}

impl Default for Bb84In {
    fn default() -> Self {
        Bb84In {
            pulses: 4000,
            noise: 0.02,
            eve: 0.0,
            transmittance: 1.0,
            ec_efficiency: 1.16,
            sample_fraction: 0.2,
            show: 48,
            seed: 7,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Pulse {
    pub alice_bit: u8,
    /// 0 = rectilinear (+), 1 = diagonal (×).
    pub alice_basis: u8,
    pub eve_basis: Option<u8>,
    pub eve_bit: Option<u8>,
    pub detected: bool,
    pub bob_basis: u8,
    pub bob_bit: Option<u8>,
    pub sifted: bool,
    pub error: bool,
    /// Disclosed for QBER estimation.
    pub sampled: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Bb84Out {
    pub pulses: Vec<Pulse>,
    pub sent: usize,
    pub detected: usize,
    pub sifted: usize,
    pub errors: usize,
    pub qber: f64,
    pub qber_estimate: f64,
    pub qber_expected: f64,
    pub sampled: usize,
    pub secret_fraction: f64,
    pub final_key_bits: usize,
    pub leak_ec_bits: f64,
    pub pa_removed_bits: f64,
    pub threshold: f64,
    pub abort: bool,
    /// r(Q) curve for the gauge.
    pub curve_q: Vec<f64>,
    pub curve_r: Vec<f64>,
}

/// Asymptotic BB84 secret fraction per sifted bit: 1 − f·h(Q) − h(Q).
pub fn secret_fraction(q: f64, f: f64) -> f64 {
    (1.0 - f * h2(q) - h2(q)).max(0.0)
}

/// QBER at which the secret fraction reaches zero (bisection).
pub fn qber_threshold(f: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, 0.5);
    for _ in 0..80 {
        let m = 0.5 * (lo + hi);
        if 1.0 - f * h2(m) - h2(m) > 0.0 {
            lo = m;
        } else {
            hi = m;
        }
    }
    0.5 * (lo + hi)
}

pub fn bb84(i: &Bb84In) -> Bb84Out {
    let n = i.pulses.clamp(10, 2_000_000);
    let mut rng = Rng::new(i.seed);
    let noise = i.noise.clamp(0.0, 0.5);
    let eve = i.eve.clamp(0.0, 1.0);
    let tr = i.transmittance.clamp(0.0, 1.0);
    let f = i.ec_efficiency.max(1.0);
    let mut shown = vec![];
    let (mut detected, mut sifted, mut errors, mut sampled, mut sample_err) = (0, 0, 0, 0, 0);
    for k in 0..n {
        let bit = (rng.next_u64() & 1) as u8;
        let basis = (rng.next_u64() & 1) as u8;
        // photon state: (basis, value) as prepared
        let (mut ph_basis, mut ph_bit) = (basis, bit);
        let (mut eb, mut ev) = (None, None);
        if rng.uniform() < eve {
            let b = (rng.next_u64() & 1) as u8;
            let v = if b == ph_basis { ph_bit } else { (rng.next_u64() & 1) as u8 };
            eb = Some(b);
            ev = Some(v);
            ph_basis = b;
            ph_bit = v;
        }
        let det = rng.uniform() < tr;
        let bob_basis = (rng.next_u64() & 1) as u8;
        let mut bob_bit = None;
        if det {
            detected += 1;
            let mut v = if bob_basis == ph_basis {
                ph_bit
            } else {
                (rng.next_u64() & 1) as u8
            };
            if rng.uniform() < noise {
                v ^= 1;
            }
            bob_bit = Some(v);
        }
        let sift = det && bob_basis == basis;
        let err = sift && bob_bit != Some(bit);
        let mut samp = false;
        if sift {
            sifted += 1;
            if err {
                errors += 1;
            }
            if rng.uniform() < i.sample_fraction {
                samp = true;
                sampled += 1;
                if err {
                    sample_err += 1;
                }
            }
        }
        if k < i.show.min(400) {
            shown.push(Pulse {
                alice_bit: bit,
                alice_basis: basis,
                eve_basis: eb,
                eve_bit: ev,
                detected: det,
                bob_basis,
                bob_bit,
                sifted: sift,
                error: err,
                sampled: samp,
            });
        }
    }
    let qber = if sifted > 0 { errors as f64 / sifted as f64 } else { 0.0 };
    let q_est = if sampled > 0 { sample_err as f64 / sampled as f64 } else { qber };
    let remaining = sifted - sampled;
    let r = secret_fraction(q_est, f);
    let thr = qber_threshold(1.0);
    let leak = remaining as f64 * f * h2(q_est);
    let pa = remaining as f64 * h2(q_est);
    let curve_q: Vec<f64> = (0..=60).map(|k| k as f64 * 0.005).collect();
    let curve_r = curve_q.iter().map(|&q| secret_fraction(q, f)).collect();
    Bb84Out {
        pulses: shown,
        sent: n,
        detected,
        sifted,
        errors,
        qber,
        qber_estimate: q_est,
        qber_expected: (1.0 - eve) * noise + eve * (noise / 2.0 + 0.25),
        sampled,
        secret_fraction: r,
        final_key_bits: (remaining as f64 * r).floor() as usize,
        leak_ec_bits: leak,
        pa_removed_bits: pa,
        threshold: thr,
        abort: r <= 0.0,
        curve_q,
        curve_r,
    }
}

// ---------------------------------------------------------------------------
// Fiber link: decoy-state BB84 vs GLLP vs PLOB
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LinkParams {
    /// Design distance, km.
    pub distance_km: f64,
    /// Fiber attenuation α, dB/km (standard single-mode fiber at 1550 nm ≈ 0.2).
    pub fiber_loss_db_km: f64,
    /// Extra fixed loss: connectors, splices, Bob's optics, dB.
    pub extra_loss_db: f64,
    /// Detector efficiency η_det (0..1).
    pub detector_efficiency: f64,
    /// Dark-count probability per detector per gate.
    pub dark_count_prob: f64,
    /// Optical misalignment error e_d (0..0.5).
    pub misalignment: f64,
    /// Error-correction inefficiency f (≥ 1).
    pub ec_efficiency: f64,
    /// Pulse repetition rate, Hz.
    pub source_rate_hz: f64,
    /// Signal mean photon number; ≤ 0 means optimise at each distance.
    pub mu: f64,
    /// Sifting factor q (1/2 for standard BB84).
    pub sifting: f64,
    /// Detector dead time, s (caps the detection rate; 0 = ignore).
    pub dead_time_s: f64,
}

impl Default for LinkParams {
    fn default() -> Self {
        LinkParams {
            distance_km: 25.0,
            fiber_loss_db_km: 0.2,
            extra_loss_db: 3.0,
            detector_efficiency: 0.2,
            dark_count_prob: 1e-6,
            misalignment: 0.015,
            ec_efficiency: 1.16,
            source_rate_hz: 1e8,
            mu: -1.0,
            sifting: 0.5,
            dead_time_s: 0.0,
        }
    }
}

impl LinkParams {
    /// The "GYS" parameters of Lo, Ma & Chen (2005), from Gobby, Yuan &
    /// Shields (APL 84, 3762, 2004): α = 0.21 dB/km, e_d = 3.3 %,
    /// Y₀ = 1.7×10⁻⁶, η_Bob = 4.5 %, f = 1.22, 2 MHz.
    pub fn gys() -> LinkParams {
        LinkParams {
            distance_km: 50.0,
            fiber_loss_db_km: 0.21,
            extra_loss_db: 0.0,
            detector_efficiency: 0.045,
            dark_count_prob: 8.5e-7,
            misalignment: 0.033,
            ec_efficiency: 1.22,
            source_rate_hz: 2e6,
            mu: -1.0,
            sifting: 0.5,
            dead_time_s: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct LinkPoint {
    pub distance_km: f64,
    pub eta: f64,
    pub mu: f64,
    pub gain: f64,
    pub qber: f64,
    pub y1: f64,
    pub q1: f64,
    pub e1: f64,
    /// Secret bits per pulse.
    pub rate_per_pulse: f64,
    pub key_rate_bps: f64,
}

fn eta_total(p: &LinkParams, l: f64) -> f64 {
    p.detector_efficiency * 10f64.powf(-(p.fiber_loss_db_km * l + p.extra_loss_db) / 10.0)
}

/// Decoy-state (infinite decoys, asymptotic) key rate per pulse at distance `l`, intensity `mu`.
pub fn decoy_point(p: &LinkParams, l: f64, mu: f64) -> LinkPoint {
    let eta = eta_total(p, l);
    let y0 = 2.0 * p.dark_count_prob;
    let ed = p.misalignment;
    let e0 = 0.5;
    let qmu = y0 + 1.0 - (-eta * mu).exp();
    let emu = (e0 * y0 + ed * (1.0 - (-eta * mu).exp())) / qmu;
    let y1 = y0 + eta;
    let q1 = y1 * mu * (-mu).exp();
    let e1 = ((e0 * y0 + ed * eta) / y1).min(0.5);
    let r = p.sifting * (-qmu * p.ec_efficiency * h2(emu) + q1 * (1.0 - h2(e1)));
    LinkPoint {
        distance_km: l,
        eta,
        mu,
        gain: qmu,
        qber: emu,
        y1,
        q1,
        e1,
        rate_per_pulse: r.max(0.0),
        key_rate_bps: r.max(0.0) * effective_rate(p, qmu),
    }
}

/// Pulse rate after the detector dead-time cap (non-paralysable: detections ≤ 1/τ).
fn effective_rate(p: &LinkParams, gain: f64) -> f64 {
    if p.dead_time_s > 0.0 && gain > 0.0 {
        let det = p.source_rate_hz * gain;
        let capped = det / (1.0 + det * p.dead_time_s);
        capped / gain
    } else {
        p.source_rate_hz
    }
}

/// Overall gain Q_μ (detection probability per pulse, dark counts included).
fn gain(p: &LinkParams, l: f64, mu: f64) -> f64 {
    2.0 * p.dark_count_prob + 1.0 - (-eta_total(p, l) * mu).exp()
}

/// GLLP without decoys (every multi-photon pulse assumed attacked).
pub fn gllp_point(p: &LinkParams, l: f64, mu: f64) -> f64 {
    let eta = eta_total(p, l);
    let y0 = 2.0 * p.dark_count_prob;
    let qmu = y0 + 1.0 - (-eta * mu).exp();
    let emu = (0.5 * y0 + p.misalignment * (1.0 - (-eta * mu).exp())) / qmu;
    let pm = 1.0 - (1.0 + mu) * (-mu).exp();
    let delta = pm / qmu;
    if delta >= 1.0 {
        return 0.0;
    }
    let r = p.sifting * (-qmu * p.ec_efficiency * h2(emu) + qmu * (1.0 - delta) * (1.0 - h2((emu / (1.0 - delta)).min(0.5))));
    r.max(0.0)
}

fn golden_max(f: impl Fn(f64) -> f64, mut a: f64, mut b: f64) -> f64 {
    let g = (5f64.sqrt() - 1.0) / 2.0;
    let mut c = b - g * (b - a);
    let mut d = a + g * (b - a);
    for _ in 0..60 {
        if f(c) > f(d) {
            b = d;
        } else {
            a = c;
        }
        c = b - g * (b - a);
        d = a + g * (b - a);
    }
    0.5 * (a + b)
}

/// Best decoy-state point at distance `l` (optimising μ unless fixed).
pub fn best_decoy(p: &LinkParams, l: f64) -> LinkPoint {
    if p.mu > 0.0 {
        return decoy_point(p, l, p.mu);
    }
    let mu = golden_max(|m| decoy_point(p, l, m).rate_per_pulse, 0.02, 1.5);
    decoy_point(p, l, mu)
}

pub fn best_gllp(p: &LinkParams, l: f64) -> (f64, f64) {
    if p.mu > 0.0 {
        return (gllp_point(p, l, p.mu), p.mu);
    }
    // the GLLP optimum is ~η so search on a log scale
    let lm = golden_max(|x| gllp_point(p, l, 10f64.powf(x)), -5.0, 0.0);
    let mu = 10f64.powf(lm);
    (gllp_point(p, l, mu), mu)
}

/// PLOB repeaterless bound on secret bits per channel use for fiber of length l.
pub fn plob(p: &LinkParams, l: f64) -> f64 {
    let eta_ch = 10f64.powf(-p.fiber_loss_db_km * l / 10.0);
    -(1.0 - eta_ch).log2()
}

/// Largest distance with a positive key rate (bisection on the given rate function).
pub fn max_distance(rate: impl Fn(f64) -> f64, hi: f64) -> f64 {
    if rate(0.0) <= 0.0 {
        return 0.0;
    }
    if rate(hi) > 0.0 {
        return hi;
    }
    let (mut a, mut b) = (0.0, hi);
    for _ in 0..50 {
        let m = 0.5 * (a + b);
        if rate(m) > 0.0 {
            a = m;
        } else {
            b = m;
        }
    }
    a
}

#[derive(Clone, Debug, Serialize)]
pub struct LinkOut {
    pub params: LinkParams,
    pub distance_km: Vec<f64>,
    pub decoy_bps: Vec<f64>,
    pub gllp_bps: Vec<f64>,
    pub plob_bps: Vec<f64>,
    pub mu_opt: Vec<f64>,
    pub qber_pct: Vec<f64>,
    pub max_distance_km: f64,
    pub max_distance_gllp_km: f64,
    pub design: LinkPoint,
    pub design_plob_bps: f64,
    pub design_gllp_bps: f64,
    /// Fraction of the PLOB bound reached at the design distance.
    pub plob_ratio: f64,
    pub loss_db: f64,
    /// Photon and bit budget at the design distance, source to secret key.
    pub budget: Vec<BudgetStage>,
}

/// One stage of the photon/bit budget (per second).
#[derive(Clone, Debug, Serialize)]
pub struct BudgetStage {
    pub label: String,
    pub rate: f64,
    pub note: String,
}

/// Rates per second from the laser to the final key at the design point.
pub fn budget(p: &LinkParams, d: &LinkPoint) -> Vec<BudgetStage> {
    let r = p.source_rate_hz;
    let ch = 10f64.powf(-(p.fiber_loss_db_km * d.distance_km + p.extra_loss_db) / 10.0);
    let det_raw = r * d.gain;
    let det = if p.dead_time_s > 0.0 {
        det_raw / (1.0 + det_raw * p.dead_time_s)
    } else {
        det_raw
    };
    let sifted = det * p.sifting;
    let leak = sifted * p.ec_efficiency * h2(d.qber);
    let st = |label: &str, rate: f64, note: String| BudgetStage {
        label: label.into(),
        rate,
        note,
    };
    vec![
        st("Laser pulses", r, format!("{:.3e} pulses/s", r)),
        st("Photons launched", r * d.mu, format!("μ = {:.3} photons per pulse (Poisson)", d.mu)),
        st(
            "Photons at Bob's detectors",
            r * d.mu * ch,
            format!(
                "× 10^(−{:.1} dB/10): fiber {:.1} dB + fixed {:.1} dB",
                -10.0 * ch.log10(),
                p.fiber_loss_db_km * d.distance_km,
                p.extra_loss_db
            ),
        ),
        st(
            "Detections",
            det,
            format!(
                "Q_μ = {:.2e} per pulse incl. dark counts, η_det = {:.0} %{}",
                d.gain,
                p.detector_efficiency * 100.0,
                if p.dead_time_s > 0.0 {
                    format!(", dead-time cap ×{:.3}", det / det_raw.max(1e-300))
                } else {
                    String::new()
                }
            ),
        ),
        st("Sifted bits", sifted, format!("× q = {} (bases agree)", p.sifting)),
        st(
            "After error-correction leakage",
            (sifted - leak).max(0.0),
            format!(
                "− f·h(E_μ) = {:.3} of the sifted bits; E_μ = {:.2} %",
                p.ec_efficiency * h2(d.qber),
                d.qber * 100.0
            ),
        ),
        st(
            "Secret key",
            d.key_rate_bps,
            "privacy amplification keeps only the single-photon part: Q₁[1 − h(e₁)]".into(),
        ),
    ]
}

pub fn link(p: &LinkParams, max_km: f64, points: usize) -> LinkOut {
    let points = points.clamp(10, 400);
    let max_km = if max_km > 0.0 { max_km } else { 250.0 };
    let dist: Vec<f64> = (0..points).map(|k| max_km * k as f64 / (points - 1) as f64).collect();
    let mut decoy = vec![];
    let mut mu = vec![];
    let mut qber = vec![];
    let mut gllp = vec![];
    let mut pl = vec![];
    for &l in &dist {
        let d = best_decoy(p, l);
        decoy.push(d.key_rate_bps);
        mu.push(d.mu);
        qber.push(d.qber * 100.0);
        let (g, gmu) = best_gllp(p, l);
        gllp.push(g * effective_rate(p, gain(p, l, gmu)));
        pl.push(plob(p, l) * p.source_rate_hz);
    }
    let dmax = max_distance(|l| best_decoy(p, l).rate_per_pulse, 600.0);
    let gmax = max_distance(|l| best_gllp(p, l).0, 600.0);
    let design = best_decoy(p, p.distance_km);
    let dp = plob(p, p.distance_km) * p.source_rate_hz;
    LinkOut {
        params: p.clone(),
        distance_km: dist,
        decoy_bps: decoy,
        gllp_bps: gllp,
        plob_bps: pl,
        mu_opt: mu,
        qber_pct: qber,
        max_distance_km: dmax,
        max_distance_gllp_km: gmax,
        design_gllp_bps: {
            let (g, gmu) = best_gllp(p, p.distance_km);
            g * effective_rate(p, gain(p, p.distance_km, gmu))
        },
        budget: budget(p, &design),
        plob_ratio: if dp > 0.0 { design.key_rate_bps / dp } else { 0.0 },
        design_plob_bps: dp,
        loss_db: p.fiber_loss_db_km * p.distance_km + p.extra_loss_db,
        design,
    }
}

// ---------------------------------------------------------------------------
// E91
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct E91In {
    pub pairs: usize,
    /// Werner visibility of the source (1 = perfect |Φ+⟩).
    pub visibility: f64,
    /// Fraction of Bob's photons Eve intercepts and resends.
    pub eve: f64,
    pub ec_efficiency: f64,
    pub show: usize,
    pub seed: u64,
}

impl Default for E91In {
    fn default() -> Self {
        E91In {
            pairs: 6000,
            visibility: 0.98,
            eve: 0.0,
            ec_efficiency: 1.16,
            show: 40,
            seed: 5,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct E91Pair {
    pub a: u8,
    pub b: u8,
    pub x: i8,
    pub y: i8,
    pub key: bool,
    pub eve: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct E91Out {
    /// Alice's and Bob's angles, degrees.
    pub alice_deg: [f64; 3],
    pub bob_deg: [f64; 3],
    pub pairs: Vec<E91Pair>,
    pub s: f64,
    pub s_expected: f64,
    pub correlations: Vec<f64>,
    pub key_bits: usize,
    pub qber: f64,
    pub secret_fraction: f64,
    pub secure: bool,
}

/// Ekert-style E91 with |Φ+⟩ (correlation cos(a − b)): Alice measures at
/// 0°, 45°, 90°, Bob at 45°, 90°, 135°. Matching angles (45°, 90°) give key
/// bits; (0°/90°) × (45°/135°) give the CHSH value.
pub fn e91(i: &E91In) -> E91Out {
    let d = PI / 180.0;
    let al = [0.0, 45.0 * d, 90.0 * d];
    let bl = [45.0 * d, 90.0 * d, 135.0 * d];
    let v = i.visibility.clamp(0.0, 1.0);
    let n = i.pairs.clamp(100, 2_000_000);
    let mut rng = Rng::new(i.seed);
    let mut sum = [[0.0f64; 3]; 3];
    let mut cnt = [[0usize; 3]; 3];
    let (mut key, mut kerr) = (0usize, 0usize);
    let mut shown = vec![];
    let sign = |r: &mut Rng, p_plus: f64| -> i8 {
        if r.uniform() < p_plus {
            1
        } else {
            -1
        }
    };
    for k in 0..n {
        let ai = rng.below(3);
        let bi = rng.below(3);
        let (a, b) = (al[ai], bl[bi]);
        let eve = rng.uniform() < i.eve;
        let (x, y) = if eve {
            // Eve measures Bob's photon along one of Bob's angles; the pair
            // collapses to a product state aligned with her result ε.
            let e = bl[rng.below(3)];
            let eps = sign(&mut rng, 0.5) as f64;
            let x = sign(&mut rng, (1.0 + v * eps * (a - e).cos()) / 2.0);
            let y = sign(&mut rng, (1.0 + eps * (b - e).cos()) / 2.0);
            (x, y)
        } else {
            let x = sign(&mut rng, 0.5);
            // P(y = x) = (1 + V cos(a − b)) / 2
            let same = rng.uniform() < (1.0 + v * (a - b).cos()) / 2.0;
            (x, if same { x } else { -x })
        };
        sum[ai][bi] += (x * y) as f64;
        cnt[ai][bi] += 1;
        let is_key = (ai == 1 && bi == 0) || (ai == 2 && bi == 1);
        if is_key {
            key += 1;
            if x != y {
                kerr += 1;
            }
        }
        if k < i.show {
            shown.push(E91Pair {
                a: ai as u8,
                b: bi as u8,
                x,
                y,
                key: is_key,
                eve,
            });
        }
    }
    let e = |a: usize, b: usize| if cnt[a][b] > 0 { sum[a][b] / cnt[a][b] as f64 } else { 0.0 };
    // CHSH with a ∈ {0°, 90°}, b ∈ {45°, 135°}
    let s = e(0, 0) - e(0, 2) + e(2, 0) + e(2, 2);
    let mut corr = vec![];
    for a in 0..3 {
        for b in 0..3 {
            corr.push(e(a, b));
        }
    }
    let q = if key > 0 { kerr as f64 / key as f64 } else { 0.0 };
    let r = secret_fraction(q, i.ec_efficiency.max(1.0));
    E91Out {
        alice_deg: [0.0, 45.0, 90.0],
        bob_deg: [45.0, 90.0, 135.0],
        pairs: shown,
        s,
        // correlations are linear in the attacked fraction
        s_expected: (1.0 - i.eve.clamp(0.0, 1.0)) * 2.0 * 2f64.sqrt() * v + i.eve.clamp(0.0, 1.0) * s_eve(v),
        correlations: corr,
        key_bits: key,
        qber: q,
        secret_fraction: r,
        secure: s > 2.0 && r > 0.0,
    }
}

/// Expected S under a full intercept-resend attack on Bob's side:
/// E(a,b) = V · avg_e cos(a−e) cos(b−e) over Eve's three angles.
fn s_eve(v: f64) -> f64 {
    let d = PI / 180.0;
    let es = [45.0 * d, 90.0 * d, 135.0 * d];
    let ee = |a: f64, b: f64| v * es.iter().map(|&e| (a - e).cos() * (b - e).cos()).sum::<f64>() / 3.0;
    let (a0, a1, b0, b1) = (0.0, 90.0 * d, 45.0 * d, 135.0 * d);
    ee(a0, b0) - ee(a0, b1) + ee(a1, b0) + ee(a1, b1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intercept_resend_gives_25_percent() {
        let o = bb84(&Bb84In {
            pulses: 200_000,
            noise: 0.0,
            eve: 1.0,
            ..Default::default()
        });
        assert!((o.qber - 0.25).abs() < 0.006, "{}", o.qber);
        assert!(o.abort);
        assert!((o.qber_expected - 0.25).abs() < 1e-12);
        // no Eve, 2 % noise: QBER ≈ 2 %, about half the pulses sifted
        let o = bb84(&Bb84In {
            pulses: 100_000,
            ..Default::default()
        });
        assert!((o.qber - 0.02).abs() < 0.003);
        assert!((o.sifted as f64 / o.sent as f64 - 0.5).abs() < 0.01);
        assert!(o.final_key_bits > 0);
    }

    #[test]
    fn shor_preskill_threshold_is_11_percent() {
        let t = qber_threshold(1.0);
        assert!((t - 0.110028).abs() < 1e-5, "{t}");
        assert!(qber_threshold(1.16) < t);
    }

    #[test]
    fn decoy_rate_hand_check_at_zero_km() {
        let p = LinkParams {
            mu: 0.5,
            extra_loss_db: 0.0,
            ..Default::default()
        };
        let pt = decoy_point(&p, 0.0, 0.5);
        let eta = 0.2;
        let y0 = 2e-6;
        let qmu = y0 + 1.0 - (-eta * 0.5f64).exp();
        let emu = (0.5 * y0 + 0.015 * (1.0 - (-eta * 0.5f64).exp())) / qmu;
        let q1 = (y0 + eta) * 0.5 * (-0.5f64).exp();
        let e1 = (0.5 * y0 + 0.015 * eta) / (y0 + eta);
        let r = 0.5 * (-qmu * 1.16 * h2(emu) + q1 * (1.0 - h2(e1)));
        assert!((pt.rate_per_pulse - r).abs() < 1e-15);
    }

    #[test]
    fn photon_budget_is_consistent() {
        let p = LinkParams {
            dead_time_s: 1e-6,
            source_rate_hz: 1e7,
            ..Default::default()
        };
        let o = link(&p, 100.0, 21);
        let b = &o.budget;
        assert_eq!(b.len(), 7);
        // each stage no larger than the one before
        assert!(b.windows(2).all(|w| w[1].rate <= w[0].rate * (1.0 + 1e-12)), "{b:?}");
        // photons at Bob = pulses × μ × 10^(−loss/10) with 25 km × 0.2 + 3 dB = 8 dB
        let want = 1e7 * o.design.mu * 10f64.powf(-0.8);
        assert!((b[2].rate - want).abs() < 1e-9 * want);
        // dead-time cap D/(1 + Dτ), then sifting ½
        let d = 1e7 * o.design.gain;
        assert!((b[3].rate - d / (1.0 + d * 1e-6)).abs() < 1e-9 * d);
        assert!((b[4].rate - b[3].rate * 0.5).abs() < 1e-9);
        assert_eq!(b[6].rate, o.design.key_rate_bps);
    }

    #[test]
    fn decoy_rate_decreases_and_cuts_off() {
        let p = LinkParams::default();
        let o = link(&p, 300.0, 121);
        let pos: Vec<f64> = o.decoy_bps.iter().copied().filter(|r| *r > 0.0).collect();
        assert!(pos.windows(2).all(|w| w[1] < w[0]));
        assert!(o.max_distance_km > 100.0 && o.max_distance_km < 300.0, "{}", o.max_distance_km);
        let beyond = best_decoy(&p, o.max_distance_km + 5.0);
        assert_eq!(beyond.key_rate_bps, 0.0);
        // decoy always beats GLLP-without-decoys, and PLOB bounds both
        for k in 0..o.distance_km.len() {
            assert!(o.decoy_bps[k] + 1e-9 >= o.gllp_bps[k]);
            assert!(o.decoy_bps[k] <= o.plob_bps[k]);
        }
        assert!(o.max_distance_gllp_km < o.max_distance_km);
    }

    #[test]
    fn gys_parameters_reach_about_140_km() {
        // Lo, Ma & Chen (2005) report ~140 km for the GYS set with infinite decoys.
        let p = LinkParams::gys();
        let d = max_distance(|l| best_decoy(&p, l).rate_per_pulse, 600.0);
        assert!(d > 125.0 && d < 155.0, "{d}");
    }

    #[test]
    fn plob_formula() {
        let p = LinkParams::default();
        // 50 km at 0.2 dB/km = 10 dB → η = 0.1 → −log2(0.9)
        assert!((plob(&p, 50.0) - (-(0.9f64).log2())).abs() < 1e-12);
        // high-loss limit ≈ 1.44 η
        let far = plob(&p, 200.0);
        assert!((far / (1.44 * 1e-4) - 1.0).abs() < 0.01, "{far}");
    }

    #[test]
    fn e91_violates_without_eve_and_not_with() {
        let o = e91(&E91In {
            pairs: 60_000,
            visibility: 1.0,
            ..Default::default()
        });
        assert!((o.s - 2.0 * 2f64.sqrt()).abs() < 0.06, "{}", o.s);
        assert!(o.qber < 0.01 && o.secure);
        let o = e91(&E91In {
            pairs: 60_000,
            visibility: 1.0,
            eve: 1.0,
            ..Default::default()
        });
        assert!(o.s < 2.0, "{}", o.s);
        assert!(!o.secure);
        assert!((o.s - o.s_expected).abs() < 0.06);
    }
}
