# Mutation record: the mutation-record gate and the mutation driver

## tools/check-mutation-record.sh

| mutation | arm(s) killed |
| --- | --- |
| the refusal never fires | an added assertion with no record is REFUSED |
| `MUTATION:` grep always matches | an added assertion with no record is REFUSED |
| `tests/mutations/` grep always matches | an added assertion with no record is REFUSED |
| base-ref resolution unchecked | **SURVIVED alone** |
| the above **and** merge-base rc unchecked | **SURVIVED as a pair** |
| the above **and** the empty-merge-base check | an unresolvable base REFUSES rather than passing |

THREE INDEPENDENTLY SUFFICIENT GUARDS sit on the range, and that is why neither
the single nor the double mutant bites. Removing the base `rev-parse` leaves
`git merge-base` to fail; removing its rc check too leaves `[ -n "$mb" ]` to
catch an empty result. Only with all three gone does the gate clear a range it
cannot see, and then arm E fails with rc 0 -- passing where it must refuse,
which is the exact failure the arm exists for.

Recorded because a reader meeting the single surviving mutant would reasonably
delete the `rev-parse` as dead. It is not dead, it is redundant on purpose, and
it is the only one of the three that names WHICH ref was bad.

## tools/event-before.py

| mutation | arm killed |
| --- | --- |
| an all-zero `before` sha accepted | a branch creation REFUSES: no before-state |

## tools/mutate.sh

Driven by hand against this repo, with the subject's checksum compared before
and after every run:

| refusal | how it was driven | result |
| --- | --- | --- |
| the `find` text is absent | a mutant whose find text is not in the file | rc 71, named |
| the `find` text occurs more than once | find = `cantilever_stiffness`, 6 occurrences | rc 71, count reported |
| the baseline is already red | `test-cmd` = `false` | rc 72, "ALREADY RED" |
| the wrong test died | a mutant introducing a type error | MISFIRED, rc 2 |
| restore after a normal run | checksum before and after all four | byte-identical |
| restore after a SIGNAL | TERM sent with the mutation CONFIRMED on disk | byte-identical |

The signal arm is worth its own line because the first attempt proved nothing:
the slow command was the BASELINE, so the TERM arrived before any mutation
existed and "restored" was vacuous. The run that counts polls until the mutated
text is observed on disk, signals then, and says so.

## Tiers

Touched: Tier 1 over a hermetic git repository the test builds itself.
Skipped: every other tier, the subject being shell and a JSON reader.
