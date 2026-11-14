#!/usr/bin/env bash
# Unpacks the official kernel's libraries, never mounting the image.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
dmg=$root/engines/LibreOffice_26.2.6_MacOS_aarch64.dmg
out=$root/engines/lo-official
command -v 7zz >/dev/null || { echo "extract-official-lo.sh: needs 7zz (brew install sevenzip)" >&2; exit 1; }
[ -f "$dmg" ] || { echo "extract-official-lo.sh: no $dmg" >&2; exit 1; }
[ "$(stat -f %z "$dmg")" = 297798926 ] || { echo "extract-official-lo.sh: $dmg is not the pinned image" >&2; exit 1; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
7zz x -y -snld -o"$work" "$dmg" "LibreOffice/LibreOffice.app/Contents/Frameworks/*" "LibreOffice/LibreOffice.app/Contents/Resources/*" >/dev/null
rm -rf "$out"
mkdir -p "$out"
# A plain folder, not a bundle, so nothing registers.
mv "$work/LibreOffice/LibreOffice.app/Contents" "$out/Contents"
# Handshakes pass, so the tile check rejects it.
"$root/scripts/kernel-id.sh" "$out/Contents" "$out/Contents/Resources/ely-kernel-id"
echo "extract-official-lo.sh: $out/Contents/Frameworks"
