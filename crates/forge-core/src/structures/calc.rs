//! Mechanical design calculators — closed-form formulas every product
//! engineer reaches for, each tied to an Iris use.
//!
//! References: R. G. Budynas, J. K. Nisbett, *Shigley's Mechanical Engineering
//! Design* (10th ed.) — beams, bolted joints (T = K F d, Wileman member
//! stiffness), press fits (Lamé thick cylinders), columns (Euler/Johnson);
//! W. C. Young, R. G. Budynas, *Roark's Formulas for Stress and Strain*
//! (7th ed.) — beam and plate buckling cases; Bayer MaterialScience / Covestro,
//! *Snap-Fit Joints for Plastics — A Design Guide* — cantilever snap fits;
//! R. A. Malloy, *Plastic Part Design for Injection Molding* (Hanser) —
//! bosses and thread-forming screws. Figures called "typical" are rules of
//! thumb: check the screw, insert or resin maker's data.

use serde::{Deserialize, Serialize};

use super::materials;

// ---------------------------------------------------------------- sections

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Section {
    /// "rect", "round" or "tube".
    pub shape: String,
    /// rect: width b (mm, along the neutral axis).
    pub b: f64,
    /// rect: height h (mm, in the bending direction); round/tube: outer diameter.
    pub h: f64,
    /// tube wall thickness, mm.
    pub t: f64,
}

impl Default for Section {
    fn default() -> Self {
        Section {
            shape: "rect".into(),
            b: 8.0,
            h: 4.0,
            t: 1.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SectionProps {
    pub area_mm2: f64,
    /// Second moment of area about the bending axis, mm⁴.
    pub i_mm4: f64,
    /// Distance from neutral axis to the outer fibre, mm.
    pub c_mm: f64,
    /// Section modulus Z = I / c, mm³.
    pub z_mm3: f64,
    /// Radius of gyration √(I/A), mm.
    pub r_gyr_mm: f64,
}

pub fn section_props(s: &Section) -> Result<SectionProps, String> {
    let pi = std::f64::consts::PI;
    let (a, i, c) = match s.shape.as_str() {
        "round" => {
            let d = s.h;
            (pi * d * d / 4.0, pi * d.powi(4) / 64.0, d / 2.0)
        }
        "tube" => {
            let (d, t) = (s.h, s.t.min(s.h / 2.0));
            let di = d - 2.0 * t;
            (pi * (d * d - di * di) / 4.0, pi * (d.powi(4) - di.powi(4)) / 64.0, d / 2.0)
        }
        _ => (s.b * s.h, s.b * s.h.powi(3) / 12.0, s.h / 2.0),
    };
    if !(a > 0.0 && i > 0.0) {
        return Err("section dimensions must be positive".into());
    }
    Ok(SectionProps {
        area_mm2: a,
        i_mm4: i,
        c_mm: c,
        z_mm3: i / c,
        r_gyr_mm: (i / a).sqrt(),
    })
}

fn modulus(e_mpa: f64, material: &Option<String>) -> (f64, f64, f64, String) {
    // Returns (E, yield, nu, name).
    match material {
        Some(m) if e_mpa <= 0.0 => {
            let mm = materials::get(m);
            (mm.e_mpa(), mm.yield_mpa, mm.nu, mm.name.into())
        }
        Some(m) => {
            let mm = materials::get(m);
            (e_mpa, mm.yield_mpa, mm.nu, mm.name.into())
        }
        None => (if e_mpa > 0.0 { e_mpa } else { 2000.0 }, 0.0, 0.35, "custom".into()),
    }
}

// ---------------------------------------------------------------- beams

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct BeamIn {
    /// "cantilever", "simply_supported" or "fixed_fixed".
    pub support: String,
    /// "point" or "udl" (uniformly distributed).
    pub load: String,
    pub length_mm: f64,
    /// Point load (N) or total distributed load (N).
    pub force_n: f64,
    /// Point-load position from the left end / root, mm (default: tip or centre).
    pub position_mm: Option<f64>,
    pub section: Section,
    pub e_mpa: f64,
    pub material: Option<String>,
}

impl Default for BeamIn {
    fn default() -> Self {
        BeamIn {
            support: "cantilever".into(),
            load: "point".into(),
            length_mm: 75.0,
            force_n: 5.0,
            position_mm: None,
            section: Section::default(),
            e_mpa: 0.0,
            material: Some("PETG".into()),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct BeamOut {
    pub section: SectionProps,
    pub e_mpa: f64,
    pub max_moment_nmm: f64,
    pub max_moment_x_mm: f64,
    pub max_stress_mpa: f64,
    pub max_deflection_mm: f64,
    pub stiffness_n_mm: f64,
    pub yield_mpa: f64,
    pub safety_factor: f64,
    pub x: Vec<f64>,
    /// Deflection (positive in the load direction), mm.
    pub deflection: Vec<f64>,
    /// Bending moment, N·mm (sagging positive).
    pub moment: Vec<f64>,
    pub formula: String,
}

/// Closed-form Euler–Bernoulli beam cases (Roark / Shigley tables).
pub fn beam(i: BeamIn) -> Result<BeamOut, String> {
    let sp = section_props(&i.section)?;
    let (e, sy, _, _) = modulus(i.e_mpa, &i.material);
    let l = i.length_mm;
    if !(l > 0.0 && e > 0.0) {
        return Err("length and modulus must be positive".into());
    }
    let ei = e * sp.i_mm4;
    let f = i.force_n;
    let n = 81;
    let xs: Vec<f64> = (0..n).map(|k| l * k as f64 / (n - 1) as f64).collect();
    let point = i.load != "udl";
    let w = f / l; // N/mm for UDL
    let (defl, mom, formula): (Vec<f64>, Vec<f64>, String) = match (i.support.as_str(), point) {
        ("simply_supported", true) => {
            let a = i.position_mm.unwrap_or(l / 2.0).clamp(0.0, l);
            let b = l - a;
            (
                xs.iter()
                    .map(|&x| {
                        if x <= a {
                            f * b * x * (l * l - b * b - x * x) / (6.0 * l * ei)
                        } else {
                            let xr = l - x;
                            f * a * xr * (l * l - a * a - xr * xr) / (6.0 * l * ei)
                        }
                    })
                    .collect(),
                xs.iter()
                    .map(|&x| if x <= a { f * b * x / l } else { f * a * (l - x) / l })
                    .collect(),
                "δ(x ≤ a) = F b x (L² − b² − x²) / (6 L E I);  M_max = F a b / L".into(),
            )
        }
        ("simply_supported", false) => (
            xs.iter()
                .map(|&x| w * x * (l.powi(3) - 2.0 * l * x * x + x.powi(3)) / (24.0 * ei))
                .collect(),
            xs.iter().map(|&x| w * x * (l - x) / 2.0).collect(),
            "δ_max = 5 w L⁴ / (384 E I);  M_max = w L² / 8".into(),
        ),
        ("fixed_fixed", true) => (
            xs.iter()
                .map(|&x| {
                    let xx = if x <= l / 2.0 { x } else { l - x };
                    f * xx * xx * (3.0 * l - 4.0 * xx) / (48.0 * ei)
                })
                .collect(),
            xs.iter()
                .map(|&x| {
                    let xx = if x <= l / 2.0 { x } else { l - x };
                    f * xx / 2.0 - f * l / 8.0
                })
                .collect(),
            "centre load: δ_max = F L³ / (192 E I);  M = F L / 8 at the ends and centre".into(),
        ),
        ("fixed_fixed", false) => (
            xs.iter().map(|&x| w * x * x * (l - x).powi(2) / (24.0 * ei)).collect(),
            xs.iter().map(|&x| w / 12.0 * (6.0 * l * x - 6.0 * x * x - l * l)).collect(),
            "δ_max = w L⁴ / (384 E I);  M_max = w L² / 12 at the ends".into(),
        ),
        (_, true) => {
            let a = i.position_mm.unwrap_or(l).clamp(0.0, l);
            (
                xs.iter()
                    .map(|&x| {
                        if x <= a {
                            f * x * x * (3.0 * a - x) / (6.0 * ei)
                        } else {
                            f * a * a * (3.0 * x - a) / (6.0 * ei)
                        }
                    })
                    .collect(),
                xs.iter().map(|&x| if x <= a { -f * (a - x) } else { 0.0 }).collect(),
                "tip load: δ = F L³ / (3 E I);  M_max = F L at the root;  σ = M c / I".into(),
            )
        }
        (_, false) => (
            xs.iter()
                .map(|&x| w * x * x * (6.0 * l * l - 4.0 * l * x + x * x) / (24.0 * ei))
                .collect(),
            xs.iter().map(|&x| -w * (l - x).powi(2) / 2.0).collect(),
            "δ_max = w L⁴ / (8 E I);  M_max = w L² / 2 at the root".into(),
        ),
    };
    let (kmax, mmax) = mom
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .map(|(k, m)| (k, m.abs()))
        .unwrap_or((0, 0.0));
    let dmax = defl.iter().cloned().fold(0.0, f64::max);
    let smax = mmax / sp.z_mm3;
    Ok(BeamOut {
        e_mpa: e,
        max_moment_nmm: mmax,
        max_moment_x_mm: xs[kmax],
        max_stress_mpa: smax,
        max_deflection_mm: dmax,
        stiffness_n_mm: if dmax > 0.0 { f / dmax } else { 0.0 },
        yield_mpa: sy,
        safety_factor: if smax > 0.0 && sy > 0.0 { sy / smax } else { 0.0 },
        section: sp,
        x: xs,
        deflection: defl,
        moment: mom,
        formula,
    })
}

// ---------------------------------------------------------------- snap fits

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct SnapIn {
    /// Arm length L (root to the hook), mm.
    pub length_mm: f64,
    /// Root thickness t, mm.
    pub thickness_mm: f64,
    /// Arm width b, mm.
    pub width_mm: f64,
    /// Undercut / deflection y, mm.
    pub undercut_mm: f64,
    /// "constant" (Bayer design type 1) or "tapered" (thickness halves to the tip, type 2).
    pub profile: String,
    /// Lead-in angle α, degrees.
    pub lead_deg: f64,
    /// Retention (return) angle α′, degrees (90 = permanent).
    pub return_deg: f64,
    pub friction: f64,
    pub material: String,
    /// Override modulus (secant modulus at the working strain), MPa.
    pub e_mpa: f64,
}

impl Default for SnapIn {
    fn default() -> Self {
        SnapIn {
            length_mm: 8.0,
            thickness_mm: 1.2,
            width_mm: 4.0,
            undercut_mm: 0.6,
            profile: "constant".into(),
            lead_deg: 30.0,
            return_deg: 45.0,
            friction: 0.3,
            material: "PETG".into(),
            e_mpa: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SnapOut {
    pub strain_pct: f64,
    pub allowable_strain_pct: f64,
    pub utilisation: f64,
    pub max_undercut_mm: f64,
    pub deflection_force_n: f64,
    pub mating_force_n: f64,
    /// Separation force, N (∞ = self-locking).
    pub separation_force_n: f64,
    pub self_locking: bool,
    pub e_mpa: f64,
    pub root_stress_mpa: f64,
    /// Arm deflection shape at full undercut (x, y) for drawing, mm.
    pub shape_x: Vec<f64>,
    pub shape_y: Vec<f64>,
    pub verdict: String,
}

/// Bayer/Covestro cantilever snap fit: ε = 1.5 y t / L² (constant section),
/// ε = 0.92 y t / L² (thickness tapering to t/2); P = b t² E_s ε / (6 L);
/// mating force W = P (μ + tan α) / (1 − μ tan α).
pub fn snap_fit(i: SnapIn) -> Result<SnapOut, String> {
    let m = materials::get(&i.material);
    let e = if i.e_mpa > 0.0 { i.e_mpa } else { m.e_mpa() };
    let (l, t, b, y) = (i.length_mm, i.thickness_mm, i.width_mm, i.undercut_mm);
    if !(l > 0.0 && t > 0.0 && b > 0.0 && y >= 0.0) {
        return Err("snap-fit dimensions must be positive".into());
    }
    let k = if i.profile == "tapered" { 1.0 / 1.09 } else { 1.0 / 0.67 };
    let eps = k * y * t / (l * l);
    let p = b * t * t * e * eps / (6.0 * l);
    let mu = i.friction.max(0.0);
    let ta = i.lead_deg.to_radians().tan();
    let w = p * (mu + ta) / (1.0 - mu * ta).max(1e-9);
    let tr = i.return_deg.clamp(0.0, 90.0).to_radians().tan();
    let denom = 1.0 - mu * tr;
    let self_locking = i.return_deg >= 89.9 || denom <= 0.0;
    let sep = if self_locking { f64::INFINITY } else { p * (mu + tr) / denom };
    let allow = m.snap_strain_pct;
    let n = 41;
    let shape_x: Vec<f64> = (0..n).map(|q| l * q as f64 / (n - 1) as f64).collect();
    // Tip-loaded cantilever shape scaled to y at the hook.
    let shape_y: Vec<f64> = shape_x.iter().map(|&x| y * x * x * (3.0 * l - x) / (2.0 * l.powi(3))).collect();
    let util = eps * 100.0 / allow;
    let verdict = if util <= 0.6 {
        "Fine for repeated assembly (below ~60 % of the single-use strain limit).".to_string()
    } else if util <= 1.0 {
        "OK for a one-time (or few-times) snap; too high for frequent opening.".to_string()
    } else {
        format!(
            "Strain exceeds the typical allowable for {} — lengthen the arm (ε ∝ 1/L²), thin the root, taper it, or reduce the undercut.",
            m.name
        )
    };
    Ok(SnapOut {
        strain_pct: eps * 100.0,
        allowable_strain_pct: allow,
        utilisation: util,
        max_undercut_mm: allow / 100.0 * l * l / (k * t),
        deflection_force_n: p,
        mating_force_n: w,
        separation_force_n: sep,
        self_locking,
        e_mpa: e,
        root_stress_mpa: e * eps,
        shape_x,
        shape_y,
        verdict,
    })
}

// ---------------------------------------------------------------- bosses & inserts

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct BossIn {
    /// Screw nominal (major) diameter, mm.
    pub screw_d_mm: f64,
    /// Thread engagement length, mm.
    pub engagement_mm: f64,
    /// Nominal part wall thickness, mm.
    pub wall_mm: f64,
    pub material: String,
    /// Service pull-out load to check, N.
    pub load_n: f64,
}

impl Default for BossIn {
    fn default() -> Self {
        BossIn {
            screw_d_mm: 2.0,
            engagement_mm: 4.0,
            wall_mm: 1.2,
            material: "PETG".into(),
            load_n: 20.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct InsertRow {
    pub thread: &'static str,
    pub hole_mm: f64,
    pub insert_od_mm: f64,
    pub length_mm: f64,
    pub min_boss_od_mm: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct BossOut {
    pub pilot_hole_mm: f64,
    pub boss_od_mm: f64,
    pub boss_od_range_mm: [f64; 2],
    pub min_engagement_mm: f64,
    pub max_boss_wall_at_base_mm: f64,
    pub base_fillet_mm: f64,
    pub pullout_force_n: f64,
    pub shear_strength_mpa: f64,
    pub safety_factor: f64,
    pub notes: Vec<String>,
    pub inserts: Vec<InsertRow>,
}

/// Screw-boss rules of thumb for thermoplastics and a thread-forming screw
/// pull-out estimate F ≈ π D_p L τ (Malloy), τ ≈ σ_y / √3, D_p ≈ mean of
/// the screw diameter and pilot hole.
pub fn boss(i: BossIn) -> Result<BossOut, String> {
    let m = materials::get(&i.material);
    let d = i.screw_d_mm;
    if !(d > 0.0 && i.engagement_mm > 0.0) {
        return Err("screw diameter and engagement must be positive".into());
    }
    let brittle = m.elongation_pct < 5.0 || m.id.contains("GF");
    let pilot = if brittle { 0.85 * d } else { 0.8 * d };
    let od = if brittle { 2.5 * d } else { 2.0 * d };
    let tau = m.yield_mpa / 3f64.sqrt();
    let dp = 0.5 * (d + pilot);
    let f = std::f64::consts::PI * dp * i.engagement_mm * tau;
    let mut notes = vec![
        "Pilot hole ≈ 0.75–0.85 × screw diameter for thread-forming screws in thermoplastics — check the screw maker's table for the exact screw and resin.".into(),
        "Boss outer diameter ≈ 2 × d (ductile) to 2.5 × d (brittle or glass-filled) avoids hoop cracking.".into(),
        "Keep the boss wall at its base ≤ ~60 % of the nominal wall (sink marks) and core it out; tie tall bosses to walls with ribs.".into(),
        "Pull-out estimate assumes the plastic shears on the cylinder at the pitch diameter; real values depend on thread form and hole finish — test.".into(),
    ];
    if m.printed {
        notes.push("3D-printed bosses: threads formed across layers are weaker; heat-set inserts are the reliable choice for parts opened repeatedly.".into());
    }
    notes.push(
        "Insert table: typical dimensions for brass heat-set inserts in printed parts — use the hole size your insert maker specifies."
            .into(),
    );
    let inserts = vec![
        InsertRow {
            thread: "M2",
            hole_mm: 3.2,
            insert_od_mm: 3.6,
            length_mm: 3.0,
            min_boss_od_mm: 7.2,
        },
        InsertRow {
            thread: "M2.5",
            hole_mm: 3.6,
            insert_od_mm: 4.0,
            length_mm: 4.0,
            min_boss_od_mm: 8.0,
        },
        InsertRow {
            thread: "M3",
            hole_mm: 4.0,
            insert_od_mm: 4.6,
            length_mm: 4.0,
            min_boss_od_mm: 9.2,
        },
        InsertRow {
            thread: "M4",
            hole_mm: 5.6,
            insert_od_mm: 6.0,
            length_mm: 6.0,
            min_boss_od_mm: 12.0,
        },
    ];
    Ok(BossOut {
        pilot_hole_mm: pilot,
        boss_od_mm: od,
        boss_od_range_mm: [2.0 * d, 2.5 * d],
        min_engagement_mm: 2.0 * d,
        max_boss_wall_at_base_mm: 0.6 * i.wall_mm,
        base_fillet_mm: 0.25 * i.wall_mm,
        pullout_force_n: f,
        shear_strength_mpa: tau,
        safety_factor: if i.load_n > 0.0 { f / i.load_n } else { f64::INFINITY },
        notes,
        inserts,
    })
}

// ---------------------------------------------------------------- bolted joints

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct BoltIn {
    /// Metric coarse thread size, e.g. "M3".
    pub size: String,
    /// Property class: "4.8", "5.8", "8.8", "10.9", "12.9", "A2-70".
    pub class: String,
    /// Grip length (clamped thickness), mm.
    pub grip_mm: f64,
    /// Nut factor K (≈ 0.2 dry steel, ≈ 0.15 lubricated).
    pub nut_factor: f64,
    /// Preload as a fraction of the proof load (0.75 reusable, 0.9 permanent).
    pub preload_fraction: f64,
    /// Optional applied torque, N·m (overrides the preload fraction).
    pub torque_nm: Option<f64>,
    /// Member modulus, MPa.
    pub member_e_mpa: f64,
    /// External separating load per bolt, N.
    pub load_n: f64,
    /// Material the internal thread is cut or formed in (tapped part, insert or nut).
    pub nut_material: String,
    /// Length of thread engagement, mm.
    pub engagement_mm: f64,
}

impl Default for BoltIn {
    fn default() -> Self {
        BoltIn {
            size: "M3".into(),
            class: "8.8".into(),
            grip_mm: 6.0,
            nut_factor: 0.2,
            preload_fraction: 0.75,
            torque_nm: None,
            member_e_mpa: 68_900.0,
            load_n: 500.0,
            nut_material: "AL6061-T6".into(),
            engagement_mm: 4.5,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct BoltOut {
    pub d_mm: f64,
    pub pitch_mm: f64,
    pub stress_area_mm2: f64,
    pub proof_mpa: f64,
    pub proof_load_n: f64,
    pub preload_n: f64,
    pub torque_nm: f64,
    pub bolt_stiffness_n_mm: f64,
    pub member_stiffness_n_mm: f64,
    pub joint_constant: f64,
    pub bolt_load_n: f64,
    pub member_load_n: f64,
    pub separation_load_n: f64,
    pub load_factor: f64,
    pub separation_factor: f64,
    pub bolt_stress_mpa: f64,
    /// Internal-thread shear area at the bolt major diameter, mm².
    pub strip_area_internal_mm2: f64,
    /// External-thread shear area at the nut minor diameter, mm².
    pub strip_area_external_mm2: f64,
    /// Force that strips the internal (tapped) thread, N.
    pub strip_force_internal_n: f64,
    /// Force that strips the bolt's own thread, N.
    pub strip_force_external_n: f64,
    /// Force that breaks the bolt in tension (proof load), N.
    pub tensile_capacity_n: f64,
    /// What fails first as the load rises.
    pub first_failure: String,
    /// Engagement at which the internal thread is as strong as the bolt (proof), mm.
    pub min_engagement_mm: f64,
    pub nut_material: String,
    pub notes: Vec<String>,
}

/// ISO metric coarse pitches.
pub fn coarse_pitch(size: &str) -> Option<(f64, f64)> {
    let t = [
        ("M1.6", 1.6, 0.35),
        ("M2", 2.0, 0.4),
        ("M2.5", 2.5, 0.45),
        ("M3", 3.0, 0.5),
        ("M4", 4.0, 0.7),
        ("M5", 5.0, 0.8),
        ("M6", 6.0, 1.0),
        ("M8", 8.0, 1.25),
        ("M10", 10.0, 1.5),
        ("M12", 12.0, 1.75),
    ];
    t.iter().find(|r| r.0.eq_ignore_ascii_case(size.trim())).map(|r| (r.1, r.2))
}

/// Minimum proof strength (MPa) by property class (ISO 898-1 for steel up to
/// M16; A2-70 uses the 0.2 % proof stress from ISO 3506-1).
pub fn proof_strength(class: &str) -> Option<f64> {
    match class {
        "4.8" => Some(310.0),
        "5.8" => Some(380.0),
        "8.8" => Some(580.0),
        "10.9" => Some(830.0),
        "12.9" => Some(970.0),
        "A2-70" | "A4-70" => Some(450.0),
        _ => None,
    }
}

/// Bolted joint (Shigley ch. 8): stress area A_t = π/4 (d − 0.9382 p)²,
/// preload F_i = f · S_p A_t, torque T = K F_i d, bolt stiffness A E / l,
/// member stiffness (Wileman) k_m = E d A e^{B d / l} with A = 0.78715,
/// B = 0.62873, joint constant C = k_b / (k_b + k_m).
pub fn bolt(i: BoltIn) -> Result<BoltOut, String> {
    let (d, p) = coarse_pitch(&i.size).ok_or_else(|| format!("unknown thread size `{}` (use M1.6 … M12)", i.size))?;
    let sp = proof_strength(&i.class).ok_or_else(|| format!("unknown property class `{}`", i.class))?;
    let at = std::f64::consts::PI / 4.0 * (d - 0.9382 * p).powi(2);
    let fp = sp * at;
    let k = i.nut_factor.clamp(0.05, 0.4);
    let (fi, t) = match i.torque_nm {
        Some(tq) if tq > 0.0 => {
            let fi = tq * 1000.0 / (k * d);
            (fi, tq)
        }
        _ => {
            let fi = i.preload_fraction.clamp(0.1, 1.0) * fp;
            (fi, k * fi * d / 1000.0)
        }
    };
    let l = i.grip_mm.max(0.1);
    let eb = if i.class.starts_with('A') { 193_000.0 } else { 207_000.0 };
    let kb = std::f64::consts::PI / 4.0 * d * d * eb / l;
    let km = i.member_e_mpa.max(1.0) * d * 0.78715 * (0.62873 * d / l).exp();
    let c = kb / (kb + km);
    let pl = i.load_n.max(0.0);
    let mut notes = vec![
        "Nut factor K is uncertain (±25 % or more): torque is a poor proxy for preload. Use K ≈ 0.2 for plain dry steel, ≈ 0.15 lubricated.".into(),
        "Member stiffness uses Wileman's fit for members of one material; it is approximate for mixed stacks.".into(),
    ];
    if i.member_e_mpa < 10_000.0 {
        notes.push("Plastic members creep and relax the preload: use metal inserts or compression limiters and modest torque.".into());
    }
    // Thread stripping on the ISO basic profile (60° thread, H = 0.866 p):
    // at the bolt major diameter d the internal thread fills 7/8 of every
    // pitch (the bolt crest flat is p/8); at the nut minor diameter
    // D1 = d − 1.0825 p the external thread fills 3/4 (the nut crest is p/4).
    // Shear strength τ ≈ 0.577 S_y (von Mises). Real threads carry tolerance
    // and uneven load sharing, so treat these as upper bounds.
    let le = i.engagement_mm.max(0.1);
    let nut = materials::get(&i.nut_material);
    let d1 = d - 1.0825 * p;
    let a_int = std::f64::consts::PI * d * le * 0.875;
    let a_ext = std::f64::consts::PI * d1 * le * 0.75;
    let tau_nut = 0.577 * nut.yield_mpa;
    let tau_bolt = 0.577 * sp;
    let f_int = a_int * tau_nut;
    let f_ext = a_ext * tau_bolt;
    let first = if fp <= f_int.min(f_ext) {
        "bolt breaks in tension first (good: a visible, safe failure)"
    } else if f_int <= f_ext {
        "internal thread strips first — lengthen the engagement or use a stronger insert"
    } else {
        "bolt thread strips first — lengthen the engagement"
    };
    let le_min = fp / (std::f64::consts::PI * d * 0.875 * tau_nut);
    notes.push(format!(
        "Thread stripping uses the basic-profile shear areas (upper bounds); in {} a rule of thumb is engagement ≥ {:.1} × d for full bolt strength.",
        nut.name,
        (le_min / d).max(0.5)
    ));
    Ok(BoltOut {
        d_mm: d,
        pitch_mm: p,
        stress_area_mm2: at,
        proof_mpa: sp,
        proof_load_n: fp,
        preload_n: fi,
        torque_nm: t,
        bolt_stiffness_n_mm: kb,
        member_stiffness_n_mm: km,
        joint_constant: c,
        bolt_load_n: fi + c * pl,
        member_load_n: fi - (1.0 - c) * pl,
        separation_load_n: fi / (1.0 - c),
        load_factor: if pl > 0.0 { (fp - fi) / (c * pl) } else { f64::INFINITY },
        separation_factor: if pl > 0.0 { fi / (pl * (1.0 - c)) } else { f64::INFINITY },
        bolt_stress_mpa: (fi + c * pl) / at,
        strip_area_internal_mm2: a_int,
        strip_area_external_mm2: a_ext,
        strip_force_internal_n: f_int,
        strip_force_external_n: f_ext,
        tensile_capacity_n: fp,
        first_failure: first.into(),
        min_engagement_mm: le_min,
        nut_material: nut.id.into(),
        notes,
    })
}

// ---------------------------------------------------------------- press fits

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct PressIn {
    /// Nominal interface diameter, mm.
    pub d_mm: f64,
    /// Diametral interference, mm.
    pub interference_mm: f64,
    /// Hub outer diameter, mm.
    pub hub_od_mm: f64,
    /// Shaft bore (0 = solid), mm.
    pub shaft_id_mm: f64,
    pub length_mm: f64,
    pub friction: f64,
    pub hub_material: String,
    pub shaft_material: String,
}

impl Default for PressIn {
    fn default() -> Self {
        PressIn {
            d_mm: 1.5,
            interference_mm: 0.02,
            hub_od_mm: 4.0,
            shaft_id_mm: 0.0,
            length_mm: 5.0,
            friction: 0.3,
            hub_material: "PETG".into(),
            shaft_material: "SS304".into(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct PressOut {
    pub pressure_mpa: f64,
    pub hub_hoop_mpa: f64,
    pub hub_von_mises_mpa: f64,
    pub shaft_hoop_mpa: f64,
    pub hub_yield_mpa: f64,
    pub hub_safety_factor: f64,
    pub axial_force_n: f64,
    pub torque_nm: f64,
    pub hub_radial_growth_um: f64,
    /// Radial profiles for drawing: r (mm), hoop and radial stress in the hub (MPa).
    pub r: Vec<f64>,
    pub hoop: Vec<f64>,
    pub radial: Vec<f64>,
}

/// Lamé thick-cylinder press fit (Shigley §3-16):
/// p = δ / [ (R/E_o)((r_o² + R²)/(r_o² − R²) + ν_o) + (R/E_i)((R² + r_i²)/(R² − r_i²) − ν_i) ]
/// with δ the radial interference; hub hoop stress at the bore
/// σ_t = p (r_o² + R²)/(r_o² − R²), radial σ_r = −p.
pub fn press_fit(i: PressIn) -> Result<PressOut, String> {
    let hub = materials::get(&i.hub_material);
    let sh = materials::get(&i.shaft_material);
    let r = i.d_mm / 2.0;
    let ro = i.hub_od_mm / 2.0;
    let ri = (i.shaft_id_mm / 2.0).max(0.0);
    if !(r > 0.0 && ro > r && ri < r) {
        return Err("need shaft bore < interface diameter < hub outer diameter".into());
    }
    let delta = i.interference_mm / 2.0;
    let term_o = r / hub.e_mpa() * ((ro * ro + r * r) / (ro * ro - r * r) + hub.nu);
    let term_i = r / sh.e_mpa() * ((r * r + ri * ri) / (r * r - ri * ri) - sh.nu);
    let p = delta / (term_o + term_i);
    let hoop = p * (ro * ro + r * r) / (ro * ro - r * r);
    let vm = (hoop * hoop + hoop * p + p * p).sqrt(); // σr = −p, σt = hoop, σz = 0
    let shaft_hoop = -p * (r * r + ri * ri) / (r * r - ri * ri);
    let area = std::f64::consts::PI * i.d_mm * i.length_mm;
    let fa = i.friction * p * area;
    let n = 30;
    let rs: Vec<f64> = (0..n).map(|k| r + (ro - r) * k as f64 / (n - 1) as f64).collect();
    let a = p * r * r / (ro * ro - r * r);
    Ok(PressOut {
        pressure_mpa: p,
        hub_hoop_mpa: hoop,
        hub_von_mises_mpa: vm,
        shaft_hoop_mpa: shaft_hoop,
        hub_yield_mpa: hub.yield_mpa,
        hub_safety_factor: hub.yield_mpa / vm,
        axial_force_n: fa,
        torque_nm: fa * r / 1000.0,
        hub_radial_growth_um: 1000.0 * term_o * p,
        hoop: rs.iter().map(|&x| a * (1.0 + ro * ro / (x * x))).collect(),
        radial: rs.iter().map(|&x| a * (1.0 - ro * ro / (x * x))).collect(),
        r: rs,
    })
}

// ---------------------------------------------------------------- buckling

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct BuckleIn {
    pub length_mm: f64,
    pub section: Section,
    /// "pinned_pinned", "fixed_free", "fixed_pinned", "fixed_fixed".
    pub ends: String,
    pub material: String,
    /// Applied compressive load, N.
    pub load_n: f64,
    /// Plate check: unloaded width b of a wall/rib panel, mm (0 = skip).
    pub plate_width_mm: f64,
    /// Plate edge support: "ss" (all edges simply supported) or "free" (one free edge, a rib).
    pub plate_edges: String,
}

impl Default for BuckleIn {
    fn default() -> Self {
        BuckleIn {
            length_mm: 12.0,
            section: Section {
                shape: "rect".into(),
                b: 6.0,
                h: 0.8,
                t: 0.0,
            },
            ends: "fixed_free".into(),
            material: "PETG".into(),
            load_n: 10.0,
            plate_width_mm: 6.0,
            plate_edges: "free".into(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct BuckleOut {
    pub k_factor: f64,
    pub slenderness: f64,
    pub transition_slenderness: f64,
    pub euler_load_n: f64,
    pub johnson_load_n: f64,
    pub critical_load_n: f64,
    pub regime: String,
    pub safety_factor: f64,
    pub plate_k: f64,
    pub plate_stress_mpa: f64,
    pub plate_load_n: f64,
    pub applied_stress_mpa: f64,
    /// Critical load vs effective length curve for plotting (L, P_cr).
    pub curve_l: Vec<f64>,
    pub curve_p: Vec<f64>,
}

/// Euler column P = π² E I / (K L)² with Johnson's parabola below the
/// transition slenderness √(2π²E/σ_y); thin-plate buckling
/// σ_cr = k π² E / (12 (1 − ν²)) (t/b)² with k = 4 (edges simply supported)
/// or 0.425 (one unloaded edge free).
pub fn buckling(i: BuckleIn) -> Result<BuckleOut, String> {
    let m = materials::get(&i.material);
    let sp = section_props(&i.section)?;
    let k = match i.ends.as_str() {
        "pinned_pinned" => 1.0,
        "fixed_pinned" => 0.699,
        "fixed_fixed" => 0.5,
        _ => 2.0,
    };
    let e = m.e_mpa();
    let sy = m.yield_mpa;
    let pi2 = std::f64::consts::PI.powi(2);
    let pcr_at = |l: f64| -> (f64, f64, f64) {
        let le = k * l;
        let euler = pi2 * e * sp.i_mm4 / (le * le);
        let lam = le / sp.r_gyr_mm;
        let john = sp.area_mm2 * (sy - (sy * lam / (2.0 * std::f64::consts::PI)).powi(2) / e);
        (euler, john.max(0.0), lam)
    };
    let (euler, john, lam) = pcr_at(i.length_mm);
    let lam_c = (2.0 * pi2 * e / sy).sqrt();
    let (pcr, regime) = if lam >= lam_c {
        (euler, "Euler (elastic buckling)")
    } else {
        (john, "Johnson (inelastic, short column)")
    };
    let plate_k = if i.plate_edges == "ss" { 4.0 } else { 0.425 };
    let t = if i.section.shape == "rect" { i.section.h } else { i.section.t };
    let (ps, pl) = if i.plate_width_mm > 0.0 && t > 0.0 {
        let s = plate_k * pi2 * e / (12.0 * (1.0 - m.nu * m.nu)) * (t / i.plate_width_mm).powi(2);
        (s, s * t * i.plate_width_mm)
    } else {
        (0.0, 0.0)
    };
    let n = 60;
    let curve_l: Vec<f64> = (1..=n).map(|q| i.length_mm * 3.0 * q as f64 / n as f64).collect();
    let curve_p: Vec<f64> = curve_l
        .iter()
        .map(|&l| {
            let (eu, jo, la) = pcr_at(l);
            if la >= lam_c {
                eu
            } else {
                jo
            }
        })
        .collect();
    Ok(BuckleOut {
        k_factor: k,
        slenderness: lam,
        transition_slenderness: lam_c,
        euler_load_n: euler,
        johnson_load_n: john,
        critical_load_n: pcr,
        regime: regime.into(),
        safety_factor: if i.load_n > 0.0 { pcr / i.load_n } else { f64::INFINITY },
        plate_k,
        plate_stress_mpa: ps,
        plate_load_n: pl,
        applied_stress_mpa: i.load_n / sp.area_mm2,
        curve_l,
        curve_p,
    })
}

// ---------------------------------------------------------------- living hinge

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct HingeIn {
    pub thickness_mm: f64,
    pub length_mm: f64,
    pub angle_deg: f64,
    pub material: String,
}

impl Default for HingeIn {
    fn default() -> Self {
        HingeIn {
            thickness_mm: 0.35,
            length_mm: 1.5,
            angle_deg: 180.0,
            material: "PP".into(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct HingeOut {
    pub strain_pct: f64,
    pub bend_radius_mm: f64,
    pub guidance_limit_pct: f64,
    pub verdict: String,
    pub min_length_mm: f64,
}

/// A living hinge bends its thin section into an arc of radius R = L/θ;
/// the outer-fibre strain is ε = t / (2R) = t θ / (2L).
pub fn living_hinge(i: HingeIn) -> Result<HingeOut, String> {
    let m = materials::get(&i.material);
    let th = i.angle_deg.to_radians();
    if !(i.thickness_mm > 0.0 && i.length_mm > 0.0 && th > 0.0) {
        return Err("thickness, length and angle must be positive".into());
    }
    let eps = i.thickness_mm * th / (2.0 * i.length_mm);
    let (limit, kind) = match m.id {
        "PP" => (50.0, "molded polypropylene: the flow orients the molecules across the hinge; large strains on the first flex are expected (flex it once right after molding)"),
        "TPU95A" => (50.0, "elastomer: strains of tens of percent stay elastic"),
        "PA12" => (8.0, "powder-bed nylon hinges work for tens to hundreds of cycles when kept thin"),
        _ => (0.6 * m.snap_strain_pct, "rigid thermoplastic: treat like a repeated snap fit"),
    };
    let verdict = if eps * 100.0 <= limit {
        format!("{:.1} % strain is within rough guidance for this material ({kind}).", eps * 100.0)
    } else {
        format!(
            "{:.1} % strain is beyond rough guidance ({limit:.0} %) — lengthen the hinge, thin it, or use PP/TPU ({kind}).",
            eps * 100.0
        )
    };
    Ok(HingeOut {
        strain_pct: eps * 100.0,
        bend_radius_mm: i.length_mm / th,
        guidance_limit_pct: limit,
        verdict,
        min_length_mm: i.thickness_mm * th / (2.0 * limit / 100.0),
    })
}

// ---------------------------------------------------------------- thermal expansion mismatch

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ThermalIn {
    /// Outer part (housing, pod, frame).
    pub outer: String,
    /// Inner part (PCB, lens, metal insert).
    pub inner: String,
    /// Length over which the parts fit, mm.
    pub length_mm: f64,
    /// Clearance at the assembly temperature (negative = interference), mm.
    pub clearance_mm: f64,
    pub t_assembly_c: f64,
    pub t_min_c: f64,
    pub t_max_c: f64,
    /// Bonded bi-material strip: thickness of the outer and inner layers, mm.
    pub t_outer_mm: f64,
    pub t_inner_mm: f64,
}

impl Default for ThermalIn {
    fn default() -> Self {
        ThermalIn {
            outer: "PETG".into(),
            inner: "FR-4".into(),
            length_mm: 60.0,
            clearance_mm: 0.4,
            t_assembly_c: 23.0,
            t_min_c: -10.0,
            t_max_c: 50.0,
            t_outer_mm: 1.2,
            t_inner_mm: 0.8,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ThermalOut {
    pub alpha_outer_ppm: f64,
    pub alpha_inner_ppm: f64,
    /// Change of clearance per kelvin, µm/K (positive = opens when hot).
    pub dclear_um_per_k: f64,
    pub clearance_cold_mm: f64,
    pub clearance_hot_mm: f64,
    /// Temperature where the clearance reaches zero (None if it never closes in ±200 K).
    pub closing_temp_c: Option<f64>,
    /// Axial stress if the parts were rigidly constrained against each other
    /// over the whole range (upper bound, uniaxial): E_outer · Δα · ΔT, MPa.
    pub constrained_stress_mpa: f64,
    pub outer_yield_mpa: f64,
    /// Bonded strip (Timoshenko 1925): curvature per kelvin, 1/m per K.
    pub bimetal_curvature_per_k: f64,
    /// Bonded strip: bow (mid-span sagitta) of `length_mm` at the cold end, mm.
    pub bimetal_bow_cold_mm: f64,
    pub bimetal_bow_hot_mm: f64,
    pub temps_c: Vec<f64>,
    pub clearance_mm: Vec<f64>,
    pub notes: Vec<String>,
}

/// Differential thermal expansion: Δc = (α_o − α_i) L ΔT. Bonded two-layer
/// strip: S. Timoshenko, "Analysis of bi-metal thermostats", J. Opt. Soc.
/// Am. 11, 1925 — κ = 6 Δα ΔT (1 + m)² / (h [3 (1 + m)² + (1 + m n)(m² + 1/(m n))])
/// with m = t₁/t₂, n = E₁/E₂, h = t₁ + t₂.
pub fn thermal(i: ThermalIn) -> Result<ThermalOut, String> {
    let o = materials::get(&i.outer);
    let n_ = materials::get(&i.inner);
    let l = i.length_mm;
    if l.is_nan() || l <= 0.0 {
        return Err("length must be positive".into());
    }
    let da = (o.cte_ppm_k - n_.cte_ppm_k) * 1e-6;
    let c_at = |t: f64| i.clearance_mm + da * l * (t - i.t_assembly_c);
    let temps: Vec<f64> = (0..=40).map(|k| i.t_min_c + (i.t_max_c - i.t_min_c) * k as f64 / 40.0).collect();
    let closing = if da.abs() > 0.0 {
        let t = i.t_assembly_c - i.clearance_mm / (da * l);
        ((t - i.t_assembly_c).abs() <= 200.0).then_some(t)
    } else {
        None
    };
    let dt_max = (i.t_max_c - i.t_assembly_c).abs().max((i.t_assembly_c - i.t_min_c).abs());
    // Bi-material strip (layer 1 = outer, layer 2 = inner).
    let (t1, t2) = (i.t_outer_mm.max(1e-3), i.t_inner_mm.max(1e-3));
    let m = t1 / t2;
    let nn = o.e_mpa() / n_.e_mpa();
    let h_m = (t1 + t2) / 1000.0;
    let kappa = 6.0 * da * (1.0 + m).powi(2) / (h_m * (3.0 * (1.0 + m).powi(2) + (1.0 + m * nn) * (m * m + 1.0 / (m * nn))));
    let bow = |dt: f64| {
        let k = kappa * dt; // 1/m
        let lm = l / 1000.0;
        k * lm * lm / 8.0 * 1000.0 // sagitta ≈ κ L² / 8, mm
    };
    let mut notes = vec![
        format!(
            "{} expands {:.0} ppm/K and {} {:.0} ppm/K (typical values — check the data sheets): over {:.0} mm the clearance changes by {:.1} µm per kelvin.",
            o.name,
            o.cte_ppm_k,
            n_.name,
            n_.cte_ppm_k,
            l,
            da * l * 1000.0
        ),
        "Polymers also shrink after printing or molding and swell with moisture — keep a margin beyond the thermal term.".into(),
    ];
    if c_at(i.t_min_c).min(c_at(i.t_max_c)) < 0.0 {
        notes.push(
            "The clearance closes inside the temperature range: the parts will press on each other (see the constrained stress).".into(),
        );
    }
    Ok(ThermalOut {
        alpha_outer_ppm: o.cte_ppm_k,
        alpha_inner_ppm: n_.cte_ppm_k,
        dclear_um_per_k: da * l * 1000.0,
        clearance_cold_mm: c_at(i.t_min_c),
        clearance_hot_mm: c_at(i.t_max_c),
        closing_temp_c: closing,
        constrained_stress_mpa: o.e_mpa() * da.abs() * dt_max,
        outer_yield_mpa: o.yield_mpa,
        bimetal_curvature_per_k: kappa,
        bimetal_bow_cold_mm: bow(i.t_min_c - i.t_assembly_c),
        bimetal_bow_hot_mm: bow(i.t_max_c - i.t_assembly_c),
        clearance_mm: temps.iter().map(|&t| c_at(t)).collect(),
        temps_c: temps,
        notes,
    })
}

// ---------------------------------------------------------------- stress concentration

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct KtIn {
    /// "hole" (central hole in a finite-width plate, tension), "ellipse"
    /// (elliptical hole in a wide plate) or "notch" (surface notch, depth a, root radius ρ).
    pub geometry: String,
    /// Plate width W (hole), mm.
    pub width_mm: f64,
    /// Hole diameter d (hole), mm.
    pub hole_mm: f64,
    /// Ellipse semi-axis across the load a / notch depth a, mm.
    pub a_mm: f64,
    /// Ellipse semi-axis along the load b, mm.
    pub b_mm: f64,
    /// Notch root radius ρ, mm.
    pub radius_mm: f64,
    /// Plate thickness, mm.
    pub thickness_mm: f64,
    /// Applied tensile force, N.
    pub force_n: f64,
    /// Notch sensitivity q ∈ [0, 1] for the fatigue factor K_f = 1 + q (K_t − 1).
    pub q: f64,
    pub material: String,
}

impl Default for KtIn {
    fn default() -> Self {
        KtIn {
            geometry: "hole".into(),
            width_mm: 8.0,
            hole_mm: 1.6,
            a_mm: 1.0,
            b_mm: 0.5,
            radius_mm: 0.3,
            thickness_mm: 4.0,
            force_n: 100.0,
            q: 1.0,
            material: "PETG".into(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct KtOut {
    pub kt: f64,
    pub kf: f64,
    /// Nominal stress the factor applies to (net section for "hole"), MPa.
    pub nominal_mpa: f64,
    pub peak_mpa: f64,
    pub yield_mpa: f64,
    /// Safety factor on the nominal stress (static, ductile) and on the peak.
    pub sf_nominal: f64,
    pub sf_peak: f64,
    pub basis: String,
    /// K_t vs the geometry ratio for the chart (x = d/W, a/b or a/ρ).
    pub curve_x: Vec<f64>,
    pub curve_kt: Vec<f64>,
    pub curve_label: String,
    pub notes: Vec<String>,
}

/// Stress concentration factors: finite-width plate with a central hole in
/// tension (Heywood's approximation K_tn = 2 + (1 − d/W)³ on the net section,
/// as tabulated in W. D. Pilkey & D. F. Pilkey, *Peterson's Stress
/// Concentration Factors*, 3rd ed., 2008); elliptical hole (Inglis 1913,
/// K_t = 1 + 2a/b); and a notch of depth a and root radius ρ approximated by
/// the equivalent ellipse, K_t ≈ 1 + 2√(a/ρ) (an upper-bound estimate).
pub fn stress_conc(i: KtIn) -> Result<KtOut, String> {
    let m = materials::get(&i.material);
    let t = i.thickness_mm;
    if t.is_nan() || t <= 0.0 {
        return Err("thickness must be positive".into());
    }
    let (kt, nominal, basis, cx, ck, lbl) = match i.geometry.as_str() {
        "ellipse" => {
            if !(i.a_mm > 0.0 && i.b_mm > 0.0 && i.width_mm > 2.0 * i.a_mm) {
                return Err("need a, b > 0 and width > 2a".into());
            }
            let kt = 1.0 + 2.0 * i.a_mm / i.b_mm;
            let x: Vec<f64> = (1..=40).map(|k| 0.25 * k as f64).collect();
            let y = x.iter().map(|r| 1.0 + 2.0 * r).collect();
            (
                kt,
                i.force_n / (i.width_mm * t),
                "gross section F / (W t) — wide plate".to_string(),
                x,
                y,
                "a / b".to_string(),
            )
        }
        "notch" => {
            if !(i.a_mm > 0.0 && i.radius_mm > 0.0 && i.width_mm > i.a_mm) {
                return Err("need depth a > 0, root radius ρ > 0 and width > a".into());
            }
            let kt = 1.0 + 2.0 * (i.a_mm / i.radius_mm).sqrt();
            let x: Vec<f64> = (1..=40).map(|k| 0.5 * k as f64).collect();
            let y = x.iter().map(|r: &f64| 1.0 + 2.0 * r.sqrt()).collect();
            (
                kt,
                i.force_n / ((i.width_mm - i.a_mm) * t),
                "net section F / ((W − a) t)".to_string(),
                x,
                y,
                "a / ρ".to_string(),
            )
        }
        _ => {
            let r = i.hole_mm / i.width_mm;
            if !(i.hole_mm > 0.0 && r < 1.0) {
                return Err("need 0 < hole diameter < width".into());
            }
            let kt = 2.0 + (1.0 - r).powi(3);
            let x: Vec<f64> = (0..=40).map(|k| 0.02 * k as f64).collect();
            let y = x.iter().map(|q| 2.0 + (1.0 - q).powi(3)).collect();
            (
                kt,
                i.force_n / ((i.width_mm - i.hole_mm) * t),
                "net section F / ((W − d) t)".to_string(),
                x,
                y,
                "d / W".to_string(),
            )
        }
    };
    let q = i.q.clamp(0.0, 1.0);
    let kf = 1.0 + q * (kt - 1.0);
    let peak = kt * nominal;
    Ok(KtOut {
        kt,
        kf,
        nominal_mpa: nominal,
        peak_mpa: peak,
        yield_mpa: m.yield_mpa,
        sf_nominal: if nominal > 0.0 { m.yield_mpa / nominal } else { f64::INFINITY },
        sf_peak: if peak > 0.0 { m.yield_mpa / peak } else { f64::INFINITY },
        basis,
        curve_x: cx,
        curve_kt: ck,
        curve_label: lbl,
        notes: vec![
            "Static loads on ductile materials: local yielding blunts the peak, so strength is usually checked on the nominal stress (Shigley). Brittle materials and fatigue feel the full peak.".into(),
            "Fatigue: use K_f = 1 + q (K_t − 1). Notch sensitivity q depends on the material and the notch radius; q = 1 (full sensitivity) is the safe assumption for printed and glass-filled plastics.".into(),
            "Printed parts: every layer line is a small notch, and the nozzle cannot print a radius much below half its width — model inside corners with a real radius.".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cantilever_formula() {
        let o = beam(BeamIn {
            length_mm: 100.0,
            force_n: 10.0,
            section: Section {
                shape: "rect".into(),
                b: 10.0,
                h: 5.0,
                t: 0.0,
            },
            e_mpa: 2000.0,
            material: None,
            ..Default::default()
        })
        .unwrap();
        let i = 10.0 * 125.0 / 12.0;
        assert!((o.section.i_mm4 - i).abs() < 1e-9);
        assert!((o.max_deflection_mm - 10.0 * 1e6 / (3.0 * 2000.0 * i)).abs() < 1e-9);
        assert!((o.max_stress_mpa - 1000.0 * 2.5 / i).abs() < 1e-9);
    }

    #[test]
    fn simply_supported_centre_load() {
        let o = beam(BeamIn {
            support: "simply_supported".into(),
            length_mm: 80.0,
            force_n: 8.0,
            e_mpa: 1000.0,
            material: None,
            section: Section {
                shape: "round".into(),
                b: 0.0,
                h: 4.0,
                t: 0.0,
            },
            ..Default::default()
        })
        .unwrap();
        let i = std::f64::consts::PI * 256.0 / 64.0;
        let exact = 8.0 * 80f64.powi(3) / (48.0 * 1000.0 * i);
        assert!((o.max_deflection_mm - exact).abs() < 1e-9 * exact);
        assert!((o.max_moment_nmm - 160.0).abs() < 1e-9);
    }

    #[test]
    fn snap_fit_bayer() {
        let o = snap_fit(SnapIn {
            length_mm: 10.0,
            thickness_mm: 1.0,
            width_mm: 5.0,
            undercut_mm: 1.0,
            lead_deg: 30.0,
            return_deg: 90.0,
            friction: 0.5,
            material: "PC".into(),
            e_mpa: 2000.0,
            profile: "constant".into(),
        })
        .unwrap();
        // ε = 1.5 · 1 · 1 / 100 ≈ 1.49 %.
        assert!((o.strain_pct - 100.0 / 0.67 / 100.0).abs() < 1e-9);
        let p = 5.0 * 1.0 * 2000.0 * (o.strain_pct / 100.0) / 60.0;
        assert!((o.deflection_force_n - p).abs() < 1e-9);
        let t = 30f64.to_radians().tan();
        assert!((o.mating_force_n - p * (0.5 + t) / (1.0 - 0.5 * t)).abs() < 1e-9);
        assert!(o.self_locking);
    }

    #[test]
    fn bolt_m3_88() {
        let o = bolt(BoltIn::default()).unwrap();
        // A_t for M3 × 0.5 ≈ 5.03 mm² (ISO 898-1 table value 5.03).
        assert!((o.stress_area_mm2 - 5.03).abs() < 0.02, "{}", o.stress_area_mm2);
        assert!((o.proof_load_n - 580.0 * o.stress_area_mm2).abs() < 1e-9);
        assert!((o.torque_nm - 0.2 * o.preload_n * 3.0 / 1000.0).abs() < 1e-12);
        assert!(o.joint_constant > 0.0 && o.joint_constant < 1.0);
    }

    #[test]
    fn press_fit_matches_equal_materials_closed_form() {
        // Same material hub and solid shaft: p = E δ (ro² − R²) / (2 R ro²) (radial δ).
        let o = press_fit(PressIn {
            d_mm: 20.0,
            interference_mm: 0.02,
            hub_od_mm: 40.0,
            shaft_id_mm: 0.0,
            length_mm: 10.0,
            friction: 0.2,
            hub_material: "AL6061-T6".into(),
            shaft_material: "AL6061-T6".into(),
        })
        .unwrap();
        let (e, r, ro, d) = (68_900.0, 10.0, 20.0, 0.01);
        let exact = e * d * (ro * ro - r * r) / (2.0 * r * ro * ro);
        assert!((o.pressure_mpa - exact).abs() < 1e-9 * exact, "{} vs {exact}", o.pressure_mpa);
        assert!((o.hoop[0] - o.hub_hoop_mpa).abs() < 1e-9 * o.hub_hoop_mpa);
        assert!((o.radial[0] + o.pressure_mpa).abs() < 1e-9 * o.pressure_mpa);
    }

    #[test]
    fn euler_column() {
        let o = buckling(BuckleIn {
            length_mm: 200.0,
            section: Section {
                shape: "round".into(),
                b: 0.0,
                h: 4.0,
                t: 0.0,
            },
            ends: "pinned_pinned".into(),
            material: "AL6061-T6".into(),
            load_n: 100.0,
            plate_width_mm: 0.0,
            plate_edges: "ss".into(),
        })
        .unwrap();
        let i = std::f64::consts::PI * 256.0 / 64.0;
        let exact = std::f64::consts::PI.powi(2) * 68_900.0 * i / 40_000.0;
        assert!((o.euler_load_n - exact).abs() < 1e-9 * exact);
        assert_eq!(o.regime, "Euler (elastic buckling)");
    }

    #[test]
    fn thread_stripping_areas() {
        let o = bolt(BoltIn::default()).unwrap();
        // M3 × 0.5, 4.5 mm engagement: A_int = π · 3 · 4.5 · 0.875.
        let a = std::f64::consts::PI * 3.0 * 4.5 * 0.875;
        assert!((o.strip_area_internal_mm2 - a).abs() < 1e-9);
        assert!((o.strip_force_internal_n - a * 0.577 * 276.0).abs() < 1e-6);
        // Engagement for equal strength scales linearly with the proof load.
        assert!((o.min_engagement_mm * std::f64::consts::PI * 3.0 * 0.875 * 0.577 * 276.0 - o.tensile_capacity_n).abs() < 1e-6);
    }

    #[test]
    fn thermal_mismatch_and_bimetal() {
        let o = thermal(ThermalIn::default()).unwrap();
        // (65 − 15) ppm/K × 60 mm = 3 µm/K; −33 K → 0.099 mm tighter.
        assert!((o.dclear_um_per_k - 3.0).abs() < 1e-9);
        assert!((o.clearance_cold_mm - (0.4 - 0.099)).abs() < 1e-9);
        // Equal layers (m = n = 1): κ = 3 Δα ΔT / (2 h) (Timoshenko's special case).
        let e = thermal(ThermalIn {
            outer: "AL6061-T6".into(),
            inner: "AL6061-T6".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(e.bimetal_curvature_per_k, 0.0);
        let da = 50e-6;
        let h = 0.002;
        let m: f64 = 1.0;
        let k = 6.0 * da * (1.0 + m).powi(2) / (h * (3.0 * (1.0 + m).powi(2) + 2.0 * 2.0));
        assert!((k - 1.5 * da / h).abs() < 1e-12);
    }

    #[test]
    fn stress_concentration_limits() {
        // Tiny hole in a wide plate → K_t → 3; ellipse a = b → 3; notch a/ρ = 1 → 3.
        let h = stress_conc(KtIn {
            width_mm: 1000.0,
            hole_mm: 0.01,
            ..Default::default()
        })
        .unwrap();
        assert!((h.kt - 3.0).abs() < 1e-4);
        let e = stress_conc(KtIn {
            geometry: "ellipse".into(),
            a_mm: 1.0,
            b_mm: 1.0,
            ..Default::default()
        })
        .unwrap();
        assert!((e.kt - 3.0).abs() < 1e-12);
        let n = stress_conc(KtIn {
            geometry: "notch".into(),
            a_mm: 0.5,
            radius_mm: 0.5,
            q: 0.5,
            ..Default::default()
        })
        .unwrap();
        assert!((n.kt - 3.0).abs() < 1e-12 && (n.kf - 2.0).abs() < 1e-12);
        // Net-section nominal stress for the default hole: 100 / ((8 − 1.6) · 4).
        let d = stress_conc(KtIn::default()).unwrap();
        assert!((d.nominal_mpa - 100.0 / 25.6).abs() < 1e-12);
        assert!((d.kt - (2.0 + 0.8f64.powi(3))).abs() < 1e-12);
    }

    #[test]
    fn living_hinge_strain() {
        let o = living_hinge(HingeIn::default()).unwrap();
        let exact = 0.35 * std::f64::consts::PI / 3.0 * 100.0;
        assert!((o.strain_pct - exact).abs() < 1e-9);
    }
}
