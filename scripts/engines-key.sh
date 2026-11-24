#!/usr/bin/env bash
# Prints the engines' key: a hash of pins and patches.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
find scripts/fetch-references.sh scripts/apply-lo-patches.sh scripts/lo-configured.sh scripts/lo-extras.sh scripts/build-libreoffice.sh scripts/fetch-fonts.sh scripts/build-mupdf.sh \
  scripts/build-git.sh scripts/fetch-git-lfs.sh scripts/fetch-tessdata.sh \
  packaging/libreoffice patches -type f | LC_ALL=C sort | xargs shasum -a 256 | shasum -a 256 | cut -c1-12
