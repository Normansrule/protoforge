//! Laser material test grid (SVG) and kerf-measurement strip.
//!
//! Absolute power and speed settings differ between machines, tubes, lenses
//! and materials, so instead of printing numbers we generate a **test grid**:
//! rows step the power, columns step the speed. Every cell gets its own
//! stroke colour so laser software that maps colours to layers (for
//! example LightBurn) can assign one setting per cell; the returned table
//! lists which colour gets which setting. Labels are single-stroke vector
//! text (the PCB silkscreen font) on a black engrave layer; the kerf strip
//! is red.
//!
//! Kerf from the strip: a W mm strip cut into N slices loses one kerf per
//! slice, so kerf = (W − measured length of the pushed-together slices) / N.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct GridIn {
    pub rows: usize,
    pub cols: usize,
    pub power_min: f64,
    pub power_max: f64,
    pub speed_min: f64,
    pub speed_max: f64,
    /// Unit shown for speed (e.g. "mm/s" or "%").
    pub speed_unit: String,
    pub cell_mm: f64,
    pub gap_mm: f64,
    /// "cut" (outlines) or "engrave" (filled squares).
    pub mode: String,
    pub kerf_strip: bool,
    pub title: String,
}

impl Default for GridIn {
    fn default() -> Self {
        GridIn {
            rows: 5,
            cols: 5,
            power_min: 20.0,
            power_max: 100.0,
            speed_min: 5.0,
            speed_max: 25.0,
            speed_unit: "mm/s".into(),
            cell_mm: 10.0,
            gap_mm: 3.0,
            mode: "cut".into(),
            kerf_strip: true,
            title: "LASER TEST".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Cell {
    pub row: usize,
    pub col: usize,
    pub power_pct: f64,
    pub speed: f64,
    pub color: String,
    pub x_mm: f64,
    pub y_mm: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct GridOut {
    pub svg: String,
    pub cells: Vec<Cell>,
    pub width_mm: f64,
    pub height_mm: f64,
    pub csv: String,
}

pub const MAX_CELLS: usize = 30;

fn hsl_hex(h: f64, s: f64, l: f64) -> String {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h / 60.0).rem_euclid(6.0);
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to = |v: f64| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", to(r), to(g), to(b))
}

/// Distinct layer colours, avoiding black (labels) and pure red (kerf strip).
pub fn palette(n: usize) -> Vec<String> {
    (0..n)
        .map(|k| {
            let h = 25.0 + 330.0 * k as f64 / n.max(1) as f64;
            let l = if k % 2 == 0 { 0.42 } else { 0.55 };
            hsl_hex(h, 0.85, l)
        })
        .collect()
}

fn lin(a: f64, b: f64, k: usize, n: usize) -> f64 {
    if n <= 1 {
        a
    } else {
        a + (b - a) * k as f64 / (n - 1) as f64
    }
}

fn text_path(s: &str, x: f64, y_base: f64, h: f64, height: f64) -> String {
    // stroke font is y-up; flip into SVG's y-down page coordinates
    let mut d = String::new();
    for stroke in crate::pcb::fab::text_strokes(&s.to_ascii_uppercase(), x, height - y_base, h) {
        for (k, (px, py)) in stroke.iter().enumerate() {
            d += &format!("{}{:.3},{:.3} ", if k == 0 { "M" } else { "L" }, px, height - py);
        }
    }
    format!(
        "<path d=\"{}\" fill=\"none\" stroke=\"#000000\" stroke-width=\"0.2\" stroke-linecap=\"round\"/>",
        d.trim_end()
    )
}

pub fn grid(i: &GridIn) -> Result<GridOut, String> {
    let rows = i.rows.clamp(1, 10);
    let cols = i.cols.clamp(1, 10);
    if rows * cols > MAX_CELLS {
        return Err(format!("at most {MAX_CELLS} cells (colour layers) — reduce rows or columns"));
    }
    if i.cell_mm <= 0.0 {
        return Err("cell size must be positive".into());
    }
    let (c, g) = (i.cell_mm, i.gap_mm.max(0.5));
    let lab = 8.0; // left label column
    let top = 12.0; // title + column labels
    let th = (c * 0.28).clamp(1.5, 3.0);
    let grid_w = cols as f64 * c + (cols - 1) as f64 * g;
    let grid_h = rows as f64 * c + (rows - 1) as f64 * g;
    let strip_h = if i.kerf_strip { 22.0 } else { 0.0 };
    let width = (lab + grid_w + 4.0).max(if i.kerf_strip { 58.0 } else { 0.0 });
    let height = top + grid_h + 4.0 + strip_h;
    let colors = palette(rows * cols);
    let mut body = String::new();
    let mut cells = Vec::new();
    body += &text_path(&i.title, lab, 5.0, 3.0, height);
    for col in 0..cols {
        let sp = lin(i.speed_min, i.speed_max, col, cols);
        body += &text_path(&format!("S{:.0}", sp), lab + col as f64 * (c + g), top - 2.0, th, height);
    }
    for row in 0..rows {
        let pw = lin(i.power_max, i.power_min, row, rows);
        let y = top + row as f64 * (c + g);
        body += &text_path(&format!("P{:.0}", pw), 1.0, y + c / 2.0 + th / 2.0, th, height);
        for col in 0..cols {
            let sp = lin(i.speed_min, i.speed_max, col, cols);
            let x = lab + col as f64 * (c + g);
            let color = colors[row * cols + col].clone();
            if i.mode == "engrave" {
                body += &format!("<rect x=\"{x:.3}\" y=\"{y:.3}\" width=\"{c:.3}\" height=\"{c:.3}\" fill=\"{color}\" stroke=\"none\"/>");
            } else {
                body += &format!("<rect x=\"{x:.3}\" y=\"{y:.3}\" width=\"{c:.3}\" height=\"{c:.3}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"0.1\"/>");
            }
            cells.push(Cell {
                row,
                col,
                power_pct: pw,
                speed: sp,
                color,
                x_mm: x,
                y_mm: y,
            });
        }
    }
    if i.kerf_strip {
        let y = top + grid_h + 8.0;
        body += &text_path("KERF 50MM / 10", 2.0, y - 1.5, 2.0, height);
        body += &format!(
            "<rect x=\"2.000\" y=\"{y:.3}\" width=\"50.000\" height=\"10.000\" fill=\"none\" stroke=\"#ff0000\" stroke-width=\"0.1\"/>"
        );
        for k in 1..10 {
            let x = 2.0 + 5.0 * k as f64;
            body += &format!(
                "<line x1=\"{x:.3}\" y1=\"{y:.3}\" x2=\"{x:.3}\" y2=\"{:.3}\" stroke=\"#ff0000\" stroke-width=\"0.1\"/>",
                y + 10.0
            );
        }
    }
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.2}mm\" height=\"{height:.2}mm\" viewBox=\"0 0 {width:.3} {height:.3}\">\n<!-- Protoforge laser test grid: rows = power (top = highest), columns = speed. One colour per cell = one layer. Black = label engraving, red = kerf strip. -->\n{body}\n</svg>\n"
    );
    let mut csv = format!("color,row,col,power_pct,speed_{}\n", i.speed_unit.replace(['/', ' '], "_"));
    for c in &cells {
        csv += &format!("{},{},{},{:.1},{:.2}\n", c.color, c.row + 1, c.col + 1, c.power_pct, c.speed);
    }
    Ok(GridOut {
        svg,
        cells,
        width_mm: width,
        height_mm: height,
        csv,
    })
}

/// Kerf from the 50 mm / 10-slice strip.
pub fn kerf_from_strip(nominal_mm: f64, measured_mm: f64, slices: usize) -> f64 {
    (nominal_mm - measured_mm) / slices.max(1) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_has_one_colour_per_cell() {
        let g = grid(&GridIn::default()).unwrap();
        assert_eq!(g.cells.len(), 25);
        let mut cols: Vec<&str> = g.cells.iter().map(|c| c.color.as_str()).collect();
        cols.sort();
        cols.dedup();
        assert_eq!(cols.len(), 25);
        assert!(!cols.contains(&"#000000") && !cols.contains(&"#ff0000"));
        assert!(g.svg.starts_with("<svg") && g.svg.contains("viewBox"));
        // top row is the highest power, first column the lowest speed
        assert_eq!(g.cells[0].power_pct, 100.0);
        assert_eq!(g.cells[0].speed, 5.0);
        assert_eq!(g.cells[24].power_pct, 20.0);
        assert_eq!(g.csv.lines().count(), 26);
    }

    #[test]
    fn too_many_cells_rejected() {
        assert!(grid(&GridIn {
            rows: 6,
            cols: 6,
            ..Default::default()
        })
        .is_err());
    }

    #[test]
    fn kerf_hand_calc() {
        // 10 slices measure 48.5 mm together → kerf = 1.5 / 10 = 0.15 mm
        assert!((kerf_from_strip(50.0, 48.5, 10) - 0.15).abs() < 1e-12);
    }
}
