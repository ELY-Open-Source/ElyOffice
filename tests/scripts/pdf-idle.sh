#!/usr/bin/env bash
# ElyPDF with no file starts no MuPDF worker (PERF-01).
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
(cd "$root" && cargo build -q -p ely-pdf -p ely-pdf-worker)
ELY_NO_DIALOG=1 ELY_SMOKE=hold "$root/target/debug/ely-pdf" >/dev/null 2>&1 &
app=$!
trap 'kill $app 2>/dev/null || true' EXIT
sleep 3
kill -0 "$app" || { echo "FAIL: ElyPDF did not stay up"; exit 1; }
if pgrep -P "$app" -f ely-pdf-worker >/dev/null; then
  echo "FAIL: a worker runs with no file open"
  exit 1
fi
echo "pdf-idle: no worker before the first file"
