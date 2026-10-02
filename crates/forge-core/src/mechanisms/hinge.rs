//! The Iris spring hinge (component HW1): a compression-spring barrel
//! ("flex") hinge whose temple end face is the detent cam.
//!
//! **Construction (top view).** The front's knuckles and the temple's
//! slider eye share the hinge screw (pin). The slider runs in a barrel
//! inside the temple; a preloaded compression spring pulls the temple
//! towards the pin, pressing the temple's end face against the endpiece's
//! stop face, which lies a distance `a` in front of the pin. The end face
//! has rounded corners (radius `r_in` on the inner, `r_out` on the outer
//! edge) at `t_in`, `t_out` from the pin.
//!
//! **Kinematics.** With φ the opening angle (0° folded, 90° worn, > 90°
//! over-travel), a corner-circle centre sits at
//! `y = (a − r) sin φ ∓ (t − r) cos φ` in front of the pin. Where the corner
//! would cross the stop plane the temple must slide away from the pin by
//! `δ(φ) = max(0, y + r − a) / sin φ` — the slider travel. Closing from 90°,
//! the inner corner climbs to a maximum δ (the detent hump) and then drops
//! away (the temple snaps shut); opening from folded, the same hump must be
//! pushed over before the spring snaps the temple open. Past 90° the outer
//! corner lifts the temple: over-travel.
//!
//! **Statics (virtual work, frictionless).** Spring energy
//! `E(φ) = ½ k [(δ₀ + δ)² − δ₀²]`, torque `T = dE/dφ = k (δ₀ + δ) dδ/dφ`.
//! Pin friction adds `μ F r_pin` against the motion. The metric
//! `mechanisms.hinge_open_torque_nmm` is the peak torque needed to push the
//! folded temple over the hump.
//!
//! **On the head.** The temple contacts the head `L_c` behind the pin (the
//! Human Factors Lab's contact point, just in front of the ear bend).
//! Spreading the contact outward by Δ is shared by the hinge (over-travel α)
//! and the frame's elastic bending — the stepped temple cantilever plus half
//! of the front's bowing, `1/k = c_total/2` from [`crate::ergonomics::clamp`],
//! so both labs give the same clamp force: `L_c sin α + F/k = Δ` with
//! `F L_c = T(90° + α)`.
//!
//! **Fatigue.** Each use folds and unfolds the temple over the hump twice,
//! spreads it on the head once, and some uses pull it to the over-travel
//! stop. Every cycle runs the spring from its preload to the peak travel;
//! Goodman on Zimmerli's data gives the safety factor, the Table 10-8
//! allowables and Miner's rule the life (see [`super::springs`]).

use serde::{Deserialize, Serialize};

use super::springs::{self, Check};
use crate::project::Project;

const DEG: f64 = std::f64::consts::PI / 180.0;

/// Design choices for the hinge, stored in `design.extra.mechanisms.hinge`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct HingeParams {
    pub material: String,
    pub wire_d_mm: f64,
    pub mean_d_mm: f64,
    pub active_coils: f64,
    pub ends: String,
    pub free_len_mm: f64,
    pub preload_mm: f64,
    pub peened: bool,
    /// Pin to stop face along the temple, mm (default: from the CAD knuckles).
    pub stop_offset_mm: Option<f64>,
    /// Pin to the temple's inner / outer face, mm (default: half the temple width).
    pub inner_mm: Option<f64>,
    pub outer_mm: Option<f64>,
    /// Corner radii of the temple end face, mm.
    pub inner_corner_r_mm: f64,
    pub outer_corner_r_mm: f64,
    /// Over-travel stop, degrees past 90°.
    pub max_overtravel_deg: f64,
    /// Head width at the ear contact, mm.
    pub head_width_mm: f64,
    pub friction: f64,
    pub pin_d_mm: f64,
    /// Usage: open/wear/close uses per day, full over-travel pulls per day, design life.
    pub uses_per_day: f64,
    pub wide_pulls_per_day: f64,
    pub design_years: f64,
}

impl Default for HingeParams {
    fn default() -> Self {
        HingeParams {
            material: "stainless_302".into(),
            wire_d_mm: 0.45,
            mean_d_mm: 2.0,
            active_coils: 6.0,
            ends: "squared_ground".into(),
            free_len_mm: 5.5,
            preload_mm: 0.6,
            peened: false,
            stop_offset_mm: None,
            inner_mm: None,
            outer_mm: None,
            inner_corner_r_mm: 1.1,
            outer_corner_r_mm: 0.3,
            max_overtravel_deg: 20.0,
            head_width_mm: 146.0,
            friction: 0.15,
            pin_d_mm: 1.4,
            uses_per_day: 12.0,
            wide_pulls_per_day: 2.0,
            design_years: 3.0,
        }
    }
}

impl HingeParams {
    /// Read from `design.extra.mechanisms.hinge`, falling back to defaults.
    pub fn from_project(p: &Project) -> HingeParams {
        p.design
            .extra
            .get("mechanisms")
            .and_then(|m| m.get("hinge"))
            .and_then(|h| serde_json::from_value(h.clone()).ok())
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct HingeGeom {
    pub stop_offset_mm: f64,
    pub inner_mm: f64,
    pub outer_mm: f64,
    pub temple_width_mm: f64,
    pub temple_height_mm: f64,
    pub temple_length_mm: f64,
    /// Pin X (wearer's right) and the temple inner face at 90°, mm.
    pub pin_x_mm: f64,
    pub temple_inner_x_mm: f64,
    /// Pin to ear contact and pin to the pod's rear end along the temple, mm.
    pub contact_len_mm: f64,
    pub pod_len_mm: f64,
    pub arm_len_mm: f64,
    /// Lateral stiffness of the temple alone at the ear contact (two-segment beam, rigid hinge and front), N/mm.
    pub temple_stiffness_n_mm: f64,
    /// Per-side stiffness of the whole frame at the contact with a rigid hinge (temple + front bowing,
    /// the Human Factors clamp model), N/mm — what the hinge works against on the head.
    pub arm_stiffness_n_mm: f64,
    /// Half the unloaded spread between the contacts (Human Factors model, includes temple splay), mm.
    pub rest_half_spread_mm: f64,
    pub frame_material: String,
}

/// Iris-derived geometry for the hinge (CAD layout + frame parameters).
pub fn geometry(p: &Project, h: &HingeParams) -> HingeGeom {
    let f = &p.design.frame;
    let g = crate::cad::frame_geom(p);
    let l = &g.layout;
    let a = h
        .stop_offset_mm
        .unwrap_or((crate::cad::frame::KNUCKLE_LEN + crate::cad::frame::KNUCKLE_GAP) / 2.0);
    let t_in = h.inner_mm.unwrap_or(f.temple_width_mm / 2.0);
    let t_out = h.outer_mm.unwrap_or(f.temple_width_mm / 2.0);
    let contact = (l.ear_bend_y - l.pin_y).abs().max(10.0);
    let pod = (l.pod_rear_y - l.pin_y).abs().min(contact - 5.0).max(0.0);
    let arm = (contact - pod).max(5.0);
    let mat = crate::structures::materials::get(&f.material);
    let e = mat.e_mpa();
    // Arm: solid rectangle bending sideways. Pod: a channel open to the
    // outside (inner wall = web, top and bottom walls = flanges), lid ignored.
    let i_arm = f.temple_height_mm * f.temple_width_mm.powi(3) / 12.0;
    let (wl, bw, hh) = (f.wall_mm.max(0.3), f.pod_width_mm.max(1.0), f.pod_height_mm.max(1.0));
    let fl = (bw - wl).max(0.0);
    let (a_web, x_web) = (wl * hh, wl / 2.0);
    let (a_fl, x_fl) = (2.0 * wl * fl, wl + fl / 2.0);
    let xbar = (a_web * x_web + a_fl * x_fl) / (a_web + a_fl);
    let i_pod = hh * wl.powi(3) / 12.0 + a_web * (x_web - xbar).powi(2) + 2.0 * wl * fl.powi(3) / 12.0 + a_fl * (x_fl - xbar).powi(2);
    // two-segment cantilever from the hinge, load at the ear contact
    let compliance = (contact.powi(3) - arm.powi(3)) / (3.0 * e * i_pod) + arm.powi(3) / (3.0 * e * i_arm);
    let k_temple = 1.0 / compliance.max(1e-12);
    // On the head: the Human Factors Lab's frame model (contact point, temple + front bowing).
    let ecfg = crate::ergonomics::model::ErgoConfig::from_project(p);
    let cm = crate::ergonomics::clamp::ClampModel::new(
        &crate::ergonomics::model::frame_geom(p),
        ecfg.temple_splay_deg,
        ecfg.contact_before_bend_mm,
    );
    let k_frame = 2.0 / cm.c_total.max(1e-12);
    HingeGeom {
        stop_offset_mm: a,
        inner_mm: t_in,
        outer_mm: t_out,
        temple_width_mm: f.temple_width_mm,
        temple_height_mm: f.temple_height_mm,
        temple_length_mm: f.temple_length_mm,
        pin_x_mm: l.pin_x,
        temple_inner_x_mm: l.pin_x - t_in,
        contact_len_mm: cm.l_c,
        pod_len_mm: pod,
        arm_len_mm: arm,
        temple_stiffness_n_mm: k_temple,
        arm_stiffness_n_mm: k_frame,
        rest_half_spread_mm: cm.s0 / 2.0,
        frame_material: mat.name.into(),
    }
}

/// Slider travel δ (mm) at opening angle φ (radians) for the end-face cam.
pub fn slider_travel(g: &HingeGeom, h: &HingeParams, phi: f64) -> f64 {
    let a = g.stop_offset_mm;
    let s = phi.sin().max(1e-6);
    let c = phi.cos();
    let ri = h.inner_corner_r_mm.clamp(0.0, g.inner_mm.min(a));
    let ro = h.outer_corner_r_mm.clamp(0.0, g.outer_mm.min(a));
    let pen_in = (a - ri) * phi.sin() + (g.inner_mm - ri) * c + ri - a;
    let pen_out = (a - ro) * phi.sin() - (g.outer_mm - ro) * c + ro - a;
    pen_in.max(pen_out).max(0.0) / s
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SpringSummary {
    pub material: String,
    pub sut_mpa: f64,
    pub ssy_mpa: f64,
    pub ssu_mpa: f64,
    pub sse_mpa: f64,
    pub g_mpa: f64,
    pub index_c: f64,
    pub k_b: f64,
    pub rate_n_mm: f64,
    pub preload_n: f64,
    pub installed_len_mm: f64,
    pub solid_len_mm: f64,
    /// Travel from installed length to solid, mm.
    pub travel_to_solid_mm: f64,
    pub tau_per_n: f64,
    pub od_mm: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct HingeBlock {
    pub name: String,
    pub per_day: f64,
    pub travel_mm: f64,
    pub f_min_n: f64,
    pub f_max_n: f64,
    pub tau_min_mpa: f64,
    pub tau_max_mpa: f64,
    pub tau_a_mpa: f64,
    pub tau_m_mpa: f64,
    pub nf: f64,
    pub cycles_to_failure: f64,
    pub runout: bool,
    pub damage_per_day: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Wear {
    /// Outward spread needed at the ear contact, per side, mm.
    pub spread_mm: f64,
    pub overtravel_deg: f64,
    pub arm_bend_mm: f64,
    pub clamp_n: f64,
    pub clamp_rigid_n: f64,
    pub hinge_share_pct: f64,
    pub hits_stop: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct HingeOut {
    pub params: HingeParams,
    pub geom: HingeGeom,
    pub spring: SpringSummary,
    pub phi_deg: Vec<f64>,
    pub travel_mm: Vec<f64>,
    pub force_n: Vec<f64>,
    /// Frictionless spring torque dE/dφ (positive resists opening), N·mm.
    pub torque_nmm: Vec<f64>,
    /// Torque the user applies to open / to close (with pin friction), N·mm.
    pub open_torque_nmm: Vec<f64>,
    pub close_torque_nmm: Vec<f64>,
    pub energy_nmm: Vec<f64>,
    pub contact_start_deg: f64,
    pub hump_deg: f64,
    pub hump_travel_mm: f64,
    /// Peak torque to push the folded temple over the detent (metric).
    pub detent_open_torque_nmm: f64,
    /// Peak torque to fold the open temple back over the detent.
    pub detent_close_torque_nmm: f64,
    /// Torque at the start of over-travel and the over-travel rate.
    pub overtravel_start_torque_nmm: f64,
    pub overtravel_rate_nmm_deg: f64,
    pub max_travel_mm: f64,
    pub max_tau_mpa: f64,
    pub ns_static: f64,
    pub wear: Wear,
    pub blocks: Vec<HingeBlock>,
    pub nf_min: f64,
    pub damage_per_day: f64,
    /// Open/close uses to failure (capped at 1e7 when every block is a runout).
    pub life_uses: f64,
    pub life_years: f64,
    pub runout: bool,
    pub design_uses: f64,
    pub checks: Vec<Check>,
    pub notes: Vec<String>,
}

fn interp_clamped(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    crate::math::interp(xs, ys, x)
}

/// Maximum open/close uses reported (runout).
pub const LIFE_CAP: f64 = 1e7;

pub fn analyze(p: &Project, h: &HingeParams) -> Result<HingeOut, String> {
    let g = geometry(p, h);
    if h.wire_d_mm.is_nan()
        || h.mean_d_mm.is_nan()
        || h.wire_d_mm <= 0.0
        || h.mean_d_mm <= h.wire_d_mm
        || h.active_coils <= 0.0
        || h.free_len_mm <= 0.0
    {
        return Err("spring needs 0 < d < D, positive coils and free length".into());
    }
    let w = springs::wire(&h.material);
    let (_, gm) = springs::moduli(w, h.wire_d_mm);
    let d = h.wire_d_mm;
    let dm = h.mean_d_mm;
    let k = d.powi(4) * gm / (8.0 * dm.powi(3) * h.active_coils);
    let sut = springs::sut(w, d);
    let ssy = w.ssy_frac * sut;
    let ssu = 0.67 * sut;
    let sse = springs::goodman_sse(h.peened, ssu);
    let c = dm / d;
    let kb = springs::bergstrasser(c);
    let tau_per_n = kb * 8.0 * dm / (std::f64::consts::PI * d.powi(3));
    let (_, ls, _) = springs::end_geometry(&h.ends, h.active_coils, d, h.free_len_mm);
    let d0 = h.preload_mm.max(0.0);
    let installed = h.free_len_mm - d0;
    let travel_solid = installed - ls;
    let spring = SpringSummary {
        material: format!("{} ({})", w.name, w.spec),
        sut_mpa: sut,
        ssy_mpa: ssy,
        ssu_mpa: ssu,
        sse_mpa: sse,
        g_mpa: gm,
        index_c: c,
        k_b: kb,
        rate_n_mm: k,
        preload_n: k * d0,
        installed_len_mm: installed,
        solid_len_mm: ls,
        travel_to_solid_mm: travel_solid,
        tau_per_n,
        od_mm: dm + d,
    };
    // sweep the opening angle
    let top = 90.0 + h.max_overtravel_deg.clamp(0.0, 60.0);
    let step = 0.25;
    let n = (top / step).round() as usize;
    let mut phi = Vec::with_capacity(n + 1);
    let mut dl = Vec::with_capacity(n + 1);
    for q in 0..=n {
        let f = (q as f64 * step).max(0.05);
        phi.push(f);
        dl.push(slider_travel(&g, h, f * DEG));
    }
    let force: Vec<f64> = dl.iter().map(|x| k * (d0 + x)).collect();
    let energy: Vec<f64> = dl.iter().map(|x| 0.5 * k * ((d0 + x).powi(2) - d0 * d0)).collect();
    let mut torque = vec![0.0; phi.len()];
    for q in 0..phi.len() {
        let (a, b) = (q.saturating_sub(1), (q + 1).min(phi.len() - 1));
        let dd = (dl[b] - dl[a]) / ((phi[b] - phi[a]) * DEG);
        torque[q] = force[q] * dd;
    }
    let fric: Vec<f64> = force.iter().map(|f| h.friction * f * h.pin_d_mm / 2.0).collect();
    let open_t: Vec<f64> = torque.iter().zip(&fric).map(|(t, f)| t + f).collect();
    let close_t: Vec<f64> = torque.iter().zip(&fric).map(|(t, f)| -t + f).collect();
    // hump below 90°
    let i90 = phi.iter().position(|x| *x >= 90.0).unwrap_or(phi.len() - 1);
    let (ih, hump) = dl[..i90]
        .iter()
        .enumerate()
        .fold((0, 0.0f64), |b, (q, v)| if *v > b.1 { (q, *v) } else { b });
    let contact_start = dl[..i90].iter().position(|v| *v > 1e-9).map(|q| phi[q]).unwrap_or(90.0);
    let detent_open = open_t[..=ih].iter().cloned().fold(0.0f64, f64::max);
    let detent_close = close_t[ih..=i90].iter().cloned().fold(0.0f64, f64::max);
    let i_ot = (i90 + 1).min(phi.len() - 1);
    let t0 = torque[i_ot];
    let ot_rate = if phi.len() > i90 + 9 {
        (torque[phi.len() - 1] - torque[i_ot]) / (phi[phi.len() - 1] - phi[i_ot]).max(1e-9)
    } else {
        0.0
    };
    let max_travel = dl.iter().cloned().fold(0.0f64, f64::max);
    let max_tau = tau_per_n * k * (d0 + max_travel);
    // ---- on the head
    let spread = h.head_width_mm / 2.0 - g.rest_half_spread_mm;
    let lc = g.contact_len_mm;
    let ka = g.arm_stiffness_n_mm.max(1e-9);
    let ot_phi: Vec<f64> = phi[i90..].iter().map(|x| x - 90.0).collect();
    let ot_t: Vec<f64> = torque[i90..].to_vec();
    let t_at = |al: f64| interp_clamped(&ot_phi, &ot_t, al).max(0.0);
    let resid = |al: f64| lc * (al * DEG).sin() + t_at(al) / lc / ka - spread;
    let mut wear = Wear {
        spread_mm: spread,
        clamp_rigid_n: (spread * ka).max(0.0),
        ..Default::default()
    };
    let amax = h.max_overtravel_deg.clamp(0.0, 60.0);
    if spread > 0.0 {
        let t_start = interp_clamped(&ot_phi, &ot_t, 0.05);
        if t_start / lc / ka >= spread || amax <= 0.0 {
            wear.overtravel_deg = 0.0;
            wear.clamp_n = spread * ka;
            wear.arm_bend_mm = spread;
        } else if resid(amax) < 0.0 {
            wear.hits_stop = true;
            wear.overtravel_deg = amax;
            wear.arm_bend_mm = spread - lc * (amax * DEG).sin();
            wear.clamp_n = wear.arm_bend_mm * ka;
        } else {
            let (mut lo, mut hi) = (0.0, amax);
            for _ in 0..80 {
                let mid = 0.5 * (lo + hi);
                if resid(mid) > 0.0 {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            let al = 0.5 * (lo + hi);
            wear.overtravel_deg = al;
            wear.clamp_n = t_at(al) / lc;
            wear.arm_bend_mm = wear.clamp_n / ka;
        }
        wear.hinge_share_pct = 100.0 * (1.0 - wear.arm_bend_mm / spread).clamp(0.0, 1.0);
    }
    let wear_travel = slider_travel(&g, h, (90.0 + wear.overtravel_deg) * DEG);
    let full_travel = slider_travel(&g, h, top * DEG);
    // ---- fatigue blocks
    let mk = |name: &str, per_day: f64, travel: f64| -> HingeBlock {
        let fmin = k * d0;
        let fmax = k * (d0 + travel);
        let (tmin, tmax) = (tau_per_n * fmin, tau_per_n * fmax);
        let ta = 0.5 * (tmax - tmin);
        let tm = 0.5 * (tmax + tmin);
        let nf = springs::goodman_nf(ta, tm, sse, ssu);
        let lf = springs::life(ta, tm, sut, h.peened);
        HingeBlock {
            name: name.into(),
            per_day,
            travel_mm: travel,
            f_min_n: fmin,
            f_max_n: fmax,
            tau_min_mpa: tmin,
            tau_max_mpa: tmax,
            tau_a_mpa: ta,
            tau_m_mpa: tm,
            nf: nf.min(1e6),
            cycles_to_failure: lf.cycles,
            runout: lf.runout,
            damage_per_day: if lf.runout || per_day <= 0.0 { 0.0 } else { per_day / lf.cycles },
        }
    };
    let blocks = vec![
        mk("Fold and unfold over the detent (2 per use)", 2.0 * h.uses_per_day, hump),
        mk(
            &format!("Worn: temples spread by a {:.0} mm head", h.head_width_mm),
            h.uses_per_day,
            wear_travel,
        ),
        mk("Pulled wide to the over-travel stop", h.wide_pulls_per_day, full_travel),
    ];
    let active: Vec<&HingeBlock> = blocks.iter().filter(|b| b.per_day > 0.0 && b.travel_mm > 1e-9).collect();
    let nf_min = active.iter().map(|b| b.nf).fold(f64::INFINITY, f64::min).min(1e6);
    let dmg: f64 = blocks.iter().map(|b| b.damage_per_day).sum();
    let runout = dmg <= 0.0;
    let uses = h.uses_per_day.max(1e-9);
    let life_uses = if runout { LIFE_CAP } else { (uses / dmg).min(LIFE_CAP) };
    let design_uses = h.uses_per_day * 365.0 * h.design_years;
    let ns = ssy / max_tau.max(1e-9);
    let mut checks = vec![
        springs_check(
            "Goodman fatigue factor ≥ 1.5 (Zimmerli)",
            nf_min >= 1.5,
            format!("n_f = {nf_min:.2} (worst block)"),
        ),
        springs_check(
            "Life ≥ design uses",
            life_uses >= design_uses,
            format!(
                "{} uses vs {:.0} ({} per day × {} years)",
                if runout {
                    "≥ 10⁷ (runout)".to_string()
                } else {
                    format!("{life_uses:.3e}")
                },
                design_uses,
                h.uses_per_day,
                h.design_years
            ),
        ),
        springs_check(
            "Static n_s ≥ 1.2 at the stop",
            ns >= 1.2,
            format!("τ_max = {max_tau:.0} MPa vs S_sy = {ssy:.0} MPa"),
        ),
        springs_check(
            "Spring never goes solid",
            max_travel < travel_solid,
            format!("peak travel {max_travel:.2} mm vs {travel_solid:.2} mm to solid"),
        ),
        springs_check("Spring index 4 ≤ C ≤ 12", (4.0..=12.0).contains(&c), format!("C = {c:.2}")),
        springs_check(
            "Temple folds flat",
            dl[0] < 1e-9,
            if dl[0] < 1e-9 {
                "the end face clears the stop when folded".into()
            } else {
                "the inner half-width exceeds the stop offset — the temple cannot fold".into()
            },
        ),
        springs_check(
            "Spring fits the temple",
            dm + d + 0.8 <= g.temple_width_mm,
            format!("OD {:.2} mm + 2 × 0.4 mm walls vs {:.1} mm temple", dm + d, g.temple_width_mm),
        ),
    ];
    if d < w.d_min_mm {
        checks.push(springs_check(
            "Wire inside the published size range",
            false,
            format!("{} strength data start at {:.2} mm", w.name, w.d_min_mm),
        ));
    }
    let mut notes = vec![
        format!(
            "Usage assumed: {} uses per day (open, wear, close — two passes over the detent each) and {} pulls to the {}° over-travel stop per day, for {} years = {:.0} uses.",
            h.uses_per_day, h.wide_pulls_per_day, h.max_overtravel_deg, h.design_years, design_uses
        ),
        format!(
            "Pin friction μ = {} on a Ø{} mm screw is included in the user torques (typical steel-on-steel value; lubrication and plating change it).",
            h.friction, h.pin_d_mm
        ),
    ];
    if spread <= 0.0 {
        notes.push("The head is narrower than the frame: no spread, so the hinge adds no clamp force.".into());
    } else {
        notes.push(format!(
            "On a {:.0} mm head the hinge takes {:.0} % of the {:.1} mm spread per side and clamps with {:.2} N per side ({:.2} N if the hinge were rigid) — the frame model (temple plus front bowing, contact point) is the Human Factors Lab's, so the two labs agree.",
            h.head_width_mm, wear.hinge_share_pct, spread, wear.clamp_n, wear.clamp_rigid_n
        ));
        notes.push(
            "The temple alone is a two-segment beam here (open-channel pod + solid arm, lid ignored); the front's bowing makes the whole frame several times softer. The Structures Lab finite element model (structures.temple_stiffness_n_mm, load at the tip) includes the knuckles and pod walls and reads softer than this beam."
                .into(),
        );
    }
    Ok(HingeOut {
        params: h.clone(),
        geom: g,
        spring,
        phi_deg: phi,
        travel_mm: dl,
        force_n: force,
        torque_nmm: torque,
        open_torque_nmm: open_t,
        close_torque_nmm: close_t,
        energy_nmm: energy,
        contact_start_deg: contact_start,
        hump_deg: if hump > 0.0 { (ih as f64 * step).max(0.05) } else { 0.0 },
        hump_travel_mm: hump,
        detent_open_torque_nmm: detent_open,
        detent_close_torque_nmm: detent_close,
        overtravel_start_torque_nmm: t0,
        overtravel_rate_nmm_deg: ot_rate,
        max_travel_mm: max_travel,
        max_tau_mpa: max_tau,
        ns_static: ns,
        wear,
        blocks,
        nf_min,
        damage_per_day: dmg,
        life_uses,
        life_years: if runout { 1e9 } else { (1.0 / dmg / 365.0).min(1e9) },
        runout,
        design_uses,
        checks,
        notes,
    })
}

fn springs_check(name: &str, ok: bool, detail: String) -> Check {
    Check {
        name: name.into(),
        ok,
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sharp(a: f64, t: f64) -> (HingeGeom, HingeParams) {
        let g = HingeGeom {
            stop_offset_mm: a,
            inner_mm: t,
            outer_mm: t,
            ..Default::default()
        };
        let h = HingeParams {
            inner_corner_r_mm: 0.0,
            outer_corner_r_mm: 0.0,
            ..Default::default()
        };
        (g, h)
    }

    /// Sharp corners: δ(φ) = (t/2… ) closed forms — inner corner
    /// δ = a(1 − 1/sin φ) + t cot φ below 90°, outer corner
    /// δ = a(1 − 1/sin φ) − t cot φ above 90°, hump at cos φ* = t/a.
    #[test]
    fn slider_travel_closed_form() {
        let (g, h) = sharp(2.7, 2.0);
        for deg in [50.0, 70.0, 89.0, 100.0, 115.0] {
            let f: f64 = deg * DEG;
            let exact = if deg < 90.0 {
                2.7 * (1.0 - 1.0 / f.sin()) + 2.0 / f.tan()
            } else {
                2.7 * (1.0 - 1.0 / f.sin()) - 2.0 / f.tan()
            };
            assert!((slider_travel(&g, &h, f) - exact.max(0.0)).abs() < 1e-12, "{deg}");
        }
        assert!(slider_travel(&g, &h, 90.0 * DEG).abs() < 1e-12);
        let star = (2.0f64 / 2.7).acos();
        let peak = slider_travel(&g, &h, star);
        for dd in [-0.02, 0.02] {
            assert!(slider_travel(&g, &h, star + dd) < peak);
        }
    }

    #[test]
    fn torque_is_energy_derivative_and_iris_numbers() {
        let p = Project::iris();
        let h = HingeParams::default();
        let o = analyze(&p, &h).unwrap();
        // T = dE/dφ numerically
        for q in [200usize, 250, 380] {
            let de = (o.energy_nmm[q + 1] - o.energy_nmm[q - 1]) / ((o.phi_deg[q + 1] - o.phi_deg[q - 1]) * DEG);
            assert!(
                (de - o.torque_nmm[q]).abs() < 0.02 * (1.0 + de.abs()),
                "{q}: {de} vs {}",
                o.torque_nmm[q]
            );
        }
        assert!(o.hump_deg > 20.0 && o.hump_deg < 90.0);
        assert!(o.detent_open_torque_nmm > 0.0 && o.detent_close_torque_nmm > 0.0);
        assert!(o.nf_min.is_finite() && o.nf_min > 0.0);
        assert!(o.life_uses > 0.0);
        // Hand check of the spring rate: k = d⁴G/(8D³Na) with G = 69 GPa.
        let k = 0.45f64.powi(4) * 69_000.0 / (8.0 * 8.0 * 6.0);
        assert!((o.spring.rate_n_mm - k).abs() < 1e-9);
        // over-travel starts with T0 = F0 (t_out − r_out) (outer corner centre lever)
        let t0 = k * 0.6 * (o.geom.outer_mm - 0.3);
        assert!(
            (o.overtravel_start_torque_nmm - t0).abs() / t0 < 0.05,
            "{} vs {t0}",
            o.overtravel_start_torque_nmm
        );
    }
}
