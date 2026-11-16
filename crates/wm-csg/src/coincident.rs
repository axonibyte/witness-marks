//! Positions that land exactly on another feature's boundary.
//!
//! # One comparison, two incidents
//!
//! **buildability 66.** The dovetail sockets sat at `horn_deg = mouth_deg / 2`
//! = 34 degrees and the mouth cut spanned exactly +/-34. Both features
//! correct; the cut sheared a flank off both sockets.
//!
//! **buildability 71.** The boss's ramp foot was a 14 mm wide slab reaching to
//! `body_r`. Reaching EXACTLY the wall radius is the coincidence. The
//! consequence is geometric: a rectangle laid against a cylinder overhangs at
//! its CORNERS, which stood 0.63 mm outside the wall -- four thin crescents of
//! unsupported flat bottom that no amount of looking at the ramp's 45 degrees
//! would have found, because the ramp was correct. It was the footprint that
//! was the wrong shape.
//!
//! The comparison is the same in both: a value that equals a boundary. What
//! differs is what goes wrong next, and that is not this module's to decide.
//!
//! # It reports and never judges
//!
//! Two features deliberately flush with each other are a design, not a defect.
//! A coincidence is a FLAG: this position is not independent of that boundary,
//! so a change to one moves the other. Whether that is wanted is a question
//! for somebody who knows what the features are.

use crate::bbox::{bbox_at, Bbox};
use crate::derive::{derived_from, Bindings, Derivation};
use crate::tree::Node;

/// One face of one node's box, in the root's frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Boundary {
    pub path: Vec<usize>,
    pub op: String,
    /// 0, 1, 2 for x, y, z.
    pub axis: usize,
    /// `true` for the low face.
    pub low: bool,
    pub value: f64,
}

impl Boundary {
    pub fn name(&self) -> String {
        format!(
            "{} at {:?}, {} {}",
            self.op,
            self.path,
            if self.low { "min" } else { "max" },
            ["x", "y", "z"][self.axis]
        )
    }
}

/// Two boundaries that land on the same value.
#[derive(Debug, Clone, PartialEq)]
pub struct Coincidence {
    pub a: Boundary,
    pub b: Boundary,
    pub value: f64,
    /// How the shared value can be explained from the bindings. A value with
    /// a derivation is the dangerous kind: it is not a number somebody typed
    /// twice, it is one parameter reaching two places.
    pub derivations: Vec<Derivation>,
}

/// Every bounded node's box faces, in the root's frame.
pub fn boundaries(root: &Node) -> Vec<Boundary> {
    fn go(root: &Node, n: &Node, path: Vec<usize>, out: &mut Vec<Boundary>) {
        if let Some(Bbox::Known { lo, hi, .. }) = bbox_at(root, &path) {
            for axis in 0..3 {
                out.push(Boundary {
                    path: path.clone(),
                    op: n.op.clone(),
                    axis,
                    low: true,
                    value: lo[axis],
                });
                out.push(Boundary {
                    path: path.clone(),
                    op: n.op.clone(),
                    axis,
                    low: false,
                    value: hi[axis],
                });
            }
        }
        for (i, c) in n.children.iter().enumerate() {
            let mut p = path.clone();
            p.push(i);
            go(root, c, p, out);
        }
    }
    let mut out = Vec::new();
    go(root, root, Vec::new(), &mut out);
    out
}

/// Pairs of boundaries on the same axis that share a value.
///
/// # Ancestors are skipped, and that is the whole of the noise control
///
/// A node's box is contained in its parent's, so a child flush with its
/// parent's face coincides with it by construction -- a cube inside a group
/// IS that group's extent. Reporting those would bury the real finding under
/// one entry per level of nesting. Only boundaries from nodes where NEITHER
/// path is a prefix of the other are compared.
pub fn coincidences(root: &Node, b: &Bindings, tol: f64) -> Vec<Coincidence> {
    let bs = boundaries(root);
    let mut out = Vec::new();
    for (i, x) in bs.iter().enumerate() {
        for y in bs.iter().skip(i + 1) {
            if x.axis != y.axis {
                continue;
            }
            if x.path.starts_with(&y.path) || y.path.starts_with(&x.path) {
                continue;
            }
            if !(x.value - y.value).abs().le(&tol) {
                continue;
            }
            out.push(Coincidence {
                a: x.clone(),
                b: y.clone(),
                value: x.value,
                derivations: derived_from(x.value, b, tol),
            });
        }
    }
    out
}

/// How far the CORNERS of a rectangle stand outside a circle its edge touches.
///
/// buildability 71, as arithmetic. A slab `2*half_width` wide reaching to
/// `radius` has corners at `hypot(radius, half_width)`, so pulling the edge
/// flush with a round wall leaves the corners proud of it.
///
/// REFUSES A NON-POSITIVE RADIUS rather than returning a number: the question
/// "how far outside the circle" has no answer without a circle.
pub fn corner_excess(radius: f64, half_width: f64) -> Result<f64, String> {
    if !radius.is_finite() || radius <= 0.0 || !half_width.is_finite() || half_width < 0.0 {
        return Err(format!(
            "need a positive finite radius and a non-negative half-width, got \
             {radius} and {half_width}"
        ));
    }
    Ok(radius.hypot(half_width) - radius)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::parse;

    /// Two separate features whose faces land on the same x. Neither is an
    /// ancestor of the other.
    const FLUSH: &str = "union() {\n\
        \tcube(size = [10, 2, 2]);\n\
        \tmultmatrix([[1, 0, 0, 10], [0, 1, 0, 5], [0, 0, 1, 0], [0, 0, 0, 1]]) {\n\
        \t\tcube(size = [3, 2, 2]);\n\
        \t}\n\
        }\n";

    #[test]
    fn two_features_flush_on_an_axis_are_reported() {
        let t = parse(FLUSH).expect("parses");
        let c = coincidences(&t, &Bindings::new(), 1e-9);
        assert!(
            c.iter()
                .any(|x| (x.value - 10.0).abs() < 1e-9 && x.a.axis == 0),
            "the first cube's max x meets the second's min x: {:?}",
            c.iter().map(|x| x.value).collect::<Vec<_>>()
        );
    }

    /// ANCESTORS ARE SKIPPED. A child flush with its parent coincides by
    /// construction, and reporting it would bury the real finding under one
    /// entry per level of nesting.
    #[test]
    fn a_child_flush_with_its_parent_is_not_a_finding() {
        let t = parse("group() {\n\tcube(size = [5, 5, 5]);\n}\n").expect("parses");
        assert!(
            coincidences(&t, &Bindings::new(), 1e-9).is_empty(),
            "a lone cube IS its group's extent; that is not a coincidence"
        );
    }

    /// A SHARED VALUE WITH A DERIVATION is the dangerous kind: not a number
    /// typed twice, but one parameter reaching two places.
    #[test]
    fn a_coincidence_carries_how_the_value_could_be_derived() {
        let t = parse(FLUSH).expect("parses");
        let b = Bindings::from([("span".to_string(), 20.0), ("reach".to_string(), 10.0)]);
        let c = coincidences(&t, &b, 1e-9);
        let at10 = c
            .iter()
            .find(|x| (x.value - 10.0).abs() < 1e-9)
            .expect("the flush pair");
        let e: Vec<&str> = at10.derivations.iter().map(|d| d.expr.as_str()).collect();
        assert!(e.contains(&"reach"), "{e:?}");
        assert!(
            e.contains(&"span / 2"),
            "a value that is ALSO half another parameter: {e:?}"
        );
    }

    /// Features that are merely near each other are not flush.
    #[test]
    fn a_near_miss_is_not_a_coincidence_at_a_tight_tolerance() {
        let t = parse(
            "union() {\n\
             \tcube(size = [10, 2, 2]);\n\
             \tmultmatrix([[1, 0, 0, 10.5], [0, 1, 0, 5], [0, 0, 1, 0], [0, 0, 0, 1]]) {\n\
             \t\tcube(size = [3, 2, 2]);\n\
             \t}\n\
             }\n",
        )
        .expect("parses");
        assert!(!coincidences(&t, &Bindings::new(), 1e-9)
            .iter()
            .any(|x| (x.value - 10.0).abs() < 1e-9 && x.a.axis == 0));
        // and at a tolerance that admits it, it IS reported
        assert!(!coincidences(&t, &Bindings::new(), 1.0).is_empty());
    }

    /// A VALUE SHARED ACROSS DIFFERENT AXES IS NOT A COINCIDENCE, and this
    /// arm exists because a mutant proved nothing covered it. Removing the
    /// axis check left every other arm green, since in all of them the only
    /// matching values happened to be on the same axis anyway.
    ///
    /// Here the first cube's max X is 10 and the second's max Y is 10, and
    /// they have nothing to do with each other: one is a length, the other is
    /// a width, and a tool that called them flush would report a finding
    /// about two numbers that merely print the same.
    #[test]
    fn a_value_shared_across_two_different_axes_is_not_a_coincidence() {
        let t = parse(
            "union() {\n\
             \tcube(size = [10, 2, 2]);\n\
             \tmultmatrix([[1, 0, 0, 20], [0, 1, 0, 5], [0, 0, 1, 1], [0, 0, 0, 1]]) {\n\
             \t\tcube(size = [2, 5, 3]);\n\
             \t}\n\
             }\n",
        )
        .expect("parses");
        // the first cube reaches x = 10; the second reaches y = 5 + 5 = 10
        let c = coincidences(&t, &Bindings::new(), 1e-9);
        assert!(
            c.is_empty(),
            "nothing is flush on a SHARED axis here; got {:?}",
            c.iter()
                .map(|x| (x.a.name(), x.b.name()))
                .collect::<Vec<_>>()
        );
    }

    /// BUILDABILITY 71 AS ARITHMETIC. A 14 mm wide slab reaching to
    /// body_r = 38.6 has corners at hypot(38.6, 7) = 39.2296, standing
    /// 0.6296 mm outside the wall. The log says "up to 0.65".
    #[test]
    fn a_rectangle_flush_with_a_cylinder_overhangs_at_its_corners() {
        let e = corner_excess(38.6, 7.0).expect("valid");
        assert!(
            (e - 0.629_6).abs() < 1e-3,
            "the corners stand 0.63 mm proud, got {e}"
        );
        assert!(e > 0.0, "a rectangle on a circle ALWAYS overhangs");
    }

    /// A WIDER SLAB OVERHANGS MORE, and a zero-width one not at all, which is
    /// the sanity the formula has to keep.
    #[test]
    fn the_overhang_grows_with_the_width_and_vanishes_at_zero() {
        assert_eq!(corner_excess(38.6, 0.0).expect("valid"), 0.0);
        let narrow = corner_excess(38.6, 3.0).expect("valid");
        let wide = corner_excess(38.6, 7.0).expect("valid");
        assert!(wide > narrow && narrow > 0.0);
    }

    /// AND THE FIX HAS ITS OWN TRAP, which the log records: pulling the foot
    /// in to sqrt(r^2 - hw^2) costs 0.64 mm of radial run, which lengthens a
    /// 45-degree ramp without changing its rise and flattens it. The arm pins
    /// the magnitude so the second half of the fix is not forgotten.
    #[test]
    fn pulling_the_foot_in_costs_radial_run() {
        let (r, hw) = (38.6f64, 7.0f64);
        let pulled = (r * r - hw * hw).sqrt();
        assert!(
            (r - pulled - 0.64).abs() < 0.01,
            "0.64 mm of run lost, got {}",
            r - pulled
        );
    }

    #[test]
    fn corner_excess_refuses_a_circle_that_is_not_one() {
        assert!(corner_excess(0.0, 7.0).is_err());
        assert!(corner_excess(-1.0, 7.0).is_err());
        assert!(corner_excess(f64::NAN, 7.0).is_err());
        assert!(corner_excess(38.6, -1.0).is_err());
    }
}
