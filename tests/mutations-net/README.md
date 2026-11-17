# Mutants whose arms need the network

`tools/mutate.sh --all` globs `tests/mutations/*/` and **CI runs it**. A mutant
whose `test-cmd` reaches pypi.org or github.com would make that gate fail, or
flake, for a reason with nothing to do with the code — so these live here
instead, outside the glob, and are run deliberately:

    for d in tests/mutations-net/*/; do [ -d "$d" ] && ./tools/mutate.sh "$d"; done

Same reasoning as `tools/tests/pin-arms.sh` sitting outside the `t_*.sh` glob,
and as the fork's mutation records staying on its own branch: a gate must not
depend on a third party being reachable. Even where the jail *can* reach those
hosts, a registry is a flaky dependency to put inside a gate.

`tools/battery.sh` runs these, because the battery is the deliberate run.
`t_battery_covers_ci.sh` is one-directional for exactly this: the battery may
hold more than CI, never less.

## The five

| mutant | the lie it tells | killed by |
| --- | --- | --- |
| `pin-sdist-mismatch-allowed` | a wrong sdist hash is acceptable | `a wrong sdist hash did not refuse with rc 4` |
| `pin-tag-mismatch-allowed` | a wrong tag commit is acceptable | `a wrong tag commit did not refuse with rc 5` |
| `pin-peel-ignored` | a tag object is a commit | `the peel is being ignored` |
| `pin-missing-pin-skipped` | a PROVIDER.toml with no pins can be verified | `a PROVIDER.toml with no pins did not refuse with rc 6` |
| `pin-kernel-mismatch-allowed` | a different kernel is the same measurement | `a wrong kernel did not refuse with rc 7` |

`pin-peel-ignored` is the one worth reading twice. Our pin is `v0.6.1`, which
is LIGHTWEIGHT — its ref *is* the commit — so swapping the peel order is
invisible on the tag we actually depend on. The arm drives it against `v0.6.0`,
which is annotated: ref `bf08011`, commit `93d04da`. That is the failure
`coop/bonemesh.sh` names by name, and it would have been right by luck here.

## A claim here that was wrong within the hour

This file first said the audit's pin-embedding was **not** mutation-coverable,
because the arm reading the verdict reads the *committed* surface and mutating
the audit changes nothing it can see. The first half is true; the conclusion was
not. A DELIBERATE arm can re-capture a surface and compare the verdict against
what the verifier actually says — which is what `pin-verdict-not-recorded` and
the last arm in `pin-arms.sh` now do.

It was caught by the battery, not by re-reading: the mutant sat in the CI glob
unrun, `--all` found it, and it SURVIVED. A gap declared permanent after one
look is usually a gap nobody tried twice.

| mutant | the lie it tells | killed by |
| --- | --- | --- |
| `pin-verdict-not-recorded` | the surface may record a verdict nobody computed | `the captured surface's pin verdict is not the verifier's` |
