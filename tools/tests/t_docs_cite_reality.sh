#!/bin/sh
# Tier 3, source-as-data: CONTRIBUTING.md claims certain rules are mechanized
# and names the tool and the CI step for each. Those names are CITATIONS, and a
# citation that stops resolving is worse than none -- a contributor reads
# "enforced by CI" and believes it.
#
# THE TABLE IS THE DATA. This reads the rows out of CONTRIBUTING.md rather than
# keeping its own copy, because a list maintained beside a document drifts from
# it and then tests itself. Add a row and this starts checking it with no edit
# here.
#
# NO PIPELINE AROUND THE LOOP. The first version piped the rows into `while`,
# which runs in a SUBSHELL, so every failure counted inside it was discarded at
# the end -- it needed a temp file to carry a count back out, and a test whose
# failure count can be lost is not a test. The rows go to a file and the loop
# reads from it.
#
# WHAT IT CANNOT PROVE, said plainly because "every citation resolves" invites
# the opposite reading: that a step named in ci.yml actually enforces the rule
# the table attributes to it. A step could be renamed to match and do nothing.
# This catches drift and deletion, not a lie.
set -u
ROOT=${DOCS_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
C="$ROOT/CONTRIBUTING.md"; W="$ROOT/.forgejo/workflows/ci.yml"
pass=0; fail=0
ok()  { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no()  { fail=$((fail+1)); printf '  FAIL %s\n' "$1"; }
done_() { printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail)); [ "$fail" = 0 ] || exit 1; exit 0; }

for f in "$C" "$W"; do
    [ -r "$f" ] || { no "$f is not readable"; done_; }
done

ROWS=$(mktemp) || exit 70
trap 'rm -f "$ROWS"' EXIT HUP INT TERM
/usr/bin/awk -F'|' '/^\| .* \| .*`.*`.* \| \*.*\* \|/ {print NR"\t"$3"\t"$4}' "$C" > "$ROWS"
n=$(/usr/bin/grep -c . "$ROWS" || true); n=${n:-0}

# REFUSE AN EMPTY TABLE. A parser that stops matching reports "every citation
# resolves" over zero citations, which is the strongest false green there is.
if [ "$n" -lt 3 ]; then
    no "only $n mechanized row(s) parsed from CONTRIBUTING.md; the table moved, or the parser did -- either way this test is blind"
    done_
fi
ok "$n mechanized rows parsed from CONTRIBUTING.md"

while IFS="$(printf '\t')" read -r ln tools step; do
    [ -n "${ln:-}" ] || continue
    # NO PATHNAME EXPANSION ON A CITATION. Without `set -f` the shell expands
    # `tools/tests/t_*.sh` against the CURRENT directory before the loop body
    # ever sees it, so the glob branch below never fires and each resolved name
    # is then checked against $ROOT -- a different tree. It passed anyway while
    # the two trees agreed, which is the worst kind of passing: the mechanism
    # was wrong and only the answer was right. Driven by pointing DOCS_ROOT at a
    # copy with the arms removed, where it reported three missing files instead
    # of one unmatched glob.
    # `set -f` must stay on THROUGH the for-expansion, not merely around the
    # substitution: the unquoted $toks in `for t in $toks` is where pathname
    # expansion happens, and my first patch turned globbing back on one line too
    # early, so the mutant kept reporting three missing files instead of one
    # unmatched glob. It is lifted inside the body only where a real match is
    # wanted, and restored immediately.
    set -f
    toks=$(printf '%s' "$tools" | tr -cs 'A-Za-z0-9_./*-' ' ')
    for t in $toks; do
        case "$t" in
            tools/*'*'*)
                # A GLOB IS A LEGITIMATE CITATION -- "all the tooling arms" is
                # the honest name for that row. It resolves when it matches at
                # least one executable file.
                m=0
                set +f
                for g in $ROOT/$t; do [ -x "$g" ] && m=$((m+1)); done
                set -f
                if [ "$m" -gt 0 ]; then ok "$t matches $m executable file(s)"
                else no "$t is cited by CONTRIBUTING.md:$ln and matches nothing executable"; fi ;;
            tools/*)
                if [ -x "$ROOT/$t" ]; then ok "$t exists and is executable"
                else no "$t is cited by CONTRIBUTING.md:$ln and is missing or not executable"; fi ;;
        esac
    done
    set +f
    s=$(printf '%s' "$step" | sed -e 's/^[^*]*\*//' -e 's/\*[^*]*$//')
    if [ -n "$s" ]; then
        if /usr/bin/grep -qF "name: $s" "$W"; then ok "ci.yml has a step named \"$s\""
        else no "CONTRIBUTING.md:$ln cites a CI step \"$s\" that ci.yml does not have"; fi
    fi
done < "$ROWS"
done_
