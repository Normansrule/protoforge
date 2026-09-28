//! STEP (ISO 10303-21 physical file) export of the Iris frame parts as
//! **faceted boundary representations** (AP214 `automotive_design` schema).
//!
//! Each part becomes one `PRODUCT` whose shape is a
//! `FACETED_BREP_SHAPE_REPRESENTATION`: one `FACETED_BREP` per closed shell
//! (the front, for example, is several drafted extrusions that touch on flat
//! faces), each shell a `CLOSED_SHELL` of planar triangular
//! `FACE_SURFACE`s bounded by a `POLY_LOOP`, as ISO 10303-42 requires for
//! faceted B-reps (all loops are poly loops, every face lies on a `PLANE`).
//! Lengths are millimetres (`SI_UNIT(.MILLI.,.METRE.)`) and every part gets
//! its display colour through a `STYLED_ITEM`.
//!
//! A faceted STEP is a mesh in STEP clothing: fine for visualisation,
//! interference checks, quoting and 3-D printing services, but a mold maker
//! cutting steel wants native B-rep with analytic surfaces (cylinders,
//! B-splines) — rebuild the parts in Onshape / FreeCAD for production.
//!
//! References: ISO 10303-21 *Clear text encoding of the exchange structure*;
//! ISO 10303-42 *Geometric and topological representation*; ISO 10303-214
//! *Core data for automotive mechanical design processes*; CAx
//! Implementor Forum, *Recommended Practices for AP203/AP214*.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::cad::mesh::{v_cross, v_len, v_sub};
use crate::cad::{Mesh, Part};
use crate::project::Project;

/// Which frame is used for the parts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepFrame {
    /// Parts positioned as worn (one assembly).
    #[default]
    Assembly,
    /// Parts lying flat in their print / mold orientation.
    Print,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct StepPartStats {
    pub name: String,
    pub shells: usize,
    pub closed_shells: usize,
    pub faces: usize,
    pub points: usize,
    pub volume_mm3: f64,
    /// Euler characteristic χ = V − E + F of each shell (2 for a sphere-like
    /// solid, 2 − 2g for a solid with g through-holes).
    pub euler: Vec<i64>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct StepStats {
    pub parts: Vec<StepPartStats>,
    pub shells: usize,
    pub closed_shells: usize,
    pub faces: usize,
    pub points: usize,
    pub entities: usize,
    pub bytes: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct StepFile {
    pub filename: String,
    pub content: String,
    pub stats: StepStats,
    pub notes: Vec<String>,
}

/// STEP REAL literal: always has a decimal point, e.g. `12.`, `-0.5`, `1.25E-07`.
pub fn real(v: f64) -> String {
    if !v.is_finite() {
        return "0.".into();
    }
    let a = v.abs();
    if a != 0.0 && !(1e-4..1e9).contains(&a) {
        let s = format!("{v:.6E}"); // e.g. 1.250000E-7
        let (m, e) = s.split_once('E').unwrap();
        let mut m = m.to_string();
        while m.ends_with('0') {
            m.pop();
        }
        let exp: i32 = e.parse().unwrap_or(0);
        return format!("{m}E{}{:02}", if exp < 0 { "-" } else { "+" }, exp.abs());
    }
    let mut s = format!("{v:.6}");
    while s.ends_with('0') {
        s.pop();
    }
    if s == "-0." {
        s = "0.".into();
    }
    s
}

/// STEP string literal: apostrophes doubled, non-ASCII replaced (ISO 10303-21
/// needs `\X2\` escapes for them; plain ASCII keeps readers happy).
pub fn sstr(s: &str) -> String {
    let t: String = s.chars().map(|c| if c.is_ascii() && c != '\\' { c } else { '_' }).collect();
    format!("'{}'", t.replace('\'', "''"))
}

struct Writer {
    lines: Vec<String>,
}

impl Writer {
    fn add(&mut self, body: String) -> usize {
        self.lines.push(body);
        self.lines.len()
    }
    fn refs(ids: &[usize]) -> String {
        ids.iter().map(|i| format!("#{i}")).collect::<Vec<_>>().join(",")
    }
}

/// Split a mesh into connected shells. Components are found on the raw
/// index buffer (every drafted extrusion of the CAD generator owns its
/// vertices, and stacked extrusions only *touch* on flat faces), then each
/// shell is welded on its own so coincident vertices inside it are shared.
fn shells(mesh: &Mesh) -> Vec<Mesh> {
    let nt = mesh.triangle_count();
    let nv = mesh.vertex_count();
    let mut parent: Vec<usize> = (0..nv).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    for t in 0..nt {
        let a = mesh.indices[3 * t] as usize;
        for k in 1..3 {
            let b = mesh.indices[3 * t + k] as usize;
            let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
            if ra != rb {
                parent[ra] = rb;
            }
        }
    }
    let mut groups: Vec<(usize, Mesh)> = Vec::new();
    for t in 0..nt {
        let r = find(&mut parent, mesh.indices[3 * t] as usize);
        let gi = match groups.iter().position(|g| g.0 == r) {
            Some(i) => i,
            None => {
                groups.push((r, Mesh::default()));
                groups.len() - 1
            }
        };
        let g = &mut groups[gi].1;
        let ids: Vec<u32> = (0..3).map(|k| g.add_vertex(mesh.vertex(mesh.indices[3 * t + k]))).collect();
        g.add_tri(ids[0], ids[1], ids[2]);
    }
    groups.into_iter().map(|(_, m)| crate::cad::export::weld(&m, 1e-6)).collect()
}

/// Every edge used exactly twice, in opposite directions.
fn shell_closed(m: &Mesh) -> bool {
    let tris: Vec<usize> = (0..m.triangle_count()).collect();
    let mut e: HashMap<(u32, u32), i32> = HashMap::new();
    for &t in &tris {
        let i = &m.indices[3 * t..3 * t + 3];
        for k in 0..3 {
            let (a, b) = (i[k], i[(k + 1) % 3]);
            *e.entry((a.min(b), a.max(b))).or_insert(0) += if a < b { 1 } else { -1 };
        }
    }
    // balanced (+1 −1) and each undirected edge seen exactly twice
    let mut count: HashMap<(u32, u32), usize> = HashMap::new();
    for &t in &tris {
        let i = &m.indices[3 * t..3 * t + 3];
        for k in 0..3 {
            let (a, b) = (i[k], i[(k + 1) % 3]);
            *count.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    e.values().all(|v| *v == 0) && count.values().all(|c| *c == 2)
}

/// V − E + F of a triangle mesh (edges counted once).
fn euler(m: &Mesh) -> i64 {
    let mut edges: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
    for t in m.indices.chunks(3) {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            edges.insert((a.min(b), a.max(b)));
        }
    }
    m.vertex_count() as i64 - edges.len() as i64 + m.triangle_count() as i64
}

fn signed_volume(m: &Mesh) -> f64 {
    (0..m.triangle_count())
        .map(|t| {
            let a = m.vertex(m.indices[3 * t]);
            let b = m.vertex(m.indices[3 * t + 1]);
            let c = m.vertex(m.indices[3 * t + 2]);
            (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0])) / 6.0
        })
        .sum()
}

fn hex_rgb(s: &str) -> (f64, f64, f64) {
    let h = s.trim_start_matches('#');
    if h.len() >= 6 {
        let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).map(|v| v as f64 / 255.0).unwrap_or(0.6);
        (c(0), c(2), c(4))
    } else {
        (0.6, 0.6, 0.62)
    }
}

/// Write named parts to one STEP file.
pub fn write_parts(parts: &[Part], title: &str, author: &str, timestamp: &str) -> (String, StepStats) {
    let mut w = Writer { lines: Vec::new() };
    let app = w.add("APPLICATION_CONTEXT('core data for automotive mechanical design processes')".into());
    w.add(format!(
        "APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2000,#{app})"
    ));
    let pc = w.add(format!("PRODUCT_CONTEXT('',#{app},'mechanical')"));
    let pdc = w.add(format!("PRODUCT_DEFINITION_CONTEXT('part definition',#{app},'design')"));
    let len = w.add("(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.))".into());
    let ang = w.add("(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.))".into());
    let sol = w.add("(NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT())".into());
    let unc = w.add(format!(
        "UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-07),#{len},'distance_accuracy_value','confusion accuracy')"
    ));
    let ctx = w.add(format!(
        "(GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{unc})) GLOBAL_UNIT_ASSIGNED_CONTEXT((#{len},#{ang},#{sol})) REPRESENTATION_CONTEXT('Context #1','3D Context with UNIT and UNCERTAINTY'))"
    ));
    let origin = w.add("CARTESIAN_POINT('',(0.,0.,0.))".into());
    let dz = w.add("DIRECTION('',(0.,0.,1.))".into());
    let dx = w.add("DIRECTION('',(1.,0.,0.))".into());
    let world = w.add(format!("AXIS2_PLACEMENT_3D('',#{origin},#{dz},#{dx})"));
    let mut stats = StepStats::default();
    let mut styled: Vec<usize> = Vec::new();
    for part in parts {
        let shells = shells(&part.mesh);
        let mut ps = StepPartStats {
            name: part.name.clone(),
            ..Default::default()
        };
        let mut breps = Vec::new();
        for (si, m) in shells.iter().enumerate() {
            // Points of this shell.
            let pid: Vec<usize> = (0..m.vertex_count())
                .map(|k| {
                    let v = m.vertex(k as u32);
                    w.add(format!("CARTESIAN_POINT('',({},{},{}))", real(v[0]), real(v[1]), real(v[2])))
                })
                .collect();
            ps.points += m.vertex_count();
            let closed = shell_closed(m);
            let mut faces = Vec::with_capacity(m.triangle_count());
            for t in 0..m.triangle_count() {
                let (ia, ib, ic) = (m.indices[3 * t], m.indices[3 * t + 1], m.indices[3 * t + 2]);
                let (a, b, c) = (m.vertex(ia), m.vertex(ib), m.vertex(ic));
                let n = v_cross(v_sub(b, a), v_sub(c, a));
                let nl = v_len(n);
                let e = v_sub(b, a);
                let el = v_len(e);
                if nl < 1e-12 || el < 1e-12 {
                    continue; // degenerate sliver: no plane
                }
                let nd = w.add(format!(
                    "DIRECTION('',({},{},{}))",
                    real(n[0] / nl),
                    real(n[1] / nl),
                    real(n[2] / nl)
                ));
                let rd = w.add(format!(
                    "DIRECTION('',({},{},{}))",
                    real(e[0] / el),
                    real(e[1] / el),
                    real(e[2] / el)
                ));
                let ax = w.add(format!("AXIS2_PLACEMENT_3D('',#{},#{nd},#{rd})", pid[ia as usize]));
                let pl = w.add(format!("PLANE('',#{ax})"));
                let lp = w.add(format!(
                    "POLY_LOOP('',(#{},#{},#{}))",
                    pid[ia as usize], pid[ib as usize], pid[ic as usize]
                ));
                let bd = w.add(format!("FACE_OUTER_BOUND('',#{lp},.T.)"));
                faces.push(w.add(format!("FACE_SURFACE('',(#{bd}),#{pl},.T.)")));
            }
            ps.faces += faces.len();
            let sh = w.add(format!("CLOSED_SHELL('',({}))", Writer::refs(&faces)));
            breps.push(w.add(format!("FACETED_BREP({},#{sh})", sstr(&format!("{} shell {}", part.name, si + 1)))));
            ps.shells += 1;
            ps.closed_shells += usize::from(closed);
            ps.volume_mm3 += signed_volume(m);
            ps.euler.push(euler(m));
        }
        let mut items = vec![world];
        items.extend(&breps);
        let rep = w.add(format!(
            "FACETED_BREP_SHAPE_REPRESENTATION({},({}),#{ctx})",
            sstr(&part.name),
            Writer::refs(&items)
        ));
        let prod = w.add(format!(
            "PRODUCT({},{},{},(#{pc}))",
            sstr(&part.name),
            sstr(&part.name),
            sstr(&format!("{title} - {}", part.name))
        ));
        w.add(format!("PRODUCT_RELATED_PRODUCT_CATEGORY('part',$,(#{prod}))"));
        let pdf = w.add(format!("PRODUCT_DEFINITION_FORMATION('','',#{prod})"));
        let pd = w.add(format!("PRODUCT_DEFINITION('design','',#{pdf},#{pdc})"));
        let pds = w.add(format!("PRODUCT_DEFINITION_SHAPE('','',#{pd})"));
        w.add(format!("SHAPE_DEFINITION_REPRESENTATION(#{pds},#{rep})"));
        // Colour.
        let (r, g, b) = hex_rgb(&part.color);
        let col = w.add(format!("COLOUR_RGB('',{},{},{})", real(r), real(g), real(b)));
        let fasc = w.add(format!("FILL_AREA_STYLE_COLOUR('',#{col})"));
        let fas = w.add(format!("FILL_AREA_STYLE('',(#{fasc}))"));
        let ssfa = w.add(format!("SURFACE_STYLE_FILL_AREA(#{fas})"));
        let sss = w.add(format!("SURFACE_SIDE_STYLE('',(#{ssfa}))"));
        let ssu = w.add(format!("SURFACE_STYLE_USAGE(.BOTH.,#{sss})"));
        let psa = w.add(format!("PRESENTATION_STYLE_ASSIGNMENT((#{ssu}))"));
        for br in &breps {
            styled.push(w.add(format!("STYLED_ITEM('color',(#{psa}),#{br})")));
        }
        stats.shells += ps.shells;
        stats.closed_shells += ps.closed_shells;
        stats.faces += ps.faces;
        stats.points += ps.points;
        stats.parts.push(ps);
    }
    if !styled.is_empty() {
        w.add(format!(
            "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION('',({}),#{ctx})",
            Writer::refs(&styled)
        ));
    }
    stats.entities = w.lines.len();
    let mut s = String::with_capacity(w.lines.len() * 60);
    s += "ISO-10303-21;\nHEADER;\n";
    s += &format!("FILE_DESCRIPTION(({}),'2;1');\n", sstr(&format!("{title} - faceted B-rep export")));
    s += &format!(
        "FILE_NAME({},{},({}),('Protoforge'),'Protoforge forge-core','Protoforge','');\n",
        sstr(&format!("{title}.step")),
        sstr(timestamp),
        sstr(author)
    );
    s += "FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }'));\nENDSEC;\nDATA;\n";
    for (i, l) in w.lines.iter().enumerate() {
        s += &format!("#{}={};\n", i + 1, l);
    }
    s += "ENDSEC;\nEND-ISO-10303-21;\n";
    stats.bytes = s.len();
    (s, stats)
}

/// STEP file of every frame part of the project.
pub fn step(p: &Project, frame: StepFrame, timestamp: &str) -> StepFile {
    let parts = match frame {
        StepFrame::Assembly => crate::cad::frame_parts(p),
        StepFrame::Print => crate::cad::frame_parts_print(p),
    };
    let base = super::kicad::base(p);
    let ts = if timestamp.is_empty() { "1970-01-01T00:00:00" } else { timestamp };
    let (content, stats) = write_parts(&parts, &base, &p.meta.author, ts);
    let open = stats.shells - stats.closed_shells;
    let mut notes = vec![
        "Faceted B-rep: every face is a flat triangle, so curved surfaces are approximations — good for viewing, fit checks, quotes and printing services; rebuild the parts as native B-rep (Onshape, FreeCAD) before cutting a mold.".to_string(),
        "Units are millimetres. Import in Onshape: Create → Import (or drag the file into the Documents page); in FreeCAD: File → Import, then Part → Convert to solid if needed.".to_string(),
    ];
    if open > 0 {
        notes.push(format!(
            "{open} shell(s) are not watertight in the source mesh; importers may show them as open surfaces."
        ));
    }
    StepFile {
        filename: format!("{base}-frame{}.step", if frame == StepFrame::Print { "-print" } else { "" }),
        content,
        stats,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reals_are_valid_step_literals() {
        assert_eq!(real(1.0), "1.");
        assert_eq!(real(-0.5), "-0.5");
        assert_eq!(real(0.0), "0.");
        assert_eq!(real(-0.0), "0.");
        assert_eq!(real(1.25e-7), "1.25E-07");
        assert_eq!(real(12.3456789), "12.345679");
        assert_eq!(sstr("it's"), "'it''s'");
    }

    fn cube() -> Mesh {
        let mut m = Mesh::default();
        let v: Vec<[f64; 3]> = (0..8)
            .map(|i| [(i & 1) as f64 * 10.0, ((i >> 1) & 1) as f64 * 10.0, ((i >> 2) & 1) as f64 * 10.0])
            .collect();
        for p in &v {
            m.add_vertex(*p);
        }
        // outward-facing quads split into triangles
        let quads = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]];
        for q in quads {
            m.add_tri(q[0], q[1], q[2]);
            m.add_tri(q[0], q[2], q[3]);
        }
        m
    }

    #[test]
    fn cube_is_one_closed_shell_of_12_faces_with_1000_mm3() {
        let part = Part {
            name: "cube".into(),
            mesh: cube(),
            color: "#3987e5".into(),
            explode: [0.0; 3],
        };
        let (s, st) = write_parts(&[part], "cube", "test", "2026-01-01T00:00:00");
        assert_eq!(st.shells, 1);
        assert_eq!(st.closed_shells, 1);
        assert_eq!(st.faces, 12);
        assert_eq!(st.points, 8);
        assert!((st.parts[0].volume_mm3 - 1000.0).abs() < 1e-9);
        assert_eq!(st.parts[0].euler, vec![2]);
        assert!(s.starts_with("ISO-10303-21;"));
        assert!(s.trim_end().ends_with("END-ISO-10303-21;"));
        assert_eq!(s.matches("FACE_SURFACE(").count(), 12);
        // every #reference points to a defined entity
        let defined = s.lines().filter(|l| l.starts_with('#')).count();
        for tok in s
            .split(|c: char| !(c == '#' || c.is_ascii_digit()))
            .filter(|t| t.starts_with('#') && t.len() > 1)
        {
            let n: usize = tok[1..].parse().unwrap();
            assert!(n >= 1 && n <= defined, "dangling {tok}");
        }
        // balanced parentheses on every entity line
        for l in s.lines().filter(|l| l.starts_with('#')) {
            assert_eq!(l.matches('(').count(), l.matches(')').count(), "{l}");
        }
    }

    #[test]
    fn iris_frame_exports_closed_shells_with_the_cad_volume() {
        let p = Project::iris();
        let f = step(&p, StepFrame::Print, "");
        assert_eq!(f.stats.parts.len(), crate::cad::frame_parts_print(&p).len());
        assert!(f.stats.faces > 3000);
        assert_eq!(f.stats.closed_shells, f.stats.shells, "every shell watertight");
        // the front's main shell has the two lens openings: genus 2, χ = −2
        let front = f.stats.parts.iter().find(|x| x.name == "front").unwrap();
        assert!(front.euler.contains(&-2), "{:?}", front.euler);
        // volume matches the CAD module's mass properties
        let vol: f64 = f.stats.parts.iter().map(|x| x.volume_mm3).sum();
        let mass_g = vol / 1000.0 * p.design.frame.density_g_cm3;
        let m = crate::cad::metrics(&p);
        let cad = m.iter().find(|m| m.key == "cad.frame_mass_g").unwrap().value;
        assert!((mass_g - cad).abs() / cad < 0.02, "{mass_g} vs {cad}");
    }
}
