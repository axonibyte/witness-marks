//! A design is a test suite: named assertions that re-evaluate.
//!
//! Spec §06.1. The harness carried twelve comments stating a derived number --
//! spring rates, clamp forces, margins -- each computed once in a scratch
//! script and pasted in. When the material changed from PLA to PETG, one
//! sentence into a conversation, six of the twelve went wrong at once and
//! nothing in the file knew. A comment four lines above the parameter still
//! read `2.4x under yield`.
//!
//! # The dependency set is DERIVED, not declared
//!
//! A claim names the inputs it believes it uses. That declaration is exactly
//! the thing that goes stale: somebody adds a term, the list is not updated,
//! and staleness propagation then skips the claim that most needed it --
//! silently, because an incomplete list looks like a complete one.
//!
//! So [`derived_inputs`] finds the real set by PERTURBATION: move each input
//! and see whether the claim's value moves. What the claim declares is then
//! checked against what it demonstrably reads, and a mismatch is a finding in
//! both directions -- an input used and not declared is a staleness hole, and
//! one declared and not used is a claim carrying a dependency it does not
//! have.
//!
//! The same perturbation is what [`crate::dominance`] reads for sensitivity,
//! so there is one mechanism and not two accounts of which inputs matter.

use std::collections::{BTreeMap, BTreeSet};

/// Where a number came from. Spec §08.
///
/// Ordered worst to best so that "the weakest input" is a `min`, and so that
/// a derived claim inheriting the weakest of its inputs is one comparison
/// rather than a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Provenance {
    /// Nobody measured this and nobody cited it. The most dangerous kind,
    /// because it formats identically to the others.
    Assumed,
    /// A published figure, usually a RANGE presented as a point.
    Literature,
    /// Computed from other inputs; inherits the weakest of them.
    Derived,
    /// Someone measured it on the actual material or the actual printer.
    Measured,
}

impl Provenance {
    /// How it reads in a report. `Assumed` says so however many decimals the
    /// value carries -- that is the whole point of the type.
    pub fn label(self) -> &'static str {
        match self {
            Provenance::Assumed => "assumed",
            Provenance::Literature => "literature",
            Provenance::Derived => "derived",
            Provenance::Measured => "measured",
        }
    }
}

/// One named number and everything known about how much to trust it.
#[derive(Debug, Clone, PartialEq)]
pub struct Input {
    pub value: f64,
    pub provenance: Provenance,
    /// Half-width of the plausible range, in the value's own units. A
    /// literature figure quoted as a point has one even when nobody wrote it
    /// down: PETG's modulus is 1700-2200 MPa, so a stated 2000 carries +/-250.
    pub uncertainty: Option<f64>,
    pub source: String,
}

impl Input {
    pub fn measured(value: f64, source: impl Into<String>) -> Self {
        Input {
            value,
            provenance: Provenance::Measured,
            uncertainty: None,
            source: source.into(),
        }
    }
    pub fn literature(value: f64, half_width: f64, source: impl Into<String>) -> Self {
        Input {
            value,
            provenance: Provenance::Literature,
            uncertainty: Some(half_width),
            source: source.into(),
        }
    }
    pub fn assumed(value: f64, source: impl Into<String>) -> Self {
        Input {
            value,
            provenance: Provenance::Assumed,
            uncertainty: None,
            source: source.into(),
        }
    }
}

/// The named numbers a claim may read.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Inputs(pub BTreeMap<String, Input>);

impl Inputs {
    pub fn new() -> Self {
        Inputs(BTreeMap::new())
    }

    pub fn with(mut self, name: &str, input: Input) -> Self {
        self.0.insert(name.to_string(), input);
        self
    }

    /// REFUSES A NAME THAT IS NOT THERE rather than defaulting it.
    ///
    /// A missing input defaulting to zero makes a claim evaluate to something
    /// plausible from nothing, which is the failure mode this whole layer
    /// exists to end.
    pub fn need(&self, name: &str) -> Result<f64, String> {
        self.0
            .get(name)
            .map(|i| i.value)
            .ok_or_else(|| format!("no input named `{name}`; refusing rather than assuming one"))
    }

    pub fn get(&self, name: &str) -> Option<&Input> {
        self.0.get(name)
    }

    pub fn names(&self) -> Vec<String> {
        self.0.keys().cloned().collect()
    }

    /// The same inputs with one value moved. Used by perturbation.
    pub fn perturbed(&self, name: &str, to: f64) -> Inputs {
        let mut next = self.clone();
        if let Some(i) = next.0.get_mut(name) {
            i.value = to;
        }
        next
    }
}

/// A named assertion with a range, a unit and a reason.
pub struct Claim {
    pub name: String,
    /// What the author believes this reads. CHECKED, not trusted.
    pub declared_inputs: Vec<String>,
    /// Inclusive bounds the value must fall within.
    pub range: (f64, f64),
    pub unit: String,
    /// Why these bounds. Not decoration: a range without a reason cannot be
    /// re-judged when the design changes, so it becomes folklore the moment
    /// its author moves on.
    pub because: String,
    pub eval: fn(&Inputs) -> Result<f64, String>,
}

impl std::fmt::Debug for Claim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Claim")
            .field("name", &self.name)
            .field("declared_inputs", &self.declared_inputs)
            .field("range", &self.range)
            .field("unit", &self.unit)
            .finish()
    }
}

/// What a claim is doing now.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Pass,
    /// Outside its range, with the distance named.
    Fail {
        by: f64,
    },
    /// Could not be evaluated. Not a pass and not a failure.
    Refused(String),
}

impl Status {
    pub fn is_pass(&self) -> bool {
        matches!(self, Status::Pass)
    }
}

/// One claim, evaluated.
#[derive(Debug, Clone, PartialEq)]
pub struct Verified {
    pub name: String,
    pub value: Option<f64>,
    pub status: Status,
    /// The inputs it DEMONSTRABLY read, found by perturbation.
    pub inputs_used: Vec<String>,
    /// The worst provenance among those inputs: what the claim is really
    /// standing on.
    pub weakest_input: Option<(String, Provenance)>,
}

/// Evaluate one claim.
pub fn verify(claim: &Claim, inputs: &Inputs) -> Verified {
    let value = match (claim.eval)(inputs) {
        Ok(v) => v,
        Err(e) => {
            return Verified {
                name: claim.name.clone(),
                value: None,
                status: Status::Refused(e),
                inputs_used: Vec::new(),
                weakest_input: None,
            }
        }
    };
    if !value.is_finite() {
        return Verified {
            name: claim.name.clone(),
            value: None,
            status: Status::Refused(format!(
                "the claim evaluated to {value}, which is not a value"
            )),
            inputs_used: Vec::new(),
            weakest_input: None,
        };
    }
    let used = derived_inputs(claim, inputs);
    let weakest = used
        .iter()
        .filter_map(|n| inputs.get(n).map(|i| (n.clone(), i.provenance)))
        .min_by_key(|(_, p)| *p);
    let (lo, hi) = claim.range;
    let status = if value < lo {
        Status::Fail { by: lo - value }
    } else if value > hi {
        Status::Fail { by: value - hi }
    } else {
        Status::Pass
    };
    Verified {
        name: claim.name.clone(),
        value: Some(value),
        status,
        inputs_used: used,
        weakest_input: weakest,
    }
}

/// How far an input is moved to see whether a claim notices, as a fraction of
/// its own value. Small enough to stay in the linear neighbourhood, large
/// enough to clear floating-point noise in a chain of arithmetic.
pub const PROBE_FRACTION: f64 = 1e-4;

/// The inputs a claim DEMONSTRABLY reads, found by moving each one.
///
/// An input whose movement does not move the claim is not a dependency of it,
/// whatever the declaration says. An input that moves it IS one, whatever the
/// declaration says.
pub fn derived_inputs(claim: &Claim, inputs: &Inputs) -> Vec<String> {
    let base = match (claim.eval)(inputs) {
        Ok(v) if v.is_finite() => v,
        _ => return Vec::new(),
    };
    let mut used = Vec::new();
    for name in inputs.names() {
        let Some(input) = inputs.get(&name) else {
            continue;
        };
        // A ZERO-VALUED INPUT CANNOT BE PERTURBED PROPORTIONALLY, so it gets
        // an absolute step. Scaling a zero by any fraction leaves it at zero,
        // which would report every zero input as unused -- and a zero is
        // exactly the kind of value somebody passes without thinking.
        let step = if input.value == 0.0 {
            PROBE_FRACTION
        } else {
            input.value.abs() * PROBE_FRACTION
        };
        let moved = (claim.eval)(&inputs.perturbed(&name, input.value + step));
        match moved {
            Ok(v) if v.is_finite() => {
                if (v - base).abs() > base.abs() * 1e-12 + f64::MIN_POSITIVE {
                    used.push(name);
                }
            }
            // AN INPUT WHOSE MOVEMENT MAKES THE CLAIM REFUSE IS A DEPENDENCY,
            // and a sharp one: the claim is defined only on part of that
            // input's range.
            _ => used.push(name),
        }
    }
    used
}

/// What the claim says it reads against what it demonstrably reads.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclarationGap {
    /// Read and not declared. A staleness hole: a change to this input would
    /// not be propagated to this claim by anything reading the declaration.
    pub undeclared: Vec<String>,
    /// Declared and not read. The claim carries a dependency it does not
    /// have, so it will be re-checked for no reason -- and, worse, somebody
    /// will believe it depends on that input.
    pub unread: Vec<String>,
}

impl DeclarationGap {
    pub fn is_clean(&self) -> bool {
        self.undeclared.is_empty() && self.unread.is_empty()
    }
}

/// Compare the declaration against the perturbation.
pub fn declaration_gap(claim: &Claim, inputs: &Inputs) -> DeclarationGap {
    let used: BTreeSet<String> = derived_inputs(claim, inputs).into_iter().collect();
    let declared: BTreeSet<String> = claim.declared_inputs.iter().cloned().collect();
    DeclarationGap {
        undeclared: used.difference(&declared).cloned().collect(),
        unread: declared.difference(&used).cloned().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wm_mech::{Beam, End};

    /// THE HARNESS CLAIM, from spec §06.1. The blade clip's clamp force must
    /// sit in [2.0, 5.0] N: below 2 the blade lifts while cutting, above 5
    /// the clip is hard to seat and creeps under permanent load.
    fn clip_clamps() -> Claim {
        Claim {
            name: "clip_clamps".into(),
            declared_inputs: vec!["E".into(), "spring_t".into(), "spring_pre".into()],
            range: (2.0, 5.0),
            unit: "N".into(),
            because: "below 2 N the blade lifts while cutting; above 5 N the clip is hard \
                      to seat and creeps under permanent load"
                .into(),
            eval: |i| {
                let beam = Beam {
                    e_mpa: i.need("E")?,
                    l_mm: 8.9,
                    w_mm: 10.0,
                    t_mm: i.need("spring_t")?,
                    end: End::Cantilever,
                };
                Ok(beam.at(i.need("spring_pre")?)?.force)
            },
        }
    }

    fn pla() -> Inputs {
        Inputs::new()
            .with("E", Input::literature(3500.0, 250.0, "PLA datasheet"))
            .with("spring_t", Input::measured(0.9, "calipers"))
            .with("spring_pre", Input::measured(0.350, "the model"))
            // AN INPUT NOTHING READS, present on purpose: the perturbation
            // must not report it as a dependency just because it is nearby.
            .with("bed_temp", Input::assumed(60.0, "the slicer profile"))
    }

    #[test]
    fn the_harness_claim_passes_in_pla_at_the_logged_figure() {
        let v = verify(&clip_clamps(), &pla());
        assert_eq!(v.status, Status::Pass);
        let f = v.value.expect("a value");
        assert!((f - 3.17).abs() < 0.01, "spec §06 says 3.17 N, got {f}");
    }

    /// THE PERTURBATION FINDS THE REAL SET, and `bed_temp` is not in it even
    /// though it sits in the same input table.
    #[test]
    fn the_inputs_actually_read_are_found_by_moving_them() {
        let v = verify(&clip_clamps(), &pla());
        assert_eq!(
            v.inputs_used,
            vec![
                "E".to_string(),
                "spring_pre".to_string(),
                "spring_t".to_string()
            ]
        );
        assert!(
            !v.inputs_used.contains(&"bed_temp".to_string()),
            "bed_temp is in the table and is not read"
        );
    }

    /// THE DECLARATION IS CHECKED, and a correct one comes back clean.
    #[test]
    fn a_declaration_matching_what_is_read_is_clean() {
        assert!(declaration_gap(&clip_clamps(), &pla()).is_clean());
    }

    /// AN UNDECLARED INPUT IS A STALENESS HOLE: a change to it would not
    /// reach this claim through anything reading the declaration.
    #[test]
    fn an_input_read_but_not_declared_is_named() {
        let mut c = clip_clamps();
        c.declared_inputs = vec!["spring_t".into(), "spring_pre".into()];
        let gap = declaration_gap(&c, &pla());
        assert_eq!(gap.undeclared, vec!["E".to_string()]);
        assert!(gap.unread.is_empty());
        assert!(!gap.is_clean());
    }

    /// AND ONE DECLARED BUT NOT READ is named too, in the other direction.
    #[test]
    fn an_input_declared_but_not_read_is_named() {
        let mut c = clip_clamps();
        c.declared_inputs.push("bed_temp".into());
        let gap = declaration_gap(&c, &pla());
        assert_eq!(gap.unread, vec!["bed_temp".to_string()]);
        assert!(gap.undeclared.is_empty());
    }

    /// THE WEAKEST INPUT IS WHAT THE CLAIM STANDS ON. Two measured values and
    /// one literature value make it a literature claim, and `E` is named.
    #[test]
    fn the_weakest_input_among_those_read_is_reported() {
        let v = verify(&clip_clamps(), &pla());
        assert_eq!(
            v.weakest_input,
            Some(("E".to_string(), Provenance::Literature))
        );
    }

    /// AND AN UNREAD ASSUMED INPUT DOES NOT DRAG IT DOWN. `bed_temp` is
    /// assumed and in the table; it is not what this claim stands on.
    #[test]
    fn an_assumed_input_the_claim_does_not_read_does_not_weaken_it() {
        let v = verify(&clip_clamps(), &pla());
        assert_ne!(
            v.weakest_input.as_ref().map(|(n, _)| n.as_str()),
            Some("bed_temp")
        );
    }

    /// A MISSING INPUT IS A REFUSAL, not a zero. A claim that evaluates to
    /// something plausible from nothing is the failure this layer ends.
    #[test]
    fn a_missing_input_refuses_rather_than_defaulting() {
        let thin = Inputs::new().with("E", Input::measured(3500.0, "x"));
        let v = verify(&clip_clamps(), &thin);
        match v.status {
            Status::Refused(e) => assert!(e.contains("spring_t"), "{e}"),
            other => panic!("expected a refusal, got {other:?}"),
        }
        assert_eq!(v.value, None, "a refusal carries no value");
    }

    /// A refusal is NOT a failure: they call for different actions, and
    /// collapsing them makes an unmeasurable claim look like a broken design.
    #[test]
    fn a_refusal_is_not_a_failure() {
        let thin = Inputs::new();
        let v = verify(&clip_clamps(), &thin);
        assert!(!v.status.is_pass());
        assert!(!matches!(v.status, Status::Fail { .. }));
    }

    /// OUT OF RANGE NAMES THE DISTANCE, because "it fails" and "it fails by
    /// 0.19 N" lead to different decisions.
    #[test]
    fn a_value_below_the_range_names_how_far_below() {
        let petg = pla().with("E", Input::literature(2000.0, 250.0, "PETG datasheet"));
        let v = verify(&clip_clamps(), &petg);
        match v.status {
            Status::Fail { by } => assert!((by - (2.0 - 1.81)).abs() < 0.01, "got {by}"),
            other => panic!("1.81 N is below [2.0, 5.0]; got {other:?}"),
        }
    }

    /// A ZERO-VALUED INPUT IS STILL PERTURBED. Scaling zero by a fraction
    /// leaves zero, which would report every zero input as unused -- and a
    /// zero is exactly what somebody passes without thinking.
    #[test]
    fn an_input_sitting_at_zero_is_still_found_to_be_read() {
        let c = Claim {
            name: "offset".into(),
            declared_inputs: vec!["z".into()],
            range: (-1.0, 1.0),
            unit: "mm".into(),
            because: "test".into(),
            eval: |i| Ok(i.need("z")? + 0.5),
        };
        let inputs = Inputs::new().with("z", Input::measured(0.0, "the model"));
        assert_eq!(derived_inputs(&c, &inputs), vec!["z".to_string()]);
    }

    /// AN INPUT WHOSE MOVEMENT MAKES THE CLAIM REFUSE is a dependency, and a
    /// sharp one: the claim is defined only on part of that input's range.
    #[test]
    fn an_input_that_breaks_the_claim_when_moved_counts_as_read() {
        let c = Claim {
            name: "sharp".into(),
            declared_inputs: vec!["t".into()],
            range: (0.0, 100.0),
            unit: "x".into(),
            because: "test".into(),
            eval: |i| {
                let t = i.need("t")?;
                if t > 1.0 {
                    return Err("t past its domain".into());
                }
                Ok(t)
            },
        };
        let inputs = Inputs::new().with("t", Input::measured(1.0, "the model"));
        assert_eq!(derived_inputs(&c, &inputs), vec!["t".to_string()]);
    }

    /// Provenance orders worst-first so the weakest is a `min`, and `assumed`
    /// says so however many decimals the value carries.
    #[test]
    fn provenance_orders_worst_first_and_labels_itself() {
        assert!(Provenance::Assumed < Provenance::Literature);
        assert!(Provenance::Literature < Provenance::Derived);
        assert!(Provenance::Derived < Provenance::Measured);
        assert_eq!(Provenance::Assumed.label(), "assumed");
    }
}
