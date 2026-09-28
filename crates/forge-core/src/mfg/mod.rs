//! # Manufacturing Hub engine — cost, vendors, order guides, fab package
//!
//! * **Cost vs quantity** — a parametric *indicative* model per category:
//!   component price breaks (a learning-curve fit to distributor price
//!   breaks), PCB fabrication and assembly (setup + per-area + per-joint +
//!   per-unique-part loading), the frame by process (home FDM, MJF/SLS
//!   service, urethane casting, injection molding with tooling amortised from
//!   the Molding engine), final assembly labour, test, packaging and yield.
//!   Every assumption is editable. **None of these numbers are quotes.**
//! * **Crossover** — the quantity above which injection molding beats the
//!   cheapest printing process for the frame.
//! * **Vendor directory** and **order guides** (Markdown) for PCB assembly,
//!   3D printing and injection-molding RFQs.
//! * **Fab package** — every file needed to go to manufacturers, produced by
//!   a list of producer functions ([`PRODUCERS`]) so other labs (STL, G-code)
//!   plug in with one line.
//!
//! References: G. Boothroyd, P. Dewhurst, W. Knight, *Product Design for
//! Manufacture and Assembly* (3rd ed., 2010); T. P. Wright, "Factors
//! affecting the cost of airplanes", *J. Aeronautical Sciences* 3 (1936)
//! (learning curves); JLCPCB help pages on BOM/CPL files
//! (<https://jlcpcb.com/help>).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Component, Metric, Project};

pub mod vendors;
use vendors::VENDORS;

// ---------------------------------------------------------------------------
// Assumptions
// ---------------------------------------------------------------------------

/// Every editable cost assumption (USD, minutes, %). Indicative defaults.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Assumptions {
    /// Component price at quantity q: p₁ · (f + (1 − f) · q^(−b)).
    pub bom_floor_fraction: f64,
    pub bom_exponent: f64,
    pub pcb_setup_usd: f64,
    pub pcb_usd_per_cm2: f64,
    pub panel_efficiency: f64,
    pub pcba_setup_usd: f64,
    pub stencil_usd: f64,
    pub usd_per_smd_joint: f64,
    pub usd_per_tht_joint: f64,
    pub usd_per_unique_part: f64,
    pub fdm_filament_usd_kg: f64,
    pub fdm_machine_usd_h: f64,
    pub fdm_post_min: f64,
    /// Effective deposition rate incl. travel/infill, cm³/h (used when the
    /// Print lab's time estimate is not supplied).
    pub fdm_rate_cm3_h: f64,
    pub fdm_waste_pct: f64,
    pub mjf_usd_cm3: f64,
    pub mjf_order_usd: f64,
    pub urethane_master_usd: f64,
    pub urethane_mold_life: f64,
    pub urethane_usd_cm3: f64,
    pub urethane_labour_min: f64,
    pub mold_setup_usd: f64,
    pub test_minutes: f64,
    pub packaging_usd: f64,
    pub yield_pct: f64,
    /// Frame process: auto (cheapest), fdm, mjf, urethane, molding.
    pub frame_process: String,
    /// Overrides from the Print lab (0 = estimate from volume).
    pub print_time_h: f64,
    pub filament_g: f64,
}

impl Default for Assumptions {
    fn default() -> Self {
        Assumptions {
            bom_floor_fraction: 0.35,
            bom_exponent: 0.18,
            pcb_setup_usd: 25.0,
            pcb_usd_per_cm2: 0.02,
            panel_efficiency: 0.75,
            pcba_setup_usd: 50.0,
            stencil_usd: 10.0,
            usd_per_smd_joint: 0.0017,
            usd_per_tht_joint: 0.02,
            usd_per_unique_part: 3.0,
            fdm_filament_usd_kg: 25.0,
            fdm_machine_usd_h: 0.5,
            fdm_post_min: 10.0,
            fdm_rate_cm3_h: 12.0,
            fdm_waste_pct: 15.0,
            mjf_usd_cm3: 0.35,
            mjf_order_usd: 20.0,
            urethane_master_usd: 1500.0,
            urethane_mold_life: 20.0,
            urethane_usd_cm3: 0.25,
            urethane_labour_min: 15.0,
            mold_setup_usd: 300.0,
            test_minutes: 3.0,
            packaging_usd: 2.5,
            yield_pct: 95.0,
            frame_process: "auto".into(),
            print_time_h: 0.0,
            filament_g: 0.0,
        }
    }
}

/// Learning-curve price at quantity q for a unit price p₁ quoted at q = 1.
pub fn price_at(p1: f64, q: f64, a: &Assumptions) -> f64 {
    let f = a.bom_floor_fraction.clamp(0.0, 1.0);
    p1 * (f + (1.0 - f) * q.max(1.0).powf(-a.bom_exponent))
}

// ---------------------------------------------------------------------------
// Cost model
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FrameOption {
    pub process: String,
    pub unit_usd: Vec<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CostCurve {
    pub quantities: Vec<f64>,
    /// Category names in stacking order.
    pub categories: Vec<String>,
    /// per category, per quantity: USD per unit
    pub per_unit: Vec<Vec<f64>>,
    pub total: Vec<f64>,
    /// Frame process chosen at each quantity.
    pub frame_choice: Vec<String>,
    pub frame_options: Vec<FrameOption>,
    pub crossover_qty: Option<f64>,
    pub target_qty: f64,
    pub target_unit_usd: f64,
    pub target_breakdown: Vec<(String, f64)>,
    pub bom_usd_at_target: f64,
    pub assumptions: Assumptions,
    pub frame_volume_cm3: f64,
    pub frame_mass_g: f64,
    pub pcb_area_cm2: f64,
    pub smd_joints: usize,
    pub tht_joints: usize,
    pub unique_parts: usize,
    pub tooling_usd: f64,
    pub notes: Vec<String>,
}

/// Inputs derived once from the project (geometry, board, molding).
pub struct CostInputs {
    pub frame_volume_cm3: f64,
    pub frame_density: f64,
    pub pcb_area_cm2: f64,
    pub smd_joints: usize,
    pub tht_joints: usize,
    pub unique_parts: usize,
    pub molded: Vec<crate::molding::PartProcess>,
}

impl CostInputs {
    pub fn from_project(p: &Project) -> CostInputs {
        let parts = crate::cad::frame_parts(p);
        let vol: f64 = parts.iter().map(|x| crate::molding::volume(&x.mesh).abs() / 1000.0).sum();
        let pcb = &p.design.pcb;
        // joints from the footprint library (no routing needed)
        let (mut smd, mut tht) = (0usize, 0usize);
        let mut uniq: Vec<(String, String)> = Vec::new();
        for c in p.components.iter().filter(|c| c.on_pcb) {
            if let Some(f) = crate::pcb::footprints::footprint(&c.footprint) {
                for pd in &f.pads {
                    if pd.drill > 0.0 {
                        tht += 1;
                    } else {
                        smd += 1;
                    }
                }
            }
            let k = (crate::pcb::fab::bom_value(c), c.footprint.clone());
            if !uniq.contains(&k) {
                uniq.push(k);
            }
        }
        let o = crate::molding::ProcessOptions::from_project(p);
        let molded: Vec<crate::molding::PartProcess> = parts.iter().filter(|x| x.mesh.triangle_count() > 0).map(|x| crate::molding::process_part(x, &o, 300)).collect();
        CostInputs {
            frame_volume_cm3: vol,
            frame_density: p.design.frame.density_g_cm3.max(0.5),
            pcb_area_cm2: pcb.width_mm * pcb.height_mm / 100.0,
            smd_joints: smd,
            tht_joints: tht,
            unique_parts: uniq.len(),
            molded,
        }
    }
}

/// The bare board appears in the BOM as a component; the PCB fabrication
/// model replaces it.
fn is_bare_pcb(c: &Component) -> bool {
    c.refdes.starts_with("PCB")
}

/// Frame cost per unit for each process at quantity q.
fn frame_costs(p: &Project, inp: &CostInputs, a: &Assumptions, q: f64) -> Vec<(String, f64)> {
    let labour = p.design.manufacturing.labor_rate_usd_h;
    let vol = inp.frame_volume_cm3;
    let mass = vol * inp.frame_density;
    let fil_g = if a.filament_g > 0.0 { a.filament_g } else { mass * (1.0 + a.fdm_waste_pct / 100.0) };
    let hours = if a.print_time_h > 0.0 { a.print_time_h } else { vol / a.fdm_rate_cm3_h.max(0.1) + 0.25 };
    let fdm = fil_g / 1000.0 * a.fdm_filament_usd_kg + hours * a.fdm_machine_usd_h + a.fdm_post_min / 60.0 * labour;
    let mjf = vol * a.mjf_usd_cm3 + a.mjf_order_usd / q;
    // master pattern + first silicone mould, then a new mould (≈30 % of the
    // master cost) every `urethane_mold_life` castings, amortised smoothly
    let extra_molds = (q / a.urethane_mold_life.max(1.0) - 1.0).max(0.0);
    let ure = (a.urethane_master_usd + extra_molds * 0.3 * a.urethane_master_usd) / q + vol * a.urethane_usd_cm3 + a.urethane_labour_min / 60.0 * labour;
    let molding = if inp.molded.is_empty() { f64::INFINITY } else { crate::molding::unit_cost_at(&inp.molded, q) + a.mold_setup_usd * inp.molded.len() as f64 / q };
    vec![("FDM (in-house)".into(), fdm), ("MJF/SLS service".into(), mjf), ("Urethane casting".into(), ure), ("Injection molding".into(), molding)]
}

fn choose_frame(opts: &[(String, f64)], a: &Assumptions) -> (String, f64) {
    let key = a.frame_process.to_ascii_lowercase();
    let pick = |name: &str| opts.iter().find(|o| o.0.to_ascii_lowercase().starts_with(name)).cloned();
    match key.as_str() {
        "fdm" => pick("fdm"),
        "mjf" => pick("mjf"),
        "urethane" => pick("urethane"),
        "molding" => pick("injection"),
        _ => None,
    }
    .unwrap_or_else(|| opts.iter().cloned().min_by(|x, y| x.1.partial_cmp(&y.1).unwrap()).unwrap())
}

/// (categories with USD/unit, chosen frame process, every frame option).
pub type Breakdown = (Vec<(String, f64)>, String, Vec<(String, f64)>);

/// Cost of one unit at quantity q, by category.
pub fn unit_cost_breakdown(p: &Project, inp: &CostInputs, a: &Assumptions, q: f64) -> Breakdown {
    let q = q.max(1.0);
    let onboard: f64 = p.components.iter().filter(|c| c.on_pcb).map(|c| price_at(c.unit_cost_usd, q * c.qty, a) * c.qty).sum();
    let offboard: f64 = p.components.iter().filter(|c| !c.on_pcb && !is_bare_pcb(c)).map(|c| price_at(c.unit_cost_usd, q * c.qty, a) * c.qty).sum();
    let pcb = a.pcb_setup_usd / q + inp.pcb_area_cm2 * a.pcb_usd_per_cm2 / a.panel_efficiency.clamp(0.1, 1.0);
    let pcba = (a.pcba_setup_usd + a.stencil_usd + a.usd_per_unique_part * inp.unique_parts as f64) / q + inp.smd_joints as f64 * a.usd_per_smd_joint + inp.tht_joints as f64 * a.usd_per_tht_joint;
    let fopts = frame_costs(p, inp, a, q);
    let (fname, fcost) = choose_frame(&fopts, a);
    let m = &p.design.manufacturing;
    let assembly = (m.assembly_minutes + a.test_minutes) / 60.0 * m.labor_rate_usd_h;
    let pack = a.packaging_usd;
    let sub = onboard + offboard + pcb + pcba + fcost + assembly + pack;
    let y = (a.yield_pct / 100.0).clamp(0.5, 1.0);
    let yield_loss = sub / y - sub;
    let cats = vec![
        ("Electronics (on board)".to_string(), onboard),
        ("Display, battery, optics & hardware".to_string(), offboard),
        ("PCB fabrication".to_string(), pcb),
        ("PCB assembly".to_string(), pcba),
        (format!("Frame — {fname}"), fcost),
        ("Final assembly & test".to_string(), assembly),
        ("Packaging".to_string(), pack),
        ("Yield loss".to_string(), yield_loss),
    ];
    (cats, fname, fopts)
}

/// Molding-vs-printing crossover by bisection on log q.
pub fn crossover(p: &Project, inp: &CostInputs, a: &Assumptions) -> Option<f64> {
    let diff = |q: f64| {
        let o = frame_costs(p, inp, a, q);
        let print = o.iter().filter(|x| x.0.starts_with("FDM") || x.0.starts_with("MJF")).map(|x| x.1).fold(f64::INFINITY, f64::min);
        let mold = o.iter().find(|x| x.0.starts_with("Injection")).map(|x| x.1).unwrap_or(f64::INFINITY);
        mold - print
    };
    let (mut lo, mut hi) = (0.0f64, 7.0f64); // log10 q
    if !diff(10f64.powf(hi)).is_finite() || diff(10f64.powf(hi)) > 0.0 {
        return None;
    }
    if diff(1.0) <= 0.0 {
        return Some(1.0);
    }
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if diff(10f64.powf(mid)) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(10f64.powf(hi).ceil())
}

pub fn cost_curve(p: &Project, a: &Assumptions) -> CostCurve {
    let inp = CostInputs::from_project(p);
    cost_curve_with(p, &inp, a)
}

pub fn cost_curve_with(p: &Project, inp: &CostInputs, a: &Assumptions) -> CostCurve {
    let mut qs = p.design.manufacturing.quantities.clone();
    if qs.is_empty() {
        qs = vec![1.0, 10.0, 100.0, 1000.0, 10000.0];
    }
    // a smooth curve for the chart: log-spaced points plus the project's quantities
    let mut grid: Vec<f64> = (0..=40).map(|k| 10f64.powf(k as f64 * 5.0 / 40.0).round()).collect();
    grid.extend(qs.iter().copied());
    grid.push(p.design.manufacturing.target_quantity.max(1.0));
    grid.sort_by(|x, y| x.partial_cmp(y).unwrap());
    grid.dedup_by(|x, y| (*x - *y).abs() < 0.5);
    let mut c = CostCurve { quantities: grid.clone(), assumptions: a.clone(), ..Default::default() };
    let mut fopts: Vec<FrameOption> = Vec::new();
    for (k, &q) in grid.iter().enumerate() {
        let (cats, fname, fo) = unit_cost_breakdown(p, inp, a, q);
        if k == 0 {
            c.categories = cats.iter().map(|x| if x.0.starts_with("Frame") { "Frame".to_string() } else { x.0.clone() }).collect();
            c.per_unit = vec![Vec::new(); cats.len()];
            fopts = fo.iter().map(|x| FrameOption { process: x.0.clone(), unit_usd: Vec::new() }).collect();
        }
        for (i, x) in cats.iter().enumerate() {
            c.per_unit[i].push(x.1);
        }
        for (i, x) in fo.iter().enumerate() {
            fopts[i].unit_usd.push(if x.1.is_finite() { x.1 } else { -1.0 });
        }
        c.total.push(cats.iter().map(|x| x.1).sum());
        c.frame_choice.push(fname);
    }
    c.frame_options = fopts;
    let tq = p.design.manufacturing.target_quantity.max(1.0);
    let (cats, _, _) = unit_cost_breakdown(p, inp, a, tq);
    c.target_qty = tq;
    c.target_unit_usd = cats.iter().map(|x| x.1).sum();
    c.bom_usd_at_target = cats[0].1 + cats[1].1;
    c.target_breakdown = cats;
    c.crossover_qty = crossover(p, inp, a);
    c.frame_volume_cm3 = inp.frame_volume_cm3;
    c.frame_mass_g = inp.frame_volume_cm3 * inp.frame_density;
    c.pcb_area_cm2 = inp.pcb_area_cm2;
    c.smd_joints = inp.smd_joints;
    c.tht_joints = inp.tht_joints;
    c.unique_parts = inp.unique_parts;
    c.tooling_usd = inp.molded.iter().map(|x| x.tool_cost_usd).sum();
    c.notes = vec![
        "Indicative parametric estimates — get real quotes before committing. Distributor price breaks, fab pricing and labour rates change constantly.".into(),
        "Component prices use a learning-curve fit p(q) = p₁·(f + (1−f)·q^−b) to typical distributor price breaks; p₁ is the qty-1 price in the project BOM.".into(),
        "The bare-PCB line of the BOM is replaced by the PCB fabrication model; the frame is costed per process and the cheapest is chosen unless you force one.".into(),
    ];
    c
}

// ---------------------------------------------------------------------------
// Order guides
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Guide {
    pub id: String,
    pub title: String,
    pub markdown: String,
}

pub fn order_guides(p: &Project) -> Vec<Guide> {
    let pcb = &p.design.pcb;
    let name = crate::pcb::fab::base_name(p);
    let onboard = p.components.iter().filter(|c| c.on_pcb).count();
    let mat = crate::molding::materials::material(&p.design.manufacturing.molding_material);
    let parts: Vec<String> = crate::cad::frame_parts(p).iter().map(|x| x.name.clone()).collect();
    let mut g = Vec::new();
    g.push(Guide {
        id: "pcba".into(),
        title: "Order assembled PCBs (JLCPCB-style PCBA)".into(),
        markdown: format!(
            "# Ordering the {name} board with assembly\n\n\
Board: {w:.1} × {h:.1} mm, {l} layers, {t:.2} mm FR-4, {n} parts on the top side.\n\n\
1. **Download the fabrication package** from the Manufacturing Hub and unzip it.\n\
2. **Zip the `gerbers/` folder** (all `.gbr` and `.drl` files) — fab houses want one zip with Gerbers + drill.\n\
3. **Upload the zip** on the vendor's PCB quote page. Check the auto-detected size ({w:.1} × {h:.1} mm) and layer count ({l}). Choose thickness {t:.2} mm, 1 oz copper; prefer **ENIG** finish (0.4 mm-pitch QFN and LGA parts).\n\
4. **Inspect every layer in the online Gerber viewer** (or gerbv / tracespace first): outline, drills, mask openings, GND pours on both copper layers.\n\
5. **Enable assembly (PCBA)**, top side, and choose the service tier that supports fine-pitch parts.\n\
6. **Upload `assembly/{name}-BOM.csv` and `assembly/{name}-CPL.csv`.** The BOM uses the columns `Comment,Designator,Footprint,LCSC Part #`; the CPL `Designator,Mid X,Mid Y,Layer,Rotation`.\n\
7. **Fill in the part numbers.** The `LCSC Part #` column is empty on purpose: search the vendor's parts library for each line (value, package, voltage rating, tolerance) and paste the part code. Check stock for your quantity.\n\
8. **Verify placements and rotations in the vendor's 3-D/2-D preview.** CAD libraries and assembly houses define 0° differently; fix rotations there (pin 1 dots are on the silkscreen and in the assembly drawing).\n\
9. **Review the DFM report** the vendor generates and compare with `{name}-FAB-README.md` (min trace/space {tr:.2}/{cl:.2} mm, via {vd:.2}/{vp:.2} mm).\n\
10. **Order a small batch first** (5–10 boards), bring one up on the bench, then scale.\n\n\
Parts that are not on the PCB (display module, battery, optics, lenses, hinges) are bought separately — see the cost report.\n",
            w = pcb.width_mm, h = pcb.height_mm, l = pcb.layers, t = pcb.thickness_mm, n = onboard, tr = pcb.min_trace_mm, cl = pcb.min_clearance_mm, vd = pcb.via_drill_mm, vp = pcb.via_pad_mm
        ),
    });
    g.push(Guide {
        id: "print".into(),
        title: "3-D print the frame (in-house or service)".into(),
        markdown: format!(
            "# 3-D printing the {} frame\n\n\
Parts: {}.\n\n\
**In-house FDM** — slice the STL files from the CAD Studio in the Print lab (or PrusaSlicer / Cura / Bambu Studio). PETG prints well for prototypes; orient the front face-down, use a brim, and print temples flat.\n\n\
**Service (MJF / SLS / SLA)** — for EVT/DVT-quality parts:\n\n\
1. Export STL (or better STEP) for every part.\n\
2. Upload to a print service or marketplace; pick material (PA12 for MJF/SLS is a common choice for eyewear prototypes; tough resin for SLA).\n\
3. Check minimum wall thickness warnings (the frame walls are {:.1} mm nominal).\n\
4. Order one set, check fit of the PCB, battery and optics, then order more.\n",
            p.meta.name,
            if parts.is_empty() { "(none yet)".to_string() } else { parts.join(", ") },
            p.design.frame.wall_mm
        ),
    });
    g.push(Guide {
        id: "molding".into(),
        title: "Request injection-molding quotes (RFQ)".into(),
        markdown: format!(
            "# Injection-molding RFQ for the {} frame\n\n\
Molders quote from **STEP files and 2-D drawings**, not STL. Export STEP from the CAD Studio geometry via Onshape or FreeCAD, then send:\n\n\
1. **STEP file per part** ({}), in mm, with the parting direction noted.\n\
2. **2-D drawing (PDF)** per part: critical dimensions and tolerances (lens-groove width, hinge boss, PCB-pod inner length), datums, surface finish (e.g. SPI A-2 on the visible front, textured elsewhere), gate/ejector-mark restrictions on cosmetic faces.\n\
3. **Material and colour**: {} ({}) — or ask the molder to propose an equivalent grade; specify colour (Pantone) and UV requirements.\n\
4. **Annual volume and batch size** (e.g. target {} units), and whether you want an aluminium bridge tool first or a steel production tool ({} planned).\n\
5. **Cavities** ({}), family-mold acceptability, and whether side actions are acceptable (the Molding Lab lists undercuts).\n\
6. **Secondary operations**: inserts (hinge screws), pad printing, assembly.\n\
7. Ask for a **DFM report** and **T1 samples** with a dimensional report before approving the tool.\n\n\
Use the Molding Lab's DFM report (draft, wall thickness, clamp force, cycle time) to sanity-check the quotes — tooling and piece-price estimates there are indicative only.\n",
            p.meta.name,
            if parts.is_empty() { "frame parts".to_string() } else { parts.join(", ") },
            mat.name,
            mat.id,
            p.design.manufacturing.target_quantity,
            p.design.manufacturing.mold_steel,
            p.design.manufacturing.cavities
        ),
    });
    g
}

// ---------------------------------------------------------------------------
// Stage checklists (prototype → EVT → DVT → PVT → MP)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stage {
    pub id: String,
    pub title: String,
    pub goal: String,
    pub checklist: Vec<String>,
    /// Project milestones mapped to this stage.
    pub milestones: Vec<String>,
}

/// (id, title, goal, checklist, milestone keywords)
type StageDef = (&'static str, &'static str, &'static str, &'static [&'static str], &'static [&'static str]);

pub fn stages(p: &Project) -> Vec<Stage> {
    let defs: [StageDef; 5] = [
        ("proto", "Prototype", "Prove the concept works at all", &["Breadboard / dev-kit electronics", "3-D printed frame", "Requirements with metrics (Systems Studio)"], &["concept", "requirements", "design"]),
        ("evt", "EVT — engineering validation", "Does the design meet its requirements?", &["Order 5–10 assembled boards (PCBA)", "Printed or MJF frames", "Bring-up, firmware, power and thermal measurements", "Fix schematic / layout issues"], &["prototype", "evt"]),
        ("dvt", "DVT — design validation", "Does the product survive the real world?", &["Soft (aluminium) tools, parts in the final material", "Drop, fold, sweat, thermal and battery-safety tests", "Pre-compliance EMC / radio scan", "Freeze the design"], &["validation", "dvt"]),
        ("pvt", "PVT — production validation", "Can the factory build it repeatably?", &["Hardened production tools (T1 → T2 samples)", "Line fixtures, test jigs, work instructions", "First pilot run (e.g. 100 units), yield tracking", "Certifications (FCC / CE / Bluetooth listing)"], &["production", "pvt"]),
        ("mp", "Mass production", "Ship at volume", &["Ramp plan and supplier contracts", "Incoming inspection and SPC", "Field-return analysis loop"], &["mp", "mass"]),
    ];
    defs.iter()
        .map(|(id, title, goal, list, keys)| Stage {
            id: id.to_string(),
            title: title.to_string(),
            goal: goal.to_string(),
            checklist: list.iter().map(|s| s.to_string()).collect(),
            milestones: p
                .milestones
                .iter()
                .filter(|m| {
                    let s = format!("{} {}", m.stage, m.title).to_ascii_lowercase();
                    keys.iter().any(|k| s.contains(k))
                })
                .map(|m| format!("{} — {}", m.id, m.title))
                .collect(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Fab package
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageFile {
    pub path: String,
    pub content: String,
    /// "utf8" or "base64"
    pub encoding: String,
}

fn utf8(path: impl Into<String>, content: impl Into<String>) -> PackageFile {
    PackageFile { path: path.into(), content: content.into(), encoding: "utf8".into() }
}

/// A producer adds files to the package. Add a line to [`PRODUCERS`] to
/// include new outputs (e.g. STL from `cad`, G-code from `print`).
pub type Producer = fn(&Project) -> Vec<PackageFile>;

pub const PRODUCERS: &[(&str, Producer)] = &[
    ("pcb", produce_pcb),
    ("molding", produce_molding),
    ("cost", produce_cost),
    ("guides", produce_guides),
    ("verification", produce_verification),
    ("project", produce_project),
];

fn produce_pcb(p: &Project) -> Vec<PackageFile> {
    if !p.components.iter().any(|c| c.on_pcb) {
        return vec![];
    }
    let l = crate::pcb::layout(p);
    crate::pcb::fab::fab_files(p, &l).into_iter().map(|f| utf8(format!("pcb/{}", f.path), f.content)).collect()
}

pub fn molding_report(p: &Project) -> String {
    let o = crate::molding::ProcessOptions::from_project(p);
    let a = crate::molding::analyze(p, &o, 1500);
    let mut s = format!("# {} — injection-molding DFM report\n\n", p.meta.name);
    s += &format!("Material **{}** ({}), tool steel **{}**, {} cavity/cavities. Values are estimates from the Molding Lab — indicative, not a molder's quote.\n\n", a.material.name, a.material.id, a.steel.name, o.cavities);
    s += "| Part | Pull | Min draft | Undercut | Wall nom / max | Projected area | Clamp | Cycle | Tool (est.) | Piece (est.) |\n|---|---|---|---|---|---|---|---|---|---|\n";
    for x in &a.parts {
        s += &format!(
            "| {} | {} | {:.2}° | {:.0} mm² | {:.2} / {:.2} mm | {:.0} mm² | {:.1} t | {:.1} s | ${:.0} | ${:.2} |\n",
            x.part, x.pull_label, x.min_draft_deg, x.undercut_area_mm2, x.wall_nominal_mm, x.wall_max_mm, x.projected_area_mm2, x.clamp_tonnes, x.cycle.total_s, x.tool_cost_usd, x.piece.total_usd
        );
    }
    s += &format!("\nShrinkage compensation: scale cavities by **{:.5}** ({} % linear shrink).\n\n", 1.0 / (1.0 - a.material.shrinkage_pct / 100.0), a.material.shrinkage_pct);
    s += "## Warnings\n\n";
    let mut any = false;
    for x in &a.parts {
        for w in &x.warnings {
            s += &format!("* **{}**: {}\n", x.part, w);
            any = true;
        }
    }
    if !any {
        s += "* none\n";
    }
    s += "\n## Design rules applied\n\n* Draft ≥ 1° on side walls (more on textured faces).\n* Uniform walls; thick sections cored out (sink marks, long cooling).\n* Ribs ≤ 60 % of the wall they stand on; generous radii at corners.\n* Gate at the thickest section so material flows thick → thin.\n";
    s
}

fn produce_molding(p: &Project) -> Vec<PackageFile> {
    vec![utf8("molding/molding-dfm-report.md", molding_report(p))]
}

pub fn cost_csv(c: &CostCurve) -> String {
    let mut s = String::from("Quantity");
    for k in &c.categories {
        s += &format!(",{}", k.replace(',', ";"));
    }
    s += ",Total USD/unit,Frame process\n";
    for (i, q) in c.quantities.iter().enumerate() {
        s += &format!("{}", q);
        for cat in &c.per_unit {
            s += &format!(",{:.4}", cat[i]);
        }
        s += &format!(",{:.4},{}\n", c.total[i], c.frame_choice[i]);
    }
    s
}

pub fn cost_markdown(p: &Project, c: &CostCurve) -> String {
    let mut s = format!("# {} — cost estimate (indicative — get real quotes)\n\n", p.meta.name);
    s += &format!("Target quantity **{}**: **${:.2} per unit**.\n\n| Category | USD / unit |\n|---|---|\n", c.target_qty, c.target_unit_usd);
    for (k, v) in &c.target_breakdown {
        s += &format!("| {} | {:.2} |\n", k, v);
    }
    s += &format!(
        "\nMolding beats printing for the frame above **{}** units. Tooling (all frame parts) ≈ ${:.0}.\n\n## Assumptions\n\n```json\n{}\n```\n\n",
        c.crossover_qty.map(|q| format!("{q:.0}")).unwrap_or_else(|| "— (never within 10⁷)".into()),
        c.tooling_usd,
        serde_json::to_string_pretty(&c.assumptions).unwrap_or_default()
    );
    for n in &c.notes {
        s += &format!("* {n}\n");
    }
    s
}

fn produce_cost(p: &Project) -> Vec<PackageFile> {
    let c = cost_curve(p, &Assumptions::default());
    vec![utf8("cost/cost-vs-quantity.csv", cost_csv(&c)), utf8("cost/cost-report.md", cost_markdown(p, &c))]
}

fn produce_guides(p: &Project) -> Vec<PackageFile> {
    order_guides(p).into_iter().map(|g| utf8(format!("guides/order-{}.md", g.id), g.markdown)).collect()
}

fn produce_verification(p: &Project) -> Vec<PackageFile> {
    let ev = crate::systems::evaluate(p);
    vec![utf8("verification-report.md", crate::systems::report_markdown(p, &ev))]
}

fn produce_project(p: &Project) -> Vec<PackageFile> {
    vec![utf8("project.json", serde_json::to_string_pretty(p).unwrap_or_default())]
}

/// Every file for the manufacturers, plus an index README.
pub fn package(p: &Project) -> Vec<PackageFile> {
    let mut files: Vec<PackageFile> = Vec::new();
    for (_, prod) in PRODUCERS {
        files.extend(prod(p));
    }
    let mut idx = format!("# {} — fabrication package\n\nGenerated by Protoforge. Contents:\n\n", p.meta.name);
    for f in &files {
        idx += &format!("* `{}`\n", f.path);
    }
    idx += "\nStart with `guides/` for step-by-step ordering instructions. All costs are indicative estimates — get real quotes.\n";
    files.insert(0, utf8("README.md", idx));
    files
}

// ---------------------------------------------------------------------------
// API
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CostIn {
    project: Project,
    #[serde(default)]
    assumptions: Option<Assumptions>,
}

/// JSON API entry point for `mfg.*` methods.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "mfg.cost" => handle(input, |i: CostIn| Ok(cost_curve(&i.project, &i.assumptions.unwrap_or_default()))),
        "mfg.assumptions" => Some(crate::api::to_value(Assumptions::default())),
        "mfg.vendors" => Some(crate::api::to_value(VENDORS)),
        "mfg.guides" => handle(input, |p: Project| Ok(order_guides(&p))),
        "mfg.stages" => handle(input, |p: Project| Ok(stages(&p))),
        "mfg.package" => handle(input, |p: Project| Ok(package(&p))),
        "mfg.molding_report" => handle(input, |p: Project| Ok(molding_report(&p))),
        "mfg.producers" => Some(crate::api::to_value(PRODUCERS.iter().map(|x| x.0).collect::<Vec<_>>())),
        _ => None,
    }
}

/// Metrics this module contributes to requirement verification.
pub fn metrics(p: &Project) -> Vec<Metric> {
    let c = cost_curve(p, &Assumptions::default());
    let fq = c.quantities.iter().position(|q| (*q - c.target_qty).abs() < 0.5).map(|i| c.frame_choice[i].clone()).unwrap_or_default();
    let mut v = vec![
        Metric::new("mfg.unit_cost_usd", "Unit cost at target quantity", c.target_unit_usd, "USD", "mfg").with_note(&format!("indicative estimate at {} units; frame by {}", c.target_qty, fq)),
        Metric::new("mfg.bom_cost_usd", "Parts cost at target quantity", c.bom_usd_at_target, "USD", "mfg").with_note("learning-curve price breaks applied to the qty-1 BOM prices"),
    ];
    if let Some(q) = c.crossover_qty {
        v.push(Metric::new("mfg.crossover_qty", "Molding beats printing above", q, "units", "mfg"));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_breaks_decrease() {
        let a = Assumptions::default();
        let mut last = f64::INFINITY;
        for q in [1.0, 10.0, 100.0, 1000.0, 10000.0] {
            let p = price_at(1.0, q, &a);
            assert!(p < last + 1e-12);
            last = p;
        }
        assert!((price_at(1.0, 1.0, &a) - 1.0).abs() < 1e-12);
        assert!(price_at(1.0, 1e9, &a) > a.bom_floor_fraction - 1e-6);
    }

    #[test]
    fn unit_cost_monotonic_and_target_ok() {
        let p = Project::iris();
        let c = cost_curve(&p, &Assumptions::default());
        for w in c.total.windows(2) {
            assert!(w[1] <= w[0] + 1e-9, "unit cost must not increase with quantity: {:?}", c.total);
        }
        assert!(c.target_unit_usd > 10.0 && c.target_unit_usd < 1000.0);
    }

    #[test]
    fn crossover_exists_for_iris() {
        let p = Project::iris();
        let c = cost_curve(&p, &Assumptions::default());
        let q = c.crossover_qty.expect("molding should win at some quantity");
        assert!(q > 10.0 && q < 1e6, "{q}");
    }

    #[test]
    fn package_contains_expected_files() {
        let p = Project::iris();
        let files = package(&p);
        let has = |s: &str| files.iter().any(|f| f.path.contains(s));
        for s in ["README.md", "F_Cu.gbr", "B_Cu.gbr", "Edge_Cuts.gbr", "PTH.drl", "BOM.csv", "CPL.csv", "molding-dfm-report.md", "cost-vs-quantity.csv", "cost-report.md", "order-pcba.md", "order-molding.md", "verification-report.md", "project.json"] {
            assert!(has(s), "missing {s}");
        }
    }

    #[test]
    fn vendors_are_facts_only() {
        for v in VENDORS {
            assert!(v.url.starts_with("https://"));
            assert!(!v.categories.is_empty() && !v.consumes.is_empty());
        }
    }
}

#[cfg(test)]
mod timing {
    use super::*;
    /// `cargo test -p forge-core metric_timing -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn metric_timing() {
        let p = Project::iris();
        for round in 0..2 {
            let t = std::time::Instant::now();
            let a = crate::pcb::metrics(&p);
            let t1 = t.elapsed();
            let t = std::time::Instant::now();
            let b = crate::molding::metrics(&p);
            let t2 = t.elapsed();
            let t = std::time::Instant::now();
            let c = metrics(&p);
            let t3 = t.elapsed();
            eprintln!("round {round}: pcb {t1:?} molding {t2:?} mfg {t3:?}");
            if round == 0 {
                for m in a.iter().chain(b.iter()).chain(c.iter()) {
                    eprintln!("  {} = {:.3} {} ({})", m.key, m.value, m.unit, m.note);
                }
            }
        }
    }
}
