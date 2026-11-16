# Mutation record: beams

| mutation | killed by |
| --- | --- |
| fixed-guided uses the cantilever coefficient | a_fixed_guided_beam_is_four_times_the_cantilever |
| fixed-guided stress is not doubled | fixed_guided_doubles_the_stress_as_well_as_quadrupling_the_stiffness |
| the second moment is squared in thickness, not cubed | the_second_moment_is_w_t_cubed_over_twelve |
| zero stress reports infinite margin instead of refusing | zero_stress_refuses_rather_than_reporting_infinite_margin |
| stress drops its linear dependence on thickness | stress_follows_its_own_law |

## The end condition is a factor of four, and it is easy to miss

A cantilever is 3EI/L^3 and a fixed-guided beam is 12EI/L^3. Both look like
"a beam of length L", the formula differs only in a coefficient, and a leaf
between two rigid bodies is the second one. Computing it as the first is wrong
by 300%.

The half that gets forgotten is that fixed-guided also doubles the STRESS at
the same deflection, so the stiffer arrangement is the one closer to yield.
Two mutants, two arms, because comparing only stiffness is the mistake.

## Zero stress is not infinite margin

Returning infinity would read as the safest possible part. There is simply no
margin to report at zero deflection, and those are different sentences.

## A ratio, not a boolean

"It fails" and "it has 1.02 of margin" are different news. The margin is
returned as a number so the second survives.

## Tiers

Touched: Tier 1. The absolute stiffness is pinned against hand arithmetic
(41.890139850 N/mm) as well as against ratios, since a ratio test cannot see a
wrong coefficient. Skipped: every other tier.
