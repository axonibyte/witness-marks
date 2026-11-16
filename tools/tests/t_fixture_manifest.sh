#!/bin/sh
# The vendored can-cutter files must be the files MANIFEST.toml records.
#
# A VENDORED FIXTURE DRIFTS SILENTLY. Somebody refreshes can_cutter.scad from
# the working copy because the design moved on, and every number in the manifest
# is now a measurement of a different part -- still printed, still precise,
# still cited. That is failure class C, a stale claim, and the corpus this
# fixture serves exists partly to catalogue it.
#
# So the hashes are checked, and a mismatch says REFRESH THE NUMBERS rather than
# merely reporting a difference, because the file changing is not the problem:
# the file changing while the numbers stay is.
set -u
ROOT=${FIXTURE_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
D="$ROOT/tests/fixtures/can_cutter"; M="$D/MANIFEST.toml"
pass=0; fail=0
ok() { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no() { fail=$((fail+1)); printf '  FAIL %s\n' "$1"; }
fin() { printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail)); [ "$fail" = 0 ] || exit 1; exit 0; }

[ -r "$M" ] || { no "MANIFEST.toml is not readable"; fin; }

n=$(/usr/bin/sed -n 's/^"\([^"]*\)" = "[0-9a-f]\{32\}"$/\1/p' "$M" | /usr/bin/grep -c . || true); n=${n:-0}
# REFUSE AN EMPTY HASH BLOCK: zero files all matching is vacuously true, and is
# what this reports if the manifest is restructured.
if [ "$n" -lt 3 ]; then
    no "only $n hash line(s) parsed from MANIFEST.toml; the manifest moved, or this parser did -- either way nothing is pinned"
    fin
fi
ok "$n hash line(s) parsed"

while IFS= read -r f; do
    [ -n "$f" ] || continue
    want=$(/usr/bin/sed -n "s/^\"$(printf '%s' "$f" | sed 's/[.[\*^$]/\\&/g')\" = \"\([0-9a-f]*\)\"$/\1/p" "$M" | head -1)
    if [ ! -r "$D/$f" ]; then
        no "$f is recorded in MANIFEST.toml and is not in the fixture directory"
        continue
    fi
    got=$( (md5 -q "$D/$f" 2>/dev/null) || md5sum "$D/$f" | cut -d' ' -f1 )
    if [ "$got" = "$want" ]; then ok "$f matches its recorded hash"
    else no "$f has changed since MANIFEST.toml was written ($want -> $got); REFRESH THE NUMBERS, do not just update the hash"; fi
done <<EOF
$(/usr/bin/sed -n 's/^"\([^"]*\)" = "[0-9a-f]\{32\}"$/\1/p' "$M")
EOF

# Every file in the fixture directory that is a model must be pinned. A file
# added and not recorded is a fixture nothing is checking.
for f in "$D"/*.scad; do
    b=$(basename "$f")
    if /usr/bin/grep -qF "\"$b\" = \"" "$M"; then ok "$b is pinned by the manifest"
    else no "$b sits in the fixture directory and MANIFEST.toml does not pin it"; fi
done
fin
