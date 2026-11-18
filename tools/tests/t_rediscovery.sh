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
sed 's|by = "tests/vectors/vectors.toml"|by = "tests/vectors/does-not-exist.toml"|' \
    "$d/rediscovery.toml" > "$d/r2" && mv "$d/r2" "$d/rediscovery.toml"
got=$(rc_of "$d")
[ "$got" = 4 ] && /usr/bin/grep -q "does not exist" "$TMP/out"
judge "a re-found verdict citing a check that does not exist refuses: coverage must point at something" $? "rc $got"

# -- rc 4: re-found with no citation at all, which is the bare assertion
d=$(world bare)
python3 - "$d" <<'PY'
import sys, pathlib
p = pathlib.Path(sys.argv[1], "rediscovery.toml")
t = p.read_text()
t = t.replace('by = "tests/vectors/vectors.toml"\n', "")
p.write_text(t)
PY
got=$(rc_of "$d")
[ "$got" = 4 ] && /usr/bin/grep -q "names no" "$TMP/out"
judge "a re-found verdict naming no check at all refuses" $? "rc $got"

# -- rc 5: a gap with no reason
d=$(world unexplained)
python3 - "$d" <<'PY'
import sys, pathlib
p = pathlib.Path(sys.argv[1], "rediscovery.toml")
t = p.read_text()
t = t.replace('why = "no fixture reconstructs this defect yet, and no check of ours names it"\n', "", 1)
p.write_text(t)
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

fin
