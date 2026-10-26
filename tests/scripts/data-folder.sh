#!/usr/bin/env bash
# A chosen data folder takes the data when ElyMark starts.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# Windows paths with forward slashes need no escaping.
native() { if command -v cygpath >/dev/null; then cygpath -m "$1"; else printf '%s' "$1"; fi; }

cargo build -q -p ely-mark --manifest-path "$root/Cargo.toml"
home="$work/first/ElyOffice"
next="$work/away/ElyOffice"
mkdir -p "$home/ely-mark" "$work/away"
printf 'kept' >"$home/ely-mark/marker"
printf '{"version": 1, "folder": {"next": "%s"}}\n' "$(native "$next")" >"$home/folder.json"
ELY_DATA_ROOT=$(native "$home") ELY_LOG=info ELY_LOCALE=en ELY_NO_DIALOG=1 ELY_SMOKE=1 \
  timeout 120 "$root/target/debug/ely-mark" >"$work/out" 2>&1 || { echo "FAIL: ElyMark did not start"; tail -20 "$work/out"; exit 1; }
[ "$(cat "$next/ely-mark/marker" 2>/dev/null)" = kept ] || { echo "FAIL: the data did not move"; tail -20 "$work/out"; exit 1; }
[ ! -e "$home/ely-mark" ] || { echo "FAIL: the old data stayed"; exit 1; }
grep -q "data folder: moved" "$next/ely-mark/ely.log" || { echo "FAIL: the move is not in the log"; exit 1; }
grep -q '"at"' "$home/folder.json" || { echo "FAIL: the pointer did not switch"; exit 1; }
mv "$next" "$work/away/gone"
if ELY_DATA_ROOT=$(native "$home") ELY_LOCALE=en ELY_NO_DIALOG=1 ELY_SMOKE=1 timeout 120 "$root/target/debug/ely-mark" >"$work/out" 2>&1; then
  echo "FAIL: ElyMark started without its data folder"; exit 1
fi
grep -q "is missing" "$work/out" || { echo "FAIL: the missing folder is not named"; tail -20 "$work/out"; exit 1; }
[ ! -e "$next" ] || { echo "FAIL: an empty data folder was made"; exit 1; }
echo "ok: ElyMark moved its data into the chosen folder at start and logged it, and refuses to start once it is gone"
