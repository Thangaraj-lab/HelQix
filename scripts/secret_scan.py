"""Fail if a tracked file looks like it contains a secret. No dependencies.

Usage: python -I scripts/secret_scan.py [repo-root]    (scans `git ls-files`)

Patterns are deliberately few and specific, so a hit is worth a human look: private key blocks, cloud and
code-host tokens, and long values assigned to names such as api_key, secret, password, token. A line may be
allowed with the marker `secret-scan: allow` when it is a documented test value.
"""
import pathlib
import re
import subprocess
import sys

PATTERNS = {
    "private key block": re.compile(r"-----BEGIN (?:RSA |EC |DSA |OPENSSH |PGP |ENCRYPTED )?PRIVATE KEY"),
    "AWS access key id": re.compile(r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b"),
    "GitHub token": re.compile(r"\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{30,}\b|\bgithub_pat_[A-Za-z0-9_]{40,}\b"),
    "Slack token": re.compile(r"\bxox[abprs]-[A-Za-z0-9-]{10,}\b"),
    "Google API key": re.compile(r"\bAIza[0-9A-Za-z_-]{35}\b"),
    "assigned secret": re.compile(r"""(?i)\b(?:api[_-]?key|secret|passwd|password|token)\b["']?\s*[:=]\s*["'][A-Za-z0-9/+_=.-]{20,}["']"""),
}
SKIP_SUFFIXES = {".png", ".jpg", ".jpeg", ".gif", ".webp", ".glb", ".hdr", ".parquet", ".wasm", ".woff", ".woff2", ".lock", ".svg"}
ALLOW = "secret-scan: allow"
MAX_BYTES = 2_000_000


def scan_text(name: str, text: str) -> list[str]:
    hits = []
    for number, line in enumerate(text.splitlines(), 1):
        if ALLOW in line:
            continue
        for label, pattern in PATTERNS.items():
            if pattern.search(line):
                hits.append(f"{name}:{number}: {label}")
    return hits


def main(root: pathlib.Path) -> int:
    files = subprocess.run(["git", "ls-files", "-z"], cwd=root, capture_output=True, check=True).stdout.decode().split("\0")
    hits: list[str] = []
    for rel in filter(None, files):
        path = root / rel
        if path.suffix.lower() in SKIP_SUFFIXES or not path.is_file() or path.stat().st_size > MAX_BYTES:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        hits += scan_text(rel, text)
    for h in hits:
        print(h)
    print(f"secret scan: {len(hits)} finding(s)")
    return 1 if hits else 0


if __name__ == "__main__":
    sys.exit(main(pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()))
