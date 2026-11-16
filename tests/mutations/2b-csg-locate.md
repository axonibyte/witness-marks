# `solid.locate` / `solid.at` -- eight mutants, eight kills

Spec §Layer 1: *"`locate` and `at` are how an agent orients in a file it did
not write."*

| mutant | the lie it tells | killed by |
| --- | --- | --- |
| `locate-consumed-is-never-run` | a module whose geometry was eaten never ran | `a_module_whose_geometry_was_entirely_eaten_is_not_a_module_that_never_ran` |
| `locate-undeclared-reported-empty` | a typo'd module name contributes nothing | `an_undeclared_module_name_is_refused_not_reported_empty` |
| `locate-eaten-without-calls-believed` | a module called nowhere can have been eaten | `an_uncalled_module_that_something_claims_to_have_eaten_is_refused` |
| `at-boundary-collapses-to-neighbor` | a point on a face is inside or outside | `a_point_exactly_on_a_face_is_on_the_boundary_not_inside_or_outside` |
| `at-zero-tolerance-allowed` | a zero-width boundary band is a usable tolerance | `a_zero_boundary_tolerance_is_refused` |
| `at-unowned-inside-point-reported` | an inside point needs no contributing module | `an_inside_point_with_no_contributing_module_is_refused` |
| `at-inside-distance-keeps-its-sign` | distance to the face is signed at the call site | `the_distance_to_the_nearest_face_is_positive_from_either_side` |
| `at-tolerance-not-carried` | OnBoundary needs no tolerance attached | `a_point_exactly_on_a_face_is_on_the_boundary_not_inside_or_outside` |

## Three ways to find nothing

Ask where a module's geometry ended up and an empty result can mean:

1. **No such module.** A typo, or a library whose `include` did not resolve --
   which in OpenSCAD is only a WARNING, and the render proceeds without it.
2. **Declared and never called.** It exists; nothing instantiated it.
3. **Called, and then consumed.** It contributed geometry and a later
   difference removed all of it.

The third is the defect: the module ran, the author believes its feature is in
the part, and the part has none of it. Collapsing the three into one empty
bbox is the error Layer 1 already names about `operand_volumes` -- *"an operand
that rendered nothing is the difference between 'these do not touch' and 'I
measured nothing'"* -- one level over, in attribution rather than interference.

`locate-consumed-is-never-run` is that collapse exactly, and its fixture is the
dovetail socket with two call sites, both eaten by the mouth cut.

## Why the boundary is not a rounding

A point lying on a face is a coin flip between inside and outside, and
clearance work is asked *precisely there*: the whole question of a 0.2 mm fit
is what happens at the surface. So `OnBoundary` is its own answer, and it
carries the tolerance it was judged at -- "on the face" has no meaning without
one. `at-tolerance-not-carried` hardcodes a plausible 0.1 mm, which is why the
fixture asserts the tolerance it passed in rather than merely matching the
variant.

A zero tolerance makes `OnBoundary` unreachable and restores the coin flip
through the parameter, so it is refused rather than accepted as "exact."
