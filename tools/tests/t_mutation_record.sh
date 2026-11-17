#!/bin/sh
# Arms for tools/check-mutation-record.sh.
#
# THE REPO UNDER TEST IS BUILT HERE, not borrowed. An earlier version of this
# check was driven by making a probe branch in the REAL working tree and running
# `git add -A`, which swept every uncommitted file into the probe commit; the
# branch was then deleted and the work with it. It was recovered from the
# reflog, and the lesson is in the shape of this file: a test that needs a dirty
# git repo makes its own, in a temp directory, and never touches the tree it
# lives in.
set -u
SRC=${RECORD_SRC:-$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)/check-mutation-record.sh}
EV=$(CDPATH= cd -- "$(dirname -- "$0")/../../tests/fixtures/push-events" && pwd)
# RESOLVED HERE, with SRC and EV, because this script changes directory later
# and `dirname $0` then resolves against the wrong place -- it landed at / and
# swept the filesystem root, which the blind-sweep guard caught.
MUT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)/mutate.sh
MUTDIR=$(CDPATH= cd -- "$(dirname -- "$0")/../../tests/mutations" && pwd)
pass=0; fail=0
say() { if [ "$1" = yes ]; then pass=$((pass+1)); printf '  ok   %-52s %s\n' "$2" "$3"
        else fail=$((fail+1)); printf '  FAIL %-52s %s\n' "$2" "$3"; fi; }

R=$(mktemp -d) || exit 70
trap 'rm -rf "$R"' EXIT HUP INT TERM
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
cd "$R" || exit 70
git init --quiet -b main .
printf 'fn main() {}\n' > a.rs; git add -A; git commit --quiet -m "base"
BASE=$(git rev-parse HEAD)

arm() {  # arm <want-rc> <want-substring> <label> -- then the range as $4 $5
    out=$(sh "$SRC" "$4" "$5" 2>&1); rc=$?
    ok=yes; [ "$rc" = "$1" ] || ok=no
    case "$out" in *"$2"*) ;; *) ok=no ;; esac
    say "$ok" "$3" "rc=$rc"
}

# A: an added assertion with no record anywhere -> REFUSED
git switch --quiet -c addtest
printf '#[test]\nfn t() { assert_eq!(1, 1); }\n' >> a.rs
git add a.rs; git commit --quiet -m "adds a test"
arm 1 "records no mutation kill" 'an added assertion with no record is REFUSED' main addtest

# B: the same change, with a MUTATION: line -> allowed
git commit --quiet --amend -m "adds a test

MUTATION: inverted the comparison; t FAILED."
arm 0 "record=yes" 'a MUTATION: line in the message satisfies it' main addtest

# C: no MUTATION: line, but a tests/mutations/ file -> allowed
git commit --quiet --amend -m "adds a test, no message record"
mkdir -p tests/mutations/x; printf 'a.rs' > tests/mutations/x/subject
git add -A; git commit --quiet -m "record the kill as a file"
arm 0 "record=yes" 'a tests/mutations/ file satisfies it' main addtest

# D: a change with no new assertions at all -> allowed, and it says 0
git switch --quiet -c docs main
printf 'docs\n' > README.md; git add -A; git commit --quiet -m "docs only"
arm 0 "0 added assertion line(s)" 'a change with no assertions passes, and says 0' main docs

# E: THE FAIL-CLOSED ARM. A base that does not resolve must REFUSE. Without
# this, every arm above is also satisfied by a script that always exits 0 on a
# range it cannot read, which is the failure mode this gate exists to avoid.
arm 65 "REFUSING" 'an unresolvable base REFUSES rather than passing' refs/heads/nope docs

# F: an empty range is not a refusal; there is genuinely nothing to check
arm 0 "nothing to check" 'an empty range passes and says so' main main

# ---- --from-event: resolving the range from the push payload
ev() {  # ev <fixture> <want-rc> <want-substring> <label>
    f="$R/ev-$1.json"
    sed "s/PLACEHOLDER/$BASE/" "$EV/$1.json" > "$f"
    out=$(GITHUB_EVENT_PATH="$f" sh "$SRC" --from-event addtest 2>&1); rc=$?
    ok=yes; [ "$rc" = "$2" ] || ok=no
    case "$out" in *"$3"*) ;; *) ok=no ;; esac
    say "$ok" "$4" "rc=$rc"
}
ev ok            0  "record=yes"        'a normal push event resolves the range'
ev branch-created 66 "no usable"        'a branch creation REFUSES: no before-state'
ev no-before     66 "no usable"         'an event with no before key REFUSES'
ev null-before   66 "no usable"         'a null before REFUSES'
ev garbage       66 "no usable"         'a non-JSON payload REFUSES'
ev not-an-object 66 "no usable"         'valid JSON of the wrong shape REFUSES'

out=$(env -u GITHUB_EVENT_PATH sh "$SRC" --from-event addtest 2>&1); rc=$?
ok=yes; [ "$rc" = 66 ] || ok=no
case "$out" in *"is unset"*) ;; *) ok=no ;; esac
say "$ok" 'an unset GITHUB_EVENT_PATH REFUSES' "rc=$rc"

out=$(GITHUB_EVENT_PATH="$R/not-a-file.json" sh "$SRC" --from-event addtest 2>&1); rc=$?
ok=yes; [ "$rc" = 66 ] || ok=no
case "$out" in *"not readable"*) ;; *) ok=no ;; esac
say "$ok" 'an unreadable event file REFUSES' "rc=$rc"

# EVERY CARGO MUTANT MUST CARRY --no-fail-fast, including the default.
#
# Without it cargo stops at the FIRST failing target, so a mutation whose
# effect reaches two crates runs only the first one's tests and the arm the
# mutant NAMES may never execute -- which refusal 4 reports as a MISFIRE,
# a red battery for a reason that has nothing to do with the code.
#
# MEASURED 2026-11-16: mech-thickness-cubed-to-squared had no test-cmd, took
# the default, and passed for weeks. Then wm-claims grew an elasticity test
# that ALSO detects a broken cubic, cargo stopped there, and wm-mech's named
# arm never ran. The instance was two mutants; the class is the default.
ok=yes
/usr/bin/grep -q "cmd='cargo test --locked --workspace --no-fail-fast'" "$MUT" || ok=no
say "$ok" "mutate.sh's default test command carries --no-fail-fast" ""

bad=""
count=0
for d in "$MUTDIR"/*/; do
    [ -d "$d" ] || continue
    [ -r "$d/test-cmd" ] || { bad="$bad $(basename "$d")(none)"; continue; }
    case "$(cat "$d/test-cmd")" in
        *cargo*)
            count=$((count + 1))
            /usr/bin/grep -q -- '--no-fail-fast' "$d/test-cmd" \
                || bad="$bad $(basename "$d")" ;;
        *) ;;
    esac
done
ok=yes; [ -z "$bad" ] || ok=no
say "$ok" "every cargo mutant names --no-fail-fast" "$count checked;${bad:- none missing}"

# REFUSE A BLIND SWEEP. Zero cargo mutants found means the directory or the
# pattern moved, and "none missing" would be a green with nothing behind it.
ok=yes; [ "$count" -ge 1 ] || ok=no
say "$ok" "the sweep found cargo mutants to check" "count=$count"

# NO MUTANT IN THE CI GLOB MAY NEED THE NETWORK.
#
# CI runs `mutate.sh --all`, which globs tests/mutations/*/. A mutant whose
# test-cmd reaches pypi.org or github.com makes that gate fail, or flake, for a
# reason with nothing to do with the code. The network ones live in
# tests/mutations-net/ and the battery runs them separately.
#
# This arm exists because I put five of them in the glob first and only noticed
# on reading the test-cmd back -- the same mistake, one layer down, as putting
# network arms into the t_*.sh suite CI runs.
netbad=""
netn=0
for d in "$MUTDIR"/*/; do
    [ -d "$d" ] || continue
    [ -r "$d/test-cmd" ] || continue
    netn=$((netn + 1))
    if /usr/bin/grep -qE 'pin-arms|pypi\.org|github\.com|ls-remote' "$d/test-cmd"; then
        netbad="$netbad $(basename "$d")"
    fi
done
ok=yes; [ -z "$netbad" ] || ok=no
say "$ok" "no mutant in the CI glob needs the network" "$netn checked;${netbad:- none}"

ok=yes; [ "$netn" -ge 1 ] || ok=no
say "$ok" "the network sweep found mutants to check" "count=$netn"

printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail))
[ "$fail" = 0 ] || exit 1
