# Mutation record: derived_from

| mutation | killed by |
| --- | --- |
| it stops at the first derivation | the_sheared_socket_angle_has_two_explanations |
| halving is not searched | the_sheared_socket_angle_has_two_explanations |
| a coincidence counts EXPRESSIONS instead of ROOTS | a_zero_parameter_is_not_five_coincidences |
| source expressions are guessed at by grabbing the first number | literal_bindings_are_read_and_expressions_are_not_guessed_at |
| a difference is searched in one order only | a_difference_is_found_in_both_orders |

## Every candidate, never the first

34 degrees is BOTH `horn_deg` and `mouth_deg / 2`, and that double explanation
IS the class G signal. A function that reported one derivation and stopped
would hide exactly the case worth reporting -- which is how the dovetail
sockets came to be sheared by a mouth cut while both features were correct.

## A zero parameter is one root, not five coincidences

Counting expressions instead of roots survived every arm, because every other
value here has one expression per root. A binding at zero explains a zero as
`x`, `x / 2`, `x / 3`, `x * 2` and `x * 3` all at once -- one root, five
expressions. Parameters sit at zero all the time (`explode = 0` while
measuring), and reporting that as a coincidence would be a false alarm on the
most common value in any model.

## Expressions are not guessed at

`can_r = can_dia / 2` is collected by nobody. Resolving it would mean
evaluating OpenSCAD, and a half-working evaluator that silently gets one case
wrong is worse than one that plainly does not try. A caller with real values,
from `scad_eval`, passes them in. The mutant that grabs the first number out
of an expression is the plausible shortcut, and it is wrong: it would read
`can_dia / 2` as 2.

## The search is bounded and the bound is written down

Equality, a binding halved, doubled, tripled or thirded, and the sum,
difference or product of a pair. A derivation this misses is a derivation
nobody is told about, so the list is in the doc comment rather than left to be
inferred from the code.

## Tiers

Touched: Tier 1, over the incident's own bindings, plus Tier 3 reading the
vendored source. Skipped: every other tier.
