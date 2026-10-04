use std::collections::BTreeMap;

use super::catalog::{self, MetricClass, CAPS, KINDS, METRIC_TABLE};
use super::*;
use crate::systems::{all_metrics, analysis, evaluate, lint, quality, Status};

fn answers(kind: &str, caps: Option<&[&str]>, targets: &[(&str, f64)]) -> Answers {
    Answers {
        name: format!("Test {kind}"),
        idea: "A small thing that helps people".into(),
        users: "Students, makers".into(),
        kind: kind.into(),
        capabilities: caps.map(|c| c.iter().map(|s| s.to_string()).collect()),
        targets: targets.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        author: String::new(),
    }
}

fn gen(kind: &str, caps: Option<&[&str]>, targets: &[(&str, f64)]) -> WizardOut {
    generate(&answers(kind, caps, targets)).unwrap_or_else(|e| panic!("{kind}: {e}"))
}

fn metric(p: &Project, f: fn(&Project) -> Vec<crate::project::Metric>, key: &str) -> f64 {
    f(p).into_iter()
        .find(|m| m.key == key)
        .unwrap_or_else(|| panic!("{key} missing"))
        .value
}

fn roll_up(p: &Project, key: &str) -> f64 {
    let load = crate::systems::budgets::load_power_mw(p);
    let eff = crate::systems::budgets::regulator_efficiency(p);
    let avg = load / eff + p.design.power_rails.quiescent_ua * 1e-3 * p.design.battery.nominal_v;
    match key {
        "avg" => avg,
        "life" => p.design.battery.capacity_mah * p.design.battery.nominal_v * p.design.battery.usable_fraction / avg,
        _ => unreachable!(),
    }
}

#[test]
fn options_cover_the_vocabulary() {
    let o = catalog::options();
    assert_eq!(o["kinds"].as_array().unwrap().len(), 10);
    assert!(o["capabilities"].as_array().unwrap().len() >= 15);
    assert!(o["targets"].as_array().unwrap().len() >= 10);
    for k in KINDS {
        for c in k.default_caps {
            assert!(catalog::cap(c).is_some(), "{}: {c}", k.id);
        }
        for (t, v) in k.defaults {
            let td = catalog::target(t).unwrap_or_else(|| panic!("{}: {t}", k.id));
            assert!(*v >= td.min && *v <= td.max, "{} {t}", k.id);
        }
    }
    for c in CAPS {
        for x in c.excludes {
            assert!(
                catalog::cap(x).unwrap().excludes.contains(&c.id),
                "{} ↔ {x} must be symmetric",
                c.id
            );
        }
    }
    // the api catalog picks the methods up
    assert!(crate::api::METHODS.contains(&"systems.wizard"));
    assert!(crate::api::METHODS.contains(&"systems.wizard_options"));
    let r = crate::api::call_value("systems.wizard", &serde_json::json!({ "kind": "wearable", "name": "Clip" })).unwrap();
    assert!(r["project"]["requirements"].as_array().unwrap().len() > 8);
    assert!(crate::api::call_value("systems.wizard", &serde_json::json!({ "kind": "spaceship" })).is_err());
}

/// Every kind × a few capability sets: lints clean, evaluates, and every
/// metric-bound requirement gets a value from a responsive metric.
#[test]
fn every_kind_generates_a_clean_verifiable_project() {
    let sets: &[(&str, Option<&[&str]>)] = &[
        ("wearable", None),
        (
            "wearable",
            Some(&["battery", "ble", "display", "microphones", "speaker", "imu", "waterproof", "molded"]),
        ),
        ("handheld", None),
        ("handheld", Some(&["battery", "wifi", "camera", "buttons"])),
        ("iot_sensor", None),
        ("iot_sensor", Some(&["battery", "wifi", "env_sensors", "imu", "display"])),
        ("desk_instrument", None),
        ("robot", None),
        ("robot", Some(&["battery", "ble", "motors", "microphones", "display", "speaker"])),
        ("smart_home", None),
        (
            "smart_home",
            Some(&["dc_adapter", "wifi", "microphones", "speaker", "leds", "molded"]),
        ),
        ("optical", None),
        ("quantum", None),
        ("dev_board", None),
        ("dev_board", Some(&["usb_c", "ble", "imu", "env_sensors", "printed"])),
        ("mechanical", None),
        ("mechanical", Some(&["molded", "leds"])),
    ];
    for (kind, caps) in sets {
        let out = gen(kind, *caps, &[]);
        let p = &out.project;
        let label = format!("{kind} {caps:?}");
        let errors: Vec<_> = lint(p).into_iter().filter(|i| i.severity == "error").collect();
        assert!(errors.is_empty(), "{label}: {errors:?}");
        let ev = evaluate(p);
        let qerr: Vec<_> = quality::quality_with(p, Some(&ev.metrics))
            .into_iter()
            .filter(|i| i.severity != "info")
            .collect();
        assert!(qerr.is_empty(), "{label}: {qerr:?}");
        let cov = analysis::coverage(p, &ev);
        assert!(cov.empty_subsystems.is_empty(), "{label}: {:?}", cov.empty_subsystems);
        assert_eq!(cov.needs_covered, cov.needs_total, "{label}");
        assert_eq!(p.milestones.len(), 6);
        assert_eq!(p.trades.len(), 1);
        let d = analysis::trade_detail(&p.trades[0]);
        assert!(d.decision_agrees, "{label}: trade decision");
        assert!(!p.risks.is_empty());
        let mut bound = 0;
        for (r, res) in p.requirements.iter().zip(&ev.results) {
            match r.verification.metric.as_deref() {
                Some(k) => {
                    bound += 1;
                    assert!(catalog::bindable(k), "{label}: {} bound to non-responsive {k}", r.id);
                    assert_ne!(res.status, Status::Unverified, "{label}: {} ({k}) has no value", r.id);
                    assert!(res.value.unwrap().is_finite(), "{label}: {k}");
                }
                None => assert_eq!(res.status, Status::Unverified, "{label}: {}", r.id),
            }
        }
        assert!(bound >= 2, "{label}: {bound} bound requirements");
        for m in &ev.metrics {
            assert!(m.value.is_finite(), "{label}: metric {} = {}", m.key, m.value);
        }
        // every explanation has arithmetic or a result
        assert!(!out.explanations.is_empty(), "{label}");
        // no NaN / undefined in the text the page shows
        let text = serde_json::to_string(&(&out.explanations, &out.warnings)).unwrap();
        assert!(!text.contains("NaN") && !text.contains("n/a"), "{label}: {text}");
        let fails: Vec<_> = p
            .requirements
            .iter()
            .zip(&ev.results)
            .filter(|(_, r)| r.status == Status::Fail)
            .map(|(q, r)| format!("{} {:?} {}", q.id, r.value, r.criterion))
            .collect();
        eprintln!(
            "{label}: {} reqs, {bound} bound, {} pass, fails {fails:?}; warnings {}",
            p.requirements.len(),
            ev.pass,
            out.warnings.len()
        );
        // a failing bound requirement is always announced in the warnings
        if !fails.is_empty() {
            assert!(
                out.warnings.iter().any(|w| w.level == "warn"),
                "{label}: fails {fails:?} without a warning"
            );
        }
    }
}

#[test]
fn defaults_pass_their_own_bound_requirements() {
    // the starter projects should start green where the physics allows it
    for k in KINDS {
        let out = gen(k.id, None, &[]);
        let ev = evaluate(&out.project);
        for (r, res) in out.project.requirements.iter().zip(&ev.results) {
            if r.verification.metric.is_some() {
                assert_eq!(
                    res.status,
                    Status::Pass,
                    "{} {} {} = {:?} ({})",
                    k.id,
                    r.id,
                    r.text,
                    res.value,
                    res.criterion
                );
            }
        }
        assert!(out.warnings.iter().all(|w| w.level != "warn"), "{}: {:?}", k.id, out.warnings);
    }
}

#[test]
fn output_is_deterministic() {
    for k in ["wearable", "robot", "quantum"] {
        let a = serde_json::to_string(&gen(k, None, &[])).unwrap();
        let b = serde_json::to_string(&gen(k, None, &[])).unwrap();
        assert_eq!(a, b, "{k}");
    }
}

#[test]
fn adding_a_display_raises_average_power() {
    let base = gen("wearable", Some(&["battery", "usb_c", "ble", "imu"]), &[]);
    let disp = gen("wearable", Some(&["battery", "usb_c", "ble", "imu", "display"]), &[]);
    let (a, b) = (roll_up(&base.project, "avg"), roll_up(&disp.project, "avg"));
    assert!(b > a * 1.5, "{a} → {b}");
    // the engine's own metrics agree with the wizard's summary
    let pm = metric(&disp.project, crate::power::metrics, "power.avg_power_mw");
    assert!(
        (pm - disp.summary.avg_power_mw).abs() < 1e-6 * pm,
        "{pm} vs {}",
        disp.summary.avg_power_mw
    );
    // the battery grew to keep the same life
    assert!(disp.summary.battery_mah > base.summary.battery_mah);
    assert!(disp.summary.mass_g > base.summary.mass_g);
}

#[test]
fn longer_life_grows_the_battery_and_the_mass() {
    let short = gen("wearable", None, &[("life_h", 12.0)]);
    let long = gen("wearable", None, &[("life_h", 200.0)]);
    assert!(long.summary.battery_mah > short.summary.battery_mah);
    assert!(long.summary.mass_g > short.summary.mass_g);
    let l1 = roll_up(&short.project, "life");
    let l2 = roll_up(&long.project, "life");
    assert!(l1 >= 12.0 && l2 >= 200.0, "{l1} {l2}");
    // cycle life: a robot used 2 h/day for 2 years needs more than its 2 h target per charge
    let r = gen("robot", None, &[]);
    let need = 2.0 * 365.0 * 2.0 * 1.15 / (800.0 * (-(0.9f64.ln())).powf(0.25));
    assert!(r.summary.life_h.unwrap() >= need, "{:?} < {need}", r.summary.life_h);
}

#[test]
fn battery_sizing_hand_calculation() {
    let out = gen("wearable", None, &[("life_h", 16.0)]);
    let p = &out.project;
    let avg = roll_up(p, "avg");
    // C = P × t × 1.2 / (3.7 × 0.85), rounded up to the next pouch size
    let need = avg * 16.0 * 1.2 / (3.7 * 0.85);
    let c = p.design.battery.capacity_mah;
    assert!(c >= need, "{c} < {need}");
    let below = parts::LIPO_SIZES.iter().copied().filter(|s| *s < c).fold(0.0, f64::max);
    assert!(below < need, "{below} would have been enough for {need}");
    // mass of the cell = C × V / energy density
    let bt = p.components.iter().find(|x| x.refdes == "BT1").unwrap();
    assert!((bt.mass_g - c * 3.7 / parts::lipo_mwh_per_g(c)).abs() < 0.01);
    // the bound life requirement is met with the 20 % allowance
    assert!(roll_up(p, "life") >= 16.0 * 1.2 - 1e-9);
    // the computed trade study picks the same cell
    assert!(p.trades[0].decision.starts_with(&fmt(c)), "{} vs {c}", p.trades[0].decision);
    // budget metric equals the roll-up
    let m = all_metrics(p);
    let life = m.iter().find(|x| x.key == "budget.battery_life_h").unwrap().value;
    assert!((life - roll_up(p, "life")).abs() < 1e-9);
}

#[test]
fn coin_cell_sensor_lasts_a_year() {
    let out = gen("iot_sensor", None, &[]);
    let p = &out.project;
    assert!(p.design.battery.chemistry.contains("coin"));
    let life = roll_up(p, "life");
    assert!(life >= 8760.0, "{life}");
    // a camera on a coin cell is flagged and the peak requirement fails
    let cam = gen("iot_sensor", Some(&["coin_cell", "ble", "camera"]), &[]);
    assert!(cam.warnings.iter().any(|w| w.level == "warn" && w.text.contains("Coin cell")));
    let peak = metric(&cam.project, crate::power::metrics, "power.peak_power_mw");
    assert!(peak > 45.0, "{peak}");
}

#[test]
fn robot_mass_power_loop_converges() {
    let out = gen("robot", None, &[]);
    let it = &gen("robot", None, &[]).project; // determinism within the loop
    assert_eq!(serde_json::to_string(it).unwrap(), serde_json::to_string(&out.project).unwrap());
    let m1 = out.project.components.iter().find(|c| c.refdes == "M1").unwrap();
    // motor power = 300 mW + m g Crr v / (2 η)
    let mass = out.project.component_mass_g() / 1000.0;
    let expect = 300.0 + mass * 9.81 * 0.03 * 0.3 / 0.7 * 1000.0;
    assert!((m1.power.active_mw - expect).abs() < 0.05, "{} vs {expect}", m1.power.active_mw);
    // heavier target life → bigger battery → heavier robot → more motor power
    let long = gen("robot", None, &[("life_h", 6.0)]);
    let m1l = long.project.components.iter().find(|c| c.refdes == "M1").unwrap();
    assert!(long.summary.mass_g > out.summary.mass_g);
    assert!(m1l.power.active_mw > m1.power.active_mw);
    // motors dominate: removing them cuts the average power
    let still = gen("robot", Some(&["battery", "usb_c", "ble", "imu", "buttons"]), &[]);
    assert!(still.summary.avg_power_mw < out.summary.avg_power_mw * 0.6);
}

#[test]
fn bracket_beam_hand_calculation() {
    let out = gen("mechanical", None, &[("load_n", 100.0), ("length_mm", 80.0), ("width_mm", 40.0)]);
    let enc = out.project.components.iter().find(|c| c.refdes == "ENC1").unwrap();
    // strength: t ≥ √(6FL / (b σ_allow)), σ_allow = 50 × 0.6 / 2 = 15 MPa ⇒ √(6·100·80/(40·15)) = √80 = 8.94 mm
    // stiffness: δ ≤ 1.6 mm ⇒ t ≥ ∛(4 F L³ / (E δ b)) = ∛(4·100·80³ / (2000·1.6·40)) = ∛1600 = 11.7 → 12 mm
    assert!(enc.name.contains("12 mm"), "{}", enc.name);
    let heavier = gen("mechanical", None, &[("load_n", 400.0), ("length_mm", 80.0), ("width_mm", 40.0)]);
    let e2 = heavier.project.components.iter().find(|c| c.refdes == "ENC1").unwrap();
    assert!(e2.mass_g > enc.mass_g);
    assert!(out.project.requirements.iter().any(|r| r.text.contains("100 N")));
    // no electronics → no power or reliability requirements
    assert!(out
        .project
        .requirements
        .iter()
        .all(|r| !r.verification.metric.as_deref().unwrap_or("").starts_with("reliability")));
}

#[test]
fn thermal_resistance_from_the_enclosure_area() {
    let out = gen("desk_instrument", None, &[]);
    let (l, w, h) = (150.0, 100.0, 60.0);
    let a = 2.0 * (l * w + l * h + w * h) * 1e-6;
    let r = out.project.design.thermal.r_pod_ambient_c_per_w;
    assert!((r - 1.0 / (10.0 * a)).abs() < 0.01, "{r}");
    // bigger box, lower resistance, cooler surface
    let big = gen("desk_instrument", None, &[("length_mm", 300.0), ("width_mm", 200.0)]);
    assert!(big.project.design.thermal.r_pod_ambient_c_per_w < r);
    assert!(big.summary.surface_c.unwrap() < out.summary.surface_c.unwrap());
}

#[test]
fn board_area_from_courtyards() {
    let out = gen("handheld", None, &[]);
    let b = (out.summary.board_mm[0], out.summary.board_mm[1]);
    let parts_area: f64 = super::Ctx::new(&answers("handheld", None, &[]))
        .map(|mut c| {
            c.size();
            c.board.parts_mm2
        })
        .unwrap();
    assert!(b.0 * b.1 >= parts_area * 2.5 - 1.0, "{b:?} vs {parts_area}");
    // the library courtyard of a QFN-48 6 × 6 mm is 7.1 × 7.1 mm
    let p = parts::Part::new("U1", "x", "x", parts::CMP).fp("QFN-48_6x6");
    assert!((p.unit_area_mm2() - 7.1 * 7.1).abs() < 1e-9);
    // Wi-Fi moves to 4 layers
    let wifi = gen("handheld", Some(&["battery", "wifi", "display"]), &[]);
    assert_eq!(wifi.summary.board_layers, 4);
}

#[test]
fn capability_rules() {
    // exclusions keep the first choice
    let o = gen("wearable", Some(&["battery", "coin_cell", "printed", "molded"]), &[]);
    assert!(o.capabilities.contains(&"battery".to_string()) && !o.capabilities.contains(&"coin_cell".to_string()));
    assert!(!o.capabilities.contains(&"molded".to_string()));
    // a battery without a charge input gets USB-C
    let o = gen("handheld", Some(&["battery", "display"]), &[]);
    assert!(o.capabilities.contains(&"usb_c".to_string()));
    // enclosure process follows the quantity
    let few = gen("handheld", Some(&["battery", "usb_c"]), &[("quantity", 100.0)]);
    let many = gen("handheld", Some(&["battery", "usb_c"]), &[("quantity", 50000.0)]);
    assert_eq!(few.summary.enclosure, "printed");
    assert_eq!(many.summary.enclosure, "molded");
    // out-of-range targets are clamped with a warning
    let o = gen("wearable", None, &[("mass_g", -5.0), ("nonsense", 1.0)]);
    assert_eq!(o.targets["mass_g"], 1.0);
    assert!(o.warnings.iter().filter(|w| w.level == "warn").count() >= 2);
    // unknown capability is reported
    let o = gen("wearable", Some(&["battery", "teleporter"]), &[]);
    assert!(o.warnings.iter().any(|w| w.text.contains("teleporter")));
    // smart-home carries the mains scope note
    let o = gen("smart_home", None, &[]);
    assert!(o.warnings.iter().any(|w| w.text.contains("Mains")));
}

/// The metric table, re-checked by experiment on generated projects:
/// generic/conditional metrics move with this project's own data, and the
/// Iris-class metrics are identical for two very different products.
#[test]
fn metric_table_matches_experiment() {
    let base = gen("handheld", Some(&["battery", "usb_c", "ble", "display", "microphones", "imu"]), &[]).project;
    let other = gen("desk_instrument", None, &[]).project;
    let snap = |p: &Project| -> BTreeMap<String, f64> { all_metrics(p).into_iter().map(|m| (m.key, m.value)).collect() };
    let b = snap(&base);
    let o = snap(&other);
    type Pert = fn(&mut Project);
    let perts: Vec<Pert> = vec![
        |p| p.components.iter_mut().for_each(|c| c.power.active_mw *= 1.5),
        |p| p.components.iter_mut().for_each(|c| c.mass_g *= 1.5),
        |p| p.components.iter_mut().for_each(|c| c.unit_cost_usd *= 1.5),
        |p| p.design.battery.capacity_mah *= 1.5,
        |p| p.design.thermal.r_pod_ambient_c_per_w *= 1.5,
        |p| p.design.audio.mic_spacing_mm *= 1.5,
        |p| p.design.audio.fir_taps = 15,
        |p| p.design.battery.nominal_v = 3.0,
        |p| p.design.extra.get_mut("rf").unwrap()["link"]["distance_m"] = serde_json::json!(25.0),
        |p| p.design.extra.get_mut("rf").unwrap()["rad_efficiency"] = serde_json::json!(0.1),
        |p| p.design.extra.get_mut("reliability").unwrap()["hours_per_day"] = serde_json::json!(20.0),
        |p| p.design.pcb.thickness_mm = 0.8,
        |p| p.design.extra.get_mut("rf").unwrap()["network"] = serde_json::Value::Null,
    ];
    let moved: Vec<BTreeMap<String, f64>> = perts
        .iter()
        .map(|f| {
            let mut p = base.clone();
            f(&mut p);
            snap(&p)
        })
        .collect();
    for row in METRIC_TABLE
        .iter()
        .filter(|r| matches!(r.class, MetricClass::Generic | MetricClass::Conditional))
    {
        for k in row.keys.iter().filter(|k| b.contains_key(**k)) {
            let v = b[*k];
            assert!(
                moved
                    .iter()
                    .any(|m| m.get(*k).is_some_and(|x| (x - v).abs() > 1e-9 * v.abs().max(1.0))),
                "{k} did not respond to any perturbation"
            );
        }
    }
    for row in METRIC_TABLE.iter().filter(|r| r.class == MetricClass::Iris) {
        for pat in row.keys {
            let prefix = pat.trim_end_matches('*');
            for (k, v) in b.iter().filter(|(k, _)| {
                if pat.ends_with('*') {
                    k.starts_with(prefix)
                } else {
                    k.as_str() == *pat
                }
            }) {
                if k.starts_with("optics.")
                    || k.starts_with("firmware.")
                    || k.starts_with("chip.")
                    || k.starts_with("cad.frame")
                    || k.starts_with("molding.")
                    || k.starts_with("print.")
                {
                    assert_eq!(Some(v), o.get(k), "{k} should be the same for a handheld and a desk instrument");
                }
            }
        }
    }
    // the table names every metric the engine produces for these projects
    for k in b.keys().chain(o.keys()) {
        let known = METRIC_TABLE.iter().any(|r| {
            r.keys.iter().any(|p| {
                if p.ends_with('*') {
                    k.starts_with(p.trim_end_matches('*'))
                } else {
                    k == p
                }
            })
        });
        assert!(known, "{k} missing from the metric table");
    }
}

/// `cargo test -p forge-core systems::wizard::tests::dump -- --ignored --nocapture`
#[test]
#[ignore]
fn dump() {
    for k in KINDS {
        let t0 = std::time::Instant::now();
        let out = gen(k.id, None, &[]);
        let dt = t0.elapsed();
        let ev = evaluate(&out.project);
        println!("== {} ({dt:?}) caps {:?}", k.id, out.capabilities);
        println!("   summary {:?}", out.summary);
        for (r, res) in out.project.requirements.iter().zip(&ev.results) {
            println!(
                "   {} {:?} {:<70} {:?} {}",
                r.id,
                res.status,
                r.text.chars().take(70).collect::<String>(),
                res.value,
                res.criterion
            );
        }
        for w in &out.warnings {
            println!("   [{}] {}", w.level, w.text);
        }
        for e in &out.explanations {
            println!("   # {} — {}", e.title, e.result);
        }
    }
}
