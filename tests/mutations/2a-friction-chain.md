# Mutation record: friction chains

| mutation | killed by |
| --- | --- |
| the effective radius becomes the MEAN radius | the_logged_land_radius_is_the_effective_one_not_the_mean |
| capacity ignores the radius | the_thread_out_resists_the_pucks_land_by_the_ratio_of_their_radii |
| the chain reports the STRONGEST interface as first to slip | the_thread_out_resists_the_pucks_land_by_the_ratio_of_their_radii |
| an empty chain reports that nothing gives way | an_empty_chain_is_refused_rather_than_reported_as_safe |
| margins are raw capacities, not normalised to the weakest | a_shared_load_and_friction_cancel_out_of_the_margins |

## A figure in the log that does not reconcile

Buildability entry 72 says the thread out-resists the puck's land "3.7:1",
with the land at 8.4 mm and the thread at 34 mm. **Those radii give 4.05:1.**
Reaching 3.7 would need a thread radius of 31.08 or a land radius of 9.19, and
the entry states neither.

The rest of the entry is sound, which is what makes it worth recording rather
than crossing out. The 8.4 is not the mean of 5 and 11, which is 8.0: it is
the annular effective friction radius, 8.375, which is the correct physics.
And the entry's fused ratio of 34 against 26 checks out exactly at 1.308
against the "about 1.3:1" it claims.

So one figure in a sentence of three is wrong while the reasoning around it is
right, and the entry's conclusion survives either way -- which is exactly the
condition under which a wrong number lives for ever: it is load-bearing for
nothing. The test asserts the arithmetic and says so; the live buildability
log has it as entry 78.

THE CORPUS VALIDATOR CANNOT CATCH THIS CLASS. It checks that every number in
an entry APPEARS in that entry, not that the numbers agree with one another.
That limit was already written down and this is its first instance.

## The shortcut and its boundary

With load and friction shared, `mu*N*r` reduces to `r` and the ranking is
purely geometric -- which is why the harness analysis could be done without
knowing either. One arm asserts the cancellation across an 18x load change and
a 6x friction change. Another shows the shortcut BREAKING when the loads are
not shared, where a smaller radius clamped harder wins.

## A mutant that refused to apply

`cargo fmt` reflowed the empty-chain guard's string continuation after the
mutant was written, so its find text matched nothing and mutate.sh refused
rather than patching nothing and reporting green. Refusal 1, earning its keep.

## Tiers

Touched: Tier 1. Skipped: every other tier.
