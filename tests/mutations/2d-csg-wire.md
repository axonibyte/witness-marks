# `solid.locate`/`at` wired -- seven mutants, seven kills

Issue #35 asked for these "over provider probe and features". The judgment
half shipped in 2b with its measurement mocked; this is the measurement.

| mutant | the lie it tells | killed by |
| --- | --- | --- |
| `wire-unknown-state-becomes-air` | a state this reader does not know is air | `a_state_this_reader_does_not_know_is_refused` |
| `wire-on-surface-is-inside` | a point on a face is inside the solid | `a_point_on_the_bore_wall_reads_as_the_boundary` |
| `wire-masked-features-ignored` | a feature the provider cannot see is absent | `a_masked_feature_count_refuses_the_whole_reading` |
| `wire-unresolved-features-ignored` | an incomplete tree is a complete account | `an_unresolved_feature_count_refuses_the_reading` |
| `wire-empty-points-accepted` | an empty point list is a measurement | `an_empty_point_list_is_refused` |
| `wire-unowned-solid-accepted` | solid material needs no owning part | `solid_naming_no_part_is_refused` |
| `wire-counts-patterns-not-entries` | a pattern is one call site | `a_feature_repeated_four_times_reports_four_call_sites` |

## A sample that cannot exhibit a case is not evidence the case is absent

The author's first probe used three points -- deep solid, inside the bore, far
outside -- saw only `solid` and `air`, and concluded the provider had NO
boundary state, that `At::OnBoundary` would have to be recovered by probing a
neighborhood, and said so.

The provider has `on_surface`. None of those three points was on a face.

The fixture now carries all three states on purpose, and the test that pins it
asserts the neighbors a tenth of a millimetre either side read plainly inside
and outside -- which is what makes the boundary reading mean anything.

## The narrowing that is real

`on_surface` arrives with NO tolerance attached. This crate's `OnBoundary`
carries the tolerance it was judged at, because "on the face" means nothing
without one. The provider's threshold is internal and undeclared, so
`PROVIDER_SURFACE_TOL` is `NAN` rather than a plausible number: an invented
figure reads exactly like a measured one.

Same rule for `nearest_face_mm` on the solid and air arms. A probe reports no
distance, so the distance is `NAN` and not a guess.

## Pre-boolean is what makes Consumed readable

The provider's own note: *"circular subtractive features from the CSG tree
(post-expansion, pre-boolean); not holes in the finished part."*

Pre-boolean is exactly `locate`'s "called, and then consumed". A feature that
ran and was later eaten still appears in this list, because the list is the
tree and not the result. So the features list answers *was it instantiated*
and only a measurement of the result answers *did its material survive* --
which is why `surviving` is an argument rather than something the reader
invents.

## The survivor, and the fixture that could not discriminate

`wire-counts-patterns-not-entries` SURVIVED its first run. The fixture held one
pattern with one entry, so counting patterns and counting entries both gave 1.
A second fixture -- four holes from one `for` loop, which the provider reports
as `4x D4.00 ... bolt circle r=14.1 at 45/135/225/315 deg`, one pattern with
four entries -- separates them, and the mutant died.

Twice in one day: the remaining-range fixture stopped at the midpoint of its
sweep, and this one held a count of one. Symmetry and unity in test data both
delete the arm you thought you were testing.
