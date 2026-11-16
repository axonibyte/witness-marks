//! One symbol, several unrelated jobs.
//!
//! Spec §05 Layer 5. The dependency index read BACKWARDS: not "what does this
//! claim consume" but "what does this symbol serve". The harness had the same
//! defect twice in one month, and the log caught its own recurrence.
//!
//! # buildability 70
//!
//! `thr_clear` was the thread clearance. It was also the rib height, and the
//! carrier centring. Raising it to make the thread run freely pushed the
//! retention NEGATIVE by 0.10 mm -- the snap stopped retaining -- and every
//! part still looked right in a render, because each individual use of the
//! symbol was reasonable.
//!
//! # Relatedness is MEASURED, not declared
//!
//! "Flagged when they have no physical relationship" needs relatedness to
//! mean something a machine can check. A declared taxonomy -- tagging claims
//! by domain -- would do it, and would be exactly as stale as the declared
//! dependency lists this layer already refuses to trust.
//!
//! So the test is the SIGN of each claim's elasticity with respect to the
//! symbol. A symbol that one claim wants larger and another wants smaller is
//! a symbol doing two jobs that FIGHT, whatever either job is called. That is
//! measurable from the claims themselves, needs no vocabulary, and is the
//! condition that actually hurts: raising `thr_clear` for the thread is the
//! same act as lowering the retention.
//!
//! Two claims pulling the same way are reported too, because a symbol serving
//! several chains is worth knowing about even when they agree -- but only the
//! opposed ones are a finding.

use crate::claim::{Claim, Inputs, PROBE_FRACTION};
use crate::dominance::dominance;

/// What one claim does with the symbol.
#[derive(Debug, Clone, PartialEq)]
pub struct Serves {
    pub claim: String,
    /// The claim's current value, so a reader can see how close to its edge
    /// the conflict is biting.
    pub value: f64,
    pub elasticity: f64,
    pub unit: String,
    /// How much room is left before this claim leaves its range, in the
    /// claim's own units. Negative means it is already out.
    pub margin: f64,
}

impl Serves {
    pub fn wants_larger(&self) -> bool {
        self.elasticity > 0.0
    }
}

/// Everything one symbol does.
#[derive(Debug, Clone, PartialEq)]
pub struct Fanout {
    pub symbol: String,
    pub serves: Vec<Serves>,
    /// Pairs that pull in opposite directions: moving the symbol to help one
    /// hurts the other. The finding.
    pub opposed: Vec<(String, String)>,
    /// The claim closest to leaving its range among those the symbol can
    /// hurt. Where the conflict bites first.
    pub bites_first: Option<String>,
}

impl Fanout {
    pub fn is_contested(&self) -> bool {
        !self.opposed.is_empty()
    }
}

/// Read the index backwards for one symbol.
///
/// REFUSES A SYMBOL NOTHING READS. "This symbol serves no claims" is a true
/// sentence about a symbol that does not exist and about one that twelve
/// claims consume through a name nobody declared, and only the first is
/// reassuring.
pub fn fanout(claims: &[Claim], inputs: &Inputs, symbol: &str) -> Result<Fanout, String> {
    if inputs.get(symbol).is_none() {
        return Err(format!(
            "no input named `{symbol}`. Reporting that it serves no claims would be true of \
             a symbol that does not exist and of one that is consumed under another name, \
             and only the first of those is reassuring"
        ));
    }
    let mut serves = Vec::new();
    for c in claims {
        // The elasticity comes from the same perturbation `dominance` uses,
        // so there is one account of how an input drives a claim and not two.
        let Ok(d) = dominance(c, inputs, None) else {
            continue;
        };
        let Some(s) = d.by_elasticity.iter().find(|s| s.input == symbol) else {
            continue;
        };
        let (lo, hi) = c.range;
        serves.push(Serves {
            claim: c.name.clone(),
            value: d.value,
            elasticity: s.elasticity,
            unit: c.unit.clone(),
            margin: (d.value - lo).min(hi - d.value),
        });
    }
    if serves.is_empty() {
        return Err(format!(
            "`{symbol}` is an input and no claim reads it. That is either a dead parameter \
             or a claim set that does not cover it, and this refuses rather than reporting \
             a clean fan-out over nothing"
        ));
    }
    serves.sort_by(|a, b| a.claim.cmp(&b.claim));

    let mut opposed = Vec::new();
    for (i, a) in serves.iter().enumerate() {
        for b in serves.iter().skip(i + 1) {
            if a.wants_larger() != b.wants_larger() {
                opposed.push((a.claim.clone(), b.claim.clone()));
            }
        }
    }
    // WHERE IT BITES FIRST: among the claims the symbol can push OUT of
    // range, the one with the least room. A conflict between two claims with
    // acres of margin is a note; one where a claim is 0.1 from its edge is
    // the print that fails.
    let bites_first = serves
        .iter()
        .min_by(|a, b| {
            a.margin
                .partial_cmp(&b.margin)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|s| s.claim.clone());

    Ok(Fanout {
        symbol: symbol.to_string(),
        serves,
        opposed,
        bites_first,
    })
}

/// Every symbol that more than one claim reads, contested ones first.
///
/// The sweep an agent runs once rather than guessing which symbol to ask
/// about -- the harness's two incidents were both symbols nobody suspected.
pub fn contested(claims: &[Claim], inputs: &Inputs) -> Vec<Fanout> {
    let mut out: Vec<Fanout> = inputs
        .names()
        .iter()
        .filter_map(|n| fanout(claims, inputs, n).ok())
        .filter(|f| f.serves.len() > 1)
        .collect();
    out.sort_by(|a, b| {
        b.opposed
            .len()
            .cmp(&a.opposed.len())
            .then(a.symbol.cmp(&b.symbol))
    });
    out
}

/// The step the perturbation uses, re-exported so a caller can state the
/// resolution a fan-out was computed at.
pub const PROBE: f64 = PROBE_FRACTION;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim::Input;

    fn c(
        name: &str,
        unit: &str,
        range: (f64, f64),
        eval: fn(&Inputs) -> Result<f64, String>,
    ) -> Claim {
        Claim {
            name: name.into(),
            declared_inputs: vec!["thr_clear".into()],
            range,
            unit: unit.into(),
            because: "buildability 70".into(),
            eval,
        }
    }

    /// BUILDABILITY 70. `thr_clear` is the thread clearance, the rib height
    /// and the carrier centring at once.
    ///
    /// - the thread runs more freely as it GROWS
    /// - the rib stands taller as it GROWS
    /// - the snap's retention SHRINKS as it grows, and at 0.45 it has gone
    ///   negative by 0.10 mm
    fn harness() -> Vec<Claim> {
        vec![
            c("thread_runs_free", "mm", (0.15, 0.60), |i| {
                i.need("thr_clear")
            }),
            c("rib_height", "mm", (0.5, 3.0), |i| {
                Ok(i.need("thr_clear")? * 2.0 + 0.6)
            }),
            c("snap_retains", "mm", (0.05, 1.00), |i| {
                Ok(0.35 - i.need("thr_clear")?)
            }),
        ]
    }

    fn at(v: f64) -> Inputs {
        Inputs::new()
            .with("thr_clear", Input::measured(v, "the model"))
            .with("unused_elsewhere", Input::assumed(1.0, "nothing"))
    }

    /// THE SYMBOL SERVES THREE CHAINS and two of them fight: raising it for
    /// the thread is the same act as lowering the retention.
    #[test]
    fn one_symbol_serving_three_chains_is_reported_with_the_conflict() {
        let f = fanout(&harness(), &at(0.30), "thr_clear").expect("valid");
        assert_eq!(f.serves.len(), 3);
        assert!(f.is_contested());
        // snap_retains opposes both of the others.
        assert_eq!(f.opposed.len(), 2, "{:?}", f.opposed);
        assert!(f
            .opposed
            .iter()
            .all(|(a, b)| a == "snap_retains" || b == "snap_retains"));
    }

    /// THE SIGNS ARE MEASURED, not declared. Nothing told this code that a
    /// snap and a thread want opposite things.
    #[test]
    fn the_directions_come_from_perturbation_and_not_from_a_taxonomy() {
        let f = fanout(&harness(), &at(0.30), "thr_clear").expect("valid");
        let g = |n: &str| {
            f.serves
                .iter()
                .find(|s| s.claim == n)
                .expect("present")
                .clone()
        };
        assert!(g("thread_runs_free").wants_larger());
        assert!(g("rib_height").wants_larger());
        assert!(
            !g("snap_retains").wants_larger(),
            "the snap wants it smaller"
        );
    }

    /// THE RETENTION IS WHERE IT BITES. At 0.30 the snap has 0.00 mm of room
    /// against its 0.05 floor, and the other two have plenty.
    #[test]
    fn the_claim_closest_to_its_edge_is_named() {
        let f = fanout(&harness(), &at(0.30), "thr_clear").expect("valid");
        assert_eq!(f.bites_first.as_deref(), Some("snap_retains"));
    }

    /// AND AT 0.45 THE RETENTION HAS GONE NEGATIVE BY 0.10 mm, which is
    /// buildability 70's number. Every part still renders correctly.
    #[test]
    fn at_the_logged_value_the_retention_is_negative_by_a_tenth() {
        let f = fanout(&harness(), &at(0.45), "thr_clear").expect("valid");
        let snap = f
            .serves
            .iter()
            .find(|s| s.claim == "snap_retains")
            .expect("present");
        assert!((snap.value - (-0.10)).abs() < 1e-12, "got {}", snap.value);
        assert!(snap.margin < 0.0, "already out of range");
        // and the thread is perfectly happy, which is why nobody looked
        let thread = f
            .serves
            .iter()
            .find(|s| s.claim == "thread_runs_free")
            .expect("present");
        assert!(thread.margin > 0.0, "the thread is fine at 0.45");
    }

    /// A SYMBOL NOTHING READS IS REFUSED, because "serves no claims" is true
    /// of a dead parameter and of one consumed under another name.
    #[test]
    fn a_symbol_no_claim_reads_is_refused() {
        let e = fanout(&harness(), &at(0.30), "unused_elsewhere").expect_err("must refuse");
        assert!(e.contains("dead parameter"), "{e}");
    }

    /// AND A SYMBOL THAT IS NOT AN INPUT AT ALL is a different refusal, so
    /// the two cannot be confused.
    #[test]
    fn a_symbol_that_is_not_an_input_is_a_different_refusal() {
        let e = fanout(&harness(), &at(0.30), "no_such_name").expect_err("must refuse");
        assert!(e.contains("does not exist"), "{e}");
    }

    /// A SYMBOL WHOSE USERS ALL AGREE IS NOT A FINDING. Several chains on one
    /// parameter is worth reporting; only the opposed ones are a problem.
    #[test]
    fn a_symbol_whose_chains_all_pull_the_same_way_is_reported_uncontested() {
        let agreeing = vec![
            c("a", "mm", (0.0, 10.0), |i| i.need("thr_clear")),
            c("b", "mm", (0.0, 10.0), |i| Ok(i.need("thr_clear")? * 3.0)),
        ];
        let f = fanout(&agreeing, &at(0.30), "thr_clear").expect("valid");
        assert_eq!(f.serves.len(), 2);
        assert!(!f.is_contested());
    }

    /// THE SWEEP finds the contested symbol without being asked about it,
    /// which is the point -- both harness incidents were symbols nobody
    /// suspected.
    #[test]
    fn the_sweep_surfaces_the_contested_symbol_unprompted() {
        let all = contested(&harness(), &at(0.30));
        assert_eq!(
            all.len(),
            1,
            "only thr_clear is read by more than one claim"
        );
        assert_eq!(all[0].symbol, "thr_clear");
        assert!(all[0].is_contested());
    }
}
