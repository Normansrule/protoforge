//! Tiny 1-bit / 4-bit raster graphics shared by the heads-up display (HUD)
//! preview and the generated firmware.
//!
//! Everything here is integer arithmetic so that the C emitted by
//! [`super::hud::c_gfx_source`] draws **exactly** the same pixels as this Rust
//! reference: a 5×7 bitmap font, rectangles, Bresenham lines (J. E.
//! Bresenham, "Algorithm for computer control of a digital plotter", IBM
//! Systems Journal 4(1), 1965), midpoint circles and an even–odd scanline
//! polygon fill sampled at pixel centres (J. D. Foley, A. van Dam et al.,
//! *Computer Graphics: Principles and Practice*, 2nd ed., 1990, ch. 3).
//! Rotation uses a Q14 sine table (1° steps) instead of floating point, so C
//! and Rust agree bit for bit.
//!
//! Framebuffer formats:
//! * **1 bpp** — row-major, `stride = ⌈w/8⌉`, least-significant bit = leftmost
//!   pixel (the XBM layout that U8g2's `drawXBM` accepts); a pixel is lit when
//!   its level is ≥ 8.
//! * **4 bpp** — row-major, `stride = ⌈w/2⌉`, even x in the high nibble, 16 grey
//!   levels (a common layout for grayscale OLED controllers — check the
//!   panel datasheet for its exact pixel order).

use super::font::{FONT_5X7, SIN_Q14};

/// Maximum polygon edge crossings per scanline (the arrow needs ≤ 4).
pub const MAX_XS: usize = 16;

#[derive(Clone, Debug)]
pub struct Fb {
    pub w: i32,
    pub h: i32,
    pub bpp: u8,
    pub stride: i32,
    pub buf: Vec<u8>,
    clip: [i32; 4],
}

/// sin(deg) in Q14 for any integer angle.
pub fn sin_q14(deg: i32) -> i32 {
    let a = deg.rem_euclid(360);
    if a <= 90 {
        SIN_Q14[a as usize]
    } else if a <= 180 {
        SIN_Q14[(180 - a) as usize]
    } else if a <= 270 {
        -SIN_Q14[(a - 180) as usize]
    } else {
        -SIN_Q14[(360 - a) as usize]
    }
}

pub fn cos_q14(deg: i32) -> i32 {
    sin_q14(deg + 90)
}

/// Round-half-away-from-zero division by 2^14 (symmetric, so it does not
/// depend on how a C compiler shifts negative numbers).
pub fn q14_round(v: i32) -> i32 {
    if v >= 0 {
        (v + 8192) >> 14
    } else {
        -((-v + 8192) >> 14)
    }
}

/// Glyph index for a byte (non-printable and non-ASCII bytes draw as '?').
pub fn glyph_index(c: u8) -> usize {
    if (32..=126).contains(&c) {
        (c - 32) as usize
    } else {
        (b'?' - 32) as usize
    }
}

impl Fb {
    pub fn new(w: i32, h: i32, bpp: u8) -> Fb {
        let w = w.clamp(8, 2048);
        let h = h.clamp(8, 2048);
        let bpp = if bpp >= 4 { 4 } else { 1 };
        let stride = if bpp == 1 { (w + 7) / 8 } else { (w + 1) / 2 };
        Fb {
            w,
            h,
            bpp,
            stride,
            buf: vec![0; (stride * h) as usize],
            clip: [0, 0, w, h],
        }
    }

    pub fn clear(&mut self) {
        self.buf.iter_mut().for_each(|b| *b = 0);
        self.reset_clip();
    }

    /// Clip to [x0, x1) × [y0, y1), intersected with the screen.
    pub fn set_clip(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.clip = [x0.max(0), y0.max(0), x1.min(self.w), y1.min(self.h)];
    }

    pub fn reset_clip(&mut self) {
        self.clip = [0, 0, self.w, self.h];
    }

    pub fn pixel(&mut self, x: i32, y: i32, level: u8) {
        if x < self.clip[0] || y < self.clip[1] || x >= self.clip[2] || y >= self.clip[3] {
            return;
        }
        if self.bpp == 1 {
            let i = (y * self.stride + (x >> 3)) as usize;
            let m = 1u8 << (x & 7);
            if level >= 8 {
                self.buf[i] |= m;
            } else {
                self.buf[i] &= !m;
            }
        } else {
            let i = (y * self.stride + (x >> 1)) as usize;
            let l = level & 0x0F;
            if x & 1 == 0 {
                self.buf[i] = (self.buf[i] & 0x0F) | (l << 4);
            } else {
                self.buf[i] = (self.buf[i] & 0xF0) | l;
            }
        }
    }

    /// Grey level 0..=15 of a pixel (1 bpp → 0 or 15).
    pub fn get(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return 0;
        }
        if self.bpp == 1 {
            let b = self.buf[(y * self.stride + (x >> 3)) as usize];
            if b & (1 << (x & 7)) != 0 {
                15
            } else {
                0
            }
        } else {
            let b = self.buf[(y * self.stride + (x >> 1)) as usize];
            if x & 1 == 0 {
                b >> 4
            } else {
                b & 0x0F
            }
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, level: u8) {
        // clamp to the clip box first so huge rectangles stay cheap
        let x0 = x.max(self.clip[0]);
        let y0 = y.max(self.clip[1]);
        let x1 = (x + w).min(self.clip[2]);
        let y1 = (y + h).min(self.clip[3]);
        for yy in y0..y1 {
            for xx in x0..x1 {
                self.pixel(xx, yy, level);
            }
        }
    }

    /// Rectangle outline with thickness `t`.
    pub fn frame_rect(&mut self, x: i32, y: i32, w: i32, h: i32, t: i32, level: u8) {
        self.fill_rect(x, y, w, t, level);
        self.fill_rect(x, y + h - t, w, t, level);
        self.fill_rect(x, y + t, t, h - 2 * t, level);
        self.fill_rect(x + w - t, y + t, t, h - 2 * t, level);
    }

    /// Bresenham line (all octants).
    pub fn line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, level: u8) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.pixel(x0, y0, level);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Midpoint circle outline.
    pub fn circle(&mut self, cx: i32, cy: i32, r: i32, level: u8) {
        let mut x = r;
        let mut y = 0;
        let mut err = 1 - r;
        while x >= y {
            self.pixel(cx + x, cy + y, level);
            self.pixel(cx + y, cy + x, level);
            self.pixel(cx - y, cy + x, level);
            self.pixel(cx - x, cy + y, level);
            self.pixel(cx - x, cy - y, level);
            self.pixel(cx - y, cy - x, level);
            self.pixel(cx + y, cy - x, level);
            self.pixel(cx + x, cy - y, level);
            y += 1;
            if err < 0 {
                err += 2 * y + 1;
            } else {
                x -= 1;
                err += 2 * (y - x) + 1;
            }
        }
    }

    /// One glyph with its top-left corner at (x, y), each font pixel an s×s block.
    pub fn glyph(&mut self, x: i32, y: i32, c: u8, s: i32, level: u8) {
        let g = FONT_5X7[glyph_index(c)];
        for (row, bits) in g.iter().enumerate() {
            for col in 0..5 {
                if bits & (0x10 >> col) != 0 {
                    self.fill_rect(x + col * s, y + row as i32 * s, s, s, level);
                }
            }
        }
    }

    /// Draw bytes left to right with a 6·s advance; returns the end x.
    pub fn text(&mut self, mut x: i32, y: i32, s: &[u8], scale: i32, level: u8) -> i32 {
        for &c in s {
            self.glyph(x, y, c, scale, level);
            x += 6 * scale;
        }
        x
    }

    /// Even–odd scanline fill, sampling each row at its centre (y + ½).
    pub fn fill_polygon(&mut self, pts: &[(i32, i32)], level: u8) {
        if pts.len() < 3 {
            return;
        }
        let mut ymin = pts[0].1;
        let mut ymax = pts[0].1;
        for p in pts {
            ymin = ymin.min(p.1);
            ymax = ymax.max(p.1);
        }
        ymin = ymin.max(self.clip[1]);
        ymax = ymax.min(self.clip[3] - 1);
        let n = pts.len();
        for y in ymin..=ymax {
            let yc = 2 * y + 1;
            let mut xs = [0i32; MAX_XS];
            let mut nx = 0usize;
            for i in 0..n {
                let (x0, y0) = pts[i];
                let (x1, y1) = pts[(i + 1) % n];
                if ((2 * y0 <= yc && yc < 2 * y1) || (2 * y1 <= yc && yc < 2 * y0)) && nx < MAX_XS {
                    xs[nx] = x0 + (yc - 2 * y0) * (x1 - x0) / (2 * (y1 - y0));
                    nx += 1;
                }
            }
            // insertion sort
            for i in 1..nx {
                let v = xs[i];
                let mut j = i;
                while j > 0 && xs[j - 1] > v {
                    xs[j] = xs[j - 1];
                    j -= 1;
                }
                xs[j] = v;
            }
            let mut k = 0;
            while k + 1 < nx {
                for x in xs[k]..xs[k + 1] {
                    self.pixel(x, y, level);
                }
                k += 2;
            }
        }
    }

    /// Lit pixels (level ≥ 8), for tests and statistics.
    pub fn lit(&self) -> usize {
        let mut n = 0;
        for y in 0..self.h {
            for x in 0..self.w {
                if self.get(x, y) >= 8 {
                    n += 1;
                }
            }
        }
        n
    }
}

/// Standard base64 (RFC 4648) without line breaks.
pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for ch in data.chunks(3) {
        let b = [ch[0], *ch.get(1).unwrap_or(&0), *ch.get(2).unwrap_or(&0)];
        let v = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(v >> 18) as usize & 63] as char);
        out.push(T[(v >> 12) as usize & 63] as char);
        out.push(if ch.len() > 1 { T[(v >> 6) as usize & 63] as char } else { '=' });
        out.push(if ch.len() > 2 { T[v as usize & 63] as char } else { '=' });
    }
    out
}

/// FNV-1a hash (determinism checks).
pub fn fnv1a(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_table_symmetry() {
        assert_eq!(sin_q14(0), 0);
        assert_eq!(sin_q14(90), 16384);
        assert_eq!(sin_q14(270), -16384);
        assert_eq!(sin_q14(-90), -16384);
        assert_eq!(cos_q14(0), 16384);
        assert_eq!(sin_q14(30), 8192);
        for d in -720..720 {
            let s = sin_q14(d) as f64 / 16384.0;
            assert!((s - (d as f64).to_radians().sin()).abs() < 1e-4, "{d}");
        }
        assert_eq!(q14_round(8192), 1);
        assert_eq!(q14_round(-8192), -1);
        assert_eq!(q14_round(8191), 0);
    }

    #[test]
    fn glyph_a_matches_font_rows() {
        let mut fb = Fb::new(16, 8, 1);
        fb.glyph(0, 0, b'A', 1, 15);
        let rows = FONT_5X7[(b'A' - 32) as usize];
        for (y, bits) in rows.iter().enumerate() {
            for x in 0..5 {
                let on = bits & (0x10 >> x) != 0;
                assert_eq!(fb.get(x, y as i32) == 15, on, "({x},{y})");
            }
        }
        // XBM order: pixel 0 is bit 0 of byte 0
        let mut fb = Fb::new(16, 8, 1);
        fb.pixel(0, 0, 15);
        fb.pixel(9, 1, 15);
        assert_eq!(fb.buf[0], 1);
        assert_eq!(fb.buf[fb.stride as usize + 1], 2);
        let mut g = Fb::new(8, 2, 4);
        g.pixel(0, 0, 12);
        g.pixel(1, 0, 3);
        assert_eq!(g.buf[0], 0xC3);
    }

    #[test]
    fn polygon_fill_area_matches_shoelace() {
        // axis-aligned 10×6 rectangle → exactly 60 pixels
        let mut fb = Fb::new(32, 32, 1);
        fb.fill_polygon(&[(2, 3), (12, 3), (12, 9), (2, 9)], 15);
        assert_eq!(fb.lit(), 60);
        // right triangle legs 20: area 200, sampled count within a few %
        let mut fb = Fb::new(32, 32, 1);
        fb.fill_polygon(&[(0, 0), (20, 20), (0, 20)], 15);
        let n = fb.lit() as f64;
        assert!((n - 200.0).abs() < 15.0, "{n}");
    }

    #[test]
    fn bresenham_and_circle_endpoints() {
        let mut fb = Fb::new(32, 32, 1);
        fb.line(1, 1, 20, 7, 15);
        assert_eq!(fb.get(1, 1), 15);
        assert_eq!(fb.get(20, 7), 15);
        assert_eq!(fb.lit(), 20); // one pixel per column for a shallow line
        let mut fb = Fb::new(32, 32, 1);
        fb.circle(16, 16, 10, 15);
        assert_eq!(fb.get(26, 16), 15);
        assert_eq!(fb.get(16, 6), 15);
        assert_eq!(fb.get(16, 16), 0);
    }

    #[test]
    fn base64_known_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }
}
