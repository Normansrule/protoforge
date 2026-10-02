//! Impedance matching: Smith-chart maths, lumped L-networks (both
//! solutions, Pozar §5.1), pi-networks set by a loaded Q (Bowick,
//! *RF Circuit Design*, ch. 4), snapping to E-series standard values with
//! finite component Q, band sweeps and Touchstone export.
//!
//! **Reflection coefficient.** Γ = (Z − Z0)/(Z + Z0); return loss
//! RL = −20·log10|Γ|; VSWR = (1 + |Γ|)/(1 − |Γ|); mismatch loss
//! = −10·log10(1 − |Γ|²).
//!
//! **L-network** (Pozar eqs. 5.3–5.6). For a load z_L = R_L + jX_L and line Z0:
//!
//! * R_L > Z0 (load inside the 1 + jx circle): shunt B next to the load,
//!   series X toward the source,
//!   `B = (X_L ± √(R_L/Z0)·√(R_L² + X_L² − Z0·R_L)) / (R_L² + X_L²)`,
//!   `X = 1/B + X_L·Z0/R_L − Z0/(B·R_L)`.
//! * R_L < Z0: series X next to the load, shunt B toward the source,
//!   `X = ±√(R_L(Z0 − R_L)) − X_L`, `B = ±√((Z0 − R_L)/R_L)/Z0`.
//!
//! A positive series reactance is an inductor L = X/ω, negative a capacitor
//! C = −1/(ωX); a positive shunt susceptance is a capacitor C = B/ω, negative
//! an inductor L = −1/(ωB). Real parts: inductor ESR = ωL/Q_L, capacitor
//! ESR = 1/(ωC·Q_C).
//!
//! **Loaded Q.** Each L-section transforming R_high ↔ R_low has
//! Q = √(R_high/R_low − 1); its bandwidth is roughly f0/Q. A pi-network
//! passes through a lower virtual resistance R_v = R_high/(Q² + 1) and so lets
//! the designer choose Q (≥ the L-network's) — narrower band, better
//! harmonic rejection.

use serde::{Deserialize, Serialize};

use crate::math::{linspace, C64};

pub const BLE_LO_GHZ: f64 = 2.400;
pub const BLE_HI_GHZ: f64 = 2.4835;
pub const BLE_MID_GHZ: f64 = 0.5 * (BLE_LO_GHZ + BLE_HI_GHZ);

pub fn gamma(z: C64, z0: f64) -> C64 {
    (z - C64::real(z0)) / (z + C64::real(z0))
}

pub fn z_from_gamma(g: C64, z0: f64) -> C64 {
    (C64::ONE + g) / (C64::ONE - g) * z0
}

pub fn return_loss_db(g: C64) -> f64 {
    -20.0 * g.abs().max(1e-12).log10()
}

pub fn vswr(g: C64) -> f64 {
    let a = g.abs().min(0.999_999);
    (1.0 + a) / (1.0 - a)
}

/// A detuned antenna modelled near resonance as a series R-L-C:
/// Z(f) = R·(1 + jQ·(f/f0 − f0/f)), parameterised by its impedance at the
/// centre frequency `fc` (R + jX) and its Q. **Assumed values; measure the
/// real feed impedance with a vector network analyser (VNA).**
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AntennaModel {
    pub r_ohm: f64,
    pub x_ohm: f64,
    pub q: f64,
    pub fc_ghz: f64,
}

impl Default for AntennaModel {
    fn default() -> Self {
        // A chip antenna on a small (58 × 10 mm) ground, tuned high and
        // capacitive — the typical "before matching" picture in chip-antenna
        // application notes. Assumed; measure with a VNA.
        AntennaModel {
            r_ohm: 18.0,
            x_ohm: -30.0,
            q: 12.0,
            fc_ghz: BLE_MID_GHZ,
        }
    }
}

impl AntennaModel {
    /// Series resonance frequency implied by X at fc.
    pub fn f0_ghz(&self) -> f64 {
        let x = self.x_ohm / (self.r_ohm.max(1e-6) * self.q.max(1e-6));
        // fc/f0 − f0/fc = x → r = fc/f0 = (x + √(x²+4))/2
        let r = 0.5 * (x + (x * x + 4.0).sqrt());
        self.fc_ghz / r
    }
    pub fn z(&self, f_ghz: f64) -> C64 {
        let f0 = self.f0_ghz();
        C64::new(self.r_ohm, self.r_ohm * self.q * (f_ghz / f0 - f0 / f_ghz))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    SeriesL,
    SeriesC,
    ShuntL,
    ShuntC,
}

impl Kind {
    pub fn is_l(self) -> bool {
        matches!(self, Kind::SeriesL | Kind::ShuntL)
    }
    pub fn is_series(self) -> bool {
        matches!(self, Kind::SeriesL | Kind::SeriesC)
    }
}

/// One lumped element. `value` in henries or farads. Networks are listed
/// from the source (50 Ω port) toward the load (antenna).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Element {
    pub kind: Kind,
    pub value: f64,
    /// Unloaded component Q (0 or negative = ideal).
    #[serde(default)]
    pub q: f64,
}

impl Element {
    fn z(&self, f_hz: f64) -> C64 {
        let w = 2.0 * std::f64::consts::PI * f_hz;
        let x = if self.kind.is_l() {
            w * self.value
        } else {
            -1.0 / (w * self.value.max(1e-30))
        };
        let r = if self.q > 0.0 { x.abs() / self.q } else { 0.0 };
        C64::new(r, x)
    }
    /// Human value: "3.9 nH" / "1.2 pF".
    pub fn label(&self) -> String {
        let (v, u) = if self.kind.is_l() {
            (self.value * 1e9, "nH")
        } else {
            (self.value * 1e12, "pF")
        };
        let s = if v >= 10.0 {
            format!("{v:.0}")
        } else {
            format!("{v:.2}").trim_end_matches('0').trim_end_matches('.').to_string()
        };
        format!("{} {s} {u}", if self.kind.is_series() { "series" } else { "shunt" })
    }
}

/// 2×2 ABCD matrix.
type Abcd = [C64; 4];

fn mul(a: Abcd, b: Abcd) -> Abcd {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
    ]
}

fn abcd(net: &[Element], f_hz: f64) -> Abcd {
    let mut m = [C64::ONE, C64::ZERO, C64::ZERO, C64::ONE];
    for e in net {
        let z = e.z(f_hz);
        let s = if e.kind.is_series() {
            [C64::ONE, z, C64::ZERO, C64::ONE]
        } else {
            [C64::ONE, C64::ZERO, z.inv(), C64::ONE]
        };
        m = mul(m, s);
    }
    m
}

/// Input impedance looking into the network from the source port.
pub fn z_in(net: &[Element], zl: C64, f_hz: f64) -> C64 {
    let m = abcd(net, f_hz);
    (m[0] * zl + m[1]) / (m[2] * zl + m[3])
}

/// Transducer gain (power into the load's resistance / available power) and Γ_in.
pub fn transducer(net: &[Element], zl: C64, f_hz: f64, z0: f64) -> (f64, C64) {
    let m = abcd(net, f_hz);
    // I2 = 1 A into the load: V2 = ZL, V1 = A·ZL + B, I1 = C·ZL + D, Vs = V1 + Z0·I1
    let v1 = m[0] * zl + m[1];
    let i1 = m[2] * zl + m[3];
    let vs = v1 + i1 * z0;
    let gt = 4.0 * z0 * zl.re / vs.norm_sqr();
    (gt, gamma(v1 / i1, z0))
}

fn from_x_series(x: f64, w: f64, ql: f64, qc: f64) -> Element {
    if x >= 0.0 {
        Element {
            kind: Kind::SeriesL,
            value: x / w,
            q: ql,
        }
    } else {
        Element {
            kind: Kind::SeriesC,
            value: -1.0 / (w * x),
            q: qc,
        }
    }
}

fn from_b_shunt(b: f64, w: f64, ql: f64, qc: f64) -> Element {
    if b >= 0.0 {
        Element {
            kind: Kind::ShuntC,
            value: b / w,
            q: qc,
        }
    } else {
        Element {
            kind: Kind::ShuntL,
            value: -1.0 / (w * b),
            q: ql,
        }
    }
}

/// All L-network solutions (source → load order), ideal reactances.
pub fn l_match(zl: C64, z0: f64, f_hz: f64) -> Vec<(Vec<Element>, f64, f64, &'static str)> {
    let w = 2.0 * std::f64::consts::PI * f_hz;
    let (rl, xl) = (zl.re, zl.im);
    let mut out = Vec::new();
    if rl <= 0.0 {
        return out;
    }
    // shunt at the load, series toward the source (valid when R_L² + X_L² ≥ Z0 R_L)
    let disc = rl * rl + xl * xl - z0 * rl;
    if disc >= 0.0 {
        for s in [1.0, -1.0] {
            let b = (xl + s * (rl / z0).sqrt() * disc.sqrt()) / (rl * rl + xl * xl);
            if b.abs() < 1e-15 {
                continue;
            }
            let x = 1.0 / b + xl * z0 / rl - z0 / (b * rl);
            out.push((
                vec![from_x_series(x, w, 0.0, 0.0), from_b_shunt(b, w, 0.0, 0.0)],
                x,
                b,
                "shunt-at-load",
            ));
        }
    }
    // series at the load, shunt toward the source (valid when R_L ≤ Z0)
    if rl <= z0 {
        for s in [1.0, -1.0] {
            let x = s * (rl * (z0 - rl)).sqrt() - xl;
            let b = s * ((z0 - rl) / rl).sqrt() / z0;
            out.push((
                vec![from_b_shunt(b, w, 0.0, 0.0), from_x_series(x, w, 0.0, 0.0)],
                x,
                b,
                "series-at-load",
            ));
        }
    }
    out
}

/// Low-pass pi network (shunt C – series L – shunt C) with a chosen loaded Q.
/// The load's reactance is absorbed into the load-side shunt element.
pub fn pi_match(zl: C64, z0: f64, f_hz: f64, q: f64) -> Result<(Vec<Element>, f64), String> {
    let w = 2.0 * std::f64::consts::PI * f_hz;
    let yl = zl.inv();
    if yl.re <= 0.0 {
        return Err("load has no resistance".into());
    }
    let rp = 1.0 / yl.re; // parallel-equivalent load resistance
    let rh = rp.max(z0);
    let qmin = (rh / rp.min(z0) - 1.0).max(0.0).sqrt();
    if q <= qmin + 1e-9 {
        return Err(format!("loaded Q must exceed the L-network Q of {qmin:.2}"));
    }
    let rv = rh / (q * q + 1.0);
    let qs = (z0 / rv - 1.0).sqrt();
    let ql = (rp / rv - 1.0).sqrt();
    let b1 = qs / z0; // source-side shunt C susceptance
    let b3 = ql / rp - yl.im; // load-side element, minus the load's own susceptance
    let xs = rv * (qs + ql); // series inductor
    let net = vec![
        from_b_shunt(b1, w, 0.0, 0.0),
        from_x_series(xs, w, 0.0, 0.0),
        from_b_shunt(b3, w, 0.0, 0.0),
    ];
    Ok((net, qmin))
}

/// E-series standard values (one decade).
pub fn e_series(name: &str) -> &'static [f64] {
    const E12: [f64; 12] = [1.0, 1.2, 1.5, 1.8, 2.2, 2.7, 3.3, 3.9, 4.7, 5.6, 6.8, 8.2];
    const E24: [f64; 24] = [
        1.0, 1.1, 1.2, 1.3, 1.5, 1.6, 1.8, 2.0, 2.2, 2.4, 2.7, 3.0, 3.3, 3.6, 3.9, 4.3, 4.7, 5.1, 5.6, 6.2, 6.8, 7.5, 8.2, 9.1,
    ];
    if name.eq_ignore_ascii_case("E12") {
        &E12
    } else {
        &E24
    }
}

/// The two neighbouring E-series values (below, above) of `v` (any unit).
pub fn e_neighbours(v: f64, series: &str) -> (f64, f64) {
    let s = e_series(series);
    let dec = 10f64.powf(v.log10().floor());
    let m = v / dec;
    let mut lo = s[s.len() - 1] * dec / 10.0;
    let mut hi = 10.0 * dec;
    for &x in s {
        if x <= m * (1.0 + 1e-9) {
            lo = x * dec;
        }
        if x >= m * (1.0 - 1e-9) {
            hi = x * dec;
            break;
        }
    }
    (lo, hi)
}

/// Nearest E-series value (log distance).
pub fn e_nearest(v: f64, series: &str) -> f64 {
    let (lo, hi) = e_neighbours(v, series);
    if (v / lo).ln() <= (hi / v).ln() {
        lo
    } else {
        hi
    }
}

/// Band sweep result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sweep {
    pub f_ghz: Vec<f64>,
    pub s11_re: Vec<f64>,
    pub s11_im: Vec<f64>,
    pub rl_db: Vec<f64>,
    pub vswr: Vec<f64>,
    /// Power reaching the antenna / available power, dB (mismatch + component loss).
    pub gt_db: Vec<f64>,
}

pub fn sweep(net: &[Element], ant: &AntennaModel, f_lo: f64, f_hi: f64, n: usize) -> Sweep {
    let fs = linspace(f_lo, f_hi, n.max(2));
    let mut s = Sweep {
        f_ghz: fs.clone(),
        s11_re: vec![],
        s11_im: vec![],
        rl_db: vec![],
        vswr: vec![],
        gt_db: vec![],
    };
    for f in fs {
        let (gt, g) = transducer(net, ant.z(f), f * 1e9, 50.0);
        s.s11_re.push(g.re);
        s.s11_im.push(g.im);
        s.rl_db.push(return_loss_db(g));
        s.vswr.push(vswr(g));
        s.gt_db.push(10.0 * gt.max(1e-12).log10());
    }
    s
}

/// Band summary of a network on an antenna model.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct BandSummary {
    pub worst_rl_db: f64,
    pub worst_vswr: f64,
    /// Worst (most negative) transducer gain in band, dB.
    pub worst_gt_db: f64,
    pub centre_rl_db: f64,
}

pub fn band_summary(net: &[Element], ant: &AntennaModel) -> BandSummary {
    let s = sweep(net, ant, BLE_LO_GHZ, BLE_HI_GHZ, 41);
    let worst_rl = s.rl_db.iter().cloned().fold(f64::INFINITY, f64::min);
    let (_, gc) = transducer(net, ant.z(ant.fc_ghz), ant.fc_ghz * 1e9, 50.0);
    BandSummary {
        worst_rl_db: worst_rl,
        worst_vswr: s.vswr.iter().cloned().fold(0.0, f64::max),
        worst_gt_db: s.gt_db.iter().cloned().fold(f64::INFINITY, f64::min),
        centre_rl_db: return_loss_db(gc),
    }
}

/// Snap every element to E-series neighbours, trying all floor/ceil
/// combinations and keeping the one with the best worst-in-band return loss.
pub fn snap(net: &[Element], ant: &AntennaModel, series: &str, ql: f64, qc: f64) -> Vec<Element> {
    let n = net.len();
    let opts: Vec<(f64, f64)> = net.iter().map(|e| e_neighbours(e.value, series)).collect();
    let mut best: Option<(f64, Vec<Element>)> = None;
    for mask in 0..(1u32 << n) {
        let cand: Vec<Element> = net
            .iter()
            .enumerate()
            .map(|(k, e)| Element {
                kind: e.kind,
                value: if mask & (1 << k) != 0 { opts[k].1 } else { opts[k].0 },
                q: if e.kind.is_l() { ql } else { qc },
            })
            .collect();
        let w = band_summary(&cand, ant).worst_rl_db;
        if best.as_ref().is_none_or(|(b, _)| w > *b) {
            best = Some((w, cand));
        }
    }
    best.map(|b| b.1).unwrap_or_default()
}

/// Γ path on the Smith chart as each element (from the load outward) is
/// added gradually — for drawing the constant-r / constant-g arcs.
pub fn smith_path(net: &[Element], zl: C64, f_hz: f64, z0: f64, pts: usize) -> Vec<Vec<[f64; 2]>> {
    let mut z = zl;
    let mut arcs = Vec::new();
    for e in net.iter().rev() {
        let ze = e.z(f_hz);
        let mut arc = Vec::new();
        for k in 0..=pts {
            let t = k as f64 / pts as f64;
            let zz = if e.kind.is_series() {
                z + C64::new(0.0, ze.im * t)
            } else {
                // admittance moves along constant conductance
                let y = z.inv() + C64::new(0.0, ze.inv().im * t);
                y.inv()
            };
            let g = gamma(zz, z0);
            arc.push([g.re, g.im]);
        }
        z = if e.kind.is_series() { z + ze } else { (z.inv() + ze.inv()).inv() };
        arcs.push(arc);
    }
    arcs
}

// ------------------------------------------------------------------ API

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MatchIn {
    pub antenna: AntennaModel,
    /// "l" or "pi".
    pub topology: String,
    pub pi_q: f64,
    pub series: String,
    pub q_l: f64,
    pub q_c: f64,
    pub z0: f64,
    /// Sweep span for plots.
    pub plot_lo_ghz: f64,
    pub plot_hi_ghz: f64,
    /// Evaluate this network too (e.g. the one stored on Iris).
    pub current: Option<Vec<Element>>,
}

impl Default for MatchIn {
    fn default() -> Self {
        MatchIn {
            antenna: AntennaModel::default(),
            topology: "l".into(),
            pi_q: 3.0,
            series: "E24".into(),
            q_l: 40.0,
            q_c: 150.0,
            z0: 50.0,
            plot_lo_ghz: 2.2,
            plot_hi_ghz: 2.7,
            current: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Solution {
    pub name: String,
    pub topology: String,
    pub ideal: Vec<Element>,
    pub snapped: Vec<Element>,
    pub labels: Vec<String>,
    pub ideal_summary: BandSummary,
    pub snapped_summary: BandSummary,
    pub sweep: Sweep,
    pub band: Sweep,
    pub path: Vec<Vec<[f64; 2]>>,
    /// Loaded Q of the network (L: fixed by the resistance ratio).
    pub loaded_q: f64,
    pub touchstone: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchOut {
    pub z_load: C64,
    pub gamma_load: C64,
    pub f0_ghz: f64,
    pub bare: BandSummary,
    pub bare_sweep: Sweep,
    pub solutions: Vec<Solution>,
    /// Index of the recommended solution (best snapped worst-in-band RL).
    pub best: usize,
    pub current: Option<Solution>,
    pub l_network_q: f64,
    pub notes: Vec<String>,
}

pub fn touchstone(net: &[Element], ant: &AntennaModel, title: &str) -> String {
    let s = sweep(net, ant, 2.30, 2.60, 61);
    let mut t = String::new();
    t += &format!("! {title}\n");
    t += "! S11 at the 50 ohm port looking into the matching network + antenna model\n";
    t += &format!(
        "! Antenna model (ASSUMED, measure with a VNA): R = {:.1} ohm, X = {:.1} ohm at {:.4} GHz, Q = {:.1}\n",
        ant.r_ohm, ant.x_ohm, ant.fc_ghz, ant.q
    );
    for e in net {
        t += &format!(
            "! {} (Q = {})\n",
            e.label(),
            if e.q > 0.0 { format!("{:.0}", e.q) } else { "ideal".into() }
        );
    }
    t += "! Generated by Protoforge RF & Antenna Lab (rf.match)\n# GHz S RI R 50\n";
    for i in 0..s.f_ghz.len() {
        t += &format!("{:.6} {:.6} {:.6}\n", s.f_ghz[i], s.s11_re[i], s.s11_im[i]);
    }
    t
}

fn solution(name: String, topology: &str, ideal: Vec<Element>, i: &MatchIn, loaded_q: f64) -> Solution {
    let ant = &i.antenna;
    let fhz = ant.fc_ghz * 1e9;
    let snapped = snap(&ideal, ant, &i.series, i.q_l, i.q_c);
    let path = smith_path(&snapped, ant.z(ant.fc_ghz), fhz, i.z0, 24);
    Solution {
        labels: snapped.iter().map(|e| e.label()).collect(),
        ideal_summary: band_summary(&ideal, ant),
        snapped_summary: band_summary(&snapped, ant),
        sweep: sweep(&snapped, ant, i.plot_lo_ghz, i.plot_hi_ghz, 121),
        band: sweep(&snapped, ant, BLE_LO_GHZ, BLE_HI_GHZ, 41),
        touchstone: touchstone(&snapped, ant, &format!("Iris 2.4 GHz antenna match — {name}")),
        name,
        topology: topology.into(),
        ideal,
        snapped,
        path,
        loaded_q,
    }
}

pub fn run(i: MatchIn) -> Result<MatchOut, String> {
    let ant = i.antenna;
    if !(ant.r_ohm > 0.0 && ant.q > 0.0 && ant.fc_ghz > 0.0) {
        return Err("antenna model needs R > 0, Q > 0, fc > 0".into());
    }
    let fhz = ant.fc_ghz * 1e9;
    let zl = ant.z(ant.fc_ghz);
    // L-section Q from the resistance ratio, taking the larger of the series-form
    // (R_L vs Z0) and parallel-form (|Z|²/R_L vs Z0) transformations
    let rp = 1.0 / zl.inv().re;
    let l_q = (rp.max(i.z0) / rp.min(i.z0) - 1.0)
        .max(zl.re.max(i.z0) / zl.re.min(i.z0) - 1.0)
        .max(0.0)
        .sqrt();
    let mut sols = Vec::new();
    if i.topology == "pi" {
        let (net, _) = pi_match(zl, i.z0, fhz, i.pi_q)?;
        sols.push(solution(format!("Pi, loaded Q = {:.1}", i.pi_q), "pi", net, &i, i.pi_q));
    } else {
        for (k, (net, _, _, topo)) in l_match(zl, i.z0, fhz).into_iter().enumerate() {
            let name = format!(
                "L-network {} ({})",
                (b'A' + k as u8) as char,
                if topo == "shunt-at-load" {
                    "shunt element at the antenna"
                } else {
                    "series element at the antenna"
                }
            );
            sols.push(solution(name, topo, net, &i, l_q));
        }
    }
    if sols.is_empty() {
        return Err("no matching solution for this load".into());
    }
    let best = sols
        .iter()
        .enumerate()
        .max_by(|a, b| {
            a.1.snapped_summary
                .worst_rl_db
                .partial_cmp(&b.1.snapped_summary.worst_rl_db)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|x| x.0)
        .unwrap_or(0);
    let current = i.current.clone().filter(|c| !c.is_empty()).map(|c| {
        let path = smith_path(&c, zl, fhz, i.z0, 24);
        Solution {
            name: "On Iris now".into(),
            topology: "custom".into(),
            labels: c.iter().map(|e| e.label()).collect(),
            ideal_summary: band_summary(&c, &ant),
            snapped_summary: band_summary(&c, &ant),
            sweep: sweep(&c, &ant, i.plot_lo_ghz, i.plot_hi_ghz, 121),
            band: sweep(&c, &ant, BLE_LO_GHZ, BLE_HI_GHZ, 41),
            touchstone: touchstone(&c, &ant, "Iris 2.4 GHz antenna match — current design"),
            ideal: c.clone(),
            snapped: c,
            path,
            loaded_q: 0.0,
        }
    });
    Ok(MatchOut {
        z_load: zl,
        gamma_load: gamma(zl, i.z0),
        f0_ghz: ant.f0_ghz(),
        bare: band_summary(&[], &ant),
        bare_sweep: sweep(&[], &ant, i.plot_lo_ghz, i.plot_hi_ghz, 121),
        solutions: sols,
        best,
        current,
        l_network_q: l_q,
        notes: vec![
            "Antenna impedance is an ASSUMED model of a detuned chip antenna — measure the real feed impedance with a VNA on the assembled board (with the frame and a head nearby) and re-run.".into(),
            format!("Standard values snapped to {} with component Q: inductors {:.0}, capacitors {:.0} (typical 0402 RF parts near 2.4 GHz — check the vendor's Q curves).", i.series, i.q_l, i.q_c),
            "Place the network right at the antenna feed; a pi footprint (three pads) lets you re-tune either L or pi on the bench.".into(),
        ],
    })
}

/// The lab's recommended network for an antenna model (best snapped L solution).
pub fn recommend(ant: &AntennaModel, series: &str, ql: f64, qc: f64) -> Vec<Element> {
    let i = MatchIn {
        antenna: *ant,
        series: series.into(),
        q_l: ql,
        q_c: qc,
        ..Default::default()
    };
    match run(i) {
        Ok(o) => o.solutions[o.best].snapped.clone(),
        Err(_) => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_match_pozar_example_5_1() {
        // Pozar Example 5.1: Z_L = 200 − j100 Ω, Z0 = 100 Ω, 500 MHz.
        // Solution 1: B = 0.0029 S (C = 0.92 pF), X = 122 Ω (L = 38.8 nH);
        // solution 2: B = −0.0069 S (L = 46.1 nH), X = −122 Ω (C = 2.61 pF).
        let f = 500e6;
        let sols = l_match(C64::new(200.0, -100.0), 100.0, f);
        assert_eq!(sols.len(), 2);
        let (n1, x1, b1, _) = &sols[0];
        assert!((b1 - 0.0029).abs() < 0.0001 && (x1 - 122.0).abs() < 1.0, "{b1} {x1}");
        assert!((n1[1].value * 1e12 - 0.92).abs() < 0.02 && (n1[0].value * 1e9 - 38.8).abs() < 0.2);
        let (n2, x2, b2, _) = &sols[1];
        assert!((b2 + 0.0069).abs() < 0.0001 && (x2 + 122.0).abs() < 1.0, "{b2} {x2}");
        assert!((n2[1].value * 1e9 - 46.1).abs() < 0.2 && (n2[0].value * 1e12 - 2.61).abs() < 0.02);
        for (n, ..) in &sols {
            let g = gamma(z_in(n, C64::new(200.0, -100.0), f), 100.0);
            assert!(g.abs() < 1e-9);
        }
    }

    #[test]
    fn l_match_low_resistance_load_both_solutions_match() {
        let zl = C64::new(18.0, -30.0);
        let sols = l_match(zl, 50.0, 2.44e9);
        assert!(sols.len() >= 2);
        for (n, ..) in &sols {
            assert!(gamma(z_in(n, zl, 2.44e9), 50.0).abs() < 1e-9);
        }
    }

    #[test]
    fn pi_match_hits_50_ohm_with_given_q() {
        let zl = C64::new(18.0, -30.0);
        let (net, qmin) = pi_match(zl, 50.0, 2.44e9, 4.0).unwrap();
        assert!(qmin < 4.0);
        assert_eq!(net.len(), 3);
        assert!(gamma(z_in(&net, zl, 2.44e9), 50.0).abs() < 1e-9);
        assert!(pi_match(zl, 50.0, 2.44e9, 0.5).is_err());
    }

    #[test]
    fn antenna_model_reproduces_centre_impedance() {
        let a = AntennaModel::default();
        let z = a.z(a.fc_ghz);
        assert!((z.re - 18.0).abs() < 1e-9 && (z.im + 30.0).abs() < 1e-9);
        assert!(a.f0_ghz() > a.fc_ghz, "capacitive → resonance above band");
    }

    #[test]
    fn e_series_snapping() {
        assert_eq!(e_nearest(3.8e-9, "E12"), 3.9e-9);
        let (lo, hi) = e_neighbours(1.25e-12, "E24");
        assert!((lo - 1.2e-12).abs() < 1e-18 && (hi - 1.3e-12).abs() < 1e-18);
        let (lo, hi) = e_neighbours(9.5, "E24");
        assert!((lo - 9.1).abs() < 1e-9 && (hi - 10.0).abs() < 1e-9);
    }

    #[test]
    fn lossy_components_reduce_transducer_gain() {
        let zl = C64::new(18.0, -30.0);
        let mut n = l_match(zl, 50.0, 2.44e9)[0].0.clone();
        let (g_ideal, _) = transducer(&n, zl, 2.44e9, 50.0);
        assert!((g_ideal - 1.0).abs() < 1e-9);
        for e in n.iter_mut() {
            e.q = 30.0;
        }
        let (g, _) = transducer(&n, zl, 2.44e9, 50.0);
        assert!(g < 0.99 && g > 0.7, "{g}");
    }

    #[test]
    fn recommended_network_meets_10_db_in_band() {
        let net = recommend(&AntennaModel::default(), "E24", 40.0, 150.0);
        let s = band_summary(&net, &AntennaModel::default());
        assert!(s.worst_rl_db > 10.0, "{:?}", s);
        let t = touchstone(&net, &AntennaModel::default(), "t");
        assert!(t.contains("# GHz S RI R 50"));
    }
}
