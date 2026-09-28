//! # Chip Lab engine
//!
//! Digital design from Boolean algebra to a Tiny Tapeout submission.
//!
//! | File | What |
//! |---|---|
//! | [`logic`] | expression parser, truth tables, Quine–McCluskey, Petrick, SOP/POS, K-maps |
//! | [`sim`] | event-driven gate simulator with delays, glitches and static timing |
//! | [`fsm`] | data-driven Moore FSMs, the Iris gesture recogniser, Verilog + testbench |
//! | [`firgen`] | CSD FIR → Verilog (transposed / decimating MAC), CIC, cost & timing model |
//! | [`tt`] | `tt_um_iris_dsp` top level, end-to-end testbench, README |
//!
//! Metrics: `chip.critical_path_ns` (R-015) and `chip.est_cells` (R-016)
//! for the IRIS-DSP co-processor = CIC + decimation FIR + gesture FSM +
//! Tiny Tapeout glue.
//!
//! The FIR architecture is read from `design.extra.chip_fir_arch`
//! (`"mac"` — the default, a decimating multiply-accumulate — or
//! `"transposed"`), with `chip_fir_adder` (`"ripple"`/`"cla"`),
//! `chip_fir_pipeline` (bool) and `chip_fsm_encoding` (`"binary"`/`"one_hot"`).

pub mod firgen;
pub mod fsm;
pub mod logic;
pub mod sim;
pub mod tt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Metric, Project};
use firgen::{AdderKind, Arch, CicSpec, DspChipCost, FirGen, FirGenOptions};
use fsm::Encoding;

/// IRIS-DSP implementation choices (from `design.extra`, overridable per call).
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ImplChoice {
    pub arch: Option<Arch>,
    pub adder: Option<AdderKind>,
    pub pipeline: Option<bool>,
    pub encoding: Option<Encoding>,
    pub coeff_bits: Option<u32>,
    pub data_bits: Option<u32>,
    /// Custom coefficients (default: the DSP lab's Iris decimation FIR).
    pub coeffs: Option<Vec<f64>>,
    pub tap_ms: Option<u32>,
    pub long_ms: Option<u32>,
    pub gap_ms: Option<u32>,
}

fn extra_str(p: &Project, k: &str) -> Option<String> {
    p.design.extra.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

impl ImplChoice {
    pub fn resolve(&self, p: &Project) -> ResolvedChoice {
        let arch = self.arch.unwrap_or_else(|| match extra_str(p, "chip_fir_arch").as_deref() {
            Some("transposed") => Arch::Transposed,
            _ => Arch::Mac,
        });
        let adder = self.adder.unwrap_or_else(|| match extra_str(p, "chip_fir_adder").as_deref() {
            Some("cla") => AdderKind::Cla,
            _ => AdderKind::Ripple,
        });
        let pipeline = self
            .pipeline
            .unwrap_or_else(|| p.design.extra.get("chip_fir_pipeline").and_then(|v| v.as_bool()).unwrap_or(false));
        let encoding = self.encoding.unwrap_or_else(|| match extra_str(p, "chip_fsm_encoding").as_deref() {
            Some("one_hot") => Encoding::OneHot,
            _ => Encoding::Binary,
        });
        ResolvedChoice {
            arch,
            adder,
            pipeline,
            encoding,
            coeff_bits: self.coeff_bits.unwrap_or(p.design.chip.coeff_bits),
            data_bits: self.data_bits.unwrap_or(p.design.chip.data_bits),
            tap_ms: self.tap_ms.unwrap_or(250),
            long_ms: self.long_ms.unwrap_or(600),
            gap_ms: self.gap_ms.unwrap_or(250),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedChoice {
    pub arch: Arch,
    pub adder: AdderKind,
    pub pipeline: bool,
    pub encoding: Encoding,
    pub coeff_bits: u32,
    pub data_bits: u32,
    pub tap_ms: u32,
    pub long_ms: u32,
    pub gap_ms: u32,
}

/// Everything about the IRIS-DSP co-processor for one project.
#[derive(Clone, Debug, Serialize)]
pub struct IrisDsp {
    pub choice: ResolvedChoice,
    pub coeff_source: String,
    pub cic: CicSpec,
    pub fir: FirGen,
    pub fsm: fsm::Fsm,
    pub summary: DspChipCost,
    /// The same design with the other FIR architecture, for comparison.
    pub alternative: DspChipCost,
}

pub fn iris_dsp(p: &Project, ch: &ImplChoice, test_len: usize) -> Result<IrisDsp, String> {
    let c = ch.resolve(p);
    let chain = crate::dsp::pdm::design_chain(&p.design.audio, &Default::default());
    let (coeffs, source) = match &ch.coeffs {
        Some(v) if !v.is_empty() => (v.clone(), "custom coefficients".to_string()),
        _ => (
            chain.fir.clone(),
            format!(
                "DSP Lab decimation FIR ({} taps at {:.0} Hz, ↓{})",
                chain.fir.len(),
                chain.fir_in_hz,
                chain.fir_decim
            ),
        ),
    };
    let gd = p.design.chip.gate_delay_ns;
    let clock = p.design.chip.clock_mhz;
    let opt = FirGenOptions {
        arch: c.arch,
        adder: c.adder,
        pipeline: c.pipeline,
        coeff_bits: c.coeff_bits,
        data_bits: c.data_bits,
        decim: chain.fir_decim,
        gate_delay_ns: gd,
        clock_mhz: clock,
        module_name: "iris_fir".into(),
        test_len,
    };
    let fir = firgen::generate(&coeffs, &opt)?;
    let alt = firgen::generate(
        &coeffs,
        &FirGenOptions {
            arch: if c.arch == Arch::Mac { Arch::Transposed } else { Arch::Mac },
            test_len: 40,
            ..opt.clone()
        },
    )?;
    let cic = CicSpec {
        order: chain.cic_order,
        rate: chain.cic_r * if chain.halfband.is_empty() { 1 } else { 2 },
        diff_delay: chain.cic_m,
        width: chain.cic_register_bits,
    };
    let f = fsm::gesture_fsm(c.tap_ms, c.long_ms, c.gap_ms);
    let common = |firc: firgen::BlockCost| {
        vec![
            firgen::cic_cost(&cic, c.adder, gd),
            firc,
            firgen::fsm_block(&f, c.encoding, gd),
            firgen::glue_cost(clock, gd),
        ]
    };
    let summary = firgen::summarize(common(fir.cost.clone()), clock, p.design.chip.cell_budget, c.arch);
    let alternative = firgen::summarize(common(alt.cost.clone()), clock, p.design.chip.cell_budget, alt.arch);
    Ok(IrisDsp {
        choice: c,
        coeff_source: source,
        cic,
        fir,
        fsm: f,
        summary,
        alternative,
    })
}

// ------------------------------------------------------------------ API inputs

#[derive(Deserialize, Default)]
#[serde(default)]
struct DspIn {
    project: Option<Project>,
    #[serde(flatten)]
    choice: ImplChoice,
    test_len: Option<usize>,
}

#[derive(Deserialize)]
#[serde(default)]
struct SimIn {
    circuit: String,
    custom: Option<sim::Circuit>,
    logic: Option<logic::MinimizeInput>,
    stimulus: Option<sim::Stimulus>,
    delay_scale: f64,
    glitch_ns: f64,
}
impl Default for SimIn {
    fn default() -> Self {
        SimIn {
            circuit: "rca4".into(),
            custom: None,
            logic: None,
            stimulus: None,
            delay_scale: 1.0,
            glitch_ns: 4.0,
        }
    }
}

#[derive(Serialize)]
struct CircuitInfo {
    name: String,
    title: String,
    description: String,
    inputs: Vec<String>,
}

#[derive(Deserialize)]
#[serde(default)]
struct FsmIn {
    tap_ms: u32,
    long_ms: u32,
    gap_ms: u32,
    encoding: Encoding,
    /// Scripted trace name (tap, double_tap, long_press, slow_press, mixed) used when `touch` is empty.
    trace: String,
    touch: Vec<u8>,
    state: usize,
    timer: u32,
    verilog: bool,
    gate_delay_ns: f64,
}
impl Default for FsmIn {
    fn default() -> Self {
        FsmIn {
            tap_ms: 250,
            long_ms: 600,
            gap_ms: 250,
            encoding: Encoding::Binary,
            trace: "mixed".into(),
            touch: vec![],
            state: 0,
            timer: 0,
            verilog: true,
            gate_delay_ns: 0.25,
        }
    }
}

#[derive(Serialize)]
struct FsmOut {
    fsm: fsm::Fsm,
    touch: Vec<u8>,
    trace: fsm::FsmTrace,
    timer_bits: u32,
    state_bits: u32,
    verilog: Option<String>,
    testbench: Option<String>,
    cost: fsm::FsmCost,
}

fn run_fsm(i: FsmIn) -> Result<FsmOut, String> {
    let f = fsm::gesture_fsm(i.tap_ms.clamp(10, 5000), i.long_ms.clamp(20, 10000), i.gap_ms.clamp(10, 5000));
    f.validate()?;
    let touch: Vec<bool> = if i.touch.is_empty() {
        fsm::scripted(&i.trace)
    } else {
        i.touch.iter().map(|&v| v != 0).collect()
    };
    let trace = fsm::simulate(&f, &touch, i.state, i.timer);
    let (verilog, testbench) = if i.verilog {
        (Some(fsm::verilog(&f, i.encoding)), Some(fsm::testbench(&f, &touch)))
    } else {
        (None, None)
    };
    Ok(FsmOut {
        timer_bits: f.timer_bits(),
        state_bits: f.state_bits(),
        cost: fsm::cost(&f, i.encoding, i.gate_delay_ns.max(0.01)),
        touch: touch.iter().map(|&b| b as u8).collect(),
        trace,
        verilog,
        testbench,
        fsm: f,
    })
}

#[derive(Serialize)]
struct TtOut {
    files: Vec<tt::File>,
    design: IrisDsp,
}

/// PDM bits of a 1 kHz tone for the end-to-end testbench.
fn test_pdm_bits(p: &Project, n_out: usize) -> Vec<bool> {
    let a = &p.design.audio;
    let n = n_out * a.decimation.max(1) as usize;
    let x: Vec<f64> = (0..n)
        .map(|i| 0.5 * (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / a.pdm_clock_hz).sin())
        .collect();
    crate::dsp::pdm::sigma_delta(&x, 2).into_iter().map(|b| b > 0).collect()
}

pub fn tiny_tapeout(p: &Project, ch: &ImplChoice) -> Result<(Vec<tt::File>, IrisDsp), String> {
    let d = iris_dsp(p, ch, 160)?;
    let (codes, _) = firgen::quantize(&d.fir.coefs.iter().map(|c| c.value).collect::<Vec<_>>(), d.fir.coeff_bits);
    let bits = test_pdm_bits(p, 48);
    let touch = fsm::scripted("mixed");
    let files = vec![
        tt::File {
            name: "tt_um_iris_dsp.v".into(),
            content: tt::top_verilog(&d.cic, &d.fir, p.design.chip.clock_mhz),
        },
        tt::File {
            name: "iris_cic.v".into(),
            content: firgen::cic_verilog(&d.cic),
        },
        tt::File {
            name: "iris_fir.v".into(),
            content: d.fir.verilog.clone(),
        },
        tt::File {
            name: "iris_gesture_fsm.v".into(),
            content: fsm::verilog(&d.fsm, d.choice.encoding),
        },
        tt::File {
            name: "tb_iris_fir.v".into(),
            content: d.fir.testbench.clone(),
        },
        tt::File {
            name: "tb_iris_gesture_fsm.v".into(),
            content: fsm::testbench(&d.fsm, &touch),
        },
        tt::File {
            name: "tb_tt_um_iris_dsp.v".into(),
            content: tt::top_testbench(&d.cic, &d.fir, &codes, &bits),
        },
        tt::File {
            name: "README.md".into(),
            content: tt::readme(
                &d.cic,
                &d.fir,
                &d.fsm,
                d.choice.encoding,
                p.design.chip.clock_mhz,
                d.summary.total_cells,
                d.summary.critical_ns,
            ),
        },
    ];
    Ok((files, d))
}

pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "chip.minimize" => handle(input, |i: logic::MinimizeInput| logic::minimize(&i)),
        "chip.circuits" => handle(input, |_: Value| {
            Ok(sim::BUILTINS
                .iter()
                .filter_map(|n| sim::builtin(n))
                .map(|c| CircuitInfo {
                    name: c.name,
                    title: c.title,
                    description: c.description,
                    inputs: c.inputs,
                })
                .collect::<Vec<_>>())
        }),
        "chip.simulate" => handle(input, |i: SimIn| {
            let mut c = if let Some(c) = i.custom {
                c
            } else if let Some(l) = i.logic {
                let r = logic::minimize(&l)?;
                sim::from_logic(&r.vars, &r.netlist)
            } else {
                sim::builtin(&i.circuit).ok_or_else(|| format!("unknown circuit `{}`", i.circuit))?
            };
            if let Some(s) = i.stimulus {
                c.stimulus = s;
            }
            Ok(sim::simulate(
                &c,
                if i.delay_scale > 0.0 { i.delay_scale } else { 1.0 },
                if i.glitch_ns > 0.0 { i.glitch_ns } else { 4.0 },
            ))
        }),
        "chip.fsm" => handle(input, run_fsm),
        "chip.iris_dsp" => handle(input, |i: DspIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            iris_dsp(&p, &i.choice, i.test_len.unwrap_or(160))
        }),
        "chip.tinytapeout" => handle(input, |i: DspIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            let (files, design) = tiny_tapeout(&p, &i.choice)?;
            Ok(TtOut { files, design })
        }),
        "chip.csd" => handle(input, |i: CsdIn| {
            Ok(i.values.iter().map(|&v| {
                let d = firgen::csd(v);
                serde_json::json!({ "value": v, "csd_lsb_first": d, "nonzero": d.iter().filter(|&&x| x != 0).count(), "binary_nonzero": v.unsigned_abs().count_ones() })
            }).collect::<Vec<_>>())
        }),
        _ => None,
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct CsdIn {
    values: Vec<i64>,
}

pub fn metrics(p: &Project) -> Vec<Metric> {
    let Ok(d) = iris_dsp(p, &ImplChoice::default(), 40) else {
        return vec![];
    };
    let s = &d.summary;
    let arch = match d.choice.arch {
        Arch::Mac => "decimating MAC",
        Arch::Transposed => "transposed-form",
    };
    vec![
        Metric::new("chip.critical_path_ns", "IRIS-DSP critical path", s.critical_ns, "ns", "chip").with_note(&format!(
            "{}: {} ({:?} adders); {arch} FIR; clock period {:.1} ns; the {} FIR would need {:.1} ns",
            s.critical_block,
            s.critical_desc,
            d.choice.adder,
            s.clock_period_ns,
            if d.choice.arch == Arch::Mac {
                "transposed-form"
            } else {
                "decimating MAC"
            },
            d.alternative.critical_ns
        )),
        Metric::new(
            "chip.est_cells",
            "IRIS-DSP estimated standard cells",
            s.total_cells.round(),
            "cells",
            "chip",
        )
        .with_note(&format!(
            "{}; {arch} FIR, {}-bit coefficients, {}-bit data (alternative architecture: {:.0} cells)",
            s.blocks
                .iter()
                .map(|b| format!("{} {:.0}", b.name, b.cells))
                .collect::<Vec<_>>()
                .join(", "),
            d.choice.coeff_bits,
            d.choice.data_bits,
            d.alternative.total_cells
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_chip_metrics() {
        let p = Project::iris();
        let t0 = std::time::Instant::now();
        let m = metrics(&p);
        let dt = t0.elapsed();
        let get = |k: &str| m.iter().find(|x| x.key == k).unwrap().value;
        let cp = get("chip.critical_path_ns");
        let cells = get("chip.est_cells");
        eprintln!(
            "chip metrics in {dt:?}: {cp:.2} ns, {cells} cells\n  {}\n  {}",
            m[0].note, m[1].note
        );
        assert!(cp > 5.0 && cp <= 20.0, "critical path {cp}");
        assert!(cells > 500.0 && cells <= 3000.0, "cells {cells}");
        // switching to the transposed form fails timing with ripple adders
        let mut p2 = p.clone();
        p2.design.extra.insert("chip_fir_arch".into(), Value::String("transposed".into()));
        let m2 = metrics(&p2);
        assert!(m2[0].value > 20.0);
    }

    #[test]
    fn tiny_tapeout_package_is_complete() {
        let (files, d) = tiny_tapeout(&Project::iris(), &ImplChoice::default()).unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        for n in [
            "tt_um_iris_dsp.v",
            "iris_cic.v",
            "iris_fir.v",
            "iris_gesture_fsm.v",
            "README.md",
            "tb_tt_um_iris_dsp.v",
        ] {
            assert!(names.contains(&n), "{n}");
        }
        let top = &files[0].content;
        for port in [
            "input  wire [7:0] ui_in",
            "output wire [7:0] uo_out",
            "input  wire [7:0] uio_in",
            "output wire [7:0] uio_out",
            "output wire [7:0] uio_oe",
            "input  wire       ena",
            "input  wire       clk",
            "input  wire       rst_n",
        ] {
            assert!(top.contains(port), "missing port {port}");
        }
        assert!(d.summary.timing_ok && d.summary.area_ok);
    }

    #[test]
    fn dispatch_methods() {
        let r = dispatch("chip.minimize", &serde_json::json!({"expr": "a b + a c + b c"}))
            .unwrap()
            .unwrap();
        assert_eq!(r["sop"], "ab + ac + bc");
        let r = dispatch("chip.simulate", &serde_json::json!({"circuit": "rca4"})).unwrap().unwrap();
        assert!(r["waves"].as_array().unwrap().len() > 10);
        let r = dispatch("chip.simulate", &serde_json::json!({"logic": {"expr": "a^b"}}))
            .unwrap()
            .unwrap();
        assert_eq!(r["outputs"][0], "F");
        let r = dispatch("chip.fsm", &serde_json::json!({"trace": "double_tap", "encoding": "one_hot"}))
            .unwrap()
            .unwrap();
        assert_eq!(r["trace"]["events"][0]["output"], "double_tap");
        assert!(dispatch("chip.nope", &Value::Null).is_none());
    }

    /// Writes every generated Verilog file to $FORGE_DUMP_DIR for checking with
    /// Icarus Verilog / Yosys: `FORGE_DUMP_DIR=/tmp/v cargo test -p forge-core dump_verilog -- --ignored`.
    #[test]
    #[ignore]
    fn dump_verilog() {
        let dir = std::env::var("FORGE_DUMP_DIR").expect("set FORGE_DUMP_DIR");
        let p = Project::iris();
        for (tag, ch) in [
            ("mac", ImplChoice::default()),
            (
                "transposed",
                ImplChoice {
                    arch: Some(Arch::Transposed),
                    ..Default::default()
                },
            ),
            (
                "transposed_pipe",
                ImplChoice {
                    arch: Some(Arch::Transposed),
                    pipeline: Some(true),
                    adder: Some(AdderKind::Cla),
                    encoding: Some(Encoding::OneHot),
                    ..Default::default()
                },
            ),
        ] {
            let (files, d) = tiny_tapeout(&p, &ch).unwrap();
            let sub = format!("{dir}/{tag}");
            std::fs::create_dir_all(&sub).unwrap();
            for f in files {
                std::fs::write(format!("{sub}/{}", f.name), f.content).unwrap();
            }
            eprintln!("{tag}: {:.0} cells, {:.2} ns", d.summary.total_cells, d.summary.critical_ns);
            for b in &d.summary.blocks {
                eprintln!(
                    "   {:<24} {:>6.0} cells ({:.0} ff, {:.0} adder, {:.0} other)  {:.2} ns  {}",
                    b.name, b.cells, b.ffs, b.adder_cells, b.other_cells, b.critical_ns, b.critical_desc
                );
            }
        }
    }
}
