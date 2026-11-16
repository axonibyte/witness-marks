#!/bin/sh
# Sum the passing tests in a `cargo test` transcript, and REFUSE rather than
# return a number it cannot stand behind.
#
# WHY THIS IS NOT A PIPELINE IN ci.yml ANY MORE. It was:
#
#   passed=$(grep -E '^test result: ok' out | sed -E 's/.* ([0-9]+) passed.*/\1/' \
#            | paste -sd+ - | bc)
#   test "${passed:-0}" -gt 0 || { echo "zero tests ran: a green with nothing behind it"; exit 1; }
#
# and `${passed:-0}` collapses THREE different states into one sentinel:
#   - bc is not installed on the runner      -> empty -> 0
#   - the transcript has no result lines     -> empty -> 0   (a build failure)
#   - the suites really ran and found 0 tests-> "0"   -> 0
# The message then asserts the third of them. A runner missing bc would have
# been reported to me as "a green with nothing behind it", and I would have gone
# looking for deleted tests. One sentinel, three meanings, and the only one it
# names is the one that is usually wrong.
#
# So: a distinct exit per state, awk instead of bc (awk is in FreeBSD base and
# bc is not guaranteed), and a refusal if any suite FAILED, because summing the
# ok lines of a transcript that also contains failures reports a number that is
# true and misleading.
#
# usage: test-count.sh <transcript>        prints the total to stdout
#   0  a total, printed, and it is > 0
#   2  no `test result:` line at all -- the suites never reported (build failure?)
#   3  at least one suite reported FAILED -- refusing to sum a red transcript
#   4  the suites ran and the total is 0 -- a green with nothing behind it
#   5  a result line this parser could not read -- the format moved under us
set -u
f=${1:-}
[ -n "$f" ] && [ -r "$f" ] || { echo "test-count: usage: test-count.sh <transcript>" >&2; exit 64; }

if grep -qE '^test result: FAILED' "$f"; then
    echo "test-count: the transcript contains a FAILED suite; refusing to sum it" >&2
    exit 3
fi

lines=$(grep -cE '^test result:' "$f" || true)
if [ "${lines:-0}" -eq 0 ]; then
    echo "test-count: no 'test result:' line in the transcript -- the suites never reported" >&2
    exit 2
fi

# EVERY result line must parse. An unparsed line is not a zero: it is this
# parser disagreeing with cargo's format, which is a finding about the parser.
total=$(/usr/bin/awk '
    /^test result:/ {
        seen++
        if (match($0, /[0-9]+ passed/)) { n = substr($0, RSTART, RLENGTH); sub(/ passed/, "", n); sum += n }
        else { bad++ }
    }
    END { printf "%d %d %d\n", sum+0, seen+0, bad+0 }
' "$f")
sum=${total%% *}; rest=${total#* }; seen=${rest%% *}; bad=${rest##* }

if [ "$bad" -ne 0 ]; then
    echo "test-count: $bad of $seen result line(s) did not carry an 'N passed' field" >&2
    exit 5
fi
# THE NUMBER GOES TO STDOUT ONLY ON SUCCESS. A refusal that also printed "0"
# would be read by `n=$(test-count.sh f)` as a count, and an unchecked caller
# would carry it forward as a fact -- the same collapse this script exists to
# undo. On every refusal stdout stays EMPTY, so a caller that ignores the rc
# gets a visibly absent value rather than a plausible wrong one.
[ "$sum" -gt 0 ] || { echo "test-count: the suites ran and the total is 0 -- a green with nothing behind it" >&2; exit 4; }
echo "$sum"
exit 0
