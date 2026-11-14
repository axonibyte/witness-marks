#!/usr/bin/env python3
"""Create the labels, milestones and issues in issues/seed.json on the forge.

Idempotent: existing labels/milestones/issues are matched by name/title and
skipped. Dry-run by default; pass --apply to write. The credential is a curl
config file (a `header = "Authorization: token ..."` line) named by
$FORGE_CURLRC and handed to curl by PATH -- never read into this process,
never printed, never on argv. Stdlib only.
"""
import json, os, subprocess, sys, urllib.parse
BASE = os.environ.get("FORGE_API", "https://forge.axonibyte.com/api/v1")
REPO = os.environ.get("FORGE_REPO", "axonibyte/witness-marks")
RC = os.environ.get("FORGE_CURLRC")
APPLY = "--apply" in sys.argv

def api(method, path, body=None):
    cmd = ["curl", "-s", "-m", "20", "-K", RC, "-X", method, "-H", "Content-Type: application/json", f"{BASE}/repos/{REPO}{path}"]
    if body is not None: cmd += ["-d", json.dumps(body)]
    out = subprocess.run(cmd, capture_output=True, text=True, check=False).stdout
    try: return json.loads(out) if out else {}
    except json.JSONDecodeError: sys.exit(f"non-JSON from forge on {method} {path}: {out[:200]}")

def main():
    seed = json.load(open(os.path.join(os.path.dirname(__file__), "..", "issues", "seed.json")))
    if not RC or not os.path.exists(RC): sys.exit("FORGE_CURLRC must name a curl config holding the Authorization header")
    labels = {l["name"]: l["id"] for l in api("GET", "/labels?limit=100")}
    for name in seed["labels"]:
        if name in labels: continue
        print(f"label    + {name}")
        if APPLY: labels[name] = api("POST", "/labels", {"name": name, "color": "8a6d2f" if name.startswith("badge") else "1d4e6f"})["id"]
    ms = {m["title"]: m["id"] for m in api("GET", "/milestones?state=all&limit=100")}
    for title in seed["milestones"]:
        if title in ms: continue
        print(f"milestone+ {title}")
        if APPLY: ms[title] = api("POST", "/milestones", {"title": title})["id"]
    existing = {i["title"] for i in api("GET", "/issues?state=all&type=issues&limit=200")} if isinstance(api("GET", "/issues?limit=1"), list) else set()
    n = 0
    for it in seed["issues"]:
        if it["title"] in existing: continue
        n += 1; print(f"issue    + [{it['milestone']}] {it['title']}")
        if APPLY:
            api("POST", "/issues", {"title": it["title"], "body": it["body"], "labels": [labels[l] for l in it["labels"] if l in labels], "milestone": ms.get(it["milestone"])})
    print(f"{'created' if APPLY else 'would create'} {n} issues" + ("" if APPLY else "  (pass --apply)"))

if __name__ == "__main__": main()
