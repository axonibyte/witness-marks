# Mutation record: the CI passing-test count

Ten mutants, all killed, each by the arm that targets it. Driven with
`COUNT_SRC` / `REPORT_SRC` pointing at a mutated copy, never by editing the
subject in place. Every mutant was checked to differ from the original and to
be valid shell before it was run; a mutant that fails to apply reports
"DID NOT APPLY" rather than a green.

## tools/test-count.sh

| mutation | arm(s) killed |
| --- | --- |
| FAILED-suite guard removed | a red transcript is REFUSED; ok suites beside a FAILED one; the old-pipeline comparison |
| no-result guard removed | a build failure is not "zero tests" |
| zero-total guard removed | a real zero run is named as a zero run |
| unparsed-line guard removed | an unreadable result line is a parser finding |
| refusal also prints the count | a real zero run is named as a zero run |

The last mutant also killed the green arm, and that kill is an ARTIFACT: the
mutation inserts an `echo` without removing the original, so green prints its
count twice. The kill that counts is the zero arm, where a refusal gains a
number on stdout. Recorded rather than quietly accepted, because a kill for the
wrong reason is not evidence about the assertion it appears to confirm.

## tools/ci-report-count.sh

| mutation | arm(s) killed |
| --- | --- |
| FAILED suite no longer fails the run | a FAILED suite fails even when cargo says 0; ok suites beside a FAILED one do not rescue it |
| cargo rc ignored | a good count does not rescue a nonzero cargo |
| build failure treated as pass | no suite reported is named as a build failure |
| zero-test run treated as pass | a real zero is named as a zero |
| unparsed line treated as pass | an unparsed line is named as tooling, not code |

## Tiers

Touched: Tier 1, as unit arms over committed fixtures. Skipped: every other
tier. The subject is two shell scripts and the transcripts they parse; there is
no component boundary, no server contract, no browser and no concurrency in it.
