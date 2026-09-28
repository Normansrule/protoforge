//! Face-fit helper: from a wearer's face width and pupillary distance (PD)
//! recommend a boxing-system size `A □ DBL – temple` for the Iris frame.
//!
//! * Frame PD (distance between box centres) = A + DBL (ISO 8624 boxing).
//! * When frame PD ≠ PD the optical centres must be **decentred** inwards by
//!   (frame PD − PD)/2 per eye. For a prescription lens of power F (dioptres)
//!   an uncorrected offset of c (cm) induces a prism of P = c·F prism
//!   dioptres (Prentice's rule).
//! * For Iris the display is centred on the right lens box, so the pupil
//!   sits (frame PD − PD)/2 from the virtual image axis; it must stay inside
//!   half the eyebox (requirement R-007: eyebox ≥ 5 mm).
//!
//! The recommendation minimises a weighted least-squares mismatch of total
//! frame width vs face width, frame PD vs PD (+2 mm allowance) and bridge
//! vs a typical 18 mm nose, over integer sizes A ∈ [40, 62], DBL ∈ [14, 24].
//!
//! References: ISO 8624:2020 *Ophthalmic optics — Spectacle frames —
//! Measuring system and vocabulary*; C. Fowler, K. Latham Petre,
//! *Spectacle Lenses: Theory and Practice*, Butterworth-Heinemann 2001
//! (decentration and Prentice's rule).

use serde::{Deserialize, Serialize};

use crate::project::FrameParams;

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct FitInput {
    /// Head width at the hinge line (temple to temple), mm.
    pub face_width_mm: f64,
    /// Binocular pupillary distance, mm.
    pub pd_mm: f64,
    /// Optional prescription sphere power for the prism estimate, D.
    pub sphere_d: f64,
    /// Current frame (rim width is kept).
    pub frame: Option<FrameParams>,
    /// Eyebox of the display, mm (defaults to the R-007 target of 5 mm).
    pub eyebox_mm: f64,
}

impl Default for FitInput {
    fn default() -> Self {
        FitInput {
            face_width_mm: 138.0,
            pd_mm: 63.0,
            sphere_d: 0.0,
            frame: None,
            eyebox_mm: 5.0,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct FitPatch {
    pub lens_width_mm: f64,
    pub bridge_mm: f64,
    pub temple_length_mm: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct FitResult {
    pub size_label: String,
    pub lens_width_mm: f64,
    pub bridge_mm: f64,
    pub temple_length_mm: f64,
    pub frame_pd_mm: f64,
    pub total_width_mm: f64,
    pub width_error_mm: f64,
    /// Inward decentration per eye, mm (positive = optical centre inwards).
    pub decentration_per_eye_mm: f64,
    /// Prism induced if the lenses were NOT decentred, prism dioptres per eye.
    pub prentice_prism_d: f64,
    /// Offset between pupil and the display axis (right eye), mm.
    pub pupil_to_display_axis_mm: f64,
    pub within_eyebox: bool,
    pub notes: Vec<String>,
    pub patch: FitPatch,
    /// Current frame for comparison.
    pub current_frame_pd_mm: f64,
    pub current_decentration_mm: f64,
}

/// Total front width of the generated frame for given A, DBL, rim.
pub fn front_width(a: f64, dbl: f64, rim: f64) -> f64 {
    2.0 * a + dbl + 2.0 * rim + 6.0
}

pub fn recommend(i: &FitInput) -> FitResult {
    let cur = i.frame.clone().unwrap_or_default();
    let rim = cur.rim_width_mm.clamp(1.5, 12.0);
    let pd = i.pd_mm.clamp(45.0, 80.0);
    let w = i.face_width_mm.clamp(100.0, 180.0);
    let mut best = (f64::INFINITY, 50.0, 18.0);
    for a in 40..=62 {
        for dbl in 14..=24 {
            let (a, dbl) = (a as f64, dbl as f64);
            let ew = (front_width(a, dbl, rim) - w) / 4.0;
            let ep = ((a + dbl) - (pd + 2.0)) / 2.5;
            let eb = (dbl - 18.0) / 3.0;
            let cost = ew * ew + ep * ep + eb * eb;
            if cost < best.0 {
                best = (cost, a, dbl);
            }
        }
    }
    let (_, a, dbl) = best;
    let temple = if w < 128.0 {
        135.0
    } else if w < 138.0 {
        140.0
    } else if w < 148.0 {
        145.0
    } else {
        150.0
    };
    let frame_pd = a + dbl;
    let dec = (frame_pd - pd) / 2.0;
    let total = front_width(a, dbl, rim);
    let offset = dec.abs();
    let eyebox = if i.eyebox_mm > 0.0 { i.eyebox_mm } else { 5.0 };
    let within = offset <= eyebox / 2.0;
    let mut notes = Vec::new();
    if (total - w).abs() > 6.0 {
        notes.push(format!(
            "Front width {total:.0} mm differs from the face width by {:.0} mm; consider a different rim width.",
            total - w
        ));
    }
    if !within {
        notes.push(format!(
            "The pupil sits {offset:.1} mm from the display axis — more than half the {eyebox:.1} mm eyebox. Shift the optics module or widen the eyebox."
        ));
    }
    if dec < -0.5 {
        notes.push("Frame PD is smaller than the wearer's PD: optical centres must be decentred outwards.".into());
    }
    let prism = (dec.abs() / 10.0) * i.sphere_d.abs();
    let cur_pd = cur.lens_width_mm + cur.bridge_mm;
    FitResult {
        size_label: format!("{a:.0} □ {dbl:.0} – {temple:.0}"),
        lens_width_mm: a,
        bridge_mm: dbl,
        temple_length_mm: temple,
        frame_pd_mm: frame_pd,
        total_width_mm: total,
        width_error_mm: total - w,
        decentration_per_eye_mm: dec,
        prentice_prism_d: prism,
        pupil_to_display_axis_mm: offset,
        within_eyebox: within,
        notes,
        patch: FitPatch {
            lens_width_mm: a,
            bridge_mm: dbl,
            temple_length_mm: temple,
        },
        current_frame_pd_mm: cur_pd,
        current_decentration_mm: (cur_pd - pd) / 2.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_adult_gets_a_typical_size() {
        let r = recommend(&FitInput {
            face_width_mm: 134.0,
            pd_mm: 64.0,
            ..Default::default()
        });
        assert!((48.0..=54.0).contains(&r.lens_width_mm), "{r:?}");
        assert!((16.0..=20.0).contains(&r.bridge_mm));
        assert!((r.frame_pd_mm - (r.lens_width_mm + r.bridge_mm)).abs() < 1e-12);
        assert!((r.decentration_per_eye_mm - (r.frame_pd_mm - 64.0) / 2.0).abs() < 1e-12);
    }

    #[test]
    fn prentice_rule() {
        // 4 mm decentration total (2 mm per eye) with a -4 D lens: 0.8 Δ.
        let r = recommend(&FitInput {
            face_width_mm: 134.0,
            pd_mm: 64.0,
            sphere_d: -4.0,
            ..Default::default()
        });
        let expect = r.decentration_per_eye_mm.abs() / 10.0 * 4.0;
        assert!((r.prentice_prism_d - expect).abs() < 1e-12);
    }

    #[test]
    fn wider_faces_get_wider_frames() {
        let narrow = recommend(&FitInput {
            face_width_mm: 124.0,
            pd_mm: 58.0,
            ..Default::default()
        });
        let wide = recommend(&FitInput {
            face_width_mm: 150.0,
            pd_mm: 70.0,
            ..Default::default()
        });
        assert!(wide.total_width_mm > narrow.total_width_mm + 15.0);
        assert!(wide.temple_length_mm > narrow.temple_length_mm);
    }
}
