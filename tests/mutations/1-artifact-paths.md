# Mutation record: `failed` never returns a path

## The producer (crates/wm-provider/src/artifact.rs)

| mutation | killed by |
| --- | --- |
| the target is not cleared BEFORE producing | a_stale_file_is_not_handed_back_as_an_empty_runs_result |
| the target is not cleared after a failure | a_partial_file_written_before_the_error_is_removed |
| the producer's own claim is trusted over the disk | a_producer_that_claims_success_without_writing_is_empty_not_at |
| an empty result is reported as a failure | an_empty_result_is_empty_not_failed_and_has_no_path |

### A survivor that found a real hole

`artifact-no-clear-before` SURVIVED the first run, and it was right to. The
stale-file test that existed forces a FAILURE, and the clear that runs after a
failure cleans up on its own, so removing the clear-before left the suite
green. The two clears are redundant on that path.

Only a run that SUCCEEDS while writing nothing can tell them apart, and that is
the incident itself: a stale file at the target, this run produces nothing --
the OpenSCAD empty case, exit 0 and no file written -- and without the
clear-before the last run's artifact is handed back as this run's result. mcp
note 26, four flank probes reporting 52.800 mm3 of material that was not there.

The arm now exists because the mutant found its absence, which is the only
reason anyone would have written it.

## The audit (tools/tests/t_artifact_paths.sh)

| mutation | killed by |
| --- | --- |
| a direct `fs::write` appended AFTER a test module | the violation arm |
| a direct `fs::write` placed BEFORE the test module | the violation arm |
| artifact.rs stops clearing the target | the clear-before arm |
| artifact.rs deleted outright | the blind-scan refusal |

### The audit's own defect, found the same way

The scan was a LINE CUT: strip from the first `#[cfg(test)]` to end of file, on
the convention that a crate's test module comes last. A mutant appending a
direct `fs::write` after the test module survived the audit completely. A
convention is not a guarantee, and catching the writer somebody adds without
thinking is the audit's only job. It is a brace walk now
(`tools/audit-file-writes.py`), so code before, after or between test modules
is all still seen, and a file whose depth does not return to zero is REPORTED
rather than silently trusted.

## Tiers

Touched: Tier 1 for the producer against the real filesystem, Tier 3 for the
source audit. Skipped: every tier above.
