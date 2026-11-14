# Changelog

## [Unreleased]

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
