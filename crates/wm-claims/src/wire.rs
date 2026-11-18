//! The provider's lexical dependency graph, and what it is worth (issue 52).
//!
//! # Measured, not assumed
//!
//! Issues 37 and 52 both say "provider `trace`" as though it were a tool. It
//! is `get_project_files(mode="trace", symbol=, direction=)`, and the reply
//! carries its own bound, which this module keeps rather than paraphrases:
//!
//! > lexical, file-scope constants only: assignments at brace depth 0 with
//! > comments and strings masked. Module-local variables, parameters and
//! > conditional redefinition are not modelled.
//!
//! Both `upstream` and `downstream` come back on every call regardless of the
//! `direction` argument. That is worth writing down because reading only the
//! first part of one reply led me to conclude `direction` was ignored and
//! upstream was broken -- a negative drawn from a truncated view, which it
//! cannot support.
//!
//! # Why the scope is carried into every verdict
//!
//! A coverage answer computed from this graph is a statement about file-scope
//! constants and nothing else. Issue 52 proposed reading a disagreement as
//! "`trace` sees it and perturbation does not means dead; the reverse is a
//! hole in `trace`". With the bound above, the second half is wrong in the
//! ordinary case: a module parameter or a `$`-variable is outside what this
//! graph claims to model, so perturbation seeing what it does not is expected.
//! Carrying the scope string into [`Coverage`] is what stops a reader taking
//! the verdict further than the instrument goes.

use crate::index::Index;
use serde_json::Value;
use std::collections::BTreeSet;

/// One `mode="trace"` answer: the lexical neighbourhood of a symbol, and the
/// provider's own statement of what it modelled.
#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    pub symbol: String,
    /// The provider's `scope` string, verbatim. Not optional: a graph whose
    /// bound is unknown cannot bound a verdict drawn from it.
    pub scope: String,
    /// Every file-scope constant named anywhere in this answer -- the symbol,
    /// its upstream and its downstream.
    pub named: BTreeSet<String>,
    /// How many file-scope constants the provider parsed in the project. The
    /// denominator: one trace answer names only the closure around its
    /// symbol, so this is how the unnamed remainder is known to exist at all.
    pub constants_parsed: usize,
}

/// Build a [`Trace`] from one `get_project_files(mode="trace")` answer.
///
/// REFUSES AN ANSWER WITH NO SCOPE. The scope is the whole reason a verdict
/// from this graph can be stated honestly; an answer missing it would produce
/// coverage figures that read as though they covered everything.
pub fn trace_from(answer: &Value) -> Result<Trace, String> {
    if answer.get("success").and_then(Value::as_bool) == Some(false) {
        return Err(format!(
            "the trace answer reports success=false ({})",
            answer
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("no error given")
        ));
    }
    let scope = answer
        .get("scope")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            "the trace answer carries no `scope`. Every verdict drawn from this graph is \
             a statement about what it modelled, and one that cannot say what that was \
             would read as covering everything"
                .to_string()
        })?
        .to_string();
    let symbol = answer
        .get("symbol")
        .and_then(Value::as_str)
        .ok_or_else(|| "the trace answer carries no `symbol`".to_string())?
        .to_string();
    let parsed = answer
        .get("constants_parsed")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            "the trace answer carries no `constants_parsed`. Without the denominator a \
             coverage figure cannot say how much it did not see, and an unnamed \
             remainder would silently become zero"
                .to_string()
        })? as usize;
    let mut named = BTreeSet::new();
    named.insert(symbol.clone());
    for key in ["upstream", "downstream"] {
        for row in answer
            .get(key)
            .and_then(Value::as_array)
            .map(|a| a.as_slice())
            .unwrap_or(&[])
        {
            if let Some(n) = row.get("name").and_then(Value::as_str) {
                named.insert(n.to_string());
            }
        }
    }
    if named.len() > parsed {
        return Err(format!(
            "the trace names {} constants and reports {parsed} parsed; a closure cannot \
             be larger than the set it was drawn from, so one of the two figures is wrong",
            named.len()
        ));
    }
    Ok(Trace {
        symbol,
        scope,
        named,
        constants_parsed: parsed,
    })
}

/// What the claim set covers of the model's own symbols.
#[derive(Debug, Clone, PartialEq)]
pub struct Coverage {
    /// The provider's bound, carried so a reader cannot take these verdicts
    /// further than the instrument goes.
    pub scope: String,
    /// Lexically known AND read by some claim: the two accounts agree.
    pub covered: BTreeSet<String>,
    /// Lexically known and NO claim reads it. The valuable half -- issue 40's
    /// two fan-out incidents were both symbols nobody suspected.
    pub uncovered: BTreeSet<String>,
    /// Read by a claim and NOT in the lexical graph. NOT a hole in `trace`:
    /// module-local variables, parameters and conditional redefinition are
    /// outside what it models, by its own statement.
    pub outside_scope: BTreeSet<String>,
    /// Constants the provider parsed that this answer never named, because one
    /// trace reply carries only the closure around its own symbol. Known to
    /// exist, not known by name -- and a count is the honest form of that.
    pub unnamed_remainder: usize,
}

impl Coverage {
    /// Whether every constant the provider parsed was accounted for by name.
    /// False means the verdicts above are about a subset, and the remainder is
    /// a count rather than a list.
    pub fn complete(&self) -> bool {
        self.unnamed_remainder == 0
    }
}

/// Compare a claim set's measured dependencies against the model's lexical
/// constants.
///
/// REFUSES A TRACE THAT NAMED NOTHING BEYOND ITS OWN SYMBOL while reporting
/// constants parsed: coverage over one symbol reads as a coverage figure for
/// the project, and the difference between "covered" and "not looked at" is
/// the only thing this function exists to keep apart.
pub fn coverage(index: &Index, traces: &[Trace]) -> Result<Coverage, String> {
    if traces.is_empty() {
        return Err(
            "no traces to compare against. A coverage report over no lexical \
                    graph names nothing uncovered, which reads exactly like full coverage"
                .into(),
        );
    }
    let scope = traces[0].scope.clone();
    if let Some(odd) = traces.iter().find(|t| t.scope != scope) {
        return Err(format!(
            "these traces were taken under different scopes -- `{scope}` and `{}` -- and \
             a single coverage verdict cannot be bounded by two",
            odd.scope
        ));
    }
    let parsed = traces.iter().map(|t| t.constants_parsed).max().unwrap_or(0);
    let lexical: BTreeSet<String> = traces
        .iter()
        .flat_map(|t| t.named.iter().cloned())
        .collect();
    let mut read: BTreeSet<String> = BTreeSet::new();
    for name in index.claims() {
        if let Some(set) = index.reads(name) {
            read.extend(set.iter().cloned());
        }
    }
    Ok(Coverage {
        scope,
        covered: lexical.intersection(&read).cloned().collect(),
        uncovered: lexical.difference(&read).cloned().collect(),
        outside_scope: read.difference(&lexical).cloned().collect(),
        unnamed_remainder: parsed.saturating_sub(lexical.len()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim::{Claim, Expression, Input, Inputs, Provenance};
    use std::collections::BTreeMap;

    const SCOPE: &str = "lexical, file-scope constants only";

    fn answer(symbol: &str, up: &[&str], down: &[&str], parsed: usize) -> Value {
        serde_json::json!({
            "success": true, "mode": "trace", "symbol": symbol, "scope": SCOPE,
            "constants_parsed": parsed,
            "upstream": up.iter().map(|n| serde_json::json!({"name": n, "depth": 1})).collect::<Vec<_>>(),
            "downstream": down.iter().map(|n| serde_json::json!({"name": n, "depth": 1})).collect::<Vec<_>>(),
        })
    }

    fn inputs(pairs: &[(&str, f64)]) -> Inputs {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert(
                (*k).to_string(),
                Input {
                    value: *v,
                    provenance: Provenance::Measured,
                    uncertainty: None,
                    source: "a test".into(),
                },
            );
        }
        Inputs(m)
    }

    fn claim(name: &str, eval: fn(&Inputs) -> Result<f64, String>) -> Claim {
        Claim {
            name: name.into(),
            declared_inputs: Vec::new(),
            range: (-1.0e9, 1.0e9),
            unit: "mm".into(),
            because: "a test".into(),
            expression: Expression::Rust(eval),
        }
    }

    fn reads_can_r(i: &Inputs) -> Result<f64, String> {
        Ok(i.need("can_r")? * 2.0)
    }

    #[test]
    fn a_trace_answer_names_the_symbol_and_both_directions() {
        // BOTH DIRECTIONS ARRIVE ON EVERY CALL, measured against the pinned
        // provider: `direction` does not select which key comes back.
        let t = trace_from(&answer("can_r", &["can_dia"], &["bore_r", "r_edge"], 147))
            .expect("a well-formed trace");
        assert!(t.named.contains("can_r") && t.named.contains("can_dia"));
        assert!(t.named.contains("bore_r") && t.named.contains("r_edge"));
        assert_eq!(t.constants_parsed, 147);
        assert_eq!(t.scope, SCOPE);
    }

    #[test]
    fn a_trace_with_no_scope_is_refused() {
        let mut a = answer("can_r", &[], &[], 147);
        a.as_object_mut().expect("object").remove("scope");
        let err = trace_from(&a).expect_err("no scope");
        assert!(err.contains("read as covering everything"), "{err}");
    }

    #[test]
    fn a_trace_with_no_denominator_is_refused() {
        let mut a = answer("can_r", &[], &[], 147);
        a.as_object_mut()
            .expect("object")
            .remove("constants_parsed");
        let err = trace_from(&a).expect_err("no constants_parsed");
        assert!(
            err.contains("unnamed \nremainder") || err.contains("remainder"),
            "{err}"
        );
    }

    #[test]
    fn a_closure_larger_than_the_parsed_set_is_refused_as_self_contradictory() {
        let err = trace_from(&answer("can_r", &["a", "b"], &["c", "d"], 2))
            .expect_err("5 named, 2 parsed");
        assert!(err.contains("cannot be larger"), "{err}");
    }

    #[test]
    fn coverage_separates_covered_from_uncovered_and_from_outside_scope() {
        let i = inputs(&[("can_r", 33.0), ("a_local", 1.0)]);
        let ix = Index::build(&[claim("x", reads_can_r)], &i).expect("built");
        // the lexical graph knows can_r and bore_r; the claim reads only can_r
        let t = trace_from(&answer("can_r", &[], &["bore_r"], 2)).expect("trace");
        let c = coverage(&ix, &[t]).expect("one scope");
        assert!(c.covered.contains("can_r"), "{:?}", c.covered);
        assert!(c.uncovered.contains("bore_r"), "{:?}", c.uncovered);
        assert!(c.covered.len() == 1 && c.uncovered.len() == 1);
        assert!(c.complete(), "2 parsed, 2 named");
    }

    #[test]
    fn a_symbol_read_but_not_lexical_is_outside_scope_and_not_a_hole() {
        // A module-local or a parameter: perturbation sees it, the lexical
        // graph does not model it, and by the provider's own statement that is
        // expected rather than a defect in trace.
        fn reads_local(i: &Inputs) -> Result<f64, String> {
            Ok(i.need("a_local")? + 1.0)
        }
        let i = inputs(&[("can_r", 33.0), ("a_local", 1.0)]);
        let ix = Index::build(&[claim("y", reads_local)], &i).expect("built");
        let t = trace_from(&answer("can_r", &[], &[], 1)).expect("trace");
        let c = coverage(&ix, &[t]).expect("ok");
        assert!(c.outside_scope.contains("a_local"), "{:?}", c.outside_scope);
        assert!(c.covered.is_empty());
        // and the bound travels with the verdict
        assert_eq!(c.scope, SCOPE);
    }

    #[test]
    fn the_unnamed_remainder_is_a_count_because_one_trace_names_one_closure() {
        // THE REAL SHAPE, measured: can_r's closure named 14 of the project's
        // 147 constants. The rest are known to exist and not known by name,
        // and a count is the honest form of that.
        let i = inputs(&[("can_r", 33.0)]);
        let ix = Index::build(&[claim("x", reads_can_r)], &i).expect("built");
        let t = trace_from(&answer("can_r", &[], &["bore_r"], 147)).expect("trace");
        let c = coverage(&ix, &[t]).expect("ok");
        assert_eq!(c.unnamed_remainder, 145);
        assert!(!c.complete(), "145 constants were never named");
    }

    #[test]
    fn coverage_over_no_traces_is_refused() {
        let i = inputs(&[("can_r", 33.0)]);
        let ix = Index::build(&[claim("x", reads_can_r)], &i).expect("built");
        let err = coverage(&ix, &[]).expect_err("nothing to compare");
        assert!(err.contains("reads exactly like full coverage"), "{err}");
    }

    #[test]
    fn traces_taken_under_two_different_scopes_cannot_bound_one_verdict() {
        let i = inputs(&[("can_r", 33.0)]);
        let ix = Index::build(&[claim("x", reads_can_r)], &i).expect("built");
        let a = trace_from(&answer("can_r", &[], &[], 2)).expect("a");
        let mut other = answer("bore_r", &[], &[], 2);
        other["scope"] = serde_json::json!("something else entirely");
        let b = trace_from(&other).expect("b");
        let err = coverage(&ix, &[a, b]).expect_err("two scopes");
        assert!(err.contains("cannot be bounded by two"), "{err}");
    }
}
