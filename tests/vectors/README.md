# Cross-implementation vectors

Methodology tier 1 (`docs/testing-methodology.md`): where two implementations
must agree, neither is the oracle — a committed vector file is. Here the two
sides are Witness Marks' own geometry checks and the pinned provider's answers
to the same question on the same model. Each vector is a model, a question, and
the expected number with its tolerance and the tessellation `$fn` it holds at.
A vector that only one side can compute is a finding about the other side.

## What the first ten found

**Neither side is the oracle, and for five of these neither side is even the
authority.** `cube-volume`, `cube-bbox`, `rotated-cube-bbox`,
`overlap-intersection` have closed forms, so the mathematics decides and both
implementations are being tested against it. The tessellated ones have no
closed form at a given `$fn`; there the recorded value is the agreement, and
the analytic figure is carried alongside as context so a mesh that is not
inscribed is visible.

**"Facets" is not a well-posed question without naming the format**, and that
was found here rather than reasoned out. The live provider reported 1986 facets
for `sphere(r=10)` at `$fn=64`; our CLI reports 4092. Both are right. OFF keeps
1984 quads plus two 64-gon polar caps, which is 1986 faces, and triangulating
them gives 1984*2 + 2*62 = 4092. The vector now names binary STL.

**The two sides agree exactly where they can be compared.** The same model
exported to OFF gave byte-identical files from OpenSCAD 2026.10.14 (ours) and
2026.11.05.nightly (the provider's), sha256
`a968dcfd9c6cdb05641864ff3499066945874cc4752be04fec65f05855de534a`, 93544
bytes. Two different builds, one answer.

**The provider is pinned at 0.6.1 and the reachable server reports 1.0.0.** A
vector set that assumes the pin while talking to a different version is
comparing against something the manifest does not describe. Recorded here
rather than resolved, because which one is wrong is not this file's call.

**A question neither side can answer.** The issue asked for component count
among the first ten. A triangle soup carries no component structure, and the
provider exports rather than measures, so neither side can say how many
separate bodies an STL holds. `two-cubes-facets` records that as a finding
about the FORMAT instead of dropping the question quietly.

## Running them

```
python3 tools/vector-run.py tests/vectors/vectors.toml
sh tools/tests/vectors-arms.sh      # the arms that prove the RUNNER can fail
```

The second matters more than it looks. With the runner's tolerance comparison
and its empty check each replaced by `True`, all ten vectors still passed. A
runner that cannot fail proves nothing, and nothing had noticed.
