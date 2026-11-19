#!/usr/bin/env python3
"""The pasted-reason check, in ONE place because it had two homes and one gap.

rook found the gap (thread:wm-review/19, F13): `rediscovery.py` refuses more
than PASTE_LIMIT verdicts sharing a `why`, and `cost.py` -- written the same
week, reading a corpus the same way -- had no such check, with all 108 of its
verdicts carrying ONE reason. I had fixed the class in one file and not its
sibling, which is the failure cal's rule names: when one of these turns up,
assume there are siblings.

A SECOND COPY WOULD HAVE BEEN THE SAME MISTAKE ONE ITERATION LATER, so the
check lives here and both tools import it. `tools/tests/t_cost.sh` also
ENUMERATES the tools that read corpus reasons and requires each to use this
module, so a third tool cannot quietly skip it ("guards enumerate, never
name").
"""

#: How many verdicts may share one reason before it stops being a reason.
#: Small on purpose: a handful of genuinely identical cases is believable, a
#: corpus is not.
PASTE_LIMIT = 5


def pasted(reasons, limit=PASTE_LIMIT):
    """The reason shared by more than `limit` verdicts, with its count.

    Returns `(why, n)` for the worst offender, or `None` when every reason is
    within the limit. The caller refuses on a result -- this returns the
    finding rather than exiting, so each tool keeps its own exit code.
    """
    counts = {}
    for why in reasons:
        key = (why or "").strip()
        if not key:
            continue
        counts[key] = counts.get(key, 0) + 1
    worst = max(counts.items(), key=lambda kv: kv[1], default=None)
    if worst and worst[1] > limit:
        return worst
    return None


def refusal(why, n, limit=PASTE_LIMIT):
    """The sentence a tool prints when `pasted` returns a finding."""
    return (f"{n} verdicts carry the SAME `why` ({why[:60]!r}...). A reason "
            f"repeated {n} times is one reason, not {n}; it says nobody looked "
            f"at {n - 1} of them. Write what is true of EACH, or say plainly in "
            f"one of them that the set is unexamined (limit {limit})")
