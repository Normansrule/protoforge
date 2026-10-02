//! D2Q9 lattice-Boltzmann method (LBM) wind tunnel with the
//! Bhatnagar–Gross–Krook (BGK) collision operator.
//!
//! ## Model
//! Nine particle populations `f_i(x, t)` per lattice node move with the
//! discrete velocities `c_i ∈ {0, ±e_x, ±e_y, ±e_x ± e_y}` (lattice units:
//! Δx = Δt = 1). Each time step is
//!
//! ```text
//!   collide:  f_i* = f_i − (f_i − f_i^eq)/τ (+ Guo forcing term)
//!   stream:   f_i(x + c_i, t + 1) = f_i*(x, t)
//!   f_i^eq = w_i ρ [1 + 3 c_i·u + 9/2 (c_i·u)² − 3/2 u²],   w = 4/9, 1/9 (axis), 1/36 (diagonal)
//! ```
//!
//! with density `ρ = Σ f_i` and momentum `ρu = Σ f_i c_i`. A Chapman–Enskog
//! expansion recovers the incompressible Navier–Stokes equations at low Mach
//! number with kinematic viscosity `ν = (τ − ½)/3` and pressure `p = ρ/3`
//! (speed of sound 1/√3). So the Reynolds number `Re = U D / ν` is set by
//! the relaxation time: `τ = 3 U D / Re + ½`. BGK becomes unstable as
//! τ → ½, so high Re needs more cells across the obstacle.
//!
//! ## Boundaries
//! * Obstacles and no-slip walls: half-way **bounce-back** — a population
//!   that would enter a solid node returns along its own link, putting the
//!   wall half-way between nodes (second-order accurate for straight walls).
//! * Free-slip walls: specular reflection of the wall-normal velocity.
//! * Inlet (prescribed velocity) and outlet (prescribed density):
//!   non-equilibrium extrapolation — the boundary node takes the equilibrium
//!   at the boundary values plus the non-equilibrium part of its neighbour
//!   (Guo, Zheng & Shi 2002).
//! * A constant body force (to drive periodic channel flow) enters through
//!   Guo's forcing scheme (Guo, Zheng & Shi 2002, Phys. Rev. E 65 046308).
//!
//! ## Forces and shedding
//! The force on an obstacle is the **momentum exchange** across every
//! fluid–solid link (Ladd 1994): each bounced population carries `2 c f`, so
//! `F = Σ_links 2 c_{i→wall} f_i*`. Drag and lift coefficients
//! `C_D = F_x / (½ ρ U² D)`, `C_L = F_y / (½ ρ U² D)` (per unit span). The
//! Strouhal number `St = f D / U` comes from the dominant frequency of the
//! transverse velocity at a probe behind the obstacle (Hann-windowed FFT
//! with parabolic peak interpolation).
//!
//! References: T. Krüger, H. Kusumaatmaja, A. Kuzmin, O. Shardt, G. Silva,
//! E. M. Viggen, *The Lattice Boltzmann Method: Principles and Practice*
//! (Springer, 2017); S. Succi, *The Lattice Boltzmann Equation for Fluid
//! Dynamics and Beyond* (Oxford, 2001); A. J. C. Ladd, "Numerical
//! simulations of particulate suspensions via a discretized Boltzmann
//! equation", J. Fluid Mech. 271 (1994); R. Mei, D. Yu, W. Shyy, L.-S. Luo,
//! "Force evaluation in the lattice Boltzmann method involving curved
//! geometry", Phys. Rev. E 65 (2002) 041203; M. Schäfer & S. Turek,
//! "Benchmark computations of laminar flow around a cylinder" (Notes on
//! Numerical Fluid Mechanics 52, Vieweg, 1996); C. H. K. Williamson,
//! "Vortex dynamics in the cylinder wake", Annu. Rev. Fluid Mech. 28 (1996).

use std::cell::RefCell;

use serde::{Deserialize, Serialize};

use crate::math::{Rng, C64};
use crate::project::Project;

pub const CX: [i32; 9] = [0, 1, 0, -1, 0, 1, -1, -1, 1];
pub const CY: [i32; 9] = [0, 0, 1, 0, -1, 1, 1, -1, -1];
pub const W: [f64; 9] = [
    4.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
];
pub const OPP: [usize; 9] = [0, 3, 4, 1, 2, 7, 8, 5, 6];
/// Direction with the y velocity mirrored (free-slip reflection).
pub const MIRROR_Y: [usize; 9] = [0, 1, 4, 3, 2, 8, 7, 6, 5];

#[inline]
pub fn feq(i: usize, rho: f64, ux: f64, uy: f64) -> f64 {
    let cu = CX[i] as f64 * ux + CY[i] as f64 * uy;
    W[i] * rho * (1.0 + 3.0 * cu + 4.5 * cu * cu - 1.5 * (ux * ux + uy * uy))
}

// ------------------------------------------------------------------ configuration

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LbmConfig {
    pub nx: usize,
    pub ny: usize,
    /// none | cylinder | naca | temple | pod | rectangle
    pub obstacle: String,
    /// Characteristic size across the flow (cylinder diameter, chord for the airfoil), cells.
    pub size_cells: f64,
    /// Obstacle centre as fractions of the domain.
    pub x_frac: f64,
    pub y_frac: f64,
    pub angle_deg: f64,
    /// Four-digit NACA code, e.g. "0012" or "2412".
    pub naca: String,
    /// Rectangle width ÷ height.
    pub aspect: f64,
    pub re: f64,
    /// Inlet velocity (mean for a parabolic profile), lattice units.
    pub u_in: f64,
    /// slip | noslip | periodic
    pub walls: String,
    /// uniform | parabolic
    pub inlet: String,
    /// Periodic in x instead of inlet/outlet (channel tests).
    pub periodic_x: bool,
    /// Body force along x per node (lattice units), for driven channels.
    pub body_force: f64,
    pub tracers: usize,
    pub seed: u64,
    /// Optional custom obstacle mask (nx·ny bytes, base64; non-zero = solid), added to the shape.
    pub mask: Option<String>,
    /// For the temple / pod cross-sections.
    pub project: Option<Project>,
    /// Transverse velocity kick at start (fraction of u_in) to trigger shedding.
    pub perturb: f64,
}

impl Default for LbmConfig {
    fn default() -> Self {
        LbmConfig {
            nx: 320,
            ny: 112,
            obstacle: "cylinder".into(),
            size_cells: 20.0,
            x_frac: 0.2,
            y_frac: 0.5,
            angle_deg: 0.0,
            naca: "0012".into(),
            aspect: 1.0,
            re: 150.0,
            u_in: 0.1,
            walls: "slip".into(),
            inlet: "uniform".into(),
            periodic_x: false,
            body_force: 0.0,
            tracers: 500,
            seed: 7,
            mask: None,
            project: None,
            perturb: 0.05,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct LbmInfo {
    pub nx: usize,
    pub ny: usize,
    pub tau: f64,
    pub nu: f64,
    /// Reynolds number actually simulated (may be capped by τ ≥ 0.505).
    pub re: f64,
    pub re_requested: f64,
    pub u_ref: f64,
    pub d_cells: f64,
    pub mach: f64,
    pub blockage: f64,
    pub probe: [usize; 2],
    pub solid_cells: usize,
    /// Real-world Reynolds number of the same shape on Iris at walking speed (1.4 m/s), if known.
    pub re_walking: Option<f64>,
    pub real_size_mm: Option<f64>,
    pub warnings: Vec<String>,
}

// ------------------------------------------------------------------ shapes

fn point_in_poly(poly: &[[f64; 2]], x: f64, y: f64) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a[1] > y) != (b[1] > y) && x < (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// NACA 4-digit outline (unit chord, leading edge at 0), closed polygon.
pub fn naca4(code: &str, n: usize) -> Vec<[f64; 2]> {
    let d: Vec<u32> = code.chars().filter_map(|c| c.to_digit(10)).collect();
    let (m, p, t) = if d.len() == 4 {
        (d[0] as f64 / 100.0, d[1] as f64 / 10.0, (d[2] * 10 + d[3]) as f64 / 100.0)
    } else {
        (0.0, 0.0, 0.12)
    };
    let t = t.clamp(0.01, 0.4);
    let mut up = Vec::new();
    let mut lo = Vec::new();
    for k in 0..=n {
        let beta = std::f64::consts::PI * k as f64 / n as f64;
        let x = 0.5 * (1.0 - beta.cos());
        let yt = 5.0 * t * (0.2969 * x.sqrt() - 0.1260 * x - 0.3516 * x * x + 0.2843 * x.powi(3) - 0.1015 * x.powi(4));
        let (yc, dyc) = if m > 0.0 && p > 0.0 {
            if x < p {
                (m / (p * p) * (2.0 * p * x - x * x), 2.0 * m / (p * p) * (p - x))
            } else {
                (
                    m / ((1.0 - p) * (1.0 - p)) * (1.0 - 2.0 * p + 2.0 * p * x - x * x),
                    2.0 * m / ((1.0 - p) * (1.0 - p)) * (p - x),
                )
            }
        } else {
            (0.0, 0.0)
        };
        let th = dyc.atan();
        up.push([x - yt * th.sin(), yc + yt * th.cos()]);
        lo.push([x + yt * th.sin(), yc - yt * th.cos()]);
    }
    let mut poly = up;
    poly.extend(lo.into_iter().rev().skip(1));
    poly
}

/// Rounded rectangle outline centred at 0 (width along x, height along y).
fn rounded_rect(w: f64, h: f64, r: f64) -> Vec<[f64; 2]> {
    let r = r.clamp(0.0, 0.5 * w.min(h));
    let mut out = Vec::new();
    let corners = [
        (w / 2.0 - r, h / 2.0 - r, 0.0),
        (-w / 2.0 + r, h / 2.0 - r, 90.0),
        (-w / 2.0 + r, -h / 2.0 + r, 180.0),
        (w / 2.0 - r, -h / 2.0 + r, 270.0),
    ];
    for (cx, cy, a0) in corners {
        for k in 0..=8 {
            let a = (a0 + 90.0 * k as f64 / 8.0f64).to_radians();
            out.push([cx + r * a.cos(), cy + r * a.sin()]);
        }
    }
    out
}

/// Build the obstacle mask and return (mask, characteristic length, real size mm).
fn shape_mask(c: &LbmConfig) -> (Vec<u8>, f64, Option<f64>, Vec<String>) {
    let (nx, ny) = (c.nx, c.ny);
    let mut mask = vec![0u8; nx * ny];
    let mut warn = Vec::new();
    let d = c.size_cells.clamp(2.0, ny as f64 * 0.8);
    let (x0, y0) = (c.x_frac.clamp(0.02, 0.98) * nx as f64, c.y_frac.clamp(0.02, 0.98) * ny as f64);
    let rot = c.angle_deg.to_radians();
    let (cs, sn) = (rot.cos(), rot.sin());
    // local polygon (x along the flow, y across), centred at the origin
    let p = c.project.clone().unwrap_or_else(Project::iris);
    let fr = &p.design.frame;
    let mut real = None;
    let poly: Option<Vec<[f64; 2]>> = match c.obstacle.as_str() {
        "cylinder" => {
            for j in 0..ny {
                for i in 0..nx {
                    let (dx, dy) = (i as f64 + 0.5 - x0, j as f64 + 0.5 - y0);
                    if dx * dx + dy * dy <= 0.25 * d * d {
                        mask[i + nx * j] = 1;
                    }
                }
            }
            None
        }
        "naca" => {
            // chord = d, pitched nose-up by angle (rotate about the quarter chord)
            let pts = naca4(&c.naca, 60);
            Some(pts.into_iter().map(|q| [(q[0] - 0.25) * d, q[1] * d]).collect())
        }
        "temple" => {
            let (w, h) = (fr.temple_width_mm.max(0.5), fr.temple_height_mm.max(0.5));
            real = Some(h);
            let s = d / h;
            Some(rounded_rect(w * s, h * s, 0.3 * w.min(h) * s))
        }
        "pod" => {
            let (w, h) = (fr.pod_width_mm + fr.wall_mm, fr.pod_height_mm);
            real = Some(h);
            let s = d / h;
            let r = ((fr.pod_height_mm - fr.temple_height_mm) / 2.0).clamp(0.0, 3.0);
            Some(rounded_rect(w * s, h * s, r * s))
        }
        "rectangle" => Some(rounded_rect(d * c.aspect.clamp(0.05, 20.0), d, 0.0)),
        "none" => None,
        other => {
            warn.push(format!("unknown obstacle '{other}' — empty tunnel"));
            None
        }
    };
    if let Some(poly) = poly {
        // rotate: positive angle pitches the leading edge (−x side) up
        let poly: Vec<[f64; 2]> = poly.iter().map(|q| [q[0] * cs + q[1] * sn, -q[0] * sn + q[1] * cs]).collect();
        let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
        for q in &poly {
            for k in 0..2 {
                lo[k] = lo[k].min(q[k]);
                hi[k] = hi[k].max(q[k]);
            }
        }
        let i0 = ((x0 + lo[0]).floor().max(0.0)) as usize;
        let i1 = ((x0 + hi[0]).ceil().max(0.0) as usize).min(nx);
        let j0 = ((y0 + lo[1]).floor().max(0.0)) as usize;
        let j1 = ((y0 + hi[1]).ceil().max(0.0) as usize).min(ny);
        for j in j0..j1 {
            for i in i0..i1 {
                if point_in_poly(&poly, i as f64 + 0.5 - x0, j as f64 + 0.5 - y0) {
                    mask[i + nx * j] = 1;
                }
            }
        }
    }
    if let Some(b) = &c.mask {
        if let Some(bytes) = base64_decode(b) {
            for (m, v) in mask.iter_mut().zip(bytes) {
                if v != 0 {
                    *m = 1;
                }
            }
        } else {
            warn.push("custom mask is not valid base64 — ignored".into());
        }
    }
    (mask, d, real, warn)
}

/// Standard base64 decoder (padding optional); None on invalid input.
pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        })
    };
    let bytes: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut acc = 0u32;
        for (k, c) in chunk.iter().enumerate() {
            acc |= val(*c)? << (18 - 6 * k);
        }
        let n = match chunk.len() {
            4 => 3,
            3 => 2,
            2 => 1,
            _ => return None,
        };
        for k in 0..n {
            out.push((acc >> (16 - 8 * k)) as u8);
        }
    }
    Some(out)
}

// ------------------------------------------------------------------ solver

#[derive(Clone, Debug)]
pub struct Lbm {
    pub nx: usize,
    pub ny: usize,
    pub f: Vec<f64>,
    tmp: Vec<f64>,
    pub mask: Vec<u8>,
    pub tau: f64,
    pub u_ref: f64,
    pub d: f64,
    pub cfg: LbmConfig,
    pub step: u64,
    pub info: LbmInfo,
    /// Force on the obstacle during the last step (lattice units).
    pub force: [f64; 2],
    pub hist_step: Vec<u64>,
    pub hist_cd: Vec<f64>,
    pub hist_cl: Vec<f64>,
    pub hist_probe: Vec<f64>,
    pub tracers: Vec<[f64; 3]>,
    rng: Rng,
    pub unstable: bool,
    ux: Vec<f64>,
    uy: Vec<f64>,
    rho: Vec<f64>,
}

const HIST_MAX: usize = 60_000;

impl Lbm {
    pub fn new(cfg: LbmConfig) -> Result<Lbm, String> {
        let (nx, ny) = (cfg.nx.clamp(8, 1200), cfg.ny.clamp(4, 600));
        if nx * ny > 400_000 {
            return Err("grid too large (limit 400 000 cells)".into());
        }
        let cfg = LbmConfig { nx, ny, ..cfg };
        let (mask, d, real, mut warnings) = shape_mask(&cfg);
        let u_ref = cfg.u_in.clamp(0.0, 0.3);
        if u_ref > 0.15 {
            warnings.push(format!(
                "u = {u_ref:.2} lattice units: Mach {:.2} — compressibility errors grow above ≈ 0.1",
                u_ref * 3f64.sqrt()
            ));
        }
        let re_req = cfg.re.max(0.1);
        let mut nu = if u_ref > 0.0 { u_ref * d / re_req } else { 1.0 / 6.0 };
        let tau_min = 0.505;
        let mut tau = 3.0 * nu + 0.5;
        if tau < tau_min {
            tau = tau_min;
            nu = (tau - 0.5) / 3.0;
            warnings.push(format!(
                "Re {:.0} needs τ = {:.4} < {tau_min}: capped at Re {:.0}. Use more cells across the obstacle (or a smaller inlet speed) for higher Re.",
                re_req,
                3.0 * u_ref * d / re_req + 0.5,
                u_ref * d / nu
            ));
        }
        let tau = tau.min(5.0);
        let re = if nu > 0.0 { u_ref * d / nu } else { 0.0 };
        let solid_cells = mask.iter().filter(|m| **m != 0).count();
        let probe = [
            ((cfg.x_frac * nx as f64 + 2.0 * d) as usize).min(nx - 1),
            ((cfg.y_frac * ny as f64) as usize).min(ny - 1),
        ];
        let re_walking = real.map(|mm| 1.4 * mm * 1e-3 / 1.516e-5);
        let blockage = if cfg.obstacle == "none" { 0.0 } else { d / ny as f64 };
        if blockage > 0.25 && cfg.walls != "periodic" {
            warnings.push(format!(
                "blockage {:.0} % — the walls speed the flow up and raise drag; open-air values need D/H below ≈ 10 %",
                blockage * 100.0
            ));
        }
        let n = nx * ny;
        let mut s = Lbm {
            nx,
            ny,
            f: vec![0.0; 9 * n],
            tmp: vec![0.0; 9 * n],
            mask,
            tau,
            u_ref,
            d,
            step: 0,
            info: LbmInfo {
                nx,
                ny,
                tau,
                nu,
                re,
                re_requested: re_req,
                u_ref,
                d_cells: d,
                mach: u_ref * 3f64.sqrt(),
                blockage,
                probe,
                solid_cells,
                re_walking,
                real_size_mm: real,
                warnings,
            },
            force: [0.0; 2],
            hist_step: vec![],
            hist_cd: vec![],
            hist_cl: vec![],
            hist_probe: vec![],
            tracers: vec![],
            rng: Rng::new(cfg.seed),
            unstable: false,
            ux: vec![0.0; n],
            uy: vec![0.0; n],
            rho: vec![1.0; n],
            cfg,
        };
        // initial state: inlet profile everywhere, a small transverse kick behind the obstacle
        for j in 0..ny {
            for i in 0..nx {
                let k = i + nx * j;
                let (ux, mut uy) = if s.mask[k] != 0 || s.cfg.periodic_x {
                    (0.0, 0.0)
                } else {
                    (s.inlet_u(j), 0.0)
                };
                let x0 = s.cfg.x_frac * nx as f64;
                if s.mask[k] == 0 && (i as f64) > x0 && (i as f64) < x0 + 3.0 * d {
                    uy = s.cfg.perturb * u_ref * (std::f64::consts::PI * (i as f64 - x0) / (3.0 * d)).sin();
                }
                for q in 0..9 {
                    s.f[q * n + k] = feq(q, 1.0, ux, uy);
                }
            }
        }
        s.macros();
        let nt = s.cfg.tracers.min(4000);
        for _ in 0..nt {
            let p = s.spawn(true);
            s.tracers.push(p);
        }
        Ok(s)
    }

    /// Inlet velocity at row j.
    pub fn inlet_u(&self, j: usize) -> f64 {
        if self.cfg.inlet == "parabolic" {
            // mean u_in, max 1.5 u_in, walls half-way outside the first/last rows
            let h = self.ny as f64;
            let y = j as f64 + 0.5;
            6.0 * self.u_ref * y * (h - y) / (h * h)
        } else {
            self.u_ref
        }
    }

    fn spawn(&mut self, anywhere: bool) -> [f64; 3] {
        for _ in 0..50 {
            let x = if anywhere {
                self.rng.uniform() * self.nx as f64
            } else {
                self.rng.uniform() * 2.0
            };
            let y = self.rng.uniform() * self.ny as f64;
            let k = (x as usize).min(self.nx - 1) + self.nx * (y as usize).min(self.ny - 1);
            if self.mask[k] == 0 {
                return [x, y, 0.0];
            }
        }
        [0.5, self.ny as f64 / 2.0, 0.0]
    }

    fn macros(&mut self) {
        let n = self.nx * self.ny;
        let fx = self.cfg.body_force;
        for k in 0..n {
            if self.mask[k] != 0 {
                self.rho[k] = 1.0;
                self.ux[k] = 0.0;
                self.uy[k] = 0.0;
                continue;
            }
            let (mut r, mut mx, mut my) = (0.0, 0.0, 0.0);
            for q in 0..9 {
                let v = self.f[q * n + k];
                r += v;
                mx += v * CX[q] as f64;
                my += v * CY[q] as f64;
            }
            self.rho[k] = r;
            self.ux[k] = (mx + 0.5 * fx) / r;
            self.uy[k] = my / r;
        }
    }

    /// Total mass Σ ρ over fluid nodes.
    pub fn mass(&self) -> f64 {
        let n = self.nx * self.ny;
        (0..n)
            .filter(|&k| self.mask[k] == 0)
            .map(|k| (0..9).map(|q| self.f[q * n + k]).sum::<f64>())
            .sum()
    }

    /// Advance `steps` time steps.
    pub fn run(&mut self, steps: usize) {
        for _ in 0..steps {
            if self.unstable {
                return;
            }
            self.one_step();
        }
    }

    fn one_step(&mut self) {
        let (nx, ny) = (self.nx, self.ny);
        let n = nx * ny;
        let omega = 1.0 / self.tau;
        let fx = self.cfg.body_force;
        // collide (in place)
        for k in 0..n {
            if self.mask[k] != 0 {
                continue;
            }
            let (r, ux, uy) = (self.rho[k], self.ux[k], self.uy[k]);
            for q in 0..9 {
                let e = feq(q, r, ux, uy);
                let mut v = self.f[q * n + k];
                v -= omega * (v - e);
                if fx != 0.0 {
                    let (cx, cy) = (CX[q] as f64, CY[q] as f64);
                    let cu = cx * ux + cy * uy;
                    v += (1.0 - 0.5 * omega) * W[q] * (3.0 * (cx - ux) + 9.0 * cu * cx) * fx;
                }
                self.f[q * n + k] = v;
            }
        }
        // stream (pull) with boundaries; momentum exchange on obstacle links
        let slip = self.cfg.walls == "slip";
        let per_y = self.cfg.walls == "periodic";
        let per_x = self.cfg.periodic_x;
        let (mut fxs, mut fys) = (0.0, 0.0);
        for j in 0..ny {
            for i in 0..nx {
                let k = i + nx * j;
                if self.mask[k] != 0 {
                    for q in 0..9 {
                        self.tmp[q * n + k] = self.f[q * n + k];
                    }
                    continue;
                }
                for q in 0..9 {
                    let mut si = i as i32 - CX[q];
                    let mut sj = j as i32 - CY[q];
                    let mut src_q = q;
                    if si < 0 || si >= nx as i32 {
                        if per_x {
                            si = si.rem_euclid(nx as i32);
                        } else {
                            si = si.clamp(0, nx as i32 - 1); // overwritten by the open boundary
                        }
                    }
                    if sj < 0 || sj >= ny as i32 {
                        if per_y {
                            sj = sj.rem_euclid(ny as i32);
                        } else if slip {
                            // specular: the mirrored population from the same row
                            sj = j as i32;
                            src_q = MIRROR_Y[q];
                            si = i as i32 - CX[q];
                            if si < 0 || si >= nx as i32 {
                                si = if per_x {
                                    si.rem_euclid(nx as i32)
                                } else {
                                    si.clamp(0, nx as i32 - 1)
                                };
                            }
                        } else {
                            // no-slip wall: bounce back
                            self.tmp[q * n + k] = self.f[OPP[q] * n + k];
                            continue;
                        }
                    }
                    let sk = si as usize + nx * sj as usize;
                    if self.mask[sk] != 0 {
                        let fo = self.f[OPP[q] * n + k];
                        self.tmp[q * n + k] = fo;
                        // population OPP[q] moved towards the wall with c = −c_q
                        fxs += 2.0 * fo * (-CX[q]) as f64;
                        fys += 2.0 * fo * (-CY[q]) as f64;
                    } else {
                        self.tmp[q * n + k] = self.f[src_q * n + sk];
                    }
                }
            }
        }
        std::mem::swap(&mut self.f, &mut self.tmp);
        // open boundaries: non-equilibrium extrapolation
        if !per_x && nx >= 3 {
            for j in 0..ny {
                let (k0, k1) = (nx * j, 1 + nx * j);
                if self.mask[k0] == 0 && self.mask[k1] == 0 {
                    let (mut r1, mut mx, mut my) = (0.0, 0.0, 0.0);
                    for q in 0..9 {
                        let v = self.f[q * n + k1];
                        r1 += v;
                        mx += v * CX[q] as f64;
                        my += v * CY[q] as f64;
                    }
                    let (u1, v1) = (mx / r1, my / r1);
                    let ub = self.inlet_u(j);
                    for q in 0..9 {
                        self.f[q * n + k0] = feq(q, r1, ub, 0.0) + (self.f[q * n + k1] - feq(q, r1, u1, v1));
                    }
                }
                let (ke, kp) = (nx - 1 + nx * j, nx - 2 + nx * j);
                if self.mask[ke] == 0 && self.mask[kp] == 0 {
                    let (mut r1, mut mx, mut my) = (0.0, 0.0, 0.0);
                    for q in 0..9 {
                        let v = self.f[q * n + kp];
                        r1 += v;
                        mx += v * CX[q] as f64;
                        my += v * CY[q] as f64;
                    }
                    let (u1, v1) = (mx / r1, my / r1);
                    for q in 0..9 {
                        self.f[q * n + ke] = feq(q, 1.0, u1, v1) + (self.f[q * n + kp] - feq(q, r1, u1, v1));
                    }
                }
            }
        }
        self.macros();
        self.step += 1;
        self.force = [fxs, fys];
        let norm = 0.5 * self.u_ref * self.u_ref * self.d;
        let (cd, cl) = if norm > 0.0 { (fxs / norm, fys / norm) } else { (0.0, 0.0) };
        let pk = self.info.probe[0] + nx * self.info.probe[1];
        let probe = self.uy[pk] / self.u_ref.max(1e-12);
        if !(cd.is_finite() && probe.is_finite()) || self.rho[pk] < 0.2 || self.rho[pk] > 5.0 {
            self.unstable = true;
        }
        if self.hist_cd.len() >= HIST_MAX {
            let half = HIST_MAX / 2;
            self.hist_step.drain(..half);
            self.hist_cd.drain(..half);
            self.hist_cl.drain(..half);
            self.hist_probe.drain(..half);
        }
        self.hist_step.push(self.step);
        self.hist_cd.push(cd);
        self.hist_cl.push(cl);
        self.hist_probe.push(probe);
        self.advect_tracers();
    }

    fn vel_at(&self, x: f64, y: f64) -> (f64, f64, bool) {
        let (nx, ny) = (self.nx, self.ny);
        let gx = (x - 0.5).clamp(0.0, nx as f64 - 1.001);
        let gy = (y - 0.5).clamp(0.0, ny as f64 - 1.001);
        let (i, j) = (gx as usize, gy as usize);
        let (tx, ty) = (gx - i as f64, gy - j as f64);
        let k = |a: usize, b: usize| a + nx * b;
        let ks = [k(i, j), k(i + 1, j), k(i, j + 1), k(i + 1, j + 1)];
        let w = [(1.0 - tx) * (1.0 - ty), tx * (1.0 - ty), (1.0 - tx) * ty, tx * ty];
        let mut u = 0.0;
        let mut v = 0.0;
        for (kk, ww) in ks.iter().zip(w) {
            u += ww * self.ux[*kk];
            v += ww * self.uy[*kk];
        }
        let cell = (x as usize).min(nx - 1) + nx * (y as usize).min(ny - 1);
        (u, v, self.mask[cell] != 0)
    }

    fn advect_tracers(&mut self) {
        let (w, h) = (self.nx as f64, self.ny as f64);
        for t in 0..self.tracers.len() {
            let [x, y, age] = self.tracers[t];
            let (u, v, _) = self.vel_at(x, y);
            // midpoint (RK2) step, one lattice time unit
            let (u2, v2, _) = self.vel_at(x + 0.5 * u, y + 0.5 * v);
            let (mut nx_, mut ny_) = (x + u2, y + v2);
            if self.cfg.periodic_x {
                nx_ = nx_.rem_euclid(w);
            }
            if self.cfg.walls == "periodic" {
                ny_ = ny_.rem_euclid(h);
            }
            let out = nx_ < 0.0 || nx_ >= w || ny_ < 0.0 || ny_ >= h;
            let solid = !out && self.vel_at(nx_, ny_).2;
            let old = age > 6.0 * w / self.u_ref.max(1e-3);
            self.tracers[t] = if out || solid || old {
                let fresh = self.cfg.periodic_x;
                self.spawn(fresh)
            } else {
                [nx_, ny_, age + 1.0]
            };
        }
    }

    /// Vorticity ω = ∂v/∂x − ∂u/∂y (central differences; zero in solids).
    pub fn vorticity(&self) -> Vec<f64> {
        let (nx, ny) = (self.nx, self.ny);
        let mut w = vec![0.0; nx * ny];
        for j in 1..ny.saturating_sub(1) {
            for i in 1..nx.saturating_sub(1) {
                let k = i + nx * j;
                if self.mask[k] != 0 {
                    continue;
                }
                w[k] = 0.5 * (self.uy[k + 1] - self.uy[k - 1]) - 0.5 * (self.ux[k + nx] - self.ux[k - nx]);
            }
        }
        w
    }

    /// Quantised field frame: (bytes, lo, hi, centre byte).
    pub fn frame(&self, field: &str, contrast: f64) -> (Vec<u8>, f64, f64) {
        let n = self.nx * self.ny;
        let c = contrast.clamp(0.1, 20.0);
        match field {
            "vorticity" => {
                let w = self.vorticity();
                let s = 1.2 * self.u_ref / self.d.max(1.0) * 4.0 / c;
                let b = w.iter().map(|v| (128.0 + 127.0 * (v / s).clamp(-1.0, 1.0)).round() as u8).collect();
                (b, -s, s)
            }
            "pressure" => {
                // p = ρ/3; show (ρ − 1)/3 against the dynamic pressure ½U²
                let q = 0.5 * self.u_ref * self.u_ref * 2.0 / c;
                let b = (0..n)
                    .map(|k| {
                        let p = (self.rho[k] - 1.0) / 3.0;
                        (128.0 + 127.0 * (p / q).clamp(-1.0, 1.0)).round() as u8
                    })
                    .collect();
                (b, -q, q)
            }
            "ux" => {
                let s = 2.0 * self.u_ref / c.max(1.0);
                let b = self
                    .ux
                    .iter()
                    .map(|v| (128.0 + 127.0 * (v / s).clamp(-1.0, 1.0)).round() as u8)
                    .collect();
                (b, -s, s)
            }
            _ => {
                let s = 1.8 * self.u_ref / c.max(0.5);
                let b = (0..n)
                    .map(|k| {
                        let sp = (self.ux[k] * self.ux[k] + self.uy[k] * self.uy[k]).sqrt();
                        ((sp / s).clamp(0.0, 1.0) * 255.0).round() as u8
                    })
                    .collect();
                (b, 0.0, s)
            }
        }
    }

    /// Mean C_D and C_L over the last `window` steps.
    pub fn mean_coeffs(&self, window: usize) -> (f64, f64) {
        let n = self.hist_cd.len();
        if n == 0 {
            return (0.0, 0.0);
        }
        let a = n.saturating_sub(window.max(1));
        let m = (n - a) as f64;
        (self.hist_cd[a..].iter().sum::<f64>() / m, self.hist_cl[a..].iter().sum::<f64>() / m)
    }

    /// Spectrum of the probe signal over the last ≤ 16384 steps and the Strouhal number.
    pub fn spectrum(&self) -> SpectrumOut {
        let n = self.hist_probe.len();
        let len = n.min(16384);
        let mut out = SpectrumOut {
            st: None,
            freq_per_step: None,
            st_axis: vec![],
            power: vec![],
            samples: len,
            periods: 0.0,
            note: String::new(),
        };
        if len < 512 {
            out.note = "record at least 512 steps".into();
            return out;
        }
        let x = &self.hist_probe[n - len..];
        let mean = x.iter().sum::<f64>() / len as f64;
        let nfft = crate::dsp::fft::next_pow2(len);
        let mut buf: Vec<C64> = x
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let w = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (len - 1) as f64).cos();
                C64::real((v - mean) * w)
            })
            .collect();
        buf.resize(nfft, C64::real(0.0));
        crate::dsp::fft::fft_in_place(&mut buf, false);
        let half = nfft / 2;
        let pw: Vec<f64> = buf[..half].iter().map(|c| c.norm_sqr()).collect();
        // ignore the lowest bins (drift)
        let k0 = (nfft as f64 * 1.5 / len as f64).ceil() as usize + 1;
        let (mut kb, mut pb) = (0usize, 0.0);
        for (k, p) in pw.iter().enumerate().take(half - 1).skip(k0) {
            if *p > pb {
                pb = *p;
                kb = k;
            }
        }
        let scale = self.d / self.u_ref.max(1e-12);
        let total: f64 = pw[k0..].iter().sum();
        if kb > 0 && pb > 0.0 && pb > 0.02 * total {
            let (a, b, c) = (pw[kb - 1].max(1e-300).ln(), pb.ln(), pw[kb + 1].max(1e-300).ln());
            let den = a - 2.0 * b + c;
            let off = if den.abs() > 1e-12 { 0.5 * (a - c) / den } else { 0.0 };
            let f = (kb as f64 + off.clamp(-0.5, 0.5)) / nfft as f64;
            out.freq_per_step = Some(f);
            out.st = Some(f * scale);
            out.periods = f * len as f64;
            if out.periods < 4.0 {
                out.note = format!("only {:.1} periods recorded — let it run longer", out.periods);
            }
        } else {
            out.note = "no dominant frequency (steady wake or not yet shedding)".into();
        }
        // spectrum on a Strouhal axis up to St = 1
        let kmax = ((1.0 / scale) * nfft as f64).ceil() as usize;
        let kmax = kmax.clamp(8, half);
        let stride = (kmax / 256).max(1);
        let pmax = pw[k0.min(kmax)..kmax].iter().cloned().fold(1e-300, f64::max);
        for k in (1..kmax).step_by(stride) {
            out.st_axis.push(k as f64 / nfft as f64 * scale);
            out.power.push(pw[k] / pmax);
        }
        out
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SpectrumOut {
    pub st: Option<f64>,
    pub freq_per_step: Option<f64>,
    pub st_axis: Vec<f64>,
    pub power: Vec<f64>,
    pub samples: usize,
    pub periods: f64,
    pub note: String,
}

// ------------------------------------------------------------------ sessions (stateful API)

thread_local! {
    static SESSIONS: RefCell<Vec<(u32, Lbm)>> = const { RefCell::new(Vec::new()) };
    static NEXT: RefCell<u32> = const { RefCell::new(1) };
}
const MAX_SESSIONS: usize = 3;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct InitIn {
    pub config: LbmConfig,
    /// Replace this session (freed first).
    pub replace: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct InitOut {
    pub session: u32,
    pub info: LbmInfo,
    /// Obstacle mask, base64 (nx·ny bytes, row-major from the bottom row).
    pub mask: String,
}

pub fn init(i: InitIn) -> Result<InitOut, String> {
    let s = Lbm::new(i.config)?;
    let info = s.info.clone();
    let mask = crate::cad::export::base64(&s.mask);
    let id = NEXT.with(|n| {
        let v = *n.borrow();
        *n.borrow_mut() = v.wrapping_add(1).max(1);
        v
    });
    SESSIONS.with(|ss| {
        let mut ss = ss.borrow_mut();
        if let Some(r) = i.replace {
            ss.retain(|(k, _)| *k != r);
        }
        while ss.len() >= MAX_SESSIONS {
            ss.remove(0);
        }
        ss.push((id, s));
    });
    Ok(InitOut { session: id, info, mask })
}

fn with_session<T>(id: u32, f: impl FnOnce(&mut Lbm) -> Result<T, String>) -> Result<T, String> {
    SESSIONS.with(|ss| {
        let mut ss = ss.borrow_mut();
        match ss.iter_mut().find(|(k, _)| *k == id) {
            Some((_, s)) => f(s),
            None => Err(format!("no wind-tunnel session {id} (it may have been replaced) — start a new one")),
        }
    })
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct StepIn {
    pub session: u32,
    pub steps: usize,
    /// speed | vorticity | pressure | ux
    pub field: String,
    pub contrast: f64,
    pub tracers: bool,
    /// Return the force/probe history downsampled to this many points (0 = none).
    pub history_points: usize,
    pub spectrum: bool,
    pub frame: bool,
}

impl Default for StepIn {
    fn default() -> Self {
        StepIn {
            session: 0,
            steps: 20,
            field: "vorticity".into(),
            contrast: 1.0,
            tracers: true,
            history_points: 0,
            spectrum: false,
            frame: true,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct History {
    pub step: Vec<f64>,
    pub cd: Vec<f64>,
    pub cl: Vec<f64>,
    pub probe: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StepOut {
    pub step: u64,
    pub frame: Option<String>,
    pub lo: f64,
    pub hi: f64,
    pub cd: f64,
    pub cl: f64,
    /// Means over the last 4000 steps.
    pub cd_mean: f64,
    pub cl_mean: f64,
    pub cl_amp: f64,
    pub mass: f64,
    pub max_speed: f64,
    pub unstable: bool,
    /// Tracer positions [x0, y0, x1, y1, …] in cells.
    pub tracers: Vec<f32>,
    pub history: Option<History>,
    pub spectrum: Option<SpectrumOut>,
}

fn downsample(v: &[f64], idx: &[usize]) -> Vec<f64> {
    idx.iter().map(|&i| v[i]).collect()
}

pub fn step_session(i: StepIn) -> Result<StepOut, String> {
    with_session(i.session, |s| {
        s.run(i.steps.min(5000));
        Ok(snapshot(s, &i))
    })
}

fn snapshot(s: &Lbm, i: &StepIn) -> StepOut {
    let (frame, lo, hi) = if i.frame {
        let (b, lo, hi) = s.frame(&i.field, if i.contrast > 0.0 { i.contrast } else { 1.0 });
        (Some(crate::cad::export::base64(&b)), lo, hi)
    } else {
        (None, 0.0, 0.0)
    };
    let (cd_mean, cl_mean) = s.mean_coeffs(4000);
    let n = s.hist_cl.len();
    let a = n.saturating_sub(4000);
    let cl_amp = if n > a {
        let mx = s.hist_cl[a..].iter().cloned().fold(f64::MIN, f64::max);
        let mn = s.hist_cl[a..].iter().cloned().fold(f64::MAX, f64::min);
        0.5 * (mx - mn)
    } else {
        0.0
    };
    let history = (i.history_points > 0 && n > 0).then(|| {
        let m = i.history_points.clamp(10, 4000).min(n);
        let idx: Vec<usize> = (0..m).map(|k| k * (n - 1) / (m - 1).max(1)).collect();
        History {
            step: idx.iter().map(|&k| s.hist_step[k] as f64).collect(),
            cd: downsample(&s.hist_cd, &idx),
            cl: downsample(&s.hist_cl, &idx),
            probe: downsample(&s.hist_probe, &idx),
        }
    });
    let max_speed = s.ux.iter().zip(&s.uy).map(|(u, v)| (u * u + v * v).sqrt()).fold(0.0, f64::max);
    StepOut {
        step: s.step,
        frame,
        lo,
        hi,
        cd: s.hist_cd.last().copied().unwrap_or(0.0),
        cl: s.hist_cl.last().copied().unwrap_or(0.0),
        cd_mean,
        cl_mean,
        cl_amp,
        mass: s.mass(),
        max_speed,
        unstable: s.unstable,
        tracers: if i.tracers {
            s.tracers
                .iter()
                .flat_map(|t| [(t[0] * 10.0).round() as f32 / 10.0, (t[1] * 10.0).round() as f32 / 10.0])
                .collect()
        } else {
            vec![]
        },
        history,
        spectrum: i.spectrum.then(|| s.spectrum()),
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct PaintIn {
    pub session: u32,
    /// Stroke points in cell coordinates.
    pub points: Vec<[f64; 2]>,
    pub radius: f64,
    /// true = add obstacle, false = erase.
    pub solid: bool,
    /// Remove every obstacle first.
    pub clear: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PaintOut {
    pub mask: String,
    pub solid_cells: usize,
}

pub fn paint(i: PaintIn) -> Result<PaintOut, String> {
    with_session(i.session, |s| {
        let (nx, ny) = (s.nx, s.ny);
        let n = nx * ny;
        let set = |s: &mut Lbm, k: usize, solid: bool| {
            if (s.mask[k] != 0) == solid {
                return;
            }
            s.mask[k] = solid as u8;
            let ux = if solid { 0.0 } else { 0.5 * s.u_ref };
            for q in 0..9 {
                s.f[q * n + k] = feq(q, 1.0, ux, 0.0);
            }
        };
        if i.clear {
            for k in 0..n {
                set(s, k, false);
            }
        }
        let r = i.radius.clamp(0.5, 40.0);
        for p in &i.points {
            let (x0, y0) = (p[0], p[1]);
            let i0 = (x0 - r).floor().max(0.0) as usize;
            let i1 = ((x0 + r).ceil().max(0.0) as usize).min(nx);
            let j0 = (y0 - r).floor().max(0.0) as usize;
            let j1 = ((y0 + r).ceil().max(0.0) as usize).min(ny);
            for j in j0..j1 {
                for ii in i0..i1 {
                    let (dx, dy) = (ii as f64 + 0.5 - x0, j as f64 + 0.5 - y0);
                    if dx * dx + dy * dy <= r * r {
                        // keep the inlet and outlet columns fluid
                        if ii == 0 || ii + 1 >= nx {
                            continue;
                        }
                        set(s, ii + nx * j, i.solid);
                    }
                }
            }
        }
        s.macros();
        s.info.solid_cells = s.mask.iter().filter(|m| **m != 0).count();
        Ok(PaintOut {
            mask: crate::cad::export::base64(&s.mask),
            solid_cells: s.info.solid_cells,
        })
    })
}

// ------------------------------------------------------------------ stateless batch run

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct RunIn {
    pub config: LbmConfig,
    pub steps: usize,
    /// Keep a frame every this many steps (0 = only the last).
    pub frame_every: usize,
    pub field: String,
}

impl Default for RunIn {
    fn default() -> Self {
        RunIn {
            config: LbmConfig::default(),
            steps: 2000,
            frame_every: 0,
            field: "vorticity".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RunOut {
    pub info: LbmInfo,
    pub mask: String,
    pub frames: Vec<String>,
    pub frame_steps: Vec<u64>,
    pub last: StepOut,
}

pub fn run(i: RunIn) -> Result<RunOut, String> {
    if i.steps > 200_000 {
        return Err("at most 200 000 steps per batch run".into());
    }
    let mut s = Lbm::new(i.config)?;
    let mut frames = Vec::new();
    let mut frame_steps = Vec::new();
    let every = if i.frame_every == 0 { i.steps.max(1) } else { i.frame_every };
    let mut done = 0;
    while done < i.steps {
        let k = every.min(i.steps - done);
        s.run(k);
        done += k;
        if i.frame_every > 0 && frames.len() < 200 {
            frames.push(crate::cad::export::base64(&s.frame(&i.field, 1.0).0));
            frame_steps.push(s.step);
        }
        if s.unstable {
            break;
        }
    }
    let last = snapshot(
        &s,
        &StepIn {
            field: i.field.clone(),
            history_points: 400,
            spectrum: true,
            ..Default::default()
        },
    );
    Ok(RunOut {
        info: s.info.clone(),
        mask: crate::cad::export::base64(&s.mask),
        frames,
        frame_steps,
        last,
    })
}

#[derive(Clone, Debug, Serialize)]
pub struct Preset {
    pub id: String,
    pub label: String,
    pub description: String,
    pub config: LbmConfig,
    /// Published value to compare with, if any.
    pub reference: Option<String>,
}

pub fn presets() -> Vec<Preset> {
    let base = LbmConfig::default();
    vec![
        Preset {
            id: "karman".into(),
            label: "Kármán vortex street (cylinder, Re 150)".into(),
            description: "A circular cylinder in open flow sheds alternating vortices once Re exceeds about 47.".into(),
            config: base.clone(),
            reference: Some(
                "Unconfined cylinder: St ≈ 0.18–0.19 at Re 150 (Williamson 1996); this tunnel has 18 % blockage, which raises St and C_D."
                    .into(),
            ),
        },
        Preset {
            id: "schafer_turek_20".into(),
            label: "Schäfer–Turek 2D-1 (Re 20, steady)".into(),
            description: "Benchmark channel: parabolic inlet, no-slip walls, H = 4.1 D, cylinder slightly off the centre line.".into(),
            config: LbmConfig {
                nx: 440,
                ny: 82,
                size_cells: 20.0,
                x_frac: 40.0 / 440.0,
                y_frac: 40.0 / 82.0,
                re: 20.0,
                u_in: 0.05,
                walls: "noslip".into(),
                inlet: "parabolic".into(),
                perturb: 0.0,
                ..base.clone()
            },
            reference: Some("C_D = 5.57–5.59, C_L = 0.0104–0.0110 (Schäfer & Turek 1996, based on the mean inlet speed)".into()),
        },
        Preset {
            id: "schafer_turek_100".into(),
            label: "Schäfer–Turek 2D-2 (Re 100, periodic)".into(),
            description: "Same channel at Re 100: periodic shedding.".into(),
            config: LbmConfig {
                nx: 440,
                ny: 82,
                size_cells: 20.0,
                x_frac: 40.0 / 440.0,
                y_frac: 40.0 / 82.0,
                re: 100.0,
                u_in: 0.06,
                walls: "noslip".into(),
                inlet: "parabolic".into(),
                perturb: 0.0,
                ..base.clone()
            },
            reference: Some("C_D,max = 3.22–3.24, C_L,max = 0.99–1.01, St = 0.295–0.305 (Schäfer & Turek 1996)".into()),
        },
        Preset {
            id: "temple".into(),
            label: "Iris temple in a side wind".into(),
            description: "The real temple cross-section (width × height from the project), wind blowing across it.".into(),
            config: LbmConfig {
                obstacle: "temple".into(),
                size_cells: 24.0,
                re: 200.0,
                ..base.clone()
            },
            reference: None,
        },
        Preset {
            id: "pod".into(),
            label: "Iris electronics pod in a side wind".into(),
            description: "The pod cross-section: a bluff rounded box sheds a wide wake.".into(),
            config: LbmConfig {
                obstacle: "pod".into(),
                size_cells: 22.0,
                re: 150.0,
                ..base.clone()
            },
            reference: None,
        },
        Preset {
            id: "naca".into(),
            label: "NACA 2412 airfoil at 6°".into(),
            description: "A streamlined shape: small drag, steady lift; raise the angle to see it stall and shed.".into(),
            config: LbmConfig {
                obstacle: "naca".into(),
                naca: "2412".into(),
                size_cells: 70.0,
                angle_deg: 6.0,
                x_frac: 0.25,
                re: 400.0,
                u_in: 0.08,
                ..base.clone()
            },
            reference: None,
        },
        Preset {
            id: "plate".into(),
            label: "Flat plate across the flow".into(),
            description: "The bluffest shape: sharp edges fix the separation points.".into(),
            config: LbmConfig {
                obstacle: "rectangle".into(),
                aspect: 0.15,
                size_cells: 20.0,
                re: 150.0,
                ..base.clone()
            },
            reference: None,
        },
        Preset {
            id: "draw".into(),
            label: "Empty tunnel — draw your own".into(),
            description: "Tap or drag on the tunnel to add obstacles, switch to erase to remove them.".into(),
            config: LbmConfig {
                obstacle: "none".into(),
                ..base
            },
            reference: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip() {
        for n in 0..12usize {
            let v: Vec<u8> = (0..n as u8).map(|x| x.wrapping_mul(37)).collect();
            assert_eq!(base64_decode(&crate::cad::export::base64(&v)).unwrap(), v);
        }
        assert!(base64_decode("@@@@").is_none());
    }

    #[test]
    fn equilibrium_moments() {
        let (r, u, v) = (1.1, 0.05, -0.02);
        let m0: f64 = (0..9).map(|q| feq(q, r, u, v)).sum();
        let mx: f64 = (0..9).map(|q| feq(q, r, u, v) * CX[q] as f64).sum();
        let my: f64 = (0..9).map(|q| feq(q, r, u, v) * CY[q] as f64).sum();
        assert!((m0 - r).abs() < 1e-14);
        assert!((mx - r * u).abs() < 1e-14);
        assert!((my - r * v).abs() < 1e-14);
        for q in 0..9 {
            assert_eq!(CX[OPP[q]], -CX[q]);
            assert_eq!(CY[OPP[q]], -CY[q]);
            assert_eq!(CX[MIRROR_Y[q]], CX[q]);
            assert_eq!(CY[MIRROR_Y[q]], -CY[q]);
        }
    }

    #[test]
    fn poiseuille_profile_is_parabolic() {
        // Body-force-driven periodic channel, half-way bounce-back walls at y = 0 and y = H:
        // u(y) = F/(2ν) · y (H − y), with y measured from the wall (node j at y = j + ½).
        let ny = 24;
        let tau = 0.8;
        let nu = (tau - 0.5) / 3.0;
        let force = 1e-6;
        let cfg = LbmConfig {
            nx: 4,
            ny,
            obstacle: "none".into(),
            periodic_x: true,
            walls: "noslip".into(),
            body_force: force,
            u_in: 0.0,
            tracers: 0,
            perturb: 0.0,
            ..Default::default()
        };
        let mut s = Lbm::new(cfg).unwrap();
        s.tau = tau;
        s.run(12_000);
        let h = ny as f64;
        let mut err2 = 0.0;
        let mut ref2 = 0.0;
        for j in 0..ny {
            let y = j as f64 + 0.5;
            let exact = force / (2.0 * nu) * y * (h - y);
            let u = s.ux[1 + s.nx * j];
            err2 += (u - exact).powi(2);
            ref2 += exact * exact;
        }
        let rel = (err2 / ref2).sqrt();
        assert!(rel < 0.01, "relative L2 error {rel}");
        // centre-line maximum F H² / (8 ν)
        let umax = force * h * h / (8.0 * nu);
        let uc = 0.5 * (s.ux[1 + s.nx * (ny / 2 - 1)] + s.ux[1 + s.nx * (ny / 2)]);
        assert!((uc - umax).abs() / umax < 0.01, "{uc} vs {umax}");
    }

    #[test]
    fn mass_is_conserved_with_bounce_back() {
        let cfg = LbmConfig {
            nx: 64,
            ny: 32,
            obstacle: "cylinder".into(),
            size_cells: 8.0,
            x_frac: 0.4,
            y_frac: 0.45,
            periodic_x: true,
            walls: "noslip".into(),
            body_force: 2e-5,
            u_in: 0.05,
            re: 50.0,
            tracers: 10,
            ..Default::default()
        };
        let mut s = Lbm::new(cfg).unwrap();
        let m0 = s.mass();
        s.run(800);
        let m1 = s.mass();
        assert!(((m1 - m0) / m0).abs() < 1e-12, "{m0} → {m1}");
        // free-slip walls conserve mass too
        let mut s2 = Lbm::new(LbmConfig {
            nx: 48,
            ny: 24,
            periodic_x: true,
            walls: "slip".into(),
            size_cells: 6.0,
            body_force: 1e-5,
            tracers: 0,
            ..Default::default()
        })
        .unwrap();
        let a = s2.mass();
        s2.run(400);
        assert!(((s2.mass() - a) / a).abs() < 1e-12);
    }

    #[test]
    fn schafer_turek_re20_drag() {
        // Benchmark 2D-1 at a coarse 10 cells per diameter: C_D reference 5.58.
        // The staircase cylinder reads ≈ 8 % high at this resolution (≈ 6.0);
        // the page's 20-cell preset comes closer.
        let d = 10.0;
        let cfg = LbmConfig {
            nx: 220,
            ny: 41,
            obstacle: "cylinder".into(),
            size_cells: d,
            x_frac: 20.0 / 220.0,
            y_frac: 20.0 / 41.0,
            re: 20.0,
            u_in: 0.05,
            walls: "noslip".into(),
            inlet: "parabolic".into(),
            tracers: 0,
            perturb: 0.0,
            ..Default::default()
        };
        let mut s = Lbm::new(cfg).unwrap();
        s.run(4000);
        // average over the last 2000 steps to remove the start-up pressure waves
        let (cd, cl) = s.mean_coeffs(2000);
        assert!(!s.unstable);
        assert!((cd - 5.58).abs() / 5.58 < 0.1, "C_D = {cd}");
        assert!(cl.abs() < 0.2, "C_L = {cl}");
    }

    #[test]
    fn spectrum_finds_a_sine() {
        let mut s = Lbm::new(LbmConfig {
            nx: 16,
            ny: 8,
            obstacle: "none".into(),
            tracers: 0,
            ..Default::default()
        })
        .unwrap();
        let f = 1.0 / 400.0;
        for k in 0..8192 {
            s.hist_probe.push((2.0 * std::f64::consts::PI * f * k as f64).sin());
        }
        let sp = s.spectrum();
        let fs = sp.freq_per_step.unwrap();
        assert!((fs - f).abs() / f < 0.01, "{fs}");
        assert!((sp.st.unwrap() - f * s.d / s.u_ref).abs() < 1e-3);
    }

    #[test]
    fn naca_shape_and_masks() {
        let p = naca4("0012", 40);
        let tmax = p.iter().map(|q| q[1]).fold(0.0, f64::max);
        assert!((tmax - 0.06).abs() < 0.002, "{tmax}");
        for id in ["cylinder", "naca", "temple", "pod", "rectangle"] {
            let s = Lbm::new(LbmConfig {
                obstacle: id.into(),
                tracers: 0,
                ..Default::default()
            })
            .unwrap();
            assert!(s.info.solid_cells > 25, "{id}: {}", s.info.solid_cells);
        }
        // cylinder area ≈ π D²/4
        let s = Lbm::new(LbmConfig {
            tracers: 0,
            ..Default::default()
        })
        .unwrap();
        let a = std::f64::consts::PI * 100.0;
        assert!((s.info.solid_cells as f64 - a).abs() / a < 0.05);
    }

    #[test]
    fn sessions_step_and_paint() {
        let o = init(InitIn {
            config: LbmConfig {
                nx: 80,
                ny: 40,
                size_cells: 8.0,
                tracers: 20,
                ..Default::default()
            },
            replace: None,
        })
        .unwrap();
        let st = step_session(StepIn {
            session: o.session,
            steps: 50,
            history_points: 20,
            spectrum: true,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(st.step, 50);
        assert!(st.frame.is_some());
        assert_eq!(st.tracers.len(), 40);
        assert!(st.cd.is_finite());
        let pr = paint(PaintIn {
            session: o.session,
            points: vec![[50.0, 20.0]],
            radius: 3.0,
            solid: true,
            clear: false,
        })
        .unwrap();
        assert!(pr.solid_cells > 50);
        assert!(step_session(StepIn {
            session: 999_999,
            ..Default::default()
        })
        .is_err());
    }
}
