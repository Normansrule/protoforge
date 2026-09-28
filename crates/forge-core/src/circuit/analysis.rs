//! Analysis drivers: operating point (with gmin- and source-stepping
//! fallbacks), DC sweep, transient (trapezoidal/backward-Euler companion
//! models, breakpoints, local-truncation-error step control) and AC
//! small-signal analysis.
//!
//! References: L. W. Nagel, SPICE2 (ERL-M520, 1975) ch. 4–5; J. Vlach and
//! K. Singhal, *Computer Methods for Circuit Analysis and Design* ch. 9–13;
//! ngspice User's Manual ch. 11.

use serde::Serialize;

use crate::math::{solve, C64};

use super::devices::{breakpoints, diode_eval, mos_eval, resolve_wave};
use super::mna::{hist, newton, vx, CDev, Ckt, Dyn, ElState, IterMem, Load, Mode, NrErr, Sys, Work};
use super::parser::{AcSweep, Device, Netlist};

/// Hard limits that keep a browser tab responsive.
pub const MAX_TRAN_POINTS: usize = 250_000;
pub const MAX_LU: usize = 2_000_000;

#[derive(Clone, Debug, Serialize, Default)]
pub struct OpResult {
    /// Voltage of every user node (index matches `SimResult::nodes`).
    pub node_v: Vec<f64>,
    /// Current through every element (SPICE convention, into n+ / drain).
    pub elem_i: Vec<f64>,
    /// Voltage across every element (v(n+) − v(n−); MOSFET: vds).
    pub elem_v: Vec<f64>,
    /// Power absorbed by every element (negative = delivering).
    pub elem_p: Vec<f64>,
    /// Human-readable device state (region, small-signal parameters).
    pub elem_note: Vec<String>,
    pub iterations: usize,
    /// "newton", "gmin stepping" or "source stepping".
    pub method: String,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct TranStats {
    pub accepted: usize,
    pub rejected_lte: usize,
    pub rejected_nr: usize,
    pub nr_iterations: usize,
    pub breakpoints: usize,
    pub min_step: f64,
    pub max_step: f64,
    pub uic: bool,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct TranResult {
    #[serde(serialize_with = "super::enc::f64s")]
    pub time: Vec<f64>,
    /// `v[node][k]`
    #[serde(serialize_with = "super::enc::f32s")]
    pub v: Vec<Vec<f32>>,
    /// `i[element][k]`
    #[serde(serialize_with = "super::enc::f32s")]
    pub i: Vec<Vec<f32>>,
    pub tstep: f64,
    pub tstop: f64,
    pub stats: TranStats,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct AcResult {
    #[serde(serialize_with = "super::enc::f64s")]
    pub freq: Vec<f64>,
    /// Magnitude (linear) of every node voltage, `mag[node][k]`.
    #[serde(serialize_with = "super::enc::f32s")]
    pub mag: Vec<Vec<f32>>,
    /// Phase in degrees, unwrapped along frequency.
    #[serde(serialize_with = "super::enc::f32s")]
    pub phase: Vec<Vec<f32>>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct DcResult {
    pub source: String,
    #[serde(serialize_with = "super::enc::f64s")]
    pub sweep: Vec<f64>,
    #[serde(serialize_with = "super::enc::f32s")]
    pub v: Vec<Vec<f32>>,
    #[serde(serialize_with = "super::enc::f32s")]
    pub i: Vec<Vec<f32>>,
}

/// Solver bookkeeping shared by all analyses.
#[derive(Clone, Debug, Serialize, Default)]
pub struct SolverStats {
    pub unknowns: usize,
    pub nodes: usize,
    pub branches: usize,
    pub lu_solves: usize,
    pub nr_iterations: usize,
}

pub struct OpSolution {
    pub x: Vec<f64>,
    pub mem: Vec<IterMem>,
    pub iters: usize,
    pub method: String,
    pub gshunt: f64,
}

fn zero_states(n: usize) -> Vec<ElState> {
    vec![ElState::default(); n]
}

/// DC operating point with the classic SPICE convergence aids.
pub fn operating_point(ckt: &Ckt, mode: Mode, stats: &mut SolverStats, warnings: &mut Vec<String>) -> Result<OpSolution, String> {
    let st = zero_states(ckt.el.len());
    let mut x0 = vec![0.0; ckt.size];
    for (n, v) in &ckt.nodesets {
        x0[n - 1] = *v;
    }
    let maxit = ckt.opts.itl1;
    let mut ws = Work::new(ckt.size);
    let mut attempt = |gshunt: f64, srcfac: f64, x0: &[f64], mem: &mut Vec<IterMem>, stats: &mut SolverStats| -> Result<(Vec<f64>, usize), NrErr> {
        let ld = Load { mode, st: &st, srcfac, gshunt };
        let before = ws.solves;
        let r = newton(ckt, x0, &ld, mem, maxit, &mut ws);
        stats.lu_solves += ws.solves - before;
        let r = r?;
        stats.nr_iterations += r.iters;
        Ok((r.x, r.iters))
    };

    // 1. plain Newton–Raphson
    let mut singular = false;
    for gshunt in [0.0, 1e-12] {
        let mut mem = vec![IterMem::default(); ckt.el.len()];
        match attempt(gshunt, 1.0, &x0, &mut mem, stats) {
            Ok((x, iters)) => {
                if gshunt > 0.0 {
                    warnings.push("matrix was singular (a node has no DC path to ground, e.g. only capacitors); added 1 pS shunts to every node — ngspice will report \"singular matrix\" for this deck".into());
                }
                return Ok(OpSolution { x, mem, iters, method: "newton".into(), gshunt });
            }
            Err(NrErr::Singular) => singular = true,
            Err(NrErr::NoConvergence) => break,
        }
    }
    let gbase = if singular { 1e-12 } else { 0.0 };

    // 2. gmin stepping: shunt every node with a large conductance and relax it
    {
        let mut mem = vec![IterMem::default(); ckt.el.len()];
        let mut x = x0.clone();
        let mut ok = true;
        let mut total = 0;
        let mut g = 1e-2;
        while g > 1e-13 {
            match attempt(g + gbase, 1.0, &x, &mut mem, stats) {
                Ok((xn, it)) => {
                    x = xn;
                    total += it;
                }
                Err(_) => {
                    ok = false;
                    break;
                }
            }
            g /= 10.0;
        }
        if ok {
            if let Ok((xn, it)) = attempt(gbase, 1.0, &x, &mut mem, stats) {
                return Ok(OpSolution { x: xn, mem, iters: total + it, method: "gmin stepping".into(), gshunt: gbase });
            }
        }
    }

    // 3. source stepping: ramp every independent source from 0 to 100 %
    {
        let mut mem = vec![IterMem::default(); ckt.el.len()];
        let mut x = vec![0.0; ckt.size];
        let mut total = 0;
        let mut f: f64 = 0.0;
        let mut df: f64 = 0.1;
        let mut guard = 0;
        while f < 1.0 && guard < 400 {
            guard += 1;
            let fnext = (f + df).min(1.0);
            let mut trial_mem = mem.clone();
            match attempt(gbase, fnext, &x, &mut trial_mem, stats) {
                Ok((xn, it)) => {
                    x = xn;
                    mem = trial_mem;
                    total += it;
                    f = fnext;
                    df = (df * 1.5).min(0.25);
                }
                Err(_) => {
                    df /= 4.0;
                    if df < 1e-5 {
                        break;
                    }
                }
            }
        }
        if f >= 1.0 {
            return Ok(OpSolution { x, mem, iters: total, method: "source stepping".into(), gshunt: gbase });
        }
    }
    Err(if singular {
        "singular matrix: check for floating nodes, loops of voltage sources/inductors, or current sources in series with capacitors".into()
    } else {
        "operating point did not converge (Newton, gmin stepping and source stepping all failed)".into()
    })
}

fn fmt_si(v: f64, unit: &str) -> String {
    super::fmt_si(v, unit)
}

/// Build the operating-point table from a solution.
pub fn op_result(ckt: &Ckt, nl: &Netlist, sol: &OpSolution, mode: Mode) -> OpResult {
    let st = zero_states(ckt.el.len());
    let ld = Load { mode, st: &st, srcfac: 1.0, gshunt: sol.gshunt };
    let x = &sol.x;
    let i = ckt.element_currents(x, &ld, &sol.mem);
    let node_v: Vec<f64> = (1..ckt.n_user).map(|n| vx(x, n)).collect();
    let mut ev = Vec::new();
    let mut ep = Vec::new();
    let mut notes = Vec::new();
    for (k, e) in nl.elements.iter().enumerate() {
        let n = &e.nodes;
        let v = vx(x, n[0]) - vx(x, n[1]);
        let (vv, p, note) = match &ckt.el[k] {
            CDev::M { d, g, s, b, m, w, l, mult, .. } => {
                let op = mos_eval(m, *w, *l, *mult, [vx(x, *d), vx(x, *g), vx(x, *s), vx(x, *b)]);
                let region = ["cutoff", "triode", "saturation"][op.region as usize];
                let vgs = vx(x, *g) - vx(x, *s);
                (v, v * i[k], format!("{region} · Vgs {} · Vth {} · gm {} · gds {}", fmt_si(vgs, "V"), fmt_si(op.vth, "V"), fmt_si(op.gm, "S"), fmt_si(op.gds, "S")))
            }
            CDev::D { ai, k: kk, m, area, .. } => {
                let vd = vx(x, *ai) - vx(x, *kk);
                let op = diode_eval(m, *area, vd);
                let state = if vd > 0.3 { "forward" } else if vd < -0.1 { "reverse" } else { "off" };
                (v, v * i[k], format!("{state} · Vd {} · rd {}", fmt_si(vd, "V"), fmt_si(1.0 / op.gd.max(1e-30), "Ω")))
            }
            CDev::S { .. } => (v, v * i[k], if sol.mem[k].on { "on".into() } else { "off".into() }),
            CDev::E { .. } | CDev::G { .. } => (v, v * i[k], String::new()),
            _ => (v, v * i[k], String::new()),
        };
        ev.push(vv);
        ep.push(p);
        notes.push(note);
    }
    OpResult { node_v, elem_i: i, elem_v: ev, elem_p: ep, elem_note: notes, iterations: sol.iters, method: sol.method.clone() }
}

/// DC sweep of one independent source.
#[allow(clippy::too_many_arguments)]
pub fn dc_sweep(ckt: &Ckt, nl: &Netlist, src: &str, start: f64, stop: f64, step: f64, stats: &mut SolverStats, warnings: &mut Vec<String>) -> Result<DcResult, String> {
    let idx = nl.element_index(src).ok_or_else(|| format!(".dc: unknown source `{src}`"))?;
    let n = (((stop - start) / step) + 1e-9).floor() as usize + 1;
    let mut c = ckt.clone();
    let mut sweep = Vec::with_capacity(n);
    let mut v = vec![Vec::with_capacity(n); ckt.n_user - 1];
    let mut i = vec![Vec::with_capacity(n); ckt.el.len()];
    let st = zero_states(ckt.el.len());
    let mut prev: Option<(Vec<f64>, Vec<IterMem>, f64)> = None;
    let mut ws = Work::new(ckt.size);
    let mode = Mode::Dc { t: None, uic_init: false, force_ic: false };
    for k in 0..n {
        let val = start + step * k as f64;
        match &mut c.el[idx] {
            CDev::V { src, .. } | CDev::I { src, .. } => src.dc = val,
            _ => return Err(format!(".dc: `{src}` is not a source")),
        }
        let sol = match &prev {
            Some((x0, mem0, gshunt)) => {
                let mut mem = mem0.clone();
                let ld = Load { mode, st: &st, srcfac: 1.0, gshunt: *gshunt };
                let before = ws.solves;
                let res = newton(&c, x0, &ld, &mut mem, c.opts.itl1, &mut ws);
                stats.lu_solves += ws.solves - before;
                match res {
                    Ok(r) => {
                        stats.nr_iterations += r.iters;
                        Some((r.x, mem, *gshunt))
                    }
                    Err(_) => None,
                }
            }
            None => None,
        };
        let sol = match sol {
            Some(s) => s,
            None => {
                let o = operating_point(&c, mode, stats, warnings).map_err(|e| format!(".dc at {src} = {val}: {e}"))?;
                (o.x, o.mem, o.gshunt)
            }
        };
        let ld = Load { mode, st: &st, srcfac: 1.0, gshunt: sol.2 };
        let cur = c.element_currents(&sol.0, &ld, &sol.1);
        for (nidx, col) in v.iter_mut().enumerate() {
            col.push(vx(&sol.0, nidx + 1) as f32);
        }
        for (e, col) in i.iter_mut().enumerate() {
            col.push(cur[e] as f32);
        }
        sweep.push(val);
        prev = Some(sol);
    }
    Ok(DcResult { source: nl.elements[idx].name.clone(), sweep, v, i })
}

/// AC frequency grid (ngspice conventions).
pub fn ac_freqs(sweep: AcSweep, n: usize, f0: f64, f1: f64) -> Vec<f64> {
    match sweep {
        AcSweep::Lin => {
            if n <= 1 {
                vec![f0]
            } else {
                (0..n).map(|k| f0 + (f1 - f0) * k as f64 / (n - 1) as f64).collect()
            }
        }
        AcSweep::Dec | AcSweep::Oct => {
            let base: f64 = if sweep == AcSweep::Dec { 10.0 } else { 2.0 };
            let ratio = base.powf(1.0 / n as f64);
            let count = ((f1 / f0).ln() / ratio.ln() + 1e-9).floor() as usize + 1;
            (0..count.min(100_000)).map(|k| f0 * ratio.powi(k as i32)).collect()
        }
    }
}

/// AC small-signal analysis around the DC operating point.
pub fn ac(ckt: &Ckt, op: &OpSolution, freqs: &[f64], stats: &mut SolverStats) -> Result<AcResult, String> {
    let nu = ckt.n_user - 1;
    let mut mag = vec![Vec::with_capacity(freqs.len()); nu];
    let mut phase = vec![Vec::with_capacity(freqs.len()); nu];
    let mut last_ph = vec![0.0f64; nu];
    for (fi, f) in freqs.iter().enumerate() {
        let w = 2.0 * std::f64::consts::PI * f;
        let mut sys = Sys::<C64>::new(ckt.size);
        ckt.load_ac(&op.x, &op.mem, w, &mut sys);
        if op.gshunt > 0.0 {
            for i in 0..ckt.nn - 1 {
                sys.diag(i, C64::real(op.gshunt));
            }
        }
        stats.lu_solves += 1;
        let x = solve(sys.rows(), sys.b).ok_or_else(|| format!("AC matrix singular at {f} Hz"))?;
        for n in 0..nu {
            let z = x[n];
            mag[n].push(z.abs() as f32);
            let mut ph = z.phase_deg();
            if fi > 0 {
                while ph - last_ph[n] > 180.0 {
                    ph -= 360.0;
                }
                while ph - last_ph[n] < -180.0 {
                    ph += 360.0;
                }
            }
            last_ph[n] = ph;
            phase[n].push(ph as f32);
        }
    }
    Ok(AcResult { freq: freqs.to_vec(), mag, phase })
}

/// Transient analysis.
#[allow(clippy::too_many_arguments)]
pub fn transient(ckt0: &Ckt, _nl: &Netlist, tstep: f64, tstop: f64, tstart: f64, tmax_in: Option<f64>, uic: bool, stats: &mut SolverStats, warnings: &mut Vec<String>) -> Result<TranResult, String> {
    // resolve source defaults for this run and collect breakpoints
    let mut ckt = ckt0.clone();
    let mut bps: Vec<f64> = Vec::new();
    for d in ckt.el.iter_mut() {
        if let CDev::V { src, .. } | CDev::I { src, .. } = d {
            src.wave = resolve_wave(&src.wave, tstep, tstop);
            breakpoints(&src.wave, tstop, &mut bps, 200_000);
        }
    }
    bps.push(tstop);
    bps.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let eps_t = tstop * 1e-13;
    bps.dedup_by(|a, b| (*a - *b).abs() <= eps_t);
    let n_bps = bps.len();

    let tmax = tmax_in.unwrap_or_else(|| tstep.min((tstop - tstart) / 50.0)).min(tstop / 10.0);
    let hmin = (tmax * 1e-9).max(1e-18);
    let o = ckt.opts.clone();
    let nel = ckt.el.len();
    let nv = ckt.nn - 1;

    // ---- initial point ----
    let (mut x, mut mem, gshunt) = if uic {
        let st = zero_states(nel);
        let mode = Mode::Dc { t: Some(0.0), uic_init: true, force_ic: true };
        let mut mem = vec![IterMem::default(); nel];
        let ld = Load { mode, st: &st, srcfac: 1.0, gshunt: 1e-12 };
        let mut ws0 = Work::new(ckt.size);
        let r0 = newton(&ckt, &vec![0.0; ckt.size], &ld, &mut mem, o.itl1, &mut ws0);
        stats.lu_solves += ws0.solves;
        match r0 {
            Ok(r) => (r.x, mem, 0.0),
            Err(_) => {
                warnings.push("UIC: could not compute a consistent initial point; starting from 0 V".into());
                (vec![0.0; ckt.size], vec![IterMem::default(); nel], 0.0)
            }
        }
    } else {
        let sol = operating_point(&ckt, Mode::Dc { t: Some(0.0), uic_init: false, force_ic: true }, stats, warnings).map_err(|e| format!("transient initial operating point: {e}"))?;
        (sol.x, sol.mem, sol.gshunt)
    };
    // initial reactive states
    let mut st = zero_states(nel);
    let q0 = ckt.charges(&x);
    for (k, d) in ckt.el.iter().enumerate() {
        st[k].dy = [Dyn { q: q0[k][0], i: 0.0 }, Dyn { q: q0[k][1], i: 0.0 }, Dyn { q: q0[k][2], i: 0.0 }, Dyn { q: q0[k][3], i: 0.0 }];
        match d {
            CDev::C { a, b, c, ic } if uic => st[k].dy[0].q = c * ckt.uic_cap_v(*a, *b, *ic),
            CDev::L { l, ic, .. } if uic => st[k].dy[0].q = l * ic.unwrap_or(0.0),
            CDev::S { init_on, .. } => st[k].on = if uic { mem[k].init && mem[k].on || (!mem[k].init && *init_on) } else { mem[k].on },
            _ => {}
        }
    }

    let nu = ckt.n_user - 1;
    let mut out = TranResult { tstep, tstop, ..Default::default() };
    out.v = vec![Vec::new(); nu];
    out.i = vec![Vec::new(); nel];
    out.stats.uic = uic;
    out.stats.breakpoints = n_bps;
    out.stats.min_step = f64::INFINITY;
    let record = |out: &mut TranResult, t: f64, x: &[f64], cur: &[f64]| {
        out.time.push(t);
        for (col, xv) in out.v.iter_mut().zip(&x[..nu]) {
            col.push(*xv as f32);
        }
        for (e, c) in cur.iter().enumerate() {
            out.i[e].push(*c as f32);
        }
    };
    {
        let mode = Mode::Dc { t: Some(0.0), uic_init: false, force_ic: false };
        let ld = Load { mode, st: &st, srcfac: 1.0, gshunt };
        let mut cur = ckt.element_currents(&x, &ld, &mem);
        if uic {
            for (k, d) in ckt.el.iter().enumerate() {
                if let CDev::L { ic, .. } = d {
                    cur[k] = ic.unwrap_or(0.0);
                }
            }
        }
        if tstart <= 0.0 {
            record(&mut out, 0.0, &x, &cur);
        }
    }

    let mut t = 0.0;
    let mut bpi = 0;
    let mut h = (tstep.min(tstop / 100.0) / 10.0).min(tmax);
    // step size in smooth regions (restored gradually after breakpoints)
    let mut h_smooth = tmax;
    let mut first_order = true;
    // history of accepted solutions since the last breakpoint: (t, x)
    let mut histq: Vec<(f64, Vec<f64>)> = vec![(0.0, x.clone())];
    let mut lu_guard = 0usize;
    let mut ws = Work::new(ckt.size);
    let mut last_rejected = false;
    while t < tstop - eps_t {
        while bpi < bps.len() && bps[bpi] <= t + eps_t {
            bpi += 1;
        }
        let next_bp = bps.get(bpi).copied().unwrap_or(tstop);
        h = h.min(tmax).max(hmin);
        let mut hit_bp = false;
        if t + h >= next_bp - hmin {
            h = next_bp - t;
            hit_bp = true;
        } else if t + 1.8 * h > next_bp {
            // avoid a sliver step before the breakpoint
            h = 0.5 * (next_bp - t);
        }
        let trap = !first_order;
        let a0 = if trap { 2.0 / h } else { 1.0 / h };
        let tn = t + h;
        let mode = Mode::Tran { t: tn, a0, trap };
        let mut trial_mem = mem.clone();
        for m in trial_mem.iter_mut() {
            m.flips = 0;
        }
        let ld = Load { mode, st: &st, srcfac: 1.0, gshunt };
        // predictor for the Newton initial guess: linear extrapolation
        let guess: Vec<f64> = if histq.len() >= 2 {
            let (t1, x1) = &histq[histq.len() - 2];
            let (t2, x2) = &histq[histq.len() - 1];
            let r = (tn - t2) / (t2 - t1);
            x2.iter().zip(x1).map(|(b, a)| b + (b - a) * r).collect()
        } else {
            x.clone()
        };
        let before = ws.solves;
        let res = newton(&ckt, &guess, &ld, &mut trial_mem, o.itl4, &mut ws);
        lu_guard += ws.solves - before;
        stats.lu_solves += ws.solves - before;
        if lu_guard > MAX_LU {
            return Err(format!("transient aborted at t = {}: too much work (more than {MAX_LU} matrix solves) — increase tstep or shorten tstop", fmt_si(t, "s")));
        }
        let xn = match res {
            Ok(r) => {
                out.stats.nr_iterations += r.iters;
                stats.nr_iterations += r.iters;
                r.x
            }
            Err(e) => {
                out.stats.rejected_nr += 1;
                h /= 8.0;
                if h < hmin {
                    return Err(format!(
                        "timestep too small at t = {} ({}) — the circuit may have a discontinuity the solver cannot resolve",
                        fmt_si(t, "s"),
                        match e {
                            NrErr::Singular => "singular matrix",
                            NrErr::NoConvergence => "Newton did not converge",
                        }
                    ));
                }
                first_order = true;
                continue;
            }
        };
        // switch events act like breakpoints
        let switched = ckt.el.iter().enumerate().any(|(k, d)| matches!(d, CDev::S { .. }) && trial_mem[k].on != st[k].on);

        // local truncation error estimate (trapezoidal, predictor–corrector)
        let mut hfactor = 2.0;
        let mut lte_done = false;
        if trap && histq.len() >= 3 && !switched {
            lte_done = true;
            let n3 = histq.len();
            let (t0, x0) = (&histq[n3 - 3].0, &histq[n3 - 3].1);
            let (t1, x1) = (&histq[n3 - 2].0, &histq[n3 - 2].1);
            let (t2, x2) = (&histq[n3 - 1].0, &histq[n3 - 1].1);
            // quadratic Lagrange extrapolation to tn
            let l0 = (tn - t1) * (tn - t2) / ((t0 - t1) * (t0 - t2));
            let l1 = (tn - t0) * (tn - t2) / ((t1 - t0) * (t1 - t2));
            let l2 = (tn - t0) * (tn - t1) / ((t2 - t0) * (t2 - t1));
            let mut ratio: f64 = 0.0;
            for i in 0..nv {
                let xp = l0 * x0[i] + l1 * x1[i] + l2 * x2[i];
                let lte = (xn[i] - xp).abs() / 13.0;
                // voltage-based analogue of SPICE's charge LTE test; trtol (7)
                // relaxes it the way SPICE does, scaled for voltages
                let tol = (o.trtol / 3.5) * (o.reltol * xn[i].abs().max(xp.abs()) + o.vntol * 10.0);
                ratio = ratio.max(lte / tol);
            }
            if ratio > 1.0 && h > 4.0 * hmin {
                out.stats.rejected_lte += 1;
                h *= (0.9 * ratio.powf(-1.0 / 3.0)).clamp(0.2, 0.9);
                last_rejected = true;
                continue;
            }
            hfactor = (0.9 * ratio.max(1e-12).powf(-1.0 / 3.0)).clamp(0.3, if last_rejected { 1.0 } else { 2.0 });
        }

        // ---- accept ----
        last_rejected = false;
        let cur = ckt.element_currents(&xn, &ld, &trial_mem);
        let q = ckt.charges(&xn);
        for (k, d) in ckt.el.iter().enumerate() {
            match d {
                CDev::C { .. } | CDev::L { .. } | CDev::D { .. } | CDev::M { .. } => {
                    for (dy, qn) in st[k].dy.iter_mut().zip(q[k]) {
                        let hh = hist(dy, a0, trap);
                        *dy = Dyn { q: qn, i: a0 * qn - hh };
                    }
                }
                CDev::S { .. } => st[k].on = trial_mem[k].on,
                _ => {}
            }
        }
        mem = trial_mem;
        out.stats.accepted += 1;
        out.stats.min_step = out.stats.min_step.min(h);
        out.stats.max_step = out.stats.max_step.max(h);
        t = tn;
        x = xn;
        if t >= tstart - eps_t {
            record(&mut out, t, &x, &cur);
        }
        if out.time.len() > MAX_TRAN_POINTS {
            return Err(format!("transient aborted: more than {MAX_TRAN_POINTS} time points — increase tstep/tmax or shorten tstop"));
        }
        if hit_bp || switched {
            histq.clear();
            histq.push((t, x.clone()));
            first_order = true;
            // restart small (like SPICE: 0.1 × the step before the corner)
            let nb = bps.iter().find(|b| **b > t + eps_t).copied().unwrap_or(tstop);
            h = (0.1 * h_smooth.min(nb - t)).max(hmin * 10.0);
        } else {
            histq.push((t, x.clone()));
            if histq.len() > 3 {
                histq.remove(0);
            }
            first_order = false;
            h *= hfactor;
            if lte_done {
                h_smooth = h.min(tmax);
            }
        }
    }
    if out.stats.min_step == f64::INFINITY {
        out.stats.min_step = 0.0;
    }
    Ok(out)
}

/// Which elements are reactive (for UI hints).
pub fn is_reactive(d: &Device) -> bool {
    matches!(d, Device::Capacitor { .. } | Device::Inductor { .. })
}
