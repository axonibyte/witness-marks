//! Parsing OpenSCAD's CSG dump into a tree.
//!
//! # Why the dump and not the mesh
//!
//! Every Layer 1 call runs on the RESULT: a mesh, where a feature that was
//! sheared in half by a cut and a feature that was drawn that way are the same
//! triangles. Class G defects -- two correct features whose positions collide
//! -- live in the OPERATIONS, and the dump is where the operations are.
//!
//! # The format
//!
//! `openscad -o out.csg in.scad` writes a nested text form:
//!
//! ```text
//! difference() {
//!     cube(size = [10, 10, 10], center = false);
//!     multmatrix([[1, 0, 0, 2], ...]) {
//!         cylinder($fn = 16, h = 12, r1 = 2, r2 = 2, center = false);
//!     }
//! }
//! ```
//!
//! A node is `name(args)` followed by either `{ children }` or `;`. That is
//! the whole grammar, and this parser implements exactly that rather than
//! understanding any particular node -- a dump containing an operation nobody
//! has heard of still parses, which matters because the set grows.
//!
//! A real model's dump holds multmatrix, group, polygon, linear_extrude,
//! cylinder, cube, square, rotate_extrude, difference, union, intersection
//! and hull. That list was read off the vendored stage, not recalled.

use std::collections::BTreeMap;

/// One node of the construction tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub op: String,
    /// Arguments as RAW TEXT, keyed by name. Positional arguments, which
    /// `multmatrix` uses, are keyed `"0"`, `"1"` and so on.
    ///
    /// RAW TEXT ON PURPOSE. Parsing every value into a typed form would mean
    /// deciding what `[[1, 0, 0, 2], ...]` is before anyone has asked, and
    /// getting it wrong for a node type that has not been seen yet. The text
    /// is what the dump said; callers that need a number ask for one.
    pub args: BTreeMap<String, String>,
    pub children: Vec<Node>,
}

/// Why a dump could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub at: usize,
    pub why: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CSG dump, byte {}: {}", self.at, self.why)
    }
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn ws(&mut self) {
        while self.i < self.s.len() {
            let c = self.s[self.i];
            if c.is_ascii_whitespace() {
                self.i += 1;
            } else if c == b'/' && self.s.get(self.i + 1) == Some(&b'/') {
                while self.i < self.s.len() && self.s[self.i] != b'\n' {
                    self.i += 1;
                }
            } else {
                break;
            }
        }
    }

    fn err(&self, why: &str) -> ParseError {
        ParseError {
            at: self.i,
            why: why.to_string(),
        }
    }

    /// Read to the matching close, honouring nesting and string literals.
    ///
    /// STRINGS ARE HONOURED because a `text()` node's content can contain a
    /// bracket, and a depth counter that ignored quotes would end the argument
    /// list in the middle of somebody's label.
    fn balanced(&mut self, open: u8, close: u8) -> Result<String, ParseError> {
        if self.s.get(self.i) != Some(&open) {
            return Err(self.err(&format!("expected {}", open as char)));
        }
        let start = self.i + 1;
        let mut depth = 0i32;
        let mut in_str = false;
        while self.i < self.s.len() {
            let c = self.s[self.i];
            if in_str {
                if c == b'\\' {
                    self.i += 2;
                    continue;
                }
                if c == b'"' {
                    in_str = false;
                }
            } else if c == b'"' {
                in_str = true;
            } else if c == open {
                depth += 1;
            } else if c == close {
                depth -= 1;
                if depth == 0 {
                    let out = String::from_utf8_lossy(&self.s[start..self.i]).into_owned();
                    self.i += 1;
                    return Ok(out);
                }
            }
            self.i += 1;
        }
        Err(self.err(&format!("unclosed {}", open as char)))
    }

    fn node(&mut self) -> Result<Option<Node>, ParseError> {
        self.ws();
        if self.i >= self.s.len() || self.s[self.i] == b'}' {
            return Ok(None);
        }
        let name_start = self.i;
        while self.i < self.s.len()
            && (self.s[self.i].is_ascii_alphanumeric()
                || self.s[self.i] == b'_'
                || self.s[self.i] == b'$')
        {
            self.i += 1;
        }
        if self.i == name_start {
            return Err(self.err("expected an operation name"));
        }
        let op = String::from_utf8_lossy(&self.s[name_start..self.i]).into_owned();
        self.ws();
        let raw = self.balanced(b'(', b')')?;
        let args = split_args(&raw);
        self.ws();
        let mut children = Vec::new();
        match self.s.get(self.i) {
            Some(b'{') => {
                let body = self.balanced(b'{', b'}')?;
                let mut inner = P {
                    s: body.as_bytes(),
                    i: 0,
                };
                while let Some(c) = inner.node()? {
                    children.push(c);
                }
            }
            Some(b';') => self.i += 1,
            // A NODE WITH NEITHER IS MALFORMED AND SAYS SO. Treating it as a
            // childless leaf would silently drop whatever followed.
            _ => return Err(self.err(&format!("after {op}(...) expected '{{' or ';'"))),
        }
        Ok(Some(Node { op, args, children }))
    }
}

/// Split an argument list at TOP-LEVEL commas only.
fn split_args(raw: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let b = raw.as_bytes();
    let (mut depth, mut in_str, mut start, mut positional) = (0i32, false, 0usize, 0usize);
    let push = |piece: &str, positional: &mut usize, out: &mut BTreeMap<String, String>| {
        let p = piece.trim();
        if p.is_empty() {
            return;
        }
        // SPLIT ON THE FIRST TOP-LEVEL '=' ONLY. A value may contain one.
        match p.find('=') {
            Some(k) if !p[..k].contains(['[', '(']) => {
                out.insert(p[..k].trim().to_string(), p[k + 1..].trim().to_string());
            }
            _ => {
                out.insert(positional.to_string(), p.to_string());
                *positional += 1;
            }
        }
    };
    for i in 0..b.len() {
        let c = b[i];
        if in_str {
            if c == b'"' {
                in_str = false;
            }
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'[' | b'(' => depth += 1,
            b']' | b')' => depth -= 1,
            b',' if depth == 0 => {
                push(&raw[start..i], &mut positional, &mut out);
                start = i + 1;
            }
            _ => {}
        }
    }
    push(&raw[start..], &mut positional, &mut out);
    out
}

/// Parse a whole dump. A dump with several top-level nodes is wrapped in an
/// implicit `group`, which is what OpenSCAD itself means by them.
pub fn parse(dump: &str) -> Result<Node, ParseError> {
    let mut p = P {
        s: dump.as_bytes(),
        i: 0,
    };
    let mut roots = Vec::new();
    while let Some(n) = p.node()? {
        roots.push(n);
    }
    p.ws();
    if p.i < p.s.len() {
        return Err(p.err("trailing text after the last node"));
    }
    match roots.len() {
        // AN EMPTY DUMP IS AN ERROR, NOT AN EMPTY TREE. OpenSCAD writes
        // nothing for a model that evaluates to no geometry, and returning a
        // cheerful empty group would make "no geometry" indistinguishable
        // from "a group with no children", which is a real thing a model can
        // contain.
        0 => Err(ParseError {
            at: 0,
            why: "the dump is empty: OpenSCAD writes no CSG for a model with no geometry, \
                  and an empty tree is not the same as a group with no children"
                .into(),
        }),
        1 => Ok(roots.remove(0)),
        _ => Ok(Node {
            op: "group".into(),
            args: BTreeMap::new(),
            children: roots,
        }),
    }
}

impl Node {
    /// Every node in the tree, parents before children.
    pub fn walk(&self) -> Vec<&Node> {
        let mut out = vec![self];
        for c in &self.children {
            out.extend(c.walk());
        }
        out
    }

    /// How many nodes carry this operation name.
    pub fn count_of(&self, op: &str) -> usize {
        self.walk().iter().filter(|n| n.op == op).count()
    }

    /// A numeric argument, where it is a plain number.
    pub fn num(&self, key: &str) -> Option<f64> {
        self.args.get(key)?.trim().parse::<f64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE: &str = "difference() {\n\tcube(size = [10, 10, 10], center = false);\n\tmultmatrix([[1, 0, 0, 2], [0, 1, 0, 2], [0, 0, 1, -1], [0, 0, 0, 1]]) {\n\t\tcylinder($fn = 16, $fa = 12, $fs = 2, h = 12, r1 = 2, r2 = 2, center = false);\n\t}\n}\n";

    /// THE REAL DUMP of the vendored stage, 8612 bytes, written by OpenSCAD
    /// 2026.10.14. Parsing a hand-written sample would prove only that the
    /// parser agrees with whoever wrote the sample.
    const STAGE: &str = include_str!("../../../tests/fixtures/can_cutter/stage.csg");

    #[test]
    fn layer_is_named() {
        assert!(!crate::LAYER.is_empty());
    }

    #[test]
    fn a_difference_parses_into_its_two_children() {
        let t = parse(SIMPLE).expect("parses");
        assert_eq!(t.op, "difference");
        assert_eq!(t.children.len(), 2);
        assert_eq!(t.children[0].op, "cube");
        assert_eq!(t.children[1].op, "multmatrix");
        assert_eq!(t.children[1].children[0].op, "cylinder");
    }

    /// Named arguments keep their names; a positional one, which multmatrix
    /// uses, is keyed by its index.
    #[test]
    fn named_and_positional_arguments_are_both_kept() {
        let t = parse(SIMPLE).expect("parses");
        assert_eq!(
            t.children[0].args.get("size").map(String::as_str),
            Some("[10, 10, 10]")
        );
        assert_eq!(
            t.children[0].args.get("center").map(String::as_str),
            Some("false")
        );
        assert!(
            t.children[1].args.contains_key("0"),
            "multmatrix's matrix is positional: {:?}",
            t.children[1].args
        );
    }

    /// COMMAS INSIDE BRACKETS ARE NOT ARGUMENT SEPARATORS. A naive split on
    /// ',' turns one matrix into sixteen arguments.
    #[test]
    fn a_nested_comma_does_not_split_an_argument() {
        let t = parse(SIMPLE).expect("parses");
        let m = &t.children[1].args;
        assert_eq!(m.len(), 1, "one matrix, not sixteen numbers: {m:?}");
        assert!(m["0"].starts_with("[[1, 0, 0, 2]"), "{}", m["0"]);
    }

    #[test]
    fn dollar_arguments_survive() {
        let c = &parse(SIMPLE).expect("parses").children[1].children[0];
        assert_eq!(c.num("$fn"), Some(16.0));
        assert_eq!(c.num("h"), Some(12.0));
        assert_eq!(c.num("r1"), Some(2.0));
    }

    /// AN UNKNOWN OPERATION STILL PARSES. The grammar is `name(args)` then
    /// `{...}` or `;`, and a parser that only knew a fixed list would fail on
    /// the next OpenSCAD release.
    #[test]
    fn an_operation_nobody_has_heard_of_still_parses() {
        let t = parse("frobnicate(wobble = 3) {\n\tcube(size = [1, 1, 1]);\n}\n").expect("parses");
        assert_eq!(t.op, "frobnicate");
        assert_eq!(t.num("wobble"), Some(3.0));
        assert_eq!(t.children.len(), 1);
    }

    /// THE REAL DUMP, and the node census read off it rather than recalled.
    ///
    /// THE GROUP COUNT IS 30 AND THE FILE SAYS 29. The dump has TWO top-level
    /// nodes, which this parser wraps in an implicit `group` because that is
    /// what OpenSCAD means by them. The census was taken from the file's text
    /// and the parser reports the tree, so the two differ by exactly that one
    /// node. Asserted as 30 with the reason, rather than quietly relaxed to a
    /// range, because the difference is a fact about the parser worth pinning.
    #[test]
    fn the_vendored_stages_dump_parses_with_the_nodes_it_has() {
        let t = parse(STAGE).expect("the real dump parses");
        assert_eq!(
            t.op, "group",
            "the implicit root wrapping two top-level nodes"
        );
        assert_eq!(t.children.len(), 2, "and it has exactly those two");
        assert_eq!(
            STAGE.matches("group(").count(),
            29,
            "the FILE holds 29; the thirtieth is the implicit root"
        );
        for (op, n) in [
            ("multmatrix", 47),
            ("group", 30),
            ("polygon", 9),
            ("linear_extrude", 9),
            ("cylinder", 5),
            ("cube", 5),
            ("square", 2),
            ("rotate_extrude", 2),
            ("difference", 2),
            ("union", 1),
            ("intersection", 1),
            ("hull", 1),
        ] {
            assert_eq!(t.count_of(op), n, "expected {n} {op} node(s)");
        }
        assert!(t.walk().len() > 100, "the whole tree is reachable");
    }

    /// AN EMPTY DUMP IS AN ERROR, NOT AN EMPTY TREE. OpenSCAD writes nothing
    /// for a model with no geometry, and a cheerful empty group would make
    /// that indistinguishable from a group that really has no children --
    /// which is a thing a model can contain.
    #[test]
    fn an_empty_dump_is_refused_rather_than_read_as_an_empty_group() {
        let e = parse("   \n  ").expect_err("must refuse");
        assert!(e.why.contains("no geometry"), "{e}");
        let ok = parse("group();").expect("an explicit empty group is a real node");
        assert_eq!(ok.op, "group");
        assert!(ok.children.is_empty());
    }

    /// A node with neither a body nor a semicolon is malformed, and treating
    /// it as a childless leaf would silently drop whatever followed.
    #[test]
    fn a_node_with_no_body_and_no_semicolon_is_refused() {
        let e =
            parse("cube(size = [1, 1, 1])\ncube(size = [2, 2, 2]);\n").expect_err("must refuse");
        assert!(e.why.contains("expected"), "{e}");
    }

    #[test]
    fn an_unclosed_brace_is_refused_with_its_position() {
        let e = parse("difference() {\n\tcube(size = [1, 1, 1]);\n").expect_err("must refuse");
        assert!(e.why.contains("unclosed"), "{e}");
        assert!(e.at > 0, "the position is reported");
    }

    /// Several top-level nodes become an implicit group, which is what
    /// OpenSCAD itself means by them.
    #[test]
    fn several_top_level_nodes_become_a_group() {
        let t = parse("cube(size = [1, 1, 1]);\nsphere(r = 2);\n").expect("parses");
        assert_eq!(t.op, "group");
        assert_eq!(t.children.len(), 2);
    }

    /// A bracket inside a string must not end the argument list. `text()`
    /// labels really do contain them.
    #[test]
    fn a_bracket_inside_a_string_does_not_end_the_arguments() {
        let t = parse("text(text = \"a [bracket], here\", size = 4);\n").expect("parses");
        assert_eq!(t.num("size"), Some(4.0));
        assert_eq!(
            t.args.get("text").map(String::as_str),
            Some("\"a [bracket], here\"")
        );
    }

    /// AN UNBALANCED DELIMITER INSIDE A STRING, which is the case that
    /// actually needs the string handling. The arm above does NOT: its
    /// bracket is balanced, so a parser ignoring quotes counts it up and back
    /// down again and still parses correctly. A mutant that disabled string
    /// tracking survived that arm, and this one is what kills it.
    ///
    /// A closing paren inside a label ends the argument list early for any
    /// parser that is only counting depth.
    #[test]
    fn an_unbalanced_delimiter_inside_a_string_is_still_inside_it() {
        let t = parse("text(text = \"smile :) here\", size = 4);\n").expect("parses");
        assert_eq!(
            t.num("size"),
            Some(4.0),
            "a parser that stopped at the paren inside the string would never reach size"
        );
        assert_eq!(
            t.args.get("text").map(String::as_str),
            Some("\"smile :) here\"")
        );
        // and the same for an unmatched opening bracket
        let u = parse("text(text = \"a [ here\", size = 7);\n").expect("parses");
        assert_eq!(u.num("size"), Some(7.0));
    }
}
