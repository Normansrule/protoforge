//! # Circuit Lab engine — a small SPICE in Rust
//!
//! A teaching-grade but honest circuit simulator:
//!
//! * [`parser`] reads a subset of the SPICE3/ngspice netlist language, so the
//!   same deck runs in `ngspice -b deck.cir` for cross-checking.
//! * [`mna`] builds the **Modified Nodal Analysis** equations `A·x = b`
//!   (Ho, Ruehli & Brennan 1975) and solves nonlinear circuits with
//!   **Newton–Raphson** plus SPICE-style junction limiting (Nagel 1975).
//! * [`analysis`] runs `.op` (with gmin- and source-stepping fallbacks),
//!   `.dc` sweeps, `.tran` (trapezoidal / backward-Euler companion models,
//!   waveform breakpoints and local-truncation-error step control) and `.ac`
//!   (complex MNA linearised at the operating point).
//! * [`presets`] ships the Iris circuits (1.8 V rail load step, buck ripple,
//!   battery-sense divider, IR LED driver, touch sensing) and classics, each
//!   with a schematic layout the web page animates.
//!
//! Metrics: `circuit.rail_droop_mv`, `circuit.buck_ripple_mv`,
//! `circuit.vbat_divider_ua`.
//!
//! References: L. W. Nagel, "SPICE2: A Computer Program to Simulate
//! Semiconductor Circuits", UC Berkeley ERL-M520, 1975; C.-W. Ho, A. Ruehli,
//! P. Brennan, "The modified nodal approach to network analysis", IEEE Trans.
//! Circuits and Systems, 1975; H. Shichman & D. Hodges, IEEE JSSC, 1968;
//! J. Vlach & K. Singhal, *Computer Methods for Circuit Analysis and Design*;
//! ngspice manual <https://ngspice.sourceforge.io/docs.html>.

pub mod analysis;
pub mod devices;
pub mod measure;
pub mod mna;
pub mod parser;
pub mod presets;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Metric, Project};

use analysis::{AcResult, DcResult, OpResult, SolverStats, TranResult};
use mna::{Ckt, Mode};
use parser::{Analysis, Device, Netlist, ParseError, Wave};

/// Optional compact encoding of waveform arrays for the web front end:
/// little-endian `f32`/`f64` bytes in base64 instead of JSON number lists
/// (4–6× smaller and much faster to produce and parse). Enabled per call
/// with `"compact": true`; the default stays plain JSON for other clients.
pub mod enc {
    use serde::ser::SerializeSeq;
    use serde::{Serialize, Serializer};
    use std::cell::Cell;

    thread_local! {
        static COMPACT: Cell<bool> = const { Cell::new(false) };
    }

    pub fn set_compact(on: bool) {
        COMPACT.with(|c| c.set(on));
    }
    fn compact() -> bool {
        COMPACT.with(|c| c.get())
    }

    const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn base64(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for c in bytes.chunks(3) {
            let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
            out.push(B64[(n >> 18) as usize & 63] as char);
            out.push(B64[(n >> 12) as usize & 63] as char);
            out.push(if c.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
            out.push(if c.len() > 2 { B64[n as usize & 63] as char } else { '=' });
        }
        out
    }

    pub fn f32s<S: Serializer>(v: &Vec<Vec<f32>>, s: S) -> Result<S::Ok, S::Error> {
        if !compact() {
            return v.serialize(s);
        }
        let mut seq = s.serialize_seq(Some(v.len()))?;
        for col in v {
            let bytes: Vec<u8> = col.iter().flat_map(|x| x.to_le_bytes()).collect();
            seq.serialize_element(&base64(&bytes))?;
        }
        seq.end()
    }

    pub fn f64s<S: Serializer>(v: &Vec<f64>, s: S) -> Result<S::Ok, S::Error> {
        if !compact() {
            return v.serialize(s);
        }
        let bytes: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
        s.serialize_str(&base64(&bytes))
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn base64_known_vectors() {
            assert_eq!(super::base64(b"Man"), "TWFu");
            assert_eq!(super::base64(b"Ma"), "TWE=");
            assert_eq!(super::base64(b"M"), "TQ==");
        }
    }
}

/// Format a value with an SI prefix and unit, e.g. `fmt_si(4.7e-6, "F")` →
/// `"4.7 µF"`.
pub fn fmt_si(v: f64, unit: &str) -> String {
    if !v.is_finite() {
        return format!("{v} {unit}");
    }
    if v == 0.0 {
        return format!("0 {unit}");
    }
    let a = v.abs();
    const P: [(f64, &str); 10] = [
        (1e12, "T"),
        (1e9, "G"),
        (1e6, "M"),
        (1e3, "k"),
        (1.0, ""),
        (1e-3, "m"),
        (1e-6, "µ"),
        (1e-9, "n"),
        (1e-12, "p"),
        (1e-15, "f"),
    ];
    let (s, p) = P.iter().find(|(s, _)| a >= *s * 0.9995).copied().unwrap_or((1e-15, "f"));
    let m = v / s;
    let txt = if m.abs() >= 99.95 {
        format!("{m:.0}")
    } else if m.abs() >= 9.995 {
        format!("{m:.1}")
    } else {
        format!("{m:.2}")
    };
    let txt = if txt.contains('.') {
        txt.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        txt
    };
    format!("{txt} {p}{unit}")
}

/// Element summary for the UI.
#[derive(Clone, Debug, Serialize)]
pub struct ElementInfo {
    pub name: String,
    /// resistor, capacitor, inductor, vsource, isource, diode, nmos, pmos,
    /// vcvs, vccs, switch
    pub kind: String,
    pub nodes: Vec<String>,
    /// Pretty value, e.g. "10 µF" or "PULSE 0 → 1.8 V".
    pub value: String,
    /// Source waveform kind (dc, pulse, sin, pwl, exp) for V/I sources.
    pub wave: String,
    /// Switches: `[RON, ROFF]` (lets the UI draw the switch open/closed).
    pub sw: Option<[f64; 2]>,
    pub line: usize,
}

/// A named measurement (like ngspice `.meas`) with optional pass/fail.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Measurement {
    pub key: String,
    pub label: String,
    pub value: f64,
    pub unit: String,
    #[serde(default)]
    pub note: String,
    /// `Some(true)` pass, `Some(false)` fail against a design limit.
    #[serde(default)]
    pub pass: Option<bool>,
}

impl Measurement {
    pub fn new(key: &str, label: &str, value: f64, unit: &str) -> Self {
        Measurement {
            key: key.into(),
            label: label.into(),
            value,
            unit: unit.into(),
            note: String::new(),
            pass: None,
        }
    }
    pub fn note(mut self, n: impl Into<String>) -> Self {
        self.note = n.into();
        self
    }
    pub fn check(mut self, ok: bool) -> Self {
        self.pass = Some(ok);
        self
    }
}

/// Plot annotation produced by preset measurements.
#[derive(Clone, Debug, Serialize, Default)]
pub struct Annotation {
    /// "tran", "ac" or "dc"
    pub plot: String,
    /// "x" (vertical line) or "y" (horizontal line)
    pub axis: String,
    pub value: f64,
    pub label: String,
}

/// Everything one simulation produces.
#[derive(Clone, Debug, Serialize, Default)]
pub struct SimResult {
    pub title: String,
    /// User node names excluding ground; `v[k]` arrays follow this order.
    pub nodes: Vec<String>,
    pub elements: Vec<ElementInfo>,
    pub params: BTreeMap<String, f64>,
    /// The analysis cards, e.g. `.tran 10n 60u`.
    pub analyses: Vec<String>,
    pub op: Option<OpResult>,
    pub dc: Option<DcResult>,
    pub tran: Option<TranResult>,
    pub ac: Option<AcResult>,
    pub measurements: Vec<Measurement>,
    pub annotations: Vec<Annotation>,
    /// Suggested initial time window for transient plots.
    pub tran_window: Option<[f64; 2]>,
    pub stats: SolverStats,
    pub warnings: Vec<String>,
    /// The effective deck (parameter overrides written into `.param` cards).
    pub netlist: String,
}

impl SimResult {
    pub fn node_idx(&self, name: &str) -> Option<usize> {
        let n = name.to_ascii_lowercase();
        self.nodes.iter().position(|x| *x == n)
    }
    pub fn elem_idx(&self, name: &str) -> Option<usize> {
        self.elements.iter().position(|e| e.name.eq_ignore_ascii_case(name))
    }
    /// Transient voltage of a node (`"0"` gives `None`).
    pub fn tv(&self, node: &str) -> Option<&Vec<f32>> {
        let t = self.tran.as_ref()?;
        t.v.get(self.node_idx(node)?)
    }
    /// Transient current of an element.
    pub fn ti(&self, el: &str) -> Option<&Vec<f32>> {
        let t = self.tran.as_ref()?;
        t.i.get(self.elem_idx(el)?)
    }
    pub fn op_v(&self, node: &str) -> Option<f64> {
        self.op.as_ref()?.node_v.get(self.node_idx(node)?).copied()
    }
    pub fn op_i(&self, el: &str) -> Option<f64> {
        self.op.as_ref()?.elem_i.get(self.elem_idx(el)?).copied()
    }
}

fn kind_name(e: &parser::Element) -> &'static str {
    match &e.dev {
        Device::Resistor { .. } => "resistor",
        Device::Capacitor { .. } => "capacitor",
        Device::Inductor { .. } => "inductor",
        Device::Vsource { .. } => "vsource",
        Device::Isource { .. } => "isource",
        Device::Diode { .. } => "diode",
        Device::Mosfet { model, .. } => {
            if model.pmos {
                "pmos"
            } else {
                "nmos"
            }
        }
        Device::Vcvs { .. } => "vcvs",
        Device::Vccs { .. } => "vccs",
        Device::Switch { .. } => "switch",
    }
}

fn wave_desc(w: &Wave, dc: f64, unit: &str) -> (String, &'static str) {
    match w {
        Wave::None => (fmt_si(dc, unit), "dc"),
        Wave::Pulse { v1, v2, per, .. } => {
            let f = per.map(|p| format!(" @ {}", fmt_si(1.0 / p, "Hz"))).unwrap_or_default();
            (format!("PULSE {} → {}{f}", fmt_si(*v1, unit), fmt_si(*v2, unit)), "pulse")
        }
        Wave::Sin { vo, va, freq, .. } => {
            let f = freq.map(|f| format!(" @ {}", fmt_si(f, "Hz"))).unwrap_or_default();
            let off = if *vo != 0.0 {
                format!("{} + ", fmt_si(*vo, unit))
            } else {
                String::new()
            };
            (format!("SIN {off}{} pk{f}", fmt_si(*va, unit)), "sin")
        }
        Wave::Exp { v1, v2, .. } => (format!("EXP {} → {}", fmt_si(*v1, unit), fmt_si(*v2, unit)), "exp"),
        Wave::Pwl { points } => (format!("PWL ({} points)", points.len()), "pwl"),
    }
}

/// Describe every element of a netlist for the UI.
pub fn element_infos(nl: &Netlist) -> Vec<ElementInfo> {
    nl.elements
        .iter()
        .map(|e| {
            let (value, wave) = match &e.dev {
                Device::Resistor { r } => (fmt_si(*r, "Ω"), ""),
                Device::Capacitor { c, ic } => (
                    format!(
                        "{}{}",
                        fmt_si(*c, "F"),
                        ic.map(|v| format!(" (IC {})", fmt_si(v, "V"))).unwrap_or_default()
                    ),
                    "",
                ),
                Device::Inductor { l, ic } => (
                    format!(
                        "{}{}",
                        fmt_si(*l, "H"),
                        ic.map(|v| format!(" (IC {})", fmt_si(v, "A"))).unwrap_or_default()
                    ),
                    "",
                ),
                Device::Vsource { src } | Device::Isource { src } => {
                    let unit = if matches!(e.dev, Device::Vsource { .. }) { "V" } else { "A" };
                    let (mut s, w) = wave_desc(&src.wave, src.dc, unit);
                    if src.ac_mag != 0.0 {
                        s.push_str(&format!(" · AC {}", fmt_si(src.ac_mag, unit)));
                    }
                    (s, w)
                }
                Device::Diode { model_name, area, .. } => (
                    if *area != 1.0 {
                        format!("{model_name} ×{area}")
                    } else {
                        model_name.clone()
                    },
                    "",
                ),
                Device::Mosfet { model_name, w, l, .. } => (format!("{model_name} W/L {}/{}", fmt_si(*w, "m"), fmt_si(*l, "m")), ""),
                Device::Vcvs { gain } => (
                    format!(
                        "gain {}",
                        if gain.abs() >= 1e4 {
                            format!("{gain:.0e}")
                        } else {
                            format!("{gain}")
                        }
                    ),
                    "",
                ),
                Device::Vccs { gm } => (format!("gm {}", fmt_si(*gm, "S")), ""),
                Device::Switch { model, model_name, .. } => (format!("{model_name} Ron {}", fmt_si(model.ron, "Ω")), ""),
            };
            ElementInfo {
                name: e.name.clone(),
                kind: kind_name(e).into(),
                nodes: e.nodes.iter().map(|n| nl.nodes[*n].clone()).collect(),
                value,
                wave: wave.into(),
                sw: match &e.dev {
                    Device::Switch { model, .. } => Some([model.ron, model.roff]),
                    _ => None,
                },
                line: e.line,
            }
        })
        .collect()
}

fn analysis_card(a: &Analysis) -> String {
    let f = parser::fmt_spice;
    match a {
        Analysis::Op => ".op".into(),
        Analysis::Dc { src, start, stop, step } => format!(".dc {src} {} {} {}", f(*start), f(*stop), f(*step)),
        Analysis::Tran {
            tstep,
            tstop,
            tstart,
            tmax,
            uic,
        } => {
            let mut s = format!(".tran {} {}", f(*tstep), f(*tstop));
            if *tstart > 0.0 || tmax.is_some() {
                s.push_str(&format!(" {}", f(*tstart)));
            }
            if let Some(m) = tmax {
                s.push_str(&format!(" {}", f(*m)));
            }
            if *uic {
                s.push_str(" uic");
            }
            s
        }
        Analysis::Ac { sweep, n, fstart, fstop } => format!(".ac {} {n} {} {}", format!("{sweep:?}").to_lowercase(), f(*fstart), f(*fstop)),
    }
}

/// Join parse errors into one message (one per line: `line N: ...`).
pub fn errors_text(errs: &[ParseError]) -> String {
    errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")
}

/// Parse + run every analysis in a deck.
pub fn simulate(text: &str, params: &BTreeMap<String, f64>) -> Result<SimResult, String> {
    let nl = parser::parse(text, params).map_err(|e| errors_text(&e))?;
    run_netlist(&nl, text, params)
}

/// Run every analysis of an already parsed netlist.
pub fn run_netlist(nl: &Netlist, text: &str, params: &BTreeMap<String, f64>) -> Result<SimResult, String> {
    let ckt = Ckt::compile(nl);
    let mut warnings = nl.warnings.clone();
    let mut stats = SolverStats {
        unknowns: ckt.size,
        nodes: ckt.nn - 1,
        branches: ckt.nbr,
        ..Default::default()
    };
    let mut res = SimResult {
        title: nl.title.clone(),
        nodes: nl.nodes[1..].to_vec(),
        elements: element_infos(nl),
        params: nl.params.clone(),
        analyses: nl.analyses.iter().map(analysis_card).collect(),
        netlist: parser::with_params(text, params),
        ..Default::default()
    };
    // The DC operating point is always useful (op table, AC bias point).
    let dc_mode = Mode::Dc {
        t: None,
        uic_init: false,
        force_ic: false,
    };
    let wants_op = nl.analyses.iter().any(|a| matches!(a, Analysis::Op | Analysis::Ac { .. }));
    let op_sol = match analysis::operating_point(&ckt, dc_mode, &mut stats, &mut warnings) {
        Ok(s) => Some(s),
        Err(e) => {
            if wants_op {
                return Err(format!(".op: {e}"));
            }
            warnings.push(format!("DC operating point unavailable ({e})"));
            None
        }
    };
    if let Some(s) = &op_sol {
        res.op = Some(analysis::op_result(&ckt, nl, s, dc_mode));
    }
    for a in &nl.analyses {
        match a {
            Analysis::Op => {}
            Analysis::Dc { src, start, stop, step } => {
                if res.dc.is_some() {
                    warnings.push("more than one .dc: keeping the last".into());
                }
                res.dc = Some(analysis::dc_sweep(&ckt, nl, src, *start, *stop, *step, &mut stats, &mut warnings)?);
            }
            Analysis::Tran {
                tstep,
                tstop,
                tstart,
                tmax,
                uic,
            } => {
                if res.tran.is_some() {
                    warnings.push("more than one .tran: keeping the last".into());
                }
                res.tran = Some(analysis::transient(
                    &ckt,
                    nl,
                    *tstep,
                    *tstop,
                    *tstart,
                    *tmax,
                    *uic,
                    &mut stats,
                    &mut warnings,
                )?);
            }
            Analysis::Ac { sweep, n, fstart, fstop } => {
                let op = op_sol.as_ref().ok_or("AC analysis needs a DC operating point")?;
                let f = analysis::ac_freqs(*sweep, *n, *fstart, *fstop);
                if res.ac.is_some() {
                    warnings.push("more than one .ac: keeping the last".into());
                }
                res.ac = Some(analysis::ac(&ckt, op, &f, &mut stats)?);
            }
        }
    }
    warnings.dedup();
    res.warnings = warnings;
    res.stats = stats;
    Ok(res)
}

// ---------------------------------------------------------------------------
// JSON API
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
#[serde(default)]
struct SimulateIn {
    netlist: String,
    params: BTreeMap<String, f64>,
    #[allow(dead_code)]
    compact: bool,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ParseIn {
    netlist: String,
    params: BTreeMap<String, f64>,
}

#[derive(Serialize)]
struct ParseOut {
    ok: bool,
    errors: Vec<ParseError>,
    warnings: Vec<String>,
    title: String,
    nodes: Vec<String>,
    elements: Vec<ElementInfo>,
    analyses: Vec<String>,
    params: BTreeMap<String, f64>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct PresetsIn {
    project: Option<Project>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RunPresetIn {
    id: String,
    params: BTreeMap<String, f64>,
    /// Optional edited deck; the preset's measurements still apply.
    netlist: Option<String>,
    project: Option<Project>,
    #[allow(dead_code)]
    compact: bool,
}

fn parse_api(i: ParseIn) -> ParseOut {
    match parser::parse(&i.netlist, &i.params) {
        Ok(nl) => ParseOut {
            ok: true,
            errors: vec![],
            warnings: nl.warnings.clone(),
            title: nl.title.clone(),
            nodes: nl.nodes[1..].to_vec(),
            elements: element_infos(&nl),
            analyses: nl.analyses.iter().map(analysis_card).collect(),
            params: nl.params.clone(),
        },
        Err(errors) => ParseOut {
            ok: false,
            errors,
            warnings: vec![],
            title: String::new(),
            nodes: vec![],
            elements: vec![],
            analyses: vec![],
            params: BTreeMap::new(),
        },
    }
}

/// JSON API entry point for `circuit.*` methods.
///
/// * `circuit.presets` `{project?}` → preset list (netlist, sliders, probes,
///   schematic layout, Iris tie-in).
/// * `circuit.parse` `{netlist, params?}` → `{ok, errors[{line,message}],
///   nodes, elements, analyses, params, warnings}`.
/// * `circuit.simulate` `{netlist, params?}` → [`SimResult`].
/// * `circuit.run_preset` `{id, params?, netlist?, project?}` → [`SimResult`]
///   with the preset's measurements and annotations.
///
/// `simulate` and `run_preset` accept `"compact": true` to receive waveform
/// arrays as base64 little-endian f32 (f64 for time/frequency) — see [`enc`].
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    let compact = input.get("compact").and_then(|v| v.as_bool()).unwrap_or(false);
    enc::set_compact(compact);
    let out = dispatch_inner(method, input);
    enc::set_compact(false);
    out
}

fn dispatch_inner(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "circuit.presets" => handle(input, |i: PresetsIn| Ok(presets::list(i.project.as_ref()))),
        "circuit.parse" => handle(input, |i: ParseIn| Ok(parse_api(i))),
        "circuit.simulate" => handle(input, |i: SimulateIn| simulate(&i.netlist, &i.params)),
        "circuit.run_preset" => handle(input, |i: RunPresetIn| {
            presets::run(&i.id, &i.params, i.netlist.as_deref(), i.project.as_ref())
        }),
        _ => None,
    }
}

/// Metrics this module contributes to requirement verification.
pub fn metrics(p: &Project) -> Vec<Metric> {
    presets::metrics(p)
}

#[cfg(test)]
mod tests;
