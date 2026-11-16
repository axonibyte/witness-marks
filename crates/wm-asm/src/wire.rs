//! Reading `check(mode=motion)` into this crate's judgments.
//!
//! The provider sweeps a named part along a vector or about an axis and
//! reports `first_contact` (the offset and the part met there), `worst` (the
//! deepest penetration anywhere in the sweep) and `min_gap_mm`. That is the
//! measurement half of both [`crate::travel`] and [`crate::insert`].
//!
//! # Two readings of one payload, and still two types
//!
//! The same sweep answers "what stops it" and "can it be installed", and this
//! module gives each its own function returning its own type. There is still
//! no conversion between [`crate::travel::Stop`] and
//! [`crate::insert::Access`], because the defect those types exist to prevent
//! is a call site holding one answer and reading it as both. Sharing a
//! transport does not make them the same question: an operating sweep and an
//! insertion path are different sweeps, and which one was run is the caller's
//! knowledge, not the payload's.
//!
//! # UNRESOLVED is not clear
//!
//! The provider reports distances inside its tessellation error bound as
//! UNRESOLVED rather than as zero -- the one refusal the spec credits it with
//! having that this project lacked. A sweep carrying unresolved rows has not
//! established that the part travels; it has established that some of the
//! question was below the resolution of the instrument. Reading that as PASS
//! is the whole family of error this project is about.

use crate::insert::Access;
use crate::travel::Stop;
use serde_json::Value;

/// The one sweep finding, checked for the things that make it unreadable.
struct Finding<'a> {
    status: &'a str,
    first_contact: Option<&'a Value>,
}

fn finding<'a>(payload: &'a Value, span: f64, max_gap: f64) -> Result<Finding<'a>, String> {
    if payload.get("success").and_then(Value::as_bool) == Some(false) {
        return Err("the provider reported success=false on the sweep".into());
    }
    let unresolved = payload
        .get("summary")
        .and_then(|s| s.get("unresolved"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if unresolved > 0 {
        return Err(format!(
            "the sweep carries {unresolved} UNRESOLVED row(s) -- distances inside the \
             provider's tessellation error bound. That is not a clear sweep; it is part of \
             the question falling below the resolution of the instrument, and reading it as \
             a pass is the error this whole layer exists to prevent"
        ));
    }
    let findings = payload
        .get("findings")
        .and_then(Value::as_array)
        .ok_or_else(|| "the answer carries no `findings` array".to_string())?;
    let f = findings.first().ok_or_else(|| {
        "the sweep produced NO finding. No finding and no contact are the same bytes, and \
         only one of them is a measurement"
            .to_string()
    })?;
    let steps = f.get("steps").and_then(Value::as_u64).unwrap_or(0);
    if steps < 2 {
        return Err(format!(
            "the sweep reports {steps} step(s); fewer than two cannot describe travel"
        ));
    }
    // THE SAME COARSE-SWEEP REFUSAL THE JUDGMENT LAYER MAKES, applied to the
    // provider's own sampling. A contact between two steps is reported at the
    // LATER one, which over-states how far the part gets -- the dangerous
    // direction -- so the step spacing is checked against the caller's limit.
    if !span.is_finite() || span <= 0.0 {
        return Err(format!("the sweep span is {span}, which is not a range"));
    }
    let gap = span / (steps as f64 - 1.0);
    if !max_gap.is_finite() || max_gap <= 0.0 {
        return Err(format!(
            "max_gap must be positive and finite, got {max_gap}"
        ));
    }
    if gap > max_gap {
        return Err(format!(
            "the sweep took {steps} step(s) over {span}, a spacing of {gap}, past the \
             {max_gap} limit. A contact inside that spacing is reported at the LATER step, \
             which over-states the travel"
        ));
    }
    Ok(Finding {
        status: f.get("status").and_then(Value::as_str).unwrap_or(""),
        first_contact: f.get("first_contact").filter(|v| !v.is_null()),
    })
}

fn contact_part_and_offset(fc: &Value) -> Result<(String, f64), String> {
    let offset = fc
        .get("offset_mm")
        .and_then(Value::as_f64)
        .ok_or_else(|| "first_contact carries no offset_mm".to_string())?;
    let contacts = fc
        .get("contacts")
        .and_then(Value::as_array)
        .ok_or_else(|| "first_contact carries no contacts".to_string())?;
    // THE DEEPEST CONTACT AT THAT OFFSET IS NAMED, not the first listed. Two
    // parts meeting at once is real -- the incident this crate was built from
    // had both jambs touching together -- and whichever sorted first would be
    // arbitrary.
    let worst = contacts
        .iter()
        .max_by(|a, b| {
            let d = |v: &Value| {
                v.get("penetration_mm")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
            };
            d(a).partial_cmp(&d(b)).unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or_else(|| {
            "first_contact names an EMPTY contacts list: a contact with nothing in it is not \
             a contact"
                .to_string()
        })?;
    let part = worst
        .get("part")
        .and_then(Value::as_str)
        .ok_or_else(|| "a contact names no part".to_string())?;
    Ok((part.to_string(), offset))
}

/// Read a sweep as an INSERTION path: can the part be gotten in.
///
/// `span` is the length of the travel the caller asked for, used to check the
/// provider's step spacing and to say how far the path was actually swept.
pub fn access_from_motion(payload: &Value, span: f64, max_gap: f64) -> Result<Access, String> {
    let f = finding(payload, span, max_gap)?;
    match f.first_contact {
        Some(fc) => {
            let (part, offset) = contact_part_and_offset(fc)?;
            Ok(Access::Blocked {
                by: part,
                at_t: offset,
            })
        }
        None => {
            if f.status == "FAIL" {
                return Err(format!(
                    "the sweep is marked {} and yet names no first_contact. One of those two \
                     readings is wrong, and guessing which would make a failure look clear",
                    f.status
                ));
            }
            Ok(Access::Clear { sampled_to: span })
        }
    }
}

/// Read the same sweep as an OPERATING travel: what stops it first.
pub fn stop_from_motion(
    payload: &Value,
    span: f64,
    max_gap: f64,
    intended: Option<&str>,
) -> Result<Stop, String> {
    let f = finding(payload, span, max_gap)?;
    match f.first_contact {
        Some(fc) => {
            let (part, offset) = contact_part_and_offset(fc)?;
            Ok(Stop::Stopped {
                remaining: if span > 0.0 {
                    (span - offset) / span
                } else {
                    0.0
                },
                as_intended: intended.map(|i| i == part),
                part,
                at_t: offset,
            })
        }
        None => {
            if f.status == "FAIL" {
                return Err(format!(
                    "the sweep is marked {} and names no first_contact",
                    f.status
                ));
            }
            Ok(Stop::Clear { sampled_to: span })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fx(case: &str) -> Value {
        let raw = include_str!("../../../tests/fixtures/provider-motion.json");
        let all: Value = serde_json::from_str(raw).expect("the captured fixture parses");
        all[case]["answer"].clone()
    }

    /// THE BLOCKED SWEEP. The blade spans x 0..5 and the frame stands at
    /// x=10, so geometry says it contacts after 5 mm, and the provider says
    /// 5.0.
    #[test]
    fn a_blocked_sweep_names_the_part_and_the_offset_it_met_it_at() {
        match access_from_motion(&fx("blocked"), 10.0, 1.5).expect("valid") {
            Access::Blocked { by, at_t } => {
                assert_eq!(by, "frame");
                assert_eq!(
                    at_t, 5.0,
                    "the blade's far face reaches the frame after 5 mm"
                );
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    /// THE CLEAR SWEEP, which is why the fixture has two cases. A reader that
    /// always reported Blocked would pass against a one-case fixture.
    #[test]
    fn a_clear_sweep_reports_how_far_it_was_actually_swept() {
        match access_from_motion(&fx("clear"), 10.0, 1.5).expect("valid") {
            Access::Clear { sampled_to } => assert_eq!(sampled_to, 10.0),
            other => panic!("the frame is lifted 40 mm clear; got {other:?}"),
        }
    }

    /// The same payload read as a travel stop, with the intended stop checked.
    #[test]
    fn the_same_sweep_read_as_a_travel_stop_judges_the_intended_part() {
        match stop_from_motion(&fx("blocked"), 10.0, 1.5, Some("can")).expect("valid") {
            Stop::Stopped {
                part,
                at_t,
                remaining,
                as_intended,
            } => {
                assert_eq!(part, "frame");
                assert_eq!(at_t, 5.0);
                assert!((remaining - 0.5).abs() < 1e-12);
                assert_eq!(as_intended, Some(false), "the can was meant to stop it");
            }
            other => panic!("expected a stop, got {other:?}"),
        }
    }

    /// UNRESOLVED IS NOT CLEAR. The provider's own refusal -- distances inside
    /// its tessellation bound -- must not read as a travelled sweep.
    #[test]
    fn an_unresolved_sweep_is_refused_rather_than_read_as_clear() {
        let mut v = fx("clear");
        v["summary"]["unresolved"] = Value::from(3u64);
        let e = access_from_motion(&v, 10.0, 1.5).expect_err("must refuse");
        assert!(e.contains("not a clear sweep"), "{e}");
        assert!(e.contains("resolution of the instrument"), "{e}");
    }

    /// A SWEEP WITH NO FINDING measured nothing, and that is not travel.
    #[test]
    fn a_sweep_that_produced_no_finding_is_refused() {
        let mut v = fx("clear");
        v["findings"] = Value::Array(vec![]);
        let e = access_from_motion(&v, 10.0, 1.5).expect_err("must refuse");
        assert!(e.contains("only one of them is a measurement"), "{e}");
    }

    /// THE PROVIDER'S OWN SAMPLING IS CHECKED. 11 steps over 10 mm is 1.0 mm
    /// spacing; under a 0.5 mm limit that is too coarse to draw a stop from.
    #[test]
    fn a_sweep_too_coarsely_stepped_for_the_callers_limit_is_refused() {
        let e = access_from_motion(&fx("blocked"), 10.0, 0.5).expect_err("must refuse");
        assert!(e.contains("over-states the travel"), "{e}");
    }

    /// And the same sweep passes at a limit it meets, so the refusal is about
    /// the spacing and not about the fixture.
    #[test]
    fn the_same_sweep_is_accepted_at_a_limit_it_meets() {
        assert!(access_from_motion(&fx("blocked"), 10.0, 1.0).is_ok());
    }

    /// A FAIL WITH NO CONTACT IS A CONTRADICTION, not a clear path. Guessing
    /// which half to believe would make a failure look clear.
    #[test]
    fn a_failing_sweep_that_names_no_contact_is_refused() {
        let mut v = fx("clear");
        v["findings"][0]["status"] = Value::String("FAIL".into());
        let e = access_from_motion(&v, 10.0, 1.5).expect_err("must refuse");
        assert!(e.contains("One of those two readings is wrong"), "{e}");
    }

    /// An empty contacts list under a first_contact is not a contact.
    ///
    /// THE MESSAGE IS ASSERTED, not merely the error. Removing this guard
    /// still produced an error -- from the "a contact names no part" check
    /// further down -- so a bare `is_err()` passed with the guard gone and
    /// could not say which one had fired.
    #[test]
    fn a_first_contact_with_no_contacts_is_refused() {
        let mut v = fx("blocked");
        v["findings"][0]["first_contact"]["contacts"] = Value::Array(vec![]);
        let e = access_from_motion(&v, 10.0, 1.5).expect_err("must refuse");
        assert!(
            e.contains("a contact with nothing in it is not a contact"),
            "the empty-contacts guard must be the one that fires, got: {e}"
        );
    }

    /// TWO PARTS MEETING AT ONE OFFSET: the deeper is named, not the first
    /// listed. Both jambs touching at once is the incident this came from.
    #[test]
    fn when_two_parts_meet_at_one_offset_the_deeper_is_named() {
        let mut v = fx("blocked");
        v["findings"][0]["first_contact"]["contacts"] = serde_json::json!([
            {"part": "frame", "penetration_mm": 0.001},
            {"part": "can",   "penetration_mm": 0.900}
        ]);
        match access_from_motion(&v, 10.0, 1.5).expect("valid") {
            Access::Blocked { by, .. } => assert_eq!(by, "can"),
            other => panic!("expected Blocked by the deeper part, got {other:?}"),
        }
    }

    #[test]
    fn an_unsuccessful_sweep_is_refused() {
        let mut v = fx("clear");
        v["success"] = Value::Bool(false);
        assert!(access_from_motion(&v, 10.0, 1.5).is_err());
    }
}
