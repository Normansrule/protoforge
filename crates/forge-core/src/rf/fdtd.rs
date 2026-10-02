//! Two-dimensional transverse-magnetic (TMz) finite-difference time-domain
//! (FDTD) solver on a Yee grid with a Bérenger split-field perfectly matched
//! layer (PML).
//!
//! **Yee scheme** (Yee 1966; Taflove & Hagness, *Computational
//! Electrodynamics*, ch. 3). The TMz fields are E_z, H_x, H_y; E_z sits on
//! integer nodes (i, j), H_x on (i, j+½), H_y on (i+½, j), and E and H are
//! staggered by half a time step (leapfrog):
//!
//! ```text
//! H_x ← H_x − Δt/(μ0 Δ) · (E_z[i,j+1] − E_z[i,j])
//! H_y ← H_y + Δt/(μ0 Δ) · (E_z[i+1,j] − E_z[i,j])
//! E_z ← C_a·E_z + C_b·[(H_y[i] − H_y[i−1]) − (H_x[j] − H_x[j−1])]
//! C_a = (1 − σΔt/2ε)/(1 + σΔt/2ε),  C_b = (Δt/εΔ)/(1 + σΔt/2ε)
//! ```
//!
//! **Stability (Courant–Friedrichs–Lewy)**: c·Δt·√(1/Δx² + 1/Δy²) ≤ 1, i.e.
//! Δt ≤ Δ/(c√2) on a square 2-D grid (Taflove & Hagness §4.7). The solver
//! runs at a Courant number S = 0.99 of that limit by default.
//!
//! **PML** (Bérenger 1994): E_z is split into E_zx + E_zy; each part is
//! attenuated only by the conductivity normal to its own direction, and the
//! magnetic conductivity σ* = σ·μ0/ε0 makes the layer reflectionless at
//! every angle in the continuum. The conductivity is graded as
//! σ(ρ) = σ_max·(ρ/N)^m with m = 3 and σ_max = −(m+1)·ln(R0)/(2η0·N·Δ) for a
//! target normal-incidence reflection R0 = 10⁻⁶; exponential time
//! differencing is used inside the layer.
//!
//! **What 2-D TMz means physically**: everything is infinitely long in z. A
//! point source is an infinite line current along z — the H-plane cut of a
//! vertical dipole (omnidirectional in the plane). A PEC line is an infinite
//! metal sheet seen edge-on.

use serde::{Deserialize, Serialize};

use super::tline::{C0, EPS0, ETA0, MU0};

const PI: f64 = std::f64::consts::PI;

/// Material description of the grid before it is turned into coefficients.
pub struct Scene {
    pub nx: usize,
    pub ny: usize,
    pub eps_r: Vec<f64>,
    pub sigma: Vec<f64>,
    pub pec: Vec<bool>,
}

impl Scene {
    pub fn vacuum(nx: usize, ny: usize) -> Scene {
        Scene {
            nx,
            ny,
            eps_r: vec![1.0; nx * ny],
            sigma: vec![0.0; nx * ny],
            pec: vec![false; nx * ny],
        }
    }
    fn id(&self, i: usize, j: usize) -> usize {
        j * self.nx + i
    }
    pub fn set_pec(&mut self, i: usize, j: usize) {
        if i < self.nx && j < self.ny {
            let k = self.id(i, j);
            self.pec[k] = true;
        }
    }
    pub fn disc(&mut self, cx: f64, cy: f64, r: f64, er: f64, sig: f64) {
        for j in 0..self.ny {
            for i in 0..self.nx {
                if (i as f64 - cx).hypot(j as f64 - cy) <= r {
                    let k = self.id(i, j);
                    self.eps_r[k] = er;
                    self.sigma[k] = sig;
                }
            }
        }
    }
}

/// The time-stepping state.
pub struct Sim {
    pub nx: usize,
    pub ny: usize,
    pub dx: f64,
    pub dt: f64,
    pub pml: usize,
    pub ez: Vec<f64>,
    ezx: Vec<f64>,
    ezy: Vec<f64>,
    pub hx: Vec<f64>,
    pub hy: Vec<f64>,
    cax: Vec<f64>,
    cbx: Vec<f64>,
    cay: Vec<f64>,
    cby: Vec<f64>,
    chx: Vec<f64>,
    dhx: Vec<f64>,
    chy: Vec<f64>,
    dhy: Vec<f64>,
    pub eps_r: Vec<f64>,
    pub sigma: Vec<f64>,
    pub pec: Vec<bool>,
}

/// Largest stable time step for a square 2-D grid of pitch `dx` (s).
pub fn dt_max(dx: f64) -> f64 {
    dx / (C0 * 2f64.sqrt())
}

fn pml_sigma(pos: f64, n: usize, npml: usize, dx: f64) -> f64 {
    if npml == 0 {
        return 0.0;
    }
    let np = npml as f64;
    let depth = (np - pos).max(pos - (n as f64 - 1.0 - np)).max(0.0);
    if depth <= 0.0 {
        return 0.0;
    }
    let m = 3.0;
    let r0: f64 = 1e-6;
    let smax = -(m + 1.0) * r0.ln() / (2.0 * ETA0 * np * dx);
    smax * (depth / np).min(1.0).powf(m)
}

/// Exponential-differencing coefficients for a conductivity σ (electric units).
fn exp_coeff(sig: f64, dt: f64, dx: f64, mu: bool) -> (f64, f64) {
    let a = (-sig * dt / EPS0).exp();
    let unit = if mu { MU0 } else { EPS0 };
    if sig * dt / EPS0 < 1e-9 {
        (1.0, dt / (unit * dx))
    } else {
        // (1 − a)/(σ_eff·Δ) with σ_eff = σ for E and σ* = σμ0/ε0 for H
        let seff = if mu { sig * MU0 / EPS0 } else { sig };
        (a, (1.0 - a) / (seff * dx))
    }
}

impl Sim {
    pub fn new(scene: Scene, dx: f64, courant: f64, pml: usize) -> Sim {
        let (nx, ny) = (scene.nx, scene.ny);
        let dt = courant * dt_max(dx);
        let n = nx * ny;
        let (mut cax, mut cbx, mut cay, mut cby) = (vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n]);
        for j in 0..ny {
            for i in 0..nx {
                let k = j * nx + i;
                if scene.pec[k] {
                    continue; // all zero → E_z pinned to 0
                }
                let sx = pml_sigma(i as f64, nx, pml, dx);
                let sy = pml_sigma(j as f64, ny, pml, dx);
                let in_pml = pml > 0 && (i < pml || j < pml || i >= nx - pml || j >= ny - pml);
                if in_pml {
                    let (a, b) = exp_coeff(sx, dt, dx, false);
                    cax[k] = a;
                    cbx[k] = b;
                    let (a, b) = exp_coeff(sy, dt, dx, false);
                    cay[k] = a;
                    cby[k] = b;
                } else {
                    let eps = EPS0 * scene.eps_r[k];
                    let l = scene.sigma[k] * dt / (2.0 * eps);
                    let ca = (1.0 - l) / (1.0 + l);
                    let cb = dt / (eps * dx) / (1.0 + l);
                    cax[k] = ca;
                    cay[k] = ca;
                    cbx[k] = cb;
                    cby[k] = cb;
                }
            }
        }
        let (mut chx, mut dhx) = (vec![0.0; ny], vec![0.0; ny]);
        for j in 0..ny {
            let (a, b) = exp_coeff(pml_sigma(j as f64 + 0.5, ny, pml, dx), dt, dx, true);
            chx[j] = a;
            dhx[j] = b;
        }
        let (mut chy, mut dhy) = (vec![0.0; nx], vec![0.0; nx]);
        for i in 0..nx {
            let (a, b) = exp_coeff(pml_sigma(i as f64 + 0.5, nx, pml, dx), dt, dx, true);
            chy[i] = a;
            dhy[i] = b;
        }
        Sim {
            nx,
            ny,
            dx,
            dt,
            pml,
            ez: vec![0.0; n],
            ezx: vec![0.0; n],
            ezy: vec![0.0; n],
            hx: vec![0.0; n],
            hy: vec![0.0; n],
            cax,
            cbx,
            cay,
            cby,
            chx,
            dhx,
            chy,
            dhy,
            eps_r: scene.eps_r,
            sigma: scene.sigma,
            pec: scene.pec,
        }
    }

    /// One full leapfrog step: H at n+½, then E at n+1.
    pub fn step(&mut self) {
        let (nx, ny) = (self.nx, self.ny);
        for j in 0..ny - 1 {
            let (c, d) = (self.chx[j], self.dhx[j]);
            let r = j * nx;
            for i in 0..nx {
                let k = r + i;
                self.hx[k] = c * self.hx[k] - d * (self.ez[k + nx] - self.ez[k]);
            }
        }
        for j in 0..ny {
            let r = j * nx;
            for i in 0..nx - 1 {
                let k = r + i;
                self.hy[k] = self.chy[i] * self.hy[k] + self.dhy[i] * (self.ez[k + 1] - self.ez[k]);
            }
        }
        for j in 1..ny - 1 {
            let r = j * nx;
            for i in 1..nx - 1 {
                let k = r + i;
                let dhy = self.hy[k] - self.hy[k - 1];
                let dhx = self.hx[k] - self.hx[k - nx];
                self.ezx[k] = self.cax[k] * self.ezx[k] + self.cbx[k] * dhy;
                self.ezy[k] = self.cay[k] * self.ezy[k] - self.cby[k] * dhx;
                self.ez[k] = self.ezx[k] + self.ezy[k];
            }
        }
    }

    /// Soft (additive) current source at a node.
    pub fn inject(&mut self, i: usize, j: usize, s: f64) {
        if i == 0 || j == 0 || i >= self.nx - 1 || j >= self.ny - 1 {
            return;
        }
        let k = j * self.nx + i;
        if self.pec[k] {
            return;
        }
        self.ezx[k] += 0.5 * s;
        self.ezy[k] += 0.5 * s;
        self.ez[k] = self.ezx[k] + self.ezy[k];
    }

    /// Hard-wall initial condition used by tests: set E_z directly.
    pub fn set_ez(&mut self, i: usize, j: usize, v: f64) {
        let k = j * self.nx + i;
        self.ezx[k] = 0.5 * v;
        self.ezy[k] = 0.5 * v;
        self.ez[k] = v;
    }

    /// Electromagnetic energy per metre in z inside the physical region (J/m).
    pub fn energy(&self) -> f64 {
        let (nx, ny, p) = (self.nx, self.ny, self.pml);
        let mut w = 0.0;
        for j in p..ny - p {
            for i in p..nx - p {
                let k = j * nx + i;
                w += 0.5 * EPS0 * self.eps_r[k] * self.ez[k] * self.ez[k] + 0.5 * MU0 * (self.hx[k] * self.hx[k] + self.hy[k] * self.hy[k]);
            }
        }
        w * self.dx * self.dx
    }

    /// Instantaneous power dissipated in lossy material (W/m).
    pub fn absorbed(&self) -> f64 {
        let (nx, ny, p) = (self.nx, self.ny, self.pml);
        let mut s = 0.0;
        for j in p..ny - p {
            for i in p..nx - p {
                let k = j * nx + i;
                if self.sigma[k] > 0.0 {
                    s += self.sigma[k] * self.ez[k] * self.ez[k];
                }
            }
        }
        s * self.dx * self.dx
    }

    /// Instantaneous outward Poynting flux through a rectangle `inset` cells
    /// inside the domain edge (W/m).
    pub fn outflow(&self, inset: usize) -> f64 {
        let (nx, ny) = (self.nx, self.ny);
        let (i0, i1, j0, j1) = (inset, nx - 1 - inset, inset, ny - 1 - inset);
        let e = |i: usize, j: usize| self.ez[j * nx + i];
        let hy = |i: usize, j: usize| 0.5 * (self.hy[j * nx + i] + self.hy[j * nx + i - 1]);
        let hx = |i: usize, j: usize| 0.5 * (self.hx[j * nx + i] + self.hx[(j - 1) * nx + i]);
        let mut p = 0.0;
        for j in j0..=j1 {
            p += -e(i1, j) * hy(i1, j); // +x face: Sx = −Ez Hy
            p += e(i0, j) * hy(i0, j); // −x face
        }
        for i in i0..=i1 {
            p += e(i, j1) * hx(i, j1); // +y face: Sy = Ez Hx
            p += -e(i, j0) * hx(i, j0); // −y face
        }
        p * self.dx
    }
}

// ------------------------------------------------------------------ presets

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FdtdIn {
    /// dipole | array | head | ground | slit | waveguide
    pub preset: String,
    pub nx: usize,
    pub ny: usize,
    pub dx_mm: f64,
    pub f_ghz: f64,
    pub steps: usize,
    pub frames: usize,
    /// Fraction of the 2-D Courant limit.
    pub courant: f64,
    pub pml: usize,
    /// Source / probe position in cells (defaults per preset).
    pub source: Option<[f64; 2]>,
    pub probe: Option<[f64; 2]>,
    /// Gaussian pulse instead of a ramped continuous wave.
    pub pulse: bool,
    /// Head phantom (IEEE 1528 / IEC 62209 head tissue-simulating liquid at 2450 MHz: εr 39.2, σ 1.80 S/m).
    pub tissue_er: f64,
    pub tissue_sigma: f64,
    pub head_radius_mm: f64,
    /// Antenna-to-head gap, mm.
    pub gap_mm: f64,
    pub slit_mm: f64,
    pub guide_mm: f64,
    pub height_mm: f64,
    pub array_n: usize,
    pub array_spacing_wl: f64,
    pub array_phase_deg: f64,
    /// Display gamma applied before quantising (0.5 lifts weak far fields).
    pub gamma: f64,
}

impl Default for FdtdIn {
    fn default() -> Self {
        FdtdIn {
            preset: "dipole".into(),
            nx: 180,
            ny: 140,
            dx_mm: 1.5,
            f_ghz: 2.44,
            steps: 1000,
            frames: 100,
            courant: 0.99,
            pml: 12,
            source: None,
            probe: None,
            pulse: false,
            tissue_er: 39.2,
            tissue_sigma: 1.80,
            head_radius_mm: 75.0,
            gap_mm: 10.0,
            slit_mm: 40.0,
            guide_mm: 80.0,
            height_mm: 30.7,
            array_n: 2,
            array_spacing_wl: 0.5,
            array_phase_deg: 0.0,
            gamma: 0.5,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Legend {
    pub id: u8,
    pub name: String,
    pub eps_r: f64,
    pub sigma: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FdtdOut {
    pub preset: String,
    pub nx: usize,
    pub ny: usize,
    pub dx_mm: f64,
    pub dt_ps: f64,
    pub dt_max_ps: f64,
    pub courant: f64,
    pub stable: bool,
    pub cells_per_wavelength: f64,
    pub cells_per_wavelength_min: f64,
    pub pml: usize,
    pub steps: usize,
    /// base64 of `n_frames × nx × ny` bytes (row-major, j = row), 128 = zero field.
    pub frames_b64: String,
    pub n_frames: usize,
    pub frame_time_ns: Vec<f64>,
    pub frame_step: Vec<usize>,
    /// base64 of `nx × ny` material ids (0 air, 1 dielectric/tissue, 2 metal).
    pub material_b64: String,
    pub legend: Vec<Legend>,
    pub scale: f64,
    pub gamma: f64,
    pub sources: Vec<[usize; 2]>,
    pub probe: [usize; 2],
    pub probe_t_ns: Vec<f64>,
    pub probe_ez: Vec<f64>,
    pub energy: Vec<f64>,
    /// Steady-state CW power balance (time-averaged over the last two periods).
    pub absorbed_fraction: Option<f64>,
    pub absorbed_db: Option<f64>,
    pub notes: Vec<String>,
    pub ms_hint: f64,
}

struct Built {
    scene: Scene,
    legend: Vec<Legend>,
    sources: Vec<([usize; 2], f64)>, // (cell, phase rad)
    probe: [usize; 2],
    notes: Vec<String>,
}

fn clampc(v: f64, n: usize, pml: usize) -> usize {
    (v.round().max((pml + 2) as f64) as usize).min(n - pml - 3)
}

fn build(i: &FdtdIn, lam_cells: f64) -> Built {
    let (nx, ny, p) = (i.nx, i.ny, i.pml);
    let mut s = Scene::vacuum(nx, ny);
    let (cx, cy) = (nx as f64 / 2.0, ny as f64 / 2.0);
    let mm = |v: f64| v / i.dx_mm;
    let mut legend = vec![Legend {
        id: 0,
        name: "Air".into(),
        eps_r: 1.0,
        sigma: 0.0,
    }];
    let pec_legend = Legend {
        id: 2,
        name: "Metal (perfect electric conductor)".into(),
        eps_r: 1.0,
        sigma: f64::INFINITY,
    };
    let mut notes = Vec::new();
    let mut src: Vec<([usize; 2], f64)> = Vec::new();
    let user = i.source.map(|v| [clampc(v[0], nx, p), clampc(v[1], ny, p)]);
    let probe_default;
    match i.preset.as_str() {
        "array" => {
            let n = i.array_n.clamp(1, 8);
            let d = i.array_spacing_wl * lam_cells;
            let (sx, sy) = user.map(|u| (u[0] as f64, u[1] as f64)).unwrap_or((cx, cy));
            for k in 0..n {
                let x = sx + (k as f64 - (n as f64 - 1.0) / 2.0) * d;
                src.push(([clampc(x, nx, p), clampc(sy, ny, p)], k as f64 * i.array_phase_deg.to_radians()));
            }
            probe_default = [cx, cy - 0.35 * ny as f64];
            notes.push(format!(
                "{n} line sources {:.2} λ apart with a {:.0}° phase step: the in-plane pattern is the array factor of the Antennas tab (H-plane cut).",
                i.array_spacing_wl, i.array_phase_deg
            ));
        }
        "head" => {
            let r = mm(i.head_radius_mm);
            let hx = nx as f64 * 0.64;
            s.disc(hx, cy, r, i.tissue_er, i.tissue_sigma);
            legend.push(Legend {
                id: 1,
                name: "Head tissue".into(),
                eps_r: i.tissue_er,
                sigma: i.tissue_sigma,
            });
            let a = user.unwrap_or([clampc(hx - r - mm(i.gap_mm), nx, p), clampc(cy, ny, p)]);
            src.push((a, 0.0));
            probe_default = [hx + r + mm(12.0), cy];
            let skin = (2.0 / (2.0 * PI * i.f_ghz * 1e9 * MU0 * i.tissue_sigma)).sqrt() * 1e3;
            notes.push(format!(
                "Homogeneous head phantom (radius {:.0} mm, εr {:.1}, σ {:.2} S/m) — the IEEE 1528 / IEC 62209 head-simulating liquid targets at 2450 MHz. Good-conductor skin depth estimate {:.0} mm: the field dies within a few cm of tissue.",
                i.head_radius_mm, i.tissue_er, i.tissue_sigma, skin
            ));
        }
        "ground" => {
            let jg = (cy + 0.25 * ny as f64).round() as usize;
            for x in 0..nx {
                s.set_pec(x, jg);
            }
            legend.push(pec_legend.clone());
            let a = user.unwrap_or([clampc(cx, nx, p), clampc(jg as f64 - mm(i.height_mm), ny, p)]);
            src.push((a, 0.0));
            probe_default = [cx, (p + 6) as f64];
            notes.push(format!(
                "Line source {:.1} mm ({:.2} λ) from a ground plane: image theory replaces the plane by an opposite-phase image, giving a lobe away from the plane and a null along it.",
                i.height_mm,
                i.height_mm / (lam_cells * i.dx_mm)
            ));
        }
        "slit" => {
            let xw = (nx as f64 * 0.45).round() as usize;
            let half = mm(i.slit_mm) / 2.0;
            for y in 0..ny {
                if (y as f64 - cy).abs() > half {
                    s.set_pec(xw, y);
                    s.set_pec(xw + 1, y);
                }
            }
            legend.push(pec_legend.clone());
            if let Some(u) = user {
                src.push((u, 0.0));
            } else {
                let xs = p + 4;
                for y in (p + 1)..(ny - p - 1) {
                    src.push(([xs, y], 0.0));
                }
            }
            probe_default = [nx as f64 * 0.8, cy];
            notes.push(format!(
                "Plane wave through a {:.0} mm slit ({:.2} λ): a narrow slit radiates like a new line source (Huygens), a wide one passes a beam with diffraction side lobes.",
                i.slit_mm,
                i.slit_mm / (lam_cells * i.dx_mm)
            ));
        }
        "waveguide" => {
            let a = mm(i.guide_mm);
            let (j0, j1) = ((cy - a / 2.0).round() as usize, (cy + a / 2.0).round() as usize);
            let x0 = (nx as f64 * 0.22).round() as usize;
            for x in x0..nx {
                s.set_pec(x, j0);
                s.set_pec(x, j1);
            }
            for y in j0..=j1 {
                s.set_pec(x0, y);
            }
            legend.push(pec_legend.clone());
            let st = user.unwrap_or([x0 + (mm(10.0).round() as usize).max(2), ((j0 + j1) / 2).max(1)]);
            src.push((st, 0.0));
            probe_default = [nx as f64 * 0.8, cy];
            let fc = C0 / (2.0 * i.guide_mm * 1e-3) / 1e9;
            notes.push(format!(
                "Parallel-plate guide {:.0} mm wide: lowest E_z mode (TM1, E_z = sin(πy/a)) cuts off at f_c = c/2a = {:.2} GHz — {} at {:.2} GHz.",
                i.guide_mm,
                fc,
                if i.f_ghz > fc { "it propagates" } else { "evanescent (below cutoff)" },
                i.f_ghz
            ));
        }
        _ => {
            src.push((user.unwrap_or([clampc(cx, nx, p), clampc(cy, ny, p)]), 0.0));
            probe_default = [cx + 0.3 * nx as f64, cy];
            notes.push("A single z-directed line source: the H-plane cut of a vertical dipole — omnidirectional in the plane, amplitude falling as 1/√r in 2-D (1/r in 3-D).".into());
        }
    }
    let pr = i.probe.unwrap_or(probe_default);
    Built {
        scene: s,
        legend,
        sources: src,
        probe: [clampc(pr[0], nx, p), clampc(pr[1], ny, p)],
        notes,
    }
}

/// Run a preset and return quantised frames plus probe/energy traces.
pub fn run(i: FdtdIn) -> Result<FdtdOut, String> {
    let mut i = i;
    i.nx = i.nx.clamp(40, 400);
    i.ny = i.ny.clamp(40, 400);
    i.pml = i.pml.min(24);
    i.steps = i.steps.clamp(10, 6000);
    i.frames = i.frames.clamp(1, 240);
    if !(i.dx_mm > 0.05 && i.f_ghz > 0.0 && i.courant > 0.0) {
        return Err("need dx > 0, f > 0 and Courant > 0".into());
    }
    if (i.nx * i.ny) as u64 * i.steps as u64 > 150_000_000 {
        return Err("grid × steps too large for the browser (limit 1.5e8 cell-steps)".into());
    }
    let dx = i.dx_mm * 1e-3;
    let lam0 = C0 / (i.f_ghz * 1e9);
    let lam_cells = lam0 / dx;
    let b = build(&i, lam_cells);
    let er_max = b.scene.eps_r.iter().cloned().fold(1.0, f64::max);
    let mat: Vec<u8> = (0..i.nx * i.ny)
        .map(|k| {
            if b.scene.pec[k] {
                2
            } else if b.scene.eps_r[k] > 1.0 || b.scene.sigma[k] > 0.0 {
                1
            } else {
                0
            }
        })
        .collect();
    let mut sim = Sim::new(b.scene, dx, i.courant, i.pml);
    let dt = sim.dt;
    let w = 2.0 * PI * i.f_ghz * 1e9;
    let period_steps = (1.0 / (i.f_ghz * 1e9) / dt).round().max(1.0) as usize;
    // pulse: Gaussian of width ≈ 1/(2f) so its spectrum covers ~0–2f
    let tau = 0.5 / (i.f_ghz * 1e9);
    let t0 = 4.0 * tau;
    let ramp = 3.0 / (i.f_ghz * 1e9);
    let every = (i.steps / i.frames).max(1);
    let mut frames: Vec<Vec<f32>> = Vec::new();
    let mut frame_step = Vec::new();
    let mut energy = Vec::new();
    let mut probe = Vec::with_capacity(i.steps);
    let (mut pabs, mut pout, mut navg) = (0.0, 0.0, 0usize);
    let avg_from = i.steps.saturating_sub(2 * period_steps);
    let amp = 1.0 / (b.sources.len() as f64).sqrt();
    let mut diverged: Option<usize> = None;
    for n in 0..i.steps {
        sim.step();
        let t = (n as f64 + 1.0) * dt;
        for (c, ph) in &b.sources {
            let s = if i.pulse {
                (-((t - t0) / tau).powi(2)).exp()
            } else {
                let r = if t < ramp { 0.5 * (1.0 - (PI * t / ramp).cos()) } else { 1.0 };
                r * (w * t + ph).sin()
            };
            sim.inject(c[0], c[1], amp * s);
        }
        probe.push(sim.ez[b.probe[1] * i.nx + b.probe[0]]);
        if !i.pulse && n >= avg_from {
            pabs += sim.absorbed();
            pout += sim.outflow(i.pml + 2);
            navg += 1;
        }
        let blown = sim.ez.iter().step_by(97).any(|v| !v.is_finite() || v.abs() > 1e30);
        if ((n % every == every - 1) || n == i.steps - 1 || blown) && frames.len() < i.frames {
            frames.push(sim.ez.iter().map(|&v| if v.is_finite() { v as f32 } else { 0.0 }).collect());
            frame_step.push(n + 1);
            energy.push(sim.energy().min(1e300));
        }
        if blown {
            diverged = Some(n + 1);
            break;
        }
    }
    // scale: the largest |Ez| away from the sources (they are singular in 2-D)
    let excl = 5i64;
    let mut scale: f64 = 0.0;
    for f in &frames {
        for j in 0..i.ny {
            for x in 0..i.nx {
                let near = b.sources.len() < 20
                    && b.sources
                        .iter()
                        .any(|(c, _)| (c[0] as i64 - x as i64).abs() <= excl && (c[1] as i64 - j as i64).abs() <= excl);
                let v = f[j * i.nx + x].abs() as f64;
                if !near && v.is_finite() {
                    scale = scale.max(v);
                }
            }
        }
    }
    if scale <= 0.0 {
        scale = 1.0;
    }
    let g = i.gamma.clamp(0.2, 1.0);
    let mut bytes = Vec::with_capacity(frames.len() * i.nx * i.ny);
    for f in &frames {
        for &v in f {
            let x = (v as f64 / scale).clamp(-1.0, 1.0);
            let y = x.signum() * x.abs().powf(g);
            bytes.push((128.0 + 127.0 * y).round().clamp(1.0, 255.0) as u8);
        }
    }
    let absorbed_fraction = if navg > 0 && pabs + pout > 0.0 && pabs > 0.0 {
        Some((pabs / (pabs + pout)).clamp(0.0, 1.0))
    } else {
        None
    };
    // decimate probe trace for the plot
    let dec = (probe.len() / 1200).max(1);
    let probe_t: Vec<f64> = (0..probe.len()).step_by(dec).map(|n| (n as f64 + 1.0) * dt * 1e9).collect();
    let probe_v: Vec<f64> = probe.iter().step_by(dec).cloned().collect();
    let mut notes = b.notes;
    if let Some(n) = diverged {
        notes.insert(
            0,
            format!("✕ Unstable: the field blew up after {n} steps — the Courant number S = {:.2} exceeds 1, so errors grow every step (checkerboard).", i.courant),
        );
    }
    if er_max > 1.0 && lam_cells / er_max.sqrt() < 10.0 {
        notes.push(format!(
            "Only {:.1} cells per wavelength inside the densest material — numerical dispersion is noticeable; use a finer grid for quantitative work.",
            lam_cells / er_max.sqrt()
        ));
    }
    Ok(FdtdOut {
        preset: i.preset.clone(),
        nx: i.nx,
        ny: i.ny,
        dx_mm: i.dx_mm,
        dt_ps: dt * 1e12,
        dt_max_ps: dt_max(dx) * 1e12,
        courant: i.courant,
        stable: i.courant <= 1.0 && diverged.is_none(),
        cells_per_wavelength: lam_cells,
        cells_per_wavelength_min: lam_cells / er_max.sqrt(),
        pml: i.pml,
        steps: i.steps,
        frames_b64: crate::cad::export::base64(&bytes),
        n_frames: frames.len(),
        frame_time_ns: frame_step.iter().map(|&n| n as f64 * dt * 1e9).collect(),
        frame_step,
        material_b64: crate::cad::export::base64(&mat),
        legend: b.legend,
        scale,
        gamma: g,
        sources: b.sources.iter().map(|x| x.0).collect(),
        probe: b.probe,
        probe_t_ns: probe_t,
        probe_ez: probe_v,
        energy,
        absorbed_db: absorbed_fraction.map(|f| -10.0 * (1.0 - f).max(1e-6).log10()),
        absorbed_fraction,
        notes,
        ms_hint: (i.nx * i.ny * i.steps) as f64 * 1.5e-5,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn courant_limit_formula() {
        // Δt_max = Δ/(c√2): 1.5 mm → 3.5376 ps
        assert!((dt_max(1.5e-3) * 1e12 - 3.5376).abs() < 1e-3);
    }

    fn cavity_energy(courant: f64, steps: usize) -> (f64, f64) {
        let mut s = Sim::new(Scene::vacuum(50, 50), 1e-3, courant, 0);
        for j in 20..30 {
            for i in 20..30 {
                s.set_ez(i, j, 1.0);
            }
        }
        s.step();
        let e0 = s.energy();
        for _ in 0..steps {
            s.step();
        }
        (e0, s.energy())
    }

    #[test]
    fn stable_below_and_unstable_above_courant_limit() {
        let (e0, e1) = cavity_energy(0.99, 2000);
        assert!(e1.is_finite() && e1 < 3.0 * e0, "stable run: {e0} → {e1}");
        let (u0, u1) = cavity_energy(1.05, 2000);
        assert!(!u1.is_finite() || u1 > 1e6 * u0, "S = 1.05 should blow up: {u0} → {u1}");
    }

    #[test]
    fn free_space_wave_speed_is_c() {
        let (nx, ny) = (240, 60);
        let dx = 1e-3;
        let mut s = Sim::new(Scene::vacuum(nx, ny), dx, 0.99, 10);
        let (si, sj) = (40, 30);
        let tau = 12.0 * s.dt;
        let t0 = 5.0 * tau;
        let (p1, p2) = (si + 50, si + 170);
        let (mut a1, mut a2) = ((0.0, 0usize), (0.0, 0usize));
        for n in 0..900 {
            s.step();
            let t = (n as f64 + 1.0) * s.dt;
            s.inject(si, sj, (-((t - t0) / tau).powi(2)).exp());
            let v1 = s.ez[sj * nx + p1].abs();
            let v2 = s.ez[sj * nx + p2].abs();
            if v1 > a1.0 {
                a1 = (v1, n);
            }
            if v2 > a2.0 {
                a2 = (v2, n);
            }
        }
        let v = (120.0 * dx) / ((a2.1 - a1.1) as f64 * s.dt);
        assert!((v / C0 - 1.0).abs() < 0.03, "speed {v:.4e} vs c");
    }

    #[test]
    fn pml_absorbs_outgoing_wave() {
        let mut s = Sim::new(Scene::vacuum(120, 120), 1.5e-3, 0.99, 12);
        let tau = 10.0 * s.dt;
        let mut peak: f64 = 0.0;
        for n in 0..1500 {
            s.step();
            let t = (n as f64 + 1.0) * s.dt;
            s.inject(60, 60, (-((t - 4.0 * tau) / tau).powi(2)).exp());
            peak = peak.max(s.energy());
        }
        let left = s.energy();
        assert!(left < 1e-3 * peak, "energy left {left:e} of peak {peak:e}");
    }

    #[test]
    fn head_preset_absorbs_a_large_fraction() {
        let o = run(FdtdIn {
            preset: "head".into(),
            nx: 120,
            ny: 100,
            dx_mm: 2.0,
            steps: 700,
            frames: 10,
            head_radius_mm: 60.0,
            ..Default::default()
        })
        .unwrap();
        let f = o.absorbed_fraction.unwrap();
        assert!(f > 0.1 && f < 0.95, "absorbed fraction {f}");
        assert_eq!(o.n_frames, 10);
    }
}
