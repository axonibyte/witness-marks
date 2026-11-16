//! Whether a part can be gotten IN, which is a different question from
//! whether it works once it is in.
//!
//! # Two questions, and neither answers the other
//!
//! From the spec: *"`insert` != `sweep`: clear through the whole range yet
//! impossible to install, or the reverse -- both happened."*
//!
//! - **Clear to operate, impossible to install.** The operating range is a
//!   band the part never leaves once assembled. The insertion path starts
//!   OUTSIDE that band, at the open air, and runs to the assembled position
//!   through geometry no operating sample ever visits. A clear operating
//!   sweep says nothing about it.
//! - **Installable, seizes in use.** The reverse. The part drops in on a
//!   straight path and then binds at one end of its travel.
//!
//! So this module deliberately does NOT convert between the two. There is no
//! `From<Stop> for Access` and no accessor that will hand back a single
//! pass/fail: a caller that has only measured one of the two questions is told
//! which one it is missing. The whole defect is a call site reading one answer
//! as if it were both.

use crate::travel::{first_stop, Sample, Stop};

/// Whether the part can reach its assembled position.
#[derive(Debug, Clone, PartialEq)]
pub enum Access {
    /// A path exists, clear to the end of what was sampled.
    Clear {
        /// Where the path was sampled to. The assembled position is normally
        /// t = 1; a path sampled to 0.6 has not reached it.
        sampled_to: f64,
    },
    /// Something is in the way.
    Blocked { by: String, at_t: f64 },
}

/// Read an insertion path. The path runs from the open air (t = 0) to the
/// assembled position (t = 1), which is the OPPOSITE sense from an operating
/// sweep -- there, t = 0 is the assembled position.
///
/// Same sweep machinery and same refusals as [`first_stop`], including the
/// coarse-path refusal: a block between two samples would be reported at the
/// later one, which over-states how far the part gets.
pub fn insert(path: &[Sample], max_gap: f64) -> Result<Access, String> {
    match first_stop(path, None, max_gap)? {
        Stop::Clear { sampled_to } => Ok(Access::Clear { sampled_to }),
        Stop::Stopped { part, at_t, .. } => Ok(Access::Blocked { by: part, at_t }),
    }
}

/// Both questions about one part, each answered or explicitly not.
///
/// Held as two `Option`s rather than two booleans so that "not measured" is a
/// distinct state from "measured and clear". A default-false boolean reads as
/// a failure and a default-true one reads as a pass; neither is true.
#[derive(Debug, Clone, Default)]
pub struct Assembly {
    pub part: String,
    pub installs: Option<Access>,
    pub operates: Option<Stop>,
}

/// What the pair of answers amounts to.
#[derive(Debug, Clone, PartialEq)]
pub enum Fitness {
    /// Both questions asked, both clear.
    Works,
    /// At least one question was answered badly. Every failure is named, not
    /// just the first, because the two have different fixes.
    Fails(Vec<String>),
    /// A question was never asked. Names which one.
    Incomplete(Vec<String>),
}

impl Assembly {
    /// The verdict, which REFUSES to be reached from one answer.
    ///
    /// This is the method the defect would have needed: a call site holding a
    /// clear operating sweep asks for the verdict and is told it never checked
    /// insertion, rather than being handed the pass it half-earned.
    ///
    /// `Incomplete` outranks `Fails`: an unanswered question cannot be ruled
    /// out by an answered one, so a part that is known to seize AND was never
    /// test-fitted reports both, with the gap named.
    pub fn fitness(&self) -> Fitness {
        let mut missing = Vec::new();
        if self.installs.is_none() {
            missing.push(format!(
                "{}: insertion was never checked -- a clear operating sweep does not \
                 cover the path in from the open air",
                self.part
            ));
        }
        if self.operates.is_none() {
            missing.push(format!(
                "{}: the operating sweep was never checked -- dropping in on a straight \
                 path does not cover binding at the ends of travel",
                self.part
            ));
        }
        if !missing.is_empty() {
            return Fitness::Incomplete(missing);
        }
        let mut bad = Vec::new();
        if let Some(Access::Blocked { by, at_t }) = &self.installs {
            bad.push(format!(
                "{}: cannot be installed -- {by} is in the way at t={at_t}",
                self.part
            ));
        }
        if let Some(Access::Clear { sampled_to }) = &self.installs {
            if *sampled_to < 1.0 {
                bad.push(format!(
                    "{}: the insertion path was only sampled to t={sampled_to}, so it has \
                     not reached the assembled position at t=1",
                    self.part
                ));
            }
        }
        if let Some(Stop::Stopped { part, at_t, .. }) = &self.operates {
            bad.push(format!(
                "{}: stopped in operation by {part} at t={at_t}",
                self.part
            ));
        }
        if bad.is_empty() {
            Fitness::Works
        } else {
            Fitness::Fails(bad)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(t: f64, part: &str, volume: f64) -> Sample {
        Sample {
            t,
            part: part.into(),
            volume,
        }
    }

    fn clear_path() -> Vec<Sample> {
        vec![
            s(0.0, "frame", 0.0),
            s(0.5, "frame", 0.0),
            s(1.0, "frame", 0.0),
        ]
    }

    /// CLEAR TO OPERATE, IMPOSSIBLE TO INSTALL. The operating range is a band
    /// the part never leaves; the insertion path starts outside it.
    #[test]
    fn clear_through_the_operating_range_and_still_not_installable() {
        let operating = [s(0.0, "frame", 0.0), s(0.5, "frame", 0.0)];
        let blocked = [s(0.0, "frame", 0.0), s(0.5, "lip", 0.84)];
        let a = Assembly {
            part: "blade".into(),
            installs: Some(insert(&blocked, 0.6).expect("valid")),
            operates: Some(first_stop(&operating, None, 0.6).expect("valid")),
        };
        match a.fitness() {
            Fitness::Fails(v) => {
                assert_eq!(v.len(), 1, "one failure, the insertion: {v:?}");
                assert!(v[0].contains("cannot be installed"), "{:?}", v);
                assert!(v[0].contains("lip"), "{:?}", v);
            }
            other => panic!("a clear operating sweep must not pass it: {other:?}"),
        }
    }

    /// THE REVERSE. It drops straight in and then binds at the end of travel.
    #[test]
    fn installable_and_still_seizes_in_operation() {
        let seizes = [
            s(0.0, "frame", 0.0),
            s(0.5, "frame", 0.0),
            s(1.0, "frame", 0.3),
        ];
        let a = Assembly {
            part: "blade".into(),
            installs: Some(insert(&clear_path(), 0.6).expect("valid")),
            operates: Some(first_stop(&seizes, None, 0.6).expect("valid")),
        };
        match a.fitness() {
            Fitness::Fails(v) => {
                assert_eq!(v.len(), 1);
                assert!(v[0].contains("stopped in operation"), "{:?}", v);
            }
            other => panic!("expected the operating failure, got {other:?}"),
        }
    }

    /// THE DEFECT ITSELF: a call site holding a clear operating sweep and
    /// nothing else asks for the verdict. It must be told which question it
    /// never asked, NOT handed a pass.
    #[test]
    fn an_operating_sweep_alone_cannot_reach_a_verdict() {
        let a = Assembly {
            part: "blade".into(),
            installs: None,
            operates: Some(first_stop(&[s(0.0, "frame", 0.0)], None, 0.6).expect("valid")),
        };
        match a.fitness() {
            Fitness::Incomplete(v) => {
                assert_eq!(v.len(), 1);
                assert!(v[0].contains("insertion was never checked"), "{:?}", v);
            }
            other => panic!("it never checked insertion; got {other:?}"),
        }
    }

    /// And the mirror: an insertion check alone is just as incomplete.
    #[test]
    fn an_insertion_check_alone_cannot_reach_a_verdict_either() {
        let a = Assembly {
            part: "blade".into(),
            installs: Some(insert(&clear_path(), 0.6).expect("valid")),
            operates: None,
        };
        match a.fitness() {
            Fitness::Incomplete(v) => {
                assert!(
                    v[0].contains("operating sweep was never checked"),
                    "{:?}",
                    v
                )
            }
            other => panic!("expected incomplete, got {other:?}"),
        }
    }

    /// AN UNANSWERED QUESTION IS NOT RULED OUT BY AN ANSWERED ONE. A part
    /// known to seize and never test-fitted reports the gap, not only the
    /// failure it happens to have measured.
    #[test]
    fn a_known_failure_does_not_absorb_the_unasked_question() {
        let seizes = [s(0.0, "frame", 0.0), s(0.5, "frame", 0.9)];
        let a = Assembly {
            part: "blade".into(),
            installs: None,
            operates: Some(first_stop(&seizes, None, 0.6).expect("valid")),
        };
        assert!(
            matches!(a.fitness(), Fitness::Incomplete(_)),
            "the unasked question outranks the answered one"
        );
    }

    /// Both asked, both clear, and the path actually reached the assembled
    /// position.
    #[test]
    fn both_questions_answered_clear_is_the_only_pass() {
        let a = Assembly {
            part: "blade".into(),
            installs: Some(insert(&clear_path(), 0.6).expect("valid")),
            operates: Some(first_stop(&clear_path(), None, 0.6).expect("valid")),
        };
        assert_eq!(a.fitness(), Fitness::Works);
    }

    /// A PATH THAT DID NOT REACH t = 1 IS NOT AN INSTALLATION. It is a path
    /// that got partway and stopped being measured, which is the sweep trap
    /// one level up.
    #[test]
    fn a_path_sampled_short_of_the_assembled_position_is_not_a_pass() {
        let short = [s(0.0, "frame", 0.0), s(0.4, "frame", 0.0)];
        let a = Assembly {
            part: "blade".into(),
            installs: Some(insert(&short, 0.6).expect("valid")),
            operates: Some(first_stop(&clear_path(), None, 0.6).expect("valid")),
        };
        match a.fitness() {
            Fitness::Fails(v) => assert!(v[0].contains("has not reached"), "{:?}", v),
            other => panic!("0.4 is not the assembled position; got {other:?}"),
        }
    }

    /// Nothing measured at all is incomplete on both counts, not a pass and
    /// not a failure.
    #[test]
    fn a_part_nobody_checked_names_both_gaps() {
        let a = Assembly {
            part: "blade".into(),
            ..Default::default()
        };
        match a.fitness() {
            Fitness::Incomplete(v) => assert_eq!(v.len(), 2, "{v:?}"),
            other => panic!("expected both gaps, got {other:?}"),
        }
    }

    /// The insertion path inherits every refusal of the sweep machinery, so a
    /// path too coarse to draw a block from is refused here too.
    #[test]
    fn a_coarse_insertion_path_is_refused() {
        let coarse = [s(0.0, "frame", 0.0), s(1.0, "lip", 0.5)];
        assert!(insert(&coarse, 0.1).is_err());
    }
}
