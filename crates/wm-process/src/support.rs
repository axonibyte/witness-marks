//! `proc.support`: the slicer's criterion, never face normals.
//!
//! # The criterion
//!
//! ```text
//! unsupported(z) = slice(z) - offset(slice(z - h), +d)      d = layer_h / tan(θ)
//! ```
//!
//! A layer is supported where the layer below, grown outward by `d`, covers
//! it. `d` is how far one layer may overhang the last and still have
//! something under its inner edge.
//!
//! # θ is measured from the horizontal, and that has to be said
//!
//! Slicers disagree on the convention. 45° is the one angle where both
//! readings agree -- which is exactly why a 45° test case proves nothing about
//! the convention, and why so much overhang advice is quoted without one.
//! At every other angle the two readings give different offsets, so the
//! convention is carried in the TYPE here: there is no function taking a bare
//! `f64` angle, because such a function cannot be called correctly without
//! reading its documentation, and most callers will not.
//!
//! # Face normals are the wrong instrument
//!
//! A face-normal test asks "which way does this triangle point" and concludes
//! that anything pointing down needs support.
//!
//! **A bridge** is where it goes wrong: horizontal, so its normal points
//! straight down -- the most unsupported reading there is -- while a bridge
//! anchored at both ends needs no support at all. MEASURED 2026-11-20 against
//! openscad-mcp 0.6.1's facet-normal analyser: a 22 x 10 mm deck spanning two
//! posts reports 220 mm2 of overhang, which is its entire underside.
//!
//! CORRECTED 2026-11-20, and the correction matters because this paragraph
//! used to credit the wrong mechanism. THE OFFSET CRITERION DOES NOT SEE THE
//! BRIDGE EITHER -- nothing is under mid-span, so `slice(z) - offset(slice(z -
//! h), +d)` reports it unsupported exactly as a face normal does. What
//! distinguishes a bridge is the ANCHOR COUNT (`Region::anchors` below), and
//! `offset_mm` refuses horizontal surfaces outright for precisely this
//! reason. Attributing the bridge case to the offset was a misattribution of
//! this module's own capability.
//!
//! AND A CLAIM THAT WAS SIMPLY FALSE, removed: this said a stack of small
//! steps "has every face vertical or horizontal, so no normal is ever in the
//! overhang band". A solid staircase's treads have UNDERSIDES, and those point
//! straight down. MEASURED on a 20-step stack: 40 of 312 facets have
//! nz < -0.999, and the same analyser's reported area scales monotonically
//! with step size -- 54.8 mm2 at 0.05 mm steps, 88.8 at 0.30, 252.0 at 1.50.
//! It distinguishes severity perfectly well. The claim was wrong and the
//! conclusion drawn from it was wrong.
//!
//! So the honest statement of this module's advantage is narrower than the one
//! that stood here: the offset criterion is the right instrument for SLOPED
//! surfaces, and the anchor count is what no facet-normal method can supply.

use crate::{critical_angle_deg, Missing, Process};

/// An overhang angle, with its convention attached.
///
/// One variant, not two. The point is not to support both conventions -- it
/// is that a bare number cannot be passed, so the convention is stated at
/// every construction site rather than assumed at every call site.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FromHorizontal(pub f64);

impl FromHorizontal {
    /// How far a layer may overhang the one below it: `d = layer_h / tan(θ)`.
    ///
    /// REFUSES A HORIZONTAL SURFACE. At θ = 0 the tangent is zero and `d` is
    /// infinite: a flat overhang may extend forever, which is the arithmetic
    /// being right and the answer being useless. A horizontal surface is a
    /// BRIDGE or a roof, judged by span and anchors, not by this offset.
    ///
    /// REFUSES θ >= 90. A vertical wall is not an overhang, and beyond
    /// vertical the tangent goes negative and `d` comes back negative --
    /// which would SHRINK the layer below and report a vertical wall as
    /// unsupported.
    pub fn offset_mm(self, layer_h: f64) -> Result<f64, String> {
        let t = self.0;
        if !t.is_finite() || t <= 0.0 {
            return Err(format!(
                "theta is {t} degrees from the horizontal; at zero the offset is infinite \
                 -- a flat overhang may extend forever. A horizontal surface is a bridge \
                 or a roof, judged by span and anchors, not by this offset"
            ));
        }
        if t >= 90.0 {
            return Err(format!(
                "theta is {t} degrees from the horizontal, which is vertical or past it. \
                 A vertical wall is not an overhang, and past vertical the tangent is \
                 negative -- the offset would shrink the layer below and report the wall \
                 as unsupported"
            ));
        }
        if !layer_h.is_finite() || layer_h <= 0.0 {
            return Err(format!("layer_h is {layer_h}, which is not a layer height"));
        }
        Ok(layer_h / t.to_radians().tan())
    }
}

/// One region of one layer that the offset criterion found uncovered.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub z_mm: f64,
    /// Uncovered area at this layer, in mm^2.
    pub area_mm2: f64,
    /// The unsupported width to cross, in mm.
    pub span_mm: f64,
    /// How many sides of this region sit on solid material below. A bridge
    /// needs two; one anchor is a cantilever.
    pub anchors: usize,
}

/// What to do about a region.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// Anchored on two or more sides within the bridging span. No support.
    Bridgeable { span_mm: f64 },
    /// Support material, with the reason named -- the two reasons have
    /// different fixes (reorient vs. shorten the span).
    NeedsSupport { why: String, area_mm2: f64 },
}

/// Judge one region.
///
/// REFUSES A REGION WITH NO AREA. A zero-area region is not a region that
/// needs no support; it is a region the slicing found nothing in, and the
/// difference matters because an empty `regions[]` is how "this orientation
/// is clean" and "the layers were never compared" look identical.
pub fn judge(r: &Region, max_bridge_mm: f64) -> Result<Verdict, String> {
    if !r.area_mm2.is_finite() || r.area_mm2 <= 0.0 {
        return Err(format!(
            "a region at z={} with area {}: a zero-area region is not one that needs no \
             support, it is one the layer comparison found nothing in -- and an empty \
             region list is how 'this orientation is clean' and 'the layers were never \
             compared' come out identical",
            r.z_mm, r.area_mm2
        ));
    }
    if !r.span_mm.is_finite() || r.span_mm < 0.0 {
        return Err(format!(
            "a region with span {}, which is not a span",
            r.span_mm
        ));
    }
    if !max_bridge_mm.is_finite() || max_bridge_mm <= 0.0 {
        return Err(format!(
            "max_bridge_mm is {max_bridge_mm}; a non-positive bridging limit makes every \
             region need support, which is not caution, it is a wrong answer that happens \
             to be conservative"
        ));
    }
    // TWO ANCHORS, NOT ONE. One anchor is a cantilever: it has material under
    // one edge and nothing under the far edge, and it droops. Face normals
    // cannot tell the two apart at all -- both are the same horizontal face.
    if r.anchors >= 2 && r.span_mm <= max_bridge_mm {
        return Ok(Verdict::Bridgeable { span_mm: r.span_mm });
    }
    let why = if r.anchors < 2 {
        format!(
            "anchored on {} side(s); a bridge needs two, and one anchor is a cantilever \
             that droops",
            r.anchors
        )
    } else {
        format!(
            "spans {} mm, past the {max_bridge_mm} mm this process bridges",
            r.span_mm
        )
    };
    Ok(Verdict::NeedsSupport {
        why,
        area_mm2: r.area_mm2,
    })
}

/// The whole answer for one orientation.
#[derive(Debug, Clone, PartialEq)]
pub struct Support {
    /// Total area needing support material, mm^2.
    pub area_mm2: f64,
    pub needs: Vec<(Region, Verdict)>,
    pub bridges: Vec<Region>,
    /// The offset the criterion ran at, so the answer can be reproduced.
    pub offset_mm: f64,
    /// The angle it came from, with its convention.
    pub theta: FromHorizontal,
}

/// Run the criterion over a set of regions under a declared process.
///
/// The angle defaults to nothing: it comes from `critical_angle_deg(p)`, which
/// refuses without a declared layer height and extrusion width. "Is 45 degrees
/// fine?" has no answer; "is 45 degrees fine at 0.1 mm on a 0.4 nozzle?" has
/// one, and it is no -- that process holds 26.565 degrees.
pub fn support(p: &Process, regions: &[Region], max_bridge_mm: f64) -> Result<Support, Missing> {
    let theta = FromHorizontal(critical_angle_deg(p)?);
    let layer_h = p.need("layer_h", "support")?.value;
    let offset_mm = theta.offset_mm(layer_h).map_err(|e| Missing {
        wanted: format!("a usable overhang angle: {e}"),
        by: "support",
    })?;
    let mut needs = Vec::new();
    let mut bridges = Vec::new();
    let mut area_mm2 = 0.0;
    for r in regions {
        match judge(r, max_bridge_mm).map_err(|e| Missing {
            wanted: format!("a well-formed region: {e}"),
            by: "support",
        })? {
            Verdict::Bridgeable { .. } => bridges.push(r.clone()),
            v @ Verdict::NeedsSupport { .. } => {
                area_mm2 += r.area_mm2;
                needs.push((r.clone(), v));
            }
        }
    }
    Ok(Support {
        area_mm2,
        needs,
        bridges,
        offset_mm,
        theta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Declared;

    fn proc_with(h: f64, w: f64) -> Process {
        let mut p = Process::new();
        p.layer_h = Some(Declared::measured(h, "the profile"));
        p.extrusion_width = Some(Declared::measured(w, "the profile"));
        p
    }

    fn region(z: f64, area: f64, span: f64, anchors: usize) -> Region {
        Region {
            z_mm: z,
            area_mm2: area,
            span_mm: span,
            anchors,
        }
    }

    /// 45 DEGREES IS THE ONE ANGLE WHERE BOTH CONVENTIONS AGREE, so the
    /// offset there equals the layer height and a 45-degree case proves
    /// nothing about the convention. This test exists to say so.
    #[test]
    fn at_forty_five_degrees_the_offset_is_exactly_one_layer() {
        let d = FromHorizontal(45.0).offset_mm(0.2).expect("valid");
        assert!(
            (d - 0.2).abs() < 1e-12,
            "tan(45) is 1, so d = layer_h: got {d}"
        );
    }

    /// AND AWAY FROM 45 THE CONVENTION BITES. At 0.1 mm on a 0.4 nozzle the
    /// critical angle is 26.565 degrees, and its offset is 0.2 mm -- twice
    /// the layer height, not half, which is what the other convention gives.
    #[test]
    fn away_from_forty_five_the_offset_is_not_the_layer_height() {
        let p = proc_with(0.1, 0.4);
        let theta = critical_angle_deg(&p).expect("declared");
        assert!((theta - 26.565_051_177_077_986).abs() < 1e-9, "got {theta}");
        let d = FromHorizontal(theta).offset_mm(0.1).expect("valid");
        assert!(
            (d - 0.2).abs() < 1e-9,
            "0.1 / tan(26.565) = 0.2, twice the layer height: got {d}"
        );
    }

    /// A HORIZONTAL SURFACE IS REFUSED, because d is infinite there: the
    /// arithmetic is right and the answer is useless. A flat face is a bridge
    /// or a roof, judged by span and anchors.
    #[test]
    fn a_horizontal_surface_is_refused_rather_than_given_an_infinite_offset() {
        let e = FromHorizontal(0.0).offset_mm(0.2).expect_err("must refuse");
        assert!(e.contains("may extend forever"), "{e}");
        assert!(e.contains("bridge"), "{e}");
    }

    /// AND PAST VERTICAL THE OFFSET GOES NEGATIVE, which would shrink the
    /// layer below and report a vertical wall as unsupported.
    #[test]
    fn an_angle_at_or_past_vertical_is_refused() {
        assert!(FromHorizontal(90.0).offset_mm(0.2).is_err());
        let e = FromHorizontal(100.0)
            .offset_mm(0.2)
            .expect_err("must refuse");
        assert!(e.contains("negative"), "{e}");
    }

    /// A BRIDGE IS NOT AN OVERHANG. Its face points straight down -- the most
    /// unsupported normal there is -- and anchored at both ends it needs no
    /// support. This is the first class face normals get wrong.
    #[test]
    fn a_bridge_anchored_at_both_ends_needs_no_support() {
        let r = region(12.0, 58.24, 8.0, 2);
        assert_eq!(
            judge(&r, 10.0).expect("valid"),
            Verdict::Bridgeable { span_mm: 8.0 }
        );
    }

    /// ONE ANCHOR IS A CANTILEVER, not a bridge. Identical face, identical
    /// normal, opposite answer -- which is why the criterion counts anchors
    /// and a normal test cannot.
    #[test]
    fn the_same_face_with_one_anchor_is_a_cantilever_and_needs_support() {
        let r = region(12.0, 58.24, 8.0, 1);
        match judge(&r, 10.0).expect("valid") {
            Verdict::NeedsSupport { why, area_mm2 } => {
                assert!(why.contains("cantilever"), "{why}");
                assert_eq!(area_mm2, 58.24);
            }
            other => panic!("one anchor droops; got {other:?}"),
        }
    }

    /// A span past what the process bridges needs support for a different
    /// reason, and the reason is named because the two have different fixes:
    /// reorient, or shorten the span.
    #[test]
    fn a_span_past_the_bridging_limit_names_the_span_not_the_anchors() {
        let r = region(12.0, 58.24, 24.0, 2);
        match judge(&r, 10.0).expect("valid") {
            Verdict::NeedsSupport { why, .. } => {
                assert!(why.contains("past the 10 mm"), "{why}");
                assert!(!why.contains("cantilever"), "the anchors are fine: {why}");
            }
            other => panic!("24 mm is past 10; got {other:?}"),
        }
    }

    /// A ZERO-AREA REGION IS REFUSED. An empty region list is how "this
    /// orientation is clean" and "the layers were never compared" come out
    /// identical, and a zero-area entry is that collapse one region at a time.
    #[test]
    fn a_zero_area_region_is_refused_rather_than_passed() {
        let e = judge(&region(12.0, 0.0, 8.0, 2), 10.0).expect_err("must refuse");
        assert!(e.contains("never compared"), "{e}");
    }

    /// A NON-POSITIVE BRIDGING LIMIT IS REFUSED. It makes every region need
    /// support, which is not caution -- it is a wrong answer that happens to
    /// be conservative, and so it never gets investigated.
    #[test]
    fn a_non_positive_bridging_limit_is_refused() {
        let e = judge(&region(12.0, 58.24, 0.0, 2), 0.0).expect_err("must refuse");
        assert!(e.contains("happens to be conservative"), "{e}");
    }

    /// The whole run carries the offset and the angle, so the answer can be
    /// reproduced without guessing which convention produced it.
    #[test]
    fn the_answer_carries_the_offset_and_the_angle_it_ran_at() {
        let p = proc_with(0.2, 0.4);
        let s = support(&p, &[region(12.0, 58.24, 8.0, 2)], 10.0).expect("declared");
        assert!((s.theta.0 - 45.0).abs() < 1e-12);
        assert!((s.offset_mm - 0.2).abs() < 1e-12);
        assert_eq!(s.area_mm2, 0.0, "the one region bridges");
        assert_eq!(s.bridges.len(), 1);
        assert!(s.needs.is_empty());
    }

    /// Only the regions that need support are summed. A bridge contributes no
    /// support area, and counting it would make every bridged part look
    /// expensive to print.
    #[test]
    fn the_support_area_sums_only_what_needs_support() {
        let p = proc_with(0.2, 0.4);
        let s = support(
            &p,
            &[
                region(12.0, 58.24, 8.0, 2), // bridges
                region(14.0, 10.00, 8.0, 1), // cantilever
                region(16.0, 5.00, 24.0, 2), // too long
            ],
            10.0,
        )
        .expect("declared");
        assert_eq!(s.area_mm2, 15.0, "10 + 5, not 73.24");
        assert_eq!(s.needs.len(), 2);
        assert_eq!(s.bridges.len(), 1);
    }

    /// REFUSES WITHOUT A DECLARED PROCESS, like every other Layer 4 call.
    /// "Does this need support?" has no answer; "at 0.1 mm on a 0.4 nozzle"
    /// has one.
    #[test]
    fn support_refuses_without_a_declared_process() {
        let p = Process::new();
        let e = support(&p, &[region(12.0, 58.24, 8.0, 2)], 10.0).expect_err("must refuse");
        assert_eq!(e.by, "critical_angle");
    }
}
