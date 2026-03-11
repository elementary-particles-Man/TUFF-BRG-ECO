#!/usr/bin/env bash
set -euo pipefail

WAL_PATH="${1:-tuff-db-lightweight.wal}"
if [[ ! -f "${WAL_PATH}" ]]; then
  echo "ERROR: WAL not found: ${WAL_PATH}" >&2
  exit 1
fi

if [[ ! -s "${WAL_PATH}" ]]; then
  echo "SKIP: WAL is empty: ${WAL_PATH}"
  exit 0
fi

backup="${WAL_PATH}.bak.$(date +%Y%m%d%H%M%S)"
cp "${WAL_PATH}" "${backup}"

tmp_pairs="$(mktemp)"
tmp_out="$(mktemp)"
trap 'rm -f "$tmp_pairs" "$tmp_out"' EXIT

# Keep only the latest line per tag. Output order follows latest appearance order.
awk '
{
  line=$0
  t=index(line, "\t")
  if (t <= 0) {
    bad[++bad_n]=line
    next
  }
  tag=substr(line, 1, t-1)
  seen[tag]=NR
  rec[tag]=line
}
END {
  for (k in rec) {
    printf "%d\t%s\n", seen[k], rec[k]
  }
}
' "${WAL_PATH}" | sort -n -k1,1 > "${tmp_pairs}"

cut -f2- "${tmp_pairs}" > "${tmp_out}"

before_lines=$(wc -l < "${WAL_PATH}" | tr -d ' ')
after_lines=$(wc -l < "${tmp_out}" | tr -d ' ')

mv "${tmp_out}" "${WAL_PATH}"

echo "NORMALIZED: ${WAL_PATH}"
echo "BACKUP: ${backup}"
echo "LINES: ${before_lines} -> ${after_lines}"
