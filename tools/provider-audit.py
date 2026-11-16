#!/usr/bin/env python3
"""Speak MCP to the PINNED provider and record what it actually has.

WHY THIS EXISTS. docs/spec.md §12 says it plainly: "every [exists] badge was
read from the provider's own API.md, not run." Fifteen claims about a
dependency, taken from its documentation. This runs it instead.

TWO SERVERS ANSWER TO "openscad-mcp" ON THIS MACHINE and that is how the audit
got written. One is the pinned `OpenSCAD MCP Server` 4.2.0 with 12 tools; the
other is an unrelated `openscad-mcp` 1.0.0 with 11 render-only tools that
happened to be wired into an agent session. Reading the second and reporting on
the first produced a confident, wrong conclusion that the whole provider
boundary was fiction. So this tool launches the provider ITSELF, from
provider/ through uv and the lockfile, and reconciles serverInfo against
PROVIDER.toml before it reports anything. A surface captured from an
unidentified server is worth less than no surface at all.

usage: provider-audit.py <out.json> [--provider-dir DIR]

exit codes, each DISTINCT because they mean different things:
  0  surface captured and serverInfo matches PROVIDER.toml
  3  the provider could not be reached or did not speak MCP
  4  serverInfo disagrees with PROVIDER.toml
  5  the provider answered with no tools -- a surface nothing can fail against
"""
import json
import re
import subprocess
import sys
from pathlib import Path

E_UNREACHABLE = 3
E_IDENTITY = 4
E_EMPTY = 5


def read_pin(provider_dir):
    """The two version numbers and the protocol, from PROVIDER.toml."""
    text = (provider_dir / "PROVIDER.toml").read_text(encoding="utf-8")
    out = {}
    for key in ("server_info_name", "server_info_version", "mcp_protocol", "version", "name"):
        m = re.search(rf'^{key}\s*=\s*"([^"]*)"', text, re.M)
        if m:
            out[key] = m.group(1)
    return out


def identity_matches(pin, server):
    """Is the server answering the one PROVIDER.toml pins?

    A PURE FUNCTION, separated from the launch so it can be driven without a
    live provider. The launch needs uv, a resolved lockfile and OpenSCAD and
    cannot run in the forge jail; this comparison is the part that was wrong in
    practice, so it is the part that has to be testable everywhere.
    """
    return (server.get("name") == pin.get("server_info_name")
            and server.get("version") == pin.get("server_info_version"))


def speak(provider_dir, timeout=180):
    """Launch the pinned provider and return (serverInfo, protocol, tools)."""
    proc = subprocess.Popen(
        ["uv", "run", "--frozen", "--no-dev", "openscad-mcp"],
        cwd=str(provider_dir),
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )

    def send(obj):
        proc.stdin.write(json.dumps(obj) + "\n")
        proc.stdin.flush()

    def read(want_id):
        # A BANNER OR A LOG LINE IS NOT A REPLY. The client skips anything that
        # is not JSON with the id it asked for, rather than parsing the first
        # line and calling whatever it found the answer.
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
            if msg.get("id") == want_id:
                return msg

    try:
        send({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {"protocolVersion": "2024-11-05", "capabilities": {},
                       "clientInfo": {"name": "wm-provider-audit", "version": "0"}},
        })
        init = read(1)
        if init is None:
            return None, None, None, "the provider exited or never answered initialize", 0, {}
        result = init.get("result", {})
        send({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
        send({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
        listed = read(2)
        if listed is None:
            return None, None, None, "the provider answered initialize and not tools/list", 0, {}
        tools = listed.get("result", {}).get("tools", []) or []

        # THE SCHEMA SIZE IS MEASURED BEFORE ANYTHING IS ANNOTATED ONTO IT.
        # The mode answers below are attached to these same dicts, and
        # serializing them afterwards measured my own annotation as part of the
        # provider's schema: 20321 bytes became 20795 and the larger number
        # looked just as much like a reading.
        schema_bytes = len(json.dumps(tools, sort_keys=True))

        # ASK EACH MODE-BEARING TOOL TO ENUMERATE ITSELF.
        next_id = 10
        for t in tools:
            props = (t.get("inputSchema") or {}).get("properties", {})
            if "mode" not in props:
                t["_wm_modes"] = []
                continue
            next_id += 1
            send({"jsonrpc": "2.0", "id": next_id, "method": "tools/call",
                  "params": {"name": t["name"],
                             "arguments": {"mode": SENTINEL_MODE,
                                           "scad_content": "cube(1);"}}})
            t["_wm_modes"] = modes_from_reply(read(next_id) or {})

        # BEHAVIORS, because some claims are not shapes.
        #
        # The spec's L436 badge says an empty result is REPORTED as empty
        # rather than passing silently. Nothing in tools/list can answer that:
        # it is what the provider does when handed geometry that renders to
        # nothing. Captured here so the claim is decided by a measurement and
        # CI still needs no live provider.
        behaviors = {}
        for key, src in (
            ("empty_3d", "difference(){cube(10);cube(10);}"),
            ("empty_2d", "difference(){square(10);square(10);}"),
            ("nonempty_3d", "cube(10);"),
        ):
            next_id += 1
            send({"jsonrpc": "2.0", "id": next_id, "method": "tools/call",
                  "params": {"name": "validate",
                             "arguments": {"mode": "geometry", "scad_content": src}}})
            reply = read(next_id) or {}
            sc = (reply.get("result", {}) or {}).get("structuredContent") or {}
            behaviors[key] = {
                "source": src,
                "hint_codes": sorted({h.get("code") for h in sc.get("hints", []) if h.get("code")}),
                "error_count": len(sc.get("errors", []) or []),
                "success": sc.get("success"),
            }

        return (result.get("serverInfo", {}), result.get("protocolVersion"),
                tools, None, schema_bytes, behaviors)
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()


# MODES ARE ASKED FOR, NOT PARSED OUT OF PROSE.
#
# `mode` is a bare string in the JSON schema with a default and no enum, so the
# schema cannot answer this. The first version of this tool read the modes out
# of each tool's description with a regex and found 4 of measure's 9 and 2 of
# check's 6 -- missing `probe`, `features` and `motion`, which are precisely
# the three capabilities this audit exists to verify. A surface built from that
# would have reported those claims FAIL and produced the same confident wrong
# conclusion a second time, by a new route.
#
# The provider answers the question directly: call a mode-bearing tool with a
# mode it does not have, and it replies "mode must be one of: ...". That is the
# server enumerating itself, which is the only account of its modes that cannot
# drift from its behavior.
MODE_LIST_RE = re.compile(r"mode must be one of:\s*([a-z_, ]+)")
SENTINEL_MODE = "__wm_audit_invalid__"


def modes_from_reply(msg):
    """Pull the enumerated modes out of a refusal, or None if it did not say.

    None and [] ARE DIFFERENT and must stay different: [] claims the tool has
    no modes, None says this tool did not tell us. Collapsing them turns "I
    could not read the answer" into a finding about the provider.
    """
    blob = json.dumps(msg)
    m = MODE_LIST_RE.search(blob)
    if not m:
        return None
    return [x.strip() for x in m.group(1).split(",") if x.strip()]


def main():
    args = [a for a in sys.argv[1:]]
    provider_dir = Path("provider")
    if "--provider-dir" in args:
        i = args.index("--provider-dir")
        provider_dir = Path(args[i + 1])
        del args[i:i + 2]
    if not args:
        print(__doc__.strip().splitlines()[-6], file=sys.stderr)
        print("usage: provider-audit.py <out.json> [--provider-dir DIR]", file=sys.stderr)
        return 64
    out_path = Path(args[0])

    if not (provider_dir / "PROVIDER.toml").is_file():
        print(f"provider-audit: no PROVIDER.toml under {provider_dir}; "
              f"refusing rather than auditing an unidentified server", file=sys.stderr)
        return E_UNREACHABLE
    pin = read_pin(provider_dir)

    server, protocol, tools, err, schema_bytes, behaviors = speak(provider_dir)
    if err:
        print(f"provider-audit: UNREACHABLE -- {err}. This is NOT 'no claims failed': "
              f"a provider that did not answer has verified nothing", file=sys.stderr)
        return E_UNREACHABLE

    # IDENTITY BEFORE CONTENT. Capturing a surface from a server that is not
    # the pinned one is how this audit's first draft concluded the provider
    # boundary was fiction.
    want_name = pin.get("server_info_name")
    want_ver = pin.get("server_info_version")
    if not identity_matches(pin, server):
        print(f"provider-audit: IDENTITY MISMATCH -- PROVIDER.toml pins "
              f"{want_name!r} {want_ver!r}, the server answering says "
              f"{server.get('name')!r} {server.get('version')!r}. Two different servers "
              f"answer to this name; auditing the wrong one is worse than not auditing",
              file=sys.stderr)
        return E_IDENTITY

    if not tools:
        print("provider-audit: the provider listed NO tools. A surface with nothing in it "
              "passes every claim by vacuity, so it is refused", file=sys.stderr)
        return E_EMPTY

    surface = {
        "server_info": server,
        "protocol_version": protocol,
        "pinned": {"version": pin.get("version"), "mcp_protocol": pin.get("mcp_protocol")},
        "behaviors": behaviors,
        "tool_count": len(tools),
        "schema_bytes": schema_bytes,
        "tools": {
            t["name"]: {
                "description": (t.get("description") or "").strip(),
                "modes": t.get("_wm_modes"),
                "properties": sorted((t.get("inputSchema") or {}).get("properties", {}).keys()),
                "required": sorted((t.get("inputSchema") or {}).get("required", []) or []),
            }
            for t in sorted(tools, key=lambda t: t["name"])
        },
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(surface, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    print(f"provider-audit: {server['name']} {server['version']}, protocol {protocol}, "
          f"{len(tools)} tools, {schema_bytes} schema bytes -> {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
