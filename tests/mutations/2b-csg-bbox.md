# Mutation record: per-node bounding boxes

| mutation | killed by |
| --- | --- |
| a transform maps two corners instead of eight | a_rotation_bounds_all_eight_corners_and_stops_being_exact |
| a difference unions its children | a_difference_is_bounded_by_what_it_started_with |
| Unknown poisons an intersection as well as a union | unknown_poisons_a_union_but_not_an_intersection |
| an unknown node falls back to its children's union | an_unboundable_node_names_itself |
| a cylinder uses r1 rather than max(r1, r2) | a_cylinder_is_an_upper_bound_and_says_so |

## Two corners is right for translation and wrong for rotation

Transforming only `lo` and `hi` gives the correct answer for every pure
translation, which is most nodes in most dumps. A box turned 45 degrees has
corners outside the box its two extreme corners describe, and the arm uses
exactly that: a 2x2x2 cube at 45 degrees reaches sqrt(2), not 1.

## The asymmetry of Unknown is the interesting part

Unknown POISONS a union and does NOT poison an intersection. A union is at
least as big as either part, so an unbounded part makes the whole unbounded.
An intersection is no bigger than EITHER part, so one known side remains a
valid upper bound -- it just stops being exact. Getting that backwards throws
away a bound that was still true.

## An unboundable node names itself

The stage's dump contains `rotate_extrude` and `linear_extrude` of polygons,
so the root of the real tree is honestly Unknown. That is the right answer,
not a disappointment: a bbox decides whether two features can possibly touch,
and a wrong one answers that question confidently. The arm pins that it
REFUSES rather than inventing a number for a part whose true extent is known
from the mesh to be 46.9 x 110.2 x 31.0 -- and that its cubes and cylinders
are still bounded, so the walk is not useless.

## Every known answer says whether it is exact

A tessellated cylinder is a prism inscribed in its radius, so the radius is an
over-estimate. A difference is bounded by its first child. A hull contains its
children's union. All three are upper bounds and all three say so, because an
over-estimate is the safe direction for "can these touch" and the wrong
direction for "do these definitely touch".

## Tiers

Touched: Tier 1 over hand-checked shapes, Tier 3 over the real vendored dump.
Skipped: every other tier.
