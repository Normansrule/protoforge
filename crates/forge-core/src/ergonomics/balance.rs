//! Weight and balance: where the mass sits, how it loads the nose and the
//! ears, the nose-pad contact pressure and whether the glasses slip when the
//! wearer looks down.
//!
//! ## Mass model
//! Frame parts come from the CAD generator (`crate::cad`, read-only): the
//! divergence-theorem volume and centroid of every closed part mesh times the
//! frame material density. Components are placed by their `location`:
//! `right_pod` at the PCB centre, `left_pod` at the battery-cavity centre,
//! `front` at the two lens centres (hinge hardware at the hinges), and
//! `front_right` at the optics housing above the right lens.
//!
//! ## Support model
//! Glasses rest on four soft-tissue supports: two nose pads and the two ear
//! roots. Four supports are one more than rigid-body statics can resolve, so
//! the frame is treated as a rigid body on **four equally stiff springs** —
//! the classic bolt-group / pile-group distribution in which reactions vary
//! linearly with position:
//!
//! ```text
//! R_i = W/n + W (y_c − ȳ)(y_i − ȳ) / Σ(y_j − ȳ)² + W (x_c − x̄)(x_i − x̄) / Σ(x_j − x̄)²
//! ```
//!
//! For two nose pads at `y_n` and two ear supports at `y_e` this reduces to
//! the lever rule for the fore–aft split, `N_nose = W (y_c − y_e)/(y_n − y_e)`,
//! and the widely spaced ears take almost all of any left/right imbalance.
//! Head pitch `α` (looking down) scales the normal loads by `cos α` and adds a
//! force `W sin α` along the nose that friction must resist; the small moment
//! of that force about the supports (±6 mm of height vs a 110 mm lever) is
//! neglected.
//!
//! ## Nose pads
//! A pad on a nose flank inclined `β` from the vertical mid-plane must press
//! with `N = V / sin β` to carry a vertical load `V` (frictionless wedge — an
//! upper bound). Contact pressure `p = N / A_pad` is compared with the
//! capillary blood pressure of skin, ≈ 32 mmHg = 4.3 kPa (Landis 1930), the
//! usual limit for pressure that is sustained for hours.
//!
//! ## Slip ("too loose")
//! Looking down by `α`, the glasses stay put while
//! `μ (N_nose + V_ears + 2 F_clamp) ≥ W sin α`, giving the smallest clamp
//! force per side that still holds them. Skin friction varies widely with
//! moisture (Derler & Gerhardt 2012); μ ≈ 0.3 is a dry-skin, low-end value.

use serde::{Deserialize, Serialize};

use super::model::{ErgoConfig, FrameGeom};
use crate::project::Project;

pub const G: f64 = 9.806_65;
/// 1 mmHg in kPa.
pub const MMHG_KPA: f64 = 0.133_322;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MassItem {
    pub name: String,
    pub kind: String,
    pub location: String,
    pub mass_g: f64,
    pub pos: [f64; 3],
}

/// Frame parts + components with their positions (assembly frame, mm).
pub fn mass_items(p: &Project, fg: &FrameGeom) -> Vec<MassItem> {
    let geom = crate::cad::frame_geom(p);
    let dens = p.design.frame.density_g_cm3;
    let mut out: Vec<MassItem> = geom
        .parts
        .iter()
        .map(|b| {
            let m = b.assembly.mass_props(dens);
            MassItem {
                name: b.label.clone(),
                kind: "frame".into(),
                location: b.name.clone(),
                mass_g: m.mass_g,
                pos: m.centroid,
            }
        })
        .collect();
    let l = &geom.layout;
    let optics = geom
        .parts
        .iter()
        .find(|b| b.name == "optics_housing")
        .map(|b| b.assembly.mass_props(dens).centroid)
        .unwrap_or([l.lens_center_x, -8.0, fg.lens_height_mm / 2.0]);
    let batt = [
        -(l.temple_inner_x + fg.pod_w_mm / 2.0),
        (l.pod_front_y + fg.battery_pod_rear_y) / 2.0,
        l.hinge_z,
    ];
    for c in &p.components {
        let m = c.mass_g * c.qty;
        if m <= 0.0 {
            continue;
        }
        let name = format!("{} {}", c.refdes, c.name);
        let lname = c.name.to_lowercase();
        let mut push = |pos: [f64; 3], mass: f64, suffix: &str| {
            out.push(MassItem {
                name: format!("{name}{suffix}"),
                kind: "component".into(),
                location: c.location.clone(),
                mass_g: mass,
                pos,
            })
        };
        match c.location.as_str() {
            "right_pod" => push(l.pcb_center, m, ""),
            "left_pod" => push(batt, m, ""),
            "front_right" => push(optics, m, ""),
            "front" if lname.contains("hinge") => {
                push([l.pin_x, l.pin_y, l.hinge_z], m / 2.0, " (right)");
                push([-l.pin_x, l.pin_y, l.hinge_z], m / 2.0, " (left)");
            }
            "front" => {
                push([l.lens_center_x, -fg.front_thickness_mm / 2.0, 0.0], m / 2.0, " (right)");
                push([-l.lens_center_x, -fg.front_thickness_mm / 2.0, 0.0], m / 2.0, " (left)");
            }
            _ => push([0.0, -fg.front_thickness_mm / 2.0, 0.0], m, " (unplaced: at the bridge)"),
        }
    }
    out
}

/// Total mass and centre of mass.
pub fn centre_of_mass(items: &[MassItem]) -> (f64, [f64; 3]) {
    let m: f64 = items.iter().map(|i| i.mass_g).sum();
    if m <= 0.0 {
        return (0.0, [0.0; 3]);
    }
    let mut c = [0.0; 3];
    for i in items {
        for (ck, pk) in c.iter_mut().zip(i.pos) {
            *ck += i.mass_g * pk;
        }
    }
    (m, [c[0] / m, c[1] / m, c[2] / m])
}

/// Reactions of a rigid body on `n` equally stiff vertical springs at
/// (x_i, y_i) carrying `w` at (x_c, y_c): the linear (bolt-group) distribution.
pub fn spring_reactions(w: f64, com: [f64; 2], supports: &[[f64; 2]]) -> Vec<f64> {
    let n = supports.len() as f64;
    let xb = supports.iter().map(|s| s[0]).sum::<f64>() / n;
    let yb = supports.iter().map(|s| s[1]).sum::<f64>() / n;
    let sxx: f64 = supports.iter().map(|s| (s[0] - xb).powi(2)).sum();
    let syy: f64 = supports.iter().map(|s| (s[1] - yb).powi(2)).sum();
    let sxy: f64 = supports.iter().map(|s| (s[0] - xb) * (s[1] - yb)).sum();
    // General 2×2 inverse so skewed layouts also work.
    let det = sxx * syy - sxy * sxy;
    let (mx, my) = (w * (com[0] - xb), w * (com[1] - yb));
    let (a, b) = if det.abs() > 1e-12 {
        ((syy * mx - sxy * my) / det, (sxx * my - sxy * mx) / det)
    } else if syy > 1e-12 {
        (0.0, my / syy)
    } else {
        (0.0, 0.0)
    };
    supports.iter().map(|s| w / n + a * (s[0] - xb) + b * (s[1] - yb)).collect()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Loads {
    pub pitch_deg: f64,
    /// Vertical (head-frame) reactions, grams-force: [right pad, left pad, right ear, left ear].
    pub reactions_gf: [f64; 4],
    pub nose_gf: f64,
    pub ears_gf: f64,
    pub nose_share_pct: f64,
    pub right_ear_share_pct: f64,
    /// Normal force on each pad, N (V / sin β).
    pub pad_normal_n: [f64; 2],
    pub pad_pressure_kpa: [f64; 2],
    /// Force along the nose that friction must hold, N.
    pub slide_force_n: f64,
    /// Friction needed with zero clamp: W sin α / (N_nose + V_ears).
    pub mu_required_no_clamp: f64,
}

pub struct Supports {
    pub pads: [[f64; 3]; 2],
    pub ears: [[f64; 3]; 2],
}

pub fn supports(fg: &FrameGeom) -> Supports {
    let [px, py, pz] = fg.pad;
    let ex = fg.temple_inner_x + fg.temple_w_mm / 2.0;
    Supports {
        pads: [[px, py, pz], [-px, py, pz]],
        ears: [[ex, fg.ear_bend_y, fg.hinge_z], [-ex, fg.ear_bend_y, fg.hinge_z]],
    }
}

pub fn loads(mass_g: f64, com: [f64; 3], s: &Supports, cfg: &ErgoConfig, pad_area_mm2: f64, pitch_deg: f64) -> Loads {
    let a = pitch_deg.to_radians();
    let w_n = mass_g * 1e-3 * G;
    let pts: Vec<[f64; 2]> = [s.pads[0], s.pads[1], s.ears[0], s.ears[1]].iter().map(|p| [p[0], p[1]]).collect();
    let r = spring_reactions(w_n * a.cos(), [com[0], com[1]], &pts);
    let gf = |n: f64| n / G * 1e3;
    let beta = cfg.nose_flank_deg.to_radians();
    let pad_n = [r[0].max(0.0) / beta.sin(), r[1].max(0.0) / beta.sin()];
    let area = pad_area_mm2.max(1e-6);
    let nose = r[0] + r[1];
    let ears = r[2] + r[3];
    let slide = w_n * a.sin();
    let normal_sum = pad_n[0] + pad_n[1] + ears.max(0.0);
    Loads {
        pitch_deg,
        reactions_gf: [gf(r[0]), gf(r[1]), gf(r[2]), gf(r[3])],
        nose_gf: gf(nose),
        ears_gf: gf(ears),
        nose_share_pct: if nose + ears != 0.0 { 100.0 * nose / (nose + ears) } else { 0.0 },
        right_ear_share_pct: if ears.abs() > 1e-12 { 100.0 * r[2] / ears } else { 50.0 },
        pad_normal_n: pad_n,
        pad_pressure_kpa: [pad_n[0] / area * 1e3, pad_n[1] / area * 1e3],
        slide_force_n: slide,
        mu_required_no_clamp: if normal_sum > 0.0 { slide / normal_sum } else { f64::INFINITY },
    }
}

/// Smallest clamp force per side that stops the glasses sliding at the
/// look-down angle: μ (ΣN + 2F) ≥ W sin α.
pub fn min_clamp_n(l: &Loads, cfg: &ErgoConfig) -> f64 {
    let normal = l.pad_normal_n[0] + l.pad_normal_n[1] + (l.ears_gf * G * 1e-3).max(0.0);
    ((l.slide_force_n / cfg.friction_mu - normal) / 2.0).max(0.0)
}

/// Largest clamp force per side that keeps the temple contact pressure
/// below the sustained-pressure limit: p_lim × (contact length × temple height).
pub fn max_clamp_n(fg: &FrameGeom, cfg: &ErgoConfig) -> f64 {
    cfg.pressure_limit_kpa * 1e-3 * cfg.temple_contact_length_mm * fg.temple_h_mm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lever_rule_for_symmetric_supports() {
        // 40 g on pads at y = 0 and ears at y = −100, CoM at y = −25, centred:
        // nose carries W·(y_c − y_e)/(y_n − y_e) = 40·75/100 = 30 gf.
        let w = 0.040 * G;
        let r = spring_reactions(w, [0.0, -25.0], &[[7.0, 0.0], [-7.0, 0.0], [70.0, -100.0], [-70.0, -100.0]]);
        let nose_gf = (r[0] + r[1]) / G * 1e3;
        assert!((nose_gf - 30.0).abs() < 1e-9, "{nose_gf}");
        assert!((r[0] - r[1]).abs() < 1e-12);
        assert!(((r.iter().sum::<f64>()) - w).abs() < 1e-12);
    }

    #[test]
    fn lateral_offset_goes_mostly_to_the_ears() {
        // Moment balance: Σ R_i x_i = W x_c, and the far-apart ears take the most.
        let w = 1.0;
        let s = [[7.0, 0.0], [-7.0, 0.0], [70.0, -100.0], [-70.0, -100.0]];
        let r = spring_reactions(w, [5.0, -50.0], &s);
        let m: f64 = r.iter().zip(s.iter()).map(|(r, s)| r * s[0]).sum();
        assert!((m - 5.0).abs() < 1e-12);
        let dear = r[2] - r[3];
        let dpad = r[0] - r[1];
        // bolt-group: ΔR ∝ x_i → ear difference is 10× the pad difference
        assert!((dear / dpad - 10.0).abs() < 1e-9);
    }

    #[test]
    fn pad_pressure_and_slip_by_hand() {
        let cfg = ErgoConfig::default();
        let s = Supports {
            pads: [[7.0, 0.0, 0.0], [-7.0, 0.0, 0.0]],
            ears: [[70.0, -100.0, 0.0], [-70.0, -100.0, 0.0]],
        };
        // 40 g, CoM 25 mm behind the pads → 30 gf on the nose, 15 gf per pad.
        let l = loads(40.0, [0.0, -25.0, 0.0], &s, &cfg, 20.0, 0.0);
        let v = 0.015 * G; // N per pad
        let n = v / (30f64.to_radians().sin()); // = 2V
        assert!((l.pad_normal_n[0] - n).abs() < 1e-9);
        assert!((l.pad_pressure_kpa[0] - n / 20.0 * 1e3).abs() < 1e-9); // N/mm² → kPa ×1e3
        assert!((l.nose_gf - 30.0).abs() < 1e-9);
        // look down 40°: slide = W sin 40°
        let l2 = loads(40.0, [0.0, -25.0, 0.0], &s, &cfg, 20.0, 40.0);
        assert!((l2.slide_force_n - 0.040 * G * 40f64.to_radians().sin()).abs() < 1e-12);
        let fmin = min_clamp_n(&l2, &cfg);
        let normal = l2.pad_normal_n[0] + l2.pad_normal_n[1] + l2.ears_gf * G * 1e-3;
        assert!((cfg.friction_mu * (normal + 2.0 * fmin) - l2.slide_force_n).abs() < 1e-9 || fmin == 0.0);
        // 32 mmHg ≈ 4.27 kPa
        assert!((32.0 * MMHG_KPA - 4.266).abs() < 0.01);
    }
}
