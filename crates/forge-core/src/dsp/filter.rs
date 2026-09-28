//! Filtering, frequency response and fixed-point quantisation.
//!
//! * [`lfilter`] — direct-form II transposed difference equation
//!   y[n] = Σ b_k x[n−k] − Σ a_k y[n−k] (O&S §6.5, Fig. 6.26); FIR is the
//!   special case a = [1].
//! * [`freqz`] — H(e^{jω}) = B(e^{jω}) / A(e^{jω}); the group delay uses the
//!   exact identity τ(ω) = Re{ Σ k c_k e^{−jωk} / Σ c_k e^{−jωk} } for each
//!   polynomial (τ = τ_B − τ_A), not a numerical derivative.
//! * [`quantize_coeffs`] — Q-format fixed point: with `bits` total bits and
//!   I integer bits (enough for the largest |coefficient|), the step is
//!   2^−(bits−1−I). Coefficient quantisation moves poles/zeros; data
//!   quantisation adds noise ≈ Δ²/12 per rounding (Widrow's model).

use serde::{Deserialize, Serialize};

use super::iir::Biquad;
use crate::math::{C64, PI};

/// y = filter(b, a, x) with direct-form II transposed state.
pub fn lfilter(b: &[f64], a: &[f64], x: &[f64]) -> Vec<f64> {
    let a0 = a.first().copied().filter(|v| v.abs() > 1e-300).unwrap_or(1.0);
    let b: Vec<f64> = b.iter().map(|v| v / a0).collect();
    let a: Vec<f64> = if a.is_empty() { vec![1.0] } else { a.iter().map(|v| v / a0).collect() };
    let n = b.len().max(a.len());
    let mut z = vec![0.0; n];
    let bk = |k: usize| b.get(k).copied().unwrap_or(0.0);
    let ak = |k: usize| a.get(k).copied().unwrap_or(0.0);
    let mut y = Vec::with_capacity(x.len());
    for &xn in x {
        let yn = bk(0) * xn + z[0];
        for k in 1..n {
            let next = if k < n - 1 { z[k] } else { 0.0 };
            z[k - 1] = bk(k) * xn - ak(k) * yn + next;
        }
        y.push(yn);
    }
    y
}

/// Plain FIR convolution (same length as x, zero initial state).
pub fn fir_filter(h: &[f64], x: &[f64]) -> Vec<f64> {
    let mut y = vec![0.0; x.len()];
    for (n, out) in y.iter_mut().enumerate() {
        let mut acc = 0.0;
        for (k, &hk) in h.iter().enumerate().take(n + 1) {
            acc += hk * x[n - k];
        }
        *out = acc;
    }
    y
}

/// Run a cascade of biquads (each in DF-II-T).
pub fn sos_filter(sos: &[Biquad], x: &[f64]) -> Vec<f64> {
    let mut y = x.to_vec();
    for s in sos {
        y = lfilter(&s.b, &s.a, &y);
    }
    y
}

/// Evaluate Σ c_k e^{−jωk} and Σ k c_k e^{−jωk}.
fn poly_z(c: &[f64], f: f64) -> (C64, C64) {
    let mut s = C64::ZERO;
    let mut d = C64::ZERO;
    for (k, &ck) in c.iter().enumerate() {
        let e = C64::from_polar(1.0, -2.0 * PI * f * k as f64);
        s = s + e * ck;
        d = d + e * (ck * k as f64);
    }
    (s, d)
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct FreqResponse {
    pub f: Vec<f64>,
    pub mag: Vec<f64>,
    pub mag_db: Vec<f64>,
    /// Unwrapped phase, degrees.
    pub phase_deg: Vec<f64>,
    /// Group delay in samples.
    pub group_delay: Vec<f64>,
}

/// Frequency grid from ~0 to fs/2 (linear or log).
pub fn grid(fs: f64, n: usize, log: bool) -> Vec<f64> {
    let n = n.clamp(8, 8192);
    if log {
        crate::math::logspace(fs / 2.0 * 1e-3, fs / 2.0, n)
    } else {
        crate::math::linspace(0.0, fs / 2.0, n)
    }
}

/// Frequency response of b/a on the frequencies `f` (Hz).
pub fn freqz(b: &[f64], a: &[f64], fs: f64, f: &[f64]) -> FreqResponse {
    let a = if a.is_empty() { &[1.0][..] } else { a };
    let mut out = FreqResponse { f: f.to_vec(), ..Default::default() };
    let mut prev_phase = 0.0;
    let mut unwrap_off = 0.0;
    let mut last_gd = 0.0;
    for (i, &fi) in f.iter().enumerate() {
        let fn_ = fi / fs;
        let (bs, bd) = poly_z(b, fn_);
        let (as_, ad) = poly_z(a, fn_);
        let h = bs / as_;
        let m = h.abs();
        out.mag.push(m);
        out.mag_db.push((20.0 * m.max(1e-15).log10()).max(-300.0));
        let ph = h.arg();
        if i > 0 {
            let mut d = ph + unwrap_off - prev_phase;
            while d > PI {
                unwrap_off -= 2.0 * PI;
                d -= 2.0 * PI;
            }
            while d < -PI {
                unwrap_off += 2.0 * PI;
                d += 2.0 * PI;
            }
        }
        let unwrapped = ph + unwrap_off;
        prev_phase = unwrapped;
        out.phase_deg.push(unwrapped.to_degrees());
        let gd = if bs.abs() > 1e-9 && as_.abs() > 1e-12 { (bd / bs).re - (ad / as_).re } else { last_gd };
        let gd = gd.clamp(-1e4, 1e4);
        last_gd = gd;
        out.group_delay.push(gd);
    }
    out
}

/// Frequency response of a biquad cascade (avoids forming a high-order polynomial).
pub fn sos_freqz(sos: &[Biquad], fs: f64, f: &[f64]) -> FreqResponse {
    let mut total = FreqResponse { f: f.to_vec(), mag: vec![1.0; f.len()], mag_db: vec![0.0; f.len()], phase_deg: vec![0.0; f.len()], group_delay: vec![0.0; f.len()] };
    for s in sos {
        let r = freqz(&s.b, &s.a, fs, f);
        for i in 0..f.len() {
            total.mag[i] *= r.mag[i];
            total.phase_deg[i] += r.phase_deg[i];
            total.group_delay[i] += r.group_delay[i];
        }
    }
    for i in 0..f.len() {
        total.mag_db[i] = (20.0 * total.mag[i].max(1e-15).log10()).max(-300.0);
    }
    total
}

/// Q-format description of a quantised coefficient set.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QFormat {
    pub bits: u32,
    pub int_bits: u32,
    pub frac_bits: u32,
    /// Integer codes (value = code · 2^−frac_bits).
    pub codes: Vec<i64>,
    pub values: Vec<f64>,
    pub max_error: f64,
}

/// Quantise to a signed `bits`-bit Q(int).(frac) format chosen so the largest
/// magnitude fits (round to nearest, saturate).
pub fn quantize_coeffs(c: &[f64], bits: u32) -> QFormat {
    let bits = bits.clamp(2, 32);
    let maxabs = c.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let mut int_bits = 0u32;
    while (1u64 << int_bits) as f64 <= maxabs && int_bits < bits - 1 {
        int_bits += 1;
    }
    let frac_bits = bits - 1 - int_bits;
    quantize_with(c, bits, frac_bits)
}

/// Quantise with an explicit number of fractional bits.
pub fn quantize_with(c: &[f64], bits: u32, frac_bits: u32) -> QFormat {
    let scale = 2f64.powi(frac_bits as i32);
    let lo = -(1i64 << (bits - 1));
    let hi = (1i64 << (bits - 1)) - 1;
    let codes: Vec<i64> = c.iter().map(|v| ((v * scale).round() as i64).clamp(lo, hi)).collect();
    let values: Vec<f64> = codes.iter().map(|&q| q as f64 / scale).collect();
    let max_error = c.iter().zip(&values).fold(0.0f64, |m, (a, b)| m.max((a - b).abs()));
    QFormat { bits, int_bits: bits - 1 - frac_bits, frac_bits, codes, values, max_error }
}

/// Round a signal to `bits` with full scale ±1 (saturating).
pub fn quantize_signal(x: &[f64], bits: u32) -> Vec<f64> {
    let q = 2f64.powi(bits.clamp(2, 32) as i32 - 1);
    x.iter().map(|v| ((v * q).round().clamp(-q, q - 1.0)) / q).collect()
}

/// SNR of `test` against `reference`, dB.
pub fn snr_db(reference: &[f64], test: &[f64]) -> f64 {
    let s: f64 = reference.iter().map(|v| v * v).sum();
    let e: f64 = reference.iter().zip(test).map(|(a, b)| (a - b).powi(2)).sum();
    if e <= 1e-300 {
        return 300.0;
    }
    10.0 * (s / e).log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lfilter_fir_matches_convolution_and_iir_impulse() {
        let h = [0.25, 0.5, 0.25];
        let x = [1.0, 2.0, 3.0, 0.0, -1.0];
        assert_eq!(lfilter(&h, &[1.0], &x), fir_filter(&h, &x));
        // y[n] = x[n] + 0.5 y[n-1]: impulse response 0.5^n
        let y = lfilter(&[1.0], &[1.0, -0.5], &[1.0, 0.0, 0.0, 0.0]);
        assert_eq!(y, vec![1.0, 0.5, 0.25, 0.125]);
    }

    #[test]
    fn symmetric_fir_group_delay_is_half_length() {
        let h = crate::dsp::fir::windowed_sinc(crate::dsp::fir::Band::Lowpass, 31, 1000.0, 0.0, 8000.0, crate::dsp::fft::WindowKind::Hamming, 0.0);
        let r = freqz(&h, &[1.0], 8000.0, &[100.0, 500.0, 900.0]);
        for g in r.group_delay {
            assert!((g - 15.0).abs() < 1e-6);
        }
    }

    #[test]
    fn quantisation_formats() {
        let q = quantize_coeffs(&[0.5, -0.25, 0.126], 8);
        assert_eq!(q.int_bits, 0);
        assert_eq!(q.frac_bits, 7);
        assert_eq!(q.codes, vec![64, -32, 16]);
        let q = quantize_coeffs(&[1.0, -1.9], 16);
        assert_eq!(q.int_bits, 1);
        // 16-bit data quantisation of a full-scale sine → SNR ≈ 6.02·16 + 1.76 dB
        let x: Vec<f64> = (0..20000).map(|i| 0.999 * (i as f64 * 0.01234).sin()).collect();
        let s = snr_db(&x, &quantize_signal(&x, 16));
        assert!((s - 98.1).abs() < 1.5, "{s}");
    }
}
