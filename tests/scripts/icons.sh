#!/usr/bin/env bash
# Each app: macOS squircle, shapeless Linux and Windows icons.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
icons=$root/assets/icons
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
[ "$(uname)" = Darwin ] || {
  echo "SKIP icons: macOS only (AppKit renders the masters)"
  exit 0
}
python3 -c 'import PIL' 2>/dev/null || {
  echo "SKIP icons: no Pillow"
  exit 0
}
swiftc -O -o "$work/icon" "$root/packaging/icons/icon.swift"
# The master build.sh uses at `px`; small cut below 48.
master() { # slug, then -document if so, variant, px
  [ $# -eq 4 ] || set -- "$1" "" "$2" "$3"
  local small=$icons/$1$2-small$3.svg
  if [ "$4" -lt 48 ] && [ -f "$small" ]; then echo "$small"; else echo "$icons/$1$2$3.svg"; fi
}
count=0
documents=0
while IFS=$'\t' read -r slug _; do
  for file in "$slug.svg" "$slug-macos.svg" "$slug.icns" "$slug.png" "$slug.ico"; do
    [ -s "$icons/$file" ] || fail "no $icons/$file; run packaging/icons/build.sh"
  done
  for made in icns png ico; do
    for source in "$icons/$slug".svg "$icons/$slug"-*.svg; do
      case $source in *-document*) continue ;; esac
      [ "$source" -nt "$icons/$slug.$made" ] && fail "$slug.$made is older than $(basename "$source"); run packaging/icons/build.sh"
    done
  done
  # Every shipped size, rendered afresh from its master.
  mkdir "$work/$slug"
  for px in 16 24 32 48 64 128 256 512 1024; do
    "$work/icon" "$px" "$(master "$slug" "" "$px")" "$work/$slug/plain-$px.png"
    "$work/icon" "$px" "$(master "$slug" -macos "$px")" "$work/$slug/macos-$px.png"
  done
  iconutil -c iconset -o "$work/$slug.iconset" "$icons/$slug.icns"
  python3 - "$icons" "$slug" "$work" <<'EOF' || fail "$slug: the rendered icons are wrong"
import os, sys
from PIL import Image
icons, slug, work = sys.argv[1:]
def raw(image):
    # Premultiplied: colour under zero alpha is not drawn.
    data = image.convert("RGBA").tobytes()
    out = bytearray(data)
    for i in range(0, len(data), 4):
        for c in range(3):
            out[i + c] = data[i + c] * data[i + 3] // 255
    return out
def same(made, px, variant, what, most=1.0):
    want = raw(Image.open(f"{work}/{slug}/{variant}-{px}.png"))
    got = raw(made)
    assert len(got) == len(want), f"{what}: {made.size}, not {px}"
    off = sum(abs(a - b) for a, b in zip(got, want)) / len(got)
    assert off < most, f"{what} is not its master (off {off:.2f})"
# Shapeless: the mark alone, no container behind it.
mark = Image.open(f"{icons}/{slug}.png")
same(mark, 512, "plain", "png")
assert mark.convert("RGBA").getpixel((0, 0))[3] == 0, "a container behind the mark"
ico = Image.open(f"{icons}/{slug}.ico")
assert sorted(ico.info["sizes"]) == [(s, s) for s in (16, 24, 32, 48, 64, 128, 256)], ico.info["sizes"]
for size in ico.info["sizes"]:
    layer = ico.ico.getimage(size)
    same(layer, size[0], "plain", f"ico {size[0]}")
    corners = [(0, 0), (size[0] - 1, 0), (0, size[0] - 1), (size[0] - 1, size[0] - 1)]
    assert all(layer.convert("RGBA").getpixel(at)[3] == 0 for at in corners), f"ico {size[0]}: a container"
names = sorted(os.listdir(f"{work}/{slug}.iconset"))
assert len(names) == 10, names
for name in names:
    made = Image.open(f"{work}/{slug}.iconset/{name}")
    # macOS stores 16 and 32 px lossily.
    same(made, made.width, "macos", f"icns {name}", 8.0 if name in ("icon_16x16.png", "icon_32x32.png") else 1.0)
# Squircle: Apple's margin is clear, the body's middle is solid.
big = Image.open(f"{work}/{slug}.iconset/icon_512x512@2x.png").convert("RGBA")
assert big.getpixel((40, 512))[3] == 0 and big.getpixel((150, 150))[3] == 0, "not a squircle"
assert big.getpixel((512, 110))[3] == 255, "no body"
EOF
  # Finder's document icon, for the apps whose types name one.
  if grep -q CFBundleTypeIconFile <<<"$("$root/packaging/macos/documents.sh" "$slug")"; then
    doc=$icons/$slug-document
    [ -s "$doc.icns" ] || fail "no $doc.icns; run packaging/icons/build.sh"
    for source in "$doc"-*.svg; do
      [ "$source" -nt "$doc.icns" ] && fail "$slug-document.icns is older than $(basename "$source")"
    done
    iconutil -c iconset -o "$work/$slug-document.iconset" "$doc.icns"
    for name in "$work/$slug-document.iconset"/*.png; do
      px=$(sips -g pixelWidth "$name" | awk '/pixelWidth/ { print $2 }')
      "$work/icon" "$px" "$(master "$slug" -document -macos "$px")" "$name.want.png"
    done
    python3 - "$work/$slug-document.iconset" <<'EOF' || fail "$slug: the document icon is not its master"
import glob, sys
from PIL import Image
pngs = sorted(p for p in glob.glob(sys.argv[1] + "/*.png") if not p.endswith(".want.png"))
assert len(pngs) == 10, pngs
for made in pngs:
    a, b = Image.open(made).convert("RGBA"), Image.open(made + ".want.png").convert("RGBA")
    assert a.size == b.size, made
    off = sum(abs(x - y) for x, y in zip(a.tobytes(), b.tobytes())) / len(a.tobytes())
    assert off < (8.0 if a.width <= 32 else 1.0), f"{made}: off {off:.2f}"
# A white page, the app's mark in its middle.
big = Image.open(sys.argv[1] + "/icon_512x512@2x.png").convert("RGBA")
assert big.getpixel((260, 120))[:3] == (255, 255, 255), "no page"
assert big.getpixel((40, 512))[3] == 0, "not a page"
EOF
    documents=$((documents + 1))
  fi
  count=$((count + 1))
done <"$root/packaging/apps.tsv"
[ "$count" -eq 8 ] || fail "checked $count apps, expected 8"
[ "$documents" -eq 6 ] || fail "checked $documents document icons, expected 6"
echo "icons: 8 apps, squircle on macOS, shapeless on Linux and Windows; 6 macOS document icons"
