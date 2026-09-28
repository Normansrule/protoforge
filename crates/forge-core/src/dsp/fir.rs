//! Linear-phase FIR design.
//!
//! * **Windowed sinc** (Oppenheim & Schafer, *Discrete-Time Signal
//!   Processing*, 3rd ed., §7.5): the ideal brick-wall impulse response
//!   h_d[n] = sin(ω_c (n−M/2)) / (π (n−M/2)) truncated by a window w[n].
//! * **Kaiser's design formulas** (J. F. Kaiser, "Nonrecursive digital filter
//!   design using the I0-sinh window function", Proc. IEEE ISCAS 1974; O&S
//!   eq. 7.62–7.63): given the stop-band attenuation A = −20 log10 δ and the
//!   transition width Δω,
//!   β = 0.1102 (A − 8.7)                         for A > 50,
//!   β = 0.5842 (A − 21)^0.4 + 0.07886 (A − 21)   for 21 ≤ A ≤ 50,
//!   β = 0                                         for A < 21,
//!   M = (A − 8) / (2.285 Δω)  (filter order, taps = M + 1).
//! * **Weighted least squares** (type-I linear phase): minimise
//!   ∫ W(ω) (A(ω) − D(ω))² dω over the amplitude A(ω) = Σ a_k cos(kω) by
//!   solving the normal equations — used for CIC droop compensation, where
//!   the desired response D(ω) is 1/|H_CIC(ω)| in the pass band.

use serde::{Deserialize, Serialize};

use super::fft::{window, WindowKind};
use crate::math::PI;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Band {
    #[default]
    Lowpass,
    Highpass,
    Bandpass,
    Bandstop,
}

/// Normalised sinc: sin(πx)/(πx).
pub fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        1.0
    } else {
        (PI * x).sin() / (PI * x)
    }
}

/// Kaiser β for a stop-band attenuation `atten_db` (Kaiser 1974).
pub fn kaiser_beta(atten_db: f64) -> f64 {
    let a = atten_db;
    if a > 50.0 {
        0.1102 * (a - 8.7)
    } else if a >= 21.0 {
        0.5842 * (a - 21.0).powf(0.4) + 0.07886 * (a - 21.0)
    } else {
        0.0
    }
}

/// Kaiser's estimate of the number of taps for attenuation `atten_db` and
/// transition width `transition_hz` at sample rate `fs` (M = (A−8)/(2.285Δω), taps = M+1).
pub fn kaiser_taps(atten_db: f64, transition_hz: f64, fs: f64) -> usize {
    let dw = 2.0 * PI * transition_hz / fs;
    let m = ((atten_db - 8.0) / (2.285 * dw)).ceil().max(1.0) as usize;
    m + 1
}

/// Windowed-sinc design. Cut-offs `f1` (and `f2` for band filters) in Hz are
/// the −6 dB points (the window method places the ideal edge at half
/// amplitude). High-pass and band-stop are forced to an odd length (type I).
pub fn windowed_sinc(band: Band, taps: usize, f1: f64, f2: f64, fs: f64, win: WindowKind, beta: f64) -> Vec<f64> {
    let mut n = taps.clamp(1, 4097);
    if matches!(band, Band::Highpass | Band::Bandstop) && n.is_multiple_of(2) {
        n += 1;
    }
    let m = (n - 1) as f64 / 2.0;
    let c1 = (f1 / fs).clamp(1e-6, 0.4999);
    let (lo, hi) = {
        let c2 = (f2 / fs).clamp(1e-6, 0.4999);
        if c1 < c2 {
            (c1, c2)
        } else {
            (c2, c1)
        }
    };
    let lp = |fc: f64, k: f64| 2.0 * fc * sinc(2.0 * fc * k);
    let w = window(win, n, beta);
    let mut h: Vec<f64> = (0..n)
        .map(|i| {
            let k = i as f64 - m;
            let delta = if k.abs() < 1e-12 { 1.0 } else { 0.0 };
            let ideal = match band {
                Band::Lowpass => lp(c1, k),
                Band::Highpass => delta - lp(c1, k),
                Band::Bandpass => lp(hi, k) - lp(lo, k),
                Band::Bandstop => delta - (lp(hi, k) - lp(lo, k)),
            };
            ideal * w[i]
        })
        .collect();
    // normalise the pass-band gain to exactly 1 at its centre
    let f_ref = match band {
        Band::Lowpass | Band::Bandstop => 0.0,
        Band::Highpass => 0.5,
        Band::Bandpass => (lo + hi) / 2.0,
    };
    let g = super::fft::dtft(&h, f_ref).abs();
    if g > 1e-9 {
        for v in h.iter_mut() {
            *v /= g;
        }
    }
    h
}

/// Result of a Kaiser-window design from specifications.
#[derive(Clone, Debug, Serialize)]
pub struct KaiserDesign {
    pub taps: usize,
    pub beta: f64,
    pub h: Vec<f64>,
}

/// Kaiser design from (attenuation, transition width). The cut-off passed in
/// is the centre of the transition band.
pub fn kaiser_design(band: Band, atten_db: f64, transition_hz: f64, f1: f64, f2: f64, fs: f64, max_taps: usize) -> KaiserDesign {
    let beta = kaiser_beta(atten_db);
    let mut taps = kaiser_taps(atten_db, transition_hz.max(fs * 1e-4), fs).min(max_taps.max(3));
    if matches!(band, Band::Highpass | Band::Bandstop) && taps.is_multiple_of(2) {
        taps += 1;
    }
    let h = windowed_sinc(band, taps, f1, f2, fs, WindowKind::Kaiser, beta);
    KaiserDesign { taps: h.len(), beta, h }
}

/// Weighted least-squares type-I (odd length, symmetric) FIR.
///
/// `desired(f)` and `weight(f)` take normalised frequency f ∈ [0, 0.5]
/// (cycles/sample). A dense grid of `grid` points approximates the integrals.
pub fn least_squares<D: Fn(f64) -> f64, W: Fn(f64) -> f64>(taps: usize, desired: D, weight: W, grid: usize) -> Vec<f64> {
    let n = if taps.is_multiple_of(2) { taps + 1 } else { taps }.max(1);
    let m = (n - 1) / 2; // unknowns a_0..a_m, A(ω) = a_0 + Σ_{k≥1} a_k cos(kω)
    let g = grid.max(8 * n);
    let mut q = vec![vec![0.0; m + 1]; m + 1];
    let mut b = vec![0.0; m + 1];
    let mut cosv = vec![0.0; m + 1];
    for i in 0..=g {
        let f = 0.5 * i as f64 / g as f64;
        let wgt = weight(f);
        if wgt <= 0.0 {
            continue;
        }
        let d = desired(f);
        let w = 2.0 * PI * f;
        for (k, c) in cosv.iter_mut().enumerate() {
            *c = (k as f64 * w).cos();
        }
        for (k, row) in q.iter_mut().enumerate() {
            b[k] += wgt * d * cosv[k];
            for (l, qv) in row.iter_mut().enumerate() {
                *qv += wgt * cosv[k] * cosv[l];
            }
        }
    }
    for (k, row) in q.iter_mut().enumerate() {
        row[k] += 1e-12; // Tikhonov guard against an ill-conditioned grid
    }
    let a = crate::math::solve(q, b).unwrap_or_else(|| vec![0.0; m + 1]);
    // h[m] = a_0, h[m±k] = a_k / 2
    let mut h = vec![0.0; n];
    h[m] = a[0];
    for k in 1..=m {
        h[m - k] = a[k] / 2.0;
        h[m + k] = a[k] / 2.0;
    }
    h
}

/// Least-squares band filter with pass/stop edges (Hz), weight on stop band.
pub fn ls_band(band: Band, taps: usize, f1: f64, f2: f64, transition_hz: f64, fs: f64, stop_weight: f64) -> Vec<f64> {
    let t = (transition_hz / fs / 2.0).max(1e-4);
    let (c1, c2) = ((f1 / fs).min(f2.max(f1) / fs), (f2 / fs).max(f1 / fs));
    let inside = |f: f64| -> Option<bool> {
        // Some(true) pass, Some(false) stop, None transition ("don't care")
        match band {
            Band::Lowpass => {
                if f <= c1 - t {
                    Some(true)
                } else if f >= c1 + t {
                    Some(false)
                } else {
                    None
                }
            }
            Band::Highpass => {
                if f >= c1 + t {
                    Some(true)
                } else if f <= c1 - t {
                    Some(false)
                } else {
                    None
                }
            }
            Band::Bandpass | Band::Bandstop => {
                let pass = f >= c1 + t && f <= c2 - t;
                let stop = f <= c1 - t || f >= c2 + t;
                let r = if pass { Some(true) } else if stop { Some(false) } else { None };
                if band == Band::Bandstop {
                    r.map(|v| !v)
                } else {
                    r
                }
            }
        }
    };
    least_squares(
        taps,
        |f| if inside(f) == Some(true) { 1.0 } else { 0.0 },
        |f| match inside(f) {
            Some(true) => 1.0,
            Some(false) => stop_weight,
            None => 0.0,
        },
        4096,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::fft::dtft;

    #[test]
    fn windowed_sinc_cutoff_is_minus_6_db() {
        let fs = 48000.0;
        for win in [WindowKind::Hamming, WindowKind::Blackman, WindowKind::Hann] {
            let h = windowed_sinc(Band::Lowpass, 101, 6000.0, 0.0, fs, win, 0.0);
            let g = 20.0 * dtft(&h, 6000.0 / fs).abs().log10();
            assert!((g + 6.02).abs() < 0.3, "{win:?}: {g}");
            let dc = dtft(&h, 0.0).abs();
            assert!((dc - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn highpass_and_bandpass_have_the_right_shape() {
        let fs = 8000.0;
        let hp = windowed_sinc(Band::Highpass, 80, 1000.0, 0.0, fs, WindowKind::Hamming, 0.0);
        assert_eq!(hp.len() % 2, 1);
        assert!(dtft(&hp, 0.0).abs() < 0.01);
        assert!((dtft(&hp, 3000.0 / fs).abs() - 1.0).abs() < 0.01);
        let bp = windowed_sinc(Band::Bandpass, 121, 1000.0, 2000.0, fs, WindowKind::Blackman, 0.0);
        assert!((dtft(&bp, 1500.0 / fs).abs() - 1.0).abs() < 0.01);
        assert!(dtft(&bp, 200.0 / fs).abs() < 0.01);
        assert!(dtft(&bp, 3500.0 / fs).abs() < 0.01);
    }

    #[test]
    fn kaiser_design_meets_its_attenuation() {
        let fs = 32000.0;
        let d = kaiser_design(Band::Lowpass, 60.0, 2000.0, 5000.0, 0.0, fs, 1000);
        // β for 60 dB = 0.1102·51.3 = 5.653
        assert!((d.beta - 5.653).abs() < 0.01);
        // stop band from 6 kHz: every point ≤ −59 dB (Kaiser's formula is an estimate)
        let worst = (0..200).map(|i| 6000.0 + i as f64 * 50.0).map(|f| 20.0 * dtft(&d.h, f / fs).abs().log10()).fold(-400.0, f64::max);
        assert!(worst < -58.0, "worst stop band {worst}");
    }

    #[test]
    fn least_squares_lowpass_is_sensible() {
        let h = ls_band(Band::Lowpass, 41, 4000.0, 0.0, 1000.0, 16000.0, 10.0);
        assert_eq!(h.len(), 41);
        assert!((dtft(&h, 1000.0 / 16000.0).abs() - 1.0).abs() < 0.02);
        assert!(dtft(&h, 6000.0 / 16000.0).abs() < 0.01);
    }
}
