#!/usr/bin/env bash
# Another program reads, replaces and saves Markdown open in ElyMark.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
# Its real path: macOS links /var to /private/var.
work=$(cd "$(mktemp -d)" && pwd -P)
pid=
cleanup() {
  [ -z "$pid" ] || kill "$pid" 2>/dev/null || true
  [ -z "$pid" ] || wait "$pid" 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT
fail() {
  echo "FAIL: $*" >&2
  [ ! -f "$work/out" ] || tail -n 20 "$work/out" >&2
  exit 1
}
native() { if command -v cygpath >/dev/null; then cygpath -m "$1"; else printf '%s' "$1"; fi; }

cargo build -q -p ely-mark --manifest-path "$root/Cargo.toml"
mark=${CARGO_TARGET_DIR:-$root/target}/debug/ely-mark
export ELY_DATA_ROOT ELY_LOCALE=en ELY_NO_DIALOG=1 ELY_LOG=info
ELY_DATA_ROOT=$(native "$work/data")
file=$(native "$work/note.md")
printf '# Note\n\nBUNDLE TEXT\n' >"$work/note.md"
text() { "$mark" text "$@"; }

ELY_SMOKE=hold "$mark" "$file" >"$work/out" 2>&1 &
pid=$!
for _ in $(seq 120); do
  text docs 2>/dev/null | grep -qF note.md && break
  kill -0 "$pid" 2>/dev/null || fail "ElyMark exited"
  sleep 1
done
text docs | grep -qF note.md || fail "note.md never showed in text docs"
got=$(text get "$file" | tr -d '\r')
[ "$got" = "$(printf '# Note\n\nBUNDLE TEXT')" ] || fail "get read '$got'"
text replace "$file" BUNDLE 'Ely 世界' || fail "replacing"
got=$(text get "$file" | tr -d '\r')
[ "$got" = "$(printf '# Note\n\nEly 世界 TEXT')" ] || fail "after replace, get read '$got'"
if text replace "$file" zebra x 2>"$work/err"; then fail "a missing word was replaced"; fi
grep -q "nothing replaced" "$work/err" || fail "the refusal says $(cat "$work/err")"
if text get "$work/other.md" 2>/dev/null; then fail "a file not open was read"; fi
if text replace "$file" BUNDLE 2>/dev/null; then fail "a short command ran"; fi
saved=$(text save "$file" | tr -d '\r') || fail "saving"
[ "$(native "$saved")" = "$file" ] || fail "save answered '$saved', not $file"
# On disk at the answer: crash recovery cannot stand in.
[ "$(cat "$work/note.md")" = "$(printf '# Note\n\nEly 世界 TEXT')" ] || fail "the saved note.md reads '$(cat "$work/note.md")'"
echo "ok: ElyMark answered text docs, get, replace and save from another process; the save was on disk at its answer"
