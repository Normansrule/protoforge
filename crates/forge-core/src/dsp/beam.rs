//! Two-microphone endfire beamformers for the Iris temple.
//!
//! Geometry: the front microphone sits at +d/2 on the array axis (pointing
//! towards the look direction θ = 0), the rear one at −d/2. A plane wave from
//! angle θ reaches the rear mic later by d·cosθ / c.
//!
//! **First-order differential array** (delay-and-subtract):
//! y = x_front − x_rear(t − τ), so
//! H(ω, θ) = 1 − e^{−jω(τ + d cosθ / c)}.
//! For kd ≪ 1, |H| ≈ ω (τ + d/c) · [α + (1 − α) cosθ] with α = τ / (τ + d/c):
//! dipole α = 0 (τ = 0), cardioid α = 1/2 (τ = d/c), hypercardioid α = 1/4
//! (τ = d/3c, maximum directivity index 6.02 dB), supercardioid
//! α = (√3 − 1)/2 (τ = d/(√3 c), maximum front-to-back ratio, DI 5.72 dB).
//! G. W. Elko, "Superdirectional microphone arrays", in *Acoustic Signal
//! Processing for Telecommunication* (S. L. Gay & J. Benesty, eds.), Kluwer
//! 2000; J. Benesty & J. Chen, *Study and Design of Differential Microphone
//! Arrays*, Springer 2013, ch. 2.
//!
//! **Directivity index** DI = 10 log10( |H(θ₀)|² / (1/4π ∮ |H|² dΩ) ),
//! integrated numerically over the sphere (the array is axisymmetric so the
//! φ integral is 2π and the θ integral uses Simpson's rule with sinθ dθ).
//!
//! **White-noise gain** WNG = |wᴴ d(θ₀)|² / wᴴw — the SNR gain against
//! uncorrelated (sensor self-) noise. Differential arrays have WNG ≪ 1 at low
//! frequency: flattening their +6 dB/octave response amplifies mic noise.

use serde::{Deserialize, Serialize};

use crate::math::{C64, PI};

pub const SPEED_OF_SOUND: f64 = 343.0;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Pattern {
    Omni,
    Dipole,
    Cardioid,
    #[default]
    Hypercardioid,
    Supercardioid,
    /// Differential array with a user-chosen τc/d ratio.
    Custom,
    /// Delay-and-sum steered to endfire.
    DelaySum,
}

impl Pattern {
    /// Internal delay as a fraction of the acoustic travel time d/c.
    pub fn tau_ratio(self, custom: f64) -> f64 {
        match self {
            Pattern::Dipole => 0.0,
            Pattern::Cardioid => 1.0,
            Pattern::Hypercardioid => 1.0 / 3.0,
            Pattern::Supercardioid => 1.0 / 3f64.sqrt(),
            Pattern::Custom => custom.max(0.0),
            Pattern::Omni | Pattern::DelaySum => 0.0,
        }
    }
    pub const NAMED: [Pattern; 6] = [Pattern::Omni, Pattern::Dipole, Pattern::Cardioid, Pattern::Hypercardioid, Pattern::Supercardioid, Pattern::DelaySum];
}

/// Microphone weights (front, rear) in the frequency domain.
fn weights(p: Pattern, tau_ratio: f64, w: f64, d: f64, c: f64) -> (C64, C64) {
    match p {
        Pattern::Omni => (C64::ONE, C64::ZERO),
        Pattern::DelaySum => (C64::from_polar(0.5, -w * d / c), C64::real(0.5)),
        _ => (C64::ONE, -C64::from_polar(1.0, -w * tau_ratio * d / c)),
    }
}

/// Array response to a far-field plane wave from angle θ (radians from the front axis).
pub fn response(p: Pattern, tau_ratio: f64, f: f64, d: f64, c: f64, theta: f64) -> C64 {
    let w = 2.0 * PI * f;
    let (w1, w2) = weights(p, tau_ratio, w, d, c);
    // front mic phase reference; rear mic delayed by d cosθ / c
    w1 + w2 * C64::from_polar(1.0, -w * d * theta.cos() / c)
}

/// Directivity factor/index by numerical integration over the sphere.
pub fn directivity_index(p: Pattern, tau_ratio: f64, f: f64, d: f64, c: f64) -> f64 {
    let n = 360; // even number of Simpson intervals
    let h = PI / n as f64;
    let mut integral = 0.0;
    for i in 0..=n {
        let th = i as f64 * h;
        let wgt = if i == 0 || i == n { 1.0 } else if i % 2 == 1 { 4.0 } else { 2.0 };
        integral += wgt * response(p, tau_ratio, f, d, c, th).norm_sqr() * th.sin();
    }
    integral *= h / 3.0;
    // (1/4π)∮|H|² dΩ = (1/4π)·2π·∫|H|² sinθ dθ = ½∫…
    let mean = 0.5 * integral;
    let on = response(p, tau_ratio, f, d, c, 0.0).norm_sqr();
    10.0 * (on / mean.max(1e-300)).log10()
}

/// White-noise gain (dB re one microphone).
pub fn white_noise_gain(p: Pattern, tau_ratio: f64, f: f64, d: f64, c: f64) -> f64 {
    let w = 2.0 * PI * f;
    let (w1, w2) = weights(p, tau_ratio, w, d, c);
    let on = response(p, tau_ratio, f, d, c, 0.0).norm_sqr();
    10.0 * (on / (w1.norm_sqr() + w2.norm_sqr()).max(1e-300)).log10()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct BeamInput {
    pub spacing_mm: f64,
    pub pattern: Pattern,
    /// τ·c/d for `custom`.
    pub tau_ratio: f64,
    /// Frequencies for polar plots, Hz.
    pub polar_freqs: Vec<f64>,
    pub f_min: f64,
    pub f_max: f64,
    pub n_freq: usize,
    pub c: f64,
    /// Below this frequency the EQ stops boosting (first-order high-pass).
    pub eq_low_hz: f64,
}

impl Default for BeamInput {
    fn default() -> Self {
        BeamInput { spacing_mm: 14.0, pattern: Pattern::Hypercardioid, tau_ratio: 1.0 / 3.0, polar_freqs: vec![500.0, 1000.0, 2000.0, 4000.0], f_min: 100.0, f_max: 8000.0, n_freq: 120, c: SPEED_OF_SOUND, eq_low_hz: 100.0 }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Polar {
    pub f: f64,
    pub angle_deg: Vec<f64>,
    /// |H(θ)| / |H(0)| in dB (clamped at −50 dB).
    pub gain_db: Vec<f64>,
    /// |H(θ)| / |H(0)| linear.
    pub gain: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PatternRow {
    pub pattern: Pattern,
    pub tau_ratio: f64,
    pub tau_us: f64,
    pub di_1k_db: f64,
    pub di_ideal_db: f64,
    pub null_deg: Option<f64>,
    pub front_back_db: f64,
    pub wng_1k_db: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct BeamResult {
    pub spacing_mm: f64,
    pub pattern: Pattern,
    pub tau_ratio: f64,
    pub tau_us: f64,
    pub alpha: f64,
    pub null_deg: Option<f64>,
    pub polar: Vec<Polar>,
    pub freq: Vec<f64>,
    pub di_db: Vec<f64>,
    pub di_das_db: Vec<f64>,
    pub wng_db: Vec<f64>,
    pub wng_das_db: Vec<f64>,
    /// On-axis magnitude re one omni mic, dB (the +6 dB/oct differential slope).
    pub onaxis_db: Vec<f64>,
    /// EQ that flattens the on-axis response above `eq_low_hz`.
    pub eq_db: Vec<f64>,
    pub equalized_db: Vec<f64>,
    /// Effective self-noise increase after EQ (= −WNG), dB.
    pub noise_boost_db: Vec<f64>,
    pub di_1k_db: f64,
    pub wng_1k_db: f64,
    pub spatial_alias_hz: f64,
    pub table: Vec<PatternRow>,
}

/// Small-spacing (ideal) DI of the pattern α + (1−α)cosθ:
/// DF = 3 / (3α² + (1−α)²) — Benesty & Chen 2013, eq. (2.37) form.
pub fn ideal_di_db(alpha: f64) -> f64 {
    10.0 * (3.0 / (3.0 * alpha * alpha + (1.0 - alpha).powi(2))).log10()
}

fn pattern_row(p: Pattern, custom: f64, d: f64, c: f64) -> PatternRow {
    let tr = p.tau_ratio(custom);
    let alpha = tr / (tr + 1.0);
    let null = if matches!(p, Pattern::Omni | Pattern::DelaySum) || tr > 1.0 { None } else { Some((-tr).acos().to_degrees()) };
    let f = 1000.0;
    let fb = {
        let front = response(p, tr, f, d, c, 0.0).abs();
        let back = response(p, tr, f, d, c, PI).abs();
        20.0 * (front / back.max(1e-9)).log10()
    };
    PatternRow {
        pattern: p,
        tau_ratio: tr,
        tau_us: tr * d / c * 1e6,
        di_1k_db: directivity_index(p, tr, f, d, c),
        di_ideal_db: match p {
            Pattern::Omni | Pattern::DelaySum => 0.0,
            _ => ideal_di_db(alpha),
        },
        null_deg: null,
        front_back_db: fb.min(99.0),
        wng_1k_db: white_noise_gain(p, tr, f, d, c),
    }
}

pub fn analyze(inp: &BeamInput) -> BeamResult {
    let d = (inp.spacing_mm / 1000.0).clamp(1e-4, 1.0);
    let c = if inp.c > 0.0 { inp.c } else { SPEED_OF_SOUND };
    let p = inp.pattern;
    let tr = p.tau_ratio(inp.tau_ratio);
    let polar = inp
        .polar_freqs
        .iter()
        .take(8)
        .map(|&f| {
            let on = response(p, tr, f, d, c, 0.0).abs().max(1e-12);
            let angle_deg: Vec<f64> = (0..=180).map(|i| i as f64 * 2.0).collect();
            let gain: Vec<f64> = angle_deg.iter().map(|a| response(p, tr, f, d, c, a.to_radians()).abs() / on).collect();
            Polar { f, gain_db: gain.iter().map(|g| (20.0 * g.max(1e-9).log10()).max(-50.0)).collect(), gain, angle_deg }
        })
        .collect();
    let freq = crate::math::logspace(inp.f_min.max(10.0), inp.f_max.max(inp.f_min * 1.1), inp.n_freq.clamp(10, 400));
    let di_db: Vec<f64> = freq.iter().map(|&f| directivity_index(p, tr, f, d, c)).collect();
    let di_das_db: Vec<f64> = freq.iter().map(|&f| directivity_index(Pattern::DelaySum, 0.0, f, d, c)).collect();
    let wng_db: Vec<f64> = freq.iter().map(|&f| white_noise_gain(p, tr, f, d, c)).collect();
    let wng_das_db: Vec<f64> = freq.iter().map(|&f| white_noise_gain(Pattern::DelaySum, 0.0, f, d, c)).collect();
    let onaxis_db: Vec<f64> = freq.iter().map(|&f| response(p, tr, f, d, c, 0.0).db().max(-120.0)).collect();
    let fl = inp.eq_low_hz.max(1.0);
    let on_fl = response(p, tr, fl, d, c, 0.0).abs().max(1e-9);
    let eq_db: Vec<f64> = freq
        .iter()
        .map(|&f| {
            if f >= fl {
                -response(p, tr, f, d, c, 0.0).db()
            } else {
                // hold the boost at its f_low value and roll off below with a first-order high-pass
                -20.0 * on_fl.log10() + 20.0 * (f / fl).log10()
            }
        })
        .map(|v: f64| v.clamp(-120.0, 120.0))
        .collect();
    let equalized_db: Vec<f64> = onaxis_db.iter().zip(&eq_db).map(|(a, b)| a + b).collect();
    let noise_boost_db = wng_db.iter().map(|w| -w).collect();
    let mut table: Vec<PatternRow> = Pattern::NAMED.iter().map(|&q| pattern_row(q, inp.tau_ratio, d, c)).collect();
    if p == Pattern::Custom {
        table.push(pattern_row(Pattern::Custom, inp.tau_ratio, d, c));
    }
    let null_deg = if matches!(p, Pattern::Omni | Pattern::DelaySum) || tr > 1.0 { None } else { Some((-tr).acos().to_degrees()) };
    BeamResult {
        spacing_mm: inp.spacing_mm,
        pattern: p,
        tau_ratio: tr,
        tau_us: tr * d / c * 1e6,
        alpha: tr / (tr + 1.0),
        null_deg,
        polar,
        di_1k_db: directivity_index(p, tr, 1000.0, d, c),
        wng_1k_db: white_noise_gain(p, tr, 1000.0, d, c),
        spatial_alias_hz: c / (2.0 * d),
        freq,
        di_db,
        di_das_db,
        wng_db,
        wng_das_db,
        onaxis_db,
        eq_db,
        equalized_db,
        noise_boost_db,
        table,
    }
}

/// Near-field sound map: output level (after on-axis EQ, dB re a far-field
/// on-axis plane wave) for a point source anywhere in the plane of the array.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FieldInput {
    pub spacing_mm: f64,
    pub pattern: Pattern,
    pub tau_ratio: f64,
    pub freq: f64,
    /// Rotation of the mic axis relative to the temple (+ = front mic tilted down towards the mouth), degrees.
    pub tilt_deg: f64,
    /// Map extent (mm) around the array centre and resolution.
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub nx: usize,
    pub ny: usize,
    /// Extra probe points (mm), e.g. the mouth and a noise source.
    pub probes: Vec<[f64; 2]>,
    pub c: f64,
}

impl Default for FieldInput {
    fn default() -> Self {
        FieldInput { spacing_mm: 14.0, pattern: Pattern::Hypercardioid, tau_ratio: 1.0 / 3.0, freq: 1000.0, tilt_deg: 0.0, x_min: -160.0, x_max: 160.0, y_min: -140.0, y_max: 100.0, nx: 64, ny: 48, probes: vec![], c: SPEED_OF_SOUND }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct FieldResult {
    pub nx: usize,
    pub ny: usize,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    /// Row-major (y then x) levels in dB.
    pub level_db: Vec<f64>,
    /// Probe results: level (dB, EQ'd, re far-field on-axis). A single omni
    /// mic with the same normalisation reads 0 dB everywhere, so this is also
    /// the gain of the array over one microphone.
    pub probe_db: Vec<f64>,
    pub probe_angle_deg: Vec<f64>,
    pub mics: Vec<[f64; 2]>,
}

pub fn field(inp: &FieldInput) -> FieldResult {
    let d = (inp.spacing_mm / 1000.0).clamp(1e-4, 1.0);
    let c = if inp.c > 0.0 { inp.c } else { SPEED_OF_SOUND };
    let p = inp.pattern;
    let tr = p.tau_ratio(inp.tau_ratio);
    let w = 2.0 * PI * inp.freq.max(1.0);
    let k = w / c;
    let tilt = inp.tilt_deg.to_radians();
    // front mic towards +x, rotated by −tilt (tilt down = negative y)
    let ux = tilt.cos();
    let uy = -tilt.sin();
    let front = [ux * d / 2.0, uy * d / 2.0];
    let rear = [-ux * d / 2.0, -uy * d / 2.0];
    let (w1, w2) = weights(p, tr, w, d, c);
    let far_on = response(p, tr, inp.freq.max(1.0), d, c, 0.0).abs().max(1e-12);
    let eval = |xm: f64, ym: f64| -> (f64, f64) {
        let (x, y) = (xm / 1000.0, ym / 1000.0);
        let r0 = (x * x + y * y).sqrt().max(1e-4);
        let rf = ((x - front[0]).powi(2) + (y - front[1]).powi(2)).sqrt().max(1e-4);
        let rr = ((x - rear[0]).powi(2) + (y - rear[1]).powi(2)).sqrt().max(1e-4);
        // spherical waves normalised to unit amplitude and zero phase at the array centre
        let pf = C64::from_polar(r0 / rf, -k * (rf - r0));
        let pr = C64::from_polar(r0 / rr, -k * (rr - r0));
        let y_out = w1 * pf + w2 * pr;
        let lvl = 20.0 * (y_out.abs() / far_on).max(1e-6).log10();
        let mut ang = (y.atan2(x) + tilt).to_degrees();
        if ang > 180.0 {
            ang -= 360.0;
        } else if ang <= -180.0 {
            ang += 360.0;
        }
        (lvl, ang)
    };
    let nx = inp.nx.clamp(4, 200);
    let ny = inp.ny.clamp(4, 200);
    let xs: Vec<f64> = crate::math::linspace(inp.x_min, inp.x_max, nx);
    let ys: Vec<f64> = crate::math::linspace(inp.y_min, inp.y_max, ny);
    let mut level_db = Vec::with_capacity(nx * ny);
    for &y in &ys {
        for &x in &xs {
            level_db.push(eval(x, y).0.clamp(-60.0, 40.0));
        }
    }
    let mut probe_db = Vec::new();
    let mut probe_angle_deg = Vec::new();
    for pr in &inp.probes {
        let (l, a) = eval(pr[0], pr[1]);
        probe_db.push(l);
        probe_angle_deg.push(a);
    }
    FieldResult {
        nx,
        ny,
        x: xs,
        y: ys,
        level_db,
        probe_db,
        probe_angle_deg,
        mics: vec![[front[0] * 1000.0, front[1] * 1000.0], [rear[0] * 1000.0, rear[1] * 1000.0]],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_spacing_directivity_matches_elko_and_benesty() {
        let d = 0.002; // 2 mm at 500 Hz: kd ≈ 0.018 → ideal limit
        let f = 500.0;
        let c = SPEED_OF_SOUND;
        let di = |p: Pattern| directivity_index(p, p.tau_ratio(0.0), f, d, c);
        assert!((di(Pattern::Cardioid) - 4.77).abs() < 0.02, "{}", di(Pattern::Cardioid));
        assert!((di(Pattern::Hypercardioid) - 6.02).abs() < 0.02, "{}", di(Pattern::Hypercardioid));
        assert!((di(Pattern::Supercardioid) - 5.72).abs() < 0.02, "{}", di(Pattern::Supercardioid));
        assert!((di(Pattern::Dipole) - 4.77).abs() < 0.02);
        assert!(di(Pattern::Omni).abs() < 1e-3);
        // delay-and-sum on a tiny array is nearly omnidirectional
        assert!(di(Pattern::DelaySum) < 0.1);
        assert!((ideal_di_db(0.25) - 6.02).abs() < 0.01);
    }

    #[test]
    fn hypercardioid_null_and_wng() {
        let row = pattern_row(Pattern::Hypercardioid, 0.0, 0.014, SPEED_OF_SOUND);
        assert!((row.null_deg.unwrap() - 109.47).abs() < 0.01);
        // WNG at 1 kHz, 14 mm: |H|² = 2 − 2cos(ω·(4/3)d/c) → ≈ −12.4 dB re one mic (and /2 for two weights)
        let w = 2.0 * PI * 1000.0 * (4.0 / 3.0) * 0.014 / SPEED_OF_SOUND;
        let expect = 10.0 * ((2.0 - 2.0 * w.cos()) / 2.0).log10();
        assert!((row.wng_1k_db - expect).abs() < 1e-9);
        // delay-and-sum WNG = 2 (3 dB)
        assert!((white_noise_gain(Pattern::DelaySum, 0.0, 1000.0, 0.014, SPEED_OF_SOUND) - 3.0103).abs() < 1e-3);
    }

    #[test]
    fn differential_slope_is_6db_per_octave() {
        let r = analyze(&BeamInput { f_min: 250.0, f_max: 500.0, n_freq: 10, pattern: Pattern::Cardioid, ..Default::default() });
        assert!(((r.onaxis_db[9] - r.onaxis_db[0]) - 6.0).abs() < 0.1);
    }

    #[test]
    fn far_field_map_matches_polar_pattern() {
        let fr = field(&FieldInput { probes: vec![[5000.0, 0.0], [-5000.0 * 0.3333, 5000.0 * 0.9428]], nx: 4, ny: 4, ..Default::default() });
        assert!(fr.probe_db[0].abs() < 0.1, "on-axis far field {}", fr.probe_db[0]);
        assert!(fr.probe_db[1] < -30.0, "null at 109.5° {}", fr.probe_db[1]);
    }
}
