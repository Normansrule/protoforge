//! Plant library for the PID designer. Every plant is described by physical
//! parameters (with units and sensible ranges for UI sliders) and turned
//! into a transfer function by [`build`].
//!
//! Several parameter sets are the classic ones from the University of
//! Michigan / Carnegie Mellon *Control Tutorials for MATLAB & Simulink*
//! (ctms.engin.umich.edu), so students can cross-check results.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::lti::{Pid, Tf};
use crate::math::poly_mul;
use crate::project::Project;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParamSpec {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub log: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlantInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    /// LaTeX-free plain-text transfer function.
    pub formula: String,
    pub input: String,
    pub output: String,
    pub params: Vec<ParamSpec>,
    /// A reasonable starting controller.
    pub pid: Pid,
    /// Typical actuator limit (symmetric unless `u_min` given).
    pub u_max: f64,
    pub u_min: f64,
    pub iris: String,
}

fn ps(key: &str, label: &str, unit: &str, value: f64, min: f64, max: f64, log: bool) -> ParamSpec {
    ParamSpec { key: key.into(), label: label.into(), unit: unit.into(), value, min, max, log }
}

/// All plants, with the thermal plant parameterised from the project.
pub fn catalog(p: Option<&Project>) -> Vec<PlantInfo> {
    let th = p.map(|p| p.design.thermal.clone()).unwrap_or_default();
    vec![
        PlantInfo {
            id: "dc_motor_speed".into(),
            name: "DC motor — speed".into(),
            description: "Armature-controlled DC motor: voltage in, shaft speed out. Electrical (L, R) and mechanical (J, b) time constants coupled by the torque/back-EMF constant K.".into(),
            formula: "ω/V = K / ((J s + b)(L s + R) + K²)".into(),
            input: "armature voltage V".into(),
            output: "speed ω (rad/s)".into(),
            params: vec![
                ps("J", "Rotor inertia J", "kg·m²", 0.01, 0.001, 0.1, true),
                ps("b", "Viscous friction b", "N·m·s", 0.1, 0.01, 1.0, true),
                ps("K", "Motor constant K", "N·m/A", 0.01, 0.001, 0.1, true),
                ps("R", "Resistance R", "Ω", 1.0, 0.1, 10.0, true),
                ps("L", "Inductance L", "H", 0.5, 0.01, 2.0, true),
            ],
            pid: Pid { kp: 100.0, ki: 200.0, kd: 10.0, n: 20.0 },
            u_max: 1000.0,
            u_min: -1000.0,
            iris: "Same physics as a vibration motor or the focus actuator a future Iris revision might use.".into(),
        },
        PlantInfo {
            id: "dc_motor_position".into(),
            name: "DC motor — position".into(),
            description: "The same motor with shaft angle as the output: an extra integrator (type-1 plant), so a P controller already removes steady-state error to steps.".into(),
            formula: "θ/V = K / (s((J s + b)(L s + R) + K²))".into(),
            input: "armature voltage V".into(),
            output: "angle θ (rad)".into(),
            params: vec![
                ps("J", "Rotor inertia J", "kg·m²", 0.01, 0.001, 0.1, true),
                ps("b", "Viscous friction b", "N·m·s", 0.1, 0.01, 1.0, true),
                ps("K", "Motor constant K", "N·m/A", 0.01, 0.001, 0.1, true),
                ps("R", "Resistance R", "Ω", 1.0, 0.1, 10.0, true),
                ps("L", "Inductance L", "H", 0.5, 0.01, 2.0, true),
            ],
            pid: Pid { kp: 80.0, ki: 20.0, kd: 25.0, n: 20.0 },
            u_max: 100.0,
            u_min: -100.0,
            iris: "A robot joint or the SCARA axes in the Robotics Lab are position servos like this.".into(),
        },
        PlantInfo {
            id: "mass_spring_damper".into(),
            name: "Mass–spring–damper".into(),
            description: "The canonical second-order system: force in, displacement out. ωn = √(k/m), ζ = c / (2√(k m)).".into(),
            formula: "x/F = 1 / (m s² + c s + k)".into(),
            input: "force F (N)".into(),
            output: "position x (m)".into(),
            params: vec![
                ps("m", "Mass m", "kg", 1.0, 0.1, 10.0, true),
                ps("c", "Damping c", "N·s/m", 10.0, 0.1, 50.0, true),
                ps("k", "Stiffness k", "N/m", 20.0, 1.0, 500.0, true),
            ],
            pid: Pid { kp: 350.0, ki: 300.0, kd: 50.0, n: 20.0 },
            u_max: 1000.0,
            u_min: -1000.0,
            iris: "Models a hinge spring or the flex of the display pod on the temple.".into(),
        },
        PlantInfo {
            id: "fopdt".into(),
            name: "First order + dead time".into(),
            description: "The workhorse process model (heaters, flows, anything with transport lag). Dead time is exact in Bode plots and simulation, Padé-approximated for the root locus.".into(),
            formula: "y/u = K e^(−θ s) / (τ s + 1)".into(),
            input: "u".into(),
            output: "y".into(),
            params: vec![
                ps("K", "Gain K", "", 1.0, 0.1, 10.0, true),
                ps("tau", "Time constant τ", "s", 1.0, 0.05, 20.0, true),
                ps("theta", "Dead time θ", "s", 0.3, 0.0, 5.0, false),
            ],
            pid: Pid { kp: 1.2, ki: 1.0, kd: 0.1, n: 10.0 },
            u_max: 100.0,
            u_min: -100.0,
            iris: "Skogestad's SIMC rules were derived for exactly this model.".into(),
        },
        PlantInfo {
            id: "thermal_rc".into(),
            name: "Iris pod thermal RC".into(),
            description: "Two-node thermal network from the project (board → pod shell → ambient). Input is SoC/display heat, output is the board temperature rise. A PI loop on this plant is thermal throttling.".into(),
            formula: "ΔT_b/P = (C_p s + g) / ((C_b s + 1/R_bp)(C_p s + g) − 1/R_bp²),  g = 1/R_bp + 1/R_pa".into(),
            input: "heat P (W)".into(),
            output: "board rise ΔT (°C)".into(),
            params: vec![
                ps("r_bp", "Board→pod R", "°C/W", th.r_board_pod_c_per_w, 1.0, 100.0, true),
                ps("r_pa", "Pod→ambient R", "°C/W", th.r_pod_ambient_c_per_w, 5.0, 200.0, true),
                ps("c_b", "Board heat capacity", "J/°C", th.c_board_j_per_c, 0.1, 20.0, true),
                ps("c_p", "Pod heat capacity", "J/°C", th.c_pod_j_per_c, 0.5, 50.0, true),
            ],
            pid: Pid { kp: 0.05, ki: 0.0005, kd: 0.0, n: 10.0 },
            u_max: 0.5,
            u_min: 0.0,
            iris: "Same R/C values the Power & Thermal lab uses (design.thermal). Heat can only be added, so the actuator saturates at 0 W — try anti-windup.".into(),
        },
        PlantInfo {
            id: "inverted_pendulum".into(),
            name: "Inverted pendulum (angle)".into(),
            description: "Linearised pendulum on a cart, force → pole angle. Open-loop unstable (a pole in the right half-plane); PID can hold the angle but the cart drifts — see the LQR tab for full-state control.".into(),
            formula: "φ/F = (m l s / q) / (s³ + b(I+ml²)s²/q − (M+m)m g l s/q − b m g l/q),  q = (M+m)(I+ml²) − (ml)²".into(),
            input: "force F (N)".into(),
            output: "angle φ (rad)".into(),
            params: vec![
                ps("M", "Cart mass M", "kg", 0.5, 0.1, 5.0, true),
                ps("m", "Pole mass m", "kg", 0.2, 0.05, 2.0, true),
                ps("b", "Cart friction b", "N·s/m", 0.1, 0.0, 1.0, false),
                ps("I", "Pole inertia I", "kg·m²", 0.006, 0.001, 0.1, true),
                ps("l", "Pivot→COM l", "m", 0.3, 0.05, 1.0, true),
            ],
            pid: Pid { kp: 100.0, ki: 1.0, kd: 20.0, n: 20.0 },
            u_max: 1000.0,
            u_min: -1000.0,
            iris: "Balancing is the same problem a head-stabilised display solves in reverse.".into(),
        },
    ]
}

fn get(params: &BTreeMap<String, f64>, info: &PlantInfo, key: &str) -> f64 {
    params
        .get(key)
        .copied()
        .unwrap_or_else(|| info.params.iter().find(|p| p.key == key).map(|p| p.value).unwrap_or(1.0))
}

/// Build the transfer function of a catalog plant (or a custom num/den).
pub fn build(id: &str, params: &BTreeMap<String, f64>, custom: Option<&Tf>, p: Option<&Project>) -> Result<Tf, String> {
    if id == "custom" {
        let t = custom.ok_or("custom plant needs num/den")?.normalized();
        if t.den.iter().all(|v| *v == 0.0) {
            return Err("denominator is zero".into());
        }
        if !t.is_proper() {
            return Err("custom plant must be proper (numerator degree ≤ denominator degree)".into());
        }
        return Ok(t);
    }
    let cat = catalog(p);
    let info = cat.iter().find(|x| x.id == id).ok_or_else(|| format!("unknown plant `{id}`"))?;
    let g = |k: &str| get(params, info, k);
    let tf = match id {
        "dc_motor_speed" | "dc_motor_position" => {
            let (j, b, k, r, l) = (g("J"), g("b"), g("K"), g("R"), g("L"));
            let den = vec![j * l, j * r + b * l, b * r + k * k];
            if id == "dc_motor_speed" {
                Tf::new(vec![k], den)
            } else {
                Tf::new(vec![k], poly_mul(&den, &[1.0, 0.0]))
            }
        }
        "mass_spring_damper" => Tf::new(vec![1.0], vec![g("m"), g("c"), g("k")]),
        "fopdt" => Tf::new(vec![g("K")], vec![g("tau"), 1.0]).with_delay(g("theta")),
        "thermal_rc" => {
            let (rbp, rpa, cb, cp) = (g("r_bp"), g("r_pa"), g("c_b"), g("c_p"));
            let a = 1.0 / rbp;
            let gg = a + 1.0 / rpa;
            let den = crate::math::poly_add(&poly_mul(&[cb, a], &[cp, gg]), &[-a * a]);
            Tf::new(vec![cp, gg], den)
        }
        "inverted_pendulum" => {
            let (mm, m, b, i, l) = (g("M"), g("m"), g("b"), g("I"), g("l"));
            let gr = 9.8;
            let q = (mm + m) * (i + m * l * l) - (m * l).powi(2);
            Tf::new(vec![m * l / q, 0.0], vec![1.0, b * (i + m * l * l) / q, -(mm + m) * m * gr * l / q, -b * m * gr * l / q])
        }
        _ => return Err(format!("unknown plant `{id}`")),
    };
    Ok(tf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thermal_dc_gain_is_series_resistance() {
        let p = Project::iris();
        let tf = build("thermal_rc", &BTreeMap::new(), None, Some(&p)).unwrap();
        let t = &p.design.thermal;
        assert!((tf.dc_gain() - (t.r_board_pod_c_per_w + t.r_pod_ambient_c_per_w)).abs() < 1e-9);
    }

    #[test]
    fn pendulum_is_open_loop_unstable() {
        let tf = build("inverted_pendulum", &BTreeMap::new(), None, None).unwrap();
        assert!(tf.poles().iter().any(|p| p.re > 0.0));
    }

    #[test]
    fn msd_natural_frequency() {
        let tf = build("mass_spring_damper", &BTreeMap::new(), None, None).unwrap();
        // product of the poles = k/m = ωn², sum = −c/m
        let p = tf.poles();
        let prod = p[0] * p[1];
        assert!((prod.re - 20.0).abs() < 1e-9 && prod.im.abs() < 1e-9);
        assert!(((p[0] + p[1]).re + 10.0).abs() < 1e-9);
    }
}
