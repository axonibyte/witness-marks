//! Reading `measure(mode=printability)` -- and the reason it is not the
//! criterion.
//!
//! # The provider measures a different quantity
//!
//! Issue #23 says it: *"The provider's printability is facet-normal based;
//! this is the thing it does not do."* Measured on the four shapes
//! `overhang-scan.scad` was hand-validated against:
//!
//! | shape | the scan | the provider |
//! | --- | --- | --- |
//! | flat 20x10 cantilever off a post | 198.5 (~197 by hand) | 200.0 |
//! | 45 deg cone | 0.0 | 0.0, bracket [0, 444.18] |
//! | 60 deg cone | 0.0 | 0.0 |
//! | 30 deg cone | 131.1 (132.8 by hand) | 362.64 |
//!
//! The 30 degree cone is 2.77x apart, and the arithmetic says why. A cone of
//! R = 10 at 30 degrees has slant length `R / cos(30) = 11.547` and lateral
//! area `pi * R * slant = 362.76`. The provider reports **the area of the
//! down-facing surface**; the slicer criterion reports **the area with
//! nothing under it**. For a flat slab those nearly coincide -- 200 against
//! 198.5, the difference being the rim the offset `d` eats. For a cone they
//! do not, and nothing about the two numbers says which question was asked.
//!
//! So this module does NOT feed the provider's area into
//! [`crate::support::Region::area_mm2`] as though it were the criterion's.
//! It reads what the provider actually measured, labels it, and keeps the two
//! answers side by side as a differential vector.
//!
//! # The bracket is the measurement
//!
//! The 45 degree cone's point estimate is 0.0 and its bracket runs to
//! **444.18**. At exactly the critical angle a facet-normal method cannot
//! decide: half the tessellation falls either side, and the provider says so
//! rather than picking. Reading the 0.0 and discarding the bracket would
//! report "no overhang" on the one shape the instrument cannot judge -- so a
//! bracket that straddles the point estimate is a REFUSAL here, not a
//! reading.

use serde_json::Value;

/// What the provider measured, with the question it answers named.
#[derive(Debug, Clone, PartialEq)]
pub struct DownFacing {
    /// Area of DOWN-FACING SURFACE, mm^2. Not the area with nothing under it.
    pub area_mm2: f64,
    /// The provider's own uncertainty, from the mesh's facet step.
    pub bracket: [f64; 2],
    pub patch_count: u64,
    /// Islands: regions appearing in a layer with nothing below them at all.
    /// The third of bridge/droop/island, which the criterion alone does not
    /// separate.
    pub island_count: u64,
    pub layer_height_mm: f64,
}

/// How far apart the two instruments are on one shape.
#[derive(Debug, Clone, PartialEq)]
pub enum Agreement {
    /// Both report nothing, and the provider is certain of it.
    BothClear,
    /// Within `ratio` of each other.
    Close { ratio: f64 },
    /// Materially apart. Both may be right about different questions.
    Apart { ratio: f64 },
    /// The provider cannot decide this shape -- its bracket straddles its own
    /// point estimate by more than `span_mm2`.
    ProviderUndecided { span_mm2: f64 },
}

/// Read a printability answer.
///
/// REFUSES A MISSING OVERHANG BLOCK. An answer with no overhang section has
/// not reported "no overhang"; it has reported nothing, and those must not
/// serialize the same.
pub fn down_facing(payload: &Value) -> Result<DownFacing, String> {
    if payload.get("success").and_then(Value::as_bool) == Some(false) {
        return Err("the provider reported success=false on printability".into());
    }
    let ov = payload.get("overhang").ok_or_else(|| {
        "the answer carries no `overhang` block. That is not a report of no overhang; \
             it is no report, and the two must not read alike"
            .to_string()
    })?;
    let area = ov
        .get("area_mm2")
        .and_then(Value::as_f64)
        .ok_or_else(|| "the overhang block carries no area_mm2".to_string())?;
    let bracket = ov
        .get("bracket")
        .and_then(Value::as_array)
        .and_then(|a| {
            let v: Vec<f64> = a.iter().filter_map(Value::as_f64).collect();
            (v.len() == 2).then(|| [v[0], v[1]])
        })
        .ok_or_else(|| {
            "the overhang block carries no usable `bracket`. The bracket is the provider's \
             own uncertainty from facet size, and a reading without it cannot be told apart \
             from a certain one"
                .to_string()
        })?;
    if !area.is_finite() || area < 0.0 {
        return Err(format!("the overhang area is {area}, which is not an area"));
    }
    if bracket[0] > bracket[1] {
        return Err(format!(
            "the bracket is [{}, {}], which is inverted",
            bracket[0], bracket[1]
        ));
    }
    let islands = payload.get("islands");
    Ok(DownFacing {
        area_mm2: area,
        bracket,
        patch_count: ov.get("patch_count").and_then(Value::as_u64).unwrap_or(0),
        island_count: islands
            .and_then(|i| i.get("count"))
            .and_then(Value::as_u64)
            .unwrap_or(0),
        layer_height_mm: islands
            .and_then(|i| i.get("layer_height_mm"))
            .and_then(Value::as_f64)
            .unwrap_or(f64::NAN),
    })
}

/// Compare the provider's down-facing area against the slicer criterion's
/// unsupported area for the same shape.
///
/// `undecided_mm2` is how wide a bracket the caller will still read a point
/// estimate from. The 45 degree cone's bracket is 444 mm^2 wide around a point
/// estimate of zero; at any sane limit that is undecided, and saying so beats
/// reporting the zero.
pub fn agreement(
    d: &DownFacing,
    criterion_mm2: f64,
    undecided_mm2: f64,
) -> Result<Agreement, String> {
    if !criterion_mm2.is_finite() || criterion_mm2 < 0.0 {
        return Err(format!(
            "the criterion's area is {criterion_mm2}, which is not an area"
        ));
    }
    if !undecided_mm2.is_finite() || undecided_mm2 <= 0.0 {
        return Err(format!(
            "undecided_mm2 is {undecided_mm2}; a non-positive limit makes every bracket \
             acceptable, so no shape could ever be reported undecided"
        ));
    }
    let span = d.bracket[1] - d.bracket[0];
    if span > undecided_mm2 {
        return Ok(Agreement::ProviderUndecided { span_mm2: span });
    }
    if d.area_mm2 == 0.0 && criterion_mm2 == 0.0 {
        return Ok(Agreement::BothClear);
    }
    // A RATIO NEEDS A DENOMINATOR THAT EXISTS. One side at zero and the other
    // not is maximal disagreement, not an infinite ratio to be formatted.
    if d.area_mm2 == 0.0 || criterion_mm2 == 0.0 {
        return Ok(Agreement::Apart {
            ratio: f64::INFINITY,
        });
    }
    let ratio = if d.area_mm2 > criterion_mm2 {
        d.area_mm2 / criterion_mm2
    } else {
        criterion_mm2 / d.area_mm2
    };
    if ratio <= 1.1 {
        Ok(Agreement::Close { ratio })
    } else {
        Ok(Agreement::Apart { ratio })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fx(case: &str) -> Value {
        let all: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/provider-printability.json"
        ))
        .expect("the captured fixture parses");
        all[case]["answer"].clone()
    }

    /// THE ACCEPTANCE TEST issue #23 names, run at last. The four shapes
    /// `overhang-scan.scad` was hand-validated against, measured through the
    /// provider, against the scan's own recorded answers.
    #[test]
    fn the_four_hand_computed_shapes_against_the_provider() {
        // (case, the scan's measured area, the hand figure)
        let rows: [(&str, f64, f64); 4] = [
            ("flat_cantilever", 198.5, 197.0),
            ("cone_45", 0.0, 0.0),
            ("cone_60", 0.0, 0.0),
            ("cone_30", 131.1, 132.8),
        ];
        for (case, scan, hand) in rows {
            let d = down_facing(&fx(case)).expect("parses");
            let a = agreement(&d, scan, 10.0).expect("valid");
            match case {
                // 200.0 against 198.5: the rim the offset eats.
                "flat_cantilever" => {
                    assert!(matches!(a, Agreement::Close { .. }), "{case}: {a:?}")
                }
                // THE PROVIDER CANNOT DECIDE THIS ONE. Point estimate 0.0,
                // bracket to 444.18, because at exactly the critical angle
                // half the tessellation falls either side.
                "cone_45" => match a {
                    Agreement::ProviderUndecided { span_mm2 } => {
                        assert!(span_mm2 > 400.0, "{case}: {span_mm2}")
                    }
                    other => panic!("{case}: expected undecided, got {other:?}"),
                },
                "cone_60" => assert_eq!(a, Agreement::BothClear, "{case}"),
                // 362.64 against 131.1 -- 2.77x, because the provider measures
                // the cone's down-facing SURFACE and the criterion measures
                // the area with nothing under it.
                "cone_30" => match a {
                    Agreement::Apart { ratio } => {
                        assert!((ratio - 2.766).abs() < 0.01, "{case}: {ratio}")
                    }
                    other => panic!("{case}: expected Apart, got {other:?}"),
                },
                _ => unreachable!(),
            }
            assert!(
                (scan - hand).abs() < 2.0,
                "{case}: the scan and the hand figure should agree closely"
            );
        }
    }

    /// The 30 degree cone's figure is the cone's lateral area, which is what
    /// says the provider answered a different question rather than answering
    /// the same one badly.
    #[test]
    fn the_thirty_degree_cone_matches_its_own_lateral_area() {
        let d = down_facing(&fx("cone_30")).expect("parses");
        let r: f64 = 10.0;
        let lateral = std::f64::consts::PI * r * (r / 30f64.to_radians().cos());
        assert!(
            (d.area_mm2 - lateral).abs() / lateral < 0.001,
            "provider {} against lateral {lateral}",
            d.area_mm2
        );
    }

    /// A BRACKET THAT STRADDLES ITS OWN POINT ESTIMATE IS NOT A READING.
    /// Taking the 0.0 and discarding the bracket reports "no overhang" on the
    /// one shape the instrument cannot judge.
    #[test]
    fn a_wide_bracket_is_undecided_rather_than_its_point_estimate() {
        let d = down_facing(&fx("cone_45")).expect("parses");
        assert_eq!(d.area_mm2, 0.0, "the point estimate really is zero");
        assert!(d.bracket[1] > 400.0, "and the bracket really is wide");
        assert!(matches!(
            agreement(&d, 0.0, 10.0).expect("valid"),
            Agreement::ProviderUndecided { .. }
        ));
    }

    /// And a tight bracket at zero is BothClear, so the refusal is about the
    /// width and not about the shape.
    #[test]
    fn the_sixty_degree_cone_has_a_tight_bracket_and_reads_clear() {
        let d = down_facing(&fx("cone_60")).expect("parses");
        assert_eq!(d.bracket, [0.0, 0.0]);
        assert_eq!(
            agreement(&d, 0.0, 10.0).expect("valid"),
            Agreement::BothClear
        );
    }

    /// ISLANDS ARE READ, which is the third of bridge/droop/island the
    /// criterion alone does not separate.
    #[test]
    fn islands_and_the_layer_height_are_read_from_the_answer() {
        let d = down_facing(&fx("flat_cantilever")).expect("parses");
        assert_eq!(d.island_count, 0);
        assert_eq!(d.layer_height_mm, 0.2);
        assert_eq!(d.patch_count, 1);
    }

    /// AN ANSWER WITH NO OVERHANG BLOCK is not a report of no overhang.
    #[test]
    fn a_missing_overhang_block_is_refused() {
        let e = down_facing(&serde_json::json!({"success": true})).expect_err("must refuse");
        assert!(e.contains("it is no report"), "{e}");
    }

    /// A READING WITHOUT ITS BRACKET cannot be told from a certain one.
    #[test]
    fn an_overhang_without_a_bracket_is_refused() {
        let e = down_facing(&serde_json::json!({
            "success": true, "overhang": {"area_mm2": 1.0}
        }))
        .expect_err("must refuse");
        assert!(e.contains("cannot be told apart from a certain one"), "{e}");
    }

    /// A NON-POSITIVE UNDECIDED LIMIT makes every bracket acceptable, so no
    /// shape could ever be reported undecided.
    #[test]
    fn a_non_positive_undecided_limit_is_refused() {
        let d = down_facing(&fx("cone_45")).expect("parses");
        let e = agreement(&d, 0.0, 0.0).expect_err("must refuse");
        assert!(e.contains("could ever be reported undecided"), "{e}");
    }

    /// One side zero and the other not is maximal disagreement, not an
    /// infinite ratio that formats as a number.
    #[test]
    fn one_side_at_zero_is_apart_and_not_a_ratio_to_be_read() {
        let d = down_facing(&fx("cone_60")).expect("parses");
        match agreement(&d, 50.0, 10.0).expect("valid") {
            Agreement::Apart { ratio } => assert!(ratio.is_infinite()),
            other => panic!("expected Apart, got {other:?}"),
        }
    }

    #[test]
    fn an_inverted_bracket_is_refused() {
        assert!(down_facing(&serde_json::json!({
            "success": true, "overhang": {"area_mm2": 1.0, "bracket": [5.0, 1.0]}
        }))
        .is_err());
    }
}
