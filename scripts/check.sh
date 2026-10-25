#!/usr/bin/env bash
# `make check`: build, lint, tests and licence gates. Exits non-zero on the first failing stage.
# Needs: cargo, cargo-deny, node/npm, Python with pytest. PyBaMM suites run only when HELQIX_PYTHON has pybamm.
set -euo pipefail
cd "$(dirname "$0")/.."
PY="${HELQIX_PYTHON:-python3}"
step() { printf '\n== %s\n' "$*"; }

step "Rust build"
cargo build --workspace --locked
step "Rust lint (clippy: errors fail, warnings reported)"
cargo clippy --workspace --locked --quiet
step "Rust tests"
cargo test --workspace --locked --quiet
step "Contract tests (serde fixtures, committed schemas match the code)"
cargo test --locked --quiet -p helqix-contracts --features schema
step "Rust licence, advisory and source gate (cargo-deny)"
cargo deny --locked check licenses bans sources
cargo deny --locked check advisories || { echo "advisories: see output above (network required)"; exit 1; }
step "JS licence gate (reality-studio dependencies)"
scripts/js-license-check.sh 21-app/reality-studio
step "Tool register: every tool has a verified licence and a verdict"
"$PY" -I scripts/check_tools_lock.py tools/TOOLS.lock.json
step "Python tests"
"$PY" -m pytest -q 15-sdk/platform/contracts/tests_py
"$PY" -m pytest -q scripts/tests tools/assets/tests
"$PY" -I tools/assets/check_licenses.py examples/assets
(cd 19-intelligence/python && "$PY" -m pytest -q tests)
(cd 15-sdk/platform/sdk/python && "$PY" -m pytest -q tests)
(cd 18-libraries/python && "$PY" -m pytest -q tests)
if "$PY" -c "import pybamm" 2>/dev/null; then
  (cd 18-libraries/python && "$PY" -m pytest -q tests_pybamm)
  "$PY" -m pytest -q 23-validation/benchmarks/battery
else
  echo "PyBaMM not installed for $PY: PyBaMM suites skipped (they run in CI)"
fi
printf '\ncheck: PASS\n'
