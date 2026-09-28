# Robotics Lab

Route: `#/robotics` · Engine: `crates/forge-core/src/robotics/` · Metric: `robotics.pnp_cycle_s`

## Tabs

| Tab | What you do | What you learn |
|---|---|---|
| Pick & place | Watch a SCARA populate the Iris main board from a feeder bank (parts fly from feeders via the up-camera to their pads); change arm speed, Z stroke, dwell times, vision, feeder pitch, board handling and sequence optimisation. Settings are saved in `design.extra.pnp` and re-verify R-023. | Where cycle time really goes (travel vs Z vs vision vs vacuum dwell), feeder ordering, sequencing, CPH. |
| Arm playground | Drag a target around a planar 2-link arm (SCARA top view), toggle elbow up/down, change link lengths and joint limits. | FK, analytic IK and its two solutions, reachability, the Jacobian, singularities and the manipulability ellipse. |
| Trajectories | Compare trapezoidal, cubic and quintic profiles (position/velocity/acceleration/jerk) under the same limits; synchronise a two-joint move; compare a Cartesian straight line with joint-space interpolation. | Time-optimality vs smoothness, why jerk matters, why straight lines near singularities need huge joint speeds. |
| Mobile robot | Drag spline waypoints, tune lookahead, speed, wheel base and wheel limits for a differential-drive robot running pure pursuit. | Pure-pursuit geometry, the lookahead trade-off, non-holonomic kinematics, wheel saturation. |

Downloads: arm solution (JSON), profiles (CSV), pick-and-place sequence and full event
timeline (CSV) and plan (JSON), pure-pursuit trajectory (CSV).

## The maths
* **Kinematics** (`arm.rs`): `x = l1 c1 + l2 c12`, `y = l1 s1 + l2 s12`;
  `cos q2 = (x²+y²−l1²−l2²)/(2 l1 l2)`, `q1 = atan2(y,x) − atan2(l2 s2, l1 + l2 c2)`.
  `J = [[−l1 s1 − l2 s12, −l2 s12], [l1 c1 + l2 c12, l2 c12]]`,
  manipulability `w = l1 l2 |sin q2|`, ellipse axes `√λ(JJᵀ)`. Tests: FK∘IK identity,
  `w = |det J|`, ellipse area consistency.
* **Profiles** (`traj.rs`): trapezoid `T = D/v + v/a` (triangular if `D < v²/a`); cubic
  `D(3τ²−2τ³)`; quintic `D(10τ³−15τ⁴+6τ⁵)`; minimum durations under limits
  (`1.5D/v, √(6D/a)` and `1.875D/v, √(5.774D/a)`); synchronised joints by solving
  `D = vT − v²/a`. Tests: trapezoid area = distance, quintic boundary conditions and peak
  velocity, synchronised joints arrive together.
* **Pick and place** (`pnp.rs`): placement comes from `crate::pcb::placement(p)` joined with
  `p.components`; one feeder per unique (example part, footprint). Feeders are ordered along
  the rail by the mean board x of their parts; placements are sequenced greedily on a
  precomputed time matrix and improved by pairwise swaps (the naïve BOM order is reported for
  comparison). Every XY move is a synchronised joint-space trapezoid of J1/J2 with the nozzle
  rotation J4 overlapped; Z moves are separate trapezoids; vacuum pick/place and up-camera
  alignment are dwells; board load/unload and two fiducials per side are handling; bottom-side
  parts trigger a flip. The result includes a 30 fps joint trajectory for animation.
* **Pure pursuit** (`mobile.rs`): closed Catmull–Rom path resampled every 1 cm; goal point
  `L` ahead of the closest point; `κ = 2y_r/L²`, `ω = vκ`, wheel speeds `v ∓ ωb/2` saturated
  by common scaling; exact arc integration of the unicycle.

## How Iris uses it
**R-023: pick-and-place ≤ 60 s per board.** With the default desktop-SCARA cell the Iris
board (25 placements, 21 feeders) takes **28.6 s** (travel 12.5 s, Z 7.3 s, vision 3.5 s,
dwell 2.3 s, handling 3.0 s; ≈ 3 500 CPH) → pass. Because placement comes from the PCB
module, the cycle time updates when the PCB Studio moves parts (today `placement()` returns
a placeholder row, so the sequencing gain is small).

## Limitations
* Generic desktop-SCARA speeds and dwell times (editable) — not a vendor specification.
* Single vacuum nozzle, no feeder indexing/splice time, no nozzle changes, no dynamics
  (torque limits) — only velocity/acceleration limits.
* Pure pursuit on a kinematic robot: no wheel slip, motor dynamics or obstacle avoidance.
* The sequencing heuristic is greedy + swaps, not an exact TSP solver.

## Graduate to
* **ROS 2** with **MoveIt 2** (motion planning, IK solvers, time parameterisation) and
  **Nav2** (its Regulated Pure Pursuit controller) for mobile robots.
* **Drake** for rigid-body dynamics, trajectory optimisation and manipulation.
* **MuJoCo** for contact-rich simulation.
* **OpenPnP** — open-source pick-and-place machine control, for real feeder and vision setups.

## References
* B. Siciliano, L. Sciavicco, L. Villani, G. Oriolo, *Robotics: Modelling, Planning and Control*, Springer 2009.
* K. M. Lynch & F. C. Park, *Modern Robotics: Mechanics, Planning, and Control*, 2017 — http://modernrobotics.org
* L. Biagiotti & C. Melchiorri, *Trajectory Planning for Automatic Machines and Robots*, Springer 2008.
* T. Yoshikawa, "Manipulability of robotic mechanisms", Int. J. Robotics Research, 1985.
* R. C. Coulter, "Implementation of the Pure Pursuit Path Tracking Algorithm", CMU-RI-TR-92-01, 1992.
