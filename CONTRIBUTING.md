# Contributing

`docs/testing-methodology.md` is **normative** for this repository. It is not
advice and not a style guide: where it and a convenience disagree, it wins, and
the way to change it is to change the document.

This file is the short version plus the places the rules are mechanized, so
that a rule and the thing enforcing it can be checked against each other.

## The non-negotiables

Stated once here; `docs/testing-methodology.md` §2 is the authority.

**Never weaken a test, check, assertion or lint to route around a defect.** Not
by disabling a phase, adding an exclusion, lowering a threshold, marking
`skip` / `ignore` / `allow`, appending `|| true`, catching and swallowing, or
narrowing a suite's scope. Each one is a permanent decision about what this
project stops noticing.

**Every narrowing needs a stated reason, and the reason must cover exactly what
it narrows.** An exclusion that suppresses four things behind a justification
for two is a bug in the justification. Narrowings go in the pull request body,
not only in a code comment.

**Every fix ships with a test that would have caught it.** Where a change is
genuinely untestable in isolation — dead-code removal, a transposed log
argument, a version bump, configuration, comments — say so explicitly rather
than inventing a test that only asserts a constant.

**A pre-existing failure must be proven pre-existing.** Stash, re-run, and name
it. Otherwise "that one always fails" becomes the sentence that hides a
regression.

**New assertions get mutation-checked.** Break the thing a new test covers and
confirm it fails. A test never observed failing is a test whose value is
unmeasured.

**Fix the cause, not the symptom.** A quiet terminal is not the goal.

## What is mechanized, and where

| rule | enforced by | CI step |
| --- | --- | --- |
| new assertions carry a recorded kill | `tools/check-mutation-record.sh` | *new assertions must carry a recorded kill* |
| recorded kills still kill | `tools/mutate.sh --all` | *every recorded mutant still kills* |
| the tooling is tested before it is trusted | `tools/tests/t_*.sh` | *the tooling's own arms* |
| a green has tests behind it | `tools/test-count.sh`, `tools/ci-report-count.sh` | *build and test, locked* |

`tools/tests/vectors-arms.sh` is deliberately NOT in that `t_*.sh` glob, and
this is a narrowing with a reason that covers exactly what it narrows: those
arms need `openscad`, and nothing yet establishes that the forge runner has it.
CI's probe step now looks for it, so the next run answers the question. Until
then the vectors run locally and under reaper, and saying so is better than a
CI step that is green because it never ran. The arms REFUSE rather than skip
when openscad is absent, so wherever they do run, absence is reported.

Nothing else is mechanized yet. Where a rule above has no row, it is carried by
the reviewer and by the pull request body, and saying so is better than implying
a gate that does not exist.

## Recording a mutation kill

Two forms, either is enough:

1. A `MUTATION:` line in a commit message naming what was broken and which test
   died.
2. A directory under `tests/mutations/` holding `subject`, `find`, `replace` and
   `expect-fail` (and optionally `test-cmd`). `tools/mutate.sh --all` re-runs
   every one of them, so this form stays true rather than becoming a claim about
   the past.

Prefer the second for anything that will still matter next month.

**A mutant that survives is a finding, and it gets written down as a survivor**
rather than deleted or quietly re-aimed. Two live examples are in
`tests/mutations/0-mutation-record.md`: a clause that survives alone because
another guard is independently sufficient, and a formula's coefficient that
survived every test in the crate because both tests compared ratios.

## Pull requests

Use `.forgejo/pull_request_template.md`. It asks three questions, and the
reason it asks them is that each has been got wrong here already:

- **which tiers you touched, and which you skipped and why** — a skipped tier
  named is a decision; a skipped tier unmentioned is an accident.
- **the mutation kills** — the mutant, and the test that died.
- **any narrowing** — in the body, where it will be read.

## Running the gates locally

```
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
for t in tools/tests/t_*.sh; do sh "$t"; done
tools/mutate.sh --all
tools/check-mutation-record.sh <base> HEAD
```

CI runs the tooling arms and the mutation gate *before* it runs cargo, so that
the instruments reporting on the build are themselves exercised first.
