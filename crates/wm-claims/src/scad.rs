//! Evaluating a claim in the model's own scope (issue 60).
//!
//! # The split, and why the transport is injected
//!
//! This crate launches no process. The provider needs `uv`, a resolved Python
//! environment and OpenSCAD, none of which the forge jail has, so a judgment
//! crate that spawned it would be a crate whose tests cannot run -- and the
//! parse is where the defects are, not the socket. That is the pattern 2d
//! settled on: a pure parser driven by captured replies, with the process
//! living in `wm-provider`.
//!
//! So [`ScadEval`] takes a TRANSPORT: anything that will send expressions and
//! variable overrides and hand back the provider's reply. CI drives it with a
//! recorded reply; `wm-provider` supplies the real one.
//!
//! # What the reply looks like, measured rather than assumed
//!
//! Driven against the pinned provider and `arts/can_cutter.scad`:
//!
//! ```text
//! {"success": true,
//!  "results": [{"index":0,"value":33.6,"type":"number","evaluated":true,
//!               "expression":"bore_r"}],
//!  "errors": [], "warnings": ["WARNING: \"can_dia\" was assigned on line 38
//!                             but was overwritten in f..."]}
//! ```
//!
//! THE WARNING IS EXPECTED AND IS NOT AN ERROR. Overriding a variable is how
//! perturbation works, and the provider names the line it overwrote. An
//! override that drew no warning would be the surprising case.

use crate::claim::{Evaluator, Expression, Inputs};
use serde_json::Value;
use std::collections::BTreeMap;

/// The variable overrides a set of inputs stands for.
///
/// EVERY INPUT IS SENT, not only the ones a claim declares. Perturbation works
/// by moving one and seeing whether the value moves; an input withheld because
/// nobody declared it could never be seen to matter, which is the declaration
/// being trusted again -- the thing this layer exists to stop.
pub fn overrides(inputs: &Inputs) -> BTreeMap<String, f64> {
    inputs
        .names()
        .into_iter()
        .filter_map(|n| inputs.get(&n).map(|i| (n, i.value)))
        .collect()
}

/// Read one expression's number out of a `scad_eval` reply.
///
/// # Every refusal here is a way a non-number could become a number
///
/// * `success: false` -- the errors are reported rather than summarized.
/// * no `results` array -- an absent array is not an empty one, and an empty
///   one is not a zero.
/// * the expression not among the results -- a reply about something else.
/// * `evaluated: false` -- the provider saying it did not evaluate this.
/// * a `type` other than `number`. **`undef` matters most**: it is OpenSCAD's
///   null, it is what an unset variable reads as, and as a `0.0` it would pass
///   every range containing zero. A vector, string, bool or range is likewise
///   not a value a range check can judge.
/// * a non-finite number, which is not a value either.
pub fn value_of(reply: &Value, expression: &str) -> Result<f64, String> {
    if reply.get("success").and_then(Value::as_bool) == Some(false) {
        let errs = reply
            .get("errors")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|e| e.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_default();
        return Err(format!(
            "scad_eval reports success=false for {expression:?}{}",
            if errs.is_empty() {
                String::new()
            } else {
                format!(": {errs}")
            }
        ));
    }
    let rows = reply
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "the scad_eval reply for {expression:?} carries no `results` array. An absent \
             array is not an empty one, and an empty one is not a zero"
            )
        })?;
    let row = rows
        .iter()
        .find(|r| r.get("expression").and_then(Value::as_str) == Some(expression))
        .ok_or_else(|| {
            format!(
                "the scad_eval reply names {} expression(s) and none of them is \
                 {expression:?}; this is an answer to a different question",
                rows.len()
            )
        })?;
    if row.get("evaluated").and_then(Value::as_bool) == Some(false) {
        return Err(format!(
            "the provider reports {expression:?} as not evaluated; an unevaluated \
             expression has no value and must not read as one"
        ));
    }
    let kind = row.get("type").and_then(Value::as_str).unwrap_or("absent");
    if kind != "number" {
        return Err(format!(
            "{expression:?} evaluated to a {kind}, not a number. A claim is a quantity \
             with a range; `undef` in particular is OpenSCAD's absent value and as a \
             zero it would pass every range containing zero"
        ));
    }
    let v = row
        .get("value")
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("{expression:?} is typed `number` and carries no numeric value"))?;
    if !v.is_finite() {
        return Err(format!(
            "{expression:?} evaluated to {v}, which is not a value"
        ));
    }
    Ok(v)
}

/// The overrides the provider did NOT report overwriting anything for.
///
/// # This is the founding defect of the project, one layer along
///
/// Spec §01's opening incident is OpenSCAD's `-D`: it APPENDS an assignment,
/// so a misspelled parameter name defines a variable nothing reads, draws no
/// warning, and the render produces exactly the bytes it produced before.
/// `scad_eval`'s `variables` has the same shape -- MEASURED against the pinned
/// provider and the real harness:
///
/// * overriding `can_dia`, which the file assigns, returns
///   `WARNING: "can_dia" was assigned on line 38 but was overwritten`
/// * overriding `can_clear`, which the file does NOT have (its symbol is
///   `can_clearance`), returns NO warning and the same value
///
/// So the warning's ABSENCE is the signal, and it is the only one on offer.
///
/// # What it means, stated narrowly
///
/// A name here is one the provider did not report overwriting a file-scope
/// assignment for. That covers a misspelling AND a symbol that exists but is
/// not a file-scope constant -- the same bound `get_project_files(mode=trace)`
/// states for itself. It does NOT by itself mean the name is wrong.
///
/// REPORTED, NOT REFUSED. `derived_inputs` sends every input as an override,
/// and in a real claim set some inputs legitimately are not model symbols --
/// a material property read only by an in-process claim, for instance.
/// Refusing would break perturbation; saying nothing would let a misspelling
/// read as "this input does not drive the claim", which is true for the wrong
/// reason.
pub fn unwarned_overrides(reply: &Value, sent: &BTreeMap<String, f64>) -> Vec<String> {
    let warnings = reply
        .get("warnings")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    sent.keys()
        .filter(|n| !warnings.contains(&format!("\"{n}\"")))
        .cloned()
        .collect()
}

/// An [`Evaluator`] that computes a claim in the model's scope through an
/// injected transport.
///
/// The transport is handed the expression source and the variable overrides,
/// and returns the provider's reply. It is a `FnMut` because a real one owns a
/// child process and a request mutates it.
pub struct ScadEval<T> {
    send: std::cell::RefCell<T>,
    /// Override names the provider never reported overwriting anything for,
    /// accumulated across every call. See [`unwarned_overrides`].
    unwarned: std::cell::RefCell<std::collections::BTreeSet<String>>,
}

impl<T> ScadEval<T>
where
    T: FnMut(&str, &BTreeMap<String, f64>) -> Result<Value, String>,
{
    pub fn new(send: T) -> Self {
        ScadEval {
            send: std::cell::RefCell::new(send),
            unwarned: std::cell::RefCell::new(std::collections::BTreeSet::new()),
        }
    }

    /// Every override name the provider never reported overwriting a
    /// file-scope assignment for, across every evaluation so far.
    ///
    /// A CALLER THAT IGNORES THIS GETS THE FOUNDING DEFECT BACK: a misspelled
    /// input reads as one the claim does not depend on.
    pub fn unwarned(&self) -> Vec<String> {
        self.unwarned.borrow().iter().cloned().collect()
    }
}

impl<T> Evaluator for ScadEval<T>
where
    T: FnMut(&str, &BTreeMap<String, f64>) -> Result<Value, String>,
{
    /// A RUST EXPRESSION IS STILL REFUSED HERE, which is the mirror of
    /// `RustOnly` refusing a scad one. This evaluator is for claims that live
    /// in the model; running an in-process closure through it would report a
    /// value measured at f64 precision under a source recorded as six-digit,
    /// and `Index` would then carry an elasticity floor the answer did not
    /// actually have.
    fn eval(&self, expression: &Expression, inputs: &Inputs) -> Result<f64, String> {
        let source = match expression {
            Expression::Scad { source } => source,
            Expression::Rust(_) => {
                return Err(
                    "this evaluator computes claims in the model's scope and was \
                            handed an in-process expression. Evaluating it here would \
                            record a six-digit source for an f64 measurement"
                        .into(),
                )
            }
        };
        let vars = overrides(inputs);
        let reply = (self.send.borrow_mut())(source, &vars)?;
        // NOTED BEFORE THE VALUE IS READ, so a refusal below does not lose it.
        for name in unwarned_overrides(&reply, &vars) {
            self.unwarned.borrow_mut().insert(name);
        }
        value_of(&reply, source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim::{verify_with, Claim, Input, Provenance, Status};
    use crate::index::{Index, Source};

    /// The reply shape MEASURED against the pinned provider, including the
    /// override warning that perturbation causes.
    fn reply(expr: &str, value: Value, kind: &str) -> Value {
        serde_json::json!({
            "success": true, "errors": [],
            "warnings": ["WARNING: \"can_dia\" was assigned on line 38 but was overwritten"],
            "results": [{"index": 0, "expression": expr, "value": value,
                         "type": kind, "evaluated": true}]
        })
    }

    fn claim_scad(name: &str, source: &str, declared: &[&str]) -> Claim {
        Claim {
            name: name.into(),
            declared_inputs: declared.iter().map(|s| (*s).to_string()).collect(),
            range: (0.0, 1000.0),
            unit: "mm".into(),
            because: "a test".into(),
            expression: Expression::Scad {
                source: source.into(),
            },
        }
    }

    fn harness() -> Inputs {
        Inputs::new()
            .with("can_dia", Input::measured(66.0, "the model"))
            .with("can_clear", Input::measured(0.6, "the model"))
    }

    #[test]
    fn a_number_in_the_models_scope_is_the_claims_value() {
        let ev = ScadEval::new(|expr: &str, vars: &BTreeMap<String, f64>| {
            // bore_r = can_dia/2 + can_clear, the real harness relation
            let v = vars["can_dia"] / 2.0 + vars["can_clear"];
            Ok(reply(expr, serde_json::json!(v), "number"))
        });
        let c = claim_scad("bore_r", "bore_r", &["can_dia", "can_clear"]);
        let v = verify_with(&c, &harness(), &ev);
        assert_eq!(v.status, Status::Pass);
        assert_eq!(v.value, Some(33.6));
    }

    #[test]
    fn every_input_is_sent_as_an_override_not_only_the_declared_ones() {
        // The declaration is exactly what goes stale, so perturbation cannot
        // be limited to it.
        let i = harness().with("undeclared", Input::assumed(7.0, "nowhere"));
        let o = overrides(&i);
        assert_eq!(o.len(), 3);
        assert_eq!(o["undeclared"], 7.0);
    }

    #[test]
    fn perturbation_through_the_transport_finds_the_input_that_drives_the_claim() {
        let ev = ScadEval::new(|expr: &str, vars: &BTreeMap<String, f64>| {
            let v = vars["can_dia"] / 2.0 + vars["can_clear"];
            Ok(reply(expr, serde_json::json!(v), "number"))
        });
        let c = claim_scad("bore_r", "bore_r", &["can_dia"]);
        let used = crate::claim::derived_inputs_with(&c, &harness(), &ev);
        assert!(used.contains(&"can_dia".to_string()), "{used:?}");
        assert!(used.contains(&"can_clear".to_string()), "{used:?}");
        // AND THE DECLARATION GAP NAMES THE ONE NOBODY DECLARED.
        let gap = crate::claim::declaration_gap_with(&c, &harness(), &ev);
        assert_eq!(gap.undeclared, vec!["can_clear".to_string()]);
    }

    #[test]
    fn an_index_over_scad_claims_records_the_coarser_source() {
        let ev = ScadEval::new(|expr: &str, vars: &BTreeMap<String, f64>| {
            Ok(reply(
                expr,
                serde_json::json!(vars["can_dia"] / 2.0),
                "number",
            ))
        });
        let c = [claim_scad("can_r", "can_r", &["can_dia"])];
        let ix = Index::build_with(&c, &harness(), &ev, Source::ScadEval).expect("one source");
        assert_eq!(ix.source(), Source::ScadEval);
        assert!(ix.reads("can_r").expect("indexed").contains("can_dia"));
    }

    // -- THE REFUSALS. Each is a way a non-number becomes a number.

    #[test]
    fn undef_is_refused_rather_than_read_as_zero() {
        // OpenSCAD's absent value. As a 0.0 it passes every range containing
        // zero, which is every range this project writes with a lower bound of 0.
        let ev = ScadEval::new(|expr: &str, _: &BTreeMap<String, f64>| {
            Ok(reply(expr, Value::Null, "undef"))
        });
        let v = verify_with(&claim_scad("x", "spring_t", &[]), &harness(), &ev);
        match v.status {
            Status::Refused(ref why) => {
                assert!(why.contains("undef"), "{why}");
                assert!(why.contains("pass every range containing zero"), "{why}");
            }
            other => panic!("undef must refuse, got {other:?}"),
        }
        assert!(v.value.is_none());
    }

    #[test]
    fn a_vector_is_refused_because_a_range_cannot_judge_it() {
        let ev = ScadEval::new(|expr: &str, _: &BTreeMap<String, f64>| {
            Ok(reply(expr, serde_json::json!([1.0, 2.0, 3.0]), "vector"))
        });
        let err = ev
            .eval(
                &Expression::Scad {
                    source: "size".into(),
                },
                &harness(),
            )
            .expect_err("a vector is not a quantity");
        assert!(err.contains("not a number"), "{err}");
    }

    #[test]
    fn an_unevaluated_expression_is_refused() {
        let ev = ScadEval::new(|expr: &str, _: &BTreeMap<String, f64>| {
            let mut r = reply(expr, serde_json::json!(1.0), "number");
            r["results"][0]["evaluated"] = serde_json::json!(false);
            Ok(r)
        });
        let err = ev
            .eval(
                &Expression::Scad {
                    source: "whatever".into(),
                },
                &harness(),
            )
            .expect_err("not evaluated");
        assert!(err.contains("not evaluated"), "{err}");
    }

    #[test]
    fn a_failed_eval_reports_the_providers_own_errors() {
        let ev = ScadEval::new(|_: &str, _: &BTreeMap<String, f64>| {
            Ok(serde_json::json!({"success": false,
                                  "errors": ["unknown variable spring_t"], "results": []}))
        });
        let err = ev
            .eval(
                &Expression::Scad {
                    source: "spring_t".into(),
                },
                &harness(),
            )
            .expect_err("success=false");
        assert!(err.contains("unknown variable spring_t"), "{err}");
    }

    #[test]
    fn an_absent_results_array_is_refused_rather_than_read_as_empty() {
        let err =
            value_of(&serde_json::json!({"success": true}), "bore_r").expect_err("no results key");
        assert!(err.contains("not an empty one"), "{err}");
    }

    #[test]
    fn a_reply_about_a_different_expression_is_refused() {
        let r = reply("can_r", serde_json::json!(33.0), "number");
        let err = value_of(&r, "bore_r").expect_err("answers a different question");
        assert!(err.contains("different question"), "{err}");
    }

    #[test]
    fn this_evaluator_refuses_an_in_process_expression() {
        // The mirror of RustOnly refusing a scad one: evaluating an f64
        // closure here would record a six-digit source for an f64 measurement.
        fn one(_: &Inputs) -> Result<f64, String> {
            Ok(1.0)
        }
        let ev = ScadEval::new(|expr: &str, _: &BTreeMap<String, f64>| {
            Ok(reply(expr, serde_json::json!(1.0), "number"))
        });
        let err = ev
            .eval(&Expression::Rust(one), &harness())
            .expect_err("wrong evaluator for a pure expression");
        assert!(err.contains("six-digit source"), "{err}");
    }

    #[test]
    fn a_transport_that_cannot_reach_the_provider_refuses() {
        let ev = ScadEval::new(|_: &str, _: &BTreeMap<String, f64>| {
            Err("the provider exited before answering".to_string())
        });
        let v = verify_with(&claim_scad("x", "bore_r", &[]), &harness(), &ev);
        assert!(
            matches!(v.status, Status::Refused(ref w) if w.contains("exited")),
            "{v:?}"
        );
    }

    #[test]
    fn provenance_still_reaches_a_scad_claim() {
        // The weakest input must still be named: a claim evaluated in the
        // model's scope is not thereby better evidenced.
        let i = harness().with("guess", Input::assumed(1.0, "nobody measured this"));
        let ev = ScadEval::new(|expr: &str, vars: &BTreeMap<String, f64>| {
            Ok(reply(
                expr,
                serde_json::json!(vars["can_dia"] / 2.0 + vars["guess"]),
                "number",
            ))
        });
        let v = verify_with(&claim_scad("x", "bore_r", &[]), &i, &ev);
        assert_eq!(v.status, Status::Pass);
        assert_eq!(
            v.weakest_input.as_ref().map(|(n, _)| n.as_str()),
            Some("guess")
        );
        assert_eq!(v.weakest_input.map(|(_, p)| p), Some(Provenance::Assumed));
    }
}

#[cfg(test)]
mod unwarned_tests {
    use super::*;
    use crate::claim::{Claim, Input};

    /// THE WARNING TEXT AS THE PINNED PROVIDER EMITS IT, measured.
    fn with_warnings(expr: &str, value: f64, warned: &[&str]) -> Value {
        serde_json::json!({
            "success": true, "errors": [],
            "warnings": warned.iter().map(|n| format!(
                "WARNING: \"{n}\" was assigned on line 38 but was overwritten in file \
                 can_cutter.scad, line 1232")).collect::<Vec<_>>(),
            "results": [{"index": 0, "expression": expr, "value": value,
                         "type": "number", "evaluated": true}]
        })
    }

    #[test]
    fn an_override_the_provider_warned_about_reached_something() {
        let sent = BTreeMap::from([("can_dia".to_string(), 66.0)]);
        let r = with_warnings("bore_r", 33.6, &["can_dia"]);
        assert!(unwarned_overrides(&r, &sent).is_empty());
    }

    #[test]
    fn an_override_with_no_warning_is_named_because_it_reached_nothing() {
        // MEASURED against the real harness: `can_clear` is not a symbol in
        // can_cutter.scad -- it is `can_clearance` -- and the provider
        // returned no warning and the same value. That is the `-D`
        // misspelling, which is spec §01's opening incident.
        let sent = BTreeMap::from([
            ("can_dia".to_string(), 66.0),
            ("can_clear".to_string(), 0.6),
        ]);
        let r = with_warnings("bore_r", 33.6, &["can_dia"]);
        assert_eq!(unwarned_overrides(&r, &sent), vec!["can_clear".to_string()]);
    }

    #[test]
    fn the_evaluator_accumulates_them_across_every_perturbation() {
        let ev = ScadEval::new(|expr: &str, vars: &BTreeMap<String, f64>| {
            // only can_dia exists in this pretend model
            let v = vars["can_dia"] / 2.0 + 0.6;
            Ok(with_warnings(expr, v, &["can_dia"]))
        });
        let c = Claim {
            name: "bore_r".into(),
            declared_inputs: vec!["can_dia".into(), "can_clear".into()],
            range: (0.0, 100.0),
            unit: "mm".into(),
            because: "a test".into(),
            expression: Expression::Scad {
                source: "bore_r".into(),
            },
        };
        let inputs = Inputs::new()
            .with("can_dia", Input::measured(66.0, "the can"))
            .with(
                "can_clear",
                Input::measured(0.6, "a misspelling of can_clearance"),
            );
        let used = crate::claim::derived_inputs_with(&c, &inputs, &ev);

        // can_clear reads as not driving the claim -- which is TRUE and for
        // the WRONG REASON, and this is the arm that keeps those apart.
        assert!(!used.contains(&"can_clear".to_string()), "{used:?}");
        assert_eq!(ev.unwarned(), vec!["can_clear".to_string()]);
    }

    #[test]
    fn a_reply_with_no_warnings_key_names_every_override() {
        // Absence of the key is not evidence that every override landed.
        let sent = BTreeMap::from([("a".to_string(), 1.0), ("b".to_string(), 2.0)]);
        let r = serde_json::json!({"success": true,
            "results": [{"expression": "x", "value": 1.0, "type": "number", "evaluated": true}]});
        assert_eq!(
            unwarned_overrides(&r, &sent),
            vec!["a".to_string(), "b".to_string()]
        );
    }
}
