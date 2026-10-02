//! Footprint (land-pattern) library for every footprint name used by the Iris
//! project. Dimensions are *typical* approximations of common IPC-7351-style
//! land patterns for these package families — always verify against the part
//! datasheet's recommended land pattern (or generate one with an IPC-7351
//! calculator / the KiCad library) before ordering boards.
//!
//! Coordinates are millimetres in the footprint's local frame, origin at the
//! package centre, +y up, rotation 0. Pin 1 conventions follow the usual
//! top-view, counter-clockwise numbering.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PadShape {
    Rect,
    RoundRect,
    Circle,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PadDef {
    /// Pin number / name ("1", "EP", "MP1").
    pub num: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub shape: PadShape,
    /// Plated drill diameter (0 = SMD pad on the top layer only).
    pub drill: f64,
    /// Fraction of the pad area that receives solder paste (large thermal
    /// pads get a windowed stencil opening to avoid floating the part).
    pub paste: f64,
    /// Mechanical / mounting pad (no electrical pin).
    pub mech: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Footprint {
    pub name: String,
    pub description: String,
    pub pads: Vec<PadDef>,
    /// Courtyard `[xmin, ymin, xmax, ymax]` — the keep-away zone used for
    /// placement legality (IPC-7351 "courtyard excess" around pads and body).
    pub courtyard: [f64; 4],
    /// Package body outline `[xmin, ymin, xmax, ymax]`.
    pub body: [f64; 4],
    /// Body height, for the 3-D preview.
    pub height: f64,
    /// Pin-1 marker position (silkscreen dot).
    pub pin1: [f64; 2],
    /// Non-plated holes `[x, y, diameter]`.
    pub npth: Vec<[f64; 3]>,
    /// Land-pattern note shown in the library browser.
    pub note: String,
}

fn smd(num: impl Into<String>, x: f64, y: f64, w: f64, h: f64) -> PadDef {
    PadDef {
        num: num.into(),
        x,
        y,
        w,
        h,
        shape: PadShape::Rect,
        drill: 0.0,
        paste: 1.0,
        mech: false,
    }
}
fn rr(num: impl Into<String>, x: f64, y: f64, w: f64, h: f64) -> PadDef {
    PadDef {
        shape: PadShape::RoundRect,
        ..smd(num, x, y, w, h)
    }
}
fn tht(num: impl Into<String>, x: f64, y: f64, d: f64, drill: f64) -> PadDef {
    PadDef {
        num: num.into(),
        x,
        y,
        w: d,
        h: d,
        shape: PadShape::Circle,
        drill,
        paste: 0.0,
        mech: false,
    }
}
fn mech(mut p: PadDef) -> PadDef {
    p.mech = true;
    p
}

/// Every footprint in the library.
pub const NAMES: &[&str] = &[
    "QFN-48_6x6",
    "LGA-14_3x2.5",
    "MIC_LGA_3.5x2.65",
    "SOT-23-5",
    "SOT-23-6",
    "DFN-6_2x2",
    "USON-8_2x3",
    "0402",
    "0603",
    "0805",
    "USB-C_16P",
    "FPC-24_0.5",
    "JST-SH-2",
    "ANT_3216",
    "XTAL-4_2x1.6",
];

pub fn library() -> Vec<Footprint> {
    NAMES.iter().filter_map(|n| footprint(n)).collect()
}

/// Look up a footprint by name.
pub fn footprint(name: &str) -> Option<Footprint> {
    let f = match name {
        "QFN-48_6x6" => {
            // 0.4 mm pitch, 12 pins per side, pads 0.2 × 0.8 mm centred 2.9 mm
            // from the package centre, 4.6 mm exposed pad (EP) tied to GND.
            let mut pads = Vec::new();
            for k in 0..12 {
                let o = -2.2 + 0.4 * k as f64;
                pads.push(smd((1 + k).to_string(), -2.9, -o, 0.8, 0.2));
                pads.push(smd((13 + k).to_string(), o, -2.9, 0.2, 0.8));
                pads.push(smd((25 + k).to_string(), 2.9, o, 0.8, 0.2));
                pads.push(smd((37 + k).to_string(), -o, 2.9, 0.2, 0.8));
            }
            let mut ep = smd("EP", 0.0, 0.0, 4.6, 4.6);
            ep.paste = 0.55;
            pads.push(ep);
            pads.sort_by_key(|p| p.num.parse::<u32>().unwrap_or(99));
            Footprint {
                name: name.into(),
                description: "QFN-48, 6 × 6 mm body, 0.4 mm pitch, exposed thermal pad".into(),
                pads,
                courtyard: [-3.55, -3.55, 3.55, 3.55],
                body: [-3.0, -3.0, 3.0, 3.0],
                height: 0.85,
                pin1: [-3.55, 2.75],
                npth: vec![],
                note: "0.4 mm pitch needs 0.15/0.15 mm trace/space or better and benefits from ENIG finish; stencil the EP in windows (~55 % coverage).".into(),
            }
        }
        "LGA-14_3x2.5" => {
            // 3.0 × 2.5 mm LGA, 0.5 mm pitch: 3 pads on each short side, 4 on each long side.
            let mut pads = Vec::new();
            for (k, y) in [0.5, 0.0, -0.5].iter().enumerate() {
                pads.push(smd((1 + k).to_string(), -1.35, *y, 0.5, 0.25));
            }
            for (k, x) in [-0.75, -0.25, 0.25, 0.75].iter().enumerate() {
                pads.push(smd((4 + k).to_string(), *x, -1.1, 0.25, 0.6));
            }
            for (k, y) in [-0.5, 0.0, 0.5].iter().enumerate() {
                pads.push(smd((8 + k).to_string(), 1.35, *y, 0.5, 0.25));
            }
            for (k, x) in [0.75, 0.25, -0.25, -0.75].iter().enumerate() {
                pads.push(smd((11 + k).to_string(), *x, 1.1, 0.25, 0.6));
            }
            Footprint {
                name: name.into(),
                description: "LGA-14, 3.0 × 2.5 mm, 0.5 mm pitch (typical 6-axis IMU package)".into(),
                pads,
                courtyard: [-1.85, -1.65, 1.85, 1.65],
                body: [-1.5, -1.25, 1.5, 1.25],
                height: 0.86,
                pin1: [-1.85, 1.1],
                npth: vec![],
                note: "LGA pads sit under the body — X-ray or careful reflow profile recommended.".into(),
            }
        }
        "MIC_LGA_3.5x2.65" => {
            // Bottom-port MEMS microphone: 4 signal pads + a GND ring around the
            // acoustic port, modelled as a round pad with a plated 0.5 mm port hole.
            let mut ring = tht("5", 0.0, 0.0, 1.3, 0.5);
            ring.paste = 0.0;
            Footprint {
                name: name.into(),
                description: "Bottom-port PDM MEMS microphone, 3.5 × 2.65 mm LGA".into(),
                pads: vec![
                    smd("1", -1.15, 0.8, 0.5, 0.6),
                    smd("2", -1.15, -0.8, 0.5, 0.6),
                    smd("3", 1.15, -0.8, 0.5, 0.6),
                    smd("4", 1.15, 0.8, 0.5, 0.6),
                    ring,
                ],
                courtyard: [-2.0, -1.575, 2.0, 1.575],
                body: [-1.75, -1.325, 1.75, 1.325],
                height: 1.0,
                pin1: [-2.0, 1.3],
                npth: vec![],
                note: "The acoustic port must go through the PCB (and the enclosure). Keep solder paste off the port ring's inner edge.".into(),
            }
        }
        "SOT-23-5" | "SOT-23-6" => {
            let six = name.ends_with('6');
            let mut pads = vec![smd("1", -0.95, -1.15, 0.6, 1.05), smd("2", 0.0, -1.15, 0.6, 1.05), smd("3", 0.95, -1.15, 0.6, 1.05)];
            if six {
                pads.push(smd("4", 0.95, 1.15, 0.6, 1.05));
                pads.push(smd("5", 0.0, 1.15, 0.6, 1.05));
                pads.push(smd("6", -0.95, 1.15, 0.6, 1.05));
            } else {
                pads.push(smd("4", 0.95, 1.15, 0.6, 1.05));
                pads.push(smd("5", -0.95, 1.15, 0.6, 1.05));
            }
            Footprint {
                name: name.into(),
                description: format!("SOT-23-{}, 0.95 mm pitch", if six { 6 } else { 5 }),
                pads,
                courtyard: [-1.8, -1.95, 1.8, 1.95],
                body: [-1.45, -0.8, 1.45, 0.8],
                height: 1.1,
                pin1: [-1.6, -1.9],
                npth: vec![],
                note: "Classic gull-wing package; easy to hand-solder.".into(),
            }
        }
        "DFN-6_2x2" => {
            let mut ep = smd("EP", 0.0, 0.0, 0.8, 1.5);
            ep.paste = 0.6;
            Footprint {
                name: name.into(),
                description: "DFN-6, 2 × 2 mm, 0.65 mm pitch, exposed pad".into(),
                pads: vec![
                    smd("1", -1.0, 0.65, 0.6, 0.3),
                    smd("2", -1.0, 0.0, 0.6, 0.3),
                    smd("3", -1.0, -0.65, 0.6, 0.3),
                    smd("4", 1.0, -0.65, 0.6, 0.3),
                    smd("5", 1.0, 0.0, 0.6, 0.3),
                    smd("6", 1.0, 0.65, 0.6, 0.3),
                    ep,
                ],
                courtyard: [-1.55, -1.3, 1.55, 1.3],
                body: [-1.0, -1.0, 1.0, 1.0],
                height: 0.75,
                pin1: [-1.5, 1.2],
                npth: vec![],
                note: "Keep the buck's switch-node copper small (EMI) and the input capacitor right at VIN/GND.".into(),
            }
        }
        "USON-8_2x3" => {
            let mut pads = Vec::new();
            for (k, y) in [0.75, 0.25, -0.25, -0.75].iter().enumerate() {
                pads.push(smd((1 + k).to_string(), -0.95, *y, 0.6, 0.25));
            }
            for (k, y) in [-0.75, -0.25, 0.25, 0.75].iter().enumerate() {
                pads.push(smd((5 + k).to_string(), 0.95, *y, 0.6, 0.25));
            }
            Footprint {
                name: name.into(),
                description: "USON-8, 2 × 3 mm, 0.5 mm pitch (serial NOR flash)".into(),
                pads,
                courtyard: [-1.5, -1.75, 1.5, 1.75],
                body: [-1.0, -1.5, 1.0, 1.5],
                height: 0.55,
                pin1: [-1.45, 1.15],
                npth: vec![],
                note: "Some USON flash parts have a centre pad that must be left unconnected or tied to GND — check the datasheet.".into(),
            }
        }
        "0402" => chip(name, 0.5, 0.5, 0.55, [-0.95, -0.5, 0.95, 0.5], [-0.5, -0.25, 0.5, 0.25], 0.35),
        "0603" => chip(name, 0.8, 0.8, 0.95, [-1.5, -0.75, 1.5, 0.75], [-0.8, -0.4, 0.8, 0.4], 0.45),
        "0805" => chip(name, 0.95, 1.0, 1.35, [-1.75, -0.95, 1.75, 0.95], [-1.0, -0.625, 1.0, 0.625], 0.9),
        "USB-C_16P" => {
            // Simplified 16-pin USB Type-C receptacle, mating face towards +x.
            // Pads 12 in a row (doubled contacts share a pad), shell tabs modelled
            // as round plated holes (the real part uses slots) and two NPTH pegs.
            let x = -3.1;
            let (l, wide, thin) = (1.15, 0.6, 0.3);
            let mut pads = vec![
                smd("A1B12", x, 3.2, l, wide),
                smd("A4B9", x, 2.4, l, wide),
                smd("B8", x, 1.75, l, thin),
                smd("A5", x, 1.25, l, thin),
                smd("B7", x, 0.75, l, thin),
                smd("A6", x, 0.25, l, thin),
                smd("A7", x, -0.25, l, thin),
                smd("B6", x, -0.75, l, thin),
                smd("A8", x, -1.25, l, thin),
                smd("B5", x, -1.75, l, thin),
                smd("B4A9", x, -2.4, l, wide),
                smd("B1A12", x, -3.2, l, wide),
            ];
            for (k, (sx, sy)) in [(-1.7, 4.0), (-1.7, -4.0), (1.9, 4.0), (1.9, -4.0)].iter().enumerate() {
                pads.push(tht(format!("S{}", k + 1), *sx, *sy, 1.0, 0.6));
            }
            Footprint {
                name: name.into(),
                description: "USB Type-C receptacle, 16-pin SMD (power + USB 2.0), simplified".into(),
                pads,
                courtyard: [-4.0, -4.7, 3.9, 4.7],
                body: [-3.7, -4.47, 3.65, 4.47],
                height: 3.2,
                pin1: [-4.0, 3.2],
                npth: vec![[-1.6, 2.9, 0.65], [-1.6, -2.9, 0.65]],
                note: "Shell tabs are slotted on real parts (use the vendor's footprint). A power-only sink still needs 5.1 kΩ Rd pull-downs on CC1 and CC2.".into(),
            }
        }
        "FPC-24_0.5" => {
            let mut pads = Vec::new();
            for k in 0..24 {
                pads.push(smd((k + 1).to_string(), -5.75 + 0.5 * k as f64, 0.9, 0.3, 1.2));
            }
            pads.push(mech(smd("MP1", -7.6, -1.2, 1.8, 2.0)));
            pads.push(mech(smd("MP2", 7.6, -1.2, 1.8, 2.0)));
            Footprint {
                name: name.into(),
                description: "24-way 0.5 mm pitch FPC/FFC connector, bottom contact, cable enters from −y".into(),
                pads,
                courtyard: [-8.8, -2.6, 8.8, 1.8],
                body: [-8.3, -2.4, 8.3, 1.0],
                height: 2.0,
                pin1: [-5.75, 1.75],
                npth: vec![],
                note: "Check contact side (top/bottom) against the flex stack-up — a mirrored flex is the classic FPC mistake.".into(),
            }
        }
        "JST-SH-2" => Footprint {
            name: name.into(),
            description: "JST SH 1.0 mm pitch 2-pin horizontal SMD header".into(),
            pads: vec![
                smd("1", -0.5, -1.3, 0.6, 1.55),
                smd("2", 0.5, -1.3, 0.6, 1.55),
                mech(smd("MP1", -1.8, 1.2, 1.2, 1.8)),
                mech(smd("MP2", 1.8, 1.2, 1.2, 1.8)),
            ],
            courtyard: [-2.7, -2.4, 2.7, 2.4],
            body: [-2.0, -1.0, 2.0, 2.2],
            height: 2.9,
            pin1: [-0.9, -2.3],
            npth: vec![],
            note: "Polarity matters: LiPo leads are not standardised — check red = pin 1 before plugging in.".into(),
        },
        "ANT_3216" => Footprint {
            name: name.into(),
            description: "2.4 GHz ceramic chip antenna, 3.2 × 1.6 mm".into(),
            pads: vec![rr("1", -1.1, 0.0, 1.0, 1.6), mech(rr("2", 1.1, 0.0, 1.0, 1.6))],
            courtyard: [-1.85, -1.05, 1.85, 1.05],
            body: [-1.6, -0.8, 1.6, 0.8],
            height: 1.1,
            pin1: [-1.85, 0.95],
            npth: vec![],
            note: "Needs a copper keep-out (no ground on any layer) around and under the antenna and a π matching network tuned on the real board.".into(),
        },
        "XTAL-4_2x1.6" => Footprint {
            name: name.into(),
            description: "Quartz crystal, 2.0 × 1.6 mm ceramic SMD package, 4 pads (crystal on 1–3, case on 2–4)".into(),
            pads: vec![
                smd("1", -0.65, -0.5, 0.65, 0.55),
                smd("2", 0.65, -0.5, 0.65, 0.55),
                smd("3", 0.65, 0.5, 0.65, 0.55),
                smd("4", -0.65, 0.5, 0.65, 0.55),
            ],
            courtyard: [-1.25, -1.05, 1.25, 1.05],
            body: [-1.0, -0.8, 1.0, 0.8],
            height: 0.5,
            pin1: [-1.2, -0.9],
            npth: vec![],
            note: "Typical 2016 land pattern; pin numbering and which pads carry the crystal vary by maker — check the datasheet. Keep the crystal and its load capacitors next to the SoC's XC pins with ground under them.".into(),
        },
        _ => return None,
    };
    Some(f)
}

fn chip(name: &str, px: f64, pw: f64, ph: f64, cy: [f64; 4], body: [f64; 4], height: f64) -> Footprint {
    Footprint {
        name: name.into(),
        description: format!("{name} chip resistor / capacitor / inductor (imperial size code)"),
        pads: vec![rr("1", -px, 0.0, pw, ph), rr("2", px, 0.0, pw, ph)],
        courtyard: cy,
        body,
        height,
        pin1: [cy[0], cy[3]],
        npth: vec![],
        note: format!("{name}: {} × {} mm body.", body[2] * 2.0, body[3] * 2.0),
    }
}

/// Rotate a local point by a multiple of 90° (counter-clockwise).
pub fn rot_pt(x: f64, y: f64, rot_deg: f64) -> (f64, f64) {
    match ((rot_deg.round() as i64).rem_euclid(360)) / 90 {
        1 => (-y, x),
        2 => (-x, -y),
        3 => (y, -x),
        _ => (x, y),
    }
}

/// Rotate a local box by a multiple of 90°.
pub fn rot_box(b: [f64; 4], rot_deg: f64) -> [f64; 4] {
    let (x0, y0) = rot_pt(b[0], b[1], rot_deg);
    let (x1, y1) = rot_pt(b[2], b[3], rot_deg);
    [x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_iris_footprint_exists() {
        let p = crate::project::Project::iris();
        for c in p.components.iter().filter(|c| c.on_pcb) {
            assert!(footprint(&c.footprint).is_some(), "missing footprint {}", c.footprint);
        }
    }

    #[test]
    fn pads_inside_courtyard_and_clear_of_each_other() {
        for f in library() {
            for (i, a) in f.pads.iter().enumerate() {
                assert!(
                    a.x - a.w / 2.0 >= f.courtyard[0] - 1e-9 && a.x + a.w / 2.0 <= f.courtyard[2] + 1e-9,
                    "{} pad {}",
                    f.name,
                    a.num
                );
                assert!(
                    a.y - a.h / 2.0 >= f.courtyard[1] - 1e-9 && a.y + a.h / 2.0 <= f.courtyard[3] + 1e-9,
                    "{} pad {}",
                    f.name,
                    a.num
                );
                for b in f.pads.iter().skip(i + 1) {
                    let gx = ((a.x - b.x).abs() - (a.w + b.w) / 2.0).max(0.0);
                    let gy = ((a.y - b.y).abs() - (a.h + b.h) / 2.0).max(0.0);
                    let gap = gx.hypot(gy);
                    // Pads of the USB shell share GND; otherwise at least 0.15 mm.
                    assert!(
                        gap >= 0.15 - 1e-9 || (a.num.starts_with('S') && b.num.starts_with('S')),
                        "{}: {}-{} gap {gap}",
                        f.name,
                        a.num,
                        b.num
                    );
                }
            }
        }
    }

    #[test]
    fn qfn_has_48_pins_plus_ep() {
        let f = footprint("QFN-48_6x6").unwrap();
        assert_eq!(f.pads.len(), 49);
        let p1 = f.pads.iter().find(|p| p.num == "1").unwrap();
        assert!(p1.x < 0.0 && p1.y > 2.0);
        let p13 = f.pads.iter().find(|p| p.num == "13").unwrap();
        assert!(p13.y < -2.0 && p13.x < -2.0);
    }
}
