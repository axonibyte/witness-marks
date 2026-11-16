# Mutation record: firmware, commanded against achieved

| mutation | killed by |
| --- | --- |
| the command is reported as achieved, always | a_commanded_value_is_never_reported_as_achieved |
| the square root is dropped from `sqrt(a*L)` | the_harness_printers_reachable_retract_speeds |
| the length is ignored, so the ceiling is constant | a_shorter_retraction_reaches_a_lower_speed |
| the cap is reported without naming the limit | forty_commanded_is_seventeen_achieved_and_the_limit_is_named |

## Two numbers, never one

`commanded` and `achieved` are separate fields because a result that returned
a single number would be read as "the speed", and the entire finding is that
there are two of them and they differ. The first mutant is the one that makes
this a tool worth having: it reports every command as met, which is exactly
what the slicer already does.

## A shorter retraction is a SLOWER one

`v = sqrt(a*L)` over the whole move, so the length is the governing term and
the intuition that short moves are quick is backwards. The mutant that drops
the length produces a constant ceiling that is right at exactly one length.

## Naming the limit is half the answer

"capped" tells a reader nothing they can act on. The refusal names
`max_extrude_only_accel`, its value, and the length it was applied over, which
is what turns the result into the fix the note proposes: raise it to 2000 and
the 40 mm/s becomes reachable, which has its own arm.

## Tiers

Touched: Tier 1, against three published rows recomputed rather than copied
(the source note rounds 15.4726 to 15.5, 17.8662 to 17.9, 19.9750 to 20.0).
Skipped: every other tier.
