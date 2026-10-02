# RF & Antenna Lab (`#/rf`, engine module `rf`)

Will the glasses talk to the phone? The RF & Antenna Lab covers the
radio-frequency (RF) side of the **Iris** Bluetooth Low Energy (BLE) link at
2.4 GHz, in the browser: a finite-difference time-domain (FDTD) field solver
with a perfectly matched layer (PML) shows waves leaving an antenna, soaking
into a head and reflecting from metal; analytic antenna patterns and an array
factor give directivity and efficiency; an interactive Smith chart synthesises
the matching network the Iris board is missing, snapped to standard values
with real component Q; transmission-line calculators size the 50 Ω antenna
feed as a grounded coplanar waveguide (CPWG) on the real 0.8 mm board; and a
link budget checks the margin to a phone with head and body loss. The field
solver runs in the lab's own Web Worker, so the page never blocks.

## What it does

| Tab | What you get |
|---|---|
| Fields (FDTD) | A 2-D transverse-magnetic (TMz) Yee solver with a Bérenger split-field PML. Presets: **dipole in free space** (a line source — the H-plane cut of a vertical dipole), **phased array** (N sources, spacing, phase step), **antenna next to a head** (tissue phantom, εr 39.2, σ 1.80 S/m), **ground plane + monopole** (image theory), **slit diffraction**, **waveguide** (parallel plates with cutoff). Animated E_z with a diverging colour map (play/pause, step, scrub, speed), click to place the source or the probe, continuous wave or Gaussian pulse, coarse/normal/fine grids, Courant-number slider (try S > 1 to watch the instability), probe trace and field-energy plots, the power absorbed by tissue (time-averaged Poynting balance). Downloads: probe CSV, energy CSV, frame PNG, run settings JSON. |
| Antennas | First-cut resonant lengths (half-wave dipole, quarter-wave monopole, printed dipole, inverted-F antenna (IFA), meandered IFA) with effective permittivity; the Chu–McLean small-antenna limit for the chip alone and for the whole Iris board; the Iris efficiency chain (radiation efficiency, match loss from the Matching tab, head loss). Dipole (short, half-wave, any length) and uniform-array patterns on E-plane and H-plane polar plots with hover read-out, a 3-D pattern in the shared 3-D viewer, numerical directivity, half-power beamwidths, realised gain and an animated beam-steering sweep. Download: pattern cuts CSV. |
| Matching | An SVG Smith chart with impedance (and optional admittance) grids, a 10 dB return-loss circle and a **draggable load point**; the assumed chip-antenna impedance model (resistance, reactance at band centre and Q — clearly labelled *assumed; measure with a vector network analyser (VNA)*); L-network synthesis (every valid solution, usually two) or a pi-network with a chosen loaded Q; snapping to E12/E24 values (all floor/ceil combinations tried) with inductor and capacitor Q; the element arcs and the BLE-band trace drawn on the chart; return loss vs frequency over 2.2–2.7 GHz with the band shaded; a ladder schematic per solution; "Use on Iris" writes the network into the project; the bill-of-materials (BOM) rows to add. Downloads: Touchstone `.s1p`, sweep CSV, BOM rows CSV. |
| Transmission lines | Microstrip (Hammerstad–Jensen with strip thickness), CPWG (conformal mapping) and an edge-coupled differential pair (USB 90 Ω, IPC-2141 approximation): analysis or width synthesis, effective permittivity, guided wavelength, electrical length, delay, conductor and dielectric loss, impedance vs width, and a cross-section drawing with an E- and H-field sketch. The Iris stack-up toggle uses the board thickness and copper; the Iris card compares the feed as routed today with the recommended 50 Ω CPWG and can store it on the project. Download: impedance-vs-width CSV. |
| Link budget | BLE LE 1M / 2M / Coded S=2 / Coded S=8: transmit power, antenna gains (the glasses' gain and match loss come from the other tabs), Friis free-space and log-distance path loss, head and body loss, a Rayleigh fade margin from the availability, typical receiver sensitivities; both directions (glasses → phone, phone → glasses). An animated waterfall bar chart, range per PHY, a top-down range map with BLE packet rings and a draggable phone, received power vs distance. "Save these assumptions to Iris" stores them on the project. Download: power-vs-distance CSV. |
| How it works | The pipeline from Maxwell's equations to a working link, an animated standing-wave demo (forward + reflected wave, envelope, VSWR), tissue at 2.4 GHz, units, the Iris RF chain today vs the lab's recommendation, next steps for the real board, tools to graduate to and references. |

## Engine API

| Method | Input | Output |
|---|---|---|
| `rf.iris` | `{project}` | the effective `design.extra.rf` (stored values + defaults), current and recommended matching networks with band summaries, current (as routed or stored) and recommended feed with Z₀, glasses gain and match loss, the link budget, BOM rows to add, the metrics, `suggested_extra` (the design with the recommendations applied) and notes |
| `rf.fdtd` | `{preset: dipole \| array \| head \| ground \| slit \| waveguide, nx, ny, dx_mm, f_ghz, steps, frames, courant, pml, source?, probe?, pulse, tissue_er, tissue_sigma, head_radius_mm, gap_mm, slit_mm, guide_mm, height_mm, array_n, array_spacing_wl, array_phase_deg, gamma}` | grid and time step, Courant limit, cells per wavelength, `frames_b64` (`n_frames × nx × ny` bytes, 128 = zero), frame times, `material_b64` (0 air, 1 tissue, 2 metal) and legend, scale, sources, probe position and trace, field energy per frame, absorbed fraction and dB (CW), notes |
| `rf.pattern` | `{element: isotropic \| short \| half_wave \| dipole, length_wl, n, spacing_wl, phase_deg, rad_efficiency, mismatch_db, body_loss_db, mesh_step_deg, mesh_range_db}` | directivity (linear, dBi), realised gain, total efficiency, beam direction, half-power beamwidths, E- and H-plane cuts (dB), 3-D mesh (positions, indices, level) |
| `rf.antenna_sizes` | `{f_ghz, er, ground_len_mm, ground_w_mm, chip_len_mm}` | λ₀, resonant-length rows with notes, ground length in λ, Chu–McLean Q of the chip and of the board |
| `rf.match` | `{antenna: {r_ohm, x_ohm, q, fc_ghz}, topology: l \| pi, pi_q, series: E12 \| E24, q_l, q_c, z0, plot_lo_ghz, plot_hi_ghz, current?}` | load Z and Γ, series resonance, bare-antenna summary and sweep, solutions (ideal and snapped elements, labels, band summaries, wide and in-band sweeps, Smith-chart arcs, loaded Q, Touchstone text), the recommended index, the stored network evaluated, notes |
| `rf.line` | `{kind: microstrip \| cpwg \| diff_microstrip, width_mm, gap_mm, h_mm, t_um, er, tan_d, f_ghz, length_mm, target_ohm?, iris_stack, project?}` | width (synthesised if a target is given), Z₀ and differential Z, εeff, guided wavelength, electrical length, delay, conductor and dielectric loss, microstrip comparison, Z vs width sweep, notes |
| `rf.link` | `{phy, f_ghz, distance_m, path_exponent, tx_dbm, glasses_gain_dbi, match_loss_db, glasses_sens_offset_db, phone_tx_dbm, phone_gain_dbi, phone_sens_1m_dbm, head_loss_db, body_loss_db, other_loss_db, availability}` | path loss, Friis loss at 1 m, fade margin, both directions as waterfall steps with received power, sensitivity and margin, the limiting direction, range, range and margin per PHY, received power vs distance, notes |
| `rf.phys` | – | the four LE PHYs with rate and typical sensitivity |

## The maths

**FDTD** (Yee 1966; Taflove & Hagness). TMz fields E_z, H_x, H_y on a
staggered grid, leapfrogged in time:
`H_x ← H_x − Δt/(μ₀Δ)(E_z[j+1] − E_z[j])`,
`H_y ← H_y + Δt/(μ₀Δ)(E_z[i+1] − E_z[i])`,
`E_z ← C_a E_z + C_b[(H_y[i] − H_y[i−1]) − (H_x[j] − H_x[j−1])]` with
`C_a = (1 − σΔt/2ε)/(1 + σΔt/2ε)`, `C_b = (Δt/εΔ)/(1 + σΔt/2ε)`. Stability:
`cΔt√(1/Δx² + 1/Δy²) ≤ 1`, so `Δt ≤ Δ/(c√2)` (3.54 ps at 1.5 mm); the lab
runs at S = 0.99. **PML** (Bérenger 1994): `E_z = E_zx + E_zy`, each damped
by the conductivity normal to it, `σ* = σμ₀/ε₀`, graded
`σ(ρ) = σ_max(ρ/N)³` with `σ_max = −(m+1)ln R₀/(2η₀NΔ)`, R₀ = 10⁻⁶, 12
cells, exponential time differencing. Sources are soft (additive) line
currents with a 3-period raised-cosine turn-on (CW) or a Gaussian pulse.
Field energy `W = Σ ½(εE_z² + μH²)Δ²`; absorbed power `Σ σE_z²Δ²`; outflow is
the Poynting flux `S = (−E_zH_y, E_zH_x)` through a contour inside the PML,
both averaged over the last two periods. Frames are scaled by the largest
|E_z| away from the source cells, compressed with a gamma of 0.5 and
quantised to bytes.

**Antennas** (Balanis ch. 4, 6). Dipole field
`[cos(kL/2·cosθ) − cos(kL/2)]/sinθ`; array factor
`sin(Nψ/2)/(N sin(ψ/2))`, `ψ = kd·sinθcosφ + β`; directivity
`D = 4πU_max/∮U dΩ` on a 1° grid (checks: short dipole 1.5, half-wave 1.643,
8-element λ/2 broadside isotropic array ≈ 8). Realised gain
`D + 10 log₁₀(η_rad(1 − |Γ|²)η_body)`. Chu–McLean `Q ≥ 1/(ka)³ + 1/(ka)`.
Printed antennas use `εeff ≈ (εr + 1)/2` as a first cut.

**Matching** (Pozar §5.1; Bowick ch. 4). `Γ = (Z − Z₀)/(Z + Z₀)`,
`RL = −20 log|Γ|`, `VSWR = (1 + |Γ|)/(1 − |Γ|)`. L-network for `R_L > Z₀`:
`B = (X_L ± √(R_L/Z₀)√(R_L² + X_L² − Z₀R_L))/(R_L² + X_L²)`,
`X = 1/B + X_LZ₀/R_L − Z₀/(BR_L)`; for `R_L < Z₀`:
`X = ±√(R_L(Z₀ − R_L)) − X_L`, `B = ±√((Z₀ − R_L)/R_L)/Z₀`. Pi network:
`R_v = R_high/(Q² + 1)`, two back-to-back L sections with
`Q_s = √(Z₀/R_v − 1)`, `Q_L = √(R_p/R_v − 1)`, load susceptance absorbed.
Components carry ESR `X/Q`; the network is evaluated with ABCD matrices and
the transducer gain `G_T = 4Z₀Re(Z_L)/|V_s|²` (unit load current). Antenna
model: `Z(f) = R(1 + jQ(f/f₀ − f₀/f))`, with f₀ chosen so Z(f_c) = R + jX.

**Transmission lines** (Hammerstad & Jensen 1980; Wadell 1991; Pozar §3.8).
Microstrip as in the Transmission lines tab, with the thickness correction
`Δu₁ = (t/πh)ln(1 + 4e/(t/h·coth²√(6.517u)))`; CPWG
`Z₀ = 60π/√εeff · 1/(K(k)/K(k′) + K(k₃)/K(k₃′))`, `k = w/(w + 2s)`,
`k₃ = tanh(πw/4h)/tanh(π(w + 2s)/4h)`, K by the arithmetic–geometric mean;
differential `Z_diff ≈ 2Z₀(1 − 0.48e^(−0.96s/h))` (IPC-2141 approximation,
±10 %). Losses `α_c ≈ R_s/(Z₀w)`, `α_d = k₀εr(εeff − 1)tanδ/(2√εeff(εr − 1))`.
Synthesis by bisection on log width.

**Link budget** (Friis 1946; Rappaport §4.9). `FSPL = 20 log₁₀(4πd/λ)`
(40.19 dB at 1 m, 2.44 GHz); `PL(d) = FSPL(1 m) + 10n log₁₀ d` beyond 1 m;
`P_rx = P_tx + G_tx + G_rx − PL − L_match − L_head − L_body`; Rayleigh fade
margin `F = −10 log₁₀(−ln(1 − p_out))` (9.77 dB at 90 %, 19.98 dB at 99 %);
margin `P_rx − S − F`; range solves `margin = 0`.

## Iris tie-in and metrics

The lab reads the board from `design.pcb` (58 × 10 mm, 2 layers, 0.8 mm →
0.73 mm of FR-4 between 35 µm copper layers) and its RF choices from
`design.extra.rf` (absent on Iris v0.5; defaults below). Every value is
computed; the metric pass takes well under a millisecond and is memoised.

| Key | Meaning | Iris v0.5 | With the lab's recommendations |
|---|---|---|---|
| `rf.match_return_loss_db` | worst return loss over 2.400–2.4835 GHz at the 50 Ω port | **3.9 dB** (no matching network — the BOM gap PCB Studio also flags; VSWR 4.5) | **14.1 dB** with series 1.8 nH + shunt 4.3 nH (E24, Q_L 40) |
| `rf.feed_z0_ohm` | Z₀ of the antenna feed on the Iris stack-up | **94.8 Ω** — PCB Studio routes the RF net at the 0.15 mm minimum width with 0.20 mm pour clearance | **49.9 Ω** — CPWG 0.91 mm wide, 0.20 mm gaps |
| `rf.link_margin_db` | LE 1M margin to a phone at 5 m, n = 2.5, head 6 dB + body 4 dB, 90 % Rayleigh fade margin, worse direction | **7.1 dB** (2.2 dB match loss) | **8.9 dB** |
| `rf.range_m` | distance at 0 dB margin | 9.6 m | 11.4 m |
| `rf.antenna_efficiency_pct` | radiation efficiency × worst in-band transducer gain | 30 % | 46 % |

The antenna impedance is an **assumption** — R = 18 Ω, X = −30 Ω at
2.442 GHz, Q = 12 — typical of a chip antenna detuned by a ground plane
smaller than the vendor's reference board. It must be replaced by a VNA
measurement of the assembled glasses (ideally worn or on a head phantom);
the Matching tab re-synthesises from whatever you type in. The radiation
efficiency (50 %), the phone's transmit power (0 dBm), gain (−2 dBi) and
sensitivity (−90 dBm on LE 1M), the head (6 dB) and body (4 dB) losses and the
path-loss exponent (2.5) are labelled assumptions too; the SoC sensitivities
are typical data-sheet values.

Default `design.extra.rf` (what the lab assumes when it is absent):

```json
{
  "antenna": { "r_ohm": 18, "x_ohm": -30, "q": 12, "fc_ghz": 2.44175 },
  "rad_efficiency": 0.5, "directivity_dbi": 1.76,
  "network": null, "feed": null,
  "er": 4.4, "tan_d": 0.02, "cu_um": 35, "q_l": 40, "q_c": 150, "series": "E24",
  "link": { "phy": "1m", "distance_m": 5, "path_exponent": 2.5, "tx_dbm": 0, "phone_tx_dbm": 0,
            "phone_gain_dbi": -2, "phone_sens_1m_dbm": -90, "head_loss_db": 6, "body_loss_db": 4, "availability": 0.9 }
}
```

The recommended values to store (`suggested_extra` from `rf.iris`) add
`"network": [{"kind":"series_l","value":1.8e-9,"q":40},{"kind":"shunt_l","value":4.3e-9,"q":40}]`
(listed from the radio toward the antenna) and
`"feed": {"kind":"cpwg","width_mm":0.91,"gap_mm":0.2,"length_mm":6}`.

## Limitations

* The FDTD solver is 2-D: sources are infinite line currents, metal is an
  infinite sheet seen edge-on, fields fall as 1/√r rather than 1/r. It
  teaches wave behaviour and gives a rough absorbed fraction for a head; it
  does not predict the 3-D input impedance or pattern of the chip antenna.
  Use openEMS or Meep in 3-D for that.
* The head phantom is homogeneous (no skin, fat, bone layers) and the tissue
  is non-dispersive (one frequency); the grid has about 13 cells per
  wavelength inside tissue at 1.5 mm, so expect a few per cent of
  numerical dispersion there.
* The antenna is a one-resonance series RLC model of an *assumed* impedance;
  real chip antennas have more structure across frequency and shift with the
  frame, the hand and the head.
* Component Q is one number per part; real 0402 inductors have
  self-resonance and frequency-dependent Q, and pads add parasitics. Check
  the vendor's S-parameter files.
* Microstrip and CPWG formulas are quasi-static (dispersion neglected; fine
  at 2.4 GHz on 0.73 mm), copper is smooth, CPWG assumes wide via-stitched
  top grounds; the differential-pair formula is a ±10 % approximation.
* The link budget uses typical sensitivities and assumed phone parameters;
  body and head losses vary strongly with posture and direction. Measure
  range with the real phone.

## Graduate to

openEMS (3-D FDTD with MATLAB/Octave/Python scripting — model the real chip
antenna, board and head), Meep (MIT's FDTD package with dispersive materials
and near-to-far-field transforms), scikit-rf (Touchstone files, VNA data,
de-embedding, Smith charts and matching in Python — load the `.s1p` exported
here), Qucs-S (S-parameter circuit simulation with transmission-line models —
put the matching network, the CPWG feed and a measured antenna file in one
schematic). No single open tool does this lab's end-to-end chain tied to a
product file and its requirements; the closest are scikit-rf for matching and
openEMS for fields, and the improvement here is that every number flows into
Iris verification in the browser.

## References

* D. M. Pozar, *Microwave Engineering*, 4th ed., Wiley, 2012.
* C. A. Balanis, *Antenna Theory: Analysis and Design*, 4th ed., Wiley, 2016.
* A. Taflove & S. C. Hagness, *Computational Electrodynamics: The Finite-Difference Time-Domain Method*, 3rd ed., Artech House, 2005.
* K. S. Yee, "Numerical solution of initial boundary value problems involving Maxwell's equations in isotropic media", *IEEE Trans. Antennas Propag.* 14(3), 302–307, 1966.
* J.-P. Bérenger, "A perfectly matched layer for the absorption of electromagnetic waves", *J. Comput. Phys.* 114(2), 185–200, 1994.
* E. Hammerstad & Ø. Jensen, "Accurate models for microstrip computer-aided design", *IEEE MTT-S International Microwave Symposium Digest*, 407–409, 1980.
* B. C. Wadell, *Transmission Line Design Handbook*, Artech House, 1991.
* Bluetooth SIG, *Bluetooth Core Specification* v5.x, Vol 6 (Low Energy Controller).
* Texas Instruments application notes on small 2.4 GHz antennas (e.g. AN043 "Small size 2.4 GHz PCB antenna", AN058 "Antenna selection guide").
* H. T. Friis, "A note on a simple transmission formula", *Proc. IRE* 34(5), 254–256, 1946.
* L. J. Chu, "Physical limitations of omni-directional antennas", *J. Appl. Phys.* 19, 1163–1175, 1948.
* J. S. McLean, "A re-examination of the fundamental limits on the radiation Q of electrically small antennas", *IEEE Trans. Antennas Propag.* 44(5), 672–676, 1996.
* T. S. Rappaport, *Wireless Communications: Principles and Practice*, 2nd ed., Prentice Hall, 2002.
* C. Bowick, *RF Circuit Design*, 2nd ed., Newnes, 2008.
* P. H. Smith, "Transmission line calculator", *Electronics* 12, 29–31, 1939.
* S. Gabriel, R. W. Lau & C. Gabriel, "The dielectric properties of biological tissues: III", *Phys. Med. Biol.* 41, 2271–2293, 1996.
* IEEE Std 1528 / IEC/IEEE 62209-1528 — head tissue-simulating liquid targets.
* IBIS Open Forum, *Touchstone File Format Specification* v2.0, 2009.
* Repositories: [openEMS](https://github.com/thliebig/openEMS), [Meep](https://github.com/NanoComp/meep), [scikit-rf](https://github.com/scikit-rf/scikit-rf), [Qucs-S](https://github.com/ra3xdh/qucs_s).
