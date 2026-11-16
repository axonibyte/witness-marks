# Mutation record: the rediscovery corpus

## tools/corpus-build.py, the validator

Seven refusals, each driven against a two-entry miniature source so the failure
is legible:

| bad input | refusal |
| --- | --- |
| a class outside A-G | `class 'Z' is not one of A-G or empty` |
| an empty judgment field | `state is empty` |
| a class_why over 25 words | `class_why is 30 words, over the 25-word bound` |
| the same id twice | `appears more than once` |
| an extracted id never classified | `extracted from the log and never classified` |
| an id the extractor never produced | `not an id the extractor produced` |
| a number not in the source body | `measurement carries the number '1234.567', which is not in the log entry` |

THE LAST ONE IS THE REASON THE VALIDATOR EXISTS. A reader summarizing 108
incidents will eventually produce a figure that is plausible, useful, and not
in the source, and that is failure class B. Building the corpus that catalogues
class B by committing one would be a poor start.

## tools/tests/t_corpus.sh, the drift check

| mutation | killed by |
| --- | --- |
| a corpus title edited away from the log | the title arm |
| an entry removed from the vendored log | the count arm |
| a vendored log deleted | the missing-file arm |
| a corpus file emptied | the no-`[[incident]]` refusal |
| a corpus file made invalid TOML | the parse refusal |

THE LAST TWO ARE A FIX, NOT A FEATURE. Emptying mcp.toml first killed the test
with a bare `KeyError: 'incident'` traceback. That exits nonzero, so it counted
as a kill, and it was still wrong: a traceback is not a verdict and an operator
reading one learns neither which file is bad nor how. Both now report by name.

## What is NOT covered, and cannot be

Whether a classification is a GOOD one. The validator checks that a number is
real and that a field is filled; it cannot check that a sentence is a fair
summary of an incident. Ten of 108 entries carry no class at all because none
of the seven fits, and that is recorded as an answer rather than smoothed away.

## Tiers

Touched: Tier 3, source-as-data, twice over -- the logs are the fixture for the
corpus, and the corpus is the fixture for the drift check. Skipped: every other
tier; nothing here executes the model.
