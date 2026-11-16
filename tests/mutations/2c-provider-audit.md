# Provider truth -- seven mutants, seven kills

## What this closes

`docs/spec.md` draft 4 said it of itself: *"every [exists] badge was read from
the provider's own API.md, **not run**."* Fifteen claims about a dependency,
taken from its documentation.

All fifteen are now measured against **OpenSCAD MCP Server 4.2.0**, protocol
2024-11-05, 12 tools, 20321 bytes of schema: **15 measured, 0 failed, 0
unverified.** Thirteen decided from the tool surface, two from behavior.

| mutant | the lie it tells | killed by |
| --- | --- | --- |
| `claims-unknown-modes-pass` | a tool that didn't enumerate its modes has the ones we wanted | `unknown modes did not produce UNVERIFIED` |
| `claims-missing-mode-passes` | a missing mode is not a failed claim | `removing measure's probe mode did not fail the L297 claim` |
| `claims-missing-surface-is-silence` | no surface means no claims failed | `a missing surface did not refuse with rc 6` |
| `claims-empty-surface-accepted` | a surface with no tools can be judged | `a surface with no tools was not refused` |
| `claims-control-not-read` | the empty-result control need not be checked | `the empty-result control did not bite` |
| `audit-identity-ignores-version` | the name alone identifies the server | `identity_matches is wrong` |
| `audit-missing-pin-not-refused` | a directory with no PROVIDER.toml can be audited | `the audit did not refuse an unidentified provider directory` |

## Two things measurement changed that reading could not

**Modes are not in the schema.** `mode` is a bare string with a default and no
enum, so a schema reader sees none of them. The first version of the audit
parsed modes out of each tool's prose and found 4 of `measure`'s 9 and 2 of
`check`'s 6 -- missing `probe`, `features` and `motion`, which are exactly the
three the spec's claims rest on. That surface would have reported those claims
FAIL. The fix is to ask: an invalid mode draws `mode must be one of: ...`, the
server enumerating itself.

`claims-unknown-modes-pass` is the guard that keeps this honest in the other
direction. `None` (the tool did not say) and `[]` (the tool has no modes) must
stay distinct, or "I could not read the answer" becomes a finding about the
provider.

**Two servers answer to "openscad-mcp" on this machine.** One is the pinned
`OpenSCAD MCP Server` 4.2.0 with 12 tools; the other an unrelated `openscad-mcp`
1.0.0 with 11 render-only tools, wired into an agent session. Reading the second
and reporting on the first produced a confident, wholly wrong conclusion that
the provider boundary was fiction -- delivered to the owner as a finding, with a
recommendation built on it.

So the audit reconciles `serverInfo` against `PROVIDER.toml` BEFORE capturing
anything, with distinct exit codes: 3 unreachable, 4 identity mismatch, 5 a
surface with no tools. An unreachable provider is not "no claims failed", and
`audit-identity-ignores-version` pins the half that actually went wrong -- the
name matched, and the version did not.

`identity_matches` is a pure function for exactly this reason: the launch needs
uv, a lockfile and OpenSCAD and cannot run in the forge jail, and the part that
was wrong in practice has to be drivable everywhere.

## One narrowing, found by running it

The spec's L436 badge says an empty result is reported as empty rather than
passing silently. True. But its own OpenSCAD evidence says an empty 3D result
says "empty" while an empty 2D one is reported as a **type error** -- two
different diagnoses -- and the provider normalizes both into one `empty_output`
hint with zero errors. That distinction does not survive the boundary.

Recorded as its own claim rather than a note, so that if the provider ever
recovers the 2D type error the claim fails and the spec gets corrected. A
narrowing written only in prose is a narrowing that goes stale silently.

## Narrowing of this arm itself

A stale snapshot passes. What `t_provider_claims.sh` proves is that the claims
agree with the surface AS CAPTURED; the surface records the serverInfo it came
from, and re-running `tools/provider-audit.py` is what makes it current. The
audit needs a live provider, so like the vector arms it sits outside what the
forge jail can run.
