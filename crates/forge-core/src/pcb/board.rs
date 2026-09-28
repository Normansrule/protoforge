//! Board model: placed parts with their pads resolved to board coordinates
//! and nets, keep-out regions and non-plated holes.

use serde::{Deserialize, Serialize};

use super::footprints::{footprint, rot_box, rot_pt, Footprint, PadShape};
use super::geom::Shape;
use super::netlist::{self, NetClass, NetDef};
use crate::project::{PcbParams, Project};

/// Design rules derived from `PcbParams` plus fixed fab-capability values.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rules {
    pub trace_mm: f64,
    pub clearance_mm: f64,
    pub via_drill_mm: f64,
    pub via_pad_mm: f64,
    pub grid_mm: f64,
    /// Copper to board edge.
    pub edge_clearance_mm: f64,
    /// Ground pour to other copper.
    pub pour_clearance_mm: f64,
    /// Smallest drill the fab offers.
    pub min_drill_mm: f64,
    /// Smallest annular ring (pad radius − drill radius).
    pub min_annular_mm: f64,
    /// Copper to non-plated hole edge.
    pub hole_clearance_mm: f64,
}

impl Rules {
    pub fn from(p: &PcbParams) -> Rules {
        Rules {
            trace_mm: p.min_trace_mm.max(0.05),
            clearance_mm: p.min_clearance_mm.max(0.05),
            via_drill_mm: p.via_drill_mm.max(0.1),
            via_pad_mm: p.via_pad_mm.max(p.via_drill_mm + 0.1),
            grid_mm: p.grid_mm.clamp(0.1, 1.0),
            edge_clearance_mm: 0.3,
            pour_clearance_mm: p.min_clearance_mm.max(0.2),
            min_drill_mm: 0.2,
            min_annular_mm: 0.1,
            hole_clearance_mm: 0.2,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlacedPart {
    pub refdes: String,
    pub footprint: String,
    pub value: String,
    pub x: f64,
    pub y: f64,
    pub rot: f64,
    /// "top" or "bottom".
    pub side: String,
    /// Board-level copper feature (electrode / test pad) — not a BOM line.
    pub feature: bool,
    pub courtyard: [f64; 4],
    pub body: [f64; 4],
    pub height: f64,
    pub pin1: [f64; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoardPad {
    pub refdes: String,
    pub pin: String,
    /// Net name ("" = unconnected).
    pub net: String,
    #[serde(skip)]
    pub net_id: u16,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub shape: PadShape,
    pub drill: f64,
    pub top: bool,
    pub bottom: bool,
    pub mech: bool,
    pub paste: f64,
}

impl BoardPad {
    pub fn shape_geom(&self) -> Shape {
        match self.shape {
            PadShape::Circle => Shape::circle(self.x, self.y, self.w / 2.0),
            _ => Shape::Rect { cx: self.x, cy: self.y, hw: self.w / 2.0, hh: self.h / 2.0 },
        }
    }
    pub fn is_smd(&self) -> bool {
        self.drill <= 0.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Keepout {
    pub name: String,
    pub rect: [f64; 4],
    pub top: bool,
    pub bottom: bool,
    /// Net allowed inside (e.g. the RF feed), "" = none.
    pub allow_net: String,
    /// Also excludes the ground pour.
    pub no_pour: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hole {
    pub x: f64,
    pub y: f64,
    pub d: f64,
    pub refdes: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetInfo {
    pub name: String,
    pub class: NetClass,
    pub pins: Vec<String>,
    pub current_a: f64,
    pub description: String,
}

#[derive(Clone, Debug)]
pub struct Board {
    pub width: f64,
    pub height: f64,
    pub rules: Rules,
    pub parts: Vec<PlacedPart>,
    pub pads: Vec<BoardPad>,
    pub nets: Vec<NetInfo>,
    pub keepouts: Vec<Keepout>,
    pub holes: Vec<Hole>,
    /// Net id of GND (u16::MAX if absent).
    pub gnd: u16,
    /// Total number of net ids (real nets + one pseudo net per unconnected pad).
    pub net_count: u16,
}

/// A placement decision for one part (component or board feature).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PartPos {
    pub refdes: String,
    pub x: f64,
    pub y: f64,
    pub rot: f64,
    #[serde(default = "top")]
    pub side: String,
}

fn top() -> String {
    "top".into()
}

/// Footprints for board features (not in the footprint library).
pub fn feature_footprint(refdes: &str) -> Option<Footprint> {
    use super::footprints::PadDef;
    let pad = |w: f64, h: f64, shape: PadShape| PadDef { num: "1".into(), x: 0.0, y: 0.0, w, h, shape, drill: 0.0, paste: 0.0, mech: false };
    match refdes {
        "E1" => Some(Footprint {
            name: "TOUCH_PAD_7x4.5".into(),
            description: "Self-capacitance touch electrode on the bottom copper (faces the pod wall)".into(),
            pads: vec![pad(7.0, 4.5, PadShape::Rect)],
            courtyard: [-3.75, -2.5, 3.75, 2.5],
            body: [-3.5, -2.25, 3.5, 2.25],
            height: 0.0,
            pin1: [-3.5, 2.25],
            npth: vec![],
            note: "Ground pour is kept 0.5 mm away so the electrode's self-capacitance stays small.".into(),
        }),
        "TP1" | "TP2" => Some(Footprint {
            name: "TESTPAD_D1.0".into(),
            description: "Ø1.0 mm bare-copper test pad for pogo-pin SWD programming".into(),
            pads: vec![pad(1.0, 1.0, PadShape::Circle)],
            courtyard: [-0.6, -0.6, 0.6, 0.6],
            body: [-0.5, -0.5, 0.5, 0.5],
            height: 0.0,
            pin1: [0.0, 0.0],
            npth: vec![],
            note: "".into(),
        }),
        _ => None,
    }
}

/// Footprint of a component or board feature.
pub fn part_footprint(p: &Project, refdes: &str) -> Option<Footprint> {
    if let Some(f) = feature_footprint(refdes) {
        return Some(f);
    }
    let c = p.components.iter().find(|c| c.refdes == refdes && c.on_pcb)?;
    footprint(&c.footprint)
}

/// Transform a footprint-local box into board coordinates.
pub fn place_box(b: [f64; 4], x: f64, y: f64, rot: f64) -> [f64; 4] {
    let r = rot_box(b, rot);
    [r[0] + x, r[1] + y, r[2] + x, r[3] + y]
}

/// Resolve placement + GPIO assignment into a full board model.
pub fn build(p: &Project, pos: &[PartPos], gpio: &[(String, String)]) -> Board {
    let rules = Rules::from(&p.design.pcb);
    let (w, h) = (p.design.pcb.width_mm, p.design.pcb.height_mm);
    let mut parts = Vec::new();
    let mut fps = Vec::new();
    for pp in pos {
        let Some(fp) = part_footprint(p, &pp.refdes) else { continue };
        let feature = feature_footprint(&pp.refdes).is_some();
        let value = if feature {
            fp.description.clone()
        } else {
            p.components.iter().find(|c| c.refdes == pp.refdes).map(|c| c.name.clone()).unwrap_or_default()
        };
        let (px, py) = rot_pt(fp.pin1[0], fp.pin1[1], pp.rot);
        parts.push(PlacedPart {
            refdes: pp.refdes.clone(),
            footprint: fp.name.clone(),
            value,
            x: pp.x,
            y: pp.y,
            rot: pp.rot,
            side: pp.side.clone(),
            feature,
            courtyard: place_box(fp.courtyard, pp.x, pp.y, pp.rot),
            body: place_box(fp.body, pp.x, pp.y, pp.rot),
            height: fp.height,
            pin1: [pp.x + px, pp.y + py],
        });
        fps.push(fp);
    }
    let present = |r: &str, pin: &str| -> bool {
        parts.iter().zip(fps.iter()).any(|(pp, fp)| pp.refdes == r && fp.pads.iter().any(|pd| pd.num == pin))
    };
    let defs: Vec<NetDef> = netlist::build_nets(p, &present, gpio);
    let nets: Vec<NetInfo> = defs
        .iter()
        .map(|d| NetInfo {
            name: d.name.clone(),
            class: d.class,
            pins: d.pins.iter().map(|(r, pin)| format!("{r}.{pin}")).collect(),
            current_a: d.current_a,
            description: d.description.clone(),
        })
        .collect();
    let mut next_id = nets.len() as u16;
    let mut pads = Vec::new();
    let mut holes = Vec::new();
    for (pp, fp) in parts.iter().zip(fps.iter()) {
        let bottom_side = pp.side == "bottom";
        for pd in &fp.pads {
            let (dx, dy) = rot_pt(pd.x, pd.y, pp.rot);
            let (pw, ph) = if ((pp.rot.round() as i64).rem_euclid(180)) == 90 { (pd.h, pd.w) } else { (pd.w, pd.h) };
            let net_idx = defs.iter().position(|d| d.pins.iter().any(|(r, pin)| *r == pp.refdes && *pin == pd.num));
            let (net, net_id) = match net_idx {
                Some(i) => (defs[i].name.clone(), i as u16),
                None => {
                    next_id += 1;
                    (String::new(), next_id - 1)
                }
            };
            let tht = pd.drill > 0.0;
            pads.push(BoardPad {
                refdes: pp.refdes.clone(),
                pin: pd.num.clone(),
                net,
                net_id,
                x: pp.x + dx,
                y: pp.y + dy,
                w: pw,
                h: ph,
                shape: pd.shape,
                drill: pd.drill,
                top: tht || !bottom_side,
                bottom: tht || bottom_side,
                mech: pd.mech,
                paste: pd.paste,
            });
        }
        for hle in &fp.npth {
            let (dx, dy) = rot_pt(hle[0], hle[1], pp.rot);
            holes.push(Hole { x: pp.x + dx, y: pp.y + dy, d: hle[2], refdes: pp.refdes.clone() });
        }
    }
    let gnd = nets.iter().position(|n| n.name == "GND").map(|i| i as u16).unwrap_or(u16::MAX);
    // Antenna keep-out: no copper on any layer except the RF feed on top.
    let mut keepouts = Vec::new();
    for pp in parts.iter().filter(|pp| pp.footprint.starts_with("ANT")) {
        let c = pp.courtyard;
        keepouts.push(Keepout {
            name: format!("{} antenna keep-out", pp.refdes),
            rect: [(c[0] - 0.8).max(0.0), (c[1] - 1.0).max(0.0), (c[2] + 0.4).min(w), h],
            top: true,
            bottom: true,
            allow_net: "RF".into(),
            no_pour: true,
        });
    }
    for pp in parts.iter().filter(|pp| pp.refdes == "E1") {
        let b = pp.body;
        keepouts.push(Keepout {
            name: "Touch electrode pour clearance".into(),
            rect: [b[0] - 0.5, b[1] - 0.5, b[2] + 0.5, b[3] + 0.5],
            top: false,
            bottom: true,
            allow_net: "TOUCH".into(),
            no_pour: true,
        });
    }
    Board { width: w, height: h, rules, parts, pads, nets, keepouts, holes, gnd, net_count: next_id }
}
