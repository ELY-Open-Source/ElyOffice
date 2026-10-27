#!/usr/bin/env bash
# Unreadable shared settings still let ElyDoc start.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
work=$(mktemp -d)
cleanup() {
  if [ -n "${pid:-}" ]; then
    # Its worker writes into the folder too.
    pkill -9 -P "$pid" 2>/dev/null || true
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
  rm -rf "$work"
}
trap cleanup EXIT

cargo build -q -p ely-lok --bin ely-lok-worker --manifest-path "$root/Cargo.toml"
cargo build -q -p ely-doc --manifest-path "$root/Cargo.toml"
shared="$work/home/Library/Application Support/ElyOffice"
mkdir -p "$shared"
printf '{"version": 2}\n' >"$shared/settings.json"
HOME="$work/home" ELY_LOG=info ELY_LOCALE=en ELY_NO_DIALOG=1 ELY_SMOKE=hold exec "$root/target/debug/ely-doc" >"$work/out" 2>&1 &
pid=$!
for _ in $(seq 90); do
  grep -q "the author is set\|did not start" "$work/out" && break
  kill -0 "$pid" 2>/dev/null || break
  sleep 1
done
grep -q "lok: the author is set" "$work/out" || { echo "FAIL: the engine did not start"; tail -20 "$work/out"; exit 1; }
kill -0 "$pid" 2>/dev/null || { echo "FAIL: ElyDoc exited"; tail -20 "$work/out"; exit 1; }
grep -q '{"version": 2}' "$shared/settings.json" || { echo "FAIL: the unreadable file was overwritten"; exit 1; }
echo "ok: ElyDoc starts past an unreadable settings file, which stays as it was"
