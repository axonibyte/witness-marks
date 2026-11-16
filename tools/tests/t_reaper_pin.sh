#!/bin/sh
# .reaper.toml must pin its container image BY DIGEST, and the toolchain it
# records must be the one rust-toolchain.toml asks for.
#
# WHY A TEST AND NOT A COMMENT. A tag is a name the registry may repoint, so a
# tag-pinned build is reproducible until somebody else decides it is not, and
# the way that regression arrives is somebody editing a digest back to a tag
# because the digest is unreadable. That edit should fail a suite.
#
# THE SECOND ARM IS THE ONE WITH TEETH. Pinning by digest fixes the IMAGE; it
# does not tell anyone what is inside it. The manifest records the Rust version
# it was verified to carry, and this checks that against rust-toolchain.toml --
# so bumping the toolchain without re-resolving the image fails HERE rather
# than in a container that silently builds with the old compiler.
#
# OFFLINE ON PURPOSE. Resolving the digest from the registry would make this
# need the network and, worse, would check the pin against the CURRENT tag --
# which is exactly what pinning exists to stop mattering.
set -u
ROOT=${REAPER_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
M="$ROOT/.reaper.toml"; T="$ROOT/rust-toolchain.toml"
pass=0; fail=0
ok() { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no() { fail=$((fail+1)); printf '  FAIL %s\n' "$1"; }
fin() { printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail)); [ "$fail" = 0 ] || exit 1; exit 0; }

[ -r "$M" ] || { no ".reaper.toml is not readable"; fin; }
[ -r "$T" ] || { no "rust-toolchain.toml is not readable"; fin; }

# REFUSE A MANIFEST WITH NO IMAGE LINE. Zero image lines all pinned by digest is
# vacuously true, and is what this test reports if the manifest is restructured.
imgs=$(/usr/bin/grep -cE '^[[:space:]]*image[[:space:]]*=' "$M" || true); imgs=${imgs:-0}
if [ "$imgs" -lt 1 ]; then
    no "no 'image =' line in .reaper.toml; the manifest moved, or this parser did -- either way the pin is unchecked"
    fin
fi
ok "$imgs image line(s) found"

bad=0
while IFS= read -r line; do
    case "$line" in
        *@sha256:*) ;;
        *) no "pinned by TAG, not digest: $line"; bad=1 ;;
    esac
done <<EOF
$(/usr/bin/grep -E '^[[:space:]]*image[[:space:]]*=' "$M")
EOF
[ "$bad" = 0 ] && ok "every image is pinned by @sha256 digest"

if /usr/bin/grep -qE 'TODO' "$M"; then
    no ".reaper.toml still carries a TODO; the first-run item is not done"
else
    ok "no TODO remains in .reaper.toml"
fi

# The manifest records the version it was VERIFIED to carry, as a comment of the
# form "#   RUST_VERSION in it:     X.Y.Z".
rec=$(/usr/bin/sed -n 's/^#[[:space:]]*RUST_VERSION in it:[[:space:]]*\([0-9][0-9.]*\).*/\1/p' "$M" | head -1)
want=$(/usr/bin/sed -n 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$T" | head -1)
if [ -z "$rec" ]; then
    no ".reaper.toml records no verified RUST_VERSION, so the image's toolchain is unchecked"
elif [ -z "$want" ]; then
    no "rust-toolchain.toml names no channel"
elif [ "$rec" = "$want" ]; then
    ok "the image's recorded Rust $rec is the channel rust-toolchain.toml pins"
else
    no "the image was verified to carry Rust $rec but rust-toolchain.toml pins $want; re-resolve the digest"
fi
fin
