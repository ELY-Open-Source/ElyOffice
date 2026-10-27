#!/usr/bin/env bash
# ElyTable's cells stay white in dark and light mode.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

cargo build -q -p ely-lok --bin ely-lok-worker --manifest-path "$root/Cargo.toml"
cargo build -q -p ely-table --manifest-path "$root/Cargo.toml"
swiftc -O -o "$work/shade" "$root/tests/scripts/window-shade.swift"
shade_of() {
  local mode=$1 home="$work/$1"
  mkdir -p "$home"
  ELY_SMOKE=hold ELY_LOCALE=en ELY_APPEARANCE="$mode" HOME="$home" XDG_DATA_HOME="$home/data" \
    perl -e 'alarm 120; exec @ARGV' "$root/target/debug/ely-table" >"$home/out" 2>"$home/err" &
  local pid=$!
  trap 'kill "$pid" 2>/dev/null; wait "$pid" 2>/dev/null || true' RETURN
  local log=
  for _ in $(seq 200); do
    log=$(find "$home" -path '*/ElyOffice/ely-table/ely.log' 2>/dev/null | head -n 1)
    [ -n "$log" ] && grep -q "table: opened" "$log" && break
    sleep 0.2
  done
  grep -q "table: opened" "$log" || fail "$mode: no workbook: $(tail -n 3 "$home/err")"
  # Tiles paint after the first frames.
  sleep 3
  local shade
  shade=$("$work/shade" "$pid")
  echo "$shade"
}
# At top level, so SKIP and FAIL end the run.
check() {
  local mode=$1 seen=$2 lines
  [ "$seen" != denied ] || { echo "SKIP: needs Screen Recording permission"; exit 0; }
  [ "$seen" != none ] || fail "$mode: no window"
  read -r shade lines <<<"$seen"
  # Only kernel tiles draw grid lines.
  [ "$lines" -ge 5 ] || fail "$mode: no grid lines ($lines per mille)"
}
check dark "$(shade_of dark)"
dark=$shade
check light "$(shade_of light)"
light=$shade
[ "$dark" -gt 200 ] || fail "the dark app's grid paints at $dark"
[ "$light" -gt 200 ] || fail "the light app's grid paints at $light"
echo "ok: theme (dark $dark, light $light)"
