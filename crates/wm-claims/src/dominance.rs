//! Which input owns the number.
//!
//! Spec §06.3. A value alone barely helps decide what to do next; what an
//! agent needs is which input it is really a function of. The harness's
//! clamp-force report would have read: thickness at a CUBED sensitivity, slot
//! clearance through a 3.54x lever, modulus linear -- and a flag that the
//! thickness is not a whole number of layers.
//!
//! That one report would have caught the quantization bug, the PETG
//! regression and the slot-slack amplification before any of them was
//! printed. All three were found by accident, in conversation, late.
//!
//! # Elasticity, not slope
//!
//! The slope `dF/dx` is in mixed units and cannot be compared between inputs:
//! newtons per MPa against newtons per millimetre says nothing about which
//! matters. The dimensionless elasticity `(x/F)(dF/dx)` can -- it is the
//! power-law exponent, so a cubed dependence reads 3.0 and a linear one 1.0,
//! whatever the units.
//!
//! # Share of variance needs uncertainties, and says so when it lacks them
//!
//! Elasticity ranks inputs by leverage. It does NOT rank them by how much
//! they will actually move the answer, because a highly-levered input that is
//! known to a micron contributes nothing. That needs each input's
//! uncertainty, and an input with none is reported as UNQUANTIFIED rather
//! than as zero -- a missing uncertainty treated as zero makes the
//! least-known input look like the safest one.

use crate::claim::{Claim, Evaluator, Inputs, Provenance, RustOnly, PROBE_FRACTION};

/// How one input drives one claim.
#[derive(Debug, Clone, PartialEq)]
pub struct Sensitivity {
    pub input: String,
    /// `dF/dx` in the claim's units per the input's units.
    pub slope: f64,
    /// `(x/F)(dF/dx)`: the power-law exponent. Comparable across inputs.
    pub elasticity: f64,
    /// `|slope * uncertainty|`: how far this input alone could move the
    /// answer. `None` when the input has no stated uncertainty.
    pub contribution: Option<f64>,
    pub provenance: Provenance,
}

/// The whole report for one claim.
#[derive(Debug, Clone, PartialEq)]
pub struct Dominance {
    pub claim: String,
    pub value: f64,
    /// Strongest leverage first.
    pub by_elasticity: Vec<Sensitivity>,
    /// Share of variance, largest first. Empty when no input carries an
    /// uncertainty -- which is itself the finding.
    pub share_of_variance: Vec<(String, f64)>,
    /// Inputs with leverage and no stated uncertainty. These are the ones
    /// where "how much should I trust this" has no answer at all.
    pub unquantified: Vec<String>,
    pub flags: Vec<String>,
}

impl Dominance {
    /// The input to go and measure: largest `slope * uncertainty`.
    ///
    /// Spec §08's "what would I have to measure to trust this". Returns
    /// `None` when nothing carries an uncertainty, because the honest answer
    /// is then "nobody has said how well anything here is known".
    pub fn worth_measuring(&self) -> Option<&Sensitivity> {
        self.by_elasticity
            .iter()
            .filter(|s| s.contribution.is_some())
            .max_by(|a, b| {
                a.contribution
                    .unwrap_or(0.0)
                    .partial_cmp(&b.contribution.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }
}

/// Finite-difference sensitivity for every input a claim reads.
///
/// `quantum` is an optional dimension that physical inputs must be a multiple
/// of -- the layer height. Passing `None` suppresses the quantization flag
/// rather than silently passing it, because a flag that never fires is
/// indistinguishable from a design with no quantization problem.
pub fn dominance(
    claim: &Claim,
    inputs: &Inputs,
    quantum: Option<f64>,
) -> Result<Dominance, String> {
    dominance_with(claim, inputs, quantum, &RustOnly)
}

/// [`dominance`] against a named evaluator (issue 60).
///
/// THE ELASTICITIES INHERIT THE EVALUATOR'S PRECISION. Over a scad expression
/// `scad_eval` answers at six significant digits, so an elasticity below about
/// 1e-2 at this probe reads as zero -- and a zero elasticity here is reported
/// as an input that does not drive the claim. That is the same floor
/// `derived_inputs_with` carries, arriving in a different answer.
pub fn dominance_with(
    claim: &Claim,
    inputs: &Inputs,
    quantum: Option<f64>,
    ev: &dyn Evaluator,
) -> Result<Dominance, String> {
    let base = ev.eval(&claim.expression, inputs)?;
    if !base.is_finite() {
        return Err(format!(
            "the claim evaluates to {base}, which is not a value"
        ));
    }
    if base == 0.0 {
        return Err(
            "the claim evaluates to zero, so an elasticity has no denominator. A slope can \
             still be read, but the dimensionless ranking this report exists to give cannot, \
             and returning the slopes alone would invite comparing newtons per MPa against \
             newtons per millimetre"
                .into(),
        );
    }
    let mut out = Vec::new();
    for name in inputs.names() {
        let Some(input) = inputs.get(&name) else {
            continue;
        };
        let step = if input.value == 0.0 {
            PROBE_FRACTION
        } else {
            input.value.abs() * PROBE_FRACTION
        };
        // CENTRAL DIFFERENCE, not forward. A forward difference on a cubic
        // carries a first-order error that reads as a real asymmetry between
        // inputs, which is exactly what this report is for.
        let up = ev.eval(
            &claim.expression,
            &inputs.perturbed(&name, input.value + step),
        );
        let down = ev.eval(
            &claim.expression,
            &inputs.perturbed(&name, input.value - step),
        );
        let slope = match (up, down) {
            (Ok(u), Ok(d)) if u.is_finite() && d.is_finite() => (u - d) / (2.0 * step),
            _ => continue,
        };
        if slope == 0.0 {
            continue;
        }
        out.push(Sensitivity {
            elasticity: input.value / base * slope,
            contribution: input.uncertainty.map(|s| (slope * s).abs()),
            slope,
            provenance: input.provenance,
            input: name,
        });
    }
    out.sort_by(|a, b| {
        b.elasticity
            .abs()
            .partial_cmp(&a.elasticity.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.input.cmp(&b.input))
    });

    let total: f64 = out
        .iter()
        .filter_map(|s| s.contribution)
        .map(|c| c * c)
        .sum();
    let mut share: Vec<(String, f64)> = if total > 0.0 {
        out.iter()
            .filter_map(|s| s.contribution.map(|c| (s.input.clone(), c * c / total)))
            .collect()
    } else {
        Vec::new()
    };
    share.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });

    let unquantified = out
        .iter()
        .filter(|s| s.contribution.is_none())
        .map(|s| s.input.clone())
        .collect();

    let mut flags = Vec::new();
    if let Some(q) = quantum {
        if !q.is_finite() || q <= 0.0 {
            return Err(format!("the quantum is {q}, which is not a dimension"));
        }
        for s in &out {
            if let Some(i) = inputs.get(&s.input) {
                let layers = i.value / q;
                if i.value > 0.0 && (layers - layers.round()).abs() > 1e-9 {
                    flags.push(format!(
                        "{} = {} is {:.3} of the {q} quantum, not a whole number of them; at \
                         an elasticity of {:.2} the rounding the slicer does is {:.1}x in the \
                         answer",
                        s.input,
                        i.value,
                        layers,
                        s.elasticity,
                        s.elasticity.abs()
                    ));
                }
            }
        }
    }
    Ok(Dominance {
        claim: claim.name.clone(),
        value: base,
        by_elasticity: out,
        share_of_variance: share,
        unquantified,
        flags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim::Expression;
    use crate::claim::Input;
    use wm_mech::{Beam, End};

    fn clip_clamps() -> Claim {
        Claim {
            name: "clip_clamps".into(),
            declared_inputs: vec!["E".into(), "spring_t".into(), "spring_pre".into()],
            range: (2.0, 5.0),
            unit: "N".into(),
            because: "the harness".into(),
            expression: Expression::Rust(|i| {
                Ok(Beam {
                    e_mpa: i.need("E")?,
                    l_mm: 8.9,
                    w_mm: 10.0,
                    t_mm: i.need("spring_t")?,
                    end: End::Cantilever,
                }
                .at(i.need("spring_pre")?)?
                .force)
            }),
        }
    }

    fn pla() -> Inputs {
        Inputs::new()
            .with("E", Input::literature(3500.0, 250.0, "datasheet"))
            .with("spring_t", Input::measured(0.9, "calipers"))
            .with("spring_pre", Input::measured(0.350, "the model"))
    }

    /// THE VECTOR, from spec §06.3: thickness is CUBED, modulus is LINEAR.
    /// Elasticity reads them off as 3.0 and 1.0 without being told the
    /// formula.
    #[test]
    fn thickness_reads_cubed_and_modulus_linear() {
        let d = dominance(&clip_clamps(), &pla(), None).expect("valid");
        let t = d
            .by_elasticity
            .iter()
            .find(|s| s.input == "spring_t")
            .expect("present");
        let e = d
            .by_elasticity
            .iter()
            .find(|s| s.input == "E")
            .expect("present");
        assert!(
            (t.elasticity - 3.0).abs() < 1e-4,
            "thickness {}",
            t.elasticity
        );
        assert!(
            (e.elasticity - 1.0).abs() < 1e-4,
            "modulus {}",
            e.elasticity
        );
    }

    /// And thickness therefore SORTS FIRST, which is the whole point: the
    /// cubed input is the one to look at, and nobody had to say so.
    #[test]
    fn the_cubed_input_sorts_above_the_linear_ones() {
        let d = dominance(&clip_clamps(), &pla(), None).expect("valid");
        assert_eq!(d.by_elasticity[0].input, "spring_t");
    }

    /// Deflection is linear too, so three inputs with exponents 3, 1, 1.
    #[test]
    fn preload_is_linear_like_the_modulus() {
        let d = dominance(&clip_clamps(), &pla(), None).expect("valid");
        let p = d
            .by_elasticity
            .iter()
            .find(|s| s.input == "spring_pre")
            .expect("present");
        assert!((p.elasticity - 1.0).abs() < 1e-4, "{}", p.elasticity);
    }

    /// SHARE OF VARIANCE IS A DIFFERENT RANKING FROM ELASTICITY, and that is
    /// the point of having both. Thickness has three times the leverage, and
    /// `E` -- known only to +/-250 of 3500 -- owns the variance, because the
    /// thickness is measured and carries no stated uncertainty at all.
    #[test]
    fn the_variance_is_owned_by_the_input_nobody_measured() {
        let d = dominance(&clip_clamps(), &pla(), None).expect("valid");
        assert_eq!(d.by_elasticity[0].input, "spring_t", "leverage");
        assert_eq!(d.share_of_variance[0].0, "E", "variance");
        assert!(
            (d.share_of_variance[0].1 - 1.0).abs() < 1e-9,
            "E is the only input with an uncertainty, so it owns all of it: {:?}",
            d.share_of_variance
        );
    }

    /// AN INPUT WITH NO UNCERTAINTY IS UNQUANTIFIED, NOT ZERO. Treating a
    /// missing uncertainty as zero makes the least-known input look safest.
    #[test]
    fn inputs_with_no_stated_uncertainty_are_named_rather_than_scored_zero() {
        let d = dominance(&clip_clamps(), &pla(), None).expect("valid");
        assert_eq!(
            d.unquantified,
            vec!["spring_t".to_string(), "spring_pre".to_string()]
        );
    }

    /// "WHAT WOULD I HAVE TO MEASURE TO TRUST THIS" -- spec §08. The answer
    /// is the largest slope x uncertainty, which is `E`.
    #[test]
    fn the_input_worth_measuring_is_the_largest_slope_times_uncertainty() {
        let d = dominance(&clip_clamps(), &pla(), None).expect("valid");
        let w = d.worth_measuring().expect("something to measure");
        assert_eq!(w.input, "E");
        // 250 MPa of uncertainty at 3.17 N / 3500 MPa is ~0.226 N on a claim
        // whose range is 3 N wide: a quarter of the margin.
        assert!(
            (w.contribution.expect("a contribution") - 0.2265).abs() < 0.01,
            "{:?}",
            w.contribution
        );
    }

    /// AND IT RETURNS NOTHING RATHER THAN GUESSING when no input carries an
    /// uncertainty: the honest answer is "nobody has said how well anything
    /// here is known".
    #[test]
    fn with_no_uncertainties_anywhere_there_is_nothing_to_recommend() {
        let all_measured = Inputs::new()
            .with("E", Input::measured(3500.0, "tensile test"))
            .with("spring_t", Input::measured(0.9, "calipers"))
            .with("spring_pre", Input::measured(0.350, "the model"));
        let d = dominance(&clip_clamps(), &all_measured, None).expect("valid");
        assert!(d.worth_measuring().is_none());
        assert!(d.share_of_variance.is_empty());
        assert_eq!(d.unquantified.len(), 3);
    }

    /// THE QUANTIZATION FLAG, the third bug that report would have caught:
    /// 0.9 mm is 4.5 layers of 0.2, and at an elasticity of 3 the slicer's
    /// rounding is cubed in the answer.
    #[test]
    fn a_thickness_that_is_not_a_whole_number_of_layers_is_flagged() {
        let d = dominance(&clip_clamps(), &pla(), Some(0.2)).expect("valid");
        assert!(
            d.flags
                .iter()
                .any(|f| f.contains("spring_t") && f.contains("4.500")),
            "{:?}",
            d.flags
        );
        assert!(
            d.flags.iter().any(|f| f.contains("3.0x in the answer")),
            "the flag carries the elasticity, so the cost is visible: {:?}",
            d.flags
        );
    }

    /// AND A WHOLE NUMBER OF LAYERS IS NOT FLAGGED, so the flag is about the
    /// value and not about the input.
    #[test]
    fn a_thickness_that_is_a_whole_number_of_layers_is_not_flagged() {
        let ok = pla().with("spring_t", Input::measured(0.8, "calipers"));
        let d = dominance(&clip_clamps(), &ok, Some(0.2)).expect("valid");
        assert!(
            !d.flags.iter().any(|f| f.contains("spring_t")),
            "0.8 is exactly 4 layers: {:?}",
            d.flags
        );
    }

    /// NO QUANTUM MEANS NO FLAG, stated rather than silent: a flag that never
    /// fires cannot be told from a design with no quantization problem.
    #[test]
    fn passing_no_quantum_suppresses_the_flag_entirely() {
        let d = dominance(&clip_clamps(), &pla(), None).expect("valid");
        assert!(d.flags.is_empty());
    }

    /// A CLAIM AT ZERO HAS NO ELASTICITY DENOMINATOR, and returning the
    /// slopes alone would invite comparing newtons per MPa against newtons
    /// per millimetre.
    #[test]
    fn a_claim_evaluating_to_zero_is_refused_rather_than_ranked() {
        let zero = Claim {
            name: "zero".into(),
            declared_inputs: vec!["a".into()],
            range: (-1.0, 1.0),
            unit: "x".into(),
            because: "test".into(),
            expression: Expression::Rust(|i| Ok(i.need("a")? * 0.0)),
        };
        let inputs = Inputs::new().with("a", Input::measured(1.0, "x"));
        let e = dominance(&zero, &inputs, None).expect_err("must refuse");
        assert!(e.contains("newtons per MPa"), "{e}");
    }

    #[test]
    fn a_non_positive_quantum_is_refused() {
        assert!(dominance(&clip_clamps(), &pla(), Some(0.0)).is_err());
        assert!(dominance(&clip_clamps(), &pla(), Some(-0.2)).is_err());
    }
}
