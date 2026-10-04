//! The Iris right-temple electronics pod as a 3-D conduction model.
//!
//! ## Geometry
//! The real CAD parts from the CAD Studio (`crate::cad::frame_geom`) — the
//! right temple with its hollow pod, the pod lid, the display-optics housing
//! and the right half of the front — are voxelised with the Structures Lab's
//! ray-casting voxeliser (`crate::structures::voxel::fill`, partial-volume
//! fractions) on one regular grid around the right side of the frame. The
//! printed circuit board (PCB) is the box the CAD layout reserves for it, held
//! off the cavity floor by two plastic ribs at its ends (an assumption: the
//! CAD has no mounting features yet); an optional heat-spreader sheet lines
//! the inside of the skin-side wall ("floor") or of the lid; the micro-display
//! is a small box at the far end of the optics housing. Empty voxels that
//! cannot be reached from outside the frame are the sealed cavity air.
//!
//! ## Materials per voxel
//! Rule of mixtures on the volume fractions: along a sheet (y, z) the
//! constituents conduct in parallel, `k = Σ fᵢ kᵢ`; across the PCB / spreader
//! (x) they are in series with the rest of the voxel. Heat capacity
//! `ρc = Σ fᵢ ρᵢ cᵢ`. The cavity air carries molecular conduction plus
//! linearised enclosure radiation (see `props::cavity_air_k`).
//!
//! ## Heat sources
//! Every powered component's average dissipation for the chosen usage
//! scenario comes from the Power Lab's budget (`crate::power::budget`), so
//! the total equals the lumped model's pod heat exactly. Board components are
//! placed where the PCB Studio's constructive placement puts them
//! (`crate::pcb::placement`), their power spread over the package body on
//! the board face; converter losses (load/η − load) and quiescent power go to
//! the buck converter. The display (`location: front_right`) heats the
//! optics housing — or the board at its connector, as the lumped model
//! implicitly assumes, if you choose so.
//!
//! ## Boundary
//! Every exposed face gets `h = h_conv + h_rad`: natural convection from the
//! correlations in `convection.rs` (vertical faces: Churchill–Chu with the
//! body height; up/down faces: horizontal-plate fits with L = A/P), blended
//! with laminar flat-plate forced convection when the wearer walks
//! (`Nu³ = Nu_N³ + Nu_F³`), plus linearised radiation. Coefficients depend
//! on the surface temperature, so they are updated from the solved mean
//! excess of each body/face class and the system re-solved (Picard
//! iteration, three passes). Faces where the model is cut out of the frame
//! are adiabatic.

use std::cell::RefCell;

use serde::{Deserialize, Serialize};

use super::convection::{blend, h_forced_plate, h_natural, h_radiation, FaceClass};
use super::fea::{self, Elem, Model, Precond};
use super::props::{self, ThermalMaterial, K0};
use crate::power::budget::{self as pbudget, UsageProfile};
use crate::project::Project;
use crate::structures::fea3d::Grid;

// ------------------------------------------------------------------ configuration

/// Per-lab design choices stored at `design.extra.fluids`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FluidsDesign {
    /// none | graphite | copper | aluminium
    pub spreader: String,
    pub spreader_um: f64,
    /// floor (skin-side wall) | lid | optics (behind the display, on the housing end wall)
    pub spreader_location: String,
    /// optics | pod
    pub display_heat_in: String,
    pub emissivity: Option<f64>,
    pub air_speed_m_s: f64,
}

impl Default for FluidsDesign {
    fn default() -> Self {
        FluidsDesign {
            spreader: "none".into(),
            spreader_um: 25.0,
            spreader_location: "floor".into(),
            display_heat_in: "optics".into(),
            emissivity: None,
            air_speed_m_s: 0.0,
        }
    }
}

impl FluidsDesign {
    pub fn from_project(p: &Project) -> FluidsDesign {
        p.design
            .extra
            .get("fluids")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }
}

/// Model options; unset fields fall back to `design.extra.fluids`, then defaults.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PodConfig {
    pub voxel_mm: Option<f64>,
    /// Power-Lab scenario id (reference, navigation, captions, notifications, standby, display_on).
    pub scenario: Option<String>,
    /// Override the display duty (0..1) on top of the scenario.
    pub display_duty: Option<f64>,
    pub spreader: Option<String>,
    pub spreader_um: Option<f64>,
    pub spreader_k_in: Option<f64>,
    pub spreader_location: Option<String>,
    pub display_heat_in: Option<String>,
    pub shell_material: Option<String>,
    pub emissivity: Option<f64>,
    pub air_speed_m_s: Option<f64>,
    /// Fixed convective coefficient for every face, W/m² K (radiation still added).
    pub h_override: Option<f64>,
    pub ambient_c: Option<f64>,
    /// ssor | jacobi
    pub precond: Option<String>,
    pub tol: Option<f64>,
    pub max_iter: Option<usize>,
    /// Copper fill fraction of the PCB layers (0..1).
    pub cu_coverage: Option<f64>,
}

/// Fully resolved options.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Resolved {
    pub voxel_mm: f64,
    pub scenario: String,
    pub display_duty: Option<f64>,
    pub spreader: String,
    pub spreader_um: f64,
    pub spreader_k_in: Option<f64>,
    pub spreader_location: String,
    pub display_heat_in: String,
    pub shell_material: String,
    pub emissivity: f64,
    pub air_speed_m_s: f64,
    pub h_override: Option<f64>,
    pub ambient_c: f64,
    pub precond: String,
    pub tol: f64,
    pub max_iter: usize,
    pub cu_coverage: f64,
}

impl PodConfig {
    pub fn resolve(&self, p: &Project) -> Resolved {
        let d = FluidsDesign::from_project(p);
        let shell = self.shell_material.clone().unwrap_or_else(|| p.design.frame.material.clone());
        let (sm, _) = props::shell_material(&shell);
        Resolved {
            voxel_mm: self.voxel_mm.unwrap_or(1.5).clamp(0.6, 2.0),
            scenario: self.scenario.clone().unwrap_or_else(|| "display_on".into()),
            display_duty: self.display_duty.map(|x| x.clamp(0.0, 1.0)),
            spreader: self.spreader.clone().unwrap_or(d.spreader),
            spreader_um: self.spreader_um.unwrap_or(d.spreader_um).clamp(5.0, 2000.0),
            spreader_k_in: self.spreader_k_in,
            spreader_location: self.spreader_location.clone().unwrap_or(d.spreader_location),
            display_heat_in: self.display_heat_in.clone().unwrap_or(d.display_heat_in),
            shell_material: shell,
            emissivity: self.emissivity.or(d.emissivity).unwrap_or(sm.emissivity).clamp(0.02, 1.0),
            air_speed_m_s: self.air_speed_m_s.unwrap_or(d.air_speed_m_s).clamp(0.0, 20.0),
            h_override: self.h_override.filter(|h| *h > 0.0),
            ambient_c: self.ambient_c.unwrap_or(p.design.thermal.ambient_c),
            precond: self.precond.clone().unwrap_or_else(|| "ssor".into()),
            tol: self.tol.unwrap_or(1e-8).clamp(1e-12, 1e-3),
            max_iter: self.max_iter.unwrap_or(6000).clamp(10, 50_000),
            cu_coverage: self.cu_coverage.unwrap_or(0.5).clamp(0.0, 1.0),
        }
    }
}

/// Usage profile of a scenario id (unknown ids → the R-008 display-on case).
pub fn profile_for(id: &str, display_duty: Option<f64>) -> UsageProfile {
    let mut prof = pbudget::scenario_profile(id).unwrap_or_else(|| UsageProfile {
        display_duty: Some(1.0),
        ..Default::default()
    });
    if let Some(d) = display_duty {
        prof.display_duty = Some(d);
    }
    prof
}

// ------------------------------------------------------------------ tags

pub const PART_POD: u8 = 0;
pub const PART_LID: u8 = 1;
pub const PART_OPTICS: u8 = 2;
pub const PART_FRONT: u8 = 3;
pub const PART_PCB: u8 = 4;
pub const PART_DISPLAY: u8 = 5;
pub const PART_AIR: u8 = 6;
pub const PART_SPREADER: u8 = 7;
pub const PART_CONNECTOR: u8 = 8;
pub const PART_NAMES: [&str; 9] = [
    "Pod (right temple)",
    "Pod lid",
    "Optics housing",
    "Front (right half)",
    "Printed circuit board",
    "Micro-display",
    "Cavity air",
    "Heat spreader",
    "USB-C receptacle",
];
const CAD_PARTS: [&str; 4] = ["temple_r", "lid_r", "optics_housing", "front"];

/// Material code used in the slice view: 0 outside, 1 shell, 2 air, 3 PCB, 4 spreader, 5 display.
fn mat_code(part: u8) -> u8 {
    match part {
        PART_POD | PART_LID | PART_OPTICS | PART_FRONT => 1,
        PART_AIR => 2,
        PART_PCB => 3,
        PART_SPREADER => 4,
        PART_DISPLAY => 5,
        PART_CONNECTOR => 6,
        _ => 0,
    }
}

/// Body a surface belongs to for its convection coefficient: 0 pod, 1 optics, 2 front.
fn body_of(part: u8) -> usize {
    match part {
        PART_OPTICS | PART_DISPLAY => 1,
        PART_FRONT => 2,
        _ => 0,
    }
}
pub const BODY_NAMES: [&str; 3] = ["pod", "optics housing", "front"];
const CLASSES: [FaceClass; 3] = [FaceClass::Vertical, FaceClass::Up, FaceClass::Down];

fn class_of(dir: usize) -> usize {
    match dir {
        5 => 1, // +z: upward-facing
        4 => 2, // −z: downward-facing
        _ => 0,
    }
}

// ------------------------------------------------------------------ geometry (memoised)

/// Voxelised CAD parts on the pod grid.
#[derive(Clone, Debug)]
pub struct Geometry {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    pub h: f64,
    pub origin: [f64; 3],
    /// Fractions of temple_r, lid_r, optics_housing, front per grid element.
    pub frac: [Vec<f32>; 4],
    pub layout: crate::cad::frame::FrameLayout,
    /// Optics-housing bounding box (assembly mm).
    pub optics_box: ([f64; 3], [f64; 3]),
    /// Clipped CAD meshes for drawing (assembly mm).
    pub cad: Vec<(String, crate::cad::Mesh)>,
}

thread_local! {
    static GEOM: crate::memo::Slot<std::rc::Rc<Geometry>> = const { RefCell::new(None) };
    static PLACE: crate::memo::Slot<Vec<crate::pcb::Placed>> = const { RefCell::new(None) };
}

fn clip_mesh(m: &crate::cad::Mesh, lo: [f64; 3], hi: [f64; 3]) -> crate::cad::Mesh {
    let mut out = crate::cad::Mesh::default();
    let mut map = vec![u32::MAX; m.vertex_count()];
    let inside = |p: [f64; 3]| (0..3).all(|d| p[d] >= lo[d] - 1e-6 && p[d] <= hi[d] + 1e-6);
    for t in 0..m.triangle_count() {
        let ids = [m.indices[3 * t], m.indices[3 * t + 1], m.indices[3 * t + 2]];
        if ids.iter().all(|&i| inside(m.vertex(i))) {
            let mut nid = [0u32; 3];
            for (k, &i) in ids.iter().enumerate() {
                if map[i as usize] == u32::MAX {
                    map[i as usize] = out.add_vertex(m.vertex(i));
                }
                nid[k] = map[i as usize];
            }
            out.add_tri(nid[0], nid[1], nid[2]);
        }
    }
    out
}

/// Voxelise the right side of the frame on a grid of pitch `h` mm (memoised).
pub fn geometry(p: &Project, h: f64) -> Result<std::rc::Rc<Geometry>, String> {
    let key = crate::memo::key(&(&p.design.frame, &p.design.optics, &p.design.pcb, (h * 1000.0).round()));
    let mut err = None;
    let g = crate::memo::memo(&GEOM, key, || match build_geometry(p, h) {
        Ok(g) => std::rc::Rc::new(g),
        Err(e) => {
            err = Some(e);
            std::rc::Rc::new(Geometry {
                nx: 0,
                ny: 0,
                nz: 0,
                h,
                origin: [0.0; 3],
                frac: [vec![], vec![], vec![], vec![]],
                layout: Default::default(),
                optics_box: ([0.0; 3], [0.0; 3]),
                cad: vec![],
            })
        }
    });
    if let Some(e) = err {
        GEOM.with(|s| *s.borrow_mut() = None);
        return Err(e);
    }
    Ok(g)
}

fn build_geometry(p: &Project, h: f64) -> Result<Geometry, String> {
    let fg = crate::cad::frame_geom(p);
    let l = fg.layout.clone();
    let part = |n: &str| fg.parts.iter().find(|x| x.name == n).map(|x| &x.assembly);
    let (temple, lid, optics, front) = match (part("temple_r"), part("lid_r"), part("optics_housing"), part("front")) {
        (Some(a), Some(b), Some(c), Some(d)) => (a, b, c, d),
        _ => return Err("frame is missing the right temple, lid, optics housing or front".into()),
    };
    let (olo, ohi) = optics.bbox();
    let (llo, lhi) = lid.bbox();
    let f = &p.design.frame;
    let pod_lo_z = l.hinge_z - f.pod_height_mm / 2.0;
    let pod_hi_z = l.hinge_z + f.pod_height_mm / 2.0;
    let (_, fhi) = front.bbox();
    let lo = [olo[0] - 5.0, l.pod_rear_y - 6.0, pod_lo_z.min(olo[2]) - 4.0];
    let hi = [
        lhi[0].max(l.temple_inner_x + f.pod_width_mm) + 0.3,
        fhi[1].max(ohi[1]) + 0.3,
        pod_hi_z.max(ohi[2]).max(lhi[2]) + 0.3,
    ];
    let _ = llo;
    let dims: [usize; 3] = std::array::from_fn(|d| (((hi[d] - lo[d]) / h).ceil() as usize).max(1));
    if dims.iter().product::<usize>() > 1_500_000 {
        return Err(format!("{:?} voxels is too many — use a coarser voxel size", dims));
    }
    let origin = lo;
    let mut frac: [Vec<f32>; 4] = [vec![], vec![], vec![], vec![]];
    for (i, m) in [temple, lid, optics, front].into_iter().enumerate() {
        let mut g = Grid::new(dims[0], dims[1], dims[2], h, origin);
        crate::structures::voxel::fill(&mut g, m);
        frac[i] = g.rho;
    }
    let hi_grid: [f64; 3] = std::array::from_fn(|d| origin[d] + dims[d] as f64 * h);
    let cad = [temple, lid, optics, front]
        .iter()
        .zip(CAD_PARTS)
        .map(|(m, n)| (n.to_string(), clip_mesh(m, origin, hi_grid)))
        .collect();
    Ok(Geometry {
        nx: dims[0],
        ny: dims[1],
        nz: dims[2],
        h,
        origin,
        frac,
        layout: l,
        optics_box: (olo, ohi),
        cad,
    })
}

fn placement(p: &Project) -> Vec<crate::pcb::Placed> {
    let comps: Vec<(&str, &str, bool)> = p
        .components
        .iter()
        .map(|c| (c.refdes.as_str(), c.footprint.as_str(), c.on_pcb))
        .collect();
    let key = crate::memo::key(&(&p.design.pcb, comps));
    crate::memo::memo(&PLACE, key, || crate::pcb::placement(p))
}

// ------------------------------------------------------------------ boxes

/// Axis-aligned box.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Aabb {
    pub lo: [f64; 3],
    pub hi: [f64; 3],
}

impl Aabb {
    fn overlap_cell(&self, lo: [f64; 3], h: f64) -> f64 {
        let mut v = 1.0;
        for ((&slo, &shi), &l) in self.lo.iter().zip(&self.hi).zip(&lo) {
            let a = slo.max(l);
            let b = shi.min(l + h);
            if b <= a {
                return 0.0;
            }
            v *= (b - a) / h;
        }
        v
    }
    fn center(&self) -> [f64; 3] {
        std::array::from_fn(|d| 0.5 * (self.lo[d] + self.hi[d]))
    }
}

// ------------------------------------------------------------------ the model

/// One heat source as placed in the model.
#[derive(Clone, Debug, Serialize)]
pub struct Source {
    pub refdes: String,
    pub name: String,
    pub location: String,
    /// Average dissipation in this scenario, mW.
    pub mw: f64,
    /// Share that scales with the display duty (for transients), mW per unit duty.
    pub pos: [f64; 3],
    /// Where the heat goes: "board top", "board bottom", "display", "board (spread)", "not modelled".
    pub placed: String,
}

/// Everything needed to solve and report.
pub struct PodModel {
    pub model: Model,
    pub csr: fea::Csr,
    pub cap: Vec<f64>,
    /// Nodal heat input for the scenario, W.
    pub q: Vec<f64>,
    /// Nodal heat input with the display at duty 0 and the slope per unit duty, W.
    pub q0: Vec<f64>,
    pub q1: Vec<f64>,
    pub sources: Vec<Source>,
    pub total_w: f64,
    pub geom: std::rc::Rc<Geometry>,
    pub cfg: Resolved,
    pub pcb_box: Aabb,
    pub display_box: Aabb,
    pub spreader_box: Option<Aabb>,
    /// Sheet volume the conducting elements carry, mm³ (Σ fraction × voxel volume).
    pub spreader_volume_mm3: f64,
    pub shell: ThermalMaterial,
    pub pcb_mat: ThermalMaterial,
    pub spreader_mat: Option<ThermalMaterial>,
    pub k_cavity: f64,
    pub k_cavity_rad: f64,
    /// Characteristic lengths per body (vertical, A/P, flow), m.
    pub lengths: [[f64; 3]; 3],
    pub cavity_cells: usize,
    pub cavity_expected_cells: f64,
    pub warnings: Vec<String>,
    /// Grid element → part tag (255 outside).
    pub part_of: Vec<u8>,
}

/// The profile with the display off and fully on (heat is linear in display duty).
fn duty_split(prof: &UsageProfile) -> (UsageProfile, UsageProfile) {
    let with = |d: f64| UsageProfile {
        display_duty: Some(d),
        ..prof.clone()
    };
    (with(0.0), with(1.0))
}

/// Build the conduction model for the project and options.
pub fn build(p: &Project, cfg: &Resolved) -> Result<PodModel, String> {
    let h = cfg.voxel_mm;
    let geom = geometry(p, h)?;
    let g = &*geom;
    let l = &g.layout;
    let f = &p.design.frame;
    let n = g.nx * g.ny * g.nz;
    let mut warnings = Vec::new();
    let (shell, known) = props::shell_material(&cfg.shell_material);
    if !known {
        warnings.push(format!(
            "Shell material '{}' not in the thermal table; using PETG values.",
            cfg.shell_material
        ));
    }
    let pcb_mat = props::pcb_material(p.design.pcb.thickness_mm, p.design.pcb.layers, 35.0, cfg.cu_coverage);
    let disp_mat = props::display_material();
    let t_mean_k = cfg.ambient_c + K0 + 8.0;
    let (k_air, k_rad) = props::cavity_air_k(t_mean_k, 0.9, 2e-3);
    let k_cav = k_air + k_rad;
    let air = props::air(t_mean_k);
    let air_rc = air.rho * air.cp;

    // PCB box and mounting ribs.
    let pc = l.pcb_center;
    let ps = l.pcb_size;
    let pcb_box = Aabb {
        lo: [pc[0] - ps[0] / 2.0, pc[1] - ps[1] / 2.0, pc[2] - ps[2] / 2.0],
        hi: [pc[0] + ps[0] / 2.0, pc[1] + ps[1] / 2.0, pc[2] + ps[2] / 2.0],
    };
    let floor_x = l.temple_inner_x + f.wall_mm;
    let mut ribs = Vec::new();
    if pcb_box.lo[0] > floor_x + 0.05 {
        for yc in [pcb_box.lo[1] + 1.0, pcb_box.hi[1] - 1.0] {
            ribs.push(Aabb {
                lo: [floor_x - 0.2, yc - 0.75, pcb_box.lo[2]],
                hi: [pcb_box.lo[0], yc + 0.75, pcb_box.hi[2]],
            });
        }
    }
    // Display box at the far end of the optics housing.
    let (olo, ohi) = g.optics_box;
    let wall = f.wall_mm;
    let dw = p.design.optics.display_width_mm.clamp(1.0, 30.0);
    let dh = p.design.optics.display_height_mm.clamp(1.0, 30.0);
    let oc = [0.5 * (olo[0] + ohi[0]), 0.5 * (olo[1] + ohi[1]), 0.5 * (olo[2] + ohi[2])];
    let dt = 1.5;
    let display_box = Aabb {
        lo: [ohi[0] - wall - dt, oc[1] - dw / 2.0, oc[2] - dh / 2.0],
        hi: [ohi[0] - wall + 0.2, oc[1] + dw / 2.0, oc[2] + dh / 2.0],
    };
    // Spreader sheet.
    let spreader_mat = props::spreader(&cfg.spreader).map(|mut m| {
        if let Some(k) = cfg.spreader_k_in {
            m.k_in = k.clamp(1.0, 5000.0);
        }
        m
    });
    let ts = cfg.spreader_um * 1e-3;
    let cav_len = l.pod_cavity_len_floor_mm.max(1.0);
    let cav_h = l.pod_cavity_height_floor_mm.max(1.0);
    let pod_yc = 0.5 * (l.pod_front_y + l.pod_rear_y);
    let spreader_box = spreader_mat.as_ref().map(|_| match cfg.spreader_location.as_str() {
        "optics" => {
            // on the inside of the housing's end wall, behind the display, across the whole wall
            let x1 = ohi[0] - wall;
            let (ey, ez) = ((ohi[1] - olo[1]) / 2.0 - wall - 1.5, (ohi[2] - olo[2]) / 2.0 - wall - 1.5);
            Aabb {
                lo: [x1 - ts, oc[1] - ey.max(1.0), oc[2] - ez.max(1.0)],
                hi: [x1, oc[1] + ey.max(1.0), oc[2] + ez.max(1.0)],
            }
        }
        loc => {
            let (x0, x1) = if loc == "lid" {
                let xl = l.temple_inner_x + f.pod_width_mm;
                (xl - ts, xl)
            } else {
                (floor_x, floor_x + ts)
            };
            Aabb {
                lo: [x0, pod_yc - cav_len / 2.0 + 0.5, l.hinge_z - cav_h / 2.0 + 0.3],
                hi: [x1, pod_yc + cav_len / 2.0 - 0.5, l.hinge_z + cav_h / 2.0 - 0.3],
            }
        }
    });
    // USB-C receptacle: its metal shell fills the slot in the pod's bottom wall under the lid tongue.
    let usb_box = Aabb {
        lo: [
            l.temple_inner_x + f.temple_width_mm - 0.2,
            l.usb_slot_y[0],
            l.hinge_z - f.pod_height_mm / 2.0 - 0.1,
        ],
        hi: [
            l.temple_inner_x + f.temple_width_mm + crate::cad::frame::USB_RECEPTACLE_H + 0.3,
            l.usb_slot_y[1],
            l.hinge_z - f.pod_height_mm / 2.0 + f.wall_mm + 1.0,
        ],
    };
    let usb_k = 5.0;
    let usb_rc = 2.0e6;

    // Per-element fractions.
    let cell_lo = |e: usize| -> [f64; 3] {
        let (i, j, k) = (e % g.nx, (e / g.nx) % g.ny, e / (g.nx * g.ny));
        [g.origin[0] + h * i as f64, g.origin[1] + h * j as f64, g.origin[2] + h * k as f64]
    };
    let box_frac = |b: &Aabb, out: &mut Vec<f32>| {
        let lo_i: [usize; 3] = std::array::from_fn(|d| (((b.lo[d] - g.origin[d]) / h).floor().max(0.0)) as usize);
        let dims = [g.nx, g.ny, g.nz];
        let hi_i: [usize; 3] = std::array::from_fn(|d| ((((b.hi[d] - g.origin[d]) / h).ceil()).max(0.0) as usize).min(dims[d]));
        for k in lo_i[2]..hi_i[2] {
            for j in lo_i[1]..hi_i[1] {
                for i in lo_i[0]..hi_i[0] {
                    let e = i + g.nx * (j + g.ny * k);
                    out[e] += b.overlap_cell(cell_lo(e), h) as f32;
                }
            }
        }
    };
    let mut f_pcb = vec![0f32; n];
    box_frac(&pcb_box, &mut f_pcb);
    let mut f_rib = vec![0f32; n];
    for r in &ribs {
        box_frac(r, &mut f_rib);
    }
    let mut f_dsp = vec![0f32; n];
    box_frac(&display_box, &mut f_dsp);
    let mut f_spr = vec![0f32; n];
    if let Some(b) = &spreader_box {
        box_frac(b, &mut f_spr);
    }
    let mut f_usb = vec![0f32; n];
    box_frac(&usb_box, &mut f_usb);
    const THR: f32 = 0.08;
    let solid_frac = |e: usize| -> f32 {
        let sh: f32 = g.frac.iter().map(|v| v[e]).sum::<f32>() + f_rib[e];
        (sh + f_pcb[e] + f_dsp[e] + f_spr[e] + f_usb[e]).min(1.0)
    };
    // Flood fill the outside air from the grid boundary.
    let mut outside = vec![false; n];
    let mut stack = Vec::new();
    for k in 0..g.nz {
        for j in 0..g.ny {
            for i in 0..g.nx {
                if i == 0 || j == 0 || k == 0 || i == g.nx - 1 || j == g.ny - 1 || k == g.nz - 1 {
                    let e = i + g.nx * (j + g.ny * k);
                    if solid_frac(e) < THR && !outside[e] {
                        outside[e] = true;
                        stack.push(e);
                    }
                }
            }
        }
    }
    while let Some(e) = stack.pop() {
        let (i, j, k) = (e % g.nx, (e / g.nx) % g.ny, e / (g.nx * g.ny));
        let mut try_push = |ii: usize, jj: usize, kk: usize| {
            let m = ii + g.nx * (jj + g.ny * kk);
            if !outside[m] && solid_frac(m) < THR {
                outside[m] = true;
                stack.push(m);
            }
        };
        if i > 0 {
            try_push(i - 1, j, k);
        }
        if i + 1 < g.nx {
            try_push(i + 1, j, k);
        }
        if j > 0 {
            try_push(i, j - 1, k);
        }
        if j + 1 < g.ny {
            try_push(i, j + 1, k);
        }
        if k > 0 {
            try_push(i, j, k - 1);
        }
        if k + 1 < g.nz {
            try_push(i, j, k + 1);
        }
    }
    let touches_outside = |e: usize| -> bool {
        let (i, j, k) = (e % g.nx, (e / g.nx) % g.ny, e / (g.nx * g.ny));
        let nb = [
            (i > 0).then(|| e - 1),
            (i + 1 < g.nx).then(|| e + 1),
            (j > 0).then(|| e - g.nx),
            (j + 1 < g.ny).then(|| e + g.nx),
            (k > 0).then(|| e - g.nx * g.ny),
            (k + 1 < g.nz).then(|| e + g.nx * g.ny),
        ];
        nb.iter().any(|x| x.is_some_and(|m| outside[m])) || nb.iter().any(|x| x.is_none())
    };
    let spr = spreader_mat.clone();
    let mut elems = Vec::new();
    let mut spreader_volume_mm3 = 0.0;
    let mut part_of = vec![255u8; n];
    let mut cavity_cells = 0usize;
    for e in 0..n {
        if outside[e] {
            continue;
        }
        let fs: [f64; 4] = std::array::from_fn(|i| g.frac[i][e] as f64);
        let rib = f_rib[e] as f64;
        let mut fsh = fs.iter().sum::<f64>() + rib;
        let mut fp = f_pcb[e] as f64;
        let mut fd = f_dsp[e] as f64;
        let mut fsp = f_spr[e] as f64;
        let mut fu = f_usb[e] as f64;
        let s = fsh + fp + fd + fsp + fu;
        if s > 1.0 {
            // Over-full voxel (boxes overlap the voxelised shell, e.g. the sheet
            // behind the display shares voxels with the housing wall and the
            // display module). The sheet keeps its volume — its conductance k·t
            // is what spreads heat — and the other solids share what is left;
            // scaling it down with them would remove more of a thicker sheet.
            let fsp_keep = fsp.min(1.0);
            let rest = s - fsp;
            let r = if rest > 1e-12 { (1.0 - fsp_keep) / rest } else { 0.0 };
            fsh *= r;
            fp *= r;
            fd *= r;
            fu *= r;
            fsp = fsp_keep;
        }
        spreader_volume_mm3 += fsp * h * h * h;
        let s = fsh + fp + fd + fsp + fu;
        let cavity = (s as f32) < THR;
        if cavity {
            cavity_cells += 1;
        }
        let fair = if cavity || !touches_outside(e) { (1.0 - s).max(0.0) } else { 0.0 };
        // parallel (in-plane) mixture
        let spk = spr.as_ref().map(|m| (m.k_in, m.k_through, m.rho_c())).unwrap_or((0.0, 1.0, 0.0));
        let k_par = fsh * shell.k_in + fp * pcb_mat.k_in + fd * disp_mat.k_in + fsp * spk.0 + fair * k_cav + fu * usb_k;
        // across the sheets (x): sheets in series with the rest
        let f_sheet = fp + fsp;
        let f_rest = fsh + fd + fair + fu;
        let k_x = if f_sheet <= 1e-9 {
            k_par
        } else {
            let k_sheet = f_sheet / (fp / pcb_mat.k_through + fsp / spk.1);
            if f_rest <= 1e-9 {
                k_sheet * f_sheet
            } else {
                let k_rest = (fsh * shell.k_in + fd * disp_mat.k_in + fair * k_cav + fu * usb_k) / f_rest;
                let tot = f_sheet + f_rest;
                tot * tot / (f_sheet / k_sheet + f_rest / k_rest)
            }
        };
        let rho_c = fsh * shell.rho_c() + fp * pcb_mat.rho_c() + fd * disp_mat.rho_c() + fsp * spk.2 + fair * air_rc + fu * usb_rc;
        // dominant part
        let mut cands: Vec<(f64, u8)> = vec![
            (fs[0] + rib, PART_POD),
            (fs[1], PART_LID),
            (fs[2], PART_OPTICS),
            (fs[3], PART_FRONT),
            (fp, PART_PCB),
            (fd, PART_DISPLAY),
            (fsp * 0.5, PART_SPREADER),
            (fu, PART_CONNECTOR),
        ];
        if cavity {
            cands.push((fair, PART_AIR));
        }
        cands.sort_by(|a, b| b.0.total_cmp(&a.0));
        let part = cands[0].1;
        part_of[e] = part;
        elems.push(Elem {
            e: e as u32,
            k: [k_x.max(1e-4), k_par.max(1e-4), k_par.max(1e-4)],
            rho_c: rho_c.max(1.0),
            mat: mat_code(part),
            part,
        });
    }
    let model = Model::new(g.nx, g.ny, g.nz, h, g.origin, elems, |el, dir| {
        Some((body_of(el.part) * 3 + class_of(dir)) as u16)
    });
    let csr = model.assemble();
    let mut cap = model.capacity();

    // Expected cavity volume (sanity check against a leaking voxel shell).
    let cav_depth = f.pod_width_mm - f.wall_mm;
    let cav_expected = (cav_len * cav_h * cav_depth - ps[0] * ps[1] * ps[2]) / (h * h * h);
    if (cavity_cells as f64) < 0.4 * cav_expected {
        warnings.push(format!(
            "Only {} cavity-air voxels found (expected ≈ {:.0}): the pod shell may leak at this voxel size — use a finer grid.",
            cavity_cells, cav_expected
        ));
    }

    // Heat sources.
    let prof = profile_for(&cfg.scenario, cfg.display_duty);
    let (p0, p1) = duty_split(&prof);
    let bud = pbudget::budget(p, &prof);
    let bud0 = pbudget::budget(p, &p0);
    let bud1 = pbudget::budget(p, &p1);
    let placed = placement(p);
    let disp_ref = pbudget::display_ref(p);
    let bw = ps[1];
    let bh = ps[2];
    // board x → assembly y direction: put J1 (USB) at the USB slot end.
    let slot_c = 0.5 * (l.usb_slot_y[0] + l.usb_slot_y[1]);
    let sgn = placed
        .iter()
        .find(|x| x.refdes == "J1")
        .map(|j| {
            let a = pc[1] + (j.x_mm - bw / 2.0);
            let b = pc[1] - (j.x_mm - bw / 2.0);
            if (a - slot_c).abs() < (b - slot_c).abs() {
                1.0
            } else {
                -1.0
            }
        })
        .unwrap_or(-1.0);
    let board_to_asm = |bx: f64, by: f64, top: bool| -> [f64; 3] {
        [
            if top { pcb_box.hi[0] - 1e-3 } else { pcb_box.lo[0] + 1e-3 },
            pc[1] + sgn * (bx - bw / 2.0),
            pc[2] + (by - bh / 2.0),
        ]
    };
    let nd = model.ndof();
    let mut q0 = vec![0.0; nd];
    let mut q1 = vec![0.0; nd];
    let mut q = vec![0.0; nd];
    let mut sources = Vec::new();
    let mut extra_cap = vec![0.0; nd];
    // Deposit over a rectangle on the board face (bx0..bx1, by0..by1).
    let spread_board = |out: &mut [f64], bx: [f64; 2], by: [f64; 2], top: bool, w: f64| {
        let ns = 4;
        let mut hits = 0;
        let mut pts = Vec::new();
        for a in 0..ns {
            for b in 0..ns {
                let x = bx[0] + (bx[1] - bx[0]) * (a as f64 + 0.5) / ns as f64;
                let y = by[0] + (by[1] - by[0]) * (b as f64 + 0.5) / ns as f64;
                pts.push(board_to_asm(x, y, top));
            }
        }
        let share = w / pts.len() as f64;
        let mut missed = 0.0;
        for pt in &pts {
            if model.deposit(out, *pt, share) {
                hits += 1;
            } else {
                missed += share;
            }
        }
        if hits > 0 && missed > 0.0 {
            // re-spread what missed onto the centre
            let c = board_to_asm(0.5 * (bx[0] + bx[1]), 0.5 * (by[0] + by[1]), top);
            model.deposit(out, c, missed);
        }
        hits > 0
    };
    let spread_box = |out: &mut [f64], b: &Aabb, w: f64| -> bool {
        let ns = 4;
        let mut pts = Vec::new();
        for a in 0..ns {
            for bb in 0..ns {
                for c in 0..2 {
                    let t = [(a as f64 + 0.5) / ns as f64, (bb as f64 + 0.5) / ns as f64, (c as f64 + 0.5) / 2.0];
                    pts.push([
                        b.lo[0] + (b.hi[0] - b.lo[0]) * t[2],
                        b.lo[1] + (b.hi[1] - b.lo[1]) * t[0],
                        b.lo[2] + (b.hi[2] - b.lo[2]) * t[1],
                    ]);
                }
            }
        }
        let share = w / pts.len() as f64;
        let mut any = false;
        let mut missed = 0.0;
        for pt in &pts {
            if model.deposit(out, *pt, share) {
                any = true;
            } else {
                missed += share;
            }
        }
        if any && missed > 0.0 {
            model.deposit(out, b.center(), missed);
        }
        any
    };
    let comp_by_ref = |r: &str| p.components.iter().find(|c| c.refdes == r);
    let pl_by_ref = |r: &str| placed.iter().find(|x| x.refdes == r);
    let body_box = |r: &str, pl: &crate::pcb::Placed| -> ([f64; 2], [f64; 2]) {
        let b = crate::pcb::board::part_footprint(p, r)
            .map(|fp| fp.body)
            .unwrap_or([-1.0, -1.0, 1.0, 1.0]);
        let bb = crate::pcb::board::place_box(b, pl.x_mm, pl.y_mm, pl.rot_deg);
        ([bb[0].max(0.0), bb[2].min(bw)], [bb[1].max(0.0), bb[3].min(bh)])
    };
    let mut place_heat =
        |r: &str, name: &str, loc: &str, w0: f64, w1: f64, w: f64, mass_g: f64, is_display: bool, sources: &mut Vec<Source>| {
            let done_where: String;
            let mut pos = pcb_box.center();
            let targets: [(&mut Vec<f64>, f64); 3] = [(&mut q0, w0), (&mut q1, w1 - w0), (&mut q, w)];
            let into_display = is_display && cfg.display_heat_in != "pod";
            if loc == "left_pod" {
                done_where = "not modelled (battery pod)".into();
            } else if into_display || loc == "front_right" || loc == "front" {
                if into_display || !is_display {
                    for (out, v) in targets {
                        spread_box(out, &display_box, v);
                    }
                    spread_box(&mut extra_cap, &display_box, mass_g * 1e-3 * 750.0);
                    pos = display_box.center();
                    done_where = "display (optics housing)".into();
                } else {
                    // display heat moved to the board at its connector
                    let at = pl_by_ref("J2")
                        .map(|pl| body_box("J2", pl))
                        .unwrap_or(([bw * 0.4, bw * 0.6], [bh * 0.3, bh * 0.7]));
                    for (out, v) in targets {
                        spread_board(out, at.0, at.1, true, v);
                    }
                    spread_board(&mut extra_cap, at.0, at.1, true, mass_g * 1e-3 * 750.0);
                    pos = board_to_asm(0.5 * (at.0[0] + at.0[1]), 0.5 * (at.1[0] + at.1[1]), true);
                    done_where = "board top (at the display connector)".into();
                }
            } else if let Some(pl) = pl_by_ref(r) {
                let (bx, by) = body_box(r, pl);
                let top = pl.side != "bottom";
                for (out, v) in targets {
                    spread_board(out, bx, by, top, v);
                }
                spread_board(&mut extra_cap, bx, by, top, mass_g * 1e-3 * 750.0);
                pos = board_to_asm(0.5 * (bx[0] + bx[1]), 0.5 * (by[0] + by[1]), top);
                done_where = if top { "board top".into() } else { "board bottom".into() };
            } else {
                for (out, v) in targets {
                    spread_board(out, [1.0, bw - 1.0], [1.0, bh - 1.0], true, v);
                }
                done_where = "board (spread)".into();
            }
            sources.push(Source {
                refdes: r.into(),
                name: name.into(),
                location: loc.into(),
                mw: w * 1e3,
                pos,
                placed: done_where,
            });
        };
    let find = |b: &pbudget::Budget, r: &str| b.components.iter().find(|c| c.refdes == r).map(|c| c.avg_mw * 1e-3).unwrap_or(0.0);
    for c in &bud.components {
        let mass = comp_by_ref(&c.refdes).map(|x| x.mass_g * x.qty).unwrap_or(0.0);
        let is_disp = disp_ref.as_deref() == Some(c.refdes.as_str());
        place_heat(
            &c.refdes,
            &c.name,
            &c.location,
            find(&bud0, &c.refdes),
            find(&bud1, &c.refdes),
            c.avg_mw * 1e-3,
            mass,
            is_disp,
            &mut sources,
        );
    }
    // Converter losses + quiescent, for the loads that live outside the battery pod.
    let loss = |b: &pbudget::Budget| {
        let remote: f64 = b.components.iter().filter(|c| c.location == "left_pod").map(|c| c.avg_mw).sum();
        let local = b.load_mw - remote;
        (local / b.buck_efficiency - local + b.quiescent_mw) * 1e-3
    };
    let buck = p
        .components
        .iter()
        .find(|c| {
            c.name.to_lowercase().contains("buck")
                && c.on_pcb
                && !c.name.to_lowercase().contains("inductor")
                && !c.name.to_lowercase().contains("cap")
        })
        .map(|c| c.refdes.clone())
        .unwrap_or_else(|| "U5".into());
    place_heat(
        &buck.clone(),
        "Converter losses (load/η − load) + quiescent",
        "right_pod",
        loss(&bud0),
        loss(&bud1),
        loss(&bud),
        0.0,
        false,
        &mut sources,
    );
    // Passive component masses (no power) still store heat.
    for c in &p.components {
        if c.location == "right_pod" && c.on_pcb && c.power.active_mw <= 0.0 && c.power.sleep_mw <= 0.0 {
            if let Some(pl) = pl_by_ref(&c.refdes) {
                let (bx, by) = body_box(&c.refdes, pl);
                spread_board(&mut extra_cap, bx, by, pl.side != "bottom", c.mass_g * c.qty * 1e-3 * 750.0);
            }
        }
    }
    for (c, e) in cap.iter_mut().zip(&extra_cap) {
        *c += e;
    }
    let total_w: f64 = q.iter().sum();
    let pod_heat = pbudget::pod_heat_mw(p, &prof) * 1e-3;
    if (total_w - pod_heat).abs() > 1e-6 + 1e-6 * pod_heat {
        warnings.push(format!(
            "Placed heat {:.1} mW differs from the pod heat {:.1} mW (some sources fell outside the model).",
            total_w * 1e3,
            pod_heat * 1e3
        ));
    }
    // Characteristic lengths per body: [vertical, A/P, flow], m.
    let pod_w_total = f.pod_width_mm + f.wall_mm;
    let pod_len = f.pod_length_mm;
    let odx = ohi[0] - olo[0];
    let ody = ohi[1] - olo[1];
    let odz = ohi[2] - olo[2];
    let lengths = [
        [
            f.pod_height_mm * 1e-3,
            pod_len * pod_w_total / (2.0 * (pod_len + pod_w_total)) * 1e-3,
            pod_len * 1e-3,
        ],
        [odz * 1e-3, odx * ody / (2.0 * (odx + ody)) * 1e-3, ody * 1e-3],
        [
            l.front_height_mm * 1e-3,
            f.rim_width_mm / 2.0 * 1e-3,
            f.front_thickness_mm.max(1.0) * 1e-3,
        ],
    ];
    Ok(PodModel {
        model,
        csr,
        cap,
        q,
        q0,
        q1,
        sources,
        total_w,
        geom: geom.clone(),
        cfg: cfg.clone(),
        pcb_box,
        display_box,
        spreader_box,
        spreader_volume_mm3,
        shell,
        pcb_mat,
        spreader_mat,
        k_cavity: k_air,
        k_cavity_rad: k_rad,
        lengths,
        cavity_cells,
        cavity_expected_cells: cav_expected,
        warnings,
        part_of,
    })
}

// ------------------------------------------------------------------ boundary coefficients

/// Convection/radiation coefficients of one face group.
#[derive(Clone, Debug, Serialize)]
pub struct HGroup {
    pub body: String,
    pub class: FaceClass,
    pub area_mm2: f64,
    /// Area-weighted mean surface excess, K.
    pub theta_mean: f64,
    pub h_conv: f64,
    pub h_rad: f64,
    pub h_total: f64,
    pub ra: f64,
    pub length_mm: f64,
    pub valid: bool,
    /// Heat leaving through this group, mW.
    pub q_mw: f64,
}

#[allow(clippy::needless_range_loop)] // several arrays indexed by the same body
fn coefficients(pm: &PodModel, means: &[(f64, f64)]) -> Vec<HGroup> {
    let cfg = &pm.cfg;
    let mut out = Vec::new();
    for b in 0..3 {
        for (c, cls) in CLASSES.iter().enumerate() {
            let gi = b * 3 + c;
            let (s, a) = means.get(gi).copied().unwrap_or((0.0, 0.0));
            let th = if a > 0.0 { s / a } else { 0.0 }.max(0.05);
            let lc = if *cls == FaceClass::Vertical {
                pm.lengths[b][0]
            } else {
                pm.lengths[b][1]
            };
            let (hn, ra, _, ok) = h_natural(*cls, th, cfg.ambient_c, lc);
            let hc = match cfg.h_override {
                Some(hh) => hh,
                None => {
                    if cfg.air_speed_m_s > 0.0 {
                        let (hf, _, _) = h_forced_plate(cfg.air_speed_m_s, cfg.ambient_c + th / 2.0, pm.lengths[b][2]);
                        blend(hn, hf)
                    } else {
                        hn
                    }
                }
            };
            let hr = h_radiation(cfg.emissivity, cfg.ambient_c + th + K0, cfg.ambient_c + K0);
            out.push(HGroup {
                body: BODY_NAMES[b].into(),
                class: *cls,
                area_mm2: a * 1e6,
                theta_mean: th,
                h_conv: hc,
                h_rad: hr,
                h_total: hc + hr,
                ra,
                length_mm: lc * 1e3,
                valid: ok || cfg.h_override.is_some(),
                q_mw: 0.0,
            });
        }
    }
    out
}

/// Steady solve with Picard updates of h. Returns θ, coefficients, PCG stats.
pub fn solve_steady(pm: &PodModel, q: &[f64], passes: usize, warm: Option<&[f64]>) -> (Vec<f64>, Vec<HGroup>, fea::PcgStats, usize) {
    let pc = Precond::parse(&pm.cfg.precond);
    let ng = 9;
    // start from a 5 K guess on every group
    let mut means = vec![(5.0, 1.0); ng];
    // A warm start only helps when the mesh is the same; a spreader or bracket
    // that fills extra voxels changes the node count, so start cold then.
    let warm = warm.filter(|w| w.len() == pm.model.ndof());
    let mut theta = warm.map(|w| w.to_vec()).unwrap_or_else(|| vec![0.0; pm.model.ndof()]);
    if warm.is_some() {
        means = pm.model.face_group_means(&theta, ng);
    }
    let mut stats = fea::PcgStats::default();
    let mut groups = Vec::new();
    let mut total_iters = 0;
    for pass in 0..passes.max(1) {
        groups = coefficients(pm, &means);
        let hs: Vec<f64> = groups.iter().map(|g| g.h_total).collect();
        let d = pm.model.face_diag(&hs);
        let tol = if pass + 1 == passes.max(1) {
            pm.cfg.tol
        } else {
            (pm.cfg.tol * 100.0).min(1e-5)
        };
        let st = fea::pcg(&pm.csr, &d, q, &mut theta, pc, tol, pm.cfg.max_iter);
        total_iters += st.iterations;
        if pass == 0 || pass + 1 == passes.max(1) {
            stats = if pass == 0 {
                st
            } else {
                fea::PcgStats {
                    history: stats.history.clone(),
                    ..st
                }
            };
        }
        means = pm.model.face_group_means(&theta, ng);
    }
    // heat leaving per group at the final coefficients
    let hm = pm.model.h_mm * 1e-3;
    let a4 = hm * hm / 4.0;
    for f in &pm.model.faces {
        let gi = f.group as usize;
        let t: f64 = pm.model.face_dofs(f).iter().map(|&n| theta[n as usize]).sum::<f64>();
        groups[gi].q_mw += groups[gi].h_total * a4 * t * 1e3;
    }
    (theta, groups, stats, total_iters)
}

// ------------------------------------------------------------------ results

#[derive(Clone, Debug, Serialize)]
pub struct PartStat {
    pub part: String,
    pub peak_c: f64,
    pub mean_c: f64,
    pub area_mm2: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Surface {
    pub name: String,
    pub label: String,
    /// base64 little-endian f32 xyz (assembly mm).
    pub positions: String,
    /// base64 little-endian u32 triangle indices.
    pub indices: String,
    /// base64 little-endian f32 temperatures, °C, per vertex.
    pub temps: String,
    pub vertices: usize,
    pub triangles: usize,
}

/// Nodal temperature volume for per-pixel colouring of the CAD surfaces:
/// grid-node values (inactive nodes take the value of the nearest active
/// node), quantised 0..255 over [t_lo, t_hi], index i + nx (j + ny k).
#[derive(Clone, Debug, Serialize)]
pub struct Volume {
    /// Node counts per axis.
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    /// Position of node (0, 0, 0), mm.
    pub origin: [f64; 3],
    pub h_mm: f64,
    pub data: String,
    pub t_lo: f64,
    pub t_hi: f64,
}

/// Breadth-first fill order that gives every inactive grid node the value of
/// a nearby active node: (target, source) pairs.
pub fn dilation_order(md: &Model) -> Vec<(u32, u32)> {
    let (nx1, ny1, nz1) = (md.nx + 1, md.ny + 1, md.nz + 1);
    let n = nx1 * ny1 * nz1;
    let mut seen = vec![false; n];
    let mut queue = std::collections::VecDeque::new();
    for &node in &md.node_of {
        seen[node as usize] = true;
        queue.push_back(node as usize);
    }
    let mut order = Vec::with_capacity(n - md.node_of.len());
    while let Some(v) = queue.pop_front() {
        let (i, j, k) = (v % nx1, (v / nx1) % ny1, v / (nx1 * ny1));
        let nb = [
            (i > 0).then(|| v - 1),
            (i + 1 < nx1).then(|| v + 1),
            (j > 0).then(|| v - nx1),
            (j + 1 < ny1).then(|| v + nx1),
            (k > 0).then(|| v - nx1 * ny1),
            (k + 1 < nz1).then(|| v + nx1 * ny1),
        ];
        for w in nb.into_iter().flatten() {
            if !seen[w] {
                seen[w] = true;
                order.push((w as u32, v as u32));
                queue.push_back(w);
            }
        }
    }
    order
}

/// Quantised nodal volume of the excess field θ (°C above ambient `ta`).
pub fn volume(md: &Model, theta: &[f64], order: &[(u32, u32)], ta: f64, t_hi: f64) -> Volume {
    let n = (md.nx + 1) * (md.ny + 1) * (md.nz + 1);
    let mut v = vec![0.0f64; n];
    for (dof, &node) in md.node_of.iter().enumerate() {
        v[node as usize] = theta[dof];
    }
    for &(t, f) in order {
        v[t as usize] = v[f as usize];
    }
    let span = (t_hi - ta).max(1e-6);
    let bytes: Vec<u8> = v.iter().map(|x| ((x / span).clamp(0.0, 1.0) * 255.0).round() as u8).collect();
    Volume {
        nx: md.nx + 1,
        ny: md.ny + 1,
        nz: md.nz + 1,
        origin: md.origin,
        h_mm: md.h_mm,
        data: crate::cad::export::base64(&bytes),
        t_lo: ta,
        t_hi,
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Slices {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    pub h_mm: f64,
    pub origin: [f64; 3],
    /// Element temperature quantised 0..255 over [t_lo, t_hi], base64, index i + nx (j + ny k).
    pub temp_u8: String,
    /// Material code per element (0 outside, 1 shell, 2 air, 3 PCB, 4 spreader, 5 display), base64.
    pub mat_u8: String,
    pub t_lo: f64,
    pub t_hi: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Lumped {
    /// Lumped two-node model (Power Lab) with the same heat: shell and board.
    pub shell_c: f64,
    pub board_c: f64,
    pub r_pod_ambient: f64,
    pub r_board_pod: f64,
    /// FEA-equivalent resistances: mean and peak shell excess ÷ heat, °C/W.
    pub r_eff_mean: f64,
    pub r_eff_peak: f64,
    /// Exposed shell area, mm², and the mean h the FEA found, W/m² K.
    pub area_mm2: f64,
    pub h_mean: f64,
    /// h that would make the lumped R_pa true for this area.
    pub h_equiv_lumped: f64,
    /// Spreading length of the shell wall √(k t / h), mm, without / with the spreader.
    pub spread_len_mm: f64,
    pub spread_len_spreader_mm: f64,
    pub explanation: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SteadyOut {
    pub config: Resolved,
    pub ambient_c: f64,
    pub skin_limit_c: f64,
    pub total_mw: f64,
    pub pod_heat_mw: f64,
    pub peak_skin_c: f64,
    pub peak_part: String,
    pub peak_pos: [f64; 3],
    pub mean_skin_c: f64,
    pub hotspot_over_mean_c: f64,
    pub board_max_c: f64,
    pub board_max_pos: [f64; 3],
    pub display_max_c: f64,
    pub air_max_c: f64,
    pub parts: Vec<PartStat>,
    pub groups: Vec<HGroup>,
    pub heat_out_mw: f64,
    pub sources: Vec<Source>,
    pub elements: usize,
    pub dofs: usize,
    pub nnz: usize,
    pub voxel_mm: f64,
    pub cavity_cells: usize,
    pub cavity_expected_cells: f64,
    pub iterations: usize,
    pub iterations_total: usize,
    pub converged: bool,
    pub residuals: Vec<f64>,
    pub precond: String,
    pub materials: Vec<ThermalMaterial>,
    pub k_cavity: f64,
    pub k_cavity_rad: f64,
    pub cavity_rayleigh: f64,
    pub lumped: Lumped,
    pub surfaces: Vec<Surface>,
    pub inner: Vec<Surface>,
    pub voxel_surface: Option<Surface>,
    /// Nodal temperature volume (with the surfaces) for per-pixel colouring.
    pub volume: Option<Volume>,
    pub slices: Option<Slices>,
    pub bbox_min: [f64; 3],
    pub bbox_max: [f64; 3],
    pub pcb_box: Aabb,
    pub display_box: Aabb,
    pub spreader_box: Option<Aabb>,
    pub warnings: Vec<String>,
}

fn b64_f32(v: &[f32]) -> String {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for x in v {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    crate::cad::export::base64(&bytes)
}
fn b64_u32(v: &[u32]) -> String {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for x in v {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    crate::cad::export::base64(&bytes)
}

/// Hottest point of the display module (sampled inside its box, robust when
/// coarse voxels tag the display's cells as the housing wall).
pub fn display_max(pm: &PodModel, theta: &[f64]) -> f64 {
    let b = &pm.display_box;
    let mut m = f64::NEG_INFINITY;
    for a in 0..5 {
        for c in 0..5 {
            for d in 0..3 {
                let t = [(d as f64 + 0.5) / 3.0, (a as f64 + 0.5) / 5.0, (c as f64 + 0.5) / 5.0];
                let pt = std::array::from_fn(|k| b.lo[k] + (b.hi[k] - b.lo[k]) * t[k]);
                if let Some(v) = pm.model.sample(theta, pt) {
                    m = m.max(v);
                }
            }
        }
    }
    m
}

/// Shell surface statistics from exposed faces of shell elements.
pub struct SkinStats {
    pub peak: f64,
    pub peak_dof: u32,
    pub peak_part: u8,
    pub mean: f64,
    pub area: f64,
    pub per_part: [(f64, f64, f64); 4],
}

pub fn skin_stats(pm: &PodModel, theta: &[f64]) -> SkinStats {
    let hm = pm.model.h_mm * 1e-3;
    let a = hm * hm;
    let mut st = SkinStats {
        peak: f64::NEG_INFINITY,
        peak_dof: 0,
        peak_part: 0,
        mean: 0.0,
        area: 0.0,
        per_part: [(f64::NEG_INFINITY, 0.0, 0.0); 4],
    };
    for f in &pm.model.faces {
        let el = &pm.model.elems[f.elem as usize];
        if el.part > PART_FRONT {
            continue;
        }
        let dofs = pm.model.face_dofs(f);
        let mut s = 0.0;
        for &d in &dofs {
            let t = theta[d as usize];
            s += t;
            if t > st.peak {
                st.peak = t;
                st.peak_dof = d;
                st.peak_part = el.part;
            }
            let pp = &mut st.per_part[el.part as usize];
            pp.0 = pp.0.max(t);
        }
        st.mean += s / 4.0 * a;
        st.area += a;
        let pp = &mut st.per_part[el.part as usize];
        pp.1 += s / 4.0 * a;
        pp.2 += a;
    }
    if st.area > 0.0 {
        st.mean /= st.area;
    }
    st
}

fn surface_from_mesh(name: &str, label: &str, m: &crate::cad::Mesh, pm: &PodModel, theta: &[f64], ta: f64) -> Surface {
    let pos: Vec<f32> = m.positions.iter().map(|v| *v as f32).collect();
    let temps: Vec<f32> = (0..m.vertex_count())
        .map(|i| (ta + pm.model.sample(theta, m.vertex(i as u32)).unwrap_or(0.0)) as f32)
        .collect();
    Surface {
        name: name.into(),
        label: label.into(),
        positions: b64_f32(&pos),
        indices: b64_u32(&m.indices),
        temps: b64_f32(&temps),
        vertices: m.vertex_count(),
        triangles: m.triangle_count(),
    }
}

/// Refine a mesh by longest-edge bisection until no edge is longer than
/// `max_edge` (edge midpoints shared through a global map, so neighbours
/// that split the same edge share the vertex). CAD triangles can span a whole
/// pod wall — too coarse to carry a hotspot — and slivers must not explode
/// into n² pieces.
pub fn subdivide(m: &crate::cad::Mesh, max_edge: f64) -> crate::cad::Mesh {
    let mut out = crate::cad::Mesh {
        positions: m.positions.clone(),
        indices: Vec::new(),
    };
    let lim2 = max_edge.max(0.05).powi(2);
    let mut mids: std::collections::HashMap<(u32, u32), u32> = std::collections::HashMap::new();
    let d2 = |o: &crate::cad::Mesh, a: u32, b: u32| {
        let (p, q) = (o.vertex(a), o.vertex(b));
        (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)
    };
    let mut stack: Vec<([u32; 3], u32)> = (0..m.triangle_count())
        .map(|t| ([m.indices[3 * t], m.indices[3 * t + 1], m.indices[3 * t + 2]], 0))
        .collect();
    while let Some((tri, depth)) = stack.pop() {
        let e = [d2(&out, tri[0], tri[1]), d2(&out, tri[1], tri[2]), d2(&out, tri[2], tri[0])];
        let (k, &emax) = e.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap();
        if emax <= lim2 || depth >= 24 {
            out.indices.extend_from_slice(&tri);
            continue;
        }
        let (a, b, c) = (tri[k], tri[(k + 1) % 3], tri[(k + 2) % 3]);
        let key = (a.min(b), a.max(b));
        let mid = match mids.get(&key) {
            Some(&v) => v,
            None => {
                let (p, q) = (out.vertex(a), out.vertex(b));
                let v = out.add_vertex([(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0, (p[2] + q[2]) / 2.0]);
                mids.insert(key, v);
                v
            }
        };
        stack.push(([a, mid, c], depth + 1));
        stack.push(([mid, b, c], depth + 1));
    }
    out
}

/// A subdivided box surface (for the board and the display).
fn box_mesh(b: &Aabb, step: f64) -> crate::cad::Mesh {
    let mut m = crate::cad::Mesh::default();
    for ax in 0..3 {
        for side in 0..2 {
            let (u, v) = ((ax + 1) % 3, (ax + 2) % 3);
            let nu = (((b.hi[u] - b.lo[u]) / step).ceil() as usize).clamp(1, 200);
            let nv = (((b.hi[v] - b.lo[v]) / step).ceil() as usize).clamp(1, 200);
            let base = m.vertex_count() as u32;
            for j in 0..=nv {
                for i in 0..=nu {
                    let mut p = [0.0; 3];
                    p[ax] = if side == 0 { b.lo[ax] } else { b.hi[ax] };
                    p[u] = b.lo[u] + (b.hi[u] - b.lo[u]) * i as f64 / nu as f64;
                    p[v] = b.lo[v] + (b.hi[v] - b.lo[v]) * j as f64 / nv as f64;
                    m.add_vertex(p);
                }
            }
            for j in 0..nv {
                for i in 0..nu {
                    let a = base + (j * (nu + 1) + i) as u32;
                    let bq = a + 1;
                    let c = a + (nu + 1) as u32;
                    let d = c + 1;
                    if side == 1 {
                        m.add_tri(a, bq, d);
                        m.add_tri(a, d, c);
                    } else {
                        m.add_tri(a, d, bq);
                        m.add_tri(a, c, d);
                    }
                }
            }
        }
    }
    m
}

/// Exposed voxel faces as a mesh (vertices = grid nodes).
fn voxel_surface(pm: &PodModel, theta: &[f64], ta: f64) -> Surface {
    let md = &pm.model;
    let mut vmap = std::collections::HashMap::new();
    let mut pos = Vec::new();
    let mut temps = Vec::new();
    let mut idx = Vec::new();
    for f in md.faces.iter().chain(md.cut_faces.iter()) {
        let el = &md.elems[f.elem as usize];
        let nodes = md.elem_nodes(el.e as usize);
        let fl = fea::face_nodes(f.dir as usize);
        // quad order around the face: 0,1,3,2 of the face node list
        let quad = [nodes[fl[0]], nodes[fl[1]], nodes[fl[3]], nodes[fl[2]]];
        let mut vid = [0u32; 4];
        for (k, &nn) in quad.iter().enumerate() {
            let id = *vmap.entry(nn).or_insert_with(|| {
                let p = md.node_pos(nn);
                pos.extend_from_slice(&[p[0] as f32, p[1] as f32, p[2] as f32]);
                temps.push((ta + theta[md.dof_of[nn] as usize]) as f32);
                (temps.len() - 1) as u32
            });
            vid[k] = id;
        }
        idx.extend_from_slice(&[vid[0], vid[1], vid[2], vid[0], vid[2], vid[3]]);
    }
    Surface {
        name: "voxels".into(),
        label: "Voxel surface".into(),
        vertices: temps.len(),
        triangles: idx.len() / 3,
        positions: b64_f32(&pos),
        indices: b64_u32(&idx),
        temps: b64_f32(&temps),
    }
}

fn slices(pm: &PodModel, theta: &[f64], ta: f64, t_hi: f64) -> Slices {
    let md = &pm.model;
    let n = md.nx * md.ny * md.nz;
    let mut tq = vec![0u8; n];
    let mut mq = vec![0u8; n];
    let span = (t_hi - ta).max(1e-6);
    for el in &md.elems {
        let nodes = md.elem_nodes(el.e as usize);
        let t: f64 = nodes.iter().map(|&nn| theta[md.dof_of[nn] as usize]).sum::<f64>() / 8.0;
        tq[el.e as usize] = ((t / span).clamp(0.0, 1.0) * 255.0).round() as u8;
        mq[el.e as usize] = el.mat;
    }
    Slices {
        nx: md.nx,
        ny: md.ny,
        nz: md.nz,
        h_mm: md.h_mm,
        origin: md.origin,
        temp_u8: crate::cad::export::base64(&tq),
        mat_u8: crate::cad::export::base64(&mq),
        t_lo: ta,
        t_hi,
    }
}

/// What to include in a steady result.
#[derive(Clone, Copy, Debug, Default)]
pub struct Want {
    pub surfaces: bool,
    pub voxels: bool,
    pub slices: bool,
}

pub fn steady(p: &Project, cfg: &Resolved, want: Want) -> Result<SteadyOut, String> {
    let pm = build(p, cfg)?;
    let (theta, groups, stats, iters_total) = solve_steady(&pm, &pm.q, 3, None);
    Ok(report(p, &pm, &theta, groups, stats, iters_total, want))
}

pub fn report(
    p: &Project,
    pm: &PodModel,
    theta: &[f64],
    groups: Vec<HGroup>,
    stats: fea::PcgStats,
    iters_total: usize,
    want: Want,
) -> SteadyOut {
    let cfg = &pm.cfg;
    let ta = cfg.ambient_c;
    let md = &pm.model;
    let sk = skin_stats(pm, theta);
    let disp_sampled = display_max(pm, theta);
    let mut board = (f64::NEG_INFINITY, 0u32);
    let mut disp = f64::NEG_INFINITY;
    let mut airmax = f64::NEG_INFINITY;
    for (ae, el) in md.elems.iter().enumerate() {
        for d in md.elem_dofs(ae) {
            let t = theta[d as usize];
            match el.part {
                PART_PCB if t > board.0 => board = (t, d),
                PART_DISPLAY => disp = disp.max(t),
                PART_AIR => airmax = airmax.max(t),
                _ => {}
            }
        }
    }
    let prof = profile_for(&cfg.scenario, cfg.display_duty);
    let pod_heat = pbudget::pod_heat_mw(p, &prof);
    let th = &p.design.thermal;
    let r_pa = th.r_pod_ambient_c_per_w.max(1e-6);
    let r_bp = th.r_board_pod_c_per_w.max(1e-6);
    let pw = pm.total_w.max(1e-12);
    let a_shell = sk.area;
    let heat_out: f64 = groups.iter().map(|g| g.q_mw).sum();
    let h_mean = if sk.mean > 0.0 && a_shell > 0.0 {
        pw / (sk.mean * a_shell)
    } else {
        0.0
    };
    let wall = p.design.frame.wall_mm * 1e-3;
    let spread = (pm.shell.k_in * wall / h_mean.max(1e-6)).sqrt() * 1e3;
    let spread_s = pm
        .spreader_mat
        .as_ref()
        .map(|m| ((pm.shell.k_in * wall + m.k_in * cfg.spreader_um * 1e-6) / h_mean.max(1e-6)).sqrt() * 1e3)
        .unwrap_or(spread);
    let lumped_shell = ta + pod_heat * 1e-3 * r_pa;
    let peak_c = ta + sk.peak;
    let mean_c = ta + sk.mean;
    let mut explanation = vec![
        format!(
            "The lumped model puts all {:.0} mW through one shell node at one temperature: {:.1} °C = {:.1} °C + {:.0} mW × {:.0} °C/W. The 3-D model spreads the same heat over the real geometry.",
            pod_heat,
            lumped_shell,
            ta,
            pod_heat,
            r_pa
        ),
        format!(
            "Mean shell temperature {:.1} °C corresponds to R = {:.0} °C/W over {:.0} cm² at a mean h = {:.1} W/m² K (convection + radiation). For the lumped 55 °C/W to hold, h would have to be {:.1} W/m² K on that area.",
            mean_c,
            sk.mean / pw,
            a_shell * 1e4,
            h_mean,
            1.0 / (r_pa * a_shell.max(1e-9))
        ),
        format!(
            "Hotspot: plastic conducts poorly (k = {:.2} W/m K), so heat does not spread far along a {:.1} mm wall — the spreading length √(k t / h) is only {:.1} mm. The surface right over the source runs {:.1} °C above the mean; a single-node model cannot see that by construction.",
            pm.shell.k_in,
            wall * 1e3,
            spread,
            sk.peak - sk.mean
        ),
    ];
    if pm.spreader_mat.is_some() {
        explanation.push(format!(
            "With the {} ({:.0} µm) the spreading length grows to {:.1} mm.",
            pm.spreader_mat.as_ref().map(|m| m.name.as_str()).unwrap_or(""),
            cfg.spreader_um,
            spread_s
        ));
    }
    if cfg.display_heat_in != "pod" {
        let disp_mw: f64 = pm.sources.iter().filter(|s| s.placed.starts_with("display")).map(|s| s.mw).sum();
        if disp_mw > 0.0 {
            explanation.push(format!(
                "The Iris bill of materials places the display ({:.0} mW) in the optics housing at the front, not in the pod — the lumped model lumps it into the pod. Switch \"display heat in\" to \"pod\" to compare like for like.",
                disp_mw
            ));
        }
    }
    let lumped = Lumped {
        shell_c: lumped_shell,
        board_c: ta + pod_heat * 1e-3 * (r_pa + r_bp),
        r_pod_ambient: r_pa,
        r_board_pod: r_bp,
        r_eff_mean: sk.mean / pw,
        r_eff_peak: sk.peak / pw,
        area_mm2: a_shell * 1e6,
        h_mean,
        h_equiv_lumped: 1.0 / (r_pa * a_shell.max(1e-9)),
        spread_len_mm: spread,
        spread_len_spreader_mm: spread_s,
        explanation,
    };
    let parts: Vec<PartStat> = (0..4)
        .filter(|&i| sk.per_part[i].2 > 0.0)
        .map(|i| PartStat {
            part: PART_NAMES[i].into(),
            peak_c: ta + sk.per_part[i].0,
            mean_c: ta + sk.per_part[i].1 / sk.per_part[i].2,
            area_mm2: sk.per_part[i].2 * 1e6,
        })
        .collect();
    let t_hi = ta + theta.iter().cloned().fold(0.0, f64::max);
    let surfaces = if want.surfaces {
        pm.geom
            .cad
            .iter()
            .enumerate()
            .map(|(i, (n, m))| surface_from_mesh(n, PART_NAMES[i], m, pm, theta, ta))
            .collect()
    } else {
        vec![]
    };
    let inner = if want.surfaces {
        let step = 4.0;
        let mut v = vec![
            surface_from_mesh("pcb", PART_NAMES[4], &box_mesh(&pm.pcb_box, step), pm, theta, ta),
            surface_from_mesh("display", PART_NAMES[5], &box_mesh(&pm.display_box, step), pm, theta, ta),
        ];
        if let Some(b) = &pm.spreader_box {
            v.push(surface_from_mesh(
                "spreader",
                PART_NAMES[7],
                &box_mesh(b, step * 2.0),
                pm,
                theta,
                ta,
            ));
        }
        v
    } else {
        vec![]
    };
    let air = props::air(ta + K0 + 5.0);
    // cavity Rayleigh number for a 2 mm board-to-wall gap with the board–shell difference
    let dtb = (board.0 - sk.mean).max(0.0);
    let cavity_ra = super::convection::rayleigh(&air, dtb, 2e-3);
    let mut materials = vec![pm.shell.clone(), pm.pcb_mat.clone(), props::display_material()];
    if let Some(s) = &pm.spreader_mat {
        materials.push(s.clone());
    }
    let mut warnings = pm.warnings.clone();
    for g in &groups {
        if !g.valid && g.area_mm2 > 0.0 {
            warnings.push(format!(
                "{} {:?} faces: Ra = {:.0} is below the correlation's validated range — extrapolated.",
                g.body, g.class, g.ra
            ));
        }
    }
    if !stats.converged {
        warnings.push("Conjugate gradient did not reach the tolerance; results are approximate.".into());
    }
    let (lo, hi) = (
        md.origin,
        [
            md.origin[0] + md.nx as f64 * md.h_mm,
            md.origin[1] + md.ny as f64 * md.h_mm,
            md.origin[2] + md.nz as f64 * md.h_mm,
        ],
    );
    SteadyOut {
        config: cfg.clone(),
        ambient_c: ta,
        skin_limit_c: th.skin_limit_c,
        total_mw: pm.total_w * 1e3,
        pod_heat_mw: pod_heat,
        peak_skin_c: peak_c,
        peak_part: PART_NAMES[sk.peak_part as usize].into(),
        peak_pos: md.node_pos(md.node_of[sk.peak_dof as usize] as usize),
        mean_skin_c: mean_c,
        hotspot_over_mean_c: sk.peak - sk.mean,
        board_max_c: ta + board.0.max(0.0),
        board_max_pos: md.node_pos(md.node_of[board.1 as usize] as usize),
        display_max_c: ta + disp.max(disp_sampled).max(0.0),
        air_max_c: ta + airmax.max(0.0),
        parts,
        heat_out_mw: heat_out,
        groups,
        sources: pm.sources.clone(),
        elements: md.elems.len(),
        dofs: md.ndof(),
        nnz: pm.csr.nnz(),
        voxel_mm: md.h_mm,
        cavity_cells: pm.cavity_cells,
        cavity_expected_cells: pm.cavity_expected_cells,
        iterations: stats.iterations,
        iterations_total: iters_total,
        converged: stats.converged,
        residuals: stats.history,
        precond: Precond::parse(&cfg.precond).name().into(),
        materials,
        k_cavity: pm.k_cavity,
        k_cavity_rad: pm.k_cavity_rad,
        cavity_rayleigh: cavity_ra,
        lumped,
        surfaces,
        inner,
        voxel_surface: want.voxels.then(|| voxel_surface(pm, theta, ta)),
        volume: want
            .surfaces
            .then(|| volume(md, theta, &dilation_order(md), ta, t_hi.max(ta + 0.01))),
        slices: want.slices.then(|| slices(pm, theta, ta, t_hi)),
        bbox_min: lo,
        bbox_max: hi,
        pcb_box: pm.pcb_box,
        display_box: pm.display_box,
        spreader_box: pm.spreader_box,
        warnings,
    }
}

// ------------------------------------------------------------------ transient

/// One piece of a use scenario: hold the display duty for a while.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Segment {
    pub duration_s: f64,
    pub display_duty: f64,
    #[serde(default)]
    pub label: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct TransientIn {
    pub project: Option<Project>,
    pub config: PodConfig,
    /// Named preset: commute | captions | r008 (ignored when `segments` is given).
    pub preset: Option<String>,
    pub segments: Vec<Segment>,
    pub dt_s: Option<f64>,
    pub frames: Option<usize>,
}

pub fn preset_segments(id: &str) -> Vec<Segment> {
    let s = |d: f64, x: f64, l: &str| Segment {
        duration_s: d,
        display_duty: x,
        label: l.into(),
    };
    match id {
        "captions" => vec![s(2700.0, 1.0, "meeting with live captions"), s(900.0, 0.1, "break")],
        "commute" => vec![
            s(600.0, 0.6, "navigation walk"),
            s(900.0, 0.1, "on the train"),
            s(600.0, 0.6, "navigation walk"),
            s(600.0, 0.0, "at the desk"),
        ],
        _ => vec![s(3600.0, 1.0, "display always on (R-008 case)")],
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TransientOut {
    pub t_s: Vec<f64>,
    pub peak_skin_c: Vec<f64>,
    pub mean_skin_c: Vec<f64>,
    pub board_max_c: Vec<f64>,
    pub display_max_c: Vec<f64>,
    pub power_mw: Vec<f64>,
    pub duty: Vec<f64>,
    pub lumped_shell_c: Vec<f64>,
    pub lumped_board_c: Vec<f64>,
    pub segments: Vec<Segment>,
    /// Frame times and nodal temperature volumes (see [`Volume`]: node counts
    /// `volume_dims`, origin `volume_origin`, pitch `volume_h_mm`), quantised
    /// 0..255 over [t_lo, t_hi], base64 per frame.
    pub frame_t_s: Vec<f64>,
    pub frames: Vec<String>,
    pub volume_dims: [usize; 3],
    pub volume_origin: [f64; 3],
    pub volume_h_mm: f64,
    pub t_lo: f64,
    pub t_hi: f64,
    pub surfaces: Vec<Surface>,
    pub inner: Vec<Surface>,
    pub steady_peak_c: f64,
    pub time_to_limit_s: Option<f64>,
    pub skin_limit_c: f64,
    pub ambient_c: f64,
    pub iterations_per_step: f64,
    pub dt_s: f64,
    pub bbox_min: [f64; 3],
    pub bbox_max: [f64; 3],
    pub note: String,
}

pub fn transient(i: TransientIn) -> Result<TransientOut, String> {
    let p = i.project.unwrap_or_else(Project::iris);
    let mut cfg = i.config.resolve(&p);
    cfg.voxel_mm = cfg.voxel_mm.max(1.0);
    let segs = if i.segments.is_empty() {
        preset_segments(i.preset.as_deref().unwrap_or("r008"))
    } else {
        i.segments.clone()
    };
    let duration: f64 = segs.iter().map(|s| s.duration_s.max(0.0)).sum();
    if duration <= 0.0 {
        return Err("the scenario has no duration".into());
    }
    if duration > 48.0 * 3600.0 {
        return Err("keep the scenario under 48 hours".into());
    }
    let dt = i.dt_s.unwrap_or((duration / 240.0).clamp(5.0, 120.0)).clamp(0.5, 600.0);
    let steps = ((duration / dt).ceil() as usize).min(3000);
    let nframes = i.frames.unwrap_or(48).clamp(2, 90);
    // Model with the display at duty 0 / 1 split (heat is linear in duty).
    cfg.scenario = if cfg.scenario == "display_on" {
        "reference".into()
    } else {
        cfg.scenario
    };
    cfg.display_duty = Some(segs.iter().map(|s| s.display_duty).fold(0.0, f64::max));
    let pm = build(&p, &cfg)?;
    // Linearise h about the hottest steady state of the scenario.
    let (theta_ss, groups, _, _) = solve_steady(&pm, &pm.q, 3, None);
    let sk_ss = skin_stats(&pm, &theta_ss);
    let hs: Vec<f64> = groups.iter().map(|g| g.h_total).collect();
    let hd = pm.model.face_diag(&hs);
    let diag: Vec<f64> = hd.iter().zip(&pm.cap).map(|(h, c)| h + c / dt).collect();
    let pc = Precond::parse(&cfg.precond);
    let duty_at = |t: f64| -> f64 {
        let mut acc = 0.0;
        for s in &segs {
            acc += s.duration_s.max(0.0);
            if t < acc {
                return s.display_duty.clamp(0.0, 1.0);
            }
        }
        segs.last().map(|s| s.display_duty).unwrap_or(0.0)
    };
    let ta = cfg.ambient_c;
    let th = &p.design.thermal;
    // lumped reference on the same schedule (Power Lab network)
    let net = crate::power::thermal::Network::from_params(th);
    let phi = net.expm(dt);
    let prof = profile_for(&cfg.scenario, None);
    let heat_lumped = |d: f64| crate::power::thermal::heat_at_duty(&p, &prof, d) * 1e-3;
    let mut xl = [0.0, 0.0];
    let mut theta = vec![0.0; pm.model.ndof()];
    let mut out = TransientOut {
        t_s: vec![],
        peak_skin_c: vec![],
        mean_skin_c: vec![],
        board_max_c: vec![],
        display_max_c: vec![],
        power_mw: vec![],
        duty: vec![],
        lumped_shell_c: vec![],
        lumped_board_c: vec![],
        segments: segs.clone(),
        frame_t_s: vec![],
        frames: vec![],
        volume_dims: [0; 3],
        volume_origin: pm.model.origin,
        volume_h_mm: pm.model.h_mm,
        t_lo: ta,
        t_hi: ta + sk_ss.peak.max(0.1),
        surfaces: vec![],
        inner: vec![],
        steady_peak_c: ta + sk_ss.peak,
        time_to_limit_s: None,
        skin_limit_c: th.skin_limit_c,
        ambient_c: ta,
        iterations_per_step: 0.0,
        dt_s: dt,
        bbox_min: pm.model.origin,
        bbox_max: std::array::from_fn(|d| pm.model.origin[d] + [pm.model.nx, pm.model.ny, pm.model.nz][d] as f64 * pm.model.h_mm),
        note: format!(
            "Implicit Euler, Δt = {:.0} s; convection and radiation linearised about the hottest steady state of the scenario (display duty {:.2}).",
            dt,
            cfg.display_duty.unwrap_or(1.0)
        ),
    };
    // surfaces: CAD parts + inner boxes; frames are nodal volumes
    let mut meshes: Vec<&crate::cad::Mesh> = pm.geom.cad.iter().map(|(_, m)| m).collect();
    let pcb_m = box_mesh(&pm.pcb_box, 4.0);
    let dsp_m = box_mesh(&pm.display_box, 4.0);
    meshes.push(&pcb_m);
    meshes.push(&dsp_m);
    let spr_m = pm.spreader_box.as_ref().map(|b| box_mesh(b, 8.0));
    if let Some(m) = &spr_m {
        meshes.push(m);
    }
    let order = dilation_order(&pm.model);
    let frame_every = (steps / (nframes - 1)).max(1);
    let mut iters = 0usize;
    let t_hi = out.t_hi;
    let push_frame = |theta: &[f64], t: f64, out: &mut TransientOut| {
        let v = volume(&pm.model, theta, &order, ta, t_hi);
        out.volume_dims = [v.nx, v.ny, v.nz];
        out.frames.push(v.data);
        out.frame_t_s.push(t);
    };
    let record = |theta: &[f64], t: f64, d: f64, xl: [f64; 2], out: &mut TransientOut| {
        let sk = skin_stats(&pm, theta);
        let (mut b, mut dm) = (0.0f64, 0.0f64);
        for (ae, el) in pm.model.elems.iter().enumerate() {
            if el.part == PART_PCB || el.part == PART_DISPLAY {
                for dd in pm.model.elem_dofs(ae) {
                    if el.part == PART_PCB {
                        b = b.max(theta[dd as usize]);
                    } else {
                        dm = dm.max(theta[dd as usize]);
                    }
                }
            }
        }
        out.t_s.push(t);
        out.peak_skin_c.push(ta + sk.peak.max(0.0));
        out.mean_skin_c.push(ta + sk.mean);
        out.board_max_c.push(ta + b);
        out.display_max_c.push(ta + dm.max(display_max(&pm, theta)));
        out.duty.push(d);
        out.power_mw
            .push(pm.q0.iter().zip(&pm.q1).map(|(a, b)| a + d * b).sum::<f64>() * 1e3);
        out.lumped_shell_c.push(ta + xl[1]);
        out.lumped_board_c.push(ta + xl[0]);
        if out.time_to_limit_s.is_none() && ta + sk.peak >= out.skin_limit_c {
            out.time_to_limit_s = Some(t);
        }
    };
    record(&theta, 0.0, duty_at(0.0), xl, &mut out);
    push_frame(&theta, 0.0, &mut out);
    let series_every = (steps / 400).max(1);
    for s in 1..=steps {
        let t = s as f64 * dt;
        let d = duty_at(t - 0.5 * dt);
        let rhs: Vec<f64> = (0..theta.len())
            .map(|k| pm.cap[k] / dt * theta[k] + pm.q0[k] + d * pm.q1[k])
            .collect();
        let st = fea::pcg(&pm.csr, &diag, &rhs, &mut theta, pc, 1e-7, cfg.max_iter);
        iters += st.iterations;
        xl = net.step_exact(xl, heat_lumped(d), &phi);
        if s % series_every == 0 || s == steps {
            record(&theta, t, d, xl, &mut out);
        }
        if s % frame_every == 0 || s == steps {
            push_frame(&theta, t, &mut out);
        }
    }
    out.iterations_per_step = iters as f64 / steps.max(1) as f64;
    // final-state surfaces (geometry for the animation)
    for (idx, m) in meshes.iter().enumerate() {
        let (name, label) = if idx < pm.geom.cad.len() {
            (pm.geom.cad[idx].0.clone(), PART_NAMES[idx].to_string())
        } else if idx == pm.geom.cad.len() {
            ("pcb".into(), PART_NAMES[4].into())
        } else if idx == pm.geom.cad.len() + 1 {
            ("display".into(), PART_NAMES[5].into())
        } else {
            ("spreader".into(), PART_NAMES[7].into())
        };
        let s = surface_from_mesh(&name, &label, m, &pm, &theta, ta);
        if idx < pm.geom.cad.len() {
            out.surfaces.push(s);
        } else {
            out.inner.push(s);
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------ spreader trade

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct TradeIn {
    pub project: Option<Project>,
    pub config: PodConfig,
    /// Sheet thicknesses for the sweep, µm.
    pub thicknesses_um: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TradeRow {
    pub id: String,
    pub label: String,
    pub spreader: String,
    pub thickness_um: f64,
    pub location: String,
    pub k_in: f64,
    pub peak_skin_c: f64,
    pub mean_skin_c: f64,
    pub hotspot_over_mean_c: f64,
    pub board_max_c: f64,
    pub gain_c: f64,
    pub added_mass_g: f64,
    pub sheet_conductance_mw_k: f64,
    pub note: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SweepLine {
    pub spreader: String,
    pub location: String,
    pub thickness_um: Vec<f64>,
    pub peak_skin_c: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TradeOut {
    pub rows: Vec<TradeRow>,
    pub sweeps: Vec<SweepLine>,
    pub baseline_peak_c: f64,
    pub skin_limit_c: f64,
    pub ambient_c: f64,
    pub voxel_mm: f64,
    pub sheet_area_mm2: f64,
    /// Where the baseline hotspot is: "optics" or "floor" (pod).
    pub hotspot_location: String,
    pub notes: Vec<String>,
}

fn peak_of(p: &Project, cfg: &Resolved, warm: Option<&[f64]>) -> Result<(f64, f64, f64, f64, Vec<f64>), String> {
    let pm = build(p, cfg)?;
    let (theta, _, _, _) = solve_steady(&pm, &pm.q, 3, warm);
    let sk = skin_stats(&pm, &theta);
    let mut b = 0.0f64;
    for (ae, el) in pm.model.elems.iter().enumerate() {
        if el.part == PART_PCB {
            for d in pm.model.elem_dofs(ae) {
                b = b.max(theta[d as usize]);
            }
        }
    }
    let ta = cfg.ambient_c;
    Ok((ta + sk.peak, ta + sk.mean, sk.peak - sk.mean, ta + b, theta))
}

pub fn trade(i: TradeIn) -> Result<TradeOut, String> {
    let p = i.project.unwrap_or_else(Project::iris);
    let base_cfg = i.config.resolve(&p);
    let mut none = base_cfg.clone();
    none.spreader = "none".into();
    let (b_peak, b_mean, b_hot, b_board, warm) = peak_of(&p, &none, None)?;
    let l = crate::cad::frame_geom(&p).layout.clone();
    let area_pod = (l.pod_cavity_len_floor_mm - 1.0) * (l.pod_cavity_height_floor_mm - 0.6);
    let probe = build(&p, &none)?;
    let peak_on_optics = {
        let (t, _, _, _) = solve_steady(&probe, &probe.q, 1, Some(&warm));
        let sk = skin_stats(&probe, &t);
        sk.peak_part == PART_OPTICS || sk.peak_part == PART_FRONT
    };
    let sheet_area = |loc: &str| -> f64 {
        if loc == "optics" {
            let mut c = none.clone();
            c.spreader = "graphite".into();
            c.spreader_location = "optics".into();
            build(&p, &c)
                .ok()
                .and_then(|m| m.spreader_box)
                .map(|b| (b.hi[1] - b.lo[1]) * (b.hi[2] - b.lo[2]))
                .unwrap_or(area_pod)
        } else {
            area_pod
        }
    };
    let area_opt = sheet_area("optics");
    let mut rows = vec![TradeRow {
        id: "none".into(),
        label: "No spreader (baseline)".into(),
        spreader: "none".into(),
        thickness_um: 0.0,
        location: "-".into(),
        k_in: 0.0,
        peak_skin_c: b_peak,
        mean_skin_c: b_mean,
        hotspot_over_mean_c: b_hot,
        board_max_c: b_board,
        gain_c: 0.0,
        added_mass_g: 0.0,
        sheet_conductance_mw_k: 0.0,
        note: "baseline".into(),
    }];
    let options: [(&str, f64, &str, &str); 8] = [
        ("graphite", 25.0, "floor", "Graphite sheet 25 µm on the pod's skin-side wall"),
        ("graphite", 25.0, "lid", "Graphite sheet 25 µm under the pod lid"),
        ("copper", 35.0, "floor", "Copper foil 35 µm on the pod's skin-side wall"),
        ("graphite", 25.0, "optics", "Graphite sheet 25 µm behind the display"),
        ("graphite", 70.0, "optics", "Graphite sheet 70 µm behind the display"),
        ("copper", 70.0, "optics", "Copper foil 70 µm behind the display"),
        ("aluminium", 500.0, "optics", "Aluminium plate 0.5 mm behind the display (bracket)"),
        ("aluminium", 1000.0, "optics", "Aluminium plate 1 mm behind the display (bracket)"),
    ];
    for (kind, t, loc, label) in options {
        let mut c = base_cfg.clone();
        c.spreader = kind.into();
        c.spreader_um = t;
        c.spreader_location = loc.into();
        c.spreader_k_in = None;
        let (pk, mn, hot, bd, _) = peak_of(&p, &c, Some(&warm))?;
        let m = props::spreader(kind).unwrap();
        rows.push(TradeRow {
            id: format!("{kind}_{t:.0}_{loc}"),
            label: label.into(),
            spreader: kind.into(),
            thickness_um: t,
            location: loc.into(),
            k_in: m.k_in,
            peak_skin_c: pk,
            mean_skin_c: mn,
            hotspot_over_mean_c: hot,
            board_max_c: bd,
            gain_c: b_peak - pk,
            added_mass_g: m.rho * (if loc == "optics" { area_opt } else { area_pod }) * 1e-6 * t * 1e-6 * 1e3,
            sheet_conductance_mw_k: m.k_in * t * 1e-6 * 1e3,
            note: m.note.clone(),
        });
    }
    let ts = if i.thicknesses_um.is_empty() {
        vec![10.0, 17.0, 25.0, 40.0, 70.0, 100.0]
    } else {
        i.thicknesses_um.iter().cloned().filter(|t| *t > 0.0).take(12).collect()
    };
    let sweep_loc = if peak_on_optics { "optics" } else { "floor" };
    let mut sweeps = Vec::new();
    for kind in ["graphite", "copper"] {
        let mut ys = Vec::new();
        for &t in &ts {
            let mut c = base_cfg.clone();
            c.spreader = kind.into();
            c.spreader_um = t;
            c.spreader_location = sweep_loc.into();
            c.spreader_k_in = None;
            ys.push(peak_of(&p, &c, Some(&warm))?.0);
        }
        sweeps.push(SweepLine {
            spreader: kind.into(),
            location: sweep_loc.into(),
            thickness_um: ts.clone(),
            peak_skin_c: ys,
        });
    }
    Ok(TradeOut {
        rows,
        sweeps,
        baseline_peak_c: b_peak,
        skin_limit_c: p.design.thermal.skin_limit_c,
        ambient_c: base_cfg.ambient_c,
        voxel_mm: base_cfg.voxel_mm,
        sheet_area_mm2: if peak_on_optics { area_opt } else { area_pod },
        hotspot_location: sweep_loc.into(),
        notes: vec![
            "Conductivities are typical datasheet values, not quotes — check the supplier's datasheet (graphite sheets vary 700–1900 W/m K in-plane).".into(),
            "Sheets thinner than a voxel are modelled by their conductance k·t (rule of mixtures), which is what in-plane spreading depends on.".into(),
            "Copper and aluminium foils also need electrical insulation (a polyimide or PET film) next to the board; not modelled.".into(),
        ],
    })
}
