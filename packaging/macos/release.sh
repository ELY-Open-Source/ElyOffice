#!/usr/bin/env bash
# Usage: release.sh [batch]; a dmg per app, one of all.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
batch=${1:-1}
dist=${ELY_DIST:-$root/dist}
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
arch=$(uname -m)
[ -z "${ELY_NOTARY_PROFILE:-}" ] || [ -n "${ELY_SIGN_IDENTITY:-}" ] || {
  echo "release.sh: notarizing needs ELY_SIGN_IDENTITY, a Developer ID" >&2
  exit 1
}
# Usage: dmg <out> <volume> <app>...; apps beside Applications.
dmg() {
  local out=$1 volume=$2 stage
  shift 2
  rm -f "$out"
  # Drag an app onto Applications.
  stage=$(mktemp -d)
  for app in "$@"; do ditto "$app" "$stage/$(basename "$app")"; done
  ln -s /Applications "$stage/Applications"
  hdiutil create -quiet -volname "$volume" -srcfolder "$stage" -format UDZO "$out"
  rm -rf "$stage"
  if [ -n "${ELY_NOTARY_PROFILE:-}" ]; then
    codesign --force --timestamp --sign "$ELY_SIGN_IDENTITY" "$out"
    xcrun notarytool submit "$out" --keychain-profile "$ELY_NOTARY_PROFILE" --wait
    xcrun stapler staple "$out"
  else
    echo "release.sh: $out is not notarized (no ELY_NOTARY_PROFILE)" >&2
  fi
  echo "$out"
}
"$root/packaging/source.sh" "$dist"
awk -F'\t' -v batch="$batch" '$4 == batch { print $1 "\t" $2 }' "$root/packaging/apps.tsv" |
  while IFS=$'\t' read -r slug name; do
    "$root/packaging/macos/bundle.sh" "$slug" release
    codesign --verify --deep --strict "$dist/$name.app"
    out=$dist/$name-$version-$arch.dmg
    dmg "$out" "$name" "$dist/$name.app"
  done
# Every app of the batch in one dmg.
apps=()
for name in $(awk -F'\t' -v batch="$batch" '$4 == batch { print $2 }' "$root/packaging/apps.tsv"); do
  apps+=("$dist/$name.app")
done
dmg "$dist/ElyOffice-$version-$arch.dmg" "Ely Office" "${apps[@]}"
rm -rf "$root/target/release/incremental"
