//! Heads-up display (HUD) frame budget: how long the SPI link needs per frame,
//! how much a partial (dirty-rectangle) update saves, what the text sizes are
//! in arcminutes at the Optics Lab's angular resolution, and what each screen
//! costs in power.
//!
//! * **Transfer time.** A W × H frame at b bits per pixel is W·H·b/8 bytes;
//!   over SPI at f_SCK it takes t = 8·bytes / f_SCK plus a fixed per-transfer
//!   overhead (command bytes, chip-select, DMA set-up). The nRF52840's SPIM3
//!   instance runs up to 32 MHz, the other SPIM instances up to 8 MHz
//!   (Nordic, *nRF52840 Product Specification*).
//! * **Partial updates.** Display controllers accept a column/row address
//!   window, so only the bounding rectangle of changed pixels has to be sent.
//!   The dirty rectangle is measured here by rendering two consecutive frames
//!   of each screen with the firmware's own renderer and diffing them.
//! * **Panel power.** An OLED emits only where pixels are lit, so its power is
//!   modelled as P = P_static + P_emit·APL, with APL (average picture level)
//!   the lit fraction measured from the framebuffer. Calibration (an
//!   assumption to replace with the panel datasheet): the Iris BOM's display
//!   power corresponds to `apl_ref` lit and `static_frac` of it is fixed.
//! * **Link power.** While a transfer runs the SoC is assumed fully active
//!   (BOM active power) — an upper bound, because EasyDMA lets the CPU sleep.
//!
//! References: Nordic Semiconductor, nRF52840 Product Specification (SPIM);
//! Solomon Systech SSD1306 datasheet (address windows); E. White, *Making
//! Embedded Systems*, 2nd ed., O'Reilly 2024 (display update strategies).

use serde::{Deserialize, Serialize};

use super::font::FONT_5X7;
use super::gfx::Fb;
use super::hud::{self, HudState, Screen};
use crate::project::Project;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayParams {
    /// 0 = take the panel size from the project's optics.
    pub panel_w: u32,
    pub panel_h: u32,
    /// Bits per pixel sent over the link (1 = the monochrome framebuffer as is).
    pub link_bpp: u32,
    pub spi_mhz: f64,
    pub overhead_us: f64,
    pub fps_full: f64,
    /// Send only the dirty rectangle (else the whole frame every period).
    pub partial: bool,
    /// Fraction of lit pixels the BOM display power refers to.
    pub apl_ref: f64,
    /// Share of the BOM display power that does not depend on content.
    pub static_frac: f64,
}

impl Default for DisplayParams {
    fn default() -> Self {
        DisplayParams {
            panel_w: 0,
            panel_h: 0,
            link_bpp: 1,
            spi_mhz: 8.0,
            overhead_us: 50.0,
            fps_full: 10.0,
            partial: true,
            apl_ref: 0.25,
            static_frac: 0.3,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TextSize {
    pub scale: u32,
    pub cap_px: u32,
    pub cap_arcmin: f64,
    pub verdict: String,
    pub used_by: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScreenBudget {
    pub screen: Screen,
    pub title: String,
    /// Content updates per second (what changes on this screen).
    pub update_hz: f64,
    pub update: String,
    pub lit_px: usize,
    pub apl_pct: f64,
    /// Dirty rectangle between two consecutive frames: x, y, w, h (px).
    pub dirty: [i32; 4],
    pub dirty_bytes: f64,
    pub dirty_ms: f64,
    /// SPI bytes per second with full frames at `fps_full` / with partial updates.
    pub full_bps: f64,
    pub partial_bps: f64,
    /// Fraction of time the SPI link is busy (chosen strategy).
    pub link_duty_pct: f64,
    pub panel_mw: f64,
    pub link_mw: f64,
    pub total_mw: f64,
    /// Same screen if every frame were sent whole.
    pub total_full_mw: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct DisplayBudget {
    pub params: DisplayParams,
    pub panel_w: u32,
    pub panel_h: u32,
    pub font_scale: i32,
    pub frame_bytes: f64,
    pub frame_ms: f64,
    pub max_fps: f64,
    pub arcmin_per_px: f64,
    pub ppd: f64,
    pub text: Vec<TextSize>,
    pub screens: Vec<ScreenBudget>,
    pub bom_display_mw: f64,
    pub soc_active_mw: f64,
    pub sweep_mhz: Vec<f64>,
    pub sweep_max_fps: Vec<f64>,
    pub notes: Vec<String>,
}

/// Cap height of the 5×7 font in font pixels (rows used by 'H').
pub fn cap_rows() -> u32 {
    FONT_5X7[(b'H' - 32) as usize].iter().filter(|r| **r != 0).count() as u32
}

/// Frame transfer time in ms for `bytes` at `mhz` plus overhead.
pub fn transfer_ms(bytes: f64, mhz: f64, overhead_us: f64) -> f64 {
    if bytes <= 0.0 {
        return 0.0;
    }
    (bytes * 8.0 / mhz.max(0.01) + overhead_us) / 1000.0
}

/// Bounding box of pixels that differ between two framebuffers: [x, y, w, h] (all 0 when equal).
pub fn dirty_rect(a: &Fb, b: &Fb) -> [i32; 4] {
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, -1, -1);
    for y in 0..a.h {
        let row = (y * a.stride) as usize..((y + 1) * a.stride) as usize;
        if a.buf[row.clone()] == b.buf[row] {
            continue;
        }
        for x in 0..a.w {
            if a.get(x, y) != b.get(x, y) {
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
    }
    if x1 < 0 {
        [0, 0, 0, 0]
    } else {
        [x0, y0, x1 - x0 + 1, y1 - y0 + 1]
    }
}

/// The next content state of a screen and how often it happens.
fn advance(st: &HudState, fs: i32, fps: f64) -> (HudState, f64, &'static str) {
    let mut n = st.clone();
    match st.screen {
        Screen::Clock => {
            n.minute = (st.minute + 1) % 60;
            (n, 1.0 / 60.0, "minute digits once a minute")
        }
        Screen::Navigation => {
            n.nav_distance_m = st.nav_distance_m.saturating_sub(1);
            (n, 1.0, "distance once a second while walking")
        }
        Screen::Captions => {
            n.captions = format!("{} Next", st.captions);
            (n, 2.5, "a new word ≈ 2.5 times a second (150 words/min)")
        }
        Screen::Notification => (n, 0.0, "static until the next notification"),
        Screen::Teleprompter => {
            n.prompter_scroll_px = st.prompter_scroll_px + fs;
            (n, fps, "smooth scroll, every frame")
        }
    }
}

fn component_mw(p: &Project, refdes: &str, key: &str, fallback: f64) -> f64 {
    p.components
        .iter()
        .find(|c| c.refdes == refdes)
        .or_else(|| p.components.iter().find(|c| c.name.to_lowercase().contains(key)))
        .map(|c| c.power.active_mw)
        .filter(|v| *v > 0.0)
        .unwrap_or(fallback)
}

pub fn budget(p: &Project, params: &DisplayParams, base: &HudState) -> DisplayBudget {
    let mut dp = params.clone();
    dp.link_bpp = match dp.link_bpp {
        0..=1 => 1,
        2..=4 => 4,
        5..=8 => 8,
        _ => 16,
    };
    dp.spi_mhz = dp.spi_mhz.clamp(0.125, 64.0);
    dp.fps_full = dp.fps_full.clamp(0.1, 120.0);
    dp.apl_ref = dp.apl_ref.clamp(0.01, 1.0);
    dp.static_frac = dp.static_frac.clamp(0.0, 1.0);
    let w = if dp.panel_w > 0 { dp.panel_w } else { p.design.optics.px_w.max(16) };
    let h = if dp.panel_h > 0 { dp.panel_h } else { p.design.optics.px_h.max(16) };
    let fs = hud::font_scale(w as i32, h as i32);
    let bpp = dp.link_bpp as f64;
    let frame_bytes = w as f64 * h as f64 * bpp / 8.0;
    let frame_ms = transfer_ms(frame_bytes, dp.spi_mhz, dp.overhead_us);
    let a = crate::optics::paraxial::analyze(&p.design.optics, &crate::optics::paraxial::Extras::default());
    // arcminutes per pixel of *this* panel: scale the Optics Lab's figure if the panel differs
    let apx = a.arcmin_per_px * p.design.optics.px_w.max(1) as f64 / w as f64;
    let caps = cap_rows();
    let mut sizes: Vec<(u32, Vec<String>)> = vec![];
    for s in Screen::ALL {
        for wg in hud::widgets(s) {
            if wg.kind == hud::W_TEXT || wg.kind == hud::W_WRAP {
                let k = wg.size as u32;
                match sizes.iter_mut().find(|(z, _)| *z == k) {
                    Some((_, v)) => {
                        if !v.contains(&s.title().to_string()) {
                            v.push(s.title().into())
                        }
                    }
                    None => sizes.push((k, vec![s.title().into()])),
                }
            }
        }
    }
    sizes.sort_by_key(|(k, _)| *k);
    let text = sizes
        .into_iter()
        .map(|(k, used)| {
            let cap_px = caps * fs as u32 * k;
            let am = cap_px as f64 * apx;
            TextSize {
                scale: k,
                cap_px,
                cap_arcmin: am,
                verdict: crate::optics::design::legibility_verdict(am).into(),
                used_by: used,
            }
        })
        .collect();
    let bom_mw = component_mw(p, "U2", "display", 140.0);
    let soc_mw = component_mw(p, "U1", "system-on-chip", 21.0);
    let static_mw = bom_mw * dp.static_frac;
    let emit_full_mw = bom_mw * (1.0 - dp.static_frac) / dp.apl_ref;
    let link_mw = |bytes_per_s: f64, transfers_per_s: f64| {
        let busy_s = bytes_per_s * 8.0 / (dp.spi_mhz * 1e6) + transfers_per_s * dp.overhead_us * 1e-6;
        (busy_s.min(1.0) * soc_mw, busy_s.min(1.0) * 100.0)
    };
    let mut screens = Vec::new();
    for s in Screen::ALL {
        let st = HudState { screen: s, ..base.clone() };
        let mut fa = Fb::new(w as i32, h as i32, 1);
        hud::render(&mut fa, &st);
        let (next, rate, what) = advance(&st, fs, dp.fps_full);
        let mut fb = Fb::new(w as i32, h as i32, 1);
        hud::render(&mut fb, &next);
        let d = dirty_rect(&fa, &fb);
        let dirty_bytes = d[2] as f64 * d[3] as f64 * bpp / 8.0;
        let lit = fa.lit();
        let apl = lit as f64 / (w as f64 * h as f64);
        let panel = static_mw + emit_full_mw * apl;
        let full_bps = frame_bytes * dp.fps_full;
        let partial_bps = dirty_bytes * rate.min(dp.fps_full);
        let (lm_full, duty_full) = link_mw(full_bps, dp.fps_full);
        let (lm_part, duty_part) = link_mw(partial_bps, if dirty_bytes > 0.0 { rate.min(dp.fps_full) } else { 0.0 });
        let (lm, duty) = if dp.partial { (lm_part, duty_part) } else { (lm_full, duty_full) };
        screens.push(ScreenBudget {
            screen: s,
            title: s.title().into(),
            update_hz: rate,
            update: what.into(),
            lit_px: lit,
            apl_pct: apl * 100.0,
            dirty: d,
            dirty_bytes,
            dirty_ms: transfer_ms(dirty_bytes, dp.spi_mhz, dp.overhead_us),
            full_bps,
            partial_bps,
            link_duty_pct: duty,
            panel_mw: panel,
            link_mw: lm,
            total_mw: panel + lm,
            total_full_mw: panel + lm_full,
        });
    }
    let sweep_mhz = vec![1.0, 2.0, 4.0, 8.0, 12.0, 16.0, 24.0, 32.0];
    let sweep_max_fps = sweep_mhz
        .iter()
        .map(|m| 1000.0 / transfer_ms(frame_bytes, *m, dp.overhead_us))
        .collect();
    let mut notes = vec![
        format!(
            "full frame {} × {} × {} bit = {:.0} B → {:.2} ms at {:.1} MHz (+{:.0} µs overhead) → at most {:.0} frames/s",
            w,
            h,
            dp.link_bpp,
            frame_bytes,
            frame_ms,
            dp.spi_mhz,
            dp.overhead_us,
            1000.0 / frame_ms
        ),
        format!(
            "panel power model: {static_mw:.0} mW fixed + {emit_full_mw:.0} mW × lit fraction (calibrated so the BOM's {bom_mw:.0} mW = {:.0} % lit — an assumption; use the panel datasheet)",
            dp.apl_ref * 100.0
        ),
        format!("link power: SoC active ({soc_mw:.0} mW, BOM) while the SPI transfer runs — an upper bound; with EasyDMA the CPU can sleep"),
    ];
    if dp.spi_mhz > 8.0 {
        notes.push(
            "above 8 MHz use the nRF52840's SPIM3 instance (up to 32 MHz); keep the flex cable short and check signal integrity".into(),
        );
    }
    if 1000.0 / frame_ms < dp.fps_full {
        notes.push(format!(
            "full frames cannot reach {:.0} fps at this clock — use partial updates or a faster SPI clock",
            dp.fps_full
        ));
    }
    DisplayBudget {
        params: dp,
        panel_w: w,
        panel_h: h,
        font_scale: fs,
        frame_bytes,
        frame_ms,
        max_fps: 1000.0 / frame_ms,
        arcmin_per_px: apx,
        // the Optics Lab's average px/deg, scaled to this panel's width
        ppd: a.ppd * w as f64 / p.design.optics.px_w.max(1) as f64,
        text,
        screens,
        bom_display_mw: bom_mw,
        soc_active_mw: soc_mw,
        sweep_mhz,
        sweep_max_fps,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_time_by_hand() {
        let p = Project::iris();
        let b = budget(&p, &DisplayParams::default(), &HudState::default());
        // 640 × 400 × 1 bit = 32 000 B; 256 000 bit / 8 MHz = 32 ms (+ 50 µs)
        assert_eq!((b.panel_w, b.panel_h, b.font_scale), (640, 400, 5));
        assert!((b.frame_bytes - 32000.0).abs() < 1e-9);
        assert!((b.frame_ms - 32.05).abs() < 1e-9);
        assert!((b.max_fps - 1000.0 / 32.05).abs() < 1e-9);
        // 8 bpp at 32 MHz: 256 000 B → 64 ms
        let b8 = budget(
            &p,
            &DisplayParams {
                link_bpp: 8,
                spi_mhz: 32.0,
                overhead_us: 0.0,
                ..Default::default()
            },
            &HudState::default(),
        );
        assert!((b8.frame_ms - 64.0).abs() < 1e-9);
    }

    #[test]
    fn text_size_in_arcminutes_uses_the_optics_lab() {
        let p = Project::iris();
        let b = budget(&p, &DisplayParams::default(), &HudState::default());
        assert_eq!(cap_rows(), 6);
        let a = crate::optics::paraxial::analyze(&p.design.optics, &Default::default());
        let t1 = &b.text[0];
        assert_eq!(t1.scale, 1);
        assert_eq!(t1.cap_px, 6 * 5);
        assert!((t1.cap_arcmin - 30.0 * a.arcmin_per_px).abs() < 1e-9);
        // Iris ≈ 27 px/deg → ≈ 2.2 arcmin/px → body text ≈ 66 arcmin: large and legible
        assert!(t1.cap_arcmin > 16.0, "{}", t1.cap_arcmin);
        assert!(b
            .text
            .iter()
            .any(|t| t.scale == 3 && t.used_by.contains(&"Clock & battery".to_string())));
    }

    #[test]
    fn partial_updates_send_less_and_static_screens_nothing() {
        let p = Project::iris();
        let b = budget(&p, &DisplayParams::default(), &HudState::default());
        let get = |s: Screen| b.screens.iter().find(|x| x.screen == s).unwrap();
        let n = get(Screen::Notification);
        assert_eq!(n.dirty, [0, 0, 0, 0]);
        assert_eq!(n.partial_bps, 0.0);
        let c = get(Screen::Clock);
        assert!(c.dirty[2] > 0 && c.dirty_bytes < b.frame_bytes / 4.0, "{:?}", c.dirty);
        for s in &b.screens {
            assert!(s.partial_bps <= s.full_bps + 1e-9);
            assert!(s.total_mw <= s.total_full_mw + 1e-9);
            // panel model: static + emit × APL
            let stat = b.bom_display_mw * 0.3;
            let emit = b.bom_display_mw * 0.7 / 0.25;
            assert!((s.panel_mw - (stat + emit * s.apl_pct / 100.0)).abs() < 1e-9);
            assert!(s.apl_pct > 0.0 && s.apl_pct < 40.0, "{} {}", s.title, s.apl_pct);
        }
        // naive full-frame refresh at 10 fps keeps the link busy 32 % of the time
        let full_duty = 10.0 * 32.05 / 1000.0;
        let naive = budget(
            &p,
            &DisplayParams {
                partial: false,
                ..Default::default()
            },
            &HudState::default(),
        );
        assert!((naive.screens[0].link_duty_pct - full_duty * 100.0).abs() < 1e-6);
    }

    #[test]
    fn dirty_rect_of_one_pixel() {
        let mut a = Fb::new(64, 32, 1);
        let b = a.clone();
        a.pixel(10, 20, 15);
        assert_eq!(dirty_rect(&a, &b), [10, 20, 1, 1]);
    }
}
