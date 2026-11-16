# Mutation record: jobs

| mutation | killed by |
| --- | --- |
| `catch_unwind` removed, so a panicking worker stays Running | a_panicking_worker_becomes_failed_and_not_running_for_ever |
| an unknown handle reads as Running | an_unknown_handle_is_unknown_not_running_and_not_failed |
| a late progress note resurrects a finished job | a_late_progress_note_cannot_resurrect_a_finished_job |
| every job gets id 0, so handles collide | ids_are_unique_across_jobs |

## Three states that must not be collapsed

**Running and dead.** A job left Running by a worker that panicked is
indistinguishable from one that is merely slow, so a caller waits for something
that died minutes ago. `catch_unwind` is not defensive decoration: it is what
makes "still running" mean still running.

**Unknown and running.** A handle that names nothing is its own answer.
Reporting Running would have a caller wait for ever; reporting Failed would
have them report a failure that never happened.

**A poisoned registry and an absent job.** If the lock is poisoned the status
is Failed with the reason, not Unknown. Unknown would say "no such job" about a
job that exists.

## The ordering arm

`start_returns_while_the_work_is_still_running` holds the worker open on a
channel rather than racing a sleep, so the assertion is about ORDERING and not
about the test being faster than the work.

## Tiers

Touched: Tier 1, and Tier 8 in spirit -- these are concurrency arms over a
shared registry, though the harness is a handful of threads rather than the
methodology's seeded concurrency tier. Skipped: every other tier.
