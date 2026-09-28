//! Scene export for the web **Ray Optics Simulation** app (Yi-Ting Tu,
//! <https://phydemo.app/ray-optics/>, source
//! <https://github.com/ricktu288/ray-optics>): a 2-D side view of the Iris
//! heads-up display — display pixels as point sources, the collimating lens
//! as an ideal lens, the 45° combiner as a partially reflecting beam
//! splitter, the housing walls as blockers and the eye pupil as a detector.
//!
//! The JSON follows the app's scene format version 5: a top-level
//! `{version, name, objs, mode, rayModeDensity, origin, scale, width,
//! height}` and objects with the keys of each class's
//! `serializableDefaults` (`IdealLens {p1, p2, focalLength}`, `BeamSplitter
//! {p1, p2, transRatio}`, `PointSource {x, y, brightness}`, `Blocker`,
//! `Detector {p1, p2}`, `TextLabel {x, y, text, fontSize}`). The app rejects
//! unknown keys, so only those are written. Load it with the app's **Open**
//! button (the URL-hash format is LZMA-compressed and is not generated here).

use serde_json::{json, Value};

use crate::project::Project;

/// Pixels per millimetre in the exported scene.
pub const SCALE: f64 = 20.0;

fn pt(x: f64, y: f64) -> Value {
    json!({ "x": (x * SCALE * 100.0).round() / 100.0, "y": (y * SCALE * 100.0).round() / 100.0 })
}

/// The Ray Optics scene JSON for the project's display optics.
pub fn scene(p: &Project) -> Value {
    let o = &p.design.optics;
    let d = o.display_to_lens_mm.max(0.1);
    let lc = o.lens_to_combiner_mm.max(0.0);
    let big_d = o.lens_diameter_mm.max(1.0);
    let h = o.display_height_mm.max(0.5);
    let er = o.eye_relief_mm.max(1.0);
    let xc = d + lc; // combiner centre on the axis
    let half = 0.5 * big_d * 1.25; // combiner half-size along x (45°)
    let mut objs = vec![
        // the housing: back wall and top/bottom walls around the display and lens
        json!({ "type": "Blocker", "p1": pt(-1.0, -big_d / 2.0 - 1.0), "p2": pt(-1.0, big_d / 2.0 + 1.0) }),
        json!({ "type": "Blocker", "p1": pt(-1.0, -big_d / 2.0 - 1.0), "p2": pt(d, -big_d / 2.0 - 1.0) }),
        json!({ "type": "Blocker", "p1": pt(-1.0, big_d / 2.0 + 1.0), "p2": pt(d, big_d / 2.0 + 1.0) }),
        json!({ "type": "IdealLens", "p1": pt(d, -big_d / 2.0), "p2": pt(d, big_d / 2.0), "focalLength": (o.lens_focal_mm * SCALE * 100.0).round() / 100.0 }),
        json!({ "type": "BeamSplitter", "p1": pt(xc - half, -half), "p2": pt(xc + half, half), "transRatio": ((1.0 - o.combiner_reflectance).clamp(0.0, 1.0) * 1000.0).round() / 1000.0 }),
        json!({ "type": "Detector", "p1": pt(xc - 2.0, er), "p2": pt(xc + 2.0, er) }),
    ];
    for y in [-h / 2.0, 0.0, h / 2.0] {
        let mut s = json!({ "type": "PointSource", "brightness": 0.5 });
        let q = pt(0.0, y);
        s["x"] = q["x"].clone();
        s["y"] = q["y"].clone();
        objs.push(s);
    }
    let label = |x: f64, y: f64, text: String| {
        let q = pt(x, y);
        json!({ "type": "TextLabel", "x": q["x"], "y": q["y"], "text": text, "fontSize": 16 })
    };
    objs.push(label(
        -1.0,
        -big_d / 2.0 - 3.0,
        format!("{} display ({} mm tall)", p.meta.codename, h),
    ));
    objs.push(label(
        d - 2.0,
        big_d / 2.0 + 4.0,
        format!("lens f = {} mm, {} mm from display", o.lens_focal_mm, d),
    ));
    objs.push(label(
        xc + half + 1.0,
        -half,
        format!("combiner R = {:.0} %, {} mm after lens", o.combiner_reflectance * 100.0, lc),
    ));
    objs.push(label(xc + 3.0, er + 1.0, format!("eye pupil ({} mm eye relief)", er)));
    json!({
        "version": 5,
        "name": format!("{} heads-up display (side view)", p.meta.name),
        "objs": objs,
        "mode": "rays",
        "rayModeDensity": 0.25,
        "origin": { "x": 120, "y": 360 },
        "scale": 1,
        "width": 1500,
        "height": 900
    })
}

/// Pretty-printed scene file content.
pub fn scene_json(p: &Project) -> String {
    serde_json::to_string_pretty(&scene(p)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_scene_uses_known_keys_only() {
        let s = scene(&Project::iris());
        let allowed: &[(&str, &[&str])] = &[
            ("IdealLens", &["type", "p1", "p2", "focalLength"]),
            ("BeamSplitter", &["type", "p1", "p2", "transRatio"]),
            ("PointSource", &["type", "x", "y", "brightness"]),
            ("Blocker", &["type", "p1", "p2"]),
            ("Detector", &["type", "p1", "p2"]),
            ("TextLabel", &["type", "x", "y", "text", "fontSize"]),
        ];
        for o in s["objs"].as_array().unwrap() {
            let t = o["type"].as_str().unwrap();
            let keys = allowed.iter().find(|a| a.0 == t).unwrap_or_else(|| panic!("unexpected {t}")).1;
            for k in o.as_object().unwrap().keys() {
                assert!(keys.contains(&k.as_str()), "{t}.{k}");
            }
        }
        let lens = s["objs"].as_array().unwrap().iter().find(|o| o["type"] == "IdealLens").unwrap();
        assert_eq!(lens["focalLength"], 400.0); // 20 mm × 20 px/mm
        let bs = s["objs"].as_array().unwrap().iter().find(|o| o["type"] == "BeamSplitter").unwrap();
        assert!((bs["transRatio"].as_f64().unwrap() - 0.7).abs() < 1e-9);
    }
}
