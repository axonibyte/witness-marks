//! Staleness propagation: what just went false, unprompted.
//!
//! Spec §06.2, and the incident it is built from. The user said four words --
//! *"this is petg"* -- and six of the harness's twelve derived numbers went
//! wrong at once. Spring rate is linear in modulus; PETG is ~2000 MPa against
//! PLA's 3500. The blade clip's clamp dropped from 3.17 N to 1.81 N, losing
//! 43% of the one property that sub-assembly exists to provide, and a comment
//! four lines above the parameter still read `2.4x under yield`.
//!
//! Nothing in the file knew, because nothing in the file was connected to
//! anything else. This module is the connection.
//!
//! # Invalidated is not the same as failing
//!
//! Six claims were INVALIDATED by the change -- their values moved. Only one
//! of those moved out of its range. Reporting only the failure hides five
//! numbers that are now different from what every comment and conversation
//! says they are, and reporting them all as failures cries wolf. They are
//! counted separately.
//!
//! # A claim that cannot be evaluated is neither
//!
//! If a claim refuses before or after the change, it has not passed, has not
//! failed, and has not been shown unaffected. Folding it into any of those
//! three is how an unmeasurable claim comes to look like a healthy one.

use crate::claim::{verify, Claim, Inputs, Provenance, Status, Verified};
use crate::index::Index;

/// One input that moved.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub input: String,
    pub before: f64,
    pub after: f64,
}

/// What the change did to one claim.
#[derive(Debug, Clone, PartialEq)]
pub struct Moved {
    pub name: String,
    pub before: Verified,
    pub after: Verified,
}

impl Moved {
    /// Was it inside its range before and outside after? The line that has to
    /// appear in red.
    pub fn newly_failing(&self) -> bool {
        self.before.status.is_pass() && matches!(self.after.status, Status::Fail { .. })
    }
    /// Did it become unevaluable? A claim that could be checked and now
    /// cannot is its own kind of regression.
    pub fn newly_unevaluable(&self) -> bool {
        !matches!(self.before.status, Status::Refused(_))
            && matches!(self.after.status, Status::Refused(_))
    }
}

/// The whole answer for one change.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub changes: Vec<Change>,
    /// Claims whose value moved, in declaration order.
    pub moved: Vec<Moved>,
    /// Claims that read a changed input but whose value did not move.
    pub read_but_unmoved: Vec<String>,
    /// Claims that do not read any changed input.
    pub untouched: Vec<String>,
    /// Claims that could not be evaluated in one state or the other.
    pub unevaluable: Vec<String>,
    /// Inputs now standing on nothing measured, worst first.
    pub weak_inputs: Vec<(String, Provenance)>,
}

impl Report {
    pub fn newly_failing(&self) -> Vec<&Moved> {
        self.moved.iter().filter(|m| m.newly_failing()).collect()
    }
    /// Every claim is in exactly one bucket. A tally that does not add up to
    /// the number of claims has lost one somewhere, and the lost one is
    /// always the interesting one.
    pub fn accounted(&self) -> usize {
        self.moved.len()
            + self.read_but_unmoved.len()
            + self.untouched.len()
            + self.unevaluable.len()
    }
}

/// Re-evaluate every claim across an input change.
///
/// REFUSES AN EMPTY CLAIM SET. "Nothing was invalidated" and "nothing was
/// checked" are the same sentence from the outside, and only one of them is
/// reassuring.
///
/// REFUSES A CHANGE THAT CHANGED NOTHING, for the same reason as `model.diff`
/// refuses a bit-identical edit: a propagation run over identical inputs
/// reports zero invalidations, which reads exactly like a safe change.
pub fn stale(claims: &[Claim], before: &Inputs, after: &Inputs) -> Result<Report, String> {
    stale_with(None, claims, before, after)
}

/// The same report, optionally reading prepared indexes (issue 37).
///
/// TWO INDEXES, NOT ONE, AND THAT IS THE WHOLE REASON THIS SIGNATURE LOOKS
/// LIKE THIS. An [`Index`] is built by perturbing one input set, so it is a
/// fact ABOUT THAT SET. What a claim reads can differ between the two sides of
/// a staleness comparison -- an input that appeared, one that vanished, a
/// branch that a changed value takes -- and this function's whole job is to
/// notice a claim that reads a changed input. Handing it a single index and
/// using it for both sides would answer the `after` question with the `before`
/// facts, which is precisely the stale-figure defect this module exists to
/// catch, arriving through the index meant to organize it.
///
/// `None` builds both. The pair is accepted so a caller holding them already
/// -- having just cross-checked them against `trace` -- does not perturb twice.
pub fn stale_with(
    indexes: Option<(&Index, &Index)>,
    claims: &[Claim],
    before: &Inputs,
    after: &Inputs,
) -> Result<Report, String> {
    if claims.is_empty() {
        return Err(
            "no claims to propagate over. 'Nothing was invalidated' and 'nothing was \
                    checked' read the same from outside, so this refuses rather than \
                    reporting a clean run over an empty set"
                .into(),
        );
    }
    let mut changes = Vec::new();
    for name in after.names() {
        let a = after.get(&name).map(|i| i.value);
        let b = before.get(&name).map(|i| i.value);
        match (b, a) {
            (Some(b), Some(a)) if b != a => changes.push(Change {
                input: name,
                before: b,
                after: a,
            }),
            // AN INPUT THAT APPEARED is a change too: it was absent and is now
            // a number, which can turn a refusing claim into a passing one.
            (None, Some(a)) => changes.push(Change {
                input: name,
                before: f64::NAN,
                after: a,
            }),
            _ => {}
        }
    }
    for name in before.names() {
        if after.get(&name).is_none() {
            if let Some(i) = before.get(&name) {
                changes.push(Change {
                    input: name,
                    before: i.value,
                    after: f64::NAN,
                });
            }
        }
    }
    if changes.is_empty() {
        return Err(
            "nothing changed between these two input sets. A propagation run over \
                    identical inputs reports zero invalidations, which reads exactly like a \
                    safe change"
                .into(),
        );
    }
    let changed_names: Vec<String> = changes.iter().map(|c| c.input.clone()).collect();

    let mut report = Report {
        changes,
        moved: Vec::new(),
        read_but_unmoved: Vec::new(),
        untouched: Vec::new(),
        unevaluable: Vec::new(),
        weak_inputs: Vec::new(),
    };
    for c in claims {
        let b = verify(c, before);
        let a = verify(c, after);
        if matches!(b.status, Status::Refused(_)) || matches!(a.status, Status::Refused(_)) {
            report.unevaluable.push(c.name.clone());
            continue;
        }
        // THE UNION OF BOTH SIDES. A claim that read a changed input on
        // either side has had its basis moved, and taking only one side would
        // miss the input that appeared or vanished.
        let reads_changed = match indexes {
            Some((bi, ai)) => {
                let empty = std::collections::BTreeSet::new();
                let br = bi.reads(&c.name).unwrap_or(&empty);
                let ar = ai.reads(&c.name).unwrap_or(&empty);
                changed_names
                    .iter()
                    .any(|n| br.contains(n) || ar.contains(n))
            }
            None => {
                a.inputs_used.iter().any(|n| changed_names.contains(n))
                    || b.inputs_used.iter().any(|n| changed_names.contains(n))
            }
        };
        if b.value != a.value {
            report.moved.push(Moved {
                name: c.name.clone(),
                before: b,
                after: a,
            });
        } else if reads_changed {
            report.read_but_unmoved.push(c.name.clone());
        } else {
            report.untouched.push(c.name.clone());
        }
    }

    // THE INPUTS THE SURVIVING ANSWERS STAND ON. After a material change the
    // interesting line is often not a failure but "two of these are now
    // literature values with no measurement behind them".
    let mut weak: Vec<(String, Provenance)> = after
        .0
        .iter()
        .filter(|(_, i)| i.provenance < Provenance::Measured)
        .map(|(n, i)| (n.clone(), i.provenance))
        .collect();
    weak.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    report.weak_inputs = weak;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim::Input;
    use wm_mech::{Beam, End};

    fn clip() -> Beam {
        Beam {
            e_mpa: 0.0,
            l_mm: 8.9,
            w_mm: 10.0,
            t_mm: 0.9,
            end: End::Cantilever,
        }
    }

    fn c(
        name: &str,
        declared: &[&str],
        range: (f64, f64),
        eval: fn(&Inputs) -> Result<f64, String>,
    ) -> Claim {
        Claim {
            name: name.into(),
            declared_inputs: declared.iter().map(|s| (*s).to_string()).collect(),
            range,
            unit: "x".into(),
            because: "the harness".into(),
            eval,
        }
    }

    /// TWELVE CLAIMS, SIX OF WHICH READ `E` -- the harness's own shape.
    fn twelve() -> Vec<Claim> {
        vec![
            // --- the six that read E ---
            c("clip_clamps", &["E", "spring_pre"], (2.0, 5.0), |i| {
                Ok(Beam {
                    e_mpa: i.need("E")?,
                    ..clip()
                }
                .at(i.need("spring_pre")?)?
                .force)
            }),
            c("clip_stress", &["E", "spring_pre"], (0.0, 40.0), |i| {
                Ok(Beam {
                    e_mpa: i.need("E")?,
                    ..clip()
                }
                .at(i.need("spring_pre")?)?
                .stress_mpa)
            }),
            c("arm_squeeze_rate", &["E"], (5.0, 30.0), |i| {
                Beam {
                    e_mpa: i.need("E")?,
                    l_mm: 12.0,
                    w_mm: 10.0,
                    t_mm: 1.5,
                    end: End::Cantilever,
                }
                .stiffness()
            }),
            c("clip_stiffness", &["E"], (1.0, 40.0), |i| {
                Beam {
                    e_mpa: i.need("E")?,
                    ..clip()
                }
                .stiffness()
            }),
            c(
                "yield_margin",
                &["E", "yield", "spring_pre"],
                (1.0, 10.0),
                |i| {
                    Beam {
                        e_mpa: i.need("E")?,
                        ..clip()
                    }
                    .yield_margin(i.need("spring_pre")?, i.need("yield")?)
                },
            ),
            c("latch_force", &["E", "latch_pre"], (0.5, 12.0), |i| {
                Ok(Beam {
                    e_mpa: i.need("E")?,
                    l_mm: 6.0,
                    w_mm: 8.0,
                    t_mm: 1.0,
                    end: End::Cantilever,
                }
                .at(i.need("latch_pre")?)?
                .force)
            }),
            // --- the six that do not ---
            c("bore_clearance", &["bore_d", "shaft_d"], (0.1, 0.6), |i| {
                Ok(i.need("bore_d")? - i.need("shaft_d")?)
            }),
            c("wall_layers", &["wall_t", "layer_h"], (2.0, 12.0), |i| {
                Ok(i.need("wall_t")? / i.need("layer_h")?)
            }),
            c("bed_footprint", &["foot_w"], (10.0, 200.0), |i| {
                Ok(i.need("foot_w")? * 2.0)
            }),
            c("mouth_angle", &["mouth_deg"], (30.0, 90.0), |i| {
                i.need("mouth_deg")
            }),
            c("stub_overlap", &["stub_lap"], (0.2, 1.0), |i| {
                i.need("stub_lap")
            }),
            c("tail_gap", &["tail_gap"], (0.5, 1.0), |i| {
                i.need("tail_gap")
            }),
        ]
    }

    fn harness(e: f64) -> Inputs {
        Inputs::new()
            .with("E", Input::literature(e, 250.0, "datasheet"))
            .with("yield", Input::literature(50.0, 10.0, "datasheet"))
            .with("spring_pre", Input::measured(0.350, "the model"))
            .with("latch_pre", Input::measured(0.5, "the model"))
            .with("bore_d", Input::measured(34.0, "calipers"))
            .with("shaft_d", Input::measured(33.7, "calipers"))
            .with("wall_t", Input::measured(1.2, "the model"))
            .with("layer_h", Input::measured(0.2, "the profile"))
            .with("foot_w", Input::measured(40.0, "the model"))
            .with("mouth_deg", Input::measured(68.0, "the model"))
            .with("stub_lap", Input::measured(0.5, "the model"))
            .with("tail_gap", Input::measured(0.723, "the model"))
    }

    /// THE INCIDENT, END TO END. "this is petg" moves E from 3500 to 2000 and
    /// SIX of the twelve claims change value -- spec §06.2's figure.
    #[test]
    fn switching_to_petg_invalidates_six_of_twelve() {
        let r = stale(&twelve(), &harness(3500.0), &harness(2000.0)).expect("valid");
        assert_eq!(r.changes.len(), 1, "one input moved");
        assert_eq!(r.changes[0].input, "E");
        assert_eq!(
            r.moved.len(),
            6,
            "six of twelve: {:?}",
            r.moved.iter().map(|m| &m.name).collect::<Vec<_>>()
        );
        assert_eq!(r.untouched.len(), 6);
        assert_eq!(r.accounted(), 12, "every claim lands in exactly one bucket");
    }

    /// AND ONLY ONE OF THE SIX ACTUALLY FAILS. Reporting only the failure
    /// hides five numbers that no longer match what every comment says;
    /// reporting all six as failures cries wolf.
    #[test]
    fn only_one_of_the_six_moves_out_of_its_range() {
        let r = stale(&twelve(), &harness(3500.0), &harness(2000.0)).expect("valid");
        let failing = r.newly_failing();
        assert_eq!(
            failing.len(),
            1,
            "{:?}",
            failing.iter().map(|m| &m.name).collect::<Vec<_>>()
        );
        assert_eq!(failing[0].name, "clip_clamps");
    }

    /// THE LOGGED FIGURES: 3.17 N becomes 1.81 N, which is 43% of the one
    /// property that sub-assembly exists to provide.
    #[test]
    fn the_clamp_force_falls_from_three_one_seven_to_one_eight_one() {
        let r = stale(&twelve(), &harness(3500.0), &harness(2000.0)).expect("valid");
        let m = r
            .moved
            .iter()
            .find(|m| m.name == "clip_clamps")
            .expect("present");
        let b = m.before.value.expect("a value");
        let a = m.after.value.expect("a value");
        assert!((b - 3.17).abs() < 0.01, "before {b}");
        assert!((a - 1.81).abs() < 0.01, "after {a}");
        assert!(
            (1.0 - a / b - 0.4286).abs() < 0.01,
            "a 43% loss, got {}",
            1.0 - a / b
        );
    }

    /// AND THE STRESS CLAIM IMPROVES. Spec §06.2: 20.9 MPa -> 11.9 MPa, "ok
    /// (margin improved)". A propagation report that only shows regressions
    /// would not have this line, and it is the one that stops an author
    /// panicking about all six.
    #[test]
    fn the_stress_claim_moves_and_is_still_inside_its_range() {
        let r = stale(&twelve(), &harness(3500.0), &harness(2000.0)).expect("valid");
        let m = r
            .moved
            .iter()
            .find(|m| m.name == "clip_stress")
            .expect("present");
        assert!((m.before.value.expect("v") - 20.9).abs() < 0.1);
        assert!((m.after.value.expect("v") - 11.9).abs() < 0.1);
        assert!(m.after.status.is_pass());
        assert!(!m.newly_failing());
    }

    /// THE WEAK INPUTS ARE NAMED. Spec §06.2's closing line: "2 inputs are
    /// now literature values with no measurement".
    #[test]
    fn the_inputs_with_no_measurement_behind_them_are_named() {
        let r = stale(&twelve(), &harness(3500.0), &harness(2000.0)).expect("valid");
        assert_eq!(
            r.weak_inputs,
            vec![
                ("E".to_string(), Provenance::Literature),
                ("yield".to_string(), Provenance::Literature)
            ]
        );
    }

    /// AN EMPTY CLAIM SET IS REFUSED: "nothing was invalidated" and "nothing
    /// was checked" read the same from outside.
    #[test]
    fn propagating_over_no_claims_is_refused() {
        let e = stale(&[], &harness(3500.0), &harness(2000.0)).expect_err("must refuse");
        assert!(e.contains("nothing was checked"), "{e}");
    }

    /// A CHANGE THAT CHANGED NOTHING IS REFUSED, for the same reason
    /// model.diff refuses a bit-identical edit.
    #[test]
    fn propagating_over_identical_inputs_is_refused() {
        let e = stale(&twelve(), &harness(3500.0), &harness(3500.0)).expect_err("must refuse");
        assert!(e.contains("reads exactly like a safe change"), "{e}");
    }

    /// A CLAIM THAT CANNOT BE EVALUATED IS ITS OWN BUCKET, not a pass and not
    /// a failure. Removing `yield` makes yield_margin unevaluable, and the
    /// tally must still account for all twelve.
    #[test]
    fn a_claim_that_cannot_be_evaluated_is_neither_passing_nor_failing() {
        let mut after = harness(2000.0);
        after.0.remove("yield");
        let r = stale(&twelve(), &harness(3500.0), &after).expect("valid");
        assert_eq!(r.unevaluable, vec!["yield_margin".to_string()]);
        assert!(!r.moved.iter().any(|m| m.name == "yield_margin"));
        assert_eq!(r.accounted(), 12, "still all twelve");
    }

    /// AN INPUT THAT APPEARED is a change: it was absent and is now a number,
    /// which can turn a refusing claim into a passing one.
    #[test]
    fn an_input_that_appeared_counts_as_a_change() {
        let mut before = harness(3500.0);
        before.0.remove("tail_gap");
        let r = stale(&twelve(), &before, &harness(3500.0)).expect("valid");
        assert!(r.changes.iter().any(|c| c.input == "tail_gap"));
        assert_eq!(r.unevaluable, vec!["tail_gap".to_string()]);
    }
}
