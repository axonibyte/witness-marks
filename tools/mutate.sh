#!/bin/sh
# Apply one named mutation, run the suite, assert the TARGETED test fails,
# restore. The methodology's rule -- "new assertions get mutation-checked" --
# mechanized, so a kill is a command anyone can re-run rather than a sentence
# in a commit message.
#
# usage: tools/mutate.sh tests/mutations/<name>/        run one
#        tools/mutate.sh --all                          run every mutant
#
# A MUTANT IS A DIRECTORY, not a line in a table, and the reason is quoting.
# A find/replace pair is arbitrary source text: it contains quotes, backslashes,
# newlines and shell metacharacters, and every delimiter-based format I might
# pick is a format some real mutation cannot be written in. Four files holds the
# literals byte-exact with nothing to escape:
#
#   subject       the path to mutate, one line, repo-relative
#   find          the exact text to replace; must occur EXACTLY ONCE
#   replace       what to put there
#   expect-fail   a substring of the test name that must go from pass to FAIL
#   test-cmd      optional; defaults to `cargo test --locked --workspace`
#
# THE FOUR REFUSALS, each of which is a mutation run reporting green while
# proving nothing, and each of which I have actually shipped:
#
#   1. THE MUTATION DID NOT APPLY. A sed that matches nothing leaves the file
#      correct, the suite passes, and an unguarded driver calls that "survived".
#      An empty FAIL list from a run that patched nothing is the strongest false
#      negative there is. So: `find` must occur exactly once, and the file must
#      differ afterwards, or this refuses before running anything.
#   2. THE ARM WAS ALREADY RED. A kill means pass -> FAIL. If the targeted test
#      is failing before the mutation, its failure after says nothing, so the
#      baseline is run first and must be green.
#   3. THE SUBJECT WAS NEVER RESTORED. A mutation left on disk silently becomes
#      the code. The restore runs from a trap on every exit path including a
#      signal, and the restored bytes are compared against the original.
#   4. THE WRONG TEST DIED. A mutant that breaks the build fails everything,
#      which looks like a kill for whichever arm you were watching. So the
#      failure must NAME the expected test, not merely be nonzero.
set -u
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT" || exit 70

run_one() {
    d=$1
    for need in subject find replace expect-fail; do
        [ -r "$d/$need" ] || { echo "mutate: $d has no '$need'" >&2; return 70; }
    done
    subj=$(cat "$d/subject")
    want=$(cat "$d/expect-fail")
    cmd='cargo test --locked --workspace'
    [ -r "$d/test-cmd" ] && cmd=$(cat "$d/test-cmd")
    name=$(basename "$d")
    [ -r "$ROOT/$subj" ] || { echo "mutate: $name: subject '$subj' is not readable" >&2; return 70; }

    # REFUSAL 1a: the find text must be present exactly once.
    n=$(python3 - "$ROOT/$subj" "$d/find" <<'PY'
import sys
src=open(sys.argv[1],encoding='utf-8').read(); pat=open(sys.argv[2],encoding='utf-8').read()
print(src.count(pat))
PY
)
    if [ "$n" != 1 ]; then
        echo "mutate: $name: 'find' occurs $n time(s) in $subj; it must occur exactly once" >&2
        return 71
    fi

    bak=$(mktemp) || return 70
    cp "$ROOT/$subj" "$bak"
    # REFUSAL 3: restore on EVERY exit path, signals included.
    trap 'cp "$bak" "$ROOT/$subj"; rm -f "$bak"; trap - EXIT HUP INT TERM' EXIT
    trap 'cp "$bak" "$ROOT/$subj"; rm -f "$bak"; exit 143' HUP INT TERM

    # REFUSAL 2: the baseline must be GREEN, or a later red proves nothing.
    if ! sh -c "$cmd" >"$bak.base" 2>&1; then
        echo "mutate: $name: the suite is ALREADY RED before the mutation; a kill means"
        echo "        pass -> FAIL, so there is nothing to measure here. Fix the baseline."
        tail -20 "$bak.base"; rm -f "$bak.base"; return 72
    fi
    rm -f "$bak.base"

    python3 - "$ROOT/$subj" "$d/find" "$d/replace" <<'PY'
import sys
p,fp,rp=sys.argv[1],sys.argv[2],sys.argv[3]
src=open(p,encoding='utf-8').read()
pat=open(fp,encoding='utf-8').read(); rep=open(rp,encoding='utf-8').read()
open(p,'w',encoding='utf-8').write(src.replace(pat,rep,1))
PY

    # REFUSAL 1b: the file must actually differ now.
    if cmp -s "$bak" "$ROOT/$subj"; then
        echo "mutate: $name: the file is unchanged after replacement -- the mutation did not apply" >&2
        return 71
    fi

    sh -c "$cmd" >"$bak.mut" 2>&1; mrc=$?
    out=$(cat "$bak.mut"); rm -f "$bak.mut"

    verdict=0
    if [ "$mrc" -eq 0 ]; then
        echo "  SURVIVED  $name -- the suite is still green, so nothing covers this"
        verdict=1
    # REFUSAL 4: the named test must be the one that died.
    elif printf '%s' "$out" | grep -qE "^(test .*$want.* \.\.\. FAILED|failures:)" \
         && printf '%s' "$out" | grep -q "$want"; then
        echo "  KILLED    $name -- by $want"
    else
        echo "  MISFIRED  $name -- the suite failed but '$want' is not named in the failure;"
        echo "            a mutant that breaks the BUILD fails every arm and looks like a kill"
        printf '%s' "$out" | grep -E '^(error|warning: unused)' | head -4 | sed 's/^/            /'
        verdict=2
    fi
    return $verdict
}

if [ "${1:-}" = "--all" ]; then
    rc=0
    for d in tests/mutations/*/; do
        [ -d "$d" ] || continue
        ( run_one "${d%/}" ); r=$?
        [ "$r" -eq 0 ] || rc=$r
    done
    [ "$rc" -eq 0 ] && echo "-- every mutant was killed by the test that names it"
    exit "$rc"
fi
[ $# -eq 1 ] || { echo "usage: mutate.sh <tests/mutations/NAME/ | --all>" >&2; exit 64; }
run_one "${1%/}"
