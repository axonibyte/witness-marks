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
else
    ok "every file's brace walk returned to depth zero"
fi

viols=$(printf '%s\n' "$OUT" | /usr/bin/grep '^VIOLATION ' || true)
if [ -n "$viols" ]; then
    no "non-test code creates files outside artifact.rs:"
    printf '%s\n' "$viols" | sed 's/^VIOLATION /         /'
else
    ok "no non-test code creates a file outside artifact.rs"
fi

# THE WALK IS TESTED ON FIXTURES, not only on the live tree. It counted every
# brace in the file until crates/wm-csg/src/tree.rs arrived -- a parser for
# OpenSCAD's `name(args) { children }` grammar, so it HAS to carry `b'{'` char
# literals, an escaped `'{{'` in a format string, and tests that put unbalanced
# delimiters inside strings deliberately. Three surplus braces, correct Rust,
# reported UNBALANCED. These arms pin the fix in both directions: a file whose
# braces live in literals walks clean, and a file that is genuinely unbalanced
# is still refused.
FIX=$(mktemp -d) || { no "could not make a fixture dir"; fin; }
trap 'rm -rf "$FIX"' EXIT INT TERM HUP
mkdir -p "$FIX/crates/c/src"

# One live writer so the scan is never blind (the SCANNED refusal above).
cat > "$FIX/crates/c/src/live.rs" <<'RS'
pub fn w() { let _ = std::fs::write("x", b"y"); }
RS

cat > "$FIX/crates/c/src/braces_in_literals.rs" <<'RS'
pub struct P<'a> { pub s: &'a [u8] }
impl<'a> P<'a> {
    pub fn f(&self, op: &str) -> String {
        match self.s.first() {
            Some(b'{') => format!("after {op}(...) expected '{{' or ';'"),
            Some(b'}') => String::from("}"),
            _ => String::new(),
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn unbalanced_inside_a_string_is_still_inside_it() {
        assert_eq!(super::P { s: b"{" }.f("x").is_empty(), false);
        let _ = "difference() {\n\tcube();\n";
        let _ = r#"a raw { brace"#;
        /* a /* nested */ comment with { */
        // and a trailing { in a line comment
    }
}
RS

cat > "$FIX/crates/c/src/truly_unbalanced.rs" <<'RS'
#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        let _ = 1;
}
RS

FOUT=$(python3 "$ROOT/tools/audit-file-writes.py" "$FIX" 2>&1) || {
    no "the audit script did not run over the fixture: $FOUT"; fin
}
if printf '%s\n' "$FOUT" | /usr/bin/grep -q '^UNBALANCED crates/c/src/braces_in_literals.rs'; then
    no "a file whose braces live in char, string and raw literals was called unbalanced; the walk is counting literal braces again"
else
    ok "braces inside char, string, raw and nested-comment literals do not move the walk"
fi
if printf '%s\n' "$FOUT" | /usr/bin/grep -q '^UNBALANCED crates/c/src/truly_unbalanced.rs'; then
    ok "a genuinely unbalanced test module is still refused"
else
    no "a genuinely unbalanced file was NOT refused: stripping literals disabled the refusal it was meant to preserve"
fi

# THE ORIGINAL MUTANT. A writer appended AFTER the test module is what defeated
# the line-cut version, so stripping literals must not have hidden it again.
cat > "$FIX/crates/c/src/after_tests.rs" <<'RS'
#[cfg(test)]
mod tests {
    #[test]
    fn t() { let _ = "a { brace in a string"; }
}
pub fn sneaky() { let _ = std::fs::write("late", b"z"); }
RS
AOUT=$(python3 "$ROOT/tools/audit-file-writes.py" "$FIX" 2>&1)
if printf '%s\n' "$AOUT" | /usr/bin/grep -q '^VIOLATION crates/c/src/after_tests.rs:6' \
   && printf '%s\n' "$AOUT" | /usr/bin/grep -q 'fs::write("late"'; then
    ok "a writer added AFTER a test module is still a violation, reported with its ORIGINAL text"
else
    no "the writer after the test module was missed, or its line number moved:"
    printf '%s\n' "$AOUT" | /usr/bin/grep after_tests | sed 's/^/         /'
fi

# A LIFETIME IS NOT A CHAR LITERAL, which is the one real ambiguity in the
# stripping. If `'a` were read as an unterminated literal the rest of the file
# would be blanked and every later brace lost.
LIFE=$(python3 - "$ROOT" <<'PYEOF'
import importlib.util, sys
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("a", sys.argv[1] + "/tools/audit-file-writes.py")
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
src = "struct P<'a> { s: &'a str }\nfn f() { let c = '{'; }\n"
st = m.strip_literals(src)
print("LEN", len(st) == len(src), "NL", st.count("\n") == src.count("\n"),
      "DEPTH", st.count("{") - st.count("}"))
PYEOF
) || { no "the lifetime probe did not run"; fin; }
case "$LIFE" in
    "LEN True NL True DEPTH 0") ok "a lifetime is not read as a char literal, and stripping preserves length and lines" ;;
    *) no "lifetime or length handling is wrong: $LIFE" ;;
esac

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
