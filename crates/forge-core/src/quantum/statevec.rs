//! State-vector simulator (Schrödinger picture), up to [`MAX_QUBITS`] qubits.
//!
//! The state is `2ⁿ` complex amplitudes; a single-qubit gate `U` on target
//! `t` with control mask `m` updates every pair `(i, i | 1<<t)` with bit `t`
//! of `i` clear and `i & m == m`:
//!
//! ```text
//! a'ᵢ = U₀₀ aᵢ + U₀₁ aⱼ      a'ⱼ = U₁₀ aᵢ + U₁₁ aⱼ      (j = i | 1<<t)
//! ```
//!
//! Measurement follows the Born rule: `P(1) = Σ |aᵢ|²` over indices with
//! bit `t` set; the state collapses onto the observed half and is
//! renormalised. Randomness is seeded so runs are reproducible.
//! Reference: Nielsen & Chuang, *Quantum Computation and Quantum
//! Information* (2010), ch. 4 (circuits) and 2.2.3 (measurement).

use serde::Serialize;

use super::circuit::{bitstring, matrix, Circuit, Op, M2};
use super::linalg;
use crate::math::{Rng, C64};

pub const MAX_QUBITS: usize = 14;

#[derive(Clone, Debug)]
pub struct StateVec {
    pub n: usize,
    pub amps: Vec<C64>,
    pub clbits: Vec<u8>,
    rng: Rng,
}

/// What a measurement did (for the step-through narrative).
#[derive(Clone, Debug, Serialize)]
pub struct MeasureEvent {
    pub qubit: usize,
    pub clbit: Option<usize>,
    pub outcome: u8,
    pub probability: f64,
}

pub fn apply_1q(amps: &mut [C64], t: usize, mask: usize, m: &M2) {
    let bit = 1usize << t;
    for i in 0..amps.len() {
        if i & bit != 0 || i & mask != mask {
            continue;
        }
        let j = i | bit;
        let (a, b) = (amps[i], amps[j]);
        amps[i] = m[0][0] * a + m[0][1] * b;
        amps[j] = m[1][0] * a + m[1][1] * b;
    }
}

/// SWAP (factor 1) or iSWAP (factor i) on qubits `a`, `b`.
pub fn apply_swap(amps: &mut [C64], a: usize, b: usize, mask: usize, factor: C64) {
    let (ba, bb) = (1usize << a, 1usize << b);
    for i in 0..amps.len() {
        if i & ba == 0 || i & bb != 0 || i & mask != mask {
            continue;
        }
        let j = (i ^ ba) | bb;
        let (x, y) = (amps[i], amps[j]);
        amps[i] = y * factor;
        amps[j] = x * factor;
    }
}

pub fn mask_of(qs: &[usize]) -> usize {
    qs.iter().fold(0, |m, &q| m | (1 << q))
}

impl StateVec {
    pub fn new(n: usize, clbits: usize, seed: u64) -> StateVec {
        let mut amps = vec![C64::ZERO; 1 << n];
        amps[0] = C64::ONE;
        StateVec {
            n,
            amps,
            clbits: vec![0; clbits],
            rng: Rng::new(seed),
        }
    }

    pub fn from_amps(amps: Vec<C64>, seed: u64) -> StateVec {
        let n = amps.len().trailing_zeros() as usize;
        StateVec {
            n,
            amps,
            clbits: vec![],
            rng: Rng::new(seed),
        }
    }

    fn cond_ok(&self, op: &Op) -> bool {
        match &op.cond {
            None => true,
            Some(c) => {
                let v = c.bits.iter().enumerate().fold(0u64, |acc, (k, &b)| acc | ((self.clbits.get(b).copied().unwrap_or(0) as u64) << k));
                v == c.value
            }
        }
    }

    /// Apply one (normalised) op.
    pub fn apply(&mut self, op: &Op) -> Option<MeasureEvent> {
        if !self.cond_ok(op) {
            return None;
        }
        let mask = mask_of(&op.controls);
        match op.gate.as_str() {
            "barrier" | "id" => None,
            "swap" => {
                apply_swap(&mut self.amps, op.targets[0], op.targets[1], mask, C64::ONE);
                None
            }
            "iswap" => {
                // iSWAP: |01⟩ → i|10⟩, |10⟩ → i|01⟩, |00⟩ and |11⟩ unchanged
                apply_swap(&mut self.amps, op.targets[0], op.targets[1], mask, C64::J);
                None
            }
            "measure" => Some(self.measure(op.targets[0], op.clbit)),
            "reset" => {
                let ev = self.measure(op.targets[0], None);
                if ev.outcome == 1 {
                    apply_1q(&mut self.amps, op.targets[0], 0, &matrix("x", &[]).unwrap());
                }
                None
            }
            g => {
                if let Some(m) = matrix(g, &op.params) {
                    apply_1q(&mut self.amps, op.targets[0], mask, &m);
                }
                None
            }
        }
    }

    pub fn prob_one(&self, q: usize) -> f64 {
        let bit = 1 << q;
        self.amps.iter().enumerate().filter(|(i, _)| i & bit != 0).map(|(_, a)| a.norm_sqr()).sum()
    }

    pub fn measure(&mut self, q: usize, clbit: Option<usize>) -> MeasureEvent {
        let p1 = self.prob_one(q).clamp(0.0, 1.0);
        let outcome = if self.rng.uniform() < p1 { 1u8 } else { 0u8 };
        let p = if outcome == 1 { p1 } else { 1.0 - p1 };
        let norm = if p > 0.0 { 1.0 / p.sqrt() } else { 0.0 };
        let bit = 1 << q;
        for (i, a) in self.amps.iter_mut().enumerate() {
            let b = u8::from(i & bit != 0);
            *a = if b == outcome { a.scale(norm) } else { C64::ZERO };
        }
        if let Some(c) = clbit {
            if c < self.clbits.len() {
                self.clbits[c] = outcome;
            }
        }
        MeasureEvent {
            qubit: q,
            clbit,
            outcome,
            probability: p,
        }
    }

    pub fn probs(&self) -> Vec<f64> {
        self.amps.iter().map(|a| a.norm_sqr()).collect()
    }

    /// Reduced density matrix of one qubit: [[ρ00, ρ01], [ρ10, ρ11]].
    pub fn reduced_1(&self, q: usize) -> [C64; 4] {
        let bit = 1 << q;
        let (mut r00, mut r11, mut r01) = (0.0, 0.0, C64::ZERO);
        for i in 0..self.amps.len() {
            if i & bit == 0 {
                let (a, b) = (self.amps[i], self.amps[i | bit]);
                r00 += a.norm_sqr();
                r11 += b.norm_sqr();
                r01 = r01 + a * b.conj();
            }
        }
        [C64::real(r00), r01, r01.conj(), C64::real(r11)]
    }

    /// Two-qubit reduced density matrix of (qa, qb), basis |b a⟩ (qa is the low bit).
    pub fn reduced_2(&self, qa: usize, qb: usize) -> Vec<C64> {
        let (ba, bb) = (1usize << qa, 1usize << qb);
        let mut rho = vec![C64::ZERO; 16];
        for i in 0..self.amps.len() {
            if i & ba != 0 || i & bb != 0 {
                continue;
            }
            let idx = [i, i | ba, i | bb, i | ba | bb];
            for r in 0..4 {
                let ar = self.amps[idx[r]];
                if ar.norm_sqr() == 0.0 {
                    continue;
                }
                for cc in 0..4 {
                    rho[r * 4 + cc] = rho[r * 4 + cc] + ar * self.amps[idx[cc]].conj();
                }
            }
        }
        rho
    }

    pub fn bloch(&self, q: usize) -> [f64; 3] {
        bloch_of(&self.reduced_1(q))
    }
}

/// Bloch vector of a 2×2 density matrix: ρ = (I + xX + yY + zZ)/2.
pub fn bloch_of(r: &[C64; 4]) -> [f64; 3] {
    [2.0 * r[1].re, -2.0 * r[1].im, r[0].re - r[3].re]
}

/// Per-qubit readout for the UI.
#[derive(Clone, Debug, Serialize)]
pub struct QubitInfo {
    pub bloch: [f64; 3],
    /// |r|: 1 for a pure single-qubit state, < 1 when entangled or noisy.
    pub radius: f64,
    pub p1: f64,
    /// Purity Tr ρ² = (1 + |r|²)/2.
    pub purity: f64,
    /// von Neumann entropy of the qubit, bits (entanglement entropy for a pure global state).
    pub entropy: f64,
}

pub fn qubit_info(b: [f64; 3]) -> QubitInfo {
    let r = (b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt().min(1.0);
    QubitInfo {
        bloch: b,
        radius: r,
        p1: (1.0 - b[2]) / 2.0,
        purity: (1.0 + r * r) / 2.0,
        entropy: linalg::h2((1.0 + r) / 2.0),
    }
}

/// Snapshot of the state after a column.
#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub column: usize,
    /// Basis indices shown (all for n ≤ 8, else the 64 most probable).
    pub basis: Vec<usize>,
    pub labels: Vec<String>,
    pub re: Vec<f64>,
    pub im: Vec<f64>,
    pub prob: Vec<f64>,
    pub qubits: Vec<QubitInfo>,
    /// Pairwise concurrence matrix (n ≤ 6), row-major n×n.
    pub concurrence: Option<Vec<f64>>,
    pub clbits: Vec<u8>,
    pub events: Vec<MeasureEvent>,
}

pub fn snapshot(s: &StateVec, column: usize, events: Vec<MeasureEvent>, pairs: bool) -> Snapshot {
    let dim = s.amps.len();
    let basis: Vec<usize> = if s.n <= 8 {
        (0..dim).collect()
    } else {
        let mut idx: Vec<usize> = (0..dim).collect();
        idx.sort_by(|&a, &b| s.amps[b].norm_sqr().partial_cmp(&s.amps[a].norm_sqr()).unwrap_or(std::cmp::Ordering::Equal));
        let mut top: Vec<usize> = idx.into_iter().take(64).collect();
        top.sort();
        top
    };
    let conc = if pairs && s.n >= 2 && s.n <= 6 {
        let n = s.n;
        let mut m = vec![0.0; n * n];
        for a in 0..n {
            for b in (a + 1)..n {
                let cval = linalg::concurrence(&s.reduced_2(a, b));
                m[a * n + b] = cval;
                m[b * n + a] = cval;
            }
        }
        Some(m)
    } else {
        None
    };
    Snapshot {
        column,
        labels: basis.iter().map(|&i| bitstring(i, s.n)).collect(),
        re: basis.iter().map(|&i| clean(s.amps[i].re)).collect(),
        im: basis.iter().map(|&i| clean(s.amps[i].im)).collect(),
        prob: basis.iter().map(|&i| clean(s.amps[i].norm_sqr())).collect(),
        basis,
        qubits: (0..s.n).map(|q| qubit_info(s.bloch(q))).collect(),
        concurrence: conc,
        clbits: s.clbits.clone(),
        events,
    }
}

fn clean(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        0.0
    } else {
        x
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct HistEntry {
    pub bits: String,
    pub count: usize,
    pub prob: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunOut {
    pub qubits: usize,
    pub clbits: usize,
    pub columns: usize,
    /// State after the last column of the first (seeded) shot.
    pub state: Snapshot,
    /// State after every column (index 0 = initial |0…0⟩), when `steps` was requested.
    pub steps: Vec<Snapshot>,
    /// Shots histogram over classical bits (or over qubits when nothing is measured).
    pub histogram: Vec<HistEntry>,
    pub histogram_over: String,
    pub shots: usize,
    /// Exact outcome probabilities for the histogram keys when they can be computed.
    pub exact: Vec<HistEntry>,
    pub sampling: String,
    pub warnings: Vec<String>,
}

pub fn run(circ: &Circuit, shots: usize, seed: u64, steps: bool) -> Result<RunOut, String> {
    let c = circ.validated()?;
    if c.qubits > MAX_QUBITS {
        return Err(format!("{} qubits is above the state-vector limit of {MAX_QUBITS} (2^{MAX_QUBITS} amplitudes)", c.qubits));
    }
    let mut warnings = vec![];
    let pairs = c.qubits <= 6;
    // First shot, with step snapshots.
    let mut sv = StateVec::new(c.qubits, c.clbits, seed);
    let mut snaps = vec![];
    if steps {
        snaps.push(snapshot(&sv, 0, vec![], pairs));
    }
    for (ci, col) in c.cols.iter().enumerate() {
        let mut ev = vec![];
        for op in col {
            if let Some(e) = sv.apply(op) {
                ev.push(e);
            }
        }
        if steps {
            snaps.push(snapshot(&sv, ci + 1, ev, pairs));
        }
    }
    let state = snapshot(&sv, c.cols.len(), vec![], pairs);

    let measured: Vec<bool> = {
        let mut m = vec![false; c.clbits];
        for op in c.ops() {
            if let (true, Some(b)) = (op.gate == "measure", op.clbit) {
                m[b] = true;
            }
        }
        m
    };
    let over_clbits = measured.iter().any(|&x| x);
    let shots = shots.clamp(1, 100_000);
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    let mut exact: Vec<HistEntry> = vec![];
    let sampling;
    let mut used_shots = shots;
    if c.is_terminal_measurement_only() {
        // One run of the unitary part, then sample outcomes from |a|².
        let mut ideal = StateVec::new(c.qubits, c.clbits, seed);
        let mut meas: Vec<(usize, usize)> = vec![];
        for op in c.ops() {
            if op.gate == "measure" {
                if let Some(b) = op.clbit {
                    meas.push((op.targets[0], b));
                }
            } else {
                ideal.apply(op);
            }
        }
        let probs = ideal.probs();
        let key_of = |i: usize| -> String {
            if over_clbits {
                let mut bits = vec![0u8; c.clbits];
                for &(q, b) in &meas {
                    bits[b] = ((i >> q) & 1) as u8;
                }
                bits.iter().rev().map(|b| if *b == 1 { '1' } else { '0' }).collect()
            } else {
                bitstring(i, c.qubits)
            }
        };
        let mut ex: std::collections::BTreeMap<String, f64> = Default::default();
        for (i, p) in probs.iter().enumerate() {
            if *p > 1e-12 {
                *ex.entry(key_of(i)).or_default() += p;
            }
        }
        exact = ex.iter().map(|(k, &p)| HistEntry { bits: k.clone(), count: 0, prob: p }).collect();
        // cumulative sampling
        let mut cdf = Vec::with_capacity(probs.len());
        let mut acc = 0.0;
        for p in &probs {
            acc += p;
            cdf.push(acc);
        }
        let mut rng = Rng::new(seed ^ 0xA5A5_5A5A);
        for _ in 0..shots {
            let u = rng.uniform() * acc;
            let i = cdf.partition_point(|&x| x < u).min(probs.len() - 1);
            *counts.entry(key_of(i)).or_default() += 1;
        }
        sampling = "sampled from the final state (all measurements are terminal)".to_string();
    } else {
        // Mid-circuit measurement / reset / classical control: re-simulate each shot.
        let ops: Vec<&Op> = c.ops().collect();
        let cost = ops.len().max(1) * (1usize << c.qubits);
        let budget = 40_000_000usize;
        used_shots = shots.min((budget / cost).max(16));
        if used_shots < shots {
            warnings.push(format!("mid-circuit measurement: re-simulated {used_shots} of {shots} shots to keep the page responsive"));
        }
        for s in 0..used_shots {
            let mut st = StateVec::new(c.qubits, c.clbits, seed.wrapping_add(1 + s as u64 * 7919));
            for op in &ops {
                st.apply(op);
            }
            let key = if over_clbits {
                st.clbits.iter().rev().map(|b| if *b == 1 { '1' } else { '0' }).collect()
            } else {
                // sample the qubits too
                let probs = st.probs();
                let u = st.rng.uniform();
                let mut acc = 0.0;
                let mut pick = 0;
                for (i, p) in probs.iter().enumerate() {
                    acc += p;
                    if u < acc {
                        pick = i;
                        break;
                    }
                }
                bitstring(pick, c.qubits)
            };
            *counts.entry(key).or_default() += 1;
        }
        sampling = "re-simulated shot by shot (mid-circuit measurement, reset or classical control)".to_string();
    }
    let histogram = counts
        .iter()
        .map(|(k, &n)| HistEntry {
            bits: k.clone(),
            count: n,
            prob: n as f64 / used_shots as f64,
        })
        .collect();
    Ok(RunOut {
        qubits: c.qubits,
        clbits: c.clbits,
        columns: c.cols.len(),
        state,
        steps: snaps,
        histogram,
        histogram_over: if over_clbits { "classical bits c[m-1]…c[0]".into() } else { "qubits q[n-1]…q[0]".into() },
        shots: used_shots,
        exact,
        sampling,
        warnings,
    })
}

/// Final state vector of the unitary part (no measurement), for tests and fidelity.
pub fn final_state(c: &Circuit, seed: u64) -> Result<StateVec, String> {
    let c = c.validated()?;
    let mut sv = StateVec::new(c.qubits, c.clbits, seed);
    for op in c.ops() {
        sv.apply(op);
    }
    Ok(sv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantum::circuit::Op;

    fn close(a: C64, b: C64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn hxh_equals_z() {
        // Apply H X H and Z to a generic state and compare.
        let psi = vec![C64::new(0.6, 0.0), C64::new(0.0, 0.8)];
        let mut a = StateVec::from_amps(psi.clone(), 1);
        for g in ["h", "x", "h"] {
            a.apply(&Op::new(g, &[0]));
        }
        let mut b = StateVec::from_amps(psi, 1);
        b.apply(&Op::new("z", &[0]));
        assert!(close(a.amps[0], b.amps[0]) && close(a.amps[1], b.amps[1]));
        // S·S = Z, T·T = S, √X·√X = X
        let mut s = StateVec::from_amps(vec![C64::new(0.6, 0.0), C64::new(0.8, 0.0)], 1);
        s.apply(&Op::new("sx", &[0]));
        s.apply(&Op::new("sx", &[0]));
        assert!(close(s.amps[0], C64::new(0.8, 0.0)) && close(s.amps[1], C64::new(0.6, 0.0)));
    }

    #[test]
    fn cnot_truth_table() {
        // control q0, target q1: |c t⟩ → |c, t ⊕ c⟩
        for input in 0..4usize {
            let mut sv = StateVec::new(2, 0, 1);
            sv.amps = vec![C64::ZERO; 4];
            sv.amps[input] = C64::ONE;
            sv.apply(&Op::ctrl("x", &[0], &[1]));
            let (c, t) = (input & 1, (input >> 1) & 1);
            let expect = c | ((t ^ c) << 1);
            assert!(close(sv.amps[expect], C64::ONE), "input {input}");
        }
    }

    #[test]
    fn toffoli_and_fredkin() {
        for input in 0..8usize {
            let mut sv = StateVec::new(3, 0, 1);
            sv.amps = vec![C64::ZERO; 8];
            sv.amps[input] = C64::ONE;
            sv.apply(&Op::ctrl("x", &[0, 1], &[2]));
            let expect = if input & 3 == 3 { input ^ 4 } else { input };
            assert!(close(sv.amps[expect], C64::ONE));
            let mut f = StateVec::new(3, 0, 1);
            f.amps = vec![C64::ZERO; 8];
            f.amps[input] = C64::ONE;
            f.apply(&Op::ctrl("swap", &[0], &[1, 2]));
            let b1 = (input >> 1) & 1;
            let b2 = (input >> 2) & 1;
            let expect = if input & 1 == 1 { 1 | (b2 << 1) | (b1 << 2) } else { input };
            assert!(close(f.amps[expect], C64::ONE));
        }
    }

    #[test]
    fn bell_state_probabilities_and_entanglement() {
        let c = Circuit::from_ops(2, 2, vec![Op::new("h", &[0]), Op::ctrl("x", &[0], &[1]), Op::measure(0, 0), Op::measure(1, 1)]);
        let out = run(&c, 4000, 3, true).unwrap();
        let p = &out.steps[2].prob;
        assert!((p[0] - 0.5).abs() < 1e-12 && (p[3] - 0.5).abs() < 1e-12 && p[1] == 0.0 && p[2] == 0.0);
        let q = &out.steps[2].qubits;
        assert!(q[0].radius < 1e-9 && (q[0].entropy - 1.0).abs() < 1e-9);
        let conc = out.steps[2].concurrence.as_ref().unwrap();
        assert!((conc[1] - 1.0).abs() < 1e-9);
        // only 00 and 11 appear, roughly half each
        assert!(out.histogram.iter().all(|h| h.bits == "00" || h.bits == "11"));
        let n00 = out.histogram.iter().find(|h| h.bits == "00").unwrap().count as f64;
        assert!((n00 / 4000.0 - 0.5).abs() < 0.04);
        assert_eq!(out.exact.len(), 2);
    }

    #[test]
    fn iswap_matrix() {
        // |01⟩ (q0=1) → i|10⟩
        let mut sv = StateVec::new(2, 0, 1);
        sv.amps = vec![C64::ZERO, C64::ONE, C64::ZERO, C64::ZERO];
        sv.apply(&Op::new("iswap", &[0, 1]));
        assert!(close(sv.amps[2], C64::J));
    }

    #[test]
    fn u3_matches_named_gates() {
        use crate::math::PI;
        // U(π/2, 0, π) = H ; U(0,0,λ) = P(λ)
        let h = matrix("u3", &[PI / 2.0, 0.0, PI]).unwrap();
        let hh = matrix("h", &[]).unwrap();
        for i in 0..2 {
            for j in 0..2 {
                assert!(close(h[i][j], hh[i][j]));
            }
        }
    }

    #[test]
    fn measurement_collapses_and_mid_circuit_reset() {
        let c = Circuit::from_ops(1, 1, vec![Op::new("h", &[0]), Op::measure(0, 0), Op::new("reset", &[0])]);
        let out = run(&c, 500, 11, true).unwrap();
        // after the measurement the qubit is a basis state
        let r = out.steps[2].qubits[0].radius;
        assert!((r - 1.0).abs() < 1e-9);
        // after reset it is |0⟩
        assert!((out.state.qubits[0].bloch[2] - 1.0).abs() < 1e-9);
        let ones = out.histogram.iter().find(|h| h.bits == "1").map(|h| h.count).unwrap_or(0) as f64;
        assert!((ones / out.shots as f64 - 0.5).abs() < 0.08);
    }
}
