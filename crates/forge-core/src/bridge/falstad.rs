//! Circuit Lab → Falstad CircuitJS converter.
//!
//! CircuitJS (P. Falstad and I. Sharp, <https://www.falstad.com/circuit/>,
//! source <https://github.com/sharpie7/circuitjs1>) stores a circuit as text:
//! a `$` options line, then one line per element,
//! `<type> x1 y1 x2 y2 flags <element fields…>`, with coordinates in pixels.
//! Elements connect **only where their posts (end points) coincide** — a
//! wire crossing another wire's middle is not a junction. The URL
//! `circuitjs.html?ctz=<lz-string compressToEncodedURIComponent(text)>`
//! opens such a circuit directly.
//!
//! Field layouts implemented here, from the `dump()` methods of the CircuitJS
//! source (`src/com/lushprojects/circuitjs1/client/*Elm.java`):
//!
//! | SPICE | CircuitJS | fields after `flags` |
//! |---|---|---|
//! | R | `r` ResistorElm | resistance |
//! | C | `c` CapacitorElm | capacitance, initial voltage (post 0 − post 1) |
//! | L | `l` InductorElm | inductance, initial current (post 0 → post 1) |
//! | V | `v` VoltageElm | waveform (0 DC, 1 AC, 5 pulse), frequency, max voltage, bias, phase (rad), duty — V(post 1) − V(post 0) = v |
//! | I | `i` CurrentElm | current, flowing post 0 → post 1 through the source (DC only) |
//! | D | `d` DiodeElm / `162` LEDElm | flag 1 = forward drop at 1 A given |
//! | M | `f` MosfetElm | threshold, beta (= KP·W/L); flag 1 = p-channel; gate = point 1, source/drain 16 px either side of point 2 |
//! | S | `s` SwitchElm | position (0 = closed), momentary |
//! | op-amp macro | `a` OpAmpElm | max out, min out, GBW, v−, v+, gain; flag 1 swaps the inputs, flag 8 = gain given |
//! | wire / ground / label | `w` / `g` / `207` LabeledNodeElm | — / symbol type / name (same name = same node) |
//!
//! Anything else (controlled sources outside the op-amp macro, PWL/EXP
//! sources) is reported as unsupported or approximated; the ngspice deck is
//! always offered as the exact fallback. Every generated circuit is checked
//! by re-extracting its connectivity from the post coordinates exactly as
//! CircuitJS does (union of coincident posts, wires, grounds and same-name
//! labels) and comparing it with the SPICE netlist.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::circuit::parser::{self, Device, Netlist, Wave};
use crate::circuit::presets::{self, Layout as SchLayout};
use crate::project::Project;

/// Pixels per Circuit Lab grid unit (CircuitJS's default grid is 16 px).
pub const GRID: f64 = 16.0;
const MARGIN: f64 = 48.0;
const THERMAL_V: f64 = 0.025_865;

#[derive(Clone, Debug, Serialize)]
pub struct FalstadCircuit {
    pub id: String,
    pub title: String,
    pub group: String,
    /// CircuitJS text format.
    pub text: String,
    pub url: String,
    /// "full" (every element exported exactly), "approximate" (exported, with
    /// simplifications listed in `notes`) or "partial" (some elements missing).
    pub support: String,
    pub elements: usize,
    pub unsupported: Vec<String>,
    pub notes: Vec<String>,
    /// Connectivity re-extracted from the CircuitJS text matches the SPICE netlist.
    pub connectivity_ok: bool,
    /// ngspice deck (exact fallback).
    pub netlist: String,
}

/// Compact number formatting Java's `Double.parseDouble` accepts, rounded
/// to 9 significant digits so float noise (`9.999999999999999e-6`) is gone.
pub fn jnum(v: f64) -> String {
    if !v.is_finite() || v == 0.0 {
        return "0".into();
    }
    let r: f64 = format!("{v:.8e}").parse().unwrap_or(v);
    if (1e-4..1e9).contains(&r.abs()) {
        format!("{r}")
    } else {
        format!("{r:e}")
    }
}

fn px(u: f64) -> i64 {
    (u * GRID).round() as i64 + MARGIN as i64
}

type Pt = (i64, i64);

#[derive(Default)]
struct Sheet {
    lines: Vec<String>,
    /// (post, spice node) for every electrical terminal emitted
    posts: Vec<(Pt, String)>,
    wires: Vec<(Pt, Pt)>,
    grounds: Vec<Pt>,
    labels: Vec<(Pt, String)>,
    notes: Vec<String>,
}

impl Sheet {
    fn wire(&mut self, a: Pt, b: Pt) {
        if a != b {
            self.lines.push(format!("w {} {} {} {} 0", a.0, a.1, b.0, b.1));
            self.wires.push((a, b));
        }
    }
    fn ground(&mut self, p: Pt, up: bool) {
        let dy = if up { -16 } else { 16 };
        self.lines.push(format!("g {} {} {} {} 0 0", p.0, p.1, p.0, p.1 + dy));
        self.grounds.push(p);
    }
    fn label(&mut self, p: Pt, dir: Pt, name: &str) {
        if name == "0" {
            self.ground(p, dir.1 < 0);
            return;
        }
        self.lines
            .push(format!("207 {} {} {} {} 0 {}", p.0, p.1, p.0 + dir.0, p.1 + dir.1, name));
        self.labels.push((p, name.to_string()));
    }
}

/// Forward voltage of the SPICE junction diode at 1 A (what CircuitJS's
/// "fwdrop" parameter means): n·Vt·ln(1/(Is·area) + 1) + Rs·1 A.
pub fn diode_drop_at_1a(is: f64, n: f64, rs: f64, area: f64) -> f64 {
    n * THERMAL_V * (1.0 / (is * area).max(1e-30) + 1.0).ln() + rs
}

fn node_name(nl: &Netlist, i: usize) -> String {
    nl.nodes.get(i).cloned().unwrap_or_default().to_lowercase()
}

/// `v` element fields after flags for a SPICE source.
fn vsource_fields(src: &parser::Source, tstop: f64, name: &str, notes: &mut Vec<String>) -> (u32, String) {
    match &src.wave {
        Wave::None => (0, format!("0 40 {} 0 0 0.5", jnum(src.dc))),
        Wave::Pulse {
            v1, v2, td, pw, per, tr, ..
        } => {
            let per = per.filter(|x| *x > 0.0).unwrap_or(2.0 * tstop.max(1e-9));
            let pw = pw.filter(|x| *x > 0.0).unwrap_or(per / 2.0).min(per);
            let f = 1.0 / per;
            let tau = 2.0 * std::f64::consts::PI;
            let phase = (-(tau * f * td)).rem_euclid(tau);
            if tr.unwrap_or(0.0) > 0.0 && tr.unwrap_or(0.0) > 0.01 * pw {
                notes.push(format!(
                    "{name}: pulse edges are ideal in CircuitJS (SPICE rise time {} s ignored).",
                    jnum(tr.unwrap_or(0.0))
                ));
            }
            (
                4,
                format!("5 {} {} {} {} {}", jnum(f), jnum(v2 - v1), jnum(*v1), jnum(phase), jnum(pw / per)),
            )
        }
        Wave::Sin {
            vo,
            va,
            freq,
            td,
            theta,
            phase_deg,
        } => {
            let f = freq.unwrap_or(1.0 / tstop.max(1e-9));
            if *td != 0.0 || *theta != 0.0 {
                notes.push(format!("{name}: sine delay/damping not represented in CircuitJS."));
            }
            (
                0,
                format!("1 {} {} {} {} 0.5", jnum(f), jnum(*va), jnum(*vo), jnum(phase_deg.to_radians())),
            )
        }
        _ => {
            notes.push(format!("{name}: EXP/PWL waveform exported as its DC value ({} V).", jnum(src.dc)));
            (0, format!("0 40 {} 0 0 0.5", jnum(src.dc)))
        }
    }
}

/// Transient stop time of the netlist (for pulse periods), s.
fn tstop_of(nl: &Netlist) -> (f64, f64) {
    for a in &nl.analyses {
        if let parser::Analysis::Tran { tstep, tstop, .. } = a {
            return (*tstep, *tstop);
        }
    }
    (1e-6, 1e-3)
}

/// Emit one element given its terminal post coordinates (in SPICE node
/// order). Returns false when the device type has no CircuitJS equivalent.
fn emit(sh: &mut Sheet, nl: &Netlist, el: &parser::Element, posts: &[Pt], sym: &str, tstop: f64) -> bool {
    let n = |k: usize| node_name(nl, el.nodes[k]);
    let (a, b) = (posts[0], *posts.get(1).unwrap_or(&posts[0]));
    match &el.dev {
        Device::Resistor { r } => {
            sh.lines.push(format!("r {} {} {} {} 0 {}", a.0, a.1, b.0, b.1, jnum(*r)));
        }
        Device::Capacitor { c, ic } => {
            sh.lines.push(format!(
                "c {} {} {} {} 0 {} {}",
                a.0,
                a.1,
                b.0,
                b.1,
                jnum(*c),
                jnum(ic.unwrap_or(0.0))
            ));
        }
        Device::Inductor { l, ic } => {
            sh.lines.push(format!(
                "l {} {} {} {} 0 {} {}",
                a.0,
                a.1,
                b.0,
                b.1,
                jnum(*l),
                jnum(ic.unwrap_or(0.0))
            ));
        }
        Device::Vsource { src } => {
            let (flags, f) = vsource_fields(src, tstop, &el.name, &mut sh.notes);
            // post 0 = negative terminal, post 1 = positive
            sh.lines.push(format!("v {} {} {} {} {flags} {f}", b.0, b.1, a.0, a.1));
        }
        Device::Isource { src } => {
            let i = match &src.wave {
                Wave::None => src.dc,
                Wave::Pulse { v2, .. } => {
                    sh.notes.push(format!(
                        "{}: CircuitJS current sources are DC-only — set to the pulse's high level ({} A).",
                        el.name,
                        jnum(*v2)
                    ));
                    *v2
                }
                _ => {
                    sh.notes.push(format!("{}: waveform exported as its DC value.", el.name));
                    src.dc
                }
            };
            sh.lines.push(format!("i {} {} {} {} 0 {}", a.0, a.1, b.0, b.1, jnum(i)));
        }
        Device::Diode { model, area, .. } => {
            let vf = diode_drop_at_1a(model.is, model.n, model.rs, *area);
            if sym == "led" {
                sh.lines
                    .push(format!("162 {} {} {} {} 1 {} 1 0 0 0.01", a.0, a.1, b.0, b.1, jnum(vf)));
            } else {
                sh.lines.push(format!("d {} {} {} {} 1 {}", a.0, a.1, b.0, b.1, jnum(vf)));
            }
        }
        Device::Switch { on, .. } => {
            sh.lines
                .push(format!("s {} {} {} {} 0 {} false", a.0, a.1, b.0, b.1, if *on { 0 } else { 1 }));
            sh.notes.push(format!(
                "{}: SPICE voltage-controlled switch exported as a manual switch (starts {}) — click it in CircuitJS to toggle.",
                el.name,
                if *on { "closed" } else { "open" }
            ));
            // control nodes are not represented
            sh.posts.push((a, n(0)));
            sh.posts.push((b, n(1)));
            return true;
        }
        Device::Mosfet { .. } => unreachable!("MOSFETs are emitted by emit_mosfet"),
        Device::Vcvs { .. } | Device::Vccs { .. } => return false,
    }
    sh.posts.push((a, n(0)));
    sh.posts.push((b, n(1)));
    true
}

/// MOSFET with its gate at `gate` and the body centre at `body` (horizontal).
/// Returns the CircuitJS posts (gate, upper, lower).
fn emit_mosfet(sh: &mut Sheet, el: &parser::Element, gate: Pt, body: Pt) -> (Pt, Pt, Pt) {
    let Device::Mosfet { model, w, l, mult, .. } = &el.dev else {
        unreachable!()
    };
    let beta = model.kp * w / l.max(1e-12) * mult.max(1e-12);
    let flags = if model.pmos { 1 } else { 0 };
    sh.lines.push(format!(
        "f {} {} {} {} {flags} {} {}",
        gate.0,
        gate.1,
        body.0,
        body.1,
        jnum(model.vto.abs()),
        jnum(beta)
    ));
    // MosfetElm.setPoints: source/drain 16 px either side of point 2, the
    // "source" (post 1) on the side given by interpPoint2(…, −16·dsign).
    (gate, (body.0, body.1 - 16), (body.0, body.1 + 16))
}

struct UF(Vec<usize>);
impl UF {
    fn find(&mut self, mut x: usize) -> usize {
        while self.0[x] != x {
            self.0[x] = self.0[self.0[x]];
            x = self.0[x];
        }
        x
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.0[ra] = rb;
        }
    }
}

/// Connectivity check: CircuitJS nets (coincident posts, wires, grounds,
/// labels) must partition the posts exactly like the SPICE nodes do.
fn verify(sh: &Sheet) -> Result<(), String> {
    let mut pts: Vec<Pt> = Vec::new();
    let id = |p: Pt, pts: &mut Vec<Pt>| -> usize {
        match pts.iter().position(|q| *q == p) {
            Some(i) => i,
            None => {
                pts.push(p);
                pts.len() - 1
            }
        }
    };
    let post_ids: Vec<usize> = sh.posts.iter().map(|(p, _)| id(*p, &mut pts)).collect();
    let wire_ids: Vec<(usize, usize)> = sh.wires.iter().map(|(a, b)| (id(*a, &mut pts), id(*b, &mut pts))).collect();
    let gnd_ids: Vec<usize> = sh.grounds.iter().map(|p| id(*p, &mut pts)).collect();
    let label_ids: Vec<(usize, String)> = sh.labels.iter().map(|(p, n)| (id(*p, &mut pts), n.clone())).collect();
    let mut uf = UF((0..pts.len()).collect());
    for (a, b) in wire_ids {
        uf.union(a, b);
    }
    for w in gnd_ids.windows(2) {
        uf.union(w[0], w[1]);
    }
    for i in 0..label_ids.len() {
        for j in i + 1..label_ids.len() {
            if label_ids[i].1 == label_ids[j].1 {
                uf.union(label_ids[i].0, label_ids[j].0);
            }
        }
    }
    let gnd_root = gnd_ids.first().map(|g| uf.find(*g));
    let mut node_root: BTreeMap<String, usize> = BTreeMap::new();
    for (k, (_, node)) in sh.posts.iter().enumerate() {
        let r = uf.find(post_ids[k]);
        if node == "0" && gnd_root != Some(r) {
            return Err(format!("a ground terminal at {:?} is not connected to ground", sh.posts[k].0));
        }
        match node_root.get(node) {
            Some(r0) if *r0 != r => return Err(format!("node `{node}` is split in the CircuitJS drawing")),
            None => {
                node_root.insert(node.clone(), r);
            }
            _ => {}
        }
    }
    let mut seen: BTreeMap<usize, String> = BTreeMap::new();
    for (node, r) in &node_root {
        if let Some(other) = seen.get(r) {
            return Err(format!("nodes `{other}` and `{node}` are shorted in the CircuitJS drawing"));
        }
        seen.insert(*r, node.clone());
    }
    Ok(())
}

/// Split polylines of one node at every point of the same node lying inside a
/// segment (CircuitJS only joins at end points).
fn wire_segments(lay: &SchLayout, node: &str, extra_pts: &[Pt]) -> Vec<(Pt, Pt)> {
    let mut segs: Vec<(Pt, Pt)> = Vec::new();
    for w in lay.wires.iter().filter(|w| w.node.eq_ignore_ascii_case(node)) {
        for pair in w.pts.windows(2) {
            segs.push(((px(pair[0][0]), px(pair[0][1])), (px(pair[1][0]), px(pair[1][1]))));
        }
    }
    let mut pts: Vec<Pt> = extra_pts.to_vec();
    for (a, b) in &segs {
        pts.push(*a);
        pts.push(*b);
    }
    let mut out = Vec::new();
    for (a, b) in segs {
        let mut cuts: Vec<(f64, Pt)> = pts
            .iter()
            .filter_map(|p| {
                let (dx, dy) = ((b.0 - a.0) as f64, (b.1 - a.1) as f64);
                let (qx, qy) = ((p.0 - a.0) as f64, (p.1 - a.1) as f64);
                let len2 = dx * dx + dy * dy;
                if len2 == 0.0 || (dx * qy - dy * qx).abs() > 1e-6 {
                    return None;
                }
                let t = (dx * qx + dy * qy) / len2;
                (t > 1e-9 && t < 1.0 - 1e-9).then_some((t, *p))
            })
            .collect();
        cuts.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
        cuts.dedup_by(|x, y| x.1 == y.1);
        let mut prev = a;
        for (_, c) in cuts {
            out.push((prev, c));
            prev = c;
        }
        out.push((prev, b));
    }
    out
}

/// Convert a netlist (+ optional Circuit Lab schematic layout) to CircuitJS.
pub fn convert(id: &str, title: &str, group: &str, deck: &str, layout: Option<&SchLayout>) -> FalstadCircuit {
    let mut out = FalstadCircuit {
        id: id.into(),
        title: title.into(),
        group: group.into(),
        text: String::new(),
        url: String::new(),
        support: "full".into(),
        elements: 0,
        unsupported: vec![],
        notes: vec![],
        connectivity_ok: false,
        netlist: deck.into(),
    };
    let nl = match parser::parse(deck, &BTreeMap::new()) {
        Ok(nl) => nl,
        Err(e) => {
            out.support = "none".into();
            out.notes.push(format!(
                "netlist does not parse: {}",
                e.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("; ")
            ));
            return out;
        }
    };
    let (tstep, tstop) = tstop_of(&nl);
    let mut sh = Sheet::default();
    let mut handled: Vec<bool> = vec![false; nl.elements.len()];
    let find_el = |name: &str| nl.elements.iter().position(|e| e.name.eq_ignore_ascii_case(name));
    let mut max_x: i64 = 0;
    if let Some(lay) = layout {
        for part in &lay.parts {
            let pin = |nm: &str| part.pins.iter().find(|p| p.name == nm).map(|p| (px(p.x), px(p.y)));
            for p in &part.pins {
                max_x = max_x.max(px(p.x));
            }
            match part.sym.as_str() {
                "opamp" => {
                    let (Some(ip), Some(im), Some(o)) = (pin("in+"), pin("in-"), pin("out")) else {
                        continue;
                    };
                    let gain = find_el(&part.el)
                        .and_then(|k| match nl.elements[k].dev {
                            Device::Vcvs { gain } => Some(gain),
                            _ => None,
                        })
                        .unwrap_or(1e5);
                    let yc = (ip.1 + im.1) / 2;
                    let swap = ip.1 < im.1; // in+ on top needs the swap flag
                    let flags = 8 | u32::from(swap);
                    sh.lines.push(format!(
                        "a {} {} {} {} {flags} 15 -15 1000000 0 0 {}",
                        ip.0,
                        yc,
                        o.0,
                        o.1,
                        jnum(gain)
                    ));
                    let node_of = |nm: &str| {
                        part.pins
                            .iter()
                            .find(|p| p.name == nm)
                            .map(|p| p.node.to_lowercase())
                            .unwrap_or_default()
                    };
                    sh.posts.push((im, node_of("in-")));
                    sh.posts.push((ip, node_of("in+")));
                    sh.posts.push((o, node_of("out")));
                    // CircuitJS's inputs sit 16 px above/below the centre line
                    let (up, dn) = ((ip.0, yc - 16), (ip.0, yc + 16));
                    let (top_pin, bot_pin) = if swap { (ip, im) } else { (im, ip) };
                    sh.wire(up, top_pin);
                    sh.wire(dn, bot_pin);
                    for m in &part.members {
                        if let Some(k) = find_el(m) {
                            handled[k] = true;
                        }
                    }
                    sh.notes.push(format!(
                        "{}: the op-amp macro-model (gain, dominant pole, output resistance) becomes CircuitJS's op-amp with gain {} and ±15 V rails; CircuitJS ignores its GBW field.",
                        part.el,
                        jnum(gain)
                    ));
                    out.elements += 1;
                }
                "nmos" | "pmos" => {
                    let Some(k) = find_el(&part.el) else { continue };
                    let (Some(g), Some(d), Some(s)) = (pin("g"), pin("d"), pin("s")) else {
                        continue;
                    };
                    let body = (px(part.x), px(part.y));
                    let (gp, upper, lower) = emit_mosfet(&mut sh, &nl.elements[k], g, body);
                    let el = &nl.elements[k];
                    let (dn, gn, sn) = (
                        node_name(&nl, el.nodes[0]),
                        node_name(&nl, el.nodes[1]),
                        node_name(&nl, el.nodes[2]),
                    );
                    // Level-1 MOSFETs are symmetric: connect whichever layout pin is above to the upper post.
                    let (top, bot, topn, botn) = if d.1 <= s.1 { (d, s, dn, sn) } else { (s, d, sn, dn) };
                    sh.posts.push((gp, gn));
                    sh.posts.push((top, topn));
                    sh.posts.push((bot, botn));
                    sh.wire(upper, top);
                    sh.wire(lower, bot);
                    handled[k] = true;
                    out.elements += 1;
                }
                _ => {
                    let Some(k) = find_el(&part.el) else { continue };
                    if part.members.len() > 1 || part.pins.len() != 2 {
                        continue;
                    }
                    let el = &nl.elements[k];
                    // order the pins like the SPICE card (n+ first)
                    let n0 = node_name(&nl, el.nodes[0]);
                    let (p0, p1) = if part.pins[0].node.eq_ignore_ascii_case(&n0) {
                        (&part.pins[0], &part.pins[1])
                    } else {
                        (&part.pins[1], &part.pins[0])
                    };
                    let posts = [(px(p0.x), px(p0.y)), (px(p1.x), px(p1.y))];
                    if emit(&mut sh, &nl, el, &posts, &part.sym, tstop) {
                        handled[k] = true;
                        out.elements += 1;
                    }
                }
            }
        }
        // Schematic wires, split so every junction is an end point.
        let mut nodes: Vec<String> = lay.wires.iter().map(|w| w.node.to_lowercase()).collect();
        nodes.sort();
        nodes.dedup();
        for node in nodes {
            let extra: Vec<Pt> = sh.posts.iter().filter(|(_, n)| *n == node).map(|(p, _)| *p).collect();
            for (a, b) in wire_segments(lay, &node, &extra) {
                sh.wire(a, b);
            }
        }
        for g in &lay.grounds {
            sh.ground((px(g[0]), px(g[1])), false);
        }
    }
    // Elements without a schematic position: a grid, nets joined by labels.
    let mut slot = 0usize;
    let x0 = max_x + 96;
    for (k, el) in nl.elements.iter().enumerate() {
        if handled[k] {
            continue;
        }
        let (cx, cy) = (x0 + (slot % 6) as i64 * 112, MARGIN as i64 + (slot / 6) as i64 * 144);
        match &el.dev {
            Device::Mosfet { .. } => {
                let gate = (cx, cy + 32);
                let body = (cx + 32, cy + 32);
                let (g, up, lo) = emit_mosfet(&mut sh, el, gate, body);
                let (dn, gn, sn) = (
                    node_name(&nl, el.nodes[0]),
                    node_name(&nl, el.nodes[1]),
                    node_name(&nl, el.nodes[2]),
                );
                sh.posts.push((g, gn.clone()));
                sh.posts.push((up, dn.clone()));
                sh.posts.push((lo, sn.clone()));
                sh.label(g, (-16, 0), &gn);
                sh.label(up, (0, -16), &dn);
                sh.label(lo, (0, 16), &sn);
            }
            Device::Vcvs { .. } | Device::Vccs { .. } => {
                out.unsupported.push(format!("{} (controlled source)", el.name));
                continue;
            }
            _ => {
                let posts = [(cx, cy), (cx, cy + 64)];
                if !emit(&mut sh, &nl, el, &posts, "", tstop) {
                    out.unsupported.push(el.name.clone());
                    continue;
                }
                let (a, b) = (node_name(&nl, el.nodes[0]), node_name(&nl, el.nodes[1]));
                sh.label(posts[0], (0, -16), &a);
                sh.label(posts[1], (0, 16), &b);
            }
        }
        out.elements += 1;
        slot += 1;
    }
    if slot > 0 && layout.is_some() {
        out.notes.push(format!(
            "{slot} element(s) without a schematic position were placed on a grid to the right, joined by named nodes."
        ));
    }
    // If the drawing leaves a SPICE node split, join the pieces with named nodes.
    if let Err(e) = verify(&sh) {
        if e.contains("split") || e.contains("not connected to ground") {
            let posts = sh.posts.clone();
            for (p, node) in posts {
                sh.label(p, (0, -16), &node);
            }
            out.notes
                .push("Some schematic wires did not reach every terminal; named nodes join them.".into());
        }
    }
    out.connectivity_ok = verify(&sh).is_ok();
    if let Err(e) = verify(&sh) {
        out.notes.push(format!("connectivity check failed: {e}"));
    }
    let dt = tstep.clamp(1e-12, 1e-3).min(tstop / 200.0).max(1e-12);
    let mut text = format!("$ 1 {} 10.2 50 5 50 {}\n", jnum(dt), jnum(dt / 1000.0));
    for l in &sh.lines {
        text += l;
        text += "\n";
    }
    let approximated = !sh.notes.is_empty();
    out.notes.extend(sh.notes.iter().cloned());
    out.notes.dedup();
    out.support = if !out.unsupported.is_empty() {
        "partial".into()
    } else if approximated {
        "approximate".into()
    } else {
        "full".into()
    };
    out.url = link(&text);
    out.text = text;
    out
}

/// `https://www.falstad.com/circuit/circuitjs.html?ctz=…` for a circuit text.
pub fn link(text: &str) -> String {
    format!(
        "https://www.falstad.com/circuit/circuitjs.html?ctz={}",
        super::lzstring::compress_to_encoded_uri_component(text)
    )
}

/// Every Circuit Lab preset (with the project's component values) plus the
/// project's own netlists, converted to CircuitJS.
pub fn all(p: &Project) -> Vec<FalstadCircuit> {
    let mut v: Vec<FalstadCircuit> = presets::list(Some(p))
        .iter()
        .map(|pr| convert(&pr.id, &pr.title, &pr.group, &pr.netlist, Some(&pr.layout)))
        .collect();
    for c in &p.design.circuits {
        v.push(convert(&c.id, &c.title, "Project", &c.netlist, None));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_java_friendly() {
        assert_eq!(jnum(9.999999999999999e-6), "1e-5");
        assert_eq!(jnum(4.9999999999999994e8), "500000000");
        assert_eq!(jnum(2.0000000000000003e-14), "2e-14");
        assert_eq!(jnum(1500000.0), "1500000");
        assert_eq!(jnum(-0.25), "-0.25");
    }

    #[test]
    fn diode_drop_matches_hand_calc() {
        // Is = 1e-14, n = 1: Vt·ln(1e14) = 0.025865·32.236 = 0.8338 V
        let v = diode_drop_at_1a(1e-14, 1.0, 0.0, 1.0);
        assert!((v - 0.8338).abs() < 1e-3, "{v}");
    }

    #[test]
    fn rc_by_hand() {
        let deck = "rc\nV1 in 0 5\nR1 in out 1k\nC1 out 0 1u\n.tran 1u 5m\n.end\n";
        let c = convert("rc", "RC", "Test", deck, None);
        assert!(c.connectivity_ok, "{:?}", c.notes);
        assert_eq!(c.support, "full");
        assert_eq!(c.elements, 3);
        assert!(c.text.starts_with("$ 1 "));
        assert!(c.text.lines().any(|l| l.starts_with("r ") && l.ends_with(" 1000")));
        assert!(c.text.lines().any(|l| l.starts_with("c ") && l.contains(" 1e-6 ")));
        assert!(c.text.lines().any(|l| l.starts_with("v ") && l.ends_with(" 0 40 5 0 0 0.5")));
        // the link decodes back to the text
        let ctz = c.url.split("ctz=").nth(1).unwrap();
        assert_eq!(super::super::lzstring::decompress_from_encoded_uri_component(ctz).unwrap(), c.text);
    }

    #[test]
    fn every_preset_converts_with_correct_connectivity() {
        let p = Project::iris();
        let all = all(&p);
        assert_eq!(all.len(), presets::list(None).len());
        for c in &all {
            assert!(c.connectivity_ok, "{}: {:?}\n{}", c.id, c.notes, c.text);
            assert!(c.unsupported.is_empty(), "{}: {:?}", c.id, c.unsupported);
            assert!(c.url.len() < 8000, "{} url too long", c.id);
            for l in c.text.lines().skip(1) {
                let t: Vec<&str> = l.split_whitespace().collect();
                assert!(t.len() >= 6, "{l}");
                for v in &t[1..5] {
                    v.parse::<i64>()
                        .unwrap_or_else(|_| panic!("{}: non-integer coordinate in {l}", c.id));
                }
            }
        }
        let divider = all.iter().find(|c| c.id == "vbat_divider").unwrap();
        // project values: R1 = 1.5 MΩ, R2 = 1 MΩ
        assert!(divider.text.lines().any(|l| l.starts_with("r ") && l.ends_with(" 1500000")));
        let inv = all.iter().find(|c| c.id == "cmos_inverter").unwrap();
        assert_eq!(inv.text.lines().filter(|l| l.starts_with("f ")).count(), 2);
        let op = all.iter().find(|c| c.id == "opamp_preamp").unwrap();
        assert!(op.text.lines().any(|l| l.starts_with("a ")));
    }

    #[test]
    fn broken_drawings_are_detected() {
        let mut sh = Sheet::default();
        sh.posts.push(((0, 0), "a".into()));
        sh.posts.push(((0, 64), "b".into()));
        sh.posts.push(((64, 0), "a".into()));
        assert!(verify(&sh).unwrap_err().contains("split"));
        sh.wire((0, 0), (64, 0));
        assert!(verify(&sh).is_ok());
        sh.wire((0, 0), (0, 64));
        assert!(verify(&sh).unwrap_err().contains("shorted"));
    }
}
