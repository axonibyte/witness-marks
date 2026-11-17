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
"""
import hashlib
import json
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


def pins(provider_dir):
    text = (provider_dir / "PROVIDER.toml").read_text(encoding="utf-8")
    out = {}
    for key in ("name", "upstream", "version", "pypi_sdist_sha256", "git_tag_sha",
                "kernel_version", "kernel_ci_pin", "kernel_backend"):
        m = re.search(rf'^{key}\s*=\s*"([^"]*)"', text, re.M)
        if m:
            out[key] = m.group(1)
    return out


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
    for key in ("name", "upstream", "version", "pypi_sdist_sha256", "git_tag_sha"):
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
