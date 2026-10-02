//! Convection and radiation correlations: Nusselt number → heat-transfer
//! coefficient `h = Nu k / L`.
//!
//! Natural convection is driven by buoyancy; its strength is the Rayleigh
//! number `Ra_L = g β ΔT L³ / (ν α)` (β = 1/T_film for an ideal gas).
//! Forced convection scales with the Reynolds number `Re = V L / ν`.
//! Properties are evaluated at the film temperature `T_f = (T_s + T_∞)/2`.
//!
//! | Geometry | Correlation | Range |
//! |---|---|---|
//! | Vertical plate, height L | Churchill & Chu (1975a): `Nu = {0.825 + 0.387 Ra^{1/6} / [1 + (0.492/Pr)^{9/16}]^{8/27}}²` | all Ra (≤ 10¹²) |
//! | Vertical plate, laminar | Churchill & Chu (1975a): `Nu = 0.68 + 0.670 Ra^{1/4} / [1 + (0.492/Pr)^{9/16}]^{4/9}` | Ra ≤ 10⁹ |
//! | Horizontal cylinder, diameter D | Churchill & Chu (1975b): `Nu = {0.60 + 0.387 Ra^{1/6} / [1 + (0.559/Pr)^{9/16}]^{8/27}}²` | Ra ≤ 10¹² |
//! | Hot plate facing up, L = A/P | `Nu = 0.54 Ra^{1/4}` (10⁴–10⁷), `0.15 Ra^{1/3}` (10⁷–10¹¹) | Incropera eq. 9.30–9.31 |
//! | Hot plate facing down, L = A/P | `Nu = 0.52 Ra^{1/5}` | 10⁴–10⁹ (Incropera eq. 9.32) |
//! | Cylinder in cross-flow | Churchill & Bernstein (1977): `Nu = 0.3 + 0.62 Re^{1/2} Pr^{1/3} / [1 + (0.4/Pr)^{2/3}]^{1/4} · [1 + (Re/282 000)^{5/8}]^{4/5}` | Re·Pr ≥ 0.2 |
//! | Flat plate, parallel flow, laminar | `Nu = 0.664 Re^{1/2} Pr^{1/3}` | Re ≤ 5·10⁵ |
//! | Mixed convection | `Nu³ = Nu_F³ + Nu_N³` (Churchill's blending, Incropera eq. 9.64) | assisting / transverse |
//! | Radiation to large surroundings | `h_r = ε σ (T_s + T_sur)(T_s² + T_sur²)` | grey surface |
//!
//! Small wearables sit at Ra ~ 10²–10⁴ — below the validated range of the
//! horizontal-plate fits, so results there are extrapolations and flagged.
//!
//! References: S. W. Churchill & H. H. S. Chu, "Correlating equations for
//! laminar and turbulent free convection from a vertical plate", Int. J. Heat
//! Mass Transfer 18 (1975) 1323–1329; "… from a horizontal cylinder", 18
//! (1975) 1049–1053; S. W. Churchill & M. Bernstein, "A correlating equation
//! for forced convection from gases and liquids to a circular cylinder in
//! crossflow", J. Heat Transfer 99 (1977) 300–306; Incropera et al.,
//! *Fundamentals of Heat and Mass Transfer*, 7th ed., ch. 7 and 9;
//! J. P. Holman, *Heat Transfer*, 10th ed. (simplified air equations).

use serde::{Deserialize, Serialize};

use super::props::{air, Air, G, K0, SIGMA};
use crate::project::Project;

/// Rayleigh number for a length `l_m` and surface excess `dt` (K).
pub fn rayleigh(a: &Air, dt: f64, l_m: f64) -> f64 {
    G * a.beta * dt.abs() * l_m.powi(3) / (a.nu * a.alpha)
}

pub fn nu_vertical_plate(ra: f64, pr: f64) -> f64 {
    let psi = (1.0 + (0.492 / pr).powf(9.0 / 16.0)).powf(8.0 / 27.0);
    let x = 0.825 + 0.387 * ra.max(0.0).powf(1.0 / 6.0) / psi;
    x * x
}

pub fn nu_vertical_plate_laminar(ra: f64, pr: f64) -> f64 {
    0.68 + 0.670 * ra.max(0.0).powf(0.25) / (1.0 + (0.492 / pr).powf(9.0 / 16.0)).powf(4.0 / 9.0)
}

pub fn nu_horizontal_cylinder(ra: f64, pr: f64) -> f64 {
    let psi = (1.0 + (0.559 / pr).powf(9.0 / 16.0)).powf(8.0 / 27.0);
    let x = 0.60 + 0.387 * ra.max(0.0).powf(1.0 / 6.0) / psi;
    x * x
}

/// Upper surface of a hot plate (or lower surface of a cold one).
pub fn nu_plate_up(ra: f64) -> f64 {
    if ra <= 1e7 {
        0.54 * ra.max(0.0).powf(0.25)
    } else {
        0.15 * ra.powf(1.0 / 3.0)
    }
}

/// Lower surface of a hot plate.
pub fn nu_plate_down(ra: f64) -> f64 {
    0.52 * ra.max(0.0).powf(0.2)
}

pub fn nu_churchill_bernstein(re: f64, pr: f64) -> f64 {
    let re = re.max(0.0);
    let a = 0.62 * re.sqrt() * pr.powf(1.0 / 3.0) / (1.0 + (0.4 / pr).powf(2.0 / 3.0)).powf(0.25);
    0.3 + a * (1.0 + (re / 282_000.0).powf(5.0 / 8.0)).powf(0.8)
}

pub fn nu_flat_plate_laminar(re: f64, pr: f64) -> f64 {
    0.664 * re.max(0.0).sqrt() * pr.powf(1.0 / 3.0)
}

/// Radiation coefficient to large surroundings, W/(m² K).
pub fn h_radiation(eps: f64, ts_k: f64, tsur_k: f64) -> f64 {
    eps.clamp(0.0, 1.0) * SIGMA * (ts_k + tsur_k) * (ts_k * ts_k + tsur_k * tsur_k)
}

/// Which face of a body a correlation applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaceClass {
    Vertical,
    Up,
    Down,
}

/// Natural-convection coefficient for one face class of a small body:
/// vertical faces use Churchill–Chu with the body height, up/down faces the
/// horizontal-plate fits with L = A/P. Returns (h, Ra, Nu, in validated range).
pub fn h_natural(class: FaceClass, dt: f64, t_amb_c: f64, l_m: f64) -> (f64, f64, f64, bool) {
    let dt = dt.max(1e-3);
    let a = air(t_amb_c + K0 + dt / 2.0);
    let l = l_m.max(1e-4);
    let ra = rayleigh(&a, dt, l);
    let (nu, ok) = match class {
        FaceClass::Vertical => (nu_vertical_plate(ra, a.pr), ra <= 1e12),
        FaceClass::Up => (nu_plate_up(ra), (1e4..=1e11).contains(&ra)),
        FaceClass::Down => (nu_plate_down(ra), (1e4..=1e9).contains(&ra)),
    };
    (nu * a.k / l, ra, nu, ok)
}

/// Forced laminar flat-plate coefficient for flow of speed `v` along length `l_m`.
pub fn h_forced_plate(v: f64, t_film_c: f64, l_m: f64) -> (f64, f64, f64) {
    let a = air(t_film_c + K0);
    let l = l_m.max(1e-4);
    let re = v.max(0.0) * l / a.nu;
    let nu = nu_flat_plate_laminar(re, a.pr);
    (nu * a.k / l, re, nu)
}

/// Mixed convection: Nu³ = Nu_F³ + Nu_N³ applied to h (same length scale).
pub fn blend(h_nat: f64, h_forced: f64) -> f64 {
    (h_nat.powi(3) + h_forced.powi(3)).cbrt()
}

// ------------------------------------------------------------------ calculator API

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ConvIn {
    pub project: Option<Project>,
    pub t_surface_c: Option<f64>,
    pub t_ambient_c: Option<f64>,
    pub emissivity: Option<f64>,
    /// Vertical plate height, mm (default: pod height).
    pub plate_height_mm: Option<f64>,
    /// Horizontal plate characteristic length A/P, mm (default: pod top).
    pub plate_ap_mm: Option<f64>,
    /// Cylinder diameter, mm (default: temple hydraulic diameter).
    pub diameter_mm: Option<f64>,
    /// Flat-plate length along the flow, mm (default: pod length).
    pub flow_length_mm: Option<f64>,
    /// Air speed for the forced correlations, m/s (default 1.4, walking).
    pub speed_m_s: Option<f64>,
    /// Largest surface excess for the sweep, K.
    pub dt_max: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CorrRow {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub length_mm: f64,
    /// Ra (natural) or Re (forced).
    pub group: f64,
    pub group_name: String,
    pub nu: f64,
    pub h: f64,
    pub valid: bool,
    pub range: String,
    pub equation: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Curve {
    pub id: String,
    pub name: String,
    pub h: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConvOut {
    pub t_surface_c: f64,
    pub t_ambient_c: f64,
    pub t_film_c: f64,
    pub film: Air,
    pub emissivity: f64,
    pub speed_m_s: f64,
    pub rows: Vec<CorrRow>,
    pub h_rad: f64,
    /// Radiation + the vertical-plate natural value (what a still-air pod side sees).
    pub h_still_total: f64,
    /// Radiation + mixed (vertical plate natural ⊕ flat-plate forced).
    pub h_walking_total: f64,
    pub dt: Vec<f64>,
    pub curves: Vec<Curve>,
    pub h_rad_curve: Vec<f64>,
    /// Heat flux at the requested state for each total, W/m².
    pub q_still_w_m2: f64,
    pub q_walking_w_m2: f64,
    pub notes: Vec<String>,
}

pub fn calculator(i: ConvIn) -> Result<ConvOut, String> {
    let p = i.project.unwrap_or_else(Project::iris);
    let f = &p.design.frame;
    let ta = i.t_ambient_c.unwrap_or(p.design.thermal.ambient_c);
    let ts = i.t_surface_c.unwrap_or(p.design.thermal.skin_limit_c);
    let eps = i.emissivity.unwrap_or(0.9).clamp(0.0, 1.0);
    let lv = i.plate_height_mm.unwrap_or(f.pod_height_mm).max(0.1) * 1e-3;
    let lap = i
        .plate_ap_mm
        .unwrap_or(f.pod_length_mm * f.pod_width_mm / (2.0 * (f.pod_length_mm + f.pod_width_mm)))
        .max(0.1)
        * 1e-3;
    let d = i
        .diameter_mm
        .unwrap_or(2.0 * f.temple_width_mm * f.temple_height_mm / (f.temple_width_mm + f.temple_height_mm))
        .max(0.1)
        * 1e-3;
    let lf = i.flow_length_mm.unwrap_or(f.pod_length_mm).max(0.1) * 1e-3;
    let v = i.speed_m_s.unwrap_or(1.4).max(0.0);
    if !(ta.is_finite() && ts.is_finite()) {
        return Err("temperatures must be numbers".into());
    }
    let rows_at = |dt: f64| -> Vec<CorrRow> {
        let tf = ta + K0 + dt / 2.0;
        let a = air(tf);
        let mut rows = Vec::new();
        let ra_v = rayleigh(&a, dt, lv);
        rows.push(CorrRow {
            id: "vplate".into(),
            name: "Vertical plate (Churchill–Chu, full range)".into(),
            kind: "natural".into(),
            length_mm: lv * 1e3,
            group: ra_v,
            group_name: "Ra".into(),
            nu: nu_vertical_plate(ra_v, a.pr),
            h: nu_vertical_plate(ra_v, a.pr) * a.k / lv,
            valid: ra_v <= 1e12,
            range: "Ra ≤ 10¹²".into(),
            equation: "Nu = {0.825 + 0.387 Ra^(1/6) / [1 + (0.492/Pr)^(9/16)]^(8/27)}²".into(),
        });
        rows.push(CorrRow {
            id: "vplate_lam".into(),
            name: "Vertical plate (Churchill–Chu, laminar)".into(),
            kind: "natural".into(),
            length_mm: lv * 1e3,
            group: ra_v,
            group_name: "Ra".into(),
            nu: nu_vertical_plate_laminar(ra_v, a.pr),
            h: nu_vertical_plate_laminar(ra_v, a.pr) * a.k / lv,
            valid: ra_v <= 1e9,
            range: "Ra ≤ 10⁹".into(),
            equation: "Nu = 0.68 + 0.670 Ra^(1/4) / [1 + (0.492/Pr)^(9/16)]^(4/9)".into(),
        });
        let ra_d = rayleigh(&a, dt, d);
        rows.push(CorrRow {
            id: "hcyl".into(),
            name: "Horizontal cylinder — temple (Churchill–Chu)".into(),
            kind: "natural".into(),
            length_mm: d * 1e3,
            group: ra_d,
            group_name: "Ra_D".into(),
            nu: nu_horizontal_cylinder(ra_d, a.pr),
            h: nu_horizontal_cylinder(ra_d, a.pr) * a.k / d,
            valid: ra_d <= 1e12,
            range: "Ra_D ≤ 10¹²".into(),
            equation: "Nu = {0.60 + 0.387 Ra^(1/6) / [1 + (0.559/Pr)^(9/16)]^(8/27)}²".into(),
        });
        let ra_p = rayleigh(&a, dt, lap);
        rows.push(CorrRow {
            id: "hplate_up".into(),
            name: "Horizontal plate, hot side up".into(),
            kind: "natural".into(),
            length_mm: lap * 1e3,
            group: ra_p,
            group_name: "Ra (L = A/P)".into(),
            nu: nu_plate_up(ra_p),
            h: nu_plate_up(ra_p) * a.k / lap,
            valid: (1e4..=1e11).contains(&ra_p),
            range: "10⁴ ≤ Ra ≤ 10¹¹".into(),
            equation: "Nu = 0.54 Ra^(1/4)  (0.15 Ra^(1/3) above 10⁷)".into(),
        });
        rows.push(CorrRow {
            id: "hplate_down".into(),
            name: "Horizontal plate, hot side down".into(),
            kind: "natural".into(),
            length_mm: lap * 1e3,
            group: ra_p,
            group_name: "Ra (L = A/P)".into(),
            nu: nu_plate_down(ra_p),
            h: nu_plate_down(ra_p) * a.k / lap,
            valid: (1e4..=1e9).contains(&ra_p),
            range: "10⁴ ≤ Ra ≤ 10⁹".into(),
            equation: "Nu = 0.52 Ra^(1/5)".into(),
        });
        let re_d = v * d / a.nu;
        rows.push(CorrRow {
            id: "cb".into(),
            name: "Cylinder in cross-flow (Churchill–Bernstein)".into(),
            kind: "forced".into(),
            length_mm: d * 1e3,
            group: re_d,
            group_name: "Re_D".into(),
            nu: nu_churchill_bernstein(re_d, a.pr),
            h: nu_churchill_bernstein(re_d, a.pr) * a.k / d,
            valid: re_d * a.pr >= 0.2,
            range: "Re·Pr ≥ 0.2".into(),
            equation: "Nu = 0.3 + 0.62 Re^(1/2) Pr^(1/3) / [1+(0.4/Pr)^(2/3)]^(1/4) · [1 + (Re/282000)^(5/8)]^(4/5)".into(),
        });
        let re_l = v * lf / a.nu;
        rows.push(CorrRow {
            id: "fplate".into(),
            name: "Flat plate, flow along the pod (laminar)".into(),
            kind: "forced".into(),
            length_mm: lf * 1e3,
            group: re_l,
            group_name: "Re_L".into(),
            nu: nu_flat_plate_laminar(re_l, a.pr),
            h: nu_flat_plate_laminar(re_l, a.pr) * a.k / lf,
            valid: re_l <= 5e5,
            range: "Re ≤ 5·10⁵, Pr ≥ 0.6".into(),
            equation: "Nu = 0.664 Re^(1/2) Pr^(1/3)".into(),
        });
        rows
    };
    let dt0 = (ts - ta).abs().max(0.01);
    let rows = rows_at(dt0);
    let tf = (ts + ta) / 2.0;
    let film = air(tf + K0);
    let h_rad = h_radiation(eps, ts + K0, ta + K0);
    let hv = rows[0].h;
    let hf = rows[6].h;
    let h_still_total = hv + h_rad;
    let h_walking_total = blend(hv, hf) + h_rad;
    let dt_max = i.dt_max.unwrap_or(30.0).clamp(1.0, 200.0);
    let dts: Vec<f64> = (0..60).map(|k| dt_max * (k as f64 + 1.0) / 60.0).collect();
    let mut curves: Vec<Curve> = rows
        .iter()
        .map(|r| Curve {
            id: r.id.clone(),
            name: r.name.clone(),
            h: Vec::with_capacity(dts.len()),
        })
        .collect();
    let mut h_rad_curve = Vec::new();
    for &dt in &dts {
        for (c, r) in curves.iter_mut().zip(rows_at(dt)) {
            c.h.push(r.h);
        }
        h_rad_curve.push(h_radiation(eps, ta + dt + K0, ta + K0));
    }
    let mut notes = Vec::new();
    for r in &rows {
        if !r.valid {
            notes.push(format!(
                "{}: {} = {:.3e} is outside the validated range ({}) — extrapolated.",
                r.name, r.group_name, r.group, r.range
            ));
        }
    }
    if film.clamped {
        notes.push("Film temperature outside the 250–400 K property table; properties clamped.".into());
    }
    Ok(ConvOut {
        t_surface_c: ts,
        t_ambient_c: ta,
        t_film_c: tf,
        film,
        emissivity: eps,
        speed_m_s: v,
        h_rad,
        h_still_total,
        h_walking_total,
        q_still_w_m2: h_still_total * (ts - ta),
        q_walking_w_m2: h_walking_total * (ts - ta),
        rows,
        dt: dts,
        curves,
        h_rad_curve,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn churchill_chu_hand_value_and_limits() {
        // Ra = 1e9, Pr = 0.71 → hand calculation ≈ 122.8
        let nu = nu_vertical_plate(1e9, 0.71);
        assert!((nu - 122.8).abs() < 0.5, "{nu}");
        // conduction limit: Ra → 0 gives 0.825² for the plate, 0.36 for the cylinder
        assert!((nu_vertical_plate(0.0, 0.7) - 0.825f64.powi(2)).abs() < 1e-12);
        assert!((nu_horizontal_cylinder(0.0, 0.7) - 0.36).abs() < 1e-12);
        // laminar and full-range forms agree within a few percent at Ra = 1e6
        let a = nu_vertical_plate(1e6, 0.71);
        let b = nu_vertical_plate_laminar(1e6, 0.71);
        assert!((a - b).abs() / b < 0.06, "{a} {b}");
    }

    #[test]
    fn holman_simplified_air_equations() {
        // Holman, Heat Transfer, simplified laminar equations for air at 1 atm:
        // vertical plane h = 1.42 (ΔT/L)^(1/4); horizontal cylinder h = 1.32 (ΔT/d)^(1/4).
        let (dt, l) = (20.0, 0.3);
        let (h, ra, _, ok) = h_natural(FaceClass::Vertical, dt, 20.0, l);
        assert!(ok && ra > 1e4 && ra < 1e9, "{ra}");
        let holman = 1.42 * (dt / l).powf(0.25);
        assert!((h - holman).abs() / holman < 0.12, "{h} vs {holman}");
        // Horizontal cylinder: correlations differ by 10–20 % here. Against
        // Morgan's power law (Incropera Table 9.1, 10⁴ ≤ Ra ≤ 10⁷: Nu = 0.48 Ra^¼)
        // within 12 %, against Holman's simplified h = 1.32 (ΔT/d)^¼ within 20 %.
        let d = 0.05;
        let a = air(20.0 + K0 + dt / 2.0);
        let ra = rayleigh(&a, dt, d);
        assert!(ra > 1e4 && ra < 1e7, "{ra}");
        let nu = nu_horizontal_cylinder(ra, a.pr);
        let morgan = 0.48 * ra.powf(0.25);
        assert!((nu - morgan).abs() / morgan < 0.12, "{nu} vs {morgan}");
        let hc = nu * a.k / d;
        let holman_c = 1.32 * (dt / d).powf(0.25);
        assert!((hc - holman_c).abs() / holman_c < 0.2, "{hc} vs {holman_c}");
    }

    #[test]
    fn churchill_bernstein_vs_hilpert() {
        // Hilpert / Zukauskas table, 4000 < Re < 40000: Nu = 0.193 Re^0.618 Pr^(1/3)
        let (re, pr) = (1e4, 0.7);
        let cb = nu_churchill_bernstein(re, pr);
        let hilpert = 0.193 * re.powf(0.618) * pr.powf(1.0 / 3.0);
        assert!((cb - hilpert).abs() / hilpert < 0.1, "{cb} vs {hilpert}");
        // hand value ≈ 53.3
        assert!((cb - 53.3).abs() < 0.5, "{cb}");
    }

    #[test]
    fn radiation_linearisation() {
        // ε = 1, 300 K both sides → 4σT³ ≈ 6.12 W/m²K
        let h = h_radiation(1.0, 300.0, 300.0);
        assert!((h - 4.0 * SIGMA * 300f64.powi(3)).abs() < 1e-9);
        assert!((h - 6.12).abs() < 0.01);
    }

    #[test]
    fn calculator_for_iris() {
        let o = calculator(ConvIn::default()).unwrap();
        assert_eq!(o.rows.len(), 7);
        assert!(o.h_rad > 5.0 && o.h_rad < 7.0, "{}", o.h_rad);
        // small wearable in still air: natural h of order 5–15 W/m²K
        assert!(o.rows[0].h > 4.0 && o.rows[0].h < 15.0, "{}", o.rows[0].h);
        assert!(o.h_walking_total > o.h_still_total);
        // the pod-top plate is far below Ra = 1e4 → flagged
        assert!(!o.rows[3].valid);
        assert!(!o.notes.is_empty());
        assert_eq!(o.curves[0].h.len(), o.dt.len());
        // h grows with ΔT for natural convection
        assert!(o.curves[0].h[59] > o.curves[0].h[0]);
    }
}
