//! Shared C modules generated from the other labs' designs:
//!
//! | file | source lab | what |
//! |---|---|---|
//! | `iris_config.h` | project | rates, thresholds, panel size, radio intervals |
//! | `iris_filters.h` | DSP Lab (`dsp::pdm`, `dsp::iir`) | decimation FIR in Q15, voice high-pass biquad in Q14 |
//! | `iris_dsp.{h,c}` | — | fixed-point FIR/biquad kernels, RMS, IMA ADPCM encoder |
//! | `iris_gesture.{h,c}` | Chip Lab (`chip::fsm`) | the touch-gesture finite-state machine as a C `switch` |
//! | `iris_policy.{h,c}` | RL Lab (`rl::power`) + Power Lab (`power::battery::ocv`) | learned power policy as a lookup table |
//! | `iris_fusion.{h,c}` | Control Lab (`design.imu.fusion_alpha`) | complementary filter |
//!
//! Fixed point: a Q15 number is an `int16_t` code c meaning c / 2^15
//! (range [−1, 1)); Q14 means c / 2^14 (range [−2, 2)), needed because biquad
//! feedback coefficients reach ±2. Conversion rounds to nearest and
//! saturates (see `dsp::filter::quantize_with`).

use serde::Serialize;

use crate::chip::fsm::Fsm;
use crate::dsp::iir::Biquad;
use crate::dsp::pdm::ChainDesign;
use crate::project::Project;

/// Everything the generators read from the project, computed once.
#[derive(Clone, Debug, Serialize)]
pub struct Inputs {
    pub name: String,
    pub version: String,
    pub fusion_alpha: f64,
    pub imu_rate_hz: f64,
    pub audio_rate_hz: f64,
    pub fsm: Fsm,
    pub tap_ms: u32,
    pub long_ms: u32,
    pub gap_ms: u32,
    pub chain: ChainDesign,
    pub decim_fir_q15: Vec<i64>,
    pub hpf: Biquad,
    pub hpf_q14: Vec<i64>,
    pub hpf_hz: f64,
    pub policy: Vec<u8>,
    pub soc_buckets: usize,
    pub policy_gain_pct: f64,
    pub fixed_life_h: f64,
    pub learned_life_h: f64,
    pub ocv_mv: Vec<u32>,
    pub full_v: f64,
    pub cutoff_v: f64,
    pub step_min: f64,
    pub conn_fast_ms: f64,
    pub conn_slow_ms: f64,
    pub mtu: u32,
    /// Project-specific 128-bit GATT service base (version-5 UUID of the project name).
    pub uuid_base: String,
}

/// Q15 codes of a coefficient set (round to nearest, saturate at ±1).
pub fn q15(c: &[f64]) -> Vec<i64> {
    crate::dsp::filter::quantize_with(c, 16, 15).codes
}

pub fn q14(c: &[f64]) -> Vec<i64> {
    crate::dsp::filter::quantize_with(c, 16, 14).codes
}

thread_local! {
    static POLICY: crate::memo::Slot<crate::rl::power::PowerResult> = const { std::cell::RefCell::new(None) };
}

/// The RL Lab's learned policy for this project (memoised; same config as its metric).
pub fn learned_policy(p: &Project) -> crate::rl::power::PowerResult {
    let cfg: crate::rl::power::PowerConfig = p
        .design
        .extra
        .get("rl_power")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let cfg = crate::rl::power::PowerConfig { with_dp: false, ..cfg };
    let key = crate::memo::key(&(&p.components, &p.design.battery, &p.design.power_rails, &cfg));
    crate::memo::memo(&POLICY, key, || crate::rl::power::train(p, &cfg))
}

impl Inputs {
    /// Full inputs, including the RL Lab's trained policy.
    pub fn from_project(p: &Project, link: &super::ble::LinkParams) -> Inputs {
        Self::build(p, link, true)
    }

    /// Same, but with the fixed duty-cycle policy instead of training (cheap; used by metrics,
    /// where only the table's size matters).
    pub fn from_project_light(p: &Project, link: &super::ble::LinkParams) -> Inputs {
        Self::build(p, link, false)
    }

    fn build(p: &Project, link: &super::ble::LinkParams, train: bool) -> Inputs {
        let ch = crate::chip::ImplChoice::default().resolve(p);
        let fsm = crate::chip::fsm::gesture_fsm(ch.tap_ms, ch.long_ms, ch.gap_ms);
        let audio = &p.design.audio;
        let chain = crate::dsp::pdm::design_chain(audio, &crate::dsp::pdm::ChainOptions::default());
        let fs_out = chain.fs_out;
        let hpf_hz = audio.voice_band_lo_hz.clamp(20.0, fs_out / 4.0);
        let hpf = crate::dsp::iir::butterworth(2, hpf_hz, fs_out, true)[0];
        let hpf_q14 = q14(&[hpf.b[0], hpf.b[1], hpf.b[2], hpf.a[1], hpf.a[2]]);
        let pol = if train {
            learned_policy(p)
        } else {
            let cfg: crate::rl::power::PowerConfig = p
                .design
                .extra
                .get("rl_power")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let nb = cfg.soc_buckets.clamp(2, 40);
            crate::rl::power::PowerResult {
                soc_buckets: nb,
                // fixed reference: display full when the user needs it, radio fast
                policy: (0..nb * 4).map(|s| if s % 4 == 0 { 0 } else { 4 }).collect(),
                ..Default::default()
            }
        };
        let b = &p.design.battery;
        let ocv_mv = (0..=20)
            .map(|i| (crate::power::battery::ocv(i as f64 / 20.0, b.full_v) * 1000.0).round() as u32)
            .collect();
        let cfg: crate::rl::power::PowerConfig = p
            .design
            .extra
            .get("rl_power")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        Inputs {
            name: if p.meta.codename.is_empty() {
                p.meta.name.clone()
            } else {
                p.meta.codename.clone()
            },
            version: if p.meta.version.is_empty() {
                "0.1.0".into()
            } else {
                p.meta.version.clone()
            },
            fusion_alpha: p.design.imu.fusion_alpha.clamp(0.0, 0.99999),
            imu_rate_hz: p.design.imu.rate_hz.clamp(10.0, 4000.0),
            audio_rate_hz: fs_out,
            fsm,
            tap_ms: ch.tap_ms,
            long_ms: ch.long_ms,
            gap_ms: ch.gap_ms,
            decim_fir_q15: q15(&chain.fir),
            chain,
            hpf,
            hpf_q14,
            hpf_hz,
            soc_buckets: pol.soc_buckets,
            policy: pol.policy.clone(),
            policy_gain_pct: pol.gain_pct,
            fixed_life_h: pol.fixed.life_h,
            learned_life_h: pol.learned.life_h,
            ocv_mv,
            full_v: b.full_v,
            cutoff_v: b.cutoff_v,
            step_min: cfg.step_min,
            conn_fast_ms: link.conn_interval_ms,
            conn_slow_ms: link.slow_interval_ms,
            mtu: link.mtu,
            uuid_base: super::ble::service_base(&p.meta.name),
        }
    }
}

fn join<T: std::fmt::Display>(v: &[T]) -> String {
    v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ")
}

const CPP_OPEN: &str = "#ifdef __cplusplus\nextern \"C\" {\n#endif\n";
const CPP_CLOSE: &str = "#ifdef __cplusplus\n}\n#endif\n";

/// Panel / target specifics for `iris_config.h`.
pub struct TargetCfg {
    pub macro_name: &'static str,
    pub hud_w: u32,
    pub hud_h: u32,
    pub hud_bpp: u32,
    /// Display bus: "SPI" or "I2C", and its clock.
    pub disp_bus: &'static str,
    pub disp_bus_hz: u32,
}

/// Wire time of one full frame (I²C: 9 bit times per byte incl. ACK).
pub fn frame_us(t: &TargetCfg) -> u32 {
    let bytes = (t.hud_w * t.hud_h * t.hud_bpp).div_ceil(8) as f64;
    let bits = if t.disp_bus == "I2C" { 9.0 } else { 8.0 };
    (bytes * bits / t.disp_bus_hz.max(1) as f64 * 1e6).round() as u32
}

pub fn config_h(i: &Inputs, t: &TargetCfg) -> String {
    let dt = 1.0 / i.imu_rate_hz;
    let fc = (1.0 - i.fusion_alpha) / (2.0 * std::f64::consts::PI * i.fusion_alpha.max(1e-6) * dt);
    format!(
        r#"/* iris_config.h — build-time constants for {name} v{ver} (generated by Protoforge).
 * Every number here comes from the project file or another lab; regenerate
 * instead of editing so the firmware and the simulations stay in step. */
#ifndef IRIS_CONFIG_H
#define IRIS_CONFIG_H

#define {tgt} 1
#define IRIS_FW_VERSION "{ver}"

/* Sensing (Control Lab): complementary filter, crossover f_c = (1 - a) / (2 pi a dt) = {fc:.2} Hz */
#define IRIS_IMU_RATE_HZ {imu:.0}
#define IRIS_IMU_PERIOD_US {imu_us}
#define IRIS_FUSION_ALPHA {alpha:.5}f

/* Audio (DSP Lab) */
#define IRIS_AUDIO_RATE_HZ {fs:.0}
#define IRIS_AUDIO_FRAME_MS 20
#define IRIS_AUDIO_FRAME_SAMPLES ({fs:.0} / 50)

/* Touch gestures (Chip Lab FSM), 1 kHz tick = 1 ms */
#define IRIS_GESTURE_TICK_HZ 1000
#define IRIS_TAP_MAX_MS {tap}
#define IRIS_LONG_PRESS_MS {long}
#define IRIS_DOUBLE_GAP_MS {gap}

/* Heads-up display panel and link timing (HUD simulator frame budget) */
#ifndef IRIS_HUD_W
#define IRIS_HUD_W {w}
#define IRIS_HUD_H {h}
#endif
#define IRIS_HUD_BPP {bpp}
#define IRIS_HUD_FPS_FULL 10
#define IRIS_HUD_FPS_LOW 2
#define IRIS_DISP_BUS_{bus} 1
#define IRIS_DISP_BUS_HZ {bus_hz}u
#define IRIS_DISP_FRAME_BYTES ((IRIS_HUD_W * IRIS_HUD_H * IRIS_HUD_BPP + 7) / 8)
/* one full frame on the wire: {frame_bytes} B x {bits_per_byte} bit / {bus_mhz} MHz = {frame_us} us (max {max_fps} fps) */
#define IRIS_DISP_FRAME_US {frame_us}u

/* Radio (Bluetooth tab): connection intervals the power policy switches between */
#define IRIS_CONN_FAST_MS {fast:.0}
#define IRIS_CONN_SLOW_MS {slow:.0}

/* Power policy (RL Lab): one decision per step */
#define IRIS_POLICY_STEP_MS {step_ms}

#endif /* IRIS_CONFIG_H */
"#,
        name = i.name,
        ver = i.version,
        tgt = t.macro_name,
        imu = i.imu_rate_hz,
        imu_us = (1e6 / i.imu_rate_hz).round() as u32,
        alpha = i.fusion_alpha,
        fs = i.audio_rate_hz,
        tap = i.tap_ms,
        long = i.long_ms,
        gap = i.gap_ms,
        w = t.hud_w,
        h = t.hud_h,
        bpp = t.hud_bpp,
        bus = t.disp_bus,
        bus_hz = t.disp_bus_hz,
        frame_bytes = (t.hud_w * t.hud_h * t.hud_bpp).div_ceil(8),
        bits_per_byte = if t.disp_bus == "I2C" { 9 } else { 8 },
        bus_mhz = t.disp_bus_hz as f64 / 1e6,
        frame_us = frame_us(t),
        max_fps = (1e6 / frame_us(t).max(1) as f64).floor(),
        fast = i.conn_fast_ms,
        slow = i.conn_slow_ms,
        step_ms = (i.step_min * 60_000.0).round() as u32,
    )
}

pub fn filters_h(i: &Inputs) -> String {
    let c = &i.chain;
    let mut s = format!(
        "/* iris_filters.h — filter coefficients from the DSP Lab (generated by Protoforge).\n *\n * PDM decimation chain: {:.0} Hz PDM -> CIC (N = {}, R = {}, M = {}) -> {}{}-tap compensation FIR (decimate by {}) -> {:.0} Hz PCM.\n * On the nRF52840 and on I2S microphones the decimation happens in hardware; this FIR is what the\n * IRIS-DSP co-processor (Chip Lab) or a software PDM path runs. Coefficients: Q15 (value = code / 32768).\n */\n#ifndef IRIS_FILTERS_H\n#define IRIS_FILTERS_H\n#include <stdint.h>\n\n",
        c.pdm_hz,
        c.cic_order,
        c.cic_r,
        c.cic_m,
        if c.halfband.is_empty() { String::new() } else { format!("half-band ({} taps) -> ", c.halfband.len()) },
        c.fir.len(),
        c.fir_decim,
        c.fs_out
    );
    s += &format!(
        "#define IRIS_CIC_ORDER {}\n#define IRIS_CIC_R {}\n#define IRIS_CIC_M {}\n#define IRIS_CIC_REGISTER_BITS {}\n#define IRIS_DECIM_FIR_DECIMATION {}\n#define IRIS_DECIM_FIR_TAPS {}\n",
        c.cic_order,
        c.cic_r,
        c.cic_m,
        c.cic_register_bits,
        c.fir_decim,
        c.fir.len()
    );
    s += &format!(
        "static const int16_t IRIS_DECIM_FIR_Q15[{}] = {{\n    {}\n}};\n\n",
        c.fir.len(),
        join(&i.decim_fir_q15)
    );
    if !c.halfband.is_empty() {
        s += &format!(
            "#define IRIS_HALFBAND_TAPS {}\nstatic const int16_t IRIS_HALFBAND_Q15[{}] = {{{}}};\n\n",
            c.halfband.len(),
            c.halfband.len(),
            join(&q15(&c.halfband))
        );
    }
    s += &format!(
        "/* Voice high-pass: 2nd-order Butterworth at {:.0} Hz, fs = {:.0} Hz (bilinear transform), Q14:\n * b0 = {:.6}, b1 = {:.6}, b2 = {:.6}, a1 = {:.6}, a2 = {:.6} */\nstatic const int16_t IRIS_HPF_Q14[5] = {{{}}}; /* b0, b1, b2, a1, a2 */\n\n#endif /* IRIS_FILTERS_H */\n",
        i.hpf_hz,
        i.audio_rate_hz,
        i.hpf.b[0],
        i.hpf.b[1],
        i.hpf.b[2],
        i.hpf.a[1],
        i.hpf.a[2],
        join(&i.hpf_q14)
    );
    s
}

pub fn dsp_h() -> String {
    format!(
        "/* iris_dsp.h — fixed-point audio kernels (generated by Protoforge). */\n#ifndef IRIS_DSP_H\n#define IRIS_DSP_H\n#include <stdint.h>\n{CPP_OPEN}\ntypedef struct {{ int16_t x1, x2, y1, y2; }} iris_biquad_t;\ntypedef struct {{ int16_t pred; uint8_t index; }} iris_adpcm_t;\n\n/* Direct-form-I biquad with Q14 coefficients {{b0, b1, b2, a1, a2}}, in place. */\nvoid iris_biquad_q14(iris_biquad_t *s, const int16_t *c, int16_t *x, int32_t n);\n/* One FIR output: sum h[k] * x[n-k], Q15 taps, x points at the newest sample. */\nint16_t iris_fir_q15(const int16_t *h, int32_t taps, const int16_t *x_newest);\n/* Root-mean-square of a block (integer square root). */\nuint16_t iris_rms(const int16_t *x, int32_t n);\n/* IMA ADPCM: n samples -> (n + 1) / 2 bytes, low nibble first. */\nvoid iris_adpcm_encode(iris_adpcm_t *st, const int16_t *x, int32_t n, uint8_t *out);\n\n{CPP_CLOSE}#endif /* IRIS_DSP_H */\n"
    )
}

pub fn dsp_c() -> String {
    let mut s = String::from("/* iris_dsp.c — generated by Protoforge. */\n#include \"iris_dsp.h\"\n\n");
    s += r#"static int16_t iris_sat16(int64_t v) {
    return (int16_t)(v > 32767 ? 32767 : (v < -32768 ? -32768 : v));
}

void iris_biquad_q14(iris_biquad_t *s, const int16_t *c, int16_t *x, int32_t n) {
    for (int32_t i = 0; i < n; i++) {
        int64_t acc = (int64_t)c[0] * x[i] + (int64_t)c[1] * s->x1 + (int64_t)c[2] * s->x2
                    - (int64_t)c[3] * s->y1 - (int64_t)c[4] * s->y2;
        int16_t y = iris_sat16((acc + (1 << 13)) >> 14);
        s->x2 = s->x1;
        s->x1 = x[i];
        s->y2 = s->y1;
        s->y1 = y;
        x[i] = y;
    }
}

int16_t iris_fir_q15(const int16_t *h, int32_t taps, const int16_t *x_newest) {
    int64_t acc = 0;
    for (int32_t k = 0; k < taps; k++) acc += (int64_t)h[k] * x_newest[-k];
    return iris_sat16((acc + (1 << 14)) >> 15);
}

uint16_t iris_rms(const int16_t *x, int32_t n) {
    if (n <= 0) return 0;
    uint64_t ss = 0;
    for (int32_t i = 0; i < n; i++) ss += (uint64_t)((int32_t)x[i] * x[i]);
    uint32_t m = (uint32_t)(ss / (uint64_t)n);
    uint32_t r = 0, bit = 1u << 30;
    while (bit > m) bit >>= 2;
    while (bit) {
        if (m >= r + bit) { m -= r + bit; r = (r >> 1) + bit; } else { r >>= 1; }
        bit >>= 2;
    }
    return (uint16_t)r;
}

"#;
    s += &super::adpcm::c_source();
    s
}

/// Rust mirror of `iris_biquad_q14` (for tests and the web's audio preview).
pub fn biquad_q14(c: &[i64], x: &[i16]) -> Vec<i16> {
    let (mut x1, mut x2, mut y1, mut y2) = (0i64, 0i64, 0i64, 0i64);
    x.iter()
        .map(|&xi| {
            let acc = c[0] * xi as i64 + c[1] * x1 + c[2] * x2 - c[3] * y1 - c[4] * y2;
            let y = ((acc + (1 << 13)) >> 14).clamp(-32768, 32767);
            x2 = x1;
            x1 = xi as i64;
            y2 = y1;
            y1 = y;
            y as i16
        })
        .collect()
}

// ------------------------------------------------------------------ gesture FSM

fn guard_c(g: &crate::chip::fsm::Guard) -> String {
    let mut p = Vec::new();
    if let Some(t) = g.touch {
        p.push(if t { "touch".to_string() } else { "!touch".to_string() });
    }
    if let Some(n) = g.timer_ge {
        p.push(format!("t >= {n}u"));
    }
    if let Some(n) = g.timer_lt {
        p.push(format!("t < {n}u"));
    }
    if p.is_empty() {
        "true".into()
    } else {
        p.join(" && ")
    }
}

pub fn gesture_h(f: &Fsm) -> String {
    let mut s = format!(
        "/* iris_gesture.h — touch-gesture recogniser (generated by Protoforge from the Chip Lab's\n * `{}` Moore machine: {} states, {} transitions, 1 kHz tick). */\n#ifndef IRIS_GESTURE_H\n#define IRIS_GESTURE_H\n#include <stdint.h>\n#include <stdbool.h>\n{CPP_OPEN}\n",
        f.name,
        f.states.len(),
        f.transitions.len()
    );
    s += "typedef enum {\n";
    for (i, st) in f.states.iter().enumerate() {
        s += &format!("    IRIS_G_{} = {i}, /* {} */\n", st.name, st.description);
    }
    s += "} iris_gesture_state_t;\n\n";
    for (i, o) in f.outputs.iter().enumerate() {
        s += &format!("#define IRIS_GESTURE_{} 0x{:02x}u\n", o.to_uppercase(), 1u32 << i);
    }
    s += &format!(
        "/* the state timer saturates like the {}-bit counter in the Verilog */\n#define IRIS_GESTURE_TIMER_MAX {}u\n\n",
        f.timer_bits(),
        (1u32 << f.timer_bits()) - 1
    );
    s += "typedef struct {\n    uint8_t state;\n    uint16_t timer; /* ticks since the current state was entered */\n} iris_gesture_t;\n\n";
    s += "void iris_gesture_init(iris_gesture_t *g);\n/* Advance one 1 ms tick; returns the output bits of the new state (IRIS_GESTURE_*). */\nuint8_t iris_gesture_step(iris_gesture_t *g, bool touch);\nconst char *iris_gesture_state_name(uint8_t s);\n\n";
    s += CPP_CLOSE;
    s += "#endif /* IRIS_GESTURE_H */\n";
    s
}

pub fn gesture_c(f: &Fsm) -> String {
    let mut s = String::from("/* iris_gesture.c — generated by Protoforge; transitions are tried in the Chip Lab's order,\n * the first matching guard wins (same semantics as chip::fsm::Fsm::step and the Verilog). */\n#include \"iris_gesture.h\"\n\n");
    let masks: Vec<String> = f
        .states
        .iter()
        .map(|st| {
            let m: u32 = f
                .outputs
                .iter()
                .enumerate()
                .filter(|(_, o)| st.outputs.contains(o))
                .map(|(i, _)| 1u32 << i)
                .sum();
            format!("0x{m:02x}u")
        })
        .collect();
    s += &format!(
        "static const uint8_t IRIS_GESTURE_OUT[{}] = {{{}}};\n",
        f.states.len(),
        masks.join(", ")
    );
    s += &format!(
        "static const char *const IRIS_GESTURE_NAMES[{}] = {{{}}};\n\n",
        f.states.len(),
        f.states.iter().map(|st| format!("\"{}\"", st.name)).collect::<Vec<_>>().join(", ")
    );
    s += &format!(
        "void iris_gesture_init(iris_gesture_t *g) {{\n    g->state = IRIS_G_{};\n    g->timer = 0;\n}}\n\n",
        f.reset
    );
    s += "const char *iris_gesture_state_name(uint8_t s) {\n    return s < sizeof IRIS_GESTURE_NAMES / sizeof IRIS_GESTURE_NAMES[0] ? IRIS_GESTURE_NAMES[s] : \"?\";\n}\n\n";
    s += "uint8_t iris_gesture_step(iris_gesture_t *g, bool touch) {\n    const uint16_t t = g->timer;\n    uint8_t next = g->state;\n    (void)t;\n    switch (g->state) {\n";
    for st in &f.states {
        s += &format!("    case IRIS_G_{}:\n", st.name);
        for tr in f.transitions.iter().filter(|t| t.from == st.name) {
            s += &format!("        if ({}) {{ next = IRIS_G_{}; break; }}\n", guard_c(&tr.guard), tr.to);
        }
        s += "        break;\n";
    }
    s += &format!("    default:\n        next = IRIS_G_{};\n        break;\n    }}\n", f.reset);
    s += "    if (next != g->state) {\n        g->state = next;\n        g->timer = 0;\n    } else if (g->timer < IRIS_GESTURE_TIMER_MAX) {\n        g->timer++;\n    }\n    return IRIS_GESTURE_OUT[g->state];\n}\n";
    s
}

// ------------------------------------------------------------------ power policy

pub fn policy_h(i: &Inputs) -> String {
    format!(
        "/* iris_policy.h — learned power-management policy (generated by Protoforge from the RL Lab).\n * Q-learning over (battery bucket x user context) -> (display mode x radio interval);\n * the RL Lab predicts {:.2} h -> {:.2} h battery life vs the fixed duty-cycle policy ({:+.1} %). */\n#ifndef IRIS_POLICY_H\n#define IRIS_POLICY_H\n#include <stdint.h>\n{CPP_OPEN}\n#define IRIS_SOC_BUCKETS {}\n#define IRIS_CTX_IDLE 0\n#define IRIS_CTX_GLANCE 1\n#define IRIS_CTX_NAVIGATE 2\n#define IRIS_CTX_CAPTION 3\n#define IRIS_DISPLAY_OFF 0\n#define IRIS_DISPLAY_LOW 1\n#define IRIS_DISPLAY_FULL 2\n#define IRIS_RADIO_FAST 0\n#define IRIS_RADIO_SLOW 1\n\n/* action = display * 2 + radio */\nuint8_t iris_policy_action(uint8_t soc_bucket, uint8_t ctx);\nstatic inline uint8_t iris_action_display(uint8_t a) {{ return (uint8_t)(a >> 1); }}\nstatic inline uint8_t iris_action_radio(uint8_t a) {{ return (uint8_t)(a & 1u); }}\n/* State of charge 0..1 from the resting cell voltage (Power Lab OCV curve). */\nfloat iris_soc_from_mv(uint16_t mv);\nuint8_t iris_soc_bucket(float soc);\n\n{CPP_CLOSE}#endif /* IRIS_POLICY_H */\n",
        i.fixed_life_h, i.learned_life_h, i.policy_gain_pct, i.soc_buckets
    )
}

pub fn policy_c(i: &Inputs) -> String {
    let nb = i.soc_buckets;
    let mut rows = String::new();
    for b in 0..nb {
        let row: Vec<String> = (0..4).map(|c| i.policy.get(b * 4 + c).copied().unwrap_or(4).to_string()).collect();
        let lo = b as f64 / nb as f64 * 100.0;
        let hi = (b + 1) as f64 / nb as f64 * 100.0;
        rows += &format!(
            "    {{{}}}, /* SoC {:.0}-{:.0} %: idle, glance, navigate, caption */\n",
            row.join(", "),
            lo,
            hi
        );
    }
    format!(
        r#"/* iris_policy.c — generated by Protoforge. */
#include "iris_policy.h"

static const uint8_t IRIS_POLICY[IRIS_SOC_BUCKETS][4] = {{
{rows}}};

/* Open-circuit voltage at SoC = 0, 5, ..., 100 % (mV). */
static const uint16_t IRIS_OCV_MV[21] = {{{ocv}}};

uint8_t iris_policy_action(uint8_t soc_bucket, uint8_t ctx) {{
    if (soc_bucket >= IRIS_SOC_BUCKETS) soc_bucket = IRIS_SOC_BUCKETS - 1;
    if (ctx > 3) ctx = 0;
    return IRIS_POLICY[soc_bucket][ctx];
}}

float iris_soc_from_mv(uint16_t mv) {{
    if (mv <= IRIS_OCV_MV[0]) return 0.0f;
    if (mv >= IRIS_OCV_MV[20]) return 1.0f;
    for (int i = 1; i < 21; i++) {{
        if (mv <= IRIS_OCV_MV[i]) {{
            float f = (float)(mv - IRIS_OCV_MV[i - 1]) / (float)(IRIS_OCV_MV[i] - IRIS_OCV_MV[i - 1]);
            return ((float)(i - 1) + f) / 20.0f;
        }}
    }}
    return 1.0f;
}}

uint8_t iris_soc_bucket(float soc) {{
    if (soc < 0.0f) soc = 0.0f;
    if (soc > 0.999999f) soc = 0.999999f;
    return (uint8_t)(soc * IRIS_SOC_BUCKETS);
}}
"#,
        ocv = join(&i.ocv_mv)
    )
}

// ------------------------------------------------------------------ fusion

pub fn fusion_h() -> String {
    format!(
        "/* iris_fusion.h — complementary filter (generated by Protoforge; alpha from the Control Lab). */\n#ifndef IRIS_FUSION_H\n#define IRIS_FUSION_H\n{CPP_OPEN}\ntypedef struct {{\n    float pitch_deg, roll_deg; /* tilt: gyro-integrated, pulled towards the accelerometer */\n    float yaw_deg;             /* gyro only: drifts (no magnetometer) */\n}} iris_attitude_t;\n\n/* Sensor frame: x forward, y left, z up, accelerometer in g (reads +1 g on z when level),\n * gyro in deg/s. Check your IMU's axis drawing and remap if it differs. */\nvoid iris_fusion_update(iris_attitude_t *a, float ax, float ay, float az, float gx, float gy, float gz, float dt_s);\n\n{CPP_CLOSE}#endif /* IRIS_FUSION_H */\n"
    )
}

pub fn fusion_c() -> String {
    r#"/* iris_fusion.c — x_hat <- a (x_hat + gyro dt) + (1 - a) x_accel (generated by Protoforge). */
#include <math.h>
#include "iris_config.h"
#include "iris_fusion.h"

#define IRIS_RAD2DEG 57.29578f

void iris_fusion_update(iris_attitude_t *a, float ax, float ay, float az, float gx, float gy, float gz, float dt_s) {
    const float k = IRIS_FUSION_ALPHA;
    const float acc_roll = atan2f(ay, az) * IRIS_RAD2DEG;
    const float acc_pitch = atan2f(-ax, sqrtf(ay * ay + az * az)) * IRIS_RAD2DEG;
    a->roll_deg = k * (a->roll_deg + gx * dt_s) + (1.0f - k) * acc_roll;
    a->pitch_deg = k * (a->pitch_deg + gy * dt_s) + (1.0f - k) * acc_pitch;
    a->yaw_deg += gz * dt_s;
    if (a->yaw_deg > 180.0f) a->yaw_deg -= 360.0f;
    if (a->yaw_deg < -180.0f) a->yaw_deg += 360.0f;
}
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chip::fsm::{gesture_fsm, scripted, simulate, Guard};

    #[test]
    fn q15_conversion_rounds_and_saturates() {
        assert_eq!(
            q15(&[0.5, -0.5, 1.0, -1.0, 0.0, 1.0 / 32768.0, 0.49998]),
            vec![16384, -16384, 32767, -32768, 0, 1, 16383]
        );
        assert_eq!(q14(&[-1.9445, 1.0]), vec![-31859, 16384]);
    }

    /// Parse the emitted `switch` back into a transition table (state → [(guard, next)]).
    fn parse_c_fsm(c: &str) -> Vec<(String, Vec<(Guard, String)>)> {
        let mut out: Vec<(String, Vec<(Guard, String)>)> = Vec::new();
        for line in c.lines().map(str::trim) {
            if let Some(rest) = line.strip_prefix("case IRIS_G_") {
                out.push((rest.trim_end_matches(':').to_string(), vec![]));
            } else if let Some(rest) = line.strip_prefix("if (").filter(|r| r.contains(") { next = IRIS_G_")) {
                let (cond, tail) = rest.split_once(") { next = IRIS_G_").unwrap();
                let to = tail.split(';').next().unwrap().to_string();
                let mut g = Guard::default();
                for term in cond.split(" && ") {
                    match term {
                        "touch" => g.touch = Some(true),
                        "!touch" => g.touch = Some(false),
                        "true" => {}
                        _ if term.starts_with("t >= ") => g.timer_ge = Some(term[5..].trim_end_matches('u').parse().unwrap()),
                        _ if term.starts_with("t < ") => g.timer_lt = Some(term[4..].trim_end_matches('u').parse().unwrap()),
                        _ => panic!("unexpected term {term}"),
                    }
                }
                out.last_mut().unwrap().1.push((g, to));
            }
        }
        out
    }

    /// Execute the parsed C table with the C semantics (first match, reset/saturate timer).
    fn run_c(table: &[(String, Vec<(Guard, String)>)], tmax: u32, trace: &[bool]) -> Vec<u8> {
        let idx = |n: &str| table.iter().position(|(s, _)| s == n).unwrap();
        let (mut s, mut t) = (0usize, 0u32);
        let mut out = Vec::new();
        for &touch in trace {
            let mut next = s;
            for (g, to) in &table[s].1 {
                if g.matches(touch, t) {
                    next = idx(to);
                    break;
                }
            }
            if next != s {
                s = next;
                t = 0;
            } else if t < tmax {
                t += 1;
            }
            out.push(s as u8);
        }
        out
    }

    #[test]
    fn generated_c_fsm_matches_the_chip_lab_on_traces() {
        for (tap, long, gap) in [(250, 600, 250), (180, 450, 300)] {
            let f = gesture_fsm(tap, long, gap);
            let c = gesture_c(&f);
            let table = parse_c_fsm(&c);
            assert_eq!(table.len(), f.states.len());
            assert_eq!(table.iter().map(|(_, v)| v.len()).sum::<usize>(), f.transitions.len());
            let tmax = (1u32 << f.timer_bits()) - 1;
            let mut rng = crate::math::Rng::new(7);
            let mut traces: Vec<Vec<bool>> = ["tap", "double_tap", "long_press", "slow_press", "mixed"]
                .iter()
                .map(|n| scripted(n))
                .collect();
            for _ in 0..20 {
                let mut tr = Vec::new();
                for _ in 0..12 {
                    let on = rng.uniform() < 0.5;
                    let n = 20 + rng.below(1400);
                    tr.extend(std::iter::repeat_n(on, n));
                }
                traces.push(tr);
            }
            for tr in &traces {
                let reference = simulate(&f, tr, 0, 0).states;
                assert_eq!(run_c(&table, tmax, tr), reference);
            }
            assert!(c.contains(&format!("t >= {long}u")));
        }
    }

    #[test]
    fn biquad_q14_tracks_the_float_filter() {
        let p = Project::iris();
        let i = Inputs::from_project(&p, &super::super::ble::LinkParams::default());
        let x: Vec<i16> = (0..4000)
            .map(|n| (6000.0 * (2.0 * std::f64::consts::PI * 1000.0 * n as f64 / 16000.0).sin()) as i16)
            .collect();
        let yq = biquad_q14(&i.hpf_q14, &x);
        let xf: Vec<f64> = x.iter().map(|v| *v as f64).collect();
        // (a) against the float filter with the *quantised* coefficients: only the output
        // rounding differs. Each rounding error (≤ ½ LSB) re-enters through the feedback, so the
        // bound is ½ · Σ|h_e[n]| with h_e the impulse response of 1 / A(z) — large for a
        // 100 Hz high-pass at 16 kHz because its poles sit close to z = 1.
        let c: Vec<f64> = i.hpf_q14.iter().map(|&v| v as f64 / 16384.0).collect();
        let bq = crate::dsp::iir::Biquad {
            b: [c[0], c[1], c[2]],
            a: [1.0, c[3], c[4]],
        };
        let yqf = crate::dsp::filter::sos_filter(&[bq], &xf);
        let err_round = yq
            .iter()
            .zip(&yqf)
            .skip(400)
            .map(|(a, b)| (*a as f64 - b).abs())
            .fold(0.0, f64::max);
        let (mut h1, mut h2, mut sum) = (0.0f64, 0.0f64, 0.0f64);
        for n in 0..50_000 {
            let h = if n == 0 { 1.0 } else { 0.0 } - c[3] * h1 - c[4] * h2;
            sum += h.abs();
            h2 = h1;
            h1 = h;
        }
        let bound = 0.5 * sum;
        assert!(err_round <= bound, "rounding error {err_round} LSB > bound {bound}");
        // the worst-case bound is loose (≈ 360 LSB); in practice the errors are random and small
        assert!(err_round < 30.0, "rounding error {err_round} LSB");
        // (b) against the ideal design: Q14 coefficient steps of 2^-14 move the poles slightly;
        // at 1 kHz the gain error stays well under 0.5 % of a 6000-count tone
        let yf = crate::dsp::filter::sos_filter(&[i.hpf], &xf);
        let err = yq.iter().zip(&yf).skip(400).map(|(a, b)| (*a as f64 - b).abs()).fold(0.0, f64::max);
        assert!(err < 30.0, "max |Q14 − float| = {err}");
        // a 1 kHz tone passes a 100 Hz high-pass nearly untouched
        let peak = yq.iter().skip(400).map(|v| v.unsigned_abs()).max().unwrap();
        assert!(peak > 5800 && peak < 6200, "{peak}");
    }

    #[test]
    fn policy_table_covers_every_state() {
        let p = Project::iris();
        let i = Inputs::from_project(&p, &super::super::ble::LinkParams::default());
        assert_eq!(i.policy.len(), i.soc_buckets * 4);
        let c = policy_c(&i);
        assert_eq!(c.matches("/* SoC ").count(), i.soc_buckets);
        assert!(i.ocv_mv.windows(2).all(|w| w[1] >= w[0]));
        assert_eq!(*i.ocv_mv.last().unwrap(), 4200);
    }

    use crate::project::Project;
}
