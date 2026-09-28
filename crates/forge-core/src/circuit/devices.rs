//! Device equations: independent-source waveforms, the SPICE2 junction diode,
//! the level-1 (Shichman–Hodges) MOSFET and the voltage-controlled switch,
//! plus the Newton–Raphson voltage-limiting functions from SPICE.
//!
//! References:
//! * L. W. Nagel, "SPICE2: A Computer Program to Simulate Semiconductor
//!   Circuits", UC Berkeley ERL Memo M520, 1975 — diode model, `pnjlim`.
//! * H. Shichman and D. A. Hodges, "Modeling and simulation of insulated-gate
//!   field-effect transistor switching circuits", IEEE J. Solid-State
//!   Circuits SC-3(3), 1968 — the level-1 MOSFET equations.
//! * ngspice User's Manual §4.1 (source waveforms), §3.2.x (SW model).

use super::parser::{DiodeModel, MosModel, Source, SwModel, Wave};

/// Boltzmann constant (J/K) and elementary charge (C).
pub const K_B: f64 = 1.380_649e-23;
pub const Q_E: f64 = 1.602_176_634e-19;
/// Nominal simulation temperature, 27 °C (SPICE default).
pub const T_NOM: f64 = 300.15;

/// Thermal voltage kT/q at 27 °C ≈ 25.865 mV.
pub fn vt() -> f64 {
    K_B * T_NOM / Q_E
}

// ---------------------------------------------------------------------------
// Sources
// ---------------------------------------------------------------------------

/// Resolve the ngspice defaults that depend on the transient setup:
/// PULSE tr/tf default to `tstep`, pw/per to `tstop`; SIN freq to 1/tstop;
/// EXP tau1/tau2 default to `tstep`, td2 to td1 + tstep.
pub fn resolve_wave(w: &Wave, tstep: f64, tstop: f64) -> Wave {
    match w {
        Wave::Pulse {
            v1,
            v2,
            td,
            tr,
            tf,
            pw,
            per,
        } => Wave::Pulse {
            v1: *v1,
            v2: *v2,
            td: *td,
            tr: Some(tr.unwrap_or(tstep)),
            tf: Some(tf.unwrap_or(tstep)),
            pw: Some(pw.unwrap_or(tstop)),
            per: Some(per.unwrap_or(f64::INFINITY)),
        },
        Wave::Sin {
            vo,
            va,
            freq,
            td,
            theta,
            phase_deg,
        } => Wave::Sin {
            vo: *vo,
            va: *va,
            freq: Some(freq.unwrap_or(1.0 / tstop)),
            td: *td,
            theta: *theta,
            phase_deg: *phase_deg,
        },
        Wave::Exp {
            v1,
            v2,
            td1,
            tau1,
            td2,
            tau2,
        } => Wave::Exp {
            v1: *v1,
            v2: *v2,
            td1: *td1,
            tau1: Some(tau1.unwrap_or(tstep)),
            td2: Some(td2.unwrap_or(td1 + tstep)),
            tau2: Some(tau2.unwrap_or(tstep)),
        },
        other => other.clone(),
    }
}

/// Value of a (resolved) waveform at time `t`.
pub fn wave_value(w: &Wave, dc: f64, t: f64) -> f64 {
    match w {
        Wave::None => dc,
        Wave::Pulse {
            v1,
            v2,
            td,
            tr,
            tf,
            pw,
            per,
        } => {
            let (tr, tf, pw, per) = (
                tr.unwrap_or(0.0),
                tf.unwrap_or(0.0),
                pw.unwrap_or(f64::INFINITY),
                per.unwrap_or(f64::INFINITY),
            );
            if t < *td {
                return *v1;
            }
            let mut tt = t - td;
            if per.is_finite() && per > 0.0 && tt >= per {
                tt -= (tt / per).floor() * per;
            }
            if tt < tr {
                v1 + (v2 - v1) * tt / tr
            } else if tt < tr + pw {
                *v2
            } else if tt < tr + pw + tf {
                v2 + (v1 - v2) * (tt - tr - pw) / tf
            } else {
                *v1
            }
        }
        Wave::Sin {
            vo,
            va,
            freq,
            td,
            theta,
            phase_deg,
        } => {
            let ph = phase_deg.to_radians();
            if t < *td {
                vo + va * ph.sin()
            } else {
                let f = freq.unwrap_or(0.0);
                let x = t - td;
                vo + va * (-x * theta).exp() * (2.0 * std::f64::consts::PI * f * x + ph).sin()
            }
        }
        Wave::Exp {
            v1,
            v2,
            td1,
            tau1,
            td2,
            tau2,
        } => {
            let (tau1, td2, tau2) = (tau1.unwrap_or(1e-12), td2.unwrap_or(f64::INFINITY), tau2.unwrap_or(1e-12));
            if t < *td1 {
                *v1
            } else if t < td2 {
                v1 + (v2 - v1) * (1.0 - (-(t - td1) / tau1).exp())
            } else {
                v1 + (v2 - v1) * (1.0 - (-(t - td1) / tau1).exp()) + (v1 - v2) * (1.0 - (-(t - td2) / tau2).exp())
            }
        }
        Wave::Pwl { points } => {
            if points.is_empty() {
                return dc;
            }
            if t <= points[0].0 {
                return points[0].1;
            }
            for w in points.windows(2) {
                let ((t0, v0), (t1, v1)) = (w[0], w[1]);
                if t <= t1 {
                    return if t1 > t0 { v0 + (v1 - v0) * (t - t0) / (t1 - t0) } else { v1 };
                }
            }
            points[points.len() - 1].1
        }
    }
}

/// Transient value of a source (the DC value when it has no waveform).
pub fn source_value(s: &Source, t: f64) -> f64 {
    wave_value(&s.wave, s.dc, t)
}

/// Time points where the waveform has a corner (slope discontinuity). The
/// integrator lands exactly on them and restarts with backward Euler.
pub fn breakpoints(w: &Wave, tstop: f64, out: &mut Vec<f64>, limit: usize) {
    match w {
        Wave::Pulse { td, tr, tf, pw, per, .. } => {
            let (tr, tf, pw, per) = (
                tr.unwrap_or(0.0),
                tf.unwrap_or(0.0),
                pw.unwrap_or(f64::INFINITY),
                per.unwrap_or(f64::INFINITY),
            );
            let mut base = *td;
            loop {
                for c in [0.0, tr, tr + pw, tr + pw + tf] {
                    let t = base + c;
                    if t.is_finite() && t > 0.0 && t < tstop {
                        out.push(t);
                    }
                }
                if !per.is_finite() || per <= 0.0 {
                    break;
                }
                base += per;
                if base >= tstop || out.len() > limit {
                    break;
                }
            }
        }
        Wave::Sin { td, .. } => {
            if *td > 0.0 && *td < tstop {
                out.push(*td);
            }
        }
        Wave::Exp { td1, td2, .. } => {
            for t in [Some(*td1), *td2].into_iter().flatten() {
                if t > 0.0 && t < tstop {
                    out.push(t);
                }
            }
        }
        Wave::Pwl { points } => {
            for (t, _) in points {
                if *t > 0.0 && *t < tstop {
                    out.push(*t);
                }
            }
        }
        Wave::None => {}
    }
}

// ---------------------------------------------------------------------------
// Diode
// ---------------------------------------------------------------------------

/// Diode operating point: junction current/conductance and stored charge /
/// incremental capacitance (depletion + diffusion).
#[derive(Clone, Copy, Debug, Default)]
pub struct DiodeOp {
    pub id: f64,
    pub gd: f64,
    pub q: f64,
    pub c: f64,
}

/// SPICE2 diode: `Id = IS·(exp(Vd/(N·Vt)) − 1)` with the smooth reverse
/// region and optional breakdown used by SPICE3/ngspice, depletion charge
/// with the FC linearisation, and transit-time (diffusion) charge `TT·Id`.
pub fn diode_eval(m: &DiodeModel, area: f64, vd: f64) -> DiodeOp {
    let vte = m.n * vt();
    let is = m.is * area;
    let (id, gd) = if vd >= -3.0 * vte {
        let e = (vd / vte).min(80.0).exp();
        (is * (e - 1.0), is * e / vte)
    } else if !m.bv.is_finite() || vd >= -m.bv {
        let a = 3.0 * vte / (vd * std::f64::consts::E);
        let a3 = a * a * a;
        (-is * (1.0 + a3), is * 3.0 * a3 / vd)
    } else {
        let e = (-(m.bv + vd) / vte).min(80.0).exp();
        (-is * e, is * e / vte)
    };
    let cjo = m.cjo * area;
    let (mut q, mut c) = (0.0, 0.0);
    if cjo > 0.0 {
        let (vj, mj, fc) = (m.vj, m.m, m.fc);
        if vd < fc * vj {
            let arg = 1.0 - vd / vj;
            let sarg = arg.powf(-mj);
            q = vj * cjo * (1.0 - arg * sarg) / (1.0 - mj);
            c = cjo * sarg;
        } else {
            let f1 = vj * (1.0 - (1.0 - fc).powf(1.0 - mj)) / (1.0 - mj);
            let f2 = (1.0 - fc).powf(1.0 + mj);
            let f3 = 1.0 - fc * (1.0 + mj);
            let czof2 = cjo / f2;
            let fcpb = fc * vj;
            q = cjo * f1 + czof2 * (f3 * (vd - fcpb) + (mj / (2.0 * vj)) * (vd * vd - fcpb * fcpb));
            c = czof2 * (f3 + mj * vd / vj);
        }
    }
    if m.tt > 0.0 {
        q += m.tt * id;
        c += m.tt * gd;
    }
    DiodeOp { id, gd, q, c }
}

/// Critical voltage above which `pnjlim` starts limiting.
pub fn vcrit(m: &DiodeModel, area: f64) -> f64 {
    let vte = m.n * vt();
    vte * (vte / (std::f64::consts::SQRT_2 * m.is * area)).ln()
}

/// SPICE `pnjlim`: limit the change of a pn-junction voltage between Newton
/// iterations so `exp()` cannot overflow and the iteration stays on the
/// convex side of the curve (Nagel 1975, §4.4). Returns (limited voltage,
/// whether limiting happened).
pub fn pnjlim(vnew: f64, vold: f64, vte: f64, vcrit: f64) -> (f64, bool) {
    if vnew > vcrit && (vnew - vold).abs() > 2.0 * vte {
        if vold > 0.0 {
            let arg = 1.0 + (vnew - vold) / vte;
            if arg > 0.0 {
                (vold + vte * arg.ln(), true)
            } else {
                (vcrit, true)
            }
        } else {
            (vte * (vnew / vte).ln(), true)
        }
    } else {
        (vnew, false)
    }
}

// ---------------------------------------------------------------------------
// MOSFET (level 1)
// ---------------------------------------------------------------------------

/// Drain current (into the drain) and its partial derivatives with respect to
/// the four terminal voltages `[vd, vg, vs, vb]`.
#[derive(Clone, Copy, Debug, Default)]
pub struct MosOp {
    pub id: f64,
    pub g: [f64; 4],
    /// Region for display: 0 cutoff, 1 triode, 2 saturation.
    pub region: u8,
    /// Small-signal parameters in the normal orientation.
    pub gm: f64,
    pub gds: f64,
    pub vth: f64,
}

/// Level-1 Shichman–Hodges drain current for an NMOS-normalised device
/// (vds ≥ 0). Returns (id, gm, gds, gmbs, vth, region).
fn level1(m: &MosModel, beta: f64, vgs: f64, vds: f64, vbs: f64, vto: f64) -> (f64, f64, f64, f64, f64, u8) {
    let (vth, dvth_dvbs) = if m.gamma > 0.0 {
        let phi = m.phi.max(0.1);
        let arg = (phi - vbs).max(0.01);
        let s = arg.sqrt();
        (vto + m.gamma * (s - phi.sqrt()), -m.gamma / (2.0 * s))
    } else {
        (vto, 0.0)
    };
    let vgst = vgs - vth;
    let clm = 1.0 + m.lambda * vds;
    if vgst <= 0.0 {
        return (0.0, 0.0, 0.0, 0.0, vth, 0);
    }
    let (id, gm, gds, region) = if vds < vgst {
        let id = beta * vds * (vgst - 0.5 * vds) * clm;
        let gm = beta * vds * clm;
        let gds = beta * (vgst - vds) * clm + beta * m.lambda * vds * (vgst - 0.5 * vds);
        (id, gm, gds, 1)
    } else {
        let id = 0.5 * beta * vgst * vgst * clm;
        let gm = beta * vgst * clm;
        let gds = 0.5 * beta * vgst * vgst * m.lambda;
        (id, gm, gds, 2)
    };
    let gmbs = -gm * dvth_dvbs;
    (id, gm, gds, gmbs, vth, region)
}

/// Evaluate a level-1 MOSFET at terminal voltages (vd, vg, vs, vb). Handles
/// PMOS by sign normalisation and source/drain interchange when vds < 0.
pub fn mos_eval(m: &MosModel, w: f64, l: f64, mult: f64, v: [f64; 4]) -> MosOp {
    let p = if m.pmos { -1.0 } else { 1.0 };
    let beta = m.kp * w / l * mult;
    let vto = p * m.vto; // NMOS-normalised threshold (positive for enhancement)
    let [vd, vg, vs, vb] = v.map(|x| p * x);
    let mut op = MosOp::default();
    if vd >= vs {
        let (id, gm, gds, gmbs, vth, region) = level1(m, beta, vg - vs, vd - vs, vb - vs, vto);
        // d/d[vd, vg, vs, vb] of the normalised drain current
        op.id = p * id;
        op.g = [gds, gm, -(gm + gds + gmbs), gmbs];
        op.gm = gm;
        op.gds = gds;
        op.vth = p * vth;
        op.region = region;
    } else {
        // reversed: the physical drain acts as the source
        let (id, gm, gds, gmbs, vth, region) = level1(m, beta, vg - vd, vs - vd, vb - vd, vto);
        op.id = -p * id;
        op.g = [gm + gds + gmbs, -gm, -gds, -gmbs];
        op.gm = gm;
        op.gds = gds;
        op.vth = p * vth;
        op.region = region;
    }
    op
}

/// Simplified SPICE `fetlim`: limit the per-iteration change of a gate
/// voltage so a square-law device cannot jump across the whole curve.
pub fn fetlim(vnew: f64, vold: f64, vto: f64) -> (f64, bool) {
    let span = 2.0 * (vold - vto).abs() + 2.0;
    let d = vnew - vold;
    if d.abs() > span {
        (vold + span * d.signum(), true)
    } else {
        (vnew, false)
    }
}

/// Limit the per-iteration change of a drain–source voltage.
pub fn limvds(vnew: f64, vold: f64) -> (f64, bool) {
    if vold >= 3.5 {
        if vnew > vold {
            (vnew.min(3.0 * vold + 2.0), vnew > 3.0 * vold + 2.0)
        } else if vnew < 3.5 {
            (vnew.max(2.0), vnew < 2.0)
        } else {
            (vnew, false)
        }
    } else if vnew > vold {
        (vnew.min(4.0), vnew > 4.0)
    } else {
        (vnew.max(-0.5), vnew < -0.5)
    }
}

// ---------------------------------------------------------------------------
// Switch
// ---------------------------------------------------------------------------

/// Next switch state given the control voltage and the previous state
/// (hysteresis band `VT ± VH`, like ngspice's SW model).
pub fn switch_state(m: &SwModel, vc: f64, prev_on: bool) -> bool {
    if vc > m.vt + m.vh {
        true
    } else if vc < m.vt - m.vh {
        false
    } else {
        prev_on
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_waveform() {
        let w = resolve_wave(
            &Wave::Pulse {
                v1: 0.0,
                v2: 1.0,
                td: 1.0,
                tr: Some(1.0),
                tf: Some(1.0),
                pw: Some(2.0),
                per: Some(10.0),
            },
            0.1,
            100.0,
        );
        assert_eq!(wave_value(&w, 0.0, 0.5), 0.0);
        assert!((wave_value(&w, 0.0, 1.5) - 0.5).abs() < 1e-12);
        assert_eq!(wave_value(&w, 0.0, 3.0), 1.0);
        assert!((wave_value(&w, 0.0, 4.5) - 0.5).abs() < 1e-12);
        assert_eq!(wave_value(&w, 0.0, 6.0), 0.0);
        assert!((wave_value(&w, 0.0, 11.5) - 0.5).abs() < 1e-12);
        let mut bp = vec![];
        breakpoints(&w, 25.0, &mut bp, 1000);
        assert!(bp.contains(&2.0) && bp.contains(&4.0) && bp.contains(&15.0));
    }

    #[test]
    fn diode_forward_drop() {
        let m = DiodeModel::default();
        // Vd = Vt·ln(I/IS): at 1 mA with IS = 1e-14 → ≈ 0.655 V
        let target = 1e-3;
        let v = vt() * (target / m.is).ln();
        let op = diode_eval(&m, 1.0, v);
        assert!((op.id - target).abs() / target < 1e-6);
        assert!(v > 0.6 && v < 0.7);
    }

    #[test]
    fn mos_square_law_and_symmetry() {
        let m = MosModel {
            vto: 0.5,
            kp: 100e-6,
            ..Default::default()
        };
        let op = mos_eval(&m, 10e-6, 1e-6, 1.0, [2.0, 1.5, 0.0, 0.0]);
        // saturation: Id = β/2 (Vgs−Vt)² = 1e-3/2 · 1 = 0.5 mA
        assert!((op.id - 0.5e-3).abs() < 1e-12);
        assert_eq!(op.region, 2);
        let rev = mos_eval(&m, 10e-6, 1e-6, 1.0, [0.0, 1.5, 2.0, 2.0]);
        assert!(rev.id < 0.0);
        // PMOS mirror
        let pm = MosModel {
            pmos: true,
            vto: -0.5,
            kp: 100e-6,
            ..Default::default()
        };
        let pop = mos_eval(&pm, 10e-6, 1e-6, 1.0, [-2.0, -1.5, 0.0, 0.0]);
        assert!((pop.id + 0.5e-3).abs() < 1e-12);
        // numerical Jacobian check
        let v = [0.3, 1.2, 0.05, 0.0];
        let base = mos_eval(&m, 10e-6, 1e-6, 1.0, v);
        for k in 0..4 {
            let mut v2 = v;
            v2[k] += 1e-7;
            let d = (mos_eval(&m, 10e-6, 1e-6, 1.0, v2).id - base.id) / 1e-7;
            assert!((d - base.g[k]).abs() < 1e-6, "k={k} {d} {}", base.g[k]);
        }
    }
}
