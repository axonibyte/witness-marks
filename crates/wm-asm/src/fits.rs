//! The mating-pair matrix, with the sign checked.
//!
//! # Two incidents, one check
//!
//! **buildability 48.** A retainer modelled with a CLEARANCE retains nothing.
//! A hold-down sitting above its part with a gap has specified free play in
//! exactly the axis it was meant to constrain. A clamp is modelled
//! INTERFERING, and the overlap volume is the proof it will clamp.
//!
//! **buildability 70.** A snap must engage deeper than its part can wander.
//! One clearance was doing three jobs: engagement came out at
//! `thr_depth - thr_clear` = 0.40 mm while lateral slop was `thr_clear`
//! = 0.50 mm. Slop exceeded engagement, so the carrier shifted sideways and
//! the rib cleared the crest with no force at all.
//!
//! The first is a SIGN mismatch; the second is a sign that is right and a
//! MAGNITUDE that is not. A matrix that only checked signs would pass the
//! second one.

/// What a pair is FOR. Without this there is nothing to check a measurement
/// against: 0.2 mm of gap is correct for a sliding fit and fatal for a clamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// Must be free to move: a positive gap is required.
    Clearance,
    /// Must grip: the parts must overlap.
    Interference,
    /// Must hold against a load: overlap AND enough of it to survive the
    /// other part's wander.
    Retention,
}

/// One measured pair.
#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    pub a: String,
    pub b: String,
    /// POSITIVE is a gap, NEGATIVE is overlap. One signed number, because two
    /// fields named `clearance` and `interference` invite a caller to fill in
    /// the one they expected.
    pub gap_mm: f64,
    pub intent: Intent,
    /// How far the other part can wander in this axis. Only read for
    /// `Retention`, where engagement must beat it.
    pub wander_mm: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Ok,
    /// The sign is wrong for the intent: a clearance where a grip was wanted
    /// or the reverse.
    WrongSign(String),
    /// The sign is right and the magnitude is not.
    TooShallow(String),
    /// The intent needs a figure the caller did not supply.
    Unanswerable(String),
}

impl Verdict {
    pub fn is_ok(&self) -> bool {
        matches!(self, Verdict::Ok)
    }
}

/// Check one pair against what it is for.
pub fn check(p: &Pair) -> Verdict {
    if !p.gap_mm.is_finite() {
        return Verdict::Unanswerable(format!(
            "{} / {}: the measured gap is {}, which is not a measurement",
            p.a, p.b, p.gap_mm
        ));
    }
    match p.intent {
        Intent::Clearance => {
            if p.gap_mm > 0.0 {
                Verdict::Ok
            } else {
                Verdict::WrongSign(format!(
                    "{} / {}: wanted CLEARANCE and the parts overlap by {} mm; they cannot \
                     be assembled, or something will be forced",
                    p.a, p.b, -p.gap_mm
                ))
            }
        }
        Intent::Interference => {
            if p.gap_mm < 0.0 {
                Verdict::Ok
            } else {
                Verdict::WrongSign(format!(
                    "{} / {}: wanted INTERFERENCE and there is {} mm of gap. A retainer \
                     modelled with a clearance retains nothing: this has specified free \
                     play in the axis it was meant to constrain",
                    p.a, p.b, p.gap_mm
                ))
            }
        }
        Intent::Retention => {
            if p.gap_mm >= 0.0 {
                return Verdict::WrongSign(format!(
                    "{} / {}: wanted RETENTION and there is {} mm of gap; nothing is held",
                    p.a, p.b, p.gap_mm
                ));
            }
            let engagement = -p.gap_mm;
            match p.wander_mm {
                // THE MISSING FIGURE IS REFUSED, NOT ASSUMED ZERO. Assuming no
                // wander passes every retention pair, which is precisely the
                // mistake buildability 70 records.
                None => Verdict::Unanswerable(format!(
                    "{} / {}: engages {engagement} mm, but no wander was given. Assuming \
                     the part cannot move would pass every retention pair ever checked, \
                     which is how a 0.40 mm engagement came to be trusted against 0.50 mm \
                     of slop",
                    p.a, p.b
                )),
                Some(w) if !w.is_finite() || w < 0.0 => Verdict::Unanswerable(format!(
                    "{} / {}: wander is {w}, which is not a distance",
                    p.a, p.b
                )),
                Some(w) if engagement > w => Verdict::Ok,
                Some(w) => Verdict::TooShallow(format!(
                    "{} / {}: engages {engagement} mm against {w} mm of wander. The part \
                     can move further sideways than the feature reaches, so it clears with \
                     no force at all",
                    p.a, p.b
                )),
            }
        }
    }
}

/// Check a whole matrix, returning only what is wrong.
///
/// RETURNS THE FAILURES, NOT A BOOLEAN. "Something is wrong" sends somebody
/// re-measuring every pair; a list says which.
pub fn check_all(pairs: &[Pair]) -> Vec<Verdict> {
    pairs.iter().map(check).filter(|v| !v.is_ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(gap: f64, intent: Intent) -> Pair {
        Pair {
            a: "retainer".into(),
            b: "blade".into(),
            gap_mm: gap,
            intent,
            wander_mm: None,
        }
    }

    #[test]
    fn a_sliding_fit_wants_a_gap_and_gets_one() {
        assert!(check(&pair(0.2, Intent::Clearance)).is_ok());
    }

    /// BUILDABILITY 48. A hold-down with a gap has specified free play in
    /// exactly the axis it was meant to constrain.
    #[test]
    fn a_retainer_modelled_with_a_clearance_is_a_finding() {
        match check(&pair(0.2, Intent::Interference)) {
            Verdict::WrongSign(w) => {
                assert!(w.contains("retains nothing"), "{w}");
                assert!(w.contains("free play"), "{w}");
            }
            other => panic!("a gap where a grip was wanted must be a finding, got {other:?}"),
        }
    }

    /// And the mirror: parts that must slide and instead overlap cannot be
    /// assembled.
    #[test]
    fn a_sliding_fit_that_overlaps_is_a_finding() {
        assert!(matches!(
            check(&pair(-0.2, Intent::Clearance)),
            Verdict::WrongSign(_)
        ));
    }

    #[test]
    fn a_clamp_that_overlaps_is_correct() {
        assert!(check(&pair(-0.3, Intent::Interference)).is_ok());
    }

    /// BUILDABILITY 70. The sign is RIGHT and the magnitude is not: 0.40 mm of
    /// engagement against 0.50 mm of lateral slop. A matrix that only checked
    /// signs would pass this.
    #[test]
    fn a_snap_shallower_than_its_parts_wander_is_a_finding() {
        let p = Pair {
            gap_mm: -0.40,
            intent: Intent::Retention,
            wander_mm: Some(0.50),
            ..pair(0.0, Intent::Retention)
        };
        match check(&p) {
            Verdict::TooShallow(w) => {
                assert!(w.contains("0.4") && w.contains("0.5"), "{w}");
                assert!(w.contains("no force at all"), "{w}");
            }
            other => panic!("engagement under wander must be a finding, got {other:?}"),
        }
        // and the sign alone WOULD have passed it
        assert!(matches!(
            check(&Pair {
                intent: Intent::Interference,
                ..p
            }),
            Verdict::Ok
        ));
    }

    #[test]
    fn a_snap_deeper_than_its_wander_is_correct() {
        let p = Pair {
            gap_mm: -0.80,
            intent: Intent::Retention,
            wander_mm: Some(0.50),
            ..pair(0.0, Intent::Retention)
        };
        assert!(check(&p).is_ok());
    }

    /// THE MISSING FIGURE IS REFUSED, NOT ASSUMED ZERO. Assuming no wander
    /// passes every retention pair ever checked, which is exactly how a
    /// 0.40 mm engagement came to be trusted against 0.50 mm of slop.
    #[test]
    fn retention_without_a_wander_figure_is_unanswerable_not_ok() {
        let p = Pair {
            gap_mm: -0.40,
            intent: Intent::Retention,
            wander_mm: None,
            ..pair(0.0, Intent::Retention)
        };
        match check(&p) {
            Verdict::Unanswerable(w) => assert!(w.contains("would pass every"), "{w}"),
            other => panic!("a missing wander must not pass, got {other:?}"),
        }
    }

    /// EXACTLY EQUAL IS NOT ENOUGH. Engagement equal to wander means the
    /// feature is cleared at the extreme of travel, which is where parts
    /// actually sit.
    #[test]
    fn engagement_exactly_equal_to_wander_is_too_shallow() {
        let p = Pair {
            gap_mm: -0.50,
            intent: Intent::Retention,
            wander_mm: Some(0.50),
            ..pair(0.0, Intent::Retention)
        };
        assert!(matches!(check(&p), Verdict::TooShallow(_)));
    }

    /// ZERO IS NOT A CLEARANCE AND NOT AN INTERFERENCE. Two faces exactly
    /// touching slide with no margin and grip with no force.
    #[test]
    fn a_zero_gap_satisfies_neither_intent() {
        assert!(matches!(
            check(&pair(0.0, Intent::Clearance)),
            Verdict::WrongSign(_)
        ));
        assert!(matches!(
            check(&pair(0.0, Intent::Interference)),
            Verdict::WrongSign(_)
        ));
    }

    /// A LIST, NOT A BOOLEAN. "Something is wrong" sends somebody
    /// re-measuring every pair.
    #[test]
    fn the_matrix_returns_what_is_wrong_and_not_merely_that_something_is() {
        let v = check_all(&[
            pair(0.2, Intent::Clearance),
            pair(0.2, Intent::Interference),
            pair(-0.2, Intent::Clearance),
        ]);
        assert_eq!(v.len(), 2, "the good pair is not reported");
        assert!(v.iter().all(|x| !x.is_ok()));
    }

    #[test]
    fn a_non_finite_measurement_is_unanswerable() {
        assert!(matches!(
            check(&pair(f64::NAN, Intent::Clearance)),
            Verdict::Unanswerable(_)
        ));
    }
}
