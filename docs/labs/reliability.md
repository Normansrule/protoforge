# Reliability & Test Lab (`#/reliability`, engine module `reliability`)

Will it last, and how do you prove it? The Reliability & Test Lab is the
workbench a systems engineer uses between design and production: fit failure
data on Weibull paper, turn years of use into weeks of accelerated life
testing (ALT), size demonstration tests, build the product's reliability
block diagram (RBD) from its bill of materials (BOM), and generate the design
verification test (DVT) plan — legs, sample sizes, environmental test
families, schedule, requirement matrix — ready to export. Every number is
computed by the Rust engine (`crates/forge-core/src/reliability`) in
WebAssembly, and the System and Test-plan tabs read the **Iris smart
glasses** project directly, so their metrics move whenever the design does.

## What it does

| Tab | What you get |
|---|---|
| Weibull analysis | Paste, type or upload (CSV) failure times with suspensions (`123+`, `123,S`, `123,0`, a count column); five sample sets (Iris hinge cycling, LiPo capacity fade, hinge flex-cable fold test, early-failure returns — all labelled synthetic — and the published Lieblein–Zelen ball-bearing data). Johnson adjusted ranks and Bernard median ranks; fits by rank regression on X (RRX) and on Y (RRY), maximum likelihood (MLE) and Weibayes (β assumed), side by side; **Weibull probability paper** (custom SVG with log time and ln(−ln(1−F)) axes, suspension ticks, B10 marker, hover read-out of F and R) with **Fisher-matrix or likelihood-ratio** confidence bounds at a chosen two-sided level; an animated "build the plot" walk-through, point by point; β and η bounds, B1/B10/B50 with bounds, mean life, a plain-language reading of β; pdf, hazard and reliability curves; a ranks table; an **estimator Monte Carlo study** (how RRX, RRY and MLE scatter and bias β for small, censored samples); an animated **bathtub curve** with a population of units failing by mode; "use this fit for an Iris block" (writes β, η into the System tab). Downloads: fit JSON, ranks CSV, data. |
| Accelerated life | Acceleration factors for **Arrhenius**, **Peck**, **Coffin–Manson**, **Norris–Landzberg** and the **inverse power law**, with the factor breakdown, an AF-vs-stress sweep and presets tied to Iris (its board temperature in use, worn damp-heat conditions, tin–silver–copper and tin–lead solder constants). A **test planner**: units for a given test length, test length for a given number of units, with allowed failures and an assumed β (Weibayes/Lipson), against the Iris field life. An **animated climatic chamber** (temperature/humidity set-point, units on shelves, a moving cursor on the profile) for change-of-temperature, damp-heat, dry-heat and cold profiles. Downloads: plan CSV, profile CSV. |
| Sample size | Success-run theorem, binomial with 0–3 allowed failures, Lipson/Weibayes extension (units vs test length for β = 1, 1.5, 2, 3), success-run curves for C = 80–99 %, the **operating-characteristic (OC) curve** of a binomial plan and of the equivalent Lipson plan with consumer's and producer's risk, the reliability a plan demonstrates, and an animated "run the test many times" demonstration whose pass rate converges on the OC curve. Download: table CSV. |
| System reliability | The **Iris RBD** built from the BOM: subsystems in series, parts in series inside, microphones optionally 1-of-2; typical failure rates in failures in time (FIT) — **typical, replace with supplier data** — scaled to the board temperature in use; the battery (capacity fade), spring hinge and hinge flex cable as Weibull wear-out items driven by the use profile. R(t), parts-count MTBF, MTTF, B10, a 20 000-unit **Monte Carlo check** with a 95 % interval, **Birnbaum and criticality importance** and Monte Carlo first-failure shares; an **animated block diagram** that replays simulated units (blocks fail with ✕ at their sampled times, the signal path breaks, the cause is named); a per-block editor (FIT or β/η) and use-profile controls saved to `design.extra.reliability`. Downloads: RBD JSON, blocks CSV. |
| Test plan (DVT) | Generated from the project: legs (baseline, mechanical robustness, climatic life, wear-out life, qualification checks, bare-cell cycling) with sample sizes from the target reliability and confidence; 19 tests with standard designations, typical severities, procedure outlines, equipment, pass criteria and links to requirements, parts and risks; test durations computed from the use profile and acceleration models; a **resource-constrained Gantt / chamber schedule** (by resource or by test, animated campaign playback); the **requirement verification matrix** (every requirement → how DVT confirms it physically, units and the pass criterion from the requirement). Downloads: plan CSV, plan Markdown, matrix CSV. |
| How it works | Vocabulary (R, F, f, h, MTTF, MTBF, FIT), an animated "37 % survive to the MTBF" demonstration, Weibull, censoring, confidence, acceleration, demonstration testing, system reliability, the loop from analysis to test plan, tools to graduate to, references. |

## Engine API

| Method | Input | Output |
|---|---|---|
| `reliability.weibull_fit` | `{text \| data[{t, failed}], method: mle \| rrx \| rry \| weibayes, bounds: fisher \| lr \| none, cl, beta_assumed, unit}` | ranks table, all four fits (β, η, ρ², log-likelihood, MTTF, B1/B10/B50), selected β/η with bounds, B-lives with bounds, fit line and bounds on a grid of F, pdf/cdf/R/hazard curves, interpretation, notes |
| `reliability.weibull_sets` | – | sample data sets (id, title, unit, source, description, text) |
| `reliability.weibull_generate` | `{beta, eta, n, censor_at?, seed}` | observations |
| `reliability.weibull_study` | `{beta, eta, n, fail_fraction, reps, seed}` | per method: all β̂ and η̂, median, mean, 5/95 % points, median B10; an example data set |
| `reliability.bathtub` | `{infant_beta, infant_eta, random_eta, wear_beta, wear_eta, horizon, units, seed}` | hazards per mode and total, R(t), simulated unit failure times and modes |
| `reliability.alt` | `{model: arrhenius \| peck \| coffin_manson \| norris_landzberg \| ipl, ea_ev, t_use_c, t_test_c, rh_use, rh_test, rh_exp, dt_use, dt_test, cm_exp, f_use, f_test, freq_exp, nl_c, tmax_use_c, tmax_test_c, s_use, s_test, ipl_exp}` | AF, factor breakdown with formulas, AF sweep, equivalence sentence, notes |
| `reliability.alt_plan` | `{af, life, reliability, confidence, beta, failures, units, test_duration, per_day}` | required Σ k^β, duration for the units, units for the duration, success-run units, table units → duration/days/lives |
| `reliability.chamber_profile` | `{kind: thermal_cycle \| damp_heat \| dry_heat \| cold, t_low_c, t_high_c, ramp_c_per_min, dwell_min, cycles, rh, hold_h, ambient_c}` | set-point time series, segments, cycle time, cycles per day |
| `reliability.sample_size` | `{reliability, confidence, max_failures, beta, multiple, plan_n, plan_failures, design_reliability}` | success-run n, binomial and Lipson n per allowed failures, curves, OC curves, risks, demonstrated R |
| `reliability.rbd` | `{root: Node, mission_h, samples, seed, horizon_h}` (Node = `{id, label, kind: block \| series \| parallel \| kofn, k, children, model: exp \| weibull, fit, beta, eta, life_unit, per_hour, qty}`) | R at the mission, random-only R, MTTF, parts-count MTBF, ΣFIT, B10, R(t) curves, per-block R/Birnbaum/criticality/first-failure share, groups, Monte Carlo (R with Wilson interval, MTTF, empirical curve, 8 sample units) |
| `reliability.iris` | `{project?, samples, seed, years?}` | the lab configuration, the use profile, the Iris RBD tree, its evaluation, excluded parts, R over 2 years |
| `reliability.dvt` | `{project?, reliability?, confidence?, failures?, life_multiple?, life_beta?, qual_units?}` | legs, tests (with schedule), requirement matrix, units, durations, resources, CSV, Markdown, notes |
| `reliability.config_defaults` | – | default `design.extra.reliability` |

## The maths

**Ranks.** Sort the data (failures before suspensions at equal times). For
each failure with reverse rank RR (units still on test, counting it),
Johnson's adjusted rank is `rank = (RR · rank_prev + N + 1)/(RR + 1)`;
Bernard's approximation gives the median rank `F ≈ (rank − 0.3)/(N + 0.4)`.

**Weibull paper.** `F(t) = 1 − exp[−(t/η)^β]` linearises to
`ln(−ln(1 − F)) = β ln t − β ln η`. RRX fits ln t on y (Abernethy's choice for
small samples), RRY fits y on ln t; ρ² is the squared correlation.

**Maximum likelihood.** With a = ln η, failures contribute
`ln β − a + (β − 1)(ln t − a) − (t/η)^β` and suspensions `−(t/η)^β`. The
score in a gives `η^β = Σ t^β / r`; β solves the monotone profile equation
`1/β + mean_f(ln t) − Σ t^β ln t / Σ t^β = 0` (bisection on ln β, times
scaled by the largest). **Weibayes** fixes β: `η = (Σ t^β / r)^{1/β}`, with
r = 1 assumed when there are no failures (a 63.2 % lower bound); its bounds use
`η = (2 Σ t^β / χ²(·; 2r + 2))^{1/β}`.

**Fisher-matrix bounds.** The observed information in (a, β) is
`I_aa = β² Σw`, `I_aβ = r − Σw − β Σ s w`, `I_ββ = r/β² + Σ s² w`
(s = ln t − a, w = e^{βs}); its inverse is the covariance. For
`u = ln t_p = a + y_p/β`, `Var u = V_aa + (y_p/β²)² V_ββ − 2 (y_p/β²) V_aβ`
and the two-sided bounds are `exp(u ± z Var u^{1/2})`, z = Φ⁻¹((1 + CL)/2).
For rank-regression fits the information is evaluated at the regression
estimates. **Likelihood-ratio bounds**: for each F, the profile likelihood
`ℓ*(u) = max_β ℓ(β, u − y/β)` (golden section) is solved for
`2[ℓ̂ − ℓ*(u)] = χ²₁(CL)` on both sides (bisection); β bounds use the
closed-form η̂(β) profile.

**Acceleration.** Arrhenius `AF = exp[(Ea/k)(1/T_u − 1/T_t)]`,
k = 8.617 333 262 × 10⁻⁵ eV/K; Peck `AF = (RH_t/RH_u)^n · Arrhenius`
(Peck's fit n ≈ 2.66, Ea ≈ 0.79 eV); Coffin–Manson `(ΔT_t/ΔT_u)^m`;
Norris–Landzberg `(ΔT_t/ΔT_u)^n (f_u/f_t)^p exp[C(1/T_max,u − 1/T_max,t)]`
with tin–lead n = 1.9, p = 1/3, C = 1414 K (Norris & Landzberg 1969) and the
commonly quoted tin–silver–copper n = 2.65, p = 0.136, C = 2185 K (Pan et al.
2005 — check the source); inverse power law `(S_t/S_u)^n`.

**Demonstration.** Success run: `n = ln(1 − C)/ln R`. Binomial with r
failures: smallest n with `Σ_{i≤r} C(n,i)(1−R)^i R^{n−i} ≤ 1 − C`.
Weibayes/Lipson: testing each unit for k lives (k = t·AF/L) needs
`n k^β ≥ χ²(C; 2r + 2)/(2(−ln R))`. OC curve: `P(pass) = Bin(≤ r; n, 1 − R_true)`,
and for the Lipson plan a unit fails the test with probability `1 − R_true^{k^β}`.

**Block diagrams.** Series `Π R`, parallel `1 − Π(1 − R)`, k-of-n by dynamic
programming over the children (they need not be identical); a quantity q is
q copies in series. MTTF `= ∫₀^∞ R dt` (adaptive Simpson over octaves);
parts-count MTBF is the same integral over constant-rate blocks only
(`10⁹/ΣFIT` for a series system). Birnbaum `I_B = R(Rᵢ=1) − R(Rᵢ=0)`,
criticality `I_C = I_B (1 − Rᵢ)/(1 − R_sys)`. Monte Carlo samples every
block's time to failure and combines them by min (series), max (parallel) or
the k-th largest (k-of-n); the interval is Wilson's.

**Special functions** (`stats.rs`): Lanczos ln Γ, regularised incomplete
gamma P and Q without cancellation, erfc = Q(½, x²), Acklam's normal quantile
with a Halley step, chi-square quantile by bisection, binomial CDF via ln Γ.

## Iris tie-in and metrics

The use profile (saved in `design.extra.reliability`, all optional) defaults
to a 2-year mission, **8 h/day worn**, **12 open/close cycles per day**, the
thermal model's 30 °C ambient and 60 % RH, a 20 K daily electronics swing,
Ea = 0.7 eV for part FITs, both microphones required, and DVT targets
R = 0.9 at 90 % confidence, 0 failures, life legs at 2 lives with β = 2, 3
units per qualification check. From the Power & Thermal lab's model it
derives the battery life per charge (10.3 h → **0.77 charge cycles per
day**) and the board temperature in use (**34.1 °C**, so thermally
activated FITs are ×0.61 of their 40 °C values).

| Key | Iris value | Meaning |
|---|---|---|
| `reliability.mtbf_h` | **3.55 × 10⁶ h** | parts-count MTBF of the constant-rate blocks (281 FIT total, operating hours) |
| `reliability.r_2yr_pct` | **76.3 %** | system reliability over 2 years at the use profile, wear-out included |
| `reliability.dvt_units` | **37 units** | product units the DVT plan needs (A 22 + B 6 + C 6 + D 3) |
| `reliability.b10_years` | 1.56 years | informational: age at which 10 % of units have failed |

**What the simulation caught.** The LiPo cell causes about **91 %** of
failures within two years (criticality, confirmed by the Monte Carlo
first-failure share): at 0.77 cycles a day it accumulates ≈ 565 cycles, and a
typical small cell reaches 80 % capacity around a characteristic 800 cycles
(β = 4, a typical assumption to replace with the cell maker's data). The hinge
flex cable is next (≈ 5 %), the spring hinge third (≈ 2 %); the electronics are
negligible. A requirement of R ≥ 90 % over two years therefore **fails**
(76.3 %). Computed design changes: a 250 mAh cell (fewer cycles per day)
→ 91.5 %; a cell rated for η = 1 200 cycles → 93.1 %; treating the battery as
a service item → 97.8 %. Making the microphones 1-of-2 barely changes R
(they are not the driver) — the importance view says where effort pays.

The DVT plan's thermal-cycling leg needs 33 cycles of −20 ↔ +60 °C (AF ≈ 45
by Norris–Landzberg for two lives of daily 20 K swings, about 1.5 days); the
10-day 40 °C / 93 % RH damp-heat leg covers ≈ 35 % of the worn humidity
exposure (Peck AF 8.4) — a robustness screen, said so in the plan; the
hinge leg needs 17 520 open/close cycles (≈ 15 h at 20/min). Product legs
take ≈ 17 days after a 2.4-day baseline; bare-cell cycling (≈ 118 days) sets
the schedule and should start in EVT.

**Recommended requirements** (ids are placeholders):

| Id | Text | Kind | Metric | Criterion | Iris now |
|---|---|---|---|---|---|
| R-0xx | Iris shall have at least 90 % reliability over 2 years of the reference use profile (8 h/day worn, 12 open/close cycles per day), counting battery capacity below 80 % as a failure. | performance | `reliability.r_2yr_pct` | ≥ 90 % | 76.3 % — **fails** (battery) |
| R-0xx | The parts-count MTBF of the electronics shall be at least 1 000 000 h (typical failure rates until supplier data replace them). | performance | `reliability.mtbf_h` | ≥ 1 000 000 h | 3.55 × 10⁶ h — passes |
| R-0xx | The DVT plan shall demonstrate R = 0.9 at 90 % confidence with at most 40 product units. | manufacturing | `reliability.dvt_units` | ≤ 40 units | 37 — passes |

## Verification (unit tests)

`cargo test -p forge-core reliability` checks: Γ(5) = 24, Γ(½) = √π;
Φ⁻¹(0.975) = 1.959 964, Φ(1), Φ(−5); chi-square table values
(χ²₀.₉₀(2) = 4.605 17, χ²₀.₉₅(1) = 3.841 46, χ²₀.₉₀(4) = 7.779 44,
χ²₀.₀₅(10) = 3.940 30); a binomial CDF by hand; Johnson adjusted ranks
1, 2.25, 4.125 on a textbook five-item set and Bernard's rank; **MLE on the
Lieblein–Zelen bearings β = 2.102, η = 81.88** (Lawless's published values);
MLE on the censored hinge set against an independent scipy optimisation
(β = 3.440, η = 59 797); **recovery of known β/η from generated data**
(2 000 complete units for MLE and RRX, 3 000 units with 40 % failing for
censored MLE); bounds containing the estimate and Fisher vs likelihood-ratio
B10 bounds within 15 % for n = 23; Weibayes zero-failure η = 2 236 h; the MLE
small-sample bias in the estimator study; parsing formats; **Arrhenius AF by
hand** (0.7 eV, 55 → 125 °C: 77.66); Peck, Coffin–Manson, Norris–Landzberg
reducing to Coffin–Manson and the inverse power law; the planner against
Lipson (6 units for 2 lives at β = 2) and one life for 22 units; the chamber
cycle time (62 min); **success run n = 22 for R = C = 0.9** (and 45, 299);
binomial n = 38 with one failure; Lipson 6, 11 and 22; demonstrated R =
0.1^{1/22}; a monotone OC curve; **series 0.72, parallel 0.98, 2-of-3 0.972**,
k = n and k = 1 limits; series MTBF = 1/Σλ and two parallel units 1.5/λ;
a Weibull block against the closed form, inside the Monte Carlo interval,
Birnbaum = product of the others; the Iris part classification (no
unclassified parts), the 1-of-2 microphone option, the Iris metrics (and
37 DVT units) and that settings move them.

## Limitations

* Failure rates are **typical orders of magnitude**, not predictions; the
  library maps part classes, not part numbers. Replace them with supplier
  reliability reports or your own data (the block editor and "use this fit").
* Two-parameter Weibull only (no three-parameter, lognormal, mixture or
  competing-mode fitting); no interval or left censoring; no reduced-bias
  adjustment of the MLE β; no beta-binomial bounds on the paper.
* Acceleration models are single-mechanism with user-supplied constants; the
  planner assumes β is known and, with allowed failures, that failures happen
  near the end of the test.
* Blocks are independent with no repair, standby switching, common-cause
  failures or load sharing; electronics accumulate failure rate in operating
  hours only (no dormant rate).
* The DVT plan is a starting point: severities are typical for a wearable,
  not taken from any product standard; designations are cited without clause
  numbers — check the current editions. The battery leg (IEC 62133-2,
  UN 38.3) relies on supplier certificates.

## Graduate to

The `reliability` Python library (MatthewReid854/reliability) for many
distributions, ALT fitting and test planners; `lifelines`
(CamDavidsonPilon/lifelines) for survival analysis with covariates and
field-return data; SurPyval (derrynknife/SurPyval) for arbitrary censoring
and truncation. Commercial suites such as ReliaSoft Weibull++ and BlockSim do
the same analyses with more distributions and reporting. Free English
reading: the NIST/SEMATECH e-Handbook chapter 8
(https://www.itl.nist.gov/div898/handbook/apr/apr.htm).

## References

* R. B. Abernethy, *The New Weibull Handbook*, 5th ed., 2006.
* W. Q. Meeker & L. A. Escobar, *Statistical Methods for Reliability Data*, Wiley, 1998.
* W. Nelson, *Accelerated Testing: Statistical Models, Test Plans and Data Analyses*, Wiley, 1990.
* P. D. T. O'Connor & A. Kleyner, *Practical Reliability Engineering*, 5th ed., Wiley, 2012.
* NIST/SEMATECH, *e-Handbook of Statistical Methods*, ch. 8, https://www.itl.nist.gov/div898/handbook/apr/apr.htm
* MIL-HDBK-217F, *Reliability Prediction of Electronic Equipment*, 1991 (Notice 2, 1995).
* C. Lipson & N. J. Sheth, *Statistical Design and Analysis of Engineering Experiments*, McGraw-Hill, 1973.
* J. F. Lawless, *Statistical Models and Methods for Lifetime Data*, 2nd ed., Wiley, 2003.
* J. Lieblein & M. Zelen, "Statistical investigation of the fatigue life of deep-groove ball bearings", J. Res. NBS 57(5), 1956.
* L. G. Johnson, *The Statistical Treatment of Fatigue Experiments*, Elsevier, 1964.
* D. S. Peck, "Comprehensive model for humidity testing correlation", Proc. IRPS, 1986.
* K. C. Norris & A. H. Landzberg, "Reliability of controlled collapse interconnections", IBM J. Res. Dev. 13(3), 1969.
* Z. W. Birnbaum, "On the importance of different components in a multicomponent system", in *Multivariate Analysis II*, 1969.
* M. Rausand & A. Høyland, *System Reliability Theory*, 2nd ed., Wiley, 2004.
* JEDEC JEP122, *Failure Mechanisms and Models for Semiconductor Devices* (check the current revision).
* IEC 60068-2 series (environmental testing), IEC 60529 (IP code), IEC 61000-4-2 (electrostatic discharge), IEC 62133-2 and UN 38.3 (lithium cells), ISO 12870 (spectacle frames) — cited by designation; check the current editions.
* W. H. Press et al., *Numerical Recipes*, 3rd ed., Cambridge, 2007 (special functions).
