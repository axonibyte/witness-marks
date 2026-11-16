# `proc.support` differential -- seven mutants, seven kills

Issue #23's acceptance test, run at last: *"Port overhang-scan.scad and
validate against its four hand-computed shapes."*

## The four shapes, measured

| shape | the scan | by hand | the provider |
| --- | --- | --- | --- |
| flat 20x10 cantilever off a post | 198.5 | ~197 | 200.0 |
| 45 deg cone | 0.0 | 0 | 0.0, **bracket [0, 444.18]** |
| 60 deg cone | 0.0 | 0 | 0.0 |
| 30 deg cone | 131.1 | 132.8 | **362.64** |

| mutant | the lie it tells | killed by |
| --- | --- | --- |
| `print-bracket-ignored` | the point estimate stands whatever the bracket | `a_wide_bracket_is_undecided_rather_than_its_point_estimate` |
| `print-missing-overhang-is-clear` | no overhang block means no overhang | `a_missing_overhang_block_is_refused` |
| `print-bracket-optional` | a reading needs no uncertainty | `an_overhang_without_a_bracket_is_refused` |
| `print-zero-limit-allowed` | a zero undecided-limit is usable | `a_non_positive_undecided_limit_is_refused` |
| `print-zero-side-is-close` | one side at zero is close agreement | `one_side_at_zero_is_apart_and_not_a_ratio_to_be_read` |
| `print-everything-is-close` | every pair of figures agrees | `the_four_hand_computed_shapes_against_the_provider` |
| `print-inverted-bracket-accepted` | a backwards bracket is a bracket | `an_inverted_bracket_is_refused` |

## The 30 degree cone is 2.77x apart, and the arithmetic says why

A cone of R = 10 at 30 degrees has slant length `R / cos(30) = 11.547` and
lateral area `pi * R * slant = 362.76`. The provider reports **362.64**.

So the provider measures **the area of the down-facing surface**, and the
slicer criterion measures **the area with nothing under it**. For a flat slab
those nearly coincide -- 200.0 against 198.5, the difference being the rim the
offset `d` eats. For a cone they do not, and nothing about the two numbers
says which question was asked.

That is why this module does NOT feed the provider's area into
`support::Region::area_mm2` as though it were the criterion's. Two instruments
answering different questions are a differential, not a substitution.

## The 45 degree cone is the instrument admitting it cannot decide

Point estimate 0.0, bracket to **444.18**. At exactly the critical angle a
facet-normal method has half its tessellation either side. Reading the 0.0 and
discarding the bracket reports "no overhang" on the one shape the instrument
cannot judge, so a bracket wider than the caller's limit is a REFUSAL and not a
reading. The 60 degree cone's bracket is [0, 0], so the refusal is about the
width and not about cones.

Forty-five degrees again, from a third direction today: it is where the two
overhang conventions agree, where an inverted `d` still passes, and now where
the provider's own method runs out of resolution.

## Islands

`measure(mode=printability)` reports `islands` with a count, a list and the
layer height it used. That is the third of bridge/droop/island the criterion
alone does not separate, and it is read here rather than left for later.
