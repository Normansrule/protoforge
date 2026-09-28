# Control Lab

Route: `#/control` · Engine: `crates/forge-core/src/control/` · Metric: `control.head_rms_err_deg`

The Control Lab covers the three things a product like Iris needs from control theory:
**estimation** (where is the wearer looking?), **classical loop design** (PID, Bode,
root locus) and **modern state feedback** (LQR on an unstable plant).

## Tabs

| Tab | What you do | What you learn |
|---|---|---|
| Head tracking | Tune the IMU (rate, noise densities, bias) and the complementary-filter α — sliders write to `design.imu` and re-verify R-011 live. Watch a 3D head with the true pose and a ghost frame for the estimate. | Why a gyro drifts, why an accelerometer lies while you walk, how a complementary filter and a 2-state Kalman filter fuse them, why yaw is unobservable. |
| PID designer | Pick a plant (DC motor speed/position, mass–spring–damper, FOPDT, the Iris pod thermal RC, the inverted pendulum or a custom TF), set Kp/Ki/Kd/N, saturation, anti-windup, derivative-on-measurement, a load disturbance; autotune with Ziegler–Nichols or SIMC. | Step metrics, Bode margins, Nyquist, root locus, sensitivity peak Ms, integrator windup, derivative kick. |
| Inverted pendulum (LQR) | Choose Q/R weights, force limit, controller period, initial tilt; push the pole or move the cart set-point while it balances. | Riccati-based LQR, discretisation, non-minimum-phase behaviour, what saturation does to a linear design. |

Everything is downloadable: head-tracking time series (CSV), step response and Bode data
(CSV), PID gains + transfer functions (JSON), cart–pole trajectory (CSV), LQR gains, A, B, P
and closed-loop poles (JSON).

## The maths

### LTI systems (`lti.rs`)
* Transfer functions `G(s) = N(s)/D(s)·e^{−sT}`, coefficients highest power first; common
  factors of `s` are cancelled exactly (e.g. a PID integrator against the pendulum's zero).
* Time responses: controllable canonical state space, discretised **exactly** for a
  zero-order-hold input (`[Ad Bd] = exp([[A B],[0 0]]Δ)` by scaling-and-squaring); dead
  time is an exact sample delay. Tested: first-order step reaches 63.2 % at τ; second-order
  overshoot equals `exp(−ζπ/√(1−ζ²))` and peak time `π/ω_d`.
* Bode: `G(jω)` with unwrapped phase; margins from gain/phase crossovers refined by bisection
  (tested on `1/(s(s+1)(s+2))`: GM = 6 at √2 rad/s, PM ≈ 53.4°). Delay margin = PM/ω_gc.
* Root locus of `1 + k·C·G = 0` with warm-started Durand–Kerner and nearest-neighbour branch
  matching (dead time → 2nd-order Padé). Tested: branches start at open-loop poles, end at
  zeros or along asymptotes.
* PID: `C(s) = Kp + Ki/s + Kd s/(Tf s + 1)`, `Tf = Kd/(Kp N)`; the loop simulation updates
  the controller every step with back-calculation anti-windup (tracking time `√(Ti Td)`).
  Tested against the linear closed loop without saturation, and anti-windup reduces overshoot.

### Autotuning (`tuning.rs`)
* **Ziegler–Nichols** ultimate gain: `Ku` = plant gain margin, `Pu = 2π/ω₁₈₀`;
  PID `Kp = 0.6Ku, Ti = Pu/2, Td = Pu/8` (P, PI, Pessen and "no overshoot" variants too).
  Tested: `1/(s+1)³` → Ku = 8, Pu = 2π/√3.
* **SIMC** (Skogestad 2003): half-rule reduction for all-real-pole plants (integrating
  plants included), otherwise a two-point FOPDT step fit (Sundaresan & Krishnaswamy);
  `Kc = τ₁/(k(τc+θ))`, `τI = min(τ₁, 4(τc+θ))`, `τD = τ₂`, converted to parallel form.
  Default τc = θ. Tested against the paper's FOPDT and half-rule examples.

### LQR (`lqr.rs`)
Nonlinear cart–pole from the Lagrangian (θ from upright), linearised by central
differences, ZOH-discretised at the controller period; the discrete algebraic Riccati
equation is solved with the structure-preserving doubling algorithm. The controller then
drives the **nonlinear** plant (RK4, 10 substeps) with force saturation and tip impulses
applied through the mass matrix. Tested: Riccati residual < 1e-9, stabilises a 15° start
plus a push, and fails (as it should) with a 2 N limit from 35°.

### Head tracking (`imu.rs`)
* Scenario (60 s, deterministic): look right/left, glance down to read, walk at 2 steps/s
  (vertical bob, lateral sway, forward surge from gait-literature magnitudes), tilt gesture,
  nods. The IMU sits in the right temple pod 13 cm from the neck pivot, so rotations add
  lever-arm accelerations.
* Sensors: gyro = body rate + constant bias (±`gyro_bias_dps` per axis) + white noise with
  per-sample σ = density·√rate; accelerometer = specific force `Rᵀ(a − g)` + noise.
* Estimators: gyro-only integration, accel-only tilt, complementary filter
  `x̂ ← α(x̂ + ẋ_gyro Δt) + (1−α) x_acc` (crossover `f_c = (1−α)/(2π α Δt)`), and a per-axis
  Kalman filter with state [angle, bias] whose measurement noise includes a motion allowance.
* **Metric** `control.head_rms_err_deg` = RMS pitch/roll error of the complementary filter.
  **Iris: 1.62° (≤ 2° → pass)** with α = 0.98 at 200 Hz. The lab's α sweep shows the
  optimum near α ≈ 0.995 (0.84°), and the Kalman filter reaches 0.39°. Yaw drifts ~25–30° in
  60 s for every estimator: gravity carries no heading information.

## How Iris uses it
R-011 (head-orientation error ≤ 2° RMS) is verified by this lab. Head-tilt gestures (UC-3),
horizon-locked captions and the teleprompter nod control all depend on pitch/roll accuracy.
The thermal plant in the PID designer uses `design.thermal`, i.e. the same R/C network as
the Power & Thermal lab — a PI loop on it is a thermal-throttling controller.

## Limitations
* Euler-angle filters (not quaternion/SO(3)); fine for ±60° head motion, singular at ±90° pitch.
* One synthetic scenario; no magnetometer, no temperature-dependent bias, no sensor scale errors
  or axis misalignment.
* Root locus uses a Padé approximation for dead time; SIMC's LHP-zero rules are replaced by a
  step-response fit.
* The PID loop is simulated in fine discrete time, not with a variable-step ODE solver.

## Graduate to
* **python-control** (Python Control Systems Library) — transfer functions, margins, root
  locus, LQR, with MATLAB-compatible names.
* **Drake** (MIT/TRI) — nonlinear dynamics, LQR/trajectory optimisation, rigid-body models.
* **MuJoCo** — physics simulation for controllers beyond the cart–pole.
* For attitude estimation: the Madgwick/Mahony filters (open-source reference code by the
  authors) and full EKF/UKF implementations such as those in PX4 or ArduPilot.

## References
* K. J. Åström & R. M. Murray, *Feedback Systems: An Introduction for Scientists and Engineers*, 2nd ed. — https://fbswiki.org
* G. F. Franklin, J. D. Powell, A. Emami-Naeini, *Feedback Control of Dynamic Systems*.
* J. G. Ziegler & N. B. Nichols, "Optimum settings for automatic controllers", Trans. ASME 64, 1942.
* S. Skogestad, "Simple analytic rules for model reduction and PID controller tuning", J. Process Control 13, 2003.
* R. E. Kalman, "A new approach to linear filtering and prediction problems", J. Basic Engineering, 1960.
* R. Mahony, T. Hamel, J.-M. Pflimlin, "Nonlinear complementary filters on the special orthogonal group", IEEE TAC, 2008.
* S. Madgwick, A. Harrison, R. Vaidyanathan, "Estimation of IMU and MARG orientation using a gradient descent algorithm", IEEE ICORR, 2011.
* B. D. O. Anderson & J. B. Moore, *Optimal Control: Linear Quadratic Methods*, 1990.
* C. Moler & C. Van Loan, "Nineteen dubious ways to compute the exponential of a matrix, twenty-five years later", SIAM Review, 2003.
* University of Michigan, *Control Tutorials for MATLAB & Simulink* (plant parameter sets).
