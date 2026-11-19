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

    // PAIRING IS GLOBALLY CLOSEST-FIRST, NOT FIRST-FIT. rook, F6 on
    // thread:wm-review/9, DRIVEN: the old loop gave each before-contour the
    // FIRST after-contour within tolerance, so the verdict depended on the
    // order the contours happened to be listed in. Their case, at tol 1 mm --
    // before x=0 and x=1.8, after x=0.9 and x=0.1, where a perfect pairing
    // exists:
    //
    //     after listed [0.9, 0.1] -> ChangedInPlace, Removed, Added
    //     after listed [0.1, 0.9] -> ChangedInPlace, ChangedInPlace
    //
    // Same geometry, two answers, and the first CLAIMS STRUCTURAL CHANGE that
    // did not happen. An instrument whose answer depends on list order is not
    // measuring the geometry.
    //
    // So every candidate pair within tolerance is costed, and the cheapest
    // available pair is taken until none remain. Cost is the Chebyshev
    // distance between centroids, tie-broken by area difference; the tolerance
    // test is unchanged and still decides WHETHER a pair is eligible -- the
    // cost only ORDERS the eligible ones. Sorting by (cost, b index, a index)
    // makes the result a function of the contour SETS and not their order.
    //
    // AND A TIE REFUSES ONLY WHEN IT COULD CHANGE THE ANSWER. If another
    // eligible pair shares a contour with the chosen one at exactly the same
    // cost, the correspondence is decided by a tie-break rather than by the
    // geometry -- but that matters only if the two competitors DIFFER. Two
    // coincident contours are interchangeable: whichever is picked, one pairs
    // and the other is Removed, with identical fields either way. rook said
    // this part plainly when reporting F6 -- the exclusivity case uses
    // coincident contours, so order never mattered there -- and the first
    // version of this refusal broke that test by refusing an immaterial tie.
    // Equality is on every recorded field, so a tie survives only where the
    // competitors are indistinguishable and therefore the verdict is too.
    let cost_of = |b: &Contour, a: &Contour| -> (f64, f64) {
        let dx = (a.centroid_mm[0] - b.centroid_mm[0]).abs();
        let dy = (a.centroid_mm[1] - b.centroid_mm[1]).abs();
        (dx.max(dy), (a.area_mm2 - b.area_mm2).abs())
    };
    let eligible = |b: &Contour, a: &Contour| -> bool {
        (a.centroid_mm[0] - b.centroid_mm[0]).abs() <= centroid_tol_mm
            && (a.centroid_mm[1] - b.centroid_mm[1]).abs() <= centroid_tol_mm
            && (a.area_mm2 - b.area_mm2).abs() <= area_tol_mm2
    };

    let mut cand: Vec<(f64, f64, usize, usize)> = Vec::new();
    for (bi, b) in bc.iter().enumerate() {
        for (ai, a) in ac.iter().enumerate() {
            if eligible(b, a) {
                let (c, t) = cost_of(b, a);
                cand.push((c, t, bi, ai));
            }
        }
    }
    // total order, and NOT partial_cmp().unwrap(): every coordinate here is
    // already proven finite by the refusal above, so total_cmp is exact and
    // cannot panic on a NaN that cannot be present.
    cand.sort_by(|x, y| {
        x.0.total_cmp(&y.0)
            .then(x.1.total_cmp(&y.1))
            .then(x.2.cmp(&y.2))
            .then(x.3.cmp(&y.3))
    });

    let mut taken = vec![false; ac.len()];
    let mut paired_b = vec![None; bc.len()];
    let mut regions = Vec::new();
    let mut unchecked_pairs = 0usize;
    for k in 0..cand.len() {
        let (c, tb, bi, ai) = cand[k];
        if paired_b[bi].is_some() || taken[ai] {
            continue;
        }
        // A COMPETING PAIR AT THE SAME COST IS A TIE, and a tie is unknown.
        if let Some(&(_, _, bj, aj)) = cand[k + 1..].iter().find(|&&(c2, tb2, bj, aj)| {
            c2 == c
                && tb2 == tb
                && paired_b[bj].is_none()
                && !taken[aj]
                && (bj == bi || aj == ai)
                // the competitor must be DISTINGUISHABLE from the chosen one,
                // or the choice between them changes nothing that is reported
                && if bj == bi {
                    ac[aj] != ac[ai]
                } else {
                    bc[bj] != bc[bi]
                }
        }) {
            return Err(format!(
                "two pairings are equally close at {:?} {} mm -- before contour {bi}                  with after contour {ai}, and before {bj} with after {aj}, both at                  Chebyshev {c} mm and area difference {tb} mm2 -- so which contours                  correspond depends on a tie-break and not on the geometry. An                  ambiguous correspondence is unknown, never a reading",
                plane.axis, plane.offset_mm
            ));
        }
        paired_b[bi] = Some(ai);
        taken[ai] = true;
    }

    for (bi, b) in bc.iter().enumerate() {
        match paired_b[bi] {
            Some(i) => {
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
                // THE BBOX IS JUDGED AT THE SAME TOLERANCE THAT PAIRED THEM.
                // rook asked this as a question beside F6, and it is the same
                // incoherence: these two contours were declared the SAME
                // contour because their centroids and areas agree to within a
                // tolerance, and the change test then compared bbox_mm for
                // EXACT f64 equality. At that strictness any last-bit noise in
                // a coordinate reads as a reshape, and a reshape smaller than
                // the tolerance is not something this instrument can see. So
                // the bbox decides only when an edge moves by MORE than the
                // centroid tolerance; below that the honest answer is that
                // nothing measurable changed.
                let bbox_differs = b
                    .bbox_mm
                    .iter()
                    .zip(ac[i].bbox_mm.iter())
                    .any(|(x, y)| (x - y).abs() > centroid_tol_mm);
                if verts_differ || bbox_differs {
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

    /// rook's F6, thread:wm-review/9, as the regression that would have caught
    /// it. The old first-fit pairing gave a DIFFERENT verdict for the same
    /// geometry depending on the order the after-contours were listed in, and
    /// one of the two answers claimed structural change that never happened.
    #[test]
    fn the_pairing_does_not_depend_on_the_order_the_contours_are_listed_in() {
        let tol = Comparison::ContourIdentity {
            centroid_tol_mm: 1.0,
            area_tol_mm2: 1.0,
        };
        // area and vertices identical, so only the centroids decide; bbox
        // follows the centroid in c(), so a true pair reports no change.
        let before = cut(3.0, vec![c(0.0, 0.0, 10.0, 4), c(1.8, 0.0, 10.0, 4)]);
        let one = cut(3.0, vec![c(0.9, 0.0, 10.0, 4), c(0.1, 0.0, 10.0, 4)]);
        let other = cut(3.0, vec![c(0.1, 0.0, 10.0, 4), c(0.9, 0.0, 10.0, 4)]);

        let a = diff_sections(&before, &one, tol).expect("listed one way");
        let b = diff_sections(&before, &other, tol).expect("listed the other way");

        // A PERFECT PAIRING EXISTS -- 0 with 0.1 and 1.8 with 0.9, both inside
        // the 1 mm tolerance -- so nothing was added or removed either way.
        assert!(
            !a.regions
                .iter()
                .any(|r| matches!(r.kind, RegionKind::Added | RegionKind::Removed)),
            "first-fit claimed structural change: {:?}",
            a.regions
        );
        assert_eq!(
            a.regions.len(),
            b.regions.len(),
            "the same geometry gave two different answers: {:?} vs {:?}",
            a.regions,
            b.regions
        );
    }

    /// The other half of F6: a tie that COULD change the answer is unknown,
    /// never a reading. Two distinguishable after-contours sit equally close
    /// to one before-contour, so which one it corresponds to is decided by a
    /// tie-break, and the two give different verdicts.
    #[test]
    fn a_tie_between_distinguishable_contours_refuses_rather_than_tie_breaking() {
        let tol = Comparison::ContourIdentity {
            centroid_tol_mm: 1.0,
            area_tol_mm2: 1.0,
        };
        let before = cut(3.0, vec![c(0.0, 0.0, 10.0, 4)]);
        // equally far either side, and NOT interchangeable: different vertex
        // counts, so one pairing reports a reshape and the other does not
        let after = cut(3.0, vec![c(-0.5, 0.0, 10.0, 4), c(0.5, 0.0, 10.0, 9)]);
        let err = diff_sections(&before, &after, tol).expect_err("a material tie");
        assert!(
            err.contains("equally close") && err.contains("tie-break"),
            "{err}"
        );
    }

    /// And the immaterial tie still reads, because the competitors are
    /// indistinguishable: whichever is chosen, the reported fields are the
    /// same. This is the case rook pointed out order never mattered for, and
    /// the first version of the tie refusal broke it.
    #[test]
    fn a_tie_between_coincident_contours_still_reads() {
        let tol = Comparison::ContourIdentity {
            centroid_tol_mm: 1.0,
            area_tol_mm2: 1.0,
        };
        let before = cut(3.0, vec![c(0.0, 0.0, 10.0, 4), c(0.0, 0.0, 10.0, 4)]);
        let after = cut(3.0, vec![c(0.0, 0.0, 10.0, 4)]);
        let d = diff_sections(&before, &after, tol).expect("an immaterial tie");
        assert_eq!(
            d.regions
                .iter()
                .filter(|r| matches!(r.kind, RegionKind::Removed))
                .count(),
            1,
            "exclusivity: one of two coincident before-contours is Removed: {:?}",
            d.regions
        );
    }

    /// rook's question beside F6, answered as a decision: the bbox is judged
    /// at the tolerance that paired the contours. An exact f64 comparison made
    /// any last-bit coordinate noise read as a reshape, while declaring the
    /// same two contours identical for pairing.
    #[test]
    fn a_bbox_moving_less_than_the_pairing_tolerance_is_not_a_reshape() {
        let tol = Comparison::ContourIdentity {
            centroid_tol_mm: 0.5,
            area_tol_mm2: 0.5,
        };
        let before = cut(3.0, vec![c(0.0, 0.0, 10.0, 4)]);
        let mut moved = c(0.0, 0.0, 10.0, 4);
        moved.bbox_mm[0] += 0.1; // inside the 0.5 mm tolerance
        let within = diff_sections(&before, &cut(3.0, vec![moved]), tol).expect("within");
        assert!(
            within.unchanged(),
            "a sub-tolerance bbox move was reported as a reshape: {:?}",
            within.regions
        );

        let mut beyond = c(0.0, 0.0, 10.0, 4);
        beyond.bbox_mm[0] += 0.9; // outside it
        let out = diff_sections(&before, &cut(3.0, vec![beyond]), tol).expect("beyond");
        assert!(
            out.regions
                .iter()
                .any(|r| matches!(r.kind, RegionKind::ChangedInPlace)),
            "a bbox move beyond the tolerance must still report: {:?}",
            out.regions
        );
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
