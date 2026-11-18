#!/bin/sh
# Tier 3, source-as-data: the falsification plan's citations resolve.
#
# SAME RULE AS t_docs_cite_reality.sh: a citation that stops resolving is worse
# than none, because a reader sees a path and believes the claim beside it. The
# table is read OUT OF the document rather than copied, so adding a row starts
# checking it with no edit here.
#
# AND IT IS DRIVEN AGAINST BROKEN COPIES, which the first version was not. That
# version only ever ran against the real document, so a mutant that DELETED the
# path-existence check SURVIVED -- with the check gone nothing was found wrong,
# and "nothing found wrong" is exactly what passing looks like. A checker needs
# a negative fixture or its own removal is invisible. Three mutants survived
# that way before this was restructured.
#
# WHAT THIS CANNOT PROVE, said plainly: that a cited file supports the claim it
# is cited for. It catches drift and deletion, not a misreading.
set -u
ROOT=${FALSIFY_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
pass=0; fail=0
ok() { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no() { fail=$((fail+1)); printf '  FAIL %s -- %s\n' "$1" "${2:-}"; }
judge() { if [ "$2" = 0 ]; then ok "$1"; else no "$1" "$3"; fi; }
fin() { printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail)); [ "$fail" = 0 ] || exit 1; exit 0; }

PLAN="$ROOT/docs/falsification-plan.md"
[ -r "$PLAN" ] || { no "docs/falsification-plan.md is missing" ""; fin; }
TMP=$(mktemp -d) || { no "mktemp failed" ""; fin; }
trap 'rm -rf "$TMP"' EXIT

# check_plan <plan-path> -> 0 if every rule holds, non-zero with a reason
check_plan() {
    p=$1; why=""
    : > "$TMP/why"   # a stale reason reads as this call's and misled an arm
    /usr/bin/grep -E '^\| .* \| [a-z]' "$p" | /usr/bin/grep -vE '^\| claim \|' \
      | sed 's/^| *//; s/ *|$//' > "$TMP/rows"
    rows=$(/usr/bin/grep -c . "$TMP/rows" || true)
    [ "$rows" -ge 6 ] || why="$why [table has only $rows rows]"
    while IFS='|' read -r _c path; do
        q=$(printf '%s' "$path" | sed 's/^ *//; s/ *$//')
        [ -n "$q" ] || continue
        [ -e "$ROOT/$q" ] || why="$why [cited path absent: $q]"
    done < "$TMP/rows"
    for t in 1 2 4 5; do
        /usr/bin/grep -q "^## Test $t " "$p" || why="$why [no section for test $t]"
        /usr/bin/grep -A 4 "^## Test $t " "$p" | /usr/bin/grep -q "Current evidence" \
            || why="$why [test $t states no current evidence]"
    done
    /usr/bin/grep -q "scope-versus-measurement accounting" "$p" \
        || why="$why [no scope accounting]"
    [ -z "$why" ] || { printf '%s' "$why" > "$TMP/why"; return 1; }
    return 0
}

check_plan "$PLAN"
judge "the committed plan satisfies every rule: 8 citations, tests 1/2/4/5, evidence before experiment" \
      $? "$(cat "$TMP/why" 2>/dev/null)"

# -- THE NEGATIVE FIXTURES. Each breaks the plan one way and must be caught.
sed 's|^| the corpus schema has no cost field | tests/corpus/build.toml |$|| a claim | docs/no-such-file.md ||' \
    "$PLAN" > "$TMP/dangling.md" 2>/dev/null || cp "$PLAN" "$TMP/dangling.md"
printf '\n| an invented claim | docs/this-file-does-not-exist.md |\n' >> "$TMP/dangling.md"
check_plan "$TMP/dangling.md"
[ $? -ne 0 ] && /usr/bin/grep -q "cited path absent" "$TMP/why"
judge "a plan citing a path that does not exist is caught" $? "$(cat "$TMP/why" 2>/dev/null)"

sed 's|^## Test 4 .*|## (the section that used to be test 4)|' "$PLAN" > "$TMP/notest.md"
check_plan "$TMP/notest.md"
[ $? -ne 0 ] && /usr/bin/grep -q "no section for test 4" "$TMP/why"
judge "a plan missing one of the four commissioned tests is caught" $? "$(cat "$TMP/why" 2>/dev/null)"

sed 's|\*\*Current evidence: has not fired|**has not fired|' "$PLAN" > "$TMP/noev.md"
check_plan "$TMP/noev.md"
[ $? -ne 0 ] && /usr/bin/grep -q "states no current evidence" "$TMP/why"
judge "a test proposing an experiment without stating what is already known is caught" $? "$(cat "$TMP/why" 2>/dev/null)"

/usr/bin/grep -v "scope-versus-measurement accounting" "$PLAN" > "$TMP/noscope.md"
check_plan "$TMP/noscope.md"
[ $? -ne 0 ] && /usr/bin/grep -q "no scope accounting" "$TMP/why"
judge "a plan omitting the scope accounting issue 46 asked to count is caught" $? "$(cat "$TMP/why" 2>/dev/null)"

/usr/bin/grep -vE '^\| .* \| [a-z]' "$PLAN" > "$TMP/notable.md"
check_plan "$TMP/notable.md"
[ $? -ne 0 ] && /usr/bin/grep -q "rows" "$TMP/why"
judge "a plan with no citation table at all is caught, not read as having nothing to check" $? "$(cat "$TMP/why" 2>/dev/null)"

# -- AND THE CORPUS CLAIM, checked against the corpus rather than asserted.
if python3 - "$ROOT" <<'PY'
import sys, tomllib, pathlib
root = pathlib.Path(sys.argv[1])
d = tomllib.load(open(root / "tests/corpus/build.toml", "rb"))
fields = set(d["incident"][0].keys())
sys.exit(0 if not (fields & {"cost", "scope", "lost_to"}) else 1)
PY
then ok "the plan's claim that the corpus has no cost field is still true of the corpus"
else no "the corpus now carries a cost field; the plan's scope section is out of date" ""
fi

fin
