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

usage: guards.py [--provenance-only]  -- check the endpoints' citations and stop,
without starting a provider

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
E_NO_PROVENANCE = 7


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


def cited_file(repo, model, cfile):
    """Where a cite's file actually is: in THIS REPOSITORY if the path names
    something here, otherwise beside the model.

    AN IN-REPO CITATION IS STRICTLY BETTER than one into cal's design file: it
    is always present, so it is machine-independent and CI checks it too. The
    project's own corpus is a legitimate source -- `leaf_t`'s floor comes from
    the buildability harness, not from the model -- and before this, such a
    citation could not resolve at all, so the endpoint read as wholly chosen
    when it was not.
    """
    inrepo = os.path.join(repo, cfile)
    if os.path.isfile(inrepo):
        return inrepo
    return os.path.join(os.path.dirname(model), os.path.basename(cfile))


def prose_of(path):
    """A source file's prose, with comment markers removed and whitespace
    flattened, so a quote may span lines the way the comment does.

    A citation into a .scad lands in a COMMENT BLOCK, where every line starts
    `//`. Joining the raw lines puts those markers inside the text, and a
    perfectly good two-line quote then fails to match -- which is what
    happened to the first `excludes` cite written here, caught by this
    tool's own check. Stripping the markers is reading the prose; it is not
    loosening the comparison, which stays exact on the words.
    """
    out = []
    for line in open(path, encoding="utf-8", errors="replace"):
        out.append(line.lstrip().lstrip("/").strip() if line.lstrip().startswith("//")
                   else line)
    return " ".join(" ".join(out).split())


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

    # EVERY ENDPOINT SAYS WHERE IT CAME FROM (rook's F15, thread:wm-review/22).
    # The header of can-cutter.toml claimed every `because` was "the harness's
    # own reason, quoted or paraphrased". rook measured the whole set: of 24
    # endpoints, 14 appeared in a `because` at all, only 2 inside an actual
    # quote -- AND BOTH OF THOSE MISREAD IT. One took "~40 mm" of bridge and
    # used it as 40 DEGREES of mouth; the other took the blade's 80 mm height
    # as a score travel. Nothing checked provenance, so nothing could notice.
    #
    # A cite is either a QUOTE that must verify against the file, or an
    # explicit `unsourced` declaration. Declaring is honest; being silent is
    # not, and silence is what let a misread stand. The count is published
    # below, because "4 of 24 endpoints are sourced" is what the envelope set
    # is actually worth -- the same discipline as the rediscovery matrix.
    sourced, declared, rejected = 0, 0, 0
    unverified_rejections = 0
    for g in guards:
        cites = {c.get("endpoint"): c for c in (g.get("cites") or [])}
        lo, hi = g["range"]
        for end, val in (("low", lo), ("high", hi)):
            c = cites.get(end)
            if not c:
                print(f"guards: `{g['name']}` gives no provenance for its {end} "
                      f"endpoint {val}. An envelope nobody has to justify is a "
                      f"number somebody chose, and two of these turned out to be "
                      f"misread units", file=sys.stderr)
                return E_NO_PROVENANCE
            if c.get("value") != val:
                print(f"guards: `{g['name']}`'s {end} cite names {c.get('value')} "
                      f"but the range says {val}; a citation to a different number "
                      f"is not a citation", file=sys.stderr)
                return E_NO_PROVENANCE
            # A SOURCE CAN REJECT A VALUE WITHOUT NAMING A BOUND, and that
            # constrains the endpoint's SIDE even when the number itself is
            # chosen. rook's F15b was this exactly: a range whose floor was
            # the value the source called "what cuts nothing anybody wants".
            # Auditing the other twenty endpoints found FOUR MORE of it --
            # ranges admitting 45 deg, 2.6 mm, 4.0 mm and 0.515 mm, each a
            # value the design history records as having FAILED. So an
            # `excludes` cite says "the source rejects this, and the range
            # must not admit it", and that is checked rather than described.
            ex = c.get("excludes")
            if ex is not None:
                bad = ex.get("value")
                if bad is None or not (ex.get("quote") and ex.get("file")):
                    print(f"guards: `{g['name']}`'s {end} cite has an `excludes` with "
                          f"no value, quote or file", file=sys.stderr)
                    return E_NO_PROVENANCE
                if lo <= bad <= hi:
                    print(f"guards: `{g['name']}` must EXCLUDE {bad} -- the source "
                          f"rejects it ({ex['quote'][:70]!r}) -- and its range "
                          f"[{lo}, {hi}] admits it. A bound that readmits a value "
                          f"the design history records as having failed is the "
                          f"defect, not the envelope", file=sys.stderr)
                    return E_NO_PROVENANCE
                near_ex = cited_file(root, model, ex["file"])
                if os.path.isfile(near_ex):
                    if " ".join(ex["quote"].split()) not in prose_of(near_ex):
                        print(f"guards: `{g['name']}`'s {end} rejection quotes "
                              f"{ex['quote']!r}, and that text is NOT in "
                              f"{ex['file']}", file=sys.stderr)
                        return E_NO_PROVENANCE
                    rejected += 1
                else:
                    # UNVERIFIED HERE, NOT UNSOURCED. rook's F17
                    # (thread:wm-review/35): on a box without cal's design file
                    # these dropped into WHOLLY CHOSEN, so the tally said
                    # "nobody sourced this side" when it meant "I cannot check
                    # here" -- a claim about the RECORD made from an absence on
                    # THIS MACHINE. Measured by rook: their box printed
                    # 0/1/19 where this one prints 4/10/10. The outright quotes
                    # were already counted unverified; the rejections were not,
                    # and the asymmetry is the whole defect.
                    unverified_rejections += 1
                    print(f"  note {g['name']} {end}: {ex['file']} is not on this "
                          f"machine, so its REJECTION is unverified here (not "
                          f"unsourced)")
            if c.get("unsourced"):
                declared += 1
                continue
            quote, cfile = c.get("quote"), c.get("file")
            if not (quote and cfile):
                print(f"guards: `{g['name']}`'s {end} cite has neither a quote with "
                      f"a file nor an `unsourced` reason", file=sys.stderr)
                return E_NO_PROVENANCE
            # THE QUOTE IS CHECKED AGAINST THE FILE, not taken on trust -- the
            # old citations named "the harness, line 22" and the text was in the
            # MODEL file at line 23. An unreachable file is NO RESULT for this
            # endpoint, never a pass, and the report says so.
            near = cited_file(root, model, cfile)
            if os.path.isfile(near):
                if " ".join(quote.split()) not in prose_of(near):
                    print(f"guards: `{g['name']}`'s {end} cite quotes {quote!r} from "
                          f"{cfile}, and that text is NOT in the file. A citation "
                          f"that does not resolve is the defect it was added to "
                          f"close", file=sys.stderr)
                    return E_NO_PROVENANCE
                sourced += 1
            else:
                print(f"  note {g['name']} {end}: {cfile} is not on this machine, so "
                      f"its quote is UNVERIFIED here (not a pass)")
    total = 2 * len(guards)
    # THE TALLY IS THREE-WAY, because "declared chosen" was lumping together
    # two different states: an endpoint nobody can source at all, and one
    # whose NUMBER is chosen while its SIDE is fixed by a rejection the source
    # states outright. Reporting them as one figure understated what the audit
    # had established -- the same complaint rook made about "1 of 108".
    wholly = declared - rejected - unverified_rejections
    print(f"-- provenance over {total} endpoints: {sourced} quoted outright and "
          f"verified, {rejected} chosen-but-side-sourced (the number is mine, the "
          f"side is held off a value the source REJECTS, quote verified), "
          f"{wholly} WHOLLY CHOSEN")
    unverified = (total - sourced - declared) + unverified_rejections
    if unverified:
        print(f"-- {unverified} endpoint(s) UNVERIFIED HERE, because the file "
              f"they cite is not on this machine: "
              f"{total - sourced - declared} outright quote(s) and "
              f"{unverified_rejections} rejection(s). Unverified is not "
              f"unsourced -- the record says what it says wherever it is read")

    # --provenance-only STOPS HERE, BEFORE THE PROVIDER IS SPOKEN TO. rook
    # measured the reason (thread:wm-review/27, a cost rather than a finding):
    # on a box with no warm environment, the first `uv run --frozen --no-dev
    # openscad-mcp` sat about FIVE MINUTES while maturin built a dependency
    # from source. That is inside this tool's own bound and NOT inside CI's,
    # which gives each arm file 300s total (ci.yml:71) -- and the arm that
    # drives a resolvable quote is the first one whose model EXISTS on a
    # clean runner, so it is the first that would spawn at all.
    #
    # An arm checking provenance has no business starting a provider: the
    # question is entirely about this repository's own text. The flag makes
    # that explicit rather than leaving it to a timeout.
    if "--provenance-only" in args:
        return 0
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
