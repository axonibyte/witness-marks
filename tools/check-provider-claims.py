#!/usr/bin/env python3
"""Check the spec's [exists] claims against the provider's MEASURED surface.

This reads a surface captured by tools/provider-audit.py, so it needs no live
provider and runs in CI. The audit captures; this judges.

usage: check-provider-claims.py <claims.toml> <surface.json>

exit codes:
  0  every surface-decidable claim holds
  1  a claim failed
  6  the check was BLIND -- no claims, no surface, or a surface with no tools
"""
import json
import sys
import tomllib
from pathlib import Path

E_BLIND = 6


def verdict(claim, surface):
    """Return (status, detail). UNVERIFIED is never PASS."""
    tool_name = claim.get("tool")
    tools = surface.get("tools", {})
    if tool_name not in tools:
        return "FAIL", f"no tool named {tool_name!r} in the surface"
    tool = tools[tool_name]

    want_modes = claim.get("modes") or []
    if want_modes:
        have = tool.get("modes")
        # None IS NOT []. The audit records None when a tool did not enumerate
        # its modes, and reporting PASS off a list we never got would be the
        # documentation-reading failure this whole audit exists to end.
        if have is None:
            return "UNVERIFIED", f"{tool_name} did not enumerate its modes, so {want_modes} is unchecked"
        missing = [m for m in want_modes if m not in have]
        if missing:
            return "FAIL", f"{tool_name} lacks mode(s) {missing}; it has {have}"

    want_props = claim.get("properties") or []
    missing = [p for p in want_props if p not in tool.get("properties", [])]
    if missing:
        return "FAIL", f"{tool_name} lacks propert(ies) {missing}"

    want_text = claim.get("description_contains") or []
    desc = (tool.get("description") or "").lower()
    missing = [w for w in want_text if w.lower() not in desc]
    if missing:
        return "FAIL", f"{tool_name}'s description does not mention {missing}"

    return "PASS", f"{tool_name}"


def behavior_verdict(claim, surface):
    """Decide a claim from the captured behaviors.

    An ABSENT behavior is UNVERIFIED, never PASS. A surface captured before
    behaviors existed has nothing to say about them, and a missing key must not
    read as a satisfied `behavior_lacks`.
    """
    behaviors = surface.get("behaviors") or {}
    if not behaviors:
        return "UNVERIFIED", "the surface carries no behaviors; recapture it with a current audit"
    for key, want in (claim.get("behavior_has") or {}).items():
        if key not in behaviors:
            return "UNVERIFIED", f"the surface has no behavior {key!r}"
        have = behaviors[key].get("hint_codes") or []
        missing = [w for w in want if w not in have]
        if missing:
            return "FAIL", f"{key} did not report {missing}; it reported {have}"
    for key, unwanted in (claim.get("behavior_lacks") or {}).items():
        if key not in behaviors:
            return "UNVERIFIED", f"the surface has no behavior {key!r}, so the control is unchecked"
        have = behaviors[key].get("hint_codes") or []
        present = [w for w in unwanted if w in have]
        if present:
            return "FAIL", f"the control {key} DID report {present}, so the signal means nothing"
    for key, want_n in (claim.get("behavior_error_count") or {}).items():
        if key not in behaviors:
            return "UNVERIFIED", f"the surface has no behavior {key!r}"
        got = behaviors[key].get("error_count")
        if got != want_n:
            return "FAIL", f"{key} reported {got} error(s), expected {want_n}"
    return "PASS", "measured against the live provider"


def main():
    if len(sys.argv) != 3:
        print("usage: check-provider-claims.py <claims.toml> <surface.json>", file=sys.stderr)
        return 64
    claims_path, surface_path = Path(sys.argv[1]), Path(sys.argv[2])
    if not claims_path.is_file():
        print(f"claims: no claims file at {claims_path}; REFUSING -- a check with no claims "
              f"passes everything", file=sys.stderr)
        return E_BLIND
    if not surface_path.is_file():
        print(f"claims: no surface at {surface_path}. That is NOT 'no claims failed' -- "
              f"run tools/provider-audit.py against the pinned provider first", file=sys.stderr)
        return E_BLIND

    claims = tomllib.loads(claims_path.read_text(encoding="utf-8")).get("claim", [])
    surface = json.loads(surface_path.read_text(encoding="utf-8"))
    if not claims:
        print("claims: the claims file holds NO claims; REFUSING", file=sys.stderr)
        return E_BLIND
    if not surface.get("tools"):
        print("claims: the surface holds NO tools; every claim would fail for one reason "
              "and that reason is not the provider; REFUSING", file=sys.stderr)
        return E_BLIND

    si = surface.get("server_info", {})
    print(f"CLAIMS against {si.get('name')} {si.get('version')}, "
          f"{surface.get('tool_count')} tools, protocol {surface.get('protocol_version')}")
    tally = {"PASS": 0, "FAIL": 0, "UNVERIFIED": 0, "RUN": 0, "META": 0}
    bad = []
    for c in claims:
        kind = c.get("kind")
        line, subject = c.get("spec_line"), c.get("subject")
        if kind == "meta":
            tally["META"] += 1
            print(f"  META       L{line:<4} {subject}")
            continue
        if kind == "behavior":
            status, detail = behavior_verdict(c, surface)
            tally[status] += 1
            print(f"  {status:<10} L{line:<4} {subject} -- {detail}")
            if status == "FAIL":
                bad.append((line, subject, detail))
            continue
        if kind == "run":
            tally["RUN"] += 1
            print(f"  RUN-ONLY   L{line:<4} {subject} -- needs a live call, not a shape")
            continue
        status, detail = verdict(c, surface)
        tally[status] += 1
        print(f"  {status:<10} L{line:<4} {subject} -- {detail}")
        if status == "FAIL":
            bad.append((line, subject, detail))

    print(f"-- {tally['PASS']} measured, {tally['FAIL']} failed, "
          f"{tally['UNVERIFIED']} unverified, {tally['RUN']} run-only, {tally['META']} meta")
    if bad:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
