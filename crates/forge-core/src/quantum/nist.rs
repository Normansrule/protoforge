//! Statistical tests for random bit streams.
//!
//! A subset of NIST SP 800-22 Rev. 1a (Rukhin et al., 2010), implemented
//! from the formulas in §2 and checked against its worked examples:
//! frequency (monobit) §2.1, frequency within a block §2.2, runs §2.3,
//! longest run of ones in a block §2.4, serial §2.11 and approximate
//! entropy §2.12. A test passes when its p-value ≥ α = 0.01.
//!
//! Plus the most-common-value min-entropy estimate of NIST SP 800-90B
//! (Turan et al., 2018) §6.3.1: p̂ = max count / L,
//! p_u = min(1, p̂ + 2.576 √(p̂(1 − p̂)/(L − 1))), H = −log₂ p_u.
//!
//! Special functions: ln Γ by the Lanczos approximation (g = 7), the
//! regularised upper incomplete gamma Q(a, x) by series / continued
//! fraction (Numerical Recipes, Press et al., 3rd ed., §6.2), and
//! erfc(x) = Q(½, x²).

use serde::Serialize;

pub const ALPHA: f64 = 0.01;

pub fn ln_gamma(x: f64) -> f64 {
    const G: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        return (std::f64::consts::PI / (std::f64::consts::PI * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = G[0];
    let t = x + 7.5;
    for (i, g) in G.iter().enumerate().skip(1) {
        a += g / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// Regularised upper incomplete gamma Q(a, x) = Γ(a, x)/Γ(a) ("igamc" in SP 800-22).
pub fn igamc(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    if a <= 0.0 {
        return 0.0;
    }
    let gln = ln_gamma(a);
    if x < a + 1.0 {
        // series for P
        let mut ap = a;
        let mut sum = 1.0 / a;
        let mut del = sum;
        for _ in 0..10_000 {
            ap += 1.0;
            del *= x / ap;
            sum += del;
            if del.abs() < sum.abs() * 1e-16 {
                break;
            }
        }
        (1.0 - sum * (-x + a * x.ln() - gln).exp()).clamp(0.0, 1.0)
    } else {
        // Lentz continued fraction for Q
        let tiny = 1e-300;
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / tiny;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..10_000 {
            let an = -(i as f64) * (i as f64 - a);
            b += 2.0;
            d = an * d + b;
            if d.abs() < tiny {
                d = tiny;
            }
            c = b + an / c;
            if c.abs() < tiny {
                c = tiny;
            }
            d = 1.0 / d;
            let del = d * c;
            h *= del;
            if (del - 1.0).abs() < 1e-16 {
                break;
            }
        }
        ((-x + a * x.ln() - gln).exp() * h).clamp(0.0, 1.0)
    }
}

pub fn erfc(x: f64) -> f64 {
    if x >= 0.0 {
        igamc(0.5, x * x)
    } else {
        2.0 - igamc(0.5, x * x)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TestResult {
    pub name: String,
    pub section: String,
    pub p_value: f64,
    /// Second p-value (serial test).
    pub p_value2: Option<f64>,
    pub pass: bool,
    pub statistic: String,
    pub note: String,
    pub applicable: bool,
}

fn result(name: &str, section: &str, p: f64, stat: String, note: &str) -> TestResult {
    TestResult {
        name: name.into(),
        section: section.into(),
        p_value: p,
        p_value2: None,
        pass: p >= ALPHA,
        statistic: stat,
        note: note.into(),
        applicable: true,
    }
}

fn not_applicable(name: &str, section: &str, why: &str) -> TestResult {
    TestResult {
        name: name.into(),
        section: section.into(),
        p_value: f64::NAN,
        p_value2: None,
        pass: false,
        statistic: String::new(),
        note: why.into(),
        applicable: false,
    }
}

pub fn monobit(bits: &[u8]) -> TestResult {
    let n = bits.len();
    if n < 100 {
        return not_applicable("Frequency (monobit)", "2.1", "needs n ≥ 100 bits");
    }
    let s: i64 = bits.iter().map(|&b| if b == 1 { 1 } else { -1 }).sum();
    let sobs = (s.abs() as f64) / (n as f64).sqrt();
    result(
        "Frequency (monobit)",
        "2.1",
        erfc(sobs / 2f64.sqrt()),
        format!("S_n = {s}, s_obs = {sobs:.4}"),
        "Are ones and zeros equally likely overall?",
    )
}

pub fn monobit_p(bits: &[u8]) -> f64 {
    let n = bits.len();
    let s: i64 = bits.iter().map(|&b| if b == 1 { 1 } else { -1 }).sum();
    erfc((s.abs() as f64) / (n as f64).sqrt() / 2f64.sqrt())
}

pub fn block_frequency(bits: &[u8], m: usize) -> TestResult {
    let n = bits.len();
    let nb = n / m.max(1);
    if n < 100 || nb == 0 {
        return not_applicable("Frequency within a block", "2.2", "needs n ≥ 100 bits");
    }
    let chi: f64 = 4.0
        * m as f64
        * (0..nb)
            .map(|k| {
                let pi = bits[k * m..(k + 1) * m].iter().map(|&b| b as f64).sum::<f64>() / m as f64;
                (pi - 0.5).powi(2)
            })
            .sum::<f64>();
    result(
        "Frequency within a block",
        "2.2",
        igamc(nb as f64 / 2.0, chi / 2.0),
        format!("M = {m}, N = {nb}, χ² = {chi:.4}"),
        "Is each block of M bits balanced?",
    )
}

pub fn block_frequency_p(bits: &[u8], m: usize) -> f64 {
    let nb = bits.len() / m;
    let chi: f64 = 4.0
        * m as f64
        * (0..nb)
            .map(|k| (bits[k * m..(k + 1) * m].iter().map(|&b| b as f64).sum::<f64>() / m as f64 - 0.5).powi(2))
            .sum::<f64>();
    igamc(nb as f64 / 2.0, chi / 2.0)
}

pub fn runs_p(bits: &[u8]) -> Option<f64> {
    let n = bits.len() as f64;
    let pi = bits.iter().map(|&b| b as f64).sum::<f64>() / n;
    if (pi - 0.5).abs() >= 2.0 / n.sqrt() {
        return None;
    }
    let v = 1 + bits.windows(2).filter(|w| w[0] != w[1]).count();
    Some(erfc((v as f64 - 2.0 * n * pi * (1.0 - pi)).abs() / (2.0 * (2.0 * n).sqrt() * pi * (1.0 - pi))))
}

pub fn runs(bits: &[u8]) -> TestResult {
    if bits.len() < 100 {
        return not_applicable("Runs", "2.3", "needs n ≥ 100 bits");
    }
    let v = 1 + bits.windows(2).filter(|w| w[0] != w[1]).count();
    match runs_p(bits) {
        Some(p) => result("Runs", "2.3", p, format!("V_n = {v}"), "Do 0↔1 switches happen as often as chance predicts?"),
        None => TestResult {
            p_value: 0.0,
            ..result("Runs", "2.3", 0.0, format!("V_n = {v}"), "Frequency prerequisite failed (|π − ½| ≥ 2/√n), so the runs test fails.")
        },
    }
}

/// Longest run of ones: class counts → p-value (SP 800-22 §2.4 tables).
pub fn longest_run_from_counts(v: &[usize], pis: &[f64], n_blocks: usize) -> (f64, f64) {
    let chi: f64 = v
        .iter()
        .zip(pis)
        .map(|(&vi, &p)| {
            let e = n_blocks as f64 * p;
            (vi as f64 - e).powi(2) / e
        })
        .sum();
    (chi, igamc((v.len() - 1) as f64 / 2.0, chi / 2.0))
}

pub fn longest_run(bits: &[u8]) -> TestResult {
    let n = bits.len();
    if n < 128 {
        return not_applicable("Longest run of ones in a block", "2.4", "needs n ≥ 128 bits");
    }
    let (m, lo, pis): (usize, usize, Vec<f64>) = if n < 6272 {
        (8, 1, vec![0.2148, 0.3672, 0.2305, 0.1875])
    } else if n < 750_000 {
        (128, 4, vec![0.1174, 0.2430, 0.2493, 0.1752, 0.1027, 0.1124])
    } else {
        (10_000, 10, vec![0.0882, 0.2092, 0.2483, 0.1933, 0.1208, 0.0675, 0.0727])
    };
    let nb = n / m;
    let k = pis.len();
    let mut v = vec![0usize; k];
    for b in 0..nb {
        let mut best = 0usize;
        let mut run = 0usize;
        for &x in &bits[b * m..(b + 1) * m] {
            if x == 1 {
                run += 1;
                best = best.max(run);
            } else {
                run = 0;
            }
        }
        let class = best.saturating_sub(lo).min(k - 1);
        let class = if best < lo { 0 } else { class };
        v[class] += 1;
    }
    let (chi, p) = longest_run_from_counts(&v, &pis, nb);
    result(
        "Longest run of ones in a block",
        "2.4",
        p,
        format!("M = {m}, ν = {v:?}, χ² = {chi:.4}"),
        "Are the longest streaks of ones the right length?",
    )
}

fn pattern_counts(bits: &[u8], m: usize) -> Vec<u64> {
    let n = bits.len();
    let mut c = vec![0u64; 1 << m];
    if m == 0 {
        return c;
    }
    for i in 0..n {
        let mut v = 0usize;
        for j in 0..m {
            v = (v << 1) | bits[(i + j) % n] as usize;
        }
        c[v] += 1;
    }
    c
}

fn psi2(bits: &[u8], m: usize) -> f64 {
    if m == 0 {
        return 0.0;
    }
    let n = bits.len() as f64;
    let c = pattern_counts(bits, m);
    (1u64 << m) as f64 / n * c.iter().map(|&x| (x as f64).powi(2)).sum::<f64>() - n
}

pub fn serial_p(bits: &[u8], m: usize) -> (f64, f64) {
    let (a, b, c) = (psi2(bits, m), psi2(bits, m - 1), if m >= 2 { psi2(bits, m - 2) } else { 0.0 });
    let d1 = a - b;
    let d2 = a - 2.0 * b + c;
    (igamc((1u64 << (m - 2)) as f64, d1 / 2.0), igamc((1u64 << (m - 3)) as f64, d2 / 2.0))
}

pub fn serial(bits: &[u8], m: usize) -> TestResult {
    let n = bits.len();
    if n < 100 || m < 3 {
        return not_applicable("Serial", "2.11", "needs n ≥ 100 bits");
    }
    let (p1, p2) = serial_p(bits, m);
    TestResult {
        p_value2: Some(p2),
        pass: p1 >= ALPHA && p2 >= ALPHA,
        ..result(
            "Serial",
            "2.11",
            p1,
            format!("m = {m}, ∇ψ² and ∇²ψ²"),
            "Are all overlapping m-bit patterns equally common?",
        )
    }
}

fn phi(bits: &[u8], m: usize) -> f64 {
    let n = bits.len() as f64;
    if m == 0 {
        return 0.0;
    }
    pattern_counts(bits, m)
        .iter()
        .filter(|&&x| x > 0)
        .map(|&x| {
            let c = x as f64 / n;
            c * c.ln()
        })
        .sum()
}

pub fn approx_entropy_p(bits: &[u8], m: usize) -> (f64, f64) {
    let n = bits.len() as f64;
    let ap = phi(bits, m) - phi(bits, m + 1);
    let chi = 2.0 * n * (2f64.ln() - ap);
    (ap, igamc((1u64 << (m - 1)) as f64, chi / 2.0))
}

pub fn approx_entropy(bits: &[u8], m: usize) -> TestResult {
    if bits.len() < 100 || m < 1 {
        return not_applicable("Approximate entropy", "2.12", "needs n ≥ 100 bits");
    }
    let (ap, p) = approx_entropy_p(bits, m);
    result(
        "Approximate entropy",
        "2.12",
        p,
        format!("m = {m}, ApEn = {ap:.6} (ideal ln 2 = 0.693147)"),
        "Do m- and (m+1)-bit patterns appear with the frequencies of a fair coin?",
    )
}

/// Run the suite with block sizes chosen from SP 800-22's recommendations.
pub fn suite(bits: &[u8]) -> Vec<TestResult> {
    let n = bits.len();
    let lg = (n.max(2) as f64).log2().floor() as usize;
    // block frequency: M ≥ 20, M > 0.01 n, N < 100
    let m_bf = (n / 99 + 1).max(20).min(n.max(1));
    // serial / approx entropy: m < ⌊log₂ n⌋ − 2 (serial), − 5 (ApEn)
    let m_ser = lg.saturating_sub(3).clamp(3, 16);
    let m_ap = lg.saturating_sub(6).clamp(2, 14);
    vec![
        monobit(bits),
        block_frequency(bits, m_bf),
        runs(bits),
        longest_run(bits),
        serial(bits, m_ser),
        approx_entropy(bits, m_ap),
    ]
}

/// SP 800-90B most-common-value estimate over symbols, bits of min-entropy per symbol.
pub fn mcv_min_entropy(symbols: &[u32]) -> (f64, f64) {
    let l = symbols.len();
    if l < 2 {
        return (0.0, 1.0);
    }
    let mut counts: std::collections::HashMap<u32, usize> = Default::default();
    for &s in symbols {
        *counts.entry(s).or_default() += 1;
    }
    let max = *counts.values().max().unwrap_or(&0) as f64;
    let p = max / l as f64;
    let pu = (p + 2.576 * (p * (1.0 - p) / (l as f64 - 1.0)).sqrt()).min(1.0);
    (-pu.log2(), pu)
}

/// Parse "0101…" (whitespace ignored) or "hex:…" into bits (MSB first per byte).
pub fn parse_bits(s: &str) -> Result<Vec<u8>, String> {
    let t = s.trim();
    if let Some(h) = t.strip_prefix("hex:") {
        let mut out = vec![];
        for ch in h.chars().filter(|c| !c.is_whitespace()) {
            let v = ch.to_digit(16).ok_or_else(|| format!("`{ch}` is not a hex digit"))?;
            for k in (0..4).rev() {
                out.push(((v >> k) & 1) as u8);
            }
        }
        return Ok(out);
    }
    let mut out = vec![];
    for ch in t.chars() {
        match ch {
            '0' => out.push(0),
            '1' => out.push(1),
            c if c.is_whitespace() || c == ',' => {}
            c => return Err(format!("unexpected character `{c}` (expected 0/1, or prefix hex:)")),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(s: &str) -> Vec<u8> {
        parse_bits(s).unwrap()
    }

    /// First 100 bits of the binary expansion of π after "11." — the ε used in SP 800-22's examples.
    fn pi100() -> Vec<u8> {
        let mut v = parse_bits("hex:C90FDAA22168C234C4C6628B80").unwrap();
        v.truncate(100);
        v
    }

    #[test]
    fn special_functions() {
        assert!((ln_gamma(5.0) - 24f64.ln()).abs() < 1e-12);
        assert!((ln_gamma(0.5) - std::f64::consts::PI.sqrt().ln()).abs() < 1e-12);
        assert!((erfc(1.0) - 0.157_299_207_050_285_1).abs() < 1e-12);
        assert!((igamc(1.0, 2.0) - (-2.0f64).exp()).abs() < 1e-14);
    }

    #[test]
    fn sp800_22_worked_examples() {
        // §2.1.4: ε = 1011010101 → P = 0.527089
        assert!((monobit_p(&b("1011010101")) - 0.527089).abs() < 1e-6);
        // §2.1.8: 100 bits of π → P = 0.109599
        assert!((monobit_p(&pi100()) - 0.109599).abs() < 1e-6);
        // §2.2.4: ε = 0110011010, M = 3 → P = 0.801252
        assert!((block_frequency_p(&b("0110011010"), 3) - 0.801252).abs() < 1e-6);
        // §2.2.8: π bits, M = 10 → P = 0.706438
        assert!((block_frequency_p(&pi100(), 10) - 0.706438).abs() < 1e-6);
        // §2.3.4: ε = 1001101011 → P = 0.147232
        assert!((runs_p(&b("1001101011")).unwrap() - 0.147232).abs() < 1e-6);
        // §2.3.8: π bits → P = 0.500798
        assert!((runs_p(&pi100()).unwrap() - 0.500798).abs() < 1e-6);
        // §2.4.8: ν = (4, 9, 3, 0), N = 16 → χ² = 4.882605, P = 0.180598
        let (chi, p) = longest_run_from_counts(&[4, 9, 3, 0], &[0.2148, 0.3672, 0.2305, 0.1875], 16);
        assert!((chi - 4.882605).abs() < 1e-5 && (p - 0.180598).abs() < 1e-5, "{chi} {p}");
        // §2.11.4: ε = 0011011101, m = 3 → P1 = 0.808792, P2 = 0.670320
        let (p1, p2) = serial_p(&b("0011011101"), 3);
        assert!((p1 - 0.808792).abs() < 1e-6 && (p2 - 0.670320).abs() < 1e-6, "{p1} {p2}");
        // §2.12.4: ε = 0100110101, m = 3 → P = 0.261961
        let (_, p) = approx_entropy_p(&b("0100110101"), 3);
        assert!((p - 0.261961).abs() < 1e-6, "{p}");
    }

    #[test]
    fn longest_run_example_sequence() {
        let e = b("11001100000101010110110001001100111000000000001001001101010100010001001111010110100000001101011111001100111001101101100010110010");
        assert_eq!(e.len(), 128);
        let r = longest_run(&e);
        assert!((r.p_value - 0.180598).abs() < 1e-5, "{}", r.statistic);
    }

    #[test]
    fn mcv_of_fair_and_biased_bits() {
        let fair: Vec<u32> = (0..100_000u32).map(|i| (i.wrapping_mul(2654435761) >> 7) & 1).collect();
        let (h, _) = mcv_min_entropy(&fair);
        assert!(h > 0.98);
        let biased: Vec<u32> = (0..10_000).map(|i| u32::from(i % 4 != 0)).collect();
        let (h, _) = mcv_min_entropy(&biased);
        assert!((h - (-(0.75f64 + 2.576 * (0.75f64 * 0.25 / 9999.0).sqrt()).log2())).abs() < 1e-12);
    }
}
