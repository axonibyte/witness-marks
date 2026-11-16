# Mutation record: the power screw

| mutation | killed by |
| --- | --- |
| self-locking compares mu against lambda in RADIANS, not its tangent | a_steeper_thread_distinguishes_the_tangent_from_the_angle |
| the lowering torque's sign is flipped | the_verdict_and_the_sign_of_the_lowering_torque_always_agree |
| the margin is a two-valued stand-in rather than a ratio | the_margin_separates_a_safe_case_from_a_marginal_one |
| a negative mu is accepted | a_negative_friction_coefficient_is_refused_as_a_sign_error |
| the helix angle drops the 2*pi | the_harness_threads_helix_angle |

## The survivor, and why the harness fixture could not catch it

`mu > lambda` and `mu > tan(lambda)` are the same answer on the can-cutter
thread: lambda is 0.0234009 rad against a tangent of 0.0234051, differing by
4.3e-06. Every arm written against that fixture passes under either formula,
so the mutant survived the first run. The fixture is the thing the project was
built from, and it is still not a sufficient test of a small-angle formula.

## And my first replacement arm was unphysical

A 57.9 degree helix with mu 1.2 does discriminate, and the code REFUSED it:
`mu*tan(lambda)` is 1.91, the raise term's denominator goes negative, and that
is not a screw any more. The guard was right and the test was wrong.

A discriminating case has to be steep enough that lambda and its tangent
diverge AND shallow enough that their product stays under 1. A 30 degree
helix with mu 0.5505 sits between 0.5236 and 0.5774 with a product of 0.318.

## The boolean and the sign are the same statement

Self-locking is `mu > tan(lambda)`; the lowering torque's numerator is
`mu - tan(lambda)`. An arm sweeps seven values of mu and asserts the two never
disagree, so if either is ever wrong the other catches it.

## Tiers

Touched: Tier 1, against the logged fixture plus a constructed case the
fixture cannot provide. Skipped: every other tier.
