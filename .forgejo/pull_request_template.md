<!-- docs/testing-methodology.md is normative. CONTRIBUTING.md is its short
     form and lists which rules are mechanized. -->

## What this changes

<!-- The behavior, not the diff. -->

## Tiers

- **Touched:**
- **Skipped, and why:**

<!-- Name every tier you skipped and the reason. A skipped tier named is a
     decision; a skipped tier unmentioned is an accident. The tiers are listed
     in docs/testing-methodology.md §3-§13. -->

## Mutation kills

<!-- For each new or changed assertion: what was broken, and which test died.
     Either a MUTATION: line in a commit, or a directory under tests/mutations/
     that `tools/mutate.sh --all` re-runs. A SURVIVING mutant is a finding and
     belongs here too, with what you concluded from it. -->

| mutation | test that died |
| --- | --- |

## Narrowings

<!-- Any exclusion, threshold, skip, allow, or scope reduction, with a reason
     that covers exactly what it narrows and no more. "None" is a fine answer
     and is better than silence. -->

## Pre-existing failures

<!-- If anything was already red, say so and say how you proved it: stash,
     re-run, name it. -->

## Evidence

<!-- The commands you ran and what they said. Counts, not adjectives. -->
