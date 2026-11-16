#!/usr/bin/env python3
"""Compute each cross-implementation vector on OUR side and compare to the file.

NEITHER IMPLEMENTATION IS THE ORACLE. The committed vector is. Some of these
have a closed form and the MATHEMATICS is the authority -- a cube's volume is
1000 whatever any kernel says. The rest are tessellated, where no closed form
applies at a given $fn and the agreed recorded value is what both sides must
reproduce; for those the analytic figure is carried alongside as context, never
as the expectation.

EMPTY IS A VALUE HERE, which is the whole reason `expect_empty` exists.
OpenSCAD reports an empty 3D result as "Current top level object is empty",
exits 0, and writes NO FILE. A runner that reads the absence of a file as an
error cannot express "this intersection is correctly empty", and one that reads
it as zero without deleting the target first will read the PREVIOUS run's file
-- which is mcp-026 in the corpus, the sharpest lying instrument in the logs.
So the target is deleted before every run and absence is a VALUE.

usage: vector-run.py <vectors.toml> [--record]
"""
import math
import os
import struct
import subprocess
import sys
import tempfile
import tomllib

OPENSCAD = os.environ.get("OPENSCAD", "openscad")


def render(src, out):
    if os.path.exists(out):
        os.unlink(out)  # see the module docstring: absence must mean absence
    with tempfile.NamedTemporaryFile("w", suffix=".scad", delete=False) as fh:
        fh.write(src)
        scad = fh.name
    try:
        p = subprocess.run([OPENSCAD, "--export-format", "binstl", "-o", out, scad],
                           capture_output=True, text=True, timeout=1800)
    finally:
        os.unlink(scad)
    return p.returncode, (p.stderr or "")


def facets(stl):
    with open(stl, "rb") as fh:
        d = fh.read()
    return struct.unpack("<I", d[80:84])[0]


def mesh(stl):
    with open(stl, "rb") as fh:
        d = fh.read()
    n = struct.unpack("<I", d[80:84])[0]
    vol = 0.0
    lo = [math.inf] * 3
    hi = [-math.inf] * 3
    for i in range(n):
        o = 84 + i * 50
        t = [struct.unpack("<3f", d[o + 12 + j * 12:o + 24 + j * 12]) for j in range(3)]
        a, b, c = t
        vol += (a[0] * (b[1] * c[2] - c[1] * b[2])
                - a[1] * (b[0] * c[2] - c[0] * b[2])
                + a[2] * (b[0] * c[1] - c[0] * b[1])) / 6
        for v in t:
            for k in range(3):
                lo[k] = min(lo[k], v[k])
                hi[k] = max(hi[k], v[k])
    return {"facets": n, "volume": abs(vol),
            "bbox": [round(hi[k] - lo[k], 6) for k in range(3)]}


def compute(v, out):
    rc, err = render(v["model"], out)
    if not os.path.exists(out):
        return {"empty": True, "rc": rc,
                "kernel_said": "empty" if "empty" in err.lower() else err.strip()[:80]}
    m = mesh(out)
    m["empty"] = False
    m["rc"] = rc
    return m


def main():
    path = sys.argv[1]
    record = "--record" in sys.argv
    doc = tomllib.load(open(path, "rb"))
    vs = doc["vector"]
    # THE OUTPUT DIRECTORY IS OVERRIDABLE SO THE STALE-FILE GUARD CAN BE DRIVEN.
    # With a fresh mkdtemp every run, `os.unlink(out)` above can never fire, so
    # the defense against reading the PREVIOUS run's file -- corpus mcp-026, the
    # sharpest lying instrument in the logs -- was untestable and therefore
    # unmeasured. $WMVEC_OUT lets a test plant a stale STL where an empty vector
    # will look for one, which is the only way to prove the guard works.
    tmp = os.environ.get("WMVEC_OUT") or tempfile.mkdtemp(prefix="wmvec-")
    os.makedirs(tmp, exist_ok=True)
    bad = 0
    for v in vs:
        out = os.path.join(tmp, v["id"] + ".stl")
        got = compute(v, out)
        q = v["question"]
        if v.get("expect_empty"):
            okk = got["empty"]
            shown = "empty" if got["empty"] else f"NOT empty ({got['facets']} facets)"
            exp = "empty"
        elif got["empty"]:
            okk, shown, exp = False, "empty", repr(v.get("expected"))
        elif q == "bbox":
            exp = [round(float(x), 6) for x in v["expected"]]
            tol = float(v.get("tolerance", 0))
            okk = all(abs(a - b) <= tol for a, b in zip(got["bbox"], exp))
            shown, exp = str(got["bbox"]), str(exp)
        else:
            val = got[q]
            exp_v = float(v["expected"])
            tol = float(v.get("tolerance", 0))
            okk = abs(val - exp_v) <= tol
            shown = f"{val:.6f}" if isinstance(val, float) else str(val)
            exp = f"{exp_v:g}"
        if record:
            print(f"{v['id']}  {q}  ->  {shown}")
            continue
        if okk:
            print(f"  ok   {v['id']:<22} {q:<10} {shown}")
        else:
            bad += 1
            print(f"  FAIL {v['id']:<22} {q:<10} got {shown}, vector says {exp}")
    if record:
        return
    print(f"-- {len(vs) - bad}/{len(vs)} arms pass")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
