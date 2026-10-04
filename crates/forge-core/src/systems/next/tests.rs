use super::levers::{self, LeversIn, SolveIn};
use super::*;
use crate::systems::templates;

fn iris_actions() -> (Project, NextActions) {
    let p = Project::iris();
    let out = next_actions(&NextIn {
        project: p.clone(),
        evaluation: None,
        levers: None,
        limit: None,
    })
    .unwrap();
    (p, out)
}

fn pos(out: &NextActions, id: &str) -> usize {
    out.actions
        .iter()
        .position(|a| a.id == id)
        .unwrap_or_else(|| panic!("no action {id}"))
}

#[test]
fn iris_open_goals_rank_above_marginal_passes() {
    let (_, out) = iris_actions();
    let s = &out.summary;
    assert_eq!(s.requirements, 53);
    assert_eq!(s.pass, 50);
    assert_eq!(s.open_goals, 3);
    assert_eq!(s.failing, 0);
    assert!(s.marginal >= 5, "{s:?}");
    assert_eq!(s.pass_with_margin + s.marginal, s.pass);
    let goals: Vec<usize> = ["goal:R-030", "goal:R-047", "goal:R-053"].iter().map(|id| pos(&out, id)).collect();
    let first_marginal = out
        .actions
        .iter()
        .position(|a| a.kind == "marginal")
        .expect("Iris has marginal passes");
    for g in &goals {
        assert!(*g < first_marginal, "goal at {g} below the first marginal pass at {first_marginal}");
    }
    // scores are sorted and kinds never interleave
    for w in out.actions.windows(2) {
        assert!(w[0].score >= w[1].score);
        assert!(base(&w[0].kind) >= base(&w[1].kind));
    }
    // the safety requirement with the thinnest margin (R-039, 1.5 %) leads the marginal passes,
    // ahead of R-048 (0.4 %, not safety)
    assert_eq!(out.actions[first_marginal].requirements, vec!["R-039".to_string()]);
    let r48 = pos(&out, "marginal:R-048");
    assert!(r48 < pos(&out, "marginal:R-005"));
    // a goal carries its value, target and margin
    let a = &out.actions[pos(&out, "goal:R-047")];
    assert_eq!(a.metric.as_deref(), Some("ergonomics.outdoor_contrast"));
    assert!((a.value.unwrap() - 1.2366).abs() < 1e-3);
    assert_eq!(a.target, Some(1.5));
    assert!(a.margin.unwrap() < 0.0);
    assert_eq!(a.severity, "medium");
    assert!(a.title.contains("R-047"));
}

#[test]
fn every_route_exists_in_the_registry() {
    let reg = include_str!("../../../../../web/src/labs/registry.ts");
    let (_, out) = iris_actions();
    assert!(!out.actions.is_empty());
    for a in &out.actions {
        let r = &a.location.route;
        assert!(
            r == "/systems" || reg.contains(&format!("path: '{r}'")),
            "{}: route {r} is not in registry.ts",
            a.id
        );
        assert!(a.location.href.starts_with(r.as_str()));
    }
    let a = &out.actions[pos(&out, "goal:R-047")];
    assert_eq!(a.location.href, "/ergonomics?tab=display");
    let a = &out.actions[pos(&out, "goal:R-030")];
    assert_eq!(a.location.href, "/optics?tab=quality");
    let a = &out.actions[pos(&out, "goal:R-053")];
    assert_eq!(a.location.href, "/program?tab=readiness");
}

#[test]
fn levers_have_the_right_sign() {
    let (p, out) = iris_actions();
    // display luminance raises outdoor contrast — and by the hand-calculated amount:
    // ACR = 1 + L·k, so a 5 % step in L moves ACR by 0.05 × (ACR − 1)
    let a = &out.actions[pos(&out, "goal:R-047")];
    assert_eq!(a.levers_status, "measured");
    let l = a
        .levers
        .iter()
        .find(|l| l.param == "optics.display_luminance_nits")
        .expect("luminance lever");
    assert_eq!(l.direction, "increase");
    let acr = a.value.unwrap();
    assert!((l.delta - 0.05 * (acr - 1.0)).abs() < 1e-6, "{} vs {}", l.delta, 0.05 * (acr - 1.0));
    assert!(l.estimate.unwrap() > l.value);
    // the combiner's reflectance also raises it
    assert!(a
        .levers
        .iter()
        .any(|l| l.param == "optics.combiner_reflectance" && l.direction == "increase"));
    // a bigger battery raises battery life (power lab), ∝ capacity
    let lv = levers::levers_for_metric(&LeversIn {
        project: p.clone(),
        metric: "power.battery_life_h".into(),
        requirement_id: None,
        limit: None,
    })
    .unwrap();
    assert_eq!(lv.requirement_id.as_deref(), Some("R-002"));
    let b = lv
        .levers
        .iter()
        .find(|l| l.param == "battery.capacity_mah")
        .expect("capacity lever");
    assert_eq!(b.direction, "increase");
    assert!(b.delta > 0.0);
    // a more efficient regulator raises it too
    assert!(lv
        .levers
        .iter()
        .any(|l| l.param == "power_rails.buck_efficiency" && l.delta > 0.0 && l.direction == "increase"));
    // a bigger frame wall adds mass: decrease it to add margin to R-001 (≤ 50 g)
    let a = &out.actions[pos(&out, "marginal:R-001")];
    assert_eq!(a.module.as_deref(), Some("systems"));
    let w = a.levers.iter().find(|l| l.param == "frame.wall_mm").expect("wall lever");
    assert!(w.delta > 0.0 && w.direction == "decrease");
    // no lever invents a parameter the project does not have
    for a in &out.actions {
        for l in &a.levers {
            assert!(crate::systems::sensitivity::get_param(&p, &l.param).is_some(), "{}", l.param);
        }
    }
    // the readiness goal has no numeric design lever: it is evidence-driven
    let a = &out.actions[pos(&out, "goal:R-053")];
    assert_eq!(a.levers_status, "none");
    // FEA-backed metrics are measured on request
    let a = &out.actions[pos(&out, "marginal:R-026")];
    assert_eq!(a.levers_status, "lazy");
}

#[test]
fn solve_r047_with_display_luminance() {
    let p = Project::iris();
    let s = levers::solve(&SolveIn {
        project: p.clone(),
        requirement_id: "R-047".into(),
        param: "optics.display_luminance_nits".into(),
        lo: None,
        hi: None,
        couple: true,
        margin: None,
        evaluation: None,
    })
    .unwrap();
    assert!(s.found, "{}", s.message);
    // hand calculation: ACR = 1 + L·k with k = (ACR0 − 1)/L0; 1.5 × 1.02 needs L = 0.53/k
    let k = (s.before_value.unwrap() - 1.0) / 3000.0;
    let l_hand = 0.53 / k;
    assert!(
        s.after_param >= l_hand && s.after_param < l_hand * 1.01,
        "{} vs {l_hand}",
        s.after_param
    );
    assert!(s.after_value.unwrap() >= 1.53 - 1e-9);
    let me = s.side_effects.iter().find(|x| x.id == "R-047").unwrap();
    assert!(me.verdict_changed && me.better);
    assert_eq!(s.open_goals_after, s.open_goals_before - 1);
    // the display's power scales with luminance (stated assumption) → power and thermal move
    let coupled: Vec<_> = s.changes.iter().filter(|c| c.assumption.is_some()).collect();
    assert_eq!(coupled.len(), 1);
    assert_eq!(coupled[0].refdes.as_deref(), Some("U2"));
    let power_or_thermal = |x: &levers::SideEffect| {
        x.metric
            .as_deref()
            .is_some_and(|m| m.starts_with("power.") || m.starts_with("thermal.") || m.starts_with("fluids."))
    };
    assert!(
        s.side_effects.iter().filter(|x| power_or_thermal(x)).count() >= 2,
        "{:#?}",
        s.side_effects
    );
    assert!(s.side_effects.iter().filter(|x| power_or_thermal(x)).all(|x| !x.better));
    // without the coupling the engine itself does not tie luminance to power
    let s2 = levers::solve(&SolveIn {
        project: p,
        requirement_id: "R-047".into(),
        param: "optics.display_luminance_nits".into(),
        lo: None,
        hi: None,
        couple: false,
        margin: None,
        evaluation: None,
    })
    .unwrap();
    assert_eq!(s2.after_param, s.after_param);
    assert_eq!(s2.changes.len(), 1);
    assert!(!s2.side_effects.iter().any(power_or_thermal));
}

#[test]
fn solve_reports_no_solution_honestly() {
    let p = Project::iris();
    // readiness does not depend on the battery
    let s = levers::solve(&SolveIn {
        project: p.clone(),
        requirement_id: "R-053".into(),
        param: "battery.capacity_mah".into(),
        lo: None,
        hi: None,
        couple: true,
        margin: None,
        evaluation: None,
    })
    .unwrap();
    assert!(!s.found);
    assert!(s.message.starts_with("No value"), "{}", s.message);
    assert_eq!(s.after_param, s.before_param);
    assert!(s.side_effects.is_empty());
    // and an already-met requirement needs no change
    let s = levers::solve(&SolveIn {
        project: p.clone(),
        requirement_id: "R-002".into(),
        param: "battery.capacity_mah".into(),
        lo: None,
        hi: None,
        couple: true,
        margin: None,
        evaluation: None,
    })
    .unwrap();
    assert!(s.found && s.after_param == s.before_param, "{}", s.message);
    // unknown inputs are errors, not panics
    let bad = SolveIn {
        project: p,
        requirement_id: "R-002".into(),
        param: "frame.material".into(),
        lo: None,
        hi: None,
        couple: true,
        margin: None,
        evaluation: None,
    };
    assert!(levers::solve(&bad).is_err());
}

#[test]
fn battery_solve_finds_capacity_and_couples_mass() {
    // tighten R-002 so the battery has to grow, then check the solver and the mass side effect
    let mut p = Project::iris();
    let r = p.requirements.iter_mut().find(|r| r.id == "R-002").unwrap();
    r.verification.target = 16.0;
    let s = levers::solve(&SolveIn {
        project: p,
        requirement_id: "R-002".into(),
        param: "battery.capacity_mah".into(),
        lo: None,
        hi: None,
        couple: true,
        margin: None,
        evaluation: None,
    })
    .unwrap();
    assert!(s.found, "{}", s.message);
    // battery life ∝ capacity in the power lab: 240 mAh × 16 × 1.02 / 13.78 h
    let hand = 240.0 * 16.0 * 1.02 / s.before_value.unwrap();
    assert!(s.after_param >= hand && s.after_param < hand * 1.01, "{} vs {hand}", s.after_param);
    assert!(s.changes.iter().any(|c| c.refdes.as_deref() == Some("BT1") && c.path == "mass_g"));
    // R-001 (≤ 50 g, 1.8 % margin) fails with the heavier cell
    let r1 = s.side_effects.iter().find(|x| x.id == "R-001").expect("mass moves");
    assert!(r1.verdict_changed && !r1.better);
}

#[test]
fn templates_run_without_panics() {
    for t in templates::templates() {
        let out = next_actions(&NextIn {
            project: t.project.clone(),
            evaluation: None,
            levers: Some("inline".into()),
            limit: Some(20),
        })
        .unwrap_or_else(|e| panic!("{}: {e}", t.id));
        assert!(out.actions.len() <= 20);
        assert_eq!(out.summary.requirements, t.project.requirements.len());
        for a in &out.actions {
            assert!(a.score.is_finite() && !a.title.is_empty() && !a.why.is_empty(), "{}: {a:?}", t.id);
        }
        if t.id == "blank" {
            assert_eq!(out.summary.readiness_pct, 0.0);
        }
    }
}

#[test]
fn dispatch_and_lazy_levers() {
    let p = Project::iris();
    let v = super::dispatch(
        "systems.next_actions",
        &serde_json::json!({ "project": p, "levers": "none", "limit": 3 }),
    )
    .unwrap()
    .unwrap();
    assert_eq!(v["actions"].as_array().unwrap().len(), 3);
    assert!(v["actions"][0]["where"]["route"].is_string());
    // lazy lever fetch for a cheap module works through the dispatcher
    let v = super::dispatch(
        "systems.next_levers",
        &serde_json::json!({ "project": p, "metric": "mechanisms.hinge_spring_sf" }),
    )
    .unwrap()
    .unwrap();
    assert_eq!(v["module"], "mechanisms");
    assert!(!v["levers"].as_array().unwrap().is_empty());
    assert!(crate::api::METHODS.contains(&"systems.solve_lever"));
}

#[test]
fn iris_list_is_interactive() {
    // Debug builds are ~10–30× slower than the release WebAssembly engine;
    // this bounds the cold (unmemoised) cost and prints it for the docs.
    let p = Project::iris();
    let ev = evaluate(&p);
    let t = std::time::Instant::now();
    let out = next_actions(&NextIn {
        project: p.clone(),
        evaluation: Some(ev.clone()),
        levers: None,
        limit: None,
    })
    .unwrap();
    let cold = t.elapsed().as_secs_f64();
    let t = std::time::Instant::now();
    let _ = next_actions(&NextIn {
        project: p,
        evaluation: Some(ev),
        levers: None,
        limit: None,
    })
    .unwrap();
    let warm = t.elapsed().as_secs_f64();
    println!(
        "next_actions on Iris (debug): cold {:.0} ms, memoised {:.0} ms, {} actions, levers measured in {:?}",
        cold * 1e3,
        warm * 1e3,
        out.actions.len(),
        out.summary.levers_measured
    );
    assert!(cold < 30.0, "cold next_actions took {cold:.1} s in debug");
    assert!(warm < cold);
}

#[test]
fn describes_parameters() {
    assert_eq!(
        levers::describe("optics.display_luminance_nits"),
        ("Optics · display luminance".to_string(), "cd/m²".to_string())
    );
    assert_eq!(
        levers::describe("optics.px_w"),
        ("Optics · horizontal pixels".to_string(), "px".to_string())
    );
    assert_eq!(
        levers::describe("extra.mechanisms.hinge.wire_d_mm"),
        ("Mechanisms · hinge wire diameter".to_string(), "mm".to_string())
    );
    assert_eq!(
        levers::describe("extra.rf.network.0.value"),
        ("Radio (RF) · network #1 component value".to_string(), String::new())
    );
    assert_eq!(levers::describe("battery.capacity_mah").1, "mAh");
    assert_eq!(levers::bounds("optics.combiner_reflectance", 0.3, false), (1e-3, 0.999));
    let (step, down) = levers::step_for("power_rails.buck_efficiency", 0.98, false);
    assert!(down && (step - 0.049).abs() < 1e-12);
    assert_eq!(levers::step_for("pcb.layers", 2.0, true), (1.0, false));
}

/// The body of the ready-to-paste `protoforge next [project.json] [--all] [--levers all|none]`
/// CLI command (forge-cli is owned elsewhere), kept here so it stays compiling.
fn cli_next(p: &Project, all: bool, levers: Option<&str>) -> Result<(String, bool), String> {
    let out = next_actions(&NextIn {
        project: p.clone(),
        evaluation: None,
        levers: levers.map(String::from),
        limit: None,
    })?;
    let s = &out.summary;
    let mut o = format!("{} v{} — what to do next\n", p.meta.name, p.meta.version);
    o += &format!(
        "{:.0} % ready: {} of {} requirements pass with ≥ {:.0} % margin · {} marginal · {} open goal(s) · {} failing · {} unverified\n\n",
        s.readiness_pct,
        s.pass_with_margin,
        s.requirements,
        s.marginal_threshold * 100.0,
        s.marginal,
        s.open_goals,
        s.failing,
        s.unverified
    );
    let shown = if all { out.actions.len() } else { out.actions.len().min(10) };
    for (k, a) in out.actions.iter().take(shown).enumerate() {
        o += &format!(
            "{:>2}. [{}] {} · score {:.1}\n",
            k + 1,
            a.severity.to_uppercase(),
            a.kind.replace('_', " "),
            a.score
        );
        o += &format!("    {}\n", a.title);
        if let (Some(v), Some(c)) = (a.value, &a.criterion) {
            o += &format!(
                "    now {} {} → {}{}\n",
                levers::fmt_num(v),
                a.unit,
                c,
                a.margin.map(|m| format!(" (margin {:+.1} %)", m * 100.0)).unwrap_or_default()
            );
        }
        o += &format!("    go to: {}  (#{})\n", a.location.label, a.location.href);
        match a.levers_status.as_str() {
            "measured" => {
                for l in &a.levers {
                    o += &format!(
                        "    {} {}: +{} {} moves it {:+.4} {}{}\n",
                        if l.direction == "increase" { "↑" } else { "↓" },
                        l.label,
                        levers::fmt_num(l.step),
                        l.unit,
                        l.delta,
                        a.unit,
                        l.estimate
                            .map(|e| format!(" · ≈ {} {} reaches the target", levers::fmt_num(e), l.unit))
                            .unwrap_or_default()
                    );
                }
            }
            "lazy" => o += "    levers: run with --levers all (re-runs a finite-element / layout model per parameter)\n",
            "none" => o += "    levers: no numeric design parameter moves this metric measurably\n",
            _ => {}
        }
        o += "\n";
    }
    if shown < out.actions.len() {
        o += &format!("… {} more (use --all)\n", out.actions.len() - shown);
    }
    Ok((o, s.failing == 0))
}

#[test]
fn cli_rendering_compiles_and_reads_well() {
    let (text, ok) = cli_next(&Project::iris(), false, None).unwrap();
    assert!(ok);
    assert!(text.contains("77 % ready: 41 of 53"));
    assert!(text.contains("go to: Human Factors Lab → Legibility & eyebox  (#/ergonomics?tab=display)"));
    println!("{text}");
}
