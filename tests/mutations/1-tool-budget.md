# Mutation record: our own tool-surface budget

| mutation | killed by |
| --- | --- |
| the per-tool cap never refuses | the_budget_bites_and_names_what_breached_it |
| only the first breach is reported | every_breach_is_reported_not_only_the_first |
| the measurement counts tools, not bytes | the_measurement_is_the_serialized_bytes |
| the per-tool cap set below the provider's largest tool | the_caps_admit_the_provider_they_were_set_from |

## The control, and why it is the arm that matters

Witness Marks registers NO tools yet. Measuring the real surface therefore
returns zero and complains about nothing, which means every arm that only
measures is equally satisfied by a function that can never refuse. The arm that
feeds it a tool over the cap is the one that proves the budget exists, and it
also pins that the refusal NAMES the offending tool and does not blame the
innocent one beside it.

## The caps are provisional and the test says so

The reference is measured, not guessed: the pinned provider advertises 12 tools
totalling 19174 bytes, largest single tool `render` at 2263, read off the wire
on 2026-11-16. TOTAL_CAP is 20000 and PER_TOOL_CAP is 2500, and a test asserts
the caps still admit the provider they were set from -- so lowering one below
its own justification fails rather than quietly tightening.

## Tiers

Touched: Tier 1. Skipped: every other tier; the subject is a measurement over
a list of JSON objects.
