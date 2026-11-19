#!/usr/bin/env python3
"""Run the twelve guards against the model they are about (issue 36).

WHY A TOOL AND NOT A cargo TEST. The guards need `scad_eval`, which needs the
pinned provider, `uv` and OpenSCAD -- so as a cargo test they would be a suite
that cannot run in CI and therefore does not run at all. This is the deliberate
tier, like vectors-arms.sh and the smoke example. What CI covers instead is the
PARSE and every refusal, in wm_claims::scad, against recorded replies.

WHAT IT REFUSES, each a way a guard could report a pass it did not earn:
  3  a guard whose expression does not evaluate, or evaluates to a non-number.
     `undef` above all: OpenSCAD's absent value, which as a 0.0 would pass
     every range with a lower bound of zero.
  4  a guard outside its range -- the ordinary failure, and the point.
  5  a file with no guards, or a model that is not there. A run over nothing
     prints no failures, which reads exactly like a run that found none.
  6  a range that does not bracket anything, or is inverted.

usage: guards.py [--file tests/claims/can-cutter.toml] [--set NAME=VALUE ...]

`--set` overrides a model variable before evaluating, which is how a guard is
shown to BITE: move the parameter the guard is about and watch it fail.
"""
import json
import os
import subprocess
import sys
import tomllib

E_UNEVALUABLE = 3
E_OUT_OF_RANGE = 4
E_NOTHING = 5
E_BAD_RANGE = 6


def speak(provider_dir, model, expressions, overrides):
    """One scad_eval call for every guard at once, in the model's scope."""
    proc = subprocess.Popen(
        ["uv", "run", "--frozen", "--no-dev", "openscad-mcp"],
        cwd=provider_dir, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, text=True, bufsize=1)
    nid = [0]

    def send(o):
        proc.stdin.write(json.dumps(o) + "\n")
        proc.stdin.flush()

    def read(want):
        while True:
            line = proc.stdout.readline()
            if not line:
                return None
            line = line.strip()
            if not line.startswith("{"):
                continue
            try:
                msg = json.loads(line)
            except ValueError:
                continue
            if msg.get("id") == want:
                return msg

    try:
        send({"jsonrpc": "2.0", "id": 1, "method": "initialize",
              "params": {"protocolVersion": "2024-11-05", "capabilities": {},
                         "clientInfo": {"name": "wm-guards", "version": "0"}}})
        init = read(1)
        if not init:
            return None, "the provider never answered initialize"
        send({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
        args = {"scad_file": model, "expressions": expressions}
        if overrides:
            args["variables"] = overrides
        nid[0] = 2
        send({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
              "params": {"name": "scad_eval", "arguments": args}})
        answer = read(2)
        if not answer:
            return None, "the provider never answered scad_eval"
        body = answer.get("result", {})
        text = "".join(c.get("text", "") for c in body.get("content", []))
        return json.loads(text or "{}"), None
    finally:
        proc.terminate()


def main():
    args = sys.argv[1:]
    path = "tests/claims/can-cutter.toml"
    if "--file" in args:
        path = args[args.index("--file") + 1]
    overrides = {}
    for i, a in enumerate(args):
        if a == "--set" and i + 1 < len(args):
            k, _, v = args[i + 1].partition("=")
            overrides[k] = float(v)
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    with open(os.path.join(root, path), "rb") as fh:
        doc = tomllib.load(fh)
    guards = doc.get("guard") or []
    model = doc.get("model", "")
    # THE FILE IS JUDGED BEFORE THE WORLD IS, and the order is a defect this
    # had the wrong way round. A structural check needs nothing and is always
    # answerable; whether the model is on this machine is a fact about the
    # machine. With the world checked first, CI -- which clones only this
    # repository and has no copy of the design -- got "the model is not there"
    # for a file with an inverted range, so the arm asserting rc 6 saw rc 5 and
    # failed. The structural verdict must not depend on where it is run.
    if not guards:
        print(f"guards: {path} declares no guards; a run over nothing prints no "
              f"failures, which reads exactly like a run that found none", file=sys.stderr)
        return E_NOTHING
    for g in guards:
        lo, hi = g.get("range", [0, 0])
        if not (lo < hi):
            print(f"guards: `{g['name']}` has range [{lo}, {hi}], which brackets "
                  f"nothing", file=sys.stderr)
            return E_BAD_RANGE
    # AND ONLY NOW THE WORLD. The model lives OUTSIDE this repository -- it is
    # cal's design file, not ours -- so its absence is an ordinary state on any
    # machine but the author's, and it is reported as its own thing rather than
    # as a defect in the guards.
    if not os.path.isfile(model):
        print(f"guards: the model {model} is not on this machine, so nothing was "
              f"checked. The guards themselves are well-formed; this is a WHERE "
              f"finding, not a WHAT one", file=sys.stderr)
        return E_NOTHING

    reply, err = speak(os.path.join(root, "provider"), model,
                       [g["source"] for g in guards], overrides)
    if reply is None:
        print(f"guards: {err}", file=sys.stderr)
        return E_UNEVALUABLE

    rows = {r.get("expression"): r for r in reply.get("results") or []}
    if overrides:
        print(f"-- with {len(overrides)} override(s): "
              f"{', '.join(f'{k}={v}' for k, v in overrides.items())}")
    bad = 0
    unevaluable = 0
    for g in guards:
        row = rows.get(g["source"])
        lo, hi = g["range"]
        if row is None or not row.get("evaluated") or row.get("type") != "number":
            kind = (row or {}).get("type", "no answer")
            print(f"  UNEVALUABLE {g['name']:<52} {g['source']} -> {kind}")
            unevaluable += 1
            continue
        v = row["value"]
        if lo <= v <= hi:
            print(f"  ok          {g['name']:<52} {v:g} {g['unit']} in [{lo:g}, {hi:g}]")
        else:
            off = lo - v if v < lo else v - hi
            print(f"  OUT OF RANGE {g['name']:<51} {v:g} {g['unit']}, outside "
                  f"[{lo:g}, {hi:g}] by {off:g}")
            bad += 1
    print(f"-- {len(guards) - bad - unevaluable}/{len(guards)} guards hold"
          + (f", {unevaluable} unevaluable" if unevaluable else ""))
    if unevaluable:
        return E_UNEVALUABLE
    return E_OUT_OF_RANGE if bad else 0


if __name__ == "__main__":
    sys.exit(main())
