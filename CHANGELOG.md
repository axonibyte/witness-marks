# Changelog

## [Unreleased]

## [0.4.0] - 2026-11-16

Milestone 2b, Geometry over the provider: the ten tools that need a geometry
kernel underneath them. Where 2a could be checked against arithmetic, every
call here is a question about a shape, and most of them exist because a
plausible cheaper instrument gives the wrong answer.

### Added
- `wm-csg::tree`: the OpenSCAD CSG dump parsed into a tree, with
  `derived_from` carrying which operands a node's position came from.
- `wm-csg::bbox` / `bbox_at`: a box with three states -- known, empty, and
  unknown-with-a-reason -- because an unmeasurable box is not an empty one.
  `bbox_at` applies ancestor transforms, with the destination handled before
  accumulation so it is not applied twice.
- `wm-csg::eats`: which subtractions remove material from a feature, compared
  in the ROOT frame. The first version compared local boxes and reported that
  the mouth cut did not reach the socket.
- `wm-csg::coincident`: a derived position landing on another feature's
  boundary. The dovetail sockets sat at exactly the mouth cut's half-angle.
- `wm-csg::flank`: material between a feature and the nearest void.
- `wm-csg::locate` / `at`: the three ways to find nothing -- no such module,
  declared and never called, called and then consumed -- and the boundary as
  its own answer rather than a rounding of its neighbors.
- `wm-asm::fits`: the mating-pair matrix with the sign checked. A retainer
  modeled with a clearance retains nothing.
- `wm-asm::travel`: what stops a part first. The blade's static interference
  at rest was exactly zero and it still could not travel.
- `wm-asm::insert`: whether it can be gotten IN, which a clear operating sweep
  is not evidence about. Both directions of that mistake have happened.
- `wm-model::diff`: geometric diff where an empty diff is an answer. A
  bit-identical result after an edit is the edit not applying -- `-D` appends
  an assignment, so a misspelled name defines a variable nothing reads, which
  is legal and draws no warning.
- `wm-process::support`: the slicer's criterion, with the angle convention in
  the type.

### Changed
- New crate `wm-model`, holding Layer E's one tool the provider does not have.
  The rest of Layer E exists upstream and is better than what this project
  would write.

### Fixed
- Measured and recorded: a 45-degree test cannot catch an inverted overhang
  convention. With `d = layer_h * tan(theta)` in place of `/`, the 45-degree
  fixture passes and the 26.565-degree one fails. tan(45) is 1, so the angle
  everyone quotes is the one angle that cannot test the thing everyone gets
  wrong.

### Fixed
- The brace walk in `tools/audit-file-writes.py` counted braces inside string,
  char and byte literals and comments, so `wm-csg/src/tree.rs` -- a parser for
  a brace grammar, which must contain `b'{'`, an escaped `'{{'`, and tests with
  deliberately unbalanced delimiters inside strings -- was reported UNBALANCED
  from 2b's first commit. The walk now strips literal and comment content
  first. Found by a red push: the local battery had run the suite, clippy, fmt
  and the mutants, and not `tools/tests/t_*.sh`.
- `tools/battery.sh` is now the single definition of the battery, and
  `t_battery_covers_ci.sh` asserts it runs every gate `ci.yml` does. That arm
  immediately found two more: CI runs `check-mutation-record.sh` and
  `ci-report-count.sh` and the battery ran neither.
- `tools/mutate.sh` reads a shell arm's `FAIL` line as a kill, not only a Rust
  test's `FAILED`. The first mutant with a Python subject MISFIRED on a real
  kill. Refusal 4's guarantee is unchanged: the label must appear in a failure
  line, never merely somewhere in the transcript.

### Known gaps
- Carried: the FreeBSD reaper guest has no Rust toolchain; the vector arms sit
  outside the glob CI runs pending the openscad probe.
- FOUR of milestone 2b's ten issues ask for provider wiring that did not land.
  Issue 23 names a specific acceptance test (port `overhang-scan.scad` and
  validate against four hand-computed shapes); 31, 34 and 35 say "over provider
  `check mode=motion`", "via provider measure and sections", and "over provider
  probe and features". What shipped for those four is the judgment half, with
  the measurement half mocked in the fixtures. They stay open.
- `solid.locate`/`at` and `proc.support` take their geometry as already-
  measured inputs -- attributions, signed distances, regions. Wiring them to
  the provider's probe and section calls is milestone 4's work, and until then
  these are the judgment half of each tool with the measurement half mocked in
  the fixtures.

## [0.3.0] - 2026-11-16

Milestone 2a, Owned layers: mechanics and process. The ten closed-form calls
Witness Marks owns because the provider does not do them. Nothing here needs a
geometry kernel; every answer is checkable against arithmetic.

Milestone 2 was split at the owner's word, and the seam is whether the work
needs the provider rather than a count. `wm-process: support` moved to 2b
mid-milestone for that reason: its own body says "over provider sections", and
the Layer 4 half of it -- the critical angle -- shipped here.

### Added
- `wm-process::Process`: the process as a typed model where every field is
  optional so a missing input can be REFUSED rather than defaulted, and the
  refusal names both the field and the caller.
- `critical_angle_deg`: 45 degrees is folklore with exactly one true case.
- `quantize`: a thickness is whole layers, and the error that matters is cubed.
- `reachable_retract_speed`: commanded and achieved are two numbers.
- `fits_bed`, `MeshFormat` (binary by default), `SliceRequest`.
- `wm-mech::Beam` with cantilever and fixed-guided ends, `Screw`,
  `friction_chain`, `PressFit`, and the tolerance `stack`.

### Fixed
- Buildability log entry 72 states a 3.7:1 ratio that its own radii make
  4.05:1. Recorded as entry 78 rather than silently corrected; the entry's
  other figures check out and its conclusion survives either way.

### Known gaps
- Carried: the FreeBSD reaper guest has no Rust toolchain; the vector arms sit
  outside the glob CI runs pending the openscad probe.
- `wm-mech::press` covers cylindrical fits only, and refuses slots by design.


## [0.2.0] - 2026-11-16

Milestone 1, Provider and envelope: all eight issues. Witness Marks now talks
to the pinned provider and wraps every answer in a verdict that refuses to
overstate itself. Still no Layer answers a geometry question of its own.

### Added
- `wm-provider`: a std-only JSON-RPC stdio client for the pinned provider. A
  reader thread so a silent server is a timeout and not a hang, id matching so
  an interleaved notification is not mistaken for an answer, and refusal at
  startup if the server is not the pinned one.
- `Status::derive`: `ok | empty | refused | failed` from what the provider
  said, with provenance hashes, kernel, and the measured subject attached.
- `wm-provider::view`: detects parameters that move or hide geometry and pins
  the ones that move, reporting both.
- `wm-provider::controls`: known-positive controls beside every probe, so a
  probe that cannot bite is reported blind rather than clear.
- `wm-provider::artifact`: a producer that clears its target first and returns
  no path on failure.
- `wm-provider::jobs`: job handles, so a long call is not a lost turn, and a
  worker that dies says so instead of reading as busy.
- `wm-provider::project`: part enumeration and staleness, where a part never
  exported is never called current.
- `witness-marks::budget`: a cap on our own tool surface, with the arm that
  makes it bite while we still register no tools.
- `tools/audit-file-writes.py` and `t_artifact_paths.sh`: nothing creates a
  file outside `artifact.rs`.

### Changed
- `provider/PROVIDER.toml` records the serverInfo version separately from the
  PyPI version. They are different schemes: the pin is 0.6.1 and the server
  advertises 4.2.0, so a refusal written against the wrong one refuses every
  run.
- `tests/vectors/`: seven vectors carry the provider's measured answers, so
  the set is cross-implementation in fact rather than in intent.

### Fixed
- A claim committed in v0.1.0 said neither side could compute component count.
  The provider computes it. The finding is about our side: a binary STL has no
  component structure for the mesh reader to see.

### Known gaps
- Carried from v0.1.0: the FreeBSD reaper guest has no Rust toolchain, and the
  vector arms sit outside the glob CI runs pending the openscad probe.
- `render`, `check` and `get_project_files` have typed entry points only
  through the generic `call`; the three with dedicated wrappers are `measure`,
  `validate` and `scad_eval`.


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
