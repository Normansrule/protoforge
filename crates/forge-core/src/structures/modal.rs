//! Modal analysis of a temple as a beam.
//!
//! The voxelised CAD temple is sliced along its length (the assembly Y axis):
//! each slice gives the area A, centroid and second moments of area
//! I = Σ ρ h² (x − x̄)² (+ h⁴/12 per voxel) for lateral (X) and vertical (Z)
//! bending, plus mass per length. Electronics, the lid and the PCB are added
//! as mass spread over the pod. An Euler–Bernoulli beam of Hermite cubic
//! elements (consistent mass matrices), clamped at the hinge, gives the
//! generalised eigenproblem K φ = ω² M φ, solved by Cholesky reduction and
//! Jacobi rotations.
//!
//! Reference: K.-J. Bathe, *Finite Element Procedures*, ch. 10–11 (2nd ed.);
//! the uniform cantilever check uses β₁L = 1.8751
//! (f₁ = (β₁L)²/(2π) √(EI/(mL⁴))).

use serde::Serialize;

/// Symmetric eigen-decomposition by cyclic Jacobi rotations.
/// Returns (eigenvalues, eigenvectors as columns).
pub fn jacobi_eigen(mut a: Vec<Vec<f64>>) -> (Vec<f64>, Vec<Vec<f64>>) {
    let n = a.len();
    let mut v = vec![vec![0.0; n]; n];
    for (i, row) in v.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    for _sweep in 0..100 {
        let off: f64 = (0..n)
            .flat_map(|i| (0..n).filter(move |&j| j != i).map(move |j| (i, j)))
            .map(|(i, j)| a[i][j] * a[i][j])
            .sum();
        let diag: f64 = (0..n).map(|i| a[i][i] * a[i][i]).sum();
        if off <= 1e-24 * diag.max(1e-300) {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                if a[p][q].abs() < 1e-300 {
                    continue;
                }
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..n {
                    let (akp, akq) = (a[k][p], a[k][q]);
                    a[k][p] = c * akp - s * akq;
                    a[k][q] = s * akp + c * akq;
                }
                for k in 0..n {
                    let (apk, aqk) = (a[p][k], a[q][k]);
                    a[p][k] = c * apk - s * aqk;
                    a[q][k] = s * apk + c * aqk;
                }
                for row in v.iter_mut() {
                    let (vkp, vkq) = (row[p], row[q]);
                    row[p] = c * vkp - s * vkq;
                    row[q] = s * vkp + c * vkq;
                }
            }
        }
    }
    ((0..n).map(|i| a[i][i]).collect(), v)
}

/// Beam segment: length (m), EI (N·m²), mass per length (kg/m).
#[derive(Clone, Copy, Debug)]
pub struct Seg {
    pub len: f64,
    pub ei: f64,
    pub m: f64,
}

/// Natural frequencies (Hz) and mode shapes (transverse displacement at the
/// nodes, normalised to max |w| = 1) of a cantilever built from `segs`
/// (clamped at node 0).
pub fn cantilever_modes(segs: &[Seg], count: usize) -> (Vec<f64>, Vec<Vec<f64>>) {
    let ne = segs.len();
    let nd = 2 * (ne + 1);
    let mut k = vec![vec![0.0; nd]; nd];
    let mut m = vec![vec![0.0; nd]; nd];
    for (e, s) in segs.iter().enumerate() {
        let l = s.len;
        let ke = [
            [12.0, 6.0 * l, -12.0, 6.0 * l],
            [6.0 * l, 4.0 * l * l, -6.0 * l, 2.0 * l * l],
            [-12.0, -6.0 * l, 12.0, -6.0 * l],
            [6.0 * l, 2.0 * l * l, -6.0 * l, 4.0 * l * l],
        ];
        let me = [
            [156.0, 22.0 * l, 54.0, -13.0 * l],
            [22.0 * l, 4.0 * l * l, 13.0 * l, -3.0 * l * l],
            [54.0, 13.0 * l, 156.0, -22.0 * l],
            [-13.0 * l, -3.0 * l * l, -22.0 * l, 4.0 * l * l],
        ];
        let kf = s.ei / (l * l * l);
        let mf = s.m * l / 420.0;
        for a in 0..4 {
            for b in 0..4 {
                k[2 * e + a][2 * e + b] += kf * ke[a][b];
                m[2 * e + a][2 * e + b] += mf * me[a][b];
            }
        }
    }
    // Clamp node 0 (drop DOFs 0 and 1).
    let n = nd - 2;
    let kr: Vec<Vec<f64>> = (2..nd).map(|i| k[i][2..].to_vec()).collect();
    let mr: Vec<Vec<f64>> = (2..nd).map(|i| m[i][2..].to_vec()).collect();
    // Cholesky M = L Lᵀ.
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let s: f64 = mr[i][j] - (0..j).map(|q| l[i][q] * l[j][q]).sum::<f64>();
            if i == j {
                l[i][i] = s.max(1e-300).sqrt();
            } else {
                l[i][j] = s / l[j][j];
            }
        }
    }
    // Linv via forward substitution.
    let mut linv = vec![vec![0.0; n]; n];
    for c in 0..n {
        for i in 0..n {
            let mut s = if i == c { 1.0 } else { 0.0 };
            for q in 0..i {
                s -= l[i][q] * linv[q][c];
            }
            linv[i][c] = s / l[i][i];
        }
    }
    // C = Linv K Linvᵀ
    let mut t = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            t[i][j] = (0..=i).map(|q| linv[i][q] * kr[q][j]).sum();
        }
    }
    let mut c = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            c[i][j] = (0..=j).map(|q| t[i][q] * linv[j][q]).sum();
        }
    }
    let (vals, vecs) = jacobi_eigen(c);
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| vals[a].total_cmp(&vals[b]));
    let mut freqs = Vec::new();
    let mut shapes = Vec::new();
    for &o in order.iter().take(count) {
        let lam = vals[o].max(0.0);
        freqs.push(lam.sqrt() / (2.0 * std::f64::consts::PI));
        // φ = Linvᵀ y
        let y: Vec<f64> = (0..n).map(|i| vecs[i][o]).collect();
        let phi: Vec<f64> = (0..n).map(|i| (i..n).map(|q| linv[q][i] * y[q]).sum()).collect();
        let mut w = vec![0.0];
        w.extend((0..ne).map(|e| phi[2 * e]));
        let mx = w.iter().cloned().fold(0.0f64, |a, b| a.max(b.abs()));
        let sgn = if w.last().copied().unwrap_or(1.0) < 0.0 { -1.0 } else { 1.0 };
        shapes.push(w.iter().map(|v| sgn * v / mx.max(1e-300)).collect());
    }
    (freqs, shapes)
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ModalOut {
    pub part: String,
    /// Station positions along the temple from the clamp, mm.
    pub stations_mm: Vec<f64>,
    pub lateral_hz: Vec<f64>,
    pub vertical_hz: Vec<f64>,
    pub lateral_shapes: Vec<Vec<f64>>,
    pub vertical_shapes: Vec<Vec<f64>>,
    /// Per-segment lateral EI (N·mm²) and mass per length (g/mm).
    pub ei_lateral: Vec<f64>,
    pub ei_vertical: Vec<f64>,
    pub mass_g_mm: Vec<f64>,
    pub total_mass_g: f64,
    pub added_mass_g: f64,
    pub first_mode_hz: f64,
    pub notes: Vec<String>,
}

/// Slice a voxel grid along Y into beam segments (from `y_clamp` towards −Y).
pub fn temple_modal(
    g: &super::fea3d::Grid,
    part: &str,
    e_mpa: f64,
    density_g_cm3: f64,
    y_clamp: f64,
    pod_rear_y: f64,
    added_pod_g: f64,
) -> ModalOut {
    let h = g.h;
    // Per y-layer sums.
    let mut rows: Vec<(f64, [f64; 6])> = Vec::new(); // (y centre, [A, Σx dA, Σz dA, Σx² dA, Σz² dA, count])
    for j in (0..g.ny).rev() {
        let yc = g.origin[1] + h * (j as f64 + 0.5);
        if yc > y_clamp {
            continue;
        }
        let mut s = [0.0; 6];
        for k in 0..g.nz {
            for i in 0..g.nx {
                let e = g.eid(i, j, k);
                if !g.solid[e] {
                    continue;
                }
                let c = g.center(e);
                let da = g.rho[e] as f64 * h * h;
                s[0] += da;
                s[1] += da * c[0];
                s[2] += da * c[2];
                s[3] += da * (c[0] * c[0] + h * h / 12.0);
                s[4] += da * (c[2] * c[2] + h * h / 12.0);
                s[5] += 1.0;
            }
        }
        if s[0] > 0.0 {
            rows.push((yc, s));
        }
    }
    let mut out = ModalOut {
        part: part.into(),
        notes: vec![
            "Euler–Bernoulli beam along the temple axis (Y), clamped at the hinge stop; the ear-bend tail is treated as straight (its projection).".into(),
            "Open pod: the lid is added as mass only. Stiffness from the voxelised CAD section at every slice.".into(),
        ],
        ..Default::default()
    };
    if rows.len() < 4 {
        return out;
    }
    // Group slices into ~30 segments.
    let per = rows.len().div_ceil(30).max(1);
    let pod_len = (y_clamp - pod_rear_y).max(1.0);
    let mut segs = Vec::new();
    let mut x = 0.0;
    out.stations_mm.push(0.0);
    for chunk in rows.chunks(per) {
        let len = h * chunk.len() as f64; // mm
        let mut inv_ei_lat = 0.0;
        let mut inv_ei_ver = 0.0;
        let mut mass = 0.0; // g
        for (yc, s) in chunk {
            let a = s[0];
            let xb = s[1] / a;
            let zb = s[2] / a;
            let ix = s[3] - a * xb * xb; // lateral bending (displacement along X)
            let iz = s[4] - a * zb * zb;
            inv_ei_lat += h / (e_mpa * ix.max(1e-9));
            inv_ei_ver += h / (e_mpa * iz.max(1e-9));
            mass += a * h * density_g_cm3 / 1000.0;
            if *yc >= pod_rear_y {
                mass += added_pod_g * h / pod_len;
            }
        }
        let ei_lat = len / inv_ei_lat; // harmonic mean (segments in series)
        let ei_ver = len / inv_ei_ver;
        out.ei_lateral.push(ei_lat);
        out.ei_vertical.push(ei_ver);
        out.mass_g_mm.push(mass / len);
        out.total_mass_g += mass;
        segs.push((len, ei_lat, ei_ver, mass / len));
        x += len;
        out.stations_mm.push(x);
    }
    out.added_mass_g = added_pod_g;
    // SI: m, N·m², kg/m (g/mm = kg/m).
    let lat: Vec<Seg> = segs
        .iter()
        .map(|s| Seg {
            len: s.0 / 1000.0,
            ei: s.1 * 1e-6,
            m: s.3,
        })
        .collect();
    let ver: Vec<Seg> = segs
        .iter()
        .map(|s| Seg {
            len: s.0 / 1000.0,
            ei: s.2 * 1e-6,
            m: s.3,
        })
        .collect();
    let (fl, sl) = cantilever_modes(&lat, 3);
    let (fv, sv) = cantilever_modes(&ver, 3);
    out.first_mode_hz = fl.first().copied().unwrap_or(0.0).min(fv.first().copied().unwrap_or(f64::INFINITY));
    out.lateral_hz = fl;
    out.vertical_hz = fv;
    out.lateral_shapes = sl;
    out.vertical_shapes = sv;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_cantilever_first_modes() {
        // Steel strip 200 × 20 × 2 mm: EI = 207e9 × (0.02 × 0.002³ / 12), m = 7850 × 4e-5.
        let (l, ei, m) = (0.2, 207e9 * 0.02 * 0.002f64.powi(3) / 12.0, 7850.0 * 0.02 * 0.002);
        let segs: Vec<Seg> = (0..20).map(|_| Seg { len: l / 20.0, ei, m }).collect();
        let (f, shapes) = cantilever_modes(&segs, 2);
        let f1 = 1.875_104f64.powi(2) / (2.0 * std::f64::consts::PI) * (ei / (m * l.powi(4))).sqrt();
        let f2 = 4.694_091f64.powi(2) / (2.0 * std::f64::consts::PI) * (ei / (m * l.powi(4))).sqrt();
        assert!((f[0] - f1).abs() < 1e-4 * f1, "{} vs {f1}", f[0]);
        assert!((f[1] - f2).abs() < 1e-3 * f2, "{} vs {f2}", f[1]);
        assert!((shapes[0].last().unwrap() - 1.0).abs() < 1e-12);
    }
}
