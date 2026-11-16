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
    /// test below fails.
    ///
    /// AND A CORRECTION TO WHAT THIS COMMENT USED TO SAY. It claimed that
    /// dropping the `3.0 *` is survived by the linear ratio "which is why both
    /// tests exist", implying the cubic one catches it. It does not. BOTH of
    /// these tests compare RATIOS, and every constant factor cancels in a
    /// ratio, so the suite was green with this function returning k/3 -- driven
    /// by `tools/mutate.sh tests/mutations/mech-drop-the-coefficient/`, which
    /// reported SURVIVED. Layer 3 exists to produce absolute numbers a person
    /// acts on, so `absolute_stiffness_pins_the_coefficient` below pins one.
    #[test]
    fn force_is_linear_in_modulus() {
        let pla = cantilever_stiffness(3500.0, 8.9, 10.0, 1.5);
        let petg = cantilever_stiffness(2000.0, 8.9, 10.0, 1.5);
        assert!((petg / pla - 2000.0 / 3500.0).abs() < 1e-12);
    }

    /// THE COEFFICIENT, PINNED AGAINST A HAND-COMPUTED VALUE, because the two
    /// ratio tests cannot see it. Worked from k = 3EI/L^3 with I = w*t^3/12,
    /// for the PLA leaf of the harness incident, E 3500 MPa, L 8.9 mm,
    /// w 10 mm, t 1.5 mm:
    ///
    ///     I   = 10 * 1.5^3 / 12 = 33.75 / 12 = 2.8125 mm^4
    ///     L^3 = 8.9^3           = 704.969 mm^3
    ///     k   = 3 * 3500 * 2.8125 / 704.969 = 41.890139850121066 N/mm
    ///
    /// The tolerance is 1e-9 and not 1e-12: the expected value is written out
    /// to the digits a person can check by hand, so the arithmetic here must be
    /// allowed to differ from it in the last places. Dropping `3.0 *` gives
    /// 13.963379950040355, a factor of 3 out and nowhere near it.
    #[test]
    fn absolute_stiffness_pins_the_coefficient() {
        let k = cantilever_stiffness(3500.0, 8.9, 10.0, 1.5);
        assert!(
            (k - 41.890139850).abs() < 1e-9,
            "k = 3EI/L^3 should be 41.890139850 N/mm here, got {k}"
        );
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

// ---------------------------------------------------------------------------
// Beams. Spec §05.
// ---------------------------------------------------------------------------

/// How a beam is held, which changes its stiffness by a factor of four.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// Fixed at one end, free to rotate at the loaded end. k = 3EI/L^3.
    Cantilever,
    /// Fixed at one end and constrained against ROTATION at the other, which
    /// is what a leaf between two rigid bodies actually is. k = 12EI/L^3.
    ///
    /// FOUR TIMES STIFFER THAN A CANTILEVER, and this is the error that is
    /// easy to make and hard to see: both look like "a beam of length L", the
    /// formula differs only in a coefficient, and a design computed as a
    /// cantilever when it is fixed-guided is wrong by 300%.
    FixedGuided,
}

impl End {
    /// The coefficient in k = C*EI/L^3.
    pub fn coefficient(self) -> f64 {
        match self {
            End::Cantilever => 3.0,
            End::FixedGuided => 12.0,
        }
    }
}

/// A rectangular beam and how it is held.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Beam {
    /// Young's modulus, MPa.
    pub e_mpa: f64,
    /// Free length, mm.
    pub l_mm: f64,
    /// Width, mm: the dimension across the bending axis.
    pub w_mm: f64,
    /// Thickness, mm: the dimension ALONG the bending axis. This is the one
    /// that is cubed.
    pub t_mm: f64,
    pub end: End,
}

/// What a beam does at a given deflection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeamResult {
    /// N/mm.
    pub stiffness: f64,
    /// N at the requested deflection.
    pub force: f64,
    /// Peak bending stress, MPa.
    pub stress_mpa: f64,
}

/// Second moment of area for a rectangle bending about its thin axis.
pub fn second_moment(w_mm: f64, t_mm: f64) -> f64 {
    w_mm * t_mm.powi(3) / 12.0
}

impl Beam {
    fn check(&self) -> Result<(), String> {
        for (name, v) in [
            ("e_mpa", self.e_mpa),
            ("l_mm", self.l_mm),
            ("w_mm", self.w_mm),
            ("t_mm", self.t_mm),
        ] {
            if !v.is_finite() || v <= 0.0 {
                return Err(format!(
                    "{name} must be positive and finite, got {v}; refusing rather than \
                     returning a number with no meaning"
                ));
            }
        }
        Ok(())
    }

    /// k = C*E*I/L^3.
    pub fn stiffness(&self) -> Result<f64, String> {
        self.check()?;
        let i = second_moment(self.w_mm, self.t_mm);
        Ok(self.end.coefficient() * self.e_mpa * i / self.l_mm.powi(3))
    }

    /// Peak bending stress at a deflection, MPa.
    ///
    /// sigma = 3*E*delta*t / (2*L^2) for the cantilever; the fixed-guided case
    /// doubles it, because the same deflection is taken over half the
    /// effective length at each fixed end.
    pub fn stress_mpa(&self, deflection_mm: f64) -> Result<f64, String> {
        self.check()?;
        if !deflection_mm.is_finite() {
            return Err(format!("deflection must be finite, got {deflection_mm}"));
        }
        let base = 3.0 * self.e_mpa * deflection_mm * self.t_mm / (2.0 * self.l_mm.powi(2));
        Ok(match self.end {
            End::Cantilever => base,
            End::FixedGuided => base * 2.0,
        })
    }

    pub fn at(&self, deflection_mm: f64) -> Result<BeamResult, String> {
        let stiffness = self.stiffness()?;
        Ok(BeamResult {
            stiffness,
            force: stiffness * deflection_mm,
            stress_mpa: self.stress_mpa(deflection_mm)?,
        })
    }

    /// Margin against yield at a deflection: `yield / stress`.
    ///
    /// GREATER THAN ONE IS SURVIVAL. Returned as a ratio rather than a
    /// boolean, because "it fails" and "it has 1.02 of margin" are different
    /// news and a boolean throws the second away.
    pub fn yield_margin(&self, deflection_mm: f64, yield_mpa: f64) -> Result<f64, String> {
        if !yield_mpa.is_finite() || yield_mpa <= 0.0 {
            return Err(format!(
                "yield_mpa must be positive and finite, got {yield_mpa}"
            ));
        }
        let s = self.stress_mpa(deflection_mm)?.abs();
        if s == 0.0 {
            return Err(
                "zero stress at this deflection: there is no margin to report, \
                 which is not the same as infinite margin"
                    .into(),
            );
        }
        Ok(yield_mpa / s)
    }
}

#[cfg(test)]
mod beam_tests {
    use super::*;

    /// The harness clip: PLA, 8.9 mm free length, 10 mm wide, 1.5 mm thick.
    fn clip(e_mpa: f64, t_mm: f64) -> Beam {
        Beam {
            e_mpa,
            l_mm: 8.9,
            w_mm: 10.0,
            t_mm,
            end: End::Cantilever,
        }
    }

    /// THE PETG INCIDENT AS A VECTOR. Force is linear in E, so a clip designed
    /// at 3500 MPa and printed in PETG at 2000 delivers 2000/3500 of its
    /// force: 3.17 N becomes 1.81 N.
    #[test]
    fn the_clip_loses_its_force_when_the_material_changes() {
        let d = 0.35;
        let pla = clip(3500.0, 1.5).at(d).expect("valid");
        let petg = clip(2000.0, 1.5).at(d).expect("valid");
        assert!(
            (petg.force / pla.force - 2000.0 / 3500.0).abs() < 1e-12,
            "force must be exactly linear in E"
        );
        // Anchored to the logged pair rather than to whatever this code says.
        let scaled: f64 = 3.17 * 2000.0 / 3500.0;
        assert!(
            (scaled - 1.8114).abs() < 1e-3,
            "the logged 3.17 N -> 1.81 N, got {scaled}"
        );
    }

    /// AND THE QUANTIZATION INCIDENT, which is the same clip losing 30% a
    /// second way. 0.9 mm asked, 0.80 mm printed.
    #[test]
    fn the_same_clip_loses_thirty_percent_again_to_quantization() {
        let d = 0.35;
        let asked = clip(3500.0, 0.9).at(d).expect("valid").force;
        let printed = clip(3500.0, 0.8).at(d).expect("valid").force;
        let ratio = printed / asked;
        assert!(
            (ratio - (0.8f64 / 0.9).powi(3)).abs() < 1e-12,
            "force follows thickness CUBED"
        );
        assert!(
            (ratio - 0.702_332).abs() < 1e-5,
            "a 29.77% loss, got {ratio}"
        );
    }

    /// FIXED-GUIDED IS FOUR TIMES A CANTILEVER. Both look like "a beam of
    /// length L"; computing one as the other is wrong by 300%.
    #[test]
    fn a_fixed_guided_beam_is_four_times_the_cantilever() {
        let mut b = clip(3500.0, 1.5);
        let cant = b.stiffness().expect("valid");
        b.end = End::FixedGuided;
        let fg = b.stiffness().expect("valid");
        assert!(
            (fg / cant - 4.0).abs() < 1e-12,
            "12EI/L^3 over 3EI/L^3 is 4, got {}",
            fg / cant
        );
    }

    /// The absolute value, hand-computed, so the coefficient is pinned and not
    /// only the ratio. I = 10*1.5^3/12 = 2.8125; L^3 = 704.969;
    /// k = 3*3500*2.8125/704.969 = 41.890139850121066.
    #[test]
    fn the_cantilever_coefficient_is_pinned_against_arithmetic() {
        let k = clip(3500.0, 1.5).stiffness().expect("valid");
        assert!(
            (k - 41.890_139_850).abs() < 1e-9,
            "3EI/L^3 should be 41.890139850 N/mm, got {k}"
        );
    }

    #[test]
    fn the_second_moment_is_w_t_cubed_over_twelve() {
        assert!((second_moment(10.0, 1.5) - 2.8125).abs() < 1e-12);
        // cubed in t, linear in w
        assert!((second_moment(20.0, 1.5) / second_moment(10.0, 1.5) - 2.0).abs() < 1e-12);
        assert!((second_moment(10.0, 3.0) / second_moment(10.0, 1.5) - 8.0).abs() < 1e-12);
    }

    /// Stress is linear in deflection and in thickness, and goes as 1/L^2.
    #[test]
    fn stress_follows_its_own_law() {
        let b = clip(3500.0, 1.5);
        let s1 = b.stress_mpa(0.35).expect("valid");
        let s2 = b.stress_mpa(0.70).expect("valid");
        assert!((s2 / s1 - 2.0).abs() < 1e-12, "linear in deflection");
        let want = 3.0 * 3500.0 * 0.35 * 1.5 / (2.0 * 8.9f64.powi(2));
        assert!((s1 - want).abs() < 1e-9);
    }

    /// A fixed-guided beam at the same deflection is stressed TWICE as hard,
    /// which is the half that gets forgotten when only stiffness is compared.
    #[test]
    fn fixed_guided_doubles_the_stress_as_well_as_quadrupling_the_stiffness() {
        let mut b = clip(3500.0, 1.5);
        let sc = b.stress_mpa(0.35).expect("valid");
        b.end = End::FixedGuided;
        assert!((b.stress_mpa(0.35).expect("valid") / sc - 2.0).abs() < 1e-12);
    }

    /// A RATIO, NOT A BOOLEAN. "it fails" and "it has 1.02 of margin" are
    /// different news, and a boolean throws the second away.
    #[test]
    fn the_yield_margin_is_a_ratio_and_greater_than_one_is_survival() {
        let b = clip(3500.0, 1.5);
        let s = b.stress_mpa(0.35).expect("valid");
        let m = b.yield_margin(0.35, s * 2.0).expect("valid");
        assert!((m - 2.0).abs() < 1e-9, "twice the stress is 2.0 of margin");
        assert!(
            b.yield_margin(0.35, s * 0.5).expect("valid") < 1.0,
            "it yields"
        );
    }

    /// Zero stress has NO margin to report, which is not the same as infinite
    /// margin, and returning infinity would read as the safest possible part.
    #[test]
    fn zero_stress_refuses_rather_than_reporting_infinite_margin() {
        let e = clip(3500.0, 1.5)
            .yield_margin(0.0, 50.0)
            .expect_err("must refuse");
        assert!(e.contains("not the same as infinite"), "{e}");
    }

    #[test]
    fn nonpositive_and_nan_geometry_is_refused() {
        for b in [
            clip(0.0, 1.5),
            clip(3500.0, 0.0),
            clip(f64::NAN, 1.5),
            Beam {
                l_mm: -1.0,
                ..clip(3500.0, 1.5)
            },
        ] {
            assert!(b.stiffness().is_err(), "{b:?} must be refused");
        }
    }
}
