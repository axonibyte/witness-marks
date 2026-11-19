#!/usr/bin/env python3
"""The rediscovery gate (issue 45): can this project's checks re-find the
corpus, and which check does it?

WHAT IT REFUSES, and each refusal is a way a coverage figure gets inflated:
  3  an incident in the corpus with no verdict, or a verdict for an id the
     corpus does not have. A matrix over a subset reads as a matrix.
  4  a `re-found` verdict whose `by` names a path that does not exist, or whose
     `fixture` does not exist. Coverage has to point at something.
  5  a `not-found` or `unassessed` verdict with no `why`.
  6  a verdict value that is not one of the three.

usage: rediscovery.py [--matrix] [--corpus DIR]

Exit 0 when every incident carries an honest verdict. --matrix prints the
counts by verdict and by failure class; the gate itself does not depend on the
numbers, only on their honesty.
"""
import collections
import os
import sys
import tomllib

E_COVERAGE = 3
E_DANGLING = 4
E_NO_REASON = 5
E_BAD_VERDICT = 6
#: How many verdicts may share one reason before it stops being a reason.
#: Small on purpose: a handful of genuinely identical cases is believable, a
#: corpus is not.
PASTE_LIMIT = 5
E_PASTED = 7
VERDICTS = ("re-found", "not-found", "unassessed")


def load(corpus):
    incidents = {}
    for name in ("build.toml", "mcp.toml"):
        path = os.path.join(corpus, name)
        with open(path, "rb") as fh:
            for i in tomllib.load(fh)["incident"]:
                incidents[i["id"]] = i
    with open(os.path.join(corpus, "rediscovery.toml"), "rb") as fh:
        verdicts = tomllib.load(fh)["rediscovery"]
    return incidents, verdicts


def main():
    args = sys.argv[1:]
    corpus = "tests/corpus"
    if "--corpus" in args:
        corpus = args[args.index("--corpus") + 1]
    # CITATIONS ARE REPO-RELATIVE, AND THE REPO IS FOUND FROM THIS FILE, not
    # from the --corpus argument. Resolving them against the corpus directory's
    # parent meant that pointing --corpus at a copy made every citation dangle,
    # so the gate refused its own committed verdicts with rc 4. The arms found
    # it by passing an absolute path, which is the ordinary way a test drives a
    # tool over a fixture world.
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    incidents, verdicts = load(corpus)

    seen = set()
    in_green, outside_green = set(), set()
    whys = collections.Counter()
    for v in verdicts:
        vid = v.get("id", "")
        if vid in seen:
            print(f"rediscovery: `{vid}` carries two verdicts; the later would win "
                  f"silently", file=sys.stderr)
            return E_COVERAGE
        seen.add(vid)
        if vid not in incidents:
            print(f"rediscovery: a verdict names `{vid}`, which is not an incident in "
                  f"the corpus. A verdict for nothing inflates the matrix", file=sys.stderr)
            return E_COVERAGE
        verdict = v.get("verdict", "")
        if verdict not in VERDICTS:
            print(f"rediscovery: `{vid}` has verdict {verdict!r}, which is not one of "
                  f"{VERDICTS}", file=sys.stderr)
            return E_BAD_VERDICT
        if verdict == "re-found":
            for key in ("by", "fixture"):
                p = v.get(key, "")
                if not p:
                    print(f"rediscovery: `{vid}` is re-found and names no `{key}`. "
                          f"Coverage has to point at something", file=sys.stderr)
                    return E_DANGLING
                if not os.path.exists(os.path.join(root, p)):
                    print(f"rediscovery: `{vid}` is re-found by `{p}`, which does not "
                          f"exist. A citation to a missing file is the shape of a "
                          f"record that went false", file=sys.stderr)
                    return E_DANGLING
            # `by` MUST NAME A CHECK, NOT THE DATA IT READS, and existence could
            # not tell the two apart. rook found this (coop wm-review/3, F4): the
            # mcp-026 verdict cited tests/vectors/vectors.toml as both `by` AND
            # `fixture`, and that file is the DATA. The arm that plants the stale
            # file is tools/tests/vectors-arms.sh. A rule that only asks "does
            # the path exist" is satisfied by citing the thing being measured.
            by = v["by"]
            if not (by.startswith("tools/tests/") or by.startswith("tests/mutations")):
                print(f"rediscovery: `{vid}` is re-found BY `{by}`, which is not a "
                      f"check -- a check lives under tools/tests/ or "
                      f"tests/mutations*/. Naming the data a verdict reads is how "
                      f"coverage points at itself", file=sys.stderr)
                return E_DANGLING
            # AND WHETHER THAT CHECK IS INSIDE THE GREEN, which is the figure's
            # real bound. rook, F3: battery.sh and ci.yml both glob
            # `tools/tests/t_*.sh`, so an arm named vectors-arms.sh is run by
            # NEITHER. "Battery green" therefore says nothing about a re-find
            # that only a deliberate arm performs, and the report must not let
            # the two be read as one number.
            if os.path.basename(by).startswith("t_") or by.startswith("tests/mutations"):
                in_green.add(vid)
            else:
                outside_green.add(vid)
        elif not (v.get("why") or "").strip():
            print(f"rediscovery: `{vid}` is {verdict} with no `why`. An unexplained "
                  f"gap is indistinguishable from one nobody looked at",
                  file=sys.stderr)
            return E_NO_REASON
        else:
            whys[(v.get("why") or "").strip()] += 1

    # A REASON REPEATED IS ONE REASON, NOT MANY. rook, wm-review/3: 107 of the
    # 108 verdicts carried the SAME `why`, pasted, and the existence check
    # above passed every one of them. That is the `--seen` guard defeated by
    # incrementing, in another costume: a refusal whose text prescribes how to
    # satisfy it without doing the work. So an identical reason may stand for a
    # handful of genuinely identical cases and not for a corpus.
    for why, n in whys.items():
        if n > PASTE_LIMIT:
            print(f"rediscovery: {n} verdicts carry the SAME `why` ({why[:60]!r}...). A "
                  f"reason repeated {n} times is one reason, not {n}; it says nobody "
                  f"looked at {n - 1} of them. Write what is true of EACH, or say "
                  f"plainly in one of them that the set is unexamined",
                  file=sys.stderr)
            return E_PASTED
    missing = sorted(set(incidents) - seen)
    if missing:
        print(f"rediscovery: {len(missing)} incident(s) carry no verdict "
              f"({', '.join(missing[:5])}{'...' if len(missing) > 5 else ''}). A matrix "
              f"over a subset reads exactly like a matrix over the whole corpus",
              file=sys.stderr)
        return E_COVERAGE

    by_verdict = collections.Counter(v["verdict"] for v in verdicts)
    if "--matrix" in args:
        print(f"-- the rediscovery matrix, {len(incidents)} incidents")
        for name in VERDICTS:
            print(f"   {by_verdict.get(name, 0):>4}  {name}")
        print()
        cls = collections.defaultdict(collections.Counter)
        for v in verdicts:
            c = (incidents[v["id"]].get("class") or "").strip() or "(none fits)"
            cls[c][v["verdict"]] += 1
        print(f"   {'class':<12}{'re-found':>9}{'not-found':>10}{'unassessed':>11}")
        for c in sorted(cls):
            r = cls[c]
            print(f"   {c:<12}{r.get('re-found', 0):>9}{r.get('not-found', 0):>10}"
                  f"{r.get('unassessed', 0):>11}")
        print()
        found = by_verdict.get("re-found", 0)
        print(f"   re-found: {found} of {len(incidents)}, of which "
              f"{len(in_green)} is inside the CI-green set and {len(outside_green)} "
              f"outside it.")
        if outside_green:
            print(f"   THE NUMBER THE BATTERY'S GREEN COVERS IS {len(in_green)}, NOT "
                  f"{found}. battery.sh and ci.yml both glob tools/tests/t_*.sh; a")
            print(f"   re-find performed only by a deliberately-run arm is outside both,")
            print(f"   and CI's runner reports `openscad ABSENT` besides. Outside:")
            for vid in sorted(outside_green):
                print(f"     {vid}")
        # AND WHAT THE FIGURE IS NOT. rook, wm-review/3: with 107 unassessed and
        # 0 not-found, this is not a hit rate -- it says nobody has tried 107 of
        # them. A hit rate needs a denominator of things ATTEMPTED.
        attempted = found + by_verdict.get("not-found", 0)
        if attempted == 0:
            print(f"\n   AND IT IS NOT A HIT RATE: 0 incidents have been ATTEMPTED and "
                  f"rejected,\n   so there is no denominator. "
                  f"{by_verdict.get('unassessed', 0)} are unassessed -- which says "
                  f"nobody\n   has looked, not that the checks cannot find them.")
        else:
            print(f"\n   of {attempted} attempted, {found} re-found "
                  f"({100.0 * found / attempted:.0f}%). "
                  f"{by_verdict.get('unassessed', 0)} still unassessed.")
    else:
        print(f"-- every one of {len(incidents)} incidents carries an honest verdict "
              f"({by_verdict.get('re-found', 0)} re-found)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
