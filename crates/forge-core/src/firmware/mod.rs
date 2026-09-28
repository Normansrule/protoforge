//! # Prototype & Firmware Lab engine (`firmware.*`)
//!
//! The software half of turning Iris into a working prototype:
//!
//! | file | what |
//! |---|---|
//! | [`boards`] | "Iris v0" breadboard builds (XIAO nRF52840 Sense; ESP32-S3), BOM, wiring model, assembly steps, power comparison through `crate::power` |
//! | [`modules`] | C modules generated from the other labs: DSP filters in Q15/Q14, the Chip Lab gesture FSM as a `switch`, the RL Lab policy as a lookup table, the Control Lab complementary filter |
//! | [`hud`], [`gfx`], [`font`] | the heads-up display (HUD) UI description, an integer rasteriser and its C twin (pixel-identical) |
//! | [`adpcm`] | IMA ADPCM audio compression for the Bluetooth uplink |
//! | [`ble`] | Generic Attribute Profile (GATT) design, packed payload structs, throughput and latency maths |
//! | [`resources`] | RAM / flash budget on the nRF52840 |
//! | [`targets`] | complete projects: Arduino (XIAO), Arduino + Wokwi (ESP32-S3), Zephyr RTOS skeleton |
//!
//! Metrics: `firmware.ram_kb`, `firmware.flash_kb`, `firmware.ble_throughput_kbps`,
//! `firmware.caption_latency_ms`.
//!
//! References: Nordic Semiconductor, *nRF52840 Product Specification*;
//! Bluetooth SIG, *Bluetooth Core Specification* v5.x and Assigned Numbers;
//! Zephyr Project documentation (docs.zephyrproject.org); E. White,
//! *Making Embedded Systems*, 2nd ed., O'Reilly 2024.

pub mod adpcm;
pub mod ble;
pub mod boards;
pub mod font;
pub mod gfx;
pub mod hud;
pub mod modules;
pub mod resources;
pub mod targets;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{handle, ApiResult};
use crate::project::{Metric, Project};

fn link_of(p: &Project, over: Option<ble::LinkParams>) -> ble::LinkParams {
    over.unwrap_or_else(|| {
        p.design
            .extra
            .get("firmware_link")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    })
}

fn resource_params_of(p: &Project, over: Option<resources::ResourceParams>) -> resources::ResourceParams {
    over.unwrap_or_else(|| {
        let mut r: resources::ResourceParams = p
            .design
            .extra
            .get("firmware_resources")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        if !p.design.extra.contains_key("firmware_resources") {
            r.panel_w = p.design.optics.px_w.max(16);
            r.panel_h = p.design.optics.px_h.max(16);
            r.mics = p.design.audio.mic_count.max(1);
        }
        r
    })
}

// ------------------------------------------------------------------ inputs

#[derive(Deserialize, Default)]
#[serde(default)]
struct BreadboardIn {
    project: Option<Project>,
    build: boards::Build,
    battery_mah: f64,
    scenario: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    #[default]
    XiaoArduino,
    Esp32s3Wokwi,
    Zephyr,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ProjectIn {
    project: Option<Project>,
    target: Target,
    link: Option<ble::LinkParams>,
}

#[derive(Serialize)]
struct Provenance {
    source: String,
    item: String,
    value: String,
    file: String,
}

#[derive(Serialize)]
struct ProjectOut {
    target: Target,
    title: String,
    files: Vec<targets::File>,
    provenance: Vec<Provenance>,
    policy: Vec<u8>,
    soc_buckets: usize,
    policy_gain_pct: f64,
    wokwi_diagram: Option<Value>,
    commands: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ResourcesIn {
    project: Option<Project>,
    params: Option<resources::ResourceParams>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct BleIn {
    project: Option<Project>,
    link: Option<ble::LinkParams>,
}

#[derive(Serialize)]
struct BleOut {
    services: Vec<ble::Service>,
    link: ble::LinkResult,
    c_header: String,
    markdown: String,
    json_spec: String,
}

#[derive(Deserialize)]
#[serde(default)]
struct HudIn {
    state: hud::HudState,
    width: i32,
    height: i32,
    bpp: u8,
}
impl Default for HudIn {
    fn default() -> Self {
        HudIn {
            state: hud::HudState::default(),
            width: 128,
            height: 64,
            bpp: 1,
        }
    }
}

#[derive(Serialize)]
struct HudOut {
    width: i32,
    height: i32,
    bpp: u8,
    stride: i32,
    font_scale: i32,
    data_b64: String,
    lit: usize,
    hash: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct GestureIn {
    project: Option<Project>,
    state: usize,
    timer: u32,
    /// Constant touch level for `ticks` 1 ms ticks.
    touch: bool,
    ticks: u32,
    /// Or an explicit per-tick trace (overrides touch/ticks).
    trace: Option<Vec<bool>>,
}

#[derive(Serialize)]
struct GestureEvent {
    tick: usize,
    output: String,
}

#[derive(Serialize)]
struct GestureOut {
    state: usize,
    state_name: String,
    timer: u32,
    events: Vec<GestureEvent>,
    states: Vec<String>,
    tap_ms: u32,
    long_ms: u32,
    gap_ms: u32,
    c_source: String,
}

// ------------------------------------------------------------------ handlers

fn provenance(i: &modules::Inputs) -> Vec<Provenance> {
    let pv = |s: &str, it: &str, v: String, f: &str| Provenance {
        source: s.into(),
        item: it.into(),
        value: v,
        file: f.into(),
    };
    vec![
        pv("Control Lab", "complementary filter α", format!("{:.4}", i.fusion_alpha), "iris_config.h / iris_fusion.c"),
        pv("Control Lab", "IMU rate", format!("{:.0} Hz", i.imu_rate_hz), "iris_config.h"),
        pv(
            "DSP Lab",
            "decimation FIR",
            format!("{} taps, Q15, CIC N={} R={}", i.chain.fir.len(), i.chain.cic_order, i.chain.cic_r),
            "iris_filters.h",
        ),
        pv("DSP Lab", "voice high-pass", format!("{:.0} Hz Butterworth, Q14", i.hpf_hz), "iris_filters.h / iris_dsp.c"),
        pv(
            "Chip Lab",
            "gesture FSM",
            format!("{} states; tap < {} ms, long ≥ {} ms, gap {} ms", i.fsm.states.len(), i.tap_ms, i.long_ms, i.gap_ms),
            "iris_gesture.c",
        ),
        pv(
            "RL Lab",
            "learned power policy",
            format!("{} × 4 table, {:+.1} % battery life", i.soc_buckets, i.policy_gain_pct),
            "iris_policy.c",
        ),
        pv("Power Lab", "OCV curve", format!("{} → {} mV", i.ocv_mv[0], i.ocv_mv[20]), "iris_policy.c"),
        pv("Bluetooth tab", "connection intervals", format!("{:.0} / {:.0} ms, MTU {}", i.conn_fast_ms, i.conn_slow_ms, i.mtu), "iris_config.h / iris_ble_protocol.h"),
        pv("HUD simulator", "UI description", format!("{} screens", hud::Screen::ALL.len()), "iris_hud.c"),
    ]
}

fn project_files(i: ProjectIn) -> Result<ProjectOut, String> {
    let p = i.project.unwrap_or_else(Project::iris);
    let link = link_of(&p, i.link);
    let inp = modules::Inputs::from_project(&p, &link);
    let (title, files, diagram, commands) = match i.target {
        Target::XiaoArduino => (
            "XIAO nRF52840 Sense — Arduino / PlatformIO",
            targets::xiao_project(&inp),
            None,
            vec!["pio run".into(), "pio run -t upload".into(), "pio device monitor -b 115200".into()],
        ),
        Target::Esp32s3Wokwi => (
            "ESP32-S3 — Arduino, runs in Wokwi",
            targets::esp32_project(&inp),
            Some(targets::wokwi_diagram()),
            vec!["pio run".into(), "pio run -t upload".into(), "# VS Code + Wokwi extension: F1 → Wokwi: Start Simulator".into()],
        ),
        Target::Zephyr => (
            "nRF52840 custom board — Zephyr RTOS skeleton",
            targets::zephyr_project(&inp),
            None,
            vec![
                "west build -b nrf52840dk/nrf52840 .".into(),
                "west flash".into(),
                "west build -t ram_report".into(),
                "west build -t rom_report".into(),
            ],
        ),
    };
    Ok(ProjectOut {
        target: i.target,
        title: title.into(),
        files,
        provenance: provenance(&inp),
        policy: inp.policy.clone(),
        soc_buckets: inp.soc_buckets,
        policy_gain_pct: inp.policy_gain_pct,
        wokwi_diagram: diagram,
        commands,
    })
}

fn app_sources(inp: &modules::Inputs) -> Vec<String> {
    targets::zephyr_project(inp)
        .into_iter()
        .filter(|f| f.path.ends_with(".c"))
        .map(|f| f.content)
        .collect()
}

pub fn resources_for(p: &Project, params: Option<resources::ResourceParams>) -> resources::Resources {
    let link = link_of(p, None);
    let inp = modules::Inputs::from_project_light(p, &link);
    let srcs = app_sources(&inp);
    resources::estimate(&inp, &resource_params_of(p, params), &srcs)
}

fn gesture(i: GestureIn) -> Result<GestureOut, String> {
    let p = i.project.unwrap_or_else(Project::iris);
    let ch = crate::chip::ImplChoice::default().resolve(&p);
    let f = crate::chip::fsm::gesture_fsm(ch.tap_ms, ch.long_ms, ch.gap_ms);
    let trace = i.trace.unwrap_or_else(|| vec![i.touch; i.ticks.min(600_000) as usize]);
    let tr = crate::chip::fsm::simulate(&f, &trace, i.state, i.timer);
    Ok(GestureOut {
        state: tr.final_state,
        state_name: f.states[tr.final_state].name.clone(),
        timer: tr.final_timer,
        events: tr
            .events
            .iter()
            .map(|e| GestureEvent {
                tick: e.tick,
                output: e.output.clone(),
            })
            .collect(),
        states: f.states.iter().map(|s| s.name.clone()).collect(),
        tap_ms: ch.tap_ms,
        long_ms: ch.long_ms,
        gap_ms: ch.gap_ms,
        c_source: modules::gesture_c(&f),
    })
}

/// JSON API entry point for `firmware.*` methods.
pub fn dispatch(method: &str, input: &Value) -> Option<ApiResult> {
    match method {
        "firmware.breadboard" => handle(input, |i: BreadboardIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            Ok(boards::breadboard(&p, i.build, i.battery_mah, &i.scenario))
        }),
        "firmware.project" => handle(input, project_files),
        "firmware.resources" => handle(input, |i: ResourcesIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            Ok(resources_for(&p, i.params))
        }),
        "firmware.ble" => handle(input, |i: BleIn| {
            let p = i.project.unwrap_or_else(Project::iris);
            let lp = link_of(&p, i.link);
            let l = ble::link(&lp);
            let services = ble::gatt(l.params.mtu);
            Ok(BleOut {
                c_header: ble::c_header(l.params.mtu),
                markdown: ble::markdown(l.params.mtu, &l),
                json_spec: serde_json::to_string_pretty(&serde_json::json!({ "services": services, "link": &l })).unwrap_or_default(),
                services,
                link: l,
            })
        }),
        "firmware.hud_render" => handle(input, |i: HudIn| {
            let mut fb = gfx::Fb::new(i.width, i.height, i.bpp);
            hud::render(&mut fb, &i.state);
            Ok(HudOut {
                width: fb.w,
                height: fb.h,
                bpp: fb.bpp,
                stride: fb.stride,
                font_scale: hud::font_scale(fb.w, fb.h),
                lit: fb.lit(),
                hash: format!("{:016x}", gfx::fnv1a(&fb.buf)),
                data_b64: gfx::base64(&fb.buf),
            })
        }),
        "firmware.hud_screens" => handle(input, |_: Value| Ok(hud::describe())),
        "firmware.gesture" => handle(input, gesture),
        _ => None,
    }
}

#[derive(Clone)]
struct Cached {
    ram_kb: f64,
    flash_kb: f64,
    ram_note: String,
    flash_note: String,
}

thread_local! {
    static METRICS: crate::memo::Slot<Cached> = const { std::cell::RefCell::new(None) };
}

/// Metrics this module contributes to requirement verification.
pub fn metrics(p: &Project) -> Vec<Metric> {
    let key = crate::memo::key(&(
        &p.design.audio,
        &p.design.imu,
        &p.design.chip,
        &p.design.optics.px_w,
        &p.design.optics.px_h,
        &p.design.battery,
        p.design.extra.get("firmware_resources"),
        p.design.extra.get("firmware_link"),
        p.design.extra.get("rl_power"),
        &p.meta.name,
        &p.meta.version,
    ));
    let c = crate::memo::memo(&METRICS, key, || {
        let r = resources_for(p, None);
        let big = |v: &[resources::Item]| {
            let mut s: Vec<&resources::Item> = v.iter().collect();
            s.sort_by(|a, b| b.bytes.total_cmp(&a.bytes));
            s.iter()
                .take(3)
                .map(|x| format!("{} {:.1} KB", x.name.to_lowercase(), x.bytes / 1024.0))
                .collect::<Vec<_>>()
                .join(", ")
        };
        Cached {
            ram_kb: r.ram_kb,
            flash_kb: r.flash_kb,
            ram_note: format!(
                "{:.0} % of the nRF52840's 256 KB; largest: {} (Bluetooth stack and kernel are stated assumptions)",
                r.ram_pct,
                big(&r.ram)
            ),
            flash_note: format!(
                "{:.0} % of the {:.0} KB image budget ({}); largest: {}",
                r.flash_pct_of_budget,
                r.flash_budget_kb,
                if r.params.dfu { "two update slots" } else { "no update slot" },
                big(&r.flash)
            ),
        }
    });
    let l = ble::link(&link_of(p, None));
    vec![
        Metric::new("firmware.ram_kb", "Firmware RAM estimate (nRF52840)", c.ram_kb, "KB", "firmware").with_note(&c.ram_note),
        Metric::new("firmware.flash_kb", "Firmware flash image estimate", c.flash_kb, "KB", "firmware").with_note(&c.flash_note),
        Metric::new("firmware.ble_throughput_kbps", "BLE application throughput (typical phone)", l.throughput_kbps, "kbit/s", "firmware")
            .with_note(&format!(
                "{} PHY, MTU {}, {:.0} ms interval, {} packets/event (phone-dependent assumption); audio uplink needs {:.0} kbit/s",
                l.params.phy, l.params.mtu, l.params.conn_interval_ms, l.packets_per_event, l.audio_demand_kbps
            )),
        Metric::new("firmware.caption_latency_ms", "Caption link latency (BLE + buffering)", l.caption_link_latency_ms, "ms", "firmware")
            .with_note(&format!(
                "20 ms audio frame + uplink {:.1} ms + downlink {:.1} ms (average); speech recognition on the phone not included (assumed {:.0} ms → {:.0} ms end to end)",
                l.audio_uplink_avg_ms, l.caption_downlink_avg_ms, l.params.asr_ms, l.caption_end_to_end_ms
            )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_metrics_are_sensible_and_fast() {
        let p = Project::iris();
        let t0 = std::time::Instant::now();
        let m = metrics(&p);
        let dt = t0.elapsed();
        let get = |k: &str| m.iter().find(|x| x.key == k).unwrap().value;
        let ram = get("firmware.ram_kb");
        let flash = get("firmware.flash_kb");
        let thr = get("firmware.ble_throughput_kbps");
        let lat = get("firmware.caption_latency_ms");
        eprintln!("firmware metrics in {dt:?}: RAM {ram:.1} KB, flash {flash:.1} KB, {thr:.0} kbit/s, {lat:.1} ms");
        for x in &m {
            eprintln!("  {} = {:.2} — {}", x.key, x.value, x.note);
        }
        assert!(ram > 60.0 && ram < 192.0, "{ram}");
        assert!(flash > 150.0 && flash < 472.0, "{flash}");
        assert!(thr > 64.0);
        assert!(lat > 20.0 && lat < 200.0);
        // memoised second call is fast
        let t1 = std::time::Instant::now();
        let _ = metrics(&p);
        assert!(t1.elapsed().as_millis() < 20);
    }

    #[test]
    fn dispatch_round_trips() {
        let r = dispatch("firmware.hud_render", &serde_json::json!({"width": 128, "height": 64})).unwrap().unwrap();
        assert_eq!(r["stride"], 16);
        assert_eq!(r["data_b64"].as_str().unwrap().len(), (128 * 64 / 8usize).div_ceil(3) * 4);
        let r = dispatch("firmware.gesture", &serde_json::json!({"trace": crate::chip::fsm::scripted("double_tap")})).unwrap().unwrap();
        assert_eq!(r["events"][0]["output"], "double_tap");
        let r = dispatch("firmware.breadboard", &serde_json::json!({"build": "esp32s3"})).unwrap().unwrap();
        assert!(r["wires"].as_array().unwrap().len() >= 15);
        let r = dispatch("firmware.ble", &Value::Null).unwrap().unwrap();
        assert!(r["c_header"].as_str().unwrap().contains("IRIS_UUID16_BATTERY_SERVICE 0x180F"));
        for t in ["xiao_arduino", "esp32s3_wokwi", "zephyr"] {
            let r = dispatch("firmware.project", &serde_json::json!({"target": t})).unwrap().unwrap();
            assert!(r["files"].as_array().unwrap().len() >= 5, "{t}");
        }
        assert!(dispatch("firmware.hud_screens", &Value::Null).unwrap().is_ok());
        assert!(dispatch("firmware.nope", &Value::Null).is_none());
    }
}
