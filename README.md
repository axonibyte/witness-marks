# witness-marks

Hands and a tape measure for agents that model in OpenSCAD.

An MCP server that lets an AI *see, measure, and make promises about* the parts
it designs — and never says "nothing there" by staying silent. The design is
`docs/spec.md`; the short version is its §00.

## What it is, in six pieces

| Piece | The question it answers |
|---|---|
| **Edit** | what is this file, what did I just change in the *shape* |
| **See** | what does it look like from here, in section, with one part lit |
| **Measure** | how big, how many pieces, how much air under a face, how much do two parts overlap and *where* |
| **Fit & move** | does it move through its whole travel, can it be put *in* |
| **Physics, small** | will it hold — four closed-form formulas, no FEA |
| **Promises** | is it still true — assertions that re-check on every change and say which just went false |

Wrapped in one rule: **`empty` is a value, never an absence.** Every result
carries a status, a provenance hash, and the controls that proved the probe
could have found something before it says it didn't.

## What it is built on

Geometry comes from a **provider**: [RobertCoop/openscad-mcp](https://github.com/RobertCoop/openscad-mcp),
pinned in `provider/` (version, PyPI sha256, git tag) and spoken to over MCP
stdio — never imported. It already does measurement, sections, interference
with witness points, and motion sweeps well; Witness Marks owns what it does
not: mechanics, claims with staleness and dominance, process-as-model with the
slicer criterion, construction-tree queries, async jobs, and the envelope as a
protocol rule. `docs/spec.md` §03 is the audit and the reasoning.

## Layout

    crates/wm-envelope    the result envelope: status, provenance, controls, subject
    crates/wm-provider    the provider client (MCP stdio, pinned)
    crates/wm-csg         Layer 1b — the construction tree: eats, flank, coincident
    crates/wm-mech        Layer 3  — beam, screw, friction chain, press, tolerance chain
    crates/wm-process     Layer 4  — printer, material, firmware, slicer criterion
    crates/wm-claims      Layer 5  — assert, verify, stale, dominance, fanout
    crates/witness-marks  the server: NDJSON JSON-RPC over stdio, no framework
    provider/             the pin: pyproject.toml + uv.lock + PROVIDER.toml
    docs/spec.md          the design (canonical)
    docs/testing-methodology.md   vendored from reaper; NORMATIVE for this repo
    tests/vectors/        cross-implementation vectors: ours vs the provider's

## Building

    cargo test --locked --workspace      # the toolchain is pinned in rust-toolchain.toml
    cargo clippy --locked --workspace --all-targets
    (cd provider && uv sync)             # the provider, at exactly the pinned version

## Testing

`docs/testing-methodology.md` is normative here. The rules that bind every
change: never weaken a test, check, assertion or lint to route around a defect;
every fix ships with the test that would have caught it; every new assertion is
mutation-checked and the check is recorded in the PR; a pre-existing failure is
proven pre-existing by stashing. The acceptance gate for the harness itself is
the rediscovery test — revert a fixed defect and confirm the battery finds it.
The 106 logged incidents behind `docs/spec.md` are the first corpus.

## Status

Skeleton. Every crate exports one symbol and one test; `wm-envelope` and
`wm-mech` carry real code with law-based tests. The server answers
`initialize`, `ping` and `tools/list` (empty) over stdio. Nothing measures yet.
Work is tracked as issues on the forge.

## License

Copyright Axonibyte Innovations, LLC. This repository is licensed **in parts**,
and which part a file falls under is decided by what the file *is*, not where it
sits:

| What | Terms |
| --- | --- |
| Source code | Blue Oak Model License 1.0.0, reproduced in full in `LICENSE` |
| Language and protocol specifications | Open Web Foundation Agreement 1.0 |
| Documentation and diagrams | Creative Commons Attribution 4.0 International, `LICENSES/CC-BY-4.0.txt` |
| Logos and trademarks | excluded from every grant above, All Rights Reserved |

`license = "BlueOak-1.0.0"` is set on `[workspace.package]` and inherited by
every crate, which is the code half only; the manifest field has no way to
express the split, so `LICENSE` governs and this table restates it.

`docs/spec.md` is **the specification**, so it is Open Web Foundation, not
CC-BY. `docs/testing-methodology.md` and the README are documentation, so they
are CC-BY.

These terms replace the BSD-2-Clause chosen on 2026-11-14; the relicense landed
2026-11-16 as PR #49 and the holder line is settled on the organization rather
than an individual.
