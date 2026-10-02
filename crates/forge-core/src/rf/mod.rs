//! # RF & Antenna Lab engine — radio-frequency (RF) design for the Iris Bluetooth link
//!
//! | file | what |
//! |---|---|
//! | [`fdtd`] | 2-D TMz finite-difference time-domain (FDTD) Yee solver with a Bérenger perfectly matched layer (PML); presets: line source (dipole H-plane), phased array, head phantom, ground plane, slit, waveguide |
//! | [`antenna`] | resonant-length first cuts (dipole, monopole, inverted-F antenna (IFA)), analytic dipole and array-factor patterns, numerical directivity, 3-D pattern mesh, Chu–McLean small-antenna limit |
//! | [`matching`] | Smith-chart maths, L-network (both solutions) and loaded-Q pi-network synthesis, E-series snapping with component Q, band sweeps, Touchstone `.s1p` |
//! | [`tline`] | microstrip (Hammerstad–Jensen), grounded coplanar waveguide (CPWG, Wadell), differential pair (IPC-2141), losses, width synthesis |
//! | [`link`] | Bluetooth Low Energy (BLE) link budget: Friis and log-distance path loss, head/body loss, Rayleigh fade margin, range per PHY |
//!
//! ## Iris design (`design.extra.rf`)
//!
//! [`IrisRf`] holds the lab's design choices. Missing fields take the
//! defaults below; **`network` and `feed` are absent on Iris v0.5**, which is
//! the honest current state: the board has no antenna matching network (a
//! bill-of-materials (BOM) gap PCB Studio also flags) and PCB Studio routes the
//! RF net at the minimum trace width. The lab's page proposes both and can
//! write them into `design.extra.rf`.
//!
//! ## Metrics
//!
//! * `rf.match_return_loss_db` — worst return loss over 2.400–2.4835 GHz at
//!   the 50 Ω port, antenna model + the matching network on Iris (none → bare
//!   antenna). Recommend ≥ 10 dB (VSWR ≤ 1.9).
//! * `rf.feed_z0_ohm` — characteristic impedance of the antenna feed trace on
//!   the Iris stack-up (CPWG over the bottom ground). Recommend 50 ± 5 Ω.
//! * `rf.link_margin_db` — BLE 1M link margin to a phone at 5 m with head and
//!   body loss and a 90 % Rayleigh fade margin, worse of both directions.
//!   Recommend ≥ 0 dB.
//! * `rf.range_m` — distance at which that margin reaches 0 dB.
//! * `rf.antenna_efficiency_pct` — radiation efficiency × worst in-band
//!   transducer gain of the match (informational).
//!
//! ## References
//!
//! * D. M. Pozar, *Microwave Engineering*, 4th ed., Wiley, 2012.
//! * C. A. Balanis, *Antenna Theory: Analysis and Design*, 4th ed., Wiley, 2016.
//! * A. Taflove & S. C. Hagness, *Computational Electrodynamics: The Finite-Difference Time-Domain Method*, 3rd ed., Artech House, 2005.
//! * K. S. Yee, "Numerical solution of initial boundary value problems involving Maxwell's equations in isotropic media", IEEE Trans. Antennas Propag. 14(3), 302–307, 1966.
//! * J.-P. Bérenger, "A perfectly matched layer for the absorption of electromagnetic waves", J. Comput. Phys. 114(2), 185–200, 1994.
//! * E. Hammerstad & Ø. Jensen, "Accurate models for microstrip computer-aided design", IEEE MTT-S International Microwave Symposium Digest, 407–409, 1980.
//! * B. C. Wadell, *Transmission Line Design Handbook*, Artech House, 1991.
//! * Bluetooth SIG, *Bluetooth Core Specification* v5.x, Vol 6 (Low Energy Controller).
//! * Texas Instruments application notes on small 2.4 GHz antennas (e.g. AN043 "Small size 2.4 GHz PCB antenna", AN058 "Antenna selection guide").
//! * H. T. Friis, "A note on a simple transmission formula", Proc. IRE 34(5), 254–256, 1946.
//! * L. J. Chu, "Physical limitations of omni-directional antennas", J. Appl. Phys. 19, 1163–1175, 1948; J. S. McLean, "A re-examination of the fundamental limits on the radiation Q of electrically small antennas", IEEE Trans. Antennas Propag. 44(5), 672–676, 1996.
//! * T. S. Rappaport, *Wireless Communications: Principles and Practice*, 2nd ed., Prentice Hall, 2002.
//! * C. Bowick, *RF Circuit Design*, 2nd ed., Newnes, 2008.
//! * IEEE Std 1528 / IEC/IEEE 62209-1528 — head tissue-simulating liquid targets (2450 MHz: εr 39.2, σ 1.80 S/m).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::api::{handle, ApiResult};
use crate::project::{Metric, Project};

pub mod antenna;
pub mod fdtd;
pub mod link;
pub mod matching;
pub mod tline;

use matching::{AntennaModel, BandSummary, Element};
use tline::LineKind;

/// A feed-trace design on the Iris stack-up.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Feed {
    pub kind: LineKind,
    pub width_mm: f64,
    pub gap_mm: f64,
    pub length_mm: f64,
}

impl Default for Feed {
    fn default() -> Self {
        Feed {
            kind: LineKind::Cpwg,
            width_mm: 1.0,
            gap_mm: 0.2,
            length_mm: 6.0,
        }
    }
}

/// Link assumptions (the glasses antenna gain and match loss are computed).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LinkDesign {
    pub phy: String,
    pub distance_m: f64,
    pub path_exponent: f64,
    pub tx_dbm: f64,
    pub phone_tx_dbm: f64,
    pub phone_gain_dbi: f64,
    pub phone_sens_1m_dbm: f64,
    pub head_loss_db: f64,
    pub body_loss_db: f64,
    pub availability: f64,
}

impl Default for LinkDesign {
    fn default() -> Self {
        let l = link::LinkIn::default();
        LinkDesign {
            phy: l.phy,
            distance_m: l.distance_m,
            path_exponent: l.path_exponent,
            tx_dbm: l.tx_dbm,
            phone_tx_dbm: l.phone_tx_dbm,
            phone_gain_dbi: l.phone_gain_dbi,
            phone_sens_1m_dbm: l.phone_sens_1m_dbm,
            head_loss_db: l.head_loss_db,
            body_loss_db: l.body_loss_db,
            availability: l.availability,
        }
    }
}

/// RF design choices stored in `design.extra.rf`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct IrisRf {
    /// Chip-antenna feed impedance model — ASSUMED; measure with a VNA.
    pub antenna: AntennaModel,
    /// Antenna radiation efficiency (0..1), typical of a chip antenna on a small ground.
    pub rad_efficiency: f64,
    /// Directivity of the antenna + ground (a small dipole-like radiator: 1.76 dBi).
    pub directivity_dbi: f64,
    /// Matching network from the 50 Ω radio port toward the antenna. `None` = no network (Iris v0.5).
    pub network: Option<Vec<Element>>,
    /// Feed trace design. `None` = as routed by PCB Studio (minimum trace width).
    pub feed: Option<Feed>,
    pub er: f64,
    pub tan_d: f64,
    pub cu_um: f64,
    /// Component Q assumptions for new networks.
    pub q_l: f64,
    pub q_c: f64,
    pub series: String,
    pub link: LinkDesign,
}

impl Default for IrisRf {
    fn default() -> Self {
        IrisRf {
            antenna: AntennaModel::default(),
            rad_efficiency: 0.5,
            directivity_dbi: 1.76,
            network: None,
            feed: None,
            er: 4.4,
            tan_d: 0.02,
            cu_um: 35.0,
            q_l: 40.0,
            q_c: 150.0,
            series: "E24".into(),
            link: LinkDesign::default(),
        }
    }
}

impl IrisRf {
    pub fn from_project(p: &Project) -> IrisRf {
        p.design
            .extra
            .get("rf")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }
}

/// Dielectric height of a 2-layer board: thickness minus both copper layers.
pub fn stack_h_mm(p: &Project, rf: &IrisRf) -> f64 {
    let t = p.design.pcb.thickness_mm;
    let cu = rf.cu_um * 1e-3;
    if p.design.pcb.layers <= 2 {
        (t - 2.0 * cu).max(0.05)
    } else {
        // assume a thin prepreg to layer 2 on multilayer boards (check your fab's stack-up)
        0.2
    }
}

/// The feed PCB Studio actually routes: RF net at the minimum trace width,
/// top ground pour held off by the pour clearance.
pub fn routed_feed(p: &Project) -> Feed {
    let r = crate::pcb::board::Rules::from(&p.design.pcb);
    Feed {
        kind: LineKind::Cpwg,
        width_mm: r.trace_mm,
        gap_mm: r.pour_clearance_mm,
        length_mm: Feed::default().length_mm,
    }
}

pub fn feed_z0(p: &Project, rf: &IrisRf, f: &Feed) -> (f64, f64) {
    let h = stack_h_mm(p, rf);
    let t = rf.cu_um * 1e-3;
    match f.kind {
        LineKind::Microstrip => tline::microstrip(f.width_mm, h, t, rf.er),
        LineKind::DiffMicrostrip => {
            let (zd, _, e) = tline::diff_microstrip(f.width_mm, f.gap_mm, h, t, rf.er);
            (zd, e)
        }
        LineKind::Cpwg => tline::cpwg(f.width_mm, f.gap_mm, h, rf.er),
    }
}

/// The lab's recommended 50 Ω feed: CPWG with a 0.20 mm gap, width rounded to 0.01 mm.
pub fn recommended_feed(p: &Project, rf: &IrisRf) -> Feed {
    let h = stack_h_mm(p, rf);
    let gap = 0.2f64.max(p.design.pcb.min_clearance_mm);
    let w = tline::synth_width(50.0, |w| tline::cpwg(w, gap, h, rf.er).0, 0.02, 20.0);
    Feed {
        kind: LineKind::Cpwg,
        width_mm: (w * 100.0).round() / 100.0,
        gap_mm: gap,
        length_mm: rf.feed.as_ref().map(|f| f.length_mm).unwrap_or(Feed::default().length_mm),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BomRow {
    pub refdes: String,
    pub name: String,
    pub value: String,
    pub footprint: String,
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IrisOut {
    /// The effective design (stored values + defaults).
    pub design: IrisRf,
    /// Whether `design.extra.rf` exists on the project.
    pub stored: bool,
    pub h_mm: f64,
    pub current_network: Vec<Element>,
    pub current_labels: Vec<String>,
    pub current_summary: BandSummary,
    pub recommended_network: Vec<Element>,
    pub recommended_labels: Vec<String>,
    pub recommended_summary: BandSummary,
    pub current_feed: Feed,
    pub current_feed_source: String,
    pub current_feed_z0: f64,
    pub current_feed_eeff: f64,
    pub recommended_feed: Feed,
    pub recommended_feed_z0: f64,
    pub glasses_gain_dbi: f64,
    pub match_loss_db: f64,
    pub link: link::LinkOut,
    pub bom: Vec<BomRow>,
    pub metrics: Vec<Metric>,
    /// `design.extra.rf` with the recommended network and feed applied.
    pub suggested_extra: Value,
    pub notes: Vec<String>,
}

pub fn link_input(rf: &IrisRf, gain: f64, match_loss: f64) -> link::LinkIn {
    let l = &rf.link;
    link::LinkIn {
        phy: l.phy.clone(),
        distance_m: l.distance_m,
        path_exponent: l.path_exponent,
        tx_dbm: l.tx_dbm,
        glasses_gain_dbi: gain,
        match_loss_db: match_loss,
        phone_tx_dbm: l.phone_tx_dbm,
        phone_gain_dbi: l.phone_gain_dbi,
        phone_sens_1m_dbm: l.phone_sens_1m_dbm,
        head_loss_db: l.head_loss_db,
        body_loss_db: l.body_loss_db,
        availability: l.availability,
        ..Default::default()
    }
}

/// Full Iris RF analysis behind the metrics and the page.
pub fn iris(p: &Project) -> Result<IrisOut, String> {
    let rf = IrisRf::from_project(p);
    let stored = p.design.extra.contains_key("rf");
    let net = rf.network.clone().unwrap_or_default();
    let cur = matching::band_summary(&net, &rf.antenna);
    let rec = matching::recommend(&rf.antenna, &rf.series, rf.q_l, rf.q_c);
    let rec_s = matching::band_summary(&rec, &rf.antenna);
    let (feed, feed_src) = match &rf.feed {
        Some(f) => (f.clone(), "design.extra.rf.feed"),
        None => (
            routed_feed(p),
            "as routed by PCB Studio (minimum trace width, pour clearance as gap)",
        ),
    };
    let (z0, eeff) = feed_z0(p, &rf, &feed);
    let rfeed = recommended_feed(p, &rf);
    let (rz0, _) = feed_z0(p, &rf, &rfeed);
    let gain = rf.directivity_dbi + 10.0 * rf.rad_efficiency.clamp(1e-4, 1.0).log10();
    let match_loss = -cur.worst_gt_db;
    let lk = link::run(link_input(&rf, gain, match_loss))?;
    let next_free = |prefix: &str| {
        p.components
            .iter()
            .filter_map(|c| c.refdes.strip_prefix(prefix).and_then(|n| n.parse::<u32>().ok()))
            .max()
            .unwrap_or(0)
            + 1
    };
    let (mut nl, mut nc) = (next_free("L"), next_free("C"));
    let bom: Vec<BomRow> = rec
        .iter()
        .map(|e| BomRow {
            refdes: if e.kind.is_l() {
                nl += 1;
                format!("L{}", nl - 1)
            } else {
                nc += 1;
                format!("C{}", nc - 1)
            },
            name: format!("Antenna match, {}", if e.kind.is_series() { "series" } else { "shunt" }),
            value: e.label().replace("series ", "").replace("shunt ", ""),
            footprint: "0402".into(),
            note: if e.kind.is_l() {
                format!(
                    "RF multilayer/wire-wound inductor, Q ≥ {:.0} at 2.4 GHz, ±0.1–0.2 nH or ±2 % (check vendor)",
                    rf.q_l
                )
            } else {
                format!("C0G/NP0 RF capacitor, Q ≥ {:.0} at 2.4 GHz, ±0.1 pF (check vendor)", rf.q_c)
            },
        })
        .collect();
    let mut suggested = serde_json::to_value(&rf).unwrap_or(Value::Null);
    suggested["network"] = serde_json::to_value(&rec).unwrap_or(Value::Null);
    suggested["feed"] = serde_json::to_value(&rfeed).unwrap_or(Value::Null);
    let mut notes = Vec::new();
    if rf.network.is_none() {
        notes.push(format!(
            "Iris has no antenna matching network yet: the bare (assumed) chip antenna gives {:.1} dB worst return loss in band. The lab's {} network gives {:.1} dB.",
            cur.worst_rl_db,
            rec.iter().map(|e| e.label()).collect::<Vec<_>>().join(" + "),
            rec_s.worst_rl_db
        ));
    }
    if rf.feed.is_none() {
        notes.push(format!(
            "The RF feed is routed at {:.2} mm with {:.2} mm pour clearance → {:.0} Ω. A {:.2} mm CPWG trace with {:.2} mm gaps on this {:.2} mm stack-up is {:.1} Ω.",
            feed.width_mm, feed.gap_mm, z0, rfeed.width_mm, rfeed.gap_mm, stack_h_mm(p, &rf), rz0
        ));
    }
    let out_metrics = build_metrics(&rf, &cur, z0, feed_src, &feed, &lk, gain, match_loss, stack_h_mm(p, &rf));
    Ok(IrisOut {
        design: rf.clone(),
        stored,
        h_mm: stack_h_mm(p, &rf),
        current_labels: net.iter().map(|e| e.label()).collect(),
        current_network: net,
        current_summary: cur,
        recommended_labels: rec.iter().map(|e| e.label()).collect(),
        recommended_network: rec,
        recommended_summary: rec_s,
        current_feed: feed,
        current_feed_source: feed_src.into(),
        current_feed_z0: z0,
        current_feed_eeff: eeff,
        recommended_feed: rfeed,
        recommended_feed_z0: rz0,
        glasses_gain_dbi: gain,
        match_loss_db: match_loss,
        link: lk,
        bom,
        metrics: out_metrics,
        suggested_extra: suggested,
        notes,
    })
}

#[allow(clippy::too_many_arguments)]
fn build_metrics(
    rf: &IrisRf,
    cur: &BandSummary,
    z0: f64,
    feed_src: &str,
    feed: &Feed,
    lk: &link::LinkOut,
    gain: f64,
    match_loss: f64,
    h: f64,
) -> Vec<Metric> {
    let src = "rf";
    let a = &rf.antenna;
    let net_desc = match &rf.network {
        Some(n) if !n.is_empty() => n.iter().map(|e| e.label()).collect::<Vec<_>>().join(" + "),
        _ => "no matching network (bare antenna — Iris v0.5 BOM gap)".into(),
    };
    vec![
        Metric::new("rf.match_return_loss_db", "Antenna port return loss, worst in the BLE band", cur.worst_rl_db, "dB", src).with_note(&format!(
            "2.400–2.4835 GHz at the 50 Ω port: {net_desc}; antenna ASSUMED R {:.0} Ω, X {:.0} Ω at {:.3} GHz, Q {:.0} (measure with a VNA); worst VSWR {:.2}",
            a.r_ohm, a.x_ohm, a.fc_ghz, a.q, cur.worst_vswr
        )),
        Metric::new("rf.feed_z0_ohm", "Antenna feed trace impedance", z0, "Ω", src).with_note(&format!(
            "{:?} {:.2} mm wide, {:.2} mm gap on {:.2} mm FR-4 (εr {:.1}) — {feed_src}",
            feed.kind, feed.width_mm, feed.gap_mm, h, rf.er
        )),
        Metric::new("rf.link_margin_db", "BLE link margin to a phone", lk.margin_db, "dB", src).with_note(&format!(
            "{} at {:.0} m, n = {:.1}, head {:.0} dB + body {:.0} dB loss, {:.1} dB Rayleigh fade margin ({:.0} %); glasses gain {:.1} dBi, match loss {:.1} dB; limited by {} (typical sensitivities)",
            lk.phy,
            rf.link.distance_m,
            rf.link.path_exponent,
            rf.link.head_loss_db,
            rf.link.body_loss_db,
            lk.fade_db,
            100.0 * rf.link.availability,
            gain,
            match_loss,
            lk.limiting
        )),
        Metric::new("rf.range_m", "BLE range at 0 dB margin", lk.range_m, "m", src)
            .with_note("same assumptions as rf.link_margin_db; log-distance model beyond 1 m"),
        Metric::new(
            "rf.antenna_efficiency_pct",
            "Antenna total efficiency (worst in band)",
            100.0 * rf.rad_efficiency * 10f64.powf(-match_loss / 10.0),
            "%",
            src,
        )
        .with_note("radiation efficiency (assumed) × worst in-band transducer gain of the match; head loss not included"),
    ]
}

thread_local! {
    static METRICS: crate::memo::Slot<Vec<Metric>> = const { std::cell::RefCell::new(None) };
}

/// Metrics feeding requirement verification.
pub fn metrics(p: &Project) -> Vec<Metric> {
    let key = crate::memo::key(&(&p.design.pcb, p.design.extra.get("rf")));
    crate::memo::memo(&METRICS, key, || iris(p).map(|o| o.metrics).unwrap_or_default())
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ProjIn {
    project: Option<Project>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct StackIn {
    project: Option<Project>,
    #[serde(flatten)]
    line: tline::LineIn,
    /// Use the Iris board's dielectric height and εr.
    iris_stack: bool,
}

/// JSON API for the RF & Antenna Lab.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "rf.iris" => handle(input, |i: ProjIn| iris(&i.project.unwrap_or_else(Project::iris))),
        "rf.fdtd" => handle(input, fdtd::run),
        "rf.pattern" => handle(input, antenna::pattern),
        "rf.antenna_sizes" => handle(input, antenna::sizing),
        "rf.match" => handle(input, matching::run),
        "rf.line" => handle(input, |i: StackIn| {
            let mut l = i.line;
            if i.iris_stack {
                let p = i.project.unwrap_or_else(Project::iris);
                let rf = IrisRf::from_project(&p);
                l.h_mm = stack_h_mm(&p, &rf);
                l.er = rf.er;
                l.t_um = rf.cu_um;
            }
            tline::line(l)
        }),
        "rf.link" => handle(input, link::run),
        "rf.phys" => Some(Ok(json!(link::PHYS
            .iter()
            .map(|p| json!({"id": p.id, "name": p.name, "rate_kbps": p.rate_kbps, "typ_sens_dbm": p.typ_sens_dbm}))
            .collect::<Vec<_>>()))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_metrics_are_computed() {
        let m = metrics(&Project::iris());
        for k in [
            "rf.match_return_loss_db",
            "rf.feed_z0_ohm",
            "rf.link_margin_db",
            "rf.range_m",
            "rf.antenna_efficiency_pct",
        ] {
            let x = m.iter().find(|x| x.key == k).unwrap_or_else(|| panic!("missing {k}"));
            assert!(x.value.is_finite(), "{k} = {}", x.value);
        }
    }

    #[test]
    fn iris_as_is_vs_recommended() {
        let p = Project::iris();
        let o = iris(&p).unwrap();
        assert!(!o.stored);
        // v0.5: no network → poor return loss; routed 0.15 mm feed is far from 50 Ω
        assert!(o.current_summary.worst_rl_db < 6.0, "{}", o.current_summary.worst_rl_db);
        assert!(o.current_feed_z0 > 70.0, "{}", o.current_feed_z0);
        assert!(o.recommended_summary.worst_rl_db > 10.0);
        assert!((o.recommended_feed_z0 - 50.0).abs() < 1.5);
        // applying the suggestion makes the metrics pass
        let mut q = p.clone();
        q.design.extra.insert("rf".into(), o.suggested_extra.clone());
        let m = metrics(&q);
        let get = |k: &str| m.iter().find(|x| x.key == k).unwrap().value;
        assert!(get("rf.match_return_loss_db") > 10.0);
        assert!((get("rf.feed_z0_ohm") - 50.0).abs() < 5.0);
        assert!(get("rf.link_margin_db") > o.link.margin_db);
    }

    #[test]
    fn dispatch_methods_answer_defaults() {
        for m in [
            "rf.iris",
            "rf.pattern",
            "rf.antenna_sizes",
            "rf.match",
            "rf.line",
            "rf.link",
            "rf.phys",
        ] {
            let r = dispatch(m, &json!({})).unwrap();
            assert!(r.is_ok(), "{m}: {:?}", r.err());
        }
        let r = dispatch("rf.fdtd", &json!({"nx": 60, "ny": 50, "steps": 60, "frames": 3}))
            .unwrap()
            .unwrap();
        assert_eq!(r["n_frames"], 3);
    }
}
