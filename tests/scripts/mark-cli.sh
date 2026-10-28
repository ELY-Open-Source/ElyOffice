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
# get prints these byte for byte, final newlines too.
printf 'bare' >"$work/bare.md"
printf 'many\n\n\n' >"$work/many.md"
text() { "$mark" text "$@"; }
same() { cmp -s "$1" "$2" || fail "$3: $(od -c "$1" | head -3)"; }

ELY_SMOKE=hold "$mark" "$file" "$(native "$work/bare.md")" "$(native "$work/many.md")" >"$work/out" 2>&1 &
pid=$!
for _ in $(seq 120); do
  text docs 2>/dev/null | grep -qF many.md && break
  kill -0 "$pid" 2>/dev/null || fail "ElyMark exited"
  sleep 1
done
for name in note bare many; do
  text docs | grep -qF "$name.md" || fail "$name.md never showed in text docs"
  text get "$(native "$work/$name.md")" >"$work/got" || fail "reading $name.md"
  same "$work/got" "$work/$name.md" "get changed $name.md"
done
text replace "$file" BUNDLE 'Ely 世界' || fail "replacing"
printf '# Note\n\nEly 世界 TEXT\n' >"$work/want"
text get "$file" >"$work/got" || fail "reading after replace"
same "$work/got" "$work/want" "after replace, get read other bytes"
if text replace "$file" zebra x 2>"$work/err"; then fail "a missing word was replaced"; fi
grep -q "nothing replaced" "$work/err" || fail "the refusal says $(cat "$work/err")"
if text get "$work/other.md" 2>/dev/null; then fail "a file not open was read"; fi
if text replace "$file" BUNDLE 2>/dev/null; then fail "a short command ran"; fi
saved=$(text save "$file" | tr -d '\r') || fail "saving"
[ "$(native "$saved")" = "$file" ] || fail "save answered '$saved', not $file"
# On disk at the answer: crash recovery cannot stand in.
same "$work/note.md" "$work/want" "the saved note.md"
echo "ok: ElyMark answered text docs, get, replace and save from another process; the save was on disk at its answer"
