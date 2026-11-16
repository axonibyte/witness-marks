//! Where a number came from, and what that makes everything downstream.
//!
//! Spec §08. Four kinds -- measured, literature, assumed, derived -- on every
//! input, a derived value inheriting the weakest of its sources, and
//! `assumed` reported as assumed however many decimals it carries.
//!
//! # The decimals are the trap
//!
//! An assumed 3500 and a measured 3500 are the same eight characters. So is
//! an assumed 3487.2, which reads MORE trustworthy than the measured figure
//! because somebody clearly computed it. Nothing about a number's appearance
//! carries its provenance, which is why it has to travel beside the value and
//! why [`crate::claim::Verified`] reports the weakest input rather than
//! leaving a reader to go and look.
//!
//! # Literature is a range wearing a point's clothes
//!
//! PETG's modulus is 1700-2200 MPa. Every datasheet states one number inside
//! that, and every design that reads one is carrying +/-250 it did not ask
//! for. [`crate::claim::Input::literature`] therefore takes the half-width as
//! an argument rather than defaulting it: a literature value whose spread
//! nobody stated is not a literature value, it is an assumption with a
//! citation.

use crate::claim::{Claim, Input, Inputs, Provenance};
use crate::dominance::dominance;

/// A value computed from other values.
///
/// Its provenance is the WEAKEST of its sources, floored at `Derived`. A
/// figure derived from a literature modulus is no better than that modulus,
/// and saying `Derived` for it would promote it above what it stands on.
pub fn derived(value: f64, from: &[&Input], source: impl Into<String>) -> Result<Input, String> {
    if from.is_empty() {
        return Err(
            "a derived value with no sources is not derived; it is assumed, and \
                    calling it derived promotes it above everything it stands on"
                .into(),
        );
    }
    let weakest = from
        .iter()
        .map(|i| i.provenance)
        .min()
        .unwrap_or(Provenance::Assumed);
    // UNCERTAINTY PROPAGATES BY THE SAME RULE, conservatively: a derived value
    // is at least as uncertain as the worst of its sources, relatively. If no
    // source states one, neither does this -- an invented uncertainty is
    // worse than none, because it reads as knowledge.
    let rel = from
        .iter()
        .filter_map(|i| {
            i.uncertainty.map(|u| {
                if i.value == 0.0 {
                    0.0
                } else {
                    (u / i.value).abs()
                }
            })
        })
        .fold(None::<f64>, |acc, r| Some(acc.map_or(r, |a: f64| a.max(r))));
    Ok(Input {
        value,
        provenance: weakest.min(Provenance::Derived),
        uncertainty: rel.map(|r| (value * r).abs()),
        source: source.into(),
    })
}

/// What a claim is really standing on.
#[derive(Debug, Clone, PartialEq)]
pub struct Trust {
    pub claim: String,
    /// The worst provenance among the inputs the claim demonstrably reads.
    pub standing_on: Provenance,
    /// Which input that is.
    pub weakest: String,
    /// Inputs that are assumed outright, worst case first. These are the ones
    /// where a number exists and nobody has any basis for it.
    pub assumed: Vec<String>,
    /// The input to go and measure: largest slope x uncertainty.
    pub measure_this: Option<String>,
    /// How much that measurement could move the answer, in the claim's units.
    pub worth_mm: Option<f64>,
    /// The claim's range width, so `worth` can be read as a fraction of the
    /// margin rather than as a bare number.
    pub range_width: f64,
}

impl Trust {
    /// Is the uncertainty in one input comparable to the whole range the
    /// claim is allowed to move in? Then the claim is not really decided.
    pub fn uncertainty_dominates(&self) -> bool {
        match self.worth_mm {
            Some(w) => w >= self.range_width * 0.25,
            None => false,
        }
    }
}

/// Answer "what would I have to measure to trust this".
///
/// REFUSES A CLAIM THAT READS NOTHING. A claim standing on no inputs is a
/// constant, and reporting its provenance as anything at all implies a
/// dependency it does not have.
pub fn trust(claim: &Claim, inputs: &Inputs) -> Result<Trust, String> {
    let d = dominance(claim, inputs, None)?;
    if d.by_elasticity.is_empty() {
        return Err(format!(
            "`{}` reads no input that moves it. That is a constant wearing a claim's \
             clothes, and giving it a provenance implies a dependency it does not have",
            claim.name
        ));
    }
    let weakest = d
        .by_elasticity
        .iter()
        .min_by_key(|s| s.provenance)
        .ok_or_else(|| "no inputs".to_string())?;
    let assumed = d
        .by_elasticity
        .iter()
        .filter(|s| s.provenance == Provenance::Assumed)
        .map(|s| s.input.clone())
        .collect();
    let best = d.worth_measuring();
    let (lo, hi) = claim.range;
    Ok(Trust {
        claim: claim.name.clone(),
        standing_on: weakest.provenance,
        weakest: weakest.input.clone(),
        assumed,
        measure_this: best.map(|s| s.input.clone()),
        worth_mm: best.and_then(|s| s.contribution),
        range_width: hi - lo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wm_mech::{Beam, End};

    fn clip() -> Claim {
        Claim {
            name: "clip_clamps".into(),
            declared_inputs: vec!["E".into(), "spring_t".into(), "spring_pre".into()],
            range: (2.0, 5.0),
            unit: "N".into(),
            because: "the harness".into(),
            eval: |i| {
                Ok(Beam {
                    e_mpa: i.need("E")?,
                    l_mm: 8.9,
                    w_mm: 10.0,
                    t_mm: i.need("spring_t")?,
                    end: End::Cantilever,
                }
                .at(i.need("spring_pre")?)?
                .force)
            },
        }
    }

    /// PETG'S MODULUS IS A RANGE, 1700-2200 MPa, and a design reading 2000
    /// carries +/-250 it never asked for. Spec §08's own example.
    #[test]
    fn a_literature_modulus_carries_the_spread_nobody_wrote_down() {
        let e = Input::literature(2000.0, 250.0, "PETG datasheet, 1700-2200 MPa");
        assert_eq!(e.provenance, Provenance::Literature);
        assert_eq!(e.uncertainty, Some(250.0));
        assert!(e.source.contains("1700-2200"));
    }

    /// A DERIVED VALUE INHERITS THE WEAKEST SOURCE. Derived from a literature
    /// modulus, it is literature -- not derived, which would promote it above
    /// what it stands on.
    #[test]
    fn a_value_derived_from_a_literature_input_is_no_better_than_literature() {
        let e = Input::literature(2000.0, 250.0, "datasheet");
        let t = Input::measured(0.9, "calipers");
        let k = derived(5.17, &[&e, &t], "3EI/L^3").expect("valid");
        assert_eq!(k.provenance, Provenance::Literature);
    }

    /// AND FROM AN ASSUMED ONE IT IS ASSUMED, which is the case that matters:
    /// one unfounded number quietly makes everything downstream unfounded.
    #[test]
    fn one_assumed_source_makes_the_whole_chain_assumed() {
        let a = Input::assumed(0.2, "nobody measured the slot");
        let m = Input::measured(0.9, "calipers");
        let k = derived(1.0, &[&m, &a], "a chain").expect("valid");
        assert_eq!(k.provenance, Provenance::Assumed);
        assert_eq!(k.provenance.label(), "assumed");
    }

    /// DERIVED FROM ALL-MEASURED IS STILL `Derived`, NOT `Measured`. Nobody
    /// measured the derived value itself; a model stands between it and the
    /// world.
    #[test]
    fn derived_from_measurements_is_derived_and_not_measured() {
        let a = Input::measured(1.0, "calipers");
        let b = Input::measured(2.0, "calipers");
        assert_eq!(
            derived(3.0, &[&a, &b], "a+b").expect("valid").provenance,
            Provenance::Derived
        );
    }

    /// RELATIVE UNCERTAINTY PROPAGATES, conservatively, by the worst source.
    #[test]
    fn a_derived_value_carries_the_worst_relative_spread_of_its_sources() {
        let e = Input::literature(2000.0, 250.0, "datasheet"); // 12.5%
        let t = Input::measured(0.9, "calipers");
        let k = derived(4.0, &[&e, &t], "x").expect("valid");
        let u = k.uncertainty.expect("propagated");
        assert!((u - 4.0 * 0.125).abs() < 1e-12, "got {u}");
    }

    /// AND WHERE NO SOURCE STATES ONE, NEITHER DOES THE RESULT. An invented
    /// uncertainty is worse than none, because it reads as knowledge.
    #[test]
    fn no_source_uncertainty_means_no_derived_uncertainty() {
        let a = Input::measured(1.0, "calipers");
        assert_eq!(derived(2.0, &[&a], "x").expect("valid").uncertainty, None);
    }

    /// A DERIVED VALUE WITH NO SOURCES IS AN ASSUMPTION WITH A NICER NAME.
    #[test]
    fn deriving_from_nothing_is_refused() {
        let e = derived(1.0, &[], "thin air").expect_err("must refuse");
        assert!(e.contains("it is assumed"), "{e}");
    }

    /// WHAT WOULD I MEASURE: the modulus, and it is worth ~0.23 N on a claim
    /// whose whole allowed range is 3 N wide.
    #[test]
    fn the_trust_report_names_the_measurement_worth_taking() {
        let inputs = Inputs::new()
            .with("E", Input::literature(3500.0, 250.0, "PLA datasheet"))
            .with("spring_t", Input::measured(0.9, "calipers"))
            .with("spring_pre", Input::measured(0.350, "the model"));
        let t = trust(&clip(), &inputs).expect("valid");
        assert_eq!(t.standing_on, Provenance::Literature);
        assert_eq!(t.weakest, "E");
        assert_eq!(t.measure_this.as_deref(), Some("E"));
        assert!((t.worth_mm.expect("a figure") - 0.2265).abs() < 0.01);
        assert_eq!(t.range_width, 3.0);
        assert!(t.assumed.is_empty());
    }

    /// AN ASSUMED INPUT IS NAMED AS ASSUMED however many decimals it carries.
    /// 3487.2 reads MORE trustworthy than 3500 and is worth less.
    #[test]
    fn an_assumed_input_with_convincing_decimals_is_still_assumed() {
        let inputs = Inputs::new()
            .with("E", Input::assumed(3487.2, "somebody's spreadsheet"))
            .with("spring_t", Input::measured(0.9, "calipers"))
            .with("spring_pre", Input::measured(0.350, "the model"));
        let t = trust(&clip(), &inputs).expect("valid");
        assert_eq!(t.standing_on, Provenance::Assumed);
        assert_eq!(t.assumed, vec!["E".to_string()]);
        assert_eq!(t.standing_on.label(), "assumed");
    }

    /// WHEN THE UNCERTAINTY IS A QUARTER OF THE WHOLE RANGE, the claim is not
    /// really decided -- and that is a different statement from "it passes".
    #[test]
    fn an_uncertainty_comparable_to_the_range_says_the_claim_is_undecided() {
        let wide = Inputs::new()
            .with("E", Input::literature(3500.0, 1200.0, "a very wide range"))
            .with("spring_t", Input::measured(0.9, "calipers"))
            .with("spring_pre", Input::measured(0.350, "the model"));
        let t = trust(&clip(), &wide).expect("valid");
        assert!(t.uncertainty_dominates(), "worth {:?} of 3.0", t.worth_mm);
    }

    /// And a tight one does not, so the test is about the width.
    #[test]
    fn a_tight_uncertainty_leaves_the_claim_decided() {
        let tight = Inputs::new()
            .with("E", Input::literature(3500.0, 20.0, "a tight range"))
            .with("spring_t", Input::measured(0.9, "calipers"))
            .with("spring_pre", Input::measured(0.350, "the model"));
        assert!(!trust(&clip(), &tight)
            .expect("valid")
            .uncertainty_dominates());
    }

    /// A CLAIM READING NOTHING is a constant wearing a claim's clothes.
    #[test]
    fn a_claim_that_reads_no_input_is_refused() {
        let k = Claim {
            name: "constant".into(),
            declared_inputs: vec![],
            range: (0.0, 10.0),
            unit: "x".into(),
            because: "test".into(),
            eval: |_| Ok(5.0),
        };
        let e = trust(&k, &Inputs::new().with("a", Input::measured(1.0, "x")))
            .expect_err("must refuse");
        assert!(e.contains("constant wearing a claim's clothes"), "{e}");
    }
}
