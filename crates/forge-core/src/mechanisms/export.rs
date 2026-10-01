//! Plain-text exports: AutoCAD DXF R12 (the lowest common denominator that
//! laser-cutter software, Inkscape, FreeCAD, Fusion and Onshape import) and
//! SVG in millimetres.

use std::fmt::Write;

/// A closed or open polyline on a named layer.
pub struct Poly<'a> {
    pub layer: &'a str,
    pub pts: &'a [[f64; 2]],
    pub closed: bool,
}

/// A circle on a named layer.
pub struct Circle<'a> {
    pub layer: &'a str,
    pub c: [f64; 2],
    pub r: f64,
}

/// DXF R12 with POLYLINE/VERTEX/SEQEND and CIRCLE entities, units = mm.
pub fn dxf(polys: &[Poly], circles: &[Circle]) -> String {
    let mut s = String::new();
    s.push_str("0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1009\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n");
    // layer table
    let mut layers: Vec<&str> = polys.iter().map(|p| p.layer).chain(circles.iter().map(|c| c.layer)).collect();
    layers.sort();
    layers.dedup();
    s.push_str("0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n");
    let _ = write!(s, "70\n{}\n", layers.len());
    for (k, l) in layers.iter().enumerate() {
        let _ = write!(s, "0\nLAYER\n2\n{l}\n70\n0\n62\n{}\n6\nCONTINUOUS\n", k + 1);
    }
    s.push_str("0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n");
    for p in polys {
        let _ = write!(
            s,
            "0\nPOLYLINE\n8\n{}\n66\n1\n70\n{}\n10\n0.0\n20\n0.0\n30\n0.0\n",
            p.layer,
            if p.closed { 1 } else { 0 }
        );
        for q in p.pts {
            let _ = write!(s, "0\nVERTEX\n8\n{}\n10\n{:.5}\n20\n{:.5}\n30\n0.0\n", p.layer, q[0], q[1]);
        }
        s.push_str("0\nSEQEND\n");
    }
    for c in circles {
        let _ = write!(
            s,
            "0\nCIRCLE\n8\n{}\n10\n{:.5}\n20\n{:.5}\n30\n0.0\n40\n{:.5}\n",
            c.layer, c.c[0], c.c[1], c.r
        );
    }
    s.push_str("0\nENDSEC\n0\nEOF\n");
    s
}

/// SVG with millimetre units (1 user unit = 1 mm), y flipped so +Y is up.
pub fn svg(polys: &[Poly], circles: &[Circle], title: &str) -> String {
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for p in polys {
        for q in p.pts {
            for k in 0..2 {
                lo[k] = lo[k].min(q[k]);
                hi[k] = hi[k].max(q[k]);
            }
        }
    }
    for c in circles {
        for k in 0..2 {
            lo[k] = lo[k].min(c.c[k] - c.r);
            hi[k] = hi[k].max(c.c[k] + c.r);
        }
    }
    if !lo[0].is_finite() {
        lo = [0.0, 0.0];
        hi = [1.0, 1.0];
    }
    let m = 2.0;
    let (w, h) = (hi[0] - lo[0] + 2.0 * m, hi[1] - lo[1] + 2.0 * m);
    let mut s = String::new();
    let _ = writeln!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.3}mm\" height=\"{h:.3}mm\" viewBox=\"{:.3} {:.3} {w:.3} {h:.3}\">",
        lo[0] - m,
        -hi[1] - m
    );
    let _ = writeln!(s, "<title>{title}</title>");
    let _ = writeln!(s, "<g transform=\"scale(1,-1)\" fill=\"none\" stroke-width=\"0.1\">");
    for p in polys {
        let color = if p.layer == "CUT" { "#000000" } else { "#3987e5" };
        let mut d = String::new();
        for (k, q) in p.pts.iter().enumerate() {
            let _ = write!(d, "{}{:.4},{:.4} ", if k == 0 { "M" } else { "L" }, q[0], q[1]);
        }
        if p.closed {
            d.push('Z');
        }
        let dash = if p.layer == "CUT" { "" } else { " stroke-dasharray=\"0.6 0.4\"" };
        let _ = writeln!(s, "<path id=\"{}\" d=\"{}\" stroke=\"{color}\"{dash}/>", p.layer, d.trim_end());
    }
    for c in circles {
        let color = if c.layer == "CUT" { "#000000" } else { "#3987e5" };
        let dash = if c.layer == "CUT" { "" } else { " stroke-dasharray=\"0.6 0.4\"" };
        let _ = writeln!(
            s,
            "<circle cx=\"{:.4}\" cy=\"{:.4}\" r=\"{:.4}\" stroke=\"{color}\"{dash}/>",
            c.c[0], c.c[1], c.r
        );
    }
    s.push_str("</g>\n</svg>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dxf_and_svg_are_well_formed() {
        let sq = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let polys = [Poly {
            layer: "CUT",
            pts: &sq,
            closed: true,
        }];
        let circles = [Circle {
            layer: "CONSTRUCTION",
            c: [5.0, 5.0],
            r: 2.0,
        }];
        let d = dxf(&polys, &circles);
        assert!(d.starts_with("0\nSECTION") && d.ends_with("0\nEOF\n"));
        assert_eq!(d.matches("\nVERTEX\n").count(), 4);
        assert_eq!(d.matches("\nCIRCLE\n").count(), 1);
        let s = svg(&polys, &circles, "test");
        assert!(s.contains("<path") && s.contains("<circle") && s.trim_end().ends_with("</svg>"));
    }
}
