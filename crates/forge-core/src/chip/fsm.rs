//! Data-driven finite-state machines, with the Iris touch-gesture recogniser
//! as the worked example, plus a synthesizable Verilog generator (binary or
//! one-hot encoding) and a self-checking testbench.
//!
//! The FSM is a **Moore machine** (outputs depend only on the state, so
//! they come straight from flip-flops through a small decoder and cannot
//! glitch on input changes — E. F. Moore, "Gedanken-experiments on
//! sequential machines", 1956; contrast G. H. Mealy, 1955, whose outputs
//! depend on state *and* input). A free-running timer counts 1 kHz ticks
//! since the current state was entered; transitions are guarded on the
//! `touch` input and on `timer >= N` / `timer < N`. The first matching
//! transition (in listed order) wins; otherwise the machine stays put.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Guard {
    pub touch: Option<bool>,
    pub timer_ge: Option<u32>,
    pub timer_lt: Option<u32>,
}

impl Guard {
    pub fn matches(&self, touch: bool, timer: u32) -> bool {
        self.touch.is_none_or(|t| t == touch) && self.timer_ge.is_none_or(|n| timer >= n) && self.timer_lt.is_none_or(|n| timer < n)
    }
    pub fn label(&self) -> String {
        let mut p = Vec::new();
        if let Some(t) = self.touch {
            p.push(if t { "touch".to_string() } else { "!touch".to_string() });
        }
        if let Some(n) = self.timer_ge {
            p.push(format!("t≥{n}"));
        }
        if let Some(n) = self.timer_lt {
            p.push(format!("t<{n}"));
        }
        if p.is_empty() {
            "always".into()
        } else {
            p.join(" · ")
        }
    }
    fn verilog(&self, tb: u32) -> String {
        let mut p = Vec::new();
        if let Some(t) = self.touch {
            p.push(if t { "touch".to_string() } else { "!touch".to_string() });
        }
        if let Some(n) = self.timer_ge {
            p.push(format!("(timer >= {tb}'d{n})"));
        }
        if let Some(n) = self.timer_lt {
            p.push(format!("(timer < {tb}'d{n})"));
        }
        if p.is_empty() {
            "1'b1".into()
        } else {
            p.join(" && ")
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub name: String,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub description: String,
    /// Diagram position in [0, 1]².
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transition {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub guard: Guard,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fsm {
    pub name: String,
    pub tick_hz: f64,
    pub outputs: Vec<String>,
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
    pub reset: String,
}

impl Fsm {
    pub fn index(&self, name: &str) -> Option<usize> {
        self.states.iter().position(|s| s.name == name)
    }
    /// Largest timer threshold used by a guard.
    pub fn max_threshold(&self) -> u32 {
        self.transitions.iter().flat_map(|t| [t.guard.timer_ge, t.guard.timer_lt]).flatten().max().unwrap_or(1)
    }
    /// Timer width: enough bits to hold the largest threshold (saturating counter).
    pub fn timer_bits(&self) -> u32 {
        let m = self.max_threshold().max(1);
        32 - m.leading_zeros()
    }
    pub fn state_bits(&self) -> u32 {
        let n = self.states.len().max(2) as u32;
        32 - (n - 1).leading_zeros()
    }
    /// One tick: returns (next state, next timer).
    pub fn step(&self, state: usize, timer: u32, touch: bool) -> (usize, u32) {
        let name = &self.states[state].name;
        let tmax = (1u32 << self.timer_bits()) - 1;
        for t in &self.transitions {
            if &t.from == name && t.guard.matches(touch, timer) {
                let to = self.index(&t.to).unwrap_or(0);
                return if to != state { (to, 0) } else { (to, (timer + 1).min(tmax)) };
            }
        }
        (state, (timer + 1).min(tmax))
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.states.is_empty() {
            return Err("FSM has no states".into());
        }
        if self.index(&self.reset).is_none() {
            return Err(format!("reset state `{}` not found", self.reset));
        }
        for t in &self.transitions {
            for s in [&t.from, &t.to] {
                if self.index(s).is_none() {
                    return Err(format!("transition refers to unknown state `{s}`"));
                }
            }
        }
        for s in &self.states {
            if !s.name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || s.name.is_empty() {
                return Err(format!("state name `{}` must be a Verilog identifier", s.name));
            }
        }
        Ok(())
    }
}

/// The Iris gesture recogniser with its three timing parameters (ticks at 1 kHz = ms).
pub fn gesture_fsm(tap_max_ms: u32, long_ms: u32, double_gap_ms: u32) -> Fsm {
    let st = |name: &str, outs: &[&str], d: &str, x: f64, y: f64| State { name: name.into(), outputs: outs.iter().map(|s| s.to_string()).collect(), description: d.into(), x, y };
    let tr = |from: &str, to: &str, touch: Option<bool>, ge: Option<u32>, lt: Option<u32>| Transition { from: from.into(), to: to.into(), guard: Guard { touch, timer_ge: ge, timer_lt: lt } };
    Fsm {
        name: "iris_gesture_fsm".into(),
        tick_hz: 1000.0,
        outputs: vec!["tap".into(), "double_tap".into(), "long_press".into()],
        states: vec![
            st("IDLE", &[], "Waiting for a finger.", 0.03, 0.5),
            st("PRESS", &[], "Finger down; the timer measures the press.", 0.27, 0.5),
            st("TAP_WAIT", &[], "Short press released; is a second tap coming?", 0.56, 0.14),
            st("TAP", &[], "Single tap recognised (one-tick pulse).", 0.95, 0.02),
            st("DOUBLE_TAP", &[], "Second press inside the gap window.", 0.95, 0.4),
            st("LONG_PRESS", &[], "Held past the long-press threshold.", 0.56, 0.92),
            st("WAIT_RELEASE", &[], "Gesture done; wait for the finger to lift.", 0.95, 0.82),
        ],
        transitions: vec![
            tr("IDLE", "PRESS", Some(true), None, None),
            tr("PRESS", "LONG_PRESS", Some(true), Some(long_ms), None),
            tr("PRESS", "TAP_WAIT", Some(false), None, Some(tap_max_ms)),
            tr("PRESS", "IDLE", Some(false), None, None),
            tr("TAP_WAIT", "DOUBLE_TAP", Some(true), None, None),
            tr("TAP_WAIT", "TAP", None, Some(double_gap_ms), None),
            tr("TAP", "IDLE", None, None, None),
            tr("DOUBLE_TAP", "WAIT_RELEASE", None, None, None),
            tr("LONG_PRESS", "WAIT_RELEASE", None, None, None),
            tr("WAIT_RELEASE", "IDLE", Some(false), None, None),
        ],
        reset: "IDLE".into(),
    }
    .with_outputs(&[("TAP", "tap"), ("DOUBLE_TAP", "double_tap"), ("LONG_PRESS", "long_press")])
}

impl Fsm {
    fn with_outputs(mut self, map: &[(&str, &str)]) -> Fsm {
        for (s, o) in map {
            if let Some(i) = self.index(s) {
                self.states[i].outputs.push(o.to_string());
            }
        }
        self
    }
}

/// Touch trace from (level, duration ms) segments at 1 kHz.
pub fn trace_from_segments(segs: &[(bool, u32)]) -> Vec<bool> {
    segs.iter().flat_map(|&(v, n)| std::iter::repeat_n(v, n as usize)).collect()
}

/// Named scripted traces for the lab.
pub fn scripted(name: &str) -> Vec<bool> {
    let segs: Vec<(bool, u32)> = match name {
        "tap" => vec![(false, 50), (true, 120), (false, 400)],
        "double_tap" => vec![(false, 50), (true, 100), (false, 120), (true, 110), (false, 300)],
        "long_press" => vec![(false, 50), (true, 900), (false, 250)],
        "slow_press" => vec![(false, 50), (true, 400), (false, 300)],
        _ => vec![(false, 50), (true, 120), (false, 400), (true, 90), (false, 100), (true, 90), (false, 350), (true, 800), (false, 200), (true, 400), (false, 200)],
    };
    trace_from_segments(&segs)
}

#[derive(Clone, Debug, Serialize)]
pub struct GestureEvent {
    pub tick: usize,
    pub output: String,
    pub state: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct FsmTrace {
    /// State index after each tick.
    pub states: Vec<u8>,
    pub timers: Vec<u32>,
    pub events: Vec<GestureEvent>,
    pub final_state: usize,
    pub final_timer: u32,
    /// Which states were visited, and which transitions (index into `transitions`) fired.
    pub fired: Vec<usize>,
}

pub fn simulate(f: &Fsm, touch: &[bool], state: usize, timer: u32) -> FsmTrace {
    let mut s = state.min(f.states.len().saturating_sub(1));
    let mut t = timer;
    let mut states = Vec::with_capacity(touch.len());
    let mut timers = Vec::with_capacity(touch.len());
    let mut events = Vec::new();
    let mut fired = Vec::new();
    for (k, &x) in touch.iter().enumerate() {
        let name = &f.states[s].name;
        if let Some(ti) = f.transitions.iter().position(|tr| &tr.from == name && tr.guard.matches(x, t)) {
            if !fired.contains(&ti) {
                fired.push(ti);
            }
        }
        let (ns, nt) = f.step(s, t, x);
        s = ns;
        t = nt;
        for o in &f.states[s].outputs {
            events.push(GestureEvent { tick: k, output: o.clone(), state: f.states[s].name.clone() });
        }
        states.push(s as u8);
        timers.push(t);
    }
    FsmTrace { states, timers, events, final_state: s, final_timer: t, fired }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    #[default]
    Binary,
    OneHot,
}

/// Synthesizable Verilog-2001 for the FSM.
pub fn verilog(f: &Fsm, enc: Encoding) -> String {
    let n = f.states.len();
    let tb = f.timer_bits();
    let sb = f.state_bits();
    let mut v = String::new();
    let enc_name = if enc == Encoding::OneHot { "one-hot" } else { "binary" };
    v += &format!(
        "// Generated by Protoforge Chip Lab — {} (Moore FSM, {} encoding)\n// Ticks at {:.0} Hz; `timer` counts ticks since the current state was entered.\n`timescale 1ns/1ps\n`default_nettype none\n\nmodule {} (\n    input  wire        clk,\n    input  wire        rst_n,\n    input  wire        tick,      // sample enable ({:.0} Hz)\n    input  wire        touch,     // synchronised touch input\n",
        f.name, enc_name, f.tick_hz, f.name, f.tick_hz
    );
    for o in &f.outputs {
        v += &format!("    output wire        {o},\n");
    }
    v += &format!("    output reg  [{}:0]  state_idx  // binary state index (debug / test)\n);\n", sb - 1);
    let sw = if enc == Encoding::OneHot { n as u32 } else { sb };
    for (i, s) in f.states.iter().enumerate() {
        match enc {
            Encoding::Binary => v += &format!("    localparam [{}:0] S_{} = {}'d{};\n", sw - 1, s.name, sw, i),
            Encoding::OneHot => v += &format!("    localparam integer I_{} = {};\n", s.name, i),
        }
    }
    let tmax = (1u64 << tb) - 1;
    v += &format!("\n    reg [{}:0] state, next;\n    reg [{}:0] timer;\n\n", sw - 1, tb - 1);
    // next-state logic
    v += "    always @* begin\n";
    match enc {
        Encoding::Binary => {
            v += "        next = state;\n        case (state)\n";
            for s in &f.states {
                v += &format!("            S_{}: begin\n", s.name);
                let mut first = true;
                for t in f.transitions.iter().filter(|t| t.from == s.name) {
                    let kw = if first { "if" } else { "else if" };
                    v += &format!("                {kw} ({}) next = S_{};\n", t.guard.verilog(tb), t.to);
                    first = false;
                }
                v += "            end\n";
            }
            v += &format!("            default: next = S_{};\n        endcase\n", f.reset);
        }
        Encoding::OneHot => {
            v += &format!("        next = {sw}'b0;\n        case (1'b1) // synopsys parallel_case\n");
            for s in &f.states {
                v += &format!("            state[I_{}]: begin\n", s.name);
                let mut first = true;
                for t in f.transitions.iter().filter(|t| t.from == s.name) {
                    let kw = if first { "if" } else { "else if" };
                    v += &format!("                {kw} ({}) next[I_{}] = 1'b1;\n", t.guard.verilog(tb), t.to);
                    first = false;
                }
                if first {
                    v += &format!("                next[I_{}] = 1'b1;\n", s.name);
                } else {
                    v += &format!("                else next[I_{}] = 1'b1;\n", s.name);
                }
                v += "            end\n";
            }
            v += &format!("            default: next[I_{}] = 1'b1;\n        endcase\n", f.reset);
        }
    }
    v += "    end\n\n";
    let reset_val = match enc {
        Encoding::Binary => format!("S_{}", f.reset),
        Encoding::OneHot => format!("{sw}'b1 << I_{}", f.reset),
    };
    v += &format!(
        "    always @(posedge clk or negedge rst_n) begin\n        if (!rst_n) begin\n            state <= {reset_val};\n            timer <= {tb}'d0;\n        end else if (tick) begin\n            state <= next;\n            if (next != state) timer <= {tb}'d0;\n            else if (timer != {tb}'d{tmax}) timer <= timer + {tb}'d1;\n        end\n    end\n\n"
    );
    // outputs
    for o in &f.outputs {
        let terms: Vec<String> = f
            .states
            .iter()
            .filter(|s| s.outputs.contains(o))
            .map(|s| match enc {
                Encoding::Binary => format!("(state == S_{})", s.name),
                Encoding::OneHot => format!("state[I_{}]", s.name),
            })
            .collect();
        let expr = if terms.is_empty() { "1'b0".to_string() } else { terms.join(" | ") };
        v += &format!("    assign {o} = {expr};\n");
    }
    // state index
    v += "\n    always @* begin\n";
    match enc {
        Encoding::Binary => v += &format!("        state_idx = state[{}:0];\n", sb - 1),
        Encoding::OneHot => {
            v += &format!("        state_idx = {sb}'d0;\n");
            for (i, s) in f.states.iter().enumerate() {
                v += &format!("        if (state[I_{}]) state_idx = {sb}'d{i};\n", s.name);
            }
        }
    }
    v += "    end\nendmodule\n\n`default_nettype wire\n";
    v
}

pub(crate) fn pack_hex(vals: &[u32], bits_each: u32) -> (usize, String) {
    // MSB first: element k occupies bits [bits_each*k +: bits_each]
    let total_bits = vals.len() * bits_each as usize;
    let width = total_bits.div_ceil(4) * 4;
    let mut bits = vec![0u8; width];
    for (k, &v) in vals.iter().enumerate() {
        for b in 0..bits_each as usize {
            bits[k * bits_each as usize + b] = ((v >> b) & 1) as u8;
        }
    }
    let mut s = String::with_capacity(width / 4);
    for nib in (0..width / 4).rev() {
        let d = (0..4).fold(0u8, |acc, b| acc | (bits[nib * 4 + b] << b));
        s.push(std::char::from_digit(d as u32, 16).unwrap());
    }
    (width, s)
}

/// Self-checking testbench: drives `touch`, compares `state_idx` after every tick.
pub fn testbench(f: &Fsm, touch: &[bool]) -> String {
    let tr = simulate(f, touch, f.index(&f.reset).unwrap_or(0), 0);
    let t = touch.len();
    let (tw, th) = pack_hex(&touch.iter().map(|&b| b as u32).collect::<Vec<_>>(), 1);
    let (ew, eh) = pack_hex(&tr.states.iter().map(|&s| s as u32).collect::<Vec<_>>(), 4);
    let sb = f.state_bits();
    let outs = f.outputs.iter().map(|o| format!("    wire {o};\n")).collect::<String>();
    let conns = f.outputs.iter().map(|o| format!(".{o}({o}), ")).collect::<String>();
    let counts = f
        .outputs
        .iter()
        .map(|o| {
            let n = tr.events.iter().filter(|e| &e.output == o).count();
            format!("        $display(\"  {o}: %0d pulses (expected {n})\", cnt_{o});\n        if (cnt_{o} != {n}) errors = errors + 1;\n")
        })
        .collect::<String>();
    let cnt_decl = f.outputs.iter().map(|o| format!("    integer cnt_{o} = 0;\n")).collect::<String>();
    let cnt_inc = f.outputs.iter().map(|o| format!("            if ({o}) cnt_{o} = cnt_{o} + 1;\n")).collect::<String>();
    format!(
        "// Self-checking testbench generated by Protoforge Chip Lab.\n// Expected states come from the Rust reference model (chip::fsm::simulate).\n// Run:  iverilog -g2005 -o tb {name}.v tb_{name}.v && vvp tb\n`timescale 1ns/1ps\n\nmodule tb_{name};\n    reg clk = 1'b0, rst_n = 1'b0, tick = 1'b1, touch = 1'b0;\n{outs}    wire [{sbm}:0] state_idx;\n\n    {name} dut (.clk(clk), .rst_n(rst_n), .tick(tick), .touch(touch), {conns}.state_idx(state_idx));\n\n    always #5 clk = ~clk;   // one clock per tick in simulation\n\n    localparam integer T = {t};\n    localparam [{twm}:0] TOUCH = {tw}'h{th};\n    localparam [{ewm}:0] EXP   = {ew}'h{eh};\n\n    integer i;\n    integer errors = 0;\n{cnt_decl}\n    initial begin\n        $dumpfile(\"tb_{name}.vcd\");\n        $dumpvars(0, tb_{name});\n        repeat (2) @(posedge clk);\n        #1 rst_n = 1'b1;\n        for (i = 0; i < T; i = i + 1) begin\n            touch = TOUCH[i];\n            @(posedge clk); #1;\n            if (state_idx !== EXP[4*i +: {sb}]) begin\n                errors = errors + 1;\n                if (errors < 10) $display(\"tick %0d: state %0d, expected %0d\", i, state_idx, EXP[4*i +: {sb}]);\n            end\n{cnt_inc}        end\n        $display(\"gesture pulses:\");\n{counts}        if (errors == 0) $display(\"PASS: %0d ticks match the reference model\", T);\n        else $display(\"FAIL: %0d mismatches\", errors);\n        $finish;\n    end\nendmodule\n",
        name = f.name,
        sbm = sb - 1,
        twm = tw - 1,
        ewm = ew - 1,
    )
}

/// Resource estimate for the FSM (standard cells) and its critical path.
#[derive(Clone, Debug, Serialize)]
pub struct FsmCost {
    pub state_ffs: u32,
    pub timer_ffs: u32,
    pub timer_incr_cells: u32,
    pub comparator_cells: u32,
    pub next_state_cells: u32,
    pub output_cells: u32,
    pub total_cells: u32,
    pub critical_ns: f64,
}

pub fn cost(f: &Fsm, enc: Encoding, gate_delay_ns: f64) -> FsmCost {
    let tb = f.timer_bits();
    let state_ffs = if enc == Encoding::OneHot { f.states.len() as u32 } else { f.state_bits() };
    let mut thresholds: Vec<u32> = f.transitions.iter().flat_map(|t| [t.guard.timer_ge, t.guard.timer_lt]).flatten().collect();
    thresholds.sort();
    thresholds.dedup();
    // a constant comparator on tb bits ≈ tb cells; incrementer ≈ 1 half-adder cell per bit + saturation detect
    let comparator_cells = thresholds.len() as u32 * tb;
    let timer_incr_cells = tb + tb / 2 + 2;
    // next-state: ~3 cells per guarded transition (+ decode of the source state for binary)
    let decode = if enc == Encoding::Binary { f.state_bits() } else { 0 };
    let next_state_cells = f.transitions.len() as u32 * (3 + decode) + state_ffs * 2;
    let output_cells = f.outputs.len() as u32 * if enc == Encoding::Binary { f.state_bits() } else { 1 };
    let total = state_ffs + tb + timer_incr_cells + comparator_cells + next_state_cells + output_cells;
    // path: timer register → comparator (log-depth, ~log2(tb)+1 levels) → next-state (≈3 levels) → timer clear mux
    let levels = (tb as f64).log2().ceil() + 1.0 + 3.0 + 2.0;
    FsmCost { state_ffs, timer_ffs: tb, timer_incr_cells, comparator_cells, next_state_cells, output_cells, total_cells: total, critical_ns: levels * gate_delay_ns * 2.0 + 3.0 * gate_delay_ns }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gestures(f: &Fsm, trace: &[bool]) -> Vec<String> {
        simulate(f, trace, 0, 0).events.into_iter().map(|e| e.output).collect()
    }

    #[test]
    fn tap_double_tap_long_press_traces() {
        let f = gesture_fsm(250, 600, 250);
        f.validate().unwrap();
        assert_eq!(gestures(&f, &scripted("tap")), vec!["tap"]);
        assert_eq!(gestures(&f, &scripted("double_tap")), vec!["double_tap"]);
        assert_eq!(gestures(&f, &scripted("long_press")), vec!["long_press"]);
        assert!(gestures(&f, &scripted("slow_press")).is_empty());
        assert_eq!(gestures(&f, &scripted("mixed")), vec!["tap", "double_tap", "long_press"]);
        // the tap is reported when the 250 ms gap expires after release
        let tr = simulate(&f, &scripted("tap"), 0, 0);
        let e = &tr.events[0];
        assert!(e.tick > 50 + 120 + 250 && e.tick < 50 + 120 + 255, "tap at tick {}", e.tick);
        // the long press fires ~600 ms into the hold
        let tr = simulate(&f, &scripted("long_press"), 0, 0);
        assert!((tr.events[0].tick as i64 - (50 + 601)).abs() <= 2, "long press at {}", tr.events[0].tick);
    }

    #[test]
    fn verilog_has_the_expected_structure() {
        let f = gesture_fsm(250, 600, 250);
        assert_eq!(f.timer_bits(), 10);
        assert_eq!(f.state_bits(), 3);
        let v = verilog(&f, Encoding::Binary);
        assert!(v.contains("module iris_gesture_fsm"));
        assert!(v.contains("timer >= 10'd600"));
        assert!(v.contains("assign tap = (state == S_TAP);"));
        let v1 = verilog(&f, Encoding::OneHot);
        assert!(v1.contains("reg [6:0] state, next;"));
        assert!(v1.contains("state[I_LONG_PRESS]"));
        let tb = testbench(&f, &scripted("mixed"));
        assert!(tb.contains("PASS"));
        assert!(tb.contains("localparam integer T = "));
    }

    #[test]
    fn hex_packing_is_lsb_element_first() {
        let (w, h) = pack_hex(&[1, 0, 0, 0, 1], 1);
        assert_eq!(w, 8);
        assert_eq!(h, "11");
        let (w, h) = pack_hex(&[3, 5], 4);
        assert_eq!((w, h.as_str()), (8, "53"));
    }
}
