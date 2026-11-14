# Cross-implementation vectors

Methodology tier 1 (`docs/testing-methodology.md`): where two implementations
must agree, neither is the oracle — a committed vector file is. Here the two
sides are Witness Marks' own geometry checks and the pinned provider's answers
to the same question on the same model. Each vector is a model, a question, and
the expected number with its tolerance and the tessellation `$fn` it holds at.
A vector that only one side can compute is a finding about the other side.
