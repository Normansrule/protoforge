//! Algorithm and protocol library. Every entry returns a circuit, a short
//! explanation and the expected result, so the page can show "what should
//! happen" next to what the simulator does.
//!
//! References: Bennett et al., "Teleporting an unknown quantum state via
//! dual classical and Einstein–Podolsky–Rosen channels", PRL 70, 1895
//! (1993); Bennett & Wiesner, superdense coding, PRL 69, 2881 (1992);
//! Żukowski, Zeilinger, Horne & Ekert, entanglement swapping, PRL 71, 4287
//! (1993); Deutsch & Jozsa, Proc. R. Soc. A 439 (1992); Bernstein &
//! Vazirani, SIAM J. Comput. 26 (1997); Grover, STOC 1996; Clauser, Horne,
//! Shimony & Holt, PRL 23, 880 (1969); Nielsen & Chuang (2010) ch. 1, 5, 6.

use serde::{Deserialize, Serialize};

use super::circuit::{adjoint, Circuit, Op};
use super::statevec::{StateVec, MAX_QUBITS};
use crate::math::{Rng, PI};

#[derive(Clone, Debug, Serialize)]
pub struct Expected {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub group: String,
    pub summary: String,
    pub explanation: String,
    pub circuit: Circuit,
    pub expected: Vec<Expected>,
    /// Extra series for animation (e.g. Grover success per iteration).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Listing {
    pub id: &'static str,
    pub title: &'static str,
    pub group: &'static str,
    pub summary: &'static str,
}

pub fn list() -> Vec<Listing> {
    let l = |id, title, group, summary| Listing { id, title, group, summary };
    vec![
        l("bell", "Bell state |Φ+⟩", "Entanglement", "H then CNOT: two qubits that always agree when measured."),
        l("ghz", "GHZ state", "Entanglement", "Three or more qubits in (|0…0⟩ + |1…1⟩)/√2."),
        l("teleport", "Teleportation (measure + classical control)", "Protocols", "Send a qubit state with one Bell pair and two classical bits."),
        l("teleport_deferred", "Teleportation (deferred measurement)", "Protocols", "Same protocol with controlled gates instead of if-statements."),
        l("superdense", "Superdense coding", "Protocols", "Send two classical bits by moving one qubit of a Bell pair."),
        l("swap_ent", "Entanglement swapping", "Protocols", "Entangle two qubits that never met: the core of a quantum repeater."),
        l("chsh", "CHSH Bell test (one setting)", "Protocols", "Measure a Bell pair along two angles; S over four settings reaches 2√2."),
        l("deutsch_jozsa", "Deutsch–Jozsa", "Algorithms", "Constant or balanced? One oracle call instead of 2^(n−1)+1."),
        l("bernstein_vazirani", "Bernstein–Vazirani", "Algorithms", "Read a secret bit string s from f(x) = s·x in one query."),
        l("grover", "Grover search", "Algorithms", "Amplitude amplification finds a marked item in ~(π/4)√N steps."),
        l("qft", "Quantum Fourier transform", "Algorithms", "The DFT on 2ⁿ amplitudes with O(n²) gates."),
        l("qpe", "Phase estimation", "Algorithms", "Read the eigenphase of a unitary into a register with the inverse QFT."),
    ]
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Params {
    /// Qubit count (GHZ, Grover, QFT, DJ/BV inputs, QPE counting qubits).
    pub n: Option<usize>,
    /// Marked item (Grover), secret (BV), oracle mask (DJ), input (QFT).
    pub value: Option<u64>,
    /// DJ oracle kind: "constant0", "constant1", "balanced".
    pub kind: Option<String>,
    /// Teleported state / eigenphase: θ, φ (radians) or phase fraction.
    pub theta: Option<f64>,
    pub phi: Option<f64>,
    /// Two classical bits for superdense coding.
    pub message: Option<u64>,
    /// Grover iterations (default: optimal).
    pub iterations: Option<usize>,
    /// CHSH angles in radians.
    pub a: Option<f64>,
    pub b: Option<f64>,
    /// Append measurements.
    pub measure: Option<bool>,
}

fn e(label: &str, value: impl Into<String>) -> Expected {
    Expected {
        label: label.into(),
        value: value.into(),
    }
}

fn h_all(ops: &mut Vec<Op>, qs: impl IntoIterator<Item = usize>) {
    for q in qs {
        ops.push(Op::new("h", &[q]));
    }
}

/// QFT ops on qubits `qs[0..m]` (qs[0] = least significant) so that
/// QFT|x⟩ = 2^{-m/2} Σ_y e^{2πi x y / 2^m} |y⟩, including the final swaps.
pub fn qft_ops(qs: &[usize]) -> Vec<Op> {
    let m = qs.len();
    let mut ops = vec![];
    for j in (0..m).rev() {
        ops.push(Op::new("h", &[qs[j]]));
        for k in (0..j).rev() {
            ops.push(Op::ctrl("p", &[qs[k]], &[qs[j]]).param(&[PI / (1u64 << (j - k)) as f64]));
        }
    }
    for i in 0..m / 2 {
        ops.push(Op::new("swap", &[qs[i], qs[m - 1 - i]]));
    }
    ops
}

pub fn inverse(ops: &[Op]) -> Vec<Op> {
    ops.iter().rev().map(|o| adjoint(o).expect("unitary op")).collect()
}

/// Multi-controlled Z on all of `qs` (symmetric).
fn mcz(qs: &[usize]) -> Op {
    let (last, ctrls) = qs.split_last().unwrap();
    Op::ctrl("z", ctrls, &[*last])
}

pub fn grover_ops(n: usize, marked: u64, iters: usize) -> Vec<Op> {
    let qs: Vec<usize> = (0..n).collect();
    let mut ops = vec![];
    h_all(&mut ops, 0..n);
    for _ in 0..iters {
        ops.push(Op::new("barrier", &qs));
        // oracle: phase-flip |marked⟩
        for q in 0..n {
            if (marked >> q) & 1 == 0 {
                ops.push(Op::new("x", &[q]).labelled("oracle"));
            }
        }
        ops.push(mcz(&qs).labelled("oracle"));
        for q in 0..n {
            if (marked >> q) & 1 == 0 {
                ops.push(Op::new("x", &[q]).labelled("oracle"));
            }
        }
        // diffusion 2|s⟩⟨s| − I
        for q in 0..n {
            ops.push(Op::new("h", &[q]).labelled("diffuser"));
        }
        for q in 0..n {
            ops.push(Op::new("x", &[q]).labelled("diffuser"));
        }
        ops.push(mcz(&qs).labelled("diffuser"));
        for q in 0..n {
            ops.push(Op::new("x", &[q]).labelled("diffuser"));
        }
        for q in 0..n {
            ops.push(Op::new("h", &[q]).labelled("diffuser"));
        }
    }
    ops
}

/// Optimal Grover iteration count ⌊π / (4θ)⌋ with sin θ = 1/√N.
pub fn grover_optimal(n: usize) -> usize {
    let th = (1.0 / ((1u64 << n) as f64).sqrt()).asin();
    (PI / (4.0 * th)).floor() as usize
}

fn with_measure(n: usize, clbits: usize, mut ops: Vec<Op>, which: &[usize]) -> Circuit {
    for (k, &q) in which.iter().enumerate() {
        ops.push(Op::measure(q, k));
    }
    Circuit::from_ops(n, clbits, ops)
}

pub fn build(id: &str, p: &Params) -> Result<Entry, String> {
    let meas = p.measure.unwrap_or(true);
    let listing = list().into_iter().find(|l| l.id == id).ok_or_else(|| format!("unknown algorithm `{id}`"))?;
    let mut entry = Entry {
        id: id.into(),
        title: listing.title.into(),
        group: listing.group.into(),
        summary: listing.summary.into(),
        explanation: String::new(),
        circuit: Circuit::new(1, 0),
        expected: vec![],
        extra: None,
    };
    match id {
        "bell" => {
            let ops = vec![Op::new("h", &[0]), Op::ctrl("x", &[0], &[1])];
            entry.circuit = if meas { with_measure(2, 2, ops, &[0, 1]) } else { Circuit::from_ops(2, 0, ops) };
            entry.explanation = "H puts q0 in (|0⟩+|1⟩)/√2; the CNOT copies the basis value onto q1, giving (|00⟩+|11⟩)/√2. Each qubit alone is maximally mixed (its Bloch vector shrinks to the centre) but the two outcomes always agree.".into();
            entry.expected = vec![e("P(00)", "0.5"), e("P(11)", "0.5"), e("Concurrence", "1"), e("Entropy of each qubit", "1 bit")];
        }
        "ghz" => {
            let n = p.n.unwrap_or(3).clamp(2, MAX_QUBITS);
            let mut ops = vec![Op::new("h", &[0])];
            for q in 1..n {
                ops.push(Op::ctrl("x", &[q - 1], &[q]));
            }
            let all: Vec<usize> = (0..n).collect();
            entry.circuit = if meas { with_measure(n, n, ops, &all) } else { Circuit::from_ops(n, 0, ops) };
            entry.explanation = format!("A chain of CNOTs spreads q0's superposition to all {n} qubits: (|{}⟩ + |{}⟩)/√2. Losing any one qubit destroys the coherence of the rest — GHZ states are fragile, which the Noise tab shows.", "0".repeat(n), "1".repeat(n));
            entry.expected = vec![e(&format!("P({})", "0".repeat(n)), "0.5"), e(&format!("P({})", "1".repeat(n)), "0.5")];
        }
        "teleport" | "teleport_deferred" => {
            let th = p.theta.unwrap_or(1.1);
            let ph = p.phi.unwrap_or(0.7);
            let mut ops = vec![
                Op::new("u3", &[0]).param(&[th, ph, 0.0]).labelled("prepare |ψ⟩"),
                Op::new("h", &[1]).labelled("Bell pair"),
                Op::ctrl("x", &[1], &[2]).labelled("Bell pair"),
                Op::new("barrier", &[0, 1, 2]),
                Op::ctrl("x", &[0], &[1]).labelled("Bell measurement"),
                Op::new("h", &[0]).labelled("Bell measurement"),
            ];
            if id == "teleport" {
                ops.push(Op::measure(0, 0));
                ops.push(Op::measure(1, 1));
                ops.push(Op::new("x", &[2]).when(&[1], 1).labelled("correction"));
                ops.push(Op::new("z", &[2]).when(&[0], 1).labelled("correction"));
                entry.circuit = Circuit::from_ops(3, 2, ops).with_cregs(&[("m0", 1), ("m1", 1)]);
                entry.explanation = "Alice holds |ψ⟩ (q0) and half of a Bell pair (q1); Bob holds q2. Alice does a Bell-basis measurement (CNOT, H, measure) and sends the two bits. Bob applies X if m1 = 1 and Z if m0 = 1, and q2 is now exactly |ψ⟩ — whatever the outcomes. Nothing travels faster than light: without the bits Bob's qubit is maximally mixed.".into();
            } else {
                ops.push(Op::ctrl("x", &[1], &[2]).labelled("correction"));
                ops.push(Op::ctrl("z", &[0], &[2]).labelled("correction"));
                if meas {
                    ops.push(Op::measure(0, 0));
                    ops.push(Op::measure(1, 1));
                }
                entry.circuit = Circuit::from_ops(3, if meas { 2 } else { 0 }, ops);
                entry.explanation = "The principle of deferred measurement (Nielsen & Chuang §4.4): a classically controlled gate after a measurement equals a quantum-controlled gate before it. This form runs on the density-matrix simulator and on hardware without mid-circuit measurement.".into();
            }
            let b = [th.sin() * ph.cos(), th.sin() * ph.sin(), th.cos()];
            entry.expected = vec![
                e("Bob's Bloch vector", format!("({:.3}, {:.3}, {:.3})", b[0], b[1], b[2])),
                e("Fidelity with |ψ⟩", "1 for every measurement outcome"),
                e("Each outcome m1 m0", "probability 1/4"),
            ];
        }
        "superdense" => {
            let m = p.message.unwrap_or(2) & 3;
            let mut ops = vec![Op::new("h", &[0]).labelled("Bell pair"), Op::ctrl("x", &[0], &[1]).labelled("Bell pair"), Op::new("barrier", &[0, 1])];
            if m & 2 == 2 {
                ops.push(Op::new("x", &[0]).labelled("encode"));
            }
            if m & 1 == 1 {
                ops.push(Op::new("z", &[0]).labelled("encode"));
            }
            ops.push(Op::new("barrier", &[0, 1]));
            ops.push(Op::ctrl("x", &[0], &[1]).labelled("decode"));
            ops.push(Op::new("h", &[0]).labelled("decode"));
            entry.circuit = with_measure(2, 2, ops, &[0, 1]);
            entry.explanation = format!("Alice and Bob share |Φ+⟩. To send bits c1 c0 = {:02b} Alice applies X if c1 = 1 and Z if c0 = 1 to her qubit — one of four orthogonal Bell states — and sends it. Bob's CNOT + H map the Bell basis back to 00, 01, 10, 11. Two bits per qubit sent (Holevo's bound says one qubit alone carries one).", m);
            entry.expected = vec![e("Measured c1 c0", format!("{m:02b} with probability 1"))];
        }
        "swap_ent" => {
            let ops = vec![
                Op::new("h", &[0]).labelled("pair A–C"),
                Op::ctrl("x", &[0], &[1]).labelled("pair A–C"),
                Op::new("h", &[2]).labelled("pair C–B"),
                Op::ctrl("x", &[2], &[3]).labelled("pair C–B"),
                Op::new("barrier", &[0, 1, 2, 3]),
                Op::ctrl("x", &[1], &[2]).labelled("Charlie's Bell measurement"),
                Op::new("h", &[1]).labelled("Charlie's Bell measurement"),
                Op::measure(1, 0),
                Op::measure(2, 1),
                Op::new("x", &[3]).when(&[1], 1).labelled("Bob's correction"),
                Op::new("z", &[3]).when(&[0], 1).labelled("Bob's correction"),
            ];
            entry.circuit = Circuit::from_ops(4, 2, ops).with_cregs(&[("m0", 1), ("m1", 1)]);
            entry.explanation = "Alice–Charlie share a Bell pair (q0, q1) and Charlie–Bob another (q2, q3). Charlie's Bell measurement on q1, q2 teleports q1's entanglement onto q3: after Bob's Pauli correction, q0 and q3 are in |Φ+⟩ although they never interacted. Chaining swaps over short fiber links is how a quantum repeater beats the loss of one long link.".into();
            entry.expected = vec![e("Concurrence(q0, q3)", "1 after correction"), e("q0 q3 outcomes", "always equal")];
        }
        "chsh" => {
            let a = p.a.unwrap_or(0.0);
            let b = p.b.unwrap_or(PI / 4.0);
            let ops = vec![
                Op::new("h", &[0]),
                Op::ctrl("x", &[0], &[1]),
                Op::new("ry", &[0]).param(&[-a]).labelled("Alice's angle"),
                Op::new("ry", &[1]).param(&[-b]).labelled("Bob's angle"),
            ];
            entry.circuit = with_measure(2, 2, ops, &[0, 1]);
            entry.explanation = "Rotating by −θ before a Z measurement measures along the direction θ in the X–Z plane. For |Φ+⟩ the correlation is E(a, b) = cos(a − b); the CHSH combination of four settings reaches 2√2 ≈ 2.83, above the bound of 2 for any local hidden-variable model.".into();
            entry.expected = vec![e("E(a,b) = P(same) − P(different)", format!("cos(a − b) = {:.4}", (a - b).cos()))];
        }
        "deutsch_jozsa" | "bernstein_vazirani" => {
            let n = p.n.unwrap_or(3).clamp(1, MAX_QUBITS - 1);
            let kind = if id == "bernstein_vazirani" { "balanced".to_string() } else { p.kind.clone().unwrap_or_else(|| "balanced".into()) };
            let s = p.value.unwrap_or(if id == "bernstein_vazirani" { 0b101 } else { 0b011 }) & ((1u64 << n) - 1);
            let anc = n;
            let mut ops = vec![Op::new("x", &[anc])];
            h_all(&mut ops, 0..=n);
            ops.push(Op::new("barrier", &(0..=n).collect::<Vec<_>>()));
            match kind.as_str() {
                "constant0" => {}
                "constant1" => ops.push(Op::new("x", &[anc]).labelled("oracle")),
                _ => {
                    for q in 0..n {
                        if (s >> q) & 1 == 1 {
                            ops.push(Op::ctrl("x", &[q], &[anc]).labelled("oracle"));
                        }
                    }
                }
            }
            ops.push(Op::new("barrier", &(0..=n).collect::<Vec<_>>()));
            h_all(&mut ops, 0..n);
            let inputs: Vec<usize> = (0..n).collect();
            entry.circuit = if meas { with_measure(n + 1, n, ops, &inputs) } else { Circuit::from_ops(n + 1, 0, ops) };
            let bits = |v: u64| -> String { (0..n).rev().map(|k| if (v >> k) & 1 == 1 { '1' } else { '0' }).collect() };
            if id == "bernstein_vazirani" {
                entry.explanation = format!("The oracle computes f(x) = s·x mod 2 with a CNOT from every qubit where s has a 1. Phase kickback from the |−⟩ ancilla writes (−1)^(s·x) into the amplitudes and the final Hadamards turn that phase pattern into |s⟩. Classically you need {n} queries; here one.");
                entry.expected = vec![e("Measured register", format!("s = {} with probability 1", bits(s)))];
            } else {
                let constant = kind != "balanced" || s == 0;
                entry.explanation = "Phase kickback turns the oracle into (−1)^f(x). After the second layer of Hadamards the amplitude of |0…0⟩ is the average of (−1)^f(x): ±1 for a constant f, 0 for a balanced one. One query decides with certainty; a deterministic classical algorithm needs 2^(n−1) + 1.".into();
                entry.expected = vec![e(
                    "Outcome",
                    if constant { format!("{} (constant)", "0".repeat(n)) } else { format!("never {} (balanced) — here {}", "0".repeat(n), bits(s)) },
                )];
            }
        }
        "grover" => {
            let n = p.n.unwrap_or(3).clamp(2, 8);
            let marked = p.value.unwrap_or(5) & ((1u64 << n) - 1);
            let kopt = grover_optimal(n);
            let k = p.iterations.unwrap_or(kopt).min(4 * kopt + 4);
            let ops = grover_ops(n, marked, k);
            let all: Vec<usize> = (0..n).collect();
            entry.circuit = if meas { with_measure(n, n, ops, &all) } else { Circuit::from_ops(n, 0, ops) };
            let big_n = (1u64 << n) as f64;
            let th = (1.0 / big_n.sqrt()).asin();
            // simulate each iteration count to get the full distribution and the theory curve
            let kmax = (2 * kopt + 2).max(k);
            let mut sim_curve = vec![];
            let mut theory = vec![];
            let mut dists = vec![];
            let mut sv = StateVec::new(n, 0, 1);
            for q in 0..n {
                sv.apply(&Op::new("h", &[q]));
            }
            for it in 0..=kmax {
                if it > 0 {
                    for op in grover_ops(n, marked, 1).iter().skip(n) {
                        sv.apply(op);
                    }
                }
                let pr = sv.probs();
                sim_curve.push(pr[marked as usize]);
                theory.push(((2 * it + 1) as f64 * th).sin().powi(2));
                dists.push(pr);
            }
            entry.explanation = format!(
                "The oracle flips the sign of the marked item |{m}⟩; the diffuser reflects every amplitude about the mean. Each iteration rotates the state by 2θ towards |{m}⟩ with sin θ = 1/√N (θ = {:.2}°). After k iterations P = sin²((2k+1)θ); the best k is ⌊π/(4θ)⌋ = {kopt} for N = {}. Going further overshoots and P falls again.",
                th.to_degrees(),
                big_n as u64,
                m = super::circuit::bitstring(marked as usize, n)
            );
            entry.expected = vec![
                e("Optimal iterations", kopt.to_string()),
                e(&format!("P(marked) after {k}"), format!("{:.4}", ((2 * k + 1) as f64 * th).sin().powi(2))),
                e("Classical average queries", format!("{:.1}", (big_n + 1.0) / 2.0)),
            ];
            entry.extra = Some(serde_json::json!({
                "iterations": (0..=kmax).collect::<Vec<_>>(),
                "p_marked_sim": sim_curve,
                "p_marked_theory": theory,
                "distributions": dists,
                "theta": th,
                "optimal": kopt,
                "marked": marked,
            }));
        }
        "qft" => {
            let n = p.n.unwrap_or(3).clamp(1, 8);
            let x = p.value.unwrap_or(5) & ((1u64 << n) - 1);
            let mut ops = vec![];
            for q in 0..n {
                if (x >> q) & 1 == 1 {
                    ops.push(Op::new("x", &[q]).labelled("input"));
                }
            }
            ops.push(Op::new("barrier", &(0..n).collect::<Vec<_>>()));
            let qs: Vec<usize> = (0..n).collect();
            ops.extend(qft_ops(&qs));
            entry.circuit = Circuit::from_ops(n, 0, ops);
            entry.explanation = format!("QFT|x⟩ = 2^(−n/2) Σ_y e^(2πi·x·y/2^n)|y⟩. Every output amplitude has magnitude 1/√{} and the phase winds x = {x} times around the colour wheel as y goes from 0 to {}. The circuit uses n Hadamards and n(n−1)/2 controlled-phase gates, then swaps to fix the bit order.", 1u64 << n, (1u64 << n) - 1);
            entry.expected = vec![e("Every |amplitude|", format!("1/√{} = {:.4}", 1u64 << n, 1.0 / ((1u64 << n) as f64).sqrt())), e("Phase of |y⟩", format!("2π·{x}·y/{}", 1u64 << n))];
        }
        "qpe" => {
            let t = p.n.unwrap_or(4).clamp(1, MAX_QUBITS - 1);
            let phase = p.theta.unwrap_or(0.3125).rem_euclid(1.0);
            let target = t;
            let mut ops = vec![Op::new("x", &[target]).labelled("eigenstate |1⟩")];
            h_all(&mut ops, 0..t);
            for k in 0..t {
                let ang = 2.0 * PI * phase * (1u64 << k) as f64;
                ops.push(Op::ctrl("p", &[k], &[target]).param(&[ang.rem_euclid(2.0 * PI)]).labelled(&format!("U^{}", 1u64 << k)));
            }
            let qs: Vec<usize> = (0..t).collect();
            ops.extend(inverse(&qft_ops(&qs)).into_iter().map(|o| o.labelled("QFT†")));
            entry.circuit = if meas { with_measure(t + 1, t, ops, &qs) } else { Circuit::from_ops(t + 1, 0, ops) };
            let best = (phase * (1u64 << t) as f64).round() as u64 % (1u64 << t);
            // exact distribution: |(1/2^t) Σ_x e^{2πi x (φ − y/2^t)}|²
            let nn = (1u64 << t) as f64;
            let pbest = {
                let d = phase - best as f64 / nn;
                if d.abs() < 1e-12 {
                    1.0
                } else {
                    ((PI * nn * d).sin() / (nn * (PI * d).sin())).powi(2)
                }
            };
            entry.explanation = format!("U = P(2πφ) has eigenvector |1⟩ with eigenvalue e^(2πiφ). Controlled-U^(2^k) kicks the phase 2πφ·2^k back onto counting qubit k, building the Fourier state of φ·2^t; the inverse QFT turns it into the integer closest to φ·2^t = {:.3}. With t = {t} counting qubits the phase is read to 1/2^t.", phase * nn);
            entry.expected = vec![
                e("Most likely readout", format!("{} = {best} → φ ≈ {:.5}", super::circuit::bitstring(best as usize, t), best as f64 / nn)),
                e("Its probability", format!("{pbest:.4}")),
            ];
        }
        _ => return Err(format!("unknown algorithm `{id}`")),
    }
    Ok(entry)
}

// ---------------------------------------------------------------------------
// CHSH Bell test
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ChshIn {
    /// Alice's two angles and Bob's two angles (radians, X–Z plane).
    pub a0: f64,
    pub a1: f64,
    pub b0: f64,
    pub b1: f64,
    /// Werner-state visibility V (1 = perfect Bell pair).
    pub visibility: f64,
    pub shots: usize,
    pub seed: u64,
}

impl Default for ChshIn {
    fn default() -> Self {
        ChshIn {
            a0: 0.0,
            a1: PI / 2.0,
            b0: PI / 4.0,
            b1: 3.0 * PI / 4.0,
            visibility: 1.0,
            shots: 2000,
            seed: 1,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ChshSetting {
    pub a: f64,
    pub b: f64,
    pub e_exact: f64,
    pub e_sampled: f64,
    /// Counts of (same, different) outcomes.
    pub same: usize,
    pub diff: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct ChshOut {
    pub settings: Vec<ChshSetting>,
    pub s_exact: f64,
    pub s_sampled: f64,
    pub s_std: f64,
    pub classical_bound: f64,
    pub tsirelson: f64,
    pub violated: bool,
    /// Visibility needed to violate (1/√2).
    pub v_threshold: f64,
    /// S vs angle offset for the plot: Bob's angles swept.
    pub sweep_x: Vec<f64>,
    pub sweep_s: Vec<f64>,
}

/// E(a, b) = V cos(a − b) for the Werner state V|Φ+⟩⟨Φ+| + (1−V) I/4,
/// computed by simulating the circuit for V = 1.
pub fn correlation(a: f64, b: f64, v: f64) -> f64 {
    let c = Circuit::from_ops(
        2,
        0,
        vec![Op::new("h", &[0]), Op::ctrl("x", &[0], &[1]), Op::new("ry", &[0]).param(&[-a]), Op::new("ry", &[1]).param(&[-b])],
    );
    let sv = super::statevec::final_state(&c, 1).expect("valid");
    let p = sv.probs();
    v * (p[0] + p[3] - p[1] - p[2])
}

pub fn chsh(i: &ChshIn) -> ChshOut {
    let v = i.visibility.clamp(0.0, 1.0);
    let shots = i.shots.clamp(10, 1_000_000);
    let mut rng = Rng::new(i.seed);
    let pairs = [(i.a0, i.b0), (i.a0, i.b1), (i.a1, i.b0), (i.a1, i.b1)];
    let signs = [1.0, -1.0, 1.0, 1.0];
    let mut settings = vec![];
    let (mut s_exact, mut s_samp, mut var) = (0.0, 0.0, 0.0);
    for (k, &(a, b)) in pairs.iter().enumerate() {
        let ex = correlation(a, b, v);
        let p_same = (1.0 + ex) / 2.0;
        let mut same = 0;
        for _ in 0..shots {
            if rng.uniform() < p_same {
                same += 1;
            }
        }
        let diff = shots - same;
        let es = (same as f64 - diff as f64) / shots as f64;
        s_exact += signs[k] * ex;
        s_samp += signs[k] * es;
        var += (1.0 - es * es) / shots as f64;
        settings.push(ChshSetting {
            a,
            b,
            e_exact: ex,
            e_sampled: es,
            same,
            diff,
        });
    }
    let sweep_x: Vec<f64> = (0..=72).map(|k| k as f64 * PI / 72.0).collect();
    // rotate Bob's pair together: b0 = a0 + x, b1 = a0 + x + π/2 (keeps Alice fixed)
    let sweep_s = sweep_x
        .iter()
        .map(|&x| {
            let (b0, b1) = (i.a0 + x, i.a0 + x + PI / 2.0);
            v * ((i.a0 - b0).cos() - (i.a0 - b1).cos() + (i.a1 - b0).cos() + (i.a1 - b1).cos())
        })
        .collect();
    ChshOut {
        settings,
        s_exact,
        s_sampled: s_samp,
        s_std: var.sqrt(),
        classical_bound: 2.0,
        tsirelson: 2.0 * 2f64.sqrt(),
        violated: s_exact.abs() > 2.0,
        v_threshold: 1.0 / 2f64.sqrt(),
        sweep_x,
        sweep_s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantum::statevec::{final_state, run};

    #[test]
    fn teleportation_fidelity_is_one() {
        for (th, ph) in [(1.1, 0.7), (0.3, -2.0), (PI, 0.0), (2.2, 1.3)] {
            let p = Params {
                theta: Some(th),
                phi: Some(ph),
                ..Default::default()
            };
            let want = [th.sin() * ph.cos(), th.sin() * ph.sin(), th.cos()];
            // with collapse + classical control, for many seeds (all four outcomes)
            let ent = build("teleport", &p).unwrap();
            for seed in 0..12 {
                let out = run(&ent.circuit, 1, seed, false).unwrap();
                let got = out.state.qubits[2].bloch;
                let fid = (1.0 + got.iter().zip(&want).map(|(a, b)| a * b).sum::<f64>()) / 2.0;
                assert!((fid - 1.0).abs() < 1e-9, "seed {seed}: {got:?} vs {want:?}");
            }
            // deferred-measurement form
            let ent = build("teleport_deferred", &Params { measure: Some(false), ..p }).unwrap();
            let sv = final_state(&ent.circuit, 1).unwrap();
            let got = sv.bloch(2);
            for k in 0..3 {
                assert!((got[k] - want[k]).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn superdense_decodes_every_message() {
        for m in 0..4u64 {
            let ent = build(
                "superdense",
                &Params {
                    message: Some(m),
                    ..Default::default()
                },
            )
            .unwrap();
            let out = run(&ent.circuit, 100, 2, false).unwrap();
            assert_eq!(out.histogram.len(), 1);
            assert_eq!(out.histogram[0].bits, format!("{m:02b}"));
        }
    }

    #[test]
    fn entanglement_swapping_entangles_the_ends() {
        let ent = build("swap_ent", &Params::default()).unwrap();
        for seed in 0..8 {
            let out = run(&ent.circuit, 1, seed, false).unwrap();
            let c = out.state.concurrence.as_ref().unwrap();
            assert!((c[3] - 1.0).abs() < 1e-9, "C(0,3) = {}", c[3]);
        }
    }

    #[test]
    fn grover_matches_sin_squared() {
        for n in 2..=5 {
            let ent = build(
                "grover",
                &Params {
                    n: Some(n),
                    value: Some(1),
                    measure: Some(false),
                    ..Default::default()
                },
            )
            .unwrap();
            let x = ent.extra.unwrap();
            let sim: Vec<f64> = serde_json::from_value(x["p_marked_sim"].clone()).unwrap();
            let th: Vec<f64> = serde_json::from_value(x["p_marked_theory"].clone()).unwrap();
            for (a, b) in sim.iter().zip(&th) {
                assert!((a - b).abs() < 1e-9, "n={n}: {a} vs {b}");
            }
            let k = grover_optimal(n);
            assert_eq!(k, [1, 2, 3, 4][n - 2]);
            let sv = final_state(&ent.circuit, 1).unwrap();
            let theta = (1.0 / ((1u64 << n) as f64).sqrt()).asin();
            assert!((sv.probs()[1] - ((2 * k + 1) as f64 * theta).sin().powi(2)).abs() < 1e-9);
        }
    }

    #[test]
    fn qft_equals_dft() {
        let n = 4;
        let dim = 1 << n;
        let mut rng = Rng::new(3);
        let mut amps: Vec<crate::math::C64> = (0..dim).map(|_| crate::math::C64::new(rng.normal(), rng.normal())).collect();
        let norm = amps.iter().map(|a| a.norm_sqr()).sum::<f64>().sqrt();
        for a in amps.iter_mut() {
            *a = a.scale(1.0 / norm);
        }
        let mut sv = StateVec::from_amps(amps.clone(), 1);
        let qs: Vec<usize> = (0..n).collect();
        for op in qft_ops(&qs) {
            sv.apply(&op);
        }
        for y in 0..dim {
            let mut s = crate::math::C64::ZERO;
            for (x, a) in amps.iter().enumerate() {
                s = s + *a * crate::math::C64::from_polar(1.0, 2.0 * PI * (x * y) as f64 / dim as f64);
            }
            s = s.scale(1.0 / (dim as f64).sqrt());
            assert!((s - sv.amps[y]).abs() < 1e-9, "y={y}");
        }
    }

    #[test]
    fn phase_estimation_reads_exact_phase() {
        let ent = build(
            "qpe",
            &Params {
                n: Some(4),
                theta: Some(0.3125),
                ..Default::default()
            },
        )
        .unwrap();
        let out = run(&ent.circuit, 200, 1, false).unwrap();
        assert_eq!(out.histogram.len(), 1);
        assert_eq!(out.histogram[0].bits, "0101"); // 0.3125 · 16 = 5
    }

    #[test]
    fn deutsch_jozsa_and_bv() {
        for (kind, zero) in [("constant0", true), ("constant1", true), ("balanced", false)] {
            let ent = build(
                "deutsch_jozsa",
                &Params {
                    n: Some(3),
                    kind: Some(kind.into()),
                    value: Some(0b110),
                    ..Default::default()
                },
            )
            .unwrap();
            let out = run(&ent.circuit, 100, 1, false).unwrap();
            let p000 = out.exact.iter().find(|h| h.bits == "000").map(|h| h.prob).unwrap_or(0.0);
            assert!((p000 - if zero { 1.0 } else { 0.0 }).abs() < 1e-9, "{kind}");
        }
        let ent = build(
            "bernstein_vazirani",
            &Params {
                n: Some(4),
                value: Some(0b1011),
                ..Default::default()
            },
        )
        .unwrap();
        let out = run(&ent.circuit, 100, 1, false).unwrap();
        assert_eq!(out.histogram[0].bits, "1011");
    }

    #[test]
    fn chsh_reaches_tsirelson() {
        let o = chsh(&ChshIn::default());
        assert!((o.s_exact - 2.0 * 2f64.sqrt()).abs() < 1e-9, "{}", o.s_exact);
        assert!(o.violated);
        assert!((o.s_sampled - o.s_exact).abs() < 5.0 * o.s_std);
        let w = chsh(&ChshIn {
            visibility: 0.6,
            ..Default::default()
        });
        assert!(!w.violated);
        assert!((correlation(0.3, 1.0, 1.0) - (0.3f64 - 1.0).cos()).abs() < 1e-12);
    }
}
