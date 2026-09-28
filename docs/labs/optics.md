# Optics Lab (`#/optics`, engine module `optics`)

First-order design of the Iris heads-up display: a micro-display, a thin collimating
lens and a 45° partially reflective combiner in front of the right eye (a
"birdbath / beam-splitter" layout). The engine (`crates/forge-core/src/optics/`)
works from `design.optics` and verifies requirements R-004 to R-007.

## Tabs

| Tab | What you do | What you learn |
|---|---|---|
| Ray diagram | To-scale top view of the folded path: fans of rays from five field points on the display, through the lens aperture, folded by the combiner into a reduced-eye model where each colour focuses to one retinal spot; animated light pulses (play/pause, reduced-motion aware); dashed virtual rays showing where the eye thinks the light comes from; world light through the combiner; focal points; a collision marker if the combiner would hit the lens. Sliders for focal length, display position, aperture, lens-to-combiner, eye relief, display width, pixels, luminance and combiner reflectance write `design.optics`. Stat tiles for field of view (FOV), pixels per degree (PPD), virtual image distance and eyebox; first-order numbers and the ray-transfer (ABCD) matrix; a to-scale unfolded view of the FOV cone and the virtual image against the R-006 1–3 m band. | The magnifier principle, thin-lens imaging, the fold, ABCD matrices, accommodation. |
| What you see | First-person street scene (drawn with SVG, no images) with the HUD overlaid at its true angular size relative to a 100° (or zoomed 50°) view, drawn from a canvas at the display's native resolution; navigation, captions or message content; background brightness from a night street to sunlit snow; contrast ratio, needed display luminance, a pixel loupe and contrast vs background. | Additive see-through displays, luminance through optics, why outdoor legibility is hard. |
| Eyebox | Slide or drag the eye sideways, change pupil size, lens aperture and eye relief; rays missing the pupil are faded and marked; per-field pupil fill; visible FOV vs eye offset with the eyebox shaded; a strip showing which part of the display the eye still sees. | Aperture stop, vignetting, why eyebox shrinks with FOV and eye relief. |
| Legibility | Font size in display pixels → cap height in arcminutes → Snellen equivalent; letters rasterised on the display's pixel grid and drawn at a common angular scale next to a 20/20 optotype; a 1 : 1 frame-buffer preview; characters per line and lines. | Text size is an angle; pixel-limited vs eye-limited resolution. |
| Design space | HFOV, PPD and eyebox vs focal length (display refocused so the image distance is unchanged) with requirement thresholds and the feasible band shaded; the PPD × FOV = pixels frontier for 640/1280/1920-pixel panels; move Iris to the band centre. | The classic FOV-vs-resolution trade of head-mounted displays. |

Downloads: ray diagram as SVG (theme colours resolved), ray data JSON (geometry, fields,
rays, analysis), eyebox sweep CSV, legibility table CSV, design-space sweep CSV.

## Engine API

| Method | Input | Output |
|---|---|---|
| `optics.analyze` | `{project? \| optics?, extras?: {pupil_mm, lens_transmission, backgrounds, world_m}}` | image distance and kind (virtual/collimated/real), virtual image distance, magnification, image size, H/V/D FOV, collimated FOV, PPD (average, centre, vertical), arcmin per pixel, pixel pitch, f-number, NA, eyebox (exact, approximation, full-pupil, partial-pupil, vertical), k, minimum combiner length, minimum lens-to-combiner distance, ABCD matrix, accommodation, focus sensitivity, see-through, image luminance, contrast table, warnings |
| `optics.trace` | `{project? \| optics?, extras?, config: {fields, rays, eye_offset_mm, back_mm, sweep_points}}` | analysis, geometry (display, lens, combiner, eye, pupil, retina), rays (folded polyline into the eye, through-pupil flag, chief flag, virtual back-extension, unfolded polyline), fields (angle, pupil fill, virtual image point, retinal point), eyebox sweep (visible FOV and per-field fill vs eye offset) |
| `optics.design_sweep` | `{project?, config: {f_min_mm, f_max_mm, points, min_hfov_deg?, min_ppd?, min_eyebox_mm?}}` | HFOV, PPD, eyebox (fixed D and fixed f-number), pixels needed vs f; feasible bands (thresholds read from the project's requirements) |
| `optics.legibility` | `{project?, config: {sizes_px, cap_ratio, advance_ratio, line_ratio}}` | per font size: cap px, arcmin, Snellen 20/x, characters per line, lines, verdict; smallest em for 16′ and 20′ |
| `optics.contrast` | `{project?, extras?}` | contrast ratio and display luminance needed for 3:1 vs background (1–31 600 nits); backgrounds where contrast reaches 3:1 and 1.5:1 |

### Metrics

| Key | Meaning | Iris (defaults) | Requirement |
|---|---|---|---|
| `optics.hfov_deg` | horizontal FOV at the eye | **23.6°** | R-004 ≥ 20° ✓ |
| `optics.ppd` | 640 px / HFOV | **27.1 px/°** | R-005 ≥ 25 ✓ |
| `optics.virtual_image_m` | \|sᵢ\| + lens-to-eye path | **1.34 m** (1313 + 26 mm) | R-006 1–3 m ✓ |
| `optics.eyebox_mm` | D·\|k\| − w·L/d (pupil-centre criterion) | **7.27 mm** (collimated approximation 7.13 mm) | R-007 ≥ 5 mm ✓ |
| `optics.vfov_deg`, `optics.dfov_deg` | vertical / diagonal FOV | 14.9°, 27.7° | – |
| `optics.image_nits` | 3000 × 0.92 × 0.30 | 828 nits | – |
| `optics.see_through_pct` | 1 − R | 70 % | – |
| `optics.combiner_clearance_mm` | lens-to-combiner minus the minimum for the 45° combiner to clear the lens | **−2.2 mm** (see below) | – |

## The maths

* **Imaging** (Hecht §5.2): `1/sₒ + 1/sᵢ = 1/f`, `sᵢ = f d/(d − f)`, `m = −sᵢ/sₒ`. With
  d = 19.7 mm, f = 20 mm: sᵢ = −1313 mm (virtual), m = 66.7, a 56 × 35 cm image at 1.34 m.
  Focus sensitivity `d(1/sᵢ)/dd = 1/d²` ≈ 2.6 dioptres per mm of display shift.
* **FOV**: `2·atan((m w/2)/(|sᵢ| + L))`, → `2·atan(w/2f)` when collimated (tested).
  **PPD** = pixels / FOV (the centre PPD, 26.7, is used for arcmin per pixel).
* **ABCD**: `T(t) = [1 t; 0 1]`, `Λ(f) = [1 0; −1/f 1]`, `M = T(L) Λ(f) T(d)`; tested that
  ABCD-traced rays back-extrapolate to the thin-lens image and that `B = 0` at a real image.
* **Trace**: exact paraxial refraction at the lens, intersection with the 45° mirror
  through (L_c, 0) and reflection `d' = d − 2(d·n)n`; the unfolded point (z, y) maps to
  (L_c − y, L_c − z) — the image is mirrored left–right, so firmware flips the frame
  buffer. Reduced eye: one surface of 60 D plus accommodation `1/s`, index 1.336, retina
  22.3 mm; tested that every ray of a field lands on one retinal point.
* **Eyebox** (Rolland & Hua 2005; Kress 2020): beam footprint at the eye is `D|k|` wide,
  shifted by `−y₀L/d`, `k = 1 + L/d − L/f`; pupil-centre eyebox `D|k| − wL/d`
  → `D − 2L tan(FOV/2)` collimated (tested); `∓ p` for full/partial pupil fill; the sweep
  counts a field point visible when it lights ≥ 50 % of the pupil.
* **Photometry**: luminance is conserved by an ideal magnifier; `L_img = L_disp T_lens R`;
  contrast `CR = (L_img + (1−R) L_bg)/((1−R) L_bg)`. Iris: 12.8 : 1 indoors (100 nits),
  1.6 : 1 overcast, 1.24 : 1 sunny street — the outdoor legibility problem.
* **Legibility**: cap height (arcmin) = cap px × 60/PPD; 20/20 letter = 5′; comfortable ≈
  16′ minimum, 20–22′ preferred (display ergonomics guidance, e.g. ANSI/HFES 100-2007).
  Iris needs ≥ 11 px em (16′); at 13 px it fits 82 characters × 24 lines.
* **Design space**: FOV ≥ 20° ⇒ f ≤ 23.8 mm, PPD ≥ 25 ⇒ f ≥ 18.5 mm; Iris's 20 mm is inside.

## Design finding: the combiner collides with the lens

With the Iris defaults (18 mm lens aperture, 8 mm lens-to-combiner), the upper edge of a
45° combiner that catches the whole beam lies *behind* the lens plane — the parts would
intersect. The engine reports the minimum lens-to-combiner distance (10.2 mm with 1 mm
clearance), a warning, the informational metric `optics.combiner_clearance_mm`, and the
ray diagram marks the collision. Raising `lens_to_combiner_mm` to 11 mm keeps every
requirement (eyebox 6.0 mm; 5.6 mm at 12 mm) — a change to `examples/smart-glasses/project.json`
is suggested but not made here.

## Limitations and where to go next

* Paraxial, thin-lens, single-wavelength: no aberrations (field curvature, distortion,
  astigmatism and lateral colour dominate a single f/1.1 lens at ±12°), no lens thickness,
  no diffraction or MTF. Graduate to Zemax OpticStudio, CODE V, or the open-source
  `rayoptics` (Python) for real surfaces and glasses.
* 2D horizontal section; the vertical eyebox is reported but not traced; combiner
  treated as an ideal thin partial mirror (no ghost from its second surface, no
  polarisation, no absorption).
* The first-person view is illustrative (perceived contrast mapped to overlay opacity).

## References

* E. Hecht, *Optics*, 5th ed., Pearson, 2017.
* W. J. Smith, *Modern Optical Engineering*, 4th ed., McGraw-Hill, 2008.
* B. C. Kress, *Optical Architectures for Augmented-, Virtual-, and Mixed-Reality Headsets*, SPIE Press, 2020.
* J. P. Rolland & H. Hua, "Head-mounted display systems", *Encyclopedia of Optical Engineering*, 2005.
* ANSI/HFES 100-2007, *Human Factors Engineering of Computer Workstations*.
* rayoptics: https://github.com/mjhoptics/ray-optics
