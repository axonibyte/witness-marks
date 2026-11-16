#!/bin/sh
# Arms for tools/test-count.sh.
#
# THE SUBJECT IS A VARIABLE so a mutant is pointed at, never edited in place.
# EVERY ARM ASSERTS THREE THINGS -- the exit code, what reached stdout, and the
# message -- because an rc is a value a broken tool produces by accident. A
# script that did nothing but `exit 2` would satisfy an rc-only arm for the
# build-failure case while telling the operator nothing.
#
# THE STDOUT ASSERTION IS THE LOAD-BEARING ONE and it is why it is here: the
# whole point of this tool is that a refusal must not look like a count. An arm
# that checked only rc and message would pass on a version that printed "0"
# alongside its refusal, which is the exact collapse the tool replaces.
set -u
SRC=${COUNT_SRC:-$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)/test-count.sh}
FIX=$(CDPATH= cd -- "$(dirname -- "$0")/../../tests/fixtures/cargo-output" && pwd)
pass=0; fail=0
arm() {  # arm <fixture> <want-rc> <want-stdout> <want-msg-substring> <label>
    out=$(sh "$SRC" "$FIX/$1.txt" 2>/dev/null); rc=$?
    err=$(sh "$SRC" "$FIX/$1.txt" 2>&1 >/dev/null)
    ok=yes
    [ "$rc" = "$2" ] || ok=no
    [ "$out" = "$3" ] || ok=no
    case "$4" in "") [ -z "$err" ] || ok=no ;; *) case "$err" in *"$4"*) ;; *) ok=no ;; esac ;; esac
    if [ "$ok" = yes ]; then pass=$((pass+1)); printf '  ok   %-46s rc=%s out=[%s]\n' "$5" "$rc" "$out"
    else fail=$((fail+1)); printf '  FAIL %-46s rc=%s (want %s) out=[%s] (want [%s]) err=%s\n' \
        "$5" "$rc" "$2" "$out" "$3" "$(printf '%s' "$err" | head -1 | cut -c1-50)"; fi
}
#   fixture    rc  stdout  message fragment                          label
arm green      0  11  ""                                  'a real green transcript sums to 11'
arm failed     3  ""  "contains a FAILED suite"           'a red transcript is REFUSED, not summed'
arm noresult   2  ""  "never reported"                    'a build failure is not "zero tests"'
arm empty      4  ""  "nothing behind it"                 'a real zero run is named as a zero run'
arm malformed  5  ""  "did not carry an"                  'an unreadable result line is a parser finding'

arm mixed      3  ""  "contains a FAILED suite"           'ok suites beside a FAILED one are still refused'

# THE REGRESSION, AND IT IS MEASURED RATHER THAN ASSERTED. The old pipeline was
# `grep '^test result: ok'`, which SKIPS the FAILED line and sums the suites
# that passed. On mixed.txt -- three passing unit tests beside one failing
# integration test, a real transcript -- it reports 3. ci.yml echoes
# "tests passed: ${passed:-0}" BEFORE it checks cargo's rc, so that job log
# carried a positive count on a red run; only the rc check two lines later
# stopped it being acted on. A number that is true of a run nobody should
# trust, printed next to its failure, is the shape this tool exists to remove:
# the refusal now comes from the TRANSCRIPT and does not depend on a second
# check happening to be in the right order.
old=$(grep -E '^test result: ok' "$FIX/mixed.txt" | sed -E 's/.* ([0-9]+) passed.*/\1/' | /usr/bin/awk '{s+=$1} END{print s+0}')
sh "$SRC" "$FIX/mixed.txt" >/dev/null 2>&1; new=$?
if [ "$old" -gt 0 ] && [ "$new" = 3 ]; then
    pass=$((pass+1)); printf '  ok   %-46s old=%s new=rc%s\n' 'the old pipeline counts 3 on a RED run; this refuses' "$old" "$new"
else
    fail=$((fail+1)); printf '  FAIL %-46s old=%s new=rc%s\n' 'old pipeline vs the new tool on a red run' "$old" "$new"
fi


# ---------------------------------------------------------------------------
# tools/ci-report-count.sh: the DISPATCH. It exists as a file, and is driven
# here, because the same nine lines living inside ci.yml could only ever be
# exercised by pushing -- and the only report would be the colour of a run.
# Every branch is driven at BOTH cargo exit codes, because the first draft of
# the dispatch was green for a FAILED transcript whenever cargo's rc was 0.
REP=${REPORT_SRC:-$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)/ci-report-count.sh}
rep() {  # rep <fixture> <cargo-rc> <want-rc> <want-stdout-substring> <label>
    out=$(COUNT_SRC=$SRC sh "$REP" "$FIX/$1.txt" "$2" 2>/dev/null); rc=$?
    ok=yes
    [ "$rc" = "$3" ] || ok=no
    case "$out" in *"$4"*) ;; *) ok=no ;; esac
    if [ "$ok" = yes ]; then pass=$((pass+1)); printf '  ok   %-46s rc=%s\n' "$5" "$rc"
    else fail=$((fail+1)); printf '  FAIL %-46s rc=%s (want %s) out=%s\n' "$5" "$rc" "$3" "$(printf '%s' "$out" | head -1 | cut -c1-44)"; fi
}
rep green     0   0 'tests passed: 11'   'green + cargo 0 is the ONLY way to pass'
rep green     101 1 'tests passed: 11'   'a good count does not rescue a nonzero cargo'
rep failed    0   1 'count is withheld'  'a FAILED suite fails even when cargo says 0'
rep mixed     0   1 'count is withheld'  'ok suites beside a FAILED one do not rescue it'
rep noresult  0   1 'BUILD failure'      'no suite reported is named as a build failure'
rep empty     0   1 'nothing behind it'  'a real zero is named as a zero'
rep malformed 0   1 'TOOLING finding'    'an unparsed line is named as tooling, not code'

printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail))
[ "$fail" = 0 ] || exit 1
