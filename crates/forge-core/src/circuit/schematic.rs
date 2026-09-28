//! Schematic editor back end: turns a drawn schematic (parts with pins on a
//! grid, wire polylines, grounds, net labels) into a SPICE netlist by
//! **connectivity extraction**, and computes, for every drawn wire segment,
//! which element currents flow through it so the web page can animate
//! current on the wires exactly as drawn.
//!
//! ## Connectivity (as in KiCad / EasyEDA netlisters)
//!
//! 1. Every pin, wire vertex, ground and net-label anchor is a point on the
//!    grid (coordinates rounded to 1/1000 of a grid unit).
//! 2. **Union–find** joins: consecutive vertices of a wire; any point lying
//!    on a wire segment (T-junctions, pins touching a wire); points that
//!    coincide (pin on pin).
//! 3. Net labels with the same text are merged; every ground symbol is node
//!    `0`.
//! 4. Each resulting set is a net, named by (in order) ground → `0`, a net
//!    label, the node name an imported preset used, or `n1, n2 …`.
//!
//! ## Current on the wires
//!
//! Within one net the wire segments form a graph whose terminals are pins.
//! Each pin injects a signed element current (SPICE convention: `i(X)`
//! flows *into* pin 1 / the drain, so pin 1 injects `−i`, pin 2 `+i`). On a
//! spanning tree of the net (net-label and ground connections are virtual
//! edges), the current along an edge is the sum of the injections on one
//! side (Kirchhoff's current law), expressed as a list `[(element, sign)]`
//! — the same `flow` format the preset schematics use (see [`super::presets`]).
//!
//! References: R. E. Tarjan, "Efficiency of a good but not linear set union
//! algorithm", *J. ACM* 22 (1975); KiCad and EasyEDA netlist documentation;
//! ngspice manual (element cards).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use super::parser::{self, Analysis};
use super::presets;
use crate::project::Project;

/// One placed symbol.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct SchPart {
    /// Unique id within the schematic.
    pub id: String,
    /// resistor, capacitor, inductor, vsource, isource, battery, diode, led,
    /// nmos, pmos, switch, opamp, ground, label, probe, text
    pub kind: String,
    /// Reference designator, e.g. `R1` (also the SPICE element name).
    pub name: String,
    pub x: f64,
    pub y: f64,
    /// Rotation in 90° clockwise steps (0: pin 1 up / pin 2 down).
    pub rot: u8,
    /// Mirror left–right before rotating.
    pub flip: bool,
    /// Main value with engineering suffix, e.g. `10k`, `100n`, `5`.
    pub value: String,
    /// Kind-specific properties (source waveform, model parameters …).
    pub props: BTreeMap<String, String>,
    /// Imported parts: raw SPICE card(s) with `<p0>`, `<p1>` … for the pins.
    pub template: Option<String>,
    /// Imported parts: the node name each pin had in the preset deck.
    pub pin_hint: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct SchWire {
    pub pts: Vec<[f64; 2]>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AnalysisSpec {
    pub op: bool,
    pub tran: bool,
    pub tstep: String,
    pub tstop: String,
    pub tmax: String,
    pub uic: bool,
    pub ac: bool,
    pub ac_points: u32,
    pub fstart: String,
    pub fstop: String,
    pub dc: bool,
    pub dc_source: String,
    pub dc_start: String,
    pub dc_stop: String,
    pub dc_step: String,
}

impl Default for AnalysisSpec {
    fn default() -> Self {
        AnalysisSpec {
            op: true,
            tran: true,
            tstep: "10u".into(),
            tstop: "5m".into(),
            tmax: String::new(),
            uic: false,
            ac: false,
            ac_points: 20,
            fstart: "10".into(),
            fstop: "1meg".into(),
            dc: false,
            dc_source: "V1".into(),
            dc_start: "0".into(),
            dc_stop: "5".into(),
            dc_step: "0.05".into(),
        }
    }
}

/// A drawn schematic (the editor's save file).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Schematic {
    pub title: String,
    pub parts: Vec<SchPart>,
    pub wires: Vec<SchWire>,
    pub analysis: AnalysisSpec,
    /// Extra deck lines (`.param`, `.model`, `.ic`, elements without a symbol …).
    pub directives: Vec<String>,
}

/// A pin of a placed part in schematic coordinates.
#[derive(Clone, Debug, Serialize)]
pub struct PinPos {
    pub name: String,
    pub x: f64,
    pub y: f64,
}

/// Local pin offsets (rotation 0) of each symbol kind.
pub fn local_pins(kind: &str) -> Vec<(&'static str, f64, f64)> {
    match kind {
        "nmos" | "pmos" => vec![("top", 0.0, -2.0), ("g", -2.0, 0.0), ("bottom", 0.0, 2.0)],
        "opamp" => vec![("in+", -3.0, -1.0), ("in-", -3.0, 1.0), ("out", 3.0, 0.0)],
        "ground" | "label" | "probe" => vec![("n", 0.0, 0.0)],
        "text" => vec![],
        "diode" | "led" => vec![("a", 0.0, -2.0), ("k", 0.0, 2.0)],
        "vsource" | "battery" | "isource" => vec![("+", 0.0, -2.0), ("-", 0.0, 2.0)],
        _ => vec![("1", 0.0, -2.0), ("2", 0.0, 2.0)],
    }
}

/// Rotate/flip a local offset: `rot` quarter turns clockwise (y down).
pub fn transform(dx: f64, dy: f64, rot: u8, flip: bool) -> (f64, f64) {
    let (mut x, mut y) = (if flip { -dx } else { dx }, dy);
    for _ in 0..(rot % 4) {
        let nx = -y;
        let ny = x;
        x = nx;
        y = ny;
    }
    (x, y)
}

pub fn pins_of(p: &SchPart) -> Vec<PinPos> {
    local_pins(&p.kind)
        .into_iter()
        .map(|(n, dx, dy)| {
            let (tx, ty) = transform(dx, dy, p.rot, p.flip);
            PinPos {
                name: n.into(),
                x: p.x + tx,
                y: p.y + ty,
            }
        })
        .collect()
}

type Key = (i64, i64);

fn key(x: f64, y: f64) -> Key {
    ((x * 1000.0).round() as i64, (y * 1000.0).round() as i64)
}

struct Dsu {
    parent: Vec<usize>,
}

impl Dsu {
    fn new(n: usize) -> Self {
        Dsu { parent: (0..n).collect() }
    }
    fn find(&mut self, mut a: usize) -> usize {
        while self.parent[a] != a {
            self.parent[a] = self.parent[self.parent[a]];
            a = self.parent[a];
        }
        a
    }
    fn union(&mut self, a: usize, b: usize) -> bool {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return false;
        }
        self.parent[rb.max(ra)] = rb.min(ra);
        true
    }
}

/// Is `p` on the closed segment a–b (within a small tolerance)?
fn on_segment(p: Key, a: Key, b: Key) -> bool {
    let (px, py) = (p.0 as i128, p.1 as i128);
    let (ax, ay) = (a.0 as i128, a.1 as i128);
    let (bx, by) = (b.0 as i128, b.1 as i128);
    let cross = (bx - ax) * (py - ay) - (by - ay) * (px - ax);
    if cross.abs() > 2 * ((bx - ax).abs() + (by - ay).abs()).max(1) {
        return false;
    }
    px >= ax.min(bx) && px <= ax.max(bx) && py >= ay.min(by) && py <= ay.max(by)
}

/// A drawn wire piece with the currents flowing along it (first → last point).
#[derive(Clone, Debug, Serialize)]
pub struct FlowWire {
    pub net: String,
    pub pts: Vec<[f64; 2]>,
    pub flow: Vec<(String, f64)>,
}

#[derive(Clone, Debug, Serialize)]
pub struct NetInfo {
    pub name: String,
    /// `R1.1`, `M1.g` …
    pub pins: Vec<String>,
    pub labels: Vec<String>,
    pub probe: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PartNets {
    pub id: String,
    pub name: String,
    pub pins: Vec<PinPos>,
    /// Net of each pin.
    pub nets: Vec<String>,
    /// SPICE elements emitted for this part (primary first).
    pub elements: Vec<String>,
    /// Netlist line numbers (1-based) of those elements.
    pub lines: Vec<usize>,
}

/// Result of netlist extraction.
#[derive(Clone, Debug, Serialize)]
pub struct Extracted {
    pub netlist: String,
    pub nets: Vec<NetInfo>,
    pub parts: Vec<PartNets>,
    pub wires: Vec<FlowWire>,
    /// Net of each drawn wire (same order as `Schematic::wires`).
    pub wire_nets: Vec<String>,
    pub junctions: Vec<[f64; 2]>,
    /// Nets marked by probe symbols (`v(net)`).
    pub probes: Vec<String>,
    /// Node count excluding ground.
    pub node_count: usize,
    pub warnings: Vec<String>,
}

fn clean_value(v: &str) -> String {
    v.trim().replace('µ', "u").replace('Ω', "").replace("ohm", "").replace(' ', "")
}

/// A SPICE number with an engineering suffix (`10k`, `4.7u`, `1meg`; `m` is milli).
pub fn num(s: &str) -> Option<f64> {
    let t = clean_value(s).to_ascii_lowercase();
    let end = t
        .char_indices()
        .find(|(i, c)| {
            !(c.is_ascii_digit()
                || *c == '.'
                || ((*c == '-' || *c == '+') && (*i == 0 || t[..*i].ends_with('e')))
                || (*c == 'e' && t[i + 1..].starts_with(|d: char| d.is_ascii_digit() || d == '-' || d == '+')))
        })
        .map(|(i, _)| i)
        .unwrap_or(t.len());
    let m: f64 = t[..end].parse().ok()?;
    let rest = &t[end..];
    let k = if rest.starts_with("meg") {
        1e6
    } else {
        match rest.chars().next() {
            None => 1.0,
            Some('t') => 1e12,
            Some('g') => 1e9,
            Some('k') => 1e3,
            Some('m') => 1e-3,
            Some('u') => 1e-6,
            Some('n') => 1e-9,
            Some('p') => 1e-12,
            Some('f') => 1e-15,
            Some(c) if c.is_alphabetic() => 1.0,
            _ => return None,
        }
    };
    Some(m * k)
}

fn prop<'a>(p: &'a SchPart, k: &str, d: &'a str) -> String {
    p.props
        .get(k)
        .map(|s| clean_value(s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| d.to_string())
}

fn with_letter(name: &str, letter: char) -> String {
    let n: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
    if n.chars().next().map(|c| c.eq_ignore_ascii_case(&letter)).unwrap_or(false) {
        n
    } else {
        format!("{letter}{n}")
    }
}

fn source_spec(p: &SchPart) -> String {
    let wave = prop(p, "wave", "dc");
    let dc = clean_value(if p.value.trim().is_empty() { "0" } else { &p.value });
    let mut s = match wave.as_str() {
        "pulse" => format!(
            "PULSE({} {} {} {} {} {} {})",
            prop(p, "v1", "0"),
            prop(p, "v2", &dc),
            prop(p, "td", "0"),
            prop(p, "tr", "1n"),
            prop(p, "tf", "1n"),
            prop(p, "pw", "0.5m"),
            prop(p, "per", "1m")
        ),
        "sin" => format!(
            "DC {} SIN({} {} {})",
            prop(p, "vo", "0"),
            prop(p, "vo", "0"),
            prop(p, "va", &dc),
            prop(p, "freq", "1k")
        ),
        _ => format!("DC {dc}"),
    };
    let ac = prop(p, "ac", "");
    if !ac.is_empty() && ac != "0" {
        s.push_str(&format!(" AC {ac}"));
    }
    s
}

/// SPICE cards and models for one part given its pin nets.
/// `(element name, card)` pairs and `.model` lines emitted for one part.
type Emitted = (Vec<(String, String)>, Vec<String>);

fn emit(p: &SchPart, nets: &[String]) -> Result<Emitted, String> {
    let n = |i: usize| nets.get(i).cloned().unwrap_or_else(|| "0".into());
    if let Some(t) = &p.template {
        // imported card(s): substitute the pins
        let mut cards = Vec::new();
        for line in t.lines().filter(|l| !l.trim().is_empty()) {
            let mut s = line.to_string();
            for (i, net) in nets.iter().enumerate() {
                s = s.replace(&format!("<p{i}>"), net);
            }
            let el = s.split_whitespace().next().unwrap_or("").to_string();
            cards.push((el, s));
        }
        return Ok((cards, vec![]));
    }
    let v = clean_value(&p.value);
    let mut models = Vec::new();
    let cards: Vec<(String, String)> = match p.kind.as_str() {
        "resistor" => {
            let e = with_letter(&p.name, 'R');
            vec![(
                e.clone(),
                format!("{e} {} {} {}", n(0), n(1), if v.is_empty() { "1k".into() } else { v }),
            )]
        }
        "capacitor" | "inductor" => {
            let (l, d) = if p.kind == "capacitor" { ('C', "1u") } else { ('L', "10u") };
            let e = with_letter(&p.name, l);
            let ic = prop(p, "ic", "");
            let ic = if ic.is_empty() { String::new() } else { format!(" IC={ic}") };
            vec![(
                e.clone(),
                format!("{e} {} {} {}{ic}", n(0), n(1), if v.is_empty() { d.into() } else { v }),
            )]
        }
        "vsource" | "battery" => {
            let e = with_letter(&p.name, 'V');
            vec![(e.clone(), format!("{e} {} {} {}", n(0), n(1), source_spec(p)))]
        }
        "isource" => {
            let e = with_letter(&p.name, 'I');
            vec![(e.clone(), format!("{e} {} {} {}", n(0), n(1), source_spec(p)))]
        }
        "diode" | "led" => {
            let e = with_letter(&p.name, 'D');
            let m = format!("D_{}", e.to_uppercase());
            let (is, nn, rs) = if p.kind == "led" {
                ("2.2e-18", "2", "2")
            } else {
                ("2.52n", "1.752", "0.568")
            };
            models.push(format!(
                ".model {m} D(IS={} N={} RS={} BV={})",
                prop(p, "is", is),
                prop(p, "n", nn),
                prop(p, "rs", rs),
                prop(p, "bv", "100")
            ));
            vec![(e.clone(), format!("{e} {} {} {m}", n(0), n(1)))]
        }
        "nmos" | "pmos" => {
            let e = with_letter(&p.name, 'M');
            let pm = p.kind == "pmos";
            let m = format!("M_{}", e.to_uppercase());
            let (d, g, s) = if pm { (n(2), n(1), n(0)) } else { (n(0), n(1), n(2)) };
            models.push(format!(
                ".model {m} {}(LEVEL=1 VTO={} KP={} LAMBDA={})",
                if pm { "PMOS" } else { "NMOS" },
                prop(p, "vto", if pm { "-0.8" } else { "0.8" }),
                prop(p, "kp", if pm { "40u" } else { "100u" }),
                prop(p, "lambda", "0.01")
            ));
            vec![(
                e.clone(),
                format!("{e} {d} {g} {s} {s} {m} W={} L={}", prop(p, "w", "1m"), prop(p, "l", "1u")),
            )]
        }
        "switch" => {
            let e = with_letter(&p.name, 'S');
            let ctl = format!("_{}ctl", e.to_lowercase());
            let m = format!("SW_{}", e.to_uppercase());
            models.push(format!(
                ".model {m} SW(RON={} ROFF={} VT=0.5 VH=0)",
                prop(p, "ron", "10m"),
                prop(p, "roff", "10meg")
            ));
            let state = prop(p, "state", "closed");
            let drive = match state.as_str() {
                "pwm" => {
                    let f = num(&prop(p, "freq", "100k"))
                        .filter(|f| *f > 0.0)
                        .ok_or("switch PWM frequency must be a positive number")?;
                    let duty = num(&prop(p, "duty", "0.5")).unwrap_or(0.5).clamp(0.0, 1.0);
                    let per = 1.0 / f;
                    let edge = (per * 1e-3).min(1e-9);
                    format!(
                        "PULSE(0 1 0 {} {} {} {})",
                        fmt_num(edge),
                        fmt_num(edge),
                        fmt_num((duty * per - edge).max(edge)),
                        fmt_num(per)
                    )
                }
                "open" => "DC 0".into(),
                _ => "DC 1".into(),
            };
            let vc = format!("V{}CTL", e.to_uppercase());
            vec![
                (e.clone(), format!("{e} {} {} {ctl} 0 {m}", n(0), n(1))),
                (vc.clone(), format!("{vc} {ctl} 0 {drive}")),
            ]
        }
        "opamp" => {
            // single-pole macro-model: E (gain A0) → R·C pole at GBW/A0 → unity buffer
            let base: String = p
                .name
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_uppercase();
            let a0v = num(&prop(p, "a0", "100k"))
                .filter(|v| *v > 0.0)
                .ok_or("op-amp gain A0 must be positive")?;
            let gbw = num(&prop(p, "gbw", "1meg"))
                .filter(|v| *v > 0.0)
                .ok_or("op-amp GBW must be positive")?;
            let a0 = fmt_num(a0v);
            let cp = fmt_num(a0v / (2.0 * std::f64::consts::PI * 1e3 * gbw));
            let x = format!("_{}x", base.to_lowercase());
            let y = format!("_{}y", base.to_lowercase());
            vec![
                (format!("EB{base}"), format!("EB{base} {} 0 {y} 0 1", n(2))),
                (format!("EA{base}"), format!("EA{base} {x} 0 {} {} {a0}", n(0), n(1))),
                (format!("RP{base}"), format!("RP{base} {x} {y} 1k")),
                (format!("CP{base}"), format!("CP{base} {y} 0 {cp}")),
            ]
        }
        _ => vec![],
    };
    Ok((cards, models))
}

/// Pin current injections `(element, sign)` for the flow animation.
fn pin_injection(p: &SchPart, elements: &[String], pin: usize) -> Vec<(String, f64)> {
    let el = match elements.first() {
        Some(e) => e.clone(),
        None => return vec![],
    };
    if p.template.is_some() && matches!(p.kind.as_str(), "opamp" | "vcvs" | "vccs") {
        return vec![];
    }
    match p.kind.as_str() {
        "nmos" | "pmos" => {
            // i(M) flows into the drain: nmos drain = top (pin 0), pmos drain = bottom (pin 2)
            let drain = if p.kind == "nmos" { 0 } else { 2 };
            let source = 2 - drain;
            if pin == drain {
                vec![(el, -1.0)]
            } else if pin == source {
                vec![(el, 1.0)]
            } else {
                vec![]
            }
        }
        "opamp" => {
            if pin == 2 {
                vec![(el, -1.0)]
            } else {
                vec![]
            }
        }
        "ground" | "label" | "probe" | "text" => vec![],
        _ => {
            if pin == 0 {
                vec![(el, -1.0)]
            } else if pin == 1 {
                vec![(el, 1.0)]
            } else {
                vec![]
            }
        }
    }
}

fn analysis_cards(a: &AnalysisSpec) -> Vec<String> {
    let mut v = Vec::new();
    if a.op {
        v.push(".op".into());
    }
    if a.dc && !a.dc_source.trim().is_empty() {
        v.push(format!(
            ".dc {} {} {} {}",
            a.dc_source.trim(),
            clean_value(&a.dc_start),
            clean_value(&a.dc_stop),
            clean_value(&a.dc_step)
        ));
    }
    if a.tran {
        let mut s = format!(".tran {} {}", clean_value(&a.tstep), clean_value(&a.tstop));
        if !a.tmax.trim().is_empty() {
            s.push_str(&format!(" 0 {}", clean_value(&a.tmax)));
        }
        if a.uic {
            s.push_str(" uic");
        }
        v.push(s);
    }
    if a.ac {
        v.push(format!(
            ".ac dec {} {} {}",
            a.ac_points.max(1),
            clean_value(&a.fstart),
            clean_value(&a.fstop)
        ));
    }
    v
}

fn sanitize_net(s: &str) -> String {
    let t: String = s
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if t == "gnd" {
        "0".into()
    } else {
        t
    }
}

/// Connectivity extraction and netlist generation.
pub fn extract(s: &Schematic) -> Result<Extracted, String> {
    let mut warnings = Vec::new();
    // ---- collect points ----------------------------------------------------
    let mut idx: HashMap<Key, usize> = HashMap::new();
    let mut coords: Vec<[f64; 2]> = Vec::new();
    let point = |x: f64, y: f64, idx: &mut HashMap<Key, usize>, coords: &mut Vec<[f64; 2]>| -> usize {
        let k = key(x, y);
        *idx.entry(k).or_insert_with(|| {
            coords.push([x, y]);
            coords.len() - 1
        })
    };
    let part_pins: Vec<Vec<PinPos>> = s.parts.iter().map(pins_of).collect();
    let mut pin_pt: Vec<Vec<usize>> = Vec::new();
    for pins in &part_pins {
        pin_pt.push(pins.iter().map(|q| point(q.x, q.y, &mut idx, &mut coords)).collect());
    }
    let mut wire_pts: Vec<Vec<usize>> = Vec::new();
    for w in &s.wires {
        let mut v: Vec<usize> = Vec::new();
        for q in &w.pts {
            let i = point(q[0], q[1], &mut idx, &mut coords);
            if v.last() != Some(&i) {
                v.push(i);
            }
        }
        wire_pts.push(v);
    }
    let np = coords.len();
    let keys: Vec<Key> = coords.iter().map(|c| key(c[0], c[1])).collect();
    // segments split at every point lying on them
    let mut segs: Vec<(usize, usize)> = Vec::new();
    for v in &wire_pts {
        for w in v.windows(2) {
            let (a, b) = (w[0], w[1]);
            let mut on: Vec<(i128, usize)> = (0..np)
                .filter(|&i| on_segment(keys[i], keys[a], keys[b]))
                .map(|i| {
                    let d = (keys[i].0 - keys[a].0).abs() as i128 + (keys[i].1 - keys[a].1).abs() as i128;
                    (d, i)
                })
                .collect();
            on.sort();
            on.dedup_by_key(|x| x.1);
            for pair in on.windows(2) {
                if pair[0].1 != pair[1].1 {
                    segs.push((pair[0].1, pair[1].1));
                }
            }
        }
    }
    let mut dsu = Dsu::new(np);
    for &(a, b) in &segs {
        dsu.union(a, b);
    }
    // labels and grounds (virtual connections)
    let mut virt: Vec<(usize, usize)> = Vec::new();
    let mut label_first: BTreeMap<String, usize> = BTreeMap::new();
    let mut ground_first: Option<usize> = None;
    for (pi, p) in s.parts.iter().enumerate() {
        match p.kind.as_str() {
            "label" => {
                let name = sanitize_net(if p.value.trim().is_empty() { &p.name } else { &p.value });
                if name.is_empty() {
                    continue;
                }
                let pt = pin_pt[pi][0];
                if name == "0" {
                    match ground_first {
                        Some(g) => virt.push((g, pt)),
                        None => ground_first = Some(pt),
                    }
                } else if let Some(&f) = label_first.get(&name) {
                    virt.push((f, pt));
                } else {
                    label_first.insert(name, pt);
                }
            }
            "ground" => {
                let pt = pin_pt[pi][0];
                match ground_first {
                    Some(g) => virt.push((g, pt)),
                    None => ground_first = Some(pt),
                }
            }
            _ => {}
        }
    }
    for &(a, b) in &virt {
        dsu.union(a, b);
    }

    // ---- name the nets ------------------------------------------------------
    let roots: Vec<usize> = (0..np).map(|i| dsu.find(i)).collect();
    let mut net_name: BTreeMap<usize, String> = BTreeMap::new();
    if let Some(g) = ground_first {
        net_name.insert(roots[g], "0".into());
    }
    let mut label_sets: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    for (pi, p) in s.parts.iter().enumerate() {
        if p.kind == "label" {
            let name = sanitize_net(if p.value.trim().is_empty() { &p.name } else { &p.value });
            if !name.is_empty() {
                label_sets.entry(roots[pin_pt[pi][0]]).or_default().insert(name);
            }
        }
    }
    for (r, names) in &label_sets {
        if names.len() > 1 {
            warnings.push(format!(
                "net labels {} are wired together (shorted)",
                names.iter().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
        if names.contains("0") {
            net_name.insert(*r, "0".into());
        } else if !net_name.contains_key(r) {
            net_name.insert(*r, names.iter().next().unwrap().clone());
        }
    }
    let mut used: BTreeSet<String> = net_name.values().cloned().collect();
    // imported node-name hints
    for (pi, p) in s.parts.iter().enumerate() {
        for (k, hint) in p.pin_hint.iter().enumerate() {
            let Some(&pt) = pin_pt[pi].get(k) else { continue };
            let r = roots[pt];
            let h = sanitize_net(hint);
            if h.is_empty() || net_name.contains_key(&r) {
                continue;
            }
            if h == "0" {
                if ground_first.is_none() {
                    net_name.insert(r, "0".into());
                }
                continue;
            }
            if used.insert(h.clone()) {
                net_name.insert(r, h);
            }
        }
    }
    let mut counter = 0;
    for (pi, _) in s.parts.iter().enumerate() {
        for &pt in &pin_pt[pi] {
            let r = roots[pt];
            if net_name.contains_key(&r) {
                continue;
            }
            loop {
                counter += 1;
                let n = format!("n{counter}");
                if used.insert(n.clone()) {
                    net_name.insert(r, n);
                    break;
                }
            }
        }
    }
    let name_of = |pt: usize, net_name: &BTreeMap<usize, String>| net_name.get(&roots[pt]).cloned().unwrap_or_else(|| "?".into());

    // ---- floating pins -----------------------------------------------------
    let mut touch: HashMap<usize, usize> = HashMap::new();
    for (pi, p) in s.parts.iter().enumerate() {
        if matches!(p.kind.as_str(), "text") {
            continue;
        }
        for &pt in &pin_pt[pi] {
            *touch.entry(roots[pt]).or_insert(0) += 1;
        }
    }
    for (pi, p) in s.parts.iter().enumerate() {
        if matches!(p.kind.as_str(), "ground" | "label" | "probe" | "text") {
            continue;
        }
        for (k, &pt) in pin_pt[pi].iter().enumerate() {
            if touch.get(&roots[pt]).copied().unwrap_or(0) <= 1 {
                warnings.push(format!("{} pin {} is not connected to anything", p.name, part_pins[pi][k].name));
            }
        }
    }

    // ---- emit the deck -----------------------------------------------------
    let title = if s.title.trim().is_empty() {
        "Schematic".to_string()
    } else {
        s.title.trim().replace('\n', " ")
    };
    let mut lines: Vec<String> = vec![title, "* netlist extracted from the Protoforge schematic editor".into()];
    let mut models: Vec<String> = Vec::new();
    let mut parts_out = Vec::new();
    let mut names_seen: BTreeSet<String> = BTreeSet::new();
    let mut probes = Vec::new();
    for (pi, p) in s.parts.iter().enumerate() {
        let nets: Vec<String> = pin_pt[pi].iter().map(|&pt| name_of(pt, &net_name)).collect();
        if p.kind == "probe" {
            if let Some(n) = nets.first() {
                if n != "0" && !probes.contains(n) {
                    probes.push(n.clone());
                }
            }
        }
        let (cards, mut m) = emit(p, &nets)?;
        let mut els = Vec::new();
        let mut lns = Vec::new();
        for (el, card) in cards {
            if !el.is_empty() && !names_seen.insert(el.to_ascii_lowercase()) {
                return Err(format!("duplicate element name `{el}` — rename one of the parts"));
            }
            lines.push(card);
            els.push(el);
            lns.push(lines.len());
        }
        models.append(&mut m);
        parts_out.push(PartNets {
            id: p.id.clone(),
            name: p.name.clone(),
            pins: part_pins[pi].clone(),
            nets,
            elements: els,
            lines: lns,
        });
    }
    lines.extend(models);
    for d in &s.directives {
        let t = d.trim();
        if t.is_empty() || t.eq_ignore_ascii_case(".end") {
            continue;
        }
        lines.push(t.to_string());
    }
    lines.extend(analysis_cards(&s.analysis));
    lines.push(".end".into());
    let netlist = lines.join("\n") + "\n";

    // ---- nets summary ------------------------------------------------------
    let mut nets_map: BTreeMap<String, NetInfo> = BTreeMap::new();
    for (pi, p) in s.parts.iter().enumerate() {
        for (k, &pt) in pin_pt[pi].iter().enumerate() {
            let n = name_of(pt, &net_name);
            let e = nets_map.entry(n.clone()).or_insert_with(|| NetInfo {
                name: n.clone(),
                pins: vec![],
                labels: vec![],
                probe: false,
            });
            match p.kind.as_str() {
                "label" => e.labels.push(p.value.clone()),
                "probe" => e.probe = true,
                "ground" => {}
                _ => e.pins.push(format!("{}.{}", p.name, part_pins[pi][k].name)),
            }
        }
    }
    let node_count = nets_map.keys().filter(|k| k.as_str() != "0").count();

    // ---- current flow on the wires ------------------------------------------
    let wires = flow_wires(s, &parts_out, &pin_pt, &segs, &virt, &roots, &net_name, &coords);
    // junction dots: ≥ 3 wire ends / pins meeting at a point
    let mut deg: HashMap<usize, usize> = HashMap::new();
    for &(a, b) in &segs {
        *deg.entry(a).or_insert(0) += 1;
        *deg.entry(b).or_insert(0) += 1;
    }
    for (pi, p) in s.parts.iter().enumerate() {
        if matches!(p.kind.as_str(), "ground" | "label" | "probe" | "text") {
            continue;
        }
        for &pt in &pin_pt[pi] {
            *deg.entry(pt).or_insert(0) += 1;
        }
    }
    let mut junctions: Vec<[f64; 2]> = deg.iter().filter(|(_, d)| **d >= 3).map(|(i, _)| coords[*i]).collect();
    junctions.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let wire_nets = wire_pts
        .iter()
        .map(|v| v.first().map(|&pt| name_of(pt, &net_name)).unwrap_or_default())
        .collect();
    Ok(Extracted {
        netlist,
        nets: nets_map.into_values().collect(),
        parts: parts_out,
        wires,
        wire_nets,
        junctions,
        probes,
        node_count,
        warnings,
    })
}

#[allow(clippy::too_many_arguments)]
fn flow_wires(
    s: &Schematic,
    parts: &[PartNets],
    pin_pt: &[Vec<usize>],
    segs: &[(usize, usize)],
    virt: &[(usize, usize)],
    roots: &[usize],
    net_name: &BTreeMap<usize, String>,
    coords: &[[f64; 2]],
) -> Vec<FlowWire> {
    let np = coords.len();
    // injections per point
    let mut inj: Vec<BTreeMap<String, f64>> = vec![BTreeMap::new(); np];
    for (pi, p) in s.parts.iter().enumerate() {
        for (k, &pt) in pin_pt[pi].iter().enumerate() {
            for (el, sg) in pin_injection(p, &parts[pi].elements, k) {
                *inj[pt].entry(el).or_insert(0.0) += sg;
            }
        }
    }
    // spanning forest: real segments first, then virtual links
    let mut dsu = Dsu::new(np);
    let mut adj: Vec<Vec<(usize, usize)>> = vec![vec![]; np]; // (neighbour, edge id)
    let mut edges: Vec<(usize, usize, bool)> = Vec::new(); // (a, b, real)
    for &(a, b) in segs {
        let id = edges.len();
        edges.push((a, b, true));
        if dsu.union(a, b) {
            adj[a].push((b, id));
            adj[b].push((a, id));
        }
    }
    for &(a, b) in virt {
        let id = edges.len();
        edges.push((a, b, false));
        if dsu.union(a, b) {
            adj[a].push((b, id));
            adj[b].push((a, id));
        }
    }
    // root each tree, accumulate subtree injections
    let mut visited = vec![false; np];
    let mut sub: Vec<BTreeMap<String, f64>> = inj.clone();
    let mut edge_flow: Vec<Option<(usize, BTreeMap<String, f64>)>> = vec![None; edges.len()]; // (from-point, flow from→parent)
    for start in 0..np {
        if visited[start] {
            continue;
        }
        // iterative DFS: order + parent edge
        let mut order = Vec::new();
        let mut parent: Vec<(usize, usize)> = Vec::new();
        let mut stack = vec![(start, usize::MAX, usize::MAX)];
        while let Some((v, par, eid)) = stack.pop() {
            if visited[v] {
                continue;
            }
            visited[v] = true;
            order.push(v);
            parent.push((par, eid));
            for &(w, e) in &adj[v] {
                if !visited[w] {
                    stack.push((w, v, e));
                }
            }
        }
        for k in (0..order.len()).rev() {
            let v = order[k];
            let (par, eid) = parent[k];
            if par == usize::MAX {
                continue;
            }
            let m = sub[v].clone();
            for (el, sg) in &m {
                *sub[par].entry(el.clone()).or_insert(0.0) += sg;
            }
            edge_flow[eid] = Some((v, m));
        }
    }
    let flow_ab = |eid: usize| -> BTreeMap<String, f64> {
        let (a, _, _) = edges[eid];
        match &edge_flow[eid] {
            Some((from, m)) => {
                if *from == a {
                    m.clone()
                } else {
                    m.iter().map(|(k, v)| (k.clone(), -v)).collect()
                }
            }
            None => BTreeMap::new(),
        }
    };
    // merge chains through plain wire bends (degree 2, no pins)
    let mut real_adj: Vec<Vec<usize>> = vec![vec![]; np];
    for (eid, &(a, b, real)) in edges.iter().enumerate() {
        if real {
            real_adj[a].push(eid);
            real_adj[b].push(eid);
        }
    }
    let is_pin: Vec<bool> = {
        let mut v = vec![false; np];
        for pts in pin_pt {
            for &p in pts {
                v[p] = true;
            }
        }
        v
    };
    let passthrough = |v: usize| real_adj[v].len() == 2 && !is_pin[v];
    let mut used = vec![false; edges.len()];
    let mut out = Vec::new();
    let clean = |m: BTreeMap<String, f64>| -> Vec<(String, f64)> { m.into_iter().filter(|(_, v)| v.abs() > 1e-9).collect() };
    for (eid, &(a, _, real)) in edges.iter().enumerate() {
        if !real || used[eid] {
            continue;
        }
        // walk back to a chain start
        let (mut cur_e, mut cur_v) = (eid, a);
        let mut guard = 0;
        while passthrough(cur_v) && guard < np + 2 {
            guard += 1;
            let other = real_adj[cur_v].iter().copied().find(|&e| e != cur_e).unwrap();
            if other == eid {
                break;
            }
            let (x, y, _) = edges[other];
            cur_v = if x == cur_v { y } else { x };
            cur_e = other;
        }
        // walk forward from cur_v through cur_e
        let mut pts = vec![coords[cur_v]];
        let (x, y, _) = edges[cur_e];
        let first_from = cur_v;
        let mut flow = flow_ab(cur_e);
        if x != first_from {
            flow = flow.into_iter().map(|(k, v)| (k, -v)).collect();
        }
        let _ = y;
        let (mut e, mut v) = (cur_e, cur_v);
        loop {
            used[e] = true;
            let (x, y, _) = edges[e];
            let nv = if x == v { y } else { x };
            pts.push(coords[nv]);
            v = nv;
            if !passthrough(v) {
                break;
            }
            let Some(next) = real_adj[v].iter().copied().find(|&q| q != e && !used[q]) else {
                break;
            };
            e = next;
        }
        let net = net_name.get(&roots[first_from]).cloned().unwrap_or_default();
        out.push(FlowWire {
            net,
            pts,
            flow: clean(flow),
        });
    }
    out
}

// ---------------------------------------------------------------------------
// Import from presets and starter examples
// ---------------------------------------------------------------------------

fn node_positions(card: &str) -> std::ops::Range<usize> {
    match card.chars().next().map(|c| c.to_ascii_uppercase()) {
        Some('M') | Some('E') | Some('G') | Some('S') => 1..5,
        _ => 1..3,
    }
}

/// Logical SPICE card starting at a 1-based line (joins `+` continuations).
fn card_at(text: &str, line: usize) -> String {
    let ls: Vec<&str> = text.lines().collect();
    if line == 0 || line > ls.len() {
        return String::new();
    }
    let mut s = ls[line - 1].trim().to_string();
    for l in ls.iter().skip(line) {
        let t = l.trim_start();
        if let Some(rest) = t.strip_prefix('+') {
            s.push(' ');
            s.push_str(rest.trim());
        } else {
            break;
        }
    }
    s
}

fn templated(card: &str, pins: &[String]) -> String {
    let toks: Vec<&str> = card.split_whitespace().collect();
    let range = node_positions(card);
    toks.iter()
        .enumerate()
        .map(|(i, t)| {
            if range.contains(&i) {
                if let Some(k) = pins.iter().position(|p| p.eq_ignore_ascii_case(t)) {
                    return format!("<p{k}>");
                }
            }
            t.to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn fmt_num(v: f64) -> String {
    parser::fmt_spice(v)
}

/// Convert a Circuit Lab preset (deck + hand-drawn layout) into an editable schematic.
pub fn from_preset(id: &str, project: Option<&Project>) -> Result<Schematic, String> {
    let list = presets::list(project);
    let pr = list.iter().find(|p| p.id == id).ok_or_else(|| format!("unknown preset `{id}`"))?;
    let nl = parser::parse(&pr.netlist, &BTreeMap::new()).map_err(|e| super::errors_text(&e))?;
    let mut parts = Vec::new();
    let mut covered: BTreeSet<String> = BTreeSet::new();
    for (k, lp) in pr.layout.parts.iter().enumerate() {
        let (kind, rot, flip) = match lp.sym.as_str() {
            "nmos" | "pmos" => (lp.sym.clone(), 0u8, lp.dir == "left"),
            "opamp" => ("opamp".into(), 0, false),
            s => {
                let rot = match lp.dir.as_str() {
                    "left" => 1,
                    "up" => 2,
                    "right" => 3,
                    _ => 0,
                };
                (s.to_string(), rot, false)
            }
        };
        // pin node names in the editor's pin order
        let hints: Vec<String> = match kind.as_str() {
            "nmos" | "pmos" => {
                let find = |n: &str| lp.pins.iter().find(|p| p.name == n).map(|p| p.node.clone()).unwrap_or_default();
                let (top, bot) = if kind == "nmos" { ("d", "s") } else { ("s", "d") };
                vec![find(top), find("g"), find(bot)]
            }
            _ => lp.pins.iter().map(|p| p.node.clone()).collect(),
        };
        let mut cards = Vec::new();
        for m in &lp.members {
            if let Some(e) = nl.elements.iter().find(|e| e.name.eq_ignore_ascii_case(m)) {
                cards.push(templated(&card_at(&pr.netlist, e.line), &hints));
                covered.insert(e.name.to_ascii_lowercase());
            }
        }
        let kind = match kind.as_str() {
            "vcvs" | "vccs" => kind,
            k => k.to_string(),
        };
        parts.push(SchPart {
            id: format!("p{k}"),
            kind,
            name: lp.el.clone(),
            x: lp.x,
            y: lp.y,
            rot,
            flip,
            value: nl
                .elements
                .iter()
                .find(|e| e.name.eq_ignore_ascii_case(&lp.el))
                .map(|e| {
                    card_at(&pr.netlist, e.line)
                        .split_whitespace()
                        .skip(node_positions(&e.name).end)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default(),
            props: BTreeMap::new(),
            template: Some(cards.join("\n")),
            pin_hint: hints,
        });
    }
    for (k, g) in pr.layout.grounds.iter().enumerate() {
        parts.push(SchPart {
            id: format!("g{k}"),
            kind: "ground".into(),
            name: format!("GND{}", k + 1),
            x: g[0],
            y: g[1],
            ..Default::default()
        });
    }
    for (k, l) in pr.layout.labels.iter().enumerate() {
        let is_port = l.port;
        parts.push(SchPart {
            id: format!("l{k}"),
            kind: if is_port { "label".into() } else { "text".into() },
            name: l.text.clone(),
            x: l.x,
            y: l.y,
            value: if is_port {
                l.node.clone().unwrap_or_default()
            } else {
                l.text.clone()
            },
            props: l.node.iter().map(|n| ("net".to_string(), n.clone())).collect(),
            ..Default::default()
        });
    }
    let wires = pr.layout.wires.iter().map(|w| SchWire { pts: w.pts.clone() }).collect();
    // everything that is not a drawn element or an analysis card stays as a directive
    let mut directives = Vec::new();
    let mut cont_skip = false;
    for (i, raw) in pr.netlist.lines().enumerate() {
        if i == 0 {
            continue;
        }
        let t = raw.trim();
        if t.starts_with('+') {
            if !cont_skip {
                if let Some(last) = directives.last_mut() {
                    let l: &mut String = last;
                    l.push(' ');
                    l.push_str(t.trim_start_matches('+').trim());
                }
            }
            continue;
        }
        cont_skip = false;
        if t.is_empty() || t.starts_with('*') {
            continue;
        }
        let lower = t.to_ascii_lowercase();
        if lower.starts_with(".tran") || lower.starts_with(".ac") || lower.starts_with(".dc") || lower.starts_with(".op") || lower == ".end"
        {
            cont_skip = true;
            continue;
        }
        if !lower.starts_with('.') {
            let el = t.split_whitespace().next().unwrap_or("").to_ascii_lowercase();
            if covered.contains(&el) {
                cont_skip = true;
                continue;
            }
        }
        directives.push(t.to_string());
    }
    let mut analysis = AnalysisSpec {
        op: false,
        tran: false,
        ..Default::default()
    };
    for a in &nl.analyses {
        match a {
            Analysis::Op => analysis.op = true,
            Analysis::Tran {
                tstep, tstop, tmax, uic, ..
            } => {
                analysis.tran = true;
                analysis.tstep = fmt_num(*tstep);
                analysis.tstop = fmt_num(*tstop);
                analysis.tmax = tmax.map(fmt_num).unwrap_or_default();
                analysis.uic = *uic;
            }
            Analysis::Ac { n, fstart, fstop, .. } => {
                analysis.ac = true;
                analysis.ac_points = *n as u32;
                analysis.fstart = fmt_num(*fstart);
                analysis.fstop = fmt_num(*fstop);
            }
            Analysis::Dc { src, start, stop, step } => {
                analysis.dc = true;
                analysis.dc_source = src.clone();
                analysis.dc_start = fmt_num(*start);
                analysis.dc_stop = fmt_num(*stop);
                analysis.dc_step = fmt_num(*step);
            }
        }
    }
    Ok(Schematic {
        title: pr.title.clone(),
        parts,
        wires,
        analysis,
        directives,
    })
}

#[allow(clippy::too_many_arguments)]
fn part(id: &str, kind: &str, name: &str, x: f64, y: f64, rot: u8, value: &str, props: &[(&str, &str)]) -> SchPart {
    SchPart {
        id: id.into(),
        kind: kind.into(),
        name: name.into(),
        x,
        y,
        rot,
        value: value.into(),
        props: props.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
        ..Default::default()
    }
}

fn wire(pts: &[(f64, f64)]) -> SchWire {
    SchWire {
        pts: pts.iter().map(|(x, y)| [*x, *y]).collect(),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Example {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub schematic: Schematic,
}

/// Starter schematics: RC low-pass, LED driver, buck converter power stage.
pub fn examples() -> Vec<Example> {
    let rc = Schematic {
        title: "RC low-pass filter".into(),
        parts: vec![
            part(
                "v1",
                "vsource",
                "V1",
                3.0,
                8.0,
                0,
                "1",
                &[
                    ("wave", "pulse"),
                    ("v1", "0"),
                    ("v2", "1"),
                    ("pw", "0.5m"),
                    ("per", "1m"),
                    ("tr", "1u"),
                    ("tf", "1u"),
                    ("ac", "1"),
                ],
            ),
            part("r1", "resistor", "R1", 9.0, 4.0, 3, "1k", &[]),
            part("c1", "capacitor", "C1", 15.0, 8.0, 0, "100n", &[]),
            part("g1", "ground", "GND", 9.0, 12.0, 0, "", &[]),
            part("l1", "label", "in", 3.0, 4.0, 0, "in", &[]),
            part("l2", "label", "out", 15.0, 4.0, 0, "out", &[]),
            part("pr1", "probe", "P1", 18.0, 4.0, 0, "", &[]),
        ],
        wires: vec![
            wire(&[(3.0, 6.0), (3.0, 4.0), (7.0, 4.0)]),
            wire(&[(11.0, 4.0), (15.0, 4.0), (15.0, 6.0)]),
            wire(&[(15.0, 4.0), (18.0, 4.0)]),
            wire(&[(3.0, 10.0), (3.0, 12.0), (15.0, 12.0), (15.0, 10.0)]),
        ],
        analysis: AnalysisSpec {
            op: true,
            tran: true,
            tstep: "5u".into(),
            tstop: "3m".into(),
            ac: true,
            ac_points: 20,
            fstart: "10".into(),
            fstop: "1meg".into(),
            ..Default::default()
        },
        directives: vec![],
    };
    let led = Schematic {
        title: "LED driver (low-side NMOS switch)".into(),
        parts: vec![
            part("v1", "vsource", "V1", -3.0, 9.0, 0, "5", &[]),
            part("r1", "resistor", "R1", 8.0, 5.0, 0, "150", &[]),
            part("d1", "led", "D1", 8.0, 10.0, 0, "", &[]),
            part("m1", "nmos", "M1", 8.0, 15.0, 0, "", &[("w", "1m"), ("l", "1u")]),
            part("rg", "resistor", "RG", 3.0, 15.0, 3, "100", &[]),
            part(
                "vg",
                "vsource",
                "VG",
                0.0,
                19.0,
                0,
                "5",
                &[
                    ("wave", "pulse"),
                    ("v1", "0"),
                    ("v2", "5"),
                    ("td", "0.1m"),
                    ("tr", "1u"),
                    ("tf", "1u"),
                    ("pw", "0.5m"),
                    ("per", "1m"),
                ],
            ),
            part("g1", "ground", "GND", 8.0, 21.0, 0, "", &[]),
            part("l1", "label", "vdd", 2.0, 3.0, 0, "vdd", &[]),
            part("pr1", "probe", "P1", 11.0, 12.5, 0, "", &[]),
        ],
        wires: vec![
            wire(&[(-3.0, 7.0), (-3.0, 3.0), (8.0, 3.0)]),
            wire(&[(8.0, 7.0), (8.0, 8.0)]),
            wire(&[(8.0, 12.0), (8.0, 13.0)]),
            wire(&[(8.0, 12.5), (11.0, 12.5)]),
            wire(&[(5.0, 15.0), (6.0, 15.0)]),
            wire(&[(1.0, 15.0), (0.0, 15.0), (0.0, 17.0)]),
            wire(&[(8.0, 17.0), (8.0, 21.0)]),
            wire(&[(-3.0, 11.0), (-3.0, 21.0), (8.0, 21.0)]),
        ],
        analysis: AnalysisSpec {
            op: false,
            tran: true,
            tstep: "2u".into(),
            tstop: "2.2m".into(),
            ..Default::default()
        },
        directives: vec![],
    };
    let buck = Schematic {
        title: "Buck converter power stage (5 V → ~1.8 V, open loop)".into(),
        parts: vec![
            part("vin", "vsource", "VIN", 2.0, 9.0, 0, "5", &[]),
            part(
                "s1",
                "switch",
                "S1",
                7.0,
                5.0,
                3,
                "",
                &[("state", "pwm"), ("freq", "500k"), ("duty", "0.4"), ("ron", "50m")],
            ),
            part(
                "d1",
                "diode",
                "D1",
                11.0,
                9.0,
                2,
                "",
                &[("is", "1u"), ("n", "1.05"), ("rs", "50m"), ("bv", "40")],
            ),
            part("l1", "inductor", "L1", 15.0, 5.0, 3, "22u", &[("ic", "0.17")]),
            part("c1", "capacitor", "C1", 20.0, 9.0, 0, "10u", &[("ic", "1.75")]),
            part("rl", "resistor", "RLOAD", 25.0, 9.0, 0, "10", &[]),
            part("g1", "ground", "GND", 11.0, 13.0, 0, "", &[]),
            part("l1n", "label", "sw", 11.0, 5.0, 0, "sw", &[]),
            part("l2n", "label", "vout", 20.0, 5.0, 0, "vout", &[]),
            part("pr1", "probe", "P1", 25.0, 3.0, 0, "", &[]),
        ],
        wires: vec![
            wire(&[(2.0, 7.0), (2.0, 5.0), (5.0, 5.0)]),
            wire(&[(9.0, 5.0), (13.0, 5.0)]),
            wire(&[(11.0, 5.0), (11.0, 7.0)]),
            wire(&[(17.0, 5.0), (25.0, 5.0), (25.0, 7.0)]),
            wire(&[(20.0, 5.0), (20.0, 7.0)]),
            wire(&[(25.0, 5.0), (25.0, 3.0)]),
            wire(&[(2.0, 11.0), (2.0, 13.0), (25.0, 13.0), (25.0, 11.0)]),
            wire(&[(11.0, 11.0), (11.0, 13.0)]),
            wire(&[(20.0, 11.0), (20.0, 13.0)]),
        ],
        analysis: AnalysisSpec {
            op: false,
            tran: true,
            tstep: "10n".into(),
            tstop: "40u".into(),
            uic: true,
            ..Default::default()
        },
        directives: vec![],
    };
    vec![
        Example {
            id: "rc_filter".into(),
            title: "RC low-pass filter".into(),
            summary: "1 kΩ and 100 nF: a 1.59 kHz corner. Square wave in, exponential edges out; the Bode plot shows −20 dB/decade.".into(),
            schematic: rc,
        },
        Example {
            id: "led_driver".into(),
            title: "LED driver".into(),
            summary: "A logic signal switches an NMOS that sinks the LED current set by R1 ≈ (5 V − Vf)/150 Ω. Watch the dots stop when the gate goes low.".into(),
            schematic: led,
        },
        Example {
            id: "buck".into(),
            title: "Buck converter power stage".into(),
            summary: "PWM switch, freewheeling Schottky, LC filter and load: Vout ≈ D·Vin minus the diode drop. Current keeps flowing in L1 while the switch is off.".into(),
            schematic: buck,
        },
    ]
}

// ---------------------------------------------------------------------------
// JSON API
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
#[serde(default)]
struct SchIn {
    schematic: Schematic,
    #[allow(dead_code)]
    compact: bool,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct FromPresetIn {
    id: String,
    project: Option<Project>,
}

#[derive(Serialize)]
struct SimOut {
    extracted: Extracted,
    result: Option<super::SimResult>,
    error: Option<String>,
}

/// `circuit.schematic_*` methods (`None` for anything else).
///
/// * `circuit.schematic_netlist` `{schematic}` → [`Extracted`] (netlist, nets, pin nets, wire flows, junctions).
/// * `circuit.schematic_simulate` `{schematic, compact?}` → `{extracted, result?, error?}`.
/// * `circuit.schematic_from_preset` `{id, project?}` → [`Schematic`].
/// * `circuit.schematic_examples` → starter schematics.
pub fn dispatch(method: &str, input: &serde_json::Value) -> Option<crate::api::ApiResult> {
    use crate::api::handle;
    match method {
        "circuit.schematic_netlist" | "circuit.schematic_to_netlist" => handle(input, |i: SchIn| extract(&i.schematic)),
        "circuit.schematic_simulate" => handle(input, |i: SchIn| {
            let x = extract(&i.schematic)?;
            let (result, error) = match super::simulate(&x.netlist, &BTreeMap::new()) {
                Ok(r) => (Some(r), None),
                Err(e) => (None, Some(e)),
            };
            Ok(SimOut {
                extracted: x,
                result,
                error,
            })
        }),
        "circuit.schematic_from_preset" => handle(input, |i: FromPresetIn| from_preset(&i.id, i.project.as_ref())),
        "circuit.schematic_examples" => handle(input, |_: serde_json::Value| Ok(examples())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn divider() -> Schematic {
        // V1 5 V, R1 top, R2 bottom, drawn with extra wire segments and a mid-wire T
        Schematic {
            title: "divider".into(),
            parts: vec![
                part("v", "vsource", "V1", 0.0, 6.0, 0, "5", &[]),
                part("a", "resistor", "R1", 6.0, 3.0, 0, "10k", &[]),
                part("b", "resistor", "R2", 6.0, 9.0, 0, "10k", &[]),
                part("g", "ground", "GND", 3.0, 13.0, 0, "", &[]),
                part("p", "probe", "P", 9.0, 6.0, 0, "", &[]),
            ],
            wires: vec![
                // top: V1+ (0,4) up to (0,1), across in two segments to R1 pin 1 (6,1)
                wire(&[(0.0, 4.0), (0.0, 1.0), (3.0, 1.0)]),
                wire(&[(3.0, 1.0), (6.0, 1.0)]),
                // middle: R1 pin 2 (6,5) to R2 pin 1 (6,7) with a T to the probe
                wire(&[(6.0, 5.0), (6.0, 7.0)]),
                wire(&[(6.0, 6.0), (9.0, 6.0)]),
                // bottom: a long rail with the ground touching its middle
                wire(&[(0.0, 8.0), (0.0, 13.0), (6.0, 13.0), (6.0, 11.0)]),
            ],
            analysis: AnalysisSpec {
                op: true,
                tran: false,
                ..Default::default()
            },
            directives: vec![],
        }
    }

    #[test]
    fn divider_with_extra_segments_has_three_nodes() {
        let x = extract(&divider()).unwrap();
        // nodes: ground, top, mid → exactly 3 nets (2 non-ground)
        assert_eq!(x.nets.len(), 3, "{:?}", x.nets.iter().map(|n| &n.name).collect::<Vec<_>>());
        assert_eq!(x.node_count, 2);
        assert!(x.warnings.is_empty(), "{:?}", x.warnings);
        assert_eq!(x.probes.len(), 1);
        let r = super::super::simulate(&x.netlist, &BTreeMap::new()).unwrap();
        let mid = &x.probes[0];
        let v = r.op_v(mid).unwrap();
        assert!((v - 2.5).abs() < 1e-6, "{v}\n{}", x.netlist);
        // the T-junction at (6,6) gets a junction dot
        assert!(x.junctions.iter().any(|j| j[0] == 6.0 && j[1] == 6.0));
    }

    #[test]
    fn flow_on_wires_obeys_kirchhoff() {
        let x = extract(&divider()).unwrap();
        let r = super::super::simulate(&x.netlist, &BTreeMap::new()).unwrap();
        let i = |w: &FlowWire| w.flow.iter().map(|(e, s)| s * r.op_i(e).unwrap()).sum::<f64>();
        // the probe branch carries no current, the R1–R2 wire carries 250 µA downwards
        let probe = x.wires.iter().find(|w| w.pts.iter().any(|p| p[0] == 9.0)).unwrap();
        assert!(i(probe).abs() < 1e-12);
        let mid = x
            .wires
            .iter()
            .find(|w| w.pts.first() == Some(&[6.0, 5.0]) || w.pts.last() == Some(&[6.0, 5.0]))
            .unwrap();
        let im = i(mid).abs();
        assert!((im - 250e-6).abs() < 1e-9, "{im}");
        // every wire on the top net carries the full current
        for w in x.wires.iter().filter(|w| w.pts.iter().any(|p| p[1] == 1.0)) {
            assert!((i(w).abs() - 250e-6).abs() < 1e-9);
        }
    }

    #[test]
    fn engineering_numbers() {
        let close = |s: &str, v: f64| {
            let x = num(s).unwrap_or_else(|| panic!("{s} did not parse"));
            assert!((x - v).abs() <= 1e-12 * v.abs(), "{s}: {x} vs {v}");
        };
        close("10k", 10e3);
        close("4.7µ", 4.7e-6);
        close("1meg", 1e6);
        close("2m", 2e-3);
        close("1e-3", 1e-3);
        close("100nF", 100e-9);
        close("2.2 kΩ", 2.2e3);
        assert_eq!(num("abc"), None);
    }

    #[test]
    fn labels_merge_nets_and_rotation_moves_pins() {
        assert_eq!(transform(0.0, -2.0, 1, false), (2.0, 0.0));
        assert_eq!(transform(0.0, -2.0, 3, false), (-2.0, 0.0));
        assert_eq!(transform(-2.0, 0.0, 0, true), (2.0, 0.0));
        let s = Schematic {
            parts: vec![
                part("v", "vsource", "V1", 0.0, 4.0, 0, "1", &[]),
                part("r", "resistor", "R1", 10.0, 4.0, 0, "1k", &[]),
                part("l1", "label", "a", 0.0, 2.0, 0, "top", &[]),
                part("l2", "label", "b", 10.0, 2.0, 0, "top", &[]),
                part("g1", "ground", "G", 0.0, 6.0, 0, "", &[]),
                part("g2", "ground", "G", 10.0, 6.0, 0, "", &[]),
            ],
            ..Default::default()
        };
        let x = extract(&s).unwrap();
        assert_eq!(x.node_count, 1);
        assert!(x.netlist.contains("V1 top 0"));
        assert!(x.netlist.contains("R1 top 0"));
    }

    #[test]
    fn floating_pins_and_duplicates_are_reported() {
        let s = Schematic {
            parts: vec![
                part("r", "resistor", "R1", 0.0, 0.0, 0, "1k", &[]),
                part("r2", "resistor", "R1", 5.0, 0.0, 0, "1k", &[]),
            ],
            ..Default::default()
        };
        assert!(extract(&s).is_err());
        let s2 = Schematic {
            parts: vec![part("r", "resistor", "R1", 0.0, 0.0, 0, "1k", &[])],
            ..Default::default()
        };
        assert_eq!(extract(&s2).unwrap().warnings.len(), 2);
    }

    #[test]
    fn examples_simulate() {
        for ex in examples() {
            let x = extract(&ex.schematic).unwrap();
            assert!(x.warnings.is_empty(), "{}: {:?}", ex.id, x.warnings);
            let r = super::super::simulate(&x.netlist, &BTreeMap::new()).unwrap_or_else(|e| panic!("{}: {e}\n{}", ex.id, x.netlist));
            match ex.id.as_str() {
                "rc_filter" => {
                    let ac = r.ac.as_ref().unwrap();
                    let k = r.node_idx("out").unwrap();
                    // |H| at the corner 1/(2πRC) = 1.59 kHz ≈ −3 dB
                    let f3 = crate::circuit::measure::f_3db(&ac.freq, &ac.mag[k], ac.mag[k][0] as f64).unwrap();
                    assert!((f3 - 1591.5).abs() / 1591.5 < 0.05, "{f3}");
                }
                "led_driver" => {
                    let tr = r.tran.as_ref().unwrap();
                    let j = r.elem_idx("D1").unwrap();
                    let imax = tr.i[j].iter().cloned().fold(0.0f32, f32::max) as f64;
                    assert!(imax > 0.015 && imax < 0.025, "LED current {imax}");
                }
                "buck" => {
                    let v = r.tv("vout").unwrap();
                    let n = v.len();
                    let avg = v[n / 2..].iter().map(|x| *x as f64).sum::<f64>() / (n - n / 2) as f64;
                    assert!(avg > 1.4 && avg < 2.1, "vout {avg}");
                }
                _ => {}
            }
        }
    }

    #[test]
    fn every_preset_imports_and_matches_the_original() {
        for pr in presets::list(None) {
            let s = from_preset(&pr.id, None).unwrap_or_else(|e| panic!("{}: {e}", pr.id));
            let x = extract(&s).unwrap_or_else(|e| panic!("{}: {e}", pr.id));
            let a = super::super::simulate(&x.netlist, &BTreeMap::new()).unwrap_or_else(|e| panic!("{}: {e}\n{}", pr.id, x.netlist));
            let b = super::super::simulate(&pr.netlist, &BTreeMap::new()).unwrap();
            // same nodes (the drawn nets reproduce the deck's connectivity)
            let mut na = a.nodes.clone();
            let mut nb = b.nodes.clone();
            na.sort();
            nb.sort();
            assert_eq!(na, nb, "{}\n{}", pr.id, x.netlist);
            if let (Some(oa), Some(ob)) = (&a.op, &b.op) {
                for n in &b.nodes {
                    let va = oa.node_v[a.node_idx(n).unwrap()];
                    let vb = ob.node_v[b.node_idx(n).unwrap()];
                    assert!((va - vb).abs() < 1e-6 * (1.0 + vb.abs()), "{} {n}: {va} vs {vb}", pr.id);
                }
            }
        }
    }
}
