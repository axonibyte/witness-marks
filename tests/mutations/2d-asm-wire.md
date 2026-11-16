# `asm.insert` / `travel_stop` wired -- six mutants, six kills

Issue #31 asked for a path sweep "over provider `check mode=motion`". One
`check(mode=motion)` call returns `first_contact` (the offset and the part met
there), `worst` (deepest penetration anywhere), and `min_gap_mm` -- the
measurement half of both `travel` and `insert`.

| mutant | the lie it tells | killed by |
| --- | --- | --- |
| `motion-unresolved-reads-clear` | an UNRESOLVED sweep is a clear one | `an_unresolved_sweep_is_refused_rather_than_read_as_clear` |
| `motion-no-finding-is-clear` | a sweep with no finding travelled freely | `a_sweep_that_produced_no_finding_is_refused` |
| `motion-step-spacing-ignored` | any step spacing is fine to draw a stop from | `a_sweep_too_coarsely_stepped_for_the_callers_limit_is_refused` |
| `motion-fail-without-contact-is-clear` | a FAIL naming no contact is clear | `a_failing_sweep_that_names_no_contact_is_refused` |
| `motion-names-the-shallower-contact` | of two parts met at once, name the lighter | `when_two_parts_meet_at_one_offset_the_deeper_is_named` |
| `motion-empty-contacts-accepted` | a contact with no contacts in it is a contact | `a_first_contact_with_no_contacts_is_refused` |

## UNRESOLVED is not clear

The provider reports distances inside its tessellation error bound as
UNRESOLVED rather than as zero -- the one refusal §03 of the spec credits it
with having that this project lacked. A sweep carrying unresolved rows has not
established that the part travels; it has established that part of the
question fell below the resolution of the instrument.

## Two readings of one payload, still two types

The same sweep answers "what stops it" and "can it be installed", and each has
its own function returning its own type. There is still no conversion between
`Stop` and `Access`: the defect those types prevent is a call site holding one
answer and reading it as both, and sharing a transport does not make them the
same question.

## Two mutants that taught something

`motion-no-finding-is-clear` first FAILED TO BUILD -- the fallback borrowed a
local, and a mutant the compiler kills says nothing about the arm. A `static`
fixed it and it then died properly.

`motion-empty-contacts-accepted` SURVIVED. Removing the empty-contacts guard
still produced an error, from the "a contact names no part" check further
down, so the test's bare `is_err()` passed with the guard gone. The test now
asserts the MESSAGE, so the guard under test is the one that has to fire.
Defense in depth is good; a test that cannot tell which defense fired is not.
