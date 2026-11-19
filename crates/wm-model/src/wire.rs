//! Reading the provider's answers into a [`crate::diff::Snapshot`].
//!
//! # The provider publishes no hash
//!
//! `measure(mode=model)` returns volume, surface area, bbox, triangle and
//! vertex counts, manifoldness -- and no content hash of any kind. That is a
//! problem, because [`crate::diff::diff`] REFUSES a snapshot without one: two
//! un-hashed snapshots compare equal and would report themselves
//! bit-identical, which is a PASS under `Intent::Refactor` and the exact
//! failure that module exists to catch.
//!
//! The identity therefore comes from the bytes. `export_model` writes a real
//! mesh file and names its path; hashing those bytes is a genuine content
//! hash, so `Snapshot`'s contract -- equal hashes mean identical output --
//! holds as written rather than being quietly weakened into "these agree on
//! the properties we happened to measure".
//!
//! A FINGERPRINT OVER MEASURED PROPERTIES WAS THE TEMPTING SHORTCUT and it is
//! a different claim: volume, area, bbox and counts can all agree between two
//! meshes that are not the same mesh. Writing that into a field documented as
//! a content hash would make the documentation false at the point where
//! someone relies on it.
//!
//! # The format is part of the identity
//!
//! The same geometry exported as ASCII and as binary STL is different bytes,
//! so the format name is hashed with them. Two snapshots taken at different
//! formats then differ, which reports a change where there is none in the
//! geometry -- the safe direction, since under `Intent::Refactor` that FAILS
//! loudly instead of hiding a real difference.

use crate::diff::Snapshot;
use crate::regions::{Contour, Plane, Section};
use serde_json::Value;
use std::collections::BTreeMap;

/// Hash the mesh `export_model` wrote.
///
/// REFUSES RATHER THAN RETURNING AN EMPTY HASH, at every step. An unreadable
/// export is not "no change": an empty-string hash equals another empty-string
/// hash, and the pair would report itself bit-identical through the one tool
/// built to catch exactly that.
pub fn digest_of_export(export: &Value) -> Result<String, String> {
    if export.get("success").and_then(Value::as_bool) == Some(false) {
        return Err(format!(
            "the export reported success=false: {}",
            export
                .get("errors")
                .map(ToString::to_string)
                .unwrap_or_else(|| "no errors named".into())
        ));
    }
    let path = export
        .get("output_path")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "the export names no output_path, so there are no bytes to identify it by. \
             Refusing rather than returning an empty hash, which would compare equal to \
             another empty hash and read as bit-identical"
                .to_string()
        })?;
    let format = export
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let bytes = std::fs::read(path)
        .map_err(|e| format!("the exported mesh at {path} could not be read: {e}"))?;
    if bytes.is_empty() {
        return Err(format!(
            "the exported mesh at {path} is EMPTY. A zero-byte export is not an empty \
             model -- the kernel writes no file at all for that -- it is a failed export"
        ));
    }
    Ok(digest_of_bytes(format, &bytes))
}

/// The pure half: the format name and the bytes make the identity.
///
/// Split out from the file read so the identity rule is drivable without a
/// provider, which is where the rule can actually be got wrong.
pub fn digest_of_bytes(format: &str, bytes: &[u8]) -> String {
    // A SMALL, EXPLICIT DIGEST rather than a dependency. This is identity, not
    // security: the requirement is that different bytes give different values
    // often enough to be useful and that identical bytes always agree.
    // FNV-1a over the format, a separator that cannot occur in a format name,
    // and the bytes.
    let mut h: u128 = 0x6c62272e07bb0142_62b821756295c58d;
    let prime: u128 = 0x0000000001000000_000000000000013B;
    let mut feed = |b: u8| {
        h ^= b as u128;
        h = h.wrapping_mul(prime);
    };
    for b in bytes {
        feed(*b);
    }
    // THE FORMAT IS THE PREFIX, and only the prefix. An earlier version also
    // fed it into the hash; a mutant that removed that feed SURVIVED, because
    // the prefix already separates the two and nothing could tell the
    // difference. Redundant work that no test can distinguish is work that
    // will drift without anyone noticing, so it is gone rather than wrapped
    // in a test asserting it happens.
    format!("{format}:{h:032x}")
}

/// Build a snapshot from one `measure(mode=model)` answer and a digest.
///
/// The digest is an argument rather than something this function fetches, so
/// that the caller cannot accidentally build a snapshot with no identity: the
/// only way to get one is through [`digest_of_export`], which refuses instead
/// of returning an empty string.
pub fn snapshot_from(
    measure: &Value,
    digest: String,
    params: BTreeMap<String, String>,
) -> Result<Snapshot, String> {
    if measure.get("success").and_then(Value::as_bool) == Some(false) {
        return Err("the measurement reported success=false".into());
    }
    let volume = measure
        .get("volume")
        .and_then(Value::as_f64)
        .ok_or_else(|| {
            "the measurement carries no `volume`. Defaulting it to zero would make an \
             unmeasured model and an empty one the same snapshot"
                .to_string()
        })?;
    if digest.trim().is_empty() {
        return Err(
            "an empty digest was handed in; refusing to build a snapshot that \
                    compares equal to every other un-identified one"
                .into(),
        );
    }
    Ok(Snapshot {
        mesh_hash: digest,
        volume_mm3: volume,
        params,
    })
}

/// Build a [`Section`] from one `measure(mode=section)` answer.
///
/// The plane is an ARGUMENT rather than something read out of the answer, and
/// that is deliberate: the provider is asked to cut at
/// `section_axis=section_offset`, so the caller is the one who knows which cut
/// this is. Reading it back out of the reply would mean trusting the reply to
/// say which question it answered -- the exact hazard issue 51 names, and the
/// one issue 23 measured when printability answered a different question than
/// the number suggested.
///
/// UNRESOLVED IS NOT AN EMPTY CUT. The provider's house rule, already held in
/// [`wm_asm::wire`] for sweeps: a row it could not resolve comes back as
/// UNRESOLVED, and an unanswered cut must not read as "no contours at this
/// plane". An empty `contours` array IS a real answer -- the plane missed the
/// solid -- so the two cannot be allowed to arrive the same way.
pub fn section_from(measure: &Value, plane: Plane) -> Result<Section, String> {
    if measure.get("success").and_then(Value::as_bool) == Some(false) {
        return Ok(Section::Unresolved {
            plane,
            why: measure
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("success=false with no error given")
                .to_string(),
        });
    }
    if let Some(state) = measure.get("state").and_then(Value::as_str) {
        if state.eq_ignore_ascii_case("UNRESOLVED") {
            return Ok(Section::Unresolved {
                plane,
                why: "the provider reported UNRESOLVED for this cut".into(),
            });
        }
    }
    // AN EMPTY CUT SAYS SO, and it is not the same as an unanswered one.
    if measure.get("empty_section").and_then(Value::as_bool) == Some(true) {
        return Ok(Section::Cut {
            plane,
            contours: Vec::new(),
        });
    }
    let rows = measure
        .get("contours")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "the section answer carries no `contours` array. Treating its absence as an \
             empty cut would make an unanswered question and a plane that missed the \
             solid the same answer"
                .to_string()
        })?;

    // THE WIRE SHAPE IS THE PROVIDER'S, NOT THE ONE I FOUND CONVENIENT.
    // rook's F11 (thread:wm-review/15): this function used to want per-contour
    // OBJECTS carrying centroid, area, bbox and vertices. The pinned provider
    // returns no such thing, so this had NEVER PARSED A REAL ANSWER -- every
    // fixture was hand-built by me, nothing outside the tests called it, and
    // the figures I wrote down were never checked against the instrument.
    // That is this project's founding defect, committed by me, in the code
    // meant to catch it.
    //
    // MEASURED 2026-11-19 against openscad-mcp 0.6.1, two cubes cut at z=0:
    //
    //   "area":200.0, "centroid":[15,5], "bbox_min":[0,0], "bbox_max":[30,10],
    //   "polygon_count":2, "hole_count":0,
    //   "contours":[[[10,10],[0,10],[0,0],[10,0]],
    //               [[30,10],[20,10],[20,0],[30,0]]]
    //
    // So `contours` is a list of POINT LISTS, and area, centroid and bbox
    // appear ONCE, aggregated over the whole section. Everything per-contour
    // is computed here from the points, which is sound because the points ARE
    // the measurement -- and the vertex count, which the old shape treated as
    // frequently absent, is simply how many there are.
    let mut contours = Vec::with_capacity(rows.len());
    let mut signed_total = 0.0f64;
    for (n, row) in rows.iter().enumerate() {
        let pts = row.as_array().ok_or_else(|| {
            format!(
                "contour {n} is not an array of points; this provider reports each \
                     contour as a point list"
            )
        })?;
        // A TRUNCATED CONTOUR IS UNKNOWN, NEVER A SHORTER ONE. MEASURED: a
        // 100-point contour comes back as 61 entries -- 60 points then
        // ["...", "40 more"], a pair of STRINGS. Computing a centroid or an
        // area over 60 of 100 points would produce a figure nobody measured,
        // and it would look exactly like a figure somebody did.
        let mut xy = Vec::with_capacity(pts.len());
        for (i, pt) in pts.iter().enumerate() {
            let pair = pt
                .as_array()
                .ok_or_else(|| format!("contour {n}'s entry {i} is not a point: {pt}"))?;
            if pair.len() != 2 {
                return Err(format!(
                    "contour {n}'s entry {i} has {} components; a point in the cut \
                     plane has exactly two",
                    pair.len()
                ));
            }
            match (pair[0].as_f64(), pair[1].as_f64()) {
                (Some(x), Some(y)) => xy.push([x, y]),
                _ => {
                    return Err(format!(
                        "contour {n} is TRUNCATED at entry {i} ({}, {}): the provider \
                         cut the point list and said so. A centroid or area over the \
                         points it did send is a figure nobody measured",
                        pair[0], pair[1]
                    ))
                }
            }
        }
        if xy.len() < 3 {
            return Err(format!(
                "contour {n} carries {} point(s); a closed contour with an area needs \
                 at least three",
                xy.len()
            ));
        }

        // THE SIGNED SHOELACE AREA CARRIES BOTH FIGURES AT ONCE: its magnitude
        // is the area, and its SIGN is the provider's own statement of
        // hole-ness (see Contour::is_hole for the ring measurement).
        let mut signed = 0.0f64;
        let mut cx = 0.0f64;
        let mut cy = 0.0f64;
        for i in 0..xy.len() {
            let [x1, y1] = xy[i];
            let [x2, y2] = xy[(i + 1) % xy.len()];
            let cross = x1 * y2 - x2 * y1;
            signed += cross;
            cx += (x1 + x2) * cross;
            cy += (y1 + y2) * cross;
        }
        signed /= 2.0;
        if signed == 0.0 {
            return Err(format!(
                "contour {n} encloses zero area, so it has no centroid; a degenerate \
                 contour must not be compared as though it had a position"
            ));
        }
        signed_total += signed;
        let centroid = [cx / (6.0 * signed), cy / (6.0 * signed)];
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for [x, y] in &xy {
            lo[0] = lo[0].min(*x);
            lo[1] = lo[1].min(*y);
            hi[0] = hi[0].max(*x);
            hi[1] = hi[1].max(*y);
        }
        contours.push(Contour {
            centroid_mm: centroid,
            area_mm2: signed.abs(),
            bbox_mm: [lo[0], lo[1], hi[0], hi[1]],
            vertices: Some(xy.len()),
            is_hole: signed < 0.0,
        });
    }

    // AND THE PARSE IS CHECKED AGAINST THE PROVIDER'S OWN AGGREGATE. If the
    // signed areas do not sum to the `area` it reported for the section, this
    // has misread the reply -- holes read as solid, a contour dropped, points
    // in the wrong order -- and a diff built on a misread is worse than none.
    // MEASURED on the $fn=8 ring: +282.844 and -70.710 sum to 212.134 against
    // a reported 212.1322, a relative difference of 8.5e-6, which is the 4-dp
    // rounding of the points the provider sent. The bound below is 1e-3
    // relative, loose enough for that accumulation over many vertices and
    // tight enough that reading a hole as solid (353.6 against 212.1, 67%)
    // cannot pass.
    if let Some(reported) = measure.get("area").and_then(Value::as_f64) {
        let scale = reported.abs().max(1.0);
        if (signed_total - reported).abs() / scale > 1e-3 {
            return Err(format!(
                "the contours sum to {signed_total} but the provider reports the \
                 section's area as {reported}; this has MISREAD the reply rather than \
                 measured a disagreement, and a diff built on a misread is worse than \
                 no diff"
            ));
        }
    }
    Ok(Section::Cut { plane, contours })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::regions::Axis;

    fn zplane() -> Plane {
        Plane {
            axis: Axis::Z,
            offset_mm: 3.0,
        }
    }

    /// A contour as THE PINNED PROVIDER SENDS ONE: a list of points, wound
    /// counter-clockwise for solid material. A unit square at (cx, cy) with
    /// the given half-extent, so its area is (2*h)^2 and its centroid is
    /// exactly (cx, cy).
    fn square(cx: f64, cy: f64, h: f64) -> Value {
        serde_json::json!([
            [cx - h, cy - h],
            [cx + h, cy - h],
            [cx + h, cy + h],
            [cx - h, cy + h]
        ])
    }

    /// The same square wound the other way, which is how this provider says
    /// "hole" -- measured on a $fn=8 ring, outer +282.844, inner -70.710.
    fn square_hole(cx: f64, cy: f64, h: f64) -> Value {
        serde_json::json!([
            [cx - h, cy - h],
            [cx - h, cy + h],
            [cx + h, cy + h],
            [cx + h, cy - h]
        ])
    }

    /// THE REAL REPLY, captured from openscad-mcp 0.6.1 on 2026-11-19 for
    /// `cube([10,10,10]); translate([20,0,0]) cube([10,10,10]);` cut at z=0.
    /// Kept verbatim because rook's F11 was that every fixture here was a
    /// shape I invented and the parser had never seen a real one.
    const REAL_TWO_CUBES: &str = r#"{"success":true,"mode":"section","units":"mm",
        "plane":{"axis":"z","offset":0.0},"in_plane_axes":{"x":"x","y":"y"},
        "area":200.0,"perimeter":80.0,"bbox_min":[0.0,0.0],"bbox_max":[30.0,10.0],
        "dimensions":[30.0,10.0],"centroid":[15.0,5.0],"polygon_count":2,
        "hole_count":0,
        "contours":[[[10.0,10.0],[0.0,10.0],[0.0,0.0],[10.0,0.0]],
                    [[30.0,10.0],[20.0,10.0],[20.0,0.0],[30.0,0.0]]],
        "errors":[],"warnings":[],"deprecated":[],"echo_output":[]}"#;

    #[test]
    fn a_real_section_reply_parses_into_the_contours_it_describes() {
        let reply: Value = serde_json::from_str(REAL_TWO_CUBES).expect("captured reply");
        let got = section_from(&reply, zplane()).expect("a real answer parses");
        match got {
            Section::Cut { contours, .. } => {
                assert_eq!(contours.len(), 2, "two cubes, two contours");
                for c in &contours {
                    assert_eq!(c.area_mm2, 100.0, "each cut face is 10x10");
                    assert_eq!(c.vertices, Some(4), "four points were sent");
                    assert!(!c.is_hole, "neither is a hole: hole_count is 0");
                }
                // the provider's own aggregate is 200.0, and the parse agrees
                let total: f64 = contours.iter().map(|c| c.area_mm2).sum();
                assert_eq!(total, 200.0);
            }
            other => panic!("expected a cut, got {other:?}"),
        }
    }

    #[test]
    fn a_section_answer_becomes_contours_in_the_plane_the_caller_named() {
        let got = section_from(
            &serde_json::json!({"contours": [square(0.0, 0.0, 1.0)]}),
            zplane(),
        )
        .expect("a well-formed section");
        match got {
            Section::Cut { plane, contours } => {
                assert_eq!(plane.axis, Axis::Z);
                assert_eq!(plane.offset_mm, 3.0);
                assert_eq!(contours.len(), 1);
                assert_eq!(contours[0].area_mm2, 4.0);
                assert_eq!(contours[0].centroid_mm, [0.0, 0.0]);
                assert_eq!(contours[0].bbox_mm, [-1.0, -1.0, 1.0, 1.0]);
            }
            other => panic!("expected a cut, got {other:?}"),
        }
    }

    /// rook's F11, the half that would have caught it: a TRUNCATED point list
    /// is unknown, never a shorter contour. MEASURED: a 100-point contour
    /// comes back as 60 points followed by ["...", "40 more"], a pair of
    /// STRINGS, and a centroid over the 60 would be a figure nobody measured.
    #[test]
    fn a_truncated_point_list_is_refused_rather_than_averaged() {
        let mut pts: Vec<Value> = (0..60)
            .map(|i| {
                let a = (i as f64) * std::f64::consts::TAU / 100.0;
                serde_json::json!([a.cos(), a.sin()])
            })
            .collect();
        pts.push(serde_json::json!(["...", "40 more"]));
        let err = section_from(&serde_json::json!({"contours": [pts]}), zplane())
            .expect_err("a truncated contour must refuse");
        assert!(err.contains("TRUNCATED"), "{err}");
    }

    /// Hole-ness comes from the winding, because the reply says `hole_count`
    /// and never which contour it means.
    #[test]
    fn a_contour_wound_the_other_way_is_read_as_a_hole() {
        let got = section_from(
            &serde_json::json!({"contours": [square(0.0, 0.0, 2.0), square_hole(0.0, 0.0, 1.0)]}),
            zplane(),
        )
        .expect("a ring");
        match got {
            Section::Cut { contours, .. } => {
                assert!(!contours[0].is_hole, "the outer contour is solid");
                assert!(contours[1].is_hole, "the inner contour is a hole");
                assert_eq!(contours[1].area_mm2, 4.0, "magnitude, not sign");
            }
            other => panic!("expected a cut, got {other:?}"),
        }
    }

    /// And a hole never pairs with a solid contour of the same size: they can
    /// agree on centroid and area exactly and still be opposite statements
    /// about the material.
    #[test]
    fn a_solid_becoming_a_void_is_not_reported_as_unchanged() {
        let solid = section_from(
            &serde_json::json!({"contours": [square(0.0, 0.0, 1.0)]}),
            zplane(),
        )
        .expect("solid");
        let void = section_from(
            &serde_json::json!({"contours": [square_hole(0.0, 0.0, 1.0)]}),
            zplane(),
        )
        .expect("void");
        let d = crate::regions::diff_sections(
            &solid,
            &void,
            crate::regions::Comparison::ContourIdentity {
                centroid_tol_mm: 0.01,
                area_tol_mm2: 0.01,
            },
        )
        .expect("same plane");
        assert!(!d.unchanged(), "a solid became a void: {:?}", d.regions);
    }

    /// THE PARSE IS CHECKED AGAINST THE PROVIDER'S OWN AGGREGATE. Misreading
    /// a hole as solid, or dropping a contour, changes the sum; a diff built
    /// on a misread reply is worse than no diff.
    #[test]
    fn contours_that_do_not_sum_to_the_reported_area_are_refused_as_a_misread() {
        let err = section_from(
            &serde_json::json!({
                "area": 999.0,
                "contours": [square(0.0, 0.0, 1.0)]
            }),
            zplane(),
        )
        .expect_err("the parse disagrees with the provider's own total");
        assert!(err.contains("MISREAD"), "{err}");
    }

    #[test]
    fn a_point_that_is_not_in_the_plane_is_refused() {
        let err = section_from(
            &serde_json::json!({"contours": [[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]]}),
            zplane(),
        )
        .expect_err("three-component points are not in the cut plane");
        assert!(err.contains("exactly two"), "{err}");
    }

    #[test]
    fn a_contour_with_too_few_points_has_no_area_and_is_refused() {
        let err = section_from(
            &serde_json::json!({"contours": [[[0.0, 0.0], [1.0, 0.0]]]}),
            zplane(),
        )
        .expect_err("two points are not a closed contour");
        assert!(err.contains("at least three"), "{err}");
    }

    /// RESTORED 2026-11-19 after I deleted it while rewriting these tests for
    /// the real wire shape. The mutant `wire-absent-contours-is-an-empty-cut`
    /// SURVIVED the next battery and said so -- removing a test is the one
    /// kind of regression a green suite cannot show you.
    ///
    /// An ABSENT `contours` array is not an empty cut. Treating it as one
    /// makes an unanswered question and a plane that missed the solid into the
    /// same answer.
    #[test]
    fn an_absent_contour_array_refuses_rather_than_reading_as_an_empty_cut() {
        let err = section_from(&serde_json::json!({"mode": "section"}), zplane())
            .expect_err("no contours key at all");
        assert!(err.contains("no `contours` array"), "{err}");
    }

    #[test]
    fn an_empty_section_is_an_answer_and_not_a_missing_one() {
        let got = section_from(
            &serde_json::json!({"empty_section": true, "contours": []}),
            zplane(),
        )
        .expect("an empty cut is a real answer");
        match got {
            Section::Cut { contours, .. } => assert!(contours.is_empty()),
            other => panic!("expected an empty cut, got {other:?}"),
        }
    }

    use crate::diff::{diff, Finding, Intent};

    fn fx() -> Value {
        serde_json::from_str(include_str!("../../../tests/fixtures/provider-diff.json"))
            .expect("the captured fixture parses")
    }

    fn snap(case: &str, params: &[(&str, &str)]) -> Snapshot {
        let all = fx();
        let digest = all[case]["sha256_of_bytes"]
            .as_str()
            .expect("the fixture recorded a digest")
            .to_string();
        snapshot_from(
            &all[case]["measure"],
            digest,
            params
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        )
        .expect("builds")
    }

    /// The ordinary edit: the bore grew, so volume fell and the bytes changed.
    #[test]
    fn an_edit_that_moved_geometry_reports_the_volume_it_removed() {
        let (m, f) = diff(
            &snap("before", &[("R", "3")]),
            &snap("after", &[("R", "4")]),
            Intent::Edit,
        )
        .expect("valid");
        assert!(f.is_empty(), "{f:?}");
        assert!(
            (m.removed_volume_mm3 - (3717.7106 - 3498.1522)).abs() < 1e-6,
            "{m:?}"
        );
    }

    /// THE CASE A VOLUME-KEYED DIFF CANNOT SEE, in measured data rather than
    /// a constructed one: the moved part has EXACTLY the volume it had --
    /// 3717.7106 both times, from the provider -- and different bytes.
    #[test]
    fn a_translated_part_has_the_volume_it_had_and_is_still_a_change() {
        let before = snap("before", &[("x", "0")]);
        let moved = snap("moved", &[("x", "3")]);
        assert_eq!(
            before.volume_mm3, moved.volume_mm3,
            "the provider measures the translated part at the same volume"
        );
        assert_ne!(before.mesh_hash, moved.mesh_hash, "and different bytes");
        let (m, f) = diff(&before, &moved, Intent::Edit).expect("valid");
        assert!(!m.identical);
        assert_eq!(m.added_volume_mm3, 0.0);
        assert_eq!(m.removed_volume_mm3, 0.0);
        assert!(f.contains(&Finding::SameVolumeDifferentMesh), "{f:?}");
    }

    /// A refactor that moved the same part fails, from the same two snapshots.
    #[test]
    fn the_same_pair_under_refactor_is_a_failure() {
        let (_, f) =
            diff(&snap("before", &[]), &snap("moved", &[]), Intent::Refactor).expect("valid");
        assert!(
            f.iter()
                .any(|x| matches!(x, Finding::RefactorMovedGeometry { .. })),
            "{f:?}"
        );
    }

    /// Identical bytes are identical output, which under Edit is the edit not
    /// applying.
    #[test]
    fn the_same_snapshot_twice_is_the_edit_not_applying() {
        let (_, f) = diff(
            &snap("before", &[("R", "3")]),
            &snap("before", &[("R", "4")]),
            Intent::Edit,
        )
        .expect("valid");
        assert!(
            f.iter()
                .any(|x| matches!(x, Finding::ChangeDidNotApply { .. })),
            "{f:?}"
        );
        assert!(
            f.iter()
                .any(|x| matches!(x, Finding::ParametersWentNowhere { .. })),
            "and names R, which is what gets fixed: {f:?}"
        );
    }

    /// THE FORMAT IS PART OF THE IDENTITY: the same bytes under two formats
    /// are not interchangeable, and the digest says so.
    #[test]
    fn the_same_bytes_under_two_formats_do_not_share_an_identity() {
        let b = b"solid x\nendsolid x\n";
        assert_ne!(digest_of_bytes("stl", b), digest_of_bytes("3mf", b));
        assert_eq!(digest_of_bytes("stl", b), digest_of_bytes("stl", b));
    }

    /// AND DIFFERENT BYTES DIFFER. A digest that collapsed everything would
    /// make every pair bit-identical, which is the Refactor pass.
    #[test]
    fn different_bytes_give_different_identities() {
        assert_ne!(
            digest_of_bytes("stl", b"aaaa"),
            digest_of_bytes("stl", b"aaab")
        );
        assert_ne!(digest_of_bytes("stl", b""), digest_of_bytes("stl", b"a"));
    }

    /// AN EXPORT WITH NO PATH IS REFUSED, never an empty hash.
    #[test]
    fn an_export_naming_no_path_is_refused_rather_than_hashed_empty() {
        let e = digest_of_export(&serde_json::json!({"success": true})).expect_err("must refuse");
        assert!(e.contains("read as bit-identical"), "{e}");
    }

    #[test]
    fn a_failed_export_is_refused() {
        assert!(digest_of_export(&serde_json::json!({"success": false})).is_err());
    }

    #[test]
    fn an_unreadable_export_path_is_refused() {
        let e = digest_of_export(&serde_json::json!({
            "success": true, "output_path": "/nonexistent/wm/none.stl", "format": "stl"
        }))
        .expect_err("must refuse");
        assert!(e.contains("could not be read"), "{e}");
    }

    /// A ZERO-BYTE EXPORT IS A FAILED EXPORT, not an empty model: the kernel
    /// writes no file at all for an empty model.
    #[test]
    fn a_zero_byte_export_is_refused() {
        let dir = std::env::temp_dir().join("wm-model-wire-test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let p = dir.join("empty.stl");
        std::fs::write(&p, b"").expect("write");
        let e = digest_of_export(&serde_json::json!({
            "success": true, "output_path": p.to_str().expect("a utf-8 temp path"), "format": "stl"
        }))
        .expect_err("must refuse");
        assert!(e.contains("failed export"), "{e}");
        let _ = std::fs::remove_file(&p);
    }

    /// A MEASUREMENT WITH NO VOLUME IS REFUSED: zero would make an unmeasured
    /// model and an empty one the same snapshot.
    #[test]
    fn a_measurement_without_a_volume_is_refused() {
        let e = snapshot_from(
            &serde_json::json!({"success": true}),
            "stl:abc".into(),
            BTreeMap::new(),
        )
        .expect_err("must refuse");
        assert!(e.contains("the same snapshot"), "{e}");
    }

    /// And an empty digest cannot get in through the back door either.
    #[test]
    fn an_empty_digest_is_refused_at_the_snapshot() {
        assert!(snapshot_from(
            &serde_json::json!({"success": true, "volume": 1.0}),
            "  ".into(),
            BTreeMap::new()
        )
        .is_err());
    }
}
