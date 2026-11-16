# The red push -- four mutants, four kills, and the gate that found it

## What happened

Milestone 2b was pushed to origin and CI failed in 6 seconds. Locally the
suite, clippy, fmt and all 157 mutants were green. CI also runs
`tools/tests/t_*.sh`, and nothing local ran those.

`t_artifact_paths.sh` had been failing since 2b's FIRST commit:

```
FAIL the brace walk did not return to depth zero, so the scan may be reading a test module as live code:
       UNBALANCED crates/wm-csg/src/tree.rs
```

## The file was never wrong

`tools/audit-file-writes.py` tracks brace depth to skip exactly the
`#[cfg(test)]` module's body. It counted every brace in the file, literals
included. `tree.rs` parses OpenSCAD's `name(args) { children }` grammar, so it
necessarily carries:

- `b'{'` and `b'}'` char literals (the grammar's own delimiters),
- an escaped `'{{'` inside a format string,
- tests that put UNBALANCED delimiters inside string literals ON PURPOSE,
  because that is the parser behavior under test.

Three surplus opening braces, correct Rust, and a correct refusal. The
docstring had anticipated this exact case and chosen to report rather than
trust, which is why it reported instead of silently misreading the file.

Exempting the file was the wrong fix: an exemption is a file the audit
permanently stops reading, and the audit's only job is to see every writer
anybody adds. So the walk now strips comment and literal CONTENT first,
preserving length and newlines so reported line numbers still point at the
right line.

| mutant | the lie it tells | killed by |
| --- | --- | --- |
| `scanner-counts-literal-braces` | braces inside literals are code braces | `the walk is counting literal braces again` |
| `scanner-quote-index-off-by-one` | `'x'`'s opening quote is at i+1 | `lifetime or length handling is wrong` |
| `scanner-unbalanced-refusal-removed` | a desynchronized walk can be trusted | `stripping literals disabled the refusal it was meant to preserve` |
| `scanner-reports-the-blanked-text` | report the stripped line, not the source line | `the writer after the test module was missed, or its line number moved` |

## The fix's own bug, found by its own test

`scanner-quote-index-off-by-one` is not hypothetical -- it is what the first
version of the fix actually did. `q` was meant to index the opening quote,
which is `i` for `'x'` and `i+1` for `b'x'`; it was `i+1` in both cases, so
every plain char literal fell through to the lifetime branch and the brace
inside it survived. It was LATENT in `tree.rs`, which only contains the `b'{'`
form, and the lifetime fixture arm caught it on the first run.

## mutate.sh learned a second failure shape

`scanner-*` are the first mutants with a non-Rust subject. A Rust test dies as
`test <name> ... FAILED`; a shell arm dies as `  FAIL <message>`. The first run
MISFIRED -- the mutation applied, the arm failed, and refusal 4 could not read
the format, so a real kill was reported as a build break.

Refusal 4's guarantee is unchanged: the expected label must appear IN A FAILURE
LINE. A bare substring match on the whole transcript would also match the label
sitting in a passing `ok` line, which is how a mutant that broke something else
entirely reads as a kill.

## And the class, not just the instance

A gate CI runs that no local command runs is a gate discovered by a red push.

- `tools/battery.sh` is now the single definition of the battery, with every
  gate reporting its own exit code.
- `tools/tests/t_battery_covers_ci.sh` mechanically asserts battery.sh runs
  every gate `ci.yml` does, one-directionally: the battery may hold more, never
  less. It refuses if it extracts too few gates to be seeing the workflow, and
  it self-tests against a fixture where a CI-only gate must be named.

That arm found two further gaps on its first run: CI runs
`check-mutation-record.sh` and `ci-report-count.sh`, and the battery ran
neither -- it called `test-count.sh`, the layer UNDERNEATH CI's reader. Near
enough to look right, and not the same gate. That is how a list kept in two
places drifts.
