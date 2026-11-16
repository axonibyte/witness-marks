# Milestone 3, Claims -- twenty-one mutants, twenty-one kills

Spec §06: *"This is the product. Everything else here is plumbing that makes
it possible."*

## The vectors, reproduced

| from | figure | reproduced |
| --- | --- | --- |
| §06.1 | `clip_clamps` = 3.17 N in PLA | yes, from `wm-mech` |
| §06.2 | E 3500 -> 2000 invalidates **6 of 12** | yes |
| §06.2 | clamp 3.17 -> 1.81 N, FAILS [2.0, 5.0] | yes, a 43% loss |
| §06.2 | stress 20.9 -> 11.9 MPa, ok, margin improved | yes |
| §06.2 | "2 inputs are now literature values" | yes, E and yield |
| §06.3 | thickness CUBED, modulus LINEAR | yes, elasticity 3.0 and 1.0 |
| §06.3 | thickness not a whole number of layers | yes, 0.9 = 4.5 layers |
| b'ty 70 | `thr_clear` serves three chains, retention -0.10 mm | yes |
| §08 | PETG modulus is literature 1700-2200 | yes, +/-250 on 2000 |

| mutant | the lie it tells | killed by |
| --- | --- | --- |
| `clm-deps-trusts-declaration` | a claim's declared inputs are its real ones | `an_input_declared_but_not_read_is_named` |
| `clm-zero-input-not-probed` | an input sitting at zero is unused | `an_input_sitting_at_zero_is_still_found_to_be_read` |
| `clm-breaking-input-not-a-dep` | an input that breaks the claim is not read by it | `an_input_that_breaks_the_claim_when_moved_counts_as_read` |
| `clm-weakest-is-strongest` | report the best provenance, not the worst | `the_weakest_input_among_those_read_is_reported` |
| `clm-missing-input-defaults` | a missing input is zero | `a_missing_input_refuses_rather_than_defaulting` |
| `stl-empty-claims-ok` | propagating over no claims is a clean run | `propagating_over_no_claims_is_refused` |
| `stl-identical-inputs-ok` | a change that changed nothing is a safe change | `propagating_over_identical_inputs_is_refused` |
| `stl-unevaluable-counted-untouched` | a claim that cannot be checked was unaffected | `a_claim_that_cannot_be_evaluated_is_neither_passing_nor_failing` |
| `stl-appearing-input-not-a-change` | an input that appeared is not a change | `an_input_that_appeared_counts_as_a_change` |
| `dom-slope-not-elasticity` | raw slopes are comparable between inputs | `thickness_reads_cubed_and_modulus_linear` |
| `dom-forward-difference` | a forward difference is good enough on a cubic | `thickness_reads_cubed_and_modulus_linear` |
| `dom-no-uncertainty-is-zero` | an unstated uncertainty is zero | `inputs_with_no_stated_uncertainty_are_named_rather_than_scored_zero` |
| `dom-zero-base-ranked` | a claim at zero can be ranked by elasticity | `a_claim_evaluating_to_zero_is_refused_rather_than_ranked` |
| `fan-opposition-not-detected` | a symbol two claims pull against is uncontested | `one_symbol_serving_three_chains_is_reported_with_the_conflict` |
| `fan-unread-symbol-reported-clean` | a symbol no claim reads has a clean fan-out | `a_symbol_no_claim_reads_is_refused` |
| `fan-bites-last-not-first` | report the claim with the MOST room | `the_claim_closest_to_its_edge_is_named` |
| `prv-derived-does-not-inherit` | a derived value is better than its sources | `a_value_derived_from_a_literature_input_is_no_better_than_literature` |
| `prv-derive-from-nothing-ok` | a derived value needs no sources | `deriving_from_nothing_is_refused` |
| `ref-orientations-without-loads` | orientations can be ranked without loads | `ranking_orientations_without_loads_is_refused` |
| `ref-clearance-ignores-tolerance` | a modelled gap is a printed gap | `the_same_modelled_gap_is_adequate_or_not_depending_on_the_spread` |
| `ref-commanded-is-achieved` | a commanded value is an achieved one | `with_the_ceiling_stated_the_commanded_value_is_capped` |

## The dependency set is DERIVED, not declared

A claim names the inputs it believes it uses, and that declaration is exactly
what goes stale: somebody adds a term, the list is not updated, and staleness
propagation then skips the claim that most needed it -- silently, because an
incomplete list looks like a complete one.

So the real set is found by PERTURBATION: move each input, see whether the
claim's value moves. `clm-deps-trusts-declaration` is the version that trusts
the declaration, and it dies against a claim declaring an input it does not
read. The same perturbation is what `dominance` reads for sensitivity, so
there is one account of which inputs matter and not two.

## Relatedness is measured, not declared

"Flagged when they have no physical relationship" needs relatedness to mean
something checkable. A declared taxonomy would be exactly as stale as the
declared dependency lists this layer already refuses to trust.

So the test is the SIGN of each claim's elasticity with respect to the symbol.
A symbol one claim wants larger and another wants smaller is doing two jobs
that FIGHT, whatever either is called. Nothing told the code that a snap and a
thread want opposite things.

## `dom-forward-difference` is not a style preference

A forward difference on a cubic carries a first-order error that reads as a
real asymmetry between inputs -- which is precisely what this report exists to
measure. The central difference reads thickness at 3.0000 and modulus at
1.0000; the forward one does not, and the elasticity test catches it.

## One test of mine was wrong, and the code was right

`the_same_modelled_gap_is_adequate_or_not_depending_on_the_spread` first
asserted that a 0.20 gap needing 0.05 fails at +/-0.15. It does not: 0.20 -
0.15 = 0.05, which exactly meets the requirement. The fixture moved to
+/-0.16, and the boundary got its own test rather than being left to a
comparison operator nobody reads.

## The battery went red for a reason that was not the claims layer

`mutate.sh --all` MISFIRED on `mech-thickness-cubed-to-squared`, a mutant
written weeks earlier that had been killed cleanly every run until now.

It had NO `test-cmd`, so it took the default -- which was
`cargo test --locked --workspace`, without `--no-fail-fast`. Cargo stops at
the first failing target. The mutation breaks `wm-mech`'s cubic second moment,
and `wm-claims` had just grown an elasticity test that ALSO detects a broken
cubic (thickness must read 3.0). Cargo stopped in `wm-claims`, so `wm-mech`'s
named arm never ran, and refusal 4 correctly reported that the test the mutant
names did not die.

The mutant was fine. The default was the trap, and it had been latent for as
long as no two crates could detect the same mutation.

Fixed at the class: the default now carries `--no-fail-fast`, the two
`wm-mech` mutants got explicit scoped commands, and
`t_mutation_record.sh` gained three arms -- the default carries the flag,
every cargo mutant names it (204 checked), and the sweep refuses if it finds
none, which is the guard that caught the first version of the arm sweeping the
filesystem root after a `cd`.

There is a pleasant corollary in the failure itself: the claims layer's
elasticity measurement independently detects `wm-mech`'s cubic exponent. Two
crates, two mechanisms, one answer -- which is why the mutation could reach
both in the first place.
