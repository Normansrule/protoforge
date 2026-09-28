//! # Print engine (FDM slicer)
//!
//! Turns the CAD Studio's parts (in print orientation) into layers of
//! toolpaths, G-code, time/filament/cost estimates and an overhang report.
//!
//! | file | what |
//! |---|---|
//! | [`slicer`] | sectioning, islands, perimeters, skin, infill, ordering |
//! | [`motion`] | extrusion (bead) model, trapezoidal time model, statistics |
//! | [`gcode`] | generic Marlin/Klipper G-code writer |
//! | [`overhang`] | overhang faces and support suggestion |
//!
//! Metrics (`print.total_time_h`, `print.filament_g`) cover the whole Iris
//! frame set. Because `metrics()` runs on every project edit, they use the
//! **fast analytic estimator** in [`estimate`] (every layer sectioned,
//! toolpath lengths from Steiner's parallel-curve formula, same motion
//! model) and cache the result per parameter set. The tests compare it
//! with full slices of every Iris part; the Print Lab shows both.

use std::cell::RefCell;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::cad::mesh::{Mesh, MeshF32};
use crate::project::{Metric, PrintParams, Project};

pub mod estimate;
pub mod gcode;
pub mod motion;
pub mod overhang;
pub mod slicer;

use motion::{LayerStats, SliceStats};
use slicer::{Layer, SliceSettings};

/// Result of slicing one part.
pub struct SliceResult {
    pub layers: Vec<Layer>,
    pub per_layer: Vec<LayerStats>,
    pub stats: SliceStats,
    pub gcode: String,
}

fn density_for(p: Option<&Project>, st: &SliceSettings) -> f64 {
    st.density_g_cm3.or_else(|| p.map(|p| p.design.frame.density_g_cm3)).unwrap_or(1.24)
}

/// Full slice of one mesh (print orientation).
pub fn slice_mesh(mesh: &Mesh, pp: &PrintParams, st: &SliceSettings, density: f64, name: &str) -> SliceResult {
    let layers = slicer::slice(mesh, pp, st, None);
    let per_layer: Vec<LayerStats> = layers.iter().map(|l| motion::layer_stats(l, pp, st)).collect();
    let stats = motion::totals(&per_layer, None, density, st.cost_per_kg, mesh.volume(), pp.filament_diameter_mm);
    let gcode = if st.gcode { gcode::write(&layers, pp, st, &stats, name) } else { String::new() };
    SliceResult { layers, per_layer, stats, gcode }
}

/// Stratified-sample estimate: slices only ~`samples` layers (+ layer 0)
/// and weights each by the number of layers it represents.
pub fn estimate_mesh(mesh: &Mesh, pp: &PrintParams, st: &SliceSettings, density: f64, samples: usize) -> SliceStats {
    let (lo, hi) = mesh.bbox();
    let n = slicer::layer_tops(hi[2] - lo[2], st.first_layer_mm, pp.layer_height_mm).len();
    if n == 0 {
        return SliceStats::default();
    }
    let rest = n - 1;
    let stride = rest.div_ceil(samples.max(1)).max(1);
    let mut idx = vec![0usize];
    let mut weights = vec![1.0];
    let mut s = 1;
    while s < n {
        let e = (s + stride).min(n);
        idx.push((s + e - 1) / 2);
        weights.push((e - s) as f64);
        s = e;
    }
    let layers = slicer::slice(mesh, pp, st, Some(&idx));
    let per: Vec<LayerStats> = layers.iter().map(|l| motion::layer_stats(l, pp, st)).collect();
    // Layers can come back missing (empty sections): align by index.
    let mut w_aligned = Vec::with_capacity(per.len());
    for l in &per {
        let k = idx.iter().position(|&i| i == l.index).unwrap_or(0);
        w_aligned.push(weights[k]);
    }
    motion::totals(&per, Some(&w_aligned), density, st.cost_per_kg, mesh.volume(), pp.filament_diameter_mm)
}

/// Resolve a printable part of the project by name (frame parts, or the
/// enclosure stored in `design.extra.enclosure`).
pub fn project_part(p: &Project, name: &str) -> Result<Mesh, String> {
    if name == "enclosure_body" || name == "enclosure_lid" {
        let params: crate::cad::enclosure::EnclosureParams = p
            .design
            .extra
            .get("enclosure")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let b = crate::cad::enclosure::build(&params);
        return Ok(if name == "enclosure_lid" { b.lid_print } else { b.body_print });
    }
    let geom = crate::cad::frame_geom(p);
    geom.parts.iter().find(|b| b.name == name).map(|b| b.print.clone()).ok_or_else(|| format!("unknown part `{name}`"))
}

// ------------------------------------------------------------------ API

#[derive(Deserialize, Default)]
#[serde(default)]
struct SliceIn {
    project: Option<Project>,
    part: Option<String>,
    mesh: Option<Mesh>,
    params: Option<PrintParams>,
    settings: SliceSettings,
}

#[derive(Serialize)]
struct PathOut {
    k: u8,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    r: bool,
    /// Flat [x0, y0, x1, y1, ...] in 1/100 mm.
    p: Vec<i32>,
}

#[derive(Serialize)]
struct LayerOut {
    i: usize,
    z: f64,
    h: f64,
    paths: Vec<PathOut>,
}

#[derive(Serialize)]
struct SliceOut {
    part: String,
    layers: Vec<LayerOut>,
    per_layer: Vec<LayerStats>,
    stats: SliceStats,
    gcode: String,
    gcode_lines: usize,
    mesh: MeshF32,
    bbox: ([f64; 3], [f64; 3]),
    params: PrintParams,
    settings: SliceSettings,
    kinds: Vec<(u8, String)>,
    ms: f64,
}

fn resolve(i: &SliceIn) -> Result<(String, Mesh, Option<Project>), String> {
    let project = i.project.clone();
    if let Some(m) = &i.mesh {
        let m = crate::cad::export::weld(m, 1e-5);
        let (lo, hi) = m.bbox();
        let m = m.translated(-(lo[0] + hi[0]) / 2.0, -(lo[1] + hi[1]) / 2.0, -lo[2]);
        return Ok((i.part.clone().unwrap_or_else(|| "mesh".into()), m, project));
    }
    let p = project.clone().unwrap_or_else(Project::iris);
    let name = i.part.clone().unwrap_or_else(|| "lid_r".into());
    let m = project_part(&p, &name)?;
    Ok((name, m, Some(p)))
}

fn now_ms() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64() * 1e3).unwrap_or(0.0)
    }
    #[cfg(target_arch = "wasm32")]
    {
        0.0
    }
}

fn slice_api(i: SliceIn) -> Result<SliceOut, String> {
    let t0 = now_ms();
    let (name, mesh, project) = resolve(&i)?;
    let pp = i.params.clone().or_else(|| project.as_ref().map(|p| p.design.print.clone())).unwrap_or_default();
    let st = i.settings.clone();
    let density = density_for(project.as_ref(), &st);
    let r = slice_mesh(&mesh, &pp, &st, density, &name);
    let layers = r
        .layers
        .iter()
        .map(|l| LayerOut {
            i: l.index,
            z: l.z,
            h: l.height,
            paths: l
                .paths
                .iter()
                .map(|p| PathOut { k: p.kind.code(), r: p.retract, p: p.pts.iter().flat_map(|q| [(q[0] * 100.0).round() as i32, (q[1] * 100.0).round() as i32]).collect() })
                .collect(),
        })
        .collect();
    let gcode_lines = r.gcode.lines().count();
    Ok(SliceOut {
        part: name,
        layers,
        per_layer: r.per_layer,
        stats: r.stats,
        gcode: r.gcode,
        gcode_lines,
        bbox: mesh.bbox(),
        mesh: mesh.to_f32(),
        params: pp,
        settings: st,
        kinds: slicer::Kind::ALL.iter().map(|k| (k.code(), k.label().to_string())).collect(),
        ms: now_ms() - t0,
    })
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct EstimateIn {
    project: Option<Project>,
    params: Option<PrintParams>,
    settings: Option<SliceSettings>,
    /// "fast" (analytic, default), "sampled" (stratified real slices) or
    /// "full" (every layer).
    method: Option<String>,
    samples: Option<usize>,
}

#[derive(Serialize, Clone)]
struct PartEstimate {
    name: String,
    stats: SliceStats,
    volume_mm3: f64,
}

#[derive(Serialize, Clone)]
struct EstimateOut {
    parts: Vec<PartEstimate>,
    total_time_h: f64,
    total_time_text: String,
    total_filament_g: f64,
    total_filament_m: f64,
    total_cost_usd: f64,
    method: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Method {
    Fast,
    Sampled(usize),
    Full,
}

fn estimate_all(p: &Project, pp: &PrintParams, st: &SliceSettings, method: Method) -> EstimateOut {
    let density = density_for(Some(p), st);
    let mut st = st.clone();
    st.gcode = false;
    let geom = crate::cad::frame_geom(p);
    let parts: Vec<PartEstimate> = geom
        .parts
        .iter()
        .map(|b| {
            let stats = match method {
                Method::Full => slice_mesh(&b.print, pp, &st, density, &b.name).stats,
                Method::Sampled(n) => estimate_mesh(&b.print, pp, &st, density, n),
                Method::Fast => estimate::estimate_fast(&b.print, pp, &st, density),
            };
            PartEstimate { name: b.name.clone(), stats, volume_mm3: b.print.volume() }
        })
        .collect();
    let t: f64 = parts.iter().map(|x| x.stats.time_s).sum();
    EstimateOut {
        total_time_h: t / 3600.0,
        total_time_text: motion::fmt_duration(t),
        total_filament_g: parts.iter().map(|x| x.stats.filament_g).sum(),
        total_filament_m: parts.iter().map(|x| x.stats.filament_m).sum(),
        total_cost_usd: parts.iter().map(|x| x.stats.cost_usd).sum(),
        parts,
        method: match method {
            Method::Full => "full slice of every layer".into(),
            Method::Sampled(n) => format!("stratified sample of ~{n} sliced layers per part"),
            Method::Fast => "fast analytic estimate (sections + Steiner offsets)".into(),
        },
    }
}

thread_local! {
    static EST_CACHE: RefCell<HashMap<String, EstimateOut>> = RefCell::new(HashMap::new());
}

/// Samples per part for the "sampled" estimate method.
pub const METRIC_SAMPLES: usize = 10;

fn cached_estimate(p: &Project) -> EstimateOut {
    let key = serde_json::to_string(&(&p.design.frame, &p.design.optics, &p.design.pcb, &p.design.print)).unwrap_or_default();
    if let Some(v) = EST_CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return v;
    }
    let out = estimate_all(p, &p.design.print, &SliceSettings::default(), Method::Fast);
    EST_CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > 64 {
            c.clear();
        }
        c.insert(key, out.clone());
    });
    out
}

#[derive(Deserialize)]
#[serde(default)]
struct OverhangIn {
    project: Option<Project>,
    part: Option<String>,
    mesh: Option<Mesh>,
    threshold_deg: f64,
    orientation: String,
}

impl Default for OverhangIn {
    fn default() -> Self {
        OverhangIn { project: None, part: None, mesh: None, threshold_deg: 45.0, orientation: "print".into() }
    }
}

fn overhangs(i: OverhangIn) -> Result<overhang::OverhangReport, String> {
    let mesh = if let Some(m) = i.mesh {
        m
    } else {
        let p = i.project.unwrap_or_else(Project::iris);
        let name = i.part.unwrap_or_else(|| "front".into());
        if i.orientation == "assembly" {
            // As worn: stand the part on the bed without re-orienting it.
            let geom = crate::cad::frame_geom(&p);
            let b = geom.parts.iter().find(|b| b.name == name).ok_or_else(|| format!("unknown part `{name}`"))?;
            let (lo, hi) = b.assembly.bbox();
            b.assembly.translated(-(lo[0] + hi[0]) / 2.0, -(lo[1] + hi[1]) / 2.0, -lo[2])
        } else {
            project_part(&p, &name)?
        }
    };
    Ok(overhang::analyse(&mesh, i.threshold_deg))
}

/// JSON API entry point for `print.*` methods.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "print.slice" => handle(input, slice_api),
        "print.estimate" => handle(input, |i: EstimateIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            let pp = i.params.unwrap_or_else(|| p.design.print.clone());
            let st = i.settings.unwrap_or_default();
            let m = match i.method.as_deref() {
                Some("full") => Method::Full,
                Some("sampled") => Method::Sampled(i.samples.unwrap_or(METRIC_SAMPLES)),
                _ => Method::Fast,
            };
            Ok(estimate_all(&p, &pp, &st, m))
        }),
        "print.overhangs" => handle(input, overhangs),
        "print.settings_defaults" => Some(crate::api::to_value(SliceSettings::default())),
        _ => None,
    }
}

/// Metrics this module contributes to requirement verification.
pub fn metrics(p: &Project) -> Vec<Metric> {
    let e = cached_estimate(p);
    let pp = &p.design.print;
    vec![
        Metric::new("print.total_time_h", "Print time, full frame set", e.total_time_h, "h", "print").with_note(&format!(
            "{} parts at {:.2} mm layers, {} perimeters, {:.0}% infill, {:.0} mm/s ({})",
            e.parts.len(),
            pp.layer_height_mm,
            pp.perimeters,
            pp.infill_pct,
            pp.print_speed_mm_s,
            e.method
        )),
        Metric::new("print.filament_g", "Filament, full frame set", e.total_filament_g, "g", "print")
            .with_note(&format!("{:.2} m of {:.2} mm filament", e.total_filament_m, pp.filament_diameter_mm)),
        Metric::new("print.cost_usd", "Filament cost, full frame set", e.total_cost_usd, "USD", "print")
            .with_note("at 20 USD/kg (adjustable in the Print Lab)"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::solid::{box_mesh, cylinder};
    use crate::math::PI;

    fn pp() -> PrintParams {
        PrintParams::default()
    }

    #[test]
    fn cube_layers_are_squares_with_correct_area() {
        let m = box_mesh([-10.0, -10.0, 0.0], [10.0, 10.0, 10.0]);
        let layers = slicer::slice(&m, &pp(), &SliceSettings::default(), None);
        assert_eq!(layers.len(), 50);
        for l in &layers {
            assert!((l.area_mm2 - 400.0).abs() < 1e-6);
            let ext: Vec<_> = l.paths.iter().filter(|p| p.kind == slicer::Kind::ExternalPerimeter).collect();
            assert_eq!(ext.len(), 1);
            // External perimeter centre line is inset by w/2: side 20 - w.
            let side = 20.0 - 0.45;
            let len = motion::path_len(&ext[0].pts);
            assert!((len - 4.0 * side).abs() < 1e-6, "{len}");
        }
    }

    #[test]
    fn cylinder_layer_area() {
        let m = cylinder(0.0, 0.0, 10.0, 0.0, 4.0, 180);
        let layers = slicer::slice(&m, &pp(), &SliceSettings::default(), None);
        for l in &layers {
            assert!((l.area_mm2 - PI * 100.0).abs() / (PI * 100.0) < 1e-3);
        }
    }

    #[test]
    fn solid_cube_extrudes_about_its_volume() {
        // 100 % infill: extruded volume ≈ part volume (bead model and
        // perimeter/infill overlap give a few percent either way).
        let m = box_mesh([-10.0, -10.0, 0.0], [10.0, 10.0, 6.0]);
        let mut p = pp();
        p.infill_pct = 100.0;
        let st = SliceSettings { skirt: false, ..Default::default() };
        let r = slice_mesh(&m, &p, &st, 1.27, "cube");
        let ratio = r.stats.fill_ratio;
        assert!((0.9..1.1).contains(&ratio), "fill ratio {ratio}");
        // 20 % sparse infill uses clearly less.
        p.infill_pct = 20.0;
        let r2 = slice_mesh(&m, &p, &st, 1.27, "cube");
        assert!(r2.stats.fill_ratio < 0.8 * ratio);
        // Filament mass consistent with volume.
        let fil_vol = r.stats.filament_mm * PI * 1.75f64.powi(2) / 4.0;
        assert!((r.stats.filament_g - fil_vol / 1000.0 * 1.27).abs() < 1e-9);
    }

    #[test]
    fn gcode_is_valid_and_e_is_monotonic() {
        let m = box_mesh([-8.0, -5.0, 0.0], [8.0, 5.0, 2.0]);
        let r = slice_mesh(&m, &pp(), &SliceSettings::default(), 1.27, "block");
        let mut e_last = 0.0f64;
        let mut before_retract: Option<f64> = None;
        let mut extrusions = 0;
        for line in r.gcode.lines() {
            assert!(gcode::tests::valid_line(line), "bad line: {line}");
            let code = line.split(';').next().unwrap().trim();
            if code == "G92 E0" {
                e_last = 0.0;
                continue;
            }
            let words: Vec<&str> = code.split_whitespace().collect();
            let e = words.iter().find(|w| w.starts_with('E')).map(|w| w[1..].parse::<f64>().unwrap());
            let has_xy = words.iter().any(|w| w.starts_with('X') || w.starts_with('Y'));
            if let Some(e) = e {
                if has_xy {
                    assert!(e >= e_last - 1e-9, "E decreased on an extrusion move: {line}");
                    extrusions += 1;
                    e_last = e;
                } else if e < e_last {
                    before_retract = Some(e_last); // retraction
                } else if let Some(b) = before_retract.take() {
                    assert!((e - b).abs() < 1e-4, "unretract must restore E");
                }
            }
        }
        assert!(extrusions > 100);
        assert!(r.gcode.contains("M109 S240") && r.gcode.contains("M190 S80") && r.gcode.contains("G28"));
    }

    #[test]
    fn iris_parts_slice_and_estimate_agrees() {
        let p = Project::iris();
        let st = SliceSettings { gcode: false, ..Default::default() };
        let full = estimate_all(&p, &p.design.print, &st, Method::Full);
        let est = estimate_all(&p, &p.design.print, &st, Method::Fast);
        for (f, e) in full.parts.iter().zip(&est.parts) {
            let dt = (e.stats.time_s - f.stats.time_s) / f.stats.time_s;
            let dg = (e.stats.filament_g - f.stats.filament_g) / f.stats.filament_g;
            eprintln!(
                "{:15} full {:6.1} min {:5.2} g | fast {:6.1} min {:5.2} g | Δt {:+.1}% Δg {:+.1}% fill {:.2}",
                f.name,
                f.stats.time_s / 60.0,
                f.stats.filament_g,
                e.stats.time_s / 60.0,
                e.stats.filament_g,
                dt * 100.0,
                dg * 100.0,
                f.stats.fill_ratio
            );
            assert!(f.stats.filament_g > 0.0 && f.stats.time_s > 0.0);
            assert!(dt.abs() < 0.10 && dg.abs() < 0.10, "{}: Δt {dt} Δg {dg}", f.name);
            assert!(f.stats.fill_ratio > 0.3 && f.stats.fill_ratio < 1.15, "{} fill {}", f.name, f.stats.fill_ratio);
        }
        eprintln!("total full {:.2} h {:.1} g, fast {:.2} h {:.1} g", full.total_time_h, full.total_filament_g, est.total_time_h, est.total_filament_g);
        assert!(full.total_time_h < 8.0);
        let dt = (est.total_time_h - full.total_time_h) / full.total_time_h;
        let dg = (est.total_filament_g - full.total_filament_g) / full.total_filament_g;
        assert!(dt.abs() < 0.05 && dg.abs() < 0.05, "total Δt {dt} Δg {dg}");
    }

    #[test]
    fn every_iris_part_prints_without_support() {
        let p = Project::iris();
        for part in crate::cad::frame_parts_print(&p) {
            let r = overhang::analyse(&part.mesh, 45.0);
            assert!(r.overhang_area_mm2 < 1.0, "{}: {} mm² overhang", part.name, r.overhang_area_mm2);
        }
    }

    #[test]
    fn fast_estimator_is_fast() {
        let p = Project::iris();
        let _ = crate::cad::frame_geom(&p);
        let t0 = std::time::Instant::now();
        let e = estimate_all(&p, &p.design.print, &SliceSettings::default(), Method::Fast);
        let ms = t0.elapsed().as_secs_f64() * 1e3;
        eprintln!("fast estimate of {} parts: {ms:.1} ms", e.parts.len());
        assert!(ms < 400.0);
    }

    #[test]
    fn metrics_present() {
        let m = metrics(&Project::iris());
        assert!(m.iter().any(|x| x.key == "print.total_time_h" && x.value > 0.1));
        assert!(m.iter().any(|x| x.key == "print.filament_g" && x.value > 5.0));
    }

    #[test]
    fn dispatch_slice_and_overhangs() {
        let p = serde_json::to_value(Project::iris()).unwrap();
        let out = dispatch("print.slice", &serde_json::json!({"project": p, "part": "lid_l"})).unwrap().unwrap();
        assert!(out["layers"].as_array().unwrap().len() > 5);
        assert!(out["gcode"].as_str().unwrap().contains(";LAYER:1"));
        let o = dispatch("print.overhangs", &serde_json::json!({"project": p, "part": "front"})).unwrap().unwrap();
        assert!(o["overhang_area_mm2"].as_f64().unwrap() < 1.0, "print orientation is support-free");
        let a = dispatch("print.overhangs", &serde_json::json!({"project": p, "part": "temple_r", "orientation": "assembly"})).unwrap().unwrap();
        assert!(a["overhang_area_mm2"].as_f64().unwrap() > 50.0, "as-worn orientation needs support: {}", a["overhang_area_mm2"]);
    }
}


