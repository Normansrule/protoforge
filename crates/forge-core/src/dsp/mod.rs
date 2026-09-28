//! # DSP Lab engine
//!
//! Spectral analysis, filter design and the two audio subsystems of the Iris
//! smart glasses: the PDM microphone decimation chain and the two-microphone
//! differential beamformer.
//!
//! | File | What |
//! |---|---|
//! | [`fft`] | radix-2 FFT/IFFT, windows (Harris 1978), amplitude spectrum, Welch PSD |
//! | [`signal`] | deterministic test signals (tones, noise, chirp, square/saw, vowel) |
//! | [`fir`] | windowed-sinc, Kaiser-formula and least-squares FIR design |
//! | [`iir`] | RBJ cookbook biquads, Butterworth via the bilinear transform |
//! | [`filter`] | DF-II-T filtering, frequency/phase/group delay, fixed-point |
//! | [`pdm`] | sigma-delta modulator, CIC (Hogenauer 1981), compensation FIR |
//! | [`beam`] | first-order differential and delay-and-sum arrays, DI, WNG |
//!
//! Metrics: `dsp.mic_di_db` (R-009) and `dsp.decim_stopband_db` (R-010).

pub mod beam;
pub mod fft;
pub mod filter;
pub mod fir;
pub mod iir;
pub mod pdm;
pub mod signal;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::math::C64;
use crate::project::{AudioParams, Metric, Project};
use fft::WindowKind;
use iir::Biquad;

// ------------------------------------------------------------------ inputs

fn default_fs() -> f64 {
    16000.0
}

#[derive(Deserialize)]
#[serde(default)]
struct AnalyzeIn {
    spec: signal::SignalSpec,
    window: WindowKind,
    beta: f64,
    nfft: Option<usize>,
}
impl Default for AnalyzeIn {
    fn default() -> Self {
        AnalyzeIn {
            spec: signal::SignalSpec::default(),
            window: WindowKind::Hann,
            beta: 8.6,
            nfft: None,
        }
    }
}

#[derive(Serialize)]
struct AnalyzeOut {
    signal: signal::Signal,
    spectrum: fft::Spectrum,
}

#[derive(Deserialize)]
#[serde(default)]
struct SpectrumIn {
    x: Vec<f64>,
    #[serde(default = "default_fs")]
    fs: f64,
    window: WindowKind,
    beta: f64,
    nfft: Option<usize>,
}
impl Default for SpectrumIn {
    fn default() -> Self {
        SpectrumIn {
            x: vec![],
            fs: 16000.0,
            window: WindowKind::Hann,
            beta: 8.6,
            nfft: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct PsdIn {
    x: Vec<f64>,
    fs: f64,
    window: WindowKind,
    beta: f64,
    seg_len: usize,
    overlap: f64,
}
impl Default for PsdIn {
    fn default() -> Self {
        PsdIn {
            x: vec![],
            fs: 16000.0,
            window: WindowKind::Hann,
            beta: 8.6,
            seg_len: 256,
            overlap: 0.5,
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct FftIn {
    re: Vec<f64>,
    im: Vec<f64>,
    inverse: bool,
}

#[derive(Serialize)]
struct FftOut {
    re: Vec<f64>,
    im: Vec<f64>,
    n: usize,
}

#[derive(Deserialize)]
#[serde(default)]
struct WindowIn {
    kind: WindowKind,
    n: usize,
    beta: f64,
}
impl Default for WindowIn {
    fn default() -> Self {
        WindowIn {
            kind: WindowKind::Hann,
            n: 64,
            beta: 8.6,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct LeakageIn {
    n: usize,
    /// Tone frequency in bins (e.g. 10.0 on a bin, 10.5 worst case).
    bin: f64,
    windows: Vec<WindowKind>,
    beta: f64,
}
impl Default for LeakageIn {
    fn default() -> Self {
        LeakageIn {
            n: 64,
            bin: 10.5,
            windows: vec![WindowKind::Rectangular, WindowKind::Hann, WindowKind::Blackman],
            beta: 8.6,
        }
    }
}

#[derive(Serialize)]
struct LeakageTrace {
    window: WindowKind,
    bins: Vec<f64>,
    db: Vec<f64>,
    /// Peak reading of the (coherent-gain corrected) DFT, dB (0 dB = true amplitude).
    peak_db: f64,
    /// Energy outside ±3 bins of the tone, dB re total.
    leakage_db: f64,
}

#[derive(Deserialize)]
#[serde(default)]
struct FirIn {
    band: fir::Band,
    method: String,
    window: WindowKind,
    beta: f64,
    taps: usize,
    f1: f64,
    f2: f64,
    fs: f64,
    atten_db: f64,
    transition_hz: f64,
    n_points: usize,
    log: bool,
}
impl Default for FirIn {
    fn default() -> Self {
        FirIn {
            band: fir::Band::Lowpass,
            method: "window".into(),
            window: WindowKind::Hamming,
            beta: 8.6,
            taps: 63,
            f1: 3400.0,
            f2: 5000.0,
            fs: 16000.0,
            atten_db: 60.0,
            transition_hz: 800.0,
            n_points: 512,
            log: false,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct IirIn {
    /// "butterworth" or "rbj".
    family: String,
    biquad: iir::BiquadKind,
    order: usize,
    highpass: bool,
    f0: f64,
    q: f64,
    gain_db: f64,
    fs: f64,
    n_points: usize,
    log: bool,
}
impl Default for IirIn {
    fn default() -> Self {
        IirIn {
            family: "butterworth".into(),
            biquad: iir::BiquadKind::Lowpass,
            order: 4,
            highpass: false,
            f0: 3400.0,
            q: std::f64::consts::FRAC_1_SQRT_2,
            gain_db: 6.0,
            fs: 16000.0,
            n_points: 512,
            log: false,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct TfIn {
    b: Vec<f64>,
    a: Vec<f64>,
    sos: Vec<Biquad>,
    fs: f64,
    n_points: usize,
    log: bool,
}
impl Default for TfIn {
    fn default() -> Self {
        TfIn {
            b: vec![1.0],
            a: vec![1.0],
            sos: vec![],
            fs: 16000.0,
            n_points: 512,
            log: false,
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ApplyIn {
    b: Vec<f64>,
    a: Vec<f64>,
    sos: Vec<Biquad>,
    x: Vec<f64>,
}

#[derive(Deserialize)]
#[serde(default)]
struct FixedIn {
    b: Vec<f64>,
    a: Vec<f64>,
    sos: Vec<Biquad>,
    fs: f64,
    coeff_bits: u32,
    data_bits: u32,
    x: Vec<f64>,
    n_points: usize,
}
impl Default for FixedIn {
    fn default() -> Self {
        FixedIn {
            b: vec![1.0],
            a: vec![1.0],
            sos: vec![],
            fs: 16000.0,
            coeff_bits: 12,
            data_bits: 16,
            x: vec![],
            n_points: 512,
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ChainIn {
    project: Option<Project>,
    audio: Option<AudioParams>,
    chain: pdm::ChainOptions,
    sim: pdm::SimOptions,
    /// Coefficient word length for the quantised-FIR comparison (0 = project chip.coeff_bits).
    coeff_bits: u32,
}

#[derive(Serialize)]
struct ChainOut {
    #[serde(flatten)]
    sim: pdm::ChainSim,
    coeff_bits: u32,
    stopband_quantized_db: f64,
}

#[derive(Serialize)]
struct DesignOut {
    design: pdm::ChainDesign,
    stopband_db: f64,
    stopband_worst_hz: f64,
}

// ------------------------------------------------------------------ outputs

/// Everything the filter designer shows.
#[derive(Serialize, Default)]
pub struct FilterReport {
    pub b: Vec<f64>,
    pub a: Vec<f64>,
    pub sections: Vec<Biquad>,
    pub fs: f64,
    pub freq: Vec<f64>,
    pub mag_db: Vec<f64>,
    pub phase_deg: Vec<f64>,
    pub group_delay: Vec<f64>,
    pub impulse: Vec<f64>,
    pub step: Vec<f64>,
    pub poles: Vec<[f64; 2]>,
    pub zeros: Vec<[f64; 2]>,
    /// Poles at the origin not listed individually (FIR: taps − 1).
    pub origin_poles: usize,
    pub stable: bool,
    pub max_pole_radius: f64,
    pub taps: usize,
    pub order: usize,
    pub beta: Option<f64>,
    pub notes: Vec<String>,
}

fn pz(v: &[C64]) -> Vec<[f64; 2]> {
    v.iter().map(|z| [z.re, z.im]).collect()
}

fn report_tf(b: Vec<f64>, a: Vec<f64>, fs: f64, n: usize, log: bool) -> FilterReport {
    let a = if a.is_empty() { vec![1.0] } else { a };
    let f = filter::grid(fs, n, log);
    let r = filter::freqz(&b, &a, fs, &f);
    let is_fir = a.len() == 1;
    let len = if is_fir { b.len().max(32) + 8 } else { 256 };
    let mut imp = vec![0.0; len];
    imp[0] = 1.0;
    let impulse = filter::lfilter(&b, &a, &imp);
    let step = filter::lfilter(&b, &a, &vec![1.0; len]);
    let mut notes = Vec::new();
    let (zeros, poles, origin) = if b.len() <= 160 && a.len() <= 40 {
        let mut z = roots(&b);
        let p = roots(&a);
        // align orders: B(z)/A(z) in z⁻¹ → extra poles/zeros at the origin
        let diff = b.len() as isize - a.len() as isize;
        let origin = if diff > 0 { diff as usize } else { 0 };
        if diff < 0 {
            z.extend(std::iter::repeat_n(C64::ZERO, (-diff) as usize));
        }
        (z, p, origin)
    } else {
        notes.push("too many coefficients for the pole–zero plot".into());
        (vec![], vec![], 0)
    };
    let max_r = poles.iter().fold(0.0f64, |m, p| m.max(p.abs()));
    FilterReport {
        taps: b.len(),
        order: (b.len().max(a.len())).saturating_sub(1),
        stable: iir::is_stable(&poles),
        max_pole_radius: max_r,
        b,
        a,
        sections: vec![],
        fs,
        freq: r.f,
        mag_db: r.mag_db,
        phase_deg: r.phase_deg,
        group_delay: r.group_delay,
        impulse,
        step,
        poles: pz(&poles),
        zeros: pz(&zeros),
        origin_poles: origin,
        beta: None,
        notes,
    }
}

fn report_sos(sos: Vec<Biquad>, fs: f64, n: usize, log: bool) -> FilterReport {
    let f = filter::grid(fs, n, log);
    let r = filter::sos_freqz(&sos, fs, &f);
    let len = 256;
    let mut imp = vec![0.0; len];
    imp[0] = 1.0;
    let impulse = filter::sos_filter(&sos, &imp);
    let step = filter::sos_filter(&sos, &vec![1.0; len]);
    let (p, z) = iir::sos_poles_zeros(&sos);
    let (b, a) = iir::sos_to_tf(&sos);
    FilterReport {
        taps: b.len(),
        order: p.len(),
        stable: iir::is_stable(&p),
        max_pole_radius: p.iter().fold(0.0f64, |m, q| m.max(q.abs())),
        b,
        a,
        sections: sos,
        fs,
        freq: r.f,
        mag_db: r.mag_db,
        phase_deg: r.phase_deg,
        group_delay: r.group_delay,
        impulse,
        step,
        poles: pz(&p),
        zeros: pz(&z),
        origin_poles: 0,
        beta: None,
        notes: vec![],
    }
}

fn fir_design(i: FirIn) -> Result<FilterReport, String> {
    let fs = i.fs.max(1.0);
    let (h, beta, note) = match i.method.as_str() {
        "kaiser" => {
            let d = fir::kaiser_design(i.band, i.atten_db, i.transition_hz, i.f1, i.f2, fs, 1025);
            let n = format!(
                "Kaiser: A = {:.0} dB, Δf = {:.0} Hz → β = {:.3}, {} taps",
                i.atten_db, i.transition_hz, d.beta, d.taps
            );
            (d.h, Some(d.beta), n)
        }
        "ls" => {
            let h = fir::ls_band(
                i.band,
                i.taps,
                i.f1,
                i.f2,
                i.transition_hz,
                fs,
                10f64.powf(i.atten_db / 40.0).min(1e4),
            );
            let n = format!(
                "least squares, {} taps, don't-care transition ±{:.0} Hz",
                h.len(),
                i.transition_hz / 2.0
            );
            (h, None, n)
        }
        _ => {
            let h = fir::windowed_sinc(i.band, i.taps, i.f1, i.f2, fs, i.window, i.beta);
            let n = format!("windowed sinc, {} taps, {:?} window", h.len(), i.window);
            (h, if i.window == WindowKind::Kaiser { Some(i.beta) } else { None }, n)
        }
    };
    let mut r = report_tf(h, vec![1.0], fs, i.n_points, i.log);
    r.beta = beta;
    r.notes.insert(0, note);
    Ok(r)
}

fn iir_design(i: IirIn) -> Result<FilterReport, String> {
    let fs = i.fs.max(1.0);
    let (sos, note) = if i.family == "rbj" {
        (
            vec![iir::rbj(i.biquad, i.f0, i.q, i.gain_db, fs)],
            format!("RBJ {:?}, f0 = {:.0} Hz, Q = {:.3}", i.biquad, i.f0, i.q),
        )
    } else {
        (
            iir::butterworth(i.order, i.f0, fs, i.highpass),
            format!(
                "Butterworth order {}, {} , fc = {:.0} Hz (bilinear, pre-warped)",
                i.order.clamp(1, 16),
                if i.highpass { "high-pass" } else { "low-pass" },
                i.f0
            ),
        )
    };
    let mut r = report_sos(sos, fs, i.n_points, i.log);
    r.notes.push(note);
    Ok(r)
}

#[derive(Serialize)]
struct FixedOut {
    coeff_bits: u32,
    data_bits: u32,
    /// Quantised coefficients (FIR: b; IIR: per-section b and a).
    b_q: filter::QFormat,
    a_q: Option<filter::QFormat>,
    sections_q: Vec<Biquad>,
    freq: Vec<f64>,
    float_db: Vec<f64>,
    fixed_db: Vec<f64>,
    stable_fixed: bool,
    max_pole_radius_fixed: f64,
    y_float: Vec<f64>,
    y_fixed: Vec<f64>,
    output_snr_db: f64,
    coeff_snr_db: f64,
    theory_snr_db: f64,
}

fn fixed_point(i: FixedIn) -> Result<FixedOut, String> {
    let fs = i.fs.max(1.0);
    let f = filter::grid(fs, i.n_points, false);
    let xq = filter::quantize_signal(&i.x, i.data_bits);
    let quant_out = |y: Vec<f64>| filter::quantize_signal(&y, i.data_bits);
    if !i.sos.is_empty() {
        // quantise every coefficient of every section with a common Q format per section
        let mut sections_q = Vec::new();
        for s in &i.sos {
            let all = [s.b[0], s.b[1], s.b[2], s.a[1], s.a[2]];
            let q = filter::quantize_coeffs(&all, i.coeff_bits);
            sections_q.push(Biquad {
                b: [q.values[0], q.values[1], q.values[2]],
                a: [1.0, q.values[3], q.values[4]],
            });
        }
        let fl = filter::sos_freqz(&i.sos, fs, &f);
        let fx = filter::sos_freqz(&sections_q, fs, &f);
        let y_float = filter::sos_filter(&i.sos, &i.x);
        // fixed: quantised coefficients and data, re-quantised between sections
        let mut y = xq.clone();
        for s in &sections_q {
            y = quant_out(filter::lfilter(&s.b, &s.a, &y));
        }
        let (p, _) = iir::sos_poles_zeros(&sections_q);
        let flat: Vec<f64> = i.sos.iter().flat_map(|s| [s.b[0], s.b[1], s.b[2], s.a[1], s.a[2]]).collect();
        let bq = filter::quantize_coeffs(&flat, i.coeff_bits);
        return Ok(FixedOut {
            coeff_bits: i.coeff_bits,
            data_bits: i.data_bits,
            coeff_snr_db: filter::snr_db(&flat, &bq.values),
            b_q: bq,
            a_q: None,
            sections_q,
            freq: f,
            float_db: fl.mag_db,
            fixed_db: fx.mag_db,
            stable_fixed: iir::is_stable(&p),
            max_pole_radius_fixed: p.iter().fold(0.0f64, |m, z| m.max(z.abs())),
            output_snr_db: filter::snr_db(&y_float, &y),
            theory_snr_db: 6.02 * i.data_bits as f64 + 1.76,
            y_float,
            y_fixed: y,
        });
    }
    let bq = filter::quantize_coeffs(&i.b, i.coeff_bits);
    let a_is_fir = i.a.len() <= 1;
    let aq = if a_is_fir {
        None
    } else {
        Some(filter::quantize_coeffs(&i.a, i.coeff_bits))
    };
    let a_vals = aq.as_ref().map(|q| q.values.clone()).unwrap_or_else(|| vec![1.0]);
    let fl = filter::freqz(&i.b, &i.a, fs, &f);
    let fx = filter::freqz(&bq.values, &a_vals, fs, &f);
    let y_float = filter::lfilter(&i.b, &i.a, &i.x);
    let y_fixed = quant_out(filter::lfilter(&bq.values, &a_vals, &xq));
    let poles = if a_is_fir { vec![] } else { roots(&a_vals) };
    Ok(FixedOut {
        coeff_bits: i.coeff_bits,
        data_bits: i.data_bits,
        coeff_snr_db: filter::snr_db(&i.b, &bq.values),
        b_q: bq,
        a_q: aq,
        sections_q: vec![],
        freq: f,
        float_db: fl.mag_db,
        fixed_db: fx.mag_db,
        stable_fixed: iir::is_stable(&poles),
        max_pole_radius_fixed: poles.iter().fold(0.0f64, |m, z| m.max(z.abs())),
        output_snr_db: filter::snr_db(&y_float, &y_fixed),
        theory_snr_db: 6.02 * i.data_bits as f64 + 1.76,
        y_float,
        y_fixed,
    })
}

fn leakage(i: LeakageIn) -> Result<Vec<LeakageTrace>, String> {
    let n = i.n.clamp(8, 4096);
    let x: Vec<f64> = (0..n)
        .map(|k| (2.0 * std::f64::consts::PI * i.bin * k as f64 / n as f64).cos())
        .collect();
    let mut out = Vec::new();
    for &w in &i.windows {
        let s = fft::spectrum(&x, n as f64, w, i.beta, Some(n));
        let fine = fft::spectrum(&x, n as f64, w, i.beta, Some(n * 16));
        let total: f64 = s.amp.iter().map(|a| a * a).sum();
        let lo = (i.bin - 3.0).max(0.0);
        let hi = i.bin + 3.0;
        let outside: f64 = s
            .amp
            .iter()
            .enumerate()
            .filter(|(k, _)| (*k as f64) < lo || (*k as f64) > hi)
            .map(|(_, a)| a * a)
            .sum();
        let peak = s.db.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        out.push(LeakageTrace {
            window: w,
            bins: fine.freq.clone(),
            db: fine.db.iter().map(|d| d.max(-160.0)).collect(),
            peak_db: peak,
            leakage_db: 10.0 * (outside / total.max(1e-300)).max(1e-16).log10(),
        });
    }
    Ok(out)
}

fn audio_of(i: &ChainIn) -> AudioParams {
    i.audio
        .clone()
        .or_else(|| i.project.as_ref().map(|p| p.design.audio.clone()))
        .unwrap_or_default()
}

/// Polynomial roots (highest power first) by the Aberth–Ehrlich iteration.
///
/// The initial guesses lie on a circle whose radius is the geometric mean of
/// the root magnitudes, |a_n / a_0|^(1/n), which keeps long linear-phase FIR
/// polynomials (roots clustered on the unit circle, tiny end taps) well
/// behaved where a Cauchy-bound start diverges. O. Aberth, "Iteration methods
/// for finding all zeros of a polynomial simultaneously", Math. Comp. 27, 1973.
pub fn roots(coeffs: &[f64]) -> Vec<C64> {
    let mut c: Vec<f64> = coeffs.to_vec();
    while c.len() > 1 && c[0].abs() < 1e-300 {
        c.remove(0);
    }
    // roots at the origin (trailing zeros)
    let mut zeros_at_origin = 0;
    while c.len() > 1 && c[c.len() - 1].abs() < 1e-300 {
        c.pop();
        zeros_at_origin += 1;
    }
    let n = c.len().saturating_sub(1);
    let mut out = vec![C64::ZERO; zeros_at_origin];
    if n == 0 {
        return out;
    }
    let lead = c[0];
    let a: Vec<f64> = c.iter().map(|v| v / lead).collect();
    let r0 = a[n].abs().powf(1.0 / n as f64).max(1e-6);
    let mut z: Vec<C64> = (0..n)
        .map(|k| C64::from_polar(r0, 2.0 * std::f64::consts::PI * (k as f64 + 0.25) / n as f64 + 0.3))
        .collect();
    let eval = |x: C64| {
        let mut p = C64::ZERO;
        let mut dp = C64::ZERO;
        for &ai in &a {
            dp = dp * x + p;
            p = p * x + ai;
        }
        (p, dp)
    };
    for _ in 0..800 {
        let mut max_step: f64 = 0.0;
        for i in 0..n {
            let (p, dp) = eval(z[i]);
            if p.abs() == 0.0 {
                continue;
            }
            let ratio = p / dp;
            let mut s = C64::ZERO;
            for j in 0..n {
                if j != i {
                    let d = z[i] - z[j];
                    if d.abs() > 1e-300 {
                        s = s + d.inv();
                    }
                }
            }
            let denom = C64::ONE - ratio * s;
            let w = if denom.abs() > 1e-300 { ratio / denom } else { ratio };
            z[i] = z[i] - w;
            max_step = max_step.max(w.abs() / (1.0 + z[i].abs()));
        }
        if max_step < 1e-14 {
            break;
        }
    }
    for r in z.iter_mut() {
        if r.im.abs() < 1e-9 * (1.0 + r.re.abs()) {
            r.im = 0.0;
        }
    }
    out.extend(z);
    out
}

// ------------------------------------------------------------------ API

pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "dsp.signal" => handle(input, |i: signal::SignalSpec| Ok(signal::generate(&i))),
        "dsp.analyze" => handle(input, |i: AnalyzeIn| {
            let s = signal::generate(&i.spec);
            let sp = fft::spectrum(&s.x, s.fs, i.window, i.beta, i.nfft);
            Ok(AnalyzeOut { signal: s, spectrum: sp })
        }),
        "dsp.spectrum" => handle(input, |i: SpectrumIn| Ok(fft::spectrum(&i.x, i.fs, i.window, i.beta, i.nfft))),
        "dsp.psd" => handle(input, |i: PsdIn| {
            Ok(fft::welch_psd(&i.x, i.fs, i.window, i.beta, i.seg_len, i.overlap))
        }),
        "dsp.fft" => handle(input, |i: FftIn| {
            let x: Vec<C64> =
                i.re.iter()
                    .enumerate()
                    .map(|(k, &r)| C64::new(r, i.im.get(k).copied().unwrap_or(0.0)))
                    .collect();
            let y = if i.inverse { fft::ifft(&x) } else { fft::fft(&x, None) };
            Ok(FftOut {
                n: y.len(),
                re: y.iter().map(|v| v.re).collect(),
                im: y.iter().map(|v| v.im).collect(),
            })
        }),
        "dsp.window" => handle(input, |i: WindowIn| Ok(fft::window_info(i.kind, i.n, i.beta))),
        "dsp.leakage" => handle(input, leakage),
        "dsp.fir_design" => handle(input, fir_design),
        "dsp.iir_design" => handle(input, iir_design),
        "dsp.filter_response" => handle(input, |i: TfIn| {
            Ok(if i.sos.is_empty() {
                report_tf(i.b, i.a, i.fs.max(1.0), i.n_points, i.log)
            } else {
                report_sos(i.sos, i.fs.max(1.0), i.n_points, i.log)
            })
        }),
        "dsp.apply" => handle(input, |i: ApplyIn| {
            let y = if !i.sos.is_empty() {
                filter::sos_filter(&i.sos, &i.x)
            } else {
                filter::lfilter(&i.b, if i.a.is_empty() { &[1.0] } else { &i.a }, &i.x)
            };
            Ok(serde_json::json!({ "y": y }))
        }),
        "dsp.fixed_point" => handle(input, fixed_point),
        "dsp.pdm_chain" => handle(input, |i: ChainIn| {
            let audio = audio_of(&i);
            let bits = if i.coeff_bits > 0 {
                i.coeff_bits
            } else {
                i.project.as_ref().map(|p| p.design.chip.coeff_bits).unwrap_or(12)
            };
            let sim = pdm::simulate(&audio, &i.chain, &i.sim);
            let q = quantized_stopband(&sim.design, audio.voice_band_hi_hz, bits);
            Ok(ChainOut {
                sim,
                coeff_bits: bits,
                stopband_quantized_db: q,
            })
        }),
        "dsp.decimation_design" => handle(input, |i: ChainIn| {
            let audio = audio_of(&i);
            let d = pdm::design_chain(&audio, &i.chain);
            let (sb, f) = d.stopband_db(audio.voice_band_hi_hz);
            Ok(DesignOut {
                design: d,
                stopband_db: sb,
                stopband_worst_hz: f,
            })
        }),
        "dsp.beamformer" => handle(input, |i: beam::BeamInput| Ok(beam::analyze(&i))),
        "dsp.beam_field" => handle(input, |i: beam::FieldInput| Ok(beam::field(&i))),
        _ => None,
    }
}

/// Stop-band attenuation with the final FIR quantised to `bits`.
pub fn quantized_stopband(d: &pdm::ChainDesign, f_voice: f64, bits: u32) -> f64 {
    let mut dq = d.clone();
    dq.fir = filter::quantize_coeffs(&d.fir, bits).values;
    dq.stopband_db(f_voice).0
}

/// The beam pattern Iris uses: hypercardioid, which maximises the
/// directivity index (diffuse-noise rejection, what R-009 asks for) among
/// first-order differential arrays.
pub const IRIS_PATTERN: beam::Pattern = beam::Pattern::Hypercardioid;

pub fn metrics(p: &Project) -> Vec<Metric> {
    let a = &p.design.audio;
    let mut out = Vec::new();
    let d = (a.mic_spacing_mm / 1000.0).max(1e-4);
    let c = beam::SPEED_OF_SOUND;
    if a.mic_count >= 2 {
        let tr = IRIS_PATTERN.tau_ratio(0.0);
        let di = beam::directivity_index(IRIS_PATTERN, tr, 1000.0, d, c);
        let wng = beam::white_noise_gain(IRIS_PATTERN, tr, 1000.0, d, c);
        out.push(
            Metric::new("dsp.mic_di_db", "Mic array directivity index @ 1 kHz", di, "dB", "dsp").with_note(&format!(
                "hypercardioid delay-and-subtract, d = {:.1} mm, τ = d/3c = {:.1} µs (ideal limit 6.02 dB); white-noise gain {:.1} dB",
                a.mic_spacing_mm,
                tr * d / c * 1e6,
                wng
            )),
        );
    } else {
        out.push(
            Metric::new("dsp.mic_di_db", "Mic array directivity index @ 1 kHz", 0.0, "dB", "dsp")
                .with_note("single omnidirectional microphone"),
        );
    }
    let chain = pdm::design_chain(a, &pdm::ChainOptions::default());
    let (sb, f) = chain.stopband_db(a.voice_band_hi_hz);
    let q = quantized_stopband(&chain, a.voice_band_hi_hz, p.design.chip.coeff_bits);
    out.push(
        Metric::new("dsp.decim_stopband_db", "PDM decimation alias attenuation", sb, "dB", "dsp").with_note(&format!(
            "CIC N={} R={} M={} + {}-tap LS compensation FIR ↓{}; worst alias at {:.1} kHz; with {}-bit coefficients {:.1} dB",
            chain.cic_order,
            chain.cic_r,
            chain.cic_m,
            chain.fir.len(),
            chain.fir_decim,
            f / 1000.0,
            p.design.chip.coeff_bits,
            q
        )),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_metrics_are_sensible() {
        let p = Project::iris();
        let t0 = std::time::Instant::now();
        let m = metrics(&p);
        let dt = t0.elapsed();
        let get = |k: &str| m.iter().find(|x| x.key == k).unwrap().value;
        let di = get("dsp.mic_di_db");
        assert!(di > 5.5 && di < 6.1, "DI {di}");
        let sb = get("dsp.decim_stopband_db");
        assert!((60.0..120.0).contains(&sb), "stop band {sb}");
        eprintln!(
            "dsp metrics: DI {di:.2} dB, stop band {sb:.1} dB in {dt:?}; {:?}",
            m.iter().map(|x| &x.note).collect::<Vec<_>>()
        );
    }

    #[test]
    fn roots_of_a_long_linear_phase_fir_lie_on_or_mirror_about_the_unit_circle() {
        let h = fir::windowed_sinc(fir::Band::Lowpass, 63, 3400.0, 0.0, 16000.0, WindowKind::Hamming, 0.0);
        let z = roots(&h);
        assert_eq!(z.len(), 62);
        for r in &z {
            // backward error: |p(z)| relative to Σ|a_i||z|^(n−i)
            let scale: f64 = h
                .iter()
                .enumerate()
                .map(|(i, a)| a.abs() * r.abs().powi((h.len() - 1 - i) as i32))
                .sum();
            let v = crate::math::poly_eval(&h, *r).abs() / scale;
            assert!(v < 1e-9, "relative residual {v} at {r:?}");
        }
        let on_circle = z.iter().filter(|r| (r.abs() - 1.0).abs() < 1e-3).count();
        assert!(on_circle > 30, "stop-band zeros on |z| = 1: {on_circle}");
        // (z-1)(z-2)(z+3) = z³ − 7z + 6
        let mut r = roots(&[1.0, 0.0, -7.0, 6.0]);
        r.sort_by(|a, b| a.re.partial_cmp(&b.re).unwrap());
        assert!((r[0].re + 3.0).abs() < 1e-10 && (r[2].re - 2.0).abs() < 1e-10);
    }

    #[test]
    fn dispatch_round_trips() {
        let r = dispatch("dsp.fir_design", &serde_json::json!({"taps": 31, "f1": 2000, "fs": 8000}))
            .unwrap()
            .unwrap();
        assert_eq!(r["b"].as_array().unwrap().len(), 31);
        let r = dispatch("dsp.iir_design", &serde_json::json!({"order": 3})).unwrap().unwrap();
        assert_eq!(r["poles"].as_array().unwrap().len(), 3);
        assert!(r["stable"].as_bool().unwrap());
        let r = dispatch("dsp.pdm_chain", &serde_json::json!({"project": Project::iris()}))
            .unwrap()
            .unwrap();
        assert!(r["stopband_db"].as_f64().unwrap() > 60.0);
        assert!(dispatch("dsp.nope", &Value::Null).is_none());
    }
}
