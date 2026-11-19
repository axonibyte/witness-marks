#!/usr/bin/env python3
"""Where the time went: scope or measurement (issue 61, spec §10 test 6).

THE ONE REFUSAL THAT MATTERS. With nothing assessed, the scope fraction is
0/0 -- and printing "0% of lost time is scope" from no data is a lie that
flatters the project, because test 6's whole worry is that scope costs MORE.
So a fraction is refused until something has been assessed, and the refusal
says how many are outstanding.

usage: cost.py [--corpus DIR] [--report]

exit codes, each meaning a different thing:
  0  every incident carries an honest verdict
  3  an incident with no verdict, a verdict for an unknown id, or two for one
  4  a verdict outside {measurement, scope, unknown}
  5  a verdict with no `why` -- an unexplained judgment is indistinguishable
     from one nobody made
  6  asked for a fraction there is no data for -- and, since 2026-11-19, asked
     anything at all of a corpus with nothing assessed
  7  a reason repeated past the paste limit: one reason, not many
"""
import collections
import os
import sys
import tomllib

# NO BYTECODE. Importing a sibling writes tools/__pycache__/*.pyc, which was
# committed once by a broad `git add` (rook F14) and dirties every tree whose
# Python differs. A stale .pyc has also, in this project's history, been
# EXECUTED in place of the source and produced an impossible probe result.
sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import corpus_reasons  # noqa: E402  -- the path above is what makes it importable

E_COVERAGE = 3
E_BAD_VERDICT = 4
E_NO_REASON = 5
E_NO_DATA = 6
E_PASTED = 7
VERDICTS = ("measurement", "scope", "unknown")


def main():
    args = sys.argv[1:]
    corpus = "tests/corpus"
    if "--corpus" in args:
        corpus = args[args.index("--corpus") + 1]
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    base = corpus if os.path.isabs(corpus) else os.path.join(root, corpus)

    incidents = {}
    for name in ("build.toml", "mcp.toml"):
        with open(os.path.join(base, name), "rb") as fh:
            for i in tomllib.load(fh)["incident"]:
                incidents[i["id"]] = i
    with open(os.path.join(base, "cost.toml"), "rb") as fh:
        verdicts = tomllib.load(fh).get("cost") or []

    seen = set()
    for v in verdicts:
        vid = v.get("id", "")
        if vid in seen:
            print(f"cost: `{vid}` carries two verdicts; the later would win silently",
                  file=sys.stderr)
            return E_COVERAGE
        seen.add(vid)
        if vid not in incidents:
            print(f"cost: a verdict names `{vid}`, which is not an incident in the "
                  f"corpus", file=sys.stderr)
            return E_COVERAGE
        if v.get("verdict") not in VERDICTS:
            print(f"cost: `{vid}` has verdict {v.get('verdict')!r}, not one of "
                  f"{VERDICTS}", file=sys.stderr)
            return E_BAD_VERDICT
        if not (v.get("why") or "").strip():
            print(f"cost: `{vid}` carries no `why`. An unexplained judgment about "
                  f"where a day went is indistinguishable from one nobody made",
                  file=sys.stderr)
            return E_NO_REASON

    # F13 (rook, wm-review/19): a `why` that EXISTS is not a `why` that means
    # anything, and all 108 verdicts here shared ONE. rediscovery.py has had
    # this refusal since F4; this file, written the same week and reading a
    # corpus the same way, did not. The check lives in corpus_reasons so there
    # is one implementation rather than two that can diverge again.
    hit = corpus_reasons.pasted(v.get("why") for v in verdicts)
    if hit:
        print(f"cost: {corpus_reasons.refusal(*hit)}", file=sys.stderr)
        return E_PASTED

    missing = sorted(set(incidents) - seen)
    if missing:
        print(f"cost: {len(missing)} incident(s) carry no verdict "
              f"({', '.join(missing[:4])}{'...' if len(missing) > 4 else ''}). A "
              f"fraction over a subset reads like a fraction over the corpus",
              file=sys.stderr)
        return E_COVERAGE

    tally = collections.Counter(v["verdict"] for v in verdicts)
    assessed = tally["measurement"] + tally["scope"]

    # TEST 6's CLAIM, CHECKED RATHER THAN RESTATED. The spec says zero logged
    # entries record building the wrong thing. The day one does, this line
    # changes and the claim stops being true.
    #
    # F12 (rook, wm-review/19): THIS PRINTED THE LIE THE DOCSTRING ABOVE
    # REFUSES. `tally["scope"] == 0` is satisfied by an empty assessment, so
    # with nothing assessed the tool said "test 6 still holds of this corpus:
    # 0 of 108 incidents record building the wrong thing" and exited 0. That is
    # the flattering 0/0 reworded as a count -- and an arm in t_cost.sh
    # asserted it, so the defect was a committed expectation. Zero scope
    # findings among zero assessments is not evidence that scope costs nothing;
    # it is evidence that nobody looked.
    if assessed == 0:
        print(f"-- spec §10 test 6 is UNANSWERED over this corpus: 0 of "
              f"{len(incidents)} incidents are assessed, so the 0 scope findings "
              f"are an absence of looking and not an absence of scope cost")
        print(f"cost: nothing is assessed, so neither the fraction nor test 6's "
              f"claim can be answered. {tally['unknown']} incident(s) are "
              f"unknown", file=sys.stderr)
        return E_NO_DATA
    if tally["scope"] == 0:
        print(f"-- spec §10 test 6 still holds of this corpus: 0 of {assessed} "
              f"ASSESSED incidents record building the wrong thing "
              f"({tally['unknown']} still unknown)")
    else:
        print(f"-- spec §10 test 6 NO LONGER HOLDS: {tally['scope']} incident(s) "
              f"record building the wrong thing, where the spec says zero")

    if "--report" in args:
        print(f"-- {len(incidents)} incidents")
        for name in VERDICTS:
            print(f"   {tally.get(name, 0):>4}  {name}")
        # NO SECOND `assessed == 0` CHECK HERE. It used to live only in this
        # block, which is exactly how F12 happened: the refusal was behind
        # --report while the flattering claim printed either way. The check is
        # now above and unconditional, so a copy here would be a second
        # authority on the same question -- the redundancy a surviving mutant
        # already taught me to collapse in wm-model's matching.
        share = 100.0 * tally["scope"] / assessed
        print(f"\n   of {assessed} assessed, {tally['scope']} scope and "
              f"{tally['measurement']} measurement -> {share:.0f}% scope")
        print(f"   ({tally['unknown']} unknown, and the fraction says nothing about "
              f"those)")
    else:
        print(f"-- every one of {len(incidents)} incidents carries an honest verdict "
              f"({assessed} assessed, {tally['unknown']} unknown)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
