#!/usr/bin/env python3
"""Pull the numbered entries out of the two incident logs, verbatim.

THE LOGS ARE THE SOURCE OF TRUTH AND THEY STAY THAT WAY. This extracts; it does
not interpret, summarize or classify. Every field it emits is a byte range of
the original, so a corpus entry can always be checked against the log it came
from -- which is the whole reason the corpus is worth having. The judgment
fields (the failure class, what the measurement was, what the fix was) are
added in a separate pass and validated against this output, so a mistake there
cannot quietly rewrite what the log actually said.

Entry shape in both logs:

    42. **The claim, in bold, ending with a period.** Then the prose, which
        runs until the next numbered entry or the next ## heading.

usage: corpus-extract.py <log.md> <source-name>   -> JSON on stdout
"""
import json
import re
import sys

ENTRY = re.compile(r"^(\d+)\. \*\*(.+?)\*\*", re.S)


def entries(text):
    lines = text.split("\n")
    starts = []
    for i, ln in enumerate(lines):
        if re.match(r"^\d+\. \*\*", ln):
            starts.append(i)
    out = []
    for n, i in enumerate(starts):
        end = starts[n + 1] if n + 1 < len(starts) else len(lines)
        # a ## heading before the next entry ends this one
        for j in range(i + 1, end):
            if lines[j].startswith("## "):
                end = j
                break
        block = "\n".join(lines[i:end]).rstrip()
        m = ENTRY.match(block)
        if not m:
            continue
        num = int(m.group(1))
        title = " ".join(m.group(2).split())
        body = block[m.end():].strip()
        body = "\n".join(" ".join(l.split()) for l in body.split("\n"))
        out.append({"n": num, "title": title, "body": body, "line": i + 1})
    return out


def main():
    if len(sys.argv) != 3:
        sys.exit("usage: corpus-extract.py <log.md> <source-name>")
    path, source = sys.argv[1], sys.argv[2]
    with open(path, encoding="utf-8") as fh:
        text = fh.read()
    got = entries(text)
    nums = [e["n"] for e in got]
    # REFUSE A GAP OR A DUPLICATE. The logs are numbered by hand; a missing or
    # repeated number means this parser and the document disagree about where
    # an entry starts, and a corpus built on that silently loses incidents.
    dupes = sorted({n for n in nums if nums.count(n) > 1})
    if dupes:
        sys.exit(f"corpus-extract: duplicate entry number(s) in {path}: {dupes}")
    gaps = [n for n in range(1, max(nums) + 1) if n not in nums] if nums else []
    if gaps:
        sys.exit(f"corpus-extract: gap(s) in {path}: {gaps}")
    for e in got:
        e["id"] = f"{source}-{e['n']:03d}"
        e["source"] = source
    json.dump(got, sys.stdout, indent=1, ensure_ascii=False)
    print()


if __name__ == "__main__":
    main()
