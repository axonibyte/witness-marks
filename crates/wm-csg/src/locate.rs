//! `solid.locate` and `solid.at`: how an agent orients in a file it did not
//! write, and the three empty answers that are not the same empty answer.
//!
//! # Three ways to find nothing
//!
//! Ask where a module's geometry ended up and an empty result can mean:
//!
//! 1. **No such module.** The name is a typo, or belongs to a library that
//!    did not resolve. (A missing `include` is a WARNING in OpenSCAD and the
//!    render proceeds without it, so this is routine, not exotic.)
//! 2. **Declared and never called.** The module exists in the file and
//!    nothing instantiates it. Its geometry is absent because it was never
//!    asked for.
//! 3. **Called, and then consumed.** It contributed geometry and a later
//!    difference removed all of it. This is the defect: the module ran, the
//!    author believes its feature is in the part, and the part has none of it.
//!
//! Collapsing those to one empty bbox is the same error as the one Layer 1
//! already names about `operand_volumes` -- *"an operand that rendered nothing
//! is the difference between 'these do not touch' and 'I measured
//! nothing.'"* -- one level over, in attribution instead of interference.
//!
//! # And on the boundary
//!
//! `solid.at(point)` on a point lying exactly on a face is a coin flip
//! between inside and outside, and clearance work lives at exactly those
//! points: the whole question of a 0.2 mm fit is what happens at the surface.
//! So the boundary is its own answer, with the tolerance it was judged at
//! carried along, because "on the face" is only meaningful relative to one.

use crate::bbox::Bbox;

/// Where a module's contribution ended up, or why there is nothing to report.
#[derive(Debug, Clone, PartialEq)]
pub enum Located {
    /// It contributed geometry that survived into the result.
    At { bbox: Bbox, centroid: [f64; 3] },
    /// Declared in the file, instantiated nowhere. Nothing ran.
    NeverInstantiated,
    /// It ran and contributed geometry, and a later operation removed all of
    /// it. The count of call sites is carried because "it ran three times and
    /// all three were eaten" is a different conversation from "it ran once".
    Consumed { call_sites: usize, by: Vec<String> },
}

/// Whether a point is in the solid.
#[derive(Debug, Clone, PartialEq)]
pub enum At {
    Inside {
        /// Which modules' geometry contains this point. An inside point with
        /// no contributing module is a contradiction -- something built that
        /// solid -- and [`at`] refuses rather than reports it.
        contributing: Vec<String>,
        /// Distance to the nearest face, positive going out.
        nearest_face_mm: f64,
    },
    Outside {
        nearest_face_mm: f64,
    },
    /// Within `tol_mm` of a face. Not a rounding of inside or outside: a
    /// clearance question is asked precisely here, and answering it with
    /// either neighbor is a coin flip the caller cannot see.
    OnBoundary {
        tol_mm: f64,
        contributing: Vec<String>,
    },
}

/// What a lookup of one module name found in the model.
///
/// This is the input `locate` reasons over, kept separate from its verdict so
/// that a caller cannot construct a "found nothing" without saying which kind
/// of nothing it is.
#[derive(Debug, Clone)]
pub struct Attribution {
    pub module: String,
    /// Is the name declared in the file at all?
    pub declared: bool,
    /// How many places instantiate it.
    pub call_sites: usize,
    /// The box of what survived, if any survived.
    pub surviving: Option<(Bbox, [f64; 3])>,
    /// Operations that removed its material, named.
    pub eaten_by: Vec<String>,
}

/// Report where a module's geometry is, distinguishing the three nothings.
///
/// REFUSES AN UNDECLARED NAME. A typo'd module name is not "that module
/// contributes nothing" -- it is a question about something that does not
/// exist, and answering it with an absence lets the typo stand while reading
/// as a measurement.
pub fn locate(a: &Attribution) -> Result<Located, String> {
    if !a.declared {
        return Err(format!(
            "no module named `{}` is declared in this model. That is not the same as a \
             module contributing nothing: answering an undeclared name with an absence \
             lets a typo -- or a library whose `include` did not resolve, which is only \
             a WARNING -- read as a measurement",
            a.module
        ));
    }
    if a.call_sites == 0 {
        // A DECLARED, UNCALLED MODULE CANNOT HAVE BEEN EATEN. If something
        // claims to have removed its material, the attribution itself is
        // inconsistent and saying so beats picking one half to believe.
        if !a.eaten_by.is_empty() {
            return Err(format!(
                "`{}` is declared, instantiated nowhere, and yet {} operation(s) are \
                 recorded as removing its material. One of those two readings is wrong",
                a.module,
                a.eaten_by.len()
            ));
        }
        if a.surviving.is_some() {
            return Err(format!(
                "`{}` is instantiated nowhere and yet has surviving geometry. One of \
                 those two readings is wrong",
                a.module
            ));
        }
        return Ok(Located::NeverInstantiated);
    }
    match &a.surviving {
        Some((bbox, centroid)) => Ok(Located::At {
            bbox: bbox.clone(),
            centroid: *centroid,
        }),
        None => Ok(Located::Consumed {
            call_sites: a.call_sites,
            by: a.eaten_by.clone(),
        }),
    }
}

/// Classify a point against a solid.
///
/// `signed_mm` is the signed distance to the nearest face, negative inside.
/// `tol_mm` is the band within which a point counts as on the boundary; it
/// must be positive, because a zero-width band makes `OnBoundary`
/// unreachable and turns every clearance question back into the coin flip.
///
/// REFUSES AN INSIDE POINT WITH NO CONTRIBUTING MODULE. Something built that
/// solid. An empty attribution there means the attribution failed, not that
/// the solid is unowned, and reporting `Inside` with an empty list hands the
/// caller a fact and loses the failure.
pub fn at(signed_mm: f64, tol_mm: f64, contributing: Vec<String>) -> Result<At, String> {
    if !signed_mm.is_finite() {
        return Err(format!(
            "the signed distance is {signed_mm}, which is not a distance"
        ));
    }
    if !tol_mm.is_finite() || tol_mm <= 0.0 {
        return Err(format!(
            "the boundary tolerance is {tol_mm}; it must be positive, or OnBoundary is \
             unreachable and a point exactly on a face silently becomes inside or \
             outside -- a coin flip the caller cannot see"
        ));
    }
    if signed_mm.abs() <= tol_mm {
        return Ok(At::OnBoundary {
            tol_mm,
            contributing,
        });
    }
    if signed_mm < 0.0 {
        if contributing.is_empty() {
            return Err(
                "a point inside the solid with no contributing module: something built \
                 that solid, so an empty attribution is an attribution failure, not a \
                 finding about the model"
                    .into(),
            );
        }
        return Ok(At::Inside {
            contributing,
            nearest_face_mm: -signed_mm,
        });
    }
    Ok(At::Outside {
        nearest_face_mm: signed_mm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attr(module: &str) -> Attribution {
        Attribution {
            module: module.into(),
            declared: true,
            call_sites: 1,
            surviving: None,
            eaten_by: Vec::new(),
        }
    }

    /// THE DEFECT. The module ran, contributed geometry, and a later
    /// difference removed all of it. The author believes its feature is in
    /// the part.
    #[test]
    fn a_module_whose_geometry_was_entirely_eaten_is_not_a_module_that_never_ran() {
        let a = Attribution {
            call_sites: 2,
            eaten_by: vec!["mouth_cut".into()],
            ..attr("dovetail_socket")
        };
        assert_eq!(
            locate(&a).expect("valid"),
            Located::Consumed {
                call_sites: 2,
                by: vec!["mouth_cut".into()]
            },
            "two call sites, both eaten by the mouth cut -- not NeverInstantiated"
        );
    }

    /// And the other nothing: declared, never called. Nothing ran, so nothing
    /// ate anything.
    #[test]
    fn a_declared_but_uncalled_module_is_its_own_answer() {
        let a = Attribution {
            call_sites: 0,
            ..attr("spare_boss")
        };
        assert_eq!(locate(&a).expect("valid"), Located::NeverInstantiated);
    }

    /// THE THIRD NOTHING IS REFUSED. A typo'd name is a question about
    /// something that does not exist; answering with an absence lets the typo
    /// stand while reading as a measurement.
    #[test]
    fn an_undeclared_module_name_is_refused_not_reported_empty() {
        let a = Attribution {
            declared: false,
            call_sites: 0,
            ..attr("dovetail_sockt")
        };
        let e = locate(&a).expect_err("must refuse");
        assert!(e.contains("read as a measurement"), "{e}");
        assert!(e.contains("dovetail_sockt"), "{e}");
    }

    /// An inconsistent attribution is called out rather than half-believed:
    /// a module instantiated nowhere cannot have had material removed.
    #[test]
    fn an_uncalled_module_that_something_claims_to_have_eaten_is_refused() {
        let a = Attribution {
            call_sites: 0,
            eaten_by: vec!["mouth_cut".into()],
            ..attr("dovetail_socket")
        };
        assert!(locate(&a).is_err());
    }

    /// Nor can it have surviving geometry.
    #[test]
    fn an_uncalled_module_with_surviving_geometry_is_refused() {
        let a = Attribution {
            call_sites: 0,
            surviving: Some((Bbox::known([0.0; 3], [1.0; 3]), [0.5; 3])),
            ..attr("dovetail_socket")
        };
        assert!(locate(&a).is_err());
    }

    /// The ordinary case: it ran and its geometry is there.
    #[test]
    fn a_module_that_contributed_reports_its_box_and_centroid() {
        let a = Attribution {
            surviving: Some((
                Bbox::known([0.0, 0.0, 0.0], [4.5, 12.0, 3.0]),
                [2.25, 6.0, 1.5],
            )),
            ..attr("stub_tenon")
        };
        match locate(&a).expect("valid") {
            Located::At { centroid, .. } => assert_eq!(centroid, [2.25, 6.0, 1.5]),
            other => panic!("expected a location, got {other:?}"),
        }
    }

    /// A POINT ON A FACE IS ITS OWN ANSWER, not a rounding of its neighbors.
    /// Clearance work is asked precisely here.
    #[test]
    fn a_point_exactly_on_a_face_is_on_the_boundary_not_inside_or_outside() {
        match at(0.0, 0.01, vec!["yoke".into()]).expect("valid") {
            At::OnBoundary { tol_mm, .. } => assert_eq!(tol_mm, 0.01),
            other => panic!("0.0 is on the face; got {other:?}"),
        }
    }

    /// The tolerance travels with the answer, because "on the face" has no
    /// meaning without one.
    #[test]
    fn the_same_point_is_on_the_boundary_at_one_tolerance_and_inside_at_another() {
        let hair = -0.005;
        assert!(matches!(
            at(hair, 0.01, vec!["yoke".into()]).expect("valid"),
            At::OnBoundary { .. }
        ));
        assert!(matches!(
            at(hair, 0.001, vec!["yoke".into()]).expect("valid"),
            At::Inside { .. }
        ));
    }

    /// A ZERO TOLERANCE IS REFUSED: it makes OnBoundary unreachable, and a
    /// point exactly on a face silently becomes inside or outside.
    #[test]
    fn a_zero_boundary_tolerance_is_refused() {
        let e = at(0.0, 0.0, vec!["yoke".into()]).expect_err("must refuse");
        assert!(e.contains("coin flip"), "{e}");
        assert!(at(0.0, -0.01, vec!["yoke".into()]).is_err());
    }

    /// AN INSIDE POINT WITH NO OWNER IS AN ATTRIBUTION FAILURE. Something
    /// built that solid.
    #[test]
    fn an_inside_point_with_no_contributing_module_is_refused() {
        let e = at(-1.0, 0.01, Vec::new()).expect_err("must refuse");
        assert!(e.contains("attribution failure"), "{e}");
    }

    /// An outside point needs no owner -- nothing is there to own.
    #[test]
    fn an_outside_point_with_no_contributing_module_is_fine() {
        match at(1.0, 0.01, Vec::new()).expect("valid") {
            At::Outside { nearest_face_mm } => assert_eq!(nearest_face_mm, 1.0),
            other => panic!("expected outside, got {other:?}"),
        }
    }

    /// Distance to the nearest face is reported positive from both sides, so
    /// "how far from the surface" does not need a sign convention explained
    /// at every call site.
    #[test]
    fn the_distance_to_the_nearest_face_is_positive_from_either_side() {
        let inside = at(-0.5, 0.01, vec!["yoke".into()]).expect("valid");
        let outside = at(0.5, 0.01, Vec::new()).expect("valid");
        assert_eq!(
            inside,
            At::Inside {
                contributing: vec!["yoke".into()],
                nearest_face_mm: 0.5
            }
        );
        assert_eq!(
            outside,
            At::Outside {
                nearest_face_mm: 0.5
            }
        );
    }

    #[test]
    fn a_non_finite_distance_is_refused() {
        assert!(at(f64::NAN, 0.01, vec!["yoke".into()]).is_err());
    }
}
