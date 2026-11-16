# Mutation record: tolerance chains

| mutation | killed by |
| --- | --- |
| the lever is dropped from a link's contribution | the_same_slack_looks_harmless_without_its_lever |
| the dominant link is the loosest tolerance | the_dominant_link_is_not_the_loosest_one |
| the verdict reads RSS instead of the worst case | worst_case_exceeds_rss_and_the_verdict_uses_the_worst_case |
| the untenable threshold is 100% instead of 50% | the_threshold_is_half_the_working_deflection |
| a negative lever subtracts instead of contributing | a_negative_lever_still_contributes_its_magnitude |

## Without the lever it looks harmless, which is the whole incident

0.05 mm of slot slack against 0.35 mm of working deflection is 14%, and nobody
would question it. Through the clip's 3.54 lever it is 51%, and it cost half
the clamp. Nothing on a drawing says the lever is there. The mutant that drops
it produces exactly the reassuring number that let the original defect ship.

## The dominant link is not the loosest one

A +/-0.05 tolerance on an 8x lever contributes 0.40; a +/-0.20 on a 1x lever
contributes 0.20. The tighter tolerance dominates, and tightening the loose
one would be wasted effort. The mutant that ranks by tolerance picks the wrong
link to go and fix.

## Worst case, not RSS, and the threshold is half

Both are reported and the verdict reads the WORST CASE. RSS is right for a
production run where errors are independent; a printed one-off is one sample
off one machine on one day with one systematic bias, and choosing the more
flattering statistic there is choosing it for the wrong reason.

The threshold is 50%, not 100%. By the time the stack equals the whole
deflection the mechanism is dead. At half it has already lost half its force,
which is precisely the logged incident.

## A negative lever is a direction, not a cancellation

A chain that let opposing levers subtract would be reporting a BEST case while
calling it a worst case.

## Tiers

Touched: Tier 1, against the logged incident's own arithmetic. Skipped: every
other tier.
