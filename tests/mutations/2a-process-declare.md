# Mutation record: the process declaration

| mutation | killed by |
| --- | --- |
| a missing field falls back to another instead of refusing | an_undeclared_field_is_refused_and_names_the_caller |
| a misspelled field name reads as an undeclared one | an_unknown_field_name_says_it_is_a_caller_bug |
| `weakest` returns the STRONGEST provenance | the_weakest_provenance_wins |

## A typo is a caller bug, not an absence

`need("layer_height", ...)` must not answer "this process does not declare
layer_height". That is a TRUE SENTENCE ABOUT THE WRONG THING: the field is
`layer_h`, the process declares it, and the caller is misspelled. The refusal
says so.

## Weakest, not strongest

A result derived from a measured layer height and an assumed extrusion width
is an ASSUMED result. Taking the maximum of the ordering is what implements
that, and the mutant that takes the minimum turns every answer into its
best-case input.

## A deliberate leak, removed before it shipped

The unknown-field branch has to name a field that is in no static list, and
the first version reached for `Box::leak` to keep the `&'static str`. That
leaks on every typo, on an error path, where the caller is already having a
bad day. `Missing.wanted` is a `String`.

## Tiers

Touched: Tier 1. Skipped: every other tier; the subject is a struct and a
lookup.
