//! KiCad exporters: the routed Iris board as a `.kicad_pcb` (KiCad 7/8
//! S-expression board file), a `.kicad_pro` project with net classes, a
//! KiCad S-expression netlist and a `.pretty` footprint library.
//!
//! ## Coordinate systems
//! Protoforge boards use millimetres with the origin at the bottom-left corner
//! and +y **up**; footprints are defined in a +y-up local frame and rotated
//! counter-clockwise. KiCad uses +y **down** with the origin at the top-left of
//! the sheet; an angle in `(at x y angle)` is counter-clockwise *as seen on
//! screen*. So a board point maps to `(X0 + x, Y0 + H − y)` and a footprint
//! pad to `(px, −py)` in the footprint's local frame, while angles keep their
//! value. The pad angle written in a board file is the pad's absolute
//! orientation (footprint angle + pad angle), as KiCad itself saves it.
//!
//! Net classes live in the project file since KiCad 6, so the class
//! definitions (track width, clearance, via size) and name patterns go into
//! the `.kicad_pro` next to the board.
//!
//! The ground pours are written as zones with their outline only: KiCad
//! refills them (press **B**) using its own clearance rules, which is more
//! faithful than importing Protoforge's raster pour.
//!
//! References: KiCad developers, *File formats* — board, footprint and
//! S-expression documentation, <https://dev-docs.kicad.org/en/file-formats/>.

use serde::Serialize;

use super::util::{num, q, uuid};
use crate::pcb::footprints::{Footprint, PadShape};
use crate::pcb::netlist::NetClass;
use crate::pcb::Layout;
use crate::project::Project;

/// Where the board sits on the KiCad sheet (A4 landscape), mm.
pub const ORIGIN_X: f64 = 100.0;
pub const ORIGIN_Y: f64 = 80.0;
/// File-format version written (KiCad 7.0; KiCad 8 and 9 open it too).
pub const KICAD_VERSION: &str = "20221018";

#[derive(Clone, Debug, Default, Serialize)]
pub struct KicadStats {
    pub footprints: usize,
    pub pads: usize,
    pub segments: usize,
    pub vias: usize,
    pub zones: usize,
    pub keepouts: usize,
    pub nets: usize,
    pub width_mm: f64,
    pub height_mm: f64,
    pub layers: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct KicadBoard {
    pub filename: String,
    pub content: String,
    pub stats: KicadStats,
    pub notes: Vec<String>,
}

fn kx(x: f64) -> f64 {
    ORIGIN_X + x
}
fn ky(h: f64, y: f64) -> f64 {
    ORIGIN_Y + h - y
}

fn layer_name(l: u8) -> &'static str {
    if l == 0 {
        "F.Cu"
    } else {
        "B.Cu"
    }
}

/// Base file name, e.g. `iris`.
pub fn base(p: &Project) -> String {
    let s = if p.meta.codename.is_empty() { &p.meta.id } else { &p.meta.codename };
    let s: String = s
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    if s.is_empty() {
        "board".into()
    } else {
        s
    }
}

/// Library-qualified footprint name used in board and netlist.
pub fn fp_lib_name(fp: &Footprint) -> String {
    format!("Protoforge:{}", fp.name)
}

struct Placement<'a> {
    refdes: &'a str,
    value: &'a str,
    x: f64,
    y: f64,
    rot: f64,
    bottom: bool,
    feature: bool,
    /// net index + name for every pad (None = unconnected)
    pad_net: Vec<Option<(usize, String)>>,
    seed: String,
}

fn stroke(w: f64) -> String {
    format!("(stroke (width {}) (type solid))", num(w))
}

/// One footprint as S-expression lines (indented by `ind`). With `pl =
/// None` it is a library footprint (`REF**`, no nets, no position).
fn footprint_sexpr(fp: &Footprint, pl: Option<&Placement>, ind: &str) -> String {
    let lib = pl.is_none();
    let bottom = pl.map(|p| p.bottom).unwrap_or(false);
    let side = |top: &str, bot: &str| if bottom { bot.to_string() } else { top.to_string() };
    let seed = pl.map(|p| p.seed.clone()).unwrap_or_else(|| format!("lib:{}", fp.name));
    let rot = pl.map(|p| p.rot).unwrap_or(0.0);
    let mut s = String::new();
    if lib {
        s += &format!("(footprint {} (version {KICAD_VERSION}) (generator protoforge)\n", q(&fp.name));
        s += &format!("{ind}  (layer \"F.Cu\")\n");
    } else {
        let p = pl.unwrap();
        s += &format!("{ind}(footprint {} (layer {})\n", q(&fp_lib_name(fp)), q(&side("F.Cu", "B.Cu")));
        s += &format!("{ind}  (tstamp {})\n", uuid(&seed));
        s += &format!(
            "{ind}  (at {} {}{})\n",
            num(p.x),
            num(p.y),
            if rot.abs() > 1e-9 { format!(" {}", num(rot)) } else { String::new() }
        );
    }
    s += &format!("{ind}  (descr {})\n", q(&fp.description));
    let has_tht = fp.pads.iter().any(|p| p.drill > 0.0);
    let feature = pl.map(|p| p.feature).unwrap_or(false);
    let mut attr = if has_tht { "through_hole" } else { "smd" }.to_string();
    if feature {
        attr = "smd board_only exclude_from_pos_files exclude_from_bom".into();
    }
    s += &format!("{ind}  (attr {attr})\n");
    let cy = fp.courtyard;
    let text_rot = if rot.abs() > 1e-9 { format!(" {}", num(rot)) } else { String::new() };
    let mirror = if bottom { " (justify mirror)" } else { "" };
    let (refdes, value) = match pl {
        Some(p) => (p.refdes.to_string(), p.value.to_string()),
        None => ("REF**".to_string(), fp.name.clone()),
    };
    s += &format!(
        "{ind}  (fp_text reference {} (at 0 {}{text_rot}) (layer {})\n{ind}    (effects (font (size 0.6 0.6) (thickness 0.1)){mirror})\n{ind}    (tstamp {})\n{ind}  )\n",
        q(&refdes),
        num(-(cy[3] + 0.5)),
        q(&side("F.SilkS", "B.SilkS")),
        uuid(&format!("{seed}:ref"))
    );
    s += &format!(
        "{ind}  (fp_text value {} (at 0 {}{text_rot}) (layer {})\n{ind}    (effects (font (size 0.5 0.5) (thickness 0.08)){mirror})\n{ind}    (tstamp {})\n{ind}  )\n",
        q(&value),
        num(-(cy[1] - 0.5)),
        q(&side("F.Fab", "B.Fab")),
        uuid(&format!("{seed}:val"))
    );
    // Courtyard, body outline (fab) and pin-1 marker (silkscreen).
    let rect = |b: [f64; 4], layer: &str, w: f64, tag: &str| -> String {
        format!(
            "{ind}  (fp_rect (start {} {}) (end {} {}) {} (fill none) (layer {}) (tstamp {}))\n",
            num(b[0]),
            num(-b[3]),
            num(b[2]),
            num(-b[1]),
            stroke(w),
            q(layer),
            uuid(&format!("{seed}:{tag}"))
        )
    };
    s += &rect(cy, &side("F.CrtYd", "B.CrtYd"), 0.05, "crtyd");
    s += &rect(fp.body, &side("F.Fab", "B.Fab"), 0.1, "fab");
    if !feature {
        let (px, py) = (fp.pin1[0], -fp.pin1[1]);
        s += &format!(
            "{ind}  (fp_circle (center {} {}) (end {} {}) {} (fill solid) (layer {}) (tstamp {}))\n",
            num(px),
            num(py),
            num(px + 0.12),
            num(py),
            stroke(0.08),
            q(&side("F.SilkS", "B.SilkS")),
            uuid(&format!("{seed}:pin1"))
        );
    }
    // Pads.
    for (k, pd) in fp.pads.iter().enumerate() {
        let tht = pd.drill > 0.0;
        let kind = if tht { "thru_hole" } else { "smd" };
        let shape = match pd.shape {
            PadShape::Rect => "rect",
            PadShape::RoundRect => "roundrect",
            PadShape::Circle => "circle",
        };
        let layers = if tht {
            "\"*.Cu\" \"*.Mask\"".to_string()
        } else {
            let (cu, paste, mask) = if bottom { ("B.Cu", "B.Paste", "B.Mask") } else { ("F.Cu", "F.Paste", "F.Mask") };
            if pd.paste > 0.0 {
                format!("{} {} {}", q(cu), q(paste), q(mask))
            } else {
                format!("{} {}", q(cu), q(mask))
            }
        };
        let ang = if rot.abs() > 1e-9 { format!(" {}", num(rot)) } else { String::new() };
        let mut line = format!(
            "{ind}  (pad {} {kind} {shape} (at {} {}{ang}) (size {} {})",
            q(&pd.num),
            num(pd.x),
            num(-pd.y),
            num(pd.w),
            num(pd.h)
        );
        if tht {
            line += &format!(" (drill {})", num(pd.drill));
        }
        line += &format!(" (layers {layers})");
        if pd.shape == PadShape::RoundRect {
            line += " (roundrect_rratio 0.25)";
        }
        if let Some(Some((ni, name))) = pl.map(|p| p.pad_net.get(k).cloned().flatten()) {
            line += &format!(" (net {ni} {})", q(&name));
        }
        line += &format!(" (tstamp {}))\n", uuid(&format!("{seed}:pad{k}:{}", pd.num)));
        s += &line;
    }
    for (k, h) in fp.npth.iter().enumerate() {
        s += &format!(
            "{ind}  (pad \"\" np_thru_hole circle (at {} {}) (size {} {}) (drill {}) (layers \"*.Cu\" \"*.Mask\") (tstamp {}))\n",
            num(h[0]),
            num(-h[1]),
            num(h[2]),
            num(h[2]),
            num(h[2]),
            uuid(&format!("{seed}:npth{k}"))
        );
    }
    s += &format!("{ind})\n");
    s
}

fn class_name(c: NetClass) -> &'static str {
    match c {
        NetClass::Ground => "Ground",
        NetClass::Power => "Power",
        NetClass::Signal => "Default",
        NetClass::Rf => "RF",
    }
}

fn rect_poly(r: [f64; 4], h: f64, ind: &str) -> String {
    format!(
        "{ind}(polygon (pts (xy {} {}) (xy {} {}) (xy {} {}) (xy {} {})))\n",
        num(kx(r[0])),
        num(ky(h, r[1])),
        num(kx(r[2])),
        num(ky(h, r[1])),
        num(kx(r[2])),
        num(ky(h, r[3])),
        num(kx(r[0])),
        num(ky(h, r[3]))
    )
}

/// The routed board of `p` as a KiCad 7 `.kicad_pcb` file.
pub fn kicad_pcb(p: &Project) -> KicadBoard {
    let l = crate::pcb::layout(p);
    kicad_pcb_from(p, &l)
}

/// Same as [`kicad_pcb`] for an already computed layout.
pub fn kicad_pcb_from(p: &Project, l: &Layout) -> KicadBoard {
    let (w, h) = (l.board.width, l.board.height);
    let name = base(p);
    let mut st = KicadStats {
        width_mm: w,
        height_mm: h,
        layers: l.board.layers,
        ..Default::default()
    };
    let mut s = String::new();
    s += &format!("(kicad_pcb (version {KICAD_VERSION}) (generator protoforge)\n\n");
    s += &format!("  (general\n    (thickness {})\n  )\n\n", num(l.board.thickness));
    s += "  (paper \"A4\")\n";
    s += &format!(
        "  (title_block\n    (title {})\n    (rev {})\n    (company {})\n    (comment 1 {})\n    (comment 2 {})\n  )\n\n",
        q(&p.meta.name),
        q(&p.meta.version),
        q(&p.meta.author),
        q("Generated by Protoforge (https://github.com/Normansrule/protoforge)"),
        q("Press B in KiCad to refill the ground zones; verify footprints against datasheets")
    );
    s += "  (layers\n    (0 \"F.Cu\" signal)\n    (31 \"B.Cu\" signal)\n    (32 \"B.Adhes\" user \"B.Adhesive\")\n    (33 \"F.Adhes\" user \"F.Adhesive\")\n    (34 \"B.Paste\" user)\n    (35 \"F.Paste\" user)\n    (36 \"B.SilkS\" user \"B.Silkscreen\")\n    (37 \"F.SilkS\" user \"F.Silkscreen\")\n    (38 \"B.Mask\" user)\n    (39 \"F.Mask\" user)\n    (40 \"Dwgs.User\" user \"User.Drawings\")\n    (41 \"Cmts.User\" user \"User.Comments\")\n    (42 \"Eco1.User\" user \"User.Eco1\")\n    (43 \"Eco2.User\" user \"User.Eco2\")\n    (44 \"Edge.Cuts\" user)\n    (45 \"Margin\" user)\n    (46 \"B.CrtYd\" user \"B.Courtyard\")\n    (47 \"F.CrtYd\" user \"F.Courtyard\")\n    (48 \"B.Fab\" user)\n    (49 \"F.Fab\" user)\n  )\n\n";
    s += "  (setup\n    (pad_to_mask_clearance 0)\n  )\n\n";
    // Nets: 0 is the unconnected net.
    s += "  (net 0 \"\")\n";
    for (i, n) in l.nets.iter().enumerate() {
        s += &format!("  (net {} {})\n", i + 1, q(&n.name));
    }
    st.nets = l.nets.len();
    s += "\n";
    let net_idx = |name: &str| -> usize {
        if name.is_empty() {
            return 0;
        }
        l.nets.iter().position(|n| n.name == name).map(|i| i + 1).unwrap_or(0)
    };
    // Footprints.
    for pp in &l.parts {
        let Some(fp) = crate::pcb::board::part_footprint(p, &pp.refdes) else { continue };
        let pad_net: Vec<Option<(usize, String)>> = fp
            .pads
            .iter()
            .map(|pd| {
                l.pads
                    .iter()
                    .find(|bp| bp.refdes == pp.refdes && bp.pin == pd.num)
                    .filter(|bp| !bp.net.is_empty())
                    .map(|bp| (net_idx(&bp.net), bp.net.clone()))
                    .filter(|x| x.0 > 0)
            })
            .collect();
        let value = if pp.feature {
            fp.name.clone()
        } else {
            p.components
                .iter()
                .find(|c| c.refdes == pp.refdes)
                .map(crate::pcb::fab::bom_value)
                .unwrap_or_else(|| pp.value.clone())
        };
        let pl = Placement {
            refdes: &pp.refdes,
            value: &value,
            x: kx(pp.x),
            y: ky(h, pp.y),
            rot: pp.rot,
            bottom: pp.side == "bottom",
            feature: pp.feature,
            pad_net,
            seed: format!("{}:fp:{}", name, pp.refdes),
        };
        s += &footprint_sexpr(&fp, Some(&pl), "  ");
        s += "\n";
        st.footprints += 1;
        st.pads += fp.pads.len() + fp.npth.len();
    }
    // Board outline.
    s += &format!(
        "  (gr_rect (start {} {}) (end {} {}) {} (fill none) (layer \"Edge.Cuts\") (tstamp {}))\n",
        num(kx(0.0)),
        num(ky(h, h)),
        num(kx(w)),
        num(ky(h, 0.0)),
        stroke(0.1),
        uuid(&format!("{name}:edge"))
    );
    s += &format!(
        "  (gr_text {} (at {} {}) (layer \"Cmts.User\")\n    (effects (font (size 1 1) (thickness 0.15)) (justify left))\n    (tstamp {})\n  )\n\n",
        q(&format!("{} v{} — {} × {} mm, {} layers", p.meta.name, p.meta.version, num(w), num(h), l.board.layers)),
        num(kx(0.0)),
        num(ky(h, h) - 2.5),
        uuid(&format!("{name}:title"))
    );
    // Tracks and vias.
    for (i, t) in l.tracks.iter().enumerate() {
        s += &format!(
            "  (segment (start {} {}) (end {} {}) (width {}) (layer {}) (net {}) (tstamp {}))\n",
            num(kx(t.x0)),
            num(ky(h, t.y0)),
            num(kx(t.x1)),
            num(ky(h, t.y1)),
            num(t.width),
            q(layer_name(t.layer)),
            net_idx(&t.net),
            uuid(&format!("{name}:seg{i}"))
        );
    }
    st.segments = l.tracks.len();
    for (i, v) in l.vias.iter().enumerate() {
        s += &format!(
            "  (via (at {} {}) (size {}) (drill {}) (layers \"F.Cu\" \"B.Cu\") (net {}) (tstamp {}))\n",
            num(kx(v.x)),
            num(ky(h, v.y)),
            num(v.pad),
            num(v.drill),
            net_idx(&v.net),
            uuid(&format!("{name}:via{i}"))
        );
    }
    st.vias = l.vias.len();
    s += "\n";
    // Ground zones (outline only — KiCad refills them).
    let gnd = net_idx("GND");
    if gnd > 0 {
        let e = l.rules.edge_clearance_mm;
        for (layer, pour) in [("B.Cu", &l.pour), ("F.Cu", &l.pour_top)] {
            if pour.rects.is_empty() && layer == "F.Cu" {
                continue;
            }
            s += &format!(
                "  (zone (net {gnd}) (net_name \"GND\") (layer {}) (tstamp {}) (hatch edge 0.5)\n    (connect_pads (clearance {}))\n    (min_thickness {}) (filled_areas_thickness no)\n    (fill (thermal_gap {}) (thermal_bridge_width {}))\n",
                q(layer),
                uuid(&format!("{name}:zone:{layer}")),
                num(l.rules.pour_clearance_mm),
                num(l.rules.trace_mm.min(0.25)),
                num(l.rules.pour_clearance_mm.max(0.25)),
                num(l.rules.trace_mm.max(0.25)),
            );
            s += &rect_poly([e, e, w - e, h - e], h, "    ");
            s += "  )\n";
            st.zones += 1;
        }
        // Rule areas: antenna and touch-electrode keep-outs.
        for (k, ko) in l.keepouts.iter().enumerate() {
            for (on, layer) in [(ko.top, "F.Cu"), (ko.bottom, "B.Cu")] {
                if !on {
                    continue;
                }
                // The RF feed may cross the antenna keep-out on the top layer.
                let tracks = if !ko.allow_net.is_empty() && layer == "F.Cu" { "allowed" } else { "not_allowed" };
                let tracks = if ko.no_pour && ko.name.contains("Touch") { "allowed" } else { tracks };
                s += &format!(
                    "  (zone (net 0) (net_name \"\") (layer {}) (tstamp {}) (name {}) (hatch edge 0.5)\n    (connect_pads (clearance 0))\n    (min_thickness 0.25) (filled_areas_thickness no)\n    (keepout (tracks {tracks}) (vias {}) (pads allowed) (copperpour not_allowed) (footprints allowed))\n    (fill (thermal_gap 0.5) (thermal_bridge_width 0.5))\n",
                    q(layer),
                    uuid(&format!("{name}:keepout{k}:{layer}")),
                    q(&ko.name),
                    if ko.name.contains("Touch") { "allowed" } else { "not_allowed" }
                );
                s += &rect_poly(ko.rect, h, "    ");
                s += "  )\n";
                st.keepouts += 1;
            }
        }
    }
    s += ")\n";
    let notes = vec![
        format!(
            "Open {name}.kicad_pcb in KiCad 7 or newer (File → Open), press B to refill the ground zones, then run Inspect → Design Rules Checker."
        ),
        "Footprints are embedded in the board and also shipped as the Protoforge.pretty library; they are typical IPC-7351-style approximations — replace them with the manufacturer's land patterns before ordering.".into(),
        "Exposed-pad paste windows (≈55 % coverage) are only in Protoforge's Gerber paste layer; in KiCad set a negative solder-paste margin or draw the windows before generating paste.".into(),
        "Net classes (Power, RF, Ground, Default) are defined in the .kicad_pro file, as KiCad 6+ expects.".into(),
    ];
    KicadBoard {
        filename: format!("{name}.kicad_pcb"),
        content: s,
        stats: st,
        notes,
    }
}

/// A `.kicad_pro` project file with the design rules and net classes.
pub fn kicad_pro(p: &Project) -> String {
    let l = crate::pcb::layout(p);
    let r = &l.rules;
    let name = base(p);
    let class = |cname: &str, width: f64, clearance: f64| {
        serde_json::json!({
            "name": cname,
            "clearance": clearance,
            "track_width": width,
            "via_diameter": r.via_pad_mm,
            "via_drill": r.via_drill_mm,
            "microvia_diameter": 0.3,
            "microvia_drill": 0.1,
            "diff_pair_gap": 0.25,
            "diff_pair_via_gap": 0.25,
            "diff_pair_width": 0.2,
            "line_style": 0,
            "wire_width": 6,
            "bus_width": 12,
            "pcb_color": "rgba(0, 0, 0, 0.000)",
            "schematic_color": "rgba(0, 0, 0, 0.000)"
        })
    };
    // Widest IPC-2221 width among the nets of a class, never below the rule.
    let class_width = |c: NetClass| {
        l.nets
            .iter()
            .filter(|n| n.class == c)
            .map(|n| n.ipc_width_mm)
            .fold(r.trace_mm, f64::max)
            .min(0.5)
    };
    let mut patterns = Vec::new();
    for n in &l.nets {
        if n.class != NetClass::Signal {
            patterns.push(serde_json::json!({ "netclass": class_name(n.class), "pattern": n.name }));
        }
    }
    let v = serde_json::json!({
        "board": {
            "design_settings": {
                "defaults": { "board_outline_line_width": 0.1, "copper_line_width": 0.2 },
                "rules": {
                    "min_clearance": r.clearance_mm,
                    "min_copper_edge_clearance": r.edge_clearance_mm,
                    "min_hole_clearance": r.hole_clearance_mm,
                    "min_through_hole_diameter": r.min_drill_mm,
                    "min_track_width": r.trace_mm,
                    "min_via_annular_width": r.min_annular_mm,
                    "min_via_diameter": r.via_pad_mm
                },
                "track_widths": [0.0, r.trace_mm, 0.25, 0.3],
                "via_dimensions": [{ "diameter": 0.0, "drill": 0.0 }, { "diameter": r.via_pad_mm, "drill": r.via_drill_mm }]
            }
        },
        "meta": { "filename": format!("{name}.kicad_pro"), "version": 1 },
        "net_settings": {
            "classes": [
                class("Default", r.trace_mm, r.clearance_mm),
                class("Power", class_width(NetClass::Power), r.clearance_mm),
                class("Ground", class_width(NetClass::Ground), r.clearance_mm),
                class("RF", class_width(NetClass::Rf), r.clearance_mm.max(0.2))
            ],
            "meta": { "version": 3 },
            "net_colors": null,
            "netclass_assignments": null,
            "netclass_patterns": patterns
        },
        "text_variables": { "PROTOFORGE_VERSION": p.meta.version }
    });
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

/// KiCad S-expression netlist (the format Eeschema exports, version "E").
pub fn kicad_netlist(p: &Project) -> String {
    let l = crate::pcb::layout(p);
    let name = base(p);
    let mut s = format!(
        "(export (version \"E\")\n  (design\n    (source {})\n    (tool \"Protoforge\")\n  )\n  (components\n",
        q(&format!("{name}.kicad_sch"))
    );
    for pp in l.parts.iter().filter(|x| !x.feature) {
        let Some(fp) = crate::pcb::board::part_footprint(p, &pp.refdes) else { continue };
        let c = p.components.iter().find(|c| c.refdes == pp.refdes);
        let value = c.map(crate::pcb::fab::bom_value).unwrap_or_default();
        s += &format!(
            "    (comp (ref {})\n      (value {})\n      (footprint {})\n      (description {})\n      (libsource (lib \"Protoforge\") (part {}) (description {}))\n      (sheetpath (names \"/\") (tstamps \"/\"))\n      (tstamps {}))\n",
            q(&pp.refdes),
            q(&value),
            q(&fp_lib_name(&fp)),
            q(c.map(|c| c.name.as_str()).unwrap_or("")),
            q(c.map(|c| c.part_class.as_str()).unwrap_or("")),
            q(&fp.description),
            q(&uuid(&format!("{name}:sch:{}", pp.refdes)))
        );
    }
    s += "  )\n  (nets\n";
    for (i, n) in l.nets.iter().enumerate() {
        s += &format!("    (net (code \"{}\") (name {}) (class {})\n", i + 1, q(&n.name), q(class_name(n.class)));
        for pin in &n.pins {
            if let Some((r, pn)) = pin.split_once('.') {
                s += &format!("      (node (ref {}) (pin {}))\n", q(r), q(pn));
            }
        }
        s += "    )\n";
    }
    s += "  )\n)\n";
    s
}

/// Every footprint used by the board as a `.kicad_mod` library file.
pub fn footprint_library(p: &Project) -> Vec<(String, String)> {
    let l = crate::pcb::layout(p);
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for pp in &l.parts {
        let Some(fp) = crate::pcb::board::part_footprint(p, &pp.refdes) else { continue };
        if seen.contains(&fp.name) {
            continue;
        }
        seen.push(fp.name.clone());
        out.push((format!("{}.kicad_mod", fp.name), footprint_sexpr(&fp, None, "")));
    }
    out
}

/// `fp-lib-table` entry pointing KiCad at the shipped `.pretty` folder.
pub fn fp_lib_table() -> String {
    "(fp_lib_table\n  (version 7)\n  (lib (name \"Protoforge\") (type \"KiCad\") (uri \"${KIPRJMOD}/Protoforge.pretty\") (options \"\") (descr \"Footprints generated by Protoforge\"))\n)\n".into()
}

#[cfg(test)]
mod tests {
    use super::super::util::parse_sexpr;
    use super::*;

    #[test]
    fn iris_board_parses_and_matches_the_layout() {
        let p = Project::iris();
        let l = crate::pcb::layout(&p);
        let b = kicad_pcb_from(&p, &l);
        let t = parse_sexpr(&b.content).expect("valid s-expression");
        assert_eq!(t.head(), Some("kicad_pcb"));
        assert_eq!(t.child("version").unwrap().text(1), Some(KICAD_VERSION));
        let fps: Vec<_> = t.children("footprint").collect();
        assert_eq!(fps.len(), l.parts.len());
        assert_eq!(t.children("segment").count(), l.tracks.len());
        assert_eq!(t.children("via").count(), l.vias.len());
        assert_eq!(t.children("net").count(), l.nets.len() + 1);
        assert!(t.children("zone").any(|z| z.child("net_name").and_then(|n| n.text(1)) == Some("GND")));
        assert!(t.children("gr_rect").any(|g| g.child("layer").and_then(|x| x.text(1)) == Some("Edge.Cuts")));
        // Every pad lands exactly where the Protoforge board model has it.
        let mut checked = 0;
        for f in &fps {
            let at = f.child("at").unwrap();
            let (fx, fy) = (at.f(1).unwrap(), at.f(2).unwrap());
            let ang = at.f(3).unwrap_or(0.0).to_radians();
            let refdes = f.children("fp_text").find(|x| x.text(1) == Some("reference")).unwrap().text(2).unwrap().to_string();
            for pad in f.children("pad") {
                let num_ = pad.text(1).unwrap();
                if num_.is_empty() {
                    continue;
                }
                let pat = pad.child("at").unwrap();
                let (lx, ly) = (pat.f(1).unwrap(), pat.f(2).unwrap());
                // KiCad: absolute = anchor + R(angle) (screen CCW, y down)
                let x = fx + lx * ang.cos() + ly * ang.sin();
                let y = fy - lx * ang.sin() + ly * ang.cos();
                let bp = l.pads.iter().find(|bp| bp.refdes == refdes && bp.pin == num_).unwrap();
                assert!((x - kx(bp.x)).abs() < 1e-4 && (y - ky(l.board.height, bp.y)).abs() < 1e-4, "{refdes}.{num_}");
                let net = pad.child("net").and_then(|n| n.text(2)).unwrap_or("");
                assert_eq!(net, bp.net, "{refdes}.{num_} net");
                checked += 1;
            }
        }
        assert!(checked > 150, "{checked} pads checked");
        assert_eq!(b.stats.footprints, l.parts.len());
    }

    #[test]
    fn netlist_and_library_parse() {
        let p = Project::iris();
        let n = parse_sexpr(&kicad_netlist(&p)).unwrap();
        assert_eq!(n.head(), Some("export"));
        let nets = n.child("nets").unwrap();
        assert!(nets.children("net").any(|x| x.child("name").and_then(|v| v.text(1)) == Some("GND")));
        let lib = footprint_library(&p);
        assert!(lib.len() >= 10);
        for (name, src) in &lib {
            let t = parse_sexpr(src).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(t.head(), Some("footprint"));
            assert!(t.children("pad").count() >= 1);
        }
        let pro: serde_json::Value = serde_json::from_str(&kicad_pro(&p)).unwrap();
        assert!(pro["net_settings"]["classes"].as_array().unwrap().len() == 4);
        assert!(pro["net_settings"]["netclass_patterns"].as_array().unwrap().iter().any(|x| x["pattern"] == "GND"));
    }
}
