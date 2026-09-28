//! Fabrication and assembly outputs.
//!
//! * **Gerber RS-274X with X2 attributes** (Ucamco, *The Gerber Layer Format
//!   Specification*): `%FSLAX46Y46*%` (absolute, 4 integer + 6 decimal
//!   digits), `%MOMM*%`, file attributes (`%TF.FileFunction,…*%`), aperture
//!   attributes (`%TA.AperFunction,…*%`), object attributes for pads and
//!   nets (`%TO.P`, `%TO.N`), D01 (draw) / D02 (move) / D03 (flash) and G36/G37
//!   regions for the ground pour.
//! * **Excellon** drill files (metric, decimal coordinates, tool table),
//!   plated and non-plated separately.
//! * **BOM / CPL** CSVs with the column names JLCPCB documents for its
//!   assembly service (`Comment,Designator,Footprint,LCSC Part #` and
//!   `Designator,Mid X,Mid Y,Layer,Rotation`). Part numbers are left blank on
//!   purpose — pick them yourself in the distributor's catalogue.
//! * A fab README (stack-up, finish, rules actually used) and an SVG
//!   assembly drawing.

use serde::{Deserialize, Serialize};

use super::board::BoardPad;
use super::footprints::PadShape;
use super::geom::Shape;
use super::Layout;
use crate::project::{Component, Project};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FabFile {
    pub path: String,
    pub content: String,
    /// Gerber layer name / file type for the viewer.
    pub kind: String,
}

fn mm(v: f64) -> i64 {
    (v * 1e6).round() as i64
}

/// Minimal Gerber X2 writer.
pub struct Gerber {
    body: String,
    apertures: Vec<(String, String, u32)>, // (definition, function, D-code)
    current: Option<u32>,
    file_attrs: Vec<String>,
}

impl Gerber {
    pub fn new(file_function: &str, polarity: &str, project: &str) -> Gerber {
        Gerber {
            body: String::new(),
            apertures: vec![],
            current: None,
            file_attrs: vec![
                "%TF.GenerationSoftware,Protoforge,forge-core,0.1*%".into(),
                format!("%TF.ProjectId,{},0,rev1*%", project.replace([',', '*', '%'], "_")),
                "%TF.SameCoordinates,Original*%".into(),
                format!("%TF.FileFunction,{file_function}*%"),
                format!("%TF.FilePolarity,{polarity}*%"),
            ],
        }
    }
    fn ap(&mut self, def: String, function: &str) -> u32 {
        if let Some(a) = self.apertures.iter().find(|a| a.0 == def && a.1 == function) {
            return a.2;
        }
        let d = 10 + self.apertures.len() as u32;
        self.apertures.push((def, function.to_string(), d));
        d
    }
    fn select(&mut self, d: u32) {
        if self.current != Some(d) {
            self.body += &format!("D{d}*\n");
            self.current = Some(d);
        }
    }
    pub fn circle_ap(&mut self, dia: f64, function: &str) -> u32 {
        self.ap(format!("C,{dia:.6}"), function)
    }
    pub fn rect_ap(&mut self, w: f64, h: f64, function: &str) -> u32 {
        self.ap(format!("R,{w:.6}X{h:.6}"), function)
    }
    pub fn flash(&mut self, d: u32, x: f64, y: f64) {
        self.select(d);
        self.body += &format!("X{}Y{}D03*\n", mm(x), mm(y));
    }
    pub fn draw(&mut self, d: u32, x0: f64, y0: f64, x1: f64, y1: f64) {
        self.select(d);
        self.body += &format!("X{}Y{}D02*\nX{}Y{}D01*\n", mm(x0), mm(y0), mm(x1), mm(y1));
    }
    pub fn polyline(&mut self, d: u32, pts: &[(f64, f64)]) {
        if pts.len() < 2 {
            return;
        }
        self.select(d);
        self.body += &format!("X{}Y{}D02*\n", mm(pts[0].0), mm(pts[0].1));
        for p in &pts[1..] {
            self.body += &format!("X{}Y{}D01*\n", mm(p.0), mm(p.1));
        }
    }
    pub fn region(&mut self, pts: &[(f64, f64)]) {
        if pts.len() < 3 {
            return;
        }
        self.body += "G36*\n";
        self.body += &format!("X{}Y{}D02*\n", mm(pts[0].0), mm(pts[0].1));
        for p in &pts[1..] {
            self.body += &format!("X{}Y{}D01*\n", mm(p.0), mm(p.1));
        }
        self.body += &format!("X{}Y{}D01*\n", mm(pts[0].0), mm(pts[0].1));
        self.body += "G37*\n";
    }
    pub fn attr(&mut self, a: &str) {
        self.body += &format!("%{a}*%\n");
    }
    pub fn clear_attrs(&mut self) {
        self.body += "%TD*%\n";
    }
    pub fn finish(self) -> String {
        let mut s = String::new();
        s += "G04 Protoforge PCB Studio — Gerber X2 (RS-274X)*\n";
        for a in &self.file_attrs {
            s += a;
            s += "\n";
        }
        s += "%FSLAX46Y46*%\n%MOMM*%\n%LPD*%\nG01*\n";
        for (def, func, d) in &self.apertures {
            s += &format!("%TA.AperFunction,{func}*%\n%ADD{d}{def}*%\n");
        }
        if !self.apertures.is_empty() {
            s += "%TD*%\n";
        }
        s += &self.body;
        s += "M02*\n";
        s
    }
}

fn pad_ap(g: &mut Gerber, p: &BoardPad, grow: f64, function: &str) -> u32 {
    match p.shape {
        PadShape::Circle => g.circle_ap(p.w + 2.0 * grow, function),
        _ => g.rect_ap(p.w + 2.0 * grow, p.h + 2.0 * grow, function),
    }
}

fn pad_function(p: &BoardPad) -> &'static str {
    if p.drill > 0.0 {
        "ComponentPad"
    } else if p.refdes.starts_with("TP") {
        "TestPad"
    } else if p.refdes == "E1" {
        "Conductor"
    } else {
        "SMDPad,CuDef"
    }
}

fn gerber_escape(s: &str) -> String {
    s.replace([',', '*', '%'], "_")
}

fn copper_layer(l: &Layout, project: &str, bottom: bool) -> String {
    let mut g = Gerber::new(if bottom { "Copper,L2,Bot" } else { "Copper,L1,Top" }, "Positive", project);
    // pour first (regions), then everything else on top of it
    let pour = if bottom { &l.pour } else { &l.pour_top };
    if !pour.rects.is_empty() {
        g.attr("TO.N,GND");
        for r in &pour.rects {
            g.region(&[(r[0], r[1]), (r[2], r[1]), (r[2], r[3]), (r[0], r[3])]);
        }
        g.clear_attrs();
    }
    for p in l.pads.iter().filter(|p| if bottom { p.bottom } else { p.top }) {
        let d = pad_ap(&mut g, p, 0.0, pad_function(p));
        g.attr(&format!("TO.P,{},{}", gerber_escape(&p.refdes), gerber_escape(&p.pin)));
        if !p.net.is_empty() {
            g.attr(&format!("TO.N,{}", gerber_escape(&p.net)));
        }
        g.flash(d, p.x, p.y);
        g.clear_attrs();
    }
    let layer = if bottom { 1 } else { 0 };
    let mut last_net = String::new();
    for t in l.tracks.iter().filter(|t| t.layer == layer) {
        let d = g.circle_ap(t.width, "Conductor");
        if t.net != last_net {
            if !last_net.is_empty() {
                g.clear_attrs();
            }
            g.attr(&format!("TO.N,{}", gerber_escape(&t.net)));
            last_net = t.net.clone();
        }
        g.draw(d, t.x0, t.y0, t.x1, t.y1);
    }
    if !last_net.is_empty() {
        g.clear_attrs();
    }
    for v in &l.vias {
        let d = g.circle_ap(v.pad, "ViaPad");
        g.attr(&format!("TO.N,{}", gerber_escape(&v.net)));
        g.flash(d, v.x, v.y);
        g.clear_attrs();
    }
    g.finish()
}

const MASK_EXPANSION: f64 = 0.05;

fn mask_layer(l: &Layout, project: &str, bottom: bool) -> String {
    let mut g = Gerber::new(if bottom { "Soldermask,Bot" } else { "Soldermask,Top" }, "Negative", project);
    for p in l.pads.iter().filter(|p| if bottom { p.bottom } else { p.top }) {
        if p.refdes == "E1" {
            continue; // electrode stays under solder mask
        }
        let d = pad_ap(&mut g, p, MASK_EXPANSION, "SMDPad,CuDef");
        g.flash(d, p.x, p.y);
    }
    g.finish()
}

fn paste_layer(l: &Layout, project: &str) -> String {
    let mut g = Gerber::new("Paste,Top", "Positive", project);
    for p in l.pads.iter().filter(|p| p.top && p.drill <= 0.0 && p.paste > 0.0) {
        if p.paste >= 0.999 {
            let d = pad_ap(&mut g, p, 0.0, "SMDPad,CuDef");
            g.flash(d, p.x, p.y);
        } else {
            // windowed stencil: 2 × 2 apertures covering `paste` of the area
            let s = p.paste.sqrt();
            let (ww, hh) = (p.w * s / 2.0 * 0.9, p.h * s / 2.0 * 0.9);
            let d = g.rect_ap(ww, hh, "SMDPad,CuDef");
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                g.flash(d, p.x + sx * p.w / 4.0, p.y + sy * p.h / 4.0);
            }
        }
    }
    g.finish()
}

fn edge_layer(l: &Layout, project: &str) -> String {
    let mut g = Gerber::new("Profile,NP", "Positive", project);
    let d = g.circle_ap(0.1, "Profile");
    let (w, h) = (l.board.width, l.board.height);
    g.polyline(d, &[(0.0, 0.0), (w, 0.0), (w, h), (0.0, h), (0.0, 0.0)]);
    g.finish()
}

// ---------------------------------------------------------------------------
// Silkscreen: outlines clipped away from pads, pin-1 dots and stroke text.
// ---------------------------------------------------------------------------

/// Tiny stroke font on a 4 × 6 grid (polylines).
fn glyph(c: char) -> Vec<Vec<(f64, f64)>> {
    let p = |s: &str| -> Vec<Vec<(f64, f64)>> {
        s.split('|')
            .map(|stroke| {
                stroke
                    .split(' ')
                    .filter(|t| !t.is_empty())
                    .map(|t| {
                        let b = t.as_bytes();
                        ((b[0] - b'0') as f64, (b[1] - b'0') as f64)
                    })
                    .collect()
            })
            .collect()
    };
    match c.to_ascii_uppercase() {
        'A' => p("00 04 26 44 40|03 43"),
        'B' => p("00 06 36 45 34 03|34 43 41 30 00"),
        'C' => p("41 30 10 01 05 16 36 45"),
        'D' => p("00 06 26 44 42 20 00"),
        'E' => p("40 00 06 46|03 33"),
        'F' => p("00 06 46|03 33"),
        'G' => p("45 36 16 05 01 10 30 41 43 23"),
        'H' => p("00 06|40 46|03 43"),
        'I' => p("10 30|20 26|16 36"),
        'J' => p("01 10 30 41 46"),
        'K' => p("00 06|46 03 40"),
        'L' => p("06 00 40"),
        'M' => p("00 06 23 46 40"),
        'N' => p("00 06 40 46"),
        'O' => p("10 01 05 16 36 45 41 30 10"),
        'P' => p("00 06 36 45 44 33 03"),
        'Q' => p("10 01 05 16 36 45 41 30 10|22 40"),
        'R' => p("00 06 36 45 44 33 03|23 40"),
        'S' => p("01 10 30 41 42 33 13 04 05 16 36 45"),
        'T' => p("06 46|26 20"),
        'U' => p("06 01 10 30 41 46"),
        'V' => p("06 20 46"),
        'W' => p("06 10 23 30 46"),
        'X' => p("00 46|06 40"),
        'Y' => p("06 23 46|23 20"),
        'Z' => p("06 46 00 40"),
        '0' => p("10 01 05 16 36 45 41 30 10|05 41"),
        '1' => p("14 26 20|10 30"),
        '2' => p("05 16 36 45 44 00 40"),
        '3' => p("05 16 36 45 44 33 13|33 42 41 30 10 01"),
        '4' => p("36 03 43|30 36"),
        '5' => p("46 06 04 34 43 41 30 10 01"),
        '6' => p("45 36 16 05 01 10 30 41 42 33 03"),
        '7' => p("06 46 20"),
        '8' => p("13 04 05 16 36 45 44 33 13 02 01 10 30 41 42 33"),
        '9' => p("41 30 10 01|03 13 33 44 45 36 16 05 04 13|44 41"),
        '-' => p("13 33"),
        '.' => p("20 21"),
        '/' => p("00 46"),
        ':' => p("22 23|24 25"),
        '+' => p("13 33|22 24"),
        _ => vec![],
    }
}

/// Polylines for a text string, baseline-left at (x, y), cap height `h`.
pub fn text_strokes(s: &str, x: f64, y: f64, h: f64) -> Vec<Vec<(f64, f64)>> {
    let k = h / 6.0;
    let adv = 5.5 * k;
    let mut out = Vec::new();
    for (i, c) in s.chars().enumerate() {
        for stroke in glyph(c) {
            out.push(stroke.iter().map(|(gx, gy)| (x + i as f64 * adv + gx * k, y + gy * k)).collect());
        }
    }
    out
}

pub fn text_width(s: &str, h: f64) -> f64 {
    let n = s.chars().count() as f64;
    (n * 5.5 - 1.5) * h / 6.0
}

/// Silkscreen line segments (for the Gerber and the web viewer).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Silk {
    pub lines: Vec<[f64; 4]>,
    pub dots: Vec<[f64; 3]>,
    pub labels: Vec<SilkLabel>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SilkLabel {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub h: f64,
}

pub const SILK_W: f64 = 0.15;

pub fn silkscreen(l: &Layout, title: &str) -> Silk {
    let keep = 0.15 + SILK_W / 2.0; // silk-to-pad clearance
    let pads: Vec<Shape> = l
        .pads
        .iter()
        .filter(|p| p.top)
        .map(|p| p.shape_geom())
        .chain(l.vias.iter().map(|v| v.shape()))
        .collect();
    let clear_of_pads = |x: f64, y: f64| pads.iter().all(|s| s.dist_point(x, y) >= keep);
    let mut silk = Silk::default();
    let edge = 0.2;
    let inside = |x: f64, y: f64| x > edge && y > edge && x < l.board.width - edge && y < l.board.height - edge;
    for part in l.parts.iter().filter(|p| !p.feature && p.side == "top") {
        let b = part.body;
        let edges = [
            (b[0], b[1], b[2], b[1]),
            (b[2], b[1], b[2], b[3]),
            (b[2], b[3], b[0], b[3]),
            (b[0], b[3], b[0], b[1]),
        ];
        for (x0, y0, x1, y1) in edges {
            let len = (x1 - x0).hypot(y1 - y0);
            let n = (len / 0.05).ceil().max(1.0) as usize;
            let mut run: Option<(f64, f64)> = None;
            let mut last = (x0, y0);
            for k in 0..=n {
                let t = k as f64 / n as f64;
                let (x, y) = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
                let good = clear_of_pads(x, y) && inside(x, y);
                match (good, run) {
                    (true, None) => run = Some((x, y)),
                    (false, Some(s)) => {
                        if (last.0 - s.0).hypot(last.1 - s.1) > 0.2 {
                            silk.lines.push([s.0, s.1, last.0, last.1]);
                        }
                        run = None;
                    }
                    _ => {}
                }
                if good {
                    last = (x, y);
                }
            }
            if let Some(s) = run {
                if (last.0 - s.0).hypot(last.1 - s.1) > 0.2 {
                    silk.lines.push([s.0, s.1, last.0, last.1]);
                }
            }
        }
        // pin-1 dot just outside the body near pin 1
        let (px, py) = (part.pin1[0], part.pin1[1]);
        if clear_of_pads(px, py) && inside(px, py) && !part.footprint.starts_with("0402") && !part.footprint.starts_with("0603") {
            silk.dots.push([px, py, 0.25]);
        }
    }
    // text: refdes where a free spot exists, then the board title
    let boxes: Vec<[f64; 4]> = l.parts.iter().filter(|p| p.side == "top" && !p.feature).map(|p| p.body).collect();
    let mut used: Vec<[f64; 4]> = Vec::new();
    let place_text = |text: &str, h: f64, cands: Vec<(f64, f64)>, silk: &mut Silk, used: &mut Vec<[f64; 4]>| -> bool {
        let tw = text_width(text, h);
        for (x, y) in cands {
            let bb = [x - 0.1, y - 0.1, x + tw + 0.1, y + h + 0.1];
            if bb[0] < 0.3 || bb[1] < 0.3 || bb[2] > l.board.width - 0.3 || bb[3] > l.board.height - 0.3 {
                continue;
            }
            let hits_box = |b: &[f64; 4]| bb[0] < b[2] && bb[2] > b[0] && bb[1] < b[3] && bb[3] > b[1];
            if boxes.iter().any(hits_box) || used.iter().any(hits_box) {
                continue;
            }
            let sh = Shape::Rect {
                cx: (bb[0] + bb[2]) / 2.0,
                cy: (bb[1] + bb[3]) / 2.0,
                hw: (bb[2] - bb[0]) / 2.0,
                hh: (bb[3] - bb[1]) / 2.0,
            };
            if pads.iter().any(|p| p.dist(&sh) < keep) {
                continue;
            }
            silk.labels.push(SilkLabel {
                text: text.into(),
                x,
                y,
                h,
            });
            used.push(bb);
            return true;
        }
        false
    };
    let h = 0.8;
    for part in l.parts.iter().filter(|p| !p.feature && p.side == "top") {
        let c = part.courtyard;
        let tw = text_width(&part.refdes, h);
        let mid = (c[0] + c[2]) / 2.0 - tw / 2.0;
        let cands = vec![
            (mid, c[3] + 0.15),
            (mid, c[1] - h - 0.15),
            (c[2] + 0.15, (c[1] + c[3]) / 2.0 - h / 2.0),
            (c[0] - tw - 0.15, (c[1] + c[3]) / 2.0 - h / 2.0),
        ];
        place_text(&part.refdes, h, cands, &mut silk, &mut used);
    }
    let mut cands = Vec::new();
    let mut y = 0.4;
    while y < l.board.height {
        let mut x = 0.4;
        while x < l.board.width {
            cands.push((x, y));
            x += 0.25;
        }
        y += 0.25;
    }
    if !place_text(title, 1.0, cands.clone(), &mut silk, &mut used) {
        place_text(title, 0.8, cands, &mut silk, &mut used);
    }
    silk
}

fn silk_layer(project: &str, silk: &Silk) -> String {
    let mut g = Gerber::new("Legend,Top", "Positive", project);
    let d = g.circle_ap(SILK_W, "Other,Legend");
    for s in &silk.lines {
        g.draw(d, s[0], s[1], s[2], s[3]);
    }
    for dot in &silk.dots {
        let dd = g.circle_ap(dot[2] * 2.0, "Other,Legend");
        g.flash(dd, dot[0], dot[1]);
    }
    let d = g.circle_ap(SILK_W, "Other,Legend");
    for lb in &silk.labels {
        for stroke in text_strokes(&lb.text, lb.x, lb.y, lb.h) {
            g.polyline(d, &stroke);
        }
    }
    g.finish()
}

// ---------------------------------------------------------------------------
// Excellon drill
// ---------------------------------------------------------------------------

pub fn excellon(holes: &[(f64, f64, f64)], plated: bool, project: &str) -> String {
    let mut tools: Vec<f64> = Vec::new();
    for h in holes {
        if !tools.iter().any(|t| (t - h.2).abs() < 1e-6) {
            tools.push(h.2);
        }
    }
    tools.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut s = String::new();
    s += "M48\n";
    s += &format!("; DRILL file — Protoforge PCB Studio — {project}\n");
    s += &format!(
        "; #@! TF.FileFunction,{},1,2,{}\n",
        if plated { "Plated" } else { "NonPlated" },
        if plated { "PTH" } else { "NPTH" }
    );
    s += "; FORMAT={-:-/ absolute / metric / decimal}\n";
    s += "FMAT,2\nMETRIC\n";
    for (i, t) in tools.iter().enumerate() {
        s += &format!("T{}C{:.3}\n", i + 1, t);
    }
    s += "%\nG90\nG05\n";
    for (i, t) in tools.iter().enumerate() {
        s += &format!("T{}\n", i + 1);
        for h in holes.iter().filter(|h| (h.2 - t).abs() < 1e-6) {
            s += &format!("X{:.3}Y{:.3}\n", h.0, h.1);
        }
    }
    s += "T0\nM30\n";
    s
}

// ---------------------------------------------------------------------------
// BOM / CPL
// ---------------------------------------------------------------------------

/// Short value string for the BOM "Comment" column (e.g. "100nF", "4.7kΩ").
pub fn bom_value(c: &Component) -> String {
    let words: Vec<&str> = c.name.split_whitespace().collect();
    let units = [
        ("pF", "pF"),
        ("nF", "nF"),
        ("uF", "uF"),
        ("µF", "uF"),
        ("nH", "nH"),
        ("uH", "uH"),
        ("µH", "uH"),
        ("k", "kΩ"),
        ("M", "MΩ"),
        ("R", "Ω"),
        ("Ω", "Ω"),
    ];
    let passive = c.refdes.starts_with('R') || c.refdes.starts_with('C') || c.refdes.starts_with('L');
    if passive {
        for w in words.windows(2) {
            if w[0].parse::<f64>().is_ok() {
                if let Some((_, u)) = units.iter().find(|(k, _)| *k == w[1]) {
                    return format!("{}{}", w[0], u);
                }
            }
        }
    }
    c.name.clone()
}

fn csv(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn natural_key(s: &str) -> (String, u32) {
    let p: String = s.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let n: u32 = s[p.len()..].parse().unwrap_or(0);
    (p, n)
}

pub fn bom_csv(p: &Project) -> String {
    let mut groups: Vec<(String, String, Vec<String>)> = Vec::new();
    for c in p.components.iter().filter(|c| c.on_pcb) {
        let v = bom_value(c);
        if let Some(g) = groups.iter_mut().find(|g| g.0 == v && g.1 == c.footprint) {
            g.2.push(c.refdes.clone());
        } else {
            groups.push((v, c.footprint.clone(), vec![c.refdes.clone()]));
        }
    }
    let mut s = String::from("Comment,Designator,Footprint,LCSC Part #\n");
    for (v, fp, mut refs) in groups {
        refs.sort_by_key(|r| natural_key(r));
        s += &format!("{},{},{},\n", csv(&v), csv(&refs.join(",")), csv(fp.as_str()));
    }
    s
}

pub fn cpl_csv(l: &Layout) -> String {
    let mut s = String::from("Designator,Mid X,Mid Y,Layer,Rotation\n");
    let mut parts: Vec<_> = l.parts.iter().filter(|p| !p.feature).collect();
    parts.sort_by_key(|p| natural_key(&p.refdes));
    for p in parts {
        s += &format!(
            "{},{:.3}mm,{:.3}mm,{},{}\n",
            p.refdes,
            p.x,
            p.y,
            if p.side == "bottom" { "Bottom" } else { "Top" },
            p.rot.round() as i64
        );
    }
    s
}

fn readme(p: &Project, l: &Layout, name: &str) -> String {
    let r = &l.rules;
    let min_drill = l
        .vias
        .iter()
        .map(|v| v.drill)
        .chain(l.pads.iter().filter(|p| p.drill > 0.0).map(|p| p.drill))
        .chain(l.holes.iter().map(|h| h.d))
        .fold(f64::INFINITY, f64::min);
    let s = &l.stats;
    let mut t = String::new();
    t += &format!("# {} — main board fabrication notes\n\n", p.meta.name);
    t += "Generated by Protoforge PCB Studio. **Review every file in a Gerber viewer before ordering.**\n\n";
    t += "## Board\n\n| Parameter | Value |\n|---|---|\n";
    t += &format!("| Outline | {:.2} × {:.2} mm, rectangular |\n", l.board.width, l.board.height);
    t += &format!("| Layers | {} (F.Cu signals, B.Cu signals + GND pour) |\n", l.board.layers);
    t += &format!("| Thickness | {:.2} mm FR-4 |\n", l.board.thickness);
    t += "| Copper | 1 oz (35 µm) outer layers |\n";
    t +=
        "| Surface finish | ENIG recommended (0.4 mm-pitch QFN and LGA parts need flat pads); lead-free HASL acceptable for prototypes |\n";
    t += "| Solder mask | both sides, any colour; vias tented |\n| Silkscreen | top |\n";
    t += &format!("| Min trace / space used | {:.3} mm / {:.3} mm |\n", r.trace_mm, r.clearance_mm);
    t += &format!(
        "| Via | {:.2} mm drill / {:.2} mm pad ({} vias) |\n",
        r.via_drill_mm, r.via_pad_mm, s.via_count
    );
    t += &format!(
        "| Smallest drill | {:.2} mm |\n",
        if min_drill.is_finite() { min_drill } else { 0.0 }
    );
    t += &format!("| Copper-to-edge | ≥ {:.2} mm |\n\n", r.edge_clearance_mm);
    t += "## Files\n\n| File | Content |\n|---|---|\n";
    for (f, d) in [
        ("-F_Cu.gbr", "Top copper (pads, tracks, vias)"),
        (
            "-B_Cu.gbr",
            "Bottom copper (GND pour as G36/G37 regions, tracks, test pads, touch electrode)",
        ),
        (
            "-F_Mask.gbr / -B_Mask.gbr",
            "Solder-mask openings (negative polarity, +0.05 mm expansion)",
        ),
        ("-F_Paste.gbr", "Stencil apertures (thermal pads windowed)"),
        ("-F_Silkscreen.gbr", "Legend"),
        ("-Edge_Cuts.gbr", "Board outline"),
        ("-PTH.drl / -NPTH.drl", "Excellon drill files (metric)"),
    ] {
        t += &format!("| `{name}{f}` | {d} |\n");
    }
    t += "\n## Assembly (PCBA)\n\n";
    t += &format!("* `{name}-BOM.csv` — columns `Comment,Designator,Footprint,LCSC Part #`. **The part-number column is intentionally empty:** choose parts in the distributor's catalogue that match value, voltage rating, tolerance and package.\n");
    t += &format!("* `{name}-CPL.csv` — `Designator,Mid X,Mid Y,Layer,Rotation` (mm from the bottom-left board corner). Rotation zero-orientations differ between CAD libraries and assembly houses: check every part in the vendor's placement preview and correct rotations there.\n");
    t += &format!(
        "* {} SMD joints, {} through-hole joints, {} unique BOM lines, all parts on the top side.\n\n",
        s.smd_joints, s.tht_joints, s.unique_parts
    );
    t += "## Verification snapshot\n\n";
    t += &format!(
        "* Routed: {}/{} connections ({:.1} %)\n* DRC violations: {}\n* Track length: {:.1} mm, vias: {}\n\n",
        s.routed, s.connections, s.routed_pct, s.drc_violations, s.track_length_mm, s.via_count
    );
    t += "## Engineering notes\n\n";
    for n in &l.notes {
        t += &format!("* {n}\n");
    }
    t += "\nThis layout is an automatically generated, educational design. Before spending money: open the Gerbers in a viewer (gerbv, tracespace, KiCad GerbView or the fab's online viewer), check footprints against datasheets, and consider re-routing critical nets (RF, buck switch node) by hand in KiCad.\n";
    t
}

fn assembly_svg(l: &Layout, title: &str) -> String {
    let s = 12.0; // px per mm
    let (w, h) = (l.board.width, l.board.height);
    let pad = 20.0;
    let (vw, vh) = (w * s + 2.0 * pad, h * s + 2.0 * pad + 24.0);
    let tx = |x: f64| pad + x * s;
    let ty = |y: f64| pad + 24.0 + (h - y) * s;
    let mut o = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{vw:.0}\" height=\"{vh:.0}\" viewBox=\"0 0 {vw:.0} {vh:.0}\" font-family=\"Helvetica, Arial, sans-serif\">\n");
    o += &format!("<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n<text x=\"{pad}\" y=\"16\" font-size=\"13\" font-weight=\"700\">{} — top assembly drawing (mm)</text>\n", xml(title));
    o += &format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"#e9f2ea\" stroke=\"#222\" stroke-width=\"1\"/>\n",
        tx(0.0),
        ty(h),
        w * s,
        h * s
    );
    for p in l.pads.iter().filter(|p| p.top) {
        let (x0, y1) = (tx(p.x - p.w / 2.0), ty(p.y + p.h / 2.0));
        if p.shape == PadShape::Circle {
            o += &format!(
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"#c8a36a\"/>\n",
                tx(p.x),
                ty(p.y),
                p.w / 2.0 * s
            );
        } else {
            o += &format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"#c8a36a\"/>\n",
                x0,
                y1,
                p.w * s,
                p.h * s
            );
        }
    }
    for part in l.parts.iter().filter(|p| !p.feature) {
        let b = part.body;
        o += &format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"#333\" stroke-width=\"0.8\"/>\n",
            tx(b[0]),
            ty(b[3]),
            (b[2] - b[0]) * s,
            (b[3] - b[1]) * s
        );
        o += &format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"1.6\" fill=\"#d0302f\"/>\n",
            tx(part.pin1[0]),
            ty(part.pin1[1])
        );
        let fs = ((b[2] - b[0]).min(b[3] - b[1]) * s * 0.45).clamp(6.0, 11.0);
        o += &format!("<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"{fs:.1}\" text-anchor=\"middle\" dominant-baseline=\"central\" fill=\"#0b3d91\" font-weight=\"700\">{}</text>\n", tx(part.x), ty(part.y), xml(&part.refdes));
    }
    o += "</svg>\n";
    o
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Base name used for the fab files.
pub fn base_name(p: &Project) -> String {
    let n = if p.meta.codename.is_empty() { &p.meta.id } else { &p.meta.codename };
    let s: String = n
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    if s.is_empty() {
        "board".into()
    } else {
        format!("{s}-main")
    }
}

/// Every file needed to order bare boards + assembly.
pub fn fab_files(p: &Project, l: &Layout) -> Vec<FabFile> {
    let name = base_name(p);
    let proj = gerber_escape(&p.meta.name);
    let title = format!(
        "{} {}",
        p.meta.name.to_uppercase(),
        if p.meta.version.is_empty() {
            "v0.1".to_string()
        } else {
            format!("v{}", p.meta.version)
        }
    );
    let silk = silkscreen(l, &title);
    let mut pth: Vec<(f64, f64, f64)> = l.vias.iter().map(|v| (v.x, v.y, v.drill)).collect();
    pth.extend(l.pads.iter().filter(|p| p.drill > 0.0).map(|p| (p.x, p.y, p.drill)));
    let npth: Vec<(f64, f64, f64)> = l.holes.iter().map(|h| (h.x, h.y, h.d)).collect();
    let dir = "gerbers";
    let mut files = vec![
        FabFile {
            path: format!("{dir}/{name}-F_Cu.gbr"),
            content: copper_layer(l, &proj, false),
            kind: "F_Cu".into(),
        },
        FabFile {
            path: format!("{dir}/{name}-B_Cu.gbr"),
            content: copper_layer(l, &proj, true),
            kind: "B_Cu".into(),
        },
        FabFile {
            path: format!("{dir}/{name}-F_Mask.gbr"),
            content: mask_layer(l, &proj, false),
            kind: "F_Mask".into(),
        },
        FabFile {
            path: format!("{dir}/{name}-B_Mask.gbr"),
            content: mask_layer(l, &proj, true),
            kind: "B_Mask".into(),
        },
        FabFile {
            path: format!("{dir}/{name}-F_Paste.gbr"),
            content: paste_layer(l, &proj),
            kind: "F_Paste".into(),
        },
        FabFile {
            path: format!("{dir}/{name}-F_Silkscreen.gbr"),
            content: silk_layer(&proj, &silk),
            kind: "F_Silkscreen".into(),
        },
        FabFile {
            path: format!("{dir}/{name}-Edge_Cuts.gbr"),
            content: edge_layer(l, &proj),
            kind: "Edge_Cuts".into(),
        },
        FabFile {
            path: format!("{dir}/{name}-PTH.drl"),
            content: excellon(&pth, true, &proj),
            kind: "Drill_PTH".into(),
        },
    ];
    if !npth.is_empty() {
        files.push(FabFile {
            path: format!("{dir}/{name}-NPTH.drl"),
            content: excellon(&npth, false, &proj),
            kind: "Drill_NPTH".into(),
        });
    }
    files.push(FabFile {
        path: format!("assembly/{name}-BOM.csv"),
        content: bom_csv(p),
        kind: "BOM".into(),
    });
    files.push(FabFile {
        path: format!("assembly/{name}-CPL.csv"),
        content: cpl_csv(l),
        kind: "CPL".into(),
    });
    files.push(FabFile {
        path: format!("assembly/{name}-assembly-top.svg"),
        content: assembly_svg(l, &title),
        kind: "Assembly".into(),
    });
    let mut nl = String::from("Net,Class,Pins\n");
    for n in &l.nets {
        nl += &format!("{},{:?},{}\n", csv(&n.name), n.class, csv(&n.pins.join(" ")));
    }
    files.push(FabFile {
        path: format!("assembly/{name}-netlist.csv"),
        content: nl,
        kind: "Netlist".into(),
    });
    files.push(FabFile {
        path: format!("{name}-FAB-README.md"),
        content: readme(p, l, &name),
        kind: "README".into(),
    });
    files
}

/// Silkscreen geometry for the web viewer.
pub fn silk_for(p: &Project, l: &Layout) -> Silk {
    let title = format!(
        "{} {}",
        p.meta.name.to_uppercase(),
        if p.meta.version.is_empty() {
            "v0.1".to_string()
        } else {
            format!("v{}", p.meta.version)
        }
    );
    silkscreen(l, &title)
}

// ---------------------------------------------------------------------------
// Validation: a tiny Gerber parser used by the tests.
// ---------------------------------------------------------------------------

#[derive(Debug, Default, PartialEq)]
pub struct GerberStats {
    pub flashes: usize,
    pub draws: usize,
    pub regions: usize,
    pub apertures: usize,
}

/// Parse a Gerber file strictly enough to catch malformed output: every
/// word ends with `*`, extended commands are wrapped in `%…%`, apertures are
/// defined before use, regions are closed, the file ends with `M02*`.
pub fn parse_gerber(src: &str) -> Result<GerberStats, String> {
    let mut st = GerberStats::default();
    let mut defined: Vec<u32> = Vec::new();
    let mut current: Option<u32> = None;
    let mut in_region = false;
    let mut region_pts = 0;
    let (mut fs, mut mo, mut ended) = (false, false, false);
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if ended {
            return Err("content after M02".into());
        }
        if c == '%' {
            let end = src[i + 1..].find('%').ok_or("unterminated extended command")? + i + 1;
            let body = &src[i + 1..end];
            if !body.ends_with('*') {
                return Err(format!("extended command without '*': {body}"));
            }
            let cmd = &body[..body.len() - 1];
            if cmd == "FSLAX46Y46" {
                fs = true;
            } else if cmd == "MOMM" {
                mo = true;
            } else if let Some(rest) = cmd.strip_prefix("ADD") {
                let n: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                let d: u32 = n.parse().map_err(|_| format!("bad aperture {cmd}"))?;
                if d < 10 {
                    return Err("aperture numbers start at D10".into());
                }
                let tmpl = &rest[n.len()..];
                if !(tmpl.starts_with("C,") || tmpl.starts_with("R,") || tmpl.starts_with("O,")) {
                    return Err(format!("unknown aperture template {tmpl}"));
                }
                defined.push(d);
                st.apertures += 1;
            } else if !(cmd.starts_with("TF.")
                || cmd.starts_with("TA.")
                || cmd.starts_with("TO.")
                || cmd.starts_with("TD")
                || cmd == "LPD"
                || cmd == "LPC")
            {
                return Err(format!("unknown extended command {cmd}"));
            }
            i = end + 1;
            continue;
        }
        let end = src[i..].find('*').ok_or("word without '*'")? + i;
        let word = &src[i..end];
        i = end + 1;
        if word.starts_with("G04") {
            continue;
        }
        match word {
            "G01" => continue,
            "G36" => {
                if in_region {
                    return Err("nested G36".into());
                }
                in_region = true;
                region_pts = 0;
                continue;
            }
            "G37" => {
                if !in_region || region_pts < 4 {
                    return Err("G37 without a closed contour".into());
                }
                in_region = false;
                st.regions += 1;
                continue;
            }
            "M02" => {
                ended = true;
                continue;
            }
            _ => {}
        }
        if !fs || !mo {
            return Err("coordinate data before %FS / %MO".into());
        }
        if let Some(n) = word.strip_prefix('D') {
            let d: u32 = n.parse().map_err(|_| format!("bad word {word}"))?;
            if !defined.contains(&d) {
                return Err(format!("D{d} used before definition"));
            }
            current = Some(d);
            continue;
        }
        if word.starts_with('X') {
            let yi = word.find('Y').ok_or(format!("missing Y in {word}"))?;
            let di = word.find('D').ok_or(format!("missing D in {word}"))?;
            word[1..yi].parse::<i64>().map_err(|_| format!("bad X in {word}"))?;
            word[yi + 1..di].parse::<i64>().map_err(|_| format!("bad Y in {word}"))?;
            match &word[di..] {
                "D01" => {
                    if in_region {
                        region_pts += 1;
                    } else {
                        if current.is_none() {
                            return Err("draw without aperture".into());
                        }
                        st.draws += 1;
                    }
                }
                "D02" => {
                    if in_region {
                        region_pts += 1;
                    }
                }
                "D03" => {
                    if in_region {
                        return Err("flash inside region".into());
                    }
                    if current.is_none() {
                        return Err("flash without aperture".into());
                    }
                    st.flashes += 1;
                }
                other => return Err(format!("bad operation {other}")),
            }
            continue;
        }
        return Err(format!("unknown word {word}"));
    }
    if !ended {
        return Err("missing M02".into());
    }
    Ok(st)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gerbers_parse_and_count() {
        let p = Project::iris();
        let l = super::super::layout_with(&p, None);
        let files = fab_files(&p, &l);
        let get = |k: &str| files.iter().find(|f| f.kind == k).unwrap();
        for f in files.iter().filter(|f| f.path.ends_with(".gbr")) {
            parse_gerber(&f.content).unwrap_or_else(|e| panic!("{}: {e}", f.path));
        }
        let top = parse_gerber(&get("F_Cu").content).unwrap();
        let top_pads = l.pads.iter().filter(|p| p.top).count();
        assert_eq!(top.flashes, top_pads + l.vias.len());
        assert_eq!(top.draws, l.tracks.iter().filter(|t| t.layer == 0).count());
        assert_eq!(top.regions, l.pour_top.rects.len());
        let bot = parse_gerber(&get("B_Cu").content).unwrap();
        assert_eq!(bot.regions, l.pour.rects.len());
        assert!(bot.regions > 0);
        let edge = parse_gerber(&get("Edge_Cuts").content).unwrap();
        assert_eq!(edge.draws, 4);
        assert!(get("F_Cu").content.contains("%TF.FileFunction,Copper,L1,Top*%"));
        // drill file lists every via
        let drl = &get("Drill_PTH").content;
        assert!(drl.starts_with("M48") && drl.trim_end().ends_with("M30"));
        let n_holes = drl.lines().filter(|l| l.starts_with('X')).count();
        assert_eq!(n_holes, l.vias.len() + l.pads.iter().filter(|p| p.drill > 0.0).count());
    }

    #[test]
    fn parser_rejects_bad_input() {
        assert!(parse_gerber("%FSLAX46Y46*%\n%MOMM*%\nD10*\nX0Y0D03*\nM02*\n").is_err());
        assert!(parse_gerber("%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nX0Y0D03\nM02*\n").is_err());
        assert!(parse_gerber("%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nX0Y0D03*\n").is_err());
        let ok = parse_gerber("%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nX0Y0D03*\nX0Y0D02*\nX1000Y0D01*\nM02*\n").unwrap();
        assert_eq!((ok.flashes, ok.draws), (1, 1));
    }

    #[test]
    fn bom_and_cpl_have_jlc_columns() {
        let p = Project::iris();
        let l = super::super::layout_with(&p, None);
        let bom = bom_csv(&p);
        assert!(bom.starts_with("Comment,Designator,Footprint,LCSC Part #\n"));
        assert!(bom.contains("\"C3,C4,C6\"") || bom.contains("100nF"));
        let cpl = cpl_csv(&l);
        assert!(cpl.starts_with("Designator,Mid X,Mid Y,Layer,Rotation\n"));
        assert_eq!(cpl.lines().count() - 1, p.components.iter().filter(|c| c.on_pcb).count());
        assert!(!cpl.lines().any(|l| l.starts_with("E1,") || l.starts_with("TP1,")));
    }
}
