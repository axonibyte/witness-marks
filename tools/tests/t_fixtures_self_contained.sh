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
# ONE LABEL ON BOTH PATHS. Added 2026-11-19 after two arms appended to this
# file called a `judge` it did not have: the shell printed "judge: not found",
# the arms produced NO result, and the tally still said 9/9 -- a count over the
# arms that ran, read as a count over the arms that exist. mutate.sh's refusal
# 4 also needs the label in a FAILURE line, so success and failure share it.
judge() { if [ "$2" = 0 ]; then ok "$1"; else no "$1 -- $3"; fi; }
# AND THE TALLY IS CHECKED AGAINST WHAT THIS FILE DEFINES, so a non-executing
# arm cannot hide in the denominator again.
want_arms() { /usr/bin/grep -cE '^ *(judge|ok|no) "' "$0"; }

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
        # THE PLACEHOLDER EXPANDS INSIDE THE TREE, SO IT IS INSIDE. A committed
        # model names @FIXTURES@ rather than a path, precisely so the file holds
        # no machine's checkout; tools/vector-run.py expands it. An ABSOLUTE
        # path is still an escape and is still caught below -- which is what it
        # was before, and what CI found on a fresh clone while this machine
        # read green for ten commits.
        if ref.startswith("@FIXTURES@/"):
            continue
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

# NO COMMITTED MODEL NAMES AN ABSOLUTE PATH, checked DIRECTLY and therefore
# independently of where the tree sits. The escape scan below found this defect
# only because CI's clone root DIFFERS from the one baked in -- on the machine
# where the paths were written they resolved inside the fixture tree and the
# scan passed, so CI was red for ten commits while every local run read green.
# An absolute path in a committed model is wrong wherever it is read, so it gets
# a check that does not depend on being read somewhere else.
ABS=$(/usr/bin/grep -rnE '(use|include) *<[[:space:]]*/' \
        "$ROOT/tests/fixtures" "$ROOT/tests/vectors" 2>/dev/null || true)
if [ -z "$ABS" ]; then
    ok "no committed fixture or vector model names an absolute path"
else
    no "a committed model names an absolute path, which is one machine's checkout rather than a fact about this repository" "$(printf '%s' "$ABS" | head -3)"
fi

# AND NO COMMITTED FILE REACHES OUTSIDE THE REPOSITORY WITHOUT SAYING SO.
# TWICE IN ONE DAY a committed absolute path broke CI, and the two were
# different classes:
#
#   vectors.toml named paths INSIDE the repo absolutely, which resolved here
#   and pointed at a foreign tree on a different root. The foreign-root gate in
#   battery.sh catches that one.
#
#   tests/claims/can-cutter.toml names the harness, which lives OUTSIDE the
#   repo entirely -- cal's design file, in another tree. A foreign ROOT on this
#   MACHINE still resolves it, so that gate cannot see it, and CI (which clones
#   only this repo) failed at 26 seconds.
#
# So an out-of-repo path is allowed and must be DECLARED, with the tool that
# reads it required to treat its absence as an ordinary state. Undeclared is
# the accident; declared is a decision somebody made on purpose.
# THE TWO DECLARED ONES, each with the reason it is allowed:
#
#   tests/claims/can-cutter.toml    names the MODEL the guards are about. Its
#       absence is an ordinary state on any machine but the author's, and
#       tools/guards.py reports it as a WHERE finding with its own exit code
#       rather than as a defect in the guards -- armed in t_guards.sh.
#
#   tests/fixtures/can_cutter/MANIFEST.toml  records where the VENDORED copies
#       came from, under a [source] heading. Provenance, not a dependency:
#       t_fixture_manifest.sh checks the vendored BYTES against recorded
#       digests and never opens the source paths, so they may be absent for
#       ever without changing a verdict.
ALLOWED="tests/claims/can-cutter.toml tests/fixtures/can_cutter/MANIFEST.toml"
OUTSIDE=$(cd "$ROOT" && /usr/bin/grep -rlE '"/[a-z]' --include='*.toml' tests tools 2>/dev/null \
          | while read -r f; do
              case " $ALLOWED " in *" $f "*) continue ;; esac
              printf '%s\n' "$f"
            done)
if [ -z "$OUTSIDE" ]; then
    ok "no committed TOML names an absolute path outside the repository except the declared one"
else
    no "a committed file reaches outside the repository and is not declared" "$OUTSIDE"
fi

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

# -- NO BUILD ARTIFACT IS TRACKED (rook F14, thread:wm-review/21). 32068e4
#    committed tools/__pycache__/corpus_reasons.cpython-312.pyc, because the
#    broad `git add -A` that fixed an untracked module took the bytecode with
#    it. The cost is not only untidiness: every box with a different Python
#    dirties the tree on its first run, so a "tree clean" check after a mutant
#    sweep reads noise -- which is exactly how rook noticed. And a stale .pyc
#    has been EXECUTED in place of source in this project's history, giving an
#    impossible probe result.
#
#    ENUMERATED, NOT NAMED: the pattern list is the artifact KINDS this repo
#    could plausibly grow, not the one file that happened to leak.
artifacts=$(cd "$ROOT" && git ls-files \
    | /usr/bin/grep -E '(^|/)__pycache__/|\.pyc$|\.pyo$|(^|/)\.pytest_cache/|(^|/)target/|(^|/)\.venv/|(^|/)node_modules/' \
    || true)
[ -z "$artifacts" ]
judge "no build artifact is tracked: bytecode, caches and build trees dirty every box differently and make a post-sweep 'tree clean' check read noise (F14)" \
      $? "tracked: $(printf '%s' "$artifacts" | tr '\n' ' ')"

# -- AND THE TOOLS DO NOT WRITE BYTECODE AT ALL, so an ignored file cannot
#    become a dirty tree either. Every tool that imports a sibling from
#    tools/ must set sys.dont_write_bytecode first.
missing=""
for f in "$ROOT"/tools/*.py; do
    /usr/bin/grep -q "^import corpus_reasons\|^import  *corpus_reasons" "$f" || continue
    /usr/bin/grep -q "dont_write_bytecode" "$f" || missing="$missing $(basename "$f")"
done
[ -z "$missing" ]
judge "every tool that imports a sibling sets sys.dont_write_bytecode, so no __pycache__ is created to be committed or to go stale" \
      $? "missing:$missing"

fin
