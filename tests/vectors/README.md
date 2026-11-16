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

**A question only the PROVIDER can answer, and I got this backwards first.**
This section originally said neither side could compute component count,
"and not the provider, which exports and does not measure". That was committed
and it was wrong. The provider's `measure` tool returns `component_count`, and
on two disjoint cubes it returns 2, driven over MCP on 2026-11-16.

The finding is therefore about OUR side. A binary STL is a triangle soup with
no component structure, so the mesh reader this runner uses cannot answer the
question at all, while the provider computes it upstream of the export. That is
exactly the case this file opens with: a vector only one side can compute is a
finding about the other side, and here the other side is ours.

**Where both sides can be compared, they agree.** Measured on the pinned
provider and recorded as `provider = { ... }` beside each vector:

| vector | ours | provider |
| --- | --- | --- |
| cube volume | 1000.0 | 1000.0 |
| rotated cube bbox | 14.142136 | 14.1421 |
| overlap intersection volume | 500.0 | 500.0 |
| two cubes facets | 24 | 24 |
| sphere $fn=64 facets | 4092 | 4092 |
| sphere $fn=64 volume | 4171.998709 | 4171.9988 |
| cylinder $fn=128 volume | 785.082777 | 785.0828 |

The provider rounds to four decimals; the agreement is to the digits it
reports. Nothing here is the oracle: five of these have closed forms, and for
those the arithmetic is.

## A third reader, and a facet count that moves again

OrcaSlicer 2.3.2's `--info` was run on the vendored stage on 2026-11-16, which
makes three independent readers of one file:

| quantity | this runner | OrcaSlicer 2.3.2 |
| --- | --- | --- |
| volume | 40805.4868 | 40805.5117 |
| size x | 46.939 | 46.939133 |
| size z | 31.000 | 31.000000 |
| facets | 806 | **788** |

Volume and bounding box agree to six significant figures. The facet count
differs by exactly 18, which is exactly the number of ZERO-AREA triangles
OpenSCAD's export contains: the slicer drops them on load.

So the earlier finding generalizes. A facet count is not well posed without
naming the export FORMAT, and it is not well posed without naming the READER
either, because a reader may repair. Volume survives both.

## Running them

```
python3 tools/vector-run.py tests/vectors/vectors.toml
sh tools/tests/vectors-arms.sh      # the arms that prove the RUNNER can fail
```

The second matters more than it looks. With the runner's tolerance comparison
and its empty check each replaced by `True`, all ten vectors still passed. A
runner that cannot fail proves nothing, and nothing had noticed.
