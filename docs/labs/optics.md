# Optics Lab (`#/optics`, engine module `optics`)

First-order design of the Iris heads-up display (HUD): a micro-display, a thin collimating
lens and a 45° partially reflective combiner in front of the right eye (a
"birdbath / beam-splitter" layout). The engine (`crates/forge-core/src/optics/`)
works from `design.optics` and verifies requirements R-004 to R-007 and R-025.

**Optics Pro** extends the lab from first-order optics to real lens design — a
sequential ray tracer ("Zemax-lite") with real glasses, exact Snell refraction on
spherical, conic and even-aspheric surfaces, spot diagrams, ray fans, field curves,
colour, wavefront, modulation transfer function (MTF) and a damped-least-squares
optimiser — in three more tabs: **Lens designer**, **Image quality** and **Materials**.
The first-order metrics above do not change; Optics Pro adds informational metrics.

## Tabs

| Tab | What you do | What you learn |
|---|---|---|
| Ray diagram | To-scale top view of the folded path: fans of rays from five field points on the display, through the lens aperture, folded by the combiner into a reduced-eye model where each colour focuses to one retinal spot; animated light pulses (play/pause, reduced-motion aware); dashed virtual rays showing where the eye thinks the light comes from; world light through the combiner; focal points; a collision marker if the combiner would hit the lens. Sliders for focal length, display position, aperture, lens-to-combiner, eye relief, display width, pixels, luminance and combiner reflectance write `design.optics`. Stat tiles for field of view (FOV), pixels per degree (PPD), virtual image distance and eyebox; first-order numbers and the ray-transfer (ABCD) matrix; a to-scale unfolded view of the FOV cone and the virtual image against the R-006 1–3 m band. | The magnifier principle, thin-lens imaging, the fold, ABCD matrices, accommodation. |
| What you see | First-person street scene (drawn with SVG, no images) with the HUD overlaid at its true angular size relative to a 100° (or zoomed 50°) view, drawn from a canvas at the display's native resolution; navigation, captions or message content; background brightness from a night street to sunlit snow; contrast ratio, needed display luminance, a pixel loupe and contrast vs background. | Additive see-through displays, luminance through optics, why outdoor legibility is hard. |
| Eyebox | Slide or drag the eye sideways, change pupil size, lens aperture and eye relief; rays missing the pupil are faded and marked; per-field pupil fill; visible FOV vs eye offset with the eyebox shaded; a strip showing which part of the display the eye still sees. | Aperture stop, vignetting, why eyebox shrinks with FOV and eye relief. |
| Legibility | Font size in display pixels → cap height in arcminutes → Snellen equivalent; letters rasterised on the display's pixel grid and drawn at a common angular scale next to a 20/20 optotype; a 1 : 1 frame-buffer preview; characters per line and lines. | Text size is an angle; pixel-limited vs eye-limited resolution. |
| Design space | HFOV, PPD and eyebox vs focal length (display refocused so the image distance is unchanged) with requirement thresholds and the feasible band shaded; the PPD × FOV = pixels frontier for 640/1280/1920-pixel panels; move Iris to the band centre. | The classic FOV-vs-resolution trade of head-mounted displays. |
| **Lens designer** (Optics Pro) | A Lens Data Editor (radius, thickness, material, conic k, A4, A6, clear semi-diameter per surface; insert/delete rows) for the collimator traced *reversed* — eye pupil (the stop) → unfolded combiner → lens → display. Live to-scale layout with real refracted ray fans at the F, d and C wavelengths (animated light pulses, play/pause, reduced-motion aware), element centre/edge thickness chips, first-order data from the paraxial y–nu trace. Presets: ideal thin lens, PMMA plano-convex singlet, PMMA asphere singlet, cemented COP + PC moulded achromat, cemented N-BK7 + N-SF11 glass achromat — all scaled to the project's focal length and aperture. Tick c / k / A4 / A6 per surface and press **Optimise**: damped least squares runs and the lens bends on screen iteration by iteration while the RMS-spot curve falls; Undo restores the start. **Use for Iris metrics** stores the preset (or the edited prescription) in `design.extra`. | Real rays vs paraxial, why eyepieces are designed backwards, what bending, conics, aspheres and a second glass buy. |
| **Image quality** (Optics Pro) | Spot diagrams at 0, 0.5, 0.71 and 1.0 of the half-diagonal field (polychromatic or per wavelength) on a common scale with one display pixel and the Airy disc drawn; RMS radius in µm and arcminutes; MTF vs spatial frequency (tangential and sagittal per field, diffraction limit) with the display's Nyquist frequency marked; field curvature (T and S, three colours) and distortion; transverse ray fans e_y(p_y), e_x(p_x) and the optical path difference (OPD) fan; chromatic focal shift vs wavelength and lateral colour vs field; wavefront RMS, peak-to-valley and Strehl per field. | Reading spot diagrams and ray fans like a lens designer; aberration-limited vs diffraction-limited. |
| **Materials** (Optics Pro) | Dispersion curves n(λ) from 400 to 800 nm; an Abbe diagram (glass map, nd vs Vd) of the catalogue; an achromat calculator that picks a crown and a flint, solves φ₁/V₁ + φ₂/V₂ = 0 and traces the singlet against the doublet side by side; the catalogue table with computed vs datasheet nd and Vd. | Sellmeier and Cauchy dispersion, the Abbe number, the achromat condition. |

Downloads: ray diagram as SVG (theme colours resolved), ray data JSON (geometry, fields,
rays, analysis), eyebox sweep CSV, legibility table CSV, design-space sweep CSV; Optics
Pro adds the lens layout SVG, the prescription as `system.json` and CSV (surface, label,
radius, thickness, material, conic, A4–A10, semi-diameter), a starting `rayoptics` Python
script, spot-diagram and MTF CSVs, dispersion CSV and the Abbe diagram SVG. A Ray Optics
Simulation scene export is not included: the converter belongs to the `bridge` module
(another branch), and duplicating it here was out of scope.

## Engine API

| Method | Input | Output |
|---|---|---|
| `optics.analyze` | `{project? \| optics?, extras?: {pupil_mm, lens_transmission, backgrounds, world_m}}` | image distance and kind (virtual/collimated/real), virtual image distance, magnification, image size, H/V/D FOV, collimated FOV, PPD (average, centre, vertical), arcmin per pixel, pixel pitch, f-number, NA, eyebox (exact, approximation, full-pupil, partial-pupil, vertical), k, minimum combiner length, minimum lens-to-combiner distance, ABCD matrix, accommodation, focus sensitivity, see-through, image luminance, contrast table, warnings |
| `optics.trace` | `{project? \| optics?, extras?, config: {fields, rays, eye_offset_mm, back_mm, sweep_points}}` | analysis, geometry (display, lens, combiner, eye, pupil, retina), rays (folded polyline into the eye, through-pupil flag, chief flag, virtual back-extension, unfolded polyline), fields (angle, pupil fill, virtual image point, retinal point), eyebox sweep (visible FOV and per-field fill vs eye offset) |
| `optics.design_sweep` | `{project?, config: {f_min_mm, f_max_mm, points, min_hfov_deg?, min_ppd?, min_eyebox_mm?}}` | HFOV, PPD, eyebox (fixed D and fixed f-number), pixels needed vs f; feasible bands (thresholds read from the project's requirements) |
| `optics.legibility` | `{project?, config: {sizes_px, cap_ratio, advance_ratio, line_ratio}}` | per font size: cap px, arcmin, Snellen 20/x, characters per line, lines, verdict; smallest em for 16′ and 20′ |
| `optics.contrast` | `{project?, extras?}` | contrast ratio and display luminance needed for 3:1 vs background (1–31 600 nits); backgrounds where contrast reaches 3:1 and 1.5:1 |
| `optics.materials` | – | catalogue: id, name, kind, nd, Vd, nF, nC, datasheet nd/Vd, density, source, notes, dispersion formula and coefficients, n(λ) sampled 400–800 nm |
| `optics.lens_presets` | `{project? \| optics?}` | presets (id, title, summary, suggested optimisation variables) as complete reversed-HUD systems at the project's focal length, with lens EFL and mass |
| `optics.lens_analyze` | `{project? \| optics?, preset? \| system?, config?: {rings, rays_per_fan, fan_points, auto_focus}}` | the focused system, meridional layout (surface profiles, element outlines, ray fans per field and wavelength with failure flags), `quality` (paraxial data per wavelength, spots, fans, field curves, colour, wavefront, MTF, worst-field summaries, warnings), lens EFL and mass, pixel pitch, Nyquist, element centre/edge thicknesses |
| `optics.lens_optimize` | `{project? \| optics?, preset? \| system?, variables?: [{surface, param}], max_iter?, efl_mm?, lens_start?}` | damped-least-squares history (RMS spot, merit, EFL, damping, variable values and lens surfaces per iteration), a layout per iteration for the animation, the final system, start/final RMS, evaluation count |
| `optics.achromat` | `{f_mm, crown, flint}` | indices and Abbe numbers, thin-lens powers φ₁, φ₂, element focal lengths, equiconvex-crown cemented radii R₁ R₂ R₃, Petzval radius, singlet colour f/V |

A `System` is `{name, surfaces: [{label, radius, thickness, material, conic, aspheric: [A4, A6, A8, A10], semi_diameter, paraxial_f, fold}], pupil_diameter_mm, object_distance_mm (null = ∞, negative = virtual), fields_deg, wavelengths_nm, primary}`; surface 0 is the stop, `material` is the medium *after* the surface (`""` = air, a catalogue id, a model glass `"nd/vd"`, or `MIRROR`), `paraxial_f ≠ 0` makes an ideal thin lens.

### Metrics

| Key | Meaning | Iris (defaults) | Requirement |
|---|---|---|---|
| `optics.hfov_deg` | horizontal FOV at the eye | **23.6°** | R-004 ≥ 20° ✓ |
| `optics.ppd` | 640 px / HFOV | **27.2 px/°** | R-005 ≥ 25 ✓ |
| `optics.virtual_image_m` | \|sᵢ\| + lens-to-eye path | **1.34 m** (1313 + 29 mm) | R-006 1–3 m ✓ |
| `optics.eyebox_mm` | D·\|k\| − w·L/d (pupil-centre criterion) | **6.03 mm** (collimated approximation 5.90 mm) | R-007 ≥ 5 mm ✓ |
| `optics.combiner_clearance_mm` | lens-to-combiner minus the minimum for the 45° combiner to clear the lens | **+0.78 mm** (11 mm vs 10.2 mm) | R-025 ≥ 0 ✓ |
| `optics.vfov_deg`, `optics.dfov_deg` | vertical / diagonal FOV | 14.9°, 27.6° | – |
| `optics.image_nits` | 3000 × 0.92 × 0.30 | 828 nits | – |
| `optics.see_through_pct` | 1 − R | 70 % | – |
| `optics.rms_spot_um` | Optics Pro: polychromatic RMS spot radius on the display, worst field, real rays, 4 mm eye pupil, display at best focus | **19.4 µm** (1.5 pixels; COP + PC achromat) | informational |
| `optics.rms_spot_arcmin` | the same as an angle at the eye (spot / EFL) | 3.33′ (one pixel = 2.26′) | informational |
| `optics.mtf_at_nyquist` | geometric × diffraction MTF at the display Nyquist (38.1 cycles/mm), worst field, min of T/S | **0.03** (on-axis 0.44; diffraction limit 0.86) | informational |
| `optics.lateral_color_arcmin` | chief-ray height F − C, worst field, as an angle | 5.79′ | informational |

The Optics Pro metrics use the lens selected in the Lens designer
(`design.extra.optics_lens`, a preset id, or `"custom"` with the prescription in
`design.extra.optics_lens_rx`); the default is the COP + PC achromat. They are
memoised, so they are recomputed only when the optics parameters or the lens change (a lighter 4-ring trace; the unit test bounds one evaluation at 400 ms in an unoptimised debug build).

**Suggested requirements (not added — `examples/` belongs to the integrator).** With the
default lens, the corner of the field is 1.5 pixels RMS and the MTF at Nyquist is only
0.03 at the corner (0.44 on axis): alternating single-pixel lines are not resolved in the
corners by any simple lens here. Realistic first targets for Iris:
`optics.rms_spot_um ≤ 26` (two pixels, worst field; passes at 19.4 µm) and
`optics.lateral_color_arcmin ≤ 2 × pixel ≈ 4.5′` (fails at 5.8′ — a real finding: the
optimiser traded colour for spot size, so a colour-weighted merit or a third element is
needed). An MTF requirement should be written on axis or at half field, e.g.
`MTF ≥ 0.3 at Nyquist/2`, rather than at full Nyquist in the corner; that needs a new
metric key.

### Lens presets (Iris: f = 20 mm, 18 mm aperture, 4 mm eye pupil, ±13.8° diagonal, 29 mm eye-to-lens)

| Preset | Worst RMS spot | Per field 0 / 6.9 / 9.8 / 13.8° | MTF at Nyquist worst / on-axis | Lateral colour | Axial colour F−C | Mass |
|---|---|---|---|---|---|---|
| Ideal thin lens | 0 | 0 | 0.86 / 0.86 (diffraction) | 0 | 0 | – |
| PMMA plano-convex singlet | 743 µm (128′) | 78 / 59 / 43 / 128′ | 0.02 / 0.01 | 22.8′ | 310 µm | 1.26 g |
| PMMA asphere singlet | 53 µm (9.1′) | 2.2 / 5.1 / 6.8 / 9.1′ | 0.06 / 0.22 | 22.4′ | 316 µm | 0.89 g |
| **COP + PC achromat (moulded, default)** | **19 µm (3.3′)** | 1.4 / 3.2 / 3.2 / 3.3′ | 0.03 / 0.44 | 5.8′ | 166 µm | 2.02 g |
| Glass achromat N-BK7 + N-SF11 (spheres) | 106 µm (18′) | 16 / 7.8 / 16 / 18′ | 0.02 / 0.02 | 6.1′ | 89 µm | 5.09 g |

The asphere and achromats come from running the optimiser from textbook starting points
(`cargo test -p forge-core optics::lensdesign::bake -- --ignored --nocapture` reprints them);
unit tests check that the aspheric singlet beats the spherical one by > 3×, the achromats cut
lateral colour of the singlet by > 3× and axial colour ≈ f/V for the singlet. The all-spherical
glass doublet is excellent for a *camera* at f/5 with the stop at the lens, but here the stop
(the eye) is 29 mm away, the corner chief ray crosses the lens 7 mm off axis, and spheres
cannot control the resulting astigmatism and field curvature — which is why moulded aspheres
dominate head-mounted displays (Kress 2020). A PMMA + PC version of the moulded achromat was
tried; every optimum found sat on a knife edge (rays grazing total internal reflection at the
field corner; a 10⁻⁵ relative change in the radii made it an order of magnitude worse), so it is not offered as a preset.

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

With the original Iris defaults (18 mm lens aperture, 8 mm lens-to-combiner), the upper edge of a
45° combiner that catches the whole beam lay *behind* the lens plane — the parts would
intersect. The engine reports the minimum lens-to-combiner distance (10.2 mm with 1 mm
clearance), a warning, the metric `optics.combiner_clearance_mm`, and the ray diagram
marks the collision. The project now uses 11 mm, which keeps every requirement (eyebox
6.0 mm) and is verified by R-025.

## Optics Pro: the maths

* **Surfaces** (Smith ch. 13; Kingslake & Johnson ch. 3): even asphere
  `z(r) = c r²/(1 + √(1 − (1+k)c²r²)) + A₄r⁴ + A₆r⁶ + A₈r⁸ + A₁₀r¹⁰`, c = 1/R, k the conic
  constant (0 sphere, −1 paraboloid). The ray–conic intersection is the stable root of a
  quadratic; the polynomial terms are refined by Newton's method (tested: the hit point lies
  on the surface to 10⁻¹⁰ mm).
* **Refraction** (Born & Wolf §3.2; Welford 1986): vector Snell's law
  `d' = μd + (√(1 − μ²(1 − cos²I)) − μ cos I) n`, μ = n₁/n₂, total internal reflection
  when the root is imaginary; mirrors `d' = d − 2(d·n)n` (tested: a paraboloid images an
  axial point perfectly; a flat interface obeys n₁ sin θ₁ = n₂ sin θ₂).
* **Paraxial y–nu trace**: `n′u′ = nu − yφ`, `φ = (n′ − n)c`, `y ← y + t u′`; EFL, BFL,
  FFL, principal planes, paraxial image, exit pupil and working f-number (tested against
  the thick-lens lensmaker's equation and R = (n − 1)f for a plano-convex lens).
* **Reversed HUD model** (Smith ch. 13; Kress 2020 ch. 5): the eye pupil is the stop,
  the object is the virtual image traced as a *virtual* object 1.34 m behind the eye (or
  infinity), the flat combiner is unfolded, the display is the image plane. Blur on the
  display converts to what the eye sees by dividing by the EFL; one 13.1 µm pixel is 2.26′.
* **Spot diagram**: hexapolar pupil grid (127 rays per field and wavelength by default),
  RMS radius about the polychromatic centroid; the image plane is moved to the closed-form
  best focus (the summed squared radius is a quadratic in the plane position). Airy radius
  `1.22 λ N_w`.
* **Ray fans, field curves, distortion**: transverse errors relative to the d-line chief
  ray; tangential and sagittal foci from close neighbour rays (Coddington numerically);
  distortion `(y_real − y_paraxial)/y_paraxial`.
* **Colour**: longitudinal = paraxial image position F − C (a singlet ≈ f/V, tested);
  lateral = chief-ray image height F − C; chromatic focal shift 420–680 nm.
* **Wavefront**: optical path to a reference sphere centred on the chief-ray image point
  through the exit pupil, piston and tilt removed; Strehl ≈ exp(−(2πσ)²) (Maréchal).
* **MTF** — *geometric × diffraction*, not a fast-Fourier-transform (FFT) of the point
  spread function: `MTF_g(ν) = |⟨exp(−2πiνy)⟩|` over the spot, multiplied by the
  circular-pupil diffraction MTF `(2/π)(φ − cos φ sin φ)`, `φ = acos(ν/ν_c)`,
  `ν_c = 1/(λN)` (Goodman §6.3; tested: 0.391 at ν_c/2). Nyquist = 1/(2·pitch) = 38.1
  cycles/mm on the display. Good when blur is either large (geometric) or tiny
  (diffraction); near the diffraction limit an FFT MTF is more accurate.
* **Glasses**: SCHOTT three-term Sellmeier `n² − 1 = Σ Bᵢλ²/(λ² − Cᵢ)` for N-BK7, N-SF11,
  N-SF6 and N-F2 (SCHOTT datasheets, as tabulated on refractiveindex.info) and fused silica
  (Malitson 1965) — unit tests reproduce the datasheet nd to 2·10⁻⁵ and Vd to 0.02.
  **Plastics** (PMMA, polycarbonate, cyclo-olefin polymer (COP, Zeonex-class), polystyrene)
  vary by grade, so they use a two-term Cauchy `n = A + B/λ²` fitted exactly to *typical*
  nd and Vd — check the resin datasheet before committing a mould.
* **Achromat** (Kingslake & Johnson ch. 5): `φ₁/V₁ + φ₂/V₂ = 0` ⇒ `φ₁ = φV₁/(V₁ − V₂)`,
  `φ₂ = −φV₂/(V₁ − V₂)`.
* **Optimiser** — damped least squares (Levenberg 1944; Marquardt 1963), the default local
  optimiser of commercial lens-design programs: residuals are angular ray errors about each
  field's centroid (all fields × F, d, C × a hexapolar grid), a heavily weighted EFL target,
  penalties for rays lost to total internal reflection (60′) and vignetting (8′, so the
  optimiser cannot "improve" a field by throwing its worst rays out of the aperture);
  finite-difference Jacobian, `(JᵀJ + λ diag JᵀJ)Δx = −Jᵀf`, λ ÷ 3 on success and × 6 on
  failure; the display is refocused in closed form and element thicknesses follow the
  curvatures so edges stay ≥ 0.8 mm (centres ≥ 1 mm). It finds the nearest local minimum.

## Limitations and where to go next

* The first-order tabs are paraxial, thin-lens, single-wavelength. Optics Pro adds real
  surfaces and glasses but stays a teaching-grade sequential model: rotationally symmetric
  surfaces only (no tilts/decentres, so the 45° combiner is unfolded rather than traced),
  meridional fields, no coatings, ghosts, polarisation, stray light, tolerancing,
  thermal or birefringence effects in moulded plastics; MTF is geometric × diffraction
  rather than FFT or Huygens; the optimiser is local, with a fixed stop and fixed glasses.
  Graduate to Zemax OpticStudio, CODE V, or the open-source `rayoptics` (Python) — the
  exported prescription and `rayoptics` script are starting points.
* 2D horizontal section; the vertical eyebox is reported but not traced; combiner
  treated as an ideal thin partial mirror (no ghost from its second surface, no
  polarisation, no absorption).
* The first-person view is illustrative (perceived contrast mapped to overlay opacity).

## References

* E. Hecht, *Optics*, 5th ed., Pearson, 2017.
* W. J. Smith, *Modern Optical Engineering*, 4th ed., McGraw-Hill, 2008.
* R. Kingslake & R. B. Johnson, *Lens Design Fundamentals*, 2nd ed., Academic Press / SPIE Press, 2010.
* M. Born & E. Wolf, *Principles of Optics*, 7th ed., Cambridge University Press, 1999.
* J. W. Goodman, *Introduction to Fourier Optics*, 3rd ed., Roberts & Company, 2005.
* D. C. O'Shea, *Elements of Modern Optical Design*, Wiley, 1985.
* W. T. Welford, *Aberrations of Optical Systems*, Adam Hilger, 1986.
* K. Levenberg, "A method for the solution of certain non-linear problems in least squares", *Quarterly of Applied Mathematics* 2, 164–168 (1944).
* D. W. Marquardt, "An algorithm for least-squares estimation of nonlinear parameters", *Journal of the Society for Industrial and Applied Mathematics* 11, 431–441 (1963).
* SCHOTT, *Optical Glass — Data Sheets* and Technical Information TIE-29, "Refractive Index and Dispersion".
* M. N. Polyanskiy, refractiveindex.info — refractive index database, https://refractiveindex.info
* I. H. Malitson, "Interspecimen comparison of the refractive index of fused silica", *J. Opt. Soc. Am.* 55, 1205 (1965).
* B. C. Kress, *Optical Architectures for Augmented-, Virtual-, and Mixed-Reality Headsets*, SPIE Press, 2020.
* J. P. Rolland & H. Hua, "Head-mounted display systems", *Encyclopedia of Optical Engineering*, 2005.
* ANSI/HFES 100-2007, *Human Factors Engineering of Computer Workstations*.
* rayoptics: https://github.com/mjhoptics/ray-optics
