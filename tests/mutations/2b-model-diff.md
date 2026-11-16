# `model.diff` -- eight mutants, eight kills (one by the compiler)

The incident, from the spec: *"a bit-identical result after a change was the
tell that the change had not applied."*

The mechanism is `-D`. OpenSCAD's `-D name=value` **appends** an assignment, so
a misspelled parameter name defines a variable nothing reads. That is legal --
an assignment nobody reads draws no warning -- and the render proceeds,
succeeds, and emits the bytes it emitted before. Exit zero, no diagnostics, no
change.

| mutant | the lie it tells | killed by |
| --- | --- | --- |
| `diff-edit-identical-not-reported` | an identical result after an edit says nothing | `a_bit_identical_result_after_an_edit_is_the_edit_not_applying` |
| `diff-name-not-reported` | report that it did nothing, without the name | `a_bit_identical_result_after_an_edit_is_the_edit_not_applying` |
| `diff-refactor-move-not-reported` | a refactor may move geometry | `a_refactor_that_moved_geometry_is_a_failure` |
| `diff-unhashed-is-compared` | a snapshot with no hash can be compared | `an_unhashed_snapshot_is_refused_rather_than_compared` |
| `diff-keyed-on-volume` | equal volume means equal geometry | `a_part_that_moved_without_resizing_is_still_a_change` |
| `diff-only-walks-the-before-map` | a parameter present on one side only is not a change | `a_parameter_present_on_only_one_side_counts_as_changed` |
| `diff-net-volume-is-signed-added` | a shrink is a negative addition | `a_shrink_is_removed_volume_not_negative_added_volume` |
| `diff-intent-case-left-unhandled` | one intent/identical combination may go unhandled | **the compiler**: non-exhaustive patterns |

## The same empty diff means opposite things

Nothing in two snapshots distinguishes "the edit did nothing" from "the
refactor correctly changed nothing". Only the caller's stated intent does, so
`Intent` has no default -- a default would make the tool wrong half the time,
silently.

`diff-intent-case-left-unhandled` is the strongest of the eight because it is
not a test at all: the match over `(Intent, identical)` is exhaustive, so a
combination cannot be left unhandled even deliberately. Two earlier attempts at
pattern-guard mutants (`if false` on an arm) also failed to build for this
reason, which is how the guard was found.

## Why the hash and not the volume

A part that moved 10 mm has exactly the volume it had. `diff-keyed-on-volume`
is the tool that would report "no change" for every pure translation, and the
fixture that kills it is a translated part with its volume held constant. The
hash decides whether anything changed; the volumes only describe the change.
