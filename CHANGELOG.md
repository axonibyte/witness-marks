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
