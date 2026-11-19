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

# THE COMMITTED CORPUS REFUSES, AND THAT IS THE CORRECT ANSWER. rook's F12
# (wm-review/19): this arm used to assert rc 0 here, so the defect was a
# COMMITTED EXPECTATION -- a tool printing "test 6 still holds of this corpus:
# 0 of 108" from zero assessments, with an arm guarding the claim. Nothing in
# this corpus records labor, measured by scanning all 108, so the honest exit
# is the refusal and the honest line says UNANSWERED.
got=$(rc_of "$ROOT/tests/corpus")
[ "$got" = 6 ] && /usr/bin/grep -q "test 6 is UNANSWERED" "$TMP/out" \
  && ! /usr/bin/grep -q "still holds" "$TMP/out"
judge "the committed corpus REFUSES rather than reporting that test 6 holds: 0 assessed is an absence of looking, not an absence of scope cost (F12)" \
      $? "rc $got: $(/usr/bin/grep -o 'test 6[^.]*' "$TMP/out" | head -1)"

# -- THE REFUSAL. A fraction over nothing assessed.
got=$(rc_of "$ROOT/tests/corpus" --report)
[ "$got" = 6 ] && /usr/bin/grep -q "neither the fraction nor test 6" "$TMP/out"
judge "a scope fraction over zero assessments is REFUSED, not printed as 0% -- and the refusal now fires WITHOUT --report too, because the claim it guards was printed either way (F12)" \
      $? "rc $got"

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

# -- AND THE POSITIVE CASE THE OLD ARM WAS STANDING IN FOR: with something
#    ASSESSED and no scope among it, the claim does hold, and the line names
#    the assessed count rather than the corpus size. This is what the deleted
#    arm was trying to cover; it covered the refusal-shaped lie instead.
d=$(world assessed-no-scope)
python3 - "$d" <<'PY' || { no "world assessed-no-scope could NOT be built"; }
import sys, pathlib
p = pathlib.Path(sys.argv[1], "cost.toml")
t = p.read_text()
n = t.count('verdict = "unknown"')
assert n > 3, f"only {n} unknown verdicts to assess"
t = t.replace('verdict = "unknown"', 'verdict = "measurement"', 3)
p.write_text(t)
PY
got=$(rc_of "$d")
[ "$got" = 0 ] && /usr/bin/grep -q "still holds of this corpus: 0 of 3 ASSESSED" "$TMP/out"
judge "with three assessed and none of them scope, test 6 DOES hold, and the line names the ASSESSED count not the corpus size" \
      $? "rc $got: $(/usr/bin/grep -o 'test 6[^.]*' "$TMP/out" | head -1)"

# -- F13: THE PASTE LIMIT'S SIBLING. rediscovery.py has refused a reason
#    repeated past the limit since F4; this tool, written the same week and
#    reading a corpus the same way, had no such check and all 108 of its
#    reasons were ONE sentence.
d=$(world pasted)
python3 - "$d" <<'PY' || { no "world pasted could NOT be built"; }
import sys, pathlib, re
p = pathlib.Path(sys.argv[1], "cost.toml")
t = p.read_text()
n = len(re.findall(r'^why = """$', t, re.M))
assert n > 5, f"only {n} why blocks to paste over"
t = re.sub(r'why = """\n.*?"""', 'why = "all the same"', t, flags=re.S)
p.write_text(t)
PY
got=$(rc_of "$d")
[ "$got" = 7 ] && /usr/bin/grep -q "is one reason, not" "$TMP/out"
judge "more than the paste limit of cost verdicts sharing one reason REFUSES, the same as rediscovery's (F13)" $? "rc $got"

# -- AND THE CHECK HAS ONE HOME, ENUMERATED RATHER THAN NAMED. F13 existed
#    because the rule lived in one file and its sibling was written without
#    it. So every tool that reads a corpus `why` must import the shared
#    module; a third one cannot quietly skip it.
users=""
missing=""
for f in "$ROOT"/tools/*.py; do
    /usr/bin/grep -q 'get("why")\|\["why"\]' "$f" || continue
    users="$users $(basename "$f")"
    /usr/bin/grep -q "corpus_reasons" "$f" || missing="$missing $(basename "$f")"
done
[ -n "$users" ] && [ -z "$missing" ]
judge "every tool that reads a corpus \`why\` uses the shared paste check:$users (a rule in one file and not its sibling is how F13 happened)" \
      $? "users:$users missing:$missing"

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

# ANCHORED ON THE FORM, NOT THE SENTENCE. This perturbation named the exact
# pasted `why` it expected to find, so rewriting the corpus for F13 left it
# matching nothing -- the world was built with no defect in it and the gate
# passed. A perturbation asserts it took.
d=$(world noreason)
python3 - "$d" <<'PY' || { no "world noreason could NOT be built: the perturbation matched nothing"; }
import sys, pathlib, re
p = pathlib.Path(sys.argv[1], "cost.toml")
t = p.read_text()
t2, n = re.subn(r'why = """\n.*?"""\n', "", t, count=1, flags=re.S)
assert n == 1, "no `why` block to remove"
p.write_text(t2)
PY
got=$(rc_of "$d")
[ "$got" = 5 ] && /usr/bin/grep -q "indistinguishable from one nobody made" "$TMP/out"
judge "a verdict with no reason refuses: an unexplained judgment is one nobody made" $? "rc $got"

d=$(world invented)
sed 's|verdict = "unknown"|verdict = "probably scope"|' "$d/cost.toml" > "$d/c2" && mv "$d/c2" "$d/cost.toml"
got=$(rc_of "$d")
[ "$got" = 4 ]
judge "a verdict outside the three is refused rather than counted as something" $? "rc $got"

fin
