//! Helical springs per Budynas & Nisbett, *Shigley's Mechanical Engineering
//! Design* (10th ed.), chapter 10 — compression, extension and torsion.
//!
//! * **Ultimate strength** of spring wire falls with diameter:
//!   `S_ut = A / d^m` (Shigley Table 10-4, SI constants in MPa·mm^m).
//! * **Rate** of a compression/extension spring: `k = d⁴ G / (8 D³ N_a)`.
//! * **Curvature correction**: Bergsträsser `K_B = (4C + 2)/(4C − 3)` (used
//!   for stress) and Wahl `K_W = (4C − 1)/(4C − 4) + 0.615/C` (shown for
//!   comparison), `C = D/d` the spring index. Shear stress `τ = K_B 8 F D / (π d³)`.
//! * **Solid height and ends** (Table 10-1), **buckling** (eq. 10-10 with
//!   the end-condition constant α of Table 10-2).
//! * **Fatigue** with the Zimmerli endurance data (`S_sa = 241 MPa`,
//!   `S_sm = 379 MPa` unpeened; `398 / 534 MPa` peened) on a Goodman line to
//!   `S_su = 0.67 S_ut`, and a finite-life estimate from the Associated
//!   Spring cyclic allowables Shigley tabulates as Table 10-8 (percent of
//!   `S_ut` for zero-to-peak torsion at 10⁵, 10⁶, 10⁷ cycles).
//! * **Torsion springs**: `k' = d⁴ E / (10.8 D N_a)` N·mm per turn with
//!   `N_a = N_b + (l₁ + l₂)/(3πD)`, bending stress `σ = K_i 32 M/(π d³)`,
//!   `K_i = (4C² − C − 1)/(4C(C − 1))`, static yield `S_y ≈ 0.78 S_ut`
//!   (music wire, hard-drawn) or `0.61 S_ut` (austenitic stainless).
//! * **Extension springs**: hook bending at A and torsion at B with the
//!   curvature factors `(K)_A = (4C₁² − C₁ − 1)/(4C₁(C₁ − 1))`,
//!   `(K)_B = (4C₂ − 1)/(4C₂ − 4)`; preferred initial-tension stress range
//!   `τ_i = 231 e^(−0.105 C) ± 6.9 (4 − (C − 3)/6.5)` MPa (eq. 10-40 in SI).
//!
//! Table values are the published ones as tabulated by Shigley; always
//! check your spring maker's data (Associated Spring, Lee Spring … publish
//! their own) — the fatigue numbers in particular scatter.

use serde::{Deserialize, Serialize};

/// Spring wire material (Shigley Tables 10-4 and 10-5).
#[derive(Clone, Debug, Serialize)]
pub struct WireMaterial {
    pub id: &'static str,
    pub name: &'static str,
    pub spec: &'static str,
    /// `S_ut = A / d^m`, A in MPa·mm^m.
    pub a_mpa: f64,
    pub m: f64,
    /// Diameter range the A, m fit is published for, mm.
    pub d_min_mm: f64,
    pub d_max_mm: f64,
    /// Static torsional yield as a fraction of S_ut (Table 10-6, before set removal).
    pub ssy_frac: f64,
    /// Static bending yield for torsion springs as a fraction of S_ut (eq. 10-57).
    pub sy_bend_frac: f64,
    pub density_g_cm3: f64,
    pub max_temp_c: f64,
    pub notes: &'static str,
}

pub const WIRES: &[WireMaterial] = &[
    WireMaterial {
        id: "music_wire",
        name: "Music wire",
        spec: "ASTM A228",
        a_mpa: 2211.0,
        m: 0.145,
        d_min_mm: 0.10,
        d_max_mm: 6.5,
        ssy_frac: 0.45,
        sy_bend_frac: 0.78,
        density_g_cm3: 7.85,
        max_temp_c: 120.0,
        notes: "Best, toughest, most widely used small-spring material; high fatigue strength. Not corrosion resistant (plate or oil it).",
    },
    WireMaterial {
        id: "hard_drawn",
        name: "Hard-drawn spring wire",
        spec: "ASTM A227",
        a_mpa: 1783.0,
        m: 0.190,
        d_min_mm: 0.7,
        d_max_mm: 12.7,
        ssy_frac: 0.45,
        sy_bend_frac: 0.78,
        density_g_cm3: 7.85,
        max_temp_c: 120.0,
        notes: "Cheapest general-purpose spring steel; use where life, accuracy and deflection are not critical. Published sizes start at 0.7 mm.",
    },
    WireMaterial {
        id: "stainless_302",
        name: "Stainless steel 302",
        spec: "ASTM A313 (302)",
        a_mpa: 1867.0,
        m: 0.146,
        d_min_mm: 0.3,
        d_max_mm: 2.5,
        ssy_frac: 0.35,
        sy_bend_frac: 0.61,
        density_g_cm3: 7.92,
        max_temp_c: 260.0,
        notes: "Corrosion resistant (skin contact, sweat): the usual choice for eyewear spring hinges. Lower allowable stress than music wire.",
    },
];

pub fn wire(id: &str) -> &'static WireMaterial {
    let k = id.to_ascii_lowercase().replace([' ', '-'], "_");
    WIRES
        .iter()
        .find(|w| w.id == k || w.spec.to_ascii_lowercase().contains(&k))
        .or_else(|| match k.as_str() {
            "music" | "a228" => Some(&WIRES[0]),
            "a227" | "harddrawn" => Some(&WIRES[1]),
            "302" | "ss302" | "stainless" | "a313" => Some(&WIRES[2]),
            _ => None,
        })
        .unwrap_or(&WIRES[0])
}

/// Elastic moduli (E, G) in MPa, Shigley Table 10-5 (size-dependent for the carbon steels).
pub fn moduli(w: &WireMaterial, d_mm: f64) -> (f64, f64) {
    match w.id {
        "music_wire" => {
            if d_mm < 0.8 {
                (203_400.0, 82_700.0)
            } else if d_mm < 1.6 {
                (200_000.0, 81_700.0)
            } else if d_mm < 3.2 {
                (196_500.0, 81_000.0)
            } else {
                (193_000.0, 80_000.0)
            }
        }
        "hard_drawn" => {
            if d_mm < 0.8 {
                (198_600.0, 80_700.0)
            } else if d_mm < 1.6 {
                (197_900.0, 80_000.0)
            } else if d_mm < 3.2 {
                (197_200.0, 79_300.0)
            } else {
                (196_500.0, 78_600.0)
            }
        }
        _ => (193_000.0, 69_000.0),
    }
}

/// Minimum tensile strength, MPa.
pub fn sut(w: &WireMaterial, d_mm: f64) -> f64 {
    w.a_mpa / d_mm.max(1e-3).powf(w.m)
}

pub fn bergstrasser(c: f64) -> f64 {
    (4.0 * c + 2.0) / (4.0 * c - 3.0)
}

pub fn wahl(c: f64) -> f64 {
    (4.0 * c - 1.0) / (4.0 * c - 4.0) + 0.615 / c
}

/// Zimmerli endurance components (S_sa, S_sm), MPa.
pub fn zimmerli(peened: bool) -> (f64, f64) {
    if peened {
        (398.0, 534.0)
    } else {
        (241.0, 379.0)
    }
}

/// Goodman endurance amplitude for zero-mean: `S_se = S_sa / (1 − S_sm/S_su)`.
pub fn goodman_sse(peened: bool, ssu: f64) -> f64 {
    let (sa, sm) = zimmerli(peened);
    sa / (1.0 - sm / ssu).max(1e-6)
}

/// Goodman fatigue safety factor for a torsional stress cycle.
pub fn goodman_nf(tau_a: f64, tau_m: f64, sse: f64, ssu: f64) -> f64 {
    let d = tau_a / sse + tau_m.max(0.0) / ssu;
    if d <= 0.0 {
        f64::INFINITY
    } else {
        1.0 / d
    }
}

/// Associated Spring cyclic allowables (Shigley Table 10-8): maximum
/// torsional stress (zero-to-peak, K_B corrected) as a fraction of S_ut at
/// 10⁵, 10⁶, 10⁷ cycles for music wire, 302 stainless and non-ferrous wire.
/// Hard-drawn A227 is not in the table; the same row is used (optimistic).
pub fn cyclic_allowable(peened: bool) -> [f64; 3] {
    if peened {
        [0.42, 0.39, 0.36]
    } else {
        [0.36, 0.33, 0.30]
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct LifeEstimate {
    /// Cycles to failure (capped at 1e12 when below the 10⁷ allowable).
    pub cycles: f64,
    /// Below the 10⁷-cycle allowable: treat as infinite life (runout).
    pub runout: bool,
    /// Outside the tabulated 10⁵–10⁷ range: extrapolated (low confidence).
    pub extrapolated: bool,
    /// Equivalent zero-to-peak stress as a fraction of S_ut.
    pub r0_frac: f64,
}

/// Finite-life estimate: convert (τ_a, τ_m) to the zero-to-peak stress
/// with the same Goodman damage (`S₀ = 2 S_er / (1 + S_er/S_su)` with
/// `S_er = τ_a / (1 − τ_m/S_su)`), then interpolate the Table 10-8 allowables
/// log-linearly in cycles.
pub fn life(tau_a: f64, tau_m: f64, sut: f64, peened: bool) -> LifeEstimate {
    let ssu = 0.67 * sut;
    if tau_a <= 0.0 {
        return LifeEstimate {
            cycles: 1e12,
            runout: true,
            extrapolated: false,
            r0_frac: 0.0,
        };
    }
    if tau_m >= ssu {
        return LifeEstimate {
            cycles: 0.5,
            runout: false,
            extrapolated: true,
            r0_frac: 1.0,
        };
    }
    let ser = tau_a / (1.0 - tau_m.max(0.0) / ssu);
    let s0 = 2.0 * ser / (1.0 + ser / ssu);
    let f = s0 / sut;
    let t = cyclic_allowable(peened);
    if f <= t[2] {
        return LifeEstimate {
            cycles: 1e12,
            runout: true,
            extrapolated: false,
            r0_frac: f,
        };
    }
    // piecewise linear frac vs log10 N through (5, t0), (6, t1), (7, t2)
    let (lg, extrap) = if f >= t[0] {
        let slope = t[0] - t[1];
        (5.0 - (f - t[0]) / slope, true)
    } else if f >= t[1] {
        (5.0 + (t[0] - f) / (t[0] - t[1]), false)
    } else {
        (6.0 + (t[1] - f) / (t[1] - t[2]), false)
    };
    LifeEstimate {
        cycles: 10f64.powf(lg.max(3.0)),
        runout: false,
        extrapolated: extrap,
        r0_frac: f,
    }
}

// ------------------------------------------------------------------ compression

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct CompressionIn {
    pub material: String,
    pub d_mm: f64,
    /// Mean coil diameter D, mm.
    pub mean_d_mm: f64,
    pub active_coils: f64,
    /// plain | plain_ground | squared | squared_ground
    pub ends: String,
    pub free_len_mm: f64,
    /// Working loads, N.
    pub f_min_n: f64,
    pub f_max_n: f64,
    pub peened: bool,
    /// End condition for buckling: fixed (parallel flat plates) | fixed_pivot | pivoted | clamped_free
    pub support: String,
}

impl Default for CompressionIn {
    fn default() -> Self {
        CompressionIn {
            material: "music_wire".into(),
            d_mm: 1.2,
            mean_d_mm: 10.0,
            active_coils: 8.0,
            ends: "squared_ground".into(),
            free_len_mm: 28.0,
            f_min_n: 10.0,
            f_max_n: 25.0,
            peened: false,
            support: "fixed".into(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Check {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

fn check(name: &str, ok: bool, detail: String) -> Check {
    Check {
        name: name.into(),
        ok,
        detail,
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CompressionOut {
    pub material: String,
    pub sut_mpa: f64,
    pub ssy_mpa: f64,
    pub ssu_mpa: f64,
    pub e_mpa: f64,
    pub g_mpa: f64,
    pub index_c: f64,
    pub k_b: f64,
    pub k_w: f64,
    pub od_mm: f64,
    pub id_mm: f64,
    pub rate_n_mm: f64,
    pub total_coils: f64,
    pub solid_len_mm: f64,
    pub pitch_mm: f64,
    pub len_at_min_mm: f64,
    pub len_at_max_mm: f64,
    pub defl_max_mm: f64,
    /// Force to close the spring solid, N.
    pub f_solid_n: f64,
    pub tau_solid_mpa: f64,
    pub ns_solid: f64,
    pub tau_min_mpa: f64,
    pub tau_max_mpa: f64,
    pub tau_a_mpa: f64,
    pub tau_m_mpa: f64,
    pub ns_static: f64,
    pub sse_mpa: f64,
    /// Goodman (Zimmerli) fatigue factor of safety.
    pub nf_goodman: f64,
    pub life: LifeEstimate,
    /// Fractional overrun to closure ξ = (y_solid − y_max)/y_max (Shigley recommends ≥ 0.15).
    pub overrun: f64,
    pub buckling_alpha: f64,
    pub slenderness: f64,
    pub buckling_stable_absolute: bool,
    /// Critical deflection for buckling (∞ when absolutely stable), mm.
    pub buckling_defl_mm: f64,
    pub surge_hz: f64,
    pub mass_g: f64,
    /// Force–deflection line to solid: deflection, force.
    pub curve_y: Vec<f64>,
    pub curve_f: Vec<f64>,
    pub checks: Vec<Check>,
    pub notes: Vec<String>,
}

/// End-condition constant α for buckling (Shigley Table 10-2).
pub fn buckling_alpha(support: &str) -> f64 {
    match support {
        "fixed_pivot" => 0.707,
        "pivoted" => 1.0,
        "clamped_free" => 2.0,
        _ => 0.5,
    }
}

/// Total coils, solid length and pitch from the end type (Shigley Table 10-1).
pub fn end_geometry(ends: &str, na: f64, d: f64, l0: f64) -> (f64, f64, f64) {
    match ends {
        "plain" => (na, d * (na + 1.0), (l0 - d) / na),
        "plain_ground" => (na + 1.0, d * (na + 1.0), l0 / (na + 1.0)),
        "squared" => (na + 2.0, d * (na + 3.0), (l0 - 3.0 * d) / na),
        _ => (na + 2.0, d * (na + 2.0), (l0 - 2.0 * d) / na),
    }
}

pub fn compression(i: CompressionIn) -> Result<CompressionOut, String> {
    let (d, dm, na, l0) = (i.d_mm, i.mean_d_mm, i.active_coils, i.free_len_mm);
    for (v, n) in [
        (d, "wire diameter"),
        (dm, "mean diameter"),
        (na, "active coils"),
        (l0, "free length"),
    ] {
        if v.is_nan() || v <= 0.0 {
            return Err(format!("{n} must be positive"));
        }
    }
    if dm <= d {
        return Err("mean coil diameter must exceed the wire diameter".into());
    }
    let w = wire(&i.material);
    let (e, g) = moduli(w, d);
    let s_ut = sut(w, d);
    let ssy = w.ssy_frac * s_ut;
    let ssu = 0.67 * s_ut;
    let c = dm / d;
    let kb = bergstrasser(c);
    let kw = wahl(c);
    let k = d.powi(4) * g / (8.0 * dm.powi(3) * na);
    let (nt, ls, pitch) = end_geometry(&i.ends, na, d, l0);
    let tau_per_n = kb * 8.0 * dm / (std::f64::consts::PI * d.powi(3));
    let y_solid = (l0 - ls).max(0.0);
    let f_solid = k * y_solid;
    let tau_solid = tau_per_n * f_solid;
    let (fmin, fmax) = (i.f_min_n.min(i.f_max_n), i.f_max_n.max(i.f_min_n));
    let (tmin, tmax) = (tau_per_n * fmin, tau_per_n * fmax);
    let ta = 0.5 * (tmax - tmin);
    let tm = 0.5 * (tmax + tmin);
    let sse = goodman_sse(i.peened, ssu);
    let nf = goodman_nf(ta, tm, sse, ssu);
    let lf = life(ta, tm, s_ut, i.peened);
    let ymax = fmax / k;
    let overrun = if ymax > 0.0 { (y_solid - ymax) / ymax } else { f64::INFINITY };
    // Buckling (Shigley eq. 10-10): y_cr = L0 C1' [1 − √(1 − C2'/λ_eff²)].
    let alpha = buckling_alpha(&i.support);
    let lam = alpha * l0 / dm;
    let c1 = e / (2.0 * (e - g));
    let c2 = 2.0 * std::f64::consts::PI.powi(2) * (e - g) / (2.0 * g + e);
    let stable = lam * lam <= c2;
    let ycr = if stable {
        f64::INFINITY
    } else {
        l0 * c1 * (1.0 - (1.0 - c2 / (lam * lam)).sqrt())
    };
    let mass_g = w.density_g_cm3 * std::f64::consts::PI.powi(2) * d * d * dm * na / 4.0 / 1000.0;
    // Surge (both ends against flat plates): f = ½ √(k/m) with the active mass.
    let surge = 0.5 * ((k * 1000.0) / (mass_g / 1000.0)).sqrt();
    let od = dm + d;
    let mut checks = vec![
        check("Spring index 4 ≤ C ≤ 12", (4.0..=12.0).contains(&c), format!("C = {c:.2}")),
        check("Active coils 3 ≤ N_a ≤ 15", (3.0..=15.0).contains(&na), format!("N_a = {na:.1}")),
        check(
            "Solid-safe: n_s ≥ 1.2 at solid",
            ssy / tau_solid.max(1e-9) >= 1.2,
            format!("τ_solid = {tau_solid:.0} MPa vs S_sy = {ssy:.0} MPa"),
        ),
        check("Overrun to closure ξ ≥ 0.15", overrun >= 0.15, format!("ξ = {overrun:.2}")),
        check(
            "Static n_s ≥ 1.2 at F_max",
            ssy / tmax.max(1e-9) >= 1.2,
            format!("τ_max = {tmax:.0} MPa, n_s = {:.2}", ssy / tmax.max(1e-9)),
        ),
        check("Fatigue n_f ≥ 1.5 (Goodman, Zimmerli)", nf >= 1.5, format!("n_f = {nf:.2}")),
        check(
            "No buckling",
            stable || ycr > ymax,
            if stable {
                format!("absolutely stable (λ_eff = {lam:.2})")
            } else {
                format!("critical deflection {ycr:.1} mm vs working {ymax:.1} mm")
            },
        ),
    ];
    if d < w.d_min_mm || d > w.d_max_mm {
        checks.push(check(
            "Wire size inside the published A, m fit",
            false,
            format!("{} data cover {:.2}–{:.1} mm", w.name, w.d_min_mm, w.d_max_mm),
        ));
    }
    let n = 24;
    let curve_y: Vec<f64> = (0..=n).map(|q| y_solid * q as f64 / n as f64).collect();
    let curve_f = curve_y.iter().map(|y| k * y).collect();
    Ok(CompressionOut {
        material: w.name.into(),
        sut_mpa: s_ut,
        ssy_mpa: ssy,
        ssu_mpa: ssu,
        e_mpa: e,
        g_mpa: g,
        index_c: c,
        k_b: kb,
        k_w: kw,
        od_mm: od,
        id_mm: dm - d,
        rate_n_mm: k,
        total_coils: nt,
        solid_len_mm: ls,
        pitch_mm: pitch,
        len_at_min_mm: l0 - fmin / k,
        len_at_max_mm: l0 - fmax / k,
        defl_max_mm: ymax,
        f_solid_n: f_solid,
        tau_solid_mpa: tau_solid,
        ns_solid: ssy / tau_solid.max(1e-9),
        tau_min_mpa: tmin,
        tau_max_mpa: tmax,
        tau_a_mpa: ta,
        tau_m_mpa: tm,
        ns_static: ssy / tmax.max(1e-9),
        sse_mpa: sse,
        nf_goodman: nf.min(1e6),
        life: lf,
        overrun: overrun.min(1e6),
        buckling_alpha: alpha,
        slenderness: lam,
        buckling_stable_absolute: stable,
        buckling_defl_mm: ycr.min(1e9),
        surge_hz: surge,
        mass_g,
        curve_y,
        curve_f,
        checks,
        notes: vec![
            format!("S_ut = A/d^m = {:.0}/{d}^{} = {s_ut:.0} MPa ({} {})", w.a_mpa, w.m, w.name, w.spec),
            "Stress uses the Bergsträsser factor K_B (Shigley's choice); the Wahl factor K_W is within about 1 % of it.".into(),
            "Fatigue: Zimmerli's data (unpeened S_sa = 241, S_sm = 379 MPa) hold for wire under 10 mm regardless of material; life from the Table 10-8 cyclic allowables."
                .into(),
            "Check stock sizes, tolerances and set removal with your spring maker.".into(),
        ],
    })
}

// ------------------------------------------------------------------ extension

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ExtensionIn {
    pub material: String,
    pub d_mm: f64,
    pub mean_d_mm: f64,
    pub body_coils: f64,
    pub initial_tension_n: f64,
    /// Hook bend radius at A (bending), mm — normally D/2.
    pub r1_mm: f64,
    /// Hook side-bend radius at B (torsion), mm.
    pub r2_mm: f64,
    pub f_min_n: f64,
    pub f_max_n: f64,
    pub peened: bool,
}

impl Default for ExtensionIn {
    fn default() -> Self {
        ExtensionIn {
            material: "music_wire".into(),
            d_mm: 1.0,
            mean_d_mm: 7.0,
            body_coils: 14.0,
            initial_tension_n: 4.0,
            r1_mm: 3.5,
            r2_mm: 2.5,
            f_min_n: 6.0,
            f_max_n: 20.0,
            peened: false,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ExtensionOut {
    pub sut_mpa: f64,
    pub index_c: f64,
    pub k_b: f64,
    pub active_coils: f64,
    pub rate_n_mm: f64,
    /// Free length inside the hooks: L0 = (2C − 1 + N_b) d.
    pub free_len_mm: f64,
    pub tau_i_mpa: f64,
    pub tau_i_pref_lo_mpa: f64,
    pub tau_i_pref_hi_mpa: f64,
    pub defl_max_mm: f64,
    pub tau_body_max_mpa: f64,
    pub sigma_hook_a_max_mpa: f64,
    pub tau_hook_b_max_mpa: f64,
    pub ns_body: f64,
    pub ns_hook_bending: f64,
    pub ns_hook_torsion: f64,
    pub nf_body: f64,
    pub nf_hook_bending: f64,
    pub curve_y: Vec<f64>,
    pub curve_f: Vec<f64>,
    pub checks: Vec<Check>,
    pub notes: Vec<String>,
}

pub fn extension(i: ExtensionIn) -> Result<ExtensionOut, String> {
    let (d, dm, nb) = (i.d_mm, i.mean_d_mm, i.body_coils);
    if d.is_nan() || dm.is_nan() || nb.is_nan() || d <= 0.0 || dm <= d || nb <= 0.0 {
        return Err("need 0 < d < D and a positive body-coil count".into());
    }
    let w = wire(&i.material);
    let (e, g) = moduli(w, d);
    let s_ut = sut(w, d);
    let c = dm / d;
    let kb = bergstrasser(c);
    let na = nb + g / e;
    let k = d.powi(4) * g / (8.0 * dm.powi(3) * na);
    let pi = std::f64::consts::PI;
    let tau_per_n = kb * 8.0 * dm / (pi * d.powi(3));
    let tau_i = tau_per_n * i.initial_tension_n;
    let mid = 231.0 * (-0.105 * c).exp();
    let band = 6.9 * (4.0 - (c - 3.0) / 6.5);
    let fi = i.initial_tension_n.max(0.0);
    let (fmin, fmax) = (i.f_min_n.min(i.f_max_n).max(fi), i.f_max_n.max(i.f_min_n).max(fi));
    let c1 = 2.0 * i.r1_mm / d;
    let c2 = 2.0 * i.r2_mm / d;
    let ka = (4.0 * c1 * c1 - c1 - 1.0) / (4.0 * c1 * (c1 - 1.0)).max(1e-9);
    let kbh = (4.0 * c2 - 1.0) / (4.0 * c2 - 4.0).max(1e-9);
    let sig_a = |f: f64| f * (ka * 16.0 * dm / (pi * d.powi(3)) + 4.0 / (pi * d * d));
    let tau_b = |f: f64| kbh * 8.0 * f * dm / (pi * d.powi(3));
    // Static allowables (guidance tabulated by Shigley for A228/302; scaled for stainless).
    let scale = if w.id == "stainless_302" { 0.35 / 0.45 } else { 1.0 };
    let body_allow = w.ssy_frac * s_ut;
    let hook_t_allow = 0.40 * scale * s_ut;
    let hook_b_allow = 0.75 * scale * s_ut;
    let ssu = 0.67 * s_ut;
    let sse = goodman_sse(i.peened, ssu);
    let (tmin, tmax) = (tau_per_n * fmin, tau_per_n * fmax);
    let nf_body = goodman_nf(0.5 * (tmax - tmin), 0.5 * (tmax + tmin), sse, ssu);
    // Hook bending fatigue: Goodman with the bending endurance from Zimmerli
    // converted by the distortion-energy ratio (S_a = S_sa/0.577), ultimate S_ut.
    let (sa0, sm0) = zimmerli(i.peened);
    let se_b = (sa0 / 0.577) / (1.0 - (sm0 / 0.577) / s_ut).max(1e-6);
    let (smin, smax) = (sig_a(fmin), sig_a(fmax));
    let nf_hook = goodman_nf(0.5 * (smax - smin), 0.5 * (smax + smin), se_b, s_ut);
    let l0 = (2.0 * c - 1.0 + nb) * d;
    let ymax = (fmax - fi) / k;
    let n = 24;
    let ytop = ymax * 1.3;
    let curve_y: Vec<f64> = (0..=n).map(|q| ytop * q as f64 / n as f64).collect();
    let curve_f = curve_y.iter().map(|y| fi + k * y).collect();
    let checks = vec![
        check("Spring index 4 ≤ C ≤ 12", (4.0..=12.0).contains(&c), format!("C = {c:.2}")),
        check(
            "Initial tension in the preferred range",
            tau_i >= mid - band && tau_i <= mid + band,
            format!("τ_i = {tau_i:.0} MPa, preferred {:.0}–{:.0} MPa", mid - band, mid + band),
        ),
        check(
            "Body n_s ≥ 1.2",
            body_allow / tmax.max(1e-9) >= 1.2,
            format!("τ = {tmax:.0} MPa vs {body_allow:.0} MPa"),
        ),
        check(
            "Hook bending n ≥ 1.2",
            hook_b_allow / smax.max(1e-9) >= 1.2,
            format!("σ_A = {smax:.0} MPa vs ≈ {hook_b_allow:.0} MPa"),
        ),
        check(
            "Hook torsion n ≥ 1.2",
            hook_t_allow / tau_b(fmax).max(1e-9) >= 1.2,
            format!("τ_B = {:.0} MPa vs ≈ {hook_t_allow:.0} MPa", tau_b(fmax)),
        ),
        check(
            "Hook fatigue n_f ≥ 1.5",
            nf_hook >= 1.5,
            format!("n_f = {nf_hook:.2} (hooks usually fail first)"),
        ),
    ];
    Ok(ExtensionOut {
        sut_mpa: s_ut,
        index_c: c,
        k_b: kb,
        active_coils: na,
        rate_n_mm: k,
        free_len_mm: l0,
        tau_i_mpa: tau_i,
        tau_i_pref_lo_mpa: mid - band,
        tau_i_pref_hi_mpa: mid + band,
        defl_max_mm: ymax,
        tau_body_max_mpa: tmax,
        sigma_hook_a_max_mpa: smax,
        tau_hook_b_max_mpa: tau_b(fmax),
        ns_body: body_allow / tmax.max(1e-9),
        ns_hook_bending: hook_b_allow / smax.max(1e-9),
        ns_hook_torsion: hook_t_allow / tau_b(fmax).max(1e-9),
        nf_body: nf_body.min(1e6),
        nf_hook_bending: nf_hook.min(1e6),
        curve_y,
        curve_f,
        checks,
        notes: vec![
            "N_a = N_b + G/E accounts for the hooks' flexibility; F = F_i + k y once the initial tension is overcome.".into(),
            "Hook allowables (≈ 0.75 S_ut bending, 0.40 S_ut torsion for A228; scaled by 0.35/0.45 for stainless) are typical guidance — check your spring maker's data."
                .into(),
            "Hook bending fatigue uses Zimmerli's torsional data converted to bending by the distortion-energy ratio 0.577 — an approximation.".into(),
        ],
    })
}

// ------------------------------------------------------------------ torsion

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct TorsionIn {
    pub material: String,
    pub d_mm: f64,
    pub mean_d_mm: f64,
    pub body_turns: f64,
    pub leg1_mm: f64,
    pub leg2_mm: f64,
    /// Working moments, N·mm.
    pub m_min_nmm: f64,
    pub m_max_nmm: f64,
    pub arbor_d_mm: f64,
    pub peened: bool,
}

impl Default for TorsionIn {
    fn default() -> Self {
        TorsionIn {
            material: "music_wire".into(),
            d_mm: 1.0,
            mean_d_mm: 8.0,
            body_turns: 6.25,
            leg1_mm: 20.0,
            leg2_mm: 20.0,
            m_min_nmm: 50.0,
            m_max_nmm: 250.0,
            arbor_d_mm: 5.0,
            peened: false,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TorsionOut {
    pub sut_mpa: f64,
    pub sy_mpa: f64,
    pub index_c: f64,
    pub k_i: f64,
    pub active_turns: f64,
    /// N·mm per turn.
    pub rate_nmm_per_turn: f64,
    pub rate_nmm_per_rad: f64,
    pub defl_max_deg: f64,
    pub sigma_max_mpa: f64,
    pub sigma_min_mpa: f64,
    pub ns_static: f64,
    pub nf_goodman: f64,
    /// Mean diameter wound down at M_max, mm.
    pub mean_d_loaded_mm: f64,
    pub id_loaded_mm: f64,
    pub arbor_clearance_mm: f64,
    pub curve_deg: Vec<f64>,
    pub curve_m: Vec<f64>,
    pub checks: Vec<Check>,
    pub notes: Vec<String>,
}

pub fn torsion(i: TorsionIn) -> Result<TorsionOut, String> {
    let (d, dm, nb) = (i.d_mm, i.mean_d_mm, i.body_turns);
    if d.is_nan() || dm.is_nan() || nb.is_nan() || d <= 0.0 || dm <= d || nb <= 0.0 {
        return Err("need 0 < d < D and positive body turns".into());
    }
    let pi = std::f64::consts::PI;
    let w = wire(&i.material);
    let (e, _) = moduli(w, d);
    let s_ut = sut(w, d);
    let sy = w.sy_bend_frac * s_ut;
    let c = dm / d;
    let ki = (4.0 * c * c - c - 1.0) / (4.0 * c * (c - 1.0));
    let na = nb + (i.leg1_mm + i.leg2_mm) / (3.0 * pi * dm);
    let kturn = d.powi(4) * e / (10.8 * dm * na);
    let (mmin, mmax) = (i.m_min_nmm.min(i.m_max_nmm), i.m_max_nmm.max(i.m_min_nmm));
    let sig = |m: f64| ki * 32.0 * m / (pi * d.powi(3));
    let (smin, smax) = (sig(mmin), sig(mmax));
    let turns = mmax / kturn;
    // Fatigue: Associated Spring R = 0 bending allowable (≈ 0.50 S_ut at 10⁶
    // unpeened, 0.60 peened, as tabulated by Shigley Table 10-10), turned into
    // an endurance amplitude on the Goodman line.
    let sr = if i.peened { 0.60 } else { 0.50 } * s_ut;
    let se = (sr / 2.0) / (1.0 - (sr / 2.0) / s_ut);
    let nf = goodman_nf(0.5 * (smax - smin), 0.5 * (smax + smin), se, s_ut);
    let dp = dm * nb / (nb + turns);
    let idp = dp - d;
    let clearance = idp - i.arbor_d_mm;
    let n = 24;
    let top = turns * 360.0 * 1.2;
    let curve_deg: Vec<f64> = (0..=n).map(|q| top * q as f64 / n as f64).collect();
    let curve_m = curve_deg.iter().map(|t| kturn * t / 360.0).collect();
    let checks = vec![
        check("Spring index 4 ≤ C ≤ 12", (4.0..=12.0).contains(&c), format!("C = {c:.2}")),
        check(
            "Static n ≥ 1.2",
            sy / smax.max(1e-9) >= 1.2,
            format!("σ = {smax:.0} MPa vs S_y = {sy:.0} MPa"),
        ),
        check("Fatigue n_f ≥ 1.5 (10⁶ cycles)", nf >= 1.5, format!("n_f = {nf:.2}")),
        check(
            "Clears the arbor when wound up (≥ 10 % of D)",
            clearance >= 0.1 * dm,
            format!("ID {idp:.2} mm vs arbor {:.2} mm", i.arbor_d_mm),
        ),
    ];
    Ok(TorsionOut {
        sut_mpa: s_ut,
        sy_mpa: sy,
        index_c: c,
        k_i: ki,
        active_turns: na,
        rate_nmm_per_turn: kturn,
        rate_nmm_per_rad: kturn / (2.0 * pi),
        defl_max_deg: turns * 360.0,
        sigma_max_mpa: smax,
        sigma_min_mpa: smin,
        ns_static: sy / smax.max(1e-9),
        nf_goodman: nf.min(1e6),
        mean_d_loaded_mm: dp,
        id_loaded_mm: idp,
        arbor_clearance_mm: clearance,
        curve_deg,
        curve_m,
        checks,
        notes: vec![
            "k′ = d⁴E/(10.8 D N_a): the 10.8 (instead of 64/2π·… = 10.2) is Shigley's empirical allowance for coil friction.".into(),
            "Load a torsion spring so it winds up (closes); the inner-fibre stress with K_i governs.".into(),
            "Fatigue uses the ~0.50 S_ut (unpeened, 10⁶ cycles, R = 0) allowable on a Goodman line — typical guidance; check your spring maker.".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shigley Example 10-1: No. 16 music wire (d = 0.037 in), OD 7/16 in,
    /// squared and ground, 12.5 total coils → S_ut ≈ 324 kpsi, S_sy ≈ 146 kpsi,
    /// K_B = 1.124 and the force to close it solid ≈ 6.46 lbf.
    #[test]
    fn shigley_example_10_1() {
        let d = 0.037 * 25.4;
        let dm = (7.0 / 16.0 - 0.037) * 25.4;
        let w = wire("music_wire");
        let s = sut(w, d);
        let kpsi = 6.894_757;
        assert!((s / kpsi - 324.0).abs() < 2.5, "Sut = {} kpsi", s / kpsi);
        let ssy = 0.45 * s;
        assert!((ssy / kpsi - 146.0).abs() < 1.5);
        let c = dm / d;
        assert!((bergstrasser(c) - 1.124).abs() < 1e-3);
        // Force that stresses the coil to S_sy: F = π d³ S_sy / (8 K_B D)
        let f = std::f64::consts::PI * d.powi(3) * ssy / (8.0 * bergstrasser(c) * dm);
        assert!((f / 4.448_222 - 6.46).abs() < 0.08, "F = {} lbf", f / 4.448_222);
        // The engine reproduces the rate formula by hand: k = d⁴G/(8D³Na).
        let o = compression(CompressionIn {
            material: "music_wire".into(),
            d_mm: d,
            mean_d_mm: dm,
            active_coils: 10.5,
            ends: "squared_ground".into(),
            free_len_mm: 50.0,
            ..Default::default()
        })
        .unwrap();
        let hand = d.powi(4) * 81_700.0 / (8.0 * dm.powi(3) * 10.5);
        assert!((o.rate_n_mm - hand).abs() / hand < 1e-9);
        assert!((o.solid_len_mm - d * 12.5).abs() < 1e-9);
        assert!((o.total_coils - 12.5).abs() < 1e-12);
    }

    #[test]
    fn wahl_and_bergstrasser_agree() {
        for c in [4.0, 6.0, 8.0, 12.0] {
            let rel = (wahl(c) - bergstrasser(c)).abs() / wahl(c);
            assert!(rel < 0.015, "C = {c}: {rel}");
        }
    }

    #[test]
    fn goodman_zimmerli_and_life() {
        // Pure alternating at the Zimmerli S_se → n_f = 1.
        let ssu = 0.67 * 2000.0;
        let sse = goodman_sse(false, ssu);
        assert!((goodman_nf(sse, 0.0, sse, ssu) - 1.0).abs() < 1e-12);
        // Zero-to-peak exactly at the 10⁶ allowable → 10⁶ cycles.
        let s0 = 0.33 * 2000.0;
        let l = life(s0 / 2.0, s0 / 2.0, 2000.0, false);
        assert!((l.cycles.log10() - 6.0).abs() < 1e-9, "{:?}", l);
        assert!(life(10.0, 20.0, 2000.0, false).runout);
    }

    #[test]
    fn buckling_and_torsion_extension_run() {
        let o = compression(CompressionIn {
            free_len_mm: 120.0,
            support: "pivoted".into(),
            ..Default::default()
        })
        .unwrap();
        assert!(!o.buckling_stable_absolute);
        assert!(o.buckling_defl_mm.is_finite() && o.buckling_defl_mm > 0.0);
        let t = torsion(TorsionIn::default()).unwrap();
        assert!(t.rate_nmm_per_turn > 0.0 && t.sigma_max_mpa > t.sigma_min_mpa);
        // hand: k' = d⁴E/(10.8 D Na)
        let na = 6.25 + 40.0 / (3.0 * std::f64::consts::PI * 8.0);
        assert!((t.rate_nmm_per_turn - 200_000.0 / (10.8 * 8.0 * na)).abs() < 1e-6);
        let e = extension(ExtensionIn::default()).unwrap();
        assert!(e.free_len_mm > 0.0 && e.sigma_hook_a_max_mpa > e.tau_body_max_mpa);
    }
}
