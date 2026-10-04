# Power & Thermal Lab (`#/power`, engine module `power`)

Where every milliwatt of the Iris smart glasses goes, how long the lithium-polymer
(LiPo) cell lasts, and how warm the temple pod gets against the wearer's skin.
Everything is computed by the Rust engine (`crates/forge-core/src/power/`)
compiled to WebAssembly, from the bill of materials (`p.components[].power`)
and `design.battery`, `design.power_rails` and `design.thermal`.

## Tabs

| Tab | What you do | What you learn |
|---|---|---|
| Energy budget | Pick a usage scenario (reference mixed use, navigation walk, all-day captions, notifications, standby, display always on); read battery life, average power, usable energy and peak power; follow the waterfall from each subsystem (or component) through conversion loss and quiescent drain to battery power; drag per-component duty-cycle sliders (written to `p.components[].power.duty`, so R-002/R-003, the RL Lab and the thermal case update); edit the cell and regulator; see life vs display duty. | Duty-cycle budgeting, why the display dominates, what the usable fraction and regulator efficiency cost. |
| Discharge | Watch an animated cell drain over a simulated day (play/pause/scrub, wall clock from 07:00); change cell temperature, state of health, internal resistance, cutoff voltage and a Peukert exponent; compare runtimes (ideal, 25 °C, 0 °C, −20 °C) and the budget estimate; plots of open-circuit voltage (OCV), terminal voltage, terminal voltage under peak load, state of charge and current; the OCV curve with the live operating point. | Equivalent-circuit battery models, constant-power loads, why cutoff is reached on voltage not charge, cold and ageing. |
| Thermal | Animated cross-section of the right-temple pod (chips → board → shell → air/skin) coloured by temperature with heat-flow dashes whose speed follows the heat flow; transient of board and shell temperature with the skin limit; display duty, ambient, on/off switching period, start condition and duration; firmware thermal throttling (board threshold, throttled duty, hysteresis) with throttled intervals shaded; steady state vs ambient and vs display duty; the network's resistances and capacitances (written to `design.thermal`); the electrical-analogy RC ladder. | Lumped RC networks, eigenvalues and time constants, the electrical analogy, skin-temperature limits, throttling as a design lever. |
| Battery trade | Capacity sweep: life and total wearable mass vs capacity with the R-002 and R-001 limits and the feasible band shaded; the options of trade study T-3 plotted and scored; try a capacity and apply it (updates `design.battery` and the cell's mass in the BOM). | Turning two hard constraints into a feasible interval; checking a weighted-sum trade study with analysis. |

Downloads: budget CSV (per component + totals), scenarios CSV, discharge CSV,
thermal transient CSV, battery-trade CSV.

## Engine API

| Method | Input | Output |
|---|---|---|
| `power.budget` | `{project?, scenario?, profile?}` | per-component and per-subsystem averages (active and sleep parts, share), load, conversion loss, quiescent, battery average, peak, usable and rated energy, life, currents, C-rate, display refdes, the resolved profile |
| `power.scenarios` | `{project?}` | every built-in scenario: id, name, description, average power, life, display duty, subsystem breakdown, profile |
| `power.discharge` | `{project?, scenario?, profile?, config: {temp_c, state_of_health, peukert_k, capacity_derate?, r_scale?, load_mw?, peak_mw?, dt_s, max_h, compare}}` | main curve and comparison curves (t, state of charge, OCV, terminal V, current, terminal V at peak load, cutoff time, reason, delivered mAh/mWh, brown-out time, equivalent usable fraction), OCV table, budget life, maximum power OCV²/4R |
| `power.ocv` | `{project?}` | OCV vs state of charge (101 points) and the temperature capacity/resistance factors |
| `power.thermal` | `{project?, scenario?, config: {display_duty?, ambient_c?, duration_s, dt_s, pulse_period_s, initial: "ambient"\|"reference", integrator: "exact"\|"rk4", throttle: {enabled, threshold_c, hysteresis_c, throttled_duty}}}` | time series (board, shell, power, display duty, heat flows, throttled), steady state, maxima, time to the skin limit, A matrix, eigenvalues, time constants, throttled fraction, mean duty, maximum safe display duty |
| `power.thermal_sweep` | `{project?, config: {display_duty, ambient_c?, ambient_min, ambient_max, points}}` | steady shell/board vs ambient (and at the reference profile), vs display duty, hottest safe ambient, maximum safe duty |
| `power.trade` | `{project?, scenario?, profile?, config: {total_mass_g?, min_mah, max_mah, points, life_req_h, mass_req_g}}` | life, battery mass and total mass vs capacity, feasible interval, current design, T-3 options with life/mass/feasibility |

A **usage profile** overrides the BOM duty cycles: `subsystem_scale` (multiplier),
then `subsystem_duty` (absolute), then `component_duty` (by refdes), then
`display_duty` (the display component, found like the RL Lab does: first powered
part whose name contains "display").

### Metrics

| Key | Meaning | Iris (v0.6) | Requirement |
|---|---|---|---|
| `power.battery_life_h` | usable energy ÷ average battery power, reference profile | **13.78 h** | R-002 ≥ 8 h ✓ |
| `power.avg_power_mw` | average battery power, reference profile | **54.76 mW** | R-003 ≤ 80 mW ✓ |
| `thermal.max_skin_c` | steady shell temperature, display always on, at `thermal.ambient_c` | **39.0 °C** (30 °C + 164 mW × 55 °C/W) | R-008 ≤ 41 °C ✓ |
| `power.peak_power_mw` | every load active | 202 mW | – |
| `power.energy_mwh` | 240 mAh × 3.7 V × 85 % | 755 mWh | – |
| `thermal.board_max_c` | steady board temperature, display always on | 42.3 °C | – |
| `thermal.time_constant_s` | dominant (slow) time constant | 419 s (fast mode 23.6 s) | – |

All metrics are closed-form (microseconds), so no memoisation is needed.
The budget is the same definition as the RL Lab's fixed policy (tested to
agree; on Iris v0.6 the RL Lab's simulated days give 13.73 h against the
budget's 13.78 h).

## The maths

**Energy budget.** `P̄ᵢ = qtyᵢ (P_active dᵢ + P_sleep (1 − dᵢ))`;
`P_battery = ΣP̄ᵢ / η + I_q V_nom`; `E = C V_nom f_usable`; `life = E / P_battery`.
Iris v0.6: 49.27 mW of loads ÷ 0.9 + 18.5 µW = 54.76 mW; 755 mWh → 13.78 h. The display is
85 % of the load, so life vs display duty is the key curve: ≥ 8 h allows the display on
up to 40 % of the time.

Scenarios (multipliers are stated assumptions; Iris v0.6, 240 mAh): reference 13.8 h ·
navigation walk (display ×2, system-on-chip ×1.6, microphones ×0.4) 7.2 h · all-day
captions (display ×2.67, microphones ×4, system-on-chip ×2.8) 5.2 h · notifications
36.8 h · standby (display and microphones off) 444 h · display always on 4.6 h.

**Discharge.** Rint model `V = OCV(s) − I R`; constant-power load
`R I² − OCV I + P = 0 → I = (OCV − √(OCV² − 4RP)) / 2R` (collapse when
`OCV² < 4RP`); `ds/dt = −I/Q`; forward Euler (30 s) with the final step interpolated
to the exact cutoff crossing. The OCV table is a *typical* 4.2 V lithium-cobalt-oxide /
graphite pouch-cell curve at low rate and 25 °C (the shape in Linden & Reddy and in cell
datasheets), shifted to `full_v` — not a specific cell. Temperature factors (capacity
0.83 and resistance ×1.9 at 0 °C; 0.60 and ×4 at −20 °C) are illustrative trends.
Iris v0.6, fresh 240 mAh cell at 25 °C: 16.8 h (delivers 104 % of C·V_nom because the
mean OCV ≈ 3.8 V exceeds the 3.7 V nominal); 0 °C 13.9 h; −20 °C 10.0 h. The budget's 85 % usable
fraction therefore designs for an aged, cool cell — the requirement must hold at end of
life. Tests: at small current with no resistance the run integrates to exactly the rated
capacity and `∫OCV dQ`; derating scales it; resistance and cold shorten runtime; Peukert
k > 1 shortens high-rate runtime.

**Thermal.** `C_b Ṫ_b = P − (T_b − T_p)/R_bp`, `C_p Ṫ_p = (T_b − T_p)/R_bp − (T_p − T_a)/R_pa`;
`x = T − T_a` gives `ẋ = A x + b P`. Steady state `T_p = T_a + P R_pa`,
`T_b = T_p + P R_bp`. Eigenvalues of the 2 × 2 `A` (real, negative) → time constants
23.6 s and 419 s for Iris. Exact discretisation for piecewise-constant power,
`x(t+h) = x_ss + e^{Ah}(x − x_ss)`, with `e^{Ah}` from Sylvester's formula; RK4 is
available as a cross-check. Heat = all battery power except loads located in the
battery pod (every electrical watt becomes heat; the display's light output is a few
mW). Throttling: when the board reaches the threshold the display duty drops to the
throttled value until the board cools by the hysteresis. At 35 °C ambient the
display-always-on shell reaches 44 °C; a 42.5 °C board threshold with 30 % throttled
duty keeps it below 41 °C. Tests: single node reaches 63 % at τ; steady state = P·R;
eigenvalues vs trace/determinant and `det A = 1/(R_bp R_pa C_b C_p)`; `e^{Ah}` vs Taylor
series; exact vs RK4; pulsed load averages out; sweep consistent with the metric.

**Trade.** `life(C) = C V_nom f_usable / P̄ ≥ 8 h ⇒ C ≥ 139 mAh`;
`mass(C) = m_rest + C (m_cell/C_cell) ≤ 50 g ⇒ C ≤ 281 mAh` (with the CAD frame mass
and v0.6's 4.5 mm rim; total 49.1 g today). The T-3 options: 120 mAh misses 8 h (6.9 h);
180 mAh meets both (10.3 h, 47.8 g); 300 mAh breaks 50 g (50.4 g). v0.6 chose 240 mAh —
inside the 139–281 mAh feasible interval but not one of T-3's listed options — because
the Reliability & Test Lab showed the 180 mAh cell wearing out too fast for R-048 (2-year
reliability 76 % → 90 %); the trade study's recorded decision in `project.json` still
says 180 mAh.

## Iris tie-in

R-002, R-003 and R-008 are verified here; R-001 (mass) through the trade; the RL Lab's
learned power policy uses the same budget; the Circuit Lab's rail-droop requirement
(R-012) is the electrical side of the peak-load brown-out shown on the Discharge tab.

## Limitations and where to go next

* Average-power budget: no wake-up transients, radio connection events or display
  content dependence (micro-OLED power depends on the number of lit pixels). Measure
  with a power profiler over a scripted session.
* Rint cell model: no RC diffusion branches, hysteresis or ageing mechanics; the OCV and
  temperature tables are typical, not measured. Graduate to a Thevenin/2-RC model fitted
  to pulse tests (Plett), PyBaMM for electrochemical models, or the battery models in
  LTspice/ngspice for circuit co-simulation.
* Thermal: two lumped nodes, one shell temperature (the spot above the system-on-chip
  is hotter), constant R (natural convection is non-linear in ΔT), no radiation term, no
  heat into the head. Graduate to thermal finite-element analysis (Elmer, FreeCAD FEM,
  or a commercial solver) and verify with thermocouples or a thermal camera.

## References

* D. Linden & T. B. Reddy, *Handbook of Batteries*, 4th ed., McGraw-Hill, 2011.
* G. L. Plett, *Battery Management Systems, Vol. 1: Battery Modeling*, Artech House, 2015.
* W. Peukert, Elektrotechnische Zeitschrift 18, 1897.
* L. Benini, A. Bogliolo & G. De Micheli, "A survey of design techniques for system-level
  dynamic power management", IEEE Trans. VLSI Systems 8(3), 2000.
* F. P. Incropera, D. P. DeWitt, T. L. Bergman & A. S. Lavine, *Fundamentals of Heat and
  Mass Transfer*, 7th ed., Wiley, 2011.
* G. Strang, *Introduction to Linear Algebra*, 5th ed., 2016 (eigenvalues, matrix exponential).
* IEC 60601-1 applied-part surface temperature limits — commonly cited as 43 °C for
  prolonged skin contact; Iris designs to 41 °C for margin.
