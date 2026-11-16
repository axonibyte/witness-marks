# cargo transcripts, for tools/test-count.sh

Four of these are REAL output, captured from `cargo test` rather than written
by hand, because a fixture invented alongside the parser agrees with it by
construction and tests nothing:

| file | how it was made | what it pins |
| --- | --- | --- |
| `green.txt` | this workspace, `cargo test --locked --workspace` | 13 result lines, 11 passing tests |
| `failed.txt` | a throwaway crate with one passing and one failing test | `test result: FAILED. 1 passed; 1 failed` |
| `noresult.txt` | the same crate with a type error | a build failure reports NO result line |
| `empty.txt` | the same crate with no tests at all | `test result: ok. 0 passed` |
| `mixed.txt` | a crate with 3 passing unit tests and 1 failing integration test | an `ok` suite and a `FAILED` suite in ONE transcript |

`malformed.txt` is the one SYNTHETIC file, and it is marked so on purpose:
cargo cannot be made to emit a result line without an `N passed` field, so the
only way to drive the "the format moved under us" branch is to write one. It is
`green.txt` with a single field removed.
