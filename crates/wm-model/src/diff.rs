//! Geometric diff, where an empty diff is an answer and not a pass.
//!
//! # The incident
//!
//! From the spec: *"One adjacent incident: a bit-identical result after a
//! change was the tell that the change had not applied."*
//!
//! The mechanism is `-D`. OpenSCAD's `-D name=value` **appends** an
//! assignment, so a misspelled parameter name defines a variable nothing
//! reads. That is legal -- an assignment nobody reads draws no warning -- and
//! the render proceeds, succeeds, and produces exactly the bytes it produced
//! before. Exit code zero, no diagnostics, no change.
//!
//! # The same empty diff means opposite things
//!
//! An unchanged result is *success* for a refactor: extracting a module,
//! renaming a variable, reflowing whitespace must all be bit-identical, and a
//! difference there is the defect. It is *failure* for a parameter edit: the
//! caller changed a number and the geometry did not move.
//!
//! Nothing in the two snapshots distinguishes those cases. Only the caller's
//! intent does, so [`diff`] requires it to be stated and refuses to guess.

use std::collections::BTreeMap;

/// One rendered state of a model.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    /// Content hash of the rendered mesh. Two equal hashes are bit-identical
    /// output, which is the strongest signal available here and the whole
    /// reason this type carries one.
    pub mesh_hash: String,
    pub volume_mm3: f64,
    /// The parameters this render was given, as the caller set them.
    pub params: BTreeMap<String, String>,
}

/// What the caller believed they were doing. There is no default: the empty
/// diff is a pass under one of these and a failure under the other, and
/// picking one silently would make the tool wrong half the time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// The change was meant to move geometry. An identical result is a
    /// failure.
    Edit,
    /// The change was meant to preserve geometry exactly -- a module
    /// extraction, a rename, a reflow. A difference is a failure.
    Refactor,
}

/// A measured difference between two renders.
#[derive(Debug, Clone, PartialEq)]
pub struct Measured {
    pub added_volume_mm3: f64,
    pub removed_volume_mm3: f64,
    /// Parameters whose value differs, with before and after.
    pub params_changed: Vec<(String, Option<String>, Option<String>)>,
    pub identical: bool,
}

/// Something about the pair that the caller needs told, beyond the numbers.
#[derive(Debug, Clone, PartialEq)]
pub enum Finding {
    /// Bit-identical output under [`Intent::Edit`]. The edit did not apply.
    ChangeDidNotApply { params_changed: Vec<String> },
    /// Output differs under [`Intent::Refactor`]. The refactor moved geometry.
    RefactorMovedGeometry { net_mm3: f64 },
    /// Parameters changed and the geometry did not. This is the `-D`
    /// misspelling: the name was accepted, assigned, and read by nothing.
    ParametersWentNowhere { names: Vec<String> },
    /// The geometry changed and no parameter did. Something else moved -- an
    /// edited source file, a library that resolved differently, a changed
    /// `$fn`.
    GeometryChangedWithNoParameterChange { net_mm3: f64 },
    /// The hash differs and the volume does not. A part moved or was
    /// reoriented without being resized, which volume alone cannot see.
    SameVolumeDifferentMesh,
}

/// Compare two renders under a stated intent.
///
/// # Why a volume comparison is not enough on its own
///
/// Equal volumes do not mean equal geometry -- a part that moved 10 mm has
/// exactly the volume it had. So the hash decides whether anything changed,
/// and the volumes only describe the change. A tool that keyed on volume
/// would report "no change" for every pure translation.
///
/// REFUSES A SNAPSHOT WITH AN EMPTY HASH. An empty-string hash compares equal
/// to another empty-string hash, so two un-hashed snapshots would report
/// themselves bit-identical -- which under `Intent::Refactor` is a pass. That
/// is the failure mode this whole module exists to catch, arriving through the
/// tool meant to catch it.
pub fn diff(
    before: &Snapshot,
    after: &Snapshot,
    intent: Intent,
) -> Result<(Measured, Vec<Finding>), String> {
    for (which, s) in [("before", before), ("after", after)] {
        if s.mesh_hash.trim().is_empty() {
            return Err(format!(
                "the {which} snapshot has no mesh hash. Two un-hashed snapshots compare \
                 equal and would report themselves bit-identical, which is a PASS under \
                 Intent::Refactor -- the exact failure this tool exists to catch"
            ));
        }
        if !s.volume_mm3.is_finite() || s.volume_mm3 < 0.0 {
            return Err(format!(
                "the {which} snapshot's volume is {}, which is not a volume",
                s.volume_mm3
            ));
        }
    }

    let identical = before.mesh_hash == after.mesh_hash;
    let net = after.volume_mm3 - before.volume_mm3;
    let mut params_changed = Vec::new();
    let mut names: Vec<&str> = before.params.keys().map(String::as_str).collect();
    for k in after.params.keys() {
        if !before.params.contains_key(k) {
            names.push(k);
        }
    }
    names.sort_unstable();
    for k in names {
        let b = before.params.get(k);
        let a = after.params.get(k);
        if b != a {
            params_changed.push((k.to_string(), b.cloned(), a.cloned()));
        }
    }

    let measured = Measured {
        added_volume_mm3: net.max(0.0),
        removed_volume_mm3: (-net).max(0.0),
        params_changed: params_changed.clone(),
        identical,
    };

    let changed_names: Vec<String> = params_changed.iter().map(|(k, _, _)| k.clone()).collect();
    let mut findings = Vec::new();
    match (intent, identical) {
        (Intent::Edit, true) => {
            findings.push(Finding::ChangeDidNotApply {
                params_changed: changed_names.clone(),
            });
            // BOTH findings, not one. "The edit did nothing" is the headline;
            // "and these are the names it was given" is what gets fixed, and
            // a misspelled name is only visible in the second.
            if !changed_names.is_empty() {
                findings.push(Finding::ParametersWentNowhere {
                    names: changed_names,
                });
            }
        }
        (Intent::Refactor, false) => {
            findings.push(Finding::RefactorMovedGeometry { net_mm3: net });
            if changed_names.is_empty() {
                findings.push(Finding::GeometryChangedWithNoParameterChange { net_mm3: net });
            }
        }
        (Intent::Edit, false) => {
            if changed_names.is_empty() {
                findings.push(Finding::GeometryChangedWithNoParameterChange { net_mm3: net });
            }
        }
        (Intent::Refactor, true) => {}
    }
    if !identical && (after.volume_mm3 - before.volume_mm3).abs() < f64::EPSILON {
        findings.push(Finding::SameVolumeDifferentMesh);
    }
    Ok((measured, findings))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(hash: &str, vol: f64, params: &[(&str, &str)]) -> Snapshot {
        Snapshot {
            mesh_hash: hash.into(),
            volume_mm3: vol,
            params: params
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        }
    }

    /// THE INCIDENT. A parameter was changed, the render succeeded, and the
    /// bytes are identical. The `-D` name was assigned and read by nothing.
    #[test]
    fn a_bit_identical_result_after_an_edit_is_the_edit_not_applying() {
        let b = snap("abc123", 744.6, &[("socket_r", "42.1")]);
        let a = snap("abc123", 744.6, &[("socket_r", "43.5")]);
        let (m, f) = diff(&b, &a, Intent::Edit).expect("valid");
        assert!(m.identical);
        assert_eq!(
            f,
            vec![
                Finding::ChangeDidNotApply {
                    params_changed: vec!["socket_r".into()]
                },
                Finding::ParametersWentNowhere {
                    names: vec!["socket_r".into()]
                },
            ],
            "both: that it did nothing, and the name that went nowhere"
        );
    }

    /// THE SAME EMPTY DIFF IS THE PASS for a refactor. Extracting a module
    /// must be bit-identical.
    #[test]
    fn the_same_empty_diff_is_success_for_a_refactor() {
        let b = snap("abc123", 744.6, &[]);
        let a = snap("abc123", 744.6, &[]);
        let (m, f) = diff(&b, &a, Intent::Refactor).expect("valid");
        assert!(m.identical);
        assert!(f.is_empty(), "nothing to report: {f:?}");
    }

    /// And a refactor that moved geometry is the failure.
    #[test]
    fn a_refactor_that_moved_geometry_is_a_failure() {
        let b = snap("abc123", 744.6, &[]);
        let a = snap("def456", 800.0, &[]);
        let (_, f) = diff(&b, &a, Intent::Refactor).expect("valid");
        assert!(
            f.contains(&Finding::RefactorMovedGeometry {
                net_mm3: 800.0 - 744.6
            }),
            "{f:?}"
        );
    }

    /// AN UN-HASHED SNAPSHOT IS REFUSED, because two empty hashes compare
    /// equal and would report themselves bit-identical -- a PASS under
    /// Refactor, through the tool built to catch exactly that.
    #[test]
    fn an_unhashed_snapshot_is_refused_rather_than_compared() {
        let b = snap("", 1.0, &[]);
        let a = snap("", 1.0, &[]);
        let e = diff(&b, &a, Intent::Refactor).expect_err("must refuse");
        assert!(e.contains("would report themselves bit-identical"), "{e}");
        // and whitespace is not a hash either
        assert!(diff(&snap("  ", 1.0, &[]), &a, Intent::Edit).is_err());
    }

    /// A PURE TRANSLATION has exactly the volume it had, so a tool keyed on
    /// volume would call it no change. The hash decides.
    #[test]
    fn a_part_that_moved_without_resizing_is_still_a_change() {
        let b = snap("abc123", 744.6, &[("y", "0")]);
        let a = snap("def456", 744.6, &[("y", "10")]);
        let (m, f) = diff(&b, &a, Intent::Edit).expect("valid");
        assert!(!m.identical, "the hashes differ, so the model changed");
        assert_eq!(m.added_volume_mm3, 0.0);
        assert_eq!(m.removed_volume_mm3, 0.0);
        assert!(
            f.contains(&Finding::SameVolumeDifferentMesh),
            "volume alone cannot see this: {f:?}"
        );
    }

    /// Geometry changed and no parameter did: something else moved. Worth
    /// saying, because the caller thinks they know what they changed.
    #[test]
    fn geometry_that_changed_with_no_parameter_change_is_reported() {
        let b = snap("abc123", 744.6, &[("r", "42.1")]);
        let a = snap("def456", 900.0, &[("r", "42.1")]);
        let (_, f) = diff(&b, &a, Intent::Edit).expect("valid");
        assert!(
            f.contains(&Finding::GeometryChangedWithNoParameterChange {
                net_mm3: 900.0 - 744.6
            }),
            "{f:?}"
        );
    }

    /// The ordinary good case: a parameter changed and the geometry moved.
    #[test]
    fn an_edit_that_applied_reports_the_volume_it_moved_and_nothing_else() {
        let b = snap("abc123", 744.6, &[("socket_r", "42.1")]);
        let a = snap("def456", 800.0, &[("socket_r", "43.5")]);
        let (m, f) = diff(&b, &a, Intent::Edit).expect("valid");
        assert!(f.is_empty(), "{f:?}");
        assert!((m.added_volume_mm3 - 55.4).abs() < 1e-9, "{m:?}");
        assert_eq!(m.removed_volume_mm3, 0.0);
        assert_eq!(m.params_changed.len(), 1);
    }

    /// Added and removed are separate figures, not one signed net, so a
    /// shrink is not reported as a negative addition.
    #[test]
    fn a_shrink_is_removed_volume_not_negative_added_volume() {
        let b = snap("abc123", 800.0, &[("r", "43.5")]);
        let a = snap("def456", 744.6, &[("r", "42.1")]);
        let (m, _) = diff(&b, &a, Intent::Edit).expect("valid");
        assert_eq!(m.added_volume_mm3, 0.0);
        assert!((m.removed_volume_mm3 - 55.4).abs() < 1e-9, "{m:?}");
    }

    /// A PARAMETER THAT APPEARED OR VANISHED is a change too. A `-D` on one
    /// render and not the other is the single most likely shape of this bug,
    /// and a key-by-key walk of only the BEFORE map would miss it.
    #[test]
    fn a_parameter_present_on_only_one_side_counts_as_changed() {
        let b = snap("abc123", 744.6, &[]);
        let a = snap("abc123", 744.6, &[("sockt_r", "43.5")]);
        let (m, f) = diff(&b, &a, Intent::Edit).expect("valid");
        assert_eq!(
            m.params_changed,
            vec![("sockt_r".to_string(), None, Some("43.5".to_string()))]
        );
        assert!(
            f.contains(&Finding::ParametersWentNowhere {
                names: vec!["sockt_r".into()]
            }),
            "the misspelled name is what gets fixed: {f:?}"
        );
    }

    #[test]
    fn a_negative_or_non_finite_volume_is_refused() {
        let ok = snap("abc", 1.0, &[]);
        assert!(diff(&snap("abc", -1.0, &[]), &ok, Intent::Edit).is_err());
        assert!(diff(&ok, &snap("def", f64::NAN, &[]), Intent::Edit).is_err());
    }
}
