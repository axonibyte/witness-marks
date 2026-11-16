#!/bin/sh
# Tier 3, source-as-data: NOTHING may create a file except through
# crates/wm-provider/src/artifact.rs.
#
# WHY A SOURCE SCAN AND NOT A UNIT TEST. The two rules that stop a stale
# artifact -- clear the target first, return no path on failure -- live in one
# function, and a unit test proves that function obeys them. It cannot prove
# that the next writer anyone adds goes through it. The failure mode this
# guards is somebody reaching for fs::write directly, which is the obvious
# thing to do and is how mcp notes 18 and 26 happened in the first place.
#
# TEST CODE IS EXEMPT AND THAT IS NAMED, not assumed: a test that seeds a stale
# file in order to prove it gets cleared has to write one directly. The
# exemption covers `#[cfg(test)]` modules and files under tests/, and nothing
# else.
set -u
ROOT=${AUDIT_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
pass=0; fail=0
ok() { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no() { fail=$((fail+1)); printf '  FAIL %s\n' "$1"; }
fin() { printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail)); [ "$fail" = 0 ] || exit 1; exit 0; }

[ -d "$ROOT/crates" ] || { no "$ROOT/crates is not a directory"; fin; }

# THE SCAN IS A BRACE WALK, in tools/audit-file-writes.py. It was a line cut --
# strip from the first `#[cfg(test)]` to end of file, on the convention that a
# crate's test module comes last -- and a mutant that appended a direct
# fs::write AFTER the test module survived the audit entirely. A convention is
# not a guarantee, and catching the writer somebody adds without thinking is
# the audit's only job.
OUT=$(python3 "$ROOT/tools/audit-file-writes.py" "$ROOT" 2>&1) || {
    no "the audit script did not run: $OUT"; fin
}
total=$(printf '%s\n' "$OUT" | /usr/bin/sed -n 's/^SCANNED //p')

# REFUSE A SCAN THAT MATCHES NOTHING AT ALL. Zero hits across a tree that
# certainly writes files means the pattern or the paths moved, and "no
# violations" would then be a green with nothing behind it.
if [ "${total:-0}" -eq 0 ]; then
    no "the scan matched NO file-creating call anywhere under crates/; the pattern or the tree moved, so this check is blind"
    fin
fi
ok "$total file(s) under crates/ contain a file-creating call"

unb=$(printf '%s\n' "$OUT" | /usr/bin/grep '^UNBALANCED ' || true)
if [ -n "$unb" ]; then
    no "the brace walk did not return to depth zero, so the scan may be reading a test module as live code:"
    printf '%s\n' "$unb" | sed 's/^/         /'
fi

viols=$(printf '%s\n' "$OUT" | /usr/bin/grep '^VIOLATION ' || true)
if [ -n "$viols" ]; then
    no "non-test code creates files outside artifact.rs:"
    printf '%s\n' "$viols" | sed 's/^VIOLATION /         /'
else
    ok "no non-test code creates a file outside artifact.rs"
fi

# And the one place it DOES belong must still clear before producing.
A="$ROOT/crates/wm-provider/src/artifact.rs"
if [ -r "$A" ]; then
    if /usr/bin/grep -q 'if let Err(e) = clear(target)' "$A"; then
        ok "artifact.rs clears the target before handing it to a producer"
    else
        no "artifact.rs no longer clears the target first; rule 2 means nothing without rule 1"
    fi
else
    no "artifact.rs is missing: the rules have nowhere to live"
fi
fin
