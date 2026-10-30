#!/usr/bin/env bash
# linux-xim.sh: runs what Cargo built; says why the app died.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
[ "$(uname)" = Linux ] || { echo "SKIP: Linux only"; exit 0; }
command -v python3 >/dev/null || { echo "SKIP: no python3"; exit 0; }
work=$(mktemp -d)
# The script under test usually stops the stand-in itself.
trap 'pkill -f "$work/bin/fcitx5 serve" 2>/dev/null || true; rm -rf "$work"' EXIT
mkdir -p "$work/repo/tests/scripts" "$work/repo/target/release" "$work/out/release" "$work/bin"
cp "$root/tests/scripts/linux-xim.sh" "$work/repo/tests/scripts/"
cp "$root/Cargo.toml" "$work/repo/"
fake() { # path, body
  printf '#!/bin/bash\n%s\n' "$2" >"$1"
  chmod +x "$1"
}
# Stand-in fcitx5: a process by that name.
fake "$work/bin/fcitx5" '[ "$1" = -d ] && { setsid "$0" serve >/dev/null 2>&1 & exit 0; }; sleep 60'
# Cargo builds into out/, not target/.
fake "$work/bin/cargo" "echo '{\"reason\":\"compiler-artifact\",\"target\":{\"name\":\"ely-mark\"},\"executable\":\"$work/out/release/ely-mark\"}'"
shake='echo "[t DEBUG zed_xim::client] <-: CreateIcReply"; sleep 60'
fake "$work/repo/target/release/ely-mark" "$shake"
run() {
  PATH=$work/bin:$PATH DISPLAY=:9 bash "$work/repo/tests/scripts/linux-xim.sh" >"$work/said" 2>&1
}

fake "$work/out/release/ely-mark" 'echo "no Vulkan driver" >&2; exit 71'
! run || fail "a stale build in target/ passed: $(cat "$work/said")"
grep -q "out/release/ely-mark exited 71" "$work/said" || fail "no exit status: $(cat "$work/said")"
grep -q "no Vulkan driver" "$work/said" || fail "no startup error: $(cat "$work/said")"
fake "$work/out/release/ely-mark" "$shake"
run || fail "the built handshake failed: $(cat "$work/said")"
echo "PASS: linux-xim.sh runs Cargo's build and reports an early exit"
