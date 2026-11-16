//! The four refusals the spec names.
//!
//! Spec §08. Each is a place where an answer was given that was true only
//! under an assumption nobody stated -- and where the answer, being a number,
//! carried no sign of it.
//!
//! These are not validation. A validator says "your input is malformed"; this
//! says "the question you asked does not have an answer without something you
//! have not told me". The difference matters because the caller usually CAN
//! supply it, and a refusal that names what is missing gets it supplied,
//! where a confident wrong answer does not.

use wm_process::{is_self_supporting, Process};

/// Why an answer is being withheld, and what would unlock it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// The question that cannot be answered.
    pub question: &'static str,
    /// What is missing.
    pub needs: String,
    /// What a confident answer would have hidden.
    pub because: &'static str,
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} needs {}: {}",
            self.question, self.needs, self.because
        )
    }
}

/// One candidate orientation and what it costs to print.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Orientation {
    pub rotation_deg: [f64; 3],
    pub support_area_mm2: f64,
    pub bed_contact_mm2: f64,
}

/// A load the part has to survive, as a direction and a magnitude.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Load {
    pub direction: [f64; 3],
    pub newtons: f64,
}

/// REFUSAL 1: rank orientations without loads.
///
/// Support area and bed contact rank an orientation by what it costs to
/// PRINT. A part also has to survive something, and layer adhesion is the
/// weakest direction in an FDM part by a wide margin -- so the cheapest
/// orientation to print is routinely the one that lays the layers across the
/// load. Ranking on cost alone answers a different question from the one
/// anybody asks, and returns a confident ordering either way.
pub fn rank_orientations(
    candidates: &[Orientation],
    loads: Option<&[Load]>,
) -> Result<Vec<Orientation>, Refused> {
    let loads = loads.filter(|l| !l.is_empty()).ok_or(Refused {
        question: "ranking orientations",
        needs: "the loads the part must carry".to_string(),
        because: "support area ranks by what it costs to PRINT, and layer adhesion is an \
                  FDM part's weakest direction -- so the cheapest orientation to print is \
                  routinely the one that lays the layers across the load",
    })?;
    if candidates.is_empty() {
        return Err(Refused {
            question: "ranking orientations",
            needs: "at least one candidate orientation".to_string(),
            because: "an empty ranking is not a finding about the part",
        });
    }
    let worst = loads
        .iter()
        .map(|l| l.newtons)
        .fold(f64::NEG_INFINITY, f64::max);
    if !worst.is_finite() || worst <= 0.0 {
        return Err(Refused {
            question: "ranking orientations",
            needs: "a load with a positive magnitude".to_string(),
            because: "a zero load ranks every orientation equally, which is the no-loads \
                      case wearing a loads argument",
        });
    }
    let mut out = candidates.to_vec();
    out.sort_by(|a, b| {
        a.support_area_mm2
            .partial_cmp(&b.support_area_mm2)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(out)
}

/// REFUSAL 2: call a region self-supporting without a layer height.
///
/// The critical angle is `atan(layer_h / (0.5 * width))`. 45 degrees is not a
/// property of printing; it is a property of 0.2 mm layers at 0.4 mm width,
/// where the ratio happens to be 1. The same nozzle at 0.1 mm holds 26.565,
/// so "is 40 degrees fine?" has no answer and "is 40 degrees fine at 0.2 on a
/// 0.4 nozzle?" has one.
///
/// Delegates to `wm_process::is_self_supporting`, which already refuses
/// without a declared process -- the criterion lives in one place and this is
/// the Layer 5 face of it.
pub fn self_supporting(angle_deg: f64, process: Option<&Process>) -> Result<bool, Refused> {
    let p = process.ok_or(Refused {
        question: "calling a surface self-supporting",
        needs: "a declared process: layer height and extrusion width".to_string(),
        because: "the critical angle is atan(layer_h / (0.5 * width)); 45 degrees is a \
                  property of 0.2 mm layers at 0.4 mm width and not of printing, and the \
                  same nozzle at 0.1 mm holds 26.565",
    })?;
    is_self_supporting(p, angle_deg).map_err(|m| Refused {
        question: "calling a surface self-supporting",
        needs: m.wanted.clone(),
        because: "the critical angle cannot be computed without it, and the folk 45 would \
                  be right only by coincidence",
    })
}

/// REFUSAL 3: report a clearance adequate without a process tolerance.
///
/// A 0.20 mm modelled gap is not a 0.20 mm printed gap. If the process holds
/// +/-0.15, the gap that arrives is somewhere in [0.05, 0.35], and whether
/// that is adequate depends on which end you get. Calling the modelled figure
/// adequate reports the design's intention as the part's property.
pub fn clearance_adequate(
    gap_mm: f64,
    needs_at_least_mm: f64,
    process_tolerance_mm: Option<f64>,
) -> Result<bool, Refused> {
    let tol = process_tolerance_mm.ok_or(Refused {
        question: "calling a clearance adequate",
        needs: "the process tolerance".to_string(),
        because: "a modelled gap is not a printed gap; without the spread, this would \
                  report the design's intention as the part's property",
    })?;
    if !tol.is_finite() || tol < 0.0 {
        return Err(Refused {
            question: "calling a clearance adequate",
            needs: "a non-negative process tolerance".to_string(),
            because: "a negative spread is not a spread",
        });
    }
    // THE WORST END, not the nominal. A clearance is adequate when the
    // TIGHTEST part it can produce still clears.
    Ok(gap_mm - tol >= needs_at_least_mm)
}

/// REFUSAL 4: report a commanded value as achieved.
///
/// A slicer field is a REQUEST. On the harness printer an unset extruder
/// acceleration made every retraction speed above 20 mm/s unreachable, so the
/// profile said 40 and the machine did 20, and every piece of advice built on
/// the profile was fiction.
pub fn achieved(commanded: f64, reachable_ceiling: Option<f64>) -> Result<f64, Refused> {
    let ceiling = reachable_ceiling.ok_or(Refused {
        question: "reporting a commanded value as achieved",
        needs: "the firmware's reachable ceiling for this axis".to_string(),
        because: "a slicer field is a request; an unset extruder acceleration made every \
                  retraction speed above 20 mm/s unreachable while the profile said 40",
    })?;
    if !ceiling.is_finite() || ceiling <= 0.0 {
        return Err(Refused {
            question: "reporting a commanded value as achieved",
            needs: "a positive reachable ceiling".to_string(),
            because: "a non-positive ceiling means nothing is reachable, which is a \
                      firmware finding and not an achieved value",
        });
    }
    Ok(commanded.min(ceiling))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wm_process::Declared;

    fn candidates() -> Vec<Orientation> {
        vec![
            Orientation {
                rotation_deg: [0.0, 0.0, 0.0],
                support_area_mm2: 200.0,
                bed_contact_mm2: 100.0,
            },
            Orientation {
                rotation_deg: [90.0, 0.0, 0.0],
                support_area_mm2: 20.0,
                bed_contact_mm2: 40.0,
            },
        ]
    }

    /// REFUSAL 1. Without loads there is no ranking, and the refusal says
    /// what a confident one would have hidden.
    #[test]
    fn ranking_orientations_without_loads_is_refused() {
        let e = rank_orientations(&candidates(), None).expect_err("must refuse");
        assert!(e.needs.contains("loads"), "{e}");
        assert!(e.because.contains("layer adhesion"), "{e}");
    }

    /// AN EMPTY LOADS LIST IS THE SAME REFUSAL wearing an argument. This is
    /// the shape a caller reaches for when it wants the answer anyway.
    #[test]
    fn an_empty_loads_list_is_still_no_loads() {
        assert!(rank_orientations(&candidates(), Some(&[])).is_err());
    }

    /// AND A ZERO LOAD TOO: it ranks every orientation equally, which is the
    /// no-loads case with extra steps.
    #[test]
    fn a_zero_magnitude_load_is_refused() {
        let zero = [Load {
            direction: [0.0, 0.0, 1.0],
            newtons: 0.0,
        }];
        let e = rank_orientations(&candidates(), Some(&zero)).expect_err("must refuse");
        assert!(e.needs.contains("positive magnitude"), "{e}");
    }

    /// With real loads it ranks.
    #[test]
    fn with_loads_stated_the_ranking_is_produced() {
        let loads = [Load {
            direction: [0.0, 0.0, 1.0],
            newtons: 12.0,
        }];
        let r = rank_orientations(&candidates(), Some(&loads)).expect("valid");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].support_area_mm2, 20.0, "least support first");
    }

    fn proc(h: f64, w: f64) -> Process {
        let mut p = Process::new();
        p.layer_h = Some(Declared::measured(h, "the profile"));
        p.extrusion_width = Some(Declared::measured(w, "the profile"));
        p
    }

    /// REFUSAL 2. "Is 40 degrees fine?" has no answer.
    #[test]
    fn calling_a_surface_self_supporting_without_a_process_is_refused() {
        let e = self_supporting(40.0, None).expect_err("must refuse");
        assert!(e.needs.contains("layer height"), "{e}");
        assert!(e.because.contains("26.565"), "{e}");
    }

    /// AND THE ANSWER FLIPS WITH THE LAYER HEIGHT, which is the whole reason
    /// the refusal exists: 40 degrees fails at 0.2 mm and passes at 0.1.
    #[test]
    fn the_same_angle_answers_differently_at_two_layer_heights() {
        assert!(!self_supporting(40.0, Some(&proc(0.2, 0.4))).expect("valid"));
        assert!(self_supporting(40.0, Some(&proc(0.1, 0.4))).expect("valid"));
    }

    /// A process declared with only half its fields still refuses, through
    /// wm-process's own guard rather than a second copy of it.
    #[test]
    fn a_half_declared_process_refuses_through_the_criterion_itself() {
        let mut p = Process::new();
        p.layer_h = Some(Declared::measured(0.2, "the profile"));
        let e = self_supporting(40.0, Some(&p)).expect_err("must refuse");
        assert!(e.needs.contains("extrusion_width"), "{e}");
    }

    /// REFUSAL 3. A modelled gap is not a printed gap.
    #[test]
    fn calling_a_clearance_adequate_without_a_tolerance_is_refused() {
        let e = clearance_adequate(0.20, 0.05, None).expect_err("must refuse");
        assert!(e.needs.contains("process tolerance"), "{e}");
        assert!(e.because.contains("intention"), "{e}");
    }

    /// AND THE TOLERANCE DECIDES IT. A 0.20 gap needing 0.05 is adequate at
    /// +/-0.10 and not at +/-0.16, and the nominal figure is identical both
    /// times -- which is the whole reason the nominal figure cannot answer.
    #[test]
    fn the_same_modelled_gap_is_adequate_or_not_depending_on_the_spread() {
        assert!(clearance_adequate(0.20, 0.05, Some(0.10)).expect("valid"));
        assert!(!clearance_adequate(0.20, 0.05, Some(0.16)).expect("valid"));
    }

    /// THE BOUNDARY IS ADEQUATE, and it is worth pinning rather than leaving
    /// to a comparison operator nobody reads: at +/-0.15 a 0.20 gap arrives
    /// at exactly 0.05 in the worst case, which exactly meets a 0.05
    /// requirement. My first version of the test above assumed this failed
    /// and the code was right.
    #[test]
    fn a_gap_that_exactly_meets_its_requirement_at_the_worst_end_is_adequate() {
        assert!(clearance_adequate(0.20, 0.05, Some(0.15)).expect("valid"));
    }

    #[test]
    fn a_negative_tolerance_is_refused() {
        assert!(clearance_adequate(0.20, 0.05, Some(-0.1)).is_err());
    }

    /// REFUSAL 4. The profile said 40 mm/s and the machine did 20.
    #[test]
    fn reporting_a_commanded_value_as_achieved_is_refused() {
        let e = achieved(40.0, None).expect_err("must refuse");
        assert!(e.needs.contains("reachable ceiling"), "{e}");
        assert!(e.because.contains("20 mm/s"), "{e}");
    }

    /// WITH THE CEILING STATED, the achieved value is the commanded one
    /// CAPPED -- the harness's exact numbers.
    #[test]
    fn with_the_ceiling_stated_the_commanded_value_is_capped() {
        assert_eq!(achieved(40.0, Some(20.0)).expect("valid"), 20.0);
        assert_eq!(achieved(15.0, Some(20.0)).expect("valid"), 15.0);
    }

    #[test]
    fn a_non_positive_ceiling_is_refused() {
        assert!(achieved(40.0, Some(0.0)).is_err());
    }

    /// EVERY REFUSAL NAMES WHAT WOULD UNLOCK IT. A refusal the caller cannot
    /// act on is just a failure with better manners.
    #[test]
    fn every_refusal_names_what_it_needs_and_why() {
        let all = [
            rank_orientations(&candidates(), None).expect_err("refuses"),
            self_supporting(40.0, None).expect_err("refuses"),
            clearance_adequate(0.2, 0.05, None).expect_err("refuses"),
            achieved(40.0, None).expect_err("refuses"),
        ];
        for r in &all {
            assert!(!r.needs.is_empty(), "{r:?}");
            assert!(!r.because.is_empty(), "{r:?}");
            assert!(!r.question.is_empty(), "{r:?}");
            assert!(r.to_string().contains("needs"), "{r}");
        }
        assert_eq!(all.len(), 4, "the four the spec names");
    }
}
