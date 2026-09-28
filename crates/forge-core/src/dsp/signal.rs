//! Deterministic test-signal generator: tones, white noise, chirps,
//! square/sawtooth waves (naive or band-limited) and a synthetic
//! "voice-like" vowel (glottal harmonic series shaped by formant resonances,
//! the source–filter model of G. Fant, *Acoustic Theory of Speech
//! Production*, 1960).

use serde::{Deserialize, Serialize};

use crate::math::{Rng, PI};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    #[default]
    Tone,
    Noise,
    Chirp,
    Square,
    Saw,
    Voice,
    Impulse,
}

/// One additive component of a test signal.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Component {
    pub kind: ComponentKind,
    /// Frequency (tone/square/saw), start frequency (chirp), or pitch f0 (voice), Hz.
    pub freq: f64,
    /// End frequency of a chirp, Hz.
    pub freq_end: f64,
    /// Peak amplitude (tone/square/saw/chirp/voice) or RMS (noise).
    pub amp: f64,
    pub phase_deg: f64,
    /// Square/saw: synthesise only harmonics below Nyquist (no aliasing).
    pub bandlimited: bool,
    /// Vowel for the voice model: "a", "i", "u", "e", "o".
    pub vowel: String,
    pub seed: u64,
}

impl Default for Component {
    fn default() -> Self {
        Component {
            kind: ComponentKind::Tone,
            freq: 1000.0,
            freq_end: 4000.0,
            amp: 0.5,
            phase_deg: 0.0,
            bandlimited: false,
            vowel: "a".into(),
            seed: 1,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SignalSpec {
    pub fs: f64,
    pub n: usize,
    pub components: Vec<Component>,
}

impl Default for SignalSpec {
    fn default() -> Self {
        SignalSpec {
            fs: 16000.0,
            n: 2048,
            components: vec![Component::default()],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Signal {
    pub fs: f64,
    /// Sample times (not serialised — t[i] = i / fs).
    #[serde(skip)]
    pub t: Vec<f64>,
    pub x: Vec<f64>,
    pub rms: f64,
    pub peak: f64,
}

/// Formant frequencies (Hz) and bandwidths for five vowels, adult male
/// averages after G. E. Peterson & H. L. Barney, JASA 24, 1952 (F1–F3).
fn formants(vowel: &str) -> [(f64, f64); 3] {
    match vowel {
        "i" => [(270.0, 60.0), (2290.0, 100.0), (3010.0, 150.0)],
        "u" => [(300.0, 60.0), (870.0, 90.0), (2240.0, 150.0)],
        "e" => [(530.0, 70.0), (1840.0, 100.0), (2480.0, 150.0)],
        "o" => [(570.0, 70.0), (840.0, 90.0), (2410.0, 150.0)],
        _ => [(730.0, 80.0), (1090.0, 90.0), (2440.0, 150.0)],
    }
}

/// Magnitude of a cascade of second-order formant resonators at f.
fn formant_gain(f: f64, fm: &[(f64, f64); 3]) -> f64 {
    fm.iter()
        .map(|&(fc, bw)| {
            // |H| of a resonator normalised to 1 at DC: fc² / sqrt((fc²-f²)² + (bw f)²)
            let num = fc * fc;
            let den = ((fc * fc - f * f).powi(2) + (bw * f).powi(2)).sqrt();
            num / den
        })
        .product()
}

/// Generate the sum of all components.
pub fn generate(spec: &SignalSpec) -> Signal {
    let fs = spec.fs.max(1.0);
    let n = spec.n.clamp(1, 1 << 20);
    let mut x = vec![0.0; n];
    let nyq = fs / 2.0;
    for c in &spec.components {
        let ph0 = c.phase_deg.to_radians();
        match c.kind {
            ComponentKind::Tone => {
                let w = 2.0 * PI * c.freq / fs;
                for (i, v) in x.iter_mut().enumerate() {
                    *v += c.amp * (w * i as f64 + ph0).sin();
                }
            }
            ComponentKind::Noise => {
                let mut rng = Rng::new(c.seed);
                for v in x.iter_mut() {
                    *v += c.amp * rng.normal();
                }
            }
            ComponentKind::Chirp => {
                // linear chirp: f(t) = f0 + (f1-f0) t / T, phase = 2π(f0 t + k t²/2)
                let t_total = n as f64 / fs;
                let k = (c.freq_end - c.freq) / t_total;
                for (i, v) in x.iter_mut().enumerate() {
                    let t = i as f64 / fs;
                    *v += c.amp * (2.0 * PI * (c.freq * t + 0.5 * k * t * t) + ph0).sin();
                }
            }
            ComponentKind::Square | ComponentKind::Saw => {
                let square = c.kind == ComponentKind::Square;
                if c.bandlimited {
                    // Fourier series truncated below Nyquist
                    let mut h = 1;
                    while (h as f64) * c.freq < nyq {
                        let (coef, skip) = if square {
                            (4.0 / (PI * h as f64), h % 2 == 0)
                        } else {
                            (2.0 / (PI * h as f64) * if h % 2 == 0 { -1.0 } else { 1.0 }, false)
                        };
                        if !skip {
                            let w = 2.0 * PI * c.freq * h as f64 / fs;
                            for (i, v) in x.iter_mut().enumerate() {
                                *v += c.amp * coef * (w * i as f64 + h as f64 * ph0).sin();
                            }
                        }
                        h += 1;
                    }
                } else {
                    for (i, v) in x.iter_mut().enumerate() {
                        let cyc = (c.freq * i as f64 / fs + ph0 / (2.0 * PI)).rem_euclid(1.0);
                        *v += c.amp
                            * if square {
                                if cyc < 0.5 {
                                    1.0
                                } else {
                                    -1.0
                                }
                            } else {
                                2.0 * ((cyc + 0.5).rem_euclid(1.0)) - 1.0
                            };
                    }
                }
            }
            ComponentKind::Voice => {
                // glottal source: harmonics of f0 (with 5 Hz vibrato ±2 %) with a
                // −12 dB/octave spectral tilt, shaped by three formant resonators,
                // and a 4 Hz syllabic amplitude envelope.
                let f0 = c.freq.clamp(60.0, 400.0);
                let fm = formants(&c.vowel);
                let mut harmonics = Vec::new();
                let mut h = 1;
                while (h as f64) * f0 * 1.02 < nyq.min(5000.0) {
                    let f = h as f64 * f0;
                    let g = formant_gain(f, &fm) / (h as f64).powi(2);
                    harmonics.push((h as f64, g));
                    h += 1;
                }
                let norm: f64 = harmonics.iter().map(|(_, g)| g).sum::<f64>().max(1e-9);
                let mut phase = 0.0;
                let mut rng = Rng::new(c.seed);
                let hph: Vec<f64> = harmonics.iter().map(|_| rng.uniform() * 2.0 * PI).collect();
                for (i, v) in x.iter_mut().enumerate() {
                    let t = i as f64 / fs;
                    let finst = f0 * (1.0 + 0.02 * (2.0 * PI * 5.0 * t).sin());
                    phase += 2.0 * PI * finst / fs;
                    let env = 0.55 + 0.45 * (2.0 * PI * 4.0 * t - PI / 2.0).sin();
                    let s: f64 = harmonics.iter().zip(&hph).map(|((hh, g), p)| g * (hh * phase + p).sin()).sum();
                    *v += c.amp * env * s / norm * 1.6;
                }
            }
            ComponentKind::Impulse => {
                let at = ((c.freq.max(0.0)) as usize).min(n - 1);
                x[at] += c.amp;
            }
        }
    }
    let t = (0..n).map(|i| i as f64 / fs).collect();
    let rms = crate::math::rms(&x);
    let peak = x.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    Signal { fs, t, x, rms, peak }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generator_is_deterministic_and_has_expected_rms() {
        let spec = SignalSpec {
            fs: 8000.0,
            n: 8000,
            components: vec![
                Component {
                    kind: ComponentKind::Tone,
                    freq: 440.0,
                    amp: 1.0,
                    ..Default::default()
                },
                Component {
                    kind: ComponentKind::Noise,
                    amp: 0.1,
                    seed: 5,
                    ..Default::default()
                },
            ],
        };
        let a = generate(&spec);
        let b = generate(&spec);
        assert_eq!(a.x, b.x);
        // rms of sine 1/√2 plus noise 0.1 → sqrt(0.5 + 0.01)
        assert!((a.rms - (0.51f64).sqrt()).abs() < 0.01, "{}", a.rms);
    }

    #[test]
    fn bandlimited_square_has_no_energy_above_nyquist_aliases() {
        let fs = 16000.0;
        let spec = SignalSpec {
            fs,
            n: 4096,
            components: vec![Component {
                kind: ComponentKind::Square,
                freq: 1000.0,
                amp: 1.0,
                bandlimited: true,
                ..Default::default()
            }],
        };
        let s = generate(&spec);
        let sp = super::super::fft::spectrum(&s.x, fs, super::super::fft::WindowKind::Blackman, 0.0, None);
        // odd harmonics 1k,3k,5k,7k only; 2k should be far down
        let idx = |f: f64| (f / sp.bin_hz).round() as usize;
        assert!(sp.db[idx(1000.0)] > -1.0);
        assert!(sp.db[idx(2000.0)] < -80.0);
    }
}
