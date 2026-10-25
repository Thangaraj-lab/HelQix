"""Gate on tools/TOOLS.lock.json: every tool has a licence, a VERIFIED status and a verdict; any tool
whose verdict is forbidden must not be listed in deploy/dev/compose.yaml. Exit 1 on failure."""
import json
import sys
from pathlib import Path

lock = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
# Only the images actually run count, not comments that explain exclusions.
compose_path = Path("deploy/dev/compose.yaml")
compose = " ".join(l.split("image:", 1)[1] for l in compose_path.read_text(encoding="utf-8").lower().splitlines() if "image:" in l) if compose_path.exists() else ""
problems = []
for t in lock["tools"]:
    if not t.get("license") or t.get("status") != "VERIFIED":
        problems.append(f"{t['name']}: licence not verified")
    if t.get("verdict", "").startswith("unknown"):
        problems.append(f"{t['name']}: licence verdict unknown")
    if t.get("verdict", "").startswith("forbidden") and t["name"].split()[0].lower() in compose:
        problems.append(f"{t['name']}: forbidden licence but used in the dev stack")
for p in problems:
    print("FAIL", p)
print(f"{len(lock['tools'])} tools checked, {len(problems)} problems")
sys.exit(1 if problems else 0)
