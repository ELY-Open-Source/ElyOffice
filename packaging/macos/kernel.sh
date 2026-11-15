#!/usr/bin/env bash
# Builds the shared kernel folder. Usage: kernel.sh <out>
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
out=${1:?the kernel folder to fill}
identity=${ELY_SIGN_IDENTITY:--}
lo=${ELY_LO_APP:-$root/engines/lo-src/instdir/LibreOffice.app}
[ -e "$lo/Contents/Frameworks" ] || {
  echo "kernel.sh: no $lo/Contents/Frameworks; run scripts/build-libreoffice.sh" >&2
  exit 1
}
rm -rf "$out"
mkdir -p "$out"
# LibreOffice's layout without its app: nothing registers.
ditto "$lo/Contents/Frameworks" "$out/Frameworks"
ditto "$lo/Contents/Resources" "$out/Resources"
"$root/packaging/lean.sh" "$out/Resources"
code=()
while IFS= read -r binary; do
  if file -b "$binary" | grep -q '^Mach-O'; then
    code+=("$binary")
  elif [ "$(cat "$binary")" = "invalid - merged lib" ]; then
    # Merged-library builds leave these placeholders.
    rm "$binary"
  else
    echo "kernel.sh: $binary is not code; Frameworks holds only code" >&2
    exit 1
  fi
done < <(find "$out/Frameworks" -type f)
for binary in ${code[@]+"${code[@]}"}; do
  if [ "$identity" = - ]; then
    codesign --force --sign - "$binary"
  else
    codesign --force --options runtime --timestamp --sign "$identity" "$binary"
  fi
done
# Signed files are the ones hashed.
"$root/scripts/kernel-id.sh" "$out" "$out/Resources/ely-kernel-id"
echo "$out"
