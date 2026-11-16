# Mutation record: eats

| mutation | killed by |
| --- | --- |
| `bbox_at` accumulates the destination node's own transform twice | the_cut_that_shears_the_socket_is_named |
| `eats` compares LOCAL boxes instead of root-frame ones | the_cut_that_shears_the_socket_is_named |
| containment in the difference's first child is not checked | a_cut_in_a_different_branch_does_not_eat_it |
| the first child of a difference counts as a subtrahend | the_first_child_is_not_a_subtrahend |
| a path naming nothing returns an empty list | a_path_that_names_no_node_is_refused |

## Two frame bugs, both found by running it

The first version compared `bbox(target)` against `bbox(subtrahend)`. Those are
each in their OWN frame, with ancestor transforms not applied, so a socket
translated into the path of a cut reads as nowhere near it. The first run
reported that the cut which sheared the dovetail sockets did not reach them,
which is the exact wrong answer this function exists to prevent.

The fix, `bbox_at`, then had the opposite bug: `bbox` already applies a
multmatrix's own matrix to its children, so accumulating it again on the way
down transformed the destination TWICE and placed the cut at double its
translation. Same symptom, opposite cause.

Both are the same class: a confident number computed in the wrong coordinate
system. Neither would have been visible without a fixture whose answer was
known in advance.

## A path that names nothing is refused

An empty list reads as "nothing eats it" -- the most reassuring possible
answer to a question that was never actually asked. The existence check is
also kept SEPARATE from the measurement, because `bbox_at` returning None for
a bad path would be indistinguishable from a node that simply cannot be
bounded.

## Candidates, never verdicts

Box overlap is NECESSARY for a subtraction to touch a feature and is not
SUFFICIENT. The asymmetry is the useful one: a subtrahend this does NOT list
cannot be eating the feature, so a short list is a real narrowing.

## Tiers

Touched: Tier 1 over a fixture in the incident's own shape. Skipped: every
other tier; nothing here evaluates geometry.
