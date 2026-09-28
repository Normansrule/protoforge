//! Process-selection wizard: a transparent weighted-scoring model.
//!
//! Each process gets a sub-score in [0, 1] for every criterion; the total is
//! the weighted mean × 100. Hard constraints (material the process cannot
//! make, geometry it cannot produce) mark the process *not feasible* and
//! multiply its score by zero, with the reason shown. The capability table
//! (quantity sweet spot, achievable tolerance, finish, strength, lead time,
//! relative setup and unit cost) is **indicative** — compiled from general
//! design-for-manufacture guidance (Boothroyd, Dewhurst & Knight; Ashby) —
//! and every weight is visible and editable.
//!
//! Relative cost index at quantity q (unitless, for ranking only):
//!
//! ```text
//! cost(q) = setup / q + unit          (setup and unit in "palm-sized part" cost units)
//! s_cost  = clamp(1 − log₁₀(cost / cost_min) / 2, 0, 1)
//! ```
//!
//! References: G. Boothroyd, P. Dewhurst, W. Knight, *Product Design for
//! Manufacture and Assembly* (3rd ed., 2010); M. F. Ashby, *Materials
//! Selection in Mechanical Design* (process selection charts).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct ProcessCap {
    pub id: &'static str,
    pub name: &'static str,
    pub materials: &'static [&'static str],
    /// Geometry suitability: [solid 3D, flat profile, bent sheet, thin-wall enclosure].
    pub geometry: [f64; 4],
    /// Quantity sweet spot [lo, hi].
    pub qty: [f64; 2],
    /// Typical achievable tolerance, ± mm.
    pub tol_mm: f64,
    /// Surface finish capability 1 (rough) … 3 (cosmetic).
    pub finish: f64,
    /// Strength / isotropy 1 … 3.
    pub strength: f64,
    /// Typical lead time [lo, hi] days.
    pub lead_days: [f64; 2],
    /// Relative fixed (tooling/programming) and per-part cost units.
    pub setup: f64,
    pub unit: f64,
    /// Largest typical part dimension, mm (service bureaus / common machines).
    pub max_size_mm: f64,
    pub vendor_categories: &'static [&'static str],
    pub upload: &'static str,
    pub lab_route: &'static str,
    pub lab_label: &'static str,
    pub note: &'static str,
}

pub const PROCESSES: &[ProcessCap] = &[
    ProcessCap {
        id: "fdm",
        name: "FDM 3D printing",
        materials: &["plastic", "elastomer"],
        geometry: [1.0, 0.5, 0.1, 0.8],
        qty: [1.0, 50.0],
        tol_mm: 0.3,
        finish: 1.0,
        strength: 1.5,
        lead_days: [1.0, 3.0],
        setup: 0.0,
        unit: 5.0,
        max_size_mm: 300.0,
        vendor_categories: &["print"],
        upload: "STL / 3MF (or G-code for your own printer)",
        lab_route: "/print",
        lab_label: "Print Lab",
        note: "Cheapest and fastest for a few parts; layer lines, weaker in Z.",
    },
    ProcessCap {
        id: "sla",
        name: "Resin printing (SLA)",
        materials: &["plastic", "elastomer"],
        geometry: [1.0, 0.5, 0.1, 0.8],
        qty: [1.0, 100.0],
        tol_mm: 0.1,
        finish: 3.0,
        strength: 1.0,
        lead_days: [1.0, 5.0],
        setup: 0.0,
        unit: 12.0,
        max_size_mm: 250.0,
        vendor_categories: &["print"],
        upload: "STL / STEP",
        lab_route: "/cad",
        lab_label: "CAD Studio",
        note: "Smooth, detailed; most resins are brittle and UV-sensitive.",
    },
    ProcessCap {
        id: "sls",
        name: "SLS / MJF nylon",
        materials: &["plastic", "elastomer"],
        geometry: [1.0, 0.5, 0.1, 0.9],
        qty: [5.0, 1000.0],
        tol_mm: 0.3,
        finish: 2.0,
        strength: 2.0,
        lead_days: [3.0, 8.0],
        setup: 0.0,
        unit: 15.0,
        max_size_mm: 350.0,
        vendor_categories: &["print"],
        upload: "STL / STEP",
        lab_route: "/cad",
        lab_label: "CAD Studio",
        note: "Functional nylon parts without supports; grainy surface.",
    },
    ProcessCap {
        id: "cnc",
        name: "CNC machining",
        materials: &["metal", "plastic", "wood"],
        geometry: [1.0, 0.7, 0.3, 0.4],
        qty: [1.0, 500.0],
        tol_mm: 0.025,
        finish: 3.0,
        strength: 3.0,
        lead_days: [3.0, 12.0],
        setup: 60.0,
        unit: 40.0,
        max_size_mm: 1000.0,
        vendor_categories: &["cnc"],
        upload: "STEP + 2D drawing (PDF) with tolerances",
        lab_route: "/cad",
        lab_label: "CAD Studio",
        note: "Tightest tolerances in real materials; cost grows with complexity and setups.",
    },
    ProcessCap {
        id: "urethane",
        name: "Urethane casting",
        materials: &["plastic", "elastomer"],
        geometry: [1.0, 0.5, 0.1, 0.9],
        qty: [10.0, 200.0],
        tol_mm: 0.25,
        finish: 3.0,
        strength: 2.0,
        lead_days: [10.0, 20.0],
        setup: 400.0,
        unit: 15.0,
        max_size_mm: 800.0,
        vendor_categories: &["urethane"],
        upload: "STEP (master pattern is printed from it)",
        lab_route: "/mfg",
        lab_label: "Manufacturing Hub",
        note: "Silicone molds from a printed master: production-like parts for DVT bridges.",
    },
    ProcessCap {
        id: "molding",
        name: "Injection molding",
        materials: &["plastic", "elastomer"],
        geometry: [0.9, 0.5, 0.1, 1.0],
        qty: [1000.0, 1_000_000.0],
        tol_mm: 0.1,
        finish: 3.0,
        strength: 3.0,
        lead_days: [20.0, 60.0],
        setup: 5000.0,
        unit: 0.5,
        max_size_mm: 600.0,
        vendor_categories: &["molding"],
        upload: "STEP + 2D drawing + molding RFQ",
        lab_route: "/molding",
        lab_label: "Molding Lab",
        note: "Lowest part cost at volume; tooling cost and weeks of lead time up front.",
    },
    ProcessCap {
        id: "sheet",
        name: "Sheet-metal fabrication",
        materials: &["metal"],
        geometry: [0.2, 0.8, 1.0, 0.6],
        qty: [1.0, 10000.0],
        tol_mm: 0.25,
        finish: 2.0,
        strength: 3.0,
        lead_days: [4.0, 10.0],
        setup: 50.0,
        unit: 8.0,
        max_size_mm: 1500.0,
        vendor_categories: &["sheet"],
        upload: "DXF flat pattern or STEP of the bent part",
        lab_route: "/cad",
        lab_label: "CAD Studio",
        note: "Brackets, shields and enclosures from flat stock.",
    },
    ProcessCap {
        id: "laser",
        name: "Laser cutting",
        materials: &["metal", "plastic", "wood"],
        geometry: [0.0, 1.0, 0.2, 0.2],
        qty: [1.0, 10000.0],
        tol_mm: 0.1,
        finish: 2.0,
        strength: 2.0,
        lead_days: [1.0, 5.0],
        setup: 10.0,
        unit: 2.0,
        max_size_mm: 1500.0,
        vendor_categories: &["laser", "sheet"],
        upload: "DXF / SVG outline",
        lab_route: "/cad",
        lab_label: "CAD Studio",
        note: "Flat profiles in minutes; metal via a fibre-laser service.",
    },
    ProcessCap {
        id: "diecast",
        name: "Die casting",
        materials: &["metal"],
        geometry: [1.0, 0.4, 0.1, 0.8],
        qty: [2000.0, 1_000_000.0],
        tol_mm: 0.15,
        finish: 2.0,
        strength: 3.0,
        lead_days: [40.0, 80.0],
        setup: 15000.0,
        unit: 2.0,
        max_size_mm: 600.0,
        vendor_categories: &["molding"],
        upload: "STEP + 2D drawing + RFQ",
        lab_route: "/molding",
        lab_label: "Molding Lab (draft rules apply)",
        note: "Aluminium / zinc / magnesium housings at volume; needs draft like molding.",
    },
];

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Weights {
    pub quantity: f64,
    pub cost: f64,
    pub tolerance: f64,
    pub finish: f64,
    pub strength: f64,
    pub lead_time: f64,
    pub size: f64,
    pub geometry: f64,
}

impl Default for Weights {
    fn default() -> Self {
        Weights {
            quantity: 3.0,
            cost: 3.0,
            tolerance: 2.0,
            finish: 1.0,
            strength: 2.0,
            lead_time: 2.0,
            size: 1.0,
            geometry: 2.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct WizardIn {
    pub quantity: f64,
    /// plastic | elastomer | metal | wood
    pub material: String,
    /// solid | flat | bent | enclosure
    pub geometry: String,
    pub size_mm: f64,
    /// Required tolerance, ± mm.
    pub tolerance_mm: f64,
    /// 1 rough … 3 cosmetic
    pub finish: f64,
    /// 1 low … 3 high
    pub strength: f64,
    /// Days available.
    pub lead_days: f64,
    pub weights: Weights,
}

impl Default for WizardIn {
    fn default() -> Self {
        WizardIn {
            quantity: 10.0,
            material: "plastic".into(),
            geometry: "enclosure".into(),
            size_mm: 150.0,
            tolerance_mm: 0.2,
            finish: 2.0,
            strength: 2.0,
            lead_days: 14.0,
            weights: Weights::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Sub {
    pub criterion: &'static str,
    pub score: f64,
    pub weight: f64,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Ranked {
    pub process: &'static ProcessCap,
    pub score: f64,
    pub feasible: bool,
    pub blockers: Vec<String>,
    pub subs: Vec<Sub>,
    pub cost_index: f64,
    /// "$" … "$$$$$" (log bands of the relative cost index).
    pub cost_band: String,
    pub lead_band: String,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WizardOut {
    pub ranked: Vec<Ranked>,
    pub weights: Weights,
    pub notes: Vec<&'static str>,
}

fn qty_fit(q: f64, r: [f64; 2]) -> f64 {
    // 1 inside the sweet spot, falling off by one decade on the log scale
    let lq = q.max(1.0).log10();
    let (lo, hi) = (r[0].log10(), r[1].log10());
    if lq < lo {
        (1.0 - (lo - lq) / 1.5).max(0.0)
    } else if lq > hi {
        (1.0 - (lq - hi) / 1.5).max(0.0)
    } else {
        1.0
    }
}

fn band(cost: f64) -> String {
    let n = if cost < 3.0 {
        1
    } else if cost < 10.0 {
        2
    } else if cost < 30.0 {
        3
    } else if cost < 100.0 {
        4
    } else {
        5
    };
    "$".repeat(n)
}

pub fn cost_index(p: &ProcessCap, q: f64) -> f64 {
    p.setup / q.max(1.0) + p.unit
}

pub fn rank(i: &WizardIn) -> WizardOut {
    let w = &i.weights;
    let q = i.quantity.max(1.0);
    let gi = match i.geometry.as_str() {
        "flat" => 1,
        "bent" => 2,
        "enclosure" => 3,
        _ => 0,
    };
    let feasible_cost = PROCESSES
        .iter()
        .filter(|p| p.materials.contains(&i.material.as_str()) && p.geometry[gi] > 0.05)
        .map(|p| cost_index(p, q))
        .fold(f64::INFINITY, f64::min);
    let mut ranked: Vec<Ranked> = PROCESSES
        .iter()
        .map(|p| {
            let mut blockers = Vec::new();
            if !p.materials.contains(&i.material.as_str()) {
                blockers.push(format!("does not make {} parts", i.material));
            }
            if p.geometry[gi] <= 0.05 {
                blockers.push("cannot produce this geometry".into());
            }
            let cost = cost_index(p, q);
            let s_qty = qty_fit(q, p.qty);
            let s_cost = if feasible_cost.is_finite() {
                (1.0 - (cost / feasible_cost).log10() / 2.0).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let s_tol = if p.tol_mm <= i.tolerance_mm {
                1.0
            } else {
                (i.tolerance_mm / p.tol_mm).clamp(0.0, 1.0)
            };
            let s_fin = if p.finish >= i.finish {
                1.0
            } else {
                (1.0 - (i.finish - p.finish) / 2.0).max(0.0)
            };
            let s_str = if p.strength >= i.strength {
                1.0
            } else {
                (1.0 - (i.strength - p.strength) / 2.0).max(0.0)
            };
            let s_lead = if p.lead_days[1] <= i.lead_days {
                1.0
            } else if p.lead_days[0] <= i.lead_days {
                0.6
            } else {
                (i.lead_days / p.lead_days[0]).clamp(0.0, 0.5)
            };
            let s_size = if i.size_mm <= p.max_size_mm { 1.0 } else { 0.2 };
            let s_geo = p.geometry[gi];
            let subs = vec![
                Sub {
                    criterion: "quantity",
                    score: s_qty,
                    weight: w.quantity,
                    reason: format!("sweet spot {}–{} parts", fmt_q(p.qty[0]), fmt_q(p.qty[1])),
                },
                Sub {
                    criterion: "cost",
                    score: s_cost,
                    weight: w.cost,
                    reason: format!("relative cost index {cost:.1} at {} parts", fmt_q(q)),
                },
                Sub {
                    criterion: "tolerance",
                    score: s_tol,
                    weight: w.tolerance,
                    reason: format!("typical ±{:.3} mm vs ±{:.3} mm needed", p.tol_mm, i.tolerance_mm),
                },
                Sub {
                    criterion: "finish",
                    score: s_fin,
                    weight: w.finish,
                    reason: format!("finish class {:.0}/3", p.finish),
                },
                Sub {
                    criterion: "strength",
                    score: s_str,
                    weight: w.strength,
                    reason: format!("strength class {:.1}/3", p.strength),
                },
                Sub {
                    criterion: "lead time",
                    score: s_lead,
                    weight: w.lead_time,
                    reason: format!("typically {:.0}–{:.0} days", p.lead_days[0], p.lead_days[1]),
                },
                Sub {
                    criterion: "size",
                    score: s_size,
                    weight: w.size,
                    reason: format!("up to ≈{:.0} mm typical", p.max_size_mm),
                },
                Sub {
                    criterion: "geometry",
                    score: s_geo,
                    weight: w.geometry,
                    reason: "fit to the part's geometry".into(),
                },
            ];
            let wsum: f64 = subs.iter().map(|s| s.weight.max(0.0)).sum();
            let raw = if wsum > 0.0 {
                subs.iter().map(|s| s.weight.max(0.0) * s.score).sum::<f64>() / wsum
            } else {
                0.0
            };
            let feasible = blockers.is_empty();
            let mut reasons: Vec<String> = Vec::new();
            let mut sorted: Vec<&Sub> = subs.iter().filter(|s| s.weight > 0.0).collect();
            sorted.sort_by(|a, b| (b.score * b.weight).total_cmp(&(a.score * a.weight)));
            for s in sorted.iter().take(2).filter(|s| s.score >= 0.8) {
                reasons.push(format!("✓ {}: {}", s.criterion, s.reason));
            }
            for s in subs.iter().filter(|s| s.weight > 0.0 && s.score < 0.5) {
                reasons.push(format!("✕ {}: {}", s.criterion, s.reason));
            }
            Ranked {
                process: p,
                score: if feasible { 100.0 * raw } else { 0.0 },
                feasible,
                blockers,
                subs,
                cost_index: cost,
                cost_band: band(cost),
                lead_band: format!("{:.0}–{:.0} days", p.lead_days[0], p.lead_days[1]),
                reasons,
            }
        })
        .collect();
    ranked.sort_by(|a, b| b.score.total_cmp(&a.score));
    WizardOut {
        ranked,
        weights: i.weights.clone(),
        notes: vec![
            "Indicative only: the capability table and relative cost index describe typical palm-sized parts. Get quotes before deciding.",
            "Score = Σ weight × sub-score / Σ weight × 100; processes that cannot make the material or geometry score 0.",
        ],
    }
}

fn fmt_q(q: f64) -> String {
    if q >= 1e6 {
        format!("{:.0}M", q / 1e6)
    } else if q >= 1e3 {
        format!("{:.0}k", q / 1e3)
    } else {
        format!("{q:.0}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top(i: WizardIn) -> &'static str {
        rank(&i).ranked[0].process.id
    }

    #[test]
    fn ranking_sanity() {
        // one loose plastic prototype → FDM
        assert_eq!(
            top(WizardIn {
                quantity: 1.0,
                tolerance_mm: 0.5,
                finish: 1.0,
                strength: 1.0,
                lead_days: 3.0,
                geometry: "solid".into(),
                ..Default::default()
            }),
            "fdm"
        );
        // 100k plastic housings → injection molding
        assert_eq!(
            top(WizardIn {
                quantity: 100_000.0,
                lead_days: 90.0,
                ..Default::default()
            }),
            "molding"
        );
        // 10 flat metal plates → laser cutting
        assert_eq!(
            top(WizardIn {
                quantity: 10.0,
                material: "metal".into(),
                geometry: "flat".into(),
                ..Default::default()
            }),
            "laser"
        );
        // 5 tight-tolerance metal blocks → CNC
        assert_eq!(
            top(WizardIn {
                quantity: 5.0,
                material: "metal".into(),
                geometry: "solid".into(),
                tolerance_mm: 0.05,
                strength: 3.0,
                ..Default::default()
            }),
            "cnc"
        );
        // 50 bent metal brackets → sheet metal
        assert_eq!(
            top(WizardIn {
                quantity: 50.0,
                material: "metal".into(),
                geometry: "bent".into(),
                ..Default::default()
            }),
            "sheet"
        );
    }

    #[test]
    fn infeasible_processes_score_zero() {
        let o = rank(&WizardIn {
            material: "metal".into(),
            ..Default::default()
        });
        let fdm = o.ranked.iter().find(|r| r.process.id == "fdm").unwrap();
        assert!(!fdm.feasible && fdm.score == 0.0 && !fdm.blockers.is_empty());
        assert!(o.ranked.windows(2).all(|w| w[0].score >= w[1].score));
    }

    #[test]
    fn cost_index_amortises_setup() {
        let m = PROCESSES.iter().find(|p| p.id == "molding").unwrap();
        assert!((cost_index(m, 10_000.0) - 1.0).abs() < 1e-12);
        assert!(cost_index(m, 10.0) > 100.0);
    }
}
