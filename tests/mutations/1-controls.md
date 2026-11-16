# Mutation record: known-positive controls

| mutation | killed by |
| --- | --- |
| a probe with NO controls reads as clean | a_probe_with_no_controls_is_blind_not_clean |
| one dead control still reads as trustworthy | a_control_that_did_not_bite_makes_the_probe_blind_and_says_which |
| the provider's `empty` flag ignored when deciding a bite | emptiness_a_zero_volume_and_an_absent_field_all_fail_to_bite |
| anything at all counts as a bite | emptiness_a_zero_volume_and_an_absent_field_all_fail_to_bite |

## Zero of zero is the mutant that matters

`ran == 0, bit == 0` is vacuously consistent and is EXACTLY what an unexercised
probe reports. Treating it as clean makes controls optional in practice, which
is the same as not having them. A probe with no controls is BLIND.

## The failing direction is driven end to end

The stub grew a `deadprobe` mode that answers every measure as empty, the way a
probe that is not running does. The subject then reads "clear" -- which is the
trap -- and the controls are what tell it apart from a probe that genuinely saw
nothing. Without that arm, the whole mechanism is exercised only in the
direction where it has nothing to say.

## The controls' own numbers are measured

- `deliberate-overlap`: exactly 48.0000 mm3 over 12 triangles.
- `deliberate-ledge`: 168.0000 mm3, and 58.24 mm2 unsupported by the slicer
  criterion at 0.15 mm layers and 36.87 degrees.

Both on OpenSCAD 2026.10.14, 2026-11-16. The commissioning issue said "a
deliberate 48 mm2 ledge"; that figure does not belong to this geometry and is
not carried forward. The 58.24 against the 60 mm2 the shape suggests is the
criterion working: the first overhanging layer may step out by
d = 0.15 / tan(36.87) = 0.2 mm, which trims the flagged perimeter. An arm pins
the models to the figures they were measured to produce, so a control whose
geometry quietly changed stops being the positive anyone reasoned about.

## Tiers

Touched: Tier 1 for the tally, Tier 2 for the controls running through the same
path as the subject against a stub. Skipped: every tier above.
