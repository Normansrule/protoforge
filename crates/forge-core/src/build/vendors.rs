//! "Where to send it" — the Manufacturing Hub's vendor directory
//! (`crate::mfg::vendors`, read-only) extended with more categories, the
//! file formats each service accepts and which Protoforge output to upload.
//!
//! Facts only: company names, public websites and service categories. No
//! prices, lead times, ratings or endorsements. Categories where naming one
//! company would be a recommendation (battery cells, cable assemblies,
//! contract manufacturers) are described as categories instead.

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Output {
    pub label: &'static str,
    /// Lab route that produces the file.
    pub route: &'static str,
}

const fn o(label: &'static str, route: &'static str) -> Output {
    Output { label, route }
}

#[derive(Clone, Debug, Serialize)]
pub struct Vendor {
    pub name: String,
    /// Empty for category cards.
    pub url: String,
    pub categories: Vec<&'static str>,
    pub services: String,
    /// File formats the service's upload / quote flow accepts.
    pub formats: Vec<&'static str>,
    /// Protoforge outputs to send.
    pub send: Vec<Output>,
    /// What comes back.
    pub returns: &'static str,
    pub region: String,
    /// true for a category description rather than a named company.
    pub category_card: bool,
    pub note: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct Category {
    pub id: &'static str,
    pub label: &'static str,
}

pub const CATEGORIES: &[Category] = &[
    Category {
        id: "pcb",
        label: "PCB fabrication",
    },
    Category {
        id: "pcba",
        label: "PCB assembly",
    },
    Category {
        id: "parts",
        label: "Electronic parts",
    },
    Category {
        id: "print",
        label: "3D printing",
    },
    Category {
        id: "cnc",
        label: "CNC machining",
    },
    Category {
        id: "sheet",
        label: "Sheet metal",
    },
    Category {
        id: "laser",
        label: "Laser cutting",
    },
    Category {
        id: "urethane",
        label: "Urethane casting",
    },
    Category {
        id: "molding",
        label: "Molding & casting",
    },
    Category {
        id: "optics",
        label: "Optics",
    },
    Category {
        id: "enclosure",
        label: "Enclosures",
    },
    Category {
        id: "battery",
        label: "Batteries",
    },
    Category {
        id: "cable",
        label: "Cables",
    },
    Category {
        id: "test",
        label: "Test & certification",
    },
    Category {
        id: "cm",
        label: "Contract manufacturing",
    },
];

const GERBER: Output = o("Gerber + drill zip (PCB Studio / fab package)", "/pcb");
const BOM: Output = o("BOM.csv", "/pcb");
const CPL: Output = o("CPL (pick-and-place) .csv", "/pcb");
const STL: Output = o("STL per part (CAD Studio)", "/cad");
const STEP_VIA: Output = o("STEP via Onshape / FreeCAD from the CAD geometry", "/cad");
const DRAWING: Output = o("2D drawing (PDF) with tolerances", "/cad");
const RFQ: Output = o("Molding RFQ + DFM report (Molding Lab / fab package)", "/molding");
const COMPLIANCE: Output = o("Compliance plan (this page, Compliance tab)", "/build");

/// Extra facts layered onto a Manufacturing Hub vendor (matched by name).
struct Ext {
    name: &'static str,
    categories: &'static [&'static str],
    formats: &'static [&'static str],
    send: &'static [Output],
    returns: &'static str,
}

const EXT: &[Ext] = &[
    Ext {
        name: "JLCPCB",
        categories: &["pcb", "pcba"],
        formats: &["Gerber RS-274X + Excellon drill (zip)", "BOM (CSV/XLS)", "CPL / centroid (CSV)"],
        send: &[GERBER, BOM, CPL],
        returns: "Bare or assembled boards, stencils",
    },
    Ext {
        name: "PCBWay",
        categories: &["pcb", "pcba", "print", "cnc", "molding"],
        formats: &["Gerber + drill (zip)", "BOM", "centroid (CPL)", "STEP / STL for mechanical parts"],
        send: &[GERBER, BOM, CPL, STL],
        returns: "Boards, assemblies, mechanical parts",
    },
    Ext {
        name: "OSH Park",
        categories: &["pcb"],
        formats: &["Gerber + drill (zip)", "KiCad .kicad_pcb (native upload)"],
        send: &[GERBER],
        returns: "Bare prototype boards",
    },
    Ext {
        name: "Aisler",
        categories: &["pcb", "pcba"],
        formats: &[
            "Gerber + drill (zip)",
            "KiCad project (native upload / plugin)",
            "BOM + CPL for assembly",
        ],
        send: &[GERBER, BOM, CPL],
        returns: "Boards, stencils, assemblies",
    },
    Ext {
        name: "Eurocircuits",
        categories: &["pcb", "pcba"],
        formats: &["Gerber + drill", "BOM", "CPL"],
        send: &[GERBER, BOM, CPL],
        returns: "Boards and assemblies (Europe)",
    },
    Ext {
        name: "Seeed Fusion",
        categories: &["pcb", "pcba"],
        formats: &["Gerber + drill (zip)", "BOM", "CPL"],
        send: &[GERBER, BOM, CPL],
        returns: "Boards and assemblies",
    },
    Ext {
        name: "LCSC",
        categories: &["parts"],
        formats: &["BOM upload (CSV/XLS)"],
        send: &[BOM],
        returns: "Components (also stocks JLCPCB's assembly library)",
    },
    Ext {
        name: "Digi-Key",
        categories: &["parts"],
        formats: &["BOM upload (CSV/XLS) via its BOM tool"],
        send: &[BOM],
        returns: "Components",
    },
    Ext {
        name: "Mouser",
        categories: &["parts"],
        formats: &["BOM upload (CSV/XLS) via its BOM tool"],
        send: &[BOM],
        returns: "Components",
    },
    Ext {
        name: "Protolabs",
        categories: &["molding", "cnc", "print", "sheet"],
        formats: &["STEP (and many native CAD formats)", "2D drawing (PDF)"],
        send: &[STEP_VIA, DRAWING, RFQ],
        returns: "Molded, machined, printed and sheet parts; automated DFM feedback",
    },
    Ext {
        name: "Xometry",
        categories: &["molding", "cnc", "print", "urethane", "sheet"],
        formats: &["STEP", "STL", "2D drawing (PDF)"],
        send: &[STEP_VIA, STL, DRAWING, RFQ],
        returns: "Parts from a manufacturing network",
    },
    Ext {
        name: "Fictiv",
        categories: &["molding", "cnc", "print", "urethane"],
        formats: &["STEP", "2D drawing (PDF)"],
        send: &[STEP_VIA, DRAWING, RFQ],
        returns: "Machined, molded and printed parts",
    },
    Ext {
        name: "JLC3DP",
        categories: &["print"],
        formats: &["STL", "STEP"],
        send: &[STL],
        returns: "SLA, MJF, SLS, FDM and metal prints",
    },
    Ext {
        name: "Craftcloud",
        categories: &["print"],
        formats: &["STL"],
        send: &[STL],
        returns: "Quotes from many print services",
    },
    Ext {
        name: "SendCutSend",
        categories: &["laser", "sheet"],
        formats: &["DXF and other 2D vector files", "STEP for bent parts"],
        send: &[o("DXF flat pattern / outline", "/cad"), STEP_VIA],
        returns: "Laser / waterjet-cut and bent sheet parts",
    },
];

fn new_vendors() -> Vec<Vendor> {
    let v = |name: &str,
             url: &str,
             cats: &[&'static str],
             services: &str,
             formats: &[&'static str],
             send: Vec<Output>,
             returns: &'static str,
             region: &str| Vendor {
        name: name.into(),
        url: url.into(),
        categories: cats.to_vec(),
        services: services.into(),
        formats: formats.to_vec(),
        send,
        returns,
        region: region.into(),
        category_card: false,
        note: "",
    };
    let lab = |name: &str, url: &str, services: &str, region: &str| Vendor {
        note: "Ask for pre-compliance scans at DVT and formal testing at PVT; bring the compliance plan.",
        ..v(
            name,
            url,
            &["test"],
            services,
            &[
                "Product description, block diagram, BOM, user manual draft",
                "Test samples (radiated / conducted builds)",
            ],
            vec![COMPLIANCE, BOM],
            "Test reports, certificates, declarations support",
            region,
        )
    };
    let cat = |name: &str, cats: &[&'static str], services: &str, send: Vec<Output>, note: &'static str| Vendor {
        category_card: true,
        note,
        ..v(name, "", cats, services, &[], send, "", "Any")
    };
    vec![
        v("Arrow Electronics", "https://www.arrow.com", &["parts"], "Electronic component distributor", &["BOM upload (CSV/XLS)"], vec![BOM], "Components", "Global"),
        v("Hubs (Protolabs network)", "https://www.hubs.com", &["cnc", "print", "sheet", "molding"], "Manufacturing network: CNC machining, 3D printing, sheet metal, injection molding", &["STEP", "STL", "2D drawing (PDF)"], vec![STEP_VIA, STL, DRAWING], "Parts from partner factories", "Europe / USA"),
        v("Edmund Optics", "https://www.edmundoptics.com", &["optics"], "Catalogue optics — lenses, beamsplitters, mirrors, filters — and custom optics", &["Catalogue part selection", "Optical drawing / prescription for custom parts"], vec![o("Lens prescription and layout (Optics Lab)", "/optics")], "Lenses, beamsplitters, test optics", "Global"),
        v("Thorlabs", "https://www.thorlabs.com", &["optics"], "Optics, optomechanics and breadboards for building an optical test bench", &["Catalogue part selection"], vec![o("Optical layout (Optics Lab)", "/optics")], "Optics, mounts, test equipment", "Global"),
        v("Hammond Manufacturing", "https://www.hammfg.com", &["enclosure"], "Standard plastic and die-cast aluminium enclosures (plus electrical products)", &["Catalogue part selection; modification drawings (PDF/DXF) for machined cut-outs"], vec![DRAWING], "Stock or modified enclosures for test fixtures and dev kits", "North America / Europe"),
        lab("UL Solutions", "https://www.ul.com", "Safety (IEC/UL 62368-1), batteries, EMC and wireless testing and certification", "Global"),
        lab("Intertek", "https://www.intertek.com", "Product safety, EMC and wireless testing and certification", "Global"),
        lab("TÜV SÜD", "https://www.tuvsud.com", "Testing, inspection and certification incl. EMC, radio and safety", "Global"),
        lab("TÜV Rheinland", "https://www.tuv.com", "Testing, inspection and certification incl. EMC, radio and safety", "Global"),
        lab("SGS", "https://www.sgs.com", "Testing, inspection and certification incl. RoHS / REACH chemical testing", "Global"),
        lab("Element Materials Technology", "https://www.element.com", "EMC, wireless, environmental and materials testing", "Global"),
        cat(
            "Battery cells and packs (category)",
            &["battery"],
            "Buy lithium cells from the authorised distributors above or the cell maker's channel — never unbranded cells. Choose a cell with a protection circuit (overcharge, over-discharge, over-current, short circuit) and ask for the UN 38.3 test summary and IEC 62133-2 report before you buy.",
            vec![o("Battery requirements (Power & Thermal)", "/power"), COMPLIANCE],
            "A cell without a UN 38.3 test summary cannot legally be shipped by most carriers.",
        ),
        cat(
            "Cables and harnesses (category)",
            &["cable"],
            "Custom flat-flex cables (FFC/FPC), wire harnesses and USB cables are made by cable-assembly houses; many PCB assemblers also build flex circuits and harnesses. Send a drawing with lengths, connectors, pin-out and bend radius.",
            vec![o("Interface list and pin-out (Systems Studio)", "/systems"), DRAWING],
            "The Iris hinge flex (FPC1) is a flex-life risk — ask for bend-cycle data.",
        ),
        cat(
            "Contract manufacturers (category)",
            &["cm"],
            "Electronics manufacturing services (EMS) and contract manufacturers (CMs) take a product from PVT to mass production: sourcing, PCBA, final assembly, test fixtures, packaging and logistics. Choose by product type, volume, region and certifications (e.g. ISO 9001, ISO 13485 for medical); visit the line and run a pilot build first.",
            vec![o("Fabrication package (Manufacturing Hub)", "/mfg"), COMPLIANCE, o("Verification report (Systems Studio)", "/systems")],
            "Protect your design with an NDA and keep ownership of tools and test fixtures in the contract.",
        ),
    ]
}

/// The full directory: Manufacturing Hub vendors (extended) + new ones.
pub fn directory() -> Vec<Vendor> {
    let mut out: Vec<Vendor> = crate::mfg::vendors::VENDORS
        .iter()
        .map(|mv| {
            let ext = EXT.iter().find(|e| e.name == mv.name);
            Vendor {
                name: mv.name.to_string(),
                url: mv.url.to_string(),
                categories: ext.map(|e| e.categories.to_vec()).unwrap_or_else(|| mv.categories.to_vec()),
                services: mv.services.to_string(),
                formats: ext.map(|e| e.formats.to_vec()).unwrap_or_else(|| mv.consumes.to_vec()),
                send: ext.map(|e| e.send.to_vec()).unwrap_or_default(),
                returns: ext.map(|e| e.returns).unwrap_or(""),
                region: mv.region.to_string(),
                category_card: false,
                note: "",
            }
        })
        .collect();
    out.extend(new_vendors());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_extends_mfg_list() {
        let d = directory();
        for mv in crate::mfg::vendors::VENDORS {
            assert!(d.iter().any(|v| v.name == mv.name), "{}", mv.name);
        }
        // every mfg vendor has an extension record (formats + outputs)
        for mv in crate::mfg::vendors::VENDORS {
            assert!(EXT.iter().any(|e| e.name == mv.name), "no extension for {}", mv.name);
        }
        for v in &d {
            assert!(v.category_card || v.url.starts_with("https://"), "{}", v.name);
            assert!(v.categories.iter().all(|c| CATEGORIES.iter().any(|k| k.id == *c)), "{}", v.name);
        }
        for c in CATEGORIES {
            assert!(d.iter().any(|v| v.categories.contains(&c.id)), "empty category {}", c.id);
        }
    }
}
