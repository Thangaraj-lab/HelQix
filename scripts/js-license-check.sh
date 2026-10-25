#!/usr/bin/env bash
# Fails if any installed npm production dependency has a licence outside the allowlist or none at all.
# HELQIX's own @helqix/* packages are excluded: their licence is a founder decision (docs/TOOLS.md).
# Usage: scripts/js-license-check.sh <app-dir>   (runs `npm ci` there if node_modules is missing)
set -euo pipefail
app="$1"
allow="MIT;MIT-0;Apache-2.0;BSD-2-Clause;BSD-3-Clause;ISC;MPL-2.0;0BSD;CC0-1.0;Zlib;BlueOak-1.0.0;Python-2.0;Unlicense;CC-BY-4.0"
cd "$app"
[ -d node_modules ] || npm ci --no-audit --no-fund
npx --yes license-checker-rseidelsohn@5.0.1 --production --excludePrivatePackages --excludePackagesStartingWith "@helqix/" --onlyAllow "$allow" --summary
