# Mutation record: bed fit, export defaults, slicer handoff

| mutation | killed by |
| --- | --- |
| a rotation is advised for a part that is too TALL | a_part_too_tall_gets_no_rotation_advice |
| the diagonal is advised unconditionally | a_part_too_big_in_both_axes_gets_no_rotation_advice |
| the export default becomes ASCII | the_export_default_is_binary |
| the slicer accepts any file extension | a_binstl_extension_is_refused_before_a_process_is_spent |
| settings are space-joined instead of semicolon-joined | the_argv_carries_the_settings_joined_by_semicolons |

## Advice that cannot work is worse than none

No rotation about Z makes a part shorter, so a part over in Z gets no
suggestion. A part over in both X and Y is not rescued by turning it either.
Both mutants produce advice that would waste somebody's afternoon, and both
arms exist because the diagonal suggestion is tempting to apply blindly.

Rotation is tried about Z ONLY, and that is stated as a limit rather than left
as an oversight: turning a part onto another face changes which surfaces need
support and which direction its layers run, so it is a DESIGN decision with
consequences this function cannot see.

## Binary by default, measured

ASCII is **5.020x** the bytes of binary for the same mesh, measured on the
vendored stage: 202728 against 40384. At that ratio the logged "111 MB
mistake" was about 22 MB of actual geometry. The default is binary so that the
bad outcome has to be chosen rather than arrived at.

## Three quirks of OrcaSlicer 2.3.2, all measured

**It dispatches on the file EXTENSION.** A perfectly good binary STL named
`stage.binstl` -- which `--export-format binstl` invites -- is rejected with
"Unknown file format". The same bytes named `.stl` load fine. The check is
done before a subprocess is spent, because leaving it to the slicer costs a
process launch and a confusing message.

**Its facet count is not the file's.** `--info` reports 788 facets where the
file holds 806, and the difference is exactly the 18 zero-area triangles in
OpenSCAD's export, which it drops on load. Volume agrees to six significant
figures. A facet count is a property of a file AND of the reader.

**The CLI matches the leaf preset name** where the GUI walks the inherits
chain, so a profile that resolves in the GUI may not on the command line, and
the failure reads as a missing preset rather than an unresolved parent.

## Tiers

Touched: Tier 1 for the geometry and the argv construction. The slicer quirks
came from running OrcaSlicer 2.3.2 by hand, which is reported as hand runs and
not as coverage. Skipped: every other tier.
