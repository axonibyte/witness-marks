# Mutation record: the CSG tree

| mutation | killed by |
| --- | --- |
| argument splitting ignores nesting depth | a_nested_comma_does_not_split_an_argument |
| an empty dump becomes an empty group | an_empty_dump_is_refused_rather_than_read_as_an_empty_group |
| a node with no body and no semicolon is read as a leaf | a_node_with_no_body_and_no_semicolon_is_refused |
| string tracking removed while balancing delimiters | an_unbalanced_delimiter_inside_a_string_is_still_inside_it |

## A survivor, and the arm that looked like it covered the case

The string mutant survived its first run. The arm that existed used
`text = "a [bracket], here"`, whose bracket is BALANCED, so a parser ignoring
quotes counts it up and back down again and parses correctly anyway. Only an
UNBALANCED delimiter needs the string handling: a closing paren inside a label
ends the argument list early for anything that is merely counting depth.

The lesson is the same one the screw fixture taught two milestones running: a
test can exercise the right feature and still not discriminate the right
behaviour.

## An empty dump is not an empty group

OpenSCAD writes NOTHING for a model with no geometry. Returning a cheerful
empty group would make that indistinguishable from a group that really has no
children, which is a thing a model can contain. Refused with the reason.

## The parser does not know any operation

The grammar is `name(args)` then `{...}` or `;`, and that is all it
implements. A dump containing `frobnicate()` parses, which matters because
the node set grows with OpenSCAD. An arm uses an invented operation to pin it.

## A count that disagreed, and why

The real dump holds 29 `group(` and the parser reports 30. The dump has TWO
top-level nodes, which the parser wraps in an implicit group because that is
what OpenSCAD means by them. The test asserts 30 AND asserts the file's 29,
rather than relaxing to a range: the difference is a fact about the parser.

## Tiers

Touched: Tier 1 for the grammar, Tier 3 for the census over the real vendored
dump. Skipped: every other tier. Nothing here executes geometry; the bbox and
`derived_from` halves of this issue are not in this commit.
