#!/bin/sh
# Tier 3: the spec's [exists] badges, checked against a MEASURED provider surface.
#
# docs/spec.md said it of itself: "every [exists] badge was read from the
# provider's own API.md, not run." Fifteen claims about a dependency, taken
# from its documentation and never executed.
#
# THE SPLIT, and why there are two tools. tools/provider-audit.py launches the
# PINNED provider and captures what it has; it needs uv, a resolved lockfile
# and OpenSCAD, so it cannot run in the forge's jail -- the same reason the
# vector arms sit outside this glob. tools/check-provider-claims.py judges the
# claims against that captured surface and needs nothing, so it runs here on
# every push. The audit captures; this judges; the snapshot is the seam.
#
# NARROWING, stated because it is one: a stale snapshot passes this arm. What
# this proves is that the claims agree with the surface AS CAPTURED, and the
# surface records the serverInfo and date it came from. Re-running the audit is
# what makes it current, and the audit REFUSES if the server answering is not
# the pinned one.
set -u
ROOT=${CLAIMS_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
pass=0; fail=0
ok() { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no() { fail=$((fail+1)); printf '  FAIL %s\n' "$1"; }
fin() { printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail)); [ "$fail" = 0 ] || exit 1; exit 0; }

CLAIMS="$ROOT/tests/fixtures/provider-claims.toml"
SURFACE="$ROOT/tests/fixtures/provider-surface.json"
CHECK="$ROOT/tools/check-provider-claims.py"
SPEC="$ROOT/docs/spec.md"
for f in "$CLAIMS" "$SURFACE" "$CHECK" "$SPEC"; do
    [ -r "$f" ] || { no "missing $f"; fin; }
done

OUT=$(python3 "$CHECK" "$CLAIMS" "$SURFACE" 2>&1); rc=$?
if [ "$rc" -eq 0 ]; then
    ok "every claim holds against the captured surface: $(printf '%s\n' "$OUT" | /usr/bin/grep '^-- ')"
else
    no "the provider claims do not hold (rc=$rc):"
    printf '%s\n' "$OUT" | /usr/bin/grep -E '^  (FAIL|UNVERIFIED)' | sed 's/^/         /'
fi

# EVERY BADGE IS ACCOUNTED FOR. The claims file carries spec_line numbers,
# which drift when the spec is edited. Counting badges against claims catches
# a badge ADDED and never claimed -- the silent case, where the tally still
# reads all-pass because the new claim was never in the list.
# SETS, NOT COUNTS. One badge may carry more than one claim -- L436 carries
# the claim and the narrowing found by running it -- so counting them is the
# wrong invariant, and it said so on its first run. What must hold is that
# every badge LINE is claimed and every claim names a line that has a badge.
ACC=$(python3 - "$SPEC" "$CLAIMS" <<'ACCEOF'
import sys, tomllib
spec_lines = {i for i, l in enumerate(open(sys.argv[1], encoding="utf-8"), 1) if "[exists" in l}
claims = tomllib.load(open(sys.argv[2], "rb"))["claim"]
claimed = {c["spec_line"] for c in claims}
if not spec_lines:
    print("BLIND no [exists] badge found in the spec")
elif spec_lines == claimed:
    print(f"OK {len(spec_lines)} badge line(s), {len(claims)} claim(s)")
else:
    print(f"DRIFT unclaimed badge line(s) {sorted(spec_lines - claimed)}; "
          f"claim(s) naming no badge {sorted(claimed - spec_lines)}")
ACCEOF
) || { no "the badge accounting did not run"; fin; }
case "$ACC" in
    OK*) ok "every [exists] badge line is claimed -- ${ACC#OK }" ;;
    *)   no "$ACC" ;;
esac

# THE SURFACE NAMES THE SERVER IT CAME FROM. A snapshot that does not is a
# snapshot of nothing in particular -- which is the exact failure this audit
# was written after: two servers answer to "openscad-mcp" on this machine and
# the wrong one was read for an hour.
pinned_name=$(/usr/bin/sed -n 's/^server_info_name = "\(.*\)"/\1/p' "$ROOT/provider/PROVIDER.toml")
if python3 -c "
import json,sys
s=json.load(open('$SURFACE'))
si=s.get('server_info',{})
sys.exit(0 if si.get('name')=='''$pinned_name''' and si.get('version') else 1)
"; then
    ok "the surface names the pinned server, matching PROVIDER.toml"
else
    no "the surface's server_info does not match PROVIDER.toml's server_info_name"
fi

# AND THE CHECKER'S REFUSALS ARE DRIVEN, because a refusal that has never
# fired is a comment.
FIX=$(mktemp -d) || { no "no fixture dir"; fin; }
trap 'rm -rf "$FIX"' EXIT INT TERM HUP

python3 "$CHECK" "$FIX/absent.toml" "$SURFACE" >/dev/null 2>&1
[ $? -eq 6 ] && ok "a missing claims file is refused as blind (rc 6)" \
             || no "a missing claims file did not refuse with rc 6"

python3 "$CHECK" "$CLAIMS" "$FIX/absent.json" >/dev/null 2>&1
[ $? -eq 6 ] && ok "a missing surface is refused, not reported as 'nothing failed' (rc 6)" \
             || no "a missing surface did not refuse with rc 6"

printf '{"server_info":{"name":"x","version":"1"},"tools":{}}\n' > "$FIX/notools.json"
python3 "$CHECK" "$CLAIMS" "$FIX/notools.json" >/dev/null 2>&1
[ $? -eq 6 ] && ok "a surface with no tools is refused rather than failing every claim" \
             || no "a surface with no tools was not refused"

# A CLAIM THAT CANNOT FAIL IS NOT A CHECK. Strip the mode the spec's central
# claim rests on and the checker must say so.
python3 - "$SURFACE" "$FIX/noprobe.json" <<'PY'
import json, sys
s = json.load(open(sys.argv[1]))
s["tools"]["measure"]["modes"] = [m for m in s["tools"]["measure"]["modes"] if m != "probe"]
json.dump(s, open(sys.argv[2], "w"))
PY
OUT2=$(python3 "$CHECK" "$CLAIMS" "$FIX/noprobe.json" 2>&1); rc2=$?
if [ "$rc2" -ne 0 ] && printf '%s\n' "$OUT2" | /usr/bin/grep -q "lacks mode(s) \['probe'\]"; then
    ok "a provider missing measure(mode=probe) fails the claim that rests on it"
else
    no "removing measure's probe mode did not fail the L297 claim (rc=$rc2)"
fi

# UNKNOWN IS NOT NONE. A tool that did not enumerate its modes must read
# UNVERIFIED, never PASS -- that distinction is the whole reason the audit asks
# the provider instead of parsing its prose.
python3 - "$SURFACE" "$FIX/unknown.json" <<'PY'
import json, sys
s = json.load(open(sys.argv[1]))
s["tools"]["measure"]["modes"] = None
json.dump(s, open(sys.argv[2], "w"))
PY
OUT3=$(python3 "$CHECK" "$CLAIMS" "$FIX/unknown.json" 2>&1)
if printf '%s\n' "$OUT3" | /usr/bin/grep -q '^  UNVERIFIED'; then
    ok "a tool that did not enumerate its modes reads UNVERIFIED, not PASS"
else
    no "unknown modes did not produce UNVERIFIED:"
    printf '%s\n' "$OUT3" | /usr/bin/grep -E 'L297|L276' | sed 's/^/         /'
fi

# THE CONTROL IS LOAD-BEARING. A provider that reported "empty" for everything
# would satisfy the empty-result claim and be worthless.
python3 - "$SURFACE" "$FIX/alwaysempty.json" <<'PY'
import json, sys
s = json.load(open(sys.argv[1]))
s["behaviors"]["nonempty_3d"]["hint_codes"] = ["empty_output"]
json.dump(s, open(sys.argv[2], "w"))
PY
OUT4=$(python3 "$CHECK" "$CLAIMS" "$FIX/alwaysempty.json" 2>&1); rc4=$?
if [ "$rc4" -ne 0 ] && printf '%s\n' "$OUT4" | /usr/bin/grep -q 'the signal means nothing'; then
    ok "a provider that calls everything empty fails the control, not passes the claim"
else
    no "the empty-result control did not bite (rc=$rc4)"
fi
# THE AUDIT'S OWN REFUSALS, as far as they can be driven without a provider.
# The launch needs uv, a lockfile and OpenSCAD and cannot run in the jail; the
# identity comparison was factored out precisely because it is the part that
# was wrong in practice, and a pure function can be driven anywhere.
AUD="$ROOT/tools/provider-audit.py"
python3 "$AUD" "$FIX/o.json" --provider-dir "$FIX" >/dev/null 2>&1
[ $? -eq 3 ] && ok "the audit refuses a directory with no PROVIDER.toml (rc 3)" \
             || no "the audit did not refuse an unidentified provider directory with rc 3"

IDENT=$(python3 - "$AUD" <<'IDEOF'
import importlib.util, sys
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("audit", sys.argv[1])
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
pin = {"server_info_name": "OpenSCAD MCP Server", "server_info_version": "4.2.0"}
same = m.identity_matches(pin, {"name": "OpenSCAD MCP Server", "version": "4.2.0"})
# THE REAL CONFUSION, as it happened: the same family of name, a different
# server, a different version.
other = m.identity_matches(pin, {"name": "openscad-mcp", "version": "1.0.0"})
ver = m.identity_matches(pin, {"name": "OpenSCAD MCP Server", "version": "9.9.9"})
print("SAME", same, "OTHER", other, "VER", ver)
IDEOF
) || { no "the identity probe did not run"; fin; }
case "$IDENT" in
    "SAME True OTHER False VER False")
        ok "the pinned server matches itself, and neither the other openscad-mcp nor a version bump does" ;;
    *)  no "identity_matches is wrong: $IDENT" ;;
esac
fin
