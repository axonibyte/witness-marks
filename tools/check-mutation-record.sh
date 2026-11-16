#!/bin/sh
# Refuse a change that ADDS test assertions without recording a mutation kill.
# The methodology's "new assertions get mutation-checked" turned into a gate, so
# that the rule survives the week somebody is in a hurry.
#
# usage: tools/check-mutation-record.sh <base-ref> <head-ref>
#
# WHAT COUNTS AS A RECORD, either one:
#   - a `MUTATION:` line in any commit message in the range, or
#   - a file added or changed under tests/mutations/ in the range.
#
# WHAT COUNTS AS A NEW ASSERTION: an ADDED line (a `+` line in the diff, not a
# context line) carrying `assert!`, `assert_eq!`, `assert_ne!`, `#[test]`, or
# -- for the shell arms this repo also keeps -- a line defining an `arm ` call.
#
# IT FAILS CLOSED ON AN UNKNOWN RANGE. If the two refs do not resolve, or there
# is no merge base, this REFUSES rather than passing: a gate that cannot see the
# change has not cleared it, and "no new assertions found" is exactly what an
# empty diff looks like. The distinction between "I looked and there were none"
# and "I could not look" is the whole point, and collapsing it is the defect
# this repo keeps finding in its own instruments.
#
# RESOLVING THE RANGE IN CI IS ITS OWN PROBLEM, so it is a mode of this script
# rather than shell written inside the workflow. `--from-event` reads the push
# event's `before` sha from $GITHUB_EVENT_PATH; a workflow cannot otherwise know
# which commits a push carried, and guessing HEAD~1 silently checks one commit
# of a push that had five. If the event file is absent, unreadable, or carries
# no usable `before`, this REFUSES -- it does not quietly fall back, because a
# fallback is how a gate ends up clearing changes it never saw.
set -u
if [ "${1:-}" = "--from-event" ]; then
    ev=${GITHUB_EVENT_PATH:-}
    [ -n "$ev" ] || { echo "mutation-record: --from-event but GITHUB_EVENT_PATH is unset; REFUSING" >&2; exit 66; }
    [ -r "$ev" ] || { echo "mutation-record: --from-event but '$ev' is not readable; REFUSING" >&2; exit 66; }
    before=$(python3 "$(dirname "$0")/event-before.py" "$ev" || true)
    [ -n "$before" ] || { echo "mutation-record: the push event carries no usable 'before' sha (a new branch?); REFUSING" >&2; exit 66; }
    set -- "$before" "${2:-HEAD}"
fi
base=${1:-}; head=${2:-}
[ -n "$base" ] && [ -n "$head" ] || { echo "usage: check-mutation-record.sh <base-ref> <head-ref> | --from-event [head]" >&2; exit 64; }

git rev-parse --verify --quiet "$base^{commit}" >/dev/null \
  || { echo "mutation-record: base ref '$base' does not resolve; REFUSING (a gate that cannot see the change has not cleared it)" >&2; exit 65; }
git rev-parse --verify --quiet "$head^{commit}" >/dev/null \
  || { echo "mutation-record: head ref '$head' does not resolve; REFUSING" >&2; exit 65; }
mb=$(git merge-base "$base" "$head" 2>/dev/null) \
  || { echo "mutation-record: no merge base between '$base' and '$head'; REFUSING" >&2; exit 65; }
[ -n "$mb" ] || { echo "mutation-record: empty merge base; REFUSING" >&2; exit 65; }

ncommits=$(git rev-list --count "$mb..$head")
if [ "$ncommits" -eq 0 ]; then
    echo "mutation-record: no commits in $base..$head; nothing to check"
    exit 0
fi

added=$(git diff --unified=0 "$mb..$head" -- '*.rs' '*.sh' \
        | grep -E '^\+' | grep -v '^+++' \
        | grep -cE 'assert(_eq|_ne)?!|#\[test\]|^\+[[:space:]]*arm[[:space:]]' || true)
added=${added:-0}

record=no
git log --format=%B "$mb..$head" | grep -qE '^[[:space:]]*MUTATION:' && record=yes
git diff --name-only "$mb..$head" | grep -qE '^tests/mutations/' && record=yes

echo "mutation-record: $ncommits commit(s), $added added assertion line(s), record=$record"
if [ "$added" -gt 0 ] && [ "$record" = no ]; then
    echo "REFUSED: this range adds $added assertion line(s) and records no mutation kill." >&2
    echo "  Record one of:" >&2
    echo "    - a 'MUTATION:' line in a commit message naming what was broken and which test died, or" >&2
    echo "    - a file under tests/mutations/ (tools/mutate.sh runs those and prints the kill)." >&2
    echo "  A test never observed failing is a test whose value is unmeasured." >&2
    exit 1
fi
exit 0
