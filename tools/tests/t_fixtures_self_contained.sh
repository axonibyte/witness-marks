#!/bin/sh
# Tier 3, source-as-data: our fixtures and vector models reach NO library.
#
# WHY. Eight OpenSCAD libraries are installed on this box -- BOSL2 2.0.716,
# threads, gear.scad and five others -- and NONE is pinned anywhere. That
# currently costs nothing, because `tests/fixtures/can_cutter/can_cutter.scad`
# includes nothing external and no vector model reaches outside the fixture
# tree. Every figure this project has recorded is therefore independent of
# every library version.
#
# THAT SELF-CONTAINMENT IS AN INVARIANT WE RELY ON AND NEVER ASSERTED. One
# fixture gaining `include <BOSL2/std.scad>` would put every figure recorded
# from it on an unpinned dependency, silently -- the geometry would change with
# a library upgrade nobody recorded, and the vectors would start disagreeing
# with the provider for a reason neither side could name. See issue 58.
#
# A library is not forbidden forever. It is forbidden until it is PINNED, and
# this arm is what forces that conversation instead of letting the dependency
# arrive unannounced.
set -u
ROOT=${SELFC_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
pass=0; fail=0
ok() { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no() { fail=$((fail+1)); printf '  FAIL %s\n' "$1"; }
fin() { printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail)); [ "$fail" = 0 ] || exit 1; exit 0; }

scan() {
    # Prints one "file:line:text" per include/use that leaves the fixture tree.
    python3 - "$1" "$2" <<'PY'
import re, sys
from pathlib import Path

root, vectors = Path(sys.argv[1]), Path(sys.argv[2])
# `include <x>` / `use <x>`; the path may be absolute, relative or a library name.
REF = re.compile(r"^\s*(include|use)\s*<([^>]+)>", re.M)
fixtures = root / "tests" / "fixtures"
scanned = 0
out = []

def judge(where, text):
    for m in REF.finditer(text):
        verb, ref = m.group(1), m.group(2).strip()
        # A reference is self-contained when it resolves INSIDE the fixture
        # tree: a sibling by relative path, or an absolute path under it.
        p = Path(ref)
        inside = False
        if p.is_absolute():
            try:
                p.resolve().relative_to(fixtures.resolve())
                inside = True
            except ValueError:
                inside = False
        else:
            # A bare name with no separator is a LIBRARY lookup on
            # OPENSCADPATH, never a sibling -- "BOSL2/std.scad" included.
            inside = "/" not in ref and (fixtures / ref).exists()
            if not inside:
                for cand in fixtures.rglob(ref):
                    inside = True
                    break
        if not inside:
            out.append(f"{where}: {verb} <{ref}>")

for f in sorted(fixtures.rglob("*.scad")):
    scanned += 1
    judge(str(f.relative_to(root)), f.read_text(encoding="utf-8", errors="replace"))

if vectors.is_file():
    import tomllib
    doc = tomllib.load(open(vectors, "rb"))
    for kind in ("vector", "relation"):
        for entry in doc.get(kind, []):
            for key in ("model", "base", "transformed"):
                if key in entry:
                    scanned += 1
                    judge(f"vectors.toml[{entry.get('id','?')}].{key}", entry[key])
            for i, part in enumerate(entry.get("sum_of", []) or []):
                scanned += 1
                judge(f"vectors.toml[{entry.get('id','?')}].sum_of[{i}]", part)

print(f"SCANNED {scanned}")
for line in out:
    print(f"ESCAPES {line}")
PY
}

OUT=$(scan "$ROOT" "$ROOT/tests/vectors/vectors.toml" 2>&1) || {
    no "the scan did not run: $OUT"; fin
}
n=$(printf '%s\n' "$OUT" | /usr/bin/sed -n 's/^SCANNED //p')

# REFUSE A BLIND SCAN. Zero models scanned means the tree or the pattern moved,
# and "nothing escapes" would then be a green with nothing behind it.
if [ "${n:-0}" -lt 5 ]; then
    no "only ${n:-0} model(s) scanned; the fixture tree or the pattern moved, so this check is blind"
    fin
fi
ok "$n fixture file(s) and vector model(s) scanned"

esc=$(printf '%s\n' "$OUT" | /usr/bin/grep '^ESCAPES ' || true)
if [ -n "$esc" ]; then
    no "a fixture or vector model reaches outside tests/fixtures/, so its geometry now depends on a library version nobody pinned (issue 58):"
    printf '%s\n' "$esc" | /usr/bin/sed 's/^ESCAPES /         /'
else
    ok "no fixture or vector model reaches a library"
fi

# THE WHOLE ARM IS DRIVEN AGAINST A DIRTY TREE AND A BLIND ONE.
#
# Two mutants survived without this: one disabling the escapes branch and one
# disabling the blind-scan refusal. Both are invisible when the real tree is
# clean and large, because neither branch ever executes -- the scan-level
# checks below call `scan` directly and never reach them. A check that cannot
# execute is not a check.
#
# $SELFC_ROOT being set means THIS IS THE INNER RUN, which is the recursion
# guard: without it the fixture run re-invokes itself forever.
if [ -z "${SELFC_ROOT:-}" ]; then
    SELF=$0
    DIRTY=$(mktemp -d) || { no "no dirty-tree dir"; fin; }
    mkdir -p "$DIRTY/tests/fixtures" "$DIRTY/tests/vectors"
    for f in a b c d e; do printf 'cube(1);\n' > "$DIRTY/tests/fixtures/$f.scad"; done
    printf 'include <BOSL2/std.scad>;\ncube(1);\n' > "$DIRTY/tests/fixtures/reaches.scad"
    out=$(SELFC_ROOT="$DIRTY" sh "$SELF" 2>&1); rc=$?
    if [ "$rc" -ne 0 ] && printf '%s\n' "$out" | /usr/bin/grep -q 'reaches outside tests/fixtures/'; then
        ok "the arm FAILS on a tree that reaches a library, and names the escape"
    else
        no "a tree including BOSL2 did not fail the arm (rc=$rc); the escapes branch cannot execute"
    fi
    rm -rf "$DIRTY"

    SMALL=$(mktemp -d) || { no "no small-tree dir"; fin; }
    mkdir -p "$SMALL/tests/fixtures" "$SMALL/tests/vectors"
    printf 'cube(1);\n' > "$SMALL/tests/fixtures/only.scad"
    out=$(SELFC_ROOT="$SMALL" sh "$SELF" 2>&1); rc=$?
    if [ "$rc" -ne 0 ] && printf '%s\n' "$out" | /usr/bin/grep -q 'this check is blind'; then
        ok "a tree too small to be the real one is refused as blind, not passed"
    else
        no "a one-file tree did not trip the blind-scan refusal (rc=$rc)"
    fi
    rm -rf "$SMALL"
fi

# AND THE SCAN CAN FAIL, driven against a fixture tree that does reach out.
# A check that has never caught anything is indistinguishable from one that
# cannot.
FIX=$(mktemp -d) || { no "no fixture dir"; fin; }
trap 'rm -rf "$FIX"' EXIT INT TERM HUP
mkdir -p "$FIX/tests/fixtures/sub" "$FIX/tests/vectors"
printf 'include <sibling.scad>;\ncube(1);\n' > "$FIX/tests/fixtures/ok.scad"
printf 'cube(2);\n' > "$FIX/tests/fixtures/sibling.scad"
printf 'cube(3);\n' > "$FIX/tests/fixtures/sub/deep.scad"
printf 'cube(4);\n' > "$FIX/tests/fixtures/a.scad"
printf 'cube(5);\n' > "$FIX/tests/fixtures/b.scad"
CLEAN=$(scan "$FIX" "$FIX/tests/vectors/none.toml" 2>&1)
if printf '%s\n' "$CLEAN" | /usr/bin/grep -q '^ESCAPES '; then
    no "a self-contained fixture tree was reported as escaping: $(printf '%s\n' "$CLEAN" | /usr/bin/grep '^ESCAPES ' | head -1)"
else
    ok "a sibling include is not reported as reaching outside"
fi

printf 'include <BOSL2/std.scad>;\ncube(1);\n' > "$FIX/tests/fixtures/bad.scad"
DIRTY=$(scan "$FIX" "$FIX/tests/vectors/none.toml" 2>&1)
if printf '%s\n' "$DIRTY" | /usr/bin/grep -q 'ESCAPES.*BOSL2/std.scad'; then
    ok "a fixture including BOSL2 is caught"
else
    no "a fixture including BOSL2 was NOT caught, so this arm cannot fail"
fi

# A bare library name with no separator is a library lookup too, and is the
# easier one to miss.
rm -f "$FIX/tests/fixtures/bad.scad"
printf 'use <threads.scad>;\ncube(1);\n' > "$FIX/tests/fixtures/bare.scad"
BARE=$(scan "$FIX" "$FIX/tests/vectors/none.toml" 2>&1)
if printf '%s\n' "$BARE" | /usr/bin/grep -q 'ESCAPES.*threads.scad'; then
    ok "a bare library name with no separator is caught too"
else
    no "a bare 'use <threads.scad>' was not caught: it resolves on OPENSCADPATH, not as a sibling"
fi
fin
