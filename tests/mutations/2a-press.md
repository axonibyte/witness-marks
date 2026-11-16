# Mutation record: press fits

| mutation | killed by |
| --- | --- |
| the diametral interference is not halved | the_pressure_is_pinned_against_arithmetic |
| it is halved twice | the_pressure_is_pinned_against_arithmetic |
| the hub wall term is dropped from Lame | a_thinner_hub_grips_less |
| a hub no larger than the shaft is accepted | a_hub_no_larger_than_the_shaft_is_refused |
| the slot refusal returns a number instead | a_slot_refuses_to_be_treated_as_a_cylinder |

## Halved exactly once

The input is DIAMETRAL, because that is what a drawing states and what a
caliper measures, and Lame wants radial. Forgetting the halving doubles every
force in the answer; doing it twice halves them. Two mutants in opposite
directions against one hand-computed value, 7.5 MPa, because a single-sided
check would pass under one of them.

## A slot is not a cylinder, and the refusal is the feature

The fixture that prompted this issue is the blade's wedge pocket: two parallel
walls squeezing a flat blade by 0.05 mm per wall. Lame does not apply. A flat
wall has no hoop stress and nothing goes into tension around a circumference;
the pressure is set by the wall's BENDING stiffness, which `Beam` already
solves.

Applying the cylindrical formula anyway would produce a confident number with
the wrong physics behind it, which is worse than no number. The function
refuses, names why, and names where to go instead. The mutant that makes it
return a number is the one that matters, because a plausible wrong answer is
the failure this whole project is about.

## Tiers

Touched: Tier 1, against hand arithmetic. Skipped: every other tier. The
assumptions are stated in the doc comment rather than implied: same modulus
both sides, a solid shaft, isotropic. A printed part is strictly none of the
three, so the provenance of any result is Derived at best.
