#!/usr/bin/env bash
# ElyMark builds from a bare checkout: no engines, no references.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
git -C "$root" archive HEAD | tar -x -C "$work"
if [ -e "$work/engines" ] || [ -e "$work/reference" ]; then
  echo "FAIL: the checkout holds engines or references"
  exit 1
fi
mkdir "$work/no-mupdf"
# Its own target: paths differ from the work tree's.
target=${CARGO_TARGET_DIR:-$root/target/bare}
if ! CARGO_TARGET_DIR=$target CARGO_INCREMENTAL=0 ELY_MUPDF_DIR=$work/no-mupdf \
  cargo build -q --locked -p ely-mark --manifest-path "$work/Cargo.toml"; then
  echo "FAIL: ElyMark needs an engine to build"
  exit 1
fi
echo "PASS: ElyMark builds without LibreOffice or MuPDF"
