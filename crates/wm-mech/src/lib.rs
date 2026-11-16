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

// ---------------------------------------------------------------------------
// Power screws. Spec §05.
// ---------------------------------------------------------------------------

/// A square-thread power screw: pitch, mean radius, friction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Screw {
    /// Axial advance per turn, mm. For a single start this is the pitch.
    pub lead_mm: f64,
    /// Mean thread radius, mm.
    pub r_mm: f64,
    /// Coefficient of friction at the thread flank.
    pub mu: f64,
}

/// What a screw does under an axial load.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrewResult {
    /// Helix angle, degrees.
    pub helix_deg: f64,
    /// N*mm to drive the load along the screw's own direction of advance.
    pub torque_raise: f64,
    /// N*mm to let it back off. POSITIVE means torque is needed to lower it,
    /// which is the self-locking case; NEGATIVE means the load drives it back
    /// on its own.
    pub torque_lower: f64,
    /// `mu > tan(lambda)`.
    pub self_locking: bool,
    /// How far from the self-locking boundary: `mu / tan(lambda)`. 1.0 is
    /// exactly on it.
    pub locking_margin: f64,
}

impl Screw {
    /// Helix angle in radians: `atan(lead / (2*pi*r))`.
    pub fn helix_rad(&self) -> Result<f64, String> {
        for (n, v) in [("lead_mm", self.lead_mm), ("r_mm", self.r_mm)] {
            if !v.is_finite() || v <= 0.0 {
                return Err(format!("{n} must be positive and finite, got {v}"));
            }
        }
        if !self.mu.is_finite() || self.mu < 0.0 {
            return Err(format!(
                "mu must be finite and non-negative, got {}; a negative friction \
                 coefficient is not a frictionless screw, it is a sign error",
                self.mu
            ));
        }
        Ok((self.lead_mm / (2.0 * std::f64::consts::PI * self.r_mm)).atan())
    }

    /// Torque and the self-locking verdict under axial load `f_n` newtons.
    ///
    /// # The power-screw terms
    ///
    /// raise: `F*r*(tan(l) + mu) / (1 - mu*tan(l))`
    /// lower: `F*r*(mu - tan(l)) / (1 + mu*tan(l))`
    ///
    /// # Self-locking is `mu > tan(lambda)`, and the sign of the lower torque
    ///
    /// They are the same statement: the lower term's numerator is
    /// `mu - tan(l)`, so a positive lowering torque means the load cannot
    /// drive the screw back on its own. Reporting the boolean AND the signed
    /// torque is deliberate -- the boolean is what a caller asks for and the
    /// sign is what makes it checkable.
    ///
    /// # The margin matters more than the verdict
    ///
    /// The harness screw is pitch 5 at r = 34, so lambda = 1.3408 degrees and
    /// tan(lambda) = 0.0234. Against a printed-plastic mu around 0.3 that is a
    /// margin of nearly 13, which is not a close call. At mu = 0.02 it is
    /// 0.855 and the thing back-drives. A boolean alone would make those look
    /// like neighbouring cases.
    pub fn under(&self, f_n: f64) -> Result<ScrewResult, String> {
        let lam = self.helix_rad()?;
        if !f_n.is_finite() || f_n < 0.0 {
            return Err(format!(
                "axial load must be finite and non-negative, got {f_n}"
            ));
        }
        let t = lam.tan();
        let denom_raise = 1.0 - self.mu * t;
        if denom_raise <= 0.0 {
            return Err(format!(
                "mu*tan(lambda) = {} reaches 1: the raise term is undefined and this is \
                 not a screw any more",
                self.mu * t
            ));
        }
        Ok(ScrewResult {
            helix_deg: lam.to_degrees(),
            torque_raise: f_n * self.r_mm * (t + self.mu) / denom_raise,
            torque_lower: f_n * self.r_mm * (self.mu - t) / (1.0 + self.mu * t),
            self_locking: self.mu > t,
            locking_margin: if t == 0.0 { f64::INFINITY } else { self.mu / t },
        })
    }
}

#[cfg(test)]
mod screw_tests {
    use super::*;

    /// The can-cutter floor thread: pitch 5 at a mean radius of 34.
    fn harness(mu: f64) -> Screw {
        Screw {
            lead_mm: 5.0,
            r_mm: 34.0,
            mu,
        }
    }

    /// THE LOGGED FIXTURE: lambda = 1.34 degrees.
    #[test]
    fn the_harness_threads_helix_angle() {
        let r = harness(0.3).under(100.0).expect("valid");
        assert!(
            (r.helix_deg - 1.340_771).abs() < 1e-5,
            "pitch 5 at r 34 is 1.340771 deg, got {}",
            r.helix_deg
        );
    }

    /// A shallow helix against ordinary printed-plastic friction is not a
    /// close call: the margin is nearly 13.
    #[test]
    fn printed_plastic_friction_locks_this_thread_comfortably() {
        let r = harness(0.3).under(100.0).expect("valid");
        assert!(r.self_locking);
        assert!(
            r.locking_margin > 12.0,
            "mu 0.3 against tan(lambda) 0.0234 is a margin near 13, got {}",
            r.locking_margin
        );
        assert!(
            r.torque_lower > 0.0,
            "self-locking means torque is NEEDED to lower it"
        );
    }

    /// THE BOOLEAN AND THE SIGN ARE THE SAME STATEMENT, and the arm checks
    /// they agree. If they ever disagree, one of them is lying.
    #[test]
    fn the_verdict_and_the_sign_of_the_lowering_torque_always_agree() {
        for mu in [0.0, 0.01, 0.02, 0.0234, 0.05, 0.3, 0.8] {
            let r = harness(mu).under(100.0).expect("valid");
            assert_eq!(
                r.self_locking,
                r.torque_lower > 0.0,
                "mu {mu}: verdict {} but lowering torque {}",
                r.self_locking,
                r.torque_lower
            );
        }
    }

    /// A SLIPPERY THREAD BACK-DRIVES, and the lowering torque goes negative:
    /// the load does the work.
    #[test]
    fn a_slippery_thread_back_drives() {
        let r = harness(0.02).under(100.0).expect("valid");
        assert!(!r.self_locking);
        assert!(r.torque_lower < 0.0, "the load drives it back on its own");
        assert!(
            r.locking_margin < 1.0,
            "below the boundary, got {}",
            r.locking_margin
        );
    }

    /// THE MARGIN IS THE NUMBER THAT MATTERS. A boolean makes mu 0.3 and
    /// mu 0.024 look like neighbouring cases; they are 13 and 1.0.
    #[test]
    fn the_margin_separates_a_safe_case_from_a_marginal_one() {
        let safe = harness(0.3).under(100.0).expect("valid").locking_margin;
        let edge = harness(0.0234).under(100.0).expect("valid").locking_margin;
        assert!(safe > 12.0 && (edge - 1.0).abs() < 0.01);
        assert!(
            safe / edge > 10.0,
            "a boolean would report both as 'locking'"
        );
    }

    /// A STEEPER THREAD IS THE ONLY PLACE THE RULE IS CHECKABLE, and this arm
    /// exists because a mutant proved it. Self-locking is `mu > tan(lambda)`,
    /// not `mu > lambda`, and on the harness screw those agree to five decimal
    /// places: lambda is 0.0234009 rad against a tangent of 0.0234051, a
    /// difference of 4.3e-06. Every other arm here passes under either
    /// formula.
    ///
    /// THE FIRST VERSION OF THIS ARM WAS UNPHYSICAL and the code refused it,
    /// correctly. At a 57.9 degree helix with mu 1.2, `mu*tan(lambda)` is 1.91
    /// and the raise term's denominator goes negative -- that is not a screw
    /// any more, and the guard says so. A discriminating case has to be steep
    /// enough that lambda and its tangent diverge AND shallow enough that the
    /// product stays under 1.
    ///
    /// A 30 degree helix: lambda 0.5236 rad, tan(lambda) 0.5774. A mu of
    /// 0.5505 sits BETWEEN them, and `mu*tan(lambda)` is 0.318. The correct
    /// rule says it does not lock; the mistaken one says it does.
    #[test]
    fn a_steeper_thread_distinguishes_the_tangent_from_the_angle() {
        let lam = 30.0f64.to_radians();
        let steep = Screw {
            lead_mm: lam.tan() * 2.0 * std::f64::consts::PI * 10.0,
            r_mm: 10.0,
            mu: 0.5505,
        };
        let r = steep
            .under(100.0)
            .expect("physical: mu*tan(lambda) is 0.318");
        assert!((r.helix_deg - 30.0).abs() < 1e-6, "got {}", r.helix_deg);
        assert!(
            steep.mu > lam,
            "the mistaken rule, comparing against lambda in radians, would say LOCKS"
        );
        assert!(
            !r.self_locking,
            "mu 0.5505 is below tan(lambda) 0.5774, so it does NOT lock"
        );
        assert!(r.torque_lower < 0.0, "and the sign agrees");
    }

    /// Raising always costs more than lowering, and both scale with the load.
    #[test]
    fn torque_scales_with_the_load_and_raising_costs_more() {
        let a = harness(0.3).under(100.0).expect("valid");
        let b = harness(0.3).under(200.0).expect("valid");
        assert!((b.torque_raise / a.torque_raise - 2.0).abs() < 1e-12);
        assert!(a.torque_raise > a.torque_lower);
    }

    /// A frictionless screw cannot lock, and its lowering torque is the
    /// negative of its raising torque.
    #[test]
    fn a_frictionless_screw_never_locks() {
        let r = harness(0.0).under(100.0).expect("valid");
        assert!(!r.self_locking);
        assert_eq!(r.locking_margin, 0.0);
        assert!((r.torque_raise + r.torque_lower).abs() < 1e-9);
    }

    /// A NEGATIVE mu IS A SIGN ERROR, NOT A FRICTIONLESS SCREW, and the
    /// refusal says so rather than quietly computing a backwards answer.
    #[test]
    fn a_negative_friction_coefficient_is_refused_as_a_sign_error() {
        let e = Screw {
            mu: -0.3,
            ..harness(0.3)
        }
        .under(100.0)
        .expect_err("must refuse");
        assert!(e.contains("sign error"), "{e}");
    }

    #[test]
    fn nonpositive_geometry_and_loads_are_refused() {
        assert!(Screw {
            lead_mm: 0.0,
            ..harness(0.3)
        }
        .under(100.0)
        .is_err());
        assert!(Screw {
            r_mm: f64::NAN,
            ..harness(0.3)
        }
        .under(100.0)
        .is_err());
        assert!(harness(0.3).under(-1.0).is_err());
    }
}

// ---------------------------------------------------------------------------
// Friction chains: which interface gives way first. Spec §05.
// ---------------------------------------------------------------------------

/// One friction interface in a chain.
#[derive(Debug, Clone, PartialEq)]
pub struct Interface {
    pub name: String,
    /// EFFECTIVE friction radius, mm. For an annular land this is
    /// [`annular_effective_radius`] and NOT the mean of the two radii: the
    /// outer part of the land carries more torque per unit area because it is
    /// further out, and the mean under-reports it.
    pub r_mm: f64,
    pub mu: f64,
    /// Normal force, N.
    pub normal_n: f64,
}

impl Interface {
    /// Torque this interface can carry before it slips, N*mm.
    pub fn capacity(&self) -> Result<f64, String> {
        for (n, v) in [
            ("r_mm", self.r_mm),
            ("mu", self.mu),
            ("normal_n", self.normal_n),
        ] {
            if !v.is_finite() || v < 0.0 {
                return Err(format!(
                    "{} : {n} must be finite and non-negative, got {v}",
                    self.name
                ));
            }
        }
        Ok(self.mu * self.normal_n * self.r_mm)
    }
}

/// The effective friction radius of an annular land under uniform pressure.
///
/// `(2/3) * (ro^3 - ri^3) / (ro^2 - ri^2)`
///
/// NOT THE MEAN RADIUS. For a 5 to 11 mm land the mean is 8.0 and the
/// effective radius is 8.375, which is 4.7% higher: area further out carries
/// proportionally more torque. Using the mean under-reports every capacity.
pub fn annular_effective_radius(ri_mm: f64, ro_mm: f64) -> Result<f64, String> {
    if !ri_mm.is_finite() || !ro_mm.is_finite() || ri_mm < 0.0 || ro_mm <= ri_mm {
        return Err(format!("need 0 <= ri < ro, got {ri_mm} and {ro_mm}"));
    }
    Ok((2.0 / 3.0) * (ro_mm.powi(3) - ri_mm.powi(3)) / (ro_mm.powi(2) - ri_mm.powi(2)))
}

/// Which interface in a chain gives way first, and by how much.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainResult {
    /// The name of the interface that slips first: the lowest capacity.
    pub first_to_slip: String,
    /// Its capacity, N*mm.
    pub weakest_capacity: f64,
    /// Every interface's capacity as a multiple of the weakest. The weakest is
    /// 1.0 by construction.
    pub margins: Vec<(String, f64)>,
}

/// Rank a chain of interfaces by the torque each can carry.
///
/// # Load and friction cancel when they are shared
///
/// If every interface in a chain sees the same normal force and the same
/// friction coefficient, `mu*N*r` reduces to `r`, and the ranking is PURELY
/// GEOMETRIC. That is the useful case and it is why the harness analysis could
/// be done without knowing either number. An arm asserts the cancellation
/// rather than leaving it as a remark.
pub fn rank(chain: &[Interface]) -> Result<ChainResult, String> {
    if chain.is_empty() {
        return Err(
            "an empty chain has no interface to slip; refusing rather than \
                    reporting that nothing gives way"
                .into(),
        );
    }
    let mut caps: Vec<(String, f64)> = Vec::with_capacity(chain.len());
    for i in chain {
        caps.push((i.name.clone(), i.capacity()?));
    }
    let weakest = caps.iter().map(|(_, c)| *c).fold(f64::INFINITY, f64::min);
    if weakest <= 0.0 {
        return Err(format!(
            "the weakest interface carries {weakest} N*mm: with zero capacity every \
             margin is infinite, which is not a useful answer"
        ));
    }
    let first = caps
        .iter()
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(n, _)| n.clone())
        .unwrap_or_default();
    Ok(ChainResult {
        first_to_slip: first,
        weakest_capacity: weakest,
        margins: caps.into_iter().map(|(n, c)| (n, c / weakest)).collect(),
    })
}

#[cfg(test)]
mod friction_tests {
    use super::*;

    fn iface(name: &str, r: f64) -> Interface {
        Interface {
            name: name.into(),
            r_mm: r,
            mu: 0.3,
            normal_n: 20.0,
        }
    }

    /// The log's 8.4 is the EFFECTIVE radius of a 5 to 11 mm land, not the
    /// mean of 8.0. The log did the right physics and this pins it.
    #[test]
    fn the_logged_land_radius_is_the_effective_one_not_the_mean() {
        let eff = annular_effective_radius(5.0, 11.0).expect("valid");
        assert!(
            (eff - 8.375).abs() < 1e-9,
            "a 5-11 land is 8.375 effective, got {eff}"
        );
        assert!(
            (eff - 8.0).abs() > 0.3,
            "and it is NOT the mean radius of 8.0"
        );
    }

    /// THE PUCK AGAINST THE THREAD, computed from the log's own radii.
    ///
    /// A DISCREPANCY IN THE LOG, recorded rather than reconciled: buildability
    /// entry 72 says the thread out-resists the puck's land "3.7:1" with the
    /// land at 8.4 mm and the thread at 34 mm. Those radii give **4.05:1**.
    /// Reaching 3.7 would need a thread radius of 31.08 or a land radius of
    /// 9.19, and the entry states neither. The entry's OTHER ratio, the fused
    /// 34 against 26, does check out at 1.308. One figure in that sentence
    /// does not follow from the others, and this test asserts the arithmetic
    /// rather than the prose.
    #[test]
    fn the_thread_out_resists_the_pucks_land_by_the_ratio_of_their_radii() {
        let chain = [iface("puck thrust land", 8.4), iface("thread", 34.0)];
        let r = rank(&chain).expect("valid");
        assert_eq!(r.first_to_slip, "puck thrust land");
        let thread = r
            .margins
            .iter()
            .find(|(n, _)| n == "thread")
            .expect("in the chain")
            .1;
        assert!(
            (thread - 34.0 / 8.4).abs() < 1e-12,
            "the margin is the ratio of radii"
        );
        assert!(
            (thread - 4.047_6).abs() < 1e-3,
            "which is 4.05:1, NOT the 3.7:1 the log states; got {thread}"
        );
    }

    /// The fused case, which the log gets right: 34 against 26 is 1.308.
    #[test]
    fn the_fused_case_matches_the_log() {
        let chain = [iface("can on the carrier top", 26.0), iface("thread", 34.0)];
        let r = rank(&chain).expect("valid");
        assert_eq!(r.first_to_slip, "can on the carrier top");
        let thread = r
            .margins
            .iter()
            .find(|(n, _)| n == "thread")
            .expect("there")
            .1;
        assert!(
            (thread - 1.307_7).abs() < 1e-3,
            "the log says about 1.3:1, got {thread}"
        );
    }

    /// LOAD AND FRICTION CANCEL when they are shared, so the ranking is PURELY
    /// GEOMETRIC. This is why the harness analysis could be done without
    /// knowing either number, and it is asserted rather than remarked upon.
    #[test]
    fn a_shared_load_and_friction_cancel_out_of_the_margins() {
        let a = [iface("small", 8.4), iface("large", 34.0)];
        let mut b = a.clone();
        for i in b.iter_mut() {
            i.mu = 0.05;
            i.normal_n = 900.0;
        }
        let ra = rank(&a).expect("valid");
        let rb = rank(&b).expect("valid");
        assert_eq!(ra.first_to_slip, rb.first_to_slip);
        for ((na, ma), (nb, mb)) in ra.margins.iter().zip(rb.margins.iter()) {
            assert_eq!(na, nb);
            assert!(
                (ma - mb).abs() < 1e-12,
                "a 18x change in load and a 6x change in mu must not move the margin"
            );
        }
        assert!(
            ra.weakest_capacity != rb.weakest_capacity,
            "the capacity DOES move"
        );
    }

    /// An interface with a smaller radius but far more load can still win,
    /// which is the case the geometric shortcut does NOT cover.
    #[test]
    fn an_unshared_load_breaks_the_geometric_shortcut() {
        let chain = [
            Interface {
                normal_n: 500.0,
                ..iface("small but clamped hard", 8.4)
            },
            iface("large but barely loaded", 34.0),
        ];
        let r = rank(&chain).expect("valid");
        assert_eq!(
            r.first_to_slip, "large but barely loaded",
            "with loads unshared the bigger radius can still be the weak link"
        );
    }

    /// AN EMPTY CHAIN HAS NOTHING TO SLIP, and reporting that nothing gives
    /// way would read as a part that cannot fail.
    #[test]
    fn an_empty_chain_is_refused_rather_than_reported_as_safe() {
        let e = rank(&[]).expect_err("must refuse");
        assert!(e.contains("refusing"), "{e}");
    }

    /// A zero-capacity interface makes every margin infinite, which is not a
    /// useful answer and is refused.
    #[test]
    fn a_zero_capacity_interface_is_refused() {
        let chain = [
            Interface {
                normal_n: 0.0,
                ..iface("unloaded", 8.4)
            },
            iface("thread", 34.0),
        ];
        assert!(rank(&chain).is_err());
    }

    #[test]
    fn the_effective_radius_refuses_a_backwards_annulus() {
        assert!(annular_effective_radius(11.0, 5.0).is_err());
        assert!(annular_effective_radius(5.0, 5.0).is_err());
        assert!(annular_effective_radius(f64::NAN, 11.0).is_err());
    }
}

// ---------------------------------------------------------------------------
// Press fits. Spec §05.
// ---------------------------------------------------------------------------

/// A cylindrical press fit: a solid shaft in a hub of the same material.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PressFit {
    /// DIAMETRAL interference, mm: how much larger the shaft is than the bore.
    /// Diametral and not radial, because that is what a drawing states and
    /// what a caliper measures, and halving it is the first thing everybody
    /// forgets.
    pub interference_mm: f64,
    /// Nominal interface diameter, mm.
    pub diameter_mm: f64,
    /// Hub outer diameter, mm. A hub thin at the interface takes the
    /// interference by stretching instead of by squeezing.
    pub hub_outer_mm: f64,
    /// Engagement length, mm.
    pub length_mm: f64,
    pub e_mpa: f64,
    pub mu: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PressResult {
    /// Interface pressure, MPa.
    pub pressure_mpa: f64,
    /// Axial force to push it together or pull it apart, N.
    pub axial_force_n: f64,
    /// Torque it can transmit before slipping, N*mm.
    pub torque_capacity: f64,
}

impl PressFit {
    /// Lamé, same material both sides, solid shaft, plane stress:
    ///
    /// `p = (delta_radial * E / r) * (ro^2 - r^2) / (2 * ro^2)`
    ///
    /// # What this assumes, stated because it is easy to apply anyway
    ///
    /// Same modulus on both sides, a SOLID shaft, and an isotropic hub. A
    /// printed part is none of those three strictly: it is layered, so its
    /// modulus across layers differs from along them. The number is a starting
    /// point and its provenance is `Derived` at best.
    pub fn solve(&self) -> Result<PressResult, String> {
        for (n, v) in [
            ("interference_mm", self.interference_mm),
            ("diameter_mm", self.diameter_mm),
            ("hub_outer_mm", self.hub_outer_mm),
            ("length_mm", self.length_mm),
            ("e_mpa", self.e_mpa),
        ] {
            if !v.is_finite() || v <= 0.0 {
                return Err(format!("{n} must be positive and finite, got {v}"));
            }
        }
        if !self.mu.is_finite() || self.mu < 0.0 {
            return Err(format!(
                "mu must be finite and non-negative, got {}",
                self.mu
            ));
        }
        if self.hub_outer_mm <= self.diameter_mm {
            return Err(format!(
                "hub_outer_mm {} is not larger than diameter_mm {}: there is no hub wall \
                 to take the interference, and the formula would return a pressure of zero \
                 or less for a joint that physically cannot be assembled",
                self.hub_outer_mm, self.diameter_mm
            ));
        }
        let r = self.diameter_mm / 2.0;
        let ro = self.hub_outer_mm / 2.0;
        // HALVED HERE, ONCE. The input is diametral; Lamé wants radial.
        let delta_radial = self.interference_mm / 2.0;
        let pressure =
            (delta_radial * self.e_mpa / r) * (ro.powi(2) - r.powi(2)) / (2.0 * ro.powi(2));
        let area = std::f64::consts::PI * self.diameter_mm * self.length_mm;
        let axial = self.mu * pressure * area;
        Ok(PressResult {
            pressure_mpa: pressure,
            axial_force_n: axial,
            torque_capacity: axial * r,
        })
    }
}

/// A SLOT IS NOT A PRESS FIT, and this function exists to say so.
///
/// The harness fixture that prompted this issue is the blade's wedge pocket:
/// two parallel walls squeezing a flat blade with `blade_press = 0.05` of
/// interference per wall. Lamé does not apply. There is no hoop stress in a
/// flat wall, nothing goes into tension around a circumference, and the
/// pressure is set by how stiff that wall is IN BENDING -- which is a beam
/// problem and is already solved by [`Beam`].
///
/// Applying the cylindrical formula anyway would produce a confident number
/// with the wrong physics behind it, which is worse than no number. So this
/// refuses and names where to go.
pub fn slot_press_is_not_a_cylinder(interference_per_wall_mm: f64) -> Result<f64, String> {
    Err(format!(
        "a slot squeezing a flat part by {interference_per_wall_mm} mm per wall is NOT a \
         Lame press fit: a flat wall has no hoop stress and nothing goes into tension \
         around a circumference. The pressure is set by the wall's BENDING stiffness -- \
         model each wall with Beam::at(deflection) and use its force over the contact \
         area. Refusing rather than returning a cylindrical number for a flat problem."
    ))
}

#[cfg(test)]
mod press_tests {
    use super::*;

    fn fit() -> PressFit {
        PressFit {
            interference_mm: 0.10,
            diameter_mm: 10.0,
            hub_outer_mm: 20.0,
            length_mm: 8.0,
            e_mpa: 2000.0,
            mu: 0.3,
        }
    }

    /// Hand-computed. delta_radial 0.05, r 5, ro 10:
    /// p = (0.05*2000/5) * (100-25)/(2*100) = 20 * 0.375 = 7.5 MPa.
    #[test]
    fn the_pressure_is_pinned_against_arithmetic() {
        let r = fit().solve().expect("valid");
        assert!(
            (r.pressure_mpa - 7.5).abs() < 1e-12,
            "expected 7.5 MPa, got {}",
            r.pressure_mpa
        );
    }

    /// THE INTERFERENCE IS DIAMETRAL AND IS HALVED EXACTLY ONCE. Forgetting
    /// the halving doubles every force in the answer.
    #[test]
    fn the_interference_is_diametral_and_halved_once() {
        let mut f = fit();
        f.interference_mm = 0.20;
        let doubled = f.solve().expect("valid");
        assert!(
            (doubled.pressure_mpa - 15.0).abs() < 1e-12,
            "twice the interference is twice the pressure, got {}",
            doubled.pressure_mpa
        );
    }

    /// A thin hub takes the interference by STRETCHING rather than squeezing,
    /// so the pressure falls toward zero as the wall vanishes.
    #[test]
    fn a_thinner_hub_grips_less() {
        let thick = PressFit {
            hub_outer_mm: 40.0,
            ..fit()
        }
        .solve()
        .expect("valid");
        let thin = PressFit {
            hub_outer_mm: 10.5,
            ..fit()
        }
        .solve()
        .expect("valid");
        assert!(
            thin.pressure_mpa < thick.pressure_mpa * 0.2,
            "a nearly-absent wall must grip far less: {} vs {}",
            thin.pressure_mpa,
            thick.pressure_mpa
        );
    }

    /// A HUB WITH NO WALL CANNOT BE ASSEMBLED, and the formula would return
    /// zero or negative pressure for it rather than saying so.
    #[test]
    fn a_hub_no_larger_than_the_shaft_is_refused() {
        let e = PressFit {
            hub_outer_mm: 10.0,
            ..fit()
        }
        .solve()
        .expect_err("must refuse");
        assert!(e.contains("no hub wall"), "{e}");
        assert!(PressFit {
            hub_outer_mm: 9.0,
            ..fit()
        }
        .solve()
        .is_err());
    }

    #[test]
    fn force_and_torque_follow_the_pressure() {
        let r = fit().solve().expect("valid");
        let area = std::f64::consts::PI * 10.0 * 8.0;
        assert!((r.axial_force_n - 0.3 * 7.5 * area).abs() < 1e-9);
        assert!((r.torque_capacity - r.axial_force_n * 5.0).abs() < 1e-9);
    }

    /// A frictionless fit holds nothing axially, however hard it squeezes.
    #[test]
    fn zero_friction_holds_nothing() {
        let r = PressFit { mu: 0.0, ..fit() }.solve().expect("valid");
        assert!(r.pressure_mpa > 0.0, "it still squeezes");
        assert_eq!(r.axial_force_n, 0.0, "and holds nothing");
    }

    /// THE HARNESS FIXTURE IS NOT THIS PROBLEM, and saying so is the point.
    /// A flat wall has no hoop stress; applying Lamé to it would give a
    /// confident number with the wrong physics behind it.
    #[test]
    fn a_slot_refuses_to_be_treated_as_a_cylinder() {
        let e = slot_press_is_not_a_cylinder(0.05).expect_err("must refuse");
        assert!(e.contains("NOT a Lame press fit"), "{e}");
        assert!(
            e.contains("Beam"),
            "the refusal must name where to go instead: {e}"
        );
        assert!(e.contains("0.05"), "and echo what was asked: {e}");
    }

    #[test]
    fn nonpositive_and_nan_inputs_are_refused() {
        assert!(PressFit {
            interference_mm: 0.0,
            ..fit()
        }
        .solve()
        .is_err());
        assert!(PressFit {
            e_mpa: f64::NAN,
            ..fit()
        }
        .solve()
        .is_err());
        assert!(PressFit { mu: -0.1, ..fit() }.solve().is_err());
    }
}

// ---------------------------------------------------------------------------
// Tolerance chains. Spec §05. The most valuable call in the spec.
// ---------------------------------------------------------------------------

/// One link in a tolerance chain.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub name: String,
    /// The intended dimension, mm. Carried for reporting; the chain's answer
    /// does not depend on it.
    pub nominal_mm: f64,
    /// Half-width of the tolerance band, mm. A +/-0.1 fit has 0.1 here.
    pub tolerance_mm: f64,
    /// How much this link's error is multiplied before it reaches the working
    /// point. 1.0 means it arrives undiminished.
    ///
    /// THE LEVER IS THE WHOLE POINT. A clearance in a joint does not reach the
    /// working point as itself; it arrives multiplied by the joint's geometry,
    /// and nothing on a drawing says so.
    pub lever: f64,
}

impl Link {
    /// What this link contributes at the working point, mm.
    pub fn amplified(&self) -> f64 {
        self.tolerance_mm.abs() * self.lever.abs()
    }
}

/// What a chain does to a mechanism's working deflection.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainVerdict {
    /// Arithmetic sum of every amplified contribution: everything wrong at
    /// once, in the same direction.
    pub worst_case_mm: f64,
    /// Root-sum-square: the statistical stack, for a production run.
    pub rss_mm: f64,
    /// The link contributing most AT THE WORKING POINT.
    pub dominant: String,
    /// That link's share of the worst case, 0 to 1.
    pub dominant_share: f64,
    /// Worst case as a fraction of the working deflection.
    pub fraction_of_working: f64,
    /// True when the worst case is the same order as the working deflection
    /// or larger, at which point a clearance fit cannot do the job.
    pub clearance_fit_is_untenable: bool,
}

/// Stack a tolerance chain against the deflection the mechanism actually works
/// over.
///
/// # Why worst case and not RSS
///
/// Both are reported, and the VERDICT reads the worst case. Root-sum-square
/// is the right statistic for a production run of many parts, where errors
/// are independent and rarely all extreme together. A printed one-off is one
/// sample: it either fits or it does not, and the parts come off the same
/// machine on the same day with the same systematic bias. Choosing RSS there
/// is choosing the more flattering number for the wrong reason.
///
/// # The threshold
///
/// `clearance_fit_is_untenable` is true at 50% of the working deflection and
/// above, not at 100%. By the time the stack equals the whole deflection the
/// mechanism is already dead; at half, it has lost half its force and is
/// behaving unpredictably. The harness incident is exactly the half case:
/// 0.05 mm of slot slack through a 3.54 lever is 0.177 mm against 0.35 mm of
/// working deflection, and that cost half the clamp.
pub fn stack(links: &[Link], working_deflection_mm: f64) -> Result<ChainVerdict, String> {
    if links.is_empty() {
        return Err(
            "an empty chain has no tolerance to stack; refusing rather than \
                    reporting a perfect fit"
                .into(),
        );
    }
    if !working_deflection_mm.is_finite() || working_deflection_mm <= 0.0 {
        return Err(format!(
            "working_deflection_mm must be positive and finite, got \
             {working_deflection_mm}: without it there is nothing to compare the stack to, \
             and a stack with no comparison is a number rather than a verdict"
        ));
    }
    for l in links {
        for (n, v) in [
            ("nominal_mm", l.nominal_mm),
            ("tolerance_mm", l.tolerance_mm),
            ("lever", l.lever),
        ] {
            if !v.is_finite() {
                return Err(format!("{} : {n} is not finite ({v})", l.name));
            }
        }
    }
    let worst: f64 = links.iter().map(Link::amplified).sum();
    let rss: f64 = links
        .iter()
        .map(|l| l.amplified().powi(2))
        .sum::<f64>()
        .sqrt();
    // THE DOMINANT LINK IS THE LARGEST AMPLIFIED CONTRIBUTION, not the largest
    // tolerance. A tight tolerance on a long lever beats a loose one on a
    // short lever, and tightening the loose one is then wasted effort.
    let dom = links
        .iter()
        .max_by(|a, b| {
            a.amplified()
                .partial_cmp(&b.amplified())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or("no links")?;
    let fraction = worst / working_deflection_mm;
    Ok(ChainVerdict {
        worst_case_mm: worst,
        rss_mm: rss,
        dominant: dom.name.clone(),
        dominant_share: if worst > 0.0 {
            dom.amplified() / worst
        } else {
            0.0
        },
        fraction_of_working: fraction,
        clearance_fit_is_untenable: fraction >= 0.5,
    })
}

#[cfg(test)]
mod tolerance_tests {
    use super::*;

    /// The blade clip: root 3.5 mm deep, leaf reaching 8.9 mm past it, so the
    /// joint amplifies by (reach + root)/root.
    fn clip_lever() -> f64 {
        (8.9 + 3.5) / 3.5
    }

    fn slot(tolerance: f64) -> Link {
        Link {
            name: "slot slack".into(),
            nominal_mm: 1.5,
            tolerance_mm: tolerance,
            lever: clip_lever(),
        }
    }

    /// THE LOGGED LEVER: 3.54.
    #[test]
    fn the_clips_joint_amplifies_by_three_and_a_half() {
        assert!(
            (clip_lever() - 3.542_857).abs() < 1e-6,
            "(8.9+3.5)/3.5 is 3.542857, got {}",
            clip_lever()
        );
    }

    /// THE INCIDENT. 0.05 mm of slot slack costs HALF the clamp; 0.15 mm, an
    /// ordinary print tolerance, costs all of it.
    #[test]
    fn an_ordinary_print_tolerance_costs_the_whole_clamp() {
        let half = stack(&[slot(0.05)], 0.35).expect("valid");
        assert!(
            (half.fraction_of_working - 0.506).abs() < 1e-3,
            "0.05 through a 3.54 lever is half the working deflection, got {}",
            half.fraction_of_working
        );
        assert!(half.clearance_fit_is_untenable, "half is already untenable");

        let all = stack(&[slot(0.15)], 0.35).expect("valid");
        assert!(
            all.fraction_of_working > 1.5,
            "0.15 costs MORE than the whole deflection, got {}",
            all.fraction_of_working
        );
    }

    /// WITHOUT THE LEVER IT LOOKS FINE. 0.05 mm against 0.35 mm of travel is
    /// 14%, which nobody would question. The lever is what turns it into 51%,
    /// and nothing on a drawing says so.
    #[test]
    fn the_same_slack_looks_harmless_without_its_lever() {
        let unlevered = stack(
            &[Link {
                lever: 1.0,
                ..slot(0.05)
            }],
            0.35,
        )
        .expect("valid");
        assert!(
            unlevered.fraction_of_working < 0.15,
            "unlevered it is 14% and unremarkable, got {}",
            unlevered.fraction_of_working
        );
        assert!(!unlevered.clearance_fit_is_untenable);
        let levered = stack(&[slot(0.05)], 0.35).expect("valid");
        assert!(
            levered.fraction_of_working > 3.0 * unlevered.fraction_of_working,
            "the lever is the entire difference"
        );
    }

    /// THE DOMINANT LINK IS THE LARGEST CONTRIBUTION, NOT THE LARGEST
    /// TOLERANCE. A tight tolerance on a long lever beats a loose one on a
    /// short lever, and tightening the loose one is wasted effort.
    #[test]
    fn the_dominant_link_is_not_the_loosest_one() {
        let links = [
            Link {
                name: "loose but unlevered".into(),
                nominal_mm: 10.0,
                tolerance_mm: 0.20,
                lever: 1.0,
            },
            Link {
                name: "tight but levered".into(),
                nominal_mm: 1.5,
                tolerance_mm: 0.05,
                lever: 8.0,
            },
        ];
        let v = stack(&links, 1.0).expect("valid");
        assert_eq!(
            v.dominant, "tight but levered",
            "0.05*8 = 0.40 beats 0.20*1 = 0.20, though the tolerance is 4x tighter"
        );
        assert!((v.dominant_share - 0.4 / 0.6).abs() < 1e-9);
    }

    /// Worst case and RSS are both reported and they differ. The verdict reads
    /// the worst case, because a printed one-off is one sample.
    #[test]
    fn worst_case_exceeds_rss_and_the_verdict_uses_the_worst_case() {
        let links = [
            slot(0.05),
            Link {
                name: "pin fit".into(),
                ..slot(0.05)
            },
        ];
        let v = stack(&links, 1.0).expect("valid");
        assert!(
            v.worst_case_mm > v.rss_mm,
            "arithmetic sum beats the quadrature"
        );
        assert!(
            (v.fraction_of_working - v.worst_case_mm / 1.0).abs() < 1e-12,
            "the verdict reads the worst case, not the RSS"
        );
    }

    /// THE THRESHOLD IS HALF, NOT ALL. By the time the stack equals the whole
    /// deflection the mechanism is dead; at half it has already lost half its
    /// force and is behaving unpredictably.
    #[test]
    fn the_threshold_is_half_the_working_deflection() {
        let under = stack(
            &[Link {
                lever: 1.0,
                ..slot(0.17)
            }],
            0.35,
        )
        .expect("valid");
        assert!(!under.clearance_fit_is_untenable, "0.486 of working");
        let over = stack(
            &[Link {
                lever: 1.0,
                ..slot(0.18)
            }],
            0.35,
        )
        .expect("valid");
        assert!(over.clearance_fit_is_untenable, "0.514 of working");
    }

    /// AN EMPTY CHAIN IS NOT A PERFECT FIT, and reporting zero stack would
    /// read as one.
    #[test]
    fn an_empty_chain_is_refused_rather_than_reported_as_perfect() {
        let e = stack(&[], 0.35).expect_err("must refuse");
        assert!(e.contains("refusing"), "{e}");
    }

    /// A STACK WITH NOTHING TO COMPARE IT TO IS A NUMBER, NOT A VERDICT.
    #[test]
    fn a_missing_working_deflection_is_refused() {
        for d in [0.0, -0.35, f64::NAN] {
            let e = stack(&[slot(0.05)], d).expect_err("must refuse");
            assert!(e.contains("rather than a verdict"), "{e}");
        }
    }

    /// A negative lever still amplifies: direction is not magnitude, and a
    /// chain that cancelled opposing errors would be reporting a best case.
    #[test]
    fn a_negative_lever_still_contributes_its_magnitude() {
        let a = stack(&[slot(0.05)], 0.35).expect("valid");
        let b = stack(
            &[Link {
                lever: -clip_lever(),
                ..slot(0.05)
            }],
            0.35,
        )
        .expect("valid");
        assert!((a.worst_case_mm - b.worst_case_mm).abs() < 1e-12);
    }
}
