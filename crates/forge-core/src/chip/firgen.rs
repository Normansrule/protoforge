//! FIR filter → synthesizable Verilog for the IRIS-DSP co-processor.
//!
//! Coefficients are quantised to `coeff_bits` (Q0.(B−1) when |h| < 1) and
//! written in **canonical signed digit** form (CSD / non-adjacent form:
//! digits ∈ {−1, 0, +1}, no two adjacent non-zeros, the minimum number of
//! non-zero digits — on average ≈ B/3 instead of B/2 for plain binary), so a
//! constant multiplication becomes a few shifts and adds (G. W. Reitwiesner,
//! "Binary arithmetic", Advances in Computers 1, 1960; K. K. Parhi,
//! *VLSI Digital Signal Processing Systems*, Wiley 1999, ch. 13).
//!
//! Two architectures:
//! * **Transposed form** — one constant multiplier per tap (CSD shift-add
//!   trees on the shared input) and a register/adder chain; one output per
//!   clock. Critical path = deepest shift-add tree + one accumulate adder.
//! * **Decimating MAC** — the IRIS audio path produces one output every
//!   `decim` inputs at 16 kHz while the clock is 50 MHz, so a single
//!   multiplier-accumulator time-shared over the taps (N cycles per output,
//!   a rotating sample buffer and a coefficient ROM) is far smaller. Pipeline:
//!   fetch (ROM + tap) → multiply → accumulate → round/saturate.
//!
//! Cell model (documented assumptions, sky130-like standard cells): 1 cell
//! per full-adder bit (ripple), ≈4 cells per bit for a Brent–Kung
//! carry-lookahead adder, 1 cell per flip-flop bit (enable flops count as
//! one), 1 cell per 2:1-mux bit, an array multiplier ≈ D·B AND cells +
//! D·(B−1) full adders. Delay model: ripple adder ≈ bits × 2 × t_gate,
//! Brent–Kung ≈ (2⌈log₂ bits⌉ + 2) × t_gate, register overhead (clk→Q +
//! setup) ≈ 3 × t_gate. N. Weste & D. Harris, *CMOS VLSI Design*, 4th ed.,
//! ch. 11 (datapath adders and multipliers).

use serde::{Deserialize, Serialize};

use super::fsm::{self, Encoding};

/// Canonical signed digits (non-adjacent form), least significant first.
pub fn csd(v: i64) -> Vec<i8> {
    let mut v = v;
    let mut d = Vec::new();
    while v != 0 {
        if v & 1 != 0 {
            let z: i8 = if v.rem_euclid(4) == 1 { 1 } else { -1 };
            d.push(z);
            v -= z as i64;
        } else {
            d.push(0);
        }
        v >>= 1;
    }
    d
}

/// Value of a CSD digit string (LSB first).
pub fn csd_value(d: &[i8]) -> i64 {
    d.iter().enumerate().map(|(i, &x)| (x as i64) << i).sum()
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Arch {
    Transposed,
    #[default]
    Mac,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AdderKind {
    #[default]
    Ripple,
    Cla,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FirGenOptions {
    pub arch: Arch,
    pub adder: AdderKind,
    /// Transposed form: register the multiplier block outputs.
    pub pipeline: bool,
    pub coeff_bits: u32,
    pub data_bits: u32,
    pub decim: u32,
    pub gate_delay_ns: f64,
    pub clock_mhz: f64,
    pub module_name: String,
    /// Number of input samples in the generated testbench.
    pub test_len: usize,
}

impl Default for FirGenOptions {
    fn default() -> Self {
        FirGenOptions { arch: Arch::Mac, adder: AdderKind::Ripple, pipeline: false, coeff_bits: 12, data_bits: 16, decim: 2, gate_delay_ns: 0.25, clock_mhz: 50.0, module_name: "iris_fir".into(), test_len: 160 }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CoefInfo {
    pub index: usize,
    pub value: f64,
    pub quantized: f64,
    pub code: i64,
    /// CSD digits, most significant first, padded to coeff_bits + 1 digits.
    pub csd: Vec<i8>,
    pub nonzero: u32,
    /// Ones in the plain two's-complement magnitude (for comparison).
    pub binary_nonzero: u32,
    pub adders: u32,
    pub tree_depth: u32,
    /// Shared with an earlier identical coefficient (symmetry).
    pub shared_with: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BlockCost {
    pub name: String,
    pub cells: f64,
    pub ffs: f64,
    pub adder_cells: f64,
    pub other_cells: f64,
    /// Longest register-to-register path inside the block, ns.
    pub critical_ns: f64,
    pub critical_desc: String,
    pub detail: String,
}

fn adder_cells(bits: u32, k: AdderKind) -> f64 {
    match k {
        AdderKind::Ripple => bits as f64,
        AdderKind::Cla => 4.0 * bits as f64,
    }
}

fn adder_delay(bits: u32, k: AdderKind, gd: f64) -> f64 {
    match k {
        AdderKind::Ripple => bits as f64 * 2.0 * gd,
        AdderKind::Cla => (2.0 * (bits.max(2) as f64).log2().ceil() + 2.0) * gd,
    }
}

fn reg_overhead(gd: f64) -> f64 {
    3.0 * gd
}

fn clog2(x: u64) -> u32 {
    if x <= 1 {
        0
    } else {
        64 - (x - 1).leading_zeros()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct FirGen {
    pub arch: Arch,
    pub taps: usize,
    pub coeff_bits: u32,
    pub frac_bits: u32,
    pub data_bits: u32,
    /// Accumulator width W = D + ⌈log₂ Σ|c|⌉ + 1.
    pub acc_bits: u32,
    pub decim: u32,
    pub coefs: Vec<CoefInfo>,
    pub total_csd_digits: u32,
    pub total_binary_digits: u32,
    pub unique_multipliers: usize,
    pub cost: BlockCost,
    pub verilog: String,
    pub testbench: String,
    pub x_test: Vec<i64>,
    pub y_expected: Vec<i64>,
    /// Cycles per output (MAC) or 1 (transposed).
    pub cycles_per_output: u32,
    /// Minimum spacing of `in_valid` pulses in clock cycles.
    pub min_input_spacing: u32,
}

/// Quantise coefficients to Q(int).(frac) with `bits` total.
pub fn quantize(coeffs: &[f64], bits: u32) -> (Vec<i64>, u32) {
    let q = crate::dsp::filter::quantize_coeffs(coeffs, bits);
    (q.codes, q.frac_bits)
}

/// Reference: exact integer convolution, round-half-up by `frac` bits,
/// saturate to `d` bits, keep every `decim`-th output (phase decim−1).
pub fn reference_output(codes: &[i64], x: &[i64], frac: u32, d: u32, decim: u32) -> Vec<i64> {
    let mut out = Vec::new();
    let half = if frac > 0 { 1i64 << (frac - 1) } else { 0 };
    let maxv = (1i64 << (d - 1)) - 1;
    let minv = -(1i64 << (d - 1));
    for n in 0..x.len() {
        if !(n as u32 + 1).is_multiple_of(decim.max(1)) {
            continue;
        }
        let acc: i64 = codes.iter().enumerate().filter(|(k, _)| *k <= n).map(|(k, &c)| c * x[n - k]).sum();
        out.push(((acc + half) >> frac).clamp(minv, maxv));
    }
    out
}

/// Cycle-level model of the transposed-form datapath (CSD shift-add
/// products, register chain) — mirrors the generated Verilog.
pub fn model_transposed(codes: &[i64], x: &[i64], frac: u32, d: u32, decim: u32) -> Vec<i64> {
    let n = codes.len();
    let half = if frac > 0 { 1i64 << (frac - 1) } else { 0 };
    let digits: Vec<Vec<i8>> = codes.iter().map(|&c| csd(c)).collect();
    let mult = |k: usize, xv: i64| -> i64 { digits[k].iter().enumerate().filter(|(_, &dg)| dg != 0).map(|(s, &dg)| (dg as i64) * (xv << s)).sum() };
    // the delay line resets to the rounding constant so every output carries it
    let mut z = vec![half; n.saturating_sub(1)];
    let (maxv, minv) = ((1i64 << (d - 1)) - 1, -(1i64 << (d - 1)));
    let mut out = Vec::new();
    let mut phase = 0u32;
    for &xv in x {
        let m: Vec<i64> = (0..n).map(|k| mult(k, xv)).collect();
        let y_acc = m[0] + if n > 1 { z[0] } else { half };
        // z_k <= m_{k+1} + z_{k+1}; the last register also carries the rounding constant
        for k in 0..n.saturating_sub(1) {
            z[k] = if k + 1 < n - 1 { m[k + 1] + z[k + 1] } else { m[k + 1] + half };
        }
        if phase == decim - 1 {
            out.push((y_acc >> frac).clamp(minv, maxv));
            phase = 0;
        } else {
            phase += 1;
        }
    }
    out
}

/// Cycle-level model of the decimating MAC (rotating buffer, reversed
/// coefficient ROM order) — mirrors the generated Verilog.
pub fn model_mac(codes: &[i64], x: &[i64], frac: u32, d: u32, decim: u32) -> Vec<i64> {
    let n = codes.len();
    let half = if frac > 0 { 1i64 << (frac - 1) } else { 0 };
    let (maxv, minv) = ((1i64 << (d - 1)) - 1, -(1i64 << (d - 1)));
    let mut buf = vec![0i64; n];
    let mut out = Vec::new();
    let mut phase = 0u32;
    for &xv in x {
        // shift in: buf[0] <= x, buf[i] <= buf[i-1]
        buf.rotate_right(1);
        buf[0] = xv;
        if phase == decim - 1 {
            phase = 0;
            let mut acc = half;
            for j in 0..n {
                let tap = buf[n - 1];
                acc += tap * codes[n - 1 - j];
                // rotate: buf[0] <= buf[N-1], buf[i] <= buf[i-1]
                buf.rotate_right(1);
            }
            out.push((acc >> frac).clamp(minv, maxv));
        } else {
            phase += 1;
        }
    }
    out
}

/// Deterministic test input: impulse, full-scale step, tone, then pseudo-random samples.
pub fn test_input(len: usize, d: u32) -> Vec<i64> {
    let maxv = (1i64 << (d - 1)) - 1;
    let mut rng = crate::math::Rng::new(2024);
    (0..len.max(8))
        .map(|i| match i {
            0 => maxv,
            1..=39 => 0,
            40..=79 => -maxv - 1,
            80..=119 => ((i as f64 * 0.7).sin() * 0.9 * maxv as f64) as i64,
            _ => ((rng.uniform() * 2.0 - 1.0) * maxv as f64) as i64,
        })
        .collect()
}

fn vconst(bits: u32, v: i64) -> String {
    if v < 0 {
        format!("-{bits}'sd{}", -v)
    } else {
        format!("{bits}'sd{v}")
    }
}

/// Shift-add Verilog expression for a constant multiplication of `xw`.
fn shift_add_expr(code: i64) -> String {
    let d = csd(code);
    let mut s = String::new();
    for (sh, &dg) in d.iter().enumerate().rev() {
        if dg == 0 {
            continue;
        }
        let term = if sh == 0 { "xw".to_string() } else { format!("(xw <<< {sh})") };
        if s.is_empty() {
            s = if dg > 0 { term } else { format!("-{term}") };
        } else {
            s += &format!(" {} {term}", if dg > 0 { "+" } else { "-" });
        }
    }
    if s.is_empty() {
        "0".into()
    } else {
        s
    }
}

pub fn generate(coeffs: &[f64], o: &FirGenOptions) -> Result<FirGen, String> {
    if coeffs.is_empty() {
        return Err("no coefficients".into());
    }
    if coeffs.len() > 255 {
        return Err("at most 255 taps".into());
    }
    let b = o.coeff_bits.clamp(4, 24);
    let d = o.data_bits.clamp(4, 24);
    let decim = o.decim.clamp(1, 16);
    let gd = o.gate_delay_ns.max(0.01);
    let (codes, frac) = quantize(coeffs, b);
    let n = codes.len();
    let sum_abs: i64 = codes.iter().map(|c| c.abs()).sum::<i64>().max(1);
    let w = d + clog2(sum_abs as u64) + 1;
    // coefficient info with sharing of identical codes
    let mut coefs: Vec<CoefInfo> = Vec::new();
    for (k, &c) in codes.iter().enumerate() {
        let dg = csd(c);
        let nz = dg.iter().filter(|&&x| x != 0).count() as u32;
        let mut msb: Vec<i8> = dg.clone();
        msb.resize((b + 1) as usize, 0);
        msb.reverse();
        let shared = coefs.iter().find(|ci| ci.code.abs() == c.abs() && c != 0).map(|ci| ci.index);
        coefs.push(CoefInfo {
            index: k,
            value: coeffs[k],
            quantized: c as f64 / 2f64.powi(frac as i32),
            code: c,
            csd: msb,
            nonzero: nz,
            binary_nonzero: c.unsigned_abs().count_ones(),
            adders: nz.saturating_sub(1),
            tree_depth: clog2(nz.max(1) as u64),
            shared_with: shared,
        });
    }
    let unique: Vec<&CoefInfo> = coefs.iter().filter(|c| c.shared_with.is_none() && c.code != 0).collect();
    let total_csd: u32 = coefs.iter().map(|c| c.nonzero).sum();
    let total_bin: u32 = coefs.iter().map(|c| c.binary_nonzero).sum();
    let x_test = test_input(o.test_len.clamp(40, 2000), d);
    let y_expected = reference_output(&codes, &x_test, frac, d, decim);
    let name = if o.module_name.is_empty() { "iris_fir".to_string() } else { o.module_name.clone() };
    let (cost, verilog, cycles, spacing) = match o.arch {
        Arch::Transposed => {
            let c = transposed_cost(&coefs, &unique, n, d, w, frac, o, gd);
            (c, transposed_verilog(&name, &codes, frac, d, w, decim, o.pipeline), 1, if o.pipeline { 2 } else { 1 })
        }
        Arch::Mac => {
            let c = mac_cost(n, b, d, w, o, gd);
            (c, mac_verilog(&name, &codes, b, frac, d, w, decim), n as u32 + 4, n as u32 + 6)
        }
    };
    let spacing = spacing.max(1);
    let testbench = fir_testbench(&name, &x_test, &y_expected, d, spacing.max(if o.arch == Arch::Mac { n as u32 + 6 } else { 3 }));
    Ok(FirGen {
        arch: o.arch,
        taps: n,
        coeff_bits: b,
        frac_bits: frac,
        data_bits: d,
        acc_bits: w,
        decim,
        total_csd_digits: total_csd,
        total_binary_digits: total_bin,
        unique_multipliers: unique.len(),
        coefs,
        cost,
        verilog,
        testbench,
        x_test,
        y_expected,
        cycles_per_output: cycles,
        min_input_spacing: spacing,
    })
}

#[allow(clippy::too_many_arguments)]
fn transposed_cost(coefs: &[CoefInfo], unique: &[&CoefInfo], n: usize, d: u32, w: u32, frac: u32, o: &FirGenOptions, gd: f64) -> BlockCost {
    let k = o.adder;
    // multiplier block
    let mut mult_adders = 0.0;
    let mut mult_inv = 0.0;
    let mut deepest = 0u32;
    let mut deepest_w = 0u32;
    for c in unique {
        let dg = csd(c.code);
        let span = dg.len() as u32;
        let width = (d + span + 1).min(w);
        mult_adders += c.adders as f64 * adder_cells(width, k);
        mult_inv += dg.iter().filter(|&&x| x < 0).count() as f64 * width as f64;
        if c.tree_depth > deepest || (c.tree_depth == deepest && width > deepest_w) {
            deepest = c.tree_depth;
            deepest_w = width;
        }
    }
    let neg_coefs = coefs.iter().filter(|c| c.code < 0).count() as f64;
    let chain_adders = n as f64 * adder_cells(w, k); // N−1 chain adders + rounding constant
    let chain_ffs = (n as f64 - 1.0) * w as f64;
    let pipe_ffs = if o.pipeline { unique.len() as f64 * w as f64 } else { 0.0 };
    let ffs = chain_ffs + pipe_ffs + d as f64 /* input reg */ + w as f64 /* y_acc */ + d as f64 /* y_out */ + 4.0;
    let sat = 2.0 * d as f64 + (w - frac - d + 1) as f64;
    let control = 12.0;
    let adders = mult_adders + chain_adders;
    let other = mult_inv + neg_coefs * 0.0 + sat + control;
    let tree = deepest as f64 * adder_delay(deepest_w.max(d), k, gd);
    let acc = adder_delay(w, k, gd);
    let (crit, desc) = if o.pipeline {
        if tree >= acc {
            (tree, format!("CSD shift-add tree, {deepest} adder level(s) × {deepest_w} bits"))
        } else {
            (acc, format!("accumulate adder, {w} bits"))
        }
    } else {
        (tree + acc, format!("CSD shift-add tree ({deepest} level(s) × {deepest_w} bits) + accumulate adder ({w} bits)"))
    };
    BlockCost {
        name: "FIR (transposed form)".into(),
        cells: adders + ffs + other,
        ffs,
        adder_cells: adders,
        other_cells: other,
        critical_ns: crit + reg_overhead(gd),
        critical_desc: desc,
        detail: format!(
            "{} unique constant multipliers ({:.0} adder cells), {} chain adders × {w} b, {:.0} flip-flops{}",
            unique.len(),
            mult_adders,
            n,
            ffs,
            if o.pipeline { " (pipelined)" } else { "" }
        ),
    }
}

fn mac_cost(n: usize, b: u32, d: u32, w: u32, o: &FirGenOptions, gd: f64) -> BlockCost {
    let k = o.adder;
    let buf_ffs = (n as u32 * d) as f64;
    let buf_mux = d as f64;
    let rom = (b as f64 * n as f64 / 4.0).ceil();
    let mult_and = (d * b) as f64;
    let mult_fa = match k {
        AdderKind::Ripple => (d * (b - 1)) as f64,
        AdderKind::Cla => (d * (b - 2)) as f64 + adder_cells(d + 1, k),
    } + (d + b) as f64; // Baugh–Wooley sign handling
    let pipe_ffs = (d + b) as f64 /* tap_r + coef_r */ + (d + b) as f64 /* prod */;
    let acc_add = adder_cells(w, k);
    let acc_ffs = w as f64;
    let acc_mux = w as f64; // reload with the rounding constant
    let out_ffs = d as f64;
    let sat = 2.0 * d as f64;
    let ctrl_bits = clog2(n as u64 + 1);
    let control = 3.0 * ctrl_bits as f64 + 16.0;
    let ffs = buf_ffs + pipe_ffs + acc_ffs + out_ffs + ctrl_bits as f64 + 6.0;
    let adders = mult_fa + acc_add;
    let other = buf_mux + rom + mult_and + acc_mux + sat + control;
    let rom_t = 2.0 * clog2(n as u64).max(1) as f64 * gd;
    let mult_t = (b - 1) as f64 * 2.0 * gd + adder_delay(d + 1, k, gd);
    let acc_t = adder_delay(w, k, gd);
    let sat_t = (2.0 * clog2(w as u64) as f64 + 2.0) * gd;
    let stages = [(rom_t, "coefficient ROM decode".to_string()), (mult_t, format!("{d}×{b} array multiplier")), (acc_t, format!("{w}-bit accumulator")), (sat_t, "round/saturate".to_string())];
    let (crit, desc) = stages.iter().cloned().fold((0.0, String::new()), |a, s| if s.0 > a.0 { s } else { a });
    BlockCost {
        name: "FIR (decimating MAC)".into(),
        cells: adders + ffs + other,
        ffs,
        adder_cells: adders,
        other_cells: other,
        critical_ns: crit + reg_overhead(gd),
        critical_desc: desc,
        detail: format!("one {d}×{b} multiplier, {w}-bit accumulator, {n}×{d}-bit rotating buffer ({buf_ffs:.0} flops), ~{rom:.0}-cell coefficient ROM; {} cycles per output", n + 4),
    }
}

fn sat_function(d: u32, w: u32, frac: u32, src: &str) -> String {
    let maxv = (1i64 << (d - 1)) - 1;
    let minv = -(1i64 << (d - 1));
    format!(
        "    // arithmetic shift by {frac} and saturate to {d} bits (the rounding constant is already in {src})\n    wire signed [{wm}:0] sh = {src} >>> {frac};\n    wire signed [{dm}:0] sat = (sh > {maxc}) ? {maxd} : (sh < {minc}) ? {mind} : sh[{dm}:0];\n\n",
        dm = d - 1,
        wm = w - 1,
        maxc = vconst(w, maxv),
        minc = vconst(w, minv),
        maxd = vconst(d, maxv),
        mind = vconst(d, minv),
    )
}

fn transposed_verilog(name: &str, codes: &[i64], frac: u32, d: u32, w: u32, decim: u32, pipeline: bool) -> String {
    let n = codes.len();
    let half = if frac > 0 { 1i64 << (frac - 1) } else { 0 };
    let pw = clog2(decim as u64).max(1);
    let mut v = format!(
        "// Generated by Protoforge Chip Lab — {n}-tap transposed-form FIR, CSD shift-add multipliers\n// Q0.{frac} coefficients, {d}-bit data, {w}-bit accumulator, decimate by {decim}.\n// y[n] = sat((Σ c_k·x[n−k] + 2^{}) >>> {frac})\n`timescale 1ns/1ps\n`default_nettype none\n\nmodule {name} (\n    input  wire               clk,\n    input  wire               rst_n,\n    input  wire               in_valid,\n    input  wire signed [{dm}:0] x_in,\n    output reg                out_valid,\n    output reg  signed [{dm}:0] y_out\n);\n",
        frac.saturating_sub(1),
        dm = d - 1
    );
    v += &format!("\n    // input register and sign extension to the accumulator width\n    reg signed [{dm}:0] x_r;\n    reg                 v_r;\n    wire signed [{wm}:0] xw = {{{{{ext}{{x_r[{dm}]}}}}, x_r}};\n\n    // constant multipliers (canonical signed digit shift-add)\n", dm = d - 1, wm = w - 1, ext = w - d);
    let mut first_of: Vec<Option<usize>> = vec![None; n];
    for k in 0..n {
        first_of[k] = (0..k).find(|&j| codes[j] == codes[k]);
    }
    for k in 0..n {
        match first_of[k] {
            Some(j) => v += &format!("    wire signed [{}:0] m{k} = m{j};  // c{k} = c{j} = {}\n", w - 1, codes[k]),
            None => v += &format!("    wire signed [{}:0] m{k} = {};  // c{k} = {}\n", w - 1, shift_add_expr(codes[k]), codes[k]),
        }
    }
    let src = |k: usize| if pipeline { format!("mp{k}") } else { format!("m{k}") };
    if pipeline {
        v += &format!("\n    // pipeline registers after the multiplier block\n    reg v_p;\n{}", (0..n).map(|k| format!("    reg signed [{}:0] mp{k};\n", w - 1)).collect::<String>());
    }
    v += &format!("\n    // transposed delay line\n{}", (0..n.saturating_sub(1)).map(|k| format!("    reg signed [{}:0] z{k};\n", w - 1)).collect::<String>());
    v += &format!("    reg signed [{}:0] y_acc;\n    reg               y_acc_v;\n    reg [{}:0] phase;\n", w - 1, pw - 1);
    let step = if pipeline { "v_p" } else { "v_r" };
    v += "\n";
    v += &sat_function(d, w, frac, "y_acc");
    v += "    always @(posedge clk or negedge rst_n) begin\n        if (!rst_n) begin\n";
    v += &format!("            x_r <= {d}'sd0; v_r <= 1'b0; phase <= {pw}'d0;\n            y_acc <= {w}'sd0; y_acc_v <= 1'b0; out_valid <= 1'b0; y_out <= {d}'sd0;\n");
    if pipeline {
        v += "            v_p <= 1'b0;\n";
        for k in 0..n {
            v += &format!("            mp{k} <= {w}'sd0;\n");
        }
    }
    for k in 0..n.saturating_sub(1) {
        v += &format!("            z{k} <= {};\n", vconst(w, half));
    }
    v += "        end else begin\n            v_r <= in_valid;\n            if (in_valid) x_r <= x_in;\n";
    if pipeline {
        v += "            v_p <= v_r;\n            if (v_r) begin\n";
        for k in 0..n {
            v += &format!("                mp{k} <= m{k};\n");
        }
        v += "            end\n";
    }
    v += &format!("            y_acc_v <= 1'b0;\n            if ({step}) begin\n");
    for k in 0..n.saturating_sub(1) {
        if k + 1 < n - 1 {
            v += &format!("                z{k} <= {} + z{};\n", src(k + 1), k + 1);
        } else {
            v += &format!("                z{k} <= {} + {};  // rounding constant rides the chain\n", src(k + 1), vconst(w, half));
        }
    }
    let y_expr = if n > 1 { format!("{} + z0", src(0)) } else { format!("{} + {}", src(0), vconst(w, half)) };
    v += &format!(
        "                if (phase == {pw}'d{}) begin\n                    phase <= {pw}'d0;\n                    y_acc <= {y_expr};\n                    y_acc_v <= 1'b1;\n                end else begin\n                    phase <= phase + {pw}'d1;\n                end\n            end\n            out_valid <= y_acc_v;\n            if (y_acc_v) y_out <= sat;\n        end\n    end\nendmodule\n\n`default_nettype wire\n",
        decim - 1
    );
    v
}

fn mac_verilog(name: &str, codes: &[i64], b: u32, frac: u32, d: u32, w: u32, decim: u32) -> String {
    let n = codes.len();
    let half = if frac > 0 { 1i64 << (frac - 1) } else { 0 };
    let jw = clog2(n as u64).max(1);
    let pw = clog2(decim as u64).max(1);
    let pb = d + b;
    let mut v = format!(
        "// Generated by Protoforge Chip Lab — {n}-tap decimating MAC FIR (one multiplier, time-shared)\n// Q0.{frac} coefficients, {d}-bit data, {w}-bit accumulator, one output per {decim} inputs.\n// Pipeline: fetch (ROM + tap) → multiply → accumulate → round/saturate; {n} + 4 cycles per output.\n// in_valid pulses must be at least {} clock cycles apart.\n`timescale 1ns/1ps\n`default_nettype none\n\nmodule {name} (\n    input  wire               clk,\n    input  wire               rst_n,\n    input  wire               in_valid,\n    input  wire signed [{dm}:0] x_in,\n    output reg                out_valid,\n    output reg  signed [{dm}:0] y_out\n);\n",
        n + 6,
        dm = d - 1
    );
    v += &format!("\n    localparam integer N = {n};\n    reg signed [{dm}:0] xbuf [0:N-1];  // xbuf[i] = x[n-i]; rotates while busy\n    reg [{jm}:0] j;          // MAC index\n    reg          busy;\n    reg [{pm}:0] phase;\n\n    // coefficient ROM, read in reverse order (the tap at xbuf[N-1] is x[n-(N-1-j)])\n    reg signed [{bm}:0] rom;\n    always @* begin\n        case (j)\n", dm = d - 1, jm = jw - 1, pm = pw - 1, bm = b - 1);
    for jj in 0..n {
        v += &format!("            {jw}'d{jj}: rom = {};\n", vconst(b, codes[n - 1 - jj]));
    }
    v += &format!("            default: rom = {b}'sd0;\n        endcase\n    end\n\n");
    v += &format!(
        "    reg signed [{dm}:0] tap_r;\n    reg signed [{bm}:0] coef_r;\n    reg signed [{pm2}:0] prod;\n    reg signed [{wm}:0] acc;\n    reg v1, v2, l1, l2, done;\n    integer i;\n\n",
        dm = d - 1,
        bm = b - 1,
        pm2 = pb - 1,
        wm = w - 1
    );
    v += &sat_function(d, w, frac, "acc");
    v += &format!(
        "    always @(posedge clk or negedge rst_n) begin\n        if (!rst_n) begin\n            for (i = 0; i < N; i = i + 1) xbuf[i] <= {d}'sd0;\n            j <= {jw}'d0; busy <= 1'b0; phase <= {pw}'d0;\n            tap_r <= {d}'sd0; coef_r <= {b}'sd0; prod <= {pb}'sd0; acc <= {half_c};\n            v1 <= 1'b0; v2 <= 1'b0; l1 <= 1'b0; l2 <= 1'b0; done <= 1'b0;\n            out_valid <= 1'b0; y_out <= {d}'sd0;\n        end else begin\n            // sample buffer: shift a new sample in, or rotate one place per MAC cycle\n            if (in_valid || busy) begin\n                xbuf[0] <= in_valid ? x_in : xbuf[N-1];\n                for (i = 1; i < N; i = i + 1) xbuf[i] <= xbuf[i-1];\n            end\n            if (in_valid) begin\n                if (phase == {pw}'d{dl}) begin\n                    phase <= {pw}'d0;\n                    busy <= 1'b1;\n                    j <= {jw}'d0;\n                end else begin\n                    phase <= phase + {pw}'d1;\n                end\n            end\n            // stage 1: fetch tap and coefficient\n            v1 <= busy;\n            l1 <= busy && (j == {jw}'d{nl});\n            if (busy) begin\n                tap_r <= xbuf[N-1];\n                coef_r <= rom;\n                if (j == {jw}'d{nl}) busy <= 1'b0;\n                else j <= j + {jw}'d1;\n            end\n            // stage 2: multiply\n            v2 <= v1;\n            l2 <= l1;\n            if (v1) prod <= tap_r * coef_r;\n            // stage 3: accumulate (acc starts at the rounding constant)\n            done <= v2 && l2;\n            if (v2) acc <= acc + prod;\n            // stage 4: round/saturate and restart the accumulator\n            out_valid <= done;\n            if (done) begin\n                y_out <= sat;\n                acc <= {half_c};\n            end\n        end\n    end\nendmodule\n\n`default_nettype wire\n",
        half_c = vconst(w, half),
        dl = decim - 1,
        nl = n - 1
    );
    v
}

fn fir_testbench(name: &str, x: &[i64], y: &[i64], d: u32, spacing: u32) -> String {
    let mask = (1u64 << d) - 1;
    let xs: Vec<u32> = x.iter().map(|&v| (v as u64 & mask) as u32).collect();
    let ys: Vec<u32> = y.iter().map(|&v| (v as u64 & mask) as u32).collect();
    let (xw, xh) = fsm::pack_hex(&xs, d);
    let (yw, yh) = fsm::pack_hex(&ys, d);
    format!(
        "// Self-checking testbench generated by Protoforge Chip Lab.\n// Expected outputs are the exact integer convolution computed in Rust\n// (chip::firgen::reference_output).\n// Run:  iverilog -g2005 -o tb_{name} {name}.v tb_{name}.v && vvp tb_{name}\n`timescale 1ns/1ps\n\nmodule tb_{name};\n    reg clk = 1'b0, rst_n = 1'b0, in_valid = 1'b0;\n    reg  signed [{dm}:0] x_in = {d}'sd0;\n    wire signed [{dm}:0] y_out;\n    wire out_valid;\n\n    {name} dut (.clk(clk), .rst_n(rst_n), .in_valid(in_valid), .x_in(x_in), .out_valid(out_valid), .y_out(y_out));\n\n    always #10 clk = ~clk;   // 50 MHz\n\n    localparam integer NX = {nx};\n    localparam integer NY = {ny};\n    localparam [{xwm}:0] XS = {xw}'h{xh};\n    localparam [{ywm}:0] YS = {yw}'h{yh};\n\n    integer i, k = 0, errors = 0;\n\n    // compare every output against the reference\n    always @(posedge clk) begin\n        if (out_valid) begin\n            if (k < NY && y_out !== $signed(YS[{d}*k +: {d}])) begin\n                errors = errors + 1;\n                if (errors < 10) $display(\"output %0d: got %0d, expected %0d\", k, y_out, $signed(YS[{d}*k +: {d}]));\n            end\n            k = k + 1;\n        end\n    end\n\n    initial begin\n        $dumpfile(\"tb_{name}.vcd\");\n        $dumpvars(0, tb_{name});\n        repeat (3) @(posedge clk);\n        #1 rst_n = 1'b1;\n        for (i = 0; i < NX; i = i + 1) begin\n            @(negedge clk);\n            x_in = XS[{d}*i +: {d}];\n            in_valid = 1'b1;\n            @(negedge clk);\n            in_valid = 1'b0;\n            repeat ({gap}) @(negedge clk);\n        end\n        repeat ({drain}) @(negedge clk);\n        if (k != NY) begin\n            $display(\"FAIL: %0d outputs, expected %0d\", k, NY);\n            errors = errors + 1;\n        end\n        if (errors == 0) $display(\"PASS: %0d outputs match the reference convolution\", NY);\n        else $display(\"FAIL: %0d errors\", errors);\n        $finish;\n    end\nendmodule\n",
        dm = d - 1,
        nx = x.len(),
        ny = y.len(),
        xwm = xw - 1,
        ywm = yw - 1,
        gap = spacing.saturating_sub(2),
        drain = spacing + 60,
    )
}

// ------------------------------------------------------------------ CIC + top level

#[derive(Clone, Debug, Serialize)]
pub struct CicSpec {
    pub order: u32,
    pub rate: u32,
    pub diff_delay: u32,
    pub width: u32,
}

pub fn cic_cost(c: &CicSpec, k: AdderKind, gd: f64) -> BlockCost {
    let n = c.order as f64;
    let w = c.width;
    let adders = 2.0 * n * adder_cells(w, k);
    let ffs = n * w as f64 /* integrators */ + n * w as f64 * (1.0 + c.diff_delay as f64) /* comb out + delay */ + w as f64 /* sampled */ + clog2(c.rate as u64) as f64 + n + 2.0;
    let other = n * w as f64 /* subtract inverters */ + 3.0 * clog2(c.rate as u64) as f64 + 6.0;
    BlockCost {
        name: "CIC decimator".into(),
        cells: adders + ffs + other,
        ffs,
        adder_cells: adders,
        other_cells: other,
        critical_ns: adder_delay(w, k, gd) + reg_overhead(gd),
        critical_desc: format!("{w}-bit integrator adder"),
        detail: format!("N = {}, R = {}, M = {}: {} integrators + {} pipelined combs, {w}-bit registers (Hogenauer)", c.order, c.rate, c.diff_delay, c.order, c.order),
    }
}

pub fn cic_verilog(c: &CicSpec) -> String {
    let n = c.order;
    let w = c.width;
    let rw = clog2(c.rate as u64).max(1);
    let mut v = format!(
        "// Generated by Protoforge Chip Lab — CIC decimator (Hogenauer 1981)\n// N = {n} stages, R = {r}, M = 1, {w}-bit two's-complement registers (wrap-around is harmless).\n// Integrators run on every PDM bit (in_valid); combs are pipelined, one per clock.\n`timescale 1ns/1ps\n`default_nettype none\n\nmodule iris_cic (\n    input  wire              clk,\n    input  wire              rst_n,\n    input  wire              in_valid,   // one PDM bit\n    input  wire              pdm,        // 1 → +1, 0 → −1\n    output wire              out_valid,\n    output wire signed [{wm}:0] y\n);\n    localparam integer N = {n};\n    reg signed [{wm}:0] integ [0:N-1];\n    reg signed [{wm}:0] cs    [0:N];     // comb pipeline\n    reg signed [{wm}:0] cd    [0:N-1];   // comb delays\n    reg [N:0]           cv;\n    reg [{rm}:0]        cnt;\n    integer i;\n    wire signed [{wm}:0] xin = pdm ? {one} : {mone};\n\n    always @(posedge clk or negedge rst_n) begin\n        if (!rst_n) begin\n            for (i = 0; i < N; i = i + 1) begin integ[i] <= {w}'sd0; cd[i] <= {w}'sd0; end\n            for (i = 0; i <= N; i = i + 1) cs[i] <= {w}'sd0;\n            cv <= {{(N+1){{1'b0}}}};\n            cnt <= {rw}'d0;\n        end else begin\n            if (in_valid) begin\n                integ[0] <= integ[0] + xin;\n                for (i = 1; i < N; i = i + 1) integ[i] <= integ[i] + integ[i-1];\n                cnt <= (cnt == {rw}'d{rl}) ? {rw}'d0 : cnt + {rw}'d1;\n            end\n            cv[0] <= in_valid && (cnt == {rw}'d{rl});\n            if (in_valid && cnt == {rw}'d{rl}) cs[0] <= integ[N-1];\n            for (i = 0; i < N; i = i + 1) begin\n                cv[i+1] <= cv[i];\n                if (cv[i]) begin\n                    cs[i+1] <= cs[i] - cd[i];\n                    cd[i]   <= cs[i];\n                end\n            end\n        end\n    end\n    assign out_valid = cv[N];\n    assign y = cs[N];\nendmodule\n\n`default_nettype wire\n",
        r = c.rate,
        wm = w - 1,
        rm = rw - 1,
        rl = c.rate - 1,
        one = vconst(w, 1),
        mone = vconst(w, -1),
    );
    v.shrink_to_fit();
    v
}

/// Bit-accurate model of the generated CIC (pipelined integrators) — used to
/// check the DC gain in tests.
pub fn model_cic(bits: &[bool], c: &CicSpec) -> Vec<i64> {
    let n = c.order as usize;
    let mut integ = vec![0i64; n];
    let mut cd = vec![0i64; n];
    let mut out = Vec::new();
    let mut cnt = 0u32;
    for &b in bits {
        let x = if b { 1 } else { -1 };
        let prev = integ.clone();
        integ[0] = prev[0] + x;
        for i in 1..n {
            integ[i] = prev[i] + prev[i - 1];
        }
        if cnt == c.rate - 1 {
            // cs[0] samples integ[N-1] *before* this update (non-blocking semantics)
            let mut s = prev[n - 1];
            for d in cd.iter_mut() {
                let o = s - *d;
                *d = s;
                s = o;
            }
            out.push(s);
            cnt = 0;
        } else {
            cnt += 1;
        }
    }
    out
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TopOptions {
    pub fir: FirGenOptions,
    pub encoding: Encoding,
    pub tap_ms: u32,
    pub long_ms: u32,
    pub gap_ms: u32,
}

impl Default for TopOptions {
    fn default() -> Self {
        TopOptions { fir: FirGenOptions::default(), encoding: Encoding::Binary, tap_ms: 250, long_ms: 600, gap_ms: 250 }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DspChipCost {
    pub blocks: Vec<BlockCost>,
    pub total_cells: f64,
    /// Same design if synthesis maps each adder bit to ~3 cells (XOR/XOR/MAJ)
    /// instead of one full-adder cell — a pessimistic bound.
    pub total_cells_pessimistic: f64,
    pub critical_ns: f64,
    pub critical_block: String,
    pub critical_desc: String,
    pub clock_period_ns: f64,
    pub slack_ns: f64,
    pub cell_budget: f64,
    pub timing_ok: bool,
    pub area_ok: bool,
    pub suggestions: Vec<String>,
}

pub fn glue_cost(clock_mhz: f64, gd: f64) -> BlockCost {
    let div_bits = clog2((clock_mhz * 1000.0).max(2.0) as u64);
    let ffs = 6.0 /* synchronisers */ + 2.0 /* edge detect */ + div_bits as f64 + 16.0 /* held sample */ + 1.0;
    let other = 2.0 * div_bits as f64 /* incrementer */ + div_bits as f64 /* compare */ + 8.0 /* byte mux */ + 6.0;
    BlockCost {
        name: "Tiny Tapeout glue".into(),
        cells: ffs + other,
        ffs,
        adder_cells: 0.0,
        other_cells: other,
        critical_ns: adder_delay(div_bits, AdderKind::Ripple, gd) + reg_overhead(gd),
        critical_desc: format!("{div_bits}-bit 1 kHz tick divider"),
        detail: "input synchronisers, PDM clock edge detector, 1 kHz tick divider, output byte mux".into(),
    }
}

pub fn fsm_block(f: &fsm::Fsm, enc: Encoding, gd: f64) -> BlockCost {
    let c = fsm::cost(f, enc, gd);
    BlockCost {
        name: "Gesture FSM".into(),
        cells: c.total_cells as f64,
        ffs: (c.state_ffs + c.timer_ffs) as f64,
        adder_cells: c.timer_incr_cells as f64,
        other_cells: (c.comparator_cells + c.next_state_cells + c.output_cells) as f64,
        critical_ns: c.critical_ns,
        critical_desc: "timer comparator → next-state logic".into(),
        detail: format!("{} states ({:?}), {}-bit timer, {} comparators", f.states.len(), enc, c.timer_ffs, c.comparator_cells / c.timer_ffs.max(1)),
    }
}

pub fn summarize(blocks: Vec<BlockCost>, clock_mhz: f64, budget: f64, arch: Arch) -> DspChipCost {
    let total: f64 = blocks.iter().map(|b| b.cells).sum();
    let crit = blocks.iter().fold(None::<&BlockCost>, |a, b| match a {
        Some(x) if x.critical_ns >= b.critical_ns => Some(x),
        _ => Some(b),
    });
    let (cns, cblock, cdesc) = crit.map(|b| (b.critical_ns, b.name.clone(), b.critical_desc.clone())).unwrap_or((0.0, String::new(), String::new()));
    let period = 1000.0 / clock_mhz.max(1e-3);
    let mut sug = Vec::new();
    if cns > period {
        if arch == Arch::Transposed {
            sug.push("Switch to the decimating MAC architecture: the audio output rate is 16 kHz, so one time-shared multiplier has thousands of cycles per output.".into());
            sug.push("Or pipeline the multiplier block and use carry-lookahead adders (costs area).".into());
        } else {
            sug.push("Use carry-lookahead adders in the multiplier and accumulator, or add a pipeline stage inside the multiplier.".into());
        }
        sug.push(format!("Or lower the clock to {:.0} MHz.", 1000.0 / cns));
    }
    if total > budget {
        if arch == Arch::Transposed {
            sug.push("The transposed form needs one constant multiplier per tap; the decimating MAC shares one multiplier across all taps.".into());
        }
        sug.push("Reduce coefficient/data word lengths, or exploit coefficient symmetry (pre-add x[n−k] + x[n−N+1+k] to halve the MACs).".into());
    }
    DspChipCost {
        total_cells: total,
        total_cells_pessimistic: total + 2.0 * blocks.iter().map(|b| b.adder_cells).sum::<f64>(),
        critical_ns: cns,
        critical_block: cblock,
        critical_desc: cdesc,
        clock_period_ns: period,
        slack_ns: period - cns,
        cell_budget: budget,
        timing_ok: cns <= period,
        area_ok: total <= budget,
        suggestions: sug,
        blocks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csd_digit_counts_and_values() {
        for v in -2000i64..2000 {
            let d = csd(v);
            assert_eq!(csd_value(&d), v);
            // non-adjacent form
            for w in d.windows(2) {
                assert!(w[0] == 0 || w[1] == 0, "{v}: {d:?}");
            }
        }
        // 7 = 8 − 1 → 2 non-zeros (binary has 3)
        assert_eq!(csd(7).iter().filter(|&&x| x != 0).count(), 2);
        // 0b0101_0101 = 85 is already non-adjacent: 4 non-zeros
        assert_eq!(csd(85).iter().filter(|&&x| x != 0).count(), 4);
        // 0b0111_0111 = 119 = 128 − 8 − 1 → 3 non-zeros (binary 6)
        assert_eq!(csd(119).iter().filter(|&&x| x != 0).count(), 3);
    }

    fn iris_fir() -> Vec<f64> {
        let p = crate::project::Project::iris();
        crate::dsp::pdm::design_chain(&p.design.audio, &Default::default()).fir
    }

    #[test]
    fn generated_architectures_match_reference_convolution() {
        let h = iris_fir();
        let (codes, frac) = quantize(&h, 12);
        let x = test_input(400, 16);
        let r = reference_output(&codes, &x, frac, 16, 2);
        assert_eq!(model_transposed(&codes, &x, frac, 16, 2), r);
        assert_eq!(model_mac(&codes, &x, frac, 16, 2), r);
        // impulse response appears in the (decimated) output
        assert_eq!(r.len(), 200);
        let g = generate(&h, &FirGenOptions::default()).unwrap();
        assert_eq!(g.y_expected, reference_output(&codes, &g.x_test, frac, 16, 2));
        assert!(g.verilog.contains("module iris_fir"));
        assert!(g.testbench.contains("PASS"));
    }

    #[test]
    fn iris_cost_model_numbers() {
        let h = iris_fir();
        let mac = generate(&h, &FirGenOptions::default()).unwrap();
        let tr = generate(&h, &FirGenOptions { arch: Arch::Transposed, ..Default::default() }).unwrap();
        assert!(mac.cost.cells < tr.cost.cells);
        assert!(mac.cost.critical_ns < 20.0, "{:?}", mac.cost);
        assert!(tr.cost.critical_ns > 20.0, "ripple transposed form should miss 20 ns: {:?}", tr.cost);
        assert!(mac.total_csd_digits < mac.total_binary_digits);
        let cla = generate(&h, &FirGenOptions { arch: Arch::Transposed, adder: AdderKind::Cla, pipeline: true, ..Default::default() }).unwrap();
        assert!(cla.cost.critical_ns < 20.0);
        assert!(cla.cost.cells > tr.cost.cells);
    }

    #[test]
    fn cic_model_dc_gain() {
        let c = CicSpec { order: 4, rate: 32, diff_delay: 1, width: 22 };
        let y = model_cic(&vec![true; 32 * 20], &c);
        assert_eq!(*y.last().unwrap(), 32i64.pow(4));
        let v = cic_verilog(&c);
        assert!(v.contains("module iris_cic"));
    }
}
