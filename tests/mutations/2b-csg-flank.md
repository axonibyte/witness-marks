# Mutation record: flank

| mutation | killed by |
| --- | --- |
| the probe overlaps the feature instead of sitting outboard | a_probe_sits_immediately_outboard_of_the_chosen_face |
| the reading is a raw volume, not a fraction of capacity | the_same_volume_means_different_things_in_different_probes |
| a reading larger than the probe is accepted | a_reading_larger_than_the_probe_is_refused |
| an unbounded feature is probed at the origin | an_unbounded_feature_cannot_be_probed |
| a zero-capacity probe is allowed | a_flat_feature_gives_a_probe_with_no_volume_and_is_refused |

## The capacity is what makes the reading mean anything

52.800 on its own is not a finding. 52.800 out of 52.800 is an intact flank;
52.800 out of 500 is mostly air. The logged incident only reads as a finding
because the two probes had the SAME capacity and returned 0.000 against
52.800. The mutant that reports a raw volume takes that away and leaves two
numbers that look like measurements.

## Two refusals that would otherwise answer 0.000 for everything

A zero-capacity probe -- from a feature flat on another axis -- returns 0.000
whatever the material, so every flank in the model reads Absent. And a probe
placed relative to an unbounded feature lands at the origin, measures
somewhere else entirely, and reports a number for it. Both are refused with
the consequence spelled out rather than the condition.

## More material than the box holds is a bug, not a very full flank

It means the probe and the measurement are not of the same thing.

## The split

This places the probe; `wm-provider` measures it. Placement is where the
mistakes are and is testable without a Python environment; measuring is a
kernel operation and belongs to the layer that owns the kernel.

## Tiers

Touched: Tier 1. Skipped: every other tier; nothing here evaluates geometry,
which is the point of the split.
