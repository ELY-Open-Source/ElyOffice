#!/usr/bin/env bash
# Duplicate groups only shrink: stale skips and returning versions fail.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
mkdir -p "$work/src"
touch "$work/src/lib.rs"
manifest() {
  printf '[package]\nname = "probe"\nversion = "0.1.0"\nedition = "2024"\npublish = false\n\n[dependencies]\n%s' "$1" >"$work/Cargo.toml"
  cargo generate-lockfile -q --manifest-path "$work/Cargo.toml"
}
config() { printf '[bans]\nmultiple-versions = "deny"\nskip = [%s]\n' "$1" >"$work/deny.toml"; }
check() { bash "$root/scripts/deny.sh" "$work" bans >/dev/null 2>&1; }
both='base64 = "0.23"
old = { package = "base64", version = "0.22" }
'
manifest "$both"
config '"base64"'
check || fail "a listed duplicate group failed"
config ''
! check || fail "an unlisted duplicate group passed"
manifest 'base64 = "0.23"
'
config '"base64"'
! check || fail "a skip no longer needed passed"
config ''
check || fail "one version without a skip failed"
manifest "$both"
! check || fail "the old version came back unnoticed"
config '"base64", "no-such-crate"'
! check || fail "a skip for a crate not in the graph passed"
echo "ok: deny rules"
