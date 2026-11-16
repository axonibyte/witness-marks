#!/usr/bin/env python3
"""List file-creating calls in non-test Rust code.

WHY A BRACE WALK AND NOT A LINE CUT. The first version of this stripped
everything from the first `#[cfg(test)]` to end of file, on the convention that
a crate's test module comes last. A mutant that appended a direct `fs::write`
AFTER the test module survived the audit completely: the convention is a
convention, not a guarantee, and the audit's whole job is to catch the writer
somebody adds without thinking. This tracks brace depth and skips exactly the
`#[cfg(test)]` module's body, so code before, after or between test modules is
all still seen.

Strings and comments are not parsed. A brace inside a string literal in a test
module could desynchronize the walk, so a file whose depth does not return to
zero is REPORTED rather than silently trusted.

usage: audit-file-writes.py <root>   -> one "path:line:text" per finding
"""
import re
import sys
from pathlib import Path

PAT = re.compile(r"fs::write|File::create|OpenOptions|fs::copy|fs::rename|create_dir")
EXEMPT_FILES = {"crates/wm-provider/src/artifact.rs"}


def non_test_lines(text):
    """Yield (lineno, line) outside any #[cfg(test)] module."""
    lines = text.split("\n")
    out = []
    i = 0
    depth_ok = True
    while i < len(lines):
        line = lines[i]
        if "#[cfg(test)]" in line:
            # find the opening brace of the item that follows, then skip to its close
            j = i
            while j < len(lines) and "{" not in lines[j]:
                j += 1
            if j >= len(lines):
                break
            depth = 0
            started = False
            while j < len(lines):
                depth += lines[j].count("{") - lines[j].count("}")
                started = started or "{" in lines[j]
                j += 1
                if started and depth <= 0:
                    break
            if started and depth > 0:
                depth_ok = False
            i = j
            continue
        out.append((i + 1, line))
        i += 1
    return out, depth_ok


def main():
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    total_hits = 0
    findings = []
    unbalanced = []
    for f in sorted(root.glob("crates/**/*.rs")):
        rel = f.relative_to(root).as_posix()
        text = f.read_text(encoding="utf-8", errors="replace")
        if PAT.search(text):
            total_hits += 1
        if rel in EXEMPT_FILES or "/tests/" in rel:
            continue
        lines, ok = non_test_lines(text)
        if not ok:
            unbalanced.append(rel)
        for n, line in lines:
            if PAT.search(line):
                findings.append(f"{rel}:{n}:{line.strip()}")
    print(f"SCANNED {total_hits}")
    for u in unbalanced:
        print(f"UNBALANCED {u}")
    for v in findings:
        print(f"VIOLATION {v}")


if __name__ == "__main__":
    main()
