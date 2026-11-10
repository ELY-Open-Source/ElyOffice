#!/usr/bin/env bash
# With github.com out of reach, updates come by Ely's relay.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Linux ] || {
  echo "SKIP: Linux only (mount namespaces)"
  exit 0
}
curl -fsS -m 20 -o /dev/null https://office.elygpui.com/release.json || {
  echo "SKIP: office.elygpui.com is out of reach from here"
  exit 0
}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# github.com resolves to this machine: nothing answers there.
printf '127.0.0.1 localhost\n127.0.0.1 github.com\n' >"$work/hosts"
sed 's/^hosts:.*/hosts: files dns/' /etc/nsswitch.conf >"$work/nsswitch.conf"
binary=$(cd "$root" && cargo test -p ely-platform --test update_sources --no-run 2>&1 | sed -n 's/.*Executable.*(\(.*\))$/\1/p' | tail -1)
[ -x "$binary" ] || {
  echo "FAIL: the update test did not build" >&2
  exit 1
}
unshare -rm sh -c 'mount --bind "$1/hosts" /etc/hosts && mount --bind "$1/nsswitch.conf" /etc/nsswitch.conf && exec "$2" --ignored --nocapture' sh "$work" "$binary" >"$work/out.log" 2>&1 || {
  echo "FAIL: $(grep -E 'panicked' -A1 "$work/out.log" | tail -1)" >&2
  exit 1
}
echo "update-relay: $(sed -n 's/^relay: //p' "$work/out.log")"
