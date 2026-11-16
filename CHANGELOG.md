# Changelog

## [Unreleased]

## [0.1.0] - 2026-11-16

Milestone 0, Foundations: the seven issues that had to land before any
measurement in this repository could be believed. No Layer answers a geometry
question yet; what exists is the apparatus that will refuse to lie about one.

### Added
- `tools/test-count.sh` and `tools/ci-report-count.sh`: the passing-test count
  CI prints, with a distinct exit per state. The previous inline pipeline made
  a missing `bc`, a build failure, and a genuinely empty suite into the same
  `0`, under a message asserting the third.
- `tools/mutate.sh`: a named mutation, applied to a pointed-at subject, the
  suite run, the TARGETED test asserted to fail, and the subject restored from
  a trap on every exit path including a signal. Four refusals.
- `tools/check-mutation-record.sh` and `tools/event-before.py`: a change that
  adds assertions must record a kill. The range comes from the push event and
  the gate REFUSES when it cannot resolve one.
- `CONTRIBUTING.md` and a pull request template, with a table of which rules
  are mechanized and which a reviewer carries. `tools/tests/t_docs_cite_reality.sh`
  holds that table's citations to the files and CI steps they name.
- `tests/corpus/`: 108 logged incidents as structured TOML, classified against
  the spec's seven failure classes, with the two source logs vendored beside
  them. Every number in the corpus was checked to appear in its own log entry.
- `tests/fixtures/can_cutter/`: the harness that generated those incidents,
  pinned by hash, with every number beside it measured on those exact files.
- `tests/vectors/`: the first ten cross-implementation vectors and a runner,
  plus the arms that prove the runner can fail.

### Changed
- `LICENSE` is now a parts split: Blue Oak Model License 1.0.0 for code, Open
  Web Foundation 1.0 for the specifications, CC-BY-4.0 for documentation.
- `.reaper.toml` pins the Rust image by digest, resolved from the registry and
  verified to carry the toolchain `rust-toolchain.toml` asks for.
- CI runs the tooling's own arms and the mutation gate BEFORE cargo, so the
  instruments reporting on the build are exercised first.

### Fixed
- `wm-mech::cantilever_stiffness` had an untested coefficient. Both existing
  tests compared RATIOS, in which a constant factor cancels, so the suite was
  green with the function returning k/3. Found by the new mutation driver on
  its first run; pinned now against a hand-computed 41.890139850 N/mm.

### Known gaps
- The FreeBSD reaper guest has no Rust toolchain, so the battery does not run
  on the platform the server ships on. A provisioning finding on template 9004;
  the arm reports ABSENT rather than passing.
- `tools/tests/vectors-arms.sh` is outside the glob CI runs, because it needs
  `openscad` and nothing yet establishes the forge runner has it. The probe
  step now looks.


### Added
- Workspace skeleton: seven crates, one symbol and one test each; `wm-envelope`
  carries the result envelope with `Empty` as a serialized value, and `wm-mech`
  carries cantilever stiffness with law-based tests (linear in E, cubic in t).
- Stdio NDJSON JSON-RPC server answering `initialize`, `ping`, `tools/list`.
- Provider pin: `openscad-mcp==0.6.1` with PyPI sha256 and git tag sha.
- `docs/spec.md` (the design, draft 4) and `docs/testing-methodology.md`
  (vendored from reaper, normative).
- `LICENSE`: BSD-2-Clause, copyright Axonibyte Innovations, LLC, chosen by the
  owner 2026-11-14; `license` set on `[workspace.package]` and inherited by
  every crate, so the tree is no longer all-rights-reserved by default.
