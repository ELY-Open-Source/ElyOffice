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
bin=${ELY_MARK_BIN:-$root/target/release/ely-mark}
if [ -z "${ELY_MARK_BIN:-}" ]; then
  version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
  cargo build -q --release --manifest-path "$root/Cargo.toml" -p "ely-mark@$version"
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
  sleep 0.5
done
said=$(grep -a -E 'zed_xim::(client|x11rb)\] (->|<-)|XIMClientError' "$work/out.log" | sed -E 's/^\[[^]]*\] *//' | tr '\n' ' ')
! grep -q XIMClientError "$work/out.log" || fail "the XIM exchange broke: $said"
grep -q '<-: CreateIcReply' "$work/out.log" || fail "no input context in 20 s: $said"
echo "PASS: ElyMark on X11 opened an XIM input context with fcitx5"
