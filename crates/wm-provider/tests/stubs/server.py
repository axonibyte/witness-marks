#!/usr/bin/env python3
"""A stub MCP server, so the REAL client can be driven without a Python
environment, OpenSCAD, or a network.

The behaviours are chosen because each one broke a client somewhere:

  banner       emit a non-JSON line before the first answer, the way the real
               provider prints a security warning
  interleave   emit an unsolicited notification BEFORE the answer, so a client
               that takes the first line gets the wrong one
  wrongver     advertise a serverInfo version the pin does not name
  wrongname    advertise a different server name
  wrongproto   advertise a different protocol version
  silent       answer initialize and then never answer again, so the timeout
               path is driven rather than assumed
  texterror    return a tools/call result with isError and a non-JSON text body
"""
import json
import os
import sys

# THE MODE IS AN ARGUMENT AND NOT AN ENVIRONMENT VARIABLE, and that is a fix
# rather than a preference. It was $STUB_MODE, set with std::env::set_var in
# the test helper -- and cargo runs tests as THREADS in one process, so every
# test shared the variable and raced. Four of nine failed in a batch and all
# nine passed one at a time, which is the signature. An argument belongs to the
# process it starts.
MODE = sys.argv[1] if len(sys.argv) > 1 else "plain"
NAME = "Some Other Server" if MODE == "wrongname" else "OpenSCAD MCP Server"
VER = "9.9.9" if MODE == "wrongver" else "4.2.0"
PROTO = "2099-01-01" if MODE == "wrongproto" else "2024-11-05"


def out(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


def main():
    if MODE == "banner":
        sys.stdout.write("security.allowed_paths is not set: not JSON at all\n")
        sys.stdout.flush()
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        req = json.loads(line)
        method = req.get("method")
        if method == "initialize":
            out({"jsonrpc": "2.0", "id": req["id"], "result": {
                "protocolVersion": PROTO,
                "capabilities": {},
                "serverInfo": {"name": NAME, "version": VER}}})
        elif method == "notifications/initialized":
            continue
        elif MODE == "silent":
            continue  # answer nothing: drive the timeout
        elif method == "tools/list":
            out({"jsonrpc": "2.0", "id": req["id"], "result": {"tools": [
                {"name": "measure", "description": "d", "inputSchema": {"type": "object"}}]}})
        elif method == "tools/call":
            if MODE == "interleave":
                out({"jsonrpc": "2.0", "method": "notifications/message",
                     "params": {"level": "info", "data": "a notification before the answer"}})
                out({"jsonrpc": "2.0", "id": 9999, "result": {"content": [
                    {"type": "text", "text": "{\"volume\": -1.0}"}]}})
            if MODE == "texterror":
                out({"jsonrpc": "2.0", "id": req["id"], "result": {
                    "isError": True,
                    "content": [{"type": "text", "text": "1 validation error for call[measure]"}]}})
                continue
            out({"jsonrpc": "2.0", "id": req["id"], "result": {
                "structuredContent": {
                    "volume": 1000.0, "empty": False,
                    "errors": [], "warnings": ["a stub warning"],
                    "hints": [{"code": "empty_output", "hint": "a stub hint"}],
                    "deprecated": []},
                "content": [{"type": "text", "text": "{\"volume\": 999.0}"}]}})
        else:
            out({"jsonrpc": "2.0", "id": req.get("id", 0),
                 "error": {"code": -32601, "message": "no such method"}})


if __name__ == "__main__":
    main()
