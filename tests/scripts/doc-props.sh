#!/usr/bin/env bash
# ElyDoc's spacing fields: quick edits keep each other.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
work=$(mktemp -d)
trap 'kill "${pid:-0}" 2>/dev/null || true; rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

cargo build -q -p ely-lok --bin ely-lok-worker --manifest-path "$root/Cargo.toml"
cargo build -q -p ely-doc --manifest-path "$root/Cargo.toml"
swiftc -O -o "$work/axv" "$root/tests/scripts/ax-values.swift"
swiftc -O -o "$work/click" "$root/tests/scripts/click.swift"
[ "$("$work/axv" $$)" != untrusted ] || { echo "SKIP: needs Accessibility permission"; exit 0; }
ELY_SMOKE=hold ELY_LOCALE=en HOME="$work" XDG_DATA_HOME="$work/data" \
  perl -e 'alarm 120; exec @ARGV' "$root/target/debug/ely-doc" >"$work/out" 2>"$work/err" &
pid=$!
log() { find "$work" -path '*/ElyOffice/ely-doc/ely.log' 2>/dev/null | head -n 1; }
for _ in $(seq 200); do grep -q "doc: opened" "$(log)" 2>/dev/null && break; sleep 0.2; done
grep -q "doc: opened Untitled 1" "$(log)" || fail "no blank tab: $(tail -n 3 "$work/err")"
keys() { osascript -e 'tell application "System Events"' -e "$1" -e 'end tell' >/dev/null; }
osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" >/dev/null
sleep 0.5
keys 'keystroke "Spacing"'
# Number fields top to bottom: above, below, tab position.
fields() { "$work/axv" "$pid" | awk -F'\t' '$1 == "AXTextField" && $2 ~ /^value=[0-9.]+$/ { sub("value=", "", $2); sub("at=", "", $5); split($5, p, ","); print p[2], p[1], $2 }' | sort -n; }
spots=
for _ in 1 2 3 4 5; do
  spots=$(fields)
  [ "$(wc -l <<<"$spots")" -ge 3 ] && break
  sleep 1
done
[ "$(wc -l <<<"$spots")" -ge 3 ] || fail "the spacing fields are not in the AX tree"
read -r above_y above_x _ <<<"$(sed -n 1p <<<"$spots")"
read -r below_y below_x _ <<<"$(sed -n 2p <<<"$spots")"
type_into() {
  "$work/click" "$1" "$2"
  sleep 0.3
  keys 'keystroke "a" using command down'
  keys "keystroke \"$3\""
  keys 'key code 36'
}
type_into "$above_x" "$above_y" 6
type_into "$below_x" "$below_y" 12
"$work/click" $((above_x - 300)) "$above_y"
want=$'6.0\n12.0'
seen=
for _ in 1 2 3 4 5 6; do
  sleep 1
  seen=$(fields | head -n 2 | awk '{ print $3 }')
  [ "$seen" = "$want" ] && break
done
[ "$seen" = "$want" ] || fail "spacing reads $(tr '\n' ' ' <<<"$seen")after 6 and 12"
grep -c "panel runs paragraph.spacing" "$(log)" | grep -qx 2 || fail "expected two spacing commands"
echo "ok: doc-props"
