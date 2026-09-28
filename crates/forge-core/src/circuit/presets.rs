//! Preset circuits: the Iris power and sensing circuits plus textbook
//! classics. Each preset carries an ngspice-compatible deck with `.param`
//! cards, the parameters exposed as sliders, default probes, a schematic
//! layout for the animated web view, and preset-specific measurements.
//!
//! ## Layout format
//!
//! Coordinates are grid units (the web page draws 1 unit ≈ 20 px), y down.
//! * `parts`: a symbol (`resistor`, `capacitor`, `inductor`, `vsource`,
//!   `isource`, `battery`, `diode`, `led`, `switch`, `nmos`, `pmos`,
//!   `opamp`, `vcvs`, `vccs`) placed at (x, y) with `dir` = direction from
//!   pin 1 to pin 2 (`down`, `up`, `left`, `right`). Two-terminal parts have
//!   pins at centre ∓ 2 units along `dir`. MOSFETs: gate 2 units left (dir
//!   `right`) or right (dir `left`), drain/source 2 units up/down (NMOS drain
//!   up, PMOS source up). Op-amp macro: in+ (−3, −1), in− (−3, +1), out
//!   (+3, 0). Every pin is resolved to its net name here (`pins`).
//! * `wires`: polylines on one net with a `flow` list `[(element, sign)]`
//!   whose signed sum is the current flowing from the first to the last
//!   point — the web page animates dots along the wire at that current.
//! * `grounds`: ground symbols (pin at the given point).
//! * `labels` (optionally `port`s that terminate a wire) and `boxes`
//!   (dashed outlines grouping a sub-circuit, e.g. "regulator model").
//! * `lpos` on a part moves its name/value label (`l`, `r`, `t`, `b`, `none`).

use std::collections::BTreeMap;

use serde::Serialize;

use super::measure::{at, average, crossing, f_3db, min_max, ripple_pp, settle_time};
use super::parser::{self, parse};
use super::{fmt_si, simulate, Annotation, Measurement, SimResult};
use crate::project::{Metric, Project};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct ParamSpec {
    pub name: String,
    pub label: String,
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub unit: String,
    pub log: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub hint: String,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct Pin {
    pub name: String,
    pub node: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct Part {
    /// Primary element name (hover shows its V/I/P).
    pub el: String,
    pub sym: String,
    pub x: f64,
    pub y: f64,
    pub dir: String,
    /// Elements represented by this symbol (macro-models), including `el`.
    pub members: Vec<String>,
    /// Label side: "" (auto), "l", "r", "t" (top), "b" (bottom) or "none".
    pub lpos: String,
    pub pins: Vec<Pin>,
    #[serde(skip)]
    pin_nodes: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Wire {
    pub node: String,
    pub pts: Vec<[f64; 2]>,
    pub flow: Vec<(String, f64)>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Label {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub node: Option<String>,
    /// A port terminates a wire (e.g. "to ADC").
    pub port: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct LBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub label: String,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct Layout {
    pub w: f64,
    pub h: f64,
    pub parts: Vec<Part>,
    pub wires: Vec<Wire>,
    pub grounds: Vec<[f64; 2]>,
    pub labels: Vec<Label>,
    pub boxes: Vec<LBox>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Link {
    pub lab: String,
    pub label: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Preset {
    pub id: String,
    pub title: String,
    /// "Iris" or "Classic"
    pub group: String,
    pub summary: String,
    pub description: String,
    /// How the circuit ties back to the Iris glasses.
    pub iris: String,
    pub netlist: String,
    pub params: Vec<ParamSpec>,
    /// Default probes, e.g. `v(vload)`, `i(L1)`.
    pub probes: Vec<String>,
    pub layout: Layout,
    pub links: Vec<Link>,
    /// Requirement metric this preset verifies, if any.
    pub metric: Option<String>,
}

// ---------------------------------------------------------------------------
// Layout builder
// ---------------------------------------------------------------------------

struct LB(Layout);

impl LB {
    fn new() -> Self {
        LB(Layout::default())
    }
    fn part(&mut self, el: &str, sym: &str, x: f64, y: f64, dir: &str) -> &mut Self {
        self.0.parts.push(Part {
            el: el.into(),
            sym: sym.into(),
            x,
            y,
            dir: dir.into(),
            members: vec![el.into()],
            ..Default::default()
        });
        self
    }
    /// Put the last part's label on another side.
    fn lpos(&mut self, side: &str) -> &mut Self {
        if let Some(p) = self.0.parts.last_mut() {
            p.lpos = side.into();
        }
        self
    }
    fn macro_part(&mut self, el: &str, sym: &str, x: f64, y: f64, members: &[&str], nodes: &[&str]) -> &mut Self {
        self.0.parts.push(Part {
            el: el.into(),
            sym: sym.into(),
            x,
            y,
            dir: "right".into(),
            members: members.iter().map(|s| s.to_string()).collect(),
            pin_nodes: Some(nodes.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        });
        self
    }
    fn wire(&mut self, node: &str, pts: &[(f64, f64)], flow: &[(&str, f64)]) -> &mut Self {
        self.0.wires.push(Wire {
            node: node.into(),
            pts: pts.iter().map(|(x, y)| [*x, *y]).collect(),
            flow: flow.iter().map(|(e, s)| (e.to_string(), *s)).collect(),
        });
        self
    }
    fn gnd(&mut self, x: f64, y: f64) -> &mut Self {
        self.0.grounds.push([x, y]);
        self
    }
    fn label(&mut self, text: &str, x: f64, y: f64, node: Option<&str>) -> &mut Self {
        self.0.labels.push(Label {
            text: text.into(),
            x,
            y,
            node: node.map(|s| s.into()),
            port: false,
        });
        self
    }
    fn port(&mut self, text: &str, x: f64, y: f64, node: &str) -> &mut Self {
        self.0.labels.push(Label {
            text: text.into(),
            x,
            y,
            node: Some(node.into()),
            port: true,
        });
        self
    }
    fn boxed(&mut self, x: f64, y: f64, w: f64, h: f64, label: &str) -> &mut Self {
        self.0.boxes.push(LBox {
            x,
            y,
            w,
            h,
            label: label.into(),
        });
        self
    }
    fn done(self) -> Layout {
        self.0
    }
}

fn dir_vec(dir: &str) -> (f64, f64) {
    match dir {
        "up" => (0.0, -1.0),
        "left" => (-1.0, 0.0),
        "right" => (1.0, 0.0),
        _ => (0.0, 1.0),
    }
}

/// Resolve pin coordinates and net names of every part from the netlist.
fn resolve_pins(layout: &mut Layout, nl: &parser::Netlist) {
    let (mut w, mut h) = (0.0f64, 0.0f64);
    for p in layout.parts.iter_mut() {
        let el = nl.elements.iter().find(|e| e.name.eq_ignore_ascii_case(&p.el));
        let names: Vec<String> = match (&p.pin_nodes, el) {
            (Some(n), _) => n.clone(),
            (None, Some(e)) => e.nodes.iter().map(|i| nl.nodes[*i].clone()).collect(),
            _ => vec![],
        };
        let get = |i: usize| names.get(i).cloned().unwrap_or_default();
        let (x, y) = (p.x, p.y);
        p.pins = match p.sym.as_str() {
            "nmos" | "pmos" => {
                let gx = if p.dir == "left" { x + 2.0 } else { x - 2.0 };
                let (top, bot) = if p.sym == "nmos" {
                    (("d", 0), ("s", 2))
                } else {
                    (("s", 2), ("d", 0))
                };
                vec![
                    Pin {
                        name: top.0.into(),
                        node: get(top.1),
                        x,
                        y: y - 2.0,
                    },
                    Pin {
                        name: "g".into(),
                        node: get(1),
                        x: gx,
                        y,
                    },
                    Pin {
                        name: bot.0.into(),
                        node: get(bot.1),
                        x,
                        y: y + 2.0,
                    },
                ]
            }
            "opamp" => vec![
                Pin {
                    name: "in+".into(),
                    node: get(0),
                    x: x - 3.0,
                    y: y - 1.0,
                },
                Pin {
                    name: "in-".into(),
                    node: get(1),
                    x: x - 3.0,
                    y: y + 1.0,
                },
                Pin {
                    name: "out".into(),
                    node: get(2),
                    x: x + 3.0,
                    y,
                },
            ],
            sym => {
                let (dx, dy) = dir_vec(&p.dir);
                let (n1, n2) = match sym {
                    "diode" | "led" => ("a", "k"),
                    "vsource" | "battery" | "vcvs" => ("+", "-"),
                    _ => ("1", "2"),
                };
                vec![
                    Pin {
                        name: n1.into(),
                        node: get(0),
                        x: x - 2.0 * dx,
                        y: y - 2.0 * dy,
                    },
                    Pin {
                        name: n2.into(),
                        node: get(1),
                        x: x + 2.0 * dx,
                        y: y + 2.0 * dy,
                    },
                ]
            }
        };
        for pin in &p.pins {
            w = w.max(pin.x + 2.0);
            h = h.max(pin.y + 2.0);
        }
    }
    for wr in &layout.wires {
        for pt in &wr.pts {
            w = w.max(pt[0] + 1.0);
            h = h.max(pt[1] + 1.0);
        }
    }
    for g in &layout.grounds {
        h = h.max(g[1] + 2.0);
    }
    for b in &layout.boxes {
        w = w.max(b.x + b.w + 0.5);
        h = h.max(b.y + b.h + 0.5);
    }
    for l in &layout.labels {
        // rough text width: ~0.34 grid units per character + a voltage readout
        let extra = if l.node.is_some() { 3.0 } else { 0.5 };
        w = w.max(l.x + 0.34 * l.text.chars().count() as f64 + extra);
    }
    layout.w = w;
    layout.h = h;
}

// ---------------------------------------------------------------------------
// Preset definitions
// ---------------------------------------------------------------------------

fn ps(name: &str, label: &str, value: f64, min: f64, max: f64, unit: &str, log: bool) -> ParamSpec {
    ParamSpec {
        name: name.into(),
        label: label.into(),
        value,
        min,
        max,
        unit: unit.into(),
        log,
        hint: String::new(),
    }
}

fn link(lab: &str, label: &str) -> Link {
    Link {
        lab: lab.into(),
        label: label.into(),
    }
}

const RAIL: &str = "Iris 1.8 V rail - display turn-on load step
* The buck regulator is modelled by its closed-loop output impedance:
* Zout = Rreg + (Lloop || Rdamp); Lloop = 1/((2*pi*fc)^2*Cout) makes |Zout|
* cross the output capacitor's impedance at the loop crossover fc.
.param vnom=1.8
.param fc=100k
.param q=1
.param cout=10u
.param esr=5m
.param esl=0.5n
.param rreg=20m
.param rbrd=30m
.param lbrd=5n
.param cdec=100n
.param ibase=1m
.param istep=60m
.param tedge=1u
.param lloop={1/((2*pi*fc)**2*cout)}
.param rdamp={q*sqrt(lloop/cout)}
Vreg vset 0 {vnom}
Rreg vset n1 {rreg}
Lloop n1 vbuck {lloop}
Rdamp n1 vbuck {rdamp}
Cout vbuck ncap {cout}
Resr ncap nesl {esr}
Lesl nesl 0 {esl}
Rbrd vbuck nb {rbrd}
Lbrd nb vload {lbrd}
Cdec vload 0 {cdec}
Iload vload 0 PULSE({ibase} {istep} 10u {tedge} {tedge} 1 2)
.tran 50n 60u
.end
";

const BUCK: &str = "Iris 1.8 V synchronous buck converter (open loop, started at steady state)
* Ideal switches with RON, body diodes for the dead time, inductor DCR and
* output-capacitor ESR. The inductor current and capacitor voltage start at
* their steady-state values (UIC) so the ripple is visible immediately.
.param vin=3.7
.param vout=1.8
.param iout=150m
.param fsw=2meg
.param l=2.2u
.param dcr=60m
.param cout=10u
.param esr=5m
.param rsw=50m
.param tdead=5n
.param vf=0.65
.param tsw={1/fsw}
.param rload={vout/iout}
.param duty={(vout + iout*(rsw + dcr) + vf*2*tdead/tsw)/vin}
.param ton={duty*tsw}
.param dil={(vin - vout)*duty/(l*fsw)}
.param il0={iout - dil/2}
Vin vin 0 {vin}
S1 vin sw g1 0 SWPWR
S2 sw 0 g2 0 SWPWR
Dhs sw vin DBODY
Dls 0 sw DBODY
L1 sw lx {l} IC={il0}
Rdcr lx out {dcr}
C2 out nc {cout} IC={vout}
Resr nc 0 {esr}
Rload out 0 {rload}
Vg1 g1 0 PULSE(0 1 0 1e-9 1e-9 {ton-1e-9} {tsw})
Vg2 g2 0 PULSE(0 1 {ton+tdead} 1e-9 1e-9 {tsw-ton-2*tdead-1e-9} {tsw})
.model SWPWR SW(RON={rsw} ROFF=1meg VT=0.5 VH=0)
.model DBODY D(IS=1e-12 N=1.1 RS=20m)
.tran 10n 12u 0 10n uic
.end
";

const VBAT: &str = "Iris battery-sense divider into the SoC ADC
* R1/R2 scale the cell voltage, C6 is the reservoir that feeds the ADC's
* sample-and-hold (S1 + Chold), which is reset (S2) before every sample.
.param vbat=4.2
.param rtop=1meg
.param rbot=1.2meg
.param cf=100n
.param chold=5p
.param ron=1k
.param fs=50
.param tacq=10u
Vbat bat 0 {vbat}
R1 bat adc {rtop}
R2 adc 0 {rbot}
C6 adc 0 {cf} IC=0
S1 adc hold samp 0 SWADC
Chold hold 0 {chold} IC=0
S2 hold 0 rst 0 SWADC
Vsamp samp 0 PULSE(0 1 {1/fs} 100n 100n {tacq} {1/fs})
Vrst rst 0 PULSE(0 1 {1/fs+2*tacq} 100n 100n {tacq} {1/fs})
.model SWADC SW(RON={ron} ROFF=1e12 VT=0.5 VH=0.1)
.tran 1m 600m uic
.end
";

const IRLED: &str = "Iris wear-detection IR LED driver (low-side NMOS switch)
* A 1.8 V GPIO drives a logic-level NMOS through a gate resistor; the IR LED
* current is set by Rled. Ciss is the MOSFET input capacitance.
.param vdd=3.3
.param rled=33
.param rg=100
.param ciss=1n
.param duty=0.1
.param fpwm=1k
.param vgate=1.8
Vdd vdd 0 {vdd}
Rled vdd a {rled}
D1 a d IRLED
M1 d g 0 0 NLOGIC W=10m L=1u
Rg gp g {rg}
Ciss g 0 {ciss}
Vpwm gp 0 PULSE(0 {vgate} 50u 20n 20n {duty/fpwm} {1/fpwm})
.model IRLED D(IS=8e-15 N=1.6 RS=1 CJO=30p)
.model NLOGIC NMOS(LEVEL=1 VTO=0.8 KP=100u LAMBDA=0.01)
.tran 2u 2.2m
.end
";

const TOUCH: &str = "Iris capacitive touch strip - RC charge time with and without a finger
* Two identical pads are charged through R from a GPIO. The finger adds Cfin
* in series with the body's capacitance to earth (Cbody); Rleak keeps the
* body node's DC operating point defined (SPICE needs a DC path).
.param vdd=1.8
.param r=1meg
.param cpad=12p
.param cfinger=10p
.param cbody=150p
Vdrv drv 0 PULSE(0 {vdd} 2u 5n 5n 60u 120u)
Rn drv pad_nf {r}
Cn pad_nf 0 {cpad}
Rf drv pad_f {r}
Cp pad_f 0 {cpad}
Cfin pad_f body {cfinger}
Cbody body 0 {cbody}
Rleak body 0 100meg
.tran 20n 125u
.end
";

const CMOS: &str = "CMOS inverter - voltage transfer characteristic and switching
* Level-1 (Shichman-Hodges) models with 0.18 um-like parameters.
.param vdd=1.8
.param wn=0.36u
.param wp=1.08u
.param lch=0.18u
.param cl=20f
Vdd vdd 0 {vdd}
Vin in 0 DC 0 PULSE(0 {vdd} 100p 50p 50p 1n 2n)
Mp out in vdd vdd PMOD W={wp} L={lch}
Mn out in 0 0 NMOD W={wn} L={lch}
Cl out 0 {cl}
.model NMOD NMOS(LEVEL=1 VTO=0.45 KP=170u LAMBDA=0.08)
.model PMOD PMOS(LEVEL=1 VTO=-0.45 KP=60u LAMBDA=0.1)
.dc Vin 0 {vdd} 0.01
.tran 2p 4.2n
.end
";

const RCAA: &str = "2nd-order RC anti-alias filter in front of a 16 kS/s ADC
* A 1 kHz wanted signal plus a 20 kHz interferer, which would alias to
* |20k - 16k| = 4 kHz after sampling.
.param r1=1k
.param c1=15n
.param r2=10k
.param c2=1.5n
.param fsig=1k
.param fint=20k
Vsig in x SIN(0 0.5 {fsig}) AC 1
Vint x 0 SIN(0 0.2 {fint})
R1 in n1 {r1}
C1 n1 0 {c1}
R2 n1 out {r2}
C2 out 0 {c2}
.ac dec 40 10 10meg
.tran 2u 3m
.end
";

const OPAMP: &str = "Non-inverting preamp with a single-pole op-amp macro-model
* E1 (open-loop gain A0) -> Rp/Cp (dominant pole at GBW/A0) -> E2 buffer ->
* Ro (output resistance). Closed-loop gain = 1 + Rf/Rg.
.param a0=100k
.param gbw=1meg
.param rf=19k
.param rg=1k
.param ro=50
.param rl=10k
.param vpk=10m
.param rp=1k
.param cp={a0/(2*pi*gbw*rp)}
Vin in 0 DC 0 AC 1 SIN(0 {vpk} 1k)
E1 p1 0 in inm {a0}
Rp p1 p2 {rp}
Cp p2 0 {cp}
E2 op 0 p2 0 1
Ro op out {ro}
Rf out inm {rf}
Rg inm 0 {rg}
Rl out 0 {rl}
.ac dec 40 10 100meg
.tran 5u 3m
.end
";

const RLC: &str = "Series RLC resonance - ringing and the resonance peak
.param r=10
.param l=1m
.param c=100n
V1 in 0 DC 0 AC 1 PULSE(0 1 20u 100n 100n 1 2)
R1 in a {r}
L1 a b {l}
C1 b 0 {c}
.ac dec 100 1k 100k
.tran 0.5u 1m
.end
";

const RECT: &str = "Half-wave rectifier with a reservoir capacitor
* Small-signal silicon diode model (typical published 1N4148-class values).
.param vpk=5
.param f=1k
.param c=10u
.param rl=1k
V1 in 0 SIN(0 {vpk} {f})
D1 in out DSIG
C1 out 0 {c}
Rl out 0 {rl}
.model DSIG D(IS=2.52n N=1.752 RS=0.568 CJO=4p M=0.4 TT=20n)
.tran 2u 5m
.end
";

fn rail_layout() -> Layout {
    let mut b = LB::new();
    b.part("Vreg", "vsource", 2.0, 10.0, "down")
        .part("Rreg", "resistor", 6.0, 4.0, "right")
        .part("Rdamp", "resistor", 12.0, 1.5, "right")
        .part("Lloop", "inductor", 12.0, 4.0, "right")
        .part("Cout", "capacitor", 18.0, 7.0, "down")
        .part("Resr", "resistor", 18.0, 11.0, "down")
        .part("Lesl", "inductor", 18.0, 15.0, "down")
        .part("Rbrd", "resistor", 22.0, 4.0, "right")
        .part("Lbrd", "inductor", 26.0, 4.0, "right")
        .part("Cdec", "capacitor", 31.0, 8.0, "down")
        .part("Iload", "isource", 36.0, 8.0, "down");
    b.wire("vset", &[(2.0, 8.0), (2.0, 4.0), (4.0, 4.0)], &[("Rreg", 1.0)])
        .wire("0", &[(2.0, 12.0), (2.0, 18.0)], &[("Vreg", 1.0)])
        .wire("n1", &[(8.0, 4.0), (9.0, 4.0)], &[("Rreg", 1.0)])
        .wire("n1", &[(9.0, 4.0), (10.0, 4.0)], &[("Lloop", 1.0)])
        .wire("n1", &[(9.0, 4.0), (9.0, 1.5), (10.0, 1.5)], &[("Rdamp", 1.0)])
        .wire("vbuck", &[(14.0, 1.5), (15.0, 1.5), (15.0, 4.0)], &[("Rdamp", 1.0)])
        .wire("vbuck", &[(14.0, 4.0), (15.0, 4.0)], &[("Lloop", 1.0)])
        .wire("vbuck", &[(15.0, 4.0), (18.0, 4.0)], &[("Lloop", 1.0), ("Rdamp", 1.0)])
        .wire("vbuck", &[(18.0, 4.0), (18.0, 5.0)], &[("Cout", 1.0)])
        .wire("vbuck", &[(18.0, 4.0), (20.0, 4.0)], &[("Rbrd", 1.0)])
        .wire("0", &[(18.0, 17.0), (18.0, 18.0)], &[("Lesl", 1.0)])
        .wire("vload", &[(28.0, 4.0), (31.0, 4.0)], &[("Lbrd", 1.0)])
        .wire("vload", &[(31.0, 4.0), (31.0, 6.0)], &[("Cdec", 1.0)])
        .wire("vload", &[(31.0, 4.0), (36.0, 4.0), (36.0, 6.0)], &[("Iload", 1.0)])
        .wire("0", &[(31.0, 10.0), (31.0, 18.0)], &[("Cdec", 1.0)])
        .wire("0", &[(36.0, 10.0), (36.0, 18.0)], &[("Iload", 1.0)]);
    b.gnd(2.0, 18.0).gnd(18.0, 18.0).gnd(31.0, 18.0).gnd(36.0, 18.0);
    b.boxed(0.4, 0.0, 15.8, 13.6, "Buck regulator · closed-loop Zout model")
        .boxed(16.4, 4.6, 3.2, 12.8, "C2")
        .boxed(19.6, 2.2, 9.2, 3.6, "PCB trace")
        .boxed(29.6, 2.2, 12.4, 9.6, "SoC + display");
    b.label("VBUCK", 15.3, 3.4, Some("vbuck")).label("VLOAD", 31.4, 3.4, Some("vload"));
    b.done()
}

fn buck_layout() -> Layout {
    let mut b = LB::new();
    b.part("Vin", "battery", 1.0, 9.0, "down")
        .part("Dhs", "diode", 6.0, 5.0, "up")
        .lpos("l")
        .part("S1", "switch", 9.0, 5.0, "down")
        .part("Dls", "diode", 6.0, 11.0, "up")
        .lpos("l")
        .part("S2", "switch", 9.0, 11.0, "down")
        .part("L1", "inductor", 15.0, 7.0, "right")
        .lpos("b")
        .part("Rdcr", "resistor", 19.0, 7.0, "right")
        .part("C2", "capacitor", 26.0, 10.0, "down")
        .part("Resr", "resistor", 26.0, 14.0, "down")
        .part("Rload", "resistor", 31.0, 11.0, "down")
        .part("Vg1", "vsource", 15.0, 13.0, "down")
        .part("Vg2", "vsource", 20.5, 13.0, "down");
    b.wire("vin", &[(1.0, 7.0), (1.0, 3.0), (6.0, 3.0)], &[("Vin", -1.0)])
        .wire("vin", &[(6.0, 3.0), (9.0, 3.0)], &[("S1", 1.0)])
        .wire("sw", &[(6.0, 7.0), (9.0, 7.0)], &[("Dhs", -1.0)])
        .wire("sw", &[(9.0, 7.0), (9.0, 9.0)], &[("S2", 1.0), ("Dls", -1.0)])
        .wire("sw", &[(6.0, 9.0), (9.0, 9.0)], &[("Dls", 1.0)])
        .wire("sw", &[(9.0, 7.0), (13.0, 7.0)], &[("L1", 1.0)])
        .wire("0", &[(1.0, 11.0), (1.0, 17.0)], &[("Vin", 1.0)])
        .wire("0", &[(6.0, 13.0), (6.0, 17.0)], &[("Dls", -1.0)])
        .wire("0", &[(9.0, 13.0), (9.0, 17.0)], &[("S2", 1.0)])
        .wire("out", &[(21.0, 7.0), (26.0, 7.0)], &[("Rdcr", 1.0)])
        .wire("out", &[(26.0, 7.0), (26.0, 8.0)], &[("C2", 1.0)])
        .wire("out", &[(26.0, 7.0), (31.0, 7.0), (31.0, 9.0)], &[("Rload", 1.0)])
        .wire("0", &[(26.0, 16.0), (26.0, 17.0)], &[("Resr", 1.0)])
        .wire("0", &[(31.0, 13.0), (31.0, 17.0)], &[("Rload", 1.0)])
        .wire("0", &[(15.0, 15.0), (15.0, 17.0)], &[("Vg1", 1.0)])
        .wire("0", &[(20.5, 15.0), (20.5, 17.0)], &[("Vg2", 1.0)]);
    b.gnd(1.0, 17.0)
        .gnd(6.0, 17.0)
        .gnd(9.0, 17.0)
        .gnd(15.0, 17.0)
        .gnd(20.5, 17.0)
        .gnd(26.0, 17.0)
        .gnd(31.0, 17.0);
    b.port("g1 → S1", 15.0, 11.0, "g1").port("g2 → S2", 20.5, 11.0, "g2");
    b.boxed(13.2, 9.9, 11.6, 5.9, "gate drive + dead time")
        .boxed(2.6, 1.6, 8.6, 13.0, "power stage");
    b.label("VIN", 1.6, 2.4, Some("vin"))
        .label("SW", 10.0, 6.4, Some("sw"))
        .label("VOUT", 26.6, 6.4, Some("out"));
    b.done()
}

fn vbat_layout() -> Layout {
    let mut b = LB::new();
    b.part("Vbat", "battery", 2.0, 8.0, "down")
        .part("R1", "resistor", 6.0, 5.0, "down")
        .part("R2", "resistor", 6.0, 11.0, "down")
        .part("C6", "capacitor", 10.0, 11.0, "down")
        .part("S1", "switch", 15.0, 8.0, "right")
        .part("Chold", "capacitor", 20.0, 11.0, "down")
        .part("S2", "switch", 25.0, 11.0, "down")
        .part("Vsamp", "vsource", 15.0, 12.0, "down")
        .part("Vrst", "vsource", 30.5, 12.0, "down");
    b.wire("bat", &[(2.0, 6.0), (2.0, 2.0), (6.0, 2.0), (6.0, 3.0)], &[("Vbat", -1.0)])
        .wire("adc", &[(6.0, 7.0), (6.0, 8.0)], &[("R1", 1.0)])
        .wire("adc", &[(6.0, 8.0), (6.0, 9.0)], &[("R2", 1.0)])
        .wire("adc", &[(6.0, 8.0), (10.0, 8.0)], &[("C6", 1.0), ("S1", 1.0)])
        .wire("adc", &[(10.0, 8.0), (10.0, 9.0)], &[("C6", 1.0)])
        .wire("adc", &[(10.0, 8.0), (13.0, 8.0)], &[("S1", 1.0)])
        .wire("hold", &[(17.0, 8.0), (20.0, 8.0)], &[("S1", 1.0)])
        .wire("hold", &[(20.0, 8.0), (20.0, 9.0)], &[("Chold", 1.0)])
        .wire("hold", &[(20.0, 8.0), (25.0, 8.0), (25.0, 9.0)], &[("S2", 1.0)])
        .wire("0", &[(2.0, 10.0), (2.0, 15.0)], &[("Vbat", 1.0)])
        .wire("0", &[(6.0, 13.0), (6.0, 15.0)], &[("R2", 1.0)])
        .wire("0", &[(10.0, 13.0), (10.0, 15.0)], &[("C6", 1.0)])
        .wire("0", &[(20.0, 13.0), (20.0, 15.0)], &[("Chold", 1.0)])
        .wire("0", &[(25.0, 13.0), (25.0, 15.0)], &[("S2", 1.0)])
        .wire("0", &[(15.0, 14.0), (15.0, 15.0)], &[("Vsamp", 1.0)])
        .wire("0", &[(30.5, 14.0), (30.5, 15.0)], &[("Vrst", 1.0)]);
    for x in [2.0, 6.0, 10.0, 15.0, 20.0, 25.0, 30.5] {
        b.gnd(x, 15.0);
    }
    b.port("sample", 15.0, 10.0, "samp").port("reset", 30.5, 10.0, "rst");
    b.boxed(12.0, 5.4, 23.4, 11.8, "SoC ADC · sample-and-hold");
    b.label("VBAT", 2.6, 1.4, Some("bat"))
        .label("ADC pin", 6.6, 7.4, Some("adc"))
        .label("hold", 20.6, 7.4, Some("hold"));
    b.done()
}

fn irled_layout() -> Layout {
    let mut b = LB::new();
    b.part("Vdd", "vsource", 19.0, 9.0, "down")
        .part("Rled", "resistor", 13.0, 5.0, "down")
        .part("D1", "led", 13.0, 9.0, "down")
        .lpos("l")
        .part("M1", "nmos", 13.0, 13.0, "right")
        .part("Rg", "resistor", 7.0, 13.0, "right")
        .part("Ciss", "capacitor", 11.0, 15.0, "down")
        .lpos("l")
        .part("Vpwm", "vsource", 2.0, 16.0, "down");
    b.wire("vdd", &[(19.0, 7.0), (19.0, 2.0), (13.0, 2.0), (13.0, 3.0)], &[("Vdd", -1.0)])
        .wire("0", &[(19.0, 11.0), (19.0, 18.0)], &[("Vdd", 1.0)])
        .wire("0", &[(13.0, 15.0), (13.0, 18.0)], &[("M1", 1.0)])
        .wire("0", &[(11.0, 17.0), (11.0, 18.0)], &[("Ciss", 1.0)])
        .wire("g", &[(9.0, 13.0), (11.0, 13.0)], &[("Rg", 1.0)])
        .wire("gp", &[(2.0, 14.0), (2.0, 13.0), (5.0, 13.0)], &[("Vpwm", -1.0)]);
    b.gnd(19.0, 18.0).gnd(13.0, 18.0).gnd(11.0, 18.0).gnd(2.0, 18.0);
    b.label("3.3 V rail", 19.6, 6.4, Some("vdd"))
        .label("GPIO", 0.4, 12.4, Some("gp"))
        .label("drain", 13.6, 10.9, Some("d"));
    b.done()
}

fn touch_layout() -> Layout {
    let mut b = LB::new();
    b.part("Vdrv", "vsource", 2.0, 9.0, "down")
        .part("Rn", "resistor", 11.0, 3.0, "right")
        .part("Cn", "capacitor", 15.0, 6.0, "down")
        .part("Rf", "resistor", 11.0, 10.0, "right")
        .part("Cp", "capacitor", 15.0, 13.0, "down")
        .part("Cfin", "capacitor", 23.0, 10.0, "right")
        .lpos("b")
        .part("Cbody", "capacitor", 26.0, 13.0, "down")
        .part("Rleak", "resistor", 30.0, 13.0, "down");
    b.wire("drv", &[(2.0, 7.0), (2.0, 3.0), (7.5, 3.0)], &[("Vdrv", -1.0)])
        .wire("drv", &[(7.5, 3.0), (9.0, 3.0)], &[("Rn", 1.0)])
        .wire("drv", &[(7.5, 3.0), (7.5, 10.0), (9.0, 10.0)], &[("Rf", 1.0)])
        .wire("pad_nf", &[(13.0, 3.0), (15.0, 3.0), (15.0, 4.0)], &[("Rn", 1.0)])
        .wire("pad_f", &[(13.0, 10.0), (15.0, 10.0)], &[("Rf", 1.0)])
        .wire("pad_f", &[(15.0, 10.0), (15.0, 11.0)], &[("Cp", 1.0)])
        .wire("pad_f", &[(15.0, 10.0), (21.0, 10.0)], &[("Cfin", 1.0)])
        .wire("body", &[(25.0, 10.0), (26.0, 10.0)], &[("Cfin", 1.0)])
        .wire("body", &[(26.0, 10.0), (26.0, 11.0)], &[("Cbody", 1.0)])
        .wire("body", &[(26.0, 10.0), (30.0, 10.0), (30.0, 11.0)], &[("Rleak", 1.0)])
        .wire("0", &[(2.0, 11.0), (2.0, 13.0)], &[("Vdrv", 1.0)]);
    b.gnd(2.0, 13.0).gnd(15.0, 8.0).gnd(15.0, 15.0).gnd(26.0, 15.0).gnd(30.0, 15.0);
    b.boxed(19.8, 7.4, 12.2, 9.6, "finger + human body");
    b.label("pad · no touch", 16.0, 2.4, Some("pad_nf"))
        .label("pad · finger", 15.6, 9.4, Some("pad_f"))
        .label("GPIO", 2.6, 2.4, Some("drv"));
    b.done()
}

fn cmos_layout() -> Layout {
    let mut b = LB::new();
    b.part("Vdd", "vsource", 22.0, 6.0, "down")
        .part("Mp", "pmos", 10.0, 4.0, "right")
        .part("Mn", "nmos", 10.0, 10.0, "right")
        .part("Cl", "capacitor", 18.0, 10.0, "down")
        .part("Vin", "vsource", 1.0, 10.0, "down");
    b.wire("vdd", &[(22.0, 4.0), (22.0, 1.0), (10.0, 1.0), (10.0, 2.0)], &[("Vdd", -1.0)])
        .wire("out", &[(10.0, 6.0), (10.0, 7.0)], &[("Mp", -1.0)])
        .wire("out", &[(10.0, 7.0), (10.0, 8.0)], &[("Mn", 1.0)])
        .wire("out", &[(10.0, 7.0), (18.0, 7.0), (18.0, 8.0)], &[("Cl", 1.0)])
        .wire("in", &[(1.0, 8.0), (1.0, 7.0), (6.0, 7.0)], &[("Vin", -1.0)])
        .wire("in", &[(6.0, 7.0), (6.0, 4.0), (8.0, 4.0)], &[])
        .wire("in", &[(6.0, 7.0), (6.0, 10.0), (8.0, 10.0)], &[])
        .wire("0", &[(10.0, 12.0), (10.0, 14.0)], &[("Mn", 1.0)])
        .wire("0", &[(18.0, 12.0), (18.0, 14.0)], &[("Cl", 1.0)])
        .wire("0", &[(1.0, 12.0), (1.0, 14.0)], &[("Vin", 1.0)])
        .wire("0", &[(22.0, 8.0), (22.0, 14.0)], &[("Vdd", 1.0)]);
    b.gnd(10.0, 14.0).gnd(18.0, 14.0).gnd(1.0, 14.0).gnd(22.0, 14.0);
    b.label("VDD", 22.6, 0.8, Some("vdd"))
        .label("IN", 1.6, 6.4, Some("in"))
        .label("OUT", 13.0, 6.4, Some("out"));
    b.done()
}

fn rcaa_layout() -> Layout {
    let mut b = LB::new();
    b.part("Vsig", "vsource", 2.0, 7.0, "down")
        .part("Vint", "vsource", 2.0, 11.0, "down")
        .part("R1", "resistor", 6.0, 3.0, "right")
        .part("C1", "capacitor", 11.0, 7.0, "down")
        .part("R2", "resistor", 15.0, 3.0, "right")
        .part("C2", "capacitor", 19.0, 7.0, "down");
    b.wire("in", &[(2.0, 5.0), (2.0, 3.0), (4.0, 3.0)], &[("Vsig", -1.0)])
        .wire("n1", &[(8.0, 3.0), (11.0, 3.0)], &[("R1", 1.0)])
        .wire("n1", &[(11.0, 3.0), (11.0, 5.0)], &[("C1", 1.0)])
        .wire("n1", &[(11.0, 3.0), (13.0, 3.0)], &[("R2", 1.0)])
        .wire("out", &[(17.0, 3.0), (19.0, 3.0)], &[("R2", 1.0)])
        .wire("out", &[(19.0, 3.0), (19.0, 5.0)], &[("C2", 1.0)])
        .wire("out", &[(19.0, 3.0), (23.0, 3.0)], &[])
        .wire("0", &[(11.0, 9.0), (11.0, 13.0)], &[("C1", 1.0)])
        .wire("0", &[(19.0, 9.0), (19.0, 13.0)], &[("C2", 1.0)]);
    b.gnd(2.0, 13.0).gnd(11.0, 13.0).gnd(19.0, 13.0);
    b.port("to ADC (16 kS/s)", 23.0, 3.0, "out");
    b.done()
}

fn opamp_layout() -> Layout {
    let mut b = LB::new();
    b.macro_part("E1", "opamp", 14.0, 6.0, &["E1", "Rp", "Cp", "E2", "Ro"], &["in", "inm", "out"])
        .part("Vin", "vsource", 3.0, 9.0, "down")
        .part("Rf", "resistor", 14.0, 10.0, "left")
        .part("Rg", "resistor", 10.5, 13.0, "down")
        .part("Rl", "resistor", 22.0, 9.0, "down");
    b.wire("in", &[(3.0, 7.0), (3.0, 5.0), (11.0, 5.0)], &[("Vin", -1.0)])
        .wire("inm", &[(11.0, 7.0), (10.5, 7.0), (10.5, 10.0)], &[])
        .wire("inm", &[(10.5, 10.0), (12.0, 10.0)], &[("Rf", -1.0)])
        .wire("inm", &[(10.5, 10.0), (10.5, 11.0)], &[("Rg", 1.0)])
        .wire("out", &[(17.0, 6.0), (19.0, 6.0)], &[("Ro", 1.0)])
        .wire("out", &[(19.0, 6.0), (19.0, 10.0), (16.0, 10.0)], &[("Rf", 1.0)])
        .wire("out", &[(19.0, 6.0), (22.0, 6.0), (22.0, 7.0)], &[("Rl", 1.0)])
        .wire("0", &[(3.0, 11.0), (3.0, 16.0)], &[("Vin", 1.0)])
        .wire("0", &[(10.5, 15.0), (10.5, 16.0)], &[("Rg", 1.0)])
        .wire("0", &[(22.0, 11.0), (22.0, 16.0)], &[("Rl", 1.0)]);
    b.gnd(3.0, 16.0).gnd(10.5, 16.0).gnd(22.0, 16.0);
    b.label("VOUT", 22.6, 5.4, Some("out")).label("in", 3.6, 4.4, Some("in"));
    b.done()
}

fn rlc_layout() -> Layout {
    let mut b = LB::new();
    b.part("V1", "vsource", 2.0, 7.0, "down")
        .part("R1", "resistor", 6.0, 3.0, "right")
        .part("L1", "inductor", 10.0, 3.0, "right")
        .part("C1", "capacitor", 14.0, 7.0, "down");
    b.wire("in", &[(2.0, 5.0), (2.0, 3.0), (4.0, 3.0)], &[("V1", -1.0)])
        .wire("b", &[(12.0, 3.0), (14.0, 3.0), (14.0, 5.0)], &[("L1", 1.0)])
        .wire("0", &[(2.0, 9.0), (2.0, 11.0)], &[("V1", 1.0)])
        .wire("0", &[(14.0, 9.0), (14.0, 11.0)], &[("C1", 1.0)]);
    b.gnd(2.0, 11.0).gnd(14.0, 11.0);
    b.label("VC", 14.6, 2.4, Some("b"));
    b.done()
}

fn rect_layout() -> Layout {
    let mut b = LB::new();
    b.part("V1", "vsource", 2.0, 7.0, "down")
        .part("D1", "diode", 6.0, 3.0, "right")
        .part("C1", "capacitor", 10.0, 6.0, "down")
        .part("Rl", "resistor", 14.0, 6.0, "down");
    b.wire("in", &[(2.0, 5.0), (2.0, 3.0), (4.0, 3.0)], &[("V1", -1.0)])
        .wire("out", &[(8.0, 3.0), (10.0, 3.0)], &[("D1", 1.0)])
        .wire("out", &[(10.0, 3.0), (10.0, 4.0)], &[("C1", 1.0)])
        .wire("out", &[(10.0, 3.0), (14.0, 3.0), (14.0, 4.0)], &[("Rl", 1.0)])
        .wire("0", &[(2.0, 9.0), (2.0, 10.0)], &[("V1", 1.0)])
        .wire("0", &[(10.0, 8.0), (10.0, 10.0)], &[("C1", 1.0)])
        .wire("0", &[(14.0, 8.0), (14.0, 10.0)], &[("Rl", 1.0)]);
    b.gnd(2.0, 10.0).gnd(10.0, 10.0).gnd(14.0, 10.0);
    b.label("VOUT", 14.6, 2.4, Some("out"));
    b.done()
}

/// All presets with their default parameters.
fn base_presets() -> Vec<Preset> {
    vec![
        Preset {
            id: "rail_load_step".into(),
            title: "1.8 V rail · display load step".into(),
            group: "Iris".into(),
            summary: "Does the core rail survive the micro-OLED switching on?".into(),
            description: "The display's turn-on pulls the 1.8 V rail from 1 mA to 60 mA in 1 µs. The regulator cannot react instantly: until its control loop catches up (≈ 1/fc), the output capacitor C2 supplies the charge, its ESR and ESL add an instant step, and the PCB trace adds IR drop. The droop at the load is what the BLE SoC's brown-out detector sees.".into(),
            iris: "Verifies R-012: the 1.8 V rail shall droop by at most 50 mV on a display turn-on load step (protects the BLE SoC brown-out threshold). C2 comes from the Iris BOM; vnom from design.power_rails.core_v.".into(),
            netlist: RAIL.into(),
            params: vec![
                ps("istep", "Load after step", 60e-3, 5e-3, 300e-3, "A", true),
                ps("tedge", "Step edge time", 1e-6, 10e-9, 10e-6, "s", true),
                ps("cout", "Output capacitor C2", 10e-6, 1e-6, 100e-6, "F", true),
                ps("esr", "C2 ESR", 5e-3, 1e-3, 200e-3, "Ω", true),
                ps("fc", "Loop crossover fc", 100e3, 10e3, 500e3, "Hz", true),
                ps("q", "Loop damping Q", 1.0, 0.3, 5.0, "", true),
                ps("lbrd", "PCB trace inductance", 5e-9, 0.5e-9, 50e-9, "H", true),
                ps("rbrd", "PCB trace resistance", 30e-3, 1e-3, 300e-3, "Ω", true),
            ],
            probes: vec!["v(vload)".into(), "v(vbuck)".into()],
            layout: rail_layout(),
            links: vec![link("power", "Power budget & battery life"), link("pcb", "PCB trace widths")],
            metric: Some("circuit.rail_droop_mv".into()),
        },
        Preset {
            id: "buck_converter".into(),
            title: "3.7 V → 1.8 V synchronous buck".into(),
            group: "Iris".into(),
            summary: "Switching ripple of the 2 MHz buck that makes the core rail.".into(),
            description: "Two switches chop the battery voltage into a 2 MHz square wave at the SW node; the LC filter (L1 2.2 µH, C2 10 µF) averages it to D·Vin. The inductor current ramps up and down by ΔI = (Vin − Vout)·D/(L·fsw); that triangle flows into C2 and its ESR and appears as output ripple ≈ ΔI/(8·fsw·C) + ΔI·ESR. Dead time between the switches forces the inductor current through a body diode — look for the short negative spikes at SW.".into(),
            iris: "Verifies R-013: buck output ripple ≤ 30 mV peak-to-peak (keeps the radio and IMU noise floor clean). L1 and C2 come from the Iris BOM; Vin = battery nominal voltage, Vout = core rail.".into(),
            netlist: BUCK.into(),
            params: vec![
                ps("fsw", "Switching frequency", 2e6, 250e3, 5e6, "Hz", true),
                ps("l", "Inductor L1", 2.2e-6, 0.47e-6, 22e-6, "H", true),
                ps("cout", "Output capacitor C2", 10e-6, 1e-6, 100e-6, "F", true),
                ps("esr", "C2 ESR", 5e-3, 1e-3, 300e-3, "Ω", true),
                ps("iout", "Load current", 150e-3, 20e-3, 600e-3, "A", true),
                ps("vin", "Battery voltage", 3.7, 3.3, 4.2, "V", false),
                ps("tdead", "Dead time", 5e-9, 1e-9, 40e-9, "s", true),
                ps("rsw", "Switch on-resistance", 50e-3, 5e-3, 500e-3, "Ω", true),
            ],
            probes: vec!["v(out)".into(), "i(L1)".into(), "v(sw)".into()],
            layout: buck_layout(),
            links: vec![link("power", "Converter efficiency in the energy budget"), link("control", "Close the loop: compensator design")],
            metric: Some("circuit.buck_ripple_mv".into()),
        },
        Preset {
            id: "vbat_divider".into(),
            title: "Battery-sense divider → ADC".into(),
            group: "Iris".into(),
            summary: "An always-on divider must sip microamps and still feed a sampling ADC.".into(),
            description: "R1/R2 scale the cell voltage for the SoC's ADC. Big resistors keep the always-on drain tiny, but then the ADC's sample-and-hold capacitor cannot charge through them in a 10 µs acquisition window — so C6 acts as a local charge reservoir. The price is a slow RC settling time (τ = (R1‖R2)·C6) after power-up. Each sample steals a little charge (kick-back ΔV ≈ V·Chold/C6).".into(),
            iris: "Verifies R-014: divider current ≤ 2 µA at a full 4.2 V cell. Values from the Iris BOM (R1, R2, C6) and design.battery.full_v. Also checks the ADC pin stays below the 1.8 V rail.".into(),
            netlist: VBAT.into(),
            params: vec![
                ps("rtop", "R1 (top)", 1e6, 100e3, 10e6, "Ω", true),
                ps("rbot", "R2 (bottom)", 1.2e6, 100e3, 10e6, "Ω", true),
                ps("cf", "Reservoir C6", 100e-9, 100e-12, 1e-6, "F", true),
                ps("vbat", "Cell voltage", 4.2, 3.0, 4.35, "V", false),
                ps("chold", "ADC hold capacitor", 5e-12, 1e-12, 50e-12, "F", true),
                ps("tacq", "ADC acquisition time", 10e-6, 1e-6, 100e-6, "s", true),
            ],
            probes: vec!["v(adc)".into(), "v(hold)".into()],
            layout: vbat_layout(),
            links: vec![link("power", "Battery life & quiescent budget"), link("dsp", "ADC sampling")],
            metric: Some("circuit.vbat_divider_ua".into()),
        },
        Preset {
            id: "ir_led_driver".into(),
            title: "IR LED wear-detection driver".into(),
            group: "Iris".into(),
            summary: "A 1.8 V GPIO pulses an IR LED through a logic-level NMOS.".into(),
            description: "Iris knows it is being worn by pulsing an infrared LED and looking for the reflection off the skin. The SoC's GPIO swings only 1.8 V, so the NMOS must turn fully on with Vgs = 1.8 V (a 'logic-level' part); its on-resistance, the LED's forward voltage and Rled set the pulse current. Rg and the MOSFET's input capacitance form an RC that slows the edges.".into(),
            iris: "Wear detection keeps the display off when the glasses are folded (power). Pulsed at low duty so the average current is tiny.".into(),
            netlist: IRLED.into(),
            params: vec![
                ps("rled", "Series resistor Rled", 33.0, 4.7, 330.0, "Ω", true),
                ps("duty", "PWM duty", 0.1, 0.01, 0.9, "", true),
                ps("fpwm", "PWM frequency", 1e3, 200.0, 5e3, "Hz", true),
                ps("vgate", "GPIO high level", 1.8, 1.0, 3.3, "V", false),
                ps("rg", "Gate resistor", 100.0, 1.0, 10e3, "Ω", true),
                ps("ciss", "MOSFET Ciss", 1e-9, 50e-12, 5e-9, "F", true),
            ],
            probes: vec!["i(D1)".into(), "v(d)".into(), "v(g)".into()],
            layout: irled_layout(),
            links: vec![link("power", "Average current in the energy budget")],
            metric: None,
        },
        Preset {
            id: "touch_rc".into(),
            title: "Capacitive touch · RC timing".into(),
            group: "Iris".into(),
            summary: "A finger adds ~10 pF — measured as a longer charge time.".into(),
            description: "The touch strip is a copper pad. Charging it through a resistor takes τ = R·C; a finger couples extra capacitance (Cfin in series with the body's capacitance to earth), so the pad crosses the GPIO's input threshold later. A timer counts that delay: the difference between the two traces is the touch signal.".into(),
            iris: "The temple touch strip (tap to wake, swipe to dismiss) uses this self-capacitance principle (U7 in the BOM).".into(),
            netlist: TOUCH.into(),
            params: vec![
                ps("r", "Charge resistor R", 1e6, 50e3, 10e6, "Ω", true),
                ps("cpad", "Pad capacitance", 12e-12, 2e-12, 50e-12, "F", true),
                ps("cfinger", "Finger coupling", 10e-12, 0.5e-12, 30e-12, "F", true),
                ps("cbody", "Body to earth", 150e-12, 50e-12, 400e-12, "F", true),
            ],
            probes: vec!["v(pad_nf)".into(), "v(pad_f)".into(), "v(drv)".into()],
            layout: touch_layout(),
            links: vec![link("dsp", "Filtering the touch counts"), link("chip", "Timer/counter logic")],
            metric: None,
        },
        Preset {
            id: "cmos_inverter".into(),
            title: "CMOS inverter".into(),
            group: "Classic".into(),
            summary: "Voltage transfer curve, noise margins and propagation delay.".into(),
            description: "A PMOS pull-up and NMOS pull-down: when one conducts the other is off, so the gate burns power only while switching (≈ C·V² per cycle). The DC sweep traces the transfer characteristic; the switching threshold VM sits where both transistors are saturated and the gain peaks. The transient shows how the load capacitance sets the delay.".into(),
            iris: "Every gate in the IRIS-DSP co-processor (Chip Lab) is built from this cell; its delay feeds the static timing estimate.".into(),
            netlist: CMOS.into(),
            params: vec![
                ps("wp", "PMOS width Wp", 1.08e-6, 0.22e-6, 4e-6, "m", true),
                ps("wn", "NMOS width Wn", 0.36e-6, 0.22e-6, 4e-6, "m", true),
                ps("cl", "Load capacitance", 20e-15, 1e-15, 500e-15, "F", true),
                ps("vdd", "Supply", 1.8, 0.9, 3.3, "V", false),
            ],
            probes: vec!["v(out)".into(), "v(in)".into()],
            layout: cmos_layout(),
            links: vec![link("chip", "Gate-level design & timing")],
            metric: None,
        },
        Preset {
            id: "rc_antialias".into(),
            title: "RC anti-alias filter".into(),
            group: "Classic".into(),
            summary: "AC Bode plot of a 2nd-order RC low-pass before an ADC.".into(),
            description: "Anything above half the sample rate folds back into the band after sampling. A passive two-pole RC rolls off at 40 dB/decade — gentle, which is why real systems oversample and finish the job with a digital filter. The AC analysis linearises the circuit and sweeps frequency; the transient shows the 20 kHz interferer being attenuated.".into(),
            iris: "The Iris microphones are digital (PDM) and filtered in the DSP Lab; an analog sensor channel sampled at 16 kS/s would need this front end.".into(),
            netlist: RCAA.into(),
            params: vec![
                ps("r1", "R1", 1e3, 100.0, 100e3, "Ω", true),
                ps("c1", "C1", 15e-9, 100e-12, 1e-6, "F", true),
                ps("r2", "R2", 10e3, 100.0, 1e6, "Ω", true),
                ps("c2", "C2", 1.5e-9, 10e-12, 100e-9, "F", true),
                ps("fint", "Interferer frequency", 20e3, 5e3, 200e3, "Hz", true),
            ],
            probes: vec!["v(out)".into(), "v(n1)".into()],
            layout: rcaa_layout(),
            links: vec![link("dsp", "Sampling, aliasing and decimation filters")],
            metric: None,
        },
        Preset {
            id: "opamp_preamp".into(),
            title: "Op-amp non-inverting preamp".into(),
            group: "Classic".into(),
            summary: "Finite gain and a dominant pole: the gain–bandwidth trade-off.".into(),
            description: "The op-amp is a macro-model: a VCVS with open-loop gain A0, an RC setting the dominant pole at GBW/A0, and a unity buffer with output resistance. Negative feedback trades gain for bandwidth: closed-loop gain ≈ 1 + Rf/Rg and bandwidth ≈ GBW / (1 + Rf/Rg).".into(),
            iris: "An analog MEMS microphone variant of Iris would need a preamp like this before its ADC.".into(),
            netlist: OPAMP.into(),
            params: vec![
                ps("rf", "Feedback Rf", 19e3, 0.0 + 100.0, 1e6, "Ω", true),
                ps("rg", "Ground leg Rg", 1e3, 100.0, 100e3, "Ω", true),
                ps("a0", "Open-loop gain A0", 100e3, 100.0, 10e6, "V/V", true),
                ps("gbw", "Gain-bandwidth", 1e6, 10e3, 100e6, "Hz", true),
                ps("vpk", "Input amplitude", 10e-3, 1e-3, 100e-3, "V", true),
            ],
            probes: vec!["v(out)".into(), "v(in)".into()],
            layout: opamp_layout(),
            links: vec![link("control", "Feedback, loop gain and Bode plots"), link("dsp", "Audio front end")],
            metric: None,
        },
        Preset {
            id: "rlc_resonance".into(),
            title: "Series RLC resonance".into(),
            group: "Classic".into(),
            summary: "Ringing in time, a peak in frequency — the same Q.".into(),
            description: "Energy sloshes between L and C at f0 = 1/(2π√(LC)); R dissipates it. The quality factor Q = (1/R)·√(L/C) sets both how long the step response rings (≈ Q cycles) and how sharp the resonance peak is (bandwidth = f0/Q).".into(),
            iris: "The same physics sets the buck's LC filter and the rail's loop/ESR damping on the Iris board.".into(),
            netlist: RLC.into(),
            params: vec![ps("r", "Resistance R", 10.0, 0.5, 1e3, "Ω", true), ps("l", "Inductance L", 1e-3, 10e-6, 100e-3, "H", true), ps("c", "Capacitance C", 100e-9, 1e-9, 10e-6, "F", true)],
            probes: vec!["v(b)".into(), "i(L1)".into()],
            layout: rlc_layout(),
            links: vec![link("control", "Second-order systems")],
            metric: None,
        },
        Preset {
            id: "half_wave_rectifier".into(),
            title: "Half-wave rectifier".into(),
            group: "Classic".into(),
            summary: "A diode and a reservoir capacitor: ripple ≈ I/(f·C).".into(),
            description: "The diode conducts only near the positive peaks, recharging C in short, large current pulses; between peaks the load drains C. Newton–Raphson with junction limiting handles the exponential diode.".into(),
            iris: "Classic nonlinear example — the same diode model is used for the IR LED and the buck's body diodes.".into(),
            netlist: RECT.into(),
            params: vec![ps("vpk", "Input amplitude", 5.0, 1.0, 20.0, "V", false), ps("c", "Reservoir C", 10e-6, 100e-9, 470e-6, "F", true), ps("rl", "Load R", 1e3, 50.0, 100e3, "Ω", true), ps("f", "Frequency", 1e3, 50.0, 20e3, "Hz", true)],
            probes: vec!["v(out)".into(), "v(in)".into(), "i(D1)".into()],
            layout: rect_layout(),
            links: vec![],
            metric: None,
        },
    ]
}

// ---------------------------------------------------------------------------
// Project coupling
// ---------------------------------------------------------------------------

/// Parse a human-readable component value from a BOM line name, e.g.
/// "Buck inductor 2.2 uH" → 2.2e-6, "VBAT divider top 1 M" → 1e6.
pub fn human_value(name: &str) -> Option<f64> {
    let toks: Vec<&str> = name.split_whitespace().collect();
    for i in (0..toks.len()).rev() {
        let t = toks[i];
        let numlen = t
            .char_indices()
            .take_while(|(_, c)| c.is_ascii_digit() || *c == '.')
            .map(|(k, c)| k + c.len_utf8())
            .last()
            .unwrap_or(0);
        if numlen == 0 {
            continue;
        }
        let Ok(v) = t[..numlen].parse::<f64>() else { continue };
        let rest = if numlen < t.len() {
            &t[numlen..]
        } else {
            toks.get(i + 1).copied().unwrap_or("")
        };
        let scale = if rest.to_lowercase().starts_with("meg") {
            1e6
        } else {
            match rest.chars().next() {
                Some('p') => 1e-12,
                Some('n') => 1e-9,
                Some('u') | Some('µ') => 1e-6,
                Some('m') => 1e-3,
                Some('k') | Some('K') => 1e3,
                Some('M') => 1e6,
                Some('G') => 1e9,
                _ => 1.0,
            }
        };
        return Some(v * scale);
    }
    None
}

fn comp_value(p: &Project, refdes: &str, lo: f64, hi: f64) -> Option<f64> {
    let c = p.components.iter().find(|c| c.refdes.eq_ignore_ascii_case(refdes))?;
    human_value(&c.name).filter(|v| *v >= lo && *v <= hi)
}

/// Default parameter values for a preset, taken from the project.
pub fn project_params(id: &str, p: &Project) -> BTreeMap<String, f64> {
    let mut m = BTreeMap::new();
    let d = &p.design;
    let mut put = |k: &str, v: Option<f64>| {
        if let Some(v) = v.filter(|v| v.is_finite() && *v > 0.0) {
            m.insert(k.to_string(), v);
        }
    };
    match id {
        "rail_load_step" => {
            put("vnom", Some(d.power_rails.core_v));
            put("cout", comp_value(p, "C2", 1e-7, 1e-3));
        }
        "buck_converter" => {
            put("vin", Some(d.battery.nominal_v));
            put("vout", Some(d.power_rails.core_v));
            put("l", comp_value(p, "L1", 1e-8, 1e-3));
            put("cout", comp_value(p, "C2", 1e-7, 1e-3));
        }
        "vbat_divider" => {
            put("vbat", Some(d.battery.full_v));
            put("rtop", comp_value(p, "R1", 1e3, 1e8));
            put("rbot", comp_value(p, "R2", 1e3, 1e8));
            put("cf", comp_value(p, "C6", 1e-12, 1e-4));
        }
        "ir_led_driver" => {
            put("vdd", Some(d.power_rails.display_rail_v));
            put("vgate", Some(d.power_rails.core_v));
        }
        "touch_rc" => put("vdd", Some(d.power_rails.core_v)),
        _ => {}
    }
    m
}

fn find_preset(id: &str) -> Option<Preset> {
    base_presets().into_iter().find(|p| p.id == id)
}

/// Preset list for the UI (defaults from the project when given).
pub fn list(project: Option<&Project>) -> Vec<Preset> {
    base_presets()
        .into_iter()
        .map(|mut pr| {
            if let Some(p) = project {
                let over = project_params(&pr.id, p);
                for s in pr.params.iter_mut() {
                    if let Some(v) = over.get(&s.name) {
                        s.value = *v;
                    }
                }
                pr.netlist = parser::with_params(&pr.netlist, &over);
            }
            if let Ok(nl) = parse(&pr.netlist, &BTreeMap::new()) {
                resolve_pins(&mut pr.layout, &nl);
            }
            pr
        })
        .collect()
}

/// Run a preset (optionally with an edited deck) and add its measurements.
pub fn run(id: &str, params: &BTreeMap<String, f64>, netlist: Option<&str>, project: Option<&Project>) -> Result<SimResult, String> {
    let pr = find_preset(id).ok_or_else(|| format!("unknown preset `{id}`"))?;
    let mut all = project.map(|p| project_params(id, p)).unwrap_or_default();
    for (k, v) in params {
        all.insert(k.to_lowercase(), *v);
    }
    let deck = netlist.unwrap_or(&pr.netlist);
    let mut res = simulate(deck, &all)?;
    measure(id, &mut res);
    Ok(res)
}

/// Run only the DC operating point of a preset (for metrics that need no
/// transient, keeping `metrics()` fast).
fn run_op_only(id: &str, p: &Project) -> Result<SimResult, String> {
    let pr = find_preset(id).ok_or_else(|| format!("unknown preset `{id}`"))?;
    let params = project_params(id, p);
    let mut nl = parse(&pr.netlist, &params).map_err(|e| super::errors_text(&e))?;
    nl.analyses = vec![parser::Analysis::Op];
    let mut res = super::run_netlist(&nl, &pr.netlist, &params)?;
    measure(id, &mut res);
    Ok(res)
}

// ---------------------------------------------------------------------------
// Measurements
// ---------------------------------------------------------------------------

fn prm(r: &SimResult, k: &str) -> f64 {
    r.params.get(k).copied().unwrap_or(f64::NAN)
}

#[derive(Default)]
struct Extra {
    ann: Vec<Annotation>,
    window: Option<[f64; 2]>,
}

fn ann(r: &mut Extra, plot: &str, axis: &str, value: f64, label: &str) {
    r.ann.push(Annotation {
        plot: plot.into(),
        axis: axis.into(),
        value,
        label: label.into(),
    });
}

fn measure(id: &str, r: &mut SimResult) {
    let mut m: Vec<Measurement> = Vec::new();
    let mut ex = Extra::default();
    let an = &mut ex;
    match id {
        "rail_load_step" => measure_rail(r, &mut m, an),
        "buck_converter" => measure_buck(r, &mut m, an),
        "vbat_divider" => measure_vbat(r, &mut m, an),
        "ir_led_driver" => measure_irled(r, &mut m, an),
        "touch_rc" => measure_touch(r, &mut m, an),
        "cmos_inverter" => measure_cmos(r, &mut m, an),
        "rc_antialias" => measure_rcaa(r, &mut m, an),
        "opamp_preamp" => measure_opamp(r, &mut m, an),
        "rlc_resonance" => measure_rlc(r, &mut m, an),
        "half_wave_rectifier" => measure_rect(r, &mut m, an),
        _ => {}
    }
    r.measurements = m;
    r.annotations = ex.ann;
    r.tran_window = ex.window;
}

fn measure_rail(r: &SimResult, m: &mut Vec<Measurement>, an: &mut Extra) {
    let vnom = prm(r, "vnom");
    let (Some(tr), Some(v), Some(vb)) = (r.tran.as_ref(), r.tv("vload"), r.tv("vbuck")) else {
        return;
    };
    let t = &tr.time;
    let tend = *t.last().unwrap_or(&0.0);
    let t_step = 10e-6;
    if let Some((mn, tmn, _, _)) = min_max(t, v, 0.0, tend) {
        let droop = (vnom - mn) * 1e3;
        m.push(
            Measurement::new("rail_droop_mv", "Max droop at load (VLOAD)", droop, "mV")
                .note(format!(
                    "minimum {} at t = {}; requirement R-012 ≤ 50 mV",
                    fmt_si(mn, "V"),
                    fmt_si(tmn, "s")
                ))
                .check(droop <= 50.0),
        );
        ann(an, "tran", "x", tmn, "worst droop");
    }
    if let Some((mn, _, _, _)) = min_max(t, vb, 0.0, tend) {
        m.push(
            Measurement::new("vbuck_droop_mv", "Droop at regulator output (VBUCK)", (vnom - mn) * 1e3, "mV").note("before the PCB trace"),
        );
    }
    let vfinal = *v.last().unwrap() as f64;
    m.push(
        Measurement::new("static_drop_mv", "Static drop after the step", (vnom - vfinal) * 1e3, "mV")
            .note("load-regulation + trace IR drop at the new load"),
    );
    if let Some(ts) = settle_time(t, v, vfinal, 1e-3, t_step) {
        m.push(Measurement::new(
            "recovery_us",
            "Recovery to within 1 mV",
            (ts - t_step) * 1e6,
            "µs",
        ));
    }
    let (fc, c, istep, ibase) = (prm(r, "fc"), prm(r, "cout"), prm(r, "istep"), prm(r, "ibase"));
    let z0 = 1.0 / (2.0 * std::f64::consts::PI * fc * c);
    m.push(
        Measurement::new("estimate_mv", "Hand estimate ΔI/(2π·fc·C)", (istep - ibase) * z0 * 1e3, "mV")
            .note(format!("|Zout| peak ≈ 1/(2π·fc·C) = {}", fmt_si(z0, "Ω"))),
    );
    ann(an, "tran", "y", vnom - 0.05, "−50 mV limit");
    ann(an, "tran", "x", t_step, "step");
    an.window = Some([t_step - 3e-6, tend]);
}

fn measure_buck(r: &SimResult, m: &mut Vec<Measurement>, an: &mut Extra) {
    let (fsw, vin, vout_set, l, c, esr, iout) = (
        prm(r, "fsw"),
        prm(r, "vin"),
        prm(r, "vout"),
        prm(r, "l"),
        prm(r, "cout"),
        prm(r, "esr"),
        prm(r, "iout"),
    );
    let tsw = 1.0 / fsw;
    let (Some(tr), Some(v), Some(il)) = (r.tran.as_ref(), r.tv("out"), r.ti("L1")) else {
        return;
    };
    let t = &tr.time;
    let tend = *t.last().unwrap_or(&0.0);
    if let Some(pp) = ripple_pp(t, v, tsw, 4, tend) {
        m.push(
            Measurement::new("buck_ripple_mv", "Output ripple (peak-to-peak)", pp * 1e3, "mV")
                .note("worst single cycle of the last four; requirement R-013 ≤ 30 mV")
                .check(pp * 1e3 <= 30.0),
        );
    }
    let t0 = tend - 4.0 * tsw;
    if let Some(avg) = average(t, v, t0, tend) {
        m.push(Measurement::new("vout_avg", "Average output", avg, "V").note(format!("target {}", fmt_si(vout_set, "V"))));
    }
    if let Some((mn, _, mx, _)) = min_max(t, il, tend - tsw, tend) {
        let duty = prm(r, "duty");
        let theory = (vin - vout_set) * duty / (l * fsw);
        m.push(
            Measurement::new("il_ripple_ma", "Inductor ripple current ΔI", (mx - mn) * 1e3, "mA")
                .note(format!("theory (Vin−Vout)·D/(L·f) = {}", fmt_si(theory, "A"))),
        );
        m.push(Measurement::new("il_peak_ma", "Peak inductor current", mx * 1e3, "mA").note("size L1's saturation current above this"));
        let cap = theory / (8.0 * fsw * c);
        m.push(
            Measurement::new(
                "ripple_theory_mv",
                "Hand estimate ΔI/(8fC) + ΔI·ESR",
                (cap + theory * esr) * 1e3,
                "mV",
            )
            .note(format!("capacitive {} + ESR {}", fmt_si(cap, "V"), fmt_si(theory * esr, "V"))),
        );
    }
    if let (Some(iin), Some(vv)) = (r.ti("Vin"), r.tv("out")) {
        let pin = average(t, iin, t0, tend).map(|i| -i * vin);
        let rload = prm(r, "rload");
        let v2: Vec<f32> = vv.iter().map(|x| x * x / rload as f32).collect();
        let pout = average(t, &v2, t0, tend);
        if let (Some(pi), Some(po)) = (pin, pout) {
            if pi > 0.0 {
                m.push(
                    Measurement::new("efficiency_pct", "Power-stage efficiency", 100.0 * po / pi, "%").note(format!(
                        "Pout {} / Pin {} (conduction + dead-time losses only)",
                        fmt_si(po, "W"),
                        fmt_si(pi, "W")
                    )),
                );
            }
        }
    }
    m.push(
        Measurement::new("duty_pct", "Duty cycle D", 100.0 * prm(r, "duty"), "%").note("open loop: D = (Vout + I·R + dead-time loss)/Vin"),
    );
    let _ = iout;
    an.window = Some([tend - 3.0 * tsw, tend]);
}

fn measure_vbat(r: &SimResult, m: &mut Vec<Measurement>, an: &mut Extra) {
    let (vbat, rtop, rbot, cf, chold, fs, tacq) = (
        prm(r, "vbat"),
        prm(r, "rtop"),
        prm(r, "rbot"),
        prm(r, "cf"),
        prm(r, "chold"),
        prm(r, "fs"),
        prm(r, "tacq"),
    );
    if let Some(i) = r.op_i("R1") {
        let ua = i * 1e6;
        m.push(
            Measurement::new("vbat_divider_ua", "Divider current at full charge", ua, "µA")
                .note(format!(
                    "= {} / ({} + {}); requirement R-014 ≤ 2 µA",
                    fmt_si(vbat, "V"),
                    fmt_si(rtop, "Ω"),
                    fmt_si(rbot, "Ω")
                ))
                .check(ua <= 2.0),
        );
        let mah_year = i * 1e3 * 8760.0;
        m.push(Measurement::new("drain_mah_year", "Charge drained per year", mah_year, "mAh").note("always-on divider during storage"));
    }
    let vfinal = vbat * rbot / (rtop + rbot);
    let rail = 1.8;
    m.push(
        Measurement::new("adc_peak_v", "ADC pin at full charge", vfinal, "V")
            .note(if vfinal > rail {
                format!("exceeds the {rail} V SoC rail / ADC range — lower the ratio, e.g. R1 1.5 MΩ, R2 1.0 MΩ")
            } else {
                "within the 1.8 V ADC range".into()
            })
            .check(vfinal <= rail),
    );
    let tau = rtop * rbot / (rtop + rbot) * cf;
    m.push(Measurement::new("tau_ms", "Time constant (R1‖R2)·C6", tau * 1e3, "ms"));
    let lsb = 1.8 / 4096.0;
    if let (Some(tr), Some(v)) = (r.tran.as_ref(), r.tv("adc")) {
        let t = &tr.time;
        if let Some(ts) = settle_time(t, v, vfinal, lsb, 0.0) {
            m.push(
                Measurement::new("settle_ms", "Settling to 1 LSB (12-bit, 1.8 V)", ts * 1e3, "ms")
                    .note(format!("1 LSB = {}", fmt_si(lsb, "V"))),
            );
        } else {
            m.push(
                Measurement::new("settle_ms", "Settling to 1 LSB (12-bit, 1.8 V)", f64::NAN, "ms")
                    .note("not settled within the simulated time"),
            );
        }
        // kick-back at the last complete sample
        let tend = *t.last().unwrap();
        let n = ((tend - 3.0 * tacq) * fs).floor();
        if n >= 1.0 {
            let ts = n / fs;
            let before = at(t, v, ts - 1e-7, false).unwrap_or(0.0);
            if let Some((mn, _, _, _)) = min_max(t, v, ts, ts + 3.0 * tacq) {
                m.push(
                    Measurement::new("kickback_uv", "Sample kick-back on the ADC pin", (before - mn) * 1e6, "µV").note(format!(
                        "theory V·Chold/(C6+Chold) = {}",
                        fmt_si(before * chold / (cf + chold), "V")
                    )),
                );
            }
            if let (Some(h), Some(vh)) = (at(t, r.tv("hold").unwrap(), ts + tacq, false), at(t, v, ts + tacq, false)) {
                m.push(Measurement::new(
                    "acq_error_lsb",
                    "Hold-capacitor error at end of acquisition",
                    (vh - h).abs() / lsb,
                    "LSB",
                ));
            }
        }
        ann(an, "tran", "y", vfinal, "final value");
    }
}

fn measure_irled(r: &SimResult, m: &mut Vec<Measurement>, an: &mut Extra) {
    let (Some(tr), Some(id), Some(va), Some(vd)) = (r.tran.as_ref(), r.ti("D1"), r.tv("a"), r.tv("d")) else {
        return;
    };
    let t = &tr.time;
    let tend = *t.last().unwrap();
    let Some((_, _, imax, tmax)) = min_max(t, id, 0.0, tend) else {
        return;
    };
    let k = t.iter().position(|x| *x >= tmax).unwrap_or(0);
    let vf = (va[k] - vd[k]) as f64;
    let vds = vd[k] as f64;
    m.push(Measurement::new("led_peak_ma", "LED pulse current", imax * 1e3, "mA"));
    m.push(Measurement::new("led_vf", "LED forward voltage", vf, "V"));
    m.push(
        Measurement::new("vds_on_mv", "MOSFET Vds(on)", vds * 1e3, "mV").note(format!("Rds(on) = {}", fmt_si(vds / imax.max(1e-12), "Ω"))),
    );
    let fpwm = prm(r, "fpwm");
    let per = 1.0 / fpwm;
    if let Some(avg) = average(t, id, 50e-6, 50e-6 + per) {
        m.push(Measurement::new("led_avg_ma", "Average LED current", avg * 1e3, "mA").note("what the battery budget sees"));
    }
    let rled = prm(r, "rled");
    m.push(Measurement::new(
        "prled_mw",
        "Rled dissipation (pulse)",
        imax * imax * rled * 1e3,
        "mW",
    ));
    let lo = crossing(t, id, 0.1 * imax, true, 50e-6);
    let hi = crossing(t, id, 0.9 * imax, true, 50e-6);
    if let (Some(a), Some(b)) = (lo, hi) {
        m.push(Measurement::new("rise_ns", "Current rise time (10–90 %)", (b - a) * 1e9, "ns").note("set by Rg·Ciss and the threshold"));
    }
    an.window = Some([40e-6, 50e-6 + 1.5 * prm(r, "duty") * per + 20e-6]);
}

fn measure_touch(r: &SimResult, m: &mut Vec<Measurement>, an: &mut Extra) {
    let (vdd, rr, cpad, cf, cb) = (prm(r, "vdd"), prm(r, "r"), prm(r, "cpad"), prm(r, "cfinger"), prm(r, "cbody"));
    let vth = 0.7 * vdd;
    let (Some(tr), Some(a), Some(b)) = (r.tran.as_ref(), r.tv("pad_nf"), r.tv("pad_f")) else {
        return;
    };
    let t = &tr.time;
    let td = 2e-6;
    let ta = crossing(t, a, vth, true, td);
    let tb = crossing(t, b, vth, true, td);
    let k = (1.0f64 / 0.3).ln();
    let ceff = cf * cb / (cf + cb);
    if let Some(ta) = ta {
        m.push(
            Measurement::new("t_nf_us", "Threshold crossing · no touch", (ta - td) * 1e6, "µs")
                .note(format!("theory R·C·ln(1/0.3) = {}", fmt_si(rr * cpad * k, "s"))),
        );
        ann(an, "tran", "x", ta, "no touch");
    }
    if let Some(tb) = tb {
        m.push(
            Measurement::new("t_f_us", "Threshold crossing · finger", (tb - td) * 1e6, "µs")
                .note(format!("theory R·(C + Cf‖Cb)·ln(1/0.3) = {}", fmt_si(rr * (cpad + ceff) * k, "s"))),
        );
        ann(an, "tran", "x", tb, "finger");
    }
    if let (Some(ta), Some(tb)) = (ta, tb) {
        let dt = tb - ta;
        m.push(Measurement::new("delta_us", "Touch signal Δt", dt * 1e6, "µs"));
        m.push(
            Measurement::new("counts_16mhz", "Δ counts @ 16 MHz timer", dt * 16e6, "counts")
                .note("well above count noise → robust detection"),
        );
    }
    m.push(Measurement::new(
        "ceff_pf",
        "Finger adds (Cf in series with Cbody)",
        ceff * 1e12,
        "pF",
    ));
    ann(an, "tran", "y", vth, "GPIO threshold 0.7·VDD");
    an.window = Some([0.0, 70e-6]);
}

fn measure_cmos(r: &SimResult, m: &mut Vec<Measurement>, an: &mut Extra) {
    let vdd = prm(r, "vdd");
    if let (Some(dc), Some(oi)) = (r.dc.as_ref(), r.node_idx("out")) {
        let x = &dc.sweep;
        let y = &dc.v[oi];
        // VM: vout = vin
        let mut vm = None;
        for k in 1..x.len() {
            let (d0, d1) = (y[k - 1] as f64 - x[k - 1], y[k] as f64 - x[k]);
            if d0 >= 0.0 && d1 < 0.0 {
                vm = Some(x[k - 1] + d0 / (d0 - d1) * (x[k] - x[k - 1]));
                break;
            }
        }
        // slopes
        let slope: Vec<f64> = (1..x.len()).map(|k| (y[k] as f64 - y[k - 1] as f64) / (x[k] - x[k - 1])).collect();
        let xm: Vec<f64> = (1..x.len()).map(|k| 0.5 * (x[k] + x[k - 1])).collect();
        let gain = slope.iter().cloned().fold(0.0f64, |a, s| a.max(-s));
        let vil = (1..slope.len()).find(|&k| slope[k - 1] > -1.0 && slope[k] <= -1.0).map(|k| xm[k]);
        let vih = (1..slope.len())
            .rev()
            .find(|&k| slope[k - 1] <= -1.0 && slope[k] > -1.0)
            .map(|k| xm[k]);
        let voh = y[0] as f64;
        let vol = *y.last().unwrap() as f64;
        if let Some(vm) = vm {
            m.push(Measurement::new("vm", "Switching threshold VM", vm, "V").note(format!("{:.0} % of VDD", 100.0 * vm / vdd)));
            ann(an, "dc", "x", vm, "VM");
        }
        m.push(Measurement::new("gain", "Peak voltage gain |dVout/dVin|", gain, "V/V"));
        if let (Some(vil), Some(vih)) = (vil, vih) {
            m.push(Measurement::new("nml", "Noise margin low NML = VIL − VOL", vil - vol, "V"));
            m.push(Measurement::new("nmh", "Noise margin high NMH = VOH − VIH", voh - vih, "V"));
        }
    }
    if let (Some(tr), Some(vi), Some(vo)) = (r.tran.as_ref(), r.tv("in"), r.tv("out")) {
        let t = &tr.time;
        let half = vdd / 2.0;
        if let (Some(a), Some(b)) = (crossing(t, vi, half, true, 0.0), crossing(t, vo, half, false, 0.0)) {
            m.push(Measurement::new("tphl_ps", "Propagation delay tpHL", (b - a) * 1e12, "ps"));
        }
        if let (Some(a), Some(b)) = (crossing(t, vi, half, false, 0.0), crossing(t, vo, half, true, 0.0)) {
            m.push(Measurement::new("tplh_ps", "Propagation delay tpLH", (b - a) * 1e12, "ps"));
        }
        if let Some(idd) = r.ti("Vdd") {
            let tend = *t.last().unwrap();
            if let Some((mn, _, _, _)) = min_max(t, idd, 0.0, tend) {
                m.push(Measurement::new("ipeak_ua", "Peak supply current", -mn * 1e6, "µA"));
            }
            // energy from VDD over one full input period (2 ns, starting at 0.1 ns)
            if let Some(avg) = average(t, idd, 0.1e-9, 2.1e-9) {
                let e = -avg * vdd * 2e-9;
                m.push(
                    Measurement::new("energy_fj", "Energy per cycle from VDD", e * 1e15, "fJ").note(format!(
                        "≈ C·VDD² = {} for the load cap alone",
                        fmt_si(prm(r, "cl") * vdd * vdd, "J")
                    )),
                );
            }
        }
    }
}

fn measure_rcaa(r: &SimResult, m: &mut Vec<Measurement>, an: &mut Extra) {
    let Some(ac) = r.ac.as_ref() else { return };
    let Some(oi) = r.node_idx("out") else { return };
    let mag = &ac.mag[oi];
    let f = &ac.freq;
    let g0 = mag[0] as f64;
    let mut ann_x = vec![];
    if let Some(f3) = f_3db(f, mag, g0) {
        m.push(Measurement::new("f3db_khz", "−3 dB frequency", f3 / 1e3, "kHz"));
        ann_x.push((f3, "−3 dB"));
    }
    let db = |x: f64| 20.0 * (x / g0).log10();
    for (fx, key, label) in [
        (8e3, "att_nyq_db", "Gain at Nyquist (8 kHz)"),
        (prm(r, "fint"), "att_int_db", "Gain at the interferer"),
        (16e3, "att_fs_db", "Gain at fs (16 kHz)"),
    ] {
        if let Some(g) = at(f, mag, fx, true) {
            m.push(Measurement::new(key, label, db(g), "dB"));
        }
    }
    let (r1, c1, r2, c2) = (prm(r, "r1"), prm(r, "c1"), prm(r, "r2"), prm(r, "c2"));
    // poles of s²·R1C1R2C2 + s(R1C1 + R2C2 + R1C2) + 1
    let (a, b) = (r1 * c1 * r2 * c2, r1 * c1 + r2 * c2 + r1 * c2);
    let disc = b * b - 4.0 * a;
    if disc >= 0.0 {
        let p1 = (b - disc.sqrt()) / (2.0 * a);
        let p2 = (b + disc.sqrt()) / (2.0 * a);
        let tp = 2.0 * std::f64::consts::PI;
        m.push(
            Measurement::new("poles_khz", "Pole frequencies", p1 / tp / 1e3, "kHz")
                .note(format!("and {} (real poles of the loaded RC ladder)", fmt_si(p2 / tp, "Hz"))),
        );
    }
    ann_x.push((8e3, "fs/2"));
    ann_x.push((prm(r, "fint"), "interferer"));
    for (x, l) in ann_x {
        ann(an, "ac", "x", x, l);
    }
}

fn measure_opamp(r: &SimResult, m: &mut Vec<Measurement>, an: &mut Extra) {
    let (a0, gbw, rf, rg) = (prm(r, "a0"), prm(r, "gbw"), prm(r, "rf"), prm(r, "rg"));
    let beta = rg / (rf + rg);
    if let (Some(ac), Some(oi)) = (r.ac.as_ref(), r.node_idx("out")) {
        let f = ac.freq.clone();
        let mag = ac.mag[oi].clone();
        let g1k = at(&f, &mag, 1e3, true).unwrap_or(f64::NAN);
        let ideal = 1.0 + rf / rg;
        m.push(Measurement::new("gain_1k", "Gain at 1 kHz", g1k, "V/V").note(format!(
            "{:.2} dB; ideal 1 + Rf/Rg = {:.3}, finite-A0 theory A0/(1+A0·β) = {:.3}",
            20.0 * g1k.log10(),
            ideal,
            a0 / (1.0 + a0 * beta)
        )));
        let g0 = mag[0] as f64;
        if let Some(f3) = f_3db(&f, &mag, g0) {
            m.push(
                Measurement::new("bw_khz", "Closed-loop bandwidth (−3 dB)", f3 / 1e3, "kHz")
                    .note(format!("theory ≈ GBW·β = {}", fmt_si(gbw * beta, "Hz"))),
            );
            m.push(Measurement::new("gbw_mhz", "Measured gain × bandwidth", g0 * f3 / 1e6, "MHz"));
            ann(an, "ac", "x", f3, "−3 dB");
        }
    }
    if let (Some(tr), Some(vo)) = (r.tran.as_ref(), r.tv("out")) {
        let tend = *tr.time.last().unwrap();
        if let Some((mn, _, mx, _)) = min_max(&tr.time, vo, tend - 1e-3, tend) {
            m.push(Measurement::new("vout_pp", "Output swing (peak-to-peak)", mx - mn, "V"));
        }
    }
}

fn measure_rlc(r: &SimResult, m: &mut Vec<Measurement>, an: &mut Extra) {
    let (rr, l, c) = (prm(r, "r"), prm(r, "l"), prm(r, "c"));
    let f0 = 1.0 / (2.0 * std::f64::consts::PI * (l * c).sqrt());
    let q = (l / c).sqrt() / rr;
    if let (Some(ac), Some(ii), Some(ia)) = (r.ac.as_ref(), r.node_idx("in"), r.node_idx("a")) {
        // current magnitude |V(in) − V(a)| / R from polar node voltages
        let cur: Vec<f32> = (0..ac.freq.len())
            .map(|k| {
                let (m1, p1) = (ac.mag[ii][k] as f64, (ac.phase[ii][k] as f64).to_radians());
                let (m2, p2) = (ac.mag[ia][k] as f64, (ac.phase[ia][k] as f64).to_radians());
                let (re, im) = (m1 * p1.cos() - m2 * p2.cos(), m1 * p1.sin() - m2 * p2.sin());
                (re.hypot(im) / rr) as f32
            })
            .collect();
        let (kmax, imax) = cur
            .iter()
            .enumerate()
            .fold((0, 0.0f32), |acc, (k, v)| if *v > acc.1 { (k, *v) } else { acc });
        let fpk = ac.freq[kmax];
        let lvl = imax as f64 / std::f64::consts::SQRT_2;
        // half-power frequencies, interpolated in log-frequency
        let f = &ac.freq;
        let lerp = |k0: usize, k1: usize| {
            let (a, b) = (cur[k0] as f64, cur[k1] as f64);
            let w = if (b - a).abs() > 0.0 { (lvl - a) / (b - a) } else { 0.0 };
            (f[k0].ln() + w * (f[k1].ln() - f[k0].ln())).exp()
        };
        let lo = (1..=kmax).rev().find(|&k| (cur[k - 1] as f64) < lvl).map(|k| lerp(k - 1, k));
        let hi = (kmax + 1..cur.len()).find(|&k| (cur[k] as f64) < lvl).map(|k| lerp(k - 1, k));
        m.push(
            Measurement::new("f0_khz", "Resonance (current peak)", fpk / 1e3, "kHz")
                .note(format!("theory 1/(2π√LC) = {}", fmt_si(f0, "Hz"))),
        );
        if let (Some(lo), Some(hi)) = (lo, hi) {
            m.push(Measurement::new("q", "Q = f0 / bandwidth", fpk / (hi - lo), "").note(format!("theory (1/R)·√(L/C) = {q:.2}")));
        }
        ann(an, "ac", "x", f0, "f0");
    }
    if let (Some(tr), Some(vb)) = (r.tran.as_ref(), r.tv("b")) {
        let t = &tr.time;
        let mut zc = vec![];
        let mut after = 21e-6;
        while let Some(tc) = crossing(t, vb, 1.0, true, after) {
            zc.push(tc);
            after = tc + 1e-9;
            if zc.len() > 6 {
                break;
            }
        }
        if zc.len() >= 3 {
            let per = (zc[zc.len() - 1] - zc[0]) / (zc.len() - 1) as f64;
            m.push(
                Measurement::new("ring_khz", "Ringing frequency (step response)", 1e-3 / per, "kHz").note(format!(
                    "damped f0·√(1 − 1/(4Q²)) = {}",
                    fmt_si(f0 * (1.0 - 1.0 / (4.0 * q * q)).max(0.0).sqrt(), "Hz")
                )),
            );
        }
        let tend = *t.last().unwrap();
        if let Some((_, _, mx, _)) = min_max(t, vb, 0.0, tend) {
            m.push(Measurement::new("overshoot_pct", "Step overshoot", (mx - 1.0) * 100.0, "%"));
        }
    }
}

fn measure_rect(r: &SimResult, m: &mut Vec<Measurement>, _an: &mut Extra) {
    let (f, c, rl) = (prm(r, "f"), prm(r, "c"), prm(r, "rl"));
    let (Some(tr), Some(vo)) = (r.tran.as_ref(), r.tv("out")) else {
        return;
    };
    let t = &tr.time;
    let tend = *t.last().unwrap();
    let per = 1.0 / f;
    if let Some((mn, _, mx, _)) = min_max(t, vo, tend - per, tend) {
        m.push(
            Measurement::new("vout_peak", "Output peak", mx, "V")
                .note(format!("input peak {} minus the diode drop", fmt_si(prm(r, "vpk"), "V"))),
        );
        m.push(
            Measurement::new("ripple_v", "Ripple (last cycle)", mx - mn, "V")
                .note(format!("theory ≈ Vpk/(f·R·C) = {}", fmt_si(mx / (f * rl * c), "V"))),
        );
    }
    if let Some(avg) = average(t, vo, tend - per, tend) {
        m.push(Measurement::new("vout_avg", "Average output", avg, "V"));
    }
    if let Some(id) = r.ti("D1") {
        if let Some((_, _, mx, _)) = min_max(t, id, tend - per, tend) {
            m.push(Measurement::new("id_peak_ma", "Diode peak current", mx * 1e3, "mA").note("short, tall recharge pulses"));
        }
    }
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

fn metric_from(p: &Project, id: &str, key: &str, metric: &str, label: &str, unit: &str, op_only: bool) -> Option<Metric> {
    let res = if op_only {
        run_op_only(id, p)
    } else {
        run(id, &BTreeMap::new(), None, Some(p))
    };
    match res {
        Ok(r) => {
            let mm = r.measurements.iter().find(|x| x.key == key)?;
            Some(Metric::new(metric, label, mm.value, unit, "circuit").with_note(&mm.note))
        }
        Err(e) => Some(Metric::new(metric, label, f64::NAN, unit, "circuit").with_note(&format!("simulation failed: {e}"))),
    }
}

/// `circuit.rail_droop_mv`, `circuit.buck_ripple_mv`, `circuit.vbat_divider_ua`.
pub fn metrics(p: &Project) -> Vec<Metric> {
    [
        metric_from(
            p,
            "rail_load_step",
            "rail_droop_mv",
            "circuit.rail_droop_mv",
            "1.8 V rail droop on display turn-on",
            "mV",
            false,
        ),
        metric_from(
            p,
            "buck_converter",
            "buck_ripple_mv",
            "circuit.buck_ripple_mv",
            "Buck output ripple (p-p)",
            "mV",
            false,
        ),
        metric_from(
            p,
            "vbat_divider",
            "vbat_divider_ua",
            "circuit.vbat_divider_ua",
            "Battery-sense divider current",
            "uA",
            true,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}
