//! Typical injection-molding properties of common thermoplastics.
//!
//! Values are *typical mid-range processing figures* compiled from standard
//! references (Rosato & Rosato, *Injection Molding Handbook*; Osswald,
//! *Understanding Polymer Processing*; resin suppliers' generic processing
//! guides). Real grades vary widely — always use the datasheet of the exact
//! grade you buy. Prices are indicative bulk resin prices, not quotes.

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Material {
    pub id: &'static str,
    pub name: &'static str,
    /// Melt (barrel) temperature, °C.
    pub t_melt_c: f64,
    /// Mold surface temperature, °C.
    pub t_mold_c: f64,
    /// Temperature at which the part is stiff enough to eject, °C.
    pub t_eject_c: f64,
    /// Thermal diffusivity α = k / (ρ·c_p), mm²/s.
    pub diffusivity_mm2_s: f64,
    /// Solid density, g/cm³.
    pub density_g_cm3: f64,
    /// Linear mold shrinkage, %.
    pub shrinkage_pct: f64,
    /// Typical mean cavity pressure used for clamp-force estimates, MPa.
    pub cavity_pressure_mpa: f64,
    /// Indicative resin price, USD/kg.
    pub price_usd_kg: f64,
    /// Typical max flow length / wall thickness ratio.
    pub max_flow_ratio: f64,
    /// Recommended minimum draft, degrees (textured surfaces need more).
    pub min_draft_deg: f64,
    pub notes: &'static str,
}

pub const MATERIALS: &[Material] = &[
    Material {
        id: "PC",
        name: "Polycarbonate",
        t_melt_c: 300.0,
        t_mold_c: 90.0,
        t_eject_c: 127.0,
        diffusivity_mm2_s: 0.13,
        density_g_cm3: 1.20,
        shrinkage_pct: 0.6,
        cavity_pressure_mpa: 55.0,
        price_usd_kg: 3.5,
        max_flow_ratio: 120.0,
        min_draft_deg: 1.0,
        notes: "Tough and optically clear; stiff melt needs high pressure and a hot mold; notch-sensitive, dry before molding.",
    },
    Material {
        id: "ABS",
        name: "ABS",
        t_melt_c: 240.0,
        t_mold_c: 60.0,
        t_eject_c: 85.0,
        diffusivity_mm2_s: 0.12,
        density_g_cm3: 1.05,
        shrinkage_pct: 0.5,
        cavity_pressure_mpa: 40.0,
        price_usd_kg: 2.2,
        max_flow_ratio: 180.0,
        min_draft_deg: 0.5,
        notes: "Easy to mold, good surface finish, paintable and platable; limited UV and chemical resistance.",
    },
    Material {
        id: "PC/ABS",
        name: "PC/ABS blend",
        t_melt_c: 265.0,
        t_mold_c: 75.0,
        t_eject_c: 100.0,
        diffusivity_mm2_s: 0.12,
        density_g_cm3: 1.15,
        shrinkage_pct: 0.55,
        cavity_pressure_mpa: 45.0,
        price_usd_kg: 3.0,
        max_flow_ratio: 150.0,
        min_draft_deg: 1.0,
        notes: "The usual consumer-electronics housing blend: tougher than ABS, easier than PC.",
    },
    Material {
        id: "PP",
        name: "Polypropylene",
        t_melt_c: 230.0,
        t_mold_c: 40.0,
        t_eject_c: 90.0,
        diffusivity_mm2_s: 0.09,
        density_g_cm3: 0.905,
        shrinkage_pct: 1.5,
        cavity_pressure_mpa: 30.0,
        price_usd_kg: 1.5,
        max_flow_ratio: 250.0,
        min_draft_deg: 1.0,
        notes: "Cheap, chemically resistant, allows living hinges; high shrinkage and warpage.",
    },
    Material {
        id: "PA66",
        name: "Nylon 66",
        t_melt_c: 285.0,
        t_mold_c: 80.0,
        t_eject_c: 150.0,
        diffusivity_mm2_s: 0.10,
        density_g_cm3: 1.14,
        shrinkage_pct: 1.5,
        cavity_pressure_mpa: 45.0,
        price_usd_kg: 3.5,
        max_flow_ratio: 200.0,
        min_draft_deg: 0.5,
        notes: "Strong, wear resistant, very fluid melt (flash risk); absorbs moisture — dimensions change.",
    },
    Material {
        id: "POM",
        name: "Acetal (POM)",
        t_melt_c: 205.0,
        t_mold_c: 90.0,
        t_eject_c: 145.0,
        diffusivity_mm2_s: 0.14,
        density_g_cm3: 1.41,
        shrinkage_pct: 2.0,
        cavity_pressure_mpa: 50.0,
        price_usd_kg: 2.8,
        max_flow_ratio: 150.0,
        min_draft_deg: 0.5,
        notes: "Low friction, stiff, great for clips and gears; high shrinkage, hard to bond or paint.",
    },
    Material {
        id: "TPU",
        name: "Thermoplastic polyurethane",
        t_melt_c: 210.0,
        t_mold_c: 35.0,
        t_eject_c: 60.0,
        diffusivity_mm2_s: 0.09,
        density_g_cm3: 1.20,
        shrinkage_pct: 1.0,
        cavity_pressure_mpa: 30.0,
        price_usd_kg: 5.0,
        max_flow_ratio: 120.0,
        min_draft_deg: 1.0,
        notes: "Rubber-like; used for nose pads, temple tips and overmolds. Needs generous draft to demould.",
    },
    Material {
        id: "PETG",
        name: "PETG copolyester",
        t_melt_c: 250.0,
        t_mold_c: 30.0,
        t_eject_c: 65.0,
        diffusivity_mm2_s: 0.11,
        density_g_cm3: 1.27,
        shrinkage_pct: 0.4,
        cavity_pressure_mpa: 35.0,
        price_usd_kg: 2.5,
        max_flow_ratio: 150.0,
        min_draft_deg: 1.0,
        notes: "Clear, tough, chemically resistant; the same family as the Print lab's PETG filament.",
    },
];

pub fn material(id: &str) -> &'static Material {
    let up = id.to_ascii_uppercase();
    MATERIALS.iter().find(|m| m.id.eq_ignore_ascii_case(&up)).unwrap_or(&MATERIALS[0])
}

#[derive(Clone, Debug, Serialize)]
pub struct Steel {
    pub id: &'static str,
    pub name: &'static str,
    /// Base cost of a simple single-cavity tool, USD (indicative).
    pub base_usd: f64,
    /// Cost per cm² of projected part area (cavity machining), USD.
    pub per_cm2_usd: f64,
    /// Cost per side action / lifter (undercut), USD.
    pub side_action_usd: f64,
    /// Typical tool life, shots.
    pub shot_life: f64,
    /// Typical lead time, weeks.
    pub lead_weeks: f64,
    pub notes: &'static str,
}

pub const STEELS: &[Steel] = &[
    Steel {
        id: "Aluminium",
        name: "Aluminium (7075) prototype tool",
        base_usd: 2500.0,
        per_cm2_usd: 45.0,
        side_action_usd: 1200.0,
        shot_life: 10_000.0,
        lead_weeks: 2.0,
        notes: "Fast and cheap soft tooling for bridge production and DVT builds.",
    },
    Steel {
        id: "P20",
        name: "P20 pre-hardened steel",
        base_usd: 6000.0,
        per_cm2_usd: 90.0,
        side_action_usd: 2500.0,
        shot_life: 500_000.0,
        lead_weeks: 5.0,
        notes: "The default production tool steel for moderate volumes.",
    },
    Steel {
        id: "H13",
        name: "H13 hardened steel",
        base_usd: 10000.0,
        per_cm2_usd: 130.0,
        side_action_usd: 3500.0,
        shot_life: 1_000_000.0,
        lead_weeks: 7.0,
        notes: "Hardened, polishable, for high volume and abrasive / glass-filled resins.",
    },
];

pub fn steel(id: &str) -> &'static Steel {
    STEELS.iter().find(|s| s.id.eq_ignore_ascii_case(id)).unwrap_or(&STEELS[1])
}
