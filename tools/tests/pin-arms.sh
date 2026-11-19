#!/bin/sh
# Arms for tools/provider-verify.py. NOT in the t_*.sh glob, DELIBERATELY.
#
# These reach pypi.org and github.com. tools/tests/t_*.sh is what CI runs, and
# an arm that needs the network would make CI red or flaky for a reason that
# has nothing to do with the code -- the same constraint vectors-arms.sh lives
# under, for the same reason. Run this deliberately, like the vectors.
#
# WHAT STAYS IN THE CI SUITE: t_provider_claims.sh asserts the committed
# surface was captured against a VERIFIED pin. That reads a file and needs no
# network, and it is the arm that makes the verdict load-bearing rather than
# decorative -- which is the whole point of issue 57.
#
#     sh tools/tests/pin-arms.sh
set -u
ROOT=${PIN_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
pass=0; fail=0
ok() { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no() { fail=$((fail+1)); printf '  FAIL %s\n' "$1"; }
fin() { printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail)); [ "$fail" = 0 ] || exit 1; exit 0; }
# ONE LABEL PER CHECK, ON BOTH PATHS. mutate.sh's refusal 4 requires the
# expected label to appear IN A FAILURE LINE, so an arm whose failure text
# differs from its success text can never satisfy it -- and a mutant against
# such an arm MISFIRES rather than killing. This file had no such helper, which
# is what invited the divergence; three arms added on 2026-11-18 diverged
# immediately. judge <label> <rc> <detail>.
judge() { if [ "$2" = 0 ]; then ok "$1"; else no "$1" "$3"; fi; }

VER="$ROOT/tools/provider-verify.py"
[ -r "$VER" ] || { no "tools/provider-verify.py is missing: the pin cannot be verified at all"; fin; }

# THE PIN AS IT STANDS MUST VERIFY. If this fails, nothing below means much.
if python3 "$VER" --provider-dir "$ROOT/provider" >/dev/null 2>&1; then
    ok "the recorded pin verifies against pypi and upstream"
else
    rc=$?
    if [ "$rc" -eq 3 ]; then
        no "a registry was unreachable (rc 3), so NOTHING here was verified -- a PROVISIONING finding, not a pass"
        fin
    fi
    no "the recorded pin does NOT verify (rc $rc); run tools/provider-verify.py and read it"
fi

VER="$ROOT/tools/provider-verify.py"
# ITS OWN TEMP DIR, not $FIX: this block runs before the fixture dir below is
# created, and borrowing a variable declared later is how a shell test comes to
# depend on its own line order.
PINFIX=$(mktemp -d) || { no "no temp dir for the pin arms"; fin; }
trap 'rm -rf "$PINFIX"' EXIT INT TERM HUP
if [ -r "$VER" ]; then
    mkdir -p "$PINFIX/nopin"
    printf '[provider]\nname = "openscad-mcp"\n' > "$PINFIX/nopin/PROVIDER.toml"
    python3 "$VER" --provider-dir "$PINFIX/nopin" >/dev/null 2>&1
    [ $? -eq 6 ] && ok "a PROVIDER.toml missing its pins is refused as blind (rc 6)" \
                 || no "a PROVIDER.toml with no pins did not refuse with rc 6"

    mkdir -p "$PINFIX/badsha"
    /usr/bin/sed 's/^pypi_sdist_sha256 = ".*"/pypi_sdist_sha256 = "0000000000000000000000000000000000000000000000000000000000000000"/' \
        "$ROOT/provider/PROVIDER.toml" > "$PINFIX/badsha/PROVIDER.toml"
    out=$(python3 "$VER" --provider-dir "$PINFIX/badsha" 2>&1); rc=$?
    if [ "$rc" -eq 4 ]; then
        ok "a wrong sdist hash is refused with its own code (rc 4)"
    elif [ "$rc" -eq 3 ]; then
        no "the registry was unreachable, so the sdist refusal could not be driven -- a PROVISIONING finding, not a pass"
    else
        no "a wrong sdist hash did not refuse with rc 4 (got $rc)"
    fi

    mkdir -p "$PINFIX/badtag"
    /usr/bin/sed 's/^git_tag_sha = ".*"/git_tag_sha = "0000000000000000000000000000000000000000"/' \
        "$ROOT/provider/PROVIDER.toml" > "$PINFIX/badtag/PROVIDER.toml"
    out=$(python3 "$VER" --provider-dir "$PINFIX/badtag" 2>&1); rc=$?
    if [ "$rc" -eq 5 ]; then
        ok "a wrong tag commit is refused with its own code (rc 5)"
    elif [ "$rc" -eq 3 ]; then
        no "upstream was unreachable, so the tag refusal could not be driven -- a PROVISIONING finding, not a pass"
    else
        no "a wrong tag commit did not refuse with rc 5 (got $rc)"
    fi
else
    no "tools/provider-verify.py is missing: the pin cannot be verified at all"
fi

# THE PEEL, tested against an ANNOTATED tag rather than the one we pin.
#
# Our pin is v0.6.1, which is LIGHTWEIGHT: its ref IS the commit, so a verifier
# that ignored the peel would be right by luck on it and wrong on its
# neighbours. v0.6.0 is annotated -- ref bf08011, commit 93d04da -- which is
# the case coop/bonemesh.sh names ("the peel matters, and wren nearly reported
# the tag object instead"). PyPI has no sdist for 0.6.0, so the whole pin
# cannot be fixtured there; the peel function is called directly instead.
PEEL=$(python3 - "$VER" <<'PEELEOF'
import importlib.util, sys
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("pv", sys.argv[1])
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
up = "https://github.com/RobertCoop/openscad-mcp.git"
annotated, err_a = m.tag_commit(up, "v0.6.0")
light, err_l = m.tag_commit(up, "v0.6.1")
if annotated is None or light is None:
    print("UNREACHABLE", err_a or err_l)
else:
    print("ANNOTATED", annotated, "LIGHT", light)
PEELEOF
) || { no "the peel probe did not run"; fin; }
case "$PEEL" in
    "ANNOTATED 93d04dadecd5e5e7136d489ae968948ecb7230ba LIGHT 258c8a4553b62d0d1df1a37aa54c218ef3de5689")
        ok "an annotated tag resolves to its PEELED commit, not the tag object" ;;
    "ANNOTATED bf08011009735117f135c088eb0d07a0d758effd"*)
        no "the annotated tag resolved to the TAG OBJECT bf08011, not the commit 93d04da -- the peel is being ignored" ;;
    UNREACHABLE*)
        no "upstream was unreachable, so the peel could not be driven -- a PROVISIONING finding, not a pass: $PEEL" ;;
    *)  no "unexpected peel result: $PEEL" ;;
esac

# THE KERNEL REFUSAL, which issue 56 asked for: a measurement on a different
# kernel is a different measurement, so a mismatch refuses rather than warns.
mkdir -p "$PINFIX/badkernel"
/usr/bin/sed 's/^kernel_version = ".*"/kernel_version = "2021.01"/' \
    "$ROOT/provider/PROVIDER.toml" > "$PINFIX/badkernel/PROVIDER.toml"
python3 "$VER" --provider-dir "$PINFIX/badkernel" >/dev/null 2>&1
rc=$?
if [ "$rc" -eq 7 ]; then
    ok "a kernel differing from the recorded one is refused with its own code (rc 7)"
elif [ "$rc" -eq 3 ]; then
    no "a registry was unreachable, so the kernel refusal could not be driven -- a PROVISIONING finding, not a pass"
else
    no "a wrong kernel did not refuse with rc 7 (got $rc)"
fi

# THE AUDIT MUST RECORD THE VERIFIER'S OWN VERDICT, not a literal.
#
# This arm exists because I first declared it uncoverable. The arm in the CI
# suite reads the COMMITTED surface, so mutating the audit changes nothing it
# can see -- true, and the wrong conclusion: a DELIBERATE arm can re-capture
# and compare. `pin-verdict-not-recorded` hardcodes {"verified": true, "why":
# "assumed"}, which is invisible to a reader of the old file and plain to one
# that runs the audit now.
AUD="$ROOT/tools/provider-audit.py"
if [ -r "$AUD" ]; then
    fresh="$PINFIX/fresh-surface.json"
    if python3 "$AUD" "$fresh" --provider-dir "$ROOT/provider" >/dev/null 2>&1; then
        want=$(python3 "$VER" --provider-dir "$ROOT/provider" --json 2>/dev/null \
               | python3 -c "import json,sys; print(json.load(sys.stdin)['checks'][0]['ok'])" 2>/dev/null)
        got=$(python3 -c "
import json
s = json.load(open('$fresh'))
p = s.get('pin') or {}
print(p.get('verified'), repr(p.get('why','')))
" 2>/dev/null)
        case "$got" in
            "True 'pypi sdist, tag peel and kernel all match'")
                ok "a freshly captured surface records the verifier's own verdict" ;;
            *)  no "the captured surface's pin verdict is not the verifier's: $got" ;;
        esac
        # AND THE WHOLE PIN, not a subset. A surface recording only the version
        # cannot be checked against a re-pin of the kernel, the backend or
        # either hash -- which is issue 50, and was the state of this file
        # until 2026-11-17.
        nfields=$(python3 -c "
import json
print(len((json.load(open('$fresh')).get('pinned') or {})))
" 2>/dev/null)
        if [ "${nfields:-0}" -ge 8 ]; then
            ok "a freshly captured surface records the whole pin ($nfields fields)"
        else
            no "a freshly captured surface records fewer pin fields than the pin has (${nfields:-0} of 8), so a re-pin could not be caught as drift"
        fi
    else
        no "the audit could not capture a surface, so the verdict could not be checked -- a PROVISIONING finding, not a pass"
    fi
else
    no "tools/provider-audit.py is missing"
fi

# THE BACKEND REFUSAL. A separate axis from the version, and the one that
# decides whether the CGAL banner exists -- which is issue 53.
mkdir -p "$PINFIX/badbackend"
/usr/bin/sed 's/^kernel_backend = ".*"/kernel_backend = "CGAL"/' \
    "$ROOT/provider/PROVIDER.toml" > "$PINFIX/badbackend/PROVIDER.toml"
python3 "$VER" --provider-dir "$PINFIX/badbackend" >/dev/null 2>&1
rc=$?
if [ "$rc" -eq 8 ]; then
    ok "a backend differing from the recorded one is refused with its own code (rc 8)"
elif [ "$rc" -eq 3 ]; then
    no "a registry was unreachable, so the backend refusal could not be driven -- a PROVISIONING finding, not a pass"
else
    no "a wrong backend did not refuse with rc 8 (got $rc)"
fi

# AND THE BACKEND ACTUALLY DECIDES THE BANNER, measured rather than asserted.
# This is issue 53's mechanism: the provider parses CGAL statistics, and only
# the CGAL backend prints them.
BK=$(python3 - <<'BKEOF'
import subprocess, tempfile, pathlib, re
src = "difference(){ cube([10,10,10]); translate([5,5,-1]) cylinder(h=12,r=3,$fn=32); }"
out = {}
with tempfile.TemporaryDirectory() as d:
    scad = pathlib.Path(d) / "p.scad"
    scad.write_text(src)
    for flag, name in (("--backend=CGAL", "cgal"), ("--backend=Manifold", "manifold")):
        p = subprocess.run(["openscad", flag, "-o", str(pathlib.Path(d) / "p.stl"), str(scad)],
                           capture_output=True, text=True, timeout=300)
        text = (p.stdout or "") + (p.stderr or "")
        out[name] = bool(re.search(r"^\s*Simple:", text, re.M))
print("CGAL", out["cgal"], "MANIFOLD", out["manifold"])
BKEOF
) || { no "the banner probe did not run"; fin; }
case "$BK" in
    "CGAL True MANIFOLD False")
        ok "only the CGAL backend prints the Simple:/Volumes: banner (issue 53's mechanism)" ;;
    *)  no "the backend/banner relationship has changed: $BK" ;;
esac

# ---------------------------------------------------------------------------
# THE AXES ISSUE 58 ENUMERATED. Each refusal sits BEHIND the pypi, tag, kernel
# and backend checks, so each arm below is an end-to-end run of the verifier
# with one axis perturbed -- which is why they are here and not in the glob.
# The rc is the assertion: the codes are distinct because they mean different
# things, and an arm that accepted any non-zero rc would pass on the wrong
# refusal.
PROV="$ROOT/provider"
rc_of() {  # rc_of <provider-dir> [VAR=VAL ...] -> the verifier's rc
    # `env` RATHER THAN A PREFIX ASSIGNMENT, and the difference is not
    # cosmetic: `VAR=x some_function` sets a SHELL variable, which an exec'd
    # python3 never sees. Measured -- the first three arms here came back rc 0,
    # the verifier having run in an environment with none of the knobs set, and
    # a rc-0 arm asserting a refusal reads exactly like a missing refusal.
    _d=$1; shift
    ( cd "$ROOT" && env "$@" python3 "$VER" --provider-dir "$_d" >/dev/null 2>&1 ); echo $?
}

# -- rc 11: a knob that defeats a pin above it
got=$(rc_of "$PROV" MCP_HARD_WARNINGS=true)
[ "$got" = 11 ] && ok "a knob set against the record refuses with rc 11 (MCP_HARD_WARNINGS)" \
                || no "MCP_HARD_WARNINGS=true gave rc $got, wanted 11"

# -- rc 11 again, and this one is the reason the axis exists: OPENSCAD_PATH
#    chooses the binary, so the kernel pin would otherwise verify a binary that
#    is not the one the provider runs.
got=$(rc_of "$PROV" OPENSCAD_PATH=/nonexistent/openscad)
[ "$got" = 11 ] && ok "OPENSCAD_PATH set refuses: the kernel pin cannot speak for a binary it did not run" \
                || no "OPENSCAD_PATH gave rc $got, wanted 11"

# -- rc 10: a library at a version the record does not name
LIBDIR=$(mktemp -d) || { no "mktemp failed"; fin; }
mkdir -p "$LIBDIR/BOSL2"
printf 'BOSL_VERSION = [9,9,999];\n' > "$LIBDIR/BOSL2/version.scad"
got=$(rc_of "$PROV" OPENSCADPATH="$LIBDIR")
# OPENSCADPATH is itself recorded as must-be-unset, and the library check runs
# FIRST, so a wrong library version is what this sees. Both refusals are
# correct; the arm asserts the EARLIER one, which is the more specific.
[ "$got" = 10 ] && ok "a library at an unrecorded version refuses with rc 10 (BOSL2 9.9.999)" \
                || no "a wrong BOSL2 version gave rc $got, wanted 10"
rm -rf "$LIBDIR"

# -- rc 12: a .env reachable from the working directory
ENVDIR=$(mktemp -d) || { no "mktemp failed"; fin; }
cp "$PROV/PROVIDER.toml" "$ENVDIR/PROVIDER.toml"
printf 'MCP_HARD_WARNINGS=true\n' > "$ENVDIR/.env"
got=$( ( cd "$ENVDIR" && python3 "$VER" --provider-dir "$ENVDIR" >/dev/null 2>&1 ); echo $? )
[ "$got" = 12 ] && ok "a .env reachable from the working directory refuses with rc 12" \
                || no "a reachable .env gave rc $got, wanted 12"
rm -rf "$ENVDIR"

# -- rc 9: the STL flavor. THE RECORD IS PERTURBED HERE, NOT THE WORLD, because
#    OpenSCAD's default is what it is and the arm must not depend on being able
#    to change it. Recording binstl while the kernel writes ASCII is exactly
#    the state the day the announced default flip lands, which is the day this
#    refusal is for.
FMTDIR=$(mktemp -d) || { no "mktemp failed"; fin; }
sed 's/^kernel_export_format_stl = .*/kernel_export_format_stl = "binstl"/' \
    "$PROV/PROVIDER.toml" > "$FMTDIR/PROVIDER.toml"
if /usr/bin/grep -q 'kernel_export_format_stl = "binstl"' "$FMTDIR/PROVIDER.toml"; then
    got=$(rc_of "$FMTDIR")
    [ "$got" = 9 ] && ok "a recorded STL flavor the kernel does not write refuses with rc 9" \
                   || no "a binstl record gave rc $got, wanted 9"
else
    no "the STL flavor arm could not perturb the record, so it proved nothing"
fi
rm -rf "$FMTDIR"

# -- AND THE CLEAN RUN, which is the arm that keeps the four above honest: a
#    verifier that refused unconditionally would pass every one of them.
got=$(rc_of "$PROV")
[ "$got" = 0 ] && ok "and the recorded pin verifies clean, so the refusals above are not unconditional" \
               || no "the clean run gave rc $got, wanted 0"

# ---------------------------------------------------------------------------
# THE TWELVE GUARDS, AGAINST THE LIVE MODEL (issue 36). Deliberate, because
# scad_eval needs the pinned provider, uv and OpenSCAD. The structural half is
# in the CI glob as t_guards.sh.
GUARDS="$ROOT/tools/guards.py"
if [ -r "$GUARDS" ]; then
    got=$( ( cd "$ROOT" && timeout 900 python3 "$GUARDS" >"$ROOT/.guards.out" 2>&1 ); echo $? )
    held=$(/usr/bin/sed -n 's/^-- \([0-9]*\)\/\([0-9]*\) guards hold.*/\1 \2/p' "$ROOT/.guards.out")
    [ "$got" = 0 ] && [ "$held" = "12 12" ]
    judge "all twelve guards hold against the live can_cutter.scad" $? "rc $got, held '$held'"

    # AND THEY BITE WHERE THE DESIGN ACTUALLY MOVED. `wall` went 4.0 -> 5.0 in
    # the harness and left two derived numbers stale in its prose; these are
    # the two guards that cover them, so moving it again must trip exactly
    # those. A guard that only ever passes has proved nothing.
    got=$( ( cd "$ROOT" && timeout 900 python3 "$GUARDS" --set wall=10 >"$ROOT/.guards.out" 2>&1 ); echo $? )
    tripped=$(/usr/bin/grep -c '^  OUT OF RANGE' "$ROOT/.guards.out" || true)
    [ "$got" = 4 ] && [ "$tripped" = 2 ] \
       && /usr/bin/grep -q 'material_left_under_the_thread_groove' "$ROOT/.guards.out" \
       && /usr/bin/grep -q 'body_wall_is_thick_enough_to_hold_a_groove' "$ROOT/.guards.out"
    judge "moving \`wall\` trips exactly the two guards whose numbers its last change staled" \
          $? "rc $got, $tripped tripped"

    # A MODEL WITH NONE OF THE SYMBOLS READS AS UNEVALUABLE, NOT AS PASSING.
    # `undef` is OpenSCAD's absent value and as a 0.0 it would pass every range
    # with a lower bound of zero.
    printf 'x = 1;\n' > "$ROOT/.guards-empty.scad"
    sed "s|^model = .*|model = \"$ROOT/.guards-empty.scad\"|" "$ROOT/tests/claims/can-cutter.toml" > "$ROOT/.guards-nosym.toml"
    got=$( ( cd "$ROOT" && timeout 900 python3 "$GUARDS" --file .guards-nosym.toml >"$ROOT/.guards.out" 2>&1 ); echo $? )
    un=$(/usr/bin/grep -c '^  UNEVALUABLE' "$ROOT/.guards.out" || true)
    [ "$got" = 3 ] && [ "$un" = 12 ]
    judge "a model with none of the symbols reads UNEVALUABLE for all twelve, never as a pass" \
          $? "rc $got, $un unevaluable"
    rm -f "$ROOT/.guards.out" "$ROOT/.guards-empty.scad" "$ROOT/.guards-nosym.toml"
else
    no "tools/guards.py is missing" ""
fi

# -- THE DOTENV AXIS ON THIS BOX, which is a measurement of the box and not of
#    the code. t_pinning_axes.sh arms the code's choice of tree on synthetic
#    roots -- that holds at any root, which is why it belongs in the CI glob.
#    THIS arm needs provider/.venv, which is gitignored and absent from a
#    clone, so it belongs here with the network arms. Found by the foreign-root
#    gate: the first version of those arms read the real venv and would have
#    gone red in CI for a reason that has nothing to do with the code.
if [ -d "$ROOT/provider/.venv" ]; then
    got=$( ( cd "$ROOT" && timeout 120 python3 - <<'PY'
import importlib.util, sys
from pathlib import Path
s = importlib.util.spec_from_file_location("pv", "tools/provider-verify.py")
m = importlib.util.module_from_spec(s); sys.dont_write_bytecode = True
s.loader.exec_module(m)
ok, detail = m.dotenv_verdict(Path("provider"))
print(detail)
sys.exit(0 if ok else 1)
PY
    ) >"$ROOT/.dotenv.out" 2>&1; echo $? )
    [ "$got" = 0 ] && /usr/bin/grep -q "openscad_mcp" "$ROOT/.dotenv.out"
    judge "no .env is reachable from THIS box's installed openscad_mcp/utils, the tree python-dotenv actually starts from" \
          $? "rc $got: $(cat "$ROOT/.dotenv.out" 2>/dev/null)"
    rm -f "$ROOT/.dotenv.out"
else
    no "provider/.venv is absent: the dotenv axis on this box is NO RESULT, not clean"
fi

# -- THE GUARD CITATIONS AGAINST CAL'S ACTUAL DESIGN FILE. This is the
#    machine-dependent half of rook's F15: t_guards.sh proves the citation
#    LOGIC on synthetic models, because that holds anywhere; whether the real
#    file still contains the quoted text can only be asked where the file is.
#    It is NOT in the t_*.sh glob for exactly that reason -- the
#    foreign-machine gate reported it when it briefly was.
MODEL=$(python3 -c "import tomllib,sys;print(tomllib.load(open(sys.argv[1],'rb'))['model'])" \
        "$ROOT/tests/claims/can-cutter.toml" 2>/dev/null || true)
if [ -n "${MODEL:-}" ] && [ -r "$MODEL" ]; then
    got=$( ( cd "$ROOT" && timeout 300 python3 tools/guards.py --provenance-only \
             >"$ROOT/.prov.out" 2>&1 ); echo $? )
    [ "$got" = 0 ] \
      && /usr/bin/grep -q "quoted outright and verified" "$ROOT/.prov.out" \
      && /usr/bin/grep -q "chosen-but-side-sourced" "$ROOT/.prov.out"
    judge "every guard citation resolves against cal's REAL design file -- the four sourced quotes and the four rejections, checked where the file actually is" \
          $? "rc $got: $(/usr/bin/grep -c . "$ROOT/.prov.out" 2>/dev/null) lines"
    rm -f "$ROOT/.prov.out"
else
    no "the model named by can-cutter.toml is not on this machine, so the citations cannot be resolved: NO RESULT, not agreement"
fi

fin
