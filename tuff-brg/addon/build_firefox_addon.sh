#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUT_DIR="${ROOT_DIR}/.firefox-build"

rm -rf "${OUT_DIR}"
mkdir -p "${OUT_DIR}"

cp -a "${ROOT_DIR}/background.js" "${OUT_DIR}/"
cp -a "${ROOT_DIR}/booster.js" "${OUT_DIR}/"
cp -a "${ROOT_DIR}/options.html" "${OUT_DIR}/"
cp -a "${ROOT_DIR}/options.js" "${OUT_DIR}/"
cp -a "${ROOT_DIR}/popup.html" "${OUT_DIR}/"
cp -a "${ROOT_DIR}/popup.js" "${OUT_DIR}/"
cp -a "${ROOT_DIR}/manifest.firefox.json" "${OUT_DIR}/manifest.json"

echo "Firefox addon build dir: ${OUT_DIR}"
echo "Load in Firefox: about:debugging -> このFirefox -> 一時的なアドオンを読み込む -> ${OUT_DIR}/manifest.json"
