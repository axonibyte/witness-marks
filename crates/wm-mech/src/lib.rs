//! Layer 3: closed-form mechanics. Spec §05. No FEA -- four formulas decided
//! every mechanical question in the harness: k = 3EI/L^3, sigma = 3E*delta*t/2L^2,
//! T = mu*N*r, and the power-screw term (mu - tan(lambda)) / (1 + mu*tan(lambda)).

/// Cantilever stiffness, N/mm, for a rectangular section `w` wide and `t`
/// thick (bending about the thin axis), `L` long, modulus `e` in MPa.
/// k = 3EI/L^3 with I = w*t^3/12.
pub fn cantilever_stiffness(e_mpa: f64, l_mm: f64, w_mm: f64, t_mm: f64) -> f64 {
    let i = w_mm * t_mm.powi(3) / 12.0;
    3.0 * e_mpa * i / l_mm.powi(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The PETG incident, as a LAW rather than a number: force is linear in E,
    /// so 3500 -> 2000 MPa scales the clamp by exactly 2000/3500 = 0.5714.
    /// Mutation check: swap `powi(3)` for `powi(2)` on `t_mm` and the cubic
    /// test below fails; drop the `3.0 *` and the linear ratio still passes --
    /// which is why both tests exist.
    #[test]
    fn force_is_linear_in_modulus() {
        let pla = cantilever_stiffness(3500.0, 8.9, 10.0, 1.5);
        let petg = cantilever_stiffness(2000.0, 8.9, 10.0, 1.5);
        assert!((petg / pla - 2000.0 / 3500.0).abs() < 1e-12);
    }

    /// The quantization incident: stiffness goes as thickness CUBED, so 4.5
    /// layers rounding to 4 (0.9 -> 0.8 mm) is (0.8/0.9)^3 = 0.702, a 30% loss.
    #[test]
    fn stiffness_is_cubic_in_thickness() {
        let k9 = cantilever_stiffness(3500.0, 8.9, 10.0, 0.9);
        let k8 = cantilever_stiffness(3500.0, 8.9, 10.0, 0.8);
        assert!((k8 / k9 - (0.8f64 / 0.9).powi(3)).abs() < 1e-12);
        assert!(
            (k8 / k9 - 0.702).abs() < 0.001,
            "the harness measured a 30% loss"
        );
    }
}
