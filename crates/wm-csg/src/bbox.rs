//! Per-node bounding boxes over the CSG tree.
//!
//! # Unknown is a value
//!
//! Most nodes in a real dump are things this cannot bound exactly: a
//! `rotate_extrude` of an arbitrary polygon, a `hull` of several children, a
//! `minkowski`. Returning a guess for those would be worse than useless,
//! because a bbox is used to decide whether two features can possibly touch
//! and a wrong one answers that question confidently.
//!
//! So [`Bbox::Unknown`] carries the REASON, and a caller that needs certainty
//! can see exactly which node defeated it.
//!
//! # Every known answer is a BOUND, and says which kind
//!
//! `difference` returns its first child's box: the result is a subset, so the
//! box is an over-estimate and never an under-estimate. `hull` returns the
//! union of its children's boxes for the same reason. An over-estimate is the
//! safe direction for "can these touch" and the wrong direction for "do these
//! definitely touch", which is why [`Bbox::Known`] records that it is an
//! upper bound rather than pretending to be exact.

use crate::tree::Node;

/// A bounding box, or an honest absence of one.
#[derive(Debug, Clone, PartialEq)]
pub enum Bbox {
    /// `[lo, hi]`. `exact` is false when the box is an upper bound that the
    /// geometry may not fill.
    Known {
        lo: [f64; 3],
        hi: [f64; 3],
        exact: bool,
    },
    /// This node has no geometry at all: an empty group, a childless boolean.
    Empty,
    /// It could not be bounded, and this is why.
    Unknown(String),
}

impl Bbox {
    pub fn known(lo: [f64; 3], hi: [f64; 3]) -> Self {
        Bbox::Known {
            lo,
            hi,
            exact: true,
        }
    }

    fn loosen(self) -> Self {
        match self {
            Bbox::Known { lo, hi, .. } => Bbox::Known {
                lo,
                hi,
                exact: false,
            },
            other => other,
        }
    }

    pub fn is_known(&self) -> bool {
        matches!(self, Bbox::Known { .. })
    }

    /// The union of two boxes. Unknown POISONS a union, because a union is at
    /// least as big as either part and an unbounded part makes the whole
    /// unbounded.
    pub fn union(self, other: Bbox) -> Bbox {
        match (self, other) {
            (Bbox::Unknown(w), _) | (_, Bbox::Unknown(w)) => Bbox::Unknown(w),
            (Bbox::Empty, b) | (b, Bbox::Empty) => b,
            (
                Bbox::Known {
                    lo: a,
                    hi: b,
                    exact: e1,
                },
                Bbox::Known {
                    lo: c,
                    hi: d,
                    exact: e2,
                },
            ) => Bbox::Known {
                lo: [a[0].min(c[0]), a[1].min(c[1]), a[2].min(c[2])],
                hi: [b[0].max(d[0]), b[1].max(d[1]), b[2].max(d[2])],
                exact: e1 && e2,
            },
        }
    }

    /// The intersection of two boxes.
    ///
    /// UNKNOWN DOES NOT POISON AN INTERSECTION the way it poisons a union: an
    /// intersection is no bigger than EITHER part, so a known box on one side
    /// is still a valid upper bound for the whole. It stops being exact.
    pub fn intersect(self, other: Bbox) -> Bbox {
        match (self, other) {
            (Bbox::Empty, _) | (_, Bbox::Empty) => Bbox::Empty,
            (Bbox::Unknown(_), b @ Bbox::Known { .. }) => b.loosen(),
            (a @ Bbox::Known { .. }, Bbox::Unknown(_)) => a.loosen(),
            (Bbox::Unknown(w), Bbox::Unknown(_)) => Bbox::Unknown(w),
            (
                Bbox::Known {
                    lo: a,
                    hi: b,
                    exact: e1,
                },
                Bbox::Known {
                    lo: c,
                    hi: d,
                    exact: e2,
                },
            ) => {
                let lo = [a[0].max(c[0]), a[1].max(c[1]), a[2].max(c[2])];
                let hi = [b[0].min(d[0]), b[1].min(d[1]), b[2].min(d[2])];
                if (0..3).any(|i| lo[i] > hi[i]) {
                    Bbox::Empty
                } else {
                    Bbox::Known {
                        lo,
                        hi,
                        exact: e1 && e2,
                    }
                }
            }
        }
    }
}

fn vec3(raw: &str) -> Option<[f64; 3]> {
    let inner = raw.trim().strip_prefix('[')?.strip_suffix(']')?;
    let parts: Vec<f64> = inner
        .split(',')
        .filter_map(|p| p.trim().parse::<f64>().ok())
        .collect();
    match parts.len() {
        3 => Some([parts[0], parts[1], parts[2]]),
        _ => None,
    }
}

/// Parse a multmatrix's 4x4 into rows.
fn matrix(raw: &str) -> Option<[[f64; 4]; 4]> {
    let inner = raw.trim().strip_prefix('[')?.strip_suffix(']')?;
    let mut rows = [[0.0f64; 4]; 4];
    let mut r = 0;
    let mut depth = 0;
    let mut start = 0usize;
    let b = inner.as_bytes();
    for i in 0..b.len() {
        match b[i] {
            b'[' => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            b']' => {
                depth -= 1;
                if depth == 0 {
                    if r >= 4 {
                        return None;
                    }
                    let nums: Vec<f64> = inner[start + 1..i]
                        .split(',')
                        .filter_map(|p| p.trim().parse::<f64>().ok())
                        .collect();
                    if nums.len() != 4 {
                        return None;
                    }
                    rows[r] = [nums[0], nums[1], nums[2], nums[3]];
                    r += 1;
                }
            }
            _ => {}
        }
    }
    if r == 4 {
        Some(rows)
    } else {
        None
    }
}

/// Transform a box by a 4x4 and take the axis-aligned bound of the result.
///
/// ALL EIGHT CORNERS, NOT TWO. Transforming only `lo` and `hi` is correct for
/// a pure translation and wrong for every rotation: a box turned 45 degrees
/// has corners outside the box its two extreme corners describe. The result
/// is an upper bound under rotation, which is marked.
fn transform(m: [[f64; 4]; 4], b: Bbox) -> Bbox {
    let Bbox::Known { lo, hi, exact } = b else {
        return b;
    };
    let mut nlo = [f64::INFINITY; 3];
    let mut nhi = [f64::NEG_INFINITY; 3];
    for i in 0..8 {
        let p = [
            if i & 1 == 0 { lo[0] } else { hi[0] },
            if i & 2 == 0 { lo[1] } else { hi[1] },
            if i & 4 == 0 { lo[2] } else { hi[2] },
        ];
        for (k, row) in m.iter().take(3).enumerate() {
            let v = row[0] * p[0] + row[1] * p[1] + row[2] * p[2] + row[3];
            nlo[k] = nlo[k].min(v);
            nhi[k] = nhi[k].max(v);
        }
    }
    // A pure translation (no rotation or shear) keeps the box exact.
    let axis_aligned = (0..3).all(|r| (0..3).filter(|c| *c != r).all(|c| m[r][c].abs() < 1e-12));
    Bbox::Known {
        lo: nlo,
        hi: nhi,
        exact: exact && axis_aligned,
    }
}

/// The bounding box of a node and everything under it.
pub fn bbox(n: &Node) -> Bbox {
    match n.op.as_str() {
        "cube" => match n.args.get("size").and_then(|s| vec3(s)) {
            None => Bbox::Unknown("cube with no readable size".into()),
            Some(s) => {
                let centered = n.args.get("center").map(String::as_str) == Some("true");
                if centered {
                    Bbox::known(
                        [-s[0] / 2.0, -s[1] / 2.0, -s[2] / 2.0],
                        [s[0] / 2.0, s[1] / 2.0, s[2] / 2.0],
                    )
                } else {
                    Bbox::known([0.0, 0.0, 0.0], s)
                }
            }
        },
        "cylinder" => {
            let (h, r1, r2) = (n.num("h"), n.num("r1"), n.num("r2"));
            match (h, r1, r2) {
                (Some(h), Some(r1), Some(r2)) => {
                    let r = r1.max(r2);
                    let centered = n.args.get("center").map(String::as_str) == Some("true");
                    let (z0, z1) = if centered {
                        (-h / 2.0, h / 2.0)
                    } else {
                        (0.0, h)
                    };
                    // AN UPPER BOUND, NOT EXACT: a tessellated cylinder is a
                    // prism inscribed in the radius, so its true box is
                    // slightly smaller and depends on $fn and the start angle.
                    Bbox::Known {
                        lo: [-r, -r, z0],
                        hi: [r, r, z1],
                        exact: false,
                    }
                }
                _ => Bbox::Unknown("cylinder missing h, r1 or r2".into()),
            }
        }
        "sphere" => match n.num("r") {
            Some(r) => Bbox::Known {
                lo: [-r, -r, -r],
                hi: [r, r, r],
                exact: false,
            },
            None => Bbox::Unknown("sphere with no readable r".into()),
        },
        "multmatrix" => match n.args.get("0").and_then(|s| matrix(s)) {
            None => Bbox::Unknown("multmatrix with no readable matrix".into()),
            Some(m) => transform(m, children_union(n)),
        },
        "union" | "group" | "hull" | "minkowski" => {
            let u = children_union(n);
            // A HULL IS NOT ITS CHILDREN'S UNION, it CONTAINS it, so the box
            // is still a valid upper bound and is marked inexact.
            if n.op == "hull" || n.op == "minkowski" {
                u.loosen()
            } else {
                u
            }
        }
        "intersection" => n
            .children
            .iter()
            .map(bbox)
            .reduce(Bbox::intersect)
            .unwrap_or(Bbox::Empty),
        "difference" => match n.children.first() {
            // THE FIRST CHILD'S BOX IS AN UPPER BOUND: subtracting can only
            // remove material, never extend it.
            Some(first) => bbox(first).loosen(),
            None => Bbox::Empty,
        },
        other => Bbox::Unknown(format!(
            "{other} is not a node this bounds; its extent depends on geometry \
             this does not evaluate"
        )),
    }
}

/// The box of the node at `path`, IN THE ROOT'S FRAME.
///
/// # Why this exists separately from [`bbox`]
///
/// `bbox(node)` is the node's own extent in its OWN frame, with its ancestors'
/// transforms not applied -- it cannot know them. Comparing two nodes' boxes
/// taken that way is comparing measurements in two different coordinate
/// systems, and it produces a confident wrong answer: a socket translated into
/// the path of a cut reads as nowhere near it.
///
/// That is not hypothetical. `eats` was written against `bbox` and its first
/// run reported that the cut which sheared the dovetail sockets did not reach
/// them, because the socket's own box sat at the origin while the cut's box
/// included its translation.
///
/// Returns `None` when the path names no node.
pub fn bbox_at(root: &Node, path: &[usize]) -> Option<Bbox> {
    fn go(n: &Node, path: &[usize], acc: Option<[[f64; 4]; 4]>) -> Option<Bbox> {
        // THE DESTINATION IS HANDLED BEFORE ANY ACCUMULATION, and that order
        // is the whole correctness of this function. `bbox` ALREADY applies a
        // multmatrix's own matrix to its children, so accumulating it here as
        // well and then applying `acc` would transform the final node twice.
        // The first version did exactly that and placed the cut at double its
        // translation, which read as "nothing reaches the socket".
        if path.is_empty() {
            return Some(match acc {
                None => bbox(n),
                Some(m) => transform(m, bbox(n)),
            });
        }
        // Descending THROUGH a multmatrix: accumulate it, because `bbox` of
        // the child below knows nothing about it.
        let acc = if n.op == "multmatrix" {
            match n.args.get("0").and_then(|s| matrix(s)) {
                Some(m) => Some(match acc {
                    None => m,
                    Some(a) => mul(a, m),
                }),
                // AN UNREADABLE TRANSFORM POISONS EVERYTHING BELOW IT. The
                // alternative is placing a child in the wrong frame and saying
                // nothing, which is the failure this function was written for.
                None => {
                    return Some(Bbox::Unknown(
                        "a multmatrix on the path has no readable matrix, so nothing below it \
                     can be placed"
                            .into(),
                    ))
                }
            }
        } else {
            acc
        };
        let (i, rest) = path.split_first()?;
        go(n.children.get(*i)?, rest, acc)
    }
    go(root, path, None)
}

fn mul(a: [[f64; 4]; 4], b: [[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut o = [[0.0f64; 4]; 4];
    for (i, row) in o.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    o
}

fn children_union(n: &Node) -> Bbox {
    n.children
        .iter()
        .map(bbox)
        .reduce(Bbox::union)
        .unwrap_or(Bbox::Empty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::parse;

    const STAGE: &str = include_str!("../../../tests/fixtures/can_cutter/stage.csg");

    fn b(dump: &str) -> Bbox {
        bbox(&parse(dump).expect("parses"))
    }

    #[test]
    fn a_cube_is_exact_and_knows_about_center() {
        assert_eq!(
            b("cube(size = [10, 20, 30], center = false);"),
            Bbox::known([0.0, 0.0, 0.0], [10.0, 20.0, 30.0])
        );
        assert_eq!(
            b("cube(size = [10, 20, 30], center = true);"),
            Bbox::known([-5.0, -10.0, -15.0], [5.0, 10.0, 15.0])
        );
    }

    /// A TESSELLATED CYLINDER IS INSCRIBED, so its true box is slightly
    /// smaller than the radius and depends on $fn. The bound is correct and
    /// is marked inexact rather than claimed as the answer.
    #[test]
    fn a_cylinder_is_an_upper_bound_and_says_so() {
        match b("cylinder($fn = 16, h = 12, r1 = 2, r2 = 3, center = false);") {
            Bbox::Known { lo, hi, exact } => {
                assert_eq!(lo, [-3.0, -3.0, 0.0], "the LARGER radius bounds it");
                assert_eq!(hi, [3.0, 3.0, 12.0]);
                assert!(!exact, "an inscribed prism does not fill its radius");
            }
            other => panic!("expected a box, got {other:?}"),
        }
    }

    /// A translation keeps the box exact; a ROTATION does not, and the box
    /// must be taken over all eight corners.
    #[test]
    fn a_rotation_bounds_all_eight_corners_and_stops_being_exact() {
        let translated = b("multmatrix([[1, 0, 0, 5], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]) {\n\tcube(size = [2, 2, 2], center = true);\n}");
        assert_eq!(
            translated,
            Bbox::known([4.0, -1.0, -1.0], [6.0, 1.0, 1.0]),
            "a pure translation stays exact"
        );
        // 45 degrees about z: a 2x2 square's corners reach 1*sqrt(2)
        let c = std::f64::consts::FRAC_1_SQRT_2;
        let rotated = b(&format!(
            "multmatrix([[{c}, {}, 0, 0], [{c}, {c}, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]) {{\n\tcube(size = [2, 2, 2], center = true);\n}}",
            -c
        ));
        match rotated {
            Bbox::Known { lo, hi, exact } => {
                assert!(
                    (hi[0] - 2.0f64.sqrt()).abs() < 1e-9,
                    "a turned box reaches sqrt(2), not 1: {hi:?}"
                );
                assert!((lo[0] + 2.0f64.sqrt()).abs() < 1e-9);
                assert!(!exact, "a rotated box is an upper bound");
            }
            other => panic!("expected a box, got {other:?}"),
        }
    }

    /// A DIFFERENCE IS BOUNDED BY ITS FIRST CHILD, because subtracting can
    /// only remove material. The second child is irrelevant however large.
    #[test]
    fn a_difference_is_bounded_by_what_it_started_with() {
        let d =
            b("difference() {\n\tcube(size = [10, 10, 10]);\n\tcube(size = [500, 500, 500]);\n}");
        match d {
            Bbox::Known { lo, hi, exact } => {
                assert_eq!(lo, [0.0, 0.0, 0.0]);
                assert_eq!(hi, [10.0, 10.0, 10.0], "the 500 cube does not enlarge it");
                assert!(!exact, "material may have been removed, so it is a bound");
            }
            other => panic!("expected a box, got {other:?}"),
        }
    }

    #[test]
    fn a_union_covers_both_children() {
        let u = b("union() {\n\tcube(size = [1, 1, 1]);\n\tmultmatrix([[1, 0, 0, 9], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]) {\n\t\tcube(size = [1, 1, 1]);\n\t}\n}");
        assert_eq!(u, Bbox::known([0.0, 0.0, 0.0], [10.0, 1.0, 1.0]));
    }

    /// Two boxes that do not overlap intersect to EMPTY, which is a value.
    #[test]
    fn a_disjoint_intersection_is_empty() {
        let i = b("intersection() {\n\tcube(size = [1, 1, 1]);\n\tmultmatrix([[1, 0, 0, 9], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]) {\n\t\tcube(size = [1, 1, 1]);\n\t}\n}");
        assert_eq!(i, Bbox::Empty);
    }

    /// UNKNOWN POISONS A UNION AND NOT AN INTERSECTION, and the asymmetry is
    /// the point. A union is at least as big as either part, so an unbounded
    /// part makes the whole unbounded. An intersection is no bigger than
    /// either part, so one known side is still a valid bound.
    #[test]
    fn unknown_poisons_a_union_but_not_an_intersection() {
        let known = Bbox::known([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let unk = Bbox::Unknown("a rotate_extrude".into());
        assert!(
            matches!(known.clone().union(unk.clone()), Bbox::Unknown(_)),
            "a union with something unbounded is unbounded"
        );
        match known.intersect(unk) {
            Bbox::Known { hi, exact, .. } => {
                assert_eq!(hi, [1.0, 1.0, 1.0], "still bounded by the known side");
                assert!(!exact, "but no longer exact");
            }
            other => panic!("an intersection keeps the known bound, got {other:?}"),
        }
    }

    /// A NODE THIS CANNOT BOUND SAYS WHICH ONE IT WAS, so a caller that needs
    /// certainty can see what defeated it rather than getting a silent guess.
    #[test]
    fn an_unboundable_node_names_itself() {
        match b("rotate_extrude(angle = 360) {\n\tsquare(size = [1, 1]);\n}") {
            Bbox::Unknown(why) => assert!(
                why.contains("rotate_extrude"),
                "the reason must name the node: {why}"
            ),
            other => panic!("a rotate_extrude must not be guessed at, got {other:?}"),
        }
    }

    /// THE REAL DUMP. The stage contains rotate_extrude and linear_extrude of
    /// polygons, so its root is honestly Unknown -- and that is the right
    /// answer rather than a disappointment. The arm pins that it REFUSES
    /// rather than inventing a number for a part whose true extent is known
    /// from the mesh to be 46.9 x 110.2 x 31.0.
    #[test]
    fn the_real_stage_is_unknown_and_that_is_the_honest_answer() {
        let t = parse(STAGE).expect("parses");
        match bbox(&t) {
            Bbox::Unknown(why) => {
                assert!(!why.is_empty(), "it must say which node defeated it");
            }
            other => panic!(
                "the stage uses extrusions this cannot bound; a number here would be \
                 invented, got {other:?}"
            ),
        }
        // and the primitives inside it ARE bounded, so the walk is not useless
        let bounded = t.walk().iter().filter(|n| bbox(n).is_known()).count();
        assert!(
            bounded >= 10,
            "its cubes and cylinders are still bounded: {bounded}"
        );
    }

    #[test]
    fn an_empty_group_has_an_empty_box() {
        assert_eq!(b("group();"), Bbox::Empty);
    }
}
