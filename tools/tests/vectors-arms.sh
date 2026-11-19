#!/bin/sh
# Arms for tools/vector-run.py.
#
# WHY THIS FILE EXISTS. Driving the VECTORS proves the data; it does not prove
# the runner. Measured: with the tolerance comparison replaced by `True` and
# the empty check replaced by `True`, all ten vectors still passed. A runner
# that cannot fail is a runner that proves nothing, and nothing here had
# noticed. These arms are the control.
#
# NEEDS OPENSCAD. If it is absent this REFUSES rather than skipping: a suite
# that silently passes when its subject cannot run is the shape of every
# instrument failure in this repo's own corpus.
set -u
ROOT=${VECTOR_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
RUN=${VECTOR_SRC:-$ROOT/tools/vector-run.py}
pass=0; fail=0
ok() { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no() { fail=$((fail+1)); printf '  FAIL %s\n' "$1"; }
fin() { printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail)); [ "$fail" = 0 ] || exit 1; exit 0; }
# ONE LABEL ON BOTH PATHS, or a mutant against the arm MISFIREs instead of
# killing (mutate.sh refusal 4 needs the label to appear in a FAILURE line).
judge() { if [ "$2" = 0 ]; then ok "$1"; else no "$1 -- $3"; fi; }

command -v openscad >/dev/null 2>&1 || {
  no "openscad is not on PATH: these vectors cannot be computed here, which is a PROVISIONING finding and not a pass"
  fin
}

T=$(mktemp -d) || exit 70
trap 'rm -rf "$T"' EXIT HUP INT TERM

# A MINIATURE VECTOR FILE, so a failure names one thing. The closed-form cube
# is the control: if it ever fails, the arms below say nothing about the runner.
cat > "$T/good.toml" <<'EOF'
[[vector]]
id = "cube"
n = 1
question = "volume"
fn = 0
why = "closed form"
model = "cube([10, 10, 10]);"
expected = 1000.0
tolerance = 1e-9

[[vector]]
id = "nothing"
n = 2
question = "volume"
fn = 0
why = "an empty intersection, and empty is a value"
model = "intersection() { cube(5); translate([50, 0, 0]) cube(5); }"
expect_empty = true
EOF
sed 's/expected = 1000.0/expected = 1234.0/' "$T/good.toml" > "$T/wrong.toml"
sed 's/^expect_empty = true/expected = 0.0\ntolerance = 1e-9/' "$T/good.toml" > "$T/notempty.toml"
# A VECTOR THAT CLAIMS EMPTY OVER A MODEL THAT IS NOT. Without this the
# `expect_empty` branch is never driven in its failing direction: replacing its
# comparison with `True` left every other arm green, because the arm above
# converts the vector to a NUMERIC expectation and so takes the other branch.
sed 's/model = "intersection() { cube(5); translate(\[50, 0, 0\]) cube(5); }"/model = "cube(3);"/' \
    "$T/good.toml" > "$T/falsely-empty.toml"

arm() {  # arm <file> <want-rc> <want-substring> <label>
    out=$(python3 "$RUN" "$T/$1" 2>&1); rc=$?
    okk=yes; [ "$rc" = "$2" ] || okk=no
    case "$out" in *"$3"*) ;; *) okk=no ;; esac
    [ "$okk" = yes ] && ok "$4" || no "$4 (rc=$rc, want $2)"
}
arm good.toml     0 "2/2 arms pass"  'the control passes: a closed form and an empty result'
arm wrong.toml    1 "FAIL cube"      'a wrong expectation FAILS, so the comparison is real'
arm notempty.toml 1 "FAIL nothing"   'an empty result asked for 0.0 FAILS rather than passing'
arm falsely-empty.toml 1 "NOT empty"  'a vector claiming empty over a solid FAILS, naming what it found'

# THE STALE-FILE ARM, which is the one this suite was worth writing for.
# A previous run's STL sitting at the output path must NOT be read as this
# run's answer. Corpus mcp-026: four probes reported 52.800 mm3 of material
# that was not there, all four a stale file.
mkdir -p "$T/out"
printf 'solid x\nendsolid x\n' > "$T/stale.scad"
python3 - "$T/out/nothing.stl" <<'PY'
import struct, sys
# a plausible non-empty STL: one triangle, so a reader sees facets and a volume
d = bytearray(b"stale" + b"\0" * 75) + struct.pack("<I", 1)
d += struct.pack("<3f", 0, 0, 1) + struct.pack("<9f", *([0.0, 0.0, 0.0, 1, 0, 0, 0, 1, 0])) + b"\0\0"
open(sys.argv[1], "wb").write(bytes(d))
PY
out=$(WMVEC_OUT="$T/out" python3 "$RUN" "$T/good.toml" 2>&1); rc=$?
case "$out" in
    *"ok   nothing"*) ok 'a stale STL at the output path does not become the answer' ;;
    *) no "a stale STL was read as this run's result: $(printf '%s' "$out" | grep nothing | head -1)" ;;
esac

# -- THE RECORDED PROVENANCE IS READ BACK AGAINST THE LIVE PROVIDER (issue 62).
#    A recorded identity nobody reads back is the defect it was written to
#    close. This arm is here rather than in the t_*.sh glob because it SPEAKS
#    to the provider, which CI's runner reports absent.
VEC="$ROOT/tests/vectors/vectors.toml"
PROV="$ROOT/provider"
if [ -d "$PROV/.venv" ]; then
    # A READ LOOP, NOT A PIPE. Piping three requests and closing stdin is a
    # race: serverInfo came back and tools/list did not, which the arm reported
    # as NO RESULT -- correctly, and it was the arm's own transport at fault,
    # not the provider. Each request now waits for its own reply.
    python3 - "$PROV" "$VEC" >"$T/prov.out" 2>&1 <<'PY'
import json, subprocess, sys, tomllib
prov, vec = sys.argv[1], sys.argv[2]
rec = tomllib.load(open(vec, "rb")).get("provider_provenance") or {}
proc = subprocess.Popen(["uv", "run", "--frozen", "--no-dev", "openscad-mcp"],
                        cwd=prov, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                        stderr=subprocess.DEVNULL, text=True, bufsize=1)


def send(o):
    proc.stdin.write(json.dumps(o) + "\n")
    proc.stdin.flush()


def reply(want):
    while True:
        line = proc.stdout.readline()
        if not line:
            sys.exit(f"the provider closed stdout before answering id {want}: "
                     f"that is NO RESULT, not a match")
        try:
            d = json.loads(line)
        except ValueError:
            continue
        if d.get("id") == want:
            return d


try:
    send({"jsonrpc": "2.0", "id": 1, "method": "initialize",
          "params": {"protocolVersion": "2024-11-05", "capabilities": {},
                     "clientInfo": {"name": "wren", "version": "1"}}})
    info = reply(1)["result"]["serverInfo"]
    send({"jsonrpc": "2.0", "method": "notifications/initialized"})
    send({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})
    tools = [x["name"] for x in reply(2)["result"]["tools"]]
finally:
    proc.terminate()

bad = []
if info.get("name") != rec.get("server_name"):
    bad.append(f"name {info.get('name')!r} vs recorded {rec.get('server_name')!r}")
if info.get("version") != rec.get("server_version"):
    bad.append(f"version {info.get('version')!r} vs recorded {rec.get('server_version')!r}")
if len(tools) != rec.get("tool_count"):
    bad.append(f"{len(tools)} tools vs recorded {rec.get('tool_count')}")
# AND IT MUST BE THE ONE THAT CAN PRODUCE THESE FIGURES. Every recorded
# provider reading is a `measure` result, so a provider without that tool
# cannot have produced one -- which is how the remote was ruled out.
if "measure" not in tools:
    bad.append("no `measure` tool, so it cannot have produced these readings")
if bad:
    sys.exit("; ".join(bad))
print(f"{info['name']} {info['version']}, {len(tools)} tools, measure present")
PY
    judge "the live provider matches the recorded [provider_provenance] -- name, version, tool count, and it HAS the measure tool the figures came from" \
          $? "$(cat "$T/prov.out" 2>/dev/null)"

    # THE KERNEL, and the composition that makes this sound rather than
    # derived: rc 11 refuses a set OPENSCAD_PATH, so the provider runs the
    # binary on PATH, so PATH's version IS the provider's. Stated because
    # "derived instead of read" is the finding this whole axis came from.
    kv=$(timeout 60 openscad --version 2>&1 | head -1)
    want=$(python3 -c "import tomllib,sys;print(tomllib.load(open(sys.argv[1],'rb'))['provider_provenance']['kernel'])" "$VEC")
    ( printf '%s' "$kv" | /usr/bin/grep -q "$want" ) \
      && ( cd "$ROOT" && timeout 600 python3 tools/provider-verify.py 2>&1 | /usr/bin/grep -q "ok   env" )
    judge "PATH's OpenSCAD is the recorded provenance kernel AND rc 11 holds that OPENSCAD_PATH is unset, which is what makes PATH's version the PROVIDER's" \
          $? "saw '$kv', recorded '$want'"
else
    no "provider/.venv is absent: the provenance cannot be read back, which is NO RESULT rather than a match"
fi
# -- THE SECTION PARSER MEETS A REAL REPLY (rook's F11). wire::section_from
#    wanted per-contour objects; the pinned provider sends POINT LISTS with
#    area, centroid and bbox once at the top level, so the parser had NEVER
#    PARSED A REAL ANSWER -- every fixture was hand-built and nothing outside
#    the tests called it. A test cannot close that, because a test runs
#    against a recording and the recording was the thing that was wrong. This
#    drives the pair end to end.
if [ -d "$PROV/.venv" ]; then
    rc=0
    ( cd "$ROOT" && timeout 900 cargo run --offline -q -p wm-provider \
        --example section-from-provider ) >"$T/section.out" 2>&1 || rc=$?
    # the figures are asserted, not just the exit: two contours of 100 mm2 at
    # the two cube centres, four points each, neither a hole, and the parse
    # agreeing with the provider's own 200 mm2 aggregate
    [ "$rc" -eq 0 ] \
      && /usr/bin/grep -q "^contours    2$" "$T/section.out" \
      && /usr/bin/grep -q "area 100.0000 centroid \[5.0000, 5.0000\] vertices Some(4) hole false" "$T/section.out" \
      && /usr/bin/grep -q "area 100.0000 centroid \[25.0000, 5.0000\] vertices Some(4) hole false" "$T/section.out" \
      && /usr/bin/grep -q "self-diff   unchanged=true" "$T/section.out" \
      && /usr/bin/grep -q "area        provider Some(200.0) parsed 200" "$T/section.out" \
      && /usr/bin/grep -q "^concise     REFUSED, correctly$" "$T/section.out" \
      && /usr/bin/grep -q "^detailed    1 contour, Some(100) points" "$T/section.out"
    # WHAT THIS ARM DOES AND DOES NOT REACH, named at rook's request
    # (wm-review/17) so nobody later reads it as "every contour is accounted
    # for". The area agreement is a 1e-3 RELATIVE comparison, so on a 200 mm2
    # section it cannot see a dropped contour below about 0.2 mm2. What DOES
    # account for every contour is exact and lives in the parser:
    # `polygon_count` against the number parsed, and `hole_count` against the
    # number the winding says -- both INTEGERS, both refusing at any size,
    # both armed by wm-model's own tests. This arm proves the pair works end
    # to end on a real reply; it is not the completeness check.
    judge "the section parser reads a REAL provider reply: two contours of 100 mm2 at the cube centres, four points each, no holes, summing to the provider's own 200 mm2 (completeness is the parser's exact polygon_count/hole_count checks, NOT this 1e-3 area agreement), AND the 60-point concise cap refuses while response_format=detailed parses all 100" \
          $? "rc $rc: $(/usr/bin/grep -c . "$T/section.out" 2>/dev/null) lines, $(tail -1 "$T/section.out" 2>/dev/null)"
else
    no "provider/.venv is absent: the section parser cannot be shown to read a real reply, which is NO RESULT rather than a pass"
fi

fin
