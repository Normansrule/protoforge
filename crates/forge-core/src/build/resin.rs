//! Resin (SLA / MSLA) exposure from the working curve.
//!
//! Jacobs' working curve for a photopolymer relates the depth that cures to
//! the energy dose at the surface (Beer–Lambert absorption of the light plus
//! a gel threshold):
//!
//! ```text
//! C_d = D_p · ln(E / E_c)          E = I · t   (mJ/cm² = mW/cm² × s)
//! ⇒ E = E_c · exp(C_d / D_p)       t = E / I
//! ```
//!
//! `D_p` (penetration depth, mm) and `E_c` (critical exposure, mJ/cm²) are
//! properties of one resin at one wavelength. They are measured by curing a
//! staircase of exposures and measuring each step's thickness: a straight
//! line of `C_d` against `ln E` has slope `D_p` and crosses zero at `E_c`, so
//! `fit` recovers both by least squares. Each layer must cure deeper than it
//! is thick (overcure) to bond to the one above; the bottom layers get a
//! multiple of the normal exposure, which adds `D_p · ln(multiple)` of depth.
//!
//! The defaults are illustrative only — not any real resin's data. Use the
//! resin maker's values or your own staircase test, then confirm with the
//! printer maker's exposure calibration print.
//!
//! References: P. F. Jacobs, *Rapid Prototyping & Manufacturing:
//! Fundamentals of Stereolithography* (Society of Manufacturing Engineers,
//! 1992); I. Gibson, D. Rosen, B. Stucker, *Additive Manufacturing
//! Technologies* (3rd ed., Springer, 2021), vat photopolymerisation chapter.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ResinIn {
    /// Penetration depth D_p, mm.
    pub dp_mm: f64,
    /// Critical exposure E_c, mJ/cm².
    pub ec_mj_cm2: f64,
    /// Irradiance at the vat floor, mW/cm².
    pub irradiance_mw_cm2: f64,
    pub layer_mm: f64,
    /// Cure depth as a multiple of the layer height (overcure for bonding).
    pub cure_ratio: f64,
    /// Bottom-layer exposure as a multiple of the normal exposure.
    pub bottom_multiple: f64,
    /// Measured staircase: `[exposure s, cured thickness mm]` at the same irradiance.
    /// Two or more points replace `dp_mm` / `ec_mj_cm2` with the fitted values.
    pub points: Vec<[f64; 2]>,
}

impl Default for ResinIn {
    fn default() -> Self {
        ResinIn {
            dp_mm: 0.12,
            ec_mj_cm2: 6.0,
            irradiance_mw_cm2: 3.0,
            layer_mm: 0.05,
            cure_ratio: 1.5,
            bottom_multiple: 8.0,
            points: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Fit {
    pub dp_mm: f64,
    pub ec_mj_cm2: f64,
    /// Coefficient of determination of the straight-line fit.
    pub r2: f64,
    pub n: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct LayerRow {
    pub layer_mm: f64,
    pub cure_mm: f64,
    pub exposure_mj_cm2: f64,
    pub time_s: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResinOut {
    pub dp_mm: f64,
    pub ec_mj_cm2: f64,
    pub fitted: Option<Fit>,
    pub cure_mm: f64,
    pub exposure_mj_cm2: f64,
    pub time_s: f64,
    pub bottom_time_s: f64,
    pub bottom_cure_mm: f64,
    /// Working curve for the plot: E (mJ/cm²) and C_d (mm).
    pub curve_e: Vec<f64>,
    pub curve_cd: Vec<f64>,
    pub table: Vec<LayerRow>,
    pub steps: Vec<String>,
    pub warnings: Vec<String>,
}

/// Cure depth for a dose (0 below the critical exposure).
pub fn cure_depth(dp: f64, ec: f64, e: f64) -> f64 {
    if e <= ec {
        0.0
    } else {
        dp * (e / ec).ln()
    }
}

/// Dose that cures to depth `cd`.
pub fn dose_for(dp: f64, ec: f64, cd: f64) -> f64 {
    ec * (cd / dp).exp()
}

/// Least-squares line C_d = D_p · ln E − D_p · ln E_c through measured points.
pub fn fit(points: &[[f64; 2]], irradiance: f64) -> Result<Fit, String> {
    let pts: Vec<(f64, f64)> = points
        .iter()
        .filter(|p| p[0] > 0.0 && p[1] > 0.0)
        .map(|p| ((p[0] * irradiance).ln(), p[1]))
        .collect();
    if pts.len() < 2 {
        return Err("need at least two staircase points with positive time and thickness".into());
    }
    let n = pts.len() as f64;
    let mx = pts.iter().map(|p| p.0).sum::<f64>() / n;
    let my = pts.iter().map(|p| p.1).sum::<f64>() / n;
    let sxx: f64 = pts.iter().map(|p| (p.0 - mx).powi(2)).sum();
    let sxy: f64 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    if sxx < 1e-12 {
        return Err("staircase exposures must differ".into());
    }
    let slope = sxy / sxx;
    if slope <= 0.0 {
        return Err("thickness must grow with exposure — check the measurements".into());
    }
    let icpt = my - slope * mx;
    let syy: f64 = pts.iter().map(|p| (p.1 - my).powi(2)).sum();
    let ssr: f64 = pts.iter().map(|p| (p.1 - (icpt + slope * p.0)).powi(2)).sum();
    Ok(Fit {
        dp_mm: slope,
        ec_mj_cm2: (-icpt / slope).exp(),
        r2: if syy > 0.0 { 1.0 - ssr / syy } else { 1.0 },
        n: pts.len(),
    })
}

pub fn exposure(i: &ResinIn) -> Result<ResinOut, String> {
    if i.irradiance_mw_cm2 <= 0.0 || i.layer_mm <= 0.0 {
        return Err("irradiance and layer height must be positive".into());
    }
    let fitted = if i.points.len() >= 2 {
        Some(fit(&i.points, i.irradiance_mw_cm2)?)
    } else {
        None
    };
    let (dp, ec) = match &fitted {
        Some(f) => (f.dp_mm, f.ec_mj_cm2),
        None => (i.dp_mm, i.ec_mj_cm2),
    };
    if dp <= 0.0 || ec <= 0.0 {
        return Err("D_p and E_c must be positive".into());
    }
    let ratio = i.cure_ratio.max(1.0);
    let cd = ratio * i.layer_mm;
    let e = dose_for(dp, ec, cd);
    let t = e / i.irradiance_mw_cm2;
    let bm = i.bottom_multiple.max(1.0);
    let bcd = cure_depth(dp, ec, e * bm);
    let mut curve_e = Vec::new();
    let mut curve_cd = Vec::new();
    let (e0, e1) = (ec * 0.8, (e * bm * 1.5).max(ec * 20.0));
    for k in 0..=80 {
        let x = e0 * (e1 / e0).powf(k as f64 / 80.0);
        curve_e.push(x);
        curve_cd.push(cure_depth(dp, ec, x));
    }
    let table = [0.025, 0.035, 0.05, 0.075, 0.1]
        .iter()
        .map(|&l| {
            let c = ratio * l;
            let ex = dose_for(dp, ec, c);
            LayerRow {
                layer_mm: l,
                cure_mm: c,
                exposure_mj_cm2: ex,
                time_s: ex / i.irradiance_mw_cm2,
            }
        })
        .collect();
    let mut warnings = Vec::new();
    if cd > 3.0 * dp {
        warnings.push(format!(
            "Cure depth {cd:.3} mm is more than 3 D_p ({:.3} mm): the dose grows exponentially — thinner layers or a more transparent resin print faster.",
            3.0 * dp
        ));
    }
    if ratio < 1.2 {
        warnings.push("Little overcure: layers may not bond — delamination risk.".into());
    }
    if t < 0.5 {
        warnings.push("Very short exposure: light bleed and timing jitter matter — check dimensions with a calibration print.".into());
    }
    if fitted.as_ref().is_some_and(|f| f.r2 < 0.95) {
        warnings.push("The staircase points scatter (R² < 0.95): re-measure with a micrometer, several points per step.".into());
    }
    let steps = vec![
        format!(
            "Cure depth C_d = {ratio:.2} × {:.3} mm = {cd:.4} mm (overcure so layers bond)",
            i.layer_mm
        ),
        format!("E = E_c · exp(C_d/D_p) = {ec:.2} · exp({cd:.4}/{dp:.4}) = {e:.2} mJ/cm²"),
        format!("t = E / I = {e:.2} / {:.2} = {t:.2} s per layer", i.irradiance_mw_cm2),
        format!(
            "Bottom layers: {bm:.0}× → {:.1} s, cure depth {bcd:.3} mm (= C_d + D_p·ln {bm:.0})",
            t * bm
        ),
    ];
    Ok(ResinOut {
        dp_mm: dp,
        ec_mj_cm2: ec,
        fitted,
        cure_mm: cd,
        exposure_mj_cm2: e,
        time_s: t,
        bottom_time_s: t * bm,
        bottom_cure_mm: bcd,
        curve_e,
        curve_cd,
        table,
        steps,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn working_curve_hand_calc() {
        // D_p = 0.1 mm, E_c = 10 mJ/cm²: E = 10·e → C_d = 0.1 mm exactly
        assert!((cure_depth(0.1, 10.0, 10.0 * std::f64::consts::E) - 0.1).abs() < 1e-12);
        assert_eq!(cure_depth(0.1, 10.0, 5.0), 0.0);
        assert!((dose_for(0.1, 10.0, 0.2) - 10.0 * (2.0f64).exp()).abs() < 1e-9);
        // defaults: C_d = 0.075, E = 6·exp(0.625) = 11.2 mJ/cm², t = 3.74 s
        let o = exposure(&ResinIn::default()).unwrap();
        assert!((o.cure_mm - 0.075).abs() < 1e-12);
        assert!((o.exposure_mj_cm2 - 6.0 * (0.625f64).exp()).abs() < 1e-9);
        assert!((o.time_s - o.exposure_mj_cm2 / 3.0).abs() < 1e-12);
        // bottom: extra depth D_p·ln 8
        assert!((o.bottom_cure_mm - (0.075 + 0.12 * 8f64.ln())).abs() < 1e-9);
        assert_eq!(o.curve_e.len(), 81);
    }

    #[test]
    fn staircase_fit_recovers_resin() {
        let (dp, ec, irr) = (0.15, 8.0, 2.5);
        let pts: Vec<[f64; 2]> = [4.0, 6.0, 9.0, 14.0, 20.0]
            .iter()
            .map(|&t| [t, cure_depth(dp, ec, t * irr)])
            .collect();
        let f = fit(&pts, irr).unwrap();
        assert!((f.dp_mm - dp).abs() < 1e-9 && (f.ec_mj_cm2 - ec).abs() < 1e-6 && f.r2 > 0.999999);
        let o = exposure(&ResinIn {
            points: pts,
            irradiance_mw_cm2: irr,
            ..Default::default()
        })
        .unwrap();
        assert!(o.fitted.is_some() && (o.dp_mm - dp).abs() < 1e-9);
        assert!(fit(&[[5.0, 0.1]], 3.0).is_err());
        assert!(fit(&[[5.0, 0.2], [10.0, 0.1]], 3.0).is_err());
    }
}
