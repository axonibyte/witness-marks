# Mutation record: the provider client

Six mutants, six kills, all recorded under `tests/mutations/provider-*` so
`tools/mutate.sh --all` re-runs them.

| mutation | killed by |
| --- | --- |
| the serverInfo version check removed | a_server_with_the_wrong_version_is_refused_at_startup |
| the server name check removed | a_server_with_the_wrong_name_is_refused_at_startup |
| the answer is the first line that parses, not the matching id | an_interleaved_notification_is_not_mistaken_for_the_answer |
| a non-JSON text block is dropped | an_error_result_keeps_its_prose |
| a PROVIDER.toml naming no expectations yields blank ones | a_file_without_expectations_is_refused |
| `hints` discarded while errors and warnings are kept | hints_survive_as_objects |

## THREE OF THESE FIRST REPORTED MISFIRED, AND THE TOOL WAS RIGHT

`cargo test -p wm-provider` stops at the FIRST FAILING TEST TARGET. Each of
these guards is covered by both a lib unit test and an integration test, so the
lib target failed, cargo never ran the integration target, and the name the
mutant expected never appeared in the output. `mutate.sh` reported MISFIRED --
"the suite failed but X is not named in the failure" -- rather than counting a
nonzero exit as a kill.

That is the refusal earning its place on its first real use. Without it, three
mutants would have been recorded as killed by tests that had not run. The fix
is `--no-fail-fast` in each mutant's `test-cmd`, so every target runs and the
kill is attributable.

## A defect in the test harness, found by running it

The stub server's mode was `$STUB_MODE`, set with `std::env::set_var` in the
spawn helper. Cargo runs tests as THREADS IN ONE PROCESS, so all nine protocol
tests shared that variable and raced: four failed in a batch and all nine
passed when run one at a time, which is the signature. The mode is an argument
now, because an argument belongs to the process it starts.

## What the stub can and cannot prove

It proves the CLIENT: the handshake past a non-JSON banner, id matching against
an interleaved notification, the timeout on a silent server, the refusals, and
that structured content wins over the text rendering while non-JSON prose
survives. It proves nothing about the PROVIDER. That pairing was driven once by
hand through `cargo run -p wm-provider --example smoke`, and its output is in
the commit.

## Tiers

Touched: Tier 1 (parsing, version comparison, diagnostics) and Tier 2,
component conformance, against a stub that speaks the protocol. Skipped: every
tier above that; nothing here crosses a server boundary into the real provider
except the one hand run, which is reported as one hand run.
