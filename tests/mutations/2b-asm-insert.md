# `asm.insert` -- six mutants, six kills

The spec's claim: *"`insert` != `sweep`: clear through the whole range yet
impossible to install, or the reverse -- both happened."*

Two independent questions. A part's operating range is a band it never leaves
once assembled; the insertion path starts outside that band and runs through
geometry no operating sample visits. So a clear operating sweep is not evidence
about installation, and a clean drop-in is not evidence about binding at the
ends of travel.

| mutant | the lie it tells | killed by |
| --- | --- | --- |
| `insert-installation-gap-ignored` | an unchecked insertion is a checked one | `an_operating_sweep_alone_cannot_reach_a_verdict` |
| `insert-operating-gap-ignored` | an unchecked operating sweep is a checked one | `an_insertion_check_alone_cannot_reach_a_verdict_either` |
| `insert-answered-absorbs-unasked` | one gap can be overlooked if something else failed | `a_known_failure_does_not_absorb_the_unasked_question` |
| `insert-short-path-passes` | a path sampled to 0.4 reached the assembled position | `a_path_sampled_short_of_the_assembled_position_is_not_a_pass` |
| `insert-blocked-path-ignored` | a blocked insertion does not count against the part | `clear_through_the_operating_range_and_still_not_installable` |
| `insert-operating-stop-ignored` | a part that seizes in use still works | `installable_and_still_seizes_in_operation` |

## Why `Incomplete` outranks `Fails`

An unanswered question cannot be ruled out by an answered one. A part known to
seize AND never test-fitted has two problems, and reporting only the measured
one leaves the second looking like it was considered. `insert-answered-absorbs-
unasked` is exactly that reading, and the test that kills it holds a measured
operating failure with insertion unchecked.

The `Option`s in `Assembly` carry the same point at the type level: "not
measured" is a third state, because a default-false boolean reads as a failure
and a default-true one reads as a pass, and neither is the truth.
