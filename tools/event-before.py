#!/usr/bin/env python3
"""Print the push event's `before` sha, or nothing.

Separate from check-mutation-record.sh because a heredoc inside a heredoc is
how this got written wrong the first time, and because "nothing" has to be the
answer for three different broken inputs -- unreadable, not JSON, no key -- each
of which the caller turns into the same REFUSAL. An all-zero sha is a BRANCH
CREATION: there is no before-state to diff against, so it counts as nothing.
"""
import json, sys

if len(sys.argv) != 2:
    sys.exit(0)
try:
    with open(sys.argv[1], encoding="utf-8") as fh:
        doc = json.load(fh)
except Exception:
    sys.exit(0)
if not isinstance(doc, dict):
    sys.exit(0)
before = doc.get("before") or ""
if not isinstance(before, str) or set(before) <= {"0"}:
    sys.exit(0)
print(before)
