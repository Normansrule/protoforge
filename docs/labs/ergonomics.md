# Human Factors Lab (`#/ergonomics`, engine module `ergonomics`)

Smart glasses live or die by comfort and fit. The Human Factors Lab takes the
**Iris smart glasses** — the frame geometry from the computer-aided design
(CAD) generator, the masses and locations of every component, and the display
optics — and puts them on people: published head and face statistics become a
seeded Monte Carlo population that tries the frame on, the masses load the
nose and the ears, the temples become springs that squeeze the head, and the
heads-up display is checked for angular text size, ambient contrast ratio
(ACR) in sunlight and whether each wearer's pupil lands inside the eyebox.
Everything runs in the Rust engine compiled to WebAssembly; the page animates
the results.

## Tabs

| Tab | What you get |
|---|---|
| Anthropometry & fit | A **parametric 3-D head** (sex and percentile sliders for head breadth, interpupillary distance (IPD), hinge-to-ear distance and nose bridge width) wearing the real Iris CAD parts; the temples swing open about their hinge pins until they meet the head, contact and ear markers turn green or red, and chips report pupil centring, temple length, clamp force and eyebox. **Fit rules** (temple splay, decentration tolerance, ear window, share of women, sample size, seed), saved to `design.extra.ergonomics`. An animated **Monte Carlo scatter** (head breadth vs IPD) with a distinct marker per failure reason (fits, too tight, too loose, pupils off-centre, temple length) over the comfort and PD windows, and coverage bars for women, men and combined. **Anthropometry** cards: probability density curves per dimension with the Iris reference lines, percentile tables (P1–P99) with the ANSUR II empirical percentiles beside the normal approximation, measured-vs-assumption chips, and editable means/SDs. **S/M/L size search**: three sizes (lens width, bridge, temple length, splay) that maximise coverage, each with its comfortable head-breadth range and the share it serves, and a "Use for Iris" button. Downloads: population CSV, percentile CSV, size-range CSV. |
| Weight & balance | **Animated force diagram**: side view that nods through the look-down angle with weight, nose and ear reactions and the slide force drawn to scale, and a rear-facing front view with each pad's normal force drawn perpendicular to the nose flank (the wedge multiplies the vertical load by 1/sin β), each ear's load and the centre-of-mass offset; a holds/slips chip and a friction-needed vs pitch plot. **Battery what-if** (move the left-pod parts towards the ear and watch the nose load fall; the 25 gf line is an assumed design target, not a published limit), **nose-pad pressure vs pad area** against the capillary-pressure limit, and the **mass breakdown** (CAD parts + components). Download: mass-properties CSV. |
| Temple clamp | **Top view at true scale**: the head breadth sweeps the population, the temples bend with their computed cantilever shape plus the front's hinge rotation, force arrows and a comfortable / too tight / too loose verdict. **Clamp force vs head-breadth percentile** for men, women and combined with the comfort band; a P5/P50/P95 table; the **compliance breakdown** (front bending vs each temple) with every segment's second moment of area; sliders for splay, contact point, contact length, pressure limit, friction and look-down angle; "use the best splay". Download: clamp-force CSV. |
| Legibility & eyebox | **Heads-up text against the world**: an animated street scene cycling dim room → office → overcast → sunlit → snow, where the caption's light is *added* to the see-through background and tone-mapped, with the ACR and verdict; a saved tint slider and a what-if display luminance. **Text size vs acuity**: letter ladder against the acuity threshold and the comfortable band, table of cap heights in arcminutes and Snellen equivalents. **ACR by scene** with the display luminance needed for 3 : 1. **Eyebox coverage**: animated right-eye pupils falling inside or outside the eyebox, the IPD density with the accepted window, coverage vs display offset, face-asymmetry slider and "centre on the population". Download: contrast CSV. |
| How it works | An animated pipeline from body statistics to requirement checks; a live **Iris scorecard** of the `ergonomics.*` metrics against the targets this lab recommends, each with the fix recomputed from the current design and a link to its tab; a **lever-rule demo** (slide or animate the centre of mass between nose pads and ear roots); a **Monte Carlo convergence demo** (running share passing a rule with its 95 % confidence band, log n axis); the "average man" fallacy, data provenance, comfort of head-worn products, seeing the display, temple mechanics, limitations and tools to graduate to. |

## Engine API

| Method | Input | Output |
|---|---|---|
| `ergonomics.defaults` | – | the `ErgoConfig` defaults (what `design.extra.ergonomics` may hold) |
| `ergonomics.population` | `{project?, config?}` | per dimension: definition, source, measured flag, men/women normal parameters, ANSUR II empirical percentiles, mean/SD and P1…P99 for men, women and combined, density curves; correlation matrices |
| `ergonomics.fit` | `{project?, config?, scatter?}` | frame summary, comfort band, coverage (fit, PD, temple, clamp, too tight/loose) for combined/men/women, the first `scatter` simulated people with their checks, clamp at P5/50/95, band head range, PD and ear windows |
| `ergonomics.sizes` | `{project?, config?, count = 3, samples = 2000, vary_splay = true}` | sizes (label, frame PD, width, alone %, served %, comfortable head range), union coverage, Iris coverage, notes |
| `ergonomics.head` | `{project?, config?, sex, head_breadth_pct, ipd_pct, ear_pct, nose_pct, size?}` | head, nose and ear meshes, eyes, pupils, ear tops, contacts; the person, the fit check, temple opening angles, hinge pin, deflections, pupil offset and eyebox |
| `ergonomics.balance` | `{project?, config?, battery_shift_mm, pitch_deg}` | mass items with positions, total mass, centre of mass, supports, loads upright / pitched / at the look-down angle (reactions, nose and ear share, pad normal force and pressure, slide force, friction needed), comfort band, pad area for the limit, curves (nose load vs battery shift, pressure vs pad area, friction vs pitch), side masses, frame summary, notes |
| `ergonomics.clamp` | `{project?, config?}` | the clamp model (segments, compliances, rest spread, contact distance), band, force vs percentile and vs head breadth, P5/50/95 table, force at P95, band coverage, best splay and its coverage, root stress, deflections, yaw, notes |
| `ergonomics.display` | `{project?, config?, custom_nits?}` | arcmin per pixel, pixels per degree, field of view, acuity, caption size and verdict, legibility rows, photometry, scene rows, the bright-scene row, eyebox analysis (coverage per group, window, best offset, coverage vs offset, eyebox for 95 %, IPD density and statistics), ACR vs background |

`config` overrides `design.extra.ergonomics`; every field is optional.

## The maths

**Normal approximations and mixtures.** Each dimension is `N(μ, σ²)` per sex;
percentiles are `x_p = μ + z_p σ` (z₅ = −1.645). The combined population is
the mixture `F(x) = w Φ((x − μ_w)/σ_w) + (1 − w) Φ((x − μ_m)/σ_m)` with `w` the
share of women; its percentiles are found by bisection, its SD by the law of
total variance. Φ uses the Numerical Recipes `erfc` approximation (error
< 1.2 × 10⁻⁷), Φ⁻¹ is Acklam's algorithm plus a Halley step.

**Monte Carlo.** People are drawn as `x = μ + σ ⊙ (L z)` with `L` the Cholesky
factor of the correlation matrix (ANSUR II: head breadth–IPD 0.31 men / 0.28
women) and a seeded generator, so the metric is reproducible.

**Fit rules.** (1) pupils centred: `|A + DBL − IPD| / 2 ≤ 4 mm` per eye;
(2) temple length: `bend − E ∈ [−2, +12] mm`, where the bend sits
`T − clamp(0.25 T, 25, 45)` behind the hinge pin (as the CAD builds it);
(3) frame width: clamp force inside the comfort band; (4, optional) bridge vs
nose. Sizes are searched over 2,700 candidates (lens 42–60, bridge 14–24,
temple 135–155 mm, splay 0–16°) with bitset coverage, greedy selection and
exchange passes.

**Support model.** Rigid frame on four equally stiff supports (two pads, two
ear roots): `Rᵢ = W/n + W (y_c − ȳ)(yᵢ − ȳ)/Σ(y − ȳ)² + W (x_c − x̄)(xᵢ − x̄)/Σ(x − x̄)²`,
which reduces to the lever rule `N_nose = W (y_c − y_e)/(y_n − y_e)` front to
back. Pad normal force `N = V / sin β` on a flank tilted β from the vertical;
pressure `p = N / A` against ≈ 32 mmHg = 4.3 kPa (Landis 1930). Looking down
by α: `μ (ΣN + 2F) ≥ W sin α` gives the smallest clamp force that holds.

**Clamp.** Temple compliance `c = ∫₀^{L_c} (L_c − s)²/(E I(s)) ds` over the
knuckle, the hollow pod `I = (h w³ − h_i w_i³)/12` and the arm
`I = h w³/12`; front hinge rotation `θ = M Σ ℓ/(EI)` with `M = F L_c`;
`F = max(0, H − S₀)/(L_c² Σ ℓ/EI + c_R + c_L)`, `S₀ = 2 x_in + 2 L_c tan(splay)`.
Band: `F_min` from friction, `F_max = p_lim × contact length × temple height`.

**Display.** `θ_text = h_px · θ_px` vs 5′ × (x/20) acuity and 16–22′ comfort
(ANSI/HFES 100-2007); `ACR = (L_img + T L_bg)/(T L_bg)`,
`L_img = L_display · T_lens · R`, `T = (1 − R) · T_tint`; eyebox coverage
`Σ_g w_g [Φ((2(x_d + e/2) − μ_g)/σ_g) − Φ((2(x_d − e/2) − μ_g)/σ_g)]` with
`x_d = (A + DBL)/2 − offset` and `e` the optics lab's eyebox (pupil centre sees
the whole field).

## Verification (unit tests)

`cargo test -p forge-core ergonomics` — 22 tests, including: Φ against table
values (Φ(1.96) = 0.9750021, Φ(1) = 0.8413447); Φ⁻¹ round trips; mixture of
N(0,1) and N(2,1) has SD √2; Cholesky of a 2 × 2 correlation; P95 head
breadth = 154.33 + 1.645·5.53 = 163.43 mm, within 1 mm of the ANSUR II
empirical 163 mm; sampled means and correlation; lever rule (40 g, CoM 25 mm
behind pads on a 100 mm base → 30 gf on the nose); bolt-group roll split
(ear/pad difference ratio = 10 for 70 vs 7 mm); pad pressure and the slip
equation; uniform cantilever `L³/3EI` (4 × 8 mm PETG, 100 mm → 3.906 mm/N,
5 mm needs 1.28 N); stepped cantilever equals uniform, rigid root leaves
`(L − a)³/3EI`; box section by hand (938.0 mm⁴); ACR (828 nits over
0.7 × 5,000 → 1.2366; with a 30 % tint 1.789); eyebox coverage by hand
(N(64, 3.4), window 62–74 mm → 0.720; centred ±6 mm → 0.922); Iris lever rule
and mass total = CAD frame + components; clamp linearity and splay relief; all
API methods; three sizes beat one.

## Iris tie-in and metrics

| Metric | Iris value | Meaning |
|---|---|---|
| `ergonomics.fit_coverage_pct` | **0.0 %** (1 of 4,000) | one size fits all three rules; PD rule 79 %, temple 71 %, clamp band 0 % — **100 % too tight** |
| `ergonomics.nose_load_g` | **38.3 gf** | 48.5 g total, CoM 30 mm behind the front, nose carries 79 % |
| `ergonomics.clamp_force_n` | **4.55 N** per side | at the P95 combined head breadth (162 mm); band 0.18–1.38 N |
| `ergonomics.eyebox_coverage_pct` | **59.8 %** | 6.0 mm eyebox centred 34 mm from the mid-line accepts IPD 62–74 mm |
| `ergonomics.outdoor_contrast` | **1.24 : 1** | 828 nits of image over a 5,000 cd/m² sunlit street at 70 % see-through |
| `ergonomics.nose_pad_pressure_kpa` | 19.3 kPa | 0.38 N on 20 mm² pads vs ≈ 4.3 kPa |
| `ergonomics.caption_arcmin` | 22.4′ | 10 px caption × 2.24′/px — comfortable |

**What the simulations caught.** The Iris CAD temples run straight back from
hinges 122 mm apart, so every adult head pushes each temple out ≈ 9–20 mm
(P5–P95): the clamp is 2–5 N per side, far above the sustained-pressure limit. A 6.4° splay
centres the comfort band on the median head (58 % inside the band, 35 % fit on
all rules), but the band is only ≈ 10 mm of head breadth wide: one size covering
P5–P95 would need the frame to be ≈ 17 mm/N compliant, twice Iris's 8.7 mm/N
(spring hinges, a thinner front). Three
sizes with their own splay cover 77.5 % of adults on all rules. The nose
carries 38 gf on 20 mm² pads — 4.5 × the capillary limit; pads of ≈ 90 mm²
each fix the pressure, and moving the battery behind the ear trims the load.
The display axis sits at the lens-box centre (34 mm) while the median
monocular PD is ≈ 31.4 mm: shifting it 2.6 mm towards the nose raises eyebox
coverage from 60 % to 90 %. In sunlight the caption washes out (1.24 : 1);
reaching 1.5 : 1 needs a tint transmitting ≤ 47 % or ≈ 6,300 nits of display
luminance.

**Recommended Iris requirements** (shown live on the How it works scorecard;
not yet in `project.json`):

| Metric | Target | Iris v0.5 | Fix found |
|---|---|---|---|
| `ergonomics.fit_coverage_pct` | ≥ 30 % | 0.03 % ✕ | 6.4° temple splay → 35 %; S/M/L → 78 % |
| `ergonomics.nose_pad_pressure_kpa` | ≤ 4.3 kPa | 19.3 kPa ✕ | ≥ 89 mm² pads → 4.2 kPa |
| `ergonomics.eyebox_coverage_pct` | ≥ 85 % | 59.8 % ✕ | display axis 2.6 mm towards the nose → 89.6 % |
| `ergonomics.outdoor_contrast` | ≥ 1.5 : 1 | 1.24 : 1 ✕ | tint ≤ 47 % transmission, or ≈ 6,300 nits |
| `ergonomics.clamp_force_n` (optional) | ≤ 1.4 N | 4.55 N ✕ | sizes, or ≈ 2 × frame compliance |
| `ergonomics.caption_arcmin` (optional) | ≥ 16′ | 22.4′ ✓ | — |

The lab's settings live in `design.extra.ergonomics` (defaults:
`female_fraction 0.5, temple_splay_deg 0, display_offset_mm 0,
tint_transmission 1, caption_cap_px 10, acuity_20_over 20,
pd_tolerance_mm 4, ear_window_mm [−2, 12], friction_mu 0.3,
look_down_deg 40, nose_flank_deg 30, pad_area_mm2 0 (CAD),
temple_contact_length_mm 40, pressure_limit_kpa 4.3,
contact_before_bend_mm 10, bright_scene_nits 5000, samples 4000, seed 7`).

## Limitations

* Normal approximations miss skew and the far tails; ANSUR II is a US military
  sample (younger and fitter than the public).
* **Hinge-to-ear distance and nose bridge width are engineering assumptions**,
  labelled as such; replace them with measured data (`overrides`).
* Head breadth (maximum breadth, above the ears) stands in for the width the
  temples span — clamp forces are upper bounds. Beams are linear; bonded
  lenses, spring hinges and hinge play are not modelled.
* The comfort band's ends (friction μ, look-down angle, pressure limit,
  contact area) are typical values; only a wear test settles them.
* The support model is quasi-static with equally stiff tissue; the moment of
  the slide force is neglected.
* Legibility ignores contrast-dependent acuity and display colour; the eyebox
  is horizontal only; the 3-D head is a procedural teaching aid, not a head
  form.

## Graduate to

* The **ANSUR II** raw data (and other anthropometric surveys) for any other
  dimension; 3-D head-scan sets and standard headforms for real fit work.
* Digital human models such as Siemens Jack or Human Solutions RAMSIS.
* Pressure-mapping film or a load cell under a nose pad, plus a structured
  comfort questionnaire over hours of wear.

## References

* C. C. Gordon et al., *2012 Anthropometric Survey of U.S. Army Personnel:
  Methods and Summary Statistics*, NATICK/TR-15/007 (2014) — ANSUR II.
* N. A. Dodgson, "Variation and extrema of human interpupillary distance",
  Proc. SPIE 5291 (2004).
* S. Pheasant & C. M. Haslegrave, *Bodyspace: Anthropometry, Ergonomics and
  the Design of Work*, 3rd ed., CRC Press (2006).
* ISO 7250-1, *Basic human body measurements for technological design —
  Part 1: Body measurement definitions and landmarks*.
* ISO 8624, *Ophthalmic optics — Spectacle frames — Measuring system and
  vocabulary*.
* M. S. Sanders & E. J. McCormick, *Human Factors in Engineering and Design*,
  7th ed., McGraw-Hill (1993).
* C. D. Wickens, J. G. Hollands, S. Banbury & R. Parasuraman, *Engineering
  Psychology and Human Performance*, 4th ed., Pearson (2013).
* B. Kress & T. Starner, "A review of head-mounted displays (HMD)
  technologies and applications for consumer electronics", Proc. SPIE 8720
  (2013).
* B. C. Kress, *Optical Architectures for Augmented-, Virtual-, and
  Mixed-Reality Headsets*, SPIE Press (2020).
* E. M. Landis, "Micro-injection studies of capillary blood pressure in human
  skin", Heart 15, 209–228 (1930).
* S. Derler & L.-C. Gerhardt, "Tribology of skin: review and analysis of
  experimental results for the friction coefficient of human skin", Tribology
  Letters 45, 1–27 (2012).
* G. S. Daniels, "The 'Average Man'?", Technical Note WCRD 53-7, Wright Air
  Development Center (1952).
* ANSI/HFES 100-2007, *Human Factors Engineering of Computer Workstations*.
* W. C. Young & R. G. Budynas, *Roark's Formulas for Stress and Strain*, 7th
  ed., McGraw-Hill (2002).
* ANSUR II public data files, as mirrored in
  [AllenDowney/ProbablyOverthinkingIt](https://github.com/AllenDowney/ProbablyOverthinkingIt)
  (`data/ANSURIIMALEPublic.csv`, `data/ANSURIIFEMALEPublic.csv`) — the source of
  the statistics computed here.
