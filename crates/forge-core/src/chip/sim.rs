//! Event-driven gate-level logic simulator with per-gate transport delays.
//!
//! Every net holds 0, 1 or X (unknown). When a net changes, each gate that
//! reads it is re-evaluated and its new output is *scheduled* `delay_ns`
//! later on a time-ordered event queue (the classic discrete-event scheme of
//! Verilog simulators — see IEEE 1364 §11, "Scheduling semantics"). Because
//! delays are transport delays, unequal path delays show up as **glitches**
//! (hazards): the ripple-carry adder's sum bits flicker while the carry
//! ripples, and a 2:1 multiplexer shows a static-1 hazard when its select
//! input falls.
//!
//! Static timing: the longest register-to-register / input-to-output path
//! through combinational gates (N. Weste & D. Harris, *CMOS VLSI Design*,
//! 4th ed., ch. 4) is reported as the critical path.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use serde::{Deserialize, Serialize};

pub const X: u8 = 2;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum GateKind {
    And,
    Or,
    Not,
    Nand,
    Nor,
    Xor,
    Xnor,
    Buf,
    Tie0,
    Tie1,
    /// D flip-flop: inputs [D, CLK] (+ optional async reset). Rising-edge triggered.
    Dff,
}

impl GateKind {
    pub fn default_delay(self) -> f64 {
        match self {
            GateKind::Not | GateKind::Buf => 1.0,
            GateKind::Nand | GateKind::Nor => 1.5,
            GateKind::And | GateKind::Or => 2.0,
            GateKind::Xor | GateKind::Xnor => 3.0,
            GateKind::Dff => 2.0,
            GateKind::Tie0 | GateKind::Tie1 => 0.0,
        }
    }
    pub fn from_name(s: &str) -> Option<GateKind> {
        Some(match s.to_ascii_uppercase().as_str() {
            "AND" => GateKind::And,
            "OR" => GateKind::Or,
            "NOT" | "INV" => GateKind::Not,
            "NAND" => GateKind::Nand,
            "NOR" => GateKind::Nor,
            "XOR" => GateKind::Xor,
            "XNOR" => GateKind::Xnor,
            "BUF" => GateKind::Buf,
            "TIE0" => GateKind::Tie0,
            "TIE1" => GateKind::Tie1,
            "DFF" => GateKind::Dff,
            _ => return None,
        })
    }
}

fn eval(kind: GateKind, ins: &[u8]) -> u8 {
    let any = |v: u8| ins.contains(&v);
    let and = || {
        if any(0) {
            0
        } else if any(X) {
            X
        } else {
            1
        }
    };
    let or = || {
        if any(1) {
            1
        } else if any(X) {
            X
        } else {
            0
        }
    };
    let inv = |v: u8| if v == X { X } else { 1 - v };
    let xor = || if any(X) { X } else { ins.iter().fold(0, |a, b| a ^ b) };
    match kind {
        GateKind::And => and(),
        GateKind::Or => or(),
        GateKind::Nand => inv(and()),
        GateKind::Nor => inv(or()),
        GateKind::Xor => xor(),
        GateKind::Xnor => inv(xor()),
        GateKind::Not => inv(ins.first().copied().unwrap_or(X)),
        GateKind::Buf => ins.first().copied().unwrap_or(X),
        GateKind::Tie0 => 0,
        GateKind::Tie1 => 1,
        GateKind::Dff => X,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimGate {
    pub id: String,
    pub kind: GateKind,
    pub inputs: Vec<String>,
    pub output: String,
    #[serde(default)]
    pub delay_ns: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Clock {
    pub net: String,
    pub period_ns: f64,
    pub start_ns: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Stimulus {
    /// (time ns, net, value 0/1)
    pub changes: Vec<(f64, String, u8)>,
    pub clocks: Vec<Clock>,
    pub t_end_ns: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Circuit {
    pub name: String,
    pub title: String,
    pub description: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub gates: Vec<SimGate>,
    pub stimulus: Stimulus,
}

fn g(id: &str, kind: GateKind, ins: &[&str], out: &str) -> SimGate {
    SimGate {
        id: id.into(),
        kind,
        inputs: ins.iter().map(|s| s.to_string()).collect(),
        output: out.into(),
        delay_ns: None,
    }
}

fn ch(t: f64, net: &str, v: u8) -> (f64, String, u8) {
    (t, net.to_string(), v)
}

/// Built-in teaching circuits.
pub fn builtin(name: &str) -> Option<Circuit> {
    use GateKind::*;
    Some(match name {
        "half_adder" => Circuit {
            name: name.into(),
            title: "Half adder".into(),
            description: "S = A ⊕ B, C = A·B — the one-bit building block.".into(),
            inputs: vec!["A".into(), "B".into()],
            outputs: vec!["S".into(), "C".into()],
            gates: vec![g("x1", Xor, &["A", "B"], "S"), g("a1", And, &["A", "B"], "C")],
            stimulus: Stimulus { changes: vec![ch(0.0, "A", 0), ch(0.0, "B", 0), ch(10.0, "B", 1), ch(20.0, "A", 1), ch(30.0, "B", 0), ch(40.0, "A", 0)], clocks: vec![], t_end_ns: 50.0 },
        },
        "full_adder" => {
            let mut changes = vec![];
            for k in 0..8u8 {
                let t = k as f64 * 10.0;
                changes.push(ch(t, "A", (k >> 2) & 1));
                changes.push(ch(t, "B", (k >> 1) & 1));
                changes.push(ch(t, "Cin", k & 1));
            }
            Circuit {
                name: name.into(),
                title: "Full adder".into(),
                description: "Two half adders and an OR: S = A ⊕ B ⊕ Cin, Cout = A·B + Cin·(A ⊕ B).".into(),
                inputs: vec!["A".into(), "B".into(), "Cin".into()],
                outputs: vec!["S".into(), "Cout".into()],
                gates: vec![
                    g("x1", Xor, &["A", "B"], "p"),
                    g("x2", Xor, &["p", "Cin"], "S"),
                    g("a1", And, &["A", "B"], "g"),
                    g("a2", And, &["p", "Cin"], "t"),
                    g("o1", Or, &["g", "t"], "Cout"),
                ],
                stimulus: Stimulus { changes, clocks: vec![], t_end_ns: 80.0 },
            }
        }
        "rca4" => {
            let mut gates = Vec::new();
            for i in 0..4 {
                let cin = if i == 0 { "Cin".to_string() } else { format!("c{i}") };
                let cout = if i == 3 { "Cout".to_string() } else { format!("c{}", i + 1) };
                let (a, b, p, gg, t, s) = (format!("A{i}"), format!("B{i}"), format!("p{i}"), format!("g{i}"), format!("t{i}"), format!("S{i}"));
                gates.push(SimGate { id: format!("fa{i}_x1"), kind: Xor, inputs: vec![a.clone(), b.clone()], output: p.clone(), delay_ns: None });
                gates.push(SimGate { id: format!("fa{i}_x2"), kind: Xor, inputs: vec![p.clone(), cin.clone()], output: s, delay_ns: None });
                gates.push(SimGate { id: format!("fa{i}_a1"), kind: And, inputs: vec![a, b], output: gg.clone(), delay_ns: None });
                gates.push(SimGate { id: format!("fa{i}_a2"), kind: And, inputs: vec![p, cin], output: t.clone(), delay_ns: None });
                gates.push(SimGate { id: format!("fa{i}_o1"), kind: Or, inputs: vec![gg, t], output: cout, delay_ns: None });
            }
            let set = |t: f64, a: u8, b: u8, c: u8, out: &mut Vec<(f64, String, u8)>| {
                for i in 0..4 {
                    out.push((t, format!("A{i}"), (a >> i) & 1));
                    out.push((t, format!("B{i}"), (b >> i) & 1));
                }
                out.push((t, "Cin".into(), c));
            };
            let mut changes = vec![];
            set(0.0, 0, 0, 0, &mut changes);
            set(10.0, 0b0111, 0b0001, 0, &mut changes);
            set(50.0, 0b1111, 0b0000, 0, &mut changes);
            set(70.0, 0b1111, 0b0000, 1, &mut changes);
            set(110.0, 0b0101, 0b0011, 0, &mut changes);
            let mut inputs: Vec<String> = (0..4).map(|i| format!("A{i}")).collect();
            inputs.extend((0..4).map(|i| format!("B{i}")));
            inputs.push("Cin".into());
            Circuit {
                name: name.into(),
                title: "4-bit ripple-carry adder".into(),
                description: "Four full adders in a chain. Watch the carry ripple: the sum bits glitch until the carry arrives (0111 + 0001, then 1111 + Cin).".into(),
                inputs,
                outputs: vec!["S0".into(), "S1".into(), "S2".into(), "S3".into(), "Cout".into()],
                gates,
                stimulus: Stimulus { changes, clocks: vec![], t_end_ns: 150.0 },
            }
        }
        "mux2" => Circuit {
            name: name.into(),
            title: "2:1 multiplexer".into(),
            description: "Y = A·S' + B·S. With A = B = 1 the output should stay 1, but when S falls the inverter's extra delay opens a gap: a static-1 hazard.".into(),
            inputs: vec!["A".into(), "B".into(), "S".into()],
            outputs: vec!["Y".into()],
            gates: vec![g("n1", Not, &["S"], "Sn"), g("a1", And, &["A", "Sn"], "t0"), g("a2", And, &["B", "S"], "t1"), g("o1", Or, &["t0", "t1"], "Y")],
            stimulus: Stimulus {
                changes: vec![ch(0.0, "A", 1), ch(0.0, "B", 0), ch(0.0, "S", 0), ch(10.0, "S", 1), ch(20.0, "B", 1), ch(30.0, "S", 0), ch(40.0, "S", 1), ch(50.0, "A", 0), ch(60.0, "S", 0)],
                clocks: vec![],
                t_end_ns: 70.0,
            },
        },
        "sr_latch" => Circuit {
            name: name.into(),
            title: "SR latch (cross-coupled NOR)".into(),
            description: "Two NOR gates feeding each other store one bit. S sets Q, R resets it; S = R = 0 holds. Q starts unknown (X) until the first reset.".into(),
            inputs: vec!["S".into(), "R".into()],
            outputs: vec!["Q".into(), "Qn".into()],
            gates: vec![g("nor1", Nor, &["R", "Qn"], "Q"), g("nor2", Nor, &["S", "Q"], "Qn")],
            stimulus: Stimulus { changes: vec![ch(0.0, "S", 0), ch(0.0, "R", 0), ch(10.0, "R", 1), ch(20.0, "R", 0), ch(30.0, "S", 1), ch(40.0, "S", 0), ch(60.0, "R", 1), ch(70.0, "R", 0)], clocks: vec![], t_end_ns: 90.0 },
        },
        "counter4" => Circuit {
            name: name.into(),
            title: "4-bit synchronous counter".into(),
            description: "Four D flip-flops with an incrementer: Dᵢ = Qᵢ ⊕ (Q₀·…·Qᵢ₋₁). The clock-to-Q delay plus the carry chain sets the maximum clock rate.".into(),
            inputs: vec!["CLK".into()],
            outputs: vec!["Q0".into(), "Q1".into(), "Q2".into(), "Q3".into()],
            gates: vec![
                g("n0", Not, &["Q0"], "D0"),
                g("x1", Xor, &["Q1", "Q0"], "D1"),
                g("a1", And, &["Q0", "Q1"], "c2"),
                g("x2", Xor, &["Q2", "c2"], "D2"),
                g("a2", And, &["c2", "Q2"], "c3"),
                g("x3", Xor, &["Q3", "c3"], "D3"),
                g("ff0", Dff, &["D0", "CLK"], "Q0"),
                g("ff1", Dff, &["D1", "CLK"], "Q1"),
                g("ff2", Dff, &["D2", "CLK"], "Q2"),
                g("ff3", Dff, &["D3", "CLK"], "Q3"),
            ],
            stimulus: Stimulus { changes: vec![], clocks: vec![Clock { net: "CLK".into(), period_ns: 20.0, start_ns: 10.0 }], t_end_ns: 350.0 },
        },
        _ => return None,
    })
}

pub const BUILTINS: [&str; 6] = ["half_adder", "full_adder", "rca4", "mux2", "sr_latch", "counter4"];

#[derive(Clone, Debug, Serialize)]
pub struct Wave {
    pub net: String,
    /// "input", "output" or "internal".
    pub role: String,
    /// Step series: (time ns, value 0/1/2=X).
    pub points: Vec<(f64, u8)>,
    pub transitions: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Glitch {
    pub net: String,
    pub t0: f64,
    pub t1: f64,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct CriticalPath {
    pub delay_ns: f64,
    pub gates: Vec<String>,
    pub nets: Vec<String>,
    pub from: String,
    pub to: String,
    /// Maximum clock frequency if the path is register-to-register (MHz).
    pub fmax_mhz: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PlacedGate {
    pub id: String,
    pub kind: GateKind,
    pub inputs: Vec<String>,
    pub output: String,
    pub delay_ns: f64,
    pub level: usize,
    pub row: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct SimResult {
    pub name: String,
    pub title: String,
    pub description: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub gates: Vec<PlacedGate>,
    pub levels: usize,
    pub t_end_ns: f64,
    pub waves: Vec<Wave>,
    pub glitches: Vec<Glitch>,
    pub critical: CriticalPath,
    pub events: usize,
    pub has_loop: bool,
    pub stimulus: Stimulus,
}

#[derive(PartialEq)]
struct Ev {
    t: f64,
    seq: u64,
    net: usize,
    v: u8,
}
impl Eq for Ev {}
impl Ord for Ev {
    fn cmp(&self, o: &Self) -> Ordering {
        // min-heap on (t, seq)
        o.t.partial_cmp(&self.t).unwrap_or(Ordering::Equal).then(o.seq.cmp(&self.seq))
    }
}
impl PartialOrd for Ev {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// Simulate a circuit with its stimulus. `delay_scale` multiplies every gate delay.
pub fn simulate(c: &Circuit, delay_scale: f64, glitch_ns: f64) -> SimResult {
    let mut names: Vec<String> = Vec::new();
    let mut idx: HashMap<String, usize> = HashMap::new();
    let mut net = |s: &str, names: &mut Vec<String>| -> usize {
        if let Some(&i) = idx.get(s) {
            i
        } else {
            names.push(s.to_string());
            idx.insert(s.to_string(), names.len() - 1);
            names.len() - 1
        }
    };
    for i in &c.inputs {
        net(i, &mut names);
    }
    let gates: Vec<(GateKind, Vec<usize>, usize, f64)> = c
        .gates
        .iter()
        .map(|gt| {
            let ins: Vec<usize> = gt.inputs.iter().map(|s| net(s, &mut names)).collect();
            let out = net(&gt.output, &mut names);
            (
                gt.kind,
                ins,
                out,
                gt.delay_ns.unwrap_or(gt.kind.default_delay()) * delay_scale.max(0.01),
            )
        })
        .collect();
    for o in &c.outputs {
        net(o, &mut names);
    }
    let nn = names.len();
    let mut fanout: Vec<Vec<usize>> = vec![vec![]; nn];
    for (gi, (_, ins, _, _)) in gates.iter().enumerate() {
        for &i in ins {
            if !fanout[i].contains(&gi) {
                fanout[i].push(gi);
            }
        }
    }
    let mut val = vec![X; nn];
    let mut waves: Vec<Vec<(f64, u8)>> = vec![vec![(0.0, X)]; nn];
    let mut heap = BinaryHeap::new();
    let mut seq = 0u64;
    let push = |heap: &mut BinaryHeap<Ev>, t: f64, net: usize, v: u8, seq: &mut u64| {
        *seq += 1;
        heap.push(Ev { t, seq: *seq, net, v });
    };
    let t_end = c.stimulus.t_end_ns.max(1.0);
    for (t, n, v) in &c.stimulus.changes {
        if let Some(&i) = idx.get(n) {
            push(&mut heap, *t, i, (*v).min(1), &mut seq);
        }
    }
    for clk in &c.stimulus.clocks {
        if let Some(&i) = idx.get(&clk.net) {
            push(&mut heap, 0.0, i, 0, &mut seq);
            let half = (clk.period_ns / 2.0).max(0.1);
            let mut t = clk.start_ns;
            let mut v = 1u8;
            while t <= t_end {
                push(&mut heap, t, i, v, &mut seq);
                v = 1 - v;
                t += half;
            }
        }
    }
    // flip-flops power up at 0 (as if reset); constant gates drive at t = 0
    for (kind, ins, out, _) in &gates {
        match kind {
            GateKind::Dff => push(&mut heap, 0.0, *out, 0, &mut seq),
            GateKind::Tie0 | GateKind::Tie1 => push(
                &mut heap,
                0.0,
                *out,
                eval(*kind, ins.iter().map(|_| 0).collect::<Vec<_>>().as_slice()),
                &mut seq,
            ),
            _ => {}
        }
    }
    let mut scheduled: Vec<u8> = vec![X; nn];
    let mut events = 0usize;
    let max_events = 200_000;
    while let Some(ev) = heap.pop() {
        if ev.t > t_end || events > max_events {
            break;
        }
        events += 1;
        if val[ev.net] == ev.v {
            continue;
        }
        let old = val[ev.net];
        val[ev.net] = ev.v;
        waves[ev.net].push((ev.t, ev.v));
        for &gi in &fanout[ev.net] {
            let (kind, ins, out, d) = &gates[gi];
            if *kind == GateKind::Dff {
                // rising edge on CLK (input 1) samples D (input 0)
                if ins.get(1) == Some(&ev.net) && old == 0 && ev.v == 1 {
                    let dv = val[ins[0]];
                    push(&mut heap, ev.t + d, *out, dv, &mut seq);
                }
                continue;
            }
            let inv: Vec<u8> = ins.iter().map(|&i| val[i]).collect();
            let nv = eval(*kind, &inv);
            if nv != scheduled[*out] {
                scheduled[*out] = nv;
                push(&mut heap, ev.t + d, *out, nv, &mut seq);
            }
        }
    }
    // roles and glitches
    let role = |n: &str| {
        if c.inputs.iter().any(|x| x == n) {
            "input"
        } else if c.outputs.iter().any(|x| x == n) {
            "output"
        } else {
            "internal"
        }
    };
    let mut glitches = Vec::new();
    for (i, w) in waves.iter().enumerate() {
        if role(&names[i]) == "input" {
            continue;
        }
        for k in 1..w.len().saturating_sub(1) {
            let (t0, v0) = w[k];
            let (t1, _) = w[k + 1];
            if w[k - 1].1 != X && v0 != X && t1 - t0 < glitch_ns && w[k + 1].1 == w[k - 1].1 {
                glitches.push(Glitch {
                    net: names[i].clone(),
                    t0,
                    t1,
                });
            }
        }
    }
    // order waves: inputs, outputs, internals
    let mut order: Vec<usize> = (0..nn).collect();
    order.sort_by_key(|&i| {
        let r = role(&names[i]);
        let rank = if r == "input" {
            0
        } else if r == "output" {
            1
        } else {
            2
        };
        let pos = c
            .inputs
            .iter()
            .position(|x| x == &names[i])
            .or_else(|| c.outputs.iter().position(|x| x == &names[i]))
            .unwrap_or(i);
        (rank, pos)
    });
    let wave_out: Vec<Wave> = order
        .iter()
        .map(|&i| {
            let mut pts = waves[i].clone();
            pts.push((t_end, val[i]));
            Wave {
                net: names[i].clone(),
                role: role(&names[i]).into(),
                transitions: waves[i].len().saturating_sub(1),
                points: pts,
            }
        })
        .collect();
    let (placed, levels, has_loop) = place(c, &gates, &names);
    SimResult {
        name: c.name.clone(),
        title: c.title.clone(),
        description: c.description.clone(),
        inputs: c.inputs.clone(),
        outputs: c.outputs.clone(),
        gates: placed,
        levels,
        t_end_ns: t_end,
        waves: wave_out,
        glitches,
        critical: critical_path(c, &gates, &names, &idx),
        events,
        has_loop,
        stimulus: c.stimulus.clone(),
    }
}

type NetGate = (GateKind, Vec<usize>, usize, f64);

/// Longest path through combinational gates (registers break paths).
fn critical_path(c: &Circuit, gates: &[NetGate], names: &[String], idx: &HashMap<String, usize>) -> CriticalPath {
    let nn = names.len();
    let mut arrival = vec![f64::NEG_INFINITY; nn];
    let mut pred: Vec<Option<usize>> = vec![None; nn]; // driving gate on the longest path
    let mut has_reg = false;
    for i in &c.inputs {
        if let Some(&k) = idx.get(i) {
            arrival[k] = 0.0;
        }
    }
    for (gi, (kind, _, out, d)) in gates.iter().enumerate() {
        if *kind == GateKind::Dff {
            arrival[*out] = *d; // clock-to-Q
            pred[*out] = Some(gi);
            has_reg = true;
        } else if matches!(kind, GateKind::Tie0 | GateKind::Tie1) {
            arrival[*out] = 0.0;
        }
    }
    // Bellman-Ford style relaxation (bounded iterations handle loops)
    for _ in 0..(gates.len() + 1) {
        let mut changed = false;
        for (gi, (kind, ins, out, d)) in gates.iter().enumerate() {
            if matches!(kind, GateKind::Dff | GateKind::Tie0 | GateKind::Tie1) {
                continue;
            }
            let a = ins.iter().map(|&i| arrival[i]).fold(f64::NEG_INFINITY, f64::max);
            if a.is_finite() && a + d > arrival[*out] + 1e-12 && a + d < 1e6 {
                arrival[*out] = a + d;
                pred[*out] = Some(gi);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // endpoints: primary outputs and D inputs of flip-flops
    let mut ends: Vec<usize> = c.outputs.iter().filter_map(|o| idx.get(o).copied()).collect();
    for (kind, ins, _, _) in gates {
        if *kind == GateKind::Dff {
            ends.push(ins[0]);
        }
    }
    let Some(&end) = ends
        .iter()
        .filter(|&&e| arrival[e].is_finite())
        .max_by(|a, b| arrival[**a].partial_cmp(&arrival[**b]).unwrap_or(Ordering::Equal))
    else {
        return CriticalPath::default();
    };
    let mut nets = vec![end];
    let mut gs = Vec::new();
    let mut cur = end;
    let mut guard = 0;
    while let Some(gi) = pred[cur] {
        guard += 1;
        if guard > gates.len() + 2 {
            break;
        }
        gs.push(c.gates[gi].id.clone());
        if gates[gi].0 == GateKind::Dff {
            break;
        }
        let ins = &gates[gi].1;
        let Some(&best) = ins
            .iter()
            .max_by(|a, b| arrival[**a].partial_cmp(&arrival[**b]).unwrap_or(Ordering::Equal))
        else {
            break;
        };
        nets.push(best);
        cur = best;
    }
    nets.reverse();
    gs.reverse();
    let delay = arrival[end];
    // register-to-register paths set fmax (add a nominal 1 ns setup)
    let fmax = if has_reg { Some(1000.0 / (delay + 1.0)) } else { None };
    CriticalPath {
        delay_ns: delay,
        from: names[nets[0]].clone(),
        to: names[end].clone(),
        gates: gs,
        nets: nets.iter().map(|&n| names[n].clone()).collect(),
        fmax_mhz: fmax,
    }
}

/// Assign each gate a drawing column (logic level) and row.
fn place(c: &Circuit, gates: &[NetGate], names: &[String]) -> (Vec<PlacedGate>, usize, bool) {
    let nn = names.len();
    let mut level = vec![0usize; nn];
    let mut has_loop = false;
    let n_iter = gates.len() + 1;
    for it in 0..=n_iter {
        let mut changed = false;
        for (kind, ins, out, _) in gates {
            if *kind == GateKind::Dff {
                continue;
            }
            let l = ins.iter().map(|&i| level[i]).max().unwrap_or(0) + 1;
            if l > level[*out] && l <= gates.len() + 1 {
                level[*out] = l;
                changed = true;
            }
        }
        if !changed {
            break;
        }
        if it == n_iter {
            has_loop = true;
        }
    }
    let mut gl: Vec<usize> = gates
        .iter()
        .map(|(kind, ins, out, _)| {
            if *kind == GateKind::Dff {
                ins.iter().map(|&i| level[i]).max().unwrap_or(0) + 1
            } else {
                level[*out]
            }
        })
        .collect();
    // cross-coupled latches: detect a loop by a gate feeding itself through levels
    for (gi, (_, ins, out, _)) in gates.iter().enumerate() {
        if ins.iter().any(|&i| gates.iter().any(|(_, i2, o2, _)| *o2 == i && i2.contains(out))) {
            has_loop = true;
            gl[gi] = 1;
        }
    }
    let levels = gl.iter().cloned().max().unwrap_or(0) + 1;
    let mut rows = vec![0usize; levels + 1];
    let placed = c
        .gates
        .iter()
        .enumerate()
        .map(|(gi, gt)| {
            let l = gl[gi];
            let r = rows[l];
            rows[l] += 1;
            PlacedGate {
                id: gt.id.clone(),
                kind: gt.kind,
                inputs: gt.inputs.clone(),
                output: gt.output.clone(),
                delay_ns: gates[gi].3,
                level: l,
                row: r,
            }
        })
        .collect();
    (placed, levels, has_loop)
}

/// Build a circuit from the logic minimiser's SOP netlist with an exhaustive stimulus.
pub fn from_logic(vars: &[String], gates: &[super::logic::Gate]) -> Circuit {
    let gs = gates
        .iter()
        .map(|g| SimGate {
            id: g.id.clone(),
            kind: GateKind::from_name(&g.kind).unwrap_or(GateKind::Buf),
            inputs: g.inputs.clone(),
            output: g.output.clone(),
            delay_ns: None,
        })
        .collect();
    let n = vars.len();
    let mut changes = Vec::new();
    for m in 0..(1u32 << n) {
        for (i, v) in vars.iter().enumerate() {
            changes.push((m as f64 * 10.0, v.clone(), ((m >> (n - 1 - i)) & 1) as u8));
        }
    }
    Circuit {
        name: "custom".into(),
        title: "Minimised function".into(),
        description: "The SOP netlist from the logic minimiser, driven through every input combination.".into(),
        inputs: vars.to_vec(),
        outputs: vec!["F".into()],
        gates: gs,
        stimulus: Stimulus {
            changes,
            clocks: vec![],
            t_end_ns: (1u32 << n) as f64 * 10.0,
        },
    }
}

/// Value of `net` at time `t` from a simulation result.
pub fn value_at(r: &SimResult, net: &str, t: f64) -> u8 {
    let w = r.waves.iter().find(|w| w.net == net).expect("net");
    let mut v = X;
    for &(tt, vv) in &w.points {
        if tt <= t {
            v = vv;
        } else {
            break;
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_adder_truth_table_by_simulation() {
        let c = builtin("full_adder").unwrap();
        let r = simulate(&c, 1.0, 4.0);
        for k in 0..8u8 {
            let t = k as f64 * 10.0 + 9.5;
            let (a, b, ci) = ((k >> 2) & 1, (k >> 1) & 1, k & 1);
            let sum = a + b + ci;
            assert_eq!(value_at(&r, "S", t), sum & 1, "S at k={k}");
            assert_eq!(value_at(&r, "Cout", t), sum >> 1, "Cout at k={k}");
        }
        // critical path A → x1 → x2 → S = 6 ns (two XORs), or A → x1 → a2 → o1 = 7 ns
        assert!((r.critical.delay_ns - 7.0).abs() < 1e-9, "{:?}", r.critical);
    }

    #[test]
    fn ripple_carry_adder_adds_and_glitches() {
        let c = builtin("rca4").unwrap();
        let r = simulate(&c, 1.0, 5.0);
        let read = |t: f64| {
            let s: u32 = (0..4).map(|i| (value_at(&r, &format!("S{i}"), t) as u32) << i).sum();
            s + ((value_at(&r, "Cout", t) as u32) << 4)
        };
        assert_eq!(read(49.0), 0b0111 + 0b0001);
        assert_eq!(read(69.0), 0b1111);
        assert_eq!(read(109.0), 0b1111 + 1);
        assert_eq!(read(149.0), 0b0101 + 0b0011);
        assert!(!r.glitches.is_empty(), "carry ripple should produce glitches");
        // critical path: A0 → p0 (3) → t0 (2) → c1 (2) … → Cout: 3 + 4·(2+2) = 19 ns
        assert!((r.critical.delay_ns - 19.0).abs() < 1e-9, "{:?}", r.critical);
        assert_eq!(r.critical.to, "Cout");
    }

    #[test]
    fn mux_static_hazard_and_counter() {
        let r = simulate(&builtin("mux2").unwrap(), 1.0, 3.0);
        assert!(r.glitches.iter().any(|g| g.net == "Y"), "static-1 hazard on Y");
        let r = simulate(&builtin("counter4").unwrap(), 1.0, 3.0);
        // after k rising edges (at 10, 30, 50, …) the count is k mod 16
        let count = |t: f64| (0..4).map(|i| (value_at(&r, &format!("Q{i}"), t) as u32) << i).sum::<u32>();
        assert_eq!(count(5.0), 0);
        assert_eq!(count(29.0), 1);
        assert_eq!(count(9.0 + 20.0 * 10.0), 10);
        assert_eq!(count(9.0 + 20.0 * 17.0), 1);
        assert!(r.critical.fmax_mhz.is_some());
    }

    #[test]
    fn sr_latch_holds_state() {
        let r = simulate(&builtin("sr_latch").unwrap(), 1.0, 2.0);
        assert_eq!(value_at(&r, "Q", 5.0), X);
        assert_eq!(value_at(&r, "Q", 25.0), 0);
        assert_eq!(value_at(&r, "Q", 50.0), 1);
        assert_eq!(value_at(&r, "Q", 80.0), 0);
        assert!(r.has_loop);
    }
}
