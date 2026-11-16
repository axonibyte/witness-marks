//! Known-positive controls beside every probe.
//!
//! # Why a probe needs one
//!
//! A probe that reports "clear" has said one of two things and the caller
//! cannot tell which: *I looked and there was nothing*, or *I did not look*.
//! Those are the same bytes. The whole of failure class B is that the second
//! one reads like the first.
//!
//! A control is a deliberate positive run through the SAME probe in the SAME
//! call: a geometry that MUST bite. If the control did not bite, the probe is
//! not working, and a clear verdict from it means nothing and is reported as
//! such rather than as clear.
//!
//! # It caught one of ours
//!
//! The mutation driver in Foundations applied no mutation and reported green.
//! An empty FAIL list from a mutation run is the strongest negative there is,
//! and it came from a tool that had patched nothing. The same shape, one layer
//! down, is a probe whose geometry never reached the kernel.
//!
//! # The control must be DIFFERENT from the subject
//!
//! A control that is the subject proves only that the subject bites. These are
//! fixed, tiny, and independent of whatever is being measured.

use serde_json::Value;
use wm_envelope::Controls;

/// A control: a model, what it must show, and a human reason it is here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Control {
    pub name: &'static str,
    pub model: &'static str,
    pub why: &'static str,
}

/// A deliberate overlap of exactly 48 mm3: two 10 x 10 x 10 cubes offset so
/// they share a 10 x 10 x 0.48 slab. An interference probe that cannot see
/// this cannot see anything.
/// MEASURED: exactly 48.0000 mm3, 12 triangles, OpenSCAD 2026.10.14.
pub const OVERLAP: Control = Control {
    name: "deliberate-overlap",
    model: "intersection() { cube([10,10,10]); translate([0,0,9.52]) cube([10,10,10]); }",
    why: "an interference probe must report 48 mm3 here; if it reports clear, it is blind",
};

/// A deliberate ledge: an 8 x 8 roof on a 2 x 2 post, so 60 mm2 of it has
/// nothing underneath. An overhang scan that reports no unsupported area here
/// is not scanning.
///
/// MEASURED, NOT ASSUMED: 58.24 mm2 by the slicer criterion at 0.15 mm layers
/// and 36.87 degrees, against the 60 mm2 the geometry suggests. The 1.76 mm2
/// difference is the criterion working -- the first overhanging layer may step
/// out by d = layer / tan(angle) = 0.2 mm, which trims the flagged perimeter.
/// The commissioning issue said "a deliberate 48 mm2 ledge"; that figure does
/// not belong to this geometry and is not carried forward.
pub const LEDGE: Control = Control {
    name: "deliberate-ledge",
    model: "union() { cube([2,2,10]); translate([-3,-3,10]) cube([8,8,2]); }",
    why: "an overhang scan must flag ~58.24 mm2 of unsupported ledge here; silence means it is not scanning",
};

/// Did a control bite?
///
/// A control BITES when the probe reported a non-empty, positive result for
/// it. The test is deliberately crude: a control exists to tell apart "the
/// probe ran" from "the probe did not", not to measure anything.
pub fn bit(payload: &Value) -> bool {
    if payload
        .get("empty")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return false;
    }
    let vol = payload.get("volume").and_then(Value::as_f64).unwrap_or(0.0);
    let tris = payload
        .get("triangle_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    vol > 0.0 || tris > 0
}

/// The verdict on a probe, once its controls have been counted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trust {
    /// Every control bit. A clear result from this probe means clear.
    Trustworthy,
    /// At least one control did not bite. A clear result means NOTHING, and
    /// the reason is carried so the caller is told rather than left to infer.
    Blind(Vec<String>),
}

/// Tally controls into [`wm_envelope::Controls`] and a verdict.
///
/// A PROBE WITH NO CONTROLS IS BLIND, not trustworthy. Zero of zero biting is
/// vacuously true and is exactly what an unexercised probe reports; treating
/// it as clean would make the whole mechanism optional in practice.
pub fn tally(results: &[(Control, bool)]) -> (Controls, Trust) {
    let ran = results.len() as u32;
    let bit = results.iter().filter(|(_, b)| *b).count() as u32;
    let c = Controls { ran, bit };
    if ran == 0 {
        return (
            c,
            Trust::Blind(vec![
                "this probe ran NO controls, so a clear result from it is unverified".into(),
            ]),
        );
    }
    let dead: Vec<String> = results
        .iter()
        .filter(|(_, b)| !*b)
        .map(|(ctl, _)| {
            format!(
                "control {:?} did NOT bite: {}. A clear verdict from this probe means nothing.",
                ctl.name, ctl.why
            )
        })
        .collect();
    if dead.is_empty() {
        (c, Trust::Trustworthy)
    } else {
        (c, Trust::Blind(dead))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_positive_result_bites() {
        assert!(bit(&json!({"volume": 48.0, "triangle_count": 12})));
        assert!(bit(&json!({"triangle_count": 12})));
    }

    /// The three shapes of NOT biting, which are the three shapes of a probe
    /// that did not run.
    #[test]
    fn emptiness_a_zero_volume_and_an_absent_field_all_fail_to_bite() {
        assert!(
            !bit(&json!({"empty": true, "volume": 48.0})),
            "the flag wins"
        );
        assert!(!bit(&json!({"volume": 0.0, "triangle_count": 0})));
        assert!(!bit(&json!({})), "an absent measurement is not a bite");
    }

    #[test]
    fn every_control_biting_is_trustworthy() {
        let (c, t) = tally(&[(OVERLAP, true), (LEDGE, true)]);
        assert_eq!(c, Controls { ran: 2, bit: 2 });
        assert_eq!(t, Trust::Trustworthy);
    }

    /// THE ARM THIS MODULE EXISTS FOR. A probe whose control did not bite is
    /// reported as blind, NOT as clear.
    #[test]
    fn a_control_that_did_not_bite_makes_the_probe_blind_and_says_which() {
        let (c, t) = tally(&[(OVERLAP, false), (LEDGE, true)]);
        assert_eq!(c, Controls { ran: 2, bit: 1 });
        match t {
            Trust::Blind(why) => {
                assert_eq!(why.len(), 1);
                assert!(why[0].contains("deliberate-overlap"), "{why:?}");
                assert!(
                    why[0].contains("means nothing"),
                    "it must say what a clear verdict is worth: {why:?}"
                );
            }
            Trust::Trustworthy => panic!("a dead control must not read as trustworthy"),
        }
    }

    /// Zero of zero biting is VACUOUSLY TRUE, and is exactly what an
    /// unexercised probe reports. Treating it as clean would make controls
    /// optional in practice, which is the same as not having them.
    #[test]
    fn a_probe_with_no_controls_is_blind_not_clean() {
        let (c, t) = tally(&[]);
        assert_eq!(c, Controls { ran: 0, bit: 0 });
        match t {
            Trust::Blind(why) => assert!(why[0].contains("NO controls"), "{why:?}"),
            Trust::Trustworthy => panic!("no controls is not a pass"),
        }
    }

    /// A control must be independent of the subject: one that IS the subject
    /// proves only that the subject bites.
    #[test]
    fn the_controls_are_fixed_and_name_their_own_reason() {
        for c in [OVERLAP, LEDGE] {
            assert!(!c.model.is_empty());
            assert!(
                c.why.len() > 20,
                "a control carries the sentence that explains a silence"
            );
        }
        assert_ne!(OVERLAP.model, LEDGE.model);
    }

    /// THE CONTROLS' OWN NUMBERS ARE MEASURED AND PINNED. A control whose
    /// geometry silently changed would still "bite" while no longer being the
    /// deliberate positive anyone reasoned about, so the models are pinned by
    /// the figures they were measured to produce.
    ///
    ///   overlap: 48.0000 mm3 over 12 triangles
    ///   ledge:   168.0000 mm3, 58.24 mm2 unsupported at 0.15 mm / 36.87 deg
    ///
    /// Both taken with OpenSCAD 2026.10.14 on 2026-11-16.
    #[test]
    fn the_control_models_are_the_ones_that_were_measured() {
        assert!(
            OVERLAP.model.contains("translate([0,0,9.52])"),
            "the 0.48 mm overlap is what makes it 48 mm3; a different offset is a different control"
        );
        assert!(
            LEDGE.model.contains("translate([-3,-3,10]) cube([8,8,2])"),
            "the 8x8 roof on a 2x2 post is what makes the ledge"
        );
        assert!(
            LEDGE.why.contains("58.24"),
            "the control states the area it was measured to produce"
        );
    }
}
