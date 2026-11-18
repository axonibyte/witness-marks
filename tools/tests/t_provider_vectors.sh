#!/bin/sh
# Tier 3: the PROVIDER side of the vectors, asserted (issue 54).
#
# WHY THIS IS IN THE CI GLOB when the vectors themselves are not. Running a
# vector needs OpenSCAD and a render; asserting the provider side needs
# neither, because BOTH figures are already recorded in the file. So this is
# arithmetic over tests/vectors/vectors.toml and it runs on every push, which
# is the difference between a tier that catches drift and a tier someone has to
# remember to run.
#
# WHAT WAS WRONG BEFORE IT. Eight of ten vectors carried a `provider = {...}`
# figure and tools/vector-run.py never read the key -- the file said so of
# itself, "recorded, not asserted". Zero of the three things learned about the
# provider on 2026-11-16 came from a test we own.
#
# THE COMPARISON IS CONTAINMENT, NOT DISTANCE, and that is the finding this
# work turned up: the provider reports to four decimals, so `14.1421` stands
# for [14.14205, 14.14215) and judging it against a closed form at a 1e-5
# tolerance asks it a question it cannot answer however correct it is. The
# rotated-cube-bbox vector failed exactly that way, 3.6e-5 against 1e-5, which
# reads as a provider defect and is our own tolerance being tighter than the
# provider's output.
set -u
ROOT=${PVEC_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
pass=0; fail=0
ok() { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no() { fail=$((fail+1)); printf '  FAIL %s -- %s\n' "$1" "${2:-}"; }
# ONE LABEL PER CHECK, ON BOTH PATHS. judge <label> <condition-rc> <detail>:
# mutate.sh's refusal 4 requires the expected label to appear IN A FAILURE
# LINE, and an arm whose failure text differs from its success text can never
# satisfy that -- every one of this file's five mutants MISFIRED that way
# before the labels were unified.
judge() { if [ "$2" = 0 ]; then ok "$1"; else no "$1" "$3"; fi; }
fin() { printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail)); [ "$fail" = 0 ] || exit 1; exit 0; }

RUN="$ROOT/tools/vector-run.py"
VEC="$ROOT/tests/vectors/vectors.toml"
[ -r "$RUN" ] || { no "tools/vector-run.py is missing"; fin; }
[ -r "$VEC" ] || { no "tests/vectors/vectors.toml is missing"; fin; }
TMP=$(mktemp -d) || { no "mktemp failed"; fin; }
trap 'rm -rf "$TMP"' EXIT

rc_of() { python3 "$RUN" "$1" --provider-only >"$TMP/out" 2>&1; echo $?; }

# -- the committed file, which is the arm the others are measured against
got=$(rc_of "$VEC")
n=$(/usr/bin/grep -c '^  ok ' "$TMP/out" || true)
[ "$got" = 0 ] && [ "$n" -ge 12 ]
judge "every recorded provider figure in the committed vectors is asserted and agrees" $? "rc $got, $n passing arms"

# -- a figure that is NOT a rounding of ours, with nothing recording why
cat > "$TMP/drift.toml" <<'EOF'
[[vector]]
id = "drifted"
provider = { volume = 1000.5 }
question = "volume"
model = "cube([10,10,10]);"
expected = 1000.0
tolerance = 1e-09
EOF
got=$(rc_of "$TMP/drift.toml")
[ "$got" = 1 ] && /usr/bin/grep -q "is not a rounding of" "$TMP/out"
judge "a provider figure that is not a rounding of ours fails, naming the delta" $? "rc $got"

# -- THE TOLERANCE MUST NOT RESCUE IT. This is the arm that matters most: the
#    sphere's real 9.1e-5 disagreement sat inside the vector's own 1e-4
#    tolerance, so a distance comparison would have reported it clean.
cat > "$TMP/hidden.toml" <<'EOF'
[[vector]]
id = "hidden-by-tolerance"
provider = { volume = 1000.0002 }
question = "volume"
model = "cube([10,10,10]);"
expected = 1000.0
tolerance = 0.01
EOF
got=$(rc_of "$TMP/hidden.toml")
[ "$got" = 1 ]
judge "a disagreement smaller than the vector's own tolerance still fails" $? "rc $got -- the tolerance rescued it"

# -- an accepted disagreement is accepted, and only up to its stated bound
cat > "$TMP/accepted.toml" <<'EOF'
[[vector]]
id = "accepted"
provider = { volume = 1000.0002 }
provider_disagrees = { max_delta = 0.001, because = "a stated reason" }
question = "volume"
model = "cube([10,10,10]);"
expected = 1000.0
tolerance = 0.01
EOF
got=$(rc_of "$TMP/accepted.toml")
[ "$got" = 0 ]
judge "a disagreement with a recorded cause and a bound that covers it passes" $? "rc $got, wanted 0"

sed 's/max_delta = 0.001/max_delta = 0.00001/' "$TMP/accepted.toml" > "$TMP/beyond.toml"
got=$(rc_of "$TMP/beyond.toml")
[ "$got" = 1 ] && /usr/bin/grep -q "beyond the accepted" "$TMP/out"
judge "an acceptance is a bound, not a waiver: one beyond its stated bound fails" $? "rc $got"

# -- a recorded fact with no closed form to judge it against IS the defect
cat > "$TMP/unjudged.toml" <<'EOF'
[[vector]]
id = "unjudged"
provider = { volume = 1000.0, component_count = 1 }
question = "volume"
model = "cube([10,10,10]);"
expected = 1000.0
tolerance = 1e-09
EOF
got=$(rc_of "$TMP/unjudged.toml")
[ "$got" = 1 ] && /usr/bin/grep -q "recorded and unasserted" "$TMP/out"
judge "a provider fact with no closed form to judge it against fails" $? "rc $got"

# -- ISSUE 53, AND IT BIT THIS FILE'S OWN AUTHOR. TOML has no null, so a
#    provider that answered `null` is recorded here as an ABSENT KEY, and the
#    first version of check_provider skipped absent keys. The provider's own
#    docs name the tri-state: mesh_health's `manifold` is "true, false, or null
#    when OpenSCAD did not perform the check".
cat > "$TMP/unanswered.toml" <<'EOF'
[[vector]]
id = "unanswered"
provider = { volume = 1000.0 }
components = 1
watertight = true
question = "volume"
model = "cube([10,10,10]);"
expected = 1000.0
tolerance = 1e-09
EOF
got=$(rc_of "$TMP/unanswered.toml")
[ "$got" = 1 ] && /usr/bin/grep -q "UNKNOWN, never a pass" "$TMP/out"
judge "a fact the vector asks for and the provider did not answer is UNKNOWN, never a pass" $? "rc $got"

# -- and an unknown DECLARED with a cause is knowledge rather than a gap
cat > "$TMP/declared.toml" <<'EOF'
[[vector]]
id = "declared-unknown"
provider = { volume = 1000.0 }
provider_unknown = { keys = ["is_watertight"], because = "mesh_health.manifold is null on the Manifold backend" }
watertight = true
question = "volume"
model = "cube([10,10,10]);"
expected = 1000.0
tolerance = 1e-09
EOF
got=$(rc_of "$TMP/declared.toml")
[ "$got" = 0 ] && /usr/bin/grep -q "UNKNOWN, declared" "$TMP/out"
judge "an unknown declared with a cause passes and stays visible as UNKNOWN in the output" $? "rc $got"

# -- A DECLARATION THAT WENT FALSE must fail, not keep waiving. Same lesson as
#    the README that named a path absent for weeks while reading as if it worked.
cat > "$TMP/stale.toml" <<'EOF'
[[vector]]
id = "stale-declaration"
provider = { volume = 1000.0, is_watertight = true }
provider_unknown = { keys = ["is_watertight"], because = "a cause that has expired" }
watertight = true
question = "volume"
model = "cube([10,10,10]);"
expected = 1000.0
tolerance = 1e-09
EOF
got=$(rc_of "$TMP/stale.toml")
[ "$got" = 1 ] && /usr/bin/grep -q "gone false" "$TMP/out"
judge "a declared unknown the provider has since answered fails: the declaration has gone false" $? "rc $got"

# -- NO RESULT is a third outcome
cat > "$TMP/none.toml" <<'EOF'
[[vector]]
id = "no-provider-side"
question = "volume"
model = "cube([10,10,10]);"
expected = 1000.0
tolerance = 1e-09
EOF
got=$(rc_of "$TMP/none.toml")
[ "$got" = 2 ]
judge "a file with no recorded provider figures REFUSES rather than reporting a clean run over nothing" $? "rc $got, wanted 2"

# ---------------------------------------------------------------------------
# ISSUE 53's STANDING GREP. The issue says "anything of ours that consumes
# mesh_health.manifold must treat null as unknown, never as a pass -- worth a
# grep before milestone 4 work lands." A grep run once is a fact about today;
# this is the same grep as an arm, so it answers for every day after.
#
# MEASURED 2026-11-18: no consumer exists. The field our vectors use is
# `is_watertight`, which this kernel answers; `mesh_health.manifold` is the
# provider's documented tri-state -- "true, false, or null when OpenSCAD did
# not perform the check" -- and it goes null on the Manifold backend, which is
# what issue 53 is. So this arm is a TRIPWIRE: it passes while there is no
# consumer, and the moment one appears it requires that file to say what it
# does with the unknown.
hits=$(cd "$ROOT" && /usr/bin/grep -rln 'mesh_health' --include='*.rs' --include='*.py' crates tools 2>/dev/null \
       | /usr/bin/grep -v '^tools/tests/' || true)
unguarded=""
for f in $hits; do
    # a consumer must name the third state somewhere in the same file
    /usr/bin/grep -qiE 'null|unknown|undecided|None' "$ROOT/$f" || unguarded="$unguarded $f"
done
[ -z "$unguarded" ]
judge "every consumer of mesh_health names the unknown: null is the provider's third state, not a pass (issue 53)" $? "unguarded:$unguarded"

fin
