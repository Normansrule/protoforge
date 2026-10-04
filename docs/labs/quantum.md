# Quantum Lab

A quantum-information workbench that runs entirely in the browser: a
14-qubit state-vector simulator and a 6-qubit density-matrix simulator with
noise, Open Quantum Assembly Language (OpenQASM) 2.0 import/export,
OpenQASM 3.0 and Qiskit Python export and Quirk links, a library of protocols and algorithms, a quantum key distribution (QKD)
link designer and a quantum random number generator (QRNG) design bench with
National Institute of Standards and Technology (NIST) statistical tests. It
is written for a two-node fiber-optic quantum link and a diode-noise QRNG
board read by an analog-to-digital converter (ADC) into a field-programmable
gate array (FPGA), and it ships the **Q-Link** systems template, a full
Protoforge project whose requirements are verified by this lab.

| Piece | Location |
|---|---|
| Engine | `crates/forge-core/src/quantum/` — `circuit.rs` (model, gates), `statevec.rs`, `density.rs`, `linalg.rs`, `formats.rs` (OpenQASM, Quirk), `library.rs` (protocols, algorithms, CHSH), `qkd.rs`, `qrng.rs`, `nist.rs`, `mod.rs` (API, metrics) |
| Page | `web/src/labs/quantum/` — `index.tsx`, `Composer.tsx`, `Protocols.tsx`, `Algorithms.tsx`, `Noise.tsx`, `Qkd.tsx`, `Qrng.tsx`, `Learn.tsx`, `viz.tsx`, `grid.ts`, `types.ts`, `quantum.css` |
| Template | `qlink()` in `crates/forge-core/src/systems/templates.rs` |

## Tabs

* **Circuit composer.** Drag gates from the palette onto the wires (or tap a
  gate, then tap cells — works on phones). Palette: H, X, Y, Z, S, S†, T, T†,
  √X, RX/RY/RZ(θ), P(λ), U(θ, φ, λ), the two- and three-qubit tiles CNOT,
  CZ, Toffoli and SWAP (dropped on the tapped wire and the ones below it),
  control dot, SWAP half, iSWAP, measure, reset, barrier. As in Quirk,
  control dots control every gate in their column, so CY, controlled
  rotations, multi-controlled gates and Fredkin (dot + two SWAPs) are built
  from dots. Selecting a gate edits its
  angles (with π-fraction shortcuts), the classical bit a measurement writes
  and an optional classical condition ("only if c[k] = 1"). Presets load any
  library circuit. The state is shown after every column with a play / step /
  scrub bar: amplitude bars coloured by phase (colour wheel), one Bloch sphere
  per qubit (drag to rotate; vectors interpolate between steps), qubit
  entropies and a pairwise concurrence matrix. Shots produce a histogram with
  the exact Born-rule probabilities as ticks. Exports: OpenQASM 2.0
  (`QuantumCircuit.from_qasm_str` in Qiskit), OpenQASM 3.0 (`stdgates.inc`
  with `ctrl @` / `inv @` modifiers, `if (c == v)` feed-forward;
  `qiskit.qasm3.loads`), a runnable Qiskit script (`if_test` for classical
  conditions, Aer sampling), step-by-step instructions for pasting into the
  IBM Quantum Composer, an **Open in Quirk** link, amplitude and count CSVs.
  OpenQASM 2.0 can be imported back. The circuit is remembered in the browser.
* **Protocols.** Teleportation, superdense coding, entanglement swapping and
  the CHSH Bell test, each an animated story on Alice / Source-or-Charlie /
  Bob lanes: qubits fly down the quantum channel, classical bits travel as
  packets on the classical channel, and each scene shows the simulated state
  (Bloch spheres, fidelity, concurrence) and highlights the circuit columns
  executed so far. The CHSH view sweeps angles and source visibility and
  reports S with its sampling error against the local bound 2 and Tsirelson's
  2√2.
* **Algorithms.** Grover search (2–6 qubits, any marked item, any iteration
  count) with the distribution animated per iteration, the geometric rotation
  picture and simulated P(marked) against sin²((2k+1)θ); the quantum Fourier
  transform (QFT) with its phase-winding amplitudes; phase estimation;
  Deutsch–Jozsa and Bernstein–Vazirani with editable oracles.
* **Noise.** Depolarising, bit-flip, phase-flip, amplitude-damping (T₁),
  phase-damping (T₂) and readout errors on any library or composer circuit
  (≤ 6 qubits). Bloch vectors shrink column by column next to the ideal
  vectors, fidelity and purity vs depth, noisy vs ideal outcome probabilities,
  the density matrix (≤ 3 qubits), fidelity vs noise strength, and a
  single-qubit channel explorer that draws the image of the Bloch sphere
  (an ellipsoid r → M r + t) and the trajectory under repeated use.
* **QKD.** (a) BB84 photon stream: Alice's random bits and bases, an optional
  intercept-resend eavesdropper, loss and channel errors, Bob's bases, sifting,
  the quantum bit error rate (QBER) gauge with the 11 % threshold, and the
  error-correction leakage and privacy-amplification cost down to the final
  key. (b) Fiber link: decoy-state BB84 key rate vs distance, the
  no-decoy Gottesman–Lo–Lütkenhaus–Preskill (GLLP) rate and the
  Pirandola–Laurenza–Ottaviani–Banchi (PLOB) repeaterless bound on a log axis,
  QBER and optimal intensity vs distance, a photon and bit budget at the
  design distance (laser pulses → photons launched → photons at Bob →
  detections after the dead-time cap → sifted → after error-correction
  leakage → secret key, with the dB lost at each step; CSV download), a
  card listing every modelling assumption, and presets (gated indium gallium
  arsenide (InGaAs) avalanche photodiodes (APDs), superconducting nanowire
  single-photon detectors (SNSPDs), and the Gobby–Yuan–Shields (GYS)
  parameters of Lo, Ma & Chen). When the project has `design.extra.qkd` the
  sliders edit it and the requirements re-verify live. (c) E91 with Ekert's
  angles, the 3 × 3 correlation table, CHSH value, key QBER and an
  eavesdropper.
* **QRNG.** A reverse-biased-diode source (white avalanche noise with an
  optional amplifier bandwidth, 1/f drift, DC offset, mains pick-up, ADC
  resolution, full scale, sample rate and digitisation by comparator or the k
  least-significant bits (LSBs)) or a photon-at-a-beam-splitter source; an
  oscilloscope view, the code histogram against the Gaussian, the pipeline
  raw → von Neumann → XOR folding → Toeplitz extractor with rate, efficiency,
  most-common-value (MCV) min-entropy, a 64 × 64 bitmap, autocorrelation and
  the NIST SP 800-22 table (icon + word) per stage; downloads as binary or
  ASCII 0/1 (the format the NIST reference suite reads). Paste or open your
  own board's bitstream to test it.
* **How it works.** Qubits and the Bloch sphere, tensor products and
  entanglement, measurement, no-cloning, why BB84 is secure, key-rate formulas,
  entropy and randomness, with references and the tools to graduate to.

## Engine API (`quantum.*`)

| Method | Input → output |
|---|---|
| `quantum.gates` | → palette catalog (id, label, name, targets, parameter names, OpenQASM name, description) |
| `quantum.run` | `{circuit, shots, seed, steps}` → final snapshot, snapshots after every column (amplitudes, probabilities, per-qubit Bloch vector / purity / entropy, concurrence matrix for ≤ 6 qubits, classical bits, measurement events), shot histogram, exact probabilities, sampling mode, warnings |
| `quantum.qasm_parse` | `{qasm}` → circuit, registers, warnings (errors carry line numbers) |
| `quantum.qasm_emit` / `quantum.qasm3_emit` / `quantum.qiskit` / `quantum.quirk` | `{circuit}` → OpenQASM 2.0 / OpenQASM 3.0 / Qiskit Python text (+ warnings) / Quirk URL + JSON |
| `quantum.export` | `{circuit}` → all of the above at once (per-format errors instead of failing) |
| `quantum.library` / `quantum.algorithm` | → list; `{id, params}` → circuit, explanation, expected results (+ Grover per-iteration distributions) |
| `quantum.chsh` | `{a0, a1, b0, b1, visibility, shots, seed}` → E per setting (exact, sampled), S, its standard error, S vs angle |
| `quantum.noise` / `quantum.noise_sweep` / `quantum.channel` | density-matrix run with a noise model; fidelity vs strength; one-qubit channel as an affine Bloch map + trajectory |
| `quantum.bb84` / `quantum.qkd_link` / `quantum.e91` | BB84 Monte Carlo; decoy/GLLP/PLOB curves, design point and photon/bit budget (`preset: "gys"` available); E91 Monte Carlo |
| `quantum.qrng` / `quantum.nist` | source → digitise → condition pipeline with NIST results per stage; NIST tests on a user bitstream (`0101…` or `hex:…`) |
| `quantum.design` | `{project}` → the project's QKD/QRNG parameters and metrics |

Circuits are JSON: `{"qubits": 2, "clbits": 2, "cols": [[{"gate": "h", "targets": [0]}], [{"gate": "x", "controls": [0], "targets": [1]}], …]}`.
Aliases such as `cx`, `ccx`, `cz`, `cp`, `cswap`, `u1`, `u2`, `u` are accepted
and normalised. Bit order follows Qiskit: qubit k is bit k of the basis index
and bit strings print `q[n−1] … q[0]`.

## The maths

**State vector.** A gate U on target t with control mask m updates each pair
(i, j = i | 2ᵗ) with bit t of i clear and all control bits set:
`a'ᵢ = U₀₀aᵢ + U₀₁aⱼ`, `a'ⱼ = U₁₀aᵢ + U₁₁aⱼ`. Measurement samples
P(1) = Σ|aᵢ|² over indices with bit t set, collapses and renormalises. When
every measurement is terminal the shots are drawn from one final state;
otherwise each shot is re-simulated (capped at 40 M amplitude updates).

**Reduced states.** One qubit: ρ₀₀ = Σ|a_{i,0}|², ρ₀₁ = Σ a_{i,0} a*_{i,1};
Bloch vector (2 Re ρ₀₁, −2 Im ρ₀₁, ρ₀₀ − ρ₁₁); purity (1 + |r|²)/2; entropy
h((1 + |r|)/2). Two qubits: the 4 × 4 reduced matrix and Wootters'
concurrence C = max(0, λ₁ − λ₂ − λ₃ − λ₄), λ the square roots of the
eigenvalues of √ρ ρ̃ √ρ with ρ̃ = (Y⊗Y)ρ*(Y⊗Y). Hermitian eigenvalues and
matrix square roots use the real symmetric embedding [[A, −B], [B, A]] and
cyclic Jacobi, which is robust for the repeated eigenvalues of pure states.

**Density matrix.** ρ is stored vectorised (element (r, c) at r + c·2ⁿ), so
UρU† is U on the row qubits and U* on the column qubits, and Kraus maps
Σ K ρ K† reuse the same kernels. Channels (Nielsen & Chuang §8.3):
depolarising (1−p)ρ + p I/2, bit flip, phase flip, amplitude damping
K₀ = [[1,0],[0,√(1−γ)]], K₁ = [[0,√γ],[0,0]], phase damping
K₀ = [[1,0],[0,√(1−λ)]], K₁ = [[0,0],[0,√λ]], applied after every column to
each qubit a gate touched (optionally the damping channels to idle qubits too);
readout error is a classical confusion per bit. Fidelity is ⟨ψ|ρ|ψ⟩ against
the ideal pure state, or Uhlmann's (Tr √(√ρ σ √ρ))² after mid-circuit
measurements (≤ 4 qubits).

**BB84.** Channel flip probability p, intercept-resend on a fraction e:
Q = (1 − e)p + e(p/2 + 1/4). Secret fraction (Shor & Preskill 2000) with
error-correction inefficiency f: r = 1 − f·h(Q) − h(Q), zero at 11.0 % for
f = 1.

**Decoy-state link** (Lo, Ma & Chen 2005, asymptotic, infinite decoys):

```text
η   = η_det · 10^(−(αL + L_fixed)/10)
Y₀  = 2 p_dark                (two detectors)
Q_μ = Y₀ + 1 − e^(−ημ)        E_μ = [Y₀/2 + e_d (1 − e^(−ημ))] / Q_μ
Y₁  = Y₀ + η                  Q₁ = Y₁ μ e^(−μ)        e₁ = (Y₀/2 + e_d η) / Y₁
R   = q { −Q_μ f h(E_μ) + Q₁ [1 − h(e₁)] }   bits per pulse, × pulse rate
```

μ is optimised per distance (golden-section search) unless fixed. Without
decoys (GLLP): Δ = (1 − (1+μ)e^(−μ))/Q_μ,
R = q{−Q_μ f h(E_μ) + Q_μ(1 − Δ)[1 − h(E_μ/(1 − Δ))]}. Detector dead time τ
caps the detection rate at a non-paralysable D/(1 + Dτ) (applied to the decoy and GLLP curves alike). PLOB:
R ≤ −log₂(1 − η_fiber) bits per channel use. The maximum distance is a
bisection on R > 0.

**E91.** Alice 0°, 45°, 90°; Bob 45°, 90°, 135°; Werner visibility V gives
E(a, b) = V cos(a − b); key from (45°, 45°) and (90°, 90°); S from the
0°/90° × 45°/135° settings. Intercept-resend on Bob's photon along one of his
angles replaces E with V·avg cos(a − e) cos(b − e).

**QRNG.** Photon source: P(1) = tη₁ / (tη₁ + (1 − t)η₀). Diode: white noise
(optionally a first-order low-pass at the amplifier bandwidth), 1/f noise as
16 octave-spaced first-order processes of equal power, offset, mains sine,
quantised with LSB = V_pp / 2ᴺ and clipped at the rails. Ideal min-entropy of
the full code for a Gaussian of rms σ: H∞ ≈ log₂(σ√(2π)/LSB). von Neumann:
01 → 0, 10 → 1 (yield 2p(1 − p) per pair). XOR folding of k bits:
bias ε → 2^(k−1) εᵏ. Toeplitz extractor: y = T x over GF(2) with
T[i][j] = s[i − j + m − 1], l = ⌊m·h − 2 log₂(1/ε)⌋ output bits per m-bit
block, h = the MCV min-entropy per input bit.

**NIST SP 800-22 subset** (α = 0.01): frequency (monobit) §2.1, frequency
within a block §2.2, runs §2.3 (fails when the frequency prerequisite fails),
longest run of ones §2.4 (block sizes 8 / 128 / 10⁴ by length), serial §2.11
(two p-values), approximate entropy §2.12 and cumulative sums §2.13
(forward and backward p-values, both must pass), with block sizes chosen from the
document's recommendations. Special functions: Lanczos ln Γ, the regularised
upper incomplete gamma by series / continued fraction, erfc(x) = Q(½, x²).
**SP 800-90B** most-common-value estimate: p_u = min(1, p̂ + 2.576
√(p̂(1 − p̂)/(L − 1))), H = −log₂ p_u.

## Verification (unit tests)

`cargo test -p forge-core quantum` covers: HXH = Z, S² = Z, √X² = X, U(π/2, 0, π) = H;
the CNOT, Toffoli and Fredkin truth tables; iSWAP and its OpenQASM
decomposition (exact up to global phase); Bell-state probabilities, entropy 1
and concurrence 1; collapse and mid-circuit reset; teleportation fidelity 1
for every outcome and in deferred form; superdense coding for all four
messages; entanglement swapping giving C(q0, q3) = 1; Grover's simulated
P(marked) = sin²((2k+1)θ) for n = 2…5 with the optimal k = 1, 2, 3, 4; QFT =
DFT on a random 4-qubit state; exact phase estimation; Deutsch–Jozsa and
Bernstein–Vazirani; CHSH S = 2√2 and no violation at V = 0.6; Hermitian square
roots, pure-state fidelity and Werner-state concurrence (3p − 1)/2;
depolarising / amplitude-damping / phase-flip Bloch maps and readout-error
mixing; OpenQASM round trip, gate definitions, expressions and line-numbered
errors; OpenQASM 3.0 and Qiskit output for feed-forward, Toffoli,
`ctrl(k) @` and partial-register conditions; Quirk ids; BB84 intercept-resend QBER ≈ 25 %, sifting ≈ ½, the 11.0 %
Shor–Preskill threshold; the decoy-state rate against a hand calculation at
0 km, monotonic decrease, zero beyond the cutoff, decoy ≥ GLLP and ≤ PLOB;
the GYS parameter set reaching ≈ 140 km as Lo, Ma & Chen report; the PLOB
formula at 50 km and its 1.44η limit; E91 with and without Eve; the NIST
SP 800-22 worked examples (monobit 0.527089 and 0.109599, block frequency
0.801252 and 0.706438, runs 0.147232 and 0.500798, longest run 0.180598,
serial 0.808792 / 0.670320, approximate entropy 0.261961, cumulative sums
0.4116588 and, on the π bits, 0.219194 forward / 0.114866 backward); MCV on biased bits;
von Neumann output unbiased with yield 2p(1 − p); the XOR piling-up lemma; the
Toeplitz extractor's linearity and a brute-force row check; and the default
diode pipeline passing all tests after extraction with the Gaussian
min-entropy the theory predicts.

## Metrics and the Q-Link template

Metrics appear only when a project describes a quantum link or a QRNG, so the
Iris smart glasses produce none.

| Key | Unit | From |
|---|---|---|
| `quantum.qkd_key_rate_bps` | bit/s | decoy-state rate at `design.extra.qkd.distance_km` |
| `quantum.qkd_qber_pct` | % | E_μ at the design distance |
| `quantum.qkd_max_distance_km` | km | largest distance with R > 0 |
| `quantum.qkd_link_loss_db` | dB | αL + fixed loss |
| `quantum.qkd_plob_fraction_pct` | % | key rate ÷ PLOB bound at the design distance |
| `quantum.qrng_min_entropy_bits` | bit | MCV per digitised sample (`design.extra.qrng`) |
| `quantum.qrng_output_rate_mbps` | Mbit/s | sample rate × bits per sample × extractor efficiency |
| `quantum.qrng_nist_failures` | count | failed SP 800-22 tests on the conditioned stream |
| `quantum.qrng_supply_ratio` | × | QRNG output ÷ (random bits per pulse × pulse rate), when both exist |

**Q-Link — two-node fiber QKD testbed** (`systems.templates`, id `qlink`):
photon source, encoder, QRNG, 25 km fiber channel, gated InGaAs detectors,
timing and synchronisation, classical post-processing and key management;
10 interfaces, 17 components with power, three trade studies (detector
technology, time-bin vs polarisation encoding, source and protocol), a seven-line
failure mode and effects analysis (FMEA) including detector blinding and QRNG
failure, the standard milestones and 12 requirements. With the default values
(0.2 dB/km, 3 dB fixed loss, 20 % detector efficiency, 10⁻⁶ dark counts,
1.5 % misalignment, f = 1.16, 10 MHz, 1 µs dead time; an 8-bit, 20 MS/s ADC
keeping 4 LSBs) the lab computes ≈ 28 kbit/s at 25 km, 1.5 % QBER, ≈ 175 km
maximum distance, 8 dB loss, ≈ 3.8 bits of min-entropy per sample,
≈ 67 Mbit/s of conditioned random bits (2.2 × what the transmitter needs), all
NIST tests passed and ≈ 109 W: eight requirements pass, three (R-010 to R-012,
verified by test or demonstration) stay unverified until a result is recorded,
and **R-005** — the goal of reaching 1 % of the PLOB bound — is open (≈ 0.52 %). It is left open on purpose:
switching the detector efficiency to an SNSPD-class 0.8 (trade T-1) meets it —
the decision the template leaves for the next design review. Component prices
are planning placeholders, not quotes; device numbers are typical published
ranges to replace with datasheet values.

## Limitations

* The simulators are exact but dense: 14 qubits (state vector) and 6 qubits
  (density matrix) are the practical limits in a browser tab.
* Noise is a per-gate Kraus model, not a pulse-level or crosstalk model;
  there is no leakage, no correlated noise and no gate-duration scheduling.
* OpenQASM 3.0 and Qiskit are export-only (import reads OpenQASM 2.0);
  `opaque` gates, relative-phase Toffolis (`rccx`, `rc3x`) and
  conditional resets are not supported. Quirk export has no reset and maps
  only single-bit classical conditions onto measured wires.
* The key-rate model is asymptotic (no finite-key effects) with infinitely
  many decoys, symmetric detectors and a simple dead-time cap; afterpulsing,
  detector-efficiency mismatch and side channels are not modelled.
* The QRNG model is a behavioural noise model, not a device model of a
  specific diode or amplifier. The NIST suite is a subset (no FFT,
  non-overlapping template, Maurer, linear complexity or random-excursion
  tests), and only the MCV estimator of SP 800-90B is implemented; passing
  statistical tests does not prove a source is random.

## Graduate to

Qiskit (and the IBM Quantum Composer), Cirq, QuTiP for open-system physics,
Quirk for quick visual experiments, and NetSquid or SeQUeNCe for repeater
chains and network protocols. Free English resources: John Preskill's Ph219
notes (http://theory.caltech.edu/~preskill/ph219/), the Qiskit Textbook /
IBM Quantum Learning, MIT OpenCourseWare (https://ocw.mit.edu: 18.435J
Quantum Computation, 8.04 Quantum Physics I video lectures), Michael
Nielsen's "Quantum computing for the determined" videos, and Nielsen &
Chuang. Before building real link hardware, read Scarani et al. (2009).

## References

* M. A. Nielsen & I. L. Chuang, *Quantum Computation and Quantum Information*, 10th anniversary ed., Cambridge University Press, 2010.
* J. Preskill, Caltech Ph219 lecture notes, http://theory.caltech.edu/~preskill/ph219/
* C. H. Bennett & G. Brassard, "Quantum cryptography: Public key distribution and coin tossing", Proc. IEEE Int. Conf. Computers, Systems and Signal Processing, 1984.
* A. K. Ekert, "Quantum cryptography based on Bell's theorem", Phys. Rev. Lett. 67, 661, 1991.
* J. F. Clauser, M. A. Horne, A. Shimony & R. A. Holt, "Proposed experiment to test local hidden-variable theories", Phys. Rev. Lett. 23, 880, 1969.
* C. H. Bennett, G. Brassard, C. Crépeau, R. Jozsa, A. Peres & W. K. Wootters, "Teleporting an unknown quantum state via dual classical and Einstein–Podolsky–Rosen channels", Phys. Rev. Lett. 70, 1895, 1993.
* C. H. Bennett & S. J. Wiesner, "Communication via one- and two-particle operators on Einstein–Podolsky–Rosen states", Phys. Rev. Lett. 69, 2881, 1992.
* M. Żukowski, A. Zeilinger, M. A. Horne & A. K. Ekert, "'Event-ready-detectors' Bell experiment via entanglement swapping", Phys. Rev. Lett. 71, 4287, 1993.
* L. K. Grover, "A fast quantum mechanical algorithm for database search", Proc. 28th ACM STOC, 1996.
* P. W. Shor, "Algorithms for quantum computation: discrete logarithms and factoring", FOCS 1994; SIAM J. Comput. 26, 1484, 1997.
* D. Deutsch & R. Jozsa, "Rapid solution of problems by quantum computation", Proc. R. Soc. Lond. A 439, 553, 1992.
* E. Bernstein & U. Vazirani, "Quantum complexity theory", SIAM J. Comput. 26, 1411, 1997.
* W. K. Wootters, "Entanglement of formation of an arbitrary state of two qubits", Phys. Rev. Lett. 80, 2245, 1998.
* W. K. Wootters & W. H. Zurek, "A single quantum cannot be cloned", Nature 299, 802, 1982.
* P. W. Shor & J. Preskill, "Simple proof of security of the BB84 quantum key distribution protocol", Phys. Rev. Lett. 85, 441, 2000.
* H.-K. Lo, X. Ma & K. Chen, "Decoy state quantum key distribution", Phys. Rev. Lett. 94, 230504, 2005.
* D. Gottesman, H.-K. Lo, N. Lütkenhaus & J. Preskill, "Security of quantum key distribution with imperfect devices", Quantum Inf. Comput. 4, 325, 2004.
* S. Pirandola, R. Laurenza, C. Ottaviani & L. Banchi, "Fundamental limits of repeaterless quantum communications", Nat. Commun. 8, 15043, 2017.
* N. Gisin, G. Ribordy, W. Tittel & H. Zbinden, "Quantum cryptography", Rev. Mod. Phys. 74, 145, 2002.
* M. Herrero-Collantes & J. C. Garcia-Escartin, "Quantum random number generators", Rev. Mod. Phys. 89, 015004, 2017.
* X. Ma, F. Xu, H. Xu, X. Tan, B. Qi & H.-K. Lo, "Postprocessing for quantum random-number generators: Entropy evaluation and randomness extraction", Phys. Rev. A 87, 062327, 2013.
* A. Rukhin et al., NIST SP 800-22 Rev. 1a, *A Statistical Test Suite for Random and Pseudorandom Number Generators for Cryptographic Applications*, 2010.
* M. S. Turan et al., NIST SP 800-90B, *Recommendation for the Entropy Sources Used for Random Bit Generation*, 2018.
* A. W. Cross, L. S. Bishop, J. A. Smolin & J. M. Gambetta, "Open Quantum Assembly Language", arXiv:1707.03429, 2017.
* A. W. Cross, A. Javadi-Abhari, T. Alexander, N. de Beaudrap, L. S. Bishop, S. Heidel, C. A. Ryan, P. Sivarajah, J. Smolin, J. M. Gambetta & B. R. Johnson, "OpenQASM 3: A broader and deeper quantum assembly language", ACM Trans. Quantum Comput. 3, 12, 2022 (arXiv:2104.14722).
* V. Scarani, H. Bechmann-Pasquinucci, N. J. Cerf, M. Dušek, N. Lütkenhaus & M. Peev, "The security of practical quantum key distribution", Rev. Mod. Phys. 81, 1301, 2009.
* C. Gidney, Quirk, https://algassert.com/quirk (source: github.com/Strilanc/Quirk).
* Qiskit, https://github.com/Qiskit/qiskit · Cirq, https://github.com/quantumlib/Cirq · QuTiP, https://qutip.org
* W. H. Press, S. A. Teukolsky, W. T. Vetterling & B. P. Flannery, *Numerical Recipes*, 3rd ed., Cambridge, 2007 (incomplete gamma function).
