//! Manufacturing-service directory. Facts only: company name, public
//! website, the kind of service and which Protoforge outputs the service
//! consumes. No prices, lead times or part numbers are stored here — those
//! change daily; get them from the vendor's own quote page.

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Vendor {
    pub name: &'static str,
    pub url: &'static str,
    /// pcb | parts | print | molding | cnc
    pub categories: &'static [&'static str],
    pub services: &'static str,
    /// Protoforge files this vendor's upload flow expects.
    pub consumes: &'static [&'static str],
    pub region: &'static str,
}

pub const VENDORS: &[Vendor] = &[
    Vendor { name: "JLCPCB", url: "https://jlcpcb.com", categories: &["pcb"], services: "PCB fabrication, PCB assembly (SMT) and SMT stencils", consumes: &["Gerber + drill zip", "BOM.csv", "CPL.csv"], region: "China" },
    Vendor { name: "PCBWay", url: "https://www.pcbway.com", categories: &["pcb", "print", "molding", "cnc"], services: "PCB fabrication and assembly, 3D printing, CNC machining, injection molding", consumes: &["Gerber + drill zip", "BOM.csv", "CPL.csv", "STEP/STL"], region: "China" },
    Vendor { name: "OSH Park", url: "https://oshpark.com", categories: &["pcb"], services: "Pooled prototype PCB fabrication (purple boards), 2 and 4 layer", consumes: &["Gerber + drill zip"], region: "USA" },
    Vendor { name: "Aisler", url: "https://aisler.net", categories: &["pcb"], services: "Prototype PCB fabrication, stencils and assembly", consumes: &["Gerber + drill zip", "BOM.csv", "CPL.csv"], region: "Europe" },
    Vendor { name: "Eurocircuits", url: "https://www.eurocircuits.com", categories: &["pcb"], services: "PCB fabrication and assembly with online DFM checks", consumes: &["Gerber + drill zip", "BOM.csv", "CPL.csv"], region: "Europe" },
    Vendor { name: "Seeed Fusion", url: "https://www.seeedstudio.com/fusion.html", categories: &["pcb"], services: "PCB fabrication and PCB assembly (Fusion PCBA)", consumes: &["Gerber + drill zip", "BOM.csv", "CPL.csv"], region: "China" },
    Vendor { name: "LCSC", url: "https://www.lcsc.com", categories: &["parts"], services: "Electronic component distributor (parts library used by JLCPCB assembly)", consumes: &["BOM.csv"], region: "China" },
    Vendor { name: "Digi-Key", url: "https://www.digikey.com", categories: &["parts"], services: "Electronic component distributor", consumes: &["BOM.csv"], region: "USA" },
    Vendor { name: "Mouser", url: "https://www.mouser.com", categories: &["parts"], services: "Electronic component distributor", consumes: &["BOM.csv"], region: "USA" },
    Vendor { name: "Protolabs", url: "https://www.protolabs.com", categories: &["molding", "print", "cnc"], services: "Injection molding (aluminium and steel tooling), 3D printing, CNC machining, sheet metal", consumes: &["STEP", "2D drawing (PDF)", "Molding RFQ"], region: "USA / Europe / Japan" },
    Vendor { name: "Xometry", url: "https://www.xometry.com", categories: &["molding", "print", "cnc"], services: "Manufacturing marketplace: injection molding, 3D printing, CNC, urethane casting", consumes: &["STEP", "STL", "2D drawing (PDF)", "Molding RFQ"], region: "USA / Europe" },
    Vendor { name: "Fictiv", url: "https://www.fictiv.com", categories: &["molding", "cnc", "print"], services: "Injection molding, CNC machining, 3D printing, urethane casting", consumes: &["STEP", "2D drawing (PDF)", "Molding RFQ"], region: "USA" },
    Vendor { name: "JLC3DP", url: "https://jlc3dp.com", categories: &["print"], services: "3D printing service (SLA, MJF, SLS, FDM, metal)", consumes: &["STL", "STEP"], region: "China" },
    Vendor { name: "Craftcloud", url: "https://craftcloud3d.com", categories: &["print"], services: "3D printing marketplace comparing many print services and materials", consumes: &["STL"], region: "Global" },
    Vendor { name: "SendCutSend", url: "https://sendcutsend.com", categories: &["cnc"], services: "Laser/waterjet-cut sheet parts, bending, finishing", consumes: &["DXF", "STEP"], region: "USA" },
];
