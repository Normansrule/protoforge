//! Iris PDM microphone chain: sigma-delta modulation → CIC decimator →
//! (optional half-band) → droop-compensating FIR → 16 kHz PCM.
//!
//! **Sigma-delta modulator.** A 1-bit quantiser inside a feedback loop
//! pushes quantisation noise to high frequencies. In error-feedback form
//! u[n] = x[n] − Σ g_k e[n−k], y[n] = sgn(u[n]), e[n] = y[n] − u[n]; the
//! output is Y(z) = X(z) + NTF(z)·E(z) with NTF = (1 − z⁻¹)^L for an L-th
//! order modulator (+20·L dB/decade noise shaping). R. Schreier & G. C.
//! Temes, *Understanding Delta-Sigma Data Converters*, Wiley 2005, ch. 2–4.
//!
//! **CIC decimator.** N integrators at the input rate, ↓R, N combs
//! (1 − z^−M) at the output rate: H(z) = [(1 − z^−RM)/(1 − z⁻¹)]^N, so
//! |H(f)| = |sin(π f R M / fs) / sin(π f / fs)|^N, DC gain (RM)^N, and the
//! registers need B_in + N·log2(RM) bits; two's-complement wrap-around in
//! the integrators is harmless (E. B. Hogenauer, "An economical class of
//! digital filters for decimation and interpolation", IEEE Trans. ASSP
//! 29(2), 1981).
//!
//! **Aliasing.** After decimating to fs_out, every input frequency within
//! ±f_voice of a multiple of fs_out folds into the voice band. The metric
//! `dsp.decim_stopband_db` is the worst attenuation of the *whole chain*
//! over those bands, relative to DC.

use serde::{Deserialize, Serialize};

use super::fft::{spectrum, welch_psd, WindowKind};
use super::filter::fir_filter;
use super::fir::{least_squares, windowed_sinc, Band};
use crate::math::{C64, PI};
use crate::project::AudioParams;

/// Normalised CIC magnitude at frequency f (Hz) for input rate fs.
pub fn cic_mag(f: f64, fs: f64, n: u32, r: u32, m: u32) -> f64 {
    let rm = (r * m) as f64;
    let x = PI * f / fs;
    let den = rm * x.sin();
    let g = if den.abs() < 1e-12 { 1.0 } else { ((rm * x).sin() / den).abs() };
    g.powi(n as i32)
}

/// Options for the decimation chain (defaults = the chain Iris uses).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ChainOptions {
    /// CIC order N; 0 = choose the smallest N that clears the target in the alias bands.
    pub cic_order: u32,
    /// CIC differential delay M.
    pub cic_m: u32,
    /// Half-band stage taps (0 = no half-band; otherwise 4k+3, e.g. 7, 11, 15).
    pub halfband_taps: usize,
    /// Override for the final FIR length (0 = project `fir_taps`).
    pub fir_taps: usize,
    /// Stop-band weight of the least-squares final FIR.
    pub stop_weight: f64,
    /// Target attenuation used to auto-select the CIC order, dB.
    pub target_db: f64,
}

impl Default for ChainOptions {
    fn default() -> Self {
        ChainOptions {
            cic_order: 0,
            cic_m: 1,
            halfband_taps: 0,
            fir_taps: 0,
            stop_weight: 200.0,
            target_db: 60.0,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ChainDesign {
    pub pdm_hz: f64,
    pub fs_out: f64,
    pub decimation: u32,
    pub cic_order: u32,
    pub cic_r: u32,
    pub cic_m: u32,
    pub cic_out_hz: f64,
    /// Hogenauer register width for a 2-bit (±1) input.
    pub cic_register_bits: u32,
    pub cic_dc_gain: f64,
    pub halfband: Vec<f64>,
    pub fir: Vec<f64>,
    pub fir_in_hz: f64,
    pub fir_decim: u32,
    pub pass_hz: f64,
    pub stop_hz: f64,
    pub notes: Vec<String>,
}

impl ChainDesign {
    /// Complete-chain response (complex for the FIRs, magnitude for CIC), normalised to DC = 1.
    pub fn total_mag(&self, f: f64) -> f64 {
        let mut g = cic_mag(f, self.pdm_hz, self.cic_order, self.cic_r, self.cic_m);
        if !self.halfband.is_empty() {
            g *= super::fft::dtft(&self.halfband, f / self.cic_out_hz).abs();
        }
        g * super::fft::dtft(&self.fir, f / self.fir_in_hz).abs()
    }

    /// Frequency bands (Hz, at the PDM rate) that alias into 0…f_voice after decimation.
    pub fn alias_bands(&self, f_voice: f64) -> Vec<(f64, f64)> {
        let nyq = self.pdm_hz / 2.0;
        let mut out = Vec::new();
        let mut k = 1;
        loop {
            let c = k as f64 * self.fs_out;
            let lo = c - f_voice;
            if lo >= nyq {
                break;
            }
            out.push((lo.max(0.0), (c + f_voice).min(nyq)));
            k += 1;
        }
        out
    }

    /// Worst-case attenuation (dB, positive) over the alias bands and where it occurs.
    pub fn stopband_db(&self, f_voice: f64) -> (f64, f64) {
        let mut worst = (f64::INFINITY, 0.0);
        let dc = self.total_mag(0.0).max(1e-300);
        for (lo, hi) in self.alias_bands(f_voice) {
            let pts = 40;
            for i in 0..=pts {
                let f = lo + (hi - lo) * i as f64 / pts as f64;
                let a = -20.0 * (self.total_mag(f) / dc).max(1e-15).log10();
                if a < worst.0 {
                    worst = (a, f);
                }
            }
        }
        worst
    }

    /// Worst attenuation of the CIC alone over the alias bands that the FIR cannot help with
    /// (those within ±f_voice of multiples of the FIR input rate).
    pub fn cic_only_floor_db(&self, f_voice: f64) -> f64 {
        let mut worst = f64::INFINITY;
        let nyq = self.pdm_hz / 2.0;
        let mut k = 1;
        while k as f64 * self.fir_in_hz - f_voice < nyq {
            let c = k as f64 * self.fir_in_hz;
            for i in 0..=20 {
                let f = (c - f_voice + 2.0 * f_voice * i as f64 / 20.0).min(nyq);
                let mut g = cic_mag(f, self.pdm_hz, self.cic_order, self.cic_r, self.cic_m);
                if !self.halfband.is_empty() {
                    g *= super::fft::dtft(&self.halfband, f / self.cic_out_hz).abs();
                }
                worst = worst.min(-20.0 * g.max(1e-15).log10());
            }
            k += 1;
        }
        worst
    }
}

/// Worst CIC attenuation at the alias band edges f = k·f_out_cic ± f_voice.
fn cic_alias_floor(n: u32, r: u32, m: u32, pdm: f64, f_voice: f64) -> f64 {
    let fo = pdm / r as f64;
    let mut worst = f64::INFINITY;
    let mut k = 1;
    while k as f64 * fo - f_voice < pdm / 2.0 {
        for s in [-1.0, 1.0] {
            let f = (k as f64 * fo + s * f_voice).min(pdm / 2.0);
            worst = worst.min(-20.0 * cic_mag(f, pdm, n, r, m).max(1e-15).log10());
        }
        k += 1;
    }
    worst
}

/// Half-band low-pass (cut-off fs/4, odd taps 4k+3, every other tap zero).
pub fn halfband(taps: usize) -> Vec<f64> {
    let mut n = taps.max(3);
    while n % 4 != 3 {
        n += 1;
    }
    let beta = 5.0;
    let mut h = windowed_sinc(Band::Lowpass, n, 0.25, 0.0, 1.0, WindowKind::Kaiser, beta);
    let mid = (n - 1) / 2;
    for (i, v) in h.iter_mut().enumerate() {
        let k = i as isize - mid as isize;
        if k != 0 && k % 2 == 0 {
            *v = 0.0; // exact zeros (they are ≈0 already)
        }
    }
    h
}

/// Design the decimation chain from the project's audio parameters.
pub fn design_chain(audio: &AudioParams, opt: &ChainOptions) -> ChainDesign {
    let pdm = audio.pdm_clock_hz.max(1000.0);
    let d = audio.decimation.max(1);
    let mut notes = Vec::new();
    let fs_out = pdm / d as f64;
    if (fs_out - audio.sample_rate_hz).abs() > 1.0 {
        notes.push(format!(
            "pdm_clock / decimation = {fs_out:.0} Hz differs from sample_rate_hz = {:.0} Hz; using {fs_out:.0} Hz",
            audio.sample_rate_hz
        ));
    }
    let fir_decim = if d.is_multiple_of(2) && d >= 4 { 2 } else { 1 };
    let use_hb = opt.halfband_taps > 0 && (d / fir_decim).is_multiple_of(2) && d / fir_decim >= 4;
    let hb_decim = if use_hb { 2 } else { 1 };
    let r = (d / fir_decim / hb_decim).max(1);
    let m = opt.cic_m.clamp(1, 2);
    let f_voice = audio.voice_band_hi_hz.clamp(10.0, fs_out / 2.0);
    let n = if opt.cic_order > 0 {
        opt.cic_order.min(8)
    } else {
        // smallest order whose own alias-band floor clears the target + 3 dB
        // margin (the FIR's pass-band droop compensation eats ~1 dB of it).
        // With a half-band stage the CIC only has to protect the bands around
        // multiples of its own output rate.
        (2..=8)
            .find(|&n| cic_alias_floor(n, r, m, pdm, f_voice) >= opt.target_db + 3.0)
            .unwrap_or(8)
    };
    let cic_out = pdm / r as f64;
    let hb = if use_hb { halfband(opt.halfband_taps) } else { vec![] };
    let fir_in = cic_out / hb_decim as f64;
    let taps = if opt.fir_taps > 0 { opt.fir_taps } else { audio.fir_taps as usize }.clamp(3, 255);
    let pass = f_voice;
    let stop = (fs_out - f_voice).max(pass + 1.0);
    let stop = if fir_decim == 1 { fir_in / 2.0 * 0.95 } else { stop };
    // CIC (and half-band) droop compensation: D(f) = 1/|H_prev(f)| in the pass band.
    let prev = |fn_: f64| {
        let f = fn_ * fir_in;
        let mut g = cic_mag(f, pdm, n, r, m);
        if !hb.is_empty() {
            g *= super::fft::dtft(&hb, f / cic_out).abs();
        }
        g
    };
    let (pn, sn) = (pass / fir_in, stop / fir_in);
    let w_stop = opt.stop_weight.max(1.0);
    let mut fir = least_squares(
        taps,
        |f| if f <= pn { 1.0 / prev(f).max(1e-3) } else { 0.0 },
        |f| {
            if f <= pn {
                1.0
            } else if f >= sn {
                w_stop
            } else {
                0.0
            }
        },
        2048,
    );
    if fir.len() != taps {
        // even length requested: least_squares returns odd; trim symmetric ends
        notes.push(format!(
            "final FIR uses {} taps (type-I linear phase needs an odd length)",
            fir.len()
        ));
    }
    let g0 = fir.iter().sum::<f64>();
    if g0.abs() > 1e-9 {
        for v in fir.iter_mut() {
            *v /= g0;
        }
    }
    let rm = (r * m) as f64;
    ChainDesign {
        pdm_hz: pdm,
        fs_out,
        decimation: d,
        cic_order: n,
        cic_r: r,
        cic_m: m,
        cic_out_hz: cic_out,
        cic_register_bits: 2 + (n as f64 * rm.log2()).ceil() as u32,
        cic_dc_gain: rm.powi(n as i32),
        halfband: hb,
        fir,
        fir_in_hz: fir_in,
        fir_decim,
        pass_hz: pass,
        stop_hz: stop,
        notes,
    }
}

/// Smallest odd final-FIR length meeting `target_db` (None if the CIC floor makes it unreachable).
pub fn min_taps_for(audio: &AudioParams, opt: &ChainOptions, target_db: f64) -> Option<usize> {
    let f_voice = audio.voice_band_hi_hz;
    // the CIC floor does not depend on the FIR, so probe with a tiny one
    let probe = design_chain(
        audio,
        &ChainOptions {
            fir_taps: 3,
            ..opt.clone()
        },
    );
    if probe.cic_only_floor_db(f_voice) < target_db {
        return None;
    }
    let ok = |taps: usize| {
        design_chain(
            audio,
            &ChainOptions {
                fir_taps: taps,
                ..opt.clone()
            },
        )
        .stopband_db(f_voice)
        .0 >= target_db
    };
    // The least-squares attenuation is not strictly monotonic in the length,
    // so scan short odd lengths exhaustively, then bisect beyond 63 taps.
    let mut taps = 3;
    while taps <= 63 {
        if ok(taps) {
            return Some(taps);
        }
        taps += 2;
    }
    let (mut lo, mut hi) = (63usize, 127usize);
    while !ok(hi) {
        lo = hi;
        hi = hi * 2 + 1;
        if hi > 255 {
            return if ok(255) { Some(255) } else { None };
        }
    }
    while hi - lo > 2 {
        let mid = (((lo + hi) / 2) | 1).clamp(lo + 2, hi - 2);
        if ok(mid) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Some(hi)
}

// ------------------------------------------------------------------ simulation

/// L-th order (1 or 2) error-feedback sigma-delta modulator, output ±1.
pub fn sigma_delta(x: &[f64], order: u32) -> Vec<i8> {
    let mut e1 = 0.0;
    let mut e2 = 0.0;
    let mut out = Vec::with_capacity(x.len());
    for &xn in x {
        // NTF = (1 − z⁻¹)^L  ⇒  u = x − (2e₁ − e₂) for L = 2, u = x − e₁ for L = 1 … with
        // e defined as u − y the signs flip; we use y = u + q, q = y − u.
        let u = if order >= 2 { xn + 2.0 * e1 - e2 } else { xn + e1 };
        let y = if u >= 0.0 { 1.0 } else { -1.0 };
        let q = u - y; // negative quantisation error fed back
        e2 = e1;
        e1 = q;
        out.push(y as i8);
    }
    out
}

/// Integer CIC decimator with two's-complement wrap at `bits` (Hogenauer).
pub fn cic_decimate(x: &[i8], n: u32, r: u32, m: u32, bits: u32) -> Vec<i64> {
    let n = n as usize;
    let mask: i64 = if bits >= 63 { -1 } else { (1i64 << bits) - 1 };
    let wrap = |v: i64| -> i64 {
        if bits >= 63 {
            return v;
        }
        let u = v & mask;
        if u >= (1i64 << (bits - 1)) {
            u - (1i64 << bits)
        } else {
            u
        }
    };
    let mut integ = vec![0i64; n];
    let mut comb_hist: Vec<Vec<i64>> = vec![vec![0; m as usize]; n];
    let mut out = Vec::with_capacity(x.len() / r as usize + 1);
    for (i, &xi) in x.iter().enumerate() {
        let mut v = xi as i64;
        for s in integ.iter_mut() {
            *s = wrap(*s + v);
            v = *s;
        }
        if (i + 1) % r as usize == 0 {
            let mut c = v;
            for h in comb_hist.iter_mut() {
                let delayed = h[0];
                h.rotate_left(1);
                let last = h.len() - 1;
                h[last] = c;
                c = wrap(c - delayed);
            }
            out.push(c);
        }
    }
    out
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SimOptions {
    pub modulator_order: u32,
    pub tone_hz: f64,
    pub amplitude: f64,
    /// Output samples to simulate at fs_out.
    pub n_out: usize,
    /// Add a strong ultrasonic interferer (Hz, 0 = none) to show aliasing protection.
    pub interferer_hz: f64,
    pub interferer_amp: f64,
    /// Bits of the PDM stream returned for the animated strip.
    pub strip_bits: usize,
}

impl Default for SimOptions {
    fn default() -> Self {
        SimOptions {
            modulator_order: 2,
            tone_hz: 1000.0,
            amplitude: 0.5,
            n_out: 512,
            interferer_hz: 0.0,
            interferer_amp: 0.2,
            strip_bits: 384,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Trace {
    pub freq: Vec<f64>,
    pub db: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ChainSim {
    pub design: ChainDesign,
    pub stopband_db: f64,
    pub stopband_worst_hz: f64,
    pub cic_floor_db: f64,
    pub min_taps_60db: Option<usize>,
    pub passband_ripple_db: f64,
    pub alias_bands: Vec<(f64, f64)>,
    /// Stage magnitude responses over 0…pdm/2 (dB).
    pub resp_freq: Vec<f64>,
    pub resp_cic_db: Vec<f64>,
    pub resp_hb_db: Vec<f64>,
    pub resp_fir_db: Vec<f64>,
    pub resp_total_db: Vec<f64>,
    /// Pass-band detail 0…fs_out/2.
    pub pass_freq: Vec<f64>,
    pub pass_cic_db: Vec<f64>,
    pub pass_total_db: Vec<f64>,
    /// First bits of the PDM stream (0/1) and the analogue input at those instants.
    pub strip_bits: Vec<u8>,
    pub strip_input: Vec<f64>,
    /// PDM stream PSD (dB re full-scale, per bin) with the ideal NTF overlay.
    pub pdm_spectrum: Trace,
    pub ntf_db: Vec<f64>,
    pub cic_spectrum: Trace,
    pub out_spectrum: Trace,
    pub out_t: Vec<f64>,
    pub out_x: Vec<f64>,
    pub sndr_db: f64,
    pub enob: f64,
}

fn decimate_trace(freq: &[f64], db: &[f64], max_pts: usize) -> Trace {
    if freq.len() <= max_pts {
        return Trace {
            freq: freq.to_vec(),
            db: db.to_vec(),
        };
    }
    // keep the max within each bucket so tones stay visible
    let step = freq.len() as f64 / max_pts as f64;
    let mut f = Vec::with_capacity(max_pts);
    let mut d = Vec::with_capacity(max_pts);
    for b in 0..max_pts {
        let s = (b as f64 * step) as usize;
        let e = (((b + 1) as f64 * step) as usize).min(freq.len());
        let (mut bi, mut bv) = (s, f64::NEG_INFINITY);
        for (i, &v) in db.iter().enumerate().take(e).skip(s) {
            if v > bv {
                bv = v;
                bi = i;
            }
        }
        f.push(freq[bi]);
        d.push(bv);
    }
    Trace { freq: f, db: d }
}

/// Least-squares sine fit at a known frequency → SNDR of the residual.
fn sine_fit_sndr(x: &[f64], f: f64, fs: f64) -> f64 {
    let n = x.len();
    if n < 8 {
        return 0.0;
    }
    let w = 2.0 * PI * f / fs;
    let (mut ss, mut sc, mut cc, mut xs, mut xc, mut s1, mut c1, mut x1) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for (i, &v) in x.iter().enumerate() {
        let (s, c) = (w * i as f64).sin_cos();
        ss += s * s;
        sc += s * c;
        cc += c * c;
        xs += v * s;
        xc += v * c;
        s1 += s;
        c1 += c;
        x1 += v;
    }
    let a = vec![vec![ss, sc, s1], vec![sc, cc, c1], vec![s1, c1, n as f64]];
    let Some(p) = crate::math::solve(a, vec![xs, xc, x1]) else {
        return 0.0;
    };
    let mut sig = 0.0;
    let mut err = 0.0;
    for (i, &v) in x.iter().enumerate() {
        let (s, c) = (w * i as f64).sin_cos();
        let fit = p[0] * s + p[1] * c;
        sig += fit * fit;
        err += (v - fit - p[2]).powi(2);
    }
    10.0 * (sig / err.max(1e-300)).log10()
}

/// Run the whole chain on a test tone.
pub fn simulate(audio: &AudioParams, opt: &ChainOptions, sim: &SimOptions) -> ChainSim {
    let design = design_chain(audio, opt);
    let f_voice = audio.voice_band_hi_hz;
    let (sb, sb_f) = design.stopband_db(f_voice);
    let pdm = design.pdm_hz;
    let d = design.decimation as usize;
    let n_out = sim.n_out.clamp(64, 4096);
    // extra settling samples for the filters
    let settle = 48;
    let n_in = (n_out + settle) * d;
    let amp = sim.amplitude.clamp(0.0, if sim.modulator_order >= 2 { 0.7 } else { 0.95 });
    let x: Vec<f64> = (0..n_in)
        .map(|i| {
            let t = i as f64 / pdm;
            let mut v = amp * (2.0 * PI * sim.tone_hz * t).sin();
            if sim.interferer_hz > 0.0 {
                v += sim.interferer_amp.clamp(0.0, 0.3) * (2.0 * PI * sim.interferer_hz * t).sin();
            }
            v
        })
        .collect();
    let bits = sigma_delta(&x, sim.modulator_order.clamp(1, 2));
    // PDM spectrum (Welch, Hann)
    let bf: Vec<f64> = bits.iter().map(|&b| b as f64).collect();
    let psd = welch_psd(&bf, pdm, WindowKind::Blackman, 0.0, 4096, 0.5);
    // express as dB re a full-scale sine per bin: convert PSD·ENBW·df to power, then to amplitude
    let enbw_hz = 1.7268 * pdm / psd.seg_len as f64; // Blackman ENBW 1.73 bins
    let pdm_db: Vec<f64> = psd.psd.iter().map(|p| 10.0 * (p * enbw_hz * 2.0).max(1e-30).log10()).collect();
    let pdm_trace = decimate_trace(&psd.freq, &pdm_db, 700);
    let order = sim.modulator_order.clamp(1, 2) as i32;
    let ntf_db: Vec<f64> = pdm_trace
        .freq
        .iter()
        .map(|&f| {
            let mag = (2.0 * (PI * f / pdm).sin()).abs().max(1e-9).powi(order);
            // quantisation noise of a ±1 quantiser (Δ = 2, power 1/3) spread over fs/2, per ENBW bin
            20.0 * mag.log10() + 10.0 * ((1.0 / 3.0) * enbw_hz / (pdm / 2.0) * 2.0).log10()
        })
        .collect();
    // CIC
    let cic = cic_decimate(&bits, design.cic_order, design.cic_r, design.cic_m, design.cic_register_bits);
    let cic_f: Vec<f64> = cic.iter().map(|&v| v as f64 / design.cic_dc_gain).collect();
    // half-band
    let mid: Vec<f64> = if design.halfband.is_empty() {
        cic_f.clone()
    } else {
        fir_filter(&design.halfband, &cic_f).into_iter().step_by(2).collect()
    };
    let out_full: Vec<f64> = fir_filter(&design.fir, &mid)
        .into_iter()
        .step_by(design.fir_decim as usize)
        .collect();
    let skip = settle.min(out_full.len());
    let out_x: Vec<f64> = out_full[skip..].to_vec();
    let fs_out = design.fs_out;
    let out_t: Vec<f64> = (0..out_x.len()).map(|i| i as f64 / fs_out).collect();
    let cic_skip = (settle * d / design.cic_r as usize).min(cic_f.len());
    let cs = spectrum(&cic_f[cic_skip..], design.cic_out_hz, WindowKind::Blackman, 0.0, None);
    let os = spectrum(&out_x, fs_out, WindowKind::Blackman, 0.0, None);
    let sndr = sine_fit_sndr(&out_x, sim.tone_hz, fs_out);
    // responses
    let nyq = pdm / 2.0;
    let resp_freq: Vec<f64> = (0..=900).map(|i| nyq * i as f64 / 900.0).collect();
    let dbv = |g: f64| (20.0 * g.max(1e-12).log10()).max(-200.0);
    let resp_cic_db: Vec<f64> = resp_freq
        .iter()
        .map(|&f| dbv(cic_mag(f, pdm, design.cic_order, design.cic_r, design.cic_m)))
        .collect();
    let resp_hb_db: Vec<f64> = if design.halfband.is_empty() {
        vec![]
    } else {
        resp_freq
            .iter()
            .map(|&f| dbv(super::fft::dtft(&design.halfband, f / design.cic_out_hz).abs()))
            .collect()
    };
    let resp_fir_db: Vec<f64> = resp_freq
        .iter()
        .map(|&f| dbv(super::fft::dtft(&design.fir, f / design.fir_in_hz).abs()))
        .collect();
    let resp_total_db: Vec<f64> = resp_freq.iter().map(|&f| dbv(design.total_mag(f))).collect();
    let pass_freq: Vec<f64> = (0..=200).map(|i| fs_out / 2.0 * i as f64 / 200.0).collect();
    let pass_cic_db: Vec<f64> = pass_freq
        .iter()
        .map(|&f| dbv(cic_mag(f, pdm, design.cic_order, design.cic_r, design.cic_m)))
        .collect();
    let pass_total_db: Vec<f64> = pass_freq.iter().map(|&f| dbv(design.total_mag(f))).collect();
    let ripple = {
        let v: Vec<f64> = pass_freq
            .iter()
            .zip(&pass_total_db)
            .filter(|(f, _)| **f >= audio.voice_band_lo_hz.min(f_voice) && **f <= f_voice)
            .map(|(_, d)| *d)
            .collect();
        let mx = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mn = v.iter().cloned().fold(f64::INFINITY, f64::min);
        if v.is_empty() {
            0.0
        } else {
            mx - mn
        }
    };
    let strip = sim.strip_bits.clamp(16, 4096).min(bits.len());
    let strip_off = settle * d / 2;
    let strip_off = strip_off.min(bits.len() - strip);
    ChainSim {
        stopband_db: sb,
        stopband_worst_hz: sb_f,
        cic_floor_db: design.cic_only_floor_db(f_voice),
        min_taps_60db: min_taps_for(audio, opt, 60.0),
        passband_ripple_db: ripple,
        alias_bands: design.alias_bands(f_voice),
        resp_freq,
        resp_cic_db,
        resp_hb_db,
        resp_fir_db,
        resp_total_db,
        pass_freq,
        pass_cic_db,
        pass_total_db,
        strip_bits: bits[strip_off..strip_off + strip]
            .iter()
            .map(|&b| if b > 0 { 1 } else { 0 })
            .collect(),
        strip_input: x[strip_off..strip_off + strip].to_vec(),
        pdm_spectrum: pdm_trace,
        ntf_db,
        cic_spectrum: decimate_trace(&cs.freq, &cs.db, 700),
        out_spectrum: Trace { freq: os.freq, db: os.db },
        out_t,
        out_x,
        sndr_db: sndr,
        enob: (sndr - 1.76) / 6.02,
        design,
    }
}

/// Complex response of the complete chain (used by tests / chip lab).
pub fn chain_response(design: &ChainDesign, f: f64) -> C64 {
    C64::real(design.total_mag(f))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Project;

    #[test]
    fn cic_dc_gain_is_rm_to_the_n() {
        for (n, r, m) in [(3u32, 8u32, 1u32), (4, 16, 1), (2, 4, 2)] {
            let x = vec![1i8; 2000];
            let y = cic_decimate(&x, n, r, m, 2 + (n as f64 * ((r * m) as f64).log2()).ceil() as u32);
            let last = *y.last().unwrap();
            assert_eq!(last, ((r * m) as i64).pow(n), "N={n} R={r} M={m}");
        }
    }

    #[test]
    fn cic_wraparound_is_harmless() {
        // −1 input saturates nothing: wrapped result equals the exact one
        let x: Vec<i8> = (0..4096).map(|i| if (i / 37) % 3 == 0 { -1 } else { 1 }).collect();
        let exact = cic_decimate(&x, 4, 32, 1, 63);
        let wrapped = cic_decimate(&x, 4, 32, 1, 22);
        assert_eq!(exact, wrapped);
    }

    #[test]
    fn cic_magnitude_formula() {
        // first null at fs/R for M = 1
        assert!(cic_mag(1_024_000.0 / 32.0, 1_024_000.0, 4, 32, 1) < 1e-10);
        // hand calculation: N=4, R=32 at 28 kHz → 4·20log10(0.1394) ≈ −68.4 dB
        let g = 20.0 * cic_mag(28_000.0, 1_024_000.0, 4, 32, 1).log10();
        assert!((g + 68.4).abs() < 0.2, "{g}");
    }

    #[test]
    fn noise_shaping_pushes_noise_up() {
        let pdm = 1_024_000.0;
        let x: Vec<f64> = (0..65536).map(|i| 0.4 * (2.0 * PI * 1000.0 * i as f64 / pdm).sin()).collect();
        for order in [1, 2] {
            let b: Vec<f64> = sigma_delta(&x, order).into_iter().map(|v| v as f64).collect();
            let p = welch_psd(&b, pdm, WindowKind::Blackman, 0.0, 4096, 0.5);
            let band = |lo: f64, hi: f64| {
                p.freq
                    .iter()
                    .zip(&p.psd)
                    .filter(|(f, _)| **f >= lo && **f < hi)
                    .map(|(_, v)| v)
                    .sum::<f64>()
            };
            let low = band(5_000.0, 8_000.0);
            let high = band(300_000.0, 303_000.0);
            assert!(high > 100.0 * low, "order {order}: {low} vs {high}");
        }
    }

    #[test]
    fn iris_chain_meets_60db_and_recovers_the_tone() {
        let p = Project::iris();
        let d = design_chain(&p.design.audio, &ChainOptions::default());
        assert_eq!(d.cic_r * d.fir_decim, 64);
        assert_eq!(d.fir.len(), 31);
        let (sb, _) = d.stopband_db(p.design.audio.voice_band_hi_hz);
        assert!(sb >= 60.0, "stop band {sb}");
        let sim = simulate(&p.design.audio, &ChainOptions::default(), &SimOptions::default());
        assert!(sim.sndr_db > 60.0, "SNDR {}", sim.sndr_db);
        assert!(sim.passband_ripple_db < 1.0, "ripple {}", sim.passband_ripple_db);
        let min = sim.min_taps_60db.expect("reachable");
        assert!((3..=31).contains(&min), "min taps {min}");
        let dmin = design_chain(
            &p.design.audio,
            &ChainOptions {
                fir_taps: min,
                ..Default::default()
            },
        );
        assert!(dmin.stopband_db(4000.0).0 >= 60.0);
        for shorter in (3..min).step_by(2) {
            let d = design_chain(
                &p.design.audio,
                &ChainOptions {
                    fir_taps: shorter,
                    ..Default::default()
                },
            );
            assert!(d.stopband_db(4000.0).0 < 60.0, "{shorter} taps already pass");
        }
        // a 3rd-order CIC cannot reach 60 dB in the bands the FIR cannot touch
        assert!(min_taps_for(
            &p.design.audio,
            &ChainOptions {
                cic_order: 3,
                ..Default::default()
            },
            60.0
        )
        .is_none());
        let pk = sim.out_x.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        assert!((pk - 0.5).abs() < 0.05, "output amplitude {pk}");
    }

    #[test]
    fn halfband_has_zero_taps() {
        let h = halfband(11);
        assert_eq!(h.len(), 11);
        assert_eq!(h[1], 0.0);
        assert_eq!(h[3], 0.0);
        assert!((h[5] - 0.5).abs() < 0.02);
    }
}
