//! # PCB Studio engine
//!
//! Takes the Iris main board from a component list to factory files:
//!
//! 1. [`footprints`] — land patterns for every package used.
//! 2. [`netlist`] — the schematic connectivity (pin → net).
//! 3. [`place`] — constraint-driven placement, GPIO pin swapping and a
//!    simulated-annealing optimiser (HPWL).
//! 4. [`route`] — ground fan-out plus a two-layer Lee/A* maze router with
//!    rip-up and re-route.
//! 5. [`pour`] — ground pours on both layers with island removal and via stitching.
//! 6. [`drc`] — connectivity extraction and design-rule check.
//! 7. [`fab`] — Gerber RS-274X (X2), Excellon drill, BOM/CPL for assembly.
//!
//! The public [`placement`] function (used by the robotics pick-and-place
//! planner) returns the constructive placement of every on-board component.
//!
//! References: IPC-2221 *Generic Standard on Printed Board Design*; IPC-7351
//! *Generic Requirements for Surface Mount Design and Land Pattern Standard*;
//! C. Y. Lee (1961); S. Kirkpatrick, C. D. Gelatt, M. P. Vecchi, "Optimization
//! by simulated annealing", *Science* 220 (1983); Ucamco, *The Gerber Layer
//! Format Specification*.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Metric, Project};

pub mod board;
pub mod drc;
pub mod fab;
pub mod footprints;
pub mod geom;
pub mod netlist;
pub mod place;
pub mod pour;
pub mod route;

use board::{BoardPad, Hole, Keepout, PartPos, PlacedPart, Rules};
use drc::{Connectivity, Violation};
use netlist::NetClass;
use route::{NetRoute, Track, Via};

/// A placed component (board coordinates in mm, origin bottom-left).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Placed {
    pub refdes: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub rot_deg: f64,
    /// "top" or "bottom"
    pub side: String,
}

/// Component placement for every `on_pcb` component (deterministic,
/// constraint-driven; see [`place::constructive`]).
pub fn placement(p: &Project) -> Vec<Placed> {
    let r = place::constructive(p);
    r.parts
        .iter()
        .filter(|pp| !netlist::BOARD_FEATURES.contains(&pp.refdes.as_str()))
        .map(|pp| Placed { refdes: pp.refdes.clone(), x_mm: pp.x, y_mm: pp.y, rot_deg: pp.rot, side: pp.side.clone() })
        .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoardInfo {
    pub width: f64,
    pub height: f64,
    pub thickness: f64,
    pub layers: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetView {
    pub name: String,
    pub class: NetClass,
    pub pins: Vec<String>,
    pub description: String,
    pub current_a: f64,
    pub routed: bool,
    pub length_mm: f64,
    pub vias: usize,
    pub unconnected: Vec<String>,
    pub ripups: u32,
    /// IPC-2221 minimum width for `current_a` at 10 °C rise (external, 1 oz).
    pub ipc_width_mm: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Stats {
    pub routed_pct: f64,
    pub connections: usize,
    pub routed: usize,
    pub drc_violations: usize,
    pub via_count: usize,
    pub track_length_mm: f64,
    pub track_length_top_mm: f64,
    pub track_length_bottom_mm: f64,
    pub hpwl_mm: f64,
    pub pour_area_mm2: f64,
    pub pour_islands_removed: usize,
    pub ripups: u32,
    pub expansions: u64,
    pub parts: usize,
    pub pads: usize,
    pub nets: usize,
    pub smd_joints: usize,
    pub tht_joints: usize,
    pub unique_parts: usize,
    /// Routing strategy that produced this layout, and how many were tried.
    pub strategy: String,
    pub strategy_index: usize,
    pub attempts: usize,
    /// True when the routed copper came from the baked result for the default inputs.
    pub from_cache: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layout {
    pub board: BoardInfo,
    pub rules: Rules,
    pub parts: Vec<PlacedPart>,
    pub pads: Vec<BoardPad>,
    pub nets: Vec<NetView>,
    pub tracks: Vec<Track>,
    pub vias: Vec<Via>,
    pub holes: Vec<Hole>,
    pub keepouts: Vec<Keepout>,
    /// Bottom-layer ground pour.
    pub pour: pour::Pour,
    /// Top-layer ground pour.
    pub pour_top: pour::Pour,
    pub drc: Vec<Violation>,
    pub connectivity: Connectivity,
    pub stats: Stats,
    /// GPIO pin swapping result: (signal, SoC pin).
    pub gpio: Vec<(String, String)>,
    /// Engineering notes (BOM gaps, assumptions).
    pub notes: Vec<String>,
}

/// IPC-2221 conductor sizing: I = k·ΔT^0.44·A^0.725 (A in mil², I in A,
/// k = 0.048 external / 0.024 internal). Returns the width in mm for a copper
/// weight in oz/ft² (1 oz ≈ 35 µm ≈ 1.378 mil).
pub fn ipc2221_width_mm(current_a: f64, temp_rise_c: f64, copper_oz: f64, external: bool) -> f64 {
    let k = if external { 0.048 } else { 0.024 };
    let area_mil2 = (current_a / (k * temp_rise_c.max(0.1).powf(0.44))).powf(1.0 / 0.725);
    let thick_mil = 1.378 * copper_oz;
    area_mil2 / thick_mil * 0.0254
}

/// Inverse of [`ipc2221_width_mm`]: allowed current for a width.
pub fn ipc2221_current_a(width_mm: f64, temp_rise_c: f64, copper_oz: f64, external: bool) -> f64 {
    let k = if external { 0.048 } else { 0.024 };
    let area = width_mm / 0.0254 * 1.378 * copper_oz;
    k * temp_rise_c.powf(0.44) * area.powf(0.725)
}

/// One deterministic routing strategy; several are tried in turn and the
/// first clean result (or the best one) is kept.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Strategy {
    pub name: &'static str,
    pub power_first: bool,
    pub bot_cost: (u32, u32),
    pub iterations: usize,
}

pub const STRATEGIES: &[Strategy] = &[
    Strategy { name: "power first, bottom = short vertical hops", power_first: true, bot_cost: (40, 16), iterations: 24 },
    Strategy { name: "signals first, bottom = short vertical hops", power_first: false, bot_cost: (40, 16), iterations: 24 },
    Strategy { name: "signals first, cheaper bottom layer", power_first: false, bot_cost: (24, 14), iterations: 24 },
    Strategy { name: "power first, cheaper bottom layer", power_first: true, bot_cost: (24, 14), iterations: 24 },
];

fn net_order(b: &board::Board, power_first: bool) -> Vec<u16> {
    let hp = |ni: usize| -> f64 {
        let ps: Vec<&BoardPad> = b.pads.iter().filter(|p| p.net_id as usize == ni).collect();
        let (x0, x1) = ps.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| (a.0.min(p.x), a.1.max(p.x)));
        let (y0, y1) = ps.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| (a.0.min(p.y), a.1.max(p.y)));
        (x1 - x0) + (y1 - y0)
    };
    let mut v: Vec<(u32, f64, u16)> = Vec::new();
    for (i, n) in b.nets.iter().enumerate() {
        if n.class == NetClass::Ground {
            continue;
        }
        let pins = b.pads.iter().filter(|p| p.net_id as usize == i).count();
        if pins < 2 {
            continue;
        }
        let pf = power_first;
        let pri = match n.class {
            NetClass::Rf => 0,
            NetClass::Power if n.name == "SW" => 1,
            NetClass::Signal => 2,
            NetClass::Power => if pf { 1 } else { 3 },
            NetClass::Ground => 9,
        };
        v.push((pri, hp(i), i as u16));
    }
    v.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.partial_cmp(&b.1).unwrap()).then(a.2.cmp(&b.2)));
    v.into_iter().map(|x| x.2).collect()
}

fn notes_for(p: &Project) -> Vec<String> {
    let has = |r: &str| p.components.iter().any(|c| c.refdes == r && c.on_pcb);
    let mut v = vec![
        "Land patterns are typical approximations — verify every footprint against the part datasheet / IPC-7351 before ordering.".to_string(),
        "The SoC pinout is illustrative: power, RF and debug pins are fixed and GPIO signals were pin-swapped to the nearest free pads; re-map to your SoC's real pin-mux table.".to_string(),
    ];
    if has("J1") && !(has("R6") && has("R7")) {
        v.push("BOM gap: a USB-C power sink needs 5.1 kΩ pull-down resistors (Rd) on CC1 and CC2, otherwise C-to-C chargers will not supply VBUS. Add R6/R7 (0402) to the project — the lab connects them automatically.".into());
    }
    if has("U1") {
        v.push("BOM gap: a BLE SoC normally needs a 32 MHz crystal (+ load capacitors) and an RF matching network between the ANT pin and the chip antenna; they are not in the Iris BOM yet.".into());
    }
    v.push("CHG_STAT is open-drain: enable the SoC's internal pull-up on that GPIO.".into());
    v
}

/// Full layout for the project (placement → fan-out → routing → pour → DRC).
static CACHE: Mutex<Option<(u64, Arc<Layout>)>> = Mutex::new(None);

/// Store a layout computed elsewhere (e.g. by the engine running in a Web
/// Worker) as the cached result for this project, so metrics don't re-route.
pub fn adopt(p: &Project, l: Layout) {
    if let Ok(mut g) = CACHE.lock() {
        *g = Some((cache_key(p), Arc::new(l)));
    }
}

pub fn layout(p: &Project) -> Arc<Layout> {
    let key = cache_key(p);
    if let Ok(g) = CACHE.lock() {
        if let Some((k, l)) = g.as_ref() {
            if *k == key {
                return l.clone();
            }
        }
    }
    let l = Arc::new(layout_with(p, None));
    if let Ok(mut g) = CACHE.lock() {
        *g = Some((key, l.clone()));
    }
    l
}

fn cache_key(p: &Project) -> u64 {
    // component names feed the BOM/labels, so they are part of the in-memory key
    let mut s = cache_key_hex(p);
    for c in p.components.iter().filter(|c| c.on_pcb) {
        s += &c.name;
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Layout with an optional placement override (e.g. the annealer's result).
pub fn layout_with(p: &Project, placement_override: Option<Vec<PartPos>>) -> Layout {
    layout_impl(p, placement_override, true)
}

/// The routing result for the default Iris inputs, produced by this router
/// (see the `baked_route_matches_router` test and `regenerate_baked_route`).
const BAKED_ROUTE: &str = include_str!("iris_route.json");

fn baked_route() -> Option<Routed> {
    serde_json::from_str(BAKED_ROUTE).ok()
}

/// Bump when routing behaviour changes so stale baked routes are ignored.
pub const ROUTER_VERSION: &str = "nc-pinswap-1";

/// FNV-1a hash of every input that influences the routed copper.
pub fn cache_key_hex(p: &Project) -> String {
    let mut s = String::from(ROUTER_VERSION);
    for c in p.components.iter().filter(|c| c.on_pcb) {
        s += &format!("|{}:{}", c.refdes, c.footprint);
    }
    s += &serde_json::to_string(&p.design.pcb).unwrap_or_default();
    s += &format!("|mic{}", p.design.audio.mic_spacing_mm);
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

fn layout_impl(p: &Project, placement_override: Option<Vec<PartPos>>, use_baked: bool) -> Layout {
    let placement_override_is_none = placement_override.is_none();
    let pr = place::constructive(p);
    let (parts, gpio) = match placement_override {
        Some(mut ov) => {
            // keep features / parts missing from the override at their constructive spot
            for pp in &pr.parts {
                if !ov.iter().any(|o| o.refdes == pp.refdes) {
                    ov.push(pp.clone());
                }
            }
            let gpio = place::assign_gpio(p, &ov);
            (ov, gpio)
        }
        None => (pr.parts.clone(), pr.gpio.clone()),
    };
    if placement_override_is_none && use_baked {
        if let Some(r) = baked_route() {
            if r.key == cache_key_hex(p) {
                let mut l = finalize(p, &parts, &r);
                l.stats.strategy = STRATEGIES[r.strategy_index.min(STRATEGIES.len() - 1)].name.to_string();
                l.stats.strategy_index = r.strategy_index;
                l.stats.attempts = r.strategy_index + 1;
                l.stats.from_cache = true;
                return l;
            }
        }
    }
    let mut best: Option<Layout> = None;
    for (k, s) in STRATEGIES.iter().enumerate() {
        let routed = route_phase(p, &parts, &gpio, s);
        let mut l = finalize(p, &parts, &routed);
        l.stats.strategy = s.name.to_string();
        l.stats.strategy_index = k;
        l.stats.attempts = k + 1;
        let clean = l.stats.routed_pct >= 100.0 && l.stats.drc_violations == 0;
        let better = match &best {
            None => true,
            Some(b) => (l.stats.routed, -(l.stats.drc_violations as i64)) > (b.stats.routed, -(b.stats.drc_violations as i64)),
        };
        if better {
            best = Some(l);
        } else if let Some(b) = best.as_mut() {
            b.stats.attempts = k + 1;
        }
        if clean {
            break;
        }
    }
    best.expect("at least one routing strategy")
}

/// The routing decisions: copper plus the final pin assignment. Everything
/// else (pours, connectivity, DRC, stats) is derived from it by [`finalize`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Routed {
    pub key: String,
    pub strategy_index: usize,
    pub gpio: Vec<(String, String)>,
    pub tracks: Vec<Track>,
    pub vias: Vec<Via>,
    #[serde(default)]
    pub status: Vec<NetRoute>,
    #[serde(default)]
    pub ripups: u32,
    #[serde(default)]
    pub expansions: u64,
}

/// Place → fan-out → negotiated routing → ground connection for one strategy.
pub fn route_phase(p: &Project, parts: &[PartPos], gpio: &[(String, String)], strat: &Strategy) -> Routed {
    let parts = parts.to_vec();
    let gpio = gpio.to_vec();
    let b0 = board::build(p, &parts, &gpio);
    let (pool, swap) = swap_pool(&b0, &gpio);
    let mut rt = route::Router::with_pool(&b0, &pool, &swap);
    rt.bot_cost = strat.bot_cost;
    rt.gnd_fanout(true);
    let order = net_order(&b0, strat.power_first);
    rt.route_negotiated(&order, strat.iterations);
    let _ = ground_connect(&b0, &mut rt);
    let tracks = rt.tracks.clone();
    let vias = rt.vias.clone();
    let status: Vec<NetRoute> = rt.status.clone();
    let ripups = rt.ripups_total;
    let expansions = rt.expansions;
    // final pin assignment = the pads the router claimed; a net that could
    // not claim one still gets a free GPIO pad (reported as unrouted).
    let mut gpio_final: Vec<(String, Option<String>)> = Vec::new();
    for (net, _) in &gpio {
        let nid = b0.nets.iter().position(|x| &x.name == net).map(|i| i as u16);
        let claimed = nid.and_then(|nid| rt.claim.iter().position(|&c| c == nid)).map(|k| b0.pads[pool[k]].pin.clone());
        gpio_final.push((net.clone(), claimed));
    }
    let taken: Vec<String> = gpio_final.iter().filter_map(|g| g.1.clone()).collect();
    let mut free: Vec<String> = pool.iter().map(|&pi| b0.pads[pi].pin.clone()).filter(|p| !taken.contains(p)).collect();
    let gpio_final: Vec<(String, String)> = gpio_final
        .into_iter()
        .map(|(net, pin)| {
            let pin = pin.unwrap_or_else(|| {
                let want = gpio.iter().find(|g| g.0 == net).map(|g| g.1.clone()).unwrap_or_default();
                let k = free.iter().position(|p| *p == want).unwrap_or(0);
                if free.is_empty() { want } else { free.remove(k) }
            });
            (net, pin)
        })
        .collect();
    drop(rt);
    Routed { key: cache_key_hex(p), strategy_index: STRATEGIES.iter().position(|s| s.name == strat.name).unwrap_or(0), gpio: gpio_final, tracks, vias, status, ripups, expansions }
}

/// Derive the full layout (pours, connectivity, DRC, statistics) from routed copper.
pub fn finalize(p: &Project, parts: &[PartPos], r: &Routed) -> Layout {
    let parts = parts.to_vec();
    let gpio = r.gpio.clone();
    let mut tracks = r.tracks.clone();
    let mut vias = r.vias.clone();
    let status = r.status.clone();
    let (ripups, expansions) = (r.ripups, r.expansions);
    let b = board::build(p, &parts, &gpio);
    for t in tracks.iter_mut() {
        t.net_id = b.nets.iter().position(|x| x.name == t.net).map(|i| i as u16).unwrap_or(u16::MAX - 2);
    }
    for v in vias.iter_mut() {
        v.net_id = b.nets.iter().position(|x| x.name == v.net).map(|i| i as u16).unwrap_or(u16::MAX - 2);
    }
    let pour_top = pour::compute(&b, &tracks, &vias, 0);
    let pour = pour::compute(&b, &tracks, &vias, 1);
    let conn = drc::connectivity(&b, &tracks, &vias, &[&pour_top, &pour]);
    let violations = drc::check(&b, &tracks, &vias, &[&pour_top, &pour], &conn);
    let mut nets = Vec::new();
    for (ni, n) in b.nets.iter().enumerate() {
        let c = conn.nets.iter().find(|c| c.net == n.name);
        let length: f64 = tracks.iter().filter(|t| t.net_id as usize == ni).map(|t| t.length()).sum();
        nets.push(NetView {
            name: n.name.clone(),
            class: n.class,
            pins: n.pins.clone(),
            description: n.description.clone(),
            current_a: n.current_a,
            routed: c.map(|c| c.unconnected.is_empty()).unwrap_or(true),
            length_mm: length,
            vias: vias.iter().filter(|v| v.net_id as usize == ni).count(),
            unconnected: c.map(|c| c.unconnected.clone()).unwrap_or_default(),
            ripups: status.iter().find(|s| s.net == n.name).map(|s| s.ripups).unwrap_or(0),
            ipc_width_mm: ipc2221_width_mm(n.current_a, 10.0, 1.0, true),
        });
    }
    let len_top: f64 = tracks.iter().filter(|t| t.layer == 0).map(|t| t.length()).sum();
    let len_bot: f64 = tracks.iter().filter(|t| t.layer == 1).map(|t| t.length()).sum();
    let comps: Vec<&PlacedPart> = b.parts.iter().filter(|x| !x.feature).collect();
    let smd_joints = b.pads.iter().filter(|x| x.drill <= 0.0 && !netlist::BOARD_FEATURES.contains(&x.refdes.as_str())).count();
    let tht_joints = b.pads.iter().filter(|x| x.drill > 0.0).count();
    let mut uniq: Vec<(String, String)> = Vec::new();
    for c in p.components.iter().filter(|c| c.on_pcb) {
        let k = (fab::bom_value(c), c.footprint.clone());
        if !uniq.contains(&k) {
            uniq.push(k);
        }
    }
    let stats = Stats {
        routed_pct: if conn.connections == 0 { 100.0 } else { 100.0 * conn.routed as f64 / conn.connections as f64 },
        connections: conn.connections,
        routed: conn.routed,
        drc_violations: violations.len(),
        via_count: vias.len(),
        track_length_mm: len_top + len_bot,
        track_length_top_mm: len_top,
        track_length_bottom_mm: len_bot,
        hpwl_mm: place::hpwl_board(&b),
        pour_area_mm2: pour.area_mm2 + pour_top.area_mm2,
        pour_islands_removed: pour.islands_removed + pour_top.islands_removed,
        ripups,
        expansions,
        parts: comps.len(),
        pads: b.pads.len(),
        nets: b.nets.len(),
        smd_joints,
        tht_joints,
        unique_parts: uniq.len(),
        ..Default::default()
    };
    Layout {
        board: BoardInfo { width: b.width, height: b.height, thickness: p.design.pcb.thickness_mm, layers: p.design.pcb.layers },
        rules: b.rules.clone(),
        parts: b.parts.clone(),
        pads: b.pads.clone(),
        nets,
        tracks,
        vias,
        holes: b.holes.clone(),
        keepouts: b.keepouts.clone(),
        pour,
        pour_top,
        drc: violations,
        connectivity: conn,
        stats,
        gpio,
        notes: notes_for(p),
    }
}

/// SoC pads usable by any GPIO signal (pin swapping) and the swappable nets.
fn swap_pool(b: &board::Board, gpio: &[(String, String)]) -> (Vec<usize>, Vec<u16>) {
    let pool: Vec<usize> = b
        .pads
        .iter()
        .enumerate()
        .filter(|(_, pd)| pd.refdes == "U1" && !netlist::SOC_RESERVED.contains(&pd.pin.as_str()))
        .map(|(i, _)| i)
        .collect();
    let swap: Vec<u16> = gpio.iter().filter_map(|(n, _)| b.nets.iter().position(|x| &x.name == n).map(|i| i as u16)).collect();
    (pool, swap)
}

/// Ground connection: pour GND on both layers, then join every ground group
/// (pour regions, pads, vias) to the main one — first with stitching vias
/// where a top region of one group overlaps a bottom region of the other,
/// otherwise with a short routed ground path. Returns (top, bottom) pours.
fn ground_connect(b: &board::Board, rt: &mut route::Router) -> (pour::Pour, pour::Pour) {
    let mut top = pour::compute(b, &rt.tracks, &rt.vias, 0);
    let mut bot = pour::compute(b, &rt.tracks, &rt.vias, 1);
    if b.gnd == u16::MAX {
        return (top, bot);
    }
    let n = rt.grid_n();
    for _ in 0..16 {
        let groups = drc::gnd_groups(b, &rt.tracks, &rt.vias, &[&top, &bot]);
        if groups.count <= 1 {
            break;
        }
        let main = groups.main;
        let group_of = |layer: usize, x: f64, y: f64| -> Option<usize> {
            let p = if layer == 0 { &top } else { &bot };
            p.region_at(x, y).map(|r| groups.region_group[layer][r])
        };
        let mut minors: Vec<usize> = groups.pad_group.iter().map(|x| x.1).filter(|&g| g != main).collect();
        minors.sort_unstable();
        minors.dedup();
        let mut progress = false;
        // 1) stitching vias: wherever a top region and a bottom region belong
        //    to different ground groups, one via merges them (union–find
        //    over the groups keeps it to one via per merge).
        let ng = groups.region_group.iter().flatten().copied().chain(groups.pad_group.iter().map(|x| x.1)).max().unwrap_or(0) + 1;
        let mut uf: Vec<usize> = (0..ng).collect();
        fn find(uf: &mut [usize], a: usize) -> usize {
            let mut r = a;
            while uf[r] != r {
                r = uf[r];
            }
            uf[a] = r;
            r
        }
        for c in 0..n {
            let (x, y) = rt.xy(c);
            let (Some(a), Some(bb)) = (group_of(0, x, y), group_of(1, x, y)) else { continue };
            let (ra, rb) = (find(&mut uf, a), find(&mut uf, bb));
            if ra == rb {
                continue;
            }
            if rt.try_gnd_via(c) {
                uf[ra] = rb;
                progress = true;
            }
        }
        let fixed: Vec<usize> = minors.iter().copied().filter(|&g| find(&mut uf, g) == find(&mut uf, main)).collect();
        // 2) routed bridges for groups a via could not join
        let mut main_nodes: Vec<usize> = Vec::new();
        for c in 0..n {
            let (x, y) = rt.xy(c);
            if group_of(0, x, y) == Some(main) {
                main_nodes.push(c);
            }
            if group_of(1, x, y) == Some(main) {
                main_nodes.push(n + c);
            }
        }
        for &g in minors.iter().filter(|g| !fixed.contains(g)) {
            let pads: Vec<usize> = groups.pad_group.iter().filter(|x| x.1 == g).map(|x| x.0).collect();
            let mut nodes: Vec<usize> = Vec::new();
            for c in 0..n {
                let (x, y) = rt.xy(c);
                if group_of(0, x, y) == Some(g) {
                    nodes.push(c);
                }
                if group_of(1, x, y) == Some(g) {
                    nodes.push(n + c);
                }
            }
            let ok = if nodes.is_empty() {
                pads.first().map(|&pi| rt.connect_pad(pi, &main_nodes)).unwrap_or(false)
            } else {
                rt.stitch(&nodes, &main_nodes)
            };
            progress |= ok;
        }
        top = pour::compute(b, &rt.tracks, &rt.vias, 0);
        bot = pour::compute(b, &rt.tracks, &rt.vias, 1);
        if !progress {
            break;
        }
    }
    (top, bot)
}

/// Re-route one net recording Lee's wavefront for the animation.
pub fn route_trace(p: &Project, net: &str, placement_override: Option<Vec<PartPos>>) -> Result<route::WaveTrace, String> {
    let pr = place::constructive(p);
    let (parts, gpio) = match placement_override {
        Some(ov) => {
            let g = place::assign_gpio(p, &ov);
            (ov, g)
        }
        None => (pr.parts, pr.gpio),
    };
    let b = board::build(p, &parts, &gpio);
    let ni = b.nets.iter().position(|n| n.name == net).ok_or_else(|| format!("unknown net `{net}`"))? as u16;
    if ni == b.gnd {
        return Err("GND is a pour connected by vias — pick a signal or power net".into());
    }
    let strat = STRATEGIES[layout(p).stats.strategy_index.min(STRATEGIES.len() - 1)];
    let (pool, swap) = swap_pool(&b, &gpio);
    let mut rt = route::Router::with_pool(&b, &pool, &swap);
    rt.bot_cost = strat.bot_cost;
    rt.gnd_fanout(true);
    let order = net_order(&b, strat.power_first);
    rt.route_negotiated(&order, strat.iterations);
    Ok(rt.trace_net(ni))
}

// ---------------------------------------------------------------------------
// API
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct LayoutIn {
    project: Project,
    #[serde(default)]
    placement: Option<Vec<PartPos>>,
}

#[derive(Deserialize)]
struct AdoptIn {
    project: Project,
    layout: Layout,
}

#[derive(Deserialize)]
struct TraceIn {
    project: Project,
    net: String,
    #[serde(default)]
    placement: Option<Vec<PartPos>>,
}

#[derive(Deserialize)]
struct AnnealIn {
    project: Project,
    #[serde(default = "default_iters")]
    iterations: usize,
    #[serde(default = "default_seed")]
    seed: u64,
}
fn default_iters() -> usize {
    20000
}
fn default_seed() -> u64 {
    42
}

#[derive(Deserialize)]
struct IpcIn {
    current_a: f64,
    #[serde(default = "ten")]
    temp_rise_c: f64,
    #[serde(default = "one")]
    copper_oz: f64,
    #[serde(default = "yes")]
    external: bool,
}
fn ten() -> f64 {
    10.0
}
fn one() -> f64 {
    1.0
}
fn yes() -> bool {
    true
}

#[derive(Serialize)]
struct LayoutView {
    #[serde(flatten)]
    layout: Layout,
    silk: fab::Silk,
}

#[derive(Serialize)]
struct FootprintLib {
    footprints: Vec<footprints::Footprint>,
    features: Vec<footprints::Footprint>,
}

/// JSON API entry point for `pcb.*` methods.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "pcb.footprints" => Some(crate::api::to_value(FootprintLib {
            footprints: footprints::library(),
            features: ["E1", "TP1"].iter().filter_map(|r| board::feature_footprint(r)).collect(),
        })),
        "pcb.layout" => handle(input, |i: LayoutIn| {
            let l = match i.placement {
                None => (*layout(&i.project)).clone(),
                Some(ov) => layout_with(&i.project, Some(ov)),
            };
            let silk = fab::silk_for(&i.project, &l);
            Ok(LayoutView { layout: l, silk })
        }),
        "pcb.adopt" => handle(input, |i: AdoptIn| {
            adopt(&i.project, i.layout);
            Ok(true)
        }),
        "pcb.route_trace" => handle(input, |i: TraceIn| route_trace(&i.project, &i.net, i.placement)),
        "pcb.fab_files" => handle(input, |i: LayoutIn| {
            let l = match i.placement {
                None => layout(&i.project),
                Some(ov) => Arc::new(layout_with(&i.project, Some(ov))),
            };
            Ok(fab::fab_files(&i.project, &l))
        }),
        "pcb.anneal" => handle(input, |i: AnnealIn| Ok(place::anneal(&i.project, i.iterations, i.seed))),
        "pcb.placement" => handle(input, |p: Project| Ok(placement(&p))),
        "pcb.ipc2221" => handle(input, |i: IpcIn| {
            Ok(serde_json::json!({
                "width_mm": ipc2221_width_mm(i.current_a, i.temp_rise_c, i.copper_oz, i.external),
                "current_for_0_15mm_a": ipc2221_current_a(0.15, i.temp_rise_c, i.copper_oz, i.external),
            }))
        }),
        _ => None,
    }
}

/// Metrics this module contributes to requirement verification.
pub fn metrics(p: &Project) -> Vec<Metric> {
    if !p.components.iter().any(|c| c.on_pcb) {
        return vec![];
    }
    let l = layout(p);
    let s = &l.stats;
    let unrouted: Vec<String> = l.connectivity.nets.iter().flat_map(|n| n.unconnected.iter().map(move |u| format!("{} ({})", u, n.net))).collect();
    let mut kinds: Vec<(String, usize)> = Vec::new();
    for v in &l.drc {
        if let Some(k) = kinds.iter_mut().find(|k| k.0 == v.kind) {
            k.1 += 1;
        } else {
            kinds.push((v.kind.clone(), 1));
        }
    }
    vec![
        Metric::new("pcb.routed_pct", "Connections routed", s.routed_pct, "%", "pcb").with_note(&if unrouted.is_empty() {
            format!("{}/{} connections on 2 layers, GND via stitched pours", s.routed, s.connections)
        } else {
            format!("{}/{} routed; open: {}", s.routed, s.connections, unrouted.join(", "))
        }),
        Metric::new("pcb.drc_violations", "DRC violations", s.drc_violations as f64, "count", "pcb").with_note(&if kinds.is_empty() {
            "clearance, width, edge, drill, keep-out, courtyard, connectivity all clean".to_string()
        } else {
            kinds.iter().map(|(k, n)| format!("{n}× {k}")).collect::<Vec<_>>().join(", ")
        }),
        Metric::new("pcb.via_count", "Vias", s.via_count as f64, "count", "pcb"),
        Metric::new("pcb.track_length_mm", "Total track length", s.track_length_mm, "mm", "pcb"),
        Metric::new("pcb.hpwl_mm", "Half-perimeter wire length", s.hpwl_mm, "mm", "pcb"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc2221_hand_calc() {
        // 1 A, 10 °C, 1 oz external: A = (1/(0.048·10^0.44))^(1/0.725) ≈ 16.3 mil²
        // → 16.3 / 1.378 ≈ 11.8 mil ≈ 0.30 mm (the familiar "12 mil per amp" rule).
        let w = ipc2221_width_mm(1.0, 10.0, 1.0, true);
        assert!((w - 0.300).abs() < 0.005, "{w}");
        let i = ipc2221_current_a(w, 10.0, 1.0, true);
        assert!((i - 1.0).abs() < 1e-9);
    }

    #[test]
    fn iris_board_routes_clean() {
        let p = Project::iris();
        let l = layout_with(&p, None);
        for v in &l.drc {
            eprintln!("DRC {}: {}", v.kind, v.message);
        }
        for n in l.nets.iter().filter(|n| !n.routed) {
            eprintln!("open {}: {:?}", n.name, n.unconnected);
        }
        eprintln!("stats {:?}", l.stats);
        assert_eq!(l.stats.routed_pct, 100.0);
        assert_eq!(l.stats.drc_violations, 0);
    }

    /// Regenerate the baked default route (run after changing the router):
    /// `cargo test -p forge-core regenerate_baked_route -- --ignored`
    #[test]
    #[ignore]
    fn regenerate_baked_route() {
        let p = Project::iris();
        let pr = place::constructive(&p);
        let mut out: Option<Routed> = None;
        for s in STRATEGIES {
            let r = route_phase(&p, &pr.parts, &pr.gpio, s);
            let l = finalize(&p, &pr.parts, &r);
            let clean = l.stats.routed_pct >= 100.0 && l.stats.drc_violations == 0;
            if out.is_none() || clean {
                out = Some(r);
            }
            if clean {
                break;
            }
        }
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/pcb/iris_route.json");
        std::fs::write(path, serde_json::to_string(&out.unwrap()).unwrap()).unwrap();
    }

    /// The baked route must be exactly what the router produces today.
    #[test]
    fn baked_route_matches_router() {
        let p = Project::iris();
        let baked = baked_route().expect("baked route parses");
        assert_eq!(baked.key, cache_key_hex(&p), "baked route is stale: run `cargo test -p forge-core regenerate_baked_route -- --ignored`");
        let pr = place::constructive(&p);
        let fresh = route_phase(&p, &pr.parts, &pr.gpio, &STRATEGIES[baked.strategy_index]);
        assert_eq!(fresh.gpio, baked.gpio);
        assert_eq!(fresh.tracks.len(), baked.tracks.len());
        assert_eq!(fresh.vias.len(), baked.vias.len());
        for (a, b) in fresh.tracks.iter().zip(baked.tracks.iter()) {
            assert!(a.net == b.net && a.layer == b.layer && (a.x0 - b.x0).abs() < 1e-9 && (a.y1 - b.y1).abs() < 1e-9, "track differs: {a:?} vs {b:?}");
        }
    }

    /// Dump the Iris layout as JSON for offline inspection:
    /// `PCB_DUMP=/tmp/l.json cargo test -p forge-core dump_layout -- --ignored`
    #[test]
    #[ignore]
    fn dump_layout() {
        let p = Project::iris();
        let l = layout_with(&p, None);
        if let Ok(path) = std::env::var("PCB_DUMP") {
            std::fs::write(path, serde_json::to_string(&l).unwrap()).unwrap();
        }
    }

    #[test]
    fn placement_keeps_public_interface() {
        let p = Project::iris();
        let pl = placement(&p);
        assert_eq!(pl.len(), p.components.iter().filter(|c| c.on_pcb).count());
        assert!(pl.iter().all(|x| x.side == "top"));
    }
}
