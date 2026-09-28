//! Density-matrix simulator with noise channels (≤ [`MAX_DM_QUBITS`] qubits).
//!
//! ρ is stored "vectorised" as a 2n-qubit amplitude array: element
//! `ρ[r][c]` lives at index `r + c·2ⁿ`, so row qubit `k` is bit `k` and
//! column qubit `k` is bit `n + k`. Then
//!
//! ```text
//! U ρ U†  =  (U on row qubits) ∘ (U* on column qubits)
//! Σₖ Kₖ ρ Kₖ†  for any Kraus set {Kₖ}
//! ```
//!
//! reuse the state-vector kernels unchanged. Channels (Nielsen & Chuang
//! §8.3): depolarising ρ → (1−p)ρ + p·I/2; bit flip ρ → (1−p)ρ + p XρX;
//! phase flip ρ → (1−p)ρ + p ZρZ; amplitude damping K₀ = [[1,0],[0,√(1−γ)]],
//! K₁ = [[0,√γ],[0,0]] (energy relaxation, T₁); phase damping
//! K₀ = [[1,0],[0,√(1−λ)]], K₁ = [[0,0],[0,√λ]] (pure dephasing, T₂).
//! Readout error flips each measured bit with probability `readout`.

use serde::{Deserialize, Serialize};

use super::circuit::{bitstring, matrix, Circuit, Op, M2};
use super::linalg;
use super::statevec::{self, apply_1q, apply_swap, mask_of, qubit_info, QubitInfo};
use crate::math::C64;

pub const MAX_DM_QUBITS: usize = 6;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Noise {
    /// Depolarising probability per gate, per touched qubit.
    pub depolarizing: f64,
    pub bit_flip: f64,
    pub phase_flip: f64,
    /// Amplitude-damping γ per gate (T₁ decay).
    pub amplitude_damping: f64,
    /// Phase-damping λ per gate (T₂ dephasing).
    pub phase_damping: f64,
    /// Probability that a measured bit is read wrongly.
    pub readout: f64,
    /// Also apply the damping channels to idle qubits in every column.
    pub idle: bool,
}

#[derive(Clone, Debug)]
pub struct DensityMatrix {
    pub n: usize,
    /// Vectorised ρ, length 4ⁿ.
    pub v: Vec<C64>,
}

fn conj_m(m: &M2) -> M2 {
    [[m[0][0].conj(), m[0][1].conj()], [m[1][0].conj(), m[1][1].conj()]]
}

fn m2(a: f64, b: f64, cc: f64, d: f64) -> M2 {
    [[C64::real(a), C64::real(b)], [C64::real(cc), C64::real(d)]]
}

impl DensityMatrix {
    pub fn new(n: usize) -> DensityMatrix {
        let mut v = vec![C64::ZERO; 1 << (2 * n)];
        v[0] = C64::ONE;
        DensityMatrix { n, v }
    }

    pub fn dim(&self) -> usize {
        1 << self.n
    }

    pub fn get(&self, r: usize, c: usize) -> C64 {
        self.v[r + c * self.dim()]
    }

    /// ρ as a row-major d×d matrix.
    pub fn matrix(&self) -> Vec<C64> {
        let d = self.dim();
        let mut m = vec![C64::ZERO; d * d];
        for r in 0..d {
            for c in 0..d {
                m[r * d + c] = self.get(r, c);
            }
        }
        m
    }

    fn unitary_1q(&mut self, t: usize, controls: &[usize], m: &M2) {
        let mask = mask_of(controls);
        apply_1q(&mut self.v, t, mask, m);
        apply_1q(&mut self.v, t + self.n, mask << self.n, &conj_m(m));
    }

    /// ρ → Σ K ρ K† on qubit q.
    pub fn kraus(&mut self, q: usize, ks: &[M2]) {
        let mut acc = vec![C64::ZERO; self.v.len()];
        for k in ks {
            let mut tmp = self.v.clone();
            apply_1q(&mut tmp, q, 0, k);
            apply_1q(&mut tmp, q + self.n, 0, &conj_m(k));
            for (a, b) in acc.iter_mut().zip(&tmp) {
                *a = *a + *b;
            }
        }
        self.v = acc;
    }

    pub fn apply(&mut self, op: &Op) -> Result<(), String> {
        if op.cond.is_some() {
            return Err("classically controlled gates are not supported by the density-matrix simulator — use the deferred-measurement form (controlled gates instead of measure + if)".into());
        }
        match op.gate.as_str() {
            "barrier" | "id" => {}
            "swap" | "iswap" => {
                let f = if op.gate == "swap" { C64::ONE } else { C64::J };
                let mask = mask_of(&op.controls);
                let (a, b) = (op.targets[0], op.targets[1]);
                apply_swap(&mut self.v, a, b, mask, f);
                apply_swap(&mut self.v, a + self.n, b + self.n, mask << self.n, f.conj());
            }
            // Non-selective measurement: dephase in the computational basis.
            "measure" => self.kraus(op.targets[0], &[m2(1.0, 0.0, 0.0, 0.0), m2(0.0, 0.0, 0.0, 1.0)]),
            "reset" => self.kraus(op.targets[0], &[m2(1.0, 0.0, 0.0, 0.0), m2(0.0, 1.0, 0.0, 0.0)]),
            g => {
                let m = matrix(g, &op.params).ok_or_else(|| format!("unknown gate `{g}`"))?;
                self.unitary_1q(op.targets[0], &op.controls, &m);
            }
        }
        Ok(())
    }

    pub fn noise_on(&mut self, q: usize, nz: &Noise, gate_noise: bool) {
        if gate_noise {
            let p = nz.depolarizing.clamp(0.0, 1.0);
            if p > 0.0 {
                // (1−p)ρ + p I/2 = (1 − 3p/4) ρ + (p/4)(XρX + YρY + ZρZ)
                let a = (1.0 - 0.75 * p).sqrt();
                let b = (p / 4.0).sqrt();
                let x = matrix("x", &[]).unwrap();
                let y = matrix("y", &[]).unwrap();
                let z = matrix("z", &[]).unwrap();
                let sc = |m: M2, s: f64| [[m[0][0].scale(s), m[0][1].scale(s)], [m[1][0].scale(s), m[1][1].scale(s)]];
                self.kraus(q, &[m2(a, 0.0, 0.0, a), sc(x, b), sc(y, b), sc(z, b)]);
            }
            let p = nz.bit_flip.clamp(0.0, 1.0);
            if p > 0.0 {
                self.kraus(
                    q,
                    &[m2((1.0 - p).sqrt(), 0.0, 0.0, (1.0 - p).sqrt()), m2(0.0, p.sqrt(), p.sqrt(), 0.0)],
                );
            }
            let p = nz.phase_flip.clamp(0.0, 1.0);
            if p > 0.0 {
                self.kraus(
                    q,
                    &[m2((1.0 - p).sqrt(), 0.0, 0.0, (1.0 - p).sqrt()), m2(p.sqrt(), 0.0, 0.0, -p.sqrt())],
                );
            }
        }
        let g = nz.amplitude_damping.clamp(0.0, 1.0);
        if g > 0.0 {
            self.kraus(q, &[m2(1.0, 0.0, 0.0, (1.0 - g).sqrt()), m2(0.0, g.sqrt(), 0.0, 0.0)]);
        }
        let l = nz.phase_damping.clamp(0.0, 1.0);
        if l > 0.0 {
            self.kraus(q, &[m2(1.0, 0.0, 0.0, (1.0 - l).sqrt()), m2(0.0, 0.0, 0.0, l.sqrt())]);
        }
    }

    pub fn probs(&self) -> Vec<f64> {
        (0..self.dim()).map(|i| self.get(i, i).re.max(0.0)).collect()
    }

    pub fn reduced_1(&self, q: usize) -> [C64; 4] {
        let bit = 1 << q;
        let (mut r00, mut r11, mut r01) = (0.0, 0.0, C64::ZERO);
        for i in 0..self.dim() {
            if i & bit == 0 {
                r00 += self.get(i, i).re;
                r11 += self.get(i | bit, i | bit).re;
                r01 = r01 + self.get(i, i | bit);
            }
        }
        [C64::real(r00), r01, r01.conj(), C64::real(r11)]
    }

    pub fn reduced_2(&self, qa: usize, qb: usize) -> Vec<C64> {
        let (ba, bb) = (1usize << qa, 1usize << qb);
        let mut rho = vec![C64::ZERO; 16];
        for i in 0..self.dim() {
            if i & ba != 0 || i & bb != 0 {
                continue;
            }
            let idx = [i, i | ba, i | bb, i | ba | bb];
            for r in 0..4 {
                for cc in 0..4 {
                    rho[r * 4 + cc] = rho[r * 4 + cc] + self.get(idx[r], idx[cc]);
                }
            }
        }
        rho
    }

    pub fn purity(&self) -> f64 {
        self.v.iter().map(|z| z.norm_sqr()).sum()
    }

    /// ⟨ψ|ρ|ψ⟩.
    pub fn overlap(&self, psi: &[C64]) -> f64 {
        let d = self.dim();
        let mut s = C64::ZERO;
        for r in 0..d {
            if psi[r].norm_sqr() == 0.0 {
                continue;
            }
            for c in 0..d {
                s = s + psi[r].conj() * self.get(r, c) * psi[c];
            }
        }
        s.re
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DmStep {
    pub column: usize,
    pub qubits: Vec<QubitInfo>,
    /// Fidelity with the ideal (noise-free) state after the same column.
    pub fidelity: Option<f64>,
    pub purity: f64,
    pub probs: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DmOut {
    pub qubits: usize,
    pub steps: Vec<DmStep>,
    pub labels: Vec<String>,
    /// Final outcome probabilities (qubit basis) including readout error.
    pub probs_measured: Vec<f64>,
    pub probs_ideal: Vec<f64>,
    pub fidelity: Option<f64>,
    pub purity: f64,
    pub entropy: f64,
    /// Final ρ (real and imaginary parts, row-major) when n ≤ 3.
    pub rho_re: Option<Vec<f64>>,
    pub rho_im: Option<Vec<f64>>,
    /// Concurrence of qubits (0, 1) when n ≥ 2.
    pub concurrence01: Option<f64>,
    pub concurrence01_ideal: Option<f64>,
    pub total_variation: f64,
    pub notes: Vec<String>,
}

/// Classical readout confusion applied bit by bit.
pub fn apply_readout(probs: &[f64], n: usize, e: f64) -> Vec<f64> {
    let mut p = probs.to_vec();
    if e <= 0.0 {
        return p;
    }
    for q in 0..n {
        let bit = 1 << q;
        let mut out = p.clone();
        for i in 0..p.len() {
            if i & bit == 0 {
                let (a, b) = (p[i], p[i | bit]);
                out[i] = (1.0 - e) * a + e * b;
                out[i | bit] = e * a + (1.0 - e) * b;
            }
        }
        p = out;
    }
    p
}

pub fn run_noisy(circ: &Circuit, nz: &Noise) -> Result<DmOut, String> {
    let c = circ.validated()?;
    if c.qubits > MAX_DM_QUBITS {
        return Err(format!(
            "the density-matrix simulator handles at most {MAX_DM_QUBITS} qubits (ρ has 4^n entries)"
        ));
    }
    let n = c.qubits;
    let mut rho = DensityMatrix::new(n);
    let mut ideal = DensityMatrix::new(n);
    let mut sv = statevec::StateVec::new(n, c.clbits, 1);
    let mut pure = true;
    let mut steps = vec![];
    let snap = |rho: &DensityMatrix, col: usize, fid: Option<f64>| DmStep {
        column: col,
        qubits: (0..n).map(|q| qubit_info(statevec::bloch_of(&rho.reduced_1(q)))).collect(),
        fidelity: fid,
        purity: rho.purity(),
        probs: rho.probs(),
    };
    steps.push(snap(&rho, 0, Some(1.0)));
    let mut notes = vec![];
    for (ci, col) in c.cols.iter().enumerate() {
        let mut touched = vec![false; n];
        for op in col {
            rho.apply(op)?;
            ideal.apply(op)?;
            if matches!(op.gate.as_str(), "measure" | "reset") {
                pure = false;
            } else {
                sv.apply(op);
            }
            if op.gate != "barrier" {
                for q in op.qubits() {
                    touched[q] = true;
                }
            }
        }
        for (q, &t) in touched.iter().enumerate() {
            if t {
                rho.noise_on(q, nz, true);
            } else if nz.idle {
                rho.noise_on(q, nz, false);
            }
        }
        let fid = if pure {
            Some(rho.overlap(&sv.amps))
        } else if n <= 4 {
            Some(linalg::fidelity(&ideal.matrix(), &rho.matrix(), 1 << n))
        } else {
            None
        };
        steps.push(snap(&rho, ci + 1, fid));
    }
    if !pure && n > 4 {
        notes.push("fidelity after a mid-circuit measurement is only computed for ≤ 4 qubits".into());
    }
    let probs_ideal = ideal.probs();
    let probs_measured = apply_readout(&rho.probs(), n, nz.readout);
    let tv = 0.5 * probs_ideal.iter().zip(&probs_measured).map(|(a, b)| (a - b).abs()).sum::<f64>();
    let d = 1 << n;
    let (rho_re, rho_im) = if n <= 3 {
        let m = rho.matrix();
        (Some(m.iter().map(|z| z.re).collect()), Some(m.iter().map(|z| z.im).collect()))
    } else {
        (None, None)
    };
    Ok(DmOut {
        qubits: n,
        labels: (0..d).map(|i| bitstring(i, n)).collect(),
        probs_measured,
        probs_ideal,
        fidelity: steps.last().and_then(|s| s.fidelity),
        purity: rho.purity(),
        entropy: linalg::entropy(&rho.matrix(), d),
        rho_re,
        rho_im,
        concurrence01: (n >= 2).then(|| linalg::concurrence(&rho.reduced_2(0, 1))),
        concurrence01_ideal: (n >= 2).then(|| linalg::concurrence(&ideal.reduced_2(0, 1))),
        total_variation: tv,
        steps,
        notes,
    })
}

#[derive(Clone, Debug, Serialize)]
pub struct SweepOut {
    pub channel: String,
    pub p: Vec<f64>,
    pub fidelity: Vec<f64>,
    pub purity: Vec<f64>,
    pub success: Vec<f64>,
    /// The most likely ideal outcome whose probability `success` tracks.
    pub target: String,
}

/// Fidelity and success probability vs the strength of one channel.
pub fn sweep(circ: &Circuit, channel: &str, pmax: f64, points: usize, readout: bool) -> Result<SweepOut, String> {
    let points = points.clamp(2, 41);
    let mut out = SweepOut {
        channel: channel.into(),
        p: vec![],
        fidelity: vec![],
        purity: vec![],
        success: vec![],
        target: String::new(),
    };
    for k in 0..points {
        let p = pmax * k as f64 / (points - 1) as f64;
        let mut nz = Noise::default();
        match channel {
            "depolarizing" => nz.depolarizing = p,
            "bit_flip" => nz.bit_flip = p,
            "phase_flip" => nz.phase_flip = p,
            "amplitude_damping" => nz.amplitude_damping = p,
            "phase_damping" => nz.phase_damping = p,
            "readout" => nz.readout = p,
            _ => return Err(format!("unknown channel `{channel}`")),
        }
        let _ = readout;
        let r = run_noisy(circ, &nz)?;
        let best = r
            .probs_ideal
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);
        if k == 0 {
            out.target = r.labels[best].clone();
        }
        out.p.push(p);
        out.fidelity.push(r.fidelity.unwrap_or(f64::NAN));
        out.purity.push(r.purity);
        out.success.push(r.probs_measured[best]);
    }
    Ok(out)
}

/// Action of a single-qubit channel on the Bloch ball: r → M r + t.
#[derive(Clone, Debug, Serialize)]
pub struct ChannelOut {
    pub m: [[f64; 3]; 3],
    pub t: [f64; 3],
    /// Bloch vector after 0..=steps applications.
    pub trajectory: Vec<[f64; 3]>,
    pub purity: Vec<f64>,
}

pub fn channel_map(nz: &Noise, start: [f64; 3], steps: usize) -> ChannelOut {
    // Push the Pauli basis through the channel to read off the affine map.
    let apply = |b: [f64; 3]| -> [f64; 3] {
        let mut dm = DensityMatrix::new(1);
        let r = [
            C64::real((1.0 + b[2]) / 2.0),
            C64::new(b[0] / 2.0, -b[1] / 2.0),
            C64::new(b[0] / 2.0, b[1] / 2.0),
            C64::real((1.0 - b[2]) / 2.0),
        ];
        // vectorised: index r + 2c
        dm.v = vec![r[0], r[2], r[1], r[3]];
        dm.noise_on(0, nz, true);
        statevec::bloch_of(&dm.reduced_1(0))
    };
    let t = apply([0.0, 0.0, 0.0]);
    let mut m = [[0.0; 3]; 3];
    for j in 0..3 {
        let mut e = [0.0; 3];
        e[j] = 1.0;
        let y = apply(e);
        for i in 0..3 {
            m[i][j] = y[i] - t[i];
        }
    }
    let mut traj = vec![start];
    let mut cur = start;
    for _ in 0..steps.min(200) {
        cur = apply(cur);
        traj.push(cur);
    }
    let purity = traj.iter().map(|b| (1.0 + b[0] * b[0] + b[1] * b[1] + b[2] * b[2]) / 2.0).collect();
    ChannelOut {
        m,
        t,
        trajectory: traj,
        purity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bell() -> Circuit {
        Circuit::from_ops(2, 0, vec![Op::new("h", &[0]), Op::ctrl("x", &[0], &[1])])
    }

    #[test]
    fn noiseless_matches_state_vector() {
        let r = run_noisy(&bell(), &Noise::default()).unwrap();
        assert!((r.fidelity.unwrap() - 1.0).abs() < 1e-12);
        assert!((r.purity - 1.0).abs() < 1e-12);
        assert!((r.concurrence01.unwrap() - 1.0).abs() < 1e-9);
        assert!((r.probs_measured[0] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn depolarising_shrinks_the_bloch_vector() {
        let nz = Noise {
            depolarizing: 0.2,
            ..Default::default()
        };
        let ch = channel_map(&nz, [0.0, 0.0, 1.0], 3);
        for i in 0..3 {
            assert!((ch.m[i][i] - 0.8).abs() < 1e-12);
        }
        assert!((ch.trajectory[3][2] - 0.8f64.powi(3)).abs() < 1e-12);
        // amplitude damping: z → γ + (1−γ) z, x → √(1−γ) x
        let ad = Noise {
            amplitude_damping: 0.3,
            ..Default::default()
        };
        let ch = channel_map(&ad, [1.0, 0.0, 0.0], 1);
        assert!((ch.t[2] - 0.3).abs() < 1e-12 && (ch.m[2][2] - 0.7).abs() < 1e-12);
        assert!((ch.m[0][0] - 0.7f64.sqrt()).abs() < 1e-12);
        // phase flip: x,y scaled by 1 − 2p
        let pf = Noise {
            phase_flip: 0.1,
            ..Default::default()
        };
        let ch = channel_map(&pf, [1.0, 0.0, 0.0], 1);
        assert!((ch.m[0][0] - 0.8).abs() < 1e-12 && (ch.m[2][2] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn noisy_bell_fidelity_drops() {
        let nz = Noise {
            depolarizing: 0.1,
            ..Default::default()
        };
        let r = run_noisy(&bell(), &nz).unwrap();
        let f = r.fidelity.unwrap();
        assert!(f < 0.95 && f > 0.7, "{f}");
        // readout error mixes 00 into 01/10
        let nz = Noise {
            readout: 0.1,
            ..Default::default()
        };
        let r = run_noisy(&bell(), &nz).unwrap();
        assert!((r.probs_measured[1] - 0.09).abs() < 1e-12, "{:?}", r.probs_measured);
        let s = sweep(&bell(), "depolarizing", 0.5, 6, false).unwrap();
        assert!(s.fidelity.windows(2).all(|w| w[1] <= w[0] + 1e-12));
    }
}
