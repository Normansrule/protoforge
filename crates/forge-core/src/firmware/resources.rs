//! Memory budget of the production firmware on the nRF52840 (1 MB flash,
//! 256 KB RAM — Nordic Semiconductor, nRF52840 Product Specification).
//!
//! RAM is added up from the buffers the firmware really allocates (the
//! framebuffer, audio blocks, sensor batches, thread stacks, message queues)
//! plus stated allowances for the Bluetooth stack and the kernel. Flash is
//! the Bluetooth stack and kernel allowances, an application-code estimate
//! from the generated C (statements × bytes/statement — a heuristic, check
//! it with `west build -t rom_report`), and the exact size of every
//! constant table the generator emits. Every line says whether it is
//! *computed* from the design or an *assumption* to replace with a
//! measurement.

use serde::{Deserialize, Serialize};

use super::hud::{self, Screen};
use super::modules::Inputs;

pub const RAM_CAPACITY_KB: f64 = 256.0;
pub const FLASH_CAPACITY_KB: f64 = 1024.0;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ResourceParams {
    pub panel_w: u32,
    pub panel_h: u32,
    /// Framebuffer bits per pixel: 1 or 4.
    pub fb_bpp: u32,
    pub double_buffer: bool,
    pub mics: u32,
    /// DMIC memory-slab blocks (20 ms each).
    pub audio_blocks: u32,
    /// Read the IMU FIFO this often (ms).
    pub imu_batch_ms: f64,
    /// Assumption: Bluetooth host + controller RAM with 251-byte buffers.
    pub ble_ram_kb: f64,
    /// Assumption: kernel objects, driver data, C library.
    pub kernel_ram_kb: f64,
    pub heap_kb: f64,
    /// Assumption: Bluetooth host + controller code.
    pub ble_flash_kb: f64,
    /// Assumption: kernel + drivers code.
    pub kernel_flash_kb: f64,
    /// Assumption: log strings and C library.
    pub log_flash_kb: f64,
    /// Heuristic bytes of Thumb-2 code per C statement.
    pub bytes_per_statement: f64,
    /// Reserve a bootloader and a second image slot for over-the-air updates.
    pub dfu: bool,
    pub bootloader_kb: f64,
    pub storage_kb: f64,
    /// Thread stacks (name, bytes).
    pub stacks: Vec<(String, u32)>,
    /// CPU clock (the nRF52840's Cortex-M4F runs at 64 MHz).
    pub cpu_mhz: f64,
    /// Cycle-count assumptions per event (estimates from instruction counts; measure with the DWT cycle counter).
    pub gesture_isr_cycles: f64,
    pub biquad_cycles_per_sample: f64,
    pub adpcm_cycles_per_sample: f64,
    pub imu_cycles_per_sample: f64,
    pub render_cycles_per_lit_px: f64,
    pub spi_setup_cycles: f64,
    pub ble_event_cycles: f64,
    pub ble_notify_cycles: f64,
    pub ui_fps: f64,
    pub conn_interval_ms: f64,
}

impl Default for ResourceParams {
    fn default() -> Self {
        ResourceParams {
            panel_w: 640,
            panel_h: 400,
            fb_bpp: 1,
            double_buffer: false,
            mics: 2,
            audio_blocks: 4,
            imu_batch_ms: 100.0,
            ble_ram_kb: 24.0,
            kernel_ram_kb: 6.0,
            heap_kb: 8.0,
            ble_flash_kb: 150.0,
            kernel_flash_kb: 40.0,
            log_flash_kb: 8.0,
            bytes_per_statement: 10.0,
            dfu: true,
            bootloader_kb: 48.0,
            storage_kb: 32.0,
            stacks: vec![
                ("sensor thread".into(), 2048),
                ("audio thread".into(), 3072),
                ("ui thread".into(), 3072),
                ("power thread".into(), 1536),
                ("main".into(), 2048),
                ("system work queue".into(), 2048),
                ("interrupt stack".into(), 2048),
                ("idle + logging".into(), 1088),
            ],
            cpu_mhz: 64.0,
            gesture_isr_cycles: 150.0,
            biquad_cycles_per_sample: 30.0,
            adpcm_cycles_per_sample: 40.0,
            imu_cycles_per_sample: 2400.0,
            render_cycles_per_lit_px: 12.0,
            spi_setup_cycles: 2000.0,
            ble_event_cycles: 3000.0,
            ble_notify_cycles: 1500.0,
            ui_fps: 10.0,
            conn_interval_ms: 30.0,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Item {
    pub name: String,
    pub group: String,
    pub bytes: f64,
    /// "computed" or "assumption".
    pub basis: String,
    pub note: String,
}

fn item(name: &str, group: &str, bytes: f64, basis: &str, note: String) -> Item {
    Item {
        name: name.into(),
        group: group.into(),
        bytes,
        basis: basis.into(),
        note,
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ThreadInfo {
    pub name: String,
    pub priority: i32,
    pub stack: u32,
    pub rate: String,
    pub work: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CpuTask {
    pub name: String,
    pub rate_hz: f64,
    pub cycles: f64,
    pub mcycles_per_s: f64,
    pub load_pct: f64,
    pub basis: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Resources {
    pub params: ResourceParams,
    pub ram: Vec<Item>,
    pub flash: Vec<Item>,
    pub ram_kb: f64,
    pub flash_kb: f64,
    pub ram_capacity_kb: f64,
    pub flash_capacity_kb: f64,
    /// Flash available to one application image (half of what is left after the bootloader and storage with DFU).
    pub flash_budget_kb: f64,
    pub ram_pct: f64,
    pub flash_pct_of_budget: f64,
    pub recommended_ram_kb: f64,
    pub threads: Vec<ThreadInfo>,
    /// Worst-case context (live captions: audio streaming + full-rate display).
    pub cpu: Vec<CpuTask>,
    pub cpu_load_pct: f64,
    pub hud_lit_px: usize,
    pub notes: Vec<String>,
}

/// Count C statements (semicolons outside comments/strings, excluding `for` headers).
pub fn count_statements(src: &str) -> usize {
    let b = src.as_bytes();
    let (mut i, mut n) = (0, 0);
    while i < b.len() {
        if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
            continue;
        }
        if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'/' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b[i] == b'"' || b[i] == b'\'' {
            let q = b[i];
            i += 1;
            while i < b.len() && b[i] != q {
                if b[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        if b[i] == b';' {
            n += 1;
        }
        i += 1;
    }
    n
}

/// CPU load per task: rate × cycles / f_CPU. Rendering cost is measured from
/// the real framebuffer: a clear (bytes / 4 word stores × 2 cycles) plus
/// `render_cycles_per_lit_px` for every lit pixel of the captions screen.
pub fn cpu_tasks(inp: &Inputs, p: &ResourceParams) -> (Vec<CpuTask>, usize) {
    let mut fb = super::gfx::Fb::new(p.panel_w as i32, p.panel_h as i32, if p.fb_bpp >= 4 { 4 } else { 1 });
    let st = hud::HudState {
        screen: Screen::Captions,
        ..Default::default()
    };
    hud::render(&mut fb, &st);
    let lit = fb.lit();
    let f = p.cpu_mhz.max(1.0) * 1e6;
    let fs = inp.audio_rate_hz;
    let events = 1000.0 / p.conn_interval_ms.max(7.5);
    let t = |name: &str, rate: f64, cycles: f64, basis: String| CpuTask {
        name: name.into(),
        rate_hz: rate,
        cycles,
        mcycles_per_s: rate * cycles / 1e6,
        load_pct: rate * cycles / f * 100.0,
        basis,
    };
    let render = fb.buf.len() as f64 / 4.0 * 2.0 + lit as f64 * p.render_cycles_per_lit_px;
    let tasks = vec![
        t(
            "Gesture timer ISR",
            1000.0,
            p.gesture_isr_cycles,
            "1 kHz: read pin, step FSM, maybe post event".into(),
        ),
        t(
            "Voice high-pass (biquad Q14)",
            fs,
            p.biquad_cycles_per_sample,
            format!("{fs:.0} samples/s × 5 multiply-accumulates"),
        ),
        t(
            "IMA ADPCM encoder",
            fs,
            p.adpcm_cycles_per_sample,
            "per sample: compare/subtract ladder + table updates".into(),
        ),
        t(
            "PDM decimation (CIC + FIR)",
            fs,
            0.0,
            "offloaded: the nRF52840 PDM peripheral decimates in hardware (IRIS-DSP on the custom board)".into(),
        ),
        t(
            "IMU read + complementary filter",
            inp.imu_rate_hz,
            p.imu_cycles_per_sample,
            "sensor API fetch + 2 atan2f + sqrtf".into(),
        ),
        t(
            "HUD render",
            p.ui_fps,
            render,
            format!(
                "clear {} B + {} lit px × {:.0} cycles (captions screen)",
                fb.buf.len(),
                lit,
                p.render_cycles_per_lit_px
            ),
        ),
        t(
            "Display SPI transfer set-up",
            p.ui_fps,
            p.spi_setup_cycles,
            "EasyDMA moves the bytes; CPU only starts it".into(),
        ),
        t(
            "BLE connection events",
            events,
            p.ble_event_cycles,
            format!("{:.1} ms interval: host + controller processing", p.conn_interval_ms),
        ),
        t(
            "BLE audio notifications",
            1000.0 / 20.0,
            p.ble_notify_cycles,
            "one 20 ms ADPCM frame per notification".into(),
        ),
        t(
            "Power policy",
            1.0 / (inp.step_min * 60.0).max(1.0),
            5000.0,
            "ADC read + table lookup".into(),
        ),
    ];
    (tasks, lit)
}

pub fn estimate(inp: &Inputs, p: &ResourceParams, app_sources: &[String]) -> Resources {
    let mut p = p.clone();
    p.fb_bpp = if p.fb_bpp >= 4 { 4 } else { 1 };
    p.panel_w = p.panel_w.clamp(16, 4096);
    p.panel_h = p.panel_h.clamp(16, 4096);
    let mut ram = Vec::new();
    let fb = (p.panel_w * p.panel_h * p.fb_bpp).div_ceil(8) as f64 * if p.double_buffer { 2.0 } else { 1.0 };
    ram.push(item(
        "HUD framebuffer",
        "display",
        fb,
        "computed",
        format!(
            "{} × {} px × {} bpp / 8{}",
            p.panel_w,
            p.panel_h,
            p.fb_bpp,
            if p.double_buffer { " × 2 (double-buffered)" } else { "" }
        ),
    ));
    let frame = (inp.audio_rate_hz / 50.0).round();
    let audio = p.audio_blocks as f64 * frame * 2.0 * p.mics.max(1) as f64;
    ram.push(item(
        "Audio blocks (DMIC slab)",
        "audio",
        audio,
        "computed",
        format!(
            "{} blocks × {} samples (20 ms at {:.0} Hz) × 2 B × {} mic(s)",
            p.audio_blocks,
            frame,
            inp.audio_rate_hz,
            p.mics.max(1)
        ),
    ));
    let adpcm_pkt = 6.0 + frame / 2.0;
    ram.push(item(
        "ADPCM packet + filter state",
        "audio",
        adpcm_pkt + 8.0 + 3.0,
        "computed",
        format!("6-byte header + {:.0} bytes of 4-bit samples, biquad and ADPCM state", frame / 2.0),
    ));
    let imu = (inp.imu_rate_hz * p.imu_batch_ms / 1000.0).ceil() * 12.0;
    ram.push(item(
        "IMU batch buffer",
        "sensors",
        imu,
        "computed",
        format!(
            "{:.0} Hz × {:.0} ms batches × 12 B (3-axis accel + gyro, 16-bit)",
            inp.imu_rate_hz, p.imu_batch_ms
        ),
    ));
    let hud_state = (1 + 1 + 1 + 1 + 1 + 3 + 4 + 4)
        + hud::CAP_STREET
        + 4
        + hud::CAP_CAPTIONS
        + 4
        + hud::CAP_APP
        + hud::CAP_TITLE
        + hud::CAP_BODY
        + hud::CAP_PROMPTER
        + 4
        + 4;
    ram.push(item(
        "HUD state + text scratch",
        "display",
        (hud_state + 512) as f64,
        "computed",
        format!("iris_hud_state_t ≈ {hud_state} B (text buffers) + 512 B field scratch"),
    ));
    ram.push(item(
        "UI event queue",
        "kernel",
        32.0 * 2.0 + 32.0,
        "computed",
        "32 events × 2 B + queue object".into(),
    ));
    for (name, bytes) in &p.stacks {
        ram.push(item(
            &format!("Stack: {name}"),
            "stacks",
            *bytes as f64,
            "computed",
            "configured stack size".into(),
        ));
    }
    ram.push(item(
        "Heap",
        "kernel",
        p.heap_kb * 1024.0,
        "computed",
        "CONFIG_HEAP_MEM_POOL_SIZE".into(),
    ));
    ram.push(item(
        "Bluetooth host + controller",
        "ble",
        p.ble_ram_kb * 1024.0,
        "assumption",
        "connection context, ATT/L2CAP buffers for 251-byte packets — measure with `west build -t ram_report`".into(),
    ));
    ram.push(item(
        "Kernel objects, drivers, C library",
        "kernel",
        p.kernel_ram_kb * 1024.0,
        "assumption",
        "measure with `west build -t ram_report`".into(),
    ));

    // ---- flash
    let mut flash = Vec::new();
    let statements: usize = app_sources.iter().map(|s| count_statements(s)).sum();
    flash.push(item(
        "Application code",
        "app",
        statements as f64 * p.bytes_per_statement,
        "computed",
        format!(
            "{} C statements in the generated sources × {:.0} B/statement (heuristic for Thumb-2 at -Os)",
            statements, p.bytes_per_statement
        ),
    ));
    let widgets: usize = Screen::ALL.iter().map(|s| hud::widgets(*s).len()).sum();
    let consts = [
        ("Font 5×7 (95 glyphs)", 95.0 * 7.0, "glyph rows"),
        ("Sine table Q14", 91.0 * 2.0, "0–90° in 1° steps"),
        (
            "HUD widget tables",
            widgets as f64 * 14.0 + 5.0 * 6.0,
            "widgets × 14 B + screen index",
        ),
        (
            "Filter coefficients",
            (inp.chain.fir.len() + inp.chain.halfband.len() + 5) as f64 * 2.0,
            "decimation FIR (Q15) + high-pass biquad (Q14)",
        ),
        (
            "Power policy + OCV table",
            (inp.soc_buckets * 4) as f64 + 21.0 * 2.0,
            "bucket × context actions, 21 OCV points",
        ),
        ("ADPCM tables", 89.0 * 2.0 + 16.0, "IMA step and index tables"),
        (
            "GATT table + UUIDs",
            8.0 * 16.0 + 14.0 * 20.0,
            "8 × 128-bit UUIDs + attribute entries (≈20 B each)",
        ),
    ];
    for (n, b, note) in consts {
        flash.push(item(n, "const", b, "computed", note.into()));
    }
    flash.push(item(
        "Bluetooth host + controller",
        "ble",
        p.ble_flash_kb * 1024.0,
        "assumption",
        "LE peripheral stack — measure with `west build -t rom_report`".into(),
    ));
    flash.push(item(
        "Zephyr kernel + drivers",
        "kernel",
        p.kernel_flash_kb * 1024.0,
        "assumption",
        "scheduler, GPIO/I²C/SPI/PDM/ADC drivers, sensor driver".into(),
    ));
    flash.push(item(
        "Log strings, C library",
        "kernel",
        p.log_flash_kb * 1024.0,
        "assumption",
        "CONFIG_LOG and printf support".into(),
    ));

    let ram_b: f64 = ram.iter().map(|x| x.bytes).sum();
    let flash_b: f64 = flash.iter().map(|x| x.bytes).sum();
    let ram_kb = ram_b / 1024.0;
    let flash_kb = flash_b / 1024.0;
    let budget = if p.dfu {
        (FLASH_CAPACITY_KB - p.bootloader_kb - p.storage_kb) / 2.0
    } else {
        FLASH_CAPACITY_KB - p.storage_kb
    };
    let mut notes = vec![format!(
        "RAM {:.1} KB of {:.0} KB ({:.0} %); recommended ceiling 75 % = {:.0} KB leaves room for stack growth and future features",
        ram_kb,
        RAM_CAPACITY_KB,
        ram_kb / RAM_CAPACITY_KB * 100.0,
        0.75 * RAM_CAPACITY_KB
    )];
    notes.push(if p.dfu {
        format!(
            "flash image {:.0} KB; with a {:.0} KB bootloader, {:.0} KB settings storage and two image slots for over-the-air updates, one image may use {:.0} KB",
            flash_kb, p.bootloader_kb, p.storage_kb, budget
        )
    } else {
        format!("flash image {:.0} KB of {:.0} KB (no update slot reserved)", flash_kb, budget)
    });
    if p.fb_bpp == 4 {
        notes.push("a 4 bpp full framebuffer is 4× the 1 bpp one — consider rendering in horizontal bands".into());
    }
    let threads = vec![
        ThreadInfo {
            name: "gesture timer (ISR)".into(),
            priority: -1,
            stack: 0,
            rate: "1 kHz".into(),
            work: "sample touch pin, step gesture FSM, post events".into(),
        },
        ThreadInfo {
            name: "audio".into(),
            priority: 4,
            stack: 3072,
            rate: "50 blocks/s".into(),
            work: "PDM DMA blocks → high-pass → ADPCM → BLE notify".into(),
        },
        ThreadInfo {
            name: "sensor".into(),
            priority: 5,
            stack: 2048,
            rate: format!("{:.0} Hz", inp.imu_rate_hz),
            work: "IMU → complementary filter".into(),
        },
        ThreadInfo {
            name: "ui".into(),
            priority: 6,
            stack: 3072,
            rate: "2–10 fps".into(),
            work: "event queue → HUD render → SPI display".into(),
        },
        ThreadInfo {
            name: "power".into(),
            priority: 8,
            stack: 1536,
            rate: format!("every {:.0} s", inp.step_min * 60.0),
            work: "battery ADC → learned policy → display + radio".into(),
        },
    ];
    let (cpu, hud_lit_px) = cpu_tasks(inp, &p);
    let cpu_load_pct = cpu.iter().map(|t| t.load_pct).sum();
    notes.push(format!(
        "CPU ≈ {cpu_load_pct:.1} % of a {:.0} MHz Cortex-M4F in the busiest context (live captions); cycle counts are estimates — measure with the DWT cycle counter",
        p.cpu_mhz
    ));
    Resources {
        cpu,
        cpu_load_pct,
        hud_lit_px,
        ram_pct: ram_kb / RAM_CAPACITY_KB * 100.0,
        flash_pct_of_budget: flash_kb / budget * 100.0,
        params: p,
        ram,
        flash,
        ram_kb,
        flash_kb,
        ram_capacity_kb: RAM_CAPACITY_KB,
        flash_capacity_kb: FLASH_CAPACITY_KB,
        flash_budget_kb: budget,
        recommended_ram_kb: 0.75 * RAM_CAPACITY_KB,
        threads,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Project;

    #[test]
    fn ram_arithmetic_by_hand() {
        let i = Inputs::from_project(&Project::iris(), &super::super::ble::LinkParams::default());
        let r = estimate(&i, &ResourceParams::default(), &[]);
        let fb = r.ram.iter().find(|x| x.name == "HUD framebuffer").unwrap().bytes;
        assert_eq!(fb, 32000.0); // 640·400/8
        let au = r.ram.iter().find(|x| x.name.starts_with("Audio blocks")).unwrap().bytes;
        assert_eq!(au, 4.0 * 320.0 * 2.0 * 2.0);
        let imu = r.ram.iter().find(|x| x.name == "IMU batch buffer").unwrap().bytes;
        assert_eq!(imu, 20.0 * 12.0);
        let total: f64 = r.ram.iter().map(|x| x.bytes).sum();
        assert!((r.ram_kb - total / 1024.0).abs() < 1e-9);
        let r4 = estimate(
            &i,
            &ResourceParams {
                fb_bpp: 4,
                double_buffer: true,
                ..Default::default()
            },
            &[],
        );
        let fb4 = r4.ram.iter().find(|x| x.name == "HUD framebuffer").unwrap().bytes;
        assert_eq!(fb4, 256000.0);
        assert!(r4.ram_kb > RAM_CAPACITY_KB, "double-buffered 4 bpp cannot fit: {}", r4.ram_kb);
        assert!(r.ram_kb < 0.75 * RAM_CAPACITY_KB, "{}", r.ram_kb);
        assert!((r.flash_budget_kb - (1024.0 - 48.0 - 32.0) / 2.0).abs() < 1e-9);
    }

    #[test]
    fn cpu_load_by_hand() {
        let i = Inputs::from_project(&Project::iris(), &super::super::ble::LinkParams::default());
        let r = estimate(&i, &ResourceParams::default(), &[]);
        let g = r.cpu.iter().find(|t| t.name.starts_with("Gesture")).unwrap();
        // 1000 Hz × 150 cycles / 64 MHz = 0.234 %
        assert!((g.load_pct - 1000.0 * 150.0 / 64e6 * 100.0).abs() < 1e-12);
        let hp = r.cpu.iter().find(|t| t.name.starts_with("Voice")).unwrap();
        assert!((hp.mcycles_per_s - 16000.0 * 30.0 / 1e6).abs() < 1e-12);
        let sum: f64 = r.cpu.iter().map(|t| t.load_pct).sum();
        assert!((r.cpu_load_pct - sum).abs() < 1e-12);
        assert!(
            r.hud_lit_px > 1000 && r.cpu_load_pct > 1.0 && r.cpu_load_pct < 50.0,
            "{} {}",
            r.hud_lit_px,
            r.cpu_load_pct
        );
    }

    #[test]
    fn statement_counter_ignores_comments_and_strings() {
        assert_eq!(count_statements("int a; /* ; ; */ char *s = \";;\"; // ;\n b(';');"), 3);
    }
}
