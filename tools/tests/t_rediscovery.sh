#!/bin/sh
# Tier 3: the rediscovery gate's own honesty (issue 45).
#
# The gate publishes "N of 108 re-found". The only way that figure goes wrong
# is by being padded, so every arm here drives a way of padding it and asserts
# the refusal. What is NOT armed is the figure itself -- it is a measurement,
# and a test asserting it equals 1 would have to be edited every time the
# project got better, which is the shape of a test that trains people to edit
# expectations.
set -u
ROOT=${REDIS_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
pass=0; fail=0
ok() { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no() { fail=$((fail+1)); printf '  FAIL %s -- %s\n' "$1" "${2:-}"; }
judge() { if [ "$2" = 0 ]; then ok "$1"; else no "$1" "$3"; fi; }
fin() { printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail)); [ "$fail" = 0 ] || exit 1; exit 0; }

TOOL="$ROOT/tools/rediscovery.py"
[ -r "$TOOL" ] || { no "tools/rediscovery.py is missing" ""; fin; }
TMP=$(mktemp -d) || { no "mktemp failed" ""; fin; }
trap 'rm -rf "$TMP"' EXIT

# A WORLD PER ARM, built from the real corpus so the arms measure the gate and
# not a toy.
world() {
    d="$TMP/$1"; mkdir -p "$d"
    cp "$ROOT/tests/corpus/build.toml" "$ROOT/tests/corpus/mcp.toml" "$d/"
    cp "$ROOT/tests/corpus/rediscovery.toml" "$d/"
    printf '%s' "$d"
}
rc_of() { ( cd "$ROOT" && python3 "$TOOL" --corpus "$1" >"$TMP/out" 2>&1 ); echo $?; }
# EVERY PERTURBATION ASSERTS IT TOOK. Three arms below went green-then-red when
# the corpus was rewritten under them (2026-11-19): their sed anchors stopped
# matching, each edited NOTHING, and the gate then passed a corpus with no
# defect in it -- rc 0 read as "the refusal did not fire". A world that cannot
# be built is NO RESULT, not a pass, so perturb() exits nonzero and the arm
# says which happened.
perturb() { python3 - "$1" || { no "world $2 could NOT be built -- the perturbation matched nothing, so there was no defect to refuse" ""; return 1; }; }

got=$(rc_of "$ROOT/tests/corpus")
[ "$got" = 0 ]
judge "the committed corpus and its verdicts pass the gate" $? "rc $got"

# -- rc 3: an incident with no verdict at all
d=$(world missing)
python3 - "$d" <<'PY'
import sys, re, pathlib
p = pathlib.Path(sys.argv[1], "rediscovery.toml")
t = p.read_text()
# drop the LAST verdict block
i = t.rstrip().rfind("[[rediscovery]]")
p.write_text(t[:i])
PY
got=$(rc_of "$d")
[ "$got" = 3 ] && /usr/bin/grep -q "carry no verdict" "$TMP/out"
judge "an incident with no verdict refuses: a matrix over a subset reads like a matrix" $? "rc $got"

# -- rc 3: a verdict for an id the corpus does not have
d=$(world phantom)
printf '\n[[rediscovery]]\nid = "build-999"\nverdict = "re-found"\nby = "tools/rediscovery.py"\nfixture = "tools/rediscovery.py"\n' >> "$d/rediscovery.toml"
got=$(rc_of "$d")
[ "$got" = 3 ] && /usr/bin/grep -q "not an incident in" "$TMP/out"
judge "a verdict for an incident that does not exist refuses rather than inflating the count" $? "rc $got"

# -- rc 3: the same id twice
d=$(world twice)
printf '\n[[rediscovery]]\nid = "mcp-026"\nverdict = "unassessed"\nwhy = "a second opinion"\n' >> "$d/rediscovery.toml"
got=$(rc_of "$d")
[ "$got" = 3 ] && /usr/bin/grep -q "two verdicts" "$TMP/out"
judge "two verdicts for one incident refuse: the later would win silently" $? "rc $got"

# -- rc 4: THE PADDING ARM. A re-found verdict citing a check that is not there.
d=$(world dangling)
perturb "$d" dangling <<'PY' || { fin; }
import sys, pathlib, re
p = pathlib.Path(sys.argv[1], "rediscovery.toml")
t = p.read_text()
t2, n = re.subn(r'^by = "[^"]+"$', 'by = "tools/tests/does-not-exist.sh"',
                t, count=1, flags=re.M)
assert n == 1, "no `by` line to point at a missing file"
p.write_text(t2)
PY
got=$(rc_of "$d")
[ "$got" = 4 ] && /usr/bin/grep -q "does not exist" "$TMP/out"
judge "a re-found verdict citing a check that does not exist refuses: coverage must point at something" $? "rc $got"

# -- rc 4: re-found with no citation at all, which is the bare assertion
d=$(world bare)
perturb "$d" bare <<'PY' || { fin; }
import sys, pathlib, re
p = pathlib.Path(sys.argv[1], "rediscovery.toml")
t = p.read_text()
t2, n = re.subn(r'^by = "[^"]+"\n', "", t, count=1, flags=re.M)
assert n == 1, "no `by` line to remove"
p.write_text(t2)
PY
got=$(rc_of "$d")
[ "$got" = 4 ] && /usr/bin/grep -q "names no" "$TMP/out"
judge "a re-found verdict naming no check at all refuses" $? "rc $got"

# -- rc 5: a gap with no reason
d=$(world unexplained)
perturb "$d" unexplained <<'PY' || { fin; }
import sys, pathlib, re
p = pathlib.Path(sys.argv[1], "rediscovery.toml")
t = p.read_text()
t2, n = re.subn(r'^why = "[^"]*"\n', "", t, count=1, flags=re.M)
assert n == 1, "no `why` line to remove"
p.write_text(t2)
PY
got=$(rc_of "$d")
[ "$got" = 5 ] && /usr/bin/grep -q "no \`why\`" "$TMP/out"
judge "an unassessed incident with no reason refuses: an unexplained gap looks like one nobody looked at" $? "rc $got"

# -- rc 6: a verdict outside the three
d=$(world invented)
sed 's|verdict = "unassessed"|verdict = "probably fine"|' "$d/rediscovery.toml" > "$d/r2" && mv "$d/r2" "$d/rediscovery.toml"
got=$(rc_of "$d")
[ "$got" = 6 ]
judge "a verdict value outside the three is refused rather than counted as something" $? "rc $got"

# -- THE MATRIX'S TOTAL IS COUNTED, NOT TYPED. The distinguishing case is a
#    corpus whose count is NOT the committed one: a literal 108 agrees with the
#    real corpus by coincidence, and a mutant hardcoding it SURVIVED an arm
#    that only ever looked at the real one.
d=$(world shortened)
python3 - "$d" <<'PY'
import sys, pathlib, tomllib
d = pathlib.Path(sys.argv[1])
# drop ONE incident and its verdict, so the gate still passes over a corpus of
# a different size
mcp = (d / "mcp.toml").read_text()
i = mcp.rstrip().rfind("[[incident]]")
dropped = tomllib.loads(mcp[i:])["incident"][0]["id"]
(d / "mcp.toml").write_text(mcp[:i])
r = (d / "rediscovery.toml").read_text()
j = r.find(f'id = "{dropped}"')
start = r.rfind("[[rediscovery]]", 0, j)
nxt = r.find("[[rediscovery]]", j)
(d / "rediscovery.toml").write_text(r[:start] + (r[nxt:] if nxt > 0 else ""))
print(dropped)
PY
short=$(cd "$ROOT" && python3 "$TOOL" --corpus "$d" --matrix 2>&1)
full=$(cd "$ROOT" && python3 "$TOOL" --matrix 2>&1)
printf '%s' "$short" | /usr/bin/grep -q "matrix, 107 incidents" \
  && printf '%s' "$full" | /usr/bin/grep -q "matrix, 108 incidents"
judge "the matrix's total is counted from the corpus it was given, not typed into the tool" $? \
  "short said $(printf '%s' "$short" | /usr/bin/grep -oE 'matrix, [0-9]+' | head -1)"

# -- rc 4: A CITATION TO THE DATA IS NOT A CITATION TO A CHECK. rook, F4: the
#    one re-found verdict cited tests/vectors/vectors.toml as `by`, and that
#    file is the fixture the check READS. Existence could not tell them apart,
#    so the arm's distinguishing case is a path that EXISTS and is still not a
#    check.
d=$(world data-as-check)
perturb "$d" data-as-check <<'PY' || { fin; }
import sys, pathlib, re
p = pathlib.Path(sys.argv[1], "rediscovery.toml")
t = p.read_text()
t2, n = re.subn(r'^by = "[^"]+"$', 'by = "tests/vectors/vectors.toml"',
                t, count=1, flags=re.M)
assert n == 1, "no `by` line to repoint at data"
p.write_text(t2)
PY
got=$(rc_of "$d")
[ "$got" = 4 ] && /usr/bin/grep -q "which is not a check" "$TMP/out"
judge "a re-found verdict citing a file that EXISTS but is data, not a check, refuses -- existence never meant the citation meant anything" $? "rc $got"

# -- rc 7: A REASON REPEATED IS ONE REASON. rook, wm-review/3: 107 of 108
#    verdicts carried the same pasted `why` and every existence check passed.
d=$(world pasted)
perturb "$d" pasted <<'PY' || { fin; }
import sys, pathlib, re
p = pathlib.Path(sys.argv[1], "rediscovery.toml")
t = p.read_text()
t2, n = re.subn(r'^why = "[^"]*"$', 'why = "not looked at"', t, flags=re.M)
assert n > 5, f"only {n} why lines to paste over"
p.write_text(t2)
PY
got=$(rc_of "$d")
[ "$got" = 7 ] && /usr/bin/grep -q "is one reason, not" "$TMP/out"
judge "more than PASTE_LIMIT verdicts sharing one reason refuses: a pasted reason says nobody looked at the rest" $? "rc $got"

# -- AND THE SPLIT IS COMPUTED FROM `by`, NOT ASSERTED. rook, F3: battery.sh
#    and ci.yml both glob tools/tests/t_*.sh, so a re-find performed only by a
#    deliberately-run arm is inside NEITHER green. The distinguishing case is
#    the SAME verdict classified both ways, because a split that always said
#    "outside" would agree with the real corpus by coincidence.
out_now=$(cd "$ROOT" && python3 "$TOOL" --matrix 2>&1)
printf '%s' "$out_now" | /usr/bin/grep -q "0 is inside the CI-green set" \
  && printf '%s' "$out_now" | /usr/bin/grep -q "THE NUMBER THE BATTERY'S GREEN COVERS IS 0"
judge "a re-find by an arm outside the t_*.sh glob is reported OUTSIDE the green, and the covered number is printed separately" $? \
  "$(printf '%s' "$out_now" | /usr/bin/grep -c 'inside the CI-green') matching lines"

d=$(world in-green)
perturb "$d" in-green <<'PY' || { fin; }
import sys, pathlib, re
p = pathlib.Path(sys.argv[1], "rediscovery.toml")
t = p.read_text()
# point the re-found verdict at an arm the battery's glob DOES run
t2, n = re.subn(r'^by = "[^"]+"$', 'by = "tools/tests/t_rediscovery.sh"',
                t, count=1, flags=re.M)
assert n == 1, "no `by` line to move inside the green"
p.write_text(t2)
PY
moved=$(cd "$ROOT" && python3 "$TOOL" --corpus "$d" --matrix 2>&1)
printf '%s' "$moved" | /usr/bin/grep -q "1 is inside the CI-green set" \
  && ! printf '%s' "$moved" | /usr/bin/grep -q "THE NUMBER THE BATTERY'S GREEN COVERS"
judge "and the SAME verdict cited to a t_*.sh arm is reported INSIDE it, with no separate-number warning: the split reads \`by\`, it is not a constant" $? \
  "$(printf '%s' "$moved" | /usr/bin/grep -oE '[0-9]+ is inside the CI-green set' | head -1)"

fin
