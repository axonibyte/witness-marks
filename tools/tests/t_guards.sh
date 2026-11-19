#!/bin/sh
# Tier 3: the twelve guards' own structure and tools/guards.py's refusals,
# without the provider (issue 36).
#
# THE SPLIT. Evaluating a guard needs scad_eval, which needs the pinned
# provider, uv and OpenSCAD -- so the live run is deliberate, like
# vectors-arms.sh. What runs here is everything decided BEFORE a word reaches
# the provider: that the file declares twelve guards, that each carries a
# reason, that no range brackets nothing, and that a run with nothing to check
# REFUSES instead of printing no failures.
#
# WHY "every guard has a because" IS AN ARM AND NOT A CONVENTION. A range
# without a reason cannot be re-judged when the design changes, so it becomes
# folklore the moment its author moves on -- which is the failure these guards
# exist to replace, arriving in the guards themselves.
set -u
ROOT=${GUARDS_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
pass=0; fail=0
ok() { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no() { fail=$((fail+1)); printf '  FAIL %s -- %s\n' "$1" "${2:-}"; }
judge() { if [ "$2" = 0 ]; then ok "$1"; else no "$1" "$3"; fi; }
fin() { printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail)); [ "$fail" = 0 ] || exit 1; exit 0; }

TOOL="$ROOT/tools/guards.py"
FILE="$ROOT/tests/claims/can-cutter.toml"
[ -r "$TOOL" ] || { no "tools/guards.py is missing" ""; fin; }
[ -r "$FILE" ] || { no "tests/claims/can-cutter.toml is missing" ""; fin; }
TMP=$(mktemp -d) || { no "mktemp failed" ""; fin; }
trap 'rm -rf "$TMP"' EXIT

# -- STRUCTURE, read out of the file rather than counted here.
OUT=$(python3 - "$FILE" <<'PY'
import sys, tomllib
d = tomllib.load(open(sys.argv[1], "rb"))
g = d.get("guard") or []
print("COUNT", len(g))
print("MODEL", d.get("model", ""))
for x in g:
    miss = [k for k in ("name", "source", "range", "unit", "because") if not x.get(k)]
    if miss:
        print("INCOMPLETE", x.get("name", "?"), ",".join(miss))
    if len((x.get("because") or "").strip()) < 40:
        print("THIN", x.get("name", "?"))
    lo, hi = (x.get("range") or [0, 0])[:2]
    if not lo < hi:
        print("BRACKETS_NOTHING", x.get("name", "?"))
PY
) || { no "the structure read did not run" "$OUT"; fin; }

n=$(printf '%s\n' "$OUT" | /usr/bin/sed -n 's/^COUNT //p')
[ "$n" = 12 ]
judge "the file declares TWELVE guards, which is what spec §06.1 names" $? "declares $n"

printf '%s\n' "$OUT" | /usr/bin/grep -q '^INCOMPLETE' && \
  no "a guard is missing a field" "$(printf '%s\n' "$OUT" | /usr/bin/grep '^INCOMPLETE' | head -2)" || \
  ok "every guard carries a name, an expression, a range, a unit and a reason"

printf '%s\n' "$OUT" | /usr/bin/grep -q '^THIN' && \
  no "a guard's reason is too thin to re-judge the range by" "$(printf '%s\n' "$OUT" | /usr/bin/grep '^THIN' | head -2)" || \
  ok "every reason is long enough to be a reason, not a restatement of the number"

printf '%s\n' "$OUT" | /usr/bin/grep -q '^BRACKETS_NOTHING' && \
  no "a guard's range brackets nothing" "" || \
  ok "no guard's range is empty or inverted"

# -- AND NO GUARD RESTATES ITS OWN CURRENT VALUE AS ITS RANGE, which is the
#    one edit that turns a guard into a tautology. A range narrower than 1% of
#    its own midpoint is that edit.
printf '%s\n' "$(python3 - "$FILE" <<'PY'
import sys, tomllib
for g in tomllib.load(open(sys.argv[1], "rb")).get("guard", []):
    lo, hi = g["range"]
    mid = (lo + hi) / 2 or 1.0
    if (hi - lo) / abs(mid) < 0.01:
        print("TAUT", g["name"])
PY
)" | /usr/bin/grep -q '^TAUT' && no "a guard's range is its own value, which passes until somebody edits it" "" || \
  ok "no guard's range is a restatement of the value it measures"

# -- REFUSALS, each driven. None of these reaches the provider.
rc_of() { ( cd "$ROOT" && timeout 120 python3 "$TOOL" --file "$1" >"$TMP/out" 2>&1 ); echo $?; }

printf 'model = "%s"\n' "$(python3 -c "import tomllib,sys;print(tomllib.load(open(sys.argv[1],'rb'))['model'])" "$FILE")" > "$TMP/none.toml"
got=$(rc_of "$TMP/none.toml")
[ "$got" = 5 ] && /usr/bin/grep -q "reads exactly like a run that found none" "$TMP/out"
judge "a file declaring no guards REFUSES rather than printing no failures" $? "rc $got"

sed 's|^model = .*|model = "/nowhere/absent.scad"|' "$FILE" > "$TMP/nomodel.toml"
got=$(rc_of "$TMP/nomodel.toml")
[ "$got" = 5 ] && /usr/bin/grep -q "nothing was checked" "$TMP/out"
judge "a model that is not there REFUSES, because nothing was checked" $? "rc $got"

sed 's|^range = \[0.5, 25.0\]|range = [25.0, 0.5]|' "$FILE" > "$TMP/inverted.toml"
got=$(rc_of "$TMP/inverted.toml")
[ "$got" = 6 ] && /usr/bin/grep -q "brackets" "$TMP/out"
judge "an inverted range REFUSES before anything is evaluated" $? "rc $got"

# -- EVERY STRUCTURAL REFUSAL MUST HOLD WITH THE MODEL ABSENT, which is the arm
#    that would have caught this. The model is cal's design file and lives
#    OUTSIDE this repository, so on CI -- which clones only this repo -- it is
#    simply not there. A structural verdict that changes with the machine is not
#    a structural verdict, and the first version of guards.py checked the world
#    before the file: CI got rc 5 "model is not there" for an inverted range and
#    the arm above failed at 26 seconds.
AWAY=$(mktemp -d) || { no "mktemp failed" ""; fin; }
sed 's|^model = .*|model = "/nonexistent/on/any/runner/can_cutter.scad"|' "$FILE" > "$AWAY/away.toml"
got=$(rc_of "$AWAY/away.toml")
[ "$got" = 5 ] && /usr/bin/grep -q "WHERE finding, not a WHAT one" "$TMP/out"
judge "an absent model is reported as a WHERE finding, not as a defect in the guards" $? "rc $got"

sed 's|^range = \[0.5, 25.0\]|range = [25.0, 0.5]|' "$AWAY/away.toml" > "$AWAY/both.toml"
got=$(rc_of "$AWAY/both.toml")
[ "$got" = 6 ]
judge "an inverted range still refuses with rc 6 when the model is absent: the structural verdict does not depend on the machine" $? "rc $got"

printf 'model = "/nonexistent/x.scad"\n' > "$AWAY/nog.toml"
got=$(rc_of "$AWAY/nog.toml")
[ "$got" = 5 ]
judge "and a file with no guards still refuses with rc 5 when the model is absent" $? "rc $got"
rm -rf "$AWAY"

# -- EVERY ENDPOINT CARRIES PROVENANCE (rook F15, thread:wm-review/22). The
#    file's own header claimed every `because` was the source's reason, quoted
#    or paraphrased. rook measured: 2 of 24 endpoints sat inside a quote and
#    BOTH misread it -- "~40 mm" of bridge used as 40 DEGREES, and the blade's
#    80 mm height used as a score travel. These arms drive the refusal, not the
#    corrected numbers, because a number is a measurement and an arm asserting
#    one trains people to edit expectations.
d="$TMP/noprov"; mkdir -p "$d"
python3 - "$ROOT" "$d" <<'PY' || { no "world noprov could NOT be built"; }
import sys, pathlib, re
root, d = sys.argv[1], pathlib.Path(sys.argv[2])
t = pathlib.Path(root, "tests/claims/can-cutter.toml").read_text()
t2, n = re.subn(r"\ncites = \[\n.*?\n\]\n", "\n", t, count=1, flags=re.S)
assert n == 1, "no cites block to remove"
(d / "noprov.toml").write_text(t2)
PY
got=$( ( cd "$ROOT" && timeout 900 python3 "$TOOL" --file "$d/noprov.toml" >"$TMP/out" 2>&1 ); echo $? )
[ "$got" = 7 ] && /usr/bin/grep -q "gives no provenance for its" "$TMP/out"
judge "a guard whose endpoint has no provenance REFUSES: an envelope nobody has to justify is a number somebody chose, and two of these were misread units (F15)" \
      $? "rc $got"

d="$TMP/badcite"; mkdir -p "$d"
python3 - "$ROOT" "$d" <<'PY' || { no "world badcite could NOT be built"; }
import sys, pathlib
root, d = sys.argv[1], pathlib.Path(sys.argv[2])
t = pathlib.Path(root, "tests/claims/can-cutter.toml").read_text()
old = 'quote = "removes a 37-64 degree window at the cut plane"'
assert old in t, "no verifiable quote to break"
(d / "badcite.toml").write_text(t.replace(old, 'quote = "a sentence that is not in the model file"', 1))
PY
got=$( ( cd "$ROOT" && timeout 900 python3 "$TOOL" --file "$d/badcite.toml" >"$TMP/out" 2>&1 ); echo $? )
[ "$got" = 7 ] && /usr/bin/grep -q "is NOT in the file" "$TMP/out"
judge "a quote that does not resolve against the model file REFUSES -- the old citations named 'the harness, line 22' while the text sat in can_cutter.scad at 23" \
      $? "rc $got"

d="$TMP/wrongval"; mkdir -p "$d"
python3 - "$ROOT" "$d" <<'PY' || { no "world wrongval could NOT be built"; }
import sys, pathlib, re
root, d = sys.argv[1], pathlib.Path(sys.argv[2])
t = pathlib.Path(root, "tests/claims/can-cutter.toml").read_text()
t2, n = re.subn(r'endpoint = "low", value = 37\.0', 'endpoint = "low", value = 41.0', t, count=1)
assert n == 1, "no low cite value to desync"
(d / "wrongval.toml").write_text(t2)
PY
got=$( ( cd "$ROOT" && timeout 900 python3 "$TOOL" --file "$d/wrongval.toml" >"$TMP/out" 2>&1 ); echo $? )
[ "$got" = 7 ] && /usr/bin/grep -q "is not a citation" "$TMP/out"
judge "a cite naming a DIFFERENT number than the range REFUSES: a citation to another value is not a citation" $? "rc $got"

# -- AND THE RATIO IS PUBLISHED, because 4 of 24 sourced is what this envelope
#    set is worth, and a reader must not have to measure that themselves.
( cd "$ROOT" && timeout 900 python3 "$TOOL" >"$TMP/out" 2>&1 )
/usr/bin/grep -qE "^-- provenance: [0-9]+ of [0-9]+ endpoints carry a quote VERIFIED" "$TMP/out" \
  && /usr/bin/grep -q "declared chosen" "$TMP/out"
judge "the run publishes how many endpoints are sourced versus chosen, rather than leaving the reader to count" \
      $? "$(/usr/bin/grep -o '^-- provenance:.*' "$TMP/out" | head -1)"

fin
