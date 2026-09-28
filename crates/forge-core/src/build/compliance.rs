//! Compliance and certification planner.
//!
//! Derives which regulations and standards a product is likely to meet from
//! its components, interfaces, requirements and design parameters (a
//! Bluetooth Low Energy radio → radio and RF-exposure rules; a lithium cell →
//! battery safety and transport testing; a micro-display → photobiological
//! safety; a frame worn on the face → eyewear and skin-contact checks), for
//! the chosen markets. Each item is traced to what triggered it, placed at a
//! validation stage (EVT / DVT / PVT) and given an **indicative** cost and
//! time band — rough orders of magnitude to be confirmed with a test lab,
//! never quotes.
//!
//! This is a planning aid, not legal advice: scope, standard editions and
//! dates change; an accredited lab or notified body confirms what applies.
//!
//! References: FCC 47 CFR Part 15 (radio-frequency devices) and Part 2
//! (equipment authorisation); Directive 2014/53/EU (Radio Equipment
//! Directive, RED); Directive 2011/65/EU (RoHS) and Delegated Directive
//! (EU) 2015/863; Regulation (EC) 1907/2006 (REACH); Directive 2012/19/EU
//! (WEEE); Regulation (EU) 2023/1542 (batteries); Regulation (EU) 2023/988
//! (General Product Safety Regulation); IEC 62368-1; IEC 62133-2; IEC 62471;
//! ISO 12870; ISO 12312-1; Regulation (EU) 2016/425 (PPE); Bluetooth SIG
//! Qualification Process; UN *Manual of Tests and Criteria*, section 38.3.

use serde::{Deserialize, Serialize};

use crate::project::Project;

#[derive(Clone, Debug, Serialize)]
pub struct Trace {
    /// component | interface | requirement | design | subsystem | risk
    pub kind: &'static str,
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Item {
    pub id: &'static str,
    pub title: String,
    pub category: &'static str,
    pub markets: Vec<&'static str>,
    pub standards: Vec<&'static str>,
    pub why: String,
    pub traces: Vec<Trace>,
    /// EVT | DVT | PVT
    pub stage: &'static str,
    /// mandatory | conditional | voluntary
    pub obligation: &'static str,
    pub route: String,
    /// Indicative cost band, USD (rough order of magnitude); None = fee schedule / varies.
    pub cost_usd: Option<[f64; 2]>,
    pub weeks: [f64; 2],
    pub notes: String,
    pub explain: &'static str,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Features {
    pub digital: bool,
    pub radio: bool,
    pub radio_kinds: Vec<String>,
    pub bluetooth: bool,
    pub radio_module: bool,
    pub lithium: bool,
    pub light_source: bool,
    pub wearable: bool,
    pub eyewear: bool,
    pub lenses: bool,
    pub metal_parts: bool,
    pub usb: bool,
    pub polycarbonate: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Options {
    /// Markets: US, EU, UK, CA.
    pub markets: Vec<String>,
    /// Override: the radio is a pre-certified module (modular approval).
    pub radio_module: Option<bool>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            markets: vec!["US".into(), "EU".into()],
            radio_module: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Phase {
    pub id: &'static str,
    pub label: String,
    pub start_week: f64,
    pub end_week: f64,
    pub milestones: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Bar {
    pub id: String,
    pub label: String,
    pub stage: &'static str,
    pub start_week: f64,
    pub weeks: f64,
    pub activity: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Plan {
    pub markets: Vec<String>,
    pub features: Features,
    pub items: Vec<Item>,
    pub phases: Vec<Phase>,
    pub timeline: Vec<Bar>,
    pub total_cost_usd: [f64; 2],
    pub mandatory: usize,
    pub notes: Vec<String>,
}

fn has(s: &str, keys: &[&str]) -> bool {
    let l = s.to_ascii_lowercase();
    keys.iter().any(|k| l.contains(k))
}

fn comp_traces(p: &Project, keys: &[&str]) -> Vec<Trace> {
    p.components
        .iter()
        .filter(|c| has(&format!("{} {} {}", c.name, c.part_class, c.example_part), keys))
        .map(|c| Trace {
            kind: "component",
            id: c.refdes.clone(),
            label: c.name.clone(),
        })
        .collect()
}

fn req_traces(p: &Project, keys: &[&str]) -> Vec<Trace> {
    p.requirements
        .iter()
        .filter(|r| has(&r.text, keys))
        .map(|r| Trace {
            kind: "requirement",
            id: r.id.clone(),
            label: r.text.clone(),
        })
        .collect()
}

const RADIO_KINDS: &[&str] = &[
    "BLE",
    "BLUETOOTH",
    "WIFI",
    "WI-FI",
    "LTE",
    "RADIO",
    "ZIGBEE",
    "THREAD",
    "LORA",
    "UWB",
    "NFC",
];

pub fn features(p: &Project, o: &Options) -> Features {
    let mut f = Features {
        digital: p.components.iter().any(|c| c.on_pcb || c.power.active_mw > 0.0),
        ..Default::default()
    };
    for i in &p.interfaces {
        let k = i.kind.to_ascii_uppercase();
        if RADIO_KINDS.iter().any(|r| k == *r) && !f.radio_kinds.contains(&k) {
            f.radio_kinds.push(k);
        }
    }
    let radio_parts = comp_traces(p, &["ble", "bluetooth", "wi-fi", "wifi", "radio", "antenna", "lte", "lora"]);
    f.radio = !f.radio_kinds.is_empty() || !radio_parts.is_empty();
    f.bluetooth =
        f.radio_kinds.iter().any(|k| k.contains("BLE") || k.contains("BLUETOOTH")) || !comp_traces(p, &["ble", "bluetooth"]).is_empty();
    f.radio_module = o.radio_module.unwrap_or_else(|| {
        p.components.iter().any(|c| {
            has(&format!("{} {}", c.name, c.part_class), &["module"])
                && has(
                    &format!("{} {}", c.name, c.part_class),
                    &["ble", "bluetooth", "wi-fi", "wifi", "radio"],
                )
        })
    });
    // the default design carries LiPo battery parameters, so also require electronics
    f.lithium = !comp_traces(p, &["lipo", "li-ion", "lithium"]).is_empty() || (f.digital && has(&p.design.battery.chemistry, &["li"]));
    f.light_source = !comp_traces(p, &["display", "oled", "lcd", "led", "laser", "infrared"]).is_empty();
    let text = format!("{} {} {}", p.meta.name, p.meta.description, p.meta.tagline);
    f.eyewear = has(&text, &["glasses", "eyewear", "spectacle"]);
    f.wearable = f.eyewear || has(&text, &["wearable", "watch", "band", "headset", "wear "]) || !req_traces(p, &["skin"]).is_empty();
    f.lenses = f.eyewear
        && !comp_traces(p, &["lens"])
            .iter()
            .all(|t| p.components.iter().any(|c| c.refdes == t.id && c.subsystem.contains("OPT")));
    f.metal_parts = !comp_traces(p, &["screw", "hinge", "metal", "steel"]).is_empty();
    f.usb = p.interfaces.iter().any(|i| has(&i.kind, &["usb"])) || !comp_traces(p, &["usb"]).is_empty();
    f.polycarbonate = !p.components.is_empty()
        && (has(&p.design.manufacturing.molding_material, &["pc"])
            || has(&p.design.frame.material, &["pc"])
            || !comp_traces(p, &["polycarbonate"]).is_empty());
    f
}

#[allow(clippy::too_many_arguments)]
fn it(
    id: &'static str,
    title: &str,
    category: &'static str,
    markets: &[&'static str],
    standards: &[&'static str],
    stage: &'static str,
    obligation: &'static str,
    route: &str,
    cost: Option<[f64; 2]>,
    weeks: [f64; 2],
    explain: &'static str,
) -> Item {
    Item {
        id,
        title: title.into(),
        category,
        markets: markets.to_vec(),
        standards: standards.to_vec(),
        why: String::new(),
        traces: vec![],
        stage,
        obligation,
        route: route.into(),
        cost_usd: cost,
        weeks,
        notes: String::new(),
        explain,
    }
}

impl Item {
    fn why(mut self, s: impl Into<String>, t: Vec<Trace>) -> Item {
        self.why = s.into();
        self.traces = t;
        self
    }
    fn note(mut self, s: impl Into<String>) -> Item {
        self.notes = s.into();
        self
    }
}

fn iface_traces(p: &Project, kinds: &[&str]) -> Vec<Trace> {
    p.interfaces
        .iter()
        .filter(|i| kinds.iter().any(|k| i.kind.eq_ignore_ascii_case(k)))
        .map(|i| Trace {
            kind: "interface",
            id: i.id.clone(),
            label: format!("{} {} → {}: {}", i.kind, i.from, i.to, i.description),
        })
        .collect()
}

fn design(id: &str, label: String) -> Trace {
    Trace {
        kind: "design",
        id: id.into(),
        label,
    }
}

pub fn plan(p: &Project, o: &Options) -> Plan {
    let f = features(p, o);
    let mk: Vec<String> = o.markets.iter().map(|m| m.to_ascii_uppercase()).collect();
    let us = mk.iter().any(|m| m == "US");
    let eu = mk.iter().any(|m| m == "EU");
    let uk = mk.iter().any(|m| m == "UK");
    let ca = mk.iter().any(|m| m == "CA");
    let mut radio_t = iface_traces(p, RADIO_KINDS);
    radio_t.extend(comp_traces(p, &["ble", "bluetooth", "wi-fi", "wifi", "radio", "antenna"]));
    let digital_t: Vec<Trace> = {
        let n = p.components.iter().filter(|c| c.on_pcb).count();
        let mut v = comp_traces(p, &["system-on-chip", "soc", "mcu", "microcontroller", "buck", "processor"]);
        v.push(design(
            "design.pcb",
            format!("{n} parts on the main PCB (clocked digital logic, switching regulator)"),
        ));
        v
    };
    let mut batt_t = comp_traces(p, &["lipo", "li-ion", "lithium", "cell"]);
    batt_t.push(design(
        "design.battery",
        format!(
            "{} {:.0} mAh {:.1} V",
            p.design.battery.chemistry, p.design.battery.capacity_mah, p.design.battery.nominal_v
        ),
    ));
    let wh = p.design.battery.capacity_mah * p.design.battery.nominal_v / 1000.0;
    let light_t = comp_traces(p, &["display", "oled", "lcd", "led", "infrared", "combiner"]);
    let mut skin_t = req_traces(p, &["skin"]);
    skin_t.extend(
        p.subsystems
            .iter()
            .filter(|s| has(&s.name, &["frame", "enclosure", "strap", "band"]))
            .map(|s| Trace {
                kind: "subsystem",
                id: s.id.clone(),
                label: s.name.clone(),
            }),
    );
    let frame_t = vec![design(
        "design.frame",
        format!(
            "{} frame, {:.0}□{:.0}-{:.0} (lens □ bridge – temple, mm)",
            p.design.frame.material, p.design.frame.lens_width_mm, p.design.frame.bridge_mm, p.design.frame.temple_length_mm
        ),
    )];
    let mut items: Vec<Item> = Vec::new();
    let module = f.radio_module;

    // ---- United States ----
    if us && f.digital {
        items.push(
            it("fcc15b", "FCC Part 15 Subpart B — unintentional radiator (Class B digital device)", "EMC", &["US"], &["47 CFR Part 15 Subpart B", "ANSI C63.4"], "PVT", "mandatory", "Supplier's Declaration of Conformity (SDoC) backed by an emissions test report; pre-scan at DVT", Some([1500.0, 5000.0]), [1.0, 2.0], "Every product with clocked digital circuits radiates. Part 15 Subpart B limits those unintended emissions (radiated and, for mains-powered products, conducted). Class B limits apply to products used in homes.")
                .why("Clocked digital logic and a switching regulator emit radio-frequency noise", digital_t.clone()),
        );
    }
    if us && f.radio {
        let route = if module {
            "Pre-certified radio module with modular approval (§15.212): follow the module's grant conditions and integration guide, label 'Contains FCC ID', verify the host with Part 15B testing and spot checks"
        } else {
            "Equipment certification through a Telecommunication Certification Body (TCB) → FCC ID; or redesign around a pre-certified module to reuse its grant"
        };
        items.push(
            it("fcc15c", "FCC Part 15 Subpart C — intentional radiator (2.4 GHz)", "Radio", &["US"], &["47 CFR §15.247", "ANSI C63.10"], "PVT", "mandatory", route, Some(if module { [1000.0, 5000.0] } else { [5000.0, 20000.0] }), if module { [1.0, 3.0] } else { [3.0, 6.0] }, "A transmitter needs an equipment authorisation before it is marketed. Using a module that already holds an FCC grant (modular approval) avoids repeating most of the radio tests — the host is then checked for unintentional emissions and correct integration.")
                .why("The product transmits on purpose", radio_t.clone())
                .note(if module { "Module grant reused." } else { "The Iris design puts a bare BLE system-on-chip and chip antenna on its own board — that needs its own certification. A pre-certified BLE module (with a small area penalty in the pod) would cut this cost and time substantially." }),
        );
        items.push(
            it("fccrf", "FCC RF exposure — portable device evaluation", "Radio", &["US"], &["47 CFR §1.1307, §2.1093", "FCC KDB 447498"], "PVT", "mandatory", "SAR test-exclusion analysis for low-power transmitters used near the body (often sufficient for a few-mW BLE radio); full SAR test otherwise", Some([500.0, 3000.0]), [1.0, 2.0], "Devices used within 20 cm of the body are 'portable' and must show the specific absorption rate (SAR) is within limits — or qualify for an exemption based on power and distance.")
                .why("The transmitter sits on the head", radio_t.clone()),
        );
    }
    if us && f.polycarbonate {
        items.push(
            it("prop65", "California Proposition 65 — chemical warning assessment", "Materials", &["US"], &["California Health & Safety Code §25249.5 et seq."], "PVT", "conditional", "Assess listed chemicals (e.g. bisphenol A in polycarbonate, lead in brass or solder, phthalates in cable jackets); warning label if exposure exceeds the safe-harbour level", Some([500.0, 3000.0]), [1.0, 3.0], "Proposition 65 requires a clear warning when a product can expose Californians to listed chemicals above safe-harbour levels.")
                .why("Polycarbonate frame material (bisphenol A is a listed chemical)", vec![design("design.manufacturing", format!("molding material {}", p.design.manufacturing.molding_material))]),
        );
    }
    if us && f.eyewear && f.lenses {
        items.push(
            it("lens_impact", "Eyeglass / sunglass lens impact resistance", "Eyewear", &["US"], &["21 CFR 801.410 (drop-ball test)"], "DVT", "conditional", "Drop-ball test of the shipped lenses (or lens supplier's certificate); ISO 12312-1 applies if sold as sunglasses", Some([500.0, 2000.0]), [1.0, 2.0], "In the US, lenses in eyeglasses and sunglasses must be impact resistant. Frames sold for prescription lenses may be regulated as medical devices — ask a regulatory consultant if you go that way.")
                .why("Lenses are shipped in the frame", comp_traces(p, &["lens"]).into_iter().filter(|t| !p.components.iter().any(|c| c.refdes == t.id && c.subsystem.contains("OPT"))).collect()),
        );
    }

    // ---- European Union ----
    if eu && f.radio {
        items.push(
            it("en300328", "RED Article 3.2 — efficient use of radio spectrum (2.4 GHz)", "Radio", &["EU"], &["Directive 2014/53/EU", "EN 300 328"], "PVT", "mandatory", "Radio test report to the harmonised standard (module reports can often be reused if the antenna and integration match)", Some([4000.0, 12000.0]), [2.0, 4.0], "The Radio Equipment Directive (RED) covers radio spectrum, electromagnetic compatibility (EMC), safety and health. EN 300 328 is the harmonised standard for wideband transmitters in the 2.4 GHz band (Bluetooth, Wi-Fi).")
                .why("2.4 GHz transmitter", radio_t.clone()),
        );
        items.push(
            it("en301489", "RED Article 3.1(b) — EMC for radio equipment", "EMC", &["EU"], &["EN 301 489-1", "EN 301 489-17"], "PVT", "mandatory", "Emissions and immunity test report (electrostatic discharge, radiated immunity …)", Some([3000.0, 8000.0]), [2.0, 3.0], "EMC covers both what the product emits and how it copes with interference (including electrostatic discharge from a user's touch — relevant to the Iris touch strip).")
                .why("Radio equipment must meet EMC requirements under RED", {
                    let mut t = radio_t.clone();
                    t.extend(comp_traces(p, &["touch"]));
                    t
                }),
        );
        items.push(
            it("en62479", "RED Article 3.1(a) — RF exposure (health)", "Radio", &["EU"], &["EN 62479 (low-power)", "EN 62311", "EN 50566 (SAR, if needed)"], "PVT", "mandatory", "Low-power exclusion assessment for a few-mW transmitter; SAR testing only if above the thresholds", Some([500.0, 3000.0]), [1.0, 2.0], "Low-power devices can show compliance with the exposure limits by a simple assessment (EN 62479) instead of measuring SAR.")
                .why("The transmitter is worn on the head", radio_t.clone()),
        );
        items.push(
            it("redcyber", "RED Article 3.3(d)(e)(f) — cybersecurity", "Radio", &["EU"], &["Delegated Regulation (EU) 2022/30", "EN 18031 series"], "DVT", "conditional", "Assess scope (radio equipment that can communicate over the internet, directly or via another device such as a phone); secure pairing, updates, data protection", Some([2000.0, 10000.0]), [2.0, 6.0], "Since August 2025, internet-connected radio equipment in the EU must protect the network, personal data and against fraud. A phone-tethered wearable that handles voice is likely in scope — confirm with a lab.")
                .why("BLE link to a phone app that relays voice and notifications", iface_traces(p, &["BLE"])),
        );
    }
    if eu && f.digital && !f.radio {
        items.push(
            it("emcd", "EMC Directive — emissions and immunity", "EMC", &["EU"], &["Directive 2014/30/EU", "EN 55032", "EN 55035"], "PVT", "mandatory", "Test report + EU Declaration of Conformity", Some([2000.0, 6000.0]), [1.0, 3.0], "Non-radio electronics sold in the EU meet the EMC Directive. (Battery products below 50 V AC / 75 V DC are outside the Low Voltage Directive; their safety falls under the General Product Safety Regulation.)")
                .why("Digital electronics without a radio", digital_t.clone()),
        );
    }
    if eu && f.digital {
        items.push(
            it("ce_doc", "CE marking — EU Declaration of Conformity and technical file", "Labelling", &["EU"], &[if f.radio { "Directive 2014/53/EU (RED)" } else { "Directive 2014/30/EU" }, "Directive 2011/65/EU (RoHS)"], "PVT", "mandatory", "Self-declaration when harmonised standards are applied in full (no notified body needed); keep the technical file for 10 years", Some([0.0, 2000.0]), [1.0, 2.0], "The manufacturer compiles the technical documentation (test reports, risk assessment, drawings, user instructions), signs the EU Declaration of Conformity and affixes the CE mark.")
                .why("Every directive above ends in one declaration", vec![]),
        );
        items.push(
            it("rohs", "RoHS — restriction of hazardous substances", "Materials", &["EU"], &["Directive 2011/65/EU", "Delegated Directive (EU) 2015/863", "EN IEC 63000 (technical documentation)"], "PVT", "mandatory", "Supplier material declarations for every BOM line + lead-free assembly (SAC305) + optional XRF screening", Some([500.0, 3000.0]), [1.0, 3.0], "Limits lead, mercury, cadmium, hexavalent chromium, two brominated flame retardants and four phthalates in electrical and electronic equipment. Assemble with lead-free solder (see the reflow calculator).")
                .why(format!("{} components in the bill of materials", p.components.len()), digital_t.clone()),
        );
        let mut reach = it("reach", "REACH — substances of very high concern (SVHC)", "Materials", &["EU"], &["Regulation (EC) 1907/2006, Article 33", "SCIP database notification"], "PVT", "mandatory", "Collect supplier SVHC declarations; inform customers if an article contains an SVHC above 0.1 % w/w and notify the SCIP database", Some([500.0, 3000.0]), [1.0, 3.0], "REACH requires communicating substances of very high concern in articles, and restricts some substances outright (Annex XVII).")
            .why("Plastics, cables and coatings in the product", vec![design("design.frame.material", p.design.frame.material.clone())]);
        if f.metal_parts && f.wearable {
            reach = reach.note("Metal parts in prolonged skin contact (hinges, screws) are subject to the REACH Annex XVII nickel-release restriction — spectacle frames are tested per EN 16128.");
            reach.traces.extend(comp_traces(p, &["screw", "hinge"]));
        }
        items.push(reach);
        items.push(
            it("weee", "WEEE — take-back marking and producer registration", "Environmental", &["EU"], &["Directive 2012/19/EU", "EN 50419 (marking)"], "PVT", "mandatory", "Crossed-out wheelie-bin symbol on the product; register as a producer (or use an authorised representative) in each EU country you sell in", None, [2.0, 6.0], "Electrical and electronic equipment must be collected and recycled at end of life; producers finance it through national schemes.")
                .why("Electronic product sold in the EU", digital_t.clone()),
        );
        items.push(
            it("gpsr", "EU responsible economic operator and product traceability", "Labelling", &["EU"], &["Regulation (EU) 2019/1020, Article 4", "Regulation (EU) 2023/988 (GPSR)"], "PVT", "mandatory", "Name an EU-based responsible person (importer / authorised representative); product, batch and contact details on the product or packaging; online listings show the same", None, [1.0, 4.0], "CE-marked products need an economic operator in the EU, and the General Product Safety Regulation adds traceability and online-sales requirements.")
                .why("Placing a consumer product on the EU market", vec![]),
        );
    }
    if eu && f.lithium {
        items.push(
            it("eubatt", "EU Batteries Regulation", "Battery", &["EU"], &["Regulation (EU) 2023/1542"], "PVT", "mandatory", "Battery labelling, documentation and producer responsibility; check the phased dates (e.g. removability / replaceability of portable batteries) for your product", None, [2.0, 6.0], "The 2023 Batteries Regulation replaced the Battery Directive: labelling (including the separate-collection symbol), extended producer responsibility and, over the coming years, requirements on removability and replaceability of portable batteries.")
                .why(format!("Portable lithium battery ({wh:.2} Wh)"), batt_t.clone()),
        );
    }

    // ---- UK / Canada ----
    if uk && f.digital {
        items.push(
            it(
                "ukca",
                "Great Britain market access (UKCA or recognised CE)",
                "Labelling",
                &["UK"],
                &[if f.radio {
                    "Radio Equipment Regulations 2017"
                } else {
                    "Electromagnetic Compatibility Regulations 2016"
                }],
                "PVT",
                "mandatory",
                "Usually the same test reports as the EU; UKCA marking or CE marking where GB still recognises it — check current GB rules",
                Some([0.0, 2000.0]),
                [1.0, 2.0],
                "Great Britain has its own regulations mirroring the EU directives.",
            )
            .why("Selling in Great Britain", radio_t.clone()),
        );
    }
    if ca && f.digital {
        items.push(
            it("ised", "Canada ISED certification", "Radio", &["CA"], if f.radio { &["RSS-Gen", "RSS-247", "RSS-102 (RF exposure)", "ICES-003"] } else { &["ICES-003"] }, "PVT", "mandatory", "Certification through a Certification Body (radio) and ICES-003 self-declaration (digital apparatus); IC number on the label", Some([3000.0, 10000.0]), [2.0, 5.0], "Innovation, Science and Economic Development Canada (ISED) mirrors the FCC process with its own standards.")
                .why("Selling in Canada", radio_t.clone()),
        );
    }

    // ---- Global / market-driven ----
    if f.digital {
        let mut t = req_traces(p, &["skin", "temperature"]);
        t.extend(batt_t.clone());
        items.push(
            it("iec62368", "Product safety — audio/video and ICT equipment", "Safety", &["Global"], &["IEC 62368-1 / EN IEC 62368-1", "UL 62368-1 (US listing)"], "DVT", "mandatory", "Hazard-based safety evaluation: electrical, thermal (touch temperature of worn parts), battery / fire, mechanical; required in the EU via RED Art. 3.1(a), expected by US retailers (NRTL listing) — test report or certification body scheme (CB) report", Some([5000.0, 15000.0]), [3.0, 6.0], "IEC 62368-1 classifies energy sources (electrical, thermal, chemical, optical …) and requires safeguards between them and the user. For glasses the big ones are battery fire and hot surfaces against skin.")
                .why(format!("Battery-powered ICT device worn on the head (skin limit {:.0} °C in the thermal model)", p.design.thermal.skin_limit_c), t),
        );
    }
    if f.lithium {
        items.push(
            it("iec62133", "Lithium cell / battery safety", "Battery", &["Global"], &["IEC 62133-2", "UL 1642 / UL 2054 (often requested in the US)"], "DVT", "mandatory", "Obtain the cell maker's IEC 62133-2 report (and certificate); test your own pack only if you assemble one", Some([3000.0, 10000.0]), [4.0, 8.0], "IEC 62133-2 covers the safety of portable lithium cells and batteries (overcharge, short circuit, crush, thermal abuse). IEC 62368-1 relies on it for the battery in the product.")
                .why("Lithium-polymer cell", batt_t.clone())
                .note("FMEA risk F-1 (swelling / thermal runaway) is mitigated partly by this certification — keep the report in the technical file."),
        );
        items.push(
            it("un383", "Lithium battery transport testing", "Battery", &["Global"], &["UN Manual of Tests and Criteria, section 38.3", "IATA Dangerous Goods Regulations (UN3481 — batteries contained in equipment)"], "DVT", "mandatory", "Get the UN 38.3 test summary from the cell / pack supplier before shipping any units by air; follow the carrier's packing instructions", Some([5000.0, 15000.0]), [4.0, 8.0], "UN 38.3 (altitude, thermal cycling, vibration, shock, external short, impact, overcharge, forced discharge) is required before lithium cells can be transported. Suppliers provide a test summary document.")
                .why(format!("Lithium cell ({wh:.2} Wh) shipped inside every unit"), batt_t.clone()),
        );
    }
    if f.light_source {
        let mut t = light_t.clone();
        t.push(design(
            "design.optics.display_luminance_nits",
            format!(
                "{:.0} nits micro-display seen through the combiner",
                p.design.optics.display_luminance_nits
            ),
        ));
        items.push(
            it("iec62471", "Photobiological safety of the light source", "Optical", &["Global"], &["IEC 62471 / EN 62471", "IEC 62368-1 (optical radiation clauses)"], "DVT", "mandatory", "Risk-group classification of the display (and any LED / IR emitter) as seen by the eye; usually 'exempt' for a dim display — confirm by measurement", Some([2000.0, 6000.0]), [1.0, 3.0], "IEC 62471 assesses blue-light, retinal-thermal and infrared hazards of lamps and LEDs. A display a few centimetres from the eye, magnified by optics, should be measured rather than assumed safe.")
                .why("A bright light source sits close to the eye", t),
        );
    }
    if f.eyewear {
        let mut t = frame_t.clone();
        t.extend(p.risks.iter().filter(|r| has(&r.item, &["frame", "hinge"])).map(|r| Trace {
            kind: "risk",
            id: r.id.clone(),
            label: format!("{}: {}", r.item, r.failure_mode),
        }));
        items.push(
            it("iso12870", "Spectacle frames — requirements and test methods", "Eyewear", &["Global"], &["ISO 12870", "EN 16128 (nickel release)"], "DVT", "voluntary", "Test to the standard (dimensional stability at elevated temperature, bridge deformation, lens retention, hinge endurance, perspiration resistance); expected by optical retailers; mandatory route if sold as a medical device frame", Some([2000.0, 8000.0]), [2.0, 4.0], "ISO 12870 sets the mechanical and material requirements of spectacle frames — exactly the failure modes the Structures Lab models (bridge crack, hinge fatigue).")
                .why("The product is a spectacle frame", t),
        );
    }
    if f.eyewear && f.lenses {
        let lens_t: Vec<Trace> = comp_traces(p, &["lens"])
            .into_iter()
            .filter(|t| !p.components.iter().any(|c| c.refdes == t.id && c.subsystem.contains("OPT")))
            .collect();
        items.push(
            it("iso12312", "Sunglass filters — only if tinted / sun lenses ship", "Eyewear", &["Global"], &["ISO 12312-1 (sunglasses for general use)", "EU PPE Regulation (EU) 2016/425 (sunglasses are personal protective equipment in the EU)"], "DVT", "conditional", "If the product ships with tinted lenses or is marketed as sunglasses: filter category, luminous transmittance, ultraviolet protection, traffic-signal recognition and marking tested to ISO 12312-1 by an optics lab; clear lenses do not need it", Some([1000.0, 4000.0]), [1.0, 3.0], "ISO 12312-1 sets the optical requirements for sunglasses — how much visible light each filter category transmits, ultraviolet protection, and that traffic signals stay recognisable. A heads-up display with sun lenses must meet it through the combiner region too.")
                .why("Front lenses are part of the product — relevant only for tinted / sun variants", lens_t),
        );
    }
    if f.wearable {
        let mut t = skin_t.clone();
        t.push(design(
            "design.frame.material",
            format!(
                "{} (prototype) / {} (molded)",
                p.design.frame.material, p.design.manufacturing.molding_material
            ),
        ));
        items.push(
            it("biocompat", "Skin-contact materials — biocompatibility screening", "Materials", &["Global"], &["ISO 10993-1 (framework)", "ISO 10993-5 (cytotoxicity)", "ISO 10993-10 / -23 (sensitisation, irritation)"], "DVT", "voluntary", "Consult a lab; may not be required for consumer eyewear. Prefer materials with supplier skin-contact data; screen colourants and coatings", Some([2000.0, 10000.0]), [3.0, 8.0], "The ISO 10993 series is written for medical devices, but its skin-contact screening tests are a common due-diligence step for wearables touching skin all day.")
                .why("Frame, nose pads and temple tips touch skin for hours", t),
        );
    }
    if f.bluetooth {
        items.push(
            it("btsig", "Bluetooth SIG qualification and product listing", "Radio", &["Global"], &["Bluetooth Qualification Process"], "PVT", "mandatory", "Reference the qualified design of the SoC / stack, complete the listing and obtain a Declaration ID; required to use the Bluetooth name and logo", None, [1.0, 2.0], "Products that use Bluetooth technology and trademarks must be listed with the Bluetooth Special Interest Group (SIG); a fee applies — see bluetooth.com.")
                .why("BLE link", iface_traces(p, &["BLE"])),
        );
    }
    if f.usb && f.lithium {
        items.push(
            it("charging", "Charging interface and power adapter", "Safety", &["Global"], &["IEC 62368-1 (adapter)", "Directive (EU) 2022/2380 (common charger, if the category is covered)"], "DVT", "conditional", "If a wall adapter ships in the box it needs its own safety certification; check whether the EU common-charger rules cover your product category (USB-C is the safe choice)", Some([0.0, 0.0]), [0.5, 1.0], "USB-C charging from any certified adapter keeps the product itself free of mains voltages.")
                .why("USB-C charging input", comp_traces(p, &["usb"])),
        );
    }
    if f.digital {
        let mut stds = vec![
            "47 CFR §15.19, §15.105, §2.935 (e-labelling)",
            "CE marking rules (Regulation (EC) 765/2008)",
            "EN 50419 (WEEE symbol)",
            "Battery marking rules",
        ];
        if !us {
            stds.remove(0);
        }
        items.push(
            it("labels", "Labels, markings and user-manual statements", "Labelling", &["Global"], &stds, "PVT", "mandatory", "FCC ID / SDoC statement, CE mark, WEEE bin, battery symbols, manufacturer name and postal address, model and serial number, required user-manual statements; electronic labels are allowed on devices with a screen (FCC)", Some([0.0, 1000.0]), [1.0, 2.0], "Small products can put some markings on the packaging and in the manual, and — for devices with a display — show the FCC information electronically. Plan label space on the temple early.")
                .why("Every certification ends in a marking", vec![design("design.frame", format!("temple {:.0} × {:.0} mm — little room for labels", p.design.frame.temple_width_mm, p.design.frame.temple_height_mm))]),
        );
    }

    // ---- timeline ----
    let find = |stage: &str| crate::build::roadmap::milestones_for(p, stage).0;
    let phases = vec![
        Phase {
            id: "EVT",
            label: "EVT — engineering validation".into(),
            start_week: 0.0,
            end_week: 8.0,
            milestones: find("evt"),
        },
        Phase {
            id: "DVT",
            label: "DVT — design validation".into(),
            start_week: 8.0,
            end_week: 20.0,
            milestones: find("dvt"),
        },
        Phase {
            id: "PVT",
            label: "PVT — production validation".into(),
            start_week: 20.0,
            end_week: 32.0,
            milestones: find("pvt"),
        },
    ];
    let mut timeline = vec![
        Bar {
            id: "prescan".into(),
            label: "Pre-compliance EMC / radio scan".into(),
            stage: "DVT",
            start_week: 12.0,
            weeks: 1.0,
            activity: true,
        },
        Bar {
            id: "freeze".into(),
            label: "Design freeze → production-intent samples".into(),
            stage: "DVT",
            start_week: 18.0,
            weeks: 2.0,
            activity: true,
        },
    ];
    let (mut n_dvt, mut n_pvt) = (0usize, 0usize);
    for i in &items {
        let w = 0.5 * (i.weeks[0] + i.weeks[1]);
        // stagger items in lab lanes so the chart reads as a plan, not one block
        let start = match i.stage {
            "EVT" => 2.0,
            "DVT" => {
                n_dvt += 1;
                8.0 + ((n_dvt - 1) as f64 * 1.5) % 9.0
            }
            _ => {
                n_pvt += 1;
                20.0 + ((n_pvt - 1) as f64 * 0.75) % 7.0
            }
        };
        timeline.push(Bar {
            id: i.id.into(),
            label: i.title.clone(),
            stage: i.stage,
            start_week: start,
            weeks: w,
            activity: false,
        });
    }
    let total = items
        .iter()
        .filter_map(|i| i.cost_usd)
        .fold([0.0, 0.0], |a, c| [a[0] + c[0], a[1] + c[1]]);
    let mandatory = items.iter().filter(|i| i.obligation == "mandatory").count();
    let mut notes = vec![
        "Commonly applicable items for these markets and features — confirm the list, editions and dates with an accredited test lab."
            .to_string(),
        "Planning aid, not legal advice. Costs and durations are rough orders of magnitude to be confirmed with a test lab — never quotes."
            .to_string(),
        "Pre-compliance scans at DVT (a few hours in a chamber) catch most emissions problems while the design can still change."
            .to_string(),
    ];
    if f.radio && !module {
        notes.push("Biggest lever: a pre-certified radio module turns the radio certification into an integration check.".into());
    }
    Plan {
        markets: mk,
        features: f,
        items,
        phases,
        timeline,
        total_cost_usd: total,
        mandatory,
        notes,
    }
}

/// Markets for the metric: `design.extra.markets` (array of strings), default US + EU.
pub fn project_options(p: &Project) -> Options {
    let markets = p
        .design
        .extra
        .get("markets")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect::<Vec<_>>())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| vec!["US".into(), "EU".into()]);
    Options {
        markets,
        radio_module: None,
    }
}

pub fn markdown(p: &Project, plan: &Plan) -> String {
    let mut s = format!(
        "# {} — compliance and certification plan\n\nMarkets: {}. Generated by Protoforge (Build Guide).\n\n> {}\n\n",
        p.meta.name,
        plan.markets.join(", "),
        plan.notes[0]
    );
    s += &format!(
        "Indicative total for costed items: ${:.0}k – ${:.0}k (rough order of magnitude).\n\n| ✓ | Item | Stage | Obligation | Standards | Why | Indicative cost | Weeks |\n|---|---|---|---|---|---|---|---|\n",
        plan.total_cost_usd[0] / 1000.0,
        plan.total_cost_usd[1] / 1000.0
    );
    for i in &plan.items {
        let cost = match i.cost_usd {
            Some([a, b]) if b > 0.0 => format!("${:.1}k–${:.1}k", a / 1000.0, b / 1000.0),
            Some(_) => "in-house".into(),
            None => "fees vary".into(),
        };
        let tr: Vec<String> = i.traces.iter().map(|t| t.id.clone()).collect();
        s += &format!(
            "| ☐ | **{}** | {} | {} | {} | {}{} | {} | {:.0}–{:.0} |\n",
            i.title,
            i.stage,
            i.obligation,
            i.standards.join("; "),
            i.why,
            if tr.is_empty() {
                String::new()
            } else {
                format!(" ({})", tr.join(", "))
            },
            cost,
            i.weeks[0],
            i.weeks[1]
        );
    }
    s += "\n## Routes\n\n";
    for i in &plan.items {
        s += &format!(
            "- **{}** — {}{}\n",
            i.title,
            i.route,
            if i.notes.is_empty() {
                String::new()
            } else {
                format!(" _{}_", i.notes)
            }
        );
    }
    s += "\n## Notes\n\n";
    for n in &plan.notes {
        s += &format!("- {n}\n");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_features() {
        let p = Project::iris();
        let f = features(&p, &Options::default());
        assert!(f.digital && f.radio && f.bluetooth && f.lithium && f.light_source && f.eyewear && f.wearable && f.usb);
        assert!(!f.radio_module, "Iris uses a bare SoC + chip antenna");
        assert!(f.polycarbonate);
    }

    #[test]
    fn iris_plan_us_eu() {
        let p = Project::iris();
        let plan = plan(&p, &Options::default());
        let ids: Vec<&str> = plan.items.iter().map(|i| i.id).collect();
        for want in [
            "fcc15b",
            "fcc15c",
            "en300328",
            "en301489",
            "iec62368",
            "iec62133",
            "un383",
            "rohs",
            "reach",
            "weee",
            "iec62471",
            "iso12870",
            "biocompat",
            "labels",
            "btsig",
            "iso12312",
        ] {
            assert!(ids.contains(&want), "missing {want}: {ids:?}");
        }
        assert!(!ids.contains(&"ised") && !ids.contains(&"emcd"));
        // radio items trace back to the BLE interface and the SoC
        let c = plan.items.iter().find(|i| i.id == "fcc15c").unwrap();
        assert!(c.traces.iter().any(|t| t.id == "IF-06") && c.traces.iter().any(|t| t.id == "U1"));
        // battery items trace to BT1
        assert!(plan
            .items
            .iter()
            .find(|i| i.id == "un383")
            .unwrap()
            .traces
            .iter()
            .any(|t| t.id == "BT1"));
        assert!(plan.total_cost_usd[1] > plan.total_cost_usd[0]);
        assert_eq!(plan.timeline.iter().filter(|b| !b.activity).count(), plan.items.len());
        let md = markdown(&p, &plan);
        assert!(md.contains("UN Manual of Tests and Criteria"));
    }

    #[test]
    fn module_and_markets_change_plan() {
        let p = Project::iris();
        let a = plan(
            &p,
            &Options {
                radio_module: Some(true),
                ..Default::default()
            },
        );
        let b = plan(&p, &Options::default());
        let ca = a.items.iter().find(|i| i.id == "fcc15c").unwrap().cost_usd.unwrap();
        let cb = b.items.iter().find(|i| i.id == "fcc15c").unwrap().cost_usd.unwrap();
        assert!(ca[1] < cb[1]);
        let c = plan(
            &p,
            &Options {
                markets: vec!["CA".into()],
                ..Default::default()
            },
        );
        assert!(c.items.iter().any(|i| i.id == "ised") && !c.items.iter().any(|i| i.id == "fcc15b"));
    }

    #[test]
    fn blank_project_has_few_items() {
        let p = Project::blank("Nothing");
        let items = plan(&p, &Options::default()).items;
        assert!(items.is_empty(), "{:?}", items.iter().map(|i| i.id).collect::<Vec<_>>());
    }
}
