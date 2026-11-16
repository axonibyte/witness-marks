//! Where a number in the dump came from.
//!
//! # The problem this exists for
//!
//! The CSG dump holds resolved numbers. A socket sitting at 34 degrees is
//! written `34`, and nothing says it was `mouth_deg / 2`. Class G -- two
//! correct features whose positions collide -- is precisely the case where a
//! DERIVED position lands on another feature's boundary, and you cannot see
//! that in a tree of literals.
//!
//! The harness incident: the stage's dovetail sockets sat at
//! `horn_deg = mouth_deg / 2` = 34 degrees, and the mouth cut spanned exactly
//! +/-34. Both features were correct. The cut sheared a flank off both
//! sockets, and nothing in either feature was wrong.
//!
//! The fixture carries the FIX now -- `horn_deg = dt_at`, with the comment
//! "NOT mouth_deg/2 any more" -- so the live file no longer has the
//! coincidence. The incident's own numbers are still the test.
//!
//! # Every candidate, never the first
//!
//! A value with TWO explanations is the signal. Reporting one derivation and
//! stopping would hide exactly the case worth reporting: that 34 is both
//! `horn_deg` and `mouth_deg / 2`.
//!
//! # The search is bounded, and the bound is stated
//!
//! Direct equality, a binding halved, doubled, tripled or divided by three,
//! and the sum, difference or product of a PAIR. That is all. It is not an
//! algebra system and does not pretend to be: a derivation this misses is a
//! derivation nobody is told about, which is why the list is written down
//! rather than left to be inferred from the code.

use std::collections::BTreeMap;

/// One way a number could have been arrived at.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Derivation {
    /// How it reads, e.g. `mouth_deg / 2`.
    pub expr: String,
    /// The bindings it uses.
    pub from: Vec<String>,
}

/// Bindings this search understands.
pub type Bindings = BTreeMap<String, f64>;

/// Top-level numeric assignments in a SCAD source.
///
/// ONLY LITERAL NUMBERS. `can_r = can_dia / 2` is an expression and is not
/// collected: resolving it would mean evaluating OpenSCAD, and a half-working
/// evaluator that silently gets one case wrong is worse than one that plainly
/// does not try. A caller with the real values -- from `scad_eval`, say --
/// passes them in.
pub fn literal_bindings(source: &str) -> Bindings {
    let mut out = Bindings::new();
    for line in source.lines() {
        let line = line.trim();
        let Some(eq) = line.find('=') else { continue };
        let name = line[..eq].trim();
        if name.is_empty()
            || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            || name.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            continue;
        }
        let rhs = line[eq + 1..]
            .split(';')
            .next()
            .unwrap_or("")
            .split("//")
            .next()
            .unwrap_or("")
            .trim();
        if let Ok(v) = rhs.parse::<f64>() {
            out.insert(name.to_string(), v);
        }
    }
    out
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    a.is_finite() && b.is_finite() && (a - b).abs() <= tol
}

/// Every way `value` could have come from these bindings, within `tol`.
///
/// Returns them SORTED and DEDUPLICATED, so two runs give the same answer and
/// a caller can compare two nodes' derivations directly.
pub fn derived_from(value: f64, b: &Bindings, tol: f64) -> Vec<Derivation> {
    let mut out: Vec<Derivation> = Vec::new();
    if !value.is_finite() {
        return out;
    }
    let mut add = |expr: String, from: Vec<String>| {
        let d = Derivation { expr, from };
        if !out.contains(&d) {
            out.push(d);
        }
    };
    for (n, v) in b {
        if close(*v, value, tol) {
            add(n.clone(), vec![n.clone()]);
        }
        for (k, label) in [(2.0, "/ 2"), (3.0, "/ 3")] {
            if close(*v / k, value, tol) {
                add(format!("{n} {label}"), vec![n.clone()]);
            }
        }
        for (k, label) in [(2.0, "* 2"), (3.0, "* 3")] {
            if close(*v * k, value, tol) {
                add(format!("{n} {label}"), vec![n.clone()]);
            }
        }
    }
    for (n1, v1) in b {
        for (n2, v2) in b {
            if n1 >= n2 {
                continue; // one order per unordered pair for + and *
            }
            if close(v1 + v2, value, tol) {
                add(format!("{n1} + {n2}"), vec![n1.clone(), n2.clone()]);
            }
            if close(v1 * v2, value, tol) {
                add(format!("{n1} * {n2}"), vec![n1.clone(), n2.clone()]);
            }
        }
        for (n2, v2) in b {
            if n1 == n2 {
                continue;
            }
            if close(v1 - v2, value, tol) {
                add(format!("{n1} - {n2}"), vec![n1.clone(), n2.clone()]);
            }
        }
    }
    out.sort();
    out
}

/// A value that two different bindings explain: the class G signal.
///
/// A COINCIDENCE IS NOT A BUG BY ITSELF. Two features legitimately placed at
/// the same angle are fine; the defect is when one of them is DERIVED from a
/// parameter nobody connected to the other. So this reports and never
/// judges, and the judging is left to whoever knows what the features are.
pub fn is_coincidence(ds: &[Derivation]) -> bool {
    let mut roots: Vec<&String> = ds.iter().flat_map(|d| d.from.iter()).collect();
    roots.sort();
    roots.dedup();
    roots.len() > 1
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../tests/fixtures/can_cutter/can_cutter.scad");

    fn incident() -> Bindings {
        // The bindings AS THEY WERE when the sockets were sheared.
        Bindings::from([
            ("mouth_deg".to_string(), 68.0),
            ("horn_deg".to_string(), 34.0),
            ("wall".to_string(), 5.0),
        ])
    }

    /// THE INCIDENT. 34 degrees is BOTH `horn_deg` and `mouth_deg / 2`, and
    /// reporting only the first would hide the thing worth reporting.
    #[test]
    fn the_sheared_socket_angle_has_two_explanations() {
        let ds = derived_from(34.0, &incident(), 1e-9);
        let exprs: Vec<&str> = ds.iter().map(|d| d.expr.as_str()).collect();
        assert!(exprs.contains(&"horn_deg"), "{exprs:?}");
        assert!(exprs.contains(&"mouth_deg / 2"), "{exprs:?}");
        assert!(
            is_coincidence(&ds),
            "two independent roots is the class G signal"
        );
    }

    /// A value with ONE explanation is not a coincidence, however many ways
    /// that one binding can be written.
    #[test]
    fn a_single_root_is_not_a_coincidence() {
        let b = Bindings::from([("wall".to_string(), 5.0)]);
        let ds = derived_from(10.0, &b, 1e-9);
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].expr, "wall * 2");
        assert!(!is_coincidence(&ds));
    }

    /// A ZERO PARAMETER EXPLAINS A ZERO EVERY WAY AT ONCE, and that is ONE
    /// root, not a coincidence. `x`, `x / 2`, `x / 3`, `x * 2` and `x * 3` are
    /// all 0 when x is 0.
    ///
    /// This arm exists because a mutant proved the need for it: counting
    /// EXPRESSIONS instead of ROOTS survived every other arm, since every
    /// other value here has one expression per root. A parameter left at zero
    /// is ordinary -- `explode = 0` while measuring, for one -- and reporting
    /// it as a class G coincidence would be a false alarm on the most common
    /// value in any model.
    #[test]
    fn a_zero_parameter_is_not_five_coincidences() {
        let b = Bindings::from([("x".to_string(), 0.0)]);
        let ds = derived_from(0.0, &b, 1e-9);
        assert!(
            ds.len() > 1,
            "zero really is explained several ways: {:?}",
            ds.iter().map(|d| &d.expr).collect::<Vec<_>>()
        );
        assert!(
            !is_coincidence(&ds),
            "but they all come from ONE binding, so it is not a coincidence"
        );
    }

    /// Pairs, so `body_r = bore_r + wall` style derivations are visible.
    #[test]
    fn a_sum_of_two_bindings_is_found() {
        let b = Bindings::from([("bore_r".to_string(), 33.6), ("wall".to_string(), 5.0)]);
        let ds = derived_from(38.6, &b, 1e-9);
        assert!(
            ds.iter().any(|d| d.expr == "bore_r + wall"),
            "{:?}",
            ds.iter().map(|d| &d.expr).collect::<Vec<_>>()
        );
    }

    /// A difference is ORDERED, and both orders are searched: `a - b` and
    /// `b - a` are different numbers and either may be the one that explains.
    #[test]
    fn a_difference_is_found_in_both_orders() {
        let b = Bindings::from([("big".to_string(), 10.0), ("small".to_string(), 4.0)]);
        assert!(derived_from(6.0, &b, 1e-9)
            .iter()
            .any(|d| d.expr == "big - small"));
        assert!(derived_from(-6.0, &b, 1e-9)
            .iter()
            .any(|d| d.expr == "small - big"));
    }

    /// NOTHING IS NOT AN ERROR. A number with no explanation is a number
    /// somebody typed, and an empty list says exactly that.
    #[test]
    fn an_unexplainable_number_yields_an_empty_list() {
        let ds = derived_from(7.919, &incident(), 1e-9);
        assert!(ds.is_empty());
        assert!(!is_coincidence(&ds));
    }

    /// The tolerance is the caller's, because "equal" at 1e-9 and "equal" at
    /// 0.01 are different questions about a printed part.
    #[test]
    fn the_tolerance_is_the_callers() {
        let b = Bindings::from([("x".to_string(), 10.0)]);
        assert!(derived_from(10.004, &b, 1e-9).is_empty());
        assert!(!derived_from(10.004, &b, 0.01).is_empty());
    }

    /// ONLY LITERAL ASSIGNMENTS ARE COLLECTED from source. `can_r = can_dia / 2`
    /// is an expression, and resolving it would mean evaluating OpenSCAD -- a
    /// half-working evaluator that silently gets one case wrong is worse than
    /// one that plainly does not try.
    #[test]
    fn literal_bindings_are_read_and_expressions_are_not_guessed_at() {
        let b = literal_bindings(FIXTURE);
        assert_eq!(b.get("mouth_deg"), Some(&68.0));
        assert_eq!(b.get("can_dia"), Some(&66.0));
        assert_eq!(b.get("wall"), Some(&5.0));
        assert!(
            !b.contains_key("can_r"),
            "can_r = can_dia / 2 is an expression and is NOT guessed at"
        );
        assert!(
            !b.contains_key("horn_deg"),
            "horn_deg = dt_at is an expression too"
        );
    }

    /// AND THE FIXTURE CARRIES THE FIX. The live file sets `horn_deg = dt_at`
    /// with the comment "NOT mouth_deg/2 any more", so the coincidence that
    /// sheared the sockets is gone from it. The incident's numbers are still
    /// the test above; this pins that the file moved on.
    #[test]
    fn the_fixture_no_longer_derives_the_horn_from_the_mouth() {
        assert!(
            FIXTURE.contains("NOT mouth_deg/2 any more"),
            "the fix and its reason are in the file"
        );
        let b = literal_bindings(FIXTURE);
        let ds = derived_from(34.0, &b, 1e-9);
        assert!(
            ds.iter().all(|d| d.expr != "horn_deg"),
            "horn_deg is no longer a literal 34: {ds:?}"
        );
    }

    #[test]
    fn a_non_finite_value_explains_nothing() {
        assert!(derived_from(f64::NAN, &incident(), 1e-9).is_empty());
        assert!(derived_from(f64::INFINITY, &incident(), 1e-9).is_empty());
    }
}
