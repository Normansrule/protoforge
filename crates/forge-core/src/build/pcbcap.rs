//! PCB fabrication capability tiers and a design-for-manufacture (DFM)
//! checker against them.
//!
//! The tiers are **generic, typical** figures for the kind of service each
//! name describes — they are not any one vendor's numbers. Every fab
//! publishes a capabilities page; limits differ with layer count, copper
//! weight and price option. Check it before ordering.
//!
//! The checker reads the board parameters (`design.pcb`) and the routed
//! layout from the PCB Studio (`crate::pcb::layout`, read-only): the finest
//! track, via and plated-hole drills, annular rings, pad pitch and the gap
//! between pads (solder-mask web).
//!
//! References: IPC-2221 *Generic Standard on Printed Board Design*;
//! IPC-6012 *Qualification and Performance Specification for Rigid Printed
//! Boards*; IPC-A-600 *Acceptability of Printed Boards*.

use serde::{Deserialize, Serialize};

use crate::project::Project;

#[derive(Clone, Debug, Serialize)]
pub struct Tier {
    pub id: &'static str,
    pub name: &'static str,
    pub trace_mm: f64,
    pub space_mm: f64,
    /// Smallest mechanical drill.
    pub drill_mm: f64,
    pub annular_mm: f64,
    pub edge_mm: f64,
    /// Smallest solder-mask web between pads.
    pub mask_web_mm: f64,
    pub max_layers: u32,
    pub min_thickness_mm: f64,
    pub max_thickness_mm: f64,
    pub finishes: &'static [&'static str],
    pub via_in_pad: bool,
    pub impedance: &'static str,
    pub notes: &'static str,
}

pub const TIERS: &[Tier] = &[
    Tier {
        id: "basic",
        name: "Basic prototype",
        trace_mm: 0.15,
        space_mm: 0.15,
        drill_mm: 0.3,
        annular_mm: 0.15,
        edge_mm: 0.3,
        mask_web_mm: 0.1,
        max_layers: 2,
        min_thickness_mm: 0.8,
        max_thickness_mm: 1.6,
        finishes: &["HASL"],
        via_in_pad: false,
        impedance: "not offered",
        notes: "Cheapest pooled / economy option: 6 mil (0.15 mm) rules, 2 layers.",
    },
    Tier {
        id: "standard",
        name: "Standard",
        trace_mm: 0.127,
        space_mm: 0.127,
        drill_mm: 0.3,
        annular_mm: 0.13,
        edge_mm: 0.25,
        mask_web_mm: 0.1,
        max_layers: 6,
        min_thickness_mm: 0.4,
        max_thickness_mm: 2.0,
        finishes: &["HASL", "lead-free HASL", "ENIG"],
        via_in_pad: false,
        impedance: "optional on multilayer",
        notes: "Typical low-cost production service: 5 mil (0.127 mm) rules, ENIG available.",
    },
    Tier {
        id: "advanced",
        name: "Advanced",
        trace_mm: 0.1,
        space_mm: 0.1,
        drill_mm: 0.2,
        annular_mm: 0.1,
        edge_mm: 0.2,
        mask_web_mm: 0.075,
        max_layers: 12,
        min_thickness_mm: 0.4,
        max_thickness_mm: 3.2,
        finishes: &["lead-free HASL", "ENIG", "OSP", "immersion silver", "ENEPIG"],
        via_in_pad: true,
        impedance: "yes, with test coupons",
        notes: "4 mil (0.1 mm) rules, 0.2 mm drills, filled and capped via-in-pad.",
    },
    Tier {
        id: "hdi",
        name: "High-density interconnect (HDI)",
        trace_mm: 0.075,
        space_mm: 0.075,
        drill_mm: 0.1,
        annular_mm: 0.075,
        edge_mm: 0.15,
        mask_web_mm: 0.06,
        max_layers: 20,
        min_thickness_mm: 0.3,
        max_thickness_mm: 3.2,
        finishes: &["ENIG", "ENEPIG", "OSP", "immersion silver"],
        via_in_pad: true,
        impedance: "yes",
        notes: "3 mil rules and laser-drilled microvias — phone-class boards, highest cost.",
    },
];

/// IPC performance classes (IPC-6011 / IPC-6012 for bare boards, IPC-A-600
/// and IPC-A-610 for acceptability). The class is a quality level ordered
/// from the fab and the assembler — it tightens inspection and acceptance
/// criteria (annular-ring breakout, plating thickness, cosmetic defects),
/// independent of the capability tier. Concepts only; the numbers live in
/// the standards.
#[derive(Clone, Debug, Serialize)]
pub struct IpcClass {
    pub class: u8,
    pub name: &'static str,
    pub typical: &'static str,
    pub effect: &'static str,
}

pub const IPC_CLASSES: &[IpcClass] = &[
    IpcClass {
        class: 1,
        name: "General electronic products",
        typical: "Toys, simple gadgets — function is what matters",
        effect: "Most relaxed acceptance; cosmetic imperfections and some annular-ring breakout accepted",
    },
    IpcClass {
        class: 2,
        name: "Dedicated service electronic products",
        typical: "Consumer and communication products, laptops, wearables — the default for most fabs and for Iris",
        effect: "Longer life and uninterrupted service desired; tighter plating and annular-ring rules than Class 1",
    },
    IpcClass {
        class: 3,
        name: "High reliability electronic products",
        typical: "Medical life support, aerospace, military — equipment that must work on demand",
        effect: "Strictest acceptance, more inspection and test coupons; costs more and not every fab offers it",
    },
];

pub fn tier(id: &str) -> &'static Tier {
    TIERS.iter().find(|t| t.id == id).unwrap_or(&TIERS[1])
}

#[derive(Clone, Debug, Serialize)]
pub struct DfmItem {
    pub rule: String,
    pub design: f64,
    pub limit: f64,
    pub unit: &'static str,
    /// "pass" | "fail" | "warn" | "info"
    pub status: &'static str,
    pub source: &'static str,
    pub note: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Dfm {
    pub tier: &'static Tier,
    pub items: Vec<DfmItem>,
    pub violations: usize,
    pub warnings: usize,
    /// The least capable tier with zero violations.
    pub cheapest_ok: Option<&'static str>,
    pub has_layout: bool,
    pub min_pitch_mm: Option<f64>,
}

/// Measured board features (from the routed layout when available).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Measured {
    pub min_track_mm: Option<f64>,
    pub min_via_drill_mm: Option<f64>,
    pub min_via_annular_mm: Option<f64>,
    pub min_pth_drill_mm: Option<f64>,
    pub min_pth_annular_mm: Option<f64>,
    pub min_pad_gap_mm: Option<f64>,
    pub via_count: usize,
    /// Vias inside small (signal) SMD pads.
    pub vias_in_pads: usize,
    /// Vias inside large exposed / thermal pads (≥ 2 mm²).
    pub thermal_vias: usize,
    pub has_layout: bool,
}

fn fmin(a: Option<f64>, b: f64) -> Option<f64> {
    Some(a.map_or(b, |x| x.min(b)))
}

/// Finest SMD pad pitch on the board (centre distance between neighbouring pads of one part).
pub fn min_pitch_mm(p: &Project) -> Option<f64> {
    let mut best: Option<f64> = None;
    for c in p.components.iter().filter(|c| c.on_pcb) {
        let Some(fp) = crate::pcb::footprints::footprint(&c.footprint) else {
            continue;
        };
        let pads: Vec<_> = fp.pads.iter().filter(|q| q.drill <= 0.0 && !q.mech).collect();
        for (i, a) in pads.iter().enumerate() {
            for b in &pads[i + 1..] {
                let d = ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt();
                // pitch is only meaningful between same-sized pads in a row
                if d > 1e-6 && (a.w - b.w).abs() < 1e-6 && (a.h - b.h).abs() < 1e-6 {
                    best = Some(best.map_or(d, |x: f64| x.min(d)));
                }
            }
        }
    }
    best.map(|x| (x * 1000.0).round() / 1000.0)
}

/// Read the routed layout (PCB Studio) for the finest features.
pub fn measure(p: &Project) -> Measured {
    let mut m = Measured::default();
    if !p.components.iter().any(|c| c.on_pcb) {
        return m;
    }
    let l = crate::pcb::layout(p);
    m.has_layout = true;
    for t in &l.tracks {
        m.min_track_mm = fmin(m.min_track_mm, t.width);
    }
    for v in &l.vias {
        m.min_via_drill_mm = fmin(m.min_via_drill_mm, v.drill);
        m.min_via_annular_mm = fmin(m.min_via_annular_mm, (v.pad - v.drill) / 2.0);
        m.via_count += 1;
        if let Some(pd) = l
            .pads
            .iter()
            .find(|pd| pd.drill <= 0.0 && (v.x - pd.x).abs() < pd.w / 2.0 && (v.y - pd.y).abs() < pd.h / 2.0)
        {
            if pd.w * pd.h >= 2.0 {
                m.thermal_vias += 1;
            } else {
                m.vias_in_pads += 1;
            }
        }
    }
    for pd in l.pads.iter().filter(|pd| pd.drill > 0.0) {
        m.min_pth_drill_mm = fmin(m.min_pth_drill_mm, pd.drill);
        m.min_pth_annular_mm = fmin(m.min_pth_annular_mm, (pd.w.min(pd.h) - pd.drill) / 2.0);
    }
    // edge-to-edge gap between neighbouring SMD pads on the same side (solder-mask web)
    let smd: Vec<_> = l.pads.iter().filter(|pd| pd.drill <= 0.0 && pd.top).collect();
    for (i, a) in smd.iter().enumerate() {
        for b in &smd[i + 1..] {
            let gx = (a.x - b.x).abs() - (a.w + b.w) / 2.0;
            let gy = (a.y - b.y).abs() - (a.h + b.h) / 2.0;
            if gx > 2.0 || gy > 2.0 {
                continue;
            }
            let gap = if gx > 0.0 && gy > 0.0 {
                (gx * gx + gy * gy).sqrt()
            } else {
                gx.max(gy)
            };
            if gap > 1e-6 {
                m.min_pad_gap_mm = fmin(m.min_pad_gap_mm, gap);
            }
        }
    }
    m
}

fn item(rule: &str, design: f64, limit: f64, unit: &'static str, ok: bool, source: &'static str, note: String) -> DfmItem {
    DfmItem {
        rule: rule.into(),
        design,
        limit,
        unit,
        status: if ok { "pass" } else { "fail" },
        source,
        note,
    }
}

/// Check the project against one tier using already-measured features.
pub fn check_with(p: &Project, m: &Measured, t: &'static Tier) -> Dfm {
    let pcb = &p.design.pcb;
    let eps = 1e-9;
    let mut items = Vec::new();
    let track = m.min_track_mm.unwrap_or(pcb.min_trace_mm).min(pcb.min_trace_mm);
    items.push(item(
        "Minimum trace width",
        track,
        t.trace_mm,
        "mm",
        track >= t.trace_mm - eps,
        if m.min_track_mm.is_some() { "layout" } else { "design rule" },
        "finest track (and the design rule)".into(),
    ));
    items.push(item(
        "Minimum spacing",
        pcb.min_clearance_mm,
        t.space_mm,
        "mm",
        pcb.min_clearance_mm >= t.space_mm - eps,
        "design rule",
        "clearance enforced by the PCB Studio's DRC".into(),
    ));
    let via_drill = m.min_via_drill_mm.unwrap_or(pcb.via_drill_mm);
    items.push(item(
        "Minimum via drill",
        via_drill,
        t.drill_mm,
        "mm",
        via_drill >= t.drill_mm - eps,
        if m.min_via_drill_mm.is_some() { "layout" } else { "design rule" },
        format!("{} vias", m.via_count),
    ));
    let ann = m.min_via_annular_mm.unwrap_or((pcb.via_pad_mm - pcb.via_drill_mm) / 2.0);
    items.push(item(
        "Via annular ring",
        ann,
        t.annular_mm,
        "mm",
        ann >= t.annular_mm - eps,
        if m.min_via_annular_mm.is_some() { "layout" } else { "design rule" },
        "(pad − drill) / 2".into(),
    ));
    if let (Some(d), Some(a)) = (m.min_pth_drill_mm, m.min_pth_annular_mm) {
        items.push(item(
            "Plated-hole drill",
            d,
            t.drill_mm,
            "mm",
            d >= t.drill_mm - eps,
            "layout",
            "through-hole pads".into(),
        ));
        items.push(item(
            "Plated-hole annular ring",
            a,
            t.annular_mm,
            "mm",
            a >= t.annular_mm - eps,
            "layout",
            String::new(),
        ));
    }
    items.push(item(
        "Layer count",
        pcb.layers as f64,
        t.max_layers as f64,
        "layers",
        pcb.layers <= t.max_layers,
        "design",
        String::new(),
    ));
    let th_ok = pcb.thickness_mm >= t.min_thickness_mm - eps && pcb.thickness_mm <= t.max_thickness_mm + eps;
    items.push(DfmItem {
        rule: "Board thickness".into(),
        design: pcb.thickness_mm,
        limit: t.min_thickness_mm,
        unit: "mm",
        status: if th_ok { "pass" } else { "fail" },
        source: "design",
        note: format!("tier offers {:.1}–{:.1} mm", t.min_thickness_mm, t.max_thickness_mm),
    });
    let edge = if m.has_layout {
        crate::pcb::layout(p).rules.edge_clearance_mm
    } else {
        0.3
    };
    items.push(item(
        "Copper to board edge",
        edge,
        t.edge_mm,
        "mm",
        edge >= t.edge_mm - eps,
        "layout rule",
        String::new(),
    ));
    if m.vias_in_pads > 0 {
        items.push(DfmItem {
            rule: "Via-in-pad".into(),
            design: m.vias_in_pads as f64,
            limit: 0.0,
            unit: "vias",
            status: if t.via_in_pad { "pass" } else { "fail" },
            source: "layout",
            note: if t.via_in_pad {
                "filled and capped".into()
            } else {
                "tier does not fill / cap vias in pads — solder wicks away".into()
            },
        });
    }
    if m.thermal_vias > 0 {
        items.push(DfmItem {
            rule: "Thermal vias in exposed pads".into(),
            design: m.thermal_vias as f64,
            limit: 0.0,
            unit: "vias",
            status: if t.via_in_pad { "pass" } else { "warn" },
            source: "layout",
            note: if t.via_in_pad {
                "can be filled and capped".into()
            } else {
                "common practice without fill: tent or plug them from the bottom, keep drills small and window the paste to limit wicking and voids".into()
            },
        });
    }
    // warnings (not counted as violations)
    if let Some(g) = m.min_pad_gap_mm {
        items.push(DfmItem {
            rule: "Solder-mask web between pads".into(),
            design: g,
            limit: t.mask_web_mm + 2.0 * 0.05,
            unit: "mm",
            status: if g >= t.mask_web_mm + 0.1 - eps { "pass" } else { "warn" },
            source: "layout",
            note: "pad gap must hold the mask web plus ≈0.05 mm mask expansion each side; otherwise the fab removes the dam (gang relief) — more bridging risk".into(),
        });
    }
    let pitch = min_pitch_mm(p);
    if let Some(pt) = pitch {
        let enig = t.finishes.contains(&"ENIG");
        let fine = pt < 0.5 + eps;
        items.push(DfmItem {
            rule: "Surface finish for finest pitch".into(),
            design: pt,
            limit: 0.5,
            unit: "mm pitch",
            status: if !fine || enig { "pass" } else { "warn" },
            source: "footprints",
            note: if fine {
                if enig {
                    "fine pitch → choose ENIG (flat pads)".into()
                } else {
                    "fine pitch but only HASL offered — uneven pads, more bridging".into()
                }
            } else {
                "any finish works".into()
            },
        });
    }
    items.push(DfmItem {
        rule: "Impedance control".into(),
        design: 0.0,
        limit: 0.0,
        unit: "",
        status: "info",
        source: "tier",
        note: format!("{} — {}", t.impedance, impedance_note(p)),
    });
    let violations = items.iter().filter(|i| i.status == "fail").count();
    let warnings = items.iter().filter(|i| i.status == "warn").count();
    Dfm {
        tier: t,
        items,
        violations,
        warnings,
        cheapest_ok: None,
        has_layout: m.has_layout,
        min_pitch_mm: pitch,
    }
}

/// What the board's one impedance-critical line is, as routed.
fn impedance_note(p: &Project) -> String {
    if !p.components.iter().any(|c| c.on_pcb && c.refdes == "AE1") {
        return "no controlled-impedance line on this board; it matters on 4+ layer stack-ups with thin dielectrics".into();
    }
    let rf = crate::rf::IrisRf::from_project(p);
    let (f, src) = crate::rf::feed_as_built(p, &rf);
    let (z0, _) = crate::rf::feed_z0(p, &rf, &f);
    format!(
        "the antenna feed is a {:.2} mm grounded coplanar waveguide (CPWG) with {:.2} mm gaps on the {:.2} mm dielectric → {z0:.0} Ω computed ({src}); etch-width and εr spread move it by a few ohms, so order impedance control with a test coupon where offered (check the fab's tolerance)",
        f.width_mm,
        f.gap_mm,
        crate::rf::stack_h_mm(p, &rf)
    )
}

pub fn check(p: &Project, tier_id: &str) -> Dfm {
    let m = measure(p);
    let mut d = check_with(p, &m, tier(tier_id));
    d.cheapest_ok = TIERS.iter().find(|t| check_with(p, &m, t).violations == 0).map(|t| t.id);
    d
}

/// Tier selected for the metric: `design.extra.pcb_tier`, default "standard".
pub fn project_tier(p: &Project) -> &'static str {
    let id = p.design.extra.get("pcb_tier").and_then(|v| v.as_str()).unwrap_or("standard");
    tier(id).id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_get_finer() {
        for w in TIERS.windows(2) {
            assert!(w[1].trace_mm <= w[0].trace_mm && w[1].drill_mm <= w[0].drill_mm && w[1].annular_mm <= w[0].annular_mm);
        }
    }

    #[test]
    fn impedance_note_states_the_routed_feed() {
        let p = Project::iris();
        let d = check(&p, "standard");
        let n = &d.items.iter().find(|i| i.rule == "Impedance control").unwrap().note;
        assert!(!n.contains("vendor reference layout"), "{n}");
        assert!(
            n.contains("0.91 mm grounded coplanar waveguide") && n.contains("as routed by PCB Studio"),
            "{n}"
        );
        let z = crate::rf::metrics(&p)
            .into_iter()
            .find(|m| m.key == "rf.feed_z0_ohm")
            .unwrap()
            .value;
        assert!(n.contains(&format!("→ {z:.0} Ω")), "{n}");
    }

    #[test]
    fn iris_on_standard_tier() {
        let p = Project::iris();
        let d = check(&p, "standard");
        for i in &d.items {
            eprintln!("{} {:.3} vs {:.3} {}", i.rule, i.design, i.limit, i.status);
        }
        assert!(d.has_layout);
        assert_eq!(d.violations, 0);
        assert_eq!(d.min_pitch_mm, Some(0.4));
        assert!(d.cheapest_ok.is_some());
    }

    #[test]
    fn finer_design_fails_basic_tier() {
        let mut p = Project::blank("x");
        p.design.pcb.min_trace_mm = 0.1;
        p.design.pcb.min_clearance_mm = 0.1;
        p.design.pcb.via_drill_mm = 0.2;
        p.design.pcb.via_pad_mm = 0.4;
        let m = measure(&p);
        assert!(!m.has_layout);
        let d = check_with(&p, &m, tier("basic"));
        assert!(d.violations >= 4, "{}", d.violations);
        assert_eq!(check_with(&p, &m, tier("advanced")).violations, 0);
    }
}
