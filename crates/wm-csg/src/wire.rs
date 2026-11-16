//! Reading the provider's answers into this crate's judgments.
//!
//! The parse lives here rather than in `wm-provider` so it stays PURE: it
//! takes a `serde_json::Value` and returns a judgment input, with no process
//! to launch. That matters because the provider needs uv, a resolved lockfile
//! and OpenSCAD and cannot run in the forge jail, while the parse is where the
//! bugs are. Fixtures captured from the real provider drive it everywhere.
//!
//! # What the provider actually answers
//!
//! `measure(mode=probe)` returns, per point, a `state` of `solid`, `air` or
//! **`on_surface`**, plus the `parts` that own it.
//!
//! The author's first sample used three points, none of which lay on a face,
//! saw only `solid` and `air`, and concluded the provider had no boundary
//! state and that [`crate::locate::At::OnBoundary`] would have to be recovered
//! by probing a neighborhood. It has one. A sample that cannot exhibit a case
//! is not evidence that the case is absent -- and a fixture here therefore
//! carries all three states on purpose.
//!
//! # The narrowing that is real
//!
//! `on_surface` arrives with NO tolerance attached. This crate's `OnBoundary`
//! carries the tolerance it was judged at, because "on the face" means nothing
//! without one; the provider's threshold is its own, internal and undeclared.
//! So a boundary verdict read from the provider is labelled with the
//! provider's name rather than a number, and [`at_from_probe`] will not invent
//! one.

use crate::locate::{At, Attribution};
use serde_json::Value;

/// The provider's own word for the tolerance it judged `on_surface` at.
///
/// Not a number, because the provider does not publish one. Putting a
/// plausible number here would be the same error as reading a mode out of
/// prose: an invented figure that reads exactly like a measured one.
pub const PROVIDER_SURFACE_TOL: f64 = f64::NAN;

/// One probed point as the provider reported it.
#[derive(Debug, Clone, PartialEq)]
pub struct Probed {
    pub at: [f64; 3],
    pub state: String,
    pub parts: Vec<String>,
}

/// Read `measure(mode=probe)` into points.
///
/// REFUSES AN UNSUCCESSFUL ANSWER and REFUSES AN UNKNOWN STATE. A state this
/// code does not recognize is not air: treating it as air turns a provider
/// that grew a fourth answer into a part that reads as empty space, which is
/// the failure this project names everywhere else as "empty is a value".
pub fn probed_points(payload: &Value) -> Result<Vec<Probed>, String> {
    if payload.get("success").and_then(Value::as_bool) == Some(false) {
        return Err(format!(
            "the provider reported success=false: {}",
            payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("no error named")
        ));
    }
    let points = payload
        .get("points")
        .and_then(Value::as_array)
        .ok_or_else(|| "the answer carries no `points` array".to_string())?;
    if points.is_empty() {
        return Err(
            "the answer carries an EMPTY `points` array. No points probed and no \
                    material found are the same bytes, and only one of them is a measurement"
                .into(),
        );
    }
    let mut out = Vec::with_capacity(points.len());
    for (i, p) in points.iter().enumerate() {
        let state = p
            .get("state")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("point {i} has no `state`"))?;
        if !matches!(state, "solid" | "air" | "on_surface") {
            return Err(format!(
                "point {i} has state {state:?}, which this reader does not know. Refusing \
                 rather than defaulting it: a state read as `air` makes material into empty \
                 space, and a provider that grows a fourth answer should break loudly"
            ));
        }
        let at = p
            .get("at")
            .and_then(Value::as_array)
            .and_then(|a| {
                let v: Vec<f64> = a.iter().filter_map(Value::as_f64).collect();
                (v.len() == 3).then(|| [v[0], v[1], v[2]])
            })
            .ok_or_else(|| format!("point {i} has no usable `at`"))?;
        out.push(Probed {
            at,
            state: state.to_string(),
            parts: p
                .get("parts")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
        });
    }
    Ok(out)
}

/// Turn one probed point into this crate's [`At`].
///
/// The three provider states map one to one. `nearest_face_mm` is NOT
/// available from a probe -- the provider reports no distance -- so it is
/// reported as zero only for the boundary case, where zero is the truth, and
/// the solid/air cases carry [`f64::NAN`] to say the distance was not
/// measured. A plausible number here would be indistinguishable from a
/// measured one.
pub fn at_from_probe(p: &Probed) -> Result<At, String> {
    match p.state.as_str() {
        "on_surface" => Ok(At::OnBoundary {
            tol_mm: PROVIDER_SURFACE_TOL,
            contributing: p.parts.clone(),
        }),
        "solid" => {
            if p.parts.is_empty() {
                return Err(format!(
                    "the provider reports solid at {:?} and names no part. Something built \
                     that material, so an empty attribution is an attribution failure",
                    p.at
                ));
            }
            Ok(At::Inside {
                contributing: p.parts.clone(),
                nearest_face_mm: f64::NAN,
            })
        }
        "air" => Ok(At::Outside {
            nearest_face_mm: f64::NAN,
        }),
        other => Err(format!("unknown probe state {other:?}")),
    }
}

/// Read `measure(mode=features)` into an attribution for one named feature.
///
/// # Pre-boolean is the whole point
///
/// The provider's own note: *"circular subtractive features from the CSG tree
/// (post-expansion, pre-boolean); not holes in the finished part. Features
/// inside hull()/minkowski() are masked."*
///
/// PRE-BOOLEAN is exactly the distinction [`crate::locate::locate`] makes. A
/// feature that ran and was then consumed by a later operation still appears
/// here, because this is the construction tree and not the result. So the
/// features list answers "was it instantiated", and only a measurement of the
/// RESULT answers "did its material survive" -- which is why `surviving` is a
/// separate argument rather than something this function invents.
///
/// MASKED FEATURES ARE REFUSED, not reported absent. `counts.masked_hull`
/// above zero means the provider could not see inside a `hull()` or
/// `minkowski()`, so "this feature is not in the list" stops meaning "this
/// feature does not exist".
pub fn attribution_from_features(
    payload: &Value,
    module: &str,
    surviving: Option<(crate::bbox::Bbox, [f64; 3])>,
) -> Result<Attribution, String> {
    let counts = payload.get("counts");
    let masked = counts
        .and_then(|c| c.get("masked_hull"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if masked > 0 {
        return Err(format!(
            "{masked} feature(s) are masked inside hull() or minkowski(), so the provider \
             cannot see them. An absence from this list no longer means the feature is not \
             there, and `{module}` cannot be judged against it"
        ));
    }
    let unresolved = counts
        .and_then(|c| c.get("unresolved"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if unresolved > 0 {
        return Err(format!(
            "{unresolved} feature(s) are unresolved, so this list is not a complete account \
             of the tree and `{module}` cannot be judged against it"
        ));
    }
    let patterns = payload
        .get("patterns")
        .and_then(Value::as_array)
        .ok_or_else(|| "the answer carries no `patterns` array".to_string())?;

    // A pattern is matched by its description, which is what the provider
    // gives a feature for a name. Counting ENTRIES and not patterns: one
    // pattern with four entries is a module that ran four times, and the
    // call-site count is what Consumed reports.
    let mut call_sites = 0usize;
    for p in patterns {
        let desc = p.get("description").and_then(Value::as_str).unwrap_or("");
        if desc.contains(module) {
            call_sites += p
                .get("entries")
                .and_then(Value::as_array)
                .map(|e| e.len())
                .or_else(|| p.get("count").and_then(Value::as_u64).map(|c| c as usize))
                .unwrap_or(1);
        }
    }
    Ok(Attribution {
        module: module.to_string(),
        // DECLARED means the tree knows the name. The features list is the
        // tree, so a name that appears anywhere in it is declared; a name that
        // appears nowhere may still be declared and uncalled, which this
        // reader cannot tell apart and says so by leaving `declared` true only
        // when something matched.
        declared: call_sites > 0,
        call_sites,
        surviving,
        eaten_by: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locate::{locate, Located};

    fn fixture() -> Value {
        let raw = include_str!("../../../tests/fixtures/provider-probe.json");
        serde_json::from_str(raw).expect("the captured fixture parses")
    }

    fn cluster(name: &str) -> Value {
        fixture()["clusters"][name].clone()
    }

    /// ALL THREE STATES ARE IN THE FIXTURE ON PURPOSE. The author's first
    /// sample held two and produced a confident wrong claim that the third
    /// did not exist.
    #[test]
    fn the_fixture_exhibits_solid_air_and_on_surface() {
        let mut seen: Vec<String> = Vec::new();
        for name in ["deep_solid", "in_the_hole", "on_the_wall"] {
            for p in probed_points(&cluster(name)).expect("parses") {
                if !seen.contains(&p.state) {
                    seen.push(p.state);
                }
            }
        }
        seen.sort();
        assert_eq!(seen, vec!["air", "on_surface", "solid"]);
    }

    /// A point exactly on the bore wall is OnBoundary, read from the
    /// provider's own `on_surface` rather than inferred.
    #[test]
    fn a_point_on_the_bore_wall_reads_as_the_boundary() {
        let pts = probed_points(&cluster("on_the_wall")).expect("parses");
        match at_from_probe(&pts[0]).expect("valid") {
            At::OnBoundary {
                tol_mm,
                contributing,
            } => {
                assert!(tol_mm.is_nan(), "the provider publishes no tolerance");
                assert_eq!(contributing, vec!["model".to_string()]);
            }
            other => panic!("13,10,5 is exactly r=3 from the axis; got {other:?}"),
        }
    }

    /// Its neighbors a tenth of a millimetre either side are NOT the boundary,
    /// which is what makes the boundary reading mean something.
    #[test]
    fn the_neighbors_of_the_wall_are_plainly_inside_and_outside() {
        let pts = probed_points(&cluster("on_the_wall")).expect("parses");
        assert!(matches!(
            at_from_probe(&pts[1]).expect("valid"),
            At::Outside { .. }
        ));
        assert!(matches!(
            at_from_probe(&pts[2]).expect("valid"),
            At::Inside { .. }
        ));
    }

    #[test]
    fn deep_solid_is_inside_and_names_its_part() {
        let pts = probed_points(&cluster("deep_solid")).expect("parses");
        match at_from_probe(&pts[0]).expect("valid") {
            At::Inside { contributing, .. } => assert_eq!(contributing, vec!["model".to_string()]),
            other => panic!("expected inside, got {other:?}"),
        }
    }

    #[test]
    fn a_point_in_the_hole_is_outside_the_solid() {
        let pts = probed_points(&cluster("in_the_hole")).expect("parses");
        assert!(matches!(
            at_from_probe(&pts[0]).expect("valid"),
            At::Outside { .. }
        ));
    }

    /// AN UNKNOWN STATE IS REFUSED. A fourth answer from the provider must not
    /// quietly become air, which would turn material into empty space.
    #[test]
    fn a_state_this_reader_does_not_know_is_refused() {
        let mut v = cluster("deep_solid");
        v["points"][0]["state"] = Value::String("partially_solid".into());
        let e = probed_points(&v).expect_err("must refuse");
        assert!(e.contains("does not know"), "{e}");
        assert!(e.contains("empty space"), "{e}");
    }

    /// AN EMPTY POINT LIST IS REFUSED: no points probed and no material found
    /// serialize identically.
    #[test]
    fn an_empty_point_list_is_refused() {
        let mut v = cluster("deep_solid");
        v["points"] = Value::Array(vec![]);
        assert!(probed_points(&v).is_err());
    }

    #[test]
    fn an_unsuccessful_answer_is_refused() {
        let mut v = cluster("deep_solid");
        v["success"] = Value::Bool(false);
        assert!(probed_points(&v).is_err());
    }

    /// Solid with no owning part is an attribution failure, same rule the
    /// judgment layer already enforces.
    #[test]
    fn solid_naming_no_part_is_refused() {
        let p = Probed {
            at: [0.0; 3],
            state: "solid".into(),
            parts: vec![],
        };
        assert!(at_from_probe(&p).is_err());
    }

    /// THE FEATURES LIST IS PRE-BOOLEAN, so a feature that ran and was then
    /// consumed still appears. Read with no surviving geometry, that is
    /// exactly `Consumed` -- the defect this crate exists to name.
    #[test]
    fn a_feature_in_the_tree_with_no_surviving_material_is_consumed() {
        let f = fixture()["features"].clone();
        let a = attribution_from_features(&f, "D6.00", None).expect("parses");
        assert_eq!(a.call_sites, 1);
        assert!(a.declared);
        match locate(&a).expect("valid") {
            Located::Consumed { call_sites, .. } => assert_eq!(call_sites, 1),
            other => panic!("pre-boolean presence plus no survivor is Consumed; got {other:?}"),
        }
    }

    /// ONE PATTERN, FOUR ENTRIES. A module called four times is four call
    /// sites, and "it ran four times and all four were eaten" is a different
    /// conversation from "it ran once".
    ///
    /// This fixture exists because the first one could not tell the two
    /// countings apart: it held a single pattern with a single entry, so
    /// counting patterns and counting entries both gave 1 and the mutant that
    /// counts patterns SURVIVED. A fixture has to contain the distinguishing
    /// case.
    #[test]
    fn a_feature_repeated_four_times_reports_four_call_sites() {
        let f = fixture()["features_repeated"].clone();
        let a = attribution_from_features(&f, "D4.00", None).expect("parses");
        assert_eq!(
            a.call_sites, 4,
            "four entries in one pattern, not one pattern"
        );
        match locate(&a).expect("valid") {
            Located::Consumed { call_sites, .. } => assert_eq!(call_sites, 4),
            other => panic!("expected Consumed with four sites, got {other:?}"),
        }
    }

    /// And with surviving material it is a location.
    #[test]
    fn the_same_feature_with_surviving_material_is_located() {
        let f = fixture()["features"].clone();
        let a = attribution_from_features(
            &f,
            "D6.00",
            Some((
                crate::bbox::Bbox::known([7.0, 7.0, -1.0], [13.0, 13.0, 11.0]),
                [10.0, 10.0, 5.0],
            )),
        )
        .expect("parses");
        assert!(matches!(locate(&a).expect("valid"), Located::At { .. }));
    }

    /// MASKED FEATURES ARE REFUSED. Inside hull() or minkowski() the provider
    /// cannot see, so an absence stops meaning absent.
    #[test]
    fn a_masked_feature_count_refuses_the_whole_reading() {
        let mut f = fixture()["features"].clone();
        f["counts"]["masked_hull"] = Value::from(2u64);
        let e = attribution_from_features(&f, "D6.00", None).expect_err("must refuse");
        assert!(
            e.contains("no longer means the feature is not there"),
            "{e}"
        );
    }

    /// As are unresolved ones, for the same reason.
    #[test]
    fn an_unresolved_feature_count_refuses_the_reading() {
        let mut f = fixture()["features"].clone();
        f["counts"]["unresolved"] = Value::from(1u64);
        assert!(attribution_from_features(&f, "D6.00", None).is_err());
    }

    /// A name nothing in the tree matches reports zero call sites and
    /// undeclared, which `locate` then REFUSES rather than calling absent.
    #[test]
    fn a_name_the_tree_does_not_carry_is_refused_by_locate() {
        let f = fixture()["features"].clone();
        let a = attribution_from_features(&f, "D99.00", None).expect("parses");
        assert_eq!(a.call_sites, 0);
        assert!(
            locate(&a).is_err(),
            "an undeclared name is a refusal, not an absence"
        );
    }
}
