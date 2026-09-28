//! Optical materials: refractive index vs wavelength.
//!
//! **Glasses** use the three-term Sellmeier formula of the SCHOTT optical
//! glass datasheets (λ in µm):
//!
//! ```text
//! n²(λ) − 1 = B₁λ²/(λ² − C₁) + B₂λ²/(λ² − C₂) + B₃λ²/(λ² − C₃)
//! ```
//!
//! with the published coefficients for N-BK7, N-SF11, N-SF6 and N-F2 (SCHOTT
//! datasheets)
//! and for fused silica (I. H. Malitson, "Interspecimen comparison of the
//! refractive index of fused silica", *J. Opt. Soc. Am.* 55, 1205 (1965)).
//! The same numbers are tabulated on refractiveindex.info.
//!
//! **Optical plastics** (PMMA/acrylic, polycarbonate, cyclo-olefin polymer
//! (COP, e.g. Zeonex E48R), polystyrene) vary from resin grade to resin grade
//! and moulding conditions, so we do not pretend to a grade-specific
//! Sellmeier fit. Instead we use a two-term **Cauchy** formula
//! `n(λ) = A + B/λ²` whose two coefficients reproduce the *typical* catalogue
//! refractive index nd and Abbe number Vd exactly (nd at the helium d line,
//! Vd = (nd − 1)/(nF − nC)). Check the resin supplier's datasheet before
//! committing a mould.
//!
//! Characteristic lines (Fraunhofer): F = 486.1327 nm (hydrogen), d =
//! 587.5618 nm (helium), C = 656.2725 nm (hydrogen).
//!
//! References: SCHOTT, *Optical Glass — Data Sheets* and TIE-29 "Refractive
//! Index and Dispersion"; W. J. Smith, *Modern Optical Engineering* (4th ed.,
//! 2008), ch. 7; M. Born & E. Wolf, *Principles of Optics* (7th ed., 1999),
//! §2.3; refractiveindex.info (<https://refractiveindex.info>).

use serde::{Deserialize, Serialize};

/// Fraunhofer F line (hydrogen), nm.
pub const LAMBDA_F: f64 = 486.1327;
/// Helium d line, nm (the reference wavelength for nd and Vd).
pub const LAMBDA_D: f64 = 587.5618;
/// Fraunhofer C line (hydrogen), nm.
pub const LAMBDA_C: f64 = 656.2725;

/// How a material's index depends on wavelength.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "formula", rename_all = "snake_case")]
pub enum Dispersion {
    /// Three-term Sellmeier, λ in µm, C in µm².
    Sellmeier { b: [f64; 3], c: [f64; 3] },
    /// Cauchy `A + B/λ²`, λ in µm.
    Cauchy { a: f64, b: f64 },
    /// Wavelength-independent index (air, ideal media).
    Constant { n: f64 },
}

impl Dispersion {
    /// Refractive index at wavelength `nm`.
    pub fn n(&self, nm: f64) -> f64 {
        let l = nm * 1e-3;
        let l2 = l * l;
        match self {
            Dispersion::Sellmeier { b, c } => {
                let s: f64 = (0..3).map(|k| b[k] * l2 / (l2 - c[k])).sum();
                (1.0 + s).sqrt()
            }
            Dispersion::Cauchy { a, b } => a + b / l2,
            Dispersion::Constant { n } => *n,
        }
    }

    /// Two-term Cauchy fitted exactly to a catalogue nd and Vd.
    pub fn from_nd_vd(nd: f64, vd: f64) -> Dispersion {
        let (lf, ld, lc) = (LAMBDA_F * 1e-3, LAMBDA_D * 1e-3, LAMBDA_C * 1e-3);
        let dn = (nd - 1.0) / vd.max(1e-6);
        let b = dn / (1.0 / (lf * lf) - 1.0 / (lc * lc));
        Dispersion::Cauchy { a: nd - b / (ld * ld), b }
    }
}

/// A catalogue material.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Material {
    pub id: String,
    pub name: String,
    /// "glass" or "plastic"
    pub kind: String,
    pub dispersion: Dispersion,
    /// Catalogue (datasheet or typical) nd and Vd the computed values are checked against.
    pub nd_catalog: f64,
    pub vd_catalog: f64,
    /// Typical density, g/cm³ (for lens mass).
    pub density_g_cm3: f64,
    /// Where the dispersion data comes from.
    pub source: String,
    pub notes: String,
}

impl Material {
    pub fn n(&self, nm: f64) -> f64 {
        self.dispersion.n(nm)
    }
    pub fn nd(&self) -> f64 {
        self.n(LAMBDA_D)
    }
    /// Abbe number Vd = (nd − 1)/(nF − nC).
    pub fn vd(&self) -> f64 {
        (self.nd() - 1.0) / (self.n(LAMBDA_F) - self.n(LAMBDA_C))
    }
}

#[allow(clippy::too_many_arguments)]
fn sellmeier(id: &str, name: &str, b: [f64; 3], c: [f64; 3], nd: f64, vd: f64, rho: f64, source: &str, notes: &str) -> Material {
    Material {
        id: id.into(),
        name: name.into(),
        kind: "glass".into(),
        dispersion: Dispersion::Sellmeier { b, c },
        nd_catalog: nd,
        vd_catalog: vd,
        density_g_cm3: rho,
        source: source.into(),
        notes: notes.into(),
    }
}

fn plastic(id: &str, name: &str, nd: f64, vd: f64, rho: f64, notes: &str) -> Material {
    Material {
        id: id.into(),
        name: name.into(),
        kind: "plastic".into(),
        dispersion: Dispersion::from_nd_vd(nd, vd),
        nd_catalog: nd,
        vd_catalog: vd,
        density_g_cm3: rho,
        source: "Cauchy fit to typical catalogue nd / Vd (grade-dependent: check the resin datasheet)".into(),
        notes: notes.into(),
    }
}

/// The built-in catalogue.
pub fn catalog() -> Vec<Material> {
    let fs = |x: f64| x * x;
    vec![
        sellmeier(
            "N-BK7",
            "SCHOTT N-BK7 (borosilicate crown)",
            [1.03961212, 0.231792344, 1.01046945],
            [0.00600069867, 0.0200179144, 103.560653],
            1.5168,
            64.17,
            2.51,
            "SCHOTT datasheet Sellmeier coefficients",
            "The workhorse crown glass: cheap, stable, the crown half of most achromats.",
        ),
        sellmeier(
            "N-SF11",
            "SCHOTT N-SF11 (dense flint)",
            [1.73759695, 0.313747346, 1.89878101],
            [0.013188707, 0.0623068142, 155.23629],
            1.78472,
            25.68,
            3.22,
            "SCHOTT datasheet Sellmeier coefficients",
            "High index, high dispersion: the flint half of a compact achromat (lead-free, but 2.7× the density of PMMA).",
        ),
        sellmeier(
            "N-SF6",
            "SCHOTT N-SF6 (dense flint)",
            [1.77931763, 0.338149866, 2.08734474],
            [0.0133714182, 0.0617533621, 174.01759],
            1.80518,
            25.36,
            3.37,
            "SCHOTT datasheet Sellmeier coefficients",
            "One of the highest-index common flints; strong negative elements stay thin.",
        ),
        sellmeier(
            "N-F2",
            "SCHOTT N-F2 (flint)",
            [1.39757037, 0.159201403, 1.2686543],
            [0.00995906143, 0.0546931752, 119.248346],
            1.62005,
            36.43,
            2.65,
            "SCHOTT datasheet Sellmeier coefficients",
            "A light, moderate flint: gentler curves than N-SF11 in an achromat, but more secondary colour correction work.",
        ),
        sellmeier(
            "F_SILICA",
            "Fused silica (SiO₂)",
            [0.6961663, 0.4079426, 0.8974794],
            [fs(0.0684043), fs(0.1162414), fs(9.896161)],
            1.4585,
            67.82,
            2.20,
            "Malitson (1965), J. Opt. Soc. Am. 55, 1205",
            "Very low dispersion and expansion; UV-transparent; hard to mould.",
        ),
        plastic(
            "PMMA",
            "PMMA (acrylic)",
            1.4918,
            57.4,
            1.19,
            "The 'crown' of optical plastics: clear, cheap, easy to injection-mould; absorbs some water.",
        ),
        plastic(
            "PC",
            "Polycarbonate (PC)",
            1.5855,
            29.9,
            1.20,
            "The 'flint' of optical plastics: tough, high dispersion — pairs with PMMA in plastic achromats.",
        ),
        plastic(
            "COP",
            "Cyclo-olefin polymer (COP, e.g. Zeonex E48R)",
            1.531,
            56.0,
            1.01,
            "Low water absorption and birefringence: the usual choice for moulded camera and HMD lenses.",
        ),
        plastic(
            "PS",
            "Polystyrene (PS)",
            1.5905,
            30.9,
            1.05,
            "Cheap flint-like plastic; more birefringent and less weather-stable than PC.",
        ),
    ]
}

/// Look a material up by catalogue id (case-insensitive). `""`/`"air"`
/// return `None` (index 1). A model glass can be written `"nd/vd"`, e.g.
/// `"1.5168/64.17"`, and uses the two-term Cauchy fit.
pub fn lookup(id: &str) -> Result<Option<Material>, String> {
    let t = id.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("air") {
        return Ok(None);
    }
    if let Some(m) = catalog().into_iter().find(|m| m.id.eq_ignore_ascii_case(t)) {
        return Ok(Some(m));
    }
    if let Some((a, b)) = t.split_once('/') {
        if let (Ok(nd), Ok(vd)) = (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
            if nd >= 1.0 && vd > 0.0 {
                return Ok(Some(Material {
                    id: t.into(),
                    name: format!("Model glass nd {nd} Vd {vd}"),
                    kind: "model".into(),
                    dispersion: Dispersion::from_nd_vd(nd, vd),
                    nd_catalog: nd,
                    vd_catalog: vd,
                    density_g_cm3: 2.5,
                    source: "model glass (Cauchy fit to nd, Vd)".into(),
                    notes: String::new(),
                }));
            }
        }
    }
    Err(format!(
        "unknown material `{t}` (use a catalogue id such as N-BK7, PMMA, PC, COP, or a model glass `nd/vd`)"
    ))
}

/// Index of a material id at a wavelength (air → 1).
pub fn index_of(id: &str, nm: f64) -> Result<f64, String> {
    Ok(lookup(id)?.map(|m| m.n(nm)).unwrap_or(1.0))
}

/// Catalogue row for the Materials tab.
#[derive(Clone, Debug, Serialize)]
pub struct MaterialRow {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub nd: f64,
    pub vd: f64,
    pub nf: f64,
    pub nc: f64,
    pub nd_catalog: f64,
    pub vd_catalog: f64,
    pub density_g_cm3: f64,
    pub source: String,
    pub notes: String,
    /// Dispersion curve n(λ) sampled at `wavelengths_nm`.
    pub curve: Vec<f64>,
    pub dispersion: Dispersion,
}

#[derive(Clone, Debug, Serialize)]
pub struct CatalogOut {
    pub wavelengths_nm: Vec<f64>,
    pub materials: Vec<MaterialRow>,
    pub lines_nm: [f64; 3],
}

/// Catalogue with nd, Vd and n(λ) curves from 400 to 800 nm.
pub fn catalog_table() -> CatalogOut {
    let wl: Vec<f64> = (0..=80).map(|k| 400.0 + 5.0 * k as f64).collect();
    let materials = catalog()
        .into_iter()
        .map(|m| MaterialRow {
            nd: m.nd(),
            vd: m.vd(),
            nf: m.n(LAMBDA_F),
            nc: m.n(LAMBDA_C),
            curve: wl.iter().map(|&l| m.n(l)).collect(),
            id: m.id.clone(),
            name: m.name.clone(),
            kind: m.kind.clone(),
            nd_catalog: m.nd_catalog,
            vd_catalog: m.vd_catalog,
            density_g_cm3: m.density_g_cm3,
            source: m.source.clone(),
            notes: m.notes.clone(),
            dispersion: m.dispersion.clone(),
        })
        .collect();
    CatalogOut {
        wavelengths_nm: wl,
        materials,
        lines_nm: [LAMBDA_F, LAMBDA_D, LAMBDA_C],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(id: &str) -> Material {
        lookup(id).unwrap().unwrap()
    }

    #[test]
    fn schott_glasses_match_the_datasheets() {
        // N-BK7: nd = 1.51680, Vd = 64.17 (SCHOTT)
        let bk7 = get("N-BK7");
        assert!((bk7.nd() - 1.5168).abs() < 2e-5, "{}", bk7.nd());
        assert!((bk7.vd() - 64.17).abs() < 0.02, "{}", bk7.vd());
        // N-SF11: nd = 1.78472, Vd = 25.68
        let sf11 = get("n-sf11");
        assert!((sf11.nd() - 1.78472).abs() < 2e-5, "{}", sf11.nd());
        assert!((sf11.vd() - 25.68).abs() < 0.02, "{}", sf11.vd());
        // N-SF6: nd = 1.80518, Vd = 25.36; N-F2: nd = 1.62005, Vd = 36.43
        let sf6 = get("N-SF6");
        assert!((sf6.nd() - 1.80518).abs() < 2e-5, "{}", sf6.nd());
        assert!((sf6.vd() - 25.36).abs() < 0.02, "{}", sf6.vd());
        let f2 = get("N-F2");
        assert!((f2.nd() - 1.62005).abs() < 2e-5, "{}", f2.nd());
        assert!((f2.vd() - 36.43).abs() < 0.02, "{}", f2.vd());
        // fused silica: nd = 1.45846, Vd = 67.8
        let fs = get("F_SILICA");
        assert!((fs.nd() - 1.45846).abs() < 2e-5, "{}", fs.nd());
        assert!((fs.vd() - 67.82).abs() < 0.05, "{}", fs.vd());
        // BK7 at 1064 nm is about 1.5066 (datasheet table)
        assert!((bk7.n(1064.0) - 1.5066).abs() < 3e-4);
    }

    #[test]
    fn plastics_reproduce_catalogue_values_and_normal_dispersion() {
        for m in catalog() {
            assert!((m.nd() - m.nd_catalog).abs() < 1e-4, "{} nd {}", m.id, m.nd());
            assert!((m.vd() - m.vd_catalog).abs() / m.vd_catalog < 0.005, "{} vd {}", m.id, m.vd());
            // normal dispersion: index falls with wavelength
            assert!(m.n(450.0) > m.n(550.0) && m.n(550.0) > m.n(650.0), "{}", m.id);
        }
        let pmma = get("PMMA");
        assert!(pmma.nd() > 1.49 && pmma.nd() < 1.493);
        let pc = get("PC");
        assert!(pc.vd() < pmma.vd() / 1.5, "PC is the high-dispersion partner");
    }

    #[test]
    fn model_glass_and_air() {
        assert_eq!(index_of("air", 500.0).unwrap(), 1.0);
        assert_eq!(index_of("", 500.0).unwrap(), 1.0);
        let m = get("1.6/40");
        assert!((m.nd() - 1.6).abs() < 1e-12);
        assert!((m.vd() - 40.0).abs() < 1e-9);
        assert!(lookup("unobtainium").is_err());
    }
}
