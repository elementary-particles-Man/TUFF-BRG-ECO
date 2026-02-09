#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ADDON_DIR="${ROOT_DIR}/tuff-brg/addon"
OUT_DIR="${ROOT_DIR}/dist"

if [[ ! -f "${ADDON_DIR}/manifest.json" ]]; then
  echo "ERROR: manifest.json not found at ${ADDON_DIR}" >&2
  exit 1
fi

if ! command -v zip >/dev/null 2>&1; then
  echo "ERROR: zip command is required" >&2
  exit 1
fi

version="$(node -e "const fs=require('fs');const m=JSON.parse(fs.readFileSync(process.argv[1],'utf8'));console.log(m.version||'0.0.0');" "${ADDON_DIR}/manifest.json")"
archive="tuff-brg-addon-v${version}.zip"

mkdir -p "${OUT_DIR}"
rm -f "${OUT_DIR}/${archive}"

(
  cd "${ADDON_DIR}"
  zip -r "${OUT_DIR}/${archive}" . \
    -x "*.map" \
    -x ".*" \
    -x "*/.*"
)

echo "PACKED: ${OUT_DIR}/${archive}"
