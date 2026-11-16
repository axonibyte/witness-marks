# Mutation record: quantization

| mutation | killed by |
| --- | --- |
| ties round UP, via plain `f64::round` | a_tie_rounds_down_rather_than_reporting_a_gain |
| the stiffness error is linear, not cubed | the_point_nine_leaf_loses_thirty_percent_of_its_stiffness |
| zero layers is allowed | a_sub_layer_thickness_becomes_one_layer_not_zero |
| it always floors, so a round-up never happens | rounding_up_reports_a_gain |

## The tie is the whole incident

0.9 mm at 0.20 mm layers is EXACTLY 4.5 layers. `f64::round` rounds half away
from zero, so the obvious implementation gives 5 layers, 1.0 mm, and reports a
stiffness GAIN -- the optimistic answer, for the case that produced the rule.
Ties go down, and the arm asserts the sign rather than the value, so it
catches any rounding that flatters the result.

## Linear would have invited a shrug

0.1 mm of thickness sounds like rounding noise. Cubed it is 29.77% of the
clamp. Reporting only the thickness error is how the original incident got
past everyone, so the mutant that drops the cube is pointed at the arm that
carries the real number.

## Zero layers is a different lie

A 0.05 mm feature at 0.20 mm layers does not vanish; it prints 0.20 mm, four
times too thick and 64 times too stiff. Reporting zero layers would say the
feature does not exist, which is the opposite of what happens.

## Tiers

Touched: Tier 1, against the harness incident's own numbers. Skipped: every
other tier.
