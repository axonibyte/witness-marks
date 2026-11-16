# Mutation record: the mating-pair matrix

| mutation | killed by |
| --- | --- |
| retention with no wander figure passes | retention_without_a_wander_figure_is_unanswerable_not_ok |
| engagement exactly equal to wander passes | engagement_exactly_equal_to_wander_is_too_shallow |
| a zero gap counts as a clearance | a_zero_gap_satisfies_neither_intent |
| retention checks the sign and ignores the magnitude | a_snap_shallower_than_its_parts_wander_is_a_finding |
| only the first failure is reported | the_matrix_returns_what_is_wrong_and_not_merely_that_something_is |

## Two incidents, and only one is a sign error

buildability 48 is a SIGN: a retainer modelled with a clearance retains
nothing, having specified free play in exactly the axis it was meant to
constrain. buildability 70 is a sign that is RIGHT and a magnitude that is
not: 0.40 mm of engagement against 0.50 mm of lateral slop, so the carrier
shifted sideways and the rib cleared the crest with no force.

A matrix that only checked signs would pass the second, and an arm asserts
exactly that -- the same pair, relabelled as plain interference, comes back Ok.

## A missing wander is unanswerable, not fine

Assuming a part cannot move passes every retention pair ever checked. That is
precisely how a 0.40 mm engagement came to be trusted against 0.50 mm of slop,
so the absent figure is refused with that sentence attached.

## Exactly equal is not enough

Engagement equal to wander means the feature is cleared at the extreme of
travel, which is where parts actually sit.

## One signed number, not two fields

`gap_mm` is positive for a gap and negative for overlap. Two fields named
`clearance` and `interference` invite a caller to fill in the one they
expected, which is the error being looked for.

## Tiers

Touched: Tier 1 over both logged incidents' numbers. Skipped: every other
tier; measuring a pair belongs to Layer 1 and the provider.
