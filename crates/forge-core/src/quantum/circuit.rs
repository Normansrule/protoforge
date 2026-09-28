//! Circuit model shared by the simulators, the OpenQASM 2.0 reader/writer,
//! the Quirk exporter and the algorithm library.
//!
//! A circuit is a list of **columns** (time slices) of operations, the same
//! layout as Quirk and the IBM Quantum Composer grid. Each [`Op`] names a base
//! gate, the target qubit(s), optional **control** qubits (any number, so
//! `x` with two controls is a Toffoli and `swap` with one control is a
//! Fredkin), rotation parameters in radians, the classical bit a measurement
//! writes to and an optional classical condition (OpenQASM `if (c==v)`).
//!
//! Bit order follows Qiskit: basis index `i` has qubit `k` in bit `k` of `i`
//! (little-endian), and bit strings are printed `q[n-1] … q[0]`.

use serde::{Deserialize, Serialize};

use crate::math::{C64, PI};

/// Classical condition: apply the op only when the classical bits `bits`
/// (least-significant first) read `value`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Cond {
    pub bits: Vec<usize>,
    pub value: u64,
}

/// One operation in a column.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Op {
    /// Base gate: `id h x y z s sdg t tdg sx sxdg rx ry rz p u3 swap iswap
    /// measure reset barrier` (aliases such as `cx`, `ccx`, `cz`, `cp`, `u1`
    /// and `cswap` are normalised by [`normalize`]).
    pub gate: String,
    #[serde(default)]
    pub targets: Vec<usize>,
    #[serde(default)]
    pub controls: Vec<usize>,
    /// Angles in radians (`rx ry rz p`: θ; `u3`: θ, φ, λ).
    #[serde(default)]
    pub params: Vec<f64>,
    /// Classical bit written by `measure`.
    #[serde(default)]
    pub clbit: Option<usize>,
    #[serde(default)]
    pub cond: Option<Cond>,
    /// Optional display label (e.g. "oracle"); ignored by the simulators.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A named classical register (used when emitting OpenQASM).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Creg {
    pub name: String,
    pub size: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Circuit {
    pub qubits: usize,
    #[serde(default)]
    pub clbits: usize,
    #[serde(default)]
    pub cols: Vec<Vec<Op>>,
    /// Classical registers in order; empty means one register `c[clbits]`.
    #[serde(default)]
    pub cregs: Vec<Creg>,
}

/// 2×2 complex matrix, row-major.
pub type M2 = [[C64; 2]; 2];

pub fn c(re: f64, im: f64) -> C64 {
    C64::new(re, im)
}

/// Matrix of a single-qubit gate.
pub fn matrix(gate: &str, p: &[f64]) -> Option<M2> {
    let s = std::f64::consts::FRAC_1_SQRT_2;
    let z = C64::ZERO;
    let o = C64::ONE;
    let arg = |k: usize| p.get(k).copied().unwrap_or(0.0);
    Some(match gate {
        "id" => [[o, z], [z, o]],
        "h" => [[c(s, 0.0), c(s, 0.0)], [c(s, 0.0), c(-s, 0.0)]],
        "x" => [[z, o], [o, z]],
        "y" => [[z, c(0.0, -1.0)], [c(0.0, 1.0), z]],
        "z" => [[o, z], [z, c(-1.0, 0.0)]],
        "s" => [[o, z], [z, C64::J]],
        "sdg" => [[o, z], [z, c(0.0, -1.0)]],
        "t" => [[o, z], [z, C64::from_polar(1.0, PI / 4.0)]],
        "tdg" => [[o, z], [z, C64::from_polar(1.0, -PI / 4.0)]],
        // √X = ½[[1+i, 1−i], [1−i, 1+i]] (Qiskit's SXGate)
        "sx" => [[c(0.5, 0.5), c(0.5, -0.5)], [c(0.5, -0.5), c(0.5, 0.5)]],
        "sxdg" => [[c(0.5, -0.5), c(0.5, 0.5)], [c(0.5, 0.5), c(0.5, -0.5)]],
        "rx" => {
            let (cs, sn) = ((arg(0) / 2.0).cos(), (arg(0) / 2.0).sin());
            [[c(cs, 0.0), c(0.0, -sn)], [c(0.0, -sn), c(cs, 0.0)]]
        }
        "ry" => {
            let (cs, sn) = ((arg(0) / 2.0).cos(), (arg(0) / 2.0).sin());
            [[c(cs, 0.0), c(-sn, 0.0)], [c(sn, 0.0), c(cs, 0.0)]]
        }
        "rz" => [[C64::from_polar(1.0, -arg(0) / 2.0), z], [z, C64::from_polar(1.0, arg(0) / 2.0)]],
        "p" => [[o, z], [z, C64::from_polar(1.0, arg(0))]],
        // U(θ,φ,λ) = [[cos θ/2, −e^{iλ} sin θ/2], [e^{iφ} sin θ/2, e^{i(φ+λ)} cos θ/2]]
        "u3" => {
            let (th, ph, la) = (arg(0), arg(1), arg(2));
            let (cs, sn) = ((th / 2.0).cos(), (th / 2.0).sin());
            [
                [c(cs, 0.0), C64::from_polar(-sn, la)],
                [C64::from_polar(sn, ph), C64::from_polar(cs, ph + la)],
            ]
        }
        _ => return None,
    })
}

/// Gates that act as a unitary on one target qubit.
pub fn is_single(gate: &str) -> bool {
    matches!(
        gate,
        "id" | "h" | "x" | "y" | "z" | "s" | "sdg" | "t" | "tdg" | "sx" | "sxdg" | "rx" | "ry" | "rz" | "p" | "u3"
    )
}

pub fn n_params(gate: &str) -> usize {
    match gate {
        "rx" | "ry" | "rz" | "p" => 1,
        "u3" => 3,
        _ => 0,
    }
}

/// Human-facing catalog entry for the composer palette.
#[derive(Clone, Debug, Serialize)]
pub struct GateInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub name: &'static str,
    pub targets: usize,
    pub params: Vec<&'static str>,
    pub group: &'static str,
    pub qasm: &'static str,
    pub description: &'static str,
}

pub fn catalog() -> Vec<GateInfo> {
    let g = |id, label, name, targets, params: Vec<&'static str>, group, qasm, description| GateInfo {
        id,
        label,
        name,
        targets,
        params,
        group,
        qasm,
        description,
    };
    vec![
        g(
            "h",
            "H",
            "Hadamard",
            1,
            vec![],
            "Clifford",
            "h",
            "Maps |0⟩ → |+⟩ and |1⟩ → |−⟩: a 180° turn about the X+Z axis. Creates superposition.",
        ),
        g(
            "x",
            "X",
            "Pauli-X (NOT)",
            1,
            vec![],
            "Pauli",
            "x",
            "Bit flip: 180° about X. With a control dot it is the CNOT; with two, the Toffoli.",
        ),
        g("y", "Y", "Pauli-Y", 1, vec![], "Pauli", "y", "Bit and phase flip: 180° about Y."),
        g(
            "z",
            "Z",
            "Pauli-Z",
            1,
            vec![],
            "Pauli",
            "z",
            "Phase flip: |1⟩ → −|1⟩, 180° about Z. With a control it is CZ (symmetric).",
        ),
        g("s", "S", "S (√Z)", 1, vec![], "Clifford", "s", "Quarter turn about Z: |1⟩ → i|1⟩."),
        g("sdg", "S†", "S-dagger", 1, vec![], "Clifford", "sdg", "Inverse of S: |1⟩ → −i|1⟩."),
        g(
            "t",
            "T",
            "T (√S)",
            1,
            vec![],
            "Phase",
            "t",
            "Eighth turn about Z: |1⟩ → e^{iπ/4}|1⟩. With H it makes a universal gate set.",
        ),
        g("tdg", "T†", "T-dagger", 1, vec![], "Phase", "tdg", "Inverse of T."),
        g(
            "sx",
            "√X",
            "Square root of X",
            1,
            vec![],
            "Clifford",
            "sx",
            "Quarter turn about X; two in a row equal X. Native on IBM hardware.",
        ),
        g(
            "rx",
            "RX",
            "X rotation",
            1,
            vec!["θ"],
            "Rotation",
            "rx",
            "RX(θ) = exp(−iθX/2): rotate the Bloch vector by θ about X.",
        ),
        g(
            "ry",
            "RY",
            "Y rotation",
            1,
            vec!["θ"],
            "Rotation",
            "ry",
            "RY(θ) = exp(−iθY/2): real amplitudes cos θ/2, sin θ/2.",
        ),
        g(
            "rz",
            "RZ",
            "Z rotation",
            1,
            vec!["θ"],
            "Rotation",
            "rz",
            "RZ(θ) = exp(−iθZ/2): rotate about Z (a phase, up to global phase).",
        ),
        g(
            "p",
            "P",
            "Phase",
            1,
            vec!["λ"],
            "Rotation",
            "u1",
            "P(λ) = diag(1, e^{iλ}). With a control it is the controlled-phase of the QFT.",
        ),
        g(
            "u3",
            "U",
            "General unitary U(θ,φ,λ)",
            1,
            vec!["θ", "φ", "λ"],
            "Rotation",
            "u3",
            "Any single-qubit gate: U = RZ(φ)·RY(θ)·RZ(λ) up to global phase.",
        ),
        g(
            "swap",
            "×",
            "SWAP",
            2,
            vec![],
            "Two-qubit",
            "swap",
            "Exchanges two qubits. With a control it is the Fredkin gate.",
        ),
        g(
            "iswap",
            "iSW",
            "iSWAP",
            2,
            vec![],
            "Two-qubit",
            "iswap",
            "Swaps |01⟩ ↔ |10⟩ with a phase i: native to many superconducting couplers.",
        ),
        g(
            "measure",
            "M",
            "Measure",
            1,
            vec![],
            "Non-unitary",
            "measure",
            "Projects onto |0⟩/|1⟩ with the Born-rule probabilities and writes a classical bit.",
        ),
        g(
            "reset",
            "|0⟩",
            "Reset",
            1,
            vec![],
            "Non-unitary",
            "reset",
            "Measures and flips to |0⟩: re-initialises a qubit mid-circuit.",
        ),
    ]
}

impl Op {
    pub fn new(gate: &str, targets: &[usize]) -> Op {
        Op {
            gate: gate.into(),
            targets: targets.to_vec(),
            ..Default::default()
        }
    }
    pub fn ctrl(gate: &str, controls: &[usize], targets: &[usize]) -> Op {
        Op {
            gate: gate.into(),
            targets: targets.to_vec(),
            controls: controls.to_vec(),
            ..Default::default()
        }
    }
    pub fn param(mut self, p: &[f64]) -> Op {
        self.params = p.to_vec();
        self
    }
    pub fn measure(q: usize, cbit: usize) -> Op {
        Op {
            gate: "measure".into(),
            targets: vec![q],
            clbit: Some(cbit),
            ..Default::default()
        }
    }
    pub fn when(mut self, bits: &[usize], value: u64) -> Op {
        self.cond = Some(Cond {
            bits: bits.to_vec(),
            value,
        });
        self
    }
    pub fn labelled(mut self, l: &str) -> Op {
        self.label = Some(l.into());
        self
    }
    /// Every qubit the op touches.
    pub fn qubits(&self) -> Vec<usize> {
        let mut v = self.controls.clone();
        v.extend(&self.targets);
        v
    }
}

/// Canonicalise gate aliases: `cx`/`cnot` → `x` with one control, `ccx` /
/// `toffoli` → two controls, `cz cy ch crx cry crz cp cu1 cu3 csx cswap`,
/// `u1` → `p`, `u` → `u3`, `u2(φ,λ)` → `u3(π/2,φ,λ)`, `fredkin` → `cswap`,
/// `c3x`/`c4x`. Leading target entries become controls.
pub fn normalize(op: &Op) -> Result<Op, String> {
    let mut o = op.clone();
    o.gate = o.gate.to_ascii_lowercase();
    let (base, nctrl) = match o.gate.as_str() {
        "cx" | "cnot" => ("x", 1),
        "ccx" | "toffoli" => ("x", 2),
        "c3x" => ("x", 3),
        "c4x" => ("x", 4),
        "cy" => ("y", 1),
        "cz" => ("z", 1),
        "ccz" => ("z", 2),
        "ch" => ("h", 1),
        "cs" => ("s", 1),
        "csdg" => ("sdg", 1),
        "ct" => ("t", 1),
        "csx" => ("sx", 1),
        "crx" => ("rx", 1),
        "cry" => ("ry", 1),
        "crz" => ("rz", 1),
        "cp" | "cu1" | "cphase" => ("p", 1),
        "cu3" => ("u3", 1),
        "cswap" | "fredkin" => ("swap", 1),
        "u1" | "phase" => ("p", 0),
        "u" => ("u3", 0),
        "i" => ("id", 0),
        "not" => ("x", 0),
        "sdag" => ("sdg", 0),
        "tdag" => ("tdg", 0),
        "u2" => {
            o.params = vec![
                PI / 2.0,
                o.params.first().copied().unwrap_or(0.0),
                o.params.get(1).copied().unwrap_or(0.0),
            ];
            ("u3", 0)
        }
        g => (g, 0),
    };
    let base = base.to_string();
    if nctrl > 0 {
        let need = nctrl + if base == "swap" { 2 } else { 1 };
        if o.targets.len() == need {
            let mut ctrls: Vec<usize> = o.targets[..nctrl].to_vec();
            ctrls.extend(&o.controls);
            o.controls = ctrls;
            o.targets = o.targets[nctrl..].to_vec();
        } else if o.targets.len() + o.controls.len() != need {
            return Err(format!("`{}` needs {need} qubits", op.gate));
        }
    }
    o.gate = base;
    let known = is_single(&o.gate) || matches!(o.gate.as_str(), "swap" | "iswap" | "measure" | "reset" | "barrier");
    if !known {
        return Err(format!("unknown gate `{}`", op.gate));
    }
    let want_t = match o.gate.as_str() {
        "swap" | "iswap" => 2,
        "barrier" => o.targets.len().max(1),
        _ => 1,
    };
    if o.targets.len() != want_t {
        return Err(format!("`{}` takes {want_t} target qubit(s), got {}", op.gate, o.targets.len()));
    }
    let np = n_params(&o.gate);
    if o.params.len() < np {
        return Err(format!("`{}` needs {np} parameter(s)", o.gate));
    }
    o.params.truncate(np);
    if matches!(o.gate.as_str(), "measure" | "reset" | "barrier") && !o.controls.is_empty() {
        return Err(format!("`{}` cannot be controlled", o.gate));
    }
    Ok(o)
}

impl Circuit {
    pub fn new(qubits: usize, clbits: usize) -> Circuit {
        Circuit {
            qubits,
            clbits,
            cols: vec![],
            cregs: vec![],
        }
    }

    /// All ops in time order.
    pub fn ops(&self) -> impl Iterator<Item = &Op> {
        self.cols.iter().flatten()
    }

    /// Check indices and normalise every op.
    pub fn validated(&self) -> Result<Circuit, String> {
        if self.qubits == 0 {
            return Err("the circuit needs at least one qubit".into());
        }
        let mut out = self.clone();
        for (ci, col) in out.cols.iter_mut().enumerate() {
            // Targets may appear once per column; control qubits may be shared
            // by several ops (Quirk's "controls apply to the whole column").
            let mut targets: Vec<usize> = vec![];
            let mut controls: Vec<usize> = vec![];
            for op in col.iter_mut() {
                *op = normalize(op).map_err(|e| format!("column {}: {e}", ci + 1))?;
                if op.gate == "barrier" {
                    continue;
                }
                for &q in op.targets.iter().chain(&op.controls) {
                    if q >= self.qubits {
                        return Err(format!("column {}: qubit {q} does not exist ({} qubits)", ci + 1, self.qubits));
                    }
                }
                for &q in &op.targets {
                    if targets.contains(&q) || controls.contains(&q) || op.controls.contains(&q) {
                        return Err(format!("column {}: qubit {q} is used twice in one column", ci + 1));
                    }
                    targets.push(q);
                }
                for &q in &op.controls {
                    if targets.contains(&q) {
                        return Err(format!("column {}: qubit {q} is both a control and a target", ci + 1));
                    }
                    if !controls.contains(&q) {
                        controls.push(q);
                    }
                }
                if let Some(b) = op.clbit {
                    if b >= self.clbits {
                        return Err(format!(
                            "column {}: classical bit {b} does not exist ({} bits)",
                            ci + 1,
                            self.clbits
                        ));
                    }
                }
                if op.gate == "measure" && op.clbit.is_none() {
                    op.clbit = if op.targets[0] < self.clbits { Some(op.targets[0]) } else { None };
                }
                if let Some(cd) = &op.cond {
                    if cd.bits.iter().any(|&b| b >= self.clbits) {
                        return Err(format!("column {}: condition uses a classical bit that does not exist", ci + 1));
                    }
                }
            }
        }
        Ok(out)
    }

    /// Pack ops into columns as early as possible (ASAP layering). Ops with
    /// controls or two targets get a column of their own so the grid view
    /// can use Quirk's "controls apply to the whole column" convention.
    pub fn from_ops(qubits: usize, clbits: usize, ops: Vec<Op>) -> Circuit {
        let mut cols: Vec<Vec<Op>> = vec![];
        let mut q_free = vec![0usize; qubits];
        let mut c_free = vec![0usize; clbits.max(1)];
        for op in ops {
            let solo = !op.controls.is_empty() || op.targets.len() > 1 || op.cond.is_some();
            let qs: Vec<usize> = if op.gate == "barrier" { (0..qubits).collect() } else { op.qubits() };
            let mut cs: Vec<usize> = op.cond.as_ref().map(|c| c.bits.clone()).unwrap_or_default();
            if let Some(b) = op.clbit {
                cs.push(b);
            }
            let mut start = qs.iter().map(|&q| q_free.get(q).copied().unwrap_or(0)).max().unwrap_or(0);
            start = start.max(cs.iter().map(|&b| c_free.get(b).copied().unwrap_or(0)).max().unwrap_or(0));
            let mut col = start;
            while col < cols.len() {
                let busy = cols[col].iter().any(|o: &Op| {
                    let other_solo = !o.controls.is_empty() || o.targets.len() > 1 || o.cond.is_some() || o.gate == "barrier";
                    other_solo || o.qubits().iter().any(|q| qs.contains(q))
                });
                if !(busy || (solo || op.gate == "barrier") && !cols[col].is_empty()) {
                    break;
                }
                col += 1;
            }
            if col >= cols.len() {
                cols.resize(col + 1, vec![]);
            }
            cols[col].push(op);
            for &q in &qs {
                if q < q_free.len() {
                    q_free[q] = col + 1;
                }
            }
            for &b in &cs {
                if b < c_free.len() {
                    c_free[b] = col + 1;
                }
            }
        }
        Circuit {
            qubits,
            clbits,
            cols,
            cregs: vec![],
        }
    }

    pub fn with_cregs(mut self, cregs: &[(&str, usize)]) -> Circuit {
        self.cregs = cregs
            .iter()
            .map(|(n, s)| Creg {
                name: (*n).into(),
                size: *s,
            })
            .collect();
        self
    }

    /// Classical registers, defaulting to one register `c` covering every bit.
    pub fn registers(&self) -> Vec<Creg> {
        if self.cregs.is_empty() {
            if self.clbits == 0 {
                vec![]
            } else {
                vec![Creg {
                    name: "c".into(),
                    size: self.clbits,
                }]
            }
        } else {
            self.cregs.clone()
        }
    }

    /// True when every measurement is terminal (nothing but measurements and
    /// barriers act on a qubit after it is measured) and there are no
    /// resets or classical conditions, so shots can be sampled from one run.
    pub fn is_terminal_measurement_only(&self) -> bool {
        let mut measured = vec![false; self.qubits];
        for op in self.ops() {
            match op.gate.as_str() {
                "reset" => return false,
                "barrier" => {}
                "measure" => measured[op.targets[0]] = true,
                _ => {
                    if op.cond.is_some() || op.qubits().iter().any(|&q| measured[q]) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// A copy without measurements (and barriers), for the ideal state.
    pub fn unitary_part(&self) -> Circuit {
        let mut out = self.clone();
        for col in out.cols.iter_mut() {
            col.retain(|o| !matches!(o.gate.as_str(), "measure" | "barrier"));
        }
        out
    }
}

/// Adjoint of a unitary op (for inverse QFT and uncomputation).
pub fn adjoint(op: &Op) -> Result<Op, String> {
    let mut o = op.clone();
    let neg = |p: &[f64]| p.iter().map(|x| -x).collect::<Vec<_>>();
    match op.gate.as_str() {
        "s" => o.gate = "sdg".into(),
        "sdg" => o.gate = "s".into(),
        "t" => o.gate = "tdg".into(),
        "tdg" => o.gate = "t".into(),
        "sx" => o.gate = "sxdg".into(),
        "sxdg" => o.gate = "sx".into(),
        "rx" | "ry" | "rz" | "p" => o.params = neg(&op.params),
        "u3" => o.params = vec![-op.params[0], -op.params[2], -op.params[1]],
        "id" | "h" | "x" | "y" | "z" | "swap" | "barrier" => {}
        g => return Err(format!("no adjoint for `{g}`")),
    }
    Ok(o)
}

/// Format a basis index as a bit string `q[n-1] … q[0]`.
pub fn bitstring(i: usize, n: usize) -> String {
    (0..n).rev().map(|k| if (i >> k) & 1 == 1 { '1' } else { '0' }).collect()
}
