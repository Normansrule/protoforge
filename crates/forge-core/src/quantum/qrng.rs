//! Quantum random number generator (QRNG) design bench.
//!
//! Two entropy sources (review: M. Herrero-Collantes & J. C. Garcia-Escartin,
//! "Quantum random number generators", Rev. Mod. Phys. 89, 015004, 2017):
//!
//! * **Photon at a beam splitter.** Each photon leaves by the transmitted
//!   (bit 1) or reflected (bit 0) port with probability t / 1 − t; unequal
//!   detector efficiencies η₁, η₀ bias the output:
//!   `P(1) = t η₁ / (t η₁ + (1 − t) η₀)`.
//! * **Reverse-biased diode avalanche noise** read by an analog-to-digital
//!   converter (ADC): white Gaussian noise (optionally band-limited by the
//!   amplifier, a first-order low-pass), plus 1/f "flicker" drift (a sum of
//!   octave-spaced first-order processes, equal power per octave), a DC
//!   offset and mains pick-up, quantised by an N-bit ADC with a given full
//!   scale. The noise is shot/avalanche noise of a physical junction; only
//!   part of it is "quantum", so the conservative practice is to credit the
//!   measured min-entropy, not the ideal.
//!
//! Digitisation: a comparator at mid-scale (1 bit/sample) or the k
//! least-significant ADC bits. Conditioning: von Neumann debiasing (pairs
//! 01 → 0, 10 → 1, 00/11 dropped; unbiased output for independent input,
//! J. von Neumann 1951), XOR folding of k bits (piling-up lemma: bias ε →
//! 2^(k−1) ε^k), and a Toeplitz-hashing extractor (a 2-universal hash; the
//! leftover hash lemma gives l = ⌊m·h − 2 log₂(1/ε)⌋ nearly-uniform bits from
//! m input bits of min-entropy h per bit; Impagliazzo, Levin & Luby 1989;
//! see also X. Ma et al., "Postprocessing for quantum random-number
//! generators", Phys. Rev. A 87, 062327, 2013).

use serde::{Deserialize, Serialize};

use super::nist::{self, TestResult};
use crate::math::{Rng, PI};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct QrngIn {
    /// "diode" or "photon".
    pub source: String,
    pub samples: usize,
    pub seed: u64,
    // photon source
    pub split_ratio: f64,
    pub eff1: f64,
    pub eff0: f64,
    pub photon_rate_hz: f64,
    // diode source (all voltages referred to the ADC input)
    pub noise_rms_mv: f64,
    pub flicker_rms_mv: f64,
    pub offset_mv: f64,
    pub hum_mv: f64,
    pub hum_hz: f64,
    /// Amplifier −3 dB bandwidth, Hz (0 = white to the sample rate).
    pub bandwidth_hz: f64,
    pub sample_rate_hz: f64,
    pub adc_bits: u32,
    /// ADC full-scale range, peak-to-peak, mV.
    pub adc_vpp_mv: f64,
    /// "comparator" or "lsb".
    pub digitize: String,
    pub lsb_bits: u32,
    // conditioning
    pub von_neumann: bool,
    pub xor_fold: usize,
    pub toeplitz: bool,
    pub toeplitz_block: usize,
    /// Extractor security parameter ε = 2^−security_bits.
    pub security_bits: f64,
}

impl Default for QrngIn {
    fn default() -> Self {
        QrngIn {
            source: "diode".into(),
            samples: 40_000,
            seed: 42,
            split_ratio: 0.5,
            eff1: 0.62,
            eff0: 0.6,
            photon_rate_hz: 1e6,
            noise_rms_mv: 40.0,
            flicker_rms_mv: 4.0,
            offset_mv: 3.0,
            hum_mv: 0.0,
            hum_hz: 60.0,
            bandwidth_hz: 0.0,
            sample_rate_hz: 10e6,
            adc_bits: 8,
            adc_vpp_mv: 400.0,
            digitize: "lsb".into(),
            lsb_bits: 4,
            von_neumann: false,
            xor_fold: 1,
            toeplitz: true,
            toeplitz_block: 1024,
            security_bits: 64.0,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Stage {
    pub id: String,
    pub name: String,
    pub bits: usize,
    pub ones_fraction: f64,
    /// Output bits per input bit of the raw stream.
    pub efficiency: f64,
    pub rate_bps: f64,
    /// MCV min-entropy per bit.
    pub min_entropy_per_bit: f64,
    pub tests: Vec<TestResult>,
    pub passed: usize,
    pub applicable: usize,
    /// First bits for the bitmap view ('0'/'1').
    pub preview: String,
    /// Full stream (capped) as hex, MSB first.
    pub hex: String,
    pub autocorr: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct QrngOut {
    pub source: String,
    /// First samples: analog (mV) and ADC codes (diode) or bits (photon).
    pub analog_mv: Vec<f64>,
    pub codes: Vec<u32>,
    pub hist_x: Vec<f64>,
    pub hist_counts: Vec<u64>,
    pub hist_expected: Vec<f64>,
    pub bits_per_sample: u32,
    pub clipped_fraction: f64,
    pub total_rms_mv: f64,
    pub lsb_mv: f64,
    /// MCV min-entropy of the digitised symbol (k-bit value), bits per sample.
    pub min_entropy_per_sample: f64,
    /// Ideal (Gaussian, unclipped) min-entropy of the full ADC code, bits.
    pub min_entropy_theory: f64,
    /// Bias P(1) − ½ of the raw bits.
    pub raw_bias: f64,
    pub photon_p1: Option<f64>,
    pub sample_rate_hz: f64,
    pub stages: Vec<Stage>,
    /// Final extracted rate if the ADC ran at `sample_rate_hz`, bits/s.
    pub output_rate_bps: f64,
    pub notes: Vec<String>,
}

fn bits_to_hex(bits: &[u8], cap_bits: usize) -> String {
    let n = bits.len().min(cap_bits) / 8 * 8;
    let mut s = String::with_capacity(n / 4);
    for chunk in bits[..n].chunks(4) {
        let v = chunk.iter().fold(0u8, |a, &b| (a << 1) | b);
        s.push(std::char::from_digit(v as u32, 16).unwrap());
    }
    s
}

pub fn autocorr(bits: &[u8], lags: usize) -> Vec<f64> {
    let n = bits.len();
    if n < 10 {
        return vec![];
    }
    let x: Vec<f64> = bits.iter().map(|&b| if b == 1 { 1.0 } else { -1.0 }).collect();
    let mean = x.iter().sum::<f64>() / n as f64;
    let var = x.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    (1..=lags)
        .map(|l| {
            if l >= n || var == 0.0 {
                return 0.0;
            }
            x.iter().zip(&x[l..]).map(|(a, b)| (a - mean) * (b - mean)).sum::<f64>() / ((n - l) as f64 * var)
        })
        .collect()
}

pub fn von_neumann(bits: &[u8]) -> Vec<u8> {
    bits.chunks_exact(2).filter(|p| p[0] != p[1]).map(|p| p[0]).collect()
}

pub fn xor_fold(bits: &[u8], k: usize) -> Vec<u8> {
    if k <= 1 {
        return bits.to_vec();
    }
    bits.chunks_exact(k).map(|c| c.iter().fold(0u8, |a, &b| a ^ b)).collect()
}

/// Read 64 bits starting at bit `off` from a packed LSB-first word array.
fn get64(w: &[u64], off: usize) -> u64 {
    let (i, s) = (off / 64, off % 64);
    let lo = w.get(i).copied().unwrap_or(0) >> s;
    if s == 0 {
        lo
    } else {
        lo | (w.get(i + 1).copied().unwrap_or(0) << (64 - s))
    }
}

/// Toeplitz hashing: blocks of m input bits → l output bits, y = T x over
/// GF(2) with T[i][j] = s[i − j + m − 1] for a seed s of m + l − 1 bits.
pub fn toeplitz(bits: &[u8], m: usize, l: usize, seed: u64) -> Vec<u8> {
    if l == 0 || m == 0 || bits.len() < m {
        return vec![];
    }
    let len = m + l - 1;
    let mut rng = Rng::new(seed);
    let s: Vec<u8> = (0..len).map(|_| (rng.next_u64() & 1) as u8).collect();
    // r[k] = s[len−1−k]; row i uses r[len−m−i .. len−i] against x[0..m]
    let mut rw = vec![0u64; len.div_ceil(64) + 1];
    for k in 0..len {
        if s[len - 1 - k] == 1 {
            rw[k / 64] |= 1 << (k % 64);
        }
    }
    let words = m.div_ceil(64);
    let mut out = Vec::with_capacity(bits.len() / m * l);
    for block in bits.chunks_exact(m) {
        let mut xw = vec![0u64; words];
        for (j, &b) in block.iter().enumerate() {
            if b == 1 {
                xw[j / 64] |= 1 << (j % 64);
            }
        }
        for i in 0..l {
            let base = len - m - i;
            let mut acc = 0u64;
            for (k, &x) in xw.iter().enumerate() {
                let mut r = get64(&rw, base + 64 * k);
                if k == words - 1 && !m.is_multiple_of(64) {
                    r &= (1u64 << (m % 64)) - 1;
                }
                acc ^= r & x;
            }
            out.push((acc.count_ones() & 1) as u8);
        }
    }
    out
}

fn stage(id: &str, name: &str, bits: &[u8], raw_len: usize, raw_rate: f64) -> Stage {
    let tests = nist::suite(bits);
    let applicable = tests.iter().filter(|t| t.applicable).count();
    let passed = tests.iter().filter(|t| t.applicable && t.pass).count();
    let syms: Vec<u32> = bits.iter().map(|&b| b as u32).collect();
    let eff = if raw_len > 0 { bits.len() as f64 / raw_len as f64 } else { 0.0 };
    Stage {
        id: id.into(),
        name: name.into(),
        bits: bits.len(),
        ones_fraction: if bits.is_empty() { 0.0 } else { bits.iter().map(|&b| b as f64).sum::<f64>() / bits.len() as f64 },
        efficiency: eff,
        rate_bps: raw_rate * eff,
        min_entropy_per_bit: nist::mcv_min_entropy(&syms).0,
        tests,
        passed,
        applicable,
        preview: bits.iter().take(4096).map(|&b| if b == 1 { '1' } else { '0' }).collect(),
        hex: bits_to_hex(bits, 1 << 19),
        autocorr: autocorr(bits, 16),
    }
}

/// Gaussian CDF.
fn phi_cdf(x: f64) -> f64 {
    0.5 * nist::erfc(-x / 2f64.sqrt())
}

pub fn simulate(i: &QrngIn) -> QrngOut {
    let n = i.samples.clamp(1000, 400_000);
    let mut rng = Rng::new(i.seed);
    let mut notes = vec![];
    let mut raw: Vec<u8> = vec![];
    let mut analog = vec![];
    let mut codes_preview = vec![];
    let mut symbols: Vec<u32> = vec![];
    let (mut hist_x, mut hist_counts, mut hist_expected) = (vec![], vec![], vec![]);
    let (mut clipped, mut total_rms, mut lsb) = (0.0, 0.0, 0.0);
    let h_theory;
    let mut bps = 1u32;
    let mut photon_p1 = None;
    let raw_rate;
    let sample_rate;
    if i.source == "photon" {
        let t = i.split_ratio.clamp(0.0, 1.0);
        let (e1, e0) = (i.eff1.clamp(1e-6, 1.0), i.eff0.clamp(1e-6, 1.0));
        let p1 = t * e1 / (t * e1 + (1.0 - t) * e0);
        photon_p1 = Some(p1);
        for _ in 0..n {
            let b = u8::from(rng.uniform() < p1);
            raw.push(b);
            symbols.push(b as u32);
        }
        codes_preview = raw.iter().take(400).map(|&b| b as u32).collect();
        // detection rate: photons × probability that one detector clicks
        sample_rate = i.photon_rate_hz * (t * e1 + (1.0 - t) * e0);
        raw_rate = sample_rate;
        h_theory = -(p1.max(1.0 - p1)).log2();
        hist_x = vec![0.0, 1.0];
        hist_counts = vec![raw.iter().filter(|&&b| b == 0).count() as u64, raw.iter().filter(|&&b| b == 1).count() as u64];
        hist_expected = vec![(1.0 - p1) * n as f64, p1 * n as f64];
    } else {
        let fs = i.sample_rate_hz.max(1.0);
        let bits = i.adc_bits.clamp(2, 16);
        let levels = 1u64 << bits;
        lsb = i.adc_vpp_mv.max(1e-9) / levels as f64;
        // band-limited white noise: AR(1) with a = exp(−2π f_c / f_s), scaled to keep the RMS
        let a = if i.bandwidth_hz > 0.0 { (-2.0 * PI * i.bandwidth_hz / fs).exp() } else { 0.0 };
        let gain = (1.0 - a * a).sqrt();
        let mut w_state = 0.0;
        // 1/f: 16 octave-spaced AR(1) processes, unit variance each
        let octaves = 16;
        let coefs: Vec<f64> = (0..octaves).map(|k| (-1.0 / 2f64.powi(k as i32 + 1)).exp()).collect();
        // start each process in its stationary (unit-variance) distribution
        let mut fl: Vec<f64> = (0..octaves).map(|_| rng.normal()).collect();
        let fl_scale = i.flicker_rms_mv / (octaves as f64).sqrt();
        let mut sum_sq = 0.0;
        let mut n_clip = 0usize;
        let mut counts = vec![0u64; levels as usize];
        let k_bits = if i.digitize == "comparator" { 1 } else { i.lsb_bits.clamp(1, bits) };
        bps = k_bits;
        for s in 0..n {
            let w = rng.normal();
            w_state = a * w_state + gain * w;
            let mut f = 0.0;
            for k in 0..octaves {
                let c = coefs[k];
                fl[k] = c * fl[k] + (1.0 - c * c).sqrt() * rng.normal();
                f += fl[k];
            }
            let t = s as f64 / fs;
            let v = i.noise_rms_mv * w_state + fl_scale * f + i.offset_mv + i.hum_mv * (2.0 * PI * i.hum_hz * t).sin();
            sum_sq += (v - i.offset_mv).powi(2);
            let code_f = ((v + i.adc_vpp_mv / 2.0) / lsb).floor();
            let code = if code_f < 0.0 {
                n_clip += 1;
                0
            } else if code_f >= levels as f64 {
                n_clip += 1;
                levels - 1
            } else {
                code_f as u64
            } as u32;
            counts[code as usize] += 1;
            if s < 1000 {
                analog.push((v * 1000.0).round() / 1000.0);
                codes_preview.push(code);
            }
            if i.digitize == "comparator" {
                let b = u8::from(code as u64 >= levels / 2);
                raw.push(b);
                symbols.push(b as u32);
            } else {
                let sym = code & ((1u32 << k_bits) - 1);
                symbols.push(sym);
                for kb in (0..k_bits).rev() {
                    raw.push(((sym >> kb) & 1) as u8);
                }
            }
        }
        clipped = n_clip as f64 / n as f64;
        total_rms = (sum_sq / n as f64).sqrt();
        let sigma = (i.noise_rms_mv.powi(2) + i.flicker_rms_mv.powi(2) + i.hum_mv.powi(2) / 2.0).sqrt();
        // expected histogram from a Gaussian of the total RMS (hum treated as extra variance)
        let bin_group = (levels as usize / 256).max(1);
        let nb = levels as usize / bin_group;
        for b in 0..nb {
            let lo_code = (b * bin_group) as f64;
            let hi_code = ((b + 1) * bin_group) as f64;
            let v_lo = lo_code * lsb - i.adc_vpp_mv / 2.0 - i.offset_mv;
            let v_hi = hi_code * lsb - i.adc_vpp_mv / 2.0 - i.offset_mv;
            let pe = phi_cdf(v_hi / sigma) - phi_cdf(v_lo / sigma);
            hist_x.push(((b * bin_group) as f64 + bin_group as f64 / 2.0 - levels as f64 / 2.0) * lsb);
            hist_counts.push(counts[b * bin_group..(b + 1) * bin_group].iter().sum());
            hist_expected.push(pe * n as f64);
        }
        // ideal min-entropy of the full code: most likely bin ≈ LSB/(σ√2π)
        let pmax = (phi_cdf(0.5 * lsb / sigma) - phi_cdf(-0.5 * lsb / sigma)).max(1e-300);
        h_theory = -pmax.log2();
        if clipped > 0.001 {
            notes.push(format!(
                "{:.1} % of samples clip at the ADC rails — reduce the gain or widen the input range",
                clipped * 100.0
            ));
        }
        if sigma < 2.0 * lsb {
            notes.push("noise RMS is below 2 LSB: the ADC cannot resolve the noise; add gain".into());
        }
        if i.bandwidth_hz > 0.0 && i.bandwidth_hz < fs / 2.0 {
            notes.push(format!(
                "amplifier bandwidth {:.2} MHz is below f_s/2 = {:.2} MHz: consecutive samples are correlated (see the autocorrelation)",
                i.bandwidth_hz / 1e6,
                fs / 2e6
            ));
        }
        sample_rate = fs;
        raw_rate = fs * bps as f64;
    }
    let (h_sample, _) = nist::mcv_min_entropy(&symbols);
    let raw_bias = if raw.is_empty() { 0.0 } else { raw.iter().map(|&b| b as f64).sum::<f64>() / raw.len() as f64 - 0.5 };
    let raw_len = raw.len();
    let mut stages = vec![stage("raw", "Raw bits", &raw, raw_len, raw_rate)];
    let mut cur = raw.clone();
    // min-entropy per bit entering the extractor: per-sample MCV spread over its bits
    let mut h_bit = (h_sample / bps as f64).min(1.0);
    if i.von_neumann {
        cur = von_neumann(&cur);
        stages.push(stage("vn", "von Neumann debiased", &cur, raw_len, raw_rate));
        h_bit = stages.last().unwrap().min_entropy_per_bit;
    }
    if i.xor_fold > 1 {
        let k = i.xor_fold.min(16);
        cur = xor_fold(&cur, k);
        stages.push(stage("xor", &format!("XOR-folded ({k}:1)"), &cur, raw_len, raw_rate));
        h_bit = (h_bit * k as f64).min(stages.last().unwrap().min_entropy_per_bit).min(1.0);
    }
    if i.toeplitz {
        let m = i.toeplitz_block.clamp(64, 8192);
        let l = ((m as f64 * h_bit - 2.0 * i.security_bits.max(0.0)).floor()).max(0.0) as usize;
        let l = l.min(m);
        if l == 0 {
            notes.push("the Toeplitz block is too short for this min-entropy and security level — raise the block size".into());
        }
        cur = toeplitz(&cur, m, l, i.seed ^ 0x7E57);
        stages.push(stage("toeplitz", &format!("Toeplitz-hashed ({m} → {l})"), &cur, raw_len, raw_rate));
    }
    let output_rate_bps = stages.last().map(|s| s.rate_bps).unwrap_or(0.0);
    QrngOut {
        source: i.source.clone(),
        analog_mv: analog,
        codes: codes_preview,
        hist_x,
        hist_counts,
        hist_expected,
        bits_per_sample: bps,
        clipped_fraction: clipped,
        total_rms_mv: total_rms,
        lsb_mv: lsb,
        min_entropy_per_sample: h_sample,
        min_entropy_theory: h_theory,
        raw_bias,
        photon_p1,
        sample_rate_hz: sample_rate,
        stages,
        output_rate_bps,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn von_neumann_output_is_unbiased() {
        let mut rng = Rng::new(9);
        let biased: Vec<u8> = (0..400_000).map(|_| u8::from(rng.uniform() < 0.8)).collect();
        let out = von_neumann(&biased);
        let ones = out.iter().filter(|&&b| b == 1).count() as f64 / out.len() as f64;
        assert!((ones - 0.5).abs() < 0.005, "{ones}");
        // expected yield: 2p(1 − p) output bits per input pair
        assert!((out.len() as f64 / 200_000.0 - 0.32).abs() < 0.005);
    }

    #[test]
    fn xor_fold_follows_piling_up_lemma() {
        let mut rng = Rng::new(4);
        let bits: Vec<u8> = (0..400_000).map(|_| u8::from(rng.uniform() < 0.6)).collect();
        let out = xor_fold(&bits, 2);
        // bias ε = 0.1 → 2 ε² = 0.02: P(1) = 0.5 − 0.02
        let p1 = out.iter().filter(|&&b| b == 1).count() as f64 / out.len() as f64;
        assert!((p1 - 0.48).abs() < 0.004, "{p1}");
    }

    #[test]
    fn toeplitz_is_linear_and_right_sized() {
        let mut rng = Rng::new(1);
        let a: Vec<u8> = (0..1024).map(|_| (rng.next_u64() & 1) as u8).collect();
        let b: Vec<u8> = (0..1024).map(|_| (rng.next_u64() & 1) as u8).collect();
        let x: Vec<u8> = a.iter().zip(&b).map(|(p, q)| p ^ q).collect();
        let (ta, tb, tx) = (toeplitz(&a, 1024, 700, 3), toeplitz(&b, 1024, 700, 3), toeplitz(&x, 1024, 700, 3));
        assert_eq!(tx.len(), 700);
        for k in 0..700 {
            assert_eq!(tx[k], ta[k] ^ tb[k]);
        }
        // brute-force check of one row against the definition
        let len = 1024 + 700 - 1;
        let mut r = Rng::new(3);
        let s: Vec<u8> = (0..len).map(|_| (r.next_u64() & 1) as u8).collect();
        for i in [0usize, 1, 350, 699] {
            let mut acc = 0u8;
            for j in 0..1024 {
                acc ^= s[i + 1024 - 1 - j] & a[j];
            }
            assert_eq!(acc, ta[i], "row {i}");
        }
    }

    #[test]
    fn diode_pipeline_passes_nist_after_extraction() {
        let o = simulate(&QrngIn::default());
        let last = o.stages.last().unwrap();
        assert_eq!(last.passed, last.applicable, "{:?}", last.tests.iter().map(|t| (&t.name, t.p_value)).collect::<Vec<_>>());
        assert!(o.min_entropy_per_sample > 3.0 && o.min_entropy_per_sample <= 4.0);
        assert!(o.clipped_fraction < 0.001);
        // 8-bit ADC, 400 mV → LSB 1.5625 mV; σ ≈ 40.2 mV → H∞ ≈ log2(σ√2π / LSB) ≈ 6.7 bits
        assert!((o.min_entropy_theory - (40.2f64 * (2.0 * PI).sqrt() / 1.5625).log2()).abs() < 0.05, "{}", o.min_entropy_theory);
    }

    #[test]
    fn comparator_with_offset_is_biased() {
        let o = simulate(&QrngIn {
            digitize: "comparator".into(),
            offset_mv: 10.0,
            flicker_rms_mv: 0.0,
            toeplitz: false,
            ..Default::default()
        });
        // P(v > 0) with v ~ N(10 − ~LSB/2 edge, 40): ≈ Φ(10/40) ≈ 0.599
        assert!(o.raw_bias > 0.07 && o.raw_bias < 0.13, "{}", o.raw_bias);
        let p = simulate(&QrngIn {
            source: "photon".into(),
            ..Default::default()
        });
        let p1 = p.photon_p1.unwrap();
        assert!((p1 - 0.62 / 1.22).abs() < 1e-12);
    }
}
