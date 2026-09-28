//! IIR filters as cascades of second-order sections ("biquads").
//!
//! * RBJ Audio-EQ-Cookbook biquads — R. Bristow-Johnson, "Cookbook formulae
//!   for audio EQ biquad filter coefficients" (W3C Note,
//!   <https://www.w3.org/TR/audio-eq-cookbook/>): with ω0 = 2π f0/fs,
//!   α = sin ω0 / (2Q) and A = 10^(G/40).
//! * N-th order Butterworth by the bilinear transform with frequency
//!   pre-warping, Ω_c = 2 fs tan(π fc / fs) (Oppenheim & Schafer §7.1.2):
//!   analogue poles p_k = Ω_c e^{jπ(2k+N+1)/(2N)} are paired into
//!   conjugate second-order sections and mapped with s = 2 fs (1−z⁻¹)/(1+z⁻¹).
//!
//! A causal IIR filter is stable iff every pole lies strictly inside the unit
//! circle |z| < 1.

use serde::{Deserialize, Serialize};

use crate::math::{poly_mul, C64, PI};

/// One second-order section, b = [b0,b1,b2], a = [1,a1,a2] (a0 normalised to 1).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Biquad {
    pub b: [f64; 3],
    pub a: [f64; 3],
}

impl Biquad {
    fn normalised(b: [f64; 3], a: [f64; 3]) -> Biquad {
        let a0 = a[0];
        Biquad { b: [b[0] / a0, b[1] / a0, b[2] / a0], a: [1.0, a[1] / a0, a[2] / a0] }
    }
    /// H(e^{jω}) at normalised frequency f (cycles/sample).
    pub fn response(&self, f: f64) -> C64 {
        let z1 = C64::from_polar(1.0, -2.0 * PI * f);
        let z2 = z1 * z1;
        let num = z1 * self.b[1] + z2 * self.b[2] + self.b[0];
        let den = z1 * self.a[1] + z2 * self.a[2] + self.a[0];
        num / den
    }
    pub fn poles(&self) -> Vec<C64> {
        quad_roots(self.a)
    }
    pub fn zeros(&self) -> Vec<C64> {
        quad_roots(self.b)
    }
}

/// Roots in z of c0 + c1 z⁻¹ + c2 z⁻² (i.e. of c0 z² + c1 z + c2); a
/// first-order section (c2 = 0) has one root, and a zero leading term none.
pub fn quad_roots(c: [f64; 3]) -> Vec<C64> {
    let [a, b, cc] = c;
    if a.abs() < 1e-300 {
        return vec![];
    }
    if cc.abs() < 1e-300 {
        if b.abs() < 1e-300 {
            return vec![];
        }
        return vec![C64::real(-b / a)];
    }
    let disc = b * b - 4.0 * a * cc;
    if disc >= 0.0 {
        let s = disc.sqrt();
        // numerically stable form (Numerical Recipes §5.6)
        let sgn = if b >= 0.0 { 1.0 } else { -1.0 };
        let q = -0.5 * (b + sgn * s);
        if q.abs() < 1e-300 {
            return vec![C64::ZERO, C64::ZERO];
        }
        vec![C64::real(q / a), C64::real(cc / q)]
    } else {
        let re = -b / (2.0 * a);
        let im = (-disc).sqrt() / (2.0 * a);
        vec![C64::new(re, im), C64::new(re, -im)]
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BiquadKind {
    #[default]
    Lowpass,
    Highpass,
    Bandpass,
    Notch,
    Peaking,
    LowShelf,
    HighShelf,
    Allpass,
}

/// RBJ cookbook biquad.
pub fn rbj(kind: BiquadKind, f0: f64, q: f64, gain_db: f64, fs: f64) -> Biquad {
    let w0 = 2.0 * PI * (f0 / fs).clamp(1e-6, 0.4999);
    let (sn, cs) = w0.sin_cos();
    let q = q.max(1e-3);
    let alpha = sn / (2.0 * q);
    let a = 10f64.powf(gain_db / 40.0);
    let sa = 2.0 * a.sqrt() * alpha;
    let (b, den) = match kind {
        BiquadKind::Lowpass => ([(1.0 - cs) / 2.0, 1.0 - cs, (1.0 - cs) / 2.0], [1.0 + alpha, -2.0 * cs, 1.0 - alpha]),
        BiquadKind::Highpass => ([(1.0 + cs) / 2.0, -(1.0 + cs), (1.0 + cs) / 2.0], [1.0 + alpha, -2.0 * cs, 1.0 - alpha]),
        BiquadKind::Bandpass => ([alpha, 0.0, -alpha], [1.0 + alpha, -2.0 * cs, 1.0 - alpha]),
        BiquadKind::Notch => ([1.0, -2.0 * cs, 1.0], [1.0 + alpha, -2.0 * cs, 1.0 - alpha]),
        BiquadKind::Allpass => ([1.0 - alpha, -2.0 * cs, 1.0 + alpha], [1.0 + alpha, -2.0 * cs, 1.0 - alpha]),
        BiquadKind::Peaking => ([1.0 + alpha * a, -2.0 * cs, 1.0 - alpha * a], [1.0 + alpha / a, -2.0 * cs, 1.0 - alpha / a]),
        BiquadKind::LowShelf => (
            [a * ((a + 1.0) - (a - 1.0) * cs + sa), 2.0 * a * ((a - 1.0) - (a + 1.0) * cs), a * ((a + 1.0) - (a - 1.0) * cs - sa)],
            [(a + 1.0) + (a - 1.0) * cs + sa, -2.0 * ((a - 1.0) + (a + 1.0) * cs), (a + 1.0) + (a - 1.0) * cs - sa],
        ),
        BiquadKind::HighShelf => (
            [a * ((a + 1.0) + (a - 1.0) * cs + sa), -2.0 * a * ((a - 1.0) + (a + 1.0) * cs), a * ((a + 1.0) + (a - 1.0) * cs - sa)],
            [(a + 1.0) - (a - 1.0) * cs + sa, 2.0 * ((a - 1.0) - (a + 1.0) * cs), (a + 1.0) - (a - 1.0) * cs - sa],
        ),
    };
    Biquad::normalised(b, den)
}

/// Bilinear transform of an analogue section (B0 s² + B1 s + B2)/(A0 s² + A1 s + A2), K = 2 fs.
fn bilinear2(bs: [f64; 3], as_: [f64; 3], k: f64) -> Biquad {
    let map = |c: [f64; 3]| {
        let k2 = k * k;
        [c[0] * k2 + c[1] * k + c[2], 2.0 * c[2] - 2.0 * c[0] * k2, c[0] * k2 - c[1] * k + c[2]]
    };
    Biquad::normalised(map(bs), map(as_))
}

/// Bilinear transform of a first-order section (B0 s + B1)/(A0 s + A1).
fn bilinear1(bs: [f64; 2], as_: [f64; 2], k: f64) -> Biquad {
    let map = |c: [f64; 2]| [c[0] * k + c[1], c[1] - c[0] * k, 0.0];
    Biquad::normalised(map(bs), map(as_))
}

/// N-th order Butterworth low- or high-pass (−3 dB at fc) as cascaded biquads.
pub fn butterworth(order: usize, fc: f64, fs: f64, highpass: bool) -> Vec<Biquad> {
    let n = order.clamp(1, 16);
    let k = 2.0 * fs;
    let wc = k * (PI * (fc / fs).clamp(1e-6, 0.4999)).tan();
    let mut out = Vec::new();
    for i in 0..n / 2 {
        let theta = PI * (2 * i + n + 1) as f64 / (2 * n) as f64;
        let p = C64::from_polar(wc, theta);
        // s² − 2Re(p) s + |p|²
        let den = [1.0, -2.0 * p.re, wc * wc];
        let num = if highpass { [1.0, 0.0, 0.0] } else { [0.0, 0.0, wc * wc] };
        out.push(bilinear2(num, den, k));
    }
    if n % 2 == 1 {
        let num = if highpass { [1.0, 0.0] } else { [0.0, wc] };
        out.push(bilinear1(num, [1.0, wc], k));
    }
    out
}

/// Combine sections into one transfer function (b, a) in powers of z⁻¹.
pub fn sos_to_tf(sos: &[Biquad]) -> (Vec<f64>, Vec<f64>) {
    let mut b = vec![1.0];
    let mut a = vec![1.0];
    for s in sos {
        let trim = |c: [f64; 3]| {
            let mut v = c.to_vec();
            while v.len() > 1 && v[v.len() - 1] == 0.0 {
                v.pop();
            }
            v
        };
        b = poly_mul(&b, &trim(s.b));
        a = poly_mul(&a, &trim(s.a));
    }
    (b, a)
}

pub fn sos_response(sos: &[Biquad], f: f64) -> C64 {
    sos.iter().fold(C64::ONE, |acc, s| acc * s.response(f))
}

/// All poles / zeros of a cascade (exact per-section quadratics).
pub fn sos_poles_zeros(sos: &[Biquad]) -> (Vec<C64>, Vec<C64>) {
    let mut p = Vec::new();
    let mut z = Vec::new();
    for s in sos {
        p.extend(s.poles());
        z.extend(s.zeros());
    }
    (p, z)
}

/// Stable iff every pole has |z| < 1.
pub fn is_stable(poles: &[C64]) -> bool {
    poles.iter().all(|p| p.abs() < 1.0 - 1e-12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn butterworth_is_minus_3db_at_cutoff_and_maximally_flat() {
        let fs = 16000.0;
        for n in 1..=8 {
            let sos = butterworth(n, 2000.0, fs, false);
            let g = sos_response(&sos, 2000.0 / fs).db();
            assert!((g + 3.0103).abs() < 1e-6, "order {n}: {g}");
            assert!((sos_response(&sos, 0.0).abs() - 1.0).abs() < 1e-9);
            let (p, _) = sos_poles_zeros(&sos);
            assert_eq!(p.len(), n);
            assert!(is_stable(&p));
        }
        let hp = butterworth(4, 500.0, fs, true);
        assert!((sos_response(&hp, 500.0 / fs).db() + 3.0103).abs() < 1e-6);
        assert!((sos_response(&hp, 0.5).abs() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn rbj_biquads_have_cookbook_gains_and_are_stable() {
        let fs = 48000.0;
        let f0 = 1000.0;
        let lp = rbj(BiquadKind::Lowpass, f0, std::f64::consts::FRAC_1_SQRT_2, 0.0, fs);
        assert!((lp.response(f0 / fs).db() + 3.0103).abs() < 1e-6);
        let notch = rbj(BiquadKind::Notch, f0, 5.0, 0.0, fs);
        assert!(notch.response(f0 / fs).abs() < 1e-9);
        let pk = rbj(BiquadKind::Peaking, f0, 1.0, 6.0, fs);
        assert!((pk.response(f0 / fs).db() - 6.0).abs() < 1e-9);
        let ls = rbj(BiquadKind::LowShelf, f0, 0.707, -9.0, fs);
        assert!((ls.response(1e-6).db() + 9.0).abs() < 1e-3);
        let hs = rbj(BiquadKind::HighShelf, f0, 0.707, 4.0, fs);
        assert!((hs.response(0.4999).db() - 4.0).abs() < 0.05);
        let bp = rbj(BiquadKind::Bandpass, f0, 2.0, 0.0, fs);
        assert!((bp.response(f0 / fs).abs() - 1.0).abs() < 1e-9);
        let ap = rbj(BiquadKind::Allpass, f0, 0.8, 0.0, fs);
        assert!((ap.response(0.1).abs() - 1.0).abs() < 1e-9);
        for s in [lp, notch, pk, ls, hs, bp, ap] {
            assert!(is_stable(&s.poles()), "{s:?}");
        }
        // an unstable section is detected
        let bad = Biquad { b: [1.0, 0.0, 0.0], a: [1.0, -2.1, 1.1] };
        assert!(!is_stable(&bad.poles()));
    }

    #[test]
    fn sos_to_tf_matches_cascade() {
        let sos = butterworth(5, 3000.0, 16000.0, false);
        let (b, a) = sos_to_tf(&sos);
        assert_eq!(a.len(), 6);
        let f = 0.1;
        let z = |c: &[f64]| c.iter().enumerate().fold(C64::ZERO, |acc, (k, v)| acc + C64::from_polar(*v, -2.0 * PI * f * k as f64));
        let h = z(&b) / z(&a);
        assert!((h - sos_response(&sos, f)).abs() < 1e-9);
    }
}
