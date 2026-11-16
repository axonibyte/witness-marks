# `asm.travel_stop` -- six mutants, six kills

The incident is buildability 74: the blade's static interference at rest was
**exactly zero**, every check passed, and it still could not travel. The frame
took it at 1.0 mm of advance (0.009 mm3, two pieces, one per jamb) and 0.158 at
1.5. The design intended the *can* to stop it.

| mutant | the lie it tells | killed by |
| --- | --- | --- |
| `travel-clear-counts-as-hit` | a clear sample (volume 0) counts as a contact | `zero_at_rest_and_stopped_by_the_frame_further_along` |
| `travel-empty-sweep-is-clear` | a sweep with no samples travels freely | `an_empty_sweep_is_refused_rather_than_reported_clear` |
| `travel-coarse-sweep-allowed` | any sample spacing is fine to draw a stop from | `a_sweep_too_coarse_to_trust_is_refused` |
| `travel-names-the-shallower` | of two parts contacting at once, name the lighter | `when_two_parts_contact_together_the_deeper_one_is_named` |
| `travel-remaining-is-elapsed` | report travel used as travel left | `the_remaining_range_is_a_fraction_of_the_sweep` |
| `travel-trusts-the-listed-order` | the first contact is the first one listed | `samples_need_not_arrive_in_order` |

## The one that survived first, and why

`travel-remaining-is-elapsed` SURVIVED its first run. The fixture swept 0 to 1
and stopped at **0.5** -- where elapsed and remaining are the same number, so
the assertion passed whichever of the two the code returned. Moving the stop to
0.25 made the two values 0.25 and 0.75 and the mutant died immediately.

A fixture has to contain the *distinguishing* case. Symmetry in test data is
not neutral; it silently deletes the arm you thought you were testing.
