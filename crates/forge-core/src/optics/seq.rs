//! Sequential ray tracer: real 3D rays through rotationally symmetric
//! surfaces (spheres, conics and even aspheres), exact vector Snell
//! refraction and mirror reflection, plus paraxial y–nu tracing for the
//! first-order data.
//!
//! ## Conventions (as in Zemax OpticStudio and Smith, *Modern Optical Engineering*)
//!
//! * Light travels towards +z. Surface `i` has its vertex on the axis at
//!   `z_i = Σ_{j<i} t_j`; `thickness` is the distance to the next vertex
//!   (negative after a mirror). The last surface's thickness is the distance
//!   to the **image plane**.
//! * Radius `R > 0`: centre of curvature to the right (+z). `R = 0` means flat.
//! * **Surface 0 is the aperture stop** — rays are launched across it
//!   (normalised pupil coordinates `px, py ∈ [−1, 1]`). This is the natural
//!   choice for an eyepiece or head-mounted display (HMD) modelled *reversed*,
//!   where the eye pupil is the stop (Kress 2020, ch. 5; Smith ch. 13), and
//!   for any lens with a front stop. No ray aiming is needed.
//! * The object is at `object_distance_mm` before surface 0 (`None` →
//!   infinity; negative → a *virtual* object after the stop, i.e. rays
//!   converge towards it — the virtual image of the HUD traced backwards).
//! * Field angles are in the y–z (meridional) plane.
//!
//! ## Surface sag (even asphere)
//!
//! ```text
//! z(r) = c r² / (1 + √(1 − (1+k) c² r²)) + A₄r⁴ + A₆r⁶ + A₈r⁸ + A₁₀r¹⁰
//! ```
//!
//! with curvature `c = 1/R` and conic constant `k` (0 sphere, −1 paraboloid,
//! < −1 hyperboloid, (−1, 0) prolate ellipsoid). The ray–surface intersection
//! is solved exactly for the conic (a quadratic, in its numerically stable
//! form) and refined by Newton's method for the polynomial terms.
//!
//! ## Refraction and reflection
//!
//! With unit ray direction **d**, unit normal **n** (oriented so that
//! **d·n** = cos I > 0) and μ = n₁/n₂ (Born & Wolf §3.2; Welford 1986, §4.3):
//!
//! ```text
//! refraction  d' = μ d + (√(1 − μ²(1 − cos²I)) − μ cos I) n     (TIR if the root is imaginary)
//! reflection  d' = d − 2 (d·n) n
//! ```
//!
//! ## Paraxial (y–nu) trace
//!
//! `n'u' = nu − y φ` at each surface, `φ = (n' − n) c`, and `y ← y + t u'`
//! between surfaces (Smith ch. 2; Kingslake & Johnson, *Lens Design
//! Fundamentals* 2nd ed. 2010, ch. 3). A mirror is `n' = −n`. From the
//! system matrix of the powered surfaces we get the effective focal length
//! (EFL), back and front focal lengths (BFL, FFL) and the principal planes.
//!
//! A flat fold mirror (such as the Iris 45° combiner) is modelled *unfolded*:
//! a `fold` surface is a dummy plane that only marks where the fold is.

use serde::{Deserialize, Serialize};

use super::glass;

pub type V3 = [f64; 3];

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt();
    if l > 0.0 {
        mul(a, 1.0 / l)
    } else {
        a
    }
}

/// One surface of a sequential system (a row of the Lens Data Editor).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Surface {
    pub label: String,
    /// Radius of curvature, mm (0 = flat).
    pub radius: f64,
    /// Distance to the next surface (or to the image plane for the last surface), mm.
    pub thickness: f64,
    /// Medium after the surface: `""`/`"air"`, a catalogue id (`PMMA`, `N-BK7` …),
    /// a model glass `"nd/vd"`, or `"MIRROR"`.
    pub material: String,
    /// Conic constant k.
    pub conic: f64,
    /// Even aspheric coefficients [A4, A6, A8, A10] (mm⁻³, mm⁻⁵, …).
    pub aspheric: Vec<f64>,
    /// Clear semi-diameter, mm (0 = unlimited). Rays outside are clipped.
    pub semi_diameter: f64,
    /// Non-zero: an ideal (paraxial) thin lens of this focal length, mm.
    pub paraxial_f: f64,
    /// A flat fold mirror modelled unfolded (dummy plane).
    pub fold: bool,
}

impl Default for Surface {
    fn default() -> Self {
        Surface {
            label: String::new(),
            radius: 0.0,
            thickness: 0.0,
            material: String::new(),
            conic: 0.0,
            aspheric: vec![],
            semi_diameter: 0.0,
            paraxial_f: 0.0,
            fold: false,
        }
    }
}

impl Surface {
    pub fn curvature(&self) -> f64 {
        if self.radius.abs() < 1e-12 || !self.radius.is_finite() {
            0.0
        } else {
            1.0 / self.radius
        }
    }
    pub fn is_mirror(&self) -> bool {
        self.material.eq_ignore_ascii_case("mirror")
    }
    pub fn is_ideal(&self) -> bool {
        self.paraxial_f.abs() > 1e-12
    }
    /// Sag z(r²) and g = (∂z/∂x)/x = (∂z/∂y)/y; `None` outside the conic.
    pub fn sag(&self, r2: f64) -> Option<(f64, f64)> {
        let c = self.curvature();
        let q = 1.0 - (1.0 + self.conic) * c * c * r2;
        if q < 0.0 {
            return None;
        }
        let sq = q.sqrt();
        let mut z = c * r2 / (1.0 + sq);
        let mut g = if sq > 1e-15 { c / sq } else { f64::INFINITY };
        let mut rp = r2; // r^(2m-2) for m = 2 → r²
        for (j, a) in self.aspheric.iter().enumerate() {
            let m = (j + 2) as f64; // term A r^(2m)
            z += a * rp * r2;
            g += 2.0 * m * a * rp;
            rp *= r2;
        }
        Some((z, g))
    }
}

/// A sequential optical system.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct System {
    pub name: String,
    /// Surface 0 is the stop.
    pub surfaces: Vec<Surface>,
    /// Stop (entrance pupil) diameter, mm.
    pub pupil_diameter_mm: f64,
    /// Object distance before surface 0, mm (`None` = infinity; negative = virtual object).
    pub object_distance_mm: Option<f64>,
    /// Field angles, degrees (y–z plane).
    pub fields_deg: Vec<f64>,
    /// Wavelengths, nm.
    pub wavelengths_nm: Vec<f64>,
    /// Index of the primary wavelength.
    pub primary: usize,
}

impl Default for System {
    fn default() -> Self {
        System {
            name: String::new(),
            surfaces: vec![],
            pupil_diameter_mm: 4.0,
            object_distance_mm: None,
            fields_deg: vec![0.0],
            wavelengths_nm: vec![glass::LAMBDA_F, glass::LAMBDA_D, glass::LAMBDA_C],
            primary: 1,
        }
    }
}

/// Pre-computed vertex positions and indices.
#[derive(Clone, Debug)]
pub struct Prepared {
    /// Vertex z of each surface.
    pub z: Vec<f64>,
    /// Image plane z.
    pub z_image: f64,
    /// `n[w][i]`: index of the medium after surface i (signed: negative after an odd number of mirrors).
    pub n: Vec<Vec<f64>>,
}

impl System {
    pub fn prepare(&self) -> Result<Prepared, String> {
        if self.surfaces.is_empty() {
            return Err("the system has no surfaces".into());
        }
        if self.wavelengths_nm.is_empty() {
            return Err("no wavelengths".into());
        }
        let mut z = Vec::with_capacity(self.surfaces.len());
        let mut acc = 0.0;
        for s in &self.surfaces {
            z.push(acc);
            acc += s.thickness;
        }
        // resolve every material once
        let mut mats = Vec::with_capacity(self.surfaces.len());
        for s in &self.surfaces {
            if s.is_mirror() {
                mats.push(None);
            } else {
                mats.push(Some(glass::lookup(&s.material)?));
            }
        }
        let n = self
            .wavelengths_nm
            .iter()
            .map(|&wl| {
                let mut cur = 1.0f64;
                mats.iter()
                    .map(|m| {
                        cur = match m {
                            None => -cur,
                            Some(mm) => {
                                let idx = mm.as_ref().map(|x| x.n(wl)).unwrap_or(1.0);
                                idx * cur.signum()
                            }
                        };
                        cur
                    })
                    .collect()
            })
            .collect();
        Ok(Prepared { z, z_image: acc, n })
    }

    pub fn primary(&self) -> usize {
        self.primary.min(self.wavelengths_nm.len().saturating_sub(1))
    }

    /// Launch a ray from normalised pupil coordinates at field angle θ.
    /// Returns (start point on the stop plane, unit direction, starting optical path).
    pub fn launch(&self, field_deg: f64, px: f64, py: f64) -> (V3, V3, f64) {
        let a = self.pupil_diameter_mm / 2.0;
        let p = [px * a, py * a, 0.0];
        let th = field_deg.to_radians();
        match self.object_distance_mm {
            Some(s) if s.abs() > 1e-9 && s.abs() < 1e12 => {
                let obj = [0.0, -s * th.tan(), -s];
                let d = if s > 0.0 { norm(sub(p, obj)) } else { norm(sub(obj, p)) };
                // wavefront: sphere around the object point (diverging if real, converging if virtual)
                let r = dot(sub(p, obj), sub(p, obj)).sqrt();
                (p, d, if s > 0.0 { r } else { -r })
            }
            _ => {
                let d = [0.0, th.sin(), th.cos()];
                (p, d, dot(p, d))
            }
        }
    }
}

/// Why a ray stopped.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Fail {
    Miss,
    Tir,
    Clipped,
}

/// Result of tracing one ray.
#[derive(Clone, Debug)]
pub struct RayOut {
    /// Surface index and reason when the ray failed.
    pub fail: Option<(usize, Fail)>,
    /// Intersection points (global) when recorded.
    pub pts: Vec<V3>,
    /// Last intersection point and the direction after the last surface.
    pub p: V3,
    pub d: V3,
    /// Optical path from the object reference to `p`.
    pub opl: f64,
    /// Index of the image-space medium (signed).
    pub n_img: f64,
    /// Largest height at each surface is tracked by the caller from `pts`.
    pub heights: Vec<f64>,
}

impl RayOut {
    pub fn ok(&self) -> bool {
        self.fail.is_none()
    }
    /// Point where the ray meets the plane `z` (straight-line continuation in image space).
    pub fn at_z(&self, z: f64) -> V3 {
        if self.d[2].abs() < 1e-15 {
            return self.p;
        }
        let t = (z - self.p[2]) / self.d[2];
        add(self.p, mul(self.d, t))
    }
}

/// Intersect a ray (local coordinates, vertex at the origin) with a surface.
fn intersect(s: &Surface, p: V3, d: V3) -> Option<f64> {
    if s.is_ideal() || (s.curvature() == 0.0 && s.aspheric.iter().all(|a| *a == 0.0)) {
        if d[2].abs() < 1e-15 {
            return None;
        }
        return Some(-p[2] / d[2]);
    }
    let c = s.curvature();
    let k1 = 1.0 + s.conic;
    // conic: c(x² + y² + (1+k) z²) − 2z = 0
    let a = c * (d[0] * d[0] + d[1] * d[1] + k1 * d[2] * d[2]);
    let b = 2.0 * c * (p[0] * d[0] + p[1] * d[1] + k1 * p[2] * d[2]) - 2.0 * d[2];
    let cc = c * (p[0] * p[0] + p[1] * p[1] + k1 * p[2] * p[2]) - 2.0 * p[2];
    let disc = b * b - 4.0 * a * cc;
    if disc < 0.0 {
        return None;
    }
    let den = -b - b.signum() * disc.sqrt();
    let mut t = if den.abs() < 1e-300 {
        if d[2].abs() < 1e-15 {
            return None;
        }
        -p[2] / d[2]
    } else {
        2.0 * cc / den
    };
    if s.aspheric.iter().any(|a| *a != 0.0) {
        // Newton refinement on G(t) = z − sag(x, y)
        for _ in 0..30 {
            let q = add(p, mul(d, t));
            let r2 = q[0] * q[0] + q[1] * q[1];
            let (zs, g) = s.sag(r2)?;
            let gv = q[2] - zs;
            let dg = d[2] - g * (q[0] * d[0] + q[1] * d[1]);
            if dg.abs() < 1e-15 {
                return None;
            }
            let dt = gv / dg;
            t -= dt;
            if dt.abs() < 1e-12 {
                break;
            }
        }
    }
    Some(t)
}

/// Surface normal (unit) at a local point on the surface.
fn normal(s: &Surface, q: V3) -> Option<V3> {
    if s.is_ideal() {
        return Some([0.0, 0.0, 1.0]);
    }
    let r2 = q[0] * q[0] + q[1] * q[1];
    let (_, g) = s.sag(r2)?;
    if !g.is_finite() {
        return None;
    }
    Some(norm([-q[0] * g, -q[1] * g, 1.0]))
}

/// Vector Snell's law; `None` on total internal reflection.
pub fn refract(d: V3, nrm: V3, n1: f64, n2: f64) -> Option<V3> {
    let mut nn = nrm;
    let mut ci = dot(d, nn);
    if ci < 0.0 {
        nn = mul(nn, -1.0);
        ci = -ci;
    }
    let mu = n1 / n2;
    let k = 1.0 - mu * mu * (1.0 - ci * ci);
    if k < 0.0 {
        return None;
    }
    Some(norm(add(mul(d, mu), mul(nn, k.sqrt() - mu * ci))))
}

/// Mirror reflection.
pub fn reflect(d: V3, nrm: V3) -> V3 {
    sub(d, mul(nrm, 2.0 * dot(d, nrm)))
}

/// Trace one ray (from [`System::launch`]) through every surface.
pub fn trace(sys: &System, prep: &Prepared, w: usize, p0: V3, d0: V3, opl0: f64, record: bool) -> RayOut {
    let mut p = p0;
    let mut d = d0;
    let mut opl = opl0;
    let mut n_before = 1.0f64;
    let mut pts = Vec::new();
    let mut heights = Vec::new();
    if record {
        pts.reserve(sys.surfaces.len() + 1);
    }
    let fail = |i: usize, f: Fail, p: V3, d: V3, opl: f64, pts: Vec<V3>, heights: Vec<f64>, n: f64| RayOut {
        fail: Some((i, f)),
        pts,
        p,
        d,
        opl,
        n_img: n,
        heights,
    };
    for (i, s) in sys.surfaces.iter().enumerate() {
        let zv = prep.z[i];
        let local = [p[0], p[1], p[2] - zv];
        let Some(t) = intersect(s, local, d) else {
            return fail(i, Fail::Miss, p, d, opl, pts, heights, n_before);
        };
        let q = add(local, mul(d, t));
        opl += n_before.abs() * t;
        p = [q[0], q[1], q[2] + zv];
        let h = (q[0] * q[0] + q[1] * q[1]).sqrt();
        if record {
            pts.push(p);
            heights.push(h);
        }
        if s.semi_diameter > 0.0 && h > s.semi_diameter * (1.0 + 1e-9) && i > 0 {
            return fail(i, Fail::Clipped, p, d, opl, pts, heights, n_before);
        }
        let n_after = prep.n[w][i];
        if s.is_ideal() {
            // ideal thin lens: tangent mapping tan' = tan − h/f (perfect imaging at every conjugate)
            let sg = d[2].signum();
            let tx = d[0] / d[2] - q[0] / s.paraxial_f;
            let ty = d[1] / d[2] - q[1] / s.paraxial_f;
            d = norm([tx * sg, ty * sg, sg]);
        } else if s.is_mirror() {
            let Some(nr) = normal(s, q) else {
                return fail(i, Fail::Miss, p, d, opl, pts, heights, n_before);
            };
            d = norm(reflect(d, nr));
        } else if (n_after.abs() - n_before.abs()).abs() > 1e-15 || s.curvature() != 0.0 {
            let Some(nr) = normal(s, q) else {
                return fail(i, Fail::Miss, p, d, opl, pts, heights, n_before);
            };
            match refract(d, nr, n_before.abs(), n_after.abs()) {
                Some(nd) => d = nd,
                None => return fail(i, Fail::Tir, p, d, opl, pts, heights, n_before),
            }
        }
        n_before = n_after;
    }
    RayOut {
        fail: None,
        pts,
        p,
        d,
        opl,
        n_img: n_before,
        heights,
    }
}

// ---------------------------------------------------------------------------
// Paraxial optics
// ---------------------------------------------------------------------------

/// First-order properties at one wavelength.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Paraxial {
    pub wavelength_nm: f64,
    /// Effective focal length, mm.
    pub efl: f64,
    /// Back focal length (last powered surface → rear focal point), mm.
    pub bfl: f64,
    /// Front focal length (first powered surface → front focal point, signed), mm.
    pub ffl: f64,
    /// Front / rear principal planes (global z).
    pub z_front_principal: f64,
    pub z_rear_principal: f64,
    pub z_front_focal: f64,
    pub z_rear_focal: f64,
    /// Paraxial image of the object (global z).
    pub z_paraxial_image: f64,
    /// Paraxial image height per unit tan(field) at the actual image plane.
    pub chief_height_per_tan: f64,
    /// Exit pupil (global z; `None` = at infinity) and its diameter.
    pub z_exit_pupil: Option<f64>,
    pub exit_pupil_diameter: f64,
    /// Image-space marginal ray slope (n'u') and working f-number 1/(2 n'|u'|).
    pub marginal_nu: f64,
    pub working_fnum: f64,
    /// Marginal and chief ray heights at each surface (for drawing / aberration sums).
    pub y_marginal: Vec<f64>,
    pub y_chief: Vec<f64>,
}

/// Paraxial trace: (y, ω = n u) through all surfaces from the stop, returning
/// the heights at each surface and the final (y, ω) after the last surface.
fn ynu(sys: &System, prep: &Prepared, w: usize, y0: f64, w0: f64, from: usize, to: usize) -> (Vec<f64>, f64, f64) {
    let mut y = y0;
    let mut om = w0;
    let mut ys = Vec::with_capacity(to - from + 1);
    let mut n_before = if from == 0 { 1.0 } else { prep.n[w][from - 1] };
    for i in from..=to {
        let s = &sys.surfaces[i];
        let n_after = prep.n[w][i];
        let phi = if s.is_ideal() {
            1.0 / s.paraxial_f
        } else {
            (n_after - n_before) * s.curvature()
        };
        ys.push(y);
        om -= y * phi;
        if i < to {
            y += s.thickness * om / n_after;
        }
        n_before = n_after;
    }
    (ys, y, om)
}

fn powered(s: &Surface) -> bool {
    s.is_ideal() || s.curvature() != 0.0 || s.is_mirror()
}

/// First-order data at wavelength index `w`.
pub fn paraxial(sys: &System, prep: &Prepared, w: usize) -> Paraxial {
    let ns = sys.surfaces.len();
    let last = ns - 1;
    // the optical group: surfaces with power or an index change
    let active = |i: usize| {
        let nb = if i == 0 { 1.0 } else { prep.n[w][i - 1] };
        powered(&sys.surfaces[i]) || (prep.n[w][i] - nb).abs() > 1e-12
    };
    let first_p = (0..ns).find(|&i| active(i)).unwrap_or(0);
    let last_p = (0..ns).rev().find(|&i| active(i)).unwrap_or(last);
    // system matrix of the powered group via two unit rays
    let (_, ya, oa) = ynu(sys, prep, w, 1.0, 0.0, first_p, last_p);
    let (_, yb, ob) = ynu(sys, prep, w, 0.0, 1.0, first_p, last_p);
    let (a, b, c, d) = (ya, yb, oa, ob);
    let _ = b;
    let n_img = prep.n[w][last_p];
    let efl = if c.abs() > 1e-15 { -1.0 / c } else { f64::INFINITY };
    let bfl = if c.abs() > 1e-15 { -a * n_img / c } else { f64::INFINITY };
    let ffl = if c.abs() > 1e-15 { d / c } else { f64::INFINITY };
    let z_fp = prep.z[first_p];
    let z_lp = prep.z[last_p];
    let dir = n_img.signum();
    let z_rear_focal = z_lp + bfl * dir;
    let z_rear_principal = z_rear_focal - efl * n_img;
    let z_front_focal = z_fp + ffl;
    let z_front_principal = z_front_focal + efl;

    // marginal ray from the object through the pupil edge
    let a0 = sys.pupil_diameter_mm / 2.0;
    let u0 = match sys.object_distance_mm {
        Some(s) if s.abs() > 1e-9 && s.abs() < 1e12 => a0 / s,
        _ => 0.0,
    };
    let (ym, yl_m, om_m) = ynu(sys, prep, w, a0, u0, 0, last);
    let n_last = prep.n[w][last];
    let u_img = om_m / n_last;
    let z_paraxial_image = if u_img.abs() > 1e-15 {
        prep.z[last] - yl_m / u_img
    } else {
        f64::INFINITY
    };
    // chief ray (stop at surface 0) with unit tan(field)
    let (yc, yl_c, om_c) = ynu(sys, prep, w, 0.0, 1.0, 0, last);
    let uc = om_c / n_last;
    let t_last = sys.surfaces[last].thickness;
    let chief_height_per_tan = yl_c + t_last * uc;
    let (z_exit_pupil, exit_pupil_diameter) = if uc.abs() > 1e-12 {
        let zxp = prep.z[last] - yl_c / uc;
        // exit pupil magnification: marginal height at the exit pupil plane
        let ym_xp = yl_m + (zxp - prep.z[last]) * u_img;
        (Some(zxp), 2.0 * ym_xp.abs())
    } else {
        (None, f64::INFINITY)
    };
    let marginal_nu = om_m;
    let working_fnum = if marginal_nu.abs() > 1e-12 {
        1.0 / (2.0 * marginal_nu.abs())
    } else {
        f64::INFINITY
    };
    Paraxial {
        wavelength_nm: sys.wavelengths_nm[w],
        efl,
        bfl,
        ffl,
        z_front_principal,
        z_rear_principal,
        z_front_focal,
        z_rear_focal,
        z_paraxial_image,
        chief_height_per_tan,
        z_exit_pupil,
        exit_pupil_diameter,
        marginal_nu,
        working_fnum,
        y_marginal: ym,
        y_chief: yc,
    }
}

/// Clamp non-finite numbers for JSON output.
pub fn fin(v: f64) -> f64 {
    if v.is_finite() {
        v
    } else if v > 0.0 {
        1e9
    } else {
        -1e9
    }
}

// ---------------------------------------------------------------------------
// Layout (2D meridional drawing)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct SurfaceShape {
    pub index: usize,
    pub label: String,
    pub z: f64,
    /// Profile points (z, y) from −sd to +sd.
    pub profile: Vec<[f64; 2]>,
    pub semi_diameter: f64,
    pub material: String,
    pub stop: bool,
    pub fold: bool,
    pub ideal: bool,
    pub mirror: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct FanRay {
    pub field: usize,
    pub wave: usize,
    pub py: f64,
    /// (z, y) points from the stop to the image plane (or to where it failed).
    pub pts: Vec<[f64; 2]>,
    pub fail: Option<Fail>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Layout {
    pub surfaces: Vec<SurfaceShape>,
    /// Closed element outlines (z, y) for filling glass between surfaces.
    pub elements: Vec<ElementShape>,
    pub rays: Vec<FanRay>,
    pub z_image: f64,
    pub image_half_height: f64,
    pub z_min: f64,
    pub z_max: f64,
    pub y_max: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ElementShape {
    pub from: usize,
    pub to: usize,
    pub material: String,
    pub outline: Vec<[f64; 2]>,
    /// Centre and edge thickness, mm.
    pub center_mm: f64,
    pub edge_mm: f64,
}

/// Auto semi-diameters: the clear aperture set by the user, or the largest
/// ray height reached (× 1.05) when unset.
pub fn auto_semi_diameters(sys: &System, prep: &Prepared) -> Vec<f64> {
    let mut hmax = vec![0.0f64; sys.surfaces.len()];
    let w = sys.primary();
    for &f in &sys.fields_deg {
        for py in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            let (p, d, o) = sys.launch(f, 0.0, py);
            let r = trace(sys, prep, w, p, d, o, true);
            for (k, h) in r.heights.iter().enumerate() {
                hmax[k] = hmax[k].max(*h);
            }
        }
    }
    sys.surfaces
        .iter()
        .enumerate()
        .map(|(k, s)| {
            if s.semi_diameter > 0.0 {
                s.semi_diameter
            } else if k == 0 {
                sys.pupil_diameter_mm / 2.0
            } else {
                (hmax[k] * 1.05).max(0.5)
            }
        })
        .collect()
}

/// Largest height inside the conic's domain (steep surfaces cannot exceed |R|/√(1+k)).
fn usable_h(s: &Surface, h: f64) -> f64 {
    let c = s.curvature();
    let k1 = 1.0 + s.conic;
    if c != 0.0 && k1 > 0.0 {
        let hmax = 1.0 / (c.abs() * k1.sqrt());
        h.min(hmax * 0.999)
    } else {
        h
    }
}

/// Sag at height h (0 where undefined).
pub fn sag_at(s: &Surface, h: f64) -> f64 {
    if s.is_ideal() {
        return 0.0;
    }
    s.sag(h * h).map(|x| x.0).unwrap_or(0.0)
}

/// Meridional drawing: surface profiles, element outlines and ray fans.
pub fn layout(sys: &System, prep: &Prepared, rays_per_fan: usize) -> Layout {
    let sds = auto_semi_diameters(sys, prep);
    let mut surfaces = Vec::new();
    let mut y_max = 0.0f64;
    for (i, s) in sys.surfaces.iter().enumerate() {
        let sd = usable_h(s, sds[i]);
        y_max = y_max.max(sd);
        let n = 41;
        let profile = (0..n)
            .map(|k| {
                let y = -sd + 2.0 * sd * k as f64 / (n - 1) as f64;
                [prep.z[i] + sag_at(s, y.abs()), y]
            })
            .collect();
        surfaces.push(SurfaceShape {
            index: i,
            label: s.label.clone(),
            z: prep.z[i],
            profile,
            semi_diameter: sd,
            material: s.material.clone(),
            stop: i == 0,
            fold: s.fold,
            ideal: s.is_ideal(),
            mirror: s.is_mirror(),
        });
    }
    // glass elements: consecutive surfaces with a non-air medium between them
    let mut elements = Vec::new();
    for i in 0..sys.surfaces.len().saturating_sub(1) {
        let s = &sys.surfaces[i];
        let m = s.material.trim();
        if m.is_empty() || m.eq_ignore_ascii_case("air") || s.is_mirror() {
            continue;
        }
        let h = sds[i].max(sds[i + 1]);
        let h1 = usable_h(s, h);
        let h2 = usable_h(&sys.surfaces[i + 1], h);
        let hh = h1.min(h2);
        let n = 31;
        let mut outline: Vec<[f64; 2]> = (0..n)
            .map(|k| {
                let y = -hh + 2.0 * hh * k as f64 / (n - 1) as f64;
                [prep.z[i] + sag_at(s, y.abs()), y]
            })
            .collect();
        let back: Vec<[f64; 2]> = (0..n)
            .rev()
            .map(|k| {
                let y = -hh + 2.0 * hh * k as f64 / (n - 1) as f64;
                [prep.z[i + 1] + sag_at(&sys.surfaces[i + 1], y.abs()), y]
            })
            .collect();
        outline.extend(back);
        let s2 = &sys.surfaces[i + 1];
        elements.push(ElementShape {
            from: i,
            to: i + 1,
            material: m.to_string(),
            outline,
            center_mm: s.thickness,
            edge_mm: s.thickness + sag_at(s2, hh) - sag_at(s, hh),
        });
    }
    // ray fans in the y–z plane
    let mut rays = Vec::new();
    let nr = rays_per_fan.max(2);
    let mut img_h = 0.0f64;
    for (fi, &f) in sys.fields_deg.iter().enumerate() {
        for w in 0..sys.wavelengths_nm.len() {
            for k in 0..nr {
                let py = -1.0 + 2.0 * k as f64 / (nr - 1) as f64;
                let (p, d, o) = sys.launch(f, 0.0, py);
                let r = trace(sys, prep, w, p, d, o, true);
                let mut pts: Vec<[f64; 2]> = vec![[p[2], p[1]]];
                // pts[0] is the stop hit (same as the launch point)
                pts.extend(r.pts.iter().skip(1).map(|q| [q[2], q[1]]));
                if r.ok() {
                    let q = r.at_z(prep.z_image);
                    pts.push([q[2], q[1]]);
                    img_h = img_h.max(q[1].abs());
                }
                rays.push(FanRay {
                    field: fi,
                    wave: w,
                    py,
                    pts,
                    fail: r.fail.map(|x| x.1),
                });
            }
        }
    }
    let z_min = prep.z.iter().cloned().fold(0.0, f64::min);
    let z_max = prep.z.iter().cloned().fold(prep.z_image, f64::max);
    Layout {
        surfaces,
        elements,
        rays,
        z_image: prep.z_image,
        image_half_height: img_h,
        z_min,
        z_max,
        y_max: y_max.max(img_h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn singlet(r1: f64, r2: f64, t: f64, mat: &str, back: f64) -> System {
        System {
            surfaces: vec![
                Surface {
                    label: "stop".into(),
                    thickness: 5.0,
                    ..Default::default()
                },
                Surface {
                    radius: r1,
                    thickness: t,
                    material: mat.into(),
                    ..Default::default()
                },
                Surface {
                    radius: r2,
                    thickness: back,
                    ..Default::default()
                },
            ],
            pupil_diameter_mm: 2.0,
            fields_deg: vec![0.0, 5.0],
            ..Default::default()
        }
    }

    #[test]
    fn thick_lens_focal_length_matches_lensmaker() {
        // Lensmaker (thick): φ = (n−1)(c1 − c2) + (n−1)² t c1 c2 / n
        let sys = singlet(50.0, -50.0, 5.0, "N-BK7", 45.0);
        let prep = sys.prepare().unwrap();
        let px = paraxial(&sys, &prep, 1);
        let n = glass::index_of("N-BK7", glass::LAMBDA_D).unwrap();
        let (c1, c2, t) = (1.0 / 50.0, -1.0 / 50.0, 5.0);
        let phi = (n - 1.0) * (c1 - c2) + (n - 1.0).powi(2) * t * c1 * c2 / n;
        assert!((px.efl - 1.0 / phi).abs() < 1e-9, "{} vs {}", px.efl, 1.0 / phi);
        // BFL = f (1 − (n−1) t c1 / n)
        let bfl = (1.0 / phi) * (1.0 - (n - 1.0) * t * c1 / n);
        assert!((px.bfl - bfl).abs() < 1e-9);
        // principal planes are separated by roughly t(n−1)/n inside a biconvex lens
        assert!(px.z_front_principal > 5.0 && px.z_rear_principal < 10.0);
        assert!(px.z_rear_principal > px.z_front_principal);
    }

    #[test]
    fn plano_convex_efl_is_r_over_n_minus_1() {
        let n = glass::index_of("PMMA", glass::LAMBDA_D).unwrap();
        let r = (n - 1.0) * 20.0;
        let sys = singlet(r, 0.0, 6.0, "PMMA", 15.0);
        let prep = sys.prepare().unwrap();
        let px = paraxial(&sys, &prep, 1);
        assert!((px.efl - 20.0).abs() < 1e-9, "{}", px.efl);
        assert!((px.bfl - (20.0 - 6.0 / n)).abs() < 1e-9);
    }

    #[test]
    fn real_rays_obey_snell_and_converge_near_paraxial_focus() {
        let sys = singlet(50.0, -50.0, 5.0, "N-BK7", 45.0);
        let prep = sys.prepare().unwrap();
        let px = paraxial(&sys, &prep, 1);
        // a near-axial ray crosses the axis at the paraxial focus
        let (p, d, o) = sys.launch(0.0, 0.0, 0.01);
        let r = trace(&sys, &prep, 1, p, d, o, true);
        assert!(r.ok());
        let zc = r.p[2] - r.p[1] * r.d[2] / r.d[1];
        assert!((zc - px.z_rear_focal).abs() < 1e-3, "{zc} vs {}", px.z_rear_focal);
        // a marginal ray focuses shorter (undercorrected spherical aberration of a positive lens)
        let (p, d, o) = sys.launch(0.0, 0.0, 1.0);
        let r = trace(&sys, &prep, 1, p, d, o, false);
        let zm = r.p[2] - r.p[1] * r.d[2] / r.d[1];
        assert!(zm < zc);
        // Snell's law at a flat interface
        let d0 = norm([0.0, 30f64.to_radians().sin(), 30f64.to_radians().cos()]);
        let d1 = refract(d0, [0.0, 0.0, 1.0], 1.0, 1.5).unwrap();
        assert!((d1[1] - 30f64.to_radians().sin() / 1.5).abs() < 1e-12);
    }

    #[test]
    fn total_internal_reflection_and_clipping_are_detected() {
        // a steep flat-to-air exit at 60° inside glass: TIR
        let d0 = norm([0.0, 60f64.to_radians().sin(), 60f64.to_radians().cos()]);
        assert!(refract(d0, [0.0, 0.0, 1.0], 1.5, 1.0).is_none());
        // a wide beam at a strong field through a steep lens clipped by the aperture
        let mut sys = singlet(10.0, -10.0, 4.0, "N-BK7", 10.0);
        sys.surfaces[1].semi_diameter = 1.0;
        sys.pupil_diameter_mm = 4.0;
        let prep = sys.prepare().unwrap();
        let (p, d, o) = sys.launch(0.0, 0.0, 1.0);
        let r = trace(&sys, &prep, 1, p, d, o, false);
        assert_eq!(r.fail.map(|f| f.1), Some(Fail::Clipped));
    }

    #[test]
    fn parabolic_mirror_is_perfect_on_axis() {
        // concave paraboloid R = −100 (f = 50) facing the incoming light
        let sys = System {
            surfaces: vec![
                Surface {
                    thickness: 60.0,
                    ..Default::default()
                },
                Surface {
                    radius: -100.0,
                    conic: -1.0,
                    material: "MIRROR".into(),
                    thickness: -50.0,
                    ..Default::default()
                },
            ],
            pupil_diameter_mm: 40.0,
            ..Default::default()
        };
        let prep = sys.prepare().unwrap();
        let px = paraxial(&sys, &prep, 1);
        assert!((px.efl.abs() - 50.0).abs() < 1e-9, "{}", px.efl);
        for py in [0.2, 0.6, 1.0] {
            let (p, d, o) = sys.launch(0.0, 0.0, py);
            let r = trace(&sys, &prep, 1, p, d, o, false);
            assert!(r.ok());
            let q = r.at_z(prep.z_image);
            assert!(q[1].abs() < 1e-9, "{py}: {}", q[1]);
        }
    }

    #[test]
    fn ideal_lens_images_perfectly_at_finite_conjugates() {
        let sys = System {
            surfaces: vec![
                Surface {
                    thickness: 10.0,
                    ..Default::default()
                },
                Surface {
                    paraxial_f: 20.0,
                    thickness: 30.0,
                    ..Default::default()
                },
            ],
            object_distance_mm: Some(50.0), // object 60 mm before the lens → image at 30 mm
            pupil_diameter_mm: 6.0,
            fields_deg: vec![3.0],
            ..Default::default()
        };
        let prep = sys.prepare().unwrap();
        let mut ys = vec![];
        for py in [-1.0, 0.0, 0.7, 1.0] {
            let (p, d, o) = sys.launch(3.0, 0.3, py);
            let r = trace(&sys, &prep, 1, p, d, o, false);
            ys.push(r.at_z(prep.z_image));
        }
        for q in &ys {
            assert!((q[1] - ys[0][1]).abs() < 1e-9 && (q[0] - ys[0][0]).abs() < 1e-9);
        }
    }

    #[test]
    fn aspheric_intersection_lies_on_the_surface() {
        let s = Surface {
            radius: 12.0,
            conic: -0.8,
            aspheric: vec![-2e-4, 3e-6],
            ..Default::default()
        };
        let p = [0.3, 2.5, -3.0];
        let d = norm([0.05, 0.1, 1.0]);
        let t = intersect(&s, p, d).unwrap();
        let q = add(p, mul(d, t));
        let (z, _) = s.sag(q[0] * q[0] + q[1] * q[1]).unwrap();
        assert!((q[2] - z).abs() < 1e-10);
    }
}
