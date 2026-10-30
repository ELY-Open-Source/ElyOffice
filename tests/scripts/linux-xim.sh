#!/usr/bin/env bash
# ElyMark on X11 opens an XIM input context with fcitx5.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
[ "$(uname)" = Linux ] && [ -n "${DISPLAY:-}" ] || { echo "SKIP: needs an X display"; exit 0; }
command -v fcitx5 >/dev/null || { echo "SKIP: no fcitx5"; exit 0; }
bin=${ELY_MARK_BIN:-}
if [ -z "$bin" ]; then
  version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
  # The binary Cargo just built, wherever its target is.
  bin=$(cargo build -q --release --manifest-path "$root/Cargo.toml" -p "ely-mark@$version" \
    --message-format=json-render-diagnostics |
    python3 -c 'import json, sys
found = [m["executable"] for m in map(json.loads, sys.stdin)
         if m.get("reason") == "compiler-artifact" and m.get("executable") and m["target"]["name"] == "ely-mark"]
print(found[-1] if found else "")')
  [ -n "$bin" ] || fail "cargo named no ely-mark binary"
fi
work=$(mktemp -d)
ours=
pid=
cleanup() {
  [ -z "$pid" ] || kill -- -"$pid" 2>/dev/null || true
  [ -z "$ours" ] || kill "$ours" 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT
# Ours only if none runs: never replace the user's.
if ! pgrep -x fcitx5 >/dev/null; then
  fcitx5 -d >"$work/fcitx.log" 2>&1
  sleep 3
  ours=$(pgrep -n -x fcitx5) || fail "fcitx5 did not start: $(tail -3 "$work/fcitx.log")"
fi
echo "# x" >"$work/a.md"
env -u WAYLAND_DISPLAY XMODIFIERS=@im=fcitx ELY_SMOKE=hold ELY_LOG=debug HOME="$work/home" \
  setsid "$bin" "$work/a.md" >"$work/out.log" 2>&1 &
pid=$!
for _ in $(seq 40); do
  grep -q -E 'CreateIcReply|XIMClientError' "$work/out.log" && break
  kill -0 "$pid" 2>/dev/null || break
  sleep 0.5
done
said=$( (grep -a -E 'zed_xim::(client|x11rb)\] (->|<-)|XIMClientError' "$work/out.log" || true) |
  sed -E 's/^\[[^]]*\] *//' | tr '\n' ' ')
! grep -q XIMClientError "$work/out.log" || fail "the XIM exchange broke: $said"
if ! grep -q '<-: CreateIcReply' "$work/out.log"; then
  if kill -0 "$pid" 2>/dev/null; then
    state="still running after 20 s"
  else
    wait "$pid" && code=0 || code=$?
    state="exited $code"
    pid=
  fi
  fail "no input context ($bin $state); XIM: ${said:-none}; last output: $(tail -5 "$work/out.log" | tr '\n' ' ')"
fi
echo "PASS: ElyMark on X11 opened an XIM input context with fcitx5"
