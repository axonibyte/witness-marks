#!/bin/sh
# THE BATTERY: every gate CI runs, in one place, runnable before a push.
#
# WHY THIS EXISTS. The gates were a list in .forgejo/workflows/ci.yml and a
# habit in whoever was pushing. On 2026-11-16 that habit ran the suite, clippy,
# fmt and the mutants -- and not `tools/tests/t_*.sh`. Milestone 2b went out
# red: t_artifact_paths.sh had been reporting crates/wm-csg/src/tree.rs as
# UNBALANCED since the first commit of the milestone, and nothing local looked.
#
# A gate CI runs that no local command runs is a gate discovered by a red
# push. So this script is the definition, ci.yml calls the same tools, and
# "the battery is green" has one meaning.
#
# EVERY GATE REPORTS ITS OWN EXIT CODE. An earlier version of this habit ran
# `cargo clippy ... >/dev/null 2>&1 && echo clean`, which prints NOTHING on
# failure -- a silent gate reads exactly like a gate nobody added.
#
# usage: tools/battery.sh [--no-mutants]
#   --no-mutants skips the slowest gate ONLY, and says so in the summary, so a
#   partial run cannot be reported as a full one.
set -u
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT" || exit 70
mutants=yes
[ "${1:-}" = "--no-mutants" ] && mutants=no

fails=''
gate() {
    name=$1
    shift
    printf '\n=== %s\n' "$name"
    rc=0
    "$@" || rc=$?
    printf -- '--- %s rc=%d\n' "$name" "$rc"
    [ "$rc" -eq 0 ] || fails="$fails $name(rc=$rc)"
    return 0
}

arms() {
    bad=0
    for t in tools/tests/t_*.sh; do
        printf '  == %s\n' "$t"
        arc=0
        timeout 300 sh "$t" > "$ROOT/.battery-arms.out" 2>&1 || arc=$?
        /usr/bin/grep -E '^-- [0-9]+/[0-9]+ arms pass' "$ROOT/.battery-arms.out" \
            || { echo "     NO COUNT: it did not run"; bad=1; }
        [ "$arc" -eq 0 ] || { /usr/bin/grep -E '^  FAIL' "$ROOT/.battery-arms.out" | sed 's/^/   /'; bad=1; }
    done
    rm -f "$ROOT/.battery-arms.out"
    return "$bad"
}

suite() {
    src=0
    timeout 1800 cargo test --locked --workspace > "$ROOT/.battery-test.out" 2>&1 || src=$?
    # ci-report-count.sh, THE SAME READER CI USES. battery.sh called
    # test-count.sh here, which is the layer underneath it -- near enough to
    # look right and not the same gate, which is how a list kept in two places
    # drifts. t_battery_covers_ci.sh found it.
    sh tools/ci-report-count.sh "$ROOT/.battery-test.out" "$src" || {
        tail -40 "$ROOT/.battery-test.out"; rm -f "$ROOT/.battery-test.out"; return 1; }
    rm -f "$ROOT/.battery-test.out"
    return "$src"
}

# THE MUTATION RECORD, over the commits a push would carry. CI resolves the
# range from the push event's `before` sha, which does not exist locally, so
# the local equivalent is origin/main..HEAD -- the commits about to go out.
# REFUSES rather than passing if origin/main is not resolvable, because a gate
# that cannot see the change has not cleared it.
mutation_record() {
    git rev-parse --verify --quiet origin/main >/dev/null 2>&1 || {
        echo "no origin/main to measure against; fetch first. A range this gate cannot resolve is not a range it cleared"
        return 65
    }
    sh tools/check-mutation-record.sh origin/main HEAD
}

# THE ARMS AT A FOREIGN ROOT, which is the gap that let CI stay red for TEN
# COMMITS while every local run of this script printed GREEN.
#
# WHAT WENT WRONG, exactly. tests/vectors/vectors.toml carried ABSOLUTE paths
# into this checkout -- `use </data/.../witness-marks/tests/fixtures/...>`. On
# this machine that path resolves INSIDE the tree under test, so
# t_fixtures_self_contained.sh passed. CI clones to a different root, where the
# same path points at a foreign tree, and the arm correctly called it an
# escape. The arm was right; the environment this battery ran it in was the
# thing that lied.
#
# t_battery_covers_ci.sh asserts this script covers every GATE ci.yml runs, and
# it did. It cannot assert the battery runs them under CI's CONDITIONS, and the
# root is one of those. So the arms run twice: once here, once over a `git
# archive` of HEAD unpacked somewhere else.
#
# IT TESTS THE WORKING TREE'S TRACKED FILES, not HEAD. `git stash create`
# writes a commit object for the current working tree WITHOUT touching the
# index, the tree or the stash list, so the copy is exactly what a commit would
# carry -- tracked files only, which is also what a clone gets. Using HEAD
# instead would make this gate lag the fix by one commit and break the rule it
# serves: the battery must be green BEFORE the commit, and a gate that can only
# see the previous commit cannot clear the next one. Falls back to HEAD when
# there is nothing uncommitted, which is what stash create returns empty for.
foreign_root() {
    far=$(mktemp -d) || { echo "no temp dir for the foreign-root pass"; return 1; }
    snap=$(git stash create 2>/dev/null)
    [ -n "$snap" ] || snap=HEAD
    printf '  testing %s at a foreign root\n' "$([ "$snap" = HEAD ] && echo HEAD || echo "the working tree")"
    git archive --format=tar "$snap" | tar -x -C "$far" || { rm -rf "$far"; return 1; }
    bad=0
    for t in "$far"/tools/tests/t_*.sh; do
        arc=0
        ( cd "$far" && timeout 300 sh "tools/tests/$(basename "$t")" ) > "$far/.out" 2>&1 || arc=$?
        if ! /usr/bin/grep -qE '^-- [0-9]+/[0-9]+ arms pass' "$far/.out"; then
            printf '  == %s  NO COUNT at a foreign root\n' "$(basename "$t")"; bad=1
        elif [ "$arc" -ne 0 ]; then
            printf '  == %s  FAILS at a foreign root but not here\n' "$(basename "$t")"
            /usr/bin/grep -E '^  FAIL' "$far/.out" | sed 's/^/     /'; bad=1
        fi
    done
    [ "$bad" -eq 0 ] && printf '  every arm passes at %s too\n' "$far"
    rm -rf "$far"
    return "$bad"
}

gate "new assertions carry a recorded kill" mutation_record
gate "the tooling's own arms" arms
gate "the arms again at a FOREIGN ROOT, as CI runs them" foreign_root
gate "cargo test --locked --workspace" suite
gate "clippy -D warnings" timeout 900 cargo clippy --locked --workspace --all-targets -- -D warnings
gate "cargo fmt --check" timeout 300 cargo fmt --all -- --check
if [ "$mutants" = yes ]; then
    gate "every recorded mutant still kills" timeout 3600 sh tools/mutate.sh --all
    # THE NETWORK MUTANTS, outside the --all glob because CI runs that one and a
    # gate must not depend on pypi.org being reachable. See
    # tests/mutations-net/README.md.
    net_mutants() {
        bad=0
        for d in tests/mutations-net/*/; do
            [ -d "$d" ] || continue
            timeout 900 sh tools/mutate.sh "${d%/}" || bad=1
        done
        return "$bad"
    }
    gate "every network mutant still kills" net_mutants
fi

printf '\n'
if [ -n "$fails" ]; then
    echo "BATTERY RED:$fails"
    exit 1
fi
if [ "$mutants" = no ]; then
    echo "BATTERY GREEN except the mutants, WHICH WERE SKIPPED (--no-mutants)"
    exit 0
fi
echo "BATTERY GREEN: arms, suite, clippy, fmt, mutants"
