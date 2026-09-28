//! # Robotics engine (`robotics.*`)
//!
//! * [`arm`] — planar 2-link / SCARA kinematics: FK, analytic IK (elbow
//!   up/down), Jacobian, manipulability ellipse, workspace.
//! * [`traj`] — trapezoidal, cubic and quintic profiles; synchronised
//!   multi-joint moves; Cartesian straight lines via sampled IK.
//! * [`pnp`] — pick-and-place of the Iris main board (feeders, sequencing,
//!   timing, full timeline). Produces `robotics.pnp_cycle_s`.
//! * [`mobile`] — differential-drive robot with pure-pursuit path tracking.
//!
//! References: Siciliano, Sciavicco, Villani & Oriolo, *Robotics:
//! Modelling, Planning and Control* (2009); Lynch & Park, *Modern Robotics*
//! (2017, modernrobotics.org); Biagiotti & Melchiorri, *Trajectory Planning
//! for Automatic Machines and Robots* (2008); Coulter, CMU-RI-TR-92-01 (1992).

#![allow(clippy::needless_range_loop)] // index-heavy numeric code reads better with explicit indices

pub mod arm;
pub mod mobile;
pub mod pnp;
pub mod traj;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Metric, Project};
use arm::Arm2;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct ArmIn {
    arm: Arm2,
    x: f64,
    y: f64,
    elbow_up: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct WorkspaceIn {
    arm: Arm2,
    n: usize,
}

#[derive(Serialize)]
struct WorkspaceOut {
    points: Vec<[f64; 2]>,
    r_min: f64,
    r_max: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
struct JointMoveIn {
    q_start: Vec<f64>,
    q_goal: Vec<f64>,
    v_max: Vec<f64>,
    a_max: Vec<f64>,
    samples: usize,
}

impl Default for JointMoveIn {
    fn default() -> Self {
        JointMoveIn {
            q_start: vec![0.0, 0.0],
            q_goal: vec![90.0, -45.0],
            v_max: vec![180.0, 240.0],
            a_max: vec![720.0, 960.0],
            samples: 200,
        }
    }
}

#[derive(Serialize, Default)]
struct JointMoveOut {
    duration: f64,
    t: Vec<f64>,
    q: Vec<Vec<f64>>,
    qd: Vec<Vec<f64>>,
    qdd: Vec<Vec<f64>>,
    /// Each joint's own minimum time before synchronisation.
    own_min_time: Vec<f64>,
}

fn joint_move(i: JointMoveIn) -> Result<JointMoveOut, String> {
    let n = i.q_start.len();
    if i.q_goal.len() != n || i.v_max.len() != n || i.a_max.len() != n {
        return Err("q_start, q_goal, v_max, a_max must have the same length".into());
    }
    let d: Vec<f64> = (0..n).map(|k| i.q_goal[k] - i.q_start[k]).collect();
    let (t, profs) = traj::synchronized(&d, &i.v_max, &i.a_max);
    let m = i.samples.clamp(10, 5000);
    let mut out = JointMoveOut {
        duration: t,
        q: vec![vec![]; n],
        qd: vec![vec![]; n],
        qdd: vec![vec![]; n],
        ..Default::default()
    };
    out.own_min_time = (0..n)
        .map(|k| traj::Trapezoid::min_time(d[k], i.v_max[k], i.a_max[k]).t_total)
        .collect();
    for s in 0..m {
        let tt = t * s as f64 / (m - 1) as f64;
        out.t.push(tt);
        for k in 0..n {
            let (p, v, a) = profs[k].eval(tt);
            out.q[k].push(i.q_start[k] + p);
            out.qd[k].push(v);
            out.qdd[k].push(a);
        }
    }
    Ok(out)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
struct LineIn {
    arm: Arm2,
    from: [f64; 2],
    to: [f64; 2],
    v_max: f64,
    a_max: f64,
    elbow_up: bool,
    samples: usize,
}

impl Default for LineIn {
    fn default() -> Self {
        LineIn {
            arm: Arm2::default(),
            from: [1.2, -0.6],
            to: [0.4, 1.0],
            v_max: 0.8,
            a_max: 2.0,
            elbow_up: false,
            samples: 150,
        }
    }
}

#[derive(Serialize, Default)]
struct LineOut {
    duration: f64,
    t: Vec<f64>,
    x: Vec<f64>,
    y: Vec<f64>,
    q1: Vec<f64>,
    q2: Vec<f64>,
    q1d: Vec<f64>,
    q2d: Vec<f64>,
    manipulability: Vec<f64>,
    reachable: bool,
    /// The same move done as a straight line in joint space (for contrast).
    joint_x: Vec<f64>,
    joint_y: Vec<f64>,
    peak_joint_speed_dps: f64,
}

fn cartesian_line(i: LineIn) -> Result<LineOut, String> {
    let dx = i.to[0] - i.from[0];
    let dy = i.to[1] - i.from[1];
    let len = (dx * dx + dy * dy).sqrt();
    let tr = traj::Trapezoid::min_time(len, i.v_max, i.a_max);
    let m = i.samples.clamp(10, 2000);
    let mut out = LineOut {
        duration: tr.t_total,
        reachable: true,
        ..Default::default()
    };
    let mut prev: Option<[f64; 2]> = None;
    let dt = tr.t_total.max(1e-9) / (m - 1) as f64;
    for s in 0..m {
        let t = s as f64 * dt;
        let f = if len > 0.0 { tr.eval(t).0 / len } else { 0.0 };
        let (x, y) = (i.from[0] + f * dx, i.from[1] + f * dy);
        match i.arm.ik(x, y, i.elbow_up) {
            Some(q) => {
                let (q1d, q2d) = match prev {
                    Some(p) => (arm::wrap_pi(q[0] - p[0]).to_degrees() / dt, (q[1] - p[1]).to_degrees() / dt),
                    None => (0.0, 0.0),
                };
                out.peak_joint_speed_dps = out.peak_joint_speed_dps.max(q1d.abs()).max(q2d.abs());
                out.t.push(t);
                out.x.push(x);
                out.y.push(y);
                out.q1.push(q[0].to_degrees());
                out.q2.push(q[1].to_degrees());
                out.q1d.push(q1d);
                out.q2d.push(q2d);
                out.manipulability.push(i.arm.manipulability(q[1]));
                prev = Some(q);
            }
            None => {
                out.reachable = false;
            }
        }
    }
    if let (Some(a), Some(b)) = (i.arm.ik(i.from[0], i.from[1], i.elbow_up), i.arm.ik(i.to[0], i.to[1], i.elbow_up)) {
        let d1 = arm::wrap_pi(b[0] - a[0]);
        for s in 0..m {
            let f = s as f64 / (m - 1) as f64;
            let (_, tip) = i.arm.fk(a[0] + f * d1, a[1] + f * (b[1] - a[1]));
            out.joint_x.push(tip[0]);
            out.joint_y.push(tip[1]);
        }
    }
    Ok(out)
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct PnpIn {
    project: Option<Project>,
    config: pnp::PnpConfig,
}

/// JSON API entry point for `robotics.*` methods.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "robotics.arm" => handle(input, |i: ArmIn| Ok(arm::query(&i.arm, i.x, i.y, i.elbow_up))),
        "robotics.workspace" => handle(input, |i: WorkspaceIn| {
            let n = if i.n == 0 { 40 } else { i.n };
            Ok(WorkspaceOut {
                points: arm::workspace(&i.arm, n),
                r_min: (i.arm.l1 - i.arm.l2).abs(),
                r_max: i.arm.l1 + i.arm.l2,
            })
        }),
        "robotics.profiles" => handle(input, |i: traj::ProfilesIn| Ok(traj::profiles(&i))),
        "robotics.joint_move" => handle(input, joint_move),
        "robotics.cartesian_line" => handle(input, cartesian_line),
        "robotics.pnp" => handle(input, |i: PnpIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            Ok(pnp::plan(&p, &i.config, true))
        }),
        "robotics.pursuit" => handle(input, |i: mobile::PursuitIn| mobile::pure_pursuit(&i)),
        _ => None,
    }
}

thread_local! {
    static METRICS_CACHE: crate::memo::Slot<Vec<Metric>> = const { std::cell::RefCell::new(None) };
}

/// Metrics this module contributes to requirement verification (memoised on
/// the inputs it reads, because verification re-runs on every project edit).
pub fn metrics(p: &Project) -> Vec<Metric> {
    let key = crate::memo::key(&(&p.components, &p.design.pcb, &p.design.audio, p.design.extra.get("pnp")));
    crate::memo::memo(&METRICS_CACHE, key, || metrics_uncached(p))
}

fn metrics_uncached(p: &Project) -> Vec<Metric> {
    if !p.components.iter().any(|c| c.on_pcb) {
        return vec![];
    }
    // the lab stores its cell settings in design.extra.pnp (defaults otherwise)
    let cfg: pnp::PnpConfig = p
        .design
        .extra
        .get("pnp")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let plan = pnp::plan(p, &cfg, false);
    let b = &plan.breakdown;
    vec![Metric::new("robotics.pnp_cycle_s", "Pick-and-place cycle time per board", plan.cycle_s, "s", "robotics").with_note(&format!(
        "{} parts from {} feeders on a desktop SCARA: travel {:.1} s, Z {:.1} s, vision {:.1} s, dwell {:.1} s, handling {:.1} s ({:.0} CPH; naïve BOM order {:.1} s)",
        plan.placements.len(),
        plan.feeders.len(),
        b.travel,
        b.z,
        b.vision,
        b.dwell,
        b.handling,
        plan.cph,
        plan.naive_cycle_s
    ))]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_metric() {
        let t0 = std::time::Instant::now();
        let m = metrics(&Project::iris());
        let el = t0.elapsed();
        assert_eq!(m[0].key, "robotics.pnp_cycle_s");
        assert!(m[0].value > 5.0 && m[0].value < 120.0, "{}", m[0].value);
        println!("pnp {} s ({:?}) — {}", m[0].value, el, m[0].note);
    }

    #[test]
    fn line_and_joint_move() {
        let r = cartesian_line(LineIn::default()).unwrap();
        assert!(r.reachable && !r.t.is_empty());
        let j = joint_move(JointMoveIn::default()).unwrap();
        assert!((j.q[0].last().unwrap() - 90.0).abs() < 1e-9);
    }
}
