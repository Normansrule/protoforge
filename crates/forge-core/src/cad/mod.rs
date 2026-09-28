//! # CAD engine
//!
//! A small, dependency-free mesh modeller that generates the Iris smart-
//! glasses frame (and a generic electronics enclosure) from parameters.
//!
//! | file | what |
//! |---|---|
//! | [`geom2d`] | polygons, superellipses, offsets, ear-clipping triangulation |
//! | [`clip`] | region booleans and offsets by the winding-number rule |
//! | [`solid`] | lofts, extrusion with draft, sweeps, boxes, hollow shells |
//! | [`mesh`] | the shared [`Mesh`]/[`Part`] types, validator, mass properties |
//! | [`frame`] | the parametric Iris frame (boxing system, pods, hinges) |
//! | [`enclosure`] | generic box + lid with bosses, standoffs, cut-outs |
//! | [`fit`] | face-width / PD → frame size recommendation |
//! | [`export`] | STL (binary/ASCII), OBJ, 3MF |
//!
//! Real CAD kernels (Parasolid in Onshape/SolidWorks, Open CASCADE in
//! FreeCAD/CadQuery/build123d) keep an exact **B-rep** (faces bounded by
//! edges on analytic surfaces); this engine works directly on triangle
//! meshes, which is what slicers, the molding analysis and the browser
//! renderer consume anyway.
//!
//! The public signatures [`Mesh`], [`Part`] and [`frame_parts`] are used by
//! `print` and `molding` and are kept stable.

use std::cell::RefCell;
use std::rc::Rc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Metric, Project};

pub mod clip;
pub mod enclosure;
pub mod export;
pub mod fit;
pub mod frame;
pub mod geom2d;
pub mod mesh;
pub mod solid;

pub use mesh::{Mesh, Part};
use mesh::{MassProps, MeshCheck, MeshF32};

thread_local! {
    static FRAME_CACHE: RefCell<Option<(String, Rc<frame::FrameGeom>)>> = const { RefCell::new(None) };
}

/// Generate (or fetch from a one-entry cache) the frame geometry. The key is
/// every design input the generator reads.
pub fn frame_geom(p: &Project) -> Rc<frame::FrameGeom> {
    let key = serde_json::to_string(&(&p.design.frame, &p.design.optics, &p.design.pcb)).unwrap_or_default();
    if let Some(g) = FRAME_CACHE.with(|c| c.borrow().as_ref().filter(|(k, _)| *k == key).map(|(_, g)| g.clone())) {
        return g;
    }
    let g = Rc::new(frame::build(p));
    FRAME_CACHE.with(|c| *c.borrow_mut() = Some((key, g.clone())));
    g
}

/// All printable/moldable parts of the product, positioned in assembly
/// coordinates (mm, Z up; X to the wearer's right, Y towards the ears).
/// `explode` gives the direction/offset for an exploded view.
pub fn frame_parts(p: &Project) -> Vec<Part> {
    frame_geom(p)
        .parts
        .iter()
        .map(|b| Part { name: b.name.clone(), mesh: b.assembly.clone(), color: b.color.clone(), explode: b.explode })
        .collect()
}

/// The same parts in their print / mold orientation: lying flat on the bed,
/// centred on X/Y, min Z = 0, mold pull direction = +Z (every side wall is
/// drafted by `frame.draft_deg` relative to +Z).
pub fn frame_parts_print(p: &Project) -> Vec<Part> {
    frame_geom(p)
        .parts
        .iter()
        .map(|b| Part { name: b.name.clone(), mesh: b.print.clone(), color: b.color.clone(), explode: [0.0; 3] })
        .collect()
}

/// Mold pull direction of each frame part in **assembly** coordinates
/// (the print frame's +Z carried through the part's placement).
pub fn frame_pull_directions(p: &Project) -> Vec<(String, [f64; 3])> {
    frame_geom(p).parts.iter().map(|b| (b.name.clone(), b.pull)).collect()
}

// ------------------------------------------------------------------ API types

#[derive(Deserialize)]
struct ProjectIn {
    project: Project,
}

#[derive(Deserialize)]
#[serde(default)]
struct PartsIn {
    project: Option<Project>,
    orientation: String,
    display: bool,
}

impl Default for PartsIn {
    fn default() -> Self {
        PartsIn { project: None, orientation: "assembly".into(), display: true }
    }
}

#[derive(Serialize)]
struct PartOut {
    name: String,
    label: String,
    color: String,
    explode: [f64; 3],
    group: String,
    mesh: MeshF32,
    mass: MassProps,
    check: MeshCheck,
    shells: usize,
}

#[derive(Serialize)]
struct PartsOut {
    orientation: String,
    parts: Vec<PartOut>,
    display: Vec<frame::DisplayItem>,
    layout: frame::FrameLayout,
    total: MassProps,
    frame_mass_g: f64,
    components_mass_g: f64,
    wearable_mass_g: f64,
    material: String,
    density_g_cm3: f64,
    warnings: Vec<String>,
}

fn total_props(parts: &[MassProps]) -> MassProps {
    let mut t = MassProps::default();
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    let mut c = [0.0; 3];
    for m in parts {
        t.volume_mm3 += m.volume_mm3;
        t.area_mm2 += m.area_mm2;
        t.mass_g += m.mass_g;
        t.triangles += m.triangles;
        for k in 0..3 {
            c[k] += m.centroid[k] * m.volume_mm3;
            lo[k] = lo[k].min(m.bbox_min[k]);
            hi[k] = hi[k].max(m.bbox_max[k]);
        }
    }
    t.volume_cm3 = t.volume_mm3 / 1000.0;
    if t.volume_mm3 > 0.0 {
        t.centroid = [c[0] / t.volume_mm3, c[1] / t.volume_mm3, c[2] / t.volume_mm3];
    }
    t.bbox_min = lo;
    t.bbox_max = hi;
    t.size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    t
}

fn parts_out(p: &Project, orientation: &str, display: bool) -> PartsOut {
    let geom = frame_geom(p);
    let density = p.design.frame.density_g_cm3;
    let print = orientation == "print";
    let parts: Vec<PartOut> = geom
        .parts
        .iter()
        .map(|b| {
            let m = if print { &b.print } else { &b.assembly };
            PartOut {
                name: b.name.clone(),
                label: b.label.clone(),
                color: b.color.clone(),
                explode: b.explode,
                group: b.group.into(),
                mesh: m.to_f32(),
                mass: m.mass_props(density),
                check: m.validate(),
                shells: b.shells,
            }
        })
        .collect();
    let props: Vec<MassProps> = parts.iter().map(|x| x.mass.clone()).collect();
    let total = total_props(&props);
    let comp = p.component_mass_g();
    PartsOut {
        orientation: if print { "print".into() } else { "assembly".into() },
        frame_mass_g: total.mass_g,
        components_mass_g: comp,
        wearable_mass_g: total.mass_g + comp,
        total,
        parts,
        display: if display && !print { geom.display.clone() } else { vec![] },
        layout: geom.layout.clone(),
        material: p.design.frame.material.clone(),
        density_g_cm3: density,
        warnings: geom.warnings.clone(),
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct StlIn {
    project: Option<Project>,
    part: String,
    binary: bool,
    orientation: String,
    /// Optional enclosure parameters: exports an enclosure part instead.
    enclosure: Option<enclosure::EnclosureParams>,
}

impl Default for StlIn {
    fn default() -> Self {
        StlIn { project: None, part: "front".into(), binary: true, orientation: "print".into(), enclosure: None }
    }
}

#[derive(Serialize)]
struct FileOut {
    filename: String,
    /// base64 for binary payloads
    base64: Option<String>,
    text: Option<String>,
    bytes: usize,
    triangles: usize,
}

fn named_mesh(i: &StlIn) -> Result<(String, Mesh), String> {
    if let Some(e) = &i.enclosure {
        let b = enclosure::build(e);
        let print = i.orientation != "assembly";
        let m = match (i.part.as_str(), print) {
            ("lid", true) => b.lid_print,
            ("lid", false) => b.lid,
            (_, true) => b.body_print,
            (_, false) => b.body,
        };
        let name = if i.part == "lid" { "enclosure_lid" } else { "enclosure_body" };
        return Ok((name.into(), m));
    }
    let p = i.project.clone().unwrap_or_else(Project::iris);
    let geom = frame_geom(&p);
    let b = geom.parts.iter().find(|b| b.name == i.part).ok_or_else(|| format!("unknown part `{}`", i.part))?;
    let m = if i.orientation == "assembly" { b.assembly.clone() } else { b.print.clone() };
    Ok((b.name.clone(), m))
}

fn stl(i: StlIn) -> Result<FileOut, String> {
    let (name, m) = named_mesh(&i)?;
    let tris = m.triangle_count();
    if i.binary {
        let bytes = export::stl_binary(&m, &name);
        Ok(FileOut { filename: format!("iris_{name}.stl"), bytes: bytes.len(), base64: Some(export::base64(&bytes)), text: None, triangles: tris })
    } else {
        let text = export::stl_ascii(&m, &name);
        Ok(FileOut { filename: format!("iris_{name}_ascii.stl"), bytes: text.len(), base64: None, text: Some(text), triangles: tris })
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct ObjIn {
    project: Option<Project>,
    orientation: String,
}

impl Default for ObjIn {
    fn default() -> Self {
        ObjIn { project: None, orientation: "assembly".into() }
    }
}

fn obj(i: ObjIn) -> Result<FileOut, String> {
    let p = i.project.unwrap_or_else(Project::iris);
    let geom = frame_geom(&p);
    let meshes: Vec<(&str, &Mesh)> = geom
        .parts
        .iter()
        .map(|b| (b.name.as_str(), if i.orientation == "print" { &b.print } else { &b.assembly }))
        .collect();
    let text = export::obj(&meshes);
    let tris = meshes.iter().map(|(_, m)| m.triangle_count()).sum();
    Ok(FileOut { filename: "iris_frame.obj".into(), bytes: text.len(), base64: None, text: Some(text), triangles: tris })
}

/// Lay print-oriented parts out on a bed with simple shelf packing.
pub fn arrange_on_bed(meshes: &[Mesh], bed: [f64; 2], gap: f64) -> Vec<Mesh> {
    let mut order: Vec<usize> = (0..meshes.len()).collect();
    let dims: Vec<[f64; 2]> = meshes
        .iter()
        .map(|m| {
            let (lo, hi) = m.bbox();
            [hi[0] - lo[0], hi[1] - lo[1]]
        })
        .collect();
    order.sort_by(|&a, &b| dims[b][1].partial_cmp(&dims[a][1]).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = vec![Mesh::default(); meshes.len()];
    let (mut x, mut y, mut row_h) = (gap, gap, 0.0f64);
    for i in order {
        let [w, h] = dims[i];
        if x + w > bed[0] - gap && x > gap {
            x = gap;
            y += row_h + gap;
            row_h = 0.0;
        }
        let (lo, _) = meshes[i].bbox();
        out[i] = meshes[i].translated(x - lo[0], y - lo[1], -lo[2]);
        x += w + gap;
        row_h = row_h.max(h);
    }
    out
}

#[derive(Serialize)]
struct ThreeMfOut {
    files: Vec<(String, String)>,
    filename: String,
}

fn three_mf(i: ProjectIn) -> Result<ThreeMfOut, String> {
    let geom = frame_geom(&i.project);
    let meshes: Vec<Mesh> = geom.parts.iter().map(|b| b.print.clone()).collect();
    let placed = arrange_on_bed(&meshes, [256.0, 256.0], 6.0);
    let items: Vec<(&str, &Mesh, &str)> = geom.parts.iter().zip(&placed).map(|(b, m)| (b.name.as_str(), m, b.color.as_str())).collect();
    Ok(ThreeMfOut { files: export::three_mf(&items, &format!("{} frame", i.project.meta.name)), filename: "iris_frame.3mf".into() })
}

#[derive(Serialize)]
struct MassRow {
    name: String,
    label: String,
    mass: MassProps,
}

#[derive(Serialize)]
struct MassOut {
    parts: Vec<MassRow>,
    total: MassProps,
    frame_mass_g: f64,
    components: Vec<(String, String, f64, String)>,
    components_mass_g: f64,
    wearable_mass_g: f64,
    layout: frame::FrameLayout,
}

fn mass(i: ProjectIn) -> Result<MassOut, String> {
    let p = &i.project;
    let geom = frame_geom(p);
    let density = p.design.frame.density_g_cm3;
    let parts: Vec<MassRow> = geom
        .parts
        .iter()
        .map(|b| MassRow { name: b.name.clone(), label: b.label.clone(), mass: b.assembly.mass_props(density) })
        .collect();
    let total = total_props(&parts.iter().map(|r| r.mass.clone()).collect::<Vec<_>>());
    let comps = p.components.iter().map(|c| (c.refdes.clone(), c.name.clone(), c.mass_g * c.qty, c.location.clone())).collect();
    let comp = p.component_mass_g();
    Ok(MassOut { frame_mass_g: total.mass_g, wearable_mass_g: total.mass_g + comp, total, parts, components: comps, components_mass_g: comp, layout: geom.layout.clone() })
}

#[derive(Deserialize)]
struct ValidateIn {
    mesh: Mesh,
    #[serde(default)]
    density_g_cm3: Option<f64>,
}

#[derive(Serialize)]
struct ValidateOut {
    check: MeshCheck,
    mass: MassProps,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TriIn {
    project: Option<Project>,
    outer: Vec<geom2d::P2>,
    holes: Vec<Vec<geom2d::P2>>,
}

#[derive(Serialize)]
struct TriOut {
    /// Concatenated loop vertices (outer, then holes).
    points: Vec<geom2d::P2>,
    loop_sizes: Vec<usize>,
    /// Triangles in the order the ears were clipped.
    triangles: Vec<[u32; 3]>,
    area_mm2: f64,
}

fn triangulate_api(i: TriIn) -> Result<TriOut, String> {
    let (outer, holes) = if i.outer.len() >= 3 {
        (geom2d::oriented(i.outer, true), i.holes.into_iter().map(|h| geom2d::oriented(h, false)).collect::<Vec<_>>())
    } else {
        let p = i.project.unwrap_or_else(Project::iris);
        let (o, l, r) = frame::front_profile(&p);
        (o, vec![geom2d::oriented(l, false), geom2d::oriented(r, false)])
    };
    let triangles = geom2d::triangulate(&outer, &holes);
    let mut points = outer.clone();
    let mut loop_sizes = vec![outer.len()];
    for h in &holes {
        points.extend_from_slice(h);
        loop_sizes.push(h.len());
    }
    let area = geom2d::area(&outer) + holes.iter().map(|h| geom2d::area(h)).sum::<f64>();
    Ok(TriOut { points, loop_sizes, triangles, area_mm2: area })
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct EnclosureIn {
    params: enclosure::EnclosureParams,
}

/// JSON API entry point for `cad.*` methods.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "cad.parts" => handle(input, |i: PartsIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            Ok(parts_out(&p, &i.orientation, i.display))
        }),
        "cad.stl" => handle(input, stl),
        "cad.obj" => handle(input, obj),
        "cad.3mf" => handle(input, three_mf),
        "cad.mass" => handle(input, mass),
        "cad.layout" => handle(input, |i: ProjectIn| Ok(frame_geom(&i.project).layout.clone())),
        "cad.fit" => handle(input, |i: fit::FitInput| Ok(fit::recommend(&i))),
        "cad.enclosure" => handle(input, |i: EnclosureIn| Ok(enclosure::evaluate(&i.params))),
        "cad.triangulate" => handle(input, triangulate_api),
        "cad.enclosure_defaults" => Some(crate::api::to_value(enclosure::EnclosureParams::default())),
        "cad.validate" => handle(input, |i: ValidateIn| {
            Ok(ValidateOut { check: i.mesh.validate(), mass: i.mesh.mass_props(i.density_g_cm3.unwrap_or(1.24)) })
        }),
        _ => None,
    }
}

/// Metrics this module contributes to requirement verification.
pub fn metrics(p: &Project) -> Vec<Metric> {
    let geom = frame_geom(p);
    let density = p.design.frame.density_g_cm3;
    let vol: f64 = geom.parts.iter().map(|b| b.assembly.volume()).sum();
    let mass = vol / 1000.0 * density;
    let l = &geom.layout;
    vec![
        Metric::new("cad.frame_mass_g", "Frame mass (all printed/molded parts)", mass, "g", "cad").with_note(&format!(
            "{} parts, {:.2} cm³ of {} at {:.2} g/cm³ (divergence-theorem volume)",
            geom.parts.len(),
            vol / 1000.0,
            p.design.frame.material,
            density
        )),
        Metric::new("cad.pod_inner_length_mm", "Electronics pod usable inner length", l.pod_usable_len_mm, "mm", "cad").with_note(&format!(
            "cavity {:.1} mm long at the floor, corner radius {:.2} mm, for a {:.1} mm tall board",
            l.pod_cavity_len_floor_mm, l.pod_cavity_corner_r_mm, p.design.pcb.height_mm
        )),
        Metric::new("cad.pod_inner_width_mm", "Electronics pod inner depth", l.pod_cavity_depth_mm, "mm", "cad")
            .with_note("floor to the lid's inner face"),
        Metric::new("cad.pod_inner_height_mm", "Electronics pod inner height", l.pod_cavity_height_floor_mm, "mm", "cad")
            .with_note("at the floor, where draft makes the cavity smallest"),
        Metric::new("cad.frame_volume_cm3", "Frame material volume", vol / 1000.0, "cm³", "cad"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_metrics_are_sensible() {
        let p = Project::iris();
        let m = metrics(&p);
        let get = |k: &str| m.iter().find(|x| x.key == k).unwrap().value;
        let mass = get("cad.frame_mass_g");
        assert!(mass > 12.0 && mass < 35.0, "frame mass {mass}");
        let inner = get("cad.pod_inner_length_mm");
        let margin = inner - p.design.pcb.width_mm;
        assert!(margin >= 1.0, "pcb margin {margin}");
        assert!(inner < p.design.frame.pod_length_mm);
        assert!(get("cad.pod_inner_height_mm") > p.design.pcb.height_mm);
    }

    #[test]
    fn frame_parts_interface() {
        let parts = frame_parts(&Project::iris());
        let names: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
        for n in ["front", "temple_r", "temple_l", "lid_r", "lid_l", "optics_housing"] {
            assert!(names.contains(&n), "missing {n}");
        }
        assert!(parts.iter().all(|p| p.mesh.triangle_count() > 0 && !p.color.is_empty()));
        // Right-handed, Y forward: the electronics temple is at +X and the
        // temples run backwards (towards −Y) from the front.
        let get = |n: &str| parts.iter().find(|p| p.name == n).unwrap().mesh.mass_props(1.0);
        assert!(get("temple_r").centroid[0] > 20.0 && get("temple_l").centroid[0] < -20.0);
        assert!(get("temple_r").centroid[1] < get("front").centroid[1] - 30.0);
        let pulls = frame_pull_directions(&Project::iris());
        assert!(pulls.iter().all(|(_, d)| (d[0].hypot(d[1]).hypot(d[2]) - 1.0).abs() < 1e-9));
    }

    #[test]
    fn metric_tracks_pod_length() {
        let mut p = Project::iris();
        let a = metrics(&p)[1].value;
        p.design.frame.pod_length_mm += 4.0;
        let b = metrics(&p)[1].value;
        assert!((b - a - 4.0).abs() < 1e-6, "{a} -> {b}");
    }

    #[test]
    fn dispatch_round_trips() {
        let p = serde_json::to_value(Project::iris()).unwrap();
        let out = dispatch("cad.parts", &serde_json::json!({"project": p, "orientation": "print"})).unwrap().unwrap();
        assert_eq!(out["parts"].as_array().unwrap().len(), 6);
        let stl = dispatch("cad.stl", &serde_json::json!({"project": p, "part": "lid_r"})).unwrap().unwrap();
        assert!(stl["base64"].as_str().unwrap().len() > 100);
        assert!(dispatch("cad.fit", &serde_json::json!({"face_width_mm": 140, "pd_mm": 64})).unwrap().is_ok());
        assert!(dispatch("cad.enclosure", &serde_json::json!({})).unwrap().is_ok());
        assert!(dispatch("cad.3mf", &serde_json::json!({"project": p})).unwrap().is_ok());
        let tri = dispatch("cad.triangulate", &serde_json::json!({"project": p})).unwrap().unwrap();
        let n: usize = tri["loop_sizes"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).sum();
        assert_eq!(tri["triangles"].as_array().unwrap().len(), n + 2);
        assert!(dispatch("cad.nope", &serde_json::json!({})).is_none());
    }

    #[test]
    fn generation_is_fast_enough() {
        let mut p = Project::iris();
        p.design.frame.bridge_mm = 19.0; // bypass cache
        let t0 = std::time::Instant::now();
        let g = frame::build(&p);
        let ms = t0.elapsed().as_secs_f64() * 1e3;
        let tris: usize = g.parts.iter().map(|x| x.print.triangle_count()).sum();
        eprintln!("frame build {ms:.2} ms, {tris} triangles");
        // Debug builds are ~20x slower than release wasm; keep a loose bound.
        assert!(ms < 1500.0);
    }
}
