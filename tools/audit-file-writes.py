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

WHY LITERALS ARE STRIPPED FIRST. The walk used to count every brace in the
file, including the ones inside string, char and byte literals and comments.
That held until a file arrived that had to contain them: crates/wm-csg/src/
tree.rs parses OpenSCAD's `name(args) { children }` grammar, so it carries
`b'{'` and `b'}'` char literals, an escaped `'{{'` inside a format string, and
tests that put UNBALANCED delimiters inside string literals on purpose. Three
surplus opening braces, correct Rust, and the audit reported it UNBALANCED --
the refusal working exactly as designed, on a file that was never wrong.

Exempting that file was the wrong fix: the audit's job is to see every writer
anybody adds, and an exemption is a file it permanently stops reading. So
`strip_literals` blanks comment and literal CONTENT, preserving length and
newlines so reported line numbers still point at the right line, and only then
does the walk count braces. Lifetimes are the one real ambiguity -- `'a` is not
an unterminated char literal -- and are distinguished by lookahead.

A file whose depth still does not return to zero is REPORTED rather than
silently trusted.

usage: audit-file-writes.py <root>   -> one "path:line:text" per finding
"""
import re
import sys
from pathlib import Path

PAT = re.compile(r"fs::write|File::create|OpenOptions|fs::copy|fs::rename|create_dir")
EXEMPT_FILES = {"crates/wm-provider/src/artifact.rs"}


def strip_literals(src):
    """Blank the content of comments and literals, preserving length and lines.

    Same length in, same length out, and every newline kept where it was, so a
    line number taken from the result is a line number in the original.
    """
    out = list(src)
    n = len(src)
    i = 0

    def blank(a, b):
        for k in range(a, min(b, n)):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = src[i]
        # comments
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            j = src.find("\n", i)
            j = n if j < 0 else j
            blank(i, j)
            i = j
            continue
        if c == "/" and i + 1 < n and src[i + 1] == "*":
            # RUST BLOCK COMMENTS NEST, so this counts rather than
            # searching for the first `*/`.
            depth = 1
            j = i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth += 1
                    j += 2
                elif src.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            blank(i, j)
            i = j
            continue
        # raw strings: r"..", r#".."#, br##".."##
        if c in "rb":
            k = i
            if src[k] == "b" and k + 1 < n and src[k + 1] == "r":
                k += 1
            if src[k] == "r" and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
                h = k + 1
                while h < n and src[h] == "#":
                    h += 1
                if h < n and src[h] == '"':
                    hashes = h - (k + 1)
                    close = '"' + "#" * hashes
                    j = src.find(close, h + 1)
                    j = n if j < 0 else j + len(close)
                    blank(i, j)
                    i = j
                    continue
        # byte and ordinary strings
        if c == '"' or (c == "b" and i + 1 < n and src[i + 1] == '"'):
            j = i + (2 if c == "b" else 1)
            while j < n:
                if src[j] == "\\":
                    j += 2
                    continue
                if src[j] == '"':
                    j += 1
                    break
                j += 1
            blank(i, j)
            i = j
            continue
        # char and byte-char literals, and the lifetime that looks like one
        if c == "'" or (c == "b" and i + 1 < n and src[i + 1] == "'"):
            # q INDEXES THE OPENING QUOTE, which is `i` itself for 'x' and
            # i+1 for b'x'. Getting this wrong sent every plain char literal
            # down the lifetime branch, where the quote is skipped and the
            # brace inside it survives -- latent in tree.rs, which only has
            # the b'{' form, and caught by the lifetime fixture arm.
            q = i if c == "'" else i + 1
            if q + 1 < n and src[q + 1] == "\\":
                j = q + 2
                while j < n and src[j] != "'":
                    j += 1
                j = min(j + 1, n)
            elif q + 2 < n and src[q + 2] == "'":
                j = q + 3
            else:
                # A LIFETIME: 'a, 'static, '_. Not a literal; skip the quote
                # only, so the rest of the line is still read.
                i = q + 1
                continue
            blank(i, j)
            i = j
            continue
        i += 1
    return "".join(out)


def non_test_lines(text):
    """Yield (lineno, line) outside any #[cfg(test)] module."""
    # THE WALK READS THE STRIPPED TEXT and the findings read the original, so
    # a brace inside a literal cannot move the walk and nothing the audit
    # looks for is hidden by the stripping.
    lines = strip_literals(text).split("\n")
    orig = text.split("\n")
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
        out.append((i + 1, orig[i] if i < len(orig) else line))
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
