#!/usr/bin/env bash
# Usage: container.sh <command...>; runs it in AlmaLinux 8.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ $# -gt 0 ] || {
  echo "container.sh: give a command, such as scripts/build-libreoffice.sh" >&2
  exit 1
}
image=ely-office-linux-build
docker build -q -t "$image" "$root/packaging/linux" >/dev/null
# Own target and Cargo home: host builds need newer glibc.
mkdir -p "$root/target/alma8" "$root/target/alma8-cargo" "$root/target/alma8-home"
# Engines shared from another clone resolve inside too.
share=()
[ -z "${ELY_SHARE:-}" ] || share=(-v "$ELY_SHARE:$ELY_SHARE")
docker run --rm --user "$(id -u):$(id -g)" -v "$root:$root" ${share[@]+"${share[@]}"} -w "$root" \
  -e HOME="$root/target/alma8-home" -e CARGO_HOME="$root/target/alma8-cargo" \
  -e CARGO_TARGET_DIR="$root/target/alma8" -e PKG_CONFIG_PATH="$root/engines/openssl/lib/pkgconfig" \
  -e ELY_RELEASE_TOKEN \
  -e ELY_MICROSOFT_CLIENT_ID \
  -e ELY_UPDATE_KEY \
  "$image" "$@"
