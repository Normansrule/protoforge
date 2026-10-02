//! # Reliability & Test Lab engine — Weibull, accelerated life, sample sizes, block diagrams, verification plans
//!
//! | file | what |
//! |---|---|
//! | [`stats`] | log-gamma, normal and chi-square quantiles, incomplete gamma, binomial |
//! | [`weibull`] | median ranks (Johnson + Bernard), rank regression (RRX/RRY), maximum likelihood with suspensions, Weibayes, Fisher-matrix and likelihood-ratio bounds, bathtub curve, estimator study, sample data |
//! | [`alt`] | Arrhenius, Peck, Coffin–Manson, Norris–Landzberg, inverse power law; zero/few-failure test planner; chamber profiles |
//! | [`sample`] | success run, binomial with failures, Lipson/Weibayes extension, operating-characteristic curves |
//! | [`rbd`] | series / parallel / k-of-n block diagrams, MTTF and parts-count MTBF, Birnbaum and criticality importance, Monte Carlo |
//! | [`iris`] | the use profile and the Iris block diagram from the bill of materials (typical failure rates) |
//! | [`dvt`] | design verification test plan, requirement matrix, schedule, CSV/Markdown |
//!
//! Metrics (see [`metrics`]): `reliability.mtbf_h` (parts-count MTBF of the
//! constant-rate blocks, operating hours), `reliability.r_2yr_pct` (system
//! reliability over two years at the use profile, wear-out included),
//! `reliability.dvt_units` (product units the DVT plan needs) and the
//! informational `reliability.b10_years`.
//!
//! ## References
//! * R. B. Abernethy, *The New Weibull Handbook*, 5th ed., 2006.
//! * W. Q. Meeker & L. A. Escobar, *Statistical Methods for Reliability Data*, Wiley 1998.
//! * W. Nelson, *Accelerated Testing: Statistical Models, Test Plans and Data Analyses*, Wiley 1990.
//! * P. D. T. O'Connor & A. Kleyner, *Practical Reliability Engineering*, 5th ed., Wiley 2012.
//! * NIST/SEMATECH, *e-Handbook of Statistical Methods*, ch. 8 (Assessing product reliability).
//! * MIL-HDBK-217F, *Reliability Prediction of Electronic Equipment*, 1991 (Notice 2, 1995).
//! * C. Lipson & N. J. Sheth, *Statistical Design and Analysis of Engineering Experiments*, 1973.
//! * J. F. Lawless, *Statistical Models and Methods for Lifetime Data*, 2nd ed., Wiley 2003.
//! * J. Lieblein & M. Zelen, "Statistical investigation of the fatigue life of deep-groove ball bearings", J. Res. NBS 57(5), 1956.
//! * Open tools studied: `reliability` (MatthewReid854/reliability), `lifelines`
//!   (CamDavidsonPilon/lifelines), SurPyval (derrynknife/SurPyval).

use serde::Deserialize;
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Metric, Project};

pub mod alt;
pub mod dvt;
pub mod iris;
pub mod rbd;
pub mod sample;
pub mod stats;
pub mod weibull;

#[derive(Deserialize)]
#[serde(default)]
struct GenIn {
    beta: f64,
    eta: f64,
    n: usize,
    censor_at: Option<f64>,
    seed: u64,
}

impl Default for GenIn {
    fn default() -> Self {
        GenIn {
            beta: 2.0,
            eta: 1000.0,
            n: 10,
            censor_at: None,
            seed: 1,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct IrisIn {
    project: Option<Project>,
    samples: usize,
    seed: u64,
    /// Years for the mission point (default: the configured mission).
    years: Option<f64>,
}

impl Default for IrisIn {
    fn default() -> Self {
        IrisIn {
            project: None,
            samples: 20_000,
            seed: 1,
            years: None,
        }
    }
}

#[derive(serde::Serialize)]
struct IrisOut {
    config: iris::Config,
    profile: iris::UseProfile,
    tree: rbd::Node,
    rbd: rbd::RbdOut,
    excluded: Vec<iris::Excluded>,
    r_2yr: f64,
}

fn iris_eval(i: IrisIn) -> Result<IrisOut, String> {
    let p = i.project.unwrap_or_else(Project::iris);
    let cfg = iris::Config::from_project(&p);
    let u = iris::use_profile(&p, &cfg);
    let (tree, excluded) = iris::tree(&p, &cfg, &u);
    let years = i.years.unwrap_or(u.mission_years);
    let out = rbd::evaluate(rbd::RbdIn {
        root: tree.clone(),
        mission_h: years * 365.0 * u.hours_per_day,
        samples: i.samples,
        seed: i.seed,
        horizon_h: 0.0,
    })?;
    let r_2yr = rbd::rel(&tree, 2.0 * 365.0 * u.hours_per_day, false, None);
    Ok(IrisOut {
        config: cfg,
        profile: u,
        tree,
        rbd: out,
        excluded,
        r_2yr,
    })
}

fn dvt_eval(i: dvt::DvtIn) -> Result<dvt::DvtOut, String> {
    let p = i.project.clone().unwrap_or_else(Project::iris);
    let cfg = iris::Config::from_project(&p);
    let u = iris::use_profile(&p, &cfg);
    Ok(dvt::plan(&p, &cfg, &u, &i))
}

/// JSON API for the Reliability & Test Lab.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "reliability.weibull_fit" => handle(input, weibull::fit),
        "reliability.weibull_sets" => Some(crate::api::to_value(weibull::sample_sets())),
        "reliability.weibull_generate" => handle(input, |i: GenIn| {
            if !(i.beta > 0.0 && i.eta > 0.0) {
                return Err("β and η must be positive".into());
            }
            Ok(weibull::generate(i.beta, i.eta, i.n.clamp(1, 5000), i.censor_at, i.seed))
        }),
        "reliability.weibull_study" => handle(input, weibull::study),
        "reliability.bathtub" => handle(input, weibull::bathtub),
        "reliability.alt" => handle(input, alt::acceleration),
        "reliability.alt_plan" => handle(input, alt::plan),
        "reliability.chamber_profile" => handle(input, alt::profile),
        "reliability.sample_size" => handle(input, sample::analyze),
        "reliability.rbd" => handle(input, rbd::evaluate),
        "reliability.iris" => handle(input, iris_eval),
        "reliability.dvt" => handle(input, dvt_eval),
        "reliability.config_defaults" => Some(crate::api::to_value(iris::Config::default())),
        _ => None,
    }
}

thread_local! {
    static METRICS: crate::memo::Slot<Vec<Metric>> = const { std::cell::RefCell::new(None) };
}

/// Metrics feeding requirement verification (memoised on everything they read).
pub fn metrics(p: &Project) -> Vec<Metric> {
    let key = crate::memo::key(&(
        &p.components,
        &p.subsystems,
        &p.design.battery,
        &p.design.power_rails,
        &p.design.thermal,
        p.design.extra.get("reliability"),
        &p.meta.name,
    ));
    crate::memo::memo(&METRICS, key, || metrics_uncached(p))
}

fn metrics_uncached(p: &Project) -> Vec<Metric> {
    let cfg = iris::Config::from_project(p);
    let u = iris::use_profile(p, &cfg);
    let (tree, _) = iris::tree(p, &cfg, &u);
    if tree.children.is_empty() {
        return vec![];
    }
    let src = "reliability";
    let t2 = 2.0 * 365.0 * u.hours_per_day;
    let out = match rbd::evaluate(rbd::RbdIn {
        root: tree.clone(),
        mission_h: t2,
        samples: 0,
        seed: 1,
        horizon_h: 0.0,
    }) {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    let d = dvt::plan(p, &cfg, &u, &dvt::DvtIn::default());
    let top = out
        .blocks
        .iter()
        .max_by(|a, b| a.criticality.total_cmp(&b.criticality))
        .map(|b| format!("{} ({:.0} % of failures)", b.label, b.criticality * 100.0))
        .unwrap_or_default();
    let hours_per_year = 365.0 * u.hours_per_day;
    vec![
        Metric::new("reliability.mtbf_h", "Parts-count MTBF (random failures)", out.mtbf_random_h, "h", src).with_note(&format!(
            "operating hours; {:.0} FIT total of typical failure rates (replace with supplier data) at a {:.1} °C board in use (Arrhenius {:.2} eV, ×{:.2} vs 40 °C); wear-out items excluded",
            out.fit_total, u.board_c, cfg.ea_ev, u.temp_factor
        )),
        Metric::new("reliability.r_2yr_pct", "Reliability over 2 years at the use profile", 100.0 * out.r_mission, "%", src).with_note(&format!(
            "{:.0} h/day worn ({:.0} operating hours), {:.2} battery cycles/day, {:.0} hinge cycles/day; random failures and wear-out (battery to 80 %, hinge, hinge flex); dominated by {top}",
            u.hours_per_day, t2, u.charge_cycles_per_day, u.hinge_cycles_per_day
        )),
        Metric::new("reliability.dvt_units", "Units needed for the DVT plan", d.total_units as f64, "units", src).with_note(&format!(
            "R = {} at {:.0} % confidence: legs {}",
            d.reliability,
            d.confidence * 100.0,
            d.legs
                .iter()
                .filter(|l| l.product_units && l.units > 0)
                .map(|l| format!("{} {}", l.id, l.units))
                .collect::<Vec<_>>()
                .join(", ")
        )),
        Metric::new("reliability.b10_years", "System B10 life (10 % failed)", out.b10_h / hours_per_year, "years", src)
            .with_note("calendar years at the use profile, all failure modes"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn iris_metrics_are_sensible_and_fast() {
        let p = Project::iris();
        let t0 = std::time::Instant::now();
        let m = metrics_uncached(&p);
        let cold = t0.elapsed();
        let v = |k: &str| m.iter().find(|x| x.key == k).unwrap_or_else(|| panic!("missing {k}")).value;
        let mtbf = v("reliability.mtbf_h");
        assert!(mtbf > 1e5 && mtbf < 1e8, "{mtbf}");
        let r2 = v("reliability.r_2yr_pct");
        assert!(r2 > 30.0 && r2 < 100.0, "{r2}");
        assert_eq!(v("reliability.dvt_units"), 37.0);
        assert!(v("reliability.b10_years") > 0.1);
        assert!(cold.as_millis() < 2000, "{cold:?}");
        let t0 = std::time::Instant::now();
        let _ = metrics(&p);
        let _ = metrics(&p);
        assert!(t0.elapsed().as_millis() < 2000);
    }

    #[test]
    fn dispatch_every_method() {
        for (mth, inp) in [
            ("reliability.weibull_fit", json!({})),
            ("reliability.weibull_fit", json!({"method": "rrx", "bounds": "lr"})),
            ("reliability.weibull_fit", json!({"text": "100+\n200+\n300+", "method": "weibayes"})),
            ("reliability.weibull_sets", json!({})),
            ("reliability.weibull_generate", json!({"beta": 1.5})),
            ("reliability.weibull_study", json!({"reps": 50})),
            ("reliability.bathtub", json!({})),
            ("reliability.alt", json!({})),
            ("reliability.alt", json!({"model": "peck"})),
            ("reliability.alt", json!({"model": "coffin_manson"})),
            ("reliability.alt", json!({"model": "norris_landzberg"})),
            ("reliability.alt", json!({"model": "ipl"})),
            ("reliability.alt_plan", json!({})),
            ("reliability.chamber_profile", json!({})),
            ("reliability.chamber_profile", json!({"kind": "damp_heat"})),
            ("reliability.sample_size", json!({})),
            ("reliability.iris", json!({"samples": 2000})),
            ("reliability.dvt", json!({})),
            ("reliability.config_defaults", json!({})),
        ] {
            let r = dispatch(mth, &inp).unwrap();
            assert!(r.is_ok(), "{mth}: {r:?}");
        }
        assert!(dispatch("reliability.nope", &json!({})).is_none());
        // the RBD method takes the tree the Iris method returns
        let iris = dispatch("reliability.iris", &json!({"samples": 0})).unwrap().unwrap();
        let r = dispatch(
            "reliability.rbd",
            &json!({"root": iris["tree"], "mission_h": 5840.0, "samples": 1000}),
        )
        .unwrap();
        assert!(r.is_ok(), "{r:?}");
    }

    #[test]
    fn config_overrides_change_the_answer() {
        let mut p = Project::iris();
        let base = metrics_uncached(&p);
        let v = |m: &[Metric], k: &str| m.iter().find(|x| x.key == k).unwrap().value;
        p.design
            .extra
            .insert("reliability".into(), json!({"battery_counts": false, "dvt_reliability": 0.95}));
        let m = metrics_uncached(&p);
        assert!(v(&m, "reliability.r_2yr_pct") > v(&base, "reliability.r_2yr_pct"));
        assert!(v(&m, "reliability.dvt_units") > v(&base, "reliability.dvt_units"));
        // MTBF (random part) is unchanged by the battery setting
        assert!((v(&m, "reliability.mtbf_h") - v(&base, "reliability.mtbf_h")).abs() / v(&base, "reliability.mtbf_h") < 1e-6);
    }

    #[test]
    #[ignore]
    fn dump() {
        let p = Project::iris();
        for m in metrics_uncached(&p) {
            println!("{:28} {:12.3} {:6} {}", m.key, m.value, m.unit, m.note);
        }
        let o = iris_eval(IrisIn {
            samples: 20000,
            ..Default::default()
        })
        .unwrap();
        println!("{:?}", o.profile);
        for b in &o.rbd.blocks {
            println!(
                "{:6} {:40} {:8.2} R={:.5} IC={:.3} mc={:.3}",
                b.refdes, b.class, b.fit, b.r_mission, b.criticality, b.mc_first_share
            );
        }
        println!("mc {:?}", o.rbd.mc.as_ref().map(|m| (m.r_mission, m.ci_lo, m.ci_hi)));
        let scen = |name: &str, p: Project| {
            let m = metrics_uncached(&p);
            let v = |k: &str| m.iter().find(|x| x.key == k).unwrap().value;
            println!(
                "{name:48} R2y {:6.2} %  MTBF {:10.0} h  B10 {:5.2} y",
                v("reliability.r_2yr_pct"),
                v("reliability.mtbf_h"),
                v("reliability.b10_years")
            );
        };
        let mut a = Project::iris();
        a.design.battery.capacity_mah = 250.0;
        scen("250 mAh cell", a);
        let mut a = Project::iris();
        a.design
            .extra
            .insert("reliability".into(), json!({"blocks": {"BT1": {"eta": 1200.0}}}));
        scen("cell rated for η = 1200 cycles", a);
        let mut a = Project::iris();
        a.design.extra.insert("reliability".into(), json!({"battery_counts": false}));
        scen("battery as service item", a);
        let mut a = Project::iris();
        a.design.extra.insert("reliability".into(), json!({"mic_k": 1}));
        scen("1-of-2 microphones", a);
        let d = dvt_eval(dvt::DvtIn::default()).unwrap();
        for t in &d.tests {
            println!(
                "{:3} {:45} {:3} {:6.1} {:6.1}-{:6.1} {}",
                t.id, t.name, t.units, t.duration_d, t.start_d, t.end_d, t.basis
            );
        }
    }
}
