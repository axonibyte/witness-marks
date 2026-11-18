//! Which inputs reached which claim, in one place (issue 37).
//!
//! # Why an index rather than three derivations
//!
//! [`crate::stale`], [`crate::dominance`] and [`crate::fanout`] all need the
//! same fact -- for each claim, the set of inputs that actually reached it --
//! and each of them re-derived it. That costs the same perturbation sweep
//! three times, but the expense is not the reason this exists. The reason is
//! that there was no single place the answer could be CHECKED.
//!
//! Issue 52 wants this index sourced from the provider's `trace` as well as
//! from perturbation, so the two can be compared. Three private derivations
//! cannot be compared against anything; one index can.
//!
//! # What `trace` actually is, measured before building on it
//!
//! Issues 37 and 52 both say "provider `trace`" as though it were a tool. It
//! is not: it is `get_project_files(mode="trace", symbol=, direction=)`, and
//! its own description bounds it -- **"lexical, file-scope constants only"**.
//! So the two sources answer different questions:
//!
//! * perturbation is BEHAVIORAL -- it moves an input and sees whether the
//!   claim's value moves, which is what the claim demonstrably reads;
//! * `trace` is LEXICAL -- it walks file-scope constant definitions.
//!
//! That bounds issue 52's stated reading of a disagreement in both
//! directions, and it is recorded here because building on the unbounded
//! version would manufacture findings:
//!
//! * `trace` sees it and perturbation does not -> NOT necessarily dead. It is
//!   lexically depended upon and behaviorally inert, which is also what a
//!   symbol used only inside an `assert` or an untaken branch looks like.
//! * perturbation sees it and `trace` does not -> NOT necessarily a hole in
//!   `trace`. Anything that is not a file-scope constant -- a module
//!   parameter, a local, a `$`-variable -- is outside what `trace` claims to
//!   see, and calling that a hole would be reading its answer as one to a
//!   question it never took.
//!
//! The comparison is still worth having. It just reports a DISAGREEMENT and
//! names both sources, rather than ruling on which one is wrong.

use crate::claim::{derived_inputs, Claim, Inputs};
use std::collections::{BTreeMap, BTreeSet};

/// Where an index's edges came from. Carried, because an edge from a lexical
/// walk and an edge from a measured perturbation are not the same claim about
/// the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Measured: the input was moved and the claim's value moved.
    Perturbation,
}

/// For each claim, the inputs that reached it -- and the inverse.
#[derive(Debug, Clone, PartialEq)]
pub struct Index {
    source: Source,
    reads: BTreeMap<String, BTreeSet<String>>,
    served_by: BTreeMap<String, BTreeSet<String>>,
    inert: Vec<String>,
}

impl Index {
    /// The inputs this claim reached for, or `None` if the index has never
    /// heard of the claim.
    ///
    /// `None` AND AN EMPTY SET ARE DIFFERENT ANSWERS, which is why this does
    /// not hand back an empty set for an unknown name: "I have no record of
    /// that claim" and "that claim reads nothing" are the two things a caller
    /// most needs told apart, and an empty set reads as the second.
    pub fn reads(&self, claim: &str) -> Option<&BTreeSet<String>> {
        self.reads.get(claim)
    }

    /// The claims that read this symbol. Empty is a real answer here: nothing
    /// in the set depends on it.
    pub fn claims_reading(&self, symbol: &str) -> BTreeSet<String> {
        self.served_by.get(symbol).cloned().unwrap_or_default()
    }

    /// Claims no input moved. Not an error and not an empty answer: a claim
    /// whose value no input can move is a CONSTANT WEARING A CLAIM'S CLOTHES,
    /// and it will pass every staleness check for ever while demonstrating
    /// nothing. Reported rather than refused, because the set it belongs to
    /// is otherwise usable.
    pub fn inert(&self) -> &[String] {
        &self.inert
    }

    pub fn source(&self) -> Source {
        self.source
    }

    pub fn claims(&self) -> impl Iterator<Item = &String> {
        self.reads.keys()
    }

    /// Build the index by perturbation over a claim set.
    ///
    /// REFUSES A DUPLICATE CLAIM NAME. The index is keyed by name, so two
    /// claims called the same thing would silently merge into one entry whose
    /// input set belongs to neither -- and every consumer would then read a
    /// union as though it were one claim's dependencies.
    ///
    /// REFUSES AN EMPTY CLAIM SET. An index over no claims answers
    /// `claims_reading` with the empty set for every symbol, which is
    /// indistinguishable from "nothing depends on this" -- the answer fanout
    /// exists to produce. Nothing to index is not the same as nothing
    /// depending on anything.
    pub fn build(claims: &[Claim], inputs: &Inputs) -> Result<Index, String> {
        if claims.is_empty() {
            return Err(
                "an index over no claims answers `nothing depends on this` for \
                        every symbol, which is not the same as having nothing to index"
                    .into(),
            );
        }
        let mut reads: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut served_by: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut inert = Vec::new();
        for claim in claims {
            if reads.contains_key(&claim.name) {
                return Err(format!(
                    "two claims are named `{}`; an index keyed by name would merge them \
                     into one entry whose input set belongs to neither",
                    claim.name
                ));
            }
            let used: BTreeSet<String> = derived_inputs(claim, inputs).into_iter().collect();
            if used.is_empty() {
                inert.push(claim.name.clone());
            }
            for symbol in &used {
                served_by
                    .entry(symbol.clone())
                    .or_default()
                    .insert(claim.name.clone());
            }
            reads.insert(claim.name.clone(), used);
        }
        Ok(Index {
            source: Source::Perturbation,
            reads,
            served_by,
            inert,
        })
    }
}

/// WHY `dominance` IS NOT IN THIS LIST, correcting issue 37's phrasing. The
/// issue says "the index that staleness, dominance and fanout all read", and
/// two of those three are right. [`crate::dominance`] is the INSTRUMENT the
/// index is built from, not a consumer of it: it perturbs one claim against
/// one input and reports an elasticity, and the index records which of those
/// came back non-zero. Giving it the index to read would be circular, and
/// saying it reads one would describe a dependency that runs the other way.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim::{Input, Provenance};
    use std::collections::BTreeMap;

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

    fn claim(name: &str, declared: &[&str], eval: fn(&Inputs) -> Result<f64, String>) -> Claim {
        Claim {
            name: name.into(),
            declared_inputs: declared.iter().map(|s| (*s).to_string()).collect(),
            range: (-1.0e9, 1.0e9),
            unit: "N".into(),
            because: "a test".into(),
            eval,
        }
    }

    fn reads_a(i: &Inputs) -> Result<f64, String> {
        Ok(i.need("a")? * 2.0)
    }
    fn reads_b(i: &Inputs) -> Result<f64, String> {
        Ok(i.need("b")? + 1.0)
    }
    fn reads_both(i: &Inputs) -> Result<f64, String> {
        Ok(i.need("a")? + i.need("b")?)
    }
    fn reads_nothing(_: &Inputs) -> Result<f64, String> {
        Ok(7.0)
    }

    #[test]
    fn the_index_records_what_each_claim_reached_for() {
        let i = inputs(&[("a", 2.0), ("b", 3.0)]);
        let ix = Index::build(
            &[claim("x", &["a"], reads_a), claim("y", &["b"], reads_b)],
            &i,
        )
        .expect("two distinct claims");
        assert_eq!(ix.reads("x").expect("x is indexed").iter().count(), 1);
        assert!(ix.reads("x").expect("x").contains("a"));
        assert!(ix.reads("y").expect("y").contains("b"));
        assert_eq!(ix.source(), Source::Perturbation);
    }

    #[test]
    fn the_inverse_answers_which_claims_read_a_symbol() {
        let i = inputs(&[("a", 2.0), ("b", 3.0)]);
        let ix = Index::build(
            &[
                claim("x", &["a"], reads_a),
                claim("y", &["b"], reads_b),
                claim("z", &["a", "b"], reads_both),
            ],
            &i,
        )
        .expect("three claims");
        let on_a = ix.claims_reading("a");
        assert!(
            on_a.contains("x") && on_a.contains("z") && !on_a.contains("y"),
            "{on_a:?}"
        );
    }

    #[test]
    fn an_unknown_claim_is_none_and_a_claim_reading_nothing_is_an_empty_set() {
        // THE TWO ANSWERS A CALLER MUST BE ABLE TO TELL APART.
        let i = inputs(&[("a", 2.0)]);
        let ix = Index::build(
            &[claim("x", &["a"], reads_a), claim("k", &[], reads_nothing)],
            &i,
        )
        .expect("built");
        assert!(ix.reads("never-heard-of-it").is_none());
        assert_eq!(ix.reads("k").expect("k is indexed").len(), 0);
    }

    #[test]
    fn a_claim_no_input_moves_is_reported_as_inert_rather_than_refused() {
        let i = inputs(&[("a", 2.0)]);
        let ix = Index::build(
            &[claim("x", &["a"], reads_a), claim("k", &[], reads_nothing)],
            &i,
        )
        .expect("an inert claim does not spoil the set");
        assert_eq!(ix.inert(), ["k".to_string()]);
        // and the usable half is still usable
        assert!(ix.claims_reading("a").contains("x"));
    }

    #[test]
    fn a_duplicate_claim_name_is_refused_rather_than_merged() {
        let i = inputs(&[("a", 2.0), ("b", 3.0)]);
        let err = Index::build(
            &[claim("x", &["a"], reads_a), claim("x", &["b"], reads_b)],
            &i,
        )
        .expect_err("two claims named x");
        assert!(
            err.contains("named `x`") && err.contains("belongs to neither"),
            "{err}"
        );
    }

    #[test]
    fn an_empty_claim_set_is_refused_rather_than_indexed() {
        let err = Index::build(&[], &inputs(&[("a", 1.0)]))
            .expect_err("nothing to index is not nothing depending on anything");
        assert!(
            err.contains("not the same as having nothing to index"),
            "{err}"
        );
    }

    #[test]
    fn the_index_is_a_fact_about_the_input_set_it_was_built_from() {
        // THE HAZARD stale_with's signature exists for. The same claim reads
        // different inputs under different sets, so an index is not portable
        // between them.
        fn branch(i: &Inputs) -> Result<f64, String> {
            if i.need("switch")? > 0.0 {
                Ok(i.need("a")? * 2.0)
            } else {
                Ok(i.need("b")? * 2.0)
            }
        }
        let on = inputs(&[("switch", 1.0), ("a", 2.0), ("b", 3.0)]);
        let off = inputs(&[("switch", -1.0), ("a", 2.0), ("b", 3.0)]);
        let c = [claim("w", &["a", "b"], branch)];
        let ix_on = Index::build(&c, &on).expect("on");
        let ix_off = Index::build(&c, &off).expect("off");
        assert!(ix_on.reads("w").expect("w").contains("a"));
        assert!(ix_off.reads("w").expect("w").contains("b"));
        assert_ne!(ix_on.reads("w"), ix_off.reads("w"));
    }
}
