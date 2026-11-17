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

fin
