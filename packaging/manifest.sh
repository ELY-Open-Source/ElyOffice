#!/usr/bin/env bash
# manifest.sh <name> <platform> <file>: writes <name>-<platform>.json.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
name=${1:?ely-office, or a 0.1.16 app slug}
platform=${2:?platform, such as windows-x86_64}
file=${3:?the release file}
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
if command -v sha256sum >/dev/null; then
  sha=$(sha256sum "$file" | cut -d' ' -f1)
else
  sha=$(shasum -a 256 "$file" | cut -d' ' -f1)
fi
url=https://github.com/ELY-Open-Source/ElyOffice/releases/download/v$version/$(basename "$file")
out=$(dirname "$file")/$name-$platform.json
printf '{"version": "%s", "assets": {"%s": {"url": "%s", "sha256": "%s"}}}\n' \
  "$version" "$platform" "$url" "$sha" >"$out"
echo "$out"
