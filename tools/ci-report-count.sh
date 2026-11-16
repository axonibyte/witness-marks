#!/bin/sh
# Turn tools/test-count.sh's exit into the line CI should print and the exit CI
# should take. It lives here rather than inline in ci.yml because a `case`
# written inside a workflow is glue that nothing can drive: the only way to
# exercise it is to push, and the only report is the colour of a run.
#
# usage: ci-report-count.sh <transcript> [<cargo-rc>]
#   0  a count was printed and it is positive
#   1  this run must not be called green, and the line above says why
#
# A FAILED SUITE IN THE TRANSCRIPT FAILS THE RUN HERE, and the first draft of
# this script did not: it printed "the count is withheld", exited 0, and left
# the failing to cargo's rc in the caller. Driven against mixed.txt with a
# cargo rc of 0 -- a transcript carrying a FAILED suite beside an ok one -- that
# draft reported GREEN. It is the same defect I had just removed from ci.yml one
# level up: a correct outcome that depends on a SECOND check being present and
# in the right order. The transcript says tests failed; that is sufficient, and
# cargo's rc is carried as well rather than instead.
set -u
f=${1:-}; cargo_rc=${2:-0}
[ -n "$f" ] || { echo "ci-report-count: usage: ci-report-count.sh <transcript> [cargo-rc]" >&2; exit 64; }
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

crc=0; passed=$(sh "${COUNT_SRC:-$here/test-count.sh}" "$f") || crc=$?
case "$crc" in
  0) echo "tests passed: $passed" ;;
  2) echo "COUNT: no suite reported -- read as a BUILD failure, not an empty suite"; exit 1 ;;
  3) echo "COUNT: the transcript carries a FAILED suite, so the count is withheld"; exit 1 ;;
  4) echo "COUNT: zero tests ran -- a green with nothing behind it"; exit 1 ;;
  5) echo "COUNT: a result line did not parse -- cargo's format moved; a TOOLING finding"; exit 1 ;;
  *) echo "COUNT: test-count.sh exited $crc, which this script does not know how to read"; exit 1 ;;
esac
[ "$cargo_rc" -eq 0 ] || exit 1
exit 0
