//! Involute spur gears generated the way a hob or rack cutter makes them.
//!
//! **Generation.** The gear rolls on the rack's generating line (the pitch
//! circle `r = m z / 2` rolls without slip on a line shifted `x m` from the
//! rack datum). The cutter is the ISO 53 basic rack: pressure angle α,
//! addendum `h_fP = 1.25 m` (it cuts the gear's dedendum) with a tip radius
//! `ρ_fP = 0.38 m`. For a point on the circle of radius ρ, writing
//! `u = γ − φ` for its angle in the moving rack frame, the right-hand edge of
//! the tooth space it can reach is
//!
//! ```text
//! β(ρ) = max_u [ u + (w(ρ cos u) − ρ sin u) / r ]
//! ```
//!
//! where `w(Y)` is the cutter tooth's half-width at height Y (straight flank
//! `π m/4 + (Y − Y_datum) tan α`, circular tip rounding below the tangency
//! point). The tooth's half-angle is `ψ(ρ) = π/z − β(ρ)`. On the straight
//! flank this reproduces the involute exactly —
//! `ψ(ρ) = π/(2z) + 2 x tan α / z + inv α − inv α_ρ`, `cos α_ρ = r_b/ρ` —
//! and below it gives the true trochoidal root fillet, including undercut
//! when `z < 2(1 − x)/sin²α` (Litvin & Fuentes, *Gear Geometry and Applied
//! Theory*, ch. 10–11).
//!
//! **Meshing.** Working pressure angle `inv α_w = inv α + 2 tan α (x₁+x₂)/(z₁+z₂)`,
//! centre distance `a_w = a cos α / cos α_w`, contact ratio
//! `ε_α = [√(r_a1² − r_b1²) + √(r_a2² − r_b2²) − a_w sin α_w] / (π m cos α)`.
//!
//! **Strength.** Lewis bending `σ = K_v W_t / (b m Y)` with the form factor
//! found from the generated tooth (the Lewis parabola tangent to the fillet,
//! load at the tip, `Y = 2x/(3m)`) and Barth's velocity factor
//! `K_v = (6.1 + V)/6.1` (V in m/s; Shigley eq. 14-4 also lists the hobbed,
//! shaved/ground and cast-iron forms). The full AGMA 2001 / ISO 6336 methods
//! add overload, size, load-distribution, rim, geometry (J) and life factors,
//! and a separate pitting (contact stress) check — see the guide.
//!
//! Studied for comparison: the BOSL2 OpenSCAD library (`gears.scad`) builds
//! the involute from a lookup table and carves undercut with a sharp rack
//! corner, then rounds the root; here the envelope of the rounded ISO 53
//! rack gives the fillet directly.

use serde::{Deserialize, Serialize};

use std::f64::consts::PI;

const DEG: f64 = PI / 180.0;

pub fn inv(a: f64) -> f64 {
    a.tan() - a
}

/// Solve inv(α) = v for α (Newton; α ∈ (0, π/2)).
pub fn inv_inverse(v: f64) -> f64 {
    let mut a = (3.0 * v).cbrt().clamp(1e-6, 1.5);
    for _ in 0..60 {
        let f = inv(a) - v;
        let d = a.tan().powi(2);
        let step = f / d.max(1e-12);
        a -= step;
        a = a.clamp(1e-9, 1.55);
        if step.abs() < 1e-15 {
            break;
        }
    }
    a
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct GearIn {
    pub module_mm: f64,
    pub teeth: u32,
    pub pressure_deg: f64,
    /// Profile shift coefficient x.
    pub shift: f64,
    /// Addendum and dedendum coefficients (× m) and the rack tip radius coefficient ρ_fP/m.
    pub addendum: f64,
    pub dedendum: f64,
    pub root_radius: f64,
    /// Tip shortening coefficient k (tip radius reduced by k m).
    pub tip_shorten: f64,
    /// Circular thinning of the tooth at the pitch circle (backlash allowance), mm.
    pub thinning_mm: f64,
    /// Radial samples along the flank.
    pub samples: usize,
}

impl Default for GearIn {
    fn default() -> Self {
        GearIn {
            module_mm: 2.0,
            teeth: 20,
            pressure_deg: 20.0,
            shift: 0.0,
            addendum: 1.0,
            dedendum: 1.25,
            root_radius: 0.38,
            tip_shorten: 0.0,
            thinning_mm: 0.0,
            samples: 90,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct GearProfile {
    pub teeth: u32,
    pub module_mm: f64,
    pub pressure_deg: f64,
    pub shift: f64,
    pub pitch_r: f64,
    pub base_r: f64,
    pub tip_r: f64,
    pub root_r: f64,
    /// Radius where the true involute starts (form circle).
    pub form_r: f64,
    /// Rack tip radius actually used (clamped to what fits), mm.
    pub rack_tip_r: f64,
    pub undercut: bool,
    pub pointed: bool,
    pub tip_thickness_mm: f64,
    pub pitch_thickness_mm: f64,
    pub base_pitch_mm: f64,
    /// Fewest teeth without undercut at this shift, and the least shift for this tooth count.
    pub z_min: f64,
    pub x_min: f64,
    /// One flank: radius (mm) and tooth half-angle ψ (rad) from the tooth centre line.
    pub flank_r: Vec<f64>,
    pub flank_psi: Vec<f64>,
    /// Index in `flank_*` where the involute begins.
    pub form_index: usize,
    /// Lewis form factor from this profile (load at the tip) and the critical section.
    pub lewis_y: f64,
    pub lewis_section_r: f64,
    pub lewis_thickness_mm: f64,
    pub lewis_height_mm: f64,
    pub notes: Vec<String>,
}

struct Rack {
    m: f64,
    tan_a: f64,
    y_datum: f64,
    y_tip: f64,
    rho: f64,
    xc: f64,
    yc: f64,
    y_t: f64,
}

impl Rack {
    fn w(&self, y: f64) -> f64 {
        if y >= self.y_t || self.rho <= 0.0 {
            PI * self.m / 4.0 + (y - self.y_datum) * self.tan_a
        } else {
            let dy = y - self.yc;
            self.xc + (self.rho * self.rho - dy * dy).max(0.0).sqrt()
        }
    }
}

fn space_half_angle(rack: &Rack, r: f64, rho: f64) -> f64 {
    let umax = (rack.y_tip / rho).clamp(-1.0, 1.0).acos();
    let f = |u: f64| u + (rack.w(rho * u.cos()) - rho * u.sin()) / r;
    if umax <= 1e-12 {
        return f(0.0);
    }
    let n = 240;
    let mut best = (f64::NEG_INFINITY, 0.0);
    for k in 0..=n {
        let u = -umax + 2.0 * umax * k as f64 / n as f64;
        let v = f(u);
        if v > best.0 {
            best = (v, u);
        }
    }
    // golden-section refinement around the best sample
    let h = 2.0 * umax / n as f64;
    let (mut lo, mut hi) = ((best.1 - h).max(-umax), (best.1 + h).min(umax));
    let g = 0.618_033_988_75;
    let mut c = hi - g * (hi - lo);
    let mut d = lo + g * (hi - lo);
    let (mut fc, mut fd) = (f(c), f(d));
    for _ in 0..80 {
        if fc > fd {
            hi = d;
            d = c;
            fd = fc;
            c = hi - g * (hi - lo);
            fc = f(c);
        } else {
            lo = c;
            c = d;
            fc = fd;
            d = lo + g * (hi - lo);
            fd = f(d);
        }
    }
    best.0.max(fc).max(fd)
}

/// Generate one tooth flank of a spur gear.
pub fn profile(i: &GearIn) -> Result<GearProfile, String> {
    let m = i.module_mm;
    let z = i.teeth;
    if m.is_nan() || m <= 0.0 {
        return Err("module must be positive".into());
    }
    if !(4..=400).contains(&z) {
        return Err("tooth count must be 4–400".into());
    }
    let al = i.pressure_deg.clamp(10.0, 35.0) * DEG;
    let zf = z as f64;
    let r = m * zf / 2.0;
    let rb = r * al.cos();
    let x = i.shift;
    let rf = r + (x - i.dedendum) * m;
    let ra = r + (x + i.addendum - i.tip_shorten) * m;
    if rf <= 0.0 {
        return Err("root circle is at or below the centre — fewer teeth need a positive shift".into());
    }
    let tan_a = al.tan();
    let y_datum = r + x * m;
    let y_tip = rf;
    // rack tip radius, clamped so the rounding fits the rack tooth tip
    let rho_max = (PI * m / 4.0 - i.dedendum * m * tan_a) / (1.0 / al.cos() - tan_a);
    let rho = (i.root_radius * m).clamp(0.0, rho_max.max(0.0));
    let yc = y_tip + rho;
    let xc = PI * m / 4.0 + (yc - y_datum) * tan_a - rho / al.cos();
    let y_t = yc - rho * al.sin();
    let rack = Rack {
        m,
        tan_a,
        y_datum,
        y_tip,
        rho,
        xc,
        yc,
        y_t,
    };
    // Involute starts where the rack's straight flank ends (Y = y_t).
    let hs = r - y_t; // below the rolling line, toward the centre
    let s_loa = r * al.sin() - hs / al.sin();
    let undercut = s_loa < 0.0;
    let form_r = if undercut { rb } else { (rb * rb + s_loa * s_loa).sqrt() };
    let thin = i.thinning_mm / (2.0 * r);
    let n = i.samples.clamp(20, 400);
    // radial samples clustered toward the root
    let mut radii: Vec<f64> = (0..=n)
        .map(|k| {
            let t = k as f64 / n as f64;
            rf + (ra - rf) * (0.55 * t * t + 0.45 * t)
        })
        .collect();
    if form_r > rf && form_r < ra {
        radii.push(form_r);
    }
    radii.sort_by(|a, b| a.total_cmp(b));
    radii.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    let mut fr = Vec::with_capacity(radii.len());
    let mut fpsi = Vec::with_capacity(radii.len());
    let mut pointed = false;
    for &rho_k in &radii {
        let psi = PI / zf - space_half_angle(&rack, r, rho_k) - thin;
        if psi <= 0.0 {
            if fr.is_empty() {
                return Err("undercut or thinning consumes the whole tooth — use more teeth or a positive profile shift".into());
            }
            // pointed tooth: close at the radius where ψ = 0
            let (r0, p0) = (*fr.last().unwrap(), *fpsi.last().unwrap());
            let t = p0 / (p0 - psi);
            fr.push(r0 + t * (rho_k - r0));
            fpsi.push(0.0);
            pointed = true;
            break;
        }
        fr.push(rho_k);
        fpsi.push(psi);
    }
    let form_index = fr.iter().position(|v| *v >= form_r - 1e-9).unwrap_or(0);
    let tip_psi = *fpsi.last().unwrap();
    let tip_r_eff = *fr.last().unwrap();
    let psi_pitch = crate::math::interp(&fr, &fpsi, r);
    let z_min = 2.0 * (1.0 - x) / al.sin().powi(2);
    let x_min = 1.0 - zf * al.sin().powi(2) / 2.0;
    // Lewis form factor from the generated tooth, load at the tip.
    let (ly, lr, lt, lh) = lewis_from_profile(&fr, &fpsi, rb, m);
    let mut notes = vec![];
    if undercut {
        notes.push(format!(
            "Undercut: {z} teeth at x = {x:.2} is below z_min = {z_min:.1}. The cutter removes part of the involute near the base circle, weakening the tooth and shortening contact — use x ≥ {x_min:.2}."
        ));
    }
    if pointed {
        notes.push("Pointed tooth: the tip has zero thickness (too much positive shift) — shorten the tip.".into());
    } else if 2.0 * tip_r_eff * tip_psi < 0.25 * m {
        notes.push(format!(
            "Thin tip: s_a = {:.3} mm < 0.25 m. Common guidance is s_a ≥ 0.25–0.4 m.",
            2.0 * tip_r_eff * tip_psi
        ));
    }
    if rho < i.root_radius * m - 1e-9 {
        notes.push(format!("Rack tip radius clamped to {rho:.3} mm (the largest that fits)."));
    }
    Ok(GearProfile {
        teeth: z,
        module_mm: m,
        pressure_deg: al / DEG,
        shift: x,
        pitch_r: r,
        base_r: rb,
        tip_r: tip_r_eff,
        root_r: rf,
        form_r,
        rack_tip_r: rho,
        undercut,
        pointed,
        tip_thickness_mm: 2.0 * tip_r_eff * tip_psi,
        pitch_thickness_mm: 2.0 * r * psi_pitch,
        base_pitch_mm: PI * m * al.cos(),
        z_min,
        x_min,
        flank_r: fr,
        flank_psi: fpsi,
        form_index,
        lewis_y: ly,
        lewis_section_r: lr,
        lewis_thickness_mm: lt,
        lewis_height_mm: lh,
        notes,
    })
}

/// Lewis parabola on a generated tooth (centre line = +Y, right flank at +ψ).
fn lewis_from_profile(fr: &[f64], fpsi: &[f64], rb: f64, m: f64) -> (f64, f64, f64, f64) {
    let ra = *fr.last().unwrap();
    let psi_a = *fpsi.last().unwrap();
    let cos_aa = (rb / ra).clamp(-1.0, 1.0);
    let al_a = cos_aa.acos();
    // load line: from the tip point along the normal (tangent to the base circle)
    let p = [ra * psi_a.sin(), ra * psi_a.cos()];
    let tang = psi_a - al_a;
    let t = [rb * tang.sin(), rb * tang.cos()];
    let dir = [t[0] - p[0], t[1] - p[1]];
    if dir[0].abs() < 1e-12 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let s = -p[0] / dir[0];
    let h_l = p[1] + s * dir[1];
    let mut best = (f64::INFINITY, 0.0, 0.0, 0.0);
    for (k, &rho) in fr.iter().enumerate() {
        let psi = fpsi[k];
        let y = rho * psi.cos();
        let l = h_l - y;
        if l <= 1e-9 || rho > ra * 0.999 {
            continue;
        }
        let th = 2.0 * rho * psi.sin();
        let xx = th * th / (4.0 * l);
        if xx < best.0 {
            best = (xx, rho, th, l);
        }
    }
    if !best.0.is_finite() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    (2.0 * best.0 / (3.0 * m), best.1, best.2, best.3)
}

/// Full closed outline (counter-clockwise) of the gear from one flank; tooth 0 on +X.
pub fn outline(p: &GearProfile, rotation: f64) -> Vec<[f64; 2]> {
    let z = p.teeth as usize;
    let mut pts = Vec::with_capacity(z * (2 * p.flank_r.len() + 16));
    let pitch = 2.0 * PI / z as f64;
    let n = p.flank_r.len();
    let psi_top = p.flank_psi[n - 1];
    let psi_root = p.flank_psi[0];
    let root_span = pitch - 2.0 * psi_root;
    let pol = |r: f64, a: f64| [r * a.cos(), r * a.sin()];
    for k in 0..z {
        let c = rotation + k as f64 * pitch;
        for j in 0..n {
            pts.push(pol(p.flank_r[j], c - p.flank_psi[j]));
        }
        if psi_top > 0.0 {
            let arc = 6;
            for q in 1..arc {
                let a = -psi_top + 2.0 * psi_top * q as f64 / arc as f64;
                pts.push(pol(p.flank_r[n - 1], c + a));
            }
        }
        for j in (0..n).rev() {
            if psi_top <= 0.0 && j == n - 1 {
                continue;
            }
            pts.push(pol(p.flank_r[j], c + p.flank_psi[j]));
        }
        let arc = 8;
        for q in 1..arc {
            let a = c + psi_root + root_span * q as f64 / arc as f64;
            pts.push(pol(p.root_r, a));
        }
    }
    pts
}

// ------------------------------------------------------------------ pairs

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct PairIn {
    pub module_mm: f64,
    pub z1: u32,
    pub z2: u32,
    pub pressure_deg: f64,
    pub x1: f64,
    pub x2: f64,
    pub root_radius: f64,
    /// Apply the tip shortening that restores standard tip clearance.
    pub shorten_tips: bool,
    /// Circular backlash of the pair, shared equally, mm.
    pub backlash_mm: f64,
    pub face_width_mm: f64,
    /// Pinion torque (N·m) and speed (rpm).
    pub torque_nm: f64,
    pub rpm1: f64,
    /// Barth velocity-factor form: cut | hobbed | shaved | cast
    pub finish: String,
    /// Material ids from the Structures table (bending fatigue strength used as the allowable).
    pub material1: String,
    pub material2: String,
    /// Override the allowable bending stress, MPa.
    pub allowable_mpa: Option<f64>,
    pub samples: usize,
}

impl Default for PairIn {
    fn default() -> Self {
        PairIn {
            module_mm: 1.0,
            z1: 18,
            z2: 36,
            pressure_deg: 20.0,
            x1: 0.0,
            x2: 0.0,
            root_radius: 0.38,
            shorten_tips: true,
            backlash_mm: 0.05,
            face_width_mm: 8.0,
            torque_nm: 0.3,
            rpm1: 600.0,
            finish: "hobbed".into(),
            material1: "POM".into(),
            material2: "POM".into(),
            allowable_mpa: None,
            samples: 90,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct PairOut {
    pub g1: GearProfile,
    pub g2: GearProfile,
    pub ratio: f64,
    pub std_center_mm: f64,
    pub center_mm: f64,
    pub working_pressure_deg: f64,
    pub tip_shortening: f64,
    pub contact_ratio: f64,
    /// Path of contact along the line of action (gear 1 at the origin, gear 2 on +X): start, end, tangent points.
    pub loa_start: [f64; 2],
    pub loa_end: [f64; 2],
    pub t1: [f64; 2],
    pub t2: [f64; 2],
    pub interference: bool,
    pub pitch_velocity_m_s: f64,
    pub tangential_load_n: f64,
    pub kv: f64,
    pub kv_all: Vec<(String, f64)>,
    /// Shigley Table 14-2 value (20° full depth, x = 0) for comparison, when applicable.
    pub lewis_y_table1: Option<f64>,
    pub lewis_y_table2: Option<f64>,
    pub sigma1_mpa: f64,
    pub sigma2_mpa: f64,
    pub allowable1_mpa: f64,
    pub allowable2_mpa: f64,
    pub sf1: f64,
    pub sf2: f64,
    pub notes: Vec<String>,
}

/// Shigley Table 14-2: Lewis form factor Y, 20° full depth, load at the tip.
pub const LEWIS_TABLE: &[(f64, f64)] = &[
    (12.0, 0.245),
    (13.0, 0.261),
    (14.0, 0.277),
    (15.0, 0.290),
    (16.0, 0.296),
    (17.0, 0.303),
    (18.0, 0.309),
    (19.0, 0.314),
    (20.0, 0.322),
    (21.0, 0.328),
    (22.0, 0.331),
    (24.0, 0.337),
    (26.0, 0.346),
    (28.0, 0.353),
    (30.0, 0.359),
    (34.0, 0.371),
    (38.0, 0.384),
    (43.0, 0.397),
    (50.0, 0.409),
    (60.0, 0.422),
    (75.0, 0.435),
    (100.0, 0.447),
    (150.0, 0.460),
    (300.0, 0.472),
    (400.0, 0.480),
];

pub fn lewis_table(z: f64) -> Option<f64> {
    if !(12.0..=400.0).contains(&z) {
        return None;
    }
    let xs: Vec<f64> = LEWIS_TABLE.iter().map(|p| p.0).collect();
    let ys: Vec<f64> = LEWIS_TABLE.iter().map(|p| p.1).collect();
    Some(crate::math::interp(&xs, &ys, z))
}

/// Velocity factors (Shigley eq. 14-4…14-6 in SI; V in m/s).
pub fn velocity_factors(v: f64) -> Vec<(String, f64)> {
    vec![
        ("cut".into(), (6.1 + v) / 6.1),
        ("hobbed".into(), (3.56 + v.sqrt()) / 3.56),
        ("shaved".into(), ((5.56 + v.sqrt()) / 5.56).sqrt()),
        ("cast".into(), (3.05 + v) / 3.05),
    ]
}

pub fn pair(i: PairIn) -> Result<PairOut, String> {
    let m = i.module_mm;
    let al = i.pressure_deg.clamp(10.0, 35.0) * DEG;
    let (z1, z2) = (i.z1 as f64, i.z2 as f64);
    let inv_w = inv(al) + 2.0 * al.tan() * (i.x1 + i.x2) / (z1 + z2);
    let alw = inv_inverse(inv_w);
    let a = m * (z1 + z2) / 2.0;
    let aw = a * al.cos() / alw.cos();
    let y = (aw - a) / m;
    let k = (i.x1 + i.x2 - y).max(0.0);
    let kk = if i.shorten_tips { k } else { 0.0 };
    let base = GearIn {
        module_mm: m,
        pressure_deg: i.pressure_deg,
        root_radius: i.root_radius,
        tip_shorten: kk,
        thinning_mm: i.backlash_mm.max(0.0) / 2.0,
        samples: i.samples,
        ..Default::default()
    };
    let g1 = profile(&GearIn {
        teeth: i.z1,
        shift: i.x1,
        ..base.clone()
    })?;
    let g2 = profile(&GearIn {
        teeth: i.z2,
        shift: i.x2,
        ..base
    })?;
    let (ra1, ra2) = (g1.tip_r, g2.tip_r);
    let (rb1, rb2) = (g1.base_r, g2.base_r);
    let l1 = (ra1 * ra1 - rb1 * rb1).max(0.0).sqrt();
    let l2 = (ra2 * ra2 - rb2 * rb2).max(0.0).sqrt();
    let lt = aw * alw.sin();
    let eps = (l1 + l2 - lt) / (PI * m * al.cos());
    // Line of action: gear 1 turns counter-clockwise driving gear 2; the line
    // through the pitch point tilted by α_w, tangent to both base circles.
    let r1w = aw * z1 / (z1 + z2);
    let pp = [r1w, 0.0];
    let dirv = [alw.sin(), alw.cos()]; // from T1 towards T2
    let t1 = [pp[0] - r1w * alw.sin() * dirv[0], pp[1] - r1w * alw.sin() * dirv[1]];
    let t2 = [t1[0] + lt * dirv[0], t1[1] + lt * dirv[1]];
    // contact starts where gear 2's tip circle cuts the line, ends at gear 1's tip circle
    let s_start = lt - l2;
    let s_end = l1;
    let at = |s: f64| [t1[0] + s * dirv[0], t1[1] + s * dirv[1]];
    let interference = s_start < 0.0 || s_end > lt;
    let d1 = 2.0 * g1.pitch_r;
    let v = PI * d1 * i.rpm1 / 60_000.0;
    let wt = 2.0 * i.torque_nm * 1000.0 / d1;
    let kvs = velocity_factors(v);
    let kv = kvs.iter().find(|(n, _)| *n == i.finish).map(|x| x.1).unwrap_or(kvs[1].1);
    let b = i.face_width_mm.max(1e-6);
    let sig = |y: f64| if y > 0.0 { kv * wt / (b * m * y) } else { f64::INFINITY };
    let s1 = sig(g1.lewis_y);
    let s2 = sig(g2.lewis_y);
    let mat1 = crate::structures::materials::get(&i.material1);
    let mat2 = crate::structures::materials::get(&i.material2);
    let allow = |mt: &crate::structures::materials::StructMaterial| {
        i.allowable_mpa
            .unwrap_or(if mt.fatigue_mpa > 0.0 { mt.fatigue_mpa } else { mt.yield_mpa / 3.0 })
    };
    let (al1, al2) = (allow(mat1), allow(mat2));
    let std = (i.pressure_deg - 20.0).abs() < 1e-9;
    let mut notes = vec![];
    if interference {
        notes.push("Interference: contact would start inside a base circle (the tip of one gear digs into the other's root). Add positive shift to the pinion or use more teeth.".into());
    }
    // active profile must stay on the true involute
    let r_start1 = (rb1 * rb1 + s_start.max(0.0).powi(2)).sqrt();
    let r_end2 = (rb2 * rb2 + (lt - s_end).max(0.0).powi(2)).sqrt();
    if r_start1 < g1.form_r - 1e-6 {
        notes.push(format!(
            "Gear 1: contact reaches r = {r_start1:.3} mm, below its form circle {:.3} mm (fillet interference).",
            g1.form_r
        ));
    }
    if r_end2 < g2.form_r - 1e-6 {
        notes.push(format!(
            "Gear 2: contact reaches r = {r_end2:.3} mm, below its form circle {:.3} mm (fillet interference).",
            g2.form_r
        ));
    }
    if eps < 1.2 {
        notes.push(format!(
            "Contact ratio {eps:.2} < 1.2: rough running (aim for ≥ 1.2, ideally ≥ 1.4)."
        ));
    }
    notes.push(
        "Lewis + Barth is a first sizing check. AGMA 2001 / ISO 6336 add overload, dynamic, size, load-distribution, rim and life factors, the geometry factor J (or Y_F·Y_S) with the fillet stress concentration, and a separate contact-stress (pitting) check."
            .into(),
    );
    if mat1.printed || mat2.printed {
        notes.push("Printed or plastic gears: strength depends on temperature, humidity and layer orientation — test at the operating speed; see the Structures material notes.".into());
    }
    Ok(PairOut {
        ratio: z2 / z1,
        std_center_mm: a,
        center_mm: aw,
        working_pressure_deg: alw / DEG,
        tip_shortening: k,
        contact_ratio: eps,
        loa_start: at(s_start),
        loa_end: at(s_end),
        t1,
        t2,
        interference,
        pitch_velocity_m_s: v,
        tangential_load_n: wt,
        kv,
        kv_all: kvs,
        lewis_y_table1: if std && i.x1 == 0.0 { lewis_table(z1) } else { None },
        lewis_y_table2: if std && i.x2 == 0.0 { lewis_table(z2) } else { None },
        sigma1_mpa: s1,
        sigma2_mpa: s2,
        allowable1_mpa: al1,
        allowable2_mpa: al2,
        sf1: (al1 / s1).min(1e6),
        sf2: (al2 / s2).min(1e6),
        notes,
        g1,
        g2,
    })
}

// ------------------------------------------------------------------ trains

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct TrainIn {
    /// Stages as (driver teeth, driven teeth).
    pub stages: Vec<[u32; 2]>,
    /// Search for a 1- or 2-stage train close to this ratio (0 = off).
    pub target_ratio: f64,
    pub min_teeth: u32,
    pub max_teeth: u32,
}

impl Default for TrainIn {
    fn default() -> Self {
        TrainIn {
            stages: vec![[12, 36], [15, 45]],
            target_ratio: 9.0,
            min_teeth: 12,
            max_teeth: 80,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TrainOut {
    pub ratio: f64,
    pub stage_ratios: Vec<f64>,
    /// Output direction relative to the input (+1 same, −1 reversed) for external meshes.
    pub direction: i32,
    pub suggestions: Vec<TrainSuggestion>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TrainSuggestion {
    pub stages: Vec<[u32; 2]>,
    pub ratio: f64,
    pub error_pct: f64,
}

pub fn train(i: TrainIn) -> Result<TrainOut, String> {
    let mut ratio = 1.0;
    let mut stage_ratios = vec![];
    for s in &i.stages {
        if s[0] == 0 || s[1] == 0 {
            return Err("tooth counts must be positive".into());
        }
        let r = s[1] as f64 / s[0] as f64;
        stage_ratios.push(r);
        ratio *= r;
    }
    let mut sug = vec![];
    let (lo, hi) = (i.min_teeth.max(6), i.max_teeth.clamp(i.min_teeth.max(6), 200));
    if i.target_ratio > 0.0 {
        let t = i.target_ratio;
        // single stage
        for a in lo..=hi.min(40) {
            let b = (a as f64 * t).round() as u32;
            if b >= lo && b <= hi {
                sug.push(TrainSuggestion {
                    stages: vec![[a, b]],
                    ratio: b as f64 / a as f64,
                    error_pct: 100.0 * (b as f64 / a as f64 - t) / t,
                });
            }
        }
        // two stages
        for a in lo..=hi.min(30) {
            for b in lo..=hi {
                let r1 = b as f64 / a as f64;
                let rest = t / r1;
                if !(0.2..=8.0).contains(&rest) {
                    continue;
                }
                for c in lo..=hi.min(30) {
                    let d = (c as f64 * rest).round() as u32;
                    if d < lo || d > hi {
                        continue;
                    }
                    let r = r1 * d as f64 / c as f64;
                    sug.push(TrainSuggestion {
                        stages: vec![[a, b], [c, d]],
                        ratio: r,
                        error_pct: 100.0 * (r - t) / t,
                    });
                }
            }
        }
        // rank: error, then fewer stages, then fewer teeth
        sug.sort_by(|x, y| {
            let tx: u32 = x.stages.iter().map(|s| s[0] + s[1]).sum();
            let ty: u32 = y.stages.iter().map(|s| s[0] + s[1]).sum();
            (x.error_pct.abs() + 0.001 * x.stages.len() as f64)
                .total_cmp(&(y.error_pct.abs() + 0.001 * y.stages.len() as f64))
                .then(tx.cmp(&ty))
        });
        sug.dedup_by(|a, b| a.stages == b.stages);
        sug.truncate(8);
    }
    Ok(TrainOut {
        ratio,
        stage_ratios,
        direction: if i.stages.len().is_multiple_of(2) { 1 } else { -1 },
        suggestions: sug,
    })
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct PlanetIn {
    pub sun: u32,
    pub ring: u32,
    pub planets: u32,
    pub module_mm: f64,
    /// Which member is held: ring | carrier | sun
    pub fixed: String,
    /// Search sun/ring for this ratio (0 = off), with the ring held, sun in, carrier out.
    pub target_ratio: f64,
}

impl Default for PlanetIn {
    fn default() -> Self {
        PlanetIn {
            sun: 18,
            ring: 72,
            planets: 3,
            module_mm: 1.0,
            fixed: "ring".into(),
            target_ratio: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct PlanetOut {
    pub planet: f64,
    pub planet_integer: bool,
    pub equal_spacing: bool,
    pub adjacency_ok: bool,
    /// Input/output speed ratio for the chosen held member (see `ratio_label`).
    pub ratio: f64,
    pub ratio_label: String,
    /// All three single-held arrangements: (held, input, output, ratio).
    pub arrangements: Vec<(String, String, String, f64)>,
    pub sun_r: f64,
    pub planet_r: f64,
    pub ring_r: f64,
    pub carrier_r: f64,
    pub suggestions: Vec<[u32; 3]>,
    pub notes: Vec<String>,
}

/// Simple planetary set (Willis: (ω_s − ω_c)/(ω_r − ω_c) = −Z_r/Z_s).
pub fn planetary(i: PlanetIn) -> Result<PlanetOut, String> {
    let (s, r, n) = (i.sun as f64, i.ring as f64, i.planets.max(1) as f64);
    if i.sun == 0 || i.ring <= i.sun {
        return Err("ring must have more teeth than the sun".into());
    }
    let p = (r - s) / 2.0;
    let m = i.module_mm;
    let ratio_ring_fixed = 1.0 + r / s; // sun in, carrier out
    let ratio_sun_fixed = 1.0 + s / r; // ring in, carrier out (reduction ring→carrier)
    let ratio_carrier_fixed = -r / s; // sun in, ring out
    let arrangements = vec![
        ("ring".into(), "sun".into(), "carrier".into(), ratio_ring_fixed),
        ("sun".into(), "ring".into(), "carrier".into(), ratio_sun_fixed),
        ("carrier".into(), "sun".into(), "ring".into(), ratio_carrier_fixed),
    ];
    let (ratio, label) = match i.fixed.as_str() {
        "sun" => (ratio_sun_fixed, "ring in → carrier out (1 + Z_s/Z_r)"),
        "carrier" => (ratio_carrier_fixed, "sun in → ring out (−Z_r/Z_s, reversed)"),
        _ => (ratio_ring_fixed, "sun in → carrier out (1 + Z_r/Z_s)"),
    };
    let equal = ((s + r) / n).fract().abs() < 1e-9;
    let adj = (s + p) * (PI / n).sin() > p + 2.0;
    let mut notes = vec![];
    if p.fract() != 0.0 {
        notes.push("Z_r − Z_s must be even for standard (unshifted) planets: planet teeth = (Z_r − Z_s)/2.".into());
    }
    if !equal {
        notes.push(format!(
            "(Z_s + Z_r)/N = {:.2} is not an integer: {} planets cannot be equally spaced.",
            (s + r) / n,
            i.planets
        ));
    }
    if !adj {
        notes.push("Neighbouring planets' tips would collide (adjacency condition): use fewer planets or a larger sun.".into());
    }
    let mut suggestions = vec![];
    if i.target_ratio > 1.0 {
        let mut cands = vec![];
        for zs in 10..=80u32 {
            for zr in (zs + 12)..=200u32 {
                if (zr - zs) % 2 != 0 || (zs + zr) % i.planets.max(1) != 0 {
                    continue;
                }
                let zp = (zr - zs) as f64 / 2.0;
                if (zs as f64 + zp) * (PI / n).sin() <= zp + 2.0 {
                    continue;
                }
                let rr = 1.0 + zr as f64 / zs as f64;
                cands.push(((rr - i.target_ratio).abs(), [zs, (zr - zs) / 2, zr]));
            }
        }
        cands.sort_by(|a, b| a.0.total_cmp(&b.0).then((a.1[2]).cmp(&b.1[2])));
        suggestions = cands.into_iter().take(6).map(|c| c.1).collect();
    }
    Ok(PlanetOut {
        planet: p,
        planet_integer: p.fract() == 0.0,
        equal_spacing: equal,
        adjacency_ok: adj,
        ratio,
        ratio_label: label.into(),
        arrangements,
        sun_r: m * s / 2.0,
        planet_r: m * p / 2.0,
        ring_r: m * r / 2.0,
        carrier_r: m * (s + p) / 2.0,
        suggestions,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_circle_and_involute_thickness() {
        for (z, x) in [(20u32, 0.0), (30, 0.3), (40, -0.2)] {
            let g = profile(&GearIn {
                module_mm: 2.0,
                teeth: z,
                shift: x,
                ..Default::default()
            })
            .unwrap();
            let al = 20.0 * DEG;
            assert!((g.pitch_r - z as f64).abs() < 1e-12);
            assert!((g.base_r - z as f64 * al.cos()).abs() < 1e-12);
            // exact involute half-angle on the flank above the form circle
            for (k, &r) in g.flank_r.iter().enumerate() {
                if r < g.form_r + 0.01 || r > g.tip_r - 1e-9 {
                    continue;
                }
                let ar = (g.base_r / r).acos();
                let exact = PI / (2.0 * z as f64) + 2.0 * x * al.tan() / z as f64 + inv(al) - inv(ar);
                assert!((g.flank_psi[k] - exact).abs() < 2e-7, "z {z} r {r}: {} vs {exact}", g.flank_psi[k]);
            }
            // tooth thickness at the pitch circle: s = m(π/2 + 2 x tan α)
            let s = 2.0 * (PI / 2.0 + 2.0 * x * al.tan());
            assert!((g.pitch_thickness_mm - s).abs() < 1e-4, "{} vs {s}", g.pitch_thickness_mm);
        }
    }

    #[test]
    fn undercut_threshold() {
        let a = profile(&GearIn {
            teeth: 14,
            ..Default::default()
        })
        .unwrap();
        assert!(a.undercut);
        let b = profile(&GearIn {
            teeth: 18,
            ..Default::default()
        })
        .unwrap();
        assert!(!b.undercut);
        let c = profile(&GearIn {
            teeth: 14,
            shift: a.x_min + 0.02,
            ..Default::default()
        })
        .unwrap();
        assert!(!c.undercut);
        assert!((a.z_min - 17.097).abs() < 0.01);
    }

    /// Two standard 20-tooth, 20° gears: ε = [2√(11² − (10 cos 20°)²) − 20 sin 20°]/(π cos 20°) ≈ 1.557 (m = 1).
    #[test]
    fn contact_ratio_standard_pair() {
        let o = pair(PairIn {
            z1: 20,
            z2: 20,
            backlash_mm: 0.0,
            ..Default::default()
        })
        .unwrap();
        let al = 20.0 * DEG;
        let rb = 10.0 * al.cos();
        let hand = (2.0 * (121.0 - rb * rb).sqrt() - 20.0 * al.sin()) / (PI * al.cos());
        assert!((o.contact_ratio - hand).abs() < 1e-9);
        assert!((o.contact_ratio - 1.557).abs() < 0.001);
        assert!((o.center_mm - 20.0).abs() < 1e-9);
        assert!(!o.interference);
    }

    #[test]
    fn shifted_pair_center_distance() {
        let o = pair(PairIn {
            z1: 12,
            z2: 30,
            x1: 0.4,
            x2: 0.0,
            ..Default::default()
        })
        .unwrap();
        // inv α_w from the shifts and a_w = a cos α / cos α_w
        assert!(o.working_pressure_deg > 20.0);
        assert!(o.center_mm > o.std_center_mm);
        assert!(o.tip_shortening >= 0.0);
        assert!((inv(inv_inverse(0.0149)) - 0.0149).abs() < 1e-14);
    }

    /// The Lewis form factor measured on the generated tooth lands near
    /// Shigley's Table 14-2 values (20° full depth, load at the tip).
    #[test]
    fn lewis_from_profile_near_table() {
        for z in [16u32, 20, 30, 50, 100] {
            let g = profile(&GearIn {
                module_mm: 1.0,
                teeth: z,
                ..Default::default()
            })
            .unwrap();
            let t = lewis_table(z as f64).unwrap();
            let rel = (g.lewis_y - t).abs() / t;
            assert!(rel < 0.10, "z {z}: Y = {} vs table {t}", g.lewis_y);
        }
    }

    #[test]
    fn trains_and_planetary() {
        let t = train(TrainIn::default()).unwrap();
        assert!((t.ratio - 9.0).abs() < 1e-12);
        assert!(t.suggestions.first().map(|s| s.error_pct.abs() < 1e-9).unwrap_or(false));
        let p = planetary(PlanetIn::default()).unwrap();
        assert_eq!(p.planet, 27.0);
        assert!((p.ratio - 5.0).abs() < 1e-12);
        assert!(p.equal_spacing && p.adjacency_ok);
        let q = planetary(PlanetIn {
            target_ratio: 4.0,
            ..Default::default()
        })
        .unwrap();
        assert!(!q.suggestions.is_empty());
    }

    #[test]
    fn outline_is_closed_and_sized() {
        let g = profile(&GearIn::default()).unwrap();
        let o = outline(&g, 0.0);
        let rmax = o.iter().map(|p| p[0].hypot(p[1])).fold(0.0, f64::max);
        let rmin = o.iter().map(|p| p[0].hypot(p[1])).fold(f64::INFINITY, f64::min);
        assert!((rmax - g.tip_r).abs() < 1e-9 && (rmin - g.root_r).abs() < 1e-9);
    }
}
