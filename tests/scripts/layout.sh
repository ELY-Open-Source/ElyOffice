#!/usr/bin/env bash
# First-batch apps at three widths: nothing passes the window's sides.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
work=$(mktemp -d)
# Screenshots outlive the run, for a look inside the panels.
shots=${ELY_LAYOUT_SHOTS:-$(mktemp -d)}
cleanup() {
  local status=$?
  [ -n "${pid:-}" ] && { pkill -9 -P "$pid" 2>/dev/null || true; kill "$pid" 2>/dev/null || true; }
  [ "$status" = 0 ] && rm -rf "$work" || echo "kept $work" >&2
  exit "$status"
}
trap cleanup EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

apps=$(awk -F'\t' '$4 == 1 { print $1 }' "$root/packaging/apps.tsv")
[ -n "$apps" ] || fail "apps.tsv lists no first-batch app"
# By version: crates.io has its own ely-mail.
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
# shellcheck disable=SC2046
cargo build -q --release --manifest-path "$root/Cargo.toml" --bins -p ely-lok -p ely-pdf-worker $(printf -- "-p %s@$version " $apps)
swiftc -O -o "$work/ax-frames" "$root/tests/scripts/ax-frames.swift"
[ "$("$work/ax-frames" $$ | head -n 1)" != untrusted ] || { echo "SKIP: needs Accessibility permission"; exit 0; }
mkdir -p "$shots" "$work/repo"
git -C "$work/repo" init -q
printf '# Notes\n\nA paragraph long enough to wrap at a narrow width, with **bold** words.\n' >"$work/notes.md"
cp "$work/notes.md" "$work/repo/notes.md"
fixtures=$root/tests/fixtures/office

# Elements past the window's sides; scrolled rows may pass below.
outside() {
  python3 - "$1" <<'PY'
import json, sys
rows = [json.loads(line) for line in open(sys.argv[1]) if line.startswith("{")]
window = next(row for row in rows if row["role"] == "AXWindow")
wx, wy, ww, wh = window["frame"]
for row in rows:
    if "frame" not in row or row["role"] in ("AXWindow", "AXMenuBar", "AXMenuBarItem", "AXMenu", "AXMenuItem"):
        continue
    x, y, w, h = row["frame"]
    if w <= 0 or h <= 0:
        continue
    if x < wx - 1 or x + w > wx + ww + 1:
        print(f'{row["role"]} "{row["name"]}" at {x:.0f},{y:.0f} {w:.0f}x{h:.0f}; window {wx:.0f},{wy:.0f} {ww:.0f}x{wh:.0f}')
PY
}

window_id() {
  swift -e 'import CoreGraphics; let l = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as! [[String: Any]]; for w in l where (w["kCGWindowOwnerPID"] as? Int) == Int(CommandLine.arguments[1])! && (w["kCGWindowLayer"] as? Int) == 0 { print(w["kCGWindowNumber"]!); break }' "$1"
}

# What each app opens; ElyMail starts with no account.
opens() { # slug
  case $1 in
    ely-doc) echo "$fixtures/t.docx" ;;
    ely-table) echo "$fixtures/t.xlsx" ;;
    ely-show) echo "$fixtures/t.pptx" ;;
    ely-pdf) echo "$fixtures/t.pdf" ;;
    ely-mail) echo ;;
    ely-tree) echo "$work/repo" ;;
    ely-mark) echo "$work/notes.md" ;;
    *) return 1 ;;
  esac
}

check() { # slug, then a file or nothing
  local slug=$1 home=$work/home-$1
  shift
  mkdir -p "$home"
  HOME=$home ELY_SMOKE=hold ELY_NO_DIALOG=1 ELY_LOCALE=en "$root/target/release/$slug" "$@" >"$work/$slug.out" 2>&1 &
  pid=$!
  sleep 8
  for width in 640 800 1100; do
    osascript -e "tell application \"System Events\" to tell (first process whose unix id is $pid)" \
      -e "set size of window 1 to {$width, 680}" -e "set position of window 1 to {80, 60}" \
      -e "set frontmost to true" -e "end tell" >/dev/null
    sleep 2
    # GPUI builds its AX tree once asked.
    "$work/ax-frames" "$pid" >/dev/null
    sleep 1
    "$work/ax-frames" "$pid" >"$work/$slug-$width.jsonl"
    [ "$(grep -c '"frame"' "$work/$slug-$width.jsonl")" -gt 8 ] || fail "$slug at $width: no AX content"
    screencapture -x -l"$(window_id "$pid")" "$shots/$slug-$width.png"
    out=$(outside "$work/$slug-$width.jsonl")
    [ -z "$out" ] || fail "$slug at $width leaves the window:
$out"
  done
  pkill -9 -P "$pid" 2>/dev/null || true
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  pid=
}

for slug in $apps; do
  file=$(opens "$slug") || fail "apps.tsv's $slug has no layout case"
  if [ -n "$file" ]; then check "$slug" "$file"; else check "$slug"; fi
done
echo "ok: layout (screenshots in $shots)"
