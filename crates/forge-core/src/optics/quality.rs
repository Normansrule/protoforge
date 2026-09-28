//! Image-quality analysis of a sequential system: spot diagrams and root
//! mean square (RMS) spot radius, ray fans (transverse aberration), field
//! curvature and distortion, longitudinal and lateral colour, optical path
//! difference (OPD) and wavefront RMS, and the modulation transfer function
//! (MTF).
//!
//! * **Spot diagram / RMS radius.** A hexapolar grid of real rays across the
//!   pupil at every field and wavelength; the RMS radius is taken about the
//!   polychromatic centroid (Smith, *Modern Optical Engineering* 4th ed.,
//!   ch. 11). Angular blur in object space = spot / EFL.
//! * **Best focus.** In image space every ray is a straight line
//!   `x(z) = x₀ + (z − z₀)·dx/dz`, so the summed squared spot radius is a
//!   quadratic in the image-plane position and its minimum is found in
//!   closed form (what a lens designer's "quick focus" does).
//! * **Ray fans.** Transverse ray error `e_y(p_y)` (tangential) and
//!   `e_x(p_x)` (sagittal) relative to the primary-wavelength chief ray
//!   (Welford, *Aberrations of Optical Systems*, 1986, ch. 8).
//! * **Field curvature** from close ("parabasal") real rays around the chief
//!   ray: where a tangential neighbour crosses it, and where a sagittal
//!   neighbour pierces the meridional plane (Coddington's equations, done
//!   numerically). **Distortion** = (y_real − y_paraxial)/y_paraxial.
//! * **Colour.** Longitudinal: paraxial image position F − C. Lateral:
//!   chief-ray image height F − C at each field.
//! * **OPD / wavefront.** Optical path along each ray to the reference sphere
//!   centred on the chief-ray image point and passing through the exit pupil;
//!   piston and tilt removed by least squares; Strehl ≈ exp(−(2πσ)²)
//!   (Maréchal; Born & Wolf, *Principles of Optics*, §9.1).
//! * **MTF.** The geometric MTF is the magnitude of the Fourier transform of
//!   the spot (line spread) distribution, `|⟨exp(−2πiνy)⟩|`; the
//!   diffraction limit of a circular pupil is
//!   `(2/π)(φ − cos φ sin φ)`, `φ = acos(ν/ν_c)`, `ν_c = 1/(λ·N_w)`
//!   (Goodman, *Introduction to Fourier Optics* 3rd ed., 2005, §6.3). The
//!   reported MTF is their product — a standard estimate that is good when
//!   the geometric blur dominates or is small; a full answer needs the
//!   Fourier transform of the pupil function (Zemax "FFT MTF").

use serde::{Deserialize, Serialize};

use super::seq::{fin, paraxial, trace, Fail, Paraxial, Prepared, RayOut, System};

pub const RAD_TO_ARCMIN: f64 = 60.0 * 180.0 / std::f64::consts::PI;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct QualityConfig {
    /// Hexapolar rings for spot diagrams (6 → 127 rays per field and wavelength).
    pub rings: usize,
    /// Points per ray fan.
    pub fan_points: usize,
    /// Fields for the field-curvature / distortion curves.
    pub curve_points: usize,
    /// Display Nyquist frequency in image space, cycles/mm (0 = none).
    pub nyquist_lp_mm: f64,
    /// Pixel pitch in image space, mm (for comparing spot size; 0 = none).
    pub pixel_mm: f64,
    /// Move the image plane to the best polychromatic focus first.
    pub auto_focus: bool,
    /// Skip fans, curves, OPD (fast path for metrics).
    pub lite: bool,
}

impl Default for QualityConfig {
    fn default() -> Self {
        QualityConfig {
            rings: 6,
            fan_points: 33,
            curve_points: 13,
            nyquist_lp_mm: 0.0,
            pixel_mm: 0.0,
            auto_focus: true,
            lite: false,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SpotField {
    pub field_deg: f64,
    /// `points[w]`: (x, y) relative to the polychromatic centroid, mm.
    pub points: Vec<Vec<[f64; 2]>>,
    pub centroid: [f64; 2],
    /// Polychromatic RMS radius, mm, and per wavelength.
    pub rms_mm: f64,
    pub rms_per_wave_mm: Vec<f64>,
    /// Largest distance from the centroid, mm.
    pub geo_radius_mm: f64,
    pub rms_arcmin: f64,
    pub rays: usize,
    pub failed: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct FanField {
    pub field_deg: f64,
    pub pupil: Vec<f64>,
    /// `ey[w][k]` tangential and `ex[w][k]` sagittal transverse error, mm (NaN-free: failed rays are skipped → 0 with `ok` false).
    pub ey: Vec<Vec<f64>>,
    pub ex: Vec<Vec<f64>>,
    pub ok: Vec<Vec<bool>>,
    /// Tangential OPD fan at the primary wavelength, waves.
    pub opd_y: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FieldCurves {
    pub field_deg: Vec<f64>,
    /// `tangential[w][k]`, `sagittal[w][k]`: focus position relative to the image plane, mm.
    pub tangential: Vec<Vec<f64>>,
    pub sagittal: Vec<Vec<f64>>,
    /// Distortion (primary wavelength), %.
    pub distortion_pct: Vec<f64>,
    /// Real chief-ray image height (primary), mm.
    pub image_height_mm: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Chromatic {
    /// Paraxial image position F − C, mm.
    pub longitudinal_mm: f64,
    /// Same, as a vergence change of the image seen through the lens, dioptres (Δz/EFL²).
    pub longitudinal_diopter: f64,
    /// Chief-ray image height F − C at each analysis field, mm and arcmin.
    pub lateral_mm: Vec<f64>,
    pub lateral_arcmin: Vec<f64>,
    /// Chromatic focal shift curve: paraxial image position relative to the primary, mm.
    pub shift_wavelengths_nm: Vec<f64>,
    pub shift_mm: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Wavefront {
    pub field_deg: f64,
    pub rms_waves: f64,
    pub pv_waves: f64,
    pub strehl: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct MtfField {
    pub field_deg: f64,
    pub tangential: Vec<f64>,
    pub sagittal: Vec<f64>,
    pub geo_tangential: Vec<f64>,
    pub geo_sagittal: Vec<f64>,
    pub at_nyquist_t: f64,
    pub at_nyquist_s: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Mtf {
    pub freq_lp_mm: Vec<f64>,
    pub diffraction: Vec<f64>,
    pub cutoff_lp_mm: f64,
    pub nyquist_lp_mm: f64,
    pub diffraction_at_nyquist: f64,
    pub fields: Vec<MtfField>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Quality {
    pub paraxial: Vec<Paraxial>,
    pub efl: f64,
    /// Image plane after focusing (global z) and the focus shift applied, mm.
    pub z_image: f64,
    pub focus_shift_mm: f64,
    /// Final last-surface thickness (distance to the image plane), mm.
    pub image_distance_mm: f64,
    pub spots: Vec<SpotField>,
    pub fans: Vec<FanField>,
    pub curves: Option<FieldCurves>,
    pub chromatic: Chromatic,
    pub wavefront: Vec<Wavefront>,
    pub mtf: Mtf,
    pub worst_rms_mm: f64,
    pub worst_rms_arcmin: f64,
    pub worst_mtf_nyquist: f64,
    pub worst_lateral_arcmin: f64,
    pub pixel_arcmin: f64,
    pub warnings: Vec<String>,
    pub ideal: bool,
}

/// Hexapolar pupil samples (normalised), centre first.
pub fn hexapolar(rings: usize) -> Vec<(f64, f64)> {
    let mut v = vec![(0.0, 0.0)];
    for k in 1..=rings {
        let r = k as f64 / rings as f64;
        let n = 6 * k;
        for j in 0..n {
            let a = 2.0 * std::f64::consts::PI * j as f64 / n as f64;
            v.push((r * a.sin(), r * a.cos()));
        }
    }
    v
}

fn trace_at(sys: &System, prep: &Prepared, w: usize, f: f64, px: f64, py: f64) -> RayOut {
    let (p, d, o) = sys.launch(f, px, py);
    trace(sys, prep, w, p, d, o, false)
}

/// Closed-form best focus: image-plane shift minimising the summed squared
/// spot radii (about each field's polychromatic centroid).
pub fn best_focus_shift(sys: &System, prep: &Prepared, rings: usize) -> f64 {
    let pupil = hexapolar(rings);
    let z0 = prep.z_image;
    let (mut num, mut den) = (0.0, 0.0);
    for &f in &sys.fields_deg {
        let mut rows: Vec<[f64; 4]> = Vec::new();
        for w in 0..sys.wavelengths_nm.len() {
            for &(px, py) in &pupil {
                let r = trace_at(sys, prep, w, f, px, py);
                if !r.ok() || r.d[2].abs() < 1e-12 {
                    continue;
                }
                let q = r.at_z(z0);
                rows.push([q[0], q[1], r.d[0] / r.d[2], r.d[1] / r.d[2]]);
            }
        }
        if rows.is_empty() {
            continue;
        }
        let n = rows.len() as f64;
        let mut m = [0.0; 4];
        for r in &rows {
            for k in 0..4 {
                m[k] += r[k] / n;
            }
        }
        for r in &rows {
            let (ax, ay, bx, by) = (r[0] - m[0], r[1] - m[1], r[2] - m[2], r[3] - m[3]);
            num += ax * bx + ay * by;
            den += bx * bx + by * by;
        }
    }
    if den > 0.0 {
        -num / den
    } else {
        0.0
    }
}

fn diffraction_mtf(nu: f64, cutoff: f64) -> f64 {
    if cutoff.is_nan() || cutoff <= 0.0 || nu >= cutoff {
        return 0.0;
    }
    let x = (nu / cutoff).clamp(0.0, 1.0);
    let phi = x.acos();
    (2.0 / std::f64::consts::PI) * (phi - phi.cos() * phi.sin())
}

fn geo_mtf(vals: &[f64], nu: f64) -> f64 {
    if vals.is_empty() {
        return 0.0;
    }
    let (mut re, mut im) = (0.0, 0.0);
    for v in vals {
        let a = -2.0 * std::f64::consts::PI * nu * v;
        re += a.cos();
        im += a.sin();
    }
    (re * re + im * im).sqrt() / vals.len() as f64
}

/// Focus of a close neighbour ray relative to the chief ray.
fn focus_z(chief: &RayOut, nb: &RayOut, tangential: bool) -> Option<f64> {
    if tangential {
        // intersect the two rays in the y–z plane
        let (p1, d1, p2, d2) = (chief.p, chief.d, nb.p, nb.d);
        let det = d1[2] * (-d2[1]) - d1[1] * (-d2[2]);
        if det.abs() < 1e-18 {
            return None;
        }
        let rz = p2[2] - p1[2];
        let ry = p2[1] - p1[1];
        let s = (rz * (-d2[1]) - ry * (-d2[2])) / det;
        Some(p1[2] + s * d1[2])
    } else {
        if nb.d[0].abs() < 1e-18 {
            return None;
        }
        let s = -nb.p[0] / nb.d[0];
        Some(nb.p[2] + s * nb.d[2])
    }
}

/// Full image-quality analysis.
pub fn analyze(sys0: &System, cfg: &QualityConfig) -> Result<Quality, String> {
    let mut sys = sys0.clone();
    let mut prep = sys.prepare()?;
    let mut warnings = Vec::new();
    let pw = sys.primary();
    let nw = sys.wavelengths_nm.len();
    let rings = cfg.rings.clamp(1, 12);
    let mut shift = 0.0;
    if cfg.auto_focus {
        shift = best_focus_shift(&sys, &prep, rings.min(5));
        if shift.is_finite() {
            let last = sys.surfaces.len() - 1;
            sys.surfaces[last].thickness += shift;
            prep = sys.prepare()?;
        } else {
            shift = 0.0;
        }
    }
    let z_img = prep.z_image;
    let ideal = sys.surfaces.iter().any(|s| s.is_ideal());
    let parax: Vec<Paraxial> = (0..nw).map(|w| paraxial(&sys, &prep, w)).collect();
    let efl = parax[pw].efl;
    let to_arcmin = |mm: f64| {
        if efl.is_finite() && efl.abs() > 1e-9 {
            mm / efl.abs() * RAD_TO_ARCMIN
        } else {
            0.0
        }
    };

    // ---- spot diagrams ----------------------------------------------------
    let pupil = hexapolar(rings);
    let mut spots = Vec::new();
    let mut spot_raw: Vec<Vec<[f64; 2]>> = Vec::new(); // all wavelengths, relative to centroid
    let mut tir = 0usize;
    let mut clipped = 0usize;
    for &f in &sys.fields_deg {
        let mut per_w: Vec<Vec<[f64; 2]>> = vec![vec![]; nw];
        let mut failed = 0;
        for (w, pts) in per_w.iter_mut().enumerate() {
            for &(px, py) in &pupil {
                let r = trace_at(&sys, &prep, w, f, px, py);
                match r.fail {
                    None => {
                        let q = r.at_z(z_img);
                        pts.push([q[0], q[1]]);
                    }
                    Some((_, e)) => {
                        failed += 1;
                        match e {
                            Fail::Tir => tir += 1,
                            Fail::Clipped => clipped += 1,
                            Fail::Miss => {}
                        }
                    }
                }
            }
        }
        let all: Vec<[f64; 2]> = per_w.iter().flatten().copied().collect();
        let n = all.len().max(1) as f64;
        let cx = all.iter().map(|p| p[0]).sum::<f64>() / n;
        let cy = all.iter().map(|p| p[1]).sum::<f64>() / n;
        let rel: Vec<Vec<[f64; 2]>> = per_w.iter().map(|v| v.iter().map(|p| [p[0] - cx, p[1] - cy]).collect()).collect();
        let rms = |v: &[[f64; 2]]| {
            if v.is_empty() {
                0.0
            } else {
                (v.iter().map(|p| p[0] * p[0] + p[1] * p[1]).sum::<f64>() / v.len() as f64).sqrt()
            }
        };
        let flat: Vec<[f64; 2]> = rel.iter().flatten().copied().collect();
        let rms_mm = rms(&flat);
        // per-wavelength RMS about that wavelength's own centroid
        let rms_per_wave_mm = per_w
            .iter()
            .map(|v| {
                if v.is_empty() {
                    return 0.0;
                }
                let m = v.len() as f64;
                let (mx, my) = (v.iter().map(|p| p[0]).sum::<f64>() / m, v.iter().map(|p| p[1]).sum::<f64>() / m);
                (v.iter().map(|p| (p[0] - mx).powi(2) + (p[1] - my).powi(2)).sum::<f64>() / m).sqrt()
            })
            .collect();
        let geo = flat.iter().map(|p| (p[0] * p[0] + p[1] * p[1]).sqrt()).fold(0.0, f64::max);
        spots.push(SpotField {
            field_deg: f,
            rays: pupil.len() * nw,
            failed,
            centroid: [cx, cy],
            rms_mm,
            rms_per_wave_mm,
            geo_radius_mm: geo,
            rms_arcmin: to_arcmin(rms_mm),
            points: rel,
        });
        spot_raw.push(flat);
    }
    if tir > 0 {
        warnings.push(format!("{tir} rays hit total internal reflection (steep surfaces)"));
    }
    if clipped > 0 {
        warnings.push(format!("{clipped} rays clipped by a clear aperture (vignetting)"));
    }

    // ---- chromatic ----------------------------------------------------------
    let (iw_f, iw_c) = {
        // shortest and longest wavelengths play F and C
        let mut idx: Vec<usize> = (0..nw).collect();
        idx.sort_by(|a, b| sys.wavelengths_nm[*a].partial_cmp(&sys.wavelengths_nm[*b]).unwrap());
        (idx[0], idx[nw - 1])
    };
    let long_mm = parax[iw_f].z_paraxial_image - parax[iw_c].z_paraxial_image;
    let long_d = if efl.abs() > 1e-9 { long_mm / (efl * efl) * 1000.0 } else { 0.0 };
    let mut lat_mm = Vec::new();
    for &f in &sys.fields_deg {
        let a = trace_at(&sys, &prep, iw_f, f, 0.0, 0.0);
        let b = trace_at(&sys, &prep, iw_c, f, 0.0, 0.0);
        lat_mm.push(if a.ok() && b.ok() {
            a.at_z(z_img)[1] - b.at_z(z_img)[1]
        } else {
            0.0
        });
    }
    let lat_arcmin: Vec<f64> = lat_mm.iter().map(|x| to_arcmin(*x)).collect();
    let (shift_wl, shift_mm) = if cfg.lite {
        (vec![], vec![])
    } else {
        let wl: Vec<f64> = (0..=26).map(|k| 420.0 + 10.0 * k as f64).collect();
        let mut s2 = sys.clone();
        let mut out = Vec::new();
        for &l in &wl {
            s2.wavelengths_nm = vec![l];
            s2.primary = 0;
            let p2 = s2.prepare()?;
            let px = paraxial(&s2, &p2, 0);
            out.push(fin(px.z_paraxial_image - parax[pw].z_paraxial_image));
        }
        (wl, out)
    };

    // ---- ray fans, OPD, field curves --------------------------------------
    let mut fans = Vec::new();
    let mut wavefront = Vec::new();
    let mut curves = None;
    let lam_mm = sys.wavelengths_nm[pw] * 1e-6;
    // reference sphere for OPD: centre at the chief-ray image point, through the exit pupil
    let opd_of = |f: f64, rays: &[(f64, f64)]| -> Option<Vec<(f64, f64, f64)>> {
        if ideal {
            return None;
        }
        let chief = trace_at(&sys, &prep, pw, f, 0.0, 0.0);
        if !chief.ok() {
            return None;
        }
        let rimg = chief.at_z(z_img);
        let e = match parax[pw].z_exit_pupil {
            Some(zx) if (zx - z_img).abs() > 1e-6 => chief.at_z(zx),
            _ => chief.at_z(z_img - 1e6 * chief.d[2].signum()),
        };
        let rr = ((e[0] - rimg[0]).powi(2) + (e[1] - rimg[1]).powi(2) + (e[2] - rimg[2]).powi(2)).sqrt();
        let path = |r: &RayOut| -> Option<f64> {
            let q = [r.p[0] - rimg[0], r.p[1] - rimg[1], r.p[2] - rimg[2]];
            let b = r.d[0] * q[0] + r.d[1] * q[1] + r.d[2] * q[2];
            let c = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] - rr * rr;
            let disc = b * b - c;
            if disc < 0.0 {
                return None;
            }
            let side = [e[0] - rimg[0], e[1] - rimg[1], e[2] - rimg[2]];
            let mut best = None;
            for s in [-b + disc.sqrt(), -b - disc.sqrt()] {
                let pt = [q[0] + s * r.d[0], q[1] + s * r.d[1], q[2] + s * r.d[2]];
                if pt[0] * side[0] + pt[1] * side[1] + pt[2] * side[2] > 0.0 {
                    best = Some(s);
                }
            }
            best.map(|s| r.opl + r.n_img.abs() * s)
        };
        let ref_opl = path(&chief)?;
        let mut out = Vec::new();
        for &(px, py) in rays {
            let r = trace_at(&sys, &prep, pw, f, px, py);
            if !r.ok() {
                continue;
            }
            if let Some(o) = path(&r) {
                out.push((px, py, (ref_opl - o) / lam_mm));
            }
        }
        Some(out)
    };
    if !cfg.lite {
        let nf = cfg.fan_points.max(5);
        let pu: Vec<f64> = (0..nf).map(|k| -1.0 + 2.0 * k as f64 / (nf - 1) as f64).collect();
        for &f in &sys.fields_deg {
            let chief = trace_at(&sys, &prep, pw, f, 0.0, 0.0);
            let c0 = if chief.ok() { chief.at_z(z_img) } else { [0.0; 3] };
            let mut ey = vec![vec![0.0; nf]; nw];
            let mut ex = vec![vec![0.0; nf]; nw];
            let mut ok = vec![vec![true; nf]; nw];
            for w in 0..nw {
                for (k, &u) in pu.iter().enumerate() {
                    let rt = trace_at(&sys, &prep, w, f, 0.0, u);
                    let rs = trace_at(&sys, &prep, w, f, u, 0.0);
                    if rt.ok() {
                        ey[w][k] = rt.at_z(z_img)[1] - c0[1];
                    } else {
                        ok[w][k] = false;
                    }
                    if rs.ok() {
                        ex[w][k] = rs.at_z(z_img)[0] - c0[0];
                    } else {
                        ok[w][k] = false;
                    }
                }
            }
            let fan_rays: Vec<(f64, f64)> = pu.iter().map(|&u| (0.0, u)).collect();
            let opd_y = match opd_of(f, &fan_rays) {
                Some(v) => {
                    // remove the tilt/piston along the fan so it reads like a Zemax OPD fan
                    let mut m = vec![0.0; nf];
                    for (px, py, o) in v {
                        let _ = px;
                        if let Some(k) = pu.iter().position(|u| (u - py).abs() < 1e-12) {
                            m[k] = o;
                        }
                    }
                    m
                }
                None => vec![0.0; nf],
            };
            fans.push(FanField {
                field_deg: f,
                pupil: pu.clone(),
                ey,
                ex,
                ok,
                opd_y,
            });
            // wavefront RMS over a square grid inside the pupil
            let g = 13;
            let mut grid = Vec::new();
            for i in 0..g {
                for j in 0..g {
                    let (x, y) = (-1.0 + 2.0 * i as f64 / (g - 1) as f64, -1.0 + 2.0 * j as f64 / (g - 1) as f64);
                    if x * x + y * y <= 1.0 {
                        grid.push((x, y));
                    }
                }
            }
            let (rms, pv) = match opd_of(f, &grid) {
                Some(v) if v.len() >= 4 => detilt(&v),
                _ => (0.0, 0.0),
            };
            wavefront.push(Wavefront {
                field_deg: f,
                rms_waves: rms,
                pv_waves: pv,
                strehl: (-(2.0 * std::f64::consts::PI * rms).powi(2)).exp(),
            });
        }
        // field curvature and distortion
        let fmax = sys.fields_deg.iter().cloned().fold(0.0, |a: f64, b: f64| a.max(b.abs()));
        let n = cfg.curve_points.max(3);
        let fs: Vec<f64> = (0..n).map(|k| fmax * k as f64 / (n - 1) as f64).collect();
        let mut tan = vec![vec![0.0; n]; nw];
        let mut sag = vec![vec![0.0; n]; nw];
        let mut dist = vec![0.0; n];
        let mut hgt = vec![0.0; n];
        let dp = 0.02;
        for (k, &f) in fs.iter().enumerate() {
            for w in 0..nw {
                let c = trace_at(&sys, &prep, w, f, 0.0, 0.0);
                if !c.ok() {
                    continue;
                }
                let t1 = trace_at(&sys, &prep, w, f, 0.0, dp);
                let s1 = trace_at(&sys, &prep, w, f, dp, 0.0);
                if t1.ok() {
                    tan[w][k] = fin(focus_z(&c, &t1, true).map(|z| z - z_img).unwrap_or(0.0)).clamp(-1e3, 1e3);
                }
                if s1.ok() {
                    sag[w][k] = fin(focus_z(&c, &s1, false).map(|z| z - z_img).unwrap_or(0.0)).clamp(-1e3, 1e3);
                }
                if w == pw {
                    let y = c.at_z(z_img)[1];
                    hgt[k] = y;
                    let yp = parax[pw].chief_height_per_tan * f.to_radians().tan();
                    dist[k] = if yp.abs() > 1e-12 { 100.0 * (y - yp) / yp } else { 0.0 };
                }
            }
        }
        curves = Some(FieldCurves {
            field_deg: fs,
            tangential: tan,
            sagittal: sag,
            distortion_pct: dist,
            image_height_mm: hgt,
        });
    }

    // ---- MTF ----------------------------------------------------------------
    let fnum = parax[pw].working_fnum;
    let cutoff = if fnum.is_finite() && fnum > 0.0 {
        1.0 / (lam_mm * fnum)
    } else {
        0.0
    };
    let nyq = cfg.nyquist_lp_mm;
    let fmax_nu = if nyq > 0.0 {
        (nyq * 2.0).min(if cutoff > 0.0 { cutoff } else { f64::INFINITY })
    } else if cutoff > 0.0 {
        cutoff * 0.5
    } else {
        100.0
    };
    let nfreq = if cfg.lite { 2 } else { 41 };
    let freq: Vec<f64> = if cfg.lite {
        vec![0.0, nyq]
    } else {
        (0..nfreq).map(|k| fmax_nu * k as f64 / (nfreq - 1) as f64).collect()
    };
    let diffraction: Vec<f64> = freq.iter().map(|&nu| diffraction_mtf(nu, cutoff)).collect();
    let d_nyq = if nyq > 0.0 { diffraction_mtf(nyq, cutoff) } else { 0.0 };
    let mut mtf_fields = Vec::new();
    for (fi, raw) in spot_raw.iter().enumerate() {
        let xs: Vec<f64> = raw.iter().map(|p| p[0]).collect();
        let ys: Vec<f64> = raw.iter().map(|p| p[1]).collect();
        let gt: Vec<f64> = freq.iter().map(|&nu| geo_mtf(&ys, nu)).collect();
        let gs: Vec<f64> = freq.iter().map(|&nu| geo_mtf(&xs, nu)).collect();
        let t: Vec<f64> = gt.iter().zip(&diffraction).map(|(a, b)| a * b).collect();
        let s: Vec<f64> = gs.iter().zip(&diffraction).map(|(a, b)| a * b).collect();
        let (nt, ns) = if nyq > 0.0 {
            let ideal_geo = ideal && raw.iter().all(|p| p[0].abs() < 1e-9 && p[1].abs() < 1e-9);
            let _ = ideal_geo;
            (geo_mtf(&ys, nyq) * d_nyq, geo_mtf(&xs, nyq) * d_nyq)
        } else {
            (0.0, 0.0)
        };
        mtf_fields.push(MtfField {
            field_deg: sys.fields_deg[fi],
            tangential: t,
            sagittal: s,
            geo_tangential: gt,
            geo_sagittal: gs,
            at_nyquist_t: nt,
            at_nyquist_s: ns,
        });
    }

    // a field where no ray reaches the image is a failure, not a perfect spot
    let dead: Vec<f64> = spots.iter().filter(|s| s.failed >= s.rays).map(|s| s.field_deg).collect();
    if !dead.is_empty() {
        warnings.push(format!(
            "no ray reaches the image at field(s) {dead:?}° (total internal reflection or vignetting)"
        ));
    }
    let worst_rms_mm = spots
        .iter()
        .map(|s| if s.failed >= s.rays { f64::INFINITY } else { s.rms_mm })
        .fold(0.0, f64::max);
    let worst_rms_mm = if worst_rms_mm.is_finite() { worst_rms_mm } else { 1e3 };
    let worst_mtf = mtf_fields.iter().map(|m| m.at_nyquist_t.min(m.at_nyquist_s)).fold(1.0, f64::min);
    let worst_lat = lat_arcmin.iter().map(|x| x.abs()).fold(0.0, f64::max);
    Ok(Quality {
        efl: fin(efl),
        z_image: z_img,
        focus_shift_mm: shift,
        image_distance_mm: sys.surfaces.last().map(|s| s.thickness).unwrap_or(0.0),
        spots,
        fans,
        curves,
        chromatic: Chromatic {
            longitudinal_mm: fin(long_mm),
            longitudinal_diopter: fin(long_d),
            lateral_mm: lat_mm,
            lateral_arcmin: lat_arcmin,
            shift_wavelengths_nm: shift_wl,
            shift_mm,
        },
        wavefront,
        mtf: Mtf {
            freq_lp_mm: freq,
            diffraction,
            cutoff_lp_mm: fin(cutoff),
            nyquist_lp_mm: nyq,
            diffraction_at_nyquist: d_nyq,
            fields: mtf_fields,
        },
        worst_rms_mm,
        worst_rms_arcmin: to_arcmin(worst_rms_mm),
        worst_mtf_nyquist: if nyq > 0.0 { worst_mtf } else { 0.0 },
        worst_lateral_arcmin: worst_lat,
        pixel_arcmin: to_arcmin(cfg.pixel_mm),
        paraxial: parax
            .into_iter()
            .map(|mut p| {
                p.efl = fin(p.efl);
                p.bfl = fin(p.bfl);
                p.ffl = fin(p.ffl);
                p.z_front_principal = fin(p.z_front_principal);
                p.z_rear_principal = fin(p.z_rear_principal);
                p.z_front_focal = fin(p.z_front_focal);
                p.z_rear_focal = fin(p.z_rear_focal);
                p.z_paraxial_image = fin(p.z_paraxial_image);
                p.exit_pupil_diameter = fin(p.exit_pupil_diameter);
                p.working_fnum = fin(p.working_fnum);
                p.z_exit_pupil = p.z_exit_pupil.map(fin);
                p
            })
            .collect(),
        warnings,
        ideal,
    })
}

/// Remove piston and tilt (least squares on 1, px, py); return (RMS, peak-to-valley).
fn detilt(v: &[(f64, f64, f64)]) -> (f64, f64) {
    // normal equations 3×3
    let mut a = [[0.0f64; 3]; 3];
    let mut b = [0.0f64; 3];
    for &(x, y, o) in v {
        let f = [1.0, x, y];
        for i in 0..3 {
            for j in 0..3 {
                a[i][j] += f[i] * f[j];
            }
            b[i] += f[i] * o;
        }
    }
    let m: Vec<Vec<f64>> = a.iter().map(|r| r.to_vec()).collect();
    let coef = crate::math::solve(m, b.to_vec()).unwrap_or_else(|| vec![0.0; 3]);
    let res: Vec<f64> = v.iter().map(|&(x, y, o)| o - coef[0] - coef[1] * x - coef[2] * y).collect();
    let n = res.len() as f64;
    let rms = (res.iter().map(|r| r * r).sum::<f64>() / n).sqrt();
    let pv = res.iter().cloned().fold(f64::MIN, f64::max) - res.iter().cloned().fold(f64::MAX, f64::min);
    (rms, pv)
}

#[cfg(test)]
mod tests {
    use super::super::seq::Surface;
    use super::*;

    fn bk7_singlet() -> System {
        System {
            surfaces: vec![
                Surface {
                    thickness: 2.0,
                    ..Default::default()
                },
                Surface {
                    radius: 51.5,
                    thickness: 4.0,
                    material: "N-BK7".into(),
                    ..Default::default()
                },
                Surface {
                    radius: 0.0,
                    thickness: 97.0,
                    ..Default::default()
                },
            ],
            pupil_diameter_mm: 10.0,
            fields_deg: vec![0.0, 2.0],
            ..Default::default()
        }
    }

    #[test]
    fn ideal_lens_has_zero_spot_and_diffraction_limited_mtf() {
        let sys = System {
            surfaces: vec![
                Surface {
                    thickness: 5.0,
                    ..Default::default()
                },
                Surface {
                    paraxial_f: 20.0,
                    thickness: 20.0,
                    ..Default::default()
                },
            ],
            pupil_diameter_mm: 4.0,
            fields_deg: vec![0.0, 8.0, 12.0],
            ..Default::default()
        };
        let q = analyze(
            &sys,
            &QualityConfig {
                nyquist_lp_mm: 38.1,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(q.worst_rms_mm < 1e-9, "{}", q.worst_rms_mm);
        // F/5 at 587.6 nm → cutoff 1/(λN) = 340 lp/mm
        assert!(
            (q.mtf.cutoff_lp_mm - 1.0 / (587.5618e-6 * 5.0)).abs() < 0.5,
            "{}",
            q.mtf.cutoff_lp_mm
        );
        assert!((q.worst_mtf_nyquist - q.mtf.diffraction_at_nyquist).abs() < 1e-9);
        // distortion-free in tan θ
        let c = q.curves.unwrap();
        assert!(c.distortion_pct.iter().all(|d| d.abs() < 1e-6));
    }

    #[test]
    fn diffraction_mtf_known_values() {
        assert!((diffraction_mtf(0.0, 100.0) - 1.0).abs() < 1e-12);
        // at half the cutoff the circular-pupil MTF is 0.3910
        assert!((diffraction_mtf(50.0, 100.0) - 0.3910).abs() < 1e-3);
        assert_eq!(diffraction_mtf(120.0, 100.0), 0.0);
    }

    #[test]
    fn singlet_spherical_aberration_matches_third_order_estimate() {
        // plano-convex BK7, f ≈ 100 mm, convex side to the collimated beam, 10 mm beam (F/10)
        let sys = bk7_singlet();
        let q = analyze(
            &sys,
            &QualityConfig {
                auto_focus: false,
                ..Default::default()
            },
        )
        .unwrap();
        let px = &q.paraxial[1];
        assert!((px.efl - 51.5 / (0.5168)).abs() < 0.05, "{}", px.efl);
        // longitudinal SA of the marginal ray: third order ≈ 0.27 · y²/f for n = 1.5 in best form;
        // plano-convex (convex first) is close to best form: expect ~0.3 mm here.
        let sysp = sys.clone();
        let prep = sysp.prepare().unwrap();
        let m = trace_at(&sysp, &prep, 1, 0.0, 0.0, 1.0);
        let zm = m.p[2] - m.p[1] * m.d[2] / m.d[1];
        let lsa = px.z_paraxial_image - zm;
        assert!(lsa > 0.15 && lsa < 0.6, "LSA {lsa}");
        // longitudinal colour of a BK7 singlet ≈ f / V
        let lc = q.chromatic.longitudinal_mm;
        assert!((lc + px.efl / 64.17).abs() / (px.efl / 64.17) < 0.1, "LCA {lc}");
        // on-axis: no lateral colour, spot is round
        assert!(q.chromatic.lateral_mm[0].abs() < 1e-12);
        // OPD of an F/10 singlet with this much SA is a fraction of a wave to a few waves
        assert!(
            q.wavefront[0].rms_waves > 0.05 && q.wavefront[0].rms_waves < 5.0,
            "{}",
            q.wavefront[0].rms_waves
        );
    }

    #[test]
    fn best_focus_reduces_rms() {
        let sys = bk7_singlet();
        let a = analyze(
            &sys,
            &QualityConfig {
                auto_focus: false,
                ..Default::default()
            },
        )
        .unwrap();
        let b = analyze(&sys, &QualityConfig::default()).unwrap();
        assert!(b.worst_rms_mm <= a.worst_rms_mm + 1e-12);
        assert!(b.focus_shift_mm < 0.0, "SA pulls best focus towards the lens");
    }
}
