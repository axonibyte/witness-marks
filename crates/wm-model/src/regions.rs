//! Where a change landed, not just how much of it there was (issue 51).
//!
//! # The hazard this module is built around
//!
//! Issue 51 states it, and issue 23 measured it in the printability half:
//! *"the provider answers a question of its own and nothing about a number
//! says which question was asked."* A region is a box and a centroid; two
//! regions from two different cut planes look exactly alike and mean entirely
//! different things. So a [`RegionDiff`] cannot be constructed without the
//! plane and the comparison that produced it, and [`diff_sections`] refuses a
//! pair cut on different planes rather than comparing them.
//!
//! # What this comparison is, and what it is not
//!
//! It is a CONTOUR-LEVEL comparison. The provider's `measure(mode=section)`
//! returns cut contours at `section_axis=section_offset`; contours are paired
//! across the two sections by centroid and area, and what pairs off is matched
//! while what does not is added or removed.
//!
//! It is NOT a polygon boolean difference. A contour that changed shape while
//! keeping its centroid and its area would pair with its old self, and this
//! comparison cannot say where inside it the change landed. That case is not
//! silently reported as unchanged -- it comes back as
//! [`RegionKind::ChangedInPlace`], which says a change is in there and that
//! this instrument cannot localize it. Naming the limit is the point; a
//! region diff that quietly rounded that case to "no change" would be the
//! same defect as the empty diff in [`crate::diff`], one layer along.

/// The axis a section was cut along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// The plane a section was cut on. Carried by every region set, because
/// nothing about a region's own numbers says which cut produced it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub axis: Axis,
    pub offset_mm: f64,
}

/// How two contour sets were turned into regions. Recorded rather than
/// assumed, so a reader can tell what the answer is worth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Comparison {
    /// Contours paired by centroid and area, within these tolerances. Not a
    /// polygon boolean: see the module docs.
    ContourIdentity {
        centroid_tol_mm: f64,
        area_tol_mm2: f64,
    },
}

/// One contour of a cut section, as the provider reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct Contour {
    /// Centroid in the cut plane's own two axes, in the provider's order.
    pub centroid_mm: [f64; 2],
    pub area_mm2: f64,
    /// Min and max in the cut plane: `[min_u, min_v, max_u, max_v]`.
    pub bbox_mm: [f64; 4],
    /// Closed-contour vertex count, when the provider gave one. `None` is
    /// UNKNOWN and not a count: a contour whose vertices nobody reported
    /// cannot be compared on them, and must not therefore read as reshaped.
    pub vertices: Option<usize>,
}

/// A section the provider cut, or its refusal to.
#[derive(Debug, Clone, PartialEq)]
pub enum Section {
    /// Contours the provider resolved. An empty vector is a real answer: the
    /// plane missed the solid.
    Cut {
        plane: Plane,
        contours: Vec<Contour>,
    },
    /// The provider could not resolve this cut. NOT an empty section -- an
    /// unanswered question is unknown, and unknown is not read.
    Unresolved { plane: Plane, why: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    /// Present after and not before.
    Added,
    /// Present before and not after.
    Removed,
    /// Paired across the cut, and the contour itself still moved. This
    /// comparison cannot say where inside it.
    ChangedInPlace,
}

/// One place the change landed, in the plane it was found in.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub kind: RegionKind,
    pub centroid_mm: [f64; 2],
    pub area_mm2: f64,
    pub bbox_mm: [f64; 4],
}

/// Regions, and the two facts that make them mean anything.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionDiff {
    pub plane: Plane,
    pub comparison: Comparison,
    pub regions: Vec<Region>,
    /// Pairs matched without vertex evidence, because one side or both
    /// reported no vertex count. The answer says what it could not check
    /// rather than leaving that in a doc comment: a caller reading
    /// `unchanged() == true` with this above zero has a weaker statement than
    /// one reading it with this at zero.
    pub unchecked_pairs: usize,
}

impl RegionDiff {
    /// Whether this cut found no change at all. Distinct from an empty
    /// section pair, which [`diff_sections`] refuses to call unchanged.
    pub fn unchanged(&self) -> bool {
        self.regions.is_empty()
    }

    /// Whether every pair was checked on its vertex counts as well as its
    /// box. `unchanged() && fully_checked()` is the strong statement; without
    /// this, "no regions" means "none this comparison could see".
    pub fn fully_checked(&self) -> bool {
        self.unchecked_pairs == 0
    }
}

fn finite(c: &Contour) -> bool {
    c.centroid_mm.iter().all(|v| v.is_finite())
        && c.bbox_mm.iter().all(|v| v.is_finite())
        && c.area_mm2.is_finite()
}

/// Compare two cut sections into the regions where they differ.
///
/// # What it refuses, and why each refusal is not a conservative no-op
///
/// * **Two different planes.** Comparing a cut at `z=3` against one at `z=7`
///   produces regions that are arithmetically fine and mean nothing. This is
///   the issue-51 hazard in its most direct form.
/// * **An unresolved section.** The provider's own house rule, which this
///   project already holds elsewhere: UNRESOLVED is not clear, and an
///   unanswered question must not read as "no contours here".
/// * **A non-finite coordinate.** A NaN centroid pairs with nothing, since
///   every comparison against NaN is false, so it would silently become an
///   added region and a removed one.
/// * **Both sections empty.** The plane missed the solid on both sides. There
///   is no change *in this plane* to report, and saying "unchanged" would
///   claim knowledge of a region the cut never touched.
pub fn diff_sections(
    before: &Section,
    after: &Section,
    comparison: Comparison,
) -> Result<RegionDiff, String> {
    let Comparison::ContourIdentity {
        centroid_tol_mm,
        area_tol_mm2,
    } = comparison;
    if !(centroid_tol_mm.is_finite() && centroid_tol_mm >= 0.0)
        || !(area_tol_mm2.is_finite() && area_tol_mm2 >= 0.0)
    {
        return Err("a pairing tolerance must be finite and non-negative".into());
    }
    let (plane, bc) = match before {
        Section::Unresolved { plane, why } => {
            return Err(format!(
                "the BEFORE section at {:?} {} mm is unresolved ({why}); an unanswered \
                 cut is unknown and must not read as having no contours",
                plane.axis, plane.offset_mm
            ))
        }
        Section::Cut { plane, contours } => (*plane, contours),
    };
    let (plane_a, ac) = match after {
        Section::Unresolved { plane, why } => {
            return Err(format!(
                "the AFTER section at {:?} {} mm is unresolved ({why}); an unanswered \
                 cut is unknown and must not read as having no contours",
                plane.axis, plane.offset_mm
            ))
        }
        Section::Cut { plane, contours } => (*plane, contours),
    };
    if plane.axis != plane_a.axis || plane.offset_mm != plane_a.offset_mm {
        return Err(format!(
            "these sections were cut on different planes -- {:?} at {} mm against {:?} \
             at {} mm -- and regions from different cuts are not comparable",
            plane.axis, plane.offset_mm, plane_a.axis, plane_a.offset_mm
        ));
    }
    if let Some(bad) = bc.iter().chain(ac.iter()).find(|c| !finite(c)) {
        return Err(format!(
            "a contour carries a non-finite coordinate ({:?}); it would pair with \
             nothing and be reported as both added and removed",
            bad.centroid_mm
        ));
    }
    if bc.is_empty() && ac.is_empty() {
        return Err(format!(
            "both sections are empty at {:?} {} mm: the cut missed the solid on both \
             sides, which is not the same as finding no change there",
            plane.axis, plane.offset_mm
        ));
    }

    let mut taken = vec![false; ac.len()];
    let mut regions = Vec::new();
    let mut unchecked_pairs = 0usize;
    for b in bc {
        let hit = ac.iter().enumerate().position(|(i, a)| {
            !taken[i]
                && (a.centroid_mm[0] - b.centroid_mm[0]).abs() <= centroid_tol_mm
                && (a.centroid_mm[1] - b.centroid_mm[1]).abs() <= centroid_tol_mm
                && (a.area_mm2 - b.area_mm2).abs() <= area_tol_mm2
        });
        match hit {
            Some(i) => {
                taken[i] = true;
                // PAIRED IS NOT THE SAME AS UNCHANGED. The pairing keys on
                // centroid and area; a contour can hold both and still have
                // moved, and this instrument cannot say where inside it.
                // EVIDENCE OF CHANGE, NOT ABSENCE OF EVIDENCE. The vertex
                // counts decide only when BOTH sides have one; an unknown
                // count compared against a known one is unknown, and
                // reporting it as a reshape would claim a measurement nobody
                // took. MEASURED: an earlier version defaulted a missing
                // count to a number and a surviving mutant proved the
                // justification for that default was false -- 0 and
                // usize::MAX behave identically in every case, because the
                // only cases are both-absent (equal either way) and
                // one-absent (unequal either way). The real answer is that
                // absence is not a value.
                let verts_differ = match (b.vertices, ac[i].vertices) {
                    (Some(x), Some(y)) => x != y,
                    _ => {
                        unchecked_pairs += 1;
                        false
                    }
                };
                if verts_differ || ac[i].bbox_mm != b.bbox_mm {
                    regions.push(Region {
                        kind: RegionKind::ChangedInPlace,
                        centroid_mm: ac[i].centroid_mm,
                        area_mm2: ac[i].area_mm2,
                        bbox_mm: ac[i].bbox_mm,
                    });
                }
            }
            None => regions.push(Region {
                kind: RegionKind::Removed,
                centroid_mm: b.centroid_mm,
                area_mm2: b.area_mm2,
                bbox_mm: b.bbox_mm,
            }),
        }
    }
    for (i, a) in ac.iter().enumerate() {
        if !taken[i] {
            regions.push(Region {
                kind: RegionKind::Added,
                centroid_mm: a.centroid_mm,
                area_mm2: a.area_mm2,
                bbox_mm: a.bbox_mm,
            });
        }
    }
    Ok(RegionDiff {
        plane,
        comparison,
        regions,
        unchecked_pairs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAIR: Comparison = Comparison::ContourIdentity {
        centroid_tol_mm: 0.01,
        area_tol_mm2: 0.01,
    };

    fn c(cx: f64, cy: f64, area: f64, verts: usize) -> Contour {
        Contour {
            centroid_mm: [cx, cy],
            area_mm2: area,
            bbox_mm: [cx - 1.0, cy - 1.0, cx + 1.0, cy + 1.0],
            vertices: Some(verts),
        }
    }

    fn cut(offset: f64, contours: Vec<Contour>) -> Section {
        Section::Cut {
            plane: Plane {
                axis: Axis::Z,
                offset_mm: offset,
            },
            contours,
        }
    }

    #[test]
    fn a_contour_present_only_after_is_an_added_region() {
        let d = diff_sections(
            &cut(3.0, vec![c(0.0, 0.0, 4.0, 4)]),
            &cut(3.0, vec![c(0.0, 0.0, 4.0, 4), c(20.0, 0.0, 9.0, 4)]),
            PAIR,
        )
        .expect("same plane, resolved");
        assert_eq!(d.regions.len(), 1);
        assert_eq!(d.regions[0].kind, RegionKind::Added);
        assert_eq!(d.regions[0].centroid_mm, [20.0, 0.0]);
        assert!(!d.unchanged());
    }

    #[test]
    fn a_contour_present_only_before_is_a_removed_region() {
        let d = diff_sections(
            &cut(3.0, vec![c(0.0, 0.0, 4.0, 4), c(20.0, 0.0, 9.0, 4)]),
            &cut(3.0, vec![c(0.0, 0.0, 4.0, 4)]),
            PAIR,
        )
        .expect("same plane");
        assert_eq!(d.regions.len(), 1);
        assert_eq!(d.regions[0].kind, RegionKind::Removed);
        assert_eq!(d.regions[0].area_mm2, 9.0);
    }

    #[test]
    fn an_unchanged_cut_reports_no_regions_and_still_names_its_plane() {
        let d = diff_sections(
            &cut(3.0, vec![c(0.0, 0.0, 4.0, 4)]),
            &cut(3.0, vec![c(0.0, 0.0, 4.0, 4)]),
            PAIR,
        )
        .expect("same plane");
        assert!(d.unchanged());
        // THE PLANE AND THE COMPARISON SURVIVE AN EMPTY ANSWER. "No regions"
        // from an unstated cut is the issue-51 hazard, and it is exactly the
        // answer a caller is most likely to carry somewhere else.
        assert_eq!(d.plane.offset_mm, 3.0);
        assert_eq!(d.plane.axis, Axis::Z);
        assert_eq!(d.comparison, PAIR);
        // AND THE STRENGTH OF THE STATEMENT IS PART OF IT: both contours
        // carried vertex counts, so "unchanged" here means checked, not
        // merely unobserved.
        assert!(d.fully_checked());
        assert_eq!(d.unchecked_pairs, 0);
    }

    #[test]
    fn a_paired_contour_that_still_moved_is_not_reported_as_unchanged() {
        // Same centroid, same area, different outline: the pairing cannot
        // distinguish these, and must not claim they are the same.
        let mut moved = c(0.0, 0.0, 4.0, 4);
        moved.vertices = Some(8);
        let d = diff_sections(
            &cut(3.0, vec![c(0.0, 0.0, 4.0, 4)]),
            &cut(3.0, vec![moved]),
            PAIR,
        )
        .expect("same plane");
        assert_eq!(d.regions.len(), 1);
        assert_eq!(d.regions[0].kind, RegionKind::ChangedInPlace);
        assert!(!d.unchanged());
    }

    #[test]
    fn two_different_planes_are_refused_rather_than_compared() {
        let err = diff_sections(
            &cut(3.0, vec![c(0.0, 0.0, 4.0, 4)]),
            &cut(7.0, vec![c(0.0, 0.0, 4.0, 4)]),
            PAIR,
        )
        .expect_err("different offsets must refuse");
        assert!(err.contains("different planes"), "{err}");
    }

    #[test]
    fn a_different_axis_at_the_same_offset_is_still_a_different_plane() {
        let x = Section::Cut {
            plane: Plane {
                axis: Axis::X,
                offset_mm: 3.0,
            },
            contours: vec![c(0.0, 0.0, 4.0, 4)],
        };
        let err = diff_sections(&x, &cut(3.0, vec![c(0.0, 0.0, 4.0, 4)]), PAIR)
            .expect_err("X at 3 is not Z at 3");
        assert!(err.contains("different planes"), "{err}");
    }

    #[test]
    fn an_unresolved_section_is_refused_on_either_side() {
        let un = Section::Unresolved {
            plane: Plane {
                axis: Axis::Z,
                offset_mm: 3.0,
            },
            why: "inside the tessellation error bound".into(),
        };
        let a = diff_sections(&un, &cut(3.0, vec![c(0.0, 0.0, 4.0, 4)]), PAIR)
            .expect_err("unresolved before");
        assert!(a.contains("BEFORE") && a.contains("unknown"), "{a}");
        let b = diff_sections(&cut(3.0, vec![c(0.0, 0.0, 4.0, 4)]), &un, PAIR)
            .expect_err("unresolved after");
        assert!(b.contains("AFTER") && b.contains("unknown"), "{b}");
    }

    #[test]
    fn two_empty_sections_are_refused_rather_than_called_unchanged() {
        let err = diff_sections(&cut(3.0, vec![]), &cut(3.0, vec![]), PAIR)
            .expect_err("the cut missed the solid on both sides");
        assert!(err.contains("missed the solid"), "{err}");
    }

    #[test]
    fn an_empty_section_on_one_side_only_is_a_real_answer() {
        // The plane missed the solid AFTER but not BEFORE: everything there
        // was removed, and that is a finding rather than a refusal.
        let d = diff_sections(
            &cut(3.0, vec![c(0.0, 0.0, 4.0, 4)]),
            &cut(3.0, vec![]),
            PAIR,
        )
        .expect("one side empty is an answer");
        assert_eq!(d.regions.len(), 1);
        assert_eq!(d.regions[0].kind, RegionKind::Removed);
    }

    #[test]
    fn a_non_finite_coordinate_is_refused_because_it_would_pair_with_nothing() {
        let mut nan = c(0.0, 0.0, 4.0, 4);
        nan.centroid_mm[0] = f64::NAN;
        let err = diff_sections(
            &cut(3.0, vec![nan]),
            &cut(3.0, vec![c(0.0, 0.0, 4.0, 4)]),
            PAIR,
        )
        .expect_err("NaN must refuse");
        assert!(err.contains("non-finite"), "{err}");
    }

    #[test]
    fn a_tolerance_that_is_not_a_number_is_refused() {
        let bad = Comparison::ContourIdentity {
            centroid_tol_mm: f64::NAN,
            area_tol_mm2: 0.01,
        };
        let err = diff_sections(&cut(3.0, vec![c(0.0, 0.0, 4.0, 4)]), &cut(3.0, vec![]), bad)
            .expect_err("a NaN tolerance pairs nothing and must refuse");
        assert!(err.contains("finite"), "{err}");
    }

    #[test]
    fn one_after_contour_cannot_pair_with_two_before_contours() {
        // Two before-contours inside each other's tolerance must not both
        // match the single after-contour: the second would read as unchanged
        // when it was removed.
        let d = diff_sections(
            &cut(3.0, vec![c(0.0, 0.0, 4.0, 4), c(0.0, 0.0, 4.0, 4)]),
            &cut(3.0, vec![c(0.0, 0.0, 4.0, 4)]),
            PAIR,
        )
        .expect("same plane");
        assert_eq!(d.regions.len(), 1);
        assert_eq!(d.regions[0].kind, RegionKind::Removed);
    }
}
