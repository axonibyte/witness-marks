# Mutation record: the envelope over provider answers

Eight mutants, eight kills, all recorded under `tests/mutations/` so
`tools/mutate.sh --all` re-runs them.

## The derivation (wm-envelope)

| mutation | killed by |
| --- | --- |
| a zero-extent subject becomes `Ok` | a_subject_with_zero_extent_is_refused_never_ok |
| a missing bbox becomes `Ok` | a_subject_with_zero_extent_is_refused_never_ok |
| degenerate requires ALL axes flat, not any | one_flat_axis_is_degenerate |
| `UNRESOLVED` no longer refuses | an_unresolved_input_is_refused_not_failed |

## The wiring (wm-provider)

| mutation | killed by |
| --- | --- |
| a refusal keeps its value | a_zero_extent_subject_is_refused_and_carries_no_value |
| `UNRESOLVED` searched at top level only | unresolved_is_found_at_any_depth |
| the missing bbox corner is invented as the origin | half_a_bounding_box_is_no_bounding_box |
| hints dropped while warnings are kept | the_providers_empty_answer_becomes_an_empty_envelope_keeping_its_hint |

## Three rules the mutants pin, and why each has its own arm

**ANY flat axis is degenerate, not all three.** The obvious implementation
checks whether the box is the origin-to-origin box; a 10 x 10 x 0 subject then
passes as `Ok` while having no volume at all. The mutant that changes `any` to
`all` is the one that catches it.

**A refusal carries no number.** Returning a value beside `refused` invites a
caller to read the number and ignore the word, which is the whole shape of a
stale claim. `Ok` is the only verdict that carries one.

**Half a box is no box.** Filling a missing `bbox_max` with the origin would
manufacture an extent the provider never claimed, and the `Ok` verdict rests on
exactly that extent.

## Tiers

Touched: Tier 1 for the derivation, which is a pure function of named inputs,
and Tier 2 for the wiring over real provider payload shapes captured from the
pinned server. Skipped: every tier above. The payload fixtures are the shapes
the provider actually returned, not invented ones.
