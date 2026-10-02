//! Temple clamp force: how hard the glasses squeeze the head.
//!
//! When the frame goes on, each temple is pushed outwards at its contact
//! point (a distance `L_c` behind the hinge pin) until the spread between the
//! contacts equals the head width `H`. With the hinge closed against its stop,
//! the outward force `F` is carried by two elastic paths in series:
//!
//! 1. **The temple** — an Euler–Bernoulli cantilever built in at the hinge,
//!    stepped in section: the hollow electronics / battery pod (a box
//!    `w × h` with wall `t`: `I = (h w³ − (h−2t)(w−2t)³)/12`) and then the
//!    solid arm (`I = h_t w_t³/12`, bending about the vertical axis). The
//!    tip compliance is the unit-load (Castigliano) integral
//!    `c = ∫₀^{L_c} (L_c − s)² / (E I(s)) ds`, which is `L³/(3EI)` for a
//!    uniform cantilever.
//! 2. **The front** — the moment `M = F L_c` at each hinge bends the front
//!    like a beam under equal end moments: relative hinge rotation
//!    `θ = M Σ ℓ_i/(E I_i)` over the rim bars (two per lens) and the bridge
//!    bar. It turns each contact outwards by `θ L_c / 2` per side.
//!
//! Equilibrium makes the two side forces equal, so
//!
//! ```text
//! F = max(0, H − S₀) / (L_c² Σ ℓ/EI + c_right + c_left),   S₀ = 2 x_in + 2 L_c tan(splay)
//! ```
//!
//! where `S₀` is the unloaded spread at the contacts. A stiffer side deflects
//! less, so an asymmetric frame also yaws slightly on the head.
//!
//! The comfortable band comes from the balance model: at least the force that
//! stops the glasses sliding when looking down (friction), at most the force
//! that keeps the temple's contact pressure under the ≈ 4.3 kPa sustained-
//! pressure limit. Both ends are **typical, assumption-driven** values —
//! confirm with wear tests.
//!
//! **Spring hinge.** When the product has a spring hinge (Iris's HW1), the
//! hinge stays shut against its stop only until the moment `F L_c` reaches
//! the spring's over-travel torque `T(0)`; past that the temple rotates by
//! an over-travel angle α with `F L_c = T(α)` — the torque curve comes from
//! the Mechanisms Lab ([`crate::mechanisms::hinge`]). Each contact then moves
//! out by `L_c sin α` on top of the elastic deflection, so
//!
//! ```text
//! H − S₀ = F c_total + 2 L_c sin α(F),   α(F) = 0 while F L_c ≤ T(0)
//! ```
//!
//! and the hinge acts as a clamp-force limiter: a wide head cannot squeeze
//! much harder than `T(0)/L_c` until the over-travel stop is reached.
//!
//! Head breadth (the maximum breadth, above the ears) stands in for the width
//! the temples actually span, which is a little smaller: forces here are
//! upper bounds. Lenses bonded into the rims and hinge play are not
//! modelled.
//!
//! References: Gere & Goodno, *Mechanics of Materials* (cantilever and
//! unit-load deflection); Young & Budynas, *Roark's Formulas for Stress and
//! Strain* (7th ed., 2002).

use serde::Serialize;

use super::model::FrameGeom;

/// One beam segment: from `s0` to `s1` (mm from the hinge pin), bending stiffness `ei` (N·mm²).
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct Segment {
    pub name: &'static str,
    pub s0: f64,
    pub s1: f64,
    pub i_mm4: f64,
    pub ei: f64,
}

/// Compliance (mm/N) at distance `l` from the root of a stepped cantilever,
/// load applied at `l`: ∫₀^l (l − s)² / EI(s) ds, exact per segment.
pub fn cantilever_compliance(segs: &[Segment], l: f64) -> f64 {
    segs.iter()
        .map(|s| {
            let a = s.s0.max(0.0).min(l);
            let b = s.s1.min(l);
            if b <= a {
                0.0
            } else {
                ((l - a).powi(3) - (l - b).powi(3)) / (3.0 * s.ei)
            }
        })
        .sum()
}

/// Second moment of a rectangle about the axis parallel to `h` (bending across `w`).
pub fn rect_i(h: f64, w: f64) -> f64 {
    h * w.powi(3) / 12.0
}

/// Hollow rectangular box, bending across `w`.
pub fn box_i(h: f64, w: f64, t: f64) -> f64 {
    let hi = (h - 2.0 * t).max(0.0);
    let wi = (w - 2.0 * t).max(0.0);
    (h * w.powi(3) - hi * wi.powi(3)) / 12.0
}

/// Over-travel torque curve of a spring hinge: torque (N·mm) against the
/// angle past the worn position (degrees), up to the over-travel stop.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct SpringHinge {
    pub alpha_deg: Vec<f64>,
    pub torque_nmm: Vec<f64>,
    pub stop_deg: f64,
    /// Torque at which the hinge starts to open past the worn position, N·mm.
    pub crack_torque_nmm: f64,
}

impl SpringHinge {
    fn torque(&self, alpha: f64) -> f64 {
        crate::math::interp(&self.alpha_deg, &self.torque_nmm, alpha).max(0.0)
    }

    /// Over-travel angle (degrees) at which the hinge carries torque `t`; `stop_deg` if it never does.
    fn angle_for(&self, t: f64) -> f64 {
        if t <= self.crack_torque_nmm {
            return 0.0;
        }
        if self.torque(self.stop_deg) < t {
            return self.stop_deg;
        }
        let (mut lo, mut hi) = (0.0, self.stop_deg);
        for _ in 0..60 {
            let mid = 0.5 * (lo + hi);
            if self.torque(mid) < t {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ClampModel {
    pub e_mpa: f64,
    /// Contact distance behind the hinge pin, mm.
    pub l_c: f64,
    /// Unloaded spread between the contacts, mm.
    pub s0: f64,
    pub splay_deg: f64,
    pub right: Vec<Segment>,
    pub left: Vec<Segment>,
    pub front: Vec<Segment>,
    /// Σ ℓ/(EI) of the front, rad/(N·mm).
    pub front_flex: f64,
    pub c_right: f64,
    pub c_left: f64,
    pub c_front: f64,
    pub c_total: f64,
    /// Spring hinge in series with the frame (None = hinge locked at its stop).
    pub hinge: Option<SpringHinge>,
    /// Clamp force at which the spring hinge starts to open, N (None without one).
    pub hinge_crack_n: Option<f64>,
    #[serde(skip)]
    hinge_table: Vec<[f64; 2]>,
}

impl ClampModel {
    pub fn new(fg: &FrameGeom, splay_deg: f64, contact_before_bend: f64) -> Self {
        let e = fg.e_mpa;
        let l_c = (fg.bend_from_pin_mm - contact_before_bend).max(10.0);
        // Pods start at the knuckle end; the arm continues to the contact.
        let pod_start = (fg.pin_y - fg.pod_front_y).max(0.0);
        let pod_end_r = fg.pin_y - fg.pod_rear_y;
        let pod_end_l = fg.pin_y - fg.battery_pod_rear_y;
        let i_arm = rect_i(fg.temple_h_mm, fg.temple_w_mm);
        let i_pod = box_i(fg.pod_h_mm, fg.pod_w_mm, fg.wall_mm);
        let temple = |pod_end: f64, pod: &'static str| {
            vec![
                Segment {
                    name: "knuckle",
                    s0: 0.0,
                    s1: pod_start,
                    i_mm4: i_arm,
                    ei: e * i_arm,
                },
                Segment {
                    name: pod,
                    s0: pod_start,
                    s1: pod_end,
                    i_mm4: i_pod,
                    ei: e * i_pod,
                },
                Segment {
                    name: "arm",
                    s0: pod_end,
                    s1: 1e9,
                    i_mm4: i_arm,
                    ei: e * i_arm,
                },
            ]
        };
        let right = temple(pod_end_r, "electronics pod");
        let left = temple(pod_end_l, "battery pod");
        // Front: rim bars top + bottom over each lens (+ rim), one bridge bar.
        let t = fg.front_thickness_mm;
        let i_rims = 2.0 * rect_i(fg.rim_mm, t);
        let t_b = (0.9 * fg.rim_mm).max(3.5);
        let i_bridge = rect_i(t_b, t);
        let lens_span = fg.lens_width_mm + fg.rim_mm;
        let front = vec![
            Segment {
                name: "left rims",
                s0: 0.0,
                s1: lens_span,
                i_mm4: i_rims,
                ei: e * i_rims,
            },
            Segment {
                name: "bridge",
                s0: lens_span,
                s1: lens_span + fg.bridge_mm,
                i_mm4: i_bridge,
                ei: e * i_bridge,
            },
            Segment {
                name: "right rims",
                s0: lens_span + fg.bridge_mm,
                s1: 2.0 * lens_span + fg.bridge_mm,
                i_mm4: i_rims,
                ei: e * i_rims,
            },
        ];
        let front_flex: f64 = front.iter().map(|s| (s.s1 - s.s0) / s.ei).sum();
        let c_right = cantilever_compliance(&right, l_c);
        let c_left = cantilever_compliance(&left, l_c);
        let c_front = l_c * l_c * front_flex;
        let s0 = 2.0 * fg.temple_inner_x + 2.0 * l_c * splay_deg.to_radians().tan();
        ClampModel {
            e_mpa: e,
            l_c,
            s0,
            splay_deg,
            right,
            left,
            front,
            front_flex,
            c_right,
            c_left,
            c_front,
            c_total: c_right + c_left + c_front,
            hinge: None,
            hinge_crack_n: None,
            hinge_table: Vec::new(),
        }
    }

    /// Put a spring hinge in series with the frame.
    pub fn with_hinge(mut self, hinge: Option<SpringHinge>) -> Self {
        self.hinge_crack_n = hinge.as_ref().map(|h| h.crack_torque_nmm / self.l_c);
        // (total spread, force) at every point of the torque curve: piecewise-linear and monotone.
        self.hinge_table = hinge
            .as_ref()
            .map(|h| {
                h.alpha_deg
                    .iter()
                    .zip(&h.torque_nmm)
                    .map(|(a, t)| {
                        let f = t / self.l_c;
                        [f * self.c_total + 2.0 * self.l_c * a.to_radians().sin(), f]
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.hinge = hinge;
        self
    }

    /// Hinge over-travel angle (degrees) while pressing with force `f`.
    pub fn overtravel_deg(&self, f: f64) -> f64 {
        self.hinge.as_ref().map(|h| h.angle_for(f * self.l_c)).unwrap_or(0.0)
    }

    /// Total spread between the contacts (beyond `S₀`) that produces force `f`, mm.
    pub fn spread_for(&self, f: f64) -> f64 {
        f * self.c_total + 2.0 * self.l_c * self.overtravel_deg(f).to_radians().sin()
    }

    /// Clamp force per side for head width `h`, N.
    pub fn force(&self, h: f64) -> f64 {
        let d = h - self.s0;
        if d <= 0.0 {
            return 0.0;
        }
        let t = &self.hinge_table;
        if t.is_empty() || d <= t[0][0] {
            return d / self.c_total;
        }
        let last = t[t.len() - 1];
        if d >= last[0] {
            // at the over-travel stop the frame is rigid again
            let stop = self.hinge.as_ref().map(|h| h.stop_deg).unwrap_or(0.0);
            return ((d - 2.0 * self.l_c * stop.to_radians().sin()) / self.c_total).max(last[1]);
        }
        let i = t.partition_point(|p| p[0] <= d).clamp(1, t.len() - 1);
        let (a, b) = (t[i - 1], t[i]);
        let w = if b[0] > a[0] { (d - a[0]) / (b[0] - a[0]) } else { 1.0 };
        a[1] + w * (b[1] - a[1])
    }

    /// Outward movement of each contact [right, left], mm (elastic bending plus hinge over-travel).
    pub fn deflections(&self, h: f64) -> [f64; 2] {
        let f = self.force(h);
        let front = f * self.c_front / 2.0;
        let hinge = self.l_c * self.overtravel_deg(f).to_radians().sin();
        [f * self.c_right + front + hinge, f * self.c_left + front + hinge]
    }

    /// Head width that produces force `f`.
    pub fn width_for(&self, f: f64) -> f64 {
        self.s0 + self.spread_for(f)
    }

    /// Splay that puts the comfort band's head-width interval centre at `c`.
    ///
    /// Splay only moves the unloaded spread `S₀`, so the band of head widths
    /// `[width_for(f_lo), width_for(f_hi)]` slides rigidly with it. (Centring
    /// on the force mid-band instead goes wrong with a spring hinge, whose
    /// force is nearly flat over most head widths.)
    pub fn splay_centring(&self, c: f64, f_lo: f64, f_hi: f64, inner_x: f64) -> f64 {
        let s0 = c - 0.5 * (self.spread_for(f_lo) + self.spread_for(f_hi));
        ((s0 / 2.0 - inner_x) / self.l_c).atan().to_degrees()
    }

    /// Bending stress at the hinge root of the arm/pod for force `f`: σ = M c / I at s = 0, MPa.
    pub fn root_stress_mpa(&self, f: f64, fg: &FrameGeom) -> f64 {
        f * self.l_c * (fg.temple_w_mm / 2.0) / rect_i(fg.temple_h_mm, fg.temple_w_mm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_cantilever_matches_l3_over_3ei() {
        // 4 × 8 mm PETG arm (E = 2000 MPa), 100 mm: I = 8·4³/12 = 42.67 mm⁴,
        // c = L³/(3EI) = 1e6 / (3·2000·42.67) = 3.906 mm/N; 1 N → 3.9 mm.
        let i = rect_i(8.0, 4.0);
        assert!((i - 42.666_666).abs() < 1e-3);
        let seg = [Segment {
            name: "arm",
            s0: 0.0,
            s1: 1e9,
            i_mm4: i,
            ei: 2000.0 * i,
        }];
        let c = cantilever_compliance(&seg, 100.0);
        assert!((c - 1e6 / (3.0 * 2000.0 * i)).abs() < 1e-9);
        assert!((c - 3.906_25).abs() < 1e-4);
        // Force for 5 mm tip deflection: F = 3EIδ/L³ = 1.28 N
        assert!((5.0 / c - 1.28).abs() < 1e-3);
    }

    #[test]
    fn stepped_cantilever_sums_segments() {
        // Two halves with the same EI must equal the uniform result.
        let ei = 1000.0;
        let a = [
            Segment {
                name: "a",
                s0: 0.0,
                s1: 40.0,
                i_mm4: 1.0,
                ei,
            },
            Segment {
                name: "b",
                s0: 40.0,
                s1: 1e9,
                i_mm4: 1.0,
                ei,
            },
        ];
        let l: f64 = 80.0;
        assert!((cantilever_compliance(&a, l) - l.powi(3) / (3.0 * ei)).abs() < 1e-9);
        // A rigid root segment leaves only the tip part: (L − a)³/(3EI).
        let b = [
            Segment {
                name: "rigid",
                s0: 0.0,
                s1: 40.0,
                i_mm4: 1.0,
                ei: 1e18,
            },
            Segment {
                name: "b",
                s0: 40.0,
                s1: 1e9,
                i_mm4: 1.0,
                ei,
            },
        ];
        assert!((cantilever_compliance(&b, l) - 40f64.powi(3) / (3.0 * ei)).abs() < 1e-9);
    }

    #[test]
    fn box_section_by_hand() {
        // 14 × 11 box, 1.2 wall: (14·11³ − 11.6·8.6³)/12 = 938.0 mm⁴
        let i = box_i(14.0, 11.0, 1.2);
        assert!((i - (14.0 * 1331.0 - 11.6 * 636.056) / 12.0).abs() < 1e-6, "{i}");
    }

    #[test]
    fn spring_hinge_limits_clamp_force() {
        let fg = crate::ergonomics::model::frame_geom(&crate::project::Project::iris());
        let rigid = ClampModel::new(&fg, 0.0, 10.0);
        // the hinge opens at 50 N·mm and stiffens to 60 N·mm at its 20° stop
        let h = SpringHinge {
            alpha_deg: vec![0.0, 10.0, 20.0],
            torque_nmm: vec![50.0, 55.0, 60.0],
            stop_deg: 20.0,
            crack_torque_nmm: 50.0,
        };
        let sprung = rigid.clone().with_hinge(Some(h));
        let crack = 50.0 / rigid.l_c;
        assert!((sprung.hinge_crack_n.unwrap() - crack).abs() < 1e-12);
        // below the crack force both models agree
        let w_small = rigid.width_for(0.5 * crack);
        assert!((sprung.force(w_small) - rigid.force(w_small)).abs() < 1e-9);
        // a much wider head is held between the crack and stop forces while the hinge opens
        let w_wide = rigid.width_for(crack) + 10.0;
        let f = sprung.force(w_wide);
        assert!(rigid.force(w_wide) > crack * 1.2);
        assert!(f > crack && f < 60.0 / rigid.l_c, "{f}");
        // the spread is shared by bending and hinge rotation: 2 L_c sin α + F c = H − S₀
        let a = sprung.overtravel_deg(f);
        assert!(a > 0.0 && a < 20.0);
        let resid = 2.0 * rigid.l_c * a.to_radians().sin() + f * rigid.c_total - (w_wide - rigid.s0);
        assert!(resid.abs() < 0.05, "{resid}");
        // width_for inverts force; past the stop the frame is rigid again
        for f in [0.1 * crack, 0.9 * crack, 1.05 * crack, 60.0 / rigid.l_c + 0.5] {
            assert!((sprung.force(sprung.width_for(f)) - f).abs() < 1e-3 * f, "{f}");
        }
    }
}
