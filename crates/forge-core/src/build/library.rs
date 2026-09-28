//! Process settings library — "ideal settings" as **typical starting points**.
//!
//! Every range here is a typical starting window compiled from general,
//! widely published guidance (filament and resin makers' generic processing
//! notes, the Prusa Knowledge Base, the Bambu Lab Wiki, Machinery's Handbook,
//! Rosato & Rosato's *Injection Molding Handbook*, IPC-7525 and J-STD-020).
//! Real materials vary by brand, grade, colour and machine: always start from
//! the maker's datasheet or the matching slicer profile, print a calibration
//! test and **tune on your machine**. Nothing here is a vendor profile.
//!
//! References: I. Gibson, D. Rosen, B. Stucker, *Additive Manufacturing
//! Technologies* (3rd ed., Springer 2021); ISO/ASTM 52900 *Additive
//! manufacturing — General principles — Fundamentals and vocabulary*;
//! E. Oberg et al., *Machinery's Handbook* (Industrial Press); D. V. Rosato,
//! M. G. Rosato, *Injection Molding Handbook* (3rd ed., 2000); Prusa
//! Knowledge Base <https://help.prusa3d.com>; Bambu Lab Wiki
//! <https://wiki.bambulab.com>.

use serde::Serialize;

use crate::project::Project;

/// A recommended numeric window, drawn as a range bar on `scale`.
#[derive(Clone, Debug, Serialize)]
pub struct Range {
    pub label: &'static str,
    pub lo: f64,
    pub hi: f64,
    pub unit: &'static str,
    /// Axis for the range bar `[min, max]`.
    pub scale: [f64; 2],
    #[serde(skip_serializing_if = "str::is_empty")]
    pub note: &'static str,
}

const fn r(label: &'static str, lo: f64, hi: f64, unit: &'static str, scale: [f64; 2], note: &'static str) -> Range {
    Range {
        label,
        lo,
        hi,
        unit,
        scale,
        note,
    }
}

/// A qualitative setting ("Enclosure: required").
#[derive(Clone, Debug, Serialize)]
pub struct Fact {
    pub label: &'static str,
    pub value: String,
}

fn f(label: &'static str, value: impl Into<String>) -> Fact {
    Fact {
        label,
        value: value.into(),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MaterialCard {
    pub id: &'static str,
    pub name: &'static str,
    pub ranges: Vec<Range>,
    pub facts: Vec<Fact>,
    pub notes: Vec<&'static str>,
}

/// A design rule. `value` may be computed from the project (e.g. from the
/// Print lab's line width).
#[derive(Clone, Debug, Serialize)]
pub struct Rule {
    pub title: &'static str,
    pub value: String,
    pub detail: &'static str,
    /// Id of the small SVG diagram drawn by the web page.
    pub diagram: &'static str,
}

fn rule(title: &'static str, value: impl Into<String>, detail: &'static str, diagram: &'static str) -> Rule {
    Rule {
        title,
        value: value.into(),
        detail,
        diagram,
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Failure {
    pub name: &'static str,
    pub symptom: &'static str,
    pub causes: &'static [&'static str],
    pub fixes: &'static [&'static str],
    pub diagram: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct LabLink {
    pub label: &'static str,
    pub route: &'static str,
}

/// Compares a project value with a recommended window.
#[derive(Clone, Debug, Serialize)]
pub struct ProjectCheck {
    pub label: String,
    pub value: f64,
    pub lo: f64,
    pub hi: f64,
    pub unit: &'static str,
    /// "pass" | "warn"
    pub status: &'static str,
    pub note: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Process {
    pub id: &'static str,
    pub name: &'static str,
    pub group: &'static str,
    pub summary: &'static str,
    /// Always shown next to the numbers.
    pub caveat: &'static str,
    /// Animation id for the settings card.
    pub anim: &'static str,
    pub materials: Vec<MaterialCard>,
    pub rules: Vec<Rule>,
    pub failures: Vec<Failure>,
    pub tips: Vec<String>,
    pub links: Vec<LabLink>,
    pub refs: Vec<&'static str>,
    pub project_checks: Vec<ProjectCheck>,
}

pub const CAVEAT: &str =
    "Typical starting points — tune on your machine. Use the material maker's datasheet or the matching slicer / machine profile first.";

// ---------------------------------------------------------------------------
// FDM
// ---------------------------------------------------------------------------

const NOZ: [f64; 2] = [170.0, 320.0];
const BED: [f64; 2] = [0.0, 130.0];
const FAN: [f64; 2] = [0.0, 100.0];
const SHR: [f64; 2] = [0.0, 2.0];

/// Typical FDM windows per filament.
pub fn fdm_materials() -> Vec<MaterialCard> {
    vec![
        MaterialCard {
            id: "PLA",
            name: "PLA",
            ranges: vec![
                r("Nozzle", 190.0, 220.0, "°C", NOZ, ""),
                r("Bed", 50.0, 65.0, "°C", BED, "can print cold on textured PEI"),
                r("Part-cooling fan", 80.0, 100.0, "%", FAN, "first layers lower"),
                r("Typical shrink", 0.2, 0.5, "%", SHR, ""),
            ],
            facts: vec![
                f("Enclosure", "Not needed — with an enclosed printer keep the door / lid open to avoid heat creep"),
                f("Drying", "Rarely needed; dry at 45–50 °C if it pops or strings"),
                f("Bed surface", "Smooth or textured PEI, clean with isopropyl alcohol"),
                f("Retraction", "Short on direct drive (≈0.5–1 mm), longer on Bowden (≈2–6 mm)"),
                f("Speed", "Fast — limited by the hotend's volumetric flow; slow outer walls for looks"),
                f("Glass transition", "≈55–60 °C — deforms in a hot car"),
            ],
            notes: vec!["Easiest material; brittle and heat-sensitive, so not for the final Iris frame (worn in sun, near skin)."],
        },
        MaterialCard {
            id: "PETG",
            name: "PETG",
            ranges: vec![
                r("Nozzle", 220.0, 250.0, "°C", NOZ, ""),
                r("Bed", 70.0, 85.0, "°C", BED, ""),
                r("Part-cooling fan", 30.0, 60.0, "%", FAN, "more fan = better overhangs, weaker layers"),
                r("Typical shrink", 0.2, 0.5, "%", SHR, ""),
            ],
            facts: vec![
                f("Enclosure", "Not needed"),
                f("Drying", "Recommended when stringy: ≈60–65 °C for a few hours"),
                f("Bed surface", "Textured PEI; on smooth PEI use glue stick as a release layer (PETG can bond too well)"),
                f("Retraction", "Moderate; PETG strings — tune retraction and travel, lower temperature a little"),
                f("Speed", "Medium; slower than PLA for clean overhangs"),
                f("Glass transition", "≈75–80 °C"),
            ],
            notes: vec!["Iris prototype frame material: tough, slightly flexible, fine for EVT frames."],
        },
        MaterialCard {
            id: "ABS",
            name: "ABS",
            ranges: vec![
                r("Nozzle", 230.0, 260.0, "°C", NOZ, ""),
                r("Bed", 90.0, 110.0, "°C", BED, ""),
                r("Part-cooling fan", 0.0, 20.0, "%", FAN, "bridges only"),
                r("Typical shrink", 0.5, 0.8, "%", SHR, ""),
            ],
            facts: vec![
                f("Enclosure", "Required — warm, draft-free chamber against warping and cracking"),
                f("Ventilation", "Emits styrene and ultrafine particles: filter or vent the enclosure"),
                f("Drying", "≈60–80 °C if it has absorbed moisture"),
                f("Bed surface", "Textured / smooth PEI (glue stick helps), brim for long parts"),
                f("Retraction", "Moderate"),
                f("Post-processing", "Acetone vapour smoothing is possible — only with proper ventilation"),
            ],
            notes: vec![],
        },
        MaterialCard {
            id: "ASA",
            name: "ASA",
            ranges: vec![
                r("Nozzle", 240.0, 260.0, "°C", NOZ, ""),
                r("Bed", 90.0, 110.0, "°C", BED, ""),
                r("Part-cooling fan", 0.0, 30.0, "%", FAN, ""),
                r("Typical shrink", 0.5, 0.7, "%", SHR, ""),
            ],
            facts: vec![
                f("Enclosure", "Required"),
                f("Ventilation", "Filter or vent (styrene-family emissions)"),
                f("Drying", "≈60–80 °C if wet"),
                f("Bed surface", "Textured PEI, brim for long parts"),
                f("Use", "UV-stable ABS alternative — good for outdoor parts"),
            ],
            notes: vec![],
        },
        MaterialCard {
            id: "TPU95A",
            name: "TPU 95A (flexible)",
            ranges: vec![
                r("Nozzle", 210.0, 235.0, "°C", NOZ, ""),
                r("Bed", 30.0, 60.0, "°C", BED, ""),
                r("Part-cooling fan", 50.0, 100.0, "%", FAN, ""),
                r("Print speed", 15.0, 40.0, "mm/s", [0.0, 300.0], "slow and steady — flexible filament buckles"),
            ],
            facts: vec![
                f("Extruder", "Direct drive strongly preferred; constrained filament path"),
                f("Retraction", "Minimal or off; avoid long travels"),
                f("Drying", "Recommended (hygroscopic): ≈50–55 °C for several hours"),
                f("Bed surface", "Textured PEI; on smooth PEI use a release agent (glue stick) so parts come off"),
                f("Use", "Nose pads, temple tips, bumpers, gaskets"),
            ],
            notes: vec![],
        },
        MaterialCard {
            id: "PA",
            name: "PA / nylon",
            ranges: vec![
                r("Nozzle", 250.0, 280.0, "°C", NOZ, "grade-dependent; some copolymers print lower"),
                r("Bed", 70.0, 100.0, "°C", BED, ""),
                r("Part-cooling fan", 0.0, 30.0, "%", FAN, ""),
                r("Typical shrink", 0.8, 1.5, "%", SHR, "warps on long flat parts"),
            ],
            facts: vec![
                f("Enclosure", "Recommended"),
                f("Drying", "Essential — nylon absorbs water fast; dry (≈70–80 °C, many hours) and print from a dry box"),
                f("Bed surface", "Glue stick on PEI or a garolite sheet"),
                f("Retraction", "Moderate; wet nylon strings and bubbles"),
                f("Use", "Tough, wear-resistant hinges, clips and gears"),
            ],
            notes: vec![],
        },
        MaterialCard {
            id: "PC",
            name: "PC (polycarbonate)",
            ranges: vec![
                r("Nozzle", 260.0, 300.0, "°C", NOZ, "check your hotend's maximum temperature"),
                r("Bed", 100.0, 120.0, "°C", BED, ""),
                r("Part-cooling fan", 0.0, 20.0, "%", FAN, ""),
                r("Typical shrink", 0.5, 0.8, "%", SHR, ""),
            ],
            facts: vec![
                f("Enclosure", "Required (hot chamber helps against layer splitting)"),
                f("Drying", "Required: ≈70–80 °C for several hours"),
                f("Bed surface", "Textured PEI with glue stick as release agent, or a dedicated sheet"),
                f("Use", "High heat resistance and toughness; the molded Iris frame is PC"),
            ],
            notes: vec![],
        },
        MaterialCard {
            id: "CF",
            name: "CF-filled PA / PETG",
            ranges: vec![
                r("Nozzle", 240.0, 290.0, "°C", NOZ, "base polymer's range, often +5–10 °C"),
                r("Bed", 60.0, 100.0, "°C", BED, "follow the base polymer"),
                r("Part-cooling fan", 0.0, 40.0, "%", FAN, ""),
                r("Typical shrink", 0.2, 0.6, "%", SHR, "fibres reduce shrink and warp"),
            ],
            facts: vec![
                f("Nozzle", "Hardened steel (or other wear-resistant) nozzle required — carbon fibre is abrasive; 0.4 mm minimum, 0.6 mm reduces clogs"),
                f("Drying", "As for the base polymer (dry PA-CF thoroughly)"),
                f("Result", "Stiff, matte, dimensionally stable; lower impact strength and weaker between layers"),
                f("Safety", "Sanding produces fine fibre dust — wear a mask"),
            ],
            notes: vec![],
        },
    ]
}

fn fdm(p: &Project) -> Process {
    let pr = &p.design.print;
    let lw = pr.line_width_mm.max(0.1);
    let wall = p.design.frame.wall_mm;
    let flow = pr.layer_height_mm * lw * pr.print_speed_mm_s;
    let mut checks = Vec::new();
    let mat_id = normalise_fdm(&p.design.frame.material);
    if let Some(m) = fdm_materials().into_iter().find(|m| m.id == mat_id) {
        for (label, v) in [("Nozzle", pr.nozzle_temp_c), ("Bed", pr.bed_temp_c)] {
            if let Some(rg) = m.ranges.iter().find(|x| x.label == label) {
                let ok = v >= rg.lo - 1e-9 && v <= rg.hi + 1e-9;
                checks.push(ProjectCheck {
                    label: format!("{label} temperature ({} frame, Print lab)", m.name),
                    value: v,
                    lo: rg.lo,
                    hi: rg.hi,
                    unit: "°C",
                    status: if ok { "pass" } else { "warn" },
                    note: if ok {
                        "inside the typical window".into()
                    } else {
                        "outside the typical window — deliberate?".into()
                    },
                });
            }
        }
    }
    let lo = 2.0 * lw;
    let hi = 3.0 * lw;
    checks.push(ProjectCheck {
        label: "Frame wall vs 2 line widths (CAD Studio)".into(),
        value: wall,
        lo,
        hi: f64::INFINITY,
        unit: "mm",
        status: if wall >= lo - 1e-9 { "pass" } else { "warn" },
        note: format!("{wall:.2} mm wall = {:.1} line widths of {lw:.2} mm", wall / lw),
    });
    let lh_ratio = pr.layer_height_mm / pr.nozzle_mm.max(0.1);
    checks.push(ProjectCheck {
        label: "Layer height / nozzle (Print lab)".into(),
        value: lh_ratio * 100.0,
        lo: 25.0,
        hi: 75.0,
        unit: "%",
        status: if (0.25..=0.75).contains(&lh_ratio) { "pass" } else { "warn" },
        note: format!("{:.2} mm layers on a {:.2} mm nozzle", pr.layer_height_mm, pr.nozzle_mm),
    });
    Process {
        id: "fdm",
        name: "FDM 3D printing",
        group: "Additive",
        summary: "Fused deposition modelling (FDM, also called fused filament fabrication): a heated nozzle lays down lines of molten plastic layer by layer. The workhorse for Iris prototype frames on a hobby printer.",
        caveat: CAVEAT,
        anim: "fdm",
        materials: fdm_materials(),
        rules: vec![
            rule("Minimum wall", format!("2–3 line widths → {lo:.2}–{hi:.2} mm for your {lw:.2} mm lines"), "A wall thinner than two lines prints as a single, weak line (or gaps). Size walls as whole multiples of the line width.", "wall"),
            rule("Overhangs", "≤ 45° from vertical without supports", "Each layer must be mostly supported by the one below; steeper overhangs droop. Chamfer the underside of holes and ledges instead of adding supports.", "overhang"),
            rule("Bridging", "Keep unsupported bridges short (test your printer)", "Bridges stretch a line between two supports with maximum fan. Short spans print cleanly; long ones sag — measure with a bridging test.", "bridge"),
            rule("Hole compensation", "Holes print ≈0.1–0.3 mm undersize", "Polygon faceting and melt flow shrink holes. Add clearance in CAD, use the slicer's hole compensation, or drill / ream to size. Horizontal holes print best as teardrops.", "hole"),
            rule("Orientation for strength", "Z (between layers) is the weakest direction", "Layer bonds are weaker than the extruded lines. Orient parts so the main load runs along the layers — print Iris temples flat, not standing up.", "orient"),
            rule("Layer height", format!("25–75 % of the nozzle → {:.2}–{:.2} mm", 0.25 * pr.nozzle_mm, 0.75 * pr.nozzle_mm), "Thinner layers show less stair-stepping on curves but take longer.", "layer"),
            rule("Volumetric flow", format!("{:.2} × {lw:.2} × {:.0} = {flow:.1} mm³/s", pr.layer_height_mm, pr.print_speed_mm_s), "Flow = layer height × line width × speed. It must stay under your hotend's melting capacity (check your printer's documentation); faster printing needs a higher-flow hotend or higher temperature.", "flow"),
            rule("Fits and clearances", "≈0.2–0.3 mm clearance per side for sliding fits", "Print a clearance test once for your machine and material, then design with the measured value.", "fit"),
            rule("Elephant's foot", "Chamfer bottom edges ≈0.3–0.5 mm", "The squished first layer bulges outwards; a small chamfer (or the slicer's compensation) keeps the base dimensionally true.", "efoot"),
        ],
        failures: vec![
            Failure { name: "Stringing", symptom: "Fine hairs between separate parts of the print", causes: &["Nozzle too hot", "Too little retraction", "Wet filament (PETG, nylon, TPU)"], fixes: &["Lower nozzle temperature 5 °C at a time", "Tune retraction distance / speed", "Dry the filament", "Reduce travel moves (avoid crossing perimeters)"], diagram: "stringing" },
            Failure { name: "Warping", symptom: "Corners lift off the bed", causes: &["Material shrinks as it cools (ABS, ASA, nylon, PC)", "Drafts / no enclosure", "Poor first-layer adhesion"], fixes: &["Use an enclosure for ABS/ASA/PC/nylon", "Clean the bed (isopropyl alcohol) and use the right surface", "Add a brim or mouse ears", "Round sharp corners in CAD"], diagram: "warping" },
            Failure { name: "Elephant's foot", symptom: "Bottom layers bulge outwards", causes: &["First layer squished too much", "Bed too hot for the material"], fixes: &["Raise Z-offset slightly", "Enable elephant's-foot compensation", "Chamfer the bottom edge in CAD"], diagram: "efoot" },
            Failure { name: "Layer separation", symptom: "Parts crack or split between layers", causes: &["Nozzle too cool", "Too much part cooling", "Drafts on ABS / PC"], fixes: &["Raise nozzle temperature", "Reduce fan speed", "Enclose the printer", "Increase line width or perimeters"], diagram: "delam" },
            Failure { name: "Under-extrusion", symptom: "Gaps between lines, thin walls, weak parts", causes: &["Partial clog or heat creep", "Flow exceeds hotend capacity", "Wet or tangled filament"], fixes: &["Slow down or raise temperature", "Clean / replace the nozzle", "Dry the filament", "Calibrate flow"], diagram: "underx" },
            Failure { name: "Poor bed adhesion", symptom: "First layer does not stick or the print detaches", causes: &["Dirty or oily bed", "Wrong Z-offset", "Wrong bed temperature or surface"], fixes: &["Wash the plate with dish soap and water or wipe with isopropyl alcohol", "Re-level / set Z-offset", "Use the recommended bed temperature and surface"], diagram: "adhesion" },
            Failure { name: "Ringing / ghosting", symptom: "Ripples echoing sharp features on walls", causes: &["Vibration at high acceleration", "Loose belts or frame"], fixes: &["Run input-shaping / vibration calibration if your printer supports it", "Lower acceleration for outer walls", "Tension belts"], diagram: "ringing" },
        ],
        tips: vec![
            "Bambu Lab: select the matching Bambu Studio filament profile (system profile for Bambu filaments, or a generic profile for other brands) as the starting point — generic profiles are starting points, not tuned values.".into(),
            "Bambu Lab: many AMS setups cannot feed TPU 95A or other flexible filaments — use the external spool holder; check the Bambu Lab Wiki for your AMS model.".into(),
            "Bambu Lab: abrasive filaments (carbon- or glass-fibre filled) need a hardened steel nozzle and can wear the feed path — check the wiki before loading them in the AMS.".into(),
            "Prusa Knowledge Base and Bambu Lab Wiki both keep per-material guides and drying tables — use them together with the filament maker's datasheet.".into(),
            format!("Your project prints {} frames with a {:.2} mm nozzle at {:.2} mm layers — see the Print lab for the slice, time and G-code.", p.design.frame.material, pr.nozzle_mm, pr.layer_height_mm),
        ],
        links: vec![LabLink { label: "Print Lab — slice, time, G-code", route: "/print" }, LabLink { label: "CAD Studio — STL export", route: "/cad" }, LabLink { label: "Structures Lab — strength", route: "/structures" }],
        refs: vec![
            "Prusa Knowledge Base — materials guide (help.prusa3d.com)",
            "Bambu Lab Wiki — filament guides (wiki.bambulab.com)",
            "I. Gibson, D. Rosen, B. Stucker, Additive Manufacturing Technologies, Springer (3rd ed., 2021)",
            "ISO/ASTM 52900 — Additive manufacturing: general principles and vocabulary",
        ],
        project_checks: checks,
    }
}

/// Map a free-form material name to an FDM card id.
pub fn normalise_fdm(s: &str) -> &'static str {
    let u = s.to_ascii_uppercase();
    if u.contains("CF") || u.contains("CARBON") {
        "CF"
    } else if u.contains("TPU") {
        "TPU95A"
    } else if u.contains("PETG") {
        "PETG"
    } else if u.contains("ASA") {
        "ASA"
    } else if u.contains("ABS") {
        "ABS"
    } else if u.contains("PA") || u.contains("NYLON") {
        "PA"
    } else if u.contains("PC") {
        "PC"
    } else {
        "PLA"
    }
}

// ---------------------------------------------------------------------------
// Resin, powder bed
// ---------------------------------------------------------------------------

fn resin() -> Process {
    Process {
        id: "resin",
        name: "Resin printing (SLA / MSLA)",
        group: "Additive",
        summary: "Stereolithography (SLA) and masked stereolithography (MSLA): ultraviolet light cures liquid photopolymer layer by layer, the part hanging upside down from the build plate. Fine detail and smooth surfaces — good for optics mounts and cosmetic models.",
        caveat: CAVEAT,
        anim: "resin",
        materials: vec![
            MaterialCard {
                id: "standard",
                name: "Standard resin",
                ranges: vec![r("Layer height", 0.025, 0.1, "mm", [0.0, 0.15], "0.05 mm is the common default")],
                facts: vec![
                    f("Exposure", "Set by the resin maker per printer — confirm with an exposure calibration print"),
                    f("Bottom layers", "A few layers with much longer exposure so the part sticks to the plate"),
                    f("Wash", "Isopropyl alcohol (or water for water-washable resin), two-stage, a few minutes"),
                    f("Post-cure", "UV (405 nm) after the part is fully dry; follow the datasheet — over-curing makes parts brittle"),
                ],
                notes: vec!["Brittle; for looks-like models and fixtures."],
            },
            MaterialCard {
                id: "tough",
                name: "Tough / ABS-like resin",
                ranges: vec![r("Layer height", 0.05, 0.1, "mm", [0.0, 0.15], "")],
                facts: vec![f("Use", "Snap fits and functional prototypes"), f("Exposure", "Usually longer than standard resin — calibrate")],
                notes: vec![],
            },
            MaterialCard {
                id: "clear",
                name: "Clear resin",
                ranges: vec![r("Layer height", 0.025, 0.05, "mm", [0.0, 0.15], "thin layers for clarity")],
                facts: vec![f("Finish", "Needs sanding + clear coat for transparency; yellows with over-curing"), f("Use", "Light pipes, optics prototypes (not optical-grade)")],
                notes: vec![],
            },
        ],
        rules: vec![
            rule("Drain holes", "≥ 2–3 mm, at least two per hollow", "Hollow parts trap liquid resin. One hole near the plate lets resin drain, a second one far from it lets air in.", "drain"),
            rule("Avoid cupping", "Open hollow shells towards the vat or vent them", "A closed cup facing the film acts like a suction cup: every peel pulls a vacuum, causing failures and blow-outs.", "cupping"),
            rule("Tilt and support", "Tilt ≈20–45°, supports on non-cosmetic faces", "Tilting reduces the cross-section of each layer (less peel force) and avoids large flat areas parallel to the film.", "tilt"),
            rule("Hollow wall thickness", "≈1.5–2.5 mm", "Thinner shells crack during peel or post-cure.", "wall"),
            rule("Tolerance", "≈±0.05–0.1 mm typical (small parts)", "Resin parts shrink slightly while curing; calibrate XY compensation with a test part.", "fit"),
            rule("Smallest features", "Pins ≥ ≈0.3–0.5 mm, embossed detail ≥ ≈0.1–0.2 mm", "Below this, features snap off during washing or are lost to light bleed.", "feature"),
        ],
        failures: vec![
            Failure { name: "Part stuck to film / detached", symptom: "Cured blob on the film, nothing on the plate", causes: &["Too few or too short bottom exposures", "Plate not levelled", "Large cross-section"], fixes: &["Increase bottom exposure", "Re-level the plate", "Tilt the part"], diagram: "adhesion" },
            Failure { name: "Layer lines / delamination", symptom: "Layers split or show bands", causes: &["Peel forces too high", "Under-exposure"], fixes: &["Slow the lift", "Increase exposure slightly", "Reduce cross-section per layer"], diagram: "delam" },
            Failure { name: "Blow-out", symptom: "Holes or craters in hollow parts", causes: &["Cupping — trapped suction"], fixes: &["Add vent / drain holes", "Re-orient the part"], diagram: "cupping" },
        ],
        tips: vec![
            "Uncured resin is a skin sensitiser: nitrile gloves, eye protection and ventilation; cure waste IPA and resin before disposal.".into(),
            "Resin models are good for Iris optics mounts and looks-like frames; for functional frames prefer MJF / SLS nylon or tough resin.".into(),
        ],
        links: vec![LabLink { label: "CAD Studio — STL export", route: "/cad" }, LabLink { label: "Optics Lab", route: "/optics" }],
        refs: vec!["I. Gibson, D. Rosen, B. Stucker, Additive Manufacturing Technologies (vat photopolymerisation chapter)", "ISO/ASTM 52900 — Additive manufacturing vocabulary"],
        project_checks: vec![],
    }
}

fn powder() -> Process {
    Process {
        id: "powder",
        name: "Powder-bed fusion (SLS / MJF) via service",
        group: "Additive",
        summary: "Selective laser sintering (SLS) and Multi Jet Fusion (MJF) fuse nylon powder layer by layer; the surrounding powder supports the part, so no support structures are needed. Usually ordered from a service bureau — the best route to strong EVT/DVT Iris frames.",
        caveat: "Typical design rules — every bureau publishes its own limits; check its design guide before ordering.",
        anim: "powder",
        materials: vec![
            MaterialCard {
                id: "PA12",
                name: "PA12 (nylon 12)",
                ranges: vec![r("Minimum wall", 0.8, 1.0, "mm", [0.0, 3.0], "thicker for large flat walls"), r("Tolerance", 0.2, 0.3, "± mm", [0.0, 1.0], "or ≈±0.3 % on large parts")],
                facts: vec![f("Finish", "Slightly grainy; bead-blasted, often dyed black"), f("Use", "Functional housings, clips, eyewear prototypes")],
                notes: vec![],
            },
            MaterialCard {
                id: "PA11",
                name: "PA11 (nylon 11)",
                ranges: vec![r("Minimum wall", 0.8, 1.0, "mm", [0.0, 3.0], "")],
                facts: vec![f("Use", "More ductile than PA12 — living hinges, snap fits, impact")],
                notes: vec![],
            },
            MaterialCard {
                id: "TPU",
                name: "TPU (flexible)",
                ranges: vec![r("Minimum wall", 1.0, 1.5, "mm", [0.0, 3.0], "")],
                facts: vec![f("Use", "Nose pads, gaskets, flexible lattices")],
                notes: vec![],
            },
        ],
        rules: vec![
            rule("Minimum wall", "≈0.8–1 mm typical", "Thinner walls may not survive de-powdering or may warp.", "wall"),
            rule("Escape holes", "≥ ≈5 mm, at least two, for hollow parts", "Trapped unfused powder must be blown out; two holes let air flow through.", "drain"),
            rule("Clearance between parts", "≈0.5 mm or more (print-in-place)", "Heat bleeds into neighbouring powder and can fuse close surfaces together.", "fit"),
            rule("Large flat areas", "Add ribs or curvature", "Long flat panels warp as they cool in the bed.", "warping"),
            rule("No supports", "Overhangs are free", "The powder cake supports everything — design freedom is the main advantage.", "overhang"),
        ],
        failures: vec![
            Failure { name: "Warped flat panels", symptom: "Bowed plates", causes: &["Uneven cooling in the bed"], fixes: &["Add ribs", "Thicken walls uniformly", "Ask the bureau about orientation"], diagram: "warping" },
            Failure { name: "Trapped powder", symptom: "Heavy part, rattles", causes: &["Closed hollow"], fixes: &["Add two escape holes", "Make the part solid if thin"], diagram: "drain" },
        ],
        tips: vec!["Upload STEP or STL from the CAD Studio; most bureaus run an automatic wall-thickness check on upload.".into()],
        links: vec![LabLink { label: "CAD Studio — STL export", route: "/cad" }, LabLink { label: "Manufacturing Hub — cost vs quantity", route: "/mfg" }],
        refs: vec!["I. Gibson, D. Rosen, B. Stucker, Additive Manufacturing Technologies (powder bed fusion chapter)", "ISO/ASTM 52900"],
        project_checks: vec![],
    }
}

// ---------------------------------------------------------------------------
// Laser cutting
// ---------------------------------------------------------------------------

/// Laser-cutting material safety entry.
#[derive(Clone, Debug, Serialize)]
pub struct LaserMaterial {
    pub name: &'static str,
    /// "safe" | "caution" | "never"
    pub verdict: &'static str,
    pub why: &'static str,
}

pub const LASER_MATERIALS: &[LaserMaterial] = &[
    LaserMaterial {
        name: "Cast / extruded acrylic (PMMA)",
        verdict: "safe",
        why: "Cuts with a flame-polished edge; cast acrylic engraves frosty white.",
    },
    LaserMaterial {
        name: "Plywood, solid wood",
        verdict: "safe",
        why: "Cuts well; glue lines in plywood vary. Ventilate the smoke.",
    },
    LaserMaterial {
        name: "MDF",
        verdict: "caution",
        why: "Cuts cleanly but the resin binder smokes heavily — good extraction needed.",
    },
    LaserMaterial {
        name: "Paper, card, cardboard",
        verdict: "safe",
        why: "Low power; watch for fire, never leave the machine.",
    },
    LaserMaterial {
        name: "Natural fabric, felt, cork",
        verdict: "safe",
        why: "Cuts easily; felt edges seal.",
    },
    LaserMaterial {
        name: "Vegetable-tanned leather",
        verdict: "safe",
        why: "Cuts and engraves well; smells. Avoid chrome-tanned leather.",
    },
    LaserMaterial {
        name: "Acetal / POM (Delrin)",
        verdict: "caution",
        why: "Cuts, but releases formaldehyde — only with strong extraction.",
    },
    LaserMaterial {
        name: "Anodised aluminium",
        verdict: "caution",
        why: "A CO₂ laser only removes the dye layer (engraving); it does not cut metal.",
    },
    LaserMaterial {
        name: "PVC, vinyl, vinyl-coated fabric",
        verdict: "never",
        why: "Releases chlorine / hydrogen chloride gas — toxic and corrodes the machine.",
    },
    LaserMaterial {
        name: "Polycarbonate (Lexan)",
        verdict: "never",
        why: "Burns and discolours instead of cutting; poor edge, smoky.",
    },
    LaserMaterial {
        name: "ABS",
        verdict: "never",
        why: "Melts into a gooey edge and can release hydrogen cyanide.",
    },
    LaserMaterial {
        name: "HDPE / polyethylene",
        verdict: "never",
        why: "Melts and catches fire.",
    },
    LaserMaterial {
        name: "PTFE (Teflon)",
        verdict: "never",
        why: "Releases fluorine compounds.",
    },
    LaserMaterial {
        name: "Fibreglass, FR-4, coated carbon fibre",
        verdict: "never",
        why: "Glass does not cut; epoxy resin smoke is noxious.",
    },
    LaserMaterial {
        name: "Polystyrene / foam",
        verdict: "never",
        why: "Catches fire readily.",
    },
];

fn laser() -> Process {
    Process {
        id: "laser",
        name: "Laser cutting & engraving",
        group: "Subtractive",
        summary: "A focused beam vaporises a narrow path (the kerf). A CO₂ laser cuts wood and acrylic; a fibre laser cuts metal (usually at a service); a diode laser engraves and cuts thin material. Iris uses it for acrylic test fixtures, packaging inserts and flat jigs.",
        caveat: "Power and speed are given as relative guidance only: they differ between machines, lenses and tube ages. Cut the test grid below on your machine and read off your own settings.",
        anim: "laser",
        materials: vec![
            MaterialCard {
                id: "acrylic",
                name: "Acrylic (PMMA)",
                ranges: vec![r("Kerf (CO₂)", 0.1, 0.25, "mm", [0.0, 0.5], "widens with thickness"), r("Relative power", 60.0, 100.0, "% of max", [0.0, 100.0], "slow + high power for thick sheet")],
                facts: vec![f("Air assist", "Low for cast acrylic cuts (polished edge); on for engraving"), f("Focus", "At the surface or slightly into the material for thick sheet")],
                notes: vec![],
            },
            MaterialCard {
                id: "plywood",
                name: "Plywood / MDF",
                ranges: vec![r("Kerf (CO₂)", 0.15, 0.3, "mm", [0.0, 0.5], ""), r("Relative power", 60.0, 100.0, "% of max", [0.0, 100.0], "")],
                facts: vec![f("Air assist", "On — reduces charring and flare-ups"), f("Masking", "Masking tape reduces scorch marks on the top face")],
                notes: vec![],
            },
            MaterialCard {
                id: "engrave",
                name: "Engraving (any safe material)",
                ranges: vec![r("Relative power", 10.0, 50.0, "% of max", [0.0, 100.0], "fast speed, low power"), r("Line interval", 0.05, 0.15, "mm", [0.0, 0.3], "")],
                facts: vec![f("Test", "Engrave a small grid first — grey levels depend on material")],
                notes: vec![],
            },
        ],
        rules: vec![
            rule("Kerf compensation", "Offset the path by kerf / 2", "The beam removes a strip of material: outside cuts must move out and holes in by half the kerf, or press fits come out loose.", "kerf"),
            rule("Minimum feature", "≳ material thickness for slots and bridges", "Thin bridges burn away or warp from heat.", "feature"),
            rule("Hole diameter", "≳ material thickness", "Small holes in thick sheet taper and may not cut through.", "hole"),
            rule("Measuring the kerf", "Cut a 50 mm strip into 10 slices, push them together", "Each slice loses one kerf width, so kerf = (50 − measured length) / 10.", "kerf"),
        ],
        failures: vec![
            Failure { name: "Does not cut through", symptom: "Back face still attached", causes: &["Speed too high / power too low", "Out of focus", "Dirty lens or mirror"], fixes: &["Slow down", "Re-focus", "Clean the optics"], diagram: "kerf" },
            Failure { name: "Charring / flare-ups", symptom: "Black edges, flames", causes: &["No air assist", "Too slow"], fixes: &["Air assist on", "Faster with more passes", "Never leave the machine running unattended"], diagram: "kerf" },
        ],
        tips: vec![
            "Never cut an unknown plastic: if you cannot identify it, do not put it in the laser.".into(),
            "The test-grid generator below makes an SVG of power × speed squares; open it in your laser software (for example LightBurn), map the rows and columns to the listed settings, and cut.".into(),
        ],
        links: vec![LabLink { label: "CAD Studio — outlines", route: "/cad" }],
        refs: vec!["Machine maker's manual and safety guide", "University / makerspace laser safety material lists (the PVC, polycarbonate and ABS warnings are universal)"],
        project_checks: vec![],
    }
}

// ---------------------------------------------------------------------------
// CNC, sheet metal (numbers live in machining.rs / sheet.rs)
// ---------------------------------------------------------------------------

fn cnc() -> Process {
    let mats = crate::build::machining::MATERIALS
        .iter()
        .map(|m| MaterialCard {
            id: m.id,
            name: m.name,
            ranges: vec![
                r(
                    "Surface speed (carbide)",
                    m.sfm_carbide[0],
                    m.sfm_carbide[1],
                    "SFM",
                    [0.0, 1600.0],
                    "",
                ),
                r(
                    "Chip load, 6 mm / ¼ in tool",
                    m.chip_mm_6[0],
                    m.chip_mm_6[1],
                    "mm/tooth",
                    [0.0, 0.3],
                    "",
                ),
                r(
                    "Specific cutting energy",
                    m.unit_power[0],
                    m.unit_power[1],
                    "W·s/mm³",
                    [0.0, 10.0],
                    "",
                ),
            ],
            facts: vec![f("Flutes", m.flutes), f("Coolant", m.coolant)],
            notes: vec![m.notes],
        })
        .collect();
    Process {
        id: "cnc",
        name: "CNC milling",
        group: "Subtractive",
        summary: "A spinning end mill removes material along a tool path. Tight tolerances and real engineering materials — aluminium hinge blocks, optics mounts, molding inserts. The feeds-and-speeds calculator turns these ranges into spindle speed, feed and power.",
        caveat: "Typical starting points for carbide end mills — tool makers publish tool-specific charts; start conservative and listen to the cut.",
        anim: "cnc",
        materials: mats,
        rules: vec![
            rule("Internal corners", "Radius ≥ tool radius (+≈10 %)", "A round tool cannot cut a sharp inside corner. A slightly larger radius than the tool avoids chatter in the corner.", "corner"),
            rule("Pocket depth", "≤ ≈3–4 × tool diameter", "Long tools deflect and chatter; deeper pockets need special tooling and cost more.", "pocket"),
            rule("Thin walls", "≳ 0.8 mm metal, ≳ 1.5 mm plastic", "Thin walls vibrate and deflect under cutting forces.", "wall"),
            rule("Holes and threads", "Standard drill sizes; thread depth ≈1.5–2 × D", "Standard sizes avoid custom tools; deeper threads add little strength.", "hole"),
            rule("Tolerances", "≈±0.1 mm general; tighter only where needed", "General tolerances (e.g. ISO 2768-m) are cheap; every tight tolerance on the drawing adds inspection and cost.", "fit"),
            rule("Setups", "Minimise the number of fixturing orientations", "Each re-clamp adds time and error; design features reachable from one or two sides.", "orient"),
        ],
        failures: vec![
            Failure { name: "Chatter", symptom: "Screeching noise, wavy surface", causes: &["Tool stick-out too long", "Speed in an unstable range", "Too light a chip (rubbing)"], fixes: &["Shorten the tool", "Change RPM ±10–20 %", "Increase chip load"], diagram: "ringing" },
            Failure { name: "Built-up edge / welding", symptom: "Aluminium sticks to the flutes", causes: &["No coolant / lubrication", "Chips re-cut"], fixes: &["Mist or flood coolant, air blast", "Fewer flutes, polished tools", "Clear chips"], diagram: "chip" },
            Failure { name: "Broken tool", symptom: "Snap", causes: &["Chip load too high for the tool size", "Full slotting too deep"], fixes: &["Use the calculator", "Adaptive / trochoidal toolpaths with low radial engagement"], diagram: "chip" },
        ],
        tips: vec!["For small parts order from a CNC service: upload STEP plus a 2D drawing (PDF) with tolerances and threads marked.".into()],
        links: vec![LabLink { label: "CAD Studio — geometry", route: "/cad" }, LabLink { label: "Structures Lab — stress", route: "/structures" }],
        refs: vec![
            "E. Oberg et al., Machinery's Handbook, Industrial Press (speeds, feeds and machining power sections)",
            "S. Kalpakjian, S. R. Schmid, Manufacturing Engineering and Technology, Pearson (specific energy in cutting)",
            "Tool makers' published speed and feed charts (Sandvik Coromant, Harvey Tool and others)",
        ],
        project_checks: vec![],
    }
}

fn sheet() -> Process {
    let mats = crate::build::sheet::SHEET_MATERIALS
        .iter()
        .map(|m| MaterialCard {
            id: m.id,
            name: m.name,
            ranges: vec![r(
                "Minimum inside bend radius",
                m.min_r_over_t[0],
                m.min_r_over_t[1],
                "× T",
                [0.0, 5.0],
                "",
            )],
            facts: vec![f("Notes", m.notes)],
            notes: vec![],
        })
        .collect();
    Process {
        id: "sheet",
        name: "Sheet metal",
        group: "Forming",
        summary: "Flat sheet is laser- or punch-cut, then bent on a press brake. Brackets, shields and enclosures. The bend calculator gives the flat-pattern length from the K-factor.",
        caveat: "Typical rules — shops publish their own minimum flange, radius and hole-to-bend distances tied to their tooling.",
        anim: "sheet",
        materials: mats,
        rules: vec![
            rule("Inside bend radius", "≈1 × T for most ductile sheet", "Too tight a radius cracks the outside of the bend, especially in hard tempers (6061-T6).", "bend"),
            rule("Minimum flange length", "≳ ≈4 × T (depends on the shop's V-die)", "The flange must span the die opening while it is bent.", "flange"),
            rule("Holes near bends", "≳ ≈2.5 × T + R from the bend", "Holes closer than this distort into ovals.", "hole"),
            rule("Bend relief", "Width ≥ T, depth ≥ R + T", "Relief cuts at the end of a bend line prevent tearing.", "relief"),
            rule("Grain direction", "Bend across the rolling direction where possible", "Bending along the grain makes cracking more likely.", "orient"),
        ],
        failures: vec![Failure { name: "Cracked bend", symptom: "Fissures on the outside of the bend", causes: &["Radius too small for the temper", "Bending along the grain"], fixes: &["Larger radius", "Softer temper (5052-H32 instead of 6061-T6)", "Rotate the part on the sheet"], diagram: "bend" }],
        tips: vec!["Upload a DXF of the flat pattern, or a STEP of the bent part — many sheet services compute the flat pattern themselves.".into()],
        links: vec![LabLink { label: "CAD Studio", route: "/cad" }],
        refs: vec!["E. Oberg et al., Machinery's Handbook (sheet-metal bending)", "DIN 6935 — cold bending of flat steel products (K-factor correction)"],
        project_checks: vec![],
    }
}

// ---------------------------------------------------------------------------
// Electronics assembly
// ---------------------------------------------------------------------------

fn electronics(p: &Project) -> Process {
    let alloys = [crate::build::reflow::Alloy::sac305(), crate::build::reflow::Alloy::snpb()];
    let mats = alloys
        .iter()
        .map(|a| MaterialCard {
            id: a.id,
            name: a.name,
            ranges: vec![
                r("Preheat ramp", 0.5, a.max_ramp_c_s, "°C/s", [0.0, 6.0], ""),
                r("Soak window", a.soak_lo_c, a.soak_hi_c, "°C", [0.0, 280.0], ""),
                r("Soak time", a.soak_s[0], a.soak_s[1], "s", [0.0, 200.0], ""),
                r("Time above liquidus", a.tal_s[0], a.tal_s[1], "s", [0.0, 200.0], ""),
                r("Peak", a.peak_c[0], a.peak_c[1], "°C", [150.0, 280.0], ""),
                r("Cooling rate", 1.0, a.max_cool_c_s, "°C/s", [0.0, 8.0], ""),
                r(
                    "Hand-soldering tip",
                    a.iron_c[0],
                    a.iron_c[1],
                    "°C",
                    [200.0, 450.0],
                    "lowest temperature that wets quickly",
                ),
            ],
            facts: vec![f("Liquidus", format!("{:.0} °C", a.liquidus_c)), f("Profile", a.notes)],
            notes: vec![],
        })
        .collect();
    let fine = crate::build::pcbcap::min_pitch_mm(p);
    Process {
        id: "electronics",
        name: "Electronics assembly",
        group: "Electronics",
        summary: "Solder paste is printed through a stencil, parts are placed, and the board passes through a reflow oven. The reflow calculator simulates the board's temperature through each oven zone.",
        caveat: "Typical windows consistent with IPC/JEDEC J-STD-020 and solder-paste datasheets — your paste's datasheet profile always wins.",
        anim: "reflow",
        materials: mats,
        rules: vec![
            rule("Stencil area ratio", "Aperture area / wall area ≥ 0.66", "Paste releases from the stencil only if the pad opening is large compared with its walls (IPC-7525). The stencil calculator checks every Iris pad.", "stencil"),
            rule("Stencil aspect ratio", "Aperture width / thickness ≥ 1.5", "The simpler check for long, narrow apertures.", "stencil"),
            rule(
                "Stencil thickness",
                match fine {
                    Some(pt) if pt <= 0.45 => format!("≈0.10–0.12 mm (finest pitch on this board: {pt:.2} mm)"),
                    Some(pt) => format!("≈0.12–0.15 mm (finest pitch on this board: {pt:.2} mm)"),
                    None => "≈0.10–0.15 mm".to_string(),
                },
                "Fine-pitch parts (0.4 mm) and 0201 passives want thinner stencils; coarse parts tolerate thicker ones.",
                "stencil",
            ),
            rule("Thermal pads", "Window the paste (≈50–80 % coverage)", "A full paste layer under an exposed pad floats the part and causes voids.", "stencil"),
            rule("Conformal coating", "Mask connectors, test points and buttons", "Acrylic (easy rework), silicone (flexible, heat), urethane (chemicals), parylene (thin, vapour deposited). Typical thickness ranges are commonly quoted from IPC-A-610; follow the coating datasheet.", "coat"),
            rule("Cleanliness and ESD", "No-clean flux, ESD-safe handling", "Residues of no-clean flux are designed to stay; wash only if the paste datasheet allows. Ground yourself when handling boards.", "coat"),
        ],
        failures: vec![
            Failure { name: "Tombstoning", symptom: "A small passive stands on one end", causes: &["Uneven heating of the two pads", "Unequal pad sizes or paste"], fixes: &["Symmetric pads and traces", "Longer soak to equalise temperature"], diagram: "tombstone" },
            Failure { name: "Solder bridges", symptom: "Shorts between fine-pitch pins", causes: &["Too much paste", "Poor stencil release"], fixes: &["Thinner stencil or reduced apertures", "Check area ratio"], diagram: "bridge" },
            Failure { name: "Cold / grainy joints", symptom: "Dull, poorly wetted fillets", causes: &["Peak too low or time above liquidus too short"], fixes: &["Raise peak or slow the conveyor", "Check the thermocouple on the heaviest part"], diagram: "cold" },
            Failure { name: "Voids / solder balls", symptom: "Voids under pads, stray balls", causes: &["Ramp too fast (flux boils)", "Old or wet paste"], fixes: &["Slower preheat, proper soak", "Fresh paste at room temperature"], diagram: "cold" },
        ],
        tips: vec![
            "At EVT, order assembled boards (PCBA) from the fab — the PCB Studio exports Gerbers, BOM and CPL. Hand-solder or hot-plate reflow only for rework and quick tests.".into(),
            "Measure a real profile with a thermocouple on the heaviest part (the QFN SoC on Iris) — the model here is a lumped estimate.".into(),
        ],
        links: vec![LabLink { label: "PCB Studio — Gerbers, BOM, CPL", route: "/pcb" }, LabLink { label: "Robotics Lab — pick-and-place", route: "/robotics" }],
        refs: vec![
            "IPC/JEDEC J-STD-020 — Moisture/reflow sensitivity classification (classification reflow profiles)",
            "IPC-7525 — Stencil design guidelines",
            "IPC-A-610 — Acceptability of electronic assemblies",
            "Solder-paste makers' datasheet profiles",
        ],
        project_checks: vec![],
    }
}

// ---------------------------------------------------------------------------
// Injection molding process windows
// ---------------------------------------------------------------------------

/// Process window per resin; ids match `crate::molding::materials`.
#[derive(Clone, Debug, Serialize)]
pub struct MoldWindow {
    pub id: &'static str,
    pub melt_c: [f64; 2],
    pub mold_c: [f64; 2],
    pub dry: &'static str,
    pub shrink_pct: [f64; 2],
}

pub const MOLD_WINDOWS: &[MoldWindow] = &[
    MoldWindow {
        id: "PC",
        melt_c: [280.0, 320.0],
        mold_c: [80.0, 110.0],
        dry: "Required: ≈120 °C, 3–4 h (to very low moisture)",
        shrink_pct: [0.5, 0.7],
    },
    MoldWindow {
        id: "ABS",
        melt_c: [220.0, 260.0],
        mold_c: [40.0, 80.0],
        dry: "≈80 °C, 2–4 h",
        shrink_pct: [0.4, 0.7],
    },
    MoldWindow {
        id: "PC/ABS",
        melt_c: [250.0, 280.0],
        mold_c: [60.0, 90.0],
        dry: "≈100–110 °C, 3–4 h",
        shrink_pct: [0.5, 0.7],
    },
    MoldWindow {
        id: "PP",
        melt_c: [200.0, 260.0],
        mold_c: [20.0, 60.0],
        dry: "Usually not required",
        shrink_pct: [1.0, 2.5],
    },
    MoldWindow {
        id: "PA66",
        melt_c: [270.0, 300.0],
        mold_c: [60.0, 100.0],
        dry: "Required: ≈80 °C, several hours (keep sealed)",
        shrink_pct: [0.8, 2.0],
    },
    MoldWindow {
        id: "POM",
        melt_c: [190.0, 220.0],
        mold_c: [70.0, 110.0],
        dry: "Usually not required (surface moisture only)",
        shrink_pct: [1.8, 2.5],
    },
    MoldWindow {
        id: "TPU",
        melt_c: [190.0, 230.0],
        mold_c: [20.0, 50.0],
        dry: "Required: ≈80–100 °C, 2–3 h",
        shrink_pct: [0.5, 1.5],
    },
    MoldWindow {
        id: "PETG",
        melt_c: [230.0, 270.0],
        mold_c: [15.0, 40.0],
        dry: "Required: ≈65 °C, 4–6 h",
        shrink_pct: [0.2, 0.5],
    },
];

fn molding(p: &Project) -> Process {
    let mats = MOLD_WINDOWS
        .iter()
        .map(|w| {
            let m = crate::molding::materials::material(w.id);
            MaterialCard {
                id: w.id,
                name: m.name,
                ranges: vec![
                    r("Melt temperature", w.melt_c[0], w.melt_c[1], "°C", [150.0, 340.0], ""),
                    r("Mold temperature", w.mold_c[0], w.mold_c[1], "°C", [0.0, 130.0], ""),
                    r(
                        "Linear shrinkage",
                        w.shrink_pct[0],
                        w.shrink_pct[1],
                        "%",
                        [0.0, 3.0],
                        "grade and wall dependent",
                    ),
                ],
                facts: vec![
                    f("Drying", w.dry),
                    f(
                        "Molding Lab uses",
                        format!(
                            "melt {:.0} °C, mold {:.0} °C, shrink {:.2} %",
                            m.t_melt_c, m.t_mold_c, m.shrinkage_pct
                        ),
                    ),
                    f("Notes", m.notes),
                ],
                notes: vec![],
            }
        })
        .collect();
    let chosen = &p.design.manufacturing.molding_material;
    Process {
        id: "molding",
        name: "Injection molding (process window)",
        group: "Forming",
        summary: "Molten plastic is injected into a steel or aluminium mold, packed, cooled and ejected. This card gives the process window per resin; the Molding Lab does the part design checks (draft, walls, clamp force, cycle time).",
        caveat: "Typical windows — the resin grade's datasheet and the molder's process engineer set the real values.",
        anim: "molding",
        materials: mats,
        rules: vec![
            rule("Fill → pack → hold → cool", "Pack / hold pressure ≈ a fraction of injection pressure", "After the cavity fills, holding pressure packs more melt in to compensate shrinkage until the gate freezes; a gate-freeze study (part weight vs hold time) finds the minimum hold time.", "molding"),
            rule("Drying", "Dry hygroscopic resins before molding", "PC, PA, PETG and TPU hydrolyse or splay if molded wet.", "molding"),
            rule("Shrinkage in the tool", format!("Tool cut oversize by the resin's shrinkage ({chosen} for this project)"), "The toolmaker scales the cavity; changing resin later changes every dimension.", "fit"),
            rule("Design for molding", "Uniform walls, draft, no undercuts — see the Molding Lab", "Draft, wall thickness and gate location are checked on the actual Iris geometry there.", "overhang"),
        ],
        failures: vec![
            Failure { name: "Short shot", symptom: "Part not completely filled", causes: &["Melt too cold", "Too little pressure / shot size", "Flow length too long for the wall"], fixes: &["Raise melt or mold temperature", "Increase shot size and pressure", "Thicken walls or add gates"], diagram: "underx" },
            Failure { name: "Sink marks", symptom: "Dimples over thick sections", causes: &["Thick walls or ribs", "Too little hold pressure / time"], fixes: &["Core out thick sections", "Ribs ≤ ≈60 % of wall", "Longer hold"], diagram: "sink" },
            Failure { name: "Flash", symptom: "Thin film at the parting line", causes: &["Too much pressure", "Worn tool or low clamp force"], fixes: &["Reduce pressure", "Check clamp force (Molding Lab)"], diagram: "flash" },
            Failure { name: "Splay / silver streaks", symptom: "Streaks on the surface", causes: &["Wet resin"], fixes: &["Dry the resin to the datasheet's moisture level"], diagram: "splay" },
        ],
        tips: vec![],
        links: vec![LabLink { label: "Molding Lab — DFM on the Iris frame", route: "/molding" }, LabLink { label: "Manufacturing Hub — tooling cost", route: "/mfg" }],
        refs: vec!["D. V. Rosato, M. G. Rosato, Injection Molding Handbook", "T. A. Osswald, Understanding Polymer Processing (Hanser)", "Resin suppliers' generic processing guides"],
        project_checks: vec![],
    }
}

fn pcb() -> Process {
    Process {
        id: "pcb",
        name: "PCB fabrication capability",
        group: "Electronics",
        summary: "Board houses sell capability tiers: the finer the traces, drills and features, the higher the price and lead time. The DFM checker compares the Iris layout with each tier.",
        caveat: "Typical tier values — check your vendor's capabilities page; they change and differ by layer count and copper weight.",
        anim: "pcb",
        materials: crate::build::pcbcap::TIERS
            .iter()
            .map(|t| MaterialCard {
                id: t.id,
                name: t.name,
                ranges: vec![
                    r("Minimum trace", t.trace_mm, t.trace_mm, "mm", [0.0, 0.3], ""),
                    r("Minimum space", t.space_mm, t.space_mm, "mm", [0.0, 0.3], ""),
                    r("Minimum drill", t.drill_mm, t.drill_mm, "mm", [0.0, 0.6], ""),
                    r("Minimum annular ring", t.annular_mm, t.annular_mm, "mm", [0.0, 0.3], ""),
                ],
                facts: vec![
                    f("Layers", format!("up to {}", t.max_layers)),
                    f("Thickness", format!("{:.1}–{:.1} mm", t.min_thickness_mm, t.max_thickness_mm)),
                    f("Finishes", t.finishes.join(", ")),
                    f("Via-in-pad", if t.via_in_pad { "yes (filled and capped)" } else { "no" }),
                    f("Impedance control", t.impedance),
                ],
                notes: vec![t.notes],
            })
            .collect(),
        rules: vec![
            rule("Surface finish", "ENIG for fine-pitch QFN / LGA", "Hot-air solder levelling (HASL) leaves uneven pads; electroless nickel immersion gold (ENIG) is flat. Organic solderability preservative (OSP) is flat and cheap but has a shorter shelf life.", "stencil"),
            rule("Copper to edge", "≳ 0.3 mm typical", "Routing the outline can expose or tear copper too close to the edge.", "edge"),
            rule("Annular ring", "(pad − drill) / 2", "Drill wander eats into the ring; too small a ring breaks out.", "annular"),
        ],
        failures: vec![],
        tips: vec![],
        links: vec![LabLink { label: "PCB Studio — layout, DRC, Gerbers", route: "/pcb" }],
        refs: vec!["IPC-2221 — Generic standard on printed board design", "IPC-A-600 — Acceptability of printed boards", "Vendors' capability pages"],
        project_checks: vec![],
    }
}

/// The whole library, with project-aware rule values.
pub fn library(p: &Project) -> Vec<Process> {
    vec![
        fdm(p),
        resin(),
        powder(),
        laser(),
        cnc(),
        sheet(),
        electronics(p),
        pcb(),
        molding(p),
    ]
}

// ---------------------------------------------------------------------------
// Settings sheet export
// ---------------------------------------------------------------------------

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Settings sheet for one process (and optionally one material) as Markdown or CSV.
pub fn sheet_text(p: &Project, process: &str, material: Option<&str>, csv: bool) -> Result<(String, String), String> {
    let lib = library(p);
    let pr = lib
        .iter()
        .find(|x| x.id == process)
        .ok_or_else(|| format!("unknown process `{process}`"))?;
    let mats: Vec<&MaterialCard> = pr.materials.iter().filter(|m| material.is_none_or(|id| id == m.id)).collect();
    let tag = material.unwrap_or("all");
    if csv {
        let mut s = String::from("process,material,setting,low,high,unit,note\n");
        for m in &mats {
            for rg in &m.ranges {
                s += &format!(
                    "{},{},{},{},{},{},{}\n",
                    pr.id,
                    csv_escape(m.name),
                    csv_escape(rg.label),
                    rg.lo,
                    rg.hi,
                    csv_escape(rg.unit),
                    csv_escape(rg.note)
                );
            }
            for fa in &m.facts {
                s += &format!(
                    "{},{},{},,,,{}\n",
                    pr.id,
                    csv_escape(m.name),
                    csv_escape(fa.label),
                    csv_escape(&fa.value)
                );
            }
        }
        return Ok((format!("{}-{}-settings.csv", pr.id, tag.to_lowercase()), s));
    }
    let mut s = format!("# {} — settings sheet\n\n> {}\n\n{}\n\n", pr.name, pr.caveat, pr.summary);
    for m in &mats {
        s += &format!("## {}\n\n| Setting | Typical range | Note |\n|---|---|---|\n", m.name);
        for rg in &m.ranges {
            let v = if (rg.hi - rg.lo).abs() < 1e-12 {
                format!("{} {}", rg.lo, rg.unit)
            } else {
                format!("{}–{} {}", rg.lo, rg.hi, rg.unit)
            };
            s += &format!("| {} | {} | {} |\n", rg.label, v, rg.note);
        }
        for fa in &m.facts {
            s += &format!("| {} | {} | |\n", fa.label, fa.value);
        }
        for n in &m.notes {
            s += &format!("\n{n}\n");
        }
        s += "\n";
    }
    if !pr.rules.is_empty() {
        s += "## Design rules\n\n";
        for ru in &pr.rules {
            s += &format!("- **{}** — {}. {}\n", ru.title, ru.value, ru.detail);
        }
        s += "\n";
    }
    if !pr.failures.is_empty() {
        s += "## Common failures\n\n| Problem | Symptom | Fixes |\n|---|---|---|\n";
        for fl in &pr.failures {
            s += &format!("| {} | {} | {} |\n", fl.name, fl.symptom, fl.fixes.join("; "));
        }
        s += "\n";
    }
    if !pr.tips.is_empty() {
        s += "## Notes\n\n";
        for t in &pr.tips {
            s += &format!("- {t}\n");
        }
        s += "\n";
    }
    s += "## References\n\n";
    for rf in &pr.refs {
        s += &format!("- {rf}\n");
    }
    Ok((format!("{}-{}-settings.md", pr.id, tag.to_lowercase()), s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_are_ordered_and_inside_scale() {
        let p = Project::iris();
        for pr in library(&p) {
            assert!(!pr.materials.is_empty(), "{}", pr.id);
            for m in &pr.materials {
                for rg in &m.ranges {
                    assert!(rg.lo <= rg.hi, "{} {} {}", pr.id, m.id, rg.label);
                    assert!(
                        rg.lo >= rg.scale[0] && rg.hi <= rg.scale[1],
                        "{} {} {} outside scale",
                        pr.id,
                        m.id,
                        rg.label
                    );
                }
            }
        }
    }

    #[test]
    fn mold_windows_contain_molding_lab_values() {
        // The Molding Lab's single mid-range values must fall inside our windows.
        for w in MOLD_WINDOWS {
            let m = crate::molding::materials::material(w.id);
            assert_eq!(m.id, w.id);
            assert!(m.t_melt_c >= w.melt_c[0] && m.t_melt_c <= w.melt_c[1], "{}", w.id);
            assert!(m.t_mold_c >= w.mold_c[0] && m.t_mold_c <= w.mold_c[1], "{}", w.id);
            assert!(m.shrinkage_pct >= w.shrink_pct[0] && m.shrinkage_pct <= w.shrink_pct[1], "{}", w.id);
        }
    }

    #[test]
    fn iris_print_settings_inside_petg_window() {
        let p = Project::iris();
        let lib = library(&p);
        let fdm = lib.iter().find(|x| x.id == "fdm").unwrap();
        assert!(fdm.project_checks.iter().all(|c| c.status == "pass"), "{:?}", fdm.project_checks);
        // min wall rule is computed from the line width: 2 × 0.45 = 0.90 mm
        assert!(fdm.rules[0].value.contains("0.90"));
    }

    #[test]
    fn sheets_export() {
        let p = Project::iris();
        let (name, md) = sheet_text(&p, "fdm", Some("PETG"), false).unwrap();
        assert_eq!(name, "fdm-petg-settings.md");
        assert!(md.contains("220–250 °C"));
        let (_, csv) = sheet_text(&p, "fdm", Some("PETG"), true).unwrap();
        assert!(csv.lines().nth(1).unwrap().starts_with("fdm,PETG,Nozzle,220,250"));
        assert!(sheet_text(&p, "nope", None, false).is_err());
    }

    #[test]
    fn material_names_normalise() {
        assert_eq!(normalise_fdm("PETG"), "PETG");
        assert_eq!(normalise_fdm("PA12-CF"), "CF");
        assert_eq!(normalise_fdm("pc"), "PC");
        assert_eq!(normalise_fdm("TPU 95A"), "TPU95A");
    }
}
