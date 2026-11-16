#!/bin/sh
# Tier 3, source-as-data: every gate CI runs must also be in tools/battery.sh.
#
# WHY. On 2026-11-16 milestone 2b was pushed red. The suite, clippy, fmt and
# the mutants were all run locally and all green; `tools/tests/t_*.sh` was not
# run at all, and it had been failing since the milestone's first commit --
# t_artifact_paths.sh reported crates/wm-csg/src/tree.rs as UNBALANCED because
# its brace walk counted braces inside char literals, which a CSG-dump parser
# must contain.
#
# battery.sh was written to be the single definition of "the battery". That
# fixes the instance. This arm fixes the class: a gate added to ci.yml and not
# to battery.sh is a gate that only a red push will find, and that is the exact
# shape of the failure. So the two are compared, mechanically, here.
#
# The comparison is one-directional ON PURPOSE. battery.sh may hold MORE than
# CI does; it may not hold less.
set -u
ROOT=${BATTERY_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
pass=0; fail=0
ok() { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no() { fail=$((fail+1)); printf '  FAIL %s\n' "$1"; }
fin() { printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail)); [ "$fail" = 0 ] || exit 1; exit 0; }

CI="$ROOT/.forgejo/workflows/ci.yml"
BAT="$ROOT/tools/battery.sh"
[ -r "$CI" ]  || { no "no ci.yml at $CI"; fin; }
[ -r "$BAT" ] || { no "no battery.sh at $BAT"; fin; }

# The gate vocabulary: a tools/ script CI invokes, or a cargo subcommand it
# uses as a gate. Extracted, never listed by hand -- a hand-kept list is the
# thing that drifted in the first place.
gates_of() {
    {
        /usr/bin/grep -oE 'tools/[a-z-]+\.(sh|py)' "$1"
        /usr/bin/grep -oE 'cargo (test|clippy|fmt)' "$1" | /usr/bin/sed 's/ /-/'
        /usr/bin/grep -oE 'tools/tests/t_\*\.sh|t_\*\.sh' "$1" | /usr/bin/sed 's|.*|tools/tests/t_star|'
    } | sort -u
}

ci_gates=$(gates_of "$CI")
bat_gates=$(gates_of "$BAT")

# REFUSE A BLIND COMPARISON. Zero gates extracted from ci.yml means the
# patterns or the workflow moved, and "no difference" would be a green with
# nothing behind it.
n_ci=$(printf '%s\n' "$ci_gates" | /usr/bin/grep -c . || true)
if [ "${n_ci:-0}" -lt 4 ]; then
    no "only ${n_ci:-0} gate(s) extracted from ci.yml; the patterns or the workflow moved, so this check is blind"
    fin
fi
ok "$n_ci gate(s) read out of ci.yml"

missing=''
for g in $ci_gates; do
    printf '%s\n' "$bat_gates" | /usr/bin/grep -qxF "$g" || missing="$missing $g"
done
if [ -n "$missing" ]; then
    no "ci.yml runs gate(s) that tools/battery.sh does not, so a local battery can be green while CI is red:$missing"
else
    ok "tools/battery.sh runs every gate ci.yml does"
fi

# AND THE COMPARISON ITSELF IS TESTED, because a loop that never finds a
# difference and a loop that cannot are the same green.
#
# THE SELF-TEST RUNS ONLY ON THE REAL TREE. It re-invokes this script against a
# fixture, and the first version let the fixture run re-invoke itself in turn --
# unbounded recursion, which the 300s arm timeout reported as rc 124 under a
# summary that had already printed. $BATTERY_ROOT being set is exactly "this is
# the inner run", so it is the guard.
if [ -n "${BATTERY_ROOT:-}" ]; then
    fin
fi
FIX=$(mktemp -d) || { no "could not make a fixture dir"; fin; }
trap 'rm -rf "$FIX"' EXIT INT TERM HUP
mkdir -p "$FIX/.forgejo/workflows" "$FIX/tools"
cat > "$FIX/.forgejo/workflows/ci.yml" <<'YML'
run: |
  sh tools/check-mutation-record.sh --from-event HEAD
  for t in tools/tests/t_*.sh; do sh "$t"; done
  cargo test --locked --workspace
  cargo clippy --locked --workspace
  cargo fmt --all -- --check
  sh tools/brand-new-gate.sh
YML
cat > "$FIX/tools/battery.sh" <<'BSH'
sh tools/check-mutation-record.sh
for t in tools/tests/t_*.sh; do sh "$t"; done
cargo test --locked --workspace
cargo clippy --locked --workspace
cargo fmt --all -- --check
BSH
OUT=$(BATTERY_ROOT="$FIX" sh "$0" 2>&1); frc=$?
if [ "$frc" -ne 0 ] && printf '%s\n' "$OUT" | /usr/bin/grep -q 'brand-new-gate'; then
    ok "a gate present only in ci.yml is detected and named"
else
    no "the comparison did not catch a CI-only gate; it cannot fail, so its green means nothing (rc=$frc)"
    printf '%s\n' "$OUT" | sed 's/^/         /'
fi
fin
