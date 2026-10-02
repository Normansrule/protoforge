//! Bluetooth Low Energy (BLE) link budget.
//!
//! **Friis** free-space path loss at distance d and wavelength λ:
//! FSPL = 20·log10(4πd/λ) dB (Friis 1946; Pozar §14.2). **Log-distance**
//! model beyond a reference distance d0 = 1 m:
//! PL(d) = FSPL(d0) + 10·n·log10(d/d0) with a path-loss exponent n
//! (2 in free space; larger indoors with clutter — Rappaport,
//! *Wireless Communications*, §4.9).
//!
//! **Received power** P_rx = P_tx + G_tx + G_rx − PL − L_match − L_head −
//! L_body − L_other. **Margin** = P_rx − sensitivity − fade margin, where the
//! Rayleigh fade margin for an outage probability p is
//! F = −10·log10(−ln(1 − p)) (≈ 10 dB for 90 % availability, 20 dB for 99 %).
//!
//! **PHYs** (Bluetooth Core Specification v5.x, Vol 6 Part A/B): LE 1M
//! (1 Mb/s), LE 2M (2 Mb/s) and LE Coded with S = 2 (500 kb/s) or S = 8
//! (125 kb/s) forward-error-correction coding. The specification only sets
//! a minimum receiver sensitivity (−70 dBm for the uncoded PHYs); the values
//! below are **typical of current BLE system-on-chip (SoC) data sheets** —
//! check the data sheet of the part you actually use.

use serde::{Deserialize, Serialize};

use super::tline::C0;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Phy {
    pub id: &'static str,
    pub name: &'static str,
    pub rate_kbps: f64,
    /// Typical SoC sensitivity (dBm) — typical, not a specification.
    pub typ_sens_dbm: f64,
}

pub const PHYS: [Phy; 4] = [
    Phy {
        id: "1m",
        name: "LE 1M",
        rate_kbps: 1000.0,
        typ_sens_dbm: -95.0,
    },
    Phy {
        id: "2m",
        name: "LE 2M",
        rate_kbps: 2000.0,
        typ_sens_dbm: -92.0,
    },
    Phy {
        id: "s2",
        name: "LE Coded S=2",
        rate_kbps: 500.0,
        typ_sens_dbm: -99.0,
    },
    Phy {
        id: "s8",
        name: "LE Coded S=8",
        rate_kbps: 125.0,
        typ_sens_dbm: -103.0,
    },
];

fn phy(id: &str) -> &'static Phy {
    PHYS.iter().find(|p| p.id == id).unwrap_or(&PHYS[0])
}

pub fn fspl_db(d_m: f64, f_ghz: f64) -> f64 {
    let lam = C0 / (f_ghz * 1e9);
    20.0 * (4.0 * std::f64::consts::PI * d_m.max(1e-3) / lam).log10()
}

/// Log-distance path loss with d0 = 1 m (free space below d0).
pub fn path_loss_db(d_m: f64, f_ghz: f64, n: f64) -> f64 {
    if d_m <= 1.0 {
        fspl_db(d_m, f_ghz)
    } else {
        fspl_db(1.0, f_ghz) + 10.0 * n * d_m.log10()
    }
}

/// Rayleigh fade margin for an availability (0..1).
pub fn rayleigh_fade_db(availability: f64) -> f64 {
    let p_out = (1.0 - availability).clamp(1e-9, 0.99);
    -10.0 * (-(1.0 - p_out).ln()).log10()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LinkIn {
    pub phy: String,
    pub f_ghz: f64,
    pub distance_m: f64,
    pub path_exponent: f64,
    /// Glasses side.
    pub tx_dbm: f64,
    pub glasses_gain_dbi: f64,
    /// Mismatch + matching-network loss at the glasses antenna port (dB, ≥ 0).
    pub match_loss_db: f64,
    /// Offset from the typical SoC sensitivity (dB; positive = worse).
    pub glasses_sens_offset_db: f64,
    /// Phone side (typical assumptions).
    pub phone_tx_dbm: f64,
    pub phone_gain_dbi: f64,
    /// Phone sensitivity on LE 1M; other PHYs follow the typical offsets.
    pub phone_sens_1m_dbm: f64,
    /// Glasses antenna next to the head (absorption + shadowing).
    pub head_loss_db: f64,
    /// Phone in a hand / pocket.
    pub body_loss_db: f64,
    pub other_loss_db: f64,
    /// Link availability for the Rayleigh fade margin (0..1).
    pub availability: f64,
}

impl Default for LinkIn {
    fn default() -> Self {
        LinkIn {
            phy: "1m".into(),
            f_ghz: 2.44,
            distance_m: 5.0,
            path_exponent: 2.5,
            tx_dbm: 0.0,
            glasses_gain_dbi: -1.25,
            match_loss_db: 0.5,
            glasses_sens_offset_db: 0.0,
            phone_tx_dbm: 0.0,
            phone_gain_dbi: -2.0,
            phone_sens_1m_dbm: -90.0,
            head_loss_db: 6.0,
            body_loss_db: 4.0,
            other_loss_db: 0.0,
            availability: 0.9,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Step {
    pub label: String,
    pub delta_db: f64,
    pub level_dbm: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Direction {
    pub name: String,
    pub steps: Vec<Step>,
    pub rx_dbm: f64,
    pub sens_dbm: f64,
    pub fade_db: f64,
    pub margin_db: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhyRange {
    pub id: String,
    pub name: String,
    pub rate_kbps: f64,
    pub margin_db: f64,
    pub range_m: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkOut {
    pub phy: String,
    pub path_loss_db: f64,
    pub fspl_1m_db: f64,
    pub fade_db: f64,
    pub forward: Direction,
    pub reverse: Direction,
    pub margin_db: f64,
    pub limiting: String,
    pub range_m: f64,
    pub phys: Vec<PhyRange>,
    /// Received power vs distance (limiting direction), for the plot.
    pub dist_m: Vec<f64>,
    pub rx_fwd_dbm: Vec<f64>,
    pub rx_rev_dbm: Vec<f64>,
    pub notes: Vec<String>,
}

struct Dir {
    tx: f64,
    gt: f64,
    gr: f64,
    sens: f64,
}

fn direction(i: &LinkIn, name: &str, d: &Dir, pl: f64, fade: f64) -> Direction {
    let mut steps = Vec::new();
    let mut lvl = d.tx;
    let mut push = |label: &str, delta: f64, steps: &mut Vec<Step>| {
        lvl += delta;
        steps.push(Step {
            label: label.into(),
            delta_db: delta,
            level_dbm: lvl,
        });
    };
    steps.push(Step {
        label: "Transmit power".into(),
        delta_db: d.tx,
        level_dbm: d.tx,
    });
    push("TX antenna gain", d.gt, &mut steps);
    push("Glasses match loss", -i.match_loss_db.max(0.0), &mut steps);
    push(
        &format!("Path loss ({:.1} m, n = {:.1})", i.distance_m, i.path_exponent),
        -pl,
        &mut steps,
    );
    push("Head loss (glasses)", -i.head_loss_db.max(0.0), &mut steps);
    push("Body loss (phone)", -i.body_loss_db.max(0.0), &mut steps);
    if i.other_loss_db.abs() > 1e-9 {
        push("Other losses", -i.other_loss_db, &mut steps);
    }
    push("RX antenna gain", d.gr, &mut steps);
    Direction {
        name: name.into(),
        rx_dbm: lvl,
        sens_dbm: d.sens,
        fade_db: fade,
        margin_db: lvl - d.sens - fade,
        steps,
    }
}

fn dirs(i: &LinkIn, p: &Phy) -> (Dir, Dir) {
    let off = p.typ_sens_dbm - PHYS[0].typ_sens_dbm;
    (
        Dir {
            tx: i.tx_dbm,
            gt: i.glasses_gain_dbi,
            gr: i.phone_gain_dbi,
            sens: i.phone_sens_1m_dbm + off,
        },
        Dir {
            tx: i.phone_tx_dbm,
            gt: i.phone_gain_dbi,
            gr: i.glasses_gain_dbi,
            sens: p.typ_sens_dbm + i.glasses_sens_offset_db,
        },
    )
}

/// Fixed losses and gains excluding path loss (dB, same for any distance).
fn fixed_budget(i: &LinkIn, d: &Dir, fade: f64) -> f64 {
    d.tx + d.gt + d.gr - i.match_loss_db.max(0.0) - i.head_loss_db.max(0.0) - i.body_loss_db.max(0.0) - i.other_loss_db - d.sens - fade
}

/// Distance where the path loss uses up the fixed budget (margin = 0).
fn range_for(budget: f64, f_ghz: f64, n: f64) -> f64 {
    let l1 = fspl_db(1.0, f_ghz);
    if budget <= l1 {
        // inside 1 m: free space
        let lam = C0 / (f_ghz * 1e9);
        return (10f64.powf(budget / 20.0) * lam / (4.0 * std::f64::consts::PI)).max(0.0);
    }
    10f64.powf((budget - l1) / (10.0 * n.max(0.5)))
}

pub fn run(i: LinkIn) -> Result<LinkOut, String> {
    if !(i.f_ghz > 0.0 && i.distance_m > 0.0) {
        return Err("need f > 0 and distance > 0".into());
    }
    let p = phy(&i.phy);
    let pl = path_loss_db(i.distance_m, i.f_ghz, i.path_exponent);
    let fade = rayleigh_fade_db(i.availability);
    let (df, dr) = dirs(&i, p);
    let fwd = direction(&i, "Glasses → phone", &df, pl, fade);
    let rev = direction(&i, "Phone → glasses", &dr, pl, fade);
    let (margin, limiting) = if fwd.margin_db <= rev.margin_db {
        (fwd.margin_db, fwd.name.clone())
    } else {
        (rev.margin_db, rev.name.clone())
    };
    let phys = PHYS
        .iter()
        .map(|q| {
            let (a, b) = dirs(&i, q);
            let budget = fixed_budget(&i, &a, fade).min(fixed_budget(&i, &b, fade));
            PhyRange {
                id: q.id.into(),
                name: q.name.into(),
                rate_kbps: q.rate_kbps,
                margin_db: budget - pl,
                range_m: range_for(budget, i.f_ghz, i.path_exponent),
            }
        })
        .collect::<Vec<_>>();
    let range = phys.iter().find(|x| x.id == p.id).map(|x| x.range_m).unwrap_or(0.0);
    let dist = crate::math::logspace(0.3, 300.0, 90);
    let bf = fixed_budget(&i, &df, 0.0) + df.sens;
    let br = fixed_budget(&i, &dr, 0.0) + dr.sens;
    Ok(LinkOut {
        phy: p.name.into(),
        path_loss_db: pl,
        fspl_1m_db: fspl_db(1.0, i.f_ghz),
        fade_db: fade,
        rx_fwd_dbm: dist.iter().map(|&d| bf - path_loss_db(d, i.f_ghz, i.path_exponent)).collect(),
        rx_rev_dbm: dist.iter().map(|&d| br - path_loss_db(d, i.f_ghz, i.path_exponent)).collect(),
        dist_m: dist,
        forward: fwd,
        reverse: rev,
        margin_db: margin,
        limiting,
        range_m: range,
        phys,
        notes: vec![
            "Sensitivities are typical of current BLE SoC data sheets; the Bluetooth Core Specification only requires −70 dBm on the uncoded PHYs.".into(),
            "Phone transmit power, antenna gain and sensitivity vary by model — the defaults are conservative assumptions, not measurements.".into(),
            format!("Rayleigh fade margin {:.1} dB for {:.0} % availability (multipath indoors).", fade, 100.0 * i.availability),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friis_at_known_distance() {
        // 2.44 GHz: λ = 122.87 mm; FSPL(1 m) = 20 log10(4π/0.12287) = 40.19 dB;
        // FSPL(10 m) = 60.19 dB (hand calculation).
        assert!((fspl_db(1.0, 2.44) - 40.19).abs() < 0.01);
        assert!((fspl_db(10.0, 2.44) - 60.19).abs() < 0.01);
        assert!((path_loss_db(10.0, 2.44, 3.0) - 70.19).abs() < 0.01);
    }

    #[test]
    fn rayleigh_fade_margins() {
        assert!((rayleigh_fade_db(0.9) - 9.77).abs() < 0.02);
        assert!((rayleigh_fade_db(0.99) - 19.98).abs() < 0.02);
    }

    #[test]
    fn range_gives_zero_margin() {
        let o = run(LinkIn::default()).unwrap();
        let at = run(LinkIn {
            distance_m: o.range_m,
            ..LinkIn::default()
        })
        .unwrap();
        assert!(at.margin_db.abs() < 1e-6, "{}", at.margin_db);
        let s8 = o.phys.iter().find(|p| p.id == "s8").unwrap();
        assert!(s8.range_m > o.range_m);
    }
}
