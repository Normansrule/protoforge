//! Shared inputs of the Human Factors Lab: the per-project settings stored in
//! `design.extra.ergonomics` and the Iris frame geometry the analyses read
//! from the CAD generator (`crate::cad`, read-only).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::project::Project;

/// Lab settings, read from `design.extra.ergonomics` (every field optional).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ErgoConfig {
    /// Share of women in the target population (0..1). ANSUR II itself is
    /// about two-thirds men; consumer products usually want 50/50.
    pub female_fraction: f64,
    /// Outward splay of each temple at the hinge stop, degrees (0 = the
    /// temples run straight back, as the Iris CAD models them).
    pub temple_splay_deg: f64,
    /// Display axis shifted towards the nose from the right lens-box centre, mm.
    pub display_offset_mm: f64,
    /// Transmission of an optional sunglass tint in front of the combiner (1 = clear).
    pub tint_transmission: f64,
    /// Cap height of the caption font on the micro-display, pixels.
    pub caption_cap_px: f64,
    /// Wearer's visual acuity as the Snellen denominator (20 = 20/20).
    pub acuity_20_over: f64,
    /// Largest acceptable decentration per eye, |frame PD − IPD| / 2, mm.
    pub pd_tolerance_mm: f64,
    /// Accepted range of (temple bend − ear distance), mm: [bend may sit this far in front, this far behind].
    pub ear_window_mm: [f64; 2],
    /// Also require the bridge to suit the nose (DBL − nose bridge width inside `nose_window_mm`).
    pub nose_check: bool,
    pub nose_window_mm: [f64; 2],
    /// Static friction coefficient between skin and the frame (dry skin on plastic is low: ~0.2–0.5).
    pub friction_mu: f64,
    /// Head flexion used for the "too loose" check (looking down at a phone), degrees.
    pub look_down_deg: f64,
    /// Angle of each nose flank from the vertical mid-plane at the pads, degrees.
    pub nose_flank_deg: f64,
    /// Contact area of one nose pad, mm² (0 = take it from the CAD pad).
    pub pad_area_mm2: f64,
    /// Length of temple in contact with the side of the head, mm.
    pub temple_contact_length_mm: f64,
    /// Sustained skin-pressure comfort limit, kPa (≈ 32 mmHg capillary pressure = 4.3 kPa).
    pub pressure_limit_kpa: f64,
    /// Where the temple presses on the head: this far in front of the ear bend, mm.
    pub contact_before_bend_mm: f64,
    /// Background luminance of the bright scene used for `ergonomics.outdoor_contrast`, cd/m².
    pub bright_scene_nits: f64,
    /// Standard deviation of left/right monocular PD asymmetry, mm (0 = symmetric faces).
    pub asymmetry_sd_mm: f64,
    /// Monte Carlo population size and seed.
    pub samples: usize,
    pub seed: u64,
    /// Replace a dimension's published statistics: id → {men: [mean, sd], women: [mean, sd]}.
    pub overrides: BTreeMap<String, DimOverride>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DimOverride {
    pub men: Option<[f64; 2]>,
    pub women: Option<[f64; 2]>,
}

impl Default for ErgoConfig {
    fn default() -> Self {
        ErgoConfig {
            female_fraction: 0.5,
            temple_splay_deg: 0.0,
            display_offset_mm: 0.0,
            tint_transmission: 1.0,
            caption_cap_px: 10.0,
            acuity_20_over: 20.0,
            pd_tolerance_mm: 4.0,
            ear_window_mm: [-2.0, 12.0],
            nose_check: false,
            nose_window_mm: [-3.0, 3.0],
            friction_mu: 0.3,
            look_down_deg: 40.0,
            nose_flank_deg: 30.0,
            pad_area_mm2: 0.0,
            temple_contact_length_mm: 40.0,
            pressure_limit_kpa: 4.3,
            contact_before_bend_mm: 10.0,
            bright_scene_nits: 5000.0,
            asymmetry_sd_mm: 0.0,
            samples: 4000,
            seed: 7,
            overrides: BTreeMap::new(),
        }
    }
}

impl ErgoConfig {
    /// Read `design.extra.ergonomics`, falling back to defaults field by field.
    pub fn from_project(p: &Project) -> Self {
        p.design
            .extra
            .get("ergonomics")
            .and_then(|v| serde_json::from_value::<ErgoConfig>(v.clone()).ok())
            .unwrap_or_default()
            .sanitized()
    }

    /// Clamp every field to a physically meaningful range.
    pub fn sanitized(mut self) -> Self {
        let fin = |v: f64, d: f64| if v.is_finite() { v } else { d };
        let d = ErgoConfig::default();
        self.female_fraction = fin(self.female_fraction, 0.5).clamp(0.0, 1.0);
        self.temple_splay_deg = fin(self.temple_splay_deg, 0.0).clamp(-10.0, 25.0);
        self.display_offset_mm = fin(self.display_offset_mm, 0.0).clamp(-15.0, 15.0);
        self.tint_transmission = fin(self.tint_transmission, 1.0).clamp(0.02, 1.0);
        self.caption_cap_px = fin(self.caption_cap_px, d.caption_cap_px).clamp(1.0, 400.0);
        self.acuity_20_over = fin(self.acuity_20_over, 20.0).clamp(5.0, 400.0);
        self.pd_tolerance_mm = fin(self.pd_tolerance_mm, d.pd_tolerance_mm).clamp(0.1, 30.0);
        self.friction_mu = fin(self.friction_mu, d.friction_mu).clamp(0.01, 3.0);
        self.look_down_deg = fin(self.look_down_deg, d.look_down_deg).clamp(0.0, 89.0);
        self.nose_flank_deg = fin(self.nose_flank_deg, d.nose_flank_deg).clamp(5.0, 85.0);
        self.pad_area_mm2 = fin(self.pad_area_mm2, 0.0).clamp(0.0, 2000.0);
        self.temple_contact_length_mm = fin(self.temple_contact_length_mm, d.temple_contact_length_mm).clamp(1.0, 200.0);
        self.pressure_limit_kpa = fin(self.pressure_limit_kpa, d.pressure_limit_kpa).clamp(0.1, 1000.0);
        self.contact_before_bend_mm = fin(self.contact_before_bend_mm, d.contact_before_bend_mm).clamp(0.0, 80.0);
        self.bright_scene_nits = fin(self.bright_scene_nits, d.bright_scene_nits).clamp(1.0, 1e6);
        self.asymmetry_sd_mm = fin(self.asymmetry_sd_mm, 0.0).clamp(0.0, 10.0);
        self.samples = self.samples.clamp(200, 50_000);
        for w in [&mut self.ear_window_mm, &mut self.nose_window_mm] {
            if !(w[0].is_finite() && w[1].is_finite()) || w[0] > w[1] {
                *w = [w[0].min(w[1]), w[0].max(w[1])];
                if !(w[0].is_finite() && w[1].is_finite()) {
                    *w = [-2.0, 12.0];
                }
            }
        }
        self
    }
}

/// Everything the lab needs to know about the frame, in the CAD assembly
/// frame (mm): X → wearer's right, Y → forward (temples run towards −Y), Z → up,
/// origin at the bridge centre on the front face.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FrameGeom {
    pub lens_width_mm: f64,
    pub bridge_mm: f64,
    pub temple_length_mm: f64,
    pub rim_mm: f64,
    pub front_thickness_mm: f64,
    pub lens_height_mm: f64,
    pub lens_center_x: f64,
    pub frame_pd_mm: f64,
    pub total_width_mm: f64,
    /// Inner face of the temples (both sides, |x|).
    pub temple_inner_x: f64,
    pub pin_y: f64,
    pub hinge_z: f64,
    pub pod_front_y: f64,
    pub pod_rear_y: f64,
    pub battery_pod_rear_y: f64,
    pub ear_bend_y: f64,
    /// Hinge pin to ear bend along the temple, mm.
    pub bend_from_pin_mm: f64,
    pub temple_w_mm: f64,
    pub temple_h_mm: f64,
    pub pod_w_mm: f64,
    pub pod_h_mm: f64,
    pub wall_mm: f64,
    /// Nose-pad centre (right pad; the left is mirrored).
    pub pad: [f64; 3],
    /// Semi-axes of the moulded pad ellipse, mm.
    pub pad_semi_mm: [f64; 2],
    pub pad_area_cad_mm2: f64,
    pub material: String,
    pub e_mpa: f64,
}

/// Length of the temple behind the ear bend, as the CAD generator builds it.
pub fn ear_tail_mm(temple_len: f64) -> f64 {
    (0.25 * temple_len).clamp(25.0, 45.0)
}

/// Read the frame geometry from the CAD generator (cached there).
pub fn frame_geom(p: &Project) -> FrameGeom {
    let f = &p.design.frame;
    let g = crate::cad::frame_geom(p);
    let l = &g.layout;
    let a = f.lens_width_mm.clamp(30.0, 70.0);
    let b = f.lens_height_mm.clamp(20.0, 60.0) / 2.0;
    let n = f.lens_shape_exponent.clamp(2.0, 8.0);
    let rim = f.rim_width_mm.clamp(1.5, 12.0);
    let t = f.front_thickness_mm.clamp(2.0, 12.0);
    // Nose pads: mirrors cad::frame (pads in the middle of the rim band at
    // v = −0.3·b, ellipse semi-axes min(rim/2 − 0.6, 1.8) × 3.5, 3 mm proud).
    let c = l.lens_center_x;
    let vp = -0.3 * b;
    let lens_u = (a / 2.0) * (1.0 - (vp.abs() / b).powf(n)).max(0.0).powf(1.0 / n);
    let u_pad = c - lens_u - rim / 2.0;
    let (ra, rb) = ((rim / 2.0 - 0.6).clamp(0.3, 1.8), 3.5);
    let has_pads = rim >= 3.0 && u_pad - ra > 0.5;
    let pad = [u_pad.max(3.0), -(t + 1.5), vp];
    let pad_area = if has_pads { std::f64::consts::PI * ra * rb } else { 0.0 };
    let mat = crate::structures::materials::get(&f.material);
    let batt_len = f.battery_pod_length_mm.clamp(15.0, 90.0);
    FrameGeom {
        lens_width_mm: a,
        bridge_mm: f.bridge_mm.clamp(10.0, 28.0),
        temple_length_mm: f.temple_length_mm.clamp(110.0, 170.0),
        rim_mm: rim,
        front_thickness_mm: t,
        lens_height_mm: 2.0 * b,
        lens_center_x: c,
        frame_pd_mm: l.frame_pd_mm,
        total_width_mm: l.total_width_mm,
        temple_inner_x: l.temple_inner_x,
        pin_y: l.pin_y,
        hinge_z: l.hinge_z,
        pod_front_y: l.pod_front_y,
        pod_rear_y: l.pod_rear_y,
        battery_pod_rear_y: l.pod_front_y - batt_len,
        ear_bend_y: l.ear_bend_y,
        bend_from_pin_mm: l.pin_y - l.ear_bend_y,
        temple_w_mm: f.temple_width_mm.clamp(2.5, 8.0),
        temple_h_mm: f.temple_height_mm.clamp(4.0, 16.0),
        pod_w_mm: f.pod_width_mm.max(f.temple_width_mm),
        pod_h_mm: f.pod_height_mm.max(f.temple_height_mm),
        wall_mm: f.wall_mm.clamp(0.6, 3.0),
        pad,
        pad_semi_mm: [ra, rb],
        pad_area_cad_mm2: pad_area,
        material: mat.name.to_string(),
        e_mpa: mat.e_mpa(),
    }
}

/// A frame size in the boxing system `A □ DBL – temple`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct FrameSize {
    pub lens_width_mm: f64,
    pub bridge_mm: f64,
    pub temple_length_mm: f64,
    /// Temple splay for this size, degrees (None = the lab setting).
    #[serde(default)]
    pub splay_deg: Option<f64>,
}

impl FrameSize {
    pub fn label(&self) -> String {
        let base = format!("{:.0} □ {:.0} – {:.0}", self.lens_width_mm, self.bridge_mm, self.temple_length_mm);
        match self.splay_deg {
            Some(s) => format!("{base} · {s:.0}° splay"),
            None => base,
        }
    }
}

impl FrameGeom {
    pub fn size(&self) -> FrameSize {
        FrameSize {
            lens_width_mm: self.lens_width_mm,
            bridge_mm: self.bridge_mm,
            temple_length_mm: self.temple_length_mm,
            splay_deg: None,
        }
    }

    /// The same frame re-sized analytically (front grows symmetrically, the
    /// temples keep their construction): used to search S/M/L sizes without
    /// regenerating CAD.
    pub fn resized(&self, s: &FrameSize) -> FrameGeom {
        let mut g = self.clone();
        let da = s.lens_width_mm - self.lens_width_mm;
        let dd = s.bridge_mm - self.bridge_mm;
        g.lens_width_mm = s.lens_width_mm;
        g.bridge_mm = s.bridge_mm;
        g.temple_length_mm = s.temple_length_mm;
        g.lens_center_x += da / 2.0 + dd / 2.0;
        g.frame_pd_mm = s.lens_width_mm + s.bridge_mm;
        g.total_width_mm += 2.0 * da + dd;
        g.temple_inner_x += da + dd / 2.0;
        g.pad[0] += dd / 2.0;
        let bend = s.temple_length_mm - ear_tail_mm(s.temple_length_mm);
        let bend0 = self.temple_length_mm - ear_tail_mm(self.temple_length_mm);
        g.bend_from_pin_mm = self.bend_from_pin_mm + (bend - bend0);
        g.ear_bend_y = g.pin_y - g.bend_from_pin_mm;
        g
    }
}
