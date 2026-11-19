#!/bin/sh
# Tier 2: the pinning axes issue 58 enumerated -- the half that needs nothing.
#
# THE SPLIT, and it is the same seam the provider-claims arm documents. The
# four REFUSALS these axes add (rc 9, 10, 11, 12) each sit behind the pypi,
# tag, kernel and backend checks, so driving one end-to-end needs the network
# and OpenSCAD: those arms live in tools/tests/pin-arms.sh, which is
# deliberately run and NOT in the glob CI walks. What runs here is every rule
# with a decision in it, driven directly: the reader, the classifier, the
# library probe, the env comparison and the dotenv search.
#
# WHY THE READER IS ARMED AT ALL, since "it reads a file" sounds untestable:
# the defect it replaces was a named subset. tools/provider-audit.py's own
# docstring says it -- "a docstring describing a subset is how the surface came
# to record a pin it could not be checked against" -- and pypi_sdist_sha256 sat
# in the record unread for three days. An arm that fails when a recorded field
# is invisible to the reader is the only thing that keeps that from recurring.
set -u
ROOT=${AXES_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)}
pass=0; fail=0
ok() { pass=$((pass+1)); printf '  ok   %s\n' "$1"; }
no() { fail=$((fail+1)); printf '  FAIL %s\n' "$1"; }
fin() { printf -- '-- %d/%d arms pass\n' "$pass" $((pass+fail)); [ "$fail" = 0 ] || exit 1; exit 0; }

VERIFY="$ROOT/tools/provider-verify.py"
[ -f "$VERIFY" ] || { printf 't_pinning_axes: no %s\n' "$VERIFY" >&2; exit 2; }

OUT=$(PV="$VERIFY" PROVIDER_DIR="$ROOT/provider" python3 - <<'PYEOF'
import importlib.util, inspect, os, sys, tempfile
from pathlib import Path
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("pv", os.environ["PV"])
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
R = []
def ck(name, cond, detail=""):
    R.append(("ok" if cond else "FAIL", name, "" if cond else detail))

# -- the reader ENUMERATES. The arm that matters: a field the named list never
# knew about must still be read, because that is the exact way a pin goes unread.
with tempfile.TemporaryDirectory() as d:
    Path(d, "PROVIDER.toml").write_text(
        '[provider]\nname = "x"\nupstream = "u"\nversion = "1"\n'
        'pypi_sdist_sha256 = "s"\ngit_tag_sha = "g"\n'
        'a_field_no_list_knows = "seen"\n[env]\nenv_FOO = "bar"\n', encoding="utf-8")
    got = m.pins(Path(d))
ck("a the reader takes every recorded key, including one no list names",
   got.get("a_field_no_list_knows") == "seen" and got.get("env_FOO") == "bar", str(sorted(got)))
ck("b and it reads across a section heading, which is cosmetic to it",
   len(got) == 7, f"{len(got)} keys: {sorted(got)}")
ck("c REQUIRED is the separate question of what may not be missing, and is a subset",
   set(m.REQUIRED) <= set(got) and "a_field_no_list_knows" not in m.REQUIRED, str(m.REQUIRED))

# -- the STL flavor, from the bytes
ck("d an ASCII STL is the one that opens with the token solid",
   m.stl_flavor_of(b"solid OpenSCAD_Model") == "asciistl")
ck("e a binary STL header may CONTAIN the word solid and is still binary, so the "
   "rule is the opening token and never a search",
   m.stl_flavor_of(b"\x00\x01 solid-ish header") == "binstl"
   and m.stl_flavor_of(b"") == "binstl",
   m.stl_flavor_of(b"\x00\x01 solid-ish header"))

# -- the env comparison. "" means MUST BE UNSET.
keep = dict(os.environ)
try:
    os.environ.pop("MCP_HARD_WARNINGS", None)
    ck("f an unset knob recorded as unset is no divergence",
       m.env_divergences({"env_MCP_HARD_WARNINGS": ""}) == [])
    os.environ["MCP_HARD_WARNINGS"] = "true"
    d1 = m.env_divergences({"env_MCP_HARD_WARNINGS": ""})
    ck("g a knob SET while the record says unset is a divergence, named with both values",
       d1 == [("MCP_HARD_WARNINGS", "", "true")], str(d1))
    ck("h a knob set to the value the record names is NOT a divergence: recording a "
       "value is how one is set deliberately",
       m.env_divergences({"env_MCP_HARD_WARNINGS": "true"}) == [])
    ck("i a non-env key is never read as a knob -- the first version recorded the "
       "dotenv rule inside this namespace and refused its own clean run",
       m.env_divergences({"dotenv_refused": "true", "library_bosl2": "2.0.716"}) == [])
finally:
    os.environ.clear(); os.environ.update(keep)

# -- the library probe, against a tree built for it
with tempfile.TemporaryDirectory() as d:
    lib = Path(d, "BOSL2"); lib.mkdir()
    lib.joinpath("version.scad").write_text("BOSL_VERSION = [2,0,716];\n", encoding="utf-8")
    keep2 = os.environ.get("OPENSCADPATH")
    try:
        os.environ["OPENSCADPATH"] = d
        v, err = m.library_version("bosl2")
        ck("j the library probe reads BOSL2's own BOSL_VERSION, not a directory name",
           v == "2.0.716" and err is None, f"{v} {err}")
        ck("k and OPENSCADPATH comes FIRST in the search, which is why it is an axis: "
           "the library set is not a property of this checkout",
           m.library_dirs()[0] == Path(d), str(m.library_dirs()[:1]))
        lib.joinpath("version.scad").write_text("nothing here\n", encoding="utf-8")
        v2, err2 = m.library_version("bosl2")
        ck("l a library whose version cannot be READ is a refusal, never a skip",
           v2 is None and err2, f"{v2} {err2}")
    finally:
        os.environ.pop("OPENSCADPATH", None)
        if keep2 is not None:
            os.environ["OPENSCADPATH"] = keep2

# -- the dotenv search, which is the surface nobody had enumerated
with tempfile.TemporaryDirectory() as d:
    deep = Path(d, "a", "b", "c"); deep.mkdir(parents=True)
    ck("m no .env above the directory reads as none reachable",
       m.dotenv_reachable(deep) is None, str(m.dotenv_reachable(deep)))
    Path(d, "a", ".env").write_text("MCP_HARD_WARNINGS=true\n", encoding="utf-8")
    found = m.dotenv_reachable(deep)
    ck("n a .env TWO directories up is reachable, because python-dotenv searches "
       "upward and the provider calls load_dotenv() on every Config.from_env()",
       found == Path(d, "a", ".env").resolve(), str(found))

# -- WHICH TREE, which is the axis the two arms above do not touch. They prove
# the WALK; rook's F2 was that the walk STARTED in the wrong place. find_dotenv
# with no path takes the calling frame's __file__ (dotenv/main.py:305-306) and
# uses os.getcwd() only under usecwd/interactive/frozen (:300), so the tree is
# the installed config.py's, never this checkout's.
# THE TREE IS SYNTHETIC ON PURPOSE. The first version of these arms read the
# REAL provider/.venv, and the foreign-root gate caught it: the venv is
# gitignored, so o/p/s failed at a clone root and would have failed in CI --
# the same class as the out-of-repo fixture paths that kept CI red for ten
# commits. What is under test here is the CODE's choice of tree, which holds at
# any root. Whether THIS BOX's provider is clean is a measurement of the box,
# and it lives in pin-arms.sh where the venv is a precondition.
def fake_provider(root, with_env_above=False):
    utils = Path(root, ".venv", "lib", "python3.12", "site-packages",
                 "openscad_mcp", "utils")
    utils.mkdir(parents=True)
    (utils / "config.py").write_text("from dotenv import load_dotenv\n", encoding="utf-8")
    if with_env_above:
        Path(root, ".venv", ".env").write_text("MCP_HARD_WARNINGS=true\n",
                                               encoding="utf-8")
    return Path(root)

with tempfile.TemporaryDirectory() as d:
    origin = m.dotenv_origin_declared(fake_provider(Path(d, "provider")))
    ck("o the dotenv origin is the INSTALLED config.py's directory, which is the "
       "frame python-dotenv would actually start from",
       origin is not None and origin.name == "utils"
       and origin.parent.name == "openscad_mcp", str(origin))
    ck("p and it is NOT the working directory -- naming cwd here is the whole of "
       "F2: a guard that walks the wrong tree passes a .env beside the provider",
       origin is not None and origin.resolve() != Path.cwd().resolve(),
       f"{origin} vs {Path.cwd()}")
with tempfile.TemporaryDirectory() as d:
    ck("q a provider directory with no installed config.py yields None, so the "
       "caller must REFUSE rather than report a tree it never found",
       m.dotenv_origin_declared(Path(d)) is None,
       str(m.dotenv_origin_declared(Path(d))))
ck("r dotenv_reachable takes its origin with NO DEFAULT -- a default is exactly "
   "how the cwd walk came back last time",
   inspect.signature(m.dotenv_reachable).parameters["origin"].default
   is inspect.Parameter.empty,
   repr(inspect.signature(m.dotenv_reachable)))

# -- THE DECISION, not its two halves. The arms above would all have passed
# while rc 12 watched the wrong tree, because the choice of tree was inline at
# the call site and nothing could call it. The resolvers are injected so every
# branch is reachable at any root; arm z asserts the DEFAULTS are the real ones,
# which is what stops the injection from becoming the hole.
def says(path, how="read from the launcher"):
    return lambda _pd, **kw: (path, how)

def cannot(how):
    return lambda _pd, **kw: (None, how)

with tempfile.TemporaryDirectory() as d:
    root = fake_provider(Path(d, "provider"))
    tree = m.dotenv_origin_declared(root)
    ok, detail = m.dotenv_verdict(root, live=says(tree))
    ck("s a clean verdict's detail NAMES THE TREE IT SEARCHED and says HOW it "
       "learned it -- 'none reachable' alone cannot be checked by a reader",
       ok and "openscad_mcp" in detail and "launcher" in detail, f"{ok} {detail}")
    ck("t and that detail does NOT name the working directory",
       str(Path.cwd()) not in detail, detail)
with tempfile.TemporaryDirectory() as d:
    root = fake_provider(Path(d, "provider"), with_env_above=True)
    tree = m.dotenv_origin_declared(root)
    ok3, detail3 = m.dotenv_verdict(root, live=says(tree))
    ck("u a .env ABOVE the installed config.py -- beside the provider, where a "
       "cwd walk would never look -- is the refusal F2 said rc 12 was missing",
       ok3 is False and ".env" in detail3, f"{ok3} {detail3}")

# -- rook's FIFTH finding (wm-review/5): the tree must be READ FROM THE THING
# THAT RUNS, not derived. A glob that happens to agree is still a verdict about
# a tree nobody proved is the live one.
with tempfile.TemporaryDirectory() as d:
    root = fake_provider(Path(d, "provider"))
    ok4, detail4 = m.dotenv_verdict(root, live=cannot("uv is not on PATH"))
    ck("v a launcher that CANNOT be asked is a refusal, not a clean tree -- the "
       "glob agreeing is not permission to answer without the reading",
       ok4 is False and "UNKNOWN" in detail4 and "uv is not on PATH" in detail4,
       f"{ok4} {detail4}")
    other = Path(d, "elsewhere", "openscad_mcp", "utils")
    other.mkdir(parents=True)
    ok5, detail5 = m.dotenv_verdict(root, live=says(other))
    ck("w a launcher importing a DIFFERENT tree than the one on disk refuses and "
       "names BOTH -- the pin would describe one while the provider loads the "
       "other, and no single number reports that honestly",
       ok5 is False and str(other) in detail5
       and str(m.dotenv_origin_declared(root)) in detail5, f"{ok5} {detail5}")
    ok6, _ = m.dotenv_verdict(root, live=says(other), declared=lambda _pd: None)
    ck("x and with nothing on disk to compare, the launcher's reading STANDS -- "
       "an absent cross-check must not veto the authority",
       ok6 is True, str(ok6))

# -- AND THE DEFAULTS ARE THE REAL RESOLVERS. Injection is how the arms reach
# every branch at a foreign root; it is also how a fix could be armed while the
# shipped path still globbed. So the defaults are asserted by NAME.
sig = inspect.signature(m.dotenv_verdict)
ck("y dotenv_verdict's live and declared both default to None, so the function "
   "chooses the real resolvers rather than a caller choosing for it",
   sig.parameters["live"].default is None
   and sig.parameters["declared"].default is None, repr(sig))
src = inspect.getsource(m.dotenv_verdict)
ck("z and the default it chooses is dotenv_origin_LIVE, the reading -- not "
   "dotenv_origin_declared, the glob: that swap is rook's finding reinstated",
   "live = dotenv_origin_live" in src
   and "live = dotenv_origin_declared" not in src,
   [l.strip() for l in src.splitlines() if "live =" in l])

for status, name, detail in R:
    print(f"{status}\t{name}\t{detail}")
PYEOF
) || { printf 't_pinning_axes: the driver itself failed\n%s\n' "$OUT" >&2; exit 2; }

printf '%s\n' "$OUT" | while IFS="$(printf '\t')" read -r status name detail; do
    [ -n "${status:-}" ] || continue
    if [ "$status" = ok ]; then printf '  ok   %s\n' "$name"; else printf '  FAIL %s -- %s\n' "$name" "$detail"; fi
done
# the loop above runs in a subshell, so the tally is recomputed here
pass=$(printf '%s\n' "$OUT" | /usr/bin/grep -c '^ok' || true)
fail=$(printf '%s\n' "$OUT" | /usr/bin/grep -c '^FAIL' || true)
# NO RESULT IS A THIRD OUTCOME, and leaving it out is how a driver that ran
# nothing reports green. An earlier mutation tool in this tree did exactly that
# -- an empty FAIL list from a run that applied no mutation -- so the count is
# checked against what this file actually defines rather than trusted.
want=$(/usr/bin/grep -cE '^ *ck\("' "$0" || true)
total=$((pass + fail))
if [ "$total" = 0 ]; then
    printf 't_pinning_axes: the driver produced NO CHECK LINES -- that is NO RESULT, not a pass\n' >&2
    exit 2
fi
if [ "$total" != "$want" ]; then
    printf 't_pinning_axes: %d checks ran but this file defines %d; a check that did not run cannot pass\n' "$total" "$want" >&2
    exit 2
fi
fin
