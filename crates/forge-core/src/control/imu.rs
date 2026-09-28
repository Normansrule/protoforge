//! Iris head tracking: a synthetic head-motion scenario, a 6-axis IMU model
//! and four attitude estimators.
//!
//! **Frames.** World = NED (x forward/north, y right/east, z down); body =
//! x forward, y right, z down. Attitude is ZYX Euler (yaw ψ, pitch θ, roll
//! φ), `R = Rz(ψ)·Ry(θ)·Rx(φ)` (body → world).
//!
//! **Sensors.** The gyro measures body rate `ω = E(φ,θ)·[φ̇ θ̇ ψ̇]` plus a
//! constant bias and white noise of density `N_g` (per-sample σ = N_g·√f_s).
//! The accelerometer measures *specific force* `f = Rᵀ(a − g)` of the IMU
//! location — including the linear acceleration of walking and of the IMU
//! swinging about the neck (lever arm), which is what corrupts tilt.
//!
//! **Estimators** (all run at the IMU rate, all start aligned):
//! * gyro-only: integrate Euler rates → drifts at the bias rate;
//! * accel-only tilt: `φ = atan2(−f_y, −f_z)`, `θ = atan2(f_x, √(f_y²+f_z²))`
//!   → noisy and wrong whenever the head accelerates;
//! * complementary filter: `x̂ ← α (x̂ + ẋ_gyro Δt) + (1 − α) x_acc`, i.e. a
//!   high-pass on the gyro path and a low-pass on the accel path with a
//!   crossover `f_c = (1 − α)/(2π α Δt)` (Mahony, Hamel & Pflimlin 2008 is
//!   the rigorous SO(3) version);
//! * Kalman filter (Kalman 1960) per axis with state `[angle, gyro bias]`,
//!   process noise from the gyro noise density and a bias random walk, and a
//!   measurement noise that includes an allowance for motion acceleration.
//!
//! Yaw is **unobservable** from gravity alone (rotating about the vertical
//! does not change the gravity vector), so without a magnetometer every
//! estimator's yaw is pure gyro integration and drifts.

use serde::{Deserialize, Serialize};

use crate::math::{Rng, PI};
use crate::project::ImuParams;

const G: f64 = 9.80665;

/// Minimum-jerk blend 0→1 over [t0, t0+d].
fn mj(t: f64, t0: f64, d: f64) -> f64 {
    let s = ((t - t0) / d).clamp(0.0, 1.0);
    s * s * s * (10.0 - 15.0 * s + 6.0 * s * s)
}

/// A move from `a` to `b` degrees starting at `t0` lasting `d` seconds.
fn mv(t: f64, t0: f64, d: f64, a: f64, b: f64) -> f64 {
    (b - a) * mj(t, t0, d)
}

/// Walking envelope (0 → 1 → 0) between 20 s and 45 s.
fn walk_env(t: f64) -> f64 {
    mj(t, 20.0, 1.0) - mj(t, 44.0, 1.0)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HeadPose {
    /// roll, pitch, yaw in radians
    pub rpy: [f64; 3],
    /// neck-pivot position in world, m
    pub pos: [f64; 3],
}

/// Deterministic ~60 s head-motion scenario (angles in degrees internally).
pub fn head_pose(t: f64) -> HeadPose {
    let env = walk_env(t);
    let w2 = 2.0 * PI * 2.0; // step frequency 2 Hz
    let w1 = 2.0 * PI * 1.0; // stride (left/right) 1 Hz
    // yaw: look right, left, centre; scan while walking; look around
    let yaw = mv(t, 4.0, 1.0, 0.0, 40.0) + mv(t, 8.0, 1.2, 0.0, -70.0) + mv(t, 12.0, 1.0, 0.0, 30.0)
        + 3.0 * env * (w1 * t).sin()
        + mv(t, 47.0, 1.2, 0.0, 60.0)
        + mv(t, 51.0, 1.5, 0.0, -60.0);
    // pitch: glance down to read, walking bob, "yes" nods
    let nods = if (55.0..57.0).contains(&t) { 10.0 * (2.0 * PI * 1.5 * (t - 55.0)).sin() * (PI * (t - 55.0) / 2.0).sin() } else { 0.0 };
    let pitch = mv(t, 14.0, 0.6, 0.0, -20.0) + mv(t, 17.0, 0.6, 0.0, 20.0) + 1.5 * env * (w2 * t + 0.4).sin() + nods;
    // roll: tilt gesture (UC-3 scroll) and walking sway
    let roll = mv(t, 30.0, 1.0, 0.0, 12.0) + mv(t, 34.0, 1.0, 0.0, -12.0) + 1.2 * env * (w1 * t).sin();
    // translation while walking at 1.3 m/s with gait oscillations
    // forward: x = 1.3·∫env + 0.09/w2·(1 − cos) style fluctuation (v' = 0.09 sin)
    let x_nominal = 1.3 * integral_env(t);
    let x_fluct = -0.09 / w2 * (w2 * t).cos() * env;
    let y = 0.018 * (w1 * t).sin() * env;
    let z = -0.013 * (w2 * t).sin() * env; // up is −z
    HeadPose { rpy: [roll.to_radians(), pitch.to_radians(), yaw.to_radians()], pos: [x_nominal + x_fluct, y, z] }
}

/// ∫₀ᵗ walk_env, computed in closed form piecewise (min-jerk integral).
fn integral_env(t: f64) -> f64 {
    // ∫ mj over [t0, t] for a ramp of duration d
    let ramp_int = |t: f64, t0: f64, d: f64| -> f64 {
        if t <= t0 {
            0.0
        } else if t >= t0 + d {
            d * 0.5 + (t - t0 - d)
        } else {
            let s = (t - t0) / d;
            // ∫ s³(10 − 15s + 6s²) ds = 2.5 s⁴ − 3 s⁵ + s⁶
            d * (2.5 * s.powi(4) - 3.0 * s.powi(5) + s.powi(6))
        }
    };
    ramp_int(t, 20.0, 1.0) - ramp_int(t, 44.0, 1.0)
}

/// Body→world rotation matrix for ZYX Euler angles.
pub fn rot(rpy: [f64; 3]) -> [[f64; 3]; 3] {
    let (sr, cr) = rpy[0].sin_cos();
    let (sp, cp) = rpy[1].sin_cos();
    let (sy, cy) = rpy[2].sin_cos();
    [
        [cy * cp, cy * sp * sr - sy * cr, cy * sp * cr + sy * sr],
        [sy * cp, sy * sp * sr + cy * cr, sy * sp * cr - cy * sr],
        [-sp, cp * sr, cp * cr],
    ]
}

/// Lever arm neck pivot → IMU (in the right temple pod), body frame, m.
pub const LEVER_ARM: [f64; 3] = [0.08, 0.07, -0.10];

fn imu_pos(t: f64) -> [f64; 3] {
    let hp = head_pose(t);
    let r = rot(hp.rpy);
    let mut p = hp.pos;
    for i in 0..3 {
        p[i] += (0..3).map(|j| r[i][j] * LEVER_ARM[j]).sum::<f64>();
    }
    p
}

/// True body rates (rad/s) and specific force (m/s²) at time t.
pub fn true_imu(t: f64) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let h = 1e-4;
    let a = head_pose(t - h).rpy;
    let b = head_pose(t + h).rpy;
    let rpy = head_pose(t).rpy;
    let d: Vec<f64> = (0..3).map(|i| (b[i] - a[i]) / (2.0 * h)).collect();
    let (sr, cr) = rpy[0].sin_cos();
    let (sp, cp) = rpy[1].sin_cos();
    let w = [d[0] - d[2] * sp, d[1] * cr + d[2] * cp * sr, -d[1] * sr + d[2] * cp * cr];
    let h2 = 2e-3;
    let pm = imu_pos(t - h2);
    let p0 = imu_pos(t);
    let pp = imu_pos(t + h2);
    let acc: Vec<f64> = (0..3).map(|i| (pp[i] - 2.0 * p0[i] + pm[i]) / (h2 * h2)).collect();
    let r = rot(rpy);
    let aw = [acc[0], acc[1], acc[2] - G];
    let mut f = [0.0; 3];
    for (j, fj) in f.iter_mut().enumerate() {
        *fj = (0..3).map(|i| r[i][j] * aw[i]).sum();
    }
    (rpy, w, f)
}

type Truth = Vec<([f64; 3], [f64; 3], [f64; 3])>;

thread_local! {
    /// The scenario only depends on the sample rate and duration, so the
    /// ground truth is cached between calls (requirement verification
    /// re-runs on every project edit).
    static TRUTH: std::cell::RefCell<Option<(u64, usize, std::rc::Rc<Truth>)>> = const { std::cell::RefCell::new(None) };
}

fn truth(rate: f64, n: usize) -> std::rc::Rc<Truth> {
    TRUTH.with(|c| {
        let mut c = c.borrow_mut();
        if let Some((r, nn, v)) = c.as_ref() {
            if *r == rate.to_bits() && *nn == n {
                return v.clone();
            }
        }
        let v = std::rc::Rc::new((0..n).map(|k| true_imu(k as f64 / rate)).collect::<Truth>());
        *c = Some((rate.to_bits(), n, v.clone()));
        v
    })
}

fn euler_rates(rpy: [f64; 3], w: [f64; 3]) -> [f64; 3] {
    let (sr, cr) = rpy[0].sin_cos();
    let (sp, cp) = rpy[1].sin_cos();
    let cp = if cp.abs() < 1e-6 { 1e-6f64.copysign(cp) } else { cp };
    let tp = sp / cp;
    [w[0] + (w[1] * sr + w[2] * cr) * tp, w[1] * cr - w[2] * sr, (w[1] * sr + w[2] * cr) / cp]
}

fn accel_tilt(f: [f64; 3]) -> [f64; 2] {
    [(-f[1]).atan2(-f[2]), f[0].atan2((f[1] * f[1] + f[2] * f[2]).sqrt())]
}

fn wrap(a: f64) -> f64 {
    (a + PI).rem_euclid(2.0 * PI) - PI
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct HeadInput {
    pub imu: ImuParams,
    pub duration_s: f64,
    pub seed: u64,
    /// Kalman measurement allowance for motion acceleration, degrees.
    pub kf_motion_deg: f64,
    /// Kalman bias random-walk density, deg/s/√s.
    pub kf_bias_rw: f64,
    /// Output decimation rate, Hz.
    pub out_hz: f64,
    /// Also sweep α and return RMS error vs α.
    pub sweep: bool,
}

impl Default for HeadInput {
    fn default() -> Self {
        HeadInput { imu: ImuParams::default(), duration_s: 60.0, seed: 42, kf_motion_deg: 2.0, kf_bias_rw: 0.002, out_hz: 50.0, sweep: false }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Estimate {
    pub id: String,
    pub name: String,
    pub roll: Vec<f64>,
    pub pitch: Vec<f64>,
    pub yaw: Vec<f64>,
    pub rms_roll: f64,
    pub rms_pitch: f64,
    /// √((rms_roll² + rms_pitch²)/2), degrees.
    pub rms_tilt: f64,
    pub max_tilt_err: f64,
    pub final_yaw_err: f64,
    pub has_yaw: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Segment {
    pub t0: f64,
    pub t1: f64,
    pub label: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HeadResult {
    pub t: Vec<f64>,
    pub true_roll: Vec<f64>,
    pub true_pitch: Vec<f64>,
    pub true_yaw: Vec<f64>,
    /// Specific-force magnitude, g.
    pub accel_norm_g: Vec<f64>,
    pub gyro_y_dps: Vec<f64>,
    pub estimates: Vec<Estimate>,
    pub events: Vec<Segment>,
    /// Complementary crossover frequency, Hz.
    pub cf_crossover_hz: f64,
    /// CF time constant α·Δt/(1−α), s.
    pub cf_tau_s: f64,
    /// Kalman bias estimate at the end (deg/s): roll, pitch axes.
    pub kf_bias_est: [f64; 2],
    pub gyro_sigma_dps: f64,
    pub accel_sigma_mg: f64,
    pub samples: usize,
    /// (alpha, rms_tilt_deg) if requested.
    pub sweep: Vec<(f64, f64)>,
}

struct Sim {
    rms_cf: f64,
}

struct Kf1 {
    x: [f64; 2],
    p: [[f64; 2]; 2],
}

impl Kf1 {
    fn new(angle: f64) -> Kf1 {
        Kf1 { x: [angle, 0.0], p: [[1e-4, 0.0], [0.0, (1.0f64).to_radians().powi(2)]] }
    }
    fn predict(&mut self, rate: f64, dt: f64, q_angle: f64, q_bias: f64) {
        self.x[0] += (rate - self.x[1]) * dt;
        let p = self.p;
        // F = [[1, −dt], [0, 1]]
        let p00 = p[0][0] - dt * (p[1][0] + p[0][1]) + dt * dt * p[1][1] + q_angle;
        let p01 = p[0][1] - dt * p[1][1];
        let p10 = p[1][0] - dt * p[1][1];
        let p11 = p[1][1] + q_bias;
        self.p = [[p00, p01], [p10, p11]];
    }
    fn update(&mut self, z: f64, r: f64) {
        let y = wrap(z - self.x[0]);
        let s = self.p[0][0] + r;
        let k0 = self.p[0][0] / s;
        let k1 = self.p[1][0] / s;
        self.x[0] += k0 * y;
        self.x[1] += k1 * y;
        let p = self.p;
        self.p = [[(1.0 - k0) * p[0][0], (1.0 - k0) * p[0][1]], [p[1][0] - k1 * p[0][0], p[1][1] - k1 * p[0][1]]];
    }
}

/// Run the scenario. With `record = false` only the CF RMS is computed.
fn run(inp: &HeadInput, alpha: f64, record: Option<&mut HeadResult>) -> Sim {
    let imu = &inp.imu;
    let rate = imu.rate_hz.clamp(10.0, 4000.0);
    let dt = 1.0 / rate;
    let n = (inp.duration_s.clamp(5.0, 120.0) * rate) as usize;
    let mut rng = Rng::new(inp.seed);
    let sg = (imu.gyro_noise_dps_rthz.max(0.0) * rate.sqrt()).to_radians();
    let sa = imu.accel_noise_mg_rthz.max(0.0) * rate.sqrt() * 1e-3 * G;
    let bias = [imu.gyro_bias_dps.to_radians(), -imu.gyro_bias_dps.to_radians(), imu.gyro_bias_dps.to_radians()];
    let alpha = alpha.clamp(0.0, 1.0);
    let decim = ((rate / inp.out_hz.clamp(5.0, 500.0)).round() as usize).max(1);

    let rpy0 = head_pose(0.0).rpy;
    let mut gyro_only = rpy0;
    let mut cf = rpy0;
    let mut kf = [Kf1::new(rpy0[0]), Kf1::new(rpy0[1])];
    let mut kf_yaw = rpy0[2];
    let q_angle = (imu.gyro_noise_dps_rthz.max(1e-6).to_radians()).powi(2) * dt;
    let q_bias = (inp.kf_bias_rw.max(1e-9).to_radians()).powi(2) * dt;
    let r_meas = (sa / G).powi(2) + inp.kf_motion_deg.max(0.0).to_radians().powi(2);

    let mut sse = [[0.0f64; 2]; 4];
    let mut maxe = [0.0f64; 4];
    let mut rec = record;
    let tr = truth(rate, n);
    for k in 0..n {
        let t = k as f64 * dt;
        let (truth, w, f) = tr[k];
        let wm = [w[0] + bias[0] + sg * rng.normal(), w[1] + bias[1] + sg * rng.normal(), w[2] + bias[2] + sg * rng.normal()];
        let fm = [f[0] + sa * rng.normal(), f[1] + sa * rng.normal(), f[2] + sa * rng.normal()];
        let acc = accel_tilt(fm);
        // gyro only
        let er = euler_rates(gyro_only, wm);
        for i in 0..3 {
            gyro_only[i] += er[i] * dt;
        }
        // complementary
        let ec = euler_rates(cf, wm);
        cf[0] = alpha * (cf[0] + ec[0] * dt) + (1.0 - alpha) * (cf[0] + wrap(acc[0] - cf[0]));
        cf[1] = alpha * (cf[1] + ec[1] * dt) + (1.0 - alpha) * (cf[1] + wrap(acc[1] - cf[1]));
        cf[2] += ec[2] * dt;
        // Kalman
        let kf_rpy = [kf[0].x[0], kf[1].x[0], kf_yaw];
        let ek = euler_rates(kf_rpy, wm);
        for (ax, kfa) in kf.iter_mut().enumerate() {
            kfa.predict(ek[ax], dt, q_angle, q_bias);
            kfa.update(acc[ax], r_meas);
        }
        kf_yaw += ek[2] * dt;
        // errors (skip the first 0.5 s alignment transient — none here, all start aligned)
        let ests = [gyro_only, [acc[0], acc[1], 0.0], cf, [kf[0].x[0], kf[1].x[0], kf_yaw]];
        for (e, est) in ests.iter().enumerate() {
            for ax in 0..2 {
                let d = wrap(est[ax] - truth[ax]);
                sse[e][ax] += d * d;
                maxe[e] = maxe[e].max(d.abs());
            }
        }
        if let Some(r) = rec.as_deref_mut() {
            if k % decim == 0 {
                r.t.push(t);
                r.true_roll.push(truth[0].to_degrees());
                r.true_pitch.push(truth[1].to_degrees());
                r.true_yaw.push(truth[2].to_degrees());
                r.accel_norm_g.push((fm[0] * fm[0] + fm[1] * fm[1] + fm[2] * fm[2]).sqrt() / G);
                r.gyro_y_dps.push(wm[1].to_degrees());
                for (e, est) in ests.iter().enumerate() {
                    r.estimates[e].roll.push(est[0].to_degrees());
                    r.estimates[e].pitch.push(est[1].to_degrees());
                    r.estimates[e].yaw.push(if e == 1 { f64::NAN } else { est[2].to_degrees() });
                }
            }
            if k == n - 1 {
                for (e, est) in ests.iter().enumerate() {
                    let es = &mut r.estimates[e];
                    es.rms_roll = (sse[e][0] / n as f64).sqrt().to_degrees();
                    es.rms_pitch = (sse[e][1] / n as f64).sqrt().to_degrees();
                    es.rms_tilt = ((sse[e][0] + sse[e][1]) / (2.0 * n as f64)).sqrt().to_degrees();
                    es.max_tilt_err = maxe[e].to_degrees();
                    es.final_yaw_err = if e == 1 { f64::NAN } else { wrap(est[2] - truth[2]).to_degrees() };
                }
                r.kf_bias_est = [kf[0].x[1].to_degrees(), kf[1].x[1].to_degrees()];
            }
        }
    }
    Sim { rms_cf: ((sse[2][0] + sse[2][1]) / (2.0 * n as f64)).sqrt().to_degrees() }
}

pub fn head_tracking(inp: &HeadInput) -> HeadResult {
    let rate = inp.imu.rate_hz.clamp(10.0, 4000.0);
    let dt = 1.0 / rate;
    let a = inp.imu.fusion_alpha.clamp(0.0, 0.99999);
    let mut r = HeadResult {
        estimates: [
            ("gyro", "Gyro only (integrate)", true),
            ("accel", "Accel only (tilt)", false),
            ("cf", "Complementary filter", true),
            ("kf", "Kalman filter (angle + bias)", true),
        ]
        .iter()
        .map(|(id, name, y)| Estimate { id: (*id).into(), name: (*name).into(), has_yaw: *y, ..Default::default() })
        .collect(),
        events: vec![
            Segment { t0: 4.0, t1: 13.0, label: "look around (yaw)".into() },
            Segment { t0: 14.0, t1: 17.6, label: "glance down".into() },
            Segment { t0: 20.0, t1: 45.0, label: "walking".into() },
            Segment { t0: 30.0, t1: 35.0, label: "tilt gesture".into() },
            Segment { t0: 47.0, t1: 52.5, label: "look around".into() },
            Segment { t0: 55.0, t1: 57.0, label: "nods".into() },
        ],
        cf_crossover_hz: (1.0 - a) / (2.0 * PI * a.max(1e-9) * dt),
        cf_tau_s: a * dt / (1.0 - a).max(1e-9),
        gyro_sigma_dps: inp.imu.gyro_noise_dps_rthz * rate.sqrt(),
        accel_sigma_mg: inp.imu.accel_noise_mg_rthz * rate.sqrt(),
        samples: (inp.duration_s.clamp(5.0, 120.0) * rate) as usize,
        ..Default::default()
    };
    run(inp, a, Some(&mut r));
    if inp.sweep {
        let alphas = [0.5, 0.7, 0.8, 0.9, 0.93, 0.95, 0.96, 0.97, 0.98, 0.985, 0.99, 0.993, 0.995, 0.997, 0.998, 0.999];
        let fast = HeadInput { duration_s: inp.duration_s, ..inp.clone() };
        r.sweep = alphas.iter().map(|&al| (al, run(&fast, al, None).rms_cf)).collect();
    }
    r
}

/// The metric: RMS tilt (pitch/roll) error of the complementary filter.
pub fn head_rms_err_deg(imu: &ImuParams) -> f64 {
    let inp = HeadInput { imu: imu.clone(), ..Default::default() };
    run(&inp, imu.fusion_alpha, None).rms_cf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stationary_accel_reads_minus_g() {
        let (_, w, f) = true_imu(1.0);
        assert!(w.iter().all(|v| v.abs() < 1e-6));
        assert!((f[2] + G).abs() < 1e-6 && f[0].abs() < 1e-6);
    }

    #[test]
    fn accel_tilt_recovers_static_pitch() {
        // during the "glance down" hold at t = 15.5 s pitch = −20°
        let (rpy, _, f) = true_imu(15.5);
        let a = accel_tilt(f);
        assert!((a[1] - rpy[1]).abs() < 1e-4 && (rpy[1].to_degrees() + 20.0).abs() < 1e-6);
    }

    #[test]
    fn kalman_beats_gyro_only_and_cf_meets_requirement() {
        let r = head_tracking(&HeadInput::default());
        let get = |id: &str| r.estimates.iter().find(|e| e.id == id).unwrap().clone();
        let (g, a, c, k) = (get("gyro"), get("accel"), get("cf"), get("kf"));
        assert!(k.rms_tilt < g.rms_tilt / 5.0, "kf {} gyro {}", k.rms_tilt, g.rms_tilt);
        assert!(c.rms_tilt < a.rms_tilt, "cf {} accel {}", c.rms_tilt, a.rms_tilt);
        // gyro-only drift over 60 s ≈ bias × t (0.5 °/s × 60 s = 30° on yaw)
        assert!((g.final_yaw_err.abs() - 30.0).abs() < 6.0, "yaw drift {}", g.final_yaw_err);
        let m = head_rms_err_deg(&crate::project::Project::iris().design.imu);
        assert!(m > 0.05 && m <= 2.0, "Iris CF RMS {m}");
        assert!((m - c.rms_tilt).abs() < 1e-9);
    }
}
