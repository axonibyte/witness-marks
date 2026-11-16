# Mutation record: the critical angle

| mutation | killed by |
| --- | --- |
| the angle is hardcoded to 45 | forty_five_is_not_a_constant |
| it reads the nozzle diameter instead of the extrusion width | it_reads_the_width_and_not_the_nozzle |
| the half-width is dropped from the denominator | the_angle_is_a_property_of_the_print |
| the NaN guard is removed, leaving only `<= 0.0` | a_nonpositive_input_is_refused_not_reported_as_zero_degrees |

## 45 is folklore with exactly one true case

0.2 mm layers at 0.4 mm width make the ratio exactly 1, and atan(1) is 45. The
number is right there and nowhere else by right: 0.1 mm holds 26.565051 and
0.15 mm holds 36.869898, all three hand-computed and pinned. The hardcoding
mutant is the one that would otherwise be invisible, because 45 is the answer
a reader expects.

## Reading the nozzle is the plausible wrong answer

A 0.4 mm nozzle commonly extrudes 0.42 to 0.48. Reading the nozzle gives a
different, slightly wrong angle that looks exactly as reasonable, which is why
it gets its own mutant and its own arm with a 0.46 width.

## Clippy found a real one

The guard was `!(h > 0.0)`, which catches NaN as a SIDE EFFECT of the
negation. Clippy refused it as hard to read and was right twice: the NaN case
was load-bearing and invisible. It is `!h.is_finite() || h <= 0.0` now, and
NaN is an arm, because a plain `h <= 0.0` lets NaN straight through -- every
comparison with NaN is false.

## Tiers

Touched: Tier 1, against hand-computed values. Skipped: every other tier.
