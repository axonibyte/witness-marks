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
got=$( ( cd "$ROOT" && timeout 300 python3 "$TOOL" --provenance-only --file "$d/noprov.toml" >"$TMP/out" 2>&1 ); echo $? )
[ "$got" = 7 ] && /usr/bin/grep -q "gives no provenance for its" "$TMP/out"
judge "a guard whose endpoint has no provenance REFUSES: an envelope nobody has to justify is a number somebody chose, and two of these were misread units (F15)" \
      $? "rc $got"

# THE QUOTE CHECK IS DRIVEN AGAINST A MODEL THIS ARM SUPPLIES, not against
# cal's design file. The first version of this arm used the real one and
# asserted rc 7 -- which held here and FAILED CI in 42 seconds, because the
# design file lives OUTSIDE the repository and CI's machine does not have it,
# so the quote could not be checked and the run refused for a different
# reason. ROOT AND MACHINE ARE DIFFERENT AXES: the foreign-root gate runs on
# this box, where the file exists, so it is blind to this by construction.
# A synthetic model makes the REFUSAL LOGIC testable anywhere; whether the
# real design file agrees stays a machine-dependent NOTE, never an assertion.
d="$TMP/quote"; mkdir -p "$d"
# THE SYNTHETIC MODEL IS GENERATED FROM THE CITES, not hand-written to match
# them. Two hand-written versions drifted -- the first held one of four
# quotes, the second missed the five rejections added later -- and each time
# the refusal fired correctly over a fixture I had built wrong. A fixture
# derived from the thing under test cannot drift from it.
python3 - "$ROOT" "$d" <<'PY' || { no "the synthetic model could NOT be generated"; }
import sys, pathlib, re
root, d = sys.argv[1], pathlib.Path(sys.argv[2])
t = pathlib.Path(root, "tests/claims/can-cutter.toml").read_text()
quotes = re.findall(r'quote = "([^"]*)"', t)
assert quotes, "no quotes to mirror"
# every quote as its own comment line, so a two-line citation still matches
# through prose_of's marker stripping
d.mkdir(parents=True, exist_ok=True)
(d / "fake.scad").write_text("".join(f"// {q}\n" for q in quotes), encoding="utf-8")
print(f"  mirrored {len(quotes)} cite quote(s) into the synthetic model")
PY
python3 - "$ROOT" "$d" <<'PY' || { no "world quote could NOT be built"; }
import sys, pathlib, re
root, d = sys.argv[1], pathlib.Path(sys.argv[2])
t = pathlib.Path(root, "tests/claims/can-cutter.toml").read_text()
t = re.sub(r'^model = ".*"', f'model = "{d}/fake.scad"', t, count=1, flags=re.M)
# every cite now points at the synthetic model beside it
t = t.replace('file = "arts/can_cutter.scad"', 'file = "fake.scad"')
assert 'file = "fake.scad"' in t, "no cite was repointed at the synthetic model"
(d / "resolves.toml").write_text(t)
bad = t.replace('quote = "removes a 37-64 degree window at the cut plane"',
                'quote = "a sentence that is not in the model file"', 1)
assert bad != t, "no quote to break"
(d / "broken.toml").write_text(bad)
PY
got=$( ( cd "$ROOT" && timeout 300 python3 "$TOOL" --provenance-only --file "$d/broken.toml" >"$TMP/out" 2>&1 ); echo $? )
[ "$got" = 7 ] && /usr/bin/grep -q "is NOT in the file" "$TMP/out"
judge "a quote that does not resolve against its model REFUSES -- the old citations named 'the harness, line 22' while the text sat in can_cutter.scad at 23 (F15)" \
      $? "rc $got"

# AND THE MIRROR, so the refusal is not unconditional: the same cite with a
# quote that IS in the model passes the provenance stage and the run goes on
# to fail for the ordinary reason that this synthetic model has no symbols.
got=$( ( cd "$ROOT" && timeout 300 python3 "$TOOL" --provenance-only --file "$d/resolves.toml" >"$TMP/out" 2>&1 ); echo $? )
[ "$got" = 0 ] && ! /usr/bin/grep -q "is NOT in the file" "$TMP/out" \
  && /usr/bin/grep -q "quoted outright and verified" "$TMP/out"
judge "a quote that DOES resolve passes the provenance stage, so the refusal above is a check and not a constant" \
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
got=$( ( cd "$ROOT" && timeout 300 python3 "$TOOL" --provenance-only --file "$d/wrongval.toml" >"$TMP/out" 2>&1 ); echo $? )
[ "$got" = 7 ] && /usr/bin/grep -q "is not a citation" "$TMP/out"
judge "a cite naming a DIFFERENT number than the range REFUSES: a citation to another value is not a citation" $? "rc $got"

# -- AND THE RATIO IS PUBLISHED, because 4 of 24 sourced is what this envelope
#    set is worth, and a reader must not have to measure that themselves.
( cd "$ROOT" && timeout 300 python3 "$TOOL" --provenance-only >"$TMP/out" 2>&1 )
/usr/bin/grep -qE "^-- provenance over [0-9]+ endpoints: [0-9]+ quoted outright" "$TMP/out" \
  && /usr/bin/grep -q "WHOLLY CHOSEN" "$TMP/out"
judge "the run publishes how many endpoints are sourced versus chosen, rather than leaving the reader to count" \
      $? "$(/usr/bin/grep -o '^-- provenance:.*' "$TMP/out" | head -1)"

# -- A RANGE MAY NOT READMIT A VALUE THE SOURCE REJECTS. rook's F15b was one
#    instance; auditing the other twenty endpoints found FOUR MORE -- ranges
#    admitting 45 deg, 2.6 mm, 4.0 mm and 0.515 mm, each a value the design
#    history records as having FAILED. An `excludes` cite names the rejection
#    and this refuses when the range still covers it.
d="$TMP/readmit"; mkdir -p "$d"
python3 - "$ROOT" "$d" <<'PY' || { no "world readmit could NOT be built"; }
import sys, pathlib, re
root, d = sys.argv[1], pathlib.Path(sys.argv[2])
t = pathlib.Path(root, "tests/claims/can-cutter.toml").read_text()
# widen the groove-material floor back under the 2.6 the source says flexed
t2, n = re.subn(r'range = \[3\.0, 6\.0\]', 'range = [1.5, 6.0]', t, count=1)
assert n == 1, "no tightened floor to loosen"
t2 = t2.replace('endpoint = "low", value = 3.0', 'endpoint = "low", value = 1.5', 1)
(d / "readmit.toml").write_text(t2)
PY
got=$( ( cd "$ROOT" && timeout 300 python3 "$TOOL" --provenance-only --file "$d/readmit.toml" >"$TMP/out" 2>&1 ); echo $? )
[ "$got" = 7 ] && /usr/bin/grep -q "must EXCLUDE 2.6" "$TMP/out"
judge "a range loosened back over a value the source REJECTS refuses: readmitting a figure the design history records as failed is the defect, not the envelope" \
      $? "rc $got"

# AND A REJECTION MUST ITSELF RESOLVE, or it is the same unchecked citation
# one level out. This one caught a real error when it was written: the quote
# spanned two comment lines and the `//` markers broke the match.
# AGAINST THE SYNTHETIC MODEL AGAIN, for the reason the foreign-machine gate
# gave when this arm first borrowed the real one: on a box without cal's
# design file the rejection cannot be checked, so there is no refusal to
# assert and the arm passes for the wrong reason. The gate caught it within
# minutes of this arm being written, which is it earning its place.
d="$TMP/badrej"; mkdir -p "$d"
cp "$TMP/quote/fake.scad" "$d/fake.scad"
python3 - "$ROOT" "$d" <<'PY' || { no "world badrej could NOT be built"; }
import sys, pathlib, re
root, d = sys.argv[1], pathlib.Path(sys.argv[2])
t = pathlib.Path(root, "tests/claims/can-cutter.toml").read_text()
t = re.sub(r'^model = ".*"', f'model = "{d}/fake.scad"', t, count=1, flags=re.M)
t = t.replace('file = "arts/can_cutter.scad"', 'file = "fake.scad"')
old = 'quote = "Was 4.0, which left only"'
assert old in t, "no rejection quote to break"
(d / "badrej.toml").write_text(t.replace(old, 'quote = "a rejection nobody wrote"', 1))
PY
got=$( ( cd "$ROOT" && timeout 300 python3 "$TOOL" --provenance-only --file "$d/badrej.toml" >"$TMP/out" 2>&1 ); echo $? )
[ "$got" = 7 ] && /usr/bin/grep -q "rejection quotes" "$TMP/out"
judge "a rejection whose quote does not resolve against the model file refuses -- a cited rejection is a citation and gets checked like one" \
      $? "rc $got"

# THE REAL FILE'S AGREEMENT IS NOT ASSERTED HERE. Whether cal's design file
# still holds these eight quotes is a question about a file this repository
# does not ship, so it lives in tools/tests/pin-arms.sh with the other
# machine-dependent checks. In this glob it would pass on one box and fail on
# every other -- which is what the foreign-machine gate reported when it was
# written here.

# -- AN IN-REPO CITATION RESOLVES AND IS MACHINE-INDEPENDENT, which is why it
#    is preferred over one into cal's design file. leaf_t's floor is two
#    extrusion widths at the nozzle the model names, and the SIDE comes from
#    this project's own buildability harness rejecting anything under two.
#    This arm needs no out-of-repo file at all, so it holds on any machine.
d="$TMP/inrepo"; mkdir -p "$d"
python3 - "$ROOT" "$d" <<'PY' || { no "world inrepo could NOT be built"; }
import sys, pathlib, re
root, d = sys.argv[1], pathlib.Path(sys.argv[2])
t = pathlib.Path(root, "tests/claims/can-cutter.toml").read_text()
old = 'quote = "A 0.7 mm slot is under two extrusion widths"'
assert old in t, "no in-repo rejection quote to break"
(d / "broken.toml").write_text(t.replace(old, 'quote = "a rule this harness never stated"', 1))
PY
got=$( ( cd "$ROOT" && timeout 300 python3 "$TOOL" --provenance-only --file "$d/broken.toml" >"$TMP/out" 2>&1 ); echo $? )
[ "$got" = 7 ] && /usr/bin/grep -q "rejection quotes" "$TMP/out"
judge "an IN-REPO rejection quote is resolved and refused when it does not match -- a citation into this repository needs no other machine, so CI checks it too" \
      $? "rc $got"

# and the three-way tally is published, because "declared chosen" was lumping
# an endpoint nobody can source with one whose SIDE the source fixes
( cd "$ROOT" && timeout 300 python3 "$TOOL" --provenance-only >"$TMP/out" 2>&1 )
/usr/bin/grep -qE "quoted outright and verified, [0-9]+ chosen-but-side-sourced" "$TMP/out" \
  && /usr/bin/grep -q "WHOLLY CHOSEN" "$TMP/out"
judge "the provenance tally is THREE-WAY: quoted, chosen-but-side-sourced, and wholly chosen -- reporting the middle as 'chosen' understated what the audit established" \
      $? "$(/usr/bin/grep -o 'provenance over.*' "$TMP/out" | head -1)"

fin
