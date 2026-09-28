//! Extrusion and motion-time models.
//!
//! **Extrusion.** A bead of width `w` laid at height `h` is modelled as a
//! rectangle with semicircular sides (the Slic3r "rounded rectangle" flow
//! model): A = (w − h)·h + π(h/2)². Feeding a filament of diameter `d`, the
//! extruder axis must advance E = A·L / (π d²/4) for a move of length L.
//!
//! **Time.** Each move is a trapezoidal velocity profile: accelerate at `a`
//! from the junction speed v_j to the cruise speed v, cruise, decelerate:
//! t = L/v + (v − v_j)²/(a·v) when the move is long enough to reach v,
//! otherwise the triangular profile t = 2(v_p − v_j)/a with
//! v_p = √(v_j² + a·L). Polylines are split into runs at corners sharper
//! than 30° (where firmware must slow to the corner speed); gentle curves
//! are treated as one continuous run. Retractions, layer changes and a
//! minimum layer time (the slicer slows small layers so plastic can cool)
//! are added on top. Real firmware look-ahead (Marlin junction deviation,
//! Klipper square-corner velocity) is more detailed, so treat results as
//! ±10–20 % estimates.
//!
//! References: Slic3r manual, "Flow math"; Klipper documentation,
//! "Kinematics" (trapezoid generator, square corner velocity); Marlin
//! G-code reference (https://marlinfw.org/meta/gcode/).

use serde::Serialize;

use super::slicer::{Kind, Layer, SliceSettings};
use crate::cad::geom2d::{self as g, P2};
use crate::math::PI;
use crate::project::PrintParams;

/// Cross-section of a bead (mm²).
pub fn bead_area(w: f64, h: f64) -> f64 {
    if w <= h {
        PI * (w / 2.0).powi(2)
    } else {
        (w - h) * h + PI * (h / 2.0).powi(2)
    }
}

/// Filament length per mm of bead.
pub fn e_per_mm(w: f64, h: f64, filament_d: f64) -> f64 {
    bead_area(w, h) / (PI * filament_d * filament_d / 4.0)
}

/// Time of one straight move with a trapezoidal velocity profile.
pub fn trapezoid_time(l: f64, v: f64, vj: f64, a: f64) -> f64 {
    if l <= 0.0 || v <= 0.0 {
        return 0.0;
    }
    let vj = vj.min(v);
    if a <= 0.0 {
        return l / v;
    }
    let d_acc = (v * v - vj * vj) / (2.0 * a);
    if 2.0 * d_acc <= l {
        l / v + (v - vj).powi(2) / (a * v)
    } else {
        let vp = (vj * vj + a * l).sqrt();
        2.0 * (vp - vj) / a
    }
}

/// Time along a polyline: runs split at corners sharper than 30°.
pub fn polyline_time(pts: &[P2], v: f64, vj: f64, a: f64) -> f64 {
    if pts.len() < 2 {
        return 0.0;
    }
    let cos_limit = 30f64.to_radians().cos();
    let mut t = 0.0;
    let mut run = 0.0;
    for i in 1..pts.len() {
        let seg = g::dist(pts[i - 1], pts[i]);
        run += seg;
        if i + 1 < pts.len() {
            let d1 = g::sub(pts[i], pts[i - 1]);
            let d2 = g::sub(pts[i + 1], pts[i]);
            let den = g::len(d1) * g::len(d2);
            let c = if den > 0.0 { g::dot(d1, d2) / den } else { 1.0 };
            if c < cos_limit {
                t += trapezoid_time(run, v, vj, a);
                run = 0.0;
            }
        }
    }
    t + trapezoid_time(run, v, vj, a)
}

pub fn path_len(pts: &[P2]) -> f64 {
    pts.windows(2).map(|w| g::dist(w[0], w[1])).sum()
}

/// Feed rate for a move kind, mm/s.
pub fn speed(kind: Kind, layer: usize, pp: &PrintParams, st: &SliceSettings) -> f64 {
    let base = pp.print_speed_mm_s.clamp(1.0, 1000.0);
    let v = match kind {
        Kind::Travel => return pp.travel_speed_mm_s.clamp(1.0, 2000.0),
        Kind::ExternalPerimeter => base * st.outer_wall_speed_pct / 100.0,
        Kind::Perimeter | Kind::Skirt | Kind::SolidInfill => base,
        Kind::SparseInfill => base * st.infill_speed_pct / 100.0,
    };
    if layer == 0 { v * st.first_layer_speed_pct / 100.0 } else { v }.max(1.0)
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct LayerStats {
    pub index: usize,
    pub z: f64,
    pub height: f64,
    /// Final layer time including the minimum-layer-time slowdown.
    pub time_s: f64,
    pub raw_time_s: f64,
    pub slowed: bool,
    /// Length per move kind (index = `Kind` code), mm.
    pub length_mm: [f64; 6],
    pub time_by_kind_s: [f64; 6],
    pub e_mm: f64,
    pub volume_mm3: f64,
    pub retracts: usize,
    pub area_mm2: f64,
}

pub fn layer_stats(layer: &Layer, pp: &PrintParams, st: &SliceSettings) -> LayerStats {
    let mut s = LayerStats {
        index: layer.index,
        z: layer.z,
        height: layer.height,
        area_mm2: layer.area_mm2,
        ..Default::default()
    };
    let w = pp.line_width_mm;
    let epm = e_per_mm(w, layer.height, pp.filament_diameter_mm);
    let area = bead_area(w, layer.height);
    for p in &layer.paths {
        let l = path_len(&p.pts);
        let k = p.kind.code() as usize;
        s.length_mm[k] += l;
        let v = speed(p.kind, layer.index, pp, st);
        let t = if p.kind == Kind::Travel {
            trapezoid_time(l, v, st.corner_speed_mm_s, st.accel_mm_s2)
        } else {
            polyline_time(&p.pts, v, st.corner_speed_mm_s, st.accel_mm_s2)
        };
        s.time_by_kind_s[k] += t;
        if p.kind != Kind::Travel {
            s.e_mm += l * epm;
            s.volume_mm3 += l * area;
        } else if p.retract && st.retract_mm > 0.0 {
            s.retracts += 1;
        }
    }
    let retract_t = s.retracts as f64 * 2.0 * st.retract_mm / st.retract_speed_mm_s.max(1.0);
    let layer_change = 0.1 + layer.height / 10.0;
    s.raw_time_s = s.time_by_kind_s.iter().sum::<f64>() + retract_t + layer_change;
    s.time_s = s.raw_time_s.max(st.min_layer_time_s);
    s.slowed = s.raw_time_s < st.min_layer_time_s;
    s
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SliceStats {
    pub layers: usize,
    pub time_s: f64,
    pub time_h: f64,
    pub time_text: String,
    /// Extra time from minimum-layer-time slowdowns, s.
    pub cooling_s: f64,
    pub slowed_layers: usize,
    pub time_by_kind_s: [f64; 6],
    pub length_by_kind_mm: [f64; 6],
    pub filament_mm: f64,
    pub filament_m: f64,
    pub filament_cm3: f64,
    pub filament_g: f64,
    pub cost_usd: f64,
    pub part_volume_mm3: f64,
    /// Extruded volume / part volume.
    pub fill_ratio: f64,
    pub retracts: usize,
    pub density_g_cm3: f64,
    pub max_z_mm: f64,
}

pub fn fmt_duration(s: f64) -> String {
    let m = (s / 60.0).round() as i64;
    if m >= 60 {
        format!("{}h {:02}m", m / 60, m % 60)
    } else {
        format!("{m} min")
    }
}

pub fn totals(
    per_layer: &[LayerStats],
    weights: Option<&[f64]>,
    density: f64,
    cost_per_kg: f64,
    part_volume: f64,
    filament_d: f64,
) -> SliceStats {
    let mut s = SliceStats {
        density_g_cm3: density,
        part_volume_mm3: part_volume,
        ..Default::default()
    };
    let mut vol = 0.0;
    for (i, l) in per_layer.iter().enumerate() {
        let wgt = weights.map(|w| w[i]).unwrap_or(1.0);
        s.time_s += l.time_s * wgt;
        s.cooling_s += (l.time_s - l.raw_time_s) * wgt;
        if l.slowed {
            s.slowed_layers += wgt.round() as usize;
        }
        for k in 0..6 {
            s.time_by_kind_s[k] += l.time_by_kind_s[k] * wgt;
            s.length_by_kind_mm[k] += l.length_mm[k] * wgt;
        }
        s.filament_mm += l.e_mm * wgt;
        vol += l.volume_mm3 * wgt;
        s.retracts += (l.retracts as f64 * wgt).round() as usize;
        s.max_z_mm = s.max_z_mm.max(l.z);
    }
    s.layers = weights.map(|w| w.iter().sum::<f64>().round() as usize).unwrap_or(per_layer.len());
    s.time_h = s.time_s / 3600.0;
    s.time_text = fmt_duration(s.time_s);
    s.filament_m = s.filament_mm / 1000.0;
    s.filament_cm3 = s.filament_mm * PI * filament_d * filament_d / 4.0 / 1000.0;
    s.filament_g = s.filament_cm3 * density;
    s.cost_usd = s.filament_g / 1000.0 * cost_per_kg;
    s.fill_ratio = if part_volume > 0.0 { vol / part_volume } else { 0.0 };
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bead_area_model() {
        // 0.45 x 0.2: (0.25)(0.2) + π(0.1)^2
        let a = bead_area(0.45, 0.2);
        assert!((a - (0.05 + PI * 0.01)).abs() < 1e-12);
        let e = e_per_mm(0.45, 0.2, 1.75);
        assert!((e - a / (PI * 1.75 * 1.75 / 4.0)).abs() < 1e-12);
    }

    #[test]
    fn trapezoid_limits() {
        // Infinite acceleration -> L/v.
        assert!((trapezoid_time(100.0, 50.0, 0.0, 1e12) - 2.0).abs() < 1e-6);
        // Long move: L/v + (v-vj)^2/(a v)
        let t = trapezoid_time(100.0, 50.0, 0.0, 1000.0);
        assert!((t - (2.0 + 2500.0 / 50000.0)).abs() < 1e-12);
        // Short move: triangle profile, t = 2 sqrt(L/a) from rest.
        let t = trapezoid_time(1.0, 100.0, 0.0, 1000.0);
        assert!((t - 2.0 * (1.0f64 / 1000.0).sqrt()).abs() < 1e-12);
    }

    #[test]
    fn corners_cost_time() {
        let square = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]];
        let t_sq = polyline_time(&square, 60.0, 5.0, 1000.0);
        let line = vec![[0.0, 0.0], [40.0, 0.0]];
        let t_line = polyline_time(&line, 60.0, 5.0, 1000.0);
        assert!(t_sq > t_line);
        assert!(t_line > 40.0 / 60.0);
    }
}
