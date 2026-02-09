#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

failures=0

run_case() {
  local name="$1"
  local cmd="$2"
  echo "==> ${name}"
  if bash -lc "${cmd}"; then
    echo "PASS: ${name}"
  else
    echo "FAIL: ${name}"
    failures=$((failures + 1))
  fi
  echo
}

run_case "Cargo Check" "cargo check"
run_case "Regression Check" "scripts/regression_check.sh"
run_case "Live Fire Test" "scripts/live_fire_test.sh"

if [[ ${failures} -ne 0 ]]; then
  echo "TEST SUITE FAILED (${failures} case(s))"
  exit 1
fi

echo "TEST SUITE PASSED"
