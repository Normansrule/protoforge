//! Design levers, measured: which numeric design parameters move a metric,
//! in which direction and by how much — and a one-parameter solver that
//! finds the value bringing a requirement to its target.
//!
//! Nothing here is claimed from a table of "known" sensitivities. For each
//! metric the owning lab module (the same per-module functions
//! [`super::super::METRIC_MODULES`] lists) is re-run on a copy of the
//! project with one candidate parameter nudged by a small step, and the
//! change is reported as measured. Candidates come from a curated shortlist
//! of the parameters each module reads ([`SHORTLIST`]) so the measurement
//! stays interactive; environment and use conditions (ambient temperature,
//! uses per day, the build quantity a cost requirement is stated at) are
//! deliberately not levers ([`DENY`]).

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::{check, evaluate, system_metrics, Evaluation, Status, METRIC_MODULES};
use crate::project::{Cmp, Metric, Project, ReqKind, Verification};

/// Relative step used to measure a lever (5 % of the parameter's value;
/// integer parameters move by at least 1).
pub const STEP_REL: f64 = 0.05;

/// Margin the solver aims for above the bare target (2 % of the target), so
/// the fixed design is not sitting exactly on the limit.
pub const SOLVE_MARGIN: f64 = 0.02;

/// Modules whose single run is expensive (3-D finite elements, autorouting,
/// cost models that re-run the router, arm planning). Their levers are
/// measured on request ([`levers_for_metric`]) rather than inline.
pub const LAZY_MODULES: &[&str] = &["structures", "fluids", "robotics", "pcb", "mfg", "molding"];

/// Candidate parameters per owning module. `sec.*` = every numeric field of
/// `design.sec`; `extra.x.**` = every numeric leaf under `design.extra.x`;
/// anything else is one exact path.
pub const SHORTLIST: &[(&str, &[&str])] = &[
    (
        "power",
        &[
            "battery.capacity_mah",
            "battery.usable_fraction",
            "battery.internal_resistance_ohm",
            "power_rails.buck_efficiency",
            "power_rails.quiescent_ua",
            "thermal.r_pod_ambient_c_per_w",
            "thermal.r_board_pod_c_per_w",
            "thermal.c_pod_j_per_c",
            "thermal.c_board_j_per_c",
        ],
    ),
    ("optics", &["optics.*"]),
    ("circuit", &["power_rails.core_v", "power_rails.display_rail_v"]),
    ("dsp", &["audio.*", "chip.coeff_bits"]),
    ("chip", &["chip.*", "audio.fir_taps", "audio.decimation"]),
    ("control", &["imu.*"]),
    ("robotics", &["pcb.width_mm", "pcb.height_mm", "extra.pnp.**"]),
    (
        "rl",
        &[
            "battery.capacity_mah",
            "power_rails.buck_efficiency",
            "power_rails.quiescent_ua",
            "extra.rl_power.**",
        ],
    ),
    ("cad", &["frame.*", "pcb.width_mm", "pcb.height_mm"]),
    ("print", &["print.*", "frame.wall_mm"]),
    ("pcb", &["pcb.*", "audio.mic_spacing_mm"]),
    ("molding", &["frame.wall_mm", "frame.draft_deg", "manufacturing.cavities"]),
    (
        "mfg",
        &[
            "manufacturing.cavities",
            "manufacturing.assembly_minutes",
            "manufacturing.labor_rate_usd_h",
            "frame.wall_mm",
            "pcb.width_mm",
            "pcb.height_mm",
        ],
    ),
    (
        "structures",
        &[
            "frame.front_thickness_mm",
            "frame.rim_width_mm",
            "frame.temple_width_mm",
            "frame.temple_height_mm",
            "frame.wall_mm",
            "optics.display_to_lens_mm",
            "pcb.width_mm",
            "extra.focus_stack.**",
        ],
    ),
    ("quantum", &["extra.qkd.**", "extra.qrng.**"]),
    (
        "firmware",
        &[
            "optics.px_w",
            "optics.px_h",
            "imu.rate_hz",
            "audio.sample_rate_hz",
            "audio.mic_count",
            "battery.capacity_mah",
            "extra.firmware_resources.**",
            "extra.firmware_link.**",
            "extra.firmware_display.**",
        ],
    ),
    (
        "build",
        &[
            "extra.stencil_thickness_mm",
            "pcb.min_trace_mm",
            "pcb.min_clearance_mm",
            "pcb.via_drill_mm",
            "pcb.via_pad_mm",
            "pcb.layers",
            "pcb.thickness_mm",
        ],
    ),
    ("program", &["extra.program.**"]),
    (
        "reliability",
        &["extra.reliability.**", "battery.capacity_mah", "power_rails.quiescent_ua"],
    ),
    (
        "ergonomics",
        &[
            "extra.ergonomics.**",
            "optics.display_luminance_nits",
            "optics.combiner_reflectance",
            "optics.eye_relief_mm",
            "optics.lens_diameter_mm",
            "optics.display_width_mm",
            "optics.px_w",
            "frame.bridge_mm",
            "frame.lens_width_mm",
            "frame.temple_length_mm",
            "frame.pantoscopic_tilt_deg",
            "frame.front_thickness_mm",
            "frame.temple_width_mm",
            "frame.temple_height_mm",
            "frame.pod_length_mm",
        ],
    ),
    ("mechanisms", &["extra.mechanisms.**", "frame.temple_length_mm"]),
    (
        "fluids",
        &[
            "extra.fluids.**",
            "frame.wall_mm",
            "frame.pod_length_mm",
            "frame.pod_width_mm",
            "frame.pod_height_mm",
            "pcb.layers",
            "pcb.thickness_mm",
        ],
    ),
    ("rf", &["extra.rf.**", "pcb.thickness_mm"]),
    (
        "systems",
        &[
            "frame.wall_mm",
            "frame.front_thickness_mm",
            "frame.rim_width_mm",
            "frame.temple_width_mm",
            "frame.temple_height_mm",
            "frame.pod_length_mm",
            "frame.pod_width_mm",
            "frame.pod_height_mm",
            "frame.battery_pod_length_mm",
            "pcb.width_mm",
            "battery.capacity_mah",
            "battery.usable_fraction",
            "power_rails.buck_efficiency",
            "power_rails.quiescent_ua",
        ],
    ),
];

/// Parameter names that describe the environment, the use profile, a
/// material constant or the condition a requirement is stated at — not
/// design choices — plus simulation settings.
pub const DENY: &[&str] = &[
    "ambient_c",
    "skin_limit_c",
    "cell_budget",
    "target_quantity",
    "density_g_cm3",
    "nominal_v",
    "full_v",
    "cutoff_v",
    "uses_per_day",
    "wide_pulls_per_day",
    "design_years",
    "head_width_mm",
    "filament_diameter_mm",
    "seed",
    "samples",
    "distance_km",
    "fiber_loss_db_km",
    "hours_per_day",
    "years",
];

/// Which module computes a metric (by key prefix), or `None` for an
/// unknown prefix.
pub fn owner(key: &str) -> Option<&'static str> {
    let prefix = key.split('.').next()?;
    if prefix == "thermal" {
        return Some("power");
    }
    if ["mass", "fit", "bom", "budget"].contains(&prefix) {
        return Some("systems");
    }
    METRIC_MODULES.iter().find(|(n, _)| *n == prefix).map(|(n, _)| *n)
}

/// Re-run one module's metrics (the systems roll-ups need the CAD module's
/// frame mass and pod size first).
pub fn owner_metrics(module: &str, p: &Project) -> Vec<Metric> {
    if module == "systems" {
        let cad = crate::cad::metrics(p);
        return system_metrics(p, &cad);
    }
    METRIC_MODULES
        .iter()
        .find(|(n, _)| *n == module)
        .map(|(_, f)| f(p))
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Parameters: discovery, bounds, labels, editing
// ---------------------------------------------------------------------------

/// A numeric design parameter found by walking `project.design`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    /// Dotted path under `design` (array indices as numbers), e.g. `extra.rf.network.0.value`.
    pub path: String,
    pub value: f64,
    /// Stored as an integer (moves in whole steps).
    pub integer: bool,
}

fn leaf_name(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

fn push_leaf(out: &mut Vec<Candidate>, path: String, v: &Value) {
    let Some(x) = v.as_f64() else { return };
    if !x.is_finite() || x <= 0.0 || DENY.contains(&leaf_name(&path)) || out.iter().any(|c| c.path == path) {
        return;
    }
    out.push(Candidate {
        integer: v.is_u64() || v.is_i64(),
        value: x,
        path,
    });
}

fn walk(out: &mut Vec<Candidate>, prefix: &str, v: &Value, depth: usize) {
    if depth > 6 {
        return;
    }
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                walk(out, &format!("{prefix}.{k}"), x, depth + 1);
            }
        }
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate().take(32) {
                walk(out, &format!("{prefix}.{i}"), x, depth + 1);
            }
        }
        _ => push_leaf(out, prefix.to_string(), v),
    }
}

/// The shortlist patterns for a module.
pub fn shortlist(module: &str) -> &'static [&'static str] {
    SHORTLIST.iter().find(|(m, _)| *m == module).map(|(_, l)| *l).unwrap_or(&[])
}

/// Candidate levers for a module, expanded against this project's design
/// (`design` is `serde_json::to_value(&project.design)`). Zero, negative
/// (often "auto" sentinels) and non-numeric values are skipped.
pub fn candidates(design: &Value, module: &str) -> Vec<Candidate> {
    let mut out = Vec::new();
    for pat in shortlist(module) {
        if let Some(sec) = pat.strip_suffix(".**") {
            if let Some(v) = design.pointer(&format!("/{}", sec.replace('.', "/"))) {
                walk(&mut out, sec, v, 0);
            }
        } else if let Some(sec) = pat.strip_suffix(".*") {
            if let Some(Value::Object(m)) = design.get(sec) {
                for (k, x) in m {
                    push_leaf(&mut out, format!("{sec}.{k}"), x);
                }
            }
        } else if let Some(v) = design.pointer(&format!("/{}", pat.replace('.', "/"))) {
            push_leaf(&mut out, pat.to_string(), v);
        }
    }
    out
}

/// Physical bounds for a parameter, by name: fractions stay inside (0, 1),
/// percentages inside (0, 100], counts at least 1, everything else positive.
pub fn bounds(path: &str, value: f64, integer: bool) -> (f64, f64) {
    let n = leaf_name(path);
    let fraction = [
        "fraction",
        "reflectance",
        "efficiency",
        "alpha",
        "duty",
        "transmission",
        "probability",
    ]
    .iter()
    .any(|w| n.contains(w))
        || (value <= 1.0 && n.ends_with("_frac"));
    if fraction {
        (1e-3, 0.999)
    } else if n.ends_with("_pct") {
        (1e-3, 100.0)
    } else if integer {
        (1.0, f64::INFINITY)
    } else {
        (value.abs() * 1e-3, f64::INFINITY)
    }
}

/// Unit suffixes used in parameter names, longest first.
const UNITS: &[(&str, &str)] = &[
    ("_dps_rthz", "°/s/√Hz"),
    ("_mg_rthz", "mg/√Hz"),
    ("_c_per_w", "°C/W"),
    ("_j_per_c", "J/°C"),
    ("_g_cm3", "g/cm³"),
    ("_usd_h", "USD/h"),
    ("_mm_s", "mm/s"),
    ("_mm2", "mm²"),
    ("_nits", "cd/m²"),
    ("_mah", "mAh"),
    ("_mhz", "MHz"),
    ("_khz", "kHz"),
    ("_ohm", "Ω"),
    ("_pct", "%"),
    ("_deg", "°"),
    ("_dps", "°/s"),
    ("_kpa", "kPa"),
    ("_nmm", "N·mm"),
    ("_minutes", "min"),
    ("_bits", "bits"),
    ("_mm", "mm"),
    ("_um", "µm"),
    ("_mw", "mW"),
    ("_ua", "µA"),
    ("_ma", "mA"),
    ("_hz", "Hz"),
    ("_ns", "ns"),
    ("_db", "dB"),
    ("_km", "km"),
    ("_w", "W"),
    ("_v", "V"),
    ("_s", "s"),
    ("_h", "h"),
    ("_c", "°C"),
    ("_g", "g"),
    ("_n", "N"),
];

/// Plain names for parameters whose field names do not read well.
const NAMES: &[(&str, &str, &str)] = &[
    ("px_w", "horizontal pixels", "px"),
    ("px_h", "vertical pixels", "px"),
    ("r_pod_ambient_c_per_w", "pod-to-air thermal resistance", "°C/W"),
    ("r_board_pod_c_per_w", "board-to-pod thermal resistance", "°C/W"),
    ("c_pod_j_per_c", "pod heat capacity", "J/°C"),
    ("c_board_j_per_c", "board heat capacity", "J/°C"),
    ("quiescent_ua", "quiescent current", "µA"),
    ("lens_focal_mm", "lens focal length", "mm"),
    ("display_to_lens_mm", "display-to-lens distance", "mm"),
    ("lens_to_combiner_mm", "lens-to-combiner distance", "mm"),
    ("wall_mm", "wall thickness", "mm"),
    ("capacity_mah", "capacity", "mAh"),
    ("internal_resistance_ohm", "internal resistance", "Ω"),
    ("mic_spacing_mm", "microphone spacing", "mm"),
    ("fir_taps", "filter taps", ""),
    ("rate_hz", "update rate", "Hz"),
    ("core_v", "core voltage", "V"),
    ("display_rail_v", "display rail voltage", "V"),
    ("spreader_um", "heat-spreader thickness", "µm"),
    ("pad_area_mm2", "nose-pad area", "mm²"),
    ("value", "component value", ""),
    ("q", "quality factor Q", ""),
    ("wire_d_mm", "wire diameter", "mm"),
    ("mean_d_mm", "mean coil diameter", "mm"),
    ("pin_d_mm", "pin diameter", "mm"),
    ("free_len_mm", "free length", "mm"),
    ("inner_corner_r_mm", "inner corner radius", "mm"),
    ("outer_corner_r_mm", "outer corner radius", "mm"),
    ("adjust_step_mm", "focus adjust step", "mm"),
    ("lock_shift_mm", "focus lock shift", "mm"),
    ("gyro_noise_dps_rthz", "gyro noise density", "°/s/√Hz"),
    ("gyro_bias_dps", "gyro bias", "°/s"),
    ("accel_noise_mg_rthz", "accelerometer noise density", "mg/√Hz"),
    ("fusion_alpha", "fusion filter weight α", ""),
    ("eye_relief_mm", "eye relief", "mm"),
];

/// Plain section names (acronyms spelled out once, as the app does).
const SECTIONS: &[(&str, &str)] = &[
    ("frame", "Frame"),
    ("battery", "Battery"),
    ("power_rails", "Power rails"),
    ("optics", "Optics"),
    ("audio", "Audio"),
    ("imu", "Motion sensor (IMU)"),
    ("pcb", "Printed circuit board (PCB)"),
    ("thermal", "Thermal"),
    ("manufacturing", "Manufacturing"),
    ("print", "3-D printing"),
    ("chip", "Chip"),
];

/// Human label and unit for a parameter path, e.g.
/// `optics.display_luminance_nits` → ("Optics · display luminance", "cd/m²").
pub fn describe(path: &str) -> (String, String) {
    let segs: Vec<&str> = path.split('.').collect();
    let (section, rest): (String, Vec<&str>) = if segs.first() == Some(&"extra") && segs.len() > 2 {
        let s = segs[1].replace('_', " ");
        let s = match s.as_str() {
            "rf" => "Radio (RF)".to_string(),
            "qkd" => "Quantum key distribution (QKD)".to_string(),
            "qrng" => "Quantum random numbers (QRNG)".to_string(),
            other => {
                let mut c = other.chars();
                c.next()
                    .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            }
        };
        (s, segs[2..].to_vec())
    } else if segs.first() == Some(&"extra") {
        ("Extra".into(), segs[1..].to_vec())
    } else {
        let s = segs.first().copied().unwrap_or("");
        let name = SECTIONS
            .iter()
            .find(|x| x.0 == s)
            .map(|x| x.1.to_string())
            .unwrap_or_else(|| s.replace('_', " "));
        (name, segs[1..].to_vec())
    };
    let mut unit = String::new();
    let mut words: Vec<String> = Vec::new();
    for (i, s) in rest.iter().enumerate() {
        if let Ok(k) = s.parse::<usize>() {
            words.push(format!("#{}", k + 1));
            continue;
        }
        let mut name = s.to_string();
        if i + 1 == rest.len() {
            if let Some((_, n, u)) = NAMES.iter().find(|x| x.0 == *s) {
                unit = (*u).to_string();
                words.push((*n).to_string());
                continue;
            }
            for (suf, u) in UNITS {
                if let Some(stem) = name.strip_suffix(suf) {
                    if !stem.is_empty() {
                        unit = (*u).to_string();
                        name = stem.to_string();
                        break;
                    }
                }
            }
        }
        words.push(name.replace('_', " "));
    }
    (format!("{section} · {}", words.join(" ")), unit)
}

fn pointer(path: &str) -> String {
    format!("/design/{}", path.replace('.', "/"))
}

/// Fast parameter edits: the project is serialised once, each probe clones
/// the JSON tree, sets one leaf and deserialises it.
pub struct Perturber {
    base: Value,
}

impl Perturber {
    pub fn new(p: &Project) -> Result<Self, String> {
        Ok(Self {
            base: serde_json::to_value(p).map_err(|e| e.to_string())?,
        })
    }

    pub fn design(&self) -> &Value {
        &self.base["design"]
    }

    pub fn get(&self, path: &str) -> Option<f64> {
        self.base.pointer(&pointer(path))?.as_f64()
    }

    pub fn integer(&self, path: &str) -> bool {
        self.base.pointer(&pointer(path)).is_some_and(|v| v.is_u64() || v.is_i64())
    }

    /// Copy of the project with one parameter set (integers rounded).
    pub fn at(&self, path: &str, x: f64) -> Result<Project, String> {
        let mut v = self.base.clone();
        let slot = v
            .pointer_mut(&pointer(path))
            .ok_or_else(|| format!("unknown design parameter `{path}`"))?;
        if !slot.is_number() {
            return Err(format!("design parameter `{path}` is not a number"));
        }
        *slot = if slot.is_u64() || slot.is_i64() {
            Value::from(x.round() as i64)
        } else {
            serde_json::Number::from_f64(x).map(Value::Number).ok_or("value must be finite")?
        };
        serde_json::from_value(v).map_err(|e| format!("cannot apply {path} = {x}: {e}"))
    }
}

// ---------------------------------------------------------------------------
// Measuring levers
// ---------------------------------------------------------------------------

/// One measured probe: the module's metrics after nudging one parameter.
#[derive(Clone, Debug)]
pub struct Probe {
    pub path: String,
    pub value: f64,
    pub integer: bool,
    /// Size of one step (always positive).
    pub step: f64,
    /// Metric change per +step, keyed by metric (only metrics that moved).
    pub deltas: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Default)]
pub struct ModuleProbes {
    pub baseline: BTreeMap<String, f64>,
    pub probes: Vec<Probe>,
}

thread_local! {
    /// module → (project hash, probes): repeated calls on an unchanged
    /// project (re-renders, the lazy lever fetch) cost nothing; any edit
    /// re-measures.
    static CACHE: RefCell<HashMap<String, (u64, ModuleProbes)>> = RefCell::new(HashMap::new());
}

fn project_hash(base: &Value) -> u64 {
    let mut h = DefaultHasher::new();
    base.to_string().hash(&mut h);
    h.finish()
}

/// Step size for a probe, and whether to probe downwards because the
/// upward step would leave the parameter's physical bounds.
pub fn step_for(path: &str, value: f64, integer: bool) -> (f64, bool) {
    let step = if integer {
        (value * STEP_REL).round().max(1.0)
    } else {
        value.abs() * STEP_REL
    };
    let (_, hi) = bounds(path, value, integer);
    (step, value + step > hi)
}

fn moved(a: f64, b: f64) -> bool {
    (a - b).abs() > 1e-9 * a.abs().max(b.abs()).max(1e-12)
}

/// Measure every candidate lever of one module (memoised per project).
/// After probing, the module is re-run on the real project so its own
/// memo cache holds the baseline again.
pub fn measure_module(p: &Project, pert: &Perturber, module: &str) -> ModuleProbes {
    let h = project_hash(&pert.base);
    if let Some(hit) = CACHE.with(|c| c.borrow().get(module).filter(|(k, _)| *k == h).map(|(_, v)| v.clone())) {
        return hit;
    }
    let base_metrics = owner_metrics(module, p);
    let baseline: BTreeMap<String, f64> = base_metrics.iter().map(|m| (m.key.clone(), m.value)).collect();
    let mut probes = Vec::new();
    let cands = candidates(pert.design(), module);
    for c in &cands {
        let (step, down) = step_for(&c.path, c.value, c.integer);
        let x = if down { c.value - step } else { c.value + step };
        let Ok(q) = pert.at(&c.path, x) else { continue };
        let out = owner_metrics(module, &q);
        let mut deltas = BTreeMap::new();
        for m in &out {
            if let Some(&b) = baseline.get(&m.key) {
                if m.value.is_finite() && moved(m.value, b) {
                    let d = if down { b - m.value } else { m.value - b };
                    deltas.insert(m.key.clone(), d);
                }
            }
        }
        probes.push(Probe {
            path: c.path.clone(),
            value: c.value,
            integer: c.integer,
            step,
            deltas,
        });
    }
    if !cands.is_empty() {
        // re-warm the module's memo cache with the real project
        let _ = owner_metrics(module, p);
    }
    let r = ModuleProbes { baseline, probes };
    CACHE.with(|c| c.borrow_mut().insert(module.to_string(), (h, r.clone())));
    r
}

/// A measured design lever for one metric.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lever {
    /// Dotted path under `design`.
    pub param: String,
    pub label: String,
    /// Unit of the parameter.
    pub unit: String,
    pub value: f64,
    /// Step used for the measurement (in the parameter's unit).
    pub step: f64,
    pub integer: bool,
    /// Change of the metric when the parameter rises by one step (metric unit).
    pub delta: f64,
    /// `delta` as a percentage of the metric's current value.
    pub effect_pct: f64,
    /// Which way to move the parameter to improve the margin: `increase` | `decrease`.
    pub direction: String,
    /// Margin gained per step in that direction (fraction of the target).
    pub margin_per_step: f64,
    /// Linear estimate of the parameter value that reaches the target (+2 % margin), when the
    /// margin is short of that and the estimate lies within ÷4 … ×4 of today's value.
    pub estimate: Option<f64>,
    /// Physical bounds used by the solver.
    pub lo: f64,
    pub hi: f64,
}

/// Turn a module's probes into ranked levers for one metric and criterion.
/// Without a criterion, "improve" means "increase the metric".
pub fn levers_from(probes: &ModuleProbes, metric: &str, ver: Option<&Verification>, limit: usize) -> Vec<Lever> {
    let Some(&m0) = probes.baseline.get(metric) else {
        return vec![];
    };
    let margin = |x: f64| -> f64 {
        match ver {
            Some(v) => check(v, x).1,
            None => x,
        }
    };
    let base_margin = margin(m0);
    let mut out: Vec<Lever> = probes
        .probes
        .iter()
        .filter_map(|pr| {
            let d = *pr.deltas.get(metric)?;
            let dm = margin(m0 + d) - base_margin;
            // a criterion-relative change too small to matter (or a move sideways inside a band)
            // below 0.01 % of the target per 5 % step: not a useful lever
            if dm.abs() < 1e-4 {
                return None;
            }
            let (lo, hi) = bounds(&pr.path, pr.value, pr.integer);
            let (label, unit) = describe(&pr.path);
            let up = dm > 0.0;
            let estimate = match ver {
                Some(v) if base_margin < SOLVE_MARGIN => {
                    let want = if v.target.abs() > 1e-12 { SOLVE_MARGIN } else { 0.0 };
                    let steps = (want - base_margin) / dm.abs();
                    let x = pr.value + if up { 1.0 } else { -1.0 } * steps * pr.step;
                    // only inside the solver's default range (÷4 … ×4) and the physical bounds
                    (x >= lo.max(pr.value / 4.0) && x <= hi.min(pr.value * 4.0)).then_some(x)
                }
                _ => None,
            };
            Some(Lever {
                param: pr.path.clone(),
                label,
                unit,
                value: pr.value,
                step: pr.step,
                integer: pr.integer,
                delta: d,
                effect_pct: if m0.abs() > 1e-12 { d / m0.abs() * 100.0 } else { 0.0 },
                direction: if up { "increase" } else { "decrease" }.into(),
                margin_per_step: dm.abs(),
                estimate,
                lo,
                hi,
            })
        })
        .collect();
    out.sort_by(|a, b| {
        b.margin_per_step
            .partial_cmp(&a.margin_per_step)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(limit);
    out
}

/// Input of `systems.next_levers`.
#[derive(Deserialize)]
pub struct LeversIn {
    pub project: Project,
    pub metric: String,
    /// Requirement whose criterion defines "better" (default: the first one bound to the metric).
    #[serde(default)]
    pub requirement_id: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LeversOut {
    pub metric: String,
    pub module: Option<String>,
    pub requirement_id: Option<String>,
    pub candidates: usize,
    pub levers: Vec<Lever>,
}

/// Measure levers for one metric on request (used for the expensive modules).
pub fn levers_for_metric(i: &LeversIn) -> Result<LeversOut, String> {
    let p = &i.project;
    let req = match &i.requirement_id {
        Some(id) => Some(
            p.requirements
                .iter()
                .find(|r| &r.id == id)
                .ok_or_else(|| format!("unknown requirement `{id}`"))?,
        ),
        None => p
            .requirements
            .iter()
            .find(|r| r.verification.metric.as_deref() == Some(i.metric.as_str())),
    };
    let Some(module) = owner(&i.metric) else {
        return Ok(LeversOut {
            metric: i.metric.clone(),
            module: None,
            requirement_id: req.map(|r| r.id.clone()),
            candidates: 0,
            levers: vec![],
        });
    };
    let pert = Perturber::new(p)?;
    let probes = measure_module(p, &pert, module);
    Ok(LeversOut {
        metric: i.metric.clone(),
        module: Some(module.into()),
        requirement_id: req.map(|r| r.id.clone()),
        candidates: probes.probes.len(),
        levers: levers_from(&probes, &i.metric, req.map(|r| &r.verification), i.limit.unwrap_or(6)),
    })
}

// ---------------------------------------------------------------------------
// Solve for target
// ---------------------------------------------------------------------------

fn yes() -> bool {
    true
}

/// Input of `systems.solve_lever`.
#[derive(Deserialize)]
pub struct SolveIn {
    pub project: Project,
    pub requirement_id: String,
    pub param: String,
    #[serde(default)]
    pub lo: Option<f64>,
    #[serde(default)]
    pub hi: Option<f64>,
    /// Apply first-order couplings the labs do not model (see [`COUPLINGS`]). Default true.
    #[serde(default = "yes")]
    pub couple: bool,
    /// Margin to aim for, as a fraction of the target (default 0.02).
    #[serde(default)]
    pub margin: Option<f64>,
    /// The caller's current evaluation of `project` (saves one full run).
    #[serde(default)]
    pub evaluation: Option<Evaluation>,
}

/// One edit to the project (what "Apply" writes).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Change {
    /// `design` (path under `design`) | `component` (refdes + field).
    pub kind: String,
    pub path: String,
    pub refdes: Option<String>,
    pub label: String,
    pub unit: String,
    pub before: f64,
    pub after: f64,
    /// Set when the change is a stated modelling assumption rather than the solved parameter.
    pub assumption: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SideEffect {
    pub id: String,
    pub text: String,
    pub metric: Option<String>,
    pub unit: String,
    pub criterion: String,
    pub before_status: Status,
    pub after_status: Status,
    pub before_value: Option<f64>,
    pub after_value: Option<f64>,
    pub before_margin: Option<f64>,
    pub after_margin: Option<f64>,
    pub verdict_changed: bool,
    /// Margin went up (or the verdict improved).
    pub better: bool,
    pub goal: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Solve {
    pub requirement_id: String,
    pub metric: String,
    pub module: String,
    pub param: String,
    pub label: String,
    pub unit: String,
    pub metric_unit: String,
    pub criterion: String,
    /// A value meeting the target was found in range.
    pub found: bool,
    pub message: String,
    pub lo: f64,
    pub hi: f64,
    pub before_param: f64,
    pub after_param: f64,
    pub before_value: Option<f64>,
    pub after_value: Option<f64>,
    pub before_margin: Option<f64>,
    pub after_margin: Option<f64>,
    /// Margin the solver aimed for (it falls back to 0 when only the bare target is reachable).
    pub aimed_margin: f64,
    /// Module runs used by the search.
    pub runs: usize,
    pub changes: Vec<Change>,
    /// Every requirement whose verdict, value or margin moved (the solved one included).
    pub side_effects: Vec<SideEffect>,
    pub pass_before: usize,
    pub pass_after: usize,
    pub fail_before: usize,
    pub fail_after: usize,
    pub open_goals_before: usize,
    pub open_goals_after: usize,
}

/// A first-order physical coupling the lab models do not capture, applied
/// (and labelled as an assumption) when a solved parameter changes.
pub struct Coupling {
    pub param: &'static str,
    pub assumption: &'static str,
}

/// Couplings applied by [`solve`]. Both scale linearly with the solved
/// parameter's ratio — a stated first-order assumption, shown to the user.
pub const COUPLINGS: &[Coupling] = &[
    Coupling {
        param: "optics.display_luminance_nits",
        assumption: "An emissive micro-display's power rises roughly in proportion to its luminance at a fixed luminous \
                     efficacy; the display line's active power is scaled by the same ratio. Real modules have a fixed \
                     driver overhead, so this is an upper bound — check the panel datasheet's power-versus-luminance curve.",
    },
    Coupling {
        param: "battery.capacity_mah",
        assumption: "Same cell chemistry and specific energy, so the cell's mass scales with its capacity (first order; \
                     small cells carry proportionally more packaging). Check a real cell of that capacity.",
    },
];

fn is_display(c: &crate::project::Component) -> bool {
    let t = format!("{} {}", c.name, c.part_class).to_lowercase();
    ["display", "oled", "microdisplay", "lcos", "lcd"].iter().any(|w| t.contains(w)) && c.power.active_mw > 0.0
}

fn is_battery(c: &crate::project::Component) -> bool {
    let t = format!("{} {}", c.name, c.part_class).to_lowercase();
    c.refdes.starts_with("BT") || t.contains("battery") || t.contains("lipo") || t.contains("li-ion")
}

/// Apply the couplings of `param` for a value ratio `r`; returns the edits.
fn apply_couplings(q: &mut Project, param: &str, r: f64) -> Vec<Change> {
    let mut out = Vec::new();
    let Some(cp) = COUPLINGS.iter().find(|c| c.param == param) else {
        return out;
    };
    if !(r.is_finite() && r > 0.0) || (r - 1.0).abs() < 1e-12 {
        return out;
    }
    let note = Some(cp.assumption.to_string());
    if param == "optics.display_luminance_nits" {
        for c in q.components.iter_mut().filter(|c| is_display(c)) {
            let before = c.power.active_mw;
            c.power.active_mw = before * r;
            out.push(Change {
                kind: "component".into(),
                path: "power.active_mw".into(),
                refdes: Some(c.refdes.clone()),
                label: format!("{} {} active power", c.refdes, c.name),
                unit: "mW".into(),
                before,
                after: c.power.active_mw,
                assumption: note.clone(),
            });
        }
    } else if param == "battery.capacity_mah" {
        let before = q.design.battery.mass_g;
        if before > 0.0 {
            q.design.battery.mass_g = before * r;
            out.push(Change {
                kind: "design".into(),
                path: "battery.mass_g".into(),
                refdes: None,
                label: "Battery · mass".into(),
                unit: "g".into(),
                before,
                after: q.design.battery.mass_g,
                assumption: note.clone(),
            });
        }
        for c in q.components.iter_mut().filter(|c| is_battery(c) && c.mass_g > 0.0) {
            let before = c.mass_g;
            c.mass_g = before * r;
            out.push(Change {
                kind: "component".into(),
                path: "mass_g".into(),
                refdes: Some(c.refdes.clone()),
                label: format!("{} {} mass", c.refdes, c.name),
                unit: "g".into(),
                before,
                after: c.mass_g,
                assumption: note.clone(),
            });
        }
    }
    out
}

/// Round to three significant figures towards `away_from_fail` (up when the
/// passing side is above the failing one), so the suggested value is a
/// number an engineer would write down.
fn round_sig(x: f64, up: bool) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let mag = 10f64.powf(x.abs().log10().floor() - 2.0);
    if up {
        (x / mag).ceil() * mag
    } else {
        (x / mag).floor() * mag
    }
}

/// Find the value of one design parameter that brings a requirement's metric
/// to its target (plus [`SOLVE_MARGIN`]): a grid scan of the owning module
/// over the bounds, then bisection on the bracket nearest today's value,
/// then one full evaluation of the changed project to list side effects.
pub fn solve(i: &SolveIn) -> Result<Solve, String> {
    let p = &i.project;
    let req = p
        .requirements
        .iter()
        .find(|r| r.id == i.requirement_id)
        .ok_or_else(|| format!("unknown requirement `{}`", i.requirement_id))?;
    let ver = &req.verification;
    let metric = ver.metric.clone().ok_or_else(|| format!("{} has no metric to solve for", req.id))?;
    let module = owner(&metric).ok_or_else(|| format!("no lab module computes `{metric}`"))?;
    let pert = Perturber::new(p)?;
    let x0 = pert
        .get(&i.param)
        .ok_or_else(|| format!("unknown or non-numeric design parameter `{}`", i.param))?;
    let integer = pert.integer(&i.param);
    let (blo, bhi) = bounds(&i.param, x0, integer);
    let default_lo = if x0 > 0.0 { x0 / 4.0 } else { x0 - 1.0 };
    let default_hi = if x0 > 0.0 { x0 * 4.0 } else { x0 + 1.0 };
    let mut lo = i.lo.unwrap_or(default_lo).max(blo);
    let mut hi = i.hi.unwrap_or(default_hi).min(bhi);
    if integer {
        lo = lo.ceil();
        hi = hi.floor();
    }
    if !(lo.is_finite() && hi.is_finite()) || hi < lo {
        return Err(format!("empty search range [{lo}, {hi}]"));
    }
    let want = i.margin.unwrap_or(if ver.target.abs() > 1e-12 { SOLVE_MARGIN } else { 0.0 });
    let mut runs = 0usize;
    let mut eval_at = |x: f64| -> Option<(f64, f64)> {
        runs += 1;
        let q = pert.at(&i.param, x).ok()?;
        let v = owner_metrics(module, &q).into_iter().find(|m| m.key == metric)?.value;
        v.is_finite().then(|| (v, check(ver, v).1))
    };

    // baseline
    let base = eval_at(x0);
    // grid (geometric when the range is positive), always including x0
    let n = 13usize;
    let mut grid: Vec<f64> = (0..n)
        .map(|k| {
            let t = k as f64 / (n - 1) as f64;
            if lo > 0.0 {
                lo * (hi / lo).powf(t)
            } else {
                lo + (hi - lo) * t
            }
        })
        .map(|x| if integer { x.round() } else { x })
        .collect();
    if x0 >= lo && x0 <= hi {
        grid.push(x0);
    }
    grid.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    grid.dedup_by(|a, b| (*a - *b).abs() <= 1e-12 * a.abs().max(1.0));
    let samples: Vec<(f64, Option<(f64, f64)>)> = grid.iter().map(|&x| (x, if x == x0 { base } else { eval_at(x) })).collect();
    let dist = |x: f64| if x > 0.0 && x0 > 0.0 { (x / x0).ln().abs() } else { (x - x0).abs() };
    let pick = |m: f64| {
        samples
            .iter()
            .enumerate()
            .filter(|(_, (_, r))| r.is_some_and(|r| r.1 >= m))
            .min_by(|a, b| dist(a.1 .0).partial_cmp(&dist(b.1 .0)).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, _)| k)
    };
    let mut aimed = want;
    let mut hit = pick(want);
    if hit.is_none() && want > SOLVE_MARGIN {
        // A marginal pass aims for the marginal band (10 %); when the range cannot give
        // that, try the 2 % margin the lever estimate promises before anything else.
        aimed = SOLVE_MARGIN;
        hit = pick(SOLVE_MARGIN);
    }
    if hit.is_none() && want > 0.0 && !base.is_some_and(|b| b.1 >= 0.0) {
        // A failing requirement: just meeting the target is still worth showing.
        aimed = 0.0;
        hit = pick(0.0);
    }
    let unit_of = describe(&i.param);
    let criterion = super::super::criterion_text(ver);
    let metric_unit = if ver.unit.is_empty() { String::new() } else { ver.unit.clone() };

    let (found, xs, message) = match hit {
        None => {
            let best = samples
                .iter()
                .filter_map(|(x, r)| r.map(|r| (*x, r)))
                .max_by(|a, b| a.1 .1.partial_cmp(&b.1 .1).unwrap_or(std::cmp::Ordering::Equal));
            let msg = match best {
                Some((bx, (bv, _))) => format!(
                    "No value of {} between {} and {} {} {} {} {}: the best in range is {} {} at {} {}.",
                    unit_of.0,
                    fmt_num(lo),
                    fmt_num(hi),
                    unit_of.1,
                    // a pass that wants more margin already meets the criterion
                    if base.is_some_and(|b| b.1 >= 0.0) {
                        "adds margin on"
                    } else {
                        "meets"
                    },
                    metric,
                    criterion,
                    fmt_num(bv),
                    metric_unit,
                    fmt_num(bx),
                    unit_of.1
                ),
                None => format!("The {module} module produced no value for {metric} in the range searched."),
            };
            (false, x0, msg)
        }
        Some(k) => {
            let (xp, _) = samples[k];
            if base.is_some_and(|b| b.1 >= aimed) {
                (true, x0, format!("{} already meets {criterion} at today's value.", req.id))
            } else {
                // neighbour on the side of x0 (the last failing sample before the passing one)
                let toward = if xp > x0 {
                    k.checked_sub(1)
                } else {
                    (k + 1 < samples.len()).then_some(k + 1)
                };
                let mut pass = xp;
                if let Some(j) = toward {
                    let mut fail = samples[j].0;
                    for _ in 0..40 {
                        if integer && (pass - fail).abs() <= 1.0 {
                            break;
                        }
                        if !integer && (pass - fail).abs() <= 1e-6 * pass.abs().max(1e-9) {
                            break;
                        }
                        let mut mid = if pass > 0.0 && fail > 0.0 {
                            (pass * fail).sqrt()
                        } else {
                            0.5 * (pass + fail)
                        };
                        if integer {
                            mid = mid.round();
                            if mid == pass || mid == fail {
                                break;
                            }
                        }
                        match eval_at(mid) {
                            Some((_, m)) if m >= aimed => pass = mid,
                            _ => fail = mid,
                        }
                    }
                    if !integer {
                        let r = round_sig(pass, pass > fail).clamp(lo, hi);
                        if eval_at(r).is_some_and(|x| x.1 >= aimed) {
                            pass = r;
                        }
                    }
                }
                let msg = if aimed < want && aimed > 0.0 {
                    format!(
                        "{} = {} {} meets {criterion} with about {:.0} % margin; the {:.0} % margin the solver aims for is out of range.",
                        unit_of.0,
                        fmt_num(pass),
                        unit_of.1,
                        aimed * 100.0,
                        want * 100.0
                    )
                } else if aimed < want {
                    format!(
                        "{} = {} {} just meets {criterion}; the {:.0} % margin the solver aims for is out of range.",
                        unit_of.0,
                        fmt_num(pass),
                        unit_of.1,
                        want * 100.0
                    )
                } else {
                    format!(
                        "{} = {} {} meets {criterion} with about {:.0} % margin.",
                        unit_of.0,
                        fmt_num(pass),
                        unit_of.1,
                        want * 100.0
                    )
                };
                (true, pass, msg)
            }
        }
    };

    // full evaluation of the changed project
    let before = i.evaluation.clone().unwrap_or_else(|| evaluate(p));
    let mut changes = vec![Change {
        kind: "design".into(),
        path: i.param.clone(),
        refdes: None,
        label: unit_of.0.clone(),
        unit: unit_of.1.clone(),
        before: x0,
        after: xs,
        assumption: None,
    }];
    let mut after_eval = None;
    if found && xs != x0 {
        let mut q = pert.at(&i.param, xs)?;
        if i.couple && x0 != 0.0 {
            changes.extend(apply_couplings(&mut q, &i.param, xs / x0));
        }
        after_eval = Some(evaluate(&q));
    }
    let after = after_eval.as_ref().unwrap_or(&before);
    let mut side_effects = Vec::new();
    for (r, b) in p.requirements.iter().zip(&before.results) {
        let Some(a) = after.results.iter().find(|x| x.id == b.id) else {
            continue;
        };
        let vm = match (a.value, b.value) {
            (Some(x), Some(y)) => moved(x, y),
            (None, None) => false,
            _ => true,
        };
        let mm = match (a.margin, b.margin) {
            (Some(x), Some(y)) => (x - y).abs() > 1e-6,
            _ => false,
        };
        if a.status != b.status || vm || mm {
            let rank = |s: Status| match s {
                Status::Pass => 2,
                Status::Unverified => 1,
                Status::Fail => 0,
            };
            let better = rank(a.status) > rank(b.status) || (a.status == b.status && a.margin.unwrap_or(0.0) > b.margin.unwrap_or(0.0));
            side_effects.push(SideEffect {
                id: r.id.clone(),
                text: r.text.clone(),
                metric: r.verification.metric.clone(),
                unit: a.unit.clone(),
                criterion: a.criterion.clone(),
                before_status: b.status,
                after_status: a.status,
                before_value: b.value,
                after_value: a.value,
                before_margin: b.margin,
                after_margin: a.margin,
                verdict_changed: a.status != b.status,
                better,
                goal: r.kind == ReqKind::Goal,
            });
        }
    }
    // verdict changes first, then the largest margin moves
    side_effects.sort_by(|a, b| {
        b.verdict_changed.cmp(&a.verdict_changed).then_with(|| {
            let d = |s: &SideEffect| (s.after_margin.unwrap_or(0.0) - s.before_margin.unwrap_or(0.0)).abs();
            d(b).partial_cmp(&d(a)).unwrap_or(std::cmp::Ordering::Equal)
        })
    });
    let rb = before.results.iter().find(|x| x.id == req.id);
    let ra = after.results.iter().find(|x| x.id == req.id);
    Ok(Solve {
        requirement_id: req.id.clone(),
        metric,
        module: module.into(),
        param: i.param.clone(),
        label: unit_of.0,
        unit: unit_of.1,
        metric_unit,
        criterion,
        found,
        message,
        lo,
        hi,
        before_param: x0,
        after_param: xs,
        before_value: rb.and_then(|r| r.value),
        after_value: ra.and_then(|r| r.value),
        before_margin: rb.and_then(|r| r.margin),
        after_margin: ra.and_then(|r| r.margin),
        aimed_margin: aimed,
        runs,
        changes,
        side_effects,
        pass_before: before.pass,
        pass_after: after.pass,
        fail_before: before.fail,
        fail_after: after.fail,
        open_goals_before: before.open_goals,
        open_goals_after: after.open_goals,
    })
}

/// Compact number formatting for messages.
pub fn fmt_num(x: f64) -> String {
    if !x.is_finite() {
        return "—".into();
    }
    let a = x.abs();
    let s = if a >= 1000.0 {
        format!("{x:.0}")
    } else if a >= 100.0 {
        format!("{x:.1}")
    } else if a >= 1.0 {
        format!("{x:.2}")
    } else if a >= 0.01 {
        format!("{x:.3}")
    } else {
        format!("{x:.3e}")
    };
    if s.contains('.') && !s.contains('e') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// `Cmp` in words, for titles.
pub fn cmp_verb(c: Cmp) -> &'static str {
    match c {
        Cmp::Le => "Bring down",
        Cmp::Ge => "Raise",
        Cmp::Eq | Cmp::Between => "Bring into range",
    }
}
