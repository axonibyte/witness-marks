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

METAMORPHIC RELATIONS are the second kind in this file, `[[relation]]`. They
assert the RELATION between two measurements rather than either measurement, so
they need no oracle at all: nobody knows the correct support area of an
arbitrary mesh, and everybody knows that scaling it by two must multiply its
area by four. See docs/testing-methodology.md, Tier 1.

A relation that cannot fail is a control that never bites. Scale-linearity
falls out of the arithmetic of summing signed tetrahedra, so it can pass on a
thoroughly broken reader -- which is why tools/tests/vectors-arms.sh drives
these against a damaged mesh reader rather than trusting that they bite.

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
    area = 0.0
    lo = [math.inf] * 3
    hi = [-math.inf] * 3
    for i in range(n):
        o = 84 + i * 50
        t = [struct.unpack("<3f", d[o + 12 + j * 12:o + 24 + j * 12]) for j in range(3)]
        a, b, c = t
        vol += (a[0] * (b[1] * c[2] - c[1] * b[2])
                - a[1] * (b[0] * c[2] - c[0] * b[2])
                + a[2] * (b[0] * c[1] - c[0] * b[1])) / 6
        # SURFACE AREA, summed from the triangles rather than read from the
        # header. A second quantity with a different exponent under scale --
        # area goes as the square where volume goes as the cube -- so a reader
        # that confused the two fails the pair in opposite directions.
        u = (b[0] - a[0], b[1] - a[1], b[2] - a[2])
        w = (c[0] - a[0], c[1] - a[1], c[2] - a[2])
        cx = u[1] * w[2] - u[2] * w[1]
        cy = u[2] * w[0] - u[0] * w[2]
        cz = u[0] * w[1] - u[1] * w[0]
        area += math.sqrt(cx * cx + cy * cy + cz * cz) / 2.0
        for v in t:
            for k in range(3):
                lo[k] = min(lo[k], v[k])
                hi[k] = max(hi[k], v[k])
    return {"facets": n, "volume": abs(vol), "area": area,
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


def measure_one(model, out):
    """Render one model and return its measurements, or None if empty.

    EMPTY IS NOT ZERO HERE EITHER. A relation over an empty side has not been
    shown to hold; it has failed to be evaluated, and the two must not read
    alike.
    """
    got = compute(model, out) if isinstance(model, dict) else compute({"model": model}, out)
    return None if got["empty"] else got


def run_relations(rels, tmp, record):
    """Assert each metamorphic relation. Returns the count that failed."""
    bad = 0
    for r in rels:
        q = r["question"]
        tol = float(r.get("tolerance", 1e-9))
        base = measure_one(r["base"], os.path.join(tmp, r["id"] + "-base.stl"))
        if base is None:
            bad += 1
            print(f"  FAIL {r['id']:<26} {q:<8} the BASE rendered empty, so the relation "
                  f"was not evaluated")
            continue

        if "sum_of" in r:
            # Additivity: the whole must equal the sum of its disjoint parts.
            total = 0.0
            missing = False
            for i, part in enumerate(r["sum_of"]):
                m = measure_one(part, os.path.join(tmp, f"{r['id']}-part{i}.stl"))
                if m is None:
                    missing = True
                    break
                total += float(m[q])
            if missing:
                bad += 1
                print(f"  FAIL {r['id']:<26} {q:<8} an addend rendered empty")
                continue
            got_v, want_v, shown = float(base[q]), total, "sum"
        else:
            other = measure_one(r["transformed"], os.path.join(tmp, r["id"] + "-t.stl"))
            if other is None:
                bad += 1
                print(f"  FAIL {r['id']:<26} {q:<8} the TRANSFORMED side rendered empty")
                continue
            factor = float(r["factor"])
            got_v, want_v, shown = float(other[q]), float(base[q]) * factor, f"x{factor:g}"

        if record:
            print(f"{r['id']}  {q}  {shown}  ->  {got_v:.6f} against {want_v:.6f}")
            continue
        # A RELATIVE TOLERANCE, because these compare two measured figures
        # rather than a measurement against a constant.
        scale = max(abs(want_v), 1.0)
        if abs(got_v - want_v) <= tol * scale:
            print(f"  ok   {r['id']:<26} {q:<8} {shown}  {got_v:.4f} ~ {want_v:.4f}")
        else:
            bad += 1
            print(f"  FAIL {r['id']:<26} {q:<8} {shown}  got {got_v:.6f}, "
                  f"the relation requires {want_v:.6f}")
    return bad


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
    rels = doc.get("relation", [])
    bad += run_relations(rels, tmp, record)
    total = len(vs) + len(rels)
    if record:
        return
    # REFUSE A FILE WITH NOTHING IN IT. Zero vectors and zero relations prints
    # "0/0 arms pass" and exits clean, which reads exactly like a green run.
    if total == 0:
        print("-- NO vectors and NO relations in this file; refusing rather than "
              "reporting a clean run over nothing")
        sys.exit(2)
    print(f"-- {total - bad}/{total} arms pass")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
