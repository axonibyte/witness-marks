# Mutation record: our own tool-surface budget

| mutation | killed by |
| --- | --- |
| the per-tool cap never refuses | the_budget_bites_and_names_what_breached_it |
| only the first breach is reported | every_breach_is_reported_not_only_the_first |
| the measurement counts tools, not bytes | the_measurement_is_the_serialized_bytes |
| the per-tool cap set below the provider's largest tool | **the COMPILER** |

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

## The cap is a compile-time rule now, and that changed the mutant

The caps were first checked by a test asserting `TOTAL_CAP >= 19_174`. Clippy
refuses that -- rightly: both sides are constants, so the assertion cannot
vary, and a test that cannot fail is not a test. It is
`const _: () = assert!(...)` now, which is stronger: lowering a cap below the
measurement it was derived from stops the BUILD.

That made the mutant report MISFIRED, because no named test runs when the build
refuses. The tool was punishing the stronger guard, so `tools/mutate.sh` grew
`expect-build-failure`: a mutant may declare that the COMPILER is what kills
it, naming the refusal it expects. Declaring both expectations is refused,
since a mutant passing on whichever matched would be an arm that cannot fail.

AND A PROCESS DEFECT OF MINE, which is how the clippy failure survived a
commit. I had been verifying with `cargo clippy ... >/dev/null 2>&1 && echo
"clippy clean"`. On failure that prints NOTHING, and I read the absence as
fine. The budget commit went in with clippy red. A check whose failure is
silent is not a check, which is the same sentence this repository keeps
writing about other people's instruments.
