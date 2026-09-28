//! Tolerance analysis: 1-D stack-ups (worst case, root-sum-square, Monte
//! Carlo), ISO 286 limits and fits, and the Iris PCB-in-pod stack.
//!
//! ## Stack-ups
//! A gap G = Σ s_i d_i (s_i = ±1 direction). With each dimension
//! d_i = nominal_i + [−lo_i, +hi_i]:
//! * **worst case** — every part at its most unfavourable limit:
//!   G_min = Σ (s_i > 0 ? d_i,min : d_i,max) … guaranteed but pessimistic;
//! * **RSS** (root sum square) — independent, centred, normally distributed
//!   dimensions whose ± tolerance is `k_i` standard deviations:
//!   σ_G = √Σ (t_i / k_i)²; G = G_mean ± 3 σ_G covers 99.73 % of assemblies;
//! * **Monte Carlo** — sample every dimension from its distribution (normal
//!   or uniform) and count how many assemblies land inside the limits.
//!
//! ## ISO 286 limits and fits
//! A fit like H7/g6 combines a *fundamental deviation* (the letter: where
//! the tolerance zone sits relative to the nominal size) and an *IT grade*
//! (the number: how wide the zone is). The standard tolerance and
//! fundamental-deviation values below are the ISO 286-1/-2 table values for
//! nominal sizes up to 500 mm (r and s shafts, and the P hole, up to 120 mm).
//! For holes K, M, N (grades ≤ 8) and P (grades ≤ 7) the upper deviation is
//! ES = −ei + Δ with Δ = IT_n − IT_(n−1) (the ISO rule); holes A–H mirror the
//! shafts (EI = −es).
//!
//! References: ISO 286-1 / ISO 286-2 (Geometrical product specifications —
//! ISO code system for tolerances on linear sizes); Budynas & Nisbett,
//! *Shigley's Mechanical Engineering Design*, §7-8 (limits and fits);
//! A. Creveling, *Tolerance Design*, Addison-Wesley, 1997.

use serde::{Deserialize, Serialize};

use crate::math::Rng;

// ---------------------------------------------------------------- stack-ups

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Dim {
    pub name: String,
    pub nominal: f64,
    /// Upper deviation (+), mm.
    pub plus: f64,
    /// Lower deviation magnitude (−), mm.
    pub minus: f64,
    /// +1 adds to the gap, −1 subtracts.
    pub dir: f64,
    /// "normal" or "uniform".
    pub dist: String,
    /// Standard deviations covered by the tolerance band (normal only), usually 3.
    pub sigmas: f64,
    #[serde(default)]
    pub note: String,
}

impl Default for Dim {
    fn default() -> Self {
        Dim {
            name: "dim".into(),
            nominal: 0.0,
            plus: 0.1,
            minus: 0.1,
            dir: 1.0,
            dist: "normal".into(),
            sigmas: 3.0,
            note: String::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct StackIn {
    pub dims: Vec<Dim>,
    /// Acceptance limits on the gap (lower, upper), mm.
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub samples: usize,
    pub seed: u64,
    pub bins: usize,
}

impl Default for StackIn {
    fn default() -> Self {
        StackIn {
            dims: vec![],
            lower: Some(0.0),
            upper: None,
            samples: 20_000,
            seed: 1,
            bins: 40,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Contribution {
    pub name: String,
    /// Share of the RSS variance, %.
    pub variance_pct: f64,
    /// Share of the worst-case range, %.
    pub range_pct: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct StackOut {
    pub nominal: f64,
    pub mean: f64,
    pub worst_min: f64,
    pub worst_max: f64,
    pub rss_sigma: f64,
    pub rss_min: f64,
    pub rss_max: f64,
    pub mc_mean: f64,
    pub mc_sigma: f64,
    pub mc_min: f64,
    pub mc_max: f64,
    /// Fraction of Monte Carlo assemblies inside the limits, %.
    pub yield_pct: f64,
    /// Normal-theory yield from the RSS model, %.
    pub rss_yield_pct: f64,
    pub cpk: f64,
    pub hist_edges: Vec<f64>,
    pub hist_counts: Vec<u32>,
    /// First samples in draw order (for an animated histogram fill).
    pub samples_head: Vec<f64>,
    pub contributions: Vec<Contribution>,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
}

/// Standard normal CDF via the complementary error function (Abramowitz &
/// Stegun 7.1.26, |error| < 1.5e-7).
pub fn norm_cdf(z: f64) -> f64 {
    let x = z.abs() / std::f64::consts::SQRT_2;
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let y = 1.0 - (((((1.061_405_429 * t - 1.453_152_027) * t) + 1.421_413_741) * t - 0.284_496_736) * t + 0.254_829_592) * t * (-x * x).exp();
    if z >= 0.0 {
        0.5 * (1.0 + y)
    } else {
        0.5 * (1.0 - y)
    }
}

pub fn stackup(i: StackIn) -> Result<StackOut, String> {
    if i.dims.is_empty() {
        return Err("add at least one dimension".into());
    }
    let mut nominal = 0.0;
    let mut mean = 0.0;
    let mut wmin = 0.0;
    let mut wmax = 0.0;
    let mut var = 0.0;
    let mut vars = Vec::new();
    let mut ranges = Vec::new();
    for d in &i.dims {
        let s = if d.dir < 0.0 { -1.0 } else { 1.0 };
        let (lo, hi) = (d.nominal - d.minus.abs(), d.nominal + d.plus.abs());
        nominal += s * d.nominal;
        mean += s * 0.5 * (lo + hi);
        if s > 0.0 {
            wmin += lo;
            wmax += hi;
        } else {
            wmin -= hi;
            wmax -= lo;
        }
        let half = 0.5 * (hi - lo);
        let sd = if d.dist == "uniform" { half / 3f64.sqrt() } else { half / d.sigmas.max(0.5) };
        var += sd * sd;
        vars.push(sd * sd);
        ranges.push(hi - lo);
    }
    let sigma = var.sqrt();
    let n = i.samples.clamp(100, 200_000);
    let mut rng = Rng::new(i.seed.max(1));
    let mut samples = Vec::with_capacity(n);
    for _ in 0..n {
        let mut g = 0.0;
        for d in &i.dims {
            let s = if d.dir < 0.0 { -1.0 } else { 1.0 };
            let (lo, hi) = (d.nominal - d.minus.abs(), d.nominal + d.plus.abs());
            let c = 0.5 * (lo + hi);
            let half = 0.5 * (hi - lo);
            let v = if d.dist == "uniform" {
                lo + (hi - lo) * rng.uniform()
            } else {
                c + half / d.sigmas.max(0.5) * rng.normal()
            };
            g += s * v;
        }
        samples.push(g);
    }
    let mc_mean = samples.iter().sum::<f64>() / n as f64;
    let mc_sigma = (samples.iter().map(|x| (x - mc_mean).powi(2)).sum::<f64>() / (n as f64 - 1.0)).sqrt();
    let mc_min = samples.iter().cloned().fold(f64::INFINITY, f64::min);
    let mc_max = samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let inside = |g: f64| i.lower.is_none_or(|l| g >= l) && i.upper.is_none_or(|u| g <= u);
    let ok = samples.iter().filter(|g| inside(**g)).count();
    let p_lo = i.lower.map(|l| norm_cdf((l - mean) / sigma.max(1e-12))).unwrap_or(0.0);
    let p_hi = i.upper.map(|u| 1.0 - norm_cdf((u - mean) / sigma.max(1e-12))).unwrap_or(0.0);
    let cpk = match (i.lower, i.upper) {
        (Some(l), Some(u)) => ((mean - l).min(u - mean)) / (3.0 * sigma),
        (Some(l), None) => (mean - l) / (3.0 * sigma),
        (None, Some(u)) => (u - mean) / (3.0 * sigma),
        _ => f64::INFINITY,
    };
    let bins = i.bins.clamp(5, 200);
    let lo = (mean - 4.0 * sigma).min(mc_min);
    let hi = (mean + 4.0 * sigma).max(mc_max);
    let w = (hi - lo).max(1e-9) / bins as f64;
    let edges: Vec<f64> = (0..=bins).map(|k| lo + w * k as f64).collect();
    let mut counts = vec![0u32; bins];
    for g in &samples {
        let k = (((g - lo) / w) as usize).min(bins - 1);
        counts[k] += 1;
    }
    let tot_range: f64 = ranges.iter().sum();
    let contributions = i
        .dims
        .iter()
        .enumerate()
        .map(|(k, d)| Contribution {
            name: d.name.clone(),
            variance_pct: if var > 0.0 { 100.0 * vars[k] / var } else { 0.0 },
            range_pct: if tot_range > 0.0 { 100.0 * ranges[k] / tot_range } else { 0.0 },
        })
        .collect();
    Ok(StackOut {
        nominal,
        mean,
        worst_min: wmin,
        worst_max: wmax,
        rss_sigma: sigma,
        rss_min: mean - 3.0 * sigma,
        rss_max: mean + 3.0 * sigma,
        mc_mean,
        mc_sigma,
        mc_min,
        mc_max,
        yield_pct: 100.0 * ok as f64 / n as f64,
        rss_yield_pct: 100.0 * (1.0 - p_lo - p_hi),
        cpk,
        hist_edges: edges,
        hist_counts: counts,
        samples_head: samples.iter().take(4000).copied().collect(),
        contributions,
        lower: i.lower,
        upper: i.upper,
    })
}

// ---------------------------------------------------------------- ISO 286

/// Upper bounds of the ISO 286 nominal size ranges, mm ("over a up to and including b").
const RANGES: [f64; 13] = [3.0, 6.0, 10.0, 18.0, 30.0, 50.0, 80.0, 120.0, 180.0, 250.0, 315.0, 400.0, 500.0];

/// Standard tolerance grades IT4–IT13, µm (rows = size ranges).
const IT: [[f64; 10]; 13] = [
    [3.0, 4.0, 6.0, 10.0, 14.0, 25.0, 40.0, 60.0, 100.0, 140.0],
    [4.0, 5.0, 8.0, 12.0, 18.0, 30.0, 48.0, 75.0, 120.0, 180.0],
    [4.0, 6.0, 9.0, 15.0, 22.0, 36.0, 58.0, 90.0, 150.0, 220.0],
    [5.0, 8.0, 11.0, 18.0, 27.0, 43.0, 70.0, 110.0, 180.0, 270.0],
    [6.0, 9.0, 13.0, 21.0, 33.0, 52.0, 84.0, 130.0, 210.0, 330.0],
    [7.0, 11.0, 16.0, 25.0, 39.0, 62.0, 100.0, 160.0, 250.0, 390.0],
    [8.0, 13.0, 19.0, 30.0, 46.0, 74.0, 120.0, 190.0, 300.0, 460.0],
    [10.0, 15.0, 22.0, 35.0, 54.0, 87.0, 140.0, 220.0, 350.0, 540.0],
    [12.0, 18.0, 25.0, 40.0, 63.0, 100.0, 160.0, 250.0, 400.0, 630.0],
    [14.0, 20.0, 29.0, 46.0, 72.0, 115.0, 185.0, 290.0, 460.0, 720.0],
    [16.0, 23.0, 32.0, 52.0, 81.0, 130.0, 210.0, 320.0, 520.0, 810.0],
    [18.0, 25.0, 36.0, 57.0, 89.0, 140.0, 230.0, 360.0, 570.0, 890.0],
    [20.0, 27.0, 40.0, 63.0, 97.0, 155.0, 250.0, 400.0, 630.0, 970.0],
];

/// Shaft fundamental deviations, µm: es for d–h, ei for k–p (per range).
const FD_D: [f64; 13] = [-20.0, -30.0, -40.0, -50.0, -65.0, -80.0, -100.0, -120.0, -145.0, -170.0, -190.0, -210.0, -230.0];
const FD_E: [f64; 13] = [-14.0, -20.0, -25.0, -32.0, -40.0, -50.0, -60.0, -72.0, -85.0, -100.0, -110.0, -125.0, -135.0];
const FD_F: [f64; 13] = [-6.0, -10.0, -13.0, -16.0, -20.0, -25.0, -30.0, -36.0, -43.0, -50.0, -56.0, -62.0, -68.0];
const FD_G: [f64; 13] = [-2.0, -4.0, -5.0, -6.0, -7.0, -9.0, -10.0, -12.0, -14.0, -15.0, -17.0, -18.0, -20.0];
/// k: IT4–IT7 values (0 for IT ≤ 3 and > 7).
const FD_K: [f64; 13] = [0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0, 4.0, 5.0];
const FD_M: [f64; 13] = [2.0, 4.0, 6.0, 7.0, 8.0, 9.0, 11.0, 13.0, 15.0, 17.0, 20.0, 21.0, 23.0];
const FD_N: [f64; 13] = [4.0, 8.0, 10.0, 12.0, 15.0, 17.0, 20.0, 23.0, 27.0, 31.0, 34.0, 37.0, 40.0];
const FD_P: [f64; 13] = [6.0, 12.0, 15.0, 18.0, 22.0, 26.0, 32.0, 37.0, 43.0, 50.0, 56.0, 62.0, 68.0];

/// r and s shafts need the finer sub-ranges above 50 mm; supported up to 120 mm.
fn fd_rs(letter: char, d: f64) -> Option<f64> {
    let r = [(3.0, 10.0), (6.0, 15.0), (10.0, 19.0), (18.0, 23.0), (30.0, 28.0), (50.0, 34.0), (65.0, 41.0), (80.0, 43.0), (100.0, 51.0), (120.0, 54.0)];
    let s = [(3.0, 14.0), (6.0, 19.0), (10.0, 23.0), (18.0, 28.0), (30.0, 35.0), (50.0, 43.0), (65.0, 53.0), (80.0, 59.0), (100.0, 71.0), (120.0, 79.0)];
    let t = if letter == 'r' { &r } else { &s };
    t.iter().find(|(ub, _)| d <= *ub + 1e-9).map(|(_, v)| *v)
}

fn range_idx(d: f64) -> Option<usize> {
    if !(d > 0.0) {
        return None;
    }
    RANGES.iter().position(|ub| d <= *ub + 1e-9)
}

pub fn it_um(d: f64, grade: u32) -> Option<f64> {
    let r = range_idx(d)?;
    if !(4..=13).contains(&grade) {
        return None;
    }
    Some(IT[r][(grade - 4) as usize])
}

fn parse_code(code: &str) -> Option<(String, u32)> {
    let code = code.trim();
    let split = code.find(|c: char| c.is_ascii_digit())?;
    let (l, g) = code.split_at(split);
    Some((l.to_string(), g.parse().ok()?))
}

/// Shaft limits (es, ei) in µm.
pub fn shaft_limits(d: f64, code: &str) -> Result<(f64, f64), String> {
    let (l, g) = parse_code(code).ok_or_else(|| format!("bad shaft code `{code}`"))?;
    let r = range_idx(d).ok_or("nominal size must be in (0, 500] mm")?;
    let it = it_um(d, g).ok_or_else(|| format!("IT{g} not supported (IT4–IT13)"))?;
    let upper_from = |es: f64| Ok((es, es - it));
    let lower_from = |ei: f64| Ok((ei + it, ei));
    match l.as_str() {
        "d" => upper_from(FD_D[r]),
        "e" => upper_from(FD_E[r]),
        "f" => upper_from(FD_F[r]),
        "g" => upper_from(FD_G[r]),
        "h" => upper_from(0.0),
        "js" => Ok((it / 2.0, -it / 2.0)),
        "k" => lower_from(if (4..=7).contains(&g) { FD_K[r] } else { 0.0 }),
        "m" => lower_from(FD_M[r]),
        "n" => lower_from(FD_N[r]),
        "p" => lower_from(FD_P[r]),
        "r" | "s" => {
            let v = fd_rs(l.chars().next().unwrap_or('r'), d).ok_or("r/s shafts supported up to 120 mm")?;
            lower_from(v)
        }
        _ => Err(format!("shaft letter `{l}` not supported (d e f g h js k m n p r s)")),
    }
}

/// Hole limits (ES, EI) in µm.
pub fn hole_limits(d: f64, code: &str) -> Result<(f64, f64), String> {
    let (l, g) = parse_code(code).ok_or_else(|| format!("bad hole code `{code}`"))?;
    let r = range_idx(d).ok_or("nominal size must be in (0, 500] mm")?;
    let it = it_um(d, g).ok_or_else(|| format!("IT{g} not supported (IT4–IT13)"))?;
    let delta = if g > 4 { it - it_um(d, g - 1).unwrap_or(it) } else { 0.0 };
    let lower_from = |ei: f64| Ok((ei + it, ei));
    let upper_from = |es: f64| Ok((es, es - it));
    match l.as_str() {
        "D" => lower_from(-FD_D[r]),
        "E" => lower_from(-FD_E[r]),
        "F" => lower_from(-FD_F[r]),
        "G" => lower_from(-FD_G[r]),
        "H" => lower_from(0.0),
        "JS" => Ok((it / 2.0, -it / 2.0)),
        "K" if g <= 8 => upper_from(-(if (4..=7).contains(&g) { FD_K[r] } else { 0.0 }) + delta),
        "M" if g <= 8 => upper_from(-FD_M[r] + delta),
        "N" if g <= 8 => upper_from(-FD_N[r] + delta),
        "P" if g <= 7 && d <= 120.0 => upper_from(-FD_P[r] + delta),
        _ => Err(format!("hole `{code}` not supported (D E F G H JS; K M N ≤ IT8; P ≤ IT7)")),
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct FitIn {
    pub nominal_mm: f64,
    pub hole: String,
    pub shaft: String,
}

impl Default for FitIn {
    fn default() -> Self {
        FitIn {
            nominal_mm: 1.5,
            hole: "H7".into(),
            shaft: "g6".into(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct FitOut {
    pub nominal_mm: f64,
    pub hole: String,
    pub shaft: String,
    /// Hole upper/lower deviation, µm.
    pub hole_es_um: f64,
    pub hole_ei_um: f64,
    pub shaft_es_um: f64,
    pub shaft_ei_um: f64,
    pub hole_max_mm: f64,
    pub hole_min_mm: f64,
    pub shaft_max_mm: f64,
    pub shaft_min_mm: f64,
    /// Positive = clearance, negative = interference, µm.
    pub max_clearance_um: f64,
    pub min_clearance_um: f64,
    pub fit_type: String,
    pub description: String,
}

pub fn fit(i: FitIn) -> Result<FitOut, String> {
    let d = i.nominal_mm;
    let (hes, hei) = hole_limits(d, &i.hole)?;
    let (ses, sei) = shaft_limits(d, &i.shaft)?;
    let cmax = hes - sei;
    let cmin = hei - ses;
    let fit_type = if cmin >= 0.0 {
        "clearance"
    } else if cmax <= 0.0 {
        "interference"
    } else {
        "transition"
    };
    let description = match (i.hole.as_str(), i.shaft.as_str()) {
        ("H9", "d9") => "Free running fit — loose, for large temperature swings.",
        ("H8", "f7") => "Close running fit — accurate location at moderate speeds.",
        ("H7", "g6") => "Sliding fit — parts move and turn freely but locate accurately (hinge pins, guides).",
        ("H7", "h6") => "Locational clearance fit — snug, assembled by hand, stationary parts.",
        ("H7", "k6") => "Locational transition fit — accurate location, slight interference possible.",
        ("H7", "n6") => "Locational transition fit — more accurate, light press.",
        ("H7", "p6") => "Locational interference fit — pressed; rigid, accurate location (light press fit).",
        ("H7", "s6") => "Medium drive fit — permanent assembly, shrink or heavy press.",
        _ => match fit_type {
            "clearance" => "Clearance fit — always a gap.",
            "interference" => "Interference fit — always pressed.",
            _ => "Transition fit — may be slightly loose or slightly tight.",
        },
    };
    Ok(FitOut {
        nominal_mm: d,
        hole: i.hole.clone(),
        shaft: i.shaft.clone(),
        hole_es_um: hes,
        hole_ei_um: hei,
        shaft_es_um: ses,
        shaft_ei_um: sei,
        hole_max_mm: d + hes / 1000.0,
        hole_min_mm: d + hei / 1000.0,
        shaft_max_mm: d + ses / 1000.0,
        shaft_min_mm: d + sei / 1000.0,
        max_clearance_um: cmax,
        min_clearance_um: cmin,
        fit_type: fit_type.into(),
        description: description.into(),
    })
}

/// Common fits for the calculator's quick picks (Shigley's preferred fits).
pub fn common_fits() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("H9", "d9", "Free running"),
        ("H8", "f7", "Close running"),
        ("H7", "g6", "Sliding"),
        ("H7", "h6", "Locational clearance"),
        ("H7", "k6", "Locational transition"),
        ("H7", "n6", "Locational transition (tight)"),
        ("H7", "p6", "Locational interference"),
        ("H7", "s6", "Medium drive"),
    ]
}

// ---------------------------------------------------------------- Iris pod stack

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct PodStackIn {
    /// "fdm" or "molded".
    pub process: String,
    /// Pod inner length tolerance (±, mm); default from the process.
    pub pod_tol_mm: Option<f64>,
    /// PCB outline tolerance (±, mm).
    pub pcb_tol_mm: f64,
    /// Edge-component overhang beyond the board outline: nominal and ±, mm.
    pub overhang_mm: f64,
    pub overhang_tol_mm: f64,
    /// Include the cold-temperature thermal mismatch term.
    pub thermal: bool,
    pub t_min_c: f64,
    pub t_max_c: f64,
    pub samples: usize,
}

impl Default for PodStackIn {
    fn default() -> Self {
        PodStackIn {
            process: "fdm".into(),
            pod_tol_mm: None,
            pcb_tol_mm: 0.15,
            overhang_mm: 0.0,
            overhang_tol_mm: 0.1,
            thermal: true,
            t_min_c: -10.0,
            t_max_c: 50.0,
            samples: 20_000,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ThermalOut {
    pub pod_cte_ppm_k: f64,
    pub pcb_cte_ppm_k: f64,
    pub temps_c: Vec<f64>,
    /// Change in clearance vs 23 °C, mm (negative = tighter).
    pub delta_mm: Vec<f64>,
    pub delta_cold_mm: f64,
    pub delta_hot_mm: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct PodStackOut {
    pub dims: Vec<Dim>,
    pub stack: StackOut,
    pub pod_inner_mm: f64,
    pub pcb_len_mm: f64,
    pub thermal: ThermalOut,
    pub process: String,
    pub notes: Vec<String>,
}

/// Build the Iris PCB-in-pod length stack from the CAD layout and the
/// project's PCB, and analyse it.
pub fn pod_stack(p: &crate::project::Project, i: &PodStackIn) -> Result<PodStackOut, String> {
    let l = crate::cad::frame_geom(p).layout.clone();
    let pod_in = l.pod_usable_len_mm;
    let pcb = p.design.pcb.width_mm;
    let molded = i.process == "molded";
    let pod_tol = i.pod_tol_mm.unwrap_or(if molded { 0.10 } else { 0.20 });
    let pod_mat = super::materials::get(if molded { &p.design.manufacturing.molding_material } else { &p.design.frame.material });
    let fr4 = super::materials::get("FR-4");
    let temps: Vec<f64> = (0..=24).map(|k| i.t_min_c + (i.t_max_c - i.t_min_c) * k as f64 / 24.0).collect();
    let dalpha = (pod_mat.cte_ppm_k - fr4.cte_ppm_k) * 1e-6;
    let delta: Vec<f64> = temps.iter().map(|t| dalpha * pod_in * (t - 23.0)).collect();
    let cold = dalpha * pod_in * (i.t_min_c - 23.0);
    let hot = dalpha * pod_in * (i.t_max_c - 23.0);
    let mut dims = vec![
        Dim {
            name: "Pod inner length (usable, at the floor)".into(),
            nominal: pod_in,
            plus: pod_tol,
            minus: pod_tol,
            dir: 1.0,
            dist: "normal".into(),
            sigmas: 3.0,
            note: if molded {
                "Injection molded: ±0.10 mm typical for a 60 mm feature in a well-controlled tool (check the molder's tolerance class).".into()
            } else {
                "FDM print: ±0.2 mm typical; internal pockets often print slightly small — calibrate on a test pod.".into()
            },
        },
        Dim {
            name: "PCB length (outline)".into(),
            nominal: pcb,
            plus: i.pcb_tol_mm,
            minus: i.pcb_tol_mm,
            dir: -1.0,
            dist: "normal".into(),
            sigmas: 3.0,
            note: "Routed board outline: ±0.1–0.2 mm typical — check the fab's capability sheet.".into(),
        },
        Dim {
            name: "Edge-component overhang".into(),
            nominal: i.overhang_mm,
            plus: i.overhang_tol_mm,
            minus: i.overhang_tol_mm,
            dir: -1.0,
            dist: "uniform".into(),
            sigmas: 3.0,
            note: "Parts that overhang the board edge (none on Iris: USB-C exits through the pod floor); ± placement tolerance.".into(),
        },
    ];
    if i.thermal && cold < 0.0 {
        dims.push(Dim {
            name: format!("Thermal mismatch at {:.0} °C", i.t_min_c),
            nominal: cold / 2.0,
            plus: -cold / 2.0,
            minus: -cold / 2.0,
            dir: 1.0,
            dist: "uniform".into(),
            sigmas: 3.0,
            note: format!(
                "Pod ({:.0} ppm/K) shrinks more than FR-4 ({:.0} ppm/K) in the cold: up to {:.3} mm tighter.",
                pod_mat.cte_ppm_k, fr4.cte_ppm_k, -cold
            ),
        });
    }
    let stack = stackup(StackIn {
        dims: dims.clone(),
        lower: Some(0.0),
        upper: None,
        samples: i.samples,
        seed: 7,
        bins: 40,
    })?;
    Ok(PodStackOut {
        dims,
        stack,
        pod_inner_mm: pod_in,
        pcb_len_mm: pcb,
        thermal: ThermalOut {
            pod_cte_ppm_k: pod_mat.cte_ppm_k,
            pcb_cte_ppm_k: fr4.cte_ppm_k,
            temps_c: temps,
            delta_mm: delta,
            delta_cold_mm: cold,
            delta_hot_mm: hot,
        },
        process: if molded { "molded".into() } else { "fdm".into() },
        notes: vec![
            "Clearance = pod usable inner length − PCB length − edge overhang (+ thermal). Positive = the board drops in.".into(),
            "Worst case stacks every limit at once; RSS and Monte Carlo assume independent, centred dimensions.".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso286_known_values() {
        // H7/g6 at 25 mm: H7 = +21/0, g6 = −7/−20 (ISO 286-2 tables).
        let f = fit(FitIn {
            nominal_mm: 25.0,
            hole: "H7".into(),
            shaft: "g6".into(),
        })
        .unwrap();
        assert_eq!((f.hole_es_um, f.hole_ei_um), (21.0, 0.0));
        assert_eq!((f.shaft_es_um, f.shaft_ei_um), (-7.0, -20.0));
        assert_eq!(f.fit_type, "clearance");
        assert_eq!((f.max_clearance_um, f.min_clearance_um), (41.0, 7.0));
        // H7/p6 at 25 mm: p6 = +35/+22 → interference.
        let f = fit(FitIn {
            nominal_mm: 25.0,
            hole: "H7".into(),
            shaft: "p6".into(),
        })
        .unwrap();
        assert_eq!((f.shaft_es_um, f.shaft_ei_um), (35.0, 22.0));
        assert_eq!(f.fit_type, "interference");
        // Holes with the Δ rule at 18–30 mm: K7 +6/−15, N7 −7/−28, P7 −14/−35.
        assert_eq!(hole_limits(25.0, "K7").unwrap(), (6.0, -15.0));
        assert_eq!(hole_limits(25.0, "N7").unwrap(), (-7.0, -28.0));
        assert_eq!(hole_limits(25.0, "P7").unwrap(), (-14.0, -35.0));
        // Range boundary: 3 mm belongs to "up to 3".
        assert_eq!(it_um(3.0, 7), Some(10.0));
        assert_eq!(it_um(3.01, 7), Some(12.0));
        // H7/s6 at 50 mm: s6 = +59/+43.
        assert_eq!(shaft_limits(50.0, "s6").unwrap(), (59.0, 43.0));
    }

    #[test]
    fn stack_worst_rss_mc() {
        let dims = vec![
            Dim {
                name: "a".into(),
                nominal: 10.0,
                plus: 0.1,
                minus: 0.1,
                dir: 1.0,
                ..Default::default()
            },
            Dim {
                name: "b".into(),
                nominal: 9.0,
                plus: 0.2,
                minus: 0.2,
                dir: -1.0,
                ..Default::default()
            },
        ];
        let o = stackup(StackIn {
            dims,
            lower: Some(0.0),
            upper: None,
            samples: 50_000,
            ..Default::default()
        })
        .unwrap();
        assert!((o.nominal - 1.0).abs() < 1e-12);
        assert!((o.worst_min - 0.7).abs() < 1e-12 && (o.worst_max - 1.3).abs() < 1e-12);
        let s = ((0.1f64 / 3.0).powi(2) + (0.2f64 / 3.0).powi(2)).sqrt();
        assert!((o.rss_sigma - s).abs() < 1e-12);
        assert!((o.mc_sigma - s).abs() < 0.02 * s, "{} vs {s}", o.mc_sigma);
        assert!(o.yield_pct > 99.99);
        assert!((norm_cdf(1.0) - 0.841_344_7).abs() < 1e-6);
    }

    #[test]
    fn iris_pod_stack() {
        let p = crate::project::Project::iris();
        let o = pod_stack(&p, &PodStackIn::default()).unwrap();
        assert!(o.stack.nominal > 0.0);
        assert!(o.stack.worst_min < o.stack.rss_min && o.stack.rss_min < o.stack.mean);
        assert!(o.thermal.delta_cold_mm < 0.0);
    }
}
