#!/usr/bin/env bash
# Fails if any installed npm production dependency has a licence that is not allowed. Whole SPDX
# expressions are judged (scripts/js_license_eval.py): AND needs every part allowed, OR needs one;
# LGPL parts only for listed dynamically loaded libraries. HELQIX's own @helqix/* packages are
# excluded: their licence is a founder decision (docs/TOOLS.md).
# Usage: scripts/js-license-check.sh <app-dir>   (runs `npm ci` there if node_modules is missing)
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
PY="${HELQIX_PYTHON:-python3}"
cd "$1"
[ -d node_modules ] || npm ci --no-audit --no-fund
npx --yes license-checker-rseidelsohn@5.0.1 --production --excludePrivatePackages --json > "${TMPDIR:-/tmp}/js-licences.json"
"$PY" -I "$here/js_license_eval.py" < "${TMPDIR:-/tmp}/js-licences.json"
