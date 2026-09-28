//! Planar two-link arm (the X–Y part of a SCARA): forward and analytic
//! inverse kinematics, Jacobian, manipulability and workspace.
//!
//! ```text
//! x = l1 cos q1 + l2 cos(q1 + q2)        cos q2 = (x² + y² − l1² − l2²) / (2 l1 l2)
//! y = l1 sin q1 + l2 sin(q1 + q2)        q1 = atan2(y, x) − atan2(l2 sin q2, l1 + l2 cos q2)
//! ```
//! `J = [[−l1 s1 − l2 s12, −l2 s12], [l1 c1 + l2 c12, l2 c12]]`, and
//! Yoshikawa's manipulability `w = √det(J Jᵀ) = l1 l2 |sin q2|` vanishes at
//! the fully stretched / folded singularities. The velocity ellipse has
//! semi-axes √λᵢ(J Jᵀ).
//!
//! References: B. Siciliano, L. Sciavicco, L. Villani, G. Oriolo,
//! *Robotics: Modelling, Planning and Control*, Springer 2009, §2.9, §3.3;
//! K. M. Lynch & F. C. Park, *Modern Robotics*, 2017 (modernrobotics.org),
//! Ch. 4–6; T. Yoshikawa, "Manipulability of robotic mechanisms", IJRR 1985.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Arm2 {
    pub l1: f64,
    pub l2: f64,
    /// Joint limits, degrees.
    pub q1_min: f64,
    pub q1_max: f64,
    pub q2_min: f64,
    pub q2_max: f64,
}

impl Default for Arm2 {
    fn default() -> Self {
        Arm2 { l1: 1.0, l2: 0.7, q1_min: -170.0, q1_max: 170.0, q2_min: -150.0, q2_max: 150.0 }
    }
}

impl Arm2 {
    pub fn fk(&self, q1: f64, q2: f64) -> ([f64; 2], [f64; 2]) {
        let elbow = [self.l1 * q1.cos(), self.l1 * q1.sin()];
        let tip = [elbow[0] + self.l2 * (q1 + q2).cos(), elbow[1] + self.l2 * (q1 + q2).sin()];
        (elbow, tip)
    }

    /// Analytic IK. `elbow_up` selects q2 ≤ 0 (elbow above the line to the
    /// target when reaching along +x). Returns radians.
    pub fn ik(&self, x: f64, y: f64, elbow_up: bool) -> Option<[f64; 2]> {
        let c2 = (x * x + y * y - self.l1 * self.l1 - self.l2 * self.l2) / (2.0 * self.l1 * self.l2);
        if !(-1.0 - 1e-12..=1.0 + 1e-12).contains(&c2) {
            return None;
        }
        let c2 = c2.clamp(-1.0, 1.0);
        let s2 = (1.0 - c2 * c2).sqrt() * if elbow_up { -1.0 } else { 1.0 };
        let q2 = s2.atan2(c2);
        let q1 = y.atan2(x) - (self.l2 * s2).atan2(self.l1 + self.l2 * c2);
        Some([wrap_pi(q1), q2])
    }

    pub fn within_limits(&self, q: [f64; 2]) -> bool {
        let (a, b) = (q[0].to_degrees(), q[1].to_degrees());
        a >= self.q1_min - 1e-9 && a <= self.q1_max + 1e-9 && b >= self.q2_min - 1e-9 && b <= self.q2_max + 1e-9
    }

    pub fn jacobian(&self, q1: f64, q2: f64) -> [[f64; 2]; 2] {
        let (s1, c1) = q1.sin_cos();
        let (s12, c12) = (q1 + q2).sin_cos();
        [[-self.l1 * s1 - self.l2 * s12, -self.l2 * s12], [self.l1 * c1 + self.l2 * c12, self.l2 * c12]]
    }

    pub fn manipulability(&self, q2: f64) -> f64 {
        (self.l1 * self.l2 * q2.sin()).abs()
    }
}

pub fn wrap_pi(a: f64) -> f64 {
    let p = std::f64::consts::PI;
    (a + p).rem_euclid(2.0 * p) - p
}

/// Velocity manipulability ellipse of J: semi-axes and orientation.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Ellipse {
    pub a: f64,
    pub b: f64,
    pub angle_deg: f64,
}

pub fn ellipse(j: &[[f64; 2]; 2]) -> Ellipse {
    // M = J Jᵀ (symmetric 2×2)
    let m00 = j[0][0] * j[0][0] + j[0][1] * j[0][1];
    let m01 = j[0][0] * j[1][0] + j[0][1] * j[1][1];
    let m11 = j[1][0] * j[1][0] + j[1][1] * j[1][1];
    let tr = m00 + m11;
    let det = m00 * m11 - m01 * m01;
    let disc = ((tr * tr / 4.0) - det).max(0.0).sqrt();
    let l1 = tr / 2.0 + disc;
    let l2 = (tr / 2.0 - disc).max(0.0);
    let angle = 0.5 * (2.0 * m01).atan2(m00 - m11);
    Ellipse { a: l1.sqrt(), b: l2.sqrt(), angle_deg: angle.to_degrees() }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ArmSolution {
    pub elbow_up: bool,
    pub q_deg: [f64; 2],
    pub elbow: [f64; 2],
    pub tip: [f64; 2],
    pub within_limits: bool,
    pub jacobian: [[f64; 2]; 2],
    pub manipulability: f64,
    /// Condition number σmax/σmin of J (∞ at singularity).
    pub condition: f64,
    pub ellipse: Ellipse,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ArmQuery {
    pub reachable: bool,
    pub r_min: f64,
    pub r_max: f64,
    pub distance: f64,
    /// Chosen (requested elbow) solution, or the nearest reachable pose.
    pub solution: Option<ArmSolution>,
    /// The other elbow solution.
    pub alternate: Option<ArmSolution>,
    pub max_manipulability: f64,
}

pub fn solution(arm: &Arm2, q: [f64; 2], elbow_up: bool) -> ArmSolution {
    let (elbow, tip) = arm.fk(q[0], q[1]);
    let j = arm.jacobian(q[0], q[1]);
    let e = ellipse(&j);
    ArmSolution {
        elbow_up,
        q_deg: [q[0].to_degrees(), q[1].to_degrees()],
        elbow,
        tip,
        within_limits: arm.within_limits(q),
        jacobian: j,
        manipulability: arm.manipulability(q[1]),
        condition: if e.b > 1e-12 { e.a / e.b } else { f64::INFINITY },
        ellipse: e,
    }
}

pub fn query(arm: &Arm2, x: f64, y: f64, elbow_up: bool) -> ArmQuery {
    let r = (x * x + y * y).sqrt();
    let r_min = (arm.l1 - arm.l2).abs();
    let r_max = arm.l1 + arm.l2;
    let mut out = ArmQuery { r_min, r_max, distance: r, max_manipulability: arm.l1 * arm.l2, ..Default::default() };
    if let (Some(a), Some(b)) = (arm.ik(x, y, elbow_up), arm.ik(x, y, !elbow_up)) {
        out.reachable = true;
        out.solution = Some(solution(arm, a, elbow_up));
        out.alternate = Some(solution(arm, b, !elbow_up));
    } else {
        // project onto the reachable annulus and point at it
        let rr = r.clamp(r_min + 1e-9, r_max - 1e-9);
        let (px, py) = if r > 1e-12 { (x / r * rr, y / r * rr) } else { (rr, 0.0) };
        if let Some(a) = arm.ik(px, py, elbow_up) {
            out.solution = Some(solution(arm, a, elbow_up));
        }
    }
    out
}

/// Sample the reachable workspace over the joint limits.
pub fn workspace(arm: &Arm2, n: usize) -> Vec<[f64; 2]> {
    let n = n.clamp(4, 200);
    let mut pts = Vec::with_capacity(n * n);
    for i in 0..n {
        let q1 = (arm.q1_min + (arm.q1_max - arm.q1_min) * i as f64 / (n - 1) as f64).to_radians();
        for j in 0..n {
            let q2 = (arm.q2_min + (arm.q2_max - arm.q2_min) * j as f64 / (n - 1) as f64).to_radians();
            pts.push(arm.fk(q1, q2).1);
        }
    }
    pts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fk_ik_identity() {
        let arm = Arm2::default();
        for &(q1, q2) in &[(0.3, 0.9), (-1.2, -0.4), (2.5, 1.7), (0.0, -2.0)] {
            let (_, tip) = arm.fk(q1, q2);
            let up = q2 < 0.0;
            let q = arm.ik(tip[0], tip[1], up).unwrap();
            assert!((wrap_pi(q[0] - q1)).abs() < 1e-9 && (q[1] - q2).abs() < 1e-9, "{q:?} vs {q1},{q2}");
        }
        assert!(arm.ik(2.0, 0.0, true).is_none());
    }

    #[test]
    fn manipulability_is_det_j() {
        let arm = Arm2::default();
        let j = arm.jacobian(0.4, 1.1);
        let det = j[0][0] * j[1][1] - j[0][1] * j[1][0];
        assert!((det.abs() - arm.manipulability(1.1)).abs() < 1e-12);
        let e = ellipse(&j);
        assert!((e.a * e.b - det.abs()).abs() < 1e-9);
    }
}
