#!/usr/bin/env bash
# ElyTable's cells stay white in dark and light mode.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
work=$(mktemp -d)
app= worker=
# The worker writes its profile while it exits.
stop() {
  [ -n "$app" ] || return 0
  [ -n "$worker" ] || worker=$(pgrep -P "$app" -x ely-lok-worker || true)
  kill "$app" 2>/dev/null || true
  wait "$app" 2>/dev/null || true
  app=
  [ -n "$worker" ] || return 0
  for _ in $(seq 150); do kill -0 "$worker" 2>/dev/null || { worker=; return 0; }; sleep 0.1; done
  echo "FAIL: worker $worker outlived the app by 15 s" >&2
  return 1
}
trap 'stop || true; rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

cargo build -q -p ely-lok --bin ely-lok-worker --manifest-path "$root/Cargo.toml"
cargo build -q -p ely-table --manifest-path "$root/Cargo.toml"
swiftc -O -o "$work/shade" "$root/tests/scripts/window-shade.swift"
# In this shell, so a failure stops the app.
measure() {
  local mode=$1 home="$work/$1" log=
  mkdir -p "$home"
  ELY_SMOKE=hold ELY_LOCALE=en ELY_APPEARANCE="$mode" HOME="$home" XDG_DATA_HOME="$home/data" \
    perl -e 'alarm 120; exec @ARGV' "$root/target/debug/ely-table" >"$home/out" 2>"$home/err" &
  app=$!
  for _ in $(seq 200); do
    log=$(find "$home" -path '*/ElyOffice/ely-table/ely.log' 2>/dev/null | head -n 1)
    [ -n "$log" ] && grep -q "table: opened" "$log" && break
    sleep 0.2
  done
  [ -n "$log" ] && grep -q "table: opened" "$log" || fail "$mode: no workbook: $(tail -n 3 "$home/err")"
  worker=$(pgrep -P "$app" -x ely-lok-worker) || fail "$mode: no worker"
  # Tiles paint after the first frames.
  sleep 3
  seen=$("$work/shade" "$app")
  stop || exit 1
  [ -n "$seen" ] || fail "$mode: not measured"
  [ "$seen" != denied ] || { echo "SKIP: needs Screen Recording permission"; exit 0; }
  [ "$seen" != none ] || fail "$mode: no window"
  read -r shade lines frame <<<"$seen"
  # Only kernel tiles draw grid lines.
  [ "$lines" -ge 5 ] || fail "$mode: no grid lines ($lines per mille)"
}
measure dark
dark=$shade dark_frame=$frame
measure light
light=$shade light_frame=$frame
# The frame proves the mode took.
[ "$dark_frame" -lt 100 ] || fail "the dark app's frame paints at $dark_frame"
[ "$light_frame" -gt 150 ] || fail "the light app's frame paints at $light_frame"
[ "$dark" -gt 200 ] || fail "the dark app's grid paints at $dark"
[ "$light" -gt 200 ] || fail "the light app's grid paints at $light"
echo "ok: theme (dark $dark, frame $dark_frame; light $light, frame $light_frame)"
