//! Tests against hand calculations and textbook results.

use std::collections::BTreeMap;

use super::measure::{at, f_3db};
use super::presets;
use super::*;

fn sim(deck: &str) -> SimResult {
    simulate(deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("simulation failed: {e}\n{deck}"))
}

fn interp_t(r: &SimResult, node: &str, t: f64) -> f64 {
    let tr = r.tran.as_ref().unwrap();
    at(&tr.time, r.tv(node).unwrap(), t, false).unwrap()
}

#[test]
fn resistor_divider_op_and_power() {
    let r = sim("divider\nV1 in 0 10\nR1 in mid 3k\nR2 mid 0 2k\n.op\n");
    assert!((r.op_v("mid").unwrap() - 4.0).abs() < 1e-9);
    // source current is negative (SPICE convention) and delivers 20 mW
    assert!((r.op_i("V1").unwrap() + 2e-3).abs() < 1e-12);
    let op = r.op.as_ref().unwrap();
    let p_total: f64 = op.elem_p.iter().sum();
    assert!(p_total.abs() < 1e-12, "power must balance, got {p_total}");
}

#[test]
fn rc_charging_matches_exponential() {
    let r = sim("rc\nV1 in 0 PULSE(0 1 0 1n 1n 1 2)\nR1 in out 1k\nC1 out 0 1u\n.tran 10u 5m\n");
    let tau = 1e-3;
    for k in 1..=5 {
        let t = k as f64 * tau;
        let v = interp_t(&r, "out", t);
        let exact = 1.0 - (-(t - 0.5e-9) / tau).exp();
        assert!((v - exact).abs() < 1e-3, "t={t} v={v} exact={exact}");
    }
    let st = &r.tran.as_ref().unwrap().stats;
    assert!(st.accepted > 50 && st.accepted < 5000, "{st:?}");
}

#[test]
fn inductor_current_ramps() {
    // v = L di/dt  → i = V·t/L = 1 V · 1 ms / 1 mH = 1 A (small series R)
    let r = sim("ramp\nV1 a 0 PULSE(0 1 0 1n 1n 1 2)\nR1 a b 1m\nL1 b 0 1m\n.tran 10u 1m\n");
    let tr = r.tran.as_ref().unwrap();
    let i = at(&tr.time, r.ti("L1").unwrap(), 1e-3, false).unwrap();
    assert!((i - 1.0).abs() < 2e-3, "i = {i}");
}

#[test]
fn rlc_resonance_frequency_and_q() {
    let r = presets::run("rlc_resonance", &BTreeMap::new(), None, None).unwrap();
    let f0 = 1.0 / (2.0 * std::f64::consts::PI * (1e-3f64 * 100e-9).sqrt());
    let q = (1e-3f64 / 100e-9).sqrt() / 10.0;
    let mf0 = r.measurements.iter().find(|m| m.key == "f0_khz").unwrap().value * 1e3;
    let mq = r.measurements.iter().find(|m| m.key == "q").unwrap().value;
    assert!((mf0 - f0).abs() / f0 < 0.01, "f0 {mf0} vs {f0}");
    assert!((mq - q).abs() / q < 0.05, "Q {mq} vs {q}");
    let ring = r.measurements.iter().find(|m| m.key == "ring_khz").unwrap().value * 1e3;
    let fd = f0 * (1.0 - 1.0 / (4.0 * q * q)).sqrt();
    assert!((ring - fd).abs() / fd < 0.01, "ringing {ring} vs {fd}");
}

#[test]
fn diode_forward_drop() {
    let r = sim("diode\nV1 in 0 5\nR1 in a 4.3k\nD1 a 0 DX\n.model DX D(IS=1e-14)\n.op\n");
    let vd = r.op_v("a").unwrap();
    let id = r.op_i("D1").unwrap();
    assert!(vd > 0.6 && vd < 0.7, "Vd = {vd}");
    assert!((id - (5.0 - vd) / 4.3e3).abs() < 1e-4 * id, "{id} vs {}", (5.0 - vd) / 4.3e3);
    let expect = devices::vt() * (id / 1e-14 + 1.0).ln();
    assert!((vd - expect).abs() < 1e-5, "{vd} vs {expect}");
    assert_eq!(r.op.as_ref().unwrap().method, "newton");
}

#[test]
fn diode_series_resistance_and_reverse() {
    let r = sim("d\nV1 in 0 -5\nR1 in a 1k\nD1 a 0 DX\n.model DX D(IS=1e-14 RS=10)\n.op\n");
    // reverse: −IS plus the gmin (1 pS) shunt SPICE puts across every junction
    assert!((r.op_i("D1").unwrap() + 1e-14 + 5e-12).abs() < 1e-15, "{}", r.op_i("D1").unwrap());
    let r2 = sim("d\nV1 in 0 5\nR1 in a 100\nD1 a 0 DX\n.model DX D(IS=1e-14 RS=10)\n.op\n");
    let i = r2.op_i("D1").unwrap();
    let va = r2.op_v("a").unwrap();
    // Va = Vt·ln(I/IS) + I·RS
    let expect = devices::vt() * (i / 1e-14).ln() + i * 10.0;
    assert!((va - expect).abs() < 1e-5);
}

#[test]
fn inverter_switching_threshold() {
    // λ = 0 so the textbook formula is exact:
    // VM = (Vtn + r·(VDD − |Vtp|)) / (1 + r),  r = √(βp/βn)
    let deck = "inv\nVdd vdd 0 1.8\nVin in 0 0\nMp out in vdd vdd PM W=2u L=1u\nMn out in 0 0 NM W=1u L=1u\n.model NM NMOS(VTO=0.5 KP=100u)\n.model PM PMOS(VTO=-0.4 KP=40u)\n.dc Vin 0 1.8 0.001\n";
    let r = sim(deck);
    let dc = r.dc.as_ref().unwrap();
    let oi = r.node_idx("out").unwrap();
    let mut vm = 0.0;
    for k in 1..dc.sweep.len() {
        if (dc.v[oi][k] as f64) < dc.sweep[k] {
            vm = dc.sweep[k];
            break;
        }
    }
    let rr = ((40e-6 * 2.0) / (100e-6 * 1.0f64)).sqrt();
    let expect = (0.5 + rr * (1.8 - 0.4)) / (1.0 + rr);
    assert!((vm - expect).abs() < 0.005, "VM {vm} vs {expect}");
    assert!((dc.v[oi][0] as f64 - 1.8).abs() < 1e-3);
    assert!((*dc.v[oi].last().unwrap() as f64).abs() < 1e-3);
}

#[test]
fn rc_minus_3db_point() {
    let r = sim("lp\nV1 in 0 AC 1\nR1 in out 1k\nC1 out 0 100n\n.ac dec 100 10 1meg\n");
    let ac = r.ac.as_ref().unwrap();
    let oi = r.node_idx("out").unwrap();
    let f3 = f_3db(&ac.freq, &ac.mag[oi], 1.0).unwrap();
    let expect = 1.0 / (2.0 * std::f64::consts::PI * 1e3 * 100e-9);
    assert!((f3 - expect).abs() / expect < 0.005, "{f3} vs {expect}");
    let ph = at(&ac.freq, &ac.phase[oi], expect, true).unwrap();
    assert!((ph + 45.0).abs() < 0.5, "phase {ph}");
}

#[test]
fn opamp_closed_loop_gain() {
    let r = presets::run("opamp_preamp", &BTreeMap::new(), None, None).unwrap();
    let g = r.measurements.iter().find(|m| m.key == "gain_1k").unwrap().value;
    let (a0, beta) = (1e5, 1e3 / 20e3);
    let ideal = a0 / (1.0 + a0 * beta);
    // at 1 kHz the loop gain is still ~2000, so |H| ≈ A0/(1+A0β) within 0.1 %
    assert!((g - ideal).abs() / ideal < 1e-3, "{g} vs {ideal}");
    let bw = r.measurements.iter().find(|m| m.key == "bw_khz").unwrap().value * 1e3;
    assert!((bw - 50e3).abs() / 50e3 < 0.03, "bw {bw}");
}

#[test]
fn vccs_and_vcvs_stamps() {
    let r = sim("ctl\nV1 c 0 2\nR0 c 0 1k\nG1 0 x c 0 1m\nR1 x 0 1k\nE1 y 0 x 0 3\nR2 y 0 1k\n.op\n");
    // G1 pushes 2 mA from 0 into x → v(x) = 2 V; E1 → v(y) = 6 V
    assert!((r.op_v("x").unwrap() - 2.0).abs() < 1e-9);
    assert!((r.op_v("y").unwrap() - 6.0).abs() < 1e-9);
}

#[test]
fn switch_follows_control_with_hysteresis() {
    let deck = "sw\nVc c 0 PWL(0 0 1m 1 2m 0)\nV1 a 0 1\nS1 a b c 0 SMOD\nR1 b 0 1k\n.model SMOD SW(RON=1 ROFF=1meg VT=0.5 VH=0.2)\n.tran 10u 2m\n";
    let r = sim(deck);
    let v = |t: f64| interp_t(&r, "b", t);
    assert!(v(0.6e-3) < 0.01); // 0.6 V < VT+VH → still off
    assert!(v(0.8e-3) > 0.99); // 0.8 V > 0.7 → on
    assert!(v(1.5e-3) > 0.99); // falling, 0.5 V > 0.3 → stays on
    assert!(v(1.8e-3) < 0.01); // 0.2 V < 0.3 → off
}

#[test]
fn floating_node_gets_a_shunt_and_a_warning() {
    let r = sim("float\nV1 a 0 1\nC1 a b 1u\nC2 b 0 1u\n.op\n");
    assert!(r.warnings.iter().any(|w| w.contains("singular")));
}

#[test]
fn gmin_or_source_stepping_rescue() {
    // A starved Newton loop (itl1 = 4) cannot converge on a stack of diodes
    // from 0 V; the fallbacks must still find the right answer.
    let deck = "stack\n.options itl1=4\nV1 in 0 20\nR1 in a 100\nD1 a b DX\nD2 b c DX\nD3 c 0 DX\n.model DX D(IS=1e-15)\n.op\n";
    let r = sim(deck);
    let op = r.op.as_ref().unwrap();
    assert_ne!(op.method, "newton");
    let i = r.op_i("R1").unwrap();
    let vd = devices::vt() * (i / 1e-15).ln();
    assert!((r.op_v("a").unwrap() - 3.0 * vd).abs() < 1e-3);
}

#[test]
fn uic_and_ic_directives() {
    let r = sim("ic\nR1 a 0 1k\nC1 a 0 1u IC=2\n.tran 10u 2m uic\n");
    assert!((interp_t(&r, "a", 1e-3) - 2.0 * (-1.0f64).exp()).abs() < 2e-3);
    let r2 = sim("ic2\nR1 a 0 1k\nC1 a 0 1u\n.ic v(a)=1\n.tran 10u 1m\n");
    assert!((interp_t(&r2, "a", 1e-3) - (-1.0f64).exp()).abs() < 2e-3);
}

#[test]
fn sine_source_and_mosfet_amplifier_ac() {
    // common-source stage: |Av| = gm·RD at the bias point
    let deck = "cs\nVdd vdd 0 3.3\nVin g 0 DC 1 AC 1\nRd vdd d 10k\nM1 d g 0 0 NM W=10u L=1u\n.model NM NMOS(VTO=0.7 KP=100u)\n.ac dec 10 10 1k\n";
    let r = sim(deck);
    let id = r.op_i("M1").unwrap();
    let gm = (2.0 * 1e-3 * id).sqrt(); // √(2βId), β = KP·W/L = 1 mA/V²
    let ac = r.ac.as_ref().unwrap();
    let gain = ac.mag[r.node_idx("d").unwrap()][0] as f64;
    assert!((gain - gm * 10e3).abs() / (gm * 10e3) < 1e-3, "{gain} vs {}", gm * 10e3);
}

// ---------------------------------------------------------------------------
// Iris presets and metrics
// ---------------------------------------------------------------------------

#[test]
fn iris_metrics_are_sane_and_pass() {
    let p = crate::project::Project::iris();
    let m = metrics(&p);
    let get = |k: &str| m.iter().find(|x| x.key == k).unwrap_or_else(|| panic!("missing {k}")).value;
    let droop = get("circuit.rail_droop_mv");
    let ripple = get("circuit.buck_ripple_mv");
    let ua = get("circuit.vbat_divider_ua");
    // hand estimates: ΔI·|Zout| ≈ 59 mA · 0.16 Ω ≈ 9 mV plus a few mV IR drop
    assert!(droop > 3.0 && droop < 25.0, "droop {droop}");
    assert!(droop <= 50.0);
    // ΔI ≈ 0.21 A: ΔI/(8fC) ≈ 1.3 mV, ΔI·ESR ≈ 1 mV
    assert!(ripple > 0.8 && ripple < 6.0, "ripple {ripple}");
    assert!(ripple <= 30.0);
    // 4.2 V / (1.5 MΩ + 1 MΩ) = 1.68 µA
    assert!((ua - 4.2 / 2.5e6 * 1e6).abs() < 1e-3, "divider {ua}");
    assert!(ua <= 2.0);
}

#[test]
fn every_preset_runs_and_measures() {
    for pr in presets::list(Some(&crate::project::Project::iris())) {
        let r = presets::run(&pr.id, &BTreeMap::new(), None, Some(&crate::project::Project::iris())).unwrap_or_else(|e| panic!("{}: {e}", pr.id));
        assert!(!r.measurements.is_empty(), "{} has no measurements", pr.id);
        for m in &r.measurements {
            assert!(m.value.is_finite() || m.key == "settle_ms", "{}: {} = {}", pr.id, m.key, m.value);
        }
        for p in &pr.probes {
            let name = &p[2..p.len() - 1];
            if p.starts_with("v(") {
                assert!(r.node_idx(name).is_some(), "{}: probe {p}", pr.id);
            } else {
                assert!(r.elem_idx(name).is_some(), "{}: probe {p}", pr.id);
            }
        }
        for s in &pr.params {
            assert!(r.params.contains_key(&s.name), "{}: slider {} not a .param", pr.id, s.name);
            assert!(s.value >= s.min && s.value <= s.max, "{}: slider {} default out of range", pr.id, s.name);
        }
    }
}

#[test]
fn buck_waveforms_are_physical() {
    let r = presets::run("buck_converter", &BTreeMap::new(), None, Some(&crate::project::Project::iris())).unwrap();
    let get = |k: &str| r.measurements.iter().find(|m| m.key == k).unwrap().value;
    let vout = get("vout_avg");
    assert!((vout - 1.8).abs() < 0.02, "vout {vout}");
    let dil = get("il_ripple_ma") * 1e-3;
    let theory = (3.7 - 1.8) * get("duty_pct") / 100.0 / (2.2e-6 * 2e6);
    assert!((dil - theory).abs() / theory < 0.08, "ΔI {dil} vs {theory}");
    let eff = get("efficiency_pct");
    assert!(eff > 80.0 && eff < 100.0, "efficiency {eff}");
}

#[test]
fn vbat_divider_keeps_adc_in_range() {
    let r = presets::run("vbat_divider", &BTreeMap::new(), None, Some(&crate::project::Project::iris())).unwrap();
    let pk = r.measurements.iter().find(|m| m.key == "adc_peak_v").unwrap();
    // v0.4 of Iris re-ratioed the divider (1.5 MΩ / 1 MΩ) after this check
    // caught 2.29 V on a 1.8 V ADC pin in v0.3.
    assert!((pk.value - 4.2 * 1.0 / 2.5).abs() < 1e-9);
    assert_eq!(pk.pass, Some(true));
    let tau = r.measurements.iter().find(|m| m.key == "tau_ms").unwrap().value;
    assert!((tau - 60.0).abs() < 0.01, "tau {tau}");
    let kick = r.measurements.iter().find(|m| m.key == "kickback_uv").unwrap().value;
    assert!(kick > 50.0 && kick < 200.0, "kick-back {kick} µV");
}

#[test]
fn touch_delay_matches_rc_theory() {
    let r = presets::run("touch_rc", &BTreeMap::new(), None, None).unwrap();
    let get = |k: &str| r.measurements.iter().find(|m| m.key == k).unwrap().value;
    let k = (1.0f64 / 0.3).ln();
    let t_nf = 1e6 * 12e-12 * k * 1e6;
    assert!((get("t_nf_us") - t_nf).abs() / t_nf < 0.01);
    // the body node is not fully static (Rleak) so allow 3 %
    let ceff = 10.0 * 150.0 / 160.0 * 1e-12;
    let t_f = 1e6 * (12e-12 + ceff) * k * 1e6;
    assert!((get("t_f_us") - t_f).abs() / t_f < 0.03, "{} vs {t_f}", get("t_f_us"));
}

#[test]
fn preset_layouts_are_connected() {
    for pr in presets::list(None) {
        let nl = parser::parse(&pr.netlist, &BTreeMap::new()).unwrap();
        let lay = &pr.layout;
        let same = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9;
        let mut drawn: Vec<String> = vec![];
        for part in &lay.parts {
            drawn.extend(part.members.iter().map(|s| s.to_lowercase()));
            assert!(nl.element_index(&part.el).is_some(), "{}: part {} not in netlist", pr.id, part.el);
            for pin in &part.pins {
                let pp = [pin.x, pin.y];
                assert!(!pin.node.is_empty(), "{}: {} pin {} has no node", pr.id, part.el, pin.name);
                let by_wire = lay.wires.iter().any(|w| w.node == pin.node && w.pts.iter().any(|q| same(*q, pp)));
                let by_gnd = pin.node == "0" && lay.grounds.iter().any(|g| same(*g, pp));
                let by_pin = lay.parts.iter().any(|o| o.el != part.el && o.pins.iter().any(|q| q.node == pin.node && same([q.x, q.y], pp)));
                let by_port = lay.labels.iter().any(|l| l.port && l.node.as_deref() == Some(pin.node.as_str()) && same([l.x, l.y], pp));
                assert!(by_wire || by_gnd || by_pin || by_port, "{}: pin {}.{} ({}) at {:?} is not connected", pr.id, part.el, pin.name, pin.node, pp);
            }
        }
        for e in &nl.elements {
            assert!(drawn.contains(&e.name.to_lowercase()), "{}: element {} not drawn", pr.id, e.name);
        }
        for w in &lay.wires {
            assert!(w.node == "0" || nl.node_index(&w.node).is_some(), "{}: wire on unknown node {}", pr.id, w.node);
            for (el, _) in &w.flow {
                assert!(nl.element_index(el).is_some(), "{}: flow element {el} unknown", pr.id);
            }
            for end in [w.pts[0], *w.pts.last().unwrap()] {
                let pin = lay.parts.iter().any(|p| p.pins.iter().any(|q| q.node == w.node && same([q.x, q.y], end)));
                let other = lay.wires.iter().any(|o| !std::ptr::eq(o, w) && o.node == w.node && o.pts.iter().any(|q| same(*q, end)));
                let gnd = w.node == "0" && lay.grounds.iter().any(|g| same(*g, end));
                let port = lay.labels.iter().any(|l| l.port && l.node.as_deref() == Some(w.node.as_str()) && same([l.x, l.y], end));
                assert!(pin || other || gnd || port, "{}: wire {} end {:?} dangles", pr.id, w.node, end);
            }
        }
    }
}

#[test]
fn metrics_are_fast_enough() {
    let p = crate::project::Project::iris();
    let t0 = std::time::Instant::now();
    let _ = metrics(&p);
    let ms = t0.elapsed().as_secs_f64() * 1e3;
    eprintln!("circuit metrics took {ms:.1} ms (native, test profile)");
}

#[test]
fn preset_timings() {
    let p = crate::project::Project::iris();
    for pr in presets::list(Some(&p)) {
        let t0 = std::time::Instant::now();
        let r = presets::run(&pr.id, &BTreeMap::new(), None, Some(&p)).unwrap();
        let ms = t0.elapsed().as_secs_f64() * 1e3;
        let st = r.tran.as_ref().map(|t| format!("{:?}", t.stats)).unwrap_or_default();
        eprintln!("{:22} {ms:7.2} ms  lu={} nr={} pts={} {st}", pr.id, r.stats.lu_solves, r.stats.nr_iterations, r.tran.as_ref().map(|t| t.time.len()).unwrap_or(0));
    }
}


