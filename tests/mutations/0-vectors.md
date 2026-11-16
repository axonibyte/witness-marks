# Mutation record: the cross-implementation vectors

Two subjects: the vector DATA and the RUNNER. Driving one does not drive the
other, and that distinction is the whole finding here.

## The vectors (data)

| mutation | killed by |
| --- | --- |
| `sphere-fn64-volume` expectation edited to the analytic 4188.790205 | the tolerance comparison |
| `frame-blade-empty` asked for 0.0 instead of empty | the empty branch |
| a model changed under a fixed expectation | the cube arm |

## The runner (code)

| mutation | killed by |
| --- | --- |
| the tolerance comparison replaced by `True` | a wrong expectation FAILS |
| the empty check replaced by `True` | a vector claiming empty over a solid FAILS |
| the output file not deleted before a run | a stale STL does not become the answer |
| a failing run exits 0 anyway | a wrong expectation FAILS |

ALL THREE OF THE FIRST RUNNER MUTANTS ORIGINALLY SURVIVED, and that is the
reason tools/tests/vectors-arms.sh exists. With the tolerance comparison and
the empty check each replaced by `True`, all ten vectors still passed: driving
the DATA proves the data and says nothing about the instrument reading it. A
runner that cannot fail proves nothing, and nothing had noticed.

TWO GUARDS HAD TO BE MADE DRIVABLE BEFORE THEY COULD BE MEASURED.

The stale-file guard could never fire, because the runner made a fresh
`mkdtemp` every run, so `os.unlink(out)` had nothing to delete. It was a
defense against corpus entry mcp-026 -- four probes reporting 52.800 mm3 of
material that was not there, every one a stale STL -- and it was untested
because it was untestable. `$WMVEC_OUT` now pins the output directory so an arm
can plant a stale file where an empty vector will look for one.

The empty check's FAILING direction was never driven. The arm that existed
converted `expect_empty` into a numeric expectation, which takes the other
branch entirely, so the mutant survived while that arm stayed green. A vector
claiming emptiness over a solid cube is what bites.

## Tiers

Touched: Tier 1, over closed forms and committed tessellations. Skipped: every
other tier. NARROWING, stated because it belongs in the summary and not only
here: `vectors-arms.sh` is outside the `t_*.sh` glob that CI runs, because it
needs `openscad` and nothing yet establishes that the forge runner has it. The
CI probe step now looks, so the next run answers it. The arms REFUSE rather
than skip when openscad is absent, so wherever they do run, absence is reported
rather than passed over.
