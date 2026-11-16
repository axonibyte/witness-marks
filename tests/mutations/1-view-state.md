# Mutation record: pinning view state

| mutation | killed by |
| --- | --- |
| `show_*` classified as displacement, so visibility flags get pinned | visibility_flags_are_reported_and_never_pinned |
| detection by NAME only, ignoring the Customizer group | a_view_parameter_named_like_a_dimension_is_caught_by_its_group |
| a view group never closes, so everything after it is view state | a_group_ends_at_the_next_group |
| the pinning plan is computed and never attached to the call | view_state_is_pinned_in_the_arguments_that_are_actually_sent |
| an empty `variables` object is sent when there is nothing to pin | a_model_without_view_state_sends_no_variables_at_all |

## The arm that asserts what was SENT

Four of these could be satisfied by a client that computes a perfect pinning
plan and never attaches it to the call. The stub grew an `echoargs` mode that
returns the call's own arguments, so one arm reads what actually went over the
wire rather than what the client decided. `view-plan-not-sent` is the mutant
that would otherwise have survived everything.

## A misfire, and the tool was right again

The first version of the visibility mutant did not compile: it opened a block
the surrounding match arm never closed. `mutate.sh` reported MISFIRED rather
than counting the nonzero exit as a kill, which is the second time that
refusal has caught a mutant that broke the build and would have read as a kill
for whichever arm was being watched. It was replaced with one that compiles,
classifying `show_*` as displacement, and that one kills.

## Tiers

Touched: Tier 1 for the detector, over the REAL incident file vendored at
tests/fixtures/can_cutter/can_cutter.scad rather than an invented sample, and
Tier 2 for the argument actually sent. Skipped: every tier above.
