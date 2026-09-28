//! Quantum Lab engine: circuits, protocols, quantum key distribution (QKD)
//! and quantum random number generation (QRNG).
//!
//! | File | What |
//! |---|---|
//! | [`circuit`] | circuit model (columns of ops, arbitrary controls, classical conditions), gate matrices |
//! | [`statevec`] | state-vector simulator ≤ 14 qubits: shots, step-through, Bloch vectors, entanglement |
//! | [`density`] | density-matrix simulator ≤ 6 qubits with noise channels and fidelity |
//! | [`linalg`] | Hermitian eigenvalues, Uhlmann fidelity, von Neumann entropy, Wootters concurrence |
//! | [`formats`] | OpenQASM 2.0 reader/writer, OpenQASM 3.0 and Qiskit Python writers, Quirk links |
//! | [`library`] | Bell/GHZ, teleportation, superdense coding, entanglement swapping, CHSH, Deutsch–Jozsa, Bernstein–Vazirani, Grover, QFT, phase estimation |
//! | [`qkd`] | BB84 Monte Carlo, decoy-state fiber link model vs the PLOB bound, E91 |
//! | [`qrng`] | photon and diode-noise entropy sources, debiasing, Toeplitz extraction |
//! | [`nist`] | NIST SP 800-22 tests and the SP 800-90B most-common-value estimate |
//!
//! Metrics are produced only for projects that describe a quantum link
//! (`design.extra.qkd`) or a QRNG (`design.extra.qrng`) — see the `qlink`
//! template in `systems::templates`. Iris has neither.

pub mod circuit;
pub mod density;
pub mod formats;
pub mod library;
pub mod linalg;
pub mod nist;
pub mod qkd;
pub mod qrng;
pub mod statevec;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::api::{handle, ApiResult};
use crate::memo::{self, Slot};
use crate::project::{Metric, Project};
use circuit::Circuit;

#[derive(Deserialize)]
struct RunIn {
    circuit: Circuit,
    #[serde(default = "default_shots")]
    shots: usize,
    #[serde(default = "one")]
    seed: u64,
    #[serde(default)]
    steps: bool,
}

fn default_shots() -> usize {
    1024
}
fn one() -> u64 {
    1
}

#[derive(Deserialize)]
struct QasmIn {
    qasm: String,
}

#[derive(Deserialize)]
struct CircIn {
    circuit: Circuit,
}

#[derive(Serialize)]
struct ExportOut {
    qasm: Option<String>,
    qasm_error: Option<String>,
    qasm3: Option<String>,
    qasm3_error: Option<String>,
    qiskit: Option<String>,
    quirk_url: Option<String>,
    quirk_json: Option<String>,
    warnings: Vec<String>,
}

fn export(c: &Circuit) -> ExportOut {
    let mut warnings = vec![];
    let (qasm, qasm_error) = match formats::emit_qasm(c) {
        Ok(e) => {
            warnings.extend(e.warnings);
            (Some(e.qasm), None)
        }
        Err(e) => (None, Some(e)),
    };
    let (qasm3, qasm3_error) = match formats::emit_qasm3(c) {
        Ok(e) => (Some(e.qasm), None),
        Err(e) => (None, Some(e)),
    };
    let qiskit = formats::emit_qiskit(c).ok().map(|e| e.qasm);
    let (quirk_url, quirk_json) = match formats::quirk(c) {
        Ok(q) => {
            warnings.extend(q.warnings);
            (Some(q.url), Some(q.json))
        }
        Err(e) => {
            warnings.push(e);
            (None, None)
        }
    };
    ExportOut {
        qasm,
        qasm_error,
        qasm3,
        qasm3_error,
        qiskit,
        quirk_url,
        quirk_json,
        warnings,
    }
}

#[derive(Deserialize)]
struct AlgoIn {
    id: String,
    #[serde(default)]
    params: library::Params,
}

#[derive(Deserialize)]
struct NoiseIn {
    circuit: Circuit,
    #[serde(default)]
    noise: density::Noise,
}

#[derive(Deserialize)]
struct SweepIn {
    circuit: Circuit,
    channel: String,
    #[serde(default = "half")]
    pmax: f64,
    #[serde(default = "twenty")]
    points: usize,
}

fn half() -> f64 {
    0.5
}
fn twenty() -> usize {
    21
}

#[derive(Deserialize)]
struct ChannelIn {
    #[serde(default)]
    noise: density::Noise,
    #[serde(default = "north")]
    start: [f64; 3],
    #[serde(default = "ten")]
    steps: usize,
}

fn north() -> [f64; 3] {
    [1.0, 0.0, 0.0]
}
fn ten() -> usize {
    10
}

#[derive(Deserialize)]
struct LinkIn {
    #[serde(default)]
    params: Option<qkd::LinkParams>,
    #[serde(default)]
    preset: Option<String>,
    #[serde(default)]
    max_km: f64,
    #[serde(default = "points")]
    points: usize,
}

fn points() -> usize {
    121
}

#[derive(Deserialize)]
struct NistIn {
    bits: String,
}

#[derive(Serialize)]
struct NistOut {
    n: usize,
    ones_fraction: f64,
    tests: Vec<nist::TestResult>,
    min_entropy_per_bit: f64,
    min_entropy_per_byte: Option<f64>,
    autocorr: Vec<f64>,
}

#[derive(Deserialize)]
struct DesignIn {
    project: Project,
}

/// JSON API entry point for `quantum.*` methods.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "quantum.gates" => Some(crate::api::to_value(circuit::catalog())),
        "quantum.run" => handle(input, |i: RunIn| statevec::run(&i.circuit, i.shots, i.seed, i.steps)),
        "quantum.qasm_parse" => handle(input, |i: QasmIn| formats::parse_qasm(&i.qasm)),
        "quantum.qasm_emit" => handle(input, |i: CircIn| formats::emit_qasm(&i.circuit)),
        "quantum.qasm3_emit" => handle(input, |i: CircIn| formats::emit_qasm3(&i.circuit)),
        "quantum.qiskit" => handle(input, |i: CircIn| formats::emit_qiskit(&i.circuit)),
        "quantum.quirk" => handle(input, |i: CircIn| formats::quirk(&i.circuit)),
        "quantum.export" => handle(input, |i: CircIn| Ok(export(&i.circuit))),
        "quantum.library" => Some(crate::api::to_value(library::list())),
        "quantum.algorithm" => handle(input, |i: AlgoIn| library::build(&i.id, &i.params)),
        "quantum.chsh" => handle(input, |i: library::ChshIn| Ok(library::chsh(&i))),
        "quantum.noise" => handle(input, |i: NoiseIn| density::run_noisy(&i.circuit, &i.noise)),
        "quantum.noise_sweep" => handle(input, |i: SweepIn| density::sweep(&i.circuit, &i.channel, i.pmax, i.points, false)),
        "quantum.channel" => handle(input, |i: ChannelIn| Ok(density::channel_map(&i.noise, i.start, i.steps))),
        "quantum.bb84" => handle(input, |i: qkd::Bb84In| Ok(qkd::bb84(&i))),
        "quantum.qkd_link" => handle(input, |i: LinkIn| {
            let p = match (i.preset.as_deref(), i.params) {
                (Some("gys"), _) => qkd::LinkParams::gys(),
                (_, Some(p)) => p,
                _ => qkd::LinkParams::default(),
            };
            Ok(qkd::link(&p, i.max_km, i.points))
        }),
        "quantum.e91" => handle(input, |i: qkd::E91In| Ok(qkd::e91(&i))),
        "quantum.qrng" => handle(input, |i: qrng::QrngIn| Ok(qrng::simulate(&i))),
        "quantum.nist" => handle(input, |i: NistIn| {
            let bits = nist::parse_bits(&i.bits)?;
            if bits.len() < 100 {
                return Err(format!(
                    "{} bits: the tests need at least 100 (SP 800-22 recommends ≥ 10⁶)",
                    bits.len()
                ));
            }
            let syms: Vec<u32> = bits.iter().map(|&b| b as u32).collect();
            let bytes: Vec<u32> = bits
                .chunks_exact(8)
                .map(|c| c.iter().fold(0u32, |a, &b| (a << 1) | b as u32))
                .collect();
            Ok(NistOut {
                n: bits.len(),
                ones_fraction: bits.iter().map(|&b| b as f64).sum::<f64>() / bits.len() as f64,
                tests: nist::suite(&bits),
                min_entropy_per_bit: nist::mcv_min_entropy(&syms).0,
                min_entropy_per_byte: (bytes.len() >= 1000).then(|| nist::mcv_min_entropy(&bytes).0),
                autocorr: qrng::autocorr(&bits, 16),
            })
        }),
        "quantum.design" => handle(input, |i: DesignIn| Ok(design_view(&i.project))),
        _ => None,
    }
}

/// Design parameters of the project's quantum link / QRNG (defaults filled in).
fn design_view(p: &Project) -> Value {
    let qkd = p
        .design
        .extra
        .get("qkd")
        .and_then(|v| serde_json::from_value::<qkd::LinkParams>(v.clone()).ok());
    let qrng = p
        .design
        .extra
        .get("qrng")
        .and_then(|v| serde_json::from_value::<qrng::QrngIn>(v.clone()).ok());
    json!({
        "qkd": qkd,
        "qrng": qrng,
        "random_bits_per_pulse": random_bits_per_pulse(p),
        "metrics": metrics(p),
    })
}

fn random_bits_per_pulse(p: &Project) -> f64 {
    p.design
        .extra
        .get("qkd")
        .and_then(|v| v.get("random_bits_per_pulse"))
        .and_then(|v| v.as_f64())
        .unwrap_or(3.0)
}

thread_local! {
    static LINK_MEMO: Slot<(qkd::LinkPoint, f64, f64, f64)> = const { std::cell::RefCell::new(None) };
    static QRNG_MEMO: Slot<(f64, f64, usize, usize, f64)> = const { std::cell::RefCell::new(None) };
}

/// Metrics this module contributes to requirement verification.
pub fn metrics(p: &Project) -> Vec<Metric> {
    let mut out = vec![];
    let link = p
        .design
        .extra
        .get("qkd")
        .and_then(|v| serde_json::from_value::<qkd::LinkParams>(v.clone()).ok());
    if let Some(lp) = &link {
        let (pt, dmax, plob_bps, loss) = memo::memo(&LINK_MEMO, memo::key(lp), || {
            let pt = qkd::best_decoy(lp, lp.distance_km);
            let dmax = qkd::max_distance(|l| qkd::best_decoy(lp, l).rate_per_pulse, 600.0);
            let plob = qkd::plob(lp, lp.distance_km) * lp.source_rate_hz;
            (pt, dmax, plob, lp.fiber_loss_db_km * lp.distance_km + lp.extra_loss_db)
        });
        out.push(
            Metric::new(
                "quantum.qkd_key_rate_bps",
                "Secret key rate at the design distance",
                pt.key_rate_bps,
                "bit/s",
                "quantum",
            )
            .with_note(&format!(
                "decoy-state BB84 (Lo–Ma–Chen 2005, asymptotic) at {:.0} km: μ = {:.2}, gain {:.2e}, {:.3e} bit/pulse × {:.0} MHz{}",
                lp.distance_km,
                pt.mu,
                pt.gain,
                pt.rate_per_pulse,
                lp.source_rate_hz / 1e6,
                if lp.dead_time_s > 0.0 {
                    format!(", {:.1} µs detector dead time", lp.dead_time_s * 1e6)
                } else {
                    String::new()
                }
            )),
        );
        out.push(
            Metric::new(
                "quantum.qkd_qber_pct",
                "Quantum bit error rate at the design distance",
                pt.qber * 100.0,
                "%",
                "quantum",
            )
            .with_note(&format!(
                "E_μ = [e₀Y₀ + e_d(1 − e^(−ημ))]/Q_μ with e_d = {:.1} %, Y₀ = {:.1e}",
                lp.misalignment * 100.0,
                2.0 * lp.dark_count_prob
            )),
        );
        out.push(
            Metric::new(
                "quantum.qkd_max_distance_km",
                "Maximum distance with a positive key rate",
                dmax,
                "km",
                "quantum",
            )
            .with_note("bisection on the decoy-state key rate with μ optimised at each distance"),
        );
        out.push(
            Metric::new(
                "quantum.qkd_link_loss_db",
                "Total channel loss at the design distance",
                loss,
                "dB",
                "quantum",
            )
            .with_note(&format!(
                "{:.2} dB/km × {:.0} km + {:.1} dB fixed",
                lp.fiber_loss_db_km, lp.distance_km, lp.extra_loss_db
            )),
        );
        let ratio = if plob_bps > 0.0 { pt.key_rate_bps / plob_bps * 100.0 } else { 0.0 };
        out.push(
            Metric::new(
                "quantum.qkd_plob_fraction_pct",
                "Key rate as a share of the PLOB bound",
                ratio,
                "%",
                "quantum",
            )
            .with_note(&format!(
                "PLOB −log₂(1 − η_fiber) × rate = {plob_bps:.3e} bit/s (Pirandola et al. 2017)"
            )),
        );
    }
    let qin = p
        .design
        .extra
        .get("qrng")
        .and_then(|v| serde_json::from_value::<qrng::QrngIn>(v.clone()).ok());
    if let Some(mut qi) = qin {
        qi.samples = qi.samples.min(20_000);
        let (h, rate, passed, applicable, clipped) = memo::memo(&QRNG_MEMO, memo::key(&qi), || {
            let o = qrng::simulate(&qi);
            let last = o.stages.last().expect("raw stage");
            (
                o.min_entropy_per_sample,
                o.output_rate_bps,
                last.passed,
                last.applicable,
                o.clipped_fraction,
            )
        });
        out.push(
            Metric::new(
                "quantum.qrng_min_entropy_bits",
                "QRNG min-entropy per digitised sample",
                h,
                "bit",
                "quantum",
            )
            .with_note("NIST SP 800-90B most-common-value estimate on simulated samples"),
        );
        out.push(
            Metric::new(
                "quantum.qrng_output_rate_mbps",
                "QRNG conditioned output rate",
                rate / 1e6,
                "Mbit/s",
                "quantum",
            )
            .with_note("ADC rate × bits per sample × extractor efficiency"),
        );
        out.push(
            Metric::new(
                "quantum.qrng_nist_failures",
                "NIST SP 800-22 tests failed by the conditioned stream",
                (applicable - passed) as f64,
                "count",
                "quantum",
            )
            .with_note(&format!(
                "{passed}/{applicable} passed at α = 0.01; clipping {:.2} %",
                clipped * 100.0
            )),
        );
        if let Some(lp) = &link {
            let need = random_bits_per_pulse(p) * lp.source_rate_hz;
            out.push(
                Metric::new(
                    "quantum.qrng_supply_ratio",
                    "QRNG output ÷ random bits the transmitter consumes",
                    rate / need,
                    "×",
                    "quantum",
                )
                .with_note(&format!(
                    "needs {:.0} bits/pulse × {:.0} MHz = {:.0} Mbit/s",
                    random_bits_per_pulse(p),
                    lp.source_rate_hz / 1e6,
                    need / 1e6
                )),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_has_no_quantum_metrics() {
        assert!(metrics(&Project::iris()).is_empty());
    }

    #[test]
    fn dispatch_round_trips() {
        let c = json!({"qubits": 2, "clbits": 2, "cols": [[{"gate":"h","targets":[0]}],[{"gate":"cx","targets":[0,1]}],[{"gate":"measure","targets":[0],"clbit":0},{"gate":"measure","targets":[1],"clbit":1}]]});
        let r = crate::api::call_value("quantum.run", &json!({"circuit": c, "shots": 100, "steps": true})).unwrap();
        assert_eq!(r["steps"].as_array().unwrap().len(), 4);
        let e = crate::api::call_value("quantum.export", &json!({"circuit": c})).unwrap();
        assert!(e["qasm"].as_str().unwrap().contains("cx q[0],q[1];"));
        assert!(e["qasm3"].as_str().unwrap().contains("c[1] = measure q[1];"));
        assert!(e["qiskit"].as_str().unwrap().contains("qc.cx(0, 1)"));
        assert!(e["quirk_url"].as_str().unwrap().starts_with("https://algassert.com/quirk#circuit="));
        let n = crate::api::call_value(
            "quantum.nist",
            &json!({"bits": "hex:C90FDAA22168C234C4C6628B80DC1CD129024E088A67CC74"}),
        )
        .unwrap();
        assert_eq!(n["n"], 192);
        for m in ["quantum.gates", "quantum.library"] {
            assert!(crate::api::call_value(m, &Value::Null).is_ok());
        }
    }
}
