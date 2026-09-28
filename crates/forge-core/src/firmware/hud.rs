//! The Iris heads-up display (HUD) user interface: a small declarative UI
//! description (screens made of widgets laid out on a 128 × 64 design grid),
//! an interpreter that renders it into a [`Fb`] framebuffer, and the C
//! emitter that ships the **same** widget tables and interpreter to the
//! firmware — so the browser preview is exactly what the glasses draw.
//!
//! Layout: a widget at design coordinates (u, v) lands at device pixel
//! (u·W/128, v·H/64); text and icon strokes scale by the integer factor
//! `fs = max(1, min(W/128, H/64))` (1 on the 128 × 64 prototype OLED, 5 on the
//! 640 × 400 micro-OLED). Word wrap is greedy (break at the last space that
//! fits, hard-break words longer than a line).

use serde::{Deserialize, Serialize};

use super::font::{FONT_5X7, SIN_Q14};
use super::gfx::{cos_q14, q14_round, sin_q14, Fb};

pub const DESIGN_W: i32 = 128;
pub const DESIGN_H: i32 = 64;

// widget kinds
pub const W_TEXT: u8 = 1;
pub const W_WRAP: u8 = 2;
pub const W_RECT: u8 = 3;
pub const W_FILL: u8 = 4;
pub const W_HLINE: u8 = 5;
pub const W_ARROW: u8 = 6;
pub const W_BAR: u8 = 7;
pub const W_BATT: u8 = 8;

// data fields
pub const F_NONE: u8 = 0;
pub const F_TIME: u8 = 1;
pub const F_BATT_PCT: u8 = 2;
pub const F_BLE: u8 = 3;
pub const F_CLOCK_INFO: u8 = 4;
pub const F_NAV_DIST: u8 = 5;
pub const F_NAV_STREET: u8 = 6;
pub const F_NAV_ETA: u8 = 7;
pub const F_CAPTIONS: u8 = 8;
pub const F_NOTIF_APP: u8 = 9;
pub const F_NOTIF_TITLE: u8 = 10;
pub const F_NOTIF_BODY: u8 = 11;
pub const F_PROMPTER: u8 = 12;
pub const F_PROMPTER_PROGRESS: u8 = 13;

const KIND_NAMES: [&str; 9] = ["", "TEXT", "WRAP", "RECT", "FILL", "HLINE", "ARROW", "BAR", "BATT"];
const FIELD_NAMES: [&str; 14] = [
    "NONE",
    "TIME",
    "BATT_PCT",
    "BLE",
    "CLOCK_INFO",
    "NAV_DIST",
    "NAV_STREET",
    "NAV_ETA",
    "CAPTIONS",
    "NOTIF_APP",
    "NOTIF_TITLE",
    "NOTIF_BODY",
    "PROMPTER",
    "PROMPTER_PROGRESS",
];

/// Capacities (bytes incl. the terminating NUL) of the C state strings.
pub const CAP_STREET: usize = 32;
pub const CAP_CAPTIONS: usize = 256;
pub const CAP_APP: usize = 16;
pub const CAP_TITLE: usize = 48;
pub const CAP_BODY: usize = 160;
pub const CAP_PROMPTER: usize = 512;

/// One widget. For TEXT `align` is 0 left / 1 centre / 2 right; for WRAP it is
/// 0 = anchor top (scroll moves up) / 2 = anchor bottom (newest line at the
/// bottom). ARROW: (x, y) is the centre and `w` the radius in `fs` units.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Widget {
    pub kind: u8,
    pub size: u8,
    pub level: u8,
    pub align: u8,
    pub field: u8,
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub text: &'static str,
}

#[allow(clippy::too_many_arguments)]
const fn wd(kind: u8, size: u8, level: u8, align: u8, field: u8, x: i16, y: i16, w: i16, h: i16) -> Widget {
    Widget {
        kind,
        size,
        level,
        align,
        field,
        x,
        y,
        w,
        h,
        text: "",
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Screen {
    #[default]
    Clock,
    Navigation,
    Captions,
    Notification,
    Teleprompter,
}

impl Screen {
    pub const ALL: [Screen; 5] = [
        Screen::Clock,
        Screen::Navigation,
        Screen::Captions,
        Screen::Notification,
        Screen::Teleprompter,
    ];
    pub fn index(self) -> usize {
        Screen::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }
    pub fn c_name(self) -> &'static str {
        ["CLOCK", "NAV", "CAPTIONS", "NOTIF", "PROMPTER"][self.index()]
    }
    pub fn title(self) -> &'static str {
        ["Clock & battery", "Navigation", "Live captions", "Notification", "Teleprompter"][self.index()]
    }
    /// Power-policy context this screen implies (index into `rl::power::CONTEXTS`:
    /// idle, glance, navigate, caption).
    pub fn context(self) -> usize {
        [0, 2, 3, 1, 3][self.index()]
    }
}

const STATUS: [Widget; 4] = [
    wd(W_TEXT, 1, 9, 0, F_TIME, 1, 1, 0, 0),
    wd(W_TEXT, 1, 9, 0, F_BLE, 92, 1, 0, 0),
    wd(W_BATT, 1, 12, 0, F_BATT_PCT, 110, 1, 0, 0),
    wd(W_HLINE, 1, 8, 0, F_NONE, 0, 10, 128, 0),
];

/// Widgets of each screen (status bar first where present).
pub fn widgets(s: Screen) -> Vec<Widget> {
    let mut v = Vec::new();
    if s != Screen::Clock {
        v.extend_from_slice(&STATUS);
    }
    match s {
        Screen::Clock => {
            v.push(wd(W_TEXT, 3, 15, 1, F_TIME, 64, 14, 0, 0));
            v.push(wd(W_TEXT, 1, 9, 1, F_CLOCK_INFO, 64, 46, 0, 0));
        }
        Screen::Navigation => {
            v.push(wd(W_ARROW, 1, 15, 0, F_NONE, 30, 38, 18, 0));
            v.push(wd(W_TEXT, 2, 15, 0, F_NAV_DIST, 60, 16, 0, 0));
            v.push(wd(W_TEXT, 1, 12, 0, F_NAV_STREET, 60, 38, 66, 0));
            v.push(wd(W_TEXT, 1, 9, 0, F_NAV_ETA, 60, 50, 66, 0));
        }
        Screen::Captions => {
            v.push(wd(W_WRAP, 1, 15, 2, F_CAPTIONS, 2, 13, 124, 50));
        }
        Screen::Notification => {
            v.push(wd(W_FILL, 1, 12, 0, F_NONE, 0, 13, 2, 50));
            v.push(wd(W_TEXT, 1, 9, 0, F_NOTIF_APP, 5, 13, 121, 0));
            v.push(wd(W_TEXT, 1, 15, 0, F_NOTIF_TITLE, 5, 23, 121, 0));
            v.push(wd(W_WRAP, 1, 12, 0, F_NOTIF_BODY, 5, 33, 121, 30));
        }
        Screen::Teleprompter => {
            v.push(wd(W_WRAP, 1, 15, 0, F_PROMPTER, 2, 13, 124, 44));
            v.push(wd(W_BAR, 1, 12, 0, F_PROMPTER_PROGRESS, 2, 59, 124, 3));
        }
    }
    v
}

/// Dynamic data the HUD shows (mirrors `iris_hud_state_t` in C).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct HudState {
    pub screen: Screen,
    pub hour: u8,
    pub minute: u8,
    pub battery_pct: u8,
    pub ble_connected: bool,
    /// Turn arrow, degrees clockwise from straight ahead.
    pub nav_arrow_deg: i32,
    pub nav_distance_m: u32,
    pub nav_street: String,
    pub nav_eta_min: u32,
    pub captions: String,
    /// Pixel offset added to the caption block (animates a new line scrolling in).
    pub caption_scroll_px: i32,
    pub notif_app: String,
    pub notif_title: String,
    pub notif_body: String,
    pub prompter: String,
    pub prompter_scroll_px: i32,
    pub prompter_progress: u8,
}

impl Default for HudState {
    fn default() -> Self {
        HudState {
            screen: Screen::Navigation,
            hour: 9,
            minute: 41,
            battery_pct: 82,
            ble_connected: true,
            nav_arrow_deg: 45,
            nav_distance_m: 350,
            nav_street: "Gaffey St".into(),
            nav_eta_min: 4,
            captions: "Welcome to the systems engineering seminar. Today we look at how a product goes from an idea to a factory."
                .into(),
            caption_scroll_px: 0,
            notif_app: "MESSAGES".into(),
            notif_title: "Lab meeting moved".into(),
            notif_body: "The prototype review is now at 3 pm in the fabrication lab. Bring the Iris breadboard.".into(),
            prompter: "Good afternoon. Iris is a monocular heads-up display that puts navigation, captions and notifications in the corner of your eye, for a full day on one charge.".into(),
            prompter_scroll_px: 0,
            prompter_progress: 20,
        }
    }
}

/// Bytes of a string truncated to a C buffer of `cap` bytes (cap − 1 characters).
fn capped(s: &str, cap: usize) -> Vec<u8> {
    let b = s.as_bytes();
    b[..b.len().min(cap - 1)].to_vec()
}

fn two(v: u32) -> String {
    format!("{:02}", v % 100)
}

/// Text of a data field (identical formatting to `iris_hud_field` in C).
pub fn field_text(st: &HudState, f: u8) -> Vec<u8> {
    match f {
        F_TIME => format!("{}:{}", two(st.hour as u32), two(st.minute as u32)).into_bytes(),
        F_BATT_PCT => format!("{}%", st.battery_pct.min(100)).into_bytes(),
        F_BLE => (if st.ble_connected { "BT" } else { "--" }).as_bytes().to_vec(),
        F_CLOCK_INFO => format!("BAT {}%  BT {}", st.battery_pct.min(100), if st.ble_connected { "ON" } else { "OFF" }).into_bytes(),
        F_NAV_DIST => {
            let m = st.nav_distance_m;
            if m < 1000 {
                format!("{m}m")
            } else if m < 10000 {
                format!("{}.{}km", m / 1000, (m % 1000) / 100)
            } else {
                format!("{}km", (m / 1000).min(99999))
            }
            .into_bytes()
        }
        F_NAV_STREET => capped(&st.nav_street, CAP_STREET),
        F_NAV_ETA => format!("ETA {} min", st.nav_eta_min.min(9999)).into_bytes(),
        F_CAPTIONS => capped(&st.captions, CAP_CAPTIONS),
        F_NOTIF_APP => capped(&st.notif_app, CAP_APP),
        F_NOTIF_TITLE => capped(&st.notif_title, CAP_TITLE),
        F_NOTIF_BODY => capped(&st.notif_body, CAP_BODY),
        F_PROMPTER => capped(&st.prompter, CAP_PROMPTER),
        _ => vec![],
    }
}

/// Greedy word wrap into (start, end) byte ranges, `cpl` characters per line.
pub fn wrap(t: &[u8], cpl: usize) -> Vec<(usize, usize)> {
    let cpl = cpl.max(1);
    let n = t.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        while i < n && t[i] == b' ' {
            i += 1;
        }
        if i >= n {
            break;
        }
        let start = i;
        let mut j = i;
        let mut col = 0;
        let mut last_space: Option<usize> = None;
        while j < n && t[j] != b'\n' && col < cpl {
            if t[j] == b' ' {
                last_space = Some(j);
            }
            j += 1;
            col += 1;
        }
        let mut end = j;
        if j < n && t[j] != b'\n' && t[j] != b' ' {
            if let Some(ls) = last_space {
                if ls > start {
                    end = ls;
                }
            }
        }
        let mut e = end;
        while e > start && t[e - 1] == b' ' {
            e -= 1;
        }
        out.push((start, e));
        i = end;
        if i < n && t[i] == b'\n' {
            i += 1;
        }
    }
    out
}

/// Integer font scale for a W × H panel.
pub fn font_scale(w: i32, h: i32) -> i32 {
    (w / DESIGN_W).min(h / DESIGN_H).max(1)
}

/// Arrow outline in units of r/16 (tip up).
pub const ARROW_PTS: [(i32, i32); 7] = [(0, -16), (11, -1), (4, -1), (4, 15), (-4, 15), (-4, -1), (-11, -1)];

/// Render one screen into `fb` (clears it first).
pub fn render(fb: &mut Fb, st: &HudState) {
    fb.clear();
    let (w_px, h_px) = (fb.w, fb.h);
    let fs = font_scale(w_px, h_px);
    for wg in widgets(st.screen) {
        let x = wg.x as i32 * w_px / DESIGN_W;
        let y = wg.y as i32 * h_px / DESIGN_H;
        let ww = wg.w as i32 * w_px / DESIGN_W;
        let hh = wg.h as i32 * h_px / DESIGN_H;
        let s = wg.size as i32 * fs;
        let lv = wg.level;
        match wg.kind {
            W_TEXT => {
                let t = if wg.field != F_NONE { field_text(st, wg.field) } else { wg.text.as_bytes().to_vec() };
                let tw = t.len() as i32 * 6 * s;
                let x0 = match wg.align {
                    1 => x - tw / 2,
                    2 => x - tw,
                    _ => x,
                };
                if ww > 0 {
                    fb.set_clip(x, 0, x + ww, h_px);
                }
                fb.text(x0, y, &t, s, lv);
                fb.reset_clip();
            }
            W_WRAP => {
                let t = field_text(st, wg.field);
                let cpl = (ww / (6 * s)).max(1) as usize;
                let lh = 8 * s;
                let lines = wrap(&t, cpl);
                let total = lines.len() as i32 * lh;
                let scroll = if wg.field == F_CAPTIONS { st.caption_scroll_px } else { st.prompter_scroll_px };
                let y0 = if wg.align == 2 {
                    (if total > hh { y + hh - total } else { y }) + scroll
                } else {
                    y - scroll
                };
                fb.set_clip(x, y, x + ww, y + hh);
                for (k, (a, b)) in lines.iter().enumerate() {
                    let ly = y0 + k as i32 * lh;
                    if ly + lh > y && ly < y + hh {
                        fb.text(x, ly, &t[*a..*b], s, lv);
                    }
                }
                fb.reset_clip();
            }
            W_RECT => fb.frame_rect(x, y, ww, hh, fs, lv),
            W_FILL => fb.fill_rect(x, y, ww, hh, lv),
            W_HLINE => fb.fill_rect(x, y, ww, fs, lv),
            W_ARROW => {
                let r = wg.w as i32 * fs;
                let (c, sn) = (cos_q14(st.nav_arrow_deg), sin_q14(st.nav_arrow_deg));
                let mut pts = [(0i32, 0i32); 7];
                for (k, (px, py)) in ARROW_PTS.iter().enumerate() {
                    let ax = px * r / 16;
                    let ay = py * r / 16;
                    pts[k] = (x + q14_round(ax * c - ay * sn), y + q14_round(ax * sn + ay * c));
                }
                fb.fill_polygon(&pts, lv);
                for t in 0..fs {
                    fb.circle(x, y, r + 2 * fs + t, 8);
                }
            }
            W_BAR => {
                let v = st.prompter_progress.min(100) as i32;
                fb.fill_rect(x, y, ww, hh, 4);
                fb.fill_rect(x, y, ww * v / 100, hh, lv);
            }
            W_BATT => {
                let pct = st.battery_pct.min(100) as i32;
                fb.frame_rect(x, y, 11 * fs, 6 * fs, fs, lv);
                fb.fill_rect(x + 11 * fs, y + 2 * fs, fs, 2 * fs, lv);
                fb.fill_rect(x + 2 * fs, y + 2 * fs, 7 * fs * pct / 100, 2 * fs, lv);
            }
            _ => {}
        }
    }
}

/// Serializable UI description for the web page.
#[derive(Clone, Debug, Serialize)]
pub struct ScreenDesc {
    pub id: Screen,
    pub title: String,
    pub c_table: String,
    pub context: String,
    pub widgets: Vec<WidgetDesc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WidgetDesc {
    pub kind_name: String,
    pub field_name: String,
    #[serde(flatten)]
    pub w: Widget,
}

pub fn describe() -> Vec<ScreenDesc> {
    Screen::ALL
        .iter()
        .map(|&s| ScreenDesc {
            id: s,
            title: s.title().into(),
            c_table: format!("IRIS_SCREEN_{}", s.c_name()),
            context: crate::rl::power::CONTEXTS[s.context()].into(),
            widgets: widgets(s)
                .into_iter()
                .map(|w| WidgetDesc {
                    kind_name: KIND_NAMES[w.kind as usize].into(),
                    field_name: FIELD_NAMES[w.field as usize].into(),
                    w,
                })
                .collect(),
        })
        .collect()
}

// ------------------------------------------------------------------ C emitter

/// `iris_gfx.h`: framebuffer type and primitives.
pub fn c_gfx_header() -> String {
    r#"/* iris_gfx.h — 1/4-bit framebuffer graphics (generated by Protoforge).
 * Integer-only mirror of the Rust renderer in forge-core firmware::gfx, so
 * the Prototype Lab's HUD preview is pixel-identical to what this draws. */
#ifndef IRIS_GFX_H
#define IRIS_GFX_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    uint8_t *buf;      /* stride * h bytes */
    int16_t w, h;
    uint8_t bpp;       /* 1: XBM order (LSB = left pixel); 4: even x in the high nibble */
    int16_t stride;
    int16_t clip[4];   /* x0, y0, x1, y1 (exclusive) */
} iris_fb_t;

#define IRIS_FB_BYTES(w, h, bpp) ((bpp) == 1 ? (((w) + 7) / 8) * (h) : (((w) + 1) / 2) * (h))

void iris_fb_init(iris_fb_t *fb, uint8_t *buf, int16_t w, int16_t h, uint8_t bpp);
void iris_fb_clear(iris_fb_t *fb);
void iris_fb_clip(iris_fb_t *fb, int32_t x0, int32_t y0, int32_t x1, int32_t y1);
void iris_fb_unclip(iris_fb_t *fb);
void iris_px(iris_fb_t *fb, int32_t x, int32_t y, uint8_t level);
void iris_fill_rect(iris_fb_t *fb, int32_t x, int32_t y, int32_t w, int32_t h, uint8_t level);
void iris_frame_rect(iris_fb_t *fb, int32_t x, int32_t y, int32_t w, int32_t h, int32_t t, uint8_t level);
void iris_line(iris_fb_t *fb, int32_t x0, int32_t y0, int32_t x1, int32_t y1, uint8_t level);
void iris_circle(iris_fb_t *fb, int32_t cx, int32_t cy, int32_t r, uint8_t level);
int32_t iris_text(iris_fb_t *fb, int32_t x, int32_t y, const char *s, int32_t n, int32_t scale, uint8_t level);
void iris_fill_polygon(iris_fb_t *fb, const int32_t *xy, int32_t n, uint8_t level);
int32_t iris_sin_q14(int32_t deg);
int32_t iris_cos_q14(int32_t deg);
int32_t iris_q14_round(int32_t v);

#ifdef __cplusplus
}
#endif
#endif /* IRIS_GFX_H */
"#
    .to_string()
}

/// `iris_gfx.c`: primitives + font + sine table.
pub fn c_gfx_source() -> String {
    let mut font = String::new();
    for (i, g) in FONT_5X7.iter().enumerate() {
        let c = (i as u8 + 32) as char;
        let label = match c {
            '\\' => "backslash".to_string(),
            '{' => "left brace".to_string(),
            '}' => "right brace".to_string(),
            '*' => "asterisk".to_string(),
            '/' => "slash".to_string(),
            _ => format!("'{c}'"),
        };
        font += &format!(
            "    {{{}}}, /* {label} */\n",
            g.iter().map(|b| format!("0x{b:02x}")).collect::<Vec<_>>().join(", ")
        );
    }
    let sin = SIN_Q14.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ");
    let mut s = String::new();
    s += "/* iris_gfx.c — generated by Protoforge. Font: X11 misc-fixed 5x7 (public domain). */\n";
    s += "#include \"iris_gfx.h\"\n\n";
    s += &format!("static const uint8_t IRIS_FONT_5X7[95][7] = {{\n{font}}};\n\n");
    s += &format!("static const int16_t IRIS_SIN_Q14[91] = {{\n    {sin}\n}};\n\n");
    s += r#"void iris_fb_init(iris_fb_t *fb, uint8_t *buf, int16_t w, int16_t h, uint8_t bpp) {
    fb->buf = buf;
    fb->w = w;
    fb->h = h;
    fb->bpp = (uint8_t)(bpp >= 4 ? 4 : 1);
    fb->stride = (int16_t)(fb->bpp == 1 ? (w + 7) / 8 : (w + 1) / 2);
    iris_fb_unclip(fb);
}

void iris_fb_unclip(iris_fb_t *fb) {
    fb->clip[0] = 0;
    fb->clip[1] = 0;
    fb->clip[2] = fb->w;
    fb->clip[3] = fb->h;
}

void iris_fb_clear(iris_fb_t *fb) {
    int32_t n = (int32_t)fb->stride * fb->h;
    for (int32_t i = 0; i < n; i++) fb->buf[i] = 0;
    iris_fb_unclip(fb);
}

void iris_fb_clip(iris_fb_t *fb, int32_t x0, int32_t y0, int32_t x1, int32_t y1) {
    fb->clip[0] = (int16_t)(x0 < 0 ? 0 : x0);
    fb->clip[1] = (int16_t)(y0 < 0 ? 0 : y0);
    fb->clip[2] = (int16_t)(x1 > fb->w ? fb->w : x1);
    fb->clip[3] = (int16_t)(y1 > fb->h ? fb->h : y1);
}

void iris_px(iris_fb_t *fb, int32_t x, int32_t y, uint8_t level) {
    if (x < fb->clip[0] || y < fb->clip[1] || x >= fb->clip[2] || y >= fb->clip[3]) return;
    if (fb->bpp == 1) {
        uint8_t *p = &fb->buf[y * fb->stride + (x >> 3)];
        uint8_t m = (uint8_t)(1u << (x & 7));
        if (level >= 8) *p |= m; else *p &= (uint8_t)~m;
    } else {
        uint8_t *p = &fb->buf[y * fb->stride + (x >> 1)];
        uint8_t l = (uint8_t)(level & 0x0F);
        if ((x & 1) == 0) *p = (uint8_t)((*p & 0x0F) | (l << 4));
        else *p = (uint8_t)((*p & 0xF0) | l);
    }
}

void iris_fill_rect(iris_fb_t *fb, int32_t x, int32_t y, int32_t w, int32_t h, uint8_t level) {
    int32_t x0 = x > fb->clip[0] ? x : fb->clip[0];
    int32_t y0 = y > fb->clip[1] ? y : fb->clip[1];
    int32_t x1 = (x + w) < fb->clip[2] ? (x + w) : fb->clip[2];
    int32_t y1 = (y + h) < fb->clip[3] ? (y + h) : fb->clip[3];
    for (int32_t yy = y0; yy < y1; yy++) {
        for (int32_t xx = x0; xx < x1; xx++) iris_px(fb, xx, yy, level);
    }
}

void iris_frame_rect(iris_fb_t *fb, int32_t x, int32_t y, int32_t w, int32_t h, int32_t t, uint8_t level) {
    iris_fill_rect(fb, x, y, w, t, level);
    iris_fill_rect(fb, x, y + h - t, w, t, level);
    iris_fill_rect(fb, x, y + t, t, h - 2 * t, level);
    iris_fill_rect(fb, x + w - t, y + t, t, h - 2 * t, level);
}

void iris_line(iris_fb_t *fb, int32_t x0, int32_t y0, int32_t x1, int32_t y1, uint8_t level) {
    int32_t dx = x1 > x0 ? x1 - x0 : x0 - x1;
    int32_t sx = x0 < x1 ? 1 : -1;
    int32_t dy = -(y1 > y0 ? y1 - y0 : y0 - y1);
    int32_t sy = y0 < y1 ? 1 : -1;
    int32_t err = dx + dy;
    for (;;) {
        iris_px(fb, x0, y0, level);
        if (x0 == x1 && y0 == y1) break;
        int32_t e2 = 2 * err;
        if (e2 >= dy) { err += dy; x0 += sx; }
        if (e2 <= dx) { err += dx; y0 += sy; }
    }
}

void iris_circle(iris_fb_t *fb, int32_t cx, int32_t cy, int32_t r, uint8_t level) {
    int32_t x = r, y = 0, err = 1 - r;
    while (x >= y) {
        iris_px(fb, cx + x, cy + y, level);
        iris_px(fb, cx + y, cy + x, level);
        iris_px(fb, cx - y, cy + x, level);
        iris_px(fb, cx - x, cy + y, level);
        iris_px(fb, cx - x, cy - y, level);
        iris_px(fb, cx - y, cy - x, level);
        iris_px(fb, cx + y, cy - x, level);
        iris_px(fb, cx + x, cy - y, level);
        y++;
        if (err < 0) {
            err += 2 * y + 1;
        } else {
            x--;
            err += 2 * (y - x) + 1;
        }
    }
}

static void iris_glyph(iris_fb_t *fb, int32_t x, int32_t y, uint8_t c, int32_t s, uint8_t level) {
    int32_t gi = (c >= 32 && c <= 126) ? (int32_t)c - 32 : '?' - 32;
    for (int32_t row = 0; row < 7; row++) {
        uint8_t bits = IRIS_FONT_5X7[gi][row];
        for (int32_t col = 0; col < 5; col++) {
            if (bits & (0x10 >> col)) iris_fill_rect(fb, x + col * s, y + row * s, s, s, level);
        }
    }
}

int32_t iris_text(iris_fb_t *fb, int32_t x, int32_t y, const char *s, int32_t n, int32_t scale, uint8_t level) {
    for (int32_t i = 0; i < n; i++) {
        iris_glyph(fb, x, y, (uint8_t)s[i], scale, level);
        x += 6 * scale;
    }
    return x;
}

#define IRIS_MAX_XS 16
void iris_fill_polygon(iris_fb_t *fb, const int32_t *xy, int32_t n, uint8_t level) {
    if (n < 3) return;
    int32_t ymin = xy[1], ymax = xy[1];
    for (int32_t i = 1; i < n; i++) {
        if (xy[2 * i + 1] < ymin) ymin = xy[2 * i + 1];
        if (xy[2 * i + 1] > ymax) ymax = xy[2 * i + 1];
    }
    if (ymin < fb->clip[1]) ymin = fb->clip[1];
    if (ymax > fb->clip[3] - 1) ymax = fb->clip[3] - 1;
    for (int32_t y = ymin; y <= ymax; y++) {
        int32_t yc = 2 * y + 1;
        int32_t xs[IRIS_MAX_XS];
        int32_t nx = 0;
        for (int32_t i = 0; i < n; i++) {
            int32_t x0 = xy[2 * i], y0 = xy[2 * i + 1];
            int32_t k = (i + 1) % n;
            int32_t x1 = xy[2 * k], y1 = xy[2 * k + 1];
            if (((2 * y0 <= yc && yc < 2 * y1) || (2 * y1 <= yc && yc < 2 * y0)) && nx < IRIS_MAX_XS) {
                xs[nx++] = x0 + (yc - 2 * y0) * (x1 - x0) / (2 * (y1 - y0));
            }
        }
        for (int32_t i = 1; i < nx; i++) {
            int32_t v = xs[i];
            int32_t j = i;
            while (j > 0 && xs[j - 1] > v) { xs[j] = xs[j - 1]; j--; }
            xs[j] = v;
        }
        for (int32_t k = 0; k + 1 < nx; k += 2) {
            for (int32_t x = xs[k]; x < xs[k + 1]; x++) iris_px(fb, x, y, level);
        }
    }
}

int32_t iris_sin_q14(int32_t deg) {
    int32_t a = deg % 360;
    if (a < 0) a += 360;
    if (a <= 90) return IRIS_SIN_Q14[a];
    if (a <= 180) return IRIS_SIN_Q14[180 - a];
    if (a <= 270) return -IRIS_SIN_Q14[a - 180];
    return -IRIS_SIN_Q14[360 - a];
}

int32_t iris_cos_q14(int32_t deg) { return iris_sin_q14(deg + 90); }

int32_t iris_q14_round(int32_t v) { return v >= 0 ? (v + 8192) >> 14 : -((-v + 8192) >> 14); }
"#;
    s
}

fn c_widget_row(w: &Widget) -> String {
    format!(
        "    {{IRIS_W_{}, {}, {}, {}, IRIS_F_{}, {}, {}, {}, {}}},\n",
        KIND_NAMES[w.kind as usize], w.size, w.level, w.align, FIELD_NAMES[w.field as usize], w.x, w.y, w.w, w.h
    )
}

/// `iris_hud.h`: HUD state + render entry point.
pub fn c_hud_header() -> String {
    let mut s = String::new();
    s += "/* iris_hud.h — Iris heads-up display screens (generated by Protoforge).\n * The widget tables in iris_hud.c are the same UI description the lab renders. */\n";
    s += "#ifndef IRIS_HUD_H\n#define IRIS_HUD_H\n#include <stdint.h>\n#include <stdbool.h>\n#include \"iris_gfx.h\"\n";
    s += "#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n";
    s += "typedef enum {\n";
    for (i, sc) in Screen::ALL.iter().enumerate() {
        s += &format!("    IRIS_SCREEN_{}_ID = {i},\n", sc.c_name());
    }
    s += "    IRIS_SCREEN_COUNT = 5\n} iris_screen_t;\n\n";
    s += &format!(
        "typedef struct {{\n    uint8_t screen;          /* iris_screen_t */\n    uint8_t hour, minute;\n    uint8_t battery_pct;\n    bool ble_connected;\n    int32_t nav_arrow_deg;   /* clockwise from straight ahead */\n    uint32_t nav_distance_m;\n    char nav_street[{CAP_STREET}];\n    uint32_t nav_eta_min;\n    char captions[{CAP_CAPTIONS}];\n    int32_t caption_scroll_px;\n    char notif_app[{CAP_APP}];\n    char notif_title[{CAP_TITLE}];\n    char notif_body[{CAP_BODY}];\n    char prompter[{CAP_PROMPTER}];\n    int32_t prompter_scroll_px;\n    uint8_t prompter_progress; /* 0..100 */\n}} iris_hud_state_t;\n\n"
    );
    s += "/* Clear fb and draw the state's current screen. */\nvoid iris_hud_render(iris_fb_t *fb, const iris_hud_state_t *st);\n";
    s += "/* Screen -> power-policy context (0 idle, 1 glance, 2 navigate, 3 caption). */\nuint8_t iris_hud_context(uint8_t screen);\n\n";
    s += "#ifdef __cplusplus\n}\n#endif\n#endif /* IRIS_HUD_H */\n";
    s
}

/// `iris_hud.c`: widget tables + interpreter.
pub fn c_hud_source() -> String {
    let mut s = String::new();
    s += "/* iris_hud.c — generated by Protoforge from the lab's UI description. */\n#include <stdio.h>\n#include <string.h>\n#include \"iris_hud.h\"\n\n";
    s += &format!("#define IRIS_DESIGN_W {DESIGN_W}\n#define IRIS_DESIGN_H {DESIGN_H}\n\n");
    for (i, k) in KIND_NAMES.iter().enumerate().skip(1) {
        s += &format!("#define IRIS_W_{k} {i}\n");
    }
    s += "\n";
    for (i, f) in FIELD_NAMES.iter().enumerate() {
        s += &format!("#define IRIS_F_{f} {i}\n");
    }
    s += "\ntypedef struct {\n    uint8_t kind, size, level, align, field;\n    int16_t x, y, w, h;\n} iris_widget_t;\n\n";
    for sc in Screen::ALL {
        let ws = widgets(sc);
        s += &format!("static const iris_widget_t IRIS_SCREEN_{}[] = {{\n", sc.c_name());
        for w in &ws {
            s += &c_widget_row(w);
        }
        s += "};\n";
    }
    s += "\nstatic const iris_widget_t *const IRIS_SCREENS[IRIS_SCREEN_COUNT] = {";
    s += &Screen::ALL.iter().map(|sc| format!("IRIS_SCREEN_{}", sc.c_name())).collect::<Vec<_>>().join(", ");
    s += "};\nstatic const uint8_t IRIS_SCREEN_LEN[IRIS_SCREEN_COUNT] = {";
    s += &Screen::ALL.iter().map(|sc| widgets(*sc).len().to_string()).collect::<Vec<_>>().join(", ");
    s += "};\nstatic const uint8_t IRIS_SCREEN_CTX[IRIS_SCREEN_COUNT] = {";
    s += &Screen::ALL.iter().map(|sc| sc.context().to_string()).collect::<Vec<_>>().join(", ");
    s += "};\n";
    let arrow = ARROW_PTS.iter().map(|(x, y)| format!("{x}, {y}")).collect::<Vec<_>>().join(", ");
    s += &format!("static const int8_t IRIS_ARROW_PTS[{}] = {{{arrow}}};\n\n", ARROW_PTS.len() * 2);
    s += r#"uint8_t iris_hud_context(uint8_t screen) {
    return screen < IRIS_SCREEN_COUNT ? IRIS_SCREEN_CTX[screen] : 0;
}

static int32_t iris_cpy(char *out, const char *src, int32_t cap) {
    int32_t n = 0;
    while (src[n] && n < cap - 1) { out[n] = src[n]; n++; }
    return n;
}

/* Text of a data field into out (NOT NUL-terminated); returns its length. */
static int32_t iris_hud_field(const iris_hud_state_t *st, uint8_t f, char *out, int32_t cap) {
    uint32_t b = st->battery_pct > 100 ? 100 : st->battery_pct;
    uint32_t m = st->nav_distance_m;
    int n = 0;
    switch (f) {
    case IRIS_F_TIME: n = snprintf(out, (size_t)cap, "%02u:%02u", (unsigned)(st->hour % 100), (unsigned)(st->minute % 100)); break;
    case IRIS_F_BATT_PCT: n = snprintf(out, (size_t)cap, "%u%%", (unsigned)b); break;
    case IRIS_F_BLE: n = snprintf(out, (size_t)cap, "%s", st->ble_connected ? "BT" : "--"); break;
    case IRIS_F_CLOCK_INFO: n = snprintf(out, (size_t)cap, "BAT %u%%  BT %s", (unsigned)b, st->ble_connected ? "ON" : "OFF"); break;
    case IRIS_F_NAV_DIST:
        if (m < 1000) n = snprintf(out, (size_t)cap, "%um", (unsigned)m);
        else if (m < 10000) n = snprintf(out, (size_t)cap, "%u.%ukm", (unsigned)(m / 1000), (unsigned)((m % 1000) / 100));
        else n = snprintf(out, (size_t)cap, "%ukm", (unsigned)(m / 1000 > 99999 ? 99999 : m / 1000));
        break;
    case IRIS_F_NAV_STREET: return iris_cpy(out, st->nav_street, cap);
    case IRIS_F_NAV_ETA: n = snprintf(out, (size_t)cap, "ETA %u min", (unsigned)(st->nav_eta_min > 9999 ? 9999 : st->nav_eta_min)); break;
    case IRIS_F_CAPTIONS: return iris_cpy(out, st->captions, cap);
    case IRIS_F_NOTIF_APP: return iris_cpy(out, st->notif_app, cap);
    case IRIS_F_NOTIF_TITLE: return iris_cpy(out, st->notif_title, cap);
    case IRIS_F_NOTIF_BODY: return iris_cpy(out, st->notif_body, cap);
    case IRIS_F_PROMPTER: return iris_cpy(out, st->prompter, cap);
    default: return 0;
    }
    if (n < 0) n = 0;
    if (n > cap - 1) n = cap - 1;
    return n;
}

/* Greedy word wrap: advances *pos, returns the line [*start, *end). 0 = no more lines. */
static int iris_wrap_next(const char *t, int32_t n, int32_t cpl, int32_t *pos, int32_t *start, int32_t *end) {
    int32_t i = *pos;
    if (cpl < 1) cpl = 1;
    while (i < n && t[i] == ' ') i++;
    if (i >= n) { *pos = i; return 0; }
    int32_t s = i, j = i, col = 0, last_space = -1;
    while (j < n && t[j] != '\n' && col < cpl) {
        if (t[j] == ' ') last_space = j;
        j++;
        col++;
    }
    int32_t e = j;
    if (j < n && t[j] != '\n' && t[j] != ' ' && last_space > s) e = last_space;
    int32_t k = e;
    while (k > s && t[k - 1] == ' ') k--;
    *start = s;
    *end = k;
    i = e;
    if (i < n && t[i] == '\n') i++;
    *pos = i;
    return 1;
}

static char iris_hud_buf[512];

void iris_hud_render(iris_fb_t *fb, const iris_hud_state_t *st) {
    iris_fb_clear(fb);
    const int32_t W = fb->w, H = fb->h;
    int32_t fs = W / IRIS_DESIGN_W < H / IRIS_DESIGN_H ? W / IRIS_DESIGN_W : H / IRIS_DESIGN_H;
    if (fs < 1) fs = 1;
    uint8_t sc = st->screen < IRIS_SCREEN_COUNT ? st->screen : 0;
    const iris_widget_t *ws = IRIS_SCREENS[sc];
    for (uint8_t i = 0; i < IRIS_SCREEN_LEN[sc]; i++) {
        const iris_widget_t *wg = &ws[i];
        int32_t x = wg->x * W / IRIS_DESIGN_W;
        int32_t y = wg->y * H / IRIS_DESIGN_H;
        int32_t ww = wg->w * W / IRIS_DESIGN_W;
        int32_t hh = wg->h * H / IRIS_DESIGN_H;
        int32_t s = wg->size * fs;
        uint8_t lv = wg->level;
        switch (wg->kind) {
        case IRIS_W_TEXT: {
            int32_t n = iris_hud_field(st, wg->field, iris_hud_buf, (int32_t)sizeof iris_hud_buf);
            int32_t tw = n * 6 * s;
            int32_t x0 = wg->align == 1 ? x - tw / 2 : (wg->align == 2 ? x - tw : x);
            if (ww > 0) iris_fb_clip(fb, x, 0, x + ww, H);
            iris_text(fb, x0, y, iris_hud_buf, n, s, lv);
            iris_fb_unclip(fb);
            break;
        }
        case IRIS_W_WRAP: {
            int32_t n = iris_hud_field(st, wg->field, iris_hud_buf, (int32_t)sizeof iris_hud_buf);
            int32_t cpl = ww / (6 * s);
            if (cpl < 1) cpl = 1;
            int32_t lh = 8 * s, pos = 0, a = 0, b = 0, lines = 0;
            while (iris_wrap_next(iris_hud_buf, n, cpl, &pos, &a, &b)) lines++;
            int32_t total = lines * lh;
            int32_t scroll = wg->field == IRIS_F_CAPTIONS ? st->caption_scroll_px : st->prompter_scroll_px;
            int32_t y0 = wg->align == 2 ? ((total > hh ? y + hh - total : y) + scroll) : (y - scroll);
            iris_fb_clip(fb, x, y, x + ww, y + hh);
            pos = 0;
            for (int32_t k = 0; iris_wrap_next(iris_hud_buf, n, cpl, &pos, &a, &b); k++) {
                int32_t ly = y0 + k * lh;
                if (ly + lh > y && ly < y + hh) iris_text(fb, x, ly, iris_hud_buf + a, b - a, s, lv);
            }
            iris_fb_unclip(fb);
            break;
        }
        case IRIS_W_RECT: iris_frame_rect(fb, x, y, ww, hh, fs, lv); break;
        case IRIS_W_FILL: iris_fill_rect(fb, x, y, ww, hh, lv); break;
        case IRIS_W_HLINE: iris_fill_rect(fb, x, y, ww, fs, lv); break;
        case IRIS_W_ARROW: {
            int32_t r = wg->w * fs;
            int32_t c = iris_cos_q14(st->nav_arrow_deg), sn = iris_sin_q14(st->nav_arrow_deg);
            int32_t pts[14];
            for (int32_t k = 0; k < 7; k++) {
                int32_t ax = IRIS_ARROW_PTS[2 * k] * r / 16;
                int32_t ay = IRIS_ARROW_PTS[2 * k + 1] * r / 16;
                pts[2 * k] = x + iris_q14_round(ax * c - ay * sn);
                pts[2 * k + 1] = y + iris_q14_round(ax * sn + ay * c);
            }
            iris_fill_polygon(fb, pts, 7, lv);
            for (int32_t t = 0; t < fs; t++) iris_circle(fb, x, y, r + 2 * fs + t, 8);
            break;
        }
        case IRIS_W_BAR: {
            int32_t v = st->prompter_progress > 100 ? 100 : st->prompter_progress;
            iris_fill_rect(fb, x, y, ww, hh, 4);
            iris_fill_rect(fb, x, y, ww * v / 100, hh, lv);
            break;
        }
        case IRIS_W_BATT: {
            int32_t p = st->battery_pct > 100 ? 100 : st->battery_pct;
            iris_frame_rect(fb, x, y, 11 * fs, 6 * fs, fs, lv);
            iris_fill_rect(fb, x + 11 * fs, y + 2 * fs, fs, 2 * fs, lv);
            iris_fill_rect(fb, x + 2 * fs, y + 2 * fs, 7 * fs * p / 100, 2 * fs, lv);
            break;
        }
        default: break;
        }
    }
}
"#;
    s
}

#[cfg(test)]
mod tests {
    use super::super::gfx::fnv1a;
    use super::*;

    #[test]
    fn wrap_is_greedy_and_hard_breaks() {
        let t = b"the quick brown fox jumps";
        let l: Vec<&[u8]> = wrap(t, 10).iter().map(|(a, b)| &t[*a..*b]).collect();
        assert_eq!(l, vec![&b"the quick"[..], b"brown fox", b"jumps"]);
        let t = b"abcdefghijkl mn";
        let l: Vec<&[u8]> = wrap(t, 5).iter().map(|(a, b)| &t[*a..*b]).collect();
        assert_eq!(l, vec![&b"abcde"[..], b"fghij", b"kl mn"]);
        let t = b"one\n\ntwo";
        assert_eq!(wrap(t, 8).len(), 3);
        assert!(wrap(b"   ", 4).is_empty());
    }

    #[test]
    fn field_formatting() {
        let mut st = HudState::default();
        assert_eq!(field_text(&st, F_TIME), b"09:41");
        st.nav_distance_m = 1250;
        assert_eq!(field_text(&st, F_NAV_DIST), b"1.2km");
        st.nav_distance_m = 25_400;
        assert_eq!(field_text(&st, F_NAV_DIST), b"25km");
        st.nav_street = "x".repeat(100);
        assert_eq!(field_text(&st, F_NAV_STREET).len(), CAP_STREET - 1);
    }

    #[test]
    fn rendering_is_deterministic_and_fills_every_screen() {
        for (w, h, bpp) in [(128, 64, 1), (640, 400, 1), (640, 400, 4)] {
            for sc in Screen::ALL {
                let st = HudState {
                    screen: sc,
                    ..Default::default()
                };
                let mut a = Fb::new(w, h, bpp);
                let mut b = Fb::new(w, h, bpp);
                render(&mut a, &st);
                render(&mut b, &st);
                assert_eq!(fnv1a(&a.buf), fnv1a(&b.buf));
                let lit = a.lit();
                assert!(lit > 40 && lit < (w * h) as usize / 2, "{sc:?} {w}x{h}: {lit}");
            }
        }
        // the arrow turns: 90° differs from 0°
        let mut a = Fb::new(128, 64, 1);
        let mut b = Fb::new(128, 64, 1);
        render(
            &mut a,
            &HudState {
                nav_arrow_deg: 0,
                ..Default::default()
            },
        );
        render(
            &mut b,
            &HudState {
                nav_arrow_deg: 90,
                ..Default::default()
            },
        );
        assert_ne!(a.buf, b.buf);
    }

    #[test]
    fn c_sources_mirror_the_tables() {
        let c = c_hud_source();
        assert!(c.contains("static const iris_widget_t IRIS_SCREEN_NAV[]"));
        assert!(c.contains("{IRIS_W_ARROW, 1, 15, 0, IRIS_F_NONE, 30, 38, 18, 0}"));
        let g = c_gfx_source();
        assert!(g.contains("IRIS_FONT_5X7[95][7]"));
        assert!(g.matches("/* ").count() >= 95);
        for src in [c, g, c_gfx_header(), c_hud_header()] {
            assert_eq!(src.matches('{').count(), src.matches('}').count());
        }
    }
}
