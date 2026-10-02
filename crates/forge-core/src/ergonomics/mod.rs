//! # Human Factors Lab engine — comfort, fit and legibility of head-worn products
//!
//! Smart glasses live or die by comfort and fit. This module ties the Iris
//! frame (`design.frame`, the CAD geometry from [`crate::cad`]), the component
//! masses and locations, and the display optics (`design.optics`, eyebox and
//! photometry from [`crate::optics::paraxial`]) to the people who will wear
//! them.
//!
//! | file | what |
//! |---|---|
//! | [`stats`] | normal CDF / inverse, two-sex mixtures, Cholesky sampling |
//! | [`anthro`] | head breadth, interpupillary distance (IPD), nose bridge, hinge-to-ear distance; percentiles; Monte Carlo fit |
//! | [`model`] | lab settings (`design.extra.ergonomics`) and frame geometry read from CAD |
//! | [`balance`] | mass distribution, centre of mass, nose/ear load split, nose-pad pressure, slip |
//! | [`clamp`] | temples as stepped cantilevers + front bending → clamp force vs head breadth |
//! | [`display`] | angular text size vs acuity, ambient contrast ratio, eyebox coverage |
//! | [`head`] | parametric head mesh for the 3-D fit view |
//!
//! ## Metrics (Iris)
//! * `ergonomics.fit_coverage_pct` — share of a seeded adult population (50 %
//!   women) that the one Iris size fits on all three rules: decentration
//!   |frame PD − IPD|/2 ≤ 4 mm, temple bend within −2…+12 mm of the ear, and
//!   clamp force inside the comfort band.
//! * `ergonomics.nose_load_g` — vertical load on the nose, grams-force, head upright.
//! * `ergonomics.clamp_force_n` — clamp force per side at the 95th-percentile
//!   (combined) head breadth.
//! * `ergonomics.eyebox_coverage_pct` — share of wearers whose right pupil
//!   lands inside the display eyebox without adjustment.
//! * `ergonomics.outdoor_contrast` — ambient contrast ratio against a
//!   5,000 cd/m² sunlit street.
//! * plus `ergonomics.nose_pad_pressure_kpa` and `ergonomics.caption_arcmin`.
//!
//! ## References
//! * C. C. Gordon et al., *2012 Anthropometric Survey of U.S. Army Personnel:
//!   Methods and Summary Statistics*, NATICK/TR-15/007 (2014) — ANSUR II.
//! * N. A. Dodgson, "Variation and extrema of human interpupillary distance",
//!   Proc. SPIE 5291, Stereoscopic Displays and Virtual Reality Systems XI (2004).
//! * S. Pheasant & C. M. Haslegrave, *Bodyspace: Anthropometry, Ergonomics and
//!   the Design of Work*, 3rd ed., CRC Press (2006).
//! * ISO 7250-1, *Basic human body measurements for technological design —
//!   Part 1: Body measurement definitions and landmarks*.
//! * M. S. Sanders & E. J. McCormick, *Human Factors in Engineering and
//!   Design*, 7th ed., McGraw-Hill (1993).
//! * C. D. Wickens, J. G. Hollands, S. Banbury & R. Parasuraman, *Engineering
//!   Psychology and Human Performance*, 4th ed., Pearson (2013).
//! * B. Kress & T. Starner, "A review of head-mounted displays (HMD)
//!   technologies and applications for consumer electronics", Proc. SPIE 8720
//!   (2013).
//! * B. C. Kress, *Optical Architectures for Augmented-, Virtual-, and
//!   Mixed-Reality Headsets*, SPIE Press (2020).
//! * E. M. Landis, "Micro-injection studies of capillary blood pressure in
//!   human skin", Heart 15, 209–228 (1930) — the ≈ 32 mmHg sustained-pressure
//!   limit.
//! * S. Derler & L.-C. Gerhardt, "Tribology of skin: review and analysis of
//!   experimental results for the friction coefficient of human skin",
//!   Tribology Letters 45, 1–27 (2012).
//! * ANSI/HFES 100-2007, *Human Factors Engineering of Computer Workstations*
//!   (character height guidance).
//! * W. C. Young & R. G. Budynas, *Roark's Formulas for Stress and Strain*,
//!   7th ed. (2002).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Metric, Project};

pub mod anthro;
pub mod balance;
pub mod clamp;
pub mod display;
pub mod head;
pub mod model;
pub mod stats;

use anthro::{Band, Coverage, FitCheck, Group, Person};
use balance::{Loads, MassItem, Supports};
use clamp::{ClampModel, SpringHinge};
use model::{ErgoConfig, FrameGeom, FrameSize};

// ------------------------------------------------------------------ shared analysis

/// Everything the tabs share, computed once per (project, settings).
#[derive(Clone, Debug)]
pub struct Analysis {
    pub cfg: ErgoConfig,
    pub fg: FrameGeom,
    pub items: Vec<MassItem>,
    pub mass_g: f64,
    pub com: [f64; 3],
    pub pad_area_mm2: f64,
    pub upright: Loads,
    pub look_down: Loads,
    pub band: Band,
    pub clamp: ClampModel,
    /// Spring hinge from the Mechanisms Lab, when the product has one.
    pub hinge: Option<SpringHinge>,
}

/// Over-travel torque curve of the product's spring hinge, if the bill of
/// materials has one (a component whose name mentions a "spring hinge").
pub fn spring_hinge(p: &Project) -> Option<SpringHinge> {
    let has = p.components.iter().any(|c| {
        let t = format!("{} {}", c.name, c.example_part).to_lowercase();
        t.contains("spring hinge") || t.contains("spring-hinge")
    });
    if !has {
        return None;
    }
    let hp = crate::mechanisms::hinge::HingeParams::from_project(p);
    let out = crate::mechanisms::hinge::analyze(p, &hp).ok()?;
    let i90 = out.phi_deg.iter().position(|x| *x >= 90.0)?;
    // the first sample past 90° (a centred difference at 90° mixes in the detent side)
    let i0 = (i90 + 1).min(out.phi_deg.len() - 1);
    let mut alpha: Vec<f64> = vec![0.0];
    let mut torque: Vec<f64> = vec![out.torque_nmm[i0].max(0.0)];
    for q in i0 + 1..out.phi_deg.len() {
        alpha.push(out.phi_deg[q] - 90.0);
        torque.push(out.torque_nmm[q].max(torque[torque.len() - 1]));
    }
    Some(SpringHinge {
        stop_deg: *alpha.last().unwrap_or(&0.0),
        crack_torque_nmm: torque[0],
        alpha_deg: alpha,
        torque_nmm: torque,
    })
}

thread_local! {
    static ANALYSIS: crate::memo::Slot<std::rc::Rc<Analysis>> = const { std::cell::RefCell::new(None) };
    static FIT: crate::memo::Slot<(Coverage, f64)> = const { std::cell::RefCell::new(None) };
}

fn s_of(fg: &FrameGeom) -> Supports {
    balance::supports(fg)
}

pub fn analysis(p: &Project, cfg: &ErgoConfig) -> std::rc::Rc<Analysis> {
    let key = crate::memo::key(&(&p.design.frame, &p.design.optics, &p.design.pcb, &p.components, p.design.extra.get("mechanisms"), cfg));
    crate::memo::memo(&ANALYSIS, key, || {
        let fg = model::frame_geom(p);
        let items = balance::mass_items(p, &fg);
        let (mass_g, com) = balance::centre_of_mass(&items);
        let pad_area = if cfg.pad_area_mm2 > 0.0 {
            cfg.pad_area_mm2
        } else {
            fg.pad_area_cad_mm2.max(1.0)
        };
        let sup = s_of(&fg);
        let upright = balance::loads(mass_g, com, &sup, cfg, pad_area, 0.0);
        let look_down = balance::loads(mass_g, com, &sup, cfg, pad_area, cfg.look_down_deg);
        let band = Band {
            min_n: balance::min_clamp_n(&look_down, cfg),
            max_n: balance::max_clamp_n(&fg, cfg),
        };
        let hinge = spring_hinge(p);
        let clamp = ClampModel::new(&fg, cfg.temple_splay_deg, cfg.contact_before_bend_mm).with_hinge(hinge.clone());
        std::rc::Rc::new(Analysis {
            cfg: cfg.clone(),
            fg,
            items,
            mass_g,
            com,
            pad_area_mm2: pad_area,
            upright,
            look_down,
            band,
            clamp,
            hinge,
        })
    })
}

/// Fit checks of a population against one frame size (`splay` None = the lab setting).
fn checks_for(a: &Analysis, fg: &FrameGeom, splay: Option<f64>, people: &[Person]) -> Vec<FitCheck> {
    let cm = if fg == &a.fg && splay.is_none() {
        a.clamp.clone()
    } else {
        ClampModel::new(fg, splay.unwrap_or(a.cfg.temple_splay_deg), a.cfg.contact_before_bend_mm).with_hinge(a.hinge.clone())
    };
    let f = |h: f64| cm.force(h);
    people.iter().map(|pp| anthro::check(fg, &a.cfg, a.band, pp, &f)).collect()
}

fn combined_hb_p95(cfg: &ErgoConfig) -> f64 {
    anthro::dims(cfg)[anthro::HEAD_BREADTH]
        .group(Group::Combined, cfg.female_fraction)
        .quantile(0.95)
}

// ------------------------------------------------------------------ API types

#[derive(Deserialize, Default)]
#[serde(default)]
struct BaseIn {
    project: Option<Project>,
    /// Settings to use instead of `design.extra.ergonomics`.
    config: Option<ErgoConfig>,
}

impl BaseIn {
    fn resolve(self) -> (Project, ErgoConfig) {
        let p = self.project.unwrap_or_else(Project::iris);
        let cfg = match self.config {
            Some(c) => c.sanitized(),
            None => ErgoConfig::from_project(&p),
        };
        (p, cfg)
    }
}

#[derive(Serialize)]
struct FrameSummary {
    size: FrameSize,
    label: String,
    geom: FrameGeom,
    s0_mm: f64,
    contact_from_pin_mm: f64,
}

fn frame_summary(a: &Analysis) -> FrameSummary {
    FrameSummary {
        size: a.fg.size(),
        label: a.fg.size().label(),
        geom: a.fg.clone(),
        s0_mm: a.clamp.s0,
        contact_from_pin_mm: a.clamp.l_c,
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct FitIn {
    #[serde(flatten)]
    base: BaseIn,
    /// How many people to return for the scatter plot.
    scatter: Option<usize>,
}

#[derive(Serialize)]
struct PersonFit {
    #[serde(flatten)]
    person: Person,
    #[serde(flatten)]
    check: FitCheck,
}

#[derive(Serialize)]
struct FitOut {
    frame: FrameSummary,
    band: Band,
    n: usize,
    combined: Coverage,
    men: Coverage,
    women: Coverage,
    people: Vec<PersonFit>,
    /// Iris clamp force at the combined 5/50/95th percentile head breadth, N.
    clamp_p: [f64; 3],
    /// Head breadths at the two ends of the comfort band, mm.
    band_heads_mm: [f64; 2],
    /// Window of IPD accepted by the decentration rule, mm.
    pd_window_mm: [f64; 2],
    /// Window of ear distance accepted by the temple rule, mm.
    ear_window_abs_mm: [f64; 2],
    config: ErgoConfig,
}

fn fit_api(i: FitIn) -> Result<FitOut, String> {
    let (p, cfg) = i.base.resolve();
    let a = analysis(&p, &cfg);
    let people = anthro::sample(&cfg, cfg.samples, cfg.seed);
    let checks = checks_for(&a, &a.fg, None, &people);
    let pick = |female: Option<bool>| {
        anthro::coverage(
            people
                .iter()
                .zip(checks.iter())
                .filter(|(pp, _)| female.is_none_or(|f| pp.female == f))
                .map(|(_, c)| c),
            a.band,
        )
    };
    let hb = anthro::dims(&cfg)[anthro::HEAD_BREADTH].group(Group::Combined, cfg.female_fraction);
    let k = i.scatter.unwrap_or(600).min(people.len());
    Ok(FitOut {
        frame: frame_summary(&a),
        band: a.band,
        n: people.len(),
        combined: pick(None),
        men: pick(Some(false)),
        women: pick(Some(true)),
        people: people
            .iter()
            .zip(checks.iter())
            .take(k)
            .map(|(pp, c)| PersonFit { person: *pp, check: *c })
            .collect(),
        clamp_p: [
            a.clamp.force(hb.quantile(0.05)),
            a.clamp.force(hb.quantile(0.5)),
            a.clamp.force(hb.quantile(0.95)),
        ],
        band_heads_mm: [a.clamp.width_for(a.band.min_n), a.clamp.width_for(a.band.max_n)],
        pd_window_mm: [
            a.fg.frame_pd_mm - 2.0 * cfg.pd_tolerance_mm,
            a.fg.frame_pd_mm + 2.0 * cfg.pd_tolerance_mm,
        ],
        ear_window_abs_mm: [
            a.fg.bend_from_pin_mm - cfg.ear_window_mm[1],
            a.fg.bend_from_pin_mm - cfg.ear_window_mm[0],
        ],
        config: cfg.clone(),
    })
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct SizesIn {
    #[serde(flatten)]
    base: BaseIn,
    count: Option<usize>,
    samples: Option<usize>,
    /// Let each size have its own temple splay (default true).
    vary_splay: Option<bool>,
}

#[derive(Serialize, Clone)]
struct SizeOut {
    name: String,
    size: FrameSize,
    label: String,
    frame_pd_mm: f64,
    total_width_mm: f64,
    /// Coverage of this size alone, %.
    alone_pct: f64,
    /// Share of the population for whom this is the size picked, %.
    served_pct: f64,
    /// Head breadth range comfortable in this size (clamp band), mm.
    head_breadth_range_mm: [f64; 2],
}

#[derive(Serialize)]
struct SizesOut {
    sizes: Vec<SizeOut>,
    union_pct: f64,
    iris_pct: f64,
    candidates: usize,
    n: usize,
    notes: Vec<String>,
}

/// Greedy + exchange search for `k` sizes maximising the covered share.
fn sizes_api(i: SizesIn) -> Result<SizesOut, String> {
    let (p, cfg) = i.base.resolve();
    let a = analysis(&p, &cfg);
    let k = i.count.unwrap_or(3).clamp(1, 5);
    let n = i.samples.unwrap_or(2000).clamp(200, 10000);
    let people = anthro::sample(&cfg, n, cfg.seed.wrapping_add(11));
    let mut cands: Vec<FrameSize> = Vec::new();
    let splays: Vec<f64> = if i.vary_splay.unwrap_or(true) {
        (0..=8).map(|j| 2.0 * j as f64).collect()
    } else {
        vec![cfg.temple_splay_deg]
    };
    for lw in (0..=9).map(|j| 42.0 + 2.0 * j as f64) {
        for dbl in (0..=5).map(|j| 14.0 + 2.0 * j as f64) {
            for t in [135.0, 140.0, 145.0, 150.0, 155.0] {
                for &sp in &splays {
                    cands.push(FrameSize {
                        lens_width_mm: lw,
                        bridge_mm: dbl,
                        temple_length_mm: t,
                        splay_deg: Some(sp),
                    });
                }
            }
        }
    }
    let words = n.div_ceil(64);
    let bits: Vec<Vec<u64>> = cands
        .iter()
        .map(|s| {
            let fg = a.fg.resized(s);
            let ch = checks_for(&a, &fg, s.splay_deg, &people);
            let mut b = vec![0u64; words];
            for (j, c) in ch.iter().enumerate() {
                if c.fits {
                    b[j / 64] |= 1 << (j % 64);
                }
            }
            b
        })
        .collect();
    let count = |sel: &[usize]| -> u32 {
        (0..words)
            .map(|w| sel.iter().fold(0u64, |acc, &s| acc | bits[s][w]).count_ones())
            .sum()
    };
    // greedy
    let mut sel: Vec<usize> = Vec::new();
    for _ in 0..k {
        let best = (0..cands.len())
            .filter(|c| !sel.contains(c))
            .max_by_key(|&c| {
                let mut t = sel.clone();
                t.push(c);
                count(&t)
            })
            .unwrap_or(0);
        sel.push(best);
    }
    // exchange passes
    for _ in 0..4 {
        let mut improved = false;
        for slot in 0..sel.len() {
            let cur = count(&sel);
            let mut best = (cur, sel[slot]);
            for c in 0..cands.len() {
                if sel.contains(&c) {
                    continue;
                }
                let mut t = sel.clone();
                t[slot] = c;
                let v = count(&t);
                if v > best.0 {
                    best = (v, c);
                }
            }
            if best.1 != sel[slot] {
                sel[slot] = best.1;
                improved = true;
            }
        }
        if !improved {
            break;
        }
    }
    sel.sort_by(|&x, &y| {
        let fx = a.fg.resized(&cands[x]).total_width_mm;
        let fy = a.fg.resized(&cands[y]).total_width_mm;
        fx.partial_cmp(&fy).unwrap_or(std::cmp::Ordering::Equal)
    });
    let names = match sel.len() {
        1 => vec!["One size"],
        2 => vec!["S", "L"],
        3 => vec!["S", "M", "L"],
        4 => vec!["S", "M", "L", "XL"],
        _ => vec!["XS", "S", "M", "L", "XL"],
    };
    // Who is served by which size: the first (smallest) size that fits.
    let mut served = vec![0usize; sel.len()];
    for j in 0..n {
        for (si, &s) in sel.iter().enumerate() {
            if bits[s][j / 64] >> (j % 64) & 1 == 1 {
                served[si] += 1;
                break;
            }
        }
    }
    let iris_checks = checks_for(&a, &a.fg, None, &people);
    let iris_pct = 100.0 * iris_checks.iter().filter(|c| c.fits).count() as f64 / n as f64;
    let sizes: Vec<SizeOut> = sel
        .iter()
        .enumerate()
        .map(|(si, &s)| {
            let c = &cands[s];
            let fg = a.fg.resized(c);
            let cm = ClampModel::new(&fg, c.splay_deg.unwrap_or(cfg.temple_splay_deg), cfg.contact_before_bend_mm).with_hinge(a.hinge.clone());
            SizeOut {
                name: names[si].to_string(),
                size: *c,
                label: c.label(),
                frame_pd_mm: fg.frame_pd_mm,
                total_width_mm: fg.total_width_mm,
                alone_pct: 100.0 * count(&[s]) as f64 / n as f64,
                served_pct: 100.0 * served[si] as f64 / n as f64,
                head_breadth_range_mm: [cm.width_for(a.band.min_n), cm.width_for(a.band.max_n)],
            }
        })
        .collect();
    let mut notes = vec![format!(
        "{} candidate sizes (lens 42–60 mm, bridge 14–24 mm, temple 135–155 mm, splay 0–16°) scored on {n} simulated adults; greedy selection then exchange passes.",
        cands.len()
    )];
    if splays.len() == 1 && a.cfg.temple_splay_deg.abs() < 1e-9 {
        notes.push("Temples are straight (no splay), so front width alone sets the clamp — sizes must track head breadth closely.".into());
    }
    Ok(SizesOut {
        union_pct: 100.0 * count(&sel) as f64 / n as f64,
        sizes,
        iris_pct,
        candidates: cands.len(),
        n,
        notes,
    })
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct HeadIn {
    #[serde(flatten)]
    base: BaseIn,
    /// "men" or "women".
    sex: Option<String>,
    /// Percentiles (0..100) for head_breadth, ipd, ear_distance, nose_bridge.
    head_breadth_pct: Option<f64>,
    ipd_pct: Option<f64>,
    ear_pct: Option<f64>,
    nose_pct: Option<f64>,
    /// Frame size to try instead of the project's (analytic re-size).
    size: Option<FrameSize>,
}

#[derive(Serialize)]
struct HeadOut {
    mesh: head::HeadMesh,
    person: Person,
    check: FitCheck,
    band: Band,
    /// Temple rotation about the hinge pin, degrees outward [right, left].
    temple_angle_deg: [f64; 2],
    pin: [f64; 3],
    deflection_mm: [f64; 2],
    /// Pupil offset from the display axis, mm (+ = pupil outboard).
    pupil_offset_mm: f64,
    eyebox_mm: f64,
    in_eyebox: bool,
    frame: FrameSummary,
}

fn head_api(i: HeadIn) -> Result<HeadOut, String> {
    let (p, cfg) = i.base.resolve();
    let a = analysis(&p, &cfg);
    let female = matches!(i.sex.as_deref(), Some("women") | Some("female") | Some("f"));
    let d = anthro::dims(&cfg);
    let q = |k: usize, pct: Option<f64>| {
        let n = if female { d[k].women } else { d[k].men };
        n.quantile((pct.unwrap_or(50.0) / 100.0).clamp(0.001, 0.999))
    };
    let person = Person {
        female,
        head_breadth: q(anthro::HEAD_BREADTH, i.head_breadth_pct),
        ipd: q(anthro::IPD, i.ipd_pct),
        ear: q(anthro::EAR, i.ear_pct),
        nose: q(anthro::NOSE, i.nose_pct),
        asym: 0.0,
    };
    let fg = match i.size {
        Some(s) => a.fg.resized(&s),
        None => a.fg.clone(),
    };
    let splay = i.size.and_then(|s| s.splay_deg).unwrap_or(cfg.temple_splay_deg);
    let cm = ClampModel::new(&fg, splay, cfg.contact_before_bend_mm).with_hinge(a.hinge.clone());
    let f = |h: f64| cm.force(h);
    let check = anthro::check(&fg, &cfg, a.band, &person, &f);
    let contact_y = fg.pin_y - cm.l_c;
    let mesh = head::head_mesh(&fg, person.head_breadth, person.ipd, person.ear, person.nose, contact_y);
    // Temple angle: at least the splay; opened further if the head is wider.
    let rest = splay;
    let need = ((person.head_breadth / 2.0 - fg.temple_inner_x) / cm.l_c).atan().to_degrees();
    let ang = rest.max(need);
    let eb = crate::optics::paraxial::analyze(&p.design.optics, &Default::default()).eyebox_mm;
    let off = person.ipd / 2.0 - (fg.lens_center_x - cfg.display_offset_mm);
    Ok(HeadOut {
        mesh,
        person,
        check,
        band: a.band,
        temple_angle_deg: [ang, ang],
        pin: [fg.temple_inner_x + fg.temple_w_mm / 2.0, fg.pin_y, fg.hinge_z],
        deflection_mm: cm.deflections(person.head_breadth),
        pupil_offset_mm: off,
        eyebox_mm: eb,
        in_eyebox: off.abs() <= eb / 2.0,
        frame: FrameSummary {
            size: FrameSize {
                splay_deg: Some(splay),
                ..fg.size()
            },
            label: FrameSize {
                splay_deg: Some(splay),
                ..fg.size()
            }
            .label(),
            s0_mm: cm.s0,
            contact_from_pin_mm: cm.l_c,
            geom: fg,
        },
    })
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct BalanceIn {
    #[serde(flatten)]
    base: BaseIn,
    /// Move the battery (and every left-pod part) this far back along the temple, mm.
    battery_shift_mm: f64,
    /// Head pitch for the animated view, degrees (+ = looking down).
    pitch_deg: f64,
}

#[derive(Serialize)]
struct BalanceOut {
    items: Vec<MassItem>,
    mass_g: f64,
    com: [f64; 3],
    pads: [[f64; 3]; 2],
    ears: [[f64; 3]; 2],
    pad_area_mm2: f64,
    pad_area_cad_mm2: f64,
    upright: Loads,
    pitched: Loads,
    look_down: Loads,
    band: Band,
    /// Pad area that brings the pressure to the limit, mm².
    pad_area_for_limit_mm2: f64,
    pressure_limit_kpa: f64,
    /// Curves.
    shift_mm: Vec<f64>,
    nose_gf_vs_shift: Vec<f64>,
    pad_area_axis: Vec<f64>,
    pressure_vs_area: Vec<f64>,
    pitch_axis: Vec<f64>,
    mu_required_vs_pitch: Vec<f64>,
    /// Friction needed with the median-head clamp force acting.
    mu_required_with_clamp: Vec<f64>,
    clamp_p50_n: f64,
    /// Side masses (x > 0 vs x < 0), g.
    right_g: f64,
    left_g: f64,
    frame: FrameSummary,
    notes: Vec<String>,
}

fn shifted(items: &[MassItem], shift: f64) -> Vec<MassItem> {
    items
        .iter()
        .map(|it| {
            let mut it = it.clone();
            if it.kind == "component" && it.location == "left_pod" {
                it.pos[1] -= shift;
            }
            it
        })
        .collect()
}

fn balance_api(i: BalanceIn) -> Result<BalanceOut, String> {
    let (p, cfg) = i.base.resolve();
    let a = analysis(&p, &cfg);
    let sup = s_of(&a.fg);
    let items = shifted(&a.items, i.battery_shift_mm);
    let (m, com) = balance::centre_of_mass(&items);
    let upright = balance::loads(m, com, &sup, &cfg, a.pad_area_mm2, 0.0);
    let pitched = balance::loads(m, com, &sup, &cfg, a.pad_area_mm2, i.pitch_deg.clamp(-30.0, 89.0));
    let look_down = balance::loads(m, com, &sup, &cfg, a.pad_area_mm2, cfg.look_down_deg);
    let band = Band {
        min_n: balance::min_clamp_n(&look_down, &cfg),
        max_n: balance::max_clamp_n(&a.fg, &cfg),
    };
    let shift_mm = crate::math::linspace(0.0, 80.0, 41);
    let nose_gf_vs_shift = shift_mm
        .iter()
        .map(|&s| {
            let it = shifted(&a.items, s);
            let (m2, c2) = balance::centre_of_mass(&it);
            balance::loads(m2, c2, &sup, &cfg, a.pad_area_mm2, 0.0).nose_gf
        })
        .collect();
    let pad_area_axis = crate::math::logspace(5.0, 400.0, 60);
    let pmax = upright.pad_normal_n[0].max(upright.pad_normal_n[1]);
    let pressure_vs_area = pad_area_axis.iter().map(|&ar| pmax / ar * 1e3).collect();
    let pitch_axis = crate::math::linspace(0.0, 80.0, 41);
    let hb = anthro::dims(&cfg)[anthro::HEAD_BREADTH].group(Group::Combined, cfg.female_fraction);
    let f50 = a.clamp.force(hb.quantile(0.5));
    let mut mu0 = Vec::new();
    let mut mu1 = Vec::new();
    for &pa in &pitch_axis {
        let l = balance::loads(m, com, &sup, &cfg, a.pad_area_mm2, pa);
        mu0.push(l.mu_required_no_clamp.min(5.0));
        let normal = l.pad_normal_n[0] + l.pad_normal_n[1] + (l.ears_gf * balance::G * 1e-3).max(0.0) + 2.0 * f50;
        mu1.push(if normal > 0.0 { (l.slide_force_n / normal).min(5.0) } else { 5.0 });
    }
    let right_g: f64 = items.iter().filter(|it| it.pos[0] > 1e-9).map(|it| it.mass_g).sum();
    let left_g: f64 = items.iter().filter(|it| it.pos[0] < -1e-9).map(|it| it.mass_g).sum();
    let limit_area = pmax / (cfg.pressure_limit_kpa * 1e-3);
    let mut notes = Vec::new();
    if upright.pad_pressure_kpa[0].max(upright.pad_pressure_kpa[1]) > cfg.pressure_limit_kpa {
        notes.push(format!(
            "Nose-pad pressure exceeds the {:.1} kPa sustained-pressure limit: pads of at least {:.0} mm² each (now {:.0} mm²) would bring it under.",
            cfg.pressure_limit_kpa, limit_area, a.pad_area_mm2
        ));
    }
    Ok(BalanceOut {
        items,
        mass_g: m,
        com,
        pads: sup.pads,
        ears: sup.ears,
        pad_area_mm2: a.pad_area_mm2,
        pad_area_cad_mm2: a.fg.pad_area_cad_mm2,
        upright,
        pitched,
        look_down,
        band,
        pad_area_for_limit_mm2: limit_area,
        pressure_limit_kpa: cfg.pressure_limit_kpa,
        shift_mm,
        nose_gf_vs_shift,
        pad_area_axis,
        pressure_vs_area,
        pitch_axis,
        mu_required_vs_pitch: mu0,
        mu_required_with_clamp: mu1,
        clamp_p50_n: f50,
        right_g,
        left_g,
        frame: frame_summary(&a),
        notes,
    })
}

#[derive(Serialize)]
struct ClampRow {
    group: Group,
    percentile: f64,
    head_breadth_mm: f64,
    force_n: f64,
    verdict: String,
}

#[derive(Serialize)]
struct ClampOut {
    model: ClampModel,
    band: Band,
    frame: FrameSummary,
    percentiles: Vec<f64>,
    /// Force vs percentile for men, women, combined.
    force_vs_pct: Vec<Vec<f64>>,
    hb_vs_pct: Vec<Vec<f64>>,
    head_axis: Vec<f64>,
    force_vs_head: Vec<f64>,
    rows: Vec<ClampRow>,
    force_p95_n: f64,
    head_p95_mm: f64,
    /// Share of the combined population inside the band, %.
    band_coverage_pct: f64,
    /// Splay that centres the band on the combined median head, degrees.
    best_splay_deg: f64,
    best_splay_coverage_pct: f64,
    /// Head-breadth window of the band, mm.
    band_heads_mm: [f64; 2],
    root_stress_p95_mpa: f64,
    deflection_p95_mm: [f64; 2],
    /// Yaw of the frame from the stiffness asymmetry at p95, degrees.
    yaw_p95_deg: f64,
    material: String,
    notes: Vec<String>,
}

fn verdict(f: f64, b: Band) -> &'static str {
    if f < b.min_n {
        "too loose"
    } else if f > b.max_n {
        "too tight"
    } else {
        "comfortable"
    }
}

fn clamp_api(i: BaseIn) -> Result<ClampOut, String> {
    let (p, cfg) = i.resolve();
    let a = analysis(&p, &cfg);
    let cm = &a.clamp;
    let d = anthro::dims(&cfg);
    let groups = [Group::Men, Group::Women, Group::Combined];
    let percentiles: Vec<f64> = crate::math::linspace(1.0, 99.0, 99);
    let mut force_vs_pct = Vec::new();
    let mut hb_vs_pct = Vec::new();
    for &g in &groups {
        let m = d[anthro::HEAD_BREADTH].group(g, cfg.female_fraction);
        let hbs: Vec<f64> = percentiles.iter().map(|pc| m.quantile(pc / 100.0)).collect();
        force_vs_pct.push(hbs.iter().map(|&h| cm.force(h)).collect());
        hb_vs_pct.push(hbs);
    }
    let head_axis = crate::math::linspace(125.0, 180.0, 111);
    let force_vs_head = head_axis.iter().map(|&h| cm.force(h)).collect();
    let mut rows = Vec::new();
    for &g in &groups {
        let m = d[anthro::HEAD_BREADTH].group(g, cfg.female_fraction);
        for pc in [5.0, 50.0, 95.0] {
            let h = m.quantile(pc / 100.0);
            let f = cm.force(h);
            rows.push(ClampRow {
                group: g,
                percentile: pc,
                head_breadth_mm: h,
                force_n: f,
                verdict: verdict(f, a.band).into(),
            });
        }
    }
    let comb = d[anthro::HEAD_BREADTH].group(Group::Combined, cfg.female_fraction);
    let h95 = comb.quantile(0.95);
    let f95 = cm.force(h95);
    let lo = cm.width_for(a.band.min_n);
    let hi = cm.width_for(a.band.max_n);
    let cov = 100.0 * comb.prob_between(lo, hi);
    let target = 0.5 * (a.band.min_n + a.band.max_n);
    let best_splay = cm.splay_for(comb.quantile(0.5), target, a.fg.temple_inner_x);
    let cm2 = ClampModel::new(&a.fg, best_splay, cfg.contact_before_bend_mm).with_hinge(a.hinge.clone());
    let cov2 = 100.0 * comb.prob_between(cm2.width_for(a.band.min_n), cm2.width_for(a.band.max_n));
    let defl = cm.deflections(h95);
    let lever = (a.fg.pad[1] - (a.fg.pin_y - cm.l_c)).abs().max(1.0);
    let yaw = ((defl[0] - defl[1]) / 2.0 / lever).atan().to_degrees();
    let mut notes = Vec::new();
    if f95 > a.band.max_n {
        notes.push(format!(
            "At the 95th-percentile head ({h95:.0} mm) the temples press with {f95:.2} N per side — above the {:.2} N comfort limit. A {best_splay:.1}° outward splay (or S/M/L fronts) centres the band.",
            a.band.max_n
        ));
    }
    if cm.c_front > 2.0 * (cm.c_left + cm.c_right) {
        notes.push(
            "Most of the compliance is the front bowing, not the temples: a stiffer front (or bonded lenses) raises clamp force.".into(),
        );
    }
    Ok(ClampOut {
        model: cm.clone(),
        band: a.band,
        frame: frame_summary(&a),
        percentiles,
        force_vs_pct,
        hb_vs_pct,
        head_axis,
        force_vs_head,
        rows,
        force_p95_n: f95,
        head_p95_mm: h95,
        band_coverage_pct: cov,
        best_splay_deg: best_splay,
        best_splay_coverage_pct: cov2,
        band_heads_mm: [lo, hi],
        root_stress_p95_mpa: cm.root_stress_mpa(f95, &a.fg),
        deflection_p95_mm: defl,
        yaw_p95_deg: yaw,
        material: a.fg.material.clone(),
        notes,
    })
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct DisplayIn {
    #[serde(flatten)]
    base: BaseIn,
    /// Extra background for a custom scene, cd/m².
    custom_nits: Option<f64>,
}

#[derive(Serialize)]
struct LegRow {
    cap_px: f64,
    arcmin: f64,
    snellen_20_over: f64,
    verdict: String,
}

#[derive(Serialize)]
struct DisplayOut {
    arcmin_per_px: f64,
    ppd: f64,
    ppd_center: f64,
    hfov_deg: f64,
    vfov_deg: f64,
    px_w: u32,
    px_h: u32,
    acuity_arcmin: f64,
    caption_px: f64,
    caption_arcmin: f64,
    caption_verdict: String,
    /// Smallest cap height (px) reaching 16 and 20 arcmin.
    min_px_16: f64,
    min_px_20: f64,
    rows: Vec<LegRow>,
    photometry: display::Photometry,
    scenes: Vec<display::SceneRow>,
    bright: display::SceneRow,
    eyebox: display::EyeboxOut,
    /// ACR vs background (log axis) for the plot.
    bg_axis: Vec<f64>,
    acr_vs_bg: Vec<f64>,
}

fn display_api(i: DisplayIn) -> Result<DisplayOut, String> {
    let (p, cfg) = i.base.resolve();
    let a = analysis(&p, &cfg);
    let o = &p.design.optics;
    let an = crate::optics::paraxial::analyze(o, &Default::default());
    let apx = an.arcmin_per_px;
    let acuity = 5.0 * cfg.acuity_20_over / 20.0;
    let rows = [5.0, 7.0, 9.0, 12.0, 16.0, 20.0, 28.0]
        .iter()
        .map(|&px| {
            let am = px * apx;
            LegRow {
                cap_px: px,
                arcmin: am,
                snellen_20_over: 20.0 * am / 5.0,
                verdict: display::legibility_verdict(am, acuity).into(),
            }
        })
        .collect();
    let ph = display::photometry(o, cfg.tint_transmission);
    let mut scenes: Vec<display::SceneRow> = display::SCENES
        .iter()
        .map(|(n, b, dsc)| display::scene_row(&ph, n, dsc, *b))
        .collect();
    if let Some(c) = i.custom_nits.filter(|c| c.is_finite() && *c > 0.0) {
        scenes.push(display::scene_row(&ph, "Custom", "your background", c));
    }
    let bright = display::scene_row(
        &ph,
        "Bright scene (metric)",
        "the scene used for ergonomics.outdoor_contrast",
        cfg.bright_scene_nits,
    );
    let bg_axis = crate::math::logspace(10.0, 20000.0, 80);
    let acr_vs_bg = bg_axis
        .iter()
        .map(|&b| crate::optics::paraxial::contrast_ratio(ph.image_nits, b, ph.see_through))
        .collect();
    let cap = cfg.caption_cap_px * apx;
    Ok(DisplayOut {
        arcmin_per_px: apx,
        ppd: an.ppd,
        ppd_center: an.ppd_center,
        hfov_deg: an.hfov_deg,
        vfov_deg: an.vfov_deg,
        px_w: o.px_w,
        px_h: o.px_h,
        acuity_arcmin: acuity,
        caption_px: cfg.caption_cap_px,
        caption_arcmin: cap,
        caption_verdict: display::legibility_verdict(cap, acuity).into(),
        min_px_16: if apx > 0.0 { (16.0 / apx).ceil() } else { 0.0 },
        min_px_20: if apx > 0.0 { (20.0 / apx).ceil() } else { 0.0 },
        rows,
        photometry: ph,
        scenes,
        bright,
        eyebox: display::eyebox(&a.fg, o, &cfg),
        bg_axis,
        acr_vs_bg,
    })
}

// ------------------------------------------------------------------ dispatch & metrics

/// JSON API for the Human Factors Lab.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    let empty = Value::Object(Default::default());
    let input = if input.is_null() { &empty } else { input };
    match method {
        "ergonomics.defaults" => Some(crate::api::to_value(ErgoConfig::default())),
        "ergonomics.population" => handle(input, |i: BaseIn| {
            let (_, cfg) = i.resolve();
            Ok(anthro::population(&cfg))
        }),
        "ergonomics.fit" => handle(input, fit_api),
        "ergonomics.sizes" => handle(input, sizes_api),
        "ergonomics.head" => handle(input, head_api),
        "ergonomics.balance" => handle(input, balance_api),
        "ergonomics.clamp" => handle(input, clamp_api),
        "ergonomics.display" => handle(input, display_api),
        _ => None,
    }
}

/// Metrics feeding requirement verification.
pub fn metrics(p: &Project) -> Vec<Metric> {
    let cfg = ErgoConfig::from_project(p);
    let a = analysis(p, &cfg);
    let fit_key = crate::memo::key(&(&a.fg, &cfg, &a.band, &a.clamp));
    let (cov, _) = crate::memo::memo(&FIT, fit_key, || {
        let people = anthro::sample(&cfg, cfg.samples, cfg.seed);
        let checks = checks_for(&a, &a.fg, None, &people);
        (anthro::coverage(checks.iter(), a.band), 0.0)
    });
    let h95 = combined_hb_p95(&cfg);
    let f95 = a.clamp.force(h95);
    let eb = display::eyebox(&a.fg, &p.design.optics, &cfg);
    let eb_comb = eb
        .groups
        .iter()
        .find(|g| g.group == Group::Combined)
        .map(|g| g.coverage_pct)
        .unwrap_or(0.0);
    let ph = display::photometry(&p.design.optics, cfg.tint_transmission);
    let bright = display::scene_row(&ph, "bright", "", cfg.bright_scene_nits);
    let apx = crate::optics::paraxial::analyze(&p.design.optics, &Default::default()).arcmin_per_px;
    let pad_p = a.upright.pad_pressure_kpa[0].max(a.upright.pad_pressure_kpa[1]);
    let pct_women = cfg.female_fraction * 100.0;
    vec![
        Metric::new("ergonomics.fit_coverage_pct", "Fit coverage (one size)", cov.fit_pct, "%", "ergonomics").with_note(&format!(
            "{} □ {} – {} on {} simulated adults ({pct_women:.0} % women): PD {:.0} %, temple {:.0} %, clamp band {:.0} % ({:.0} % too tight, {:.0} % too loose)",
            a.fg.lens_width_mm, a.fg.bridge_mm, a.fg.temple_length_mm, cov.n, cov.pd_pct, cov.ear_pct, cov.width_pct, cov.too_tight_pct, cov.too_loose_pct
        )),
        Metric::new("ergonomics.nose_load_g", "Load on the nose", a.upright.nose_gf, "gf", "ergonomics").with_note(&format!(
            "{:.1} g total, centre of mass {:.0} mm behind the front; nose carries {:.0} % (lever rule between pads and ear roots)",
            a.mass_g,
            -a.com[1],
            a.upright.nose_share_pct
        )),
        Metric::new("ergonomics.clamp_force_n", "Temple clamp force at P95 head breadth", f95, "N", "ergonomics").with_note(&format!(
            "per side at {h95:.0} mm head breadth; spread at contact {:.0} mm, compliance {:.2} mm/N ({:.0} % front bending){}; comfort band {:.2}–{:.2} N",
            a.clamp.s0,
            a.clamp.c_total,
            100.0 * a.clamp.c_front / a.clamp.c_total,
            match a.clamp.hinge_crack_n {
                Some(c) => format!("; spring hinge (Mechanisms Lab) opens past {c:.2} N"),
                None => String::new(),
            },
            a.band.min_n,
            a.band.max_n
        )),
        Metric::new("ergonomics.eyebox_coverage_pct", "Eyebox coverage without adjustment", eb_comb, "%", "ergonomics").with_note(&format!(
            "{:.1} mm eyebox centred {:.1} mm from the mid-line accepts IPD {:.1}–{:.1} mm; centring on the median would give {:.0} %",
            eb.eyebox_mm, eb.display_x_mm, eb.ipd_window_mm[0], eb.ipd_window_mm[1], eb.best_coverage_pct
        )),
        Metric::new("ergonomics.outdoor_contrast", "Ambient contrast ratio, bright scene", bright.contrast_ratio, ":1", "ergonomics").with_note(&format!(
            "{:.0} cd/m² sunlit background × {:.2} see-through vs {:.0} nits of image ({})",
            cfg.bright_scene_nits, ph.see_through, ph.image_nits, bright.verdict
        )),
        Metric::new("ergonomics.nose_pad_pressure_kpa", "Nose-pad contact pressure", pad_p, "kPa", "ergonomics").with_note(&format!(
            "{:.2} N normal on {:.0} mm² per pad (flank {:.0}° from vertical); sustained-pressure limit ≈ {:.1} kPa",
            a.upright.pad_normal_n[0].max(a.upright.pad_normal_n[1]),
            a.pad_area_mm2,
            cfg.nose_flank_deg,
            cfg.pressure_limit_kpa
        )),
        Metric::new("ergonomics.caption_arcmin", "Caption cap height (angular)", cfg.caption_cap_px * apx, "arcmin", "ergonomics").with_note(&format!(
            "{:.0} px × {:.2} arcmin/px; comfortable reading ≥ 16–20 arcmin",
            cfg.caption_cap_px, apx
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(m: &[Metric], k: &str) -> f64 {
        m.iter().find(|x| x.key == k).unwrap_or_else(|| panic!("missing {k}")).value
    }

    #[test]
    fn iris_metrics_are_sensible() {
        let p = Project::iris();
        let m = metrics(&p);
        for x in &m {
            assert!(x.value.is_finite(), "{} = {}", x.key, x.value);
            println!("{} = {:.3} {} — {}", x.key, x.value, x.unit, x.note);
        }
        let fit = get(&m, "ergonomics.fit_coverage_pct");
        assert!((0.0..=100.0).contains(&fit));
        let nose = get(&m, "ergonomics.nose_load_g");
        let total = p.component_mass_g();
        assert!(nose > 0.3 * total && nose < 60.0, "{nose}");
        let eb = get(&m, "ergonomics.eyebox_coverage_pct");
        assert!(eb > 40.0 && eb < 80.0, "{eb}");
        let acr = get(&m, "ergonomics.outdoor_contrast");
        assert!((acr - (828.0 + 3500.0) / 3500.0).abs() < 1e-6, "{acr}");
        let f = get(&m, "ergonomics.clamp_force_n");
        assert!(f > 0.0 && f < 50.0, "{f}");
    }

    #[test]
    fn nose_load_matches_the_lever_rule() {
        let p = Project::iris();
        let cfg = ErgoConfig::default();
        let a = analysis(&p, &cfg);
        let (yn, ye) = (a.fg.pad[1], a.fg.ear_bend_y);
        let expect = a.mass_g * (a.com[1] - ye) / (yn - ye);
        assert!((a.upright.nose_gf - expect).abs() < 1e-9, "{} vs {expect}", a.upright.nose_gf);
        // total mass = CAD frame + components
        let frame: f64 = a.items.iter().filter(|i| i.kind == "frame").map(|i| i.mass_g).sum();
        assert!((a.mass_g - frame - p.component_mass_g()).abs() < 1e-9);
    }

    #[test]
    fn clamp_force_scales_with_head_and_splay_relieves_it() {
        // a rigid hinge (no spring hinge in the BOM): a linear spring, ΔF = ΔH / c_total
        let mut p = Project::iris();
        p.components.retain(|c| !c.name.to_lowercase().contains("spring hinge"));
        let a = analysis(&p, &ErgoConfig::default());
        assert!(a.hinge.is_none());
        let f1 = a.clamp.force(150.0);
        let f2 = a.clamp.force(160.0);
        assert!(f2 > f1 && f1 >= 0.0);
        assert!(((f2 - f1) - 10.0 / a.clamp.c_total).abs() < 1e-9 || f1 == 0.0);
        // Iris's spring hinge can only lower the force
        let iris = analysis(&Project::iris(), &ErgoConfig::default());
        assert!(iris.hinge.is_some());
        assert!(iris.clamp.force(160.0) <= f2 + 1e-9);
        let cfg = ErgoConfig {
            temple_splay_deg: 8.0,
            ..Default::default()
        };
        let b = analysis(&p, &cfg);
        assert!(b.clamp.force(160.0) < f2);
    }

    #[test]
    fn api_methods_run() {
        let p = serde_json::to_value(Project::iris()).unwrap();
        for m in ["population", "fit", "sizes", "head", "balance", "clamp", "display"] {
            let r = dispatch(&format!("ergonomics.{m}"), &serde_json::json!({ "project": p })).unwrap();
            assert!(r.is_ok(), "{m}: {:?}", r.err());
        }
        let c = dispatch("ergonomics.clamp", &Value::Null).unwrap().unwrap();
        println!(
            "best splay {} → {} %, band {:?}, p95 {}",
            c["best_splay_deg"], c["best_splay_coverage_pct"], c["band_heads_mm"], c["force_p95_n"]
        );
        let d = dispatch("ergonomics.defaults", &Value::Null).unwrap().unwrap();
        assert_eq!(d["female_fraction"], 0.5);
        // null input works (Iris default)
        assert!(dispatch("ergonomics.clamp", &Value::Null).unwrap().is_ok());
    }

    #[test]
    fn sizes_beat_one_size() {
        let out = sizes_api(SizesIn {
            samples: Some(800),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(out.sizes.len(), 3);
        assert!(out.union_pct >= out.iris_pct);
        assert!(out.sizes.iter().all(|s| s.alone_pct <= out.union_pct + 1e-9));
        println!(
            "sizes {:?} union {:.1} iris {:.1}",
            out.sizes.iter().map(|s| (&s.label, s.alone_pct)).collect::<Vec<_>>(),
            out.union_pct,
            out.iris_pct
        );
    }
}
