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
    /// Whether this contour bounds a HOLE rather than solid material.
    ///
    /// MEASURED 2026-11-19 on the pinned provider (rook's F11): a section
    /// reply carries `hole_count` but NOTHING saying which contour is the
    /// hole. The winding does: for a ring at $fn=8 the outer contour's signed
    /// shoelace area is +282.844 and the inner's is -70.710, and their sum,
    /// 212.134, is the `area` the provider reports for the whole section.
    /// So orientation is the provider's own statement of hole-ness, and
    /// `wire::section_from` reads it rather than guessing from size.
    pub is_hole: bool,
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

    // PAIRING IS A MAXIMUM-CARDINALITY MATCHING, AND THE REPORT MUST BE THE
    // SAME FOR EVERY ONE OF THEM. Two findings from rook took this through
    // three versions, and the third is the one that is actually defensible.
    //
    // F6 (wm-review/9): the first version was FIRST-FIT, so the verdict
    // depended on the order the contours happened to be listed in.
    //
    // F9 (wm-review/13): the second version took the globally cheapest pair
    // first. Order stopped mattering and the instrument could still MISS A
    // FULL PAIRING. rook drove it -- tol 1 mm, before x=0 and x=1.2, after
    // x=0.3 and x=-0.5: the cheapest pair (0 to 0.3, cost 0.3) consumes
    // 1.2's ONLY eligible partner, and the answer came back
    // ["Removed", "Added"] even though 0 to -0.5 and 1.2 to 0.3 are both
    // inside the tolerance. Greedy maximises the first choice, not the number
    // of correspondences, and it is the NUMBER that decides whether this
    // instrument claims something was added or removed.
    //
    // F10 (same line): my tie test compared costs with `==` on f64. By this
    // project's OWN rule for reported figures (issue 54, and `reported_interval`
    // in tools/vector-run.py), a provider reporting four decimals cannot
    // separate two costs closer than 1e-4 -- so an exact-equality tie test
    // decides correspondence on digits nobody vouched for.
    //
    // THE FIX ANSWERS BOTH AT ONCE BY MAKING COST DECIDE NOTHING REPORTABLE.
    // Maximum cardinality is computed first, by augmenting paths, which is
    // what rook prescribed and is what settles Added/Removed. Then EVERY
    // maximum matching is enumerated and the report each would produce is
    // compared. If they all agree, that report is the answer and which
    // matching produced it is immaterial. If any two disagree, the
    // correspondence is genuinely ambiguous and this refuses. Cost is gone
    // entirely: nothing is ordered by a float comparison, so F10's digits
    // cannot reach the verdict.
    let eligible: Vec<Vec<usize>> = bc
        .iter()
        .map(|b| {
            ac.iter()
                .enumerate()
                .filter(|(_, a)| {
                    // A HOLE IS NOT A SOLID CONTOUR OF THE SAME SIZE. They can
                    // agree on centroid and area exactly and still be opposite
                    // statements about the material, so they are never
                    // eligible for each other -- otherwise a solid becoming a
                    // void reads as "unchanged".
                    a.is_hole == b.is_hole
                        && (a.centroid_mm[0] - b.centroid_mm[0]).abs() <= centroid_tol_mm
                        && (a.centroid_mm[1] - b.centroid_mm[1]).abs() <= centroid_tol_mm
                        && (a.area_mm2 - b.area_mm2).abs() <= area_tol_mm2
                })
                .map(|(i, _)| i)
                .collect()
        })
        .collect();

    // Kuhn's algorithm: the size of a maximum matching, in polynomial time.
    // This is the figure that decides how many contours are Added or Removed,
    // so it is computed exactly rather than approached greedily.
    let max_k = {
        let mut to_b: Vec<Option<usize>> = vec![None; ac.len()];
        fn augment(
            b: usize,
            eligible: &[Vec<usize>],
            to_b: &mut Vec<Option<usize>>,
            seen: &mut Vec<bool>,
        ) -> bool {
            for &a in &eligible[b] {
                if seen[a] {
                    continue;
                }
                seen[a] = true;
                // no unwrap: the two cases are "this after-contour is free"
                // and "it is taken, so try to re-house whoever holds it"
                let can_take = match to_b[a] {
                    None => true,
                    Some(held) => augment(held, eligible, to_b, seen),
                };
                if can_take {
                    to_b[a] = Some(b);
                    return true;
                }
            }
            false
        }
        let mut k = 0usize;
        for b in 0..bc.len() {
            let mut seen = vec![false; ac.len()];
            if augment(b, &eligible, &mut to_b, &mut seen) {
                k += 1;
            }
        }
        k
    };

    // The report one matching would produce, as a value that can be compared
    // against another matching's. `unchecked_pairs` is part of it: a pairing
    // that silently checked fewer vertex counts is a different answer.
    let report_of = |m: &[Option<usize>]| -> (Vec<Region>, usize) {
        let mut regions = Vec::new();
        let mut unchecked = 0usize;
        for (bi, b) in bc.iter().enumerate() {
            match m[bi] {
                Some(i) => {
                    // PAIRED IS NOT THE SAME AS UNCHANGED. The pairing keys on
                    // centroid and area; a contour can hold both and still
                    // have moved, and this instrument cannot say where inside
                    // it. EVIDENCE OF CHANGE, NOT ABSENCE OF EVIDENCE: the
                    // vertex counts decide only when BOTH sides have one. An
                    // earlier version defaulted a missing count to a number
                    // and a surviving mutant proved that justification false --
                    // 0 and usize::MAX behave identically, because the only
                    // cases are both-absent (equal either way) and one-absent
                    // (unequal either way). Absence is not a value.
                    let verts_differ = match (b.vertices, ac[i].vertices) {
                        (Some(x), Some(y)) => x != y,
                        _ => {
                            unchecked += 1;
                            false
                        }
                    };
                    // THE BBOX IS JUDGED AT THE TOLERANCE THAT PAIRED THEM.
                    // rook asked this beside F6 and it is the same
                    // incoherence: these contours were declared the same
                    // because centroid and area agree to within a tolerance,
                    // and the change test then demanded EXACT f64 equality of
                    // bbox_mm. At that strictness any last-bit noise reads as
                    // a reshape, and a reshape below the tolerance is not
                    // something this instrument can see.
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
            if !m.contains(&Some(i)) {
                regions.push(Region {
                    kind: RegionKind::Added,
                    centroid_mm: a.centroid_mm,
                    area_mm2: a.area_mm2,
                    bbox_mm: a.bbox_mm,
                });
            }
        }
        (regions, unchecked)
    };
    let key_of = |r: &(Vec<Region>, usize)| -> String {
        let mut parts: Vec<String> =
            r.0.iter()
                .map(|g| {
                    format!(
                        "{:?}|{:?}|{:?}|{:?}",
                        g.kind, g.centroid_mm, g.area_mm2, g.bbox_mm
                    )
                })
                .collect();
        parts.sort();
        format!("{}#{}", parts.join(";"), r.1)
    };

    // ENUMERATE EVERY MAXIMUM MATCHING, under a budget. The budget is a
    // REFUSAL and not a fallback: past it this cannot show the answer is
    // unique, and an answer it cannot show to be unique is unknown.
    const MATCHING_BUDGET: usize = 20_000;
    let mut visited = 0usize;
    let mut first: Option<((Vec<Region>, usize), String)> = None;
    let mut disagreement: Option<String> = None;
    let mut m: Vec<Option<usize>> = vec![None; bc.len()];
    let mut used = vec![false; ac.len()];

    #[allow(clippy::too_many_arguments)]
    fn walk(
        bi: usize,
        matched: usize,
        remaining: usize,
        max_k: usize,
        eligible: &[Vec<usize>],
        m: &mut Vec<Option<usize>>,
        used: &mut Vec<bool>,
        visited: &mut usize,
        budget: usize,
        on_full: &mut dyn FnMut(&[Option<usize>]),
    ) -> bool {
        if *visited > budget {
            return false;
        }
        // cannot still reach a maximum matching: prune
        if matched + remaining < max_k {
            return true;
        }
        if bi == m.len() {
            // THE PRUNE ABOVE IS THE SINGLE AUTHORITY ON CARDINALITY. A leaf
            // is reachable only with matched == max_k, because at a leaf
            // `remaining` is 0 and the prune fires on anything smaller. This
            // used to re-check it, and the mutant
            // `regions-accepts-a-smaller-matching` SURVIVED -- relaxing the
            // second check changed nothing, which is the proof that it was
            // decorative. Two mutually-redundant guards mean neither is
            // load-bearing and neither can be tested.
            *visited += 1;
            on_full(m);
            return true;
        }
        for &a in &eligible[bi] {
            if used[a] {
                continue;
            }
            used[a] = true;
            m[bi] = Some(a);
            if !walk(
                bi + 1,
                matched + 1,
                remaining - 1,
                max_k,
                eligible,
                m,
                used,
                visited,
                budget,
                on_full,
            ) {
                used[a] = false;
                m[bi] = None;
                return false;
            }
            used[a] = false;
            m[bi] = None;
        }
        // leaving this before-contour unmatched
        walk(
            bi + 1,
            matched,
            remaining - 1,
            max_k,
            eligible,
            m,
            used,
            visited,
            budget,
            on_full,
        )
    }

    let within_budget = {
        let mut on_full = |cand: &[Option<usize>]| {
            if disagreement.is_some() {
                return;
            }
            let rep = report_of(cand);
            let key = key_of(&rep);
            match &first {
                None => first = Some((rep, key)),
                Some((_, k0)) => {
                    if *k0 != key {
                        disagreement = Some(key);
                    }
                }
            }
        };
        walk(
            0,
            0,
            bc.len(),
            max_k,
            &eligible,
            &mut m,
            &mut used,
            &mut visited,
            MATCHING_BUDGET,
            &mut on_full,
        )
    };
    if !within_budget {
        return Err(format!(
            "there are more than {MATCHING_BUDGET} equally-large ways to pair the \
             contours at {:?} {} mm, so this cannot show that the correspondence it \
             would report is the only one. An answer that cannot be shown unique is \
             unknown, never a reading",
            plane.axis, plane.offset_mm
        ));
    }
    if let Some(other) = disagreement {
        let (_, mine) = first
            .as_ref()
            .expect("a disagreement implies a first report");
        return Err(format!(
            "the contours at {:?} {} mm can be paired two equally-large ways that \
             REPORT DIFFERENTLY -- {mine} against {other} -- so which contours \
             correspond is decided by a tie-break and not by the geometry. An \
             ambiguous correspondence is unknown, never a reading",
            plane.axis, plane.offset_mm
        ));
    }
    // report_of already accounts for every after-contour the matching did not
    // use, so there is no second pass here: a loop over `taken` would be a
    // SECOND place that decides what Added means.
    let ((regions, unchecked_pairs), _key) = first.expect("a non-empty section yields one report");
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
            is_hole: false,
        }
    }

    /// The same contour as a HOLE, for the cases where solid-versus-void is
    /// the thing under test.
    fn hole(cx: f64, cy: f64, area: f64, verts: usize) -> Contour {
        Contour {
            is_hole: true,
            ..c(cx, cy, area, verts)
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

    /// RESTORED 2026-11-19. The wire-level version of this was deleted when
    /// those tests were rewritten for the real shape -- where the vertex count
    /// is always known, because it is how many points arrived. The BEHAVIOUR
    /// still matters here, because a fixture or a future source can carry
    /// `None`, and an unknown count compared against anything is unknown.
    ///
    /// Absence on both sides must not make every paired contour look reshaped,
    /// and it must be COUNTED: `unchecked_pairs` is how the reader learns the
    /// comparison was partial.
    #[test]
    fn an_unknown_vertex_count_is_not_a_reshape_and_is_counted_as_unchecked() {
        let mut a = c(0.0, 0.0, 10.0, 4);
        a.vertices = None;
        let mut b = c(0.0, 0.0, 10.0, 4);
        b.vertices = None;
        let d = diff_sections(&cut(3.0, vec![a]), &cut(3.0, vec![b]), PAIR).expect("same plane");
        assert!(d.unchanged(), "absence read as change: {:?}", d.regions);
        assert_eq!(
            d.unchecked_pairs, 1,
            "the partial comparison must be counted"
        );
        assert!(
            !d.fully_checked(),
            "a pair nobody could check is not fully checked"
        );
    }

    /// A hole and a solid contour agreeing EXACTLY on centroid and area are
    /// opposite statements about the material, so they must never pair. This
    /// is the regions-level half of rook's F11 -- the wire half proves the
    /// winding is read; this proves reading it changes the verdict.
    #[test]
    fn a_hole_never_pairs_with_a_solid_contour_of_the_same_size() {
        let d = diff_sections(
            &cut(3.0, vec![c(0.0, 0.0, 10.0, 4)]),
            &cut(3.0, vec![hole(0.0, 0.0, 10.0, 4)]),
            PAIR,
        )
        .expect("same plane");
        assert!(
            !d.unchanged(),
            "a solid and a void of the same size paired: {:?}",
            d.regions
        );
        assert_eq!(
            d.regions
                .iter()
                .filter(|r| matches!(r.kind, RegionKind::Removed))
                .count(),
            1,
            "the solid is gone: {:?}",
            d.regions
        );
    }

    /// rook's F9, thread:wm-review/13, DRIVEN by them before I had it. The
    /// cheapest-first pairing that replaced first-fit was order-independent
    /// and STILL missed a full pairing: the cheapest pair consumes the only
    /// eligible partner of another contour, and the instrument then claims a
    /// contour was removed and another added when a complete correspondence
    /// exists. Greedy maximises the first choice; the NUMBER of
    /// correspondences is what decides Added and Removed.
    #[test]
    fn a_full_pairing_is_found_even_when_the_cheapest_pair_would_block_it() {
        let tol = Comparison::ContourIdentity {
            centroid_tol_mm: 1.0,
            area_tol_mm2: 1.0,
        };
        // rook's figures exactly: the cheapest pair is 0 -> 0.3 at 0.3, which
        // takes 1.2's ONLY partner. The maximum matching is 0 -> -0.5 and
        // 1.2 -> 0.3, both inside the tolerance.
        let before = cut(3.0, vec![c(0.0, 0.0, 10.0, 4), c(1.2, 0.0, 10.0, 4)]);
        let after = cut(3.0, vec![c(0.3, 0.0, 10.0, 4), c(-0.5, 0.0, 10.0, 4)]);
        let d = diff_sections(&before, &after, tol).expect("a full pairing exists");
        assert!(
            !d.regions
                .iter()
                .any(|r| matches!(r.kind, RegionKind::Added | RegionKind::Removed)),
            "greedy claimed structural change where a complete pairing exists: {:?}",
            d.regions
        );
    }

    /// And the cardinality is what is maximised, not the cost: a pairing that
    /// leaves a contour unmatched when it need not be is the F9 defect, so
    /// the count of correspondences is asserted directly.
    #[test]
    fn the_number_of_correspondences_is_maximised_not_the_closeness_of_the_first() {
        let tol = Comparison::ContourIdentity {
            centroid_tol_mm: 1.0,
            area_tol_mm2: 1.0,
        };
        // three before, three after, where the greedy first pick strands two
        let before = cut(
            3.0,
            vec![
                c(0.0, 0.0, 10.0, 4),
                c(1.2, 0.0, 10.0, 4),
                c(2.4, 0.0, 10.0, 4),
            ],
        );
        let after = cut(
            3.0,
            vec![
                c(0.3, 0.0, 10.0, 4),
                c(-0.5, 0.0, 10.0, 4),
                c(2.0, 0.0, 10.0, 4),
            ],
        );
        let d = diff_sections(&before, &after, tol).expect("three correspondences exist");
        assert_eq!(
            d.regions
                .iter()
                .filter(|r| matches!(r.kind, RegionKind::Added | RegionKind::Removed))
                .count(),
            0,
            "a maximum matching pairs all three: {:?}",
            d.regions
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
            err.contains("REPORT DIFFERENTLY") && err.contains("tie-break"),
            "{err}"
        );
    }

    /// rook's F10, thread:wm-review/13: the tie test compared costs with `==`
    /// on f64, and by this project's own rule for reported figures a provider
    /// reporting four decimals cannot separate costs closer than 1e-4 -- so an
    /// exact-equality tie test decided correspondence on digits nobody
    /// vouched for.
    ///
    /// THE ANSWER IS THAT NO COST DECIDES ANYTHING. Cost was removed; the
    /// cardinality is maximised and then every maximum matching's report is
    /// compared. So a near-tie cannot be silently resolved by invisible
    /// digits: it REFUSES, naming that the two pairings report differently.
    /// Refusing is the conservative direction and it is the point -- the
    /// instrument declines to answer rather than answering from a difference
    /// it cannot vouch for.
    #[test]
    fn a_near_tie_refuses_rather_than_being_decided_by_digits_nobody_vouched_for() {
        let tol = Comparison::ContourIdentity {
            centroid_tol_mm: 1.0,
            area_tol_mm2: 1.0,
        };
        let before = cut(3.0, vec![c(0.0, 0.0, 10.0, 4)]);
        // two candidates a hair apart -- far closer than any reported figure
        // could distinguish. One will be paired and one reported Added, and
        // WHICH is decided by nothing the geometry vouches for.
        let after = cut(
            3.0,
            vec![c(0.3, 0.0, 10.0, 4), c(0.3 + 1e-12, 0.0, 10.0, 4)],
        );
        let err =
            diff_sections(&before, &after, tol).expect_err("a near-tie must refuse, not pick");
        assert!(
            err.contains("REPORT DIFFERENTLY"),
            "a near-tie was resolved silently: {err}"
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
