//! Waveform measurements (the equivalent of ngspice `.meas`): extrema,
//! threshold crossings, settling time, per-cycle ripple and −3 dB points.

/// Minimum and maximum of `y` for samples with `t ∈ [t0, t1]`.
pub fn min_max(t: &[f64], y: &[f32], t0: f64, t1: f64) -> Option<(f64, f64, f64, f64)> {
    let mut best: Option<(f64, f64, f64, f64)> = None;
    for (k, &tk) in t.iter().enumerate() {
        if tk < t0 || tk > t1 {
            continue;
        }
        let v = y[k] as f64;
        best = Some(match best {
            None => (v, tk, v, tk),
            Some((mn, tmn, mx, tmx)) => {
                let (mn, tmn) = if v < mn { (v, tk) } else { (mn, tmn) };
                let (mx, tmx) = if v > mx { (v, tk) } else { (mx, tmx) };
                (mn, tmn, mx, tmx)
            }
        });
    }
    best
}

/// First time after `after` at which `y` crosses `level` in the given
/// direction (linear interpolation between samples).
pub fn crossing(t: &[f64], y: &[f32], level: f64, rising: bool, after: f64) -> Option<f64> {
    for k in 1..t.len() {
        if t[k] < after {
            continue;
        }
        let (a, b) = (y[k - 1] as f64, y[k] as f64);
        let hit = if rising { a < level && b >= level } else { a > level && b <= level };
        if hit {
            let f = if (b - a).abs() > 0.0 { (level - a) / (b - a) } else { 0.0 };
            let tc = t[k - 1] + f * (t[k] - t[k - 1]);
            if tc >= after {
                return Some(tc);
            }
        }
    }
    None
}

/// Last time `y` was outside `final ± tol` (after `after`), i.e. the time
/// from which the waveform stays settled. `None` if it never settles.
pub fn settle_time(t: &[f64], y: &[f32], final_v: f64, tol: f64, after: f64) -> Option<f64> {
    let mut last_out = after;
    let mut any = false;
    for (k, &tk) in t.iter().enumerate() {
        if tk < after {
            continue;
        }
        any = true;
        if ((y[k] as f64) - final_v).abs() > tol {
            last_out = tk;
        }
    }
    if !any || last_out >= *t.last()? {
        None
    } else {
        Some(last_out)
    }
}

/// Mean of `y` over `[t0, t1]` (trapezoidal time average).
pub fn average(t: &[f64], y: &[f32], t0: f64, t1: f64) -> Option<f64> {
    let mut acc = 0.0;
    let mut span = 0.0;
    for k in 1..t.len() {
        if t[k - 1] < t0 || t[k] > t1 {
            continue;
        }
        let dt = t[k] - t[k - 1];
        acc += 0.5 * (y[k] as f64 + y[k - 1] as f64) * dt;
        span += dt;
    }
    if span > 0.0 {
        Some(acc / span)
    } else {
        None
    }
}

/// Largest per-cycle peak-to-peak over the last `cycles` periods before `tend`.
pub fn ripple_pp(t: &[f64], y: &[f32], period: f64, cycles: usize, tend: f64) -> Option<f64> {
    let mut worst: Option<f64> = None;
    for c in 0..cycles {
        let t1 = tend - c as f64 * period;
        let t0 = t1 - period;
        if let Some((mn, _, mx, _)) = min_max(t, y, t0, t1) {
            worst = Some(worst.unwrap_or(0.0).max(mx - mn));
        }
    }
    worst
}

/// Interpolated value of `y` at `x` (log-x interpolation when `logx`).
pub fn at(x: &[f64], y: &[f32], xv: f64, logx: bool) -> Option<f64> {
    if x.is_empty() {
        return None;
    }
    if xv <= x[0] {
        return Some(y[0] as f64);
    }
    for k in 1..x.len() {
        if x[k] >= xv {
            let (a, b) = if logx { (x[k - 1].ln(), x[k].ln()) } else { (x[k - 1], x[k]) };
            let xx = if logx { xv.ln() } else { xv };
            let f = if b > a { (xx - a) / (b - a) } else { 0.0 };
            return Some(y[k - 1] as f64 + f * (y[k] as f64 - y[k - 1] as f64));
        }
    }
    Some(*y.last()? as f64)
}

/// First frequency where |H| falls below `ref_mag/√2` (log interpolation).
pub fn f_3db(freq: &[f64], mag: &[f32], ref_mag: f64) -> Option<f64> {
    let lvl = ref_mag / std::f64::consts::SQRT_2;
    for k in 1..freq.len() {
        let (a, b) = (mag[k - 1] as f64, mag[k] as f64);
        if a >= lvl && b < lvl {
            let (la, lb) = (20.0 * a.log10(), 20.0 * b.log10());
            let ll = 20.0 * lvl.log10();
            let f = (ll - la) / (lb - la);
            return Some((freq[k - 1].ln() + f * (freq[k].ln() - freq[k - 1].ln())).exp());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossings_and_extrema() {
        let t: Vec<f64> = (0..11).map(|k| k as f64).collect();
        let y: Vec<f32> = t.iter().map(|v| *v as f32).collect();
        assert!((crossing(&t, &y, 4.5, true, 0.0).unwrap() - 4.5).abs() < 1e-9);
        let (mn, _, mx, _) = min_max(&t, &y, 2.0, 5.0).unwrap();
        assert_eq!((mn, mx), (2.0, 5.0));
        assert!((average(&t, &y, 0.0, 10.0).unwrap() - 5.0).abs() < 1e-9);
    }
}
