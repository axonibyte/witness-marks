# Mutation record: CONTRIBUTING.md's citations resolve

`tools/tests/t_docs_cite_reality.sh` reads the "what is mechanized, and where"
table out of CONTRIBUTING.md and checks that every tool and every CI step it
names exists. Six mutants, six kills, each driven by pointing `DOCS_ROOT` at a
COPY of the cited world and breaking it there, so the repository is never
touched.

| mutation | killed by |
| --- | --- |
| a cited CI step renamed in the doc | the step name no longer appears in ci.yml |
| a cited CI step renamed in ci.yml | the same arm, from the other side |
| a cited tool loses its executable bit | the tool arm |
| a cited tool deleted | the tool arm |
| the glob row matches nothing | the glob arm, reporting one unmatched citation |
| the table reshaped from a pipe table to bullets | the blind-parser refusal |

THE LAST ONE IS THE IMPORTANT ARM and it is not about the document. A parser
that stops matching reports "every citation resolves" over ZERO citations,
which is a green with nothing behind it. The test refuses fewer than three
rows and says the table moved or the parser did.

## Two defects of mine in the test itself, both found by driving it

**The subshell.** The first version piped rows into `while`, so every failure
counted inside the loop was discarded with the subshell, and it needed a temp
file to carry the count back out. A test whose failure count can be lost is not
a test.

**Pathname expansion on a citation.** `tools/tests/t_*.sh` is a legitimate
citation -- "all the tooling arms" -- but the shell expanded it against the
CURRENT directory before the loop body saw it, so the glob branch never fired
and each resolved name was checked against a different tree. It passed anyway
while the two trees agreed, which is the worst kind of passing: the mechanism
was wrong and only the answer was right. `set -f` fixes it, and it has to stay
on THROUGH the for-expansion -- my first attempt restored globbing one line too
early and the mutant went on reporting three missing files instead of one
unmatched glob.

## What this cannot prove

That a step named in ci.yml enforces the rule the table attributes to it. A
step could be renamed to match and do nothing. This catches drift and deletion,
not a lie, and the test says so in its own header.

## Tiers

Touched: Tier 3, source-as-data structural tests, which is where this belongs --
the document is the fixture. Skipped: every other tier.
