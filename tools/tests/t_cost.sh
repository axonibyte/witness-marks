#!/bin/sh
# Tier 3: where the time went, and the refusal that keeps it honest
# (issue 61, spec §10 test 6).
#
# THE ARM THAT MATTERS is the one asserting a fraction is REFUSED over no data.
# "0% of lost time is scope" drawn from zero assessments would flatter the
# project in the exact direction test 6 is worried about -- its concern is that
# scope costs MORE than measurement, so a spurious 0% is not a harmless default,
# it is the answer the project would like to be true.
#
# AND THE MIRROR: with data present the fraction must actually compute, or the
# refusal above is just a tool that never answers.
set -u
ROOT=${COST_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
pass=0; fail=0
ok() { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no() { fail=$((fail+1)); printf '  FAIL %s -- %s\n' "$1" "${2:-}"; }
judge() { if [ "$2" = 0 ]; then ok "$1"; else no "$1" "$3"; fi; }
fin() { printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail)); [ "$fail" = 0 ] || exit 1; exit 0; }

TOOL="$ROOT/tools/cost.py"
[ -r "$TOOL" ] || { no "tools/cost.py is missing" ""; fin; }
TMP=$(mktemp -d) || { no "mktemp failed" ""; fin; }
trap 'rm -rf "$TMP"' EXIT

world() {
    d="$TMP/$1"; mkdir -p "$d"
    cp "$ROOT/tests/corpus/build.toml" "$ROOT/tests/corpus/mcp.toml" \
       "$ROOT/tests/corpus/cost.toml" "$d/"
    printf '%s' "$d"
}
rc_of() { ( cd "$ROOT" && timeout 120 python3 "$TOOL" --corpus "$1" ${2:-} >"$TMP/out" 2>&1 ); echo $?; }

got=$(rc_of "$ROOT/tests/corpus")
[ "$got" = 0 ]
judge "every incident in the committed corpus carries an honest cost verdict" $? "rc $got"

# -- THE REFUSAL. A fraction over nothing assessed.
got=$(rc_of "$ROOT/tests/corpus" --report)
[ "$got" = 6 ] && /usr/bin/grep -q "NOT COMPUTABLE" "$TMP/out"
judge "a scope fraction over zero assessments is REFUSED, not printed as 0%" $? "rc $got"

# -- AND THE MIRROR: with data, it computes. Two assessed, one of them scope.
d=$(world withdata)
python3 - "$d" <<'PY'
import sys, pathlib, re
p = pathlib.Path(sys.argv[1], "cost.toml")
t = p.read_text()
# assess the first two entries: one scope, one measurement
t = t.replace('verdict = "unknown"', 'verdict = "scope"', 1)
t = t.replace('verdict = "unknown"', 'verdict = "measurement"', 1)
p.write_text(t)
PY
got=$(rc_of "$d" --report)
[ "$got" = 0 ] && /usr/bin/grep -q "50% scope" "$TMP/out"
judge "with two assessed, one scope, the fraction computes and says 50%" $? "rc $got: $(/usr/bin/grep -o '[0-9]*% scope' "$TMP/out" | head -1)"

# -- AND TEST 6's CLAIM IS CHECKED, NOT RESTATED.
/usr/bin/grep -q "NO LONGER HOLDS" "$TMP/out"
judge "one scope incident makes the tool say spec §10 test 6 no longer holds" $? "$(/usr/bin/grep -o 'test 6[^.]*' "$TMP/out" | head -1)"

got=$(rc_of "$ROOT/tests/corpus")
/usr/bin/grep -q "still holds of this corpus: 0 of 108" "$TMP/out"
judge "and with none, it says the claim still holds, naming the count it checked" $? ""

# -- REFUSALS
d=$(world missing)
python3 -c "
import pathlib,sys
p=pathlib.Path(sys.argv[1],'cost.toml'); t=p.read_text()
i=t.rstrip().rfind('[[cost]]'); p.write_text(t[:i])" "$d"
got=$(rc_of "$d")
[ "$got" = 3 ] && /usr/bin/grep -q "carry no verdict" "$TMP/out"
judge "an incident with no cost verdict refuses: a fraction over a subset reads like one over the corpus" $? "rc $got"

d=$(world phantom)
printf '\n[[cost]]\nid = "build-999"\nverdict = "scope"\nwhy = "invented"\n' >> "$d/cost.toml"
got=$(rc_of "$d")
[ "$got" = 3 ]
judge "a verdict for an incident that does not exist refuses" $? "rc $got"

d=$(world noreason)
python3 -c "
import pathlib,sys
p=pathlib.Path(sys.argv[1],'cost.toml'); t=p.read_text()
p.write_text(t.replace('why = \"the record predates the question: it says how the defect was found, not what finding it cost\"\n','',1))" "$d"
got=$(rc_of "$d")
[ "$got" = 5 ] && /usr/bin/grep -q "indistinguishable from one nobody made" "$TMP/out"
judge "a verdict with no reason refuses: an unexplained judgment is one nobody made" $? "rc $got"

d=$(world invented)
sed 's|verdict = "unknown"|verdict = "probably scope"|' "$d/cost.toml" > "$d/c2" && mv "$d/c2" "$d/cost.toml"
got=$(rc_of "$d")
[ "$got" = 4 ]
judge "a verdict outside the three is refused rather than counted as something" $? "rc $got"

fin
