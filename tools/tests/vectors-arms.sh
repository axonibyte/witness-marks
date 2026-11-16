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
fin
