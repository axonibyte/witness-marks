# Mutation record: coincident

| mutation | killed by |
| --- | --- |
| ancestors are compared against their descendants | a_child_flush_with_its_parent_is_not_a_finding |
| the axis check is removed | a_value_shared_across_two_different_axes_is_not_a_coincidence |
| derivations are dropped from the report | a_coincidence_carries_how_the_value_could_be_derived |
| `corner_excess` returns the half-width instead of the excess | a_rectangle_flush_with_a_cylinder_overhangs_at_its_corners |
| `corner_excess` accepts a radius that is not one | corner_excess_refuses_a_circle_that_is_not_one |

## Ancestors are the noise, and skipping them is the feature

A node's box is contained in its parent's, so a lone cube IS its group's
extent and coincides with it by construction. Reporting those buries the real
finding under one entry per level of nesting, and a real dump nests deeply:
the vendored stage has 47 multmatrix nodes.

## A shared value across different axes is not a coincidence

The axis mutant survived its first run, because in every arm that existed the
only matching values happened to lie on the same axis anyway. A first cube
reaching x = 10 and a second reaching y = 10 have nothing to do with each
other: one is a length and the other a width. A tool that called them flush
would report a finding about two numbers that merely print the same.

## A derivation is what makes a coincidence dangerous

Two features flush by accident are a number somebody typed twice. Two features
flush because one of them is `mouth_deg / 2` are ONE PARAMETER reaching two
places, and a change to it moves both. The derivations ride along so that
difference is visible.

## And it reports without judging

Two features deliberately flush are a design. A coincidence is a flag that a
position is not independent of a boundary; whether that is wanted is for
somebody who knows what the features are.

## Tiers

Touched: Tier 1 over both logged incidents' arithmetic. Skipped: every other
tier; nothing here evaluates geometry.
