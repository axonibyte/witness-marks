#!/usr/bin/env python3
"""Verify the provider pin against what the registries actually serve.

WHY THIS EXISTS. `provider/PROVIDER.toml` recorded `pypi_sdist_sha256` and
`git_tag_sha` from 2026-11-14 and NOTHING READ EITHER. Grepped across the tree,
the only occurrence of either key was its own definition, and `provider/uv.lock`
resolves the package by VERSION STRING from pypi.org. So the pin was a version
plus two decorative hashes: a yank-and-republish, a mirror substitution, or a
typo in the recorded sha were all invisible.

THE HOUSE STANDARD IS STRICTER, and it is written down. coop/bonemesh.sh, under
a signed decision:

    FETCH AND VERIFY RATHER THAN VENDOR. [...] the thing we actually need
    pinned is a COMMIT. So the commit is what is written down, the tag is the
    human's half of it, and this refuses when they disagree.

THE PEEL IS THE TRAP THAT STANDARD NAMES. An annotated tag's ref points at a
tag OBJECT, not at a commit; `refs/tags/X^{}` is the commit. Upstream has both
kinds -- v0.3.0 and v0.6.0 are annotated, v0.6.1 is lightweight -- so a
verifier that ignored the peel would be right by luck on the tag we pin and
wrong on its neighbours.

WHY THE PINS LIVE IN PROVIDER.toml AND NOT IN THIS FILE, which is a deliberate
divergence from bonemesh.sh's shape: PROVIDER.toml already is the record, and
copying the figures here would create a second place for them to drift. One
source, read at run time.

usage: provider-verify.py [--provider-dir DIR] [--json]

exit codes, each DISTINCT because they mean different things:
  0  every recorded pin matches what the registry serves
  3  a registry could not be reached -- NOT a pass; an unverifiable pin is not
     a verified one, and this is the code that keeps those apart
  4  the PyPI sdist hash disagrees with the recorded one
  5  the tag and the recorded commit disagree
  6  a pin is missing from PROVIDER.toml, so there was nothing to verify
  7  the OpenSCAD kernel differs from the one recorded
  8  the geometry BACKEND differs from the one recorded -- a separate axis from
     the version, and the one that decides whether the CGAL banner exists
  9  the STL export FLAVOR differs from the one recorded. Every digest this
     project takes over an export is taken over those bytes, and OpenSCAD's
     own --help announces that this default is planned to change
 10  a pinned LIBRARY's installed version differs, or cannot be read. BOSL2 is
     the one the corpus uses, and its version is not a property of this
     checkout
 11  an ENVIRONMENT knob diverges from the record. Not cosmetic: OPENSCAD_PATH
     selects the binary, so a kernel pin verified against `openscad` on PATH
     says nothing about the binary the provider will actually run
 12  a `.env` is reachable from the working directory. The provider calls
     load_dotenv() on every Config.from_env(), searching upward, so such a
     file supplies any of the knobs above invisibly
"""
import hashlib
import json
import os
import re
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

E_UNREACHABLE = 3
E_SDIST = 4
E_TAG = 5
E_MISSING_PIN = 6
E_KERNEL = 7
E_BACKEND = 8
E_EXPORT_FORMAT = 9
E_LIBRARY = 10
E_ENV = 11
E_DOTENV = 12


#: The pin fields this refuses to run without. Everything else in the file is
#: read whether or not this list knows about it -- see `pins`.
REQUIRED = ("name", "upstream", "version", "pypi_sdist_sha256", "git_tag_sha")


def pins(provider_dir):
    """Every `key = "value"` in PROVIDER.toml, ENUMERATED RATHER THAN NAMED.

    THIS USED TO NAME A SUBSET and that is a defect with a history in this very
    tree: tools/provider-audit.py's own read_pin docstring says it, "a
    docstring describing a subset is how the surface came to record a pin it
    could not be checked against." A named list means every new axis has to be
    added in two places, and the failure mode when it is not is SILENT -- the
    field sits in the record, unread, exactly as pypi_sdist_sha256 did for
    three days. So the reader takes what the file holds and the REQUIRED tuple
    above carries the separate question of what may not be missing.

    Section headers are cosmetic to this: the keys are flat and distinct, and
    a `[env]` heading is there for a human reading the file."""
    text = (provider_dir / "PROVIDER.toml").read_text(encoding="utf-8")
    return {m.group(1): m.group(2)
            for m in re.finditer(r'^([A-Za-z_][A-Za-z0-9_]*)\s*=\s*"([^"]*)"', text, re.M)}


def pypi_sdist_sha(name, version, timeout=60):
    """The sha256 PyPI serves for the sdist, or None with a reason."""
    url = f"https://pypi.org/pypi/{name}/{version}/json"
    try:
        with urllib.request.urlopen(url, timeout=timeout) as fh:
            doc = json.load(fh)
    except (urllib.error.URLError, OSError, ValueError) as e:
        return None, f"pypi.org could not be read: {e}"
    for u in doc.get("urls", []):
        if u.get("packagetype") == "sdist":
            return u.get("digests", {}).get("sha256"), None
    return None, f"{name} {version} has no sdist on PyPI"


def tag_commit(upstream, tag, timeout=120):
    """The PEELED commit of a tag, or None with a reason.

    An annotated tag's ref is a tag object; `^{}` is the commit. Both forms are
    requested and the peeled one wins when present, which is the whole point.
    """
    try:
        p = subprocess.run(["git", "ls-remote", "--tags", upstream, tag, tag + "^{}"],
                           capture_output=True, text=True, timeout=timeout)
    except (OSError, subprocess.TimeoutExpired) as e:
        return None, f"{upstream} could not be reached: {e}"
    if p.returncode != 0:
        return None, f"git ls-remote exited {p.returncode}: {p.stderr.strip()[:120]}"
    refs = {}
    for line in p.stdout.splitlines():
        parts = line.split()
        if len(parts) == 2:
            refs[parts[1]] = parts[0]
    peeled = refs.get(f"refs/tags/{tag}^{{}}")
    plain = refs.get(f"refs/tags/{tag}")
    if peeled:
        return peeled, None
    if plain:
        return plain, None
    return None, f"{upstream} serves no tag {tag}"


def kernel_backend(timeout=120):
    """Which geometry backend the kernel actually used, by probing it.

    THE BACKEND IS A SEPARATE AXIS FROM THE VERSION and it changes both the
    geometry and the diagnostics. OpenSCAD 2026.10.14 takes
    `--backend=CGAL|Manifold`, defaults to Manifold on this build, and only
    CGAL emits the `Simple:` / `Volumes:` banner that the provider's
    diagnostics parse. That is issue 53: `mesh_health.manifold` goes null not
    because the binary changed name but because its default backend changed.
    So pinning `kernel_version` alone leaves the thing that actually decides
    the answer unpinned -- the third time this pin has been incomplete, after
    the provider and the kernel themselves.

    Detected rather than asked: there is no `--print-backend`, so a trivial
    model is rendered and the banner's presence is the reading. CGAL prints it;
    Manifold does not.
    """
    import tempfile

    src = "difference(){ cube([10,10,10]); translate([5,5,-1]) cylinder(h=12,r=3,$fn=32); }"
    with tempfile.TemporaryDirectory() as d:
        scad = Path(d) / "probe.scad"
        scad.write_text(src, encoding="utf-8")
        try:
            p = subprocess.run(["openscad", "-o", str(Path(d) / "probe.stl"), str(scad)],
                               capture_output=True, text=True, timeout=timeout)
        except (OSError, subprocess.TimeoutExpired) as e:
            return None, f"the backend probe could not run: {e}"
    text = (p.stdout or "") + (p.stderr or "")
    # THE BANNER IS INDENTED: the line is "   Simple:     yes", three leading
    # spaces. An anchored ^Simple: never matched, so this returned "Manifold"
    # unconditionally and PASSED because that is what PROVIDER.toml recorded --
    # a detector broken in the direction of its own expectation. The arm that
    # asserts CGAL prints the banner and Manifold does not is what caught it.
    if re.search(r"^\s*Simple:", text, re.M):
        return "CGAL", None
    # ABSENCE IS THE READING HERE, which is weaker than a positive one and is
    # said so: any future backend that also prints no CGAL banner reads as
    # Manifold. The alternative is inventing a flag OpenSCAD does not have.
    return "Manifold", None


def kernel_version(timeout=60):
    try:
        p = subprocess.run(["openscad", "--version"], capture_output=True, text=True,
                           timeout=timeout)
    except (OSError, subprocess.TimeoutExpired) as e:
        return None, f"openscad could not be run: {e}"
    # OpenSCAD prints its version on stderr on some builds and stdout on others.
    text = (p.stdout or "") + (p.stderr or "")
    m = re.search(r"OpenSCAD version (\S+)", text)
    return (m.group(1), None) if m else (None, f"no version in {text.strip()[:80]!r}")


def stl_export_format(timeout=120):
    """Which STL flavor `-o x.stl` writes, read from the BYTES rather than
    from the flag.

    WHY THIS IS AN AXIS AND NOT A DETAIL. OpenSCAD's own --help says it:
    "ASCII export is the current stl default, but binary stl is planned as the
    future default so asciistl should be explicitly specified in scripts when
    needed." Every digest this project records over an export -- wm-model's
    digest_of_export and digest_of_bytes both -- is taken over those bytes, so
    the day that default flips, every recorded digest changes and NOTHING we
    have would say why. An announced change is the cheapest kind to pin.

    MEASURED, not inferred: an ASCII STL begins with the token `solid`."""
    import tempfile
    with tempfile.TemporaryDirectory() as d:
        scad, out = Path(d) / "p.scad", Path(d) / "p.stl"
        scad.write_text("cube([10,10,10]);\n", encoding="utf-8")
        try:
            subprocess.run(["openscad", "-o", str(out), str(scad)],
                           capture_output=True, timeout=timeout, check=False)
        except FileNotFoundError:
            return None, "openscad is not on PATH"
        except subprocess.TimeoutExpired:
            return None, f"openscad did not finish within {timeout}s"
        if not out.is_file() or out.stat().st_size == 0:
            return None, "openscad wrote no STL, so the flavor cannot be read"
        head = out.read_bytes()[:8]
    return stl_flavor_of(head), None


def stl_flavor_of(head):
    """Which STL flavor a file's first bytes say it is.

    SEPARATE FROM THE RUN so it can be driven without OpenSCAD on the box: the
    classification is the part with a rule in it, and the rule is the ASCII
    format's own -- an ASCII STL opens with the token `solid`. A binary STL
    opens with an 80-byte header that may contain ANYTHING, including the word
    solid, which is why the check is `startswith` on the first token and not a
    search."""
    return "asciistl" if bytes(head).startswith(b"solid") else "binstl"


#: Where a pinned library's version is written, and the pattern that holds it.
#: One entry per `library_<name>` pin. A library whose version this cannot read
#: is a REFUSAL rather than a skip -- see verify_libraries.
LIBRARY_PROBES = {
    "bosl2": ("BOSL2/version.scad", re.compile(r"BOSL_VERSION\s*=\s*\[\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)")),
}


def library_dirs():
    """The directories OpenSCAD searches for libraries, in its own order.

    OPENSCADPATH FIRST, which is the whole reason this is an axis: it is an
    environment variable, so the library set a figure was measured against is
    not a property of this checkout at all."""
    dirs = []
    for part in (os.environ.get("OPENSCADPATH") or "").split(os.pathsep):
        if part:
            dirs.append(Path(part))
    dirs.append(Path.home() / ".local" / "share" / "OpenSCAD" / "libraries")
    dirs.append(Path("/usr/local/share/openscad/libraries"))
    return dirs


def library_version(name):
    """The installed version of one pinned library, or (None, why)."""
    rel, pattern = LIBRARY_PROBES[name]
    for d in library_dirs():
        f = d / rel
        if f.is_file():
            m = pattern.search(f.read_text(encoding="utf-8", errors="replace"))
            if m:
                return ".".join(m.groups()), None
            return None, f"{f} has no version this can read"
    return None, f"{rel} is in none of: " + ", ".join(str(d) for d in library_dirs())


def env_divergences(p):
    """Every recorded `env_NAME` pin whose live value differs from the record.

    "" MEANS MUST BE UNSET, and that is the useful half. OPENSCAD_PATH set in
    the environment chooses which binary the provider runs, which makes the
    kernel_version pin a statement about a binary that may not be the one used
    -- a pin defeated by a variable rather than by a disagreement. Recording a
    value here is the supported way to set one deliberately.

    THE `env_` NAMESPACE HOLDS ENVIRONMENT VARIABLES AND NOTHING ELSE. The
    first version of this recorded the dotenv rule as `env_dotenv_refused`,
    and this loop dutifully looked for a variable of that name, found it
    unset, and refused the whole run with rc 11 on its first execution. A
    namespace that holds two kinds of thing will be read as one of them.
    """
    out = []
    for key, want in sorted(p.items()):
        if not key.startswith("env_"):
            continue
        name = key[4:]
        live = os.environ.get(name, "")
        if live != want:
            out.append((name, want, live))
    return out


def dotenv_origin(provider_dir):
    """Where python-dotenv's search actually STARTS for the pinned provider.

    NOT THE WORKING DIRECTORY, and the first version of this guard had it
    wrong. rook found it (coop wm-review/2, F1) and the library's own source
    settles it: `find_dotenv` uses `os.getcwd()` only when `usecwd=True`, or
    when interactive, under a debugger, or frozen (dotenv/main.py:300).
    Otherwise it takes `sys._getframe()` and walks up from THE CALLER'S FILE
    (:305-306). The provider runs as a console script, so `__main__` has a
    `__file__`, it is not interactive, and the search begins at
    `openscad_mcp/utils/config.py` -- INSIDE THE VENV.

    So the tree to check is the installed package's. The guard that checked the
    repository root was looking somewhere the provider never consults: a `.env`
    beside the installed package would have loaded while it reported "none
    reachable", and one at the repo root would have been reported while the
    provider ignored it. Wrong in both directions.

    Returns None when the installed package cannot be found, which
    `dotenv_reachable`'s caller must treat as a refusal rather than a clean
    answer."""
    pattern = ".venv/lib/python*/site-packages/openscad_mcp/utils/config.py"
    for cfg in Path(provider_dir).glob(pattern):
        return cfg.parent
    return None


def dotenv_reachable(origin):
    """The nearest `.env` from `origin` upward, or None.

    `origin` IS AN ARGUMENT AND DOES NOT DEFAULT. A default of `Path.cwd()` is
    exactly the defect this had; making the caller name the tree keeps "which
    tree" answerable at the call site instead of hidden in a fallback."""
    here = Path(origin).resolve()
    for d in [here, *here.parents]:
        f = d / ".env"
        if f.is_file():
            return f
    return None


def dotenv_verdict(provider_dir):
    """The dotenv axis's whole decision, as a VALUE an arm can take.

    This was inline at the call site, which is why rook's F2 -- rc 12 walking
    up from `Path.cwd()` instead of from the tree python-dotenv actually starts
    in -- was reachable by unit arms only one layer down, where the walk lived
    but the CHOICE OF TREE did not. An inline decision is an untested decision;
    the arms call this.

    Returns (ok, detail). A provider whose installed config.py cannot be found
    is a REFUSAL, not a clean run: "none reachable" would then be an answer
    about a tree nobody looked in, which is the same defect one level up.
    """
    origin = dotenv_origin(provider_dir)
    if origin is None:
        return (False, "the installed openscad_mcp/utils/config.py was not found "
                       "under the provider venv, so the tree python-dotenv would "
                       "search is unknown")
    found = dotenv_reachable(origin)
    if found is not None:
        return (False, f"{found} is reachable from {origin}, where python-dotenv's "
                       f"search BEGINS; the provider loads it on every "
                       f"Config.from_env()")
    return (True, f"none reachable from {origin}")


def main():
    args = sys.argv[1:]
    provider_dir = Path("provider")
    if "--provider-dir" in args:
        i = args.index("--provider-dir")
        provider_dir = Path(args[i + 1])
    as_json = "--json" in args

    toml = provider_dir / "PROVIDER.toml"
    if not toml.is_file():
        print(f"provider-verify: no PROVIDER.toml at {toml}", file=sys.stderr)
        return E_MISSING_PIN
    p = pins(provider_dir)
    report = {"pins": p, "checks": []}

    def record(name, ok, detail):
        report["checks"].append({"check": name, "ok": ok, "detail": detail})
        if not as_json:
            print(f"  {'ok  ' if ok else 'FAIL'} {name:<22} {detail}")

    # A MISSING PIN IS REFUSED, NOT SKIPPED. Skipping it silently is how a
    # verifier comes to pass over a record with nothing in it.
    for key in REQUIRED:
        if not p.get(key):
            print(f"provider-verify: PROVIDER.toml has no `{key}`; there is nothing to "
                  f"verify and this refuses rather than reporting a clean pin",
                  file=sys.stderr)
            return E_MISSING_PIN

    served, err = pypi_sdist_sha(p["name"], p["version"])
    if served is None:
        record("pypi sdist", False, err)
        if not as_json:
            print(f"provider-verify: UNREACHABLE -- {err}. That is NOT a pass: an "
                  f"unverifiable pin is not a verified one", file=sys.stderr)
        return E_UNREACHABLE
    if served != p["pypi_sdist_sha256"]:
        record("pypi sdist", False, f"serves {served}, recorded {p['pypi_sdist_sha256']}")
        return E_SDIST
    record("pypi sdist", True, f"{served[:16]}... matches")

    tag = "v" + p["version"]
    commit, err = tag_commit(p["upstream"], tag)
    if commit is None:
        record("tag peel", False, err)
        return E_UNREACHABLE
    if commit != p["git_tag_sha"]:
        record("tag peel", False, f"{tag} is {commit}, recorded {p['git_tag_sha']}")
        return E_TAG
    record("tag peel", True, f"{tag} -> {commit[:12]}... matches")

    # THE KERNEL, which #56 named as the half the pin never had. Recorded is
    # optional only so this script can be adopted before PROVIDER.toml carries
    # the field; once it does, a mismatch refuses.
    running, err = kernel_version()
    if running is None:
        record("kernel", False, err)
        return E_UNREACHABLE
    want = p.get("kernel_version")
    if want and running != want:
        record("kernel", False, f"running {running}, recorded {want}")
        return E_KERNEL
    if want:
        record("kernel", True, f"{running} matches")
    else:
        record("kernel", True, f"{running} (PROVIDER.toml records none; see issue 56)")

    # THE BACKEND, which decides whether the CGAL banner exists at all.
    backend, err = kernel_backend()
    if backend is None:
        record("backend", False, err)
        return E_UNREACHABLE
    want_b = p.get("kernel_backend")
    if want_b and backend != want_b:
        record("backend", False, f"running {backend}, recorded {want_b}")
        return E_BACKEND
    record("backend", True, f"{backend}" + ("" if want_b else " (PROVIDER.toml records none)"))
    report["pins"]["detected_backend"] = backend

    # THE AXES ISSUE 58 ENUMERATED. Each is recorded-optional for the same
    # reason the kernel was: the script is adoptable before the record carries
    # the field. Once recorded, a divergence refuses.
    want_fmt = p.get("kernel_export_format_stl")
    if want_fmt:
        fmt, err = stl_export_format()
        if fmt is None:
            record("stl flavor", False, err)
            return E_UNREACHABLE
        if fmt != want_fmt:
            record("stl flavor", False, f"writes {fmt}, recorded {want_fmt} -- every "
                                       f"recorded export digest is over these bytes")
            return E_EXPORT_FORMAT
        record("stl flavor", True, f"{fmt} matches")
        report["pins"]["detected_stl_flavor"] = fmt

    for key in sorted(k for k in p if k.startswith("library_")):
        name = key[len("library_"):]
        if name not in LIBRARY_PROBES:
            record(f"library {name}", False, "no probe knows where this library "
                                             "writes its version")
            return E_LIBRARY
        got, err = library_version(name)
        if got is None:
            record(f"library {name}", False, err)
            return E_LIBRARY
        if got != p[key]:
            record(f"library {name}", False, f"installed {got}, recorded {p[key]}")
            return E_LIBRARY
        record(f"library {name}", True, f"{got} matches")

    diverged = env_divergences(p)
    if diverged:
        for name, want, live in diverged:
            record(f"env {name}", False,
                   (f"set to {live!r}, record says it must be unset" if want == ""
                    else f"is {live!r}, recorded {want!r}"))
        return E_ENV
    recorded_env = [k for k in p if k.startswith("env_")]
    if recorded_env:
        record("env", True, f"{len(recorded_env)} knobs match the record")

    if p.get("dotenv_refused", "") == "true":
        ok, detail = dotenv_verdict(provider_dir)
        record("dotenv", ok, detail)
        if not ok:
            return E_DOTENV

    if as_json:
        print(json.dumps(report, indent=1, sort_keys=True))
    else:
        ci = p.get("kernel_ci_pin")
        if ci and ci != running:
            print(f"  note the provider's own CI tests against OpenSCAD {ci}; this box "
                  f"runs {running}, which is an untested combination (issue 53)")
        print(f"-- the pin verifies: {p['name']} {p['version']}, tag {tag}, "
              f"kernel {running}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
