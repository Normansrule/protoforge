# New-product wizard

The wizard turns a plain-words description — *what it is, what kind of product,
what it must do, how good it must be* — into a complete starter project:
stakeholder needs, an architecture (subsystems and interfaces), a typical bill
of materials (BOM), a sized battery, board and enclosure, requirements, a
failure mode and effects analysis (FMEA), one computed trade study and
engineering, design and production validation test (EVT, DVT, PVT) stage gates.

Its one rule: **a requirement is bound to a metric only if the engine really
computes that metric from this product's own data.** Everything else is
verified by test, inspection or demonstration and reads “unverified” in
Systems Studio until someone does it. A default that fails is reported, never
tuned away.

## Using it

### In the browser — `/new`

1. **Describe** — name, one-line idea, who uses it. Example ideas fill all
   five steps at once; “Or start another way” loads a template or a blank
   project instead.
2. **Kind** — ten illustrated archetypes (arrow keys move between them). The
   kind sets default capabilities and targets and the surface-temperature
   limit.
3. **Capabilities** — toggle chips. Choices that exclude each other replace
   each other (rechargeable battery ↔ coin cell, printed ↔ molded). Chips
   marked *auto* were added by the wizard: a power source when none was
   chosen, USB-C to charge a battery, and an enclosure process picked from the
   production quantity.
4. **Targets** — sliders with units (logarithmic where the range is wide).
   Only targets that apply are shown (battery life only with a battery, radio
   range only with a radio, design load only for a mechanical part …).
5. **Review** — what was generated, every warning, and the actions:
   **Create project** (asks for confirmation and offers to download the
   current project first, then opens Systems Studio with the requirements
   already evaluated) and **Download project.json**.

The **live preview** beside the steps updates as you type:

| Tab | Shows |
|---|---|
| Architecture | Block diagram of the subsystems and interfaces (power, data, mechanical/optical lines; data lines flow — pause button, off under reduced motion). |
| Parts | The BOM with typical mass, average power and price, and totals. |
| Requirements | Bound requirements with a live pass/fail from evaluating the preview project in the Web Worker (debounced), then the test/inspection/demonstration ones. |
| Why? | Every sizing step with its arithmetic (“why this battery?”). |
| Labs | Which labs analyse *your* data, which partly, and which show the Iris example geometry — plus the full generic-vs-Iris metric table. |

The draft is kept in this browser (local storage) so a reload does not lose it.

After **Create project**, the Overview and Systems Studio → Mission control
show [What should I do next?](next-actions.md) for the new project: the
failing and unverified requirements first, each with the lab tab to open and
the design levers that move it. The wizard is also reachable from
[Start here](start-here.md) (*Describe your own product*) and the command
palette (Ctrl K / ⌘ K → *New product*).

### From the command line

```bash
# wizard flags on `new` (without --kind it still prints a blank project)
protoforge new "Posture Clip" --kind wearable --with ble,battery,imu,buttons \
    --life 24h --idea "A clip-on sensor that buzzes when you slouch" > clip.json
protoforge evaluate clip.json

# other flags: --users TEXT --mass 50g --cost 20 --qty 1000 --retail 99
#              --size 45x38x14 --range 10m --set design_years=3
protoforge wizard @answers.json > project.json   # same answers JSON as the web page
protoforge wizard --options                      # kinds, capabilities, targets, metric table
```

`project.json` goes to standard output; the summary, sizing results and
warnings go to standard error.

### Engine API

| Method | Input | Output |
|---|---|---|
| `systems.wizard_options` | — | `{ kinds[], capabilities[], targets[], metric_table[] }` with help text, defaults, ranges, units |
| `systems.wizard` | `{ name, idea, users, kind, capabilities?, targets{}, author }` | `{ project, explanations[], warnings[], labs[], summary, capabilities[], targets{} }` |

`capabilities` omitted means the kind's defaults; missing targets take the
kind's defaults; out-of-range targets are clamped with a warning. The output
is deterministic.

## Archetypes

| Kind | Default capabilities | Typical targets | Surface limit |
|---|---|---|---|
| Wearable | battery, USB-C, BLE, motion, buttons | 45 g, 16 h, 25 USD, 45 × 38 × 14 mm, 5 m | 41 °C (skin) |
| Handheld gadget | battery, USB-C, BLE, display, buttons, LEDs | 200 g, 8 h, 40 USD | 45 °C (hand) |
| Internet of Things (IoT) sensor node | coin cell, BLE, environmental sensors | 30 g, 8760 h (one year), 10 USD | 50 °C |
| Desk instrument | USB-C, Wi-Fi, display, buttons, environmental sensors | 500 g, 60 USD | 50 °C |
| Robot / mechatronic | battery, USB-C, BLE, motion, motors, buttons | 400 g, 2 h, 60 USD | 50 °C |
| Smart-home (low-voltage) | USB-C, Wi-Fi, environmental sensors, LEDs, buttons | 250 g, 25 USD | 50 °C |
| Optical device | battery, USB-C, display, buttons (+ LED illuminator, lens, photodiode) | 300 g, 4 h, 80 USD | 45 °C |
| Quantum / photonics testbed | bench supplies (two-node fiber quantum key distribution link + quantum random number generator) | 25 kg, 40,000 USD, 10 km fiber | 50 °C |
| Bare printed circuit board (PCB) / dev board | USB-C, light-emitting diodes (LEDs), buttons (+ pin headers, no enclosure) | 15 g, 8 USD, 70 × 30 mm | 50 °C |
| Mechanical part / fixture | printed part (+ L-bracket sized for the load) | 120 g, 8 USD, 50 N | — |

BLE is Bluetooth Low Energy; QKD is quantum key distribution; QRNG is a
quantum random number generator. Surface limits are conservative design
choices, not quotes: check the touch-temperature tables of IEC 62368-1 for
your material and contact time. **Mains (line-voltage) design is out of
scope** — smart-home products are powered from USB or a certified external
low-voltage adapter, and the quantum testbed from certified bench supplies.

## Capabilities

| Group | Capability | What it adds |
|---|---|---|
| Power | Rechargeable battery | lithium-polymer (LiPo) pouch sized from power × life and cycle life, charger, buck converter (L1/C2), connector |
| | Coin cell | CR2032- or CR2450-class cell, holder, bulk capacitor; no regulator |
| | USB-C power / charging | receptacle, configuration-channel (CC) resistors, electrostatic-discharge (ESD) protection; 5 V × 500 mA default budget |
| | Low-voltage DC adapter | barrel jack, reverse-polarity MOSFET, certified 12 V adapter in the box |
| Connectivity | Bluetooth Low Energy | BLE system-on-chip (SoC), crystal, chip antenna, the radio-frequency (RF) lab's matching network and 50 Ω feed |
| | Wi-Fi | Wi-Fi + BLE SoC, flash, 4-layer board |
| Interaction | Display | round liquid-crystal display (LCD) on wearables, e-paper on sensors, organic LED (OLED) on dev boards, thin-film-transistor (TFT) LCD otherwise + flexible printed circuit (FPC) connector |
| | Buttons / touch | tactile switches, or a capacitive touch controller on wearables |
| | Status LEDs | RGB light-emitting diode (LED) + resistors |
| Audio | Microphones | two pulse-density-modulation (PDM) micro-electromechanical (MEMS) microphones as a beamforming pair |
| | Speaker | class-D amplifier + micro speaker |
| Sensing | Motion sensing | 6-axis inertial measurement unit (IMU), or a wake-on-motion accelerometer on coin cells |
| | Environmental sensors | temperature, humidity and pressure sensor |
| | Camera (2 MP class) | camera module, FPC connector, pseudo-static RAM (PSRAM) frame buffer; upgrades the SoC to a Wi-Fi-class part |
| Motion | Motors / actuators | two drive gear motors, wheels and caster on robots (one actuator otherwise) + H-bridge |
| Enclosure | 3-D printed | polyethylene terephthalate glycol (PETG) shells, 2 mm walls, screws and heat-set inserts |
| | Injection-molded | polycarbonate/acrylonitrile-butadiene-styrene (PC/ABS) shells, 1.5 mm walls, tooling estimate (not in the BOM) |
| | Waterproof | gasket, pressure-equalising vent, IP67 test per IEC 60529 |

Every part line names a *class* with typical mass, power (active, sleep, duty)
and price — planning numbers, not quotes and not part numbers.

## The derivation maths

All of it is shown in the **Why?** tab and in the CLI's standard error.

**Average power** (same definition as the Systems roll-up and the Power lab):

$$\bar P = \frac{\sum_i n_i\,(P_{a,i}\,d_i + P_{s,i}\,(1-d_i))}{\eta} + I_q V$$

with duty cycles typical of each kind (for example a wearable's display is on
30 % of the time, a robot's motors 35 %).

**Rechargeable battery.** Energy per charge with a 20 % growth allowance
(early power estimates grow), and enough capacity that the cell is not
recharged so often that it wears out before the design life:

$$t_{charge} = \max\!\Big(1.2\,t_{target},\ \frac{1.15 \cdot Y \cdot 365 \cdot h_{day}}{B_{10}}\Big),\qquad B_{10} = \eta\,(-\ln 0.9)^{1/\beta}$$

with the cell's Weibull wear-out (β = 4, η = 800 equivalent full cycles to
80 % capacity) taken from the Reliability Lab's part library, so
B₁₀ ≈ 455 cycles. Capacity C = P̄ · t / (3.7 V · 0.85 usable), rounded up to a
common pouch size; mass = C · V / ρ with a typical energy density
ρ = 120 + 60 · C/(C + 300) mWh/g; charge time ≈ 1.2 · C / min(0.5 C, 500 mA).

**Coin cell.** The smallest of CR2032 (≈ 225 mAh) and CR2450 (≈ 620 mAh)
that meets 1.2 × life; the peak power is checked against ≈ 45 mW (15 mA
pulses × 3 V with a bulk capacitor — check the cell maker's pulse data).

**Robot mass–power loop.** Per drive motor while driving at v = 0.3 m/s:
P = P₀ + m g C_rr v / (2 η) with P₀ = 0.3 W no-load, C_rr = 0.03, η = 0.35
(typical N20-class gear motor). Mass sets motor power, motor power sets the
battery, the battery adds mass — iterated to a fixed point (shown). Wheel
torque T = (m g C_rr + m a) r / 2 is checked against typical small gear
motors.

**Board area.** Σ courtyard areas from the PCB library (IPC-7351 courtyards)
or typical land areas for modules × a routing factor (2.5 on 2 layers, 1.8 on
4). If it does not fit inside the enclosure (walls + 1 mm), parts go on both
sides, then 4 layers; Wi-Fi and cameras start at 4 layers. The outline keeps
the enclosure's aspect ratio.

**Enclosure.** Shell surface A = 2(LW + LH + WH), volume A · wall · 1.15
(bosses, ribs), mass V · ρ. Printing ≈ filament × 1.15 × 25 USD/kg + hours ×
1.5 USD/h; molding ≈ material + 2 shots × 30 s × 40 USD/h per set, plus a
rough tooling estimate. Printing is chosen below 2,000 units, molding above
(the trade study shows the computed unit costs).

**Bracket (mechanical part).** Strength t ≥ √(6 F L / (b σ_allow)) with
σ_allow = σ_y · k / 2 (printed PETG: σ_y ≈ 50 MPa, k = 0.6 across layers;
molded PC: 60 MPa, k = 1); stiffness t ≥ ∛(4 F L³ / (E δ b)) for a tip
deflection δ of 2 % of L. The larger wins.

**Thermal.** Shell to air R = 1 / (h A) with h ≈ 10 W/m²K (natural convection
≈ 5 + radiation ≈ 5, typical; 70 % of the area for skin-worn products); board
to shell ≈ 20 °C/W · √(20 cm² / board area); heat capacities from the shell and
board masses. The Power lab's two-node network then predicts the surface
temperature with the display always on.

**Reliability targets.** R(t) = exp(−t/MTBF) ≥ 0.9 over the design life
t = years · 365 · hours/day ⇒ MTBF ≥ t / 0.105 (mean time between failures);
B10 life ≥ design life.

**Radio.** Friis free-space loss at 1 m, log-distance indoors (n = 2.5), the
antenna efficiency by board size (typical 30–50 %), head/body losses by kind,
the RF lab's recommended matching network, and a 10 dB fading allowance:
margin ≥ 3 dB at the range target.

**Microphones.** Spacing d = 0.4 × envelope length (8–40 mm); spatial
aliasing above c / (2d).

**Quantum link.** Channel loss 0.2 dB/km × L + 3 dB; key rate, quantum bit error rate (QBER) and
maximum distance from the Quantum Lab's decoy-state BB84 model; QRNG
min-entropy and NIST SP 800-22 results from its diode-noise simulation.

## Which metrics are generic? (the experiment)

A generic starter project was perturbed one input at a time — component mass,
power, cost and names; battery capacity and voltage; thermal network; frame,
optics, PCB, audio, IMU, chip and regulator parameters; quantity; retail price;
RF link distance and antenna efficiency; reliability hours per day; subsystem
domains — and every metric was compared before and after, and against Iris.
The test `systems::wizard::tests::metric_table_matches_experiment` repeats
the experiment on generated projects so the table cannot drift.

| Metrics | Lab | Class | Responds to | Bound by the wizard |
|---|---|---|---|---|
| `budget.avg_power_mw`, `budget.battery_life_h` | Power / Systems | generic | component power × duty, regulator efficiency, quiescent current, battery | yes |
| `power.avg_power_mw`, `power.battery_life_h`, `power.peak_power_mw`, `power.energy_mwh` | Power | generic | same | peak power |
| `thermal.max_skin_c`, `thermal.board_max_c`, `thermal.time_constant_s` | Power | generic | heat from the power budget, the lumped network in `design.thermal` | surface temperature |
| `mass.components_g`, `bom.cost_usd` | Systems | generic | BOM mass and quantity-1 prices | mass |
| `mfg.bom_cost_usd` | Manufacturing Hub | generic | BOM prices with learning-curve price breaks at the target quantity | parts cost |
| `reliability.mtbf_h`, `reliability.r_2yr_pct`, `reliability.b10_years` | Reliability | generic | part classes, board temperature, charge cycles, hours per day | MTBF, B10 |
| `dsp.mic_di_db`, `dsp.decim_stopband_db` | DSP | conditional | `design.audio` (microphone count and spacing, decimation chain) | with microphones |
| `circuit.buck_ripple_mv` | Circuit | conditional | battery voltage, core rail, L1/C2 values parsed from the BOM | with a rechargeable battery |
| `rf.link_margin_db`, `rf.range_m`, `rf.antenna_efficiency_pct`, `rf.match_return_loss_db`, `rf.feed_z0_ohm` | RF & Antenna | conditional | `design.extra.rf` (distance, losses, efficiency, matching network, feed), board thickness | link margin, with BLE |
| `quantum.qkd_*`, `quantum.qrng_*` | Quantum | conditional | `design.extra.qkd`, `design.extra.qrng` (absent otherwise) | quantum testbed |
| `control.head_rms_err_deg` | Control | partial | `design.imu` — but scores a synthetic head-motion scenario | no (bench test instead) |
| `rl.policy_life_gain_pct` | RL | partial | component power, battery — user contexts are the Iris use cases | no |
| `mfg.unit_cost_usd`, `mfg.crossover_qty` | Manufacturing Hub | partial | BOM and quantity **plus the Iris frame's printing/molding cost** | no |
| `program.gross_margin_pct`, `breakeven_units`, `npv_usd`, `bac_usd` | Program & Business | partial | retail price, the Manufacturing Hub unit cost (Iris frame included) | no |
| `build.*` | Build Guide | partial | markets, radios and batteries in the BOM, on-PCB parts | no |
| `program.cpm_weeks`, `p80_weeks`, `min_trl`, `min_mrl`; `reliability.dvt_units` | Program, Reliability | template | the standard task network / the DVT confidence targets | no |
| `cad.*`, `mass.total_g`, `fit.pcb_pod_margin_mm` | CAD | Iris | the Iris frame (`mass.total_g` adds it to the BOM mass) | no |
| `structures.*`, `ergonomics.*`, `fluids.*`, `mechanisms.*`, `molding.*`, `print.*` | Optics & Mechanics, Production | Iris | the Iris frame, pod and hinge geometry | no |
| `optics.*` | Optics | Iris | `design.optics`, a birdbath head-up display | no |
| `chip.*`, `firmware.*` | Chip, Prototype & Firmware | Iris | the Iris finite-impulse-response (FIR) filter and firmware (identical for every product) | no |
| `circuit.rail_droop_mv`, `circuit.vbat_divider_ua` | Circuit | Iris | Iris presets (display load step, R1/R2 divider) | no |
| `pcb.*`, `robotics.pnp_cycle_s` | PCB, Robotics | Iris | parts marked on-PCB, nets from the Iris schematic table | no (absent while parts are off the PCB) |

## What gets generated

* **Needs** — the idea itself, carry/size, battery life, safety, durability,
  connectivity, voice, environment, affordability, buildability (only those
  that apply). Every need has at least one derived requirement.
* **Subsystems** — enclosure (or bracket / racks), power, compute and radio,
  main board, firmware, user interface, audio, sensing, motors, optics,
  QRNG and quantum link, manufacturing; each with the lab that designs it and
  at least one allocated requirement.
* **Interfaces** — power rails, I2C/SPI/PDM/I2S/PWM/DVP/BLE data links and
  mechanical/optical joints between the subsystems that exist.
* **Requirements** — bound ones (mass, parts cost at volume, battery life,
  average power, peak power, surface temperature, MTBF, B10, buck ripple, BLE
  margin, microphone directivity and decimation, quantum key rate, QBER,
  maximum distance, QRNG entropy and tests) and test/inspection/demonstration
  ones with real acceptance criteria (envelope, charge time, display
  legibility, button response, LED visibility, loudness, tilt error,
  temperature accuracy, camera capture, drive speed, illuminance, IP67, drop,
  bracket load and deflection, board outline, firmware update, radio
  certification, battery protection, coin-cell door, laser interlock).
* **FMEA** — risks per energy source and capability with severity,
  occurrence, detection and mitigation.
* **Trade study** — battery capacity (three neighbouring pouch sizes scored
  from computed life, mass, cost and charge time), primary cell, enclosure /
  manufacturing process (unit costs computed at the target quantity) or
  detector technology; the recorded decision is the weighted winner.
* **Stage gates** — concept, requirements, preliminary design, EVT, DVT and
  PVT with exit criteria from the capabilities (battery life on 3 units,
  emissions pre-scan, IP67 on 5 units, drop, proof load, pilot-run yield).
* **Design parameters** — battery, rails, PCB outline and layers, audio,
  thermal network, manufacturing quantity and process, and in
  `design.extra`: `wizard` (the answers), `reliability` (use profile),
  `program` (retail price), `rf` (link and matching) and `qkd`/`qrng`.

## Limitations

* Part values are typical classes; replace them with data-sheet numbers as
  soon as parts are chosen. Prices are planning numbers, not quotes.
* The Iris-geometry labs (CAD, Structures, Thermal & Fluids, Mechanisms, Human
  Factors, Molding, Print, Optics) still analyse the glasses: model your own
  part in your CAD tool, and use those labs to learn the method.
* Board parts start off the PCB (like the built-in templates): PCB Studio's
  nets come from the Iris schematic table, and routing a new outline takes
  seconds, which would stall live verification.
* The thermal model is a two-node lumped network; hot spots need the Thermal &
  Fluids lab's 3-D model of your own geometry.
* Wi-Fi has no link-budget metric yet, so Wi-Fi range is a test requirement.
* Robot drive power assumes a flat desk at 0.3 m/s; ramps and stalls need the
  Robotics and Mechanisms labs.
* Mains-powered and laser products are out of scope.

## References

* NASA, *Systems Engineering Handbook*, SP-2016-6105 Rev 2 (2016) — technical
  requirements, technical performance measures and margins.
* INCOSE, *Guide to Writing Requirements* (2023) — the quality checks the
  generated requirements pass.
* D. Linden & T. B. Reddy, *Handbook of Batteries*, 4th ed., McGraw-Hill
  (2011) — energy from capacity × voltage, usable fraction.
* F. P. Incropera, D. P. DeWitt, T. L. Bergman & A. S. Lavine, *Fundamentals
  of Heat and Mass Transfer*, 7th ed., Wiley (2011), ch. 9 — natural
  convection.
* R. C. Hibbeler, *Mechanics of Materials*, 10th ed., Pearson (2017) —
  bending stress and cantilever deflection.
* P. D. T. O'Connor & A. Kleyner, *Practical Reliability Engineering*, 5th
  ed., Wiley (2012) — failure rates, Weibull wear-out, B10 life.
* T. S. Rappaport, *Wireless Communications: Principles and Practice*, 2nd
  ed., Prentice Hall (2002), §4.9 — log-distance path loss.
* IPC-7351B, *Generic Requirements for Surface Mount Design and Land Pattern
  Standard* — courtyards.
* IEC 60529 (IP code) and IEC 62368-1 (touch temperatures) — check the
  current editions for test conditions and limits.
