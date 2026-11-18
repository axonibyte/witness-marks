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
    let rows = measure
        .get("contours")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "the section answer carries no `contours` array. Treating its absence as an \
             empty cut would make an unanswered question and a plane that missed the \
             solid the same answer"
                .to_string()
        })?;
    let mut contours = Vec::with_capacity(rows.len());
    for (n, row) in rows.iter().enumerate() {
        let num = |k: &str| -> Result<f64, String> {
            row.get(k)
                .and_then(Value::as_f64)
                .ok_or_else(|| format!("contour {n} carries no numeric `{k}`"))
        };
        let centroid = row
            .get("centroid")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("contour {n} carries no `centroid`"))?;
        if centroid.len() != 2 {
            return Err(format!(
                "contour {n}'s centroid has {} components; a cut contour lies in the \
                 plane and has exactly two",
                centroid.len()
            ));
        }
        let bbox = row
            .get("bbox")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("contour {n} carries no `bbox`"))?;
        if bbox.len() != 4 {
            return Err(format!(
                "contour {n}'s bbox has {} components; an in-plane box has four",
                bbox.len()
            ));
        }
        let pick = |a: &[Value], i: usize, what: &str| -> Result<f64, String> {
            a[i].as_f64()
                .ok_or_else(|| format!("contour {n}'s {what}[{i}] is not a number"))
        };
        contours.push(Contour {
            centroid_mm: [
                pick(centroid, 0, "centroid")?,
                pick(centroid, 1, "centroid")?,
            ],
            area_mm2: num("area")?,
            bbox_mm: [
                pick(bbox, 0, "bbox")?,
                pick(bbox, 1, "bbox")?,
                pick(bbox, 2, "bbox")?,
                pick(bbox, 3, "bbox")?,
            ],
            // ABSENCE IS CARRIED AS ABSENCE. Defaulting a missing vertex
            // count to any number makes an unmeasured contour look measured,
            // and the comparison then reports a reshape nobody observed. A
            // surviving mutant proved the first version's justification for
            // picking usize::MAX over 0 was false: both behave identically,
            // because absence is not a value and no number can stand in for
            // it.
            vertices: row
                .get("vertices")
                .and_then(Value::as_u64)
                .map(|v| v as usize),
        });
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

    fn row(cx: f64, cy: f64, area: f64) -> Value {
        serde_json::json!({
            "centroid": [cx, cy], "area": area,
            "bbox": [cx - 1.0, cy - 1.0, cx + 1.0, cy + 1.0], "vertices": 4
        })
    }

    #[test]
    fn a_section_answer_becomes_contours_in_the_plane_the_caller_named() {
        let got = section_from(
            &serde_json::json!({"contours": [row(0.0, 0.0, 4.0)]}),
            zplane(),
        )
        .expect("a well-formed section");
        match got {
            Section::Cut { plane, contours } => {
                assert_eq!(plane, zplane());
                assert_eq!(contours.len(), 1);
                assert_eq!(contours[0].area_mm2, 4.0);
                assert_eq!(contours[0].centroid_mm, [0.0, 0.0]);
            }
            other => panic!("wanted a cut, got {other:?}"),
        }
    }

    #[test]
    fn an_empty_contour_array_is_a_cut_that_missed_and_not_an_unresolved_one() {
        let got = section_from(&serde_json::json!({"contours": []}), zplane())
            .expect("empty is an answer");
        assert!(
            matches!(got, Section::Cut { ref contours, .. } if contours.is_empty()),
            "{got:?}"
        );
    }

    #[test]
    fn an_absent_contour_array_refuses_rather_than_reading_as_an_empty_cut() {
        let err = section_from(&serde_json::json!({}), zplane()).expect_err("no contours key");
        assert!(err.contains("no `contours`"), "{err}");
    }

    #[test]
    fn unresolved_is_carried_as_unresolved_and_never_as_no_contours() {
        let s =
            section_from(&serde_json::json!({"state": "UNRESOLVED"}), zplane()).expect("carried");
        assert!(matches!(s, Section::Unresolved { .. }), "{s:?}");
        let f = section_from(
            &serde_json::json!({"success": false, "error": "cut failed"}),
            zplane(),
        )
        .expect("carried");
        match f {
            Section::Unresolved { why, .. } => assert!(why.contains("cut failed"), "{why}"),
            other => panic!("wanted unresolved, got {other:?}"),
        }
    }

    #[test]
    fn a_centroid_that_is_not_in_the_plane_is_refused() {
        let bad = serde_json::json!({"contours": [{"centroid": [0.0, 0.0, 0.0], "area": 4.0,
                                                   "bbox": [0.0, 0.0, 1.0, 1.0]}]});
        let err = section_from(&bad, zplane()).expect_err("three components");
        assert!(err.contains("exactly two"), "{err}");
    }

    #[test]
    fn a_bbox_that_is_not_an_in_plane_box_is_refused() {
        let bad = serde_json::json!({"contours": [{"centroid": [0.0, 0.0], "area": 4.0,
                                                   "bbox": [0.0, 0.0, 1.0]}]});
        let err = section_from(&bad, zplane()).expect_err("three components");
        assert!(err.contains("four"), "{err}");
    }

    #[test]
    fn a_contour_with_no_vertex_count_is_unknown_rather_than_a_count() {
        // THE CASE THAT DISCRIMINATES, and the one the first version of this
        // test missed: both sides built from the same JSON default
        // identically, so that test passed whatever number absence became. One
        // side reporting a count and the other not is where a default shows
        // up -- as a reshape nobody measured.
        let known = serde_json::json!({"contours": [{"centroid": [0.0, 0.0], "area": 4.0,
                                                     "bbox": [-1.0, -1.0, 1.0, 1.0], "vertices": 4}]});
        let unknown = serde_json::json!({"contours": [{"centroid": [0.0, 0.0], "area": 4.0,
                                                       "bbox": [-1.0, -1.0, 1.0, 1.0]}]});
        let a = section_from(&known, zplane()).expect("ok");
        let b = section_from(&unknown, zplane()).expect("ok");
        match b {
            Section::Cut { ref contours, .. } => assert_eq!(contours[0].vertices, None),
            ref other => panic!("wanted a cut, got {other:?}"),
        }
        let d = crate::regions::diff_sections(
            &a,
            &b,
            crate::regions::Comparison::ContourIdentity {
                centroid_tol_mm: 0.01,
                area_tol_mm2: 0.01,
            },
        )
        .expect("same plane");
        // No region: a known count against an unknown one is not evidence of
        // a reshape. But the answer SAYS it could not check.
        assert!(d.unchanged(), "{:?}", d.regions);
        assert_eq!(d.unchecked_pairs, 1);
        assert!(!d.fully_checked());
    }

    #[test]
    fn a_missing_vertex_count_compares_equal_rather_than_reading_as_reshaped() {
        // Absence on BOTH sides must not make every paired contour look like
        // it changed shape -- which a default of 0 would, and which usize::MAX
        // avoids by comparing equal to itself.
        let no_verts = serde_json::json!({"contours": [{"centroid": [0.0, 0.0], "area": 4.0,
                                                        "bbox": [-1.0, -1.0, 1.0, 1.0]}]});
        let a = section_from(&no_verts, zplane()).expect("ok");
        let b = section_from(&no_verts, zplane()).expect("ok");
        let d = crate::regions::diff_sections(
            &a,
            &b,
            crate::regions::Comparison::ContourIdentity {
                centroid_tol_mm: 0.01,
                area_tol_mm2: 0.01,
            },
        )
        .expect("same plane");
        assert!(d.unchanged(), "{:?}", d.regions);
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
