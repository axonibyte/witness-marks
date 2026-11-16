#!/bin/sh
# The corpus must still describe the logs it came from.
#
# A CORPUS IS A DERIVED ARTIFACT, and a derived artifact that outlives its
# source becomes a confident, precise, wrong description of something that
# changed -- failure class C, which this corpus itself catalogues. The logs are
# vendored beside it so that relationship is checkable here instead of pointing
# at a path outside the repository.
#
# WHAT IT CHECKS: every incident's title is still the title of that numbered
# entry in the vendored log, and the entry count matches. What it does NOT
# check is whether a classification is a good one; no mechanical check can, and
# claiming otherwise would be the kind of overstated instrument the corpus is
# about.
set -u
ROOT=${CORPUS_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
pass=0; fail=0
ok() { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no() { fail=$((fail+1)); printf '  FAIL %s\n' "$1"; }

python3 - "$ROOT" <<'PY'
import re, sys, tomllib
from pathlib import Path
root = Path(sys.argv[1]); c = root / "tests" / "corpus"
LOGS = {"build": "source-buildability-tests.md", "mcp": "source-openscad-mcp-notes.md"}
ENTRY = re.compile(r"^(\d+)\. \*\*(.+?)\*\*", re.S)
bad = 0; good = 0
for source, fname in LOGS.items():
    log = c / fname; toml = c / f"{source}.toml"
    if not log.is_file() or not toml.is_file():
        print(f"  FAIL {source}: {log.name} or {toml.name} is missing"); bad += 1; continue
    titles = {}
    lines = log.read_text(encoding="utf-8").split("\n")
    starts = [i for i, l in enumerate(lines) if re.match(r"^\d+\. \*\*", l)]
    for n, i in enumerate(starts):
        end = starts[n+1] if n+1 < len(starts) else len(lines)
        for j in range(i+1, end):
            if lines[j].startswith("## "): end = j; break
        m = ENTRY.match("\n".join(lines[i:end]))
        if m: titles[int(m.group(1))] = " ".join(m.group(2).split())
    # A MALFORMED CORPUS FILE MUST REPORT, NOT CRASH. Emptying mcp.toml made
    # this raise KeyError, which exits nonzero and so "fails" -- but a traceback
    # is not a verdict, and an operator reading one learns nothing about which
    # file is wrong or how. Driven by emptying a corpus file.
    try:
        doc = tomllib.loads(toml.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as exc:
        print(f"  FAIL {source}: {toml.name} is not valid TOML: {exc}"); bad += 1; continue
    inc = doc.get("incident")
    if not isinstance(inc, list):
        print(f"  FAIL {source}: {toml.name} carries no [[incident]] array; it is empty or restructured")
        bad += 1; continue
    # REFUSE AN EMPTY SIDE: zero incidents all matching is vacuously true.
    if not inc or not titles:
        print(f"  FAIL {source}: {len(inc)} incident(s) against {len(titles)} log entr(ies); one side is empty so nothing is compared")
        bad += 1; continue
    if len(inc) != len(titles):
        print(f"  FAIL {source}: {len(inc)} incident(s) but the log has {len(titles)} entr(ies)"); bad += 1
    else:
        print(f"  ok   {source}: {len(inc)} incident(s), the same count as the log"); good += 1
    drift = [e["id"] for e in inc if titles.get(e["n"]) != e["title"]]
    if drift:
        print(f"  FAIL {source}: {len(drift)} title(s) no longer match the log: {drift[:4]}"); bad += 1
    else:
        print(f"  ok   {source}: every title still matches its numbered entry"); good += 1
    empties = [e["id"] for e in inc if not e["class"]]
    print(f"  ok   {source}: {len(empties)} entr(ies) carry no class, which is an answer and not a gap")
    good += 1
print(f"-- {good}/{good+bad} arms pass")
sys.exit(1 if bad else 0)
PY
