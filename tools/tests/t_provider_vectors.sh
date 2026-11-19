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
[provider_provenance]
server_name = "OpenSCAD MCP Server"
server_version = "4.2.0"
tool_count = 12
kernel = "2026.10.14"
launcher = "uv run --frozen --no-dev openscad-mcp"
measured = "2026-11-16"

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
[provider_provenance]
server_name = "OpenSCAD MCP Server"
server_version = "4.2.0"
tool_count = 12
kernel = "2026.10.14"
launcher = "uv run --frozen --no-dev openscad-mcp"
measured = "2026-11-16"

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
[provider_provenance]
server_name = "OpenSCAD MCP Server"
server_version = "4.2.0"
tool_count = 12
kernel = "2026.10.14"
launcher = "uv run --frozen --no-dev openscad-mcp"
measured = "2026-11-16"

[[vector]]
id = "accepted"
provider = { volume = 1000.0002 }
provider_disagrees = { max_delta = 0.001, cause = "measured", because = "a stated reason" }
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
[provider_provenance]
server_name = "OpenSCAD MCP Server"
server_version = "4.2.0"
tool_count = 12
kernel = "2026.10.14"
launcher = "uv run --frozen --no-dev openscad-mcp"
measured = "2026-11-16"

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
[provider_provenance]
server_name = "OpenSCAD MCP Server"
server_version = "4.2.0"
tool_count = 12
kernel = "2026.10.14"
launcher = "uv run --frozen --no-dev openscad-mcp"
measured = "2026-11-16"

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
[provider_provenance]
server_name = "OpenSCAD MCP Server"
server_version = "4.2.0"
tool_count = 12
kernel = "2026.10.14"
launcher = "uv run --frozen --no-dev openscad-mcp"
measured = "2026-11-16"

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
[provider_provenance]
server_name = "OpenSCAD MCP Server"
server_version = "4.2.0"
tool_count = 12
kernel = "2026.10.14"
launcher = "uv run --frozen --no-dev openscad-mcp"
measured = "2026-11-16"

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
# REWRITTEN 2026-11-19, rook F8 (thread:wm-review/11). The guard was
# file-wide: any file containing `mesh_health` passed if the word
# null|unknown|undecided|None appeared ANYWHERE in it. rook drove it --
# an unguarded `x["mesh_health"]["manifold"] == true` appended to
# crates/wm-provider/src/artifact.rs passed on that file's unrelated `None`s,
# and the only placement that failed was my own control's FRESH file. Most
# Rust and every Python file says None. The two real hits are COMMENTS and
# passed on 16 and 42 unrelated matches, so the tripwire proved nothing.
#
# IT NOW LOOKS AT THE USE SITE. Each occurrence is classified comment or code;
# a comment is not a consumer and is counted, not judged; a CODE occurrence
# must name the third state within WINDOW lines of itself. That is still a
# heuristic and this arm says so -- it is a tripwire with a stated window, not
# a proof -- but it is a claim about the HANDLING rather than about the file.
python3 - "$ROOT" >"$TMP/tripwire.out" 2>&1 <<'PY'
import pathlib, re, sys
root = pathlib.Path(sys.argv[1])
WINDOW = 3
THIRD = re.compile(r"\bnull\b|\bunknown\b|\bundecided\b|\bNone\b", re.I)
comments, unguarded, guarded = [], [], []
for sub in ("crates", "tools"):
    for f in sorted((root / sub).rglob("*")):
        if f.suffix not in (".rs", ".py") or "tools/tests" in f.as_posix():
            continue
        lines = f.read_text(encoding="utf-8", errors="replace").splitlines()
        for n, line in enumerate(lines):
            if "mesh_health" not in line:
                continue
            where = f"{f.relative_to(root)}:{n + 1}"
            bare = line.strip()
            if bare.startswith(("//", "#", "*", "///", "//!")):
                comments.append(where)
                continue
            lo, hi = max(0, n - WINDOW), min(len(lines), n + WINDOW + 1)
            (guarded if THIRD.search("\n".join(lines[lo:hi])) else
             unguarded).append(where)
print(f"comments {len(comments)} code-guarded {len(guarded)} "
      f"code-UNGUARDED {len(unguarded)}")
for w in unguarded:
    print(f"  UNGUARDED {w}")
sys.exit(1 if unguarded else 0)
PY
judge "every CODE reference to mesh_health names the third state within 3 lines of itself: null is the provider's third state, not a pass (issue 53, rook F8)" \
      $? "$(head -3 "$TMP/tripwire.out" | tr '\n' ' ')"

# AND THE TRIPWIRE ITSELF IS DRIVEN, in rook's own placement -- appended to a
# REAL file full of unrelated `None`s, which is the case the old guard passed.
#
# WMVEC_NO_PLANT GUARDS A RECURSION I WROTE AND IMMEDIATELY HIT: this arm runs
# this same file to see the tripwire fire, and without the guard that inner run
# plants and recurses forever. It also left the plant in artifact.rs when I
# killed it, which is the worse half -- a self-driving arm must restore the
# tree on EVERY exit, so the trap below is set before the file is touched.
victim="$ROOT/crates/wm-provider/src/artifact.rs"
if [ -n "${WMVEC_NO_PLANT:-}" ]; then
    :   # inner run: the outer one is driving the plant, do not recurse
elif [ -r "$victim" ]; then
    cp "$victim" "$TMP/victim.bak"
    # RESTORE ON ANY EXIT, including a kill: an arm that edits a tracked file
    # and dies leaves a planted defect behind, which is what happened once.
    trap 'cp "$TMP/victim.bak" "$victim" 2>/dev/null' EXIT HUP INT TERM
    printf '\n// planted by t_provider_vectors.sh; removed below\nfn _f(x: &Value) -> bool { x["mesh_health"]["manifold"] == true }\n' >> "$victim"
    planted=$( ( cd "$ROOT" && WMVEC_NO_PLANT=1 timeout 180 sh tools/tests/t_provider_vectors.sh 2>&1 | /usr/bin/grep -c "UNGUARDED crates/wm-provider/src/artifact" ) || true )
    cp "$TMP/victim.bak" "$victim"
    trap - EXIT HUP INT TERM
    [ "${planted:-0}" -ge 1 ]
    judge "an unguarded mesh_health read planted in a REAL file full of unrelated Nones is CAUGHT -- the old guard passed exactly this (rook F8)" \
          $? "saw $planted UNGUARDED lines naming artifact.rs"
else
    no "crates/wm-provider/src/artifact.rs is absent: rook's placement cannot be driven, which is NO RESULT rather than a pass"
fi
# -- A DECLARED CAUSE MUST COME FROM A CLOSED SET (issue 62). The one
#    `provider_disagrees` here attributed a gap to two kernels tessellating
#    differently and cited as evidence a note recording the exported bytes as
#    IDENTICAL across those kernels. The citation existed and meant the
#    opposite; nothing checked it, because free text cannot be wrong. Both arms
#    below perturb the REAL fixture, and both assert the perturbation took --
#    an edit that matched nothing would leave a clean file reading as a pass.
for case in missing invented; do
    python3 - "$TMP" "$case" "$VEC" <<'PY' || { no "world $case could NOT be built: the perturbation matched nothing, so there was no defect to refuse"; continue; }
import sys, pathlib
tmp, case, src = sys.argv[1], sys.argv[2], pathlib.Path(sys.argv[3])
t = src.read_text()
# THE FIELD, not the text: the comment above it quotes `cause = "unknown"` as
# prose, and anchoring on the bare literal matched twice. The assertion caught
# that rather than perturbing the wrong occurrence.
want = ', cause = "unknown", because'
assert t.count(want) == 1, f"{t.count(want)} cause FIELDS, expected 1"
t = t.replace(want, ", because" if case == "missing" else ', cause = "obvious", because')
pathlib.Path(tmp, f"v-{case}.toml").write_text(t)
PY
    got=$( ( cd "$ROOT" && timeout 300 python3 "$RUN" "$TMP/v-$case.toml" --provider-only >"$TMP/out" 2>&1 ); echo $? )
    [ "$got" != 0 ] && /usr/bin/grep -q "is not one of" "$TMP/out"
    judge "a provider_disagrees whose cause is $case refuses: a free-text reason cannot be wrong, which is how a REFUTED explanation survived here" \
          $? "rc $got: $(/usr/bin/grep -c 'is not one of' "$TMP/out" 2>/dev/null) matching lines"
done

# -- AND THE SHIPPED FIXTURE DOES NOT CLAIM A CAUSE IT HAS NOT MEASURED. This
#    is the specific contradiction of issue 62, mechanized: no `because` may
#    attribute a disagreement to the two kernels differing while this same file
#    records their exported bytes as identical.
if /usr/bin/grep -q "the exported bytes are IDENTICAL" "$VEC"; then
    /usr/bin/grep -o 'because = "[^"]*"' "$VEC" | /usr/bin/grep -qiE "tessellate the same model|different tessellation"
    [ $? -ne 0 ]
    judge "no accepted disagreement blames different tessellation while this file records the exported bytes as IDENTICAL across those kernels (issue 62)" $? \
          "$(/usr/bin/grep -o 'because = \"[^\"]*\"' "$VEC" | /usr/bin/grep -icE 'tessellate the same model|different tessellation') such claims"
else
    no "the identical-bytes measurement is gone from the fixture: this arm's premise is absent, which is NO RESULT rather than a pass"
fi

# -- F7(a): THE MAGNITUDE USES THE SAME RULE AS THE VERDICT. Agreement is
#    containment in the interval a reported figure stands for; the SIZE of a
#    disagreement was |ours - the POINT|, which is the question issue 54 calls
#    wrong. Driven as arithmetic, not through the fixture, so the distinguishing
#    case is explicit: a value INSIDE the interval must measure zero, and one
#    outside must measure the distance to the EDGE, never to the centre.
out=$(cd "$ROOT" && python3 - <<'PY' 2>&1
import importlib.util, sys
sys.dont_write_bytecode = True
s = importlib.util.spec_from_file_location("vr", "tools/vector-run.py")
m = importlib.util.module_from_spec(s); s.loader.exec_module(m)
inside = m.interval_distance(4171.99880, 4171.9988)
edge = m.interval_distance(4171.998709, 4171.9988)
point = abs(4171.9988 - 4171.998709)
print("INSIDE", inside)
print("EDGE", edge)
print("POINT", round(point, 6))
PY
)
printf '%s' "$out" | /usr/bin/grep -q "^INSIDE 0$" \
  && printf '%s' "$out" | /usr/bin/grep -q "^EDGE 0.000041$" \
  && printf '%s' "$out" | /usr/bin/grep -q "^POINT 9.1e-05$"
judge "a disagreement is measured to the EDGE of the reported interval (4.1e-5), not to its point (9.1e-5), and a contained value measures ZERO -- F7(a)" \
      $? "$(printf '%s' "$out" | tr '\n' ' ')"

# -- F7(b): AN ACCEPTANCE AT THE VECTOR'S OWN TOLERANCE IS A WAIVER. The
#    shipped fixture had max_delta == tolerance == 0.0001, the exact width
#    issue 54 says would have reported the disagreement clean.
python3 - "$TMP" "$VEC" <<'PY' || { no "world waiver could NOT be built: the perturbation matched nothing"; }
import sys, pathlib, re
tmp, src = sys.argv[1], pathlib.Path(sys.argv[2])
t = src.read_text()
m = re.search(r'max_delta = ([0-9.]+)', t)
assert m, "no max_delta to widen"
tol = re.search(r'^tolerance = ([0-9.]+)', t[m.end():], re.M)
assert tol, "no tolerance after it"
t = t[:m.start()] + f"max_delta = {tol.group(1)}" + t[m.end():]
pathlib.Path(tmp, "waiver.toml").write_text(t)
PY
got=$( ( cd "$ROOT" && timeout 300 python3 "$RUN" "$TMP/waiver.toml" --provider-only >"$TMP/out" 2>&1 ); echo $? )
[ "$got" != 0 ] && /usr/bin/grep -q "is a WAIVER, not a bound" "$TMP/out"
judge "an accepted max_delta equal to the vector's own tolerance REFUSES: a bound at the old width re-admits what the tighter rule was built to surface -- F7(b)" \
      $? "rc $got"

# -- AND A PROVIDER READING WITH NO NAMED INSTRUMENT REFUSES (issue 62). The
#    seven worlds above now carry [provider_provenance] because this rule made
#    them fail, which is the rule working; this arm drives it ON PURPOSE so it
#    is not merely a side effect of the others.
cat > "$TMP/unattributed.toml" <<'EOF'
[[vector]]
id = "unattributed"
provider = { volume = 1000.0 }
question = "volume"
model = "cube([10,10,10]);"
expected = 1000.0
tolerance = 0.01
EOF
got=$(rc_of "$TMP/unattributed.toml")
[ "$got" != 0 ] && /usr/bin/grep -q "no \[provider_provenance\]" "$TMP/out"
judge "a recorded provider reading with no [provider_provenance] refuses: a figure from an unnamed instrument cannot be checked against anything" \
      $? "rc $got"

# and a PARTIAL identity must not read as a complete one
sed '/^kernel = /d' "$TMP/unattributed.toml" > "$TMP/x.toml"
python3 - "$TMP" <<'PY' || { no "world partial could NOT be built"; }
import sys, pathlib
tmp = pathlib.Path(sys.argv[1])
t = (tmp / "accepted.toml").read_text()
assert "kernel = " in t, "no kernel line to drop"
(tmp / "partial.toml").write_text(t.replace('kernel = "2026.10.14"\n', ""))
PY
got=$(rc_of "$TMP/partial.toml")
[ "$got" != 0 ] && /usr/bin/grep -q "missing \['kernel'\]" "$TMP/out"
judge "a [provider_provenance] missing a field refuses: a partial identity reads as a complete one" $? "rc $got"

fin
