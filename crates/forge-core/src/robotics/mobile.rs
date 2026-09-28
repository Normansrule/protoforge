//! Differential-drive mobile robot following a spline path with **pure
//! pursuit** (R. C. Coulter, "Implementation of the Pure Pursuit Path
//! Tracking Algorithm", CMU-RI-TR-92-01, 1992).
//!
//! * Path: closed centripetal-free uniform Catmull–Rom spline through the
//!   waypoints, resampled by arc length.
//! * Each control step: find the closest path point (searching forward
//!   from the last one), walk `L` metres ahead along the path to the
//!   *goal point*, express it in the robot frame (`y_r` = lateral offset)
//!   and steer on the circle through it: `κ = 2 y_r / L²`, `ω = v κ`.
//! * Wheel speeds `v_{L,R} = v ∓ ω b/2` are saturated (scaling both to keep
//!   the curvature) and the unicycle `ẋ = v cos θ, ẏ = v sin θ, θ̇ = ω`
//!   is integrated exactly along arcs.
//!
//! A short lookahead tracks tightly but oscillates; a long one cuts
//! corners — try it in the lab.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PursuitIn {
    pub waypoints: Vec<[f64; 2]>,
    pub lookahead: f64,
    pub speed: f64,
    pub wheelbase: f64,
    pub wheel_vmax: f64,
    pub dt: f64,
    pub laps: f64,
    /// Initial pose offset from the first path point: [dx, dy, dθ(deg)].
    pub start_offset: [f64; 3],
    /// Slow down in curves: v = min(v, √(a_lat / |κ|)).
    pub a_lat_max: f64,
}

impl Default for PursuitIn {
    fn default() -> Self {
        PursuitIn {
            waypoints: vec![
                [0.0, 0.0],
                [2.0, -0.4],
                [3.6, 0.6],
                [3.4, 2.4],
                [1.8, 2.2],
                [0.8, 3.2],
                [-1.0, 2.8],
                [-1.6, 1.0],
            ],
            lookahead: 0.5,
            speed: 0.6,
            wheelbase: 0.25,
            wheel_vmax: 0.9,
            dt: 0.02,
            laps: 1.0,
            start_offset: [0.0, -0.4, 20.0],
            a_lat_max: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PursuitOut {
    pub path_x: Vec<f64>,
    pub path_y: Vec<f64>,
    pub path_len: f64,
    pub t: Vec<f64>,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub theta: Vec<f64>,
    pub goal_x: Vec<f64>,
    pub goal_y: Vec<f64>,
    pub v_left: Vec<f64>,
    pub v_right: Vec<f64>,
    pub curvature: Vec<f64>,
    pub cross_track: Vec<f64>,
    pub rms_cross_track: f64,
    pub max_cross_track: f64,
    pub lap_time: f64,
}

fn catmull(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2], p3: [f64; 2], t: f64) -> [f64; 2] {
    let t2 = t * t;
    let t3 = t2 * t;
    let f = |a: f64, b: f64, c: f64, d: f64| {
        0.5 * (2.0 * b + (-a + c) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (-a + 3.0 * b - 3.0 * c + d) * t3)
    };
    [f(p0[0], p1[0], p2[0], p3[0]), f(p0[1], p1[1], p2[1], p3[1])]
}

/// Closed spline through the waypoints resampled every `ds` metres.
pub fn spline_path(wp: &[[f64; 2]], ds: f64) -> Vec<[f64; 2]> {
    let n = wp.len();
    if n < 3 {
        return wp.to_vec();
    }
    let mut dense = Vec::new();
    for i in 0..n {
        let (p0, p1, p2, p3) = (wp[(i + n - 1) % n], wp[i], wp[(i + 1) % n], wp[(i + 2) % n]);
        for k in 0..40 {
            dense.push(catmull(p0, p1, p2, p3, k as f64 / 40.0));
        }
    }
    dense.push(dense[0]);
    // resample by arc length
    let mut out = vec![dense[0]];
    let mut acc = 0.0;
    for w in dense.windows(2) {
        let seg = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
        acc += seg;
        while acc >= ds {
            acc -= ds;
            let f = 1.0 - acc / seg.max(1e-12);
            out.push([w[0][0] + f * (w[1][0] - w[0][0]), w[0][1] + f * (w[1][1] - w[0][1])]);
        }
    }
    out
}

pub fn pure_pursuit(i: &PursuitIn) -> Result<PursuitOut, String> {
    if i.waypoints.len() < 3 {
        return Err("need at least 3 waypoints".into());
    }
    let ds = 0.01;
    let path = spline_path(&i.waypoints, ds);
    let np = path.len();
    let total = np as f64 * ds;
    let la = i.lookahead.clamp(0.05, 10.0);
    let la_n = (la / ds).round() as usize;
    let mut out = PursuitOut {
        path_x: path.iter().map(|p| p[0]).collect(),
        path_y: path.iter().map(|p| p[1]).collect(),
        path_len: total,
        ..Default::default()
    };
    // initial pose: first point + offset, heading along the path
    let h0 = (path[1][1] - path[0][1]).atan2(path[1][0] - path[0][0]);
    let (mut x, mut y, mut th) = (
        path[0][0] + i.start_offset[0],
        path[0][1] + i.start_offset[1],
        h0 + i.start_offset[2].to_radians(),
    );
    let dt = i.dt.clamp(0.001, 0.2);
    let mut idx = 0usize;
    let mut progress = 0usize; // total path samples advanced (for lap counting)
    let target = (i.laps.clamp(0.1, 5.0) * np as f64) as usize;
    let t_max = 3.0 * i.laps.max(0.1) * total / i.speed.max(0.01) + 20.0;
    let mut t = 0.0;
    let mut sse = 0.0;
    let mut cnt = 0usize;
    while t < t_max {
        // closest point in a forward window
        let mut best = (f64::INFINITY, idx);
        let win = if t == 0.0 { np } else { (la_n * 3).max(50).min(np) };
        for k in 0..win {
            let j = (idx + k) % np;
            let d = (path[j][0] - x).powi(2) + (path[j][1] - y).powi(2);
            if d < best.0 {
                best = (d, j);
            }
        }
        let adv = (best.1 + np - idx) % np;
        if t > 0.0 {
            progress += adv;
        }
        idx = best.1;
        let cte = best.0.sqrt();
        // signed cross-track (left positive) using the path tangent
        let j2 = (idx + 1) % np;
        let (tx, ty) = (path[j2][0] - path[idx][0], path[j2][1] - path[idx][1]);
        let sign = if tx * (y - path[idx][1]) - ty * (x - path[idx][0]) >= 0.0 {
            1.0
        } else {
            -1.0
        };
        let g = path[(idx + la_n) % np];
        let (dx, dy) = (g[0] - x, g[1] - y);
        let yr = -th.sin() * dx + th.cos() * dy;
        let l2 = (dx * dx + dy * dy).max(1e-9);
        let kappa = 2.0 * yr / l2;
        let mut v = i.speed;
        if i.a_lat_max > 0.0 && kappa.abs() > 1e-9 {
            v = v.min((i.a_lat_max / kappa.abs()).sqrt());
        }
        let mut w = v * kappa;
        let (mut vl, mut vr) = (v - w * i.wheelbase / 2.0, v + w * i.wheelbase / 2.0);
        let m = vl.abs().max(vr.abs());
        if m > i.wheel_vmax && m > 0.0 {
            let s = i.wheel_vmax / m;
            vl *= s;
            vr *= s;
        }
        v = (vl + vr) / 2.0;
        w = (vr - vl) / i.wheelbase;
        out.t.push(t);
        out.x.push(x);
        out.y.push(y);
        out.theta.push(th);
        out.goal_x.push(g[0]);
        out.goal_y.push(g[1]);
        out.v_left.push(vl);
        out.v_right.push(vr);
        out.curvature.push(kappa);
        out.cross_track.push(sign * cte);
        if t > 1.0 {
            sse += cte * cte;
            cnt += 1;
            out.max_cross_track = out.max_cross_track.max(cte);
        }
        // exact arc integration
        if w.abs() < 1e-9 {
            x += v * dt * th.cos();
            y += v * dt * th.sin();
        } else {
            let r = v / w;
            x += r * ((th + w * dt).sin() - th.sin());
            y -= r * ((th + w * dt).cos() - th.cos());
        }
        th += w * dt;
        t += dt;
        if progress >= target {
            break;
        }
    }
    out.rms_cross_track = if cnt > 0 { (sse / cnt as f64).sqrt() } else { 0.0 };
    out.lap_time = t / i.laps.clamp(0.1, 5.0);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_the_path() {
        let r = pure_pursuit(&PursuitIn::default()).unwrap();
        assert!(r.rms_cross_track < 0.15, "rms {}", r.rms_cross_track);
        assert!(r.lap_time > 0.0 && r.lap_time.is_finite());
    }
}
