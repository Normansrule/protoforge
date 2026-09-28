//! Mesh exchange formats: binary and ASCII STL, Wavefront OBJ with part
//! groups, and the XML payload of a 3MF package (the browser zips it).
//!
//! * STL: 80-byte header, `u32` triangle count, then per triangle a normal
//!   and three vertices as little-endian `f32` plus a `u16` attribute.
//!   Units are implicitly millimetres (what every slicer assumes).
//! * 3MF: 3MF Consortium, *3D Manufacturing Format Core Specification*
//!   v1.3 — an OPC zip with `[Content_Types].xml`, `_rels/.rels` and
//!   `3D/3dmodel.model` (explicit `unit="millimeter"`, one object per part).

use std::fmt::Write as _;

use super::mesh::Mesh;

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 (RFC 4648) with padding.
pub fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

pub fn stl_binary(mesh: &Mesh, name: &str) -> Vec<u8> {
    let n = mesh.triangle_count();
    let mut out = Vec::with_capacity(84 + n * 50);
    let mut header = format!("Protoforge STL: {name} (mm)").into_bytes();
    header.resize(80, b' ');
    out.extend_from_slice(&header);
    out.extend_from_slice(&(n as u32).to_le_bytes());
    for t in 0..n {
        let nrm = mesh.face_normal(t);
        for v in nrm {
            out.extend_from_slice(&(v as f32).to_le_bytes());
        }
        for p in mesh.tri(t) {
            for v in p {
                out.extend_from_slice(&(v as f32).to_le_bytes());
            }
        }
        out.extend_from_slice(&0u16.to_le_bytes());
    }
    out
}

pub fn stl_ascii(mesh: &Mesh, name: &str) -> String {
    let mut s = String::with_capacity(mesh.triangle_count() * 260);
    let _ = writeln!(s, "solid {name}");
    for t in 0..mesh.triangle_count() {
        let n = mesh.face_normal(t);
        let _ = writeln!(s, "  facet normal {:e} {:e} {:e}", n[0], n[1], n[2]);
        s.push_str("    outer loop\n");
        for p in mesh.tri(t) {
            let _ = writeln!(s, "      vertex {:.5} {:.5} {:.5}", p[0], p[1], p[2]);
        }
        s.push_str("    endloop\n  endfacet\n");
    }
    let _ = writeln!(s, "endsolid {name}");
    s
}

/// Parse a binary or ASCII STL back into a (non-welded) mesh.
pub fn parse_stl(bytes: &[u8]) -> Result<Mesh, String> {
    let is_ascii = bytes.starts_with(b"solid") && std::str::from_utf8(bytes).map(|s| s.contains("facet")).unwrap_or(false);
    let mut m = Mesh::default();
    if is_ascii {
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        for line in text.lines() {
            let l = line.trim();
            if let Some(rest) = l.strip_prefix("vertex") {
                let v: Vec<f64> = rest.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                if v.len() != 3 {
                    return Err(format!("bad vertex line: {l}"));
                }
                let i = m.add_vertex([v[0], v[1], v[2]]);
                m.indices.push(i);
            }
        }
    } else {
        if bytes.len() < 84 {
            return Err("STL too short".into());
        }
        let n = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
        if bytes.len() < 84 + n * 50 {
            return Err("STL truncated".into());
        }
        let f = |o: usize| f32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]) as f64;
        for t in 0..n {
            let base = 84 + t * 50 + 12;
            for k in 0..3 {
                let o = base + k * 12;
                let i = m.add_vertex([f(o), f(o + 4), f(o + 8)]);
                m.indices.push(i);
            }
        }
    }
    if m.indices.len() % 3 != 0 {
        return Err("vertex count is not a multiple of 3".into());
    }
    Ok(m)
}

/// Merge coincident vertices (tolerance `tol`), e.g. after STL import.
pub fn weld(mesh: &Mesh, tol: f64) -> Mesh {
    use std::collections::HashMap;
    let mut map: HashMap<(i64, i64, i64), u32> = HashMap::new();
    let mut out = Mesh::default();
    let mut remap = Vec::with_capacity(mesh.vertex_count());
    for k in 0..mesh.vertex_count() {
        let p = mesh.vertex(k as u32);
        let key = (
            (p[0] / tol).round() as i64,
            (p[1] / tol).round() as i64,
            (p[2] / tol).round() as i64,
        );
        let idx = *map.entry(key).or_insert_with(|| out.add_vertex(p));
        remap.push(idx);
    }
    for t in mesh.indices.chunks(3) {
        let (a, b, c) = (remap[t[0] as usize], remap[t[1] as usize], remap[t[2] as usize]);
        if a != b && b != c && a != c {
            out.add_tri(a, b, c);
        }
    }
    out
}

/// Wavefront OBJ with one `o`/`g` group per part (1-based indices).
pub fn obj(parts: &[(&str, &Mesh)]) -> String {
    let mut s = String::new();
    s.push_str("# Protoforge OBJ export (units: mm, Z up)\n");
    let mut base = 1usize;
    for (name, m) in parts {
        let _ = writeln!(s, "o {name}\ng {name}");
        for p in m.positions.chunks(3) {
            let _ = writeln!(s, "v {:.5} {:.5} {:.5}", p[0], p[1], p[2]);
        }
        for t in m.indices.chunks(3) {
            let _ = writeln!(s, "f {} {} {}", t[0] as usize + base, t[1] as usize + base, t[2] as usize + base);
        }
        base += m.vertex_count();
    }
    s
}

/// The three text files of a 3MF package: (path, content).
pub fn three_mf(parts: &[(&str, &Mesh, &str)], title: &str) -> Vec<(String, String)> {
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/>
</Types>
"#
    .to_string();
    let rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/>
</Relationships>
"#
    .to_string();
    let mut m = String::new();
    m.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    m.push_str("<model unit=\"millimeter\" xml:lang=\"en-US\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\">\n");
    let _ = writeln!(m, "  <metadata name=\"Title\">{}</metadata>", xml_escape(title));
    m.push_str("  <metadata name=\"Application\">Protoforge</metadata>\n  <resources>\n    <basematerials id=\"1\">\n");
    for (name, _, color) in parts {
        let _ = writeln!(
            m,
            "      <base name=\"{}\" displaycolor=\"{}\"/>",
            xml_escape(name),
            color.to_uppercase()
        );
    }
    m.push_str("    </basematerials>\n");
    for (i, (name, mesh, _)) in parts.iter().enumerate() {
        let _ = writeln!(
            m,
            "    <object id=\"{}\" name=\"{}\" type=\"model\" pid=\"1\" pindex=\"{}\">\n      <mesh>\n        <vertices>",
            i + 2,
            xml_escape(name),
            i
        );
        for p in mesh.positions.chunks(3) {
            let _ = writeln!(m, "          <vertex x=\"{:.4}\" y=\"{:.4}\" z=\"{:.4}\"/>", p[0], p[1], p[2]);
        }
        m.push_str("        </vertices>\n        <triangles>\n");
        for t in mesh.indices.chunks(3) {
            let _ = writeln!(m, "          <triangle v1=\"{}\" v2=\"{}\" v3=\"{}\"/>", t[0], t[1], t[2]);
        }
        m.push_str("        </triangles>\n      </mesh>\n    </object>\n");
    }
    m.push_str("  </resources>\n  <build>\n");
    for i in 0..parts.len() {
        let _ = writeln!(m, "    <item objectid=\"{}\"/>", i + 2);
    }
    m.push_str("  </build>\n</model>\n");
    vec![
        ("[Content_Types].xml".into(), content_types),
        ("_rels/.rels".into(), rels),
        ("3D/3dmodel.model".into(), m),
    ]
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::solid::box_mesh;

    #[test]
    fn base64_known_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn stl_round_trip() {
        let m = box_mesh([0.0; 3], [10.0, 20.0, 5.0]);
        let bin = stl_binary(&m, "box");
        assert_eq!(bin.len(), 84 + 12 * 50);
        let back = weld(&parse_stl(&bin).unwrap(), 1e-4);
        assert!(back.validate().watertight);
        assert!((back.volume() - 1000.0).abs() < 1e-3);
        let txt = stl_ascii(&m, "box");
        let back = weld(&parse_stl(txt.as_bytes()).unwrap(), 1e-4);
        assert!((back.volume() - 1000.0).abs() < 1e-3);
    }

    #[test]
    fn obj_and_3mf_are_well_formed() {
        let m = box_mesh([0.0; 3], [1.0; 3]);
        let o = obj(&[("a", &m), ("b", &m)]);
        assert!(o.contains("o b") && o.lines().filter(|l| l.starts_with("f ")).count() == 24);
        assert!(o.contains("f 9 "), "second part indices are offset");
        let files = three_mf(&[("a", &m, "#ff0000")], "t");
        assert_eq!(files.len(), 3);
        assert!(files[2].1.contains("unit=\"millimeter\"") && files[2].1.contains("<triangle v1"));
    }
}
