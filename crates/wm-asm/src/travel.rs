//! What stops a moving part first, and whether it is the part intended.
//!
//! # The incident
//!
//! buildability 74. The blade's cutting edge sweeps outward in ANGLE as it
//! advances, so it reaches further round the mouth the deeper it cuts. Static
//! interference at rest was **exactly zero** -- every check passed -- and the
//! blade still could not travel: `frame X blade` measured 0.009 mm3 at 1.0 mm
//! of advance and 0.158 at 1.5, in two pieces, one per jamb.
//!
//! The travel stop was the FRAME. The design intended the can to stop it.
//!
//! # Zero at rest is the whole trap
//!
//! A moving part has to be checked THROUGH ITS TRAVEL, and the assembled
//! position is not a sample of that -- it is the one position where everything
//! was designed to be clear. A sweep that starts at t = 0 and finds nothing
//! has found nothing about t > 0.

/// One sample of one part's interference at one point in the travel.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    /// Where in the travel, 0 to 1 or any monotone parameter.
    pub t: f64,
    pub part: String,
    /// Interference volume at this t. Zero means clear.
    pub volume: f64,
}

/// What stopped the motion.
#[derive(Debug, Clone, PartialEq)]
pub enum Stop {
    /// Nothing interfered anywhere in the sampled range.
    Clear {
        /// The largest `t` actually sampled. A caller that asked for 0 to 1
        /// and sampled to 0.4 has NOT cleared 1.0, and this is how they see
        /// that.
        sampled_to: f64,
    },
    /// Something stopped it.
    Stopped {
        part: String,
        at_t: f64,
        /// How much travel is left beyond `at_t`, as a fraction of the
        /// sampled range.
        remaining: f64,
        /// The intended stop, if the caller named one, and whether this is it.
        as_intended: Option<bool>,
    },
}

/// Find the first interference in a sweep.
///
/// # Samples are a lower bound on the truth
///
/// A sweep sees what it sampled. The first interference it reports is the
/// first one AT A SAMPLE, and the real contact is somewhere between that
/// sample and the one before. Reporting the sample's `t` as "where it stops"
/// is an over-estimate of the travel, and that is the dangerous direction --
/// so the gap between samples is checked and a coarse sweep is refused.
///
/// REFUSES AN EMPTY SWEEP. No samples and no interference is not "it travels
/// freely"; it is "nobody looked", and those are the same bytes otherwise.
pub fn first_stop(
    samples: &[Sample],
    intended: Option<&str>,
    max_gap: f64,
) -> Result<Stop, String> {
    if samples.is_empty() {
        return Err(
            "an empty sweep: no samples and therefore no interference, which is \
                    not the same as travelling freely. Refusing rather than reporting \
                    Clear over nothing"
                .into(),
        );
    }
    if !max_gap.is_finite() || max_gap <= 0.0 {
        return Err(format!(
            "max_gap must be positive and finite, got {max_gap}"
        ));
    }
    let mut ts: Vec<f64> = samples.iter().map(|s| s.t).collect();
    ts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    ts.dedup_by(|a, b| (*a - *b).abs() < f64::EPSILON);
    if ts.iter().any(|t| !t.is_finite()) {
        return Err("a sample has a non-finite t".into());
    }
    for w in ts.windows(2) {
        if w[1] - w[0] > max_gap {
            return Err(format!(
                "the sweep jumps from t={} to t={}, a gap of {}, over the {max_gap} limit. \
                 A contact inside that gap would be reported as happening at the LATER \
                 sample, which over-states the travel -- the dangerous direction",
                w[0],
                w[1],
                w[1] - w[0]
            ));
        }
    }
    let lo = *ts.first().unwrap_or(&0.0);
    let hi = *ts.last().unwrap_or(&0.0);
    let span = hi - lo;

    let mut hits: Vec<&Sample> = samples.iter().filter(|s| s.volume > 0.0).collect();
    hits.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
    match hits.first() {
        None => Ok(Stop::Clear { sampled_to: hi }),
        Some(first) => {
            // EVERY PART TOUCHING AT THAT SAME t IS CONSIDERED, and the one
            // with the LARGEST interference is named. Two parts contacting at
            // once is a real thing, and naming whichever sorted first would
            // be arbitrary.
            let at = first.t;
            let worst = hits
                .iter()
                .filter(|s| (s.t - at).abs() < f64::EPSILON)
                .max_by(|a, b| {
                    a.volume
                        .partial_cmp(&b.volume)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(first);
            Ok(Stop::Stopped {
                part: worst.part.clone(),
                at_t: at,
                remaining: if span > 0.0 { (hi - at) / span } else { 0.0 },
                as_intended: intended.map(|i| i == worst.part),
            })
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

    /// THE INCIDENT. Static interference at rest is EXACTLY ZERO and the
    /// blade still cannot travel: the frame takes it at 1.0 mm of advance.
    #[test]
    fn zero_at_rest_and_stopped_by_the_frame_further_along() {
        let sweep = [
            s(0.0, "frame", 0.0),
            s(0.5, "frame", 0.0),
            s(1.0, "frame", 0.009),
            s(1.5, "frame", 0.158),
        ];
        match first_stop(&sweep, Some("can"), 0.6).expect("valid") {
            Stop::Stopped {
                part,
                at_t,
                as_intended,
                ..
            } => {
                assert_eq!(part, "frame");
                assert_eq!(at_t, 1.0);
                assert_eq!(
                    as_intended,
                    Some(false),
                    "the design intended the CAN to stop it"
                );
            }
            other => panic!("the frame stops it, got {other:?}"),
        }
    }

    /// A SWEEP THAT ONLY SAMPLES THE ASSEMBLED POSITION FINDS NOTHING, and
    /// that is the trap: t = 0 is the one position everything was designed to
    /// be clear at.
    #[test]
    fn sampling_only_the_rest_position_reports_clear_only_to_there() {
        match first_stop(&[s(0.0, "frame", 0.0)], None, 0.6).expect("valid") {
            Stop::Clear { sampled_to } => assert_eq!(
                sampled_to, 0.0,
                "it has cleared t=0 and says so, rather than claiming the travel"
            ),
            other => panic!("expected Clear, got {other:?}"),
        }
    }

    /// AN EMPTY SWEEP IS REFUSED. No samples and no interference is "nobody
    /// looked", which is otherwise the same bytes as "it travels freely".
    #[test]
    fn an_empty_sweep_is_refused_rather_than_reported_clear() {
        let e = first_stop(&[], None, 0.6).expect_err("must refuse");
        assert!(e.contains("not the same as travelling freely"), "{e}");
    }

    /// A COARSE SWEEP IS REFUSED, because a contact inside the gap would be
    /// reported at the LATER sample and over-state the travel.
    #[test]
    fn a_sweep_too_coarse_to_trust_is_refused() {
        let sweep = [s(0.0, "frame", 0.0), s(1.0, "frame", 0.5)];
        let e = first_stop(&sweep, None, 0.1).expect_err("must refuse");
        assert!(e.contains("over-states the travel"), "{e}");
        assert!(e.contains("dangerous direction"), "{e}");
    }

    /// The intended stop being the actual stop is the good case, and it is
    /// reported as such rather than silently passing.
    #[test]
    fn the_intended_stop_is_confirmed_when_it_is_the_one() {
        let sweep = [s(0.0, "can", 0.0), s(0.5, "can", 0.2)];
        match first_stop(&sweep, Some("can"), 0.6).expect("valid") {
            Stop::Stopped { as_intended, .. } => assert_eq!(as_intended, Some(true)),
            other => panic!("expected a stop, got {other:?}"),
        }
    }

    /// TWO PARTS TOUCHING AT ONCE is real, and naming whichever sorted first
    /// would be arbitrary. The larger interference is named.
    #[test]
    fn when_two_parts_contact_together_the_deeper_one_is_named() {
        let sweep = [
            s(0.0, "frame", 0.0),
            s(0.0, "can", 0.0),
            s(0.5, "frame", 0.001),
            s(0.5, "can", 0.900),
        ];
        match first_stop(&sweep, None, 0.6).expect("valid") {
            Stop::Stopped { part, .. } => assert_eq!(part, "can"),
            other => panic!("expected a stop, got {other:?}"),
        }
    }

    /// Remaining range is a FRACTION of what was swept, so it means something
    /// without knowing the units.
    #[test]
    fn the_remaining_range_is_a_fraction_of_the_sweep() {
        let sweep = [
            s(0.0, "frame", 0.0),
            s(0.25, "frame", 0.1),
            s(0.5, "frame", 0.2),
            s(0.75, "frame", 0.3),
            s(1.0, "frame", 0.4),
        ];
        // IT STOPS A QUARTER IN, not halfway. A stop at the midpoint makes
        // remaining and elapsed the SAME NUMBER, and a fixture that cannot
        // tell the two apart passes whichever one the code computes.
        match first_stop(&sweep, None, 0.3).expect("valid") {
            Stop::Stopped { remaining, .. } => {
                assert!(
                    (remaining - 0.75).abs() < 1e-12,
                    "0.75 of the sweep is left, not the 0.25 elapsed; got {remaining}"
                )
            }
            other => panic!("expected a stop, got {other:?}"),
        }
    }

    /// Samples out of order are still read in order of t, because a caller
    /// collecting results in parallel has no reason to have sorted them.
    #[test]
    fn samples_need_not_arrive_in_order() {
        let sweep = [
            s(1.0, "frame", 0.158),
            s(0.0, "frame", 0.0),
            s(0.5, "frame", 0.009),
        ];
        match first_stop(&sweep, None, 0.6).expect("valid") {
            Stop::Stopped { at_t, .. } => {
                assert_eq!(at_t, 0.5, "the FIRST contact, not the first listed")
            }
            other => panic!("expected a stop, got {other:?}"),
        }
    }

    #[test]
    fn a_non_finite_sample_is_refused() {
        let sweep = [s(0.0, "frame", 0.0), s(f64::NAN, "frame", 0.1)];
        assert!(first_stop(&sweep, None, 0.6).is_err());
    }
}
