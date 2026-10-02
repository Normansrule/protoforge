//! Anthropometry: head and face dimensions of adult men and women as normal
//! approximations, percentile tables, a seeded Monte Carlo population and the
//! fit of a spectacle frame to each simulated person.
//!
//! ## Data
//! * **Head breadth** and **interpupillary distance (IPD)** — means and
//!   standard deviations computed in this project from the public
//!   *ANSUR II* working data (4,082 men, 1,986 women; Gordon et al. 2014,
//!   US Army Natick technical report NATICK/TR-15/007). ANSUR II records
//!   IPD in tenths of a millimetre; it is converted here. The empirical 1st …
//!   99th percentiles of the same files are kept alongside so the page can show
//!   how close the normal approximation is (skewness is < 0.25 for both).
//!   The adult IPD range agrees with Dodgson (2004): mean ≈ 63 mm, nearly all
//!   adults within 50–75 mm.
//! * **Nose bridge width** (at the height where nose pads sit) and
//!   **hinge-to-ear distance** (length from the frame hinge to the top of the
//!   ear root, i.e. the temple "length to bend" a person needs) are *not* in
//!   ANSUR II. The values here are **engineering assumptions** chosen to be
//!   consistent with common eyewear sizing (bridges 14–24 mm, temples
//!   135–150 mm with ≈ 35–45 mm behind the bend); the women/men ratio and the
//!   spread follow ANSUR II head length. They are labelled as such everywhere
//!   and can be replaced (`design.extra.ergonomics.overrides`) with measured
//!   data, e.g. from a 3-D head-scan study.
//!
//! Correlations between dimensions (for the Monte Carlo sample) are ANSUR II
//! Pearson coefficients: head breadth–IPD 0.31 (men) / 0.28 (women); the ear
//! distance borrows head length's correlations (0.18/0.22 with head breadth,
//! 0.27/0.34 with IPD); the nose is sampled independently.
//!
//! Percentile definitions follow ISO 7250-1 measurement conventions and the
//! percentile practice of Pheasant & Haslegrave, *Bodyspace* (3rd ed., 2006):
//! `x_p = μ + z_p σ` with `z_5 = −1.645`, `z_95 = +1.645`.

use serde::{Deserialize, Serialize};

use super::model::{ErgoConfig, FrameGeom};
use super::stats::{cholesky, Mixture, Normal};
use crate::math::Rng;

pub const PERCENTILES: [f64; 9] = [1.0, 5.0, 10.0, 25.0, 50.0, 75.0, 90.0, 95.0, 99.0];
/// Percentiles at which ANSUR II empirical values are stored.
pub const ANSUR_P: [f64; 7] = [1.0, 5.0, 25.0, 50.0, 75.0, 95.0, 99.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    Men,
    Women,
    Combined,
}

/// One body dimension.
#[derive(Clone, Debug, Serialize)]
pub struct Dim {
    pub id: &'static str,
    pub label: &'static str,
    pub symbol: &'static str,
    pub unit: &'static str,
    pub men: Normal,
    pub women: Normal,
    /// true = from a measured dataset; false = engineering assumption.
    pub measured: bool,
    pub source: &'static str,
    pub definition: &'static str,
    /// Empirical ANSUR II percentiles [men, women] at `ANSUR_P`, when measured.
    pub ansur: Option<[[f64; 7]; 2]>,
    /// true when the user replaced the statistics.
    pub overridden: bool,
}

pub const HEAD_BREADTH: usize = 0;
pub const IPD: usize = 1;
pub const EAR: usize = 2;
pub const NOSE: usize = 3;

/// The four dimensions, with any user overrides applied.
pub fn dims(cfg: &ErgoConfig) -> Vec<Dim> {
    let mut d = vec![
        Dim {
            id: "head_breadth",
            label: "Head breadth",
            symbol: "HB",
            unit: "mm",
            men: Normal::new(154.33, 5.53),
            women: Normal::new(147.77, 5.19),
            measured: true,
            source: "ANSUR II public data (Gordon et al. 2014), computed in this project",
            definition: "Maximum horizontal breadth of the head above the ears (euryon to euryon).",
            ansur: Some([[142.0, 146.0, 151.0, 154.0, 158.0, 163.0, 168.0], [136.0, 140.0, 144.0, 147.0, 151.0, 156.0, 160.0]]),
            overridden: false,
        },
        Dim {
            id: "ipd",
            label: "Interpupillary distance",
            symbol: "IPD",
            unit: "mm",
            men: Normal::new(64.02, 3.42),
            women: Normal::new(61.71, 3.62),
            measured: true,
            source: "ANSUR II public data (Gordon et al. 2014), computed in this project; range agrees with Dodgson 2004",
            definition: "Distance between the centres of the pupils, eyes looking straight ahead at a far target.",
            ansur: Some([[56.0, 58.5, 61.5, 64.0, 66.0, 70.0, 72.5], [53.5, 55.5, 59.5, 62.0, 64.0, 67.5, 70.5]]),
            overridden: false,
        },
        Dim {
            id: "ear_distance",
            label: "Hinge-to-ear distance",
            symbol: "E",
            unit: "mm",
            men: Normal::new(103.0, 5.0),
            women: Normal::new(98.0, 5.0),
            measured: false,
            source: "Engineering assumption (typical eyewear sizing; spread and women/men ratio from ANSUR II head length) — replace with measured data",
            definition: "Along the temple, from the frame hinge to the top of the ear root: the length-to-bend a person needs.",
            ansur: None,
            overridden: false,
        },
        Dim {
            id: "nose_bridge",
            label: "Nose bridge width",
            symbol: "N",
            unit: "mm",
            men: Normal::new(18.5, 2.5),
            women: Normal::new(17.5, 2.5),
            measured: false,
            source: "Engineering assumption (typical spectacle bridge sizes 14–24 mm) — replace with measured data",
            definition: "Width of the nose where the nose pads sit, about 10 mm below the nasal root.",
            ansur: None,
            overridden: false,
        },
    ];
    for dim in d.iter_mut() {
        if let Some(o) = cfg.overrides.get(dim.id) {
            if let Some([m, s]) = o.men {
                if m.is_finite() && s.is_finite() && s > 0.0 {
                    dim.men = Normal::new(m, s);
                    dim.overridden = true;
                }
            }
            if let Some([m, s]) = o.women {
                if m.is_finite() && s.is_finite() && s > 0.0 {
                    dim.women = Normal::new(m, s);
                    dim.overridden = true;
                }
            }
        }
    }
    d
}

/// Correlation matrices [men, women] for (HB, IPD, E, N).
pub fn correlations() -> [[[f64; 4]; 4]; 2] {
    [
        [
            [1.0, 0.306, 0.176, 0.0],
            [0.306, 1.0, 0.268, 0.0],
            [0.176, 0.268, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
        [
            [1.0, 0.284, 0.220, 0.0],
            [0.284, 1.0, 0.343, 0.0],
            [0.220, 0.343, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    ]
}

impl Dim {
    pub fn group(&self, g: Group, female_fraction: f64) -> Mixture {
        match g {
            Group::Men => Mixture(vec![(1.0, self.men)]),
            Group::Women => Mixture(vec![(1.0, self.women)]),
            Group::Combined => Mixture(vec![(1.0 - female_fraction, self.men), (female_fraction, self.women)]),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct GroupStats {
    pub group: Group,
    pub mean: f64,
    pub sd: f64,
    /// Values at `PERCENTILES`.
    pub percentiles: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DimTable {
    pub dim: Dim,
    pub groups: Vec<GroupStats>,
    /// Probability density curves for plotting: x and one y per group.
    pub x: Vec<f64>,
    pub pdf: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PopulationOut {
    pub percentiles: Vec<f64>,
    pub ansur_percentiles: Vec<f64>,
    pub female_fraction: f64,
    pub tables: Vec<DimTable>,
    pub correlations: [[[f64; 4]; 4]; 2],
}

pub fn population(cfg: &ErgoConfig) -> PopulationOut {
    let ff = cfg.female_fraction;
    let tables = dims(cfg)
        .into_iter()
        .map(|dim| {
            let groups: Vec<GroupStats> = [Group::Men, Group::Women, Group::Combined]
                .iter()
                .map(|&g| {
                    let m = dim.group(g, ff);
                    GroupStats {
                        group: g,
                        mean: m.mean(),
                        sd: m.sd(),
                        percentiles: PERCENTILES.iter().map(|p| m.quantile(p / 100.0)).collect(),
                    }
                })
                .collect();
            let lo = dim.men.mean.min(dim.women.mean) - 4.0 * dim.men.sd.max(dim.women.sd);
            let hi = dim.men.mean.max(dim.women.mean) + 4.0 * dim.men.sd.max(dim.women.sd);
            let x = crate::math::linspace(lo, hi, 121);
            let pdf = [Group::Men, Group::Women, Group::Combined]
                .iter()
                .map(|&g| {
                    let m = dim.group(g, ff);
                    x.iter().map(|&v| m.pdf(v)).collect()
                })
                .collect();
            DimTable { dim, groups, x, pdf }
        })
        .collect();
    PopulationOut {
        percentiles: PERCENTILES.to_vec(),
        ansur_percentiles: ANSUR_P.to_vec(),
        female_fraction: ff,
        tables,
        correlations: correlations(),
    }
}

/// One simulated person.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Person {
    pub female: bool,
    pub head_breadth: f64,
    pub ipd: f64,
    pub ear: f64,
    pub nose: f64,
    /// Right monocular PD minus IPD/2 (face asymmetry), mm.
    pub asym: f64,
}

/// Seeded multivariate-normal sample of the population.
pub fn sample(cfg: &ErgoConfig, n: usize, seed: u64) -> Vec<Person> {
    let d = dims(cfg);
    let corr = correlations();
    let chol: Vec<Vec<Vec<f64>>> = corr
        .iter()
        .map(|c| {
            let m: Vec<Vec<f64>> = c.iter().map(|r| r.to_vec()).collect();
            cholesky(&m).unwrap_or_else(|| (0..4).map(|i| (0..4).map(|j| if i == j { 1.0 } else { 0.0 }).collect()).collect())
        })
        .collect();
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let female = rng.uniform() < cfg.female_fraction;
            let k = usize::from(female);
            let z: Vec<f64> = (0..4).map(|_| rng.normal()).collect();
            let l = &chol[k];
            let c: Vec<f64> = (0..4).map(|i| (0..=i).map(|j| l[i][j] * z[j]).sum()).collect();
            let v = |i: usize| {
                let nrm = if female { d[i].women } else { d[i].men };
                nrm.mean + nrm.sd * c[i]
            };
            let asym = if cfg.asymmetry_sd_mm > 0.0 {
                cfg.asymmetry_sd_mm * rng.normal()
            } else {
                0.0
            };
            Person {
                female,
                head_breadth: v(HEAD_BREADTH),
                ipd: v(IPD),
                ear: v(EAR),
                nose: v(NOSE),
                asym,
            }
        })
        .collect()
}

/// Which fit rule a person fails (for the Pareto of reasons).
#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq)]
pub struct FitCheck {
    pub pd_ok: bool,
    pub ear_ok: bool,
    pub width_ok: bool,
    pub nose_ok: bool,
    /// Clamp force per side, N.
    pub clamp_n: f64,
    /// |frame PD − IPD| / 2, mm.
    pub decentration_mm: f64,
    /// Temple bend − ear distance, mm (> 0: bend behind the ear).
    pub bend_minus_ear_mm: f64,
    pub fits: bool,
}

/// The clamp-force window a frame must land in for a given person (N per side).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Band {
    pub min_n: f64,
    pub max_n: f64,
}

/// Evaluate one person against one frame. `clamp` maps head breadth → force.
pub fn check(frame: &FrameGeom, cfg: &ErgoConfig, band: Band, person: &Person, clamp: &dyn Fn(f64) -> f64) -> FitCheck {
    let dec = (frame.frame_pd_mm - person.ipd).abs() / 2.0;
    let pd_ok = dec <= cfg.pd_tolerance_mm;
    let dbe = frame.bend_from_pin_mm - person.ear;
    let ear_ok = dbe >= cfg.ear_window_mm[0] && dbe <= cfg.ear_window_mm[1];
    let f = clamp(person.head_breadth);
    let width_ok = f >= band.min_n && f <= band.max_n;
    let dn = frame.bridge_mm - person.nose;
    let nose_ok = !cfg.nose_check || (dn >= cfg.nose_window_mm[0] && dn <= cfg.nose_window_mm[1]);
    FitCheck {
        pd_ok,
        ear_ok,
        width_ok,
        nose_ok,
        clamp_n: f,
        decentration_mm: dec,
        bend_minus_ear_mm: dbe,
        fits: pd_ok && ear_ok && width_ok && nose_ok,
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Coverage {
    pub n: usize,
    pub fit_pct: f64,
    pub pd_pct: f64,
    pub ear_pct: f64,
    pub width_pct: f64,
    pub nose_pct: f64,
    pub too_tight_pct: f64,
    pub too_loose_pct: f64,
}

pub fn coverage<'a>(checks: impl Iterator<Item = &'a FitCheck>, band: Band) -> Coverage {
    let mut c = Coverage::default();
    let (mut fit, mut pd, mut ear, mut w, mut nose, mut tight, mut loose) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    for k in checks {
        c.n += 1;
        fit += usize::from(k.fits);
        pd += usize::from(k.pd_ok);
        ear += usize::from(k.ear_ok);
        w += usize::from(k.width_ok);
        nose += usize::from(k.nose_ok);
        tight += usize::from(k.clamp_n > band.max_n);
        loose += usize::from(k.clamp_n < band.min_n);
    }
    let pct = |v: usize| if c.n > 0 { 100.0 * v as f64 / c.n as f64 } else { 0.0 };
    c.fit_pct = pct(fit);
    c.pd_pct = pct(pd);
    c.ear_pct = pct(ear);
    c.width_pct = pct(w);
    c.nose_pct = pct(nose);
    c.too_tight_pct = pct(tight);
    c.too_loose_pct = pct(loose);
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_follow_mean_plus_z_sigma() {
        let cfg = ErgoConfig::default();
        let pop = population(&cfg);
        let hb = &pop.tables[HEAD_BREADTH];
        let men = &hb.groups[0];
        // 95th percentile = μ + 1.6449 σ = 154.33 + 1.6449·5.53 = 163.43 mm
        assert!(
            (men.percentiles[7] - (154.33 + 1.644_854 * 5.53)).abs() < 1e-3,
            "{}",
            men.percentiles[7]
        );
        // …and lands on the ANSUR II empirical 95th percentile (163 mm) within 1 mm
        assert!((men.percentiles[7] - 163.0).abs() < 1.0);
        // women's 5th percentile IPD: 61.71 − 1.6449·3.62 = 55.75 (ANSUR II: 55.5)
        let ipd_w = &pop.tables[IPD].groups[1];
        assert!((ipd_w.percentiles[1] - 55.755).abs() < 0.01, "{}", ipd_w.percentiles[1]);
        // combined median sits between the two sexes
        let comb = &hb.groups[2];
        assert!(comb.percentiles[4] > 147.77 && comb.percentiles[4] < 154.33);
    }

    #[test]
    fn sample_reproduces_means_and_correlation() {
        let cfg = ErgoConfig {
            female_fraction: 0.0,
            ..Default::default()
        };
        let s = sample(&cfg, 20_000, 3);
        let n = s.len() as f64;
        let m_hb = s.iter().map(|p| p.head_breadth).sum::<f64>() / n;
        let m_ipd = s.iter().map(|p| p.ipd).sum::<f64>() / n;
        assert!((m_hb - 154.33).abs() < 0.15, "{m_hb}");
        assert!((m_ipd - 64.02).abs() < 0.1, "{m_ipd}");
        let cov = s.iter().map(|p| (p.head_breadth - m_hb) * (p.ipd - m_ipd)).sum::<f64>() / n;
        let r = cov / (5.53 * 3.42);
        assert!((r - 0.306).abs() < 0.03, "{r}");
        // deterministic
        assert_eq!(sample(&cfg, 5, 9), sample(&cfg, 5, 9));
    }

    #[test]
    fn overrides_replace_statistics() {
        let mut cfg = ErgoConfig::default();
        cfg.overrides.insert(
            "nose_bridge".into(),
            super::super::model::DimOverride {
                men: Some([20.0, 1.0]),
                women: None,
            },
        );
        let d = dims(&cfg);
        assert_eq!(d[NOSE].men, Normal::new(20.0, 1.0));
        assert!(d[NOSE].overridden);
        assert_eq!(d[NOSE].women, Normal::new(17.5, 2.5));
    }
}
