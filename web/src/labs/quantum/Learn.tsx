// "How it works": the ideas behind every tab, with equations and references.
import { Link } from 'react-router-dom';
import { Card, Explainer, Formula } from '../../components/ui';
import { BlochSphere } from './viz';

export const REFS = {
  nc: { label: 'M. A. Nielsen & I. L. Chuang, Quantum Computation and Quantum Information, 10th anniversary ed. (Cambridge, 2010)' },
  preskill: { label: 'J. Preskill, Caltech Ph219 lecture notes', href: 'http://theory.caltech.edu/~preskill/ph219/' },
  bb84: { label: 'C. H. Bennett & G. Brassard, "Quantum cryptography: Public key distribution and coin tossing" (1984)' },
  ekert: { label: 'A. K. Ekert, "Quantum cryptography based on Bell\'s theorem", PRL 67, 661 (1991)' },
  chsh: { label: 'J. F. Clauser, M. A. Horne, A. Shimony & R. A. Holt, PRL 23, 880 (1969)' },
  teleport: { label: 'C. H. Bennett et al., "Teleporting an unknown quantum state via dual classical and Einstein–Podolsky–Rosen channels", PRL 70, 1895 (1993)' },
  grover: { label: 'L. K. Grover, "A fast quantum mechanical algorithm for database search", STOC (1996)' },
  shor: { label: 'P. W. Shor, "Algorithms for quantum computation: discrete logarithms and factoring" (1994); SIAM J. Comput. 26, 1484 (1997)' },
  lmc: { label: 'H.-K. Lo, X. Ma & K. Chen, "Decoy state quantum key distribution", PRL 94, 230504 (2005)' },
  gllp: { label: 'D. Gottesman, H.-K. Lo, N. Lütkenhaus & J. Preskill, "Security of quantum key distribution with imperfect devices", Quantum Inf. Comput. 4, 325 (2004)' },
  sp: { label: 'P. W. Shor & J. Preskill, "Simple proof of security of the BB84 quantum key distribution protocol", PRL 85, 441 (2000)' },
  plob: { label: 'S. Pirandola, R. Laurenza, C. Ottaviani & L. Banchi, "Fundamental limits of repeaterless quantum communications", Nat. Commun. 8, 15043 (2017)' },
  gisin: { label: 'N. Gisin, G. Ribordy, W. Tittel & H. Zbinden, "Quantum cryptography", Rev. Mod. Phys. 74, 145 (2002)' },
  qrng: { label: 'M. Herrero-Collantes & J. C. Garcia-Escartin, "Quantum random number generators", Rev. Mod. Phys. 89, 015004 (2017)' },
  sp80022: { label: 'NIST SP 800-22 Rev. 1a, A Statistical Test Suite for Random and Pseudorandom Number Generators (2010)' },
  sp80090b: { label: 'NIST SP 800-90B, Recommendation for the Entropy Sources Used for Random Bit Generation (2018)' },
  wootters: { label: 'W. K. Wootters, "Entanglement of formation of an arbitrary state of two qubits", PRL 80, 2245 (1998)' },
  qasm: { label: 'A. W. Cross, L. S. Bishop, J. A. Smolin & J. M. Gambetta, "Open Quantum Assembly Language", arXiv:1707.03429 (2017)', href: 'https://arxiv.org/abs/1707.03429' },
  quirk: { label: 'C. Gidney, Quirk — in-browser quantum circuit simulator', href: 'https://algassert.com/quirk' },
  qiskit: { label: 'Qiskit', href: 'https://github.com/Qiskit/qiskit' },
  cirq: { label: 'Cirq', href: 'https://github.com/quantumlib/Cirq' },
  qutip: { label: 'QuTiP', href: 'https://qutip.org' },
};

export default function Learn() {
  const s = Math.SQRT1_2;
  return (
    <div className="stack qx-learn">
      <div className="grid grid-2">
        <Explainer title="A qubit and its Bloch sphere" refs={[REFS.nc, REFS.preskill]}>
          A qubit is a unit vector |ψ⟩ = α|0⟩ + β|1⟩ with |α|² + |β|² = 1. Dropping the unobservable global phase leaves two angles:
          <Formula>{'|ψ⟩ = cos(θ/2)|0⟩ + e^{iφ} sin(θ/2)|1⟩   →   Bloch vector (sin θ cos φ, sin θ sin φ, cos θ)'}</Formula>
          Every single-qubit gate is a rotation of this sphere: X, Y, Z are half-turns about their axes, H is a half-turn about (X + Z)/√2, S and T are quarter- and eighth-turns about Z.
          <div className="row" style={{ justifyContent: 'center', marginTop: 8 }}>
            <BlochSphere v={[0, 0, 1]} size={110} label="|0⟩" />
            <BlochSphere v={[1, 0, 0]} size={110} label="|+⟩ = H|0⟩" />
            <BlochSphere v={[s, s, 0]} size={110} label="T|+⟩" />
          </div>
        </Explainer>
        <Explainer title="Many qubits: tensor products and entanglement" refs={[REFS.nc, REFS.wootters]}>
          n qubits live in the tensor product of n two-dimensional spaces: 2ⁿ amplitudes, one per bit string. A product state factorises, |a⟩⊗|b⟩; an entangled one does not:
          <Formula>{'|Φ+⟩ = (|00⟩ + |11⟩)/√2  ≠  (α|0⟩ + β|1⟩) ⊗ (γ|0⟩ + δ|1⟩)'}</Formula>
          Tracing out the partner leaves each qubit in the reduced state ρ = I/2 — a Bloch vector of length 0 and one full bit of entropy S = −Tr ρ log₂ ρ. For two qubits the Wootters concurrence C = 2|α₀₀α₁₁ − α₀₁α₁₀| (pure states) measures the entanglement from 0 to 1.
        </Explainer>
        <Explainer title="Measurement and the Born rule" refs={[REFS.nc]}>
          Measuring qubit k in the computational basis gives 1 with probability P(1) = Σ|αᵢ|² over basis states with bit k = 1. The state then collapses onto the observed half and is renormalised. Repeating a circuit (shots) estimates the probabilities with a standard error √(P(1 − P)/N).
          <Formula>{'P(outcome m) = ⟨ψ| Πₘ |ψ⟩        |ψ⟩ → Πₘ|ψ⟩ / √P(m)'}</Formula>
          Measuring along another axis = rotate first, then measure Z — how the CHSH tab measures at angles θ.
        </Explainer>
        <Explainer title="No-cloning" refs={[{ label: 'W. K. Wootters & W. H. Zurek, "A single quantum cannot be cloned", Nature 299, 802 (1982)' }]}>
          No device can copy an unknown state: if U|ψ⟩|0⟩ = |ψ⟩|ψ⟩ and U|φ⟩|0⟩ = |φ⟩|φ⟩, taking inner products gives ⟨ψ|φ⟩ = ⟨ψ|φ⟩², so the two states are identical or orthogonal. This is why teleportation must destroy the original, why a quantum signal cannot be amplified like a classical one (hence repeaters, not amplifiers), and why an eavesdropper cannot keep a perfect copy of a QKD photon.
        </Explainer>
        <Explainer title="Why BB84 is secure" refs={[REFS.bb84, REFS.sp, REFS.gisin]}>
          Alice sends each bit in one of two conjugate bases chosen at random. Eve does not know the basis; measuring in the wrong one randomises the result, so an intercept-resend attack on every photon causes a 25 % error rate on the sifted key. More generally, information gained by Eve is bounded by the observed error rate: after error correction (leaking f·h(Q) bits) and privacy amplification (removing h(Q) bits), the remaining fraction
          <Formula>{'r = 1 − f·h(Q) − h(Q),    h(p) = −p log₂ p − (1−p) log₂(1−p)'}</Formula>
          is secret. With f = 1 it hits zero at Q ≈ 11.0 % (Shor & Preskill 2000). Security rests on the protocol's model of the devices — real attacks (e.g. detector blinding) target implementation gaps, which the Qlink template lists in its FMEA.
        </Explainer>
        <Explainer title="Key-rate formulas for a fiber link" refs={[REFS.lmc, REFS.gllp, REFS.plob]}>
          Weak laser pulses contain Poisson-distributed photon numbers with mean μ. Decoy intensities reveal the single-photon yield, giving the Lo–Ma–Chen rate
          <Formula>{'R = q { −Q_μ f h(E_μ) + Q₁ [1 − h(e₁)] }   with   Q_μ ≈ 1 − e^(−ημ),  Q₁ ≈ η μ e^(−μ)'}</Formula>
          so R ∝ η = η_det·10^(−αL/10) at short range and collapses where dark counts make E_μ approach 11 %. The PLOB bound −log₂(1 − η) ≈ 1.44 η shows every repeaterless link decays exponentially with distance.
        </Explainer>
        <Explainer title="Entropy and randomness" refs={[REFS.qrng, REFS.sp80090b, REFS.sp80022]}>
          Shannon entropy averages surprise; cryptography needs the worst case — min-entropy H∞ = −log₂ maxₓ P(x), the number of nearly uniform bits an extractor can distil. A quantum source (a photon at a beam splitter, shot and avalanche noise) is unpredictable in principle; classical noise and drift mixed into it must not be credited. SP 800-90B estimates H∞ from data (the most-common-value estimator here); SP 800-22 tests the output for statistical defects.
        </Explainer>
        <Explainer title="Simulating it" refs={[REFS.qasm, REFS.quirk]}>
          The state-vector simulator stores 2ⁿ amplitudes (≤ 14 qubits); the density-matrix simulator stores 4ⁿ entries (≤ 6 qubits) and applies Kraus maps. Everything is Rust compiled to WebAssembly, runs in your browser, is seeded and deterministic, and is unit-tested against textbook identities (HXH = Z, CNOT truth table, QFT = DFT, Grover's sin²((2k+1)θ), CHSH = 2√2, NIST SP 800-22 worked examples).
        </Explainer>
      </div>

      <div className="grid grid-2">
        <Card title="Graduate to the professional tools">
          <ul className="qx-reflist">
            <li><a href={REFS.qiskit.href} target="_blank" rel="noreferrer">Qiskit</a> — IBM's open-source SDK: circuits, transpiler, simulators (Aer) and real hardware. Loads the OpenQASM this lab exports.</li>
            <li><a href={REFS.cirq.href} target="_blank" rel="noreferrer">Cirq</a> — Google's circuit framework, strong on noise models and hardware-native gates.</li>
            <li><a href={REFS.qutip.href} target="_blank" rel="noreferrer">QuTiP</a> — open quantum systems: master equations, T₁/T₂ physics, pulse-level dynamics.</li>
            <li><b>NetSquid</b> and <b>SeQUeNCe</b> — discrete-event quantum-network simulators for repeater chains, memories and link-layer protocols: the next step after this lab's two-node link.</li>
            <li><a href={REFS.quirk.href} target="_blank" rel="noreferrer">Quirk</a> and the <b>IBM Quantum Composer</b> — drag-and-drop circuits in the browser; this lab exports to both.</li>
          </ul>
        </Card>
        <Card title="Free learning resources (English)">
          <ul className="qx-reflist">
            <li><a href={REFS.preskill.href} target="_blank" rel="noreferrer">John Preskill's Ph219 lecture notes</a> (Caltech) — the classic free course notes on quantum information and computation.</li>
            <li><b>Qiskit Textbook / IBM Quantum Learning</b> — free interactive chapters from qubits to Grover, QFT and QKD, with runnable Qiskit code.</li>
            <li><b>Nielsen & Chuang</b> — the standard textbook every chapter here cites; chapters 1–2 are a complete introduction.</li>
            <li>Michael Nielsen's <b>“Quantum computing for the determined”</b> — a free English video series on the basics.</li>
          </ul>
          <div className="callout small" style={{ marginTop: 10 }}>
            <b>Iris and the quantum lab.</b> The smart glasses need no quantum hardware, so Iris has no quantum requirements. The <Link to="/systems">Systems Studio</Link>'s <b>Qlink</b> template is the quantum product: a two-node fiber QKD link with a diode-noise QRNG whose 12 requirements are verified live by the QKD and QRNG tabs.
          </div>
        </Card>
      </div>

      <Card title="References">
        <ol className="qx-reflist">
          {Object.values(REFS).map((r) => (
            <li key={r.label}>{'href' in r && r.href ? <a href={r.href} target="_blank" rel="noreferrer">{r.label}</a> : r.label}</li>
          ))}
        </ol>
      </Card>
    </div>
  );
}
