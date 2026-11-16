//! Which subtractions remove material from a feature.
//!
//! # The incident
//!
//! buildability 66. The stage's two dovetail sockets sat at
//! `horn_deg = mouth_deg / 2` = 34 degrees, and the mouth was removed by
//! `pie(angle = mouth_deg)`, spanning exactly +/-34. The cut sheared the
//! inboard flank off BOTH sockets. Each was left as one angled face with open
//! air where the other belonged, so nothing captured the tenon.
//!
//! Neither feature was wrong. The socket was a correct socket and the mouth
//! was a correct mouth. `eats(socket)` has to name the pie cut, because
//! nothing else in the model will.
//!
//! # Candidates, not verdicts
//!
//! This answers from BOUNDING BOXES, so it reports what COULD be eating a
//! feature and never claims to have measured material removed. Box overlap is
//! NECESSARY for a subtraction to touch a feature and is not SUFFICIENT: two
//! boxes can overlap while the solids inside them miss entirely.
//!
//! That asymmetry is the useful one. A subtrahend this does NOT list cannot
//! be eating the feature, so a short list is a real narrowing, and the ones it
//! does list are where to look.

use crate::bbox::{bbox_at, Bbox};
use crate::tree::Node;

/// A subtraction that may be removing material from the feature.
#[derive(Debug, Clone, PartialEq)]
pub struct Eater {
    /// The subtrahend's operation name.
    pub op: String,
    /// Its path from the root, as child indices, so a caller can find it again
    /// in a tree full of identical-looking `multmatrix` nodes.
    pub path: Vec<usize>,
    /// The `difference` node it belongs to, by path.
    pub difference_at: Vec<usize>,
    /// The overlap of the two boxes, where both were known.
    pub overlap: Bbox,
}

fn boxes_overlap(a: &Bbox, b: &Bbox) -> Option<Bbox> {
    match a.clone().intersect(b.clone()) {
        Bbox::Empty => None,
        other => Some(other),
    }
}

/// Walk the tree collecting `(path, node)` for everything under `root`.
fn walk_with_paths<'a>(n: &'a Node, path: Vec<usize>, out: &mut Vec<(Vec<usize>, &'a Node)>) {
    out.push((path.clone(), n));
    for (i, c) in n.children.iter().enumerate() {
        let mut p = path.clone();
        p.push(i);
        walk_with_paths(c, p, out);
    }
}

/// Everything that might be eating the node at `target_path`.
///
/// A SUBTRAHEND IS ANY CHILD OF A `difference` AFTER THE FIRST. OpenSCAD's
/// first child is the thing being cut; every later one cuts it.
///
/// A feature is only eaten by a difference it is actually INSIDE. A cut
/// elsewhere in the tree does not reach it however much its box overlaps, and
/// listing those would bury the real answer in noise.
pub fn eats(root: &Node, target_path: &[usize]) -> Result<Vec<Eater>, String> {
    let mut all = Vec::new();
    walk_with_paths(root, Vec::new(), &mut all);
    // THE EXISTENCE CHECK IS SEPARATE FROM THE MEASUREMENT on purpose: a path
    // that names nothing must be refused, and `bbox_at` returning None for it
    // would be indistinguishable from a node that simply cannot be bounded.
    if !all.iter().any(|(p, _)| p.as_slice() == target_path) {
        return Err({
            format!(
                "path {target_path:?} names nothing; the tree has {} nodes. A path that \
                 names nothing is refused rather than answered with an empty list, which \
                 would read as 'nothing eats it'",
                all.len()
            )
        });
    }
    // IN THE ROOT'S FRAME, BOTH SIDES. Comparing a node's own box against
    // another's is comparing two coordinate systems; the first version of this
    // did exactly that and reported that the cut which sheared the dovetail
    // sockets did not reach them.
    let tbox = bbox_at(root, target_path).unwrap_or(Bbox::Unknown("no such node".into()));
    let mut out = Vec::new();
    for (dpath, d) in all.iter().filter(|(_, n)| n.op == "difference") {
        // IS THE TARGET INSIDE THIS DIFFERENCE'S FIRST CHILD? Only then can
        // this difference reach it.
        let mut first = dpath.clone();
        first.push(0);
        if !target_path.starts_with(&first) {
            continue;
        }
        for (i, sub) in d.children.iter().enumerate().skip(1) {
            let mut spath = dpath.clone();
            spath.push(i);
            let sbox = bbox_at(root, &spath).unwrap_or(Bbox::Unknown("no such node".into()));
            if let Some(ov) = boxes_overlap(&tbox, &sbox) {
                out.push(Eater {
                    op: sub.op.clone(),
                    path: spath,
                    difference_at: dpath.clone(),
                    overlap: ov,
                });
            }
        }
    }
    Ok(out)
}

/// Find a node by operation name, returning every path that matches.
pub fn paths_of(root: &Node, op: &str) -> Vec<Vec<usize>> {
    let mut all = Vec::new();
    walk_with_paths(root, Vec::new(), &mut all);
    all.into_iter()
        .filter(|(_, n)| n.op == op)
        .map(|(p, _)| p)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::parse;

    /// THE INCIDENT, in the shape the dump has it: a socket inside the thing
    /// being cut, and a cut that reaches it.
    const SHEARED: &str = "difference() {\n\
        \tunion() {\n\
        \t\tcube(size = [20, 20, 10]);\n\
        \t\tmultmatrix([[1, 0, 0, 5], [0, 1, 0, 5], [0, 0, 1, 0], [0, 0, 0, 1]]) {\n\
        \t\t\tcube(size = [4, 4, 10]);\n\
        \t\t}\n\
        \t}\n\
        \tmultmatrix([[1, 0, 0, 6], [0, 1, 0, 6], [0, 0, 1, -1], [0, 0, 0, 1]]) {\n\
        \t\tcube(size = [10, 10, 12]);\n\
        \t}\n\
        }\n";

    /// The inner cube of the union, under its multmatrix.
    const SOCKET: [usize; 3] = [0, 1, 0];

    #[test]
    fn the_cut_that_shears_the_socket_is_named() {
        let t = parse(SHEARED).expect("parses");
        let e = eats(&t, &SOCKET).expect("the path exists");
        assert_eq!(e.len(), 1, "one subtraction reaches it: {e:?}");
        assert_eq!(e[0].op, "multmatrix", "the cut, as the dump wraps it");
        assert_eq!(
            e[0].difference_at,
            Vec::<usize>::new(),
            "the root difference"
        );
        assert!(e[0].overlap.is_known(), "and the overlap is bounded");
    }

    /// A SUBTRACTION THAT MISSES IS NOT LISTED, which is what makes a short
    /// list a real narrowing.
    #[test]
    fn a_cut_that_misses_the_feature_is_not_listed() {
        let t = parse(
            "difference() {\n\
             \tunion() {\n\
             \t\tcube(size = [20, 20, 10]);\n\
             \t\tmultmatrix([[1, 0, 0, 1], [0, 1, 0, 1], [0, 0, 1, 0], [0, 0, 0, 1]]) {\n\
             \t\t\tcube(size = [2, 2, 2]);\n\
             \t\t}\n\
             \t}\n\
             \tmultmatrix([[1, 0, 0, 15], [0, 1, 0, 15], [0, 0, 1, 0], [0, 0, 0, 1]]) {\n\
             \t\tcube(size = [3, 3, 3]);\n\
             \t}\n\
             }\n",
        )
        .expect("parses");
        assert!(eats(&t, &[0, 1, 0]).expect("exists").is_empty());
    }

    /// A CUT ELSEWHERE IN THE TREE DOES NOT REACH IT, however much the boxes
    /// overlap. Listing those would bury the real answer in noise.
    #[test]
    fn a_cut_in_a_different_branch_does_not_eat_it() {
        let t = parse(
            "union() {\n\
             \tcube(size = [10, 10, 10]);\n\
             \tdifference() {\n\
             \t\tcube(size = [10, 10, 10]);\n\
             \t\tcube(size = [10, 10, 10]);\n\
             \t}\n\
             }\n",
        )
        .expect("parses");
        // the FIRST cube is not inside the difference, though every box
        // overlaps every other
        assert!(
            eats(&t, &[0]).expect("exists").is_empty(),
            "a sibling's difference does not reach it"
        );
        // and the difference's own first child IS eaten
        assert_eq!(eats(&t, &[1, 0]).expect("exists").len(), 1);
    }

    /// A PATH THAT NAMES NOTHING IS REFUSED, because an empty list would read
    /// as "nothing eats it" -- the most reassuring possible answer to a
    /// question that was never asked.
    #[test]
    fn a_path_that_names_no_node_is_refused() {
        let t = parse(SHEARED).expect("parses");
        let e = eats(&t, &[9, 9, 9]).expect_err("must refuse");
        assert!(e.contains("names nothing"), "{e}");
        assert!(e.contains("read as"), "{e}");
    }

    /// Several subtractions reaching one feature are ALL listed; stopping at
    /// the first would hide the second cut.
    #[test]
    fn every_reaching_subtraction_is_listed() {
        let t = parse(
            "difference() {\n\
             \tcube(size = [20, 20, 20]);\n\
             \tcube(size = [5, 5, 5]);\n\
             \tmultmatrix([[1, 0, 0, 2], [0, 1, 0, 2], [0, 0, 1, 2], [0, 0, 0, 1]]) {\n\
             \t\tcube(size = [5, 5, 5]);\n\
             \t}\n\
             }\n",
        )
        .expect("parses");
        assert_eq!(eats(&t, &[0]).expect("exists").len(), 2);
    }

    /// The first child of a difference is the thing being CUT, never a cutter.
    #[test]
    fn the_first_child_is_not_a_subtrahend() {
        let t = parse(SHEARED).expect("parses");
        let e = eats(&t, &[0, 0]).expect("exists");
        assert!(
            e.iter().all(|x| x.path != vec![0]),
            "the thing being cut must not be listed as eating itself: {e:?}"
        );
    }

    #[test]
    fn paths_of_finds_every_node_with_an_operation() {
        let t = parse(SHEARED).expect("parses");
        assert_eq!(paths_of(&t, "difference").len(), 1);
        assert_eq!(paths_of(&t, "cube").len(), 3);
        assert!(paths_of(&t, "sphere").is_empty());
    }
}
