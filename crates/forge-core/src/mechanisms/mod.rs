//! # Mechanisms Lab engine — linkages, springs, gears, cams and the Iris spring hinge
//!
//! | file | what |
//! |---|---|
//! | [`linkage`] | four-bar (Grashof class, Freudenstein position, velocity, acceleration, coupler curves, transmission angle, toggles), offset slider-crank, three-position motion synthesis |
//! | [`springs`] | helical compression, extension and torsion springs per Shigley: wire table (A, m), rate, Bergsträsser/Wahl, solid height, buckling, Goodman–Zimmerli fatigue, finite life |
//! | [`gears`] | rack-generated involute spur gears (profile shift, trochoidal root fillet, undercut), meshing and contact ratio, Lewis + Barth bending, gear trains, planetary sets |
//! | [`cams`] | SVAJ for cycloidal, 3-4-5 and harmonic motion; translating roller-follower profile, pressure angle, radius of curvature |
//! | [`hinge`] | the Iris flex hinge (HW1): end-face detent cam + barrel compression spring, torque vs angle, clamp on the head, fatigue life |
//! | [`export`] | DXF R12 and SVG writers |
//!
//! Metrics (see [`metrics`]), all from the Iris hinge in `design.extra.mechanisms.hinge`:
//! `mechanisms.hinge_spring_sf` (Goodman fatigue safety factor, worst usage block),
//! `mechanisms.hinge_life_cycles` (open/close uses to failure, capped at 10⁷ = runout) and
//! `mechanisms.hinge_open_torque_nmm` (torque to push the folded temple over the detent).
//!
//! Snap fits, screws and living hinges live in the Structures Lab (`structures.snap_fit`, `structures.bolt`,
//! `structures.living_hinge`) and are not duplicated here.
//!
//! ## REFERENCES
//! * R. L. Norton, *Design of Machinery*, 6th ed., McGraw-Hill, 2019 — linkages (ch. 2–7), cams (ch. 8), gears (ch. 9).
//! * J. J. Uicker, G. R. Pennock & J. E. Shigley, *Theory of Machines and Mechanisms*, 5th ed., Oxford, 2017.
//! * R. G. Budynas & J. K. Nisbett, *Shigley's Mechanical Engineering Design*, 10th ed., McGraw-Hill, 2015 — springs (ch. 10), gears (ch. 13–14).
//! * A. M. Wahl, *Mechanical Springs*, 2nd ed., McGraw-Hill, 1963.
//! * F. L. Litvin & A. Fuentes, *Gear Geometry and Applied Theory*, 2nd ed., Cambridge, 2004.
//! * F. Freudenstein, "Approximate synthesis of four-bar linkages", Trans. ASME 77, 1955.
//! * ISO 53:1998, cylindrical gears — standard basic rack tooth profile; ISO 21771 (gear geometry); AGMA 2001 and ISO 6336 (rating).
//! * O. Zimmerli, "Fatigue of spring wire", 1957 (via Shigley ch. 10).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Metric, Project};

pub mod cams;
pub mod export;
pub mod gears;
pub mod hinge;
pub mod linkage;
pub mod springs;

#[derive(Deserialize, Default)]
#[serde(default)]
struct HingeIn {
    project: Option<Project>,
    /// Override the project's hinge parameters.
    params: Option<hinge::HingeParams>,
}

#[derive(Deserialize)]
#[serde(default)]
struct GearExportIn {
    gear: gears::GearIn,
    bore_mm: f64,
    /// Title written into the SVG.
    title: String,
}

impl Default for GearExportIn {
    fn default() -> Self {
        GearExportIn {
            gear: gears::GearIn::default(),
            bore_mm: 5.0,
            title: "Protoforge spur gear".into(),
        }
    }
}

#[derive(Serialize)]
struct ExportOut {
    svg: String,
    dxf: String,
    points: usize,
    notes: Vec<String>,
}

fn gear_export(i: GearExportIn) -> Result<ExportOut, String> {
    let g = gears::profile(&i.gear)?;
    let pts = gears::outline(&g, 0.0);
    let mut circles = vec![export::Circle {
        layer: "CONSTRUCTION",
        c: [0.0, 0.0],
        r: g.pitch_r,
    }];
    if i.bore_mm > 0.0 && i.bore_mm / 2.0 < g.root_r * 0.8 {
        circles.push(export::Circle {
            layer: "CUT",
            c: [0.0, 0.0],
            r: i.bore_mm / 2.0,
        });
    }
    let polys = [export::Poly {
        layer: "CUT",
        pts: &pts,
        closed: true,
    }];
    let title = format!("{} — m {} z {} α {}° x {}", i.title, g.module_mm, g.teeth, g.pressure_deg, g.shift);
    Ok(ExportOut {
        svg: export::svg(&polys, &circles, &title),
        dxf: export::dxf(&polys, &circles),
        points: pts.len(),
        notes: vec![
            "Layer CUT = outline and bore; CONSTRUCTION = pitch circle (dashed in the SVG).".into(),
            "For laser cutting, the kerf removes about half its width from every edge — offset the outline outward by kerf/2 or add backlash.".into(),
        ],
    })
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct CamExportIn {
    cam: cams::CamIn,
    bore_mm: f64,
}

fn cam_export(i: CamExportIn) -> Result<ExportOut, String> {
    let o = cams::cam(i.cam)?;
    let pts: Vec<[f64; 2]> = o.cam_x.iter().zip(&o.cam_y).map(|(x, y)| [*x, *y]).collect();
    let pitch: Vec<[f64; 2]> = o.pitch_x.iter().zip(&o.pitch_y).map(|(x, y)| [*x, *y]).collect();
    let polys = [
        export::Poly {
            layer: "CUT",
            pts: &pts,
            closed: true,
        },
        export::Poly {
            layer: "CONSTRUCTION",
            pts: &pitch,
            closed: true,
        },
    ];
    let mut circles = vec![];
    if i.bore_mm > 0.0 {
        circles.push(export::Circle {
            layer: "CUT",
            c: [0.0, 0.0],
            r: i.bore_mm / 2.0,
        });
    }
    Ok(ExportOut {
        svg: export::svg(&polys, &circles, "Protoforge disk cam (roller follower)"),
        dxf: export::dxf(&polys, &circles),
        points: pts.len(),
        notes: vec![
            "Layer CUT = cam profile (and bore); CONSTRUCTION = pitch curve traced by the roller centre. θ = 0 follower position is +Y."
                .into(),
        ],
    })
}

/// JSON API for the Mechanisms Lab.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "mechanisms.fourbar" => handle(input, linkage::fourbar),
        "mechanisms.slider_crank" => handle(input, linkage::slider_crank),
        "mechanisms.synthesis3" => handle(input, linkage::synth3),
        "mechanisms.grashof" => handle(input, |i: linkage::FourBarIn| Ok(linkage::grashof(i.a, i.b, i.c, i.d))),
        "mechanisms.spring_materials" => Some(crate::api::to_value(springs::WIRES)),
        "mechanisms.compression_spring" => handle(input, springs::compression),
        "mechanisms.extension_spring" => handle(input, springs::extension),
        "mechanisms.torsion_spring" => handle(input, springs::torsion),
        "mechanisms.gear" => handle(input, |i: gears::GearIn| gears::profile(&i)),
        "mechanisms.gear_pair" => handle(input, gears::pair),
        "mechanisms.gear_export" => handle(input, gear_export),
        "mechanisms.gear_train" => handle(input, gears::train),
        "mechanisms.planetary" => handle(input, gears::planetary),
        "mechanisms.cam" => handle(input, cams::cam),
        "mechanisms.cam_export" => handle(input, cam_export),
        "mechanisms.hinge" => handle(input, |i: HingeIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            let h = i.params.unwrap_or_else(|| hinge::HingeParams::from_project(&p));
            hinge::analyze(&p, &h)
        }),
        "mechanisms.hinge_defaults" => Some(crate::api::to_value(hinge::HingeParams::default())),
        _ => None,
    }
}

thread_local! {
    static METRICS: crate::memo::Slot<Vec<Metric>> = const { std::cell::RefCell::new(None) };
}

/// Metrics feeding requirement verification (the Iris hinge).
pub fn metrics(p: &Project) -> Vec<Metric> {
    let key = crate::memo::key(&(&p.design.frame, &p.design.optics, &p.design.pcb, p.design.extra.get("mechanisms")));
    crate::memo::memo(&METRICS, key, || metrics_uncached(p))
}

fn metrics_uncached(p: &Project) -> Vec<Metric> {
    let h = hinge::HingeParams::from_project(p);
    let o = match hinge::analyze(p, &h) {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    let src = "mechanisms";
    let worst = o
        .blocks
        .iter()
        .filter(|b| b.per_day > 0.0 && b.travel_mm > 1e-9)
        .min_by(|a, b| a.nf.total_cmp(&b.nf))
        .map(|b| b.name.to_lowercase())
        .unwrap_or_default();
    vec![
        Metric::new("mechanisms.hinge_spring_sf", "Hinge spring fatigue safety factor (Goodman)", o.nf_min, "x", src).with_note(&format!(
            "worst block: {worst}; {} Ø{} mm wire, D {} mm, {} active coils, k = {:.2} N/mm, preload {:.2} N; Zimmerli S_se = {:.0} MPa, S_su = {:.0} MPa",
            o.spring.material, h.wire_d_mm, h.mean_d_mm, h.active_coils, o.spring.rate_n_mm, o.spring.preload_n, o.spring.sse_mpa, o.spring.ssu_mpa
        )),
        Metric::new("mechanisms.hinge_life_cycles", "Hinge spring life, open/close uses", o.life_uses, "cycles", src).with_note(&format!(
            "{}; Miner's rule on the Table 10-8 allowables; design need {:.0} uses ({} per day × {} years)",
            if o.runout {
                "every block below the 10⁷-cycle allowable: runout, reported as 10⁷".to_string()
            } else {
                format!("{:.1} years at {} uses per day", o.life_years, h.uses_per_day)
            },
            o.design_uses,
            h.uses_per_day,
            h.design_years
        )),
        Metric::new("mechanisms.hinge_open_torque_nmm", "Torque to open the temple past the detent", o.detent_open_torque_nmm, "N·mm", src).with_note(&format!(
            "end-face detent: contact from {:.0}°, hump at {:.0}° ({:.2} mm slider travel); includes pin friction μ = {}; closing needs {:.1} N·mm",
            o.contact_start_deg, o.hump_deg, o.hump_travel_mm, h.friction, o.detent_close_torque_nmm
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn metrics_present_for_iris() {
        let m = metrics(&Project::iris());
        for (k, u) in [
            ("mechanisms.hinge_spring_sf", "x"),
            ("mechanisms.hinge_life_cycles", "cycles"),
            ("mechanisms.hinge_open_torque_nmm", "N·mm"),
        ] {
            let x = m.iter().find(|x| x.key == k).unwrap_or_else(|| panic!("missing {k}"));
            assert!(x.value.is_finite() && x.value > 0.0, "{k} = {}", x.value);
            assert_eq!(x.unit, u);
        }
        let t = std::time::Instant::now();
        let again = metrics(&Project::iris());
        assert_eq!(again, m);
        assert!(t.elapsed().as_millis() < 100);
    }

    #[test]
    fn dispatch_every_method() {
        for (mth, inp) in [
            ("mechanisms.fourbar", json!({})),
            ("mechanisms.slider_crank", json!({})),
            ("mechanisms.synthesis3", json!({})),
            ("mechanisms.grashof", json!({"a": 1, "b": 3, "c": 2, "d": 2.5})),
            ("mechanisms.spring_materials", json!({})),
            ("mechanisms.compression_spring", json!({})),
            ("mechanisms.extension_spring", json!({})),
            ("mechanisms.torsion_spring", json!({})),
            ("mechanisms.gear", json!({"teeth": 12})),
            ("mechanisms.gear_pair", json!({})),
            ("mechanisms.gear_export", json!({})),
            ("mechanisms.gear_train", json!({})),
            ("mechanisms.planetary", json!({})),
            ("mechanisms.cam", json!({})),
            ("mechanisms.cam_export", json!({})),
            ("mechanisms.hinge", json!({})),
            ("mechanisms.hinge_defaults", json!({})),
        ] {
            let r = dispatch(mth, &inp).unwrap();
            assert!(r.is_ok(), "{mth}: {r:?}");
        }
        assert!(dispatch("mechanisms.nope", &json!({})).is_none());
    }

    #[test]
    fn hinge_reads_project_extra() {
        let mut p = Project::iris();
        let base = hinge::analyze(&p, &hinge::HingeParams::from_project(&p)).unwrap();
        let mut stiffer = hinge::HingeParams::from_project(&p);
        stiffer.preload_mm += 0.4;
        p.design.extra.insert("mechanisms".into(), json!({ "hinge": stiffer }));
        let h = hinge::HingeParams::from_project(&p);
        assert_eq!(h.preload_mm, stiffer.preload_mm);
        let o = hinge::analyze(&p, &h).unwrap();
        assert!(o.spring.preload_n > base.spring.preload_n);
        assert!(o.detent_open_torque_nmm > base.detent_open_torque_nmm);
    }
}
