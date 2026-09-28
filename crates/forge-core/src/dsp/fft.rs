//! Fast Fourier transform, windows and spectral estimation.
//!
//! * [`fft`] / [`ifft`] — iterative radix-2 decimation-in-time Cooley–Tukey
//!   (J. W. Cooley & J. W. Tukey, "An algorithm for the machine calculation
//!   of complex Fourier series", *Math. Comp.* 19, 1965). Inputs are
//!   zero-padded to the next power of two.
//! * [`window`] — rectangular, Hann, Hamming, Blackman and Kaiser(β)
//!   (F. J. Harris, "On the use of windows for harmonic analysis with the
//!   discrete Fourier transform", *Proc. IEEE* 66(1), 1978).
//! * [`spectrum`] — single-sided amplitude spectrum corrected for the
//!   window's coherent gain, so a sine of amplitude `A` reads `A` (or
//!   `20·log10 A` dB) when it falls on a bin centre.
//! * [`welch_psd`] — Welch's averaged modified periodogram (P. D. Welch,
//!   IEEE Trans. Audio Electroacoust. AU-15, 1967), scaled to units²/Hz with
//!   the window's equivalent noise bandwidth.

use serde::{Deserialize, Serialize};

use crate::math::{C64, PI};

/// Next power of two ≥ n (n = 0 → 1).
pub fn next_pow2(n: usize) -> usize {
    n.max(1).next_power_of_two()
}

/// In-place iterative radix-2 FFT. `x.len()` must be a power of two.
/// `inverse = true` computes the unscaled inverse transform (caller divides by N).
pub fn fft_in_place(x: &mut [C64], inverse: bool) {
    let n = x.len();
    if n <= 1 {
        return;
    }
    debug_assert!(n.is_power_of_two());
    // bit-reversal permutation
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if j > i {
            x.swap(i, j);
        }
    }
    let sign = if inverse { 1.0 } else { -1.0 };
    let mut len = 2;
    while len <= n {
        let ang = sign * 2.0 * PI / len as f64;
        let wlen = C64::from_polar(1.0, ang);
        let half = len / 2;
        // twiddles by phasor rotation, re-anchored every 64 steps for accuracy
        let tw: Vec<C64> = (0..half.div_ceil(64))
            .map(|b| C64::from_polar(1.0, ang * (b * 64) as f64))
            .collect();
        let mut i = 0;
        while i < n {
            let mut w = C64::ONE;
            for k in 0..half {
                if k % 64 == 0 {
                    w = tw[k / 64];
                }
                let u = x[i + k];
                let v = x[i + k + half] * w;
                x[i + k] = u + v;
                x[i + k + half] = u - v;
                w = w * wlen;
            }
            i += len;
        }
        len <<= 1;
    }
}

/// FFT of a complex sequence, zero-padded to the next power of two (or to `nfft` if larger).
pub fn fft(x: &[C64], nfft: Option<usize>) -> Vec<C64> {
    let n = next_pow2(nfft.unwrap_or(0).max(x.len()));
    let mut buf = vec![C64::ZERO; n];
    buf[..x.len()].copy_from_slice(x);
    fft_in_place(&mut buf, false);
    buf
}

/// FFT of a real sequence (zero-padded to a power of two).
pub fn fft_real(x: &[f64], nfft: Option<usize>) -> Vec<C64> {
    let c: Vec<C64> = x.iter().map(|&v| C64::real(v)).collect();
    fft(&c, nfft)
}

/// Inverse FFT (scaled by 1/N). Length must be a power of two.
pub fn ifft(x: &[C64]) -> Vec<C64> {
    let n = next_pow2(x.len());
    let mut buf = vec![C64::ZERO; n];
    buf[..x.len()].copy_from_slice(x);
    fft_in_place(&mut buf, true);
    let k = 1.0 / n as f64;
    buf.iter().map(|v| v.scale(k)).collect()
}

/// Window families.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WindowKind {
    Rectangular,
    #[default]
    Hann,
    Hamming,
    Blackman,
    Kaiser,
}

/// Modified Bessel function of the first kind, order 0, by its power series
/// I0(x) = Σ ((x/2)^k / k!)² (converges quickly for the β ≤ 20 used here).
pub fn bessel_i0(x: f64) -> f64 {
    let y = x * x / 4.0;
    let mut sum = 1.0;
    let mut term = 1.0;
    for k in 1..200 {
        term *= y / (k as f64 * k as f64);
        sum += term;
        if term < 1e-17 * sum {
            break;
        }
    }
    sum
}

/// Symmetric window of length `n` (the form used for FIR design). For
/// spectral analysis the periodic ("DFT-even") variant is used; see
/// [`window_periodic`].
pub fn window(kind: WindowKind, n: usize, beta: f64) -> Vec<f64> {
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![1.0];
    }
    let m = (n - 1) as f64;
    (0..n).map(|i| window_value(kind, i as f64, m, beta)).collect()
}

/// Periodic window of length `n` (length-(n+1) symmetric window with the last
/// sample dropped) — what Harris recommends for DFT analysis.
pub fn window_periodic(kind: WindowKind, n: usize, beta: f64) -> Vec<f64> {
    if n <= 1 {
        return vec![1.0; n];
    }
    let m = n as f64;
    (0..n).map(|i| window_value(kind, i as f64, m, beta)).collect()
}

fn window_value(kind: WindowKind, i: f64, m: f64, beta: f64) -> f64 {
    let x = 2.0 * PI * i / m;
    match kind {
        WindowKind::Rectangular => 1.0,
        WindowKind::Hann => 0.5 - 0.5 * x.cos(),
        WindowKind::Hamming => 0.54 - 0.46 * x.cos(),
        WindowKind::Blackman => 0.42 - 0.5 * x.cos() + 0.08 * (2.0 * x).cos(),
        WindowKind::Kaiser => {
            let r = 2.0 * i / m - 1.0;
            bessel_i0(beta * (1.0 - r * r).max(0.0).sqrt()) / bessel_i0(beta)
        }
    }
}

/// Figures of merit for a window (Harris 1978, Table 1).
#[derive(Clone, Debug, Serialize)]
pub struct WindowInfo {
    pub kind: WindowKind,
    pub n: usize,
    pub beta: f64,
    pub w: Vec<f64>,
    /// Coherent gain Σw/N (1.0 for rectangular, 0.5 for Hann).
    pub coherent_gain: f64,
    /// Equivalent noise bandwidth in bins: N·Σw² / (Σw)².
    pub enbw_bins: f64,
    /// Scalloping loss (worst-case amplitude loss half-way between bins), dB.
    pub scalloping_loss_db: f64,
    /// Highest side-lobe level relative to the main lobe, dB.
    pub peak_sidelobe_db: f64,
    /// Main-lobe null-to-null width in bins.
    pub mainlobe_width_bins: f64,
    /// Normalised frequency axis in bins (−N/2 … N/2 around the main lobe) for plotting.
    pub bins: Vec<f64>,
    /// Window transform magnitude in dB (normalised to 0 dB at DC).
    pub response_db: Vec<f64>,
}

/// Window samples plus Harris' figures of merit and its zero-padded transform.
pub fn window_info(kind: WindowKind, n: usize, beta: f64) -> WindowInfo {
    let n = n.clamp(2, 1 << 16);
    let w = window_periodic(kind, n, beta);
    let s1: f64 = w.iter().sum();
    let s2: f64 = w.iter().map(|v| v * v).sum();
    let cg = s1 / n as f64;
    let enbw = n as f64 * s2 / (s1 * s1);
    // scalloping: |W(0.5 bin)| / W(0)
    let half = dtft_mag(&w, 0.5 / n as f64);
    let sl = 20.0 * (half / s1).log10();
    // transform with 32× zero padding, show ±16 bins
    let pad = 32;
    let nfft = next_pow2(n * pad);
    let spec = fft_real(&w, Some(nfft));
    let per_bin = nfft as f64 / n as f64;
    let span_bins = 16.0f64.min(n as f64 / 2.0);
    let kmax = (span_bins * per_bin) as usize;
    let mut bins = Vec::with_capacity(2 * kmax + 1);
    let mut db = Vec::with_capacity(2 * kmax + 1);
    for k in -(kmax as isize)..=(kmax as isize) {
        let idx = ((k + nfft as isize) as usize) % nfft;
        bins.push(k as f64 / per_bin);
        db.push(20.0 * (spec[idx].abs() / s1).max(1e-12).log10());
    }
    // main lobe: first local minimum right of DC
    let mut first_null = nfft / 2;
    for k in 1..nfft / 2 {
        if spec[k].abs() <= spec[k - 1].abs() && spec[k].abs() <= spec[k + 1].abs() {
            first_null = k;
            break;
        }
    }
    let psl = (first_null..nfft / 2).map(|k| spec[k].abs()).fold(0.0f64, f64::max);
    WindowInfo {
        kind,
        n,
        beta,
        w,
        coherent_gain: cg,
        enbw_bins: enbw,
        scalloping_loss_db: -sl,
        peak_sidelobe_db: 20.0 * (psl / s1).max(1e-12).log10(),
        mainlobe_width_bins: 2.0 * first_null as f64 / per_bin,
        bins,
        response_db: db,
    }
}

/// |Σ w[n] e^{-j2π f n}| at normalised frequency f (cycles/sample).
pub fn dtft_mag(x: &[f64], f: f64) -> f64 {
    dtft(x, f).abs()
}

/// Σ x[n] e^{-j2π f n} at normalised frequency f (cycles/sample).
pub fn dtft(x: &[f64], f: f64) -> C64 {
    let w = -2.0 * PI * f;
    let (mut re, mut im) = (0.0, 0.0);
    // rotate a phasor instead of calling sin/cos per sample
    let step = C64::from_polar(1.0, w);
    let mut ph = C64::ONE;
    for (i, &v) in x.iter().enumerate() {
        if i % 128 == 0 {
            ph = C64::from_polar(1.0, w * i as f64);
        }
        re += v * ph.re;
        im += v * ph.im;
        ph = ph * step;
    }
    C64::new(re, im)
}

/// Single-sided amplitude spectrum.
#[derive(Clone, Debug, Serialize)]
pub struct Spectrum {
    pub freq: Vec<f64>,
    /// Peak amplitude per bin (a bin-centred sine of amplitude A reads A).
    pub amp: Vec<f64>,
    /// 20·log10(amp) in dB (dBFS when full scale = 1).
    pub db: Vec<f64>,
    pub nfft: usize,
    pub bin_hz: f64,
    pub coherent_gain: f64,
    pub enbw_bins: f64,
    /// Strongest non-DC components (frequency, dB), strongest first.
    pub peaks: Vec<(f64, f64)>,
}

/// Windowed, coherent-gain-corrected single-sided amplitude spectrum.
pub fn spectrum(x: &[f64], fs: f64, kind: WindowKind, beta: f64, nfft: Option<usize>) -> Spectrum {
    let n = x.len().max(1);
    let w = window_periodic(kind, n, beta);
    let s1: f64 = w.iter().sum();
    let s2: f64 = w.iter().map(|v| v * v).sum();
    let xw: Vec<f64> = x.iter().zip(&w).map(|(a, b)| a * b).collect();
    let spec = fft_real(&xw, nfft);
    let nf = spec.len();
    let half = nf / 2;
    let mut freq = Vec::with_capacity(half + 1);
    let mut amp = Vec::with_capacity(half + 1);
    for (k, v) in spec.iter().enumerate().take(half + 1) {
        freq.push(k as f64 * fs / nf as f64);
        let scale = if k == 0 || k == half { 1.0 } else { 2.0 };
        amp.push(scale * v.abs() / s1);
    }
    let db: Vec<f64> = amp.iter().map(|a| 20.0 * a.max(1e-12).log10()).collect();
    // local maxima, sorted
    let mut peaks: Vec<(f64, f64)> = (2..half.saturating_sub(1))
        .filter(|&k| db[k] > db[k - 1] && db[k] >= db[k + 1])
        .map(|k| (freq[k], db[k]))
        .collect();
    peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    peaks.truncate(8);
    Spectrum {
        freq,
        amp,
        db,
        nfft: nf,
        bin_hz: fs / nf as f64,
        coherent_gain: s1 / n as f64,
        enbw_bins: n as f64 * s2 / (s1 * s1),
        peaks,
    }
}

/// Welch power spectral density.
#[derive(Clone, Debug, Serialize)]
pub struct Psd {
    pub freq: Vec<f64>,
    /// One-sided PSD in units²/Hz.
    pub psd: Vec<f64>,
    /// 10·log10(psd) in dB/Hz.
    pub db: Vec<f64>,
    pub segments: usize,
    pub seg_len: usize,
    /// Total power recovered by integrating the PSD (≈ mean-square of x).
    pub total_power: f64,
}

/// Welch's method: split into `seg_len` segments with `overlap` fraction,
/// window each, average |X|², scale by 1/(fs·Σw²) and double non-DC bins.
pub fn welch_psd(x: &[f64], fs: f64, kind: WindowKind, beta: f64, seg_len: usize, overlap: f64) -> Psd {
    let seg = next_pow2(seg_len.clamp(8, x.len().max(8)));
    let seg = if seg > x.len() && x.len() >= 8 {
        next_pow2(x.len()) / 2
    } else {
        seg
    }
    .max(8);
    let hop = ((seg as f64) * (1.0 - overlap.clamp(0.0, 0.95))).max(1.0) as usize;
    let w = window_periodic(kind, seg, beta);
    let s2: f64 = w.iter().map(|v| v * v).sum();
    let half = seg / 2;
    let mut acc = vec![0.0; half + 1];
    let mut count = 0;
    let mut start = 0;
    while start + seg <= x.len() {
        // no detrending, so the integrated PSD equals the mean-square (Parseval)
        let seg_x: Vec<f64> = (0..seg).map(|i| x[start + i] * w[i]).collect();
        let sp = fft_real(&seg_x, None);
        for k in 0..=half {
            acc[k] += sp[k].norm_sqr();
        }
        count += 1;
        start += hop;
    }
    if count == 0 {
        // shorter than one segment: single zero-padded periodogram
        let mut seg_x: Vec<f64> = x.iter().zip(&w).map(|(a, b)| a * b).collect();
        seg_x.resize(seg, 0.0);
        let sp = fft_real(&seg_x, None);
        for k in 0..=half {
            acc[k] += sp[k].norm_sqr();
        }
        count = 1;
    }
    let mut psd = Vec::with_capacity(half + 1);
    let mut freq = Vec::with_capacity(half + 1);
    for (k, a) in acc.iter().enumerate() {
        let scale = if k == 0 || k == half { 1.0 } else { 2.0 };
        psd.push(scale * a / (count as f64 * fs * s2));
        freq.push(k as f64 * fs / seg as f64);
    }
    let df = fs / seg as f64;
    let total_power = psd.iter().sum::<f64>() * df;
    Psd {
        db: psd.iter().map(|p| 10.0 * p.max(1e-30).log10()).collect(),
        freq,
        psd,
        segments: count,
        seg_len: seg,
        total_power,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fft_pure_tone_lands_in_its_bin_with_its_amplitude() {
        let n = 1024;
        let fs = 1024.0;
        let x: Vec<f64> = (0..n).map(|i| 0.7 * (2.0 * PI * 50.0 * i as f64 / fs).sin()).collect();
        let s = spectrum(&x, fs, WindowKind::Rectangular, 0.0, None);
        let (k, a) = s
            .amp
            .iter()
            .enumerate()
            .fold((0, 0.0), |b, (k, &a)| if a > b.1 { (k, a) } else { b });
        assert_eq!(k, 50);
        assert!((a - 0.7).abs() < 1e-9, "amplitude {a}");
        // Hann: coherent gain corrected, same amplitude on a bin centre
        let s = spectrum(&x, fs, WindowKind::Hann, 0.0, None);
        assert!((s.amp[50] - 0.7).abs() < 1e-9);
    }

    #[test]
    fn fft_inverse_roundtrip_and_zero_padding() {
        let x: Vec<C64> = (0..100)
            .map(|i| C64::new((i as f64 * 0.3).sin(), (i as f64 * 0.11).cos()))
            .collect();
        let xf = fft(&x, None);
        assert_eq!(xf.len(), 128);
        let back = ifft(&xf);
        for i in 0..100 {
            assert!((back[i] - x[i]).abs() < 1e-12);
        }
        for v in back.iter().skip(100) {
            assert!(v.abs() < 1e-12);
        }
    }

    #[test]
    fn parseval_holds() {
        let mut rng = crate::math::Rng::new(3);
        let x: Vec<f64> = (0..512).map(|_| rng.normal()).collect();
        let e_t: f64 = x.iter().map(|v| v * v).sum();
        let xf = fft_real(&x, None);
        let e_f: f64 = xf.iter().map(|v| v.norm_sqr()).sum::<f64>() / xf.len() as f64;
        assert!((e_t - e_f).abs() / e_t < 1e-12);
    }

    #[test]
    fn window_coherent_gains_match_harris() {
        let n = 4096;
        let cg = |k| window_info(k, n, 8.6).coherent_gain;
        assert!((cg(WindowKind::Rectangular) - 1.0).abs() < 1e-12);
        assert!((cg(WindowKind::Hann) - 0.5).abs() < 1e-9);
        assert!((cg(WindowKind::Hamming) - 0.54).abs() < 1e-9);
        assert!((cg(WindowKind::Blackman) - 0.42).abs() < 1e-9);
        let hann = window_info(WindowKind::Hann, n, 0.0);
        assert!((hann.enbw_bins - 1.5).abs() < 1e-3);
        assert!((hann.scalloping_loss_db - 1.42).abs() < 0.02);
        assert!((hann.peak_sidelobe_db + 31.5).abs() < 0.5, "{}", hann.peak_sidelobe_db);
        let rect = window_info(WindowKind::Rectangular, n, 0.0);
        assert!((rect.scalloping_loss_db - 3.92).abs() < 0.02);
        assert!((rect.peak_sidelobe_db + 13.26).abs() < 0.2);
        let ham = window_info(WindowKind::Hamming, n, 0.0);
        assert!((ham.peak_sidelobe_db + 42.7).abs() < 0.8, "{}", ham.peak_sidelobe_db);
    }

    #[test]
    fn bessel_i0_matches_tables() {
        assert!((bessel_i0(0.0) - 1.0).abs() < 1e-15);
        assert!((bessel_i0(1.0) - 1.266_065_877_752_008).abs() < 1e-12);
        assert!((bessel_i0(5.0) - 27.239_871_823_604_45).abs() < 1e-9);
    }

    #[test]
    fn welch_psd_integrates_to_signal_power() {
        let mut rng = crate::math::Rng::new(9);
        let x: Vec<f64> = (0..16384).map(|_| 0.5 * rng.normal()).collect();
        let p = welch_psd(&x, 8000.0, WindowKind::Hann, 0.0, 512, 0.5);
        assert!((p.total_power - 0.25).abs() < 0.02, "{}", p.total_power);
        // white noise: flat at σ²/(fs/2)
        let mid = p.psd[100];
        assert!((mid / (0.25 / 4000.0) - 1.0).abs() < 0.35);
    }
}
